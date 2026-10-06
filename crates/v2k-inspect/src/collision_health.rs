//! Compact, read-only collision and health evidence for the retail player.
//!
//! This deliberately stays narrower than [`crate::entity::EntityRuntimeEvidence`].
//! A high-rate collision capture needs the already decoded entity state and
//! the proven session/controller HUD chain. It does not need type callbacks,
//! wind, behavior windows, component slots, or unrelated display latches.

use serde::Serialize;

use crate::controller::{locate_controller, ControllerLocation, SESSION_POINTER_GLOBAL};
use crate::entity::{EntityRecord, TICK_50HZ};
use crate::process::{plausible_heap_pointer, Process};

/// Hard-coded divisor used by the retail hull-health HUD in `FUN_004292B0`.
pub const TYPE_46_HULL_HEALTH_BAR_MAXIMUM: i32 = 40_000;
/// `DAT_004DB1C8`, the visible trailing value eased toward raw hull health.
pub const SMOOTHED_VISIBLE_HULL_HEALTH_GLOBAL: usize = 0x004D_B1C8;

const SESSION_PHASE_OFFSET: usize = 0x296;
const CONTROLLER_HULL_HEALTH_MIRROR_OFFSET: usize = 0x8C;
const Q16_ONE: i64 = 1 << 16;

/// One compact sample suitable for a high-rate collision timeline.
#[derive(Debug, Clone, Serialize)]
pub struct CollisionHealthEvidence {
    pub player: PlayerCollisionState,
    pub session_hud: SessionHudCollisionEvidence,
}

/// Fields copied from a tick-bracketed [`EntityRecord`] snapshot. Offset names
/// preserve the retail provenance and avoid claiming more gameplay semantics
/// than the decompiled collision paths prove.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlayerCollisionState {
    pub intrusive_list_index: usize,
    pub entity_pointer: u32,
    pub next_entity_pointer: u32,
    pub entity_type: u32,
    pub handle: u32,
    pub body_basis_raw: [i32; 9],
    pub position_raw_8_8: [i16; 3],
    pub rotation_raw_u16: [u16; 3],
    pub velocity_raw_8_8: [i16; 3],
    pub flags_raw_at_0x08: u32,
    pub collision_flags: CollisionFlagState,
    /// Authoritative raw value consumed by the retail hull-health bar.
    pub hull_health_raw_at_entity_0x30: i32,
    pub last_damage_tick_at_0x34: u32,
    /// Damage is drained here before it reaches `hull_health_raw_at_entity_0x30`.
    /// This buffer is not drawn by the hull-health bar.
    pub pre_health_damage_buffer_at_entity_0x50: i32,
    pub recent_relation_handle_at_entity_0x60: u32,
    pub recent_relation_elapsed_us_at_entity_0x68: u32,
    pub subject_scan_gate_at_entity_0x70: u32,
    pub hull_health_bar_normalized: HullHealthBarNormalization,
    pub active_model_slot: usize,
    pub active_model_id: u16,
    pub active_model_resource_pointer: u32,
    pub active_model_radius_raw_at_0x08: Option<u16>,
    pub active_model_collision_radius_raw_at_0x0a: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CollisionFlagState {
    pub pair_collision_enabled_state_0x00008000: bool,
    pub pair_collision_ineligible_state_0x00001000: bool,
    pub active_model_slot_low_state_0x00004000: bool,
    pub active_model_slot_high_state_0x00002000: bool,
    pub pair_collision_fixed_state_0x08000000: bool,
    pub pair_collision_cross_domain_state_0x80000000: bool,
    pub terrain_water_collision_enabled_0x00010000: bool,
    pub deferred_destroy_0x00100000: bool,
    pub fully_below_water_0x00200000: bool,
    pub fully_above_water_0x00400000: bool,
    pub contact_response_0x00800000: bool,
}

/// Inspector-derived representations of the retail bar's proven raw/max pair.
/// The raw source and maximum are exact; these integer ratios are convenient
/// capture annotations rather than additional retail memory fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct HullHealthBarNormalization {
    pub observed_value: i32,
    pub retail_bar_maximum: i32,
    pub signed_ratio_q16: i64,
    /// 10,000 means 100.00%; retained as an integer to avoid float drift.
    pub signed_percent_x100: i64,
}

/// Session/HUD reads bracketed by the retail 50 Hz tick. All fields are
/// optional so process teardown and the gameplay-to-menu handoff produce a
/// final partial sample instead of failing the capture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionHudCollisionEvidence {
    pub tick_address: u32,
    pub tick_before: Option<u32>,
    pub tick_after: Option<u32>,
    pub tick_stable: Option<bool>,
    pub session_pointer_global_address: u32,
    pub session_pointer: Option<u32>,
    pub phase_byte_address_at_session_0x296: Option<u32>,
    pub phase_byte_at_session_0x296: Option<u8>,
    pub phase_signed_at_session_0x296: Option<i8>,
    pub controller_location: Option<ControllerLocation>,
    /// Exact hull-health mirror written from entity `+0x30` by `FUN_00443440`.
    pub hull_health_mirror_address_at_controller_0x8c: Option<u32>,
    pub hull_health_mirror_at_controller_0x8c: Option<i32>,
    pub hull_health_mirror_normalized: Option<HullHealthBarNormalization>,
    pub smoothed_visible_hull_health_global_address: u32,
    pub smoothed_visible_hull_health_at_0x004db1c8: Option<i32>,
    pub smoothed_visible_hull_health_normalized: Option<HullHealthBarNormalization>,
    pub warnings: Vec<String>,
}

pub fn read(process: &Process, player: &EntityRecord) -> CollisionHealthEvidence {
    let player_state = PlayerCollisionState::from(player);
    CollisionHealthEvidence {
        player: player_state,
        session_hud: read_session_hud(process, Some(player.handle)),
    }
}

impl From<&EntityRecord> for PlayerCollisionState {
    fn from(player: &EntityRecord) -> Self {
        Self {
            intrusive_list_index: player.intrusive_list_index,
            entity_pointer: player.pointer,
            next_entity_pointer: player.next,
            entity_type: player.entity_type,
            handle: player.handle,
            body_basis_raw: player.body_basis_raw,
            position_raw_8_8: player.position_raw_8_8,
            rotation_raw_u16: player.rotation,
            velocity_raw_8_8: player.velocity_raw_8_8,
            flags_raw_at_0x08: player.flags,
            collision_flags: collision_flags(player.flags),
            hull_health_raw_at_entity_0x30: player.collision_health_at_0x30,
            last_damage_tick_at_0x34: player.last_damage_tick_at_0x34,
            pre_health_damage_buffer_at_entity_0x50: player.damage_buffer_at_0x50,
            recent_relation_handle_at_entity_0x60: player.recent_relation_handle_at_0x60,
            recent_relation_elapsed_us_at_entity_0x68: player.recent_relation_elapsed_us_at_0x68,
            subject_scan_gate_at_entity_0x70: player.subject_scan_gate_at_0x70,
            hull_health_bar_normalized: hull_health_bar_normalization(
                player.collision_health_at_0x30,
            ),
            active_model_slot: player.active_slot,
            active_model_id: player.active_model,
            active_model_resource_pointer: player.active_model_resource,
            active_model_radius_raw_at_0x08: player.active_model_radius_raw_at_0x08,
            active_model_collision_radius_raw_at_0x0a: player
                .active_model_collision_radius_raw_at_0x0a,
        }
    }
}

pub fn read_session_hud(
    process: &Process,
    expected_entity_handle: Option<u32>,
) -> SessionHudCollisionEvidence {
    let mut warnings = Vec::new();
    let tick_before = read_u32_optional(process, TICK_50HZ, "tick before", &mut warnings);
    let session_pointer = read_u32_optional(
        process,
        SESSION_POINTER_GLOBAL,
        "session pointer global",
        &mut warnings,
    )
    .filter(|pointer| {
        if *pointer == 0 {
            return false;
        }
        if plausible_heap_pointer(*pointer as usize) {
            true
        } else {
            warnings.push(format!(
                "session pointer {pointer:08X} is outside the bounded aligned read range"
            ));
            false
        }
    });

    let phase_byte_address =
        session_pointer.map(|pointer| pointer.wrapping_add(SESSION_PHASE_OFFSET as u32));
    let phase_byte = phase_byte_address.and_then(|address| {
        read_u8_optional(
            process,
            address as usize,
            "session phase byte +0x296",
            &mut warnings,
        )
    });

    let controller_location = locate_controller(process, expected_entity_handle, &mut warnings);
    let controller_health_address = controller_location
        .filter(|location| {
            expected_entity_handle.is_none()
                || location.entity_handle_matches_expected == Some(true)
        })
        .map(|location| {
            location
                .controller_pointer
                .wrapping_add(CONTROLLER_HULL_HEALTH_MIRROR_OFFSET as u32)
        });
    let controller_health = controller_health_address.and_then(|address| {
        read_i32_optional(
            process,
            address as usize,
            "controller hull-health mirror +0x8C",
            &mut warnings,
        )
    });
    let smoothed_visible_hull_health = read_i32_optional(
        process,
        SMOOTHED_VISIBLE_HULL_HEALTH_GLOBAL,
        "smoothed visible hull-health global",
        &mut warnings,
    );

    let tick_after = read_u32_optional(process, TICK_50HZ, "tick after", &mut warnings);

    SessionHudCollisionEvidence {
        tick_address: TICK_50HZ as u32,
        tick_before,
        tick_after,
        tick_stable: tick_before
            .zip(tick_after)
            .map(|(before, after)| before == after),
        session_pointer_global_address: SESSION_POINTER_GLOBAL as u32,
        session_pointer,
        phase_byte_address_at_session_0x296: phase_byte_address,
        phase_byte_at_session_0x296: phase_byte,
        phase_signed_at_session_0x296: phase_byte.map(|byte| byte as i8),
        controller_location,
        hull_health_mirror_address_at_controller_0x8c: controller_health_address,
        hull_health_mirror_at_controller_0x8c: controller_health,
        hull_health_mirror_normalized: controller_health.map(hull_health_bar_normalization),
        smoothed_visible_hull_health_global_address: SMOOTHED_VISIBLE_HULL_HEALTH_GLOBAL as u32,
        smoothed_visible_hull_health_at_0x004db1c8: smoothed_visible_hull_health,
        smoothed_visible_hull_health_normalized: smoothed_visible_hull_health
            .map(hull_health_bar_normalization),
        warnings,
    }
}

fn collision_flags(flags: u32) -> CollisionFlagState {
    CollisionFlagState {
        pair_collision_enabled_state_0x00008000: flags & 0x0000_8000 != 0,
        pair_collision_ineligible_state_0x00001000: flags & 0x0000_1000 != 0,
        active_model_slot_low_state_0x00004000: flags & 0x0000_4000 != 0,
        active_model_slot_high_state_0x00002000: flags & 0x0000_2000 != 0,
        pair_collision_fixed_state_0x08000000: flags & 0x0800_0000 != 0,
        pair_collision_cross_domain_state_0x80000000: flags & 0x8000_0000 != 0,
        terrain_water_collision_enabled_0x00010000: flags & 0x0001_0000 != 0,
        deferred_destroy_0x00100000: flags & 0x0010_0000 != 0,
        fully_below_water_0x00200000: flags & 0x0020_0000 != 0,
        fully_above_water_0x00400000: flags & 0x0040_0000 != 0,
        contact_response_0x00800000: flags & 0x0080_0000 != 0,
    }
}

fn hull_health_bar_normalization(observed_value: i32) -> HullHealthBarNormalization {
    let reference = TYPE_46_HULL_HEALTH_BAR_MAXIMUM;
    HullHealthBarNormalization {
        observed_value,
        retail_bar_maximum: reference,
        signed_ratio_q16: i64::from(observed_value) * Q16_ONE / i64::from(reference),
        signed_percent_x100: i64::from(observed_value) * 10_000 / i64::from(reference),
    }
}

fn read_u32_optional(
    process: &Process,
    address: usize,
    label: &str,
    warnings: &mut Vec<String>,
) -> Option<u32> {
    match process.read_u32(address) {
        Ok(value) => Some(value),
        Err(error) => {
            warnings.push(format!("{label} at {address:08X} is unreadable: {error}"));
            None
        }
    }
}

fn read_u8_optional(
    process: &Process,
    address: usize,
    label: &str,
    warnings: &mut Vec<String>,
) -> Option<u8> {
    match process.read_bytes(address, 1) {
        Ok(bytes) => bytes.first().copied(),
        Err(error) => {
            warnings.push(format!("{label} at {address:08X} is unreadable: {error}"));
            None
        }
    }
}

fn read_i32_optional(
    process: &Process,
    address: usize,
    label: &str,
    warnings: &mut Vec<String>,
) -> Option<i32> {
    read_u32_optional(process, address, label, warnings).map(|value| value as i32)
}

#[cfg(test)]
mod tests {
    use super::{collision_flags, hull_health_bar_normalization};

    #[test]
    fn comparison_q16_is_signed_unclamped_and_explicitly_referenced() {
        let full = hull_health_bar_normalization(40_000);
        assert_eq!(full.signed_ratio_q16, 0x1_0000);
        assert_eq!(full.signed_percent_x100, 10_000);
        assert_eq!(
            hull_health_bar_normalization(20_000).signed_ratio_q16,
            0x8000
        );
        assert_eq!(
            hull_health_bar_normalization(20_000).signed_percent_x100,
            5_000
        );
        assert_eq!(
            hull_health_bar_normalization(-10_000).signed_ratio_q16,
            -0x4000
        );
    }

    #[test]
    fn collision_bits_are_independently_decoded() {
        let decoded = collision_flags(
            0x0000_8000
                | 0x0000_1000
                | 0x0000_4000
                | 0x0800_0000
                | 0x8000_0000
                | 0x0001_0000
                | 0x0010_0000
                | 0x0020_0000
                | 0x0080_0000,
        );
        assert!(decoded.pair_collision_enabled_state_0x00008000);
        assert!(decoded.pair_collision_ineligible_state_0x00001000);
        assert!(decoded.active_model_slot_low_state_0x00004000);
        assert!(!decoded.active_model_slot_high_state_0x00002000);
        assert!(decoded.pair_collision_fixed_state_0x08000000);
        assert!(decoded.pair_collision_cross_domain_state_0x80000000);
        assert!(decoded.terrain_water_collision_enabled_0x00010000);
        assert!(decoded.deferred_destroy_0x00100000);
        assert!(decoded.fully_below_water_0x00200000);
        assert!(!decoded.fully_above_water_0x00400000);
        assert!(decoded.contact_response_0x00800000);
    }
}
