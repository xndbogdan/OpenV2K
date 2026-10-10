//! Common 12DA0 task execution for separately authenticated ground actors.
//!
//! Type53 and Type122 share Follow/Search/Capture callback structure, not
//! construction identity. Profiles retain their actual component receipts,
//! default policy, selectors, emitter and capture-relation ownership.

pub(crate) mod behavior;
pub mod contact;
pub(crate) mod defecate;
pub mod impact;
pub(crate) mod live;
pub(crate) mod mover;
pub(crate) mod run_away;
pub(crate) mod search;
mod world;

pub use live::{
    tick_native_ground_actor, NativeGroundActorBlock, NativeGroundActorFrame,
    NativeGroundActorOutcome, NativeGroundActorOwner, NativeGroundActorTick,
};

use crate::{
    actor_task_dispatcher::SharedGenericConstructorEffect,
    actor_task_owner::ActorTaskSlot,
    common_mover::{
        component_dispatch::CommonMoverDispatchMode,
        sub_d::{Type9SubDFrameOwner, Type9SubDRuntime},
    },
    entity::{Entity, EntityManager},
    entity_behavior::{
        select_initial_behavior, BehaviorContextRuntime, BehaviorSelection, BehaviorWeightRule,
    },
    entity_collision_state::{
        CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
    gameplay_notifications::GameplayNotifications,
    guard_location_owner::acquisition::{
        select_guard_location_candidate, GuardLocationCandidateFilter,
        GuardLocationCandidateRequest, GuardLocationCandidateSelection, GuardLocationEntityRef,
        GuardLocationSearchContext,
    },
    intro2_common_dying::Intro2CommonDyingOwner,
    world_fx::WorldFx,
    wrapped_axis_range::WrappedAxisRange,
};
use std::num::NonZeroU32;
use v2k_formats::{
    collision::{BehaviorChoice, CommonAxisDescriptor},
    terrain::TerrainGrid,
};

/// Capture support is an explicit actor admission, independent of task shape.
pub enum NativeCaptureDispatch<'a> {
    NoCapture,
    AbsentJ {
        tasks: &'a mut dyn NativeGroundTaskCustody,
        notifications: &'a mut GameplayNotifications,
    },
    PursuitOnly,
    Transport {
        tasks: &'a mut dyn crate::intro2_type17::capture::CaptureTaskCustody,
        notifications: &'a mut GameplayNotifications,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCapturePolicy {
    NoCapture,
    AbsentJ,
    PursuitOnly,
    Transport,
}

/// Ground-family task ownership, including source-inline class18 children.
/// The sealed Type56 owner comes only from its real dynamic constructor.
pub trait NativeGroundTaskCustody: crate::intro2_type17::capture::CaptureTaskCustody {
    fn register_split_type56_child(
        &mut self,
        owner: crate::native_type56::Type56Owner,
    ) -> Result<(), &'static str>;
    /// Type27's class18 Type3 child, published before the next launch draw.
    fn register_split_rolling_boulder_child(
        &mut self,
        owner: crate::rolling_boulder::RollingBoulderOwner,
    ) -> Result<(), &'static str>;
}

/// Concrete death dependencies for genuine native policies. Basic quiet/common
/// death reads resources and FX; Capture retains its existing relation owner;
/// Split owns synchronous child construction/publication at the current tick.
pub enum NativeGroundDeathContext<'a> {
    Basic {
        resources: &'a crate::resource_cache::ResourceCache,
        fx: &'a mut WorldFx,
    },
    Capture {
        resources: &'a crate::resource_cache::ResourceCache,
        context: crate::intro2_type17::capture::CaptureContext<'a>,
    },
    Split {
        resources: &'a crate::resource_cache::ResourceCache,
        fx: &'a mut WorldFx,
        tick: u32,
        tasks: &'a mut dyn NativeGroundTaskCustody,
    },
}
impl NativeGroundDeathContext<'_> {
    pub fn resources(&self) -> &crate::resource_cache::ResourceCache {
        match self {
            Self::Basic { resources, .. }
            | Self::Capture { resources, .. }
            | Self::Split { resources, .. } => resources,
        }
    }
    pub fn world_fx(&mut self) -> &mut WorldFx {
        match self {
            Self::Basic { fx, .. } | Self::Split { fx, .. } => fx,
            Self::Capture { context, .. } => context.world_fx,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeGroundSubDState {
    pub origin: NativeGroundAllocationOrigin,
    pub runtime: Type9SubDRuntime,
    pub owner: Type9SubDFrameOwner,
}

/// Constructor provenance, authenticated against the current manager before
/// motion. An enum tag never grants dynamic allocation custody.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeGroundAllocationOrigin {
    Authored {
        spawn_index: usize,
    },
    Dynamic {
        allocation: crate::main_base_abort::MainBaseAbortActorLease,
    },
}
impl NativeGroundAllocationOrigin {
    pub(crate) fn authenticates(self, manager: &EntityManager, entity: &Entity) -> bool {
        match self {
            Self::Authored { spawn_index } => entity.authored_spawn_index == Some(spawn_index),
            Self::Dynamic { allocation } => {
                entity.authored_spawn_index.is_none()
                    && allocation.entity_id == entity.id
                    && manager
                        .main_base_abort_actor_observation(entity.id)
                        .is_some_and(|observation| observation.lease == allocation)
            }
        }
    }
    pub(crate) fn authenticates_entity(self, entity: &Entity) -> bool {
        match self {
            Self::Authored { spawn_index } => entity.authored_spawn_index == Some(spawn_index),
            Self::Dynamic { allocation } => {
                entity.authored_spawn_index.is_none() && allocation.entity_id == entity.id
            }
        }
    }
}

/// Completed synchronous terminal publication. Quiet death has no timed task;
/// its issuing allocation/context remain damage-addressable until14990.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeGroundDeferredDeathReceipt {
    allocation: crate::main_base_abort::MainBaseAbortActorLease,
    context: BehaviorContextRuntime,
}
impl NativeGroundDeferredDeathReceipt {
    pub(crate) const fn new(
        allocation: crate::main_base_abort::MainBaseAbortActorLease,
        context: BehaviorContextRuntime,
    ) -> Self {
        Self {
            allocation,
            context,
        }
    }
    pub const fn allocation(self) -> crate::main_base_abort::MainBaseAbortActorLease {
        self.allocation
    }
    pub const fn context(self) -> BehaviorContextRuntime {
        self.context
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeGroundTerminalPublication {
    CommonDying(Intro2CommonDyingOwner),
    Deferred(NativeGroundDeferredDeathReceipt),
}

/// Existing53/122 callbacks only read resources. Class4's coarse02850 writes
/// the terrain immediately before the remaining actor phases read it.
pub enum NativeGroundResources<'a> {
    ReadOnly(&'a crate::resource_cache::ResourceCache),
    Mutable(&'a mut crate::resource_cache::ResourceCache),
}
impl std::ops::Deref for NativeGroundResources<'_> {
    type Target = crate::resource_cache::ResourceCache;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::ReadOnly(resources) => resources,
            Self::Mutable(resources) => resources,
        }
    }
}
impl NativeGroundResources<'_> {
    pub(crate) fn apply_infection_write(
        &mut self,
        write: crate::infection_evolution::InfectionCellWrite,
    ) -> Result<(), NativeGroundActorBlock> {
        let Self::Mutable(resources) = self else {
            return Err(NativeGroundActorBlock::Runtime(
                "live class4 terrain writer",
            ));
        };
        resources
            .apply_level_infection_writes(&[write])
            .ok_or(NativeGroundActorBlock::Runtime("class4 infection terrain"))?;
        Ok(())
    }
}

// Profile operations are private to the actor adapters. The public marker is
// sealed only so existing public owner facades can preserve their concrete type.
pub(crate) mod sealed {
    use super::*;
    pub trait Sealed: Copy + std::fmt::Debug + Eq {
        const ENTITY_TYPE: u32;
        const CAPTURE_POLICY: NativeCapturePolicy;
        const DEFAULT_FLAGS: u32;
        const TOPOLOGY: CommonMoverComponentTopology;
        const AXIS: CommonAxisDescriptor;
        const CHOICES: &'static [BehaviorChoice];
        const LIVING_CLASSES: &'static [u8];
        fn graph_authenticates(_: &Entity) -> bool {
            true
        }
        fn completed_terminal_authenticates(_: &EntityManager, _: u32) -> bool {
            false
        }
        // Sealed crate-local component custody is intentionally not public API.
        fn kl_components(_: &Entity) -> Option<crate::intro2_common_mover::Intro2KlComponents> {
            None
        }
        fn store_kl_components(_: &mut Entity, _: crate::intro2_common_mover::Intro2KlComponents) {
            unreachable!("profile has no Sub-K/L allocation")
        }
        fn allocation_authenticates(entity: &Entity) -> bool;
        fn register_owner(
            scheduler: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
            owner: NativeGroundActorOwner<Self>,
        );
        fn publish_standard_death(
            manager: &mut EntityManager,
            id: u32,
            context: &mut NativeGroundDeathContext<'_>,
        ) -> Result<
            crate::live_actor_checked_damage::LiveActorDeathResult<NativeGroundTerminalPublication>,
            crate::intro2_common_dying::Intro2CommonDyingBlock,
        >;
        fn publish_direct_surface_death(
            _: &mut EntityManager,
            _: u32,
            _: &mut NativeGroundDeathContext<'_>,
        ) -> Result<
            crate::live_actor_checked_damage::LiveActorDeathResult<NativeGroundTerminalPublication>,
            crate::intro2_common_dying::Intro2CommonDyingBlock,
        > {
            Err(crate::intro2_common_dying::Intro2CommonDyingBlock::Runtime(
                "direct surface terminal custody",
            ))
        }
        fn manager_authenticates(manager: &EntityManager, id: u32) -> bool;
        fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool;
        fn sub_d_state(entity: &Entity) -> Option<NativeGroundSubDState>;
        fn store_sub_d_state(entity: &mut Entity, state: NativeGroundSubDState);
        fn publish_acquiring(
            entity: &mut Entity,
            metadata: &EntityTypeRuntimeMetadata,
            selection: BehaviorSelection,
            context: BehaviorContextRuntime,
            next_random: &mut impl FnMut() -> u32,
        ) -> bool;
        fn acquire_capture(
            manager: &mut EntityManager,
            id: u32,
            tick: u32,
            fx: &mut WorldFx,
        ) -> Result<(), NativeGroundActorBlock>;
        fn tick_capture_primary(
            manager: &mut EntityManager,
            id: u32,
            frame: mover::MoverFrame<'_>,
            fx: &mut WorldFx,
        ) -> Result<bool, NativeGroundActorBlock>;
        fn tick_aim(
            manager: &mut EntityManager,
            id: u32,
            dt: u32,
            mode: CommonMoverDispatchMode,
            fx: &mut WorldFx,
        ) -> Result<bool, NativeGroundActorBlock>;
        fn update_attachment(
            manager: &mut EntityManager,
            id: u32,
        ) -> Result<(), NativeGroundActorBlock>;
    }
}
#[doc(hidden)]
pub trait NativeGroundActorProfile: sealed::Sealed {}

pub(crate) fn candidate(entity: &Entity) -> GuardLocationEntityRef {
    GuardLocationEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle: RetailRuntimeValue::Known(None),
    }
}

pub(crate) fn nearby<P: NativeGroundActorProfile>(
    owner: GuardLocationEntityRef,
    prefix: &[GuardLocationEntityRef],
    mask: u32,
) -> Result<bool, NativeGroundActorBlock> {
    select_guard_location_candidate(GuardLocationCandidateRequest {
        owner,
        candidates_in_intrusive_order: prefix,
        search_context: GuardLocationSearchContext::new(
            WrappedAxisRange::from_raw(P::AXIS.strict_axis_limit_raw),
            GuardLocationCandidateFilter::CapabilityMask(NonZeroU32::new(mask).unwrap()),
        ),
    })
    .map(|selection| matches!(selection, GuardLocationCandidateSelection::Selected(_)))
    .map_err(|_| NativeGroundActorBlock::Runtime("nearby selector"))
}
