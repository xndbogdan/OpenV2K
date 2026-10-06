//! Scheduler-owned Level-1 Working Factory arrival and production.
//!
//! Arrival still requires a named scientist whose Primary Go-To-Job targets
//! the factory. The pair walker may queue that scientist after an oriented
//! factory/scientist contact; tests may queue it directly. Production reuses
//! the existing full-health phase-0/1 and finite-stock phase-2 bridge. The
//! owner visits the published Working Factory Primary first (`FUN_00401120` ->
//! `FUN_00425C60` under-attack latch/`0xD3`), then `FUN_00419010`, then any
//! leftover queued arrival. Pair-pass applies use [`apply_queued_arrival`]
//! without a production tick. Proximity is never used.
//! The shared `apply_queued_arrival` also services actual native Type66
//! allocations; their task/production visit stays with `Intro2Type66Owner`.

use v2k_formats::terrain::TerrainGrid;

use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::actor_task_owner::{ActorTaskSlot, ActorTaskVisit};
use crate::entity::{Entity, EntityManager};
use crate::entity_collision_state::RetailRuntimeValue;
use crate::factory_activation_live::{
    apply_prepared_factory_scientist_arrival, prepare_factory_scientist_arrival,
    tick_level_one_factory_owner, FactoryActivationLiveError, FactoryProductionFrameOutcome,
    FactoryScientistDeliveryOutcome,
};
use crate::factory_delivery::FactoryDeliveryPairSuffix;
use crate::factory_pair_suffix::FactoryPairSuffixOutcome;
use crate::gameplay_notifications::GameplayNotifications;
use crate::main_base_type66_abort::WorkingFactoryNotificationOutcome;
use crate::type17_impact_reselection::evaluate_under_attack;
use crate::world_fx::WorldFx;

const LEVEL_ONE_FACTORY_SPAWN_INDEX: usize = 23;
const LEVEL_ONE_FACTORY_ENTITY_TYPE: u32 = 66;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelOneFactoryArrivalOwner {
    factory_id: u32,
    pending_scientist_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelOneFactoryArrivalAdoptionError {
    IdentityMismatch { entity_id: u32 },
    NativeType66Owner { entity_id: u32 },
}

/// The delivery machine still reports [`FactoryDeliveryPairSuffix::Unclaimed`].
/// Scheduler application owns the remaining `FUN_00411AD0` visit after that
/// tagged `0xA300` result: null candidate behavior, factory then scientist
/// component slots, and the physical skip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelOneFactoryArrivalPairClaim {
    SchedulerOwned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelOneFactoryArrivalProductionDrop {
    EntityUnavailable,
    IdentityMismatch,
    NativeType66Owner,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LevelOneFactoryArrivalFrame {
    None,
    Applied {
        scientist_id: u32,
        delivery: FactoryScientistDeliveryOutcome,
        pair_claim: LevelOneFactoryArrivalPairClaim,
        machine_pair_suffix: FactoryDeliveryPairSuffix,
        visit_suffix: FactoryPairSuffixOutcome,
    },
    Blocked {
        scientist_id: u32,
        reason: FactoryActivationLiveError,
    },
}

/// Result of the scheduler-owned Working Factory Primary visit that retail
/// runs as `FUN_00401120` -> `FUN_00425C60` before `FUN_00419010`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelOneFactoryTaskVisit {
    WorkingFactory {
        notification: WorkingFactoryNotificationOutcome,
    },
    /// `+0x34` is not a known tick, so the latch is left unchanged.
    LastHitUnresolved,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LevelOneFactoryArrivalProductionOutcome {
    Ticked {
        factory_id: u32,
        task_visit: LevelOneFactoryTaskVisit,
        production: Result<FactoryProductionFrameOutcome, FactoryActivationLiveError>,
        arrival: LevelOneFactoryArrivalFrame,
    },
    Dropped {
        factory_id: u32,
        reason: LevelOneFactoryArrivalProductionDrop,
    },
}

impl LevelOneFactoryArrivalProductionOutcome {
    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::Ticked { factory_id, .. } | Self::Dropped { factory_id, .. } => *factory_id,
        }
    }
}

pub struct LevelOneFactoryArrivalOwnerTick {
    pub outcome: LevelOneFactoryArrivalProductionOutcome,
    pub retained_owner: Option<LevelOneFactoryArrivalOwner>,
}

impl LevelOneFactoryArrivalOwner {
    pub const fn entity_id(self) -> u32 {
        self.factory_id
    }

    pub const fn pending_scientist_id(self) -> Option<u32> {
        self.pending_scientist_id
    }

    pub(crate) fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }

    pub fn adopt_published(entity: &Entity) -> Result<Self, LevelOneFactoryArrivalAdoptionError> {
        // A native receipt owns the complete 25C60/19010 visit, including
        // source instance data. Spawn23 is not authority to replace it with
        // the retained Level1 bridge, even when that bridge is queried first.
        if entity.intro2_type66_runtime.is_some() {
            return Err(LevelOneFactoryArrivalAdoptionError::NativeType66Owner {
                entity_id: entity.id,
            });
        }
        if entity.authored_spawn_index != Some(LEVEL_ONE_FACTORY_SPAWN_INDEX)
            || entity.entity_type != LEVEL_ONE_FACTORY_ENTITY_TYPE
        {
            return Err(LevelOneFactoryArrivalAdoptionError::IdentityMismatch {
                entity_id: entity.id,
            });
        }
        Ok(Self {
            factory_id: entity.id,
            pending_scientist_id: None,
        })
    }

    pub fn queue_explicit_scientist(&mut self, scientist_id: u32) {
        self.pending_scientist_id = Some(scientist_id);
    }

    pub(crate) fn take_pending_scientist(&mut self) -> Option<u32> {
        self.pending_scientist_id.take()
    }
}

pub fn tick_level_one_factory_arrival_owner(
    manager: &mut EntityManager,
    mut owner: LevelOneFactoryArrivalOwner,
    terrain: &TerrainGrid,
    world_fx: &mut WorldFx,
    notifications: &mut GameplayNotifications,
    retail_tick: u32,
    elapsed_micros: u32,
    main_base_abort_active: bool,
) -> LevelOneFactoryArrivalOwnerTick {
    let Some(factory) = manager
        .iter_all()
        .find(|entity| entity.id == owner.factory_id)
    else {
        return drop_owner(
            owner,
            LevelOneFactoryArrivalProductionDrop::EntityUnavailable,
        );
    };
    if factory.intro2_type66_runtime.is_some() {
        // Defend retained custody as well as new adoption. Do this before
        // opening the WorkingFactory wrapper or accumulating its elapsed time.
        return drop_owner(
            owner,
            LevelOneFactoryArrivalProductionDrop::NativeType66Owner,
        );
    }
    if factory.authored_spawn_index != Some(LEVEL_ONE_FACTORY_SPAWN_INDEX)
        || factory.entity_type != LEVEL_ONE_FACTORY_ENTITY_TYPE
    {
        return drop_owner(
            owner,
            LevelOneFactoryArrivalProductionDrop::IdentityMismatch,
        );
    }

    // FUN_00413500 visits the factory task before FUN_00411A80 pair intake.
    // FUN_00425C60 samples under-attack, then calls FUN_00419010 in-callback.
    let (task_visit, open_visit) = begin_level_one_factory_working_visit(
        manager,
        owner.factory_id,
        notifications,
        retail_tick,
        elapsed_micros,
        main_base_abort_active,
    );
    let production = tick_level_one_factory_owner(
        manager,
        terrain,
        world_fx,
        notifications,
        retail_tick,
        owner.factory_id,
        elapsed_micros,
    );
    finish_level_one_factory_working_visit(manager, owner.factory_id, open_visit);
    let arrival = match owner.pending_scientist_id.take() {
        None => LevelOneFactoryArrivalFrame::None,
        Some(scientist_id) => apply_queued_arrival(
            manager,
            owner.factory_id,
            scientist_id,
            world_fx,
            notifications,
            retail_tick,
        ),
    };
    LevelOneFactoryArrivalOwnerTick {
        outcome: LevelOneFactoryArrivalProductionOutcome::Ticked {
            factory_id: owner.factory_id,
            task_visit,
            production,
            arrival,
        },
        retained_owner: Some(owner),
    }
}

fn begin_level_one_factory_working_visit(
    manager: &mut EntityManager,
    factory_id: u32,
    notifications: &mut GameplayNotifications,
    retail_tick: u32,
    elapsed_micros: u32,
    main_base_abort_active: bool,
) -> (LevelOneFactoryTaskVisit, Option<ActorTaskVisit>) {
    let Some(entity) = manager.entity_mut(factory_id) else {
        return (LevelOneFactoryTaskVisit::Unavailable, None);
    };
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) else {
        return (LevelOneFactoryTaskVisit::Unavailable, None);
    };
    if !matches!(
        entity.actor_tasks.state_in_slot(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::WorkingFactory(_))
    ) {
        return (LevelOneFactoryTaskVisit::Unavailable, None);
    }
    let last_hit_tick = match entity.collision.last_hit_presentation_tick_at_0x34 {
        RetailRuntimeValue::Known(tick) => Some(tick),
        RetailRuntimeValue::Unresolved => None,
    };
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    if entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| match runtime {
            ActorTaskRuntime::WorkingFactory(task) => {
                Some(task.accumulate_elapsed_prefix(elapsed_micros))
            }
            _ => None,
        })
        .flatten()
        .is_none()
    {
        return (LevelOneFactoryTaskVisit::Unavailable, None);
    }
    let Some(last_hit_tick) = last_hit_tick else {
        return (LevelOneFactoryTaskVisit::LastHitUnresolved, Some(visit));
    };
    let Some(ActorTaskRuntime::WorkingFactory(task)) =
        entity.actor_tasks.exact_callback_state_mut(visit)
    else {
        let _ = entity.actor_tasks.finish_exact_visit(visit);
        return (LevelOneFactoryTaskVisit::Unavailable, None);
    };
    let under_attack = evaluate_under_attack(retail_tick, last_hit_tick);
    let notification = task.sample_under_attack_with(under_attack, main_base_abort_active, |_| {
        notifications.queue_factory_under_attack(retail_tick as i32);
    });
    (
        LevelOneFactoryTaskVisit::WorkingFactory { notification },
        Some(visit),
    )
}

fn finish_level_one_factory_working_visit(
    manager: &mut EntityManager,
    factory_id: u32,
    visit: Option<ActorTaskVisit>,
) {
    let Some(visit) = visit else {
        return;
    };
    if let Some(entity) = manager.entity_mut(factory_id) {
        let _ = entity.actor_tasks.finish_exact_visit(visit);
    }
}

pub(crate) fn apply_queued_arrival(
    manager: &mut EntityManager,
    factory_id: u32,
    scientist_id: u32,
    world_fx: &mut WorldFx,
    notifications: &mut GameplayNotifications,
    retail_tick: u32,
) -> LevelOneFactoryArrivalFrame {
    match prepare_factory_scientist_arrival(manager, factory_id, scientist_id) {
        Ok(prepared) => {
            match apply_prepared_factory_scientist_arrival(
                manager,
                world_fx,
                notifications,
                retail_tick,
                prepared,
            ) {
                Ok(delivery) => LevelOneFactoryArrivalFrame::Applied {
                    scientist_id,
                    machine_pair_suffix: delivery.pair_suffix,
                    visit_suffix: delivery.visit_suffix,
                    delivery,
                    pair_claim: LevelOneFactoryArrivalPairClaim::SchedulerOwned,
                },
                Err(reason) => LevelOneFactoryArrivalFrame::Blocked {
                    scientist_id,
                    reason,
                },
            }
        }
        Err(reason) => LevelOneFactoryArrivalFrame::Blocked {
            scientist_id,
            reason,
        },
    }
}

fn drop_owner(
    owner: LevelOneFactoryArrivalOwner,
    reason: LevelOneFactoryArrivalProductionDrop,
) -> LevelOneFactoryArrivalOwnerTick {
    LevelOneFactoryArrivalOwnerTick {
        outcome: LevelOneFactoryArrivalProductionOutcome::Dropped {
            factory_id: owner.factory_id,
            reason,
        },
        retained_owner: None,
    }
}

/// Stamp spawn-23 `+0x34` so a later owner visit can prove `FUN_00416490`.
/// Live hits still go through checked projectile damage.
pub fn stamp_level_one_factory_last_hit_tick(
    entities: &mut EntityManager,
    factory_id: u32,
    retail_tick: u32,
) -> bool {
    let Some(entity) = entities.entity_mut(factory_id) else {
        return false;
    };
    if entity.authored_spawn_index != Some(LEVEL_ONE_FACTORY_SPAWN_INDEX)
        || entity.entity_type != LEVEL_ONE_FACTORY_ENTITY_TYPE
    {
        return false;
    }
    entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(retail_tick);
    true
}

/// Read the Working Factory Primary's `FUN_00425C60` latch.
pub fn level_one_factory_under_attack_latched(
    entities: &EntityManager,
    factory_id: u32,
) -> Option<bool> {
    let entity = entities.iter_all().find(|entity| entity.id == factory_id)?;
    match entity.actor_task_state(ActorTaskSlot::Primary) {
        Some(ActorTaskRuntime::WorkingFactory(task)) => {
            Some(task.under_attack_notification_latched())
        }
        _ => None,
    }
}
