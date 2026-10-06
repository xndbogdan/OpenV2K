//! Shared `15120` objective predicate and the live `1BEB0` Hive prefix.
//!
//! Each Hive owns Sub-N state and signed timer +50. Neither callback selects
//! a world, model ID, hostile family, or initial cohort.

use v2k_formats::terrain::TerrainGrid;

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    component_update::ComponentUpdateMode,
    entity::{AuthoredHiveInfectionBlock, Entity},
    entity_behavior::audited_behavior_style,
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
    },
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    hive_death::HiveRadialTaskState,
    world_complete_results::WorldCompleteTally,
};

pub const HIVE_ENTITY_TYPE: u32 = 67;
pub const OBJECTIVE_CAPABILITY_BIT: u32 = 0x08;
pub const OBJECTIVE_STATE_BIT: u32 = 0x0100_0000;
pub const HIVE_LOCKED_HEALTH_RAW: i32 = 1_000_000_000;
pub const HIVE_CLEAR_GRACE_US: i32 = 2_000_000;

/// Inputs for the post-player `1BEB0` wreck walk. The session abort byte
/// (`456CB0`) selects direct retry; ordinary wreck suction also runs outside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveWreckPlayerContactFrame {
    pub elapsed_us: u32,
    pub session_aborted: bool,
}

/// `41C7DC..41C806`: use the original signed, wrapping raw deltas, after
/// the marker/live-player/dead-Hive/positive-clock gates admitted the contact.
/// The outer `25370` envelope still applies to the inner retry branch.
pub fn hive_wreck_retry_requested(
    origin_raw: [i16; 3],
    player_position_raw: [i16; 3],
    session_aborted: bool,
) -> bool {
    if !session_aborted
        || crate::radial_damage::radial_distance_raw(origin_raw, player_position_raw) >= 8_000
    {
        return false;
    }
    let dx = i32::from(player_position_raw[0].wrapping_sub(origin_raw[0]));
    let dy = i32::from(player_position_raw[1].wrapping_sub(origin_raw[1]));
    let dz = i32::from(player_position_raw[2].wrapping_sub(origin_raw[2]));
    let xz_sq = dx.wrapping_mul(dx).wrapping_add(dz.wrapping_mul(dz));
    xz_sq < 0x1_0000 && dy < -0x80
}

/// World inputs shared by the ordered component pass. Actor identity, authored
/// health and Sub-N storage are re-resolved from the manager for every visit.
pub struct AuthoredHiveComponentFrame<'a> {
    pub elapsed_us: u32,
    pub terrain: Option<&'a TerrainGrid>,
    pub retail_tick: i32,
    pub notification_phase: GameplayNotificationPhase,
    pub notifications: &'a mut GameplayNotifications,
    pub world_complete_tally: &'a mut WorldCompleteTally,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectivePredicateBlock {
    UnresolvedObjectiveState { entity_id: u32 },
}

/// `15120`: live-list order, capability8, non-dying and state01000000.
pub fn objective_hostile_present<'a>(
    entities: impl IntoIterator<Item = &'a Entity>,
) -> Result<bool, ObjectivePredicateBlock> {
    for entity in entities {
        if !entity.active || entity.capability_flags & OBJECTIVE_CAPABILITY_BIT == 0 {
            continue;
        }
        let state_mask = ACTIVE_MODEL_SLOT_LOW_STATE_BIT | OBJECTIVE_STATE_BIT;
        let state = match entity.collision.state_flags_at_0x08.masked(state_mask) {
            RetailRuntimeValue::Known(state) => state,
            RetailRuntimeValue::Unresolved => {
                return Err(ObjectivePredicateBlock::UnresolvedObjectiveState {
                    entity_id: entity.id,
                });
            }
        };
        if state & ACTIVE_MODEL_SLOT_LOW_STATE_BIT == 0 && state & OBJECTIVE_STATE_BIT != 0 {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Live positive-health branch. The guarded1CE10 latch and its distinct
/// two-visit component-death effect host are separate from this health policy.
pub(crate) struct HiveLiveHealth<'a> {
    pub health_raw: &'a mut i32,
    pub authored_health_raw: i32,
    pub grace_timer_us: &'a mut i32,
}

pub(crate) enum HiveComponentHealth<'a> {
    Live(HiveLiveHealth<'a>),
    Dying,
}

/// Authenticate the actual live slot0 and loader Sub-N before adding the
/// formerly omitted health prefix. No model-number or hostile-cohort test.
pub(crate) fn live_component_authenticates(
    entity: &Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> bool {
    let style = entity
        .current_behavior_context
        .map(|context| context.and_then(|context| context.active_style().audited()));
    let attachment = metadata
        .and_then(|metadata| metadata.sub_n_payload)
        .map(|raw| {
            std::array::from_fn(|axis| i16::from_le_bytes([raw[4 + axis * 2], raw[5 + axis * 2]]))
        });
    entity.entity_type == HIVE_ENTITY_TYPE
        && style == RetailRuntimeValue::Known(audited_behavior_style(46, 0).copied())
        && entity.actor_task_state(ActorTaskSlot::Primary)
            == Some(&ActorTaskRuntime::HiveRadial(HiveRadialTaskState::live()))
        && matches!((entity.sub_n_runtime, attachment), (RetailRuntimeValue::Known(Some(runtime)), Some(attachment))
            if runtime.descriptor_attachment_at_birth() == RetailRuntimeValue::Known(attachment))
}

/// Each production visit authenticates its own live component before changing
/// health. A missing owner cannot discard an earlier Hive's terrain writes.
pub(crate) fn live_health_inputs(
    entity: &Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<(i32, i32), AuthoredHiveInfectionBlock> {
    let entity_id = entity.id;
    if !live_component_authenticates(entity, metadata) {
        return Err(AuthoredHiveInfectionBlock::UnauthenticatedHiveComponent { entity_id });
    }
    let RetailRuntimeValue::Known(health) = entity.collision.health_raw else {
        return Err(AuthoredHiveInfectionBlock::UnresolvedHiveHealth { entity_id });
    };
    if health <= 0 {
        return Err(AuthoredHiveInfectionBlock::HiveDeathContinuationRequired { entity_id });
    }
    let authored = metadata
        .and_then(|metadata| metadata.initial_health_raw)
        .ok_or(AuthoredHiveInfectionBlock::MissingAuthoredHiveHealth { entity_id })?;
    Ok((health, authored))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HiveHealthTransition {
    Locked,
    Unlocked,
}

/// `41BF9B..41C040`: signed comparisons, wrapping ADD, then state/health.
/// The threshold check is independent of the hostile predicate after ADD;
/// retail therefore also permits a single sufficiently large elapsed visit.
pub(crate) fn advance_live_health(
    controller_state: &mut u32,
    live: HiveLiveHealth<'_>,
    objective_hostile_present: bool,
    elapsed_us: u32,
) -> Option<HiveHealthTransition> {
    if !objective_hostile_present || *live.grace_timer_us <= 0 {
        if *live.grace_timer_us < HIVE_CLEAR_GRACE_US {
            *live.grace_timer_us = live.grace_timer_us.wrapping_add(elapsed_us as i32);
        }
    } else {
        *live.grace_timer_us = 0;
    }
    if *live.grace_timer_us >= HIVE_CLEAR_GRACE_US || *controller_state != 1 {
        if *controller_state == 1 || *live.health_raw > live.authored_health_raw {
            *controller_state = 2;
            *live.health_raw = live.authored_health_raw;
            return Some(HiveHealthTransition::Unlocked);
        }
    } else if *live.health_raw < HIVE_LOCKED_HEALTH_RAW {
        *live.health_raw = HIVE_LOCKED_HEALTH_RAW;
        return Some(HiveHealthTransition::Locked);
    }
    None
}

pub(crate) fn notify_health_transition(
    transition: HiveHealthTransition,
    mode: ComponentUpdateMode,
    frame: &mut AuthoredHiveComponentFrame<'_>,
) {
    if frame.notification_phase == GameplayNotificationPhase::Playing {
        match transition {
            HiveHealthTransition::Locked if mode == ComponentUpdateMode::Detailed => {
                frame
                    .notifications
                    .queue_hive_locked_hint(frame.retail_tick);
            }
            HiveHealthTransition::Unlocked => {
                frame
                    .notifications
                    .queue_hive_now_vulnerable(frame.retail_tick);
            }
            HiveHealthTransition::Locked => {}
        }
    }
    // 456790 is outside the text helpers' controller-phase gates. It arms
    // the timestamp sentinel without opening results or changing game mode.
    if transition == HiveHealthTransition::Unlocked && frame.world_complete_tally.ticks_0x2bc == 0 {
        frame.world_complete_tally.ticks_0x2bc = -1;
    }
}

#[cfg(test)]
mod tests;
