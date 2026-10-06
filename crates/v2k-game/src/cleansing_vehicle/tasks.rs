//! Class42 AD50/ADB0 and403040, plus the shared class68 C490 timer.
use super::*;
use crate::{
    actor_task_dispatcher::{
        prepare_shared_generic_constructor_suffix, ActorTaskRuntime, SharedGenericConstructorEffect,
    },
    actor_task_owner::{ActorTaskSlot, PreparedActorTask},
    class0_timer::Class0TimerTaskState,
    cleansing_landscape_owner::{
        apply_cleansing_landscape_setup, CleansingLandscapeSetupRequest, CleansingLandscapeTaskRole,
    },
    defecate_virus::DefecateVirusTerrainTaskState,
    entity_behavior::{
        audited_behavior_style, initial_behavior_state_policy, select_initial_behavior,
        translate_state_policy, BehaviorContextRuntime, BehaviorDescriptorIdentity,
        BehaviorSelection, BehaviorWeightRule,
    },
    guard_location_owner::acquisition::{
        select_guard_location_candidate, GuardLocationCandidateFilter,
        GuardLocationCandidateRequest, GuardLocationCandidateSelection, GuardLocationEntityRef,
        GuardLocationSearchContext,
    },
    wander_near_location::WanderNearPrivateState,
    wrapped_axis_range::WrappedAxisRange,
};
use std::num::NonZeroU32;
use v2k_formats::terrain::TerrainGrid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleansingMovementTaskState {
    pub elapsed_ms: u32,
    pub private: WanderNearPrivateState,
}
impl CleansingMovementTaskState {
    pub const fn new(position: [i16; 3]) -> Self {
        Self {
            elapsed_ms: 0,
            private: WanderNearPrivateState::ordinary_type9(position),
        }
    }
    pub fn advance_prefix(&mut self, elapsed_micros: u32) {
        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1000);
    }
    /// 40308D..4031E7. The unsigned low-word divisions deliberately differ
    /// from Wander's bit shift once the expanding radius ceases to divide32768.
    pub fn retarget(
        &mut self,
        anchor: [i16; 3],
        terrain: &TerrainGrid,
        next_random: &mut impl FnMut() -> u32,
    ) {
        let infected = |position: [i16; 3]| {
            terrain
                .cell(
                    usize::from(position[0] as u16 >> 8),
                    usize::from(position[2] as u16 >> 8),
                )
                .expect("complete terrain")
                .terrain_type
                & 0x10
                != 0
        };
        if infected(self.private.target_position_raw) && next_random() as u16 & 63 != 0 {
            return;
        }
        let mut radius = 256i32;
        for _ in 0..8 {
            let divisor = 32768 / radius;
            let x = i32::from(next_random() as u16) / divisor;
            self.private.target_position_raw[0] =
                anchor[0].wrapping_add(x as i16).wrapping_sub(radius as i16);
            self.private.target_position_raw[1] = anchor[1];
            let z = i32::from(next_random() as u16) / divisor;
            self.private.target_position_raw[2] =
                anchor[2].wrapping_add(z as i16).wrapping_sub(radius as i16);
            if infected(self.private.target_position_raw) {
                break;
            }
            radius = radius * 3 / 2;
        }
    }
}

pub(crate) fn candidate(entity: &Entity) -> GuardLocationEntityRef {
    GuardLocationEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
    }
}

pub(crate) fn select_behavior(
    entity: &Entity,
    candidates: &[Entity],
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> Result<BehaviorSelection, CleansingVehicleError> {
    authenticate_metadata(metadata)?;
    let candidates: Vec<_> = candidates
        .iter()
        .filter(|e| e.active)
        .map(candidate)
        .collect();
    select_with_candidates(entity, &candidates, next_random)
}

fn select_with_candidates(
    entity: &Entity,
    candidates: &[GuardLocationEntityRef],
    next_random: &mut impl FnMut() -> u32,
) -> Result<BehaviorSelection, CleansingVehicleError> {
    let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
        return Err(CleansingVehicleError::Runtime("cleansing common axis"));
    };
    let base = base_nearby(candidate(entity), candidates, axis)?;
    select_with_base(base, next_random)
}

pub(crate) fn base_nearby(
    owner: GuardLocationEntityRef,
    candidates: &[GuardLocationEntityRef],
    axis: CommonAxisDescriptor,
) -> Result<bool, CleansingVehicleError> {
    select_guard_location_candidate(GuardLocationCandidateRequest {
        owner,
        candidates_in_intrusive_order: candidates,
        search_context: GuardLocationSearchContext::new(
            WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
            GuardLocationCandidateFilter::CapabilityMask(NonZeroU32::new(0x20).unwrap()),
        ),
    })
    .map(|base| matches!(base, GuardLocationCandidateSelection::Selected(_)))
    .map_err(|_| CleansingVehicleError::NearbyEvidence)
}

pub(crate) fn select_with_base(
    base: bool,
    next_random: &mut impl FnMut() -> u32,
) -> Result<BehaviorSelection, CleansingVehicleError> {
    select_initial_behavior(
        &CHOICES,
        |rule| match rule {
            BehaviorWeightRule::BaseNearby => i32::from(base),
            BehaviorWeightRule::Always => 1,
            _ => unreachable!("authenticated cleansing choices"),
        },
        next_random,
    )
    .map_err(|_| CleansingVehicleError::Metadata)?
    .ok_or(CleansingVehicleError::Graph)
}

pub(crate) fn publish_selection(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    selection: BehaviorSelection,
    context: BehaviorContextRuntime,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), CleansingVehicleError> {
    if !matches!(selection.program.class_id, 42 | 68)
        || context.descriptor() != BehaviorDescriptorIdentity::Named(selection.program)
        || context.style_table_index_raw_at_0x10() != 0
    {
        return Err(CleansingVehicleError::Graph);
    }
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    let position = entity.position_raw();
    let Entity {
        actor_tasks,
        sub_a_propulsion_runtime,
        ..
    } = entity;
    let RetailRuntimeValue::Known(Some(a)) = sub_a_propulsion_runtime else {
        return Err(CleansingVehicleError::Runtime("cleansing Sub-A allocation"));
    };
    let mut prepare = |runtime| {
        let prepared =
            prepare_shared_generic_constructor_suffix(PreparedActorTask::new(runtime), metadata)
                .map_err(|_| CleansingVehicleError::Graph)?;
        Ok(
            prepared.apply_suffix(&mut *next_random, |effect| match effect {
                SharedGenericConstructorEffect::WriteSubADirection {
                    direction_multiplier,
                } => a.set_direction_multiplier(direction_multiplier),
                SharedGenericConstructorEffect::WriteSubATargetSpeed {
                    target_speed_raw, ..
                } => a.apply_shared_initializer_target_speed_write(target_speed_raw),
                SharedGenericConstructorEffect::WriteSubHState08 { .. } => {
                    unreachable!("Type49 has no H")
                }
            }),
        )
    };
    if selection.program.class_id == 68 {
        actor_tasks.clear_slot(ActorTaskSlot::Secondary);
        actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
        let prepared = prepare(ActorTaskRuntime::Class0Timer(Class0TimerTaskState::new()))?;
        actor_tasks.replace_prepared(ActorTaskSlot::Primary, prepared);
    } else {
        apply_cleansing_landscape_setup(
            actor_tasks,
            CleansingLandscapeSetupRequest {
                terrain_task_lifetime_ms: 0,
            },
            |request| {
                prepare(match request.task.role {
                    CleansingLandscapeTaskRole::TerrainCleansing => {
                        ActorTaskRuntime::TerrainCleansing(DefecateVirusTerrainTaskState::new(0))
                    }
                    CleansingLandscapeTaskRole::TerrainSeekingMovement => {
                        ActorTaskRuntime::CleansingLandscape(CleansingMovementTaskState::new(
                            position,
                        ))
                    }
                })
            },
        )
        .map_err(|error| error.error)?;
    }
    Ok(())
}

pub(crate) fn publish_carrying(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), CleansingVehicleError> {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(CleansingVehicleError::Graph);
    };
    let style = *audited_behavior_style(42, 1).unwrap();
    // CD70's capability1000 arm, before the variant initializer policy.
    entity.collision.state_flags_at_0x08.overwrite(0x8000, 0);
    let context = context
        .reselect_named_type_default(
            crate::entity_behavior::behavior_program(42).unwrap(),
            1,
            style,
        )
        .ok_or(CleansingVehicleError::Graph)?;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let enabled = translate_state_policy(0x80);
    let disabled = translate_state_policy(2);
    let set = enabled.set_bits | disabled.clear_bits;
    let clear = enabled.clear_bits | disabled.set_bits;
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(set | clear, set);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
    let prepared = prepare_shared_generic_constructor_suffix(
        PreparedActorTask::new(ActorTaskRuntime::None),
        metadata,
    )
    .map_err(|_| CleansingVehicleError::Graph)?;
    let RetailRuntimeValue::Known(Some(a)) = &mut entity.sub_a_propulsion_runtime else {
        return Err(CleansingVehicleError::Graph);
    };
    let prepared = prepared.apply_suffix(next_random, |effect| match effect {
        SharedGenericConstructorEffect::WriteSubADirection {
            direction_multiplier,
        } => a.set_direction_multiplier(direction_multiplier),
        SharedGenericConstructorEffect::WriteSubATargetSpeed {
            target_speed_raw, ..
        } => a.apply_shared_initializer_target_speed_write(target_speed_raw),
        SharedGenericConstructorEffect::WriteSubHState08 { .. } => unreachable!(),
    });
    entity
        .actor_tasks
        .replace_prepared(ActorTaskSlot::Primary, prepared);
    Ok(())
}

pub(crate) fn reselect(
    manager: &mut EntityManager,
    id: u32,
    task_result: bool,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), CleansingVehicleError> {
    let entity = manager
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(CleansingVehicleError::Allocation)?;
    if !allocation_authenticates(manager, id) {
        return Err(CleansingVehicleError::Allocation);
    }
    if task_result {
        match entity.collision.state_flags_at_0x08.masked(0x1000) {
            RetailRuntimeValue::Known(0) => {}
            RetailRuntimeValue::Known(_) => return Ok(()),
            RetailRuntimeValue::Unresolved => {
                return Err(CleansingVehicleError::Runtime("task-result suppression"))
            }
        }
    }
    let RetailRuntimeValue::Known(flags) = entity.collision.state_flags_at_0x08.masked(0x4000)
    else {
        return Err(CleansingVehicleError::Graph);
    };
    if flags != 0 {
        return Err(CleansingVehicleError::UnsupportedDeath);
    }
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(CleansingVehicleError::Graph);
    };
    let metadata = manager
        .type_runtime_metadata(49)
        .cloned()
        .ok_or(CleansingVehicleError::Metadata)?;
    authenticate_metadata(&metadata)?;
    let candidates: Vec<_> = manager
        .retail_live_order_ids()
        .filter_map(|id| manager.iter_all().find(|e| e.id == id && e.active))
        .map(candidate)
        .collect();
    let selection = select_with_candidates(entity, &candidates, next_random)?;
    let context = previous
        .reselect_named_type_default(selection.program, 0, selection.program.initial_style)
        .ok_or(CleansingVehicleError::Graph)?;
    publish_selection(
        manager.entity_mut(id).unwrap(),
        &metadata,
        selection,
        context,
        next_random,
    )
}
