//! Shared task-family phases for class-54 `"Go To Job"`.
//!
//! Retail's task owner is heterogeneous: any of its three slots may hold a
//! different task family, and a transition may replace one family with
//! another. This module therefore does **not** own a family-specific scheduler
//! loop. It provides the exact state construction, callback evaluator, and
//! post-unwind decision while [`crate::actor_task_owner::ActorTaskOwner`] owns
//! slots and [`crate::actor_task_dispatcher`] owns fresh-slot traversal.
//! [`crate::ordinary_type9_go_to_job_production`] now binds those phases to the
//! authenticated fresh-Level-1 ordinary Type-9 scheduler, entity lookup,
//! actor-local route context, common mover, root transition, and direct F70
//! publication. Cargo-converted type-8 scientists use a separate
//! [`crate::type8_go_to_job_production`] visit that binds the shared D/I/A/B
//! kernel without Type-9 selected custody or an invented E370 surface owner.
//!
//! The callback evaluator exposes every `FUN_00401430` argument explicitly.
//! Its caller must stage [`GoToJobCallbackStage`] before entering the callback
//! and commit it only if the same wrapper survives unwind. Live entity lookup,
//! controller/component resolution, the common mover itself, and behavior
//! transitions remain outside this behavior-neutral seam and belong to each
//! adopting production adapter.

use crate::actor_task_owner::ActorTaskVisit;
use crate::go_to_job::GoToJobTaskSpec;
use crate::shared_target_route::{
    evaluate_shared_target_route_callback, shared_target_route_transition_after_unwind,
    SharedTargetRouteCallbackStage, SharedTargetRouteTaskState,
};
use crate::wander_near_location::{WanderNearPrivateState, WANDER_NEAR_NO_TRACKED_ENTITY};

pub use crate::shared_target_route::{
    SharedTargetRouteCallbackCall as GoToJobCallbackCall,
    SharedTargetRouteCallbackError as GoToJobCallbackError,
    SharedTargetRouteCallbackPrefix as GoToJobCallbackPrefix,
    SharedTargetRouteCallbackRequest as GoToJobCallbackRequest,
    SharedTargetRouteCallbackResult as GoToJobCallbackResult,
    SharedTargetRouteCommonMoverRequest as GoToJobCommonMoverRequest,
    SharedTargetRouteCommonMoverReturn as GoToJobCommonMoverReturn,
    SharedTargetRouteLifetimeStatus as GoToJobLifetimeStatus,
    SharedTargetRoutePredicate as GoToJobRoutePredicate,
    SharedTargetRoutePredicateRequest as GoToJobRoutePredicateRequest,
    SharedTargetRouteTaggedSingleton as GoToJobTaggedSingleton,
    SharedTargetRouteTargetRuntimeState as GoToJobTargetRuntimeState,
    SharedTargetRouteTransitionReason as GoToJobTransitionReason,
    SharedTargetRouteTransitionRequest as GoToJobTransitionRequest,
};

pub const GO_TO_JOB_ROUTE_PREDICATE_ADDRESS: u32 =
    crate::shared_target_route::SHARED_TARGET_ROUTE_PREDICATE_ADDRESS;
pub const GO_TO_JOB_COMMON_MOVER_ADDRESS: u32 =
    crate::shared_target_route::SHARED_TARGET_ROUTE_COMMON_MOVER_ADDRESS;
pub const GO_TO_JOB_INVALID_TARGET_SINGLETON_ADDRESS: u32 =
    crate::shared_target_route::SHARED_TARGET_ROUTE_INVALID_TARGET_SINGLETON_ADDRESS;
pub const GO_TO_JOB_NONZERO_PREDICATE_ZERO_MOVER_SINGLETON_ADDRESS: u32 =
    crate::shared_target_route::SHARED_TARGET_ROUTE_NONZERO_PREDICATE_ZERO_MOVER_SINGLETON_ADDRESS;
pub const GO_TO_JOB_ZERO_PREDICATE_SINGLETON_ADDRESS: u32 =
    crate::shared_target_route::SHARED_TARGET_ROUTE_ZERO_PREDICATE_SINGLETON_ADDRESS;
pub const GO_TO_JOB_OWNER_TRANSITION_TAG: u32 =
    crate::shared_target_route::SHARED_TARGET_ROUTE_OWNER_TRANSITION_TAG;

/// Exact consumed fields of the slot-0 class-54 state.
///
/// `FUN_00401350` allocates the shared 0x24-byte private record initialized by
/// `FUN_004012E0`; the generic wrapper owns elapsed milliseconds separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoToJobTaskState {
    shared_target_route: SharedTargetRouteTaskState,
}

impl GoToJobTaskState {
    /// Construct the callback-consumed private state after allocation.
    ///
    /// The private specification binds the selected target to the position
    /// captured by the same setup transaction. Retail does not retain owner
    /// identity in this private record; the adopting live dispatcher must
    /// verify it when resolving the callback entity. A missing target uses
    /// retail's zero handle sentinel and fails the callback's first validity
    /// gate.
    ///
    /// The task-private state intentionally excludes `FUN_00403650`'s
    /// external component suffix. [`crate::go_to_job::GoToJobSetupPlan::apply`]
    /// commits that proven Sub-A reset after this allocation/state phase and
    /// before publishing the wrapper.
    pub fn after_allocation(specification: GoToJobTaskSpec) -> Self {
        let tracked_entity_handle = specification
            .target_id()
            .unwrap_or(WANDER_NEAR_NO_TRACKED_ENTITY);
        Self {
            shared_target_route: SharedTargetRouteTaskState::after_allocation(
                specification.initial_position_raw(),
                tracked_entity_handle,
            ),
        }
    }

    pub const fn private_state(self) -> WanderNearPrivateState {
        self.shared_target_route.private_state()
    }

    pub const fn target_id(self) -> Option<u32> {
        self.shared_target_route.target_id()
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.shared_target_route.elapsed_ms()
    }

    pub const fn shared_target_route(self) -> SharedTargetRouteTaskState {
        self.shared_target_route
    }

    /// Run generic-owner phase 1 for this task only.
    pub fn before_callback(&mut self, elapsed_micros: u32) -> GoToJobCallbackPrefix {
        self.shared_target_route.before_callback(elapsed_micros)
    }

    /// Copy the mutable task-private record before callback entry.
    ///
    /// The central dispatcher commits this stage only after the wrapper
    /// survives callback unwind. This avoids borrowing the task state while a
    /// common-mover adapter is allowed to clear or replace the owner slot.
    pub const fn stage_callback(self) -> GoToJobCallbackStage {
        GoToJobCallbackStage {
            shared_target_route: self.shared_target_route.stage_callback(),
        }
    }
}

/// Staged `FUN_004012E0` private state for one callback invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoToJobCallbackStage {
    shared_target_route: SharedTargetRouteCallbackStage,
}

impl GoToJobCallbackStage {
    pub const fn private_state(self) -> WanderNearPrivateState {
        self.shared_target_route.private_state()
    }

    pub fn private_state_mut(&mut self) -> &mut WanderNearPrivateState {
        self.shared_target_route.private_state_mut()
    }

    /// Commit only after the central owner proves the same task survived.
    pub fn commit(self, surviving_state: &mut GoToJobTaskState) {
        self.shared_target_route
            .commit(&mut surviving_state.shared_target_route);
    }
}

/// Evaluate `FUN_00403780` against a staged private record.
///
/// Order is target validity, `FUN_00423030`, then `FUN_00401430`. The
/// predicate-zero branch always returns its singleton after invoking the
/// mover; the predicate-nonzero branch returns a singleton only for mover
/// zero. The caller owns wrapper unwind and stage commit.
pub fn evaluate_go_to_job_callback<
    MovementState,
    ControllerContext,
    TargetError,
    PredicateError,
    MoverError,
>(
    stage: &mut GoToJobCallbackStage,
    request: GoToJobCallbackRequest<'_, MovementState, ControllerContext>,
    validate_target: impl FnMut(u32) -> Result<GoToJobTargetRuntimeState, TargetError>,
    route_predicate: impl FnMut(
        GoToJobRoutePredicateRequest<'_, ControllerContext>,
    ) -> Result<GoToJobRoutePredicate, PredicateError>,
    common_mover: impl FnMut(
        GoToJobCommonMoverRequest<'_, MovementState, ControllerContext>,
    ) -> Result<GoToJobCommonMoverReturn, MoverError>,
) -> Result<GoToJobCallbackResult, GoToJobCallbackError<TargetError, PredicateError, MoverError>> {
    evaluate_shared_target_route_callback(
        &mut stage.shared_target_route,
        request,
        validate_target,
        route_predicate,
        common_mover,
    )
}

/// Resolve generic-owner phase 3 after the wrapper survived callback unwind.
///
/// A tagged callback result pre-empts timeout. Only a null/continue result
/// falls through to the strict `elapsed > 5000` lifetime decision.
pub const fn go_to_job_transition_after_unwind(
    visit: ActorTaskVisit,
    committed_prefix: GoToJobCallbackPrefix,
    callback_result: GoToJobCallbackResult,
) -> Option<GoToJobTransitionRequest> {
    shared_target_route_transition_after_unwind(visit, committed_prefix, callback_result)
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::convert::Infallible;

    use super::*;
    use crate::actor_task_owner::{
        ActorTaskOwner, ActorTaskSlot, ActorTaskVisitControl, PreparedActorTask,
    };
    use crate::common_mover::SubAPropulsionRuntime;
    use crate::entity_collision_state::{
        CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
        RetailStateWord,
    };
    use crate::go_to_job::{
        plan_go_to_job_setup, GoToJobCandidate, GoToJobOwner, GoToJobSetupPlan,
        GoToJobSetupRequest, GO_TO_JOB_FAST_TARGET_CAPABILITY_BIT,
        GO_TO_JOB_FIRST_WORLD_SCIENTIST_TYPE, GO_TO_JOB_TASK_LIFETIME_MS,
    };
    use crate::wrapped_axis_range::WrappedAxisRange;
    use v2k_formats::collision::SubAPropulsionDescriptor;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestTask {
        Other(&'static str),
        GoToJob(GoToJobTaskState),
    }

    fn type8_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                SubAPropulsionDescriptor {
                    acceleration_raw: 1,
                    overspeed_correction_raw: 2,
                    target_speed_base_raw: 0x0300,
                },
            )),
            common_mover_topology: RetailRuntimeValue::Known(CommonMoverComponentTopology {
                sub_a: true,
                ..CommonMoverComponentTopology::default()
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn sub_a_runtime() -> SubAPropulsionRuntime {
        SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(77), -1, 73)
    }

    fn type8_owner() -> GoToJobOwner {
        GoToJobOwner::from_type_metadata(
            42,
            [0x0100, 0x0200, 0x0300],
            RetailRuntimeValue::Known(0x0800),
            GO_TO_JOB_FIRST_WORLD_SCIENTIST_TYPE,
            &type8_metadata(),
        )
    }

    fn setup_plan(target_id: Option<u32>) -> GoToJobSetupPlan {
        let candidate = target_id.map(|id| GoToJobCandidate {
            id,
            position_raw: [0x0400, 0x0500, 0x0600],
            state_flags: RetailStateWord::exact(1),
            capability_flags: RetailRuntimeValue::Known(GO_TO_JOB_FAST_TARGET_CAPABILITY_BIT),
            capacity: RetailRuntimeValue::Unresolved,
        });
        plan_go_to_job_setup(GoToJobSetupRequest {
            owner: type8_owner(),
            candidates_in_intrusive_order: candidate.as_slice(),
            range: WrappedAxisRange::strict(0x1000).unwrap(),
        })
        .unwrap()
    }

    fn task_state(target_id: Option<u32>) -> GoToJobTaskState {
        let mut owner = ActorTaskOwner::new();
        let mut sub_a = sub_a_runtime();
        setup_plan(target_id)
            .apply(
                type8_owner().bind_runtime(&mut owner, &mut sub_a),
                || 0x1234_ABCD,
                |specification| {
                    Ok::<_, Infallible>(PreparedActorTask::new(TestTask::GoToJob(
                        GoToJobTaskState::after_allocation(specification),
                    )))
                },
            )
            .unwrap();
        match owner
            .state_in_slot(ActorTaskSlot::Primary)
            .copied()
            .unwrap()
        {
            TestTask::GoToJob(state) => state,
            TestTask::Other(_) => panic!("setup installed the wrong task family"),
        }
    }

    fn visit(slot: ActorTaskSlot) -> ActorTaskVisit {
        let mut owner = ActorTaskOwner::new();
        let task_id = owner.replace_prepared(slot, PreparedActorTask::new(()));
        ActorTaskVisit { slot, task_id }
    }

    #[test]
    fn generic_setup_installs_exact_private_state_into_a_heterogeneous_owner() {
        let mut owner = ActorTaskOwner::new();
        let mut sub_a = sub_a_runtime();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(TestTask::Other("old primary")),
        );
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(TestTask::Other("old secondary")),
        );
        owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(TestTask::Other("old tertiary")),
        );

        setup_plan(Some(77))
            .apply(
                type8_owner().bind_runtime(&mut owner, &mut sub_a),
                || 0x1234_ABCD,
                |specification| {
                    assert_eq!(
                        specification.initial_position_raw(),
                        [0x0100, 0x0200, 0x0300]
                    );
                    Ok::<_, Infallible>(PreparedActorTask::new(TestTask::GoToJob(
                        GoToJobTaskState::after_allocation(specification),
                    )))
                },
            )
            .unwrap();

        let TestTask::GoToJob(state) = owner
            .state_in_slot(ActorTaskSlot::Primary)
            .copied()
            .unwrap()
        else {
            panic!("primary task was not Go To Job");
        };
        assert_eq!(
            state.private_state(),
            WanderNearPrivateState::tracked_entity([0x0100, 0x0200, 0x0300], 77)
        );
        assert_eq!(state.elapsed_ms(), 0);
        assert_eq!(owner.state_in_slot(ActorTaskSlot::Secondary), None);
        assert_eq!(owner.state_in_slot(ActorTaskSlot::Tertiary), None);
    }

    #[test]
    fn missing_target_uses_zero_sentinel_and_exact_shared_initializer_defaults() {
        let state = task_state(None);
        assert_eq!(state.target_id(), None);
        assert_eq!(
            state.private_state(),
            WanderNearPrivateState {
                target_position_raw: [0x0100, 0x0200, 0x0300],
                tracked_entity_handle: WANDER_NEAR_NO_TRACKED_ENTITY,
                direction: 1,
                reversal_timer_ms: 0,
            }
        );
    }

    #[test]
    fn tagged_singletons_keep_distinct_addresses_with_the_same_retail_tag() {
        let values = [
            GoToJobTaggedSingleton::InvalidTarget,
            GoToJobTaggedSingleton::NonZeroPredicateAndZeroMover,
            GoToJobTaggedSingleton::ZeroPredicate,
        ];
        assert_eq!(
            values.map(GoToJobTaggedSingleton::address),
            [0x004B_E0B8, 0x004B_E0C0, 0x004B_E0C8]
        );
        assert!(values
            .into_iter()
            .all(|singleton| singleton.tag() == 0x0000_9C01));
    }

    #[test]
    fn invalid_target_short_circuits_predicate_and_mover() {
        let state = task_state(Some(77));
        let mut stage = state.stage_callback();
        let predicate_calls = Cell::new(0);
        let mover_calls = Cell::new(0);
        let mut movement = ();
        let mut controller = ();

        let result = evaluate_go_to_job_callback(
            &mut stage,
            GoToJobCallbackRequest {
                visit: visit(ActorTaskSlot::Primary),
                entity_id: 42,
                movement_state: &mut movement,
                controller_context: &mut controller,
                elapsed_micros: 1_999,
                scheduler_mode: 1,
            },
            |target_id| {
                assert_eq!(target_id, 77);
                Ok::<_, Infallible>(GoToJobTargetRuntimeState::Dying)
            },
            |_request| {
                predicate_calls.set(predicate_calls.get() + 1);
                Ok::<_, Infallible>(GoToJobRoutePredicate::NonZero)
            },
            |_request| {
                mover_calls.set(mover_calls.get() + 1);
                Ok::<_, Infallible>(GoToJobCommonMoverReturn::NonZero)
            },
        )
        .unwrap();

        assert_eq!(
            result,
            GoToJobCallbackResult::Tagged(GoToJobTaggedSingleton::InvalidTarget)
        );
        assert_eq!(predicate_calls.get(), 0);
        assert_eq!(mover_calls.get(), 0);
    }

    #[test]
    fn zero_target_sentinel_skips_lookup_predicate_and_mover() {
        let state = task_state(None);
        let mut stage = state.stage_callback();
        let target_calls = Cell::new(0);
        let predicate_calls = Cell::new(0);
        let mover_calls = Cell::new(0);
        let mut movement = ();
        let mut controller = ();

        let result = evaluate_go_to_job_callback(
            &mut stage,
            GoToJobCallbackRequest {
                visit: visit(ActorTaskSlot::Primary),
                entity_id: 42,
                movement_state: &mut movement,
                controller_context: &mut controller,
                elapsed_micros: 1_000,
                scheduler_mode: 0,
            },
            |_target_id| {
                target_calls.set(target_calls.get() + 1);
                Ok::<_, Infallible>(GoToJobTargetRuntimeState::Live)
            },
            |_request| {
                predicate_calls.set(predicate_calls.get() + 1);
                Ok::<_, Infallible>(GoToJobRoutePredicate::NonZero)
            },
            |_request| {
                mover_calls.set(mover_calls.get() + 1);
                Ok::<_, Infallible>(GoToJobCommonMoverReturn::NonZero)
            },
        )
        .unwrap();

        assert_eq!(
            result,
            GoToJobCallbackResult::Tagged(GoToJobTaggedSingleton::InvalidTarget)
        );
        assert_eq!(target_calls.get(), 0);
        assert_eq!(predicate_calls.get(), 0);
        assert_eq!(mover_calls.get(), 0);
    }

    #[derive(Debug, Default)]
    struct MovementState {
        mover_calls: u32,
    }

    #[derive(Debug)]
    struct ControllerContext {
        route_limit_raw: i32,
        mover_calls: u32,
    }

    fn run_branch(
        predicate: GoToJobRoutePredicate,
        mover_return: GoToJobCommonMoverReturn,
    ) -> (GoToJobCallbackResult, Vec<&'static str>) {
        let state = task_state(Some(77));
        let mut stage = state.stage_callback();
        let events = RefCell::new(Vec::new());
        let mut movement = MovementState::default();
        let mut controller = ControllerContext {
            route_limit_raw: 0x0700,
            mover_calls: 0,
        };

        let result = evaluate_go_to_job_callback(
            &mut stage,
            GoToJobCallbackRequest {
                visit: visit(ActorTaskSlot::Primary),
                entity_id: 42,
                movement_state: &mut movement,
                controller_context: &mut controller,
                elapsed_micros: 0x0001_7700,
                scheduler_mode: 1,
            },
            |target_id| {
                events.borrow_mut().push("validate");
                assert_eq!(target_id, 77);
                Ok::<_, Infallible>(GoToJobTargetRuntimeState::Live)
            },
            |request| {
                events.borrow_mut().push("predicate");
                assert_eq!(request.entity_id, 42);
                assert_eq!(request.target_id, 77);
                assert_eq!(request.controller_context.route_limit_raw, 0x0700);
                Ok::<_, Infallible>(predicate)
            },
            |request| {
                events.borrow_mut().push("mover");
                assert_eq!(request.visit, visit(ActorTaskSlot::Primary));
                assert_eq!(request.entity_id, 42);
                assert_eq!(request.elapsed_micros, 0x0001_7700);
                assert_eq!(request.scheduler_mode, 1);
                assert_eq!(request.target_state.tracked_entity_handle, 77);
                request.target_state.target_position_raw[0] = 0x0555;
                request.movement_state.mover_calls += 1;
                request.controller_context.mover_calls += 1;
                Ok::<_, Infallible>(mover_return)
            },
        )
        .unwrap();

        assert_eq!(movement.mover_calls, 1);
        assert_eq!(controller.mover_calls, 1);
        assert_eq!(stage.private_state().target_position_raw[0], 0x0555);
        (result, events.into_inner())
    }

    #[test]
    fn predicate_and_mover_results_preserve_all_four_callback_branches() {
        assert_eq!(
            run_branch(
                GoToJobRoutePredicate::NonZero,
                GoToJobCommonMoverReturn::NonZero
            ),
            (
                GoToJobCallbackResult::Continue,
                vec!["validate", "predicate", "mover"]
            )
        );
        assert_eq!(
            run_branch(
                GoToJobRoutePredicate::NonZero,
                GoToJobCommonMoverReturn::Zero
            )
            .0,
            GoToJobCallbackResult::Tagged(GoToJobTaggedSingleton::NonZeroPredicateAndZeroMover)
        );
        assert_eq!(
            run_branch(
                GoToJobRoutePredicate::Zero,
                GoToJobCommonMoverReturn::NonZero
            )
            .0,
            GoToJobCallbackResult::Tagged(GoToJobTaggedSingleton::ZeroPredicate)
        );
        assert_eq!(
            run_branch(GoToJobRoutePredicate::Zero, GoToJobCommonMoverReturn::Zero).0,
            GoToJobCallbackResult::Tagged(GoToJobTaggedSingleton::ZeroPredicate)
        );
    }

    #[test]
    fn callback_stage_is_not_visible_until_surviving_owner_commits_it() {
        let mut state = task_state(Some(77));
        let mut stage = state.stage_callback();
        stage.private_state_mut().target_position_raw = [9, 8, 7];

        assert_eq!(
            state.private_state().target_position_raw,
            [0x0100, 0x0200, 0x0300]
        );
        stage.commit(&mut state);
        assert_eq!(state.private_state().target_position_raw, [9, 8, 7]);
    }

    #[test]
    fn mover_error_leaves_mutation_staged_for_the_surviving_wrapper_policy() {
        let mut state = task_state(Some(77));
        let mut stage = state.stage_callback();
        let mut movement = ();
        let mut controller = ();

        let result = evaluate_go_to_job_callback(
            &mut stage,
            GoToJobCallbackRequest {
                visit: visit(ActorTaskSlot::Primary),
                entity_id: 42,
                movement_state: &mut movement,
                controller_context: &mut controller,
                elapsed_micros: 1_000,
                scheduler_mode: 0,
            },
            |_target_id| Ok::<_, Infallible>(GoToJobTargetRuntimeState::Live),
            |_request| Ok::<_, Infallible>(GoToJobRoutePredicate::NonZero),
            |request| {
                request.target_state.target_position_raw = [9, 8, 7];
                Err::<GoToJobCommonMoverReturn, _>("unresolved mover dependency")
            },
        );

        assert!(matches!(
            result,
            Err(GoToJobCallbackError::CommonMover {
                error: "unresolved mover dependency",
                ..
            })
        ));
        assert_eq!(
            state.private_state().target_position_raw,
            [0x0100, 0x0200, 0x0300]
        );
        assert_eq!(stage.private_state().target_position_raw, [9, 8, 7]);

        // The central dispatcher decides this after callback unwind. A
        // surviving wrapper commits the staged retail prefix even though the
        // portable adapter could not finish the mover.
        stage.commit(&mut state);
        assert_eq!(state.private_state().target_position_raw, [9, 8, 7]);
    }

    #[test]
    fn strict_timeout_and_tagged_result_precedence_are_post_unwind_pure() {
        let mut state = task_state(Some(77));
        let first_prefix = state.before_callback(5_000_999);
        assert_eq!(
            first_prefix.lifetime_status,
            GoToJobLifetimeStatus::WithinLifetime
        );
        assert_eq!(
            go_to_job_transition_after_unwind(
                visit(ActorTaskSlot::Primary),
                first_prefix,
                GoToJobCallbackResult::Continue,
            ),
            None
        );

        let due_prefix = state.before_callback(1_000);
        assert_eq!(state.elapsed_ms(), GO_TO_JOB_TASK_LIFETIME_MS + 1);
        assert_eq!(
            go_to_job_transition_after_unwind(
                visit(ActorTaskSlot::Primary),
                due_prefix,
                GoToJobCallbackResult::Continue,
            )
            .unwrap()
            .reason,
            GoToJobTransitionReason::LifetimeExpired
        );
        assert_eq!(
            go_to_job_transition_after_unwind(
                visit(ActorTaskSlot::Primary),
                due_prefix,
                GoToJobCallbackResult::Tagged(GoToJobTaggedSingleton::ZeroPredicate),
            )
            .unwrap()
            .reason,
            GoToJobTransitionReason::TaggedCallbackResult(GoToJobTaggedSingleton::ZeroPredicate)
        );
    }

    #[derive(Debug)]
    enum TestBefore {
        GoToJob(GoToJobCallbackPrefix),
        Other,
    }

    #[derive(Debug)]
    enum TestCallback {
        GoToJob(GoToJobCallbackStage, GoToJobCallbackResult),
        Other,
    }

    #[test]
    fn post_unwind_transition_can_replace_a_later_slot_and_visit_it_fresh() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(TestTask::GoToJob(task_state(Some(77)))),
        );
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(TestTask::Other("old secondary")),
        );
        let events = RefCell::new(Vec::new());
        let mut movement = ();
        let mut controller = ();

        owner.visit_slots_fresh_phased(
            |task, _visit| match task {
                TestTask::GoToJob(state) => TestBefore::GoToJob(state.before_callback(1_000)),
                TestTask::Other(_) => TestBefore::Other,
            },
            |owner, visit| match owner.task_state(visit.task_id).copied().unwrap() {
                TestTask::GoToJob(state) => {
                    events.borrow_mut().push("go callback");
                    let mut stage = state.stage_callback();
                    let result = evaluate_go_to_job_callback(
                        &mut stage,
                        GoToJobCallbackRequest {
                            visit,
                            entity_id: 42,
                            movement_state: &mut movement,
                            controller_context: &mut controller,
                            elapsed_micros: 1_000,
                            scheduler_mode: 0,
                        },
                        |_target_id| Ok::<_, Infallible>(GoToJobTargetRuntimeState::Live),
                        |_request| Ok::<_, Infallible>(GoToJobRoutePredicate::NonZero),
                        |_request| Ok::<_, Infallible>(GoToJobCommonMoverReturn::Zero),
                    )
                    .unwrap();
                    TestCallback::GoToJob(stage, result)
                }
                TestTask::Other(name) => {
                    events.borrow_mut().push(name);
                    TestCallback::Other
                }
            },
            |owner, visit, before, callback| {
                match (before, callback) {
                    (TestBefore::GoToJob(prefix), TestCallback::GoToJob(stage, result)) => {
                        let TestTask::GoToJob(state) = owner.task_state_mut(visit.task_id).unwrap()
                        else {
                            panic!("surviving task changed family without replacement");
                        };
                        stage.commit(state);
                        if go_to_job_transition_after_unwind(visit, prefix, result).is_some() {
                            events.borrow_mut().push("transition");
                            owner.replace_prepared(
                                ActorTaskSlot::Secondary,
                                PreparedActorTask::new(TestTask::Other("new secondary")),
                            );
                        }
                    }
                    (TestBefore::Other, TestCallback::Other) => {
                        events.borrow_mut().push("other post-unwind");
                    }
                    _ => panic!("dispatcher phase mismatch"),
                }
                ActorTaskVisitControl::<()>::Continue
            },
        );

        assert_eq!(
            events.into_inner(),
            [
                "go callback",
                "transition",
                "new secondary",
                "other post-unwind"
            ]
        );
    }
}
