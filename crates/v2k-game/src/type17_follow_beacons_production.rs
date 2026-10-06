//! Production scheduler attachment for fresh Level-1 Follow Beacons.
//!
//! Cold New-Game construction already publishes class-33 variant zero on the
//! authenticated spawn-18 graph. This owner moves that published lease into
//! the specialized scheduler and ticks the closed live acquire/handoff path
//! against the current intrusive list. A positive type-52 score publishes
//! variant one. The next visit drives mover-first `FUN_00403CE0`: type-17
//! `FUN_00401430` on A/B/C/D/H/J, live target validation, `FUN_00423030`
//! through controller `+0x28`, and the `0x300` X/Z reached test. The common
//! mover applies the `V200002.run` first `FUN_0041FCB0` full-reset for
//! spawn-18 seed `0x32`. Live following visits then rebuild `FUN_00413F70`,
//! apply E870/DCA0 `FUN_0040E100` for type `+0xC0` `0x39`, integrate
//! `FUN_00412DA0`, and commit `FUN_0041D360` onto the eight-record Sub-H
//! so `FUN_0041D0A0` can start a stride from constructor-zero flags.
//! Direct `--level 13` and Capture People/Run Away births stay out of
//! custody.

use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::actor_task_owner::{ActorTaskSlot, ActorTaskVisit};
use crate::common_dying_live::LevelOneType17CommonDyingOwner;
use crate::common_mover::sub_d::type17_seed_for_fresh_level1_spawn;
use crate::common_mover::target_prelude::CommonMoverTrackedTargetSnapshot;
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::common_mover::SubAPropulsionRuntime;
use crate::entity::{
    apply_first_world_type17_environment_raw, commit_known_common_master_motion, Entity,
    EntityManager,
};
use crate::entity_collision_state::{
    EntityTypeRuntimeMetadata, RetailRuntimeValue, DYING_STATE_BIT,
};
use crate::follow_beacons::{
    evaluate_follow_beacons_following_callback, FollowBeaconsFollowingCallbackError,
    FollowBeaconsFollowingCallbackPrefix, FollowBeaconsFollowingCallbackRequest,
    FollowBeaconsFollowingCallbackResult, FollowBeaconsFollowingPostMoverPositions,
    FollowBeaconsFollowingReachedTarget, FollowBeaconsFollowingTaggedSingleton,
    FollowBeaconsFollowingTargetRuntimeState,
};
use crate::resource_cache::ResourceCache;
use crate::static_damage::StaticDamageScheduler;
use crate::sub_h_external_frame::commit_sub_h_geometry;
use crate::type17_follow_beacons_live::{
    FollowBeaconLiveEntitySnapshot, Type17FollowBeaconsAcquiringOwner,
    Type17FollowBeaconsAcquisitionOutcome, Type17FollowBeaconsFollowingOwner,
    Type17FollowBeaconsLiveError,
};
use crate::type17_follow_beacons_mover::{
    evaluate_type17_follow_beacons_common_mover, Type17FollowBeaconsMoverBlock,
    Type17FollowBeaconsMoverOutcome, Type17FollowBeaconsMoverRequest,
    TYPE17_FOLLOW_BEACONS_SCHEDULER_MODE,
};
use crate::type17_impact_live::TYPE17_FOLLOW_BEACONS_LIVE_AXIS_DESCRIPTOR;
use crate::type17_impact_reselection::TYPE17_IMPACT_ENTITY_TYPE;
use crate::type17_static_contact_live::{
    resolve_type17_follow_static_contact, Type17StaticContactError, Type17StaticContactFrame,
    Type17StaticContactOutcome,
};
use crate::world_fx::WorldFx;
use crate::wrapped_axis_range::{within_wrapped_axis_range, WrappedAxisRange};
use v2k_formats::collision::CommonAxisDescriptor;
use v2k_formats::terrain::TerrainGrid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type17FollowBeaconsSchedulerOwner {
    Acquiring(Type17FollowBeaconsAcquiringOwner),
    Following(Type17FollowBeaconsFollowingOwner),
}

impl Type17FollowBeaconsSchedulerOwner {
    pub const fn entity_id(self) -> u32 {
        match self {
            Self::Acquiring(owner) => owner.entity_id(),
            Self::Following(owner) => owner.entity_id(),
        }
    }

    pub(crate) fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }

    pub fn adopt_published(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<Self, Type17FollowBeaconsLiveError> {
        if let Ok(owner) = Type17FollowBeaconsAcquiringOwner::adopt_published(entity, metadata) {
            return Ok(Self::Acquiring(owner));
        }
        Type17FollowBeaconsFollowingOwner::adopt_published(entity, metadata).map(Self::Following)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type17FollowBeaconsSchedulerProductionDrop {
    EntityUnavailable,
    GraphMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type17FollowBeaconsSchedulerProductionBlock {
    VisitUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type17FollowBeaconsFollowingVisitResult {
    Tagged(FollowBeaconsFollowingTaggedSingleton),
    Continue,
    CommonMoverBlocked(Type17FollowBeaconsMoverBlock),
    TargetValidationUnresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type17FollowBeaconsFollowingVisit {
    pub prefix: FollowBeaconsFollowingCallbackPrefix,
    pub result: Type17FollowBeaconsFollowingVisitResult,
    /// The outer static pass runs only after a committed mover and task unwind.
    pub static_contact: Option<Result<Type17StaticContactOutcome, Type17StaticContactError>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Type17FollowBeaconsControllerContext {
    axis_at_0x28: CommonAxisDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type17FollowBeaconsSchedulerProductionOutcome {
    Acquiring {
        entity_id: u32,
        outcome: Type17FollowBeaconsAcquisitionOutcome,
    },
    Following {
        entity_id: u32,
        target_id: u32,
        visit: Type17FollowBeaconsFollowingVisit,
    },
    Blocked {
        entity_id: u32,
        reason: Type17FollowBeaconsSchedulerProductionBlock,
    },
    Dropped {
        entity_id: u32,
        reason: Type17FollowBeaconsSchedulerProductionDrop,
    },
}

impl Type17FollowBeaconsSchedulerProductionOutcome {
    pub fn published_common_dying(&self) -> Option<LevelOneType17CommonDyingOwner> {
        let Self::Following { visit, .. } = self else {
            return None;
        };
        let Some(Ok(Type17StaticContactOutcome::Applied(contact))) = &visit.static_contact else {
            return None;
        };
        contact
            .actor_damage
            .as_ref()
            .and_then(|damage| damage.common_dying)
    }

    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::Acquiring { entity_id, .. }
            | Self::Following { entity_id, .. }
            | Self::Blocked { entity_id, .. }
            | Self::Dropped { entity_id, .. } => *entity_id,
        }
    }
}

pub struct Type17FollowBeaconsSchedulerOwnerTick {
    pub outcome: Type17FollowBeaconsSchedulerProductionOutcome,
    pub retained_owner: Option<Type17FollowBeaconsSchedulerOwner>,
}

pub fn follow_beacon_live_list_snapshot(
    manager: &EntityManager,
) -> Vec<FollowBeaconLiveEntitySnapshot> {
    manager
        .iter_all()
        .map(FollowBeaconLiveEntitySnapshot::from_entity)
        .collect()
}

pub struct Type17FollowBeaconsProductionFrame<'a> {
    pub resources: &'a mut ResourceCache,
    pub static_damage: &'a mut StaticDamageScheduler,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
}

pub fn tick_type17_follow_beacons_scheduler_owner(
    manager: &mut EntityManager,
    owner: Type17FollowBeaconsSchedulerOwner,
    frame: Type17FollowBeaconsProductionFrame<'_>,
) -> Type17FollowBeaconsSchedulerOwnerTick {
    let metadata = manager
        .type_runtime_metadata(TYPE17_IMPACT_ENTITY_TYPE)
        .cloned();
    let Some(metadata) = metadata else {
        return Type17FollowBeaconsSchedulerOwnerTick {
            outcome: Type17FollowBeaconsSchedulerProductionOutcome::Dropped {
                entity_id: owner.entity_id(),
                reason: Type17FollowBeaconsSchedulerProductionDrop::GraphMismatch,
            },
            retained_owner: None,
        };
    };
    match owner {
        Type17FollowBeaconsSchedulerOwner::Acquiring(acquiring) => {
            tick_acquiring(manager, acquiring, &metadata, frame.world_fx)
        }
        Type17FollowBeaconsSchedulerOwner::Following(following) => {
            tick_following(manager, following, &metadata, frame)
        }
    }
}

fn tick_acquiring(
    manager: &mut EntityManager,
    owner: Type17FollowBeaconsAcquiringOwner,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
) -> Type17FollowBeaconsSchedulerOwnerTick {
    let candidates = follow_beacon_live_list_snapshot(manager);
    let Some(entity) = manager.type17_follow_beacons_entity_mut(owner.entity_id()) else {
        return Type17FollowBeaconsSchedulerOwnerTick {
            outcome: Type17FollowBeaconsSchedulerProductionOutcome::Dropped {
                entity_id: owner.entity_id(),
                reason: Type17FollowBeaconsSchedulerProductionDrop::EntityUnavailable,
            },
            retained_owner: None,
        };
    };
    match owner.acquire_and_handoff(entity, metadata, &candidates, world_fx) {
        Ok(outcome @ Type17FollowBeaconsAcquisitionOutcome::NoPositiveScore { .. }) => {
            Type17FollowBeaconsSchedulerOwnerTick {
                outcome: Type17FollowBeaconsSchedulerProductionOutcome::Acquiring {
                    entity_id: owner.entity_id(),
                    outcome,
                },
                retained_owner: Some(Type17FollowBeaconsSchedulerOwner::Acquiring(owner)),
            }
        }
        Ok(
            outcome @ Type17FollowBeaconsAcquisitionOutcome::FollowingPublished {
                owner: following,
                ..
            },
        ) => Type17FollowBeaconsSchedulerOwnerTick {
            outcome: Type17FollowBeaconsSchedulerProductionOutcome::Acquiring {
                entity_id: following.entity_id(),
                outcome,
            },
            retained_owner: Some(Type17FollowBeaconsSchedulerOwner::Following(following)),
        },
        Ok(
            outcome @ Type17FollowBeaconsAcquisitionOutcome::InitializerFallbackPublished { .. },
        ) => Type17FollowBeaconsSchedulerOwnerTick {
            outcome: Type17FollowBeaconsSchedulerProductionOutcome::Acquiring {
                entity_id: owner.entity_id(),
                outcome,
            },
            retained_owner: None,
        },
        Err(_) => Type17FollowBeaconsSchedulerOwnerTick {
            outcome: Type17FollowBeaconsSchedulerProductionOutcome::Dropped {
                entity_id: owner.entity_id(),
                reason: Type17FollowBeaconsSchedulerProductionDrop::GraphMismatch,
            },
            retained_owner: None,
        },
    }
}

fn commit_type17_follow_pose(
    entity: &mut Entity,
    position_raw: [i16; 3],
    mut velocity_raw: [i16; 3],
    heading_raw: u16,
    sub_a_runtime: RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
    elapsed_micros: u32,
) {
    entity.set_heading_raw(heading_raw);
    let [heading_raw, pitch_raw, roll_raw] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
        heading_raw,
        pitch_raw,
        roll_raw,
    ));
    apply_first_world_type17_environment_raw(
        &mut velocity_raw,
        elapsed_micros,
        entity.mass_raw.max(1),
    );
    entity.set_motion_raw(position_raw, velocity_raw);
    if let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_runtime {
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(sub_a));
    }
    commit_known_common_master_motion(entity, elapsed_micros);
}

fn commit_type17_follow_geometry(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    terrain: Option<&TerrainGrid>,
    model_records: Option<&[[i16; 4]]>,
) {
    let Some(model_records) = model_records else {
        return;
    };
    let RetailRuntimeValue::Known(Some(descriptor)) = &metadata.sub_h_external_frame_descriptor
    else {
        return;
    };
    let axes = match entity.physical_body_basis_q31() {
        RetailRuntimeValue::Known(basis) => Some([basis.lateral, basis.up, basis.forward]),
        RetailRuntimeValue::Unresolved => None,
    };
    let origin_raw = entity.position_raw();
    let RetailRuntimeValue::Known(Some(runtime)) = &mut entity.sub_h_external_frame_runtime else {
        return;
    };
    let _ = commit_sub_h_geometry(
        runtime,
        descriptor,
        model_records,
        origin_raw,
        axes,
        |x, z| terrain.map_or(0, |grid| grid.bilinear_height_raw(x, z)),
    );
}

fn tick_following(
    manager: &mut EntityManager,
    owner: Type17FollowBeaconsFollowingOwner,
    metadata: &EntityTypeRuntimeMetadata,
    frame: Type17FollowBeaconsProductionFrame<'_>,
) -> Type17FollowBeaconsSchedulerOwnerTick {
    let Type17FollowBeaconsProductionFrame {
        resources,
        static_damage,
        world_fx,
        elapsed_micros,
        retail_tick,
    } = frame;
    let terrain = resources.level_terrain();
    let (owner_position_raw, velocity_raw, heading_raw, roll_raw, sub_a_runtime, axis_at_0x28) = {
        let Some(entity) = manager
            .iter_all()
            .find(|entity| entity.id == owner.entity_id())
        else {
            return Type17FollowBeaconsSchedulerOwnerTick {
                outcome: Type17FollowBeaconsSchedulerProductionOutcome::Dropped {
                    entity_id: owner.entity_id(),
                    reason: Type17FollowBeaconsSchedulerProductionDrop::EntityUnavailable,
                },
                retained_owner: None,
            };
        };
        if owner.validate(entity, metadata).is_err() {
            return Type17FollowBeaconsSchedulerOwnerTick {
                outcome: Type17FollowBeaconsSchedulerProductionOutcome::Dropped {
                    entity_id: owner.entity_id(),
                    reason: Type17FollowBeaconsSchedulerProductionDrop::GraphMismatch,
                },
                retained_owner: None,
            };
        }
        let RetailRuntimeValue::Known(axis_at_0x28) = entity.actor_common_axis_descriptor else {
            return Type17FollowBeaconsSchedulerOwnerTick {
                outcome: Type17FollowBeaconsSchedulerProductionOutcome::Dropped {
                    entity_id: owner.entity_id(),
                    reason: Type17FollowBeaconsSchedulerProductionDrop::GraphMismatch,
                },
                retained_owner: None,
            };
        };
        (
            entity.position_raw(),
            entity.velocity_raw(),
            entity.heading_raw(),
            entity.rotation_heading_pitch_roll_raw()[2] as u16,
            entity.sub_a_propulsion_runtime,
            axis_at_0x28,
        )
    };
    debug_assert_eq!(axis_at_0x28, TYPE17_FOLLOW_BEACONS_LIVE_AXIS_DESCRIPTOR);

    let (target_state, tracked_target, target_position_raw) =
        type17_follow_target_runtime_state(manager, owner.target_id());

    let Some(entity) = manager.type17_follow_beacons_entity_mut(owner.entity_id()) else {
        return Type17FollowBeaconsSchedulerOwnerTick {
            outcome: Type17FollowBeaconsSchedulerProductionOutcome::Dropped {
                entity_id: owner.entity_id(),
                reason: Type17FollowBeaconsSchedulerProductionDrop::EntityUnavailable,
            },
            retained_owner: None,
        };
    };
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id: owner.primary_task_id(),
    };
    let Some(prefix) = entity.actor_tasks.begin_exact_visit_with(visit, |runtime| {
        let ActorTaskRuntime::FollowBeaconsFollowing(state) = runtime else {
            unreachable!("following Primary is FollowBeaconsFollowing")
        };
        state.before_callback(elapsed_micros)
    }) else {
        return Type17FollowBeaconsSchedulerOwnerTick {
            outcome: Type17FollowBeaconsSchedulerProductionOutcome::Blocked {
                entity_id: owner.entity_id(),
                reason: Type17FollowBeaconsSchedulerProductionBlock::VisitUnavailable,
            },
            retained_owner: Some(Type17FollowBeaconsSchedulerOwner::Following(owner)),
        };
    };
    let Some(ActorTaskRuntime::FollowBeaconsFollowing(state)) =
        entity.actor_tasks.state_in_slot(ActorTaskSlot::Primary)
    else {
        if entity
            .actor_tasks
            .wrapper_flags(visit.task_id)
            .is_some_and(|flags| flags.in_callback)
        {
            let _ = entity.actor_tasks.finish_exact_visit(visit);
        }
        return Type17FollowBeaconsSchedulerOwnerTick {
            outcome: Type17FollowBeaconsSchedulerProductionOutcome::Dropped {
                entity_id: owner.entity_id(),
                reason: Type17FollowBeaconsSchedulerProductionDrop::GraphMismatch,
            },
            retained_owner: None,
        };
    };
    let mut stage = state.stage_callback();
    let mut movement = ();
    let mut controller = Type17FollowBeaconsControllerContext { axis_at_0x28 };
    let mut applied_mover = None;
    let evaluated = evaluate_follow_beacons_following_callback(
        &mut stage,
        FollowBeaconsFollowingCallbackRequest {
            visit,
            entity_id: owner.entity_id(),
            movement_state: &mut movement,
            controller_context: &mut controller,
            elapsed_micros,
            scheduler_mode: TYPE17_FOLLOW_BEACONS_SCHEDULER_MODE as u32,
        },
        |request| {
            let outcome = evaluate_type17_follow_beacons_common_mover(
                Type17FollowBeaconsMoverRequest {
                    entity_id: owner.entity_id(),
                    entity_type: TYPE17_IMPACT_ENTITY_TYPE,
                    metadata: Some(metadata),
                    position_raw: owner_position_raw,
                    velocity_raw,
                    heading_raw,
                    roll_raw,
                    sub_a_runtime,
                    sub_d_stagger_seed: entity
                        .authored_spawn_index
                        .and_then(type17_seed_for_fresh_level1_spawn),
                    sub_d_frame_owner: entity.type17_sub_d_frame_owner.as_mut(),
                    sub_d_runtime: entity.type17_sub_d_runtime.as_mut(),
                    body_right_q31: match entity.physical_body_basis_q31 {
                        RetailRuntimeValue::Known(Type9BodyBasis { lateral, .. }) => {
                            RetailRuntimeValue::Known(lateral)
                        }
                        RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
                    },
                    body_forward_q31: match entity.physical_body_basis_q31 {
                        RetailRuntimeValue::Known(Type9BodyBasis { forward, .. }) => {
                            RetailRuntimeValue::Known(forward)
                        }
                        RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
                    },
                    body_up_q31: match entity.physical_body_basis_q31 {
                        RetailRuntimeValue::Known(Type9BodyBasis { up, .. }) => {
                            RetailRuntimeValue::Known(up)
                        }
                        RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
                    },
                    sub_h_runtime: match &mut entity.sub_h_external_frame_runtime {
                        RetailRuntimeValue::Known(Some(runtime)) => Some(runtime),
                        _ => None,
                    },
                    terrain,
                    target_private: request.target_state,
                    tracked_target,
                    elapsed_micros,
                    global_elapsed_micros: elapsed_micros,
                },
                || u32::from(world_fx.next_shared_retail_random_u16()),
            )?;
            let result = outcome.result;
            applied_mover = Some(outcome);
            Ok(result)
        },
        |_| target_state,
        |request| {
            Ok::<_, ()>(target_position_raw.is_some_and(|target_position_raw| {
                within_wrapped_axis_range(
                    WrappedAxisRange::from_raw(
                        request
                            .controller_context
                            .axis_at_0x28
                            .strict_axis_limit_raw,
                    ),
                    owner_position_raw,
                    target_position_raw,
                )
            }))
        },
        |_, _| {
            Ok::<_, ()>(FollowBeaconsFollowingPostMoverPositions {
                owner_position_raw,
                target_position_raw: target_position_raw.unwrap_or(owner_position_raw),
            })
        },
        None::<fn(FollowBeaconsFollowingReachedTarget) -> u32>,
    );
    let should_commit = match &evaluated {
        Ok(_) => true,
        Err(error) => error.mover_completed(),
    };
    if should_commit {
        if let Some(ActorTaskRuntime::FollowBeaconsFollowing(surviving)) =
            entity.actor_tasks.task_state_mut(visit.task_id)
        {
            stage.commit(surviving);
        }
    }
    if entity
        .actor_tasks
        .wrapper_flags(visit.task_id)
        .is_some_and(|flags| flags.in_callback)
    {
        let _ = entity.actor_tasks.finish_exact_visit(visit);
    }
    // The task dispatcher unwinds before the outer environment/integration
    // and contact suffix. A contact hook must not run under its tick receipt.
    let mut static_contact = None;
    if should_commit {
        if let Some(Type17FollowBeaconsMoverOutcome {
            position_raw,
            velocity_raw,
            heading_raw,
            sub_a_runtime,
            ..
        }) = applied_mover
        {
            commit_type17_follow_pose(
                entity,
                position_raw,
                velocity_raw,
                heading_raw,
                sub_a_runtime,
                elapsed_micros,
            );
            static_contact = Some(resolve_type17_follow_static_contact(
                manager,
                owner,
                metadata,
                Type17StaticContactFrame {
                    resources,
                    static_damage,
                    world_fx,
                    retail_tick,
                },
            ));
            // Re-resolve after collision damage: death may have replaced the
            // task and disabled Sub-H. The geometry writer consumes that live
            // post-contact pose/component state, never the pre-contact copy.
            if static_contact.as_ref().is_some_and(Result::is_ok) {
                if let Some(entity) = manager.type17_follow_beacons_entity_mut(owner.entity_id()) {
                    let model_records = entity
                        .model_index
                        .and_then(|id| resources.global_model(id))
                        .map(|model| model.records.as_slice());
                    commit_type17_follow_geometry(
                        entity,
                        metadata,
                        resources.level_terrain(),
                        model_records,
                    );
                }
            }
        }
    }
    let result = match evaluated {
        Ok(FollowBeaconsFollowingCallbackResult::Tagged(singleton)) => {
            Type17FollowBeaconsFollowingVisitResult::Tagged(singleton)
        }
        Ok(FollowBeaconsFollowingCallbackResult::Continue) => {
            Type17FollowBeaconsFollowingVisitResult::Continue
        }
        Ok(FollowBeaconsFollowingCallbackResult::PropagateStyleResult { .. }) => {
            Type17FollowBeaconsFollowingVisitResult::Continue
        }
        Err(FollowBeaconsFollowingCallbackError::CommonMover { error, .. }) => {
            Type17FollowBeaconsFollowingVisitResult::CommonMoverBlocked(error)
        }
        Err(FollowBeaconsFollowingCallbackError::TargetValidation { .. }) => {
            Type17FollowBeaconsFollowingVisitResult::TargetValidationUnresolved
        }
        Err(_) => Type17FollowBeaconsFollowingVisitResult::CommonMoverBlocked(
            Type17FollowBeaconsMoverBlock::SubARuntimeUnavailable,
        ),
    };
    let outcome = Type17FollowBeaconsSchedulerProductionOutcome::Following {
        entity_id: owner.entity_id(),
        target_id: owner.target_id(),
        visit: Type17FollowBeaconsFollowingVisit {
            prefix,
            result,
            static_contact,
        },
    };
    let contact_blocked = matches!(&outcome, Type17FollowBeaconsSchedulerProductionOutcome::Following { visit, .. } if matches!(visit.static_contact, Some(Err(_))));
    let retain = !contact_blocked && outcome.published_common_dying().is_none();
    Type17FollowBeaconsSchedulerOwnerTick {
        outcome,
        retained_owner: retain.then_some(Type17FollowBeaconsSchedulerOwner::Following(owner)),
    }
}

fn type17_follow_target_runtime_state(
    manager: &EntityManager,
    target_id: u32,
) -> (
    Result<FollowBeaconsFollowingTargetRuntimeState, ()>,
    RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    Option<[i16; 3]>,
) {
    let Some(target) = manager.iter_all().find(|entity| entity.id == target_id) else {
        return (
            Ok(FollowBeaconsFollowingTargetRuntimeState::Missing),
            RetailRuntimeValue::Known(None),
            None,
        );
    };
    if !target.active {
        return (
            Ok(FollowBeaconsFollowingTargetRuntimeState::Inactive),
            RetailRuntimeValue::Known(None),
            Some(target.position_raw()),
        );
    }
    let state = target.collision.state_flags_at_0x08;
    match follow_beacons_tracked_target_from_state(
        state,
        target.position_raw(),
        target.velocity_raw(),
    ) {
        Ok((status, tracked)) => (Ok(status), tracked, Some(target.position_raw())),
        Err(()) => (
            Err(()),
            RetailRuntimeValue::Unresolved,
            Some(target.position_raw()),
        ),
    }
}

fn follow_beacons_tracked_target_from_state(
    state: crate::entity_collision_state::RetailStateWord,
    position_raw: [i16; 3],
    velocity_raw: [i16; 3],
) -> Result<
    (
        FollowBeaconsFollowingTargetRuntimeState,
        RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    ),
    (),
> {
    let snapshot = CommonMoverTrackedTargetSnapshot {
        state_flags: state,
        position_raw,
        velocity_raw,
    };
    match state.masked(DYING_STATE_BIT) {
        RetailRuntimeValue::Unresolved => Err(()),
        RetailRuntimeValue::Known(bits) if bits != 0 => Ok((
            FollowBeaconsFollowingTargetRuntimeState::Dying,
            RetailRuntimeValue::Known(Some(snapshot)),
        )),
        RetailRuntimeValue::Known(_) if state.known_value_bits() != 0 => Ok((
            FollowBeaconsFollowingTargetRuntimeState::Live,
            RetailRuntimeValue::Known(Some(snapshot)),
        )),
        RetailRuntimeValue::Known(_) if state.known_mask() == u32::MAX => Ok((
            FollowBeaconsFollowingTargetRuntimeState::Inactive,
            RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: crate::entity_collision_state::RetailStateWord::exact(0),
                position_raw,
                velocity_raw,
            })),
        )),
        RetailRuntimeValue::Known(_) => Ok((
            FollowBeaconsFollowingTargetRuntimeState::Live,
            RetailRuntimeValue::Unresolved,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_collision_state::{
        RetailStateWord, BODY_BASIS_REBUILT_STATE_BIT, SURFACE_STATE_MASK,
    };

    #[test]
    fn constructor_surface_unknown_still_supplies_a_live_prelude_snapshot() {
        let mut state = RetailStateWord::exact(0x0607_8805);
        state.invalidate(SURFACE_STATE_MASK);
        state.overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
        let (status, tracked) = follow_beacons_tracked_target_from_state(state, [1, 2, 3], [0; 3])
            .expect("dying known");
        assert_eq!(status, FollowBeaconsFollowingTargetRuntimeState::Live);
        let RetailRuntimeValue::Known(Some(snapshot)) = tracked else {
            panic!("live constructor bits must be enough for FUN_00401430");
        };
        assert_eq!(snapshot.state_flags, state);
        assert_eq!(
            snapshot.state_flags.masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(snapshot.position_raw, [1, 2, 3]);
    }

    #[test]
    fn unknown_nonzero_word_stays_unresolved() {
        let state = RetailStateWord::from_known_bits(0, !SURFACE_STATE_MASK);
        let (status, tracked) =
            follow_beacons_tracked_target_from_state(state, [0; 3], [0; 3]).expect("dying known");
        assert_eq!(status, FollowBeaconsFollowingTargetRuntimeState::Live);
        assert_eq!(tracked, RetailRuntimeValue::Unresolved);
    }
}
