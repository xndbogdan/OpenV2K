//! Software backend adapter: queue a materialized model body through the
//! retail face constructors (`crate::software::model`).
//!
//! Retail executes the model's command stream against a vertex cache of
//! VIEW points it projects with the node's axes. The port materializes the
//! stream instead, so this adapter rebuilds the constructors' inputs from a
//! [`ModelDraw`]: each callback-resolved world point becomes a cache entry
//! (VIEW from the scene's native viewport when it has one, else from the
//! floating camera), each face's lighting becomes a normal-cache shade
//! dword, and each face becomes the command the constructor tables would
//! have run. The constructors themselves are byte-exact; VIEW points of
//! world-resolved vertices are only as exact as the materializer's world
//! points.

use std::collections::HashMap;

use v2k_formats::models::{
    ModelEdgeEndpointSnapshot, ModelEdgeStyle, ModelFaceCull, ModelFaceShading, ModelFaceVertices,
    ModelSlotClip, ModelVertexProjection,
};

use crate::gl_backend::{
    authored_face_plane_visible, model_lighting_normals, resolve_model_vertices,
    retail_model_shade, transform_model_light_normal,
};
use crate::projection::NativeViewportWords;
use crate::renderer::WorldSpriteBlend;
use crate::renderer::{
    ModelBillboardDraw, ModelDepthFade, ModelDraw, ModelEdgeProjection, NativeModelFogPass,
    WorldModelFog,
};
use crate::software::model::{
    construct_billboard, construct_edge, construct_face, screen_midpoint, BillboardCommand,
    EdgeCommand, FaceContext, FacePass, ModelCorner, ModelNormal,
};
use crate::software::terrain::GroundProjection;
use crate::software::{material_flags, MaterialId, PrimitiveQueue, QueueError};

/// The scene a model body is queued into.
pub(crate) struct ModelScene {
    pub camera_position: [f32; 3],
    /// GL view rows (x right, y up, z toward the viewer).
    pub camera_basis: [[f32; 3]; 3],
    pub native: Option<NativeViewportWords>,
    /// The lens with identity axes; fade terms are per draw.
    pub lens: GroundProjection,
    pub world_fog: Option<WorldModelFog>,
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
            (dot(self.camera_basis[0]) * 256.0).round() as i32,
            (dot(self.camera_basis[1]) * 256.0).round() as i32,
            (-dot(self.camera_basis[2]) * 256.0).round() as i32,
        ]
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
        let radius = (f32::from(radius_raw) * scale.abs() / 100.0 * 256.0) as i32;
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
                select((near * 256.0) as i32, (far * 256.0) as i32, color)
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

/// Queue the faces of one model body.
pub(crate) fn queue_model_body(
    queue: &mut PrimitiveQueue,
    draw: &ModelDraw<'_>,
    scene: &ModelScene,
    materials: &mut dyn ModelMaterials,
) -> Result<(), QueueError> {
    let mesh = draw.mesh;
    if mesh.vertices.is_empty() {
        return Ok(());
    }
    let Some(fog) = scene.draw_fog(draw) else {
        return Ok(());
    };
    let lens = GroundProjection {
        fade: fog.fade,
        ..scene.lens
    };
    let resolved = resolve_model_vertices(draw, scene.camera_position, scene.camera_basis);
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
                // Callback world points are projected on their own.
                let view = match mesh.vertex_projection.get(index) {
                    Some(ModelVertexProjection::WorldPoint(_)) => scene.view_raw(*world),
                    _ => scene.node_view_raw(node, *world),
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
    let mut commands: Vec<(u8, Vec<i16>)> = Vec::new();
    let mut previous: Option<(ModelFaceVertices, usize)> = None;
    let shade_dword = |normal: [f32; 3]| -> u32 {
        retail_model_shade(
            mesh.shade_table,
            transform_model_light_normal(draw.transform.orientation, normal),
            mesh.light_direction_raw,
            mesh.shade_shift,
        )
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
        if let Some((face, first)) = previous {
            if face == source {
                // Second triangle of a quad: its last corner's normal is the
                // quad's fourth Gouraud reference.
                if let (ModelFaceShading::Gouraud, Some(corner)) = (
                    mesh.face_shading
                        .get(first)
                        .copied()
                        .unwrap_or(ModelFaceShading::Flat),
                    mesh.face_corner_normals.get(index),
                ) {
                    if let Some((_, words)) = commands.last_mut() {
                        let at = words.len() - 1;
                        let reference = normals.len() as i16;
                        normals.push(ModelNormal {
                            shade: shade_dword(corner[2]),
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
                scene.camera_position,
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
        normals.push(ModelNormal {
            shade: lighting.map_or(0, |normals| shade_dword(normals[0])),
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
                        shade: shade_dword(normal),
                        culled: false,
                    });
                    words.push(reference);
                }
            }
        }
        commands.push((opcode, words));
    }

    // Edges follow the faces: the materialized mesh keeps no interleaving.
    // Native endpoint snapshots carry their own VIEW points.
    let snapshots = match (mesh.edge_projection, scene.native) {
        (ModelEdgeProjection::CommandSnapshots(snapshots), Some(_)) => snapshots,
        _ => &[],
    };
    let raw_scale = draw.transform.scale / 100.0;
    let mut sizes = vec![(0u16, 0u16); sprites.len()];
    let mut edges = Vec::new();
    for (index, edge) in mesh.edges.iter().enumerate() {
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
                let size = (f32::from(size as i16) * raw_scale * 256.0).round() as i32 as i16;
                EdgeCommand::Ribbon {
                    sprite: sprites.len() as i16 - 1,
                    size,
                    vertices,
                }
            }
        };
        edges.push(command);
    }

    let palette = |index: i16| colours.get(index as usize).copied().unwrap_or(0);
    let sprite = |index: i16| sprites.get(index as usize).copied().unwrap_or(0);
    let sprite_size = |index: i16| sizes.get(index as usize).copied().unwrap_or((0, 0));
    let context = FaceContext {
        corners: &corners,
        normals: &normals,
        palette: &palette,
        sprite: &sprite,
        fog_colour: fog.colour,
    };
    for (opcode, words) in &commands {
        construct_face(queue, &context, fog.pass, *opcode, words)?;
    }
    for command in edges {
        construct_edge(queue, &context, &lens, &sprite_size, fog.pass, command)?;
    }
    Ok(())
}

/// Queue the billboards attached to one model body.
pub(crate) fn queue_model_billboards(
    queue: &mut PrimitiveQueue,
    draw: &ModelBillboardDraw<'_>,
    scene: &ModelScene,
    materials: &mut dyn ModelMaterials,
) -> Result<(), QueueError> {
    if draw.vertices.is_empty() || draw.billboards.is_empty() {
        return Ok(());
    }
    let Some(fog) = scene.node_fog(
        draw.transform.position,
        draw.radius_raw,
        draw.transform.scale,
        draw.depth_fade,
    ) else {
        return Ok(());
    };
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
        .map(|(world, &admitted)| match world {
            Some(world) if admitted || scene.native.is_some() => {
                let view = scene.node_view_raw(node, *world);
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
    let mut commands = Vec::new();
    for (billboard, material) in draw.billboards.iter().zip(draw.materials) {
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
        let size = (f32::from(billboard.size) * raw_scale * 256.0).round() as i32;
        commands.push(BillboardCommand {
            vertex: usize::from(billboard.vertex),
            id,
            size,
            angle: billboard.angle,
            textured: billboard.textured,
        });
    }
    let palette = |index: i16| colours.get(index as usize).copied().unwrap_or(0);
    let sprite = |index: i16| sprites.get(index as usize).copied().unwrap_or(0);
    let sprite_size = |index: i16| sizes.get(index as usize).copied().unwrap_or((1, 1));
    let context = FaceContext {
        corners: &corners,
        normals: &[],
        palette: &palette,
        sprite: &sprite,
        fog_colour: fog.colour,
    };
    for command in commands {
        if command.vertex < corners.len() {
            construct_billboard(queue, &context, &lens, &sprite_size, fog.pass, command)?;
        }
    }
    Ok(())
}

/// Derived sprite materials, keyed by texture and face flags.
#[derive(Default)]
pub(crate) struct DerivedMaterials {
    pub(crate) by_flags: HashMap<(u32, u16), MaterialId>,
}

/// Queue camera-facing world sprites (`FUN_0043D410`'s particle quads):
/// an axis-aligned textured quad around the projected centre, keyed by the
/// sprite's retail painter key, `+0x1098` near or `+0x109C` with the
/// sprite's far fade byte. The port supplies sprites after its own
/// presentation choices, so sizes come from the sprite's world extent.
pub(crate) fn queue_world_sprites(
    queue: &mut PrimitiveQueue,
    sprites: &[crate::renderer::WorldSprite],
    scene: &ModelScene,
    materials: &mut dyn ModelMaterials,
) -> Result<(), QueueError> {
    use crate::renderer::SpriteFog;
    use crate::software::FillSlot;
    let fog_colour = scene.world_fog.map_or(0, |fog| colour_word(fog.color));
    for sprite in sprites {
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
            ((extent * 0.5 * 256.0 * focal as f32 / depth) as i32).max(1)
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
