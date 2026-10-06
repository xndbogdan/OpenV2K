//! Shared post-F70 Type-9 outer tail for selected production owners.
//!
//! Retail `FUN_00412DA0` continues from the already-published `FUN_00413F70`
//! basis through E100, DF70, E370, an unconditional `+0xB2` clear, and master
//! motion. This module owns that adapter transaction. It does not invent
//! first-callback `+0xB2` mass residue. E370 `RunLifecycleContinuation` runs
//! the generic Type-9 `FUN_00416750` prefix plus class-14 publisher when the
//! selected style's release hook is null and the actor is unattached.

use std::num::NonZeroU64;

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    common_mover::{
        type9_owner::{
            IssuedOrdinaryType9OwnerAction, OrdinaryType9OwnerAction,
            OrdinaryType9OwnerCommitPhase, OrdinaryType9OwnerFrame, OrdinaryType9OwnerLease,
            OrdinaryType9OwnerPoll, OrdinaryType9OwnerResume, OrdinaryType9OwnerState,
            OrdinaryType9OwnerTransaction, OrdinaryType9OwnerTransactionId,
        },
        type9_surface::{Type9SurfaceRuntime, ORDINARY_TYPE9_AUTHORED_LIFETIME_MS},
    },
    entity::EntityManager,
    entity_collision_state::{RetailRuntimeValue, SURFACE_STATE_MASK},
    entity_scheduler::{
        commit_common_scheduler_post_callback, commit_common_scheduler_prefix,
        common_scheduler_callback_mass, plan_common_scheduler_prefix, CommonSchedulerPrefixFlow,
        SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    },
    entity_view_detail::{VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT, VIEW_DETAIL_STATE_MASK},
    main_base_abort::MainBaseAbortActorLease,
    main_base_type9_abort::{
        MainBaseType9ExplodingTaskLease, MainBaseType9ResultScreenState,
        LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS, LEVEL_ONE_TYPE9_MASS_RAW, LEVEL_ONE_TYPE9_MODEL_ID,
    },
    main_base_type9_actor_production::LEVEL_ONE_TYPE9_MODEL_EXTENT_RAW,
    ordinary_type9_standard_death::{
        type9_entity_has_class14_publication, OrdinaryType9StandardDeathEntry,
        OrdinaryType9StandardDeathOutcome,
    },
    resource_cache::ResourceCache,
    world_fx::{ParticleEnvironment, TerrainCollisionContext, WorldFx},
};

// 129B0 owns these two surface-classification bits in the later collision
// scan. E100/DF70/E370/master neither read nor write them. In particular E370's
// nonzero-state sound gate is already true because F70 has set known bit4.
// Snapshot and commit every other bit strictly, preserving this independent
// domain (including unresolved constructor state) through the outer tail.
const TYPE9_OUTER_STATE_MASK: u32 = !SURFACE_STATE_MASK;

/// Linear custody for one in-flight or completed outer tail.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OrdinaryType9OuterTailCustody {
    Transaction {
        transaction: OrdinaryType9OwnerTransaction,
        block: crate::common_mover::type9_owner::OrdinaryType9OwnerBlock,
        origin_retail_tick: u32,
        expected_state: OrdinaryType9OwnerState,
        expected_animation_offset_at_0xb2: RetailRuntimeValue<u16>,
    },
    LifecyclePending {
        transaction: OrdinaryType9OwnerTransaction,
        issued: IssuedOrdinaryType9OwnerAction,
        origin_retail_tick: u32,
        expected_state: OrdinaryType9OwnerState,
        expected_animation_offset_at_0xb2: RetailRuntimeValue<u16>,
    },
    BubblePending {
        transaction: OrdinaryType9OwnerTransaction,
        issued: IssuedOrdinaryType9OwnerAction,
        origin_retail_tick: u32,
        expected_state: OrdinaryType9OwnerState,
        expected_animation_offset_at_0xb2: RetailRuntimeValue<u16>,
    },
    Complete {
        expected_state: OrdinaryType9OwnerState,
        expected_animation_offset_at_0xb2: RetailRuntimeValue<u16>,
    },
}

impl OrdinaryType9OuterTailCustody {
    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        match self {
            Self::Transaction {
                transaction,
                block,
                origin_retail_tick,
                expected_state,
                expected_animation_offset_at_0xb2,
            } => Self::Transaction {
                transaction: transaction.fork_for_main_base_abort_transaction(),
                block: *block,
                origin_retail_tick: *origin_retail_tick,
                expected_state: *expected_state,
                expected_animation_offset_at_0xb2: *expected_animation_offset_at_0xb2,
            },
            Self::LifecyclePending {
                transaction,
                issued,
                origin_retail_tick,
                expected_state,
                expected_animation_offset_at_0xb2,
            } => Self::LifecyclePending {
                transaction: transaction.fork_for_main_base_abort_transaction(),
                issued: issued.fork_for_main_base_abort_transaction(),
                origin_retail_tick: *origin_retail_tick,
                expected_state: *expected_state,
                expected_animation_offset_at_0xb2: *expected_animation_offset_at_0xb2,
            },
            Self::BubblePending {
                transaction,
                issued,
                origin_retail_tick,
                expected_state,
                expected_animation_offset_at_0xb2,
            } => Self::BubblePending {
                transaction: transaction.fork_for_main_base_abort_transaction(),
                issued: issued.fork_for_main_base_abort_transaction(),
                origin_retail_tick: *origin_retail_tick,
                expected_state: *expected_state,
                expected_animation_offset_at_0xb2: *expected_animation_offset_at_0xb2,
            },
            Self::Complete {
                expected_state,
                expected_animation_offset_at_0xb2,
            } => Self::Complete {
                expected_state: *expected_state,
                expected_animation_offset_at_0xb2: *expected_animation_offset_at_0xb2,
            },
        }
    }

    pub(crate) fn expected_observation(
        &self,
    ) -> (OrdinaryType9OwnerState, RetailRuntimeValue<u16>) {
        match self {
            Self::Transaction {
                expected_state,
                expected_animation_offset_at_0xb2,
                ..
            }
            | Self::LifecyclePending {
                expected_state,
                expected_animation_offset_at_0xb2,
                ..
            }
            | Self::BubblePending {
                expected_state,
                expected_animation_offset_at_0xb2,
                ..
            }
            | Self::Complete {
                expected_state,
                expected_animation_offset_at_0xb2,
            } => (*expected_state, *expected_animation_offset_at_0xb2),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrdinaryType9OuterTailBlock {
    CurrentTerrainUnavailable,
    ActiveModelUnavailable,
    OuterOwnerStateUnavailable,
    OuterOwner(crate::common_mover::type9_owner::OrdinaryType9OwnerBlock),
    OuterLifecyclePending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrdinaryType9OuterTailStartError {
    CurrentTerrainUnavailable,
    ActiveModelUnavailable,
    OuterOwnerStateUnavailable,
    PostBasisPublicationMismatch,
}

impl From<OrdinaryType9OuterTailStartError> for OrdinaryType9OuterTailBlock {
    fn from(error: OrdinaryType9OuterTailStartError) -> Self {
        match error {
            OrdinaryType9OuterTailStartError::CurrentTerrainUnavailable => {
                Self::CurrentTerrainUnavailable
            }
            OrdinaryType9OuterTailStartError::ActiveModelUnavailable => {
                Self::ActiveModelUnavailable
            }
            OrdinaryType9OuterTailStartError::OuterOwnerStateUnavailable => {
                Self::OuterOwnerStateUnavailable
            }
            OrdinaryType9OuterTailStartError::PostBasisPublicationMismatch => {
                unreachable!("start mismatch is a drop, not a parked block")
            }
        }
    }
}

pub(crate) enum OrdinaryType9OuterTailDrive {
    Complete(OrdinaryType9OwnerState),
    Blocked {
        custody: OrdinaryType9OuterTailCustody,
        reason: OrdinaryType9OuterTailBlock,
    },
    StateMismatch,
}

pub(crate) fn snapshot_outer_tail_state(
    manager: &EntityManager,
    actor_lease: MainBaseAbortActorLease,
) -> Option<OrdinaryType9OwnerState> {
    if manager.ordinary_type9_selected_actor_lease(actor_lease.entity_id) != Some(actor_lease) {
        return None;
    }
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == actor_lease.entity_id)?;
    let RetailRuntimeValue::Known(state_flags) = entity
        .collision
        .state_flags_at_0x08
        .masked(TYPE9_OUTER_STATE_MASK)
    else {
        return None;
    };
    let RetailRuntimeValue::Known(body_basis) = entity.physical_body_basis_q31 else {
        return None;
    };
    let RetailRuntimeValue::Known(lifetime_timer_ms_at_0x48) =
        entity.surface_lifetime_timer_ms_at_0x48
    else {
        return None;
    };
    let [heading_raw, pitch_raw, roll_raw] = entity.rotation_heading_pitch_roll_raw();
    Some(OrdinaryType9OwnerState {
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

pub(crate) fn snapshot_outer_tail_animation_offset(
    manager: &EntityManager,
    actor_lease: MainBaseAbortActorLease,
) -> Option<RetailRuntimeValue<u16>> {
    if manager.ordinary_type9_selected_actor_lease(actor_lease.entity_id) != Some(actor_lease) {
        return None;
    }
    manager
        .iter_all()
        .find(|entity| entity.id == actor_lease.entity_id)
        .map(|entity| entity.collision.animation_offset_at_0xb2)
}

pub(crate) fn outer_tail_entity_published_class14(
    manager: &EntityManager,
    actor_lease: MainBaseAbortActorLease,
) -> bool {
    manager
        .iter_all()
        .find(|entity| entity.id == actor_lease.entity_id)
        .is_some_and(type9_entity_has_class14_publication)
}

/// Reconstruct the class-14 Primary lease after selected-owner E370 consume.
///
/// Initializer fallback publishes class 14 without a Primary, so this stays
/// `None` instead of inventing a Main Base abort exploding owner.
pub(crate) fn outer_tail_class14_exploding_task_lease(
    manager: &EntityManager,
    actor_lease: MainBaseAbortActorLease,
) -> Option<MainBaseType9ExplodingTaskLease> {
    if !outer_tail_entity_published_class14(manager, actor_lease) {
        return None;
    }
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == actor_lease.entity_id)?;
    let task_id = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary)?;
    match entity.actor_tasks.task_state(task_id) {
        Some(ActorTaskRuntime::SharedRetarget(task))
            if task.lifetime_ms() == LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS =>
        {
            Some(MainBaseType9ExplodingTaskLease::issue(actor_lease, task_id))
        }
        _ => None,
    }
}

pub(crate) fn outer_tail_observation_authenticates(
    manager: &EntityManager,
    actor_lease: MainBaseAbortActorLease,
    custody: &OrdinaryType9OuterTailCustody,
) -> bool {
    let (expected_state, expected_animation_offset_at_0xb2) = custody.expected_observation();
    let Some(mut current_state) = snapshot_outer_tail_state(manager, actor_lease) else {
        return false;
    };
    if matches!(custody, OrdinaryType9OuterTailCustody::Complete { .. })
        && expected_state.state_flags & VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT != 0
    {
        // FUN_00411400 runs after the completed FUN_00412DA0 visit and owns
        // these two bits for the next callback. Accept only that field change;
        // pose, movement, lifecycle, every other state bit and +B2 stay exact.
        // In-flight plans keep the full comparison: they must not change their
        // frozen callback mode or overwrite a later presentation publication.
        current_state.state_flags = (current_state.state_flags & !VIEW_DETAIL_STATE_MASK)
            | (expected_state.state_flags & VIEW_DETAIL_STATE_MASK);
    }
    current_state == expected_state
        && snapshot_outer_tail_animation_offset(manager, actor_lease)
            == Some(expected_animation_offset_at_0xb2)
}

fn commit_outer_tail_state(
    manager: &mut EntityManager,
    actor_lease: MainBaseAbortActorLease,
    phase: OrdinaryType9OwnerCommitPhase,
    before: OrdinaryType9OwnerState,
    before_animation_offset_at_0xb2: RetailRuntimeValue<u16>,
    after: OrdinaryType9OwnerState,
) -> Option<OrdinaryType9OwnerState> {
    // Authenticate the complete owner-visible state before retail's
    // unconditional +0xB2 clear can make the final phase observable.
    if snapshot_outer_tail_state(manager, actor_lease)? != before
        || snapshot_outer_tail_animation_offset(manager, actor_lease)?
            != before_animation_offset_at_0xb2
    {
        return None;
    }
    let entity = manager.ordinary_type9_selected_entity_mut(actor_lease.entity_id)?;
    if phase == OrdinaryType9OwnerCommitPhase::MasterMotion {
        commit_common_scheduler_post_callback(&mut entity.collision);
    }
    entity.set_motion_raw(after.position_raw, after.velocity_raw);
    entity.set_rotation_heading_pitch_roll_raw([
        after.heading_raw,
        after.pitch_raw,
        after.roll_raw,
    ]);
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(after.body_basis);
    entity.surface_lifetime_timer_ms_at_0x48 =
        RetailRuntimeValue::Known(after.surface_runtime.lifetime_timer_ms_at_0x48);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(TYPE9_OUTER_STATE_MASK, after.state_flags);
    if phase == OrdinaryType9OwnerCommitPhase::MasterMotion
        && entity.collision.animation_offset_at_0xb2 != RetailRuntimeValue::Known(0)
    {
        return None;
    }
    Some(after)
}

pub(crate) fn take_outer_transaction_id(
    next_transaction_id: &mut u64,
) -> OrdinaryType9OwnerTransactionId {
    let raw = (*next_transaction_id).max(1);
    *next_transaction_id = raw.wrapping_add(1).max(1);
    OrdinaryType9OwnerTransactionId::new(NonZeroU64::new(raw).expect("nonzero by construction"))
}

fn mode_zero_drag_for_actor(
    manager: &EntityManager,
    entity_id: u32,
) -> Result<crate::common_mover::type9_tail::Type9ModeZeroDrag, OrdinaryType9OuterTailStartError> {
    let (wind_mode, strength) = manager.intro2_type13_environment();
    if wind_mode != 0 {
        return Err(OrdinaryType9OuterTailStartError::OuterOwnerStateUnavailable);
    }
    let callback_mass_raw = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .and_then(|entity| std::num::NonZeroU16::new(entity.mass_raw))
        .ok_or(OrdinaryType9OuterTailStartError::OuterOwnerStateUnavailable)?;
    Ok(crate::common_mover::type9_tail::Type9ModeZeroDrag {
        callback_mass_raw,
        strength,
    })
}

pub(crate) fn start_outer_tail_transaction(
    manager: &EntityManager,
    actor_lease: MainBaseAbortActorLease,
    transaction_id: OrdinaryType9OwnerTransactionId,
    resources: &ResourceCache,
    callback_elapsed_micros: u32,
) -> Result<
    (OrdinaryType9OwnerTransaction, RetailRuntimeValue<u16>),
    OrdinaryType9OuterTailStartError,
> {
    let Some(terrain) = resources.level_terrain() else {
        return Err(OrdinaryType9OuterTailStartError::CurrentTerrainUnavailable);
    };
    let Some(model_id) = manager
        .iter_all()
        .find(|entity| entity.id == actor_lease.entity_id)
        .and_then(|entity| entity.model_index)
    else {
        return Err(OrdinaryType9OuterTailStartError::ActiveModelUnavailable);
    };
    // The adopted fresh-Level-1 provenance authenticates retail's exact
    // Section-12 row: all four model slots are model 558. Its original
    // Section-8 header +0x08 is the surface extent 165 (not +0x0A radius),
    // so an absent optional model corpus does not make this fixed profile
    // unresolved; any loaded contradictory header still fails closed.
    if model_id != LEVEL_ONE_TYPE9_MODEL_ID {
        return Err(OrdinaryType9OuterTailStartError::ActiveModelUnavailable);
    }
    if resources
        .global_model(model_id)
        .is_some_and(|model| model.radius != LEVEL_ONE_TYPE9_MODEL_EXTENT_RAW)
    {
        return Err(OrdinaryType9OuterTailStartError::ActiveModelUnavailable);
    }
    let Some(initial) = snapshot_outer_tail_state(manager, actor_lease) else {
        return Err(OrdinaryType9OuterTailStartError::OuterOwnerStateUnavailable);
    };
    let Some(initial_animation_offset_at_0xb2) =
        snapshot_outer_tail_animation_offset(manager, actor_lease)
    else {
        return Err(OrdinaryType9OuterTailStartError::OuterOwnerStateUnavailable);
    };
    let owner_lease = OrdinaryType9OwnerLease {
        controlled_entity_id: actor_lease.entity_id,
        frame_token: NonZeroU64::new(transaction_id.get()).expect("transaction id is nonzero"),
    };
    let transaction = OrdinaryType9OwnerTransaction::start_after_body_basis(
        transaction_id,
        owner_lease,
        initial,
        OrdinaryType9OwnerFrame {
            terrain,
            effective_elapsed_micros: callback_elapsed_micros,
            mode_zero_drag: mode_zero_drag_for_actor(manager, actor_lease.entity_id)?,
            active_model_extent_raw: LEVEL_ONE_TYPE9_MODEL_EXTENT_RAW,
            flat_surface_y_raw: terrain.sea_level_raw(),
            authored_lifetime_ms: ORDINARY_TYPE9_AUTHORED_LIFETIME_MS,
        },
    )
    .map_err(|_| OrdinaryType9OuterTailStartError::PostBasisPublicationMismatch)?;
    Ok((transaction, initial_animation_offset_at_0xb2))
}

/// Class-14 `FUN_00412DA0` prefix before the exploding callback.
///
/// `20260817-072421` INSTALL flags `0x06C64825` have wait-disable `0x02000000`
/// set, so the random wait gates are skipped. This does not require Main Base
/// post-abort `0x00C64825`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrdinaryType9Class14SchedulerPrefix {
    Waiting,
    Continue { callback_elapsed_us: u32 },
}

pub(crate) fn apply_class14_scheduler_prefix(
    manager: &mut EntityManager,
    actor_lease: MainBaseAbortActorLease,
    elapsed_micros: u32,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Result<OrdinaryType9Class14SchedulerPrefix, OrdinaryType9OuterTailStartError> {
    if manager.ordinary_type9_selected_actor_lease(actor_lease.entity_id) != Some(actor_lease) {
        return Err(OrdinaryType9OuterTailStartError::OuterOwnerStateUnavailable);
    }
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == actor_lease.entity_id)
        .ok_or(OrdinaryType9OuterTailStartError::OuterOwnerStateUnavailable)?;
    let wait_disabled = match entity
        .collision
        .state_flags_at_0x08
        .masked(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT)
    {
        RetailRuntimeValue::Known(0) => false,
        RetailRuntimeValue::Known(_) => true,
        RetailRuntimeValue::Unresolved => {
            return Err(OrdinaryType9OuterTailStartError::OuterOwnerStateUnavailable)
        }
    };
    let mass_before_prefix = if wait_disabled {
        match common_scheduler_callback_mass(
            LEVEL_ONE_TYPE9_MASS_RAW,
            entity.collision.animation_offset_at_0xb2,
        ) {
            RetailRuntimeValue::Known(value) => Some(value),
            RetailRuntimeValue::Unresolved => {
                return Err(OrdinaryType9OuterTailStartError::OuterOwnerStateUnavailable)
            }
        }
    } else {
        None
    };
    let mut random = || next_shared_random(world_fx);
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, elapsed_micros, &mut random)
    else {
        return Err(OrdinaryType9OuterTailStartError::OuterOwnerStateUnavailable);
    };
    let entity = manager
        .ordinary_type9_selected_entity_mut(actor_lease.entity_id)
        .ok_or(OrdinaryType9OuterTailStartError::OuterOwnerStateUnavailable)?;
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    match prefix.flow {
        CommonSchedulerPrefixFlow::WaitingAtCallbackGate => {
            Ok(OrdinaryType9Class14SchedulerPrefix::Waiting)
        }
        CommonSchedulerPrefixFlow::Continue {
            callback_elapsed_us,
        } => {
            let mass = match mass_before_prefix {
                Some(value) => value,
                None => match common_scheduler_callback_mass(
                    LEVEL_ONE_TYPE9_MASS_RAW,
                    entity.collision.animation_offset_at_0xb2,
                ) {
                    RetailRuntimeValue::Known(value) => value,
                    RetailRuntimeValue::Unresolved => {
                        return Err(OrdinaryType9OuterTailStartError::OuterOwnerStateUnavailable)
                    }
                },
            };
            entity.mass_raw = mass;
            Ok(OrdinaryType9Class14SchedulerPrefix::Continue {
                callback_elapsed_us,
            })
        }
    }
}

/// Post-class-14 E870 suffix: F70 then E100/DF70/E370/+0xB2/master.
///
/// This is the same ordinary Type-9 latched-`0x2F` callback used by selected
/// owners. It does not require Main Base post-abort `0x00C64825`.
pub(crate) fn start_class14_suffix_transaction(
    manager: &EntityManager,
    actor_lease: MainBaseAbortActorLease,
    transaction_id: OrdinaryType9OwnerTransactionId,
    resources: &ResourceCache,
    callback_elapsed_micros: u32,
) -> Result<
    (OrdinaryType9OwnerTransaction, RetailRuntimeValue<u16>),
    OrdinaryType9OuterTailStartError,
> {
    let Some(terrain) = resources.level_terrain() else {
        return Err(OrdinaryType9OuterTailStartError::CurrentTerrainUnavailable);
    };
    let Some(model_id) = manager
        .iter_all()
        .find(|entity| entity.id == actor_lease.entity_id)
        .and_then(|entity| entity.model_index)
    else {
        return Err(OrdinaryType9OuterTailStartError::ActiveModelUnavailable);
    };
    if model_id != LEVEL_ONE_TYPE9_MODEL_ID {
        return Err(OrdinaryType9OuterTailStartError::ActiveModelUnavailable);
    }
    if resources
        .global_model(model_id)
        .is_some_and(|model| model.radius != LEVEL_ONE_TYPE9_MODEL_EXTENT_RAW)
    {
        return Err(OrdinaryType9OuterTailStartError::ActiveModelUnavailable);
    }
    let Some(initial) = snapshot_outer_tail_state(manager, actor_lease) else {
        return Err(OrdinaryType9OuterTailStartError::OuterOwnerStateUnavailable);
    };
    let Some(initial_animation_offset_at_0xb2) =
        snapshot_outer_tail_animation_offset(manager, actor_lease)
    else {
        return Err(OrdinaryType9OuterTailStartError::OuterOwnerStateUnavailable);
    };
    let owner_lease = OrdinaryType9OwnerLease {
        controlled_entity_id: actor_lease.entity_id,
        frame_token: NonZeroU64::new(transaction_id.get()).expect("transaction id is nonzero"),
    };
    let transaction = OrdinaryType9OwnerTransaction::start(
        transaction_id,
        owner_lease,
        initial,
        OrdinaryType9OwnerFrame {
            terrain,
            effective_elapsed_micros: callback_elapsed_micros,
            mode_zero_drag: mode_zero_drag_for_actor(manager, actor_lease.entity_id)?,
            active_model_extent_raw: LEVEL_ONE_TYPE9_MODEL_EXTENT_RAW,
            flat_surface_y_raw: terrain.sea_level_raw(),
            authored_lifetime_ms: ORDINARY_TYPE9_AUTHORED_LIFETIME_MS,
        },
    );
    Ok((transaction, initial_animation_offset_at_0xb2))
}

pub(crate) fn drive_outer_tail_transaction(
    manager: &mut EntityManager,
    actor_lease: MainBaseAbortActorLease,
    mut transaction: OrdinaryType9OwnerTransaction,
    origin_retail_tick: u32,
    mut expected_animation_offset_at_0xb2: RetailRuntimeValue<u16>,
    terrain_collision: Option<TerrainCollisionContext<'_>>,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> OrdinaryType9OuterTailDrive {
    loop {
        match transaction.poll() {
            OrdinaryType9OwnerPoll::Action(issued) => match issued.action {
                OrdinaryType9OwnerAction::CommitState {
                    phase,
                    before,
                    after,
                } => {
                    let Some(committed) = commit_outer_tail_state(
                        manager,
                        actor_lease,
                        phase,
                        before,
                        expected_animation_offset_at_0xb2,
                        after,
                    ) else {
                        return OrdinaryType9OuterTailDrive::StateMismatch;
                    };
                    if phase == OrdinaryType9OwnerCommitPhase::MasterMotion {
                        expected_animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
                    }
                    transaction
                        .resume(
                            issued.receipt,
                            OrdinaryType9OwnerResume::StateCommitted { committed },
                        )
                        .expect("the selected adapter owns the exact commit receipt");
                }
                OrdinaryType9OwnerAction::PlanSurface { .. } => {
                    transaction
                        .plan_surface(issued.receipt, || next_shared_random(&mut *world_fx))
                        .expect("the selected adapter owns the exact surface receipt");
                }
                OrdinaryType9OwnerAction::SpawnBubble(request) => {
                    let Some(terrain_collision) = terrain_collision else {
                        let Some(expected_state) = snapshot_outer_tail_state(manager, actor_lease)
                        else {
                            return OrdinaryType9OuterTailDrive::StateMismatch;
                        };
                        return OrdinaryType9OuterTailDrive::Blocked {
                            custody: OrdinaryType9OuterTailCustody::BubblePending {
                                transaction,
                                issued,
                                origin_retail_tick,
                                expected_state,
                                expected_animation_offset_at_0xb2,
                            },
                            reason: OrdinaryType9OuterTailBlock::CurrentTerrainUnavailable,
                        };
                    };
                    let _ = world_fx.materialize_actor_surface_bubble_request(
                        request,
                        ParticleEnvironment::Terrain(terrain_collision),
                        origin_retail_tick,
                    );
                    let Some(fresh_state) = snapshot_outer_tail_state(manager, actor_lease) else {
                        return OrdinaryType9OuterTailDrive::StateMismatch;
                    };
                    if snapshot_outer_tail_animation_offset(manager, actor_lease)
                        != Some(expected_animation_offset_at_0xb2)
                    {
                        return OrdinaryType9OuterTailDrive::StateMismatch;
                    }
                    transaction
                        .resume(
                            issued.receipt,
                            OrdinaryType9OwnerResume::EffectCompleted { fresh_state },
                        )
                        .expect("the selected adapter owns the exact bubble receipt");
                }
                OrdinaryType9OwnerAction::PlaySound(request) => {
                    world_fx
                        .queue_fixed_positional_sound_raw(request.sound_id, request.position_raw);
                    let Some(fresh_state) = snapshot_outer_tail_state(manager, actor_lease) else {
                        return OrdinaryType9OuterTailDrive::StateMismatch;
                    };
                    if snapshot_outer_tail_animation_offset(manager, actor_lease)
                        != Some(expected_animation_offset_at_0xb2)
                    {
                        return OrdinaryType9OuterTailDrive::StateMismatch;
                    }
                    transaction
                        .resume(
                            issued.receipt,
                            OrdinaryType9OwnerResume::EffectCompleted { fresh_state },
                        )
                        .expect("the selected adapter owns the exact sound receipt");
                }
                OrdinaryType9OwnerAction::RunLifecycleContinuation(request) => {
                    if request.entity_id != actor_lease.entity_id {
                        return OrdinaryType9OuterTailDrive::StateMismatch;
                    }
                    match manager.publish_ordinary_type9_standard_death(
                        request.entity_id,
                        OrdinaryType9StandardDeathEntry::SurfaceLifetimeExpiry,
                        MainBaseType9ResultScreenState::NotShown,
                        world_fx,
                        origin_retail_tick as i32,
                        None,
                    ) {
                        Ok(
                            OrdinaryType9StandardDeathOutcome::Published { .. }
                            | OrdinaryType9StandardDeathOutcome::InitializerFallback { .. }
                            | OrdinaryType9StandardDeathOutcome::RemoteOwnedNoOp { .. }
                            | OrdinaryType9StandardDeathOutcome::AlreadyDyingNoOp { .. },
                        ) => {
                            let Some(fresh_state) = snapshot_outer_tail_state(manager, actor_lease)
                            else {
                                transaction
                                    .resume(
                                        issued.receipt,
                                        OrdinaryType9OwnerResume::LifecycleCompleted {
                                            fresh_state: None,
                                        },
                                    )
                                    .expect("lifecycle already mutated the allocation");
                                continue;
                            };
                            if snapshot_outer_tail_animation_offset(manager, actor_lease)
                                != Some(expected_animation_offset_at_0xb2)
                            {
                                return OrdinaryType9OuterTailDrive::StateMismatch;
                            }
                            transaction
                                .resume(
                                    issued.receipt,
                                    OrdinaryType9OwnerResume::LifecycleCompleted {
                                        fresh_state: Some(fresh_state),
                                    },
                                )
                                .expect("the selected adapter owns the exact lifecycle receipt");
                        }
                        Err(_) => {
                            let Some(expected_state) =
                                snapshot_outer_tail_state(manager, actor_lease)
                            else {
                                return OrdinaryType9OuterTailDrive::StateMismatch;
                            };
                            return OrdinaryType9OuterTailDrive::Blocked {
                                custody: OrdinaryType9OuterTailCustody::LifecyclePending {
                                    transaction,
                                    issued,
                                    origin_retail_tick,
                                    expected_state,
                                    expected_animation_offset_at_0xb2,
                                },
                                reason: OrdinaryType9OuterTailBlock::OuterLifecyclePending,
                            };
                        }
                    }
                }
            },
            OrdinaryType9OwnerPoll::Complete(completion) => {
                debug_assert_eq!(completion.lease.controlled_entity_id, actor_lease.entity_id);
                return OrdinaryType9OuterTailDrive::Complete(completion.state);
            }
            OrdinaryType9OwnerPoll::Blocked(block) => {
                return OrdinaryType9OuterTailDrive::Blocked {
                    custody: OrdinaryType9OuterTailCustody::Transaction {
                        transaction,
                        block,
                        origin_retail_tick,
                        expected_state: block.state_before_action,
                        expected_animation_offset_at_0xb2,
                    },
                    reason: OrdinaryType9OuterTailBlock::OuterOwner(block),
                };
            }
            OrdinaryType9OwnerPoll::Awaiting => {
                unreachable!("the synchronous adapter retains every issued receipt")
            }
        }
    }
}

pub(crate) fn resume_outer_tail_bubble(
    manager: &mut EntityManager,
    actor_lease: MainBaseAbortActorLease,
    custody: OrdinaryType9OuterTailCustody,
    resources: &ResourceCache,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Result<OrdinaryType9OuterTailDrive, OrdinaryType9OuterTailCustody> {
    let OrdinaryType9OuterTailCustody::BubblePending {
        mut transaction,
        issued,
        origin_retail_tick,
        expected_state,
        expected_animation_offset_at_0xb2,
    } = custody
    else {
        unreachable!("bubble resume retains bubble custody")
    };
    let Some(terrain_collision) = TerrainCollisionContext::from_current_level_cache(resources)
    else {
        return Err(OrdinaryType9OuterTailCustody::BubblePending {
            transaction,
            issued,
            origin_retail_tick,
            expected_state,
            expected_animation_offset_at_0xb2,
        });
    };
    let IssuedOrdinaryType9OwnerAction { receipt, action } = issued;
    let OrdinaryType9OwnerAction::SpawnBubble(request) = action else {
        unreachable!("bubble custody retains the exact issued bubble action")
    };
    let _ = world_fx.materialize_actor_surface_bubble_request(
        request,
        ParticleEnvironment::Terrain(terrain_collision),
        origin_retail_tick,
    );
    transaction
        .resume(
            receipt,
            OrdinaryType9OwnerResume::EffectCompleted {
                fresh_state: expected_state,
            },
        )
        .expect("the retained adapter owns the exact bubble receipt");
    Ok(drive_outer_tail_transaction(
        manager,
        actor_lease,
        transaction,
        origin_retail_tick,
        expected_animation_offset_at_0xb2,
        Some(terrain_collision),
        world_fx,
        next_shared_random,
    ))
}
