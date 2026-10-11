//! Live-list-ordered custody for specialized actor task owners.
//!
//! Retail `FUN_00413500` visits tasks through the actor manager's intrusive
//! order. Type-specific receipt vectors therefore cannot be run as separate
//! batches when their callbacks share the process RNG or effect owners. This
//! scheduler holds only authenticated receipts and selects each family at its
//! actor's one position in a single pass.
//!
//! The bounded composition joins the already-live Type-17 and Type-47
//! Common-Dying families and Type-17 Follow Beacons' published acquire/handoff
//! graph with cargo-converted type-8 Go-To-Job, the Level-1 factory's
//! explicit-arrival and production owner, Main Base Type-54's terrain-owning Change-Sea-
//! Level task, Main Base Type-9's Exploding Person task, and Main Base Type-66's
//! progressive Working Factory task, ordinary Type-9's selected Run Away,
//! Attract-Attention, Go-To-Job, and Wander graphs, ordinary Type-47's
//! published Guard/Wander graphs, Intro2 type-13's captured B6C0 Search And
//! Attack graph, Intro2 type-26's captured Defecate Virus
//! Primary, Intro2 Type-47's captured Guard-anchor Primary, and hard-water
//! Type 60's class-48 Exploding
//! Ring task. Class-49 Type-60 tails remain deliberately absent from retained scheduler custody:
//! they are born after the normal task pass and die later in the same Main
//! Base sweep.

mod capture;
mod hive;
pub use hive::HiveComponentProductionContext;
#[cfg(test)]
mod capture_radial_tests;
mod contact_prefix;
mod delivered_type9_contact;
mod native_ground;
mod native_type38;
mod native_type43;
mod native_weapons;
pub use native_weapons::{
    NativeWeaponProductionBlock, NativeWeaponProductionOutcome, NativeWeaponRegistrationBlock,
    SpecializedActorTaskWorld,
};
pub(crate) mod playing_radial;
pub use playing_radial::{
    PlayerRadialBlock, PlayerRadialBlockReason, PlayingRadialBlock, PlayingRadialFrame,
    PlayingRadialOutcome,
};

use crate::common_dying_live::LevelOneType17CommonDyingOwner;
use crate::common_mover::type9_owner::OrdinaryType9OwnerTransactionId;
use crate::common_mover::type9_surface::ActorSurfaceBubbleRequest;
use crate::entity::EntityManager;
use crate::entity_collision_state::RetailRuntimeValue;
#[cfg(test)]
use crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT;
use crate::entity_view_detail::RetailViewDetailContext;
use crate::factory_arrival_production::{
    apply_queued_arrival, tick_level_one_factory_arrival_owner, LevelOneFactoryArrivalFrame,
    LevelOneFactoryArrivalOwner, LevelOneFactoryArrivalProductionDrop,
    LevelOneFactoryArrivalProductionOutcome,
};
use crate::gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications};
use crate::guard_location_owner::acquisition::GuardLocationEntityRef;
use crate::intro2_flyers_live::{
    tick_intro2_flyer_scheduler_owner_with_random, Intro2FlyerFrame, Intro2FlyerSchedulerOwner,
    Intro2FlyerSchedulerProductionDrop, Intro2FlyerSchedulerProductionOutcome,
};
use crate::intro2_meteors::{
    tick_intro2_meteor_with_random, Intro2MeteorFrame, Intro2MeteorOutcome, Intro2MeteorOwner,
};
use crate::intro2_type13_live::{
    tick_intro2_type13_world_owner_with_random, Intro2Type13PrimaryFrame, Intro2Type13WorldDrop,
    Intro2Type13WorldOutcome, Intro2Type13WorldOwner,
};
use crate::intro2_type26_defecate_virus::{
    tick_intro2_type26_world, Intro2Type26WorldFrame, Intro2Type26WorldOutcome,
    Intro2Type26WorldOwner,
};
use crate::intro2_type47_live::world::{
    tick_intro2_type47_world_owner, Intro2Type47WorldDrop, Intro2Type47WorldFrame,
    Intro2Type47WorldOutcome, Intro2Type47WorldOwner,
};
use crate::main_base_abort::MainBaseAbortActorLease;
use crate::main_base_type54_abort::{
    tick_main_base_type54_sea_level_owner, MainBaseType54SeaLevelProductionOutcome,
    MainBaseType54SeaLevelTickBlock, MainBaseType54SeaLevelTickReceipt,
};
use crate::main_base_type66_abort::MainBaseType66WorkingFactoryTaskLease;
use crate::main_base_type66_production::{
    tick_main_base_type66_production_owner, MainBaseType66ProductionDrop,
    MainBaseType66ProductionFrame, MainBaseType66ProductionOutcome, MainBaseType66ProductionOwner,
};
use crate::main_base_type9_abort::MainBaseType9ExplodingTaskLease;
use crate::main_base_type9_actor_production::{
    tick_main_base_type9_actor_owner_with_random, MainBaseType9ActorProductionFrame,
    MainBaseType9ActorProductionOwner,
};
use crate::main_base_type9_production::{
    tick_main_base_type9_exploding_owner, MainBaseType9ExplodingProductionBlock,
    MainBaseType9ExplodingProductionDrop, MainBaseType9ExplodingProductionFrame,
    MainBaseType9ExplodingProductionOutcome, MainBaseType9ExplodingProductionOwner,
};
use crate::ordinary_type47_death_live::FreshLevelOneType47CommonDyingOwner;
use crate::ordinary_type9_attract_attention_production::{
    tick_ordinary_type9_attract_attention_owner_with_random,
    OrdinaryType9AttractAttentionProductionDrop, OrdinaryType9AttractAttentionProductionFrame,
    OrdinaryType9AttractAttentionProductionOutcome, OrdinaryType9AttractAttentionProductionOwner,
    OrdinaryType9AttractAttentionProductionResume,
};
use crate::ordinary_type9_cargo::Type9CarriedOwner;
use crate::ordinary_type9_carried_production::{
    tick_type9_carried_owner, Type9CarriedProductionDrop, Type9CarriedProductionFrame,
    Type9CarriedProductionOutcome,
};
use crate::ordinary_type9_current_task::OrdinaryType9CurrentTaskAuthority;
use crate::ordinary_type9_go_to_job_production::{
    tick_ordinary_type9_go_to_job_owner_with_random, OrdinaryType9GoToJobProductionDrop,
    OrdinaryType9GoToJobProductionFrame, OrdinaryType9GoToJobProductionOutcome,
    OrdinaryType9GoToJobProductionOwner, OrdinaryType9GoToJobProductionResume,
};
use crate::ordinary_type9_initial_production::{
    FreshLevel1Type9InitialProductionOwner, FreshLevel1Type9PublishedSchedulerBranch,
};
use crate::ordinary_type9_live::OrdinaryType9SelectedRuntimeKind;
use crate::ordinary_type9_outer_tail::{
    apply_class14_scheduler_prefix, drive_outer_tail_transaction, start_class14_suffix_transaction,
    OrdinaryType9Class14SchedulerPrefix,
};
use crate::ordinary_type9_run_away_production::{
    tick_ordinary_type9_run_away_owner_with_random, OrdinaryType9RunAwayProductionDrop,
    OrdinaryType9RunAwayProductionFrame, OrdinaryType9RunAwayProductionOutcome,
    OrdinaryType9RunAwayProductionOwner, OrdinaryType9RunAwayProductionResume,
};
use crate::ordinary_type9_wander_production::{
    tick_ordinary_type9_wander_owner_with_random, OrdinaryType9WanderProductionDrop,
    OrdinaryType9WanderProductionFrame, OrdinaryType9WanderProductionOutcome,
    OrdinaryType9WanderProductionOwner, OrdinaryType9WanderProductionResume,
};
use crate::resource_cache::ResourceCache;
use crate::type17_common_dying_production::{
    publish_type17_common_dying_owner_presented_view_detail, tick_type17_common_dying_owner,
    Type17CommonDyingProductionDrop, Type17CommonDyingProductionFrame,
    Type17CommonDyingProductionOutcome,
};
use crate::type17_follow_beacons_production::{
    tick_type17_follow_beacons_scheduler_owner, Type17FollowBeaconsProductionFrame,
    Type17FollowBeaconsSchedulerOwner, Type17FollowBeaconsSchedulerProductionDrop,
    Type17FollowBeaconsSchedulerProductionOutcome,
};
use crate::type17_impact_reselection::TYPE17_IMPACT_ENTITY_TYPE;
use crate::type17_initial_behavior_live::FRESH_LEVEL1_TYPE17_SPAWN_INDICES;
use crate::type47_common_dying_production::{
    tick_type47_common_dying_owner, Type47CommonDyingProductionDrop,
    Type47CommonDyingProductionFrame, Type47CommonDyingProductionOutcome,
};
use crate::type47_scheduler_production::{
    tick_ordinary_type47_scheduler_owner, OrdinaryType47SchedulerAdoptionError,
    OrdinaryType47SchedulerOwner, OrdinaryType47SchedulerProductionDrop,
    OrdinaryType47SchedulerProductionOutcome,
};
use crate::type60_exploding_ring::Type60ExplodingRingTaskLease;
use crate::type60_exploding_ring_production::{
    tick_type60_exploding_ring_production_owner, Type60ExplodingRingProductionDrop,
    Type60ExplodingRingProductionOutcome, Type60ExplodingRingProductionOwner,
};
use crate::type8_go_to_job_production::{
    tick_type8_go_to_job_scheduler_owner, Type8GoToJobSchedulerOwner,
    Type8GoToJobSchedulerProductionDrop, Type8GoToJobSchedulerProductionOutcome,
};
use crate::world_fx::{
    ParticleEnvironment, TerrainCollisionContext, TerrainExplosionLight, WorldFx,
};

/// Specialized family whose authenticated task receipt is in scheduler
/// custody.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecializedActorTaskFamily {
    OrdinaryType9Carried,
    MainBaseType54SeaLevel,
    MainBaseType9Exploding,
    MainBaseType66Production,
    OrdinaryType9AttractAttention,
    OrdinaryType9Class14,
    OrdinaryType9GoToJob,
    OrdinaryType9RunAway,
    OrdinaryType9Wander,
    Type17CommonDying,
    Type17FollowBeacons,
    Type8GoToJob,
    LevelOneFactoryArrival,
    Type47CommonDying,
    OrdinaryType47Scheduler,
    Intro2Type13SearchAttack,
    Intro2Type26,
    Intro2Type47Scheduler,
    Intro2FlyerScheduler,
    Intro2Meteor,
    Intro2CommonDying,
    Intro2Type9Class14,
    Intro2Type8,
    Intro2Type53,
    NativeType122,
    NativeType30,
    NativeType40,
    NativeType56,
    NativeType43,
    NativeType38Family,
    NativeType123,
    NativeType86,
    SharedFish,
    CleansingVehicle,
    Intro2Type16,
    Intro2Type58,
    Intro2Type94,
    Intro2Type66,
    MainBase,
    Class0Actor,
    Intro2Type10,
    Intro2Type57,
    Intro2GunTurret,
    Intro2Type10Tumble,
    Intro2Type57Tumble,
    Intro2Type17,
    Type60ExplodingRing,
    NativeWeapon,
}

#[derive(Debug, PartialEq, Eq)]
enum SpecializedActorTaskOwner {
    /// A real443D10 release completed before10B70 retired ordinary visits.
    /// Contact may borrow its exact graph until14990 removes the allocation.
    DeliveredType9Contact {
        allocation: MainBaseAbortActorLease,
        retained: Box<SpecializedActorTaskOwner>,
    },
    /// A late contact stopped after a committed source prefix. Keep the exact
    /// linear owner, but lend neither another tick nor external mutation.
    NativeContactPrefix {
        allocation: MainBaseAbortActorLease,
        family: SpecializedActorTaskFamily,
        retained: Box<SpecializedActorTaskOwner>,
    },
    OrdinaryType9Carried(Type9CarriedOwner),
    MainBaseType54SeaLevel(MainBaseType54SeaLevelTickReceipt),
    MainBaseType9Exploding(MainBaseType9ActorProductionOwner),
    MainBaseType66Production(MainBaseType66ProductionOwner),
    OrdinaryType9AttractAttention(OrdinaryType9AttractAttentionProductionOwner),
    OrdinaryType9Class14(MainBaseType9ExplodingProductionOwner),
    OrdinaryType9GoToJob(OrdinaryType9GoToJobProductionOwner),
    OrdinaryType9RunAway(OrdinaryType9RunAwayProductionOwner),
    OrdinaryType9Wander(OrdinaryType9WanderProductionOwner),
    Type17CommonDying(LevelOneType17CommonDyingOwner),
    Type17FollowBeacons(Type17FollowBeaconsSchedulerOwner),
    Type8GoToJob(Type8GoToJobSchedulerOwner),
    LevelOneFactoryArrival(LevelOneFactoryArrivalOwner),
    Type47CommonDying(FreshLevelOneType47CommonDyingOwner),
    OrdinaryType47Scheduler(OrdinaryType47SchedulerOwner),
    Intro2Type13SearchAttack(Intro2Type13WorldOwner),
    Intro2Type26(Intro2Type26WorldOwner),
    Intro2Type47Scheduler(Intro2Type47WorldOwner),
    Intro2FlyerScheduler(Intro2FlyerSchedulerOwner),
    Intro2Meteor(Intro2MeteorOwner),
    Intro2CommonDying(crate::intro2_common_dying::Intro2CommonDyingOwner),
    Intro2Type9Class14(crate::intro2_type9_class14::Intro2Type9Class14Owner),
    Intro2Type8(crate::intro2_type8::Intro2Type8Owner),
    Intro2Type53(crate::intro2_type53::Intro2Type53Owner),
    NativeType122(crate::native_type122::Type122Owner),
    NativeType30(crate::native_type30::Type30Owner),
    NativeType40(crate::native_type40::Type40Owner),
    NativeType56(crate::native_type56::Type56Owner),
    NativeType43(crate::native_type43::Type43Owner),
    NativeType38Family(crate::native_type38::Type38FamilyOwner),
    NativeType123(crate::native_type123::Type123Owner),
    NativeType86(crate::native_type86::Type86Owner),
    SharedFish(crate::shared_fish::SharedFishOwner),
    CleansingVehicle(crate::cleansing_vehicle::CleansingVehicleOwner),
    Intro2Type16(crate::intro2_type16::Intro2Type16Owner),
    Intro2Type58(crate::intro2_type58::Intro2Type58Owner),
    Intro2Type94(crate::intro2_type94::Intro2Type94Owner),
    Intro2Type66(crate::intro2_type66::Intro2Type66Owner),
    MainBase(crate::main_base_runtime::MainBaseOwner),
    Class0Actor(crate::entity::Class0ActorOwner),
    Intro2Type10(crate::intro2_type10::Intro2Type10Owner),
    Intro2Type57(crate::intro2_type57::Intro2Type57Owner),
    Intro2GunTurret(crate::intro2_gun_turret::Intro2GunTurretOwner),
    Intro2Type10Tumble(crate::intro2_type10::Intro2Type10TumbleOwner),
    Intro2Type57Tumble(crate::intro2_type57::death::Intro2Type57TumbleOwner),
    Intro2Type17(crate::intro2_type17::Intro2Type17Owner),
    Type60ExplodingRing(Type60ExplodingRingProductionOwner),
    NativeWeapon(crate::native_entity_weapons::NativeEntityWeaponOwner),
}

impl SpecializedActorTaskOwner {
    /// Shared allocation identity for Type-9 simulation and presentation.
    fn type9_actor_lease(&self) -> Option<MainBaseAbortActorLease> {
        match self {
            Self::NativeContactPrefix { retained, .. }
            | Self::DeliveredType9Contact { retained, .. } => retained.type9_actor_lease(),
            Self::OrdinaryType9Carried(owner) => Some(owner.actor),
            Self::MainBaseType9Exploding(owner) => Some(owner.task_lease().actor()),
            Self::OrdinaryType9Class14(owner) => Some(owner.task_lease().actor()),
            Self::Intro2Type9Class14(owner) => Some(owner.actor_lease()),
            Self::OrdinaryType9RunAway(owner) => Some(owner.actor_lease()),
            Self::OrdinaryType9AttractAttention(owner) => Some(owner.actor_lease()),
            Self::OrdinaryType9GoToJob(owner) => Some(owner.actor_lease()),
            Self::OrdinaryType9Wander(owner) => Some(owner.actor_lease()),
            _ => None,
        }
    }

    fn from_current_type9(
        authority: OrdinaryType9CurrentTaskAuthority,
        actor_lease: MainBaseAbortActorLease,
        next_transaction_id: u64,
    ) -> Self {
        if authority.has_completed_attract_continuation() {
            return Self::OrdinaryType9AttractAttention(
                OrdinaryType9AttractAttentionProductionOwner::from_current_attract(
                    authority,
                    actor_lease,
                    next_transaction_id,
                ),
            );
        }
        match authority.kind() {
            OrdinaryType9SelectedRuntimeKind::AttractAttentionTargetRoutePublished => {
                Self::OrdinaryType9AttractAttention(
                    OrdinaryType9AttractAttentionProductionOwner::from_current_attract(
                        authority,
                        actor_lease,
                        next_transaction_id,
                    ),
                )
            }
            OrdinaryType9SelectedRuntimeKind::WanderNearPublished
            | OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished => {
                Self::OrdinaryType9Wander(OrdinaryType9WanderProductionOwner::from_current_task(
                    authority,
                    actor_lease,
                    next_transaction_id,
                ))
            }
            OrdinaryType9SelectedRuntimeKind::GoToJobPublished => {
                Self::OrdinaryType9GoToJob(OrdinaryType9GoToJobProductionOwner::from_current_task(
                    authority,
                    actor_lease,
                    next_transaction_id,
                ))
            }
            OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished
            | OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished => {
                Self::OrdinaryType9RunAway(OrdinaryType9RunAwayProductionOwner::from_current_task(
                    authority,
                    actor_lease,
                    next_transaction_id,
                ))
            }
            _ => {
                unreachable!("a current task receipt comes from a supported exact root publication")
            }
        }
    }

    fn continue_current_type9(mut self, manager: &EntityManager) -> Self {
        let transfer = match &mut self {
            Self::OrdinaryType9Wander(owner) => owner
                .take_completed_current_task(manager)
                .map(|transfer| (transfer, owner.actor_lease())),
            Self::OrdinaryType9GoToJob(owner) => owner
                .take_completed_current_task(manager)
                .map(|transfer| (transfer, owner.actor_lease())),
            Self::OrdinaryType9RunAway(owner) => owner
                .take_completed_current_task(manager)
                .map(|transfer| (transfer, owner.actor_lease())),
            Self::OrdinaryType9AttractAttention(owner) => owner
                .take_completed_current_task(manager)
                .map(|transfer| (transfer, owner.actor_lease())),
            _ => None,
        };
        match transfer {
            Some(((authority, next_transaction_id), actor_lease)) => {
                Self::from_current_type9(authority, actor_lease, next_transaction_id)
            }
            None => self,
        }
    }
    const fn entity_id(&self) -> u32 {
        match self {
            Self::NativeContactPrefix { allocation, .. }
            | Self::DeliveredType9Contact { allocation, .. } => allocation.entity_id,
            Self::OrdinaryType9Carried(owner) => owner.entity_id(),
            Self::MainBaseType54SeaLevel(receipt) => receipt.lease().actor().entity_id,
            Self::MainBaseType9Exploding(owner) => owner.entity_id(),
            Self::MainBaseType66Production(owner) => owner.entity_id(),
            Self::OrdinaryType9AttractAttention(owner) => owner.entity_id(),
            Self::OrdinaryType9Class14(owner) => owner.entity_id(),
            Self::OrdinaryType9GoToJob(owner) => owner.entity_id(),
            Self::OrdinaryType9RunAway(owner) => owner.entity_id(),
            Self::OrdinaryType9Wander(owner) => owner.entity_id(),
            Self::Type17CommonDying(owner) => owner.entity_id(),
            Self::Type17FollowBeacons(owner) => owner.entity_id(),
            Self::Type8GoToJob(owner) => owner.entity_id(),
            Self::LevelOneFactoryArrival(owner) => owner.entity_id(),
            Self::Type47CommonDying(owner) => owner.entity_id(),
            Self::OrdinaryType47Scheduler(owner) => owner.entity_id(),
            Self::Intro2Type13SearchAttack(owner) => owner.entity_id(),
            Self::Intro2Type26(owner) => owner.entity_id(),
            Self::Intro2Type47Scheduler(owner) => owner.entity_id(),
            Self::Intro2FlyerScheduler(owner) => owner.entity_id(),
            Self::Intro2Meteor(owner) => owner.entity_id(),
            Self::Intro2CommonDying(owner) => owner.entity_id(),
            Self::Intro2Type9Class14(owner) => owner.entity_id(),
            Self::Intro2Type8(owner) => owner.entity_id(),
            Self::Intro2Type53(owner) => owner.entity_id(),
            Self::NativeType122(owner) => owner.entity_id(),
            Self::NativeType30(owner) => owner.entity_id(),
            Self::NativeType40(owner) => owner.entity_id(),
            Self::NativeType56(owner) => owner.entity_id(),
            Self::NativeType43(owner) => owner.entity_id(),
            Self::NativeType38Family(owner) => owner.entity_id(),
            Self::NativeType123(owner) => owner.entity_id(),
            Self::NativeType86(owner) => owner.entity_id(),
            Self::SharedFish(owner) => owner.entity_id(),
            Self::CleansingVehicle(owner) => owner.entity_id(),
            Self::Intro2Type16(owner) => owner.entity_id(),
            Self::Intro2Type58(owner) => owner.entity_id(),
            Self::Intro2Type94(owner) => owner.entity_id(),
            Self::Intro2Type66(owner) => owner.entity_id(),
            Self::MainBase(owner) => owner.entity_id(),
            Self::Class0Actor(owner) => owner.entity_id(),
            Self::Intro2Type10(owner) => owner.entity_id(),
            Self::Intro2Type57(owner) => owner.entity_id(),
            Self::Intro2GunTurret(owner) => owner.entity_id(),
            Self::Intro2Type10Tumble(owner) => owner.entity_id(),
            Self::Intro2Type57Tumble(owner) => owner.entity_id(),
            Self::Intro2Type17(owner) => owner.entity_id(),
            Self::Type60ExplodingRing(owner) => owner.entity_id(),
            Self::NativeWeapon(owner) => owner.entity_id(),
        }
    }

    const fn family(&self) -> SpecializedActorTaskFamily {
        match self {
            Self::NativeContactPrefix { family, .. } => *family,
            Self::DeliveredType9Contact { retained, .. } => retained.family(),
            Self::OrdinaryType9Carried(_) => SpecializedActorTaskFamily::OrdinaryType9Carried,
            Self::MainBaseType54SeaLevel(_) => SpecializedActorTaskFamily::MainBaseType54SeaLevel,
            Self::MainBaseType9Exploding(_) => SpecializedActorTaskFamily::MainBaseType9Exploding,
            Self::MainBaseType66Production(_) => {
                SpecializedActorTaskFamily::MainBaseType66Production
            }
            Self::OrdinaryType9AttractAttention(_) => {
                SpecializedActorTaskFamily::OrdinaryType9AttractAttention
            }
            Self::OrdinaryType9Class14(_) => SpecializedActorTaskFamily::OrdinaryType9Class14,
            Self::OrdinaryType9GoToJob(_) => SpecializedActorTaskFamily::OrdinaryType9GoToJob,
            Self::OrdinaryType9RunAway(_) => SpecializedActorTaskFamily::OrdinaryType9RunAway,
            Self::OrdinaryType9Wander(_) => SpecializedActorTaskFamily::OrdinaryType9Wander,
            Self::Type17CommonDying(_) => SpecializedActorTaskFamily::Type17CommonDying,
            Self::Type17FollowBeacons(_) => SpecializedActorTaskFamily::Type17FollowBeacons,
            Self::Type8GoToJob(_) => SpecializedActorTaskFamily::Type8GoToJob,
            Self::LevelOneFactoryArrival(_) => SpecializedActorTaskFamily::LevelOneFactoryArrival,
            Self::Type47CommonDying(_) => SpecializedActorTaskFamily::Type47CommonDying,
            Self::OrdinaryType47Scheduler(_) => SpecializedActorTaskFamily::OrdinaryType47Scheduler,
            Self::Intro2Type13SearchAttack(_) => {
                SpecializedActorTaskFamily::Intro2Type13SearchAttack
            }
            Self::Intro2Type26(_) => SpecializedActorTaskFamily::Intro2Type26,
            Self::Intro2Type47Scheduler(_) => SpecializedActorTaskFamily::Intro2Type47Scheduler,
            Self::Intro2FlyerScheduler(_) => SpecializedActorTaskFamily::Intro2FlyerScheduler,
            Self::Intro2Meteor(_) => SpecializedActorTaskFamily::Intro2Meteor,
            Self::Intro2CommonDying(_) => SpecializedActorTaskFamily::Intro2CommonDying,
            Self::Intro2Type9Class14(_) => SpecializedActorTaskFamily::Intro2Type9Class14,
            Self::Intro2Type8(_) => SpecializedActorTaskFamily::Intro2Type8,
            Self::Intro2Type53(_) => SpecializedActorTaskFamily::Intro2Type53,
            Self::NativeType122(_) => SpecializedActorTaskFamily::NativeType122,
            Self::NativeType30(_) => SpecializedActorTaskFamily::NativeType30,
            Self::NativeType40(_) => SpecializedActorTaskFamily::NativeType40,
            Self::NativeType56(_) => SpecializedActorTaskFamily::NativeType56,
            Self::NativeType43(_) => SpecializedActorTaskFamily::NativeType43,
            Self::NativeType38Family(_) => SpecializedActorTaskFamily::NativeType38Family,
            Self::NativeType123(_) => SpecializedActorTaskFamily::NativeType123,
            Self::NativeType86(_) => SpecializedActorTaskFamily::NativeType86,
            Self::SharedFish(_) => SpecializedActorTaskFamily::SharedFish,
            Self::CleansingVehicle(_) => SpecializedActorTaskFamily::CleansingVehicle,
            Self::Intro2Type16(_) => SpecializedActorTaskFamily::Intro2Type16,
            Self::Intro2Type58(_) => SpecializedActorTaskFamily::Intro2Type58,
            Self::Intro2Type94(_) => SpecializedActorTaskFamily::Intro2Type94,
            Self::Intro2Type66(_) => SpecializedActorTaskFamily::Intro2Type66,
            Self::MainBase(_) => SpecializedActorTaskFamily::MainBase,
            Self::Class0Actor(_) => SpecializedActorTaskFamily::Class0Actor,
            Self::Intro2Type10(_) => SpecializedActorTaskFamily::Intro2Type10,
            Self::Intro2Type57(_) => SpecializedActorTaskFamily::Intro2Type57,
            Self::Intro2GunTurret(_) => SpecializedActorTaskFamily::Intro2GunTurret,
            Self::Intro2Type10Tumble(_) => SpecializedActorTaskFamily::Intro2Type10Tumble,
            Self::Intro2Type57Tumble(_) => SpecializedActorTaskFamily::Intro2Type57Tumble,
            Self::Intro2Type17(_) => SpecializedActorTaskFamily::Intro2Type17,
            Self::Type60ExplodingRing(_) => SpecializedActorTaskFamily::Type60ExplodingRing,
            Self::NativeWeapon(_) => SpecializedActorTaskFamily::NativeWeapon,
        }
    }

    fn fork_for_main_base_abort_transaction(&self) -> Self {
        match self {
            Self::DeliveredType9Contact {
                allocation,
                retained,
            } => Self::DeliveredType9Contact {
                allocation: *allocation,
                retained: Box::new(retained.fork_for_main_base_abort_transaction()),
            },
            Self::NativeContactPrefix {
                allocation,
                family,
                retained,
            } => Self::NativeContactPrefix {
                allocation: *allocation,
                family: *family,
                retained: Box::new(retained.fork_for_main_base_abort_transaction()),
            },
            Self::OrdinaryType9Carried(owner) => {
                Self::OrdinaryType9Carried(owner.fork_for_main_base_abort_transaction())
            }
            Self::MainBaseType54SeaLevel(receipt) => {
                Self::MainBaseType54SeaLevel(receipt.fork_for_main_base_abort_transaction())
            }
            Self::MainBaseType9Exploding(owner) => {
                Self::MainBaseType9Exploding(owner.fork_for_main_base_abort_transaction())
            }
            Self::MainBaseType66Production(owner) => {
                Self::MainBaseType66Production(owner.fork_for_main_base_abort_transaction())
            }
            Self::OrdinaryType9AttractAttention(owner) => {
                Self::OrdinaryType9AttractAttention(owner.fork_for_main_base_abort_transaction())
            }
            Self::OrdinaryType9Class14(owner) => {
                Self::OrdinaryType9Class14(owner.fork_for_main_base_abort_transaction())
            }
            Self::OrdinaryType9GoToJob(owner) => {
                Self::OrdinaryType9GoToJob(owner.fork_for_main_base_abort_transaction())
            }
            Self::OrdinaryType9RunAway(owner) => {
                Self::OrdinaryType9RunAway(owner.fork_for_main_base_abort_transaction())
            }
            Self::OrdinaryType9Wander(owner) => {
                Self::OrdinaryType9Wander(owner.fork_for_main_base_abort_transaction())
            }
            Self::Type17CommonDying(owner) => Self::Type17CommonDying(*owner),
            Self::Type17FollowBeacons(owner) => {
                Self::Type17FollowBeacons(owner.fork_for_main_base_abort_transaction())
            }
            Self::Type8GoToJob(owner) => {
                Self::Type8GoToJob(owner.fork_for_main_base_abort_transaction())
            }
            Self::LevelOneFactoryArrival(owner) => {
                Self::LevelOneFactoryArrival(owner.fork_for_main_base_abort_transaction())
            }
            Self::Type47CommonDying(owner) => Self::Type47CommonDying(*owner),
            Self::OrdinaryType47Scheduler(owner) => {
                Self::OrdinaryType47Scheduler(owner.fork_for_main_base_abort_transaction())
            }
            Self::Intro2Type13SearchAttack(owner) => {
                Self::Intro2Type13SearchAttack(owner.fork_for_main_base_abort_transaction())
            }
            Self::Intro2Type26(owner) => {
                Self::Intro2Type26(owner.fork_for_main_base_abort_transaction())
            }
            Self::Intro2Type47Scheduler(owner) => {
                Self::Intro2Type47Scheduler(owner.fork_for_main_base_abort_transaction())
            }
            Self::Intro2FlyerScheduler(owner) => {
                Self::Intro2FlyerScheduler(owner.fork_for_main_base_abort_transaction())
            }
            Self::Intro2Meteor(owner) => {
                Self::Intro2Meteor(owner.fork_for_main_base_abort_transaction())
            }
            Self::Intro2Type8(owner) => {
                Self::Intro2Type8(owner.fork_for_main_base_abort_transaction())
            }
            Self::Intro2Type53(owner) => {
                Self::Intro2Type53(owner.fork_for_main_base_abort_transaction())
            }
            Self::NativeType122(owner) => {
                Self::NativeType122(owner.fork_for_main_base_abort_transaction())
            }
            Self::NativeType30(owner) => {
                Self::NativeType30(owner.fork_for_main_base_abort_transaction())
            }
            Self::NativeType40(owner) => {
                Self::NativeType40(owner.fork_for_main_base_abort_transaction())
            }
            Self::NativeType56(owner) => {
                Self::NativeType56(owner.fork_for_main_base_abort_transaction())
            }
            Self::NativeType43(owner) => {
                Self::NativeType43(owner.fork_for_main_base_abort_transaction())
            }
            Self::NativeType38Family(owner) => {
                Self::NativeType38Family(owner.fork_for_main_base_abort_transaction())
            }
            Self::NativeType123(owner) => {
                Self::NativeType123(owner.fork_for_main_base_abort_transaction())
            }
            Self::NativeType86(owner) => {
                Self::NativeType86(owner.fork_for_main_base_abort_transaction())
            }
            Self::SharedFish(owner) => {
                Self::SharedFish(owner.fork_for_main_base_abort_transaction())
            }
            Self::CleansingVehicle(owner) => {
                Self::CleansingVehicle(owner.fork_for_main_base_abort_transaction())
            }
            Self::Intro2Type16(owner) => {
                Self::Intro2Type16(owner.fork_for_main_base_abort_transaction())
            }
            Self::Intro2Type58(owner) => {
                Self::Intro2Type58(owner.fork_for_main_base_abort_transaction())
            }
            Self::Intro2Type94(owner) => {
                Self::Intro2Type94(owner.fork_for_main_base_abort_transaction())
            }
            Self::Intro2Type66(owner) => {
                Self::Intro2Type66(owner.fork_for_main_base_abort_transaction())
            }
            Self::MainBase(owner) => Self::MainBase(owner.fork_for_main_base_abort_transaction()),
            Self::Class0Actor(owner) => {
                Self::Class0Actor(owner.fork_for_main_base_abort_transaction())
            }
            Self::Intro2Type10(owner) => {
                Self::Intro2Type10(owner.fork_for_main_base_abort_transaction())
            }
            Self::Intro2Type57(owner) => {
                Self::Intro2Type57(owner.fork_for_main_base_abort_transaction())
            }
            Self::Intro2GunTurret(owner) => {
                Self::Intro2GunTurret(owner.fork_for_main_base_abort_transaction())
            }
            Self::Intro2Type10Tumble(owner) => {
                Self::Intro2Type10Tumble(owner.fork_for_main_base_abort_transaction())
            }
            Self::Intro2Type57Tumble(owner) => {
                Self::Intro2Type57Tumble(owner.fork_for_main_base_abort_transaction())
            }
            Self::Intro2Type17(owner) => {
                Self::Intro2Type17(owner.fork_for_main_base_abort_transaction())
            }
            Self::Intro2CommonDying(owner) => {
                Self::Intro2CommonDying(owner.fork_for_main_base_abort_transaction())
            }
            Self::Intro2Type9Class14(owner) => {
                Self::Intro2Type9Class14(owner.fork_for_main_base_abort_transaction())
            }
            Self::Type60ExplodingRing(owner) => {
                Self::Type60ExplodingRing(owner.fork_for_main_base_abort_transaction())
            }
            Self::NativeWeapon(owner) => Self::NativeWeapon(*owner),
        }
    }
}

/// A Main Base transaction asked for selected ordinary Type-9 production
/// custody but the scheduler's exact allocation claim did not match. The
/// owner remains parked on every error path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9SelectedCustodyTakeBlock {
    DifferentFamily {
        entity_id: u32,
        actual: SpecializedActorTaskFamily,
    },
    ActorLeaseMismatch {
        expected: MainBaseAbortActorLease,
        actual: MainBaseAbortActorLease,
    },
    /// Root planning/publication is linear non-cloneable custody. A forked
    /// Main Base transaction must leave this scheduler owner installed rather
    /// than decompose it through the stale fresh-construction sidecar.
    RootCustodyUnavailable { entity_id: u32 },
}

#[cfg(test)]
pub type OrdinaryType9RunAwayCustodyTakeBlock = OrdinaryType9SelectedCustodyTakeBlock;

/// Linear selected ordinary Type-9 authority transferred between the live
/// scheduler and an isolated Main Base abort transaction without probing one
/// branch and accidentally dropping the other.
#[derive(Debug, PartialEq, Eq)]
pub enum OrdinaryType9SelectedProductionOwner {
    RunAway(OrdinaryType9RunAwayProductionOwner),
    AttractAttention(OrdinaryType9AttractAttentionProductionOwner),
    GoToJob(OrdinaryType9GoToJobProductionOwner),
    Wander(OrdinaryType9WanderProductionOwner),
}

impl OrdinaryType9SelectedProductionOwner {
    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::RunAway(owner) => owner.entity_id(),
            Self::AttractAttention(owner) => owner.entity_id(),
            Self::GoToJob(owner) => owner.entity_id(),
            Self::Wander(owner) => owner.entity_id(),
        }
    }

    pub const fn actor_lease(&self) -> MainBaseAbortActorLease {
        match self {
            Self::RunAway(owner) => owner.actor_lease(),
            Self::AttractAttention(owner) => owner.actor_lease(),
            Self::GoToJob(owner) => owner.actor_lease(),
            Self::Wander(owner) => owner.actor_lease(),
        }
    }

    #[cfg(test)]
    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        match self {
            Self::RunAway(owner) => Self::RunAway(owner.fork_for_main_base_abort_transaction()),
            Self::AttractAttention(owner) => {
                Self::AttractAttention(owner.fork_for_main_base_abort_transaction())
            }
            Self::GoToJob(owner) => Self::GoToJob(owner.fork_for_main_base_abort_transaction()),
            Self::Wander(owner) => Self::Wander(owner.fork_for_main_base_abort_transaction()),
        }
    }

    pub(crate) fn decompose_for_main_base_abort(
        self,
    ) -> (
        usize,
        FreshLevel1Type9InitialProductionOwner,
        OrdinaryType9SelectedProductionResume,
    ) {
        match self {
            Self::RunAway(owner) => {
                let (index, initial_owner, resume) = owner.decompose_for_main_base_abort();
                (
                    index,
                    initial_owner,
                    OrdinaryType9SelectedProductionResume::RunAway(resume),
                )
            }
            Self::AttractAttention(owner) => {
                let (index, initial_owner, resume) = owner.decompose_for_main_base_abort();
                (
                    index,
                    initial_owner,
                    OrdinaryType9SelectedProductionResume::AttractAttention(resume),
                )
            }
            Self::GoToJob(owner) => {
                let (index, initial_owner, resume) = owner.decompose_for_main_base_abort();
                (
                    index,
                    initial_owner,
                    OrdinaryType9SelectedProductionResume::GoToJob(resume),
                )
            }
            Self::Wander(owner) => {
                let (index, initial_owner, resume) = owner.decompose_for_main_base_abort();
                (
                    index,
                    initial_owner,
                    OrdinaryType9SelectedProductionResume::Wander(resume),
                )
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OrdinaryType9SelectedProductionResume {
    RunAway(OrdinaryType9RunAwayProductionResume),
    AttractAttention(OrdinaryType9AttractAttentionProductionResume),
    GoToJob(OrdinaryType9GoToJobProductionResume),
    Wander(OrdinaryType9WanderProductionResume),
}

impl OrdinaryType9SelectedProductionResume {
    pub(crate) fn resume_after_main_base_abort_noop(
        self,
        original_manager_sidecar_index: usize,
        initial_owner: FreshLevel1Type9InitialProductionOwner,
    ) -> Result<OrdinaryType9SelectedProductionOwner, FreshLevel1Type9InitialProductionOwner> {
        match self {
            Self::RunAway(resume) => {
                OrdinaryType9RunAwayProductionOwner::resume_after_main_base_abort_noop(
                    original_manager_sidecar_index,
                    initial_owner,
                    resume,
                )
                .map(OrdinaryType9SelectedProductionOwner::RunAway)
            }
            Self::AttractAttention(resume) => {
                OrdinaryType9AttractAttentionProductionOwner::resume_after_main_base_abort_noop(
                    original_manager_sidecar_index,
                    initial_owner,
                    resume,
                )
                .map(OrdinaryType9SelectedProductionOwner::AttractAttention)
            }
            Self::GoToJob(resume) => {
                OrdinaryType9GoToJobProductionOwner::resume_after_main_base_abort_noop(
                    original_manager_sidecar_index,
                    initial_owner,
                    resume,
                )
                .map(OrdinaryType9SelectedProductionOwner::GoToJob)
            }
            Self::Wander(resume) => {
                OrdinaryType9WanderProductionOwner::resume_after_main_base_abort_noop(
                    original_manager_sidecar_index,
                    initial_owner,
                    resume,
                )
                .map(OrdinaryType9SelectedProductionOwner::Wander)
            }
        }
    }
}

/// Load-time adoption failed before any selected ordinary Type-9 owner entered
/// scheduler custody. Manager construction custody is unchanged on every path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9SelectedLoadAdoptionError {
    RegisteredEntityConflict { entity_id: u32 },
    OwnerAuthenticationFailed { entity_id: u32 },
    ActorUnavailable { entity_id: u32 },
}

/// Load-time adoption failed before any Run Away owner entered scheduler
/// custody. The manager retains every initial-production sidecar on either
/// error path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9RunAwayLoadAdoptionError {
    /// A successful level load must bind initial owners before any specialized
    /// family can publish. Reusing a non-empty scheduler would mix allocation
    /// generations.
    SchedulerNotEmpty { registered_len: usize },
    /// A selected Run Away sidecar no longer authenticates the exact entity,
    /// context, component kind, and current task leases it is meant to move.
    /// The complete manager batch remains untouched.
    OwnerAuthenticationFailed { entity_id: u32 },
    /// A manager sidecar passed branch/entity authentication but its exact
    /// allocation lease could no longer be observed during the same
    /// synchronous load transaction.
    ActorUnavailable { entity_id: u32 },
}

/// A Main Base transaction asked for Type-60 task custody but the scheduler's
/// exact allocation claim did not match. The owner remains parked on either
/// error path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type60ExplodingRingCustodyTakeBlock {
    DifferentFamily {
        entity_id: u32,
        actual: SpecializedActorTaskFamily,
    },
    ActorLeaseMismatch {
        expected: MainBaseAbortActorLease,
        actual: MainBaseAbortActorLease,
    },
}

/// A second family attempted to claim the same actor allocation.
///
/// Same-family publications replace their older receipt. Cross-family
/// replacement would hide a task-topology bug, so it is rejected explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecializedActorTaskRegistrationConflict {
    pub entity_id: u32,
    pub existing: SpecializedActorTaskFamily,
    pub attempted: SpecializedActorTaskFamily,
}

/// Failed cross-family registration with the rejected authority returned to
/// its caller. This matters for linear task receipts: reporting a topology
/// conflict must not silently discard the only authority able to diagnose or
/// recover that publication.
#[derive(Debug, PartialEq, Eq)]
pub struct SpecializedActorTaskRegistrationFailure<T> {
    pub conflict: SpecializedActorTaskRegistrationConflict,
    pub rejected: T,
}

/// One capped task-pass invocation shared by the currently adopted families.
pub struct SpecializedActorTaskProductionFrame<'a> {
    /// Scene-owned player damage context for synchronous weapon explosions.
    pub world: SpecializedActorTaskWorld<'a>,
    /// Complete retained Hive lane. Component-only test callers may omit it;
    /// runtime scenes supply its campaign tally and current view context.
    pub hive_components: Option<HiveComponentProductionContext<'a>>,
    /// Strict current-level resource owner. Section 10 is borrowed afresh at
    /// each live-list visit; Type 54 alone receives a mutable borrow.
    pub resources: &'a mut ResourceCache,
    /// Process-global RNG/effect owner shared by every specialized callback.
    pub world_fx: &'a mut WorldFx,
    /// Shared static-object damage queue, submitted at the contact's live-list
    /// position before the corresponding actor damage callback.
    pub static_damage: &'a mut crate::static_damage::StaticDamageScheduler,
    pub elapsed_micros: u32,
    /// Retail's process-global callback clock. This is deliberately distinct
    /// from the per-pass elapsed value consumed by task wrappers.
    pub global_elapsed_micros: u32,
    pub retail_tick: u32,
    /// Explicit controller phase supplied by the enclosing runtime scene.
    pub notification_phase: GameplayNotificationPhase,
    /// Whether the enclosing Main Base abort transaction still suppresses the
    /// Working Factory's direct under-attack notification.
    pub main_base_abort_active: bool,
}

/// One family result at its exact position in manager live-list order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecializedActorTaskProductionOutcome {
    NativeContactPrefixBlocked {
        entity_id: u32,
        family: SpecializedActorTaskFamily,
    },
    OrdinaryType9Carried(Type9CarriedProductionOutcome),
    MainBaseType54SeaLevel(MainBaseType54SeaLevelProductionOutcome),
    MainBaseType9Exploding(MainBaseType9ExplodingProductionOutcome),
    MainBaseType66Production(MainBaseType66ProductionOutcome),
    OrdinaryType9AttractAttention(OrdinaryType9AttractAttentionProductionOutcome),
    OrdinaryType9Class14(MainBaseType9ExplodingProductionOutcome),
    OrdinaryType9GoToJob(OrdinaryType9GoToJobProductionOutcome),
    OrdinaryType9RunAway(OrdinaryType9RunAwayProductionOutcome),
    OrdinaryType9Wander(OrdinaryType9WanderProductionOutcome),
    Type17CommonDying(Type17CommonDyingProductionOutcome),
    Type17FollowBeacons(Type17FollowBeaconsSchedulerProductionOutcome),
    Type8GoToJob(Type8GoToJobSchedulerProductionOutcome),
    LevelOneFactoryArrival(LevelOneFactoryArrivalProductionOutcome),
    Type47CommonDying(Type47CommonDyingProductionOutcome),
    OrdinaryType47Scheduler(OrdinaryType47SchedulerProductionOutcome),
    Intro2Type13SearchAttack(Intro2Type13WorldOutcome),
    Intro2Type26(Intro2Type26WorldOutcome),
    Intro2Type47Scheduler(Intro2Type47WorldOutcome),
    Intro2FlyerScheduler(Intro2FlyerSchedulerProductionOutcome),
    Intro2CommonDying(crate::intro2_common_dying::Intro2CommonDyingOutcome),
    Intro2Type9Class14(crate::intro2_type9_class14::Intro2Type9Class14Outcome),
    Intro2Type8(crate::intro2_type8::Intro2Type8Outcome),
    Intro2Type53(crate::intro2_type53::Intro2Type53Outcome),
    NativeType122(crate::native_type122::Type122Outcome),
    NativeType30(crate::native_type30::Type30Outcome),
    NativeType40(crate::native_type40::Type40Outcome),
    NativeType56(crate::native_type56::Type56Outcome),
    NativeType43(crate::native_type43::Type43Outcome),
    NativeType38Family(crate::native_type38::Type38Outcome),
    NativeType123(crate::native_type123::Type123Outcome),
    NativeType86(crate::native_type86::Type86Outcome),
    SharedFish(crate::shared_fish::SharedFishOutcome),
    CleansingVehicle(crate::cleansing_vehicle::CleansingVehicleOutcome),
    Intro2Type16(crate::intro2_type16::Intro2Type16Outcome),
    Intro2Type58(crate::intro2_type58::Intro2Type58Outcome),
    Intro2Type94(crate::intro2_type94::Intro2Type94Outcome),
    Intro2Type66(crate::intro2_type66::Intro2Type66Outcome),
    MainBase(crate::main_base_runtime::MainBaseOutcome),
    Class0Actor(crate::entity::Class0ActorOutcome),
    Intro2Type10(crate::intro2_type10::Intro2Type10Outcome),
    Intro2Type57(crate::intro2_type57::Intro2Type57Outcome),
    Intro2GunTurret(crate::intro2_gun_turret::Intro2GunTurretOutcome),
    Intro2Type10Tumble(crate::intro2_type10::Intro2Type10TumbleOutcome),
    Intro2Type57Tumble(crate::intro2_type57::Intro2Type57TumbleOutcome),
    Intro2Type17(crate::intro2_type17::Intro2Type17Outcome),
    Intro2Meteor {
        outcome: Intro2MeteorOutcome,
        death: Option<crate::intro2_radial::Intro2MeteorDeathReport>,
    },
    Type60ExplodingRing(Type60ExplodingRingProductionOutcome),
    NativeWeapon(NativeWeaponProductionOutcome),
}

impl SpecializedActorTaskProductionOutcome {
    pub const fn family(&self) -> SpecializedActorTaskFamily {
        match self {
            Self::NativeContactPrefixBlocked { family, .. } => *family,
            Self::OrdinaryType9Carried(_) => SpecializedActorTaskFamily::OrdinaryType9Carried,
            Self::MainBaseType54SeaLevel(_) => SpecializedActorTaskFamily::MainBaseType54SeaLevel,
            Self::MainBaseType9Exploding(_) => SpecializedActorTaskFamily::MainBaseType9Exploding,
            Self::MainBaseType66Production(_) => {
                SpecializedActorTaskFamily::MainBaseType66Production
            }
            Self::OrdinaryType9AttractAttention(_) => {
                SpecializedActorTaskFamily::OrdinaryType9AttractAttention
            }
            Self::OrdinaryType9Class14(_) => SpecializedActorTaskFamily::OrdinaryType9Class14,
            Self::OrdinaryType9GoToJob(_) => SpecializedActorTaskFamily::OrdinaryType9GoToJob,
            Self::OrdinaryType9RunAway(_) => SpecializedActorTaskFamily::OrdinaryType9RunAway,
            Self::OrdinaryType9Wander(_) => SpecializedActorTaskFamily::OrdinaryType9Wander,
            Self::Type17CommonDying(_) => SpecializedActorTaskFamily::Type17CommonDying,
            Self::Type17FollowBeacons(_) => SpecializedActorTaskFamily::Type17FollowBeacons,
            Self::Type8GoToJob(_) => SpecializedActorTaskFamily::Type8GoToJob,
            Self::LevelOneFactoryArrival(_) => SpecializedActorTaskFamily::LevelOneFactoryArrival,
            Self::Type47CommonDying(_) => SpecializedActorTaskFamily::Type47CommonDying,
            Self::OrdinaryType47Scheduler(_) => SpecializedActorTaskFamily::OrdinaryType47Scheduler,
            Self::Intro2Type13SearchAttack(_) => {
                SpecializedActorTaskFamily::Intro2Type13SearchAttack
            }
            Self::Intro2Type26(_) => SpecializedActorTaskFamily::Intro2Type26,
            Self::Intro2Type47Scheduler(_) => SpecializedActorTaskFamily::Intro2Type47Scheduler,
            Self::Intro2FlyerScheduler(_) => SpecializedActorTaskFamily::Intro2FlyerScheduler,
            Self::Intro2Meteor { .. } => SpecializedActorTaskFamily::Intro2Meteor,
            Self::Intro2CommonDying(_) => SpecializedActorTaskFamily::Intro2CommonDying,
            Self::Intro2Type9Class14(_) => SpecializedActorTaskFamily::Intro2Type9Class14,
            Self::Intro2Type8(_) => SpecializedActorTaskFamily::Intro2Type8,
            Self::Intro2Type53(_) => SpecializedActorTaskFamily::Intro2Type53,
            Self::NativeType122(_) => SpecializedActorTaskFamily::NativeType122,
            Self::NativeType30(_) => SpecializedActorTaskFamily::NativeType30,
            Self::NativeType40(_) => SpecializedActorTaskFamily::NativeType40,
            Self::NativeType56(_) => SpecializedActorTaskFamily::NativeType56,
            Self::NativeType43(_) => SpecializedActorTaskFamily::NativeType43,
            Self::NativeType38Family(_) => SpecializedActorTaskFamily::NativeType38Family,
            Self::NativeType123(_) => SpecializedActorTaskFamily::NativeType123,
            Self::NativeType86(_) => SpecializedActorTaskFamily::NativeType86,
            Self::SharedFish(_) => SpecializedActorTaskFamily::SharedFish,
            Self::CleansingVehicle(_) => SpecializedActorTaskFamily::CleansingVehicle,
            Self::Intro2Type16(_) => SpecializedActorTaskFamily::Intro2Type16,
            Self::Intro2Type58(_) => SpecializedActorTaskFamily::Intro2Type58,
            Self::Intro2Type94(_) => SpecializedActorTaskFamily::Intro2Type94,
            Self::Intro2Type66(_) => SpecializedActorTaskFamily::Intro2Type66,
            Self::MainBase(_) => SpecializedActorTaskFamily::MainBase,
            Self::Class0Actor(_) => SpecializedActorTaskFamily::Class0Actor,
            Self::Intro2Type10(_) => SpecializedActorTaskFamily::Intro2Type10,
            Self::Intro2Type57(_) => SpecializedActorTaskFamily::Intro2Type57,
            Self::Intro2GunTurret(_) => SpecializedActorTaskFamily::Intro2GunTurret,
            Self::Intro2Type10Tumble(_) => SpecializedActorTaskFamily::Intro2Type10Tumble,
            Self::Intro2Type57Tumble(_) => SpecializedActorTaskFamily::Intro2Type57Tumble,
            Self::Intro2Type17(_) => SpecializedActorTaskFamily::Intro2Type17,
            Self::Type60ExplodingRing(_) => SpecializedActorTaskFamily::Type60ExplodingRing,
            Self::NativeWeapon(_) => SpecializedActorTaskFamily::NativeWeapon,
        }
    }

    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::NativeContactPrefixBlocked { entity_id, .. } => *entity_id,
            Self::OrdinaryType9Carried(outcome) => outcome.entity_id(),
            Self::MainBaseType54SeaLevel(outcome) => match outcome {
                MainBaseType54SeaLevelProductionOutcome::Continuing { entity_id, .. }
                | MainBaseType54SeaLevelProductionOutcome::DeferredDestroyStaged {
                    entity_id,
                    ..
                }
                | MainBaseType54SeaLevelProductionOutcome::Blocked { entity_id, .. }
                | MainBaseType54SeaLevelProductionOutcome::Dropped { entity_id, .. } => *entity_id,
            },
            Self::MainBaseType9Exploding(outcome) => match outcome {
                MainBaseType9ExplodingProductionOutcome::SchedulerWaiting { entity_id }
                | MainBaseType9ExplodingProductionOutcome::Continuing { entity_id, .. }
                | MainBaseType9ExplodingProductionOutcome::TransitionSuppressed {
                    entity_id, ..
                }
                | MainBaseType9ExplodingProductionOutcome::Blocked { entity_id, .. }
                | MainBaseType9ExplodingProductionOutcome::Dropped { entity_id, .. } => *entity_id,
                MainBaseType9ExplodingProductionOutcome::Terminal(outcome) => outcome.entity_id,
            },
            Self::MainBaseType66Production(outcome) => match outcome {
                MainBaseType66ProductionOutcome::Continuing { entity_id, .. }
                | MainBaseType66ProductionOutcome::Terminal { entity_id, .. }
                | MainBaseType66ProductionOutcome::Blocked { entity_id, .. }
                | MainBaseType66ProductionOutcome::Dropped { entity_id, .. } => *entity_id,
            },
            Self::OrdinaryType9AttractAttention(outcome) => outcome.entity_id(),
            Self::OrdinaryType9Class14(outcome) => match outcome {
                MainBaseType9ExplodingProductionOutcome::SchedulerWaiting { entity_id }
                | MainBaseType9ExplodingProductionOutcome::Continuing { entity_id, .. }
                | MainBaseType9ExplodingProductionOutcome::TransitionSuppressed {
                    entity_id, ..
                }
                | MainBaseType9ExplodingProductionOutcome::Blocked { entity_id, .. }
                | MainBaseType9ExplodingProductionOutcome::Dropped { entity_id, .. } => *entity_id,
                MainBaseType9ExplodingProductionOutcome::Terminal(outcome) => outcome.entity_id,
            },
            Self::OrdinaryType9GoToJob(outcome) => outcome.entity_id(),
            Self::OrdinaryType9RunAway(outcome) => outcome.entity_id(),
            Self::OrdinaryType9Wander(outcome) => outcome.entity_id(),
            Self::Type17CommonDying(outcome) => match outcome {
                Type17CommonDyingProductionOutcome::SchedulerWaiting { entity_id }
                | Type17CommonDyingProductionOutcome::Advanced { entity_id, .. }
                | Type17CommonDyingProductionOutcome::Blocked { entity_id, .. }
                | Type17CommonDyingProductionOutcome::Dropped { entity_id, .. }
                | Type17CommonDyingProductionOutcome::LiveError { entity_id, .. } => *entity_id,
            },
            Self::Type17FollowBeacons(outcome) => outcome.entity_id(),
            Self::Type8GoToJob(outcome) => outcome.entity_id(),
            Self::LevelOneFactoryArrival(outcome) => outcome.entity_id(),
            Self::Type47CommonDying(outcome) => match outcome {
                Type47CommonDyingProductionOutcome::SchedulerWaiting { entity_id }
                | Type47CommonDyingProductionOutcome::Blocked { entity_id, .. }
                | Type47CommonDyingProductionOutcome::Dropped { entity_id, .. } => *entity_id,
                Type47CommonDyingProductionOutcome::DeferredDestroyStaged(frame) => frame.entity_id,
            },
            Self::OrdinaryType47Scheduler(outcome) => outcome.entity_id(),
            Self::Intro2Type13SearchAttack(outcome) => outcome.entity_id(),
            Self::Intro2Type26(outcome) => outcome.entity_id(),
            Self::Intro2Type47Scheduler(outcome) => outcome.entity_id(),
            Self::Intro2FlyerScheduler(outcome) => outcome.entity_id(),
            Self::Intro2Meteor { outcome, .. } => outcome.entity_id(),
            Self::Intro2CommonDying(outcome) => outcome.entity_id(),
            Self::Intro2Type9Class14(outcome) => outcome.entity_id(),
            Self::Intro2Type8(outcome) => outcome.entity_id(),
            Self::Intro2Type53(outcome) => outcome.entity_id(),
            Self::NativeType122(outcome) => outcome.entity_id(),
            Self::NativeType30(outcome) => outcome.entity_id(),
            Self::NativeType40(outcome) => outcome.entity_id(),
            Self::NativeType56(outcome) => outcome.entity_id(),
            Self::NativeType43(outcome) => outcome.entity_id(),
            Self::NativeType38Family(outcome) => outcome.entity_id(),
            Self::NativeType123(outcome) => outcome.entity_id(),
            Self::NativeType86(outcome) => outcome.entity_id(),
            Self::SharedFish(outcome) => outcome.entity_id(),
            Self::CleansingVehicle(outcome) => outcome.entity_id(),
            Self::Intro2Type16(outcome) => outcome.entity_id(),
            Self::Intro2Type58(outcome) => outcome.entity_id(),
            Self::Intro2Type94(outcome) => outcome.entity_id(),
            Self::Intro2Type66(outcome) => outcome.entity_id(),
            Self::MainBase(outcome) => outcome.entity_id(),
            Self::Class0Actor(outcome) => outcome.entity_id(),
            Self::Intro2Type10(outcome) => outcome.entity_id(),
            Self::Intro2Type57(outcome) => outcome.entity_id(),
            Self::Intro2GunTurret(outcome) => outcome.entity_id(),
            Self::Intro2Type10Tumble(outcome) => outcome.entity_id(),
            Self::Intro2Type57Tumble(outcome) => outcome.entity_id(),
            Self::Intro2Type17(outcome) => outcome.entity_id(),
            Self::Type60ExplodingRing(outcome) => match outcome {
                Type60ExplodingRingProductionOutcome::Continuing { entity_id, .. }
                | Type60ExplodingRingProductionOutcome::TerminalClass2 { entity_id, .. }
                | Type60ExplodingRingProductionOutcome::Blocked { entity_id, .. }
                | Type60ExplodingRingProductionOutcome::Dropped { entity_id, .. } => *entity_id,
            },
            Self::NativeWeapon(outcome) => outcome.entity_id(),
        }
    }
}

/// Pass-wide resource boundary resolved before any owner custody or state is
/// taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecializedActorTaskProductionBlock {
    CurrentLevelTerrainUnavailable,
    CurrentLevelCollisionContextUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelOneFactoryArrivalQueueError {
    FactoryOwnerUnavailable { factory_id: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecializedActorTaskProductionPass {
    pub block: Option<SpecializedActorTaskProductionBlock>,
    pub outcomes: Vec<SpecializedActorTaskProductionOutcome>,
    pub surface_bubbles_materialized: usize,
    pub surface_bubbles_dropped: usize,
    /// Whether any inline Type-54 visit changed the strict Section-10 sea
    /// state. Multiple visits accumulate through logical OR.
    pub sea_level_changed: bool,
    /// Immediate terrain-light writes produced by Type-66 staged effects, in
    /// exact live-list/effect-point order. Terminal and dropped callbacks keep
    /// every command already emitted before custody ended.
    pub explosion_lights: Vec<TerrainExplosionLight>,
    pub progressive_death_presentation_requested: bool,
    pub terrain_changed: bool,
}

/// Receipt scheduler whose storage order is never execution order.
#[derive(Debug, Default)]
pub struct SpecializedActorTaskScheduler {
    owners: Vec<SpecializedActorTaskOwner>,
    terminal_abort_origins: Vec<crate::main_base_abort::MainBaseTerminalAbortOrigin>,
    hive_diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeType9HitCustody {
    Selected {
        actor: MainBaseAbortActorLease,
        next_transaction_id: u64,
    },
    Carried,
    Class14,
}

impl SpecializedActorTaskScheduler {
    fn record_hive_diagnostics(&mut self, messages: Vec<String>) {
        for message in messages {
            if !self.hive_diagnostics.contains(&message) {
                eprintln!("{message}");
                self.hive_diagnostics.push(message);
            }
        }
    }
    /// Move each completed native19750 Base origin once into the host's
    /// synchronous level-abort owner. This receipt deliberately is not Clone.
    pub fn take_main_base_terminal_abort_origins(
        &mut self,
    ) -> Vec<crate::main_base_abort::MainBaseTerminalAbortOrigin> {
        std::mem::take(&mut self.terminal_abort_origins)
    }
    /// External hit writers run between completed actor visits. Consume
    /// only that finished observation, retaining task clocks and transaction
    /// identity; a partial mover/root/outer transaction cannot be replaced.
    pub(crate) fn begin_native_type9_external_mutation(
        &mut self,
        manager: &EntityManager,
        entity_id: u32,
    ) -> Option<NativeType9HitCustody> {
        begin_native_type9_external_mutation(&mut self.owners, manager, entity_id)
    }

    /// Admit a radial writer without exposing or replacing its current task
    /// graph. Failed admission leaves the retained owner unchanged.
    pub fn prepare_native_type9_external_mutation(
        &mut self,
        manager: &EntityManager,
        entity_id: u32,
    ) -> bool {
        self.begin_native_type9_external_mutation(manager, entity_id)
            .is_some()
    }

    /// C690 has synchronously published a new graph during the admitted hit.
    pub(crate) fn replace_native_type9_hit_graph(
        &mut self,
        manager: &EntityManager,
        authority: OrdinaryType9CurrentTaskAuthority,
        custody: NativeType9HitCustody,
    ) -> Result<(), OrdinaryType9CurrentTaskAuthority> {
        let NativeType9HitCustody::Selected {
            actor: retained_actor,
            next_transaction_id,
        } = custody
        else {
            return Err(authority);
        };
        let id = authority.entity_id();
        if self.has_native_contact_prefix(id) {
            return Err(authority);
        }
        let Some(actor) = manager.ordinary_type9_selected_actor_lease(id) else {
            return Err(authority);
        };
        if actor != retained_actor {
            return Err(authority);
        }
        let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
            return Err(authority);
        };
        if !authority.authenticates_retained_entity(entity) {
            return Err(authority);
        }
        let Some(index) = self
            .owners
            .iter()
            .position(|owner| owner.entity_id() == id && owner.type9_actor_lease() == Some(actor))
        else {
            return Err(authority);
        };
        let replacement =
            SpecializedActorTaskOwner::from_current_type9(authority, actor, next_transaction_id);
        self.owners[index] =
            delivered_type9_contact::preserve_retirement(&self.owners, replacement);
        Ok(())
    }

    /// Transfer only completed Wander visits to the atomic player-pair writer.
    /// Authenticate every changed Type-9 body before consuming the first tail;
    /// parked task/outer transactions and absent adopted owners stay frozen.
    pub(crate) fn consume_type9_player_contact_visits(
        &mut self,
        manager: &EntityManager,
        entity_ids: &[u32],
    ) -> Result<(), u32> {
        let mut selected_indices = Vec::with_capacity(entity_ids.len());
        for &entity_id in entity_ids {
            let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
                return Err(entity_id);
            };
            if !entity
                .ordinary_type9_selected_component_runtime
                .is_some_and(|selected| {
                    selected.kind() == OrdinaryType9SelectedRuntimeKind::WanderNearPublished
                })
            {
                return Err(entity_id);
            }
            match self
                .owners
                .iter()
                .position(|owner| owner.entity_id() == entity_id)
            {
                Some(index) => {
                    let SpecializedActorTaskOwner::OrdinaryType9Wander(owner) = &self.owners[index]
                    else {
                        return Err(entity_id);
                    };
                    if selected_indices.contains(&index)
                        || owner.completed_visit_lease(manager) != Some(owner.actor_lease())
                    {
                        return Err(entity_id);
                    }
                    selected_indices.push(index);
                }
                None => {
                    // Before load adoption, a genuine initial owner remains in
                    // the manager. An empty scheduler alone is never authority.
                    if !manager
                        .fresh_level1_type9_initial_productions()
                        .iter()
                        .any(|owner| {
                            owner.entity_id() == entity_id
                                && owner.authenticates_retained_entity(entity)
                        })
                    {
                        return Err(entity_id);
                    }
                }
            }
        }
        for index in selected_indices {
            let SpecializedActorTaskOwner::OrdinaryType9Wander(owner) = &mut self.owners[index]
            else {
                unreachable!("the preflighted owner vector has not changed")
            };
            assert!(owner.consume_completed_visit_for_player_contact(manager));
        }
        Ok(())
    }

    /// Read-only proof that the synchronous cargo callback may replace the
    /// selected task graph. Partial prefix/task/outer transactions stay owned.
    pub(crate) fn prepare_type9_cargo_attach(
        &self,
        manager: &EntityManager,
        entity_id: u32,
    ) -> Option<MainBaseAbortActorLease> {
        match self
            .owners
            .iter()
            .find(|owner| owner.entity_id() == entity_id)?
        {
            SpecializedActorTaskOwner::OrdinaryType9Wander(owner) => {
                owner.completed_visit_lease(manager)
            }
            SpecializedActorTaskOwner::OrdinaryType9GoToJob(owner) => {
                owner.completed_visit_lease(manager)
            }
            SpecializedActorTaskOwner::OrdinaryType9RunAway(owner) => {
                owner.completed_visit_lease(manager)
            }
            SpecializedActorTaskOwner::OrdinaryType9AttractAttention(owner) => {
                owner.completed_visit_lease(manager)
            }
            _ => None,
        }
    }

    pub(crate) fn take_type9_for_cargo(
        &mut self,
        manager: &EntityManager,
        actor_lease: MainBaseAbortActorLease,
    ) -> bool {
        if self.prepare_type9_cargo_attach(manager, actor_lease.entity_id) != Some(actor_lease) {
            return false;
        }
        let index = self
            .owners
            .iter()
            .position(|owner| owner.entity_id() == actor_lease.entity_id)
            .expect("the authenticated selected owner remains installed");
        self.owners.remove(index);
        true
    }

    pub(crate) fn adopt_type9_current_task(
        &mut self,
        manager: &EntityManager,
        authority: OrdinaryType9CurrentTaskAuthority,
    ) -> Result<(), OrdinaryType9CurrentTaskAuthority> {
        let entity_id = authority.entity_id();
        let Some(actor_lease) = manager.ordinary_type9_selected_actor_lease(entity_id) else {
            return Err(authority);
        };
        if self
            .owners
            .iter()
            .any(|owner| owner.entity_id() == entity_id)
            || !manager
                .iter_all()
                .find(|entity| entity.id == entity_id)
                .is_some_and(|entity| authority.authenticates_retained_entity(entity))
        {
            return Err(authority);
        }
        self.owners
            .push(SpecializedActorTaskOwner::from_current_type9(
                authority,
                actor_lease,
                1,
            ));
        Ok(())
    }
    pub const fn new() -> Self {
        Self {
            owners: Vec::new(),
            terminal_abort_origins: Vec::new(),
            hive_diagnostics: Vec::new(),
        }
    }

    /// Deep-copy retained linear owners only for the isolated Main Base abort
    /// transaction.
    ///
    /// This is deliberately not `Clone`.  The production wrapper evaluates
    /// this scheduler only with the matching forked EntityManager, never ticks
    /// the parked live owners, and swaps or drops the pair as one unit.
    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            hive_diagnostics: self.hive_diagnostics.clone(),
            owners: self
                .owners
                .iter()
                .map(SpecializedActorTaskOwner::fork_for_main_base_abort_transaction)
                .collect(),
            terminal_abort_origins: self.terminal_abort_origins.iter()
                .map(crate::main_base_abort::MainBaseTerminalAbortOrigin::fork_for_main_base_abort_transaction)
                .collect(),
        }
    }

    /// Move every authenticated successful fresh-Level-1 Run Away, Attract-
    /// Attention, Go-To-Job, and Wander owner into scheduler custody as one
    /// atomic load transaction.
    /// Original manager order and branch identity are retained across both the
    /// success path and any preflight rollback. Earlier load phases may already
    /// own other actors (451C00 requires class0 custody before attachment);
    /// disjoint owners are retained without imposing a family-specific order.
    pub fn adopt_fresh_level1_type9_selected(
        &mut self,
        manager: &mut EntityManager,
    ) -> Result<usize, OrdinaryType9SelectedLoadAdoptionError> {
        let sidecars = manager
            .take_fresh_level1_type9_scheduler_productions()
            .map_err(|entity_id| {
                OrdinaryType9SelectedLoadAdoptionError::OwnerAuthenticationFailed { entity_id }
            })?;
        if let Some(entity_id) = sidecars.iter().find_map(|(_, _, owner)| {
            self.owners
                .iter()
                .any(|registered| registered.entity_id() == owner.entity_id())
                .then_some(owner.entity_id())
        }) {
            restore_fresh_level1_type9_selected_sidecars(manager, sidecars);
            return Err(
                OrdinaryType9SelectedLoadAdoptionError::RegisteredEntityConflict { entity_id },
            );
        }
        let mut leases = Vec::with_capacity(sidecars.len());
        let unavailable = sidecars.iter().find_map(|(_, _, owner)| {
            let entity_id = owner.entity_id();
            if manager.iter_all().find(|entity| entity.id == entity_id)
                .is_some_and(|entity| entity.ordinary_type9_native_receipt.is_some())
                && !crate::ordinary_type9_construction::ordinary_type9_native_allocation_authenticates(manager, entity_id) {
                return Some(entity_id);
            }
            manager
                .ordinary_type9_selected_actor_lease(entity_id)
                .map(|lease| leases.push(lease))
                .is_none()
                .then_some(entity_id)
        });
        if let Some(entity_id) = unavailable {
            restore_fresh_level1_type9_selected_sidecars(manager, sidecars);
            return Err(OrdinaryType9SelectedLoadAdoptionError::ActorUnavailable { entity_id });
        }

        let adopted_len = sidecars.len();
        self.owners.extend(sidecars.into_iter().zip(leases).map(
            |((original_index, branch, initial_owner), actor_lease)| {
                match branch {
                    FreshLevel1Type9PublishedSchedulerBranch::RunAway => {
                        SpecializedActorTaskOwner::OrdinaryType9RunAway(
                            OrdinaryType9RunAwayProductionOwner::adopt(
                                original_index,
                                initial_owner,
                                actor_lease,
                            )
                            .expect("manager extraction authenticates selected Run Away adoption"),
                        )
                    }
                    FreshLevel1Type9PublishedSchedulerBranch::AttractAttention => {
                        SpecializedActorTaskOwner::OrdinaryType9AttractAttention(
                            OrdinaryType9AttractAttentionProductionOwner::adopt(
                                original_index,
                                initial_owner,
                                actor_lease,
                            )
                            .expect(
                                "manager extraction authenticates selected Attract Attention adoption",
                            ),
                        )
                    }
                    FreshLevel1Type9PublishedSchedulerBranch::GoToJob => {
                        SpecializedActorTaskOwner::OrdinaryType9GoToJob(
                            OrdinaryType9GoToJobProductionOwner::adopt(
                                original_index,
                                initial_owner,
                                actor_lease,
                            )
                            .expect("manager extraction authenticates selected Go-To-Job adoption"),
                        )
                    }
                    FreshLevel1Type9PublishedSchedulerBranch::Wander => {
                        SpecializedActorTaskOwner::OrdinaryType9Wander(
                            OrdinaryType9WanderProductionOwner::adopt(
                                original_index,
                                initial_owner,
                                actor_lease,
                            )
                            .expect("manager extraction authenticates selected Wander adoption"),
                        )
                    }
                }
            },
        ));
        Ok(adopted_len)
    }

    /// Move successful fresh-Level-1 Type-47 Guard/Wander publications into
    /// scheduler custody. Fallback graphs stay in manager custody. Adoption
    /// is atomic: a later graph failure restores every sidecar already taken.
    pub fn adopt_fresh_level1_type47_scheduler(
        &mut self,
        manager: &mut EntityManager,
    ) -> Result<usize, OrdinaryType47SchedulerAdoptionError> {
        let sidecars = manager.take_fresh_level1_type47_scheduler_productions();
        let mut adopted = Vec::with_capacity(sidecars.len());
        for publication in sidecars {
            if manager.iter_all().any(|entity| {
                entity.id == publication.entity_id && entity.native_type47_construction.is_some()
            }) {
                // Native construction retains its own complete graph receipt.
                // A stale captured sidecar cannot install a second live owner.
                continue;
            }
            match OrdinaryType47SchedulerOwner::adopt(manager, publication) {
                Ok(owner) => adopted.push(owner),
                Err(error) => {
                    for owner in adopted {
                        manager.retain_fresh_level1_type47_initial_production(owner.publication());
                    }
                    manager.retain_fresh_level1_type47_initial_production(publication);
                    return Err(error);
                }
            }
        }
        let adopted_len = adopted.len();
        for owner in adopted {
            self.owners
                .push(SpecializedActorTaskOwner::OrdinaryType47Scheduler(owner));
        }
        Ok(adopted_len)
    }

    /// Adopt published fresh-Level-1 Follow Beacons graphs.
    ///
    /// Cold New-Game spawn 18 publishes class 33. Capture People/Run Away and
    /// direct `--level 13` stay out of custody. Variant-one first `FUN_00403CE0`
    /// starts `FUN_00401430` and fails closed at type 17's first `FUN_0041FCB0`.
    pub fn adopt_fresh_level1_type17_follow_beacons(&mut self, manager: &EntityManager) -> usize {
        let metadata = match manager.type_runtime_metadata(TYPE17_IMPACT_ENTITY_TYPE) {
            Some(metadata) => metadata.clone(),
            None => return 0,
        };
        let owners = manager
            .iter_all()
            .filter(|entity| {
                entity.active
                    && entity.entity_type == TYPE17_IMPACT_ENTITY_TYPE
                    && entity.intro2_type17_runtime.is_none()
                    && entity
                        .authored_spawn_index
                        .is_some_and(|index| FRESH_LEVEL1_TYPE17_SPAWN_INDICES.contains(&index))
            })
            .filter_map(|entity| {
                Type17FollowBeaconsSchedulerOwner::adopt_published(entity, &metadata).ok()
            })
            .collect::<Vec<_>>();
        let adopted = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::Type17FollowBeacons),
        );
        adopted
    }

    /// Adopt Main-Base cargo-converted type-8 scientists that already own
    /// Go-To-Job plus the `V200002` seed `0x38` Sub-D owner.
    ///
    /// Factory-ejected type-8 stays out of custody. Already-owned ids are
    /// skipped so later conversion births can be adopted without resetting
    /// earlier visits.
    pub fn adopt_live_type8_go_to_job(&mut self, manager: &EntityManager) -> usize {
        let metadata = match manager.type_runtime_metadata(8) {
            Some(metadata) => metadata.clone(),
            None => return 0,
        };
        let mut adopted = 0;
        for entity in manager.iter_all() {
            if entity.intro2_type8_runtime.is_some() {
                continue;
            }
            if self
                .owners
                .iter()
                .any(|owner| owner.entity_id() == entity.id)
            {
                continue;
            }
            let Ok(owner) = Type8GoToJobSchedulerOwner::adopt_published(entity, &metadata) else {
                continue;
            };
            self.owners
                .push(SpecializedActorTaskOwner::Type8GoToJob(owner));
            adopted += 1;
        }
        adopted
    }

    /// Adopt the authored Level-1 Working Factory as an explicit-arrival owner.
    ///
    /// The owner starts with no pending scientist. Callers must queue a named
    /// Go-To-Job scientist; nearby type-8 allocations are not scanned.
    pub fn adopt_level_one_factory_arrival(&mut self, manager: &EntityManager) -> usize {
        if self
            .owners
            .iter()
            .any(|owner| matches!(owner, SpecializedActorTaskOwner::LevelOneFactoryArrival(_)))
        {
            return 0;
        }
        let Some(owner) = manager
            .iter_all()
            .find_map(|entity| LevelOneFactoryArrivalOwner::adopt_published(entity).ok())
        else {
            return 0;
        };
        self.owners
            .push(SpecializedActorTaskOwner::LevelOneFactoryArrival(owner));
        1
    }

    /// Drop the live Level-1 factory arrival/production owner so abort can
    /// publish the replacement Working Factory Primary on the same allocation.
    pub(crate) fn release_level_one_factory_arrival(
        &mut self,
        factory_id: u32,
    ) -> Option<LevelOneFactoryArrivalOwner> {
        let index = self.owners.iter().position(|owner| {
            matches!(
                owner,
                SpecializedActorTaskOwner::LevelOneFactoryArrival(owner)
                    if owner.entity_id() == factory_id
            )
        })?;
        match self.owners.remove(index) {
            SpecializedActorTaskOwner::LevelOneFactoryArrival(owner) => Some(owner),
            _ => None,
        }
    }

    /// Apply one already-queued factory arrival without ticking production.
    ///
    /// The pair walker uses this after `FUN_00411A80` contact so intake happens
    /// in the pair pass, not on the next factory-task frame.
    pub fn apply_pending_level_one_factory_arrival(
        &mut self,
        manager: &mut EntityManager,
        world_fx: &mut WorldFx,
        notifications: &mut GameplayNotifications,
        retail_tick: u32,
    ) -> Option<LevelOneFactoryArrivalFrame> {
        let owner = self.owners.iter_mut().find_map(|owner| match owner {
            SpecializedActorTaskOwner::LevelOneFactoryArrival(owner) => Some(owner),
            _ => None,
        })?;
        let scientist_id = owner.take_pending_scientist()?;
        let factory_id = owner.entity_id();
        Some(apply_queued_arrival(
            manager,
            factory_id,
            scientist_id,
            world_fx,
            notifications,
            retail_tick,
        ))
    }

    /// Queue one explicit factory/scientist pair. Does not infer arrival from
    /// proximity or from a Go-To-Job tagged result.
    pub fn queue_explicit_level_one_factory_arrival(
        &mut self,
        factory_id: u32,
        scientist_id: u32,
    ) -> Result<(), LevelOneFactoryArrivalQueueError> {
        let Some(owner) = self.owners.iter_mut().find_map(|owner| match owner {
            SpecializedActorTaskOwner::LevelOneFactoryArrival(owner)
                if owner.entity_id() == factory_id =>
            {
                Some(owner)
            }
            _ => None,
        }) else {
            return Err(LevelOneFactoryArrivalQueueError::FactoryOwnerUnavailable { factory_id });
        };
        owner.queue_explicit_scientist(scientist_id);
        Ok(())
    }

    /// Apply an actual pair-pass intake while the factory's task owner remains
    /// in this scheduler. Native production stays with its existing Type66 owner.
    pub fn apply_factory_pair_arrival(
        &mut self,
        manager: &mut EntityManager,
        world_fx: &mut WorldFx,
        notifications: &mut GameplayNotifications,
        factory_id: u32,
        scientist_id: u32,
        retail_tick: u32,
    ) -> Option<LevelOneFactoryArrivalFrame> {
        let owns_factory = self.owners.iter().any(|owner| match owner {
            SpecializedActorTaskOwner::Intro2Type66(owner) => {
                owner.entity_id() == factory_id && owner.completed_pair_boundary(manager)
            }
            SpecializedActorTaskOwner::LevelOneFactoryArrival(owner) => {
                owner.entity_id() == factory_id
            }
            _ => false,
        });
        let native_worker = manager
            .iter_all()
            .find(|entity| entity.id == scientist_id)
            .is_some_and(|entity| entity.intro2_type8_runtime.is_some());
        if native_worker && !self.begin_intro2_type8_external_mutation(manager, scientist_id) {
            return None;
        }
        owns_factory.then(|| {
            apply_queued_arrival(
                manager,
                factory_id,
                scientist_id,
                world_fx,
                notifications,
                retail_tick,
            )
        })
    }

    /// Adopt Intro2 spawn0 and every ordinary Type13 receipt once.
    /// Birth is class-7 B6C0 or class-5 ACD0; a later C690 graph stays in the
    /// same owner. Direct `--level 50` and unpublished type 13 stay out.
    pub fn adopt_intro2_type13_search_attack(&mut self, manager: &EntityManager) -> usize {
        let owners = manager
            .iter_all()
            .filter(|entity| {
                entity.entity_type == crate::intro2_type13_live::TYPE13_ENTITY_TYPE
                    && !self
                        .owners
                        .iter()
                        .any(|owner| owner.entity_id() == entity.id)
            })
            .filter_map(|entity| Intro2Type13WorldOwner::adopt_entity(manager, entity.id).ok())
            .collect::<Vec<_>>();
        let adopted = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::Intro2Type13SearchAttack),
        );
        adopted
    }

    /// Adopt the actual native Intro2 Type26 class4/26/33 graphs once.
    pub fn adopt_intro2_type26(&mut self, manager: &EntityManager) -> usize {
        let owners = manager
            .iter_all()
            .filter(|entity| {
                !self
                    .owners
                    .iter()
                    .any(|owner| owner.entity_id() == entity.id)
            })
            .filter_map(|entity| Intro2Type26WorldOwner::adopt(manager, entity.id).ok())
            .collect::<Vec<_>>();
        let adopted = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::Intro2Type26),
        );
        adopted
    }

    /// Adopt completed shared Type47 graphs and explicit retained Intro2 replay.
    pub fn adopt_intro2_type47_guards(&mut self, manager: &EntityManager) -> usize {
        let owners = manager
            .iter_all()
            .filter(|entity| {
                !self
                    .owners
                    .iter()
                    .any(|owner| owner.entity_id() == entity.id)
            })
            .filter_map(|entity| Intro2Type47WorldOwner::adopt(manager, entity.id).ok())
            .collect::<Vec<_>>();
        let adopted = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::Intro2Type47Scheduler),
        );
        adopted
    }

    /// Adopt the captured Intro2 live flyer movers (spawns 44 and 46) if
    /// published.
    pub fn adopt_intro2_flyers(&mut self, manager: &EntityManager) -> usize {
        let owners = manager
            .iter_all()
            .filter_map(|entity| Intro2FlyerSchedulerOwner::adopt_published(entity).ok())
            .collect::<Vec<_>>();
        let adopted = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::Intro2FlyerScheduler),
        );
        adopted
    }

    /// Transfer each authenticated native peasant's initial graph once into
    /// the shared Type9 world/task owners. Capture-specific scheduler fields
    /// are not written by this handoff.
    pub fn adopt_intro2_type9(&mut self, manager: &mut EntityManager) -> usize {
        let ids: Vec<_> = manager
            .iter_all()
            .filter(|entity| crate::intro2_type9::intro2_type9_allocation_authenticates(entity))
            .map(|entity| entity.id)
            .filter(|id| !self.owners.iter().any(|owner| owner.entity_id() == *id))
            .collect();
        let mut adopted = 0;
        for id in ids {
            if manager.ordinary_type9_selected_actor_lease(id).is_none() {
                continue;
            }
            let Some(authority) = manager
                .entity_mut(id)
                .and_then(crate::intro2_type9::take_intro2_type9_task_authority)
            else {
                continue;
            };
            self.adopt_type9_current_task(manager, authority)
                .expect("authenticated native birth retains its graph during transfer");
            adopted += 1;
        }
        adopted
    }

    pub fn adopt_intro2_type8(&mut self, manager: &mut EntityManager) -> usize {
        let ids: Vec<_> = manager
            .iter_all()
            .map(|entity| entity.id)
            .filter(|id| !self.owners.iter().any(|owner| owner.entity_id() == *id))
            .collect();
        let mut count = 0;
        for id in ids {
            if !crate::intro2_type8::intro2_type8_manager_allocation_authenticates(manager, id) {
                continue;
            }
            if let Some(owner) = manager
                .entity_mut(id)
                .and_then(crate::intro2_type8::Intro2Type8Owner::take_birth)
            {
                self.owners
                    .push(SpecializedActorTaskOwner::Intro2Type8(owner));
                count += 1;
            }
        }
        count
    }

    pub(crate) fn begin_intro2_type8_external_mutation(
        &mut self,
        manager: &EntityManager,
        id: u32,
    ) -> bool {
        self.owners.iter().any(|owner| {
            matches!(owner,SpecializedActorTaskOwner::Intro2Type8(owner)
            if owner.entity_id()==id && owner.completed_hit_boundary(manager))
        })
    }
    pub(crate) fn register_intro2_type8(&mut self, owner: crate::intro2_type8::Intro2Type8Owner) {
        if self.has_native_contact_prefix(owner.entity_id()) {
            return;
        }
        self.owners
            .retain(|current| current.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::Intro2Type8(owner));
    }
    pub(crate) fn park_intro2_type8_external_prefix(&mut self, id: u32) {
        for owner in &mut self.owners {
            if let SpecializedActorTaskOwner::Intro2Type8(owner) = owner {
                if owner.entity_id() == id {
                    owner.park_external_prefix();
                }
            }
        }
    }

    pub fn adopt_intro2_type53(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| {
                crate::intro2_type53::Intro2Type53Owner::adopt(manager, entity.id).ok()
            })
            .filter(|owner| {
                !self
                    .owners
                    .iter()
                    .any(|present| present.entity_id() == owner.entity_id())
            })
            .collect();
        let count = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::Intro2Type53),
        );
        count
    }
    pub fn adopt_type122(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| {
                crate::native_type122::Type122Owner::adopt(manager, entity.id).ok()
            })
            .filter(|owner| {
                !self
                    .owners
                    .iter()
                    .any(|present| present.entity_id() == owner.entity_id())
            })
            .collect();
        let count = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::NativeType122),
        );
        count
    }
    pub fn adopt_native_type123(&mut self, manager: &mut EntityManager) -> usize {
        let ids: Vec<_> = manager
            .iter_all()
            .map(|entity| entity.id)
            .filter(|id| !self.owners.iter().any(|owner| owner.entity_id() == *id))
            .collect();
        let mut count = 0;
        for id in ids {
            if !crate::native_type123::native_type123_manager_allocation_authenticates(manager, id)
            {
                continue;
            }
            if let Some(owner) = manager
                .entity_mut(id)
                .and_then(crate::native_type123::Type123Owner::take_birth)
            {
                self.owners
                    .push(SpecializedActorTaskOwner::NativeType123(owner));
                count += 1;
            }
        }
        count
    }
    pub(crate) fn begin_native_type123_external_mutation(
        &mut self,
        manager: &EntityManager,
        id: u32,
    ) -> bool {
        self.owners.iter().any(|owner| {
            matches!(owner,SpecializedActorTaskOwner::NativeType123(owner)
            if owner.entity_id()==id && owner.completed_hit_boundary(manager))
        })
    }
    pub(crate) fn register_native_type123(&mut self, owner: crate::native_type123::Type123Owner) {
        if self.has_native_contact_prefix(owner.entity_id()) {
            return;
        }
        self.owners
            .retain(|current| current.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::NativeType123(owner));
    }
    pub(crate) fn park_native_type123_external_prefix(&mut self, id: u32) {
        for owner in &mut self.owners {
            if let SpecializedActorTaskOwner::NativeType123(owner) = owner {
                if owner.entity_id() == id {
                    owner.park_external_prefix();
                }
            }
        }
    }
    pub fn adopt_native_type86(&mut self, manager: &mut EntityManager) -> usize {
        let ids: Vec<_> = manager
            .iter_all()
            .map(|entity| entity.id)
            .filter(|id| !self.owners.iter().any(|owner| owner.entity_id() == *id))
            .collect();
        let mut count = 0;
        for id in ids {
            if !crate::native_type86::native_type86_manager_allocation_authenticates(manager, id) {
                continue;
            }
            if let Some(owner) = manager
                .entity_mut(id)
                .and_then(crate::native_type86::Type86Owner::take_birth)
            {
                self.owners
                    .push(SpecializedActorTaskOwner::NativeType86(owner));
                count += 1;
            }
        }
        count
    }
    pub(crate) fn begin_native_type86_external_mutation(
        &mut self,
        manager: &EntityManager,
        id: u32,
    ) -> bool {
        self.owners.iter().any(|owner| {
            matches!(owner,SpecializedActorTaskOwner::NativeType86(owner)
            if owner.entity_id()==id && owner.completed_hit_boundary(manager))
        })
    }
    pub(crate) fn register_native_type86(&mut self, owner: crate::native_type86::Type86Owner) {
        if self.has_native_contact_prefix(owner.entity_id()) {
            return;
        }
        self.owners
            .retain(|current| current.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::NativeType86(owner));
    }
    pub(crate) fn park_native_type86_external_prefix(&mut self, id: u32) {
        for owner in &mut self.owners {
            if let SpecializedActorTaskOwner::NativeType86(owner) = owner {
                if owner.entity_id() == id {
                    owner.park_external_prefix();
                }
            }
        }
    }
    pub fn adopt_shared_fish(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| {
                crate::shared_fish::SharedFishOwner::adopt(manager, entity.id).ok()
            })
            .filter(|owner| {
                !self
                    .owners
                    .iter()
                    .any(|present| present.entity_id() == owner.entity_id())
            })
            .collect();
        let count = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::SharedFish),
        );
        count
    }
    pub(crate) fn shared_fish_completed_owner(
        &self,
        manager: &EntityManager,
        entity_id: u32,
    ) -> bool {
        !self.has_native_contact_prefix(entity_id)
            && self.owners.iter().any(|owner| {
                matches!(owner, SpecializedActorTaskOwner::SharedFish(owner)
                    if owner.entity_id() == entity_id
                        && owner.completed_mutation_boundary(manager))
            })
    }
    pub(crate) fn register_shared_fish(&mut self, owner: crate::shared_fish::SharedFishOwner) {
        if self.has_native_contact_prefix(owner.entity_id()) {
            return;
        }
        if let Some(current) = self
            .owners
            .iter_mut()
            .find(|current| current.entity_id() == owner.entity_id())
        {
            *current = SpecializedActorTaskOwner::SharedFish(owner);
        } else {
            self.owners
                .push(SpecializedActorTaskOwner::SharedFish(owner));
        }
    }
    pub(crate) fn retire_shared_fish(&mut self, entity_id: u32) {
        self.owners.retain(|owner| {
            !matches!(owner, SpecializedActorTaskOwner::SharedFish(owner)
                if owner.entity_id() == entity_id)
        });
    }
    pub(crate) fn begin_cleansing_vehicle_external_mutation(
        &self,
        manager: &EntityManager,
        id: u32,
    ) -> bool {
        self.owners.iter().any(|owner| {
            matches!(owner,
            SpecializedActorTaskOwner::CleansingVehicle(owner)
            if owner.entity_id() == id && owner.completed_mutation_boundary(manager))
        })
    }
    pub(crate) fn park_cleansing_vehicle_external_prefix(&mut self, id: u32) {
        for owner in &mut self.owners {
            if let SpecializedActorTaskOwner::CleansingVehicle(owner) = owner {
                if owner.entity_id() == id {
                    owner.park_external_prefix();
                }
            }
        }
    }
    pub(crate) fn register_cleansing_vehicle(
        &mut self,
        owner: crate::cleansing_vehicle::CleansingVehicleOwner,
    ) {
        if self.has_native_contact_prefix(owner.entity_id()) {
            return;
        }
        self.owners
            .retain(|current| current.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::CleansingVehicle(owner));
    }
    pub fn adopt_cleansing_vehicle(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| {
                crate::cleansing_vehicle::CleansingVehicleOwner::adopt(manager, entity.id).ok()
            })
            .filter(|owner| {
                !self
                    .owners
                    .iter()
                    .any(|present| present.entity_id() == owner.entity_id())
            })
            .collect();
        let count = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::CleansingVehicle),
        );
        count
    }
    pub fn adopt_intro2_type16(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| {
                crate::intro2_type16::Intro2Type16Owner::adopt(manager, entity.id).ok()
            })
            .filter(|owner| {
                !self
                    .owners
                    .iter()
                    .any(|present| present.entity_id() == owner.entity_id())
            })
            .collect();
        let count = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::Intro2Type16),
        );
        count
    }
    pub fn adopt_intro2_type58(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| {
                crate::intro2_type58::Intro2Type58Owner::adopt(manager, entity.id).ok()
            })
            .filter(|owner| {
                !self
                    .owners
                    .iter()
                    .any(|present| present.entity_id() == owner.entity_id())
            })
            .collect();
        let count = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::Intro2Type58),
        );
        count
    }
    pub fn adopt_intro2_type94(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| {
                crate::intro2_type94::Intro2Type94Owner::adopt(manager, entity.id).ok()
            })
            .filter(|owner| {
                !self
                    .owners
                    .iter()
                    .any(|present| present.entity_id() == owner.entity_id())
            })
            .collect();
        let count = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::Intro2Type94),
        );
        count
    }
    pub fn adopt_intro2_type66(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| {
                crate::intro2_type66::Intro2Type66Owner::adopt(manager, entity.id).ok()
            })
            .filter(|owner| {
                !self
                    .owners
                    .iter()
                    .any(|present| present.entity_id() == owner.entity_id())
            })
            .collect();
        let count = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::Intro2Type66),
        );
        count
    }
    pub fn adopt_main_base(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| {
                crate::main_base_runtime::MainBaseOwner::adopt(manager, entity.id).ok()
            })
            .filter(|owner| {
                !self
                    .owners
                    .iter()
                    .any(|present| present.entity_id() == owner.entity_id())
            })
            .collect();
        let count = owners.len();
        self.owners
            .extend(owners.into_iter().map(SpecializedActorTaskOwner::MainBase));
        count
    }
    pub fn adopt_class0_actors(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| crate::entity::Class0ActorOwner::adopt(manager, entity.id).ok())
            .filter(|owner| {
                !self
                    .owners
                    .iter()
                    .any(|present| present.entity_id() == owner.entity_id())
            })
            .collect();
        let count = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::Class0Actor),
        );
        count
    }
    pub(crate) fn prepare_class0_actor_external_mutation(
        &self,
        manager: &EntityManager,
        id: u32,
    ) -> bool {
        let Ok(current) = crate::entity::Class0ActorOwner::adopt(manager, id) else {
            return false;
        };
        self.owners.iter().any(|present|
            matches!(present, SpecializedActorTaskOwner::Class0Actor(owner) if *owner == current))
    }
    pub(crate) fn take_class0_actor_external_mutation(
        &mut self,
        manager: &EntityManager,
        id: u32,
    ) -> bool {
        if !self.prepare_class0_actor_external_mutation(manager, id) {
            return false;
        }
        self.owners.retain(|present| present.entity_id() != id);
        true
    }
    pub(crate) fn register_class0_actor(&mut self, owner: crate::entity::Class0ActorOwner) {
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::Class0Actor(owner));
    }
    pub fn adopt_intro2_type10(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| {
                crate::intro2_type10::Intro2Type10Owner::adopt(manager, entity.id).ok()
            })
            .filter(|owner| {
                !self
                    .owners
                    .iter()
                    .any(|present| present.entity_id() == owner.entity_id())
            })
            .collect();
        let count = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::Intro2Type10),
        );
        count
    }
    pub fn adopt_intro2_type57(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| {
                crate::intro2_type57::Intro2Type57Owner::adopt(manager, entity.id).ok()
            })
            .filter(|owner| {
                !self
                    .owners
                    .iter()
                    .any(|present| present.entity_id() == owner.entity_id())
            })
            .collect();
        let count = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::Intro2Type57),
        );
        count
    }
    pub fn adopt_intro2_gun_turret(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| {
                crate::intro2_gun_turret::Intro2GunTurretOwner::adopt(manager, entity.id).ok()
            })
            .filter(|owner| {
                !self
                    .owners
                    .iter()
                    .any(|present| present.entity_id() == owner.entity_id())
            })
            .collect();
        let count = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::Intro2GunTurret),
        );
        count
    }
    pub fn adopt_intro2_type17(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| {
                crate::intro2_type17::Intro2Type17Owner::adopt(manager, entity.id).ok()
            })
            .filter(|owner| {
                !self
                    .owners
                    .iter()
                    .any(|present| present.entity_id() == owner.entity_id())
            })
            .collect();
        let count = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::Intro2Type17),
        );
        count
    }

    /// A synchronous impact C690 replaces a previously retained Type26 graph.
    pub(crate) fn register_intro2_type26(&mut self, owner: Intro2Type26WorldOwner) {
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::Intro2Type26(owner));
    }

    /// A synchronous impact C690 replaces a previously retained Type53 graph.
    pub(crate) fn register_intro2_type53(
        &mut self,
        owner: crate::intro2_type53::Intro2Type53Owner,
    ) {
        if self.has_native_contact_prefix(owner.entity_id())
            || self.owners.iter().any(|present| {
                matches!(present,
                SpecializedActorTaskOwner::Intro2Type53(current)
                    if current.entity_id() == owner.entity_id() && current.has_pending_prefix())
            })
        {
            return;
        }
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::Intro2Type53(owner));
    }
    pub(crate) fn register_type122(&mut self, owner: crate::native_type122::Type122Owner) {
        if self.has_native_contact_prefix(owner.entity_id())
            || self.owners.iter().any(|present| {
                matches!(present,
                SpecializedActorTaskOwner::NativeType122(current)
                    if current.entity_id() == owner.entity_id() && current.has_pending_prefix())
            })
        {
            return;
        }
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::NativeType122(owner));
    }
    pub(crate) fn intro2_type13_has_pending_prefix(&self, entity_id: u32) -> bool {
        self.has_native_contact_prefix(entity_id)
            || self.owners.iter().any(|owner| {
                matches!(owner, SpecializedActorTaskOwner::Intro2Type13SearchAttack(owner)
                if owner.entity_id() == entity_id && owner.has_pending_prefix())
            })
    }
    pub(crate) fn native_ground_has_pending_prefix(&self, entity_id: u32) -> bool {
        self.has_native_contact_prefix(entity_id)
            || self.owners.iter().any(|owner| match owner {
                SpecializedActorTaskOwner::Intro2Type53(owner) => {
                    owner.entity_id() == entity_id && owner.has_pending_prefix()
                }
                SpecializedActorTaskOwner::NativeType122(owner) => {
                    owner.entity_id() == entity_id && owner.has_pending_prefix()
                }
                SpecializedActorTaskOwner::NativeType30(owner) => {
                    owner.entity_id() == entity_id && owner.has_pending_prefix()
                }
                SpecializedActorTaskOwner::NativeType40(owner) => {
                    owner.entity_id() == entity_id && owner.has_pending_prefix()
                }
                SpecializedActorTaskOwner::NativeType56(owner) => {
                    owner.entity_id() == entity_id && owner.has_pending_prefix()
                }
                SpecializedActorTaskOwner::NativeType38Family(owner) => {
                    owner.entity_id() == entity_id && owner.has_pending_prefix()
                }
                _ => false,
            })
    }
    pub(crate) fn shared_fish_has_pending_prefix(&self, entity_id: u32) -> bool {
        self.has_native_contact_prefix(entity_id)
            || self.owners.iter().any(|owner| match owner {
                SpecializedActorTaskOwner::SharedFish(owner) => {
                    owner.entity_id() == entity_id && owner.has_pending_prefix()
                }
                _ => false,
            })
    }
    pub(crate) fn intro2_type16_has_pending_prefix(&self, entity_id: u32) -> bool {
        self.has_native_contact_prefix(entity_id)
            || self.owners.iter().any(|owner| {
                matches!(owner, SpecializedActorTaskOwner::Intro2Type16(owner)
                if owner.entity_id() == entity_id && owner.has_pending_prefix())
            })
    }
    pub(crate) fn intro2_type58_has_pending_prefix(&self, entity_id: u32) -> bool {
        self.has_native_contact_prefix(entity_id)
            || self.owners.iter().any(|owner| {
                matches!(owner, SpecializedActorTaskOwner::Intro2Type58(owner)
                if owner.entity_id() == entity_id && owner.has_pending_prefix())
            })
    }

    pub(crate) fn park_intro2_type58_external_prefix(&mut self, entity_id: u32) {
        for owner in &mut self.owners {
            if let SpecializedActorTaskOwner::Intro2Type58(owner) = owner {
                if owner.entity_id() == entity_id {
                    owner.park_external_prefix();
                }
            }
        }
    }
    pub(crate) fn intro2_type94_has_pending_prefix(&self, entity_id: u32) -> bool {
        self.has_native_contact_prefix(entity_id)
            || self.owners.iter().any(|owner| {
                matches!(owner, SpecializedActorTaskOwner::Intro2Type94(owner)
                if owner.entity_id() == entity_id && owner.has_pending_prefix())
            })
    }
    pub(crate) fn intro2_type66_has_pending_prefix(&self, entity_id: u32) -> bool {
        self.owners.iter().any(|owner| {
            matches!(owner, SpecializedActorTaskOwner::Intro2Type66(owner)
                if owner.entity_id() == entity_id && owner.has_pending_prefix())
        })
    }
    pub(crate) fn main_base_has_pending_prefix(&self, entity_id: u32) -> bool {
        self.owners.iter().any(|owner| {
            matches!(owner, SpecializedActorTaskOwner::MainBase(owner)
                if owner.entity_id() == entity_id && owner.has_pending_prefix())
        })
    }
    pub(crate) fn intro2_gun_turret_has_pending_prefix(&self, entity_id: u32) -> bool {
        self.owners.iter().any(|owner| {
            matches!(owner, SpecializedActorTaskOwner::Intro2GunTurret(owner)
                if owner.entity_id() == entity_id && owner.has_pending_prefix())
        })
    }

    pub(crate) fn intro2_gun_turret_completed_owner(
        &self,
        manager: &EntityManager,
        entity_id: u32,
    ) -> bool {
        let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
            return false;
        };
        if crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
            .any(|task| {
                entity
                    .actor_tasks
                    .wrapper_flags(task)
                    .is_none_or(|flags| !flags.alive || flags.in_callback)
            })
        {
            return false;
        }
        self.owners.iter().any(|owner| {
            matches!(owner, SpecializedActorTaskOwner::Intro2GunTurret(owner)
                if owner.entity_id() == entity_id && !owner.has_pending_prefix()
                    && crate::intro2_gun_turret::Intro2GunTurretOwner::adopt(manager, entity_id) == Ok(*owner))
        })
    }

    pub(crate) fn park_intro2_gun_turret_external_prefix(&mut self, entity_id: u32) {
        for owner in &mut self.owners {
            if let SpecializedActorTaskOwner::Intro2GunTurret(owner) = owner {
                if owner.entity_id() == entity_id {
                    owner.park_external_prefix();
                }
            }
        }
    }

    pub(crate) fn intro2_type10_has_pending_prefix(&self, entity_id: u32) -> bool {
        self.has_native_contact_prefix(entity_id)
            || self.owners.iter().any(|owner| {
                matches!(owner, SpecializedActorTaskOwner::Intro2Type10(owner)
                if owner.entity_id() == entity_id && owner.has_pending_prefix())
                    || matches!(owner, SpecializedActorTaskOwner::Intro2Type10Tumble(owner)
                    if owner.entity_id() == entity_id && owner.has_pending_prefix())
            })
    }
    pub(crate) fn intro2_type10_tumble_completed_owner(
        &self,
        manager: &EntityManager,
        entity_id: u32,
    ) -> bool {
        if !self.intro2_type10_completed_owner(manager, entity_id) {
            return false;
        }
        let Ok(current) = crate::intro2_type10::Intro2Type10TumbleOwner::adopt(manager, entity_id)
        else {
            return false;
        };
        self.owners.iter().any(|owner| {
            matches!(owner, SpecializedActorTaskOwner::Intro2Type10Tumble(owner) if *owner == current)
        })
    }
    pub(crate) fn intro2_type10_completed_owner(
        &self,
        manager: &EntityManager,
        entity_id: u32,
    ) -> bool {
        let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
            return false;
        };
        if crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
            .any(|task| {
                entity
                    .actor_tasks
                    .wrapper_flags(task)
                    .is_none_or(|flags| !flags.alive || flags.in_callback)
            })
        {
            return false;
        }
        self.owners.iter().any(|owner| match owner {
            SpecializedActorTaskOwner::Intro2Type10(owner) => {
                crate::intro2_type10::Intro2Type10Owner::adopt(manager, entity_id) == Ok(*owner)
            }
            SpecializedActorTaskOwner::Intro2Type10Tumble(owner) => {
                crate::intro2_type10::Intro2Type10TumbleOwner::adopt(manager, entity_id)
                    == Ok(*owner)
            }
            _ => false,
        })
    }

    pub(crate) fn park_intro2_type10_external_prefix(&mut self, entity_id: u32) {
        for owner in &mut self.owners {
            match owner {
                SpecializedActorTaskOwner::Intro2Type10(owner)
                    if owner.entity_id() == entity_id =>
                {
                    owner.park_external_prefix();
                }
                SpecializedActorTaskOwner::Intro2Type10Tumble(owner)
                    if owner.entity_id() == entity_id =>
                {
                    owner.park_contact_prefix();
                }
                _ => {}
            }
        }
    }
    pub(crate) fn park_intro2_type10_tumble_contact_prefix(&mut self, entity_id: u32) {
        for owner in &mut self.owners {
            if let SpecializedActorTaskOwner::Intro2Type10Tumble(owner) = owner {
                if owner.entity_id() == entity_id {
                    owner.park_contact_prefix();
                }
            }
        }
    }

    pub(crate) fn retire_intro2_type10_tumble(
        &mut self,
        completed: crate::intro2_type10::Intro2Type10TumbleOwner,
    ) {
        self.owners.retain(|owner| {
            !matches!(owner, SpecializedActorTaskOwner::Intro2Type10Tumble(owner) if *owner == completed)
        });
    }
    pub(crate) fn intro2_type57_has_pending_prefix(&self, entity_id: u32) -> bool {
        self.has_native_contact_prefix(entity_id)
            || self.owners.iter().any(|owner| {
                matches!(owner, SpecializedActorTaskOwner::Intro2Type57(owner)
                if owner.entity_id() == entity_id && owner.has_pending_prefix())
                    || matches!(owner, SpecializedActorTaskOwner::Intro2Type57Tumble(owner)
                    if owner.entity_id() == entity_id && owner.has_pending_prefix())
            })
    }
    pub(crate) fn intro2_type57_tumble_completed_owner(
        &self,
        manager: &EntityManager,
        entity_id: u32,
    ) -> bool {
        if !self.intro2_type57_completed_owner(manager, entity_id) {
            return false;
        }
        let Ok(current) =
            crate::intro2_type57::death::Intro2Type57TumbleOwner::adopt(manager, entity_id)
        else {
            return false;
        };
        self.owners.iter().any(|owner| {
            matches!(owner, SpecializedActorTaskOwner::Intro2Type57Tumble(owner) if *owner == current)
        })
    }
    pub(crate) fn intro2_type57_completed_owner(
        &self,
        manager: &EntityManager,
        entity_id: u32,
    ) -> bool {
        let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
            return false;
        };
        if crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
            .any(|task| {
                entity
                    .actor_tasks
                    .wrapper_flags(task)
                    .is_none_or(|flags| !flags.alive || flags.in_callback)
            })
        {
            return false;
        }
        self.owners.iter().any(|owner| match owner {
            SpecializedActorTaskOwner::Intro2Type57(owner) => {
                crate::intro2_type57::Intro2Type57Owner::adopt(manager, entity_id) == Ok(*owner)
            }
            SpecializedActorTaskOwner::Intro2Type57Tumble(owner) => {
                crate::intro2_type57::death::Intro2Type57TumbleOwner::adopt(manager, entity_id)
                    == Ok(*owner)
            }
            _ => false,
        })
    }
    pub(crate) fn intro2_type13_completed_owner(
        &self,
        manager: &EntityManager,
        entity_id: u32,
    ) -> bool {
        let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
            return false;
        };
        if crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
            .any(|task| {
                entity
                    .actor_tasks
                    .wrapper_flags(task)
                    .is_none_or(|flags| !flags.alive || flags.in_callback)
            })
        {
            return false;
        }
        self.owners.iter().any(|owner| match owner {
            SpecializedActorTaskOwner::Intro2Type13SearchAttack(owner) => {
                owner.entity_id() == entity_id && owner.completed_contact_boundary(manager)
            }
            _ => false,
        })
    }
    /// Read-only custody for the native people accepted by the Main Base.
    /// Planning transfers nothing; the ordered apply consumes Type9's last
    /// observation before deferred destruction changes the body.
    pub(crate) fn main_base_person_completed_owner(
        &self,
        manager: &EntityManager,
        entity_id: u32,
    ) -> bool {
        if self.has_native_contact_prefix(entity_id) {
            return false;
        }
        if self.ordinary_type9_completed_walking_owner(manager, entity_id) {
            return true;
        }
        self.owners.iter().any(|owner| match owner {
            SpecializedActorTaskOwner::NativeType86(owner) => {
                owner.entity_id() == entity_id && owner.completed_hit_boundary(manager)
            }
            SpecializedActorTaskOwner::NativeType123(owner) => {
                owner.entity_id() == entity_id && owner.completed_hit_boundary(manager)
            }
            _ => false,
        })
    }

    /// Read-only completed-visit custody for ordinary Type9 walking tasks.
    ///
    /// This check transfers nothing. A static writer must subsequently consume
    /// the completed observation before changing motion, even when its damage
    /// packet is zero. Carried, Class14, and unfinished selected owners are not
    /// walking custody.
    pub fn ordinary_type9_completed_walking_owner(
        &self,
        manager: &EntityManager,
        entity_id: u32,
    ) -> bool {
        let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
            return false;
        };
        if entity.entity_type != 9 {
            return false;
        }
        if crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
            .any(|task| {
                entity
                    .actor_tasks
                    .wrapper_flags(task)
                    .is_none_or(|flags| !flags.alive || flags.in_callback)
            })
        {
            return false;
        }
        self.owners.iter().any(|owner| match owner {
            SpecializedActorTaskOwner::OrdinaryType9Wander(owner) => {
                owner.entity_id() == entity_id
                    && owner.completed_visit_lease(manager) == Some(owner.actor_lease())
            }
            SpecializedActorTaskOwner::OrdinaryType9RunAway(owner) => {
                owner.entity_id() == entity_id
                    && owner.completed_visit_lease(manager) == Some(owner.actor_lease())
            }
            SpecializedActorTaskOwner::OrdinaryType9GoToJob(owner) => {
                owner.entity_id() == entity_id
                    && owner.completed_visit_lease(manager) == Some(owner.actor_lease())
            }
            SpecializedActorTaskOwner::OrdinaryType9AttractAttention(owner) => {
                owner.entity_id() == entity_id
                    && owner.completed_visit_lease(manager) == Some(owner.actor_lease())
            }
            _ => false,
        })
    }
    /// Read-only completed-visit custody for Type47 scheduler tasks.
    ///
    /// Covers both the ordinary and Intro2 scheduler owners with live,
    /// out-of-callback task leases. Like the Type9 equivalent this transfers
    /// nothing: static contact replaces no hit graph. CommonDying owners stay
    /// with the death path.
    pub fn type47_completed_scheduler_owner(
        &self,
        manager: &EntityManager,
        entity_id: u32,
    ) -> bool {
        let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
            return false;
        };
        if entity.entity_type != 47 {
            return false;
        }
        if crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
            .any(|task| {
                entity
                    .actor_tasks
                    .wrapper_flags(task)
                    .is_none_or(|flags| !flags.alive || flags.in_callback)
            })
        {
            return false;
        }
        self.owners.iter().any(|owner| match owner {
            SpecializedActorTaskOwner::OrdinaryType47Scheduler(owner) => {
                owner.entity_id() == entity_id
            }
            SpecializedActorTaskOwner::Intro2Type47Scheduler(owner) => {
                owner.entity_id() == entity_id
            }
            _ => false,
        })
    }
    pub(crate) fn park_intro2_type57_external_prefix(&mut self, entity_id: u32) {
        for owner in &mut self.owners {
            match owner {
                SpecializedActorTaskOwner::Intro2Type57(owner)
                    if owner.entity_id() == entity_id =>
                {
                    owner.park_external_prefix();
                }
                SpecializedActorTaskOwner::Intro2Type57Tumble(owner)
                    if owner.entity_id() == entity_id =>
                {
                    owner.park_contact_prefix();
                }
                _ => {}
            }
        }
    }
    /// Attached direct15040 admits two living flyer families outside the old
    /// radial custody matrix. Their completed prefix cannot resume an old task.
    pub(crate) fn park_attached_damage_prefix(&mut self, entity_id: u32) {
        self.park_intro2_type57_external_prefix(entity_id);
        for owner in &mut self.owners {
            if let SpecializedActorTaskOwner::Intro2Type16(owner) = owner {
                if owner.entity_id() == entity_id {
                    owner.park_external_prefix();
                }
            }
        }
    }
    pub(crate) fn park_intro2_type57_tumble_contact_prefix(&mut self, entity_id: u32) {
        for owner in &mut self.owners {
            if let SpecializedActorTaskOwner::Intro2Type57Tumble(owner) = owner {
                if owner.entity_id() == entity_id {
                    owner.park_contact_prefix();
                }
            }
        }
    }
    pub(crate) fn retire_intro2_type57_tumble(
        &mut self,
        completed: crate::intro2_type57::death::Intro2Type57TumbleOwner,
    ) {
        self.owners.retain(|owner| {
            !matches!(owner, SpecializedActorTaskOwner::Intro2Type57Tumble(owner) if *owner == completed)
        });
    }
    pub(crate) fn intro2_flyer_completed_owner(
        &self,
        manager: &EntityManager,
        entity_id: u32,
    ) -> bool {
        if !crate::intro2_flyers_live::flyer_manager_identity_authenticates(manager, entity_id) {
            return false;
        }
        let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
            return false;
        };
        self.owners.iter().any(|owner| {
            matches!(owner, SpecializedActorTaskOwner::Intro2FlyerScheduler(owner)
                if owner.entity_id() == entity_id && owner.matches_completed_entity(entity))
        })
    }

    pub(crate) fn register_intro2_flyer(
        &mut self,
        owner: crate::intro2_flyers_live::Intro2FlyerSchedulerOwner,
    ) {
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::Intro2FlyerScheduler(owner));
    }

    pub(crate) fn park_intro2_flyer_contact_prefix(&mut self, entity_id: u32) {
        for owner in &mut self.owners {
            if let SpecializedActorTaskOwner::Intro2FlyerScheduler(owner) = owner {
                if owner.entity_id() == entity_id {
                    owner.park_contact_prefix();
                }
            }
        }
    }
    /// Inspect the retained living graph in callback fixtures. Production
    /// contacts use the shared living/Class12 mutation-custody entry.
    #[cfg(test)]
    pub(crate) fn intro2_type58_completed_owner(
        &self,
        manager: &EntityManager,
        entity_id: u32,
    ) -> bool {
        let Ok(current) = crate::intro2_type58::Intro2Type58Owner::adopt(manager, entity_id) else {
            return false;
        };
        self.owners.iter().any(|owner| {
            matches!(owner, SpecializedActorTaskOwner::Intro2Type58(owner) if *owner == current)
        })
    }
    pub(crate) fn intro2_type17_has_pending_prefix(&self, entity_id: u32) -> bool {
        self.owners.iter().any(|owner| {
            matches!(owner, SpecializedActorTaskOwner::Intro2Type17(owner)
                if owner.entity_id() == entity_id && owner.has_pending_prefix())
                || matches!(owner, SpecializedActorTaskOwner::Intro2CommonDying(owner)
                    if owner.entity_id() == entity_id && owner.has_pending_prefix())
        })
    }

    pub(crate) fn park_intro2_type17_external_prefix(&mut self, entity_id: u32) {
        park_type17_external_prefix(&mut self.owners, entity_id);
    }

    pub(crate) fn finish_native_type47_external_mutation(
        &mut self,
        manager: &EntityManager,
        id: u32,
    ) -> bool {
        self.owners.iter_mut().any(|owner| match owner {
            SpecializedActorTaskOwner::Intro2Type47Scheduler(owner) if owner.entity_id() == id => {
                owner.finish_external_mutation(manager)
            }
            _ => false,
        })
    }

    pub(crate) fn native_type47_has_pending_prefix(&self, entity_id: u32) -> bool {
        self.owners.iter().any(|owner| match owner {
            SpecializedActorTaskOwner::Intro2Type47Scheduler(owner) => {
                owner.entity_id() == entity_id && owner.has_pending_prefix()
            }
            SpecializedActorTaskOwner::Intro2CommonDying(owner) => {
                owner.entity_id() == entity_id && owner.has_pending_prefix()
            }
            _ => false,
        })
    }

    pub(crate) fn park_native_type47_external_prefix(&mut self, entity_id: u32) {
        park_type47_external_prefix(&mut self.owners, entity_id);
    }

    pub(crate) fn register_intro2_type16(
        &mut self,
        owner: crate::intro2_type16::Intro2Type16Owner,
    ) {
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::Intro2Type16(owner));
    }
    pub(crate) fn register_intro2_type58(
        &mut self,
        owner: crate::intro2_type58::Intro2Type58Owner,
    ) {
        if self.has_native_contact_prefix(owner.entity_id()) {
            return;
        }
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::Intro2Type58(owner));
    }
    pub(crate) fn register_intro2_type94(
        &mut self,
        owner: crate::intro2_type94::Intro2Type94Owner,
    ) {
        if self.has_native_contact_prefix(owner.entity_id()) {
            return;
        }
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::Intro2Type94(owner));
    }
    pub(crate) fn register_intro2_type66(
        &mut self,
        owner: crate::intro2_type66::Intro2Type66Owner,
    ) {
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::Intro2Type66(owner));
    }
    pub(crate) fn register_main_base(&mut self, owner: crate::main_base_runtime::MainBaseOwner) {
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners.push(SpecializedActorTaskOwner::MainBase(owner));
    }
    pub(crate) fn register_intro2_type10(
        &mut self,
        owner: crate::intro2_type10::Intro2Type10Owner,
    ) {
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::Intro2Type10(owner));
    }
    pub(crate) fn register_intro2_gun_turret(
        &mut self,
        owner: crate::intro2_gun_turret::Intro2GunTurretOwner,
    ) {
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::Intro2GunTurret(owner));
    }
    pub(crate) fn register_intro2_type10_tumble(
        &mut self,
        owner: crate::intro2_type10::Intro2Type10TumbleOwner,
    ) {
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::Intro2Type10Tumble(owner));
    }
    pub(crate) fn register_intro2_type13_search_attack(&mut self, owner: Intro2Type13WorldOwner) {
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::Intro2Type13SearchAttack(owner));
    }
    pub(crate) fn register_intro2_type57(
        &mut self,
        owner: crate::intro2_type57::Intro2Type57Owner,
    ) {
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::Intro2Type57(owner));
    }
    pub(crate) fn register_intro2_type57_tumble(
        &mut self,
        owner: crate::intro2_type57::death::Intro2Type57TumbleOwner,
    ) {
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::Intro2Type57Tumble(owner));
    }
    pub(crate) fn register_intro2_type17(
        &mut self,
        owner: crate::intro2_type17::Intro2Type17Owner,
    ) {
        if self.has_native_contact_prefix(owner.entity_id()) {
            return;
        }
        self.owners
            .retain(|present| present.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::Intro2Type17(owner));
    }

    /// Adopt only the constructor-published class-19 Primary/Tertiary pairs.
    pub fn adopt_intro2_meteors(&mut self, manager: &EntityManager) -> usize {
        let owners: Vec<_> = manager
            .iter_all()
            .filter_map(|entity| Intro2MeteorOwner::adopt_published(entity).ok())
            .filter(|owner| {
                !self
                    .owners
                    .iter()
                    .any(|present| present.entity_id() == owner.entity_id())
            })
            .collect();
        let count = owners.len();
        self.owners.extend(
            owners
                .into_iter()
                .map(SpecializedActorTaskOwner::Intro2Meteor),
        );
        count
    }

    /// Particle delivery must retain the exact class-19 task pair owned by
    /// this scheduler; matching a generic Type34 entity is insufficient.
    pub(crate) fn intro2_meteor_owner(&self, entity_id: u32) -> Option<Intro2MeteorOwner> {
        self.owners.iter().find_map(|owner| match owner {
            SpecializedActorTaskOwner::Intro2Meteor(owner) if owner.entity_id() == entity_id => {
                Some(*owner)
            }
            _ => None,
        })
    }

    /// A late contact can finish and clear this exact task pair after 13500.
    /// Retire its scheduler custody immediately; 14990 still owns entity removal.
    pub(crate) fn retire_intro2_meteor(&mut self, owner: Intro2MeteorOwner) {
        self.owners.retain(|present| {
            !matches!(present, SpecializedActorTaskOwner::Intro2Meteor(current) if *current == owner)
        });
    }

    /// Move every authenticated successful fresh-Level-1 Run Away owner from
    /// manager construction custody into this load's live task scheduler.
    ///
    /// Adoption is deliberately restricted to an empty scheduler before the
    /// manager becomes globally visible. Exact actor leases are collected for
    /// the complete batch before any scheduler owner is constructed. If that
    /// preflight cannot observe one actor, every drained sidecar is restored in
    /// original/live order and scheduler custody remains empty.
    pub fn adopt_fresh_level1_type9_run_away(
        &mut self,
        manager: &mut EntityManager,
    ) -> Result<usize, OrdinaryType9RunAwayLoadAdoptionError> {
        if !self.owners.is_empty() {
            return Err(OrdinaryType9RunAwayLoadAdoptionError::SchedulerNotEmpty {
                registered_len: self.owners.len(),
            });
        }

        let sidecars = manager
            .take_fresh_level1_type9_run_away_productions()
            .map_err(|entity_id| {
                OrdinaryType9RunAwayLoadAdoptionError::OwnerAuthenticationFailed { entity_id }
            })?;
        let mut leases = Vec::with_capacity(sidecars.len());
        let unavailable = sidecars.iter().find_map(|(_, owner)| {
            let entity_id = owner.entity_id();
            manager
                .main_base_abort_actor_observation(entity_id)
                .map(|observation| leases.push(observation.lease))
                .is_none()
                .then_some(entity_id)
        });
        if let Some(entity_id) = unavailable {
            restore_fresh_level1_type9_run_away_sidecars(manager, sidecars);
            return Err(OrdinaryType9RunAwayLoadAdoptionError::ActorUnavailable { entity_id });
        }

        let adopted_len = sidecars.len();
        self.owners.extend(sidecars.into_iter().zip(leases).map(
            |((original_index, initial_owner), actor_lease)| {
                SpecializedActorTaskOwner::OrdinaryType9RunAway(
                    OrdinaryType9RunAwayProductionOwner::adopt(
                        original_index,
                        initial_owner,
                        actor_lease,
                    )
                    .expect(
                        "manager extraction and exact lease preflight authenticate Run Away adoption",
                    ),
                )
            },
        ));
        Ok(adopted_len)
    }

    /// Adopt a newly published linear Change-Sea-Level receipt.
    ///
    /// A same-allocation publication replaces the stored receipt and returns
    /// its displaced authority for deliberate disposal/diagnosis. A cross-
    /// family conflict returns the attempted receipt in the failure. Neither
    /// path silently loses a non-copyable authority.
    pub fn register_main_base_type54_sea_level(
        &mut self,
        receipt: MainBaseType54SeaLevelTickReceipt,
    ) -> Result<
        Option<MainBaseType54SeaLevelTickReceipt>,
        SpecializedActorTaskRegistrationFailure<MainBaseType54SeaLevelTickReceipt>,
    > {
        self.register(SpecializedActorTaskOwner::MainBaseType54SeaLevel(receipt))
            .map(|displaced| {
                displaced.map(|displaced| {
                    let SpecializedActorTaskOwner::MainBaseType54SeaLevel(displaced) = displaced
                    else {
                        unreachable!("same-family replacement preserves its family")
                    };
                    displaced
                })
            })
            .map_err(|(conflict, rejected)| {
                let SpecializedActorTaskOwner::MainBaseType54SeaLevel(rejected) = rejected else {
                    unreachable!("registration failure preserves its attempted family")
                };
                SpecializedActorTaskRegistrationFailure { conflict, rejected }
            })
    }

    /// Adopt a published Main Base Type-9 Exploding Person owner.
    ///
    /// Same-family replacement returns the displaced non-copyable authority;
    /// cross-family rejection returns the attempted authority in the failure.
    pub fn register_main_base_type9_exploding(
        &mut self,
        task_lease: MainBaseType9ExplodingTaskLease,
    ) -> Result<
        Option<MainBaseType9ActorProductionOwner>,
        SpecializedActorTaskRegistrationFailure<MainBaseType9ActorProductionOwner>,
    > {
        let owner = MainBaseType9ActorProductionOwner::adopt(task_lease);
        self.register(SpecializedActorTaskOwner::MainBaseType9Exploding(owner))
            .map(|displaced| {
                displaced.map(|displaced| {
                    let SpecializedActorTaskOwner::MainBaseType9Exploding(displaced) = displaced
                    else {
                        unreachable!("same-family replacement preserves its family")
                    };
                    displaced
                })
            })
            .map_err(|(conflict, rejected)| {
                let SpecializedActorTaskOwner::MainBaseType9Exploding(rejected) = rejected else {
                    unreachable!("registration failure preserves its attempted family")
                };
                SpecializedActorTaskRegistrationFailure { conflict, rejected }
            })
    }

    /// Adopt a published Main Base Type-66 progressive Working Factory owner.
    ///
    /// Same-family replacement returns the displaced non-copyable authority;
    /// cross-family rejection returns the attempted authority in the failure.
    pub fn register_main_base_type66_production(
        &mut self,
        task_lease: MainBaseType66WorkingFactoryTaskLease,
    ) -> Result<
        Option<MainBaseType66ProductionOwner>,
        SpecializedActorTaskRegistrationFailure<MainBaseType66ProductionOwner>,
    > {
        let owner = MainBaseType66ProductionOwner::adopt(task_lease);
        self.register(SpecializedActorTaskOwner::MainBaseType66Production(owner))
            .map(|displaced| {
                displaced.map(|displaced| {
                    let SpecializedActorTaskOwner::MainBaseType66Production(displaced) = displaced
                    else {
                        unreachable!("same-family replacement preserves its family")
                    };
                    displaced
                })
            })
            .map_err(|(conflict, rejected)| {
                let SpecializedActorTaskOwner::MainBaseType66Production(rejected) = rejected else {
                    unreachable!("registration failure preserves its attempted family")
                };
                SpecializedActorTaskRegistrationFailure { conflict, rejected }
            })
    }

    /// Adopt one manager-transferred selected ordinary Type-9 Run Away owner.
    ///
    /// Same-family replacement returns the displaced non-copyable authority;
    /// cross-family rejection returns the attempted authority in the failure.
    pub fn register_ordinary_type9_run_away(
        &mut self,
        owner: OrdinaryType9RunAwayProductionOwner,
    ) -> Result<
        Option<OrdinaryType9RunAwayProductionOwner>,
        SpecializedActorTaskRegistrationFailure<OrdinaryType9RunAwayProductionOwner>,
    > {
        self.register(SpecializedActorTaskOwner::OrdinaryType9RunAway(owner))
            .map(|displaced| {
                displaced.map(|displaced| {
                    let SpecializedActorTaskOwner::OrdinaryType9RunAway(displaced) = displaced
                    else {
                        unreachable!("same-family replacement preserves its family")
                    };
                    displaced
                })
            })
            .map_err(|(conflict, rejected)| {
                let SpecializedActorTaskOwner::OrdinaryType9RunAway(rejected) = rejected else {
                    unreachable!("registration failure preserves its attempted family")
                };
                SpecializedActorTaskRegistrationFailure { conflict, rejected }
            })
    }

    pub fn register_ordinary_type9_go_to_job(
        &mut self,
        owner: OrdinaryType9GoToJobProductionOwner,
    ) -> Result<
        Option<OrdinaryType9GoToJobProductionOwner>,
        SpecializedActorTaskRegistrationFailure<OrdinaryType9GoToJobProductionOwner>,
    > {
        self.register(SpecializedActorTaskOwner::OrdinaryType9GoToJob(owner))
            .map(|displaced| {
                displaced.map(|displaced| {
                    let SpecializedActorTaskOwner::OrdinaryType9GoToJob(displaced) = displaced
                    else {
                        unreachable!("same-family replacement preserves its family")
                    };
                    displaced
                })
            })
            .map_err(|(conflict, rejected)| {
                let SpecializedActorTaskOwner::OrdinaryType9GoToJob(rejected) = rejected else {
                    unreachable!("registration failure preserves its attempted family")
                };
                SpecializedActorTaskRegistrationFailure { conflict, rejected }
            })
    }

    pub fn register_ordinary_type9_attract_attention(
        &mut self,
        owner: OrdinaryType9AttractAttentionProductionOwner,
    ) -> Result<
        Option<OrdinaryType9AttractAttentionProductionOwner>,
        SpecializedActorTaskRegistrationFailure<OrdinaryType9AttractAttentionProductionOwner>,
    > {
        self.register(SpecializedActorTaskOwner::OrdinaryType9AttractAttention(
            owner,
        ))
        .map(|displaced| {
            displaced.map(|displaced| {
                let SpecializedActorTaskOwner::OrdinaryType9AttractAttention(displaced) = displaced
                else {
                    unreachable!("same-family replacement preserves its family")
                };
                displaced
            })
        })
        .map_err(|(conflict, rejected)| {
            let SpecializedActorTaskOwner::OrdinaryType9AttractAttention(rejected) = rejected
            else {
                unreachable!("registration failure preserves its attempted family")
            };
            SpecializedActorTaskRegistrationFailure { conflict, rejected }
        })
    }

    /// Adopt the class-14 exploding task after a selected owner is consumed.
    ///
    /// This is the inner SharedRetarget / `FUN_004032A0` owner plus the
    /// ordinary Type-9 latched-`0x2F` E870 suffix. It does not claim Main
    /// Base post-abort `0x00C64825`.
    /// Retire selected Type-9 Wander/Go-To-Job/Attract/Run Away custody after
    /// projectile `FUN_00410C10` published class 14, then adopt the exploding
    /// Primary. Same pattern as Type-47 C690 class-12 retiring Guard/Wander.
    pub fn adopt_ordinary_type9_class14_after_checked_death(
        &mut self,
        task_lease: MainBaseType9ExplodingTaskLease,
    ) -> Result<
        Option<MainBaseType9ExplodingProductionOwner>,
        SpecializedActorTaskRegistrationFailure<MainBaseType9ExplodingProductionOwner>,
    > {
        let entity_id = task_lease.actor().entity_id;
        self.owners.retain(|existing| {
            !(existing.entity_id() == entity_id
                && matches!(
                    existing,
                    SpecializedActorTaskOwner::OrdinaryType9AttractAttention(_)
                        | SpecializedActorTaskOwner::OrdinaryType9GoToJob(_)
                        | SpecializedActorTaskOwner::OrdinaryType9RunAway(_)
                        | SpecializedActorTaskOwner::OrdinaryType9Wander(_)
                ))
        });
        self.register_ordinary_type9_class14(task_lease)
    }

    pub fn register_ordinary_type9_class14(
        &mut self,
        task_lease: MainBaseType9ExplodingTaskLease,
    ) -> Result<
        Option<MainBaseType9ExplodingProductionOwner>,
        SpecializedActorTaskRegistrationFailure<MainBaseType9ExplodingProductionOwner>,
    > {
        let owner = MainBaseType9ExplodingProductionOwner::adopt(task_lease);
        self.register(SpecializedActorTaskOwner::OrdinaryType9Class14(owner))
            .map(|displaced| {
                displaced.map(|displaced| {
                    let SpecializedActorTaskOwner::OrdinaryType9Class14(displaced) = displaced
                    else {
                        unreachable!("same-family replacement preserves its family")
                    };
                    displaced
                })
            })
            .map_err(|(conflict, rejected)| {
                let SpecializedActorTaskOwner::OrdinaryType9Class14(rejected) = rejected else {
                    unreachable!("registration failure preserves its attempted family")
                };
                SpecializedActorTaskRegistrationFailure { conflict, rejected }
            })
    }

    pub fn register_ordinary_type9_wander(
        &mut self,
        owner: OrdinaryType9WanderProductionOwner,
    ) -> Result<
        Option<OrdinaryType9WanderProductionOwner>,
        SpecializedActorTaskRegistrationFailure<OrdinaryType9WanderProductionOwner>,
    > {
        self.register(SpecializedActorTaskOwner::OrdinaryType9Wander(owner))
            .map(|displaced| {
                displaced.map(|displaced| {
                    let SpecializedActorTaskOwner::OrdinaryType9Wander(displaced) = displaced
                    else {
                        unreachable!("same-family replacement preserves its family")
                    };
                    displaced
                })
            })
            .map_err(|(conflict, rejected)| {
                let SpecializedActorTaskOwner::OrdinaryType9Wander(rejected) = rejected else {
                    unreachable!("registration failure preserves its attempted family")
                };
                SpecializedActorTaskRegistrationFailure { conflict, rejected }
            })
    }

    pub(crate) fn register_ordinary_type9_selected(
        &mut self,
        owner: OrdinaryType9SelectedProductionOwner,
    ) -> Result<
        Option<OrdinaryType9SelectedProductionOwner>,
        SpecializedActorTaskRegistrationFailure<OrdinaryType9SelectedProductionOwner>,
    > {
        match owner {
            OrdinaryType9SelectedProductionOwner::RunAway(owner) => self
                .register_ordinary_type9_run_away(owner)
                .map(|displaced| displaced.map(OrdinaryType9SelectedProductionOwner::RunAway))
                .map_err(|failure| SpecializedActorTaskRegistrationFailure {
                    conflict: failure.conflict,
                    rejected: OrdinaryType9SelectedProductionOwner::RunAway(failure.rejected),
                }),
            OrdinaryType9SelectedProductionOwner::AttractAttention(owner) => self
                .register_ordinary_type9_attract_attention(owner)
                .map(|displaced| {
                    displaced.map(OrdinaryType9SelectedProductionOwner::AttractAttention)
                })
                .map_err(|failure| SpecializedActorTaskRegistrationFailure {
                    conflict: failure.conflict,
                    rejected: OrdinaryType9SelectedProductionOwner::AttractAttention(
                        failure.rejected,
                    ),
                }),
            OrdinaryType9SelectedProductionOwner::GoToJob(owner) => self
                .register_ordinary_type9_go_to_job(owner)
                .map(|displaced| displaced.map(OrdinaryType9SelectedProductionOwner::GoToJob))
                .map_err(|failure| SpecializedActorTaskRegistrationFailure {
                    conflict: failure.conflict,
                    rejected: OrdinaryType9SelectedProductionOwner::GoToJob(failure.rejected),
                }),
            OrdinaryType9SelectedProductionOwner::Wander(owner) => self
                .register_ordinary_type9_wander(owner)
                .map(|displaced| displaced.map(OrdinaryType9SelectedProductionOwner::Wander))
                .map_err(|failure| SpecializedActorTaskRegistrationFailure {
                    conflict: failure.conflict,
                    rejected: OrdinaryType9SelectedProductionOwner::Wander(failure.rejected),
                }),
        }
    }

    /// Transfer any selected ordinary Type-9 production family into the
    /// forked Main Base abort transaction as one linear authority.
    pub(crate) fn take_ordinary_type9_selected_for_main_base_abort(
        &mut self,
        actor: MainBaseAbortActorLease,
    ) -> Result<Option<OrdinaryType9SelectedProductionOwner>, OrdinaryType9SelectedCustodyTakeBlock>
    {
        let Some(index) = self
            .owners
            .iter()
            .position(|owner| owner.entity_id() == actor.entity_id)
        else {
            return Ok(None);
        };
        if self.owners[index].is_native_contact_prefix() {
            return Err(
                OrdinaryType9SelectedCustodyTakeBlock::RootCustodyUnavailable {
                    entity_id: actor.entity_id,
                },
            );
        }
        let actual_family = self.owners[index].family();
        let already_class14 = match &self.owners[index] {
            SpecializedActorTaskOwner::MainBaseType9Exploding(owner) => {
                Some(owner.task_lease().actor())
            }
            SpecializedActorTaskOwner::OrdinaryType9Class14(owner) => {
                Some(owner.task_lease().actor())
            }
            _ => None,
        };
        if let Some(expected) = already_class14 {
            if expected != actor {
                return Err(OrdinaryType9SelectedCustodyTakeBlock::ActorLeaseMismatch {
                    expected,
                    actual: actor,
                });
            }
            return Ok(None);
        }

        let (expected, compatible) = match &self.owners[index] {
            SpecializedActorTaskOwner::OrdinaryType9RunAway(owner) => {
                (owner.actor_lease(), owner.main_base_abort_compatible())
            }
            SpecializedActorTaskOwner::OrdinaryType9AttractAttention(owner) => {
                (owner.actor_lease(), owner.main_base_abort_compatible())
            }
            SpecializedActorTaskOwner::OrdinaryType9GoToJob(owner) => {
                (owner.actor_lease(), owner.main_base_abort_compatible())
            }
            SpecializedActorTaskOwner::OrdinaryType9Wander(owner) => {
                (owner.actor_lease(), owner.main_base_abort_compatible())
            }
            _ => {
                return Err(OrdinaryType9SelectedCustodyTakeBlock::DifferentFamily {
                    entity_id: actor.entity_id,
                    actual: actual_family,
                })
            }
        };
        if expected != actor {
            return Err(OrdinaryType9SelectedCustodyTakeBlock::ActorLeaseMismatch {
                expected,
                actual: actor,
            });
        }
        if !compatible {
            return Err(
                OrdinaryType9SelectedCustodyTakeBlock::RootCustodyUnavailable {
                    entity_id: actor.entity_id,
                },
            );
        }
        Ok(Some(match self.owners.remove(index) {
            SpecializedActorTaskOwner::OrdinaryType9RunAway(owner) => {
                OrdinaryType9SelectedProductionOwner::RunAway(owner)
            }
            SpecializedActorTaskOwner::OrdinaryType9AttractAttention(owner) => {
                OrdinaryType9SelectedProductionOwner::AttractAttention(owner)
            }
            SpecializedActorTaskOwner::OrdinaryType9GoToJob(owner) => {
                OrdinaryType9SelectedProductionOwner::GoToJob(owner)
            }
            SpecializedActorTaskOwner::OrdinaryType9Wander(owner) => {
                OrdinaryType9SelectedProductionOwner::Wander(owner)
            }
            _ => unreachable!("the inspected selected owner cannot change family before removal"),
        }))
    }

    pub(crate) fn restore_ordinary_type9_selected_after_main_base_abort_noop(
        &mut self,
        owner: OrdinaryType9SelectedProductionOwner,
    ) -> Result<
        Option<OrdinaryType9SelectedProductionOwner>,
        SpecializedActorTaskRegistrationFailure<OrdinaryType9SelectedProductionOwner>,
    > {
        self.register_ordinary_type9_selected(owner)
    }

    /// Test-only compatibility seam for the earlier Run Away-specific Main
    /// Base coverage. Production callers transfer branch-neutral selected
    /// custody through `take_ordinary_type9_selected_for_main_base_abort`.
    #[cfg(test)]
    pub(crate) fn take_ordinary_type9_run_away_for_main_base_abort(
        &mut self,
        actor: MainBaseAbortActorLease,
    ) -> Result<Option<OrdinaryType9RunAwayProductionOwner>, OrdinaryType9RunAwayCustodyTakeBlock>
    {
        let Some(index) = self
            .owners
            .iter()
            .position(|owner| owner.entity_id() == actor.entity_id)
        else {
            return Ok(None);
        };
        let actual_family = self.owners[index].family();
        let already_class14 = match &self.owners[index] {
            SpecializedActorTaskOwner::MainBaseType9Exploding(owner) => {
                Some(owner.task_lease().actor())
            }
            SpecializedActorTaskOwner::OrdinaryType9Class14(owner) => {
                Some(owner.task_lease().actor())
            }
            _ => None,
        };
        if let Some(expected) = already_class14 {
            if expected != actor {
                return Err(OrdinaryType9RunAwayCustodyTakeBlock::ActorLeaseMismatch {
                    expected,
                    actual: actor,
                });
            }
            // A preceding Main Base sweep already consumed ordinary Run Away
            // custody and replaced it with Exploding Person. Reaching this
            // allocation again is the generic already-dying no-op, not a
            // cross-family topology conflict.
            return Ok(None);
        }
        let SpecializedActorTaskOwner::OrdinaryType9RunAway(owner) = &self.owners[index] else {
            return Err(OrdinaryType9RunAwayCustodyTakeBlock::DifferentFamily {
                entity_id: actor.entity_id,
                actual: actual_family,
            });
        };
        if owner.actor_lease() != actor {
            return Err(OrdinaryType9RunAwayCustodyTakeBlock::ActorLeaseMismatch {
                expected: owner.actor_lease(),
                actual: actor,
            });
        }
        if !owner.main_base_abort_compatible() {
            return Err(
                OrdinaryType9RunAwayCustodyTakeBlock::RootCustodyUnavailable {
                    entity_id: actor.entity_id,
                },
            );
        }
        let SpecializedActorTaskOwner::OrdinaryType9RunAway(owner) = self.owners.remove(index)
        else {
            unreachable!("the inspected owner cannot change family before removal")
        };
        Ok(Some(owner))
    }

    /// Test-only Run Away compatibility seam for exact no-op restoration.
    #[cfg(test)]
    pub(crate) fn restore_ordinary_type9_run_away_after_main_base_abort_noop(
        &mut self,
        owner: OrdinaryType9RunAwayProductionOwner,
    ) -> Result<
        Option<OrdinaryType9RunAwayProductionOwner>,
        SpecializedActorTaskRegistrationFailure<OrdinaryType9RunAwayProductionOwner>,
    > {
        self.register_ordinary_type9_run_away(owner)
    }

    pub fn register_type17_common_dying(
        &mut self,
        owner: LevelOneType17CommonDyingOwner,
    ) -> Result<(), SpecializedActorTaskRegistrationFailure<LevelOneType17CommonDyingOwner>> {
        self.register(SpecializedActorTaskOwner::Type17CommonDying(owner))
            .map(|_| ())
            .map_err(|(conflict, rejected)| {
                let SpecializedActorTaskOwner::Type17CommonDying(rejected) = rejected else {
                    unreachable!("registration failure preserves its attempted family")
                };
                SpecializedActorTaskRegistrationFailure { conflict, rejected }
            })
    }

    /// A death publication replaces the allocation's previous task custody.
    pub fn register_intro2_common_dying(
        &mut self,
        owner: crate::intro2_common_dying::Intro2CommonDyingOwner,
    ) {
        if self.has_native_contact_prefix(owner.entity_id()) {
            return;
        }
        self.owners
            .retain(|previous| previous.entity_id() != owner.entity_id());
        self.owners
            .push(SpecializedActorTaskOwner::Intro2CommonDying(owner));
    }
    /// A death publication replaces the allocation's previous task custody.
    pub fn register_intro2_type9_class14(
        &mut self,
        owner: crate::intro2_type9_class14::Intro2Type9Class14Owner,
    ) {
        if self.has_native_contact_prefix(owner.entity_id()) {
            return;
        }
        let entity_id = owner.entity_id();
        let replacement = delivered_type9_contact::preserve_retirement(
            &self.owners,
            SpecializedActorTaskOwner::Intro2Type9Class14(owner),
        );
        self.owners
            .retain(|previous| previous.entity_id() != entity_id);
        self.owners.push(replacement);
    }

    /// Contacts run after the mover pass, so all of their published tasks wait
    /// for the next frame. Inline mover deaths use the unvisited live prefix.
    pub fn register_radial_death_publications(
        &mut self,
        publications: &[crate::entity::DynamicRadialDeathPublication],
    ) {
        for publication in publications {
            let replacement = match specialized_radial_death_owner(*publication) {
                SpecializedRadialDeath::Task(owner) => owner,
                SpecializedRadialDeath::Deferred(allocation) => {
                    self.owners
                        .retain(|owner| owner.entity_id() != allocation.entity_id);
                    continue;
                }
            };
            let owner = delivered_type9_contact::preserve_retirement(&self.owners, replacement);
            if self.has_native_contact_prefix(owner.entity_id()) {
                continue;
            }
            self.owners
                .retain(|previous| previous.entity_id() != owner.entity_id());
            self.owners.push(owner);
        }
    }

    pub fn register_type47_common_dying(
        &mut self,
        owner: FreshLevelOneType47CommonDyingOwner,
    ) -> Result<(), SpecializedActorTaskRegistrationFailure<FreshLevelOneType47CommonDyingOwner>>
    {
        // Class-12 publication retires the live Guard/Wander/Pursuing
        // scheduler. C690 dying-bit install uses this same receipt.
        self.owners.retain(|existing| {
            !(matches!(
                existing,
                SpecializedActorTaskOwner::OrdinaryType47Scheduler(_)
                    | SpecializedActorTaskOwner::Intro2Type47Scheduler(_)
            ) && existing.entity_id() == owner.entity_id())
        });
        self.register(SpecializedActorTaskOwner::Type47CommonDying(owner))
            .map(|_| ())
            .map_err(|(conflict, rejected)| {
                let SpecializedActorTaskOwner::Type47CommonDying(rejected) = rejected else {
                    unreachable!("registration failure preserves its attempted family")
                };
                SpecializedActorTaskRegistrationFailure { conflict, rejected }
            })
    }

    /// Adopt a published hard-water Type-60 class-48 Primary.
    ///
    /// The scheduler authenticates hard-water provenance at the first visit;
    /// callers must not register the class-49 tails which are consumed by the
    /// same later Main Base sweep that constructed them.
    pub fn register_type60_exploding_ring(
        &mut self,
        task_lease: Type60ExplodingRingTaskLease,
    ) -> Result<
        Option<Type60ExplodingRingProductionOwner>,
        SpecializedActorTaskRegistrationFailure<Type60ExplodingRingProductionOwner>,
    > {
        let owner = Type60ExplodingRingProductionOwner::adopt(task_lease);
        self.register(SpecializedActorTaskOwner::Type60ExplodingRing(owner))
            .map(|displaced| {
                displaced.map(|displaced| {
                    let SpecializedActorTaskOwner::Type60ExplodingRing(displaced) = displaced
                    else {
                        unreachable!("same-family replacement preserves its family")
                    };
                    displaced
                })
            })
            .map_err(|(conflict, rejected)| {
                let SpecializedActorTaskOwner::Type60ExplodingRing(rejected) = rejected else {
                    unreachable!("registration failure preserves its attempted family")
                };
                SpecializedActorTaskRegistrationFailure { conflict, rejected }
            })
    }

    /// Transfer exact class49 or hard-water Type-60 custody into the forked
    /// Main Base abort transaction.
    ///
    /// Absence is valid for a same-sweep class-49 tail or its task-allocation
    /// fallback. A stale allocation claim or a different family at the same
    /// id is an explicit block and leaves scheduler storage unchanged.
    pub(crate) fn take_type60_exploding_ring_for_main_base_abort(
        &mut self,
        actor: MainBaseAbortActorLease,
    ) -> Result<Option<Type60ExplodingRingProductionOwner>, Type60ExplodingRingCustodyTakeBlock>
    {
        let Some(index) = self
            .owners
            .iter()
            .position(|owner| owner.entity_id() == actor.entity_id)
        else {
            return Ok(None);
        };
        let actual_family = self.owners[index].family();
        let SpecializedActorTaskOwner::Type60ExplodingRing(owner) = &self.owners[index] else {
            return Err(Type60ExplodingRingCustodyTakeBlock::DifferentFamily {
                entity_id: actor.entity_id,
                actual: actual_family,
            });
        };
        if owner.actor_lease() != actor {
            return Err(Type60ExplodingRingCustodyTakeBlock::ActorLeaseMismatch {
                expected: owner.actor_lease(),
                actual: actor,
            });
        }
        let SpecializedActorTaskOwner::Type60ExplodingRing(owner) = self.owners.remove(index)
        else {
            unreachable!("the inspected owner cannot change family before removal")
        };
        Ok(Some(owner))
    }

    /// Return an owner after Main Base's remote/already-dying generic no-op.
    /// This preserves its progressed callback sequence instead of rebuilding
    /// sequence one from the copyable task lease.
    pub(crate) fn restore_type60_exploding_ring_after_main_base_abort_noop(
        &mut self,
        owner: Type60ExplodingRingProductionOwner,
    ) -> Result<
        Option<Type60ExplodingRingProductionOwner>,
        SpecializedActorTaskRegistrationFailure<Type60ExplodingRingProductionOwner>,
    > {
        self.register(SpecializedActorTaskOwner::Type60ExplodingRing(owner))
            .map(|displaced| {
                displaced.map(|displaced| {
                    let SpecializedActorTaskOwner::Type60ExplodingRing(displaced) = displaced
                    else {
                        unreachable!("same-family replacement preserves its family")
                    };
                    displaced
                })
            })
            .map_err(|(conflict, rejected)| {
                let SpecializedActorTaskOwner::Type60ExplodingRing(rejected) = rejected else {
                    unreachable!("registration failure preserves its attempted family")
                };
                SpecializedActorTaskRegistrationFailure { conflict, rejected }
            })
    }

    pub(crate) fn register_type9_carried(
        &mut self,
        owner: Type9CarriedOwner,
    ) -> Result<Option<Type9CarriedOwner>, SpecializedActorTaskRegistrationFailure<Type9CarriedOwner>>
    {
        self.register(SpecializedActorTaskOwner::OrdinaryType9Carried(owner))
            .map(|displaced| {
                displaced.map(|owner| {
                    let SpecializedActorTaskOwner::OrdinaryType9Carried(owner) = owner else {
                        unreachable!("same-family registration preserves its carried receipt")
                    };
                    owner
                })
            })
            .map_err(|(conflict, rejected)| {
                let SpecializedActorTaskOwner::OrdinaryType9Carried(rejected) = rejected else {
                    unreachable!("failed registration preserves its attempted carried receipt")
                };
                SpecializedActorTaskRegistrationFailure { conflict, rejected }
            })
    }

    pub(crate) fn type9_carried(&self, entity_id: u32) -> Option<&Type9CarriedOwner> {
        self.owners.iter().find_map(|owner| match owner {
            SpecializedActorTaskOwner::OrdinaryType9Carried(owner)
                if owner.entity_id() == entity_id =>
            {
                Some(owner)
            }
            _ => None,
        })
    }

    pub(crate) fn take_type9_carried(&mut self, entity_id: u32) -> Option<Type9CarriedOwner> {
        let index = self.owners.iter().position(|owner| matches!(owner,
            SpecializedActorTaskOwner::OrdinaryType9Carried(owner) if owner.entity_id() == entity_id))?;
        let SpecializedActorTaskOwner::OrdinaryType9Carried(owner) = self.owners.remove(index)
        else {
            unreachable!("inspected carried receipt cannot change family")
        };
        Some(owner)
    }

    fn register(
        &mut self,
        owner: SpecializedActorTaskOwner,
    ) -> Result<
        Option<SpecializedActorTaskOwner>,
        (
            SpecializedActorTaskRegistrationConflict,
            SpecializedActorTaskOwner,
        ),
    > {
        let entity_id = owner.entity_id();
        let attempted = owner.family();
        let Some(index) = self
            .owners
            .iter()
            .enumerate()
            .find(|(_, existing)| existing.entity_id() == entity_id)
            .map(|(index, _)| index)
        else {
            self.owners.push(owner);
            return Ok(None);
        };
        let existing = self.owners[index].family();
        if existing != attempted || self.owners[index].is_native_contact_prefix() {
            return Err((
                SpecializedActorTaskRegistrationConflict {
                    entity_id,
                    existing,
                    attempted,
                },
                owner,
            ));
        }
        let displaced = std::mem::replace(&mut self.owners[index], owner);
        Ok(Some(displaced))
    }

    /// Cancel every retained receipt after the owning EntityManager allocation
    /// has already been invalidated by a level/reset transition.
    ///
    /// Calling this against a still-live manager would abandon retained linear
    /// task authority and is therefore deliberately not named as a generic
    /// clear.
    pub fn clear_after_manager_reset(&mut self) {
        self.owners.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.owners.is_empty()
    }

    pub fn registered_len(&self) -> usize {
        self.owners.len()
    }

    pub fn family_for(&self, entity_id: u32) -> Option<SpecializedActorTaskFamily> {
        self.owners
            .iter()
            .find(|owner| owner.entity_id() == entity_id)
            .map(SpecializedActorTaskOwner::family)
    }

    /// Exact current Main Base Type-9 allocation claims whose special Sub-I
    /// animation is advanced inside the Exploding Person task callback.
    ///
    /// Retained for callers which need that one family specifically. The
    /// manager-wide neutral animation pass must instead use
    /// [`Self::actor_animation_claims`].
    pub fn main_base_type9_actor_claims(
        &self,
    ) -> impl Iterator<Item = MainBaseAbortActorLease> + '_ {
        self.owners.iter().filter_map(|owner| match owner {
            SpecializedActorTaskOwner::MainBaseType9Exploding(owner) => {
                Some(owner.task_lease().actor())
            }
            _ => None,
        })
    }

    /// Exact Type-9 and native Type-8 allocation claims whose Sub-I dispatch
    /// belongs to this scheduler.
    ///
    /// Detailed common movers advance Sub-I; coarse common movers deliberately
    /// preserve it. Both decisions exclude the compatibility animation pass.
    /// Carrying None advances Sub-I in either mode. Waiting and blocked visits
    /// also retain their claim, so no fallback can invent an extra callback.
    ///
    /// Claims use exact allocation leases rather than every actor sharing an
    /// id across manager generations. Snapshot this iterator before ticking
    /// the scheduler and retain that entry snapshot through the later pass so
    /// a mode-zero owner which blocks, terminates, or drops cannot receive a
    /// second animation advance in the same retail frame.
    pub fn actor_animation_claims(&self) -> impl Iterator<Item = MainBaseAbortActorLease> + '_ {
        self.owners.iter().filter_map(|owner| match owner {
            SpecializedActorTaskOwner::NativeContactPrefix {
                allocation, family, ..
            } if *family == SpecializedActorTaskFamily::Intro2Type8 => Some(*allocation),
            // Type8's shared 01430 mode0 owns Sub-I; mode1 skips it. Its
            // carrying None callback owns Sub-I in both modes (03250).
            SpecializedActorTaskOwner::Intro2Type8(owner) => Some(owner.allocation()),
            // Type123's shared ABDI mode0 owns Sub-I; mode1 skips it. Its
            // carrying None callback owns Sub-I in both modes (03250).
            SpecializedActorTaskOwner::NativeType123(owner) => Some(owner.allocation()),
            // Type86's shared ABDI mode0 owns Sub-I; mode1 skips it. Its
            // carrying None callback owns Sub-I in both modes (03250).
            SpecializedActorTaskOwner::NativeType86(owner) => Some(owner.allocation()),
            _ => owner.type9_actor_lease(),
        })
    }

    /// Exact current allocation claims whose progressive-death clock is
    /// advanced inside the Type-66 Working Factory task callback. A legacy
    /// manager-wide progression pass must exclude these leases without
    /// suppressing an actor whose id was reused by another manager.
    ///
    /// Snapshot this iterator before ticking the scheduler and retain that
    /// entry snapshot through any later legacy pass. An owner that blocks or
    /// drops during its visit must not fall through to a second owner in the
    /// same retail frame.
    pub fn main_base_type66_actor_claims(
        &self,
    ) -> impl Iterator<Item = MainBaseAbortActorLease> + '_ {
        self.owners.iter().filter_map(|owner| match owner {
            SpecializedActorTaskOwner::MainBaseType66Production(owner) => Some(owner.actor_lease()),
            SpecializedActorTaskOwner::MainBase(owner) => Some(owner.allocation()),
            SpecializedActorTaskOwner::Intro2Type66(owner) => Some(owner.allocation()),
            _ => None,
        })
    }

    /// Publish FUN_00411400's later presentation classification for surviving
    /// native actor receipts, before model culling. The next simulation
    /// visit consumes these flags; the current simulation is already complete.
    pub fn publish_presented_view_detail(
        &mut self,
        manager: &mut EntityManager,
        context: RetailViewDetailContext,
    ) {
        for owner in &mut self.owners {
            if owner.is_native_contact_prefix() {
                continue;
            }
            if let SpecializedActorTaskOwner::Class0Actor(class0) = owner {
                // 11400's 800 gate remains separate from 12DA0's 20000
                // callback gate: a Type68 weight still changes the next
                // scheduler's random-wait policy while its task is disabled.
                // Readoption authenticates allocation and completed custody;
                // a parked prefix must retain its current detail state.
                if crate::entity::Class0ActorOwner::adopt(manager, class0.entity_id())
                    == Ok(*class0)
                {
                    if let Some(entity) = manager.entity_mut(class0.entity_id()) {
                        if entity.active {
                            context.publish(
                                entity.position_raw(),
                                &mut entity.collision.state_flags_at_0x08,
                            );
                        }
                    }
                }
            }
            if let SpecializedActorTaskOwner::Intro2Meteor(meteor) = owner {
                if let Some(entity) = manager.entity_mut(meteor.entity_id()) {
                    if Intro2MeteorOwner::adopt_published(entity).as_ref() == Ok(meteor) {
                        context.publish(
                            entity.position_raw(),
                            &mut entity.collision.state_flags_at_0x08,
                        );
                    }
                }
            }
            if let SpecializedActorTaskOwner::SharedFish(fish) = owner {
                if fish.completed_mutation_boundary(manager) {
                    if let Some(entity) = manager.entity_mut(fish.entity_id()) {
                        context.publish(
                            entity.position_raw(),
                            &mut entity.collision.state_flags_at_0x08,
                        );
                    }
                }
            }
            if let SpecializedActorTaskOwner::CleansingVehicle(vehicle) = owner {
                if vehicle.completed_mutation_boundary(manager) {
                    if let Some(entity) = manager.entity_mut(vehicle.entity_id()) {
                        context.publish(
                            entity.position_raw(),
                            &mut entity.collision.state_flags_at_0x08,
                        );
                    }
                }
            }
            // Retail does not present halfway through FUN_00412DA0. A parked
            // selected-owner transaction keeps its frozen mode and state;
            // presentation may classify only a completed or not-yet-started
            // visit, the same boundary that admits relation callbacks.
            let presented_type9 = match owner {
                SpecializedActorTaskOwner::OrdinaryType9Wander(owner) => {
                    owner.completed_visit_lease(manager)
                }
                SpecializedActorTaskOwner::OrdinaryType9GoToJob(owner) => {
                    owner.completed_visit_lease(manager)
                }
                SpecializedActorTaskOwner::OrdinaryType9RunAway(owner) => {
                    owner.completed_visit_lease(manager)
                }
                SpecializedActorTaskOwner::OrdinaryType9AttractAttention(owner) => {
                    owner.completed_visit_lease(manager)
                }
                // This separate post-abort adapter still authenticates its
                // captured complete state word. Its special animation owns
                // Sub-I independently of the selected common-mover mode.
                SpecializedActorTaskOwner::MainBaseType9Exploding(_) => None,
                _ => owner.type9_actor_lease(),
            };
            if let Some(lease) = presented_type9 {
                if manager.ordinary_type9_selected_actor_lease(lease.entity_id) == Some(lease) {
                    if let Some(entity) =
                        manager.ordinary_type9_selected_entity_mut(lease.entity_id)
                    {
                        if entity.active {
                            let position_raw = entity.position_raw();
                            let previous_state = entity.collision.state_flags_at_0x08;
                            if matches!(
                                context.publish(
                                    position_raw,
                                    &mut entity.collision.state_flags_at_0x08
                                ),
                                RetailRuntimeValue::Known(Some(_))
                            ) {
                                let acknowledged = match owner {
                                    SpecializedActorTaskOwner::OrdinaryType9Wander(owner) => owner
                                        .acknowledge_presented_view_detail(entity, previous_state),
                                    SpecializedActorTaskOwner::OrdinaryType9GoToJob(owner) => owner
                                        .acknowledge_presented_view_detail(entity, previous_state),
                                    SpecializedActorTaskOwner::OrdinaryType9AttractAttention(
                                        owner,
                                    ) => owner
                                        .acknowledge_presented_view_detail(entity, previous_state),
                                    _ => true,
                                };
                                assert!(acknowledged, "presented view detail preserves the admitted Type-9 publication");
                            }
                        }
                    }
                }
            }
            if let SpecializedActorTaskOwner::Type17CommonDying(owner) = owner {
                publish_type17_common_dying_owner_presented_view_detail(manager, *owner, context);
            }
        }
    }

    /// Tick the current intrusive live list with the process-global WorldFx RNG.
    ///
    /// Each callback's successor is read after it returns. A Hive-born tail
    /// registers its receipt immediately and can receive its first visit in
    /// this pass. Stale receipts are diagnosed after all live allocations.
    pub fn tick(
        &mut self,
        manager: &mut EntityManager,
        frame: SpecializedActorTaskProductionFrame<'_>,
        notifications: &mut GameplayNotifications,
    ) -> SpecializedActorTaskProductionPass {
        self.tick_with_random(manager, frame, notifications, |world_fx| {
            u32::from(world_fx.next_shared_retail_random_u16())
        })
    }

    #[cfg(test)]
    /// Scripted oracle for the four legacy detached families. Type-66 always
    /// consumes the concrete `WorldFx` stream and tests containing that family
    /// must use public `tick` when they assert its RNG words.
    pub(crate) fn tick_with_scripted_random(
        &mut self,
        manager: &mut EntityManager,
        frame: SpecializedActorTaskProductionFrame<'_>,
        next_shared_random: &mut impl FnMut() -> u32,
    ) -> SpecializedActorTaskProductionPass {
        let mut notifications = GameplayNotifications::new();
        self.tick_with_random(manager, frame, &mut notifications, |_| next_shared_random())
    }

    fn tick_with_random(
        &mut self,
        manager: &mut EntityManager,
        frame: SpecializedActorTaskProductionFrame<'_>,
        notifications: &mut GameplayNotifications,
        mut next_shared_random: impl FnMut(&mut WorldFx) -> u32,
    ) -> SpecializedActorTaskProductionPass {
        let SpecializedActorTaskProductionFrame {
            mut world,
            mut hive_components,
            resources,
            world_fx,
            static_damage,
            elapsed_micros,
            global_elapsed_micros,
            retail_tick,
            notification_phase,
            main_base_abort_active,
        } = frame;
        if self.owners.is_empty() && hive_components.is_none() {
            return SpecializedActorTaskProductionPass {
                block: None,
                outcomes: Vec::new(),
                surface_bubbles_materialized: 0,
                surface_bubbles_dropped: 0,
                sea_level_changed: false,
                explosion_lights: Vec::new(),
                progressive_death_presentation_requested: false,
                terrain_changed: false,
            };
        }
        let actor_resource_block = if resources.level_terrain().is_none() {
            Some(SpecializedActorTaskProductionBlock::CurrentLevelTerrainUnavailable)
        } else if TerrainCollisionContext::from_current_level_cache(resources).is_none() {
            Some(SpecializedActorTaskProductionBlock::CurrentLevelCollisionContextUnavailable)
        } else {
            None
        };
        if let Some(block) = actor_resource_block {
            let mut pass = blocked_production_pass(block);
            // Terrain-dependent actors keep their old preflight boundary.
            // Independent Hive radial/clocks/CA90 still receive their visit.
            if let Some(context) = hive_components.as_mut() {
                let ids = manager
                    .iter_all()
                    .filter(|entity| entity.authored_radial_emitter.is_some())
                    .map(|entity| entity.id)
                    .collect::<Vec<_>>();
                for entity_id in ids {
                    let hive_pass = hive::tick_hive_component(
                        manager,
                        entity_id,
                        resources,
                        world_fx,
                        context,
                        notifications,
                        elapsed_micros,
                        retail_tick,
                        notification_phase,
                        main_base_abort_active,
                    );
                    pass.terrain_changed |= hive_pass.terrain_changed;
                    self.record_hive_diagnostics(hive_pass.diagnostics);
                    self.owners.extend(
                        hive_pass
                            .newborn_owners
                            .into_iter()
                            .map(SpecializedActorTaskOwner::Intro2FlyerScheduler),
                    );
                }
            }
            return pass;
        }

        let mut pending = std::mem::take(&mut self.owners);
        let mut retained = Vec::with_capacity(pending.len());
        let mut outcomes = Vec::with_capacity(pending.len());
        let mut surface_bubbles_materialized = 0;
        let mut surface_bubbles_dropped = 0;
        let mut sea_level_changed = false;
        let mut explosion_lights = Vec::new();
        let mut progressive_death_presentation_requested = false;
        let mut terrain_changed = false;

        // 13500 reloads current->next after 12DA0 returns, including new tails.
        let mut previous_id = None;
        loop {
            let entity_id = match previous_id {
                None => manager.retail_live_order_ids().next(),
                Some(previous) => match manager.retail_live_successor_id(previous) {
                    Ok(successor) => successor,
                    Err(()) => {
                        eprintln!("actor{previous}: current allocation removed before13500 successor read; remaining callbacks skipped");
                        break;
                    }
                },
            };
            let Some(entity_id) = entity_id else {
                break;
            };
            previous_id = Some(entity_id);
            if let Some(context) = hive_components.as_mut() {
                let hive_pass = hive::tick_hive_component(
                    manager,
                    entity_id,
                    resources,
                    world_fx,
                    context,
                    notifications,
                    elapsed_micros,
                    retail_tick,
                    notification_phase,
                    main_base_abort_active,
                );
                terrain_changed |= hive_pass.terrain_changed;
                self.record_hive_diagnostics(hive_pass.diagnostics);
                pending.extend(
                    hive_pass
                        .newborn_owners
                        .into_iter()
                        .map(SpecializedActorTaskOwner::Intro2FlyerScheduler),
                );
            }
            // External writer hosts receive a fresh suffix at this callback.
            let live_order = manager.retail_live_order_ids().collect::<Vec<_>>();
            let live_index = live_order
                .iter()
                .position(|&id| id == entity_id)
                .expect("current allocation survives the task callback");
            let Some(index) = pending
                .iter()
                .position(|owner| owner.entity_id() == entity_id)
            else {
                continue;
            };
            let owner = pending.remove(index);
            if owner.is_delivered_type9_contact() {
                if owner.delivered_type9_contact_authenticates(manager) {
                    retained.push(owner);
                }
                continue;
            }
            match owner {
                SpecializedActorTaskOwner::DeliveredType9Contact { .. } => {
                    unreachable!("contact-only receipt bypasses task dispatch")
                }
                owner @ SpecializedActorTaskOwner::NativeContactPrefix { .. } => {
                    outcomes.push(owner.native_contact_prefix_outcome());
                    retained.push(owner);
                }
                SpecializedActorTaskOwner::OrdinaryType9Carried(owner) => {
                    let tick = tick_type9_carried_owner(
                        manager,
                        owner,
                        Type9CarriedProductionFrame {
                            resources: &*resources,
                            world_fx: &mut *world_fx,
                            notifications,
                            elapsed_micros,
                            retail_tick,
                        },
                        &mut next_shared_random,
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::OrdinaryType9Carried(owner));
                    }
                    adopt_class14_after_selected_consume(&mut retained, tick.class14_task_lease);
                    outcomes.push(SpecializedActorTaskProductionOutcome::OrdinaryType9Carried(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::MainBaseType54SeaLevel(receipt) => {
                    let sea_before = resources
                        .level_terrain()
                        .expect("pass preflight retains strict Section 10")
                        .header[0];
                    let tick = {
                        let terrain = resources
                            .level_terrain_mut()
                            .expect("pass preflight retains strict Section 10");
                        tick_main_base_type54_sea_level_owner(
                            manager,
                            receipt,
                            terrain,
                            elapsed_micros,
                        )
                    };
                    let sea_after = resources
                        .level_terrain()
                        .expect("Type 54 cannot remove strict Section 10")
                        .header[0];
                    sea_level_changed |= sea_before != sea_after;
                    if let Some(receipt) = tick.retained_receipt {
                        retained.push(SpecializedActorTaskOwner::MainBaseType54SeaLevel(receipt));
                    }
                    outcomes.push(
                        SpecializedActorTaskProductionOutcome::MainBaseType54SeaLevel(tick.outcome),
                    );
                }
                SpecializedActorTaskOwner::MainBaseType9Exploding(owner) => {
                    let tick = {
                        let random = |world_fx: &mut WorldFx| next_shared_random(world_fx);
                        tick_main_base_type9_actor_owner_with_random(
                            manager,
                            owner,
                            MainBaseType9ActorProductionFrame {
                                resources: &*resources,
                                elapsed_micros,
                                global_elapsed_micros,
                                retail_tick,
                            },
                            world_fx,
                            random,
                        )
                    };
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::MainBaseType9Exploding(owner));
                    }
                    surface_bubbles_materialized += tick.effects.surface_bubbles_materialized;
                    surface_bubbles_dropped += tick.effects.surface_bubbles_dropped;
                    outcomes.push(
                        SpecializedActorTaskProductionOutcome::MainBaseType9Exploding(tick.outcome),
                    );
                }
                SpecializedActorTaskOwner::MainBaseType66Production(owner) => {
                    let tick = tick_main_base_type66_production_owner(
                        manager,
                        owner,
                        MainBaseType66ProductionFrame {
                            resources: &*resources,
                            elapsed_micros,
                            retail_tick,
                            main_base_abort_active,
                        },
                        world_fx,
                        notifications,
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::MainBaseType66Production(owner));
                    }
                    explosion_lights.extend(tick.explosion_lights);
                    outcomes.push(
                        SpecializedActorTaskProductionOutcome::MainBaseType66Production(
                            tick.outcome,
                        ),
                    );
                }
                SpecializedActorTaskOwner::OrdinaryType9AttractAttention(owner) => {
                    let candidates = ordinary_type9_attract_attention_candidates(manager);
                    let text_notifications = std::cell::RefCell::new(&mut *notifications);
                    let dispatch_resource_text = |request, tick| {
                        text_notifications
                            .borrow_mut()
                            .queue_attract_attention_resource_text(request, tick as i32)
                            .expect("BA40 dispatches the canonical resource event");
                    };
                    let tick = {
                        let random = |world_fx: &mut WorldFx| next_shared_random(world_fx);
                        tick_ordinary_type9_attract_attention_owner_with_random(
                            manager,
                            owner,
                            OrdinaryType9AttractAttentionProductionFrame {
                                dispatch_resource_text: &dispatch_resource_text,
                                resources: &*resources,
                                retail_tick,
                                elapsed_micros,
                                global_elapsed_micros,
                                candidates_in_intrusive_order: &candidates,
                            },
                            world_fx,
                            random,
                        )
                    };
                    let class14_lease = match &tick.outcome {
                        OrdinaryType9AttractAttentionProductionOutcome::SurfaceLifecycleClass14Published {
                            task_lease,
                            ..
                        } => *task_lease,
                        _ => None,
                    };
                    if let Some(owner) = tick.retained_owner {
                        retained.push(
                            SpecializedActorTaskOwner::OrdinaryType9AttractAttention(owner)
                                .continue_current_type9(manager),
                        );
                    } else {
                        adopt_class14_after_selected_consume(&mut retained, class14_lease);
                    }
                    outcomes.push(
                        SpecializedActorTaskProductionOutcome::OrdinaryType9AttractAttention(
                            tick.outcome,
                        ),
                    );
                }
                SpecializedActorTaskOwner::OrdinaryType9Class14(owner) => {
                    let terrain = resources
                        .level_terrain()
                        .expect("pass preflight retains strict Section 10");
                    let actor_lease = owner.task_lease().actor();
                    let entity_id = owner.entity_id();
                    let prefix = {
                        let mut random = |world_fx: &mut WorldFx| next_shared_random(world_fx);
                        apply_class14_scheduler_prefix(
                            manager,
                            actor_lease,
                            elapsed_micros,
                            world_fx,
                            &mut random,
                        )
                    };
                    let callback_elapsed_us = match prefix {
                        Ok(OrdinaryType9Class14SchedulerPrefix::Waiting) => {
                            retained.push(SpecializedActorTaskOwner::OrdinaryType9Class14(owner));
                            outcomes.push(
                                SpecializedActorTaskProductionOutcome::OrdinaryType9Class14(
                                    MainBaseType9ExplodingProductionOutcome::SchedulerWaiting {
                                        entity_id,
                                    },
                                ),
                            );
                            continue;
                        }
                        Ok(OrdinaryType9Class14SchedulerPrefix::Continue {
                            callback_elapsed_us,
                        }) => callback_elapsed_us,
                        Err(_) => {
                            retained.push(SpecializedActorTaskOwner::OrdinaryType9Class14(owner));
                            outcomes.push(
                                SpecializedActorTaskProductionOutcome::OrdinaryType9Class14(
                                    MainBaseType9ExplodingProductionOutcome::Blocked {
                                        entity_id,
                                        reason: MainBaseType9ExplodingProductionBlock::SchedulerStateUnresolved,
                                    },
                                ),
                            );
                            continue;
                        }
                    };
                    let tick = {
                        let mut random = || next_shared_random(&mut *world_fx);
                        tick_main_base_type9_exploding_owner(
                            manager,
                            owner,
                            MainBaseType9ExplodingProductionFrame {
                                terrain,
                                elapsed_micros: callback_elapsed_us,
                                global_elapsed_micros,
                                scheduler_mode: 0,
                            },
                            &mut random,
                        )
                    };
                    if !matches!(
                        &tick.outcome,
                        MainBaseType9ExplodingProductionOutcome::Blocked { .. }
                            | MainBaseType9ExplodingProductionOutcome::Dropped { .. }
                    ) {
                        // FUN_0040C470 stages deferred destroy, then the same
                        // visit still runs the latched-0x2F E870 suffix.
                        let suffix_id =
                            OrdinaryType9OwnerTransactionId::new(std::num::NonZeroU64::MIN);
                        if let Ok((transaction, expected_b2)) = start_class14_suffix_transaction(
                            manager,
                            actor_lease,
                            suffix_id,
                            resources,
                            callback_elapsed_us,
                        ) {
                            let mut random = |world_fx: &mut WorldFx| next_shared_random(world_fx);
                            let _ = drive_outer_tail_transaction(
                                manager,
                                actor_lease,
                                transaction,
                                retail_tick,
                                expected_b2,
                                TerrainCollisionContext::from_current_level_cache(resources),
                                world_fx,
                                &mut random,
                            );
                        }
                    }
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::OrdinaryType9Class14(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::OrdinaryType9Class14(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::OrdinaryType9GoToJob(owner) => {
                    let text_notifications = std::cell::RefCell::new(&mut *notifications);
                    let dispatch_resource_text = |request, tick| {
                        text_notifications
                            .borrow_mut()
                            .queue_attract_attention_resource_text(request, tick as i32)
                            .expect("BA40 dispatches the canonical resource event");
                    };
                    let tick = {
                        let random = |world_fx: &mut WorldFx| next_shared_random(world_fx);
                        tick_ordinary_type9_go_to_job_owner_with_random(
                            manager,
                            owner,
                            OrdinaryType9GoToJobProductionFrame {
                                dispatch_resource_text: &dispatch_resource_text,
                                resources: &*resources,
                                elapsed_micros,
                                global_elapsed_micros,
                                retail_tick,
                            },
                            world_fx,
                            random,
                        )
                    };
                    let class14_lease = match &tick.outcome {
                        OrdinaryType9GoToJobProductionOutcome::SurfaceLifecycleClass14Published {
                            task_lease,
                            ..
                        } => *task_lease,
                        _ => None,
                    };
                    if let Some(owner) = tick.retained_owner {
                        retained.push(
                            SpecializedActorTaskOwner::OrdinaryType9GoToJob(owner)
                                .continue_current_type9(manager),
                        );
                    } else {
                        adopt_class14_after_selected_consume(&mut retained, class14_lease);
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::OrdinaryType9GoToJob(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::OrdinaryType9RunAway(owner) => {
                    let text_notifications = std::cell::RefCell::new(&mut *notifications);
                    let dispatch_resource_text = |request, tick| {
                        text_notifications
                            .borrow_mut()
                            .queue_attract_attention_resource_text(request, tick as i32)
                            .expect("BA40 dispatches the canonical resource event");
                    };
                    let tick = {
                        let random = |world_fx: &mut WorldFx| next_shared_random(world_fx);
                        tick_ordinary_type9_run_away_owner_with_random(
                            manager,
                            owner,
                            OrdinaryType9RunAwayProductionFrame {
                                dispatch_resource_text: &dispatch_resource_text,
                                resources: &*resources,
                                elapsed_micros,
                                global_elapsed_micros,
                                retail_tick,
                            },
                            world_fx,
                            random,
                        )
                    };
                    let class14_lease = match &tick.outcome {
                        OrdinaryType9RunAwayProductionOutcome::SurfaceLifecycleClass14Published {
                            task_lease,
                            ..
                        } => *task_lease,
                        OrdinaryType9RunAwayProductionOutcome::RootContinuation {
                            outcome, ..
                        } => match outcome.as_ref() {
                            OrdinaryType9WanderProductionOutcome::SurfaceLifecycleClass14Published {
                                task_lease,
                                ..
                            } => *task_lease,
                            _ => None,
                        },
                        _ => None,
                    };
                    if let Some(owner) = tick.replacement_owner {
                        retained.push(
                            SpecializedActorTaskOwner::OrdinaryType9Wander(owner)
                                .continue_current_type9(manager),
                        );
                    } else if let Some(owner) = tick.retained_owner {
                        retained.push(
                            SpecializedActorTaskOwner::OrdinaryType9RunAway(owner)
                                .continue_current_type9(manager),
                        );
                    } else {
                        adopt_class14_after_selected_consume(&mut retained, class14_lease);
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::OrdinaryType9RunAway(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::OrdinaryType9Wander(owner) => {
                    let text_notifications = std::cell::RefCell::new(&mut *notifications);
                    let dispatch_resource_text = |request, tick| {
                        text_notifications
                            .borrow_mut()
                            .queue_attract_attention_resource_text(request, tick as i32)
                            .expect("BA40 dispatches the canonical resource event");
                    };
                    let tick = {
                        let random = |world_fx: &mut WorldFx| next_shared_random(world_fx);
                        tick_ordinary_type9_wander_owner_with_random(
                            manager,
                            owner,
                            OrdinaryType9WanderProductionFrame {
                                dispatch_resource_text: &dispatch_resource_text,
                                resources: &*resources,
                                elapsed_micros,
                                global_elapsed_micros,
                                retail_tick,
                            },
                            world_fx,
                            random,
                        )
                    };
                    let class14_lease = match &tick.outcome {
                        OrdinaryType9WanderProductionOutcome::SurfaceLifecycleClass14Published {
                            task_lease,
                            ..
                        } => *task_lease,
                        _ => None,
                    };
                    if let Some(owner) = tick.retained_owner {
                        retained.push(
                            SpecializedActorTaskOwner::OrdinaryType9Wander(owner)
                                .continue_current_type9(manager),
                        );
                    } else {
                        adopt_class14_after_selected_consume(&mut retained, class14_lease);
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::OrdinaryType9Wander(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::Type17CommonDying(owner) => {
                    let tick = {
                        let terrain = resources
                            .level_terrain()
                            .expect("pass preflight retains strict Section 10");
                        let mut random = || next_shared_random(&mut *world_fx);
                        tick_type17_common_dying_owner(
                            manager,
                            owner,
                            Type17CommonDyingProductionFrame {
                                terrain,
                                elapsed_micros,
                            },
                            &mut random,
                        )
                    };
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Type17CommonDying(owner));
                    }
                    if let Some(request) = tick.surface_bubble {
                        materialize_surface_bubble(
                            resources,
                            world_fx,
                            request,
                            retail_tick,
                            &mut surface_bubbles_materialized,
                            &mut surface_bubbles_dropped,
                        );
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Type17CommonDying(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::Type17FollowBeacons(owner) => {
                    let tick = tick_type17_follow_beacons_scheduler_owner(
                        manager,
                        owner,
                        Type17FollowBeaconsProductionFrame {
                            resources,
                            static_damage,
                            world_fx,
                            elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Type17FollowBeacons(owner));
                    }
                    if let Some(owner) = tick.outcome.published_common_dying() {
                        // This callback just published class 12. It first
                        // ticks on the next live-list pass, like other births.
                        retained.push(SpecializedActorTaskOwner::Type17CommonDying(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Type17FollowBeacons(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::Type8GoToJob(owner) => {
                    let tick = tick_type8_go_to_job_scheduler_owner(
                        manager,
                        owner,
                        world_fx,
                        resources.level_terrain(),
                        elapsed_micros,
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Type8GoToJob(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Type8GoToJob(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::LevelOneFactoryArrival(owner) => {
                    let tick = tick_level_one_factory_arrival_owner(
                        manager,
                        owner,
                        resources
                            .level_terrain()
                            .expect("pass preflight retains strict Section 10"),
                        world_fx,
                        notifications,
                        retail_tick,
                        elapsed_micros,
                        main_base_abort_active,
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::LevelOneFactoryArrival(owner));
                    }
                    outcomes.push(
                        SpecializedActorTaskProductionOutcome::LevelOneFactoryArrival(tick.outcome),
                    );
                }
                SpecializedActorTaskOwner::OrdinaryType47Scheduler(owner) => {
                    let tick = {
                        let mut random = |world_fx: &mut WorldFx| next_shared_random(world_fx);
                        let model_id = manager
                            .iter_all()
                            .find(|entity| entity.id == owner.entity_id())
                            .and_then(|entity| entity.model_index);
                        let model_records = model_id
                            .and_then(|id| resources.global_model(id))
                            .map(|model| model.records.as_slice());
                        tick_ordinary_type47_scheduler_owner(
                            manager,
                            owner,
                            world_fx,
                            resources.level_terrain(),
                            model_records,
                            elapsed_micros,
                            &mut random,
                        )
                    };
                    let replacement_common_dying = tick.outcome.replacement_common_dying_owner();
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::OrdinaryType47Scheduler(owner));
                    }
                    if let Some(owner) = replacement_common_dying {
                        retained.push(SpecializedActorTaskOwner::Type47CommonDying(owner));
                    }
                    outcomes.push(
                        SpecializedActorTaskProductionOutcome::OrdinaryType47Scheduler(
                            tick.outcome,
                        ),
                    );
                }
                SpecializedActorTaskOwner::Intro2Type13SearchAttack(owner) => {
                    let entity_id = owner.entity_id();
                    let active_model_extent_raw = manager
                        .iter_all()
                        .find(|entity| entity.id == entity_id)
                        .and_then(|entity| match entity.collision.active_model_slot() {
                            RetailRuntimeValue::Known(slot) => entity.model_in_slot(slot),
                            RetailRuntimeValue::Unresolved => None,
                        })
                        .and_then(|model_id| resources.global_model(model_id))
                        .map(|model| model.radius);
                    let attached_cargo_mass = manager.attached_mass_for_entity_state(entity_id);
                    let frame = match (
                        resources.level_terrain(),
                        active_model_extent_raw,
                        attached_cargo_mass,
                    ) {
                        (
                            Some(terrain),
                            Some(active_model_extent_raw),
                            RetailRuntimeValue::Known(attached_cargo_mass),
                        ) => Some(Intro2Type13PrimaryFrame {
                            dispatch_mode: crate::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
                            terrain,
                            active_model_extent_raw,
                            attached_cargo_mass,
                            elapsed_micros,
                            global_elapsed_micros,
                            retail_tick,
                        }),
                        _ => None,
                    };
                    let tick = tick_intro2_type13_world_owner_with_random(
                        manager,
                        world_fx,
                        owner,
                        frame,
                        &mut next_shared_random,
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2Type13SearchAttack(owner));
                    }
                    outcomes.push(
                        SpecializedActorTaskProductionOutcome::Intro2Type13SearchAttack(
                            tick.outcome,
                        ),
                    );
                }
                SpecializedActorTaskOwner::Intro2Type26(owner) => {
                    let tick = tick_intro2_type26_world(
                        manager,
                        owner,
                        Intro2Type26WorldFrame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            global_elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.replacement_common_dying_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2CommonDying(owner));
                    } else if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2Type26(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Intro2Type26(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::Intro2Type47Scheduler(owner) => {
                    let tick = {
                        let terrain_collision =
                            TerrainCollisionContext::from_current_level_cache(resources);
                        let model_id = manager
                            .iter_all()
                            .find(|entity| entity.id == owner.entity_id())
                            .and_then(|entity| entity.model_index);
                        let model = model_id.and_then(|id| resources.global_model(id));
                        let waves_enabled = resources
                            .level_desc()
                            .and_then(|level| level.raw_u32(0x84))
                            .map(|value| value != 0);
                        let frame = terrain_collision
                            .zip(model)
                            .zip(resources.global_entity_type(47))
                            .zip(waves_enabled)
                            .map(
                                |(((terrain_collision, model), type_record), waves_enabled)| {
                                    Intro2Type47WorldFrame {
                                        type_record,
                                        terrain_collision,
                                        active_model_extent_raw: model.radius,
                                        elapsed_micros,
                                        global_elapsed_micros,
                                        retail_tick,
                                        waves_enabled,
                                    }
                                },
                            );
                        tick_intro2_type47_world_owner(
                            manager,
                            owner,
                            world_fx,
                            frame,
                            &mut next_shared_random,
                        )
                    };
                    let replacement_common_dying = tick.outcome.replacement_common_dying_owner();
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2Type47Scheduler(owner));
                    }
                    if let Some(owner) = replacement_common_dying {
                        // Native and explicit Intro2 allocations use the same complete
                        // class12 outer lifecycle. Publish custody immediately so a
                        // later hit in this frame sees the replacement graph.
                        match crate::intro2_common_dying::Intro2CommonDyingOwner::adopt(
                            manager,
                            owner.entity_id(),
                        ) {
                            Ok(native) => {
                                retained.push(SpecializedActorTaskOwner::Intro2CommonDying(native))
                            }
                            Err(_) => {
                                retained.push(SpecializedActorTaskOwner::Type47CommonDying(owner))
                            }
                        }
                    }
                    outcomes.push(
                        SpecializedActorTaskProductionOutcome::Intro2Type47Scheduler(tick.outcome),
                    );
                }
                SpecializedActorTaskOwner::Intro2FlyerScheduler(owner) => {
                    let tick = tick_intro2_flyer_scheduler_owner_with_random(
                        manager,
                        owner,
                        Intro2FlyerFrame {
                            resources,
                            elapsed_micros,
                            global_elapsed_micros,
                            retail_tick,
                        },
                        world_fx,
                        &mut next_shared_random,
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2FlyerScheduler(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Intro2FlyerScheduler(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::Intro2Type8(owner) => {
                    let tick = crate::intro2_type8::tick_intro2_type8(
                        manager,
                        owner,
                        crate::intro2_type8::Intro2Type8Frame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            global_elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2Type8(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Intro2Type8(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::Intro2Type53(owner) => {
                    let tick = crate::intro2_type53::tick_intro2_type53(
                        manager,
                        owner,
                        crate::intro2_type53::Intro2Type53Frame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(terminal) = tick.replacement_terminal {
                        native_ground::retain_terminal(&mut retained, terminal);
                    } else if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2Type53(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Intro2Type53(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::NativeType122(owner) => {
                    let tick = crate::native_type122::tick_type122(
                        manager,
                        owner,
                        crate::native_type122::Type122Frame {
                            resources,
                            world_fx,
                            notifications,
                            capture_tasks: &mut Intro2RadialCursorCustody {
                                pending: &mut pending,
                                retained: &mut retained,
                                remaining_live_ids: &live_order[live_index + 1..],
                            },
                            elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(terminal) = tick.replacement_terminal {
                        native_ground::retain_terminal(&mut retained, terminal);
                    } else if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::NativeType122(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::NativeType122(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::NativeType30(owner) => {
                    let tick = crate::native_type30::tick_type30(
                        manager,
                        owner,
                        crate::native_type30::Type30Frame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(terminal) = tick.replacement_terminal {
                        native_ground::retain_terminal(&mut retained, terminal);
                    } else if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::NativeType30(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::NativeType30(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::NativeType40(owner) => {
                    let tick = crate::native_type40::tick_type40(
                        manager,
                        owner,
                        crate::native_type40::Type40Frame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            retail_tick,
                            tasks: &mut Intro2RadialCursorCustody {
                                pending: &mut pending,
                                retained: &mut retained,
                                remaining_live_ids: &live_order[live_index + 1..],
                            },
                            notifications,
                        },
                    );
                    if let Some(terminal) = tick.replacement_terminal {
                        native_ground::retain_terminal(&mut retained, terminal);
                    } else if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::NativeType40(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::NativeType40(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::NativeType56(owner) => {
                    let tick = crate::native_type56::tick_type56(
                        manager,
                        owner,
                        crate::native_type56::Type56Frame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(terminal) = tick.replacement_terminal {
                        native_ground::retain_terminal(&mut retained, terminal);
                    } else if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::NativeType56(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::NativeType56(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::NativeType38Family(owner) => {
                    let tick = crate::native_type38::tick_type38_family(
                        manager,
                        owner,
                        crate::native_type38::Type38Frame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::NativeType38Family(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::NativeType38Family(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::NativeType43(owner) => {
                    let tick = crate::native_type43::tick_type43(
                        manager,
                        owner,
                        crate::native_type43::Type43Frame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::NativeType43(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::NativeType43(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::NativeType123(owner) => {
                    let tick = crate::native_type123::tick_type123(
                        manager,
                        owner,
                        crate::native_type123::Type123Frame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            global_elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::NativeType123(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::NativeType123(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::NativeType86(owner) => {
                    let mut tick = crate::native_type86::tick_type86(
                        manager,
                        owner,
                        crate::native_type86::Type86Frame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            global_elapsed_micros,
                            retail_tick,
                        },
                    );
                    if notifications
                        .drain_attract_attention_receipts(manager, retail_tick as i32)
                        .is_err()
                    {
                        // The completed BA40 graph and its RNG belong to the
                        // callback prefix. Keep that owner parked if transfer
                        // of the retained resource receipt cannot complete.
                        if let Some(owner) = tick.retained_owner.as_mut() {
                            owner.park_external_prefix();
                        }
                        tick.outcome = crate::native_type86::Type86Outcome::Blocked {
                            entity_id: tick.outcome.entity_id(),
                            reason: crate::native_type86::Type86Block::Runtime(
                                "attract notification receipt",
                            ),
                            prefix_committed: true,
                        };
                    }
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::NativeType86(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::NativeType86(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::SharedFish(owner) => {
                    let tick = crate::shared_fish::tick_shared_fish(
                        manager,
                        owner,
                        crate::shared_fish::SharedFishFrame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            global_elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::SharedFish(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::SharedFish(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::CleansingVehicle(owner) => {
                    let tick = crate::cleansing_vehicle::tick_cleansing_vehicle(
                        manager,
                        owner,
                        crate::cleansing_vehicle::CleansingVehicleFrame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            global_elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::CleansingVehicle(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::CleansingVehicle(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::Intro2Type16(owner) => {
                    let tick = crate::intro2_type16::tick_intro2_type16(
                        manager,
                        owner,
                        crate::intro2_type16::Intro2Type16Frame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.replacement_common_dying_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2CommonDying(owner));
                    } else if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2Type16(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Intro2Type16(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::Intro2Type58(owner) => {
                    let tick = crate::intro2_type58::tick_intro2_type58(
                        manager,
                        owner,
                        crate::intro2_type58::Intro2Type58Frame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.replacement_common_dying_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2CommonDying(owner));
                    } else if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2Type58(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Intro2Type58(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::Intro2Type94(owner) => {
                    let tick = crate::intro2_type94::tick_intro2_type94(
                        manager,
                        owner,
                        crate::intro2_type94::Intro2Type94Frame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.replacement_common_dying_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2CommonDying(owner));
                    } else if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2Type94(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Intro2Type94(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::Intro2Type10(owner) => {
                    let tick = crate::intro2_type10::tick_intro2_type10(
                        manager,
                        owner,
                        crate::intro2_type10::Intro2Type10Frame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2Type10(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Intro2Type10(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::Intro2Type57(owner) => {
                    let tick = crate::intro2_type57::tick_intro2_type57(
                        manager,
                        owner,
                        crate::intro2_type57::Intro2Type57Frame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2Type57(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Intro2Type57(
                        tick.outcome,
                    ));
                }

                SpecializedActorTaskOwner::Intro2GunTurret(owner) => {
                    let tick = crate::intro2_gun_turret::tick_intro2_gun_turret(
                        manager,
                        owner,
                        crate::intro2_gun_turret::Intro2GunTurretFrame {
                            resources,
                            world_fx,
                            notifications,
                            notification_phase,
                            elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2GunTurret(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Intro2GunTurret(
                        tick.outcome,
                    ));
                }

                SpecializedActorTaskOwner::Intro2Type10Tumble(owner) => {
                    let tick = crate::intro2_type10::tick_intro2_type10_tumble(
                        manager,
                        owner,
                        crate::intro2_type10::Intro2Type10TumbleFrame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2Type10Tumble(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Intro2Type10Tumble(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::Intro2Type57Tumble(owner) => {
                    let tick = crate::intro2_type57::tick_intro2_type57_tumble(
                        manager,
                        owner,
                        crate::intro2_type57::Intro2Type57TumbleFrame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2Type57Tumble(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Intro2Type57Tumble(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::Intro2Type66(owner) => {
                    let Some(world_style_raw) =
                        resources.level_desc().map(|level| level.world_style)
                    else {
                        let entity_id = owner.entity_id();
                        retained.push(SpecializedActorTaskOwner::Intro2Type66(owner));
                        outcomes.push(SpecializedActorTaskProductionOutcome::Intro2Type66(
                            crate::intro2_type66::Intro2Type66Outcome::Blocked {
                                entity_id,
                                reason: crate::intro2_type66::Intro2Type66Block::Runtime(
                                    "factory world-style descriptor",
                                ),
                                prefix_committed: false,
                            },
                        ));
                        continue;
                    };
                    let tick = crate::intro2_type66::tick_intro2_type66_owner(
                        manager,
                        owner,
                        crate::intro2_type66::Intro2Type66Frame {
                            resources,
                            world_fx,
                            notifications,
                            static_damage,
                            elapsed_micros,
                            retail_tick,
                            world_style_raw,
                            main_base_abort_active,
                        },
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2Type66(owner));
                    }
                    explosion_lights.extend(tick.explosion_lights);
                    progressive_death_presentation_requested |=
                        tick.progressive_death_presentation_requested;
                    terrain_changed |= tick.terrain_changed;
                    outcomes.push(SpecializedActorTaskProductionOutcome::Intro2Type66(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::MainBase(owner) => {
                    let tick = crate::main_base_runtime::tick_main_base_owner(
                        manager,
                        owner,
                        crate::main_base_runtime::MainBaseFrame {
                            resources,
                            world_fx,
                            notifications,
                            elapsed_micros,
                            retail_tick,
                            main_base_abort_active,
                        },
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::MainBase(owner));
                    }
                    explosion_lights.extend(tick.explosion_lights);
                    self.terminal_abort_origins
                        .extend(tick.terminal_abort_origin);
                    progressive_death_presentation_requested |=
                        tick.progressive_death_presentation_requested;
                    outcomes.push(SpecializedActorTaskProductionOutcome::MainBase(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::Class0Actor(owner) => {
                    let tick = crate::entity::tick_class0_actor_owner(
                        manager,
                        owner,
                        crate::entity::Class0ActorFrame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Class0Actor(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Class0Actor(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::Intro2Type17(owner) => {
                    let tick = crate::intro2_type17::tick_intro2_type17(
                        manager,
                        owner,
                        crate::intro2_type17::Intro2Type17Frame {
                            resources,
                            world_fx,
                            notifications,
                            capture_tasks: &mut Intro2RadialCursorCustody {
                                pending: &mut pending,
                                retained: &mut retained,
                                remaining_live_ids: &live_order[live_index + 1..],
                            },
                            elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.replacement_common_dying_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2CommonDying(owner));
                    } else if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2Type17(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Intro2Type17(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::Intro2CommonDying(owner) => {
                    let tick = crate::intro2_common_dying::tick_intro2_common_dying(
                        manager,
                        owner,
                        crate::intro2_common_dying::Intro2CommonDyingFrame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2CommonDying(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Intro2CommonDying(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::Intro2Type9Class14(owner) => {
                    let tick = crate::intro2_type9_class14::tick_intro2_type9_class14(
                        manager,
                        owner,
                        crate::intro2_type9_class14::Intro2Type9Class14Frame {
                            resources,
                            world_fx,
                            elapsed_micros,
                            global_elapsed_micros,
                            retail_tick,
                        },
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2Type9Class14(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Intro2Type9Class14(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::Intro2Meteor(owner) => {
                    let tick = tick_intro2_meteor_with_random(
                        manager,
                        owner,
                        Intro2MeteorFrame {
                            resources: &*resources,
                            world_fx,
                            elapsed_micros,
                            retail_tick,
                        },
                        &mut next_shared_random,
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Intro2Meteor(owner));
                    }
                    let death = if let Intro2MeteorOutcome::Terminal { receipt, .. } = tick.outcome
                    {
                        Some(crate::intro2_radial::complete_intro2_meteor_death(
                            crate::intro2_radial::Intro2RadialFrame {
                                active_terminal_calls: Vec::new(),
                                entities: manager,
                                resources,
                                world_fx,
                                static_damage,
                                notifications,
                                retail_tick,
                                actor_tasks: &mut Intro2RadialCursorCustody {
                                    pending: &mut pending,
                                    retained: &mut retained,
                                    remaining_live_ids: &live_order[live_index + 1..],
                                },
                            },
                            receipt,
                        ))
                    } else {
                        None
                    };
                    outcomes.push(SpecializedActorTaskProductionOutcome::Intro2Meteor {
                        outcome: tick.outcome,
                        death,
                    });
                }
                SpecializedActorTaskOwner::Type47CommonDying(owner) => {
                    let native_intro2 = manager
                        .iter_all()
                        .find(|entity| entity.id == owner.entity_id())
                        .is_some_and(|entity| entity.native_type47_construction.is_some()
                            || crate::intro2_type47_live::intro2_type47_cohort_runtime_authenticates(entity));
                    if native_intro2 {
                        match crate::intro2_common_dying::Intro2CommonDyingOwner::adopt(
                            manager,
                            owner.entity_id(),
                        ) {
                            Ok(native) => {
                                let tick = crate::intro2_common_dying::tick_intro2_common_dying(
                                    manager,
                                    native,
                                    crate::intro2_common_dying::Intro2CommonDyingFrame {
                                        resources,
                                        world_fx,
                                        elapsed_micros,
                                        retail_tick,
                                    },
                                );
                                if let Some(owner) = tick.retained_owner {
                                    retained
                                        .push(SpecializedActorTaskOwner::Intro2CommonDying(owner));
                                }
                                outcomes.push(
                                    SpecializedActorTaskProductionOutcome::Intro2CommonDying(
                                        tick.outcome,
                                    ),
                                );
                            }
                            Err(reason) => {
                                outcomes
                                    .push(SpecializedActorTaskProductionOutcome::Intro2CommonDying(
                                    crate::intro2_common_dying::Intro2CommonDyingOutcome::Blocked {
                                        entity_id: owner.entity_id(),
                                        reason,
                                        prefix_committed: false,
                                    },
                                ));
                                retained.push(SpecializedActorTaskOwner::Type47CommonDying(owner));
                            }
                        }
                        continue;
                    }
                    let tick = {
                        let terrain = resources
                            .level_terrain()
                            .expect("pass preflight retains strict Section 10");
                        let mut random = || next_shared_random(&mut *world_fx);
                        tick_type47_common_dying_owner(
                            manager,
                            owner,
                            Type47CommonDyingProductionFrame {
                                terrain,
                                elapsed_micros,
                            },
                            &mut random,
                        )
                    };
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Type47CommonDying(owner));
                    }
                    if let Some(request) = tick.surface_bubble {
                        materialize_surface_bubble(
                            resources,
                            world_fx,
                            request,
                            retail_tick,
                            &mut surface_bubbles_materialized,
                            &mut surface_bubbles_dropped,
                        );
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Type47CommonDying(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::Type60ExplodingRing(owner) => {
                    let tick = tick_type60_exploding_ring_production_owner(
                        manager,
                        owner,
                        elapsed_micros,
                        world_fx,
                    );
                    if let Some(owner) = tick.retained_owner {
                        retained.push(SpecializedActorTaskOwner::Type60ExplodingRing(owner));
                    }
                    outcomes.push(SpecializedActorTaskProductionOutcome::Type60ExplodingRing(
                        tick.outcome,
                    ));
                }
                SpecializedActorTaskOwner::NativeWeapon(owner) => {
                    let outcome = self.tick_native_weapon_in_pass(
                        owner,
                        native_weapons::NativeWeaponPassVisit {
                            manager,
                            pending: &mut pending,
                            retained: &mut retained,
                            resources,
                            world_fx,
                            static_damage,
                            notifications,
                            world: &mut world,
                            elapsed_micros,
                            retail_tick,
                        },
                    );
                    outcomes.push(SpecializedActorTaskProductionOutcome::NativeWeapon(outcome));
                }
            }
        }

        // A delivered child missing from the live list has already been freed.
        pending.retain(|owner| !owner.is_delivered_type9_contact());
        outcomes.extend(pending.into_iter().map(|owner| match owner {
            SpecializedActorTaskOwner::DeliveredType9Contact { .. } => {
                unreachable!("missing contact-only receipt was discarded")
            }
            owner @ SpecializedActorTaskOwner::NativeContactPrefix { .. } => {
                let outcome = owner.native_contact_prefix_outcome();
                retained.push(owner);
                outcome
            }
            SpecializedActorTaskOwner::OrdinaryType9Carried(owner) => {
                SpecializedActorTaskProductionOutcome::OrdinaryType9Carried(
                    Type9CarriedProductionOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        reason: Type9CarriedProductionDrop::EntityUnavailable,
                    },
                )
            }
            SpecializedActorTaskOwner::MainBaseType54SeaLevel(receipt) => {
                SpecializedActorTaskProductionOutcome::MainBaseType54SeaLevel(
                    MainBaseType54SeaLevelProductionOutcome::Dropped {
                        entity_id: receipt.lease().actor().entity_id,
                        reason: MainBaseType54SeaLevelTickBlock::EntityMissing,
                    },
                )
            }
            SpecializedActorTaskOwner::MainBaseType9Exploding(owner) => {
                SpecializedActorTaskProductionOutcome::MainBaseType9Exploding(
                    MainBaseType9ExplodingProductionOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        reason: MainBaseType9ExplodingProductionDrop::EntityUnavailable,
                    },
                )
            }
            SpecializedActorTaskOwner::MainBaseType66Production(owner) => {
                SpecializedActorTaskProductionOutcome::MainBaseType66Production(
                    MainBaseType66ProductionOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        reason: MainBaseType66ProductionDrop::EntityUnavailable,
                    },
                )
            }
            SpecializedActorTaskOwner::OrdinaryType9GoToJob(owner) => {
                SpecializedActorTaskProductionOutcome::OrdinaryType9GoToJob(
                    OrdinaryType9GoToJobProductionOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        reason: OrdinaryType9GoToJobProductionDrop::EntityUnavailable,
                    },
                )
            }
            SpecializedActorTaskOwner::OrdinaryType9RunAway(owner) => {
                SpecializedActorTaskProductionOutcome::OrdinaryType9RunAway(
                    OrdinaryType9RunAwayProductionOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        reason: OrdinaryType9RunAwayProductionDrop::EntityUnavailable,
                    },
                )
            }
            SpecializedActorTaskOwner::OrdinaryType9AttractAttention(owner) => {
                SpecializedActorTaskProductionOutcome::OrdinaryType9AttractAttention(
                    OrdinaryType9AttractAttentionProductionOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        reason: OrdinaryType9AttractAttentionProductionDrop::EntityUnavailable,
                    },
                )
            }
            SpecializedActorTaskOwner::OrdinaryType9Class14(owner) => {
                SpecializedActorTaskProductionOutcome::OrdinaryType9Class14(
                    MainBaseType9ExplodingProductionOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        reason: MainBaseType9ExplodingProductionDrop::EntityUnavailable,
                    },
                )
            }
            SpecializedActorTaskOwner::OrdinaryType9Wander(owner) => {
                SpecializedActorTaskProductionOutcome::OrdinaryType9Wander(
                    OrdinaryType9WanderProductionOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        reason: OrdinaryType9WanderProductionDrop::EntityUnavailable,
                    },
                )
            }
            SpecializedActorTaskOwner::Type17CommonDying(owner) => {
                SpecializedActorTaskProductionOutcome::Type17CommonDying(
                    Type17CommonDyingProductionOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        reason: Type17CommonDyingProductionDrop::EntityUnavailable,
                    },
                )
            }
            SpecializedActorTaskOwner::Type17FollowBeacons(owner) => {
                SpecializedActorTaskProductionOutcome::Type17FollowBeacons(
                    Type17FollowBeaconsSchedulerProductionOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        reason: Type17FollowBeaconsSchedulerProductionDrop::EntityUnavailable,
                    },
                )
            }
            SpecializedActorTaskOwner::Type8GoToJob(owner) => {
                SpecializedActorTaskProductionOutcome::Type8GoToJob(
                    Type8GoToJobSchedulerProductionOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        reason: Type8GoToJobSchedulerProductionDrop::EntityUnavailable,
                    },
                )
            }
            SpecializedActorTaskOwner::LevelOneFactoryArrival(owner) => {
                SpecializedActorTaskProductionOutcome::LevelOneFactoryArrival(
                    LevelOneFactoryArrivalProductionOutcome::Dropped {
                        factory_id: owner.entity_id(),
                        reason: LevelOneFactoryArrivalProductionDrop::EntityUnavailable,
                    },
                )
            }
            SpecializedActorTaskOwner::Type47CommonDying(owner) => {
                SpecializedActorTaskProductionOutcome::Type47CommonDying(
                    Type47CommonDyingProductionOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        reason: Type47CommonDyingProductionDrop::EntityUnavailable,
                    },
                )
            }
            SpecializedActorTaskOwner::OrdinaryType47Scheduler(owner) => {
                SpecializedActorTaskProductionOutcome::OrdinaryType47Scheduler(
                    OrdinaryType47SchedulerProductionOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        reason: OrdinaryType47SchedulerProductionDrop::EntityUnavailable,
                    },
                )
            }
            SpecializedActorTaskOwner::Intro2Type13SearchAttack(owner) => {
                SpecializedActorTaskProductionOutcome::Intro2Type13SearchAttack(
                    Intro2Type13WorldOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        reason: Intro2Type13WorldDrop::EntityUnavailable,
                    },
                )
            }
            SpecializedActorTaskOwner::Intro2Type26(owner) => {
                SpecializedActorTaskProductionOutcome::Intro2Type26(
                    Intro2Type26WorldOutcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::Intro2Type47Scheduler(owner) => {
                SpecializedActorTaskProductionOutcome::Intro2Type47Scheduler(
                    Intro2Type47WorldOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        reason: Intro2Type47WorldDrop::EntityUnavailable,
                    },
                )
            }
            SpecializedActorTaskOwner::Intro2FlyerScheduler(owner) => {
                SpecializedActorTaskProductionOutcome::Intro2FlyerScheduler(
                    Intro2FlyerSchedulerProductionOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        reason: Intro2FlyerSchedulerProductionDrop::EntityUnavailable,
                    },
                )
            }
            SpecializedActorTaskOwner::Intro2Type8(owner) => {
                SpecializedActorTaskProductionOutcome::Intro2Type8(
                    crate::intro2_type8::Intro2Type8Outcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::Intro2Type53(owner) => {
                SpecializedActorTaskProductionOutcome::Intro2Type53(
                    crate::intro2_type53::Intro2Type53Outcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::NativeType122(owner) => {
                SpecializedActorTaskProductionOutcome::NativeType122(
                    crate::native_type122::Type122Outcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::NativeType30(owner) => {
                SpecializedActorTaskProductionOutcome::NativeType30(
                    crate::native_type30::Type30Outcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::NativeType40(owner) => {
                SpecializedActorTaskProductionOutcome::NativeType40(
                    crate::native_type40::Type40Outcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::NativeType56(owner) => {
                SpecializedActorTaskProductionOutcome::NativeType56(
                    crate::native_type56::Type56Outcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::NativeType43(owner) => {
                SpecializedActorTaskProductionOutcome::NativeType43(
                    crate::native_type43::Type43Outcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::NativeType38Family(owner) => {
                SpecializedActorTaskProductionOutcome::NativeType38Family(
                    crate::native_type38::Type38Outcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::NativeType123(owner) => {
                SpecializedActorTaskProductionOutcome::NativeType123(
                    crate::native_type123::Type123Outcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::NativeType86(owner) => {
                SpecializedActorTaskProductionOutcome::NativeType86(
                    crate::native_type86::Type86Outcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::SharedFish(owner) => {
                SpecializedActorTaskProductionOutcome::SharedFish(
                    crate::shared_fish::SharedFishOutcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::CleansingVehicle(owner) => {
                SpecializedActorTaskProductionOutcome::CleansingVehicle(
                    crate::cleansing_vehicle::CleansingVehicleOutcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::Intro2Type16(owner) => {
                SpecializedActorTaskProductionOutcome::Intro2Type16(
                    crate::intro2_type16::Intro2Type16Outcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::Intro2Type58(owner) => {
                SpecializedActorTaskProductionOutcome::Intro2Type58(
                    crate::intro2_type58::Intro2Type58Outcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::Intro2Type94(owner) => {
                SpecializedActorTaskProductionOutcome::Intro2Type94(
                    crate::intro2_type94::Intro2Type94Outcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::Intro2Type66(owner) => {
                SpecializedActorTaskProductionOutcome::Intro2Type66(
                    crate::intro2_type66::Intro2Type66Outcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::MainBase(owner) => {
                SpecializedActorTaskProductionOutcome::MainBase(
                    crate::main_base_runtime::MainBaseOutcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::Class0Actor(owner) => {
                SpecializedActorTaskProductionOutcome::Class0Actor(
                    crate::entity::Class0ActorOutcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::Intro2Type10(owner) => {
                SpecializedActorTaskProductionOutcome::Intro2Type10(
                    crate::intro2_type10::Intro2Type10Outcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::Intro2Type57(owner) => {
                SpecializedActorTaskProductionOutcome::Intro2Type57(
                    crate::intro2_type57::Intro2Type57Outcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::Intro2GunTurret(owner) => {
                SpecializedActorTaskProductionOutcome::Intro2GunTurret(
                    crate::intro2_gun_turret::Intro2GunTurretOutcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::Intro2Type10Tumble(owner) => {
                SpecializedActorTaskProductionOutcome::Intro2Type10Tumble(
                    crate::intro2_type10::Intro2Type10TumbleOutcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::Intro2Type57Tumble(owner) => {
                SpecializedActorTaskProductionOutcome::Intro2Type57Tumble(
                    crate::intro2_type57::Intro2Type57TumbleOutcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::Intro2Type17(owner) => {
                SpecializedActorTaskProductionOutcome::Intro2Type17(
                    crate::intro2_type17::Intro2Type17Outcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::Intro2CommonDying(owner) => {
                SpecializedActorTaskProductionOutcome::Intro2CommonDying(
                    crate::intro2_common_dying::Intro2CommonDyingOutcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::Intro2Type9Class14(owner) => {
                SpecializedActorTaskProductionOutcome::Intro2Type9Class14(
                    crate::intro2_type9_class14::Intro2Type9Class14Outcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                )
            }
            SpecializedActorTaskOwner::Intro2Meteor(owner) => {
                SpecializedActorTaskProductionOutcome::Intro2Meteor {
                    outcome: Intro2MeteorOutcome::Dropped {
                        entity_id: owner.entity_id(),
                    },
                    death: None,
                }
            }
            SpecializedActorTaskOwner::Type60ExplodingRing(owner) => {
                SpecializedActorTaskProductionOutcome::Type60ExplodingRing(
                    Type60ExplodingRingProductionOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        reason: Type60ExplodingRingProductionDrop::EntityUnavailable,
                    },
                )
            }
            SpecializedActorTaskOwner::NativeWeapon(owner) => {
                SpecializedActorTaskProductionOutcome::NativeWeapon(
                    NativeWeaponProductionOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        kind: owner.kind(),
                    },
                )
            }
        }));
        self.owners = retained;

        SpecializedActorTaskProductionPass {
            block: None,
            outcomes,
            surface_bubbles_materialized,
            surface_bubbles_dropped,
            sea_level_changed,
            explosion_lights,
            progressive_death_presentation_requested,
            terrain_changed,
        }
    }
}

/// Transfer an external writer against the actual storage that owns this
/// allocation. During a live walk that is either `pending` or `retained`;
/// reinserting at the same index never schedules an additional callback.
fn begin_native_type9_external_mutation(
    owners: &mut Vec<SpecializedActorTaskOwner>,
    manager: &EntityManager,
    entity_id: u32,
) -> Option<NativeType9HitCustody> {
    if owners
        .iter()
        .any(|owner| owner.entity_id() == entity_id && owner.is_delivered_type9_contact())
    {
        let authenticated = delivered_type9_contact::authenticated_retained_type9_contact(
            owners, manager, entity_id,
        );
        return delivered_type9_contact::with_retained_type9_contact(
            owners,
            entity_id,
            authenticated,
            None,
            |current| begin_native_type9_external_mutation(current, manager, entity_id),
        );
    }
    let entity = manager.iter_all().find(|entity| entity.id == entity_id)?;
    if entity.ordinary_type9_native_receipt.is_some() {
        if !crate::ordinary_type9_construction::ordinary_type9_native_allocation_authenticates(
            manager, entity_id,
        ) {
            return None;
        }
    } else if !crate::intro2_type9::intro2_type9_allocation_authenticates(entity) {
        return None;
    }
    let index = owners
        .iter()
        .position(|owner| owner.entity_id() == entity_id)?;
    if owners[index].is_native_contact_prefix() {
        return None;
    }
    let actor = owners[index].type9_actor_lease()?;
    if manager.main_base_abort_actor_observation(entity_id)?.lease != actor {
        return None;
    }
    if let SpecializedActorTaskOwner::Intro2Type9Class14(owner) = &owners[index] {
        return owner
            .completed_hit_boundary(manager)
            .then_some(NativeType9HitCustody::Class14);
    }
    if let SpecializedActorTaskOwner::OrdinaryType9Class14(owner) = &owners[index] {
        return owner
            .completed_hit_boundary(manager)
            .then_some(NativeType9HitCustody::Class14);
    }
    if let SpecializedActorTaskOwner::OrdinaryType9Carried(owner) = &owners[index] {
        return (owner.authenticates(entity)
            && entity.actor_tasks.wrapper_flags(owner.visit.task_id)
                == Some(crate::actor_task_owner::ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: false,
                }))
        .then_some(NativeType9HitCustody::Carried);
    }
    let previous = owners.remove(index);
    let transfer = match previous {
        SpecializedActorTaskOwner::OrdinaryType9Wander(owner) => owner
            .into_completed_native_hit(manager)
            .map_err(SpecializedActorTaskOwner::OrdinaryType9Wander),
        SpecializedActorTaskOwner::OrdinaryType9RunAway(owner) => owner
            .into_completed_native_hit(manager)
            .map_err(SpecializedActorTaskOwner::OrdinaryType9RunAway),
        SpecializedActorTaskOwner::OrdinaryType9GoToJob(owner) => owner
            .into_completed_native_hit(manager)
            .map_err(SpecializedActorTaskOwner::OrdinaryType9GoToJob),
        SpecializedActorTaskOwner::OrdinaryType9AttractAttention(owner) => owner
            .into_completed_native_hit(manager)
            .map_err(SpecializedActorTaskOwner::OrdinaryType9AttractAttention),
        other => Err(other),
    };
    match transfer {
        Ok((authority, next_transaction_id)) => {
            owners.insert(
                index,
                SpecializedActorTaskOwner::from_current_type9(
                    authority,
                    actor,
                    next_transaction_id,
                ),
            );
            Some(NativeType9HitCustody::Selected {
                actor,
                next_transaction_id,
            })
        }
        Err(previous) => {
            owners.insert(index, previous);
            None
        }
    }
}

impl crate::intro2_radial::Intro2RadialTaskCustody for SpecializedActorTaskScheduler {
    fn park_class49_terminal(&mut self, manager: &EntityManager, id: u32) {
        park_class49_terminal_owner(&mut self.owners, manager, id);
    }
    fn finish_class49_terminal(
        &mut self,
        receipt: crate::class49_death::Class49TerminalReceipt,
        ring: Option<Type60ExplodingRingProductionOwner>,
    ) {
        self.owners
            .retain(|owner| owner.entity_id() != receipt.entity_id);
        if let Some(ring) = ring {
            self.owners
                .push(SpecializedActorTaskOwner::Type60ExplodingRing(ring));
        }
    }
    fn prepare_native_actor_mutation(&mut self, manager: &EntityManager, entity_id: u32) -> bool {
        prepare_intro2_radial_actor_mutation(&mut self.owners, manager, entity_id)
    }

    fn retain_dynamic_result(&mut self, result: &crate::entity::DynamicRadialLiveOutcome) {
        self.register_radial_death_publications(&result.death_publications);
        if let Some(block) = &result.blocked {
            if block.target_prefix_committed {
                self.park_intro2_type8_external_prefix(block.target_id);
                self.park_intro2_type10_external_prefix(block.target_id);
                self.park_intro2_gun_turret_external_prefix(block.target_id);
                self.park_cleansing_vehicle_external_prefix(block.target_id);
                self.park_intro2_type17_external_prefix(block.target_id);
                self.park_native_type47_external_prefix(block.target_id);
                self.park_intro2_type58_external_prefix(block.target_id);
                for owner in &mut self.owners {
                    match owner {
                        SpecializedActorTaskOwner::Intro2Type53(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::NativeType122(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::NativeType30(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::NativeType40(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::NativeType56(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::NativeType43(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::NativeType38Family(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

struct Intro2RadialCursorCustody<'a> {
    pending: &'a mut Vec<SpecializedActorTaskOwner>,
    retained: &'a mut Vec<SpecializedActorTaskOwner>,
    remaining_live_ids: &'a [u32],
}

impl crate::intro2_radial::Intro2RadialTaskCustody for Intro2RadialCursorCustody<'_> {
    fn park_class49_terminal(&mut self, manager: &EntityManager, id: u32) {
        if self.pending.iter().any(|owner| owner.entity_id() == id) {
            park_class49_terminal_owner(self.pending, manager, id);
        } else {
            park_class49_terminal_owner(self.retained, manager, id);
        }
    }
    fn finish_class49_terminal(
        &mut self,
        receipt: crate::class49_death::Class49TerminalReceipt,
        ring: Option<Type60ExplodingRingProductionOwner>,
    ) {
        self.pending
            .retain(|owner| owner.entity_id() != receipt.entity_id);
        self.retained
            .retain(|owner| owner.entity_id() != receipt.entity_id);
        // Newly inserted ring allocations are visited by the next world pass;
        // the current cursor already owns its live-list identities.
        if let Some(ring) = ring {
            self.retained
                .push(SpecializedActorTaskOwner::Type60ExplodingRing(ring));
        }
    }
    fn prepare_native_actor_mutation(&mut self, manager: &EntityManager, entity_id: u32) -> bool {
        let owners = if self
            .pending
            .iter()
            .any(|owner| owner.entity_id() == entity_id)
        {
            &mut *self.pending
        } else {
            &mut *self.retained
        };
        prepare_intro2_radial_actor_mutation(owners, manager, entity_id)
    }

    fn retain_dynamic_result(&mut self, result: &crate::entity::DynamicRadialLiveOutcome) {
        for publication in &result.death_publications {
            let replacement = match specialized_radial_death_owner(*publication) {
                SpecializedRadialDeath::Task(owner) => owner,
                SpecializedRadialDeath::Deferred(allocation) => {
                    self.pending
                        .retain(|owner| owner.entity_id() != allocation.entity_id);
                    self.retained
                        .retain(|owner| owner.entity_id() != allocation.entity_id);
                    continue;
                }
            };
            let replacement =
                delivered_type9_contact::preserve_retirement(self.pending, replacement);
            let replacement =
                delivered_type9_contact::preserve_retirement(self.retained, replacement);
            let id = replacement.entity_id();
            if self
                .pending
                .iter()
                .chain(self.retained.iter())
                .any(|owner| owner.entity_id() == id && owner.is_native_contact_prefix())
            {
                continue;
            }
            self.pending.retain(|owner| owner.entity_id() != id);
            self.retained.retain(|owner| owner.entity_id() != id);
            // A later live-list allocation receives its new Primary in this
            // pass; an already visited allocation waits until the next pass.
            if self.remaining_live_ids.contains(&id) {
                self.pending.push(replacement);
            } else {
                self.retained.push(replacement);
            }
        }
        if let Some(block) = &result.blocked {
            if block.target_prefix_committed {
                for owner in self.pending.iter_mut().chain(self.retained.iter_mut()) {
                    match owner {
                        SpecializedActorTaskOwner::Intro2Type8(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::CleansingVehicle(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::Intro2GunTurret(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::Intro2Type10(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::Intro2Type57(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::Intro2Type10Tumble(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_contact_prefix();
                        }
                        SpecializedActorTaskOwner::Intro2Type57Tumble(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_contact_prefix();
                        }
                        SpecializedActorTaskOwner::Intro2Type17(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::Intro2Type47Scheduler(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::Intro2Type58(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::Intro2Type53(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::NativeType122(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::NativeType30(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::NativeType40(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::NativeType56(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::NativeType43(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::NativeType38Family(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        SpecializedActorTaskOwner::Intro2CommonDying(owner)
                            if owner.entity_id() == block.target_id =>
                        {
                            owner.park_external_prefix();
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn prepare_intro2_radial_actor_mutation(
    owners: &mut Vec<SpecializedActorTaskOwner>,
    manager: &EntityManager,
    entity_id: u32,
) -> bool {
    if owners
        .iter()
        .any(|owner| owner.entity_id() == entity_id && owner.is_delivered_type9_contact())
    {
        let authenticated = delivered_type9_contact::authenticated_retained_type9_contact(
            owners, manager, entity_id,
        );
        return delivered_type9_contact::with_retained_type9_contact(
            owners,
            entity_id,
            authenticated,
            false,
            |current| prepare_intro2_radial_actor_mutation(current, manager, entity_id),
        );
    }
    let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return false;
    };
    if crate::native_type61::has_native_allocation(entity) {
        //256C0/class23 is infallible and creates no task. A different or
        //suspended owner cannot stand in for that authored absence.
        return crate::native_type61::allocation_authenticates(manager, entity_id)
            && !owners.iter().any(|owner| owner.entity_id() == entity_id)
            && crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
                .into_iter()
                .all(|slot| entity.actor_tasks.task_in_slot(slot).is_none());
    }
    if crate::native_entity_weapons::entity_authenticates(entity) {
        return owners.iter().any(|owner| {
            matches!(owner,
            SpecializedActorTaskOwner::NativeWeapon(owner)
                if owner.entity_id() == entity_id && owner.authenticates(manager))
        });
    }
    // Class12 owns the same completed callback boundary regardless of its
    // living constructor family (16/17/26/47/53/58/94). Match retained custody
    // before selecting a living-family adapter; never infer it from style only.
    if let Some(owner) = owners.iter().find(|owner| owner.entity_id() == entity_id) {
        if owner.is_native_contact_prefix() {
            return false;
        }
        if let SpecializedActorTaskOwner::Intro2CommonDying(owner) = owner {
            return !owner.has_pending_prefix()
                && crate::intro2_common_dying::Intro2CommonDyingOwner::adopt(manager, entity_id)
                    == Ok(*owner)
                && crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
                    .into_iter()
                    .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
                    .all(|id| {
                        entity.actor_tasks.wrapper_flags(id)
                            == Some(crate::actor_task_owner::ActorTaskWrapperFlags {
                                alive: true,
                                in_callback: false,
                            })
                    });
        }
    }
    if crate::class49_death::intro2_type13_explosion_source_authenticates(entity) {
        return crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
            .all(|task| {
                entity
                    .actor_tasks
                    .wrapper_flags(task)
                    .is_some_and(|flags| flags.alive && !flags.in_callback)
            })
            && owners.iter().any(|owner| {
                matches!(owner,
                SpecializedActorTaskOwner::Intro2Type13SearchAttack(owner)
                if owner.entity_id() == entity_id && owner.completed_contact_boundary(manager))
            });
    }
    if entity.intro2_type16_runtime.is_some() {
        return owners.iter().any(|owner| {
            matches!(owner, SpecializedActorTaskOwner::Intro2Type16(owner)
                if owner.entity_id() == entity_id && owner.completed_mutation_boundary(manager))
        });
    }
    if entity.intro2_type53_runtime.is_some() {
        return owners.iter().any(|owner| {
            matches!(owner,
            SpecializedActorTaskOwner::Intro2Type53(owner)
                if owner.entity_id() == entity_id && owner.completed_mutation_boundary(manager))
        });
    }
    if entity.native_type122_runtime.is_some() {
        return owners.iter().any(|owner| {
            matches!(owner,
            SpecializedActorTaskOwner::NativeType122(owner)
                if owner.entity_id() == entity_id && owner.completed_mutation_boundary(manager))
        });
    }
    if entity.native_type30_runtime.is_some() {
        return owners.iter().any(|owner| {
            matches!(owner,
            SpecializedActorTaskOwner::NativeType30(owner)
                if owner.entity_id() == entity_id && owner.completed_mutation_boundary(manager))
        });
    }
    if entity.native_type40_runtime.is_some() {
        return owners.iter().any(|owner| {
            matches!(owner,
            SpecializedActorTaskOwner::NativeType40(owner)
                if owner.entity_id() == entity_id && owner.completed_mutation_boundary(manager))
        });
    }
    if entity.native_type56_runtime.is_some() {
        return owners.iter().any(|owner| {
            matches!(owner,
            SpecializedActorTaskOwner::NativeType56(owner)
                if owner.entity_id() == entity_id && owner.completed_mutation_boundary(manager))
        });
    }
    if entity.native_type38_runtime.is_some() {
        return owners.iter().any(|owner| {
            matches!(owner,
            SpecializedActorTaskOwner::NativeType38Family(owner)
                if owner.entity_id() == entity_id && owner.completed_mutation_boundary(manager))
        });
    }
    if entity.native_type43_runtime.is_some() {
        // A matching graph alone must not steal an executing or retired wrapper.
        if crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
            .any(|task| {
                entity
                    .actor_tasks
                    .wrapper_flags(task)
                    .is_none_or(|flags| !flags.alive || flags.in_callback)
            })
        {
            return false;
        }
        return owners.iter().any(|owner| {
            matches!(owner, SpecializedActorTaskOwner::NativeType43(owner)
                if !owner.has_pending_prefix()
                    && crate::native_type43::Type43Owner::adopt(manager, entity_id) == Ok(*owner))
        });
    }
    if entity.intro2_type58_runtime.is_some() {
        return owners.iter().any(|owner| {
            matches!(owner,
            SpecializedActorTaskOwner::Intro2Type58(owner)
                if owner.entity_id() == entity_id && owner.completed_mutation_boundary(manager))
        });
    }
    if entity.shared_fish_runtime.is_some() {
        return owners.iter().any(|owner| {
            matches!(owner,
            SpecializedActorTaskOwner::SharedFish(owner)
                if owner.entity_id() == entity_id && owner.completed_mutation_boundary(manager))
        });
    }
    if entity.cleansing_vehicle_runtime.is_some() {
        return owners.iter().any(|owner| {
            matches!(owner,
            SpecializedActorTaskOwner::CleansingVehicle(owner)
                if owner.entity_id() == entity_id && owner.completed_mutation_boundary(manager))
        });
    }
    if entity.intro2_type94_runtime.is_some() {
        return owners.iter().any(|owner| {
            matches!(owner,
            SpecializedActorTaskOwner::Intro2Type94(owner)
                if owner.entity_id() == entity_id && owner.completed_mutation_boundary(manager))
        });
    }
    if entity.native_type47_construction.is_some() {
        if !crate::shared_type47::type47_manager_allocation_authenticates(manager, entity_id)
            || crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
                .into_iter()
                .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
                .any(|task| {
                    entity
                        .actor_tasks
                        .wrapper_flags(task)
                        .is_none_or(|flags| !flags.alive || flags.in_callback)
                })
        {
            return false;
        }
        return owners.iter().any(|owner| match owner {
            SpecializedActorTaskOwner::Intro2Type47Scheduler(owner) => {
                owner.entity_id() == entity_id && owner.completed_mutation_boundary(manager)
            }
            SpecializedActorTaskOwner::Intro2CommonDying(owner) => {
                !owner.has_pending_prefix()
                    && crate::intro2_common_dying::Intro2CommonDyingOwner::adopt(manager, entity_id)
                        == Ok(*owner)
            }
            _ => false,
        });
    }
    if entity.intro2_type8_runtime.is_some() {
        return owners.iter().any(|owner| {
            matches!(owner,SpecializedActorTaskOwner::Intro2Type8(owner)
            if owner.entity_id()==entity_id && owner.completed_hit_boundary(manager))
        });
    }
    if entity.native_type123_runtime.is_some() {
        return owners.iter().any(|owner| {
            matches!(owner,SpecializedActorTaskOwner::NativeType123(owner)
            if owner.entity_id()==entity_id && owner.completed_hit_boundary(manager))
        });
    }
    if entity.native_type86_runtime.is_some() {
        return owners.iter().any(|owner| {
            matches!(owner,SpecializedActorTaskOwner::NativeType86(owner)
            if owner.entity_id()==entity_id && owner.completed_hit_boundary(manager))
        });
    }
    if entity.intro2_type17_runtime.is_some() {
        if crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
            .any(|task| {
                entity
                    .actor_tasks
                    .wrapper_flags(task)
                    .is_none_or(|flags| !flags.alive || flags.in_callback)
            })
        {
            return false;
        }
        return owners.iter().any(|owner| match owner {
            SpecializedActorTaskOwner::Intro2Type17(owner) => {
                !owner.has_pending_prefix()
                    && crate::intro2_type17::Intro2Type17Owner::adopt(manager, entity_id)
                        == Ok(*owner)
            }
            SpecializedActorTaskOwner::Intro2CommonDying(owner) => {
                !owner.has_pending_prefix()
                    && crate::intro2_common_dying::Intro2CommonDyingOwner::adopt(manager, entity_id)
                        == Ok(*owner)
            }
            _ => false,
        });
    }
    if entity.intro2_gun_turret_runtime.is_some() {
        if crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
            .any(|task| {
                entity
                    .actor_tasks
                    .wrapper_flags(task)
                    .is_none_or(|flags| !flags.alive || flags.in_callback)
            })
        {
            return false;
        }
        return owners.iter().any(|owner|matches!(owner,SpecializedActorTaskOwner::Intro2GunTurret(owner)
            if !owner.has_pending_prefix() && crate::intro2_gun_turret::Intro2GunTurretOwner::adopt(manager,entity_id)==Ok(*owner)));
    }
    if entity.intro2_type10_runtime.is_some() {
        if crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
            .any(|task| {
                entity
                    .actor_tasks
                    .wrapper_flags(task)
                    .is_none_or(|flags| !flags.alive || flags.in_callback)
            })
        {
            return false;
        }
        return owners.iter().any(|owner| match owner {
            SpecializedActorTaskOwner::Intro2Type10(owner) => {
                !owner.has_pending_prefix()
                    && crate::intro2_type10::Intro2Type10Owner::adopt(manager, entity_id)
                        == Ok(*owner)
            }
            SpecializedActorTaskOwner::Intro2Type57(owner) => {
                !owner.has_pending_prefix()
                    && crate::intro2_type57::Intro2Type57Owner::adopt(manager, entity_id)
                        == Ok(*owner)
            }
            SpecializedActorTaskOwner::Intro2Type10Tumble(owner) => {
                !owner.has_pending_prefix()
                    && crate::intro2_type10::Intro2Type10TumbleOwner::adopt(manager, entity_id)
                        == Ok(*owner)
            }
            SpecializedActorTaskOwner::Intro2Type57Tumble(owner) => {
                !owner.has_pending_prefix()
                    && crate::intro2_type57::death::Intro2Type57TumbleOwner::adopt(
                        manager, entity_id,
                    ) == Ok(*owner)
            }
            _ => false,
        });
    }
    if entity.intro2_type66_runtime.is_some() {
        // Unlike Type9's observation transfer, a fixed Factory has no pose
        // transaction to consume. A pending or stale receipt must still stop
        // damage before it can replace the graph that owns that prefix.
        return owners.iter().any(|owner| {
            matches!(owner, SpecializedActorTaskOwner::Intro2Type66(owner)
                if owner.entity_id() == entity_id
                    && !owner.has_pending_prefix()
                    && crate::intro2_type66::Intro2Type66Owner::adopt(manager, entity_id) == Ok(*owner))
        });
    }
    if entity.main_base_runtime.is_some() {
        return owners.iter().any(|owner| {
            matches!(owner, SpecializedActorTaskOwner::MainBase(owner)
                if owner.entity_id() == entity_id && !owner.has_pending_prefix()
                    && crate::main_base_runtime::MainBaseOwner::adopt(manager, entity_id) == Ok(*owner))
        });
    }
    if crate::intro2_type26_defecate_virus::type26_manager_allocation_authenticates(
        manager, entity_id,
    ) {
        // A newly adopted comparison owner has pending=false; equality thus
        // authenticates both the current graph and the retained completed
        // prefix. A matching graph alone must not steal an executing wrapper.
        return crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .filter_map(|slot| entity.actor_tasks.task_in_slot(slot))
            .all(|task| {
                entity
                    .actor_tasks
                    .wrapper_flags(task)
                    .is_some_and(|flags| flags.alive && !flags.in_callback)
            })
            && owners.iter().any(|owner| {
                matches!(owner, SpecializedActorTaskOwner::Intro2Type26(owner)
                    if owner.entity_id() == entity_id
                        && Intro2Type26WorldOwner::adopt(manager, entity_id) == Ok(*owner))
            });
    }
    begin_native_type9_external_mutation(owners, manager, entity_id).is_some()
}

fn park_type17_external_prefix(owners: &mut [SpecializedActorTaskOwner], id: u32) {
    for owner in owners {
        match owner {
            SpecializedActorTaskOwner::Intro2Type17(owner) if owner.entity_id() == id => {
                owner.park_external_prefix();
            }
            SpecializedActorTaskOwner::Intro2CommonDying(owner) if owner.entity_id() == id => {
                owner.park_external_prefix();
            }
            _ => {}
        }
    }
}

fn park_type47_external_prefix(owners: &mut [SpecializedActorTaskOwner], id: u32) {
    for owner in owners {
        match owner {
            SpecializedActorTaskOwner::Intro2Type47Scheduler(owner) if owner.entity_id() == id => {
                owner.park_external_prefix()
            }
            SpecializedActorTaskOwner::Intro2CommonDying(owner) if owner.entity_id() == id => {
                owner.park_external_prefix()
            }
            _ => {}
        }
    }
}

fn park_class49_terminal_owner(
    owners: &mut Vec<SpecializedActorTaskOwner>,
    manager: &EntityManager,
    id: u32,
) {
    if manager.iter_all().any(|entity| {
        entity.id == id
            && (entity.cleansing_vehicle_runtime.is_some()
                || entity.native_entity_weapon_runtime.is_some()
                || entity.shared_fish_runtime.is_some()
                || crate::intro2_type10::type10_auto_pilot_profile(entity).is_some()
                || crate::intro2_type16::type16_auto_pilot_row(entity).is_some()
                || crate::native_type43::allocation_authenticates(entity)
                || crate::native_type38::allocation_authenticates(entity)
                || crate::class49_death::intro2_type13_explosion_source_authenticates(entity))
    }) {
        contact_prefix::park_native_contact_prefix(owners, manager, id);
        return;
    }
    if let Ok(owner) =
        crate::intro2_gun_turret::Intro2GunTurretOwner::adopt_blocked_prefix(manager, id)
    {
        if let Some(current) = owners.iter_mut().find(|owner| owner.entity_id() == id) {
            *current = SpecializedActorTaskOwner::Intro2GunTurret(owner);
        }
    }
}

fn adopt_class14_after_selected_consume(
    retained: &mut Vec<SpecializedActorTaskOwner>,
    task_lease: Option<MainBaseType9ExplodingTaskLease>,
) {
    if let Some(task_lease) = task_lease {
        retained.push(SpecializedActorTaskOwner::OrdinaryType9Class14(
            MainBaseType9ExplodingProductionOwner::adopt(task_lease),
        ));
    }
}

fn restore_fresh_level1_type9_run_away_sidecars(
    manager: &mut EntityManager,
    sidecars: Vec<(
        usize,
        crate::ordinary_type9_initial_production::FreshLevel1Type9InitialProductionOwner,
    )>,
) {
    for (original_index, owner) in sidecars {
        manager
            .restore_fresh_level1_type9_initial_production(original_index, owner)
            .expect("load adoption rollback restores the just-drained unique Type-9 sidecar");
    }
}

fn ordinary_type9_attract_attention_candidates(
    manager: &EntityManager,
) -> Vec<GuardLocationEntityRef> {
    manager
        .iter_all()
        .map(|entity| GuardLocationEntityRef {
            id: entity.id,
            entity_type: entity.entity_type,
            position_raw: entity.position_raw(),
            state_flags_raw: entity.collision.state_flags_at_0x08,
            capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
            attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
        })
        .collect()
}

fn restore_fresh_level1_type9_selected_sidecars(
    manager: &mut EntityManager,
    sidecars: Vec<(
        usize,
        FreshLevel1Type9PublishedSchedulerBranch,
        FreshLevel1Type9InitialProductionOwner,
    )>,
) {
    for (original_index, _, owner) in sidecars {
        manager
            .restore_fresh_level1_type9_initial_production(original_index, owner)
            .expect("load adoption rollback restores the just-drained unique Type-9 sidecar");
    }
}

fn blocked_production_pass(
    reason: SpecializedActorTaskProductionBlock,
) -> SpecializedActorTaskProductionPass {
    SpecializedActorTaskProductionPass {
        block: Some(reason),
        outcomes: Vec::new(),
        surface_bubbles_materialized: 0,
        surface_bubbles_dropped: 0,
        sea_level_changed: false,
        explosion_lights: Vec::new(),
        progressive_death_presentation_requested: false,
        terrain_changed: false,
    }
}

fn materialize_surface_bubble(
    resources: &ResourceCache,
    world_fx: &mut WorldFx,
    request: ActorSurfaceBubbleRequest,
    retail_tick: u32,
    materialized: &mut usize,
    dropped: &mut usize,
) {
    let context = TerrainCollisionContext::from_current_level_cache(resources)
        .expect("pass preflight keeps strict Section 10/13 context available");
    if world_fx
        .materialize_actor_surface_bubble_request(
            request,
            ParticleEnvironment::Terrain(context),
            retail_tick,
        )
        .is_some()
    {
        *materialized += 1;
    } else {
        *dropped += 1;
    }
}

#[cfg(test)]
mod intro2_surface_tests;

#[cfg(test)]
mod intro2_radial_tests;

#[cfg(test)]
mod intro2_type10_tests;

#[cfg(test)]
mod intro2_type57_tests;

#[cfg(test)]
mod intro2_type13_tests;

#[cfg(test)]
mod native_type9_hit_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_animation::ActorAnimationController;
    use crate::actor_task_dispatcher::ActorTaskRuntime;
    use crate::actor_task_owner::{
        ActorTaskOwner, ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags, PreparedActorTask,
    };
    use crate::common_mover::sub_d::Type9SubDRuntime;
    use crate::common_mover::type9_attitude::Type9BodyBasis;
    use crate::entity::{
        exact_level_one_type9_attract_attention_manager, exact_level_one_type9_go_to_job_manager,
        exact_level_one_type9_selected_mixed_manager, exact_level_one_type9_wander_manager, Entity,
        EntityKind,
    };
    use crate::entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord,
        ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT,
        DEFERRED_DESTROY_PENDING_STATE_BIT, FULLY_ABOVE_SURFACE_STATE_BIT, SURFACE_STATE_MASK,
    };
    use crate::entity_scheduler::COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT;
    use crate::level::LevelState;
    use crate::main_base_type17_abort::{MainBaseType17DeathAdvance, MainBaseType17DeathOutcome};
    use crate::main_base_type47_abort::{MainBaseType47DeathAdvance, MainBaseType47DeathOutcome};
    use crate::main_base_type54_abort::{
        exact_type54_sea_level_owner_fixture, issue_exact_type54_sea_level_owner_receipt,
        MainBaseType54DeathAdvance, MainBaseType54DeathOutcome, MainBaseType54NetworkSession,
        LEVEL_ONE_TYPE54_MODEL_EXTENT_RAW,
    };
    use crate::main_base_type66_production::{
        effect_program as type66_effect_program, model_cache as type66_model_cache,
        post_abort_owner as exact_type66_post_abort_owner,
        set_terminal_predecessor as set_type66_terminal_predecessor,
    };
    use crate::main_base_type9_abort::{
        MainBaseType9CapturedProfile, MainBaseType9DeathAdvance, MainBaseType9DeathOutcome,
        MainBaseType9ResultScreenState, LEVEL_ONE_TYPE9_ENTITY_TYPE, LEVEL_ONE_TYPE9_MODEL_ID,
    };
    use crate::main_base_type9_production::tests::{
        exact_main_base_type9_exploding_owner_fixture, issue_exact_main_base_type9_exploding_owner,
    };
    use crate::main_base_type9_production::{
        tick_main_base_type9_exploding_owner, MainBaseType9ExplodingProductionBlock,
        MainBaseType9ExplodingProductionFrame, MainBaseType9ExplodingProductionOwner,
    };
    use crate::shared_retarget_mover::SharedRetarget;
    use crate::type17_common_dying_production::coarse_type17_common_dying_composite_fixture;
    use crate::type47_common_dying_production::exact_type47_single_owner_fixture;
    use v2k_formats::collision::ActorAnimationDescriptor;
    use v2k_formats::levels::LevelDescriptor;
    use v2k_formats::models::{ModelCollection, ModelEntry, StreamStats};
    use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};

    fn task_visit(replacements: usize) -> ActorTaskVisit {
        let mut tasks = ActorTaskOwner::new();
        let mut task_id =
            tasks.replace_prepared(ActorTaskSlot::Primary, PreparedActorTask::new(()));
        for _ in 0..replacements {
            task_id = tasks.replace_prepared(ActorTaskSlot::Primary, PreparedActorTask::new(()));
        }
        ActorTaskVisit {
            slot: ActorTaskSlot::Primary,
            task_id,
        }
    }

    fn type17_owner(entity_id: u32, replacements: usize) -> LevelOneType17CommonDyingOwner {
        LevelOneType17CommonDyingOwner::from_authenticated_publication(
            entity_id,
            17,
            task_visit(replacements),
        )
    }

    fn type47_owner(entity_id: u32, replacements: usize) -> FreshLevelOneType47CommonDyingOwner {
        FreshLevelOneType47CommonDyingOwner::from_authenticated_publication(
            entity_id,
            11,
            task_visit(replacements),
        )
    }

    fn flat_terrain() -> TerrainGrid {
        TerrainGrid {
            header: [0; 5],
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

    fn collision_level_descriptor() -> LevelDescriptor {
        LevelDescriptor {
            raw_header: [0; 0xd0],
            name: "specialized-task-test".into(),
            world_style: 0,
            terrain_sprite_base: 0,
            sky_color_index: 0,
            sky_model: 0,
            main_base_abort_sky_color_index: 0,
            main_base_abort_sky_model: 0,
            terrain_draw_depth: 0,
            sub_count: 0,
            campaign_record_count: 0,
            entities: Vec::new(),
            campaign_records: Vec::new(),
        }
    }

    fn level_state(
        terrain: Option<TerrainGrid>,
        descriptor: Option<LevelDescriptor>,
    ) -> LevelState {
        LevelState {
            source_path: "specialized-task-test.ovl".into(),
            system_level: None,
            fixup_data: None,
            fixup_code: None,
            strings: Vec::new(),
            sprites: None,
            params: None,
            display_modes: None,
            fog_gradient: None,
            color_palettes: None,
            models: None,
            anim_frames: None,
            terrain,
            anim_sound: None,
            collision: None,
            level: descriptor,
            linkage: None,
        }
    }

    fn type9_model_entry(index: usize, radius: u16) -> ModelEntry {
        ModelEntry {
            index,
            cmd_word_count: 0,
            extra_count: 0,
            flags: 0x40,
            slot_count: 0,
            face_val: 2,
            radius,
            collision_radius_raw: 0,
            collision_program: Vec::new(),
            records: Vec::new(),
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
            painter_program: Vec::new(),
            name: None,
        }
    }

    fn type9_model_layer() -> LevelState {
        const LEVEL6_GLOBAL_BASE: usize = 324;
        const TYPE9_LOCAL_MODEL_ID: usize = LEVEL_ONE_TYPE9_MODEL_ID - LEVEL6_GLOBAL_BASE;
        let mut models = (0..=TYPE9_LOCAL_MODEL_ID)
            .map(|index| type9_model_entry(index, 0))
            .collect::<Vec<_>>();
        models[TYPE9_LOCAL_MODEL_ID].radius =
            crate::main_base_type9_actor_production::LEVEL_ONE_TYPE9_MODEL_EXTENT_RAW;
        let mut layer = level_state(None, None);
        layer.system_level = Some(6);
        layer.models = Some(ModelCollection {
            sub_blocks: Vec::new(),
            all_entries: models,
            stats: StreamStats::default(),
        });
        layer
    }

    fn strict_resources(terrain: TerrainGrid) -> ResourceCache {
        let mut resources = ResourceCache::new(vec![type9_model_layer()]);
        resources.load_level(level_state(
            Some(terrain),
            Some(collision_level_descriptor()),
        ));
        resources
    }

    fn strict_type66_resources(active_program: Vec<u8>) -> ResourceCache {
        let mut resources = type66_model_cache(active_program);
        resources.add_auxiliary(type9_model_layer());
        resources.load_level(level_state(
            Some(flat_terrain()),
            Some(collision_level_descriptor()),
        ));
        resources
    }

    fn production_frame<'a>(
        notification_phase: GameplayNotificationPhase,
        resources: &'a mut ResourceCache,
        world_fx: &'a mut WorldFx,
        static_damage: &'a mut crate::static_damage::StaticDamageScheduler,
        elapsed_micros: u32,
        global_elapsed_micros: u32,
    ) -> SpecializedActorTaskProductionFrame<'a> {
        SpecializedActorTaskProductionFrame {
            world: SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase,
            resources,
            world_fx,
            static_damage,
            elapsed_micros,
            global_elapsed_micros,
            retail_tick: 0,
            main_base_abort_active: false,
        }
    }

    #[v2k_test_support::retail_test]
    fn intro2_inline_radial_death_advances_only_later_live_slots() {
        use crate::common_mover::sub_d::Type9SubDFrameOwner;
        use crate::entity::DynamicRadialDeathPublication;
        use crate::intro2_common_dying::Intro2CommonDyingOutcome;
        use crate::intro2_meteors::BOULDER_ROLLING_LIFETIME_MS;
        use crate::intro2_radial::Intro2MeteorDeathReport;
        use crate::intro2_type9_class14::Intro2Type9Class14Outcome;
        use crate::ordinary_type9_live::OrdinaryType9SelectedComponentRuntime;

        fn death_age(manager: &EntityManager, id: u32) -> u32 {
            match manager
                .iter_all()
                .find(|entity| entity.id == id)
                .unwrap()
                .actor_task_state(ActorTaskSlot::Primary)
            {
                Some(ActorTaskRuntime::CommonDying(task)) => task.elapsed_ms(),
                Some(ActorTaskRuntime::SharedRetarget(task)) => task.elapsed_ms(),
                other => panic!("expected a published death task: {other:?}"),
            }
        }

        let mut registration_order_oracle = None;
        for reverse_registration in [false, true] {
            let Some((mut session, mut manager, _)) =
                crate::intro2_type47_live::world::native_intro2_fixture()
            else {
                return;
            };
            let [earlier, meteor, later, peasant] = [20, 31, 38, 49].map(|spawn| {
                manager
                    .iter_all()
                    .find(|entity| entity.authored_spawn_index == Some(spawn))
                    .unwrap()
                    .id
            });
            const ORIGIN: [i16; 3] = [0, 20_000, 0];
            // Keep real authored allocation identity, metadata, task graphs
            // and manager order. Only pose, mode and the meteor's task clock
            // are controlled: the high shared pose excludes static terrain
            // and every unrelated actor from this one radial callback.
            for id in [earlier, meteor, later, peasant] {
                let entity = manager.entity_mut(id).unwrap();
                entity.set_motion_raw(ORIGIN, [0; 3]);
                entity
                    .collision
                    .state_flags_at_0x08
                    .overwrite(0x0206_8000, 0x0206_8000);
                entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
                entity.collision.callback_scheduler_accumulator_us_at_0x6c =
                    RetailRuntimeValue::Known(0);
            }
            // The earlier living owner completes a real callback-disabled
            // visit. Its retained living receipt must still be replaced when
            // the later meteor kills it, without a second visit this frame.
            manager
                .entity_mut(earlier)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, 0);
            let entity = manager.entity_mut(peasant).unwrap();
            let selected = entity.ordinary_type9_selected_component_runtime.unwrap();
            let mut components = selected.components();
            let seed = components
                .sub_d_frame_owner
                .classifier_cache()
                .stagger_counter();
            // This is an explicit classifier fixture, not first-query origin
            // evidence. Preserve its own seed while forcing a terrain rebuild.
            components.sub_d_frame_owner =
                Type9SubDFrameOwner::from_retail_state([0; 8], [0, 0], seed);
            entity.ordinary_type9_selected_component_runtime = Some(
                OrdinaryType9SelectedComponentRuntime::new(components, selected.kind()),
            );
            let meteor_entity = manager.entity_mut(meteor).unwrap();
            let primary = meteor_entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            let Some(ActorTaskRuntime::BoulderRolling(task)) =
                meteor_entity.actor_tasks.task_state_mut(primary)
            else {
                panic!()
            };
            task.elapsed_ms = BOULDER_ROLLING_LIFETIME_MS;
            task.previous_position_raw = ORIGIN;
            let old_tasks = [earlier, later, peasant].map(|id| {
                manager
                    .entity_mut(id)
                    .unwrap()
                    .actor_tasks
                    .task_in_slot(ActorTaskSlot::Primary)
            });
            let mut owners = vec![
                SpecializedActorTaskOwner::Intro2Type53(
                    crate::intro2_type53::Intro2Type53Owner::adopt(&manager, earlier).unwrap(),
                ),
                SpecializedActorTaskOwner::Intro2Meteor(
                    Intro2MeteorOwner::adopt_published(manager.entity_mut(meteor).unwrap())
                        .unwrap(),
                ),
                SpecializedActorTaskOwner::Intro2Type53(
                    crate::intro2_type53::Intro2Type53Owner::adopt(&manager, later).unwrap(),
                ),
            ];
            // Radial mutation now requires the actual native peasant owner,
            // even when this later live slot will receive a death replacement
            // before its first callback. Adoption consumes no constructor RNG.
            let mut native_type9_tasks = SpecializedActorTaskScheduler::default();
            assert_eq!(native_type9_tasks.adopt_intro2_type9(&mut manager), 13);
            owners.extend(
                native_type9_tasks
                    .owners
                    .into_iter()
                    .filter(|owner| owner.entity_id() == peasant),
            );
            if reverse_registration {
                owners.reverse();
            }
            let mut scheduler = SpecializedActorTaskScheduler::default();
            for owner in owners {
                scheduler.register(owner).unwrap();
            }
            let mut fx = WorldFx::new();
            let mut rng_oracle = WorldFx::new();
            // The existing explosion allocator owns its particle/sound RNG.
            // The scheduler adds exactly three death constructor words and
            // the later peasant's two immediate retarget words; no coarse wait,
            // old later task, or duplicate earlier death callback may run.
            rng_oracle.emit_intro2_meteor_impact_raw(
                ORIGIN,
                meteor,
                session
                    .cache
                    .global_model(crate::intro2_meteors::INTRO2_METEOR_MODEL)
                    .unwrap()
                    .radius,
                Some(session.cache.level_terrain().unwrap().sea_level_raw()),
            );
            for _ in 0..5 {
                rng_oracle.next_shared_retail_random_u16();
            }
            let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
            let mut notifications = GameplayNotifications::new();
            let pass = scheduler.tick(
                &mut manager,
                production_frame(
                    GameplayNotificationPhase::NonGameplay,
                    &mut session.cache,
                    &mut fx,
                    &mut static_damage,
                    20_000,
                    20_000,
                ),
                &mut notifications,
            );
            assert_eq!(pass.block, None);
            assert_eq!(
                pass.outcomes
                    .iter()
                    .map(SpecializedActorTaskProductionOutcome::entity_id)
                    .collect::<Vec<_>>(),
                [earlier, meteor, later, peasant],
            );
            assert!(matches!(
                &pass.outcomes[0],
                SpecializedActorTaskProductionOutcome::Intro2Type53(
                    crate::intro2_type53::Intro2Type53Outcome::Advanced {
                        callback_enabled: false,
                        ..
                    }
                )
            ));
            let SpecializedActorTaskProductionOutcome::Intro2Meteor {
                outcome: Intro2MeteorOutcome::Terminal { .. },
                death:
                    Some(Intro2MeteorDeathReport::Applied {
                        dynamic,
                        static_deliveries,
                        finalized: true,
                    }),
            } = &pass.outcomes[1]
            else {
                panic!("{:?}", pass.outcomes)
            };
            assert!(static_deliveries.is_empty());
            assert_eq!(dynamic.blocked, None);
            assert!(matches!(dynamic.death_publications.as_slice(), [
                DynamicRadialDeathPublication::Intro2Class12(first),
                DynamicRadialDeathPublication::Intro2Class12(second),
                DynamicRadialDeathPublication::Intro2Type9Class14(third),
            ] if [first.entity_id(), second.entity_id(), third.actor().entity_id] == [earlier, later, peasant]));
            assert!(matches!(
                &pass.outcomes[2],
                SpecializedActorTaskProductionOutcome::Intro2CommonDying(
                    Intro2CommonDyingOutcome::Advanced {
                        callback_elapsed_micros: 20_000,
                        terminal: false,
                        ..
                    }
                )
            ));
            assert!(matches!(
                &pass.outcomes[3],
                SpecializedActorTaskProductionOutcome::Intro2Type9Class14(
                    Intro2Type9Class14Outcome::Advanced {
                        callback_elapsed_micros: 20_000,
                        detailed: true,
                        ..
                    }
                )
            ));
            assert_eq!(
                [earlier, later, peasant].map(|id| death_age(&manager, id)),
                [0, 20, 20]
            );
            let death_tasks = [earlier, later, peasant].map(|id| {
                manager
                    .entity_mut(id)
                    .unwrap()
                    .actor_tasks
                    .task_in_slot(ActorTaskSlot::Primary)
            });
            for (old, new) in old_tasks.into_iter().zip(death_tasks) {
                assert_ne!(
                    old, new,
                    "radial death replaces each old Primary exactly once"
                );
            }
            assert_eq!(scheduler.registered_len(), 3);
            assert_eq!(
                scheduler.family_for(earlier),
                Some(SpecializedActorTaskFamily::Intro2CommonDying)
            );
            assert_eq!(
                scheduler.family_for(later),
                Some(SpecializedActorTaskFamily::Intro2CommonDying)
            );
            assert_eq!(
                scheduler.family_for(peasant),
                Some(SpecializedActorTaskFamily::Intro2Type9Class14)
            );
            assert_eq!(manager.pending_actor_deferred_destroy_ids(), &[meteor]);
            let first_next_random = fx.next_shared_retail_random_u16();
            assert_eq!(
                first_next_random,
                rng_oracle.next_shared_retail_random_u16()
            );
            fx.process_pending();
            let sound_ids = fx
                .take_positional_sounds()
                .into_iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>();
            assert_eq!(sound_ids, [62, 75, 75, 35]);
            assert_eq!(manager.cleanup_pending_actor_deferred_destroys(), [meteor]);
            manager
                .entity_mut(earlier)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(
                    COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT,
                    COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT,
                );
            let next = scheduler.tick(
                &mut manager,
                production_frame(
                    GameplayNotificationPhase::NonGameplay,
                    &mut session.cache,
                    &mut fx,
                    &mut static_damage,
                    20_000,
                    20_000,
                ),
                &mut notifications,
            );
            assert_eq!(next.block, None);
            assert!(
                next.outcomes.iter().all(|outcome| matches!(
                    outcome,
                    SpecializedActorTaskProductionOutcome::Intro2CommonDying(
                        Intro2CommonDyingOutcome::Advanced {
                            terminal: false,
                            ..
                        }
                    ) | SpecializedActorTaskProductionOutcome::Intro2Type9Class14(
                        Intro2Type9Class14Outcome::Advanced { .. }
                    )
                )),
                "second pass must complete each retained death callback: {:?}",
                next.outcomes
            );
            assert_eq!(
                next.outcomes
                    .iter()
                    .map(SpecializedActorTaskProductionOutcome::entity_id)
                    .collect::<Vec<_>>(),
                [earlier, later, peasant]
            );
            assert_eq!(
                [earlier, later, peasant].map(|id| death_age(&manager, id)),
                [20, 40, 40]
            );
            assert_eq!(
                [earlier, later, peasant].map(|id| manager
                    .entity_mut(id)
                    .unwrap()
                    .actor_tasks
                    .task_in_slot(ActorTaskSlot::Primary)),
                death_tasks
            );
            assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
            assert_eq!(
                fx.pending_event_count(),
                0,
                "retained death owners do not replay publication sound"
            );
            let observation = (
                first_next_random,
                fx.next_shared_retail_random_u16(),
                [earlier, later, peasant].map(|id| manager.entity_mut(id).unwrap().position_raw()),
            );
            if let Some(expected) = &registration_order_oracle {
                assert_eq!(
                    &observation, expected,
                    "registration order cannot change live-list RNG or movement"
                );
            } else {
                registration_order_oracle = Some(observation);
            }
        }
    }

    fn assert_unblocked_without_surface_bubbles(pass: &SpecializedActorTaskProductionPass) {
        assert_eq!(pass.block, None);
        assert_eq!(pass.surface_bubbles_materialized, 0);
        assert_eq!(pass.surface_bubbles_dropped, 0);
        assert!(pass.explosion_lights.is_empty());
    }

    fn exact_type9_manager(
        entity_id: u32,
    ) -> (EntityManager, MainBaseType9ExplodingProductionOwner) {
        let (entity, metadata, task_id) =
            exact_main_base_type9_exploding_owner_fixture(entity_id, [0; 3]);
        let mut metadata_rows = vec![EntityTypeRuntimeMetadata::default(); 48];
        metadata_rows[LEVEL_ONE_TYPE9_ENTITY_TYPE as usize] = metadata;
        let manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![entity],
            metadata_rows,
            true,
        );
        let owner = issue_exact_main_base_type9_exploding_owner(&manager, entity_id, task_id);
        (manager, owner)
    }

    fn adopt_exact_other_families(
        manager: &mut EntityManager,
    ) -> (
        MainBaseType54SeaLevelTickReceipt,
        MainBaseType9ExplodingTaskLease,
        FreshLevelOneType47CommonDyingOwner,
        LevelOneType17CommonDyingOwner,
    ) {
        let mut abort_fx = WorldFx::new();
        let mut type54_terrain = flat_terrain();
        type54_terrain.header[0] = -216_832;

        let type54_actor = manager.main_base_abort_actor_observation(3).unwrap().lease;
        let type54_outcome = match manager
            .apply_main_base_abort_type54_death(
                type54_actor,
                &type54_terrain,
                LEVEL_ONE_TYPE54_MODEL_EXTENT_RAW,
                MainBaseType54NetworkSession::SoloNetworkingDisabled,
                &mut abort_fx,
            )
            .unwrap()
        {
            MainBaseType54DeathAdvance::Advanced { outcome, .. }
            | MainBaseType54DeathAdvance::SuccessorUnavailableAfterCommit { outcome } => outcome,
        };
        let MainBaseType54DeathOutcome::SeaLevelTaskPublished { tick_receipt, .. } = type54_outcome
        else {
            panic!("the exact spawn-8 Type-54 publishes Change-Sea-Level")
        };

        let type9_actor = manager.main_base_abort_actor_observation(4).unwrap().lease;
        let type9_outcome = match manager
            .apply_main_base_abort_type9_death(
                type9_actor,
                MainBaseType9ResultScreenState::AlreadyShownByMainBaseAbort,
                &mut abort_fx,
            )
            .unwrap()
        {
            MainBaseType9DeathAdvance::Advanced { outcome, .. }
            | MainBaseType9DeathAdvance::SuccessorUnavailableAfterCommit { outcome } => outcome,
        };
        let MainBaseType9DeathOutcome::ExplodingTaskPublished { task_lease, .. } = type9_outcome
        else {
            panic!("the exact spawn-9 Type-9 publishes Exploding Person")
        };

        let type47_actor = manager.main_base_abort_actor_observation(6).unwrap().lease;
        let type47_outcome = match manager
            .apply_main_base_abort_type47_death(type47_actor, &mut abort_fx)
            .unwrap()
        {
            MainBaseType47DeathAdvance::Advanced { outcome, .. }
            | MainBaseType47DeathAdvance::SuccessorUnavailableAfterCommit { outcome } => outcome,
        };
        let MainBaseType47DeathOutcome::CommonDyingPublished(type47) = type47_outcome else {
            panic!("the exact spawn-11 Type-47 publishes Common Dying")
        };

        let type17_actor = manager.main_base_abort_actor_observation(12).unwrap().lease;
        let type17_outcome = match manager
            .apply_main_base_abort_type17_death(type17_actor, &mut abort_fx)
            .unwrap()
        {
            MainBaseType17DeathAdvance::Advanced { outcome, .. }
            | MainBaseType17DeathAdvance::SuccessorUnavailableAfterCommit { outcome } => outcome,
        };
        let MainBaseType17DeathOutcome::CommonDyingPublished(type17) = type17_outcome else {
            panic!("the exact spawn-17 Type-17 publishes Common Dying")
        };

        (tick_receipt, task_lease, type47.owner, type17.owner)
    }

    type Type9RuntimeSnapshot = (u32, [i16; 3], Type9SubDRuntime, i32, u8, u32);

    fn type9_runtime_snapshot(manager: &EntityManager, entity_id: u32) -> Type9RuntimeSnapshot {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("the Type-9 fixture remains live");
        let death = entity
            .main_base_type9_death_component_runtime
            .as_ref()
            .expect("the Type-9 owner retains its component runtime");
        let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
            panic!("the Type-9 owner retains its special Sub-I runtime")
        };
        let Some(ActorTaskRuntime::SharedRetarget(task)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("the Type-9 owner retains its shared-retarget task")
        };
        (
            entity.heading.to_bits(),
            entity.velocity_raw(),
            death.components.sub_d_runtime,
            animation.countdown_millis(),
            animation.phase(),
            task.elapsed_ms(),
        )
    }

    fn type66_task_elapsed_ms(
        manager: &EntityManager,
        task_lease: MainBaseType66WorkingFactoryTaskLease,
    ) -> u32 {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == task_lease.actor().entity_id)
            .expect("the Type-66 fixture remains live");
        let Some(ActorTaskRuntime::WorkingFactory(task)) =
            entity.actor_tasks.task_state(task_lease.task_id())
        else {
            panic!("the Type-66 fixture retains its Working Factory Primary")
        };
        task.elapsed_ms()
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Type66SchedulerLiveSnapshot {
        collision_state: RetailStateWord,
        last_hit_tick: RetailRuntimeValue<u32>,
        health: RetailRuntimeValue<i32>,
        model_index: Option<usize>,
        factory: crate::entity::BaseFactoryRuntimeState,
        task: crate::main_base_type66_abort::WorkingFactoryTaskState,
        wrapper: ActorTaskWrapperFlags,
        pending_power_up_destroys: Vec<u32>,
        pending_main_base_conversion_destroys: Vec<u32>,
        pending_factory_scientist_destroys: Vec<u32>,
        pending_actor_destroys: Vec<u32>,
    }

    fn type66_live_snapshot(
        manager: &EntityManager,
        task_lease: MainBaseType66WorkingFactoryTaskLease,
    ) -> Type66SchedulerLiveSnapshot {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == task_lease.actor().entity_id)
            .expect("the Type-66 fixture remains live");
        let RetailRuntimeValue::Known(Some(factory)) = entity.base_factory_runtime else {
            panic!("the Type-66 fixture retains exact factory state")
        };
        let Some(ActorTaskRuntime::WorkingFactory(task)) =
            entity.actor_tasks.task_state(task_lease.task_id())
        else {
            panic!("the Type-66 fixture retains its Working Factory Primary")
        };
        Type66SchedulerLiveSnapshot {
            collision_state: entity.collision.state_flags_at_0x08,
            last_hit_tick: entity.collision.last_hit_presentation_tick_at_0x34,
            health: entity.collision.health_raw,
            model_index: entity.model_index,
            factory,
            task: *task,
            wrapper: entity
                .actor_tasks
                .wrapper_flags(task_lease.task_id())
                .expect("the exact Type-66 wrapper remains installed"),
            pending_power_up_destroys: manager.pending_power_up_destroy_ids().to_vec(),
            pending_main_base_conversion_destroys: manager
                .pending_main_base_conversion_destroy_ids()
                .to_vec(),
            pending_factory_scientist_destroys: manager
                .pending_factory_scientist_destroy_ids()
                .to_vec(),
            pending_actor_destroys: manager.pending_actor_deferred_destroy_ids().to_vec(),
        }
    }

    fn assert_type66_owner_unchanged(
        scheduler: &mut SpecializedActorTaskScheduler,
        task_lease: MainBaseType66WorkingFactoryTaskLease,
    ) {
        let displaced = scheduler
            .register_main_base_type66_production(task_lease)
            .unwrap()
            .expect("the retryable block retains exact Type-66 owner custody");
        assert_eq!(
            displaced,
            MainBaseType66ProductionOwner::adopt(task_lease),
            "a block must not advance the scheduler owner's linear sequence"
        );
    }

    fn direct_type9_snapshot(
        basis: Type9BodyBasis,
        elapsed_micros: u32,
        global_elapsed_micros: u32,
    ) -> (
        MainBaseType9ExplodingProductionOutcome,
        Type9RuntimeSnapshot,
    ) {
        let terrain = flat_terrain();
        let (mut manager, owner) = exact_type9_manager(9);
        manager
            .entity_mut_for_test(9)
            .expect("the exact Type-9 fixture remains live")
            .physical_body_basis_q31 = RetailRuntimeValue::Known(basis);
        manager
            .entity_mut_for_test(9)
            .expect("the exact Type-9 fixture remains live")
            .set_velocity_raw([8_192, 777, 0]);
        let mut words = [0x9000, 0x7000].into_iter();
        let mut draws = 0;
        let tick = tick_main_base_type9_exploding_owner(
            &mut manager,
            owner,
            MainBaseType9ExplodingProductionFrame {
                terrain: &terrain,
                elapsed_micros,
                global_elapsed_micros,
                scheduler_mode: 0,
            },
            &mut || {
                draws += 1;
                words.next().expect("the near retarget consumes two words")
            },
        );
        assert_eq!(draws, 2);
        assert!(tick.retained_owner.is_some());
        let snapshot = type9_runtime_snapshot(&manager, 9);
        (tick.outcome, snapshot)
    }

    #[test]
    fn load_run_away_adoption_requires_an_empty_scheduler() {
        let mut manager = EntityManager::from_entities_for_test(Vec::new());
        let mut scheduler = SpecializedActorTaskScheduler::new();

        assert_eq!(
            scheduler.adopt_fresh_level1_type9_run_away(&mut manager),
            Ok(0),
            "a manager without selected Run Away sidecars is an exact no-op"
        );
        scheduler
            .register_type17_common_dying(type17_owner(7, 0))
            .unwrap();
        assert_eq!(
            scheduler.adopt_fresh_level1_type9_run_away(&mut manager),
            Err(OrdinaryType9RunAwayLoadAdoptionError::SchedulerNotEmpty { registered_len: 1 },)
        );
        assert!(manager.fresh_level1_type9_initial_productions().is_empty());
        assert_eq!(scheduler.registered_len(), 1);
    }

    fn native_load_after_campaign_cargo(
        level_id: u32,
    ) -> (
        crate::session::GameSession,
        EntityManager,
        SpecializedActorTaskScheduler,
        WorldFx,
    ) {
        use crate::entity::{
            AuthoredPlayerArrival, AuthoredWorldConstruction, CampaignCargoControllerState,
            CampaignCargoRestoreContext, EntityConstructionResources,
        };
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").is_file(),
            "normal-tier retail corpus required"
        );
        let mut session = crate::session::GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(level_id, 1).unwrap();
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(id, _)| {
                EntityTypeRuntimeMetadata::from_section12(
                    session.cache.global_entity_type(id).unwrap(),
                )
            })
            .collect();
        let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
        let resources = EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&model_extent),
        };
        let mut fx = WorldFx::new();
        let mut manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                logical_world_index: (level_id - 12) as i32,
                level: session.cache.level_desc().unwrap(),
                type_metadata: &metadata,
                resources,
                player_arrival: Some(AuthoredPlayerArrival {
                    position_raw: [19_712, -500, 14_848],
                    heading_raw: 0x4000,
                }),
                retail_tick: 71,
            },
            &mut fx,
        )
        .unwrap();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        let mut notifications = GameplayNotifications::new();
        // Match main's successful ordinary load:451C00 acquires class0 custody
        // before the remaining selected Type9 sidecars leave the manager.
        manager
            .restore_campaign_cargo_controller_state(
                CampaignCargoControllerState::from_packed(5, []),
                CampaignCargoRestoreContext {
                    level: session.cache.level_desc().unwrap(),
                    resources,
                    retail_tick: 71,
                    scheduler: &mut scheduler,
                    world_fx: &mut fx,
                    notifications: &mut notifications,
                },
            )
            .unwrap();
        assert!(!scheduler.is_empty());
        (session, manager, scheduler, fx)
    }

    #[v2k_test_support::retail_test]
    fn selected_type9_adoption_retains_class0_owners_published_by_451c00() {
        for level in [14, 39] {
            let (_session, mut manager, mut scheduler, mut fx) =
                native_load_after_campaign_cargo(level);
            let class0: Vec<_> = scheduler
                .owners
                .iter()
                .map(|owner| match owner {
                    SpecializedActorTaskOwner::Class0Actor(owner) => *owner,
                    _ => panic!("451C00 only adopts its required class0 family"),
                })
                .collect();
            let selected: Vec<_> = manager
                .fresh_level1_type9_initial_productions()
                .iter()
                .filter(|owner| owner.successful_scheduler_branch().is_some())
                .map(FreshLevel1Type9InitialProductionOwner::entity_id)
                .collect();
            assert!(!selected.is_empty());
            let stamp_counter = manager.next_common_body_ordinal();
            let mut expected_fx = fx.fork_for_main_base_abort_transaction();
            assert_eq!(
                scheduler.adopt_fresh_level1_type9_selected(&mut manager),
                Ok(selected.len())
            );
            assert_eq!(scheduler.registered_len(), class0.len() + selected.len());
            for (index, owner) in class0.iter().enumerate() {
                assert!(
                    matches!(&scheduler.owners[index], SpecializedActorTaskOwner::Class0Actor(retained)
                    if retained == owner)
                );
                assert!(
                    scheduler.prepare_class0_actor_external_mutation(&manager, owner.entity_id())
                );
            }
            assert_eq!(
                scheduler.owners[class0.len()..]
                    .iter()
                    .map(|owner| owner.entity_id())
                    .collect::<Vec<_>>(),
                selected
            );
            assert_eq!(
                scheduler.adopt_fresh_level1_type9_selected(&mut manager),
                Ok(0)
            );
            assert_eq!(scheduler.registered_len(), class0.len() + selected.len());
            assert_eq!(manager.next_common_body_ordinal(), stamp_counter);
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                expected_fx.next_shared_retail_random_u16()
            );
        }
    }

    #[v2k_test_support::retail_test]
    fn selected_type9_conflict_and_graph_failures_preserve_existing_custody_and_all_sidecars() {
        for corrupt_graph in [false, true] {
            let (_session, mut manager, mut scheduler, _fx) = native_load_after_campaign_cargo(14);
            // Obtain a real selected owner through the production adoption on
            // an exact transaction fork; do not fabricate an enum/task lease.
            let mut fork = manager.fork_for_main_base_abort_transaction();
            let mut occupied = SpecializedActorTaskScheduler::new();
            assert!(
                occupied
                    .adopt_fresh_level1_type9_selected(&mut fork)
                    .unwrap()
                    > 0
            );
            let conflicting = occupied.owners.remove(0);
            let entity_id = conflicting.entity_id();
            scheduler.owners.push(conflicting);
            if corrupt_graph {
                manager
                    .entity_mut(entity_id)
                    .unwrap()
                    .current_behavior_context = RetailRuntimeValue::Unresolved;
            }
            let before_owners: Vec<_> = scheduler
                .owners
                .iter()
                .map(|owner| (owner.entity_id(), owner.family()))
                .collect();
            let before_sidecars: Vec<_> = manager
                .fresh_level1_type9_initial_productions()
                .iter()
                .map(|owner| (owner.entity_id(), owner.successful_scheduler_branch()))
                .collect();
            let before_tasks: Vec<_> = manager
                .iter_all()
                .map(|entity| {
                    (
                        entity.id,
                        entity.current_behavior_context,
                        entity.collision.state_flags_at_0x08,
                        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                        entity.actor_tasks.task_in_slot(ActorTaskSlot::Secondary),
                        entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary),
                    )
                })
                .collect();
            let expected = if corrupt_graph {
                OrdinaryType9SelectedLoadAdoptionError::OwnerAuthenticationFailed { entity_id }
            } else {
                OrdinaryType9SelectedLoadAdoptionError::RegisteredEntityConflict { entity_id }
            };
            assert_eq!(
                scheduler.adopt_fresh_level1_type9_selected(&mut manager),
                Err(expected)
            );
            assert_eq!(
                scheduler
                    .owners
                    .iter()
                    .map(|owner| (owner.entity_id(), owner.family()))
                    .collect::<Vec<_>>(),
                before_owners
            );
            assert_eq!(
                manager
                    .fresh_level1_type9_initial_productions()
                    .iter()
                    .map(|owner| (owner.entity_id(), owner.successful_scheduler_branch()))
                    .collect::<Vec<_>>(),
                before_sidecars
            );
            assert_eq!(
                manager
                    .iter_all()
                    .map(|entity| (
                        entity.id,
                        entity.current_behavior_context,
                        entity.collision.state_flags_at_0x08,
                        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                        entity.actor_tasks.task_in_slot(ActorTaskSlot::Secondary),
                        entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary),
                    ))
                    .collect::<Vec<_>>(),
                before_tasks
            );
            // Restored sidecars remain authentic after removing the deliberate
            // conflict; retrying the uncorrupted transaction adopts all of them.
            if !corrupt_graph {
                scheduler.owners.pop();
                let expected = before_sidecars
                    .iter()
                    .filter(|(_, branch)| branch.is_some())
                    .count();
                assert_eq!(
                    scheduler.adopt_fresh_level1_type9_selected(&mut manager),
                    Ok(expected)
                );
            }
        }
    }

    #[test]
    fn selected_type9_adoption_preserves_branch_order_and_authenticates_atomically() {
        let mut manager = exact_level_one_type9_selected_mixed_manager();
        let expected = manager
            .fresh_level1_type9_initial_productions()
            .iter()
            .filter_map(|owner| {
                owner
                    .successful_scheduler_branch()
                    .map(|branch| (owner.entity_id(), branch))
            })
            .collect::<Vec<_>>();
        assert!(expected
            .iter()
            .any(|(_, branch)| { *branch == FreshLevel1Type9PublishedSchedulerBranch::Wander }));
        assert!(expected
            .iter()
            .any(|(_, branch)| { *branch != FreshLevel1Type9PublishedSchedulerBranch::Wander }));

        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(
            scheduler.adopt_fresh_level1_type9_selected(&mut manager),
            Ok(expected.len())
        );
        assert_eq!(
            scheduler
                .owners
                .iter()
                .map(|owner| (owner.entity_id(), owner.family()))
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(|(entity_id, branch)| (
                    *entity_id,
                    match branch {
                        FreshLevel1Type9PublishedSchedulerBranch::RunAway => {
                            SpecializedActorTaskFamily::OrdinaryType9RunAway
                        }
                        FreshLevel1Type9PublishedSchedulerBranch::AttractAttention => {
                            SpecializedActorTaskFamily::OrdinaryType9AttractAttention
                        }
                        FreshLevel1Type9PublishedSchedulerBranch::GoToJob => {
                            SpecializedActorTaskFamily::OrdinaryType9GoToJob
                        }
                        FreshLevel1Type9PublishedSchedulerBranch::Wander => {
                            SpecializedActorTaskFamily::OrdinaryType9Wander
                        }
                    },
                ))
                .collect::<Vec<_>>()
        );
        assert!(manager
            .fresh_level1_type9_initial_productions()
            .iter()
            .all(|owner| owner.successful_scheduler_branch().is_none()));

        let mut corrupted = exact_level_one_type9_selected_mixed_manager();
        let corrupt_id = corrupted
            .fresh_level1_type9_initial_productions()
            .iter()
            .find(|owner| owner.successful_wander_selection().is_some())
            .map(FreshLevel1Type9InitialProductionOwner::entity_id)
            .unwrap();
        corrupted
            .entity_mut_for_test(corrupt_id)
            .unwrap()
            .current_behavior_context = RetailRuntimeValue::Unresolved;
        let sidecars_before = corrupted
            .fresh_level1_type9_initial_productions()
            .iter()
            .map(FreshLevel1Type9InitialProductionOwner::entity_id)
            .collect::<Vec<_>>();
        let mut rejected = SpecializedActorTaskScheduler::new();
        assert_eq!(
            rejected.adopt_fresh_level1_type9_selected(&mut corrupted),
            Err(
                OrdinaryType9SelectedLoadAdoptionError::OwnerAuthenticationFailed {
                    entity_id: corrupt_id,
                }
            )
        );
        assert!(rejected.is_empty());
        assert_eq!(
            corrupted
                .fresh_level1_type9_initial_productions()
                .iter()
                .map(FreshLevel1Type9InitialProductionOwner::entity_id)
                .collect::<Vec<_>>(),
            sidecars_before
        );
    }

    /// Same explicit E370/tail inputs as the owner-production fixtures.
    ///
    /// Exact births keep construction-time surface bits outside the
    /// pre-publication mask and leave `+0x48` unresolved until a spawn
    /// constructor or this tail precondition writes them. That is not
    /// constructor residue: the live spawn path already publishes
    /// `Known(0)` at `+0x48`, and surface bits come from `FUN_004129B0`.
    fn prepare_selected_owner_for_post_f70_outer_tail(entity: &mut Entity) {
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(BODY_BASIS_REBUILT_STATE_BIT, 0);
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(0);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(SURFACE_STATE_MASK, FULLY_ABOVE_SURFACE_STATE_BIT);
    }

    #[test]
    fn selected_go_to_job_ticks_in_live_order_and_publishes_post_task_basis() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let mut manager = exact_level_one_type9_go_to_job_manager();
        let go_to_job_id = manager
            .fresh_level1_type9_initial_productions()
            .iter()
            .find(|owner| owner.successful_go_to_job_selection().is_some())
            .map(FreshLevel1Type9InitialProductionOwner::entity_id)
            .unwrap();
        prepare_selected_owner_for_post_f70_outer_tail(
            manager.entity_mut_for_test(go_to_job_id).unwrap(),
        );

        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .adopt_fresh_level1_type9_selected(&mut manager)
            .unwrap();
        let mut resources = strict_resources(flat_terrain());
        let mut world_fx = WorldFx::new();
        let mut words = std::iter::repeat(0);
        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                1,
                1,
            ),
            &mut || words.next().unwrap(),
        );
        assert_unblocked_without_surface_bubbles(&pass);
        let outcome = pass
            .outcomes
            .iter()
            .find(|outcome| outcome.entity_id() == go_to_job_id)
            .unwrap();
        assert!(
            matches!(
                outcome,
                SpecializedActorTaskProductionOutcome::OrdinaryType9GoToJob(
                    OrdinaryType9GoToJobProductionOutcome::PostBasisTailPending { .. }
                ) | SpecializedActorTaskProductionOutcome::OrdinaryType9GoToJob(
                    OrdinaryType9GoToJobProductionOutcome::RootWanderPublished { .. }
                )
            ),
            "unexpected selected Go-To-Job production outcome: {outcome:?}"
        );
        let entity = manager.entity_mut_for_test(go_to_job_id).unwrap();
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0),
            "the bounded owner completes the shared outer tail and clears +0xB2 in the same visit"
        );
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT),
            RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
        );
    }

    #[test]
    fn type9_presentation_switches_next_visit_detail_without_advancing_animation() {
        use crate::entity_view_detail::{RetailViewDetail, VIEW_DETAIL_STATE_MASK};

        let mut manager = exact_level_one_type9_wander_manager();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .adopt_fresh_level1_type9_selected(&mut manager)
            .unwrap();
        let lease = scheduler.actor_animation_claims().next().unwrap();
        let entity = manager.entity_mut_for_test(lease.entity_id).unwrap();
        let origin = entity.position_raw().map(i32::from);
        let before_animation = entity.actor_animation_runtime;
        let before_other_flags = entity
            .collision
            .state_flags_at_0x08
            .masked(!VIEW_DETAIL_STATE_MASK);
        let far = RetailViewDetailContext::from_raw(
            [origin[0] + 0x4000, origin[1], origin[2]],
            0,
            (52, 30),
        );
        let near = RetailViewDetailContext::from_raw(
            [origin[0], origin[1], origin[2] - 0x100],
            0,
            (52, 30),
        );

        for (context, detail) in [
            (far, RetailViewDetail::Coarse),
            (near, RetailViewDetail::Full),
            (far, RetailViewDetail::Coarse),
        ] {
            scheduler.publish_presented_view_detail(&mut manager, context);
            let entity = manager.entity_mut_for_test(lease.entity_id).unwrap();
            assert_eq!(
                entity
                    .collision
                    .state_flags_at_0x08
                    .masked(VIEW_DETAIL_STATE_MASK),
                RetailRuntimeValue::Known(detail.state_bits())
            );
            assert_eq!(
                entity
                    .collision
                    .state_flags_at_0x08
                    .masked(!VIEW_DETAIL_STATE_MASK),
                before_other_flags
            );
            assert_eq!(
                entity.actor_animation_runtime, before_animation,
                "presentation only selects next simulation mode"
            );
            let claims: Vec<_> = scheduler.actor_animation_claims().collect();
            assert!(
                claims.contains(&lease),
                "coarse and detailed callbacks both own the Sub-I decision"
            );
            manager.advance_unclaimed_actor_animations(20_000, &claims);
            assert_eq!(
                manager
                    .entity_mut_for_test(lease.entity_id)
                    .unwrap()
                    .actor_animation_runtime,
                before_animation,
                "fallback must not animate a coarse or already-visited owner"
            );
        }
    }

    #[test]
    fn type9_presentation_respects_allocation_lifetime_and_classification_gate() {
        use crate::entity_view_detail::VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT;

        let mut original = exact_level_one_type9_wander_manager();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .adopt_fresh_level1_type9_selected(&mut original)
            .unwrap();
        let lease = scheduler.actor_animation_claims().next().unwrap();
        let mut replacement = exact_level_one_type9_wander_manager();
        let entity = replacement.entity_mut_for_test(lease.entity_id).unwrap();
        let position = entity.position_raw().map(i32::from);
        let before = entity.collision.state_flags_at_0x08;
        let far = RetailViewDetailContext::from_raw(
            [position[0] + 0x4000, position[1], position[2]],
            0,
            (52, 30),
        );
        scheduler.publish_presented_view_detail(&mut replacement, far);
        assert_eq!(
            replacement
                .entity_mut_for_test(lease.entity_id)
                .unwrap()
                .collision
                .state_flags_at_0x08,
            before,
            "same entity id in a new manager is not the claimed allocation"
        );

        for gate in [RetailRuntimeValue::Known(0), RetailRuntimeValue::Unresolved] {
            let entity = original.entity_mut_for_test(lease.entity_id).unwrap();
            match gate {
                RetailRuntimeValue::Known(value) => entity
                    .collision
                    .state_flags_at_0x08
                    .overwrite(VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT, value),
                RetailRuntimeValue::Unresolved => entity
                    .collision
                    .state_flags_at_0x08
                    .invalidate(VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT),
            }
            let before = entity.collision.state_flags_at_0x08;
            scheduler.publish_presented_view_detail(&mut original, far);
            assert_eq!(
                original
                    .entity_mut_for_test(lease.entity_id)
                    .unwrap()
                    .collision
                    .state_flags_at_0x08,
                before
            );
        }
        let entity = original.entity_mut_for_test(lease.entity_id).unwrap();
        entity.active = false;
        entity.collision.state_flags_at_0x08.overwrite(
            VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT,
            VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT,
        );
        let before = entity.collision.state_flags_at_0x08;
        scheduler.publish_presented_view_detail(&mut original, far);
        assert_eq!(
            original
                .entity_mut_for_test(lease.entity_id)
                .unwrap()
                .collision
                .state_flags_at_0x08,
            before
        );
    }

    #[test]
    fn selected_go_to_job_animation_claim_and_main_base_round_trip_are_linear() {
        let mut manager = exact_level_one_type9_go_to_job_manager();
        let go_to_job_id = manager
            .fresh_level1_type9_initial_productions()
            .iter()
            .find(|owner| owner.successful_go_to_job_selection().is_some())
            .map(FreshLevel1Type9InitialProductionOwner::entity_id)
            .unwrap();
        manager
            .entity_mut_for_test(go_to_job_id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            );
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .adopt_fresh_level1_type9_selected(&mut manager)
            .unwrap();
        let lease = manager
            .ordinary_type9_selected_actor_lease(go_to_job_id)
            .unwrap();
        assert!(scheduler
            .actor_animation_claims()
            .any(|claim| claim == lease));

        let owner = scheduler
            .take_ordinary_type9_selected_for_main_base_abort(lease)
            .unwrap()
            .unwrap();
        assert!(matches!(
            owner,
            OrdinaryType9SelectedProductionOwner::GoToJob(_)
        ));
        let (index, initial_owner, resume) = owner.decompose_for_main_base_abort();
        manager
            .restore_fresh_level1_type9_initial_production(index, initial_owner)
            .unwrap();
        let (restored_index, restored_initial_owner) = manager
            .take_fresh_level1_type9_initial_production_for_entity(go_to_job_id)
            .unwrap();
        let resumed = resume
            .resume_after_main_base_abort_noop(restored_index, restored_initial_owner)
            .unwrap();
        assert!(scheduler
            .restore_ordinary_type9_selected_after_main_base_abort_noop(resumed)
            .unwrap()
            .is_none());
        assert_eq!(scheduler.registered_len(), 1);
    }

    #[test]
    fn selected_wander_adoption_tick_and_main_base_round_trip_are_linear() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let mut manager = exact_level_one_type9_wander_manager();
        let wander_id = manager
            .fresh_level1_type9_initial_productions()
            .iter()
            .find(|owner| owner.successful_wander_selection().is_some())
            .map(FreshLevel1Type9InitialProductionOwner::entity_id)
            .expect("the exact fixture publishes Wander");
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(
            scheduler.adopt_fresh_level1_type9_selected(&mut manager),
            Ok(1)
        );
        assert_eq!(
            scheduler
                .owners
                .iter()
                .map(SpecializedActorTaskOwner::family)
                .collect::<Vec<_>>(),
            [SpecializedActorTaskFamily::OrdinaryType9Wander]
        );
        let entity = manager.entity_mut_for_test(wander_id).unwrap();
        prepare_selected_owner_for_post_f70_outer_tail(entity);
        let lease = manager
            .ordinary_type9_selected_actor_lease(wander_id)
            .unwrap();
        assert!(scheduler
            .actor_animation_claims()
            .any(|claim| claim == lease));

        let owner = scheduler
            .take_ordinary_type9_selected_for_main_base_abort(lease)
            .unwrap()
            .unwrap();
        assert!(matches!(
            owner,
            OrdinaryType9SelectedProductionOwner::Wander(_)
        ));
        let (index, initial_owner, resume) = owner.decompose_for_main_base_abort();
        manager
            .restore_fresh_level1_type9_initial_production(index, initial_owner)
            .unwrap();
        let (restored_index, restored_initial_owner) = manager
            .take_fresh_level1_type9_initial_production_for_entity(wander_id)
            .unwrap();
        let resumed = resume
            .resume_after_main_base_abort_noop(restored_index, restored_initial_owner)
            .unwrap();
        assert!(scheduler
            .restore_ordinary_type9_selected_after_main_base_abort_noop(resumed)
            .unwrap()
            .is_none());

        let mut resources = strict_resources(flat_terrain());
        let mut world_fx = WorldFx::new();
        let mut words = std::iter::repeat(0);
        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                1,
                1,
            ),
            &mut || words.next().unwrap(),
        );
        assert_unblocked_without_surface_bubbles(&pass);
        let outcome = pass
            .outcomes
            .iter()
            .find(|outcome| outcome.entity_id() == wander_id)
            .unwrap();
        assert!(
            matches!(
                outcome,
                SpecializedActorTaskProductionOutcome::OrdinaryType9Wander(
                    OrdinaryType9WanderProductionOutcome::PostBasisTailPending { .. }
                ) | SpecializedActorTaskProductionOutcome::OrdinaryType9Wander(
                    OrdinaryType9WanderProductionOutcome::RootWanderPublished { .. }
                )
            ),
            "unexpected selected Wander production outcome: {outcome:?}"
        );
        let entity = manager.entity_mut_for_test(wander_id).unwrap();
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0),
            "the bounded owner completes the shared outer tail and clears +0xB2 in the same visit"
        );
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT),
            RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
        );
    }

    #[test]
    fn player_contact_custody_authenticates_all_actors_before_consuming_completed_visits() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let mut manager = exact_level_one_type9_wander_manager();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(
            scheduler.adopt_fresh_level1_type9_selected(&mut manager),
            Ok(1)
        );
        let wander_id = scheduler.owners[0].entity_id();
        prepare_selected_owner_for_post_f70_outer_tail(
            manager.entity_mut_for_test(wander_id).unwrap(),
        );
        let mut resources = strict_resources(flat_terrain());
        let mut world_fx = WorldFx::new();
        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                1,
                1,
            ),
            &mut || 0,
        );
        assert_unblocked_without_surface_bubbles(&pass);
        let SpecializedActorTaskOwner::OrdinaryType9Wander(owner) = &scheduler.owners[0] else {
            panic!("fixture retains Wander")
        };
        assert!(matches!(
            owner.state(),
            crate::ordinary_type9_wander_production::OrdinaryType9WanderProductionState::PostBasisTailPending { .. }
        ));
        assert!(owner.completed_visit_lease(&manager).is_some());
        let owner_before = format!("{scheduler:?}");
        let entity = manager.entity_mut_for_test(wander_id).unwrap();
        let basis_before = entity.physical_body_basis_q31;
        let rotation_before = entity.rotation_heading_pitch_roll_raw();

        assert_eq!(
            scheduler.consume_type9_player_contact_visits(&manager, &[wander_id, u32::MAX]),
            Err(u32::MAX)
        );
        assert_eq!(
            format!("{scheduler:?}"),
            owner_before,
            "a later absent owner must not consume the first completed receipt"
        );

        assert_eq!(
            scheduler.consume_type9_player_contact_visits(&manager, &[wander_id]),
            Ok(())
        );
        let SpecializedActorTaskOwner::OrdinaryType9Wander(owner) = &scheduler.owners[0] else {
            panic!("contact keeps the same task family")
        };
        assert_eq!(
            owner.state(),
            crate::ordinary_type9_wander_production::OrdinaryType9WanderProductionState::Active
        );
        let entity = manager.entity_mut_for_test(wander_id).unwrap();
        assert_eq!(entity.physical_body_basis_q31, basis_before);
        assert_eq!(entity.rotation_heading_pitch_roll_raw(), rotation_before);
    }

    #[test]
    fn selected_wander_adoption_rejects_tampered_mint_state_atomically() {
        let mut manager = exact_level_one_type9_wander_manager();
        let wander_id = manager
            .fresh_level1_type9_initial_productions()
            .iter()
            .find(|owner| owner.successful_wander_selection().is_some())
            .map(FreshLevel1Type9InitialProductionOwner::entity_id)
            .unwrap();
        let wander_visit = manager
            .fresh_level1_type9_initial_productions()
            .iter()
            .find(|owner| owner.entity_id() == wander_id)
            .and_then(FreshLevel1Type9InitialProductionOwner::successful_wander_live_owner)
            .map(|owner| owner.visit())
            .unwrap();
        let entity = manager.entity_mut_for_test(wander_id).unwrap();
        let Some(ActorTaskRuntime::OrdinaryType9Wander(task)) =
            entity.actor_tasks.task_state_mut(wander_visit.task_id)
        else {
            panic!("the exact fixture retains its Wander Primary")
        };
        task.before_callback(1_000);
        let sidecars_before = manager
            .fresh_level1_type9_initial_productions()
            .iter()
            .map(FreshLevel1Type9InitialProductionOwner::entity_id)
            .collect::<Vec<_>>();

        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(
            scheduler.adopt_fresh_level1_type9_selected(&mut manager),
            Err(
                OrdinaryType9SelectedLoadAdoptionError::OwnerAuthenticationFailed {
                    entity_id: wander_id,
                }
            )
        );
        assert!(scheduler.is_empty());
        assert_eq!(
            manager
                .fresh_level1_type9_initial_productions()
                .iter()
                .map(FreshLevel1Type9InitialProductionOwner::entity_id)
                .collect::<Vec<_>>(),
            sidecars_before
        );
    }

    #[test]
    fn selected_wander_adoption_rejects_constructor_speed_tamper_atomically() {
        let mut manager = exact_level_one_type9_wander_manager();
        let wander_id = manager
            .fresh_level1_type9_initial_productions()
            .iter()
            .find(|owner| owner.successful_wander_selection().is_some())
            .map(FreshLevel1Type9InitialProductionOwner::entity_id)
            .unwrap();
        let entity = manager.entity_mut_for_test(wander_id).unwrap();
        let RetailRuntimeValue::Known(Some(mut sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("the exact fixture retains its initialized Sub-A runtime")
        };
        sub_a.apply_go_to_job_reset(i32::MIN);
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(sub_a));
        let sidecars_before = manager
            .fresh_level1_type9_initial_productions()
            .iter()
            .map(FreshLevel1Type9InitialProductionOwner::entity_id)
            .collect::<Vec<_>>();

        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(
            scheduler.adopt_fresh_level1_type9_selected(&mut manager),
            Err(
                OrdinaryType9SelectedLoadAdoptionError::OwnerAuthenticationFailed {
                    entity_id: wander_id,
                }
            )
        );
        assert!(scheduler.is_empty());
        assert_eq!(
            manager
                .fresh_level1_type9_initial_productions()
                .iter()
                .map(FreshLevel1Type9InitialProductionOwner::entity_id)
                .collect::<Vec<_>>(),
            sidecars_before
        );
    }

    #[test]
    fn same_family_replaces_but_cross_family_claim_is_rejected() {
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .register_type17_common_dying(type17_owner(7, 0))
            .unwrap();
        scheduler
            .register_type17_common_dying(type17_owner(7, 1))
            .unwrap();
        assert_eq!(scheduler.registered_len(), 1);

        let rejected = type47_owner(7, 0);
        assert_eq!(
            scheduler.register_type47_common_dying(rejected),
            Err(SpecializedActorTaskRegistrationFailure {
                conflict: SpecializedActorTaskRegistrationConflict {
                    entity_id: 7,
                    existing: SpecializedActorTaskFamily::Type17CommonDying,
                    attempted: SpecializedActorTaskFamily::Type47CommonDying,
                },
                rejected,
            })
        );
        assert_eq!(scheduler.registered_len(), 1);

        let (type54_entity, task_id) = exact_type54_sea_level_owner_fixture(7, -1_000);
        let manager = EntityManager::from_entities_for_test(vec![type54_entity]);
        let receipt = issue_exact_type54_sea_level_owner_receipt(&manager, 7, task_id);
        let failure = scheduler
            .register_main_base_type54_sea_level(receipt)
            .expect_err("the linear Type-54 authority cannot replace another family");
        assert_eq!(
            failure.conflict,
            SpecializedActorTaskRegistrationConflict {
                entity_id: 7,
                existing: SpecializedActorTaskFamily::Type17CommonDying,
                attempted: SpecializedActorTaskFamily::MainBaseType54SeaLevel,
            }
        );
        assert_eq!(failure.rejected.lease().actor().entity_id, 7);
        assert_eq!(failure.rejected.sequence(), 1);
        assert_eq!(scheduler.registered_len(), 1);
    }

    #[test]
    fn type9_registration_returns_displaced_and_rejected_linear_custody() {
        let (_manager, owner) = exact_type9_manager(9);
        let task_lease = owner.task_lease();
        let actor = task_lease.actor();
        let mut scheduler = SpecializedActorTaskScheduler::new();

        assert!(scheduler
            .register_main_base_type9_exploding(task_lease)
            .unwrap()
            .is_none());
        let displaced = scheduler
            .register_main_base_type9_exploding(task_lease)
            .unwrap()
            .expect("same-family replacement returns its non-copyable owner");
        assert_eq!(displaced.task_lease(), task_lease);
        assert_eq!(
            scheduler.main_base_type9_actor_claims().collect::<Vec<_>>(),
            [actor]
        );
        assert_eq!(scheduler.registered_len(), 1);

        let mut conflicting = SpecializedActorTaskScheduler::new();
        conflicting
            .register_type17_common_dying(type17_owner(9, 0))
            .unwrap();
        let failure = conflicting
            .register_main_base_type9_exploding(task_lease)
            .expect_err("a Type-9 owner cannot hide an existing family claim");
        assert_eq!(
            failure.conflict,
            SpecializedActorTaskRegistrationConflict {
                entity_id: 9,
                existing: SpecializedActorTaskFamily::Type17CommonDying,
                attempted: SpecializedActorTaskFamily::MainBaseType9Exploding,
            }
        );
        assert_eq!(failure.rejected.task_lease(), task_lease);
        assert!(conflicting.main_base_type9_actor_claims().next().is_none());
        assert_eq!(conflicting.registered_len(), 1);
    }

    #[test]
    fn repeated_main_base_type9_sweep_does_not_retake_consumed_run_away_custody() {
        let (_manager, owner) = exact_type9_manager(9);
        let task_lease = owner.task_lease();
        let actor = task_lease.actor();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .register_main_base_type9_exploding(task_lease)
            .unwrap();

        assert_eq!(
            scheduler.take_ordinary_type9_run_away_for_main_base_abort(actor),
            Ok(None)
        );
        assert_eq!(scheduler.registered_len(), 1);
        assert_eq!(
            scheduler.actor_animation_claims().collect::<Vec<_>>(),
            [actor]
        );
        let stale = MainBaseAbortActorLease {
            entity_id: actor.entity_id,
            allocation_identity: actor.allocation_identity.wrapping_add(1),
        };
        assert_eq!(
            scheduler.take_ordinary_type9_run_away_for_main_base_abort(stale),
            Err(OrdinaryType9RunAwayCustodyTakeBlock::ActorLeaseMismatch {
                expected: actor,
                actual: stale,
            })
        );
        assert_eq!(scheduler.registered_len(), 1);
    }

    #[test]
    fn type66_registration_returns_displaced_and_rejected_linear_custody() {
        let (manager, owner) = exact_type66_post_abort_owner();
        let task_lease = owner.task_lease();
        let entity_id = owner.entity_id();
        let mut scheduler = SpecializedActorTaskScheduler::new();

        assert!(scheduler
            .register_main_base_type66_production(task_lease)
            .unwrap()
            .is_none());
        let displaced = scheduler
            .register_main_base_type66_production(task_lease)
            .unwrap()
            .expect("same-family replacement returns its non-copyable owner");
        assert_eq!(displaced.task_lease(), task_lease);
        assert_eq!(scheduler.registered_len(), 1);

        let mut conflicting = SpecializedActorTaskScheduler::new();
        conflicting
            .register_type17_common_dying(type17_owner(entity_id, 0))
            .unwrap();
        let failure = conflicting
            .register_main_base_type66_production(task_lease)
            .expect_err("a Type-66 owner cannot hide an existing family claim");
        assert_eq!(
            failure.conflict,
            SpecializedActorTaskRegistrationConflict {
                entity_id,
                existing: SpecializedActorTaskFamily::Type17CommonDying,
                attempted: SpecializedActorTaskFamily::MainBaseType66Production,
            }
        );
        assert_eq!(failure.rejected.task_lease(), task_lease);
        assert_eq!(conflicting.registered_len(), 1);

        let mut type66_first = SpecializedActorTaskScheduler::new();
        type66_first
            .register_main_base_type66_production(task_lease)
            .unwrap();
        let rejected = type17_owner(entity_id, 1);
        assert_eq!(
            type66_first.register_type17_common_dying(rejected),
            Err(SpecializedActorTaskRegistrationFailure {
                conflict: SpecializedActorTaskRegistrationConflict {
                    entity_id,
                    existing: SpecializedActorTaskFamily::MainBaseType66Production,
                    attempted: SpecializedActorTaskFamily::Type17CommonDying,
                },
                rejected,
            })
        );
        assert_eq!(type66_first.registered_len(), 1);
        assert!(type66_first.main_base_type9_actor_claims().next().is_none());
        drop(manager);
        type66_first.clear_after_manager_reset();
        assert!(type66_first.is_empty());
    }

    #[test]
    fn same_family_type54_replacement_returns_the_displaced_linear_receipt() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let (entity, task_id) = exact_type54_sea_level_owner_fixture(54, -1_000);
        let mut manager = EntityManager::from_entities_for_test(vec![entity]);
        let first = issue_exact_type54_sea_level_owner_receipt(&manager, 54, task_id);
        let lease = first.lease();
        let replacement = MainBaseType54SeaLevelTickReceipt::issue(
            lease.actor(),
            lease.task_id(),
            first.sequence() + 1,
        );
        let mut scheduler = SpecializedActorTaskScheduler::new();

        assert!(scheduler
            .register_main_base_type54_sea_level(first)
            .unwrap()
            .is_none());
        let displaced = scheduler
            .register_main_base_type54_sea_level(replacement)
            .unwrap()
            .expect("same-family replacement must return the old authority");
        assert_eq!(displaced.lease(), lease);
        assert_eq!(displaced.sequence(), 1);
        assert_eq!(scheduler.registered_len(), 1);

        let mut resources = strict_resources(flat_terrain());
        let mut world_fx = WorldFx::new();
        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                20_000,
                20_000,
            ),
            &mut || panic!("a stale Type-54 sequence consumes no shared RNG"),
        );
        assert_eq!(
            pass.outcomes,
            [
                SpecializedActorTaskProductionOutcome::MainBaseType54SeaLevel(
                    MainBaseType54SeaLevelProductionOutcome::Dropped {
                        entity_id: 54,
                        reason: MainBaseType54SeaLevelTickBlock::ReceiptSequenceMismatch {
                            expected: 1,
                            actual: 2,
                        },
                    },
                ),
            ]
        );
        assert_unblocked_without_surface_bubbles(&pass);
        assert!(!pass.sea_level_changed);
        assert_eq!(scheduler.registered_len(), 0);
    }

    #[test]
    fn selected_attract_attention_even_and_odd_adoption_and_main_base_handoff_are_linear() {
        for odd_parity in [false, true] {
            let mut manager = exact_level_one_type9_attract_attention_manager(odd_parity);
            let entity_id = manager
                .fresh_level1_type9_initial_productions()
                .first()
                .map(FreshLevel1Type9InitialProductionOwner::entity_id)
                .expect("exact Attract fixture retains one owner");
            let expected_slots = if odd_parity { 3 } else { 2 };
            assert_eq!(
                manager
                    .fresh_level1_type9_initial_productions()
                    .first()
                    .unwrap()
                    .task_visits()
                    .into_iter()
                    .flatten()
                    .count(),
                expected_slots
            );

            let mut scheduler = SpecializedActorTaskScheduler::new();
            assert_eq!(
                scheduler.adopt_fresh_level1_type9_selected(&mut manager),
                Ok(1)
            );
            assert!(manager.fresh_level1_type9_initial_productions().is_empty());
            assert_eq!(
                scheduler
                    .owners
                    .iter()
                    .map(SpecializedActorTaskOwner::family)
                    .collect::<Vec<_>>(),
                [SpecializedActorTaskFamily::OrdinaryType9AttractAttention]
            );
            let lease = manager
                .ordinary_type9_selected_actor_lease(entity_id)
                .expect("selected Attract actor remains observable");
            assert!(scheduler
                .actor_animation_claims()
                .any(|claim| claim == lease));

            let owner = scheduler
                .take_ordinary_type9_selected_for_main_base_abort(lease)
                .unwrap()
                .expect("initial Attract graph is Main Base compatible");
            assert!(matches!(
                owner,
                OrdinaryType9SelectedProductionOwner::AttractAttention(_)
            ));
            let (index, initial_owner, resume) = owner.decompose_for_main_base_abort();
            assert_eq!(
                initial_owner.task_visits().into_iter().flatten().count(),
                expected_slots,
                "inverse composition retains the exact one/two-slot graph"
            );
            let resumed = resume
                .resume_after_main_base_abort_noop(index, initial_owner)
                .expect("exact Attract sidecar resumes without reminting owners");
            assert!(scheduler
                .restore_ordinary_type9_selected_after_main_base_abort_noop(resumed)
                .unwrap()
                .is_none());
            assert_eq!(scheduler.registered_len(), 1);
        }
    }

    #[test]
    fn later_type47_surface_phase_observes_type54s_inline_sea_change() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let mut terrain = flat_terrain();
        terrain.header[0] = 46 << 8;
        let mut resources = strict_resources(terrain);
        let mut world_fx = WorldFx::new();
        let (type54_entity, task_id) = exact_type54_sea_level_owner_fixture(54, -1_000);
        let (type47_entity, type47_metadata, type47_owner) =
            exact_type47_single_owner_fixture(47, 11);
        let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 48];
        metadata[47] = type47_metadata;
        let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![type54_entity, type47_entity],
            metadata,
            true,
        );
        manager
            .entity_mut_for_test(47)
            .unwrap()
            .surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(1_980);
        let type54_receipt = issue_exact_type54_sea_level_owner_receipt(&manager, 54, task_id);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .register_type47_common_dying(type47_owner)
            .unwrap();
        assert!(scheduler
            .register_main_base_type54_sea_level(type54_receipt)
            .unwrap()
            .is_none());
        let mut draws = 0;

        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                20_000,
                20_000,
            ),
            &mut || {
                draws += 1;
                0
            },
        );

        assert_eq!(
            pass.outcomes
                .iter()
                .map(|outcome| (outcome.family(), outcome.entity_id()))
                .collect::<Vec<_>>(),
            [
                (SpecializedActorTaskFamily::MainBaseType54SeaLevel, 54),
                (SpecializedActorTaskFamily::Type47CommonDying, 47),
            ]
        );
        assert_eq!(draws, 2);
        let terrain = resources.level_terrain().unwrap();
        assert_eq!(terrain.header[0], (46 << 8) - 244);
        assert_eq!(terrain.sea_level_raw(), 45);
        assert_eq!(
            manager
                .iter_all()
                .find(|entity| entity.id == 47)
                .unwrap()
                .surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(1_960),
            "Type 47 must see the post-Type-54 sea level; the entry value 46 would take the deep lifecycle and publish 2,000 ms",
        );
        assert_eq!(scheduler.registered_len(), 1);
        assert_unblocked_without_surface_bubbles(&pass);
        assert!(pass.sea_level_changed);
    }

    #[test]
    fn one_live_snapshot_orders_families_and_reports_stale_receipts_last() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let mut resources = strict_resources(flat_terrain());
        let mut world_fx = WorldFx::new();
        let mut manager = EntityManager::from_entities_for_test(vec![
            Entity::unresolved_port_entity(47, EntityKind::Enemy, 47),
            Entity::unresolved_port_entity(17, EntityKind::Enemy, 17),
        ]);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .register_type17_common_dying(type17_owner(17, 0))
            .unwrap();
        scheduler
            .register_type47_common_dying(type47_owner(99, 0))
            .unwrap();
        scheduler
            .register_type47_common_dying(type47_owner(47, 0))
            .unwrap();

        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                20_000,
                20_000,
            ),
            &mut || panic!("unauthenticated fixtures must stop before shared RNG"),
        );

        assert_eq!(
            pass.outcomes
                .iter()
                .map(|outcome| (outcome.family(), outcome.entity_id()))
                .collect::<Vec<_>>(),
            [
                (SpecializedActorTaskFamily::Type47CommonDying, 47),
                (SpecializedActorTaskFamily::Type17CommonDying, 17),
                (SpecializedActorTaskFamily::Type47CommonDying, 99),
            ]
        );
        assert_eq!(scheduler.registered_len(), 2);
        assert_unblocked_without_surface_bubbles(&pass);
        assert!(!pass.sea_level_changed);
    }

    #[test]
    fn one_live_snapshot_orders_all_five_families_and_reports_stale_last() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let (mut manager, type66_owner) = exact_type66_post_abort_owner();
        let type66_lease = type66_owner.task_lease();
        let type66_id = type66_owner.entity_id();
        let other_ids = manager
            .retail_live_order_ids()
            .filter(|entity_id| *entity_id != type66_id)
            .take(4)
            .collect::<Vec<_>>();
        assert_eq!(other_ids.len(), 4);
        let [type17_id, type47_id, type54_id, type9_id] = other_ids.as_slice() else {
            unreachable!()
        };
        let type54_actor = manager
            .main_base_abort_actor_observation(*type54_id)
            .unwrap()
            .lease;
        let type9_actor = manager
            .main_base_abort_actor_observation(*type9_id)
            .unwrap()
            .lease;
        let type54_visit = task_visit(4);
        let type9_visit = task_visit(5);
        let stale_id = u32::MAX;

        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .register_type47_common_dying(type47_owner(stale_id, 0))
            .unwrap();
        scheduler
            .register_main_base_type66_production(type66_lease)
            .unwrap();
        scheduler
            .register_main_base_type9_exploding(MainBaseType9ExplodingTaskLease::issue(
                type9_actor,
                type9_visit.task_id,
            ))
            .unwrap();
        scheduler
            .register_type47_common_dying(type47_owner(*type47_id, 0))
            .unwrap();
        scheduler
            .register_main_base_type54_sea_level(MainBaseType54SeaLevelTickReceipt::issue(
                type54_actor,
                type54_visit.task_id,
                1,
            ))
            .unwrap();
        scheduler
            .register_type17_common_dying(type17_owner(*type17_id, 0))
            .unwrap();
        assert_eq!(scheduler.registered_len(), 6);
        assert_eq!(
            scheduler.main_base_type9_actor_claims().collect::<Vec<_>>(),
            [type9_actor],
            "Type-66 must never become a Type-9 Sub-I animation claim"
        );

        let tracked = [
            (*type17_id, SpecializedActorTaskFamily::Type17CommonDying),
            (*type47_id, SpecializedActorTaskFamily::Type47CommonDying),
            (
                *type54_id,
                SpecializedActorTaskFamily::MainBaseType54SeaLevel,
            ),
            (
                *type9_id,
                SpecializedActorTaskFamily::MainBaseType9Exploding,
            ),
            (
                type66_id,
                SpecializedActorTaskFamily::MainBaseType66Production,
            ),
        ];
        let mut expected = manager
            .retail_live_order_ids()
            .filter_map(|entity_id| {
                tracked
                    .iter()
                    .find(|(tracked_id, _)| *tracked_id == entity_id)
                    .map(|(_, family)| (*family, entity_id))
            })
            .collect::<Vec<_>>();
        expected.push((SpecializedActorTaskFamily::Type47CommonDying, stale_id));

        let mut resources = strict_type66_resources(vec![0x88]);
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let pass = scheduler.tick(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                1,
                1,
            ),
            &mut notifications,
        );

        assert_eq!(
            pass.outcomes
                .iter()
                .map(|outcome| (outcome.family(), outcome.entity_id()))
                .collect::<Vec<_>>(),
            expected
        );
        assert!(matches!(
            pass.outcomes.last(),
            Some(SpecializedActorTaskProductionOutcome::Type47CommonDying(
                Type47CommonDyingProductionOutcome::Dropped {
                    entity_id,
                    reason: Type47CommonDyingProductionDrop::EntityUnavailable,
                }
            )) if *entity_id == stale_id
        ));
        assert!(pass.explosion_lights.is_empty());
        assert_eq!(scheduler.registered_len(), 2);
    }

    #[test]
    fn exact_five_family_pass_uses_live_order_shared_owners_and_type66_effects() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let (mut manager, type66_owner) = exact_type66_post_abort_owner();
        let type66_lease = type66_owner.task_lease();
        let type66_id = type66_owner.entity_id();
        assert_eq!(type66_id, 18);
        let (type54, type9, type47, type17) = adopt_exact_other_families(&mut manager);
        set_type66_terminal_predecessor(&mut manager, type66_id, 0, 0, 3_000_001);
        manager
            .entity_mut_for_test(type66_id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(250);

        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .register_main_base_type66_production(type66_lease)
            .unwrap();
        scheduler.register_type17_common_dying(type17).unwrap();
        scheduler.register_type47_common_dying(type47).unwrap();
        scheduler.register_main_base_type9_exploding(type9).unwrap();
        scheduler
            .register_main_base_type54_sea_level(type54)
            .unwrap();

        let mut resources = strict_type66_resources(type66_effect_program(0x0200));
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let mut frame = production_frame(
            GameplayNotificationPhase::Playing,
            &mut resources,
            &mut world_fx,
            &mut static_damage,
            100_000,
            100_000,
        );
        frame.retail_tick = 250;

        let pass = scheduler.tick(&mut manager, frame, &mut notifications);

        assert_eq!(
            pass.outcomes
                .iter()
                .map(|outcome| (outcome.family(), outcome.entity_id()))
                .collect::<Vec<_>>(),
            [
                (SpecializedActorTaskFamily::MainBaseType54SeaLevel, 3),
                (SpecializedActorTaskFamily::MainBaseType9Exploding, 4),
                (SpecializedActorTaskFamily::Type47CommonDying, 6),
                (SpecializedActorTaskFamily::Type17CommonDying, 12),
                (
                    SpecializedActorTaskFamily::MainBaseType66Production,
                    type66_id,
                ),
            ]
        );
        assert!(matches!(
            pass.outcomes.last(),
            Some(SpecializedActorTaskProductionOutcome::MainBaseType66Production(
                MainBaseType66ProductionOutcome::Continuing {
                    notification: crate::main_base_type66_abort::WorkingFactoryNotificationOutcome::NotificationRequested,
                    under_attack_notification_requested: true,
                    model_effect_stages: 1,
                    accepted_effect_points: 1,
                    ..
                }
            ))
        ));
        assert_eq!(pass.explosion_lights.len(), 1);
        assert_eq!(pass.explosion_lights[0].radius_raw, 0x600);
        assert_eq!(
            scheduler.main_base_type9_actor_claims().collect::<Vec<_>>(),
            [type9.actor()]
        );
    }

    #[test]
    fn one_shared_rng_stream_follows_live_order_not_registration_order() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let mut resources = strict_resources(flat_terrain());
        let mut world_fx = WorldFx::new();
        let (type47_entity, type47_metadata, type47_owner) =
            exact_type47_single_owner_fixture(47, 11);
        let (type17_entity, type17_metadata, type17_owner) =
            coarse_type17_common_dying_composite_fixture(17, 17);
        let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 48];
        metadata[17] = type17_metadata;
        metadata[47] = type47_metadata;
        let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![type47_entity, type17_entity],
            metadata,
            true,
        );
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .register_type17_common_dying(type17_owner)
            .unwrap();
        scheduler
            .register_type47_common_dying(type47_owner)
            .unwrap();
        let mut samples = [0x0026, 0x1e27, 0x8000, 0x8000].into_iter();
        let mut draws = 0;

        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                1_000,
                1_000,
            ),
            &mut || {
                draws += 1;
                samples
                    .next()
                    .expect("the two scheduler prefixes consume four samples")
            },
        );

        assert_eq!(draws, 4);
        assert_eq!(
            pass.outcomes,
            [
                SpecializedActorTaskProductionOutcome::Type47CommonDying(
                    Type47CommonDyingProductionOutcome::SchedulerWaiting { entity_id: 47 },
                ),
                SpecializedActorTaskProductionOutcome::Type17CommonDying(
                    Type17CommonDyingProductionOutcome::SchedulerWaiting { entity_id: 17 },
                ),
            ]
        );
        assert_eq!(scheduler.registered_len(), 2);
        assert_unblocked_without_surface_bubbles(&pass);
        assert!(!pass.sea_level_changed);
        assert_eq!(
            manager
                .iter_all()
                .find(|entity| entity.id == 47)
                .unwrap()
                .collision
                .subject_scan_gate_at_0x70,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            manager
                .iter_all()
                .find(|entity| entity.id == 17)
                .unwrap()
                .collision
                .subject_scan_gate_at_0x70,
            RetailRuntimeValue::Known(1_000)
        );
    }

    #[test]
    fn type47_type9_type17_share_live_order_rng_independent_of_registration_order() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let mut resources = strict_resources(flat_terrain());
        let mut world_fx = WorldFx::new();
        let (type47_entity, type47_metadata, type47_owner) =
            exact_type47_single_owner_fixture(47, 11);
        let (type9_entity, type9_metadata, type9_task_id) =
            exact_main_base_type9_exploding_owner_fixture(9, [0; 3]);
        let (type17_entity, type17_metadata, type17_owner) =
            coarse_type17_common_dying_composite_fixture(17, 17);
        let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 48];
        metadata[LEVEL_ONE_TYPE9_ENTITY_TYPE as usize] = type9_metadata;
        metadata[17] = type17_metadata;
        metadata[47] = type47_metadata;
        let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![type47_entity, type9_entity, type17_entity],
            metadata,
            true,
        );
        let type9_owner = issue_exact_main_base_type9_exploding_owner(&manager, 9, type9_task_id);
        let type9_task_lease = type9_owner.task_lease();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .register_type17_common_dying(type17_owner)
            .unwrap();
        assert!(scheduler
            .register_main_base_type9_exploding(type9_task_lease)
            .unwrap()
            .is_none());
        scheduler
            .register_type47_common_dying(type47_owner)
            .unwrap();
        let mut samples = [
            0x0026, 0x1e27, // Type 47 scheduler.
            0x9000, 0, // Type 9 scheduler: admit the callback.
            0x9000, 0x7000, // Type 9 shared-retarget callback.
            0x8000, 0x8000, // Type 17 scheduler.
        ]
        .into_iter();
        let mut draws = 0;

        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                1_000,
                1_000,
            ),
            &mut || {
                draws += 1;
                samples
                    .next()
                    .expect("scheduler and task draws follow one live-order stream")
            },
        );

        assert_eq!(draws, 8);
        assert_eq!(
            pass.outcomes
                .iter()
                .map(|outcome| (outcome.family(), outcome.entity_id()))
                .collect::<Vec<_>>(),
            [
                (SpecializedActorTaskFamily::Type47CommonDying, 47),
                (SpecializedActorTaskFamily::MainBaseType9Exploding, 9),
                (SpecializedActorTaskFamily::Type17CommonDying, 17),
            ]
        );
        let SpecializedActorTaskProductionOutcome::MainBaseType9Exploding(
            MainBaseType9ExplodingProductionOutcome::Continuing {
                committed_prefix, ..
            },
        ) = &pass.outcomes[1]
        else {
            panic!("the middle Type-9 callback must continue")
        };
        assert_eq!(
            committed_prefix.retarget,
            SharedRetarget::Replaced {
                trigger: crate::shared_retarget_mover::SharedRetargetTrigger::NearTargetAxis,
                target_position_raw: [512, 0, -512],
            }
        );
        assert_eq!(
            manager
                .iter_all()
                .find(|entity| entity.id == 47)
                .unwrap()
                .collision
                .subject_scan_gate_at_0x70,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            manager
                .iter_all()
                .find(|entity| entity.id == 17)
                .unwrap()
                .collision
                .subject_scan_gate_at_0x70,
            RetailRuntimeValue::Known(1_000)
        );
        assert_eq!(scheduler.registered_len(), 3);
        assert_unblocked_without_surface_bubbles(&pass);
        assert!(!pass.sea_level_changed);
        assert_eq!(
            scheduler.main_base_type9_actor_claims().collect::<Vec<_>>(),
            [type9_task_lease.actor()]
        );
    }

    #[test]
    fn type9_reads_entity_basis_and_forwards_distinct_elapsed_clocks_without_observation_rows() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        const ELAPSED_MICROS: u32 = 20_000;
        const GLOBAL_ELAPSED_MICROS: u32 = 64_000;
        let basis = MainBaseType9CapturedProfile::for_spawn(9)
            .unwrap()
            .physical_body_basis_q31;
        let wrong_basis = MainBaseType9CapturedProfile::for_spawn(10)
            .unwrap()
            .physical_body_basis_q31;
        let (expected_outcome, expected_snapshot) =
            direct_type9_snapshot(basis, ELAPSED_MICROS, GLOBAL_ELAPSED_MICROS);
        let (_, aliased_clock_snapshot) =
            direct_type9_snapshot(basis, ELAPSED_MICROS, ELAPSED_MICROS);
        let (_, wrong_basis_snapshot) =
            direct_type9_snapshot(wrong_basis, ELAPSED_MICROS, GLOBAL_ELAPSED_MICROS);
        assert_ne!(
            expected_snapshot, aliased_clock_snapshot,
            "the process-global clock must not be aliased to wrapper elapsed"
        );
        assert_ne!(
            expected_snapshot, wrong_basis_snapshot,
            "the retained callback-entry basis must not be reconstructed from angles"
        );

        let mut resources = strict_resources(flat_terrain());
        let mut world_fx = WorldFx::new();
        let (mut manager, owner) = exact_type9_manager(9);
        manager
            .entity_mut_for_test(9)
            .expect("the exact Type-9 fixture remains live")
            .set_velocity_raw([8_192, 777, 0]);
        let task_lease = owner.task_lease();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler
            .register_main_base_type9_exploding(task_lease)
            .unwrap()
            .is_none());
        let mut words = [0xffff, 0, 0x9000, 0x7000].into_iter();
        let mut draws = 0;
        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                ELAPSED_MICROS,
                GLOBAL_ELAPSED_MICROS,
            ),
            &mut || {
                draws += 1;
                words
                    .next()
                    .expect("the scheduler and near retarget consume four words")
            },
        );

        assert_eq!(draws, 4, "outcomes: {:?}", pass.outcomes);
        assert_eq!(
            pass.outcomes,
            [SpecializedActorTaskProductionOutcome::MainBaseType9Exploding(expected_outcome,)]
        );
        let actual_snapshot = type9_runtime_snapshot(&manager, 9);
        assert_eq!(actual_snapshot.0, expected_snapshot.0);
        assert_eq!(actual_snapshot.2, expected_snapshot.2);
        assert_eq!(actual_snapshot.3, expected_snapshot.3);
        assert_eq!(actual_snapshot.4, expected_snapshot.4);
        assert_eq!(actual_snapshot.5, expected_snapshot.5);
        assert_ne!(
            actual_snapshot.1, expected_snapshot.1,
            "the complete owner must apply the post-task E870 drag/snap suffix"
        );
        assert_eq!(scheduler.registered_len(), 1);
        assert_unblocked_without_surface_bubbles(&pass);
        assert!(!pass.sea_level_changed);
    }

    #[test]
    fn type9_scheduler_wait_commits_only_the_prefix_and_retains_complete_custody() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        const ELAPSED_MICROS: u32 = 20_000;

        let mut resources = strict_resources(flat_terrain());
        let mut world_fx = WorldFx::new();
        let (mut manager, owner) = exact_type9_manager(9);
        let task_lease = owner.task_lease();
        {
            let entity = manager.entity_mut_for_test(9).unwrap();
            entity.set_motion_raw([1_234, 567, -890], [321, -654, 987]);
            entity.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(7);
            entity.collision.callback_scheduler_accumulator_us_at_0x6c =
                RetailRuntimeValue::Known(0);
            entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        }
        let before = {
            let entity = manager.iter_all().find(|entity| entity.id == 9).unwrap();
            let Some(ActorTaskRuntime::SharedRetarget(task)) =
                entity.actor_tasks.task_state(task_lease.task_id())
            else {
                panic!("the exact Type-9 task must remain published")
            };
            (
                entity.collision.state_flags_at_0x08,
                entity.collision.recent_relation_id_at_0x60,
                entity.position_raw(),
                entity.velocity_raw(),
                entity.rotation_heading_pitch_roll_raw(),
                entity.physical_body_basis_q31,
                entity.surface_lifetime_timer_ms_at_0x48,
                entity.mass_raw,
                *task,
            )
        };
        let mut words = [0xffff, 0xffff].into_iter();
        let mut draws = 0;
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler
            .register_main_base_type9_exploding(task_lease)
            .unwrap()
            .is_none());

        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                ELAPSED_MICROS,
                ELAPSED_MICROS,
            ),
            &mut || {
                draws += 1;
                words
                    .next()
                    .expect("the two scheduler gates consume exactly two words")
            },
        );

        assert_eq!(draws, 2);
        assert_eq!(
            pass.outcomes,
            [
                SpecializedActorTaskProductionOutcome::MainBaseType9Exploding(
                    MainBaseType9ExplodingProductionOutcome::SchedulerWaiting { entity_id: 9 },
                )
            ]
        );
        let entity = manager.iter_all().find(|entity| entity.id == 9).unwrap();
        assert_eq!(
            entity.collision.subject_scan_gate_at_0x70,
            RetailRuntimeValue::Known(ELAPSED_MICROS)
        );
        assert_eq!(
            entity.collision.callback_scheduler_accumulator_us_at_0x6c,
            RetailRuntimeValue::Known(ELAPSED_MICROS)
        );
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(7),
            "the callback-age clock advances only after the callback gate admits"
        );
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        let Some(ActorTaskRuntime::SharedRetarget(task)) =
            entity.actor_tasks.task_state(task_lease.task_id())
        else {
            panic!("a scheduler wait must retain the exact task")
        };
        assert_eq!(
            (
                entity.collision.state_flags_at_0x08,
                entity.collision.recent_relation_id_at_0x60,
                entity.position_raw(),
                entity.velocity_raw(),
                entity.rotation_heading_pitch_roll_raw(),
                entity.physical_body_basis_q31,
                entity.surface_lifetime_timer_ms_at_0x48,
                entity.mass_raw,
                *task,
            ),
            before,
            "the waiting visit must not enter the task or E870 suffix"
        );
        assert_eq!(scheduler.registered_len(), 1);
        assert_eq!(
            scheduler.main_base_type9_actor_claims().collect::<Vec<_>>(),
            [task_lease.actor()]
        );
        assert_unblocked_without_surface_bubbles(&pass);
        assert!(!pass.sea_level_changed);
    }

    #[test]
    fn terminal_type9_callback_still_runs_e870_before_disabled_master_motion() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        const MASTER_MOTION_ENABLE_STATE_BIT: u32 = 0x0004_0000;
        const GROUND_HEIGHT_RAW: i16 = -10 * 32;
        const INITIAL_POSITION_RAW: [i16; 3] = [4_096, 1_234, -4_096];

        let mut terrain = flat_terrain();
        for cell in &mut terrain.cells {
            cell.height = (-10_i8) as u8;
        }
        let mut resources = strict_resources(terrain);
        let mut world_fx = WorldFx::new();
        let (type9_entity, type9_metadata, task_id) =
            exact_main_base_type9_exploding_owner_fixture(9, INITIAL_POSITION_RAW);
        let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 48];
        metadata[LEVEL_ONE_TYPE9_ENTITY_TYPE as usize] = type9_metadata;
        let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![type9_entity],
            metadata,
            true,
        );
        let owner = issue_exact_main_base_type9_exploding_owner(&manager, 9, task_id);
        let task_lease = owner.task_lease();
        let initial_basis;
        {
            let entity = manager.entity_mut_for_test(9).unwrap();
            let Some(ActorTaskRuntime::SharedRetarget(task)) =
                entity.actor_tasks.task_state_mut(task_id)
            else {
                panic!("the exact Type-9 task must remain published")
            };
            task.before_callback(1_000_000);
            entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(1_000);
            entity.set_velocity_raw([8_192, 777, -4_096]);
            entity.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(7);
            entity.collision.callback_scheduler_accumulator_us_at_0x6c =
                RetailRuntimeValue::Known(125_001);
            entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(250_000);
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
            initial_basis = entity.physical_body_basis_q31;
        }
        let mut words = [0x9000, 0x7000].into_iter();
        let mut draws = 0;
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler
            .register_main_base_type9_exploding(task_lease)
            .unwrap()
            .is_none());

        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                0,
                125_000,
            ),
            &mut || {
                draws += 1;
                words
                    .next()
                    .expect("only the terminal task's X/Z retarget words are consumed")
            },
        );

        assert_eq!(draws, 2, "E370 must remain before its random-effect band");
        assert!(matches!(
            pass.outcomes.as_slice(),
            [SpecializedActorTaskProductionOutcome::MainBaseType9Exploding(
                MainBaseType9ExplodingProductionOutcome::Terminal(outcome)
            )] if outcome.entity_id == 9
        ));
        assert_eq!(scheduler.registered_len(), 0);
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), [9]);
        let entity = manager.iter_all().find(|entity| entity.id == 9).unwrap();
        assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_none());
        let RetailRuntimeValue::Known(state_flags) =
            entity.collision.state_flags_at_0x08.masked(u32::MAX)
        else {
            panic!("the terminal E870 suffix must retain an exact state word")
        };
        assert_ne!(state_flags & DEFERRED_DESTROY_PENDING_STATE_BIT, 0);
        assert_eq!(state_flags & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, 0);
        assert_eq!(state_flags & MASTER_MOTION_ENABLE_STATE_BIT, 0);
        assert_ne!(state_flags & BODY_BASIS_REBUILT_STATE_BIT, 0);
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0),
            "the terminal callback still reaches the unconditional post-callback clear"
        );
        assert_eq!(
            entity.position_raw(),
            [
                INITIAL_POSITION_RAW[0],
                GROUND_HEIGHT_RAW,
                INITIAL_POSITION_RAW[2],
            ],
            "ground snap runs at old X/Z, while disabled master motion cannot integrate"
        );
        assert_eq!(entity.velocity_raw()[1], 0);
        assert!(
            entity.velocity_raw()[0] != 0 || entity.velocity_raw()[2] != 0,
            "nonzero horizontal velocity makes the disabled master gate observable"
        );
        let [heading_raw, pitch_raw, roll_raw] = entity.rotation_heading_pitch_roll_raw();
        let expected_basis = Type9BodyBasis::from_angle_words(heading_raw, pitch_raw, roll_raw);
        assert_eq!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Known(expected_basis)
        );
        assert_ne!(
            entity.physical_body_basis_q31, initial_basis,
            "the terminal frame must publish the newly rebuilt basis"
        );
        assert_eq!(
            entity.surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(1_125),
            "the terminal callback uses the capped 125-ms carry and stays below E370 effects"
        );
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(125_007)
        );
        assert_eq!(
            entity.collision.callback_scheduler_accumulator_us_at_0x6c,
            RetailRuntimeValue::Known(1)
        );
        assert_eq!(
            entity.collision.subject_scan_gate_at_0x70,
            RetailRuntimeValue::Known(0)
        );
        assert_unblocked_without_surface_bubbles(&pass);
        assert!(!pass.sea_level_changed);
    }

    #[test]
    fn unresolved_entity_owned_type9_basis_blocks_without_consumption() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let mut resources = strict_resources(flat_terrain());
        let mut world_fx = WorldFx::new();
        let (mut manager, owner) = exact_type9_manager(9);
        let task_lease = owner.task_lease();
        let actor = task_lease.actor();
        manager
            .entity_mut_for_test(9)
            .unwrap()
            .physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
        let before = type9_runtime_snapshot(&manager, 9);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler
            .register_main_base_type9_exploding(task_lease)
            .unwrap()
            .is_none());

        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                20_000,
                64_000,
            ),
            &mut || panic!("an unresolved retained matrix must block before shared RNG"),
        );

        assert_eq!(
            pass.outcomes,
            [
                SpecializedActorTaskProductionOutcome::MainBaseType9Exploding(
                    MainBaseType9ExplodingProductionOutcome::Blocked {
                        entity_id: 9,
                        reason: MainBaseType9ExplodingProductionBlock::PhysicalBodyBasisUnresolved,
                    },
                )
            ]
        );
        assert_eq!(type9_runtime_snapshot(&manager, 9), before);
        assert_eq!(scheduler.registered_len(), 1);
        assert_unblocked_without_surface_bubbles(&pass);
        assert!(!pass.sea_level_changed);
        assert_eq!(
            scheduler.main_base_type9_actor_claims().collect::<Vec<_>>(),
            [actor]
        );
    }

    #[test]
    fn type9_elapsed_and_global_caps_remain_independent() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        for (elapsed_micros, global_elapsed_micros, expected) in [
            (
                u32::MAX,
                20_000,
                MainBaseType9ExplodingProductionBlock::ElapsedExceedsRetailCap { actual: u32::MAX },
            ),
            (
                20_000,
                u32::MAX,
                MainBaseType9ExplodingProductionBlock::GlobalElapsedExceedsRetailCap {
                    actual: u32::MAX,
                },
            ),
        ] {
            let mut resources = strict_resources(flat_terrain());
            let mut world_fx = WorldFx::new();
            let (mut manager, owner) = exact_type9_manager(9);
            let task_lease = owner.task_lease();
            let before = type9_runtime_snapshot(&manager, 9);
            let mut scheduler = SpecializedActorTaskScheduler::new();
            assert!(scheduler
                .register_main_base_type9_exploding(task_lease)
                .unwrap()
                .is_none());

            let pass = scheduler.tick_with_scripted_random(
                &mut manager,
                production_frame(
                    GameplayNotificationPhase::Playing,
                    &mut resources,
                    &mut world_fx,
                    &mut static_damage,
                    elapsed_micros,
                    global_elapsed_micros,
                ),
                &mut || panic!("oversized clocks block before shared RNG"),
            );

            assert_eq!(
                pass.outcomes,
                [
                    SpecializedActorTaskProductionOutcome::MainBaseType9Exploding(
                        MainBaseType9ExplodingProductionOutcome::Blocked {
                            entity_id: 9,
                            reason: expected,
                        },
                    )
                ]
            );
            assert_eq!(type9_runtime_snapshot(&manager, 9), before);
            assert_eq!(scheduler.registered_len(), 1);
            assert_unblocked_without_surface_bubbles(&pass);
            assert!(!pass.sea_level_changed);
        }
    }

    #[test]
    fn stale_type9_allocation_drops_before_entity_basis_or_rng() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let (_old_manager, old_owner) = exact_type9_manager(9);
        let stale_task_lease = old_owner.task_lease();
        let (mut manager, current_owner) = exact_type9_manager(9);
        let current_actor = current_owner.task_lease().actor();
        assert_ne!(stale_task_lease.actor(), current_actor);
        let before = type9_runtime_snapshot(&manager, 9);
        let mut resources = strict_resources(flat_terrain());
        let mut world_fx = WorldFx::new();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler
            .register_main_base_type9_exploding(stale_task_lease)
            .unwrap()
            .is_none());

        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                20_000,
                64_000,
            ),
            &mut || panic!("a stale allocation lease consumes no shared RNG"),
        );

        assert_eq!(
            pass.outcomes,
            [
                SpecializedActorTaskProductionOutcome::MainBaseType9Exploding(
                    MainBaseType9ExplodingProductionOutcome::Dropped {
                        entity_id: 9,
                        reason: MainBaseType9ExplodingProductionDrop::ActorLeaseMismatch {
                            expected: current_actor,
                            actual: stale_task_lease.actor(),
                        },
                    },
                )
            ]
        );
        assert_eq!(type9_runtime_snapshot(&manager, 9), before);
        assert_eq!(scheduler.registered_len(), 0);
        assert!(scheduler.main_base_type9_actor_claims().next().is_none());
        assert_unblocked_without_surface_bubbles(&pass);
        assert!(!pass.sea_level_changed);
    }

    #[test]
    fn absent_type9_owner_drops_after_the_complete_live_snapshot() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let (_source_manager, source_owner) = exact_type9_manager(99);
        let missing_task_lease = source_owner.task_lease();
        let (type47_entity, type47_metadata, type47_owner) =
            exact_type47_single_owner_fixture(47, 11);
        let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 48];
        metadata[47] = type47_metadata;
        let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![type47_entity],
            metadata,
            true,
        );
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler
            .register_main_base_type9_exploding(missing_task_lease)
            .unwrap()
            .is_none());
        scheduler
            .register_type47_common_dying(type47_owner)
            .unwrap();
        let mut resources = strict_resources(flat_terrain());
        let mut world_fx = WorldFx::new();
        let mut words = [0x0026, 0x1e27].into_iter();
        let mut draws = 0;

        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                1_000,
                64_000,
            ),
            &mut || {
                draws += 1;
                words
                    .next()
                    .expect("only the live Type-47 callback consumes RNG")
            },
        );

        assert_eq!(draws, 2);
        assert_eq!(
            pass.outcomes,
            [
                SpecializedActorTaskProductionOutcome::Type47CommonDying(
                    Type47CommonDyingProductionOutcome::SchedulerWaiting { entity_id: 47 },
                ),
                SpecializedActorTaskProductionOutcome::MainBaseType9Exploding(
                    MainBaseType9ExplodingProductionOutcome::Dropped {
                        entity_id: 99,
                        reason: MainBaseType9ExplodingProductionDrop::EntityUnavailable,
                    },
                ),
            ]
        );
        assert_eq!(scheduler.registered_len(), 1);
        assert!(scheduler.main_base_type9_actor_claims().next().is_none());
        assert_unblocked_without_surface_bubbles(&pass);
        assert!(!pass.sea_level_changed);
    }

    #[test]
    fn unresolved_type9_transition_gate_blocks_before_scheduler_rng() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let mut resources = strict_resources(flat_terrain());
        let mut world_fx = WorldFx::new();
        let (mut manager, owner) = exact_type9_manager(9);
        let task_lease = owner.task_lease();
        let actor = task_lease.actor();
        let entity = manager.entity_mut_for_test(9).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .invalidate(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT);
        let before = type9_runtime_snapshot(&manager, 9);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler
            .register_main_base_type9_exploding(task_lease)
            .unwrap()
            .is_none());
        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                20_000,
                64_000,
            ),
            &mut || panic!("an unresolved transition gate consumes no scheduler RNG"),
        );

        assert_eq!(
            pass.outcomes,
            [
                SpecializedActorTaskProductionOutcome::MainBaseType9Exploding(
                    MainBaseType9ExplodingProductionOutcome::Blocked {
                        entity_id: 9,
                        reason: MainBaseType9ExplodingProductionBlock::OuterOwnerStateUnresolved,
                    }
                )
            ]
        );
        assert_eq!(type9_runtime_snapshot(&manager, 9), before);
        assert_eq!(scheduler.registered_len(), 1);
        assert_eq!(
            scheduler.main_base_type9_actor_claims().collect::<Vec<_>>(),
            [actor]
        );
        assert_unblocked_without_surface_bubbles(&pass);
        assert!(!pass.sea_level_changed);
    }

    #[test]
    fn terminal_type47_receipt_releases_before_the_later_deferred_sweep() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let mut terrain = flat_terrain();
        terrain.header[0] = -1_000 << 8;
        let mut resources = strict_resources(terrain);
        let mut world_fx = WorldFx::new();
        let (entity, type47_metadata, owner) = exact_type47_single_owner_fixture(47, 11);
        let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 48];
        metadata[47] = type47_metadata;
        let mut manager =
            EntityManager::from_entities_with_type_metadata_for_test(vec![entity], metadata, true);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.register_type47_common_dying(owner).unwrap();

        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                20_000,
                20_000,
            ),
            &mut || 0,
        );

        assert_eq!(scheduler.registered_len(), 0);
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), [47]);
        assert!(manager.iter_all().any(|entity| entity.id == 47));
        assert_eq!(pass.outcomes.len(), 1);
        let SpecializedActorTaskProductionOutcome::Type47CommonDying(
            Type47CommonDyingProductionOutcome::DeferredDestroyStaged(terminal),
        ) = &pass.outcomes[0]
        else {
            panic!("the terminal owner must stage removal and release its receipt")
        };
        assert_eq!(terminal.entity_id, 47);
        assert_eq!(terminal.visit, owner.visit());
        assert_unblocked_without_surface_bubbles(&pass);
        assert!(!pass.sea_level_changed);
    }

    #[test]
    fn retryable_type54_block_retains_then_advances_after_evidence_is_repaired() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let mut terrain = flat_terrain();
        terrain.header[0] = 10_000;
        let mut resources = strict_resources(terrain);
        let mut world_fx = WorldFx::new();
        let (entity, task_id) = exact_type54_sea_level_owner_fixture(54, -1_000);
        let mut manager = EntityManager::from_entities_for_test(vec![entity]);
        manager
            .entity_mut_for_test(54)
            .unwrap()
            .collision
            .state_flags_at_0x08 = RetailStateWord::unknown();
        let receipt = issue_exact_type54_sea_level_owner_receipt(&manager, 54, task_id);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler
            .register_main_base_type54_sea_level(receipt)
            .unwrap()
            .is_none());

        let blocked = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                0x1_0000,
                0x1_0000,
            ),
            &mut || panic!("an evidence block consumes no shared RNG"),
        );
        assert_eq!(
            blocked.outcomes,
            [
                SpecializedActorTaskProductionOutcome::MainBaseType54SeaLevel(
                    MainBaseType54SeaLevelProductionOutcome::Blocked {
                        entity_id: 54,
                        reason: MainBaseType54SeaLevelTickBlock::DeferredDestroyStateUnresolved,
                    },
                ),
            ]
        );
        assert_eq!(scheduler.registered_len(), 1);
        assert_eq!(resources.level_terrain().unwrap().header[0], 10_000);
        assert_unblocked_without_surface_bubbles(&blocked);
        assert!(!blocked.sea_level_changed);

        manager
            .entity_mut_for_test(54)
            .unwrap()
            .collision
            .state_flags_at_0x08 = RetailStateWord::exact(0);
        let resumed = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                0x1_0000,
                0x1_0000,
            ),
            &mut || panic!("Change-Sea-Level consumes no shared RNG"),
        );
        assert_eq!(
            resumed.outcomes,
            [
                SpecializedActorTaskProductionOutcome::MainBaseType54SeaLevel(
                    MainBaseType54SeaLevelProductionOutcome::Continuing {
                        entity_id: 54,
                        elapsed_ms: 65,
                        remaining_delta_raw: -200,
                    },
                ),
            ]
        );
        assert_eq!(scheduler.registered_len(), 1);
        assert_eq!(resources.level_terrain().unwrap().header[0], 9_200);
        assert_unblocked_without_surface_bubbles(&resumed);
        assert!(resumed.sea_level_changed);
    }

    #[test]
    fn absent_type54_snapshot_entry_drops_linear_receipt_without_a_visit() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let (entity, task_id) = exact_type54_sea_level_owner_fixture(54, -1_000);
        let source_manager = EntityManager::from_entities_for_test(vec![entity]);
        let receipt = issue_exact_type54_sea_level_owner_receipt(&source_manager, 54, task_id);
        let mut manager = EntityManager::from_entities_for_test(Vec::new());
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler
            .register_main_base_type54_sea_level(receipt)
            .unwrap()
            .is_none());
        let mut resources = strict_resources(flat_terrain());
        let mut world_fx = WorldFx::new();

        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                20_000,
                20_000,
            ),
            &mut || panic!("a missing allocation consumes no shared RNG"),
        );

        assert_eq!(
            pass.outcomes,
            [
                SpecializedActorTaskProductionOutcome::MainBaseType54SeaLevel(
                    MainBaseType54SeaLevelProductionOutcome::Dropped {
                        entity_id: 54,
                        reason: MainBaseType54SeaLevelTickBlock::EntityMissing,
                    },
                ),
            ]
        );
        assert_eq!(scheduler.registered_len(), 0);
        assert_eq!(resources.level_terrain().unwrap().header[0], 0);
        assert_unblocked_without_surface_bubbles(&pass);
        assert!(!pass.sea_level_changed);
    }

    #[test]
    fn terminal_type54_receipt_releases_after_inline_terrain_completion() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let mut terrain = flat_terrain();
        terrain.header[0] = 10_000;
        let mut resources = strict_resources(terrain);
        let mut world_fx = WorldFx::new();
        let (entity, task_id) = exact_type54_sea_level_owner_fixture(54, -1);
        let mut manager = EntityManager::from_entities_for_test(vec![entity]);
        let receipt = issue_exact_type54_sea_level_owner_receipt(&manager, 54, task_id);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler
            .register_main_base_type54_sea_level(receipt)
            .unwrap()
            .is_none());

        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                0x1_0000,
                0x1_0000,
            ),
            &mut || panic!("Change-Sea-Level consumes no shared RNG"),
        );

        assert_eq!(resources.level_terrain().unwrap().header[0], 9_999);
        assert_eq!(scheduler.registered_len(), 0);
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), [54]);
        assert!(manager.iter_all().any(|entity| entity.id == 54));
        assert_eq!(
            pass.outcomes,
            [
                SpecializedActorTaskProductionOutcome::MainBaseType54SeaLevel(
                    MainBaseType54SeaLevelProductionOutcome::DeferredDestroyStaged {
                        entity_id: 54,
                        elapsed_ms: 65,
                        final_sea_level_raw: 9_999,
                    },
                ),
            ]
        );
        assert_unblocked_without_surface_bubbles(&pass);
        assert!(pass.sea_level_changed);
    }

    #[test]
    fn empty_scheduler_needs_no_current_level_resources_or_rng() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let mut manager = EntityManager::from_entities_for_test(Vec::new());
        let mut resources = ResourceCache::new(Vec::new());
        let mut world_fx = WorldFx::new();
        let mut random_oracle = WorldFx::new();
        let expected_first_word = random_oracle.next_shared_retail_random_u16();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        let mut notifications = GameplayNotifications::new();

        let pass = scheduler.tick(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                u32::MAX,
                u32::MAX,
            ),
            &mut notifications,
        );

        assert_eq!(
            pass,
            SpecializedActorTaskProductionPass {
                block: None,
                outcomes: Vec::new(),
                surface_bubbles_materialized: 0,
                surface_bubbles_dropped: 0,
                sea_level_changed: false,
                explosion_lights: Vec::new(),
                progressive_death_presentation_requested: false,
                terrain_changed: false,
            }
        );
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            expected_first_word
        );
    }

    #[test]
    fn incomplete_current_level_resources_block_the_whole_pass_before_custody_or_rng() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        for (case, level, expected_block) in [
            (
                "terrain",
                level_state(None, Some(collision_level_descriptor())),
                SpecializedActorTaskProductionBlock::CurrentLevelTerrainUnavailable,
            ),
            (
                "collision descriptor",
                level_state(Some(flat_terrain()), None),
                SpecializedActorTaskProductionBlock::CurrentLevelCollisionContextUnavailable,
            ),
        ] {
            let (mut manager, owner) = exact_type9_manager(9);
            let task_lease = owner.task_lease();
            let actor = task_lease.actor();
            let before = type9_runtime_snapshot(&manager, 9);
            let mut scheduler = SpecializedActorTaskScheduler::new();
            scheduler
                .register_main_base_type9_exploding(task_lease)
                .unwrap();
            let mut resources = ResourceCache::new(Vec::new());
            resources.load_level(level);
            let mut world_fx = WorldFx::new();
            let mut notifications = GameplayNotifications::new();
            let mut random_oracle = WorldFx::new();
            let expected_first_word = random_oracle.next_shared_retail_random_u16();

            let pass = scheduler.tick(
                &mut manager,
                production_frame(
                    GameplayNotificationPhase::Playing,
                    &mut resources,
                    &mut world_fx,
                    &mut static_damage,
                    20_000,
                    64_000,
                ),
                &mut notifications,
            );

            assert_eq!(pass.block, Some(expected_block), "{case}");
            assert!(pass.outcomes.is_empty(), "{case}");
            assert_eq!(pass.surface_bubbles_materialized, 0, "{case}");
            assert_eq!(pass.surface_bubbles_dropped, 0, "{case}");
            assert!(!pass.sea_level_changed, "{case}");
            assert_eq!(type9_runtime_snapshot(&manager, 9), before, "{case}");
            assert_eq!(scheduler.registered_len(), 1, "{case}");
            assert_eq!(
                scheduler.main_base_type9_actor_claims().collect::<Vec<_>>(),
                [actor],
                "{case}"
            );
            assert_eq!(
                world_fx.next_shared_retail_random_u16(),
                expected_first_word,
                "{case}"
            );
        }
    }

    #[test]
    fn type66_global_and_model_resource_blocks_retain_atomic_custody() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let (mut manager, owner) = exact_type66_post_abort_owner();
        let task_lease = owner.task_lease();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .register_main_base_type66_production(task_lease)
            .unwrap();
        let entry_claims = scheduler
            .main_base_type66_actor_claims()
            .collect::<Vec<_>>();
        let mut notifications = GameplayNotifications::new();
        let mut world_fx = WorldFx::new();
        let mut rng_oracle = WorldFx::new();

        let mut unavailable = ResourceCache::new(Vec::new());
        let live_before_global = type66_live_snapshot(&manager, task_lease);
        let notifications_before_global = format!("{notifications:?}");
        let world_fx_before_global = format!("{world_fx:?}");
        let global_block = scheduler.tick(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut unavailable,
                &mut world_fx,
                &mut static_damage,
                100_000,
                100_000,
            ),
            &mut notifications,
        );
        assert_eq!(
            global_block.block,
            Some(SpecializedActorTaskProductionBlock::CurrentLevelTerrainUnavailable)
        );
        assert!(global_block.outcomes.is_empty());
        assert!(global_block.explosion_lights.is_empty());
        assert_eq!(scheduler.registered_len(), 1);
        assert_eq!(
            type66_live_snapshot(&manager, task_lease),
            live_before_global
        );
        assert_eq!(format!("{notifications:?}"), notifications_before_global);
        assert_eq!(format!("{world_fx:?}"), world_fx_before_global);
        assert!(manager
            .advance_unclaimed_base_factory_progressive_deaths(100_000, &entry_claims)
            .is_empty());
        assert_eq!(
            type66_live_snapshot(&manager, task_lease),
            live_before_global,
            "a blocked specialized owner remains claimed for the later legacy pass"
        );
        assert_type66_owner_unchanged(&mut scheduler, task_lease);
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            rng_oracle.next_shared_retail_random_u16()
        );

        let mut no_global_models = strict_resources(flat_terrain());
        let live_before_model = type66_live_snapshot(&manager, task_lease);
        let notifications_before_model = format!("{notifications:?}");
        let world_fx_before_model = format!("{world_fx:?}");
        let model_block = scheduler.tick(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut no_global_models,
                &mut world_fx,
                &mut static_damage,
                100_000,
                100_000,
            ),
            &mut notifications,
        );
        assert!(matches!(
            model_block.outcomes.as_slice(),
            [SpecializedActorTaskProductionOutcome::MainBaseType66Production(
                MainBaseType66ProductionOutcome::Blocked {
                    reason: crate::main_base_type66_production::MainBaseType66ProductionBlock::ActiveModelUnavailable { .. },
                    ..
                }
            )]
        ));
        assert!(model_block.explosion_lights.is_empty());
        assert_eq!(scheduler.registered_len(), 1);
        assert_eq!(
            type66_live_snapshot(&manager, task_lease),
            live_before_model
        );
        assert_eq!(format!("{notifications:?}"), notifications_before_model);
        assert_eq!(format!("{world_fx:?}"), world_fx_before_model);
        assert_type66_owner_unchanged(&mut scheduler, task_lease);
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            rng_oracle.next_shared_retail_random_u16()
        );

        let mut repaired = strict_type66_resources(type66_effect_program(0x0100));
        let resumed = scheduler.tick(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut repaired,
                &mut world_fx,
                &mut static_damage,
                100_000,
                100_000,
            ),
            &mut notifications,
        );
        assert!(matches!(
            resumed.outcomes.as_slice(),
            [
                SpecializedActorTaskProductionOutcome::MainBaseType66Production(
                    MainBaseType66ProductionOutcome::Continuing {
                        elapsed_ms: 100,
                        model_effect_stages: 1,
                        ..
                    }
                )
            ]
        ));
        assert_eq!(scheduler.registered_len(), 1);
        assert_eq!(type66_task_elapsed_ms(&manager, task_lease), 100);
        let committed = type66_live_snapshot(&manager, task_lease);
        assert_eq!(committed.task.elapsed_ms(), 100);
        assert_eq!(
            committed
                .factory
                .live_owner
                .expect("the continuing owner remains live")
                .state_version,
            live_before_model
                .factory
                .live_owner
                .expect("the pre-retry owner remains live")
                .state_version
                + 1,
            "the repaired retry commits the owner frame exactly once"
        );
    }

    #[test]
    fn type66_samples_live_hit_tick_at_its_visit_without_an_observation_table() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let (mut manager, owner) = exact_type66_post_abort_owner();
        let task_lease = owner.task_lease();
        let entity_id = owner.entity_id();
        manager
            .entity_mut_for_test(entity_id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(250);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .register_main_base_type66_production(task_lease)
            .unwrap();
        let mut resources = strict_type66_resources(vec![0x88]);
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let mut frame = production_frame(
            GameplayNotificationPhase::Playing,
            &mut resources,
            &mut world_fx,
            &mut static_damage,
            1,
            1,
        );
        frame.retail_tick = 250;

        let pass = scheduler.tick(&mut manager, frame, &mut notifications);

        assert!(matches!(
            pass.outcomes.as_slice(),
            [SpecializedActorTaskProductionOutcome::MainBaseType66Production(
                MainBaseType66ProductionOutcome::Continuing {
                    notification: crate::main_base_type66_abort::WorkingFactoryNotificationOutcome::NotificationRequested,
                    under_attack_notification_requested: true,
                    ..
                }
            )]
        ));
        assert_eq!(scheduler.registered_len(), 1);
        assert!(scheduler.main_base_type9_actor_claims().next().is_none());
    }

    #[test]
    fn missing_type66_actor_maps_to_entity_unavailable_and_releases_owner() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let (_source_manager, owner) = exact_type66_post_abort_owner();
        let task_lease = owner.task_lease();
        let entity_id = owner.entity_id();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .register_main_base_type66_production(task_lease)
            .unwrap();
        let mut manager = EntityManager::from_entities_for_test(Vec::new());
        let mut resources = strict_type66_resources(vec![0x88]);
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();

        let pass = scheduler.tick(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                1,
                1,
            ),
            &mut notifications,
        );

        assert_eq!(
            pass.outcomes,
            [
                SpecializedActorTaskProductionOutcome::MainBaseType66Production(
                    MainBaseType66ProductionOutcome::Dropped {
                        entity_id,
                        reason: MainBaseType66ProductionDrop::EntityUnavailable,
                    },
                )
            ]
        );
        assert!(pass.explosion_lights.is_empty());
        assert!(scheduler.is_empty());
    }

    #[test]
    fn terminal_type66_releases_live_and_stale_owners_without_replaying_effects() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let (mut manager, owner) = exact_type66_post_abort_owner();
        let task_lease = owner.task_lease();
        let entity_id = owner.entity_id();
        let pickup_id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 61)
            .expect("the exact composition contains a Power-Up")
            .id;
        set_type66_terminal_predecessor(&mut manager, entity_id, pickup_id, 7, 3_000_001);
        manager
            .entity_mut_for_test(entity_id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(250);

        let stale_id = 999;
        let stale_visit = task_visit(8);
        let stale_lease = MainBaseType66WorkingFactoryTaskLease::issue(
            MainBaseAbortActorLease {
                entity_id: stale_id,
                allocation_identity: u64::from(stale_id) + 1,
            },
            stale_visit.task_id,
            1,
        );
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .register_main_base_type66_production(stale_lease)
            .unwrap();
        scheduler
            .register_main_base_type66_production(task_lease)
            .unwrap();
        let mut resources = strict_type66_resources(type66_effect_program(0x0200));
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let mut frame = production_frame(
            GameplayNotificationPhase::Playing,
            &mut resources,
            &mut world_fx,
            &mut static_damage,
            200_000,
            200_000,
        );
        frame.retail_tick = 250;

        let pass = scheduler.tick(&mut manager, frame, &mut notifications);

        assert!(matches!(
            pass.outcomes.as_slice(),
            [
                SpecializedActorTaskProductionOutcome::MainBaseType66Production(
                    MainBaseType66ProductionOutcome::Terminal {
                        entity_id: live_id,
                        elapsed_ms: 200,
                        notification: crate::main_base_type66_abort::WorkingFactoryNotificationOutcome::NotificationRequested,
                        model_effect_stages: 1,
                        accepted_effect_points: 1,
                        ..
                    }
                ),
                SpecializedActorTaskProductionOutcome::MainBaseType66Production(
                    MainBaseType66ProductionOutcome::Dropped {
                        entity_id: missing_id,
                        reason: MainBaseType66ProductionDrop::EntityUnavailable,
                    }
                ),
            ] if *live_id == entity_id && *missing_id == stale_id
        ));
        assert_eq!(pass.explosion_lights.len(), 1);
        assert_eq!(pass.explosion_lights[0].radius_raw, 0x600);
        assert!(scheduler.is_empty());
        assert_eq!(manager.pending_power_up_destroy_ids(), [pickup_id]);
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());

        let factory = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("terminal Type-66 remains until its ordinary owner removes it");
        let Some(ActorTaskRuntime::WorkingFactory(task)) =
            factory.actor_tasks.task_state(task_lease.task_id())
        else {
            panic!("terminal completion leaves the same Primary wrapper installed")
        };
        assert_eq!(task.elapsed_ms(), 200);
        assert_eq!(task.next_tick_sequence(), 1);
        let wrapper = factory
            .actor_tasks
            .wrapper_flags(task_lease.task_id())
            .unwrap();
        assert!(wrapper.alive);
        assert!(!wrapper.in_callback);

        let particle_count = world_fx.particle_count();
        let second = scheduler.tick(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                200_000,
                200_000,
            ),
            &mut notifications,
        );
        assert!(second.outcomes.is_empty());
        assert!(second.explosion_lights.is_empty());
        assert_eq!(world_fx.particle_count(), particle_count);
        assert_eq!(type66_task_elapsed_ms(&manager, task_lease), 200);

        assert_eq!(manager.cleanup_pending_power_up_destroys(), [pickup_id]);
        assert!(manager.iter_all().any(|entity| entity.id == entity_id));
        assert!(scheduler.is_empty());
    }

    #[test]
    fn public_tick_consumes_the_concrete_world_fx_rng_stream() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let (entity, type47_metadata, owner) = exact_type47_single_owner_fixture(47, 11);
        let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 48];
        metadata[47] = type47_metadata;
        let mut manager =
            EntityManager::from_entities_with_type_metadata_for_test(vec![entity], metadata, true);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.register_type47_common_dying(owner).unwrap();
        let mut resources = strict_resources(flat_terrain());
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let mut random_oracle = WorldFx::new();
        let _first_scheduler_word = random_oracle.next_shared_retail_random_u16();
        let _second_scheduler_word = random_oracle.next_shared_retail_random_u16();
        let expected_next_word = random_oracle.next_shared_retail_random_u16();

        let pass = scheduler.tick(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                1_000,
                1_000,
            ),
            &mut notifications,
        );

        assert!(matches!(
            pass.outcomes.as_slice(),
            [SpecializedActorTaskProductionOutcome::Type47CommonDying(
                Type47CommonDyingProductionOutcome::SchedulerWaiting { entity_id: 47 }
            )]
        ));
        assert_unblocked_without_surface_bubbles(&pass);
        assert!(!pass.sea_level_changed);
        assert_eq!(world_fx.next_shared_retail_random_u16(), expected_next_word);
    }

    #[test]
    fn type17_and_type47_bubbles_materialize_immediately_in_live_and_fixed_pool_order() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let (mut type17_entity, type17_metadata, type17_owner) =
            coarse_type17_common_dying_composite_fixture(17, 17);
        type17_entity.set_position_raw([0, -1_000, 0]);
        type17_entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(500);
        let (mut type47_entity, type47_metadata, type47_owner) =
            exact_type47_single_owner_fixture(47, 11);
        type47_entity.set_position_raw([0, -1_000, 0]);
        type47_entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(1_000);
        let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 48];
        metadata[17] = type17_metadata;
        metadata[47] = type47_metadata;
        let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![type17_entity, type47_entity],
            metadata,
            true,
        );
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .register_type47_common_dying(type47_owner)
            .unwrap();
        scheduler
            .register_type17_common_dying(type17_owner)
            .unwrap();
        let mut resources = strict_resources(flat_terrain());
        let mut world_fx = WorldFx::new();
        let mut draws = 0;

        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                20_000,
                20_000,
            ),
            &mut || {
                draws += 1;
                0
            },
        );

        assert_eq!(draws, 16);
        assert_eq!(pass.block, None);
        assert_eq!(
            pass.outcomes
                .iter()
                .map(SpecializedActorTaskProductionOutcome::entity_id)
                .collect::<Vec<_>>(),
            [17, 47]
        );
        assert_eq!(pass.surface_bubbles_materialized, 2);
        assert_eq!(pass.surface_bubbles_dropped, 0);
        assert!(!pass.sea_level_changed);
        assert_eq!(world_fx.particle_count(), 2);

        let presentation = world_fx.prepare_presentation([320, 240], 0x1800, |_| {
            v2k_render::ParticleCenterProjection {
                screen: [160, 120],
                depth_raw: 0x100,
                clip: 0,
            }
        });
        assert_eq!(
            presentation
                .particles()
                .map(|prepared| (prepared.slot, prepared.particle.owner_id))
                .collect::<Vec<_>>(),
            [(198, Some(47)), (199, Some(17))],
            "live-order births use virgin slots 199 then 198; priority traversal exposes newest first"
        );
    }

    #[test]
    fn sea_level_changed_stays_set_when_a_later_type54_visit_does_not_change_it() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let (changing_entity, changing_task_id) = exact_type54_sea_level_owner_fixture(54, -1_000);
        let (mut blocked_entity, blocked_task_id) =
            exact_type54_sea_level_owner_fixture(55, -1_000);
        blocked_entity.collision.state_flags_at_0x08 = RetailStateWord::unknown();
        let mut manager =
            EntityManager::from_entities_for_test(vec![changing_entity, blocked_entity]);
        let changing_receipt =
            issue_exact_type54_sea_level_owner_receipt(&manager, 54, changing_task_id);
        let blocked_receipt =
            issue_exact_type54_sea_level_owner_receipt(&manager, 55, blocked_task_id);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .register_main_base_type54_sea_level(blocked_receipt)
            .unwrap();
        scheduler
            .register_main_base_type54_sea_level(changing_receipt)
            .unwrap();
        let mut terrain = flat_terrain();
        terrain.header[0] = 10_000;
        let mut resources = strict_resources(terrain);
        let mut world_fx = WorldFx::new();

        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                0x1_0000,
                0x1_0000,
            ),
            &mut || panic!("Type 54 consumes no shared RNG"),
        );

        assert_eq!(
            pass.outcomes
                .iter()
                .map(SpecializedActorTaskProductionOutcome::entity_id)
                .collect::<Vec<_>>(),
            [54, 55]
        );
        assert_eq!(resources.level_terrain().unwrap().header[0], 9_200);
        assert_unblocked_without_surface_bubbles(&pass);
        assert!(pass.sea_level_changed);
        assert_eq!(scheduler.registered_len(), 2);
    }

    #[test]
    fn scheduler_type9_claims_compose_with_the_neutral_animation_pass() {
        let (claimed_entity, metadata, claimed_task_id) =
            exact_main_base_type9_exploding_owner_fixture(9, [0; 3]);
        let mut neutral_entity = Entity::unresolved_port_entity(10, EntityKind::Enemy, 9);
        neutral_entity.actor_animation_runtime = RetailRuntimeValue::Known(Some(
            ActorAnimationController::from_descriptor(ActorAnimationDescriptor {
                capability_bit_3_sound_id: 72,
                capability_mask_0x201_sound_id: 0,
                attention_stop_sound_id: 72,
                variable_binding: 1,
                frames_per_direction: 4,
            })
            .unwrap(),
        ));
        let mut metadata_rows = vec![EntityTypeRuntimeMetadata::default(); 48];
        metadata_rows[LEVEL_ONE_TYPE9_ENTITY_TYPE as usize] = metadata;
        let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![claimed_entity, neutral_entity],
            metadata_rows,
            true,
        );
        let owner = issue_exact_main_base_type9_exploding_owner(&manager, 9, claimed_task_id);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .register_main_base_type9_exploding(owner.task_lease())
            .unwrap();
        let claimed_before = manager
            .iter_all()
            .find(|entity| entity.id == 9)
            .unwrap()
            .actor_animation_runtime;
        let neutral_before = manager
            .iter_all()
            .find(|entity| entity.id == 10)
            .unwrap()
            .actor_animation_runtime;
        let claims = scheduler.main_base_type9_actor_claims().collect::<Vec<_>>();

        manager.advance_unclaimed_actor_animations(20_000, &claims);

        assert_eq!(claims, [owner.task_lease().actor()]);
        assert_eq!(
            manager
                .iter_all()
                .find(|entity| entity.id == 9)
                .unwrap()
                .actor_animation_runtime,
            claimed_before
        );
        assert_ne!(
            manager
                .iter_all()
                .find(|entity| entity.id == 10)
                .unwrap()
                .actor_animation_runtime,
            neutral_before
        );
    }

    #[test]
    fn scheduler_type66_claim_excludes_the_exact_current_factory_allocation() {
        let (mut manager, owner) = exact_type66_post_abort_owner();
        let task_lease = owner.task_lease();
        let actor = task_lease.actor();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .register_main_base_type66_production(task_lease)
            .unwrap();
        let before = type66_live_snapshot(&manager, task_lease);
        let claims = scheduler
            .main_base_type66_actor_claims()
            .collect::<Vec<_>>();

        let events = manager.advance_unclaimed_base_factory_progressive_deaths(100_000, &claims);

        assert_eq!(claims, [actor]);
        assert!(events.is_empty());
        assert_eq!(type66_live_snapshot(&manager, task_lease), before);
    }

    #[test]
    fn stale_type66_claim_with_the_same_entity_id_does_not_exclude_a_new_allocation() {
        let (_old_manager, old_owner) = exact_type66_post_abort_owner();
        let old_task_lease = old_owner.task_lease();
        let (mut current_manager, current_owner) = exact_type66_post_abort_owner();
        let current_task_lease = current_owner.task_lease();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .register_main_base_type66_production(old_task_lease)
            .unwrap();
        let claims = scheduler
            .main_base_type66_actor_claims()
            .collect::<Vec<_>>();

        assert_eq!(claims[0].entity_id, current_task_lease.actor().entity_id);
        assert_ne!(
            claims[0].allocation_identity,
            current_task_lease.actor().allocation_identity
        );
        let events =
            current_manager.advance_unclaimed_base_factory_progressive_deaths(100_000, &claims);

        assert!(!events.is_empty());
        assert_eq!(
            type66_live_snapshot(&current_manager, current_task_lease)
                .factory
                .progressive_death
                .elapsed_micros_raw,
            100_001
        );
    }

    #[test]
    fn scheduler_type66_claims_compose_with_legacy_progression_exactly_once() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let (mut manager, owner) = exact_type66_post_abort_owner();
        let task_lease = owner.task_lease();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler
            .register_main_base_type66_production(task_lease)
            .unwrap();
        let claims = scheduler
            .main_base_type66_actor_claims()
            .collect::<Vec<_>>();
        let mut resources = strict_type66_resources(type66_effect_program(0x0100));
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();

        let pass = scheduler.tick(
            &mut manager,
            production_frame(
                GameplayNotificationPhase::Playing,
                &mut resources,
                &mut world_fx,
                &mut static_damage,
                100_000,
                100_000,
            ),
            &mut notifications,
        );
        let legacy_events =
            manager.advance_unclaimed_base_factory_progressive_deaths(100_000, &claims);

        assert!(legacy_events.is_empty());
        assert!(matches!(
            pass.outcomes.as_slice(),
            [
                SpecializedActorTaskProductionOutcome::MainBaseType66Production(
                    MainBaseType66ProductionOutcome::Continuing {
                        elapsed_ms: 100,
                        ..
                    }
                )
            ]
        ));
        assert_eq!(
            type66_live_snapshot(&manager, task_lease)
                .factory
                .progressive_death
                .elapsed_micros_raw,
            100_001,
            "the claimed legacy pass and specialized callback compose to one clock advance"
        );
        assert_eq!(
            scheduler
                .main_base_type66_actor_claims()
                .collect::<Vec<_>>(),
            claims,
            "the continuing specialized owner retains the same exact allocation claim"
        );
    }
}

enum SpecializedRadialDeath {
    Task(SpecializedActorTaskOwner),
    Deferred(MainBaseAbortActorLease),
}

fn specialized_radial_death_owner(
    publication: crate::entity::DynamicRadialDeathPublication,
) -> SpecializedRadialDeath {
    use crate::entity::DynamicRadialDeathPublication;
    SpecializedRadialDeath::Task(match publication {
        DynamicRadialDeathPublication::NativeGroundDeferred(receipt) => {
            return SpecializedRadialDeath::Deferred(receipt.allocation());
        }
        DynamicRadialDeathPublication::Intro2Type8(owner) => {
            SpecializedActorTaskOwner::Intro2Type8(owner)
        }
        DynamicRadialDeathPublication::NativeType123(owner) => {
            SpecializedActorTaskOwner::NativeType123(owner)
        }
        DynamicRadialDeathPublication::NativeType86(owner) => {
            SpecializedActorTaskOwner::NativeType86(owner)
        }
        DynamicRadialDeathPublication::Intro2Type66(owner) => {
            SpecializedActorTaskOwner::Intro2Type66(owner)
        }
        DynamicRadialDeathPublication::MainBase(owner) => {
            SpecializedActorTaskOwner::MainBase(owner)
        }
        DynamicRadialDeathPublication::Intro2Type10Tumble(owner) => {
            SpecializedActorTaskOwner::Intro2Type10Tumble(owner)
        }
        DynamicRadialDeathPublication::Intro2Type57Tumble(owner) => {
            SpecializedActorTaskOwner::Intro2Type57Tumble(owner)
        }
        DynamicRadialDeathPublication::Intro2Class12(owner) => {
            SpecializedActorTaskOwner::Intro2CommonDying(owner)
        }
        DynamicRadialDeathPublication::Type47Class12(owner) => {
            SpecializedActorTaskOwner::Type47CommonDying(owner)
        }
        DynamicRadialDeathPublication::Intro2Type9Class14(lease) => {
            SpecializedActorTaskOwner::Intro2Type9Class14(
                crate::intro2_type9_class14::Intro2Type9Class14Owner::adopt(lease),
            )
        }
        DynamicRadialDeathPublication::Type9Class14(lease) => {
            SpecializedActorTaskOwner::OrdinaryType9Class14(
                MainBaseType9ExplodingProductionOwner::adopt(lease),
            )
        }
    })
}
