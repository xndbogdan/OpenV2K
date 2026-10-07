//! Software backend adapter: queue a materialized model body through the
//! retail face constructors (`crate::software::model`).
//!
//! Retail executes the model's command stream against a vertex cache of
//! VIEW points it projects with the node's axes. The port materializes the
//! stream instead, so this adapter rebuilds the constructors' inputs from a
//! [`ModelDraw`]: each vertex becomes a cache entry, each face's lighting a
//! normal-cache shade dword, and each face the command the constructor
//! tables would have run. A producer that owns the node's integer frame
//! supplies the VIEW points and the model-space light, so those entries and
//! shades are retail's; otherwise each world point is brought into VIEW
//! through the scene's native viewport or, without one, the floating
//! camera, and is only as exact as the materializer's world point. The
//! constructors themselves are byte-exact.

use std::collections::HashMap;

use v2k_formats::models::{
    ModelEdgeEndpointSnapshot, ModelEdgeStyle, ModelFaceCull, ModelFaceShading, ModelFaceVertices,
    ModelPainterDepthKey, ModelSlotClip, ModelVertexProjection,
};

use crate::gl_backend::{
    authored_face_plane_visible, model_lighting_normals, resolve_model_vertices,
    retail_model_shade, retail_model_shade_raw, transform_model_light_normal, view_light_normal,
};
use crate::projection::NativeViewportWords;
use crate::renderer::WorldSpriteBlend;
use crate::renderer::{
    ModelBillboardDraw, ModelDepthFade, ModelDraw, ModelEdgeProjection, ModelNearClip,
    NativeModelFogPass, WorldModelFog,
};
use crate::software::model::{
    construct_billboard, construct_edge, construct_face, screen_midpoint, BillboardCommand,
    EdgeCommand, FaceContext, FacePass, ModelCorner, ModelNormal,
};
use crate::software::particle::{queue_particle, Particle, ParticleScene, ParticleShadow};
use crate::software::store::MaterialStore;
use crate::software::terrain::{GroundMaterial, GroundProjection};
use crate::software::{material_flags, MaterialId, PrimitiveQueue, QueueError};

/// The scene a model body is queued into.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ModelScene {
    pub camera_position: [f32; 3],
    /// GL view rows (x right, y up, z toward the viewer).
    pub camera_basis: [[f32; 3]; 3],
    pub native: Option<NativeViewportWords>,
    /// The lens with identity axes; fade terms are per draw.
    pub lens: GroundProjection,
    pub world_fog: Option<WorldModelFog>,
    /// VIEW units per port world unit off the native path: 256 for the
    /// world's 8.8 coordinates, 100 for frontend models.
    pub units: f32,
}

impl ModelScene {
    /// VIEW units per world unit for a draw's coordinate domain. Native
    /// scenes always use the world's 8.8 coordinates.
    pub(crate) fn units_for(native: bool, near_clip: ModelNearClip) -> f32 {
        match near_clip {
            ModelNearClip::RetailFrontend if !native => 100.0,
            _ => 256.0,
        }
    }
}

/// Fade terms, constructor table and fog colour for one draw.
struct DrawFog {
    fade: [i32; 3],
    pass: FacePass,
    colour: u32,
}

fn fade_terms(near: i32, far: i32) -> [i32; 3] {
    if far > near {
        [0x0100_0000 / (far - near), near, far]
    } else {
        // Equal planes: an authored hard step at `near`.
        [0, near, near]
    }
}

fn colour_word(colour: [f32; 3]) -> u32 {
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0) as u8;
    u32::from(crate::software::store::rgb565(
        byte(colour[0]),
        byte(colour[1]),
        byte(colour[2]),
    ))
}

impl ModelScene {
    /// VIEW point of a world point: the native viewport's Q31 rows over the
    /// raw delta when the scene has one, else the floating camera. Native
    /// deltas wrap to signed words, as retail forms actor-minus-viewport
    /// deltas, so any torus image of the point lands beside the eye.
    fn view_raw(&self, world: [f32; 3]) -> [i32; 3] {
        self.node_view_raw(world, world)
    }

    /// VIEW point of a model vertex at `world` whose node sits at `node`.
    /// Retail wraps only the node's delta; vertex offsets are added in VIEW
    /// space, so a large model stays whole across the wrap.
    fn node_view_raw(&self, node: [f32; 3], world: [f32; 3]) -> [i32; 3] {
        if let Some(native) = self.native {
            let delta: [i32; 3] = std::array::from_fn(|axis| {
                let node_raw = (node[axis] * 256.0).round() as i32;
                let offset = ((world[axis] - node[axis]) * 256.0).round() as i32;
                i32::from(node_raw.wrapping_sub(native.origin_raw[axis]) as i16)
                    .wrapping_add(offset)
            });
            return native.axes_q31.map(|row| {
                row.iter().zip(delta).fold(0i32, |sum, (&axis, value)| {
                    sum.wrapping_add(crate::software::mul_q31(axis, value))
                })
            });
        }
        let relative: [f32; 3] =
            std::array::from_fn(|axis| world[axis] - self.camera_position[axis]);
        let dot =
            |row: [f32; 3]| row[0] * relative[0] + row[1] * relative[1] + row[2] * relative[2];
        [
            (dot(self.camera_basis[0]) * self.units).round() as i32,
            (dot(self.camera_basis[1]) * self.units).round() as i32,
            (-dot(self.camera_basis[2]) * self.units).round() as i32,
        ]
    }

    /// The producer's source-owned VIEW point for vertex `index`, which only
    /// a scene projecting with the native viewport consumes.
    fn native_vertex_view(&self, views: &[Option<[i32; 3]>], index: usize) -> Option<[i32; 3]> {
        self.native?;
        views.get(index).copied().flatten()
    }

    /// VIEW-space direction of a model normal, which `FUN_0046D3F0`
    /// effectively dots with the context's light: the native viewport's rows
    /// when the scene has them, else the floating camera.
    fn view_light_normal(&self, orientation: [[f32; 3]; 3], normal: [f32; 3]) -> [f32; 3] {
        let Some(native) = self.native else {
            return view_light_normal(self.camera_basis, orientation, normal);
        };
        let world = transform_model_light_normal(orientation, normal);
        native.axes_q31.map(|row| {
            row.iter()
                .zip(world)
                .map(|(&axis, value)| axis as f32 / 2_147_483_648.0 * value)
                .sum()
        })
    }

    /// The eye in the world image nearest `node`: the native viewport's
    /// origin brought within a signed word of the node, else the floating
    /// camera (which, in a native scene, may sit in another torus image).
    fn eye_near(&self, node: [f32; 3]) -> [f32; 3] {
        match self.native {
            Some(native) => std::array::from_fn(|axis| {
                let node_raw = (node[axis] * 256.0).round() as i32;
                let delta = native.origin_raw[axis].wrapping_sub(node_raw) as i16;
                node[axis] + f32::from(delta) / 256.0
            }),
            None => self.camera_position,
        }
    }

    /// `FUN_00464E60`'s table choice for this node, or `None` when the node
    /// lies wholly beyond the far plane.
    fn draw_fog(&self, draw: &ModelDraw<'_>) -> Option<DrawFog> {
        self.node_fog(
            draw.transform.position,
            draw.mesh.radius_raw,
            draw.transform.scale,
            draw.depth_fade,
        )
    }

    fn node_fog(
        &self,
        position: [f32; 3],
        radius_raw: u16,
        scale: f32,
        depth_fade: ModelDepthFade,
    ) -> Option<DrawFog> {
        let origin_depth = self.view_raw(position)[2];
        let radius = (f32::from(radius_raw) * scale.abs() / 100.0 * self.units) as i32;
        let select = |near: i32, far: i32, colour: [f32; 3]| -> Option<DrawFog> {
            if origin_depth.wrapping_sub(radius) >= far {
                return None;
            }
            let pass = if origin_depth.wrapping_add(radius) >= near {
                FacePass::Fog
            } else {
                FacePass::Near
            };
            Some(DrawFog {
                fade: fade_terms(near, far),
                pass,
                colour: colour_word(colour),
            })
        };
        match depth_fade {
            ModelDepthFade::Disabled => Some(DrawFog {
                fade: [0, i32::MAX, i32::MAX],
                pass: FacePass::Near,
                colour: 0,
            }),
            ModelDepthFade::WorldRaw { fog, pass } => Some(DrawFog {
                fade: fade_terms(fog.planes.near_raw, fog.planes.far_raw),
                pass: match pass {
                    NativeModelFogPass::Near => FacePass::Near,
                    NativeModelFogPass::Fog => FacePass::Fog,
                },
                colour: colour_word(fog.color),
            }),
            ModelDepthFade::FrontendFixedPointLinear {
                near_raw,
                far_raw,
                color,
            } => select(near_raw, far_raw, color),
            ModelDepthFade::Linear { near, far, color } => {
                select((near * self.units) as i32, (far * self.units) as i32, color)
            }
            ModelDepthFade::InheritWorldFog => match self.world_fog {
                Some(fog) => select(fog.planes.near_raw, fog.planes.far_raw, fog.color),
                None => Some(DrawFog {
                    fade: [0, i32::MAX, i32::MAX],
                    pass: FacePass::Near,
                    colour: 0,
                }),
            },
        }
    }
}

/// Material ids for sprite faces: the registered texture plus the render
/// flags the face carries (keyed, row 28, half-additive, additive).
pub(crate) trait ModelMaterials {
    fn face_material(&mut self, texture: u32, flags: u16) -> Option<MaterialId>;
    /// A bound material's sprite-record size.
    fn material_size(&self, id: MaterialId) -> Option<(u16, u16)>;
}

fn face_flags(blend: WorldSpriteBlend, shade_row: u8) -> u16 {
    let blend = match blend {
        WorldSpriteBlend::Masked => 0,
        WorldSpriteBlend::HalfAdditive => material_flags::HALF_ADDITIVE,
        WorldSpriteBlend::Additive => material_flags::ADDITIVE,
    };
    blend
        | if shade_row == 28 {
            material_flags::ROW_28
        } else {
            0
        }
}

/// A cache entry the projector rejected at the near plane.
const REJECTED_CORNER: ModelCorner = ModelCorner {
    view: [0; 3],
    screen: [0; 2],
    clip: 0x40 | 0x80,
    fade: 0xFF,
};

/// Project every type-1 vertex from its (recursively projected) sources
/// with `FUN_0046DC00`. Retail never writes such a vertex's VIEW X/Y; the
/// constructors read only its depth.
fn screen_midpoints(
    corners: &mut [ModelCorner],
    projections: &[ModelVertexProjection],
    bounds: [u32; 2],
) {
    fn resolve(
        index: usize,
        corners: &mut [ModelCorner],
        projections: &[ModelVertexProjection],
        bounds: [u32; 2],
        state: &mut [u8],
    ) -> ModelCorner {
        const VISITING: u8 = 1;
        const DONE: u8 = 2;
        let Some(&current) = corners.get(index) else {
            return REJECTED_CORNER;
        };
        match state[index] {
            DONE => return current,
            // Formats reject cyclic sources; fail closed regardless.
            VISITING => return REJECTED_CORNER,
            _ => {}
        }
        let Some(ModelVertexProjection::ScreenMidpoint(sources)) = projections.get(index) else {
            state[index] = DONE;
            return current;
        };
        state[index] = VISITING;
        let a = resolve(usize::from(sources[0]), corners, projections, bounds, state);
        let b = resolve(usize::from(sources[1]), corners, projections, bounds, state);
        let corner = screen_midpoint(a, b, current, bounds);
        corners[index] = corner;
        state[index] = DONE;
        corner
    }
    if !projections
        .iter()
        .any(|projection| matches!(projection, ModelVertexProjection::ScreenMidpoint(_)))
    {
        return;
    }
    let mut state = vec![0u8; corners.len()];
    for index in 0..corners.len() {
        resolve(index, corners, projections, bounds, &mut state);
    }
}

/// One model body's constructor inputs, owned so that its painter program
/// can run them after the draw returns: the warm vertex cache, normal
/// cache, palette and sprite operands, and one command per source polygon
/// and edge.
pub(crate) struct PreparedNode {
    scene: ModelScene,
    position: [f32; 3],
    orientation: [[f32; 3]; 3],
    /// Raw model units to world units.
    raw_scale: f32,
    pass: FacePass,
    fog_colour: u32,
    lens: GroundProjection,
    corners: Vec<ModelCorner>,
    normals: Vec<ModelNormal>,
    colours: Vec<u32>,
    sprites: Vec<MaterialId>,
    /// Sprite record sizes, parallel to `sprites` (edges read them).
    sizes: Vec<(u16, u16)>,
    /// The command of each source polygon, by its first triangle.
    faces: HashMap<usize, (u8, Vec<i16>)>,
    /// Polygon first triangles in mesh order.
    face_order: Vec<usize>,
    /// The command of each edge, by edge index.
    edges: Vec<Option<EdgeCommand>>,
}

impl PreparedNode {
    fn context<'s>(
        &'s self,
        palette: &'s dyn Fn(i16) -> u32,
        sprite: &'s dyn Fn(i16) -> MaterialId,
    ) -> FaceContext<'s> {
        FaceContext {
            corners: &self.corners,
            normals: &self.normals,
            palette,
            sprite,
            fog_colour: self.fog_colour,
        }
    }

    /// Run the face whose polygon starts at triangle `first`.
    pub(crate) fn emit_face(
        &self,
        queue: &mut PrimitiveQueue,
        first: usize,
    ) -> Result<(), QueueError> {
        let Some((opcode, words)) = self.faces.get(&first) else {
            return Ok(());
        };
        let palette = |index: i16| self.colours.get(index as usize).copied().unwrap_or(0);
        let sprite = |index: i16| self.sprites.get(index as usize).copied().unwrap_or(0);
        construct_face(
            queue,
            &self.context(&palette, &sprite),
            self.pass,
            *opcode,
            words,
        )?;
        Ok(())
    }

    /// Run edge `index`.
    pub(crate) fn emit_edge(
        &self,
        queue: &mut PrimitiveQueue,
        index: usize,
    ) -> Result<(), QueueError> {
        let Some(Some(command)) = self.edges.get(index) else {
            return Ok(());
        };
        let palette = |index: i16| self.colours.get(index as usize).copied().unwrap_or(0);
        let sprite = |index: i16| self.sprites.get(index as usize).copied().unwrap_or(0);
        let size = |index: i16| self.sizes.get(index as usize).copied().unwrap_or((0, 0));
        let context = self.context(&palette, &sprite);
        construct_edge(queue, &context, &self.lens, &size, self.pass, *command)
    }

    /// Every face in mesh order, then every edge: the order of a body
    /// without a painter program.
    pub(crate) fn emit_all(&self, queue: &mut PrimitiveQueue) -> Result<(), QueueError> {
        for &first in &self.face_order {
            self.emit_face(queue, first)?;
        }
        for index in 0..self.edges.len() {
            self.emit_edge(queue, index)?;
        }
        Ok(())
    }

    /// A painter group's key in VIEW depth units, or `None` for an
    /// unresolved one (the group is skipped rather than given a guess).
    pub(crate) fn group_key(&self, key: &ModelPainterDepthKey) -> Option<i32> {
        let depth = |position_raw: &[f64; 3]| -> i32 {
            let local = position_raw.map(|value| value as f32 * self.raw_scale);
            let world: [f32; 3] = std::array::from_fn(|axis| {
                self.position[axis]
                    + (0..3)
                        .map(|k| self.orientation[axis][k] * local[k])
                        .sum::<f32>()
            });
            self.scene.node_view_raw(self.position, world)[2]
        };
        Some(match key {
            ModelPainterDepthKey::Native(depth) | ModelPainterDepthKey::Fixed(depth) => *depth,
            ModelPainterDepthKey::Vertex {
                position_raw,
                offset_raw,
            } => {
                // Offsets are model units, VIEW units in gameplay.
                let offset =
                    (*offset_raw as f32 * self.raw_scale * self.scene.units).round() as i32;
                depth(position_raw).wrapping_add(offset)
            }
            ModelPainterDepthKey::Minimum(points) => {
                points.iter().map(depth).min().unwrap_or(i32::MAX)
            }
            ModelPainterDepthKey::Maximum(points) => {
                points.iter().map(depth).max().unwrap_or(-i32::MAX)
            }
            ModelPainterDepthKey::Unresolved => return None,
        })
    }
}

/// Build one body's constructor inputs, or `None` when the node lies
/// beyond the far plane or has no vertices.
pub(crate) fn prepare_model_body(
    draw: &ModelDraw<'_>,
    scene: &ModelScene,
    materials: &mut dyn ModelMaterials,
) -> Option<PreparedNode> {
    let mesh = draw.mesh;
    if mesh.vertices.is_empty() {
        return None;
    }
    let fog = scene.draw_fog(draw)?;
    let lens = GroundProjection {
        fade: fog.fade,
        ..scene.lens
    };
    // Camera-dependent vertices and back faces need the eye beside the node.
    let eye = scene.eye_near(draw.transform.position);
    let resolved = resolve_model_vertices(draw, eye, scene.camera_basis);
    let node = draw.transform.position;
    let mut corners: Vec<ModelCorner> = resolved
        .world
        .iter()
        .zip(&resolved.admitted)
        .enumerate()
        .map(|(index, (world, &admitted))| match world {
            // A native scene rejects through the native depth's 0x40
            // outcode; the floating camera may sit in another torus image.
            Some(world) if admitted || scene.native.is_some() => {
                // A source-owned VIEW point is the vertex cache itself.
                // Otherwise callback world points are projected on their own.
                let native = scene.native_vertex_view(mesh.vertex_view_raw, index);
                let view = match (native, mesh.vertex_projection.get(index)) {
                    (Some(view), _) => view,
                    (None, Some(ModelVertexProjection::WorldPoint(_))) => scene.view_raw(*world),
                    (None, _) => scene.node_view_raw(node, *world),
                };
                let point = lens.project_view(view);
                ModelCorner {
                    view,
                    screen: point.screen,
                    clip: point.clip | 0x80,
                    fade: point.fade,
                }
            }
            _ => REJECTED_CORNER,
        })
        .collect();
    screen_midpoints(&mut corners, mesh.vertex_projection, lens.bounds);

    // One command per source face: consecutive triangles of a quad share it.
    let mut normals: Vec<ModelNormal> = Vec::new();
    let mut colours: Vec<u32> = Vec::new();
    let mut sprites: Vec<MaterialId> = Vec::new();
    let mut faces: HashMap<usize, (u8, Vec<i16>)> = HashMap::new();
    let mut face_order = Vec::new();
    let mut previous: Option<(ModelFaceVertices, usize)> = None;
    // A node with its integer frame shades from the raw pool normals and
    // its model-space light; slot 0 is the face normal, 1..=3 its corners.
    let native_light = mesh.native_light_raw.filter(|_| scene.native.is_some());
    let shade_dword = |face: usize, slot: usize, normal: [f32; 3]| -> u32 {
        let raw = native_light.zip(mesh.face_normals_raw.get(face));
        match raw {
            Some((light, normals)) => {
                retail_model_shade_raw(mesh.shade_table, normals[slot], light, mesh.shade_shift)
            }
            None => retail_model_shade(
                mesh.shade_table,
                scene.view_light_normal(draw.transform.orientation, normal),
                mesh.light_direction_raw,
                mesh.shade_shift,
            ),
        }
        .map_or(0, |entry| {
            u32::from_le_bytes([entry.r, entry.g, entry.b, entry.shade_level])
        })
    };
    for (index, triangle) in mesh.triangles.iter().enumerate() {
        let source = mesh
            .face_vertices
            .get(index)
            .copied()
            .unwrap_or(ModelFaceVertices::Triangle(*triangle));
        // A quad spans exactly two triangles; a following face with the
        // same corners (a front/back pair) is a polygon of its own.
        if let Some((face, first)) = previous.take() {
            if face == source && matches!(source, ModelFaceVertices::Quad(_)) {
                // Second triangle of a quad: its last corner's normal is the
                // quad's fourth Gouraud reference.
                if let (ModelFaceShading::Gouraud, Some(corner)) = (
                    mesh.face_shading
                        .get(first)
                        .copied()
                        .unwrap_or(ModelFaceShading::Flat),
                    mesh.face_corner_normals.get(index),
                ) {
                    if let Some((_, words)) = faces.get_mut(&first) {
                        let at = words.len() - 1;
                        let reference = normals.len() as i16;
                        normals.push(ModelNormal {
                            shade: shade_dword(index, 3, corner[2]),
                            culled: false,
                        });
                        words[at] = reference;
                    }
                }
                continue;
            }
        }
        previous = Some((source, index));
        let shading = mesh
            .face_shading
            .get(index)
            .copied()
            .unwrap_or(ModelFaceShading::Flat);
        let material = mesh.materials.get(index);
        let texture = material.and_then(|material| material.texture);
        let culled = mesh.face_cull.get(index).is_some_and(|cull| {
            matches!(cull, ModelFaceCull::Plane(plane) if !authored_face_plane_visible(
                *plane,
                draw.transform.orientation,
                draw.transform.position,
                draw.transform.scale,
                eye,
            ))
        });
        let face_normal = mesh.normals.get(index).copied().unwrap_or([0.0, 1.0, 0.0]);
        let corner_normals = mesh
            .face_corner_normals
            .get(index)
            .copied()
            .unwrap_or([face_normal; 3]);
        let lighting = model_lighting_normals(shading, face_normal, corner_normals);
        let normal_index = normals.len() as i16;
        // FlatLit reads the face normal, Gouraud its first corner.
        let first_slot = usize::from(matches!(shading, ModelFaceShading::Gouraud));
        normals.push(ModelNormal {
            shade: lighting.map_or(0, |normals| shade_dword(index, first_slot, normals[0])),
            culled,
        });
        let vertices: Vec<i16> = match source {
            ModelFaceVertices::Triangle(v) => v.iter().map(|&v| v as i16).collect(),
            ModelFaceVertices::Quad(v) => v.iter().map(|&v| v as i16).collect(),
        };
        let mut opcode = if vertices.len() == 4 { 0x04 } else { 0x03 };
        let material_index = match (texture, material) {
            (Some(texture), Some(material)) => {
                let flags = face_flags(material.blend, material.flat_shade_row);
                let Some(id) = materials.face_material(texture.0, flags) else {
                    continue;
                };
                opcode |= 0x80;
                sprites.push(id);
                sprites.len() as i16 - 1
            }
            (None, Some(material)) => {
                let colour = material.palette_rgb555.map_or(0, |rgb555| {
                    u32::from(((rgb555 & 0x7FE0) << 1) | (rgb555 & 0x1F))
                });
                colours.push(colour);
                colours.len() as i16 - 1
            }
            _ => {
                colours.push(0);
                colours.len() as i16 - 1
            }
        };
        let mut words = vec![material_index, normal_index];
        words.extend_from_slice(&vertices);
        match shading {
            ModelFaceShading::Flat => {}
            ModelFaceShading::FlatLit => opcode |= 0x40,
            ModelFaceShading::Gouraud => {
                opcode |= 0x20;
                let normals_for = lighting.unwrap_or([face_normal; 3]);
                for corner in 0..vertices.len() {
                    let reference = normals.len() as i16;
                    let normal = normals_for[corner.min(2)];
                    normals.push(ModelNormal {
                        shade: shade_dword(index, 1 + corner.min(2), normal),
                        culled: false,
                    });
                    words.push(reference);
                }
            }
        }
        faces.insert(index, (opcode, words));
        face_order.push(index);
    }

    // Native endpoint snapshots carry their own VIEW points.
    let snapshots = match (mesh.edge_projection, scene.native) {
        (ModelEdgeProjection::CommandSnapshots(snapshots), Some(_)) => snapshots,
        _ => &[],
    };
    let raw_scale = draw.transform.scale / 100.0;
    let mut sizes = vec![(0u16, 0u16); sprites.len()];
    let mut edges = Vec::with_capacity(mesh.edges.len());
    for (index, edge) in mesh.edges.iter().enumerate() {
        edges.push(None);
        let mut vertices = edge.vertices.map(usize::from);
        match snapshots.get(index) {
            None | Some(ModelEdgeEndpointSnapshot::Compatibility) => {}
            Some(ModelEdgeEndpointSnapshot::Native { endpoints }) => {
                for (end, endpoint) in endpoints.iter().enumerate() {
                    vertices[end] = corners.len();
                    corners.push(if endpoint.clip == ModelSlotClip::Clear {
                        let point = lens.project_view(endpoint.view_raw);
                        ModelCorner {
                            view: endpoint.view_raw,
                            screen: point.screen,
                            clip: point.clip | 0x80,
                            fade: point.fade,
                        }
                    } else {
                        REJECTED_CORNER
                    });
                }
            }
            // Retail allocated nothing, or the native producer failed.
            Some(_) => continue,
        }
        let material = mesh.edge_materials.get(index);
        let command = match edge.style {
            ModelEdgeStyle::Palette { .. } => {
                colours.push(
                    material
                        .and_then(|material| material.palette_rgb555)
                        .map_or(0, |rgb555| {
                            u32::from(((rgb555 & 0x7FE0) << 1) | (rgb555 & 0x1F))
                        }),
                );
                EdgeCommand::Line {
                    colour: colours.len() as i16 - 1,
                    vertices,
                }
            }
            ModelEdgeStyle::Sprite { size, .. } => {
                let Some((texture, material)) = material
                    .and_then(|material| material.texture.map(|texture| (texture, material)))
                else {
                    continue;
                };
                let flags = face_flags(material.blend, material.flat_shade_row);
                let Some(id) = materials.face_material(texture.0, flags) else {
                    continue;
                };
                sprites.push(id);
                sizes.push(mesh.edge_widths.get(index).copied().unwrap_or((0, 0)));
                // Retail sizes are VIEW units; see the billboards.
                let size = (f32::from(size as i16) * raw_scale * scene.units).round() as i32 as i16;
                EdgeCommand::Ribbon {
                    sprite: sprites.len() as i16 - 1,
                    size,
                    vertices,
                }
            }
        };
        edges[index] = Some(command);
    }

    Some(PreparedNode {
        scene: *scene,
        position: draw.transform.position,
        orientation: draw.transform.orientation,
        raw_scale,
        pass: fog.pass,
        fog_colour: fog.colour,
        lens,
        corners,
        normals,
        colours,
        sprites,
        sizes,
        faces,
        face_order,
        edges,
    })
}

/// One body's billboards, owned like [`PreparedNode`].
pub(crate) struct PreparedBillboards {
    pass: FacePass,
    fog_colour: u32,
    lens: GroundProjection,
    corners: Vec<ModelCorner>,
    colours: Vec<u32>,
    sprites: Vec<MaterialId>,
    sizes: Vec<(u16, u16)>,
    /// The command of each billboard, by billboard index.
    commands: Vec<Option<BillboardCommand>>,
}

impl PreparedBillboards {
    /// Run billboard `index`.
    pub(crate) fn emit(&self, queue: &mut PrimitiveQueue, index: usize) -> Result<(), QueueError> {
        let Some(Some(command)) = self.commands.get(index) else {
            return Ok(());
        };
        if command.vertex >= self.corners.len() {
            return Ok(());
        }
        let palette = |index: i16| self.colours.get(index as usize).copied().unwrap_or(0);
        let sprite = |index: i16| self.sprites.get(index as usize).copied().unwrap_or(0);
        let size = |index: i16| self.sizes.get(index as usize).copied().unwrap_or((1, 1));
        let context = FaceContext {
            corners: &self.corners,
            normals: &[],
            palette: &palette,
            sprite: &sprite,
            fog_colour: self.fog_colour,
        };
        construct_billboard(queue, &context, &self.lens, &size, self.pass, *command)
    }

    pub(crate) fn emit_all(&self, queue: &mut PrimitiveQueue) -> Result<(), QueueError> {
        for index in 0..self.commands.len() {
            self.emit(queue, index)?;
        }
        Ok(())
    }
}

/// Build one body's billboard constructor inputs.
pub(crate) fn prepare_model_billboards(
    draw: &ModelBillboardDraw<'_>,
    scene: &ModelScene,
    materials: &mut dyn ModelMaterials,
) -> Option<PreparedBillboards> {
    if draw.vertices.is_empty() || draw.billboards.is_empty() {
        return None;
    }
    let fog = scene.node_fog(
        draw.transform.position,
        draw.radius_raw,
        draw.transform.scale,
        draw.depth_fade,
    )?;
    let lens = GroundProjection {
        fade: fog.fade,
        ..scene.lens
    };
    let raw_scale = draw.transform.scale / 100.0;
    let world: Vec<Option<[f32; 3]>> = draw
        .vertices
        .iter()
        .enumerate()
        .map(|(index, anchor)| {
            if draw
                .vertex_clip
                .get(index)
                .is_some_and(|clip| *clip != v2k_formats::models::ModelSlotClip::Clear)
            {
                return None;
            }
            let local = anchor.map(|value| value as f32 * raw_scale);
            Some(std::array::from_fn(|axis| {
                draw.transform.position[axis]
                    + (0..3)
                        .map(|k| draw.transform.orientation[axis][k] * local[k])
                        .sum::<f32>()
            }))
        })
        .collect();
    let admitted = crate::model_near::vertex_admission(
        draw.near_clip,
        &world,
        draw.vertex_projection,
        scene.camera_position,
        scene.camera_basis[2],
    );
    let node = draw.transform.position;
    let mut corners: Vec<ModelCorner> = world
        .iter()
        .zip(&admitted)
        .enumerate()
        .map(|(index, (world, &admitted))| match world {
            Some(world) if admitted || scene.native.is_some() => {
                let view = scene
                    .native_vertex_view(draw.vertex_view_raw, index)
                    .unwrap_or_else(|| scene.node_view_raw(node, *world));
                let point = lens.project_view(view);
                ModelCorner {
                    view,
                    screen: point.screen,
                    clip: point.clip | 0x80,
                    fade: point.fade,
                }
            }
            _ => REJECTED_CORNER,
        })
        .collect();
    screen_midpoints(&mut corners, draw.vertex_projection, lens.bounds);
    let mut colours = Vec::new();
    let mut sprites = Vec::new();
    let mut sizes = Vec::new();
    let mut commands = Vec::with_capacity(draw.billboards.len());
    for (billboard, material) in draw.billboards.iter().zip(draw.materials) {
        commands.push(None);
        let id = if billboard.textured {
            let Some(texture) = material.face.texture else {
                continue;
            };
            let flags = face_flags(material.blend, material.face.flat_shade_row);
            let Some(id) = materials.face_material(texture.0, flags) else {
                continue;
            };
            sprites.push(id);
            sizes.push((material.width, material.height));
            sprites.len() as i16 - 1
        } else {
            let colour = material.face.palette_rgb555.map_or(0, |rgb555| {
                u32::from(((rgb555 & 0x7FE0) << 1) | (rgb555 & 0x1F))
            });
            colours.push(colour);
            colours.len() as i16 - 1
        };
        // Model units are VIEW units in gameplay (scale 100/256); other
        // scenes convert their model units to the VIEW domain.
        let size = (f32::from(billboard.size) * raw_scale * scene.units).round() as i32;
        if let Some(slot) = commands.last_mut() {
            *slot = Some(BillboardCommand {
                vertex: usize::from(billboard.vertex),
                id,
                size,
                angle: billboard.angle,
                textured: billboard.textured,
            });
        }
    }
    Some(PreparedBillboards {
        pass: fog.pass,
        fog_colour: fog.colour,
        lens,
        corners,
        colours,
        sprites,
        sizes,
        commands,
    })
}

/// `FUN_0043D410` for one particle: the scene's projector with the
/// particle fog planes as its fade ramp.
fn queue_native_particle(
    queue: &mut PrimitiveQueue,
    base: &ParticleScene,
    native: crate::renderer::NativeParticle,
    sprite: GroundMaterial,
) -> Result<(), QueueError> {
    let mut scene = *base;
    scene.fog_near = native.fog_near_raw;
    scene.far = native.fog_far_raw;
    scene.projection.fade = fade_terms(native.fog_near_raw, native.fog_far_raw);
    let particle = Particle {
        position: native.position_raw,
        scale: native.scale_raw,
        sprite,
        flags: native.flags,
        sort_bias: native.sort_bias_raw,
        frame_size: native.frame_size_raw,
        shadow: native.shadow.map(|shadow| ParticleShadow {
            size: shadow.size,
            ground: shadow.ground_raw,
            colour: shadow.colour,
        }),
    };
    queue_particle(queue, &scene, &particle).map(|_| ())
}

/// Derived sprite materials, keyed by texture and face flags.
#[derive(Default)]
pub(crate) struct DerivedMaterials {
    pub(crate) by_flags: HashMap<(u32, u16), MaterialId>,
}

impl DerivedMaterials {
    /// Drop what was derived from `base` once its slot is freed: the store
    /// reuses ids, and a later material in that slot must not inherit them.
    pub(crate) fn forget(&mut self, base: MaterialId, store: &mut MaterialStore) {
        self.by_flags.retain(|&(texture, _), &mut derived| {
            if texture != base {
                return true;
            }
            if derived != base {
                store.remove(derived);
            }
            false
        });
    }
}

/// Queue camera-facing world sprites. Particles that carry their
/// `FUN_0043D410` inputs go through the ported drawer when the scene has a
/// native context (`particles`). Any other sprite is approximated by an
/// axis-aligned textured quad around its projected centre, sized from its
/// world extent and keyed by its painter key, `+0x1098` near or `+0x109C`
/// with its far fade byte.
pub(crate) fn queue_world_sprites(
    queue: &mut PrimitiveQueue,
    sprites: &[crate::renderer::WorldSprite],
    scene: &ModelScene,
    particles: Option<&ParticleScene>,
    materials: &mut dyn ModelMaterials,
) -> Result<(), QueueError> {
    use crate::renderer::SpriteFog;
    use crate::software::FillSlot;
    let fog_colour = scene.world_fog.map_or(0, |fog| colour_word(fog.color));
    for sprite in sprites {
        if let (Some(native), Some(base)) = (sprite.native, particles) {
            let flags = face_flags(sprite.blend, sprite.flat_shade_row);
            let Some(id) = materials.face_material(sprite.texture.0, flags) else {
                continue;
            };
            let (width, height) = materials.material_size(id).unwrap_or((1, 1));
            queue_native_particle(queue, base, native, GroundMaterial { id, width, height })?;
            continue;
        }
        if !sprite.position.iter().all(|v| v.is_finite())
            || !sprite.size.iter().all(|v| v.is_finite() && *v > 0.0)
        {
            continue;
        }
        let view = scene.view_raw(sprite.position);
        let centre = scene.lens.project_view(view);
        if centre.clip & 0x40 != 0 {
            continue;
        }
        let depth = view[2] as f32;
        let half = |extent: f32, focal: i32| -> i32 {
            ((extent * 0.5 * scene.units * focal as f32 / depth) as i32).max(1)
        };
        let hx = half(sprite.size[0], scene.lens.focal[0]);
        let hy = half(sprite.size[1], scene.lens.focal[1]);
        let [cx, cy] = centre.screen.map(i32::from);
        let corners = [
            (cx - hx, cy - hy),
            (cx + hx, cy - hy),
            (cx + hx, cy + hy),
            (cx - hx, cy + hy),
        ];
        let outcode = corners.iter().fold(0u8, |code, &(x, y)| {
            code | crate::software::terrain::outcode_of(x, y, scene.lens.bounds)
        });
        if crate::software::terrain::OUTCODE_VISIBLE[usize::from(outcode)] == 0 {
            continue;
        }
        let flags = face_flags(sprite.blend, sprite.flat_shade_row);
        let Some(material) = materials.face_material(sprite.texture.0, flags) else {
            continue;
        };
        let (slot, bytes, fade) = match sprite.fog {
            SpriteFog::Near => (FillSlot::TexturedQuad, 0x18, None),
            SpriteFog::Far { fade_byte } => (FillSlot::TexturedFogQuad, 0x20, Some(fade_byte)),
        };
        let payload = queue.push(sprite.sort_key_raw, slot, bytes)?;
        for (index, (x, y)) in corners.into_iter().enumerate() {
            payload[4 * index..4 * index + 2].copy_from_slice(&(x as i16).to_le_bytes());
            payload[4 * index + 2..4 * index + 4].copy_from_slice(&(y as i16).to_le_bytes());
        }
        payload[0x10..0x14].copy_from_slice(&material.to_le_bytes());
        payload[0x14..0x18].copy_from_slice(&0u32.to_le_bytes());
        if let Some(fade) = fade {
            payload[0x18..0x1C].copy_from_slice(&fog_colour.to_le_bytes());
            payload[0x1C..0x20].fill(fade);
        }
    }
    Ok(())
}

#[cfg(test)]
impl PreparedNode {
    /// A node of on-screen flat triangles, one per `(first triangle, colour,
    /// depth)`: its constructor packet carries the colour, and the corners'
    /// depth keys it.
    pub(crate) fn test_triangles(faces: &[(usize, u32, i32)]) -> Self {
        let lens = GroundProjection {
            axes_q31: [[0; 3]; 3],
            translation: [0; 3],
            focal: [256, 256],
            bounds: [640, 480],
            centre: [320, 240],
            fade: [0, i32::MAX, i32::MAX],
            wet_clock: None,
        };
        let mut corners = Vec::new();
        let mut colours = Vec::new();
        let mut map = HashMap::new();
        let mut order = Vec::new();
        for &(first, colour, depth) in faces {
            let base = corners.len() as i16;
            for screen in [[10, 10], [100, 10], [10, 100]] {
                corners.push(ModelCorner {
                    view: [0, 0, depth],
                    screen,
                    clip: 0x12 | 0x80,
                    fade: 0,
                });
            }
            colours.push(colour);
            let words = vec![colours.len() as i16 - 1, 0, base, base + 1, base + 2];
            map.insert(first, (0x03, words));
            order.push(first);
        }
        Self {
            scene: ModelScene {
                camera_position: [0.0; 3],
                camera_basis: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                native: None,
                lens,
                world_fog: None,
                units: 256.0,
            },
            position: [0.0; 3],
            orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            raw_scale: 1.0 / 256.0,
            pass: FacePass::Near,
            fog_colour: 0,
            lens,
            corners,
            normals: vec![ModelNormal {
                shade: 0,
                culled: false,
            }],
            colours,
            sprites: Vec::new(),
            sizes: Vec::new(),
            faces: map,
            face_order: order,
            edges: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::{
        ExternalFrameMode, FaceMaterial, ModelDepthPolicy, ModelMesh, ModelOverlayKind,
        ModelSurfaceResolution, ModelTransform, ViewPinMode,
    };
    use v2k_formats::models::ModelEntry;

    struct NoSprites;

    impl ModelMaterials for NoSprites {
        fn face_material(&mut self, _texture: u32, _flags: u16) -> Option<MaterialId> {
            None
        }
        fn material_size(&self, _id: MaterialId) -> Option<(u16, u16)> {
            None
        }
    }

    #[test]
    fn a_front_back_quad_pair_stays_two_faces() {
        let quad = ModelFaceVertices::Quad([0, 1, 2, 3]);
        let model = ModelEntry {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [256.0, 0.0, 0.0],
                [256.0, 256.0, 0.0],
                [0.0, 256.0, 0.0],
            ],
            vertex_type_flags: vec![0; 4],
            vertex_projection: vec![ModelVertexProjection::Position; 4],
            vertex_clip: vec![ModelSlotClip::Clear; 4],
            triangles: vec![[0, 1, 2], [0, 2, 3], [0, 1, 2], [0, 2, 3]],
            face_vertices: vec![quad; 4],
            normals: vec![[0.0, 0.0, 1.0]; 4],
            face_shading: vec![ModelFaceShading::Flat; 4],
            ..ModelEntry::default()
        };
        let materials = vec![
            FaceMaterial {
                color: [1.0; 3],
                palette_rgb555: Some(0x7FFF),
                emissive: [0.0; 3],
                texture: None,
                blend: WorldSpriteBlend::Masked,
                flat_shade_row: 28,
            };
            4
        ];
        let draw = ModelDraw {
            mesh: ModelMesh::from_model(&model, &materials),
            projection_authority: Default::default(),
            transform: ModelTransform {
                orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                position: [0.0, 0.0, -10.0],
                scale: 100.0 / 256.0,
            },
            near_clip: ModelNearClip::Camera,
            depth_fade: ModelDepthFade::Disabled,
            depth_policy: ModelDepthPolicy::Geometry,
            view_pin: ViewPinMode::Raw,
            surface_resolution: ModelSurfaceResolution::Intrinsic,
            world_surface: None,
            external_frame: ExternalFrameMode::Raw,
            overlay: ModelOverlayKind::None,
            painter: None,
        };
        let scene = PreparedNode::test_triangles(&[]).scene;
        let node = prepare_model_body(&draw, &scene, &mut NoSprites).unwrap();
        assert_eq!(node.face_order, [0, 2], "the back face is its own polygon");
    }

    #[test]
    fn a_native_scene_takes_the_producers_view_points() {
        let model = ModelEntry {
            vertices: vec![[0.0, 0.0, 0.0], [256.0, 0.0, 0.0], [0.0, 256.0, 0.0]],
            vertex_type_flags: vec![0; 3],
            vertex_projection: vec![ModelVertexProjection::Position; 3],
            vertex_clip: vec![ModelSlotClip::Clear; 3],
            triangles: vec![[0, 1, 2]],
            face_vertices: vec![ModelFaceVertices::Triangle([0, 1, 2])],
            normals: vec![[0.0, 0.0, 1.0]],
            face_shading: vec![ModelFaceShading::Flat],
            ..ModelEntry::default()
        };
        let materials = [FaceMaterial {
            color: [1.0; 3],
            palette_rgb555: Some(0x7FFF),
            emissive: [0.0; 3],
            texture: None,
            blend: WorldSpriteBlend::Masked,
            flat_shade_row: 28,
        }];
        let views = [Some([-100, 50, 2_000]), None, Some([300, -20, 2_400])];
        let mut mesh = ModelMesh::from_model(&model, &materials);
        mesh.vertex_view_raw = &views;
        let position = [4.0, 0.0, 8.0];
        let draw = ModelDraw {
            mesh,
            projection_authority: Default::default(),
            transform: ModelTransform {
                orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                position,
                scale: 100.0 / 256.0,
            },
            near_clip: ModelNearClip::RetailWorld,
            depth_fade: ModelDepthFade::Disabled,
            depth_policy: ModelDepthPolicy::Geometry,
            view_pin: ViewPinMode::Raw,
            surface_resolution: ModelSurfaceResolution::Intrinsic,
            world_surface: None,
            external_frame: ExternalFrameMode::Raw,
            overlay: ModelOverlayKind::None,
            painter: None,
        };
        let mut scene = PreparedNode::test_triangles(&[]).scene;
        let float = prepare_model_body(&draw, &scene, &mut NoSprites).unwrap();
        assert_ne!(float.corners[0].view, [-100, 50, 2_000]);
        scene.native = Some(NativeViewportWords {
            origin_raw: [0; 3],
            axes_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
        });
        let native = prepare_model_body(&draw, &scene, &mut NoSprites).unwrap();
        assert_eq!(native.corners[0].view, [-100, 50, 2_000]);
        assert_eq!(native.corners[2].view, [300, -20, 2_400]);
        // A vertex the producer does not own keeps the transformed point.
        assert_eq!(
            native.corners[1].view,
            scene.node_view_raw(position, [5.0, 0.0, 8.0])
        );
    }
}
