//! Native weapon visits and synchronous explosion custody at one13500 slot.
//!
//! Playing radial delivery needs the same retained scheduler that owns every
//! target. Temporarily reunite the pass cursor before BAF0 and repartition
//! against the current intrusive list afterwards, including newly born tails.

use super::*;
use crate::{
    class49_terminal::{
        run_class49_standard_death, Class49TerminalBlock, Class49TerminalFrame, Class49WorldContext,
    },
    native_entity_weapons::production::{
        tick_entity_weapon, EntityWeaponTickError, EntityWeaponTickFrame, EntityWeaponTickOutcome,
    },
    native_entity_weapons::{EntityWeaponBlock, EntityWeaponKind, NativeEntityWeaponOwner},
    player_hull::PlayerHull,
};

pub enum SpecializedActorTaskWorld<'a> {
    Cinematic,
    Playing {
        player_hull: &'a mut PlayerHull,
        extra_lives: RetailRuntimeValue<u8>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeWeaponRegistrationBlock {
    OwnerUnavailable,
    Conflict(SpecializedActorTaskRegistrationConflict),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeWeaponProductionBlock {
    Owned(EntityWeaponBlock),
    Terminal(Box<Class49TerminalBlock>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeWeaponProductionOutcome {
    Waiting {
        entity_id: u32,
        kind: EntityWeaponKind,
    },
    Advanced {
        entity_id: u32,
        kind: EntityWeaponKind,
        callback_elapsed_micros: u32,
        trail_attempts: u32,
    },
    Terminal {
        entity_id: u32,
        kind: EntityWeaponKind,
    },
    Blocked {
        entity_id: u32,
        kind: EntityWeaponKind,
        reason: NativeWeaponProductionBlock,
        /// The authenticated visit may have consumed RNG or committed a death
        /// prefix. Its retained host stop cannot re-enter as a fresh callback.
        prefix_committed: bool,
    },
    Dropped {
        entity_id: u32,
        kind: EntityWeaponKind,
    },
}

impl NativeWeaponProductionOutcome {
    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::Waiting { entity_id, .. }
            | Self::Advanced { entity_id, .. }
            | Self::Terminal { entity_id, .. }
            | Self::Blocked { entity_id, .. }
            | Self::Dropped { entity_id, .. } => *entity_id,
        }
    }
}

pub(super) struct NativeWeaponPassVisit<'a, 'world> {
    pub manager: &'a mut EntityManager,
    pub pending: &'a mut Vec<SpecializedActorTaskOwner>,
    pub retained: &'a mut Vec<SpecializedActorTaskOwner>,
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub static_damage: &'a mut crate::static_damage::StaticDamageScheduler,
    pub notifications: &'a mut GameplayNotifications,
    pub world: &'a mut SpecializedActorTaskWorld<'world>,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
}

impl SpecializedActorTaskScheduler {
    pub(crate) fn native_weapon_has_pending_prefix(&self, entity_id: u32) -> bool {
        self.owners.iter().any(|owner| {
            owner.entity_id() == entity_id
                && matches!(owner, SpecializedActorTaskOwner::NativeContactPrefix { .. })
        })
    }
    /// Current constructor receipt after all task wrappers have unwound.
    /// A completed class1/49 continuation has separate terminal custody and
    /// cannot manufacture this live receipt after its task slots are cleared.
    pub fn native_weapon_owner(
        &self,
        manager: &EntityManager,
        entity_id: u32,
    ) -> Option<NativeEntityWeaponOwner> {
        self.owners.iter().find_map(|owner| {
            let SpecializedActorTaskOwner::NativeWeapon(owner) = owner else {
                return None;
            };
            (owner.entity_id() == entity_id
                && owner.authenticates(manager)
                && manager
                    .iter_all()
                    .find(|entity| entity.id == entity_id)
                    .is_some_and(|entity| {
                        owner.task_ids.iter().flatten().all(|&task| {
                            entity
                                .actor_tasks
                                .wrapper_flags(task)
                                .is_some_and(|flags| flags.alive && !flags.in_callback)
                        })
                    }))
            .then_some(*owner)
        })
    }

    pub fn register_native_weapon(
        &mut self,
        manager: &EntityManager,
        owner: NativeEntityWeaponOwner,
    ) -> Result<(), NativeWeaponRegistrationBlock> {
        if !owner.authenticates(manager) {
            return Err(NativeWeaponRegistrationBlock::OwnerUnavailable);
        }
        self.register(SpecializedActorTaskOwner::NativeWeapon(owner))
            .map(|_| ())
            .map_err(|(conflict, _)| NativeWeaponRegistrationBlock::Conflict(conflict))
    }

    pub(super) fn tick_native_weapon_in_pass(
        &mut self,
        owner: NativeEntityWeaponOwner,
        visit: NativeWeaponPassVisit<'_, '_>,
    ) -> NativeWeaponProductionOutcome {
        let NativeWeaponPassVisit {
            manager,
            pending,
            retained,
            resources,
            world_fx,
            static_damage,
            notifications,
            world,
            elapsed_micros,
            retail_tick,
        } = visit;
        let entity_id = owner.entity_id();
        let kind = owner.kind();
        if !manager
            .main_base_abort_actor_observation(entity_id)
            .is_some_and(|body| body.lease == owner.actor)
        {
            return NativeWeaponProductionOutcome::Dropped { entity_id, kind };
        }

        debug_assert!(self.owners.is_empty());
        self.owners.append(pending);
        self.owners.append(retained);
        self.owners
            .push(SpecializedActorTaskOwner::NativeWeapon(owner));
        let result = tick_entity_weapon(
            manager,
            owner,
            EntityWeaponTickFrame {
                resources,
                world_fx,
                elapsed_micros,
                retail_tick,
            },
            |manager, resources, world_fx, id| {
                let world = match world {
                    SpecializedActorTaskWorld::Cinematic => Class49WorldContext::Cinematic {
                        actor_tasks: self,
                        active_terminal_calls: Vec::new(),
                    },
                    SpecializedActorTaskWorld::Playing {
                        player_hull,
                        extra_lives,
                    } => Class49WorldContext::Playing {
                        scheduler: self,
                        player_hull,
                        extra_lives: *extra_lives,
                        active_terminal_calls: Vec::new(),
                    },
                };
                run_class49_standard_death(
                    Class49TerminalFrame {
                        entities: manager,
                        resources,
                        world_fx,
                        static_damage,
                        notifications,
                        retail_tick,
                        world,
                    },
                    id,
                )
                .map(|_| ())
            },
        );

        if result.is_err() {
            // Validation may fail after the scheduler/task prefix has already
            // committed. A host stop preserves that actual allocation and
            // never reruns trail RNG or terminal dispatch on the next pass.
            self.park_native_contact_prefix(manager, entity_id);
        }
        let live_order: Vec<_> = manager.retail_live_order_ids().collect();
        let current_index = live_order.iter().position(|&id| id == entity_id);
        for retained_owner in std::mem::take(&mut self.owners) {
            let index = live_order
                .iter()
                .position(|&id| id == retained_owner.entity_id());
            if matches!((current_index, index), (Some(current), Some(index)) if index <= current) {
                retained.push(retained_owner);
            } else {
                pending.push(retained_owner);
            }
        }
        match result {
            Ok(EntityWeaponTickOutcome::Waiting) => {
                NativeWeaponProductionOutcome::Waiting { entity_id, kind }
            }
            Ok(EntityWeaponTickOutcome::Advanced {
                callback_elapsed_micros,
                trail_attempts,
            }) => NativeWeaponProductionOutcome::Advanced {
                entity_id,
                kind,
                callback_elapsed_micros,
                trail_attempts,
            },
            Ok(EntityWeaponTickOutcome::Terminal) => {
                NativeWeaponProductionOutcome::Terminal { entity_id, kind }
            }
            Err(error) => NativeWeaponProductionOutcome::Blocked {
                entity_id,
                kind,
                prefix_committed: error.prefix_committed(),
                reason: match error {
                    EntityWeaponTickError::Owned { block, .. } => {
                        NativeWeaponProductionBlock::Owned(block)
                    }
                    EntityWeaponTickError::Terminal(block) => {
                        NativeWeaponProductionBlock::Terminal(Box::new(block))
                    }
                },
            },
        }
    }
}

#[cfg(test)]
mod tests;
