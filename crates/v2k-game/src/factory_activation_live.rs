//! Shared Working Factory scientist intake and bounded Level-1 production.
//!
//! This module binds the already-closed scientist-delivery transaction and
//! inner `FUN_00419010` owner to real [`EntityManager`] allocations. Arrival
//! itself is explicit: the caller must supply a live authenticated worker
//! whose Go-To-Job task still targets the factory. After the tagged `0xA300`
//! callback, the live coordinator preflights and applies the remaining pair
//! visit: skip the known-null class-54 candidate behavior, skip the factory's
//! null component slots, run the scientist's slot-0 `FUN_00402DA0`, and omit
//! physical response because `0xA300` already cleared it. No proximity
//! shortcut is inferred here.
//! Native intake authenticates the actual descriptor-backed Type66 allocation;
//! its normal production remains with `intro2_type66`, independently of the
//! retained first-world production adapter below.
//!
//! Production admits the exact Level-1 factory through its first type-61
//! product, five-second delivery, positive-stock pickup presence / cooldown /
//! repeat, two finite-stock worker ejections, idle-empty, and the already-closed
//! damaged-repair / cached-health owner branches. Authored Level-1 cooldown is
//! still negative, so remaining stock becomes zero at the first product; the
//! presence branch is the live `FUN_0043a580` lookup used whenever stock stays
//! positive. The nonzero-multiplayer phase-1 presentation callback,
//! progressive-death frames, understaffed death, and behavior wrapper
//! `FUN_00425C60` remain separate.

use crate::intro2_type8::NativeWorkerProfile;
use v2k_formats::terrain::TerrainGrid;

use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::actor_task_owner::ActorTaskSlot;
use crate::entity::{raw_position_world, BaseFactoryRuntimeState, Entity, EntityManager};
use crate::entity_behavior::PairContactCallbackPolicy;
use crate::entity_collision_state::{
    RetailRuntimeValue, RetailStateWord, DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
};
use crate::entity_pair_callbacks::{
    LifterSourceSnapshot, PairBehaviorCallbackReturn, PairCallbackEntitySnapshot,
};
use crate::factory_delivery::{
    FactoryDeliveryAction, FactoryDeliveryAdmissionRejection, FactoryDeliveryFactorySnapshot,
    FactoryDeliveryMachine, FactoryDeliveryPairSuffix, FactoryDeliveryPoll, FactoryDeliveryRequest,
    FactoryDeliveryResume, FactoryDeliveryScientistSnapshot, FactoryDeliveryTransactionId,
};
use crate::factory_pair_suffix::{
    apply_factory_pair_suffix, plan_factory_pair_suffix, FactoryPairSuffixCommitError,
    FactoryPairSuffixOutcome, FactoryPairSuffixPlan, FactoryPairSuffixUnresolved,
};
use crate::factory_production::{
    FactoryProductionPhase, FactoryProductionRuntime, FACTORY_HIGH_THRESHOLD_DIRECT_TEXT_ID,
    FACTORY_MATERIALISER_ENTITY_TYPE, FACTORY_OUTPUT_CONVERSION_HUD_RESOURCE_ID,
    FACTORY_OUTPUT_CONVERSION_SOUND_ID, FACTORY_PICKUP_ENTITY_TYPE,
};
use crate::factory_production_live::{
    FactoryAnimationTransition, FactoryEntitySpawnRequest, FactoryEntitySpawnResult,
    FactoryPickupPresence, FactoryProductionAction, FactoryProductionActionPhase,
    FactoryProductionEntityVersion, FactoryProductionTransactionId, FactorySpawnRole,
    FactorySpawnedEntity, FactoryStatusPublication, FACTORY_ENTITY_DIRTY_FLAG,
    FACTORY_ENTITY_LIFETIME_OFFSET,
};
use crate::factory_production_owner::{
    FactoryProductionOwnerAction, FactoryProductionOwnerActionPhase,
    FactoryProductionOwnerAdmissionRejection, FactoryProductionOwnerExternalBlock,
    FactoryProductionOwnerFrameRequest, FactoryProductionOwnerMachine, FactoryProductionOwnerPoll,
    FactoryProductionOwnerResume, FactoryProductionOwnerTransactionId,
    FACTORY_OWNER_CAPACITY_DIRECT_TEXT_ID, FACTORY_OWNER_CAPACITY_HUD_RESOURCE_ID,
};
use crate::gameplay_notifications::GameplayNotifications;
use crate::world_fx::WorldFx;

const LEVEL_ONE_FACTORY_SPAWN_INDEX: usize = 23;
const LEVEL_ONE_FACTORY_ENTITY_TYPE: u32 = 66;
const LEVEL_ONE_FACTORY_MODEL_ID: usize = 227;
const LEVEL_ONE_FACTORY_BEHAVIOR_CLASS_ID: u8 = 39;
const LEVEL_ONE_FACTORY_POSITION_RAW: [i16; 3] = [0x5700, -0x0300, 0x3A00];
const LEVEL_ONE_FACTORY_MAX_HEALTH_RAW: i32 = 99_999;
const LEVEL_ONE_FACTORY_OUTPUT_PAYLOAD: u32 = 0x0001_F412;
const LEVEL_ONE_FACTORY_CAPACITY: i32 = 2;
const LEVEL_ONE_FACTORY_PRODUCTION_THRESHOLD_US: i32 = 6_000_000;
const LEVEL_ONE_FACTORY_DELIVERY_DURATION_US: i32 = 5_000_000;
const LEVEL_ONE_FACTORY_COOLDOWN_US: i32 = -1_000;
const LEVEL_ONE_FACTORY_UNDERSTAFFED_LIMIT_US: i32 = 0;
const LEVEL_ONE_WORLD_STYLE: u32 = 1;
#[cfg(test)]
const SCIENTIST_ENTITY_TYPE: u32 = 8;
const SCIENTIST_FACTORY_CAPABILITY_BIT: u32 = 0x400;
const FACTORY_CALLBACK_STATE_MASK: u32 = DYING_STATE_BIT | REMOTE_OWNED_STATE_BIT;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryActivationLiveError {
    FactoryUnavailable {
        factory_id: u32,
    },
    FactoryIdentityMismatch {
        factory_id: u32,
    },
    FactoryBehaviorUnavailable {
        factory_id: u32,
    },
    FactoryRuntimeUnavailable {
        factory_id: u32,
    },
    FactoryLiveOwnerUnavailable {
        factory_id: u32,
    },
    FactoryCallbackStateUnavailable {
        factory_id: u32,
    },
    FactoryCallbackIneligible {
        factory_id: u32,
    },
    FactoryLeaseStale {
        factory_id: u32,
    },
    FactoryStateVersionExhausted {
        factory_id: u32,
    },
    FactoryTransactionSpaceExhausted {
        factory_id: u32,
        next_transaction_id_raw: u64,
    },
    PickupCallbackStateUnavailable {
        pickup_id: u32,
    },
    PickupUnavailable {
        pickup_id: u32,
    },
    RemainingStockRejected {
        remaining_stock_raw: i32,
    },
    ScientistUnavailable {
        scientist_id: u32,
    },
    ScientistIdentityMismatch {
        scientist_id: u32,
    },
    ScientistCallbackStateUnavailable {
        scientist_id: u32,
    },
    ScientistCallbackIneligible {
        scientist_id: u32,
    },
    ScientistTaskMismatch {
        scientist_id: u32,
        factory_id: u32,
    },
    ScientistDestroyAlreadyPending {
        scientist_id: u32,
    },
    ScientistLeaseStale {
        scientist_id: u32,
    },
    PickupConstructorUnavailable,
    FactoryStaffingOutsideBound {
        current: i32,
        capacity: i32,
    },
    FactoryNotAtFullHealth {
        current: i32,
        maximum: i32,
    },
    FactoryProgressiveDeathActive {
        elapsed_micros_raw: i32,
    },
    FactoryCachedHealthMismatch {
        cached: i32,
        current: i32,
    },
    FactoryPublishedProjectionMismatch {
        published_current: u16,
        published_capacity: u16,
        production_current: i32,
        production_capacity: i32,
    },
    UnsupportedProductionPhase(FactoryProductionPhase),
    DeliveryAdmission(FactoryDeliveryAdmissionRejection),
    ProductionAdmission(FactoryProductionOwnerAdmissionRejection),
    UnexpectedDeliveryAction,
    PairSuffix(FactoryPairSuffixUnresolved),
    UnexpectedProductionAction(FactoryProductionOwnerActionPhase),
    UnsupportedConvertedOutput {
        entity_type: u32,
    },
    AttractAttentionNotification(
        crate::gameplay_notifications::FreshLevel1Type9NotificationDrainError,
    ),
    ProductionBlocked {
        phase: FactoryProductionOwnerActionPhase,
        reason: FactoryProductionOwnerExternalBlock,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryActivationLiveEvent {
    Operation33 {
        position_raw: [i16; 3],
        source_factory_id: u32,
    },
    DeliveryHudResource {
        event_id: u8,
    },
    StaffingCommitted {
        before: i32,
        after: i32,
    },
    CapacityHudResource {
        event_id: u8,
    },
    ScientistDestroyQueued {
        scientist_id: u32,
    },
    ProductSpawned {
        product: FactorySpawnedEntity,
    },
    ProductOwnerLinked {
        product_id: u32,
        factory_id: u32,
    },
    ConversionHudResource {
        resource_id: u16,
    },
    ConvertedOutputSpawned {
        output: FactorySpawnedEntity,
    },
    ConvertedOutputOwnerLinked {
        output_id: u32,
        factory_id: u32,
    },
    MaterialiserSpawned {
        materialiser: FactorySpawnedEntity,
    },
    MaterialiserOutputLinked {
        materialiser_id: u32,
        output_id: u32,
    },
    ConversionPositionalSound {
        sound_id: u16,
        position_raw: [i16; 3],
    },
    DirectText {
        string_id: u16,
    },
    LifetimeCleared,
    AnimationTransition(FactoryAnimationTransition),
    StatusPublished(FactoryStatusPublication),
    EntityMarkedDirty {
        flag: u32,
    },
    CountdownPublished {
        remaining_seconds_raw: i32,
    },
    PickupPresence {
        pickup_id: u32,
        presence: FactoryPickupPresence,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactoryScientistDeliveryOutcome {
    pub factory_id: u32,
    pub scientist_id: u32,
    pub factory_runtime: FactoryProductionRuntime,
    pub callback_return: PairBehaviorCallbackReturn,
    pub pair_suffix: FactoryDeliveryPairSuffix,
    pub visit_suffix: FactoryPairSuffixOutcome,
    pub events: Vec<FactoryActivationLiveEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactoryProductionFrameOutcome {
    pub factory_id: u32,
    pub production: FactoryProductionRuntime,
    pub product: Option<FactorySpawnedEntity>,
    pub converted_output: Option<FactorySpawnedEntity>,
    pub materialiser: Option<FactorySpawnedEntity>,
    pub status_published: bool,
    pub dirty_flag_written: bool,
    pub events: Vec<FactoryActivationLiveEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FactoryLease {
    construction: FactoryConstructionPolicy,
    allocation: crate::main_base_abort::MainBaseAbortActorLease,
    version: FactoryProductionEntityVersion,
    base: BaseFactoryRuntimeState,
    current_health_raw: i32,
    callback_state: RetailStateWord,
    position_raw: [i16; 3],
    pickup_spawn_offset_raw: [i16; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FactoryConstructionPolicy {
    RetainedLevelOne,
    NativeType66,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScientistLease {
    entity_id: u32,
    entity_type: u32,
    primary_task: crate::actor_task_owner::ActorTaskId,
    behavior: RetailRuntimeValue<Option<crate::entity_behavior::BehaviorContextRuntime>>,
    native_runtime: crate::factory_pair_suffix::FactoryScientistRuntime,
    allocation_identity: u64,
    state_version: u64,
    position_raw: [i16; 3],
    state_flags_raw: u32,
    capability_flags_raw: u32,
    target_factory_id: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PreparedFactoryScientistArrival {
    factory: FactoryLease,
    scientist: ScientistLease,
    machine: FactoryDeliveryMachine,
    pair_suffix: FactoryPairSuffixPlan,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PreparedFactoryProductionFrame {
    factory: FactoryLease,
    machine: FactoryProductionOwnerMachine,
}

/// Authenticate one explicit scientist/factory arrival and close the complete
/// delivery journal before operation `0x33` can escape.
pub fn prepare_factory_scientist_arrival(
    entities: &EntityManager,
    factory_id: u32,
    scientist_id: u32,
) -> Result<PreparedFactoryScientistArrival, FactoryActivationLiveError> {
    let factory = snapshot_factory_for_delivery(entities, factory_id)?;
    let scientist = snapshot_arrived_scientist(entities, factory_id, scientist_id)?;
    let production = factory
        .base
        .production
        .ok_or(FactoryActivationLiveError::FactoryRuntimeUnavailable { factory_id })?;
    if !level_one_entry_projection_is_authentic(factory.base, production) {
        return Err(
            FactoryActivationLiveError::FactoryPublishedProjectionMismatch {
                published_current: factory.base.current_scientists,
                published_capacity: factory.base.required_scientists,
                production_current: production.current_scientists_raw,
                production_capacity: production.scientist_capacity_raw,
            },
        );
    }
    let request = FactoryDeliveryRequest {
        factory: FactoryDeliveryFactorySnapshot {
            lifter: LifterSourceSnapshot {
                id: factory_id,
                state_flags_raw: factory.callback_state.known_value_bits(),
            },
            allocation_identity: factory.version.allocation_identity,
            state_version: factory.version.state_version,
            type_extension_present: true,
            behavior_state_present: true,
            delivery_enabled: true,
            production,
        },
        scientist: FactoryDeliveryScientistSnapshot {
            entity: PairCallbackEntitySnapshot {
                id: scientist.entity_id,
                position_raw: scientist.position_raw,
                state_flags_raw: scientist.state_flags_raw,
                capability_flags_raw: scientist.capability_flags_raw,
            },
            allocation_identity: scientist.allocation_identity,
            state_version: scientist.state_version,
        },
    };
    let transaction_raw = factory
        .base
        .live_owner
        .expect("factory snapshot requires live owner")
        .next_transaction_id_raw;
    if transaction_raw == 0 || transaction_raw.checked_add(1).is_none() {
        return Err(
            FactoryActivationLiveError::FactoryTransactionSpaceExhausted {
                factory_id,
                next_transaction_id_raw: transaction_raw,
            },
        );
    }
    if factory.version.state_version.checked_add(1).is_none() {
        return Err(FactoryActivationLiveError::FactoryStateVersionExhausted { factory_id });
    }
    let transaction_id = FactoryDeliveryTransactionId::new(transaction_raw)
        .expect("fresh live owner retains a nonzero transaction id");
    let machine = FactoryDeliveryMachine::preflight(transaction_id, request)
        .map_err(FactoryActivationLiveError::DeliveryAdmission)?;
    let pair_suffix = plan_factory_pair_suffix(entities, factory_id, scientist_id)
        .map_err(FactoryActivationLiveError::PairSuffix)?;
    Ok(PreparedFactoryScientistArrival {
        factory,
        scientist,
        machine,
        pair_suffix,
    })
}

/// Apply a prepared explicit arrival to real factory/scientist allocations.
pub fn apply_prepared_factory_scientist_arrival(
    entities: &mut EntityManager,
    world_fx: &mut WorldFx,
    notifications: &mut GameplayNotifications,
    retail_tick: u32,
    prepared: PreparedFactoryScientistArrival,
) -> Result<FactoryScientistDeliveryOutcome, FactoryActivationLiveError> {
    let PreparedFactoryScientistArrival {
        mut factory,
        scientist,
        mut machine,
        pair_suffix,
    } = prepared;
    validate_factory_lease(entities, &factory)?;
    validate_scientist_lease(entities, &scientist)?;
    crate::factory_pair_suffix::validate_factory_pair_suffix_plan(entities, &pair_suffix).map_err(
        |error| match error {
            FactoryPairSuffixCommitError::FactoryChanged { factory_id } => {
                FactoryActivationLiveError::FactoryLeaseStale { factory_id }
            }
            FactoryPairSuffixCommitError::ScientistChanged { scientist_id } => {
                FactoryActivationLiveError::ScientistLeaseStale { scientist_id }
            }
        },
    )?;
    let mut events = Vec::new();

    loop {
        match machine.poll() {
            FactoryDeliveryPoll::Action(issued) => {
                validate_factory_lease(entities, &factory)?;
                validate_scientist_lease(entities, &scientist)?;
                let phase = issued.action.phase();
                match issued.action {
                    FactoryDeliveryAction::EmitOperation33 {
                        position_raw,
                        source_factory_entity_id,
                        remote_owned: false,
                        ..
                    } => {
                        world_fx.queue_cargo_transfer_particle(
                            raw_position_world(position_raw),
                            Some(source_factory_entity_id),
                        );
                        events.push(FactoryActivationLiveEvent::Operation33 {
                            position_raw,
                            source_factory_id: source_factory_entity_id,
                        });
                    }
                    FactoryDeliveryAction::QueueDeliveryHudResource { event_id, .. } => {
                        notifications.queue_factory_delivery(retail_tick as i32);
                        events.push(FactoryActivationLiveEvent::DeliveryHudResource { event_id });
                    }
                    FactoryDeliveryAction::CommitStaffing { commit, .. } => {
                        let before = commit.before.current_scientists_raw;
                        let after = commit.after.current_scientists_raw;
                        set_factory_production(entities, &factory, commit.after)?;
                        factory =
                            snapshot_factory_for_delivery(entities, factory.version.entity_id)?;
                        events
                            .push(FactoryActivationLiveEvent::StaffingCommitted { before, after });
                    }
                    FactoryDeliveryAction::QueueCapacityHudResource { event_id, .. } => {
                        notifications.queue_factory_capacity_reached(retail_tick as i32);
                        events.push(FactoryActivationLiveEvent::CapacityHudResource { event_id });
                    }
                    FactoryDeliveryAction::PublishCapacityCountdown {
                        remaining_seconds_raw,
                        ..
                    } => {
                        events.push(FactoryActivationLiveEvent::CountdownPublished {
                            remaining_seconds_raw,
                        });
                    }
                    FactoryDeliveryAction::EmitCapacityDirectText {
                        direct_text_id: FACTORY_OWNER_CAPACITY_DIRECT_TEXT_ID,
                        sub_parameter: 0,
                        ..
                    } => {
                        notifications.queue_factory_capacity_staffing(retail_tick as i32);
                        events.push(FactoryActivationLiveEvent::DirectText {
                            string_id: FACTORY_OWNER_CAPACITY_DIRECT_TEXT_ID,
                        });
                    }
                    FactoryDeliveryAction::QueueDeferredScientistDestroy {
                        scientist_entity_id,
                        ..
                    } => {
                        if !entities.queue_factory_scientist_destroy(scientist_entity_id) {
                            return Err(FactoryActivationLiveError::ScientistLeaseStale {
                                scientist_id: scientist_entity_id,
                            });
                        }
                        events.push(FactoryActivationLiveEvent::ScientistDestroyQueued {
                            scientist_id: scientist_entity_id,
                        });
                    }
                    FactoryDeliveryAction::EmitOperation33 { .. }
                    | FactoryDeliveryAction::EmitCapacityDirectText { .. }
                    | FactoryDeliveryAction::EmitRemoteScientistFeedback { .. } => {
                        return Err(FactoryActivationLiveError::UnexpectedDeliveryAction);
                    }
                }
                machine
                    .resume(
                        issued.receipt,
                        FactoryDeliveryResume::Acknowledged { phase },
                    )
                    .expect("the live bridge resumes the exact issued delivery receipt");
            }
            FactoryDeliveryPoll::Complete(completion) => {
                let factory_id = completion.pair_version.factory.entity_id;
                let scientist_id = completion.pair_version.scientist.entity_id;
                finish_factory_transaction(entities, &factory, completion.factory_runtime, 1)?;
                let visit_suffix = apply_factory_pair_suffix(entities, pair_suffix).map_err(
                    |error| match error {
                        FactoryPairSuffixCommitError::FactoryChanged { factory_id } => {
                            FactoryActivationLiveError::FactoryLeaseStale { factory_id }
                        }
                        FactoryPairSuffixCommitError::ScientistChanged { scientist_id } => {
                            FactoryActivationLiveError::ScientistLeaseStale { scientist_id }
                        }
                    },
                )?;
                return Ok(FactoryScientistDeliveryOutcome {
                    factory_id,
                    scientist_id,
                    factory_runtime: completion.factory_runtime,
                    callback_return: completion.callback_return,
                    pair_suffix: completion.pair_suffix,
                    visit_suffix,
                    events,
                });
            }
            FactoryDeliveryPoll::Awaiting(_) => {
                unreachable!("the synchronous bridge resumes every issued delivery action")
            }
            FactoryDeliveryPoll::Blocked(_) => {
                return Err(FactoryActivationLiveError::UnexpectedDeliveryAction)
            }
        }
    }
}

/// Convenience wrapper for one explicit, already-arrived scientist.
pub fn deliver_arrived_scientist(
    entities: &mut EntityManager,
    world_fx: &mut WorldFx,
    notifications: &mut GameplayNotifications,
    retail_tick: u32,
    factory_id: u32,
    scientist_id: u32,
) -> Result<FactoryScientistDeliveryOutcome, FactoryActivationLiveError> {
    let prepared = prepare_factory_scientist_arrival(entities, factory_id, scientist_id)?;
    apply_prepared_factory_scientist_arrival(
        entities,
        world_fx,
        notifications,
        retail_tick,
        prepared,
    )
}

/// Close one exact full-health phase-0/1, phase-2, cooldown, or idle-empty
/// owner frame before any allocation or presentation action escapes.
pub fn prepare_level_one_factory_production_frame(
    entities: &EntityManager,
    factory_id: u32,
    elapsed_micros: u32,
) -> Result<PreparedFactoryProductionFrame, FactoryActivationLiveError> {
    let factory = snapshot_level_one_factory(entities, factory_id)?;
    let production = factory
        .base
        .production
        .ok_or(FactoryActivationLiveError::FactoryRuntimeUnavailable { factory_id })?;
    if !level_one_entry_projection_is_authentic(factory.base, production) {
        return Err(
            FactoryActivationLiveError::FactoryPublishedProjectionMismatch {
                published_current: factory.base.current_scientists,
                published_capacity: factory.base.required_scientists,
                production_current: production.current_scientists_raw,
                production_capacity: production.scientist_capacity_raw,
            },
        );
    }
    match production.phase {
        FactoryProductionPhase::Producing => {
            if !entities.factory_pickup_constructor_ready() {
                return Err(FactoryActivationLiveError::PickupConstructorUnavailable);
            }
        }
        FactoryProductionPhase::Delivering => {}
        FactoryProductionPhase::WaitingForPickup if production.remaining_stock_raw == 0 => {}
        FactoryProductionPhase::WaitingForPickup => {
            live_factory_pickup_presence(entities, production.spawned_pickup_handle)?;
        }
        FactoryProductionPhase::Cooldown => {}
        FactoryProductionPhase::IdleEmpty => {}
    }
    if production.current_scientists_raw < 0
        || production.scientist_capacity_raw < 0
        || production.current_scientists_raw > production.scientist_capacity_raw
        || production.scientist_capacity_raw > LEVEL_ONE_FACTORY_CAPACITY
    {
        return Err(FactoryActivationLiveError::FactoryStaffingOutsideBound {
            current: production.current_scientists_raw,
            capacity: production.scientist_capacity_raw,
        });
    }
    let live_owner = factory
        .base
        .live_owner
        .expect("factory snapshot requires live owner");
    if live_owner.state_version.checked_add(1).is_none() {
        return Err(FactoryActivationLiveError::FactoryStateVersionExhausted { factory_id });
    }
    if live_owner.next_transaction_id_raw == 0
        || live_owner.next_transaction_id_raw.checked_add(2).is_none()
    {
        return Err(
            FactoryActivationLiveError::FactoryTransactionSpaceExhausted {
                factory_id,
                next_transaction_id_raw: live_owner.next_transaction_id_raw,
            },
        );
    }
    validate_level_one_live_owner_outer_state(
        factory.base.progressive_death,
        factory.current_health_raw,
        live_owner.maximum_health_raw,
    )?;
    let owner_transaction_id =
        FactoryProductionOwnerTransactionId::new(live_owner.next_transaction_id_raw)
            .expect("live owner transaction id stays nonzero");
    let child_transaction_id = FactoryProductionTransactionId::new(
        live_owner
            .next_transaction_id_raw
            .checked_add(1)
            .expect("bounded factory transaction counter cannot overflow"),
    )
    .expect("child transaction id stays nonzero");
    let request = FactoryProductionOwnerFrameRequest {
        factory: factory.version,
        position_raw: factory.position_raw,
        pickup_spawn_offset_raw: factory.pickup_spawn_offset_raw,
        current_health_raw: factory.current_health_raw,
        maximum_health_raw: live_owner.maximum_health_raw,
        progressive_death: factory.base.progressive_death,
        production,
        animation_state_raw: live_owner.animation_state_raw,
        elapsed_micros,
        world_style_raw: LEVEL_ONE_WORLD_STYLE,
        phase1_presentation_enabled: false,
        suppress_status_publication: false,
    };
    let machine = FactoryProductionOwnerMachine::preflight(
        owner_transaction_id,
        child_transaction_id,
        request,
    )
    .map_err(FactoryActivationLiveError::ProductionAdmission)?;
    Ok(PreparedFactoryProductionFrame { factory, machine })
}

/// Apply one prepared inner owner frame to the exact live factory.
pub fn apply_prepared_level_one_factory_production_frame(
    entities: &mut EntityManager,
    terrain: &TerrainGrid,
    world_fx: &mut WorldFx,
    notifications: &mut GameplayNotifications,
    retail_tick: u32,
    prepared: PreparedFactoryProductionFrame,
) -> Result<FactoryProductionFrameOutcome, FactoryActivationLiveError> {
    let PreparedFactoryProductionFrame {
        mut factory,
        mut machine,
    } = prepared;
    validate_factory_lease(entities, &factory)?;
    let mut events = Vec::new();
    let mut product = None;
    let mut converted_output = None;
    let mut materialiser = None;

    loop {
        match machine.poll() {
            FactoryProductionOwnerPoll::Action(issued) => {
                validate_factory_lease(entities, &factory)?;
                persist_owner_prefix(entities, &factory, issued.before_action)?;
                factory = snapshot_level_one_factory(entities, factory.version.entity_id)?;
                let phase = issued.action.phase();
                let resume = match issued.action {
                    FactoryProductionOwnerAction::Production { action, .. } => {
                        apply_production_action(
                            entities,
                            terrain,
                            world_fx,
                            notifications,
                            retail_tick,
                            &mut factory,
                            &mut product,
                            &mut converted_output,
                            &mut materialiser,
                            &mut events,
                            phase,
                            action,
                        )?
                    }
                    FactoryProductionOwnerAction::ApplyAnimationTransition {
                        transition, ..
                    } => {
                        apply_animation_transition(entities, &factory, transition)?;
                        factory = snapshot_level_one_factory(entities, factory.version.entity_id)?;
                        events.push(FactoryActivationLiveEvent::AnimationTransition(transition));
                        FactoryProductionOwnerResume::Acknowledged { phase }
                    }
                    FactoryProductionOwnerAction::PublishStatus { status, .. } => {
                        apply_status_publication(entities, &factory, status)?;
                        factory = snapshot_level_one_factory(entities, factory.version.entity_id)?;
                        events.push(FactoryActivationLiveEvent::StatusPublished(status));
                        FactoryProductionOwnerResume::Acknowledged { phase }
                    }
                    FactoryProductionOwnerAction::PublishCountdownSeconds {
                        remaining_seconds_raw,
                        ..
                    } => {
                        events.push(FactoryActivationLiveEvent::CountdownPublished {
                            remaining_seconds_raw,
                        });
                        FactoryProductionOwnerResume::Acknowledged { phase }
                    }
                    FactoryProductionOwnerAction::EmitDirectText {
                        direct_text_id: FACTORY_OWNER_CAPACITY_DIRECT_TEXT_ID,
                        sub_parameter: 0,
                        ..
                    } => {
                        notifications.queue_factory_capacity_staffing(retail_tick as i32);
                        events.push(FactoryActivationLiveEvent::DirectText {
                            string_id: FACTORY_OWNER_CAPACITY_DIRECT_TEXT_ID,
                        });
                        FactoryProductionOwnerResume::Acknowledged { phase }
                    }
                    FactoryProductionOwnerAction::QueueHudResource {
                        resource_id: FACTORY_OWNER_CAPACITY_HUD_RESOURCE_ID,
                        ..
                    } => {
                        notifications.queue_factory_capacity_reached(retail_tick as i32);
                        events.push(FactoryActivationLiveEvent::CapacityHudResource {
                            event_id: FACTORY_OWNER_CAPACITY_HUD_RESOURCE_ID as u8,
                        });
                        FactoryProductionOwnerResume::Acknowledged { phase }
                    }
                    _ => {
                        return Err(FactoryActivationLiveError::UnexpectedProductionAction(
                            phase,
                        ))
                    }
                };
                machine
                    .resume(issued.receipt, resume)
                    .expect("the live bridge resumes the exact issued owner receipt");
            }
            FactoryProductionOwnerPoll::Complete(completion) => {
                finish_owner_frame(entities, &factory, completion)?;
                return Ok(FactoryProductionFrameOutcome {
                    factory_id: completion.factory.entity_id,
                    production: completion.production,
                    product,
                    converted_output,
                    materialiser,
                    status_published: completion.status_published,
                    dirty_flag_written: completion.dirty_flag_written,
                    events,
                });
            }
            FactoryProductionOwnerPoll::Blocked(block) => {
                return Err(FactoryActivationLiveError::ProductionBlocked {
                    phase: block.phase,
                    reason: block.reason,
                })
            }
            FactoryProductionOwnerPoll::Awaiting(_) => {
                unreachable!("the synchronous bridge resumes every issued owner action")
            }
        }
    }
}

/// Convenience wrapper for one exact bounded owner frame.
///
/// The frame that completes delivery may enter phase 2. Finite-stock phase-2
/// frames execute both finite worker-ejection transactions and settle into
/// phase 4. A positive-stock frame queries the live spawned pickup through
/// `FUN_0043a580` and may enter cooldown. Damaged-repair and cached-health
/// frames use the closed outer owner.
/// Retail's nonzero-multiplayer phase-1 presentation callback is not admitted.
/// Keep extra remaining stock on the exact Level-1 live factory.
///
/// Authored cooldown stays negative. This only writes the runtime stock
/// counter so a later owner frame can take the already-closed pickup-presence
/// branch.
pub fn retain_level_one_factory_remaining_stock(
    entities: &mut EntityManager,
    factory_id: u32,
    remaining_stock_raw: i32,
) -> Result<(), FactoryActivationLiveError> {
    if remaining_stock_raw <= 0 {
        return Err(FactoryActivationLiveError::RemainingStockRejected {
            remaining_stock_raw,
        });
    }
    let _ = snapshot_level_one_factory(entities, factory_id)?;
    let entity = entities
        .factory_activation_entity_mut(factory_id)
        .ok_or(FactoryActivationLiveError::FactoryUnavailable { factory_id })?;
    match &mut entity.base_factory_runtime {
        RetailRuntimeValue::Known(Some(state)) => {
            let production = state
                .production
                .as_mut()
                .ok_or(FactoryActivationLiveError::FactoryRuntimeUnavailable { factory_id })?;
            production.remaining_stock_raw = remaining_stock_raw;
            Ok(())
        }
        _ => Err(FactoryActivationLiveError::FactoryRuntimeUnavailable { factory_id }),
    }
}

/// Remove the factory's spawned pickup from the live list.
///
/// This is the completed deferred-destroy splice that makes
/// `FUN_0043a580(handle)` miss.
pub fn splice_level_one_factory_spawned_pickup(
    entities: &mut EntityManager,
    factory_id: u32,
) -> Result<u32, FactoryActivationLiveError> {
    let factory = snapshot_level_one_factory(entities, factory_id)?;
    let production = factory
        .base
        .production
        .ok_or(FactoryActivationLiveError::FactoryRuntimeUnavailable { factory_id })?;
    let pickup_id = production.spawned_pickup_handle;
    if pickup_id == 0 || !entities.splice_live_entity_id(pickup_id) {
        return Err(FactoryActivationLiveError::PickupUnavailable { pickup_id });
    }
    Ok(pickup_id)
}

/// Retail `FUN_0043a580` + state-word gates for phase-2 pickup presence.
pub fn live_factory_pickup_presence(
    entities: &EntityManager,
    pickup_handle: u32,
) -> Result<FactoryPickupPresence, FactoryActivationLiveError> {
    if pickup_handle == 0 {
        return Ok(FactoryPickupPresence::Gone);
    }
    let flags = entities
        .iter_all()
        .find(|entity| entity.id == pickup_handle)
        .map(|entity| entity.collision.state_flags_at_0x08);
    classify_factory_pickup_presence(flags).map_err(|()| {
        FactoryActivationLiveError::PickupCallbackStateUnavailable {
            pickup_id: pickup_handle,
        }
    })
}

fn classify_factory_pickup_presence(
    flags: Option<RetailStateWord>,
) -> Result<FactoryPickupPresence, ()> {
    let Some(flags) = flags else {
        return Ok(FactoryPickupPresence::Gone);
    };
    match flags.masked(DYING_STATE_BIT) {
        RetailRuntimeValue::Unresolved => Err(()),
        RetailRuntimeValue::Known(bits) if bits != 0 => Ok(FactoryPickupPresence::Gone),
        RetailRuntimeValue::Known(0) => match flags.masked(u32::MAX) {
            RetailRuntimeValue::Known(0) => Ok(FactoryPickupPresence::Gone),
            RetailRuntimeValue::Known(_) => Ok(FactoryPickupPresence::Active),
            RetailRuntimeValue::Unresolved => Err(()),
        },
        RetailRuntimeValue::Known(_) => unreachable!("dying mask is one bit"),
    }
}

pub fn tick_level_one_factory_owner(
    entities: &mut EntityManager,
    terrain: &TerrainGrid,
    world_fx: &mut WorldFx,
    notifications: &mut GameplayNotifications,
    retail_tick: u32,
    factory_id: u32,
    elapsed_micros: u32,
) -> Result<FactoryProductionFrameOutcome, FactoryActivationLiveError> {
    let prepared =
        prepare_level_one_factory_production_frame(entities, factory_id, elapsed_micros)?;
    apply_prepared_level_one_factory_production_frame(
        entities,
        terrain,
        world_fx,
        notifications,
        retail_tick,
        prepared,
    )
}

/// Drive one converted-output spawn through the live production executor.
pub fn spawn_factory_converted_output(
    entities: &mut EntityManager,
    terrain: &TerrainGrid,
    world_fx: &mut WorldFx,
    retail_tick: u32,
    factory_id: u32,
    request: FactoryEntitySpawnRequest,
) -> Result<FactoryProductionOwnerResume, FactoryActivationLiveError> {
    let mut factory = snapshot_level_one_factory(entities, factory_id)?;
    let mut product = None;
    let mut converted_output = None;
    let mut materialiser = None;
    let mut events = Vec::new();
    let mut notifications = GameplayNotifications::new();
    apply_production_action(
        entities,
        terrain,
        world_fx,
        &mut notifications,
        retail_tick,
        &mut factory,
        &mut product,
        &mut converted_output,
        &mut materialiser,
        &mut events,
        FactoryProductionOwnerActionPhase::Production(
            FactoryProductionActionPhase::SpawnConvertedOutput,
        ),
        FactoryProductionAction::SpawnEntity {
            phase: FactoryProductionActionPhase::SpawnConvertedOutput,
            role: FactorySpawnRole::ConvertedOutput,
            request,
        },
    )
}

fn apply_production_action(
    entities: &mut EntityManager,
    terrain: &TerrainGrid,
    world_fx: &mut WorldFx,
    notifications: &mut GameplayNotifications,
    retail_tick: u32,
    factory: &mut FactoryLease,
    product: &mut Option<FactorySpawnedEntity>,
    converted_output: &mut Option<FactorySpawnedEntity>,
    materialiser: &mut Option<FactorySpawnedEntity>,
    events: &mut Vec<FactoryActivationLiveEvent>,
    owner_phase: FactoryProductionOwnerActionPhase,
    action: FactoryProductionAction,
) -> Result<FactoryProductionOwnerResume, FactoryActivationLiveError> {
    let child_phase = action.phase();
    match action {
        FactoryProductionAction::SpawnEntity {
            phase: FactoryProductionActionPhase::SpawnPickup,
            role: FactorySpawnRole::Pickup,
            request,
        } if request.entity_type == FACTORY_PICKUP_ENTITY_TYPE => {
            let result = entities
                .append_factory_pickup(factory.version, request, world_fx)
                .map(FactoryEntitySpawnResult::Spawned)
                .unwrap_or(FactoryEntitySpawnResult::Failed);
            if let FactoryEntitySpawnResult::Spawned(spawned) = result {
                *product = Some(spawned);
                events.push(FactoryActivationLiveEvent::ProductSpawned { product: spawned });
            }
            Ok(FactoryProductionOwnerResume::SpawnCompleted {
                phase: owner_phase,
                result,
            })
        }
        FactoryProductionAction::LinkOwner {
            phase: FactoryProductionActionPhase::LinkPickupOwner,
            spawned,
            owner_factory,
        } if owner_factory == factory.version => {
            if entities.link_factory_pickup_owner(spawned, owner_factory) {
                events.push(FactoryActivationLiveEvent::ProductOwnerLinked {
                    product_id: spawned.entity_id.get(),
                    factory_id: owner_factory.entity_id,
                });
            }
            // FUN_00419010 treats a fresh-child lookup miss as an acknowledged
            // no-op and still commits the following product prefix.
            Ok(FactoryProductionOwnerResume::Acknowledged { phase: owner_phase })
        }
        FactoryProductionAction::QueryPickupPresence {
            phase: FactoryProductionActionPhase::QueryPickupPresence,
            pickup_handle,
        } => {
            let presence = live_factory_pickup_presence(entities, pickup_handle)?;
            events.push(FactoryActivationLiveEvent::PickupPresence {
                pickup_id: pickup_handle,
                presence,
            });
            Ok(FactoryProductionOwnerResume::PickupPresence {
                phase: owner_phase,
                presence,
            })
        }
        FactoryProductionAction::QueueHudResource {
            phase: FactoryProductionActionPhase::QueueConversionHudResource,
            resource_id: FACTORY_OUTPUT_CONVERSION_HUD_RESOURCE_ID,
        } => {
            notifications.queue_factory_output_conversion(retail_tick as i32);
            events.push(FactoryActivationLiveEvent::ConversionHudResource {
                resource_id: FACTORY_OUTPUT_CONVERSION_HUD_RESOURCE_ID,
            });
            Ok(FactoryProductionOwnerResume::Acknowledged { phase: owner_phase })
        }
        FactoryProductionAction::SpawnEntity {
            phase: FactoryProductionActionPhase::SpawnConvertedOutput,
            role: FactorySpawnRole::ConvertedOutput,
            request,
        } if NativeWorkerProfile::from_entity_type(request.entity_type).is_some()
            || request.entity_type == 7 =>
        {
            let result = entities
                .append_factory_converted_output(
                    factory.version,
                    request,
                    retail_tick,
                    terrain,
                    world_fx,
                )
                .map(FactoryEntitySpawnResult::Spawned)
                .unwrap_or(FactoryEntitySpawnResult::Failed);
            if let FactoryEntitySpawnResult::Spawned(spawned) = result {
                *converted_output = Some(spawned);
                events.push(FactoryActivationLiveEvent::ConvertedOutputSpawned { output: spawned });
                notifications
                    .drain_attract_attention_receipts(entities, retail_tick as i32)
                    .map_err(FactoryActivationLiveError::AttractAttentionNotification)?;
            }
            Ok(FactoryProductionOwnerResume::SpawnCompleted {
                phase: owner_phase,
                result,
            })
        }
        FactoryProductionAction::SpawnEntity {
            phase: FactoryProductionActionPhase::SpawnConvertedOutput,
            role: FactorySpawnRole::ConvertedOutput,
            request,
        } => Err(FactoryActivationLiveError::UnsupportedConvertedOutput {
            entity_type: request.entity_type,
        }),
        FactoryProductionAction::LinkOwner {
            phase: FactoryProductionActionPhase::LinkConvertedOutputOwner,
            spawned,
            owner_factory,
        } if owner_factory == factory.version => {
            if entities.link_factory_converted_output_owner(spawned, owner_factory) {
                events.push(FactoryActivationLiveEvent::ConvertedOutputOwnerLinked {
                    output_id: spawned.entity_id.get(),
                    factory_id: owner_factory.entity_id,
                });
            }
            Ok(FactoryProductionOwnerResume::Acknowledged { phase: owner_phase })
        }
        FactoryProductionAction::SpawnEntity {
            phase: FactoryProductionActionPhase::SpawnMaterialiser,
            role: FactorySpawnRole::Materialiser,
            request,
        } if request.entity_type == FACTORY_MATERIALISER_ENTITY_TYPE => {
            let result = entities
                .append_factory_output_materialiser(factory.version, request, terrain, world_fx)
                .map(FactoryEntitySpawnResult::Spawned)
                .unwrap_or(FactoryEntitySpawnResult::Failed);
            if let FactoryEntitySpawnResult::Spawned(spawned) = result {
                *materialiser = Some(spawned);
                events.push(FactoryActivationLiveEvent::MaterialiserSpawned {
                    materialiser: spawned,
                });
            }
            Ok(FactoryProductionOwnerResume::SpawnCompleted {
                phase: owner_phase,
                result,
            })
        }
        FactoryProductionAction::LinkMaterialiserOutput {
            phase: FactoryProductionActionPhase::LinkMaterialiserOutput,
            materialiser: spawned_materialiser,
            output,
        } => {
            if entities.link_factory_materialiser_output(spawned_materialiser, output, world_fx) {
                events.push(FactoryActivationLiveEvent::MaterialiserOutputLinked {
                    materialiser_id: spawned_materialiser.entity_id.get(),
                    output_id: output.entity_id.get(),
                });
            }
            // Retail disposes FUN_00408F00's return value and always advances
            // to positional sound, including an error-object result.
            Ok(FactoryProductionOwnerResume::Acknowledged { phase: owner_phase })
        }
        FactoryProductionAction::PlayConversionPositionalSound {
            phase: FactoryProductionActionPhase::PlayConversionPositionalSound,
            request,
        } if request.sound_id == FACTORY_OUTPUT_CONVERSION_SOUND_ID
            && request.gain_raw_16_16 == 0x1_0000
            && request.rate_raw_16_16 == 0x1_0000 =>
        {
            world_fx.queue_fixed_positional_sound_raw_at_rate(
                request.sound_id,
                request.position_raw,
                request.rate_raw_16_16 as u32,
            );
            events.push(FactoryActivationLiveEvent::ConversionPositionalSound {
                sound_id: request.sound_id,
                position_raw: request.position_raw,
            });
            Ok(FactoryProductionOwnerResume::Acknowledged { phase: owner_phase })
        }
        FactoryProductionAction::EmitDirectText {
            direct_text_id,
            sub_parameter: 0,
            ..
        } if direct_text_id == FACTORY_HIGH_THRESHOLD_DIRECT_TEXT_ID => {
            notifications.queue_factory_product_ready(retail_tick as i32);
            events.push(FactoryActivationLiveEvent::DirectText {
                string_id: direct_text_id,
            });
            Ok(FactoryProductionOwnerResume::Acknowledged { phase: owner_phase })
        }
        FactoryProductionAction::ClearFactoryLifetime {
            factory: action_factory,
            lifetime_offset: FACTORY_ENTITY_LIFETIME_OFFSET,
            value_raw: 0,
            ..
        } if action_factory == factory.version => {
            set_factory_lifetime(entities, factory, 0)?;
            *factory = snapshot_level_one_factory(entities, factory.version.entity_id)?;
            events.push(FactoryActivationLiveEvent::LifetimeCleared);
            Ok(FactoryProductionOwnerResume::Acknowledged { phase: owner_phase })
        }
        FactoryProductionAction::ApplyAnimationTransition {
            factory: action_factory,
            transition,
            ..
        } if action_factory == factory.version => {
            apply_animation_transition(entities, factory, transition)?;
            *factory = snapshot_level_one_factory(entities, factory.version.entity_id)?;
            events.push(FactoryActivationLiveEvent::AnimationTransition(transition));
            Ok(FactoryProductionOwnerResume::Acknowledged { phase: owner_phase })
        }
        FactoryProductionAction::PublishStatus {
            factory: action_factory,
            status,
            ..
        } if action_factory == factory.version => {
            apply_status_publication(entities, factory, status)?;
            *factory = snapshot_level_one_factory(entities, factory.version.entity_id)?;
            events.push(FactoryActivationLiveEvent::StatusPublished(status));
            Ok(FactoryProductionOwnerResume::Acknowledged { phase: owner_phase })
        }
        FactoryProductionAction::MarkEntityDirty {
            factory: action_factory,
            flag: FACTORY_ENTITY_DIRTY_FLAG,
            ..
        } if action_factory == factory.version => {
            mark_factory_dirty(entities, factory, FACTORY_ENTITY_DIRTY_FLAG)?;
            *factory = snapshot_level_one_factory(entities, factory.version.entity_id)?;
            events.push(FactoryActivationLiveEvent::EntityMarkedDirty {
                flag: FACTORY_ENTITY_DIRTY_FLAG,
            });
            Ok(FactoryProductionOwnerResume::Acknowledged { phase: owner_phase })
        }
        _ => Err(FactoryActivationLiveError::UnexpectedProductionAction(
            FactoryProductionOwnerActionPhase::Production(child_phase),
        )),
    }
}

fn snapshot_level_one_factory(
    entities: &EntityManager,
    factory_id: u32,
) -> Result<FactoryLease, FactoryActivationLiveError> {
    snapshot_factory(
        entities,
        factory_id,
        FactoryConstructionPolicy::RetainedLevelOne,
    )
}

fn snapshot_factory_for_delivery(
    entities: &EntityManager,
    factory_id: u32,
) -> Result<FactoryLease, FactoryActivationLiveError> {
    let construction = if entities
        .iter_all()
        .any(|entity| entity.id == factory_id && entity.intro2_type66_runtime.is_some())
    {
        FactoryConstructionPolicy::NativeType66
    } else {
        FactoryConstructionPolicy::RetainedLevelOne
    };
    snapshot_factory(entities, factory_id, construction)
}

/// Pair queries and suffixes require the same actual allocation as intake.
pub(crate) fn validate_factory_delivery_owner(
    entities: &EntityManager,
    factory_id: u32,
) -> Result<(), FactoryActivationLiveError> {
    snapshot_factory_for_delivery(entities, factory_id).map(|_| ())
}

fn snapshot_factory(
    entities: &EntityManager,
    factory_id: u32,
    construction: FactoryConstructionPolicy,
) -> Result<FactoryLease, FactoryActivationLiveError> {
    let entity = entities
        .iter_all()
        .find(|entity| entity.id == factory_id)
        .ok_or(FactoryActivationLiveError::FactoryUnavailable { factory_id })?;
    let identity_matches = match construction {
        FactoryConstructionPolicy::RetainedLevelOne => {
            entity.active
                && entity.authored_spawn_index == Some(LEVEL_ONE_FACTORY_SPAWN_INDEX)
                && entity.entity_type == LEVEL_ONE_FACTORY_ENTITY_TYPE
                && entity.model_index == Some(LEVEL_ONE_FACTORY_MODEL_ID)
                && entity.position_raw() == LEVEL_ONE_FACTORY_POSITION_RAW
                && entity.heading_raw() == 0
                && entity.intro2_type66_runtime.is_none()
        }
        FactoryConstructionPolicy::NativeType66 => {
            crate::intro2_type66::type66_manager_allocation_authenticates(entities, factory_id)
                && crate::intro2_type66::Intro2Type66Owner::adopt(entities, factory_id).is_ok()
        }
    };
    if !identity_matches {
        return Err(FactoryActivationLiveError::FactoryIdentityMismatch { factory_id });
    }
    let style = match entity.current_behavior_context {
        RetailRuntimeValue::Known(Some(context)) => context.active_style().audited(),
        RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => None,
    }
    .ok_or(FactoryActivationLiveError::FactoryBehaviorUnavailable { factory_id })?;
    if style.class_id != LEVEL_ONE_FACTORY_BEHAVIOR_CLASS_ID
        || style.pair_contact_callback_policy() != PairContactCallbackPolicy::LifterDelivery
    {
        return Err(FactoryActivationLiveError::FactoryBehaviorUnavailable { factory_id });
    }
    let base = match entity.base_factory_runtime {
        RetailRuntimeValue::Known(Some(state)) => state,
        RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => {
            return Err(FactoryActivationLiveError::FactoryRuntimeUnavailable { factory_id })
        }
    };
    let production = base
        .production
        .ok_or(FactoryActivationLiveError::FactoryRuntimeUnavailable { factory_id })?;
    let live_owner = base
        .live_owner
        .ok_or(FactoryActivationLiveError::FactoryLiveOwnerUnavailable { factory_id })?;
    let template_matches = match construction {
        FactoryConstructionPolicy::RetainedLevelOne => {
            is_exact_level_one_template(base, production, live_owner.maximum_health_raw)
        }
        FactoryConstructionPolicy::NativeType66 => {
            let runtime = entity
                .intro2_type66_runtime
                .expect("native allocation authenticated");
            let template = FactoryProductionRuntime::from_retail_template(
                runtime.config(),
                live_owner.maximum_health_raw,
            );
            base.status_descriptor == crate::intro2_type66::STATUS_DESCRIPTOR
                && live_owner.maximum_health_raw == crate::intro2_type66::INITIAL_HEALTH_RAW
                && production.output_payload_packed == template.output_payload_packed
                && production.production_threshold_micros_raw
                    == template.production_threshold_micros_raw
                && production.delivery_duration_micros_raw == template.delivery_duration_micros_raw
                && production.cooldown_duration_micros_raw == template.cooldown_duration_micros_raw
                && production.understaffed_limit_micros_raw
                    == template.understaffed_limit_micros_raw
                && (0..=template.scientist_capacity_raw)
                    .contains(&production.scientist_capacity_raw)
        }
    };
    if !template_matches {
        return Err(FactoryActivationLiveError::FactoryIdentityMismatch { factory_id });
    }
    let current_health_raw = match entity.collision.health_raw {
        RetailRuntimeValue::Known(value) => value,
        RetailRuntimeValue::Unresolved => {
            return Err(FactoryActivationLiveError::FactoryRuntimeUnavailable { factory_id })
        }
    };
    let callback_state = entity.collision.state_flags_at_0x08;
    match callback_state.masked(FACTORY_CALLBACK_STATE_MASK) {
        RetailRuntimeValue::Known(0) if callback_state.known_value_bits() != 0 => {}
        RetailRuntimeValue::Known(_) => {
            return Err(FactoryActivationLiveError::FactoryCallbackIneligible { factory_id })
        }
        RetailRuntimeValue::Unresolved => {
            return Err(FactoryActivationLiveError::FactoryCallbackStateUnavailable { factory_id })
        }
    }
    Ok(FactoryLease {
        construction,
        allocation: match construction {
            FactoryConstructionPolicy::NativeType66 => entity
                .intro2_type66_runtime
                .expect("native allocation authenticated")
                .allocation(),
            FactoryConstructionPolicy::RetainedLevelOne => {
                entities
                    .main_base_abort_actor_observation(factory_id)
                    .ok_or(FactoryActivationLiveError::FactoryIdentityMismatch { factory_id })?
                    .lease
            }
        },
        version: FactoryProductionEntityVersion {
            entity_id: factory_id,
            allocation_identity: live_owner.allocation_identity,
            state_version: live_owner.state_version,
        },
        base,
        current_health_raw,
        callback_state,
        position_raw: entity.position_raw(),
        pickup_spawn_offset_raw: status_pickup_spawn_offset(base),
    })
}

fn snapshot_arrived_scientist(
    entities: &EntityManager,
    factory_id: u32,
    scientist_id: u32,
) -> Result<ScientistLease, FactoryActivationLiveError> {
    if entities
        .pending_factory_scientist_destroy_ids()
        .contains(&scientist_id)
    {
        return Err(FactoryActivationLiveError::ScientistDestroyAlreadyPending { scientist_id });
    }
    let entity = entities
        .iter_all()
        .find(|entity| entity.id == scientist_id)
        .ok_or(FactoryActivationLiveError::ScientistUnavailable { scientist_id })?;
    if !entity.active
        || entity.capability_flags & SCIENTIST_FACTORY_CAPABILITY_BIT == 0
        || !crate::factory_pair_suffix::scientist_allocation_authenticates(entities, scientist_id)
    {
        return Err(FactoryActivationLiveError::ScientistIdentityMismatch { scientist_id });
    }
    let target_factory_id = match entity.actor_task_state(ActorTaskSlot::Primary) {
        Some(ActorTaskRuntime::GoToJob(state)) => state.target_id(),
        _ => None,
    }
    .ok_or(FactoryActivationLiveError::ScientistTaskMismatch {
        scientist_id,
        factory_id,
    })?;
    if target_factory_id != factory_id {
        return Err(FactoryActivationLiveError::ScientistTaskMismatch {
            scientist_id,
            factory_id,
        });
    }
    let state = entity.collision.state_flags_at_0x08;
    let state_flags_raw = match state.masked(FACTORY_CALLBACK_STATE_MASK) {
        RetailRuntimeValue::Known(0) if state.known_value_bits() != 0 => state.known_value_bits(),
        RetailRuntimeValue::Known(_) => {
            return Err(FactoryActivationLiveError::ScientistCallbackIneligible { scientist_id })
        }
        RetailRuntimeValue::Unresolved => {
            return Err(
                FactoryActivationLiveError::ScientistCallbackStateUnavailable { scientist_id },
            )
        }
    };
    let position_raw = entity.position_raw();
    let allocation_identity = entities
        .main_base_abort_actor_observation(scientist_id)
        .ok_or(FactoryActivationLiveError::ScientistUnavailable { scientist_id })?
        .lease
        .allocation_identity;
    let state_version = scientist_state_version(
        scientist_id,
        position_raw,
        state_flags_raw,
        entity.capability_flags,
        target_factory_id,
    );
    Ok(ScientistLease {
        entity_id: scientist_id,
        entity_type: entity.entity_type,
        primary_task: entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .ok_or(FactoryActivationLiveError::ScientistTaskMismatch {
                scientist_id,
                factory_id,
            })?,
        behavior: entity.current_behavior_context,
        native_runtime: crate::factory_pair_suffix::FactoryScientistRuntime::from_entity(entity)
            .expect("scientist allocation authenticated"),
        allocation_identity,
        state_version,
        position_raw,
        state_flags_raw,
        capability_flags_raw: entity.capability_flags,
        target_factory_id,
    })
}

fn scientist_state_version(
    entity_id: u32,
    position_raw: [i16; 3],
    state_flags_raw: u32,
    capability_flags_raw: u32,
    target_factory_id: u32,
) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for word in [
        entity_id,
        position_raw[0] as u16 as u32,
        position_raw[1] as u16 as u32,
        position_raw[2] as u16 as u32,
        state_flags_raw,
        capability_flags_raw,
        target_factory_id,
    ] {
        hash ^= u64::from(word);
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash.max(1)
}

fn validate_scientist_lease(
    entities: &EntityManager,
    expected: &ScientistLease,
) -> Result<(), FactoryActivationLiveError> {
    match snapshot_arrived_scientist(entities, expected.target_factory_id, expected.entity_id) {
        Ok(actual) if actual == *expected => Ok(()),
        _ => Err(FactoryActivationLiveError::ScientistLeaseStale {
            scientist_id: expected.entity_id,
        }),
    }
}

fn validate_factory_lease(
    entities: &EntityManager,
    expected: &FactoryLease,
) -> Result<(), FactoryActivationLiveError> {
    match snapshot_factory(entities, expected.version.entity_id, expected.construction) {
        Ok(actual) if actual == *expected => Ok(()),
        _ => Err(FactoryActivationLiveError::FactoryLeaseStale {
            factory_id: expected.version.entity_id,
        }),
    }
}

fn is_exact_level_one_template(
    base: BaseFactoryRuntimeState,
    production: FactoryProductionRuntime,
    maximum_health_raw: i32,
) -> bool {
    base.required_scientists <= LEVEL_ONE_FACTORY_CAPACITY as u16
        && base.current_scientists <= base.required_scientists
        && maximum_health_raw == LEVEL_ONE_FACTORY_MAX_HEALTH_RAW
        && production.output_payload_packed == LEVEL_ONE_FACTORY_OUTPUT_PAYLOAD
        && (0..=LEVEL_ONE_FACTORY_CAPACITY).contains(&production.scientist_capacity_raw)
        && production.production_threshold_micros_raw == LEVEL_ONE_FACTORY_PRODUCTION_THRESHOLD_US
        && production.delivery_duration_micros_raw == LEVEL_ONE_FACTORY_DELIVERY_DURATION_US
        && production.cooldown_duration_micros_raw == LEVEL_ONE_FACTORY_COOLDOWN_US
        && production.understaffed_limit_micros_raw == LEVEL_ONE_FACTORY_UNDERSTAFFED_LIMIT_US
        && status_pickup_spawn_offset(base) == [0; 3]
}

fn status_pickup_spawn_offset(base: BaseFactoryRuntimeState) -> [i16; 3] {
    let tail = base.status_descriptor.raw_tail;
    [
        i16::from_le_bytes([tail[4], tail[5]]),
        i16::from_le_bytes([tail[6], tail[7]]),
        i16::from_le_bytes([tail[8], tail[9]]),
    ]
}

/// At a public frame boundary the published capacity must match production.
/// Current staffing may lead the last published pole because multiple
/// scientist-delivery callbacks can commit before the next owner frame
/// publishes status. It may not trail the pole or exceed capacity. Internal
/// action snapshots deliberately use the looser template check because
/// production prefixes lead status within one frame.
fn level_one_entry_projection_is_authentic(
    base: BaseFactoryRuntimeState,
    production: FactoryProductionRuntime,
) -> bool {
    production.scientist_capacity_raw == i32::from(base.required_scientists)
        && i32::from(base.current_scientists) <= production.current_scientists_raw
        && production.current_scientists_raw <= production.scientist_capacity_raw
}

fn validate_level_one_live_owner_outer_state(
    progressive_death: crate::base_factory_progression::ProgressiveDeathState,
    current_health_raw: i32,
    maximum_health_raw: i32,
) -> Result<(), FactoryActivationLiveError> {
    if progressive_death.elapsed_micros_raw != 0 {
        return Err(FactoryActivationLiveError::FactoryProgressiveDeathActive {
            elapsed_micros_raw: progressive_death.elapsed_micros_raw,
        });
    }
    if current_health_raw <= 0 || current_health_raw > maximum_health_raw {
        return Err(FactoryActivationLiveError::FactoryNotAtFullHealth {
            current: current_health_raw,
            maximum: maximum_health_raw,
        });
    }
    Ok(())
}

fn set_factory_production(
    entities: &mut EntityManager,
    expected: &FactoryLease,
    production: FactoryProductionRuntime,
) -> Result<(), FactoryActivationLiveError> {
    let state = factory_state_mut(entities, expected)?;
    state.production = Some(production);
    Ok(())
}

fn persist_owner_prefix(
    entities: &mut EntityManager,
    expected: &FactoryLease,
    before: crate::factory_production_owner::FactoryProductionOwnerPreActionState,
) -> Result<(), FactoryActivationLiveError> {
    let entity = factory_entity_mut(entities, expected)?;
    entity
        .persist_factory_production_owner_prefix(expected.version, before)
        .then_some(())
        .ok_or(FactoryActivationLiveError::FactoryLeaseStale {
            factory_id: expected.version.entity_id,
        })
}

fn apply_animation_transition(
    entities: &mut EntityManager,
    expected: &FactoryLease,
    transition: FactoryAnimationTransition,
) -> Result<(), FactoryActivationLiveError> {
    let entity = factory_entity_mut(entities, expected)?;
    entity
        .apply_factory_production_owner_animation(expected.version, transition)
        .then_some(())
        .ok_or(FactoryActivationLiveError::FactoryLeaseStale {
            factory_id: expected.version.entity_id,
        })
}

fn apply_status_publication(
    entities: &mut EntityManager,
    expected: &FactoryLease,
    status: FactoryStatusPublication,
) -> Result<(), FactoryActivationLiveError> {
    let entity = factory_entity_mut(entities, expected)?;
    entity
        .apply_factory_production_owner_status(expected.version, status)
        .then_some(())
        .ok_or(FactoryActivationLiveError::FactoryLeaseStale {
            factory_id: expected.version.entity_id,
        })
}

fn set_factory_lifetime(
    entities: &mut EntityManager,
    expected: &FactoryLease,
    value_raw: u32,
) -> Result<(), FactoryActivationLiveError> {
    let state = factory_state_mut(entities, expected)?;
    state
        .live_owner
        .as_mut()
        .expect("validated live owner")
        .lifetime_at_0x74_raw = value_raw;
    Ok(())
}

fn mark_factory_dirty(
    entities: &mut EntityManager,
    expected: &FactoryLease,
    flag: u32,
) -> Result<(), FactoryActivationLiveError> {
    let entity = factory_entity_mut(entities, expected)?;
    entity.collision.state_flags_at_0x08.overwrite(flag, flag);
    Ok(())
}

fn finish_factory_transaction(
    entities: &mut EntityManager,
    expected: &FactoryLease,
    production: FactoryProductionRuntime,
    consumed_transaction_ids: u64,
) -> Result<(), FactoryActivationLiveError> {
    let state = factory_state_mut(entities, expected)?;
    state.production = Some(production);
    let live_owner = state.live_owner.as_mut().expect("validated live owner");
    live_owner.state_version = live_owner
        .state_version
        .checked_add(1)
        .expect("bounded live factory version cannot overflow");
    live_owner.next_transaction_id_raw = live_owner
        .next_transaction_id_raw
        .checked_add(consumed_transaction_ids)
        .expect("bounded live factory transaction counter cannot overflow");
    Ok(())
}

fn finish_owner_frame(
    entities: &mut EntityManager,
    expected: &FactoryLease,
    completion: crate::factory_production_owner::FactoryProductionOwnerCompletion,
) -> Result<(), FactoryActivationLiveError> {
    let entity = factory_entity_mut(entities, expected)?;
    entity
        .finish_factory_production_owner_frame(expected.version, completion)
        .then_some(())
        .ok_or(FactoryActivationLiveError::FactoryLeaseStale {
            factory_id: expected.version.entity_id,
        })
}

fn factory_state_mut<'a>(
    entities: &'a mut EntityManager,
    expected: &FactoryLease,
) -> Result<&'a mut BaseFactoryRuntimeState, FactoryActivationLiveError> {
    let entity = factory_entity_mut(entities, expected)?;
    match &mut entity.base_factory_runtime {
        RetailRuntimeValue::Known(Some(state)) => Ok(state),
        _ => Err(FactoryActivationLiveError::FactoryLeaseStale {
            factory_id: expected.version.entity_id,
        }),
    }
}

fn factory_entity_mut<'a>(
    entities: &'a mut EntityManager,
    expected: &FactoryLease,
) -> Result<&'a mut Entity, FactoryActivationLiveError> {
    let entity = entities
        .factory_activation_entity_mut(expected.version.entity_id)
        .ok_or(FactoryActivationLiveError::FactoryLeaseStale {
            factory_id: expected.version.entity_id,
        })?;
    if entity.base_factory_runtime != RetailRuntimeValue::Known(Some(expected.base))
        || entity.collision.health_raw != RetailRuntimeValue::Known(expected.current_health_raw)
        || entity.collision.state_flags_at_0x08 != expected.callback_state
    {
        return Err(FactoryActivationLiveError::FactoryLeaseStale {
            factory_id: expected.version.entity_id,
        });
    }
    Ok(entity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base_factory_progression::ProgressiveDeathState;

    #[test]
    fn scientist_version_changes_with_every_authenticated_field() {
        let baseline = scientist_state_version(1, [2, 3, 4], 5, 6, 7);
        for changed in [
            scientist_state_version(2, [2, 3, 4], 5, 6, 7),
            scientist_state_version(1, [3, 3, 4], 5, 6, 7),
            scientist_state_version(1, [2, 3, 4], 6, 6, 7),
            scientist_state_version(1, [2, 3, 4], 5, 7, 7),
            scientist_state_version(1, [2, 3, 4], 5, 6, 8),
        ] {
            assert_ne!(changed, baseline);
        }
    }

    #[test]
    fn status_tail_offsets_are_signed_words_at_descriptor_0x0c_through_0x10() {
        let mut base = BaseFactoryRuntimeState {
            status_descriptor: v2k_formats::collision::StatusComponentDescriptor {
                raw_word_at_0x00: 0,
                variable_bindings: [0; 6],
                raw_tail: [0, 0, 0, 0, 0x34, 0x12, 0x00, 0x80, 0xff, 0xff],
            },
            control_value_raw: 0,
            required_scientists: 0,
            current_scientists: 0,
            lifter_progress_raw: 0,
            production_progress_raw: 0,
            recovery_progress_raw: 0,
            production: None,
            live_owner: None,
            progressive_death: ProgressiveDeathState::idle(0),
        };
        assert_eq!(status_pickup_spawn_offset(base), [0x1234, i16::MIN, -1]);
        base.status_descriptor.raw_tail = [0; 10];
        assert_eq!(status_pickup_spawn_offset(base), [0; 3]);
    }

    #[test]
    fn pickup_presence_matches_retail_handle_and_state_gates() {
        assert_eq!(
            classify_factory_pickup_presence(None),
            Ok(FactoryPickupPresence::Gone)
        );
        assert_eq!(
            classify_factory_pickup_presence(Some(RetailStateWord::exact(0))),
            Ok(FactoryPickupPresence::Gone)
        );
        assert_eq!(
            classify_factory_pickup_presence(Some(RetailStateWord::exact(DYING_STATE_BIT))),
            Ok(FactoryPickupPresence::Gone)
        );
        assert_eq!(
            classify_factory_pickup_presence(Some(RetailStateWord::exact(0x0E40_8805))),
            Ok(FactoryPickupPresence::Active)
        );
        assert_eq!(
            classify_factory_pickup_presence(Some(RetailStateWord::exact(0x0E40_C805))),
            Ok(FactoryPickupPresence::Gone)
        );
        assert_eq!(
            classify_factory_pickup_presence(Some(RetailStateWord::unknown())),
            Err(())
        );
        assert_eq!(
            classify_factory_pickup_presence(Some(RetailStateWord::from_known_bits(
                0,
                DYING_STATE_BIT
            ))),
            Err(())
        );
    }

    #[test]
    fn converted_output_classes_2_to_6_select_non_type8_families() {
        use crate::factory_production_live::converted_output_entity_type;
        assert_eq!(converted_output_entity_type(1), SCIENTIST_ENTITY_TYPE);
        for (class, entity_type) in [(2, 0x5B), (3, 0x5A), (4, 0x4F), (5, 0x74), (6, 7)] {
            assert_eq!(converted_output_entity_type(class), entity_type);
            assert_ne!(entity_type, SCIENTIST_ENTITY_TYPE);
        }
    }

    #[test]
    fn live_owner_admits_damaged_repair_and_rejects_progressive_death() {
        let idle = ProgressiveDeathState::idle(0);
        assert_eq!(
            validate_level_one_live_owner_outer_state(idle, 99_999, 99_999),
            Ok(())
        );
        assert_eq!(
            validate_level_one_live_owner_outer_state(idle, 99_998, 99_999),
            Ok(())
        );
        assert_eq!(
            validate_level_one_live_owner_outer_state(
                ProgressiveDeathState {
                    elapsed_micros_raw: 1,
                    config_flags_at_0x18: 0,
                },
                99_999,
                99_999,
            ),
            Err(FactoryActivationLiveError::FactoryProgressiveDeathActive {
                elapsed_micros_raw: 1,
            })
        );
        assert_eq!(
            validate_level_one_live_owner_outer_state(idle, 0, 99_999),
            Err(FactoryActivationLiveError::FactoryNotAtFullHealth {
                current: 0,
                maximum: 99_999,
            })
        );
    }

    #[test]
    fn exact_template_retains_authored_capacity_while_runtime_capacity_drains() {
        let base = BaseFactoryRuntimeState {
            status_descriptor: v2k_formats::collision::StatusComponentDescriptor {
                raw_word_at_0x00: 0,
                variable_bindings: [0; 6],
                raw_tail: [0; 10],
            },
            control_value_raw: 0,
            required_scientists: LEVEL_ONE_FACTORY_CAPACITY as u16,
            current_scientists: 0,
            lifter_progress_raw: 0,
            production_progress_raw: 0,
            recovery_progress_raw: 0,
            production: None,
            live_owner: None,
            progressive_death: ProgressiveDeathState::idle(0),
        };
        let mut production = FactoryProductionRuntime {
            output_payload_packed: LEVEL_ONE_FACTORY_OUTPUT_PAYLOAD,
            scientist_capacity_raw: LEVEL_ONE_FACTORY_CAPACITY,
            production_threshold_micros_raw: LEVEL_ONE_FACTORY_PRODUCTION_THRESHOLD_US,
            delivery_duration_micros_raw: LEVEL_ONE_FACTORY_DELIVERY_DURATION_US,
            cooldown_duration_micros_raw: LEVEL_ONE_FACTORY_COOLDOWN_US,
            understaffed_limit_micros_raw: LEVEL_ONE_FACTORY_UNDERSTAFFED_LIMIT_US,
            remaining_stock_raw: 0,
            cooldown_remaining_micros_raw: 0,
            production_progress_micros_raw: 0,
            delivery_progress_micros_raw: 0,
            current_scientists_raw: 0,
            understaffed_countdown_micros_raw: 0,
            health_loss_accumulator_raw: 0,
            cached_health_raw: LEVEL_ONE_FACTORY_MAX_HEALTH_RAW,
            health_per_scientist_raw: LEVEL_ONE_FACTORY_MAX_HEALTH_RAW / LEVEL_ONE_FACTORY_CAPACITY,
            repair_rate_remainder_raw: 0,
            repair_rate_raw: 0,
            understaffed_step_per_scientist_raw: 0,
            spawned_pickup_handle: 0,
            phase: FactoryProductionPhase::WaitingForPickup,
            primary_voice: None,
            secondary_voice: None,
        };
        for capacity in 0..=LEVEL_ONE_FACTORY_CAPACITY {
            production.scientist_capacity_raw = capacity;
            assert!(is_exact_level_one_template(
                base,
                production,
                LEVEL_ONE_FACTORY_MAX_HEALTH_RAW
            ));
        }
        production.scientist_capacity_raw = -1;
        assert!(!is_exact_level_one_template(
            base,
            production,
            LEVEL_ONE_FACTORY_MAX_HEALTH_RAW
        ));
    }

    #[test]
    fn public_entry_projection_allows_bounded_unpublished_deliveries() {
        let mut base = BaseFactoryRuntimeState {
            status_descriptor: v2k_formats::collision::StatusComponentDescriptor {
                raw_word_at_0x00: 0,
                variable_bindings: [0; 6],
                raw_tail: [0; 10],
            },
            control_value_raw: 0,
            required_scientists: 2,
            current_scientists: 0,
            lifter_progress_raw: 0,
            production_progress_raw: 0,
            recovery_progress_raw: 0,
            production: None,
            live_owner: None,
            progressive_death: ProgressiveDeathState::idle(0),
        };
        let mut production = FactoryProductionRuntime {
            output_payload_packed: 0,
            scientist_capacity_raw: 2,
            production_threshold_micros_raw: 0,
            delivery_duration_micros_raw: 0,
            cooldown_duration_micros_raw: 0,
            understaffed_limit_micros_raw: 0,
            remaining_stock_raw: 0,
            cooldown_remaining_micros_raw: 0,
            production_progress_micros_raw: 0,
            delivery_progress_micros_raw: 0,
            current_scientists_raw: 0,
            understaffed_countdown_micros_raw: 0,
            health_loss_accumulator_raw: 0,
            cached_health_raw: 0,
            health_per_scientist_raw: 0,
            repair_rate_remainder_raw: 0,
            repair_rate_raw: 0,
            understaffed_step_per_scientist_raw: 0,
            spawned_pickup_handle: 0,
            phase: FactoryProductionPhase::Producing,
            primary_voice: None,
            secondary_voice: None,
        };
        assert!(level_one_entry_projection_is_authentic(base, production));
        production.current_scientists_raw = 1;
        assert!(level_one_entry_projection_is_authentic(base, production));
        production.current_scientists_raw = 2;
        assert!(level_one_entry_projection_is_authentic(base, production));
        base.current_scientists = 1;
        assert!(level_one_entry_projection_is_authentic(base, production));
        production.current_scientists_raw = 0;
        assert!(!level_one_entry_projection_is_authentic(base, production));
        production.current_scientists_raw = 2;
        production.scientist_capacity_raw = 1;
        assert!(!level_one_entry_projection_is_authentic(base, production));
    }

    #[v2k_test_support::retail_test]
    fn damaged_staffed_level_one_factory_repairs_on_the_live_owner() {
        let dir = v2k_test_support::retail_dir();
        let mut session = crate::session::GameSession::init(&dir).expect("session");
        session.load_auxiliary_ovl(3, 0).expect("L3");
        let table = session.cache.global_entity_model_table();
        let type_metadata: Vec<_> = table
            .iter()
            .copied()
            .enumerate()
            .map(|(entity_type, model_slots)| {
                session
                    .cache
                    .global_entity_type(entity_type)
                    .map(crate::entity_collision_state::EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(crate::entity_collision_state::EntityTypeRuntimeMetadata {
                        model_slots,
                        ..crate::entity_collision_state::EntityTypeRuntimeMetadata::default()
                    })
            })
            .collect();
        session.load_level_by_id(13, 0).expect("level 13");
        let level = session.cache.level_desc().expect("Section 13");
        let terrain = session.cache.terrain().expect("terrain");
        let mut world_fx = crate::world_fx::WorldFx::new();
        // The live factory owner is published only by the authenticated first-world route.
        let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
            level,
            &type_metadata,
            Some(terrain),
            0,
            &mut world_fx,
        )
        .expect("fresh first-world construction");
        let factory_id = manager
            .iter_all()
            .find(|entity| entity.entity_type == LEVEL_ONE_FACTORY_ENTITY_TYPE)
            .expect("factory")
            .id;
        {
            let factory = manager.entity_mut(factory_id).expect("factory mut");
            factory.collision.health_raw = RetailRuntimeValue::Known(99_000);
            if let RetailRuntimeValue::Known(Some(state)) = &mut factory.base_factory_runtime {
                if let Some(production) = state.production.as_mut() {
                    production.current_scientists_raw = LEVEL_ONE_FACTORY_CAPACITY;
                    production.scientist_capacity_raw = LEVEL_ONE_FACTORY_CAPACITY;
                }
            }
        }
        let mut notifications = crate::gameplay_notifications::GameplayNotifications::new();
        tick_level_one_factory_owner(
            &mut manager,
            terrain,
            &mut world_fx,
            &mut notifications,
            0,
            factory_id,
            20_000,
        )
        .expect("damaged live owner frame");
        let health = match manager
            .iter_all()
            .find(|entity| entity.id == factory_id)
            .expect("factory")
            .collision
            .health_raw
        {
            RetailRuntimeValue::Known(health) => health,
            RetailRuntimeValue::Unresolved => panic!("health"),
        };
        assert!(
            health > 99_000,
            "staffed repair must raise health, got {health}"
        );
        assert!(health <= LEVEL_ONE_FACTORY_MAX_HEALTH_RAW);
    }
}
