//! Type63, the Defender Ball, on the class0 constructor in class50.
//!
//! Operation `0x38` births it at the player and, lacking capability
//! `0x1000`, puts it in the controller's `+220` list. Class50's initializer
//! `FUN_0040B0F0` clears Tertiary then Secondary and `FUN_00407070` builds the
//! Primary (tick `FUN_00407150`, no timeout) and draws the pitch. While the
//! list holds a live ball, the player's damage modifier `FUN_004484A0` hands
//! the ball each packet (`FUN_00415040`) and the craft takes none; health 1
//! makes any filtered hit lethal, and its death is `FUN_00410C10` into class2.

use super::*;
use crate::damage::DamageDeliveryRecord;
use crate::live_actor_checked_damage::{
    apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageFeedback,
    LiveActorDamageRequest, LiveActorDeathResult,
};

#[cfg(test)]
mod tests;

pub(crate) const ENTITY_TYPE: u32 = crate::main_base_type54_abort::TYPE63_ENTITY_TYPE;

impl EntityManager {
    /// `FUN_00407150`: on its owner (`+60`) with zero velocity when the owner
    /// resolves, then roll by `(dt >> 5) + (dt >> 4)` and pitch by `dt >> 7`.
    pub(super) fn defender_ball_visit(&mut self, id: u32, dt: u32) -> Result<(), Class0ActorError> {
        let entity = self
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(Class0ActorError::Allocation)?;
        let RetailRuntimeValue::Known(owner) = entity.collision.recent_relation_id_at_0x60 else {
            return Err(Class0ActorError::Runtime("Defender Ball owner +60"));
        };
        let owner_position = owner.and_then(|owner| {
            self.iter_all()
                .find(|entity| entity.id == owner)
                .map(Entity::position_raw)
        });
        let entity = self.entity_mut(id).ok_or(Class0ActorError::Allocation)?;
        if let Some(position) = owner_position {
            entity.set_position_raw(position);
            entity.set_velocity_raw([0; 3]);
        }
        let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
        entity.set_rotation_heading_pitch_roll_raw([
            heading,
            pitch.wrapping_add((dt >> 7) as i16),
            roll.wrapping_add((dt >> 5) as i16)
                .wrapping_add((dt >> 4) as i16),
        ]);
        Ok(())
    }

    /// The ball's completed class0 owner, which lends its body and its null
    /// contact hook to a pair visit.
    pub(crate) fn defender_ball_owner_completed(&self, id: u32) -> bool {
        Class0ActorOwner::adopt(self, id).is_ok() && !self.native_class0_has_pending_prefix(id)
    }

    /// Whether `FUN_00415040` on this ball can run to its end in the port:
    /// a local, living class50 ball with no modifier or hit callback, whose
    /// lethal hit takes the class2 death. Checked before the player's hit
    /// commits anything.
    pub(crate) fn defender_ball_hit_ready(&self, id: u32) -> bool {
        let Some(entity) = self.iter_all().find(|entity| entity.id == id) else {
            return false;
        };
        let class50 = crate::entity_behavior::behavior_program(50)
            .map(|program| program.initial_style.frame_address);
        entity.entity_type == ENTITY_TYPE
            && Class0ActorOwner::adopt(self, id).is_ok()
            && !self.native_class0_has_pending_prefix(id)
            && entity
                .collision
                .state_flags_at_0x08
                .masked(REMOTE_OWNED_STATE_BIT | DYING_STATE_BIT)
                == RetailRuntimeValue::Known(0)
            && matches!(
                entity.collision.damage_profile,
                RetailRuntimeValue::Known(_)
            )
            && matches!(entity.collision.health_raw, RetailRuntimeValue::Known(_))
            && matches!(
                entity.collision.pre_health_damage_buffer_raw,
                RetailRuntimeValue::Known(_)
            )
            && entity.collision.pair_callbacks.damage_modifier_address
                == RetailRuntimeValue::Known(None)
            && entity.collision.generic_hit_sound_id == RetailRuntimeValue::Known(None)
            && entity.collision.constructor_sound_attachment_id_at_0x8c
                == RetailRuntimeValue::Known(None)
            && matches!(
                entity.current_behavior_context,
                RetailRuntimeValue::Known(Some(context))
                    if Some(context.active_style().style_address()) == class50
                        && context.active_style().death_callback_policy()
                            == DeathCallbackPolicy::None
            )
    }

    /// `FUN_00415040(ball, packet, 0, 0)` from the player's modifier, or a
    /// projectile's or pair's hit on the ball: admission, the ball's filter,
    /// buffer and health, and at zero health `FUN_00410C10` into class2.
    pub(crate) fn apply_defender_ball_checked_damage(
        &mut self,
        id: u32,
        delivery: DamageDeliveryRecord,
        world_fx: &mut WorldFx,
        feedback: Option<LiveActorDamageFeedback<'_>>,
    ) -> Result<bool, &'static str> {
        if !self.defender_ball_hit_ready(id) {
            return Err("Defender Ball hit: body not ready");
        }
        apply_live_actor_checked_damage(
            self,
            world_fx,
            LiveActorDamageRequest {
                entity_id: id,
                delivery,
                entry: LiveActorDamageEntry::Checked,
                ratio_numerator: 0,
                ratio_denominator: 0,
                feedback,
            },
            |manager, world_fx, _| {
                manager.apply_megablast_death(id, world_fx)?;
                Ok::<_, Class0ActorError>(LiveActorDeathResult {
                    returned_nonzero: true,
                    publication: Some(()),
                })
            },
        )
        .map(|outcome| outcome.filtered_damage_raw != 0)
        .map_err(|_| "Defender Ball hit stopped")
    }
}
