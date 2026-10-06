//! Fresh Level-1 Type-47 `FUN_00415040` after impact C690.
//!
//! The accepted pair census `20260718-024449` / `20260718-024518` samples
//! all three type-47/model-302 bodies with instance `+0x44` null. This owner
//! therefore uses [`checked_local_damage_transition_after_null_modifier`]
//! for a *surviving* hit. A lethal filter result is `FUN_00414E90` →
//! [`publish_fresh_level_one_type47_standard_death`] (`FUN_00410C10`), not a
//! newly invented death prefix. It does not add Type-47 to the Base/Factory
//! [`crate::entity::CheckedProjectileDamageUnresolved::TargetSurvivorPolicy`]
//! whitelist.
//!
//! After a nonzero 15040 return, `FUN_00410EB0` plays the cached type
//! `+0x80` cue when entity `+0x08` bit `0x4000` is clear. Surviving hits
//! therefore queue that positional sound here. Lethal `FUN_00410C10` sets
//! dying first, so the suffix skips accepted-hit audio. An already-dying
//! 15040 still returns the filtered amount after `FUN_00414E90` consumes
//! only the pre-health buffer; it does not subtract health or enter
//! `FUN_00410C10`. Capability byte `+0x64` bit `8` then selects model
//! `+0xA8/+0xAA/+0xAC/+0xAE` from current `0x2000/0x4000` state and emits
//! class 5 at scale `0x0800` through the shared `FUN_00440DC0` owner.
//! Model 302 header `+0x08` is the recovered extent 182; that Z subtract
//! still runs on a dying target. This owner does not invent a second
//! allocator or a second death prefix.

use crate::damage::EntityHitEntry;
use crate::damage::{
    checked_local_damage_transition_after_null_modifier,
    primary_projectile_hit_presentation_transition, CheckedLocalDamageState,
    CheckedLocalDamageTransition, DamageDeliveryRecord, GenericEntityDamageState,
    GenericEntityDamageTransition, PrimaryProjectileHitPresentationTransition,
};
use crate::entity::{Entity, EntityManager};
use crate::entity_collision_state::{
    active_model_slot_from_state_flags, RetailRuntimeValue, ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
    ACTIVE_MODEL_SLOT_LOW_STATE_BIT, CHECKED_DAMAGE_ENABLED_STATE_BIT, REMOTE_OWNED_STATE_BIT,
};
use crate::ordinary_type47_death_live::{
    publish_fresh_level_one_type47_standard_death, Type47CommonDyingPublication,
    Type47CommonDyingPublicationError, Type47StandardDeathOutcome, TYPE47_COMMON_DYING_ENTITY_TYPE,
    TYPE47_COMMON_DYING_INITIAL_HEALTH_RAW,
};
use crate::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID;
use crate::primary_hit::{
    PrimaryHitCapabilityEmission, PRIMARY_HIT_CAPABILITY_EFFECT_BIT,
    PRIMARY_HIT_CAPABILITY_PARTICLE_CLASS, PRIMARY_HIT_CAPABILITY_PARTICLE_SCALE_RAW,
};
use crate::type47_common_dying_production::LEVEL_ONE_TYPE47_MODEL_EXTENT_RAW;
use crate::world_fx::WorldFx;

/// Why a Type-47 15040 visit cannot apply local health arithmetic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type47CheckedDamageBlock {
    UnauthenticatedSpawn {
        actual: Option<usize>,
    },
    UnexpectedModelSlots {
        actual: [Option<usize>; 4],
    },
    UnexpectedActiveModel {
        actual: Option<usize>,
    },
    DamageModifierUnresolved,
    DamageModifierPresent {
        address: u32,
    },
    CheckedDamageEligibilityUnresolved,
    RemoteOwned,
    DamageProfileUnresolved,
    HealthUnresolved,
    PreHealthBufferUnresolved,
    DyingStateUnresolved,
    GenericHitSoundUnresolved,
    AcceptedHitSoundUnresolved,
    /// Capability `0x8` may take the filtered-zero feedback path.
    FilteredZeroFeedback,
    MissingTypeMetadata,
    /// `FUN_00410C10` is the recovered Type-47 standard-death publisher.
    StandardDeath(Type47CommonDyingPublicationError),
    /// Capability `0x8` is set, but the `0x2000`/`0x4000` slot bits or the
    /// state-word sign needed by `FUN_00440DC0` are unresolved.
    CapabilityFollowUpStateUnresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type47CheckedDamageOutcome {
    NotApplicable,
    Ineligible,
    /// Zero filter result with no player feedback: no health, buffer, sound
    /// or 10EB0 capability suffix is entered.
    FilteredZero {
        entity_id: u32,
    },
    Survived {
        entity_id: u32,
        health_after_raw: i32,
        accepted_damage_raw: i32,
        presentation: Option<PrimaryProjectileHitPresentationTransition>,
        capability_follow_up: Option<PrimaryHitCapabilityEmission>,
    },
    /// Nonzero 15040 after `FUN_00414E90` consumed buffer on a dying target.
    /// Health and the death publisher are left untouched.
    AlreadyDying {
        entity_id: u32,
        health_after_raw: i32,
        accepted_damage_raw: i32,
        pre_health_buffer_after_raw: i32,
        presentation: Option<PrimaryProjectileHitPresentationTransition>,
        capability_follow_up: Option<PrimaryHitCapabilityEmission>,
    },
    Lethal {
        entity_id: u32,
        accepted_damage_raw: i32,
        standard_death: Type47StandardDeathOutcome,
        publication: Option<Type47CommonDyingPublication>,
        presentation: Option<PrimaryProjectileHitPresentationTransition>,
        capability_follow_up: Option<PrimaryHitCapabilityEmission>,
    },
    Blocked {
        entity_id: u32,
        reason: Type47CheckedDamageBlock,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type47CheckedDamageRequest {
    pub delivery: DamageDeliveryRecord,
    pub entry: EntityHitEntry,
    pub retail_tick: u32,
}

impl Type47CheckedDamageRequest {
    fn presentation(
        self,
        already_dying: bool,
        sound: Option<u16>,
    ) -> Option<PrimaryProjectileHitPresentationTransition> {
        (self.entry == EntityHitEntry::PrimaryProjectile).then(|| {
            primary_projectile_hit_presentation_transition(self.retail_tick, already_dying, sound)
        })
    }
}

/// Apply null-modifier `FUN_00415040` to one fresh Level-1 Type-47.
pub fn apply_type47_checked_damage_after_c690(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    request: Type47CheckedDamageRequest,
) -> Type47CheckedDamageOutcome {
    let Some(index) = manager.iter_all().position(|entity| entity.id == entity_id) else {
        return Type47CheckedDamageOutcome::NotApplicable;
    };
    let entity = &manager.iter_all().nth(index).expect("index from position");
    if entity.entity_type != TYPE47_COMMON_DYING_ENTITY_TYPE {
        return Type47CheckedDamageOutcome::NotApplicable;
    }
    if let Err(reason) = authenticate_type47_checked_damage_owner(entity) {
        return Type47CheckedDamageOutcome::Blocked { entity_id, reason };
    }

    match entity
        .collision
        .state_flags_at_0x08
        .masked(CHECKED_DAMAGE_ENABLED_STATE_BIT)
    {
        RetailRuntimeValue::Known(0) => return Type47CheckedDamageOutcome::Ineligible,
        RetailRuntimeValue::Known(_) => {}
        RetailRuntimeValue::Unresolved => {
            return Type47CheckedDamageOutcome::Blocked {
                entity_id,
                reason: Type47CheckedDamageBlock::CheckedDamageEligibilityUnresolved,
            };
        }
    }
    if entity
        .collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT)
        == RetailRuntimeValue::Known(REMOTE_OWNED_STATE_BIT)
    {
        return Type47CheckedDamageOutcome::Blocked {
            entity_id,
            reason: Type47CheckedDamageBlock::RemoteOwned,
        };
    }

    let profile = match entity.collision.damage_profile {
        RetailRuntimeValue::Known(profile) => profile,
        RetailRuntimeValue::Unresolved => {
            return Type47CheckedDamageOutcome::Blocked {
                entity_id,
                reason: Type47CheckedDamageBlock::DamageProfileUnresolved,
            };
        }
    };
    let filtered_damage_raw = request.delivery.packet.filtered_raw(Some(&profile));
    if filtered_damage_raw == 0 {
        return if request
            .delivery
            .filtered_zero_feedback_required(entity.capability_flags)
        {
            Type47CheckedDamageOutcome::Blocked {
                entity_id,
                reason: Type47CheckedDamageBlock::FilteredZeroFeedback,
            }
        } else {
            Type47CheckedDamageOutcome::FilteredZero { entity_id }
        };
    }

    let health_raw = match entity.collision.health_raw {
        RetailRuntimeValue::Known(health) => health,
        RetailRuntimeValue::Unresolved => {
            return Type47CheckedDamageOutcome::Blocked {
                entity_id,
                reason: Type47CheckedDamageBlock::HealthUnresolved,
            };
        }
    };
    let pre_health_buffer_raw = match entity.collision.pre_health_damage_buffer_raw {
        RetailRuntimeValue::Known(buffer) => buffer,
        RetailRuntimeValue::Unresolved => {
            return Type47CheckedDamageOutcome::Blocked {
                entity_id,
                reason: Type47CheckedDamageBlock::PreHealthBufferUnresolved,
            };
        }
    };
    let already_dying = match entity
        .collision
        .state_flags_at_0x08
        .masked(crate::entity_collision_state::DYING_STATE_BIT)
    {
        RetailRuntimeValue::Known(bits) => bits != 0,
        RetailRuntimeValue::Unresolved => {
            return Type47CheckedDamageOutcome::Blocked {
                entity_id,
                reason: Type47CheckedDamageBlock::DyingStateUnresolved,
            };
        }
    };
    let generic_hit_sound_id = match entity.collision.generic_hit_sound_id {
        RetailRuntimeValue::Known(sound) => sound,
        RetailRuntimeValue::Unresolved => {
            return Type47CheckedDamageOutcome::Blocked {
                entity_id,
                reason: Type47CheckedDamageBlock::GenericHitSoundUnresolved,
            };
        }
    };
    let accepted_hit_sound_id = match (
        request.entry,
        entity.collision.accepted_hit_presentation_sound_id,
    ) {
        (EntityHitEntry::Infected | EntityHitEntry::Cured, _) => None,
        (_, RetailRuntimeValue::Known(sound)) => sound,
        (_, RetailRuntimeValue::Unresolved) => {
            return Type47CheckedDamageOutcome::Blocked {
                entity_id,
                reason: Type47CheckedDamageBlock::AcceptedHitSoundUnresolved,
            };
        }
    };

    let transition = checked_local_damage_transition_after_null_modifier(
        CheckedLocalDamageState {
            generic: GenericEntityDamageState {
                health_raw,
                pre_health_buffer_raw,
                already_dying,
            },
            generic_hit_sound_id,
        },
        std::num::NonZeroI32::new(filtered_damage_raw)
            .expect("filtered-zero Type-47 damage returns before this path"),
    );
    match transition {
        CheckedLocalDamageTransition::AlreadyDying { generic } => apply_type47_already_dying_15040(
            manager,
            world_fx,
            entity_id,
            generic,
            accepted_hit_sound_id,
            request,
        ),
        CheckedLocalDamageTransition::DeathDispatchRequired { generic } => {
            apply_type47_lethal_10c10(
                manager,
                world_fx,
                entity_id,
                generic,
                accepted_hit_sound_id,
                request,
            )
        }
        CheckedLocalDamageTransition::Survived(applied) => {
            let presentation = request.presentation(false, accepted_hit_sound_id);
            let entity = manager
                .iter_all()
                .find(|entity| entity.id == entity_id)
                .map(|entity| entity.id);
            debug_assert_eq!(entity, Some(entity_id));
            let Some(entity) = manager.ordinary_type47_entity_mut(entity_id) else {
                return Type47CheckedDamageOutcome::NotApplicable;
            };
            let capability_follow_up =
                match type47_10eb0_capability_follow_up(entity, request.entry) {
                    Ok(follow_up) => follow_up,
                    Err(reason) => {
                        return Type47CheckedDamageOutcome::Blocked { entity_id, reason };
                    }
                };
            entity.collision.health_raw =
                RetailRuntimeValue::Known(applied.generic.health_after_subtraction_raw);
            entity.collision.pre_health_damage_buffer_raw =
                RetailRuntimeValue::Known(applied.generic.pre_health_buffer_after_raw);
            if let Some(presentation) = presentation {
                entity.collision.last_hit_presentation_tick_at_0x34 =
                    RetailRuntimeValue::Known(presentation.last_tick_after);
            }
            let position_raw = entity.position_raw();
            // FUN_00410EB0 after 15040 != 0: play type +0x80 if not dying,
            // then class-5 if +0x64 bit 8 through shared FUN_00440DC0.
            if let Some(sound_id) = presentation.and_then(|value| value.sound_id) {
                world_fx.queue_fixed_positional_sound_raw(sound_id, position_raw);
            }
            if let Some(emission) = capability_follow_up {
                world_fx.emit_primary_hit_capability_follow_up_raw(emission);
            }
            Type47CheckedDamageOutcome::Survived {
                entity_id,
                health_after_raw: applied.generic.health_after_subtraction_raw,
                accepted_damage_raw: applied.accepted_damage_raw,
                presentation,
                capability_follow_up,
            }
        }
    }
}

fn apply_type47_already_dying_15040(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    generic: GenericEntityDamageTransition,
    accepted_hit_sound_id: Option<u16>,
    request: Type47CheckedDamageRequest,
) -> Type47CheckedDamageOutcome {
    let presentation = request.presentation(true, accepted_hit_sound_id);
    let Some(entity) = manager.ordinary_type47_entity_mut(entity_id) else {
        return Type47CheckedDamageOutcome::NotApplicable;
    };
    let capability_follow_up = match type47_10eb0_capability_follow_up(entity, request.entry) {
        Ok(follow_up) => follow_up,
        Err(reason) => {
            return Type47CheckedDamageOutcome::Blocked { entity_id, reason };
        }
    };
    entity.collision.pre_health_damage_buffer_raw =
        RetailRuntimeValue::Known(generic.pre_health_buffer_after_raw);
    if let Some(presentation) = presentation {
        entity.collision.last_hit_presentation_tick_at_0x34 =
            RetailRuntimeValue::Known(presentation.last_tick_after);
    }
    debug_assert_eq!(
        entity.collision.health_raw,
        RetailRuntimeValue::Known(generic.health_after_subtraction_raw)
    );
    if let Some(emission) = capability_follow_up {
        world_fx.emit_primary_hit_capability_follow_up_raw(emission);
    }
    Type47CheckedDamageOutcome::AlreadyDying {
        entity_id,
        health_after_raw: generic.health_after_subtraction_raw,
        accepted_damage_raw: generic.requested_damage_raw,
        pre_health_buffer_after_raw: generic.pre_health_buffer_after_raw,
        presentation,
        capability_follow_up,
    }
}

fn apply_type47_lethal_10c10(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    generic: GenericEntityDamageTransition,
    accepted_hit_sound_id: Option<u16>,
    request: Type47CheckedDamageRequest,
) -> Type47CheckedDamageOutcome {
    let Some(metadata) = manager
        .type_runtime_metadata(TYPE47_COMMON_DYING_ENTITY_TYPE)
        .cloned()
    else {
        return Type47CheckedDamageOutcome::Blocked {
            entity_id,
            reason: Type47CheckedDamageBlock::MissingTypeMetadata,
        };
    };
    match publish_fresh_level_one_type47_standard_death(manager, entity_id, &metadata, world_fx) {
        Err(reason) => Type47CheckedDamageOutcome::Blocked {
            entity_id,
            reason: Type47CheckedDamageBlock::StandardDeath(reason),
        },
        Ok(standard_death) => {
            if let Some(entity) = manager.ordinary_type47_entity_mut(entity_id) {
                entity.collision.pre_health_damage_buffer_raw =
                    RetailRuntimeValue::Known(generic.pre_health_buffer_after_raw);
            }
            let publication = match standard_death {
                Type47StandardDeathOutcome::Published(publication) => Some(publication),
                Type47StandardDeathOutcome::RemoteOwnedNoOp
                | Type47StandardDeathOutcome::AlreadyDyingNoOp => None,
            };
            // Class-5 still runs after 10C10: dying does not skip +0x64 bit 8.
            let capability_follow_up = manager
                .iter_all()
                .find(|entity| entity.id == entity_id)
                .and_then(|entity| type47_10eb0_capability_follow_up(entity, request.entry).ok())
                .flatten();
            if let Some(emission) = capability_follow_up {
                world_fx.emit_primary_hit_capability_follow_up_raw(emission);
            }
            Type47CheckedDamageOutcome::Lethal {
                entity_id,
                accepted_damage_raw: generic.requested_damage_raw,
                standard_death,
                publication,
                presentation: request.presentation(true, accepted_hit_sound_id),
                capability_follow_up,
            }
        }
    }
}

const STATE_SIGN_BIT: u32 = 0x8000_0000;

/// Plan `FUN_00410EB0`'s post-15040 class-5 suffix for a fresh Type-47.
///
/// Retail reads `+0x64` bit 8, then the cached target's current
/// `0x2000`/`0x4000` model words and subtracts the selected model's signed
/// `+0x08` from entity Z. All four Level-1 Type-47 slots are model 302, whose
/// recovered header extent is [`LEVEL_ONE_TYPE47_MODEL_EXTENT_RAW`].
fn type47_10eb0_capability_follow_up(
    entity: &Entity,
    entry: EntityHitEntry,
) -> Result<Option<PrimaryHitCapabilityEmission>, Type47CheckedDamageBlock> {
    if entry != EntityHitEntry::PrimaryProjectile {
        return Ok(None);
    }
    if entity.capability_flags as u8 & PRIMARY_HIT_CAPABILITY_EFFECT_BIT == 0 {
        return Ok(None);
    }
    let slot_mask = ACTIVE_MODEL_SLOT_LOW_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT;
    let slot_bits = match entity.collision.state_flags_at_0x08.masked(slot_mask) {
        RetailRuntimeValue::Known(bits) => bits,
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CheckedDamageBlock::CapabilityFollowUpStateUnresolved);
        }
    };
    let owner_sign = match entity.collision.state_flags_at_0x08.masked(STATE_SIGN_BIT) {
        RetailRuntimeValue::Known(bits) => bits >> 31,
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CheckedDamageBlock::CapabilityFollowUpStateUnresolved);
        }
    };
    let selected_model_slot = active_model_slot_from_state_flags(slot_bits);
    let Some(selected_global_model_id) = entity.model_slots[selected_model_slot] else {
        return Err(Type47CheckedDamageBlock::UnexpectedActiveModel {
            actual: entity.model_index,
        });
    };
    debug_assert_eq!(
        selected_global_model_id,
        FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID
    );
    let mut position_raw = entity.position_raw();
    position_raw[2] = position_raw[2].wrapping_sub(LEVEL_ONE_TYPE47_MODEL_EXTENT_RAW as i16);
    Ok(Some(PrimaryHitCapabilityEmission {
        target_handle: entity.id,
        target_allocation_identity: u64::from(entity.id),
        selected_model_slot,
        selected_global_model_id: selected_global_model_id as u16,
        position_raw,
        particle_class: PRIMARY_HIT_CAPABILITY_PARTICLE_CLASS,
        particle_scale_raw: PRIMARY_HIT_CAPABILITY_PARTICLE_SCALE_RAW,
        owner_sign,
    }))
}

fn authenticate_type47_checked_damage_owner(
    entity: &Entity,
) -> Result<(), Type47CheckedDamageBlock> {
    if crate::type47_initial_behavior_live::live_type47_cohort(entity).is_none() {
        return Err(Type47CheckedDamageBlock::UnauthenticatedSpawn {
            actual: entity.authored_spawn_index,
        });
    }
    let expected_slots = [Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID); 4];
    if entity.model_slots != expected_slots {
        return Err(Type47CheckedDamageBlock::UnexpectedModelSlots {
            actual: entity.model_slots,
        });
    }
    if entity.model_index != Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID) {
        return Err(Type47CheckedDamageBlock::UnexpectedActiveModel {
            actual: entity.model_index,
        });
    }
    match entity.collision.pair_callbacks.damage_modifier_address {
        RetailRuntimeValue::Known(None) => {}
        RetailRuntimeValue::Known(Some(address)) => {
            return Err(Type47CheckedDamageBlock::DamageModifierPresent { address });
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CheckedDamageBlock::DamageModifierUnresolved);
        }
    }
    debug_assert_eq!(entity.entity_type, TYPE47_COMMON_DYING_ENTITY_TYPE);
    debug_assert_eq!(TYPE47_COMMON_DYING_INITIAL_HEALTH_RAW, 3_000);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::damage::{
        DamagePacket, DamageProfile, FUN_0043F780_DAMAGE_DELIVERY, PRIMARY_PROJECTILE_DAMAGE_PACKET,
    };
    use crate::entity::EntityKind;
    use crate::entity_collision_state::{
        EntityPairCallbackRuntimeState, RetailStateWord, DYING_STATE_BIT,
        PAIR_COLLISION_ENABLED_STATE_BIT,
    };
    use crate::ordinary_type47_death_live::{
        TYPE47_COMMON_DYING_CAPABILITY_FLAGS, TYPE47_COMMON_DYING_DEATH_SOUND_ID,
    };
    use crate::primary_hit::{
        PrimaryHitCapabilityEmission, PRIMARY_HIT_CAPABILITY_PARTICLE_CLASS,
        PRIMARY_HIT_CAPABILITY_PARTICLE_SCALE_RAW,
    };
    use crate::type47_common_dying_production::LEVEL_ONE_TYPE47_MODEL_EXTENT_RAW;
    use crate::world_fx::PositionalSoundEvent;

    const ENTITY_ID: u32 = 0x042F_000B;
    const PLAYER_PRIMARY_DELIVERY: DamageDeliveryRecord = DamageDeliveryRecord {
        packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
        source_entity_type_raw: 46,
        owner_handle: 0x1234,
    };

    fn primary_request(retail_tick: u32) -> Type47CheckedDamageRequest {
        Type47CheckedDamageRequest {
            delivery: PLAYER_PRIMARY_DELIVERY,
            entry: EntityHitEntry::PrimaryProjectile,
            retail_tick,
        }
    }

    fn identity_profile() -> DamageProfile {
        DamageProfile {
            thresholds_raw: [0; 7],
            multipliers_q8: [256; 7],
        }
    }

    fn surviving_entity() -> Entity {
        let mut entity = Entity::unresolved_port_entity(ENTITY_ID, EntityKind::Enemy, 47);
        entity.authored_spawn_index = Some(11);
        entity.model_slots = [Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID); 4];
        entity.model_index = Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID);
        entity.collision.health_raw =
            RetailRuntimeValue::Known(TYPE47_COMMON_DYING_INITIAL_HEALTH_RAW);
        entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
        entity.collision.damage_profile = RetailRuntimeValue::Known(identity_profile());
        entity.collision.generic_hit_sound_id = RetailRuntimeValue::Known(None);
        entity.capability_flags = TYPE47_COMMON_DYING_CAPABILITY_FLAGS;
        entity.collision.accepted_hit_presentation_sound_id = RetailRuntimeValue::Known(Some(92));
        entity.collision.state_flags_at_0x08 =
            RetailStateWord::exact(0x2039 | PAIR_COLLISION_ENABLED_STATE_BIT);
        entity.collision.pair_callbacks = EntityPairCallbackRuntimeState {
            component_contact: RetailRuntimeValue::Unresolved,
            damage_modifier_address: RetailRuntimeValue::Known(None),
            damage_modifier_identity_context_empty: RetailRuntimeValue::Known(true),
            type_hit_callback_address: RetailRuntimeValue::Known(None),
            orientation_policy: RetailRuntimeValue::Unresolved,
        };
        entity
    }

    fn expected_capability_follow_up(slot: usize, owner_sign: u32) -> PrimaryHitCapabilityEmission {
        PrimaryHitCapabilityEmission {
            target_handle: ENTITY_ID,
            target_allocation_identity: u64::from(ENTITY_ID),
            selected_model_slot: slot,
            selected_global_model_id: FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID as u16,
            position_raw: [0, 0, -(LEVEL_ONE_TYPE47_MODEL_EXTENT_RAW as i16)],
            particle_class: PRIMARY_HIT_CAPABILITY_PARTICLE_CLASS,
            particle_scale_raw: PRIMARY_HIT_CAPABILITY_PARTICLE_SCALE_RAW,
            owner_sign,
        }
    }

    #[test]
    fn zero_filter_observes_source_and_channel_words_before_any_health_or_feedback_state() {
        for (source, capability, channels, amount, feedback) in [
            (34, 8, [2, 0], 2000, false),
            (0, 8, [6, 0], 2000, false),
            (46, 8, [2, 0], 2000, true),
            (46, 0, [2, 0], 2000, false),
            (46, 8, [1, 0], 2000, false),
            (46, 8, [1, 3], 0, true),
        ] {
            let mut entity = surviving_entity();
            entity.capability_flags = capability;
            entity.collision.damage_profile = RetailRuntimeValue::Known(DamageProfile {
                thresholds_raw: [i32::MAX; 7],
                multipliers_q8: [256; 7],
            });
            entity.collision.health_raw = RetailRuntimeValue::Unresolved;
            entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Unresolved;
            entity.collision.generic_hit_sound_id = RetailRuntimeValue::Unresolved;
            entity.collision.accepted_hit_presentation_sound_id = RetailRuntimeValue::Unresolved;
            let before = entity.collision.clone();
            let mut manager = EntityManager::from_entities_for_test(vec![entity]);
            let mut fx = WorldFx::new();
            let outcome = apply_type47_checked_damage_after_c690(
                &mut manager,
                &mut fx,
                ENTITY_ID,
                Type47CheckedDamageRequest {
                    delivery: DamageDeliveryRecord {
                        packet: DamagePacket {
                            channels,
                            amounts_raw: [amount, 0],
                        },
                        source_entity_type_raw: source,
                        owner_handle: 0x1234,
                    },
                    ..primary_request(250)
                },
            );
            assert_eq!(
                outcome,
                if feedback {
                    Type47CheckedDamageOutcome::Blocked {
                        entity_id: ENTITY_ID,
                        reason: Type47CheckedDamageBlock::FilteredZeroFeedback,
                    }
                } else {
                    Type47CheckedDamageOutcome::FilteredZero {
                        entity_id: ENTITY_ID,
                    }
                }
            );
            assert_eq!(manager.iter_all().next().unwrap().collision, before);
            assert!(fx.take_positional_sounds().is_empty());
            assert_eq!(fx.particle_count(), 0);
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                WorldFx::new().next_shared_retail_random_u16()
            );
        }
    }

    #[test]
    fn infected_damage_omits_the_primary_sound_stamp_and_class5_suffix() {
        for already_dying in [false, true] {
            let mut entity = surviving_entity();
            entity.collision.accepted_hit_presentation_sound_id = RetailRuntimeValue::Unresolved;
            entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
            entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(500);
            if already_dying {
                entity
                    .collision
                    .state_flags_at_0x08
                    .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
            }
            let mut manager = EntityManager::from_entities_for_test(vec![entity]);
            let mut fx = WorldFx::new();
            let outcome = apply_type47_checked_damage_after_c690(
                &mut manager,
                &mut fx,
                ENTITY_ID,
                Type47CheckedDamageRequest {
                    delivery: FUN_0043F780_DAMAGE_DELIVERY,
                    entry: EntityHitEntry::Infected,
                    retail_tick: 250,
                },
            );
            if already_dying {
                assert!(
                    matches!(
                        outcome,
                        Type47CheckedDamageOutcome::AlreadyDying {
                            presentation: None,
                            capability_follow_up: None,
                            ..
                        }
                    ),
                    "{outcome:?}"
                );
            } else {
                assert!(
                    matches!(
                        outcome,
                        Type47CheckedDamageOutcome::Survived {
                            health_after_raw: 1500,
                            presentation: None,
                            capability_follow_up: None,
                            ..
                        }
                    ),
                    "{outcome:?}"
                );
            }
            let after = manager.iter_all().next().unwrap();
            assert_eq!(
                after.collision.last_hit_presentation_tick_at_0x34,
                RetailRuntimeValue::Known(17)
            );
            assert_eq!(
                after.collision.pre_health_damage_buffer_raw,
                RetailRuntimeValue::Known(0)
            );
            assert!(fx.take_positional_sounds().is_empty());
            assert_eq!(fx.particle_count(), 0);
            assert_eq!(fx.pending_event_count(), 0);
        }
    }

    #[test]
    fn surviving_primary_packet_writes_health_without_target_survivor_policy() {
        let mut manager = EntityManager::from_entities_for_test(vec![surviving_entity()]);
        let mut world_fx = WorldFx::new();
        let outcome = apply_type47_checked_damage_after_c690(
            &mut manager,
            &mut world_fx,
            ENTITY_ID,
            primary_request(250),
        );
        let Type47CheckedDamageOutcome::Survived {
            entity_id,
            health_after_raw,
            accepted_damage_raw,
            capability_follow_up,
            ..
        } = outcome
        else {
            panic!("expected surviving 15040: {outcome:?}");
        };
        assert_eq!(entity_id, ENTITY_ID);
        assert_eq!(health_after_raw, 1_000);
        assert_eq!(accepted_damage_raw, 2_000);
        assert_eq!(
            capability_follow_up,
            Some(expected_capability_follow_up(2, 0)),
            "0x2039 keeps 0x2000 and clears 0x4000, so slot 2 uses model 302 extent 182"
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert_eq!(
            entity.collision.health_raw,
            RetailRuntimeValue::Known(1_000)
        );
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(250)
        );
        assert_eq!(
            entity.capability_flags, TYPE47_COMMON_DYING_CAPABILITY_FLAGS,
            "Type-47 capability 0x8 is the recovered class-5 gate"
        );
        assert_eq!(world_fx.pending_event_count(), 1);
        assert_eq!(world_fx.particle_count(), 1);
        world_fx.process_pending();
        assert_eq!(
            world_fx.take_positional_sounds(),
            vec![PositionalSoundEvent::fixed(92, [0.0, 0.0, 0.0])]
        );
        assert_eq!(
            world_fx.particle_count(),
            1,
            "capability 0x8 emits one cold-pacing FUN_00440DC0 class-5 carrier"
        );
    }

    #[test]
    fn surviving_suffix_skips_class5_when_capability_bit_is_clear() {
        let mut entity = surviving_entity();
        entity.capability_flags = 0;
        let mut manager = EntityManager::from_entities_for_test(vec![entity]);
        let mut world_fx = WorldFx::new();
        let outcome = apply_type47_checked_damage_after_c690(
            &mut manager,
            &mut world_fx,
            ENTITY_ID,
            primary_request(250),
        );
        assert!(matches!(
            outcome,
            Type47CheckedDamageOutcome::Survived {
                capability_follow_up: None,
                ..
            }
        ));
        world_fx.process_pending();
        assert_eq!(
            world_fx.take_positional_sounds(),
            vec![PositionalSoundEvent::fixed(92, [0.0, 0.0, 0.0])]
        );
        assert_eq!(world_fx.particle_count(), 0);
    }

    #[test]
    fn already_dying_packet_consumes_buffer_without_a_second_death_prefix() {
        let mut entity = surviving_entity();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
        entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(500);
        let health_before = entity.collision.health_raw;
        let mut manager = EntityManager::from_entities_for_test(vec![entity]);
        let mut world_fx = WorldFx::new();
        let outcome = apply_type47_checked_damage_after_c690(
            &mut manager,
            &mut world_fx,
            ENTITY_ID,
            primary_request(255),
        );
        let Type47CheckedDamageOutcome::AlreadyDying {
            entity_id,
            health_after_raw,
            accepted_damage_raw,
            pre_health_buffer_after_raw,
            presentation,
            capability_follow_up,
        } = outcome
        else {
            panic!("expected already-dying 15040: {outcome:?}");
        };
        assert_eq!(entity_id, ENTITY_ID);
        assert_eq!(health_after_raw, TYPE47_COMMON_DYING_INITIAL_HEALTH_RAW);
        assert_eq!(accepted_damage_raw, 2_000);
        assert_eq!(pre_health_buffer_after_raw, 0);
        assert_eq!(presentation.unwrap().sound_id, None);
        assert_eq!(
            capability_follow_up,
            Some(expected_capability_follow_up(3, 0)),
            "dying sets 0x4000 on a 0x2000 body, so slot 3 uses extent 182"
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert_eq!(entity.collision.health_raw, health_before);
        assert_eq!(
            entity.collision.pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(255)
        );
        assert!(entity
            .actor_task_state(crate::actor_task_owner::ActorTaskSlot::Primary)
            .is_none());
        world_fx.process_pending();
        assert!(
            world_fx.take_positional_sounds().is_empty(),
            "already dying skips +0x80 and does not invent death sound 75"
        );
        assert_eq!(world_fx.particle_count(), 1);
    }

    #[test]
    fn lethal_packet_does_not_invent_standard_death() {
        let mut entity = surviving_entity();
        entity.collision.health_raw = RetailRuntimeValue::Known(100);
        let mut manager = EntityManager::from_entities_for_test(vec![entity]);
        let mut world_fx = WorldFx::new();
        assert!(matches!(
            apply_type47_checked_damage_after_c690(
                &mut manager,
                &mut world_fx,
                ENTITY_ID,
                primary_request(251),
            ),
            Type47CheckedDamageOutcome::Blocked {
                reason: Type47CheckedDamageBlock::MissingTypeMetadata,
                ..
            }
        ));
        assert_eq!(
            manager
                .iter_all()
                .find(|entity| entity.id == ENTITY_ID)
                .unwrap()
                .collision
                .health_raw,
            RetailRuntimeValue::Known(100)
        );
    }

    #[test]
    fn lethal_packet_uses_recovered_10c10_standard_death() {
        use std::convert::Infallible;

        use crate::actor_task_dispatcher::ActorTaskRuntime;
        use crate::actor_task_owner::{ActorTaskSlot, PreparedActorTask};
        use crate::common_mover::SubAPropulsionRuntime;
        use crate::entity_behavior::{
            audited_behavior_style, behavior_program, BehaviorChoiceListSource,
            BehaviorContextRuntime,
        };
        use crate::entity_collision_state::{EntityInitializerSpec, EntityTypeRuntimeMetadata};
        use crate::guard_location_owner::acquisition::GuardLocationAcquisitionTaskState;
        use crate::ordinary_type47_death_live::{
            TYPE47_COMMON_DYING_BEHAVIOR_CHOICES, TYPE47_COMMON_DYING_BEHAVIOR_CLASS_ID,
            TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR, TYPE47_COMMON_DYING_DRIVE_SCALE_PERCENT,
            TYPE47_COMMON_DYING_INITIALIZER_STATE_FLAGS_RAW, TYPE47_COMMON_DYING_MASS_RAW,
            TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR, TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR,
            TYPE47_COMMON_DYING_SUB_H_RECORDS,
        };
        use crate::ordinary_type47_live::{
            FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY,
            FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT,
        };
        use crate::ordinary_type9_wander_owner::plan_ordinary_type9_wander_setup;
        use crate::sub_h_external_frame::SubHRuntimeState;
        use v2k_formats::collision::SubHExternalFrameDescriptor;

        let metadata = EntityTypeRuntimeMetadata {
            model_slots: [FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID as u16; 4],
            mass_raw: TYPE47_COMMON_DYING_MASS_RAW,
            capability_flags: TYPE47_COMMON_DYING_CAPABILITY_FLAGS,
            initial_health_raw: Some(TYPE47_COMMON_DYING_INITIAL_HEALTH_RAW),
            accepted_hit_presentation_sound_id: RetailRuntimeValue::Known(Some(92)),
            death_sound_id: RetailRuntimeValue::Known(Some(TYPE47_COMMON_DYING_DEATH_SOUND_ID)),
            constructor_sound_attachment_id: RetailRuntimeValue::Known(None),
            generic_hit_sound_id: RetailRuntimeValue::Known(None),
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR,
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
            common_mover_topology: RetailRuntimeValue::Known(
                FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY,
            ),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: TYPE47_COMMON_DYING_INITIALIZER_STATE_FLAGS_RAW,
                common_axis_descriptor: TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR,
                behavior_choices: TYPE47_COMMON_DYING_BEHAVIOR_CHOICES
                    .to_vec()
                    .into_boxed_slice(),
                behavior_rule_ref: 0,
                alternate_behavior_class_ref: TYPE47_COMMON_DYING_BEHAVIOR_CLASS_ID,
            }),
            ..EntityTypeRuntimeMetadata::default()
        };

        let mut entity = surviving_entity();
        entity.mass_raw = TYPE47_COMMON_DYING_MASS_RAW;
        entity.capability_flags = TYPE47_COMMON_DYING_CAPABILITY_FLAGS;
        entity.collision.health_raw = RetailRuntimeValue::Known(100);
        entity.collision.death_sound_id =
            RetailRuntimeValue::Known(Some(TYPE47_COMMON_DYING_DEATH_SOUND_ID));
        entity.collision.default_state_flags_at_0xc8 =
            RetailRuntimeValue::Known(TYPE47_COMMON_DYING_INITIALIZER_STATE_FLAGS_RAW);
        entity.actor_common_axis_descriptor =
            RetailRuntimeValue::Known(TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR);
        entity.sub_a_propulsion_runtime =
            RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_retail_words(
                RetailRuntimeValue::Known(777),
                -1,
                TYPE47_COMMON_DYING_DRIVE_SCALE_PERCENT,
            )));
        entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(
            SubHRuntimeState::new(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT).unwrap(),
        ));
        let program = behavior_program(32).expect("class-32");
        let style = *audited_behavior_style(32, 0).expect("Guard v0");
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(
            BehaviorContextRuntime::named_audited(
                program,
                0,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(None),
                RetailRuntimeValue::Known(0),
                style,
            )
            .unwrap(),
        ));
        let position_raw = entity.position_raw();
        {
            let Entity {
                actor_tasks,
                sub_a_propulsion_runtime,
                ..
            } = &mut entity;
            let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
                panic!("fixture retains Sub-A")
            };
            plan_ordinary_type9_wander_setup(
                position_raw,
                TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR.target_speed_base_raw,
            )
            .apply(actor_tasks, sub_a, |specification| {
                Ok::<_, Infallible>(
                    specification
                        .prepare_after_allocation(|| 0x1234)
                        .map_task(ActorTaskRuntime::OrdinaryType9Wander),
                )
            })
            .unwrap();
        }
        entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::GuardLocationAcquisition(
                GuardLocationAcquisitionTaskState::new(0),
            )),
        );

        let mut table = vec![EntityTypeRuntimeMetadata::default(); 48];
        table[47] = metadata;
        let mut manager =
            EntityManager::from_entities_with_type_metadata_for_test(vec![entity], table, true);
        let mut world_fx = WorldFx::new();
        let outcome = apply_type47_checked_damage_after_c690(
            &mut manager,
            &mut world_fx,
            ENTITY_ID,
            primary_request(254),
        );
        let Type47CheckedDamageOutcome::Lethal {
            entity_id,
            publication: Some(_),
            standard_death: Type47StandardDeathOutcome::Published(_),
            capability_follow_up,
            ..
        } = outcome
        else {
            panic!("expected lethal 10C10: {outcome:?}");
        };
        assert_eq!(entity_id, ENTITY_ID);
        assert_eq!(
            capability_follow_up,
            Some(expected_capability_follow_up(3, 0)),
            "10C10 sets 0x4000 on a 0x2000 body, so slot 3 still uses extent 182"
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert!(entity
            .actor_task_state(ActorTaskSlot::Primary)
            .is_some_and(|task| task.family()
                == crate::actor_task_dispatcher::ActorTaskRuntimeFamily::CommonDying));
        world_fx.process_pending();
        let sounds = world_fx.take_positional_sounds();
        assert!(
            sounds
                .iter()
                .any(|sound| { sound.sound_id == usize::from(TYPE47_COMMON_DYING_DEATH_SOUND_ID) }),
            "10C10 queues death 75"
        );
        assert!(
            sounds.iter().all(|sound| sound.sound_id != 92),
            "10C10 sets dying before the 10EB0 suffix, so +0x80 is skipped"
        );
        assert_eq!(
            world_fx.particle_count(),
            1,
            "class-5 still runs after 10C10 because dying does not skip +0x64 bit 8"
        );
    }

    #[test]
    fn unresolved_modifier_does_not_become_null() {
        let mut entity = surviving_entity();
        entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Unresolved;
        let mut manager = EntityManager::from_entities_for_test(vec![entity]);
        let mut world_fx = WorldFx::new();
        assert_eq!(
            apply_type47_checked_damage_after_c690(
                &mut manager,
                &mut world_fx,
                ENTITY_ID,
                primary_request(252),
            ),
            Type47CheckedDamageOutcome::Blocked {
                entity_id: ENTITY_ID,
                reason: Type47CheckedDamageBlock::DamageModifierUnresolved,
            }
        );
    }

    #[test]
    fn other_entity_types_are_not_applicable() {
        let mut entity = surviving_entity();
        entity.entity_type = 17;
        let mut manager = EntityManager::from_entities_for_test(vec![entity]);
        let mut world_fx = WorldFx::new();
        assert_eq!(
            apply_type47_checked_damage_after_c690(
                &mut manager,
                &mut world_fx,
                ENTITY_ID,
                primary_request(253),
            ),
            Type47CheckedDamageOutcome::NotApplicable
        );
    }
}
