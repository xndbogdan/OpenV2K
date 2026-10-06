//! Exact detached core of the retail Section-12 Sub-H external-frame runtime.
//!
//! `FUN_0041D0A0` advances every live 0x3C-byte record once per caller update,
//! rotating only the first record processed. `FUN_0041D120` owns the phase and
//! dependency state reproduced here. `FUN_0041D360` owns the paired endpoint
//! geometry recovered below. Draw-time type-0 slot lookup (`slot >> 1`, odd
//! slots mirror X) is the bounded `FUN_0041e310` adapter; gameplay type-0
//! records are already 8.8. Other type flags stay fail-closed.

use std::fmt;

use crate::actor_emitter_external_frame::ActorEmitterExternalFramePresentation;
use crate::actor_external_frame_fallback::ActorExternalFrameFallback;

use v2k_formats::collision::{SubHExternalFrameDescriptor, SubHExternalFrameRecord};
use v2k_formats::fixed_math::{retail_integer_sqrt, retail_sine_q15};
use v2k_formats::terrain::{wave_surface_raw, TerrainGrid};

pub const MAX_SUB_H_RECORDS: usize = 16;
pub const COMPLETION_SOUND_VOLUME_16_16: u32 = 0x1_0000;

const COPY_TARGET_ON_COMPLETION: u32 = 0x02;
const WAIT_FOR_DEPENDENCIES: u32 = 0x08;
const TRANSIENT_STATE_MASK: u32 = 0x1e;
const LENGTH_CACHE_VALID: u32 = 0x01;
const PRIMARY_CACHE_VALID: u32 = 0x02;
const SECONDARY_CACHE_VALID: u32 = 0x04;
const TERRAIN_OFFSET_VALID: u32 = 0x08;
const PREVIOUSLY_ENABLED: u32 = 0x10;

/// Surface selection retained at Sub-H outer `+0x0C`, independently of the
/// animation-enabled word at `+0x08`. `1D2A0` constructs terrain-only; `09A80`
/// calls `1E4A0(..., 1)` when the authored Sub-C `+0x0C` byte is nonzero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubHSurfacePolicy {
    Terrain,
    TerrainAndWater,
}

/// The common surface input to D360's settled foot and each DF20 probe.
/// `waves_enabled` is retail `4FECE4`, not the renderer's water-plane gate.
#[derive(Debug, Clone, Copy)]
pub struct SubHSurfaceSampler<'a> {
    pub terrain: &'a TerrainGrid,
    pub retail_tick: u32,
    pub waves_enabled: bool,
    pub policy: SubHSurfacePolicy,
}

impl SubHSurfaceSampler<'_> {
    pub fn sample_raw(self, x_raw: i16, z_raw: i16) -> i16 {
        let floor = self.terrain.bilinear_height_raw(x_raw, z_raw);
        match self.policy {
            SubHSurfacePolicy::Terrain => floor,
            SubHSurfacePolicy::TerrainAndWater => {
                let sea = self.terrain.sea_level_raw();
                if self.waves_enabled {
                    // 445920 plus D360's signed-word floor clamp; DF20
                    // inlines the same three-wave arithmetic and clamp.
                    wave_surface_raw(x_raw, z_raw, self.retail_tick as i32, sea, floor)
                } else {
                    // Disabled waves retain the static sea plane. Neither
                    // D360 nor DF20 tests TerrainGrid::water_enabled().
                    floor.max(sea)
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubHEndpoint {
    Primary,
    Secondary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubHSelectorBinding {
    pub record_index: usize,
    pub endpoint: SubHEndpoint,
}

/// Type-0 `FUN_0041e310` slot lookup. Odd slots take model-space `(-x, y, z)`.
///
/// Gameplay Section-8 type-0 records already live in the signed 8.8 domain
/// (`GAMEPLAY_MODEL_SCALE` is `100/256` so raw/256 is world). Do not apply
/// the menu `* 256 / 100` percent conversion here.
pub fn type0_slot_model_raw(records: &[[i16; 4]], slot: u16) -> Option<[i16; 3]> {
    let record = records.get(usize::from(slot) >> 1)?;
    if record[0] != 0 {
        return None;
    }
    let x = if slot & 1 == 0 {
        record[1]
    } else {
        record[1].wrapping_neg()
    };
    Some([x, record[2], record[3]])
}

fn q31_mul_i16(axis: i32, component: i16) -> i16 {
    ((i64::from(axis) * i64::from(component)) >> 31) as i16
}

/// Body-space 8.8 through Q31 lateral/up/forward axes, then origin.
pub fn rotate_q31_world_8_8(
    origin_raw: [i16; 3],
    axes_q31: [[i32; 3]; 3],
    local_8_8: [i16; 3],
) -> [i16; 3] {
    let [lateral, up, forward] = axes_q31;
    core::array::from_fn(|axis| {
        origin_raw[axis]
            .wrapping_add(q31_mul_i16(lateral[axis], local_8_8[0]))
            .wrapping_add(q31_mul_i16(up[axis], local_8_8[1]))
            .wrapping_add(q31_mul_i16(forward[axis], local_8_8[2]))
    })
}

/// `FUN_00413F70` stores a Q15 word in the high half (`word << 16`). Identity
/// heading 0 writes `0x7FFF0000`. `FUN_0041DF20` and D360 secondary then
/// narrow with `>> 0x13` to Q12, so this unit must be 4095 after that shift.
const UNIT_Q31_HIGH_WORD: i32 = 0x7FFF_0000;

fn identity_model_axes() -> SubHModelAxesRaw {
    SubHModelAxesRaw {
        axes: [
            [UNIT_Q31_HIGH_WORD, 0, 0],
            [0, UNIT_Q31_HIGH_WORD, 0],
            [0, 0, UNIT_Q31_HIGH_WORD],
        ],
    }
}

fn model_axes_from_q31(axes_q31: [[i32; 3]; 3]) -> SubHModelAxesRaw {
    // DF20 / D360 already do `>> 0x13` on the entity/model words at
    // `+0x0C/+0x18/+0x24`. Shifting Q31 here first collapses live 13F70
    // axes to 0, so the constraint search never moves and the knee basis
    // is a zero cross product (planted feet trail, legs stay straight).
    SubHModelAxesRaw { axes: axes_q31 }
}

/// Actor-relative display adapter for an unowned Free/intrinsic viewport.
///
/// Internal geometry retains signed wrapping 8.8 endpoint words. A signed-word
/// displacement keeps a limb near the selected actor image when that limb
/// crosses the stored-coordinate seam; sign-extending each endpoint as an
/// absolute float would place it a full world away.
///
/// Native `40D350` instead selects each endpoint around the inherited viewport.
/// `SubHPresentation` owns that policy separately when its native context exists.
pub fn endpoint_as_draw_world(
    origin_world: [f32; 3],
    origin_raw: [i16; 3],
    endpoint_raw: [i32; 3],
) -> [f32; 3] {
    core::array::from_fn(|axis| {
        let delta_raw = (endpoint_raw[axis] as i16).wrapping_sub(origin_raw[axis]);
        origin_world[axis] + f32::from(delta_raw) / 256.0
    })
}

/// Write `FUN_0041D360` cache bits and endpoints back into the live Sub-H
/// records.
///
/// `FUN_0041D2A0` constructs those records with flags 0. The writer
/// `FUN_0041D0A0` will not start a phase unless live bit `0x02` is set.
/// That bit is also `PRIMARY_CACHE_VALID`: retail D360 sets it on the same
/// dword, and bit `0x08` (`TERRAIN_OFFSET_VALID` / `WAIT_FOR_DEPENDENCIES`)
/// is the stride gate. Draw can keep a copy for presentation; the live
/// writer needs this commit or gait never leaves rest pose.
pub fn commit_sub_h_geometry(
    runtime: &mut SubHRuntimeState,
    descriptor: &SubHExternalFrameDescriptor,
    records: &[[i16; 4]],
    origin_raw: [i16; 3],
    body_axes_q31: Option<[[i32; 3]; 3]>,
    mut effective_height_raw: impl FnMut(i16, i16) -> i16,
) -> Option<()> {
    if descriptor.records.len() != runtime.records.len() {
        return None;
    }
    let model_axes = body_axes_q31.map_or_else(identity_model_axes, model_axes_from_q31);
    let enabled = runtime.is_enabled();
    for (record_index, authored) in descriptor.records.iter().enumerate() {
        let resolve_vertex = |role: SubHVertexRole| {
            let slot = match role {
                SubHVertexRole::AnchorA => authored.vertex_refs[0],
                SubHVertexRole::BendB => authored.vertex_refs[1],
                SubHVertexRole::EndpointC => authored.vertex_refs[2],
            };
            let local_8_8 = type0_slot_model_raw(records, slot)?;
            Some(match body_axes_q31 {
                Some(axes) => rotate_q31_world_8_8(origin_raw, axes, local_8_8),
                None => [
                    origin_raw[0].wrapping_add(local_8_8[0]),
                    origin_raw[1].wrapping_add(local_8_8[1]),
                    origin_raw[2].wrapping_add(local_8_8[2]),
                ],
            })
        };
        let request = SubHGeometryRequest {
            endpoint: SubHEndpoint::Primary,
            enabled,
            axis_mode_raw: authored.axis_mode_raw,
            model_axes_raw: model_axes,
        };
        let _ = resolve_geometry_endpoint(
            &mut runtime.records[record_index],
            request,
            resolve_vertex,
            &mut effective_height_raw,
        );
        let _ = resolve_geometry_endpoint(
            &mut runtime.records[record_index],
            SubHGeometryRequest {
                endpoint: SubHEndpoint::Secondary,
                ..request
            },
            resolve_vertex,
            &mut effective_height_raw,
        );
    }
    Some(())
}

/// Resolve `FUN_0041D360` selector world points for one Known Sub-H runtime.
///
/// Type-0 vertex slots are required. A missing or non-type-0 ref fail-closes
/// the whole actor to [`None`] so draw keeps raw type-14 operands instead of
/// collapsing legs to the origin. `origin_raw` is the toroidal 8.8 origin
/// used by geometry and terrain; `origin_world` is the matching draw-space
/// f32 origin (camera-relative unwrap). Presentation keeps a copy; live gait
/// uses [`commit_sub_h_geometry`].
pub fn resolve_selector_world_points(
    runtime: &SubHRuntimeState,
    descriptor: &SubHExternalFrameDescriptor,
    records: &[[i16; 4]],
    origin_raw: [i16; 3],
    origin_world: [f32; 3],
    body_axes_q31: Option<[[i32; 3]; 3]>,
    mut effective_height_raw: impl FnMut(i16, i16) -> i16,
) -> Option<[Option<[f32; 3]>; 16]> {
    if descriptor.records.len() != runtime.records().len() {
        return None;
    }
    let model_axes = body_axes_q31.map_or_else(identity_model_axes, model_axes_from_q31);
    let mut phase_records = runtime.records().to_vec();
    let mut points = [None; 16];
    for (record_index, authored) in descriptor.records.iter().enumerate() {
        let resolve_vertex = |role: SubHVertexRole| {
            let slot = match role {
                SubHVertexRole::AnchorA => authored.vertex_refs[0],
                SubHVertexRole::BendB => authored.vertex_refs[1],
                SubHVertexRole::EndpointC => authored.vertex_refs[2],
            };
            let local_8_8 = type0_slot_model_raw(records, slot)?;
            Some(match body_axes_q31 {
                Some(axes) => rotate_q31_world_8_8(origin_raw, axes, local_8_8),
                None => [
                    origin_raw[0].wrapping_add(local_8_8[0]),
                    origin_raw[1].wrapping_add(local_8_8[1]),
                    origin_raw[2].wrapping_add(local_8_8[2]),
                ],
            })
        };
        let request = SubHGeometryRequest {
            endpoint: SubHEndpoint::Primary,
            enabled: runtime.is_enabled(),
            axis_mode_raw: authored.axis_mode_raw,
            model_axes_raw: model_axes,
        };
        // One degenerate record must not Raw the whole actor. Live newants
        // then keep only the six type-13 ground ribbons while every type-14
        // leg drops to `[selector,0,0]`.
        let Ok(primary) = resolve_geometry_endpoint(
            &mut phase_records[record_index],
            request,
            resolve_vertex,
            &mut effective_height_raw,
        ) else {
            continue;
        };
        if record_index < 16 {
            points[record_index] = Some(endpoint_as_draw_world(origin_world, origin_raw, primary));
        }
        let Ok(secondary) = resolve_geometry_endpoint(
            &mut phase_records[record_index],
            SubHGeometryRequest {
                endpoint: SubHEndpoint::Secondary,
                ..request
            },
            resolve_vertex,
            &mut effective_height_raw,
        ) else {
            continue;
        };
        let secondary_selector = record_index + descriptor.records.len();
        if secondary_selector < 16 {
            points[secondary_selector] =
                Some(endpoint_as_draw_world(origin_world, origin_raw, secondary));
        }
    }
    Some(points)
}

/// Draw-owned `6ECF0 -> D350 -> A9F0 -> 1D360` context. The actor's root
/// model supplies the source slots and physical basis throughout its hierarchy.
/// Call only for selectors requested by the selected model commands after
/// authored face rejection; unsubmitted limbs must retain their live caches.
pub struct SubHPresentation<'a> {
    pub runtime: &'a mut SubHRuntimeState,
    pub descriptor: &'a SubHExternalFrameDescriptor,
    pub model_records: &'a [[i16; 4]],
    pub retail_tick: u32,
    pub origin_raw: [i16; 3],
    pub origin_world: [f32; 3],
    pub body_axes_q31: Option<[[i32; 3]; 3]>,
    /// Current node's source VIEW frame and inherited viewport. Free-camera
    /// and intrinsic contexts explicitly leave native admission unowned.
    pub native_context: Option<(
        crate::native_model_frame::NativeModelFrame,
        crate::native_model_frame::NativeWorldViewport,
    )>,
    pub fallback: Option<ActorExternalFrameFallback<'a>>,
    pub emitter: Option<ActorEmitterExternalFramePresentation<'a>>,
}

impl SubHPresentation<'_> {
    /// 40D350 chooses each callback WORD's image around the inherited viewport,
    /// independently of the actor centre. Keep the complete DWORD difference
    /// between those images: narrowing it again would join opposite sides of
    /// the half-world boundary. The selected actor draw origin transports the
    /// source signed-camera image into GL's unsigned X/Z display image.
    fn endpoint_as_presentation_world(&self, endpoint_raw: [i32; 3]) -> [f32; 3] {
        if let Some((_, viewport)) = self.native_context.as_ref() {
            let endpoint_image = viewport.actor_world_image(endpoint_raw.map(|value| value as i16));
            let actor_image = viewport.actor_world_image(self.origin_raw);
            return core::array::from_fn(|axis| {
                self.origin_world[axis]
                    + endpoint_image[axis].wrapping_sub(actor_image[axis]) as f32 / 256.0
            });
        }
        // Free/intrinsic presentation has no native viewport owner. Preserve
        // its existing local limb placement rather than inventing a camera.
        endpoint_as_draw_world(self.origin_world, self.origin_raw, endpoint_raw)
    }

    pub fn prepare_post_h_node(
        &mut self,
        model: &v2k_formats::models::ModelEntry,
        vars: &v2k_formats::models::AnimVars,
    ) {
        if let Some(emitter) = self.emitter.as_mut() {
            if let Some((frame, _)) = self.native_context.as_ref() {
                emitter.prepare_native_node(model, Some(frame));
            } else {
                emitter.prepare_node(model, vars);
            }
        }
    }

    /// A9F0 tries Sub-E before its terminal relation/self suffix. A provider
    /// boundary stops this selector; it never falls through to another owner.
    pub fn resolve_post_h_world(&mut self, parameters: [i16; 3]) -> Option<[f32; 3]> {
        self.resolve_post_h_point(parameters).map(|point| point.0)
    }

    pub(crate) fn resolve_post_h_point(
        &mut self,
        parameters: [i16; 3],
    ) -> Option<([f32; 3], [i32; 3])> {
        if let Some(emitter) = self.emitter.as_mut() {
            match emitter.resolve(self.descriptor.records.len() as u16, parameters) {
                Ok(Some(point)) => {
                    return Some((
                        self.endpoint_as_presentation_world(point.position_raw),
                        point.position_raw,
                    ))
                }
                Ok(None) => {}
                Err(_) => return None,
            }
        }
        let endpoint = self.fallback.as_mut()?.resolve_raw(parameters).ok()??;
        let raw = endpoint.position_raw.map(i32::from);
        Some((self.endpoint_as_presentation_world(raw), raw))
    }

    pub fn resolve_fallback_world(&mut self, parameters: [i16; 3]) -> Option<[f32; 3]> {
        let endpoint = self.fallback.as_mut()?.resolve_raw(parameters).ok()??;
        Some(self.endpoint_as_presentation_world(endpoint.position_raw.map(i32::from)))
    }

    pub fn resolve(
        &mut self,
        selectors: impl IntoIterator<Item = i32>,
        mut effective_height_raw: impl FnMut(i16, i16) -> i16,
    ) -> Option<[Option<[f32; 3]>; 16]> {
        if self.descriptor.records.len() != self.runtime.records.len() {
            return None;
        }
        let mut points = [None; 16];
        let model_axes = self
            .body_axes_q31
            .map_or_else(identity_model_axes, model_axes_from_q31);
        for selector in selectors {
            let Some(binding) = selector_binding(selector, self.runtime.records.len()) else {
                continue;
            };
            let Some(point) = points.get_mut(selector as usize) else {
                continue;
            };
            let authored = self.descriptor.records[binding.record_index];
            let resolve_vertex = |role| {
                let slot = authored.vertex_refs[match role {
                    SubHVertexRole::AnchorA => 0,
                    SubHVertexRole::BendB => 1,
                    SubHVertexRole::EndpointC => 2,
                }];
                if let Some((frame, viewport)) = self.native_context.as_ref() {
                    // 41E310 resolves the current VIEW slot first, then applies
                    // the inverse viewport. These separately shifted products
                    // cannot be collapsed into a physical-body rotation.
                    let view = frame.resolve_type0_slot_view_raw(self.model_records, slot)?;
                    return Some(viewport.view_point_to_world(view).map(|value| value as i16));
                }
                let local = type0_slot_model_raw(self.model_records, slot)?;
                Some(match self.body_axes_q31 {
                    Some(axes) => rotate_q31_world_8_8(self.origin_raw, axes, local),
                    None => {
                        core::array::from_fn(|axis| self.origin_raw[axis].wrapping_add(local[axis]))
                    }
                })
            };
            let request = SubHGeometryRequest {
                endpoint: binding.endpoint,
                enabled: self.runtime.is_enabled(),
                axis_mode_raw: authored.axis_mode_raw,
                model_axes_raw: model_axes,
            };
            if let Ok(endpoint) = resolve_geometry_endpoint(
                &mut self.runtime.records[binding.record_index],
                request,
                resolve_vertex,
                &mut effective_height_raw,
            ) {
                *point = Some(self.endpoint_as_presentation_world(endpoint));
            }
        }
        Some(points)
    }
}

/// Map one authored type-14 selector to the paired live Sub-H record endpoint.
///
/// `FUN_0041D360` treats selectors `0..count` as the primary point and
/// `count..2*count` as the secondary point of the same record. Negative and
/// out-of-range authored values would index invalid retail memory, so the port
/// fails closed.
pub fn selector_binding(selector: i32, record_count: usize) -> Option<SubHSelectorBinding> {
    if selector < 0 || record_count > MAX_SUB_H_RECORDS {
        return None;
    }
    let selector = usize::try_from(selector).ok()?;
    if selector < record_count {
        Some(SubHSelectorBinding {
            record_index: selector,
            endpoint: SubHEndpoint::Primary,
        })
    } else if selector < record_count.checked_mul(2)? {
        Some(SubHSelectorBinding {
            record_index: selector - record_count,
            endpoint: SubHEndpoint::Secondary,
        })
    } else {
        None
    }
}

/// Mutable fields of one retail 0x3C-byte external-frame record.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SubHPhaseRecord {
    pub flags_raw: u32,
    pub primary_raw: [i16; 3],
    pub secondary_raw: [i16; 3],
    pub terrain_offset_raw: [i16; 3],
    pub animated_target_raw: [i16; 3],
    pub phase_origin_raw: [i16; 3],
    pub phase_raw: u16,
    pub chord_length_raw: i16,
    pub two_edge_length_raw: i16,
    pub excess_length_raw: i16,
    pub excess_length_squared_raw: i32,
    pub ab_length_squared_raw: i32,
    pub bc_length_squared_raw: i32,
    pub two_edge_length_squared_raw: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubHVertexRole {
    EndpointC,
    AnchorA,
    BendB,
}

/// Three model/entity basis vectors as stored at `+0x0C`, `+0x18`, and `+0x24`.
///
/// `FUN_00413F70` writes Q31 (`word << 16`). `FUN_0041DF20` and D360
/// secondary narrow each component with `>> 0x13` to Q12. Keep the stored
/// words here; do not pre-shift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubHModelAxesRaw {
    pub axes: [[i32; 3]; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubHGeometryRequest {
    pub endpoint: SubHEndpoint,
    pub enabled: bool,
    pub axis_mode_raw: u16,
    pub model_axes_raw: SubHModelAxesRaw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubHGeometryError {
    MissingVertex(SubHVertexRole),
    InvalidAxisMode(u16),
    DegeneratePrimaryChord,
    ArithmeticTrap,
}

impl fmt::Display for SubHGeometryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::MissingVertex(role) => write!(formatter, "missing Sub-H {role:?} vertex"),
            Self::InvalidAxisMode(mode) => write!(formatter, "invalid Sub-H axis mode {mode}"),
            Self::DegeneratePrimaryChord => {
                write!(
                    formatter,
                    "Sub-H secondary endpoint has a zero primary chord"
                )
            }
            Self::ArithmeticTrap => write!(formatter, "Sub-H retail division would trap"),
        }
    }
}

impl std::error::Error for SubHGeometryError {}

fn squared_distance_raw(a: [i16; 3], b: [i16; 3]) -> i32 {
    a.into_iter()
        .zip(b)
        .map(|(lhs, rhs)| i32::from(lhs.wrapping_sub(rhs)))
        .fold(0_i32, |sum, delta| {
            sum.wrapping_add(delta.wrapping_mul(delta))
        })
}

fn normalized_q12(delta: [i16; 3], length: i16) -> [i16; 3] {
    if length == 0 {
        return [0x1000, 0, 0];
    }
    delta.map(|component| ((i32::from(component) << 12) / i32::from(length)) as i16)
}

fn scale_q12(vector: [i16; 3], scale: i32) -> [i16; 3] {
    vector.map(|component| (scale.wrapping_mul(i32::from(component)) >> 12) as i16)
}

fn cross_q12(lhs: [i16; 3], rhs: [i16; 3]) -> [i16; 3] {
    [
        (i32::from(lhs[1])
            .wrapping_mul(i32::from(rhs[2]))
            .wrapping_sub(i32::from(lhs[2]).wrapping_mul(i32::from(rhs[1])))
            >> 12) as i16,
        (i32::from(lhs[2])
            .wrapping_mul(i32::from(rhs[0]))
            .wrapping_sub(i32::from(lhs[0]).wrapping_mul(i32::from(rhs[2])))
            >> 12) as i16,
        (i32::from(lhs[0])
            .wrapping_mul(i32::from(rhs[1]))
            .wrapping_sub(i32::from(lhs[1]).wrapping_mul(i32::from(rhs[0])))
            >> 12) as i16,
    ]
}

fn signed_midpoint_raw(low: i16, high: i16) -> i16 {
    // 41DF20 narrows the endpoint difference to a signed word before / 2.
    // Preserve truncation toward zero, then the final wrapped short sum.
    // A full-width difference bisects the long arc at the signed 8.8 seam.
    low.wrapping_add(high.wrapping_sub(low) / 2)
}

/// Exact four-probe midpoint search from retail `FUN_0041DF20`.
///
/// `effective_height_raw` represents either bare bilinear terrain or the
/// caller-selected max of terrain and its external/procedural surface.
pub fn constraint_midpoint_raw(
    center_raw: [i16; 3],
    basis_raw: [i32; 3],
    excess_length_raw: i16,
    mut effective_height_raw: impl FnMut(i16, i16) -> i16,
) -> [i16; 3] {
    let diameter = i32::from(excess_length_raw).wrapping_mul(2);
    let basis_q12 = basis_raw.map(|component| (component >> 19) as i16);
    let delta =
        basis_q12.map(|component| (diameter.wrapping_mul(i32::from(component)) >> 12) as i16);
    let mut low = [
        center_raw[0].wrapping_sub(delta[0]),
        center_raw[1].wrapping_sub(delta[1]),
        center_raw[2].wrapping_sub(delta[2]),
    ];
    let mut high = [
        center_raw[0].wrapping_add(delta[0]),
        center_raw[1].wrapping_add(delta[1]),
        center_raw[2].wrapping_add(delta[2]),
    ];
    let mut midpoint = [
        signed_midpoint_raw(low[0], high[0]),
        signed_midpoint_raw(low[1], high[1]),
        signed_midpoint_raw(low[2], high[2]),
    ];

    for _ in 0..4 {
        let height = effective_height_raw(midpoint[0], midpoint[2]);
        if midpoint[1] < height {
            low = midpoint;
        } else {
            high = midpoint;
        }
        midpoint = [
            signed_midpoint_raw(low[0], high[0]),
            signed_midpoint_raw(low[1], high[1]),
            signed_midpoint_raw(low[2], high[2]),
        ];
    }
    midpoint
}

/// Retail's nonstandard vertical arch term for an animated Sub-H primary.
pub fn phase_arch_raw(chord_length_raw: i16, phase_raw: u16) -> i16 {
    let chord = i32::from(chord_length_raw);
    let amplitude = chord.wrapping_add((chord >> 31) & 3) >> 2;
    let repeated_sine = retail_sine_q15(u32::from(phase_raw)).wrapping_mul(0x1_0001);
    ((i64::from(amplitude) * i64::from(repeated_sine)) >> 31) as i16
}

fn interpolate_phase_raw(origin: i16, target: i16, phase_raw: u16) -> i16 {
    let delta = i32::from(target.wrapping_sub(origin));
    let step = delta.wrapping_mul(i32::from(phase_raw)) >> 15;
    origin.wrapping_add(step as i16)
}

fn resolve_required_vertex(
    role: SubHVertexRole,
    resolver: &mut impl FnMut(SubHVertexRole) -> Option<[i16; 3]>,
) -> Result<[i16; 3], SubHGeometryError> {
    resolver(role).ok_or(SubHGeometryError::MissingVertex(role))
}

/// Resolve one paired endpoint through the exact detached `FUN_0041D360`
/// fixed-point geometry transaction.
///
/// Vertex callbacks are requested in retail order: endpoint C, anchor A, then
/// bend B, and only when the current cache-bit topology requires each value.
/// Mutations commit atomically after all fallible work succeeds. This differs
/// only from retail's fatal divide fault: a degenerate authored secondary
/// fails closed instead of crashing the process.
pub fn resolve_geometry_endpoint(
    record: &mut SubHPhaseRecord,
    request: SubHGeometryRequest,
    mut resolve_vertex: impl FnMut(SubHVertexRole) -> Option<[i16; 3]>,
    mut effective_height_raw: impl FnMut(i16, i16) -> i16,
) -> Result<[i32; 3], SubHGeometryError> {
    let original_flags = record.flags_raw;
    let endpoint_c = if original_flags & 3 != 3 {
        Some(resolve_required_vertex(
            SubHVertexRole::EndpointC,
            &mut resolve_vertex,
        )?)
    } else {
        None
    };
    let anchor_a = if original_flags & 7 != 7 {
        Some(resolve_required_vertex(
            SubHVertexRole::AnchorA,
            &mut resolve_vertex,
        )?)
    } else {
        None
    };
    let bend_b = if original_flags & LENGTH_CACHE_VALID == 0 {
        Some(resolve_required_vertex(
            SubHVertexRole::BendB,
            &mut resolve_vertex,
        )?)
    } else {
        None
    };

    let mut next = *record;
    let mut flags = original_flags;

    if flags & LENGTH_CACHE_VALID == 0 {
        let a = anchor_a.expect("cold length cache always resolves anchor A");
        let b = bend_b.expect("cold length cache always resolves bend B");
        let c = endpoint_c.expect("cold length cache always resolves endpoint C");
        next.ab_length_squared_raw = squared_distance_raw(a, b);
        next.bc_length_squared_raw = squared_distance_raw(b, c);
        next.chord_length_raw = retail_integer_sqrt(squared_distance_raw(a, c)) as i16;
        let ab_length = retail_integer_sqrt(next.ab_length_squared_raw) as i16;
        let bc_length = retail_integer_sqrt(next.bc_length_squared_raw) as i16;
        next.two_edge_length_raw = ab_length.wrapping_add(bc_length);
        next.excess_length_raw = next.two_edge_length_raw.wrapping_sub(next.chord_length_raw);
        next.two_edge_length_squared_raw =
            i32::from(next.two_edge_length_raw).wrapping_mul(i32::from(next.two_edge_length_raw));
        next.excess_length_squared_raw =
            i32::from(next.excess_length_raw).wrapping_mul(i32::from(next.excess_length_raw));
        flags |= LENGTH_CACHE_VALID;
    }

    if flags & PRIMARY_CACHE_VALID == 0 {
        let c = endpoint_c.expect("uncached primary always resolves endpoint C");
        if !request.enabled {
            next.primary_raw = c;
        } else if flags & PREVIOUSLY_ENABLED == 0 {
            flags |= PREVIOUSLY_ENABLED;
            next.primary_raw = c;
        } else if next.phase_raw == 0 {
            let a = anchor_a.expect("settled primary always resolves anchor A");
            next.primary_raw[1] = effective_height_raw(next.primary_raw[0], next.primary_raw[2]);

            if flags & TERRAIN_OFFSET_VALID == 0 {
                let constrained = constraint_midpoint_raw(
                    c,
                    request.model_axes_raw.axes[1],
                    next.excess_length_raw,
                    &mut effective_height_raw,
                );
                let constraint_distance = squared_distance_raw(next.primary_raw, constrained);
                if constraint_distance > next.excess_length_squared_raw {
                    flags |= TERRAIN_OFFSET_VALID;
                    let direction = normalized_q12(
                        [
                            constrained[0].wrapping_sub(next.primary_raw[0]),
                            constrained[1].wrapping_sub(next.primary_raw[1]),
                            constrained[2].wrapping_sub(next.primary_raw[2]),
                        ],
                        retail_integer_sqrt(constraint_distance) as i16,
                    );
                    next.terrain_offset_raw =
                        scale_q12(direction, i32::from(next.excess_length_raw) / 2);
                }
            }

            let distance_from_a = squared_distance_raw(next.primary_raw, a);
            if distance_from_a > next.two_edge_length_squared_raw {
                let direction = normalized_q12(
                    [
                        next.primary_raw[0].wrapping_sub(a[0]),
                        next.primary_raw[1].wrapping_sub(a[1]),
                        next.primary_raw[2].wrapping_sub(a[2]),
                    ],
                    retail_integer_sqrt(distance_from_a) as i16,
                );
                let offset = scale_q12(direction, i32::from(next.two_edge_length_raw));
                next.primary_raw = [
                    a[0].wrapping_add(offset[0]),
                    a[1].wrapping_add(offset[1]),
                    a[2].wrapping_add(offset[2]),
                ];
            }
        } else {
            let target = [
                c[0].wrapping_add(next.terrain_offset_raw[0]),
                c[1].wrapping_add(next.terrain_offset_raw[1]),
                c[2].wrapping_add(next.terrain_offset_raw[2]),
            ];
            next.animated_target_raw = constraint_midpoint_raw(
                target,
                request.model_axes_raw.axes[1],
                next.excess_length_raw,
                &mut effective_height_raw,
            );
            next.primary_raw[0] = interpolate_phase_raw(
                next.phase_origin_raw[0],
                next.animated_target_raw[0],
                next.phase_raw,
            );
            next.primary_raw[1] = interpolate_phase_raw(
                next.phase_origin_raw[1],
                next.animated_target_raw[1],
                next.phase_raw,
            )
            .wrapping_add(phase_arch_raw(next.chord_length_raw, next.phase_raw));
            next.primary_raw[2] = interpolate_phase_raw(
                next.phase_origin_raw[2],
                next.animated_target_raw[2],
                next.phase_raw,
            );
        }
        flags |= PRIMARY_CACHE_VALID;
    }

    if request.endpoint == SubHEndpoint::Secondary && flags & SECONDARY_CACHE_VALID == 0 {
        let a = anchor_a.expect("uncached secondary always resolves anchor A");
        let delta = [
            next.primary_raw[0].wrapping_sub(a[0]),
            next.primary_raw[1].wrapping_sub(a[1]),
            next.primary_raw[2].wrapping_sub(a[2]),
        ];
        let primary_length = retail_integer_sqrt(squared_distance_raw(next.primary_raw, a)) as i16;
        if primary_length == 0 {
            return Err(SubHGeometryError::DegeneratePrimaryChord);
        }
        let normal = normalized_q12(delta, primary_length);

        let mode = usize::from(request.axis_mode_raw);
        if mode > 5 {
            return Err(SubHGeometryError::InvalidAxisMode(request.axis_mode_raw));
        }
        let basis_index = [1, 2, 0][mode % 3];
        let basis =
            request.model_axes_raw.axes[basis_index].map(|component| (component >> 19) as i16);
        let tangent = if mode < 3 {
            cross_q12(normal, basis)
        } else {
            cross_q12(basis, normal)
        };
        let radial = cross_q12(tangent, normal);

        let squared_difference = (next.bc_length_squared_raw as u32)
            .wrapping_sub(next.ab_length_squared_raw as u32)
            as i32;
        let denominator = i32::from(primary_length).wrapping_mul(2);
        let quotient = squared_difference
            .checked_div(denominator)
            .ok_or(SubHGeometryError::ArithmeticTrap)?;
        let axial = i32::from(primary_length) / 2 - quotient;
        let radius_squared = next
            .ab_length_squared_raw
            .wrapping_sub(axial.wrapping_mul(axial));
        let radius = if radius_squared < 0 {
            0
        } else {
            retail_integer_sqrt(radius_squared) as i32
        };
        let axial_offset = scale_q12(normal, axial);
        let radial_offset = scale_q12(radial, radius);
        next.secondary_raw = [
            a[0].wrapping_add(axial_offset[0])
                .wrapping_add(radial_offset[0]),
            a[1].wrapping_add(axial_offset[1])
                .wrapping_add(radial_offset[1]),
            a[2].wrapping_add(axial_offset[2])
                .wrapping_add(radial_offset[2]),
        ];
        flags |= SECONDARY_CACHE_VALID;
    }

    next.flags_raw = flags;
    let endpoint = match request.endpoint {
        SubHEndpoint::Primary => next.primary_raw,
        SubHEndpoint::Secondary => next.secondary_raw,
    };
    *record = next;
    Ok(endpoint.map(i32::from))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubHCompletionCue {
    pub record_index: usize,
    pub sound_id: u16,
}

/// Apply the pitch expression used by `FUN_0041D120` after its one
/// `Random_Next` call for a nonzero Sub-H completion sound.
pub fn completion_sound_pitch_16_16(random_value: u32) -> u32 {
    0xe000 + ((random_value & 0xffff) >> 2)
}

/// Low 32 bits of retail's signed `(elapsed_us * phase_rate) >> 31`.
pub fn phase_delta_raw(elapsed_us: i32, phase_rate_raw: i32) -> u32 {
    ((i64::from(elapsed_us) * i64::from(phase_rate_raw)) >> 31) as u32
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubHUpdateError {
    TooManyRecords {
        count: usize,
    },
    RecordCountMismatch {
        descriptor: usize,
        runtime: usize,
    },
    CursorOutOfRange {
        cursor: usize,
        count: usize,
    },
    DependencyOutOfRange {
        record_index: usize,
        dependency: usize,
        count: usize,
    },
}

impl fmt::Display for SubHUpdateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::TooManyRecords { count } => {
                write!(
                    formatter,
                    "Sub-H record count {count} exceeds retail limit 16"
                )
            }
            Self::RecordCountMismatch {
                descriptor,
                runtime,
            } => write!(
                formatter,
                "Sub-H descriptor/runtime count mismatch ({descriptor} != {runtime})"
            ),
            Self::CursorOutOfRange { cursor, count } => write!(
                formatter,
                "Sub-H rotating cursor {cursor} is outside {count} records"
            ),
            Self::DependencyOutOfRange {
                record_index,
                dependency,
                count,
            } => write!(
                formatter,
                "Sub-H record {record_index} dependency {dependency} is outside {count} records"
            ),
        }
    }
}

impl std::error::Error for SubHUpdateError {}

/// Retail Sub-H runtime state detached from its still-unowned live adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubHRuntimeState {
    cursor: usize,
    enabled: bool,
    surface_policy: SubHSurfacePolicy,
    records: Vec<SubHPhaseRecord>,
}

impl SubHRuntimeState {
    pub fn new(record_count: usize) -> Result<Self, SubHUpdateError> {
        if record_count > MAX_SUB_H_RECORDS {
            return Err(SubHUpdateError::TooManyRecords {
                count: record_count,
            });
        }
        Ok(Self {
            cursor: 0,
            enabled: true,
            surface_policy: SubHSurfacePolicy::Terrain,
            records: vec![SubHPhaseRecord::default(); record_count],
        })
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn surface_policy(&self) -> SubHSurfacePolicy {
        self.surface_policy
    }

    /// `FUN_0041E4A0` writes only outer `+0x0C`; it does not restart phases,
    /// enable animation, or invalidate already resolved endpoint caches.
    pub fn set_surface_policy(&mut self, policy: SubHSurfacePolicy) {
        self.surface_policy = policy;
    }

    pub fn records(&self) -> &[SubHPhaseRecord] {
        &self.records
    }

    pub fn records_mut(&mut self) -> &mut [SubHPhaseRecord] {
        &mut self.records
    }

    /// Read the endpoint returned at the end of `FUN_0041D360`.
    pub fn endpoint_raw(&self, selector: i32) -> Option<[i16; 3]> {
        let binding = selector_binding(selector, self.records.len())?;
        let record = self.records.get(binding.record_index)?;
        Some(match binding.endpoint {
            SubHEndpoint::Primary => record.primary_raw,
            SubHEndpoint::Secondary => record.secondary_raw,
        })
    }

    /// Execute one complete `FUN_0041D0A0` writer call.
    ///
    /// Completion cues are returned in retail update order. A live caller must
    /// consume one RNG value for each cue and feed it through
    /// [`completion_sound_pitch_16_16`].
    pub fn update(
        &mut self,
        descriptor: &SubHExternalFrameDescriptor,
        elapsed_us: i32,
    ) -> Result<Vec<SubHCompletionCue>, SubHUpdateError> {
        self.validate_descriptor(descriptor)?;

        if !self.enabled {
            for record in &mut self.records {
                record.flags_raw &= !TRANSIENT_STATE_MASK;
            }
            return Ok(Vec::new());
        }

        let count = self.records.len();
        if count == 0 {
            self.cursor = 0;
            return Ok(Vec::new());
        }

        let mut cues = Vec::new();
        for offset in 0..count {
            let record_index = (self.cursor + offset) % count;
            let authored = descriptor.records[record_index];
            let delta = phase_delta_raw(elapsed_us, authored.phase_rate_raw);
            self.update_record(
                record_index,
                authored,
                delta,
                descriptor.completion_sound_id,
                &mut cues,
            );
        }
        self.cursor = (self.cursor + 1) % count;
        Ok(cues)
    }

    fn validate_descriptor(
        &self,
        descriptor: &SubHExternalFrameDescriptor,
    ) -> Result<(), SubHUpdateError> {
        let count = descriptor.records.len();
        if count > MAX_SUB_H_RECORDS {
            return Err(SubHUpdateError::TooManyRecords { count });
        }
        if count != self.records.len() {
            return Err(SubHUpdateError::RecordCountMismatch {
                descriptor: count,
                runtime: self.records.len(),
            });
        }
        if count != 0 && self.cursor >= count {
            return Err(SubHUpdateError::CursorOutOfRange {
                cursor: self.cursor,
                count,
            });
        }
        for (record_index, record) in descriptor.records.iter().enumerate() {
            for &dependency in &record.dependencies {
                let dependency = usize::from(dependency);
                if dependency >= count {
                    return Err(SubHUpdateError::DependencyOutOfRange {
                        record_index,
                        dependency,
                        count,
                    });
                }
            }
        }
        Ok(())
    }

    fn update_record(
        &mut self,
        record_index: usize,
        authored: SubHExternalFrameRecord,
        delta: u32,
        completion_sound_id: Option<u16>,
        cues: &mut Vec<SubHCompletionCue>,
    ) {
        let phase = self.records[record_index].phase_raw;
        if phase != 0 {
            let sum = delta.wrapping_add(u32::from(phase));
            if sum < 0x8000 {
                let record = &mut self.records[record_index];
                record.phase_raw = sum as u16;
                record.flags_raw &= 0xffff_fff9;
                return;
            }

            if let Some(sound_id) = completion_sound_id {
                cues.push(SubHCompletionCue {
                    record_index,
                    sound_id,
                });
            }
            let record = &mut self.records[record_index];
            record.phase_raw = 0;
            if record.flags_raw & COPY_TARGET_ON_COMPLETION != 0 {
                record.primary_raw = record.animated_target_raw;
                record.flags_raw &= 0xffff_fff9;
            } else {
                record.flags_raw &= 0xffff_ffe1;
            }
            return;
        }

        let flags = self.records[record_index].flags_raw;
        if flags & COPY_TARGET_ON_COMPLETION == 0 {
            self.records[record_index].flags_raw &= 0xffff_ffe7;
            return;
        }

        if flags & WAIT_FOR_DEPENDENCIES != 0 {
            let dependencies_finished = authored
                .dependencies
                .iter()
                .all(|&dependency| self.records[usize::from(dependency)].phase_raw == 0);
            if dependencies_finished {
                let record = &mut self.records[record_index];
                record.phase_origin_raw = record.primary_raw;
                record.phase_raw = if delta > 0x7fff { 0x7fff } else { delta as u16 };
                record.flags_raw &= 0xffff_fff1;
                return;
            }
        }

        self.records[record_index].flags_raw &= 0xffff_fff9;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authored_record(rate: i32, dependencies: [u8; 4]) -> SubHExternalFrameRecord {
        SubHExternalFrameRecord {
            resolver_flags_raw: 0,
            phase_rate_raw: rate,
            vertex_refs: [0; 3],
            axis_mode_raw: 0,
            dependencies,
        }
    }

    fn descriptor(
        sound_id: Option<u16>,
        records: Vec<SubHExternalFrameRecord>,
    ) -> SubHExternalFrameDescriptor {
        SubHExternalFrameDescriptor {
            completion_sound_id: sound_id,
            records,
        }
    }

    fn geometry_request(
        endpoint: SubHEndpoint,
        enabled: bool,
        axis_mode_raw: u16,
    ) -> SubHGeometryRequest {
        SubHGeometryRequest {
            endpoint,
            enabled,
            axis_mode_raw,
            model_axes_raw: SubHModelAxesRaw { axes: [[0; 3]; 3] },
        }
    }

    fn surface_terrain(height: i8, sea_raw: i16) -> TerrainGrid {
        TerrainGrid {
            header: [i32::from(sea_raw) << 8, 0, 0, 0, 0],
            cells: vec![
                v2k_formats::terrain::TerrainCell {
                    height: height as u8,
                    attribute: 0,
                    terrain_type: 0,
                };
                256 * 256
            ],
        }
    }

    #[test]
    fn surface_policy_retains_static_sea_when_waves_are_disabled() {
        let terrain = surface_terrain(-128, 4096);
        let sampler = SubHSurfaceSampler {
            terrain: &terrain,
            retail_tick: 1,
            waves_enabled: false,
            policy: SubHSurfacePolicy::TerrainAndWater,
        };
        assert_eq!(sampler.sample_raw(0, 0), 4096);
        assert_eq!(
            SubHSurfaceSampler {
                waves_enabled: true,
                ..sampler
            }
            .sample_raw(0, 0),
            4114,
            "445920's exact three-wave displacement at tick1"
        );
        assert_eq!(
            SubHSurfaceSampler {
                policy: SubHSurfacePolicy::Terrain,
                ..sampler
            }
            .sample_raw(0, 0),
            -4096
        );
        for (floor, sea, expected) in [(-2, -32, -32), (-1, -64, -32), (2, 32, 64)] {
            let terrain = surface_terrain(floor, sea);
            assert_eq!(
                SubHSurfaceSampler {
                    terrain: &terrain,
                    ..sampler
                }
                .sample_raw(0, 0),
                expected,
                "the surface maximum compares signed words"
            );
        }
    }

    #[test]
    fn surface_sampler_wraps_both_bilinear_axes_before_applying_water_policy() {
        let mut terrain = surface_terrain(0, 16);
        for (x, z, height) in [(255, 255, -4_i8), (0, 255, -2), (255, 0, 6), (0, 0, 10)] {
            terrain.cells[x * 256 + z].height = height as u8;
        }
        let sampler = SubHSurfaceSampler {
            terrain: &terrain,
            retail_tick: u32::MAX,
            waves_enabled: false,
            policy: SubHSurfacePolicy::Terrain,
        };
        // X interpolation gives -96 and 256, then Z's quarter gives -8.
        assert_eq!(sampler.sample_raw(-128, -192), -8);
        assert_eq!(
            SubHSurfaceSampler {
                policy: SubHSurfacePolicy::TerrainAndWater,
                ..sampler
            }
            .sample_raw(-128, -192),
            16
        );
        let deep = surface_terrain(-32, 4096);
        let waves = SubHSurfaceSampler {
            terrain: &deep,
            retail_tick: 4793,
            waves_enabled: true,
            policy: SubHSurfacePolicy::TerrainAndWater,
        };
        assert_eq!(waves.sample_raw(-14_189, 19_589), 4371);
    }

    #[test]
    fn surface_policy_does_not_change_first_foot_or_cache_and_survives_phase_updates() {
        let terrain = surface_terrain(0, 150);
        let authored = descriptor(None, vec![authored_record(1, [0; 4])]);
        for (policy, expected_y) in [
            (SubHSurfacePolicy::Terrain, 0),
            (SubHSurfacePolicy::TerrainAndWater, 150),
        ] {
            let mut runtime = SubHRuntimeState::new(1).unwrap();
            assert_eq!(runtime.surface_policy(), SubHSurfacePolicy::Terrain);
            runtime.set_surface_policy(policy);
            let resolve_vertex = |role| {
                Some(match role {
                    SubHVertexRole::AnchorA => [0, 0, 0],
                    SubHVertexRole::BendB => [0, 100, 0],
                    SubHVertexRole::EndpointC => [100, 100, 0],
                })
            };
            let request = geometry_request(SubHEndpoint::Primary, true, 0);
            assert_eq!(
                resolve_geometry_endpoint(
                    &mut runtime.records[0],
                    request,
                    resolve_vertex,
                    |_, _| panic!("first enabled foot copies C without sampling a surface"),
                ),
                Ok([100, 100, 0])
            );
            let first = runtime.records[0];
            runtime.set_surface_policy(SubHSurfacePolicy::TerrainAndWater);
            assert_eq!(
                runtime.records[0], first,
                "1E4A0 does not invalidate caches"
            );
            runtime.set_surface_policy(policy);
            runtime.update(&authored, 1000).unwrap();
            assert_eq!(runtime.surface_policy(), policy);
            let sampler = SubHSurfaceSampler {
                terrain: &terrain,
                retail_tick: 1,
                waves_enabled: false,
                policy: runtime.surface_policy(),
            };
            let mut samples = 0;
            let endpoint = resolve_geometry_endpoint(
                &mut runtime.records[0],
                request,
                resolve_vertex,
                |x, z| {
                    samples += 1;
                    sampler.sample_raw(x, z)
                },
            )
            .unwrap();
            assert_eq!(endpoint, [100, expected_y, 0]);
            assert_eq!(samples, 5, "settled foot plus four DF20 probes");
            assert_eq!(
                resolve_geometry_endpoint(
                    &mut runtime.records[0],
                    request,
                    resolve_vertex,
                    |_, _| panic!("warm primary cache suppresses surface lookup"),
                ),
                Ok(endpoint)
            );
            runtime.set_enabled(false);
            runtime.update(&authored, 1000).unwrap();
            assert_eq!(runtime.surface_policy(), policy);
        }
    }

    #[test]
    fn cold_geometry_resolves_c_a_b_and_builds_the_exact_length_cache() {
        let mut record = SubHPhaseRecord::default();
        let mut order = Vec::new();
        let endpoint = resolve_geometry_endpoint(
            &mut record,
            geometry_request(SubHEndpoint::Primary, false, 0),
            |role| {
                order.push(role);
                Some(match role {
                    SubHVertexRole::EndpointC => [100, 100, 0],
                    SubHVertexRole::AnchorA => [0, 0, 0],
                    SubHVertexRole::BendB => [0, 100, 0],
                })
            },
            |_, _| panic!("disabled initialization does not sample a surface"),
        )
        .unwrap();

        assert_eq!(
            order,
            [
                SubHVertexRole::EndpointC,
                SubHVertexRole::AnchorA,
                SubHVertexRole::BendB,
            ]
        );
        assert_eq!(endpoint, [100, 100, 0]);
        assert_eq!(record.primary_raw, [100, 100, 0]);
        assert_eq!(record.chord_length_raw, 141);
        assert_eq!(record.ab_length_squared_raw, 10_000);
        assert_eq!(record.bc_length_squared_raw, 10_000);
        assert_eq!(record.two_edge_length_raw, 200);
        assert_eq!(record.excess_length_raw, 59);
        assert_eq!(record.excess_length_squared_raw, 3_481);
        assert_eq!(record.two_edge_length_squared_raw, 40_000);
        assert_eq!(
            record.flags_raw & 7,
            LENGTH_CACHE_VALID | PRIMARY_CACHE_VALID
        );
    }

    #[test]
    fn warm_secondary_cache_suppresses_all_external_work() {
        let mut record = SubHPhaseRecord {
            flags_raw: 7,
            primary_raw: [1, 2, 3],
            secondary_raw: [4, 5, 6],
            ..SubHPhaseRecord::default()
        };
        assert_eq!(
            resolve_geometry_endpoint(
                &mut record,
                geometry_request(SubHEndpoint::Secondary, true, u16::MAX),
                |_| panic!("warm cache must not resolve vertices"),
                |_, _| panic!("warm cache must not sample a surface"),
            ),
            Ok([4, 5, 6])
        );
    }

    #[test]
    fn q31_identity_survives_retail_df20_shift_to_q12_unit() {
        let q31 = model_axes_from_q31([[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]]);
        let live = model_axes_from_q31([
            [UNIT_Q31_HIGH_WORD, 0, 0],
            [0, UNIT_Q31_HIGH_WORD, 0],
            [0, 0, UNIT_Q31_HIGH_WORD],
        ]);
        assert_eq!(q31.axes[1][1] >> 19, 4095);
        assert_eq!(live.axes[1][1] >> 19, 4095);
        assert_eq!(identity_model_axes().axes[1][1] >> 19, 4095);
        let collapsed = UNIT_Q31_HIGH_WORD >> 12;
        assert_eq!(
            collapsed >> 19,
            0,
            "an extra >> 12 before DF20's >> 0x13 must not be reintroduced"
        );
    }

    #[test]
    fn constraint_search_uses_four_probes_and_returns_the_fifth_midpoint() {
        let basis_x = [4095_i32 << 19, 0, 0];
        let mut probes = Vec::new();
        let below = constraint_midpoint_raw([0, 0, 0], basis_x, 256, |x, z| {
            probes.push((x, z));
            1
        });
        assert_eq!(probes, [(0, 0), (255, 0), (383, 0), (447, 0)]);
        assert_eq!(below, [479, 0, 0]);

        probes.clear();
        let equal = constraint_midpoint_raw([0, 0, 0], basis_x, 256, |x, z| {
            probes.push((x, z));
            0
        });
        assert_eq!(probes, [(0, 0), (-256, 0), (-384, 0), (-448, 0)]);
        assert_eq!(equal, [-480, 0, 0]);
    }

    #[test]
    fn signed_midpoint_narrows_delta_before_halving_and_truncates_toward_zero() {
        for (low, high, expected) in [
            (32760, -32760, -32768),
            (-32760, 32760, -32768),
            (32760, -32759, -32768),
            (-32760, 32759, -32768),
            (5, -2, 2),
            (-5, 2, -2),
            (0, -1, 0),
            (-1, 0, -1),
        ] {
            assert_eq!(signed_midpoint_raw(low, high), expected, "{low} -> {high}");
        }
    }

    #[test]
    fn constraint_search_stays_on_the_short_interval_across_xyz_signed_seams() {
        for axis in 0..3 {
            for coordinate in [32765, -32765] {
                for direction in [-1, 1] {
                    let mut center = [0_i16; 3];
                    center[axis] = coordinate;
                    let mut basis = [0_i32; 3];
                    basis[axis] = direction * (4095_i32 << 19);
                    let mut probes = Vec::new();
                    let result = constraint_midpoint_raw(center, basis, 16, |x, z| {
                        probes.push((x, z));
                        0
                    });
                    assert_eq!(probes.len(), 4);
                    assert_eq!(probes[0], (center[0], center[2]));
                    for (x, z) in probes {
                        assert!(i32::from(x.wrapping_sub(center[0])).abs() <= 32);
                        assert!(i32::from(z.wrapping_sub(center[2])).abs() <= 32);
                    }
                    for coordinate in 0..3 {
                        assert!(
                            i32::from(result[coordinate].wrapping_sub(center[coordinate])).abs()
                                <= 32,
                            "axis{axis}, direction{direction}, center{center:?}, result{result:?}"
                        );
                    }
                }
            }
        }
    }

    #[v2k_test_support::retail_test]
    fn real_world13_moving_newant_constraint_uses_four_nearby_terrain_probes() {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").is_file(),
            "real moving-newant assets are required"
        );
        let mut session = crate::session::GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(13, 1).unwrap();
        let descriptor = session
            .cache
            .global_entity_type(47)
            .unwrap()
            .sub_h_external_frame_descriptor()
            .unwrap();
        let model = session.cache.global_model(302).unwrap();
        let terrain = session.cache.terrain().unwrap();
        assert_eq!(descriptor.records.len(), 6);
        // Captured native ordinary-task controls, one world draw per 20ms:
        // path0/rng-prefix137 tick29 leg0, path2/prefix37 tick185 leg1,
        // path2/prefix137 tick183 leg2. The center includes the retained
        // terrain offset; these are controlled DF20 inputs, not a claim of
        // an identical full retail actor frame.
        for (record, local, center, up, excess, expected, expected_probes) in [
            (
                0,
                [133, -35, 98],
                [-18880, -471, 32765],
                [-226033664, 2134769664, -48168960],
                65,
                [-18868, -576, -32768],
                [
                    ((-18880, 32765), -584),
                    ((-18873, 32767), -582),
                    ((-18869, -32768), -580),
                    ((-18867, -32768), -580),
                ],
            ),
            (
                1,
                [-133, -35, 98],
                [-18368, -359, 32767],
                [-308019200, 2115108864, -205127680],
                68,
                [-18366, -368, -32768],
                [
                    ((-18368, 32767), -360),
                    ((-18358, -32762), -356),
                    ((-18363, -32765), -359),
                    ((-18365, -32767), -359),
                ],
            ),
            (
                2,
                [182, -35, -14],
                [-18689, -417, -32760],
                [-226033664, 2112815104, -309198848],
                55,
                [-18679, -505, -32747],
                [
                    ((-18689, -32760), -510),
                    ((-18683, -32752), -505),
                    ((-18680, -32748), -501),
                    ((-18678, -32746), -499),
                ],
            ),
        ] {
            assert_eq!(
                type0_slot_model_raw(&model.records, descriptor.records[record].vertex_refs[2]),
                Some(local)
            );
            let mut probes = Vec::new();
            let result = constraint_midpoint_raw(center, up, excess, |x, z| {
                let height = terrain.bilinear_height_raw(x, z);
                probes.push(((x, z), height));
                height
            });
            assert_eq!(probes, expected_probes);
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn animated_primary_uses_retail_interpolation_and_repeated_word_sine_arch() {
        assert_eq!(phase_arch_raw(400, 0x4000), 99);
        let mut record = SubHPhaseRecord {
            flags_raw: LENGTH_CACHE_VALID | PREVIOUSLY_ENABLED,
            phase_raw: 0x4000,
            chord_length_raw: 400,
            two_edge_length_raw: 2_000,
            two_edge_length_squared_raw: 4_000_000,
            ..SubHPhaseRecord::default()
        };
        let endpoint = resolve_geometry_endpoint(
            &mut record,
            geometry_request(SubHEndpoint::Primary, true, 0),
            |role| {
                Some(match role {
                    SubHVertexRole::EndpointC => [1_000, 0, 0],
                    SubHVertexRole::AnchorA => [0, 0, 0],
                    SubHVertexRole::BendB => unreachable!(),
                })
            },
            |_, _| 0,
        )
        .unwrap();
        assert_eq!(endpoint, [500, 99, 0]);
        assert_eq!(record.animated_target_raw, [1_000, 0, 0]);
        assert_eq!(record.flags_raw & PRIMARY_CACHE_VALID, PRIMARY_CACHE_VALID);
    }

    #[test]
    fn secondary_axis_modes_preserve_the_retail_cross_product_sign_pair() {
        let base = SubHPhaseRecord {
            flags_raw: LENGTH_CACHE_VALID | PRIMARY_CACHE_VALID | PREVIOUSLY_ENABLED,
            primary_raw: [100, 0, 0],
            ab_length_squared_raw: 10_000,
            bc_length_squared_raw: 10_000,
            ..SubHPhaseRecord::default()
        };
        let mut axes = SubHModelAxesRaw { axes: [[0; 3]; 3] };
        axes.axes[0][0] = 4095_i32 << 19;
        axes.axes[1][1] = 4095_i32 << 19;
        axes.axes[2][2] = 4095_i32 << 19;

        let solve = |mode| {
            let mut record = base;
            let mut request = geometry_request(SubHEndpoint::Secondary, true, mode);
            request.model_axes_raw = axes;
            let endpoint = resolve_geometry_endpoint(
                &mut record,
                request,
                |role| match role {
                    SubHVertexRole::AnchorA => Some([0, 0, 0]),
                    _ => panic!("cached primary/lengths only require anchor A"),
                },
                |_, _| panic!("secondary-only solve does not sample a surface"),
            )
            .unwrap();
            (endpoint, record)
        };

        let expected = [
            [50, 85, 0],
            [50, 0, 85],
            [50, 0, 0],
            [50, -86, 0],
            [50, 0, -86],
            [50, 0, 0],
        ];
        for (mode, expected_endpoint) in expected.into_iter().enumerate() {
            let (endpoint, record) = solve(mode as u16);
            assert_eq!(endpoint, expected_endpoint, "axis mode {mode}");
            assert_eq!(
                record.flags_raw & SECONDARY_CACHE_VALID,
                SECONDARY_CACHE_VALID,
                "axis mode {mode}"
            );
        }
    }

    #[test]
    fn invalid_or_degenerate_secondary_fails_without_mutating_the_record() {
        let original = SubHPhaseRecord {
            flags_raw: LENGTH_CACHE_VALID | PRIMARY_CACHE_VALID | PREVIOUSLY_ENABLED,
            primary_raw: [100, 0, 0],
            ab_length_squared_raw: 10_000,
            bc_length_squared_raw: 10_000,
            ..SubHPhaseRecord::default()
        };
        let mut invalid = original;
        assert_eq!(
            resolve_geometry_endpoint(
                &mut invalid,
                geometry_request(SubHEndpoint::Secondary, true, 6),
                |role| (role == SubHVertexRole::AnchorA).then_some([0, 0, 0]),
                |_, _| 0,
            ),
            Err(SubHGeometryError::InvalidAxisMode(6))
        );
        assert_eq!(invalid, original);

        let mut degenerate = SubHPhaseRecord {
            primary_raw: [0, 0, 0],
            ..original
        };
        let before = degenerate;
        assert_eq!(
            resolve_geometry_endpoint(
                &mut degenerate,
                geometry_request(SubHEndpoint::Secondary, true, 0),
                |role| (role == SubHVertexRole::AnchorA).then_some([0, 0, 0]),
                |_, _| 0,
            ),
            Err(SubHGeometryError::DegeneratePrimaryChord)
        );
        assert_eq!(degenerate, before);
    }

    #[test]
    fn endpoint_export_rebases_wrapped_8_8_onto_draw_space() {
        let origin_world = [200.0_f32, 4.0, 30.0];
        let origin_raw = [
            ((200.0_f32 * 256.0).round() as i32) as i16,
            ((4.0_f32 * 256.0).round() as i32) as i16,
            ((30.0_f32 * 256.0).round() as i32) as i16,
        ];
        assert!(origin_raw[0] < 0, "200 cells overflows signed 8.8");
        let local = [150, -75, 240];
        let endpoint = [
            i32::from(origin_raw[0].wrapping_add(local[0])),
            i32::from(origin_raw[1].wrapping_add(local[1])),
            i32::from(origin_raw[2].wrapping_add(local[2])),
        ];
        let absolute = endpoint.map(|word| word as f32 / 256.0);
        let draw = endpoint_as_draw_world(origin_world, origin_raw, endpoint);
        assert!(
            (draw[0] - (200.0 + 150.0 / 256.0)).abs() < 1.0e-4,
            "draw {draw:?}"
        );
        assert!(
            (absolute[0] - draw[0]).abs() > 200.0,
            "absolute i16/256 is one world period away ({absolute:?} vs {draw:?})"
        );
    }

    #[test]
    fn endpoint_export_uses_wrapping_short_delta_at_the_signed_8_8_seam() {
        // Level-1 spawn 11 stores +0x9A = 0x7F00 (world Z 127). A +4 world
        // foot wraps the 8.8 word; i32 subtract then lands ~254 world behind.
        let origin_world = [127.0_f32, 4.0, 127.0];
        let origin_raw = [
            ((127.0_f32 * 256.0).round() as i32) as i16,
            ((4.0_f32 * 256.0).round() as i32) as i16,
            ((127.0_f32 * 256.0).round() as i32) as i16,
        ];
        assert_eq!(origin_raw[2], 0x7F00_u16 as i16);
        let local_z = 1024_i16;
        let endpoint = [
            i32::from(origin_raw[0]),
            i32::from(origin_raw[1]),
            i32::from(origin_raw[2].wrapping_add(local_z)),
        ];
        let draw = endpoint_as_draw_world(origin_world, origin_raw, endpoint);
        assert!(
            (draw[2] - (127.0 + f32::from(local_z) / 256.0)).abs() < 1.0e-4,
            "seam foot must stay +4 world, draw={draw:?} endpoint={endpoint:?} origin_raw={origin_raw:?}"
        );
        let naive = origin_world[2] + (endpoint[2] - i32::from(origin_raw[2])) as f32 / 256.0;
        assert!(
            (naive - draw[2]).abs() > 200.0,
            "sign-extended i32 subtract is the sky-cable path ({naive} vs {})",
            draw[2]
        );
    }

    #[test]
    fn selector_world_points_keep_high_x_feet_on_the_draw_origin() {
        let runtime = SubHRuntimeState::new(1).unwrap();
        let descriptor = descriptor(
            None,
            vec![SubHExternalFrameRecord {
                resolver_flags_raw: 0,
                phase_rate_raw: 0,
                vertex_refs: [0, 0, 2],
                axis_mode_raw: 0,
                dependencies: [0; 4],
            }],
        );
        let records = vec![[0, 0, 0, 0], [0, 150, -75, 240]];
        let origin_world = [200.0_f32, 0.0, 0.0];
        let origin_raw = [((200.0_f32 * 256.0).round() as i32) as i16, 0, 0];
        let points = resolve_selector_world_points(
            &runtime,
            &descriptor,
            &records,
            origin_raw,
            origin_world,
            None,
            |_, _| 0,
        )
        .expect("type-0 Sub-H geometry");
        let foot = points[0].expect("primary selector");
        assert!(
            (foot[0] - (200.0 + 150.0 / 256.0)).abs() < 1.0e-3,
            "foot {foot:?} must stay on the body"
        );
        assert!(foot[0] > 199.0, "must not wrap signed 8.8 to negatives");
    }

    #[test]
    fn one_missing_type0_ref_does_not_raw_the_other_records() {
        let runtime = SubHRuntimeState::new(2).unwrap();
        let descriptor = descriptor(
            None,
            vec![
                SubHExternalFrameRecord {
                    resolver_flags_raw: 0,
                    phase_rate_raw: 0,
                    vertex_refs: [0, 0, 2],
                    axis_mode_raw: 0,
                    dependencies: [0; 4],
                },
                SubHExternalFrameRecord {
                    resolver_flags_raw: 0,
                    phase_rate_raw: 0,
                    vertex_refs: [4, 4, 4],
                    axis_mode_raw: 0,
                    dependencies: [0; 4],
                },
            ],
        );
        let records = vec![[0, 0, 0, 0], [0, 150, -75, 240]];
        let points = resolve_selector_world_points(
            &runtime,
            &descriptor,
            &records,
            [0; 3],
            [10.0, 0.0, 0.0],
            None,
            |_, _| 0,
        )
        .expect("partial Sub-H must still return a selector map");
        assert!(points[0].is_some(), "good record keeps primary");
        assert!(points[2].is_some(), "good record keeps secondary");
        assert!(points[1].is_none(), "bad record does not invent a foot");
        assert!(points[3].is_none());
    }

    #[test]
    fn selectors_map_the_two_halves_to_paired_endpoints() {
        assert_eq!(
            selector_binding(0, 8),
            Some(SubHSelectorBinding {
                record_index: 0,
                endpoint: SubHEndpoint::Primary,
            })
        );
        assert_eq!(
            selector_binding(7, 8),
            Some(SubHSelectorBinding {
                record_index: 7,
                endpoint: SubHEndpoint::Primary,
            })
        );
        assert_eq!(
            selector_binding(8, 8),
            Some(SubHSelectorBinding {
                record_index: 0,
                endpoint: SubHEndpoint::Secondary,
            })
        );
        assert_eq!(
            selector_binding(15, 8),
            Some(SubHSelectorBinding {
                record_index: 7,
                endpoint: SubHEndpoint::Secondary,
            })
        );
        assert_eq!(selector_binding(-1, 8), None);
        assert_eq!(selector_binding(16, 8), None);
    }

    #[test]
    fn native_tick952_six_sub_h_records_match_the_same_retail_draw() {
        use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
        // Read-only V2000-01 tick952: actual physical pose, VIEW viewport,
        // H_DESCRIPTOR and H_RECORDS_BEFORE/AFTER from one model302 draw.
        // Only its reached 4x4 terrain-height patch is retained; an unrelated
        // terrain probe must fail rather than consume an invented cell.
        fn captured(words: [i32; 24]) -> SubHPhaseRecord {
            let point = |at| std::array::from_fn(|axis| words[at + axis] as i16);
            SubHPhaseRecord {
                flags_raw: words[0] as u32,
                primary_raw: point(1),
                secondary_raw: point(4),
                terrain_offset_raw: point(7),
                animated_target_raw: point(10),
                phase_origin_raw: point(13),
                phase_raw: words[16] as u16,
                chord_length_raw: words[17] as i16,
                two_edge_length_raw: words[18] as i16,
                excess_length_raw: words[19] as i16,
                excess_length_squared_raw: words[20],
                ab_length_squared_raw: words[21],
                bc_length_squared_raw: words[22],
                two_edge_length_squared_raw: words[23],
            }
        }
        let before = [
            [
                17, -17334, -688, 31293, -17370, -589, 31299, 5, -7, -35, -17334, -688, 31293,
                -17333, -657, 31447, 0, 128, 198, 70, 4900, 8019, 11906, 39204,
            ],
            [
                17, -17530, -614, 31550, -17511, -566, 31453, -16, 6, -30, -17530, -612, 31550,
                -17505, -624, 31646, 0, 129, 197, 68, 4624, 7638, 12114, 38809,
            ],
            [
                25, -17291, -680, 31348, -17381, -606, 31324, -17, -5, -24, -17286, -686, 31350,
                -17361, -669, 31339, 0, 151, 209, 58, 3364, 8404, 14027, 43681,
            ],
            [
                25, -17633, -588, 31517, -17564, -566, 31424, -3, -1, -29, -17635, -585, 31521,
                -17472, -631, 31686, 0, 150, 208, 58, 3364, 8030, 14242, 43264,
            ],
            [
                17, -17420, -676, 31220, -17422, -579, 31237, 0, -4, -32, -17420, -680, 31220,
                -17397, -661, 31347, 0, 129, 194, 65, 4225, 7707, 11645, 37636,
            ],
            [
                17, -17639, -610, 31378, -17589, -527, 31361, -21, 7, -25, -17639, -617, 31378,
                -17532, -614, 31567, 0, 128, 195, 67, 4489, 7589, 11674, 38025,
            ],
        ]
        .map(captured);
        let expected = [
            [
                23, -17334, -682, 31293, -17381, -587, 31289, 5, -7, -35, -17334, -688, 31293,
                -17333, -657, 31447, 0, 128, 198, 70, 4900, 8019, 11906, 39204,
            ],
            [
                31, -17529, -608, 31529, -17521, -567, 31426, -11, 4, -32, -17530, -612, 31550,
                -17505, -624, 31646, 0, 129, 197, 68, 4624, 7638, 12114, 38809,
            ],
            [
                31, -17308, -671, 31341, -17398, -601, 31307, -17, -5, -24, -17286, -686, 31350,
                -17361, -669, 31339, 0, 151, 209, 58, 3364, 8404, 14027, 43681,
            ],
            [
                31, -17627, -586, 31505, -17571, -548, 31405, -3, -1, -29, -17635, -585, 31521,
                -17472, -631, 31686, 0, 150, 208, 58, 3364, 8030, 14242, 43264,
            ],
            [
                31, -17420, -676, 31220, -17425, -579, 31227, -24, -4, -22, -17420, -680, 31220,
                -17397, -661, 31347, 0, 129, 194, 65, 4225, 7707, 11645, 37636,
            ],
            [
                23, -17639, -610, 31378, -17598, -525, 31353, -21, 7, -25, -17639, -617, 31378,
                -17532, -614, 31567, 0, 128, 195, 67, 4489, 7589, 11674, 38025,
            ],
        ]
        .map(captured);
        let mut model_records = vec![[0; 4]; 75];
        for (index, record) in [
            (1, [0, 36, 0, -7]),
            (43, [0, 35, 0, 21]),
            (44, [0, 98, 56, 49]),
            (45, [0, 105, 56, -14]),
            (46, [0, 84, 56, -56]),
            (47, [0, 133, -35, 98]),
            (48, [0, 182, -35, -14]),
            (49, [0, 133, -35, -84]),
            (74, [0, 35, 0, 0]),
        ] {
            model_records[index] = record;
        }
        let descriptor = descriptor(
            None,
            [
                ([86, 88, 94], [1, 2, 3, 4]),
                ([87, 89, 95], [0, 2, 3, 5]),
                ([148, 90, 96], [0, 1, 4, 5]),
                ([149, 91, 97], [0, 1, 4, 5]),
                ([2, 92, 98], [0, 2, 3, 5]),
                ([3, 93, 99], [1, 2, 3, 4]),
            ]
            .map(|(vertex_refs, dependencies)| SubHExternalFrameRecord {
                resolver_flags_raw: 0x2000_0000,
                phase_rate_raw: 0x3000_0000,
                vertex_refs,
                axis_mode_raw: 0,
                dependencies,
            })
            .to_vec(),
        );
        let origin = [-17501, -551, 31307];
        let axes = [
            [1621229568, -462225408, -1329135616],
            [398852096, 2095841280, -242089984],
            [1349451776, -64225280, 1668481024],
        ];
        let viewport = NativeWorldViewport {
            origin_raw: [-15097, -243, 28884],
            axes_q31: [
                [2105482641, 0, 422129991],
                [-61663396, 2124313119, 307562155],
                [-417665851, -313697527, 2082951585],
            ],
            identity: false,
        };
        let mut terrain = surface_terrain(0, 0);
        // Original full-terrain SHA256
        // 3000534ac8a2488b147060e790952afdb1f8df793a576722267545937a5489f5.
        for [x, z, height] in [
            [186, 121, 233],
            [186, 122, 236],
            [186, 123, 238],
            [186, 124, 237],
            [187, 121, 233],
            [187, 122, 236],
            [187, 123, 238],
            [187, 124, 237],
            [188, 121, 232],
            [188, 122, 235],
            [188, 123, 236],
            [188, 124, 236],
            [189, 121, 231],
            [189, 122, 233],
            [189, 123, 235],
            [189, 124, 237],
        ] {
            terrain.cells[x * 256 + z].height = height as u8;
        }
        let mut runtime = SubHRuntimeState::new(6).unwrap();
        runtime.records_mut().copy_from_slice(&before);
        runtime.set_enabled(true);
        runtime.set_surface_policy(SubHSurfacePolicy::Terrain);
        let mut probes = 0;
        let points = SubHPresentation {
            runtime: &mut runtime,
            descriptor: &descriptor,
            model_records: &model_records,
            retail_tick: 952,
            origin_raw: origin,
            origin_world: origin.map(|v| f32::from(v) / 256.0),
            body_axes_q31: Some(axes),
            native_context: Some((
                NativeModelFrame::from_actor(viewport, origin, axes),
                viewport,
            )),
            fallback: None,
            emitter: None,
        }
        .resolve([0, 2, 4, 1, 3, 5, 6, 7, 8, 9, 10, 11], |x, z| {
            assert!(
                (186..=188).contains(&(usize::from(x as u16) >> 8)),
                "unowned terrain X {x}"
            );
            assert!(
                (121..=123).contains(&(usize::from(z as u16) >> 8)),
                "unowned terrain Z {z}"
            );
            probes += 1;
            terrain.bilinear_height_raw(x, z)
        })
        .unwrap();
        assert_eq!(probes, 22);
        assert!(points[..12].iter().all(Option::is_some));
        assert!(points[12..].iter().all(Option::is_none));
        assert_eq!(
            runtime.records(),
            &expected,
            "every live cache field belongs to this same native draw"
        );
    }

    #[test]
    fn native_draw_type0_vertices_preserve_the_captured_view_round_trip() {
        use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
        // Accepted V2000-01 tick952 model302 draw: physical pose and viewport
        // are independent inputs. All twelve cached VIEW points match retail.
        let origin = [-17501, -551, 31307];
        let axes = [
            [1621229568, -462225408, -1329135616],
            [398852096, 2095841280, -242089984],
            [1349451776, -64225280, 1668481024],
        ];
        let viewport = NativeWorldViewport {
            origin_raw: [-15097, -243, 28884],
            axes_q31: [
                [2105482641, 0, 422129991],
                [-61663396, 2124313119, 307562155],
                [-417665851, -313697527, 2082951585],
            ],
            identity: false,
        };
        let frame = NativeModelFrame::from_actor(viewport, origin, axes);
        assert_eq!(frame.origin_view_raw, [-1881, 111, 2861]);
        let mut records = vec![[0; 4]; 75];
        for (index, record) in [
            (43, [0, 35, 0, 21]),
            (47, [0, 133, -35, 98]),
            (74, [0, 35, 0, 0]),
            (48, [0, 182, -35, -14]),
            (1, [0, 36, 0, -7]),
            (49, [0, 133, -35, -84]),
        ] {
            records[index] = record;
        }
        for (slot, expected_view, expected_world) in [
            (86, [-1844, 100, 2848], [-17462, -562, 31297]),
            (87, [-1886, 124, 2900], [-17516, -545, 31342]),
            (94, [-1730, 40, 2837], [-17348, -619, 31299]),
            (95, [-1894, 126, 3029], [-17548, -562, 31466]),
            (148, [-1860, 99, 2835], [-17476, -561, 31281]),
            (149, [-1902, 123, 2887], [-17528, -544, 31327]),
            (96, [-1786, 17, 2731], [-17382, -626, 31182]),
            (97, [-2010, 135, 2993], [-17655, -548, 31410]),
            (2, [-1865, 98, 2830], [-17480, -561, 31275]),
            (3, [-1909, 122, 2882], [-17534, -544, 31320]),
            (98, [-1870, 28, 2721], [-17462, -614, 31159]),
            (99, [-2034, 114, 2913], [-17663, -557, 31325]),
        ] {
            assert_eq!(
                frame.resolve_type0_slot_view_raw(&records, slot),
                Some(expected_view)
            );
            let descriptor = descriptor(
                None,
                vec![SubHExternalFrameRecord {
                    vertex_refs: [slot; 3],
                    ..authored_record(0, [0; 4])
                }],
            );
            let mut runtime = SubHRuntimeState::new(1).unwrap();
            runtime.set_enabled(false);
            SubHPresentation {
                runtime: &mut runtime,
                descriptor: &descriptor,
                model_records: &records,
                retail_tick: 952,
                origin_raw: origin,
                origin_world: origin.map(|v| f32::from(v) / 256.0),
                body_axes_q31: Some(axes),
                native_context: Some((frame.clone(), viewport)),
                fallback: None,
                emitter: None,
            }
            .resolve([0], |_, _| panic!("disabled endpoint does not use terrain"))
            .unwrap();
            assert_eq!(runtime.records()[0].primary_raw, expected_world);
        }
        // This fractional odd-X product truncates before its sign is reversed.
        // Negating the local coordinate before multiplication gives another VIEW point.
        let local = type0_slot_model_raw(&records, 87).unwrap();
        assert_ne!(
            rotate_q31_world_8_8(origin, axes, local),
            [-17516, -545, 31342]
        );
    }

    #[test]
    fn presentation_commits_only_the_requested_endpoint_and_record() {
        let authored = SubHExternalFrameRecord {
            vertex_refs: [0, 2, 4],
            ..authored_record(65_536, [0; 4])
        };
        let descriptor = descriptor(None, vec![authored; 2]);
        let records = [[0, 0, 0, 0], [0, 0, 100, 0], [0, 100, 100, 0]];
        let mut runtime = SubHRuntimeState::new(2).unwrap();
        runtime.set_enabled(false);
        let points = SubHPresentation {
            runtime: &mut runtime,
            descriptor: &descriptor,
            model_records: &records,
            retail_tick: 0,
            origin_raw: [0; 3],
            origin_world: [0.0; 3],
            body_axes_q31: None,
            native_context: None,
            fallback: None,
            emitter: None,
        }
        .resolve([0, -1, 16], |_, _| {
            panic!("disabled draw does not sample terrain")
        })
        .unwrap();
        assert_eq!(points[0], Some([100.0 / 256.0, 100.0 / 256.0, 0.0]));
        assert!(points[1..].iter().all(Option::is_none));
        assert_eq!(
            runtime.records[0].flags_raw,
            LENGTH_CACHE_VALID | PRIMARY_CACHE_VALID
        );
        assert_eq!(runtime.records[0].phase_raw, 0);
        assert_eq!(runtime.records[1], SubHPhaseRecord::default());

        let before = runtime.clone();
        let points = SubHPresentation {
            runtime: &mut runtime,
            descriptor: &descriptor,
            model_records: &records,
            retail_tick: 0,
            origin_raw: [0; 3],
            origin_world: [0.0; 3],
            body_axes_q31: None,
            native_context: None,
            fallback: None,
            emitter: None,
        }
        .resolve([], |_, _| panic!("no selected geometry"))
        .unwrap();
        assert_eq!(points, [None; 16]);
        assert_eq!(runtime, before);
    }

    #[test]
    fn presentation_secondary_performs_its_own_primary_dependency_once() {
        let authored = SubHExternalFrameRecord {
            vertex_refs: [0, 2, 4],
            ..authored_record(65_536, [0; 4])
        };
        let descriptor = descriptor(None, vec![authored; 2]);
        let records = [[0, 0, 0, 0], [0, 0, 100, 0], [0, 100, 100, 0]];
        let mut runtime = SubHRuntimeState::new(2).unwrap();
        runtime.set_enabled(false);
        let points = SubHPresentation {
            runtime: &mut runtime,
            descriptor: &descriptor,
            model_records: &records,
            retail_tick: 0,
            origin_raw: [0; 3],
            origin_world: [0.0; 3],
            body_axes_q31: None,
            native_context: None,
            fallback: None,
            emitter: None,
        }
        .resolve([3], |_, _| 0)
        .unwrap();
        assert!(points[3].is_some());
        assert!(points[0..3].iter().all(Option::is_none));
        assert_eq!(runtime.records[0], SubHPhaseRecord::default());
        assert_eq!(runtime.records[1].flags_raw, 7);
        let before = runtime.clone();
        let warm_points = SubHPresentation {
            runtime: &mut runtime,
            descriptor: &descriptor,
            model_records: &[],
            retail_tick: 0,
            origin_raw: [0; 3],
            origin_world: [0.0; 3],
            body_axes_q31: None,
            native_context: None,
            fallback: None,
            emitter: None,
        }
        .resolve([3], |_, _| panic!("warm cache"))
        .unwrap();
        assert_eq!(warm_points, points);
        assert_eq!(runtime, before);
    }

    #[test]
    fn phase_delta_and_completion_pitch_match_retail_integer_expressions() {
        assert_eq!(phase_delta_raw(1_000_000, 65_536), 30);
        assert_eq!(phase_delta_raw(1_000_000, -65_536), (-31_i32) as u32);
        assert_eq!(completion_sound_pitch_16_16(0x1234_abcd), 0x10af3);
    }

    #[test]
    fn d360_writeback_sets_copy_target_so_the_writer_can_start_a_stride() {
        let mut runtime = SubHRuntimeState::new(1).unwrap();
        let authored = SubHExternalFrameRecord {
            resolver_flags_raw: 0,
            phase_rate_raw: 0x3000_0000,
            vertex_refs: [0, 0, 2],
            axis_mode_raw: 0,
            dependencies: [0; 4],
        };
        let descriptor = descriptor(None, vec![authored]);
        let records = vec![[0, 0, 0, 0], [0, 150, 100, 240]];
        let identity = [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]];
        assert_eq!(runtime.records[0].flags_raw, 0);
        assert_eq!(runtime.records[0].phase_raw, 0);

        commit_sub_h_geometry(
            &mut runtime,
            &descriptor,
            &records,
            [0; 3],
            Some(identity),
            |_, _| 0,
        )
        .unwrap();
        runtime.update(&descriptor, 19_500).unwrap();
        commit_sub_h_geometry(
            &mut runtime,
            &descriptor,
            &records,
            [0; 3],
            Some(identity),
            |_, _| 0,
        )
        .unwrap();
        runtime.update(&descriptor, 19_500).unwrap();

        assert_ne!(
            runtime.records[0].phase_raw, 0,
            "D360 cache bits alias COPY_TARGET/WAIT_FOR_DEPS so D0A0 can leave rest pose"
        );
    }

    #[test]
    fn writer_advances_phase_and_rotates_the_first_record() {
        let authored = descriptor(
            None,
            vec![
                authored_record(65_536, [0; 4]),
                authored_record(65_536, [0; 4]),
            ],
        );
        let mut runtime = SubHRuntimeState::new(2).unwrap();
        runtime.records[0].phase_raw = 10;
        runtime.records[0].flags_raw = 0xffff_ffff;
        runtime.records[1].phase_raw = 20;
        runtime.records[1].flags_raw = 0xffff_ffff;

        assert_eq!(runtime.update(&authored, 1_000_000).unwrap(), []);
        assert_eq!(runtime.records[0].phase_raw, 40);
        assert_eq!(runtime.records[1].phase_raw, 50);
        assert_eq!(runtime.records[0].flags_raw, 0xffff_fff9);
        assert_eq!(runtime.records[1].flags_raw, 0xffff_fff9);
        assert_eq!(runtime.cursor(), 1);
    }

    #[test]
    fn phase_completion_copies_target_and_returns_cues_in_rotated_order() {
        let authored = descriptor(
            Some(72),
            vec![
                authored_record(65_536, [0; 4]),
                authored_record(65_536, [0; 4]),
            ],
        );
        let mut runtime = SubHRuntimeState::new(2).unwrap();
        runtime.cursor = 1;
        for (index, record) in runtime.records.iter_mut().enumerate() {
            record.phase_raw = 0x7fff;
            record.flags_raw = COPY_TARGET_ON_COMPLETION | 0x80;
            record.primary_raw = [1, 2, 3];
            record.animated_target_raw = [10 + index as i16, 20, 30];
        }

        assert_eq!(
            runtime.update(&authored, 1_000_000).unwrap(),
            [
                SubHCompletionCue {
                    record_index: 1,
                    sound_id: 72,
                },
                SubHCompletionCue {
                    record_index: 0,
                    sound_id: 72,
                },
            ]
        );
        assert_eq!(runtime.records[0].primary_raw, [10, 20, 30]);
        assert_eq!(runtime.records[1].primary_raw, [11, 20, 30]);
        assert_eq!(runtime.records[0].phase_raw, 0);
        assert_eq!(runtime.records[1].phase_raw, 0);
    }

    #[test]
    fn dependency_gate_reads_mutated_phases_in_dispatch_order() {
        let authored = descriptor(
            None,
            vec![
                authored_record(65_536, [1; 4]),
                authored_record(65_536, [0; 4]),
            ],
        );
        let mut runtime = SubHRuntimeState::new(2).unwrap();
        runtime.records[0].flags_raw = COPY_TARGET_ON_COMPLETION | WAIT_FOR_DEPENDENCIES;
        runtime.records[1].flags_raw = COPY_TARGET_ON_COMPLETION | WAIT_FOR_DEPENDENCIES;
        runtime.records[0].primary_raw = [7, 8, 9];

        runtime.update(&authored, 1_000_000).unwrap();
        assert_eq!(runtime.records[0].phase_raw, 30);
        assert_eq!(runtime.records[0].phase_origin_raw, [7, 8, 9]);
        assert_eq!(
            runtime.records[1].phase_raw, 0,
            "record 1 observes record 0's newly started phase"
        );
    }

    #[test]
    fn idle_no_copy_path_preserves_secondary_cache_while_clearing_terrain_state() {
        let authored = descriptor(None, vec![authored_record(65_536, [0; 4])]);
        let mut runtime = SubHRuntimeState::new(1).unwrap();
        runtime.records[0].flags_raw = LENGTH_CACHE_VALID
            | SECONDARY_CACHE_VALID
            | TERRAIN_OFFSET_VALID
            | PREVIOUSLY_ENABLED
            | 0x80;

        runtime.update(&authored, 1_000_000).unwrap();
        assert_eq!(
            runtime.records[0].flags_raw,
            LENGTH_CACHE_VALID | SECONDARY_CACHE_VALID | 0x80
        );
    }

    #[test]
    fn disabled_writer_clears_only_transient_flags_and_keeps_cursor_and_phase() {
        let authored = descriptor(None, vec![authored_record(65_536, [0; 4])]);
        let mut runtime = SubHRuntimeState::new(1).unwrap();
        runtime.records[0].flags_raw = 0xffff_ffff;
        runtime.records[0].phase_raw = 123;
        runtime.set_enabled(false);

        assert_eq!(runtime.update(&authored, 1_000_000).unwrap(), []);
        assert_eq!(runtime.records[0].flags_raw, 0xffff_ffe1);
        assert_eq!(runtime.records[0].phase_raw, 123);
        assert_eq!(runtime.cursor(), 0);
    }

    #[test]
    fn endpoint_reader_uses_the_recovered_selector_pairing() {
        let mut runtime = SubHRuntimeState::new(1).unwrap();
        runtime.records[0].primary_raw = [1, 2, 3];
        runtime.records[0].secondary_raw = [4, 5, 6];
        assert_eq!(runtime.endpoint_raw(0), Some([1, 2, 3]));
        assert_eq!(runtime.endpoint_raw(1), Some([4, 5, 6]));
        assert_eq!(runtime.endpoint_raw(2), None);
    }
}

#[cfg(test)]
mod native_external_point_image_tests {
    use super::*;
    use crate::actor_external_frame_fallback::{
        ActorExternalFrameFallback, ActorExternalFrameReservations,
    };
    use crate::entity_collision_state::RetailRuntimeValue;
    use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
    use v2k_formats::models::AnimVars;

    const AXES: [[i32; 3]; 3] = [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]];
    const ACTOR: [i16; 3] = [35748u16 as i16, -318, 32648];
    const DRAW: [f32; 3] = [139.640625, -1.2421875, 127.53125];
    const PRIMARY: [i16; 3] = [-29607, -353, 32634];
    const EXPECTED: [f32; 3] = [140.34765625, -1.37890625, 383.4765625];

    fn fixture() -> crate::session::GameSession {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").is_file(),
            "required retail assets cannot silently skip"
        );
        let mut session = crate::session::GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(50, 1).unwrap();
        session
    }

    fn viewport(signed_source: bool) -> NativeWorldViewport {
        NativeWorldViewport {
            origin_raw: if signed_source {
                [-19805, 80, -120]
            } else {
                [45731, 80, 65416]
            },
            axes_q31: AXES,
            identity: true,
        }
    }

    #[v2k_test_support::retail_test]
    fn actual_model302_cached_h_endpoint_selects_its_own_viewport_image() {
        let session = fixture();
        let model = session.cache.global_model(302).unwrap();
        let descriptor = session
            .cache
            .global_entity_type(53)
            .unwrap()
            .sub_h_external_frame_descriptor()
            .unwrap();
        assert_eq!(
            model.records[57],
            [14, 2, 0, 0],
            "actual slot114 is H primary selector2"
        );
        assert_eq!(descriptor.records.len(), 6);
        for signed_source in [false, true] {
            let viewport = viewport(signed_source);
            let frame = NativeModelFrame::from_actor(viewport, ACTOR, AXES);
            let mut runtime = SubHRuntimeState::new(descriptor.records.len()).unwrap();
            // Exact primary/flags recorded by the completed controlled draw.
            runtime.records_mut()[2].flags_raw = 23;
            runtime.records_mut()[2].primary_raw = PRIMARY;
            let before = runtime.clone();
            let mut presentation = SubHPresentation {
                runtime: &mut runtime,
                descriptor: &descriptor,
                model_records: &model.records,
                retail_tick: 2370,
                origin_raw: ACTOR,
                origin_world: DRAW,
                body_axes_q31: Some(AXES),
                native_context: Some((frame, viewport)),
                fallback: None,
                emitter: None,
            };
            let points = presentation
                .resolve([2], |_, _| panic!("warm primary cannot sample terrain"))
                .unwrap();
            assert_eq!(points[2], Some(EXPECTED));
            let source_image = viewport.actor_world_image(PRIMARY);
            let view = viewport.world_vector_to_view(std::array::from_fn(|axis| {
                source_image[axis].wrapping_sub(viewport.origin_raw[axis])
            }));
            assert_eq!(
                view,
                [-9802, -433, 32754],
                "matches sealed native command slot114 VIEW"
            );
            assert_eq!(
                runtime, before,
                "display-image selection cannot mutate warm H"
            );
        }
    }

    #[v2k_test_support::retail_test]
    fn none_keeps_compatibility_actor_image_and_native_policy_wraps_all_axes() {
        let session = fixture();
        let model = session.cache.global_model(302).unwrap();
        let descriptor = session
            .cache
            .global_entity_type(53)
            .unwrap()
            .sub_h_external_frame_descriptor()
            .unwrap();
        let mut runtime = SubHRuntimeState::new(descriptor.records.len()).unwrap();
        let mut presentation = SubHPresentation {
            runtime: &mut runtime,
            descriptor: &descriptor,
            model_records: &model.records,
            retail_tick: 2370,
            origin_raw: ACTOR,
            origin_world: DRAW,
            body_axes_q31: Some(AXES),
            native_context: None,
            fallback: None,
            emitter: None,
        };
        assert_eq!(
            presentation.endpoint_as_presentation_world(PRIMARY.map(i32::from)),
            [140.34765625, -1.37890625, 127.4765625]
        );
        let viewport = NativeWorldViewport {
            origin_raw: [32767, -32768, 65535],
            axes_q31: AXES,
            identity: true,
        };
        presentation.origin_raw = [32767, -32768, -1];
        presentation.origin_world = [32767.0 / 256.0, -32768.0 / 256.0, 65535.0 / 256.0];
        presentation.native_context = Some((
            NativeModelFrame::from_actor(viewport, presentation.origin_raw, AXES),
            viewport,
        ));
        assert_eq!(
            presentation.endpoint_as_presentation_world([-32768, 32767, 0]),
            [128.0, -32769.0 / 256.0, 256.0]
        );
    }

    #[v2k_test_support::retail_test]
    fn terminal_relation_suffix_uses_same_policy_and_keeps_three_draws() {
        let session = fixture();
        let model = session.cache.global_model(302).unwrap();
        let descriptor = session
            .cache
            .global_entity_type(53)
            .unwrap()
            .sub_h_external_frame_descriptor()
            .unwrap();
        let viewport = viewport(true);
        let mut runtime = SubHRuntimeState::new(descriptor.records.len()).unwrap();
        let mut calls = 0;
        let mut random = || {
            calls += 1;
            0u16
        };
        let fallback = ActorExternalFrameFallback {
            reservations: ActorExternalFrameReservations {
                sub_h_records: 6,
                sub_e_joint_slots: 0,
                sub_m_present: false,
                live_sub_g_present: false,
            },
            own_origin_raw: ACTOR,
            relation_origin_raw: RetailRuntimeValue::Known(Some(PRIMARY)),
            random_u16: &mut random,
            last_boundary: None,
        };
        let mut presentation = SubHPresentation {
            runtime: &mut runtime,
            descriptor: &descriptor,
            model_records: &model.records,
            retail_tick: 2370,
            origin_raw: ACTOR,
            origin_world: DRAW,
            body_axes_q31: Some(AXES),
            native_context: Some((
                NativeModelFrame::from_actor(viewport, ACTOR, AXES),
                viewport,
            )),
            fallback: Some(fallback),
            emitter: None,
        };
        assert_eq!(
            presentation.resolve_post_h_point([12, 0, 0]),
            Some((EXPECTED, PRIMARY.map(i32::from)))
        );
        assert_eq!(
            presentation.resolve_fallback_world([12, 0, 0]),
            Some(EXPECTED)
        );
        drop(presentation);
        assert_eq!(
            calls, 6,
            "each reached helper still invokes exactly three source RNG draws"
        );
    }

    #[v2k_test_support::retail_test]
    fn missing_terminal_relation_returns_none_without_draw_or_borrowed_origin() {
        let session = fixture();
        let model = session.cache.global_model(302).unwrap();
        let descriptor = session
            .cache
            .global_entity_type(53)
            .unwrap()
            .sub_h_external_frame_descriptor()
            .unwrap();
        let viewport = viewport(true);
        let mut runtime = SubHRuntimeState::new(descriptor.records.len()).unwrap();
        let before = runtime.clone();
        let mut calls = 0;
        let mut random = || {
            calls += 1;
            0u16
        };
        let mut presentation = SubHPresentation {
            runtime: &mut runtime,
            descriptor: &descriptor,
            model_records: &model.records,
            retail_tick: 2370,
            origin_raw: ACTOR,
            origin_world: DRAW,
            body_axes_q31: Some(AXES),
            native_context: Some((
                NativeModelFrame::from_actor(viewport, ACTOR, AXES),
                viewport,
            )),
            fallback: Some(ActorExternalFrameFallback {
                reservations: ActorExternalFrameReservations {
                    sub_h_records: 6,
                    sub_e_joint_slots: 0,
                    sub_m_present: false,
                    live_sub_g_present: false,
                },
                own_origin_raw: ACTOR,
                relation_origin_raw: RetailRuntimeValue::Unresolved,
                random_u16: &mut random,
                last_boundary: None,
            }),
            emitter: None,
        };
        assert_eq!(presentation.resolve_post_h_point([12, 0, 0]), None);
        assert_eq!(presentation.fallback.as_ref().unwrap().last_boundary, Some(crate::actor_external_frame_fallback::ActorExternalFrameFallbackBoundary::UnresolvedRelation));
        drop(presentation);
        assert_eq!(calls, 0);
        assert_eq!(runtime, before);
    }

    #[v2k_test_support::retail_test]
    fn actual_model302_emitter_keeps_source_stamp_and_independent_display_image() {
        use crate::actor_emitter_external_frame::{
            resolve_actor_emitter_external_frame, ActorEmitterExternalFramePresentation,
        };
        let session = fixture();
        let model = session.cache.global_model(302).unwrap();
        let ty = session.cache.global_entity_type(47).unwrap();
        let descriptor = ty.sub_h_external_frame_descriptor().unwrap();
        let emitter_descriptor = ty.projectile_emitter_descriptor().unwrap();
        assert_eq!(emitter_descriptor.raw_word_at_0x12, 150);
        let viewport = viewport(true);
        let reverse = [[-i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, -i32::MAX]];
        let frame = NativeModelFrame::from_actor(viewport, ACTOR, reverse);
        let vars = AnimVars::default();
        let point = resolve_actor_emitter_external_frame(
            6,
            emitter_descriptor,
            [12, 0, 0],
            model,
            &vars,
            &frame,
            viewport,
        )
        .unwrap()
        .unwrap();
        let image = viewport.actor_world_image(point.position_words);
        let actor_image = viewport.actor_world_image(ACTOR);
        let expected: [f32; 3] = std::array::from_fn(|axis| {
            DRAW[axis] + image[axis].wrapping_sub(actor_image[axis]) as f32 / 256.0
        });
        assert_ne!(
            expected,
            endpoint_as_draw_world(DRAW, ACTOR, point.position_raw),
            "controlled reversed emitter crosses its actor's image branch"
        );
        let mut runtime = SubHRuntimeState::new(descriptor.records.len()).unwrap();
        let mut stamps = Vec::new();
        let mut stamp = |point| stamps.push(point);
        let mut boundary = None;
        let mut presentation = SubHPresentation {
            runtime: &mut runtime,
            descriptor: &descriptor,
            model_records: &model.records,
            retail_tick: 2370,
            origin_raw: ACTOR,
            origin_world: DRAW,
            body_axes_q31: Some(reverse),
            native_context: Some((frame.clone(), viewport)),
            fallback: None,
            emitter: Some(ActorEmitterExternalFramePresentation {
                descriptor: emitter_descriptor,
                root_model: model,
                root_vars: &vars,
                frame,
                viewport,
                stamp_origin: &mut stamp,
                last_boundary: &mut boundary,
                current_node_owned: true,
                current_source_points_view: None,
            }),
        };
        assert_eq!(
            presentation.resolve_post_h_point([12, 0, 0]),
            Some((expected, point.position_raw))
        );
        drop(presentation);
        assert_eq!(
            stamps,
            [point],
            "source shot-node origin stamp stays in native raw words"
        );
        assert_eq!(boundary, None);
    }
}
