//! Live specialized-scheduler attachment for fresh Level-1 Type-47 births.
//!
//! Construction already publishes Guard Location or Wander Near. This owner
//! moves those successful graphs into [`crate::specialized_actor_task_production`]
//! custody. Guard acquisition now consumes one shared RNG word and, on the
//! accepted one-in-four gate, walks the live list with the authored common-axis
//! `FUN_00422C10` pair (`strict_axis_limit_raw` / `+0x04`). Style `+0x44` is
//! zero, so the private filter override does not write.
//!
//! A found candidate invokes the EXE-proven class-32 variant-0 `+0x04`
//! callback `FUN_0040C7D0` at `0x004C7BB8`: store the target at context
//! `+0x08`, then `FUN_0040C6B0(variant + 1)`. C6B0 publishes style
//! `0x004C7C00`, whose `+0x40` is `FUN_0040ADE0`. ADE0 prepares Aim then
//! Chase before mutation, publishes Tertiary Aim (sound/period 0), clears
//! search, and publishes Primary Chase with Sub-A `(base * 4) / 3`. That is
//! the captured Type-47 pursuing graph, not an invented task table.
//!
//! Guard visits slot-0 `FUN_00402EB0` before slot-1 acquisition. The
//! companion consumes the one-or-three-word retarget around construction
//! `+0x90` from shared `FUN_0040D4A0` (type-default `+0xC0` bit `0x20`;
//! first-world word `0x2039`). Missing construction terrain stays
//! unresolved and blocks only a passing retarget gate. Its `FUN_00401430`
//! call uses the Type-47 frame machine and applies the `V200003.run` first
//! `FUN_0041FCB0` full-reset for seeds `0x2B/0x2C/0x2D`. A later
//! pursuing visit runs slot 0 `FUN_00403490` before slot 2 Aim: lifetime,
//! live target validation (`lookup` / dying `0x4000` / zero `+0x08`; unknown
//! `SURFACE_STATE_MASK` bits are not that test), and `FUN_00423030` with the
//! authored common-axis pair. Type 47 uses scheduler mode 0. Chase starts the shared
//! `FUN_00401430` frame machine on the authored A/B/C/D/E/H/J route and
//! consumes that same Level-1 first-query owner, then Sub-H and the
//! terrain-only C -> A -> B tail. Constructor seeds `0x2B/0x2C/0x2D` do
//! not authorize Type-9's reset or the Intro2 Type-47 first-query bind.
//! Replay-level seeds `0x3C/0x3D/0x3E` stay fail-closed.
//! A successful ADE0
//! handoff visits the newly published Tertiary Aim in the same owner pass;
//! Primary Chase is not revisited that pass. Class-32 variant-1 `+0x00`
//! and `+0x04` are both `FUN_0040C690`. Chase tag `0x9C01` is the
//! `FUN_00401120` `+0x00` slot; Aim tag `0x9C00` is `+0x04`. Both
//! calls pass through D760/D7A0, which forward the allocated behavior context
//! and a local zero word. C690 therefore reaches AC60's TypeDefault list.
//! Spawn 13 binds Aim-and-Fire through that same already-published lease.

use crate::actor_task_dispatcher::{
    prepare_aim_and_fire_runtime_task, ActorTaskRuntime, ActorTaskRuntimeFamily,
    AimAndFireRuntimeTaskPreparation,
};
use crate::actor_task_owner::{ActorTaskSlot, ActorTaskVisit};
use crate::aim_and_fire::{
    AimAndFireFrameOutcome, AimAndFireTransitionReason, AIM_AND_FIRE_CONSTRUCTOR_ADDRESS,
    AIM_AND_FIRE_LIFETIME_MS, AIM_AND_FIRE_TICK_ADDRESS,
};
use crate::aim_and_fire_transaction::AimAndFireEmitterTransactionId;
use crate::chase_target::{
    evaluate_chase_target_callback, ChaseTargetCallbackPrefix, ChaseTargetCallbackRequest,
    ChaseTargetCallbackResult, ChaseTargetLifetimeStatus, ChaseTargetProximityControl,
    ChaseTargetTaggedSingleton, ChaseTargetTargetRuntimeState, ChaseTargetTaskState,
};

use crate::common_mover::target_prelude::CommonMoverTrackedTargetSnapshot;
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::common_mover::type9_tail::{
    plan_common_master_motion, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
};
use crate::entity::{
    apply_first_world_common_dying_environment_raw, commit_common_master_motion, Entity,
    EntityManager,
};
use crate::entity_behavior::{
    audited_behavior_style, behavior_program, ActiveBehaviorStyle, BehaviorContextRuntime,
    BehaviorDescriptorIdentity,
};
use crate::entity_collision_state::{
    EntityTypeRuntimeMetadata, RetailRuntimeValue, DYING_STATE_BIT,
};
use crate::generic_projectile_emitter::GenericEmitterTransactionId;
use crate::guard_location_owner::acquisition::{
    evaluate_guard_location_acquisition_callback, GuardLocationAcquisitionCallbackError,
    GuardLocationAcquisitionCallbackPrefix, GuardLocationAcquisitionCallbackResult,
    GuardLocationCandidateFilter, GuardLocationEntityRef, GuardLocationSearchContext,
};
use crate::ordinary_type47_death_live::{
    publish_type47_c690_alternate_class12, FreshLevelOneType47CommonDyingOwner,
    Type47CommonDyingPublication, Type47CommonDyingPublicationError,
};
use crate::ordinary_type47_live::{
    bind_fresh_level1_ordinary_type47_aim_and_fire, tick_fresh_level1_ordinary_type47_aim_and_fire,
    OrdinaryType47AimAndFireFrameRequest, OrdinaryType47AimAndFireTickOutcome,
    OrdinaryType47LiveError, FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID,
    FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES,
};
use crate::ordinary_type9_wander_owner::OrdinaryType9WanderCallbackStage;
use crate::sub_h_external_frame::commit_sub_h_geometry;
use crate::type47_c690::{
    plan_type47_scheduler_c690, Type47C690Reselection, Type47C690ReselectionBlock, Type47C690Slot,
    Type47ImpactC690Outcome, Type47SchedulerC690Request, TYPE47_C690_SUPPRESS_STATE_BIT,
    TYPE47_IMPACT_DYING_STATE_BIT,
};
use crate::type47_chase_mover::{
    evaluate_type47_chase_common_mover, Type47ChaseMoverBlock, Type47ChaseMoverOutcome,
    Type47ChaseMoverRequest,
};
use crate::type47_initial_behavior_live::{
    publish_type47_reselected_initial_behavior, FreshType47InitialBehaviorError,
    FreshType47InitialBehaviorPublication, FreshType47InitializerPublication,
    Type47ReselectionCohort,
};
use crate::wander_near_location::{
    WanderNearLifetimeStatus, WanderNearPrivateState, WanderNearRetarget,
    WANDER_NEAR_RETARGET_GATE_MASK,
};
use crate::world_fx::WorldFx;
use crate::wrapped_axis_range::WrappedAxisRange;
use v2k_formats::terrain::TerrainGrid;

const ORDINARY_TYPE47_ENTITY_TYPE: u32 = 47;
const TYPE47_GUARD_BEHAVIOR_CLASS_ID: u32 = 32;
const TYPE47_GUARD_PURSUING_STYLE_INDEX: u32 = 1;
const TYPE47_CHASE_SCHEDULER_MODE: u32 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType47SchedulerOwner {
    publication: FreshType47InitialBehaviorPublication,
    next_transaction_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType47SchedulerBranch {
    GuardLocation,
    WanderNear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType47SchedulerAdoptionError {
    InitializerFallback { entity_id: u32 },
    GraphUnavailable { entity_id: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType47SchedulerProductionDrop {
    EntityUnavailable,
    GraphMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType47SchedulerProductionBlock {
    AimAndFire(OrdinaryType47LiveError),
    SearchContextUnresolved,
    AcquisitionVisitUnavailable,
    Acquisition(GuardLocationAcquisitionCallbackError),
    PursuingHandoffUnavailable,
    ChaseVisitUnavailable,
    ChaseSearchContextUnresolved,
    ChaseTargetStateUnresolved,
    WanderVisitUnavailable,
    C690MetadataUnavailable,
    C690(Type47C690ReselectionBlock),
    C690Publication(FreshType47InitialBehaviorError),
    C690AlternateClass12(Type47CommonDyingPublicationError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type47SchedulerC690Transition {
    SuppressedByEntityState {
        slot: Type47C690Slot,
    },
    GuardPublished {
        slot: Type47C690Slot,
        planned: Type47ImpactC690Outcome,
        initializer: FreshType47InitializerPublication,
    },
    WanderPublished {
        slot: Type47C690Slot,
        planned: Type47ImpactC690Outcome,
        initializer: FreshType47InitializerPublication,
    },
    Class12Published {
        slot: Type47C690Slot,
        planned: Type47ImpactC690Outcome,
        publication: Type47CommonDyingPublication,
    },
}

impl Type47SchedulerC690Transition {
    pub const fn slot(&self) -> Type47C690Slot {
        match self {
            Self::SuppressedByEntityState { slot }
            | Self::GuardPublished { slot, .. }
            | Self::WanderPublished { slot, .. }
            | Self::Class12Published { slot, .. } => *slot,
        }
    }

    pub(crate) const fn completed(&self) -> bool {
        !matches!(self, Self::SuppressedByEntityState { .. })
    }

    const fn common_dying_owner(&self) -> Option<FreshLevelOneType47CommonDyingOwner> {
        match self {
            Self::Class12Published { publication, .. } => Some(publication.owner),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type47PostChaseGuardPass {
    pub prefix: GuardLocationAcquisitionCallbackPrefix,
    pub result: GuardLocationAcquisitionCallbackResult,
    pub same_pass_aim: Option<Result<OrdinaryType47AimAndFireTickOutcome, OrdinaryType47LiveError>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type47WanderVisitResult {
    Continue,
    TaggedZeroMover,
    CommonMoverBlocked(Type47ChaseMoverBlock),
    AnchorUnresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type47WanderVisit {
    pub lifetime: WanderNearLifetimeStatus,
    pub retarget: WanderNearRetarget,
    pub result: Type47WanderVisitResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type47ChaseVisitResult {
    Tagged(ChaseTargetTaggedSingleton),
    Continue,
    CommonMoverBlocked(Type47ChaseMoverBlock),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type47ChaseVisit {
    pub prefix: ChaseTargetCallbackPrefix,
    pub result: Type47ChaseVisitResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType47SchedulerProductionOutcome {
    GraphRetained {
        entity_id: u32,
        branch: OrdinaryType47SchedulerBranch,
    },
    WanderNear {
        entity_id: u32,
        wander: Type47WanderVisit,
    },
    AimAndFire {
        entity_id: u32,
        tick: OrdinaryType47AimAndFireTickOutcome,
    },
    Acquisition {
        entity_id: u32,
        prefix: GuardLocationAcquisitionCallbackPrefix,
        result: GuardLocationAcquisitionCallbackResult,
        wander: Type47WanderVisit,
        same_pass_aim: Option<Result<OrdinaryType47AimAndFireTickOutcome, OrdinaryType47LiveError>>,
    },
    Pursuing {
        entity_id: u32,
        chase: Type47ChaseVisit,
        post_chase_plus00_c690: Option<Type47SchedulerC690Transition>,
        post_chase_guard: Option<Type47PostChaseGuardPass>,
        aim: Option<Result<OrdinaryType47AimAndFireTickOutcome, OrdinaryType47LiveError>>,
        post_aim_plus04_c690: Option<Type47SchedulerC690Transition>,
        post_aim_plus00_c690: Option<Type47SchedulerC690Transition>,
    },
    Blocked {
        entity_id: u32,
        reason: OrdinaryType47SchedulerProductionBlock,
    },
    Dropped {
        entity_id: u32,
        reason: OrdinaryType47SchedulerProductionDrop,
    },
}

impl OrdinaryType47SchedulerProductionOutcome {
    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::GraphRetained { entity_id, .. }
            | Self::WanderNear { entity_id, .. }
            | Self::AimAndFire { entity_id, .. }
            | Self::Acquisition { entity_id, .. }
            | Self::Pursuing { entity_id, .. }
            | Self::Blocked { entity_id, .. }
            | Self::Dropped { entity_id, .. } => *entity_id,
        }
    }

    pub(crate) const fn replacement_common_dying_owner(
        &self,
    ) -> Option<FreshLevelOneType47CommonDyingOwner> {
        let Self::Pursuing {
            post_chase_plus00_c690,
            post_aim_plus04_c690,
            post_aim_plus00_c690,
            ..
        } = self
        else {
            return None;
        };
        if let Some(owner) = match post_chase_plus00_c690 {
            Some(transition) => transition.common_dying_owner(),
            None => None,
        } {
            return Some(owner);
        }
        if let Some(owner) = match post_aim_plus04_c690 {
            Some(transition) => transition.common_dying_owner(),
            None => None,
        } {
            return Some(owner);
        }
        match post_aim_plus00_c690 {
            Some(transition) => transition.common_dying_owner(),
            None => None,
        }
    }
}

pub struct OrdinaryType47SchedulerOwnerTick {
    pub outcome: OrdinaryType47SchedulerProductionOutcome,
    pub retained_owner: Option<OrdinaryType47SchedulerOwner>,
}

impl OrdinaryType47SchedulerOwner {
    pub const fn entity_id(self) -> u32 {
        self.publication.entity_id
    }

    pub const fn publication(self) -> FreshType47InitialBehaviorPublication {
        self.publication
    }

    pub const fn branch(self) -> Option<OrdinaryType47SchedulerBranch> {
        match self.publication.initializer {
            FreshType47InitializerPublication::GuardLocation { .. } => {
                Some(OrdinaryType47SchedulerBranch::GuardLocation)
            }
            FreshType47InitializerPublication::WanderNear { .. } => {
                Some(OrdinaryType47SchedulerBranch::WanderNear)
            }
            FreshType47InitializerPublication::InitializerFallback { .. } => None,
        }
    }

    pub fn adopt(
        manager: &EntityManager,
        publication: FreshType47InitialBehaviorPublication,
    ) -> Result<Self, OrdinaryType47SchedulerAdoptionError> {
        if matches!(
            publication.initializer,
            FreshType47InitializerPublication::InitializerFallback { .. }
        ) {
            return Err(OrdinaryType47SchedulerAdoptionError::InitializerFallback {
                entity_id: publication.entity_id,
            });
        }
        let owner = Self {
            publication,
            next_transaction_id: 1,
        };
        if !owner.birth_graph_authenticates(manager) {
            return Err(OrdinaryType47SchedulerAdoptionError::GraphUnavailable {
                entity_id: publication.entity_id,
            });
        }
        Ok(owner)
    }

    pub(crate) fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }

    fn birth_graph_authenticates(self, manager: &EntityManager) -> bool {
        let Some(entity) = manager
            .iter_all()
            .find(|entity| entity.id == self.publication.entity_id)
        else {
            return false;
        };
        published_type47_graph_authenticates(entity, self.publication)
    }

    fn live_graph_authenticates(self, manager: &EntityManager) -> bool {
        let Some(entity) = manager
            .iter_all()
            .find(|entity| entity.id == self.publication.entity_id)
        else {
            return false;
        };
        published_type47_graph_authenticates(entity, self.publication)
            || (matches!(
                self.publication.initializer,
                FreshType47InitializerPublication::GuardLocation { .. }
            ) && type47_pursuing_graph_authenticates(entity)
                && entity.authored_spawn_index == Some(self.publication.authored_spawn_index))
            || crate::type47_impact_live::type47_live_graph_is_initial_guard(entity)
            || crate::type47_impact_live::type47_live_graph_is_initial_wander(entity)
    }

    fn take_aim_transaction_pair(
        &mut self,
    ) -> (AimAndFireEmitterTransactionId, GenericEmitterTransactionId) {
        let parent = self.next_transaction_id.max(1);
        let child = parent.wrapping_add(1).max(1);
        self.next_transaction_id = child.wrapping_add(1).max(1);
        (
            AimAndFireEmitterTransactionId::new(parent).expect("parent id is nonzero"),
            GenericEmitterTransactionId::new(child).expect("child id is nonzero"),
        )
    }
}

pub(crate) fn published_type47_graph_authenticates(
    entity: &Entity,
    publication: FreshType47InitialBehaviorPublication,
) -> bool {
    if entity.id != publication.entity_id
        || entity.entity_type != ORDINARY_TYPE47_ENTITY_TYPE
        || entity.authored_spawn_index != Some(publication.authored_spawn_index)
        || !FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES.contains(&publication.authored_spawn_index)
        || entity.model_index != Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID)
        || !entity.active
    {
        return false;
    }
    match publication.initializer {
        FreshType47InitializerPublication::GuardLocation { .. } => {
            task_family(entity, ActorTaskSlot::Secondary)
                == Some(ActorTaskRuntimeFamily::GuardLocationAcquisition)
                && task_family(entity, ActorTaskSlot::Primary)
                    == Some(ActorTaskRuntimeFamily::OrdinaryType9Wander)
        }
        FreshType47InitializerPublication::WanderNear { .. } => {
            task_family(entity, ActorTaskSlot::Primary)
                == Some(ActorTaskRuntimeFamily::OrdinaryType9Wander)
                && task_family(entity, ActorTaskSlot::Secondary).is_none()
        }
        FreshType47InitializerPublication::InitializerFallback { .. } => false,
    }
}

fn type47_pursuing_graph_authenticates(entity: &Entity) -> bool {
    entity
        .authored_spawn_index
        .is_some_and(|index| FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES.contains(&index))
        && type47_guard_pursuing_graph_contract_authenticates(entity)
}

/// Shared class-32 variant-1 graph contract after `FUN_0040ADE0` succeeds.
///
/// Scene-specific owners must authenticate their own captured spawn cohort
/// before calling this helper. Keeping that identity check outside the common
/// contract lets Intro2 reuse the statically identical Guard handoff without
/// treating its spawns 6/7/8 as Level-1 spawns 11/12/13.
pub(crate) fn type47_guard_pursuing_graph_contract_authenticates(entity: &Entity) -> bool {
    let model_admitted = match entity.native_type47_construction.as_ref() {
        Some(receipt) => receipt.entity_authenticates(entity),
        None => entity.model_index == Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID),
    };
    if entity.entity_type != ORDINARY_TYPE47_ENTITY_TYPE || !model_admitted || !entity.active {
        return false;
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return false;
    };
    let Some(program) = behavior_program(TYPE47_GUARD_BEHAVIOR_CLASS_ID) else {
        return false;
    };
    let Some(style) = audited_behavior_style(
        TYPE47_GUARD_BEHAVIOR_CLASS_ID,
        TYPE47_GUARD_PURSUING_STYLE_INDEX as u8,
    ) else {
        return false;
    };
    let RetailRuntimeValue::Known(Some(context_target)) = context.target_handle_at_0x08() else {
        return false;
    };
    if context.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || context.style_table_index_raw_at_0x10() != TYPE47_GUARD_PURSUING_STYLE_INDEX
        || context.active_style() != ActiveBehaviorStyle::Audited(*style)
    {
        return false;
    }
    let Some(ActorTaskRuntime::ChaseTarget(chase)) =
        entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        return false;
    };
    let Some(ActorTaskRuntime::AimAndFire(aim)) = entity.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        return false;
    };
    chase.target_id() == context_target
        && aim.private_state().target_entity_id() == context_target
        && task_family(entity, ActorTaskSlot::Secondary).is_none()
}

fn task_family(entity: &Entity, slot: ActorTaskSlot) -> Option<ActorTaskRuntimeFamily> {
    entity.actor_task_state(slot).map(ActorTaskRuntime::family)
}

pub fn tick_ordinary_type47_scheduler_owner(
    manager: &mut EntityManager,
    owner: OrdinaryType47SchedulerOwner,
    world_fx: &mut WorldFx,
    terrain: Option<&TerrainGrid>,
    model_records: Option<&[[i16; 4]]>,
    elapsed_micros: u32,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> OrdinaryType47SchedulerOwnerTick {
    let entity_id = owner.entity_id();
    if !owner.live_graph_authenticates(manager) {
        return OrdinaryType47SchedulerOwnerTick {
            outcome: OrdinaryType47SchedulerProductionOutcome::Dropped {
                entity_id,
                reason: OrdinaryType47SchedulerProductionDrop::GraphMismatch,
            },
            retained_owner: None,
        };
    }
    let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return OrdinaryType47SchedulerOwnerTick {
            outcome: OrdinaryType47SchedulerProductionOutcome::Dropped {
                entity_id,
                reason: OrdinaryType47SchedulerProductionDrop::EntityUnavailable,
            },
            retained_owner: None,
        };
    };
    if type47_pursuing_graph_authenticates(entity) {
        return tick_type47_pursuing(
            manager,
            owner,
            world_fx,
            terrain,
            model_records,
            elapsed_micros,
            next_shared_random,
        );
    }
    if crate::type47_impact_live::type47_live_graph_is_initial_guard(entity) {
        return tick_type47_guard_acquisition(
            manager,
            owner,
            world_fx,
            terrain,
            model_records,
            elapsed_micros,
            next_shared_random,
        );
    }
    if crate::type47_impact_live::type47_live_graph_is_initial_wander(entity) {
        return tick_type47_wander_near(
            manager,
            owner,
            world_fx,
            terrain,
            model_records,
            elapsed_micros,
            next_shared_random,
        );
    }
    match owner.branch() {
        Some(OrdinaryType47SchedulerBranch::GuardLocation) => tick_type47_guard_acquisition(
            manager,
            owner,
            world_fx,
            terrain,
            model_records,
            elapsed_micros,
            next_shared_random,
        ),
        Some(OrdinaryType47SchedulerBranch::WanderNear) => tick_type47_wander_near(
            manager,
            owner,
            world_fx,
            terrain,
            model_records,
            elapsed_micros,
            next_shared_random,
        ),
        None => OrdinaryType47SchedulerOwnerTick {
            outcome: OrdinaryType47SchedulerProductionOutcome::Dropped {
                entity_id,
                reason: OrdinaryType47SchedulerProductionDrop::GraphMismatch,
            },
            retained_owner: None,
        },
    }
}

fn tick_type47_pursuing(
    manager: &mut EntityManager,
    mut owner: OrdinaryType47SchedulerOwner,
    world_fx: &mut WorldFx,
    terrain: Option<&TerrainGrid>,
    model_records: Option<&[[i16; 4]]>,
    elapsed_micros: u32,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> OrdinaryType47SchedulerOwnerTick {
    let entity_id = owner.entity_id();
    let chase = match tick_type47_pursuing_chase(
        manager,
        world_fx,
        entity_id,
        terrain,
        model_records,
        elapsed_micros,
        next_shared_random,
    ) {
        Ok(chase) => chase,
        Err(OrdinaryType47SchedulerProductionDrop::EntityUnavailable) => {
            return OrdinaryType47SchedulerOwnerTick {
                outcome: OrdinaryType47SchedulerProductionOutcome::Dropped {
                    entity_id,
                    reason: OrdinaryType47SchedulerProductionDrop::EntityUnavailable,
                },
                retained_owner: None,
            };
        }
        Err(OrdinaryType47SchedulerProductionDrop::GraphMismatch) => {
            return OrdinaryType47SchedulerOwnerTick {
                outcome: OrdinaryType47SchedulerProductionOutcome::Dropped {
                    entity_id,
                    reason: OrdinaryType47SchedulerProductionDrop::GraphMismatch,
                },
                retained_owner: None,
            };
        }
    };
    let chase = match chase {
        Ok(chase) => chase,
        Err(reason) => {
            return OrdinaryType47SchedulerOwnerTick {
                outcome: OrdinaryType47SchedulerProductionOutcome::Blocked { entity_id, reason },
                retained_owner: Some(owner),
            };
        }
    };

    let mut post_chase_plus00_c690 = None;
    let mut post_chase_guard = None;
    if type47_chase_requests_plus00(&chase) {
        let transition = match apply_fresh_level1_type47_pursuing_c690(
            manager,
            entity_id,
            world_fx,
            Type47C690Slot::Plus00,
            next_shared_random,
        ) {
            Ok(transition) => transition,
            Err(reason) => {
                return OrdinaryType47SchedulerOwnerTick {
                    outcome: OrdinaryType47SchedulerProductionOutcome::Blocked {
                        entity_id,
                        reason,
                    },
                    retained_owner: Some(owner),
                };
            }
        };
        let completed = transition.completed();
        let published_guard = matches!(
            transition,
            Type47SchedulerC690Transition::GuardPublished { .. }
        );
        let published_class12 = matches!(
            transition,
            Type47SchedulerC690Transition::Class12Published { .. }
        );
        post_chase_plus00_c690 = Some(transition);
        if published_class12 {
            return OrdinaryType47SchedulerOwnerTick {
                outcome: OrdinaryType47SchedulerProductionOutcome::Pursuing {
                    entity_id,
                    chase,
                    post_chase_plus00_c690,
                    post_chase_guard,
                    aim: None,
                    post_aim_plus04_c690: None,
                    post_aim_plus00_c690: None,
                },
                retained_owner: None,
            };
        }
        if published_guard {
            post_chase_guard = match tick_type47_guard_acquisition_after_primary(
                manager,
                &mut owner,
                world_fx,
                elapsed_micros,
                next_shared_random,
            ) {
                Ok(pass) => Some(pass),
                Err(Type47GuardAcquisitionPassFailure::Blocked(reason)) => {
                    return OrdinaryType47SchedulerOwnerTick {
                        outcome: OrdinaryType47SchedulerProductionOutcome::Blocked {
                            entity_id,
                            reason,
                        },
                        retained_owner: Some(owner),
                    };
                }
                Err(Type47GuardAcquisitionPassFailure::Dropped(reason)) => {
                    return OrdinaryType47SchedulerOwnerTick {
                        outcome: OrdinaryType47SchedulerProductionOutcome::Dropped {
                            entity_id,
                            reason,
                        },
                        retained_owner: None,
                    };
                }
            };
            return OrdinaryType47SchedulerOwnerTick {
                outcome: OrdinaryType47SchedulerProductionOutcome::Pursuing {
                    entity_id,
                    chase,
                    post_chase_plus00_c690,
                    post_chase_guard,
                    aim: None,
                    post_aim_plus04_c690: None,
                    post_aim_plus00_c690: None,
                },
                retained_owner: owner.live_graph_authenticates(manager).then_some(owner),
            };
        }
        if completed {
            return OrdinaryType47SchedulerOwnerTick {
                outcome: OrdinaryType47SchedulerProductionOutcome::Pursuing {
                    entity_id,
                    chase,
                    post_chase_plus00_c690,
                    post_chase_guard,
                    aim: None,
                    post_aim_plus04_c690: None,
                    post_aim_plus00_c690: None,
                },
                retained_owner: owner.live_graph_authenticates(manager).then_some(owner),
            };
        }
    }

    let aim = Some(tick_type47_bound_aim(
        manager,
        &mut owner,
        world_fx,
        elapsed_micros,
    ));
    let mut post_aim_plus04_c690 = None;
    let mut post_aim_plus00_c690 = None;
    if let Some(Ok(outcome)) = aim.as_ref() {
        match outcome.resolution.outcome {
            AimAndFireFrameOutcome::RequestOwnerTransition {
                reason: AimAndFireTransitionReason::TaggedInvalidTarget { .. },
            } => {
                let transition = match apply_fresh_level1_type47_pursuing_c690(
                    manager,
                    entity_id,
                    world_fx,
                    Type47C690Slot::Plus04,
                    next_shared_random,
                ) {
                    Ok(transition) => transition,
                    Err(reason) => {
                        return OrdinaryType47SchedulerOwnerTick {
                            outcome: OrdinaryType47SchedulerProductionOutcome::Blocked {
                                entity_id,
                                reason,
                            },
                            retained_owner: Some(owner),
                        };
                    }
                };
                let suppressed = !transition.completed();
                post_aim_plus04_c690 = Some(transition);
                if suppressed
                    && outcome.resolution.prefix.lifetime_status
                        == crate::aim_and_fire::AimAndFireLifetimeStatus::OwnerTransitionDue
                {
                    post_aim_plus00_c690 = Some(
                        match apply_fresh_level1_type47_pursuing_c690(
                            manager,
                            entity_id,
                            world_fx,
                            Type47C690Slot::Plus00,
                            next_shared_random,
                        ) {
                            Ok(transition) => transition,
                            Err(reason) => {
                                return OrdinaryType47SchedulerOwnerTick {
                                    outcome: OrdinaryType47SchedulerProductionOutcome::Blocked {
                                        entity_id,
                                        reason,
                                    },
                                    retained_owner: Some(owner),
                                };
                            }
                        },
                    );
                }
            }
            AimAndFireFrameOutcome::RequestOwnerTransition {
                reason: AimAndFireTransitionReason::LifetimeExpired,
            } => {
                post_aim_plus00_c690 = Some(
                    match apply_fresh_level1_type47_pursuing_c690(
                        manager,
                        entity_id,
                        world_fx,
                        Type47C690Slot::Plus00,
                        next_shared_random,
                    ) {
                        Ok(transition) => transition,
                        Err(reason) => {
                            return OrdinaryType47SchedulerOwnerTick {
                                outcome: OrdinaryType47SchedulerProductionOutcome::Blocked {
                                    entity_id,
                                    reason,
                                },
                                retained_owner: Some(owner),
                            };
                        }
                    },
                );
            }
            AimAndFireFrameOutcome::Continue
            | AimAndFireFrameOutcome::ReturnGenericEmitterResult(_) => {}
        }
    }
    let replacement_is_common_dying =
        post_aim_plus04_c690.as_ref().is_some_and(|transition| {
            matches!(
                transition,
                Type47SchedulerC690Transition::Class12Published { .. }
            )
        }) || post_aim_plus00_c690.as_ref().is_some_and(|transition| {
            matches!(
                transition,
                Type47SchedulerC690Transition::Class12Published { .. }
            )
        });
    OrdinaryType47SchedulerOwnerTick {
        outcome: OrdinaryType47SchedulerProductionOutcome::Pursuing {
            entity_id,
            chase,
            post_chase_plus00_c690,
            post_chase_guard,
            aim,
            post_aim_plus04_c690,
            post_aim_plus00_c690,
        },
        retained_owner: (!replacement_is_common_dying && owner.live_graph_authenticates(manager))
            .then_some(owner),
    }
}

fn type47_chase_requests_plus00(chase: &Type47ChaseVisit) -> bool {
    matches!(chase.result, Type47ChaseVisitResult::Tagged(_))
        || (matches!(chase.result, Type47ChaseVisitResult::Continue)
            && chase.prefix.lifetime_status == ChaseTargetLifetimeStatus::OwnerTransitionDue)
}

pub(crate) fn plan_type47_scheduler_c690_live(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    slot: Type47C690Slot,
    next_random: impl FnMut() -> u32,
) -> Result<Type47C690Reselection, Type47C690ReselectionBlock> {
    let (current_style, choice_list_source) = match entity.current_behavior_context {
        RetailRuntimeValue::Known(Some(context)) => (
            RetailRuntimeValue::Known(Some(context.active_style())),
            context.choice_list_source(),
        ),
        RetailRuntimeValue::Known(None) => (
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Unresolved,
        ),
        RetailRuntimeValue::Unresolved => (
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
        ),
    };
    plan_type47_scheduler_c690(
        Type47SchedulerC690Request {
            slot,
            entity_type: entity.entity_type,
            current_style,
            choice_list_source,
            state_flags_raw: entity
                .collision
                .state_flags_at_0x08
                .masked(TYPE47_C690_SUPPRESS_STATE_BIT | TYPE47_IMPACT_DYING_STATE_BIT),
            metadata,
        },
        next_random,
    )
}

fn apply_fresh_level1_type47_pursuing_c690(
    manager: &mut EntityManager,
    entity_id: u32,
    world_fx: &mut WorldFx,
    slot: Type47C690Slot,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Result<Type47SchedulerC690Transition, OrdinaryType47SchedulerProductionBlock> {
    let metadata = manager
        .type_runtime_metadata(ORDINARY_TYPE47_ENTITY_TYPE)
        .cloned()
        .ok_or(OrdinaryType47SchedulerProductionBlock::C690MetadataUnavailable)?;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .ok_or(OrdinaryType47SchedulerProductionBlock::C690MetadataUnavailable)?;
    let reselection =
        plan_type47_scheduler_c690_live(entity, &metadata, slot, || next_shared_random(world_fx))
            .map_err(OrdinaryType47SchedulerProductionBlock::C690)?;
    match reselection {
        Type47C690Reselection::SuppressedByEntityState => {
            Ok(Type47SchedulerC690Transition::SuppressedByEntityState { slot })
        }
        Type47C690Reselection::Applied(planned) => match planned {
            Type47ImpactC690Outcome::AlternateCommonDying { .. } => {
                let entity = manager
                    .ordinary_type47_entity_mut(entity_id)
                    .ok_or(OrdinaryType47SchedulerProductionBlock::C690MetadataUnavailable)?;
                let publication =
                    publish_type47_c690_alternate_class12(entity, &metadata, world_fx)
                        .map_err(OrdinaryType47SchedulerProductionBlock::C690AlternateClass12)?;
                Ok(Type47SchedulerC690Transition::Class12Published {
                    slot,
                    planned,
                    publication,
                })
            }
            Type47ImpactC690Outcome::Weighted { selection, .. } => {
                let entity = manager
                    .ordinary_type47_entity_mut(entity_id)
                    .ok_or(OrdinaryType47SchedulerProductionBlock::C690MetadataUnavailable)?;
                let initializer = publish_type47_reselected_initial_behavior(
                    entity,
                    &metadata,
                    Type47ReselectionCohort::FreshLevel1,
                    selection,
                    world_fx,
                )
                .map_err(OrdinaryType47SchedulerProductionBlock::C690Publication)?;
                Ok(match selection.program.class_id {
                    32 => Type47SchedulerC690Transition::GuardPublished {
                        slot,
                        planned,
                        initializer,
                    },
                    6 => Type47SchedulerC690Transition::WanderPublished {
                        slot,
                        planned,
                        initializer,
                    },
                    _ => unreachable!("C690 planner admits only Type-47 Guard/Wander choices"),
                })
            }
        },
    }
}

fn tick_type47_bound_aim(
    manager: &mut EntityManager,
    owner: &mut OrdinaryType47SchedulerOwner,
    world_fx: &mut WorldFx,
    elapsed_micros: u32,
) -> Result<OrdinaryType47AimAndFireTickOutcome, OrdinaryType47LiveError> {
    let Some(entity) = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
    else {
        return Err(OrdinaryType47LiveError::SourceEntityMissing);
    };
    match bind_fresh_level1_ordinary_type47_aim_and_fire(entity) {
        Ok(aim_owner) => {
            let (parent_transaction_id, child_transaction_id) = owner.take_aim_transaction_pair();
            tick_fresh_level1_ordinary_type47_aim_and_fire(
                manager,
                world_fx,
                aim_owner,
                OrdinaryType47AimAndFireFrameRequest {
                    parent_transaction_id,
                    child_transaction_id,
                    elapsed_micros,
                },
            )
        }
        Err(reason) => Err(reason),
    }
}

fn tick_type47_wander_near(
    manager: &mut EntityManager,
    owner: OrdinaryType47SchedulerOwner,
    world_fx: &mut WorldFx,
    terrain: Option<&TerrainGrid>,
    model_records: Option<&[[i16; 4]]>,
    elapsed_micros: u32,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> OrdinaryType47SchedulerOwnerTick {
    let entity_id = owner.entity_id();
    let metadata = manager
        .type_runtime_metadata(ORDINARY_TYPE47_ENTITY_TYPE)
        .cloned();
    let Some(entity) = manager.ordinary_type47_entity_mut(entity_id) else {
        return OrdinaryType47SchedulerOwnerTick {
            outcome: OrdinaryType47SchedulerProductionOutcome::Dropped {
                entity_id,
                reason: OrdinaryType47SchedulerProductionDrop::EntityUnavailable,
            },
            retained_owner: None,
        };
    };
    match tick_type47_class6_wander(
        entity,
        metadata.as_ref(),
        terrain,
        model_records,
        elapsed_micros,
        &mut || next_shared_random(world_fx),
    ) {
        Ok(wander) => OrdinaryType47SchedulerOwnerTick {
            outcome: OrdinaryType47SchedulerProductionOutcome::WanderNear { entity_id, wander },
            retained_owner: Some(owner),
        },
        Err(OrdinaryType47SchedulerProductionBlock::WanderVisitUnavailable) => {
            OrdinaryType47SchedulerOwnerTick {
                outcome: OrdinaryType47SchedulerProductionOutcome::Dropped {
                    entity_id,
                    reason: OrdinaryType47SchedulerProductionDrop::GraphMismatch,
                },
                retained_owner: None,
            }
        }
        Err(reason) => OrdinaryType47SchedulerOwnerTick {
            outcome: OrdinaryType47SchedulerProductionOutcome::Blocked { entity_id, reason },
            retained_owner: Some(owner),
        },
    }
}

fn tick_type47_class6_wander(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
    terrain: Option<&TerrainGrid>,
    model_records: Option<&[[i16; 4]]>,
    elapsed_micros: u32,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Type47WanderVisit, OrdinaryType47SchedulerProductionBlock> {
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) else {
        return Err(OrdinaryType47SchedulerProductionBlock::WanderVisitUnavailable);
    };
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let Some(lifetime) = entity.actor_tasks.begin_exact_visit_with(visit, |runtime| {
        let ActorTaskRuntime::OrdinaryType9Wander(state) = runtime else {
            unreachable!("Type-47 class-6 Primary is OrdinaryType9Wander")
        };
        state.before_callback(elapsed_micros)
    }) else {
        return Err(OrdinaryType47SchedulerProductionBlock::WanderVisitUnavailable);
    };
    let mut stage = match entity.type47_immutable_anchor_raw_at_0x90 {
        RetailRuntimeValue::Known(anchor) => {
            let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) =
                entity.actor_tasks.task_state_mut(visit.task_id)
            else {
                return Err(OrdinaryType47SchedulerProductionBlock::WanderVisitUnavailable);
            };
            state.stage_callback(anchor, &mut *next_random)
        }
        RetailRuntimeValue::Unresolved => {
            let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) =
                entity.actor_tasks.state_in_slot(ActorTaskSlot::Primary)
            else {
                return Err(OrdinaryType47SchedulerProductionBlock::WanderVisitUnavailable);
            };
            let gate = next_random();
            if (gate as u16) & WANDER_NEAR_RETARGET_GATE_MASK != 0 {
                OrdinaryType9WanderCallbackStage::retained(state.private_state())
            } else {
                let target_position_raw = state.private_state().target_position_raw;
                if entity
                    .actor_tasks
                    .wrapper_flags(visit.task_id)
                    .is_some_and(|flags| flags.in_callback)
                {
                    let _ = entity.actor_tasks.finish_exact_visit(visit);
                }
                return Ok(Type47WanderVisit {
                    lifetime,
                    retarget: WanderNearRetarget::Replaced {
                        target_position_raw,
                    },
                    result: Type47WanderVisitResult::AnchorUnresolved,
                });
            }
        }
    };
    let mover = apply_type47_chase_mover(
        entity,
        metadata,
        terrain,
        stage.private_state_mut(),
        RetailRuntimeValue::Known(None),
        elapsed_micros,
        next_random,
    );
    let result = match mover {
        Ok(outcome) => {
            commit_type47_chase_pose(
                entity,
                metadata,
                terrain,
                model_records,
                outcome,
                elapsed_micros,
            );
            match outcome.result {
                crate::chase_target::ChaseTargetCommonMoverReturn::NonZero => {
                    Type47WanderVisitResult::Continue
                }
                crate::chase_target::ChaseTargetCommonMoverReturn::Zero => {
                    Type47WanderVisitResult::TaggedZeroMover
                }
            }
        }
        Err(block) => Type47WanderVisitResult::CommonMoverBlocked(block),
    };
    if matches!(
        result,
        Type47WanderVisitResult::Continue | Type47WanderVisitResult::TaggedZeroMover
    ) {
        if let Some(ActorTaskRuntime::OrdinaryType9Wander(surviving)) =
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
    Ok(Type47WanderVisit {
        lifetime,
        retarget: stage.retarget(),
        result,
    })
}

fn tick_type47_pursuing_chase(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    terrain: Option<&TerrainGrid>,
    model_records: Option<&[[i16; 4]]>,
    elapsed_micros: u32,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Result<
    Result<Type47ChaseVisit, OrdinaryType47SchedulerProductionBlock>,
    OrdinaryType47SchedulerProductionDrop,
> {
    let metadata = manager
        .type_runtime_metadata(ORDINARY_TYPE47_ENTITY_TYPE)
        .cloned();
    let (target_id, axis, owner_position_raw) = {
        let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
            return Err(OrdinaryType47SchedulerProductionDrop::EntityUnavailable);
        };
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(OrdinaryType47SchedulerProductionDrop::GraphMismatch);
        };
        let RetailRuntimeValue::Known(Some(target_id)) = context.target_handle_at_0x08() else {
            return Err(OrdinaryType47SchedulerProductionDrop::GraphMismatch);
        };
        let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
            return Ok(Err(
                OrdinaryType47SchedulerProductionBlock::ChaseSearchContextUnresolved,
            ));
        };
        (target_id, axis, entity.position_raw())
    };
    let (target_state, tracked_target) = match type47_chase_target_runtime_state(manager, target_id)
    {
        Ok(state) => state,
        Err(()) => {
            return Ok(Err(
                OrdinaryType47SchedulerProductionBlock::ChaseTargetStateUnresolved,
            ));
        }
    };

    let Some(entity) = manager.ordinary_type47_entity_mut(entity_id) else {
        return Err(OrdinaryType47SchedulerProductionDrop::EntityUnavailable);
    };
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) else {
        return Err(OrdinaryType47SchedulerProductionDrop::GraphMismatch);
    };
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let Some(prefix) = entity.actor_tasks.begin_exact_visit_with(visit, |runtime| {
        let ActorTaskRuntime::ChaseTarget(state) = runtime else {
            unreachable!("pursuing Primary is ChaseTarget")
        };
        state.before_callback(elapsed_micros)
    }) else {
        return Ok(Err(
            OrdinaryType47SchedulerProductionBlock::ChaseVisitUnavailable,
        ));
    };
    let Some(ActorTaskRuntime::ChaseTarget(state)) =
        entity.actor_tasks.state_in_slot(ActorTaskSlot::Primary)
    else {
        return Err(OrdinaryType47SchedulerProductionDrop::GraphMismatch);
    };
    let mut stage = state.stage_callback();
    let mut movement = ();
    let mut controller = ();
    let mut applied_mover = None;
    let evaluated = evaluate_chase_target_callback(
        &mut stage,
        ChaseTargetCallbackRequest {
            visit,
            entity_id,
            owner_position_raw,
            route_range: WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
            proximity_control: ChaseTargetProximityControl::Disabled,
            movement_state: &mut movement,
            controller_context: &mut controller,
            elapsed_micros,
            scheduler_mode: TYPE47_CHASE_SCHEDULER_MODE,
        },
        |_| Ok::<_, Type47ChaseMoverBlock>(target_state),
        |request| {
            let outcome = apply_type47_chase_mover(
                entity,
                metadata.as_ref(),
                terrain,
                request.target_state,
                tracked_target,
                elapsed_micros,
                &mut || next_shared_random(world_fx),
            )?;
            let result = outcome.result;
            applied_mover = Some(outcome);
            Ok(result)
        },
        |_, _| {
            unreachable!("Type-47 Chase does not invent post-mover positions without FUN_00401430")
        },
    );
    if entity
        .actor_tasks
        .wrapper_flags(visit.task_id)
        .is_some_and(|flags| flags.in_callback)
    {
        let _ = entity.actor_tasks.finish_exact_visit(visit);
    }
    let result = match evaluated {
        Ok(ChaseTargetCallbackResult::Tagged(singleton)) => {
            Type47ChaseVisitResult::Tagged(singleton)
        }
        Ok(ChaseTargetCallbackResult::Continue { .. }) => Type47ChaseVisitResult::Continue,
        Err(crate::chase_target::ChaseTargetCallbackError::CommonMover { error, .. }) => {
            Type47ChaseVisitResult::CommonMoverBlocked(error)
        }
        Err(_) => Type47ChaseVisitResult::CommonMoverBlocked(
            Type47ChaseMoverBlock::SubARuntimeUnavailable,
        ),
    };
    if matches!(
        result,
        Type47ChaseVisitResult::Continue | Type47ChaseVisitResult::Tagged(_)
    ) {
        if let Some(outcome) = applied_mover {
            commit_type47_chase_pose(
                entity,
                metadata.as_ref(),
                terrain,
                model_records,
                outcome,
                elapsed_micros,
            );
        }
    }
    Ok(Ok(Type47ChaseVisit { prefix, result }))
}

pub(crate) fn type47_chase_target_runtime_state(
    manager: &EntityManager,
    target_id: u32,
) -> Result<
    (
        ChaseTargetTargetRuntimeState,
        RetailRuntimeValue<
            Option<crate::common_mover::target_prelude::CommonMoverTrackedTargetSnapshot>,
        >,
    ),
    (),
> {
    let Some(target) = manager.iter_all().find(|entity| entity.id == target_id) else {
        return Ok((
            ChaseTargetTargetRuntimeState::Missing,
            RetailRuntimeValue::Known(None),
        ));
    };
    if !target.active {
        return Ok((
            ChaseTargetTargetRuntimeState::Inactive,
            RetailRuntimeValue::Known(None),
        ));
    }
    let state = target.collision.state_flags_at_0x08;
    let snapshot = RetailRuntimeValue::Known(Some(
        crate::common_mover::target_prelude::CommonMoverTrackedTargetSnapshot {
            state_flags: state,
            position_raw: target.position_raw(),
            velocity_raw: target.velocity_raw(),
        },
    ));
    match state.masked(DYING_STATE_BIT) {
        RetailRuntimeValue::Unresolved => return Err(()),
        RetailRuntimeValue::Known(bits) if bits != 0 => {
            return Ok((ChaseTargetTargetRuntimeState::Dying, snapshot));
        }
        RetailRuntimeValue::Known(_) => {}
    }
    // FUN_00403490 tags InvalidTarget on lookup failure, dying bit 0x4000, or
    // a zero +0x08 word. Constructor-unknown SURFACE_STATE_MASK bits are not
    // that test: any known nonzero remainder proves the dword is live.
    if state.known_value_bits() != 0 {
        return Ok((
            ChaseTargetTargetRuntimeState::Live {
                position_raw: target.position_raw(),
            },
            snapshot,
        ));
    }
    if state.known_mask() != u32::MAX {
        return Err(());
    }
    Ok((ChaseTargetTargetRuntimeState::Inactive, snapshot))
}

fn tick_type47_guard_acquisition(
    manager: &mut EntityManager,
    mut owner: OrdinaryType47SchedulerOwner,
    world_fx: &mut WorldFx,
    terrain: Option<&TerrainGrid>,
    model_records: Option<&[[i16; 4]]>,
    elapsed_micros: u32,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> OrdinaryType47SchedulerOwnerTick {
    let entity_id = owner.entity_id();
    let metadata = manager
        .type_runtime_metadata(ORDINARY_TYPE47_ENTITY_TYPE)
        .cloned();
    let Some(entity) = manager.ordinary_type47_entity_mut(entity_id) else {
        return OrdinaryType47SchedulerOwnerTick {
            outcome: OrdinaryType47SchedulerProductionOutcome::Dropped {
                entity_id,
                reason: OrdinaryType47SchedulerProductionDrop::EntityUnavailable,
            },
            retained_owner: None,
        };
    };
    let wander = match tick_type47_class6_wander(
        entity,
        metadata.as_ref(),
        terrain,
        model_records,
        elapsed_micros,
        &mut || next_shared_random(world_fx),
    ) {
        Ok(wander) => wander,
        Err(OrdinaryType47SchedulerProductionBlock::WanderVisitUnavailable) => {
            return OrdinaryType47SchedulerOwnerTick {
                outcome: OrdinaryType47SchedulerProductionOutcome::Dropped {
                    entity_id,
                    reason: OrdinaryType47SchedulerProductionDrop::GraphMismatch,
                },
                retained_owner: None,
            };
        }
        Err(reason) => {
            return OrdinaryType47SchedulerOwnerTick {
                outcome: OrdinaryType47SchedulerProductionOutcome::Blocked { entity_id, reason },
                retained_owner: Some(owner),
            };
        }
    };
    let pass = match tick_type47_guard_acquisition_after_primary(
        manager,
        &mut owner,
        world_fx,
        elapsed_micros,
        next_shared_random,
    ) {
        Ok(pass) => pass,
        Err(Type47GuardAcquisitionPassFailure::Blocked(reason)) => {
            return OrdinaryType47SchedulerOwnerTick {
                outcome: OrdinaryType47SchedulerProductionOutcome::Blocked { entity_id, reason },
                retained_owner: Some(owner),
            };
        }
        Err(Type47GuardAcquisitionPassFailure::Dropped(reason)) => {
            return OrdinaryType47SchedulerOwnerTick {
                outcome: OrdinaryType47SchedulerProductionOutcome::Dropped { entity_id, reason },
                retained_owner: None,
            };
        }
    };
    OrdinaryType47SchedulerOwnerTick {
        outcome: OrdinaryType47SchedulerProductionOutcome::Acquisition {
            entity_id,
            prefix: pass.prefix,
            result: pass.result,
            wander,
            same_pass_aim: pass.same_pass_aim,
        },
        retained_owner: Some(owner),
    }
}

enum Type47GuardAcquisitionPassFailure {
    Blocked(OrdinaryType47SchedulerProductionBlock),
    Dropped(OrdinaryType47SchedulerProductionDrop),
}

fn tick_type47_guard_acquisition_after_primary(
    manager: &mut EntityManager,
    owner: &mut OrdinaryType47SchedulerOwner,
    world_fx: &mut WorldFx,
    elapsed_micros: u32,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Result<Type47PostChaseGuardPass, Type47GuardAcquisitionPassFailure> {
    let entity_id = owner.entity_id();
    let candidates = manager
        .retail_live_order_ids()
        .filter_map(|id| {
            manager
                .iter_all()
                .find(|entity| entity.id == id)
                .map(type47_guard_entity_ref)
        })
        .collect::<Vec<_>>();
    let metadata = manager
        .type_runtime_metadata(ORDINARY_TYPE47_ENTITY_TYPE)
        .cloned();
    let Some(entity) = manager.ordinary_type47_entity_mut(entity_id) else {
        return Err(Type47GuardAcquisitionPassFailure::Dropped(
            OrdinaryType47SchedulerProductionDrop::EntityUnavailable,
        ));
    };
    let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
        return Err(Type47GuardAcquisitionPassFailure::Blocked(
            OrdinaryType47SchedulerProductionBlock::SearchContextUnresolved,
        ));
    };
    let mut search_context = GuardLocationSearchContext::new(
        WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
        GuardLocationCandidateFilter::from_raw(axis.raw_word_at_0x04),
    );
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Secondary) else {
        return Err(Type47GuardAcquisitionPassFailure::Dropped(
            OrdinaryType47SchedulerProductionDrop::GraphMismatch,
        ));
    };
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Secondary,
        task_id,
    };
    let Some(prefix) = entity.actor_tasks.begin_exact_visit_with(visit, |runtime| {
        let ActorTaskRuntime::GuardLocationAcquisition(state) = runtime else {
            unreachable!("adopted Guard Secondary is GuardLocationAcquisition")
        };
        let random = next_shared_random(world_fx);
        state.before_callback(&mut search_context, random)
    }) else {
        return Err(Type47GuardAcquisitionPassFailure::Blocked(
            OrdinaryType47SchedulerProductionBlock::AcquisitionVisitUnavailable,
        ));
    };
    let owner_ref = type47_guard_entity_ref(entity);
    let result = evaluate_guard_location_acquisition_callback(
        prefix,
        owner_ref,
        &candidates,
        Some(
            |handoff: crate::guard_location_owner::acquisition::GuardLocationCandidateHandoff| {
                let Some(metadata) = metadata.as_ref() else {
                    return 1;
                };
                match apply_type47_guard_pursuing_handoff(entity, metadata, handoff.candidate.id) {
                    Ok(()) => 0,
                    Err(_) => 1,
                }
            },
        ),
    );
    if entity
        .actor_tasks
        .wrapper_flags(visit.task_id)
        .is_some_and(|flags| flags.in_callback)
    {
        let _ = entity.actor_tasks.finish_exact_visit(visit);
    }
    let result = result.map_err(|error| {
        Type47GuardAcquisitionPassFailure::Blocked(
            OrdinaryType47SchedulerProductionBlock::Acquisition(error),
        )
    })?;
    let same_pass_aim = matches!(
        result,
        GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted { .. }
    )
    .then(|| tick_type47_bound_aim(manager, owner, world_fx, elapsed_micros));
    Ok(Type47PostChaseGuardPass {
        prefix,
        result,
        same_pass_aim,
    })
}

pub(crate) fn apply_type47_guard_pursuing_handoff(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    target_id: u32,
) -> Result<(), OrdinaryType47SchedulerProductionBlock> {
    let RetailRuntimeValue::Known(Some(current)) = entity.current_behavior_context else {
        return Err(OrdinaryType47SchedulerProductionBlock::PursuingHandoffUnavailable);
    };
    let program = behavior_program(TYPE47_GUARD_BEHAVIOR_CLASS_ID)
        .ok_or(OrdinaryType47SchedulerProductionBlock::PursuingHandoffUnavailable)?;
    let ActiveBehaviorStyle::Audited(initial_style) = current.active_style() else {
        return Err(OrdinaryType47SchedulerProductionBlock::PursuingHandoffUnavailable);
    };
    if current.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || current.style_table_index_raw_at_0x10() != 0
        || initial_style.class_id != TYPE47_GUARD_BEHAVIOR_CLASS_ID as u8
        || initial_style.variant != 0
    {
        return Err(OrdinaryType47SchedulerProductionBlock::PursuingHandoffUnavailable);
    }
    if !matches!(
        entity.sub_a_propulsion_runtime,
        RetailRuntimeValue::Known(Some(_))
    ) {
        return Err(OrdinaryType47SchedulerProductionBlock::PursuingHandoffUnavailable);
    }
    let pursuing_style = *audited_behavior_style(
        TYPE47_GUARD_BEHAVIOR_CLASS_ID,
        TYPE47_GUARD_PURSUING_STYLE_INDEX as u8,
    )
    .ok_or(OrdinaryType47SchedulerProductionBlock::PursuingHandoffUnavailable)?;
    debug_assert_eq!(pursuing_style.frame_address, 0x004C_7C00);
    let owner_position_raw = entity.position_raw();
    let aim = prepare_aim_and_fire_runtime_task(
        AimAndFireRuntimeTaskPreparation {
            slot: ActorTaskSlot::Tertiary,
            constructor_address: AIM_AND_FIRE_CONSTRUCTOR_ADDRESS,
            tick_address: AIM_AND_FIRE_TICK_ADDRESS,
            lifetime_ms: AIM_AND_FIRE_LIFETIME_MS,
            target_id,
        },
        entity.id,
        owner_position_raw,
        0,
        0,
        metadata,
    )
    .map_err(|_| OrdinaryType47SchedulerProductionBlock::PursuingHandoffUnavailable)?;
    let chase = ChaseTargetTaskState::prepare_after_allocation(
        entity.id,
        owner_position_raw,
        target_id,
        metadata,
    )
    .map_err(|_| OrdinaryType47SchedulerProductionBlock::PursuingHandoffUnavailable)?
    .map_task(ActorTaskRuntime::ChaseTarget);

    let with_target = BehaviorContextRuntime::named_audited(
        program,
        0,
        current.choice_list_source(),
        RetailRuntimeValue::Known(Some(target_id)),
        current.auxiliary_word_at_0x0c(),
        initial_style,
    )
    .ok_or(OrdinaryType47SchedulerProductionBlock::PursuingHandoffUnavailable)?;
    let pursuing = BehaviorContextRuntime::named_audited(
        program,
        TYPE47_GUARD_PURSUING_STYLE_INDEX,
        with_target.choice_list_source(),
        with_target.target_handle_at_0x08(),
        with_target.auxiliary_word_at_0x0c(),
        pursuing_style,
    )
    .ok_or(OrdinaryType47SchedulerProductionBlock::PursuingHandoffUnavailable)?;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(pursuing));

    let Entity {
        actor_tasks,
        sub_a_propulsion_runtime,
        ..
    } = entity;
    let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
        return Err(OrdinaryType47SchedulerProductionBlock::PursuingHandoffUnavailable);
    };
    let aim_task = aim.apply_suffix(|_, suffix| {
        debug_assert!(!suffix.enables_sub_f());
    });
    actor_tasks.replace_prepared(ActorTaskSlot::Tertiary, aim_task);
    actor_tasks.clear_slot(ActorTaskSlot::Secondary);
    let chase_task = chase.apply_suffix(|_, suffix| {
        debug_assert!(!suffix.enables_sub_f());
        if let Some(speed) = suffix.sub_a_target_speed_raw() {
            sub_a.apply_shared_initializer_target_speed_write(speed);
        }
    });
    actor_tasks.replace_prepared(ActorTaskSlot::Primary, chase_task);
    Ok(())
}

fn type47_chase_body_basis(
    entity: &Entity,
) -> (
    RetailRuntimeValue<[i32; 3]>,
    RetailRuntimeValue<[i32; 3]>,
    RetailRuntimeValue<[i32; 3]>,
) {
    match entity.physical_body_basis_q31 {
        RetailRuntimeValue::Known(Type9BodyBasis {
            lateral,
            forward,
            up,
        }) => (
            RetailRuntimeValue::Known(lateral),
            RetailRuntimeValue::Known(forward),
            RetailRuntimeValue::Known(up),
        ),
        RetailRuntimeValue::Unresolved => (
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
        ),
    }
}

fn commit_type47_chase_pose(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
    terrain: Option<&TerrainGrid>,
    model_records: Option<&[[i16; 4]]>,
    outcome: Type47ChaseMoverOutcome,
    elapsed_micros: u32,
) {
    entity.set_heading_raw(outcome.heading_raw);
    let [heading_raw, pitch_raw, roll_raw] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
        heading_raw,
        pitch_raw,
        roll_raw,
    ));
    // FUN_0040E870 / FUN_0040DCA0 after A800: E100 then optional DF70, then
    // 12DA0. Guard styles 0x004C7BB8 / 0x004C7C00 have +0x34/+0x38 zero, so
    // effective flags are type +0xC0 `0x2039`. Bit 0x04 clear runs gravity,
    // bit 0x08 runs mode-zero drag, bit 0x02 clear skips DF70. Sub-C lift
    // without that E100 suffix lets 12DA0 integrate Y into the sky; type-13
    // WorldSurface still plants the moving stain on the ground/water.
    let mut velocity_raw = outcome.velocity_raw;
    apply_first_world_common_dying_environment_raw(
        &mut velocity_raw,
        elapsed_micros,
        entity.mass_raw.max(1),
    );
    entity.set_motion_raw(outcome.position_raw, velocity_raw);
    if let RetailRuntimeValue::Known(Some(sub_a)) = outcome.sub_a_runtime {
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(sub_a));
    }
    if let RetailRuntimeValue::Known(bits) = entity
        .collision
        .state_flags_at_0x08
        .masked(COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)
    {
        commit_common_master_motion(
            entity,
            plan_common_master_motion(
                entity.position_raw(),
                entity.velocity_raw(),
                bits,
                elapsed_micros,
            ),
        );
    }
    // FUN_0041D360 writes PRIMARY_CACHE_VALID / TERRAIN_OFFSET_VALID on the
    // live 0x3C-byte record. Those bits alias COPY_TARGET / WAIT_FOR_DEPS
    // in FUN_0041D0A0, so a rest-pose constructor (flags 0) can start a
    // stride. Presentation still copies the record; this commit is the
    // writer's live input.
    let Some(model_records) = model_records else {
        return;
    };
    let Some(metadata) = metadata else {
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

fn apply_type47_chase_mover(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
    terrain: Option<&TerrainGrid>,
    target_private: &mut WanderNearPrivateState,
    tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    elapsed_micros: u32,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Type47ChaseMoverOutcome, Type47ChaseMoverBlock> {
    let position_raw = entity.position_raw();
    let velocity_raw = entity.velocity_raw();
    let heading_raw = entity.heading_raw();
    let roll_raw = entity.rotation_heading_pitch_roll_raw()[2] as u16;
    let sub_a_runtime = entity.sub_a_propulsion_runtime;
    let sub_d_stagger_seed = entity
        .ordinary_type47_aim_and_fire_runtime
        .as_ref()
        .map(|runtime| runtime.sub_d_stagger_seed());
    let (body_right_q31, body_forward_q31, body_up_q31) = type47_chase_body_basis(entity);
    let (sub_d_frame_owner, sub_d_runtime) =
        match entity.ordinary_type47_aim_and_fire_runtime.as_mut() {
            Some(runtime) => runtime.sub_d_pair_mut(),
            None => (None, None),
        };
    let sub_h_runtime = match &mut entity.sub_h_external_frame_runtime {
        RetailRuntimeValue::Known(Some(runtime)) => Some(runtime),
        _ => None,
    };
    evaluate_type47_chase_common_mover(
        Type47ChaseMoverRequest {
            entity_id: entity.id,
            entity_type: ORDINARY_TYPE47_ENTITY_TYPE,
            metadata,
            position_raw,
            velocity_raw,
            heading_raw,
            roll_raw,
            sub_a_runtime,
            sub_d_stagger_seed,
            sub_d_frame_owner,
            sub_d_runtime,
            body_right_q31,
            body_forward_q31,
            body_up_q31,
            sub_h_runtime,
            terrain,
            global_elapsed_micros: elapsed_micros,
            target_private,
            tracked_target,
            elapsed_micros,
        },
        next_random,
    )
}

pub(crate) fn type47_guard_entity_ref(entity: &Entity) -> GuardLocationEntityRef {
    GuardLocationEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common_mover::type9_attitude::Type9BodyBasis;
    use crate::common_mover::SubAPropulsionRuntime;
    use crate::entity::EntityKind;
    use crate::entity_behavior::ActiveBehaviorStyle;
    use crate::entity_collision_state::{
        EntityInitializerSpec, EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord,
        SURFACE_STATE_MASK,
    };
    use crate::guard_location_owner::acquisition::{
        GuardLocationAcquisitionCallbackPrefix, GuardLocationAcquisitionCallbackResult,
        GuardLocationAcquisitionZeroReason,
    };
    use crate::ordinary_type47_death_live::{
        TYPE47_COMMON_DYING_BEHAVIOR_CHOICES, TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR,
        TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR, TYPE47_COMMON_DYING_SUB_B_DESCRIPTOR,
        TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR, TYPE47_COMMON_DYING_SUB_H_RECORDS,
    };
    use crate::ordinary_type47_live::{
        admit_fresh_level1_ordinary_type47, FreshLevel1OrdinaryType47SpawnFacts,
        FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY,
        FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR,
        FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT,
    };
    use crate::sub_h_external_frame::SubHRuntimeState;
    use crate::type47_c690::{
        TYPE47_C690_SUPPRESS_STATE_BIT, TYPE47_GUARD_PURSUING_PLUS_00,
        TYPE47_GUARD_PURSUING_PLUS_04, TYPE47_GUARD_VARIANT0_PLUS_04,
    };
    use crate::type47_initial_behavior_live::{
        plan_fresh_type47_initial_behavior, publish_fresh_type47_initial_behavior,
    };
    use crate::wander_near_location::WanderNearRetarget;
    use v2k_formats::collision::SubHExternalFrameDescriptor;
    use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};

    const ENTITY_ID: u32 = 0x042F_000B;
    const PLAYER_ID: u32 = 0x047F_0001;
    const PLAYER_NEARBY_CAPABILITY: u32 = 0x0000_0001;

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID as u16; 4],
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR,
            )),
            sub_b_lateral_descriptor: RetailRuntimeValue::Known(Some(
                TYPE47_COMMON_DYING_SUB_B_DESCRIPTOR,
            )),
            sub_c_lift_descriptor: RetailRuntimeValue::Known(Some(
                TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR,
            )),
            sub_h_external_frame_descriptor: RetailRuntimeValue::Known(Some(
                SubHExternalFrameDescriptor {
                    completion_sound_id: None,
                    records: TYPE47_COMMON_DYING_SUB_H_RECORDS.to_vec(),
                },
            )),
            projectile_emitter_descriptor: RetailRuntimeValue::Known(Some(
                FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR,
            )),
            common_mover_topology: RetailRuntimeValue::Known(
                FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY,
            ),
            sub_d_steering_descriptor: RetailRuntimeValue::Known(Some(
                crate::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D,
            )),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: 0x2039,
                common_axis_descriptor: TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR,
                behavior_choices: TYPE47_COMMON_DYING_BEHAVIOR_CHOICES
                    .to_vec()
                    .into_boxed_slice(),
                behavior_rule_ref: 0,
                alternate_behavior_class_ref: 12,
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn publish_guard(spawn_index: usize) -> (EntityManager, FreshType47InitialBehaviorPublication) {
        let metadata = exact_metadata();
        let mut entity = Entity::unresolved_port_entity(ENTITY_ID, EntityKind::Enemy, 47);
        entity.authored_spawn_index = Some(spawn_index);
        entity.model_slots = [Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID); 4];
        entity.model_index = Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID);
        entity.actor_common_axis_descriptor =
            RetailRuntimeValue::Known(TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR);
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(777), -1, 100),
        ));
        entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(
            SubHRuntimeState::new(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT).unwrap(),
        ));
        entity.physical_body_basis_q31 =
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0, 0, 0));
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0x2039);
        let mut world_fx = WorldFx::new();
        let weighted =
            plan_fresh_type47_initial_behavior(&entity, &metadata, &mut world_fx).unwrap();
        entity.initial_behavior = RetailRuntimeValue::Known(Some(weighted.selection));
        let publication =
            publish_fresh_type47_initial_behavior(&mut entity, &metadata, weighted, &mut world_fx)
                .unwrap();
        assert!(matches!(
            publication.initializer,
            FreshType47InitializerPublication::GuardLocation { .. }
        ));
        let mut manager = EntityManager::from_entities_for_test(vec![entity]);
        manager.retain_fresh_level1_type47_initial_production(publication);
        (manager, publication)
    }

    fn in_range_player() -> Entity {
        let mut player = Entity::unresolved_port_entity(PLAYER_ID, EntityKind::Player, 46);
        player.capability_flags = PLAYER_NEARBY_CAPABILITY;
        // Fresh type-46 construction knows 0x06078805 and leaves only
        // SURFACE_STATE_MASK unknown. FUN_00403490 still sees a live dword.
        player.collision.state_flags_at_0x08 =
            RetailStateWord::from_known_bits(0x0607_8805, !SURFACE_STATE_MASK);
        player.set_motion_raw([1_000, 0, 0], [0; 3]);
        player
    }

    fn publish_guard_with_in_range_player(
        spawn_index: usize,
    ) -> (EntityManager, FreshType47InitialBehaviorPublication) {
        let metadata = exact_metadata();
        let mut entity = Entity::unresolved_port_entity(ENTITY_ID, EntityKind::Enemy, 47);
        entity.authored_spawn_index = Some(spawn_index);
        entity.model_slots = [Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID); 4];
        entity.model_index = Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID);
        entity.actor_common_axis_descriptor =
            RetailRuntimeValue::Known(TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR);
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(777), -1, 100),
        ));
        entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(
            SubHRuntimeState::new(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT).unwrap(),
        ));
        entity.physical_body_basis_q31 =
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0, 0, 0));
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0x2039);
        entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        entity.set_motion_raw([0, 0, 0], [10, 0, 0]);
        entity.set_rotation_heading_pitch_roll_raw([0; 3]);
        entity.ordinary_type47_aim_and_fire_runtime = Some(
            admit_fresh_level1_ordinary_type47(
                FreshLevel1OrdinaryType47SpawnFacts {
                    retail_first_world: true,
                    authored_spawn_index: spawn_index,
                    entity_type: ORDINARY_TYPE47_ENTITY_TYPE,
                    active_model: Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID),
                },
                Some(&metadata),
            )
            .unwrap()
            .aim_and_fire_runtime(),
        );
        let mut world_fx = WorldFx::new();
        let weighted =
            plan_fresh_type47_initial_behavior(&entity, &metadata, &mut world_fx).unwrap();
        entity.initial_behavior = RetailRuntimeValue::Known(Some(weighted.selection));
        let publication =
            publish_fresh_type47_initial_behavior(&mut entity, &metadata, weighted, &mut world_fx)
                .unwrap();
        assert!(matches!(
            publication.initializer,
            FreshType47InitializerPublication::GuardLocation { .. }
        ));
        let mut table = vec![EntityTypeRuntimeMetadata::default(); 48];
        table[47] = metadata;
        let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![entity, in_range_player()],
            table,
            true,
        );
        manager.retain_fresh_level1_type47_initial_production(publication);
        (manager, publication)
    }

    fn assert_pursuing_graph(entity: &Entity, target_id: u32) {
        assert_eq!(
            entity
                .actor_task_state(ActorTaskSlot::Primary)
                .map(ActorTaskRuntime::family),
            Some(ActorTaskRuntimeFamily::ChaseTarget)
        );
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Secondary), None);
        assert_eq!(
            entity
                .actor_task_state(ActorTaskSlot::Tertiary)
                .map(ActorTaskRuntime::family),
            Some(ActorTaskRuntimeFamily::AimAndFire)
        );
        let Some(ActorTaskRuntime::ChaseTarget(chase)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("Pursuing Primary is Chase")
        };
        assert_eq!(chase.target_id(), target_id);
        let Some(ActorTaskRuntime::AimAndFire(aim)) =
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        else {
            panic!("Pursuing Tertiary is Aim")
        };
        assert_eq!(aim.private_state().target_entity_id(), target_id);
        assert_eq!(aim.private_state().optional_sound_id_raw(), 0);
        assert_eq!(aim.private_state().sound_period_us_raw(), 0);
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("Pursuing context is published")
        };
        assert!(matches!(
            context.active_style(),
            ActiveBehaviorStyle::Audited(style)
                if style.class_id == 32 && style.variant == 1
        ));
        assert_eq!(
            context.style_table_index_raw_at_0x10(),
            TYPE47_GUARD_PURSUING_STYLE_INDEX
        );
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(target_id))
        );
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("Pursuing Sub-A remains Known")
        };
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(400));
    }

    fn tick_with_random(
        manager: &mut EntityManager,
        owner: OrdinaryType47SchedulerOwner,
        random: u32,
    ) -> OrdinaryType47SchedulerOwnerTick {
        tick_with_randoms(manager, owner, &[1, random])
    }

    fn tick_with_randoms(
        manager: &mut EntityManager,
        owner: OrdinaryType47SchedulerOwner,
        randoms: &[u32],
    ) -> OrdinaryType47SchedulerOwnerTick {
        let mut world_fx = WorldFx::new();
        let mut words = randoms.iter().copied();
        let terrain = flat_terrain(0);
        tick_ordinary_type47_scheduler_owner(
            manager,
            owner,
            &mut world_fx,
            Some(&terrain),
            None,
            19_500,
            &mut |_: &mut WorldFx| words.next().expect("scripted Type-47 RNG word"),
        )
    }

    #[test]
    fn adopt_ticks_guard_acquisition_chance_reject_without_pursuing() {
        let (mut manager, publication) = publish_guard(11);
        let owner = OrdinaryType47SchedulerOwner::adopt(&manager, publication).unwrap();
        let tick = tick_with_random(&mut manager, owner, 1);
        assert!(matches!(
            tick.outcome,
            OrdinaryType47SchedulerProductionOutcome::Acquisition {
                entity_id: ENTITY_ID,
                prefix: GuardLocationAcquisitionCallbackPrefix::ChanceRejected { random_low16: 1 },
                result: GuardLocationAcquisitionCallbackResult::Zero(
                    GuardLocationAcquisitionZeroReason::ChanceRejected { random_low16: 1 },
                ),
                wander: Type47WanderVisit {
                    retarget: WanderNearRetarget::Retained,
                    ..
                },
                same_pass_aim: None,
            }
        ));
        assert!(tick.retained_owner.is_some());
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("Guard birth retains its context")
        };
        assert!(matches!(
            context.active_style(),
            ActiveBehaviorStyle::Audited(style) if style.class_id == 32 && style.variant == 0
        ));
    }

    #[test]
    fn adopt_ticks_guard_acquisition_walk_without_installing_pursuing() {
        let (mut manager, publication) = publish_guard(12);
        let owner = OrdinaryType47SchedulerOwner::adopt(&manager, publication).unwrap();
        let tick = tick_with_random(&mut manager, owner, 0);
        assert!(matches!(
            tick.outcome,
            OrdinaryType47SchedulerProductionOutcome::Acquisition {
                entity_id: ENTITY_ID,
                prefix: GuardLocationAcquisitionCallbackPrefix::Acquire { .. },
                result: GuardLocationAcquisitionCallbackResult::Zero(
                    GuardLocationAcquisitionZeroReason::SelectorTagConsumed { .. },
                ),
                wander: Type47WanderVisit {
                    retarget: WanderNearRetarget::Retained,
                    ..
                },
                same_pass_aim: None,
            }
        ));
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("Guard birth retains its context")
        };
        assert!(matches!(
            context.active_style(),
            ActiveBehaviorStyle::Audited(style) if style.class_id == 32 && style.variant == 0
        ));
    }

    fn flat_terrain(height: i8) -> TerrainGrid {
        TerrainGrid {
            header: [-1 << 8, 0, 0, 0, 0],
            cells: vec![
                TerrainCell {
                    height: height as u8,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    #[test]
    fn type47_d4a0_anchor_follows_shared_c0_bit_not_type9_latch() {
        let authored = [100, 200, 300];
        let terrain = flat_terrain(6);
        let snapped = [100, (6i16 << 5) + 50, 300];
        assert_eq!(
            crate::entity::type47_d4a0_immutable_anchor_raw(None, authored, Some(&terrain)),
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(
            crate::entity::type47_d4a0_immutable_anchor_raw(
                Some(&exact_metadata()),
                authored,
                None
            ),
            RetailRuntimeValue::Known(authored)
        );
        assert_eq!(
            crate::entity::type47_d4a0_immutable_anchor_raw(
                Some(&exact_metadata()),
                authored,
                Some(&terrain)
            ),
            RetailRuntimeValue::Known(snapped)
        );
        let mut no_snap = exact_metadata();
        no_snap
            .initializer
            .as_mut()
            .unwrap()
            .initializer_state_flags_raw = 0x2019;
        assert_eq!(
            crate::entity::type47_d4a0_immutable_anchor_raw(
                Some(&no_snap),
                authored,
                Some(&terrain)
            ),
            RetailRuntimeValue::Known(authored)
        );
    }

    #[test]
    fn guard_wander_retargets_from_d4a0_birth_anchor() {
        let (mut manager, publication) = publish_guard(11);
        let terrain = flat_terrain(6);
        let metadata = exact_metadata();
        manager
            .entity_mut_for_test(ENTITY_ID)
            .unwrap()
            .type47_immutable_anchor_raw_at_0x90 = crate::entity::type47_d4a0_immutable_anchor_raw(
            Some(&metadata),
            [100, 200, 300],
            Some(&terrain),
        );
        let owner = OrdinaryType47SchedulerOwner::adopt(&manager, publication).unwrap();
        let tick = tick_with_randoms(&mut manager, owner, &[0, 0, 0, 1]);
        assert!(matches!(
            tick.outcome,
            OrdinaryType47SchedulerProductionOutcome::Acquisition {
                wander: Type47WanderVisit {
                    retarget: WanderNearRetarget::Replaced {
                        target_position_raw: [-924, 242, -724],
                    },
                    ..
                },
                same_pass_aim: None,
                ..
            }
        ));
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("Guard Primary remains Wander");
        };
        assert_eq!(state.private_state().target_position_raw, [-924, 242, -724]);
    }

    #[test]
    fn guard_wander_passing_gate_blocks_when_d4a0_anchor_is_unresolved() {
        let (mut manager, publication) = publish_guard(11);
        assert_eq!(
            manager
                .entity_mut_for_test(ENTITY_ID)
                .unwrap()
                .type47_immutable_anchor_raw_at_0x90,
            RetailRuntimeValue::Unresolved
        );
        let owner = OrdinaryType47SchedulerOwner::adopt(&manager, publication).unwrap();
        let tick = tick_with_randoms(&mut manager, owner, &[0, 1]);
        assert!(matches!(
            tick.outcome,
            OrdinaryType47SchedulerProductionOutcome::Acquisition {
                wander: Type47WanderVisit {
                    retarget: WanderNearRetarget::Replaced {
                        target_position_raw: [0, 0, 0],
                    },
                    result: Type47WanderVisitResult::AnchorUnresolved,
                    ..
                },
                same_pass_aim: None,
                ..
            }
        ));
    }

    #[test]
    fn adopt_rejects_when_the_live_graph_is_cleared() {
        let (mut manager, publication) = publish_guard(12);
        manager
            .entity_mut_for_test(ENTITY_ID)
            .unwrap()
            .actor_tasks
            .clear_slot(ActorTaskSlot::Secondary);
        assert_eq!(
            OrdinaryType47SchedulerOwner::adopt(&manager, publication),
            Err(OrdinaryType47SchedulerAdoptionError::GraphUnavailable {
                entity_id: ENTITY_ID
            })
        );
    }

    #[test]
    fn class32_style_table_keeps_c7d0_handoff_and_ade0_initializer() {
        let pursuing = audited_behavior_style(32, 1).expect("class-32 variant 1 is audited");
        assert_eq!(pursuing.frame_address, 0x004C_7C00);
        assert_eq!(pursuing.class_id, 32);
        assert_eq!(pursuing.variant, 1);
        assert_eq!(TYPE47_GUARD_VARIANT0_PLUS_04, 0x0040_C7D0);
        assert_eq!(TYPE47_GUARD_PURSUING_PLUS_00, 0x0040_C690);
        assert_eq!(TYPE47_GUARD_PURSUING_PLUS_04, 0x0040_C690);
        assert_eq!(pursuing.impact_callback_address, Some(0x0040_C690));
    }

    #[test]
    fn adopt_ticks_guard_acquisition_handoff_to_pursuing_for_spawn_11() {
        let (mut manager, publication) = publish_guard_with_in_range_player(11);
        let owner = OrdinaryType47SchedulerOwner::adopt(&manager, publication).unwrap();
        let tick = tick_with_random(&mut manager, owner, 0);
        assert!(matches!(
            tick.outcome,
            OrdinaryType47SchedulerProductionOutcome::Acquisition {
                entity_id: ENTITY_ID,
                prefix: GuardLocationAcquisitionCallbackPrefix::Acquire { .. },
                result: GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted {
                    candidate,
                    ..
                },
                wander: Type47WanderVisit {
                    retarget: WanderNearRetarget::Retained,
                    ..
                },
                same_pass_aim: Some(Ok(_)),
            } if candidate.id == PLAYER_ID
        ));
        let owner = tick.retained_owner.expect("Pursuing owner stays live");
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert_pursuing_graph(entity, PLAYER_ID);
        assert!(bind_fresh_level1_ordinary_type47_aim_and_fire(entity).is_ok());
        assert!(
            entity
                .ordinary_type47_aim_and_fire_runtime
                .as_ref()
                .and_then(|runtime| runtime.sub_d_frame_owner())
                .is_some_and(|owner| matches!(
                    owner.classifier_cache().origin(),
                    RetailRuntimeValue::Known(_)
                )),
            "Guard wander must apply the V200003 first query before Pursuing"
        );

        let tick = tick_with_random(&mut manager, owner, 0);
        assert!(matches!(
            tick.outcome,
            OrdinaryType47SchedulerProductionOutcome::Pursuing {
                entity_id: ENTITY_ID,
                chase: Type47ChaseVisit {
                    result: Type47ChaseVisitResult::Continue,
                    ..
                },
                aim: Some(Ok(_)),
                post_chase_plus00_c690: None,
                post_aim_plus04_c690: None,
                post_aim_plus00_c690: None,
                ..
            }
        ));
        assert!(tick.retained_owner.is_some());
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        let Some(ActorTaskRuntime::ChaseTarget(chase)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("Pursuing Primary remains Chase")
        };
        assert_eq!(chase.elapsed_ms(), 19);
    }

    #[test]
    fn pursuing_graph_rejects_a_context_target_that_differs_from_chase_and_aim() {
        let (mut manager, publication) = publish_guard_with_in_range_player(11);
        let owner = OrdinaryType47SchedulerOwner::adopt(&manager, publication).unwrap();
        let tick = tick_with_random(&mut manager, owner, 0);
        assert!(tick.retained_owner.is_some());
        let entity = manager.entity_mut_for_test(ENTITY_ID).unwrap();
        assert!(type47_guard_pursuing_graph_contract_authenticates(entity));

        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("Pursuing context is published")
        };
        let program = behavior_program(TYPE47_GUARD_BEHAVIOR_CLASS_ID).unwrap();
        let style = audited_behavior_style(
            TYPE47_GUARD_BEHAVIOR_CLASS_ID,
            TYPE47_GUARD_PURSUING_STYLE_INDEX as u8,
        )
        .unwrap();
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(
            BehaviorContextRuntime::named_audited(
                program,
                TYPE47_GUARD_PURSUING_STYLE_INDEX,
                context.choice_list_source(),
                RetailRuntimeValue::Known(Some(PLAYER_ID.wrapping_add(1))),
                context.auxiliary_word_at_0x0c(),
                *style,
            )
            .unwrap(),
        ));
        assert!(!type47_guard_pursuing_graph_contract_authenticates(entity));
    }

    #[test]
    fn adopt_ticks_guard_acquisition_handoff_to_pursuing_for_spawn_13() {
        let (mut manager, publication) = publish_guard_with_in_range_player(13);
        let owner = OrdinaryType47SchedulerOwner::adopt(&manager, publication).unwrap();
        let tick = tick_with_random(&mut manager, owner, 0);
        assert!(matches!(
            tick.outcome,
            OrdinaryType47SchedulerProductionOutcome::Acquisition {
                entity_id: ENTITY_ID,
                prefix: GuardLocationAcquisitionCallbackPrefix::Acquire { .. },
                result: GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted {
                    candidate,
                    ..
                },
                wander: Type47WanderVisit {
                    retarget: WanderNearRetarget::Retained,
                    ..
                },
                same_pass_aim: Some(Ok(_)),
            } if candidate.id == PLAYER_ID
        ));
        let owner = tick
            .retained_owner
            .expect("spawn-13 Pursuing owner stays live");
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert_pursuing_graph(entity, PLAYER_ID);
        assert!(bind_fresh_level1_ordinary_type47_aim_and_fire(entity).is_ok());

        let tick = tick_with_random(&mut manager, owner, 0);
        assert!(matches!(
            tick.outcome,
            OrdinaryType47SchedulerProductionOutcome::Pursuing {
                entity_id: ENTITY_ID,
                chase: Type47ChaseVisit {
                    result: Type47ChaseVisitResult::Continue,
                    ..
                },
                aim: Some(Ok(_)),
                post_chase_plus00_c690: None,
                post_aim_plus04_c690: None,
                post_aim_plus00_c690: None,
                ..
            }
        ));
        assert!(tick.retained_owner.is_some());
    }

    #[test]
    fn pursuing_chase_tags_invalid_target_without_inventing_a_mover_return() {
        let (mut manager, publication) = publish_guard_with_in_range_player(11);
        let owner = OrdinaryType47SchedulerOwner::adopt(&manager, publication).unwrap();
        let tick = tick_with_random(&mut manager, owner, 0);
        let owner = tick.retained_owner.expect("handoff retains the owner");
        manager.entity_mut_for_test(PLAYER_ID).unwrap().active = false;

        let tick = tick_with_random(&mut manager, owner, 0);
        assert!(matches!(
            tick.outcome,
            OrdinaryType47SchedulerProductionOutcome::Pursuing {
                entity_id: ENTITY_ID,
                chase: Type47ChaseVisit {
                    result: Type47ChaseVisitResult::Tagged(
                        ChaseTargetTaggedSingleton::InvalidTarget
                    ),
                    ..
                },
                post_chase_plus00_c690: Some(Type47SchedulerC690Transition::GuardPublished { .. }),
                post_chase_guard: Some(Type47PostChaseGuardPass {
                    result: GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted { .. },
                    same_pass_aim: Some(_),
                    ..
                }),
                aim: None,
                post_aim_plus04_c690: None,
                post_aim_plus00_c690: None,
            }
        ));
        assert!(tick.retained_owner.is_some());
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        // The same-pass guard callback uses retail state flags, so clearing only
        // port-side `active` lets it reacquire this otherwise eligible player.
        assert_pursuing_graph(entity, PLAYER_ID);
    }

    #[test]
    fn spawn_13_chase_tag_0x9c01_applies_plus00_before_later_slots() {
        let (mut manager, publication) = publish_guard_with_in_range_player(13);
        let owner = OrdinaryType47SchedulerOwner::adopt(&manager, publication).unwrap();
        let tick = tick_with_random(&mut manager, owner, 0);
        let owner = tick.retained_owner.expect("handoff retains the owner");
        manager.entity_mut_for_test(PLAYER_ID).unwrap().active = false;

        let tick = tick_with_random(&mut manager, owner, 0);
        assert!(matches!(
            tick.outcome,
            OrdinaryType47SchedulerProductionOutcome::Pursuing {
                entity_id: ENTITY_ID,
                chase: Type47ChaseVisit {
                    result: Type47ChaseVisitResult::Tagged(
                        ChaseTargetTaggedSingleton::InvalidTarget
                    ),
                    ..
                },
                post_chase_plus00_c690: Some(Type47SchedulerC690Transition::GuardPublished { .. }),
                post_chase_guard: Some(Type47PostChaseGuardPass {
                    result: GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted { .. },
                    same_pass_aim: Some(_),
                    ..
                }),
                aim: None,
                post_aim_plus04_c690: None,
                post_aim_plus00_c690: None,
            }
        ));
        assert!(tick.retained_owner.is_some());
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert_pursuing_graph(entity, PLAYER_ID);
    }

    #[test]
    fn plus00_and_plus04_c690_are_suppressed_when_state_bit_0x1000_is_set() {
        let (mut manager, publication) = publish_guard_with_in_range_player(11);
        let owner = OrdinaryType47SchedulerOwner::adopt(&manager, publication).unwrap();
        let tick = tick_with_random(&mut manager, owner, 0);
        let owner = tick.retained_owner.expect("handoff retains the owner");
        manager.entity_mut_for_test(PLAYER_ID).unwrap().active = false;
        manager
            .entity_mut_for_test(ENTITY_ID)
            .unwrap()
            .collision
            .state_flags_at_0x08 = RetailStateWord::exact(0x2039 | TYPE47_C690_SUPPRESS_STATE_BIT);

        let tick = tick_with_random(&mut manager, owner, 0);
        assert!(matches!(
            tick.outcome,
            OrdinaryType47SchedulerProductionOutcome::Pursuing {
                post_chase_plus00_c690: Some(
                    Type47SchedulerC690Transition::SuppressedByEntityState {
                        slot: Type47C690Slot::Plus00
                    }
                ),
                aim: Some(Ok(_)),
                post_aim_plus04_c690: Some(
                    Type47SchedulerC690Transition::SuppressedByEntityState {
                        slot: Type47C690Slot::Plus04
                    }
                ),
                ..
            }
        ));
        assert!(tick.retained_owner.is_some());
    }
}
