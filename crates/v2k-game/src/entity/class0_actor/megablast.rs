//! Type65, the Megablast seedpod, on the class0 constructor in class55.
//!
//! Births: operation `0x39` (`FUN_00445A90`, mode 0) and the deploy
//! `FUN_00444400` (three in mode 1), each a `FUN_00438080` record with the
//! player's position, velocity and attitude and the mode in `+88`, then the
//! player's handle in `+60`. The pod's death is `FUN_00410C10` through the
//! style's slot `+04` (`FUN_0040CEF0`): its alternate class2 `FUN_0040C470`
//! clears the tasks and stages the removal.

use super::*;
use crate::megablast::{MegablastTaskState, ENTITY_TYPE};

#[cfg(test)]
mod tests;

/// One `FUN_00438080` record for a Type65.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MegablastBirth {
    pub owner_id: u32,
    pub position_raw: [i16; 3],
    pub rotation_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    /// Record `+20`, entity `+88`: 0 for the pickup, 1 for the deploy.
    pub mode: i32,
}

impl EntityManager {
    /// Build one seedpod and return its class0 owner, which the caller
    /// registers with its scheduler.
    pub(crate) fn append_megablast(
        &mut self,
        birth: MegablastBirth,
        resources: EntityConstructionResources<'_>,
        retail_tick: u32,
        waves_enabled: bool,
        world_fx: &mut WorldFx,
    ) -> Result<Class0ActorOwner, Class0ActorError> {
        let metadata = self
            .type_metadata
            .get(ENTITY_TYPE as usize)
            .cloned()
            .ok_or(Class0ActorError::Metadata)?;
        authenticate_metadata(ENTITY_TYPE, &metadata)?;
        let terrain = resources
            .terrain
            .ok_or(Class0ActorError::Runtime("constructor terrain"))?;
        let constructor_surface_bits = crate::entity_initializer::constructor_surface_bits_at_tick(
            birth.position_raw,
            terrain,
            retail_tick,
            waves_enabled,
        )
        .ok_or(Class0ActorError::Runtime("constructor surface"))?;
        let id = self.next_entity_id;
        std::num::NonZeroU32::new(id).ok_or(Class0ActorError::Allocation)?;
        let stamp = self.begin_common_body_attempt();
        let mut entity = native_instance_body(
            id,
            stamp,
            &metadata,
            terrain,
            NativeInstanceBodyRequest::zeroed(ENTITY_TYPE),
        );
        entity.set_position_raw(birth.position_raw);
        entity.set_rotation_heading_pitch_roll_raw(birth.rotation_raw);
        entity.set_velocity_raw(birth.velocity_raw);
        let allocation = observe_main_base_abort_actor(&entity, self.allocation_generation).lease;
        let runtime = construct_class0_actor(
            &mut entity,
            Class0ActorConstruction {
                allocation,
                metadata: &metadata,
                spawn: Class0SpawnInput::AtPose {
                    entity_type: ENTITY_TYPE,
                    position_raw: birth.position_raw,
                    rotation_raw: birth.rotation_raw,
                    velocity_raw: birth.velocity_raw,
                },
                resources,
                constructor_surface_bits,
            },
            &mut || u32::from(world_fx.next_shared_retail_random_u16()),
        )?;
        // 40B140 -> 07800 hands the record's +88 word to the Primary.
        entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::Megablast(MegablastTaskState::new(
                birth.mode,
            ))),
        );
        // The caller writes +60 once 438080 has returned.
        entity.collision.recent_relation_id_at_0x60 =
            RetailRuntimeValue::Known(Some(birth.owner_id));
        assert_eq!(self.allocate_live_entity_id(), id);
        self.append_live_entity(entity);
        self.native_class0_actors.insert(id, runtime);
        Class0ActorOwner::adopt(self, id)
    }

    /// `FUN_00410C10` on a living seedpod (or Defender Ball): health 0 and dying, the type's
    /// sound 62 (sent in network play), no `+8C` attachment, the style's
    /// null death callback, then class2's `FUN_0040C470`, which clears the
    /// three task slots and stages the removal.
    pub(crate) fn apply_megablast_death(
        &mut self,
        id: u32,
        world_fx: &mut WorldFx,
    ) -> Result<(), Class0ActorError> {
        let entity = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == id)
            .ok_or(Class0ActorError::Allocation)?;
        // Type63's Defender Ball shares the alternate class2 and sound 62.
        if !matches!(
            entity.entity_type,
            ENTITY_TYPE | super::defender_ball::ENTITY_TYPE
        ) {
            return Err(Class0ActorError::Metadata);
        }
        match entity
            .collision
            .state_flags_at_0x08
            .masked(REMOTE_OWNED_STATE_BIT | DYING_STATE_BIT)
        {
            RetailRuntimeValue::Known(0) => {}
            RetailRuntimeValue::Known(_) => return Ok(()),
            RetailRuntimeValue::Unresolved => {
                return Err(Class0ActorError::Runtime("generic death entry"))
            }
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Class0ActorError::Graph);
        };
        if context.active_style().death_callback_policy() != DeathCallbackPolicy::None
            || entity.collision.constructor_sound_attachment_id_at_0x8c
                != RetailRuntimeValue::Known(None)
        {
            return Err(Class0ActorError::Graph);
        }
        let selected = context
            .reselect_audited_type_default(
                &QUIET_DEATH_BEHAVIOR_PROGRAM,
                QUIET_DEATH_BEHAVIOR_PROGRAM.initial_style_table_index_raw,
                QUIET_DEATH_STYLE,
            )
            .ok_or(Class0ActorError::Graph)?;
        entity.collision.health_raw = RetailRuntimeValue::Known(0);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
        world_fx.queue_networked_positional_sound_raw(62, entity.position_raw());
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(selected));
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            entity.actor_tasks.clear_slot(slot);
        }
        entity.mark_actor_deferred_destroy_pending();
        self.queue_actor_deferred_destroy(id);
        Ok(())
    }

    /// The tail of [`Self::apply_megablast_death`] for a body whose health,
    /// dying bit and death cue a radial plan already wrote: class2's style,
    /// the cleared tasks and the staged removal.
    pub(crate) fn finish_class2_death(&mut self, id: u32) {
        let Some(entity) = self.entities.iter_mut().find(|entity| entity.id == id) else {
            return;
        };
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return;
        };
        if let Some(selected) = context.reselect_audited_type_default(
            &QUIET_DEATH_BEHAVIOR_PROGRAM,
            QUIET_DEATH_BEHAVIOR_PROGRAM.initial_style_table_index_raw,
            QUIET_DEATH_STYLE,
        ) {
            entity.current_behavior_context = RetailRuntimeValue::Known(Some(selected));
        }
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            entity.actor_tasks.clear_slot(slot);
        }
        entity.mark_actor_deferred_destroy_pending();
        self.queue_actor_deferred_destroy(id);
    }

    /// `FUN_00444330` (the player) or `FUN_0040EAB0` on the blast's owner:
    /// `shielded` clears checked damage `0x8000` on the owner and on every
    /// live, living, nonzero-state body in the player's hold (`+220`); the
    /// restore sets it on the player, and on a hold body only when its
    /// effective policy (`+C8`, style `+34`/`+38`) lacks bit `0x80`.
    pub(crate) fn shield_megablast_owner(
        &mut self,
        owner_id: u32,
        shielded: bool,
    ) -> Result<(), Class0ActorError> {
        let player = self.player().map(|player| player.id);
        if player != Some(owner_id) {
            // 40EAB0 on a missing owner reads through a null record in retail.
            return Err(Class0ActorError::Runtime(
                "Megablast owner is not the player",
            ));
        }
        let members: Vec<u32> = self.auxiliary_owned_ids.clone();
        let player = self
            .entity_mut(owner_id)
            .ok_or(Class0ActorError::Allocation)?;
        player
            .collision
            .state_flags_at_0x08
            .overwrite(0x8000, if shielded { 0 } else { 0x8000 });
        for member in members {
            let Some(entity) = self.entity_mut(member) else {
                continue;
            };
            let RetailRuntimeValue::Known(state) =
                entity.collision.state_flags_at_0x08.masked(u32::MAX)
            else {
                return Err(Class0ActorError::Runtime("Megablast hold body state"));
            };
            if state & DYING_STATE_BIT != 0 || state == 0 {
                continue;
            }
            if shielded {
                entity.collision.state_flags_at_0x08.overwrite(0x8000, 0);
                continue;
            }
            let (RetailRuntimeValue::Known(defaults), RetailRuntimeValue::Known(Some(context))) = (
                entity.collision.default_state_flags_at_0xc8,
                entity.current_behavior_context,
            ) else {
                return Err(Class0ActorError::Runtime("Megablast hold body policy"));
            };
            let program = context
                .active_style()
                .audited()
                .and_then(|style| {
                    crate::entity_behavior::audited_behavior_program(u32::from(style.class_id))
                })
                .filter(|program| {
                    context.active_style().style_address() == program.initial_style.frame_address
                })
                .ok_or(Class0ActorError::Runtime("Megablast hold body style"))?;
            let effective =
                (defaults | program.authored_enable_policy) & !program.authored_disable_policy;
            if effective & 0x80 == 0 {
                entity
                    .collision
                    .state_flags_at_0x08
                    .overwrite(0x8000, 0x8000);
            }
        }
        Ok(())
    }
}
