//! Direct `FUN_00411760 -> FUN_00415040` damage for fresh Level-1 spiders.
//!
//! Collision supplies channel 1, source type 17, and the contacted actor's own
//! handle. It does not stamp a projectile hit tick, reselect an impact style,
//! or apply `FUN_00411030`. Capability 8 excludes the network and managed-death
//! tails; self provenance excludes player feedback. Live modifier and generic
//! hit callbacks must remain proven null at the phase which consumes them.
//! The common standard-death publisher owns the authenticated class-12 suffix.
//!
//! Buffer, sound, and health effects commit in retail order. A late rejection
//! retains that prefix and must not be replayed as another collision delivery.
//! The standard-death call has a whole-helper admission boundary: unavailable
//! class-12 evidence stops before its health-zero/flag/sound prefix. This
//! deliberately earlier evidence stop retains the caller's lethal subtraction;
//! it does not model an original callback returning with unresolved components.

use crate::{
    actor_standard_death_live::{
        publish_fresh_level_one_type17_standard_death, Type17CommonDyingPublicationError,
        Type17StandardDeathOutcome, TYPE17_STANDARD_DEATH_CAPABILITY_FLAGS,
    },
    common_dying_live::{
        LevelOneType17CommonDyingOwner, FRESH_LEVEL1_TYPE17_COMMON_DYING_SPAWN_INDICES,
        TYPE17_COMMON_DYING_ENTITY_TYPE, TYPE17_COMMON_DYING_MODEL_ID,
    },
    damage::{
        absorb_pre_health_damage_buffer, generic_entity_damage_transition, DamageDeliveryRecord,
        GenericEntityDamageStage, GenericEntityDamageState,
    },
    entity::{Entity, EntityManager},
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, CHECKED_DAMAGE_ENABLED_STATE_BIT,
        DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
    },
    world_fx::WorldFx,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17CollisionDamageOutcome {
    /// The signed filtered value returned by `FUN_00415040`, before buffering.
    pub filtered_damage_raw: i32,
    pub damage_after_buffer_raw: i32,
    pub common_dying: Option<LevelOneType17CommonDyingOwner>,
}

impl Type17CollisionDamageOutcome {
    const ZERO: Self = Self {
        filtered_damage_raw: 0,
        damage_after_buffer_raw: 0,
        common_dying: None,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type17CollisionDamagePhase {
    Admission,
    Filter,
    Modifier,
    RemoteOwner,
    Buffer,
    Dying,
    HitSound,
    HitCallback,
    Health,
    Death,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type17CollisionDamageBlock {
    UnexpectedDelivery,
    UnauthenticatedEntity,
    MetadataMismatch(&'static str),
    RuntimeUnresolved(&'static str),
    RuntimeMismatch(&'static str),
    UnsupportedCallback { address: u32 },
    RemoteOwned,
    Death(Type17CommonDyingPublicationError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type17CollisionDamageError {
    pub phase: Type17CollisionDamagePhase,
    pub reason: Type17CollisionDamageBlock,
    /// At least one buffer, positional sound, or health effect already ran.
    pub committed_prefix: bool,
}

/// Consume one self-attributed collision packet with retail's zero/zero ratio.
/// Static-cell damage must already have run, including its shared RNG draws.
pub fn apply_type17_collision_damage(
    manager: &mut EntityManager,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
    delivery: DamageDeliveryRecord,
) -> Result<Type17CollisionDamageOutcome, Type17CollisionDamageError> {
    use Type17CollisionDamageBlock as Block;
    use Type17CollisionDamagePhase as Phase;

    let mut committed_prefix = false;
    let error = |phase, reason, committed_prefix| Type17CollisionDamageError {
        phase,
        reason,
        committed_prefix,
    };
    if delivery.source_entity_type_raw != TYPE17_COMMON_DYING_ENTITY_TYPE
        || delivery.packet.channels != [1, 0]
        || delivery.packet.amounts_raw[1] != 0
    {
        return Err(error(Phase::Admission, Block::UnexpectedDelivery, false));
    }
    let entity_id = delivery.owner_handle;
    let fresh_first_world = manager.is_fresh_new_game_first_world();
    let Some(entity) = manager.entity_mut(entity_id) else {
        return Ok(Type17CollisionDamageOutcome::ZERO);
    };
    if !fresh_first_world
        || !entity.active
        || entity.entity_type != TYPE17_COMMON_DYING_ENTITY_TYPE
        || !entity
            .authored_spawn_index
            .is_some_and(|spawn| FRESH_LEVEL1_TYPE17_COMMON_DYING_SPAWN_INDICES.contains(&spawn))
        || entity.model_slots != [Some(TYPE17_COMMON_DYING_MODEL_ID); 4]
        || entity.model_index != Some(TYPE17_COMMON_DYING_MODEL_ID)
    {
        return Err(error(Phase::Admission, Block::UnauthenticatedEntity, false));
    }
    if metadata.model_slots != [TYPE17_COMMON_DYING_MODEL_ID as u16; 4]
        || metadata.capability_flags != TYPE17_STANDARD_DEATH_CAPABILITY_FLAGS
        || entity.capability_flags != TYPE17_STANDARD_DEATH_CAPABILITY_FLAGS
    {
        return Err(error(
            Phase::Admission,
            Block::MetadataMismatch("model/capabilities"),
            false,
        ));
    }
    let enabled = state_bit(entity, CHECKED_DAMAGE_ENABLED_STATE_BIT).ok_or_else(|| {
        error(
            Phase::Admission,
            Block::RuntimeUnresolved("admission bit"),
            false,
        )
    })?;
    if !enabled {
        return Ok(Type17CollisionDamageOutcome::ZERO);
    }
    let profile = metadata.damage_profile.ok_or_else(|| {
        error(
            Phase::Filter,
            Block::MetadataMismatch("damage profile"),
            false,
        )
    })?;
    match entity.collision.damage_profile {
        RetailRuntimeValue::Known(runtime) if runtime == profile => {}
        RetailRuntimeValue::Known(_) => {
            return Err(error(
                Phase::Filter,
                Block::RuntimeMismatch("damage profile"),
                false,
            ));
        }
        RetailRuntimeValue::Unresolved => {
            return Err(error(
                Phase::Filter,
                Block::RuntimeUnresolved("damage profile"),
                false,
            ));
        }
    }
    let filtered_damage_raw = profile.filter(delivery.packet);
    if filtered_damage_raw == 0 {
        // Source type 17 cannot enter the source-46 filter-zero feedback path.
        return Ok(Type17CollisionDamageOutcome::ZERO);
    }
    require_null_callback(entity.collision.pair_callbacks.damage_modifier_address)
        .map_err(|reason| error(Phase::Modifier, reason, false))?;
    match state_bit(entity, REMOTE_OWNED_STATE_BIT) {
        Some(false) => {}
        Some(true) => return Err(error(Phase::RemoteOwner, Block::RemoteOwned, false)),
        None => {
            return Err(error(
                Phase::RemoteOwner,
                Block::RuntimeUnresolved("remote bit"),
                false,
            ))
        }
    }

    let buffer_before = match entity.collision.pre_health_damage_buffer_raw {
        RetailRuntimeValue::Known(buffer) => buffer,
        RetailRuntimeValue::Unresolved => {
            return Err(error(
                Phase::Buffer,
                Block::RuntimeUnresolved("pre-health buffer"),
                false,
            ));
        }
    };
    let (buffer_after, damage_after_buffer_raw) =
        absorb_pre_health_damage_buffer(buffer_before, filtered_damage_raw);
    if buffer_before > 0 {
        entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(buffer_after);
        committed_prefix = true;
    }
    let mut outcome = Type17CollisionDamageOutcome {
        filtered_damage_raw,
        damage_after_buffer_raw,
        common_dying: None,
    };
    match state_bit(entity, DYING_STATE_BIT) {
        Some(true) => return Ok(outcome),
        Some(false) => {}
        None => {
            return Err(error(
                Phase::Dying,
                Block::RuntimeUnresolved("dying bit"),
                committed_prefix,
            ))
        }
    }
    let sound = match (
        metadata.generic_hit_sound_id,
        entity.collision.generic_hit_sound_id,
    ) {
        (RetailRuntimeValue::Known(expected), RetailRuntimeValue::Known(actual))
            if expected == actual =>
        {
            actual
        }
        (RetailRuntimeValue::Known(_), RetailRuntimeValue::Known(_)) => {
            return Err(error(
                Phase::HitSound,
                Block::RuntimeMismatch("generic-hit sound"),
                committed_prefix,
            ));
        }
        _ => {
            return Err(error(
                Phase::HitSound,
                Block::RuntimeUnresolved("generic-hit sound"),
                committed_prefix,
            ))
        }
    };
    if let Some(sound_id) = sound {
        world_fx.queue_fixed_positional_sound_raw(sound_id, entity.position_raw());
        committed_prefix = true;
    }
    require_null_callback(entity.collision.pair_callbacks.type_hit_callback_address)
        .map_err(|reason| error(Phase::HitCallback, reason, committed_prefix))?;
    let health_before = match entity.collision.health_raw {
        RetailRuntimeValue::Known(health) => health,
        RetailRuntimeValue::Unresolved => {
            return Err(error(
                Phase::Health,
                Block::RuntimeUnresolved("health"),
                committed_prefix,
            ));
        }
    };
    // The buffer write and dying gate above have already run. The shared
    // arithmetic now owns only the unbuffered health subtraction and test.
    let transition = generic_entity_damage_transition(
        GenericEntityDamageState {
            health_raw: health_before,
            pre_health_buffer_raw: 0,
            already_dying: false,
        },
        damage_after_buffer_raw,
    );
    entity.collision.health_raw =
        RetailRuntimeValue::Known(transition.health_after_subtraction_raw);
    committed_prefix = true;
    if transition.stage != GenericEntityDamageStage::DeathDispatchRequired {
        return Ok(outcome);
    }
    if entity.collision.constructor_sound_attachment_id_at_0x8c != RetailRuntimeValue::Known(None) {
        return Err(error(
            Phase::Death,
            Block::RuntimeMismatch("death attachment"),
            committed_prefix,
        ));
    }
    match publish_fresh_level_one_type17_standard_death(manager, entity_id, metadata, world_fx)
        .map_err(|reason| error(Phase::Death, Block::Death(reason), committed_prefix))?
    {
        Type17StandardDeathOutcome::Published(publication) => {
            outcome.common_dying = Some(publication.owner);
        }
        Type17StandardDeathOutcome::RemoteOwnedNoOp
        | Type17StandardDeathOutcome::AlreadyDyingNoOp => {}
    }
    // Capabilities == 8 and source == 17 prove the network, managed-death,
    // and player-kill suffix predicates false; no unrelated state is sampled.
    Ok(outcome)
}

fn state_bit(entity: &Entity, bit: u32) -> Option<bool> {
    match entity.collision.state_flags_at_0x08.masked(bit) {
        RetailRuntimeValue::Known(value) => Some(value != 0),
        RetailRuntimeValue::Unresolved => None,
    }
}

fn require_null_callback(
    callback: RetailRuntimeValue<Option<u32>>,
) -> Result<(), Type17CollisionDamageBlock> {
    match callback {
        RetailRuntimeValue::Known(None) => Ok(()),
        RetailRuntimeValue::Known(Some(address)) => {
            Err(Type17CollisionDamageBlock::UnsupportedCallback { address })
        }
        RetailRuntimeValue::Unresolved => {
            Err(Type17CollisionDamageBlock::RuntimeUnresolved("callback"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        actor_task_dispatcher::ActorTaskRuntime,
        actor_task_owner::ActorTaskSlot,
        common_mover::SubAPropulsionRuntime,
        damage::{DamagePacket, DamageProfile},
        entity::{EntityKind, LEVEL_ONE_TYPE17_SELF_MASS_RAW},
        entity_behavior::{
            audited_behavior_style, behavior_program, BehaviorChoiceListSource,
            BehaviorContextRuntime,
        },
        entity_collision_state::{
            EntityCollisionRuntimeState, EntityInitializerSpec, RetailStateWord, SURFACE_STATE_MASK,
        },
        sub_h_external_frame::SubHRuntimeState,
        type17_impact_live::{
            TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR, TYPE17_MODEL256_COMPONENT_TOPOLOGY,
            TYPE17_MODEL256_SUB_A_TARGET_SPEED_BASE_RAW, TYPE17_MODEL256_SUB_H_RECORD_COUNT,
        },
        type17_impact_reselection::{TYPE17_BEHAVIOR_RULE_REF, TYPE17_IMPACT_BEHAVIOR_CHOICES},
    };
    use v2k_formats::collision::{
        SubAPropulsionDescriptor, SubHExternalFrameDescriptor, SubHExternalFrameRecord,
    };

    const ENTITY_ID: u32 = 19;

    fn metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [256; 4],
            mass_raw: LEVEL_ONE_TYPE17_SELF_MASS_RAW,
            capability_flags: 8,
            initial_health_raw: Some(5_000),
            damage_profile: Some(DamageProfile {
                thresholds_raw: [0, 2000, 200, 0, 200, 0, 0],
                multipliers_q8: [0, 256, 256, 512, 128, 0, 0],
            }),
            death_sound_id: RetailRuntimeValue::Known(Some(94)),
            constructor_sound_attachment_id: RetailRuntimeValue::Known(None),
            generic_hit_sound_id: RetailRuntimeValue::Known(None),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: 0x39,
                common_axis_descriptor: TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR,
                behavior_choices: TYPE17_IMPACT_BEHAVIOR_CHOICES.to_vec().into_boxed_slice(),
                behavior_rule_ref: TYPE17_BEHAVIOR_RULE_REF,
                alternate_behavior_class_ref: 12,
            }),
            common_mover_topology: RetailRuntimeValue::Known(TYPE17_MODEL256_COMPONENT_TOPOLOGY),
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                SubAPropulsionDescriptor {
                    acceleration_raw: 100,
                    overspeed_correction_raw: 200,
                    target_speed_base_raw: TYPE17_MODEL256_SUB_A_TARGET_SPEED_BASE_RAW,
                },
            )),
            sub_h_external_frame_descriptor: RetailRuntimeValue::Known(Some(
                SubHExternalFrameDescriptor {
                    completion_sound_id: None,
                    records: vec![
                        SubHExternalFrameRecord {
                            resolver_flags_raw: 0,
                            phase_rate_raw: 0,
                            vertex_refs: [0; 3],
                            axis_mode_raw: 0,
                            dependencies: [0; 4],
                        };
                        TYPE17_MODEL256_SUB_H_RECORD_COUNT
                    ],
                },
            )),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn manager(metadata: &EntityTypeRuntimeMetadata) -> EntityManager {
        let mut entity = Entity::unresolved_port_entity(ENTITY_ID, EntityKind::Enemy, 17);
        entity.authored_spawn_index = Some(17);
        entity.model_slots = [Some(256); 4];
        entity.model_index = Some(256);
        entity.capability_flags = 8;
        entity.mass_raw = LEVEL_ONE_TYPE17_SELF_MASS_RAW;
        entity.collision = EntityCollisionRuntimeState::from_constructor(
            Some(metadata),
            0,
            RetailStateWord::from_known_bits(0x0147_8825, !SURFACE_STATE_MASK),
        );
        entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Known(None);
        entity.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Known(None);
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(
            BehaviorContextRuntime::named_audited(
                behavior_program(33).unwrap(),
                1,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(Some(0x1234)),
                RetailRuntimeValue::Known(0x1357),
                *audited_behavior_style(33, 1).unwrap(),
            )
            .unwrap(),
        ));
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(333), 1, 100),
        ));
        entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(
            SubHRuntimeState::new(TYPE17_MODEL256_SUB_H_RECORD_COUNT).unwrap(),
        ));
        entity.set_motion_raw([123, -456, 789], [11, -22, 33]);
        let mut all_metadata = vec![EntityTypeRuntimeMetadata::default(); 18];
        all_metadata[17] = metadata.clone();
        EntityManager::from_entities_with_type_metadata_for_test(vec![entity], all_metadata, true)
    }

    fn delivery(impact_raw: i32) -> DamageDeliveryRecord {
        DamageDeliveryRecord {
            packet: DamagePacket::collision(impact_raw),
            source_entity_type_raw: 17,
            owner_handle: ENTITY_ID,
        }
    }

    fn entity(manager: &EntityManager) -> &Entity {
        manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap()
    }

    #[test]
    fn strict_collision_filter_preserves_unrelated_unknown_state_and_no_projectile_prefix() {
        for (impact, filtered) in [(0, 0), (2000, 0), (2001, 1), (2500, 500), (i32::MAX, -2001)] {
            let metadata = metadata();
            let mut manager = manager(&metadata);
            let before = entity(&manager).current_behavior_context;
            let state_before = entity(&manager).collision.state_flags_at_0x08;
            let mut fx = WorldFx::new();
            let result =
                apply_type17_collision_damage(&mut manager, &metadata, &mut fx, delivery(impact))
                    .unwrap();
            assert_eq!(result.filtered_damage_raw, filtered);
            assert_eq!(result.damage_after_buffer_raw, filtered);
            assert_eq!(result.common_dying, None);
            assert_eq!(
                entity(&manager).collision.health_raw,
                RetailRuntimeValue::Known(5000 - filtered)
            );
            assert_eq!(
                entity(&manager)
                    .collision
                    .last_hit_presentation_tick_at_0x34,
                RetailRuntimeValue::Known(0)
            );
            assert_eq!(entity(&manager).collision.state_flags_at_0x08, state_before);
            assert_eq!(entity(&manager).current_behavior_context, before);
            assert_eq!(entity(&manager).velocity_raw(), [11, -22, 33]);
            assert_eq!(fx.pending_event_count(), 0);
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                WorldFx::new().next_shared_retail_random_u16()
            );
        }
    }

    #[test]
    fn disabled_and_filtered_zero_paths_do_not_require_unreached_fields() {
        for disabled in [false, true] {
            let metadata = metadata();
            let mut manager = manager(&metadata);
            let target = manager.entity_mut(ENTITY_ID).unwrap();
            target.collision.pair_callbacks.damage_modifier_address =
                RetailRuntimeValue::Unresolved;
            target.collision.health_raw = RetailRuntimeValue::Unresolved;
            target.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Unresolved;
            target
                .collision
                .state_flags_at_0x08
                .invalidate(REMOTE_OWNED_STATE_BIT | DYING_STATE_BIT);
            if disabled {
                target
                    .collision
                    .state_flags_at_0x08
                    .overwrite(CHECKED_DAMAGE_ENABLED_STATE_BIT, 0);
                target.collision.damage_profile = RetailRuntimeValue::Unresolved;
            }
            let result = apply_type17_collision_damage(
                &mut manager,
                &metadata,
                &mut WorldFx::new(),
                delivery(if disabled { 7000 } else { 2000 }),
            )
            .unwrap();
            assert_eq!(result, Type17CollisionDamageOutcome::ZERO);
        }
    }

    #[test]
    fn buffer_is_consumed_before_health_and_before_already_dying_gate() {
        for (buffer, after, residual) in [(600, 100, 0), (300, 0, 200), (-7, -7, 500)] {
            let metadata = metadata();
            let mut manager = manager(&metadata);
            manager
                .entity_mut(ENTITY_ID)
                .unwrap()
                .collision
                .pre_health_damage_buffer_raw = RetailRuntimeValue::Known(buffer);
            let result = apply_type17_collision_damage(
                &mut manager,
                &metadata,
                &mut WorldFx::new(),
                delivery(2500),
            )
            .unwrap();
            assert_eq!(result.damage_after_buffer_raw, residual);
            assert_eq!(
                entity(&manager).collision.pre_health_damage_buffer_raw,
                RetailRuntimeValue::Known(after)
            );
            assert_eq!(
                entity(&manager).collision.health_raw,
                RetailRuntimeValue::Known(5000 - residual)
            );
        }
        let metadata = metadata();
        let mut manager = manager(&metadata);
        let target = manager.entity_mut(ENTITY_ID).unwrap();
        target.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(300);
        target.collision.health_raw = RetailRuntimeValue::Unresolved;
        target.collision.generic_hit_sound_id = RetailRuntimeValue::Unresolved;
        target.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Unresolved;
        target
            .collision
            .state_flags_at_0x08
            .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
        let result = apply_type17_collision_damage(
            &mut manager,
            &metadata,
            &mut WorldFx::new(),
            delivery(2500),
        )
        .unwrap();
        assert_eq!(result.damage_after_buffer_raw, 200);
        assert_eq!(
            entity(&manager).collision.pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity(&manager).collision.health_raw,
            RetailRuntimeValue::Unresolved
        );
    }

    #[test]
    fn reached_callback_and_state_blocks_preserve_exact_prefixes() {
        for (phase, buffer_after, committed) in [
            (Type17CollisionDamagePhase::Modifier, 300, false),
            (Type17CollisionDamagePhase::RemoteOwner, 300, false),
            (Type17CollisionDamagePhase::Dying, 0, true),
            (Type17CollisionDamagePhase::HitCallback, 0, true),
            (Type17CollisionDamagePhase::Health, 0, true),
        ] {
            let metadata = metadata();
            let mut manager = manager(&metadata);
            let target = manager.entity_mut(ENTITY_ID).unwrap();
            target.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(300);
            match phase {
                Type17CollisionDamagePhase::Modifier => {
                    target.collision.pair_callbacks.damage_modifier_address =
                        RetailRuntimeValue::Known(Some(0x1234))
                }
                Type17CollisionDamagePhase::RemoteOwner => target
                    .collision
                    .state_flags_at_0x08
                    .overwrite(REMOTE_OWNED_STATE_BIT, REMOTE_OWNED_STATE_BIT),
                Type17CollisionDamagePhase::Dying => target
                    .collision
                    .state_flags_at_0x08
                    .invalidate(DYING_STATE_BIT),
                Type17CollisionDamagePhase::HitCallback => {
                    target.collision.pair_callbacks.type_hit_callback_address =
                        RetailRuntimeValue::Known(Some(0x1234))
                }
                Type17CollisionDamagePhase::Health => {
                    target.collision.health_raw = RetailRuntimeValue::Unresolved
                }
                _ => unreachable!(),
            }
            let failure = apply_type17_collision_damage(
                &mut manager,
                &metadata,
                &mut WorldFx::new(),
                delivery(2500),
            )
            .unwrap_err();
            assert_eq!(failure.phase, phase);
            assert_eq!(failure.committed_prefix, committed);
            assert_eq!(
                entity(&manager).collision.pre_health_damage_buffer_raw,
                RetailRuntimeValue::Known(buffer_after)
            );
            if phase != Type17CollisionDamagePhase::Health {
                assert_eq!(
                    entity(&manager).collision.health_raw,
                    RetailRuntimeValue::Known(5000)
                );
            }
        }
    }

    #[test]
    fn generic_hit_sound_precedes_callback_even_when_buffer_absorbs_everything() {
        let mut metadata = metadata();
        metadata.generic_hit_sound_id = RetailRuntimeValue::Known(Some(91));
        let mut manager = manager(&metadata);
        let target = manager.entity_mut(ENTITY_ID).unwrap();
        target.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(600);
        target.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Unresolved;
        let mut fx = WorldFx::new();
        let failure =
            apply_type17_collision_damage(&mut manager, &metadata, &mut fx, delivery(2500))
                .unwrap_err();
        assert_eq!(failure.phase, Type17CollisionDamagePhase::HitCallback);
        assert!(failure.committed_prefix);
        assert_eq!(
            entity(&manager).collision.health_raw,
            RetailRuntimeValue::Known(5000)
        );
        fx.process_pending();
        let sounds = fx.take_positional_sounds();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].sound_id, 91);
        assert_eq!(sounds[0].position, entity(&manager).position);
    }

    #[test]
    fn lethal_collision_publishes_common_dying_once_without_player_feedback() {
        let metadata = metadata();
        let mut manager = manager(&metadata);
        let mut fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        oracle.next_shared_retail_random_u16();
        let result =
            apply_type17_collision_damage(&mut manager, &metadata, &mut fx, delivery(7000))
                .unwrap();
        let owner = result
            .common_dying
            .expect("lethal self collision publishes class 12");
        assert_eq!(owner.entity_id(), ENTITY_ID);
        assert_eq!(
            entity(&manager).collision.health_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(entity(&manager).velocity_raw(), [11, 500, 33]);
        assert!(matches!(
            entity(&manager).actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::CommonDying(_))
        ));
        assert_eq!(
            entity(&manager)
                .collision
                .state_flags_at_0x08
                .masked(SURFACE_STATE_MASK),
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
        fx.process_pending();
        let sounds = fx.take_positional_sounds();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].sound_id, 94);
        let result =
            apply_type17_collision_damage(&mut manager, &metadata, &mut fx, delivery(7000))
                .unwrap();
        assert_eq!(result.common_dying, None);
        assert_eq!(fx.pending_event_count(), 0);
    }

    #[test]
    fn unresolved_death_suffix_retains_lethal_health_without_replaying_sound_or_rng() {
        let metadata = metadata();
        let mut manager = manager(&metadata);
        manager
            .entity_mut(ENTITY_ID)
            .unwrap()
            .sub_h_external_frame_runtime = RetailRuntimeValue::Unresolved;
        let before = entity(&manager).current_behavior_context;
        let mut fx = WorldFx::new();
        let failure =
            apply_type17_collision_damage(&mut manager, &metadata, &mut fx, delivery(7001))
                .unwrap_err();
        assert_eq!(failure.phase, Type17CollisionDamagePhase::Death);
        assert!(failure.committed_prefix);
        assert_eq!(
            entity(&manager).collision.health_raw,
            RetailRuntimeValue::Known(-1)
        );
        assert_eq!(entity(&manager).current_behavior_context, before);
        assert_eq!(fx.pending_event_count(), 0);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            WorldFx::new().next_shared_retail_random_u16()
        );
    }

    #[test]
    fn other_provenance_or_packet_families_cannot_enter_collision_adapter() {
        for wrong in [
            DamageDeliveryRecord {
                source_entity_type_raw: 46,
                ..delivery(2500)
            },
            DamageDeliveryRecord {
                packet: DamagePacket {
                    channels: [2, 0],
                    amounts_raw: [2500, 0],
                },
                ..delivery(2500)
            },
            DamageDeliveryRecord {
                packet: DamagePacket {
                    channels: [1, 0],
                    amounts_raw: [2500, 1],
                },
                ..delivery(2500)
            },
        ] {
            let metadata = metadata();
            let mut manager = manager(&metadata);
            let failure =
                apply_type17_collision_damage(&mut manager, &metadata, &mut WorldFx::new(), wrong)
                    .unwrap_err();
            assert_eq!(
                failure.reason,
                Type17CollisionDamageBlock::UnexpectedDelivery
            );
            assert!(!failure.committed_prefix);
            assert_eq!(
                entity(&manager).collision.health_raw,
                RetailRuntimeValue::Known(5000)
            );
        }
    }
}
