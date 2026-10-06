//! One-owner production bridge for Main Base Type-9 Exploding Person tasks.
//!
//! Class 14 publishes the shared `FUN_00402BA0` retarget task only after its
//! constructor has consumed the initializer's one RNG word. This bridge starts
//! at that published-task boundary: it never replays constructor RNG, visits
//! the exact Primary wrapper through scheduler before/callback/after-unwind
//! phases, and atomically persists the proven Type-9 D -> I -> A -> B mover.
//!
//! A post-unwind owner transition can block after elapsed time, retarget RNG,
//! and mover state have already committed. The retained owner therefore has a
//! distinct pending-transition stage. Retrying that stage calls only the class
//! 14 terminal adapter; it cannot replay the callback prefix.
//!
//! This module deliberately stops at the published task boundary. The
//! production live-list adapter in `main_base_type9_actor_production` composes
//! this owner with the enclosing common-scheduler prefix, `FUN_0040E870`, and
//! master motion. Keeping the task transaction independently testable retains
//! its post-unwind retry contract without allowing production to omit the
//! outer basis/surface/motion writes.

use std::num::NonZeroU64;

use v2k_formats::terrain::TerrainGrid;

use crate::actor_animation::ActorAnimationController;
use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::actor_task_owner::{
    ActorTaskOwner, ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags,
};
use crate::common_mover::actor_abdi::ActorAbdiAnimationPolicy;
use crate::common_mover::type9::{
    advance_ordinary_type9_common_mover_with_animation_policy, OrdinaryType9FrameBlock,
    OrdinaryType9FrameRequest, OrdinaryType9FrameState, OrdinaryType9Topology,
    OrdinaryType9TopologyError,
};
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::common_mover::type9_transaction::{
    OrdinaryType9Action, OrdinaryType9AppliedState, OrdinaryType9ExternalBlock,
    OrdinaryType9FrameLease, OrdinaryType9Poll, OrdinaryType9Resume, OrdinaryType9Transaction,
    OrdinaryType9TransactionId,
};
use crate::common_mover::SubAPropulsionRuntime;
use crate::entity::{
    binary_angle_to_radians, radians_to_binary_angle, raw_position_world, world_position_raw,
    Entity, EntityManager,
};
use crate::entity_collision_state::{
    RetailRuntimeValue, DEFERRED_DESTROY_PENDING_STATE_BIT, DYING_STATE_BIT,
};
use crate::main_base_abort::MainBaseAbortActorLease;
use crate::main_base_type9_abort::{
    exact_level_one_type9_metadata, exploding_person_terminal_context,
    MainBaseType9DeathComponentRuntime, MainBaseType9ExplodingTaskLease,
    MainBaseType9TerminalBlock, MainBaseType9TerminalOutcome, LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
    LEVEL_ONE_TYPE9_ENTITY_TYPE, LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS,
    LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR,
};
use crate::retail_clock::RETAIL_FRAME_DELTA_MAX_US;
use crate::shared_retarget_mover::{
    shared_retarget_after_unwind, SharedRetarget, SharedRetargetCallbackPrefix,
    SharedRetargetCallbackStage, SharedRetargetLifetimePrefix, SharedRetargetPostUnwind,
    SharedRetargetTransitionRequest,
};
use crate::wander_near_location::{WanderNearCommonMoverReturn, WanderNearPrivateState};

/// Complete frame inputs not retained on the actor allocation.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MainBaseType9ExplodingProductionFrame<'a> {
    /// Retail task mode: DCA0 supplies zero, E870 supplies one.
    pub scheduler_mode: i32,
    pub terrain: &'a TerrainGrid,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
}

/// Evidence/runtime boundary which preserves the current owner for a later
/// manager pass. A callback-consuming block must never be retried in the same
/// pass even though its owner is returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType9ExplodingProductionBlock {
    ElapsedExceedsRetailCap {
        actual: u32,
    },
    GlobalElapsedExceedsRetailCap {
        actual: u32,
    },
    PhysicalBodyBasisUnresolved,
    TypeMetadataUnavailable,
    TypeMetadataMismatch,
    Topology(OrdinaryType9TopologyError),
    CurrentBehaviorContextUnresolved,
    DyingStateUnresolved,
    DeferredDestroyStateUnresolved,
    TaskAlreadyInCallback {
        visit: ActorTaskVisit,
    },
    SubARuntimeUnresolved,
    SubATargetSpeedUnresolved,
    ActorAnimationRuntimeUnresolved,
    TaskFramePreflight {
        visit: ActorTaskVisit,
        reason: OrdinaryType9FrameBlock,
    },
    FrameBlocked {
        visit: ActorTaskVisit,
        committed_prefix: SharedRetargetCallbackPrefix,
        reason: OrdinaryType9FrameBlock,
    },
    CommitBlocked {
        visit: ActorTaskVisit,
        committed_prefix: SharedRetargetCallbackPrefix,
        reason: OrdinaryType9ExternalBlock,
    },
    TerminalTransition(MainBaseType9TerminalBlock),
    /// A complete live-list owner never permits a task-only pending transition:
    /// every transition prerequisite is closed before scheduler/task RNG.
    PendingTransitionOutsideOriginalFrame,
    RemoteSchedulerOwnerUnsupported,
    SchedulerCallbackDisabled,
    DetailedViewOwnerUnsupported,
    OwnerTransitionSuppressed,
    UnexpectedRelationAttachment {
        actual: Option<u32>,
    },
    OuterOwnerStateUnresolved,
    SchedulerStateUnresolved,
    AnimationOffsetUnresolved,
    UnexpectedCallbackMass {
        actual: u16,
    },
    ActiveModelUnavailable,
    ActiveModelMismatch,
    SurfaceLifetimeTimerUnresolved,
    SurfaceLifetimeInvariantMismatch {
        timer_ms: u32,
        task_elapsed_ms: u32,
    },
    SurfaceLifetimeEscapesClass14 {
        maximum_timer_ms: u32,
    },
    OuterOwner(crate::common_mover::type9_owner::OrdinaryType9OwnerBlock),
}

/// Stale or terminal custody which cannot authorize another callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType9ExplodingProductionDrop {
    EntityUnavailable,
    ActorLeaseMismatch {
        expected: MainBaseAbortActorLease,
        actual: MainBaseAbortActorLease,
    },
    NotFreshNewGameFirstWorld,
    EntityInactive,
    WrongEntityType {
        actual: u32,
    },
    UnexpectedCapabilityFlags {
        actual: u32,
    },
    CurrentBehaviorContextMismatch,
    DyingStateCleared,
    DeferredDestroyAlreadyPending,
    DeferredDestroyAlreadyQueued,
    PrimaryTaskMissingOrReplaced,
    WrongTaskFamily,
    TaskLifetimeMismatch {
        actual: u32,
    },
    TrackedEntityHandleMismatch {
        actual: u32,
    },
    AdditionalPublishedTask {
        slot: ActorTaskSlot,
    },
    TaskWrapperNotRunnable {
        visit: ActorTaskVisit,
    },
    DeathComponentRuntimeUnavailable,
    SubARuntimeUnavailable,
    ActorAnimationRuntimeUnavailable,
    ActorAnimationDescriptorMismatch,
    ActorAnimationSpecialStateMismatch,
    ExpectedVisitNotTicked,
    TerminalTransition(MainBaseType9TerminalBlock),
}

/// Receipt-free observation suitable for a later heterogeneous scheduler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MainBaseType9ExplodingProductionOutcome {
    /// The common `FUN_00412DA0` callback gate retained the task without
    /// entering `FUN_0040E870` this visit.
    SchedulerWaiting {
        entity_id: u32,
    },
    Continuing {
        entity_id: u32,
        visit: ActorTaskVisit,
        committed_prefix: SharedRetargetCallbackPrefix,
        mover_return: WanderNearCommonMoverReturn,
    },
    /// The task completed its frame, but a known-set entity `0x1000` bit
    /// suppressed only this frame's owner transition. Retail runs the whole
    /// callback again on the next manager pass.
    TransitionSuppressed {
        entity_id: u32,
        request: SharedRetargetTransitionRequest,
    },
    Terminal(MainBaseType9TerminalOutcome),
    Blocked {
        entity_id: u32,
        reason: MainBaseType9ExplodingProductionBlock,
    },
    Dropped {
        entity_id: u32,
        reason: MainBaseType9ExplodingProductionDrop,
    },
}

/// Private production custody for one published class-14 task.
///
/// The type is deliberately neither `Copy` nor `Clone`. Running custody owns
/// the next unique detached-mover transaction id. Pending-transition custody
/// owns the exact post-unwind request and can only retry terminal completion.
#[derive(Debug, PartialEq, Eq)]
pub struct MainBaseType9ExplodingProductionOwner {
    task_lease: MainBaseType9ExplodingTaskLease,
    stage: MainBaseType9ExplodingProductionStage,
}

#[derive(Debug, PartialEq, Eq)]
enum MainBaseType9ExplodingProductionStage {
    Running {
        next_transaction_id: NonZeroU64,
    },
    PendingTransition {
        request: SharedRetargetTransitionRequest,
        next_transaction_id: NonZeroU64,
    },
}

impl MainBaseType9ExplodingProductionOwner {
    pub(crate) fn adopt(task_lease: MainBaseType9ExplodingTaskLease) -> Self {
        Self {
            task_lease,
            stage: MainBaseType9ExplodingProductionStage::Running {
                next_transaction_id: NonZeroU64::new(1).expect("one is nonzero"),
            },
        }
    }

    pub(crate) const fn entity_id(&self) -> u32 {
        self.task_lease.actor().entity_id
    }

    pub(crate) const fn task_lease(&self) -> MainBaseType9ExplodingTaskLease {
        self.task_lease
    }

    pub(crate) fn completed_hit_boundary(&self, manager: &EntityManager) -> bool {
        matches!(
            self.stage,
            MainBaseType9ExplodingProductionStage::Running { .. }
        ) && crate::ordinary_type9_outer_tail::outer_tail_class14_exploding_task_lease(
            manager,
            self.task_lease.actor(),
        ) == Some(self.task_lease)
    }

    /// Duplicate linear custody only for the isolated Main Base abort
    /// transaction.  The enclosing scheduler fork prevents either copy from
    /// being ticked until one complete world is selected at commit.
    pub(crate) const fn fork_for_main_base_abort_transaction(&self) -> Self {
        let stage = match &self.stage {
            MainBaseType9ExplodingProductionStage::Running {
                next_transaction_id,
            } => MainBaseType9ExplodingProductionStage::Running {
                next_transaction_id: *next_transaction_id,
            },
            MainBaseType9ExplodingProductionStage::PendingTransition {
                request,
                next_transaction_id,
            } => MainBaseType9ExplodingProductionStage::PendingTransition {
                request: *request,
                next_transaction_id: *next_transaction_id,
            },
        };
        Self {
            task_lease: self.task_lease,
            stage,
        }
    }

    #[cfg(test)]
    const fn pending_transition(&self) -> Option<SharedRetargetTransitionRequest> {
        match self.stage {
            MainBaseType9ExplodingProductionStage::Running { .. } => None,
            MainBaseType9ExplodingProductionStage::PendingTransition { request, .. } => {
                Some(request)
            }
        }
    }
}

/// One exact owner visit selected by an outer live-list scheduler.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MainBaseType9ExplodingOwnerTick {
    pub outcome: MainBaseType9ExplodingProductionOutcome,
    pub retained_owner: Option<MainBaseType9ExplodingProductionOwner>,
}

/// Tick one adopted Exploding Person owner without traversing sibling actors.
///
/// Running owners execute one phased task visit. Pending-transition owners
/// skip elapsed accounting, RNG, and movement and retry only the terminal
/// behavior-owner operation. Deferred cleanup remains a later manager phase.
pub(crate) fn tick_main_base_type9_exploding_owner(
    manager: &mut EntityManager,
    owner: MainBaseType9ExplodingProductionOwner,
    frame: MainBaseType9ExplodingProductionFrame<'_>,
    next_shared_random: &mut impl FnMut() -> u32,
) -> MainBaseType9ExplodingOwnerTick {
    let MainBaseType9ExplodingProductionOwner { task_lease, stage } = owner;
    match stage {
        MainBaseType9ExplodingProductionStage::PendingTransition {
            request,
            next_transaction_id,
        } => finish_pending_transition(manager, task_lease, request, next_transaction_id),
        MainBaseType9ExplodingProductionStage::Running {
            next_transaction_id,
        } => {
            let successor_transaction_id =
                NonZeroU64::new(next_transaction_id.get().wrapping_add(1))
                    .unwrap_or(NonZeroU64::MIN);
            match tick_running_owner(
                manager,
                task_lease,
                OrdinaryType9TransactionId::new(next_transaction_id),
                frame,
                next_shared_random,
            ) {
                RunningOwnerResult::Continuing {
                    visit,
                    committed_prefix,
                    mover_return,
                } => MainBaseType9ExplodingOwnerTick {
                    outcome: MainBaseType9ExplodingProductionOutcome::Continuing {
                        entity_id: task_lease.actor().entity_id,
                        visit,
                        committed_prefix,
                        mover_return,
                    },
                    retained_owner: Some(MainBaseType9ExplodingProductionOwner {
                        task_lease,
                        stage: MainBaseType9ExplodingProductionStage::Running {
                            next_transaction_id: successor_transaction_id,
                        },
                    }),
                },
                RunningOwnerResult::Transition(request) => finish_pending_transition(
                    manager,
                    task_lease,
                    request,
                    successor_transaction_id,
                ),
                RunningOwnerResult::Blocked { reason, consumed } => {
                    MainBaseType9ExplodingOwnerTick {
                        outcome: MainBaseType9ExplodingProductionOutcome::Blocked {
                            entity_id: task_lease.actor().entity_id,
                            reason,
                        },
                        retained_owner: Some(MainBaseType9ExplodingProductionOwner {
                            task_lease,
                            stage: MainBaseType9ExplodingProductionStage::Running {
                                next_transaction_id: if consumed {
                                    successor_transaction_id
                                } else {
                                    next_transaction_id
                                },
                            },
                        }),
                    }
                }
                RunningOwnerResult::Dropped(reason) => MainBaseType9ExplodingOwnerTick {
                    outcome: MainBaseType9ExplodingProductionOutcome::Dropped {
                        entity_id: task_lease.actor().entity_id,
                        reason,
                    },
                    retained_owner: None,
                },
            }
        }
    }
}

fn finish_pending_transition(
    manager: &mut EntityManager,
    task_lease: MainBaseType9ExplodingTaskLease,
    request: SharedRetargetTransitionRequest,
    next_transaction_id: NonZeroU64,
) -> MainBaseType9ExplodingOwnerTick {
    let entity_id = task_lease.actor().entity_id;
    match manager.complete_main_base_abort_type9_exploding_transition(task_lease, request) {
        Ok(outcome) => MainBaseType9ExplodingOwnerTick {
            outcome: MainBaseType9ExplodingProductionOutcome::Terminal(outcome),
            retained_owner: None,
        },
        Err(MainBaseType9TerminalBlock::OwnerTransitionSuppressed) => {
            MainBaseType9ExplodingOwnerTick {
                outcome: MainBaseType9ExplodingProductionOutcome::TransitionSuppressed {
                    entity_id,
                    request,
                },
                retained_owner: Some(MainBaseType9ExplodingProductionOwner {
                    task_lease,
                    stage: MainBaseType9ExplodingProductionStage::Running {
                        next_transaction_id,
                    },
                }),
            }
        }
        Err(reason) if terminal_block_retains_pending_transition(reason) => {
            MainBaseType9ExplodingOwnerTick {
                outcome: MainBaseType9ExplodingProductionOutcome::Blocked {
                    entity_id,
                    reason: MainBaseType9ExplodingProductionBlock::TerminalTransition(reason),
                },
                retained_owner: Some(MainBaseType9ExplodingProductionOwner {
                    task_lease,
                    stage: MainBaseType9ExplodingProductionStage::PendingTransition {
                        request,
                        next_transaction_id,
                    },
                }),
            }
        }
        Err(reason) => MainBaseType9ExplodingOwnerTick {
            outcome: MainBaseType9ExplodingProductionOutcome::Dropped {
                entity_id,
                reason: MainBaseType9ExplodingProductionDrop::TerminalTransition(reason),
            },
            retained_owner: None,
        },
    }
}

const fn terminal_block_retains_pending_transition(reason: MainBaseType9TerminalBlock) -> bool {
    matches!(
        reason,
        MainBaseType9TerminalBlock::TaskStillInCallback
            | MainBaseType9TerminalBlock::OwnerTransitionSuppressionStateUnresolved
            | MainBaseType9TerminalBlock::DeferredDestroyStateUnresolved
    )
}

enum RunningOwnerResult {
    Continuing {
        visit: ActorTaskVisit,
        committed_prefix: SharedRetargetCallbackPrefix,
        mover_return: WanderNearCommonMoverReturn,
    },
    Transition(SharedRetargetTransitionRequest),
    Blocked {
        reason: MainBaseType9ExplodingProductionBlock,
        consumed: bool,
    },
    Dropped(MainBaseType9ExplodingProductionDrop),
}

enum OwnerValidationFailure {
    Blocked(MainBaseType9ExplodingProductionBlock),
    Dropped(MainBaseType9ExplodingProductionDrop),
}

/// Read-only result used by the enclosing common-scheduler owner to close all
/// task-local blockers before it consumes the process-shared scheduler RNG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MainBaseType9ExplodingTaskPreflight {
    pub(crate) task_elapsed_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainBaseType9ExplodingTaskPreflightFailure {
    Blocked(MainBaseType9ExplodingProductionBlock),
    Dropped(MainBaseType9ExplodingProductionDrop),
}

struct PreflightedRunningOwner {
    visit: ActorTaskVisit,
    topology: OrdinaryType9Topology,
    pre_mover_basis: Type9BodyBasis,
    task_elapsed_ms: u32,
}

/// Validate a task-only owner without mutation or process RNG consumption.
///
/// This includes a copied common-mover plan. The class-14 target handle is the
/// constructor-proven zero sentinel, so dummy target-prefix words can change
/// only copied steering values; every possible mover evidence blocker is
/// independent of those values and is therefore closed before the enclosing
/// scheduler draws from the live stream.
pub(crate) fn preflight_main_base_type9_exploding_owner(
    manager: &EntityManager,
    owner: &MainBaseType9ExplodingProductionOwner,
    frame: MainBaseType9ExplodingProductionFrame<'_>,
) -> Result<MainBaseType9ExplodingTaskPreflight, MainBaseType9ExplodingTaskPreflightFailure> {
    if matches!(
        owner.stage,
        MainBaseType9ExplodingProductionStage::PendingTransition { .. }
    ) {
        return Err(MainBaseType9ExplodingTaskPreflightFailure::Blocked(
            MainBaseType9ExplodingProductionBlock::PendingTransitionOutsideOriginalFrame,
        ));
    }
    preflight_running_owner(manager, owner.task_lease, frame, true).map(|preflight| {
        MainBaseType9ExplodingTaskPreflight {
            task_elapsed_ms: preflight.task_elapsed_ms,
        }
    })
}

fn tick_running_owner(
    manager: &mut EntityManager,
    task_lease: MainBaseType9ExplodingTaskLease,
    transaction_id: OrdinaryType9TransactionId,
    frame: MainBaseType9ExplodingProductionFrame<'_>,
    next_shared_random: &mut impl FnMut() -> u32,
) -> RunningOwnerResult {
    let entity_id = task_lease.actor().entity_id;
    let preflight = match preflight_running_owner(manager, task_lease, frame, false) {
        Ok(preflight) => preflight,
        Err(MainBaseType9ExplodingTaskPreflightFailure::Blocked(reason)) => {
            return RunningOwnerResult::Blocked {
                reason,
                consumed: false,
            }
        }
        Err(MainBaseType9ExplodingTaskPreflightFailure::Dropped(reason)) => {
            return RunningOwnerResult::Dropped(reason)
        }
    };
    let Some(entity) = manager.main_base_type9_exploding_entity_mut(entity_id) else {
        return RunningOwnerResult::Dropped(
            MainBaseType9ExplodingProductionDrop::EntityUnavailable,
        );
    };
    let PreflightedRunningOwner {
        visit,
        topology,
        pre_mover_basis,
        task_elapsed_ms: _,
    } = preflight;

    let position_raw = entity.position_raw();
    let result = {
        let Entity {
            actor_tasks,
            main_base_type9_death_component_runtime,
            sub_a_propulsion_runtime,
            actor_animation_runtime,
            heading,
            velocity,
            ..
        } = entity;
        let mut context = ExplodingFrameContext {
            entity_id,
            visit,
            topology,
            transaction_id,
            frame,
            pre_mover_basis,
            position_raw,
            death_runtime: main_base_type9_death_component_runtime,
            sub_a: sub_a_propulsion_runtime,
            animation: actor_animation_runtime,
            heading,
            velocity,
            next_random: next_shared_random,
        };
        let Some(lifetime) = actor_tasks
            .begin_exact_visit_with(visit, |state| context.before_callback(state, visit))
        else {
            return RunningOwnerResult::Dropped(
                MainBaseType9ExplodingProductionDrop::ExpectedVisitNotTicked,
            );
        };
        let callback = context.callback(actor_tasks, visit);
        actor_tasks
            .finish_exact_visit(visit)
            .then(|| context.after_unwind(visit, lifetime, callback))
    };

    match result {
        Some(ExplodingFrameResult::Continuing {
            visit,
            committed_prefix,
            mover_return,
        }) => RunningOwnerResult::Continuing {
            visit,
            committed_prefix,
            mover_return,
        },
        Some(ExplodingFrameResult::Transition(request)) => RunningOwnerResult::Transition(request),
        Some(ExplodingFrameResult::FrameBlocked {
            visit,
            committed_prefix,
            reason,
        }) => RunningOwnerResult::Blocked {
            reason: MainBaseType9ExplodingProductionBlock::FrameBlocked {
                visit,
                committed_prefix,
                reason,
            },
            consumed: true,
        },
        Some(ExplodingFrameResult::CommitBlocked {
            visit,
            committed_prefix,
            reason,
        }) => RunningOwnerResult::Blocked {
            reason: MainBaseType9ExplodingProductionBlock::CommitBlocked {
                visit,
                committed_prefix,
                reason,
            },
            consumed: true,
        },
        None => RunningOwnerResult::Dropped(
            MainBaseType9ExplodingProductionDrop::ExpectedVisitNotTicked,
        ),
    }
}

fn preflight_running_owner(
    manager: &EntityManager,
    task_lease: MainBaseType9ExplodingTaskLease,
    frame: MainBaseType9ExplodingProductionFrame<'_>,
    preflight_mover: bool,
) -> Result<PreflightedRunningOwner, MainBaseType9ExplodingTaskPreflightFailure> {
    let entity_id = task_lease.actor().entity_id;
    let Some(observation) = manager.main_base_abort_actor_observation(entity_id) else {
        return Err(MainBaseType9ExplodingTaskPreflightFailure::Dropped(
            MainBaseType9ExplodingProductionDrop::EntityUnavailable,
        ));
    };
    if observation.lease != task_lease.actor() {
        return Err(MainBaseType9ExplodingTaskPreflightFailure::Dropped(
            MainBaseType9ExplodingProductionDrop::ActorLeaseMismatch {
                expected: observation.lease,
                actual: task_lease.actor(),
            },
        ));
    }
    // Native Intro2's immutable birth receipt authenticates the same retail
    // ABDI row. Its outer world owner supplies the live scheduler mode and
    // preserves its own pending Sub-D origin; no Level1 reset is borrowed.
    let native_intro2 = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .is_some_and(crate::intro2_type9::intro2_type9_allocation_authenticates);
    let native_ordinary =
        crate::ordinary_type9_construction::ordinary_type9_native_allocation_authenticates(
            manager, entity_id,
        );
    if manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .is_some_and(|entity| entity.ordinary_type9_native_receipt.is_some())
        && !native_ordinary
    {
        return Err(MainBaseType9ExplodingTaskPreflightFailure::Dropped(
            MainBaseType9ExplodingProductionDrop::NotFreshNewGameFirstWorld,
        ));
    }
    if !manager.is_fresh_new_game_first_world() && !native_intro2 && !native_ordinary {
        return Err(MainBaseType9ExplodingTaskPreflightFailure::Dropped(
            MainBaseType9ExplodingProductionDrop::NotFreshNewGameFirstWorld,
        ));
    }
    if observation.entity_type != LEVEL_ONE_TYPE9_ENTITY_TYPE {
        return Err(MainBaseType9ExplodingTaskPreflightFailure::Dropped(
            MainBaseType9ExplodingProductionDrop::WrongEntityType {
                actual: observation.entity_type,
            },
        ));
    }
    if observation.capability_flags != LEVEL_ONE_TYPE9_CAPABILITY_FLAGS {
        return Err(MainBaseType9ExplodingTaskPreflightFailure::Dropped(
            MainBaseType9ExplodingProductionDrop::UnexpectedCapabilityFlags {
                actual: observation.capability_flags,
            },
        ));
    }
    if manager
        .pending_actor_deferred_destroy_ids()
        .contains(&entity_id)
    {
        return Err(MainBaseType9ExplodingTaskPreflightFailure::Dropped(
            MainBaseType9ExplodingProductionDrop::DeferredDestroyAlreadyQueued,
        ));
    }

    let Some(metadata) = manager.type_runtime_metadata(observation.entity_type) else {
        return Err(MainBaseType9ExplodingTaskPreflightFailure::Blocked(
            MainBaseType9ExplodingProductionBlock::TypeMetadataUnavailable,
        ));
    };
    if !exact_level_one_type9_metadata(metadata) {
        return Err(MainBaseType9ExplodingTaskPreflightFailure::Blocked(
            MainBaseType9ExplodingProductionBlock::TypeMetadataMismatch,
        ));
    }
    let topology =
        match OrdinaryType9Topology::from_metadata(observation.entity_type as u16, metadata) {
            Ok(topology) => topology,
            Err(reason) => {
                return Err(MainBaseType9ExplodingTaskPreflightFailure::Blocked(
                    MainBaseType9ExplodingProductionBlock::Topology(reason),
                ))
            }
        };

    let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return Err(MainBaseType9ExplodingTaskPreflightFailure::Dropped(
            MainBaseType9ExplodingProductionDrop::EntityUnavailable,
        ));
    };
    let visit = match validate_running_owner(entity, task_lease) {
        Ok(visit) => visit,
        Err(OwnerValidationFailure::Blocked(reason)) => {
            return Err(MainBaseType9ExplodingTaskPreflightFailure::Blocked(reason))
        }
        Err(OwnerValidationFailure::Dropped(reason)) => {
            return Err(MainBaseType9ExplodingTaskPreflightFailure::Dropped(reason))
        }
    };

    if frame.elapsed_micros > RETAIL_FRAME_DELTA_MAX_US {
        return Err(MainBaseType9ExplodingTaskPreflightFailure::Blocked(
            MainBaseType9ExplodingProductionBlock::ElapsedExceedsRetailCap {
                actual: frame.elapsed_micros,
            },
        ));
    }
    if frame.global_elapsed_micros > RETAIL_FRAME_DELTA_MAX_US {
        return Err(MainBaseType9ExplodingTaskPreflightFailure::Blocked(
            MainBaseType9ExplodingProductionBlock::GlobalElapsedExceedsRetailCap {
                actual: frame.global_elapsed_micros,
            },
        ));
    }
    // Retail `FUN_00401430` copies entity `+0x0C..+0x2C` at callback entry.
    // Snapshot the allocation-owned matrix only after actor/task validation;
    // never reconstruct it here from the independently retained angle words.
    let RetailRuntimeValue::Known(pre_mover_basis) = entity.physical_body_basis_q31 else {
        return Err(MainBaseType9ExplodingTaskPreflightFailure::Blocked(
            MainBaseType9ExplodingProductionBlock::PhysicalBodyBasisUnresolved,
        ));
    };
    let Some(ActorTaskRuntime::SharedRetarget(task)) = entity.actor_tasks.task_state(visit.task_id)
    else {
        unreachable!("the read-only owner validation authenticated the task family")
    };
    let task_elapsed_ms = task.elapsed_ms();
    if preflight_mover {
        let mut staged_task = *task;
        let callback_stage = staged_task.stage_callback(entity.position_raw(), || 0);
        let death = entity
            .main_base_type9_death_component_runtime
            .expect("the read-only owner validation authenticated component custody");
        let RetailRuntimeValue::Known(Some(sub_a_runtime)) = entity.sub_a_propulsion_runtime else {
            unreachable!("the read-only owner validation authenticated Sub-A")
        };
        let RetailRuntimeValue::Known(Some(actor_animation)) = entity.actor_animation_runtime
        else {
            unreachable!("the read-only owner validation authenticated Sub-I")
        };
        let mut copied_frame = OrdinaryType9FrameState {
            sub_d_frame_owner: death.components.sub_d_frame_owner,
            sub_d_runtime: death.components.sub_d_runtime,
            sub_a_runtime,
            actor_animation,
        };
        if let Err(reason) = advance_ordinary_type9_common_mover_with_animation_policy(
            &mut copied_frame,
            OrdinaryType9FrameRequest {
                topology,
                wander: callback_stage.private_state(),
                tracked_target: RetailRuntimeValue::Known(None),
                terrain: frame.terrain,
                position_raw: entity.position_raw(),
                pre_mover_right_q31: pre_mover_basis.lateral,
                pre_mover_forward_q31: pre_mover_basis.forward,
                heading_raw: radians_to_binary_angle(entity.heading),
                velocity_raw: world_position_raw(entity.velocity),
                elapsed_micros: frame.elapsed_micros,
                global_elapsed_micros: frame.global_elapsed_micros,
                scheduler_mode: frame.scheduler_mode,
            },
            ActorAbdiAnimationPolicy::ExplodingPersonSpecial,
            || 0,
        ) {
            return Err(MainBaseType9ExplodingTaskPreflightFailure::Blocked(
                MainBaseType9ExplodingProductionBlock::TaskFramePreflight { visit, reason },
            ));
        }
    }

    Ok(PreflightedRunningOwner {
        visit,
        topology,
        pre_mover_basis,
        task_elapsed_ms,
    })
}

fn validate_running_owner(
    entity: &Entity,
    task_lease: MainBaseType9ExplodingTaskLease,
) -> Result<ActorTaskVisit, OwnerValidationFailure> {
    if !entity.active {
        return Err(OwnerValidationFailure::Dropped(
            MainBaseType9ExplodingProductionDrop::EntityInactive,
        ));
    }
    match entity.current_behavior_context {
        RetailRuntimeValue::Known(Some(context))
            if exploding_person_terminal_context(context).is_some() => {}
        RetailRuntimeValue::Unresolved => {
            return Err(OwnerValidationFailure::Blocked(
                MainBaseType9ExplodingProductionBlock::CurrentBehaviorContextUnresolved,
            ))
        }
        RetailRuntimeValue::Known(Some(_)) | RetailRuntimeValue::Known(None) => {
            return Err(OwnerValidationFailure::Dropped(
                MainBaseType9ExplodingProductionDrop::CurrentBehaviorContextMismatch,
            ))
        }
    }
    match entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT) {
        RetailRuntimeValue::Known(DYING_STATE_BIT) => {}
        RetailRuntimeValue::Known(_) => {
            return Err(OwnerValidationFailure::Dropped(
                MainBaseType9ExplodingProductionDrop::DyingStateCleared,
            ))
        }
        RetailRuntimeValue::Unresolved => {
            return Err(OwnerValidationFailure::Blocked(
                MainBaseType9ExplodingProductionBlock::DyingStateUnresolved,
            ))
        }
    }
    match entity
        .collision
        .state_flags_at_0x08
        .masked(DEFERRED_DESTROY_PENDING_STATE_BIT)
    {
        RetailRuntimeValue::Known(0) => {}
        RetailRuntimeValue::Known(_) => {
            return Err(OwnerValidationFailure::Dropped(
                MainBaseType9ExplodingProductionDrop::DeferredDestroyAlreadyPending,
            ))
        }
        RetailRuntimeValue::Unresolved => {
            return Err(OwnerValidationFailure::Blocked(
                MainBaseType9ExplodingProductionBlock::DeferredDestroyStateUnresolved,
            ))
        }
    }

    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id: task_lease.task_id(),
    };
    if entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) != Some(visit.task_id) {
        return Err(OwnerValidationFailure::Dropped(
            MainBaseType9ExplodingProductionDrop::PrimaryTaskMissingOrReplaced,
        ));
    }
    let Some(ActorTaskRuntime::SharedRetarget(task)) = entity.actor_tasks.task_state(visit.task_id)
    else {
        return Err(OwnerValidationFailure::Dropped(
            MainBaseType9ExplodingProductionDrop::WrongTaskFamily,
        ));
    };
    if task.lifetime_ms() != LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS {
        return Err(OwnerValidationFailure::Dropped(
            MainBaseType9ExplodingProductionDrop::TaskLifetimeMismatch {
                actual: task.lifetime_ms(),
            },
        ));
    }
    if task.private_state().tracked_entity_handle != 0 {
        return Err(OwnerValidationFailure::Dropped(
            MainBaseType9ExplodingProductionDrop::TrackedEntityHandleMismatch {
                actual: task.private_state().tracked_entity_handle,
            },
        ));
    }
    for slot in [ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary] {
        if entity.actor_tasks.task_in_slot(slot).is_some() {
            return Err(OwnerValidationFailure::Dropped(
                MainBaseType9ExplodingProductionDrop::AdditionalPublishedTask { slot },
            ));
        }
    }
    match entity.actor_tasks.wrapper_flags(visit.task_id) {
        Some(ActorTaskWrapperFlags {
            alive: true,
            in_callback: false,
        }) => {}
        Some(ActorTaskWrapperFlags {
            alive: true,
            in_callback: true,
        }) => {
            return Err(OwnerValidationFailure::Blocked(
                MainBaseType9ExplodingProductionBlock::TaskAlreadyInCallback { visit },
            ))
        }
        Some(_) | None => {
            return Err(OwnerValidationFailure::Dropped(
                MainBaseType9ExplodingProductionDrop::TaskWrapperNotRunnable { visit },
            ))
        }
    }
    if entity.main_base_type9_death_component_runtime.is_none() {
        return Err(OwnerValidationFailure::Dropped(
            MainBaseType9ExplodingProductionDrop::DeathComponentRuntimeUnavailable,
        ));
    }
    match entity.sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(sub_a)) => {
            if sub_a.target_speed_raw() == RetailRuntimeValue::Unresolved {
                return Err(OwnerValidationFailure::Blocked(
                    MainBaseType9ExplodingProductionBlock::SubATargetSpeedUnresolved,
                ));
            }
        }
        RetailRuntimeValue::Known(None) => {
            return Err(OwnerValidationFailure::Dropped(
                MainBaseType9ExplodingProductionDrop::SubARuntimeUnavailable,
            ))
        }
        RetailRuntimeValue::Unresolved => {
            return Err(OwnerValidationFailure::Blocked(
                MainBaseType9ExplodingProductionBlock::SubARuntimeUnresolved,
            ))
        }
    }
    match entity.actor_animation_runtime {
        RetailRuntimeValue::Known(Some(animation))
            if animation.descriptor() == LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR
                && animation.special_mode()
                && animation.linked_handle().is_none() => {}
        RetailRuntimeValue::Known(Some(animation))
            if animation.descriptor() == LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR =>
        {
            return Err(OwnerValidationFailure::Dropped(
                MainBaseType9ExplodingProductionDrop::ActorAnimationSpecialStateMismatch,
            ))
        }
        RetailRuntimeValue::Known(Some(_)) => {
            return Err(OwnerValidationFailure::Dropped(
                MainBaseType9ExplodingProductionDrop::ActorAnimationDescriptorMismatch,
            ))
        }
        RetailRuntimeValue::Known(None) => {
            return Err(OwnerValidationFailure::Dropped(
                MainBaseType9ExplodingProductionDrop::ActorAnimationRuntimeUnavailable,
            ))
        }
        RetailRuntimeValue::Unresolved => {
            return Err(OwnerValidationFailure::Blocked(
                MainBaseType9ExplodingProductionBlock::ActorAnimationRuntimeUnresolved,
            ))
        }
    }
    Ok(visit)
}

#[derive(Debug)]
struct ExplodingCallback {
    retarget: SharedRetarget,
    mover: Result<WanderNearCommonMoverReturn, ExplodingMoverError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExplodingMoverError {
    FrameBlocked(OrdinaryType9FrameBlock),
    CommitBlocked(OrdinaryType9ExternalBlock),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExplodingFrameResult {
    Continuing {
        visit: ActorTaskVisit,
        committed_prefix: SharedRetargetCallbackPrefix,
        mover_return: WanderNearCommonMoverReturn,
    },
    Transition(SharedRetargetTransitionRequest),
    FrameBlocked {
        visit: ActorTaskVisit,
        committed_prefix: SharedRetargetCallbackPrefix,
        reason: OrdinaryType9FrameBlock,
    },
    CommitBlocked {
        visit: ActorTaskVisit,
        committed_prefix: SharedRetargetCallbackPrefix,
        reason: OrdinaryType9ExternalBlock,
    },
}

struct ExplodingFrameContext<'entity, 'frame, Random> {
    entity_id: u32,
    visit: ActorTaskVisit,
    topology: OrdinaryType9Topology,
    transaction_id: OrdinaryType9TransactionId,
    frame: MainBaseType9ExplodingProductionFrame<'frame>,
    pre_mover_basis: Type9BodyBasis,
    position_raw: [i16; 3],
    death_runtime: &'entity mut Option<MainBaseType9DeathComponentRuntime>,
    sub_a: &'entity mut RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
    animation: &'entity mut RetailRuntimeValue<Option<ActorAnimationController>>,
    heading: &'entity mut f32,
    velocity: &'entity mut [f32; 3],
    next_random: &'entity mut Random,
}

impl<Random: FnMut() -> u32> ExplodingFrameContext<'_, '_, Random> {
    fn before_callback(
        &mut self,
        state: &mut ActorTaskRuntime,
        visit: ActorTaskVisit,
    ) -> SharedRetargetLifetimePrefix {
        debug_assert_eq!(visit, self.visit);
        let ActorTaskRuntime::SharedRetarget(state) = state else {
            unreachable!("the exact shared-retarget visit was prevalidated")
        };
        state.before_callback(self.frame.elapsed_micros)
    }

    fn callback(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
    ) -> ExplodingCallback {
        debug_assert_eq!(visit, self.visit);
        let mut stage = {
            let state = owner
                .exact_callback_state_mut(visit)
                .expect("the executing shared-retarget wrapper must retain its state");
            let ActorTaskRuntime::SharedRetarget(state) = state else {
                unreachable!("the exact shared-retarget visit was prevalidated")
            };
            state.stage_callback(self.position_raw, || (self.next_random)() as u16)
        };
        let retarget = stage.retarget();
        let mover = self.run_mover(owner, visit, &mut stage);
        ExplodingCallback { retarget, mover }
    }

    fn after_unwind(
        &mut self,
        visit: ActorTaskVisit,
        lifetime: SharedRetargetLifetimePrefix,
        callback: ExplodingCallback,
    ) -> ExplodingFrameResult {
        debug_assert_eq!(visit, self.visit);
        let committed_prefix =
            SharedRetargetCallbackPrefix::from_parts(lifetime, callback.retarget);
        let mover_return = match callback.mover {
            Ok(value) => value,
            Err(ExplodingMoverError::FrameBlocked(reason)) => {
                return ExplodingFrameResult::FrameBlocked {
                    visit,
                    committed_prefix,
                    reason,
                }
            }
            Err(ExplodingMoverError::CommitBlocked(reason)) => {
                return ExplodingFrameResult::CommitBlocked {
                    visit,
                    committed_prefix,
                    reason,
                }
            }
        };
        match shared_retarget_after_unwind(visit, committed_prefix, mover_return) {
            SharedRetargetPostUnwind::Continue => ExplodingFrameResult::Continuing {
                visit,
                committed_prefix,
                mover_return,
            },
            SharedRetargetPostUnwind::Transition(request) => {
                ExplodingFrameResult::Transition(request)
            }
            SharedRetargetPostUnwind::UnresolvedCommonMover => {
                unreachable!("the exact mover returns only resolved zero or nonzero")
            }
        }
    }

    fn run_mover(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        stage: &mut SharedRetargetCallbackStage,
    ) -> Result<WanderNearCommonMoverReturn, ExplodingMoverError> {
        let initial = self.snapshot(stage.private_state());
        let mut transaction = OrdinaryType9Transaction::start_with_animation_policy(
            self.transaction_id,
            OrdinaryType9FrameLease {
                controlled_entity_id: self.entity_id,
                task_visit: visit,
            },
            initial.frame,
            OrdinaryType9FrameRequest {
                topology: self.topology,
                wander: stage.private_state(),
                tracked_target: RetailRuntimeValue::Known(None),
                terrain: self.frame.terrain,
                position_raw: self.position_raw,
                pre_mover_right_q31: self.pre_mover_basis.lateral,
                pre_mover_forward_q31: self.pre_mover_basis.forward,
                heading_raw: radians_to_binary_angle(*self.heading),
                velocity_raw: world_position_raw(*self.velocity),
                elapsed_micros: self.frame.elapsed_micros,
                global_elapsed_micros: self.frame.global_elapsed_micros,
                scheduler_mode: self.frame.scheduler_mode,
            },
            ActorAbdiAnimationPolicy::ExplodingPersonSpecial,
            || (self.next_random)(),
        );

        match transaction.poll() {
            OrdinaryType9Poll::PlanningBlocked(block) => {
                Err(ExplodingMoverError::FrameBlocked(block.reason))
            }
            OrdinaryType9Poll::ReturnedZero(_) => {
                self.commit_resolved_zero(owner, visit, *stage)?;
                Ok(WanderNearCommonMoverReturn::Zero)
            }
            OrdinaryType9Poll::Action(issued) => {
                let response = self.commit_action(owner, visit, stage, issued.action);
                let blocked_reason = match response {
                    OrdinaryType9Resume::Acknowledged { .. } => None,
                    OrdinaryType9Resume::Blocked { reason } => Some(reason),
                };
                transaction
                    .resume(issued.receipt, response)
                    .expect("the production owner resumes its own unmodified linear receipt");
                match (blocked_reason, transaction.poll()) {
                    (None, OrdinaryType9Poll::ReturnOne(_)) => {
                        Ok(WanderNearCommonMoverReturn::NonZero)
                    }
                    (Some(reason), OrdinaryType9Poll::Blocked(block)) => {
                        debug_assert_eq!(block.reason, reason);
                        Err(ExplodingMoverError::CommitBlocked(reason))
                    }
                    _ => unreachable!(
                        "acknowledged and blocked commits have exact terminal transaction states"
                    ),
                }
            }
            OrdinaryType9Poll::Awaiting
            | OrdinaryType9Poll::Blocked(_)
            | OrdinaryType9Poll::ReturnOne(_) => {
                unreachable!("a fresh transaction has exactly one initial poll state")
            }
        }
    }

    fn snapshot(&self, wander: WanderNearPrivateState) -> OrdinaryType9AppliedState {
        let death = self
            .death_runtime
            .as_ref()
            .expect("prevalidated class-14 component custody must survive the callback");
        let RetailRuntimeValue::Known(Some(sub_a_runtime)) = *self.sub_a else {
            unreachable!("prevalidated Type-9 runtime must retain Sub-A")
        };
        let RetailRuntimeValue::Known(Some(actor_animation)) = *self.animation else {
            unreachable!("prevalidated Type-9 runtime must retain Sub-I")
        };
        OrdinaryType9AppliedState {
            frame: OrdinaryType9FrameState {
                sub_d_frame_owner: death.components.sub_d_frame_owner,
                sub_d_runtime: death.components.sub_d_runtime,
                sub_a_runtime,
                actor_animation,
            },
            wander,
            heading_raw: radians_to_binary_angle(*self.heading),
            velocity_raw: world_position_raw(*self.velocity),
        }
    }

    fn commit_resolved_zero(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        stage: SharedRetargetCallbackStage,
    ) -> Result<(), ExplodingMoverError> {
        let Some(ActorTaskRuntime::SharedRetarget(task)) = owner.task_state_mut(visit.task_id)
        else {
            return Err(ExplodingMoverError::CommitBlocked(
                OrdinaryType9ExternalBlock::LeaseUnavailable,
            ));
        };
        task.commit_callback_stage(stage);
        Ok(())
    }

    fn commit_action(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        stage: &mut SharedRetargetCallbackStage,
        action: OrdinaryType9Action,
    ) -> OrdinaryType9Resume {
        let OrdinaryType9Action::CommitApplied {
            lease,
            before,
            after,
            step: _,
        } = action;
        if lease
            != (OrdinaryType9FrameLease {
                controlled_entity_id: self.entity_id,
                task_visit: self.visit,
            })
        {
            return OrdinaryType9Resume::Blocked {
                reason: OrdinaryType9ExternalBlock::LeaseUnavailable,
            };
        }
        let Some(ActorTaskRuntime::SharedRetarget(task)) = owner.task_state(visit.task_id) else {
            return OrdinaryType9Resume::Blocked {
                reason: OrdinaryType9ExternalBlock::LeaseUnavailable,
            };
        };
        if self.snapshot(task.private_state()) != before {
            return OrdinaryType9Resume::Blocked {
                reason: OrdinaryType9ExternalBlock::StateCommitUnavailable,
            };
        }

        let Some(death) = self.death_runtime.as_mut() else {
            unreachable!("the complete before-state validated class-14 component custody")
        };
        let Some(ActorTaskRuntime::SharedRetarget(task)) = owner.task_state_mut(visit.task_id)
        else {
            unreachable!("the validated shared-retarget task survives its synchronous commit")
        };
        *stage.private_state_mut() = after.wander;
        let committed_stage = *stage;
        let heading = binary_angle_to_radians(after.heading_raw);
        let velocity = raw_position_world(after.velocity_raw);

        death.components.sub_d_frame_owner = after.frame.sub_d_frame_owner;
        death.components.sub_d_runtime = after.frame.sub_d_runtime;
        *self.sub_a = RetailRuntimeValue::Known(Some(after.frame.sub_a_runtime));
        *self.animation = RetailRuntimeValue::Known(Some(after.frame.actor_animation));
        task.commit_callback_stage(committed_stage);
        *self.heading = heading;
        *self.velocity = velocity;
        OrdinaryType9Resume::Acknowledged { committed: after }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::actor_task_owner::{ActorTaskId, PreparedActorTask};
    use crate::common_mover::sub_d::{Type9SubDFrameOwner, ORDINARY_TYPE9_SUB_D};
    use crate::entity::EntityKind;
    use crate::entity_behavior::{
        BehaviorChoiceListSource, BehaviorContextRuntime, EXPLODING_PERSON_BEHAVIOR_PROGRAM,
    };
    use crate::entity_collision_state::{
        EntityInitializerSpec, EntityTypeRuntimeMetadata, RetailStateWord,
    };
    use crate::main_base_type9_abort::{
        MainBaseType9CapturedProfile, MainBaseType9DeathComponentRuntime,
        LEVEL_ONE_TYPE9_BEHAVIOR_CHOICES, LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR,
        LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY, LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS,
        LEVEL_ONE_TYPE9_DEATH_SOUND_ID, LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW,
        LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW, LEVEL_ONE_TYPE9_MASS_RAW, LEVEL_ONE_TYPE9_MODEL_ID,
        LEVEL_ONE_TYPE9_OWNER_TRANSITION_SUPPRESSION_BIT, LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR,
        LEVEL_ONE_TYPE9_SUB_B_DESCRIPTOR,
    };
    use crate::ordinary_type9_live::{
        admit_fresh_level1_ordinary_type9, FreshLevel1OrdinaryType9SpawnFacts,
    };
    use crate::shared_retarget_mover::{
        SharedRetargetTaskState, SharedRetargetTransitionReason, SharedRetargetTrigger,
    };
    use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

    /// Exact post-publication entity for sibling composite-scheduler tests.
    ///
    /// Allocation identity belongs to the manager containing this entity, so
    /// [`issue_exact_main_base_type9_exploding_owner`] must be called only
    /// after the final mixed-family manager has been assembled.
    pub(crate) fn exact_main_base_type9_exploding_owner_fixture(
        entity_id: u32,
        position_raw: [i16; 3],
    ) -> (Entity, EntityTypeRuntimeMetadata, ActorTaskId) {
        let metadata = exact_metadata();
        let mut entity = Entity::unresolved_port_entity(
            entity_id,
            EntityKind::Enemy,
            LEVEL_ONE_TYPE9_ENTITY_TYPE,
        );
        entity.authored_spawn_index = Some(9);
        entity.model_slots = [Some(LEVEL_ONE_TYPE9_MODEL_ID); 4];
        entity.model_index = Some(LEVEL_ONE_TYPE9_MODEL_ID);
        entity.mass_raw = LEVEL_ONE_TYPE9_MASS_RAW;
        entity.capability_flags = LEVEL_ONE_TYPE9_CAPABILITY_FLAGS;
        entity.collision.health_raw = RetailRuntimeValue::Known(0);
        entity.collision.death_sound_id =
            RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE9_DEATH_SOUND_ID));
        // Accepted sample 5564 after class-14 publication: generic death set
        // 0x4000 and the initializer cleared the former pair bit 0x8000.
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0x00C6_4825);
        entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        entity.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(0);
        entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        entity.collision.scheduler_unit_delta_flag_at_0xb6 = RetailRuntimeValue::Known(0);
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        entity.set_motion_raw(position_raw, [0; 3]);
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(0);
        entity.physical_body_basis_q31 = RetailRuntimeValue::Known(
            MainBaseType9CapturedProfile::for_spawn(9)
                .expect("spawn 9 has accepted pre-abort matrix evidence")
                .physical_body_basis_q31,
        );

        let program = &EXPLODING_PERSON_BEHAVIOR_PROGRAM;
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(
            BehaviorContextRuntime::named_audited(
                program,
                0,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Unresolved,
                RetailRuntimeValue::Unresolved,
                program.initial_style,
            )
            .expect("class 14 initial style is audited"),
        ));
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(1), 1, 100),
        ));
        let mut animation =
            ActorAnimationController::from_descriptor(LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR)
                .expect("the exact Sub-I descriptor binds a retained animation word");
        animation.apply_exploding_person_reset();
        entity.actor_animation_runtime = RetailRuntimeValue::Known(Some(animation));

        let components = admit_fresh_level1_ordinary_type9(FreshLevel1OrdinaryType9SpawnFacts {
            retail_first_world: true,
            authored_spawn_index: 9,
            entity_type: LEVEL_ONE_TYPE9_ENTITY_TYPE,
            active_model_slot: RetailRuntimeValue::Known(0),
            active_model: Some(LEVEL_ONE_TYPE9_MODEL_ID),
            rotation: [0; 3],
            immutable_anchor_raw_at_0x90: RetailRuntimeValue::Known(position_raw),
        })
        .expect("spawn 9 is capture-authenticated")
        .pending_initial_selection();
        entity.main_base_type9_death_component_runtime =
            Some(MainBaseType9DeathComponentRuntime { components });

        let task_id = entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                SharedRetargetTaskState::new(position_raw, LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS),
            )),
        );
        (entity, metadata, task_id)
    }

    pub(crate) fn issue_exact_main_base_type9_exploding_owner(
        manager: &EntityManager,
        entity_id: u32,
        task_id: ActorTaskId,
    ) -> MainBaseType9ExplodingProductionOwner {
        let actor = manager
            .main_base_abort_actor_observation(entity_id)
            .expect("the exact Type-9 fixture must be live");
        MainBaseType9ExplodingProductionOwner::adopt(MainBaseType9ExplodingTaskLease::issue(
            actor.lease,
            task_id,
        ))
    }

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [LEVEL_ONE_TYPE9_MODEL_ID as u16; 4],
            mass_raw: LEVEL_ONE_TYPE9_MASS_RAW,
            capability_flags: LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
            initial_health_raw: Some(LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW),
            accepted_hit_presentation_sound_id: RetailRuntimeValue::Known(Some(95)),
            death_sound_id: RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE9_DEATH_SOUND_ID)),
            constructor_sound_attachment_id: RetailRuntimeValue::Known(None),
            generic_hit_sound_id: RetailRuntimeValue::Known(None),
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR,
            )),
            sub_b_lateral_descriptor: RetailRuntimeValue::Known(Some(
                LEVEL_ONE_TYPE9_SUB_B_DESCRIPTOR,
            )),
            sub_d_steering_descriptor: RetailRuntimeValue::Known(Some(ORDINARY_TYPE9_SUB_D)),
            actor_animation_descriptor: RetailRuntimeValue::Known(Some(
                LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR,
            )),
            common_mover_topology: RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW,
                common_axis_descriptor: LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR,
                behavior_choices: LEVEL_ONE_TYPE9_BEHAVIOR_CHOICES.into(),
                behavior_rule_ref: 1,
                alternate_behavior_class_ref: u32::from(LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS),
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn owner_fixture(entity_id: u32) -> (EntityManager, MainBaseType9ExplodingProductionOwner) {
        let (entity, metadata, task_id) =
            exact_main_base_type9_exploding_owner_fixture(entity_id, [0; 3]);
        let mut type_metadata = vec![EntityTypeRuntimeMetadata::default(); 10];
        type_metadata[LEVEL_ONE_TYPE9_ENTITY_TYPE as usize] = metadata;
        let manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![entity],
            type_metadata,
            true,
        );
        let owner = issue_exact_main_base_type9_exploding_owner(&manager, entity_id, task_id);
        (manager, owner)
    }

    fn flat_terrain() -> TerrainGrid {
        TerrainGrid {
            header: [-1 << 8, 0, 0, 0, 0],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn frame(
        terrain: &TerrainGrid,
        elapsed_micros: u32,
    ) -> MainBaseType9ExplodingProductionFrame<'_> {
        MainBaseType9ExplodingProductionFrame {
            scheduler_mode: 0,
            terrain,
            elapsed_micros,
            global_elapsed_micros: elapsed_micros,
        }
    }

    fn shared_task(manager: &mut EntityManager, entity_id: u32) -> SharedRetargetTaskState {
        let entity = manager
            .entity_mut_for_test(entity_id)
            .expect("fixture entity remains live");
        let Some(ActorTaskRuntime::SharedRetarget(task)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("fixture must retain class 14's Primary task")
        };
        *task
    }

    fn set_far_target(manager: &mut EntityManager, entity_id: u32) {
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        let position_raw = entity.position_raw();
        let task_id = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .expect("fixture must retain class 14's Primary task");
        let Some(ActorTaskRuntime::SharedRetarget(task)) =
            entity.actor_tasks.task_state_mut(task_id)
        else {
            panic!("fixture must retain class 14's Primary task")
        };
        let mut draws = 0;
        let stage = task.stage_callback(position_raw, || {
            draws += 1;
            0xffff
        });
        task.commit_callback_stage(stage);
        assert_eq!(draws, 2);
        assert_eq!(task.private_state().target_position_raw, [4_095, 0, 4_095]);
    }

    fn advance_to_exact_lifetime(
        manager: &mut EntityManager,
        mut owner: MainBaseType9ExplodingProductionOwner,
        terrain: &TerrainGrid,
        draws: &mut usize,
    ) -> (
        MainBaseType9ExplodingProductionOwner,
        SharedRetargetCallbackPrefix,
    ) {
        let mut final_prefix = None;
        for _ in 0..8 {
            let tick = tick_main_base_type9_exploding_owner(
                manager,
                owner,
                frame(terrain, RETAIL_FRAME_DELTA_MAX_US),
                &mut || {
                    *draws += 1;
                    0x8000
                },
            );
            let MainBaseType9ExplodingProductionOutcome::Continuing {
                committed_prefix, ..
            } = tick.outcome
            else {
                panic!("each capped visit through exactly 1,000 ms must continue")
            };
            final_prefix = Some(committed_prefix);
            owner = tick
                .retained_owner
                .expect("a nonterminal capped visit retains running custody");
        }
        (
            owner,
            final_prefix.expect("the eight capped visits produce a final prefix"),
        )
    }

    #[test]
    fn near_and_far_retarget_paths_consume_only_callback_rng() {
        let terrain = flat_terrain();

        let (mut near_manager, near_owner) = owner_fixture(9);
        let near_words = [0x8000, 0x8000];
        let mut near_draws = 0usize;
        let near = tick_main_base_type9_exploding_owner(
            &mut near_manager,
            near_owner,
            frame(&terrain, 20_000),
            &mut || {
                let word = near_words[near_draws];
                near_draws += 1;
                word
            },
        );
        assert_eq!(near_draws, 2, "constructor RNG must not be replayed");
        let MainBaseType9ExplodingProductionOutcome::Continuing {
            committed_prefix, ..
        } = near.outcome
        else {
            panic!("a live near-target frame must continue")
        };
        assert_eq!(
            committed_prefix.retarget,
            SharedRetarget::Replaced {
                trigger: SharedRetargetTrigger::NearTargetAxis,
                target_position_raw: [0; 3],
            }
        );
        let RetailRuntimeValue::Known(Some(animation)) = near_manager
            .entity_mut_for_test(9)
            .unwrap()
            .actor_animation_runtime
        else {
            panic!("the exact class-14 Sub-I runtime must survive")
        };
        assert!(animation.special_mode());
        assert_eq!(animation.linked_handle(), None);
        assert_eq!(animation.phase(), 1);
        assert_eq!(animation.output(), 39);
        assert!(near.retained_owner.is_some());

        let (mut rejected_manager, rejected_owner) = owner_fixture(10);
        set_far_target(&mut rejected_manager, 10);
        let mut rejected_draws = 0;
        let rejected = tick_main_base_type9_exploding_owner(
            &mut rejected_manager,
            rejected_owner,
            frame(&terrain, 20_000),
            &mut || {
                rejected_draws += 1;
                1
            },
        );
        assert_eq!(rejected_draws, 1);
        let MainBaseType9ExplodingProductionOutcome::Continuing {
            committed_prefix, ..
        } = rejected.outcome
        else {
            panic!("a rejected far gate must continue")
        };
        assert_eq!(committed_prefix.retarget, SharedRetarget::Retained);

        let (mut accepted_manager, accepted_owner) = owner_fixture(11);
        set_far_target(&mut accepted_manager, 11);
        let accepted_words = [0, 0x8000, 0x8000];
        let mut accepted_draws = 0usize;
        let accepted = tick_main_base_type9_exploding_owner(
            &mut accepted_manager,
            accepted_owner,
            frame(&terrain, 20_000),
            &mut || {
                let word = accepted_words[accepted_draws];
                accepted_draws += 1;
                word
            },
        );
        assert_eq!(accepted_draws, 3);
        let MainBaseType9ExplodingProductionOutcome::Continuing {
            committed_prefix, ..
        } = accepted.outcome
        else {
            panic!("an accepted far gate must continue")
        };
        assert_eq!(
            committed_prefix.retarget,
            SharedRetarget::Replaced {
                trigger: SharedRetargetTrigger::RandomGate,
                target_position_raw: [0; 3],
            }
        );
    }

    #[test]
    fn strict_one_thousand_boundary_retains_then_one_thousand_one_terminates() {
        let terrain = flat_terrain();
        let (mut manager, owner) = owner_fixture(12);
        let mut exact_draws = 0;
        let (owner, committed_prefix) =
            advance_to_exact_lifetime(&mut manager, owner, &terrain, &mut exact_draws);
        assert_eq!(exact_draws, 16);
        assert_eq!(committed_prefix.elapsed_ms, 1_000);
        assert_eq!(shared_task(&mut manager, 12).elapsed_ms(), 1_000);

        let mut expired_draws = 0;
        let expired = tick_main_base_type9_exploding_owner(
            &mut manager,
            owner,
            frame(&terrain, 1_000),
            &mut || {
                expired_draws += 1;
                0x8000
            },
        );
        assert_eq!(expired_draws, 2);
        let MainBaseType9ExplodingProductionOutcome::Terminal(terminal) = expired.outcome else {
            panic!("elapsed > lifetime must complete class 14")
        };
        assert_eq!(terminal.entity_id, 12);
        assert_eq!(
            terminal.reason,
            SharedRetargetTransitionReason::LifetimeExpired
        );
        assert!(expired.retained_owner.is_none());
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), [12]);
        assert!(manager
            .entity_mut_for_test(12)
            .unwrap()
            .actor_task_state(ActorTaskSlot::Primary)
            .is_none());
    }

    #[test]
    fn oversized_running_frame_clocks_block_before_visit_rng_or_mutation() {
        let terrain = flat_terrain();
        for (entity_id, elapsed_micros, global_elapsed_micros, reason) in [
            (
                19,
                RETAIL_FRAME_DELTA_MAX_US + 1,
                RETAIL_FRAME_DELTA_MAX_US,
                MainBaseType9ExplodingProductionBlock::ElapsedExceedsRetailCap {
                    actual: RETAIL_FRAME_DELTA_MAX_US + 1,
                },
            ),
            (
                20,
                RETAIL_FRAME_DELTA_MAX_US,
                RETAIL_FRAME_DELTA_MAX_US + 1,
                MainBaseType9ExplodingProductionBlock::GlobalElapsedExceedsRetailCap {
                    actual: RETAIL_FRAME_DELTA_MAX_US + 1,
                },
            ),
        ] {
            let (mut manager, owner) = owner_fixture(entity_id);
            let task_lease = owner.task_lease();
            let task_before = shared_task(&mut manager, entity_id);
            let runtime_before = {
                let entity = manager.entity_mut_for_test(entity_id).unwrap();
                (
                    entity.main_base_type9_death_component_runtime,
                    entity.sub_a_propulsion_runtime,
                    entity.actor_animation_runtime,
                    entity.heading_raw(),
                    entity.velocity_raw(),
                    entity.actor_tasks.wrapper_flags(task_lease.task_id()),
                )
            };
            let mut draws = 0;
            let tick = tick_main_base_type9_exploding_owner(
                &mut manager,
                owner,
                MainBaseType9ExplodingProductionFrame {
                    scheduler_mode: 0,
                    terrain: &terrain,
                    elapsed_micros,
                    global_elapsed_micros,
                },
                &mut || {
                    draws += 1;
                    0x8000
                },
            );

            assert_eq!(draws, 0);
            assert_eq!(
                tick.outcome,
                MainBaseType9ExplodingProductionOutcome::Blocked { entity_id, reason }
            );
            assert_eq!(
                tick.retained_owner,
                Some(MainBaseType9ExplodingProductionOwner::adopt(task_lease))
            );
            assert_eq!(shared_task(&mut manager, entity_id), task_before);
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            assert_eq!(
                (
                    entity.main_base_type9_death_component_runtime,
                    entity.sub_a_propulsion_runtime,
                    entity.actor_animation_runtime,
                    entity.heading_raw(),
                    entity.velocity_raw(),
                    entity.actor_tasks.wrapper_flags(task_lease.task_id()),
                ),
                runtime_before
            );
        }
    }

    #[test]
    fn mover_evidence_block_retains_the_consumed_prefix_for_next_frame_only() {
        let terrain = flat_terrain();
        let (mut manager, owner) = owner_fixture(14);
        let entity = manager.entity_mut_for_test(14).unwrap();
        entity
            .main_base_type9_death_component_runtime
            .as_mut()
            .unwrap()
            .components
            .sub_d_frame_owner = Type9SubDFrameOwner::pending_constructor_origin(0x29);
        let death_before = entity.main_base_type9_death_component_runtime;
        let heading_before = entity.heading_raw();
        let velocity_before = entity.velocity_raw();
        let mut draws = 0;

        let tick = tick_main_base_type9_exploding_owner(
            &mut manager,
            owner,
            frame(&terrain, 20_000),
            &mut || {
                draws += 1;
                0x8000
            },
        );

        assert_eq!(draws, 2);
        let MainBaseType9ExplodingProductionOutcome::Blocked {
            reason:
                MainBaseType9ExplodingProductionBlock::FrameBlocked {
                    committed_prefix,
                    reason:
                        OrdinaryType9FrameBlock::SubD(
                            crate::common_mover::sub_d::Type9SubDStep::UnresolvedClassifierCache,
                        ),
                    ..
                },
            ..
        } = tick.outcome
        else {
            panic!("the unresolved classifier cache must block after retarget")
        };
        assert_eq!(committed_prefix.elapsed_ms, 20);
        assert!(matches!(
            committed_prefix.retarget,
            SharedRetarget::Replaced {
                trigger: SharedRetargetTrigger::NearTargetAxis,
                ..
            }
        ));
        let retained = tick
            .retained_owner
            .expect("a consumed evidence block retains next-frame custody");
        assert!(retained.pending_transition().is_none());
        let entity = manager.entity_mut_for_test(14).unwrap();
        assert_eq!(entity.main_base_type9_death_component_runtime, death_before);
        assert_eq!(entity.heading_raw(), heading_before);
        assert_eq!(entity.velocity_raw(), velocity_before);
        assert_eq!(shared_task(&mut manager, 14).elapsed_ms(), 20);
    }

    #[test]
    fn stale_replaced_primary_drops_without_rng_or_callback_prefix() {
        let terrain = flat_terrain();
        let (mut manager, owner) = owner_fixture(15);
        let task_lease = owner.task_lease();
        let entity = manager.entity_mut_for_test(15).unwrap();
        let old_state = *entity
            .actor_tasks
            .task_state(task_lease.task_id())
            .expect("the adopted task exists");
        entity
            .actor_tasks
            .replace_prepared(ActorTaskSlot::Primary, PreparedActorTask::new(old_state));
        let replacement_before = shared_task(&mut manager, 15);

        let tick = tick_main_base_type9_exploding_owner(
            &mut manager,
            owner,
            frame(&terrain, 20_000),
            &mut || panic!("a stale lease consumes no RNG"),
        );

        assert_eq!(
            tick.outcome,
            MainBaseType9ExplodingProductionOutcome::Dropped {
                entity_id: 15,
                reason: MainBaseType9ExplodingProductionDrop::PrimaryTaskMissingOrReplaced,
            }
        );
        assert!(tick.retained_owner.is_none());
        assert_eq!(shared_task(&mut manager, 15), replacement_before);
    }

    #[test]
    fn invalid_special_sub_i_state_drops_before_elapsed_or_retarget_rng() {
        let terrain = flat_terrain();
        let (mut manager, owner) = owner_fixture(18);
        let task_before = shared_task(&mut manager, 18);
        manager
            .entity_mut_for_test(18)
            .unwrap()
            .actor_animation_runtime = RetailRuntimeValue::Known(Some(
            ActorAnimationController::from_descriptor(LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR)
                .expect("the exact descriptor has a valid binding"),
        ));

        let tick = tick_main_base_type9_exploding_owner(
            &mut manager,
            owner,
            frame(&terrain, 20_000),
            &mut || panic!("invalid class-14 Sub-I state consumes no retarget RNG"),
        );

        assert_eq!(
            tick.outcome,
            MainBaseType9ExplodingProductionOutcome::Dropped {
                entity_id: 18,
                reason: MainBaseType9ExplodingProductionDrop::ActorAnimationSpecialStateMismatch,
            }
        );
        assert!(tick.retained_owner.is_none());
        assert_eq!(shared_task(&mut manager, 18), task_before);
    }

    #[test]
    fn pending_terminal_retry_never_replays_elapsed_rng_or_mover() {
        let terrain = flat_terrain();
        let (mut manager, owner) = owner_fixture(16);
        manager
            .entity_mut_for_test(16)
            .unwrap()
            .collision
            .state_flags_at_0x08 = RetailStateWord::from_known_bits(
            DYING_STATE_BIT,
            DYING_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT,
        );
        let mut prefix_draws = 0;
        let (owner, exact_prefix) =
            advance_to_exact_lifetime(&mut manager, owner, &terrain, &mut prefix_draws);
        assert_eq!(prefix_draws, 16);
        assert_eq!(exact_prefix.elapsed_ms, 1_000);
        let mut transition_draws = 0;
        let first = tick_main_base_type9_exploding_owner(
            &mut manager,
            owner,
            frame(&terrain, 1_000),
            &mut || {
                transition_draws += 1;
                0x8000
            },
        );
        assert_eq!(transition_draws, 2);
        assert_eq!(
            first.outcome,
            MainBaseType9ExplodingProductionOutcome::Blocked {
                entity_id: 16,
                reason: MainBaseType9ExplodingProductionBlock::TerminalTransition(
                    MainBaseType9TerminalBlock::OwnerTransitionSuppressionStateUnresolved,
                ),
            }
        );
        let pending = first
            .retained_owner
            .expect("the exact post-unwind request must remain in custody");
        let transition = pending
            .pending_transition()
            .expect("terminal evidence blocks retain pending-transition state");
        assert_eq!(transition.committed_prefix.elapsed_ms, 1_001);
        let task_before_retry = shared_task(&mut manager, 16);
        let entity = manager.entity_mut_for_test(16).unwrap();
        let mover_before_retry = (
            entity.main_base_type9_death_component_runtime,
            entity.heading_raw(),
            entity.velocity_raw(),
        );

        entity.collision.state_flags_at_0x08 = RetailStateWord::from_known_bits(
            DYING_STATE_BIT,
            DYING_STATE_BIT | LEVEL_ONE_TYPE9_OWNER_TRANSITION_SUPPRESSION_BIT,
        );
        let second = tick_main_base_type9_exploding_owner(
            &mut manager,
            pending,
            frame(&terrain, u32::MAX),
            &mut || panic!("pending terminal retry must consume no RNG"),
        );
        assert_eq!(
            second.outcome,
            MainBaseType9ExplodingProductionOutcome::Blocked {
                entity_id: 16,
                reason: MainBaseType9ExplodingProductionBlock::TerminalTransition(
                    MainBaseType9TerminalBlock::DeferredDestroyStateUnresolved,
                ),
            }
        );
        let pending = second
            .retained_owner
            .expect("a second terminal evidence block retains the same request");
        assert_eq!(pending.pending_transition(), Some(transition));
        assert_eq!(shared_task(&mut manager, 16), task_before_retry);
        let entity = manager.entity_mut_for_test(16).unwrap();
        assert_eq!(
            (
                entity.main_base_type9_death_component_runtime,
                entity.heading_raw(),
                entity.velocity_raw(),
            ),
            mover_before_retry
        );

        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(DYING_STATE_BIT);
        let terminal = tick_main_base_type9_exploding_owner(
            &mut manager,
            pending,
            frame(&terrain, u32::MAX),
            &mut || panic!("resolved pending transition must consume no RNG"),
        );
        assert!(matches!(
            terminal.outcome,
            MainBaseType9ExplodingProductionOutcome::Terminal(_)
        ));
        assert!(terminal.retained_owner.is_none());
        let entity = manager.entity_mut_for_test(16).unwrap();
        assert_eq!(
            (
                entity.main_base_type9_death_component_runtime,
                entity.heading_raw(),
                entity.velocity_raw(),
            ),
            mover_before_retry
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_none());
    }

    #[test]
    fn resolved_suppression_returns_running_custody_for_the_next_full_callback() {
        let terrain = flat_terrain();
        let (mut manager, owner) = owner_fixture(17);
        manager
            .entity_mut_for_test(17)
            .unwrap()
            .collision
            .state_flags_at_0x08 = RetailStateWord::from_known_bits(
            DYING_STATE_BIT,
            DYING_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT,
        );
        let mut prefix_draws = 0;
        let (owner, exact_prefix) =
            advance_to_exact_lifetime(&mut manager, owner, &terrain, &mut prefix_draws);
        assert_eq!(prefix_draws, 16);
        assert_eq!(exact_prefix.elapsed_ms, 1_000);
        let mut transition_draws = 0;
        let first = tick_main_base_type9_exploding_owner(
            &mut manager,
            owner,
            frame(&terrain, 1_000),
            &mut || {
                transition_draws += 1;
                0x8000
            },
        );
        assert_eq!(transition_draws, 2);
        let pending = first
            .retained_owner
            .expect("unresolved suppression retains the exact transition request");
        assert!(pending.pending_transition().is_some());
        assert_eq!(shared_task(&mut manager, 17).elapsed_ms(), 1_001);

        manager
            .entity_mut_for_test(17)
            .unwrap()
            .collision
            .state_flags_at_0x08 = RetailStateWord::exact(
            DYING_STATE_BIT | LEVEL_ONE_TYPE9_OWNER_TRANSITION_SUPPRESSION_BIT,
        );
        let suppressed = tick_main_base_type9_exploding_owner(
            &mut manager,
            pending,
            frame(&terrain, u32::MAX),
            &mut || panic!("resolving a pending transition consumes no callback RNG"),
        );
        let MainBaseType9ExplodingProductionOutcome::TransitionSuppressed {
            request: suppressed_request,
            ..
        } = suppressed.outcome
        else {
            panic!("known 0x1000 suppresses only the retained transition")
        };
        assert_eq!(suppressed_request.committed_prefix.elapsed_ms, 1_001);
        let running = suppressed
            .retained_owner
            .expect("known suppression returns running custody");
        assert!(running.pending_transition().is_none());
        assert_eq!(shared_task(&mut manager, 17).elapsed_ms(), 1_001);

        let mut next_frame_draws = 0;
        let next = tick_main_base_type9_exploding_owner(
            &mut manager,
            running,
            frame(&terrain, 1_000),
            &mut || {
                next_frame_draws += 1;
                0x8000
            },
        );
        assert_eq!(
            next_frame_draws, 2,
            "the next pass must rerun retarget rather than retry the old transition"
        );
        let MainBaseType9ExplodingProductionOutcome::TransitionSuppressed {
            request: next_request,
            ..
        } = next.outcome
        else {
            panic!("the still-set suppression bit suppresses the new frame transition")
        };
        assert_eq!(next_request.committed_prefix.elapsed_ms, 1_002);
        assert_eq!(shared_task(&mut manager, 17).elapsed_ms(), 1_002);
        assert!(next
            .retained_owner
            .expect("suppressed task remains runnable")
            .pending_transition()
            .is_none());
    }

    #[test]
    fn only_retryable_terminal_gates_retain_pending_transition() {
        for retryable in [
            MainBaseType9TerminalBlock::TaskStillInCallback,
            MainBaseType9TerminalBlock::OwnerTransitionSuppressionStateUnresolved,
            MainBaseType9TerminalBlock::DeferredDestroyStateUnresolved,
        ] {
            assert!(terminal_block_retains_pending_transition(retryable));
        }
        for stale in [
            MainBaseType9TerminalBlock::EntityMissing,
            MainBaseType9TerminalBlock::PrimaryTaskMissingOrReplaced,
            MainBaseType9TerminalBlock::WrongTaskFamily,
            MainBaseType9TerminalBlock::BehaviorContextMismatch,
            MainBaseType9TerminalBlock::DeferredDestroyAlreadyPending,
            MainBaseType9TerminalBlock::DeferredDestroyAlreadyQueued,
            MainBaseType9TerminalBlock::OwnerTransitionSuppressed,
        ] {
            assert!(!terminal_block_retains_pending_transition(stale));
        }
    }
}
