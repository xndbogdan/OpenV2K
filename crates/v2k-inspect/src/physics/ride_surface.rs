use serde::Serialize;

use super::{read_pointer_window, read_u32_optional, VtolReadWindow, VtolRuntimeValues};
use crate::entity::{EntityRecord, TICK_50HZ};
use crate::process::{i16_at, plausible_heap_pointer, Process};

const WAVE_ENABLE: usize = 0x004F_ECE4;
const TERRAIN_POOL_POINTER: usize = 0x004F_E648;
const CURRENT_TERRAIN_INDEX: usize = 0x004F_EC40;
const STATIC_OBJECT_POOL_POINTER: usize = 0x004F_E644;
const CURRENT_STATIC_OBJECT_INDEX: usize = 0x004F_EC44;
const MODEL_POOL_POINTER: usize = 0x004F_E640;
const RETAIL_SINE_TABLE: usize = 0x004D_14D0;
const FUN_VTOL_UPDATE: u32 = 0x0044_5310;
const FUN_TERRAIN_HEIGHT: u32 = 0x0044_5860;
const FUN_WAVE_HEIGHT: u32 = 0x0044_5920;
const FUN_LIFT_PROJECTION: u32 = 0x0041_B210;
const FUN_LIFT_TERRAIN_HEIGHT: u32 = 0x0041_DE60;
const MAX_TERRAIN_RESOURCES: u32 = 4096;
const TERRAIN_HEADER_BYTES: usize = 0x14;
const TERRAIN_GRID_SIZE: u32 = 0x100;
const TERRAIN_CELL_BYTES: u32 = 3;
const STATIC_OBJECT_DESCRIPTOR_BYTES: usize = 0x0C;
const MODEL_HEADER_BYTES: usize = 0x0C;
const MAX_STATIC_OBJECT_SCAN_AXIS_CELLS: usize = 32;

/// One center/cardinal surface probe from the type-46 five-probe lift-limit
/// branch in `FUN_0041B210`. Ordered arrays avoid repeating verbose role/term
/// provenance at capture frequency. Their orders are recorded once in the
/// capture metadata.
#[derive(Debug, Clone, Serialize, Default)]
pub struct VtolLiftSurfaceProbeEvidence {
    pub offset_raw_xz: [i16; 2],
    pub cell_xz: [u8; 2],
    pub fraction_xz: [u8; 2],
    pub terrain_corner_addresses: [u32; 4],
    /// Exact Section-10 triples in x0_z0, x1_z0, x0_z1, x1_z1 order:
    /// signed height, static-object attribute, and packed material/shade byte.
    pub terrain_corner_cell_bytes: [Option<[u8; 3]>; 4],
    pub terrain_corner_height_bytes: [Option<u8>; 4],
    #[serde(skip_serializing_if = "all_none")]
    pub terrain_corner_read_errors: [Option<String>; 4],
    pub terrain_surface_raw: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wave_sample: Option<VtolWaveProbeEvidence>,
    pub wave_surface_raw: Option<i16>,
    pub ride_surface_raw: Option<i32>,
}

/// Compact raw sine-table evidence for one wave probe. Arrays are ordered as
/// X, X+Z, Z-X. `quadrant_negated_mask` bit N applies to entry N.
#[derive(Debug, Clone, Serialize)]
pub struct VtolWaveProbeEvidence {
    pub wrapped_phases: [u32; 3],
    pub table_addresses: [u32; 3],
    pub quadrant_negated_mask: u8,
    pub table_values_q15: [Option<u16>; 3],
    #[serde(skip_serializing_if = "all_none")]
    pub read_errors: [Option<String>; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TerrainCornerRead {
    address: u32,
    cell_bytes: Option<[u8; 3]>,
    read_error: Option<String>,
}

/// One non-empty Section-10 cell in the bounded, retail radius-derived
/// static-object scan around the tracked craft. The selected model follows the
/// retail Section-9 lookup `(terrain_state >> 3) & 3`; no collision callbacks
/// are invoked.
#[derive(Debug, Clone, Serialize)]
pub struct VtolStaticObjectCellEvidence {
    pub cell_xz: [u8; 2],
    pub cell_address: u32,
    pub height_byte: u8,
    pub attribute: u8,
    pub terrain_state: u8,
    pub descriptor_address: Option<u32>,
    pub descriptor_model_ids: Option<[u16; 4]>,
    pub object_kind_index: Option<u32>,
    pub selected_model_slot: usize,
    pub selected_model_id: Option<u16>,
    pub selected_model_resource_pointer: Option<u32>,
    pub selected_model_collision_radius_raw_at_0x0a: Option<u16>,
    pub read_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct VtolStaticObjectScanEvidence {
    pub static_object_pool_pointer_address: u32,
    pub static_object_pool_pointer: Option<u32>,
    pub current_static_object_index_address: u32,
    pub current_static_object_index: Option<u32>,
    pub static_object_table_pointer_slot_address: Option<u32>,
    pub static_object_table_pointer: Option<u32>,
    pub model_pool_pointer_address: u32,
    pub model_pool_pointer: Option<u32>,
    pub center_cell_xz: Option<[u8; 2]>,
    pub active_model_collision_radius_raw: Option<u16>,
    pub retail_scan_radius_raw: Option<i16>,
    pub scan_start_raw_xz: Option<[i16; 2]>,
    pub scan_span_raw: Option<u16>,
    pub candidate_axis_cells: Option<usize>,
    pub nonempty_cells: Vec<VtolStaticObjectCellEvidence>,
}

/// Focused, read-only evidence for the retail static-scenery collision
/// candidate scan around one already-decoded entity.
///
/// This deliberately excludes VTOL runtime, lift, wave, and terrain-height
/// sampling. It resolves only the current Section-10 terrain resource and
/// reuses the bounded Section-9 candidate lookup performed by
/// `read_static_object_scan`; no game callback is invoked.
#[derive(Debug, Clone, Serialize)]
pub struct NearbyStaticCollisionScanEvidence {
    pub entity_pointer: u32,
    pub terrain_pool_pointer_address: u32,
    pub terrain_pool_pointer: Option<u32>,
    pub current_terrain_index_address: u32,
    pub current_terrain_index: Option<u32>,
    pub terrain_pointer_slot_address: Option<u32>,
    pub terrain_pointer: Option<u32>,
    pub static_object_scan: VtolStaticObjectScanEvidence,
    pub warnings: Vec<String>,
}

/// One read-only terrain/wave sample at an arbitrary signed-8.8 world
/// position.
///
/// This is intentionally independent of the player model radius and the
/// five-probe VTOL lift branch. It lets camera captures retain the local
/// surface directly beneath the sampled eye instead of substituting the
/// craft's ride surface.
#[derive(Debug, Clone, Serialize)]
pub struct WorldSurfaceProbeEvidence {
    pub tick_address: u32,
    pub tick_before: Option<u32>,
    pub tick_after: Option<u32>,
    pub tick_stable: Option<bool>,
    pub derived_from_stable_sample: bool,
    pub position_raw_8_8: [i16; 3],
    pub terrain_pool_pointer_address: u32,
    pub terrain_pool_pointer: Option<u32>,
    pub current_terrain_index_address: u32,
    pub current_terrain_index: Option<u32>,
    pub terrain_pointer_slot_address: Option<u32>,
    pub terrain_pointer: Option<u32>,
    pub terrain_header_window: Option<VtolReadWindow>,
    pub sea_level_header_dword_at_0x00: Option<i32>,
    pub sea_level_shifted_i32: Option<i32>,
    pub wave_enable_address: u32,
    pub wave_enable: Option<u32>,
    pub surface_probe: VtolLiftSurfaceProbeEvidence,
    pub height_above_ride_surface_raw: Option<i32>,
    pub height_above_ride_surface_cells: Option<f32>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct CurrentTerrainResolution {
    pool_pointer: Option<u32>,
    index: Option<u32>,
    pointer_slot_address: Option<u32>,
    pointer: Option<u32>,
}

/// Read only the proven, radius-bounded Section-10/Section-9 static-object
/// candidates around `entity`.
pub fn read_nearby_static_collision_candidates(
    process: &Process,
    entity: &EntityRecord,
) -> NearbyStaticCollisionScanEvidence {
    let mut warnings = Vec::new();
    let terrain = read_current_terrain(process, &mut warnings);
    let static_object_scan = read_static_object_scan(
        process,
        terrain.pointer,
        Some(entity.position_raw_8_8),
        entity.active_model_collision_radius_raw_at_0x0a,
        &mut warnings,
    );

    NearbyStaticCollisionScanEvidence {
        entity_pointer: entity.pointer,
        terrain_pool_pointer_address: TERRAIN_POOL_POINTER as u32,
        terrain_pool_pointer: terrain.pool_pointer,
        current_terrain_index_address: CURRENT_TERRAIN_INDEX as u32,
        current_terrain_index: terrain.index,
        terrain_pointer_slot_address: terrain.pointer_slot_address,
        terrain_pointer: terrain.pointer,
        static_object_scan,
        warnings,
    }
}

/// Sample the exact retail terrain/wave height beneath one world position.
///
/// The derived surface and clearance are retained only when the 50-Hz tick is
/// stable across the complete read. No callback is invoked and no retail
/// memory is modified.
pub fn read_world_surface_probe(
    process: &Process,
    position_raw_8_8: [i16; 3],
) -> WorldSurfaceProbeEvidence {
    let mut warnings = Vec::new();
    let tick_before = read_u32_optional(
        process,
        TICK_50HZ as u32,
        "world-surface start tick",
        &mut warnings,
    );
    let terrain = read_current_terrain(process, &mut warnings);
    let (terrain_header_window, terrain_header_bytes) = terrain
        .pointer
        .map(|pointer| {
            read_pointer_window(
                process,
                pointer,
                TERRAIN_HEADER_BYTES,
                "world-surface terrain header",
                &mut warnings,
            )
        })
        .unwrap_or((None, None));
    let sea_level_header_dword_at_0x00 = terrain_header_bytes
        .as_deref()
        .and_then(|bytes| bounded_i32(bytes, 0));
    let sea_level_shifted_i32 = sea_level_header_dword_at_0x00.map(|value| value >> 8);
    let wave_enable = read_u32_optional(
        process,
        WAVE_ENABLE as u32,
        "wave enable for world-surface probe",
        &mut warnings,
    );
    let position_raw_xz = [position_raw_8_8[0], position_raw_8_8[2]];
    let mut surface_probe = terrain
        .pointer
        .map(|terrain_pointer| {
            SurfaceReadContext {
                process,
                terrain_pointer,
                sea_level_shifted_i32,
                wave_enable,
                tick: tick_before,
            }
            .read_probe([0, 0], position_raw_xz, &mut warnings)
        })
        .unwrap_or_else(|| {
            let x = position_raw_xz[0] as u16;
            let z = position_raw_xz[1] as u16;
            VtolLiftSurfaceProbeEvidence {
                offset_raw_xz: [0, 0],
                cell_xz: [(x >> 8) as u8, (z >> 8) as u8],
                fraction_xz: [x as u8, z as u8],
                ..VtolLiftSurfaceProbeEvidence::default()
            }
        });
    let tick_after = read_u32_optional(
        process,
        TICK_50HZ as u32,
        "world-surface end tick",
        &mut warnings,
    );
    let tick_stable = tick_before
        .zip(tick_after)
        .map(|(before, after)| before == after);
    let derived_from_stable_sample = tick_stable == Some(true);
    if !derived_from_stable_sample {
        surface_probe.terrain_surface_raw = None;
        surface_probe.wave_surface_raw = None;
        surface_probe.ride_surface_raw = None;
    }
    let height_above_ride_surface_raw = derived_from_stable_sample
        .then(|| {
            surface_probe
                .ride_surface_raw
                .map(|surface| i32::from(position_raw_8_8[1]) - surface)
        })
        .flatten();

    WorldSurfaceProbeEvidence {
        tick_address: TICK_50HZ as u32,
        tick_before,
        tick_after,
        tick_stable,
        derived_from_stable_sample,
        position_raw_8_8,
        terrain_pool_pointer_address: TERRAIN_POOL_POINTER as u32,
        terrain_pool_pointer: terrain.pool_pointer,
        current_terrain_index_address: CURRENT_TERRAIN_INDEX as u32,
        current_terrain_index: terrain.index,
        terrain_pointer_slot_address: terrain.pointer_slot_address,
        terrain_pointer: terrain.pointer,
        terrain_header_window,
        sea_level_header_dword_at_0x00,
        sea_level_shifted_i32,
        wave_enable_address: WAVE_ENABLE as u32,
        wave_enable,
        surface_probe,
        height_above_ride_surface_raw,
        height_above_ride_surface_cells: height_above_ride_surface_raw
            .map(|height| height as f32 / 256.0),
        warnings,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WaveSineRead {
    wrapped_phase: u32,
    table_address: u32,
    quadrant_negated: bool,
    table_value_q15: Option<u16>,
    signed_value_q15: Option<i32>,
    read_error: Option<String>,
}

/// Exact, bounded inputs and derived output for the VTOL ride-surface sample
/// in `FUN_00445310`.
///
/// All remote sources carry their fixed address or pointer-chain slot. The
/// derived heights are populated only when the narrow surface-specific 50-Hz
/// tick bracket is stable. Raw engine heights use the retail signed 8.8 world
/// convention: 256 raw units equal one terrain cell.
#[derive(Debug, Clone, Serialize, Default)]
pub struct VtolRideSurfaceEvidence {
    pub consumer_function: u32,
    pub terrain_sampler_function: u32,
    pub wave_sampler_function: u32,
    pub tick_address: u32,
    pub tick_before: Option<u32>,
    pub tick_after: Option<u32>,
    pub tick_stable: Option<bool>,
    pub derived_from_stable_sample: bool,
    /// The enclosing VTOL evidence bracket also covers the pre-inner active
    /// model radius, entity type flags, runtime Sub-G, and session-mode reads
    /// used by radius/type/runtime-coupled derivations.
    pub outer_vtol_tick_stable: Option<bool>,

    pub position_address_at_entity_0x96: u32,
    pub position_raw_8_8: Option<[i16; 3]>,
    pub velocity_raw_8_8: Option<[i16; 3]>,
    pub position_read_error: Option<String>,

    pub terrain_pool_pointer_address: u32,
    pub terrain_pool_pointer: Option<u32>,
    pub current_terrain_index_address: u32,
    pub current_terrain_index: Option<u32>,
    pub terrain_pointer_slot_address: Option<u32>,
    pub terrain_pointer: Option<u32>,
    pub terrain_header_window: Option<VtolReadWindow>,
    pub sea_level_header_dword_at_0x00: Option<i32>,
    pub sea_level_shifted_i32: Option<i32>,
    pub sea_level_raw: Option<i16>,
    pub wave_enable_address: u32,
    pub wave_enable: Option<u32>,

    /// Bounded Section-10/Section-9 context for distinguishing static
    /// tree/building contact from dynamic-entity contact.
    pub nearby_static_objects: VtolStaticObjectScanEvidence,

    pub terrain_surface_raw: Option<i32>,
    /// The direct signed-word result of `FUN_00445920`. When waves are
    /// disabled this is the flat sea word; the caller still takes the maximum
    /// with terrain.
    pub wave_surface_raw: Option<i16>,
    pub ride_surface_raw: Option<i32>,

    pub active_model_half_radius_raw: Option<u16>,
    pub craft_origin_y_raw: Option<i32>,
    pub craft_bottom_y_raw: Option<i32>,
    pub height_above_terrain_raw: Option<i32>,
    pub height_above_wave_raw: Option<i32>,
    pub height_above_ride_surface_raw: Option<i32>,
    pub height_above_ride_surface_cells: Option<f32>,
    /// Exact `FUN_00445310` PD gate: altitude error must be in
    /// `[-299, 999]` inclusive. `None` means the stable derivation was not
    /// available, not that the sample was outside the window.
    pub within_vtol_altitude_assist_window: Option<bool>,

    pub lift_projection_function: u32,
    pub lift_terrain_sampler_function: u32,
    pub type_flags_at_entity_0x64: Option<u32>,
    pub uses_player_five_probe_branch: Option<bool>,
    pub lift_surface_probes: Vec<VtolLiftSurfaceProbeEvidence>,
    pub lift_limit_minimum_surface_raw: Option<i32>,
    pub lift_limit_clearance_raw: Option<i32>,
    pub normal_lift_attenuation_threshold_raw: i32,
    pub turbo_lift_attenuation_threshold_raw: i32,
    pub selected_lift_attenuation_threshold_raw: Option<i32>,
    pub lift_force_branch_gate: Option<bool>,
    /// Signed interpretation returned by `FUN_00456F10` from the raw session
    /// byte at `+0x296`.
    pub game_mode_signed_at_session_0x296: Option<i8>,
    pub height_attenuation_bypassed_by_game_mode_4: Option<bool>,
    pub lift_attenuation_height_divisor: Option<i32>,
    pub lift_attenuation_velocity_divisor: Option<i32>,
    pub manual_lift_before_attenuation: Option<i32>,
    pub manual_lift_after_height_and_velocity_attenuation: Option<i32>,

    /// Common `FUN_0040E100` underwater response does not use the animated
    /// wave. It compares entity origin Y with max(integer-cell terrain, sea).
    pub common_static_terrain_cell_raw: Option<i32>,
    pub common_buoyancy_surface_raw: Option<i32>,
    pub common_depth_raw: Option<i32>,
    /// Only the strict depth predicate. The actual response additionally
    /// requires effective environment bit 2 to be clear and a nested
    /// type/environment byte to be nonzero; this focused probe captures
    /// neither complete gate.
    pub common_underwater_depth_threshold_crossed: Option<bool>,
}

pub(super) struct VtolRideSurfaceRequest<'a> {
    pub process: &'a Process,
    pub entity_pointer: u32,
    pub active_model_half_radius_raw: Option<u16>,
    pub active_model_collision_radius_raw: Option<u16>,
    pub type_flags_at_entity_0x64: Option<u32>,
    pub runtime: &'a VtolRuntimeValues,
    pub game_mode_signed_at_session_0x296: Option<i8>,
    pub warnings: &'a mut Vec<String>,
}

pub(super) fn read(request: VtolRideSurfaceRequest<'_>) -> VtolRideSurfaceEvidence {
    let VtolRideSurfaceRequest {
        process,
        entity_pointer,
        active_model_half_radius_raw,
        active_model_collision_radius_raw,
        type_flags_at_entity_0x64,
        runtime,
        game_mode_signed_at_session_0x296,
        warnings,
    } = request;
    let tick_before = read_u32_optional(
        process,
        TICK_50HZ as u32,
        "VTOL ride-surface start tick",
        warnings,
    );

    let position_address_at_entity_0x96 = entity_pointer.wrapping_add(0x96);
    let (position_raw_8_8, velocity_raw_8_8, position_read_error) = match process
        .read_bytes(position_address_at_entity_0x96 as usize, 12)
    {
        Ok(bytes) => (
            Some([i16_at(&bytes, 0), i16_at(&bytes, 2), i16_at(&bytes, 4)]),
            Some([i16_at(&bytes, 6), i16_at(&bytes, 8), i16_at(&bytes, 10)]),
            None,
        ),
        Err(error) => {
            warnings.push(format!(
                "VTOL ride-surface position at {position_address_at_entity_0x96:08X} is unreadable: {error}"
            ));
            (None, None, Some(error))
        }
    };

    let terrain = read_current_terrain(process, warnings);
    let terrain_pool_pointer = terrain.pool_pointer;
    let current_terrain_index = terrain.index;
    let terrain_pointer_slot_address = terrain.pointer_slot_address;
    let terrain_pointer = terrain.pointer;
    let (terrain_header_window, terrain_header_bytes) = terrain_pointer
        .map(|pointer| {
            read_pointer_window(
                process,
                pointer,
                TERRAIN_HEADER_BYTES,
                "current terrain header",
                warnings,
            )
        })
        .unwrap_or((None, None));
    let sea_level_header_dword_at_0x00 = terrain_header_bytes
        .as_deref()
        .and_then(|bytes| bounded_i32(bytes, 0));
    let sea_level_shifted_i32 = sea_level_header_dword_at_0x00.map(|value| value >> 8);
    let sea_level_raw = sea_level_shifted_i32.map(|value| value as i16);
    let wave_enable = read_u32_optional(
        process,
        WAVE_ENABLE as u32,
        "wave enable for VTOL ride surface",
        warnings,
    );
    let nearby_static_objects = read_static_object_scan(
        process,
        terrain_pointer,
        position_raw_8_8,
        active_model_collision_radius_raw,
        warnings,
    );

    let uses_player_five_probe_branch = type_flags_at_entity_0x64.map(|flags| flags & 1 != 0);
    let probe_offsets: &[[i16; 2]] = if uses_player_five_probe_branch == Some(true) {
        &[[0, 0], [-0x100, 0], [0, -0x100], [0x100, 0], [0, 0x100]]
    } else {
        &[[0, 0]]
    };
    let mut lift_surface_probes = Vec::new();
    if let (Some(position), Some(terrain)) = (position_raw_8_8, terrain_pointer) {
        let surface_source = SurfaceReadContext {
            process,
            terrain_pointer: terrain,
            sea_level_shifted_i32,
            wave_enable,
            tick: tick_before,
        };
        for &[offset_x, offset_z] in probe_offsets {
            lift_surface_probes.push(surface_source.read_probe(
                [offset_x, offset_z],
                [
                    position[0].wrapping_add(offset_x),
                    position[2].wrapping_add(offset_z),
                ],
                warnings,
            ));
        }
    }

    let tick_after = read_u32_optional(
        process,
        TICK_50HZ as u32,
        "VTOL ride-surface end tick",
        warnings,
    );
    let tick_stable = tick_before
        .zip(tick_after)
        .map(|(before, after)| before == after);
    let derived_from_stable_sample = tick_stable == Some(true);

    if !derived_from_stable_sample {
        for probe in &mut lift_surface_probes {
            probe.terrain_surface_raw = None;
            probe.wave_surface_raw = None;
            probe.ride_surface_raw = None;
        }
    }

    let center_probe = lift_surface_probes.first();

    let mut terrain_surface_raw = None;
    let mut wave_surface_raw = None;
    let mut ride_surface_raw = None;
    let mut craft_origin_y_raw = None;
    let mut craft_bottom_y_raw = None;
    let mut height_above_terrain_raw = None;
    let mut height_above_wave_raw = None;
    let mut height_above_ride_surface_raw = None;
    let mut height_above_ride_surface_cells = None;
    let mut within_vtol_altitude_assist_window = None;
    let mut lift_limit_minimum_surface_raw = None;
    let mut lift_limit_clearance_raw = None;
    let selected_lift_attenuation_threshold_raw = runtime
        .turbo_height_limit_flag_at_0x3d
        .map(|flag| if flag == 0 { 0x800 } else { 0xC00 });
    let lift_force_branch_gate = runtime
        .manual_lift_at_0x20
        .zip(
            runtime
                .turbo_height_limit_flag_at_0x3d
                .zip(runtime.phase_counter_at_0x34)
                .zip(runtime.byte_at_0x42),
        )
        .map(|(manual_lift, ((turbo, phase_counter), byte_42))| {
            manual_lift != 0 && (turbo == 0 || phase_counter < 200 || byte_42 != 0)
        });
    let height_attenuation_bypassed_by_game_mode_4 =
        game_mode_signed_at_session_0x296.map(|mode| mode == 4);
    let mut lift_attenuation_height_divisor = None;
    let mut lift_attenuation_velocity_divisor = None;
    let mut manual_lift_after_height_and_velocity_attenuation = None;
    let mut common_static_terrain_cell_raw = None;
    let mut common_buoyancy_surface_raw = None;
    let mut common_depth_raw = None;
    let mut common_underwater_depth_threshold_crossed = None;

    if derived_from_stable_sample {
        terrain_surface_raw = center_probe.and_then(|probe| probe.terrain_surface_raw);
        wave_surface_raw = center_probe.and_then(|probe| probe.wave_surface_raw);
        ride_surface_raw = center_probe.and_then(|probe| probe.ride_surface_raw);
        craft_origin_y_raw = position_raw_8_8.map(|position| i32::from(position[1]));
        craft_bottom_y_raw = craft_origin_y_raw
            .zip(active_model_half_radius_raw)
            .map(|(origin, radius)| origin - i32::from(radius));
        height_above_terrain_raw = craft_bottom_y_raw
            .zip(terrain_surface_raw)
            .map(|(bottom, terrain)| bottom - terrain);
        height_above_wave_raw = craft_bottom_y_raw
            .zip(wave_surface_raw)
            .map(|(bottom, wave)| bottom - i32::from(wave));
        height_above_ride_surface_raw = craft_bottom_y_raw
            .zip(ride_surface_raw)
            .map(|(bottom, surface)| bottom - surface);
        height_above_ride_surface_cells =
            height_above_ride_surface_raw.map(|height| height as f32 / 256.0);
        within_vtol_altitude_assist_window =
            height_above_ride_surface_raw.map(is_within_vtol_altitude_assist_window);

        if uses_player_five_probe_branch == Some(true) && lift_surface_probes.len() == 5 {
            lift_limit_minimum_surface_raw = minimum_complete_surface(
                lift_surface_probes
                    .iter()
                    .map(|probe| probe.ride_surface_raw),
                5,
            );
            lift_limit_clearance_raw = craft_bottom_y_raw
                .zip(lift_limit_minimum_surface_raw)
                .map(|(bottom, surface)| bottom - surface);
        }
        lift_attenuation_height_divisor = lift_limit_clearance_raw
            .zip(selected_lift_attenuation_threshold_raw)
            .zip(game_mode_signed_at_session_0x296)
            .map(|((clearance, threshold), game_mode)| {
                lift_height_divisor_for_game_mode(clearance, threshold, game_mode)
            });
        lift_attenuation_velocity_divisor =
            velocity_raw_8_8.map(|velocity| lift_velocity_divisor(i32::from(velocity[1])));
        manual_lift_after_height_and_velocity_attenuation = runtime
            .manual_lift_at_0x20
            .zip(lift_attenuation_height_divisor)
            .zip(lift_attenuation_velocity_divisor)
            .filter(|_| lift_force_branch_gate == Some(true))
            .map(|((lift, height_divisor), velocity_divisor)| {
                lift / height_divisor / velocity_divisor
            });

        common_static_terrain_cell_raw = center_probe
            .and_then(|probe| probe.terrain_corner_height_bytes[0])
            .map(signed_terrain_height_raw);
        common_buoyancy_surface_raw = common_static_terrain_cell_raw
            .zip(sea_level_raw)
            .map(|(terrain, sea)| common_buoyancy_surface(terrain, sea));
        common_depth_raw = craft_origin_y_raw
            .zip(common_buoyancy_surface_raw)
            .map(|(origin, surface)| origin - surface);
        common_underwater_depth_threshold_crossed = common_depth_raw.map(|depth| depth < -100);
    }

    VtolRideSurfaceEvidence {
        consumer_function: FUN_VTOL_UPDATE,
        terrain_sampler_function: FUN_TERRAIN_HEIGHT,
        wave_sampler_function: FUN_WAVE_HEIGHT,
        tick_address: TICK_50HZ as u32,
        tick_before,
        tick_after,
        tick_stable,
        derived_from_stable_sample,
        outer_vtol_tick_stable: None,
        position_address_at_entity_0x96,
        position_raw_8_8,
        velocity_raw_8_8,
        position_read_error,
        terrain_pool_pointer_address: TERRAIN_POOL_POINTER as u32,
        terrain_pool_pointer,
        current_terrain_index_address: CURRENT_TERRAIN_INDEX as u32,
        current_terrain_index,
        terrain_pointer_slot_address,
        terrain_pointer,
        terrain_header_window,
        sea_level_header_dword_at_0x00,
        sea_level_shifted_i32,
        sea_level_raw,
        wave_enable_address: WAVE_ENABLE as u32,
        wave_enable,
        nearby_static_objects,
        terrain_surface_raw,
        wave_surface_raw,
        ride_surface_raw,
        active_model_half_radius_raw,
        craft_origin_y_raw,
        craft_bottom_y_raw,
        height_above_terrain_raw,
        height_above_wave_raw,
        height_above_ride_surface_raw,
        height_above_ride_surface_cells,
        within_vtol_altitude_assist_window,
        lift_projection_function: FUN_LIFT_PROJECTION,
        lift_terrain_sampler_function: FUN_LIFT_TERRAIN_HEIGHT,
        type_flags_at_entity_0x64,
        uses_player_five_probe_branch,
        lift_surface_probes,
        lift_limit_minimum_surface_raw,
        lift_limit_clearance_raw,
        normal_lift_attenuation_threshold_raw: 0x800,
        turbo_lift_attenuation_threshold_raw: 0xC00,
        selected_lift_attenuation_threshold_raw,
        lift_force_branch_gate,
        game_mode_signed_at_session_0x296,
        height_attenuation_bypassed_by_game_mode_4,
        lift_attenuation_height_divisor,
        lift_attenuation_velocity_divisor,
        manual_lift_before_attenuation: runtime.manual_lift_at_0x20,
        manual_lift_after_height_and_velocity_attenuation,
        common_static_terrain_cell_raw,
        common_buoyancy_surface_raw,
        common_depth_raw,
        common_underwater_depth_threshold_crossed,
    }
}

pub(super) fn set_outer_tick_stability(
    evidence: &mut VtolRideSurfaceEvidence,
    stable: Option<bool>,
) {
    evidence.outer_vtol_tick_stable = stable;
    if stable != Some(true) {
        evidence.active_model_half_radius_raw = None;
        evidence.craft_bottom_y_raw = None;
        evidence.height_above_terrain_raw = None;
        evidence.height_above_wave_raw = None;
        evidence.height_above_ride_surface_raw = None;
        evidence.height_above_ride_surface_cells = None;
        evidence.within_vtol_altitude_assist_window = None;
        evidence.uses_player_five_probe_branch = None;
        evidence.lift_limit_minimum_surface_raw = None;
        evidence.lift_limit_clearance_raw = None;
        evidence.selected_lift_attenuation_threshold_raw = None;
        evidence.lift_force_branch_gate = None;
        evidence.game_mode_signed_at_session_0x296 = None;
        evidence.height_attenuation_bypassed_by_game_mode_4 = None;
        evidence.lift_attenuation_height_divisor = None;
        evidence.lift_attenuation_velocity_divisor = None;
        evidence.manual_lift_before_attenuation = None;
        evidence.manual_lift_after_height_and_velocity_attenuation = None;
    }
}

struct SurfaceReadContext<'a> {
    process: &'a Process,
    terrain_pointer: u32,
    sea_level_shifted_i32: Option<i32>,
    wave_enable: Option<u32>,
    tick: Option<u32>,
}

impl SurfaceReadContext<'_> {
    fn read_probe(
        &self,
        offset_raw_xz: [i16; 2],
        position_raw_xz: [i16; 2],
        warnings: &mut Vec<String>,
    ) -> VtolLiftSurfaceProbeEvidence {
        let x = position_raw_xz[0] as u16;
        let z = position_raw_xz[1] as u16;
        let x_cell = (x >> 8) as u8;
        let z_cell = (z >> 8) as u8;
        let x_fraction = x as u8;
        let z_fraction = z as u8;
        let x1 = x_cell.wrapping_add(1);
        let z1 = z_cell.wrapping_add(1);
        let terrain_corners = [
            read_terrain_corner(
                self.process,
                self.terrain_pointer,
                "x0_z0",
                x_cell,
                z_cell,
                warnings,
            ),
            read_terrain_corner(
                self.process,
                self.terrain_pointer,
                "x1_z0",
                x1,
                z_cell,
                warnings,
            ),
            read_terrain_corner(
                self.process,
                self.terrain_pointer,
                "x0_z1",
                x_cell,
                z1,
                warnings,
            ),
            read_terrain_corner(
                self.process,
                self.terrain_pointer,
                "x1_z1",
                x1,
                z1,
                warnings,
            ),
        ];
        let terrain_corner_height_bytes = std::array::from_fn(|index| {
            terrain_corners[index]
                .cell_bytes
                .map(|cell_bytes| cell_bytes[0])
        });
        let terrain_corner_cell_bytes =
            std::array::from_fn(|index| terrain_corners[index].cell_bytes);
        let terrain_surface_raw =
            decode_terrain_surface(&terrain_corner_height_bytes, x_fraction, z_fraction);

        let wave_reads = if let (Some(tick), Some(terrain), Some(sea), Some(enabled)) = (
            self.tick,
            terrain_surface_raw,
            self.sea_level_shifted_i32,
            self.wave_enable,
        ) {
            if enabled != 0 && sea > terrain {
                let phases = wave_phases(position_raw_xz[0], position_raw_xz[1], tick);
                Some([
                    read_wave_sine(self.process, "wave_1_x", phases[0], warnings),
                    read_wave_sine(self.process, "wave_2_x_plus_z", phases[1], warnings),
                    read_wave_sine(self.process, "wave_3_z_minus_x", phases[2], warnings),
                ])
            } else {
                None
            }
        } else {
            None
        };
        let sine_values = wave_reads.as_ref().and_then(|samples| {
            Some([
                samples[0].signed_value_q15?,
                samples[1].signed_value_q15?,
                samples[2].signed_value_q15?,
            ])
        });
        let wave_surface_raw = match (
            terrain_surface_raw,
            self.sea_level_shifted_i32,
            self.wave_enable,
        ) {
            (Some(terrain), Some(sea), Some(enabled)) if enabled == 0 || sea <= terrain => {
                Some(sea as i16)
            }
            (Some(terrain), Some(sea), Some(_)) => {
                sine_values.map(|values| wave_surface_from_sines(terrain, sea, values))
            }
            _ => None,
        };
        let ride_surface_raw = terrain_surface_raw
            .zip(wave_surface_raw)
            .map(|(terrain, wave)| terrain.max(i32::from(wave)));

        VtolLiftSurfaceProbeEvidence {
            offset_raw_xz,
            cell_xz: [x_cell, z_cell],
            fraction_xz: [x_fraction, z_fraction],
            terrain_corner_addresses: std::array::from_fn(|index| terrain_corners[index].address),
            terrain_corner_cell_bytes,
            terrain_corner_height_bytes,
            terrain_corner_read_errors: std::array::from_fn(|index| {
                terrain_corners[index].read_error.clone()
            }),
            terrain_surface_raw,
            wave_sample: wave_reads.as_ref().map(compact_wave_sample),
            wave_surface_raw,
            ride_surface_raw,
        }
    }
}

fn read_terrain_corner(
    process: &Process,
    terrain_pointer: u32,
    role: &'static str,
    cell_x: u8,
    cell_z: u8,
    warnings: &mut Vec<String>,
) -> TerrainCornerRead {
    let cell_index = u32::from(cell_x)
        .wrapping_mul(TERRAIN_GRID_SIZE)
        .wrapping_add(u32::from(cell_z));
    let address = terrain_pointer
        .wrapping_add(TERRAIN_HEADER_BYTES as u32)
        .wrapping_add(cell_index.wrapping_mul(TERRAIN_CELL_BYTES));
    match process.read_bytes(address as usize, TERRAIN_CELL_BYTES as usize) {
        Ok(bytes) => TerrainCornerRead {
            address,
            cell_bytes: Some([bytes[0], bytes[1], bytes[2]]),
            read_error: None,
        },
        Err(error) => {
            warnings.push(format!(
                "VTOL terrain corner {role} at {address:08X} is unreadable: {error}"
            ));
            TerrainCornerRead {
                address,
                cell_bytes: None,
                read_error: Some(error),
            }
        }
    }
}

fn read_static_object_scan(
    process: &Process,
    terrain_pointer: Option<u32>,
    position_raw_8_8: Option<[i16; 3]>,
    active_model_collision_radius_raw: Option<u16>,
    warnings: &mut Vec<String>,
) -> VtolStaticObjectScanEvidence {
    let static_object_pool_pointer = read_u32_optional(
        process,
        STATIC_OBJECT_POOL_POINTER as u32,
        "Section-9 static-object pool pointer",
        warnings,
    );
    let current_static_object_index = read_u32_optional(
        process,
        CURRENT_STATIC_OBJECT_INDEX as u32,
        "current Section-9 static-object index",
        warnings,
    );
    let static_object_table_pointer_slot_address = static_object_pool_pointer
        .filter(|pointer| plausible_heap_pointer(*pointer as usize))
        .zip(current_static_object_index.filter(|index| *index < MAX_TERRAIN_RESOURCES))
        .and_then(|(pool, index)| {
            index
                .checked_mul(4)
                .and_then(|offset| pool.checked_add(offset))
        });
    let static_object_table_pointer =
        static_object_table_pointer_slot_address.and_then(|address| {
            read_u32_optional(
                process,
                address,
                "current Section-9 static-object table",
                warnings,
            )
        });
    let model_pool_pointer = read_u32_optional(
        process,
        MODEL_POOL_POINTER as u32,
        "Section-8 model pool pointer for static objects",
        warnings,
    );
    let center_cell_xz = position_raw_8_8.map(|position| {
        [
            (position[0] as u16 >> 8) as u8,
            (position[2] as u16 >> 8) as u8,
        ]
    });
    let scan_geometry = position_raw_8_8
        .zip(active_model_collision_radius_raw)
        .map(|(position, radius)| static_scan_geometry(position, radius));
    let retail_scan_radius_raw = scan_geometry.map(|geometry| geometry.radius_raw);
    let scan_start_raw_xz = scan_geometry.map(|geometry| geometry.start_raw_xz);
    let scan_span_raw = scan_geometry.map(|geometry| geometry.span_raw);
    let candidate_axis_cells = scan_geometry.map(|geometry| geometry.axis_cells);
    let mut nonempty_cells = Vec::new();

    if let (Some(terrain), Some(start), Some(axis_cells)) =
        (terrain_pointer, scan_start_raw_xz, candidate_axis_cells)
    {
        if axis_cells > MAX_STATIC_OBJECT_SCAN_AXIS_CELLS {
            warnings.push(format!(
                "retail static-object scan axis length {axis_cells} exceeds passive-read safety limit {MAX_STATIC_OBJECT_SCAN_AXIS_CELLS}"
            ));
        } else {
            let start_cell_z = (start[1] as u16 >> 8) as u8;
            for x_index in 0..axis_cells {
                let raw_x = start[0].wrapping_add((x_index as i16).wrapping_mul(0x100));
                let cell_x = (raw_x as u16 >> 8) as u8;
                let strip = match read_terrain_cell_strip(
                    process,
                    terrain,
                    cell_x,
                    start_cell_z,
                    axis_cells,
                ) {
                    Ok(strip) => strip,
                    Err(error) => {
                        warnings.push(format!(
                            "static-object terrain strip X={cell_x} Z={start_cell_z} count={axis_cells} is unreadable: {error}"
                        ));
                        continue;
                    }
                };
                for (z_index, cell_bytes) in strip.into_iter().enumerate() {
                    let cell_z = start_cell_z.wrapping_add(z_index as u8);
                    let cell_index = u32::from(cell_x)
                        .wrapping_mul(TERRAIN_GRID_SIZE)
                        .wrapping_add(u32::from(cell_z));
                    let cell_address = terrain
                        .wrapping_add(TERRAIN_HEADER_BYTES as u32)
                        .wrapping_add(cell_index.wrapping_mul(TERRAIN_CELL_BYTES));
                    let attribute = cell_bytes[1];
                    if attribute == 0 {
                        continue;
                    }
                    let terrain_state = cell_bytes[2];
                    let selected_model_slot = static_model_slot(terrain_state);
                    let descriptor_address = static_object_table_pointer.map(|table| {
                        table.wrapping_add(
                            u32::from(attribute) * STATIC_OBJECT_DESCRIPTOR_BYTES as u32,
                        )
                    });
                    let (descriptor_model_ids, object_kind_index, descriptor_error) =
                        descriptor_address.map_or((None, None, None), |address| {
                            match process
                                .read_bytes(address as usize, STATIC_OBJECT_DESCRIPTOR_BYTES)
                            {
                                Ok(bytes) => (
                                    Some([
                                        u16::from_le_bytes([bytes[0], bytes[1]]),
                                        u16::from_le_bytes([bytes[2], bytes[3]]),
                                        u16::from_le_bytes([bytes[4], bytes[5]]),
                                        u16::from_le_bytes([bytes[6], bytes[7]]),
                                    ]),
                                    Some(u32::from_le_bytes([
                                        bytes[8], bytes[9], bytes[10], bytes[11],
                                    ])),
                                    None,
                                ),
                                Err(error) => (None, None, Some(error)),
                            }
                        });
                    let selected_model_id =
                        descriptor_model_ids.map(|models| models[selected_model_slot]);
                    let selected_model_resource_pointer = model_pool_pointer
                        .filter(|pointer| plausible_heap_pointer(*pointer as usize))
                        .zip(selected_model_id)
                        .and_then(|(pool, model_id)| {
                            process
                                .read_u32(pool as usize + usize::from(model_id) * 4)
                                .ok()
                        });
                    let selected_model_collision_radius_raw_at_0x0a =
                        selected_model_resource_pointer
                            .filter(|pointer| plausible_heap_pointer(*pointer as usize))
                            .and_then(|pointer| {
                                process
                                    .read_bytes(pointer as usize, MODEL_HEADER_BYTES)
                                    .ok()
                            })
                            .map(|bytes| u16::from_le_bytes([bytes[0x0A], bytes[0x0B]]));
                    nonempty_cells.push(VtolStaticObjectCellEvidence {
                        cell_xz: [cell_x, cell_z],
                        cell_address,
                        height_byte: cell_bytes[0],
                        attribute,
                        terrain_state,
                        descriptor_address,
                        descriptor_model_ids,
                        object_kind_index,
                        selected_model_slot,
                        selected_model_id,
                        selected_model_resource_pointer,
                        selected_model_collision_radius_raw_at_0x0a,
                        read_error: descriptor_error,
                    });
                }
            }
        }
    }

    VtolStaticObjectScanEvidence {
        static_object_pool_pointer_address: STATIC_OBJECT_POOL_POINTER as u32,
        static_object_pool_pointer,
        current_static_object_index_address: CURRENT_STATIC_OBJECT_INDEX as u32,
        current_static_object_index,
        static_object_table_pointer_slot_address,
        static_object_table_pointer,
        model_pool_pointer_address: MODEL_POOL_POINTER as u32,
        model_pool_pointer,
        center_cell_xz,
        active_model_collision_radius_raw,
        retail_scan_radius_raw,
        scan_start_raw_xz,
        scan_span_raw,
        candidate_axis_cells,
        nonempty_cells,
    }
}

fn read_current_terrain(process: &Process, warnings: &mut Vec<String>) -> CurrentTerrainResolution {
    let pool_pointer = read_u32_optional(
        process,
        TERRAIN_POOL_POINTER as u32,
        "Section-10 terrain pool pointer",
        warnings,
    );
    let index = read_u32_optional(
        process,
        CURRENT_TERRAIN_INDEX as u32,
        "current terrain index",
        warnings,
    );
    let pointer_slot_address = pool_pointer
        .filter(|pointer| plausible_heap_pointer(*pointer as usize))
        .zip(index.filter(|index| *index < MAX_TERRAIN_RESOURCES))
        .and_then(|(pool, index)| {
            index
                .checked_mul(4)
                .and_then(|offset| pool.checked_add(offset))
        });
    if let Some(index) = index.filter(|index| *index >= MAX_TERRAIN_RESOURCES) {
        warnings.push(format!(
            "current terrain index {index} exceeds passive-read safety limit {MAX_TERRAIN_RESOURCES}"
        ));
    }
    let pointer = pointer_slot_address.and_then(|address| {
        read_u32_optional(process, address, "current terrain pointer", warnings)
    });

    CurrentTerrainResolution {
        pool_pointer,
        index,
        pointer_slot_address,
        pointer,
    }
}

fn read_terrain_cell_strip(
    process: &Process,
    terrain_pointer: u32,
    cell_x: u8,
    start_cell_z: u8,
    count: usize,
) -> Result<Vec<[u8; 3]>, String> {
    let first_count = count.min(0x100 - usize::from(start_cell_z));
    let second_count = count - first_count;
    let mut cells = Vec::with_capacity(count);
    for (cell_z, segment_count) in [(start_cell_z, first_count), (0, second_count)] {
        if segment_count == 0 {
            continue;
        }
        let cell_index = u32::from(cell_x)
            .wrapping_mul(TERRAIN_GRID_SIZE)
            .wrapping_add(u32::from(cell_z));
        let address = terrain_pointer
            .wrapping_add(TERRAIN_HEADER_BYTES as u32)
            .wrapping_add(cell_index.wrapping_mul(TERRAIN_CELL_BYTES));
        let bytes = process.read_bytes(address as usize, segment_count * 3)?;
        cells.extend(
            bytes
                .chunks_exact(3)
                .map(|cell| [cell[0], cell[1], cell[2]]),
        );
    }
    Ok(cells)
}

fn static_model_slot(terrain_state: u8) -> usize {
    usize::from((terrain_state >> 3) & 3)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StaticScanGeometry {
    radius_raw: i16,
    start_raw_xz: [i16; 2],
    span_raw: u16,
    axis_cells: usize,
}

fn static_scan_geometry(
    position_raw_8_8: [i16; 3],
    active_model_collision_radius_raw: u16,
) -> StaticScanGeometry {
    let radius_raw = (active_model_collision_radius_raw as i16).wrapping_add(0x200);
    let start_raw_xz = [
        position_raw_8_8[0].wrapping_sub(radius_raw),
        position_raw_8_8[2].wrapping_sub(radius_raw),
    ];
    let span_raw = (((i32::from(radius_raw) * 2 + 0x1FF) as u32) & 0xFF00) as u16;
    StaticScanGeometry {
        radius_raw,
        start_raw_xz,
        span_raw,
        axis_cells: usize::from(span_raw >> 8),
    }
}

fn decode_terrain_surface(
    height_bytes: &[Option<u8>; 4],
    x_fraction: u8,
    z_fraction: u8,
) -> Option<i32> {
    let x0_z0 = signed_terrain_height_raw(height_bytes[0]?);
    let x1_z0 = signed_terrain_height_raw(height_bytes[1]?);
    let x0_z1 = signed_terrain_height_raw(height_bytes[2]?);
    let x1_z1 = signed_terrain_height_raw(height_bytes[3]?);
    let interpolate_x =
        |left: i32, right: i32| (((right - left) * i32::from(x_fraction)) >> 8) + left;
    let z0 = interpolate_x(x0_z0, x1_z0);
    let z1 = interpolate_x(x0_z1, x1_z1);
    Some((((z1 - z0) * i32::from(z_fraction)) >> 8) + z0)
}

fn signed_terrain_height_raw(height_byte: u8) -> i32 {
    i32::from(height_byte as i8) << 5
}

fn wave_phases(x_raw: i16, z_raw: i16, tick: u32) -> [u32; 3] {
    let x = u32::from(x_raw as u16);
    let z = u32::from(z_raw as u16);
    [
        tick.wrapping_mul(0x40)
            .wrapping_add(x.wrapping_mul(0x0C))
            .wrapping_mul(2),
        x.wrapping_add(z)
            .wrapping_mul(0x20)
            .wrapping_add(tick.wrapping_mul(0x140)),
        tick.wrapping_mul(0x40)
            .wrapping_add(z.wrapping_sub(x).wrapping_mul(6))
            .wrapping_mul(8),
    ]
}

fn fold_sine_phase(phase: u32) -> (u16, u16, bool) {
    let mut folded_byte_offset = (phase & 0x3FFC) as u16;
    if phase & 0x4000 != 0 {
        folded_byte_offset = 0x3FFC - folded_byte_offset;
    }
    (
        folded_byte_offset,
        folded_byte_offset >> 2,
        phase & 0x8000 != 0,
    )
}

fn read_wave_sine(
    process: &Process,
    term: &'static str,
    wrapped_phase: u32,
    warnings: &mut Vec<String>,
) -> WaveSineRead {
    let (_, table_index, quadrant_negated) = fold_sine_phase(wrapped_phase);
    let table_address = (RETAIL_SINE_TABLE as u32).wrapping_add(u32::from(table_index) * 2);
    match process.read_u16(table_address as usize) {
        Ok(value) => WaveSineRead {
            wrapped_phase,
            table_address,
            quadrant_negated,
            table_value_q15: Some(value),
            signed_value_q15: Some(if quadrant_negated {
                -i32::from(value)
            } else {
                i32::from(value)
            }),
            read_error: None,
        },
        Err(error) => {
            warnings.push(format!(
                "VTOL wave sine term {term} at {table_address:08X} is unreadable: {error}"
            ));
            WaveSineRead {
                wrapped_phase,
                table_address,
                quadrant_negated,
                table_value_q15: None,
                signed_value_q15: None,
                read_error: Some(error),
            }
        }
    }
}

fn compact_wave_sample(samples: &[WaveSineRead; 3]) -> VtolWaveProbeEvidence {
    VtolWaveProbeEvidence {
        wrapped_phases: std::array::from_fn(|index| samples[index].wrapped_phase),
        table_addresses: std::array::from_fn(|index| samples[index].table_address),
        quadrant_negated_mask: samples.iter().enumerate().fold(0, |mask, (index, sample)| {
            mask | (u8::from(sample.quadrant_negated) << index)
        }),
        table_values_q15: std::array::from_fn(|index| samples[index].table_value_q15),
        read_errors: std::array::from_fn(|index| samples[index].read_error.clone()),
    }
}

fn wave_surface_from_sines(terrain_raw: i32, sea_level_shifted_i32: i32, waves: [i32; 3]) -> i16 {
    let coarse = (waves[2] >> 6) as i16;
    let combined = (waves[1].wrapping_add(waves[0]) >> 5) as i16;
    let wave_sum = coarse.wrapping_add(combined);
    let depth = sea_level_shifted_i32
        .wrapping_sub(terrain_raw)
        .wrapping_add(0x200);
    let displacement = (i32::from(wave_sum).wrapping_mul(depth) >> 15) as i16;
    let candidate = sea_level_shifted_i32.wrapping_add(i32::from(displacement));
    if candidate < terrain_raw {
        terrain_raw as i16
    } else {
        candidate as i16
    }
}

fn is_within_vtol_altitude_assist_window(clearance_raw: i32) -> bool {
    (-299..=999).contains(&clearance_raw)
}

fn lift_height_divisor(clearance_raw: i32, threshold_raw: i32) -> i32 {
    if clearance_raw > threshold_raw {
        ((clearance_raw - threshold_raw) / 128).max(1)
    } else {
        1
    }
}

fn lift_height_divisor_for_game_mode(
    clearance_raw: i32,
    threshold_raw: i32,
    game_mode_signed_at_session_0x296: i8,
) -> i32 {
    if game_mode_signed_at_session_0x296 == 4 {
        1
    } else {
        lift_height_divisor(clearance_raw, threshold_raw)
    }
}

fn lift_velocity_divisor(vertical_velocity_raw: i32) -> i32 {
    (vertical_velocity_raw / 512).max(1)
}

fn common_buoyancy_surface(terrain_raw: i32, sea_level_raw: i16) -> i32 {
    terrain_raw.max(i32::from(sea_level_raw))
}

fn minimum_complete_surface<I>(surfaces: I, expected_len: usize) -> Option<i32>
where
    I: IntoIterator<Item = Option<i32>>,
{
    let mut count = 0;
    let mut minimum = None;
    for surface in surfaces {
        let surface = surface?;
        minimum = Some(minimum.map_or(surface, |value: i32| value.min(surface)));
        count += 1;
    }
    (count == expected_len).then_some(minimum?)
}

fn all_none<T, const N: usize>(values: &[Option<T>; N]) -> bool {
    values.iter().all(Option::is_none)
}

fn bounded_i32(bytes: &[u8], offset: usize) -> Option<i32> {
    let bytes = bytes.get(offset..offset.checked_add(4)?)?;
    Some(i32::from_le_bytes(bytes.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::{
        common_buoyancy_surface, decode_terrain_surface, fold_sine_phase,
        is_within_vtol_altitude_assist_window, lift_height_divisor,
        lift_height_divisor_for_game_mode, lift_velocity_divisor, minimum_complete_surface,
        set_outer_tick_stability, static_model_slot, static_scan_geometry, wave_phases,
        wave_surface_from_sines, VtolLiftSurfaceProbeEvidence, VtolRideSurfaceEvidence,
        VtolWaveProbeEvidence,
    };

    #[test]
    fn terrain_surface_replays_retail_signed_bilinear_shift_order() {
        let corners = [Some(0), Some(1), Some(2), Some(3)];
        assert_eq!(decode_terrain_surface(&corners, 128, 64), Some(32));

        let signed = [Some((-2_i8) as u8), Some(1), Some((-1_i8) as u8), Some(2)];
        assert_eq!(decode_terrain_surface(&signed, 85, 170), Some(-12));

        assert_eq!(
            decode_terrain_surface(&[Some(0), Some(1), Some(2), None], 128, 64),
            None
        );
    }

    #[test]
    fn static_object_scan_replays_retail_radius_rounding_and_model_slot() {
        let geometry = static_scan_geometry([0x1234, 0, -0x7000], 248);
        assert_eq!(geometry.radius_raw, 760);
        assert_eq!(geometry.start_raw_xz, [0x0F3C, -0x72F8]);
        assert_eq!(geometry.span_raw, 0x700);
        assert_eq!(geometry.axis_cells, 7);

        assert_eq!(static_model_slot(0b0000_0000), 0);
        assert_eq!(static_model_slot(0b0000_1000), 1);
        assert_eq!(static_model_slot(0b0001_0000), 2);
        assert_eq!(static_model_slot(0b0001_1000), 3);
        assert_eq!(static_model_slot(0b1111_1111), 3);
    }

    #[test]
    fn wave_sine_lookup_fold_preserves_quadrants_and_table_bounds() {
        assert_eq!(fold_sine_phase(0), (0, 0, false));
        assert_eq!(fold_sine_phase(0x3FFC), (0x3FFC, 0x0FFF, false));
        assert_eq!(fold_sine_phase(0x4000), (0x3FFC, 0x0FFF, false));
        assert_eq!(fold_sine_phase(0x8000), (0, 0, true));
        assert_eq!(fold_sine_phase(0xC000), (0x3FFC, 0x0FFF, true));
    }

    #[test]
    fn wave_phase_math_wraps_the_signed_position_words_like_retail() {
        assert_eq!(wave_phases(0, 0, 1), [0x80, 0x140, 0x200]);
        assert_eq!(
            wave_phases(-1, 1, 1),
            [0x0018_0068, 0x0020_0140, 0xFFD0_0260]
        );
    }

    #[test]
    fn wave_surface_uses_retail_narrowing_and_terrain_clamp() {
        assert_eq!(wave_surface_from_sines(-1024, 4096, [0, 0, 0]), 4096);
        assert_eq!(
            wave_surface_from_sines(-1024, 4096, [32767, 32767, 32767]),
            4535
        );
        assert_eq!(
            wave_surface_from_sines(4000, 4001, [-32767, -32767, -32767]),
            4000
        );
        // Retail compares the full i32 candidate before narrowing the return
        // word. Comparing the narrowed -30099 here would incorrectly clamp
        // this result back to terrain.
        assert_eq!(
            wave_surface_from_sines(-1024, 32760, [32767, 32767, 32767]),
            -30099
        );
        assert_eq!(
            wave_surface_from_sines(-4096, 0x007F_FFFF, [32767, 32767, 32767]),
            -154
        );
    }

    #[test]
    fn common_buoyancy_surface_sign_extends_the_narrowed_sea_word() {
        assert_eq!(common_buoyancy_surface(-1024, 4096), 4096);
        // FUN_0040E100 narrows (header >> 8) to i16 before comparing it with
        // terrain; it must not reuse FUN_00445920's full i32 wave input.
        assert_eq!(common_buoyancy_surface(-1024, 32768_u16 as i16), -1024);
    }

    #[test]
    fn vtol_clearance_windows_and_attenuation_boundaries_are_strict() {
        assert!(!is_within_vtol_altitude_assist_window(-300));
        assert!(is_within_vtol_altitude_assist_window(-299));
        assert!(is_within_vtol_altitude_assist_window(999));
        assert!(!is_within_vtol_altitude_assist_window(1000));

        assert_eq!(lift_height_divisor(0x800, 0x800), 1);
        assert_eq!(lift_height_divisor(0x880, 0x800), 1);
        assert_eq!(lift_height_divisor(0x900, 0x800), 2);
        assert_eq!(lift_velocity_divisor(-1024), 1);
        assert_eq!(lift_velocity_divisor(511), 1);
        assert_eq!(lift_velocity_divisor(1024), 2);
    }

    #[test]
    fn game_mode_four_bypasses_height_but_not_upward_velocity_attenuation() {
        let normal_height_divisor = lift_height_divisor_for_game_mode(0x1000, 0x800, 0);
        let mode_four_height_divisor = lift_height_divisor_for_game_mode(0x1000, 0x800, 4);
        let upward_velocity_divisor = lift_velocity_divisor(1024);

        assert_eq!(normal_height_divisor, 16);
        assert_eq!(mode_four_height_divisor, 1);
        assert_eq!(upward_velocity_divisor, 2);
        assert_eq!(320 / normal_height_divisor / upward_velocity_divisor, 10);
        assert_eq!(
            320 / mode_four_height_divisor / upward_velocity_divisor,
            160
        );
    }

    #[test]
    fn five_probe_minimum_requires_every_expected_surface() {
        assert_eq!(
            minimum_complete_surface([Some(320), Some(96), Some(-64), Some(512), Some(0)], 5),
            Some(-64)
        );
        assert_eq!(
            minimum_complete_surface([Some(320), Some(96), None, Some(512), Some(0)], 5),
            None
        );
        assert_eq!(
            minimum_complete_surface([Some(320), Some(96), Some(-64), Some(512)], 5),
            None
        );
    }

    #[test]
    fn probe_json_uses_compact_ordered_raw_arrays_without_center_duplicates() {
        let probe = VtolLiftSurfaceProbeEvidence {
            offset_raw_xz: [0, 0],
            cell_xz: [12, 34],
            fraction_xz: [56, 78],
            terrain_corner_addresses: [0x1000, 0x1300, 0x1003, 0x1303],
            terrain_corner_cell_bytes: [
                Some([1, 0, 0x21]),
                Some([2, 3, 0x22]),
                Some([3, 0, 0x23]),
                Some([4, 0, 0x24]),
            ],
            terrain_corner_height_bytes: [Some(1), Some(2), Some(3), Some(4)],
            terrain_corner_read_errors: [None, None, None, None],
            terrain_surface_raw: Some(64),
            wave_sample: Some(VtolWaveProbeEvidence {
                wrapped_phases: [1, 2, 3],
                table_addresses: [0x004D_14D0, 0x004D_14D2, 0x004D_14D4],
                quadrant_negated_mask: 0b010,
                table_values_q15: [Some(10), Some(20), Some(30)],
                read_errors: [None, None, None],
            }),
            wave_surface_raw: Some(96),
            ride_surface_raw: Some(96),
        };
        let value = serde_json::to_value(&probe).expect("probe serializes");
        let object = value.as_object().expect("probe is an object");

        for removed in [
            "label",
            "position_raw_xz",
            "terrain_corners",
            "wave_sine_samples",
            "terrain_corner_read_errors",
        ] {
            assert!(!object.contains_key(removed), "unexpected key {removed}");
        }
        assert!(object.contains_key("terrain_corner_addresses"));
        assert!(object.contains_key("terrain_corner_cell_bytes"));
        assert!(object.contains_key("terrain_corner_height_bytes"));
        let wave = object["wave_sample"]
            .as_object()
            .expect("wave sample is an object");
        assert!(!wave.contains_key("term"));
        assert!(!wave.contains_key("table_index"));
        assert!(!wave.contains_key("signed_value_q15"));
        assert!(!wave.contains_key("read_errors"));
        assert!(
            serde_json::to_string(&probe)
                .expect("probe serializes")
                .len()
                < 1_000
        );

        let mut errored = probe;
        errored.terrain_corner_read_errors[2] = Some("unreadable".to_owned());
        let errored = serde_json::to_value(errored).expect("errored probe serializes");
        assert!(errored["terrain_corner_read_errors"].is_array());
    }

    #[test]
    fn unstable_outer_tick_clears_only_outer_coupled_derivations() {
        let mut evidence = VtolRideSurfaceEvidence {
            position_raw_8_8: Some([1, 2, 3]),
            terrain_surface_raw: Some(32),
            ride_surface_raw: Some(64),
            active_model_half_radius_raw: Some(16),
            craft_origin_y_raw: Some(256),
            craft_bottom_y_raw: Some(240),
            height_above_terrain_raw: Some(208),
            height_above_wave_raw: Some(176),
            height_above_ride_surface_raw: Some(176),
            height_above_ride_surface_cells: Some(0.6875),
            within_vtol_altitude_assist_window: Some(true),
            uses_player_five_probe_branch: Some(true),
            lift_limit_minimum_surface_raw: Some(32),
            lift_limit_clearance_raw: Some(208),
            selected_lift_attenuation_threshold_raw: Some(0x800),
            lift_force_branch_gate: Some(true),
            game_mode_signed_at_session_0x296: Some(0),
            height_attenuation_bypassed_by_game_mode_4: Some(false),
            lift_attenuation_height_divisor: Some(1),
            lift_attenuation_velocity_divisor: Some(1),
            manual_lift_before_attenuation: Some(128),
            manual_lift_after_height_and_velocity_attenuation: Some(128),
            common_depth_raw: Some(-101),
            common_underwater_depth_threshold_crossed: Some(true),
            ..VtolRideSurfaceEvidence::default()
        };

        set_outer_tick_stability(&mut evidence, Some(false));

        assert_eq!(evidence.outer_vtol_tick_stable, Some(false));
        assert_eq!(evidence.position_raw_8_8, Some([1, 2, 3]));
        assert_eq!(evidence.terrain_surface_raw, Some(32));
        assert_eq!(evidence.ride_surface_raw, Some(64));
        assert_eq!(evidence.craft_origin_y_raw, Some(256));
        assert_eq!(evidence.common_depth_raw, Some(-101));
        assert_eq!(
            evidence.common_underwater_depth_threshold_crossed,
            Some(true)
        );
        assert_eq!(evidence.active_model_half_radius_raw, None);
        assert_eq!(evidence.craft_bottom_y_raw, None);
        assert_eq!(evidence.height_above_terrain_raw, None);
        assert_eq!(evidence.height_above_wave_raw, None);
        assert_eq!(evidence.height_above_ride_surface_raw, None);
        assert_eq!(evidence.height_above_ride_surface_cells, None);
        assert_eq!(evidence.within_vtol_altitude_assist_window, None);
        assert_eq!(evidence.uses_player_five_probe_branch, None);
        assert_eq!(evidence.lift_limit_minimum_surface_raw, None);
        assert_eq!(evidence.lift_limit_clearance_raw, None);
        assert_eq!(evidence.selected_lift_attenuation_threshold_raw, None);
        assert_eq!(evidence.lift_force_branch_gate, None);
        assert_eq!(evidence.game_mode_signed_at_session_0x296, None);
        assert_eq!(evidence.height_attenuation_bypassed_by_game_mode_4, None);
        assert_eq!(evidence.lift_attenuation_height_divisor, None);
        assert_eq!(evidence.lift_attenuation_velocity_divisor, None);
        assert_eq!(evidence.manual_lift_before_attenuation, None);
        assert_eq!(
            evidence.manual_lift_after_height_and_velocity_attenuation,
            None
        );
    }
}
