//! Shared CD50/CE70 -> CD70 attachment suffix for authenticated native ABDI
//! people/workers. Profile owners prove their metadata and current graph first.
//! ADB0 publishes carrying None; Class14's variant1 instead runs C470/A860
//! and410B70 immediately. Neither path owns removal of the parent's Sub-J row.

use crate::{
    actor_animation::ActorAnimationController,
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskId, ActorTaskSlot, PreparedActorTask},
    entity::EntityManager,
    entity_behavior::BehaviorContextRuntime,
    entity_collision_state::{
        RetailRuntimeValue, DEFERRED_DESTROY_PENDING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
    },
    main_base_abort::MainBaseAbortActorLease,
    world_fx::WorldFx,
};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeActorAttachOutcome {
    RetainedGraph,
    DeferredDestroy,
}

/// Exact callback inputs, excluding the caller-owned Sub-J/16700 prefix.
pub(crate) struct NativeActorAttachmentPlan {
    allocation: MainBaseAbortActorLease,
    parent: MainBaseAbortActorLease,
    parent_capability: u32,
    parent_position: [i16; 3],
    tasks: [Option<(ActorTaskId, ActorTaskRuntime)>; 3],
    before_context: BehaviorContextRuntime,
    context: BehaviorContextRuntime,
    animation: ActorAnimationController,
    outcome: NativeActorAttachOutcome,
}

impl NativeActorAttachmentPlan {
    pub(crate) fn prepare(
        manager: &EntityManager,
        id: u32,
        parent: u32,
        context: BehaviorContextRuntime,
        outcome: NativeActorAttachOutcome,
    ) -> Result<Self, &'static str> {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or("attachment entity")?;
        let parent_entity = manager
            .iter_all()
            .find(|entity| entity.id == parent)
            .ok_or("attachment parent")?;
        let blocked_state = REMOTE_OWNED_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT;
        if id == parent
            || !entity.active
            || !parent_entity.active
            || entity.attached_to.is_some()
            || entity.collision.state_flags_at_0x08.masked(blocked_state)
                != RetailRuntimeValue::Known(0)
            || parent_entity
                .collision
                .state_flags_at_0x08
                .masked(blocked_state)
                != RetailRuntimeValue::Known(0)
            || manager.pending_actor_deferred_destroy_ids().contains(&id)
            || manager
                .pending_actor_deferred_destroy_ids()
                .contains(&parent)
            || ActorTaskSlot::IN_RETAIL_TICK_ORDER.iter().any(|&slot| {
                entity.actor_tasks.task_in_slot(slot).is_some_and(|task| {
                    entity
                        .actor_tasks
                        .wrapper_flags(task)
                        .is_none_or(|flags| !flags.alive || flags.in_callback)
                })
            })
        {
            return Err("attachment completed local allocation");
        }
        let RetailRuntimeValue::Known(Some(before_context)) = entity.current_behavior_context
        else {
            return Err("attachment context");
        };
        let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
            return Err("attachment Sub-I");
        };
        if outcome == NativeActorAttachOutcome::DeferredDestroy
            && crate::main_base_type9_abort::exploding_person_terminal_context(before_context)
                != Some(context)
        {
            return Err("attachment terminal context");
        }
        Ok(Self {
            allocation: manager
                .main_base_abort_actor_observation(id)
                .ok_or("attachment allocation")?
                .lease,
            parent: manager
                .main_base_abort_actor_observation(parent)
                .ok_or("attachment parent allocation")?
                .lease,
            parent_capability: parent_entity.capability_flags,
            parent_position: parent_entity.position_raw(),
            tasks: ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
                entity
                    .actor_tasks
                    .task_in_slot(slot)
                    .map(|task| (task, *entity.actor_tasks.task_state(task).unwrap()))
            }),
            before_context,
            context,
            animation,
            outcome,
        })
    }

    /// The profile owner and caller's completed-task custody are still required.
    /// All assertions precede the optional sound and callback-owned writes.
    pub(crate) fn commit(
        self,
        manager: &mut EntityManager,
        id: u32,
        fx: &mut WorldFx,
    ) -> NativeActorAttachOutcome {
        assert_eq!(id, self.allocation.entity_id);
        assert_eq!(
            manager
                .main_base_abort_actor_observation(id)
                .map(|actor| actor.lease),
            Some(self.allocation)
        );
        assert_eq!(
            manager
                .main_base_abort_actor_observation(self.parent.entity_id)
                .map(|actor| actor.lease),
            Some(self.parent)
        );
        let parent = manager
            .iter_all()
            .find(|entity| entity.id == self.parent.entity_id)
            .unwrap();
        assert!(parent.active);
        assert_eq!(
            parent
                .collision
                .state_flags_at_0x08
                .masked(REMOTE_OWNED_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(parent.capability_flags, self.parent_capability);
        assert_eq!(parent.position_raw(), self.parent_position);
        assert!(!manager.pending_actor_deferred_destroy_ids().contains(&id));
        assert!(!manager
            .pending_actor_deferred_destroy_ids()
            .contains(&self.parent.entity_id));
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.current_behavior_context,
            RetailRuntimeValue::Known(Some(self.before_context))
        );
        assert_eq!(
            entity.actor_animation_runtime,
            RetailRuntimeValue::Known(Some(self.animation))
        );
        assert_eq!(entity.attached_to, Some(self.parent.entity_id));
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(REMOTE_OWNED_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
                entity
                    .actor_tasks
                    .task_in_slot(slot)
                    .map(|task| (task, *entity.actor_tasks.task_state(task).unwrap()))
            }),
            self.tasks
        );

        // 420760 reads the linked PARENT's capabilities and position. The
        // child's descriptor selects the cue; no random pitch word is used.
        if let Some(sound) = self
            .animation
            .relation_attach_sound_id(self.parent_capability)
        {
            fx.queue_fixed_positional_sound_raw(sound, self.parent_position);
        }
        let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
            unreachable!();
        };
        animation.apply_relation_attach(self.parent.entity_id);
        entity.collision.state_flags_at_0x08.overwrite(0x8000, 0);
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(self.context));
        match self.outcome {
            NativeActorAttachOutcome::RetainedGraph => {
                // ADB0's explicit clears are2→1, followed by replacement0.
                for slot in [ActorTaskSlot::Tertiary, ActorTaskSlot::Secondary] {
                    entity
                        .actor_tasks
                        .clear_slot_with_retirement(slot, |task| task.retire_animation(animation));
                }
                entity.actor_tasks.replace_prepared_with_retirement(
                    ActorTaskSlot::Primary,
                    PreparedActorTask::new(ActorTaskRuntime::None),
                    |task| task.retire_animation(animation),
                );
            }
            NativeActorAttachOutcome::DeferredDestroy => {
                // C470 calls A860 (40A87F..40A8A3): physical slots0→1→2.
                for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
                    entity
                        .actor_tasks
                        .clear_slot_with_retirement(slot, |task| task.retire_animation(animation));
                }
                entity.mark_actor_deferred_destroy_pending();
                manager.queue_actor_deferred_destroy(id);
            }
        }
        self.outcome
    }
}
