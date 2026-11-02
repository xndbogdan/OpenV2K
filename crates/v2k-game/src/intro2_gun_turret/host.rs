//! The bodies that run class29's `408BD0` turret task and its `425160` fire:
//! the E/L turrets themselves, and the moving Type107/Type121 ground actors
//! whose class29 choice stops them and turrets in place. Each keeps its own
//! receipt, L outputs, E record, joint words and command FIFO.

use crate::{
    entity::{Entity, EntityManager},
    generic_projectile_emitter::{GenericEmitterAppendRequest, GenericEmitterRuntime},
};
use std::collections::VecDeque;
use v2k_formats::collision::ProjectileEmitterDescriptor;

/// A body's `4147A0` command FIFO. False when the append fails.
pub(crate) trait ShotQueue {
    fn try_append(&mut self, request: GenericEmitterAppendRequest) -> bool;
}

impl ShotQueue for VecDeque<GenericEmitterAppendRequest> {
    fn try_append(&mut self, request: GenericEmitterAppendRequest) -> bool {
        self.try_reserve(1).is_ok() && {
            self.push_back(request);
            true
        }
    }
}

impl ShotQueue for Vec<GenericEmitterAppendRequest> {
    fn try_append(&mut self, request: GenericEmitterAppendRequest) -> bool {
        self.try_reserve(1).is_ok() && {
            self.push(request);
            true
        }
    }
}

/// A `24EE0` joint write: `0xFFFF` to the bank word a binding names. Returns
/// false for a binding the body does not own.
pub(crate) type JointWriter<'a> = &'a mut dyn FnMut(u64) -> bool;

pub(crate) trait GunTurretHost {
    /// The living allocation in its issuing manager.
    fn manager_authenticates(manager: &EntityManager, id: u32) -> bool;
    /// The allocation alone.
    fn allocation_authenticates(entity: &Entity) -> bool;
    /// The emitter the allocation authenticated at birth.
    fn descriptor(entity: &Entity) -> Option<ProjectileEmitterDescriptor>;
    /// L's current outputs, which `408DF0` reads as the starting yaw/pitch.
    fn l_outputs(entity: &Entity) -> Option<[i16; 2]>;
    /// `408BD0`'s write through L's output pointers.
    fn store_l_outputs(entity: &mut Entity, outputs: [i16; 2]) -> bool;
    /// The FIFO, the E record and the joint writer together. None when the
    /// allocation or its queue custody fails.
    fn with_aim<R>(
        entity: &mut Entity,
        f: impl FnOnce(&mut dyn ShotQueue, &mut GenericEmitterRuntime, JointWriter<'_>) -> R,
    ) -> Option<R>;
    /// `44EA60`: a gravity-lead projectile method.
    fn gravity_lead(entity: &Entity) -> bool;
}
