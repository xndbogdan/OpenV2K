//! Type8's post-unwind 401120 -> 40AC60(NULL) -> 425680 boundary.
//!
//! All evidence reads finish before the weighted word. The two admitted
//! initializers are infallible port allocations, so the selected plan and its
//! one constructor word are consumed synchronously without a redraw window.

use super::*;
use crate::actor_task_owner::PreparedActorTask;
use crate::entity_behavior::{
    behavior_program, initial_behavior_state_policy, select_initial_behavior,
    BehaviorChoiceListSource, BehaviorDescriptorIdentity, BehaviorWeightRule,
};
use crate::entity_collision_state::ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT;
use crate::go_to_job::{
    plan_go_to_job_setup, GoToJobCandidate, GoToJobOwner, GoToJobSetupError, GoToJobSetupRequest,
};
use crate::go_to_job_owner::{GoToJobTaskState, GoToJobTransitionReason};
use crate::job_nearby::{
    evaluate_job_nearby, JobCapacityState, JobNearbyCandidate, JobNearbyEvaluationError,
    JobNearbyEvaluationRequest, JobNearbyOwner,
};
use crate::ordinary_type9_wander_owner::{
    plan_ordinary_type9_wander_setup, OrdinaryType9WanderTransitionReason,
};
use std::convert::Infallible;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type8RootTransitionReason {
    GoToJob(GoToJobTransitionReason),
    Wander(OrdinaryType9WanderTransitionReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type8RootTransitionBlock {
    StateUnavailable,
    DyingAlternateUnclaimed,
    InitializerUnavailable,
    UnsupportedChoiceList,
    ComponentRuntimeUnavailable,
    JobNearby(JobNearbyEvaluationError),
    GoToJob(GoToJobSetupError),
}

pub(super) fn context_matches_task(context: BehaviorContextRuntime, task: ScientistTask) -> bool {
    let class_id = match task {
        ScientistTask::GoToJob { .. } => 54,
        ScientistTask::Wander => 6,
    };
    let program = behavior_program(class_id).expect("shared scientist programs are audited");
    context.descriptor() == BehaviorDescriptorIdentity::Named(program)
        && context.choice_list_source()
            == RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        && context.style_table_index_raw_at_0x10() == program.initial_style_table_index_raw
        && context.active_style()
            == crate::entity_behavior::ActiveBehaviorStyle::Audited(program.initial_style)
}

pub(super) fn resume_root_transition(
    manager: &mut EntityManager,
    mut owner: Type8GoToJobSchedulerOwner,
    world_fx: &mut WorldFx,
    metadata: &EntityTypeRuntimeMetadata,
) -> Type8GoToJobSchedulerOwnerTick {
    let VisitPhase::RootPending {
        reason,
        elapsed_micros,
    } = owner.phase
    else {
        unreachable!("only a consumed callback may enter root reselection")
    };
    let blocked = |owner: Type8GoToJobSchedulerOwner, block| Type8GoToJobSchedulerOwnerTick {
        outcome: Type8GoToJobSchedulerProductionOutcome::TransitionBlocked {
            entity_id: owner.entity_id(),
            reason,
            block,
        },
        retained_owner: Some(owner),
    };
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("caller authenticated the live owner");
    let state = entity.collision.state_flags_at_0x08;
    let master_gate = match state.masked(COMMON_MASTER_MOTION_REQUIRED_STATE_MASK) {
        RetailRuntimeValue::Known(bits) => bits,
        RetailRuntimeValue::Unresolved => {
            return blocked(owner, Type8RootTransitionBlock::StateUnavailable)
        }
    };
    match state.masked(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT) {
        RetailRuntimeValue::Unresolved => {
            return blocked(owner, Type8RootTransitionBlock::StateUnavailable)
        }
        RetailRuntimeValue::Known(bits) if bits != 0 => {
            let entity = manager
                .type8_go_to_job_entity_mut(owner.entity_id())
                .unwrap();
            finish_completed_frame(entity, elapsed_micros, master_gate);
            owner.phase = VisitPhase::Ready;
            return Type8GoToJobSchedulerOwnerTick {
                outcome: Type8GoToJobSchedulerProductionOutcome::TransitionSuppressed {
                    entity_id: owner.entity_id(),
                    reason,
                },
                retained_owner: Some(owner),
            };
        }
        _ => {}
    }
    match state.masked(DYING_STATE_BIT) {
        RetailRuntimeValue::Unresolved => {
            return blocked(owner, Type8RootTransitionBlock::StateUnavailable)
        }
        RetailRuntimeValue::Known(bits) if bits != 0 => {
            return blocked(owner, Type8RootTransitionBlock::DyingAlternateUnclaimed)
        }
        _ => {}
    }
    let Some(initializer) = metadata.initializer.as_ref() else {
        return blocked(owner, Type8RootTransitionBlock::InitializerUnavailable);
    };
    // Normal-tier Level1 Type8 authors these two choices. Preserve their
    // authored order; don't turn a forced test table into retail authority.
    let choices = &initializer.behavior_choices;
    if choices.len() != 2
        || !choices.iter().any(|choice| {
            (
                choice.weight_rule_id,
                choice.weight_multiplier,
                choice.behavior_class_id,
            ) == (13, 100, 54)
        })
        || !choices.iter().any(|choice| {
            (
                choice.weight_rule_id,
                choice.weight_multiplier,
                choice.behavior_class_id,
            ) == (1, 1, 6)
        })
    {
        return blocked(owner, Type8RootTransitionBlock::UnsupportedChoiceList);
    }
    let RetailRuntimeValue::Known(topology) = metadata.common_mover_topology else {
        return blocked(owner, Type8RootTransitionBlock::ComponentRuntimeUnavailable);
    };
    let RetailRuntimeValue::Known(Some(sub_a_descriptor)) = metadata.sub_a_propulsion_descriptor
    else {
        return blocked(owner, Type8RootTransitionBlock::ComponentRuntimeUnavailable);
    };
    if !topology.sub_a
        || topology.sub_h
        || topology.sub_g
        || topology.sub_f
        || !matches!(
            entity.sub_a_propulsion_runtime,
            RetailRuntimeValue::Known(Some(_))
        )
    {
        return blocked(owner, Type8RootTransitionBlock::ComponentRuntimeUnavailable);
    }
    let range =
        WrappedAxisRange::from_raw(initializer.common_axis_descriptor.strict_axis_limit_raw);
    let capacity = |candidate: &Entity| match candidate.base_factory_runtime {
        RetailRuntimeValue::Known(Some(state)) => {
            RetailRuntimeValue::Known(Some(JobCapacityState {
                current_jobs_raw: i32::from(state.current_scientists),
                capacity_raw: i32::from(state.required_scientists),
            }))
        }
        RetailRuntimeValue::Known(None) => RetailRuntimeValue::Known(None),
        RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
    };
    let candidates = manager
        .iter_all()
        .map(|candidate| JobNearbyCandidate {
            id: candidate.id,
            position_raw: candidate.position_raw(),
            state_flags: candidate.collision.state_flags_at_0x08,
            capacity: capacity(candidate),
        })
        .collect::<Vec<_>>();
    let job_nearby = match evaluate_job_nearby(JobNearbyEvaluationRequest {
        owner: JobNearbyOwner {
            id: entity.id,
            position_raw: entity.position_raw(),
            capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        },
        candidates_in_intrusive_order: &candidates,
        range,
    }) {
        Ok(value) => value,
        Err(error) => return blocked(owner, Type8RootTransitionBlock::JobNearby(error)),
    };
    let go_owner = GoToJobOwner::from_type_metadata(
        entity.id,
        entity.position_raw(),
        RetailRuntimeValue::Known(entity.capability_flags),
        8,
        metadata,
    );
    let go_plan = if job_nearby {
        let candidates = manager
            .iter_all()
            .map(|candidate| GoToJobCandidate {
                id: candidate.id,
                position_raw: candidate.position_raw(),
                state_flags: candidate.collision.state_flags_at_0x08,
                capability_flags: RetailRuntimeValue::Known(candidate.capability_flags),
                capacity: capacity(candidate),
            })
            .collect::<Vec<_>>();
        match plan_go_to_job_setup(GoToJobSetupRequest {
            owner: go_owner,
            candidates_in_intrusive_order: &candidates,
            range,
        }) {
            Ok(plan) => Some(plan),
            Err(error) => return blocked(owner, Type8RootTransitionBlock::GoToJob(error)),
        }
    } else {
        None
    };
    // No fallible dependency remains. 425680's one word precedes AF90/AD10's
    // one constructor word, and the new Primary is not visited this frame.
    let selection = select_initial_behavior(
        choices,
        |rule| match rule {
            BehaviorWeightRule::Always => 1,
            BehaviorWeightRule::JobNearby => i32::from(job_nearby),
            _ => unreachable!("exact Type8 choices"),
        },
        || u32::from(world_fx.next_shared_retail_random_u16()),
    )
    .expect("exact choices")
    .expect("class6 always has positive weight");
    let context = owner
        .context
        .reselect_named_type_default(
            selection.program,
            selection.program.initial_style_table_index_raw,
            selection.program.initial_style,
        )
        .expect("canonical class6/54 context replacement");
    let entity = manager
        .type8_go_to_job_entity_mut(owner.entity_id())
        .unwrap();
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    owner.task = match selection.program.class_id {
        54 => {
            let plan = go_plan.expect("positive class54 weight preflights its target constructor");
            let target_id = plan.target_id();
            let runtime = go_owner
                .bind_entity_runtime(entity)
                .expect("same-entity SubA preflight");
            plan.apply(
                runtime,
                || u32::from(world_fx.next_shared_retail_random_u16()),
                |specification| {
                    Ok::<_, Infallible>(PreparedActorTask::new(ActorTaskRuntime::GoToJob(
                        GoToJobTaskState::after_allocation(specification),
                    )))
                },
            )
            .expect("infallible prepared constructor");
            ScientistTask::GoToJob { target_id }
        }
        6 => {
            let position = entity.position_raw();
            let Entity {
                actor_tasks,
                sub_a_propulsion_runtime,
                ..
            } = entity;
            let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
                unreachable!("preflight")
            };
            plan_ordinary_type9_wander_setup(position, sub_a_descriptor.target_speed_base_raw)
                .apply(actor_tasks, sub_a, |specification| {
                    Ok::<_, Infallible>(
                        specification
                            .prepare_after_allocation(|| {
                                u32::from(world_fx.next_shared_retail_random_u16())
                            })
                            .map_task(ActorTaskRuntime::OrdinaryType9Wander),
                    )
                })
                .expect("infallible prepared constructor");
            ScientistTask::Wander
        }
        _ => unreachable!("exact choices"),
    };
    let predecessor_task_id = owner.primary_task_id;
    owner.primary_task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .expect("initializer published Primary");
    owner.context = context;
    owner.phase = VisitPhase::Ready;
    // Both admitted initializers reinstall the same 402DA0 component pair
    // callback as the constructor; the entity's audited pair identity stays.
    let post_task_gate = entity
        .collision
        .state_flags_at_0x08
        .masked(COMMON_MASTER_MOTION_REQUIRED_STATE_MASK);
    let RetailRuntimeValue::Known(post_task_gate) = post_task_gate else {
        unreachable!("masked style writes preserve preflight evidence")
    };
    finish_completed_frame(entity, elapsed_micros, post_task_gate);
    Type8GoToJobSchedulerOwnerTick {
        outcome: Type8GoToJobSchedulerProductionOutcome::Transition {
            entity_id: owner.entity_id(),
            reason,
            selection,
            predecessor_task_id,
            published_task_id: owner.primary_task_id,
        },
        retained_owner: Some(owner),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common_mover::{
        sub_d::{type8_first_query_owner_for_seed, Type9SubDRuntime},
        SubAPropulsionRuntime,
    };
    use crate::entity::EntityKind;
    use crate::entity_collision_state::RetailStateWord;
    use crate::ordinary_type9_wander_owner::OrdinaryType9WanderTaskState;

    #[v2k_test_support::retail_test]
    fn pending_root_retains_consumed_callback_across_descriptor_metadata_repair() {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").exists(),
            "Type8 root metadata test requires retail PRELOAD.DAT"
        );
        let mut session = crate::session::GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        let metadata =
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(8).unwrap());
        let mut scientist = Entity::unresolved_port_entity(7, EntityKind::from_type(8), 8);
        scientist.type8_wander_anchor_raw_at_0x90 = RetailRuntimeValue::Known([0; 3]);
        scientist.type8_sub_d_frame_owner = type8_first_query_owner_for_seed(0x38);
        scientist.type8_sub_d_runtime = Some(Type9SubDRuntime::from_constructor());
        scientist.collision.state_flags_at_0x08 = RetailStateWord::from_known_bits(0, u32::MAX);
        scientist.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(10), 1, 100),
        ));
        let program = behavior_program(6).unwrap();
        let context = BehaviorContextRuntime::named_audited(
            program,
            0,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(0),
            program.initial_style,
        )
        .unwrap();
        scientist.current_behavior_context = RetailRuntimeValue::Known(Some(context));
        scientist.actor_tasks.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(
                OrdinaryType9WanderTaskState::from_allocated_anchor([0; 3]),
            )),
        );
        let task_id = scientist
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        // Explicit already-unwound fixture: metadata repair must not invoke
        // this old task's stochastic callback or re-evaluate its lifetime.
        let owner = Type8GoToJobSchedulerOwner {
            entity_id: 7,
            primary_task_id: task_id,
            task: ScientistTask::Wander,
            context,
            immutable_anchor_raw: [0; 3],
            phase: VisitPhase::RootPending {
                reason: Type8RootTransitionReason::Wander(
                    OrdinaryType9WanderTransitionReason::LifetimeExpired,
                ),
                elapsed_micros: 20_000,
            },
        };
        let mut types = vec![EntityTypeRuntimeMetadata::default(); 9];
        types[8] = metadata.clone();
        types[8].sub_a_propulsion_descriptor = RetailRuntimeValue::Unresolved;
        let mut manager =
            EntityManager::from_entities_with_type_metadata_for_test(vec![scientist], types, false);
        let mut fx = WorldFx::new();
        let blocked =
            tick_type8_go_to_job_scheduler_owner(&mut manager, owner, &mut fx, None, 999_000);
        assert!(
            matches!(
                blocked.outcome,
                Type8GoToJobSchedulerProductionOutcome::TransitionBlocked {
                    block: Type8RootTransitionBlock::ComponentRuntimeUnavailable,
                    ..
                }
            ),
            "{:?}",
            blocked.outcome
        );
        *manager.type_runtime_metadata_mut_for_test(8).unwrap() = metadata;
        let completed = tick_type8_go_to_job_scheduler_owner(
            &mut manager,
            blocked.retained_owner.unwrap(),
            &mut fx,
            None,
            999_000,
        );
        assert!(
            matches!(
                completed.outcome,
                Type8GoToJobSchedulerProductionOutcome::Transition {
                    reason: Type8RootTransitionReason::Wander(
                        OrdinaryType9WanderTransitionReason::LifetimeExpired
                    ),
                    ..
                }
            ),
            "{:?}",
            completed.outcome
        );
        let mut expected = WorldFx::new();
        expected.next_shared_retail_random_u16();
        expected.next_shared_retail_random_u16();
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
    }
}
