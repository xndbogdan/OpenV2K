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

use v2k_formats::models::{ModelFaceCull, ModelFaceShading, ModelFaceVertices};

use crate::gl_backend::{
    authored_face_plane_visible, model_lighting_normals, resolve_model_vertices,
    retail_model_shade, transform_model_light_normal,
};
use crate::projection::NativeViewportWords;
use crate::renderer::WorldSpriteBlend;
use crate::renderer::{ModelDepthFade, ModelDraw, NativeModelFogPass, WorldModelFog};
use crate::software::model::{construct_face, FaceContext, FacePass, ModelCorner, ModelNormal};
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
    /// raw delta when the scene has one, else the floating camera.
    fn view_raw(&self, world: [f32; 3]) -> [i32; 3] {
        if let Some(native) = self.native {
            let delta: [i32; 3] = std::array::from_fn(|axis| {
                ((world[axis] * 256.0).round() as i32).wrapping_sub(native.origin_raw[axis])
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
        let origin_depth = self.view_raw(draw.transform.position)[2];
        let radius =
            (f32::from(draw.mesh.radius_raw) * draw.transform.scale.abs() / 100.0 * 256.0) as i32;
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
        match draw.depth_fade {
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
    let corners: Vec<ModelCorner> = resolved
        .world
        .iter()
        .zip(&resolved.admitted)
        .map(|(world, &admitted)| match world {
            Some(world) if admitted => {
                let view = scene.view_raw(*world);
                let point = lens.project_view(view);
                ModelCorner {
                    view,
                    screen: point.screen,
                    clip: point.clip | 0x80,
                    fade: point.fade,
                }
            }
            _ => ModelCorner {
                clip: 0x40 | 0x80,
                fade: 0xFF,
                ..ModelCorner::default()
            },
        })
        .collect();

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

    let palette = |index: i16| colours.get(index as usize).copied().unwrap_or(0);
    let sprite = |index: i16| sprites.get(index as usize).copied().unwrap_or(0);
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
    Ok(())
}

/// Derived sprite materials, keyed by texture and face flags.
#[derive(Default)]
pub(crate) struct DerivedMaterials {
    pub(crate) by_flags: HashMap<(u32, u16), MaterialId>,
}
