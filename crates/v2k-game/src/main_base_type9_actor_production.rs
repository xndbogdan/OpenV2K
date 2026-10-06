//! Complete common-scheduler owner for the six Main Base Type-9 actors.
//!
//! The class-14 task bridge deliberately owns only the Primary callback. This
//! adapter restores the allocation's surrounding retail order:
//!
//! 1. preflight the normal scheduler, task, model, and E870 suffix without
//!    consuming the process RNG;
//! 2. consume and publish the common `FUN_00412DA0` scheduler prefix;
//! 3. run the class-14 task owner with the scheduler's capped callback delta;
//! 4. synchronously execute `FUN_0040E870` body-basis, environment,
//!    old-X/Z ground snap, and E370 surface phases;
//! 5. clear entity `+0xB2` after the callback and only then commit master
//!    motion.
//!
//! A scheduler wait commits only the scheduler prefix and retains the complete
//! owner unchanged. A callback always closes the outer transaction in the
//! same visit, including after class 14 has selected its terminal style and
//! staged deferred destruction. That terminal helper clears master-motion
//! state bits, so the later latched E870 suffix still runs while master motion
//! naturally performs no integration.
//!
//! The task owner's read-only preflight seam proves every task-local block
//! (including the private Running/PendingTransition stage) against the preview
//! callback delta before the real scheduler RNG is touched.

use std::num::NonZeroU64;

use crate::common_mover::type9_owner::{
    OrdinaryType9OwnerAction, OrdinaryType9OwnerCommitPhase, OrdinaryType9OwnerFrame,
    OrdinaryType9OwnerLease, OrdinaryType9OwnerPoll, OrdinaryType9OwnerResume,
    OrdinaryType9OwnerState, OrdinaryType9OwnerTransaction, OrdinaryType9OwnerTransactionId,
};
use crate::common_mover::type9_surface::{
    Type9SurfaceRuntime, ORDINARY_TYPE9_AUTHORED_LIFETIME_MS,
};
use crate::entity::{Entity, EntityManager};
use crate::entity_collision_state::{
    RetailRuntimeValue, RetailStateWord, ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
    REMOTE_OWNED_STATE_BIT,
};
use crate::entity_scheduler::{
    commit_common_scheduler_post_callback, commit_common_scheduler_prefix,
    common_scheduler_callback_mass, plan_common_scheduler_prefix, CommonSchedulerPrefixFlow,
    COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
};
use crate::main_base_type9_abort::{
    MainBaseType9ExplodingTaskLease, LEVEL_ONE_TYPE9_ENTITY_TYPE,
    LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS, LEVEL_ONE_TYPE9_MASS_RAW, LEVEL_ONE_TYPE9_MODEL_ID,
};
use crate::main_base_type9_production::{
    preflight_main_base_type9_exploding_owner, tick_main_base_type9_exploding_owner,
    MainBaseType9ExplodingProductionBlock, MainBaseType9ExplodingProductionDrop,
    MainBaseType9ExplodingProductionFrame, MainBaseType9ExplodingProductionOutcome,
    MainBaseType9ExplodingProductionOwner, MainBaseType9ExplodingTaskPreflightFailure,
};
use crate::resource_cache::ResourceCache;
use crate::retail_clock::RETAIL_FRAME_DELTA_MAX_US;
use crate::world_fx::{ParticleEnvironment, TerrainCollisionContext, WorldFx};

/// Model 558 Section-8 header `+0x08`; this is not Type-9's collision radius.
pub const LEVEL_ONE_TYPE9_MODEL_EXTENT_RAW: u16 = 165;

/// Exact state immediately after the Main Base abort transaction hands the
/// actor to the class-14 task/common-scheduler owner.
const LEVEL_ONE_TYPE9_POST_ABORT_STATE: u32 = 0x00C6_4825;

/// One outer manager visit. `elapsed_micros` feeds the common scheduler;
/// `global_elapsed_micros` remains retail `DAT_004D04E4`, independently used
/// by Type-9 Sub-D yaw integration after the scheduler selects its callback
/// delta.
#[derive(Clone, Copy)]
pub(crate) struct MainBaseType9ActorProductionFrame<'a> {
    pub resources: &'a ResourceCache,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
    pub retail_tick: u32,
}

/// Linear custody for both the task callback and its next outer E870 frame.
/// Deliberately neither `Clone` nor `Copy`.
#[derive(Debug, PartialEq, Eq)]
pub struct MainBaseType9ActorProductionOwner {
    task_owner: MainBaseType9ExplodingProductionOwner,
    next_outer_transaction_id: NonZeroU64,
}

impl MainBaseType9ActorProductionOwner {
    pub(crate) fn adopt(task_lease: MainBaseType9ExplodingTaskLease) -> Self {
        Self {
            task_owner: MainBaseType9ExplodingProductionOwner::adopt(task_lease),
            next_outer_transaction_id: NonZeroU64::MIN,
        }
    }

    pub(crate) const fn entity_id(&self) -> u32 {
        self.task_owner.entity_id()
    }

    pub(crate) const fn task_lease(&self) -> MainBaseType9ExplodingTaskLease {
        self.task_owner.task_lease()
    }

    /// Duplicate composite custody only for the isolated Main Base abort
    /// transaction.  The scheduler transaction keeps the live owner parked
    /// while this fork is evaluated against the forked EntityManager.
    pub(crate) const fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            task_owner: self.task_owner.fork_for_main_base_abort_transaction(),
            next_outer_transaction_id: self.next_outer_transaction_id,
        }
    }
}

/// Exact effects materialized inside one synchronous outer callback.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MainBaseType9ActorEffects {
    pub surface_bubbles_materialized: usize,
    pub surface_bubbles_dropped: usize,
    pub surface_sounds_queued: usize,
}

/// One composite owner visit selected by the heterogeneous live-list pass.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MainBaseType9ActorOwnerTick {
    pub outcome: MainBaseType9ExplodingProductionOutcome,
    pub retained_owner: Option<MainBaseType9ActorProductionOwner>,
    pub effects: MainBaseType9ActorEffects,
}

enum ActorPlanFailure {
    Blocked(MainBaseType9ExplodingProductionBlock),
    Dropped(MainBaseType9ExplodingProductionDrop),
}

impl From<MainBaseType9ExplodingProductionBlock> for ActorPlanFailure {
    fn from(value: MainBaseType9ExplodingProductionBlock) -> Self {
        Self::Blocked(value)
    }
}

#[derive(Clone, Copy)]
struct PreflightedActorFrame<'a> {
    callback_elapsed_micros: u32,
    callback_mass_raw: u16,
    terrain_collision: TerrainCollisionContext<'a>,
}

pub(crate) fn tick_main_base_type9_actor_owner_with_random(
    manager: &mut EntityManager,
    owner: MainBaseType9ActorProductionOwner,
    frame: MainBaseType9ActorProductionFrame<'_>,
    world_fx: &mut WorldFx,
    mut next_shared_random: impl FnMut(&mut WorldFx) -> u32,
) -> MainBaseType9ActorOwnerTick {
    let entity_id = owner.entity_id();
    let actor = owner.task_lease().actor();
    let preflight = match preflight_actor_frame(manager, &owner.task_owner, frame) {
        Ok(preflight) => preflight,
        Err(ActorPlanFailure::Blocked(reason)) => {
            return MainBaseType9ActorOwnerTick {
                outcome: MainBaseType9ExplodingProductionOutcome::Blocked { entity_id, reason },
                retained_owner: Some(owner),
                effects: MainBaseType9ActorEffects::default(),
            }
        }
        Err(ActorPlanFailure::Dropped(reason)) => {
            return MainBaseType9ActorOwnerTick {
                outcome: MainBaseType9ExplodingProductionOutcome::Dropped { entity_id, reason },
                retained_owner: None,
                effects: MainBaseType9ActorEffects::default(),
            }
        }
    };

    // Only now may the common scheduler consume its branch-local +0x70/+0x6C
    // words. Every task and E870 blocker has already been closed above.
    let scheduler_prefix = {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("the preflighted actor remains live synchronously");
        let mut random = || next_shared_random(&mut *world_fx);
        let RetailRuntimeValue::Known(prefix) =
            plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut random)
        else {
            unreachable!("the zero-RNG preflight authenticated scheduler state")
        };
        prefix
    };

    if scheduler_prefix.flow == CommonSchedulerPrefixFlow::WaitingAtCallbackGate {
        let entity = manager
            .main_base_type9_exploding_entity_mut(entity_id)
            .expect("the preflighted actor remains live synchronously");
        commit_common_scheduler_prefix(&mut entity.collision, scheduler_prefix);
        return MainBaseType9ActorOwnerTick {
            outcome: MainBaseType9ExplodingProductionOutcome::SchedulerWaiting { entity_id },
            retained_owner: Some(owner),
            effects: MainBaseType9ActorEffects::default(),
        };
    }

    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us,
    } = scheduler_prefix.flow
    else {
        unreachable!("the wait branch returned above")
    };
    debug_assert_eq!(callback_elapsed_us, preflight.callback_elapsed_micros);

    let MainBaseType9ActorProductionOwner {
        task_owner,
        next_outer_transaction_id,
    } = owner;
    let successor_outer_transaction_id = next_nonzero(next_outer_transaction_id);
    {
        let entity = manager
            .main_base_type9_exploding_entity_mut(entity_id)
            .expect("the preflighted actor remains live synchronously");
        commit_common_scheduler_prefix(&mut entity.collision, scheduler_prefix);
        entity.mass_raw = preflight.callback_mass_raw;
    }

    let task_tick = {
        let mut random = || next_shared_random(&mut *world_fx);
        tick_main_base_type9_exploding_owner(
            manager,
            task_owner,
            MainBaseType9ExplodingProductionFrame {
                scheduler_mode: 0,
                terrain: preflight.terrain_collision.terrain,
                elapsed_micros: callback_elapsed_us,
                global_elapsed_micros: frame.global_elapsed_micros,
            },
            &mut random,
        )
    };
    let task_outcome = task_tick.outcome;
    let retained_owner =
        task_tick
            .retained_owner
            .map(|task_owner| MainBaseType9ActorProductionOwner {
                task_owner,
                next_outer_transaction_id: successor_outer_transaction_id,
            });

    // Synthetic evidence blocks cannot be resumed after the scheduler prefix
    // has committed. The read-only task preflight makes this unreachable for
    // the authenticated production profile; retain returned custody only for
    // diagnosis rather than replaying the same callback inline.
    if matches!(
        &task_outcome,
        MainBaseType9ExplodingProductionOutcome::Blocked { .. }
            | MainBaseType9ExplodingProductionOutcome::Dropped { .. }
            | MainBaseType9ExplodingProductionOutcome::SchedulerWaiting { .. }
    ) {
        debug_assert!(false, "a preflighted task callback must not block or wait");
        return MainBaseType9ActorOwnerTick {
            outcome: task_outcome,
            retained_owner,
            effects: MainBaseType9ActorEffects::default(),
        };
    }

    let outer_initial = match snapshot_outer_state(manager, actor) {
        Ok(state) => state,
        Err(reason) => {
            return MainBaseType9ActorOwnerTick {
                outcome: MainBaseType9ExplodingProductionOutcome::Blocked { entity_id, reason },
                retained_owner,
                effects: MainBaseType9ActorEffects::default(),
            }
        }
    };
    let mut transaction = OrdinaryType9OwnerTransaction::start(
        OrdinaryType9OwnerTransactionId::new(next_outer_transaction_id),
        OrdinaryType9OwnerLease {
            controlled_entity_id: entity_id,
            frame_token: next_outer_transaction_id,
        },
        outer_initial,
        OrdinaryType9OwnerFrame {
            terrain: preflight.terrain_collision.terrain,
            effective_elapsed_micros: callback_elapsed_us,
            mode_zero_drag: crate::common_mover::type9_tail::Type9ModeZeroDrag {
                callback_mass_raw: std::num::NonZeroU16::new(preflight.callback_mass_raw)
                    .expect("preflight authenticates the nonzero captured callback mass"),
                strength: crate::common_mover::type9_tail::ORDINARY_TYPE9_DRAG_STRENGTH as u32,
            },
            active_model_extent_raw: LEVEL_ONE_TYPE9_MODEL_EXTENT_RAW,
            flat_surface_y_raw: preflight.terrain_collision.terrain.sea_level_raw(),
            authored_lifetime_ms: ORDINARY_TYPE9_AUTHORED_LIFETIME_MS,
        },
    );
    let outer = run_outer_transaction(
        manager,
        entity_id,
        actor,
        &mut transaction,
        preflight.terrain_collision,
        frame.retail_tick,
        world_fx,
        &mut next_shared_random,
    );

    match outer {
        Ok(effects) => MainBaseType9ActorOwnerTick {
            outcome: task_outcome,
            retained_owner,
            effects,
        },
        Err(reason) => MainBaseType9ActorOwnerTick {
            outcome: MainBaseType9ExplodingProductionOutcome::Blocked { entity_id, reason },
            retained_owner,
            effects: MainBaseType9ActorEffects::default(),
        },
    }
}

fn preflight_actor_frame<'a>(
    manager: &EntityManager,
    task_owner: &MainBaseType9ExplodingProductionOwner,
    frame: MainBaseType9ActorProductionFrame<'a>,
) -> Result<PreflightedActorFrame<'a>, ActorPlanFailure> {
    let entity_id = task_owner.entity_id();
    if frame.elapsed_micros > RETAIL_FRAME_DELTA_MAX_US {
        return Err(
            MainBaseType9ExplodingProductionBlock::ElapsedExceedsRetailCap {
                actual: frame.elapsed_micros,
            }
            .into(),
        );
    }
    if frame.global_elapsed_micros > RETAIL_FRAME_DELTA_MAX_US {
        return Err(
            MainBaseType9ExplodingProductionBlock::GlobalElapsedExceedsRetailCap {
                actual: frame.global_elapsed_micros,
            }
            .into(),
        );
    }

    let entity = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .ok_or(ActorPlanFailure::Dropped(
            MainBaseType9ExplodingProductionDrop::EntityUnavailable,
        ))?;
    let observation =
        manager
            .main_base_abort_actor_observation(entity_id)
            .ok_or(ActorPlanFailure::Dropped(
                MainBaseType9ExplodingProductionDrop::EntityUnavailable,
            ))?;
    if observation.lease != task_owner.task_lease().actor() {
        return Err(ActorPlanFailure::Dropped(
            MainBaseType9ExplodingProductionDrop::ActorLeaseMismatch {
                expected: observation.lease,
                actual: task_owner.task_lease().actor(),
            },
        ));
    }
    if observation.entity_type != LEVEL_ONE_TYPE9_ENTITY_TYPE {
        return Err(ActorPlanFailure::Dropped(
            MainBaseType9ExplodingProductionDrop::WrongEntityType {
                actual: observation.entity_type,
            },
        ));
    }

    preflight_normal_scheduler_owner(entity)?;
    let callback_mass_raw = preflight_callback_mass(entity)?;
    let terrain_collision = TerrainCollisionContext::from_current_level_cache(frame.resources)
        .ok_or(MainBaseType9ExplodingProductionBlock::OuterOwnerStateUnresolved)?;
    preflight_model_and_surface_profile(entity, frame.resources)?;

    // A zero threshold can never wait and therefore exposes the exact capped
    // callback delta without consuming the process stream or mutating state.
    let RetailRuntimeValue::Known(preview) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || 0)
    else {
        return Err(MainBaseType9ExplodingProductionBlock::SchedulerStateUnresolved.into());
    };
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us,
    } = preview.flow
    else {
        unreachable!("a zero scheduler threshold cannot wait")
    };

    // This seam inspects the task owner's private Running/PendingTransition
    // stage and dry-runs all callback-local planning. It performs no mutation
    // and consumes no RNG. Do not replace it with duplicated task logic here.
    let task_preflight = match preflight_main_base_type9_exploding_owner(
        manager,
        task_owner,
        MainBaseType9ExplodingProductionFrame {
            scheduler_mode: 0,
            terrain: terrain_collision.terrain,
            elapsed_micros: callback_elapsed_us,
            global_elapsed_micros: frame.global_elapsed_micros,
        },
    ) {
        Ok(preflight) => preflight,
        Err(MainBaseType9ExplodingTaskPreflightFailure::Blocked(reason)) => {
            return Err(ActorPlanFailure::Blocked(reason));
        }
        Err(MainBaseType9ExplodingTaskPreflightFailure::Dropped(reason)) => {
            return Err(ActorPlanFailure::Dropped(reason));
        }
    };

    let task_elapsed_ms = task_preflight.task_elapsed_ms;
    let RetailRuntimeValue::Known(surface_timer_ms) = entity.surface_lifetime_timer_ms_at_0x48
    else {
        return Err(MainBaseType9ExplodingProductionBlock::SurfaceLifetimeTimerUnresolved.into());
    };
    preflight_surface_lifetime(surface_timer_ms, task_elapsed_ms, callback_elapsed_us)?;
    snapshot_outer_state(manager, task_owner.task_lease().actor())?;

    Ok(PreflightedActorFrame {
        callback_elapsed_micros: callback_elapsed_us,
        callback_mass_raw,
        terrain_collision,
    })
}

fn preflight_normal_scheduler_owner(
    entity: &Entity,
) -> Result<(), MainBaseType9ExplodingProductionBlock> {
    match entity
        .collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT)
    {
        RetailRuntimeValue::Known(bits) if bits & REMOTE_OWNED_STATE_BIT != 0 => {
            return Err(MainBaseType9ExplodingProductionBlock::RemoteSchedulerOwnerUnsupported)
        }
        RetailRuntimeValue::Known(bits)
            if bits & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT == 0 =>
        {
            return Err(MainBaseType9ExplodingProductionBlock::SchedulerCallbackDisabled)
        }
        RetailRuntimeValue::Known(_) => {}
        RetailRuntimeValue::Unresolved => {
            return Err(MainBaseType9ExplodingProductionBlock::SchedulerStateUnresolved)
        }
    }
    match entity
        .collision
        .state_flags_at_0x08
        .masked(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT)
    {
        // The exact Main Base Type-9 profile enters coarse E870 with task mode
        // zero. Set routes DCA0 and is intentionally outside this adapter.
        RetailRuntimeValue::Known(0) => {}
        RetailRuntimeValue::Known(_) => {
            return Err(MainBaseType9ExplodingProductionBlock::DetailedViewOwnerUnsupported)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(MainBaseType9ExplodingProductionBlock::SchedulerStateUnresolved)
        }
    }
    match entity
        .collision
        .state_flags_at_0x08
        .masked(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT)
    {
        RetailRuntimeValue::Known(0) => {}
        RetailRuntimeValue::Known(_) => {
            return Err(MainBaseType9ExplodingProductionBlock::OwnerTransitionSuppressed)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(MainBaseType9ExplodingProductionBlock::OuterOwnerStateUnresolved)
        }
    }
    if entity.attached_to.is_some() {
        // The exact cohort is unattached. A future broader owner needs the
        // full relation-reconciliation phase before it may admit this path.
        return Err(
            MainBaseType9ExplodingProductionBlock::UnexpectedRelationAttachment {
                actual: entity.attached_to,
            },
        );
    }
    match entity.collision.recent_relation_id_at_0x60 {
        RetailRuntimeValue::Known(None) => {}
        RetailRuntimeValue::Known(actual) => {
            return Err(
                MainBaseType9ExplodingProductionBlock::UnexpectedRelationAttachment { actual },
            )
        }
        RetailRuntimeValue::Unresolved => {
            return Err(MainBaseType9ExplodingProductionBlock::OuterOwnerStateUnresolved)
        }
    }
    match entity.collision.state_flags_at_0x08.masked(u32::MAX) {
        RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_POST_ABORT_STATE) => {}
        RetailRuntimeValue::Known(_) | RetailRuntimeValue::Unresolved => {
            return Err(MainBaseType9ExplodingProductionBlock::SchedulerStateUnresolved)
        }
    }
    Ok(())
}

fn preflight_callback_mass(entity: &Entity) -> Result<u16, MainBaseType9ExplodingProductionBlock> {
    let RetailRuntimeValue::Known(callback_mass_raw) = common_scheduler_callback_mass(
        LEVEL_ONE_TYPE9_MASS_RAW,
        entity.collision.animation_offset_at_0xb2,
    ) else {
        return Err(MainBaseType9ExplodingProductionBlock::AnimationOffsetUnresolved);
    };
    if callback_mass_raw != LEVEL_ONE_TYPE9_MASS_RAW {
        return Err(
            MainBaseType9ExplodingProductionBlock::UnexpectedCallbackMass {
                actual: callback_mass_raw,
            },
        );
    }
    Ok(callback_mass_raw)
}

fn preflight_model_and_surface_profile(
    entity: &Entity,
    resources: &ResourceCache,
) -> Result<(), MainBaseType9ExplodingProductionBlock> {
    if entity.model_slots != [Some(LEVEL_ONE_TYPE9_MODEL_ID); 4]
        || entity.collision.active_model_slot() != RetailRuntimeValue::Known(1)
        || entity.model_in_slot(1) != Some(LEVEL_ONE_TYPE9_MODEL_ID)
        || entity.model_index != Some(LEVEL_ONE_TYPE9_MODEL_ID)
    {
        return Err(MainBaseType9ExplodingProductionBlock::ActiveModelMismatch);
    }
    let model = resources
        .global_model(LEVEL_ONE_TYPE9_MODEL_ID)
        .ok_or(MainBaseType9ExplodingProductionBlock::ActiveModelUnavailable)?;
    if model.radius != LEVEL_ONE_TYPE9_MODEL_EXTENT_RAW {
        return Err(MainBaseType9ExplodingProductionBlock::ActiveModelMismatch);
    }

    Ok(())
}

fn preflight_surface_lifetime(
    timer_ms: u32,
    task_elapsed_ms: u32,
    callback_elapsed_micros: u32,
) -> Result<(), MainBaseType9ExplodingProductionBlock> {
    if task_elapsed_ms > LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS {
        return Err(
            MainBaseType9ExplodingProductionBlock::SurfaceLifetimeEscapesClass14 {
                maximum_timer_ms: task_elapsed_ms,
            },
        );
    }
    if timer_ms > task_elapsed_ms {
        return Err(
            MainBaseType9ExplodingProductionBlock::SurfaceLifetimeInvariantMismatch {
                timer_ms,
                task_elapsed_ms,
            },
        );
    }
    let callback_elapsed_ms = callback_elapsed_micros / 1_000;
    let Some(maximum_timer_ms) = timer_ms.checked_add(callback_elapsed_ms) else {
        return Err(
            MainBaseType9ExplodingProductionBlock::SurfaceLifetimeEscapesClass14 {
                maximum_timer_ms: u32::MAX,
            },
        );
    };
    let Some(task_elapsed_after) = task_elapsed_ms.checked_add(callback_elapsed_ms) else {
        return Err(
            MainBaseType9ExplodingProductionBlock::SurfaceLifetimeEscapesClass14 {
                maximum_timer_ms: u32::MAX,
            },
        );
    };
    if maximum_timer_ms > task_elapsed_after {
        return Err(
            MainBaseType9ExplodingProductionBlock::SurfaceLifetimeInvariantMismatch {
                timer_ms: maximum_timer_ms,
                task_elapsed_ms: task_elapsed_after,
            },
        );
    }
    let maximum_class14_surface_timer_ms = LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS
        .checked_add(RETAIL_FRAME_DELTA_MAX_US / 1_000)
        .expect("the authenticated class-14 lifetime bound fits u32");
    if maximum_timer_ms > maximum_class14_surface_timer_ms {
        return Err(
            MainBaseType9ExplodingProductionBlock::SurfaceLifetimeEscapesClass14 {
                maximum_timer_ms,
            },
        );
    }
    debug_assert!(LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS < ORDINARY_TYPE9_AUTHORED_LIFETIME_MS);
    Ok(())
}

fn snapshot_outer_state(
    manager: &EntityManager,
    actor: crate::main_base_abort::MainBaseAbortActorLease,
) -> Result<OrdinaryType9OwnerState, MainBaseType9ExplodingProductionBlock> {
    let Some(observation) = manager.main_base_abort_actor_observation(actor.entity_id) else {
        return Err(MainBaseType9ExplodingProductionBlock::OuterOwnerStateUnresolved);
    };
    if observation.lease != actor {
        return Err(MainBaseType9ExplodingProductionBlock::OuterOwnerStateUnresolved);
    }
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == actor.entity_id)
        .ok_or(MainBaseType9ExplodingProductionBlock::OuterOwnerStateUnresolved)?;
    let RetailRuntimeValue::Known(state_flags) =
        entity.collision.state_flags_at_0x08.masked(u32::MAX)
    else {
        return Err(MainBaseType9ExplodingProductionBlock::OuterOwnerStateUnresolved);
    };
    let RetailRuntimeValue::Known(body_basis) = entity.physical_body_basis_q31 else {
        return Err(MainBaseType9ExplodingProductionBlock::PhysicalBodyBasisUnresolved);
    };
    let RetailRuntimeValue::Known(lifetime_timer_ms_at_0x48) =
        entity.surface_lifetime_timer_ms_at_0x48
    else {
        return Err(MainBaseType9ExplodingProductionBlock::SurfaceLifetimeTimerUnresolved);
    };
    let [heading_raw, pitch_raw, roll_raw] = entity.rotation_heading_pitch_roll_raw();
    Ok(OrdinaryType9OwnerState {
        state_flags,
        position_raw: entity.position_raw(),
        velocity_raw: entity.velocity_raw(),
        heading_raw,
        pitch_raw,
        roll_raw,
        body_basis,
        surface_runtime: Type9SurfaceRuntime {
            lifetime_timer_ms_at_0x48,
        },
    })
}

fn commit_outer_state(
    manager: &mut EntityManager,
    actor: crate::main_base_abort::MainBaseAbortActorLease,
    before: OrdinaryType9OwnerState,
    after: OrdinaryType9OwnerState,
) -> Option<OrdinaryType9OwnerState> {
    if snapshot_outer_state(manager, actor).ok()? != before {
        return None;
    }
    let entity = manager.main_base_type9_exploding_entity_mut(actor.entity_id)?;
    entity.set_motion_raw(after.position_raw, after.velocity_raw);
    entity.set_rotation_heading_pitch_roll_raw([
        after.heading_raw,
        after.pitch_raw,
        after.roll_raw,
    ]);
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(after.body_basis);
    entity.surface_lifetime_timer_ms_at_0x48 =
        RetailRuntimeValue::Known(after.surface_runtime.lifetime_timer_ms_at_0x48);
    // FUN_00413F70 publishes the rebuilt matrix before setting its retained
    // completion bit. Keeping the exact state word last preserves that order
    // for the attitude phase and is also valid for the later suffix commits.
    entity.collision.state_flags_at_0x08 = RetailStateWord::exact(after.state_flags);
    Some(after)
}

#[allow(clippy::too_many_arguments)]
fn run_outer_transaction(
    manager: &mut EntityManager,
    entity_id: u32,
    actor: crate::main_base_abort::MainBaseAbortActorLease,
    transaction: &mut OrdinaryType9OwnerTransaction,
    terrain_collision: TerrainCollisionContext<'_>,
    retail_tick: u32,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Result<MainBaseType9ActorEffects, MainBaseType9ExplodingProductionBlock> {
    let mut effects = MainBaseType9ActorEffects::default();
    loop {
        match transaction.poll() {
            OrdinaryType9OwnerPoll::Action(issued) => match issued.action {
                OrdinaryType9OwnerAction::CommitState {
                    phase,
                    before,
                    after,
                } => {
                    if phase == OrdinaryType9OwnerCommitPhase::MasterMotion {
                        let entity = manager
                            .main_base_type9_exploding_entity_mut(entity_id)
                            .ok_or(
                                MainBaseType9ExplodingProductionBlock::OuterOwnerStateUnresolved,
                            )?;
                        // Retail's unconditional post-callback +0xB2 clear is
                        // observable before the final master-motion reread.
                        commit_common_scheduler_post_callback(&mut entity.collision);
                    }
                    let Some(committed) = commit_outer_state(manager, actor, before, after) else {
                        transaction
                            .resume(
                                issued.receipt,
                                OrdinaryType9OwnerResume::Blocked {
                                    reason: crate::common_mover::type9_owner::OrdinaryType9OwnerExternalBlock::StateCommitUnavailable,
                                },
                            )
                            .expect("the adapter owns the current outer receipt");
                        continue;
                    };
                    transaction
                        .resume(
                            issued.receipt,
                            OrdinaryType9OwnerResume::StateCommitted { committed },
                        )
                        .expect("the adapter acknowledges the exact planned state");
                }
                OrdinaryType9OwnerAction::PlanSurface { .. } => {
                    let mut random = || next_shared_random(&mut *world_fx);
                    transaction
                        .plan_surface(issued.receipt, &mut random)
                        .expect("the adapter owns the current surface receipt");
                }
                OrdinaryType9OwnerAction::SpawnBubble(request) => {
                    if world_fx
                        .materialize_actor_surface_bubble_request(
                            request,
                            ParticleEnvironment::Terrain(terrain_collision),
                            retail_tick,
                        )
                        .is_some()
                    {
                        effects.surface_bubbles_materialized += 1;
                    } else {
                        effects.surface_bubbles_dropped += 1;
                    }
                    let fresh_state = snapshot_outer_state(manager, actor)?;
                    transaction
                        .resume(
                            issued.receipt,
                            OrdinaryType9OwnerResume::EffectCompleted { fresh_state },
                        )
                        .expect("the adapter owns the current bubble receipt");
                }
                OrdinaryType9OwnerAction::PlaySound(request) => {
                    world_fx
                        .queue_fixed_positional_sound_raw(request.sound_id, request.position_raw);
                    effects.surface_sounds_queued += 1;
                    let fresh_state = snapshot_outer_state(manager, actor)?;
                    transaction
                        .resume(
                            issued.receipt,
                            OrdinaryType9OwnerResume::EffectCompleted { fresh_state },
                        )
                        .expect("the adapter owns the current sound receipt");
                }
                OrdinaryType9OwnerAction::RunLifecycleContinuation(_) => {
                    // Surface/task clock custody proves the 5,000-ms lifecycle
                    // cannot be reached by a class-14 owner whose strict
                    // terminal threshold is 1,000 ms.
                    transaction
                        .resume(
                            issued.receipt,
                            OrdinaryType9OwnerResume::Blocked {
                                reason: crate::common_mover::type9_owner::OrdinaryType9OwnerExternalBlock::LifecycleContinuationUnavailable,
                            },
                        )
                        .expect("the adapter owns the impossible lifecycle receipt");
                }
            },
            OrdinaryType9OwnerPoll::Complete(completion) => {
                debug_assert_eq!(completion.lease.controlled_entity_id, entity_id);
                return Ok(effects);
            }
            OrdinaryType9OwnerPoll::Blocked(block) => {
                return Err(MainBaseType9ExplodingProductionBlock::OuterOwner(block))
            }
            OrdinaryType9OwnerPoll::Awaiting => {
                unreachable!("the synchronous adapter never abandons an issued receipt")
            }
        }
    }
}

const fn next_nonzero(value: NonZeroU64) -> NonZeroU64 {
    match NonZeroU64::new(value.get().wrapping_add(1)) {
        Some(value) => value,
        None => NonZeroU64::MIN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_clock_cannot_outrun_the_class14_task() {
        assert!(preflight_surface_lifetime(500, 500, 125_000).is_ok());
        assert_eq!(
            preflight_surface_lifetime(501, 500, 0),
            Err(
                MainBaseType9ExplodingProductionBlock::SurfaceLifetimeInvariantMismatch {
                    timer_ms: 501,
                    task_elapsed_ms: 500,
                }
            )
        );
    }

    #[test]
    fn lifecycle_escape_is_rejected_before_any_owner_rng() {
        assert_eq!(
            preflight_surface_lifetime(1_000, 1_001, 0),
            Err(
                MainBaseType9ExplodingProductionBlock::SurfaceLifetimeEscapesClass14 {
                    maximum_timer_ms: 1_001,
                }
            )
        );
        assert!(preflight_surface_lifetime(1_000, 1_000, 125_000).is_ok());
        assert_eq!(
            preflight_surface_lifetime(1_000, 1_000, 126_000),
            Err(
                MainBaseType9ExplodingProductionBlock::SurfaceLifetimeEscapesClass14 {
                    maximum_timer_ms: 1_126,
                }
            )
        );
    }

    #[test]
    fn outer_transaction_sequence_promotes_wrapped_zero() {
        assert_eq!(next_nonzero(NonZeroU64::MIN).get(), 2);
        assert_eq!(next_nonzero(NonZeroU64::MAX), NonZeroU64::MIN);
    }
}
