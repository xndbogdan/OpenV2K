//! Source-owned Type42 rocket and Type59 grenade/depth-charge programs.
//!
//! These share launch and common-body machinery, while retaining distinct
//! class22/class35 tasks and class1/class49 terminal policies. Guided missiles
//! (class28) require their own owner.

pub mod contact;
pub mod grenade;
pub mod impact;
pub mod launch;
pub mod production;
pub mod rocket;

#[cfg(test)]
mod alpine_hits_tests;
#[cfg(test)]
mod recorded_motion_tests;

use crate::{
    actor_task_owner::{ActorTaskId, ActorTaskSlot},
    entity::{Entity, EntityManager},
    guard_location_owner::acquisition::GuardLocationSearchContext,
    main_base_abort::MainBaseAbortActorLease,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityWeaponKind {
    Grenade,
    DepthCharge,
    Rocket,
}

impl EntityWeaponKind {
    pub const fn from_selector(selector: u32) -> Option<Self> {
        match selector {
            3 => Some(Self::Grenade),
            4 => Some(Self::DepthCharge),
            5 => Some(Self::Rocket),
            _ => None,
        }
    }
    pub const fn entity_type(self) -> usize {
        match self {
            Self::Rocket => 42,
            Self::Grenade | Self::DepthCharge => 59,
        }
    }
    pub const fn launch_speed_raw(self) -> i32 {
        match self {
            Self::Grenade => 2000,
            Self::DepthCharge => 1000,
            Self::Rocket => 1,
        }
    }
}

/// A modeled 44E770 request, after the transient's shooter words are resolved.
/// Rotation is copied by14870 from the shooter's current Euler words; the
/// barrel direction changes velocity independently.
#[derive(Debug, Clone, Copy)]
pub struct EntityWeaponConstructionRequest {
    pub kind: EntityWeaponKind,
    pub source_actor_id: u32,
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub rotation_raw: [i16; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityWeaponBlock {
    SourceAllocation,
    Metadata,
    Terrain,
    Allocation,
    Graph,
    TaskChanged,
    Runtime(&'static str),
}

/// Constructor-owned custody; a coincidental type/model never grants it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeEntityWeaponRuntime {
    pub(crate) allocation: MainBaseAbortActorLease,
    pub(crate) kind: EntityWeaponKind,
    pub(crate) search_context: Option<GuardLocationSearchContext>,
    pub(crate) model_effect_bits_at_0x84: u16,
}

/// Exact live allocation and published task wrappers, retained by13500.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeEntityWeaponOwner {
    pub(crate) actor: MainBaseAbortActorLease,
    pub(crate) kind: EntityWeaponKind,
    pub(crate) task_ids: [Option<ActorTaskId>; 3],
}

impl NativeEntityWeaponOwner {
    pub const fn entity_id(self) -> u32 {
        self.actor.entity_id
    }
    pub const fn kind(self) -> EntityWeaponKind {
        self.kind
    }
    pub(crate) fn authenticates(self, manager: &EntityManager) -> bool {
        manager
            .main_base_abort_actor_observation(self.entity_id())
            .is_some_and(|observation| observation.lease == self.actor)
            && manager
                .iter_all()
                .find(|entity| entity.id == self.entity_id())
                .is_some_and(|entity| {
                    entity_authenticates(entity)
                        && entity.native_entity_weapon_runtime.unwrap().allocation == self.actor
                        && ActorTaskSlot::IN_RETAIL_TICK_ORDER.into_iter().all(|slot| {
                            entity.actor_tasks.task_in_slot(slot) == self.task_ids[slot as usize]
                        })
                })
    }
}

pub(crate) fn entity_authenticates(entity: &Entity) -> bool {
    entity.native_entity_weapon_runtime.is_some_and(|runtime| {
        entity.id == runtime.allocation.entity_id
            && entity.entity_type == runtime.kind.entity_type() as u32
            && entity.authored_spawn_index.is_none()
    })
}

pub(crate) fn authenticate_weapon_metadata(
    kind: EntityWeaponKind,
    metadata: &crate::entity_collision_state::EntityTypeRuntimeMetadata,
) -> Result<(), EntityWeaponBlock> {
    match kind {
        EntityWeaponKind::Rocket => rocket::authenticate_rocket_metadata(metadata),
        EntityWeaponKind::Grenade | EntityWeaponKind::DepthCharge => {
            grenade::authenticate_grenade_metadata(metadata)
                .map_err(|_| EntityWeaponBlock::Metadata)
        }
    }
}
