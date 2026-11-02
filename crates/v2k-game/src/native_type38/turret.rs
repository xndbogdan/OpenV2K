//! Type107/Type121's class29: D190 clears Secondary and Primary, so the body
//! stops steering, and runs the gun-turret task (`408BD0`) in Tertiary over
//! its own E record, L bank words and `4147A0` command FIFO.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, PreparedActorTask},
    entity_behavior::initial_behavior_state_policy,
    intro2_gun_turret::{
        host::{GunTurretHost, JointWriter, ShotQueue},
        method_has_gravity_lead,
        task::GunTurretTaskState,
    },
};

/// The mobile turrets as a gun-turret body.
pub(crate) struct Type107Turret;

impl GunTurretHost for Type107Turret {
    fn manager_authenticates(manager: &EntityManager, id: u32) -> bool {
        manager_allocation_authenticates(manager, id)
            && manager
                .iter_all()
                .find(|entity| entity.id == id)
                .is_some_and(Self::allocation_authenticates)
    }
    fn allocation_authenticates(entity: &Entity) -> bool {
        type38_row(entity).is_some_and(Type38Row::turrets)
    }
    fn descriptor(entity: &Entity) -> Option<ProjectileEmitterDescriptor> {
        type38_row(entity).map(Type38Row::emitter)
    }
    /// L's output selectors name bank words 1 and 2.
    fn l_outputs(entity: &Entity) -> Option<[i16; 2]> {
        let kl = &entity.native_type38_runtime.as_ref()?.kl_components;
        let [yaw, pitch] = kl.sub_l_output_selectors()?;
        Some([kl.model_word(yaw)?, kl.model_word(pitch)?])
    }
    fn store_l_outputs(entity: &mut Entity, outputs: [i16; 2]) -> bool {
        let Some(runtime) = entity.native_type38_runtime.as_mut() else {
            return false;
        };
        let kl = &mut runtime.kl_components;
        let Some([yaw, pitch]) = kl.sub_l_output_selectors() else {
            return false;
        };
        kl.write_model_word(yaw, outputs[0]).is_some()
            && kl.write_model_word(pitch, outputs[1]).is_some()
    }
    /// The FIFO is the Search aim's own: one E queue per allocation.
    fn with_aim<R>(
        entity: &mut Entity,
        f: impl FnOnce(&mut dyn ShotQueue, &mut GenericEmitterRuntime, JointWriter<'_>) -> R,
    ) -> Option<R> {
        let Entity {
            native_type38_runtime,
            native_type38_aim_runtime,
            ..
        } = entity;
        let Type38Runtime {
            sub_e_runtime,
            kl_components,
            ..
        } = native_type38_runtime.as_mut()?;
        let queue = native_type38_aim_runtime.as_mut()?.transient_shots_mut();
        // 24EE0 writes 0xFFFF through the bound bank word.
        let mut joint = |binding: u64| {
            u8::try_from(binding)
                .ok()
                .and_then(|selector| kl_components.write_model_word(selector, -1))
                .is_some()
        };
        Some(f(queue, sub_e_runtime, &mut joint))
    }
    fn gravity_lead(entity: &Entity) -> bool {
        type38_row(entity)
            .is_some_and(|row| method_has_gravity_lead(u32::from(row.emitter().projectile_method)))
    }
}

/// D190 for a mobile turret: clear Secondary then Primary, 4086D0 builds the
/// task from L's current outputs (408DF0), an infected body trades
/// capability 0x1004 for 8, then state |= 0x100, the Tertiary task, |= 0x28.
pub(super) fn publish_class29(
    entity: &mut Entity,
    selection: BehaviorSelection,
    context: BehaviorContextRuntime,
) -> bool {
    let Some(outputs) = Type107Turret::l_outputs(entity) else {
        return false;
    };
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Primary);
    let state = GunTurretTaskState::from_408df0(
        entity.capability_flags,
        outputs,
        Type107Turret::gravity_lead(entity),
    );
    if entity.collision.state_flags_at_0x08.masked(0x2000) == RetailRuntimeValue::Known(0x2000) {
        entity.capability_flags = (entity.capability_flags & !0x1004) | 8;
    }
    entity.collision.state_flags_at_0x08.overwrite(0x100, 0x100);
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Tertiary,
        PreparedActorTask::new(ActorTaskRuntime::Intro2GunTurret(state)),
    );
    entity.collision.state_flags_at_0x08.overwrite(0x28, 0x28);
    true
}
