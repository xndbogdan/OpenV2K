//! Allocation-authenticated owner for the post-Main-Base-abort Type-66 task.
//!
//! The bridge starts only after the death hook has published its replacement
//! Working Factory Primary and a positive progressive clock. Every fallible
//! resource/model check completes before the wrapper elapsed prefix, private
//! under-attack latch, shared RNG, or live factory state can change.

use v2k_formats::models::{StagedEffectError, StagedEffectPoint, StagedEffectRequest};

use crate::actor_task_dispatcher::{ActorTaskRuntime, ActorTaskRuntimeFamily};
use crate::actor_task_owner::{ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags};
use crate::base_factory_progression::PROGRESSION_REVIVE_HEALTH_RAW;
use crate::entity::{authenticate_fresh_level_one_type66_context, EntityManager};
use crate::entity_collision_state::{
    RetailRuntimeValue, RetailStateWord, DEFERRED_DESTROY_PENDING_STATE_BIT, DYING_STATE_BIT,
    REMOTE_OWNED_STATE_BIT,
};
use crate::factory_production::FactoryProductionRuntime;
use crate::factory_production_live::{
    FactoryProductionEntityVersion, FactoryProductionTransactionId,
};
use crate::factory_production_owner::{
    FactoryProductionOwnerAction, FactoryProductionOwnerActionPhase,
    FactoryProductionOwnerAdmissionRejection, FactoryProductionOwnerFrameRequest,
    FactoryProductionOwnerMachine, FactoryProductionOwnerPoll, FactoryProductionOwnerResume,
    FactoryProductionOwnerTransactionId,
};
use crate::gameplay_notifications::GameplayNotifications;
use crate::main_base_abort::MainBaseAbortActorLease;
use crate::main_base_type66_abort::{
    exact_level_one_type66_metadata, MainBaseType66WorkingFactoryTaskLease,
    WorkingFactoryNotificationOutcome, LEVEL_ONE_TYPE66_ACTIVE_MODEL_ID,
    LEVEL_ONE_TYPE66_CAPABILITY_FLAGS, LEVEL_ONE_TYPE66_CAPTURED_MODEL_SLOTS,
    LEVEL_ONE_TYPE66_CAPTURED_PREABORT_STATE_RAW, LEVEL_ONE_TYPE66_DEATH_SOUND_ID,
    LEVEL_ONE_TYPE66_ENTITY_TYPE, LEVEL_ONE_TYPE66_INITIAL_BEHAVIOR_CLASS,
    LEVEL_ONE_TYPE66_INITIAL_HEALTH_RAW, LEVEL_ONE_TYPE66_MASS_RAW, LEVEL_ONE_TYPE66_SPAWN_INDEX,
    LEVEL_ONE_TYPE66_STATUS_DESCRIPTOR,
};
use crate::player_active_contact::candidate_pair_orientation;
use crate::resource_cache::ResourceCache;
use crate::type17_impact_reselection::evaluate_under_attack;
use crate::world_fx::{TerrainExplosionLight, WorldFx};

const LEVEL_ONE_FACTORY_OUTPUT_PAYLOAD: u32 = 0x0001_F412;
const LEVEL_ONE_FACTORY_CAPACITY: i32 = 2;
const LEVEL_ONE_FACTORY_PRODUCTION_THRESHOLD_US: i32 = 6_000_000;
const LEVEL_ONE_FACTORY_DELIVERY_DURATION_US: i32 = 5_000_000;
const LEVEL_ONE_FACTORY_COOLDOWN_US: i32 = -1_000;
const LEVEL_ONE_FACTORY_UNDERSTAFFED_LIMIT_US: i32 = 0;
const LEVEL_ONE_WORLD_STYLE: u32 = 1;

#[derive(Clone, Copy)]
pub(crate) struct MainBaseType66ProductionFrame<'a> {
    pub resources: &'a ResourceCache,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
    pub main_base_abort_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType66ProductionBlock {
    TypeMetadataUnavailable,
    TypeMetadataMismatch,
    CurrentBehaviorContextUnresolved,
    FactoryRuntimeUnresolved,
    LastHitPresentationTickUnresolved,
    OrientationUnresolved,
    ActiveModelUnavailable {
        model_id: usize,
    },
    DestroyedModelUnavailable {
        model_id: usize,
    },
    StagedEffectWalk(StagedEffectError),
    TaskAlreadyInCallback {
        visit: ActorTaskVisit,
    },
    TaskSequenceExhausted,
    FactoryTransactionSpaceExhausted,
    FactoryStateVersionExhausted,
    PickupWrongEntityType {
        entity_id: u32,
        actual: u32,
    },
    PickupDeferredStateUnresolved {
        entity_id: u32,
    },
    PickupDeferredStateQueueMismatch {
        entity_id: u32,
        pending_state: bool,
        queued: bool,
    },
    OwnerAdmission(FactoryProductionOwnerAdmissionRejection),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType66ProductionDrop {
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
    UnauthenticatedSpawn {
        actual: Option<usize>,
    },
    UnexpectedMassOrCapabilities,
    LiveModelMismatch,
    CollisionStateMismatch,
    HealthOrSoundMismatch,
    InitialBehaviorMismatch,
    CurrentBehaviorContextMismatch,
    FactoryRuntimeMismatch,
    FactoryTemplateMismatch,
    FactoryLiveOwnerMismatch,
    ProgressiveDeathConfigMismatch {
        actual: u8,
    },
    ProgressiveClockNotActive {
        actual: i32,
    },
    PrimaryTaskMissingOrReplaced,
    WrongTaskFamily {
        actual: ActorTaskRuntimeFamily,
    },
    TaskAllocationMismatch,
    TaskSequenceMismatch {
        expected: u64,
        actual: u64,
    },
    AdditionalPublishedTask {
        slot: ActorTaskSlot,
    },
    TaskWrapperNotRunnable {
        visit: ActorTaskVisit,
    },
    ExpectedVisitNotTicked,
    FactoryLeaseChangedDuringCallback,
    UnexpectedOwnerAction(FactoryProductionOwnerActionPhase),
    WrapperReplacedDuringCallback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MainBaseType66ProductionOutcome {
    Continuing {
        entity_id: u32,
        visit: ActorTaskVisit,
        elapsed_ms: u32,
        notification: WorkingFactoryNotificationOutcome,
        under_attack_notification_requested: bool,
        model_effect_stages: usize,
        accepted_effect_points: usize,
    },
    Terminal {
        entity_id: u32,
        visit: ActorTaskVisit,
        elapsed_ms: u32,
        notification: WorkingFactoryNotificationOutcome,
        under_attack_notification_requested: bool,
        progressive_death_presentation_requested: bool,
        model_effect_stages: usize,
        accepted_effect_points: usize,
        death_sound_id: u16,
    },
    Blocked {
        entity_id: u32,
        reason: MainBaseType66ProductionBlock,
    },
    Dropped {
        entity_id: u32,
        reason: MainBaseType66ProductionDrop,
    },
}

/// Linear custody for the next exact callback of one published Primary.
/// Deliberately neither `Clone` nor `Copy`.
#[derive(Debug, PartialEq, Eq)]
pub struct MainBaseType66ProductionOwner {
    task_lease: MainBaseType66WorkingFactoryTaskLease,
    next_tick_sequence: u64,
}

impl MainBaseType66ProductionOwner {
    pub(crate) const fn adopt(task_lease: MainBaseType66WorkingFactoryTaskLease) -> Self {
        Self {
            task_lease,
            next_tick_sequence: 1,
        }
    }

    pub(crate) const fn entity_id(&self) -> u32 {
        self.task_lease.actor().entity_id
    }

    pub(crate) const fn actor_lease(&self) -> MainBaseAbortActorLease {
        self.task_lease.actor()
    }

    /// Duplicate linear custody only for the isolated Main Base abort
    /// transaction.  The enclosing scheduler fork keeps the original owner
    /// inaccessible until the coherent world transaction commits or rolls
    /// back.
    pub(crate) const fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            task_lease: self.task_lease,
            next_tick_sequence: self.next_tick_sequence,
        }
    }

    #[cfg(test)]
    pub(crate) const fn task_lease(&self) -> MainBaseType66WorkingFactoryTaskLease {
        self.task_lease
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MainBaseType66ProductionOwnerTick {
    pub outcome: MainBaseType66ProductionOutcome,
    pub retained_owner: Option<MainBaseType66ProductionOwner>,
    pub explosion_lights: Vec<TerrainExplosionLight>,
}

pub(crate) fn tick_main_base_type66_production_owner(
    manager: &mut EntityManager,
    owner: MainBaseType66ProductionOwner,
    frame: MainBaseType66ProductionFrame<'_>,
    world_fx: &mut WorldFx,
    notifications: &mut GameplayNotifications,
) -> MainBaseType66ProductionOwnerTick {
    tick_main_base_type66_production_owner_with_observer(
        manager,
        owner,
        frame,
        world_fx,
        notifications,
        |_, _| {},
    )
}

fn tick_main_base_type66_production_owner_with_observer(
    manager: &mut EntityManager,
    owner: MainBaseType66ProductionOwner,
    frame: MainBaseType66ProductionFrame<'_>,
    world_fx: &mut WorldFx,
    notifications: &mut GameplayNotifications,
    mut before_action: impl FnMut(&mut EntityManager, FactoryProductionOwnerActionPhase),
) -> MainBaseType66ProductionOwnerTick {
    tick_preflight_and_run(
        manager,
        owner,
        frame,
        world_fx,
        notifications,
        &mut before_action,
    )
}

struct PreparedType66Frame {
    visit: ActorTaskVisit,
    factory: FactoryProductionEntityVersion,
    machine: FactoryProductionOwnerMachine,
    staged_effect_points: Vec<StagedEffectPoint>,
    model_effect_stages: usize,
    reaches_terminal_death: bool,
    under_attack: bool,
}

enum Type66AdmissionFailure {
    Blocked(MainBaseType66ProductionBlock),
    Dropped(MainBaseType66ProductionDrop),
}

fn tick_preflight_and_run(
    manager: &mut EntityManager,
    owner: MainBaseType66ProductionOwner,
    frame: MainBaseType66ProductionFrame<'_>,
    world_fx: &mut WorldFx,
    notifications: &mut GameplayNotifications,
    before_action: &mut impl FnMut(&mut EntityManager, FactoryProductionOwnerActionPhase),
) -> MainBaseType66ProductionOwnerTick {
    let entity_id = owner.entity_id();
    let prepared = match prepare_type66_frame(manager, &owner, frame) {
        Ok(prepared) => prepared,
        Err(Type66AdmissionFailure::Blocked(reason)) => {
            return MainBaseType66ProductionOwnerTick {
                outcome: MainBaseType66ProductionOutcome::Blocked { entity_id, reason },
                retained_owner: Some(owner),
                explosion_lights: Vec::new(),
            }
        }
        Err(Type66AdmissionFailure::Dropped(reason)) => {
            return MainBaseType66ProductionOwnerTick {
                outcome: MainBaseType66ProductionOutcome::Dropped { entity_id, reason },
                retained_owner: None,
                explosion_lights: Vec::new(),
            }
        }
    };
    run_prepared_type66_frame(
        manager,
        owner,
        frame,
        world_fx,
        notifications,
        prepared,
        before_action,
    )
}

fn prepare_type66_frame(
    manager: &EntityManager,
    owner: &MainBaseType66ProductionOwner,
    frame: MainBaseType66ProductionFrame<'_>,
) -> Result<PreparedType66Frame, Type66AdmissionFailure> {
    let task_lease = owner.task_lease;
    let entity_id = task_lease.actor().entity_id;
    let Some(observation) = manager.main_base_abort_actor_observation(entity_id) else {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::EntityUnavailable,
        ));
    };
    if observation.lease != task_lease.actor() {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::ActorLeaseMismatch {
                expected: observation.lease,
                actual: task_lease.actor(),
            },
        ));
    }
    if !manager.is_fresh_new_game_first_world() {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::NotFreshNewGameFirstWorld,
        ));
    }
    let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::EntityUnavailable,
        ));
    };
    if !entity.active {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::EntityInactive,
        ));
    }
    if entity.entity_type != LEVEL_ONE_TYPE66_ENTITY_TYPE {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::WrongEntityType {
                actual: entity.entity_type,
            },
        ));
    }
    if entity.authored_spawn_index != Some(LEVEL_ONE_TYPE66_SPAWN_INDEX) {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::UnauthenticatedSpawn {
                actual: entity.authored_spawn_index,
            },
        ));
    }
    if entity.mass_raw != LEVEL_ONE_TYPE66_MASS_RAW
        || entity.capability_flags != LEVEL_ONE_TYPE66_CAPABILITY_FLAGS
    {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::UnexpectedMassOrCapabilities,
        ));
    }

    let Some(metadata) = manager.type_runtime_metadata(entity.entity_type) else {
        return Err(Type66AdmissionFailure::Blocked(
            MainBaseType66ProductionBlock::TypeMetadataUnavailable,
        ));
    };
    if !exact_level_one_type66_metadata(metadata) {
        return Err(Type66AdmissionFailure::Blocked(
            MainBaseType66ProductionBlock::TypeMetadataMismatch,
        ));
    }
    if entity.model_slots != LEVEL_ONE_TYPE66_CAPTURED_MODEL_SLOTS
        || entity.collision.active_model_slot() != RetailRuntimeValue::Known(0)
        || entity.model_index != Some(LEVEL_ONE_TYPE66_ACTIVE_MODEL_ID)
    {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::LiveModelMismatch,
        ));
    }
    if entity.collision.state_flags_at_0x08
        != RetailStateWord::exact(LEVEL_ONE_TYPE66_CAPTURED_PREABORT_STATE_RAW)
        || entity
            .collision
            .state_flags_at_0x08
            .masked(REMOTE_OWNED_STATE_BIT | DYING_STATE_BIT)
            != RetailRuntimeValue::Known(0)
    {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::CollisionStateMismatch,
        ));
    }
    if entity.collision.health_raw != RetailRuntimeValue::Known(PROGRESSION_REVIVE_HEALTH_RAW)
        || entity.collision.death_sound_id
            != RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE66_DEATH_SOUND_ID))
    {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::HealthOrSoundMismatch,
        ));
    }
    let RetailRuntimeValue::Known(Some(initial_behavior)) = entity.initial_behavior else {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::InitialBehaviorMismatch,
        ));
    };
    if initial_behavior.choice_index != 0
        || initial_behavior.program.class_id != LEVEL_ONE_TYPE66_INITIAL_BEHAVIOR_CLASS
    {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::InitialBehaviorMismatch,
        ));
    }
    match entity.current_behavior_context {
        RetailRuntimeValue::Known(Some(context))
            if authenticate_fresh_level_one_type66_context(context).is_some() => {}
        RetailRuntimeValue::Unresolved => {
            return Err(Type66AdmissionFailure::Blocked(
                MainBaseType66ProductionBlock::CurrentBehaviorContextUnresolved,
            ))
        }
        RetailRuntimeValue::Known(Some(_)) | RetailRuntimeValue::Known(None) => {
            return Err(Type66AdmissionFailure::Dropped(
                MainBaseType66ProductionDrop::CurrentBehaviorContextMismatch,
            ))
        }
    }

    let base = match entity.base_factory_runtime {
        RetailRuntimeValue::Known(Some(base)) => base,
        RetailRuntimeValue::Unresolved => {
            return Err(Type66AdmissionFailure::Blocked(
                MainBaseType66ProductionBlock::FactoryRuntimeUnresolved,
            ))
        }
        RetailRuntimeValue::Known(None) => {
            return Err(Type66AdmissionFailure::Dropped(
                MainBaseType66ProductionDrop::FactoryRuntimeMismatch,
            ))
        }
    };
    if base.status_descriptor != LEVEL_ONE_TYPE66_STATUS_DESCRIPTOR
        || base.control_value_raw != 0xF412
        || base.required_scientists != LEVEL_ONE_FACTORY_CAPACITY as u16
    {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::FactoryRuntimeMismatch,
        ));
    }
    let Some(production) = base.production else {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::FactoryRuntimeMismatch,
        ));
    };
    if !exact_level_one_factory_template(production) {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::FactoryTemplateMismatch,
        ));
    }
    let Some(live_owner) = base.live_owner else {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::FactoryLiveOwnerMismatch,
        ));
    };
    if live_owner.allocation_identity == 0
        || live_owner.allocation_identity != task_lease.owner_allocation_identity()
        || live_owner.maximum_health_raw != LEVEL_ONE_TYPE66_INITIAL_HEALTH_RAW
        || live_owner.animation_state_raw & !7 != 0
    {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::FactoryLiveOwnerMismatch,
        ));
    }
    if base.progressive_death.config_flags_at_0x18 != 0 {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::ProgressiveDeathConfigMismatch {
                actual: base.progressive_death.config_flags_at_0x18,
            },
        ));
    }
    if base.progressive_death.elapsed_micros_raw <= 0 {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::ProgressiveClockNotActive {
                actual: base.progressive_death.elapsed_micros_raw,
            },
        ));
    }

    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id: task_lease.task_id(),
    };
    if entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) != Some(visit.task_id) {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::PrimaryTaskMissingOrReplaced,
        ));
    }
    let Some(task_runtime) = entity.actor_tasks.task_state(visit.task_id) else {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::PrimaryTaskMissingOrReplaced,
        ));
    };
    let ActorTaskRuntime::WorkingFactory(task) = task_runtime else {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::WrongTaskFamily {
                actual: task_runtime.family(),
            },
        ));
    };
    if task.owner_allocation_identity() != task_lease.owner_allocation_identity() {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::TaskAllocationMismatch,
        ));
    }
    if task.next_tick_sequence() != owner.next_tick_sequence {
        return Err(Type66AdmissionFailure::Dropped(
            MainBaseType66ProductionDrop::TaskSequenceMismatch {
                expected: task.next_tick_sequence(),
                actual: owner.next_tick_sequence,
            },
        ));
    }
    for slot in [ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary] {
        if entity.actor_tasks.task_in_slot(slot).is_some() {
            return Err(Type66AdmissionFailure::Dropped(
                MainBaseType66ProductionDrop::AdditionalPublishedTask { slot },
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
            return Err(Type66AdmissionFailure::Blocked(
                MainBaseType66ProductionBlock::TaskAlreadyInCallback { visit },
            ))
        }
        Some(_) | None => {
            return Err(Type66AdmissionFailure::Dropped(
                MainBaseType66ProductionDrop::TaskWrapperNotRunnable { visit },
            ))
        }
    }
    if live_owner.state_version == u64::MAX {
        return Err(Type66AdmissionFailure::Blocked(
            MainBaseType66ProductionBlock::FactoryStateVersionExhausted,
        ));
    }
    let Some(child_transaction_raw) = live_owner.next_transaction_id_raw.checked_add(1) else {
        return Err(Type66AdmissionFailure::Blocked(
            MainBaseType66ProductionBlock::FactoryTransactionSpaceExhausted,
        ));
    };
    if child_transaction_raw == u64::MAX {
        return Err(Type66AdmissionFailure::Blocked(
            MainBaseType66ProductionBlock::FactoryTransactionSpaceExhausted,
        ));
    }
    let Some(owner_transaction_id) =
        FactoryProductionOwnerTransactionId::new(live_owner.next_transaction_id_raw)
    else {
        return Err(Type66AdmissionFailure::Blocked(
            MainBaseType66ProductionBlock::FactoryTransactionSpaceExhausted,
        ));
    };
    let child_transaction_id = FactoryProductionTransactionId::new(child_transaction_raw)
        .expect("checked nonzero successor of a nonzero transaction id");
    let factory = FactoryProductionEntityVersion {
        entity_id,
        allocation_identity: live_owner.allocation_identity,
        state_version: live_owner.state_version,
    };

    // Snapshot every presentation input before the callback can advance the
    // progressive clock or switch to destroyed model 225.
    let anim_vars = entity.presentation_anim_vars(frame.retail_tick);
    let preview = base.progressive_death.advance(frame.elapsed_micros);
    let reaches_terminal_death = preview.reached_terminal_death;
    if !reaches_terminal_death && owner.next_tick_sequence == u64::MAX {
        return Err(Type66AdmissionFailure::Blocked(
            MainBaseType66ProductionBlock::TaskSequenceExhausted,
        ));
    }
    let staged_effect_points = if preview.effects.is_empty() {
        Vec::new()
    } else {
        let Some(model) = frame
            .resources
            .global_model(LEVEL_ONE_TYPE66_ACTIVE_MODEL_ID)
        else {
            return Err(Type66AdmissionFailure::Blocked(
                MainBaseType66ProductionBlock::ActiveModelUnavailable {
                    model_id: LEVEL_ONE_TYPE66_ACTIVE_MODEL_ID,
                },
            ));
        };
        let RetailRuntimeValue::Known(orientation) = candidate_pair_orientation(entity) else {
            return Err(Type66AdmissionFailure::Blocked(
                MainBaseType66ProductionBlock::OrientationUnresolved,
            ));
        };
        model
            .staged_effect_points_raw(
                StagedEffectRequest {
                    model_to_output_basis: orientation.map(|row| row.map(f64::from)),
                    model_origin_raw: entity.position_raw().map(f64::from),
                },
                &anim_vars,
                frame.resources,
            )
            .map_err(|error| {
                Type66AdmissionFailure::Blocked(MainBaseType66ProductionBlock::StagedEffectWalk(
                    error,
                ))
            })?
    };
    if reaches_terminal_death
        && frame
            .resources
            .global_model(LEVEL_ONE_TYPE66_CAPTURED_MODEL_SLOTS[1].unwrap())
            .is_none()
    {
        return Err(Type66AdmissionFailure::Blocked(
            MainBaseType66ProductionBlock::DestroyedModelUnavailable {
                model_id: LEVEL_ONE_TYPE66_CAPTURED_MODEL_SLOTS[1].unwrap(),
            },
        ));
    }
    if reaches_terminal_death {
        validate_terminal_pickup(manager, production.spawned_pickup_handle)?;
    }

    let RetailRuntimeValue::Known(last_hit_tick) =
        entity.collision.last_hit_presentation_tick_at_0x34
    else {
        return Err(Type66AdmissionFailure::Blocked(
            MainBaseType66ProductionBlock::LastHitPresentationTickUnresolved,
        ));
    };
    let under_attack = evaluate_under_attack(frame.retail_tick, last_hit_tick);

    let request = FactoryProductionOwnerFrameRequest {
        factory,
        position_raw: entity.position_raw(),
        pickup_spawn_offset_raw: [0; 3],
        current_health_raw: PROGRESSION_REVIVE_HEALTH_RAW,
        maximum_health_raw: live_owner.maximum_health_raw,
        progressive_death: base.progressive_death,
        production,
        animation_state_raw: live_owner.animation_state_raw,
        elapsed_micros: frame.elapsed_micros,
        world_style_raw: LEVEL_ONE_WORLD_STYLE,
        phase1_presentation_enabled: false,
        suppress_status_publication: false,
    };
    let machine = FactoryProductionOwnerMachine::preflight(
        owner_transaction_id,
        child_transaction_id,
        request,
    )
    .map_err(|reason| {
        Type66AdmissionFailure::Blocked(MainBaseType66ProductionBlock::OwnerAdmission(reason))
    })?;
    Ok(PreparedType66Frame {
        visit,
        factory,
        machine,
        staged_effect_points,
        model_effect_stages: preview.effects.len(),
        reaches_terminal_death,
        under_attack,
    })
}

fn exact_level_one_factory_template(production: FactoryProductionRuntime) -> bool {
    production.output_payload_packed == LEVEL_ONE_FACTORY_OUTPUT_PAYLOAD
        && production.scientist_capacity_raw == LEVEL_ONE_FACTORY_CAPACITY
        && production.production_threshold_micros_raw == LEVEL_ONE_FACTORY_PRODUCTION_THRESHOLD_US
        && production.delivery_duration_micros_raw == LEVEL_ONE_FACTORY_DELIVERY_DURATION_US
        && production.cooldown_duration_micros_raw == LEVEL_ONE_FACTORY_COOLDOWN_US
        && production.understaffed_limit_micros_raw == LEVEL_ONE_FACTORY_UNDERSTAFFED_LIMIT_US
}

fn validate_terminal_pickup(
    manager: &EntityManager,
    pickup_id: u32,
) -> Result<(), Type66AdmissionFailure> {
    if pickup_id == 0 {
        return Ok(());
    }
    let Some(pickup) = manager.iter_all().find(|entity| entity.id == pickup_id) else {
        // FUN_00410B70 acknowledges a stale/missing handle as a no-op.
        return Ok(());
    };
    if pickup.entity_type != 61 {
        return Err(Type66AdmissionFailure::Blocked(
            MainBaseType66ProductionBlock::PickupWrongEntityType {
                entity_id: pickup_id,
                actual: pickup.entity_type,
            },
        ));
    }
    let pending_state = match pickup
        .collision
        .state_flags_at_0x08
        .masked(DEFERRED_DESTROY_PENDING_STATE_BIT)
    {
        RetailRuntimeValue::Known(value) => value != 0,
        RetailRuntimeValue::Unresolved => {
            return Err(Type66AdmissionFailure::Blocked(
                MainBaseType66ProductionBlock::PickupDeferredStateUnresolved {
                    entity_id: pickup_id,
                },
            ))
        }
    };
    let queued = manager.is_power_up_destroy_pending(pickup_id);
    if pending_state != queued {
        return Err(Type66AdmissionFailure::Blocked(
            MainBaseType66ProductionBlock::PickupDeferredStateQueueMismatch {
                entity_id: pickup_id,
                pending_state,
                queued,
            },
        ));
    }
    Ok(())
}

fn run_prepared_type66_frame(
    manager: &mut EntityManager,
    owner: MainBaseType66ProductionOwner,
    frame: MainBaseType66ProductionFrame<'_>,
    world_fx: &mut WorldFx,
    notifications: &mut GameplayNotifications,
    prepared: PreparedType66Frame,
    before_action: &mut impl FnMut(&mut EntityManager, FactoryProductionOwnerActionPhase),
) -> MainBaseType66ProductionOwnerTick {
    let entity_id = owner.entity_id();
    let PreparedType66Frame {
        visit,
        factory,
        mut machine,
        staged_effect_points,
        model_effect_stages,
        reaches_terminal_death,
        under_attack,
    } = prepared;

    let (elapsed_ms, notification) = {
        let Some(entity) = manager.main_base_type66_production_entity_mut(entity_id) else {
            return dropped_tick(
                entity_id,
                MainBaseType66ProductionDrop::EntityUnavailable,
                Vec::new(),
            );
        };
        let Some(elapsed_ms) = entity
            .actor_tasks
            .begin_exact_visit_with(visit, |runtime| match runtime {
                ActorTaskRuntime::WorkingFactory(task) => {
                    Some(task.accumulate_elapsed_prefix(frame.elapsed_micros))
                }
                _ => None,
            })
            .flatten()
        else {
            return dropped_tick(
                entity_id,
                MainBaseType66ProductionDrop::ExpectedVisitNotTicked,
                Vec::new(),
            );
        };
        let Some(ActorTaskRuntime::WorkingFactory(task)) =
            entity.actor_tasks.exact_callback_state_mut(visit)
        else {
            let _ = entity.actor_tasks.finish_exact_visit(visit);
            return dropped_tick(
                entity_id,
                MainBaseType66ProductionDrop::ExpectedVisitNotTicked,
                Vec::new(),
            );
        };
        let notification =
            task.sample_under_attack_with(under_attack, frame.main_base_abort_active, |_| {
                notifications.queue_factory_under_attack(frame.retail_tick as i32)
            });
        (elapsed_ms, notification)
    };
    let under_attack_notification_requested =
        notification == WorkingFactoryNotificationOutcome::NotificationRequested;
    let mut progressive_death_presentation_requested = false;
    let mut accepted_effect_points = 0usize;
    let mut explosion_lights = Vec::new();

    loop {
        match machine.poll() {
            FactoryProductionOwnerPoll::Action(issued) => {
                let phase = issued.action.phase();
                let prefix_committed = manager
                    .main_base_type66_production_entity_mut(entity_id)
                    .is_some_and(|entity| {
                        entity
                            .persist_factory_production_owner_prefix(factory, issued.before_action)
                    });
                if !prefix_committed {
                    unwind_exact_visit(manager, entity_id, visit);
                    return dropped_tick(
                        entity_id,
                        MainBaseType66ProductionDrop::FactoryLeaseChangedDuringCallback,
                        explosion_lights,
                    );
                }

                // Test-only callers use this private seam to observe ordering
                // or retire the executing wrapper. Production supplies a no-op.
                before_action(manager, phase);
                let action_applied = match issued.action {
                    FactoryProductionOwnerAction::DispatchProgressiveModelEffect {
                        effect, ..
                    } => {
                        for point in &staged_effect_points {
                            let accepted = effect.threshold >= 0x1_0000
                                || i32::from(world_fx.next_shared_retail_random_u16() & 0xff)
                                    < effect.threshold;
                            if !accepted {
                                continue;
                            }
                            accepted_effect_points += 1;
                            let position_raw = point
                                .center_raw
                                .map(|component| (component.round() as i32) as i16);
                            explosion_lights.push(world_fx.emit_common_explosion_bundle_raw(
                                position_raw,
                                point.scatter_radius_raw,
                            ));
                        }
                        true
                    }
                    FactoryProductionOwnerAction::QueueDeferredDestroy { entity_id, .. } => {
                        // Retail acknowledges zero, a missing handle, and an
                        // already queued Power-Up as ordinary no-ops.
                        if entity_id != 0 {
                            manager.queue_power_up_destroys(&[entity_id]);
                        }
                        true
                    }
                    FactoryProductionOwnerAction::InvokeProgressiveFactoryDeath { .. } => {
                        let Some(position_raw) = manager
                            .main_base_type66_production_entity_mut(entity_id)
                            .and_then(|entity| {
                                entity.begin_main_base_type66_progressive_terminal_death(factory)
                            })
                        else {
                            unwind_exact_visit(manager, entity_id, visit);
                            return dropped_tick(
                                entity_id,
                                MainBaseType66ProductionDrop::FactoryLeaseChangedDuringCallback,
                                explosion_lights,
                            );
                        };
                        world_fx.queue_fixed_positional_sound_raw(
                            LEVEL_ONE_TYPE66_DEATH_SOUND_ID,
                            position_raw,
                        );
                        manager
                            .main_base_type66_production_entity_mut(entity_id)
                            .is_some_and(|entity| {
                                entity.finish_main_base_type66_progressive_terminal_death(factory)
                            })
                    }
                    FactoryProductionOwnerAction::DispatchProgressiveDeathPresentation {
                        ..
                    } => {
                        progressive_death_presentation_requested = true;
                        true
                    }
                    FactoryProductionOwnerAction::ApplyAnimationTransition {
                        transition, ..
                    } => manager
                        .main_base_type66_production_entity_mut(entity_id)
                        .is_some_and(|entity| {
                            entity.apply_factory_production_owner_animation(factory, transition)
                        }),
                    FactoryProductionOwnerAction::PublishStatus { status, .. } => manager
                        .main_base_type66_production_entity_mut(entity_id)
                        .is_some_and(|entity| {
                            entity.apply_factory_production_owner_status(factory, status)
                        }),
                    action => {
                        let unexpected_phase = action.phase();
                        unwind_exact_visit(manager, entity_id, visit);
                        return dropped_tick(
                            entity_id,
                            MainBaseType66ProductionDrop::UnexpectedOwnerAction(unexpected_phase),
                            explosion_lights,
                        );
                    }
                };
                if !action_applied {
                    unwind_exact_visit(manager, entity_id, visit);
                    return dropped_tick(
                        entity_id,
                        MainBaseType66ProductionDrop::FactoryLeaseChangedDuringCallback,
                        explosion_lights,
                    );
                }
                machine
                    .resume(
                        issued.receipt,
                        FactoryProductionOwnerResume::Acknowledged { phase },
                    )
                    .expect("the synchronous Type-66 bridge resumes its exact issued receipt");
            }
            FactoryProductionOwnerPoll::Complete(completion) => {
                let frame_committed = manager
                    .main_base_type66_production_entity_mut(entity_id)
                    .is_some_and(|entity| {
                        entity.finish_factory_production_owner_frame(factory, completion)
                    });
                if !frame_committed {
                    unwind_exact_visit(manager, entity_id, visit);
                    return dropped_tick(
                        entity_id,
                        MainBaseType66ProductionDrop::FactoryLeaseChangedDuringCallback,
                        explosion_lights,
                    );
                }
                let wrapper_survived = finish_type66_visit(
                    manager,
                    entity_id,
                    visit,
                    owner.next_tick_sequence,
                    !reaches_terminal_death,
                );
                if !wrapper_survived {
                    return dropped_tick(
                        entity_id,
                        MainBaseType66ProductionDrop::WrapperReplacedDuringCallback,
                        explosion_lights,
                    );
                }

                let outcome = if reaches_terminal_death {
                    debug_assert_eq!(completion.progressive_death.elapsed_micros_raw, -1);
                    debug_assert_eq!(completion.current_health_raw, 0);
                    MainBaseType66ProductionOutcome::Terminal {
                        entity_id,
                        visit,
                        elapsed_ms,
                        notification,
                        under_attack_notification_requested,
                        progressive_death_presentation_requested,
                        model_effect_stages,
                        accepted_effect_points,
                        death_sound_id: LEVEL_ONE_TYPE66_DEATH_SOUND_ID,
                    }
                } else {
                    MainBaseType66ProductionOutcome::Continuing {
                        entity_id,
                        visit,
                        elapsed_ms,
                        notification,
                        under_attack_notification_requested,
                        model_effect_stages,
                        accepted_effect_points,
                    }
                };
                let retained_owner =
                    (!reaches_terminal_death).then_some(MainBaseType66ProductionOwner {
                        task_lease: owner.task_lease,
                        next_tick_sequence: owner.next_tick_sequence + 1,
                    });
                return MainBaseType66ProductionOwnerTick {
                    outcome,
                    retained_owner,
                    explosion_lights,
                };
            }
            FactoryProductionOwnerPoll::Awaiting(_) => {
                unreachable!("the synchronous Type-66 bridge resumes every issued action")
            }
            FactoryProductionOwnerPoll::Blocked(_) => {
                unreachable!("preflight closes every Type-66 external action")
            }
        }
    }
}

fn finish_type66_visit(
    manager: &mut EntityManager,
    entity_id: u32,
    visit: ActorTaskVisit,
    expected_sequence: u64,
    advance_sequence: bool,
) -> bool {
    let Some(entity) = manager.main_base_type66_production_entity_mut(entity_id) else {
        return false;
    };
    let sequence_valid = match entity.actor_tasks.exact_callback_state_mut(visit) {
        Some(ActorTaskRuntime::WorkingFactory(task))
            if task.next_tick_sequence() == expected_sequence =>
        {
            if advance_sequence {
                task.advance_tick_sequence();
            }
            true
        }
        _ => false,
    };
    let wrapper_survived = entity.actor_tasks.finish_exact_visit(visit);
    sequence_valid && wrapper_survived
}

fn unwind_exact_visit(manager: &mut EntityManager, entity_id: u32, visit: ActorTaskVisit) {
    if let Some(entity) = manager.main_base_type66_production_entity_mut(entity_id) {
        let _ = entity.actor_tasks.finish_exact_visit(visit);
    }
}

fn dropped_tick(
    entity_id: u32,
    reason: MainBaseType66ProductionDrop,
    explosion_lights: Vec<TerrainExplosionLight>,
) -> MainBaseType66ProductionOwnerTick {
    MainBaseType66ProductionOwnerTick {
        outcome: MainBaseType66ProductionOutcome::Dropped { entity_id, reason },
        retained_owner: None,
        explosion_lights,
    }
}

#[cfg(test)]
pub(crate) use tests::{effect_program, model_cache, post_abort_owner, set_terminal_predecessor};

#[cfg(test)]
mod tests {
    use super::*;

    use v2k_formats::models::{ModelCollection, ModelEntry, StreamStats};

    use crate::actor_task_owner::PreparedActorTask;
    use crate::entity::{terminal_abort_composition_manager, BaseFactoryRuntimeState};
    use crate::entity_collision_state::PairOrientationPolicy;
    use crate::factory_production_live::FactoryAnimationRange;
    use crate::gameplay_notifications::{TextTypewriterCadence, FACTORY_UNDER_ATTACK_TEXT_ID};
    use crate::level::LevelState;
    use crate::main_base_type66_abort::{
        MainBaseType66DeathAdvance, MainBaseType66DeathOutcome, WorkingFactoryTaskState,
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Type66Snapshot {
        collision_state: RetailStateWord,
        health: RetailRuntimeValue<i32>,
        model_index: Option<usize>,
        factory: BaseFactoryRuntimeState,
        task: WorkingFactoryTaskState,
        wrapper: ActorTaskWrapperFlags,
    }

    pub(crate) fn post_abort_owner() -> (EntityManager, MainBaseType66ProductionOwner) {
        let mut manager = terminal_abort_composition_manager();
        let entity_id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(LEVEL_ONE_TYPE66_SPAWN_INDEX))
            .expect("fixture contains spawn-23 Type-66")
            .id;
        let actor = manager
            .main_base_abort_actor_observation(entity_id)
            .expect("factory remains in the abort list");
        let mut abort_fx = WorldFx::new();
        let task_lease = match manager
            .apply_main_base_abort_type66_death(actor.lease, &mut abort_fx)
            .expect("exact fixture enters Type-66 progressive death")
        {
            MainBaseType66DeathAdvance::Advanced {
                outcome: MainBaseType66DeathOutcome::WorkingFactoryTaskPublished { task_lease, .. },
                ..
            }
            | MainBaseType66DeathAdvance::SuccessorUnavailableAfterCommit {
                outcome: MainBaseType66DeathOutcome::WorkingFactoryTaskPublished { task_lease, .. },
            } => task_lease,
            other => panic!("unexpected Type-66 abort result: {other:?}"),
        };
        manager
            .main_base_type66_production_entity_mut(entity_id)
            .unwrap()
            .collision
            .pair_callbacks
            .orientation_policy =
            RetailRuntimeValue::Known(PairOrientationPolicy::LiveHeadingWithAuthoredPitchRoll {
                pitch_raw: 0,
                roll_raw: 0,
            });
        (manager, MainBaseType66ProductionOwner::adopt(task_lease))
    }

    fn snapshot(manager: &EntityManager, owner: &MainBaseType66ProductionOwner) -> Type66Snapshot {
        snapshot_exact(
            manager,
            owner.entity_id(),
            ActorTaskVisit {
                slot: ActorTaskSlot::Primary,
                task_id: owner.task_lease.task_id(),
            },
        )
    }

    fn snapshot_exact(
        manager: &EntityManager,
        entity_id: u32,
        visit: ActorTaskVisit,
    ) -> Type66Snapshot {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        let RetailRuntimeValue::Known(Some(factory)) = entity.base_factory_runtime else {
            panic!("exact factory runtime")
        };
        let Some(ActorTaskRuntime::WorkingFactory(task)) =
            entity.actor_tasks.task_state(visit.task_id)
        else {
            panic!("exact Working Factory task")
        };
        Type66Snapshot {
            collision_state: entity.collision.state_flags_at_0x08,
            health: entity.collision.health_raw,
            model_index: entity.model_index,
            factory,
            task: *task,
            wrapper: entity.actor_tasks.wrapper_flags(visit.task_id).unwrap(),
        }
    }

    fn model_entry(index: usize, collision_program: Vec<u8>, records: Vec<[i16; 4]>) -> ModelEntry {
        ModelEntry {
            index,
            cmd_word_count: 0,
            extra_count: 0,
            flags: 0x40,
            slot_count: (records.len() * 2) as u16,
            face_val: 2,
            radius: 0,
            collision_radius_raw: 0,
            collision_program,
            records,
            normal_pool: Vec::new(),
            cmd_words: Vec::new(),
            has_view_commands: false,
            vertices: Vec::new(),
            vertex_type_flags: Vec::new(),
            vertex_projection: Vec::new(),
            vertex_clip: Vec::new(),
            vertex_surface_origin: Vec::new(),
            triangles: Vec::new(),
            face_vertices: Vec::new(),
            normals: Vec::new(),
            face_cull: Vec::new(),
            face_materials: Vec::new(),
            face_uvs: Vec::new(),
            face_corner_normals: Vec::new(),
            face_shading: Vec::new(),
            shadow_triangles: Vec::new(),
            edges: Vec::new(),
            billboards: Vec::new(),
            instances: Vec::new(),
            name: None,
        }
    }

    pub(crate) fn effect_program(radius_raw: u16) -> Vec<u8> {
        let mut program = vec![0_u8; 11];
        program[0] = 0x8E;
        program[4..8].copy_from_slice(&u32::from(radius_raw).to_le_bytes());
        program[8..10].copy_from_slice(&0_u16.to_le_bytes());
        program[10] = 0x88;
        program
    }

    pub(crate) fn model_cache(active_program: Vec<u8>) -> ResourceCache {
        let mut models = (0..=LEVEL_ONE_TYPE66_ACTIVE_MODEL_ID)
            .map(|index| model_entry(index, vec![0x88], Vec::new()))
            .collect::<Vec<_>>();
        models[LEVEL_ONE_TYPE66_ACTIVE_MODEL_ID] = model_entry(
            LEVEL_ONE_TYPE66_ACTIVE_MODEL_ID,
            active_program,
            vec![[0, 10, 20, 30]],
        );
        ResourceCache::new(vec![LevelState {
            source_path: "type66-progressive-test".to_owned(),
            system_level: Some(2),
            fixup_data: None,
            fixup_code: None,
            strings: Vec::new(),
            sprites: None,
            params: None,
            display_modes: None,
            fog_gradient: None,
            color_palettes: None,
            models: Some(ModelCollection {
                sub_blocks: Vec::new(),
                all_entries: models,
                stats: StreamStats::default(),
            }),
            anim_frames: None,
            terrain: None,
            anim_sound: None,
            collision: None,
            level: None,
            linkage: None,
        }])
    }

    fn frame<'a>(
        resources: &'a ResourceCache,
        elapsed_micros: u32,
    ) -> MainBaseType66ProductionFrame<'a> {
        MainBaseType66ProductionFrame {
            resources,
            elapsed_micros,
            retail_tick: 250,
            main_base_abort_active: false,
        }
    }

    fn resolve_factory_under_attack(id: usize) -> Option<&'static str> {
        (id == FACTORY_UNDER_ATTACK_TEXT_ID).then_some("\t<*,3000, 4,30, *>Factory under attack")
    }

    fn tick_attack_sample(
        manager: &mut EntityManager,
        owner: MainBaseType66ProductionOwner,
        resources: &ResourceCache,
        retail_tick: u32,
        under_attack: bool,
        main_base_abort_active: bool,
        world_fx: &mut WorldFx,
        notifications: &mut GameplayNotifications,
    ) -> MainBaseType66ProductionOwnerTick {
        manager
            .main_base_type66_production_entity_mut(owner.entity_id())
            .expect("the exact Type-66 owner remains live")
            .collision
            .last_hit_presentation_tick_at_0x34 =
            RetailRuntimeValue::Known(if under_attack { retail_tick } else { 0 });
        tick_main_base_type66_production_owner(
            manager,
            owner,
            MainBaseType66ProductionFrame {
                resources,
                elapsed_micros: 1,
                retail_tick,
                main_base_abort_active,
            },
            world_fx,
            notifications,
        )
    }

    fn continuing_owner_with_notification(
        tick: MainBaseType66ProductionOwnerTick,
        expected: WorkingFactoryNotificationOutcome,
    ) -> MainBaseType66ProductionOwner {
        assert!(matches!(
            &tick.outcome,
            MainBaseType66ProductionOutcome::Continuing {
                notification,
                model_effect_stages: 0,
                accepted_effect_points: 0,
                ..
            } if *notification == expected
        ));
        assert!(tick.explosion_lights.is_empty());
        tick.retained_owner
            .expect("a nonterminal callback retains linear owner custody")
    }

    pub(crate) fn set_terminal_predecessor(
        manager: &mut EntityManager,
        entity_id: u32,
        pickup_id: u32,
        animation_state_raw: u32,
        progressive_clock: i32,
    ) {
        let entity = manager
            .main_base_type66_production_entity_mut(entity_id)
            .unwrap();
        let RetailRuntimeValue::Known(Some(factory)) = &mut entity.base_factory_runtime else {
            panic!("exact factory state")
        };
        factory.progressive_death.elapsed_micros_raw = progressive_clock;
        let production = factory.production.as_mut().unwrap();
        production.current_scientists_raw = LEVEL_ONE_FACTORY_CAPACITY;
        production.spawned_pickup_handle = pickup_id;
        let live_owner = factory.live_owner.as_mut().unwrap();
        live_owner.animation_state_raw = animation_state_raw;
        live_owner.primary_animation_range = FactoryAnimationRange {
            start_raw_16_16: 0x1_0000,
            end_raw_16_16: 0x1_8000,
        };
        live_owner.secondary_animation_range = FactoryAnimationRange {
            start_raw_16_16: 0x1_0000,
            end_raw_16_16: 0x1_8000,
        };
    }

    #[test]
    fn missing_and_malformed_effect_models_block_before_any_live_or_rng_change() {
        let missing_cache = ResourceCache::new(Vec::new());
        let (mut manager, owner) = post_abort_owner();
        let before = snapshot(&manager, &owner);
        let mut world_fx = WorldFx::new();
        let mut rng_oracle = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let tick = tick_main_base_type66_production_owner(
            &mut manager,
            owner,
            frame(&missing_cache, 100_000),
            &mut world_fx,
            &mut notifications,
        );
        assert!(matches!(
            tick.outcome,
            MainBaseType66ProductionOutcome::Blocked {
                reason: MainBaseType66ProductionBlock::ActiveModelUnavailable {
                    model_id: LEVEL_ONE_TYPE66_ACTIVE_MODEL_ID
                },
                ..
            }
        ));
        let retained = tick.retained_owner.expect("resource block retains owner");
        assert_eq!(snapshot(&manager, &retained), before);
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            rng_oracle.next_shared_retail_random_u16()
        );

        let malformed_cache = model_cache(vec![0x8E]);
        let (mut manager, owner) = post_abort_owner();
        let before = snapshot(&manager, &owner);
        let mut world_fx = WorldFx::new();
        let mut rng_oracle = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let tick = tick_main_base_type66_production_owner(
            &mut manager,
            owner,
            frame(&malformed_cache, 100_000),
            &mut world_fx,
            &mut notifications,
        );
        assert!(matches!(
            tick.outcome,
            MainBaseType66ProductionOutcome::Blocked {
                reason: MainBaseType66ProductionBlock::StagedEffectWalk(
                    StagedEffectError::Truncated {
                        pc: 0,
                        opcode: 0x8E
                    }
                ),
                ..
            }
        ));
        let retained = tick.retained_owner.expect("model block retains owner");
        assert_eq!(snapshot(&manager, &retained), before);
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            rng_oracle.next_shared_retail_random_u16()
        );
    }

    #[test]
    fn unresolved_live_hit_tick_blocks_atomically_then_known_retry_commits_once() {
        let cache = model_cache(vec![0x88]);
        let (mut manager, owner) = post_abort_owner();
        let entity_id = owner.entity_id();
        manager
            .main_base_type66_production_entity_mut(entity_id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Unresolved;
        let before = snapshot(&manager, &owner);
        let mut world_fx = WorldFx::new();
        let mut rng_oracle = WorldFx::new();
        let mut notifications = GameplayNotifications::new();

        let blocked = tick_main_base_type66_production_owner(
            &mut manager,
            owner,
            frame(&cache, 1_000),
            &mut world_fx,
            &mut notifications,
        );
        assert!(matches!(
            blocked.outcome,
            MainBaseType66ProductionOutcome::Blocked {
                entity_id: actual,
                reason: MainBaseType66ProductionBlock::LastHitPresentationTickUnresolved,
            } if actual == entity_id
        ));
        let owner = blocked
            .retained_owner
            .expect("unresolved live hit evidence retains exact owner custody");
        assert_eq!(snapshot(&manager, &owner), before);
        assert!(blocked.explosion_lights.is_empty());
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            rng_oracle.next_shared_retail_random_u16()
        );
        let presentation = notifications.presentation(
            250,
            &mut TextTypewriterCadence::default(),
            resolve_factory_under_attack,
        );
        assert!(presentation.lines.is_empty());

        manager
            .main_base_type66_production_entity_mut(entity_id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(250);
        let retry = tick_main_base_type66_production_owner(
            &mut manager,
            owner,
            frame(&cache, 1_000),
            &mut world_fx,
            &mut notifications,
        );
        let owner = continuing_owner_with_notification(
            retry,
            WorkingFactoryNotificationOutcome::NotificationRequested,
        );
        let after = snapshot(&manager, &owner);
        assert_eq!(after.task.elapsed_ms(), 1);
        assert_eq!(after.task.next_tick_sequence(), 2);
        assert!(after.task.under_attack_notification_latched());
    }

    #[test]
    fn nonzero_progressive_config_drops_before_wrapper_notification_rng_or_live_state() {
        let cache = model_cache(effect_program(0x0123));
        let (mut manager, owner) = post_abort_owner();
        let entity_id = owner.entity_id();
        manager
            .main_base_type66_production_entity_mut(entity_id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(250);
        let visit = ActorTaskVisit {
            slot: ActorTaskSlot::Primary,
            task_id: owner.task_lease.task_id(),
        };
        let RetailRuntimeValue::Known(Some(factory)) = &mut manager
            .main_base_type66_production_entity_mut(entity_id)
            .unwrap()
            .base_factory_runtime
        else {
            panic!("exact factory runtime")
        };
        factory.progressive_death.config_flags_at_0x18 = 0x20;
        let before = snapshot_exact(&manager, entity_id, visit);
        let mut world_fx = WorldFx::new();
        let mut rng_oracle = WorldFx::new();
        let mut notifications = GameplayNotifications::new();

        let tick = tick_main_base_type66_production_owner(
            &mut manager,
            owner,
            frame(&cache, 100_000),
            &mut world_fx,
            &mut notifications,
        );

        assert!(matches!(
            tick.outcome,
            MainBaseType66ProductionOutcome::Dropped {
                entity_id: actual,
                reason: MainBaseType66ProductionDrop::ProgressiveDeathConfigMismatch {
                    actual: 0x20,
                },
            } if actual == entity_id
        ));
        assert!(tick.retained_owner.is_none());
        assert_eq!(snapshot_exact(&manager, entity_id, visit), before);
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            rng_oracle.next_shared_retail_random_u16()
        );
        assert!(notifications
            .presentation(
                250,
                &mut TextTypewriterCadence::default(),
                resolve_factory_under_attack,
            )
            .lines
            .is_empty());
    }

    #[test]
    fn stage_one_advances_exact_wrapper_latch_rng_and_linear_owner_once() {
        let cache = model_cache(effect_program(0x0123));
        let (mut manager, owner) = post_abort_owner();
        let entity_id = owner.entity_id();
        manager
            .main_base_type66_production_entity_mut(entity_id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(250);
        let visit = ActorTaskVisit {
            slot: ActorTaskSlot::Primary,
            task_id: owner.task_lease.task_id(),
        };
        let mut world_fx = WorldFx::new();
        let mut rng_oracle = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let accepted = i32::from(rng_oracle.next_shared_retail_random_u16() & 0xff) < 0x19;
        if accepted {
            for _ in 0..18 {
                let _ = rng_oracle.next_shared_retail_random_u16();
            }
        }

        let tick = tick_main_base_type66_production_owner(
            &mut manager,
            owner,
            frame(&cache, 100_000),
            &mut world_fx,
            &mut notifications,
        );
        assert_eq!(tick.explosion_lights.len(), usize::from(accepted));
        assert!(matches!(
            tick.outcome,
            MainBaseType66ProductionOutcome::Continuing {
                entity_id: actual_entity_id,
                visit: actual_visit,
                elapsed_ms: 100,
                notification: WorkingFactoryNotificationOutcome::NotificationRequested,
                under_attack_notification_requested: true,
                model_effect_stages: 1,
                accepted_effect_points,
            } if actual_entity_id == entity_id
                && actual_visit == visit
                && accepted_effect_points == usize::from(accepted)
        ));
        let retained = tick
            .retained_owner
            .expect("nonterminal frame retains owner");
        assert_eq!(retained.next_tick_sequence, 2);
        let after = snapshot(&manager, &retained);
        assert_eq!(after.task.elapsed_ms(), 100);
        assert_eq!(after.task.next_tick_sequence(), 2);
        assert!(after.task.under_attack_notification_latched());
        assert_eq!(after.factory.progressive_death.elapsed_micros_raw, 100_001);
        let live_owner = after.factory.live_owner.unwrap();
        assert_eq!(live_owner.state_version, 2);
        assert_eq!(live_owner.next_transaction_id_raw, 3);
        assert_eq!(
            after.wrapper,
            ActorTaskWrapperFlags {
                alive: true,
                in_callback: false
            }
        );
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            rng_oracle.next_shared_retail_random_u16()
        );
        assert!(notifications
            .presentation(
                250,
                &mut TextTypewriterCadence::default(),
                resolve_factory_under_attack,
            )
            .lines
            .is_empty());
        let presentation = notifications.presentation(
            252,
            &mut TextTypewriterCadence::default(),
            resolve_factory_under_attack,
        );
        assert_eq!(presentation.lines.len(), 1);
        assert_eq!(
            presentation.lines[0].string_id,
            FACTORY_UNDER_ATTACK_TEXT_ID
        );
        assert_eq!(presentation.lines[0].text, "F");
    }

    #[test]
    fn under_attack_text_respects_abort_latch_repeat_and_no_attack_rearm() {
        let cache = ResourceCache::new(Vec::new());
        let (mut manager, owner) = post_abort_owner();
        let mut world_fx = WorldFx::new();
        let mut rng_oracle = WorldFx::new();
        let mut notifications = GameplayNotifications::new();

        let tick = tick_attack_sample(
            &mut manager,
            owner,
            &cache,
            250,
            true,
            true,
            &mut world_fx,
            &mut notifications,
        );
        let owner = continuing_owner_with_notification(
            tick,
            WorkingFactoryNotificationOutcome::SuppressedByMainBaseAbort,
        );
        assert!(!snapshot(&manager, &owner)
            .task
            .under_attack_notification_latched());
        assert!(notifications
            .presentation(
                250,
                &mut TextTypewriterCadence::default(),
                resolve_factory_under_attack,
            )
            .lines
            .is_empty());

        let tick = tick_attack_sample(
            &mut manager,
            owner,
            &cache,
            260,
            true,
            false,
            &mut world_fx,
            &mut notifications,
        );
        let owner = continuing_owner_with_notification(
            tick,
            WorkingFactoryNotificationOutcome::NotificationRequested,
        );
        assert!(snapshot(&manager, &owner)
            .task
            .under_attack_notification_latched());
        assert!(notifications
            .presentation(
                260,
                &mut TextTypewriterCadence::default(),
                resolve_factory_under_attack,
            )
            .lines
            .is_empty());
        let presentation = notifications.presentation(
            262,
            &mut TextTypewriterCadence::default(),
            resolve_factory_under_attack,
        );
        assert_eq!(presentation.lines.len(), 1);
        assert_eq!(
            presentation.lines[0].string_id,
            FACTORY_UNDER_ATTACK_TEXT_ID
        );
        assert_eq!(presentation.lines[0].text, "F");

        let tick = tick_attack_sample(
            &mut manager,
            owner,
            &cache,
            270,
            true,
            false,
            &mut world_fx,
            &mut notifications,
        );
        let owner = continuing_owner_with_notification(
            tick,
            WorkingFactoryNotificationOutcome::AlreadyLatched,
        );
        assert!(snapshot(&manager, &owner)
            .task
            .under_attack_notification_latched());
        let presentation = notifications.presentation(
            270,
            &mut TextTypewriterCadence::default(),
            resolve_factory_under_attack,
        );
        assert_eq!(presentation.lines.len(), 1);
        assert_eq!(
            presentation.lines[0].text, "Factor",
            "an already-latched callback must not restart the direct slot"
        );

        let tick = tick_attack_sample(
            &mut manager,
            owner,
            &cache,
            280,
            false,
            false,
            &mut world_fx,
            &mut notifications,
        );
        let owner = continuing_owner_with_notification(
            tick,
            WorkingFactoryNotificationOutcome::NoAttackAndLatchCleared,
        );
        assert!(!snapshot(&manager, &owner)
            .task
            .under_attack_notification_latched());

        let tick = tick_attack_sample(
            &mut manager,
            owner,
            &cache,
            290,
            true,
            false,
            &mut world_fx,
            &mut notifications,
        );
        let owner = continuing_owner_with_notification(
            tick,
            WorkingFactoryNotificationOutcome::NotificationRequested,
        );
        assert!(snapshot(&manager, &owner)
            .task
            .under_attack_notification_latched());
        assert!(
            notifications
                .presentation(
                    290,
                    &mut TextTypewriterCadence::default(),
                    resolve_factory_under_attack,
                )
                .lines
                .is_empty(),
            "a fresh attack must restart direct text"
        );
        let presentation = notifications.presentation(
            292,
            &mut TextTypewriterCadence::default(),
            resolve_factory_under_attack,
        );
        assert_eq!(presentation.lines.len(), 1);
        assert_eq!(
            presentation.lines[0].string_id,
            FACTORY_UNDER_ATTACK_TEXT_ID
        );
        assert_eq!(
            presentation.lines[0].text, "F",
            "a fresh attack after a no-attack sample must restart direct text"
        );
        assert_eq!(owner.next_tick_sequence, 6);
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            rng_oracle.next_shared_retail_random_u16()
        );
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct ActionObservation {
        phase: FactoryProductionOwnerActionPhase,
        progressive_clock: i32,
        health: i32,
        active_model: Option<usize>,
        current_scientists: i32,
        scientist_capacity: i32,
        primary_range: FactoryAnimationRange,
        secondary_range: FactoryAnimationRange,
        wrapper: ActorTaskWrapperFlags,
        pickup_pending: bool,
    }

    #[test]
    fn crossed_stage_31_and_32_preserves_action_order_and_terminal_same_wrapper() {
        let cache = model_cache(effect_program(0x0234));
        let (mut manager, owner) = post_abort_owner();
        let entity_id = owner.entity_id();
        manager
            .main_base_type66_production_entity_mut(entity_id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(250);
        let task_id = owner.task_lease.task_id();
        let pickup_id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 61)
            .expect("composition fixture contains captured Power-Ups")
            .id;
        set_terminal_predecessor(&mut manager, entity_id, pickup_id, 7, 3_000_001);
        let factory_position = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap()
            .position;
        let mut observations = Vec::new();
        let mut world_fx = WorldFx::new();
        let mut rng_oracle = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        for _ in 0..18 {
            let _ = rng_oracle.next_shared_retail_random_u16();
        }

        let tick = tick_main_base_type66_production_owner_with_observer(
            &mut manager,
            owner,
            frame(&cache, 200_000),
            &mut world_fx,
            &mut notifications,
            |manager, phase| {
                let entity = manager
                    .iter_all()
                    .find(|entity| entity.id == entity_id)
                    .unwrap();
                let RetailRuntimeValue::Known(Some(factory)) = entity.base_factory_runtime else {
                    panic!("exact factory state")
                };
                let production = factory.production.unwrap();
                let live_owner = factory.live_owner.unwrap();
                observations.push(ActionObservation {
                    phase,
                    progressive_clock: factory.progressive_death.elapsed_micros_raw,
                    health: match entity.collision.health_raw {
                        RetailRuntimeValue::Known(health) => health,
                        RetailRuntimeValue::Unresolved => panic!("exact health"),
                    },
                    active_model: entity.model_index,
                    current_scientists: production.current_scientists_raw,
                    scientist_capacity: production.scientist_capacity_raw,
                    primary_range: live_owner.primary_animation_range,
                    secondary_range: live_owner.secondary_animation_range,
                    wrapper: entity.actor_tasks.wrapper_flags(task_id).unwrap(),
                    pickup_pending: manager.is_power_up_destroy_pending(pickup_id),
                });
            },
        );

        assert!(matches!(
            tick.outcome,
            MainBaseType66ProductionOutcome::Terminal {
                entity_id: actual,
                visit: ActorTaskVisit {
                    slot: ActorTaskSlot::Primary,
                    task_id: actual_task,
                },
                elapsed_ms: 200,
                notification: WorkingFactoryNotificationOutcome::NotificationRequested,
                under_attack_notification_requested: true,
                progressive_death_presentation_requested: true,
                model_effect_stages: 1,
                accepted_effect_points: 1,
                death_sound_id: LEVEL_ONE_TYPE66_DEATH_SOUND_ID,
            } if actual == entity_id && actual_task == task_id
        ));
        assert!(tick.retained_owner.is_none());
        assert_eq!(tick.explosion_lights.len(), 1);
        assert_eq!(
            observations
                .iter()
                .map(|observation| observation.phase)
                .collect::<Vec<_>>(),
            [
                FactoryProductionOwnerActionPhase::ProgressiveModelEffect,
                FactoryProductionOwnerActionPhase::QueueProgressivePickupDeferredDestroy,
                FactoryProductionOwnerActionPhase::InvokeProgressiveFactoryDeath,
                FactoryProductionOwnerActionPhase::DispatchProgressiveDeathPresentation,
                FactoryProductionOwnerActionPhase::ApplyAnimationTransition,
            ]
        );
        assert!(observations
            .iter()
            .all(|observation| observation.wrapper.in_callback));
        assert_eq!(observations[0].progressive_clock, 3_200_001);
        assert_eq!(observations[0].health, PROGRESSION_REVIVE_HEALTH_RAW);
        assert_eq!(observations[1].progressive_clock, -1);
        assert!(!observations[1].pickup_pending);
        assert_eq!(observations[2].progressive_clock, -1);
        assert!(observations[2].pickup_pending);
        assert_eq!(observations[2].health, PROGRESSION_REVIVE_HEALTH_RAW);
        let reset_range = FactoryAnimationRange {
            start_raw_16_16: 0,
            end_raw_16_16: 0x1_0000,
        };
        assert_eq!(observations[3].health, 0);
        assert_eq!(observations[3].active_model, Some(225));
        assert_eq!(observations[3].current_scientists, 0);
        assert_eq!(observations[3].scientist_capacity, 0);
        assert_eq!(observations[3].primary_range, reset_range);
        assert_eq!(observations[3].secondary_range, reset_range);

        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            Some(task_id)
        );
        assert_eq!(
            entity.actor_tasks.wrapper_flags(task_id),
            Some(ActorTaskWrapperFlags {
                alive: true,
                in_callback: false,
            })
        );
        let Some(ActorTaskRuntime::WorkingFactory(task)) = entity.actor_tasks.task_state(task_id)
        else {
            panic!("terminal callback leaves the same Primary wrapper installed")
        };
        assert_eq!(task.next_tick_sequence(), 1);
        assert_eq!(task.elapsed_ms(), 200);
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            entity.collision.active_model_slot(),
            RetailRuntimeValue::Known(1)
        );
        assert_eq!(entity.model_index, Some(225));
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
        let RetailRuntimeValue::Known(Some(factory)) = entity.base_factory_runtime else {
            panic!("factory state survives terminal callback")
        };
        assert_eq!(factory.progressive_death.elapsed_micros_raw, -1);
        let production = factory.production.unwrap();
        assert_eq!(production.current_scientists_raw, 0);
        assert_eq!(production.scientist_capacity_raw, 0);
        assert_eq!(production.spawned_pickup_handle, pickup_id);
        assert_eq!(factory.current_scientists, 0);
        assert_eq!(factory.required_scientists, 0);
        let live_owner = factory.live_owner.unwrap();
        assert_eq!(live_owner.animation_state_raw, 0);
        assert_eq!(live_owner.primary_animation_range, reset_range);
        assert_eq!(live_owner.secondary_animation_range, reset_range);
        assert_eq!(live_owner.state_version, 2);
        assert_eq!(live_owner.next_transaction_id_raw, 3);
        assert_eq!(manager.pending_power_up_destroy_ids(), [pickup_id]);
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            rng_oracle.next_shared_retail_random_u16()
        );
        world_fx.process_pending();
        assert!(world_fx.take_positional_sounds().iter().any(|sound| {
            sound.sound_id == usize::from(LEVEL_ONE_TYPE66_DEATH_SOUND_ID)
                && sound.position == factory_position
                && sound.frequency_q16 == 0x1_0000
        }));
    }

    #[test]
    fn terminal_pickup_zero_missing_clean_and_already_pending_follow_exact_semantics() {
        #[derive(Clone, Copy)]
        enum Case {
            Zero,
            Missing,
            Clean,
            AlreadyPending,
        }

        let cache = model_cache(vec![0x88]);
        for case in [Case::Zero, Case::Missing, Case::Clean, Case::AlreadyPending] {
            let (mut manager, owner) = post_abort_owner();
            let entity_id = owner.entity_id();
            let live_pickup_id = manager
                .iter_all()
                .find(|entity| entity.entity_type == 61)
                .unwrap()
                .id;
            let pickup_id = match case {
                Case::Zero => 0,
                Case::Missing => u32::MAX,
                Case::Clean | Case::AlreadyPending => live_pickup_id,
            };
            if matches!(case, Case::AlreadyPending) {
                manager.queue_power_up_destroys(&[live_pickup_id]);
                assert_eq!(manager.pending_power_up_destroy_ids(), [live_pickup_id]);
            }
            set_terminal_predecessor(&mut manager, entity_id, pickup_id, 0, 3_100_001);
            let mut world_fx = WorldFx::new();
            let mut rng_oracle = WorldFx::new();
            let mut notifications = GameplayNotifications::new();
            let tick = tick_main_base_type66_production_owner(
                &mut manager,
                owner,
                frame(&cache, 100_000),
                &mut world_fx,
                &mut notifications,
            );
            assert!(matches!(
                tick.outcome,
                MainBaseType66ProductionOutcome::Terminal {
                    model_effect_stages: 0,
                    accepted_effect_points: 0,
                    ..
                }
            ));
            assert!(tick.retained_owner.is_none());
            let expected_pending: &[u32] = match case {
                Case::Zero | Case::Missing => &[],
                Case::Clean | Case::AlreadyPending => &[live_pickup_id],
            };
            assert_eq!(manager.pending_power_up_destroy_ids(), expected_pending);
            assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
            assert_eq!(
                world_fx.next_shared_retail_random_u16(),
                rng_oracle.next_shared_retail_random_u16()
            );
        }
    }

    #[test]
    fn terminal_pickup_wrong_type_and_pending_queue_mismatch_block_atomically() {
        let cache = model_cache(vec![0x88]);

        let (mut manager, owner) = post_abort_owner();
        let entity_id = owner.entity_id();
        set_terminal_predecessor(&mut manager, entity_id, entity_id, 0, 3_100_001);
        let before = snapshot(&manager, &owner);
        let mut world_fx = WorldFx::new();
        let mut rng_oracle = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let tick = tick_main_base_type66_production_owner(
            &mut manager,
            owner,
            frame(&cache, 100_000),
            &mut world_fx,
            &mut notifications,
        );
        assert!(matches!(
            tick.outcome,
            MainBaseType66ProductionOutcome::Blocked {
                reason: MainBaseType66ProductionBlock::PickupWrongEntityType {
                    entity_id: actual,
                    actual: LEVEL_ONE_TYPE66_ENTITY_TYPE,
                },
                ..
            } if actual == entity_id
        ));
        let retained = tick.retained_owner.unwrap();
        assert_eq!(snapshot(&manager, &retained), before);
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            rng_oracle.next_shared_retail_random_u16()
        );

        let (mut manager, owner) = post_abort_owner();
        let entity_id = owner.entity_id();
        let pickup_id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 61)
            .unwrap()
            .id;
        manager
            .entity_mut_for_test(pickup_id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(
                DEFERRED_DESTROY_PENDING_STATE_BIT,
                DEFERRED_DESTROY_PENDING_STATE_BIT,
            );
        set_terminal_predecessor(&mut manager, entity_id, pickup_id, 0, 3_100_001);
        let before = snapshot(&manager, &owner);
        let mut world_fx = WorldFx::new();
        let mut rng_oracle = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let tick = tick_main_base_type66_production_owner(
            &mut manager,
            owner,
            frame(&cache, 100_000),
            &mut world_fx,
            &mut notifications,
        );
        assert!(matches!(
            tick.outcome,
            MainBaseType66ProductionOutcome::Blocked {
                reason:
                    MainBaseType66ProductionBlock::PickupDeferredStateQueueMismatch {
                        entity_id: actual,
                        pending_state: true,
                        queued: false,
                    },
                ..
            } if actual == pickup_id
        ));
        let retained = tick.retained_owner.unwrap();
        assert_eq!(snapshot(&manager, &retained), before);
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            rng_oracle.next_shared_retail_random_u16()
        );
    }

    #[test]
    fn stale_sequence_actor_allocation_and_primary_custody_drop_without_rng() {
        let empty_cache = ResourceCache::new(Vec::new());

        let (mut manager, owner) = post_abort_owner();
        let entity_id = owner.entity_id();
        let task_id = owner.task_lease.task_id();
        let Some(ActorTaskRuntime::WorkingFactory(task)) = manager
            .main_base_type66_production_entity_mut(entity_id)
            .unwrap()
            .actor_tasks
            .task_state_mut(task_id)
        else {
            panic!("exact task")
        };
        task.advance_tick_sequence();
        let mut world_fx = WorldFx::new();
        let mut rng_oracle = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let tick = tick_main_base_type66_production_owner(
            &mut manager,
            owner,
            frame(&empty_cache, 1),
            &mut world_fx,
            &mut notifications,
        );
        assert!(matches!(
            tick.outcome,
            MainBaseType66ProductionOutcome::Dropped {
                reason: MainBaseType66ProductionDrop::TaskSequenceMismatch {
                    expected: 2,
                    actual: 1,
                },
                ..
            }
        ));
        assert!(tick.retained_owner.is_none());
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            rng_oracle.next_shared_retail_random_u16()
        );

        let (mut manager, owner) = post_abort_owner();
        let entity_id = owner.entity_id();
        let task_id = owner.task_lease.task_id();
        let stale_actor = MainBaseAbortActorLease {
            entity_id,
            allocation_identity: owner.task_lease.actor().allocation_identity + 1,
        };
        let stale_lease = MainBaseType66WorkingFactoryTaskLease::issue(
            stale_actor,
            task_id,
            owner.task_lease.owner_allocation_identity(),
        );
        let mut world_fx = WorldFx::new();
        let mut rng_oracle = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let tick = tick_main_base_type66_production_owner(
            &mut manager,
            MainBaseType66ProductionOwner::adopt(stale_lease),
            frame(&empty_cache, 1),
            &mut world_fx,
            &mut notifications,
        );
        assert!(matches!(
            tick.outcome,
            MainBaseType66ProductionOutcome::Dropped {
                reason: MainBaseType66ProductionDrop::ActorLeaseMismatch { .. },
                ..
            }
        ));
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            rng_oracle.next_shared_retail_random_u16()
        );

        let (mut manager, owner) = post_abort_owner();
        let entity_id = owner.entity_id();
        let RetailRuntimeValue::Known(Some(factory)) = &mut manager
            .main_base_type66_production_entity_mut(entity_id)
            .unwrap()
            .base_factory_runtime
        else {
            panic!("exact factory")
        };
        factory.live_owner.as_mut().unwrap().allocation_identity += 1;
        let mut world_fx = WorldFx::new();
        let mut rng_oracle = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let tick = tick_main_base_type66_production_owner(
            &mut manager,
            owner,
            frame(&empty_cache, 1),
            &mut world_fx,
            &mut notifications,
        );
        assert!(matches!(
            tick.outcome,
            MainBaseType66ProductionOutcome::Dropped {
                reason: MainBaseType66ProductionDrop::FactoryLiveOwnerMismatch,
                ..
            }
        ));
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            rng_oracle.next_shared_retail_random_u16()
        );

        let (mut manager, owner) = post_abort_owner();
        let entity_id = owner.entity_id();
        let allocation_identity = owner.task_lease.owner_allocation_identity();
        manager
            .main_base_type66_production_entity_mut(entity_id)
            .unwrap()
            .actor_tasks
            .replace_prepared(
                ActorTaskSlot::Primary,
                PreparedActorTask::new(ActorTaskRuntime::WorkingFactory(
                    WorkingFactoryTaskState::new(allocation_identity),
                )),
            );
        let mut world_fx = WorldFx::new();
        let mut rng_oracle = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let tick = tick_main_base_type66_production_owner(
            &mut manager,
            owner,
            frame(&empty_cache, 1),
            &mut world_fx,
            &mut notifications,
        );
        assert!(matches!(
            tick.outcome,
            MainBaseType66ProductionOutcome::Dropped {
                reason: MainBaseType66ProductionDrop::PrimaryTaskMissingOrReplaced,
                ..
            }
        ));
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            rng_oracle.next_shared_retail_random_u16()
        );
    }

    #[test]
    fn replacement_during_effect_reclaims_old_wrapper_and_issues_no_authority() {
        let cache = model_cache(effect_program(0x0100));
        let (mut manager, owner) = post_abort_owner();
        let entity_id = owner.entity_id();
        let old_task_id = owner.task_lease.task_id();
        let allocation_identity = owner.task_lease.owner_allocation_identity();
        let mut replacement_task_id = None;
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();

        let tick = tick_main_base_type66_production_owner_with_observer(
            &mut manager,
            owner,
            frame(&cache, 100_000),
            &mut world_fx,
            &mut notifications,
            |manager, phase| {
                if phase != FactoryProductionOwnerActionPhase::ProgressiveModelEffect
                    || replacement_task_id.is_some()
                {
                    return;
                }
                let entity = manager
                    .main_base_type66_production_entity_mut(entity_id)
                    .unwrap();
                assert_eq!(
                    entity.actor_tasks.wrapper_flags(old_task_id),
                    Some(ActorTaskWrapperFlags {
                        alive: true,
                        in_callback: true,
                    })
                );
                replacement_task_id = Some(entity.actor_tasks.replace_prepared(
                    ActorTaskSlot::Primary,
                    PreparedActorTask::new(ActorTaskRuntime::WorkingFactory(
                        WorkingFactoryTaskState::new(allocation_identity),
                    )),
                ));
            },
        );

        assert!(matches!(
            tick.outcome,
            MainBaseType66ProductionOutcome::Dropped {
                entity_id: actual,
                reason: MainBaseType66ProductionDrop::WrapperReplacedDuringCallback,
            } if actual == entity_id
        ));
        assert!(tick.retained_owner.is_none());
        let replacement_task_id = replacement_task_id.expect("observer replaced Primary");
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert_eq!(entity.actor_tasks.wrapper_flags(old_task_id), None);
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            Some(replacement_task_id)
        );
        assert_eq!(
            entity.actor_tasks.wrapper_flags(replacement_task_id),
            Some(ActorTaskWrapperFlags {
                alive: true,
                in_callback: false,
            })
        );
        let RetailRuntimeValue::Known(Some(factory)) = entity.base_factory_runtime else {
            panic!("committed owner prefix and completion survive wrapper replacement")
        };
        assert_eq!(factory.progressive_death.elapsed_micros_raw, 100_001);
        assert_eq!(factory.live_owner.unwrap().state_version, 2);
    }
}
