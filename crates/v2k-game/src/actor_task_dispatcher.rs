//! Detached heterogeneous dispatcher for closed actor task families.
//!
//! Retail stores different task families in one three-slot owner. This module
//! supplies that shared runtime shape and performs the exact
//! before-callback/callback/post-unwind composition for the families whose
//! per-frame owner contracts are currently closed:
//!
//! - ordinary type-9 `"Wander Near Location"`;
//! - class-45 `"Attract Attention"`'s Candidate Secondary and Cue Tertiary
//!   tasks (its target-route Primary remains a dedicated exact owner);
//! - class-54 `"Go To Job"`;
//! - shared Search-and-Attack / Guard Location `"Chase Target"`;
//! - shared Search-and-Attack / Run Away callback-owned target acquisition;
//! - class-33 `"Follow Beacons"`' distinct ranked acquisition and following;
//! - shared duration-owned `FUN_00402BA0` retarget movement;
//! - Search-and-Attack's slot-2 `"Aim and Fire"`;
//! - Guard Location's one-in-four candidate acquisition;
//! - class-10 `"Run Away"`'s slot-0 flee task;
//! - class-4 `"Defecate Virus"`'s slot-0 wander and slot-2 terrain tasks; and
//! - class-12 `"Flip Over And Die"`.
//!
//! External movement, target lookup, behavior transitions, and live entity
//! attachment remain explicit adapter boundaries. Search And Attack's
//! Guard Location's outer behavior remains detached; the class-7 acquisition,
//! Aim-and-Fire, and shared pursuing lifecycles are closed here without
//! guessing that still-open phase.

use crate::actor_task_owner::{
    ActorTaskOwner, ActorTaskSlot, ActorTaskVisit, ActorTaskVisitControl, PreparedActorTask,
};
use crate::aim_and_fire::{
    aim_and_fire_after_unwind, AimAndFireCallbackPrefix, AimAndFireCallbackResult,
    AimAndFireConstructorSuffixError, AimAndFireFrameOutcome, AimAndFirePrivateState,
    AimAndFireTaskState, AimAndFireTransitionReason, PreparedAimAndFireTask,
    AIM_AND_FIRE_CONSTRUCTOR_ADDRESS, AIM_AND_FIRE_LIFETIME_MS, AIM_AND_FIRE_TICK_ADDRESS,
};
use crate::attract_attention::AttractAttentionCandidateTaskState;
use crate::chase_target::{
    chase_target_transition_after_unwind, ChaseTargetCallbackPrefix, ChaseTargetCallbackResult,
    ChaseTargetCallbackStage, ChaseTargetTaskState, ChaseTargetTransitionRequest,
};
use crate::common_dying::{
    common_dying_after_unwind, CommonDyingAfterUnwindOutcome, CommonDyingCallbackPrefix,
    CommonDyingCallbackResult, CommonDyingTaskState, CommonDyingTransitionReason,
};
use crate::defecate_virus::{
    defecate_virus_terrain_after_unwind, defecate_virus_wander_after_unwind,
    plan_defecate_virus_callback, DefecateVirusCallbackPlan, DefecateVirusCallbackRequest,
    DefecateVirusTerrainCallbackPrefix, DefecateVirusTerrainTaskState,
    DefecateVirusTerrainTransitionRequest, DefecateVirusTransitionOutcome,
    DefecateVirusWanderCallbackPrefix, DefecateVirusWanderPostUnwind, DefecateVirusWanderTaskState,
    DefecateVirusWanderTransitionRequest,
};
use crate::defecate_virus_owner::{
    DefecateVirusTaskConstructorInputs, DefecateVirusTaskPreparation, DEFECATE_VIRUS_TERRAIN_MODE,
    DEFECATE_VIRUS_TERRAIN_PACKED_MODE, DEFECATE_VIRUS_TERRAIN_PAYLOAD,
    DEFECATE_VIRUS_TERRAIN_TASK, DEFECATE_VIRUS_WANDER_AUX_ADDRESS,
    DEFECATE_VIRUS_WANDER_DESTRUCTOR_ADDRESS, DEFECATE_VIRUS_WANDER_INITIALIZER_ADDRESS,
    DEFECATE_VIRUS_WANDER_LIFETIME_MS, DEFECATE_VIRUS_WANDER_PAIR_ADDRESS,
    DEFECATE_VIRUS_WANDER_TASK,
};
use crate::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use crate::follow_beacons::{
    follow_beacons_following_after_unwind, FollowBeaconAcquisitionCallbackPrefix,
    FollowBeaconAcquisitionCallbackResult, FollowBeaconAcquisitionTaskState,
    FollowBeaconsFollowingCallbackPrefix, FollowBeaconsFollowingCallbackResult,
    FollowBeaconsFollowingCallbackStage, FollowBeaconsFollowingConstructorSuffix,
    FollowBeaconsFollowingConstructorSuffixError, FollowBeaconsFollowingLifetimeStatus,
    FollowBeaconsFollowingPostUnwind, FollowBeaconsFollowingTaggedSingleton,
    FollowBeaconsFollowingTaskPreparation, FollowBeaconsFollowingTaskState,
    FollowBeaconsFollowingTransitionReason, FollowBeaconsFollowingTransitionRequest,
    FollowBeaconsTaskPreparation, FOLLOW_BEACONS_ACQUIRING_RETARGET_TASK,
    FOLLOW_BEACONS_FOLLOWING_TASK, FOLLOW_BEACON_ACQUISITION_TASK,
};
use crate::go_to_job_owner::{
    go_to_job_transition_after_unwind, GoToJobCallbackPrefix, GoToJobCallbackResult,
    GoToJobCallbackStage, GoToJobTaskState, GoToJobTransitionRequest,
};
use crate::guard_location_owner::acquisition::{
    GuardLocationAcquisitionCallbackPrefix, GuardLocationAcquisitionCallbackResult,
    GuardLocationAcquisitionTaggedSingleton, GuardLocationAcquisitionTaskState,
    GuardLocationCandidate,
};
use crate::guard_location_owner::{
    GuardLocationTaskConstructorInputs, GuardLocationTaskPreparation, GuardLocationTaskRole,
    GUARD_LOCATION_SEARCH_CONSTRUCTOR_ADDRESS, GUARD_LOCATION_SEARCH_FIXED_ARGUMENT,
    GUARD_LOCATION_SEARCH_TICK_ADDRESS,
};
use crate::hive_death::HiveRadialTaskState;
use crate::ordinary_type9_attract_attention_cue::{
    OrdinaryType9AttractAttentionCueCallbackPrefix, OrdinaryType9AttractAttentionCueLifetimeStatus,
    OrdinaryType9AttractAttentionCueTransition,
    OrdinaryType9AttractAttentionCueTransitionSelection,
};
use crate::ordinary_type9_wander_owner::{
    ordinary_type9_wander_after_unwind, OrdinaryType9WanderCallbackPrefix,
    OrdinaryType9WanderPostUnwind, OrdinaryType9WanderTaskState,
    OrdinaryType9WanderTransitionOutcome, OrdinaryType9WanderTransitionRequest,
};
use crate::run_away::{
    run_away_transition_after_unwind, RunAwayCallbackPrefix, RunAwayCallbackResult,
    RunAwayCallbackStage, RunAwayConstructorSuffixError, RunAwayTaskLifetime,
    RunAwayTaskPreparation, RunAwayTaskRole, RunAwayTaskSetupRequest, RunAwayTaskState,
    RunAwayTransitionRequest, RUN_AWAY_ACQUIRING_WANDER_LIFETIME_MS,
    RUN_AWAY_TASK_CONSTRUCTOR_ADDRESS, RUN_AWAY_TASK_LIFETIME_MS, RUN_AWAY_TASK_TICK_ADDRESS,
};
use crate::search_attack::{
    SearchAttackCandidateFilter, SearchAttackRadius, SearchAttackTaskLifetime, SearchAttackTaskRole,
};
use crate::search_attack_acquisition::{
    TargetAcquisitionCallbackPrefix, TargetAcquisitionCallbackResult, TargetAcquisitionTaskState,
    TARGET_ACQUISITION_CONSTRUCTOR_ADDRESS, TARGET_ACQUISITION_TICK_ADDRESS,
};
use crate::search_attack_owner::{SearchAttackTaskPreparation, SearchAttackTaskSetupRequest};
use crate::shared_retarget_mover::{
    shared_retarget_after_unwind, SharedRetarget, SharedRetargetCallbackPrefix,
    SharedRetargetLifetimePrefix, SharedRetargetPostUnwind, SharedRetargetTaskState,
    SharedRetargetTransitionRequest, SHARED_RETARGET_TASK_CONSTRUCTOR_ADDRESS,
    SHARED_RETARGET_TASK_TICK_ADDRESS,
};
use crate::shared_target_route::SharedTargetRouteTaskState;
use crate::wander_near_location::{
    map_common_mover_return, WanderNearCommonMoverReturn, WanderNearPrivateState,
    WanderNearRetarget, WanderNearSubAReset, WanderNearTaskCallbackResult,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorTaskRuntimeFamily {
    None,
    OrdinaryType9Wander,
    AttractAttentionCandidate,
    AttractAttentionCue,
    AttractAttentionTargetRoute,
    CapturePeoplePursuit,
    FishTargetRoute,
    CleansingLandscape,
    TerrainCleansing,
    CaptureBeaconAcquisition,
    CapturePeopleFollowing,
    GoToJob,
    ChaseTarget,
    TargetAcquisition,
    FollowBeaconAcquisition,
    FollowBeaconsFollowing,
    AimAndFire,
    GuardLocationAcquisition,
    RunAway,
    SharedRetarget,
    DefecateVirusWander,
    DefecateVirusTerrain,
    TrashFurniture,
    CommonDying,
    TumbleOutOfSky,
    Intro2Type57Tumble,
    ChangeSeaLevel,
    WorkingFactory,
    MainBase,
    Class0Timer,
    Intro2GunTurret,
    ExplodingRing,
    HiveRadial,
    BoulderRolling,
    BoulderResting,
    TrailingFire,
    RocketFlight,
    RocketTrail,
}

/// Inner state stored by the shared retail-style task owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorTaskRuntime {
    /// `FUN_00403230`: zero-private-size, unlimited-lifetime task whose
    /// `FUN_00403250` callback advances only the owner's Sub-I controller.
    /// The relation owner supplies its live linked-target read.
    None,
    OrdinaryType9Wander(OrdinaryType9WanderTaskState),
    AttractAttentionCandidate(crate::attract_attention::AttractAttentionCandidateTaskState),
    AttractAttentionCue(crate::attract_attention::AttractAttentionCueTaskState),
    AttractAttentionTargetRoute(SharedTargetRouteTaskState),
    /// Class9 variants1/2 share 403650/403780, with distinct duration and root callbacks.
    CapturePeoplePursuit(SharedTargetRouteTaskState),
    /// Class13 variant1 shares 03650/03780 with its own C690 transition owner.
    FishTargetRoute(SharedTargetRouteTaskState),
    CleansingLandscape(crate::cleansing_vehicle::tasks::CleansingMovementTaskState),
    TerrainCleansing(DefecateVirusTerrainTaskState),
    /// Class9 carrying search 402190/4021B0 has no task-private storage.
    CaptureBeaconAcquisition,
    /// Class9 variant4 shares 403B70/403CE0 data, retaining its own C790/CF90 owner.
    CapturePeopleFollowing(FollowBeaconsFollowingTaskState),
    GoToJob(GoToJobTaskState),
    ChaseTarget(ChaseTargetTaskState),
    TargetAcquisition(TargetAcquisitionTaskState),
    FollowBeaconAcquisition(FollowBeaconAcquisitionTaskState),
    FollowBeaconsFollowing(FollowBeaconsFollowingTaskState),
    AimAndFire(AimAndFireTaskState),
    GuardLocationAcquisition(GuardLocationAcquisitionTaskState),
    RunAway(RunAwayTaskState),
    SharedRetarget(SharedRetargetTaskState),
    DefecateVirusWander(DefecateVirusWanderTaskState),
    DefecateVirusTerrain(DefecateVirusTerrainTaskState),
    TrashFurniture(crate::trash_furniture::TrashFurnitureTaskState),
    CommonDying(CommonDyingTaskState),
    TumbleOutOfSky(crate::intro2_type10::death::Intro2Type10TumbleTask),
    Intro2Type57Tumble(crate::intro2_type57::death::Intro2Type57TumbleTask),
    ChangeSeaLevel(crate::main_base_type54_abort::MainBaseType54SeaLevelTaskState),
    WorkingFactory(crate::main_base_type66_abort::WorkingFactoryTaskState),
    MainBase(crate::main_base_runtime::MainBaseTaskState),
    /// C490/02800: finite null callback whose reselection requires its actor owner.
    Class0Timer(crate::class0_timer::Class0TimerTaskState),
    Intro2GunTurret(crate::intro2_gun_turret::task::GunTurretTaskState),
    ExplodingRing(crate::type60_exploding_ring::Type60ExplodingRingTaskState),
    HiveRadial(HiveRadialTaskState),
    BoulderRolling(crate::intro2_meteors::BoulderRollingTaskState),
    /// Class20 style1 `404B40 -> 404B60` stop-when-slow task.
    BoulderResting(crate::rolling_boulder::BoulderRestingTaskState),
    TrailingFire(crate::intro2_meteors::TrailingFireTaskState),
    RocketFlight(crate::native_entity_weapons::rocket::RocketFlightTaskState),
    /// 4069E0's class31 private payload; no timer or component reset.
    RocketTrail,
}

impl ActorTaskRuntime {
    /// Apply the recovered component destructor of the retiring task. The
    /// cue's `FUN_00402AC0` resolves Sub-I and calls `FUN_00420830`; ordinary
    /// task families have no animation effect at this boundary.
    pub(crate) fn retire_animation(
        &self,
        animation: &mut crate::actor_animation::ActorAnimationController,
    ) {
        if matches!(self, Self::AttractAttentionCue(_)) {
            animation.apply_attention_cue_retirement();
        }
    }

    pub const fn family(&self) -> ActorTaskRuntimeFamily {
        match self {
            Self::None => ActorTaskRuntimeFamily::None,
            Self::OrdinaryType9Wander(_) => ActorTaskRuntimeFamily::OrdinaryType9Wander,
            Self::AttractAttentionCandidate(_) => ActorTaskRuntimeFamily::AttractAttentionCandidate,
            Self::AttractAttentionCue(_) => ActorTaskRuntimeFamily::AttractAttentionCue,
            Self::AttractAttentionTargetRoute(_) => {
                ActorTaskRuntimeFamily::AttractAttentionTargetRoute
            }
            Self::CapturePeoplePursuit(_) => ActorTaskRuntimeFamily::CapturePeoplePursuit,
            Self::FishTargetRoute(_) => ActorTaskRuntimeFamily::FishTargetRoute,
            Self::CleansingLandscape(_) => ActorTaskRuntimeFamily::CleansingLandscape,
            Self::TerrainCleansing(_) => ActorTaskRuntimeFamily::TerrainCleansing,
            Self::CaptureBeaconAcquisition => ActorTaskRuntimeFamily::CaptureBeaconAcquisition,
            Self::CapturePeopleFollowing(_) => ActorTaskRuntimeFamily::CapturePeopleFollowing,
            Self::GoToJob(_) => ActorTaskRuntimeFamily::GoToJob,
            Self::ChaseTarget(_) => ActorTaskRuntimeFamily::ChaseTarget,
            Self::TargetAcquisition(_) => ActorTaskRuntimeFamily::TargetAcquisition,
            Self::FollowBeaconAcquisition(_) => ActorTaskRuntimeFamily::FollowBeaconAcquisition,
            Self::FollowBeaconsFollowing(_) => ActorTaskRuntimeFamily::FollowBeaconsFollowing,
            Self::AimAndFire(_) => ActorTaskRuntimeFamily::AimAndFire,
            Self::GuardLocationAcquisition(_) => ActorTaskRuntimeFamily::GuardLocationAcquisition,
            Self::RunAway(_) => ActorTaskRuntimeFamily::RunAway,
            Self::SharedRetarget(_) => ActorTaskRuntimeFamily::SharedRetarget,
            Self::DefecateVirusWander(_) => ActorTaskRuntimeFamily::DefecateVirusWander,
            Self::DefecateVirusTerrain(_) => ActorTaskRuntimeFamily::DefecateVirusTerrain,
            Self::TrashFurniture(_) => ActorTaskRuntimeFamily::TrashFurniture,
            Self::CommonDying(_) => ActorTaskRuntimeFamily::CommonDying,
            Self::TumbleOutOfSky(_) => ActorTaskRuntimeFamily::TumbleOutOfSky,
            Self::Intro2Type57Tumble(_) => ActorTaskRuntimeFamily::Intro2Type57Tumble,
            Self::ChangeSeaLevel(_) => ActorTaskRuntimeFamily::ChangeSeaLevel,
            Self::WorkingFactory(_) => ActorTaskRuntimeFamily::WorkingFactory,
            Self::MainBase(_) => ActorTaskRuntimeFamily::MainBase,
            Self::Class0Timer(_) => ActorTaskRuntimeFamily::Class0Timer,
            Self::Intro2GunTurret(_) => ActorTaskRuntimeFamily::Intro2GunTurret,
            Self::ExplodingRing(_) => ActorTaskRuntimeFamily::ExplodingRing,
            Self::HiveRadial(_) => ActorTaskRuntimeFamily::HiveRadial,
            Self::BoulderRolling(_) => ActorTaskRuntimeFamily::BoulderRolling,
            Self::BoulderResting(_) => ActorTaskRuntimeFamily::BoulderResting,
            Self::TrailingFire(_) => ActorTaskRuntimeFamily::TrailingFire,
            Self::RocketFlight(_) => ActorTaskRuntimeFamily::RocketFlight,
            Self::RocketTrail => ActorTaskRuntimeFamily::RocketTrail,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetAcquisitionRuntimePreparationError {
    TaskContractMismatch,
}

/// Map one already-allocated class-7 acquisition phase into the heterogeneous
/// owner.
///
/// This deliberately accepts only variant-zero phase zero. The same owner
/// transaction later asks the caller to prepare its concurrent Wander task;
/// Aim-and-Fire and external-event state remain separate families until their
/// callback contracts are closed.
pub fn prepare_target_acquisition_runtime_task(
    preparation: SearchAttackTaskPreparation,
    radius: SearchAttackRadius,
    type_authored_filter: SearchAttackCandidateFilter,
    constructor_filter_override_raw: u32,
) -> Result<PreparedActorTask<ActorTaskRuntime>, TargetAcquisitionRuntimePreparationError> {
    let task = preparation.task;
    if preparation.request != SearchAttackTaskSetupRequest::Acquiring
        || preparation.phase_index != 0
        || task.role != SearchAttackTaskRole::AcquireTarget
        || task.slot != ActorTaskSlot::Secondary as u8
        || task.constructor_address != TARGET_ACQUISITION_CONSTRUCTOR_ADDRESS
        || task.tick_address != TARGET_ACQUISITION_TICK_ADDRESS
        || task.lifetime != SearchAttackTaskLifetime::CallbackOwned
    {
        return Err(TargetAcquisitionRuntimePreparationError::TaskContractMismatch);
    }

    Ok(PreparedActorTask::new(ActorTaskRuntime::TargetAcquisition(
        TargetAcquisitionTaskState::new(
            radius,
            type_authored_filter,
            constructor_filter_override_raw,
        ),
    )))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAttackAimAndFireRuntimePreparationError {
    TaskContractMismatch,
    ConstructorSuffix(AimAndFireConstructorSuffixError),
}

/// Behavior-neutral identity and target for one already-allocated shared
/// Aim-and-Fire task.
///
/// Search And Attack and Guard Location's pursuing style both reach the same
/// retail slot-2 constructor. Their behavior owners remain responsible for
/// authenticating how the target was selected; this request retains only the
/// shared task contract consumed at the heterogeneous-owner boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AimAndFireRuntimeTaskPreparation {
    pub slot: ActorTaskSlot,
    pub constructor_address: u32,
    pub tick_address: u32,
    pub lifetime_ms: u32,
    pub target_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AimAndFireRuntimePreparationError {
    TaskContractMismatch,
    ConstructorSuffix(AimAndFireConstructorSuffixError),
}

/// Map the exact shared slot-2 Aim-and-Fire contract into the heterogeneous
/// owner without assigning it to a particular behavior class.
pub(crate) fn prepare_aim_and_fire_runtime_task(
    preparation: AimAndFireRuntimeTaskPreparation,
    owner_entity_id: u32,
    owner_position_raw: [i16; 3],
    optional_sound_id_raw: u32,
    sound_period_us_raw: i32,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<PreparedAimAndFireTask<ActorTaskRuntime>, AimAndFireRuntimePreparationError> {
    if preparation.slot != ActorTaskSlot::Tertiary
        || preparation.constructor_address != AIM_AND_FIRE_CONSTRUCTOR_ADDRESS
        || preparation.tick_address != AIM_AND_FIRE_TICK_ADDRESS
        || preparation.lifetime_ms != AIM_AND_FIRE_LIFETIME_MS
    {
        return Err(AimAndFireRuntimePreparationError::TaskContractMismatch);
    }

    AimAndFireTaskState::prepare_after_allocation(
        owner_entity_id,
        owner_position_raw,
        preparation.target_id,
        optional_sound_id_raw,
        sound_period_us_raw,
        metadata,
    )
    .map(|prepared| prepared.map_task(ActorTaskRuntime::AimAndFire))
    .map_err(AimAndFireRuntimePreparationError::ConstructorSuffix)
}

/// Map the already-allocated first pursuing phase into the heterogeneous owner.
///
/// The returned wrapper deliberately retains Aim-and-Fire's constructor suffix.
/// Callers must apply that suffix before publishing the prepared task, exactly
/// as [`crate::search_attack_owner::apply_search_attack_task_setup`] expects.
pub fn prepare_search_attack_aim_and_fire_runtime_task(
    preparation: SearchAttackTaskPreparation,
    owner_entity_id: u32,
    owner_position_raw: [i16; 3],
    optional_sound_id_raw: u32,
    sound_period_us_raw: i32,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<PreparedAimAndFireTask<ActorTaskRuntime>, SearchAttackAimAndFireRuntimePreparationError>
{
    let SearchAttackTaskSetupRequest::Pursuing { target_id } = preparation.request else {
        return Err(SearchAttackAimAndFireRuntimePreparationError::TaskContractMismatch);
    };
    let task = preparation.task;
    if preparation.phase_index != 0 || task.role != SearchAttackTaskRole::AimAndFire {
        return Err(SearchAttackAimAndFireRuntimePreparationError::TaskContractMismatch);
    }
    let slot = match task.slot {
        0 => ActorTaskSlot::Primary,
        1 => ActorTaskSlot::Secondary,
        2 => ActorTaskSlot::Tertiary,
        _ => return Err(SearchAttackAimAndFireRuntimePreparationError::TaskContractMismatch),
    };
    let SearchAttackTaskLifetime::FixedMilliseconds(lifetime_ms) = task.lifetime else {
        return Err(SearchAttackAimAndFireRuntimePreparationError::TaskContractMismatch);
    };

    prepare_aim_and_fire_runtime_task(
        AimAndFireRuntimeTaskPreparation {
            slot,
            constructor_address: task.constructor_address,
            tick_address: task.tick_address,
            lifetime_ms,
            target_id,
        },
        owner_entity_id,
        owner_position_raw,
        optional_sound_id_raw,
        sound_period_us_raw,
        metadata,
    )
    .map_err(|error| match error {
        AimAndFireRuntimePreparationError::TaskContractMismatch => {
            SearchAttackAimAndFireRuntimePreparationError::TaskContractMismatch
        }
        AimAndFireRuntimePreparationError::ConstructorSuffix(error) => {
            SearchAttackAimAndFireRuntimePreparationError::ConstructorSuffix(error)
        }
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardLocationAcquisitionRuntimePreparationError {
    TaskContractMismatch,
}

/// Map one already-allocated Guard Location slot-1 task into the heterogeneous
/// owner.
///
/// The behavior-owned selector range and authored filter are deliberately not
/// stored in the task. Retail keeps those words in the Guard context and the
/// callback reaches them through its live behavior binding.
pub fn prepare_guard_location_acquisition_runtime_task(
    preparation: GuardLocationTaskPreparation,
) -> Result<PreparedActorTask<ActorTaskRuntime>, GuardLocationAcquisitionRuntimePreparationError> {
    let GuardLocationTaskPreparation {
        phase_index: 0,
        task,
        constructor_inputs:
            GuardLocationTaskConstructorInputs::AcquireCandidate {
                fixed_argument: GUARD_LOCATION_SEARCH_FIXED_ARGUMENT,
                search_task_context_word,
            },
    } = preparation
    else {
        return Err(GuardLocationAcquisitionRuntimePreparationError::TaskContractMismatch);
    };

    if task.role != GuardLocationTaskRole::AcquireCandidate
        || task.slot != ActorTaskSlot::Secondary
        || task.constructor_address != GUARD_LOCATION_SEARCH_CONSTRUCTOR_ADDRESS
        || task.tick_address != GUARD_LOCATION_SEARCH_TICK_ADDRESS
    {
        return Err(GuardLocationAcquisitionRuntimePreparationError::TaskContractMismatch);
    }

    Ok(PreparedActorTask::new(
        ActorTaskRuntime::GuardLocationAcquisition(GuardLocationAcquisitionTaskState::new(
            search_task_context_word,
        )),
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunAwayRuntimePreparationError {
    TaskContractMismatch,
    ConstructorSuffix(RunAwayConstructorSuffixError),
    GenericConstructorSuffix(SharedGenericConstructorSuffixError),
}

/// One ordered external write in Run Away variant one's constructor.
///
/// Retail completes the shared generic constructor, including its one RNG
/// word and Sub-A reset, before the dedicated initializer overwrites only the
/// target-speed dword with signed `(base * 5) / 3`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunAwayRuntimeConstructorEffect {
    Generic(SharedGenericConstructorEffect),
    ApplyFixedSubATargetSpeed {
        owner_id: u32,
        suffix: crate::run_away::RunAwayConstructorSuffix,
    },
}

/// Allocated flee task with both still-unpublished constructor suffixes.
#[derive(Debug)]
pub struct PreparedRunAwayRuntimeTask {
    generic: PreparedSharedGenericRuntimeTask,
    owner_id: u32,
    custom_suffix: crate::run_away::RunAwayConstructorSuffix,
}

impl PreparedRunAwayRuntimeTask {
    pub fn apply_suffix(
        self,
        next_shared_random: impl FnMut() -> u32,
        mut apply: impl FnMut(RunAwayRuntimeConstructorEffect),
    ) -> PreparedActorTask<ActorTaskRuntime> {
        let task = self.generic.apply_suffix(next_shared_random, |effect| {
            apply(RunAwayRuntimeConstructorEffect::Generic(effect));
        });
        apply(RunAwayRuntimeConstructorEffect::ApplyFixedSubATargetSpeed {
            owner_id: self.owner_id,
            suffix: self.custom_suffix,
        });
        task
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedAcquiringRuntimePreparationError {
    TaskContractMismatch,
    MissingInitializer,
    UnresolvedComponentTopology,
    UnsupportedSharedConstructorTopology,
    UnresolvedSubADescriptor,
    MissingSubADescriptor,
}

/// Why an already-allocated task cannot enter the bounded shared
/// `FUN_00406070` constructor suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedGenericConstructorSuffixError {
    UnresolvedComponentTopology,
    UnsupportedSharedConstructorTopology,
    UnresolvedSubADescriptor,
    MissingSubADescriptor,
}

/// The two statically closed component branches of shared `FUN_00406070`.
///
/// One branch reaches only Sub-A; the other first enables Sub-H and then
/// reaches the same Sub-A branch. Sub-G and Sub-F remain unsupported because
/// they own additional ordered effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedGenericConstructorTopology {
    SubAOnly,
    SubHThenSubA,
}

/// One exact external write in a task constructor's shared `FUN_00406070`
/// suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedGenericConstructorEffect {
    WriteSubHState08 {
        value: u32,
    },
    WriteSubADirection {
        direction_multiplier: i32,
    },
    WriteSubATargetSpeed {
        target_speed_raw: i32,
        random_sample_low16: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SharedGenericConstructorSuffix {
    topology: SharedGenericConstructorTopology,
    sub_a_target_speed_base_raw: i16,
}

/// Already-allocated phase paired with every still-unpublished constructor
/// effect. No prepared task can be released until the caller applies the
/// topology's optional Sub-H write, consumes one process-shared RNG word, and
/// applies the two Sub-A writes in retail order.
#[derive(Debug)]
pub struct PreparedSharedGenericRuntimeTask {
    task: PreparedActorTask<ActorTaskRuntime>,
    suffix: SharedGenericConstructorSuffix,
}

impl PreparedSharedGenericRuntimeTask {
    pub const fn topology(&self) -> SharedGenericConstructorTopology {
        self.suffix.topology
    }

    pub fn apply_suffix(
        self,
        mut next_shared_random: impl FnMut() -> u32,
        mut apply: impl FnMut(SharedGenericConstructorEffect),
    ) -> PreparedActorTask<ActorTaskRuntime> {
        if self.suffix.topology == SharedGenericConstructorTopology::SubHThenSubA {
            apply(SharedGenericConstructorEffect::WriteSubHState08 { value: 1 });
        }
        let random_sample_low16 = next_shared_random() as u16;
        let target_speed_raw = crate::common_mover::shared_initializer_target_speed_raw(
            self.suffix.sub_a_target_speed_base_raw,
            random_sample_low16,
        );
        let reset = WanderNearSubAReset {
            target_speed_raw,
            direction_multiplier: 1,
        };
        apply(SharedGenericConstructorEffect::WriteSubADirection {
            direction_multiplier: reset.direction_multiplier,
        });
        apply(SharedGenericConstructorEffect::WriteSubATargetSpeed {
            target_speed_raw: reset.target_speed_raw,
            random_sample_low16,
        });
        self.task
    }
}

/// Authenticate one statically closed component graph reached by
/// `FUN_00406070`, then retain its still-unpublished ordered effects.
///
/// This suffix is shared by multiple behavior constructors. The task family
/// and slot contract remain the caller's responsibility; publication remains
/// impossible until [`PreparedSharedGenericRuntimeTask::apply_suffix`] has
/// committed the optional Sub-H write, one shared RNG draw, and both Sub-A
/// writes.
pub fn prepare_shared_generic_constructor_suffix(
    task: PreparedActorTask<ActorTaskRuntime>,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<PreparedSharedGenericRuntimeTask, SharedGenericConstructorSuffixError> {
    let topology = match metadata.common_mover_topology {
        RetailRuntimeValue::Known(topology) => topology,
        RetailRuntimeValue::Unresolved => {
            return Err(SharedGenericConstructorSuffixError::UnresolvedComponentTopology);
        }
    };
    // The bounded records reach either Sub-A alone or Sub-H then Sub-A.
    // Sub-G/F would add distinct ordered effects and are deliberately rejected
    // by this preparer.
    let topology = match (
        topology.sub_h,
        topology.sub_g,
        topology.sub_f,
        topology.sub_a,
    ) {
        (false, false, false, true) => SharedGenericConstructorTopology::SubAOnly,
        (true, false, false, true) => SharedGenericConstructorTopology::SubHThenSubA,
        _ => {
            return Err(SharedGenericConstructorSuffixError::UnsupportedSharedConstructorTopology);
        }
    };
    let sub_a_target_speed_base_raw = match metadata.sub_a_propulsion_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => descriptor.target_speed_base_raw,
        RetailRuntimeValue::Known(None) => {
            return Err(SharedGenericConstructorSuffixError::MissingSubADescriptor);
        }
        RetailRuntimeValue::Unresolved => {
            return Err(SharedGenericConstructorSuffixError::UnresolvedSubADescriptor);
        }
    };
    Ok(PreparedSharedGenericRuntimeTask {
        task,
        suffix: SharedGenericConstructorSuffix {
            topology,
            sub_a_target_speed_base_raw,
        },
    })
}

fn map_suffix_error_to_shared_acquiring(
    error: SharedGenericConstructorSuffixError,
) -> SharedAcquiringRuntimePreparationError {
    match error {
        SharedGenericConstructorSuffixError::UnresolvedComponentTopology => {
            SharedAcquiringRuntimePreparationError::UnresolvedComponentTopology
        }
        SharedGenericConstructorSuffixError::UnsupportedSharedConstructorTopology => {
            SharedAcquiringRuntimePreparationError::UnsupportedSharedConstructorTopology
        }
        SharedGenericConstructorSuffixError::UnresolvedSubADescriptor => {
            SharedAcquiringRuntimePreparationError::UnresolvedSubADescriptor
        }
        SharedGenericConstructorSuffixError::MissingSubADescriptor => {
            SharedAcquiringRuntimePreparationError::MissingSubADescriptor
        }
    }
}

/// Map either already-allocated `FUN_0040B6C0` phase into its shared runtime
/// family.
///
/// Each phase authenticates either the bounded Sub-A-only or Sub-H/Sub-A
/// topology and returns its own one-shot effect bundle. Each bundle consumes
/// one shared RNG word only after that phase's allocation succeeds and before
/// its task is published.
/// `constructor_filter_override_raw` is initial style `+0x44`: Capture People
/// supplies `0x0C00`, while Run Away supplies zero. The phase-zero callback
/// retains it separately from the type-authored common-axis filter.
/// Callers retain the exact clear-tertiary, prepare-secondary, then
/// prepare-primary order by invoking this through
/// [`crate::run_away::apply_run_away_task_setup`].
pub fn prepare_shared_acquiring_runtime_task(
    preparation: RunAwayTaskPreparation,
    owner_position_raw: [i16; 3],
    metadata: &EntityTypeRuntimeMetadata,
    constructor_filter_override_raw: u32,
) -> Result<PreparedSharedGenericRuntimeTask, SharedAcquiringRuntimePreparationError> {
    if preparation.request != RunAwayTaskSetupRequest::Acquiring {
        return Err(SharedAcquiringRuntimePreparationError::TaskContractMismatch);
    }
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(SharedAcquiringRuntimePreparationError::MissingInitializer)?;
    let radius =
        SearchAttackRadius::from_raw(initializer.common_axis_descriptor.strict_axis_limit_raw);
    let type_authored_filter =
        SearchAttackCandidateFilter::from_raw(initializer.common_axis_descriptor.raw_word_at_0x04);
    let task = preparation.task;
    let runtime = match preparation.phase_index {
        0 if task.role == RunAwayTaskRole::AcquireTarget
            && task.slot == ActorTaskSlot::Secondary
            && task.constructor_address == TARGET_ACQUISITION_CONSTRUCTOR_ADDRESS
            && task.tick_address == TARGET_ACQUISITION_TICK_ADDRESS
            && task.lifetime == RunAwayTaskLifetime::CallbackOwned =>
        {
            ActorTaskRuntime::TargetAcquisition(TargetAcquisitionTaskState::new(
                radius,
                type_authored_filter,
                constructor_filter_override_raw,
            ))
        }
        1 if task.role == RunAwayTaskRole::Wander
            && task.slot == ActorTaskSlot::Primary
            && task.constructor_address == SHARED_RETARGET_TASK_CONSTRUCTOR_ADDRESS
            && task.tick_address == SHARED_RETARGET_TASK_TICK_ADDRESS
            && task.lifetime
                == RunAwayTaskLifetime::FixedMilliseconds(
                    RUN_AWAY_ACQUIRING_WANDER_LIFETIME_MS,
                ) =>
        {
            ActorTaskRuntime::SharedRetarget(SharedRetargetTaskState::new(
                owner_position_raw,
                RUN_AWAY_ACQUIRING_WANDER_LIFETIME_MS,
            ))
        }
        _ => {
            return Err(SharedAcquiringRuntimePreparationError::TaskContractMismatch);
        }
    };
    prepare_shared_generic_constructor_suffix(PreparedActorTask::new(runtime), metadata)
        .map_err(map_suffix_error_to_shared_acquiring)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconsAcquiringRuntimePreparationError {
    TaskContractMismatch,
    MissingInitializer,
    ConstructorSuffix(SharedGenericConstructorSuffixError),
}

/// Map either already-allocated Follow Beacons variant-zero phase into its
/// exact detached runtime family and authenticate the shared type-17 suffix.
///
/// Each successful phase still owns one Sub-H/Sub-A effect bundle and one
/// process-shared RNG draw. Callers must apply that bundle before publication
/// through [`crate::follow_beacons::apply_follow_beacons_acquiring_task_setup`].
pub fn prepare_follow_beacons_acquiring_runtime_task(
    preparation: FollowBeaconsTaskPreparation,
    owner_position_raw: [i16; 3],
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<PreparedSharedGenericRuntimeTask, FollowBeaconsAcquiringRuntimePreparationError> {
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(FollowBeaconsAcquiringRuntimePreparationError::MissingInitializer)?;
    let runtime = match preparation.phase_index {
        0 if preparation.task == FOLLOW_BEACON_ACQUISITION_TASK => {
            ActorTaskRuntime::FollowBeaconAcquisition(FollowBeaconAcquisitionTaskState::new(
                SearchAttackRadius::from_raw(
                    initializer.common_axis_descriptor.strict_axis_limit_raw,
                ),
                initializer.common_axis_descriptor.raw_word_at_0x04,
            ))
        }
        1 if preparation.task == FOLLOW_BEACONS_ACQUIRING_RETARGET_TASK => {
            ActorTaskRuntime::SharedRetarget(SharedRetargetTaskState::new(
                owner_position_raw,
                crate::follow_beacons::FOLLOW_BEACONS_ACQUIRING_RETARGET_LIFETIME_MS,
            ))
        }
        _ => {
            return Err(FollowBeaconsAcquiringRuntimePreparationError::TaskContractMismatch);
        }
    };
    prepare_shared_generic_constructor_suffix(PreparedActorTask::new(runtime), metadata)
        .map_err(FollowBeaconsAcquiringRuntimePreparationError::ConstructorSuffix)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconsFollowingRuntimePreparationError {
    TaskContractMismatch,
    ConstructorSuffix(FollowBeaconsFollowingConstructorSuffixError),
    GenericConstructorSuffix(SharedGenericConstructorSuffixError),
}

/// One ordered external write in Follow Beacons variant one's constructor.
///
/// The three shared type-17 writes are followed by B70's fixed `base * 4 / 3`
/// Sub-A overwrite. Keeping the custom suffix as a typed payload preserves the
/// descriptor-authentication result without manufacturing a live component
/// binding inside this detached dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowBeaconsFollowingConstructorEffect {
    Generic(SharedGenericConstructorEffect),
    ApplyFixedSubATargetSpeed {
        owner_id: u32,
        suffix: FollowBeaconsFollowingConstructorSuffix,
    },
}

/// An allocated Follow Beacons following task paired with every constructor
/// effect that must complete before publication.
#[derive(Debug)]
pub struct PreparedFollowBeaconsFollowingRuntimeTask {
    generic: PreparedSharedGenericRuntimeTask,
    owner_id: u32,
    custom_suffix: FollowBeaconsFollowingConstructorSuffix,
}

impl PreparedFollowBeaconsFollowingRuntimeTask {
    /// Apply shared Sub-H/RNG/Sub-A writes, then B70's fixed-speed overwrite.
    pub fn apply_suffix(
        self,
        next_shared_random: impl FnMut() -> u32,
        mut apply: impl FnMut(FollowBeaconsFollowingConstructorEffect),
    ) -> PreparedActorTask<ActorTaskRuntime> {
        let task = self.generic.apply_suffix(next_shared_random, |effect| {
            apply(FollowBeaconsFollowingConstructorEffect::Generic(effect));
        });
        apply(
            FollowBeaconsFollowingConstructorEffect::ApplyFixedSubATargetSpeed {
                owner_id: self.owner_id,
                suffix: self.custom_suffix,
            },
        );
        task
    }
}

/// Map the allocated `FUN_00403B70` task into its distinct dispatcher family.
///
/// Task identity is authenticated before either constructor suffix. The
/// resulting wrapper cannot publish until the exact shared type-17 effects and
/// the final fixed-speed overwrite have run in retail order.
pub fn prepare_follow_beacons_following_runtime_task(
    preparation: FollowBeaconsFollowingTaskPreparation,
    owner_entity_id: u32,
    owner_position_raw: [i16; 3],
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<PreparedFollowBeaconsFollowingRuntimeTask, FollowBeaconsFollowingRuntimePreparationError>
{
    if preparation.task != FOLLOW_BEACONS_FOLLOWING_TASK {
        return Err(FollowBeaconsFollowingRuntimePreparationError::TaskContractMismatch);
    }
    let prepared = FollowBeaconsFollowingTaskState::prepare_after_allocation(
        owner_entity_id,
        owner_position_raw,
        preparation.target_id,
        metadata,
    )
    .map_err(FollowBeaconsFollowingRuntimePreparationError::ConstructorSuffix)?
    .map_task(ActorTaskRuntime::FollowBeaconsFollowing);
    let (task, owner_id, custom_suffix) = prepared.into_generic_task();
    let generic = prepare_shared_generic_constructor_suffix(task, metadata)
        .map_err(FollowBeaconsFollowingRuntimePreparationError::GenericConstructorSuffix)?;
    Ok(PreparedFollowBeaconsFollowingRuntimeTask {
        generic,
        owner_id,
        custom_suffix,
    })
}

/// Map one already-allocated Run Away variant-one flee task into the
/// heterogeneous owner.
///
/// Acquiring variant tasks use the shared target-acquisition and retarget
/// callbacks, while variant two remains an external-event task. Those task
/// families are intentionally rejected here rather than being conflated with
/// the closed `FUN_00403F40` flee contract.
pub fn prepare_run_away_runtime_task(
    preparation: RunAwayTaskPreparation,
    owner_entity_id: u32,
    owner_position_raw: [i16; 3],
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<PreparedRunAwayRuntimeTask, RunAwayRuntimePreparationError> {
    let RunAwayTaskSetupRequest::Fleeing { target_id, audio } = preparation.request else {
        return Err(RunAwayRuntimePreparationError::TaskContractMismatch);
    };
    let task = preparation.task;
    if preparation.phase_index != 0
        || task.role != RunAwayTaskRole::Flee
        || task.slot != ActorTaskSlot::Primary
        || task.constructor_address != RUN_AWAY_TASK_CONSTRUCTOR_ADDRESS
        || task.tick_address != RUN_AWAY_TASK_TICK_ADDRESS
        || task.lifetime != RunAwayTaskLifetime::FixedMilliseconds(RUN_AWAY_TASK_LIFETIME_MS)
    {
        return Err(RunAwayRuntimePreparationError::TaskContractMismatch);
    }

    let prepared = RunAwayTaskState::prepare_after_allocation(
        owner_entity_id,
        owner_position_raw,
        target_id,
        audio,
        metadata,
    )
    .map(|prepared| prepared.map_task(ActorTaskRuntime::RunAway))
    .map_err(RunAwayRuntimePreparationError::ConstructorSuffix)?;
    let (task, owner_id, custom_suffix) = prepared.into_generic_task();
    let generic = prepare_shared_generic_constructor_suffix(task, metadata)
        .map_err(RunAwayRuntimePreparationError::GenericConstructorSuffix)?;
    Ok(PreparedRunAwayRuntimeTask {
        generic,
        owner_id,
        custom_suffix,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefecateVirusRuntimePreparationError {
    TaskContractMismatch,
}

/// Map one already-allocated class-4 setup phase into the heterogeneous owner.
///
/// The setup transaction remains owned by
/// [`crate::defecate_virus_owner::apply_defecate_virus_setup`]. This function
/// only closes the infallible family-state suffix and rejects any phase,
/// callback-specification, or constructor mismatch instead of guessing a
/// runtime family.
pub fn prepare_defecate_virus_runtime_task(
    preparation: DefecateVirusTaskPreparation,
    current_position_raw: [i16; 3],
) -> Result<PreparedActorTask<ActorTaskRuntime>, DefecateVirusRuntimePreparationError> {
    let runtime = match preparation {
        DefecateVirusTaskPreparation {
            phase_index: 0,
            task,
            constructor_inputs:
                DefecateVirusTaskConstructorInputs::TerrainInfection {
                    lifetime_ms,
                    payload: DEFECATE_VIRUS_TERRAIN_PAYLOAD,
                    mode: DEFECATE_VIRUS_TERRAIN_MODE,
                    packed_mode: DEFECATE_VIRUS_TERRAIN_PACKED_MODE,
                },
        } if task == DEFECATE_VIRUS_TERRAIN_TASK => {
            ActorTaskRuntime::DefecateVirusTerrain(DefecateVirusTerrainTaskState::new(lifetime_ms))
        }
        DefecateVirusTaskPreparation {
            phase_index: 1,
            task,
            constructor_inputs:
                DefecateVirusTaskConstructorInputs::Wander {
                    lifetime_ms: DEFECATE_VIRUS_WANDER_LIFETIME_MS,
                    initializer_address: DEFECATE_VIRUS_WANDER_INITIALIZER_ADDRESS,
                    destructor_address: DEFECATE_VIRUS_WANDER_DESTRUCTOR_ADDRESS,
                    pair_address: DEFECATE_VIRUS_WANDER_PAIR_ADDRESS,
                    auxiliary_address: DEFECATE_VIRUS_WANDER_AUX_ADDRESS,
                },
        } if task == DEFECATE_VIRUS_WANDER_TASK => ActorTaskRuntime::DefecateVirusWander(
            DefecateVirusWanderTaskState::new(current_position_raw),
        ),
        _ => return Err(DefecateVirusRuntimePreparationError::TaskContractMismatch),
    };
    Ok(PreparedActorTask::new(runtime))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorTaskDispatcherFrame {
    pub elapsed_micros: u32,
    /// Seventh shared task-callback input forwarded by retail's scheduler.
    pub scheduler_mode: u32,
}

/// Exact post-unwind owner handoff for a surviving Aim-and-Fire task.
///
/// `TaggedInvalidTarget` represents the scheduler's attempt to invoke owner
/// callback `+4`; `LifetimeExpired` represents the distinct strict
/// `elapsed > 5000` attempt at owner callback `+0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AimAndFireTransitionRequest {
    pub visit: ActorTaskVisit,
    pub reason: AimAndFireTransitionReason,
    pub committed_prefix: AimAndFireCallbackPrefix,
}

/// Observable result of one specific Aim-and-Fire owner-callback attempt.
///
/// Retail exits post-processing whenever an installed callback actually runs,
/// even when it returns zero. Only absence or entity-state suppression permits
/// a tag-`0x9C00` attempt to fall through to the independent lifetime check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AimAndFireTransitionOutcome<T> {
    Completed(Option<T>),
    CallbackAbsent,
    SuppressedByEntityState,
}

/// Observable result of Attract Attention's root-first cue transition.
///
/// The cue remains installed when the behavior callback is absent or the
/// post-unwind entity-state gate suppresses it. A completed callback may
/// publish a replacement graph; `Some` preserves its arbitrary nonzero
/// scheduler result while `None` continues to the next freshly read slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttractAttentionCueTransitionOutcome<T> {
    Completed(Option<T>),
    CallbackAbsent,
    SuppressedByEntityState,
}

/// Exact phase-3 handoff for a surviving class-12 task.
///
/// The detached common-dying shell determines only whether the generic actor
/// owner transition is requested. Live behavior-owner callbacks and entity
/// state gates remain adapter-owned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonDyingTransitionRequest {
    pub visit: ActorTaskVisit,
    pub reason: CommonDyingTransitionReason,
    pub committed_prefix: CommonDyingCallbackPrefix,
}

/// Adapter failure boundary for Follow Beacons' mover-first callback.
///
/// `CommonMover` is the only failure reached before mover-owned task-private
/// writes become authoritative. Every `AfterCommonMover` failure occurs after
/// a successful mover and therefore commits the staged private record even
/// though the dispatcher still fails closed. The explicit variants prevent a
/// phase distinction from being collapsed into an undocumented boolean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowBeaconsFollowingAdapterError<E> {
    CommonMover(E),
    AfterCommonMover(E),
}

/// A nonzero result returned by Follow Beacons' synchronous style callback.
///
/// This value re-enters the generic task scheduler's tag/state-bit handling;
/// it is not automatically a final dispatcher result. The live adapter owns
/// that classification and uses `Propagate` only when the generic scheduler
/// itself preserves the exact opaque result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsFollowingStyleResultRequest {
    pub visit: ActorTaskVisit,
    pub committed_prefix: FollowBeaconsFollowingCallbackPrefix,
    pub result: std::num::NonZeroU32,
}

/// Observable result of one known Follow Beacons owner-callback attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowBeaconsFollowingTransitionOutcome<T> {
    Completed(Option<T>),
    CallbackAbsent,
    SuppressedByEntityState,
}

/// Generic scheduler classification of an opaque nonzero style result.
///
/// Only `Propagate` corresponds to a tag outside `0x9C00..=0x9C02` and keeps
/// the exact returned pointer. `FallThroughLifetime` is distinct from an
/// immediate zero: tag `0x9C00` falls through when its callback is absent or
/// actor bit `0x1000` is set, while tag `0x9C01` falls through only when its
/// callback is absent. A bit-suppressed `0x9C01` returns zero immediately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowBeaconsFollowingStyleResultOutcome<T> {
    Completed(Option<T>),
    FallThroughLifetime,
    ReturnZero,
    Propagate(T),
}

/// Live dependencies deliberately kept outside the detached task owner.
///
/// The callback methods run while the wrapper has `in_callback == true` and
/// may clear or replace task slots. Their outputs are ignored automatically
/// when the current wrapper does not survive. Transition methods run only
/// after unwind and may replace later slots; the same pass will observe those
/// replacements freshly.
pub trait ActorTaskDispatcherAdapter {
    type Output;
    type Error;

    fn ordinary_wander_anchor_raw(&mut self, visit: ActorTaskVisit) -> [i16; 3];

    fn next_random(&mut self) -> u32;

    /// Consume Attract Attention Secondary's one callback RNG word and commit
    /// its live behavior-context filter update.
    ///
    /// This hook runs with the exact wrapper marked `in_callback`. A live
    /// class-45 adapter must authenticate its linear Candidate lease and retain
    /// the sibling Cue lease before mutating context. It must call
    /// [`GuardLocationAcquisitionTaskState::before_callback`] on the supplied
    /// shared state and must not consume another RNG value.
    fn attract_attention_candidate_prefix(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        task_state: AttractAttentionCandidateTaskState,
        random: u32,
    ) -> Result<GuardLocationAcquisitionCallbackPrefix, Self::Error>;

    /// Run class 45's first-eligible selector and synchronous target-style
    /// handoff against the already committed Candidate prefix.
    ///
    /// The callback runs under `in_callback` and may retire Secondary, clear
    /// the sibling Cue, and replace Primary. The phased owner then discards
    /// its result or error automatically when the executing wrapper did not
    /// survive. Live adapters must consume, retain, or replace their linear
    /// Candidate/Cue custody in the same callback transaction.
    fn attract_attention_candidate_callback(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        prefix: GuardLocationAcquisitionCallbackPrefix,
    ) -> Result<GuardLocationAcquisitionCallbackResult, Self::Error>;

    /// Convert a non-tagged, nonzero class-45 handoff result without remapping
    /// the retail value.
    fn attract_attention_candidate_propagated_result(
        &mut self,
        result: std::num::NonZeroU32,
    ) -> Self::Output;

    /// Authenticate the surviving class-45 Cue and return its entity owner.
    ///
    /// This post-unwind hook exists because the existing typed transition
    /// receipt carries the entity id while the generic three-slot owner does
    /// not. A live adapter must validate the exact Cue lease and whichever
    /// Candidate/Primary sibling leases remain before returning the id.
    fn attract_attention_cue_transition_owner_id(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        committed_prefix: OrdinaryType9AttractAttentionCueCallbackPrefix,
    ) -> Result<u32, Self::Error>;

    /// Apply an expired Cue's type-default/root-first transition after unwind.
    ///
    /// `CallbackAbsent` and `SuppressedByEntityState` retain the expired Cue
    /// for a later retry. A completed transition may replace any task slot;
    /// later slots are still read freshly in this same pass.
    fn attract_attention_cue_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        request: OrdinaryType9AttractAttentionCueTransition,
    ) -> Result<AttractAttentionCueTransitionOutcome<Self::Output>, Self::Error>;

    fn ordinary_wander_common_mover(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        private_state: &mut WanderNearPrivateState,
    ) -> Result<WanderNearCommonMoverReturn, Self::Error>;

    fn ordinary_wander_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        request: OrdinaryType9WanderTransitionRequest,
    ) -> Result<OrdinaryType9WanderTransitionOutcome<Self::Output>, Self::Error>;

    /// Evaluate the closed target -> route-predicate -> common-mover callback
    /// against the supplied stage.
    ///
    /// Live adapters should call
    /// [`crate::go_to_job_owner::evaluate_go_to_job_callback`] here. Keeping
    /// movement/controller storage outside this dispatcher avoids inventing a
    /// common entity layout before the live binding is proven.
    fn go_to_job_callback(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        stage: &mut GoToJobCallbackStage,
    ) -> Result<GoToJobCallbackResult, Self::Error>;

    /// Apply the behavior-owner transition represented by a closed Go To Job
    /// phase-3 request. `Some` propagates the owner result; `None` continues
    /// fresh-slot traversal.
    fn go_to_job_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        request: GoToJobTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error>;

    /// Evaluate the closed target validation -> wrapped range -> seven-input
    /// common-mover -> proximity-controller callback against the supplied
    /// detached stage.
    ///
    /// Live adapters should call
    /// [`crate::chase_target::evaluate_chase_target_callback`] and apply any
    /// returned controller write before returning success. Because portable
    /// lookup/mover failures have no retail analogue, the adapter must evaluate
    /// movement/controller changes transactionally and commit them only with
    /// `Ok`; this dispatcher applies the same rule to task-private state.
    /// Entity/component storage remains outside this dispatcher until that
    /// binding is proven.
    fn chase_target_callback(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        frame: ActorTaskDispatcherFrame,
        stage: &mut ChaseTargetCallbackStage,
    ) -> Result<ChaseTargetCallbackResult, Self::Error>;

    /// Apply the shared Chase Target behavior-owner transition after callback
    /// unwind. `Some` propagates the owner result; `None` continues fresh-slot
    /// traversal.
    fn chase_target_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        request: ChaseTargetTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error>;

    /// Run the shared acquisition callback against the prefix committed
    /// immediately before callback entry.
    ///
    /// Live adapters should call
    /// [`crate::search_attack_acquisition::evaluate_target_acquisition_callback`].
    /// Its behavior-handoff closure must be selected from the adapter's current
    /// live behavior. It executes synchronously and may replace the current
    /// wrapper or later task slots through `owner`.
    fn target_acquisition_callback(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        prefix: TargetAcquisitionCallbackPrefix,
    ) -> Result<TargetAcquisitionCallbackResult, Self::Error>;

    /// Convert an opaque nonzero behavior result into this adapter's shared
    /// scheduler output. Retail forwards that pointer unchanged; keeping the
    /// conversion explicit avoids constraining unrelated detached adapters to
    /// use raw addresses as their output type.
    fn target_acquisition_propagated_result(
        &mut self,
        result: std::num::NonZeroU32,
    ) -> Self::Output;

    /// Run Follow Beacons' distinct ranked acquisition callback against the
    /// prefix committed immediately before callback entry.
    ///
    /// Live adapters should call
    /// [`crate::follow_beacons::evaluate_follow_beacon_acquisition_callback`].
    /// The current-style handoff executes synchronously and may replace this
    /// wrapper with variant one; phased owner traversal will then discard the
    /// retired wrapper's callback result automatically.
    fn follow_beacon_acquisition_callback(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        prefix: FollowBeaconAcquisitionCallbackPrefix,
    ) -> Result<FollowBeaconAcquisitionCallbackResult, Self::Error>;

    /// Convert an opaque nonzero current-style result without interpreting or
    /// remapping the retail value.
    fn follow_beacon_acquisition_propagated_result(
        &mut self,
        result: std::num::NonZeroU32,
    ) -> Self::Output;

    /// Evaluate Follow Beacons variant one's exact mover-first callback.
    ///
    /// Live adapters should call
    /// [`crate::follow_beacons::evaluate_follow_beacons_following_callback`]
    /// and map only its `CommonMover` error to
    /// [`FollowBeaconsFollowingAdapterError::CommonMover`]. Target, route, and
    /// post-mover position failures map to `AfterCommonMover`; this typed
    /// boundary keeps staged-private-state commit policy explicit.
    fn follow_beacons_following_callback(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        frame: ActorTaskDispatcherFrame,
        stage: &mut FollowBeaconsFollowingCallbackStage,
    ) -> Result<FollowBeaconsFollowingCallbackResult, FollowBeaconsFollowingAdapterError<Self::Error>>;

    /// Apply a known tagged-result or strict-timeout owner transition after
    /// callback unwind. The typed outcome preserves callback absence and the
    /// actor-state gate for the dispatcher's tag-specific lifetime fallback.
    fn follow_beacons_following_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        request: FollowBeaconsFollowingTransitionRequest,
    ) -> Result<FollowBeaconsFollowingTransitionOutcome<Self::Output>, Self::Error>;

    /// Classify a nonzero synchronous style result through the generic
    /// scheduler's tag/state-bit path. The raw result must not be treated as a
    /// final output without this adapter-owned classification.
    fn follow_beacons_following_style_result(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        request: FollowBeaconsFollowingStyleResultRequest,
    ) -> Result<FollowBeaconsFollowingStyleResultOutcome<Self::Output>, Self::Error>;

    /// Execute one exact Aim-and-Fire callback while the wrapper is marked as
    /// in-callback.
    ///
    /// The dispatcher performs the scheduler-mode gate before calling this
    /// method. Live adapters should use
    /// [`crate::aim_and_fire::evaluate_aim_and_fire_callback`] so target
    /// validation precedes optional sound/RNG and the generic emitter. The
    /// emitter may clear or replace task slots through `owner`; this owner's
    /// phased traversal then discards any callback result or error if the
    /// current wrapper did not survive.
    fn aim_and_fire_callback(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        frame: ActorTaskDispatcherFrame,
        private_state: AimAndFirePrivateState,
    ) -> Result<AimAndFireCallbackResult, Self::Error>;

    /// Attempt owner `+4` for tag `0x9C00` or owner `+0` for strict lifetime
    /// expiry after callback unwind.
    ///
    /// A completed callback exits the retail post-unwind path even when it
    /// returns zero. An absent or entity-state-suppressed `+4` callback allows
    /// the dispatcher to attempt `+0` only when the committed lifetime has
    /// already expired.
    fn aim_and_fire_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        request: AimAndFireTransitionRequest,
    ) -> Result<AimAndFireTransitionOutcome<Self::Output>, Self::Error>;

    /// Convert the exact nonzero generic-emitter result without remapping it.
    fn aim_and_fire_propagated_result(&mut self, result: std::num::NonZeroU32) -> Self::Output;

    /// Consume the dispatcher-supplied shared RNG draw and commit Guard
    /// Location's exact pre-selector context mutation.
    ///
    /// Live adapters must call
    /// [`GuardLocationAcquisitionTaskState::before_callback`] against the
    /// behavior-owned [`GuardLocationSearchContext`]. A successful return
    /// means that context write is committed even if candidate selection or
    /// handoff subsequently fails. The adapter must not consume another RNG
    /// value.
    fn guard_location_acquisition_prefix(
        &mut self,
        visit: ActorTaskVisit,
        task_state: GuardLocationAcquisitionTaskState,
        random: u32,
    ) -> Result<GuardLocationAcquisitionCallbackPrefix, Self::Error>;

    /// Run Guard Location's selector and synchronous behavior handoff against
    /// the already committed prefix.
    ///
    /// Live adapters should call
    /// [`crate::guard_location_owner::acquisition::evaluate_guard_location_acquisition_callback`].
    /// Intrusive-list snapshots, entity state, and the optional behavior
    /// callback remain outside the detached dispatcher.
    fn guard_location_acquisition_callback(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        prefix: GuardLocationAcquisitionCallbackPrefix,
    ) -> Result<GuardLocationAcquisitionCallbackResult, Self::Error>;

    /// Convert an opaque nonzero behavior result without interpreting or
    /// remapping the retail value.
    fn guard_location_acquisition_propagated_result(
        &mut self,
        result: std::num::NonZeroU32,
    ) -> Self::Output;

    /// Evaluate one exact Run Away flee callback against the supplied staged
    /// private task state.
    ///
    /// Live adapters should call [`crate::run_away::evaluate_run_away_callback`].
    /// Target lookup, route evaluation inputs, movement/controller storage,
    /// optional positional audio, and process RNG remain explicit adapter
    /// dependencies. They must be evaluated transactionally: task-private
    /// state is committed by this dispatcher only after an `Ok` return.
    fn run_away_callback(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        frame: ActorTaskDispatcherFrame,
        stage: &mut RunAwayCallbackStage,
    ) -> Result<RunAwayCallbackResult, Self::Error>;

    /// Apply Run Away's generic behavior-owner transition after callback
    /// unwind. `Some` propagates the owner result; `None` continues fresh-slot
    /// traversal.
    fn run_away_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        request: RunAwayTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error>;

    /// Resolve the current actor position consumed by shared callback
    /// `FUN_00402BA0`.
    fn shared_retarget_actor_position_raw(&mut self, visit: ActorTaskVisit) -> [i16; 3];

    /// Run the seven-input common mover against staged shared-retarget state.
    fn shared_retarget_common_mover(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        private_state: &mut WanderNearPrivateState,
    ) -> Result<WanderNearCommonMoverReturn, Self::Error>;

    /// Apply the current behavior owner's generic transition after callback
    /// unwind. Run Away Acquiring's concrete owner handoff remains a live
    /// adapter boundary rather than being inferred from the task address.
    fn shared_retarget_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        request: SharedRetargetTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error>;

    /// Resolve live actor/model inputs for the class-4 slot-2 callback.
    fn defecate_virus_terrain_request(
        &mut self,
        visit: ActorTaskVisit,
        elapsed_micros: u32,
    ) -> Result<DefecateVirusCallbackRequest, Self::Error>;

    /// Materialize one fully planned detailed/coarse terrain effect.
    ///
    /// Suppressed and chance-rejected callbacks are proven null paths in
    /// `FUN_00402850` and therefore never reach this effect boundary.
    fn apply_defecate_virus_terrain_plan(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        plan: DefecateVirusCallbackPlan,
    ) -> Result<(), Self::Error>;

    fn defecate_virus_terrain_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        request: DefecateVirusTerrainTransitionRequest,
    ) -> Result<DefecateVirusTransitionOutcome<Self::Output>, Self::Error>;

    fn defecate_virus_wander_actor_position_raw(&mut self, visit: ActorTaskVisit) -> [i16; 3];

    fn defecate_virus_wander_common_mover(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        private_state: &mut WanderNearPrivateState,
    ) -> Result<WanderNearCommonMoverReturn, Self::Error>;

    fn defecate_virus_wander_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        request: DefecateVirusWanderTransitionRequest,
    ) -> Result<DefecateVirusTransitionOutcome<Self::Output>, Self::Error>;

    /// Run one exact class-12 callback against live component storage.
    ///
    /// Live adapters should call [`crate::common_dying::tick_common_dying`]
    /// here. Effect selection, effect-before-mover ordering, ignored mover
    /// return, and X/Z damping belong to that detached callback; this
    /// dispatcher owns only wrapper lifetime and phase composition.
    fn common_dying_callback(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        frame: ActorTaskDispatcherFrame,
    ) -> Result<CommonDyingCallbackResult, Self::Error>;

    /// Apply the generic actor-owner transition requested after a surviving
    /// class-12 callback unwinds. `Some` propagates the owner result; `None`
    /// continues fresh-slot traversal.
    fn common_dying_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        request: CommonDyingTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActorTaskDispatcherError<E> {
    /// The named family still requires a receipt-bound or behavior-owned exact
    /// visit. The generic heterogeneous pass must fail before visiting any
    /// slot rather than silently skipping or partially executing the graph.
    DedicatedExactVisitRequired {
        visit: ActorTaskVisit,
        family: ActorTaskRuntimeFamily,
    },
    AttractAttentionCandidatePrefix {
        visit: ActorTaskVisit,
        error: E,
    },
    AttractAttentionCandidateCallback {
        visit: ActorTaskVisit,
        committed_prefix: GuardLocationAcquisitionCallbackPrefix,
        error: E,
    },
    AttractAttentionCandidateSurvivingAccepted {
        visit: ActorTaskVisit,
        committed_prefix: GuardLocationAcquisitionCallbackPrefix,
        candidate: GuardLocationCandidate,
        singleton: GuardLocationAcquisitionTaggedSingleton,
    },
    AttractAttentionCueOwner {
        visit: ActorTaskVisit,
        committed_prefix: OrdinaryType9AttractAttentionCueCallbackPrefix,
        error: E,
    },
    AttractAttentionCueTransition {
        request: OrdinaryType9AttractAttentionCueTransition,
        error: E,
    },
    OrdinaryWanderCommonMover {
        slot: ActorTaskSlot,
        committed_prefix: OrdinaryType9WanderCallbackPrefix,
        error: E,
    },
    OrdinaryWanderUnresolvedCommonMover {
        slot: ActorTaskSlot,
        committed_prefix: OrdinaryType9WanderCallbackPrefix,
    },
    OrdinaryWanderTransition {
        request: OrdinaryType9WanderTransitionRequest,
        error: E,
    },
    GoToJobCallback {
        visit: ActorTaskVisit,
        committed_prefix: GoToJobCallbackPrefix,
        error: E,
    },
    GoToJobTransition {
        request: GoToJobTransitionRequest,
        error: E,
    },
    ChaseTargetCallback {
        visit: ActorTaskVisit,
        committed_prefix: ChaseTargetCallbackPrefix,
        error: E,
    },
    ChaseTargetTransition {
        request: ChaseTargetTransitionRequest,
        error: E,
    },
    TargetAcquisitionCallback {
        visit: ActorTaskVisit,
        committed_prefix: TargetAcquisitionCallbackPrefix,
        error: E,
    },
    FollowBeaconAcquisitionCallback {
        visit: ActorTaskVisit,
        committed_prefix: FollowBeaconAcquisitionCallbackPrefix,
        error: E,
    },
    FollowBeaconsFollowingCallback {
        visit: ActorTaskVisit,
        committed_prefix: FollowBeaconsFollowingCallbackPrefix,
        error: FollowBeaconsFollowingAdapterError<E>,
    },
    FollowBeaconsFollowingTransition {
        request: FollowBeaconsFollowingTransitionRequest,
        error: E,
    },
    FollowBeaconsFollowingStyleResult {
        request: FollowBeaconsFollowingStyleResultRequest,
        error: E,
    },
    AimAndFireCallback {
        visit: ActorTaskVisit,
        committed_prefix: AimAndFireCallbackPrefix,
        error: E,
    },
    AimAndFireTransition {
        request: AimAndFireTransitionRequest,
        error: E,
    },
    GuardLocationAcquisitionPrefix {
        visit: ActorTaskVisit,
        error: E,
    },
    GuardLocationAcquisitionCallback {
        visit: ActorTaskVisit,
        committed_prefix: GuardLocationAcquisitionCallbackPrefix,
        error: E,
    },
    RunAwayCallback {
        visit: ActorTaskVisit,
        committed_prefix: RunAwayCallbackPrefix,
        error: E,
    },
    RunAwayTransition {
        request: RunAwayTransitionRequest,
        error: E,
    },
    SharedRetargetCommonMover {
        slot: ActorTaskSlot,
        committed_prefix: SharedRetargetCallbackPrefix,
        error: E,
    },
    SharedRetargetUnresolvedCommonMover {
        slot: ActorTaskSlot,
        committed_prefix: SharedRetargetCallbackPrefix,
    },
    SharedRetargetTransition {
        request: SharedRetargetTransitionRequest,
        error: E,
    },
    DefecateVirusTerrainRequest {
        visit: ActorTaskVisit,
        committed_prefix: DefecateVirusTerrainCallbackPrefix,
        error: E,
    },
    DefecateVirusTerrainModelExtentUnavailable {
        visit: ActorTaskVisit,
        committed_prefix: DefecateVirusTerrainCallbackPrefix,
        model_slot_index: u8,
    },
    DefecateVirusTerrainPlan {
        visit: ActorTaskVisit,
        committed_prefix: DefecateVirusTerrainCallbackPrefix,
        plan: DefecateVirusCallbackPlan,
        error: E,
    },
    DefecateVirusTerrainTransition {
        request: DefecateVirusTerrainTransitionRequest,
        error: E,
    },
    DefecateVirusWanderCommonMover {
        slot: ActorTaskSlot,
        committed_prefix: DefecateVirusWanderCallbackPrefix,
        error: E,
    },
    DefecateVirusWanderUnresolvedCommonMover {
        slot: ActorTaskSlot,
        committed_prefix: DefecateVirusWanderCallbackPrefix,
    },
    DefecateVirusWanderTransition {
        request: DefecateVirusWanderTransitionRequest,
        error: E,
    },
    CommonDyingCallback {
        visit: ActorTaskVisit,
        committed_prefix: CommonDyingCallbackPrefix,
        error: E,
    },
    CommonDyingTransition {
        request: CommonDyingTransitionRequest,
        error: E,
    },
}

#[derive(Debug)]
enum DispatchBefore {
    OrdinaryWander(crate::wander_near_location::WanderNearLifetimeStatus),
    AttractAttentionCandidate(AttractAttentionCandidateTaskState),
    AttractAttentionCue(OrdinaryType9AttractAttentionCueCallbackPrefix),
    GoToJob(GoToJobCallbackPrefix),
    ChaseTarget(ChaseTargetCallbackPrefix),
    TargetAcquisition(TargetAcquisitionCallbackPrefix),
    FollowBeaconAcquisition(FollowBeaconAcquisitionCallbackPrefix),
    FollowBeaconsFollowing(FollowBeaconsFollowingCallbackPrefix),
    AimAndFire(AimAndFireCallbackPrefix),
    GuardLocationAcquisition,
    RunAway(RunAwayCallbackPrefix),
    SharedRetarget(SharedRetargetLifetimePrefix),
    DefecateVirusTerrain(DefecateVirusTerrainCallbackPrefix),
    DefecateVirusWander(u32),
    CommonDying(CommonDyingCallbackPrefix),
}

#[derive(Debug)]
enum DefecateVirusTerrainCallback<E> {
    RequestError(E),
    ModelExtentUnavailable {
        model_slot_index: u8,
    },
    Planned {
        plan: DefecateVirusCallbackPlan,
        result: Result<(), E>,
    },
}

#[derive(Debug)]
enum GuardLocationAcquisitionCallback<E> {
    PrefixError(E),
    Evaluated {
        committed_prefix: GuardLocationAcquisitionCallbackPrefix,
        result: Result<GuardLocationAcquisitionCallbackResult, E>,
    },
}

#[derive(Debug)]
enum AttractAttentionCandidateCallback<E> {
    PrefixError(E),
    Evaluated {
        committed_prefix: GuardLocationAcquisitionCallbackPrefix,
        result: Result<GuardLocationAcquisitionCallbackResult, E>,
    },
}

#[derive(Debug)]
enum DispatchCallback<E> {
    OrdinaryWander {
        retarget: WanderNearRetarget,
        result: Result<WanderNearTaskCallbackResult, E>,
    },
    AttractAttentionCandidate(AttractAttentionCandidateCallback<E>),
    AttractAttentionCue,
    GoToJob {
        stage: GoToJobCallbackStage,
        result: Result<GoToJobCallbackResult, E>,
    },
    ChaseTarget {
        stage: ChaseTargetCallbackStage,
        result: Result<ChaseTargetCallbackResult, E>,
    },
    TargetAcquisition(Result<TargetAcquisitionCallbackResult, E>),
    FollowBeaconAcquisition(Result<FollowBeaconAcquisitionCallbackResult, E>),
    FollowBeaconsFollowing(
        Result<FollowBeaconsFollowingCallbackResult, FollowBeaconsFollowingAdapterError<E>>,
    ),
    AimAndFire(Result<AimAndFireCallbackResult, E>),
    GuardLocationAcquisition(GuardLocationAcquisitionCallback<E>),
    RunAway {
        stage: RunAwayCallbackStage,
        result: Result<RunAwayCallbackResult, E>,
    },
    SharedRetarget {
        retarget: SharedRetarget,
        result: Result<WanderNearCommonMoverReturn, E>,
    },
    DefecateVirusTerrain(DefecateVirusTerrainCallback<E>),
    DefecateVirusWander {
        retarget: crate::defecate_virus::DefecateVirusWanderRetarget,
        result: Result<WanderNearCommonMoverReturn, E>,
    },
    CommonDying(Result<CommonDyingCallbackResult, E>),
}

/// Tick one heterogeneous three-slot owner in retail order.
///
/// Family-specific live prefixes are committed before callback entry.
/// Common-mover/private-state writes are staged according to each recovered
/// task contract, and post-unwind transitions may replace a later family
/// without making this pass use a stale slot snapshot.
pub fn tick_actor_task_dispatcher<Adapter: ActorTaskDispatcherAdapter>(
    owner: &mut ActorTaskOwner<ActorTaskRuntime>,
    frame: ActorTaskDispatcherFrame,
    adapter: &mut Adapter,
) -> Result<Option<Adapter::Output>, ActorTaskDispatcherError<Adapter::Error>> {
    tick_actor_task_dispatcher_from_slot(owner, frame, ActorTaskSlot::Primary, adapter)
}

/// Resume one already-entered A800 pass at an exact later physical slot.
///
/// The normal entry above remains Primary-first. This seam exists for linear
/// owners which retain a post-Primary continuation across a retryable external
/// preflight; it never replays the skipped prefix or an earlier slot.
pub fn tick_actor_task_dispatcher_from_slot<Adapter: ActorTaskDispatcherAdapter>(
    owner: &mut ActorTaskOwner<ActorTaskRuntime>,
    frame: ActorTaskDispatcherFrame,
    start_slot: ActorTaskSlot,
    adapter: &mut Adapter,
) -> Result<Option<Adapter::Output>, ActorTaskDispatcherError<Adapter::Error>> {
    for &slot in start_slot.retail_tick_suffix() {
        let dedicated_family = match owner.state_in_slot(slot) {
            Some(ActorTaskRuntime::None) => Some(ActorTaskRuntimeFamily::None),
            Some(ActorTaskRuntime::AttractAttentionTargetRoute(_)) => {
                Some(ActorTaskRuntimeFamily::AttractAttentionTargetRoute)
            }
            Some(ActorTaskRuntime::CapturePeoplePursuit(_)) => {
                Some(ActorTaskRuntimeFamily::CapturePeoplePursuit)
            }
            Some(ActorTaskRuntime::FishTargetRoute(_)) => {
                Some(ActorTaskRuntimeFamily::FishTargetRoute)
            }
            Some(ActorTaskRuntime::CleansingLandscape(_)) => {
                Some(ActorTaskRuntimeFamily::CleansingLandscape)
            }
            Some(ActorTaskRuntime::TerrainCleansing(_)) => {
                Some(ActorTaskRuntimeFamily::TerrainCleansing)
            }
            Some(ActorTaskRuntime::CaptureBeaconAcquisition) => {
                Some(ActorTaskRuntimeFamily::CaptureBeaconAcquisition)
            }
            Some(ActorTaskRuntime::CapturePeopleFollowing(_)) => {
                Some(ActorTaskRuntimeFamily::CapturePeopleFollowing)
            }
            Some(ActorTaskRuntime::ChangeSeaLevel(_)) => {
                Some(ActorTaskRuntimeFamily::ChangeSeaLevel)
            }
            Some(ActorTaskRuntime::WorkingFactory(_)) => {
                Some(ActorTaskRuntimeFamily::WorkingFactory)
            }
            Some(ActorTaskRuntime::MainBase(_)) => Some(ActorTaskRuntimeFamily::MainBase),
            Some(ActorTaskRuntime::Intro2GunTurret(_)) => {
                Some(ActorTaskRuntimeFamily::Intro2GunTurret)
            }
            Some(ActorTaskRuntime::Class0Timer(_)) => Some(ActorTaskRuntimeFamily::Class0Timer),
            Some(ActorTaskRuntime::ExplodingRing(_)) => Some(ActorTaskRuntimeFamily::ExplodingRing),
            Some(ActorTaskRuntime::HiveRadial(_)) => Some(ActorTaskRuntimeFamily::HiveRadial),
            Some(ActorTaskRuntime::BoulderRolling(_)) => {
                Some(ActorTaskRuntimeFamily::BoulderRolling)
            }
            Some(ActorTaskRuntime::BoulderResting(_)) => {
                Some(ActorTaskRuntimeFamily::BoulderResting)
            }
            Some(ActorTaskRuntime::TrailingFire(_)) => Some(ActorTaskRuntimeFamily::TrailingFire),
            Some(ActorTaskRuntime::RocketFlight(_)) => Some(ActorTaskRuntimeFamily::RocketFlight),
            Some(ActorTaskRuntime::RocketTrail) => Some(ActorTaskRuntimeFamily::RocketTrail),
            Some(ActorTaskRuntime::TrashFurniture(_)) => {
                Some(ActorTaskRuntimeFamily::TrashFurniture)
            }
            Some(ActorTaskRuntime::TumbleOutOfSky(_)) => {
                Some(ActorTaskRuntimeFamily::TumbleOutOfSky)
            }
            Some(ActorTaskRuntime::Intro2Type57Tumble(_)) => {
                Some(ActorTaskRuntimeFamily::Intro2Type57Tumble)
            }
            _ => None,
        };
        if let Some(family) = dedicated_family {
            let task_id = owner
                .task_in_slot(slot)
                .expect("a live dedicated task state has a task wrapper");
            return Err(ActorTaskDispatcherError::DedicatedExactVisitRequired {
                visit: ActorTaskVisit { slot, task_id },
                family,
            });
        }
    }

    let propagated = owner.visit_slots_fresh_phased_with_from(
        start_slot,
        adapter,
        |_adapter, state, _visit| match state {
            ActorTaskRuntime::None => {
                unreachable!("None requires the relation owner's linked Sub-I visit")
            }
            ActorTaskRuntime::OrdinaryType9Wander(state) => {
                DispatchBefore::OrdinaryWander(state.before_callback(frame.elapsed_micros))
            }
            ActorTaskRuntime::AttractAttentionCandidate(state) => {
                DispatchBefore::AttractAttentionCandidate(*state)
            }
            ActorTaskRuntime::AttractAttentionCue(cue) => {
                cue.advance_elapsed(frame.elapsed_micros);
                DispatchBefore::AttractAttentionCue(
                    OrdinaryType9AttractAttentionCueCallbackPrefix {
                        elapsed_ms: cue.elapsed_ms(),
                        lifetime_ms: cue.lifetime_ms(),
                    },
                )
            }
            ActorTaskRuntime::AttractAttentionTargetRoute(_) => {
                unreachable!("Attract target route was rejected before generic task traversal")
            }
            ActorTaskRuntime::CapturePeoplePursuit(_)
            | ActorTaskRuntime::FishTargetRoute(_)
            | ActorTaskRuntime::CleansingLandscape(_)
            | ActorTaskRuntime::TerrainCleansing(_)
            | ActorTaskRuntime::CaptureBeaconAcquisition
            | ActorTaskRuntime::CapturePeopleFollowing(_) => {
                unreachable!("targeted pursuit requires its native owner exact visit")
            }
            ActorTaskRuntime::GoToJob(state) => {
                DispatchBefore::GoToJob(state.before_callback(frame.elapsed_micros))
            }
            ActorTaskRuntime::ChaseTarget(state) => {
                DispatchBefore::ChaseTarget(state.before_callback(frame.elapsed_micros))
            }
            ActorTaskRuntime::TargetAcquisition(state) => {
                DispatchBefore::TargetAcquisition(state.before_callback())
            }
            ActorTaskRuntime::FollowBeaconAcquisition(state) => {
                DispatchBefore::FollowBeaconAcquisition(state.before_callback())
            }
            ActorTaskRuntime::FollowBeaconsFollowing(state) => {
                DispatchBefore::FollowBeaconsFollowing(
                    state.before_callback(frame.elapsed_micros),
                )
            }
            ActorTaskRuntime::AimAndFire(state) => {
                DispatchBefore::AimAndFire(state.before_callback(frame.elapsed_micros))
            }
            ActorTaskRuntime::GuardLocationAcquisition(_) => {
                DispatchBefore::GuardLocationAcquisition
            }
            ActorTaskRuntime::RunAway(state) => {
                DispatchBefore::RunAway(state.before_callback(frame.elapsed_micros))
            }
            ActorTaskRuntime::SharedRetarget(state) => {
                DispatchBefore::SharedRetarget(state.before_callback(frame.elapsed_micros))
            }
            ActorTaskRuntime::DefecateVirusTerrain(state) => {
                DispatchBefore::DefecateVirusTerrain(state.before_callback(frame.elapsed_micros))
            }
            ActorTaskRuntime::DefecateVirusWander(state) => {
                DispatchBefore::DefecateVirusWander(state.before_callback(frame.elapsed_micros))
            }
            ActorTaskRuntime::CommonDying(state) => {
                DispatchBefore::CommonDying(state.before_callback(frame.elapsed_micros))
            }
            ActorTaskRuntime::ChangeSeaLevel(_) => {
                unreachable!("Change-Sea-Level was rejected before generic task traversal")
            }
            ActorTaskRuntime::WorkingFactory(_) => {
                unreachable!("Working-Factory was rejected before generic task traversal")
            }
            ActorTaskRuntime::MainBase(_) => unreachable!("Main Base requires its native visit"),
            ActorTaskRuntime::Intro2GunTurret(_) => unreachable!("Turret requires its native visit"),
            ActorTaskRuntime::Class0Timer(_) => unreachable!("Class 0 timer requires its native visit"),
            ActorTaskRuntime::ExplodingRing(_) => {
                unreachable!("Exploding-Ring was rejected before generic task traversal")
            }
            ActorTaskRuntime::HiveRadial(_) => {
                unreachable!("Hive-Radial was rejected before generic task traversal")
            }
            ActorTaskRuntime::BoulderRolling(_) | ActorTaskRuntime::TrailingFire(_) => {
                unreachable!("Boulder tasks require the exact Intro2 meteor visit")
            }
            ActorTaskRuntime::BoulderResting(_) => {
                unreachable!("Resting boulders require their native class20 visit")
            }
            ActorTaskRuntime::RocketFlight(_) | ActorTaskRuntime::RocketTrail => {
                unreachable!("Entity weapons require their native body visit")
            }
            ActorTaskRuntime::TrashFurniture(_) => unreachable!("Furniture requires its exact native visit"),
            ActorTaskRuntime::TumbleOutOfSky(_) | ActorTaskRuntime::Intro2Type57Tumble(_) => {
                unreachable!("Tumble requires its exact native visit")
            }
        },
        |adapter, owner, visit| match owner
            .task_state(visit.task_id)
            .expect("a live callback must retain its inner task state")
            .family()
        {
            ActorTaskRuntimeFamily::OrdinaryType9Wander => {
                let anchor = adapter.ordinary_wander_anchor_raw(visit);
                let mut stage = {
                    let state = ordinary_wander_state_mut(owner, visit);
                    state.stage_callback(anchor, || adapter.next_random())
                };
                let retarget = stage.retarget();
                let mover =
                    adapter.ordinary_wander_common_mover(owner, visit, stage.private_state_mut());
                if matches!(
                    &mover,
                    Ok(WanderNearCommonMoverReturn::NonZero | WanderNearCommonMoverReturn::Zero)
                ) {
                    if let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) =
                        owner.task_state_mut(visit.task_id)
                    {
                        stage.commit(state);
                    }
                }
                DispatchCallback::OrdinaryWander {
                    retarget,
                    result: mover.map(map_common_mover_return),
                }
            }
            ActorTaskRuntimeFamily::AttractAttentionCandidate => {
                let task_state = match owner.task_state(visit.task_id) {
                    Some(ActorTaskRuntime::AttractAttentionCandidate(state)) => *state,
                    _ => unreachable!("candidate visit retained its runtime family"),
                };
                let random = adapter.next_random();
                let callback = match adapter
                    .attract_attention_candidate_prefix(owner, visit, task_state, random)
                {
                    Err(error) => AttractAttentionCandidateCallback::PrefixError(error),
                    Ok(committed_prefix) => {
                        let result = adapter.attract_attention_candidate_callback(
                            owner,
                            visit,
                            committed_prefix,
                        );
                        AttractAttentionCandidateCallback::Evaluated {
                            committed_prefix,
                            result,
                        }
                    }
                };
                DispatchCallback::AttractAttentionCandidate(callback)
            }
            ActorTaskRuntimeFamily::AttractAttentionCue => {
                // Retail `FUN_00438050` is an explicit, unconditional zero
                // leaf. Duration interpretation belongs to post-unwind.
                DispatchCallback::AttractAttentionCue
            }
            ActorTaskRuntimeFamily::AttractAttentionTargetRoute => {
                unreachable!("Attract target route was rejected before generic task traversal")
            }
            ActorTaskRuntimeFamily::CapturePeoplePursuit
            | ActorTaskRuntimeFamily::FishTargetRoute
            | ActorTaskRuntimeFamily::CleansingLandscape
            | ActorTaskRuntimeFamily::TerrainCleansing
            | ActorTaskRuntimeFamily::CaptureBeaconAcquisition
            | ActorTaskRuntimeFamily::CapturePeopleFollowing => {
                unreachable!("targeted pursuit requires its native owner exact visit")
            }
            ActorTaskRuntimeFamily::GoToJob => {
                let mut stage = go_to_job_state(owner, visit).stage_callback();
                let result = adapter.go_to_job_callback(owner, visit, &mut stage);
                DispatchCallback::GoToJob { stage, result }
            }
            ActorTaskRuntimeFamily::ChaseTarget => {
                let mut stage = chase_target_state(owner, visit).stage_callback();
                let result = adapter.chase_target_callback(owner, visit, frame, &mut stage);
                DispatchCallback::ChaseTarget { stage, result }
            }
            ActorTaskRuntimeFamily::TargetAcquisition => {
                let prefix = target_acquisition_state(owner, visit);
                DispatchCallback::TargetAcquisition(
                    adapter.target_acquisition_callback(owner, visit, prefix),
                )
            }
            ActorTaskRuntimeFamily::FollowBeaconAcquisition => {
                let prefix = follow_beacon_acquisition_state(owner, visit);
                DispatchCallback::FollowBeaconAcquisition(
                    adapter.follow_beacon_acquisition_callback(owner, visit, prefix),
                )
            }
            ActorTaskRuntimeFamily::FollowBeaconsFollowing => {
                let mut stage = follow_beacons_following_state(owner, visit).stage_callback();
                let result =
                    adapter.follow_beacons_following_callback(owner, visit, frame, &mut stage);
                if matches!(
                    &result,
                    Ok(_) | Err(FollowBeaconsFollowingAdapterError::AfterCommonMover(_))
                ) {
                    if let Some(ActorTaskRuntime::FollowBeaconsFollowing(state)) =
                        owner.task_state_mut(visit.task_id)
                    {
                        stage.commit(state);
                    }
                }
                DispatchCallback::FollowBeaconsFollowing(result)
            }
            ActorTaskRuntimeFamily::AimAndFire => {
                let result = if frame.scheduler_mode != 0 {
                    Ok(AimAndFireCallbackResult::Zero)
                } else {
                    let private_state = aim_and_fire_private_state(owner, visit);
                    adapter.aim_and_fire_callback(owner, visit, frame, private_state)
                };
                DispatchCallback::AimAndFire(result)
            }
            ActorTaskRuntimeFamily::GuardLocationAcquisition => {
                let task_state = guard_location_acquisition_state(owner, visit);
                let random = adapter.next_random();
                let callback =
                    match adapter.guard_location_acquisition_prefix(visit, task_state, random) {
                        Err(error) => GuardLocationAcquisitionCallback::PrefixError(error),
                        Ok(committed_prefix) => {
                            let result = adapter.guard_location_acquisition_callback(
                                owner,
                                visit,
                                committed_prefix,
                            );
                            GuardLocationAcquisitionCallback::Evaluated {
                                committed_prefix,
                                result,
                            }
                        }
                };
                DispatchCallback::GuardLocationAcquisition(callback)
            }
            ActorTaskRuntimeFamily::RunAway => {
                let mut stage = run_away_state(owner, visit).stage_callback();
                let result = adapter.run_away_callback(owner, visit, frame, &mut stage);
                DispatchCallback::RunAway { stage, result }
            }
            ActorTaskRuntimeFamily::SharedRetarget => {
                let actor_position_raw = adapter.shared_retarget_actor_position_raw(visit);
                let mut stage = {
                    let state = shared_retarget_state_mut(owner, visit);
                    state.stage_callback(actor_position_raw, || adapter.next_random() as u16)
                };
                let retarget = stage.retarget();
                let mover = adapter.shared_retarget_common_mover(
                    owner,
                    visit,
                    stage.private_state_mut(),
                );
                if matches!(
                    &mover,
                    Ok(WanderNearCommonMoverReturn::NonZero | WanderNearCommonMoverReturn::Zero)
                ) {
                    if let Some(ActorTaskRuntime::SharedRetarget(state)) =
                        owner.task_state_mut(visit.task_id)
                    {
                        state.commit_callback_stage(stage);
                    }
                }
                DispatchCallback::SharedRetarget {
                    retarget,
                    result: mover,
                }
            }
            ActorTaskRuntimeFamily::DefecateVirusTerrain => {
                let callback = match adapter
                    .defecate_virus_terrain_request(visit, frame.elapsed_micros)
                {
                    Err(error) => DefecateVirusTerrainCallback::RequestError(error),
                    Ok(mut request) => {
                        // `FUN_00401120` forwards the same frame duration to
                        // scheduler accounting and `FUN_00402850`. Do not let
                        // a live-data adapter manufacture a second duration.
                        request.elapsed_micros = frame.elapsed_micros;
                        let plan =
                            plan_defecate_virus_callback(request, || adapter.next_random() as u16);
                        match plan {
                            DefecateVirusCallbackPlan::SuppressedByEntityState
                            | DefecateVirusCallbackPlan::DetailedChanceRejected => {
                                DefecateVirusTerrainCallback::Planned {
                                    plan,
                                    result: Ok(()),
                                }
                            }
                            DefecateVirusCallbackPlan::DetailedModelExtentUnavailable {
                                model_slot_index,
                            } => DefecateVirusTerrainCallback::ModelExtentUnavailable {
                                model_slot_index,
                            },
                            plan => {
                                let result =
                                    adapter.apply_defecate_virus_terrain_plan(owner, visit, plan);
                                DefecateVirusTerrainCallback::Planned { plan, result }
                            }
                        }
                    }
                };
                DispatchCallback::DefecateVirusTerrain(callback)
            }
            ActorTaskRuntimeFamily::DefecateVirusWander => {
                let actor_position_raw = adapter.defecate_virus_wander_actor_position_raw(visit);
                let mut stage = {
                    let state = defecate_virus_wander_state_mut(owner, visit);
                    state.stage_callback(actor_position_raw, || adapter.next_random() as u16)
                };
                let retarget = stage.retarget();
                let mover = adapter.defecate_virus_wander_common_mover(
                    owner,
                    visit,
                    stage.private_state_mut(),
                );
                if matches!(
                    &mover,
                    Ok(WanderNearCommonMoverReturn::NonZero | WanderNearCommonMoverReturn::Zero)
                ) {
                    if let Some(ActorTaskRuntime::DefecateVirusWander(state)) =
                        owner.task_state_mut(visit.task_id)
                    {
                        stage.commit(state);
                    }
                }
                DispatchCallback::DefecateVirusWander {
                    retarget,
                    result: mover,
                }
            }
            ActorTaskRuntimeFamily::CommonDying => {
                DispatchCallback::CommonDying(adapter.common_dying_callback(owner, visit, frame))
            }
            ActorTaskRuntimeFamily::None => {
                unreachable!("None requires the relation owner's linked Sub-I visit")
            }
            ActorTaskRuntimeFamily::ChangeSeaLevel => {
                unreachable!("Change-Sea-Level was rejected before generic task traversal")
            }
            ActorTaskRuntimeFamily::WorkingFactory => {
                unreachable!("Working-Factory was rejected before generic task traversal")
            }
            ActorTaskRuntimeFamily::MainBase => unreachable!("Main Base requires its native visit"),
            ActorTaskRuntimeFamily::Intro2GunTurret => unreachable!("Turret requires its native visit"),
            ActorTaskRuntimeFamily::Class0Timer => unreachable!("Class 0 timer requires its native visit"),
            ActorTaskRuntimeFamily::ExplodingRing => {
                unreachable!("Exploding-Ring was rejected before generic task traversal")
            }
            ActorTaskRuntimeFamily::HiveRadial => {
                unreachable!("Hive-Radial was rejected before generic task traversal")
            }
            ActorTaskRuntimeFamily::BoulderRolling | ActorTaskRuntimeFamily::TrailingFire => {
                unreachable!("Boulder tasks require the exact Intro2 meteor visit")
            }
            ActorTaskRuntimeFamily::BoulderResting => {
                unreachable!("Resting boulders require their native class20 visit")
            }
            ActorTaskRuntimeFamily::RocketFlight | ActorTaskRuntimeFamily::RocketTrail => {
                unreachable!("Entity weapons require their native body visit")
            }
            ActorTaskRuntimeFamily::TrashFurniture => unreachable!("Furniture requires its exact native visit"),
            ActorTaskRuntimeFamily::TumbleOutOfSky | ActorTaskRuntimeFamily::Intro2Type57Tumble => {
                unreachable!("Tumble requires its exact native visit")
            }
        },
        |adapter, owner, visit, before, callback| match (before, callback) {
            (
                DispatchBefore::AttractAttentionCandidate(_task_state),
                DispatchCallback::AttractAttentionCandidate(callback),
            ) => {
                assert_surviving_family(
                    owner,
                    visit,
                    ActorTaskRuntimeFamily::AttractAttentionCandidate,
                );
                let (committed_prefix, callback_result) = match callback {
                    AttractAttentionCandidateCallback::PrefixError(error) => {
                        return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::AttractAttentionCandidatePrefix {
                                visit,
                                error,
                            },
                        ));
                    }
                    AttractAttentionCandidateCallback::Evaluated {
                        committed_prefix,
                        result: Err(error),
                    } => {
                        return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::AttractAttentionCandidateCallback {
                                visit,
                                committed_prefix,
                                error,
                            },
                        ));
                    }
                    AttractAttentionCandidateCallback::Evaluated {
                        committed_prefix,
                        result: Ok(result),
                    } => (committed_prefix, result),
                };
                match callback_result {
                    GuardLocationAcquisitionCallbackResult::Zero(_) => {
                        ActorTaskVisitControl::Continue
                    }
                    GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted {
                        candidate,
                        singleton,
                    } => ActorTaskVisitControl::Propagate(Err(
                        ActorTaskDispatcherError::AttractAttentionCandidateSurvivingAccepted {
                            visit,
                            committed_prefix,
                            candidate,
                            singleton,
                        },
                    )),
                    GuardLocationAcquisitionCallbackResult::PropagateBehaviorResult {
                        result,
                        ..
                    } => ActorTaskVisitControl::Propagate(Ok(
                        adapter.attract_attention_candidate_propagated_result(result),
                    )),
                }
            }
            (
                DispatchBefore::AttractAttentionCue(committed_prefix),
                DispatchCallback::AttractAttentionCue,
            ) => {
                assert_surviving_family(owner, visit, ActorTaskRuntimeFamily::AttractAttentionCue);
                if committed_prefix.lifetime_status()
                    == OrdinaryType9AttractAttentionCueLifetimeStatus::Active
                {
                    return ActorTaskVisitControl::Continue;
                }
                let entity_id = match adapter.attract_attention_cue_transition_owner_id(
                    owner,
                    visit,
                    committed_prefix,
                ) {
                    Ok(entity_id) => entity_id,
                    Err(error) => {
                        return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::AttractAttentionCueOwner {
                                visit,
                                committed_prefix,
                                error,
                            },
                        ));
                    }
                };
                let request = OrdinaryType9AttractAttentionCueTransition {
                    entity_id,
                    expired_visit: visit,
                    committed_prefix,
                    selection:
                        OrdinaryType9AttractAttentionCueTransitionSelection::TypeDefaultRootFirst,
                };
                match adapter.attract_attention_cue_transition(owner, request) {
                    Ok(AttractAttentionCueTransitionOutcome::Completed(Some(value))) => {
                        ActorTaskVisitControl::Propagate(Ok(value))
                    }
                    Ok(
                        AttractAttentionCueTransitionOutcome::Completed(None)
                        | AttractAttentionCueTransitionOutcome::CallbackAbsent
                        | AttractAttentionCueTransitionOutcome::SuppressedByEntityState,
                    ) => ActorTaskVisitControl::Continue,
                    Err(error) => ActorTaskVisitControl::Propagate(Err(
                        ActorTaskDispatcherError::AttractAttentionCueTransition {
                            request,
                            error,
                        },
                    )),
                }
            }
            (
                DispatchBefore::OrdinaryWander(lifetime_status),
                DispatchCallback::OrdinaryWander { retarget, result },
            ) => {
                assert_surviving_family(owner, visit, ActorTaskRuntimeFamily::OrdinaryType9Wander);
                let committed_prefix = OrdinaryType9WanderCallbackPrefix {
                    lifetime_status,
                    retarget,
                };
                let callback_result = match result {
                    Ok(result) => result,
                    Err(error) => {
                        return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::OrdinaryWanderCommonMover {
                                slot: visit.slot,
                                committed_prefix,
                                error,
                            },
                        ));
                    }
                };
                let transition_request = match ordinary_type9_wander_after_unwind(
                    visit,
                    committed_prefix,
                    callback_result,
                ) {
                    OrdinaryType9WanderPostUnwind::Continue => {
                        return ActorTaskVisitControl::Continue;
                    }
                    OrdinaryType9WanderPostUnwind::UnresolvedCommonMover => {
                        return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::OrdinaryWanderUnresolvedCommonMover {
                                slot: visit.slot,
                                committed_prefix,
                            },
                        ));
                    }
                    OrdinaryType9WanderPostUnwind::Transition(request) => request,
                };
                match adapter.ordinary_wander_transition(owner, transition_request) {
                    Ok(OrdinaryType9WanderTransitionOutcome::Completed(Some(value))) => {
                        ActorTaskVisitControl::Propagate(Ok(value))
                    }
                    Ok(
                        OrdinaryType9WanderTransitionOutcome::CallbackAbsent
                        | OrdinaryType9WanderTransitionOutcome::SuppressedByEntityState
                        | OrdinaryType9WanderTransitionOutcome::Completed(None),
                    ) => ActorTaskVisitControl::Continue,
                    Err(error) => ActorTaskVisitControl::Propagate(Err(
                        ActorTaskDispatcherError::OrdinaryWanderTransition {
                            request: transition_request,
                            error,
                        },
                    )),
                }
            }
            (
                DispatchBefore::GoToJob(committed_prefix),
                DispatchCallback::GoToJob { stage, result },
            ) => {
                assert_surviving_family(owner, visit, ActorTaskRuntimeFamily::GoToJob);
                stage.commit(go_to_job_state_mut(owner, visit));
                let callback_result = match result {
                    Ok(result) => result,
                    Err(error) => {
                        return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::GoToJobCallback {
                                visit,
                                committed_prefix,
                                error,
                            },
                        ));
                    }
                };
                let Some(transition_request) =
                    go_to_job_transition_after_unwind(visit, committed_prefix, callback_result)
                else {
                    return ActorTaskVisitControl::Continue;
                };
                match adapter.go_to_job_transition(owner, transition_request) {
                    Ok(Some(value)) => ActorTaskVisitControl::Propagate(Ok(value)),
                    Ok(None) => ActorTaskVisitControl::Continue,
                    Err(error) => ActorTaskVisitControl::Propagate(Err(
                        ActorTaskDispatcherError::GoToJobTransition {
                            request: transition_request,
                            error,
                        },
                    )),
                }
            }
            (
                DispatchBefore::ChaseTarget(committed_prefix),
                DispatchCallback::ChaseTarget { stage, result },
            ) => {
                assert_surviving_family(owner, visit, ActorTaskRuntimeFamily::ChaseTarget);
                let callback_result = match result {
                    Ok(result) => {
                        stage.commit(chase_target_state_mut(owner, visit));
                        result
                    }
                    Err(error) => {
                        return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::ChaseTargetCallback {
                                visit,
                                committed_prefix,
                                error,
                            },
                        ));
                    }
                };
                let Some(transition_request) =
                    chase_target_transition_after_unwind(visit, committed_prefix, callback_result)
                else {
                    return ActorTaskVisitControl::Continue;
                };
                match adapter.chase_target_transition(owner, transition_request) {
                    Ok(Some(value)) => ActorTaskVisitControl::Propagate(Ok(value)),
                    Ok(None) => ActorTaskVisitControl::Continue,
                    Err(error) => ActorTaskVisitControl::Propagate(Err(
                        ActorTaskDispatcherError::ChaseTargetTransition {
                            request: transition_request,
                            error,
                        },
                    )),
                }
            }
            (
                DispatchBefore::TargetAcquisition(committed_prefix),
                DispatchCallback::TargetAcquisition(result),
            ) => {
                assert_surviving_family(
                    owner,
                    visit,
                    ActorTaskRuntimeFamily::TargetAcquisition,
                );
                match result {
                    Err(error) => ActorTaskVisitControl::Propagate(Err(
                        ActorTaskDispatcherError::TargetAcquisitionCallback {
                            visit,
                            committed_prefix,
                            error,
                        },
                    )),
                    Ok(
                        TargetAcquisitionCallbackResult::Zero(_)
                        | TargetAcquisitionCallbackResult::TaggedTargetAccepted { .. },
                    ) => ActorTaskVisitControl::Continue,
                    Ok(TargetAcquisitionCallbackResult::PropagateBehaviorResult {
                        result,
                        ..
                    }) => ActorTaskVisitControl::Propagate(Ok(
                        adapter.target_acquisition_propagated_result(result)
                    )),
                }
            }
            (
                DispatchBefore::FollowBeaconAcquisition(committed_prefix),
                DispatchCallback::FollowBeaconAcquisition(result),
            ) => {
                assert_surviving_family(
                    owner,
                    visit,
                    ActorTaskRuntimeFamily::FollowBeaconAcquisition,
                );
                match result {
                    Err(error) => ActorTaskVisitControl::Propagate(Err(
                        ActorTaskDispatcherError::FollowBeaconAcquisitionCallback {
                            visit,
                            committed_prefix,
                            error,
                        },
                    )),
                    Ok(
                        FollowBeaconAcquisitionCallbackResult::Zero(_)
                        | FollowBeaconAcquisitionCallbackResult::TaggedTargetAccepted { .. },
                    ) => ActorTaskVisitControl::Continue,
                    Ok(FollowBeaconAcquisitionCallbackResult::PropagateStyleResult {
                        result,
                        ..
                    }) => ActorTaskVisitControl::Propagate(Ok(
                        adapter.follow_beacon_acquisition_propagated_result(result),
                    )),
                }
            }
            (
                DispatchBefore::FollowBeaconsFollowing(committed_prefix),
                DispatchCallback::FollowBeaconsFollowing(result),
            ) => {
                assert_surviving_family(
                    owner,
                    visit,
                    ActorTaskRuntimeFamily::FollowBeaconsFollowing,
                );
                let callback_result = match result {
                    Ok(result) => result,
                    Err(error) => {
                        return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::FollowBeaconsFollowingCallback {
                                visit,
                                committed_prefix,
                                error,
                            },
                        ));
                    }
                };
                match follow_beacons_following_after_unwind(
                    visit,
                    committed_prefix,
                    callback_result,
                ) {
                    FollowBeaconsFollowingPostUnwind::Continue => {
                        ActorTaskVisitControl::Continue
                    }
                    FollowBeaconsFollowingPostUnwind::Transition(request) => {
                        apply_follow_beacons_following_transition(adapter, owner, request)
                    }
                    FollowBeaconsFollowingPostUnwind::PropagateStyleResult { result } => {
                        let request = FollowBeaconsFollowingStyleResultRequest {
                            visit,
                            committed_prefix,
                            result,
                        };
                        match adapter.follow_beacons_following_style_result(owner, request) {
                            Ok(FollowBeaconsFollowingStyleResultOutcome::Completed(Some(value))
                            | FollowBeaconsFollowingStyleResultOutcome::Propagate(value)) => {
                                ActorTaskVisitControl::Propagate(Ok(value))
                            }
                            Ok(
                                FollowBeaconsFollowingStyleResultOutcome::Completed(None)
                                | FollowBeaconsFollowingStyleResultOutcome::ReturnZero,
                            ) => ActorTaskVisitControl::Continue,
                            Ok(FollowBeaconsFollowingStyleResultOutcome::FallThroughLifetime) => {
                                if committed_prefix.lifetime_status
                                    == FollowBeaconsFollowingLifetimeStatus::OwnerTransitionDue
                                {
                                    apply_follow_beacons_following_transition(
                                        adapter,
                                        owner,
                                        FollowBeaconsFollowingTransitionRequest {
                                            slot: visit.slot,
                                            task_id: visit.task_id,
                                            reason:
                                                FollowBeaconsFollowingTransitionReason::LifetimeExpired,
                                            committed_prefix,
                                        },
                                    )
                                } else {
                                    ActorTaskVisitControl::Continue
                                }
                            }
                            Err(error) => ActorTaskVisitControl::Propagate(Err(
                                ActorTaskDispatcherError::FollowBeaconsFollowingStyleResult {
                                    request,
                                    error,
                                },
                            )),
                        }
                    }
                }
            }
            (
                DispatchBefore::AimAndFire(committed_prefix),
                DispatchCallback::AimAndFire(result),
            ) => {
                assert_surviving_family(owner, visit, ActorTaskRuntimeFamily::AimAndFire);
                let callback_result = match result {
                    Ok(result) => result,
                    Err(error) => {
                        return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::AimAndFireCallback {
                                visit,
                                committed_prefix,
                                error,
                            },
                        ));
                    }
                };
                match aim_and_fire_after_unwind(committed_prefix, callback_result).outcome {
                    AimAndFireFrameOutcome::Continue => ActorTaskVisitControl::Continue,
                    AimAndFireFrameOutcome::ReturnGenericEmitterResult(result) => {
                        ActorTaskVisitControl::Propagate(Ok(
                            adapter.aim_and_fire_propagated_result(result)
                        ))
                    }
                    AimAndFireFrameOutcome::RequestOwnerTransition { reason } => {
                        let request = AimAndFireTransitionRequest {
                            visit,
                            reason,
                            committed_prefix,
                        };
                        match adapter.aim_and_fire_transition(owner, request) {
                            Ok(AimAndFireTransitionOutcome::Completed(Some(value))) => {
                                ActorTaskVisitControl::Propagate(Ok(value))
                            }
                            Ok(AimAndFireTransitionOutcome::Completed(None)) => {
                                ActorTaskVisitControl::Continue
                            }
                            Ok(
                                AimAndFireTransitionOutcome::CallbackAbsent
                                | AimAndFireTransitionOutcome::SuppressedByEntityState,
                            ) if matches!(
                                reason,
                                AimAndFireTransitionReason::TaggedInvalidTarget { .. }
                            ) && committed_prefix.lifetime_status
                                == crate::aim_and_fire::AimAndFireLifetimeStatus::OwnerTransitionDue =>
                            {
                                let timeout_request = AimAndFireTransitionRequest {
                                    visit,
                                    reason: AimAndFireTransitionReason::LifetimeExpired,
                                    committed_prefix,
                                };
                                match adapter.aim_and_fire_transition(owner, timeout_request) {
                                    Ok(AimAndFireTransitionOutcome::Completed(Some(value))) => {
                                        ActorTaskVisitControl::Propagate(Ok(value))
                                    }
                                    Ok(
                                        AimAndFireTransitionOutcome::Completed(None)
                                        | AimAndFireTransitionOutcome::CallbackAbsent
                                        | AimAndFireTransitionOutcome::SuppressedByEntityState,
                                    ) => ActorTaskVisitControl::Continue,
                                    Err(error) => ActorTaskVisitControl::Propagate(Err(
                                        ActorTaskDispatcherError::AimAndFireTransition {
                                            request: timeout_request,
                                            error,
                                        },
                                    )),
                                }
                            }
                            Ok(
                                AimAndFireTransitionOutcome::CallbackAbsent
                                | AimAndFireTransitionOutcome::SuppressedByEntityState,
                            ) => ActorTaskVisitControl::Continue,
                            Err(error) => ActorTaskVisitControl::Propagate(Err(
                                ActorTaskDispatcherError::AimAndFireTransition { request, error },
                            )),
                        }
                    }
                }
            }
            (
                DispatchBefore::GuardLocationAcquisition,
                DispatchCallback::GuardLocationAcquisition(callback),
            ) => {
                assert_surviving_family(
                    owner,
                    visit,
                    ActorTaskRuntimeFamily::GuardLocationAcquisition,
                );
                let callback_result = match callback {
                    GuardLocationAcquisitionCallback::PrefixError(error) => {
                        return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::GuardLocationAcquisitionPrefix {
                                visit,
                                error,
                            },
                        ));
                    }
                    GuardLocationAcquisitionCallback::Evaluated {
                        committed_prefix,
                        result: Err(error),
                    } => {
                        return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::GuardLocationAcquisitionCallback {
                                visit,
                                committed_prefix,
                                error,
                            },
                        ));
                    }
                    GuardLocationAcquisitionCallback::Evaluated {
                        committed_prefix: _,
                        result: Ok(result),
                    } => result,
                };

                match callback_result {
                    GuardLocationAcquisitionCallbackResult::Zero(_)
                    | GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted { .. } => {
                        ActorTaskVisitControl::Continue
                    }
                    GuardLocationAcquisitionCallbackResult::PropagateBehaviorResult {
                        result,
                        ..
                    } => ActorTaskVisitControl::Propagate(Ok(
                        adapter.guard_location_acquisition_propagated_result(result)
                    )),
                }
            }
            (
                DispatchBefore::RunAway(committed_prefix),
                DispatchCallback::RunAway { stage, result },
            ) => {
                assert_surviving_family(owner, visit, ActorTaskRuntimeFamily::RunAway);
                let callback_result = match result {
                    Ok(result) => {
                        stage.commit(run_away_state_mut(owner, visit));
                        result
                    }
                    Err(error) => {
                        return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::RunAwayCallback {
                                visit,
                                committed_prefix,
                                error,
                            },
                        ));
                    }
                };
                let Some(request) =
                    run_away_transition_after_unwind(visit, committed_prefix, callback_result)
                else {
                    return ActorTaskVisitControl::Continue;
                };
                match adapter.run_away_transition(owner, request) {
                    Ok(Some(value)) => ActorTaskVisitControl::Propagate(Ok(value)),
                    Ok(None) => ActorTaskVisitControl::Continue,
                    Err(error) => ActorTaskVisitControl::Propagate(Err(
                        ActorTaskDispatcherError::RunAwayTransition { request, error },
                    )),
                }
            }
            (
                DispatchBefore::SharedRetarget(lifetime_prefix),
                DispatchCallback::SharedRetarget { retarget, result },
            ) => {
                assert_surviving_family(owner, visit, ActorTaskRuntimeFamily::SharedRetarget);
                let committed_prefix =
                    SharedRetargetCallbackPrefix::from_parts(lifetime_prefix, retarget);
                let mover_return = match result {
                    Ok(result) => result,
                    Err(error) => {
                        return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::SharedRetargetCommonMover {
                                slot: visit.slot,
                                committed_prefix,
                                error,
                            },
                        ));
                    }
                };
                let request = match shared_retarget_after_unwind(
                    visit,
                    committed_prefix,
                    mover_return,
                ) {
                    SharedRetargetPostUnwind::Continue => {
                        return ActorTaskVisitControl::Continue;
                    }
                    SharedRetargetPostUnwind::UnresolvedCommonMover => {
                        return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::SharedRetargetUnresolvedCommonMover {
                                slot: visit.slot,
                                committed_prefix,
                            },
                        ));
                    }
                    SharedRetargetPostUnwind::Transition(request) => request,
                };
                match adapter.shared_retarget_transition(owner, request) {
                    Ok(Some(value)) => ActorTaskVisitControl::Propagate(Ok(value)),
                    Ok(None) => ActorTaskVisitControl::Continue,
                    Err(error) => ActorTaskVisitControl::Propagate(Err(
                        ActorTaskDispatcherError::SharedRetargetTransition { request, error },
                    )),
                }
            }
            (
                DispatchBefore::DefecateVirusTerrain(committed_prefix),
                DispatchCallback::DefecateVirusTerrain(callback),
            ) => {
                assert_surviving_family(owner, visit, ActorTaskRuntimeFamily::DefecateVirusTerrain);
                match callback {
                    DefecateVirusTerrainCallback::RequestError(error) => {
                        return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::DefecateVirusTerrainRequest {
                                visit,
                                committed_prefix,
                                error,
                            },
                        ));
                    }
                    DefecateVirusTerrainCallback::ModelExtentUnavailable { model_slot_index } => {
                        return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::DefecateVirusTerrainModelExtentUnavailable {
                                visit,
                                committed_prefix,
                                model_slot_index,
                            },
                        ));
                    }
                    DefecateVirusTerrainCallback::Planned {
                        plan,
                        result: Err(error),
                    } => {
                        return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::DefecateVirusTerrainPlan {
                                visit,
                                committed_prefix,
                                plan,
                                error,
                            },
                        ));
                    }
                    DefecateVirusTerrainCallback::Planned { result: Ok(()), .. } => {}
                }

                let Some(request) = defecate_virus_terrain_after_unwind(visit, committed_prefix)
                else {
                    return ActorTaskVisitControl::Continue;
                };
                match adapter.defecate_virus_terrain_transition(owner, request) {
                    Ok(DefecateVirusTransitionOutcome::Completed(Some(value))) => {
                        ActorTaskVisitControl::Propagate(Ok(value))
                    }
                    Ok(
                        DefecateVirusTransitionOutcome::CallbackAbsent
                        | DefecateVirusTransitionOutcome::SuppressedByEntityState
                        | DefecateVirusTransitionOutcome::Completed(None),
                    ) => ActorTaskVisitControl::Continue,
                    Err(error) => ActorTaskVisitControl::Propagate(Err(
                        ActorTaskDispatcherError::DefecateVirusTerrainTransition { request, error },
                    )),
                }
            }
            (
                DispatchBefore::DefecateVirusWander(elapsed_ms),
                DispatchCallback::DefecateVirusWander { retarget, result },
            ) => {
                assert_surviving_family(owner, visit, ActorTaskRuntimeFamily::DefecateVirusWander);
                let committed_prefix = DefecateVirusWanderCallbackPrefix {
                    elapsed_ms,
                    retarget,
                };
                let mover_return = match result {
                    Ok(result) => result,
                    Err(error) => {
                        return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::DefecateVirusWanderCommonMover {
                                slot: visit.slot,
                                committed_prefix,
                                error,
                            },
                        ));
                    }
                };
                let request =
                    match defecate_virus_wander_after_unwind(visit, committed_prefix, mover_return)
                    {
                        DefecateVirusWanderPostUnwind::Continue => {
                            return ActorTaskVisitControl::Continue;
                        }
                        DefecateVirusWanderPostUnwind::UnresolvedCommonMover => {
                            return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::DefecateVirusWanderUnresolvedCommonMover {
                                slot: visit.slot,
                                committed_prefix,
                            },
                        ));
                        }
                        DefecateVirusWanderPostUnwind::Transition(request) => request,
                    };
                match adapter.defecate_virus_wander_transition(owner, request) {
                    Ok(DefecateVirusTransitionOutcome::Completed(Some(value))) => {
                        ActorTaskVisitControl::Propagate(Ok(value))
                    }
                    Ok(
                        DefecateVirusTransitionOutcome::CallbackAbsent
                        | DefecateVirusTransitionOutcome::SuppressedByEntityState
                        | DefecateVirusTransitionOutcome::Completed(None),
                    ) => ActorTaskVisitControl::Continue,
                    Err(error) => ActorTaskVisitControl::Propagate(Err(
                        ActorTaskDispatcherError::DefecateVirusWanderTransition { request, error },
                    )),
                }
            }
            (
                DispatchBefore::CommonDying(committed_prefix),
                DispatchCallback::CommonDying(result),
            ) => {
                assert_surviving_family(owner, visit, ActorTaskRuntimeFamily::CommonDying);
                let callback_result = match result {
                    Ok(result) => result,
                    Err(error) => {
                        return ActorTaskVisitControl::Propagate(Err(
                            ActorTaskDispatcherError::CommonDyingCallback {
                                visit,
                                committed_prefix,
                                error,
                            },
                        ));
                    }
                };
                let reason = match common_dying_after_unwind(committed_prefix, callback_result) {
                    CommonDyingAfterUnwindOutcome::Continue => {
                        return ActorTaskVisitControl::Continue;
                    }
                    CommonDyingAfterUnwindOutcome::RequestOwnerTransition { reason } => reason,
                };
                let request = CommonDyingTransitionRequest {
                    visit,
                    reason,
                    committed_prefix,
                };
                match adapter.common_dying_transition(owner, request) {
                    Ok(Some(value)) => ActorTaskVisitControl::Propagate(Ok(value)),
                    Ok(None) => ActorTaskVisitControl::Continue,
                    Err(error) => ActorTaskVisitControl::Propagate(Err(
                        ActorTaskDispatcherError::CommonDyingTransition { request, error },
                    )),
                }
            }
            _ => panic!("actor task dispatcher phase family mismatch"),
        },
    );

    match propagated {
        None => Ok(None),
        Some(Ok(value)) => Ok(Some(value)),
        Some(Err(error)) => Err(error),
    }
}

fn apply_follow_beacons_following_transition<Adapter: ActorTaskDispatcherAdapter>(
    adapter: &mut Adapter,
    owner: &mut ActorTaskOwner<ActorTaskRuntime>,
    request: FollowBeaconsFollowingTransitionRequest,
) -> ActorTaskVisitControl<Result<Adapter::Output, ActorTaskDispatcherError<Adapter::Error>>> {
    let outcome = match adapter.follow_beacons_following_transition(owner, request) {
        Ok(outcome) => outcome,
        Err(error) => {
            return ActorTaskVisitControl::Propagate(Err(
                ActorTaskDispatcherError::FollowBeaconsFollowingTransition { request, error },
            ));
        }
    };
    let falls_through_lifetime =
        follow_beacons_following_transition_falls_through_lifetime(request.reason, &outcome);
    match outcome {
        FollowBeaconsFollowingTransitionOutcome::Completed(Some(value)) => {
            ActorTaskVisitControl::Propagate(Ok(value))
        }
        FollowBeaconsFollowingTransitionOutcome::Completed(None) => ActorTaskVisitControl::Continue,
        FollowBeaconsFollowingTransitionOutcome::CallbackAbsent
        | FollowBeaconsFollowingTransitionOutcome::SuppressedByEntityState
            if falls_through_lifetime
                && request.committed_prefix.lifetime_status
                    == FollowBeaconsFollowingLifetimeStatus::OwnerTransitionDue =>
        {
            let timeout_request = FollowBeaconsFollowingTransitionRequest {
                slot: request.slot,
                task_id: request.task_id,
                reason: FollowBeaconsFollowingTransitionReason::LifetimeExpired,
                committed_prefix: request.committed_prefix,
            };
            match adapter.follow_beacons_following_transition(owner, timeout_request) {
                Ok(FollowBeaconsFollowingTransitionOutcome::Completed(Some(value))) => {
                    ActorTaskVisitControl::Propagate(Ok(value))
                }
                Ok(
                    FollowBeaconsFollowingTransitionOutcome::Completed(None)
                    | FollowBeaconsFollowingTransitionOutcome::CallbackAbsent
                    | FollowBeaconsFollowingTransitionOutcome::SuppressedByEntityState,
                ) => ActorTaskVisitControl::Continue,
                Err(error) => ActorTaskVisitControl::Propagate(Err(
                    ActorTaskDispatcherError::FollowBeaconsFollowingTransition {
                        request: timeout_request,
                        error,
                    },
                )),
            }
        }
        FollowBeaconsFollowingTransitionOutcome::CallbackAbsent
        | FollowBeaconsFollowingTransitionOutcome::SuppressedByEntityState => {
            ActorTaskVisitControl::Continue
        }
    }
}

fn follow_beacons_following_transition_falls_through_lifetime<T>(
    reason: FollowBeaconsFollowingTransitionReason,
    outcome: &FollowBeaconsFollowingTransitionOutcome<T>,
) -> bool {
    match (reason, outcome) {
        (
            FollowBeaconsFollowingTransitionReason::TaggedCallbackResult(_),
            FollowBeaconsFollowingTransitionOutcome::CallbackAbsent,
        ) => true,
        (
            FollowBeaconsFollowingTransitionReason::TaggedCallbackResult(
                FollowBeaconsFollowingTaggedSingleton::ReachedZeroCallback,
            ),
            FollowBeaconsFollowingTransitionOutcome::SuppressedByEntityState,
        ) => true,
        _ => false,
    }
}

fn ordinary_wander_state_mut(
    owner: &mut ActorTaskOwner<ActorTaskRuntime>,
    visit: ActorTaskVisit,
) -> &mut OrdinaryType9WanderTaskState {
    match owner.task_state_mut(visit.task_id) {
        Some(ActorTaskRuntime::OrdinaryType9Wander(state)) => state,
        _ => panic!("ordinary Wander visit changed family without wrapper replacement"),
    }
}

fn go_to_job_state(
    owner: &ActorTaskOwner<ActorTaskRuntime>,
    visit: ActorTaskVisit,
) -> GoToJobTaskState {
    match owner.task_state(visit.task_id) {
        Some(ActorTaskRuntime::GoToJob(state)) => *state,
        _ => panic!("Go To Job visit changed family without wrapper replacement"),
    }
}

fn go_to_job_state_mut(
    owner: &mut ActorTaskOwner<ActorTaskRuntime>,
    visit: ActorTaskVisit,
) -> &mut GoToJobTaskState {
    match owner.task_state_mut(visit.task_id) {
        Some(ActorTaskRuntime::GoToJob(state)) => state,
        _ => panic!("Go To Job visit changed family without wrapper replacement"),
    }
}

fn chase_target_state(
    owner: &ActorTaskOwner<ActorTaskRuntime>,
    visit: ActorTaskVisit,
) -> ChaseTargetTaskState {
    match owner.task_state(visit.task_id) {
        Some(ActorTaskRuntime::ChaseTarget(state)) => *state,
        _ => panic!("Chase Target visit changed family without wrapper replacement"),
    }
}

fn chase_target_state_mut(
    owner: &mut ActorTaskOwner<ActorTaskRuntime>,
    visit: ActorTaskVisit,
) -> &mut ChaseTargetTaskState {
    match owner.task_state_mut(visit.task_id) {
        Some(ActorTaskRuntime::ChaseTarget(state)) => state,
        _ => panic!("Chase Target visit changed family without wrapper replacement"),
    }
}

fn target_acquisition_state(
    owner: &ActorTaskOwner<ActorTaskRuntime>,
    visit: ActorTaskVisit,
) -> TargetAcquisitionCallbackPrefix {
    match owner.task_state(visit.task_id) {
        Some(ActorTaskRuntime::TargetAcquisition(state)) => TargetAcquisitionCallbackPrefix {
            radius: state.radius(),
            filter: state.filter(),
        },
        _ => panic!("target-acquisition visit changed family without replacement"),
    }
}

fn follow_beacon_acquisition_state(
    owner: &ActorTaskOwner<ActorTaskRuntime>,
    visit: ActorTaskVisit,
) -> FollowBeaconAcquisitionCallbackPrefix {
    match owner.task_state(visit.task_id) {
        Some(ActorTaskRuntime::FollowBeaconAcquisition(state)) => {
            FollowBeaconAcquisitionCallbackPrefix::new(state.route_range())
        }
        _ => panic!("Follow Beacons acquisition visit changed family without replacement"),
    }
}

fn follow_beacons_following_state(
    owner: &ActorTaskOwner<ActorTaskRuntime>,
    visit: ActorTaskVisit,
) -> FollowBeaconsFollowingTaskState {
    match owner.task_state(visit.task_id) {
        Some(ActorTaskRuntime::FollowBeaconsFollowing(state)) => *state,
        _ => panic!("Follow Beacons following visit changed family without replacement"),
    }
}

fn aim_and_fire_private_state(
    owner: &ActorTaskOwner<ActorTaskRuntime>,
    visit: ActorTaskVisit,
) -> AimAndFirePrivateState {
    match owner.task_state(visit.task_id) {
        Some(ActorTaskRuntime::AimAndFire(state)) => state.private_state(),
        _ => panic!("Aim-and-Fire visit changed family without wrapper replacement"),
    }
}

fn guard_location_acquisition_state(
    owner: &ActorTaskOwner<ActorTaskRuntime>,
    visit: ActorTaskVisit,
) -> GuardLocationAcquisitionTaskState {
    match owner.task_state(visit.task_id) {
        Some(ActorTaskRuntime::GuardLocationAcquisition(state)) => *state,
        _ => panic!("Guard Location acquisition visit changed family without replacement"),
    }
}

fn run_away_state(
    owner: &ActorTaskOwner<ActorTaskRuntime>,
    visit: ActorTaskVisit,
) -> RunAwayTaskState {
    match owner.task_state(visit.task_id) {
        Some(ActorTaskRuntime::RunAway(state)) => *state,
        _ => panic!("Run Away visit changed family without wrapper replacement"),
    }
}

fn run_away_state_mut(
    owner: &mut ActorTaskOwner<ActorTaskRuntime>,
    visit: ActorTaskVisit,
) -> &mut RunAwayTaskState {
    match owner.task_state_mut(visit.task_id) {
        Some(ActorTaskRuntime::RunAway(state)) => state,
        _ => panic!("Run Away visit changed family without wrapper replacement"),
    }
}

fn shared_retarget_state_mut(
    owner: &mut ActorTaskOwner<ActorTaskRuntime>,
    visit: ActorTaskVisit,
) -> &mut SharedRetargetTaskState {
    match owner.task_state_mut(visit.task_id) {
        Some(ActorTaskRuntime::SharedRetarget(state)) => state,
        _ => panic!("shared-retarget visit changed family without wrapper replacement"),
    }
}

fn defecate_virus_wander_state_mut(
    owner: &mut ActorTaskOwner<ActorTaskRuntime>,
    visit: ActorTaskVisit,
) -> &mut DefecateVirusWanderTaskState {
    match owner.task_state_mut(visit.task_id) {
        Some(ActorTaskRuntime::DefecateVirusWander(state)) => state,
        _ => panic!("Defecate Virus wander visit changed family without wrapper replacement"),
    }
}

fn assert_surviving_family(
    owner: &ActorTaskOwner<ActorTaskRuntime>,
    visit: ActorTaskVisit,
    expected: ActorTaskRuntimeFamily,
) {
    assert_eq!(
        owner
            .task_state(visit.task_id)
            .map(ActorTaskRuntime::family),
        Some(expected),
        "a surviving task cannot change family without wrapper replacement"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_task_owner::{ActorTaskId, PreparedActorTask};
    use crate::chase_target::{ChaseTargetTaggedSingleton, ChaseTargetTransitionReason};
    use crate::common_dying::{
        CommonDyingComponentDescriptors, CommonDyingConstructorEffect, CommonDyingTaggedResult,
        COMMON_DYING_INITIAL_VERTICAL_VELOCITY_RAW, COMMON_DYING_OWNER_TRANSITION_TAG,
        COMMON_DYING_SCHEDULER_SINGLETON_ADDRESS,
    };
    use crate::common_mover::SubAPropulsionRuntime;
    use crate::defecate_virus::{
        DefecateVirusTerrainLifetimeStatus, DefecateVirusWanderTransitionReason,
    };
    use crate::defecate_virus_owner::{
        apply_defecate_virus_setup, DefecateVirusSetupRequest, DefecateVirusSubATopology,
    };
    use crate::entity_collision_state::{
        CommonMoverComponentTopology, EntityInitializerSpec, EntityTypeRuntimeMetadata,
        RetailRuntimeValue, RetailStateWord,
    };
    use crate::follow_beacons::{
        apply_follow_beacons_acquiring_task_setup, FollowBeaconAcquisitionTaggedSingleton,
        FollowBeaconAcquisitionZeroReason, FollowBeaconTarget,
    };
    use crate::go_to_job::{
        plan_go_to_job_setup, GoToJobCandidate, GoToJobOwner, GoToJobSetupRequest,
        GO_TO_JOB_FAST_TARGET_CAPABILITY_BIT, GO_TO_JOB_FIRST_WORLD_SCIENTIST_TYPE,
    };
    use crate::go_to_job_owner::GoToJobTaggedSingleton;
    use crate::guard_location_owner::acquisition::{
        GuardLocationAcquisitionTaggedSingleton, GuardLocationCandidate, GuardLocationSearchContext,
    };
    use crate::ordinary_type9_wander_owner::plan_ordinary_type9_wander_setup;
    use crate::run_away::{apply_run_away_task_setup, run_away_variant_setup, RunAwayVariant};
    use crate::search_attack::{search_attack_variant_setup, SearchAttackVariant};
    use crate::session::GameSession;
    use crate::shared_retarget_mover::SharedRetargetTransitionReason;
    use crate::wrapped_axis_range::WrappedAxisRange;
    use std::cell::Cell;
    use std::collections::VecDeque;
    use std::convert::Infallible;
    use std::rc::Rc;
    use v2k_formats::collision::{CommonAxisDescriptor, SubAPropulsionDescriptor};

    fn wander_state() -> OrdinaryType9WanderTaskState {
        let mut owner = ActorTaskOwner::new();
        let mut sub_a =
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(0), 1, 100);
        plan_ordinary_type9_wander_setup([100, 200, 300], 0x0300)
            .apply(&mut owner, &mut sub_a, |specification| {
                Ok::<_, Infallible>(specification.prepare_after_allocation(|| 1))
            })
            .unwrap();
        *owner.state_in_slot(ActorTaskSlot::Primary).unwrap()
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

    fn type17_run_away_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                SubAPropulsionDescriptor {
                    acceleration_raw: 100,
                    overspeed_correction_raw: 200,
                    target_speed_base_raw: 250,
                },
            )),
            common_mover_topology: RetailRuntimeValue::Known(CommonMoverComponentTopology {
                sub_a: true,
                sub_b: true,
                sub_c: true,
                sub_d: true,
                sub_h: true,
                sub_j: true,
                ..CommonMoverComponentTopology::default()
            }),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: 0,
                common_axis_descriptor: CommonAxisDescriptor {
                    strict_axis_limit_raw: 2_560,
                    raw_word_at_0x04: 3,
                },
                behavior_choices: Box::new([]),
                behavior_rule_ref: 0,
                alternate_behavior_class_ref: 0,
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn type9_run_away_suffix_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                crate::main_base_type9_abort::LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR,
            )),
            common_mover_topology: RetailRuntimeValue::Known(
                crate::main_base_type9_abort::LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY,
            ),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw:
                    crate::main_base_type9_abort::LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW,
                common_axis_descriptor:
                    crate::main_base_type9_abort::LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR,
                behavior_choices: crate::main_base_type9_abort::LEVEL_ONE_TYPE9_BEHAVIOR_CHOICES
                    .into(),
                behavior_rule_ref: 1,
                alternate_behavior_class_ref: u32::from(
                    crate::main_base_type9_abort::LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS,
                ),
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn go_to_job_state() -> GoToJobTaskState {
        let owner_snapshot = GoToJobOwner::from_type_metadata(
            42,
            [0x0100, 0x0200, 0x0300],
            RetailRuntimeValue::Known(0x0800),
            GO_TO_JOB_FIRST_WORLD_SCIENTIST_TYPE,
            &type8_metadata(),
        );
        let candidate = GoToJobCandidate {
            id: 77,
            position_raw: [0x0400, 0x0500, 0x0600],
            state_flags: RetailStateWord::exact(1),
            capability_flags: RetailRuntimeValue::Known(GO_TO_JOB_FAST_TARGET_CAPABILITY_BIT),
            capacity: RetailRuntimeValue::Unresolved,
        };
        let plan = plan_go_to_job_setup(GoToJobSetupRequest {
            owner: owner_snapshot,
            candidates_in_intrusive_order: &[candidate],
            range: WrappedAxisRange::strict(0x1000).unwrap(),
        })
        .unwrap();
        let mut owner = ActorTaskOwner::new();
        let mut sub_a =
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(77), -1, 73);
        plan.apply(
            owner_snapshot.bind_runtime(&mut owner, &mut sub_a),
            || 0x1234_ABCD,
            |specification| {
                Ok::<_, Infallible>(PreparedActorTask::new(ActorTaskRuntime::GoToJob(
                    GoToJobTaskState::after_allocation(specification),
                )))
            },
        )
        .unwrap();
        match owner.state_in_slot(ActorTaskSlot::Primary).unwrap() {
            ActorTaskRuntime::GoToJob(state) => *state,
            _ => panic!("wrong task family"),
        }
    }

    fn chase_target_state(target_id: u32) -> ChaseTargetTaskState {
        let metadata = EntityTypeRuntimeMetadata {
            common_mover_topology: RetailRuntimeValue::Known(
                CommonMoverComponentTopology::default(),
            ),
            ..EntityTypeRuntimeMetadata::default()
        };
        let prepared = ChaseTargetTaskState::prepare_after_allocation(
            7,
            [100, 200, 300],
            target_id,
            &metadata,
        )
        .unwrap();
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            prepared
                .map_task(ActorTaskRuntime::ChaseTarget)
                .apply_suffix(|owner_id, suffix| {
                    assert_eq!(owner_id, 7);
                    assert_eq!(suffix.sub_a_target_speed_raw(), None);
                    assert!(!suffix.enables_sub_f());
                }),
        );
        match owner.state_in_slot(ActorTaskSlot::Primary).unwrap() {
            ActorTaskRuntime::ChaseTarget(state) => *state,
            _ => panic!("wrong task family"),
        }
    }

    fn target_acquisition_state(
        constructor_filter_override_raw: u32,
    ) -> TargetAcquisitionTaskState {
        TargetAcquisitionTaskState::new(
            SearchAttackRadius::strict(0x1000).unwrap(),
            SearchAttackCandidateFilter::CapabilityMask(std::num::NonZeroU32::new(0x20).unwrap()),
            constructor_filter_override_raw,
        )
    }

    fn follow_beacon_acquisition_task(
        type_authored_filter_raw: u32,
    ) -> PreparedActorTask<ActorTaskRuntime> {
        PreparedActorTask::new(ActorTaskRuntime::FollowBeaconAcquisition(
            FollowBeaconAcquisitionTaskState::new(
                WrappedAxisRange::strict(0x0A00).unwrap(),
                type_authored_filter_raw,
            ),
        ))
    }

    fn follow_beacons_following_task() -> PreparedActorTask<ActorTaskRuntime> {
        prepare_follow_beacons_following_runtime_task(
            FollowBeaconsFollowingTaskPreparation {
                target_id: 0x047F_0001,
                task: FOLLOW_BEACONS_FOLLOWING_TASK,
            },
            0x0497_0001,
            [100, 200, 300],
            &type17_run_away_metadata(),
        )
        .unwrap()
        .apply_suffix(|| 1, |_| {})
    }

    fn aim_and_fire_task() -> PreparedActorTask<ActorTaskRuntime> {
        let metadata = EntityTypeRuntimeMetadata {
            common_mover_topology: RetailRuntimeValue::Known(
                CommonMoverComponentTopology::default(),
            ),
            ..EntityTypeRuntimeMetadata::default()
        };
        AimAndFireTaskState::prepare_after_allocation(
            0x0497_0001,
            [100, 200, 300],
            0x047F_0001,
            70,
            750_000,
            &metadata,
        )
        .unwrap()
        .map_task(ActorTaskRuntime::AimAndFire)
        .apply_suffix(|owner_id, suffix| {
            assert_eq!(owner_id, 0x0497_0001);
            assert!(!suffix.enables_sub_f());
        })
    }

    fn guard_location_acquisition_task(
        constructor_filter_override_raw: u32,
    ) -> PreparedActorTask<ActorTaskRuntime> {
        PreparedActorTask::new(ActorTaskRuntime::GuardLocationAcquisition(
            GuardLocationAcquisitionTaskState::new(constructor_filter_override_raw),
        ))
    }

    fn run_away_task() -> PreparedActorTask<ActorTaskRuntime> {
        let metadata = EntityTypeRuntimeMetadata {
            common_mover_topology: RetailRuntimeValue::Known(
                CommonMoverComponentTopology::default(),
            ),
            ..EntityTypeRuntimeMetadata::default()
        };
        RunAwayTaskState::prepare_after_allocation(
            0x0497_0001,
            [100, 200, 300],
            0x047F_0001,
            crate::run_away::RunAwayAuthoredAudio {
                sound_id: 0,
                period_raw: 0,
            },
            &metadata,
        )
        .unwrap()
        .map_task(ActorTaskRuntime::RunAway)
        .apply_suffix(|owner_id, suffix| {
            assert_eq!(owner_id, 0x0497_0001);
            assert_eq!(suffix.sub_a_target_speed_raw(), None);
        })
    }

    fn shared_retarget_task() -> PreparedActorTask<ActorTaskRuntime> {
        PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
            SharedRetargetTaskState::new([100, 200, 300], RUN_AWAY_ACQUIRING_WANDER_LIFETIME_MS),
        ))
    }

    fn common_dying_task(owner_entity_id: u32) -> PreparedActorTask<ActorTaskRuntime> {
        let metadata = EntityTypeRuntimeMetadata {
            common_mover_topology: RetailRuntimeValue::Known(
                CommonMoverComponentTopology::default(),
            ),
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(None),
            ..EntityTypeRuntimeMetadata::default()
        };
        let prepared = CommonDyingTaskState::prepare_after_allocation(
            owner_entity_id,
            &metadata,
            CommonDyingComponentDescriptors::default(),
        )
        .unwrap()
        .map_task(ActorTaskRuntime::CommonDying);
        let mut constructor_effects = Vec::new();
        let task = prepared.apply_suffix(
            || panic!("component-free class-12 setup cannot consume RNG"),
            |effect| constructor_effects.push(effect),
        );
        assert_eq!(
            constructor_effects,
            [CommonDyingConstructorEffect::WriteOwnerVerticalVelocity {
                velocity_raw: COMMON_DYING_INITIAL_VERTICAL_VELOCITY_RAW
            }]
        );
        task
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Mode {
        AttractCandidateRetiredError,
        AttractCandidatePropagate,
        AttractCandidateSurvivingTag,
        RunAwayToAttractEven,
        RunAwayToAttractOddMiss,
        RunAwayToAttractOddAccepted,
        AttractCueSuppressed,
        AttractCueCompleted,
        ReplaceSecondary,
        ClearGoDuringCallback,
        ChaseContinue,
        ChaseError,
        ClearChaseDuringCallback,
        ReplaceSecondaryFromChase,
        TargetAcquisitionContinue,
        TargetAcquisitionTagged,
        TargetAcquisitionError,
        TargetAcquisitionPropagate,
        ReplaceTargetAcquisitionWithTertiary,
        FollowBeaconContinue,
        FollowBeaconTagged,
        FollowBeaconError,
        FollowBeaconPropagate,
        ReplaceFollowBeaconTagged,
        ReplaceFollowBeaconError,
        ReplaceFollowBeaconPropagate,
        FollowFollowingContinue,
        FollowFollowingMoverError,
        FollowFollowingPostMoverError,
        FollowFollowingTagged,
        FollowFollowingPropagate,
        FollowFollowingRouteCallbackAbsent,
        FollowFollowingRouteSuppressed,
        FollowFollowingReachedSuppressed,
        FollowFollowingStyleFallThrough,
        ReplaceFollowFollowingTagged,
        ReplaceFollowFollowingPostMoverError,
        ReplaceFollowFollowingContinue,
        ReplaceSecondaryFromFollowFollowing,
        AimTagged,
        AimTagCallbackAbsent,
        AimTagCallbackSuppressed,
        AimError,
        AimPropagate,
        ClearAimAndReplaceSecondary,
        GuardChanceRejected,
        GuardPrefixError,
        GuardCallbackError,
        GuardTagged,
        GuardPropagate,
        ReplaceGuardWithTertiary,
        RunAwayError,
        ClearRunAwayDuringCallback,
        ReplaceSecondaryFromRunAway,
        SharedRetargetContinue,
        SharedRetargetZero,
        SharedRetargetUnresolved,
        ReplaceSharedRetargetWithTertiaryError,
        ReplaceSharedRetargetWithTertiaryZero,
        UnresolvedWander,
        DefecateMixedRng,
        DefecateElapsedMismatch,
        ReplaceSecondaryFromDefecate,
        ClearDefecateTerrainDuringCallback,
        UnresolvedDefecateWander,
        DefecateTagCallbackAbsent,
        DefecateTagCallbackSuppressed,
        DefecateTerrainTimeout,
        CommonDyingContinue,
        CommonDyingError,
        ClearCommonDyingDuringCallback,
        ReplaceSecondaryFromCommonDying,
    }

    struct TestAdapter {
        mode: Mode,
        replacement: OrdinaryType9WanderTaskState,
        old_secondary: Option<ActorTaskId>,
        visited_wander: Vec<ActorTaskId>,
        events: Vec<&'static str>,
        random: VecDeque<u32>,
        random_draws: usize,
        attract_candidate_context: GuardLocationSearchContext,
        attract_candidate_prefixes: Vec<GuardLocationAcquisitionCallbackPrefix>,
        attract_cue_transitions: Vec<OrdinaryType9AttractAttentionCueTransition>,
        go_transitions: usize,
        chase_transitions: Vec<ChaseTargetTransitionReason>,
        target_acquisition_prefixes: Vec<TargetAcquisitionCallbackPrefix>,
        follow_beacon_prefixes: Vec<FollowBeaconAcquisitionCallbackPrefix>,
        follow_following_transitions: Vec<FollowBeaconsFollowingTransitionRequest>,
        follow_following_style_results: Vec<FollowBeaconsFollowingStyleResultRequest>,
        aim_transitions: Vec<AimAndFireTransitionRequest>,
        guard_location_context: GuardLocationSearchContext,
        guard_location_prefixes: Vec<GuardLocationAcquisitionCallbackPrefix>,
        run_away_transitions: Vec<RunAwayTransitionRequest>,
        shared_retarget_transitions: Vec<SharedRetargetTransitionRequest>,
        defecate_wander_visits: Vec<ActorTaskId>,
        defecate_terrain_visits: Vec<ActorTaskId>,
        defecate_terrain_plans: Vec<DefecateVirusCallbackPlan>,
        defecate_wander_transitions: Vec<DefecateVirusWanderTransitionReason>,
        defecate_terrain_transitions: Vec<DefecateVirusTerrainCallbackPrefix>,
        common_dying_visits: Vec<ActorTaskVisit>,
        common_dying_frames: Vec<ActorTaskDispatcherFrame>,
        common_dying_transitions: Vec<CommonDyingTransitionRequest>,
    }

    impl TestAdapter {
        fn new(mode: Mode) -> Self {
            Self {
                mode,
                replacement: wander_state(),
                old_secondary: None,
                visited_wander: Vec::new(),
                events: Vec::new(),
                random: VecDeque::from([1]),
                random_draws: 0,
                attract_candidate_context: GuardLocationSearchContext::new(
                    WrappedAxisRange::Unbounded,
                    SearchAttackCandidateFilter::SameEntityType,
                ),
                attract_candidate_prefixes: Vec::new(),
                attract_cue_transitions: Vec::new(),
                go_transitions: 0,
                chase_transitions: Vec::new(),
                target_acquisition_prefixes: Vec::new(),
                follow_beacon_prefixes: Vec::new(),
                follow_following_transitions: Vec::new(),
                follow_following_style_results: Vec::new(),
                aim_transitions: Vec::new(),
                guard_location_context: GuardLocationSearchContext::new(
                    WrappedAxisRange::strict(0x1000).unwrap(),
                    SearchAttackCandidateFilter::SameEntityType,
                ),
                guard_location_prefixes: Vec::new(),
                run_away_transitions: Vec::new(),
                shared_retarget_transitions: Vec::new(),
                defecate_wander_visits: Vec::new(),
                defecate_terrain_visits: Vec::new(),
                defecate_terrain_plans: Vec::new(),
                defecate_wander_transitions: Vec::new(),
                defecate_terrain_transitions: Vec::new(),
                common_dying_visits: Vec::new(),
                common_dying_frames: Vec::new(),
                common_dying_transitions: Vec::new(),
            }
        }
    }

    impl ActorTaskDispatcherAdapter for TestAdapter {
        type Output = &'static str;
        type Error = &'static str;

        fn ordinary_wander_anchor_raw(&mut self, _visit: ActorTaskVisit) -> [i16; 3] {
            [100, 200, 300]
        }

        fn next_random(&mut self) -> u32 {
            self.random_draws += 1;
            self.random.pop_front().unwrap_or(1)
        }

        fn attract_attention_candidate_prefix(
            &mut self,
            _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            visit: ActorTaskVisit,
            task_state: AttractAttentionCandidateTaskState,
            random: u32,
        ) -> Result<GuardLocationAcquisitionCallbackPrefix, Self::Error> {
            self.events.push("attract candidate prefix");
            assert_eq!(visit.slot, ActorTaskSlot::Secondary);
            let prefix = task_state
                .shared_acquisition()
                .before_callback(&mut self.attract_candidate_context, random);
            self.attract_candidate_prefixes.push(prefix);
            Ok(prefix)
        }

        fn attract_attention_candidate_callback(
            &mut self,
            owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            visit: ActorTaskVisit,
            prefix: GuardLocationAcquisitionCallbackPrefix,
        ) -> Result<GuardLocationAcquisitionCallbackResult, Self::Error> {
            self.events.push("attract candidate callback");
            assert!(owner.wrapper_flags(visit.task_id).unwrap().in_callback);
            if self.mode == Mode::RunAwayToAttractOddAccepted {
                let candidate = GuardLocationCandidate { id: 0x04FC_0001 };
                owner.clear_slot(visit.slot);
                owner.clear_slot(ActorTaskSlot::Tertiary);
                owner.replace_prepared(
                    ActorTaskSlot::Primary,
                    PreparedActorTask::new(ActorTaskRuntime::AttractAttentionTargetRoute(
                        SharedTargetRouteTaskState::after_allocation(
                            [0x0100, -0x0200, 0x0300],
                            candidate.id,
                        ),
                    )),
                );
                return Ok(
                    GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted {
                        candidate,
                        singleton: GuardLocationAcquisitionTaggedSingleton::CandidateAccepted,
                    },
                );
            }
            if self.mode == Mode::AttractCandidateRetiredError {
                owner.clear_slot(visit.slot);
                return Err("discarded Attract candidate callback error");
            }
            if self.mode == Mode::AttractCandidatePropagate {
                return Ok(
                    GuardLocationAcquisitionCallbackResult::PropagateBehaviorResult {
                        candidate: GuardLocationCandidate { id: 0x04FC_0001 },
                        result: std::num::NonZeroU32::new(0x00FE_DCBA).unwrap(),
                    },
                );
            }
            if self.mode == Mode::AttractCandidateSurvivingTag {
                return Ok(
                    GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted {
                        candidate: GuardLocationCandidate { id: 0x04FC_0001 },
                        singleton: GuardLocationAcquisitionTaggedSingleton::CandidateAccepted,
                    },
                );
            }
            let GuardLocationAcquisitionCallbackPrefix::ChanceRejected { random_low16 } = prefix
            else {
                panic!("chance-rejection test requires a rejected candidate prefix")
            };
            Ok(GuardLocationAcquisitionCallbackResult::Zero(
                crate::guard_location_owner::acquisition::GuardLocationAcquisitionZeroReason::ChanceRejected {
                    random_low16,
                },
            ))
        }

        fn attract_attention_candidate_propagated_result(
            &mut self,
            result: std::num::NonZeroU32,
        ) -> Self::Output {
            assert_eq!(result.get(), 0x00FE_DCBA);
            "Attract candidate propagated"
        }

        fn attract_attention_cue_transition_owner_id(
            &mut self,
            _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            visit: ActorTaskVisit,
            _committed_prefix: OrdinaryType9AttractAttentionCueCallbackPrefix,
        ) -> Result<u32, Self::Error> {
            assert_eq!(visit.slot, ActorTaskSlot::Tertiary);
            Ok(0x04A9_0001)
        }

        fn attract_attention_cue_transition(
            &mut self,
            owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            request: OrdinaryType9AttractAttentionCueTransition,
        ) -> Result<AttractAttentionCueTransitionOutcome<Self::Output>, Self::Error> {
            assert_eq!(
                owner.wrapper_flags(request.expired_visit.task_id),
                Some(crate::actor_task_owner::ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: false,
                }),
                "cue transition runs only after the exact wrapper unwinds"
            );
            self.events.push("attract cue transition");
            self.attract_cue_transitions.push(request);
            Ok(if self.mode == Mode::AttractCueSuppressed {
                AttractAttentionCueTransitionOutcome::SuppressedByEntityState
            } else {
                AttractAttentionCueTransitionOutcome::Completed(None)
            })
        }

        fn ordinary_wander_common_mover(
            &mut self,
            _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            visit: ActorTaskVisit,
            private_state: &mut WanderNearPrivateState,
        ) -> Result<WanderNearCommonMoverReturn, Self::Error> {
            self.events.push("wander callback");
            self.visited_wander.push(visit.task_id);
            if self.mode == Mode::UnresolvedWander {
                private_state.target_position_raw = [9, 8, 7];
                Ok(WanderNearCommonMoverReturn::Unresolved)
            } else {
                Ok(WanderNearCommonMoverReturn::NonZero)
            }
        }

        fn ordinary_wander_transition(
            &mut self,
            _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            _request: OrdinaryType9WanderTransitionRequest,
        ) -> Result<OrdinaryType9WanderTransitionOutcome<Self::Output>, Self::Error> {
            self.events.push("wander transition");
            Ok(OrdinaryType9WanderTransitionOutcome::Completed(None))
        }

        fn go_to_job_callback(
            &mut self,
            owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            visit: ActorTaskVisit,
            stage: &mut GoToJobCallbackStage,
        ) -> Result<GoToJobCallbackResult, Self::Error> {
            self.events.push("go callback");
            stage.private_state_mut().target_position_raw = [9, 8, 7];
            if self.mode == Mode::ClearGoDuringCallback {
                owner.clear_slot(visit.slot);
                return Err("discarded callback error");
            }
            Ok(GoToJobCallbackResult::Tagged(
                GoToJobTaggedSingleton::ZeroPredicate,
            ))
        }

        fn go_to_job_transition(
            &mut self,
            owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            _request: GoToJobTransitionRequest,
        ) -> Result<Option<Self::Output>, Self::Error> {
            self.events.push("go transition");
            self.go_transitions += 1;
            if self.mode == Mode::ReplaceSecondary {
                owner.replace_prepared(
                    ActorTaskSlot::Secondary,
                    PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(self.replacement)),
                );
            }
            Ok(None)
        }

        fn chase_target_callback(
            &mut self,
            owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            visit: ActorTaskVisit,
            frame: ActorTaskDispatcherFrame,
            stage: &mut ChaseTargetCallbackStage,
        ) -> Result<ChaseTargetCallbackResult, Self::Error> {
            self.events.push("chase callback");
            assert_eq!(frame.elapsed_micros, 1_000);
            assert_eq!(frame.scheduler_mode, 0xA5);
            stage.private_state_mut().target_position_raw = [9, 8, 7];
            match self.mode {
                Mode::ClearChaseDuringCallback => {
                    owner.clear_slot(visit.slot);
                    Err("discarded Chase callback error")
                }
                Mode::ChaseError => Err("unresolved Chase callback"),
                Mode::ChaseContinue => Ok(ChaseTargetCallbackResult::Continue {
                    controller_write: None,
                }),
                _ => Ok(ChaseTargetCallbackResult::Tagged(
                    ChaseTargetTaggedSingleton::OutOfRange,
                )),
            }
        }

        fn chase_target_transition(
            &mut self,
            owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            request: ChaseTargetTransitionRequest,
        ) -> Result<Option<Self::Output>, Self::Error> {
            self.events.push("chase transition");
            self.chase_transitions.push(request.reason);
            if self.mode == Mode::ReplaceSecondaryFromChase {
                owner.replace_prepared(
                    ActorTaskSlot::Secondary,
                    PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(self.replacement)),
                );
            }
            Ok(None)
        }

        fn target_acquisition_callback(
            &mut self,
            owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            visit: ActorTaskVisit,
            prefix: TargetAcquisitionCallbackPrefix,
        ) -> Result<TargetAcquisitionCallbackResult, Self::Error> {
            self.events.push("target acquisition callback");
            self.target_acquisition_prefixes.push(prefix);
            let target = crate::search_attack::SearchAttackTarget {
                id: 0x047F_0001,
                scaled_distance_squared_raw: 7,
            };
            match self.mode {
                Mode::TargetAcquisitionError => Err("unresolved target acquisition"),
                Mode::TargetAcquisitionTagged => {
                    Ok(TargetAcquisitionCallbackResult::TaggedTargetAccepted {
                        target,
                        singleton: crate::search_attack_acquisition::TargetAcquisitionTaggedSingleton::TargetAccepted,
                    })
                }
                Mode::TargetAcquisitionPropagate => Ok(
                    TargetAcquisitionCallbackResult::PropagateBehaviorResult {
                        target,
                        result: std::num::NonZeroU32::new(0x004B_E1C0).unwrap(),
                    },
                ),
                Mode::ReplaceTargetAcquisitionWithTertiary => {
                    owner.clear_slot(visit.slot);
                    owner.replace_prepared(
                        ActorTaskSlot::Tertiary,
                        PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(
                            self.replacement,
                        )),
                    );
                    Err("discarded target acquisition error")
                }
                _ => Ok(TargetAcquisitionCallbackResult::Zero(
                    crate::search_attack_acquisition::TargetAcquisitionZeroReason::BehaviorHandoffAbsent {
                        target,
                    },
                )),
            }
        }

        fn target_acquisition_propagated_result(
            &mut self,
            result: std::num::NonZeroU32,
        ) -> Self::Output {
            assert_eq!(result.get(), 0x004B_E1C0);
            "target acquisition propagated"
        }

        fn follow_beacon_acquisition_callback(
            &mut self,
            owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            visit: ActorTaskVisit,
            prefix: FollowBeaconAcquisitionCallbackPrefix,
        ) -> Result<FollowBeaconAcquisitionCallbackResult, Self::Error> {
            self.events.push("follow beacon callback");
            self.follow_beacon_prefixes.push(prefix);
            assert_eq!(visit.slot, ActorTaskSlot::Secondary);
            assert!(owner.wrapper_flags(visit.task_id).unwrap().in_callback);
            let target = FollowBeaconTarget {
                id: 0x047F_0001,
                score_raw: 70,
            };

            if matches!(
                self.mode,
                Mode::ReplaceFollowBeaconTagged
                    | Mode::ReplaceFollowBeaconError
                    | Mode::ReplaceFollowBeaconPropagate
            ) {
                // Model synchronous `FUN_0040C7D0 -> FUN_0040C6B0`: variant
                // one clears the current acquisition and later tertiary slot,
                // then publishes its new primary task. Primary has already
                // been visited in this 0 -> 1 -> 2 pass.
                owner.clear_slot(visit.slot);
                owner.clear_slot(ActorTaskSlot::Tertiary);
                owner.replace_prepared(
                    ActorTaskSlot::Primary,
                    PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(self.replacement)),
                );
            }

            match self.mode {
                Mode::FollowBeaconError | Mode::ReplaceFollowBeaconError => {
                    Err("unresolved Follow Beacons snapshot")
                }
                Mode::FollowBeaconTagged | Mode::ReplaceFollowBeaconTagged => Ok(
                    FollowBeaconAcquisitionCallbackResult::TaggedTargetAccepted {
                        target,
                        singleton: FollowBeaconAcquisitionTaggedSingleton::TargetAccepted,
                    },
                ),
                Mode::FollowBeaconPropagate | Mode::ReplaceFollowBeaconPropagate => Ok(
                    FollowBeaconAcquisitionCallbackResult::PropagateStyleResult {
                        target,
                        result: std::num::NonZeroU32::new(0x00FE_DCBA).unwrap(),
                    },
                ),
                _ => Ok(FollowBeaconAcquisitionCallbackResult::Zero(
                    FollowBeaconAcquisitionZeroReason::StyleHandoffAbsent { target },
                )),
            }
        }

        fn follow_beacon_acquisition_propagated_result(
            &mut self,
            result: std::num::NonZeroU32,
        ) -> Self::Output {
            assert_eq!(result.get(), 0x00FE_DCBA);
            "Follow Beacons propagated"
        }

        fn follow_beacons_following_callback(
            &mut self,
            owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            visit: ActorTaskVisit,
            _frame: ActorTaskDispatcherFrame,
            stage: &mut FollowBeaconsFollowingCallbackStage,
        ) -> Result<
            FollowBeaconsFollowingCallbackResult,
            FollowBeaconsFollowingAdapterError<Self::Error>,
        > {
            self.events.push("Follow Beacons following callback");
            stage.private_state_mut().direction = -1;

            if matches!(
                self.mode,
                Mode::ReplaceFollowFollowingTagged
                    | Mode::ReplaceFollowFollowingPostMoverError
                    | Mode::ReplaceFollowFollowingContinue
            ) {
                owner.replace_prepared(
                    visit.slot,
                    PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(self.replacement)),
                );
            }

            match self.mode {
                Mode::FollowFollowingMoverError => {
                    Err(FollowBeaconsFollowingAdapterError::CommonMover(
                        "unresolved Follow Beacons common mover",
                    ))
                }
                Mode::FollowFollowingPostMoverError
                | Mode::ReplaceFollowFollowingPostMoverError => {
                    Err(FollowBeaconsFollowingAdapterError::AfterCommonMover(
                        "unresolved Follow Beacons post-mover lookup",
                    ))
                }
                Mode::FollowFollowingTagged
                | Mode::FollowFollowingRouteCallbackAbsent
                | Mode::FollowFollowingRouteSuppressed
                | Mode::ReplaceFollowFollowingTagged
                | Mode::ReplaceSecondaryFromFollowFollowing => {
                    Ok(FollowBeaconsFollowingCallbackResult::Tagged(
                        FollowBeaconsFollowingTaggedSingleton::RouteRejected,
                    ))
                }
                Mode::FollowFollowingReachedSuppressed => {
                    Ok(FollowBeaconsFollowingCallbackResult::Tagged(
                        FollowBeaconsFollowingTaggedSingleton::ReachedZeroCallback,
                    ))
                }
                Mode::FollowFollowingPropagate | Mode::FollowFollowingStyleFallThrough => {
                    Ok(FollowBeaconsFollowingCallbackResult::PropagateStyleResult {
                        result: std::num::NonZeroU32::new(0x00FE_DCBA).unwrap(),
                    })
                }
                _ => Ok(FollowBeaconsFollowingCallbackResult::Continue),
            }
        }

        fn follow_beacons_following_transition(
            &mut self,
            owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            request: FollowBeaconsFollowingTransitionRequest,
        ) -> Result<FollowBeaconsFollowingTransitionOutcome<Self::Output>, Self::Error> {
            self.events.push("Follow Beacons following transition");
            self.follow_following_transitions.push(request);
            if self.mode == Mode::ReplaceSecondaryFromFollowFollowing {
                owner.replace_prepared(
                    ActorTaskSlot::Secondary,
                    PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(self.replacement)),
                );
            }
            Ok(match self.mode {
                Mode::FollowFollowingRouteCallbackAbsent => {
                    FollowBeaconsFollowingTransitionOutcome::CallbackAbsent
                }
                Mode::FollowFollowingRouteSuppressed | Mode::FollowFollowingReachedSuppressed => {
                    FollowBeaconsFollowingTransitionOutcome::SuppressedByEntityState
                }
                _ => FollowBeaconsFollowingTransitionOutcome::Completed(None),
            })
        }

        fn follow_beacons_following_style_result(
            &mut self,
            _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            request: FollowBeaconsFollowingStyleResultRequest,
        ) -> Result<FollowBeaconsFollowingStyleResultOutcome<Self::Output>, Self::Error> {
            self.events.push("Follow Beacons following style result");
            self.follow_following_style_results.push(request);
            Ok(if self.mode == Mode::FollowFollowingStyleFallThrough {
                FollowBeaconsFollowingStyleResultOutcome::FallThroughLifetime
            } else {
                FollowBeaconsFollowingStyleResultOutcome::Propagate(
                    "Follow Beacons following propagated",
                )
            })
        }

        fn aim_and_fire_callback(
            &mut self,
            owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            visit: ActorTaskVisit,
            frame: ActorTaskDispatcherFrame,
            private_state: AimAndFirePrivateState,
        ) -> Result<AimAndFireCallbackResult, Self::Error> {
            self.events.push("aim callback");
            assert_eq!(frame.scheduler_mode, 0);
            assert_eq!(private_state.target_entity_id(), 0x047F_0001);
            match self.mode {
                Mode::AimTagged | Mode::AimTagCallbackAbsent | Mode::AimTagCallbackSuppressed => {
                    Ok(AimAndFireCallbackResult::TaggedInvalidTarget {
                        singleton: crate::aim_and_fire::AimAndFireTaggedSingleton::InvalidTarget,
                        reason: crate::aim_and_fire::AimAndFireInvalidTargetReason::Dying,
                    })
                }
                Mode::AimError => Err("Aim-and-Fire emitter error"),
                Mode::AimPropagate => Ok(AimAndFireCallbackResult::ReturnGenericEmitterResult(
                    std::num::NonZeroU32::new(0x00AB_CDEF).unwrap(),
                )),
                Mode::ClearAimAndReplaceSecondary => {
                    crate::aim_and_fire::evaluate_aim_and_fire_callback(
                        private_state,
                        crate::aim_and_fire::AimAndFireFrameRequest {
                            owner_entity_id: 0x0497_0001,
                            elapsed_micros: frame.elapsed_micros,
                            scheduler_mode: frame.scheduler_mode,
                            target_state:
                                crate::aim_and_fire::AimAndFireTargetRuntimeState::Present {
                                    state_flags: 1,
                                },
                            owner_sound_position_raw: None,
                            sub_e_descriptor: Some(()),
                            emitter_runtime: (),
                        },
                        || u32::MAX,
                        |_| panic!("rejected optional sound cannot be submitted"),
                        |_| {
                            owner.clear_slot(visit.slot);
                            owner.replace_prepared(
                                ActorTaskSlot::Secondary,
                                PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(
                                    self.replacement,
                                )),
                            );
                            Err("discarded Aim-and-Fire emitter error")
                        },
                    )
                    .map_err(|error| match error {
                        crate::aim_and_fire::AimAndFireFrameError::GenericEmitter(error) => error,
                    })
                }
                _ => Ok(AimAndFireCallbackResult::Zero),
            }
        }

        fn aim_and_fire_transition(
            &mut self,
            _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            request: AimAndFireTransitionRequest,
        ) -> Result<AimAndFireTransitionOutcome<Self::Output>, Self::Error> {
            self.events.push("aim transition");
            self.aim_transitions.push(request);
            Ok(match (self.mode, request.reason) {
                (
                    Mode::AimTagCallbackAbsent,
                    AimAndFireTransitionReason::TaggedInvalidTarget { .. },
                ) => AimAndFireTransitionOutcome::CallbackAbsent,
                (
                    Mode::AimTagCallbackSuppressed,
                    AimAndFireTransitionReason::TaggedInvalidTarget { .. },
                ) => AimAndFireTransitionOutcome::SuppressedByEntityState,
                _ => AimAndFireTransitionOutcome::Completed(None),
            })
        }

        fn aim_and_fire_propagated_result(&mut self, result: std::num::NonZeroU32) -> Self::Output {
            assert_eq!(result.get(), 0x00AB_CDEF);
            "Aim-and-Fire propagated"
        }

        fn guard_location_acquisition_prefix(
            &mut self,
            visit: ActorTaskVisit,
            task_state: GuardLocationAcquisitionTaskState,
            random: u32,
        ) -> Result<GuardLocationAcquisitionCallbackPrefix, Self::Error> {
            self.events.push("guard acquisition prefix");
            assert_eq!(visit.slot, ActorTaskSlot::Secondary);
            if self.mode == Mode::GuardPrefixError {
                return Err("unresolved Guard context");
            }
            let prefix = task_state.before_callback(&mut self.guard_location_context, random);
            self.guard_location_prefixes.push(prefix);
            Ok(prefix)
        }

        fn guard_location_acquisition_callback(
            &mut self,
            owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            visit: ActorTaskVisit,
            prefix: GuardLocationAcquisitionCallbackPrefix,
        ) -> Result<GuardLocationAcquisitionCallbackResult, Self::Error> {
            self.events.push("guard acquisition callback");
            assert_eq!(
                owner.wrapper_flags(visit.task_id).unwrap().in_callback,
                true
            );
            let candidate = GuardLocationCandidate { id: 0x04FC_0001 };
            match self.mode {
                Mode::GuardCallbackError => Err("unresolved Guard candidate snapshot"),
                Mode::GuardTagged => Ok(
                    GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted {
                        candidate,
                        singleton: GuardLocationAcquisitionTaggedSingleton::CandidateAccepted,
                    },
                ),
                Mode::GuardPropagate => {
                    Ok(
                        GuardLocationAcquisitionCallbackResult::PropagateBehaviorResult {
                            candidate,
                            result: std::num::NonZeroU32::new(0x004B_E1C0).unwrap(),
                        },
                    )
                }
                Mode::ReplaceGuardWithTertiary => {
                    owner.clear_slot(visit.slot);
                    owner.replace_prepared(
                        ActorTaskSlot::Tertiary,
                        PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(
                            self.replacement,
                        )),
                    );
                    Err("discarded Guard acquisition error")
                }
                Mode::GuardChanceRejected => {
                    let GuardLocationAcquisitionCallbackPrefix::ChanceRejected {
                        random_low16,
                    } = prefix
                    else {
                        panic!("chance-rejection mode received an accepted prefix")
                    };
                    Ok(GuardLocationAcquisitionCallbackResult::Zero(
                        crate::guard_location_owner::acquisition::GuardLocationAcquisitionZeroReason::ChanceRejected {
                            random_low16,
                        },
                    ))
                }
                _ => Ok(GuardLocationAcquisitionCallbackResult::Zero(
                    crate::guard_location_owner::acquisition::GuardLocationAcquisitionZeroReason::BehaviorHandoffAbsent {
                        candidate,
                    },
                )),
            }
        }

        fn guard_location_acquisition_propagated_result(
            &mut self,
            result: std::num::NonZeroU32,
        ) -> Self::Output {
            assert_eq!(result.get(), 0x004B_E1C0);
            "guard acquisition propagated"
        }

        fn run_away_callback(
            &mut self,
            owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            visit: ActorTaskVisit,
            frame: ActorTaskDispatcherFrame,
            stage: &mut RunAwayCallbackStage,
        ) -> Result<RunAwayCallbackResult, Self::Error> {
            self.events.push("run away callback");
            assert_eq!(frame.elapsed_micros, 1_000);
            assert_eq!(frame.scheduler_mode, 0xA5);
            stage.private_state_mut().target_position_raw = [9, 8, 7];
            match self.mode {
                Mode::ClearRunAwayDuringCallback => {
                    owner.clear_slot(visit.slot);
                    Err("discarded Run Away callback error")
                }
                Mode::RunAwayError => Err("unresolved Run Away callback"),
                _ => Ok(RunAwayCallbackResult::Tagged(
                    crate::run_away::RunAwayTaggedSingleton::DirectTarget,
                )),
            }
        }

        fn run_away_transition(
            &mut self,
            owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            request: RunAwayTransitionRequest,
        ) -> Result<Option<Self::Output>, Self::Error> {
            self.events.push("run away transition");
            self.run_away_transitions.push(request);
            if matches!(
                self.mode,
                Mode::RunAwayToAttractEven
                    | Mode::RunAwayToAttractOddMiss
                    | Mode::RunAwayToAttractOddAccepted
            ) {
                owner.replace_prepared(
                    ActorTaskSlot::Primary,
                    PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                        SharedRetargetTaskState::new([111, 22, -333], 1_000),
                    )),
                );
                if matches!(
                    self.mode,
                    Mode::RunAwayToAttractOddMiss | Mode::RunAwayToAttractOddAccepted
                ) {
                    owner.replace_prepared(
                        ActorTaskSlot::Secondary,
                        PreparedActorTask::new(ActorTaskRuntime::AttractAttentionCandidate(
                            crate::attract_attention::AttractAttentionCandidateTaskState::new(
                                0x201,
                            ),
                        )),
                    );
                } else {
                    owner.clear_slot(ActorTaskSlot::Secondary);
                }
                owner.replace_prepared(
                    ActorTaskSlot::Tertiary,
                    PreparedActorTask::new(ActorTaskRuntime::AttractAttentionCue(
                        crate::attract_attention::AttractAttentionCueTaskState::new(1_000),
                    )),
                );
                return Ok(None);
            }
            if self.mode == Mode::ReplaceSecondaryFromRunAway {
                owner.replace_prepared(
                    ActorTaskSlot::Secondary,
                    PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(self.replacement)),
                );
            }
            Ok(None)
        }

        fn shared_retarget_actor_position_raw(&mut self, _visit: ActorTaskVisit) -> [i16; 3] {
            [100, 200, 300]
        }

        fn shared_retarget_common_mover(
            &mut self,
            owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            visit: ActorTaskVisit,
            private_state: &mut WanderNearPrivateState,
        ) -> Result<WanderNearCommonMoverReturn, Self::Error> {
            self.events.push("shared retarget callback");
            private_state.direction = -1;
            match self.mode {
                Mode::SharedRetargetZero => Ok(WanderNearCommonMoverReturn::Zero),
                Mode::SharedRetargetUnresolved => Ok(WanderNearCommonMoverReturn::Unresolved),
                Mode::ReplaceSharedRetargetWithTertiaryError
                | Mode::ReplaceSharedRetargetWithTertiaryZero => {
                    owner.replace_prepared(visit.slot, shared_retarget_task());
                    owner.replace_prepared(
                        ActorTaskSlot::Tertiary,
                        PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(
                            self.replacement,
                        )),
                    );
                    if self.mode == Mode::ReplaceSharedRetargetWithTertiaryError {
                        Err("discarded shared-retarget callback error")
                    } else {
                        Ok(WanderNearCommonMoverReturn::Zero)
                    }
                }
                _ => Ok(WanderNearCommonMoverReturn::NonZero),
            }
        }

        fn shared_retarget_transition(
            &mut self,
            _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            request: SharedRetargetTransitionRequest,
        ) -> Result<Option<Self::Output>, Self::Error> {
            self.events.push("shared retarget transition");
            self.shared_retarget_transitions.push(request);
            Ok(None)
        }

        fn defecate_virus_terrain_request(
            &mut self,
            visit: ActorTaskVisit,
            elapsed_micros: u32,
        ) -> Result<DefecateVirusCallbackRequest, Self::Error> {
            self.events.push("virus terrain callback");
            self.defecate_terrain_visits.push(visit.task_id);
            Ok(DefecateVirusCallbackRequest {
                update_mode: if matches!(
                    self.mode,
                    Mode::DefecateMixedRng | Mode::ClearDefecateTerrainDuringCallback
                ) {
                    crate::component_update::ComponentUpdateMode::Coarse
                } else {
                    crate::component_update::ComponentUpdateMode::Detailed
                },
                elapsed_micros: if self.mode == Mode::DefecateElapsedMismatch {
                    4_000
                } else {
                    elapsed_micros
                },
                entity_state_flags: if matches!(
                    self.mode,
                    Mode::DefecateMixedRng
                        | Mode::DefecateElapsedMismatch
                        | Mode::ClearDefecateTerrainDuringCallback
                ) {
                    0
                } else {
                    crate::defecate_virus::DEFECATE_VIRUS_SUPPRESSION_STATE_BIT
                },
                position_raw: [0x1200, 0x3400, 0x5600],
                forward_q31: [0; 3],
                model_extent_raw_by_state: if self.mode == Mode::DefecateElapsedMismatch {
                    [Some(512); 4]
                } else {
                    [None; 4]
                },
                owner_entity_handle: 0,
            })
        }

        fn apply_defecate_virus_terrain_plan(
            &mut self,
            owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            visit: ActorTaskVisit,
            plan: DefecateVirusCallbackPlan,
        ) -> Result<(), Self::Error> {
            self.defecate_terrain_plans.push(plan);
            if self.mode == Mode::ClearDefecateTerrainDuringCallback {
                owner.clear_slot(visit.slot);
                return Err("discarded Defecate terrain callback error");
            }
            Ok(())
        }

        fn defecate_virus_terrain_transition(
            &mut self,
            _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            request: DefecateVirusTerrainTransitionRequest,
        ) -> Result<DefecateVirusTransitionOutcome<Self::Output>, Self::Error> {
            self.defecate_terrain_transitions
                .push(request.committed_prefix);
            Ok(DefecateVirusTransitionOutcome::Completed(None))
        }

        fn defecate_virus_wander_actor_position_raw(&mut self, _visit: ActorTaskVisit) -> [i16; 3] {
            [100, 200, 300]
        }

        fn defecate_virus_wander_common_mover(
            &mut self,
            _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            visit: ActorTaskVisit,
            private_state: &mut WanderNearPrivateState,
        ) -> Result<WanderNearCommonMoverReturn, Self::Error> {
            self.events.push("virus wander callback");
            self.defecate_wander_visits.push(visit.task_id);
            if self.mode == Mode::UnresolvedDefecateWander {
                private_state.direction = -1;
                Ok(WanderNearCommonMoverReturn::Unresolved)
            } else if matches!(
                self.mode,
                Mode::DefecateTagCallbackAbsent
                    | Mode::DefecateTagCallbackSuppressed
                    | Mode::ReplaceSecondaryFromDefecate
            ) {
                Ok(WanderNearCommonMoverReturn::Zero)
            } else {
                Ok(WanderNearCommonMoverReturn::NonZero)
            }
        }

        fn defecate_virus_wander_transition(
            &mut self,
            owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            request: DefecateVirusWanderTransitionRequest,
        ) -> Result<DefecateVirusTransitionOutcome<Self::Output>, Self::Error> {
            self.defecate_wander_transitions.push(request.reason);
            match self.mode {
                Mode::ReplaceSecondaryFromDefecate => {
                    owner.replace_prepared(
                        ActorTaskSlot::Secondary,
                        PreparedActorTask::new(ActorTaskRuntime::DefecateVirusTerrain(
                            DefecateVirusTerrainTaskState::new(0),
                        )),
                    );
                    Ok(DefecateVirusTransitionOutcome::Completed(None))
                }
                Mode::DefecateTagCallbackAbsent
                    if matches!(
                        request.reason,
                        DefecateVirusWanderTransitionReason::CommonMoverCompleted(_)
                    ) =>
                {
                    Ok(DefecateVirusTransitionOutcome::CallbackAbsent)
                }
                Mode::DefecateTagCallbackSuppressed => {
                    Ok(DefecateVirusTransitionOutcome::SuppressedByEntityState)
                }
                _ => Ok(DefecateVirusTransitionOutcome::Completed(None)),
            }
        }

        fn common_dying_callback(
            &mut self,
            owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            visit: ActorTaskVisit,
            frame: ActorTaskDispatcherFrame,
        ) -> Result<CommonDyingCallbackResult, Self::Error> {
            self.events.push("common dying callback");
            self.common_dying_visits.push(visit);
            self.common_dying_frames.push(frame);
            match self.mode {
                Mode::ClearCommonDyingDuringCallback => {
                    owner.clear_slot(visit.slot);
                    Err("discarded common-dying callback error")
                }
                Mode::CommonDyingError => Err("unresolved common-dying callback"),
                Mode::ReplaceSecondaryFromCommonDying => Ok(
                    CommonDyingCallbackResult::TaggedOwnerTransition(CommonDyingTaggedResult {
                        singleton_address: COMMON_DYING_SCHEDULER_SINGLETON_ADDRESS,
                        tag: COMMON_DYING_OWNER_TRANSITION_TAG,
                    }),
                ),
                _ => Ok(CommonDyingCallbackResult::Continue),
            }
        }

        fn common_dying_transition(
            &mut self,
            owner: &mut ActorTaskOwner<ActorTaskRuntime>,
            request: CommonDyingTransitionRequest,
        ) -> Result<Option<Self::Output>, Self::Error> {
            self.events.push("common dying transition");
            self.common_dying_transitions.push(request);
            if self.mode == Mode::ReplaceSecondaryFromCommonDying {
                owner.replace_prepared(
                    ActorTaskSlot::Secondary,
                    PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(self.replacement)),
                );
            }
            Ok(None)
        }
    }

    #[test]
    fn run_away_transition_installs_even_or_odd_attract_graph_and_ticks_later_slots_same_pass() {
        for (mode, odd) in [
            (Mode::RunAwayToAttractEven, false),
            (Mode::RunAwayToAttractOddMiss, true),
        ] {
            let mut owner = ActorTaskOwner::new();
            owner.replace_prepared(ActorTaskSlot::Primary, run_away_task());
            let mut adapter = TestAdapter::new(mode);
            adapter.random = VecDeque::from([1]);

            assert_eq!(
                tick_actor_task_dispatcher(
                    &mut owner,
                    ActorTaskDispatcherFrame {
                        elapsed_micros: 1_000,
                        scheduler_mode: 0xA5,
                    },
                    &mut adapter,
                ),
                Ok(None)
            );
            assert_eq!(adapter.random_draws, usize::from(odd));
            assert_eq!(
                adapter.events,
                if odd {
                    vec![
                        "run away callback",
                        "run away transition",
                        "attract candidate prefix",
                        "attract candidate callback",
                    ]
                } else {
                    vec!["run away callback", "run away transition"]
                }
            );
            assert!(matches!(
                owner.state_in_slot(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::SharedRetarget(state)) if state.elapsed_ms() == 0
            ));
            assert!(matches!(
                owner.state_in_slot(ActorTaskSlot::Tertiary),
                Some(ActorTaskRuntime::AttractAttentionCue(cue)) if cue.elapsed_ms() == 1
            ));
        }
    }

    #[test]
    fn accepted_attract_candidate_retires_itself_discards_tag_and_does_not_revisit_new_primary() {
        let mut owner = ActorTaskOwner::new();
        let old_primary = owner.replace_prepared(ActorTaskSlot::Primary, run_away_task());
        let mut adapter = TestAdapter::new(Mode::RunAwayToAttractOddAccepted);
        adapter.random = VecDeque::from([0]);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert!(owner.wrapper_flags(old_primary).is_none());
        assert_eq!(adapter.random_draws, 1);
        assert!(owner.task_in_slot(ActorTaskSlot::Secondary).is_none());
        assert!(owner.task_in_slot(ActorTaskSlot::Tertiary).is_none());
        assert!(matches!(
            owner.state_in_slot(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::AttractAttentionTargetRoute(state)) if state.elapsed_ms() == 0
        ));
    }

    #[test]
    fn retired_attract_candidate_discards_callback_error_and_continues_fresh_slots() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::AttractAttentionCandidate(
                crate::attract_attention::AttractAttentionCandidateTaskState::new(0x201),
            )),
        );
        let cue_id = owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(ActorTaskRuntime::AttractAttentionCue(
                crate::attract_attention::AttractAttentionCueTaskState::new(1_000),
            )),
        );
        let mut adapter = TestAdapter::new(Mode::AttractCandidateRetiredError);
        adapter.random = VecDeque::from([0]);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 7_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert!(matches!(
            owner.task_state(cue_id),
            Some(ActorTaskRuntime::AttractAttentionCue(cue)) if cue.elapsed_ms() == 7
        ));
    }

    #[test]
    fn surviving_attract_candidate_propagates_opaque_result_before_later_cue() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::AttractAttentionCandidate(
                crate::attract_attention::AttractAttentionCandidateTaskState::new(0x201),
            )),
        );
        let cue_id = owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(ActorTaskRuntime::AttractAttentionCue(
                crate::attract_attention::AttractAttentionCueTaskState::new(1_000),
            )),
        );
        let mut adapter = TestAdapter::new(Mode::AttractCandidatePropagate);
        adapter.random = VecDeque::from([0]);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 7_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Ok(Some("Attract candidate propagated"))
        );
        assert_eq!(adapter.random_draws, 1);
        assert!(matches!(
            owner.task_state(cue_id),
            Some(ActorTaskRuntime::AttractAttentionCue(cue)) if cue.elapsed_ms() == 0
        ));
    }

    #[test]
    fn surviving_attract_candidate_accepted_tag_fails_closed_before_later_cue() {
        let mut owner = ActorTaskOwner::new();
        let candidate_id = owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::AttractAttentionCandidate(
                crate::attract_attention::AttractAttentionCandidateTaskState::new(0x201),
            )),
        );
        let cue_id = owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(ActorTaskRuntime::AttractAttentionCue(
                crate::attract_attention::AttractAttentionCueTaskState::new(1_000),
            )),
        );
        let mut adapter = TestAdapter::new(Mode::AttractCandidateSurvivingTag);
        adapter.random = VecDeque::from([0]);

        let result = tick_actor_task_dispatcher(
            &mut owner,
            ActorTaskDispatcherFrame {
                elapsed_micros: 7_000,
                scheduler_mode: 0,
            },
            &mut adapter,
        );
        assert_eq!(adapter.random_draws, 1);
        assert_eq!(
            result,
            Err(
                ActorTaskDispatcherError::AttractAttentionCandidateSurvivingAccepted {
                    visit: ActorTaskVisit {
                        slot: ActorTaskSlot::Secondary,
                        task_id: candidate_id,
                    },
                    committed_prefix: adapter.attract_candidate_prefixes[0],
                    candidate: GuardLocationCandidate { id: 0x04FC_0001 },
                    singleton: GuardLocationAcquisitionTaggedSingleton::CandidateAccepted,
                }
            )
        );
        assert!(matches!(
            owner.task_state(cue_id),
            Some(ActorTaskRuntime::AttractAttentionCue(cue)) if cue.elapsed_ms() == 0
        ));
    }

    #[test]
    fn attract_cue_uses_strict_lifetime_and_transitions_only_after_unwind() {
        let mut owner = ActorTaskOwner::new();
        let cue_id = owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(ActorTaskRuntime::AttractAttentionCue(
                crate::attract_attention::AttractAttentionCueTaskState::new(1_000),
            )),
        );
        let mut adapter = TestAdapter::new(Mode::AttractCueCompleted);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert!(adapter.attract_cue_transitions.is_empty());
        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert_eq!(adapter.attract_cue_transitions.len(), 1);
        assert_eq!(
            adapter.attract_cue_transitions[0].expired_visit.task_id,
            cue_id
        );
        assert_eq!(
            adapter.attract_cue_transitions[0]
                .committed_prefix
                .elapsed_ms,
            1_001
        );
        assert!(!owner.wrapper_flags(cue_id).unwrap().in_callback);
    }

    #[test]
    fn suppressed_attract_cue_transition_retains_the_expired_wrapper() {
        let mut owner = ActorTaskOwner::new();
        let cue_id = owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(ActorTaskRuntime::AttractAttentionCue(
                crate::attract_attention::AttractAttentionCueTaskState::new(1_000),
            )),
        );
        let mut adapter = TestAdapter::new(Mode::AttractCueSuppressed);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_001_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert_eq!(owner.task_in_slot(ActorTaskSlot::Tertiary), Some(cue_id));
        assert_eq!(adapter.attract_cue_transitions.len(), 1);
    }

    #[test]
    fn attract_target_route_fails_closed_before_an_earlier_task_can_tick() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                SharedRetargetTaskState::new([111, 22, -333], 1_000),
            )),
        );
        let target_route_id = owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::AttractAttentionTargetRoute(
                SharedTargetRouteTaskState::after_allocation(
                    [0x0100, -0x0200, 0x0300],
                    0x04AC_0001,
                ),
            )),
        );
        let before =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| owner.state_in_slot(slot).copied());
        let mut adapter = TestAdapter::new(Mode::ChaseContinue);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 96_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Err(ActorTaskDispatcherError::DedicatedExactVisitRequired {
                visit: ActorTaskVisit {
                    slot: ActorTaskSlot::Secondary,
                    task_id: target_route_id,
                },
                family: ActorTaskRuntimeFamily::AttractAttentionTargetRoute,
            })
        );
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| owner.state_in_slot(slot).copied()),
            before
        );
        assert!(adapter.events.is_empty());
        assert_eq!(adapter.random_draws, 0);
    }

    #[test]
    fn dedicated_class0_timer_pre_scan_leaves_earlier_task_untouched() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let timer = ActorTaskRuntime::Class0Timer(crate::class0_timer::Class0TimerTaskState::new());
        assert_ne!(timer.family(), ActorTaskRuntime::None.family());
        let exact_task_id =
            owner.replace_prepared(ActorTaskSlot::Secondary, PreparedActorTask::new(timer));
        let before =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| owner.state_in_slot(slot).copied());
        let mut adapter = TestAdapter::new(Mode::ChaseContinue);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 9_001_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Err(ActorTaskDispatcherError::DedicatedExactVisitRequired {
                visit: ActorTaskVisit {
                    slot: ActorTaskSlot::Secondary,
                    task_id: exact_task_id,
                },
                family: ActorTaskRuntimeFamily::Class0Timer,
            })
        );
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| owner.state_in_slot(slot).copied()),
            before
        );
        assert!(adapter.events.is_empty());
        assert!(adapter.visited_wander.is_empty());
        assert_eq!(adapter.random_draws, 0);
    }

    #[test]
    fn dedicated_change_sea_level_pre_scan_leaves_earlier_task_untouched() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let exact_task_id = owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::ChangeSeaLevel(
                crate::main_base_type54_abort::MainBaseType54SeaLevelTaskState::new(
                    crate::main_base_type54_abort::LEVEL_ONE_TYPE54_ACCEPTED_SEA_DELTA_RAW,
                ),
            )),
        );
        let primary_before = *owner.state_in_slot(ActorTaskSlot::Primary).unwrap();
        let secondary_before = *owner.state_in_slot(ActorTaskSlot::Secondary).unwrap();
        let mut adapter = TestAdapter::new(Mode::ChaseContinue);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 96_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Err(ActorTaskDispatcherError::DedicatedExactVisitRequired {
                visit: ActorTaskVisit {
                    slot: ActorTaskSlot::Secondary,
                    task_id: exact_task_id,
                },
                family: ActorTaskRuntimeFamily::ChangeSeaLevel,
            })
        );
        assert_eq!(
            owner.state_in_slot(ActorTaskSlot::Primary),
            Some(&primary_before)
        );
        assert_eq!(
            owner.state_in_slot(ActorTaskSlot::Secondary),
            Some(&secondary_before)
        );
        assert!(adapter.events.is_empty());
        assert!(adapter.visited_wander.is_empty());
        assert_eq!(adapter.random_draws, 0);
    }

    #[test]
    fn dedicated_working_factory_pre_scan_leaves_earlier_task_untouched() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let exact_task_id = owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::WorkingFactory(
                crate::main_base_type66_abort::WorkingFactoryTaskState::new(25),
            )),
        );
        let primary_before = *owner.state_in_slot(ActorTaskSlot::Primary).unwrap();
        let secondary_before = *owner.state_in_slot(ActorTaskSlot::Secondary).unwrap();
        let mut adapter = TestAdapter::new(Mode::ChaseContinue);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 96_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Err(ActorTaskDispatcherError::DedicatedExactVisitRequired {
                visit: ActorTaskVisit {
                    slot: ActorTaskSlot::Secondary,
                    task_id: exact_task_id,
                },
                family: ActorTaskRuntimeFamily::WorkingFactory,
            })
        );
        assert_eq!(
            owner.state_in_slot(ActorTaskSlot::Primary),
            Some(&primary_before)
        );
        assert_eq!(
            owner.state_in_slot(ActorTaskSlot::Secondary),
            Some(&secondary_before)
        );
        assert!(adapter.events.is_empty());
        assert!(adapter.visited_wander.is_empty());
        assert_eq!(adapter.random_draws, 0);
    }

    #[test]
    fn dedicated_exploding_ring_pre_scan_leaves_earlier_task_untouched() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let exact_task_id = owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::ExplodingRing(
                crate::type60_exploding_ring::Type60ExplodingRingTaskState::new(),
            )),
        );
        let primary_before = *owner.state_in_slot(ActorTaskSlot::Primary).unwrap();
        let secondary_before = *owner.state_in_slot(ActorTaskSlot::Secondary).unwrap();
        let mut adapter = TestAdapter::new(Mode::ChaseContinue);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 96_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Err(ActorTaskDispatcherError::DedicatedExactVisitRequired {
                visit: ActorTaskVisit {
                    slot: ActorTaskSlot::Secondary,
                    task_id: exact_task_id,
                },
                family: ActorTaskRuntimeFamily::ExplodingRing,
            })
        );
        assert_eq!(
            owner.state_in_slot(ActorTaskSlot::Primary),
            Some(&primary_before)
        );
        assert_eq!(
            owner.state_in_slot(ActorTaskSlot::Secondary),
            Some(&secondary_before)
        );
        assert!(adapter.events.is_empty());
        assert!(adapter.visited_wander.is_empty());
        assert_eq!(adapter.random_draws, 0);
    }

    #[test]
    fn dedicated_hive_radial_pre_scan_leaves_earlier_task_untouched() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let exact_task_id = owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::HiveRadial(
                crate::hive_death::HiveRadialTaskState::live(),
            )),
        );
        let primary_before = *owner.state_in_slot(ActorTaskSlot::Primary).unwrap();
        let secondary_before = *owner.state_in_slot(ActorTaskSlot::Secondary).unwrap();
        let mut adapter = TestAdapter::new(Mode::ChaseContinue);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 96_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Err(ActorTaskDispatcherError::DedicatedExactVisitRequired {
                visit: ActorTaskVisit {
                    slot: ActorTaskSlot::Secondary,
                    task_id: exact_task_id,
                },
                family: ActorTaskRuntimeFamily::HiveRadial,
            })
        );
        assert_eq!(
            owner.state_in_slot(ActorTaskSlot::Primary),
            Some(&primary_before)
        );
        assert_eq!(
            owner.state_in_slot(ActorTaskSlot::Secondary),
            Some(&secondary_before)
        );
        assert!(adapter.events.is_empty());
        assert!(adapter.visited_wander.is_empty());
        assert_eq!(adapter.random_draws, 0);
    }

    #[test]
    fn hive_death_clears_later_slots_then_reallocates_slot0() {
        let mut owner = ActorTaskOwner::new();
        let live_id = owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::HiveRadial(
                crate::hive_death::HiveRadialTaskState::live(),
            )),
        );
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );

        owner.clear_slot(ActorTaskSlot::Tertiary);
        owner.clear_slot(ActorTaskSlot::Secondary);
        let dying_id = owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::HiveRadial(
                crate::hive_death::HiveRadialTaskState::dying(),
            )),
        );

        assert_ne!(live_id, dying_id);
        assert_eq!(
            owner.state_in_slot(ActorTaskSlot::Primary),
            Some(&ActorTaskRuntime::HiveRadial(
                crate::hive_death::HiveRadialTaskState::dying()
            ))
        );
        assert_eq!(owner.state_in_slot(ActorTaskSlot::Secondary), None);
        assert_eq!(owner.state_in_slot(ActorTaskSlot::Tertiary), None);
    }

    #[test]
    fn common_dying_uses_strict_lifetime_after_committing_each_frame_prefix() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, common_dying_task(0x0497_0001));
        let mut adapter = TestAdapter::new(Mode::CommonDyingContinue);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 9_000_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert!(adapter.common_dying_transitions.is_empty());
        let Some(ActorTaskRuntime::CommonDying(state)) =
            owner.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("primary task changed family")
        };
        assert_eq!(state.elapsed_ms(), 9_000);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert_eq!(
            adapter.common_dying_transitions,
            [CommonDyingTransitionRequest {
                visit: adapter.common_dying_visits[1],
                reason: CommonDyingTransitionReason::LifetimeExpired,
                committed_prefix: CommonDyingCallbackPrefix {
                    elapsed_ms: 9_001,
                    lifetime_status:
                        crate::common_dying::CommonDyingLifetimeStatus::OwnerTransitionDue,
                },
            }]
        );
    }

    #[test]
    fn common_dying_tag_preempts_expired_lifetime_and_visits_replaced_later_slot() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, common_dying_task(0x0497_0001));
        let old_secondary = owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let mut adapter = TestAdapter::new(Mode::ReplaceSecondaryFromCommonDying);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 9_001_000,
                    scheduler_mode: 1,
                },
                &mut adapter,
            ),
            Ok(None)
        );

        assert_eq!(
            adapter.common_dying_transitions[0].reason,
            CommonDyingTransitionReason::TaggedSchedulerMode(CommonDyingTaggedResult {
                singleton_address: COMMON_DYING_SCHEDULER_SINGLETON_ADDRESS,
                tag: COMMON_DYING_OWNER_TRANSITION_TAG,
            })
        );
        assert_eq!(
            adapter.common_dying_transitions[0]
                .committed_prefix
                .lifetime_status,
            crate::common_dying::CommonDyingLifetimeStatus::OwnerTransitionDue
        );
        assert_eq!(
            adapter.events,
            [
                "common dying callback",
                "common dying transition",
                "wander callback",
                "wander transition"
            ]
        );
        assert_eq!(adapter.visited_wander.len(), 1);
        assert_ne!(adapter.visited_wander[0], old_secondary);
    }

    #[test]
    fn common_dying_self_clear_discards_callback_error_and_visits_later_slot() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, common_dying_task(0x0497_0001));
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let mut adapter = TestAdapter::new(Mode::ClearCommonDyingDuringCallback);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert_eq!(owner.state_in_slot(ActorTaskSlot::Primary), None);
        assert!(adapter.common_dying_transitions.is_empty());
        assert_eq!(adapter.events, ["common dying callback", "wander callback"]);
    }

    #[test]
    fn surviving_common_dying_callback_error_keeps_committed_elapsed_prefix() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, common_dying_task(0x0497_0001));
        let mut adapter = TestAdapter::new(Mode::CommonDyingError);

        let result = tick_actor_task_dispatcher(
            &mut owner,
            ActorTaskDispatcherFrame {
                elapsed_micros: 1_999,
                scheduler_mode: 0,
            },
            &mut adapter,
        );

        assert!(matches!(
            result,
            Err(ActorTaskDispatcherError::CommonDyingCallback {
                visit: ActorTaskVisit {
                    slot: ActorTaskSlot::Primary,
                    ..
                },
                committed_prefix: CommonDyingCallbackPrefix {
                    elapsed_ms: 1,
                    lifetime_status: crate::common_dying::CommonDyingLifetimeStatus::WithinLifetime,
                },
                error: "unresolved common-dying callback",
            })
        ));
        let Some(ActorTaskRuntime::CommonDying(state)) =
            owner.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("primary task changed family")
        };
        assert_eq!(state.elapsed_ms(), 1);
        assert!(adapter.common_dying_transitions.is_empty());
    }

    #[test]
    fn go_transition_replaces_later_family_and_same_pass_visits_it_fresh() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::GoToJob(go_to_job_state())),
        );
        let old_secondary = owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let mut adapter = TestAdapter::new(Mode::ReplaceSecondary);
        adapter.old_secondary = Some(old_secondary);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );

        assert_eq!(
            adapter.events,
            ["go callback", "go transition", "wander callback"]
        );
        assert_eq!(adapter.visited_wander.len(), 1);
        assert_ne!(adapter.visited_wander[0], old_secondary);
        let ActorTaskRuntime::GoToJob(state) = owner.state_in_slot(ActorTaskSlot::Primary).unwrap()
        else {
            panic!("primary task changed family")
        };
        assert_eq!(state.private_state().target_position_raw, [9, 8, 7]);
    }

    #[test]
    fn self_clear_discards_callback_error_and_staged_go_state() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::GoToJob(go_to_job_state())),
        );
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let mut adapter = TestAdapter::new(Mode::ClearGoDuringCallback);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );

        assert_eq!(owner.state_in_slot(ActorTaskSlot::Primary), None);
        assert_eq!(adapter.go_transitions, 0);
        assert_eq!(adapter.events, ["go callback", "wander callback"]);
    }

    #[test]
    fn resolved_chase_callback_commits_staged_private_state() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::ChaseTarget(chase_target_state(77))),
        );
        let mut adapter = TestAdapter::new(Mode::ChaseContinue);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );

        let Some(ActorTaskRuntime::ChaseTarget(state)) =
            owner.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("primary task changed family")
        };
        assert_eq!(state.elapsed_ms(), 1);
        assert_eq!(state.private_state().target_position_raw, [9, 8, 7]);
        assert!(adapter.chase_transitions.is_empty());
    }

    #[test]
    fn unresolved_chase_callback_commits_elapsed_but_discards_staged_state() {
        let original = chase_target_state(77);
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::ChaseTarget(original)),
        );
        let mut adapter = TestAdapter::new(Mode::ChaseError);

        let result = tick_actor_task_dispatcher(
            &mut owner,
            ActorTaskDispatcherFrame {
                elapsed_micros: 1_000,
                scheduler_mode: 0xA5,
            },
            &mut adapter,
        );

        assert!(matches!(
            result,
            Err(ActorTaskDispatcherError::ChaseTargetCallback {
                visit: ActorTaskVisit {
                    slot: ActorTaskSlot::Primary,
                    ..
                },
                error: "unresolved Chase callback",
                ..
            })
        ));
        let Some(ActorTaskRuntime::ChaseTarget(state)) =
            owner.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("primary task changed family")
        };
        assert_eq!(state.elapsed_ms(), 1);
        assert_eq!(state.private_state(), original.private_state());
        assert!(adapter.chase_transitions.is_empty());
    }

    #[test]
    fn chase_self_clear_discards_callback_error_and_visits_later_slot() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::ChaseTarget(chase_target_state(77))),
        );
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let mut adapter = TestAdapter::new(Mode::ClearChaseDuringCallback);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );

        assert_eq!(owner.state_in_slot(ActorTaskSlot::Primary), None);
        assert_eq!(adapter.events, ["chase callback", "wander callback"]);
        assert!(adapter.chase_transitions.is_empty());
    }

    #[test]
    fn chase_transition_replaces_later_family_and_visits_it_fresh() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::ChaseTarget(chase_target_state(77))),
        );
        let old_secondary = owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let mut adapter = TestAdapter::new(Mode::ReplaceSecondaryFromChase);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );

        assert_eq!(
            adapter.chase_transitions,
            [ChaseTargetTransitionReason::TaggedCallbackResult(
                ChaseTargetTaggedSingleton::OutOfRange
            )]
        );
        assert_eq!(
            adapter.events,
            ["chase callback", "chase transition", "wander callback"]
        );
        assert_eq!(adapter.visited_wander.len(), 1);
        assert_ne!(adapter.visited_wander[0], old_secondary);
    }

    #[test]
    fn target_acquisition_runtime_preparation_accepts_only_its_exact_phase() {
        let setup = search_attack_variant_setup(SearchAttackVariant::Acquiring);
        let preparation = SearchAttackTaskPreparation {
            request: SearchAttackTaskSetupRequest::Acquiring,
            phase_index: 0,
            task: setup.ordered_phases[0].install,
        };
        let prepared = prepare_target_acquisition_runtime_task(
            preparation,
            SearchAttackRadius::strict(0x1000).unwrap(),
            SearchAttackCandidateFilter::SameEntityType,
            0x20,
        )
        .unwrap();
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Secondary, prepared);

        let Some(ActorTaskRuntime::TargetAcquisition(state)) =
            owner.state_in_slot(ActorTaskSlot::Secondary)
        else {
            panic!("secondary task changed family")
        };
        assert_eq!(state.radius(), SearchAttackRadius::strict(0x1000).unwrap());
        assert_eq!(state.filter(), SearchAttackCandidateFilter::SameEntityType);
        assert_eq!(state.constructor_filter_override_raw(), 0x20);

        let error = prepare_target_acquisition_runtime_task(
            SearchAttackTaskPreparation {
                request: SearchAttackTaskSetupRequest::Acquiring,
                phase_index: 1,
                task: setup.ordered_phases[1].install,
            },
            SearchAttackRadius::Unbounded,
            SearchAttackCandidateFilter::SameEntityType,
            0,
        )
        .unwrap_err();
        assert_eq!(
            error,
            TargetAcquisitionRuntimePreparationError::TaskContractMismatch
        );
    }

    #[test]
    fn guard_pursuing_aim_preparation_uses_the_shared_exact_task_contract() {
        let metadata = EntityTypeRuntimeMetadata {
            common_mover_topology: RetailRuntimeValue::Known(CommonMoverComponentTopology {
                sub_e: true,
                ..CommonMoverComponentTopology::default()
            }),
            ..EntityTypeRuntimeMetadata::default()
        };
        let prepared = prepare_aim_and_fire_runtime_task(
            AimAndFireRuntimeTaskPreparation {
                slot: ActorTaskSlot::Tertiary,
                constructor_address: AIM_AND_FIRE_CONSTRUCTOR_ADDRESS,
                tick_address: AIM_AND_FIRE_TICK_ADDRESS,
                lifetime_ms: AIM_AND_FIRE_LIFETIME_MS,
                target_id: 0x047F_0001,
            },
            0x0497_0001,
            [100, 200, 300],
            0,
            0,
            &metadata,
        )
        .unwrap();
        assert!(!prepared.suffix().enables_sub_f());

        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            prepared.apply_suffix(|_, suffix| assert!(!suffix.enables_sub_f())),
        );
        let Some(ActorTaskRuntime::AimAndFire(state)) =
            owner.state_in_slot(ActorTaskSlot::Tertiary)
        else {
            panic!("tertiary task changed family")
        };
        assert_eq!(state.private_state().target_entity_id(), 0x047F_0001);
        assert_eq!(state.private_state().optional_sound_id(), None);
        assert_eq!(state.private_state().sound_period_us_raw(), 0);
    }

    #[test]
    fn shared_aim_preparation_rejects_task_identity_mismatches_before_the_suffix() {
        let exact = AimAndFireRuntimeTaskPreparation {
            slot: ActorTaskSlot::Tertiary,
            constructor_address: AIM_AND_FIRE_CONSTRUCTOR_ADDRESS,
            tick_address: AIM_AND_FIRE_TICK_ADDRESS,
            lifetime_ms: AIM_AND_FIRE_LIFETIME_MS,
            target_id: 0x047F_0001,
        };
        let mismatches = [
            AimAndFireRuntimeTaskPreparation {
                slot: ActorTaskSlot::Secondary,
                ..exact
            },
            AimAndFireRuntimeTaskPreparation {
                constructor_address: AIM_AND_FIRE_CONSTRUCTOR_ADDRESS + 1,
                ..exact
            },
            AimAndFireRuntimeTaskPreparation {
                tick_address: AIM_AND_FIRE_TICK_ADDRESS + 1,
                ..exact
            },
            AimAndFireRuntimeTaskPreparation {
                lifetime_ms: AIM_AND_FIRE_LIFETIME_MS + 1,
                ..exact
            },
        ];

        for mismatch in mismatches {
            assert_eq!(
                prepare_aim_and_fire_runtime_task(
                    mismatch,
                    0x0497_0001,
                    [0; 3],
                    0,
                    0,
                    &EntityTypeRuntimeMetadata::default(),
                )
                .unwrap_err(),
                AimAndFireRuntimePreparationError::TaskContractMismatch
            );
        }
    }

    #[test]
    fn aim_and_fire_runtime_preparation_accepts_only_the_exact_pursuing_phase() {
        let setup = search_attack_variant_setup(SearchAttackVariant::Pursuing);
        let request = SearchAttackTaskSetupRequest::Pursuing {
            target_id: 0x047F_0001,
        };
        let preparation = SearchAttackTaskPreparation {
            request,
            phase_index: 0,
            task: setup.ordered_phases[0].install,
        };
        let metadata = EntityTypeRuntimeMetadata {
            common_mover_topology: RetailRuntimeValue::Known(CommonMoverComponentTopology {
                sub_f: true,
                ..CommonMoverComponentTopology::default()
            }),
            ..EntityTypeRuntimeMetadata::default()
        };
        let prepared = prepare_search_attack_aim_and_fire_runtime_task(
            preparation,
            0x0497_0001,
            [100, 200, 300],
            70,
            750_000,
            &metadata,
        )
        .unwrap();
        assert!(prepared.suffix().enables_sub_f());
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            prepared.apply_suffix(|owner_id, suffix| {
                assert_eq!(owner_id, 0x0497_0001);
                assert!(suffix.enables_sub_f());
            }),
        );
        let Some(ActorTaskRuntime::AimAndFire(state)) =
            owner.state_in_slot(ActorTaskSlot::Tertiary)
        else {
            panic!("tertiary task changed family")
        };
        assert_eq!(state.private_state().target_entity_id(), 0x047F_0001);

        let error = prepare_search_attack_aim_and_fire_runtime_task(
            SearchAttackTaskPreparation {
                request,
                phase_index: 1,
                task: setup.ordered_phases[1].install,
            },
            0x0497_0001,
            [0; 3],
            0,
            0,
            &metadata,
        )
        .unwrap_err();
        assert_eq!(
            error,
            SearchAttackAimAndFireRuntimePreparationError::TaskContractMismatch
        );
    }

    #[test]
    fn aim_scheduler_gate_advances_elapsed_and_routes_strict_timeout_without_callback() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Tertiary, aim_and_fire_task());
        let mut adapter = TestAdapter::new(Mode::AimError);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 5_001_000,
                    scheduler_mode: 1,
                },
                &mut adapter,
            ),
            Ok(None)
        );

        assert_eq!(adapter.events, ["aim transition"]);
        assert_eq!(adapter.aim_transitions.len(), 1);
        assert_eq!(
            adapter.aim_transitions[0].reason,
            AimAndFireTransitionReason::LifetimeExpired
        );
        assert_eq!(
            adapter.aim_transitions[0].committed_prefix.elapsed_ms,
            5_001
        );
    }

    #[test]
    fn aim_completed_zero_tag_callback_exits_without_expired_lifetime_callback() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Tertiary, aim_and_fire_task());
        let mut adapter = TestAdapter::new(Mode::AimTagged);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 5_001_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Ok(None)
        );

        assert_eq!(adapter.events, ["aim callback", "aim transition"]);
        assert_eq!(adapter.aim_transitions.len(), 1);
        assert_eq!(
            adapter.aim_transitions[0].reason,
            AimAndFireTransitionReason::TaggedInvalidTarget {
                singleton: crate::aim_and_fire::AimAndFireTaggedSingleton::InvalidTarget,
                reason: crate::aim_and_fire::AimAndFireInvalidTargetReason::Dying,
            }
        );
        assert_eq!(
            crate::aim_and_fire::AimAndFireTaggedSingleton::InvalidTarget.tag(),
            0x9C00
        );
    }

    #[test]
    fn aim_absent_tag_callback_falls_through_to_expired_lifetime_callback() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Tertiary, aim_and_fire_task());
        let mut adapter = TestAdapter::new(Mode::AimTagCallbackAbsent);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 5_001_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Ok(None)
        );

        assert_eq!(
            adapter.events,
            ["aim callback", "aim transition", "aim transition"]
        );
        assert_eq!(adapter.aim_transitions.len(), 2);
        assert!(matches!(
            adapter.aim_transitions[0].reason,
            AimAndFireTransitionReason::TaggedInvalidTarget { .. }
        ));
        assert_eq!(
            adapter.aim_transitions[1].reason,
            AimAndFireTransitionReason::LifetimeExpired
        );
    }

    #[test]
    fn aim_suppressed_tag_callback_falls_through_to_expired_lifetime_callback() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Tertiary, aim_and_fire_task());
        let mut adapter = TestAdapter::new(Mode::AimTagCallbackSuppressed);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 5_001_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Ok(None)
        );

        assert_eq!(
            adapter.events,
            ["aim callback", "aim transition", "aim transition"]
        );
        assert_eq!(adapter.aim_transitions.len(), 2);
        assert!(matches!(
            adapter.aim_transitions[0].reason,
            AimAndFireTransitionReason::TaggedInvalidTarget { .. }
        ));
        assert_eq!(
            adapter.aim_transitions[1].reason,
            AimAndFireTransitionReason::LifetimeExpired
        );
    }

    #[test]
    fn aim_nonzero_emitter_result_propagates_before_expired_lifetime() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Tertiary, aim_and_fire_task());
        let mut adapter = TestAdapter::new(Mode::AimPropagate);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 5_001_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Ok(Some("Aim-and-Fire propagated"))
        );
        assert!(adapter.aim_transitions.is_empty());
    }

    #[test]
    fn surviving_aim_callback_error_keeps_pre_callback_elapsed_commit() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Tertiary, aim_and_fire_task());
        let mut adapter = TestAdapter::new(Mode::AimError);

        let result = tick_actor_task_dispatcher(
            &mut owner,
            ActorTaskDispatcherFrame {
                elapsed_micros: 1_999,
                scheduler_mode: 0,
            },
            &mut adapter,
        );
        assert!(matches!(
            result,
            Err(ActorTaskDispatcherError::AimAndFireCallback {
                committed_prefix: AimAndFireCallbackPrefix {
                    elapsed_ms: 1,
                    lifetime_status: crate::aim_and_fire::AimAndFireLifetimeStatus::WithinLifetime,
                },
                error: "Aim-and-Fire emitter error",
                ..
            })
        ));
        let Some(ActorTaskRuntime::AimAndFire(state)) =
            owner.state_in_slot(ActorTaskSlot::Tertiary)
        else {
            panic!("tertiary task changed family")
        };
        assert_eq!(state.elapsed_ms(), 1);
        assert!(adapter.aim_transitions.is_empty());
    }

    #[test]
    fn aim_self_clear_discards_emitter_error_and_visits_fresh_later_slot() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, aim_and_fire_task());
        let old_secondary = owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let mut adapter = TestAdapter::new(Mode::ClearAimAndReplaceSecondary);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert_eq!(owner.state_in_slot(ActorTaskSlot::Primary), None);
        assert!(adapter.aim_transitions.is_empty());
        assert_eq!(adapter.events, ["aim callback", "wander callback"]);
        assert_eq!(adapter.visited_wander.len(), 1);
        assert_ne!(adapter.visited_wander[0], old_secondary);
    }

    #[test]
    fn target_acquisition_prefix_commits_override_and_neutral_result_continues() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::TargetAcquisition(
                target_acquisition_state(u32::MAX),
            )),
        );
        let mut adapter = TestAdapter::new(Mode::TargetAcquisitionContinue);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert_eq!(
            adapter.target_acquisition_prefixes,
            [TargetAcquisitionCallbackPrefix {
                radius: SearchAttackRadius::strict(0x1000).unwrap(),
                filter: SearchAttackCandidateFilter::SameEntityType,
            }]
        );
        let Some(ActorTaskRuntime::TargetAcquisition(state)) =
            owner.state_in_slot(ActorTaskSlot::Secondary)
        else {
            panic!("secondary task changed family")
        };
        assert_eq!(state.filter(), SearchAttackCandidateFilter::SameEntityType);
    }

    #[test]
    fn surviving_target_acquisition_propagates_nonzero_behavior_result() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::TargetAcquisition(
                target_acquisition_state(0),
            )),
        );
        let mut adapter = TestAdapter::new(Mode::TargetAcquisitionPropagate);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(Some("target acquisition propagated"))
        );
    }

    #[test]
    fn tagged_target_acquisition_result_is_consumed_and_continues() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::TargetAcquisition(
                target_acquisition_state(0),
            )),
        );
        owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let mut adapter = TestAdapter::new(Mode::TargetAcquisitionTagged);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert_eq!(
            adapter.events,
            ["target acquisition callback", "wander callback"]
        );
    }

    #[test]
    fn surviving_target_acquisition_error_fails_closed() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::TargetAcquisition(
                target_acquisition_state(0),
            )),
        );
        let mut adapter = TestAdapter::new(Mode::TargetAcquisitionError);

        assert!(matches!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Err(ActorTaskDispatcherError::TargetAcquisitionCallback {
                visit: ActorTaskVisit {
                    slot: ActorTaskSlot::Secondary,
                    ..
                },
                error: "unresolved target acquisition",
                ..
            })
        ));
    }

    #[test]
    fn target_acquisition_self_replacement_discards_result_and_visits_new_tertiary() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::TargetAcquisition(
                target_acquisition_state(0),
            )),
        );
        let mut adapter = TestAdapter::new(Mode::ReplaceTargetAcquisitionWithTertiary);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert_eq!(owner.state_in_slot(ActorTaskSlot::Secondary), None);
        assert!(matches!(
            owner.state_in_slot(ActorTaskSlot::Tertiary),
            Some(ActorTaskRuntime::OrdinaryType9Wander(_))
        ));
        assert_eq!(
            adapter.events,
            ["target acquisition callback", "wander callback"]
        );
        assert_eq!(adapter.visited_wander.len(), 1);
    }

    #[test]
    fn follow_beacon_prefix_forces_ranked_mask_and_neutral_results_visit_later_slots() {
        for mode in [Mode::FollowBeaconContinue, Mode::FollowBeaconTagged] {
            let mut owner = ActorTaskOwner::new();
            owner.replace_prepared(
                ActorTaskSlot::Secondary,
                follow_beacon_acquisition_task(0xDEAD_BEEF),
            );
            owner.replace_prepared(
                ActorTaskSlot::Tertiary,
                PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
            );
            let mut adapter = TestAdapter::new(mode);

            assert_eq!(
                tick_actor_task_dispatcher(
                    &mut owner,
                    ActorTaskDispatcherFrame {
                        elapsed_micros: 1_000,
                        scheduler_mode: 0xA5,
                    },
                    &mut adapter,
                ),
                Ok(None)
            );
            assert_eq!(
                adapter.follow_beacon_prefixes,
                [FollowBeaconAcquisitionCallbackPrefix::new(
                    WrappedAxisRange::strict(0x0A00).unwrap(),
                )]
            );
            let Some(ActorTaskRuntime::FollowBeaconAcquisition(state)) =
                owner.state_in_slot(ActorTaskSlot::Secondary)
            else {
                panic!("secondary task changed family")
            };
            assert_eq!(
                state.filter_raw(),
                crate::follow_beacons::FOLLOW_BEACON_REQUIRED_CAPABILITY_MASK
            );
            assert_eq!(
                adapter.events,
                ["follow beacon callback", "wander callback"]
            );
        }
    }

    #[test]
    fn surviving_follow_beacon_nonzero_propagates_and_prevents_later_slot_visit() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Secondary, follow_beacon_acquisition_task(0));
        owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let mut adapter = TestAdapter::new(Mode::FollowBeaconPropagate);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(Some("Follow Beacons propagated"))
        );
        assert_eq!(adapter.events, ["follow beacon callback"]);
        assert!(adapter.visited_wander.is_empty());
    }

    #[test]
    fn surviving_follow_beacon_error_fails_closed_and_prevents_later_slot_visit() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Secondary, follow_beacon_acquisition_task(0));
        owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let mut adapter = TestAdapter::new(Mode::FollowBeaconError);

        assert!(matches!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Err(ActorTaskDispatcherError::FollowBeaconAcquisitionCallback {
                visit: ActorTaskVisit {
                    slot: ActorTaskSlot::Secondary,
                    ..
                },
                committed_prefix,
                error: "unresolved Follow Beacons snapshot",
            }) if committed_prefix.required_capability_mask()
                == crate::follow_beacons::FOLLOW_BEACON_REQUIRED_CAPABILITY_MASK
        ));
        assert_eq!(adapter.events, ["follow beacon callback"]);
        assert!(adapter.visited_wander.is_empty());
    }

    #[test]
    fn synchronous_follow_handoff_discards_retired_result_and_does_not_revisit_primary() {
        for mode in [
            Mode::ReplaceFollowBeaconTagged,
            Mode::ReplaceFollowBeaconPropagate,
            Mode::ReplaceFollowBeaconError,
        ] {
            let mut owner = ActorTaskOwner::new();
            owner.replace_prepared(ActorTaskSlot::Secondary, follow_beacon_acquisition_task(0));
            owner.replace_prepared(
                ActorTaskSlot::Tertiary,
                PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
            );
            let mut adapter = TestAdapter::new(mode);

            assert_eq!(
                tick_actor_task_dispatcher(
                    &mut owner,
                    ActorTaskDispatcherFrame {
                        elapsed_micros: 1_000,
                        scheduler_mode: 0xA5,
                    },
                    &mut adapter,
                ),
                Ok(None),
                "retired callback outcome must be discarded for {mode:?}"
            );
            assert_eq!(owner.state_in_slot(ActorTaskSlot::Secondary), None);
            assert_eq!(owner.state_in_slot(ActorTaskSlot::Tertiary), None);
            assert!(matches!(
                owner.state_in_slot(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::OrdinaryType9Wander(_))
            ));
            assert_eq!(adapter.events, ["follow beacon callback"]);
            assert!(
                adapter.visited_wander.is_empty(),
                "a Primary installed from Secondary cannot be revisited this pass"
            );
        }
    }

    #[test]
    fn following_constructor_applies_generic_type17_suffix_before_fixed_speed_overwrite() {
        let prepared = prepare_follow_beacons_following_runtime_task(
            FollowBeaconsFollowingTaskPreparation {
                target_id: 0x047F_0001,
                task: FOLLOW_BEACONS_FOLLOWING_TASK,
            },
            0x0497_0001,
            [100, 200, 300],
            &type17_run_away_metadata(),
        )
        .unwrap();
        let random_draws = Cell::new(0);
        let mut effects = Vec::new();
        let task = prepared.apply_suffix(
            || {
                random_draws.set(random_draws.get() + 1);
                0xABCD_7F00
            },
            |effect| effects.push(effect),
        );

        assert_eq!(random_draws.get(), 1);
        assert_eq!(
            &effects[..3],
            &[
                FollowBeaconsFollowingConstructorEffect::Generic(
                    SharedGenericConstructorEffect::WriteSubHState08 { value: 1 },
                ),
                FollowBeaconsFollowingConstructorEffect::Generic(
                    SharedGenericConstructorEffect::WriteSubADirection {
                        direction_multiplier: 1,
                    },
                ),
                FollowBeaconsFollowingConstructorEffect::Generic(
                    SharedGenericConstructorEffect::WriteSubATargetSpeed {
                        target_speed_raw: 262,
                        random_sample_low16: 0x7F00,
                    },
                ),
            ]
        );
        let FollowBeaconsFollowingConstructorEffect::ApplyFixedSubATargetSpeed { owner_id, suffix } =
            effects[3]
        else {
            panic!("fixed-speed overwrite was not last")
        };
        assert_eq!(owner_id, 0x0497_0001);
        assert_eq!(suffix.sub_a_target_speed_raw(), Some(333));
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, task);
        let Some(ActorTaskRuntime::FollowBeaconsFollowing(state)) =
            owner.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("prepared task changed family")
        };
        assert_eq!(state.target_id(), 0x047F_0001);
    }

    #[test]
    fn following_mover_stage_commits_on_resolved_and_post_mover_paths_only() {
        let mut resolved_owner = ActorTaskOwner::new();
        resolved_owner.replace_prepared(ActorTaskSlot::Primary, follow_beacons_following_task());
        let mut resolved_adapter = TestAdapter::new(Mode::FollowFollowingContinue);
        assert_eq!(
            tick_actor_task_dispatcher(
                &mut resolved_owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut resolved_adapter,
            ),
            Ok(None)
        );
        let Some(ActorTaskRuntime::FollowBeaconsFollowing(resolved)) =
            resolved_owner.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("resolved task changed family")
        };
        assert_eq!(resolved.private_state().direction, -1);
        assert_eq!(resolved.elapsed_ms(), 1);

        let mut post_mover_owner = ActorTaskOwner::new();
        post_mover_owner.replace_prepared(ActorTaskSlot::Primary, follow_beacons_following_task());
        let mut post_mover_adapter = TestAdapter::new(Mode::FollowFollowingPostMoverError);
        assert!(matches!(
            tick_actor_task_dispatcher(
                &mut post_mover_owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut post_mover_adapter,
            ),
            Err(ActorTaskDispatcherError::FollowBeaconsFollowingCallback {
                error: FollowBeaconsFollowingAdapterError::AfterCommonMover(
                    "unresolved Follow Beacons post-mover lookup"
                ),
                ..
            })
        ));
        let Some(ActorTaskRuntime::FollowBeaconsFollowing(post_mover)) =
            post_mover_owner.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("post-mover-error task changed family")
        };
        assert_eq!(post_mover.private_state().direction, -1);
        assert_eq!(post_mover.elapsed_ms(), 1);

        let mut mover_error_owner = ActorTaskOwner::new();
        mover_error_owner.replace_prepared(ActorTaskSlot::Primary, follow_beacons_following_task());
        let mut mover_error_adapter = TestAdapter::new(Mode::FollowFollowingMoverError);
        assert!(matches!(
            tick_actor_task_dispatcher(
                &mut mover_error_owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut mover_error_adapter,
            ),
            Err(ActorTaskDispatcherError::FollowBeaconsFollowingCallback {
                error: FollowBeaconsFollowingAdapterError::CommonMover(
                    "unresolved Follow Beacons common mover"
                ),
                ..
            })
        ));
        let Some(ActorTaskRuntime::FollowBeaconsFollowing(mover_error)) =
            mover_error_owner.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("mover-error task changed family")
        };
        assert_eq!(mover_error.private_state().direction, 1);
        assert_eq!(mover_error.elapsed_ms(), 1);
    }

    #[test]
    fn following_tag_preempts_expired_lifetime_and_raw_other_tag_propagates() {
        let mut tagged_owner = ActorTaskOwner::new();
        tagged_owner.replace_prepared(ActorTaskSlot::Primary, follow_beacons_following_task());
        let mut tagged_adapter = TestAdapter::new(Mode::FollowFollowingTagged);
        assert_eq!(
            tick_actor_task_dispatcher(
                &mut tagged_owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 9_001_000,
                    scheduler_mode: 0xA5,
                },
                &mut tagged_adapter,
            ),
            Ok(None)
        );
        assert_eq!(tagged_adapter.follow_following_transitions.len(), 1);
        assert_eq!(
            tagged_adapter.follow_following_transitions[0].reason,
            FollowBeaconsFollowingTransitionReason::TaggedCallbackResult(
                FollowBeaconsFollowingTaggedSingleton::RouteRejected,
            )
        );
        assert_eq!(
            tagged_adapter.follow_following_transitions[0]
                .committed_prefix
                .lifetime_status,
            FollowBeaconsFollowingLifetimeStatus::OwnerTransitionDue
        );

        let mut propagated_owner = ActorTaskOwner::new();
        propagated_owner.replace_prepared(ActorTaskSlot::Primary, follow_beacons_following_task());
        propagated_owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let mut propagated_adapter = TestAdapter::new(Mode::FollowFollowingPropagate);
        assert_eq!(
            tick_actor_task_dispatcher(
                &mut propagated_owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 9_001_000,
                    scheduler_mode: 0xA5,
                },
                &mut propagated_adapter,
            ),
            Ok(Some("Follow Beacons following propagated"))
        );
        assert_eq!(propagated_adapter.follow_following_style_results.len(), 1);
        assert!(propagated_adapter.follow_following_transitions.is_empty());
        assert!(propagated_adapter.visited_wander.is_empty());
    }

    #[test]
    fn following_generic_tag_gate_distinguishes_absence_and_bit_suppression() {
        for (mode, tagged, expected_reasons) in [
            (
                Mode::FollowFollowingRouteCallbackAbsent,
                FollowBeaconsFollowingTaggedSingleton::RouteRejected,
                vec![
                    FollowBeaconsFollowingTransitionReason::TaggedCallbackResult(
                        FollowBeaconsFollowingTaggedSingleton::RouteRejected,
                    ),
                    FollowBeaconsFollowingTransitionReason::LifetimeExpired,
                ],
            ),
            (
                Mode::FollowFollowingRouteSuppressed,
                FollowBeaconsFollowingTaggedSingleton::RouteRejected,
                vec![
                    FollowBeaconsFollowingTransitionReason::TaggedCallbackResult(
                        FollowBeaconsFollowingTaggedSingleton::RouteRejected,
                    ),
                ],
            ),
            (
                Mode::FollowFollowingReachedSuppressed,
                FollowBeaconsFollowingTaggedSingleton::ReachedZeroCallback,
                vec![
                    FollowBeaconsFollowingTransitionReason::TaggedCallbackResult(
                        FollowBeaconsFollowingTaggedSingleton::ReachedZeroCallback,
                    ),
                    FollowBeaconsFollowingTransitionReason::LifetimeExpired,
                ],
            ),
        ] {
            let mut owner = ActorTaskOwner::new();
            owner.replace_prepared(ActorTaskSlot::Primary, follow_beacons_following_task());
            let mut adapter = TestAdapter::new(mode);
            assert_eq!(
                tick_actor_task_dispatcher(
                    &mut owner,
                    ActorTaskDispatcherFrame {
                        elapsed_micros: 9_001_000,
                        scheduler_mode: 0xA5,
                    },
                    &mut adapter,
                ),
                Ok(None),
                "unexpected scheduler result for {mode:?} / {tagged:?}"
            );
            assert_eq!(
                adapter
                    .follow_following_transitions
                    .iter()
                    .map(|request| request.reason)
                    .collect::<Vec<_>>(),
                expected_reasons,
            );
        }
    }

    #[test]
    fn following_style_result_lifetime_fallthrough_uses_strict_timeout() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, follow_beacons_following_task());
        let mut adapter = TestAdapter::new(Mode::FollowFollowingStyleFallThrough);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 9_001_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert_eq!(adapter.follow_following_style_results.len(), 1);
        assert_eq!(
            adapter
                .follow_following_transitions
                .iter()
                .map(|request| request.reason)
                .collect::<Vec<_>>(),
            [FollowBeaconsFollowingTransitionReason::LifetimeExpired],
        );
    }

    #[test]
    fn following_self_replacement_discards_result_error_and_timeout() {
        for mode in [
            Mode::ReplaceFollowFollowingTagged,
            Mode::ReplaceFollowFollowingPostMoverError,
            Mode::ReplaceFollowFollowingContinue,
        ] {
            let mut owner = ActorTaskOwner::new();
            owner.replace_prepared(ActorTaskSlot::Primary, follow_beacons_following_task());
            owner.replace_prepared(
                ActorTaskSlot::Secondary,
                PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
            );
            let mut adapter = TestAdapter::new(mode);

            assert_eq!(
                tick_actor_task_dispatcher(
                    &mut owner,
                    ActorTaskDispatcherFrame {
                        elapsed_micros: 9_001_000,
                        scheduler_mode: 0xA5,
                    },
                    &mut adapter,
                ),
                Ok(None),
                "retired callback outcome must be discarded for {mode:?}"
            );
            assert!(matches!(
                owner.state_in_slot(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::OrdinaryType9Wander(_))
            ));
            assert!(adapter.follow_following_transitions.is_empty());
            assert!(adapter.follow_following_style_results.is_empty());
            assert_eq!(adapter.visited_wander.len(), 1);
        }
    }

    #[test]
    fn following_transition_observes_fresh_replaced_later_slot() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, follow_beacons_following_task());
        let old_secondary = owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let mut adapter = TestAdapter::new(Mode::ReplaceSecondaryFromFollowFollowing);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert_eq!(adapter.visited_wander.len(), 1);
        assert_ne!(adapter.visited_wander[0], old_secondary);
        assert_eq!(
            adapter.events,
            [
                "Follow Beacons following callback",
                "Follow Beacons following transition",
                "wander callback",
            ]
        );
    }

    #[test]
    fn guard_acquisition_runtime_preparation_accepts_only_the_exact_slot_one_phase() {
        let task = crate::guard_location_owner::GuardLocationTaskSpec {
            role: GuardLocationTaskRole::AcquireCandidate,
            slot: ActorTaskSlot::Secondary,
            constructor_address: GUARD_LOCATION_SEARCH_CONSTRUCTOR_ADDRESS,
            tick_address: GUARD_LOCATION_SEARCH_TICK_ADDRESS,
        };
        let prepared =
            prepare_guard_location_acquisition_runtime_task(GuardLocationTaskPreparation {
                phase_index: 0,
                task,
                constructor_inputs: GuardLocationTaskConstructorInputs::AcquireCandidate {
                    fixed_argument: GUARD_LOCATION_SEARCH_FIXED_ARGUMENT,
                    search_task_context_word: 0x40,
                },
            })
            .unwrap();
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Secondary, prepared);

        let Some(ActorTaskRuntime::GuardLocationAcquisition(state)) =
            owner.state_in_slot(ActorTaskSlot::Secondary)
        else {
            panic!("secondary task changed family")
        };
        assert_eq!(state.constructor_filter_override_raw(), 0x40);

        let error = prepare_guard_location_acquisition_runtime_task(GuardLocationTaskPreparation {
            phase_index: 1,
            task,
            constructor_inputs: GuardLocationTaskConstructorInputs::AcquireCandidate {
                fixed_argument: GUARD_LOCATION_SEARCH_FIXED_ARGUMENT,
                search_task_context_word: 0x40,
            },
        })
        .unwrap_err();
        assert_eq!(
            error,
            GuardLocationAcquisitionRuntimePreparationError::TaskContractMismatch
        );
    }

    #[test]
    fn guard_acquisition_draws_once_and_rejected_gate_preserves_context() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            guard_location_acquisition_task(0x40),
        );
        let mut adapter = TestAdapter::new(Mode::GuardChanceRejected);
        adapter.random = VecDeque::from([3]);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert_eq!(adapter.random_draws, 1);
        assert_eq!(
            adapter.guard_location_prefixes,
            [GuardLocationAcquisitionCallbackPrefix::ChanceRejected { random_low16: 3 }]
        );
        assert_eq!(
            adapter.guard_location_context.filter(),
            SearchAttackCandidateFilter::SameEntityType
        );
    }

    #[test]
    fn accepted_guard_prefix_write_survives_candidate_callback_error() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            guard_location_acquisition_task(0x40),
        );
        let mut adapter = TestAdapter::new(Mode::GuardCallbackError);
        adapter.random = VecDeque::from([4]);

        let result = tick_actor_task_dispatcher(
            &mut owner,
            ActorTaskDispatcherFrame {
                elapsed_micros: 1_000,
                scheduler_mode: 0xA5,
            },
            &mut adapter,
        );

        let expected_filter =
            SearchAttackCandidateFilter::CapabilityMask(std::num::NonZeroU32::new(0x40).unwrap());
        assert_eq!(adapter.random_draws, 1);
        assert_eq!(adapter.guard_location_context.filter(), expected_filter);
        assert!(matches!(
            result,
            Err(
                ActorTaskDispatcherError::GuardLocationAcquisitionCallback {
                    visit: ActorTaskVisit {
                        slot: ActorTaskSlot::Secondary,
                        ..
                    },
                    committed_prefix: GuardLocationAcquisitionCallbackPrefix::Acquire {
                        random_low16: 4,
                        filter_write: Some(filter),
                        ..
                    },
                    error: "unresolved Guard candidate snapshot",
                }
            ) if filter == expected_filter
        ));
    }

    #[test]
    fn guard_candidate_tag_is_consumed_after_unwind_without_owner_transition() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Secondary, guard_location_acquisition_task(0));
        let mut adapter = TestAdapter::new(Mode::GuardTagged);
        adapter.random = VecDeque::from([0]);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert_eq!(
            adapter.events,
            ["guard acquisition prefix", "guard acquisition callback"]
        );
        assert!(adapter.visited_wander.is_empty());
    }

    #[test]
    fn surviving_guard_acquisition_propagates_nonzero_behavior_result_unchanged() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Secondary, guard_location_acquisition_task(0));
        let mut adapter = TestAdapter::new(Mode::GuardPropagate);
        adapter.random = VecDeque::from([0]);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(Some("guard acquisition propagated"))
        );
    }

    #[test]
    fn guard_handoff_self_replacement_discards_error_and_visits_new_tertiary() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Secondary, guard_location_acquisition_task(0));
        let mut adapter = TestAdapter::new(Mode::ReplaceGuardWithTertiary);
        adapter.random = VecDeque::from([0, 1]);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert_eq!(owner.state_in_slot(ActorTaskSlot::Secondary), None);
        assert!(matches!(
            owner.state_in_slot(ActorTaskSlot::Tertiary),
            Some(ActorTaskRuntime::OrdinaryType9Wander(_))
        ));
        assert_eq!(
            adapter.events,
            [
                "guard acquisition prefix",
                "guard acquisition callback",
                "wander callback"
            ]
        );
        assert_eq!(adapter.visited_wander.len(), 1);
    }

    #[test]
    fn unresolved_guard_prefix_fails_closed_after_consuming_exactly_one_rng_draw() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Secondary, guard_location_acquisition_task(0));
        let mut adapter = TestAdapter::new(Mode::GuardPrefixError);
        adapter.random = VecDeque::from([0]);

        let result = tick_actor_task_dispatcher(
            &mut owner,
            ActorTaskDispatcherFrame {
                elapsed_micros: 1_000,
                scheduler_mode: 0xA5,
            },
            &mut adapter,
        );

        assert_eq!(adapter.random_draws, 1);
        assert!(matches!(
            result,
            Err(ActorTaskDispatcherError::GuardLocationAcquisitionPrefix {
                visit: ActorTaskVisit {
                    slot: ActorTaskSlot::Secondary,
                    ..
                },
                error: "unresolved Guard context",
            })
        ));
        assert_eq!(adapter.events, ["guard acquisition prefix"]);
    }

    #[test]
    fn run_away_runtime_preparation_accepts_only_the_exact_flee_phase() {
        let setup =
            crate::run_away::run_away_variant_setup(crate::run_away::RunAwayVariant::Fleeing);
        let request = RunAwayTaskSetupRequest::Fleeing {
            target_id: 0x047F_0001,
            audio: crate::run_away::RunAwayAuthoredAudio {
                sound_id: 0x55,
                period_raw: 0,
            },
        };
        let metadata = type9_run_away_suffix_metadata();
        let prepared = prepare_run_away_runtime_task(
            RunAwayTaskPreparation {
                request,
                phase_index: 0,
                task: setup.ordered_phases[0].install,
            },
            0x0497_0001,
            [100, 200, 300],
            &metadata,
        )
        .unwrap();
        let mut effects = Vec::new();
        let random_draws = Cell::new(0);
        let task = prepared.apply_suffix(
            || {
                random_draws.set(random_draws.get() + 1);
                0xD2F6
            },
            |effect| effects.push(effect),
        );
        assert_eq!(random_draws.get(), 1);
        assert_eq!(effects.len(), 3);
        assert_eq!(
            effects[0],
            RunAwayRuntimeConstructorEffect::Generic(
                SharedGenericConstructorEffect::WriteSubADirection {
                    direction_multiplier: 1,
                },
            )
        );
        assert_eq!(
            effects[1],
            RunAwayRuntimeConstructorEffect::Generic(
                SharedGenericConstructorEffect::WriteSubATargetSpeed {
                    target_speed_raw: crate::common_mover::shared_initializer_target_speed_raw(
                        250, 0xD2F6
                    ),
                    random_sample_low16: 0xD2F6,
                },
            )
        );
        let RunAwayRuntimeConstructorEffect::ApplyFixedSubATargetSpeed { owner_id, suffix } =
            effects[2]
        else {
            panic!("fixed Run Away overwrite must follow the generic suffix")
        };
        assert_eq!(owner_id, 0x0497_0001);
        assert_eq!(suffix.sub_a_target_speed_raw(), Some(416));
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, task);

        let Some(ActorTaskRuntime::RunAway(state)) = owner.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("primary task changed family")
        };
        assert_eq!(state.target_id(), 0x047F_0001);
        assert_eq!(state.audio().sound_id, 0x55);

        let error = prepare_run_away_runtime_task(
            RunAwayTaskPreparation {
                request,
                phase_index: 1,
                task: setup.ordered_phases[0].install,
            },
            0x0497_0001,
            [100, 200, 300],
            &metadata,
        )
        .unwrap_err();
        assert_eq!(error, RunAwayRuntimePreparationError::TaskContractMismatch);
    }

    #[test]
    fn acquiring_setup_applies_one_ordered_shared_suffix_per_successful_phase() {
        let metadata = type17_run_away_metadata();
        let mut owner = ActorTaskOwner::new();
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            owner.replace_prepared(
                slot,
                PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
            );
        }
        let draws = Cell::new(0);
        let mut random = VecDeque::from([0x0000_D2F6, 0x0000_0985]);
        let mut effects = Vec::new();

        apply_run_away_task_setup(
            &mut owner,
            RunAwayTaskSetupRequest::Acquiring,
            |preparation| {
                let prepared = prepare_shared_acquiring_runtime_task(
                    preparation,
                    [100, 200, 300],
                    &metadata,
                    0,
                )?;
                Ok::<_, SharedAcquiringRuntimePreparationError>(prepared.apply_suffix(
                    || {
                        draws.set(draws.get() + 1);
                        random.pop_front().expect("one draw per acquiring phase")
                    },
                    |effect| effects.push(effect),
                ))
            },
        )
        .unwrap();

        assert_eq!(draws.get(), 2);
        assert!(random.is_empty());
        assert_eq!(
            effects,
            [
                SharedGenericConstructorEffect::WriteSubHState08 { value: 1 },
                SharedGenericConstructorEffect::WriteSubADirection {
                    direction_multiplier: 1
                },
                SharedGenericConstructorEffect::WriteSubATargetSpeed {
                    target_speed_raw: 270,
                    random_sample_low16: 0xD2F6,
                },
                SharedGenericConstructorEffect::WriteSubHState08 { value: 1 },
                SharedGenericConstructorEffect::WriteSubADirection {
                    direction_multiplier: 1
                },
                SharedGenericConstructorEffect::WriteSubATargetSpeed {
                    target_speed_raw: 250,
                    random_sample_low16: 0x0985,
                },
            ]
        );
        assert!(owner.task_in_slot(ActorTaskSlot::Tertiary).is_none());
        let Some(ActorTaskRuntime::TargetAcquisition(acquisition)) =
            owner.state_in_slot(ActorTaskSlot::Secondary)
        else {
            panic!("secondary task changed family")
        };
        assert_eq!(
            acquisition.radius(),
            SearchAttackRadius::strict(2_560).unwrap()
        );
        assert_eq!(
            acquisition.filter(),
            SearchAttackCandidateFilter::CapabilityMask(std::num::NonZeroU32::new(3).unwrap())
        );
        assert_eq!(acquisition.constructor_filter_override_raw(), 0);
        let Some(ActorTaskRuntime::SharedRetarget(wander)) =
            owner.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("primary task changed family")
        };
        assert_eq!(
            wander.private_state(),
            WanderNearPrivateState::ordinary_type9([100, 200, 300])
        );
        assert_eq!(wander.elapsed_ms(), 0);
        assert_eq!(wander.lifetime_ms(), RUN_AWAY_ACQUIRING_WANDER_LIFETIME_MS);
    }

    #[test]
    fn detached_type9_run_away_acquiring_composes_both_sub_a_only_phases() {
        let metadata = type9_run_away_suffix_metadata();
        let initializer = metadata.initializer.as_ref().unwrap();
        assert_eq!(initializer.initializer_state_flags_raw, 0x2F);
        assert_eq!(
            initializer.common_axis_descriptor.strict_axis_limit_raw,
            0x0F00
        );
        assert_eq!(initializer.common_axis_descriptor.raw_word_at_0x04, 0x84);

        let mut owner = ActorTaskOwner::new();
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            owner.replace_prepared(
                slot,
                PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
            );
        }
        let draws = Cell::new(0);
        let mut random = VecDeque::from([0x0000_D2F6, 0x0000_0985]);
        let mut topologies = Vec::new();
        let mut effects = Vec::new();

        // This detached seam begins at task preparation. Selected-initializer
        // context/filter publication and the actor common-axis copy remain a
        // separate live-adapter checkpoint.
        apply_run_away_task_setup(
            &mut owner,
            RunAwayTaskSetupRequest::Acquiring,
            |preparation| {
                let prepared = prepare_shared_acquiring_runtime_task(
                    preparation,
                    [100, 200, 300],
                    &metadata,
                    0,
                )?;
                topologies.push(prepared.topology());
                Ok::<_, SharedAcquiringRuntimePreparationError>(prepared.apply_suffix(
                    || {
                        draws.set(draws.get() + 1);
                        random.pop_front().expect("one draw per acquiring phase")
                    },
                    |effect| effects.push(effect),
                ))
            },
        )
        .unwrap();

        assert_eq!(
            topologies,
            [
                SharedGenericConstructorTopology::SubAOnly,
                SharedGenericConstructorTopology::SubAOnly,
            ]
        );
        assert_eq!(draws.get(), 2);
        assert!(random.is_empty());
        assert_eq!(
            effects,
            [
                SharedGenericConstructorEffect::WriteSubADirection {
                    direction_multiplier: 1,
                },
                SharedGenericConstructorEffect::WriteSubATargetSpeed {
                    target_speed_raw: 270,
                    random_sample_low16: 0xD2F6,
                },
                SharedGenericConstructorEffect::WriteSubADirection {
                    direction_multiplier: 1,
                },
                SharedGenericConstructorEffect::WriteSubATargetSpeed {
                    target_speed_raw: 250,
                    random_sample_low16: 0x0985,
                },
            ]
        );
        assert!(effects.iter().all(|effect| !matches!(
            effect,
            SharedGenericConstructorEffect::WriteSubHState08 { .. }
        )));
        assert_eq!(owner.task_in_slot(ActorTaskSlot::Tertiary), None);
        let Some(ActorTaskRuntime::TargetAcquisition(acquisition)) =
            owner.state_in_slot(ActorTaskSlot::Secondary)
        else {
            panic!("secondary task changed family")
        };
        assert_eq!(
            acquisition.radius(),
            SearchAttackRadius::strict(0x0F00).unwrap()
        );
        assert_eq!(
            acquisition.filter(),
            SearchAttackCandidateFilter::CapabilityMask(std::num::NonZeroU32::new(0x84).unwrap())
        );
        assert_eq!(acquisition.constructor_filter_override_raw(), 0);
        let Some(ActorTaskRuntime::SharedRetarget(retarget)) =
            owner.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("primary task changed family")
        };
        assert_eq!(
            retarget.private_state(),
            WanderNearPrivateState::ordinary_type9([100, 200, 300])
        );
        assert_eq!(retarget.elapsed_ms(), 0);
        assert_eq!(retarget.lifetime_ms(), 500);
    }

    #[test]
    fn follow_beacons_acquiring_setup_uses_distinct_ranked_family_and_generic_suffix() {
        let metadata = type17_run_away_metadata();
        let mut owner = ActorTaskOwner::new();
        let draws = Cell::new(0);
        let mut random = VecDeque::from([0x0000_D2F6, 0x0000_0985]);
        let mut effects = Vec::new();

        apply_follow_beacons_acquiring_task_setup(&mut owner, |preparation| {
            let prepared = prepare_follow_beacons_acquiring_runtime_task(
                preparation,
                [100, 200, 300],
                &metadata,
            )?;
            Ok::<_, FollowBeaconsAcquiringRuntimePreparationError>(prepared.apply_suffix(
                || {
                    draws.set(draws.get() + 1);
                    random
                        .pop_front()
                        .expect("one shared RNG draw per successful phase")
                },
                |effect| effects.push(effect),
            ))
        })
        .unwrap();

        assert_eq!(draws.get(), 2);
        assert!(random.is_empty());
        assert_eq!(
            effects,
            [
                SharedGenericConstructorEffect::WriteSubHState08 { value: 1 },
                SharedGenericConstructorEffect::WriteSubADirection {
                    direction_multiplier: 1,
                },
                SharedGenericConstructorEffect::WriteSubATargetSpeed {
                    target_speed_raw: 270,
                    random_sample_low16: 0xD2F6,
                },
                SharedGenericConstructorEffect::WriteSubHState08 { value: 1 },
                SharedGenericConstructorEffect::WriteSubADirection {
                    direction_multiplier: 1,
                },
                SharedGenericConstructorEffect::WriteSubATargetSpeed {
                    target_speed_raw: 250,
                    random_sample_low16: 0x0985,
                },
            ]
        );
        assert!(owner.task_in_slot(ActorTaskSlot::Tertiary).is_none());
        let Some(ActorTaskRuntime::FollowBeaconAcquisition(acquisition)) =
            owner.state_in_slot(ActorTaskSlot::Secondary)
        else {
            panic!("Follow Beacons phase zero changed family")
        };
        assert_eq!(
            acquisition.route_range(),
            WrappedAxisRange::strict(2_560).unwrap()
        );
        assert_eq!(acquisition.filter_raw(), 3);
        let Some(ActorTaskRuntime::SharedRetarget(retarget)) =
            owner.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("Follow Beacons phase one changed family")
        };
        assert_eq!(
            retarget.private_state(),
            WanderNearPrivateState::ordinary_type9([100, 200, 300])
        );
        assert_eq!(
            retarget.lifetime_ms(),
            crate::follow_beacons::FOLLOW_BEACONS_ACQUIRING_RETARGET_LIFETIME_MS
        );
    }

    #[test]
    fn follow_beacons_setup_failure_matrix_preserves_published_prefix_and_old_destinations() {
        let valid_metadata = type17_run_away_metadata();
        let mut blocked_metadata = type17_run_away_metadata();
        let RetailRuntimeValue::Known(mut blocked_topology) =
            blocked_metadata.common_mover_topology
        else {
            unreachable!()
        };
        blocked_topology.sub_g = true;
        blocked_metadata.common_mover_topology = RetailRuntimeValue::Known(blocked_topology);

        // Phase zero cannot consume constructor RNG/effects or retire either
        // destination when its shared suffix cannot be authenticated.
        let mut phase_zero_owner = ActorTaskOwner::new();
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            phase_zero_owner.replace_prepared(
                slot,
                PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
            );
        }
        let old_zero_primary = phase_zero_owner.task_in_slot(ActorTaskSlot::Primary);
        let old_zero_secondary = phase_zero_owner.task_in_slot(ActorTaskSlot::Secondary);
        let phase_zero_draws = Cell::new(0);
        let mut phase_zero_effects = Vec::new();
        let phase_zero_error =
            apply_follow_beacons_acquiring_task_setup(&mut phase_zero_owner, |preparation| {
                let prepared = prepare_follow_beacons_acquiring_runtime_task(
                    preparation,
                    [100, 200, 300],
                    &blocked_metadata,
                )?;
                Ok::<_, FollowBeaconsAcquiringRuntimePreparationError>(prepared.apply_suffix(
                    || {
                        phase_zero_draws.set(phase_zero_draws.get() + 1);
                        0
                    },
                    |effect| phase_zero_effects.push(effect),
                ))
            })
            .unwrap_err();
        assert_eq!(phase_zero_error.phase_index, 0);
        assert_eq!(
            phase_zero_error.error,
            FollowBeaconsAcquiringRuntimePreparationError::ConstructorSuffix(
                SharedGenericConstructorSuffixError::UnsupportedSharedConstructorTopology,
            )
        );
        assert_eq!(phase_zero_draws.get(), 0);
        assert!(phase_zero_effects.is_empty());
        assert_eq!(
            phase_zero_owner.task_in_slot(ActorTaskSlot::Secondary),
            old_zero_secondary
        );
        assert_eq!(
            phase_zero_owner.task_in_slot(ActorTaskSlot::Primary),
            old_zero_primary
        );
        assert_eq!(phase_zero_owner.task_in_slot(ActorTaskSlot::Tertiary), None);

        // A phase-one block occurs after phase zero's complete suffix and
        // publication, but before the old Primary is retired.
        let mut phase_one_owner = ActorTaskOwner::new();
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            phase_one_owner.replace_prepared(
                slot,
                PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
            );
        }
        let old_one_primary = phase_one_owner.task_in_slot(ActorTaskSlot::Primary);
        let old_one_secondary = phase_one_owner.task_in_slot(ActorTaskSlot::Secondary);
        let phase_one_draws = Cell::new(0);
        let mut phase_one_effects = Vec::new();
        let phase_one_error =
            apply_follow_beacons_acquiring_task_setup(&mut phase_one_owner, |preparation| {
                let metadata = if preparation.phase_index == 0 {
                    &valid_metadata
                } else {
                    &blocked_metadata
                };
                let prepared = prepare_follow_beacons_acquiring_runtime_task(
                    preparation,
                    [100, 200, 300],
                    metadata,
                )?;
                Ok::<_, FollowBeaconsAcquiringRuntimePreparationError>(prepared.apply_suffix(
                    || {
                        phase_one_draws.set(phase_one_draws.get() + 1);
                        0x0000_D2F6
                    },
                    |effect| phase_one_effects.push(effect),
                ))
            })
            .unwrap_err();
        assert_eq!(phase_one_error.phase_index, 1);
        assert_eq!(
            phase_one_error.error,
            FollowBeaconsAcquiringRuntimePreparationError::ConstructorSuffix(
                SharedGenericConstructorSuffixError::UnsupportedSharedConstructorTopology,
            )
        );
        assert_eq!(phase_one_draws.get(), 1);
        assert_eq!(
            phase_one_effects,
            [
                SharedGenericConstructorEffect::WriteSubHState08 { value: 1 },
                SharedGenericConstructorEffect::WriteSubADirection {
                    direction_multiplier: 1,
                },
                SharedGenericConstructorEffect::WriteSubATargetSpeed {
                    target_speed_raw: 270,
                    random_sample_low16: 0xD2F6,
                },
            ]
        );
        assert!(matches!(
            phase_one_owner.state_in_slot(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::FollowBeaconAcquisition(_))
        ));
        assert_ne!(
            phase_one_owner.task_in_slot(ActorTaskSlot::Secondary),
            old_one_secondary
        );
        assert_eq!(
            phase_one_owner.task_in_slot(ActorTaskSlot::Primary),
            old_one_primary
        );
        assert_eq!(phase_one_owner.task_in_slot(ActorTaskSlot::Tertiary), None);
    }

    #[test]
    fn follow_beacons_preparer_rejects_missing_initializer_and_wrong_phase_contract() {
        let setup = crate::follow_beacons::follow_beacons_acquiring_setup();
        let mut metadata = type17_run_away_metadata();
        metadata.initializer = None;
        assert_eq!(
            prepare_follow_beacons_acquiring_runtime_task(
                FollowBeaconsTaskPreparation {
                    phase_index: 0,
                    task: setup.ordered_phases[0].install,
                },
                [100, 200, 300],
                &metadata,
            )
            .unwrap_err(),
            FollowBeaconsAcquiringRuntimePreparationError::MissingInitializer
        );

        let metadata = type17_run_away_metadata();
        assert_eq!(
            prepare_follow_beacons_acquiring_runtime_task(
                FollowBeaconsTaskPreparation {
                    phase_index: 1,
                    task: setup.ordered_phases[0].install,
                },
                [100, 200, 300],
                &metadata,
            )
            .unwrap_err(),
            FollowBeaconsAcquiringRuntimePreparationError::TaskContractMismatch
        );
    }

    #[test]
    fn shared_generic_suffix_fails_closed_for_each_unauthenticated_metadata_shape() {
        let cases = [
            (
                RetailRuntimeValue::Unresolved,
                RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                    acceleration_raw: 1,
                    overspeed_correction_raw: 2,
                    target_speed_base_raw: 3,
                })),
                SharedGenericConstructorSuffixError::UnresolvedComponentTopology,
            ),
            (
                RetailRuntimeValue::Known(CommonMoverComponentTopology {
                    sub_a: true,
                    sub_f: true,
                    sub_h: true,
                    ..CommonMoverComponentTopology::default()
                }),
                RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                    acceleration_raw: 1,
                    overspeed_correction_raw: 2,
                    target_speed_base_raw: 3,
                })),
                SharedGenericConstructorSuffixError::UnsupportedSharedConstructorTopology,
            ),
            (
                RetailRuntimeValue::Known(CommonMoverComponentTopology {
                    sub_a: true,
                    sub_g: true,
                    ..CommonMoverComponentTopology::default()
                }),
                RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                    acceleration_raw: 1,
                    overspeed_correction_raw: 2,
                    target_speed_base_raw: 3,
                })),
                SharedGenericConstructorSuffixError::UnsupportedSharedConstructorTopology,
            ),
            (
                RetailRuntimeValue::Known(CommonMoverComponentTopology::default()),
                RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                    acceleration_raw: 1,
                    overspeed_correction_raw: 2,
                    target_speed_base_raw: 3,
                })),
                SharedGenericConstructorSuffixError::UnsupportedSharedConstructorTopology,
            ),
            (
                RetailRuntimeValue::Known(CommonMoverComponentTopology {
                    sub_a: true,
                    sub_h: true,
                    ..CommonMoverComponentTopology::default()
                }),
                RetailRuntimeValue::Unresolved,
                SharedGenericConstructorSuffixError::UnresolvedSubADescriptor,
            ),
            (
                RetailRuntimeValue::Known(CommonMoverComponentTopology {
                    sub_a: true,
                    sub_h: true,
                    ..CommonMoverComponentTopology::default()
                }),
                RetailRuntimeValue::Known(None),
                SharedGenericConstructorSuffixError::MissingSubADescriptor,
            ),
        ];

        for (common_mover_topology, sub_a_propulsion_descriptor, expected) in cases {
            let metadata = EntityTypeRuntimeMetadata {
                common_mover_topology,
                sub_a_propulsion_descriptor,
                ..EntityTypeRuntimeMetadata::default()
            };
            let error = prepare_shared_generic_constructor_suffix(
                follow_beacon_acquisition_task(0),
                &metadata,
            )
            .unwrap_err();
            assert_eq!(error, expected);
        }

        let sub_a_only = prepare_shared_generic_constructor_suffix(
            follow_beacon_acquisition_task(0),
            &type9_run_away_suffix_metadata(),
        )
        .expect("the exact type-9 A/B/D/I graph closes the Sub-A-only branch");
        assert_eq!(
            sub_a_only.topology(),
            SharedGenericConstructorTopology::SubAOnly
        );

        let sub_h_then_sub_a = prepare_shared_generic_constructor_suffix(
            follow_beacon_acquisition_task(0),
            &type17_run_away_metadata(),
        )
        .expect("the authenticated type-17 graph closes the Sub-H/Sub-A branch");
        assert_eq!(
            sub_h_then_sub_a.topology(),
            SharedGenericConstructorTopology::SubHThenSubA
        );
    }

    #[test]
    fn detached_type9_acquiring_suffixes_precede_replacement_and_preserve_phase_order() {
        #[derive(Debug)]
        enum PublicationProbeTask {
            Old {
                retired: Rc<Cell<bool>>,
            },
            New {
                phase_index: usize,
                _runtime: ActorTaskRuntime,
            },
        }

        impl Drop for PublicationProbeTask {
            fn drop(&mut self) {
                if let Self::Old { retired } = self {
                    retired.set(true);
                }
            }
        }

        let metadata = type9_run_away_suffix_metadata();
        let primary_retired = Rc::new(Cell::new(false));
        let secondary_retired = Rc::new(Cell::new(false));
        let tertiary_retired = Rc::new(Cell::new(false));
        let mut owner = ActorTaskOwner::new();
        for (slot, retired) in [
            (ActorTaskSlot::Primary, Rc::clone(&primary_retired)),
            (ActorTaskSlot::Secondary, Rc::clone(&secondary_retired)),
            (ActorTaskSlot::Tertiary, Rc::clone(&tertiary_retired)),
        ] {
            owner.replace_prepared(
                slot,
                PreparedActorTask::new(PublicationProbeTask::Old { retired }),
            );
        }
        let mut effect_phases = Vec::new();
        let mut random_receipts = Vec::new();
        let mut ordered_effects = Vec::new();

        apply_run_away_task_setup(
            &mut owner,
            RunAwayTaskSetupRequest::Acquiring,
            |preparation| {
                let phase_index = preparation.phase_index;
                let prepared = prepare_shared_acquiring_runtime_task(
                    preparation,
                    [100, 200, 300],
                    &metadata,
                    0,
                )?;
                assert_eq!(
                    prepared.topology(),
                    SharedGenericConstructorTopology::SubAOnly
                );
                let task = prepared.apply_suffix(
                    || {
                        let sample = if phase_index == 0 { 0xD2F6 } else { 0x0985 };
                        random_receipts.push((phase_index, sample as u16));
                        sample
                    },
                    |effect| {
                        effect_phases.push(phase_index);
                        ordered_effects.push((phase_index, effect));
                        assert!(tertiary_retired.get());
                        if phase_index == 0 {
                            assert!(!secondary_retired.get());
                            assert!(!primary_retired.get());
                        } else {
                            assert!(secondary_retired.get());
                            assert!(!primary_retired.get());
                        }
                    },
                );
                Ok::<_, SharedAcquiringRuntimePreparationError>(task.map(|runtime| {
                    PublicationProbeTask::New {
                        phase_index,
                        _runtime: runtime,
                    }
                }))
            },
        )
        .unwrap();

        assert_eq!(effect_phases, [0, 0, 1, 1]);
        assert_eq!(random_receipts, [(0, 0xD2F6), (1, 0x0985)]);
        assert_eq!(
            ordered_effects,
            [
                (
                    0,
                    SharedGenericConstructorEffect::WriteSubADirection {
                        direction_multiplier: 1,
                    },
                ),
                (
                    0,
                    SharedGenericConstructorEffect::WriteSubATargetSpeed {
                        target_speed_raw: 270,
                        random_sample_low16: 0xD2F6,
                    },
                ),
                (
                    1,
                    SharedGenericConstructorEffect::WriteSubADirection {
                        direction_multiplier: 1,
                    },
                ),
                (
                    1,
                    SharedGenericConstructorEffect::WriteSubATargetSpeed {
                        target_speed_raw: 250,
                        random_sample_low16: 0x0985,
                    },
                ),
            ]
        );
        assert!(primary_retired.get());
        assert!(secondary_retired.get());
        assert!(tertiary_retired.get());
        assert!(matches!(
            owner.state_in_slot(ActorTaskSlot::Secondary),
            Some(PublicationProbeTask::New { phase_index: 0, .. })
        ));
        assert!(matches!(
            owner.state_in_slot(ActorTaskSlot::Primary),
            Some(PublicationProbeTask::New { phase_index: 1, .. })
        ));
    }

    #[test]
    fn detached_type9_acquiring_allocation_failures_preserve_each_committed_prefix() {
        fn seeded_owner() -> ActorTaskOwner<ActorTaskRuntime> {
            let mut owner = ActorTaskOwner::new();
            owner.replace_prepared(
                ActorTaskSlot::Primary,
                PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
            );
            owner.replace_prepared(ActorTaskSlot::Secondary, aim_and_fire_task());
            owner.replace_prepared(ActorTaskSlot::Tertiary, run_away_task());
            owner
        }

        let metadata = type9_run_away_suffix_metadata();

        let mut phase_zero_owner = seeded_owner();
        let old_zero_primary = phase_zero_owner.task_in_slot(ActorTaskSlot::Primary);
        let old_zero_secondary = phase_zero_owner.task_in_slot(ActorTaskSlot::Secondary);
        let phase_zero_draws = Cell::new(0);
        let mut phase_zero_effects = Vec::new();
        let phase_zero_error = apply_run_away_task_setup(
            &mut phase_zero_owner,
            RunAwayTaskSetupRequest::Acquiring,
            |preparation| {
                if preparation.phase_index == 0 {
                    return Err("phase-zero allocation");
                }
                let prepared = prepare_shared_acquiring_runtime_task(
                    preparation,
                    [100, 200, 300],
                    &metadata,
                    0,
                )
                .expect("exact type-9 phase one must authenticate if reached");
                Ok(prepared.apply_suffix(
                    || {
                        phase_zero_draws.set(phase_zero_draws.get() + 1);
                        0x0000_0985
                    },
                    |effect| phase_zero_effects.push(effect),
                ))
            },
        )
        .unwrap_err();
        assert_eq!(phase_zero_error.phase_index, 0);
        assert_eq!(phase_zero_error.slot, ActorTaskSlot::Secondary);
        assert_eq!(phase_zero_error.error, "phase-zero allocation");
        assert_eq!(phase_zero_draws.get(), 0);
        assert!(phase_zero_effects.is_empty());
        assert_eq!(phase_zero_owner.task_in_slot(ActorTaskSlot::Tertiary), None);
        assert_eq!(
            phase_zero_owner.task_in_slot(ActorTaskSlot::Secondary),
            old_zero_secondary
        );
        assert_eq!(
            phase_zero_owner.task_in_slot(ActorTaskSlot::Primary),
            old_zero_primary
        );

        let mut phase_one_owner = seeded_owner();
        let old_one_primary = phase_one_owner.task_in_slot(ActorTaskSlot::Primary);
        let old_one_secondary = phase_one_owner.task_in_slot(ActorTaskSlot::Secondary);
        let phase_one_draws = Cell::new(0);
        let mut phase_one_effects = Vec::new();
        let phase_one_error = apply_run_away_task_setup(
            &mut phase_one_owner,
            RunAwayTaskSetupRequest::Acquiring,
            |preparation| {
                if preparation.phase_index == 1 {
                    return Err("phase-one allocation");
                }
                let prepared = prepare_shared_acquiring_runtime_task(
                    preparation,
                    [100, 200, 300],
                    &metadata,
                    0,
                )
                .expect("exact type-9 phase zero must authenticate");
                assert_eq!(
                    prepared.topology(),
                    SharedGenericConstructorTopology::SubAOnly
                );
                Ok(prepared.apply_suffix(
                    || {
                        phase_one_draws.set(phase_one_draws.get() + 1);
                        0x0000_D2F6
                    },
                    |effect| phase_one_effects.push(effect),
                ))
            },
        )
        .unwrap_err();
        assert_eq!(phase_one_error.phase_index, 1);
        assert_eq!(phase_one_error.slot, ActorTaskSlot::Primary);
        assert_eq!(phase_one_error.error, "phase-one allocation");
        assert_eq!(phase_one_draws.get(), 1);
        assert_eq!(
            phase_one_effects,
            [
                SharedGenericConstructorEffect::WriteSubADirection {
                    direction_multiplier: 1,
                },
                SharedGenericConstructorEffect::WriteSubATargetSpeed {
                    target_speed_raw: 270,
                    random_sample_low16: 0xD2F6,
                },
            ]
        );
        assert_eq!(phase_one_owner.task_in_slot(ActorTaskSlot::Tertiary), None);
        assert_ne!(
            phase_one_owner.task_in_slot(ActorTaskSlot::Secondary),
            old_one_secondary
        );
        assert!(matches!(
            phase_one_owner.state_in_slot(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::TargetAcquisition(_))
        ));
        assert_eq!(
            phase_one_owner.task_in_slot(ActorTaskSlot::Primary),
            old_one_primary
        );
    }

    #[test]
    fn shared_acquiring_preparer_rejects_missing_initializer_and_wrong_phase_contract() {
        let setup = run_away_variant_setup(RunAwayVariant::Acquiring);
        let mut metadata = type17_run_away_metadata();
        metadata.initializer = None;

        let error = prepare_shared_acquiring_runtime_task(
            RunAwayTaskPreparation {
                request: RunAwayTaskSetupRequest::Acquiring,
                phase_index: 0,
                task: setup.ordered_phases[0].install,
            },
            [100, 200, 300],
            &metadata,
            0,
        )
        .unwrap_err();

        assert_eq!(
            error,
            SharedAcquiringRuntimePreparationError::MissingInitializer
        );

        let error = prepare_shared_acquiring_runtime_task(
            RunAwayTaskPreparation {
                request: RunAwayTaskSetupRequest::Acquiring,
                phase_index: 1,
                task: setup.ordered_phases[0].install,
            },
            [100, 200, 300],
            &type9_run_away_suffix_metadata(),
            0,
        )
        .unwrap_err();
        assert_eq!(
            error,
            SharedAcquiringRuntimePreparationError::TaskContractMismatch
        );
    }

    #[test]
    fn acquiring_phase_zero_unsupported_topology_preserves_old_tasks_after_tertiary_clear() {
        let mut metadata = type17_run_away_metadata();
        let RetailRuntimeValue::Known(mut topology) = metadata.common_mover_topology else {
            unreachable!()
        };
        topology.sub_f = true;
        metadata.common_mover_topology = RetailRuntimeValue::Known(topology);

        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        owner.replace_prepared(ActorTaskSlot::Secondary, aim_and_fire_task());
        owner.replace_prepared(ActorTaskSlot::Tertiary, run_away_task());
        let old_primary = owner.task_in_slot(ActorTaskSlot::Primary);
        let old_secondary = owner.task_in_slot(ActorTaskSlot::Secondary);
        let draws = Cell::new(0);
        let mut effects = Vec::new();

        let error = apply_run_away_task_setup(
            &mut owner,
            RunAwayTaskSetupRequest::Acquiring,
            |preparation| {
                let prepared = prepare_shared_acquiring_runtime_task(
                    preparation,
                    [100, 200, 300],
                    &metadata,
                    0,
                )?;
                Ok::<_, SharedAcquiringRuntimePreparationError>(prepared.apply_suffix(
                    || {
                        draws.set(draws.get() + 1);
                        0
                    },
                    |effect| effects.push(effect),
                ))
            },
        )
        .unwrap_err();

        assert_eq!(error.phase_index, 0);
        assert_eq!(error.slot, ActorTaskSlot::Secondary);
        assert_eq!(
            error.error,
            SharedAcquiringRuntimePreparationError::UnsupportedSharedConstructorTopology
        );
        assert_eq!(draws.get(), 0);
        assert!(effects.is_empty());
        assert!(owner.task_in_slot(ActorTaskSlot::Tertiary).is_none());
        assert_eq!(owner.task_in_slot(ActorTaskSlot::Secondary), old_secondary);
        assert_eq!(owner.task_in_slot(ActorTaskSlot::Primary), old_primary);
    }

    #[v2k_test_support::retail_test]
    fn retail_type9_metadata_closes_detached_sub_a_only_acquiring_phases() {
        let dir = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&dir).expect("init session");
        session
            .load_auxiliary_ovl(3, 1)
            .expect("load normal-tier first-world resources");
        session
            .load_level_by_id(13, 1)
            .expect("load normal-tier Level 1");
        let metadata = EntityTypeRuntimeMetadata::from_section12(
            session
                .cache
                .global_entity_type(9)
                .expect("retail type-9 Section-12 record"),
        );
        assert!(crate::main_base_type9_abort::exact_level_one_type9_metadata(&metadata));
        assert_eq!(
            metadata.common_mover_topology,
            RetailRuntimeValue::Known(CommonMoverComponentTopology {
                sub_a: true,
                sub_b: true,
                sub_d: true,
                sub_i: true,
                ..CommonMoverComponentTopology::default()
            })
        );
        assert!(matches!(
            metadata.sub_a_propulsion_descriptor,
            RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                target_speed_base_raw: 250,
                ..
            }))
        ));
        let initializer = metadata.initializer.as_ref().expect("type-9 initializer");
        assert_eq!(initializer.initializer_state_flags_raw, 0x2F);
        assert_eq!(
            initializer.common_axis_descriptor,
            CommonAxisDescriptor {
                strict_axis_limit_raw: 0x0F00,
                raw_word_at_0x04: 0x84,
            }
        );

        let setup = run_away_variant_setup(RunAwayVariant::Acquiring);
        let acquisition_prepared = prepare_shared_acquiring_runtime_task(
            RunAwayTaskPreparation {
                request: RunAwayTaskSetupRequest::Acquiring,
                phase_index: 0,
                task: setup.ordered_phases[0].install,
            },
            [100, 200, 300],
            &metadata,
            0,
        )
        .expect("retail type-9 target-acquisition profile");
        assert_eq!(
            acquisition_prepared.topology(),
            SharedGenericConstructorTopology::SubAOnly
        );
        let mut acquisition_effects = Vec::new();
        let acquisition = acquisition_prepared
            .apply_suffix(|| 0x0000_D2F6, |effect| acquisition_effects.push(effect));
        assert_eq!(
            acquisition_effects,
            [
                SharedGenericConstructorEffect::WriteSubADirection {
                    direction_multiplier: 1,
                },
                SharedGenericConstructorEffect::WriteSubATargetSpeed {
                    target_speed_raw: 270,
                    random_sample_low16: 0xD2F6,
                },
            ]
        );
        let mut acquisition_owner = ActorTaskOwner::new();
        acquisition_owner.replace_prepared(ActorTaskSlot::Secondary, acquisition);
        assert!(matches!(
            acquisition_owner.state_in_slot(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::TargetAcquisition(state))
                if state.radius() == SearchAttackRadius::strict(0x0F00).unwrap()
                    && state.filter()
                        == SearchAttackCandidateFilter::CapabilityMask(
                            std::num::NonZeroU32::new(0x84).unwrap()
                        )
                    && state.constructor_filter_override_raw() == 0
        ));

        let retarget_prepared = prepare_shared_acquiring_runtime_task(
            RunAwayTaskPreparation {
                request: RunAwayTaskSetupRequest::Acquiring,
                phase_index: 1,
                task: setup.ordered_phases[1].install,
            },
            [100, 200, 300],
            &metadata,
            0,
        )
        .expect("retail type-9 retarget profile");
        assert_eq!(
            retarget_prepared.topology(),
            SharedGenericConstructorTopology::SubAOnly
        );
        let mut retarget_effects = Vec::new();
        let retarget =
            retarget_prepared.apply_suffix(|| 0x0000_0985, |effect| retarget_effects.push(effect));
        assert_eq!(
            retarget_effects,
            [
                SharedGenericConstructorEffect::WriteSubADirection {
                    direction_multiplier: 1,
                },
                SharedGenericConstructorEffect::WriteSubATargetSpeed {
                    target_speed_raw: 250,
                    random_sample_low16: 0x0985,
                },
            ]
        );
        let mut retarget_owner = ActorTaskOwner::new();
        retarget_owner.replace_prepared(ActorTaskSlot::Primary, retarget);
        assert!(matches!(
            retarget_owner.state_in_slot(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(state))
                if state.private_state()
                    == WanderNearPrivateState::ordinary_type9([100, 200, 300])
                    && state.lifetime_ms() == 500
        ));
    }

    #[v2k_test_support::retail_test]
    fn retail_type17_metadata_closes_the_sub_h_then_sub_a_suffix_for_both_phases() {
        let dir = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&dir).expect("init session");
        session
            .load_auxiliary_ovl(3, 1)
            .expect("load normal-tier first-world resources");
        session
            .load_level_by_id(13, 1)
            .expect("load normal-tier Level 1");
        let metadata = EntityTypeRuntimeMetadata::from_section12(
            session
                .cache
                .global_entity_type(17)
                .expect("retail type-17 Section-12 record"),
        );
        assert_eq!(
            metadata.common_mover_topology,
            RetailRuntimeValue::Known(CommonMoverComponentTopology {
                sub_a: true,
                sub_b: true,
                sub_c: true,
                sub_d: true,
                sub_h: true,
                sub_j: true,
                ..CommonMoverComponentTopology::default()
            })
        );
        assert!(matches!(
            &metadata.sub_h_external_frame_descriptor,
            RetailRuntimeValue::Known(Some(descriptor)) if descriptor.records.len() == 8
        ));
        assert!(matches!(
            metadata.sub_a_propulsion_descriptor,
            RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                target_speed_base_raw: 250,
                ..
            }))
        ));
        assert_eq!(
            metadata
                .initializer
                .as_ref()
                .expect("type-17 initializer")
                .common_axis_descriptor,
            CommonAxisDescriptor {
                strict_axis_limit_raw: 2_560,
                raw_word_at_0x04: 3,
            }
        );

        let setup = run_away_variant_setup(RunAwayVariant::Acquiring);
        let mut acquisition_effects = Vec::new();
        let acquisition = prepare_shared_acquiring_runtime_task(
            RunAwayTaskPreparation {
                request: RunAwayTaskSetupRequest::Acquiring,
                phase_index: 0,
                task: setup.ordered_phases[0].install,
            },
            [100, 200, 300],
            &metadata,
            0,
        )
        .expect("retail type-17 target acquisition profile")
        .apply_suffix(|| 0x1234_ABCD, |effect| acquisition_effects.push(effect));
        let expected_effects = [
            SharedGenericConstructorEffect::WriteSubHState08 { value: 1 },
            SharedGenericConstructorEffect::WriteSubADirection {
                direction_multiplier: 1,
            },
            SharedGenericConstructorEffect::WriteSubATargetSpeed {
                target_speed_raw: 266,
                random_sample_low16: 0xABCD,
            },
        ];
        assert_eq!(acquisition_effects, expected_effects);
        let mut acquisition_owner = ActorTaskOwner::new();
        acquisition_owner.replace_prepared(ActorTaskSlot::Secondary, acquisition);
        assert!(matches!(
            acquisition_owner.state_in_slot(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::TargetAcquisition(state))
                if state.radius() == SearchAttackRadius::strict(2_560).unwrap()
                    && state.filter()
                        == SearchAttackCandidateFilter::CapabilityMask(
                            std::num::NonZeroU32::new(3).unwrap()
                        )
        ));

        let prepared = prepare_shared_acquiring_runtime_task(
            RunAwayTaskPreparation {
                request: RunAwayTaskSetupRequest::Acquiring,
                phase_index: 1,
                task: setup.ordered_phases[1].install,
            },
            [100, 200, 300],
            &metadata,
            0,
        )
        .expect("retail type-17 shared constructor profile");
        let mut effects = Vec::new();
        let task = prepared.apply_suffix(|| 0x1234_ABCD, |effect| effects.push(effect));
        assert_eq!(effects, expected_effects);
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, task);
        assert!(matches!(
            owner.state_in_slot(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(state))
                if state.lifetime_ms() == RUN_AWAY_ACQUIRING_WANDER_LIFETIME_MS
        ));
    }

    #[test]
    fn resolved_run_away_callback_commits_stage_and_visits_transition_replacement() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, run_away_task());
        let mut adapter = TestAdapter::new(Mode::ReplaceSecondaryFromRunAway);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        let Some(ActorTaskRuntime::RunAway(state)) = owner.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("primary task changed family")
        };
        assert_eq!(state.private_state().target_position_raw, [9, 8, 7]);
        assert_eq!(state.elapsed_ms(), 1);
        assert!(matches!(
            adapter.run_away_transitions.as_slice(),
            [RunAwayTransitionRequest {
                reason: crate::run_away::RunAwayTransitionReason::TaggedCallbackResult(
                    crate::run_away::RunAwayTaggedSingleton::DirectTarget
                ),
                ..
            }]
        ));
        assert_eq!(
            adapter.events,
            [
                "run away callback",
                "run away transition",
                "wander callback"
            ]
        );
    }

    #[test]
    fn unresolved_run_away_callback_commits_elapsed_but_discards_staged_state() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, run_away_task());
        let original = match owner.state_in_slot(ActorTaskSlot::Primary) {
            Some(ActorTaskRuntime::RunAway(state)) => *state,
            _ => panic!("primary task changed family"),
        };
        let mut adapter = TestAdapter::new(Mode::RunAwayError);

        let result = tick_actor_task_dispatcher(
            &mut owner,
            ActorTaskDispatcherFrame {
                elapsed_micros: 1_000,
                scheduler_mode: 0xA5,
            },
            &mut adapter,
        );

        assert!(matches!(
            result,
            Err(ActorTaskDispatcherError::RunAwayCallback {
                visit: ActorTaskVisit {
                    slot: ActorTaskSlot::Primary,
                    ..
                },
                committed_prefix: RunAwayCallbackPrefix {
                    lifetime_status: crate::run_away::RunAwayLifetimeStatus::WithinLifetime
                },
                error: "unresolved Run Away callback",
            })
        ));
        let Some(ActorTaskRuntime::RunAway(state)) = owner.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("primary task changed family")
        };
        assert_eq!(state.private_state(), original.private_state());
        assert_eq!(state.elapsed_ms(), 1);
        assert!(adapter.run_away_transitions.is_empty());
    }

    #[test]
    fn shared_retarget_uses_shared_rng_and_expires_strictly_after_500_ms() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, shared_retarget_task());
        let mut adapter = TestAdapter::new(Mode::SharedRetargetContinue);
        adapter.random = VecDeque::from([0, 0]);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 500_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        let Some(ActorTaskRuntime::SharedRetarget(state)) =
            owner.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("primary task changed family")
        };
        assert_eq!(state.elapsed_ms(), 500);
        assert_eq!(state.private_state().direction, -1);
        assert_eq!(adapter.random_draws, 2);
        assert!(adapter.shared_retarget_transitions.is_empty());

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert_eq!(adapter.random_draws, 3);
        assert!(matches!(
            adapter.shared_retarget_transitions.as_slice(),
            [SharedRetargetTransitionRequest {
                reason: SharedRetargetTransitionReason::LifetimeExpired,
                committed_prefix: SharedRetargetCallbackPrefix {
                    elapsed_ms: 501,
                    lifetime_ms: RUN_AWAY_ACQUIRING_WANDER_LIFETIME_MS,
                    ..
                },
                ..
            }]
        ));
    }

    #[test]
    fn shared_retarget_tag_precedes_expired_lifetime() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, shared_retarget_task());
        let mut adapter = TestAdapter::new(Mode::SharedRetargetZero);
        adapter.random = VecDeque::from([0, 0]);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 501_000,
                    scheduler_mode: 0,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert!(matches!(
            adapter.shared_retarget_transitions.as_slice(),
            [SharedRetargetTransitionRequest {
                reason: SharedRetargetTransitionReason::CommonMoverCompleted(
                    crate::wander_near_location::WanderNearTaggedResult {
                        singleton_address:
                            crate::shared_retarget_mover::SHARED_RETARGET_OWNER_TRANSITION_SINGLETON_ADDRESS,
                        tag: crate::shared_retarget_mover::SHARED_RETARGET_OWNER_TRANSITION_TAG,
                    }
                ),
                ..
            }]
        ));
    }

    #[test]
    fn unresolved_shared_retarget_keeps_elapsed_and_rng_prefix_only() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, shared_retarget_task());
        let mut adapter = TestAdapter::new(Mode::SharedRetargetUnresolved);
        adapter.random = VecDeque::from([0, 0]);

        let result = tick_actor_task_dispatcher(
            &mut owner,
            ActorTaskDispatcherFrame {
                elapsed_micros: 1_000,
                scheduler_mode: 0,
            },
            &mut adapter,
        );

        assert!(matches!(
            result,
            Err(
                ActorTaskDispatcherError::SharedRetargetUnresolvedCommonMover {
                    slot: ActorTaskSlot::Primary,
                    committed_prefix: SharedRetargetCallbackPrefix {
                        elapsed_ms: 1,
                        lifetime_ms: RUN_AWAY_ACQUIRING_WANDER_LIFETIME_MS,
                        retarget: SharedRetarget::Replaced { .. },
                    },
                }
            )
        ));
        let Some(ActorTaskRuntime::SharedRetarget(state)) =
            owner.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("primary task changed family")
        };
        assert_eq!(state.elapsed_ms(), 1);
        assert_eq!(
            state.private_state().target_position_raw,
            [-3_996, 200, -3_796]
        );
        assert_eq!(state.private_state().direction, 1);
        assert_eq!(adapter.random_draws, 2);
        assert!(adapter.shared_retarget_transitions.is_empty());
    }

    #[test]
    fn shared_retarget_self_replacement_discards_stale_outcomes_and_visits_fresh_tertiary() {
        for mode in [
            Mode::ReplaceSharedRetargetWithTertiaryError,
            Mode::ReplaceSharedRetargetWithTertiaryZero,
        ] {
            let mut owner = ActorTaskOwner::new();
            owner.replace_prepared(ActorTaskSlot::Primary, shared_retarget_task());
            let mut adapter = TestAdapter::new(mode);
            adapter.random = VecDeque::from([0, 0]);

            assert_eq!(
                tick_actor_task_dispatcher(
                    &mut owner,
                    ActorTaskDispatcherFrame {
                        elapsed_micros: 501_000,
                        scheduler_mode: 0,
                    },
                    &mut adapter,
                ),
                Ok(None)
            );

            let Some(ActorTaskRuntime::SharedRetarget(replacement)) =
                owner.state_in_slot(ActorTaskSlot::Primary)
            else {
                panic!("primary replacement changed family")
            };
            assert_eq!(replacement.elapsed_ms(), 0);
            assert_eq!(
                replacement.private_state(),
                WanderNearPrivateState::ordinary_type9([100, 200, 300])
            );
            assert!(adapter.shared_retarget_transitions.is_empty());
            assert_eq!(
                adapter.events,
                ["shared retarget callback", "wander callback"]
            );
            assert_eq!(adapter.visited_wander.len(), 1);
            assert!(matches!(
                owner.state_in_slot(ActorTaskSlot::Tertiary),
                Some(ActorTaskRuntime::OrdinaryType9Wander(_))
            ));
        }
    }

    #[test]
    fn run_away_self_clear_discards_callback_error_and_visits_later_slot() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, run_away_task());
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let mut adapter = TestAdapter::new(Mode::ClearRunAwayDuringCallback);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );
        assert_eq!(owner.state_in_slot(ActorTaskSlot::Primary), None);
        assert_eq!(adapter.events, ["run away callback", "wander callback"]);
        assert!(adapter.run_away_transitions.is_empty());
    }

    #[test]
    fn unresolved_wander_mover_keeps_only_the_committed_prefix() {
        let original = wander_state();
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(original)),
        );
        let mut adapter = TestAdapter::new(Mode::UnresolvedWander);

        let result = tick_actor_task_dispatcher(
            &mut owner,
            ActorTaskDispatcherFrame {
                elapsed_micros: 1_000,
                scheduler_mode: 0xA5,
            },
            &mut adapter,
        );

        assert!(matches!(
            result,
            Err(
                ActorTaskDispatcherError::OrdinaryWanderUnresolvedCommonMover {
                    slot: ActorTaskSlot::Primary,
                    ..
                }
            )
        ));
        let ActorTaskRuntime::OrdinaryType9Wander(state) =
            owner.state_in_slot(ActorTaskSlot::Primary).unwrap()
        else {
            panic!("primary task changed family")
        };
        assert_eq!(state.private_state(), original.private_state());
        assert_eq!(state.elapsed_ms(), 1);
        assert_eq!(adapter.events, ["wander callback"]);
    }

    #[test]
    fn ordinary_setup_maps_prepared_state_without_splitting_sub_a_reset() {
        let mut owner = ActorTaskOwner::new();
        let mut sub_a =
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(0), -1, 73);

        plan_ordinary_type9_wander_setup([1, 2, 3], 0x0300)
            .apply(&mut owner, &mut sub_a, |specification| {
                Ok::<_, Infallible>(
                    specification
                        .prepare_after_allocation(|| 1)
                        .map_task(ActorTaskRuntime::OrdinaryType9Wander),
                )
            })
            .unwrap();

        assert!(matches!(
            owner.state_in_slot(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::OrdinaryType9Wander(_))
        ));
    }

    #[test]
    fn defecate_setup_maps_exact_slots_lifetime_and_wander_anchor() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let anchor = [11, 22, 33];

        apply_defecate_virus_setup(
            &mut owner,
            DefecateVirusSetupRequest {
                terrain_task_lifetime_ms: 67,
            },
            DefecateVirusSubATopology::NotAuthored,
            |preparation| prepare_defecate_virus_runtime_task(preparation, anchor),
        )
        .unwrap();

        let Some(ActorTaskRuntime::DefecateVirusWander(wander)) =
            owner.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("slot 0 did not receive the Defecate wander companion")
        };
        assert_eq!(wander.private_state().target_position_raw, anchor);
        assert_eq!(wander.elapsed_ms(), 0);
        assert_eq!(owner.state_in_slot(ActorTaskSlot::Secondary), None);
        let Some(ActorTaskRuntime::DefecateVirusTerrain(terrain)) =
            owner.state_in_slot(ActorTaskSlot::Tertiary)
        else {
            panic!("slot 2 did not receive the Defecate terrain callback")
        };
        assert_eq!(terrain.lifetime_ms(), 67);
        assert_eq!(terrain.elapsed_ms(), 0);
    }

    #[test]
    fn defecate_runtime_preparation_rejects_a_nonretail_phase_contract() {
        let malformed = DefecateVirusTaskPreparation {
            phase_index: 1,
            task: DEFECATE_VIRUS_TERRAIN_TASK,
            constructor_inputs: DefecateVirusTaskConstructorInputs::TerrainInfection {
                lifetime_ms: 67,
                payload: DEFECATE_VIRUS_TERRAIN_PAYLOAD,
                mode: DEFECATE_VIRUS_TERRAIN_MODE,
                packed_mode: DEFECATE_VIRUS_TERRAIN_PACKED_MODE,
            },
        };

        assert!(matches!(
            prepare_defecate_virus_runtime_task(malformed, [11, 22, 33]),
            Err(DefecateVirusRuntimePreparationError::TaskContractMismatch)
        ));
    }

    #[test]
    fn mixed_defecate_slots_preserve_retail_order_and_rng_cadence() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::DefecateVirusWander(
                DefecateVirusWanderTaskState::new([100, 200, 300]),
            )),
        );
        owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(ActorTaskRuntime::DefecateVirusTerrain(
                DefecateVirusTerrainTaskState::new(0),
            )),
        );
        let mut adapter = TestAdapter::new(Mode::DefecateMixedRng);
        adapter.random = VecDeque::from([0x0000, 0xFFFF, 0x0000, 0xFFFF]);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );

        assert_eq!(
            adapter.events,
            ["virus wander callback", "virus terrain callback"]
        );
        assert_eq!(adapter.random_draws, 4);
        assert_eq!(
            adapter.defecate_terrain_plans,
            [DefecateVirusCallbackPlan::CoarseTerrainMutation(
                crate::infection_evolution::InfectionCellWrite {
                    cell: [0x12, 0x55],
                    infected: true,
                }
            )]
        );
        let Some(ActorTaskRuntime::DefecateVirusWander(wander)) =
            owner.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("slot 0 changed family")
        };
        assert_eq!(
            wander.private_state().target_position_raw,
            [100i16.wrapping_sub(4_096), 200, 300i16.wrapping_add(4_095),]
        );
    }

    #[test]
    fn defecate_terrain_self_clear_discards_callback_error_and_visits_later_slot() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::DefecateVirusTerrain(
                DefecateVirusTerrainTaskState::new(0),
            )),
        );
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::DefecateVirusWander(
                DefecateVirusWanderTaskState::new([100, 200, 300]),
            )),
        );
        let mut adapter = TestAdapter::new(Mode::ClearDefecateTerrainDuringCallback);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );

        assert_eq!(owner.state_in_slot(ActorTaskSlot::Primary), None);
        assert!(matches!(
            owner.state_in_slot(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::DefecateVirusWander(_))
        ));
        assert_eq!(
            adapter.events,
            ["virus terrain callback", "virus wander callback"]
        );
        assert_eq!(adapter.random_draws, 4);
        assert!(adapter.defecate_terrain_transitions.is_empty());
    }

    #[test]
    fn defecate_transition_replaces_later_family_and_visits_it_fresh() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::DefecateVirusWander(
                DefecateVirusWanderTaskState::new([100, 200, 300]),
            )),
        );
        let old_secondary = owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(wander_state())),
        );
        let mut adapter = TestAdapter::new(Mode::ReplaceSecondaryFromDefecate);
        adapter.old_secondary = Some(old_secondary);
        adapter.random = VecDeque::from([0, 0]);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );

        let replacement = owner
            .task_in_slot(ActorTaskSlot::Secondary)
            .expect("Defecate transition should install the replacement");
        assert_ne!(replacement, old_secondary);
        assert_eq!(adapter.defecate_terrain_visits, [replacement]);
        assert_eq!(
            adapter.events,
            ["virus wander callback", "virus terrain callback"]
        );
    }

    #[test]
    fn unresolved_defecate_mover_commits_prefix_but_discards_mover_writes() {
        let original = DefecateVirusWanderTaskState::new([100, 200, 300]);
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::DefecateVirusWander(original)),
        );
        owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(ActorTaskRuntime::DefecateVirusTerrain(
                DefecateVirusTerrainTaskState::new(67),
            )),
        );
        let mut adapter = TestAdapter::new(Mode::UnresolvedDefecateWander);

        let result = tick_actor_task_dispatcher(
            &mut owner,
            ActorTaskDispatcherFrame {
                elapsed_micros: 1_000,
                scheduler_mode: 0xA5,
            },
            &mut adapter,
        );

        assert!(matches!(
            result,
            Err(
                ActorTaskDispatcherError::DefecateVirusWanderUnresolvedCommonMover {
                    slot: ActorTaskSlot::Primary,
                    ..
                }
            )
        ));
        let Some(ActorTaskRuntime::DefecateVirusWander(state)) =
            owner.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("slot 0 changed family")
        };
        assert_eq!(state.elapsed_ms(), 1);
        assert_eq!(adapter.random_draws, 2);
        assert_eq!(adapter.events, ["virus wander callback"]);
        assert!(adapter.defecate_terrain_visits.is_empty());
        assert!(adapter.defecate_terrain_plans.is_empty());
        let Some(ActorTaskRuntime::DefecateVirusTerrain(terrain)) =
            owner.state_in_slot(ActorTaskSlot::Tertiary)
        else {
            panic!("blocked Primary must preserve the unvisited Tertiary task")
        };
        assert_eq!(terrain.elapsed_ms(), 0);
        assert_eq!(
            state.private_state().direction,
            original.private_state().direction
        );
        assert_eq!(
            state.private_state().target_position_raw,
            [100i16.wrapping_sub(4_096), 200, 300i16.wrapping_sub(4_096),]
        );
    }

    #[test]
    fn absent_tag_callback_reaches_timeout_check_but_remains_absent() {
        let state = DefecateVirusWanderTaskState::from_parts(
            WanderNearPrivateState::ordinary_type9([100, 200, 300]),
            2_000,
        );
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::DefecateVirusWander(state)),
        );
        let mut adapter = TestAdapter::new(Mode::DefecateTagCallbackAbsent);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );

        assert!(matches!(
            adapter.defecate_wander_transitions.as_slice(),
            [DefecateVirusWanderTransitionReason::CommonMoverCompleted(_)]
        ));
    }

    #[test]
    fn suppressed_tag_callback_skips_expired_timeout_fallthrough() {
        let state = DefecateVirusWanderTaskState::from_parts(
            WanderNearPrivateState::ordinary_type9([100, 200, 300]),
            2_000,
        );
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::DefecateVirusWander(state)),
        );
        let mut adapter = TestAdapter::new(Mode::DefecateTagCallbackSuppressed);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 1_000,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );

        assert!(matches!(
            adapter.defecate_wander_transitions.as_slice(),
            [DefecateVirusWanderTransitionReason::CommonMoverCompleted(_)]
        ));
    }

    #[test]
    fn terrain_timeout_is_strict_and_zero_rng_when_callback_is_suppressed() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(ActorTaskRuntime::DefecateVirusTerrain(
                DefecateVirusTerrainTaskState::new(1),
            )),
        );
        let mut adapter = TestAdapter::new(Mode::DefecateTerrainTimeout);

        for expected_elapsed in [1, 2] {
            assert_eq!(
                tick_actor_task_dispatcher(
                    &mut owner,
                    ActorTaskDispatcherFrame {
                        elapsed_micros: 1_000,
                        scheduler_mode: 0xA5,
                    },
                    &mut adapter,
                ),
                Ok(None)
            );
            let Some(ActorTaskRuntime::DefecateVirusTerrain(state)) =
                owner.state_in_slot(ActorTaskSlot::Tertiary)
            else {
                panic!("slot 2 changed family")
            };
            assert_eq!(state.elapsed_ms(), expected_elapsed);
        }

        assert_eq!(adapter.random_draws, 0);
        assert!(adapter.defecate_terrain_plans.is_empty());
        assert_eq!(adapter.defecate_terrain_transitions.len(), 1);
        assert_eq!(
            adapter.defecate_terrain_transitions[0],
            DefecateVirusTerrainCallbackPrefix {
                elapsed_ms: 2,
                lifetime_status: DefecateVirusTerrainLifetimeStatus::OwnerTransitionDue,
            }
        );
    }

    #[test]
    fn terrain_callback_uses_the_dispatcher_frame_duration() {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(ActorTaskRuntime::DefecateVirusTerrain(
                DefecateVirusTerrainTaskState::new(0),
            )),
        );
        let mut adapter = TestAdapter::new(Mode::DefecateElapsedMismatch);
        adapter.random = VecDeque::from([200]);

        assert_eq!(
            tick_actor_task_dispatcher(
                &mut owner,
                ActorTaskDispatcherFrame {
                    elapsed_micros: 400,
                    scheduler_mode: 0xA5,
                },
                &mut adapter,
            ),
            Ok(None)
        );

        // The adapter proposed 4,000 us, which would accept draw 200.
        // Retail's shared 400-us frame duration rejects it instead.
        assert_eq!(adapter.random_draws, 1);
        assert!(adapter.defecate_terrain_plans.is_empty());
    }
}
