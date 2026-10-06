//! Generic Type-9 release/standard-death toward class 14.
//!
//! Retail E370 expiry runs `FUN_00416750` then `FUN_00410C10`. Player-kill and
//! other generic death enter `FUN_00410C10` directly. Both paths share Type-9
//! vtable `+0x08` `FUN_0040DB80`, a null selected-style `+0x2C` hook, then
//! `FUN_0040AC60` / `FUN_00425660` installing Section-12 alternate class 14.
//! `FUN_0040C3A0` is the shared initializer. Session-zero
//! `FUN_00456900(0xC6, 0)` is authorized by `20260817-072421` when capability
//! `0x800` is set and session byte `+0x28F` is zero.
//!
//! This module does not require a Main Base abort lease or the sample-5564
//! matrix bridge. Live body basis and live Primary/Tertiary survive until the
//! C3A0 clears and the installer replaces Primary. Selected-owner E370
//! `RunLifecycleContinuation` calls this publisher with
//! [`OrdinaryType9StandardDeathEntry::SurfaceLifetimeExpiry`].
//! Native Intro2 allocation receipts admit the identical retail Type9 row;
//! their own component origins survive the same class14 custody transfer.

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::EntityManager,
    entity_behavior::{
        DeathCallbackPolicy, ReleaseCallbackPolicy, EXPLODING_PERSON_BEHAVIOR_PROGRAM,
    },
    entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT},
    entity_relation_release::{relation_release_state_word_after, RELATION_ATTACHED_STATE_BIT},
    gameplay_notifications::GameplayNotifications,
    main_base_abort::{classify_generic_death_entry, GenericDeathEntry},
    main_base_type9_abort::{
        exact_level_one_type9_metadata, type9_class14_optional_message,
        HostMainBaseType9PrimaryTaskAllocator, MainBaseType9DeathComponentRuntime,
        MainBaseType9MessageDisposition, MainBaseType9PrimaryTaskAllocator,
        MainBaseType9ResultScreenState, LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
        LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS, LEVEL_ONE_TYPE9_DEATH_SOUND_ID,
        LEVEL_ONE_TYPE9_ENTITY_TYPE, LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS,
        LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW, LEVEL_ONE_TYPE9_PAIR_COLLISION_STATE_BIT,
        LEVEL_ONE_TYPE9_RETAIL_OPTIONAL_MESSAGE_ID, LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR,
    },
    ordinary_type9_initial_production::FreshLevel1Type9InitialProductionOwner,
    shared_retarget_mover::SharedRetargetTaskState,
    world_fx::WorldFx,
};

/// How the caller reached the shared Type-9 standard-death body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9StandardDeathEntry {
    /// `FUN_00410C10` from generic/player death. No `FUN_00416750`.
    GenericDeath,
    /// E370 authored-lifetime expiry: `FUN_00416750` then `FUN_00410C10`.
    SurfaceLifetimeExpiry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9StandardDeathOutcome {
    RemoteOwnedNoOp {
        entity_id: u32,
    },
    AlreadyDyingNoOp {
        entity_id: u32,
    },
    Published {
        entity_id: u32,
        message: MainBaseType9MessageDisposition,
        applied_release_prefix: bool,
        sub_a_rng_word: u16,
    },
    InitializerFallback {
        entity_id: u32,
        message: MainBaseType9MessageDisposition,
        applied_release_prefix: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9StandardDeathBlock {
    NotFreshNewGameFirstWorld,
    NativeAllocationMismatch,
    EntityUnavailable { entity_id: u32 },
    UnsupportedEntityType { actual: u32 },
    TypeMetadataUnavailable,
    TypeMetadataMismatch,
    AlternateBehaviorClassMismatch { actual: u32 },
    RemoteOwnershipUnresolved,
    DyingStateUnresolved,
    DeathSoundMismatch,
    ConstructorSoundAttachmentNotExactNull,
    CurrentBehaviorContextUnavailable,
    CurrentBehaviorDeathCallback(DeathCallbackPolicy),
    CurrentBehaviorReleaseCallback(ReleaseCallbackPolicy),
    RelationAlreadyAttached,
    DefaultStateFlagsUnresolved,
    SubARuntimeUnavailable,
    ActorAnimationRuntimeUnavailable,
    PairCollisionStateUnresolved,
    ResultScreen(MainBaseType9ResultScreenState),
    DeferredDestroyAlreadyQueued,
}

/// True after this publisher (or the Main Base abort twin) has installed class 14.
pub(crate) fn type9_entity_has_class14_publication(entity: &crate::entity::Entity) -> bool {
    matches!(
        entity.current_behavior_context,
        RetailRuntimeValue::Known(Some(context))
            if context.descriptor()
                == crate::entity_behavior::BehaviorDescriptorIdentity::Named(
                    &EXPLODING_PERSON_BEHAVIOR_PROGRAM
                )
    ) && entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
        == RetailRuntimeValue::Known(DYING_STATE_BIT)
}

impl EntityManager {
    /// Publish class14 from an authenticated Level1 or native Intro2 Type9
    /// allocation, without requiring a Main Base abort lease.
    pub fn publish_ordinary_type9_standard_death(
        &mut self,
        entity_id: u32,
        entry: OrdinaryType9StandardDeathEntry,
        result_screen: MainBaseType9ResultScreenState,
        world_fx: &mut WorldFx,
        retail_tick: i32,
        notifications: Option<&mut GameplayNotifications>,
    ) -> Result<OrdinaryType9StandardDeathOutcome, OrdinaryType9StandardDeathBlock> {
        self.publish_ordinary_type9_standard_death_with_allocator(
            entity_id,
            entry,
            result_screen,
            world_fx,
            retail_tick,
            notifications,
            &mut HostMainBaseType9PrimaryTaskAllocator,
        )
    }

    fn publish_ordinary_type9_standard_death_with_allocator(
        &mut self,
        entity_id: u32,
        entry: OrdinaryType9StandardDeathEntry,
        result_screen: MainBaseType9ResultScreenState,
        world_fx: &mut WorldFx,
        retail_tick: i32,
        notifications: Option<&mut GameplayNotifications>,
        allocator: &mut impl MainBaseType9PrimaryTaskAllocator,
    ) -> Result<OrdinaryType9StandardDeathOutcome, OrdinaryType9StandardDeathBlock> {
        let native_intro2 = self
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .is_some_and(crate::intro2_type9::intro2_type9_allocation_authenticates);
        let native_ordinary =
            crate::ordinary_type9_construction::ordinary_type9_native_allocation_authenticates(
                self, entity_id,
            );
        if self
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .is_some_and(|entity| entity.ordinary_type9_native_receipt.is_some())
            && !native_ordinary
        {
            return Err(OrdinaryType9StandardDeathBlock::NativeAllocationMismatch);
        }
        if !self.is_fresh_new_game_first_world() && !native_intro2 && !native_ordinary {
            return Err(OrdinaryType9StandardDeathBlock::NotFreshNewGameFirstWorld);
        }
        let deferred_destroy_already_queued = self
            .pending_actor_deferred_destroy_ids()
            .contains(&entity_id);

        let metadata = self
            .type_runtime_metadata(LEVEL_ONE_TYPE9_ENTITY_TYPE)
            .ok_or(OrdinaryType9StandardDeathBlock::TypeMetadataUnavailable)?;
        if !exact_level_one_type9_metadata(metadata) {
            return Err(OrdinaryType9StandardDeathBlock::TypeMetadataMismatch);
        }
        let alternate_class = metadata
            .initializer
            .as_ref()
            .map(|initializer| initializer.alternate_behavior_class_ref)
            .unwrap_or(0);
        if alternate_class != u32::from(LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS) {
            return Err(
                OrdinaryType9StandardDeathBlock::AlternateBehaviorClassMismatch {
                    actual: alternate_class,
                },
            );
        }
        let constructor_attachment = metadata.constructor_sound_attachment_id;
        let type_death_sound = metadata.death_sound_id;

        let entity = self
            .common_actor_dying_entity_mut(entity_id)
            .ok_or(OrdinaryType9StandardDeathBlock::EntityUnavailable { entity_id })?;
        if entity.entity_type != LEVEL_ONE_TYPE9_ENTITY_TYPE {
            return Err(OrdinaryType9StandardDeathBlock::UnsupportedEntityType {
                actual: entity.entity_type,
            });
        }
        if entity.capability_flags != LEVEL_ONE_TYPE9_CAPABILITY_FLAGS {
            return Err(OrdinaryType9StandardDeathBlock::DeathSoundMismatch);
        }
        // FUN_00410C10 reads the type record when the live entity word was
        // never written. Attract fixtures leave death-sound / +0x8C unresolved.
        let death_sound_matches = match entity.collision.death_sound_id {
            RetailRuntimeValue::Known(Some(sound_id)) => sound_id == LEVEL_ONE_TYPE9_DEATH_SOUND_ID,
            RetailRuntimeValue::Unresolved => {
                type_death_sound == RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE9_DEATH_SOUND_ID))
            }
            RetailRuntimeValue::Known(None) => false,
        };
        if !death_sound_matches {
            return Err(OrdinaryType9StandardDeathBlock::DeathSoundMismatch);
        }
        let live_attachment_ok = match entity.collision.constructor_sound_attachment_id_at_0x8c {
            RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => {
                constructor_attachment == RetailRuntimeValue::Known(None)
            }
            RetailRuntimeValue::Known(Some(_)) => false,
        };
        if !live_attachment_ok {
            return Err(OrdinaryType9StandardDeathBlock::ConstructorSoundAttachmentNotExactNull);
        }

        let remote_owned = match entity
            .collision
            .state_flags_at_0x08
            .masked(REMOTE_OWNED_STATE_BIT)
        {
            RetailRuntimeValue::Known(value) => value != 0,
            RetailRuntimeValue::Unresolved => {
                return Err(OrdinaryType9StandardDeathBlock::RemoteOwnershipUnresolved)
            }
        };
        let already_dying = match entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT) {
            RetailRuntimeValue::Known(value) => value != 0,
            RetailRuntimeValue::Unresolved => {
                return Err(OrdinaryType9StandardDeathBlock::DyingStateUnresolved)
            }
        };
        match classify_generic_death_entry(remote_owned, already_dying) {
            GenericDeathEntry::RemoteOwnedNoOp | GenericDeathEntry::AlreadyDyingNoOp
                if entry == OrdinaryType9StandardDeathEntry::SurfaceLifetimeExpiry =>
            {
                // FUN_004162B0 always runs FUN_00416750 before FUN_00410C10.
                // Already-dying / remote 10C10 is a no-op after that prefix.
                let type_default_state_flags = match entity.collision.default_state_flags_at_0xc8 {
                    RetailRuntimeValue::Known(value) => value,
                    RetailRuntimeValue::Unresolved => {
                        return Err(OrdinaryType9StandardDeathBlock::DefaultStateFlagsUnresolved);
                    }
                };
                entity.collision.state_flags_at_0x08 = relation_release_state_word_after(
                    entity.collision.state_flags_at_0x08,
                    type_default_state_flags,
                );
                entity.collision.default_state_flags_at_0xc8 =
                    RetailRuntimeValue::Known(type_default_state_flags);
                entity.attached_to = None;
                return Ok(if remote_owned {
                    OrdinaryType9StandardDeathOutcome::RemoteOwnedNoOp { entity_id }
                } else {
                    OrdinaryType9StandardDeathOutcome::AlreadyDyingNoOp { entity_id }
                });
            }
            GenericDeathEntry::RemoteOwnedNoOp => {
                return Ok(OrdinaryType9StandardDeathOutcome::RemoteOwnedNoOp { entity_id });
            }
            GenericDeathEntry::AlreadyDyingNoOp => {
                return Ok(OrdinaryType9StandardDeathOutcome::AlreadyDyingNoOp { entity_id });
            }
            GenericDeathEntry::Dispatch => {
                if deferred_destroy_already_queued {
                    return Err(OrdinaryType9StandardDeathBlock::DeferredDestroyAlreadyQueued);
                }
            }
        }

        let RetailRuntimeValue::Known(Some(current_context)) = entity.current_behavior_context
        else {
            return Err(OrdinaryType9StandardDeathBlock::CurrentBehaviorContextUnavailable);
        };
        match current_context.active_style().death_callback_policy() {
            DeathCallbackPolicy::None => {}
            policy => {
                return Err(OrdinaryType9StandardDeathBlock::CurrentBehaviorDeathCallback(policy));
            }
        }
        if entry == OrdinaryType9StandardDeathEntry::SurfaceLifetimeExpiry {
            match current_context.active_style().release_callback_policy() {
                ReleaseCallbackPolicy::None => {}
                policy => {
                    return Err(
                        OrdinaryType9StandardDeathBlock::CurrentBehaviorReleaseCallback(policy),
                    );
                }
            }
            match entity
                .collision
                .state_flags_at_0x08
                .masked(RELATION_ATTACHED_STATE_BIT)
            {
                RetailRuntimeValue::Known(0) => {}
                RetailRuntimeValue::Known(_) => {
                    return Err(OrdinaryType9StandardDeathBlock::RelationAlreadyAttached);
                }
                RetailRuntimeValue::Unresolved => {
                    return Err(OrdinaryType9StandardDeathBlock::RelationAlreadyAttached);
                }
            }
        }

        let Some(selected_context) = current_context.reselect_audited_type_default(
            &EXPLODING_PERSON_BEHAVIOR_PROGRAM,
            EXPLODING_PERSON_BEHAVIOR_PROGRAM.initial_style_table_index_raw,
            EXPLODING_PERSON_BEHAVIOR_PROGRAM.initial_style,
        ) else {
            return Err(OrdinaryType9StandardDeathBlock::CurrentBehaviorContextUnavailable);
        };
        match entity.sub_a_propulsion_runtime {
            RetailRuntimeValue::Known(Some(_)) => {}
            RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => {
                return Err(OrdinaryType9StandardDeathBlock::SubARuntimeUnavailable);
            }
        }
        match entity.actor_animation_runtime {
            RetailRuntimeValue::Known(Some(controller))
                if controller.descriptor()
                    == crate::main_base_type9_abort::LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR => {}
            _ => {
                return Err(OrdinaryType9StandardDeathBlock::ActorAnimationRuntimeUnavailable);
            }
        }
        if entity
            .collision
            .state_flags_at_0x08
            .masked(LEVEL_ONE_TYPE9_PAIR_COLLISION_STATE_BIT)
            == RetailRuntimeValue::Unresolved
        {
            return Err(OrdinaryType9StandardDeathBlock::PairCollisionStateUnresolved);
        }
        let message = type9_class14_optional_message(entity.capability_flags, result_screen)
            .map_err(|_block| OrdinaryType9StandardDeathBlock::ResultScreen(result_screen))?;

        let type_default_state_flags = match (entry, entity.collision.default_state_flags_at_0xc8) {
            (
                OrdinaryType9StandardDeathEntry::SurfaceLifetimeExpiry,
                RetailRuntimeValue::Known(value),
            ) => value,
            (
                OrdinaryType9StandardDeathEntry::SurfaceLifetimeExpiry,
                RetailRuntimeValue::Unresolved,
            ) => {
                return Err(OrdinaryType9StandardDeathBlock::DefaultStateFlagsUnresolved);
            }
            (OrdinaryType9StandardDeathEntry::GenericDeath, _) => {
                LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW
            }
        };
        let position_raw = entity.position_raw();
        let apply_release_prefix = entry == OrdinaryType9StandardDeathEntry::SurfaceLifetimeExpiry;

        let mut production_indices = self
            .fresh_level1_type9_initial_productions
            .iter()
            .enumerate()
            .filter(|(_, owner)| owner.entity_id() == entity_id)
            .map(|(index, _)| index);
        let production_index = production_indices.next();
        if production_indices.next().is_some() {
            return Err(OrdinaryType9StandardDeathBlock::CurrentBehaviorContextUnavailable);
        }
        let components = match production_index {
            Some(sidecar_index) => {
                let owner = self
                    .fresh_level1_type9_initial_productions
                    .remove(sidecar_index);
                debug_assert_eq!(
                    FreshLevel1Type9InitialProductionOwner::entity_id(&owner),
                    entity_id
                );
                self.common_actor_dying_entity_mut(entity_id)
                    .and_then(|entity| entity.ordinary_type9_selected_component_runtime.take())
                    .map(|runtime| runtime.components())
            }
            None => self
                .common_actor_dying_entity_mut(entity_id)
                .and_then(|entity| {
                    entity
                        .ordinary_type9_selected_component_runtime
                        .take()
                        .map(|runtime| runtime.components())
                        .or_else(|| entity.ordinary_type9_pending_initial_selection.take())
                }),
        };

        let entity = self
            .common_actor_dying_entity_mut(entity_id)
            .expect("preflight authenticated the allocation");
        if apply_release_prefix {
            entity.collision.state_flags_at_0x08 = relation_release_state_word_after(
                entity.collision.state_flags_at_0x08,
                type_default_state_flags,
            );
            entity.collision.default_state_flags_at_0xc8 =
                RetailRuntimeValue::Known(type_default_state_flags);
            entity.attached_to = None;
        }

        entity.collision.health_raw = RetailRuntimeValue::Known(0);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
        world_fx.queue_fixed_positional_sound_raw(LEVEL_ONE_TYPE9_DEATH_SOUND_ID, position_raw);
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(selected_context));

        let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
            unreachable!("preflight authenticated the exact Sub-I runtime");
        };
        entity
            .actor_tasks
            .clear_slot_with_retirement(ActorTaskSlot::Secondary, |task| {
                task.retire_animation(animation)
            });
        entity
            .actor_tasks
            .clear_slot_with_retirement(ActorTaskSlot::Tertiary, |task| {
                task.retire_animation(animation)
            });
        animation.apply_exploding_person_reset();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(LEVEL_ONE_TYPE9_PAIR_COLLISION_STATE_BIT, 0);
        if let Some(components) = components {
            entity.main_base_type9_death_component_runtime =
                Some(MainBaseType9DeathComponentRuntime { components });
        }

        let task_state =
            SharedRetargetTaskState::new(position_raw, LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS);
        let Some(prepared) = allocator.prepare(task_state) else {
            entity.publish_behavior_initializer_failure_fallback(selected_context);
            return Ok(OrdinaryType9StandardDeathOutcome::InitializerFallback {
                entity_id,
                message,
                applied_release_prefix: apply_release_prefix,
            });
        };

        let sub_a_rng_word = world_fx.next_shared_retail_random_u16();
        let RetailRuntimeValue::Known(Some(sub_a)) = &mut entity.sub_a_propulsion_runtime else {
            unreachable!("preflight authenticated the exact Sub-A runtime");
        };
        sub_a.apply_shared_initializer_rng_reset(
            LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw,
            sub_a_rng_word,
        );
        sub_a.apply_shared_initializer_target_speed_write(1);
        let _task_id = entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Primary,
            prepared.map(ActorTaskRuntime::SharedRetarget),
        );

        if let (Some(notifications), MainBaseType9MessageDisposition::QueuedSessionZeroDirectC6) =
            (notifications, message)
        {
            notifications.queue_type9_session_zero_class14(retail_tick);
            let _ = LEVEL_ONE_TYPE9_RETAIL_OPTIONAL_MESSAGE_ID;
        }

        Ok(OrdinaryType9StandardDeathOutcome::Published {
            entity_id,
            message,
            applied_release_prefix: apply_release_prefix,
            sub_a_rng_word,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_zero_policy_is_the_c3a0_gate() {
        assert_eq!(
            type9_class14_optional_message(
                LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
                MainBaseType9ResultScreenState::NotShown,
            ),
            Ok(MainBaseType9MessageDisposition::QueuedSessionZeroDirectC6)
        );
        assert_eq!(LEVEL_ONE_TYPE9_RETAIL_OPTIONAL_MESSAGE_ID, 0x00c6);
        assert_eq!(
            EXPLODING_PERSON_BEHAVIOR_PROGRAM.initial_style.class_id,
            LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS
        );
        assert_eq!(
            EXPLODING_PERSON_BEHAVIOR_PROGRAM
                .initial_style
                .death_callback_policy(),
            DeathCallbackPolicy::None
        );
        assert_eq!(
            EXPLODING_PERSON_BEHAVIOR_PROGRAM
                .initial_style
                .release_callback_policy(),
            ReleaseCallbackPolicy::None
        );
    }
}
