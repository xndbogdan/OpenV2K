//! Native Intro2 peasant birth, with independent Sub-D allocator evidence.
//!
//! The shared retail Type9 row is identical to Level1. Its authored spawn
//! parameter, prefix candidates, process seeds, and uninitialized classifier
//! origins are separate. Construction consumes 20450 before 425680 and then
//! reuses the exact selected task transactions. Each authored birth has its
//! own accepted Intro2 first-query receipt; no Level1 first-scheduler
//! overwrite or allocator origin is transferred here.

use crate::{
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit},
    common_mover::{
        sub_d::intro2_type9_first_query_owner_for_birth, type9_attitude::Type9BodyBasis,
        SubAPropulsionRuntime,
    },
    entity::Entity,
    entity_behavior::{
        initial_behavior_state_policy, select_initial_behavior, BehaviorContextRuntime,
        BehaviorSelection, BehaviorWeightRule,
    },
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, BODY_BASIS_REBUILT_STATE_BIT,
    },
    go_to_job::GoToJobOwner,
    guard_location_owner::acquisition::GuardLocationEntityRef,
    job_nearby::JobCapacityState,
    main_base_type9_abort::{
        exact_level_one_type9_metadata, LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR,
        LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR,
    },
    ordinary_type9_current_task::OrdinaryType9CurrentTaskAuthority,
    ordinary_type9_go_to_job_initializer::{
        apply_ordinary_type9_go_to_job_task_transaction,
        plan_ordinary_type9_go_to_job_task_transaction, OrdinaryType9GoToJobAllocationDecision,
        OrdinaryType9GoToJobCandidateEvidence,
    },
    ordinary_type9_initial_selection::evaluate_nearby,
    ordinary_type9_live::{
        OrdinaryType9PendingInitialSelection, OrdinaryType9SelectedComponentRuntime,
        OrdinaryType9SelectedRuntimeKind,
    },
    ordinary_type9_run_away_initializer::{
        apply_ordinary_type9_run_away_acquiring_task_transaction_parts,
        OrdinaryType9RunAwayAllocationDecision, OrdinaryType9RunAwayTaskTransactionOutcome,
    },
    ordinary_type9_wander_initializer::{
        apply_ordinary_type9_wander_task_transaction, OrdinaryType9WanderAllocationDecision,
        OrdinaryType9WanderTaskTransactionOutcome,
    },
    wrapped_axis_range::WrappedAxisRange,
};
use v2k_formats::terrain::TerrainGrid;

#[cfg(test)]
mod tests;

pub const INTRO2_TYPE9_SPAWN_INDICES: [usize; 13] =
    [2, 3, 9, 11, 16, 17, 18, 19, 49, 50, 58, 59, 60];
const BIRTHS: [(usize, [u8; 2], u8); 13] = [
    (2, [140, 126], 0x02),
    (3, [142, 126], 0x03),
    (9, [142, 128], 0x09),
    (11, [141, 124], 0x0b),
    (16, [180, 17], 0x0f),
    (17, [174, 37], 0x10),
    (18, [181, 19], 0x11),
    (19, [185, 22], 0x12),
    (49, [188, 22], 0x20),
    (50, [191, 21], 0x21),
    (58, [191, 125], 0x24),
    (59, [159, 128], 0x25),
    (60, [162, 127], 0x26),
];

/// Immutable native-allocation identity. Task and component custody remain
/// in the shared Type9 fields and survive a style change independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type9Runtime {
    entity_id: u32,
    spawn_index: usize,
    birth_anchor_raw: [i16; 3],
    sub_d_seed: u8,
    birth_context: BehaviorContextRuntime,
    birth_tasks: [Option<ActorTaskVisit>; 3],
    birth_authority_pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type9Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
    pub nearby_weights: [i32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type9Error {
    Identity,
    Metadata,
    AlreadyPublished,
    ComponentStorage,
    Prefix,
    NearbyEvidence,
    PlayerCandidateOutsideIntro2Contract,
    JobTargetEvidence,
    Selection,
    TaskPublication,
}

pub(crate) fn intro2_type9_allocation_authenticates(entity: &Entity) -> bool {
    entity.intro2_type9_runtime.is_some_and(|runtime| {
        entity.active
            && entity.id == runtime.entity_id
            && entity.entity_type == 9
            && entity.authored_spawn_index == Some(runtime.spawn_index)
            && entity.model_slots == [Some(558); 4]
            && BIRTHS.iter().any(|&(spawn, xz, seed)| {
                spawn == runtime.spawn_index
                    && seed == runtime.sub_d_seed
                    && [runtime.birth_anchor_raw[0], runtime.birth_anchor_raw[2]]
                        == xz.map(|cell| (u16::from(cell) << 8) as i16)
            })
    })
}

pub(crate) fn intro2_type9_birth_tasks_authenticate(entity: &Entity) -> bool {
    intro2_type9_allocation_authenticates(entity)
        && entity.intro2_type9_runtime.is_some_and(|runtime| {
            runtime.birth_authority_pending
                && entity.current_behavior_context
                    == RetailRuntimeValue::Known(Some(runtime.birth_context))
                && matches!(entity.initial_behavior, RetailRuntimeValue::Known(Some(selection))
                    if BehaviorContextRuntime::from_fresh_weighted_selection(selection)
                        == Some(runtime.birth_context))
                && task_visits(entity) == runtime.birth_tasks
                && !entity.collision.fresh_level1_type9_first_scheduler_pending
        })
}

/// Transfer the exact native constructor graph to the shared selected-task
/// scheduler once. Allocation authenticity survives this claim and later
/// task/death transitions; only this initial graph's admission is consumed.
pub(crate) fn take_intro2_type9_task_authority(
    entity: &mut Entity,
) -> Option<OrdinaryType9CurrentTaskAuthority> {
    let authority = OrdinaryType9CurrentTaskAuthority::from_intro2_birth(entity)?;
    entity
        .intro2_type9_runtime
        .as_mut()?
        .birth_authority_pending = false;
    Some(authority)
}

fn task_visits(entity: &Entity) -> [Option<ActorTaskVisit>; 3] {
    ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
        entity
            .actor_tasks
            .task_in_slot(slot)
            .map(|task_id| ActorTaskVisit { slot, task_id })
    })
}

fn candidate(entity: &Entity) -> GuardLocationEntityRef {
    GuardLocationEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
    }
}

/// Called only within the native Intro2 constructor's authored prefix walk.
/// A pending first-query reset belongs to this exact authored allocation.
pub(crate) fn publish_intro2_type9(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    preceding: &[Entity],
    terrain: &TerrainGrid,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type9Publication, Intro2Type9Error> {
    let Some(&(spawn, xz, seed)) = BIRTHS
        .iter()
        .find(|(spawn, _, _)| entity.authored_spawn_index == Some(*spawn))
    else {
        return Err(Intro2Type9Error::Identity);
    };
    let xz = xz.map(|cell| (u16::from(cell) << 8) as i16);
    if !entity.active
        || entity.entity_type != 9
        || entity.model_slots != [Some(558); 4]
        || [entity.position_raw()[0], entity.position_raw()[2]] != xz
        || entity.rotation_heading_pitch_roll_raw() != [0; 3]
    {
        return Err(Intro2Type9Error::Identity);
    }
    if !exact_level_one_type9_metadata(metadata) {
        return Err(Intro2Type9Error::Metadata);
    }
    if entity.intro2_type9_runtime.is_some()
        || entity.ordinary_type9_pending_initial_selection.is_some()
        || entity.ordinary_type9_selected_component_runtime.is_some()
        || entity.main_base_type9_death_component_runtime.is_some()
        || entity.initial_behavior != RetailRuntimeValue::Unresolved
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .iter()
            .any(|&slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Intro2Type9Error::AlreadyPublished);
    }
    if entity.collision.recent_relation_id_at_0x60 != RetailRuntimeValue::Known(None)
        || entity.collision.health_raw != RetailRuntimeValue::Known(1500)
        || !matches!(entity.sub_a_propulsion_runtime,RetailRuntimeValue::Known(Some(sub_a)) if sub_a.drive_scale_percent()==100)
        || !matches!(
            entity.actor_animation_runtime,
            RetailRuntimeValue::Known(Some(_))
        )
    {
        return Err(Intro2Type9Error::ComponentStorage);
    }
    if preceding
        .iter()
        .any(|e| e.authored_spawn_index.is_none_or(|index| index >= spawn))
    {
        return Err(Intro2Type9Error::Prefix);
    }
    let anchor = [xz[0], terrain.bilinear_height_raw(xz[0], xz[1]), xz[1]];
    let mut owner = candidate(entity);
    owner.position_raw = anchor;
    let prefix: Vec<_> = preceding
        .iter()
        .filter(|e| e.active)
        .map(candidate)
        .collect();
    let range =
        WrappedAxisRange::from_raw(LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR.strict_axis_limit_raw);
    let mut weights = [0; 3];
    for (weight, mask) in weights.iter_mut().zip([8, 1, 0x20]) {
        *weight = evaluate_nearby(owner, &prefix, range, mask)
            .map_err(|_| Intro2Type9Error::NearbyEvidence)?
            .weight();
    }
    // Intro2 has no persistent player allocation. Preserve this explicit
    // constructor boundary instead of dropping class45's event/sound effects.
    if weights[1] != 0 {
        return Err(Intro2Type9Error::PlayerCandidateOutsideIntro2Contract);
    }
    entity.sub_a_propulsion_runtime =
        RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_20450_constructor(
            LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR,
            next_random() as u16,
        )));
    let mut selector_word = 0;
    let selection = select_initial_behavior(
        &metadata.initializer.as_ref().unwrap().behavior_choices,
        |rule| match rule {
            BehaviorWeightRule::BaddieNearby => weights[0],
            BehaviorWeightRule::PlayerNearby => weights[1],
            BehaviorWeightRule::BaseNearby => weights[2],
            BehaviorWeightRule::Always => 1,
            _ => unreachable!("exact Type9 selector"),
        },
        || {
            selector_word = next_random();
            selector_word
        },
    )
    .map_err(|_| Intro2Type9Error::Selection)?
    .ok_or(Intro2Type9Error::Selection)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Intro2Type9Error::Selection)?;
    entity.set_position_raw(anchor);
    entity.actor_common_axis_descriptor =
        RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR);
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    let kind = match selection.program.class_id {
        6 => {
            if !matches!(
                apply_ordinary_type9_wander_task_transaction(
                    entity,
                    250,
                    |_| OrdinaryType9WanderAllocationDecision::Prepared,
                    &mut *next_random
                ),
                OrdinaryType9WanderTaskTransactionOutcome::Published(_)
            ) {
                return Err(Intro2Type9Error::TaskPublication);
            }
            OrdinaryType9SelectedRuntimeKind::WanderNearPublished
        }
        10 => {
            let position = entity.position_raw();
            let RetailRuntimeValue::Known(Some(sub_a)) = &mut entity.sub_a_propulsion_runtime
            else {
                unreachable!()
            };
            if !matches!(
                apply_ordinary_type9_run_away_acquiring_task_transaction_parts(
                    position,
                    &mut entity.actor_tasks,
                    sub_a,
                    metadata,
                    |_| OrdinaryType9RunAwayAllocationDecision::Prepared,
                    &mut *next_random,
                    |_| {}
                ),
                OrdinaryType9RunAwayTaskTransactionOutcome::Published { .. }
            ) {
                return Err(Intro2Type9Error::TaskPublication);
            }
            OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished
        }
        54 => {
            let evidence: Vec<_> = preceding
                .iter()
                .filter(|e| e.active)
                .map(|e| OrdinaryType9GoToJobCandidateEvidence {
                    candidate_id: e.id,
                    state_flags: e.collision.state_flags_at_0x08,
                    capability_flags: RetailRuntimeValue::Known(e.capability_flags),
                    capacity: match e.base_factory_runtime {
                        RetailRuntimeValue::Known(Some(state)) => {
                            RetailRuntimeValue::Known(Some(JobCapacityState {
                                current_jobs_raw: i32::from(state.current_scientists),
                                capacity_raw: i32::from(state.required_scientists),
                            }))
                        }
                        RetailRuntimeValue::Known(None) => RetailRuntimeValue::Known(None),
                        RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
                    },
                })
                .collect();
            let job_owner = GoToJobOwner::from_type_metadata(
                entity.id,
                anchor,
                RetailRuntimeValue::Known(entity.capability_flags),
                9,
                metadata,
            );
            let transaction = plan_ordinary_type9_go_to_job_task_transaction(
                job_owner, &prefix, &evidence, range,
            )
            .map_err(|_| Intro2Type9Error::JobTargetEvidence)?;
            apply_ordinary_type9_go_to_job_task_transaction(
                entity,
                transaction,
                |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
                &mut *next_random,
            )
            .map_err(|_| Intro2Type9Error::TaskPublication)?;
            OrdinaryType9SelectedRuntimeKind::GoToJobPublished
        }
        _ => return Err(Intro2Type9Error::Selection),
    };
    entity.ordinary_type9_selected_component_runtime =
        Some(OrdinaryType9SelectedComponentRuntime::new(
            OrdinaryType9PendingInitialSelection::from_native_constructor(
                anchor,
                intro2_type9_first_query_owner_for_birth(spawn, seed)
                    .expect("authenticated Intro2 Type9 birth has its own first-query witness"),
            ),
            kind,
        ));
    entity.intro2_type9_runtime = Some(Intro2Type9Runtime {
        entity_id: entity.id,
        spawn_index: spawn,
        birth_anchor_raw: anchor,
        sub_d_seed: seed,
        birth_context: context,
        birth_tasks: task_visits(entity),
        birth_authority_pending: true,
    });
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0, 0, 0));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    Ok(Intro2Type9Publication {
        selection,
        selector_word,
        nearby_weights: weights,
    })
}
