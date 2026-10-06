//! The component-local `1BEB0` death branch, before ordinary `10C10`.
//!
//! Infection precedes this decision. A latched Sub-N `+44` bypasses positive
//! health without a damage packet. `425EA0` ignores this callback's return;
//! the visit keeps its live model/tasks. Only the following state-zero visit
//! requests ordinary death, whose `25F60` burst has its own current-position
//! and dying-model extent. The host must retain that separate ordering.

use super::{
    hive_death_radial_template, hive_death_surface_burst_program, HiveDeathSurfaceBurstProgram,
};
use crate::radial_damage::RadialDamageTemplate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveComponentDeathRequest {
    pub entity_id: u32,
    pub controller_state: u32,
    pub forced_death_requested: bool,
    pub health_raw: i32,
    pub objective_hostile_present: bool,
    /// The source scatter template byte is the entity state sign bit.
    pub source_remote_owned: bool,
    pub death_sound_id: usize,
    pub entity_position_raw: [i16; 3],
    /// Retained Sub-N `+54/+56/+58`, not the current entity position.
    pub anchor_position_raw: [i16; 3],
    /// Currently selected model header `+08`, before any dying-model switch.
    pub current_model_extent_raw: u16,
    pub marker_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HiveComponentDeathPlan {
    /// `4568B0(3) -> 1D050(hostile) -> 4567B0` if the helper returns zero.
    /// A forced death while objective actors remain skips all three calls.
    pub completion_prefix: bool,
    pub objective_hostile_present: bool,
    pub source_remote_owned: bool,
    pub controller_state_after: u32,
    pub health_after_raw: i32,
    pub death_sound_id: usize,
    pub sound_position_raw: [i16; 3],
    pub effects_position_raw: [i16; 3],
    pub surface_burst: HiveDeathSurfaceBurstProgram,
    pub radial: RadialDamageTemplate,
    /// The host writes this after scatter and radial, only for a marker.
    pub suction_timer_after_us: Option<i32>,
    /// State zero suppresses class5 spit before the row-birth phase.
    pub radial_accumulator_after_us: u32,
}

/// Ordered plan: optional completion prefix, state/health writes, fixed cue,
/// anchor scatter, anchor radial, optional suction timer, radial reset.
/// This intentionally contains no generic damage/model/task transition.
pub fn plan_hive_component_death(
    request: HiveComponentDeathRequest,
) -> Option<HiveComponentDeathPlan> {
    if request.controller_state == 0 || (!request.forced_death_requested && request.health_raw > 0)
    {
        return None;
    }
    Some(HiveComponentDeathPlan {
        completion_prefix: !request.forced_death_requested || !request.objective_hostile_present,
        objective_hostile_present: request.objective_hostile_present,
        source_remote_owned: request.source_remote_owned,
        controller_state_after: 0,
        health_after_raw: 0,
        death_sound_id: request.death_sound_id,
        sound_position_raw: request.entity_position_raw,
        effects_position_raw: request.anchor_position_raw,
        surface_burst: hive_death_surface_burst_program(request.current_model_extent_raw),
        radial: hive_death_radial_template(request.entity_id),
        suction_timer_after_us: request.marker_present.then_some(-6_000_000),
        radial_accumulator_after_us: 0,
    })
}

/// `1BEB0`'s state-zero branch calls `10C10` before advancing the marker clock.
/// It requires a resolved entity, nonzero state flags, and a clear dying bit.
pub const fn hive_state_zero_requests_standard_death(
    controller_state: u32,
    entity_state_flags: Option<u32>,
) -> bool {
    match entity_state_flags {
        Some(flags) => controller_state == 0 && flags != 0 && flags & 0x4000 == 0,
        None => false,
    }
}

#[cfg(test)]
mod tests;
