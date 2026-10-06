use super::*;
use crate::damage::{FUN_0043F780_DAMAGE_DELIVERY, FUN_0043F7C0_DAMAGE_DELIVERY};
use crate::gameplay_notifications::GameplayNotifications;
use crate::world_fx::BallisticDamageRequest;

#[v2k_test_support::retail_test]
fn lethal_cure_commits_the_native_constructor_effects_before_returning() {
    for extra_lives in [0, 1] {
        let (mut manager, mut hull, mut fx, resources) = fixture(
            500,
            0,
            CHECKED_DAMAGE_ENABLED_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
        );
        manager.player_mut().unwrap().collision.generic_hit_sound_id =
            RetailRuntimeValue::Known(Some(100));
        manager.player_mut().unwrap().collision.death_sound_id =
            RetailRuntimeValue::Known(Some(101));
        let mut notifications = GameplayNotifications::new();
        let mut expected_notifications = GameplayNotifications::new();
        let mut expected_fx = WorldFx::new();
        // Retail's eight-sample governor history admits the full scatter count.
        // A cold owner deliberately emits only one scatter plus the surface.
        for _ in 0..8 {
            fx.advance_frame_pacing(20_000);
            expected_fx.advance_frame_pacing(20_000);
        }
        for cue in [93, 100, 101] {
            expected_fx.emit_fixed_positional_sound_raw(cue, [0; 3]);
        }
        expected_fx.emit_player_wreck_burst_raw(
            [0; 3],
            resources.global_model(67).unwrap().radius >> 1,
            None,
            7,
        );
        expected_notifications.queue_player_destroyed(extra_lives, 100);
        let next_burst = 100 + i32::from(expected_fx.next_shared_retail_random_u16() >> 13);
        let result = manager.apply_player_model_switch_particle_hit(
            impact(EntityHitEntry::Cured),
            PlayerCheckedDamageFrame {
                hull: &mut hull,
                resources: &resources,
                world_fx: &mut fx,
                retail_tick: 100,
                notifications: &mut notifications,
                extra_lives,
            },
        );
        assert!(matches!(
            result,
            PlayerModelSwitchHitOutcome::Applied {
                model_slot: 1,
                checked: PlayerCheckedDamageOutcome::Applied { dying: true, .. },
                ..
            }
        ));
        assert_eq!(
            fx.test_particles_in_virgin_birth_order(),
            expected_fx.test_particles_in_virgin_birth_order()
        );
        assert_eq!(fx.particle_count(), 11);
        assert_eq!(
            fx.take_positional_sounds(),
            expected_fx.take_positional_sounds()
        );
        assert_eq!(notifications, expected_notifications);
        assert_eq!(
            manager
                .player_dying_contact_runtime()
                .unwrap()
                .next_burst_tick(),
            next_burst
        );
        let RetailRuntimeValue::Known(Some(context)) =
            manager.player().unwrap().current_behavior_context
        else {
            panic!("dying context");
        };
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::Named(
                crate::entity_behavior::audited_behavior_program(25).unwrap()
            )
        );
        assert!(!manager
            .begin_player_dying(PlayerCheckedDamageFrame {
                hull: &mut hull,
                resources: &resources,
                world_fx: &mut fx,
                retail_tick: 999,
                notifications: &mut notifications,
                extra_lives,
            })
            .unwrap());
        assert_eq!(fx.particle_count(), 11);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected_fx.next_shared_retail_random_u16()
        );
        assert_stamp(&manager);
    }
}

#[test]
fn contact_burst_uses_strict_signed_latch_and_reseeds_after_the_shared_burst() {
    let mut runtime = PlayerDyingContactRuntime {
        player_id: 7,
        next_burst_retail_tick: 100,
    };
    let mut fx = WorldFx::new();
    let mut expected = WorldFx::new();
    let request = PlayerDyingContactBurstRequest {
        position_raw: [12, 34, 56],
        source_extent_raw: 44,
        sea_level_raw: None,
        retail_tick: 100,
    };
    assert!(!runtime.emit_if_due(&mut fx, request));
    assert_eq!(fx.particle_count(), 0);
    assert!(runtime.emit_if_due(
        &mut fx,
        PlayerDyingContactBurstRequest {
            retail_tick: 101,
            ..request
        }
    ));
    expected.emit_player_wreck_burst_raw(request.position_raw, request.source_extent_raw, None, 7);
    let next = 101 + i32::from(expected.next_shared_retail_random_u16() >> 11);
    assert_eq!(runtime.next_burst_tick(), next);
    assert_eq!(
        fx.test_particles_in_virgin_birth_order(),
        expected.test_particles_in_virgin_birth_order()
    );
    assert_eq!(
        fx.take_positional_sounds(),
        expected.take_positional_sounds()
    );
    assert!(!runtime.emit_if_due(
        &mut fx,
        PlayerDyingContactBurstRequest {
            retail_tick: next as u32,
            ..request
        }
    ));
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
    // Signed retail comparison admits -1<0; it is not an unsigned timestamp.
    let mut signed = PlayerDyingContactRuntime {
        player_id: 7,
        next_burst_retail_tick: -1,
    };
    assert!(signed.emit_if_due(
        &mut WorldFx::new(),
        PlayerDyingContactBurstRequest {
            retail_tick: 0,
            ..request
        }
    ));
}

#[v2k_test_support::retail_test]
fn pending_replacement_or_unowned_death_allocation_blocks_before_the_prefix() {
    for pending in [
        RetailRuntimeValue::Known(Some(99)),
        RetailRuntimeValue::Unresolved,
    ] {
        let (mut manager, mut hull, mut fx, resources) = fixture(
            500,
            0,
            CHECKED_DAMAGE_ENABLED_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
        );
        manager.player_pending_replacement_handle = pending;
        let original = manager.player().unwrap().collision.clone();
        let mut notifications = GameplayNotifications::new();
        let result = manager.apply_player_model_switch_particle_hit(
            impact(EntityHitEntry::Cured),
            PlayerCheckedDamageFrame {
                hull: &mut hull,
                resources: &resources,
                world_fx: &mut fx,
                retail_tick: 100,
                notifications: &mut notifications,
                extra_lives: 1,
            },
        );
        assert!(matches!(
            result,
            PlayerModelSwitchHitOutcome::Blocked(PlayerModelSwitchHitBlock::Checked(_))
        ));
        assert_eq!(manager.player().unwrap().collision, original);
        assert_eq!(hull.health_raw, 500);
        assert!(fx.take_positional_sounds().is_empty());
        assert_eq!(fx.particle_count(), 0);
        assert_eq!(notifications, GameplayNotifications::new());
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            WorldFx::new().next_shared_retail_random_u16()
        );
    }
    let (mut manager, mut hull, mut fx, _) = fixture(
        500,
        0,
        CHECKED_DAMAGE_ENABLED_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
    );
    let result = manager.apply_player_model_switch_particle_hit(
        impact(EntityHitEntry::Cured),
        PlayerCheckedDamageFrame {
            hull: &mut hull,
            resources: &crate::resource_cache::ResourceCache::new(Vec::new()),
            world_fx: &mut fx,
            retail_tick: 100,
            notifications: &mut GameplayNotifications::new(),
            extra_lives: 0,
        },
    );
    assert_eq!(
        result,
        PlayerModelSwitchHitOutcome::Blocked(PlayerModelSwitchHitBlock::Checked(
            PlayerCheckedDamageBlock::Runtime("dying model allocation")
        ))
    );
    assert_eq!(hull.health_raw, 500);
    assert!(fx.take_positional_sounds().is_empty());
    assert_eq!(fx.particle_count(), 0);
}

#[v2k_test_support::retail_test]
fn direct_lethal_feedback_uses_the_delivered_source_word_after_native_death() {
    for source in [0, 46, (-5_i32) as u32] {
        let (mut manager, mut hull, mut fx, resources) =
            fixture(500, 0, CHECKED_DAMAGE_ENABLED_STATE_BIT | 0x0100_0000);
        let mut notifications = GameplayNotifications::new();
        let request = PlayerCheckedDamageRequest {
            entry: PlayerDamageEntry::Checked,
            target_id: 7,
            delivery: DamageDeliveryRecord {
                source_entity_type_raw: source,
                owner_handle: 99,
                ..FUN_0043F7C0_DAMAGE_DELIVERY
            },
            ratio_numerator: 0,
            ratio_denominator: 0,
        };
        assert!(matches!(
            manager.apply_player_checked_damage(
                request,
                PlayerCheckedDamageFrame {
                    hull: &mut hull,
                    resources: &resources,
                    world_fx: &mut fx,
                    retail_tick: 100,
                    notifications: &mut notifications,
                    extra_lives: 1,
                }
            ),
            PlayerCheckedDamageOutcome::Applied { dying: true, .. }
        ));
        let mut expected = GameplayNotifications::new();
        expected.queue_player_destroyed(1, 100);
        if source == 46 {
            expected.queue_player_kill(100);
        }
        assert_eq!(notifications, expected);
        assert_eq!(
            fx.particle_count(),
            2,
            "cold governor retains its source minimum"
        );
        assert_stamp(&manager);
    }
}

#[v2k_test_support::retail_test]
fn unchecked_radial_entry_filters_without_the_checked_admission_or_zero_feedback() {
    let (mut manager, mut hull, mut fx, resources) = fixture(40_000, 0, 0);
    let mut notifications = GameplayNotifications::new();
    let request = PlayerCheckedDamageRequest {
        entry: PlayerDamageEntry::Unchecked,
        target_id: 7,
        delivery: FUN_0043F7C0_DAMAGE_DELIVERY,
        ratio_numerator: 0,
        ratio_denominator: 0,
    };
    assert!(matches!(
        manager.apply_player_checked_damage(
            request,
            PlayerCheckedDamageFrame {
                hull: &mut hull,
                resources: &resources,
                world_fx: &mut fx,
                retail_tick: 100,
                notifications: &mut notifications,
                extra_lives: 0,
            }
        ),
        PlayerCheckedDamageOutcome::Applied {
            filtered_damage_raw: 800,
            dying: false,
            ..
        }
    ));
    assert_eq!(hull.health_raw, 39_200);
    manager.player_mut().unwrap().capability_flags = 8;
    assert_eq!(
        manager.apply_player_checked_damage(
            PlayerCheckedDamageRequest {
                delivery: DamageDeliveryRecord {
                    source_entity_type_raw: 46,
                    ..request.delivery
                },
                ratio_denominator: 1,
                ..request
            },
            PlayerCheckedDamageFrame {
                hull: &mut hull,
                resources: &resources,
                world_fx: &mut fx,
                retail_tick: 100,
                notifications: &mut notifications,
                extra_lives: 0,
            }
        ),
        PlayerCheckedDamageOutcome::FilteredOut
    );
    assert_stamp(&manager);
}

#[v2k_test_support::retail_test]
fn physical_begin_publishes_dying_state_and_its_death_cue_once() {
    let (mut manager, mut hull, mut fx, resources) =
        fixture(40_000, 200, ACTIVE_MODEL_SLOT_HIGH_STATE_BIT);
    manager.player_mut().unwrap().collision.death_sound_id = RetailRuntimeValue::Known(Some(101));
    hull.health_raw = 0;
    hull.dying = true;
    let mut notifications = GameplayNotifications::new();
    assert!(manager
        .begin_player_dying(PlayerCheckedDamageFrame {
            hull: &mut hull,
            resources: &resources,
            world_fx: &mut fx,
            retail_tick: 100,
            notifications: &mut notifications,
            extra_lives: 0,
        })
        .unwrap());
    let player = manager.player().unwrap();
    assert_eq!(player.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        player.collision.active_model_slot(),
        RetailRuntimeValue::Known(3)
    );
    assert_eq!(
        player.collision.pre_health_damage_buffer_raw,
        RetailRuntimeValue::Known(200)
    );
    assert_eq!(
        fx.take_positional_sounds()
            .iter()
            .map(|sound| sound.sound_id)
            .collect::<Vec<_>>(),
        [101, 62, 62]
    );
    assert!(!manager
        .begin_player_dying(PlayerCheckedDamageFrame {
            hull: &mut hull,
            resources: &resources,
            world_fx: &mut fx,
            retail_tick: 101,
            notifications: &mut notifications,
            extra_lives: 0,
        })
        .unwrap());
    assert!(fx.take_positional_sounds().is_empty());
    manager.remove_live_entity_ids(&[7]);
    assert_eq!(manager.player_dying_contact_runtime(), None);
}

fn resources() -> crate::resource_cache::ResourceCache {
    let data = v2k_test_support::retail_dir();
    let mut session = crate::session::GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.cache
}

fn fixture(
    health: i32,
    shield: i32,
    flags: u32,
) -> (
    EntityManager,
    PlayerHull,
    WorldFx,
    crate::resource_cache::ResourceCache,
) {
    let resources = resources();
    let mut player = Entity::unresolved_port_entity(7, EntityKind::Player, PLAYER_ENTITY_TYPE);
    player.mass_raw = 100;
    player.model_slots = [Some(41), Some(67), Some(75), Some(67)];
    player.collision.state_flags_at_0x08 = RetailStateWord::exact(flags);
    player.model_index = player.model_in_slot(active_model_slot_from_state_flags(flags));
    player.collision.health_raw = RetailRuntimeValue::Known(health);
    player.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(shield);
    player.collision.damage_profile =
        RetailRuntimeValue::Known(crate::damage::TYPE_46_DAMAGE_PROFILE);
    player.collision.generic_hit_sound_id = RetailRuntimeValue::Known(None);
    player.collision.death_sound_id = RetailRuntimeValue::Known(None);
    player.collision.constructor_sound_attachment_id_at_0x8c = RetailRuntimeValue::Known(None);
    player.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
    player.collision.pair_callbacks = EntityPairCallbackRuntimeState::audited_player();
    player.sub_j_attachment_runtime = RetailRuntimeValue::Known(None);
    player.current_behavior_context = RetailRuntimeValue::Known(Some(
        BehaviorContextRuntime::from_fresh_weighted_selection(BehaviorSelection {
            choice_index: 0,
            program: behavior_program(24).unwrap(),
        })
        .unwrap(),
    ));
    let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 47];
    metadata[46] =
        EntityTypeRuntimeMetadata::from_section12(resources.global_entity_type(46).unwrap());
    metadata[46].infected_model_presentation_sound_id = RetailRuntimeValue::Known(Some(92));
    metadata[46].cured_model_presentation_sound_id = RetailRuntimeValue::Known(Some(93));
    let manager =
        EntityManager::from_entities_with_type_metadata_for_test(vec![player], metadata, true);
    let mut hull = PlayerHull::default();
    hull.health_raw = health;
    hull.pre_health_damage_buffer_raw = shield;
    hull.dying = flags & DYING_STATE_BIT != 0;
    (manager, hull, WorldFx::new(), resources)
}

fn impact(entry: EntityHitEntry) -> ParticleEntityImpact {
    let delivery = if entry == EntityHitEntry::Infected {
        FUN_0043F780_DAMAGE_DELIVERY
    } else {
        FUN_0043F7C0_DAMAGE_DELIVERY
    };
    ParticleEntityImpact {
        source_particle_class: if entry == EntityHitEntry::Infected {
            5
        } else {
            6
        },
        impact_position_argument_va: 0x1234,
        target_entity_id: 7,
        position_world: [0.0; 3],
        velocity_raw: [i16::MAX, i16::MIN, 0],
        // F780/F7C0 use their unchanged static packet, not birth provenance.
        damage: Some(BallisticDamageRequest {
            packet: delivery.packet,
            source_entity_type_at_birth: Some(46),
            source_owner_id: Some(7),
        }),
    }
}

fn assert_stamp(manager: &EntityManager) {
    assert_eq!(
        manager
            .player()
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(17)
    );
}

#[v2k_test_support::retail_test]
fn authored_normal_tier_player_models_and_damage_match_the_live_contract() {
    let data = v2k_test_support::retail_dir();
    let mut session =
        crate::session::GameSession::init(&data).expect("retail preload data required");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("normal-tier retail data required");
    let record = session
        .cache
        .global_entity_type(46)
        .expect("authored player row");
    assert_eq!(
        session.cache.global_entity_model_table()[46],
        [41, 67, 41, 67]
    );
    let metadata = EntityTypeRuntimeMetadata::from_section12(record);
    assert_eq!(metadata.mass_raw, 100);
    assert_eq!(metadata.initial_health_raw, Some(40_000));
    assert_eq!(
        metadata.constructor_sound_attachment_id,
        RetailRuntimeValue::Known(None)
    );
    assert_eq!(
        metadata.damage_profile,
        Some(crate::damage::TYPE_46_DAMAGE_PROFILE)
    );
    assert_eq!(
        metadata.infected_model_presentation_sound_id,
        RetailRuntimeValue::Known(None)
    );
    assert_eq!(
        metadata.cured_model_presentation_sound_id,
        RetailRuntimeValue::Known(None)
    );
    assert_eq!(
        (
            record.behavior_rule_ref,
            record.alternate_behavior_class_ref
        ),
        (1, 25)
    );
}

#[v2k_test_support::retail_test]
fn local_controlled_constructor_publishes_class24_for_native_and_fresh_entries() {
    let resources = resources();
    let authored = EntityTypeRuntimeMetadata::from_section12(
        resources.global_entity_type(46).expect("authored Type46"),
    );
    for fresh in [false, true] {
        let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 47];
        metadata[46] = authored.clone();
        let mut manager =
            EntityManager::from_entities_with_type_metadata_for_test(Vec::new(), metadata, fresh);
        manager.append_persistent_player(Some(&authored), [0.0; 3], 0.0, None);
        let player = manager.player().expect("controlled allocation");
        let RetailRuntimeValue::Known(Some(context)) = player.current_behavior_context else {
            panic!("successful local controller must publish its selected context");
        };
        assert!(crate::player_contact_style::is_player_style(context, 24));
        assert_eq!(
            player.model_slots[0],
            Some(usize::from(authored.model_slots[0]))
        );
        assert_eq!(
            player.collision.pair_callbacks,
            EntityPairCallbackRuntimeState::audited_player()
        );
    }
}

#[v2k_test_support::retail_test]
fn local_controlled_constructor_does_not_publish_from_missing_or_foreign_metadata() {
    let resources = resources();
    let authored = EntityTypeRuntimeMetadata::from_section12(
        resources.global_entity_type(46).expect("authored Type46"),
    );
    for mutation in 0..3 {
        let mut metadata = authored.clone();
        match mutation {
            0 => metadata.model_slots[0] = 99,
            1 => metadata.initializer = None,
            _ => metadata.initializer.as_mut().unwrap().behavior_choices = Box::new([]),
        }
        let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
            Vec::new(),
            vec![EntityTypeRuntimeMetadata::default(); 47],
            false,
        );
        manager.append_persistent_player(Some(&metadata), [0.0; 3], 0.0, None);
        assert!(
            !matches!(manager.player().unwrap().current_behavior_context,
            RetailRuntimeValue::Known(Some(context))
                if crate::player_contact_style::is_player_style(context, 24))
        );
    }
}

#[v2k_test_support::retail_test]
fn infection_then_cure_switches_the_live_model_impulse_hull_and_shared_rng() {
    let (mut manager, mut hull, mut fx, resources) = fixture(
        40_000,
        0,
        CHECKED_DAMAGE_ENABLED_STATE_BIT | IMPACT_REACTION_ENABLED_STATE_BIT,
    );
    let mut expected_fx = WorldFx::new();
    let mut expected = ImpactReactionBody {
        state_flags_at_0x08: IMPACT_REACTION_ENABLED_STATE_BIT,
        mass_raw_at_0xb0: 100,
        linear_velocity_xyz_raw: [0; 3],
        angular_heading_pitch_roll_raw: [0; 3],
    };
    for (entry, model, health) in [
        (EntityHitEntry::Infected, 75, 36_000),
        (EntityHitEntry::Cured, 41, 35_200),
    ] {
        let hit = impact(entry);
        apply_impact_reaction(
            &mut expected,
            7,
            hit.damage.unwrap().packet.impact_sum_raw(),
            hit.velocity_raw,
            || u32::from(expected_fx.next_shared_retail_random_u16()),
        )
        .unwrap();
        let outcome = manager.apply_player_model_switch_particle_hit(
            hit,
            PlayerCheckedDamageFrame {
                hull: &mut hull,
                resources: &resources,
                world_fx: &mut fx,
                retail_tick: 0,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                extra_lives: 0,
            },
        );
        assert!(
            matches!(outcome, PlayerModelSwitchHitOutcome::Applied { .. }),
            "{outcome:?}"
        );
        assert_eq!(manager.player().unwrap().model_index, Some(model));
        assert_eq!(hull.health_raw, health);
        assert_eq!(
            manager.player().unwrap().velocity_raw(),
            expected.linear_velocity_xyz_raw
        );
        assert_eq!(
            manager.player().unwrap().rotation_heading_pitch_roll_raw(),
            expected.angular_heading_pitch_roll_raw
        );
        assert_stamp(&manager);
    }
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_fx.next_shared_retail_random_u16()
    );
    assert_eq!(
        fx.take_positional_sounds()
            .iter()
            .map(|sound| sound.sound_id)
            .collect::<Vec<_>>(),
        [92, 93]
    );
}

#[v2k_test_support::retail_test]
fn dying_cure_plays_its_cue_and_consumes_shield_without_restarting_health() {
    let (mut manager, mut hull, mut fx, resources) = fixture(
        0,
        1000,
        CHECKED_DAMAGE_ENABLED_STATE_BIT | DYING_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
    );
    let cure = manager.apply_player_model_switch_particle_hit(
        impact(EntityHitEntry::Cured),
        PlayerCheckedDamageFrame {
            hull: &mut hull,
            resources: &resources,
            world_fx: &mut fx,
            retail_tick: 0,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            extra_lives: 0,
        },
    );
    assert!(matches!(
        cure,
        PlayerModelSwitchHitOutcome::Applied {
            model_slot: 1,
            checked: PlayerCheckedDamageOutcome::Applied {
                filtered_damage_raw: 800,
                dying: true,
                ..
            },
            ..
        }
    ));
    assert_eq!(hull.health_raw, 0);
    assert_eq!(hull.pre_health_damage_buffer_raw, 200);
    assert_eq!(
        manager.player().unwrap().collision.active_model_slot(),
        RetailRuntimeValue::Known(1)
    );
    let infection = manager.apply_player_model_switch_particle_hit(
        impact(EntityHitEntry::Infected),
        PlayerCheckedDamageFrame {
            hull: &mut hull,
            resources: &resources,
            world_fx: &mut fx,
            retail_tick: 0,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            extra_lives: 0,
        },
    );
    assert!(matches!(
        infection,
        PlayerModelSwitchHitOutcome::Applied { model_slot: 3, .. }
    ));
    assert_eq!(hull.pre_health_damage_buffer_raw, 0);
    assert_eq!(
        fx.take_positional_sounds()
            .iter()
            .map(|sound| sound.sound_id)
            .collect::<Vec<_>>(),
        [93]
    );
    assert_stamp(&manager);
}

#[v2k_test_support::retail_test]
fn ineligible_checked_tail_still_runs_cure_prefix_without_stamping() {
    let (mut manager, mut hull, mut fx, resources) =
        fixture(40_000, 0, ACTIVE_MODEL_SLOT_HIGH_STATE_BIT);
    let outcome = manager.apply_player_model_switch_particle_hit(
        impact(EntityHitEntry::Cured),
        PlayerCheckedDamageFrame {
            hull: &mut hull,
            resources: &resources,
            world_fx: &mut fx,
            retail_tick: 0,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            extra_lives: 0,
        },
    );
    assert!(matches!(
        outcome,
        PlayerModelSwitchHitOutcome::Applied {
            checked: PlayerCheckedDamageOutcome::Ineligible,
            ..
        }
    ));
    assert_eq!(hull.health_raw, 40_000);
    assert_eq!(manager.player().unwrap().model_index, Some(41));
    assert_stamp(&manager);
}

#[v2k_test_support::retail_test]
fn filtered_zero_keeps_model_prefix_and_impulse_without_primary_presentation() {
    let (mut manager, mut hull, mut fx, resources) = fixture(
        40_000,
        0,
        CHECKED_DAMAGE_ENABLED_STATE_BIT
            | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT
            | IMPACT_REACTION_ENABLED_STATE_BIT,
    );
    manager.player_mut().unwrap().collision.damage_profile =
        RetailRuntimeValue::Known(crate::damage::DamageProfile {
            thresholds_raw: [0; 7],
            multipliers_q8: [0; 7],
        });
    let outcome = manager.apply_player_model_switch_particle_hit(
        impact(EntityHitEntry::Cured),
        PlayerCheckedDamageFrame {
            hull: &mut hull,
            resources: &resources,
            world_fx: &mut fx,
            retail_tick: 0,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            extra_lives: 0,
        },
    );
    assert!(matches!(
        outcome,
        PlayerModelSwitchHitOutcome::Applied {
            checked: PlayerCheckedDamageOutcome::FilteredOut,
            ..
        }
    ));
    assert_eq!(hull.health_raw, 40_000);
    assert_ne!(manager.player().unwrap().velocity_raw(), [0; 3]);
    assert_eq!(fx.take_positional_sounds().len(), 1);
    assert_stamp(&manager);
}

#[v2k_test_support::retail_test]
fn unresolved_style_or_controller_links_block_before_prefix_sound_and_rng() {
    for links in [false, true] {
        let (mut manager, mut hull, mut fx, resources) = fixture(
            40_000,
            0,
            CHECKED_DAMAGE_ENABLED_STATE_BIT
                | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT
                | IMPACT_REACTION_ENABLED_STATE_BIT,
        );
        if links {
            let descriptor = SubJAttachmentDescriptor {
                reserved_at_0x01: 0,
                slots: vec![SubJAttachmentSlotDescriptor {
                    policy_word_raw: 0,
                    local_offset_raw: [0; 3],
                }]
                .into_boxed_slice(),
            };
            let mut list = SubJAttachmentRuntime::from_descriptor(&descriptor).unwrap();
            list.append(8).unwrap();
            manager.player_mut().unwrap().sub_j_attachment_runtime =
                RetailRuntimeValue::Known(Some(list));
        } else {
            manager.player_mut().unwrap().current_behavior_context = RetailRuntimeValue::Unresolved;
        }
        let before = manager.player().unwrap().collision.clone();
        let outcome = manager.apply_player_model_switch_particle_hit(
            impact(EntityHitEntry::Cured),
            PlayerCheckedDamageFrame {
                hull: &mut hull,
                resources: &resources,
                world_fx: &mut fx,
                retail_tick: 0,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                extra_lives: 0,
            },
        );
        assert_eq!(
            outcome,
            PlayerModelSwitchHitOutcome::Blocked(if links {
                PlayerModelSwitchHitBlock::Checked(PlayerCheckedDamageBlock::LinkedAttachments)
            } else {
                PlayerModelSwitchHitBlock::Style
            })
        );
        assert_eq!(manager.player().unwrap().collision, before);
        assert_eq!(manager.player().unwrap().velocity_raw(), [0; 3]);
        assert_eq!(hull.health_raw, 40_000);
        assert!(fx.take_positional_sounds().is_empty());
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            WorldFx::new().next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn lethal_preflight_authenticates_the_prospective_dying_slot() {
    for (entry, flags, missing_slot) in [
        (EntityHitEntry::Cured, ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, 1),
        (EntityHitEntry::Infected, 0, 3),
    ] {
        let (mut manager, mut hull, mut fx, resources) = fixture(
            500,
            0,
            flags | CHECKED_DAMAGE_ENABLED_STATE_BIT | IMPACT_REACTION_ENABLED_STATE_BIT,
        );
        manager.player_mut().unwrap().model_slots[missing_slot] = None;
        let before = manager.player().unwrap().collision.clone();
        let outcome = manager.apply_player_model_switch_particle_hit(
            impact(entry),
            PlayerCheckedDamageFrame {
                hull: &mut hull,
                resources: &resources,
                world_fx: &mut fx,
                retail_tick: 0,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                extra_lives: 0,
            },
        );
        assert_eq!(
            outcome,
            PlayerModelSwitchHitOutcome::Blocked(PlayerModelSwitchHitBlock::Checked(
                PlayerCheckedDamageBlock::Runtime("dying model slot")
            ))
        );
        assert_eq!(manager.player().unwrap().collision, before);
        assert_eq!(hull.health_raw, 500);
        assert!(fx.take_positional_sounds().is_empty());
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            WorldFx::new().next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn lethal_model_switch_keeps_the_post_prefix_infection_selector() {
    for (entry, slot, damage) in [
        (EntityHitEntry::Cured, 1, 800),
        (EntityHitEntry::Infected, 3, 4000),
    ] {
        let (mut manager, mut hull, mut fx, resources) = fixture(
            500,
            0,
            CHECKED_DAMAGE_ENABLED_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
        );
        let outcome = manager.apply_player_model_switch_particle_hit(
            impact(entry),
            PlayerCheckedDamageFrame {
                hull: &mut hull,
                resources: &resources,
                world_fx: &mut fx,
                retail_tick: 0,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                extra_lives: 0,
            },
        );
        assert!(
            matches!(outcome, PlayerModelSwitchHitOutcome::Applied { checked: PlayerCheckedDamageOutcome::Applied { filtered_damage_raw, dying: true, .. }, .. } if filtered_damage_raw == damage)
        );
        assert_eq!(hull.health_raw, 0);
        assert!(hull.dying);
        assert_eq!(
            manager.player().unwrap().collision.active_model_slot(),
            RetailRuntimeValue::Known(slot)
        );
        assert_eq!(manager.switch_player_to_dying_model(), Some(67));
        assert_eq!(
            manager.player().unwrap().collision.active_model_slot(),
            RetailRuntimeValue::Known(slot)
        );
        assert_stamp(&manager);
    }
}

#[v2k_test_support::retail_test]
fn direct_scaled_checked_entry_retains_provenance_ratio_and_stamp() {
    let (mut manager, mut hull, mut fx, resources) =
        fixture(40_000, 100, CHECKED_DAMAGE_ENABLED_STATE_BIT);
    let outcome = manager.apply_player_checked_damage(
        PlayerCheckedDamageRequest {
            entry: PlayerDamageEntry::Checked,
            target_id: 7,
            delivery: DamageDeliveryRecord {
                source_entity_type_raw: (-5_i32) as u32,
                owner_handle: 7,
                ..FUN_0043F7C0_DAMAGE_DELIVERY
            },
            ratio_numerator: 0xABCD_0002,
            ratio_denominator: 4,
        },
        PlayerCheckedDamageFrame {
            hull: &mut hull,
            resources: &resources,
            world_fx: &mut fx,
            retail_tick: 0,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            extra_lives: 0,
        },
    );
    assert!(matches!(
        outcome,
        PlayerCheckedDamageOutcome::Applied {
            filtered_damage_raw: 400,
            dying: false,
            ..
        }
    ));
    assert_eq!(hull.pre_health_damage_buffer_raw, 0);
    assert_eq!(hull.health_raw, 39_700);
    assert_eq!(manager.player().unwrap().model_index, Some(41));
    assert_stamp(&manager);
}

#[v2k_test_support::retail_test]
fn direct_checked_zero_feedback_remains_a_named_boundary() {
    let (mut manager, mut hull, mut fx, resources) =
        fixture(40_000, 0, CHECKED_DAMAGE_ENABLED_STATE_BIT);
    manager.player_mut().unwrap().capability_flags = 8;
    let request = PlayerCheckedDamageRequest {
        entry: PlayerDamageEntry::Checked,
        target_id: 7,
        delivery: DamageDeliveryRecord {
            source_entity_type_raw: 46,
            ..FUN_0043F7C0_DAMAGE_DELIVERY
        },
        ratio_numerator: 0,
        ratio_denominator: 1,
    };
    assert_eq!(
        manager.apply_player_checked_damage(
            request,
            PlayerCheckedDamageFrame {
                hull: &mut hull,
                resources: &resources,
                world_fx: &mut fx,
                retail_tick: 0,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                extra_lives: 0
            }
        ),
        PlayerCheckedDamageOutcome::Blocked(PlayerCheckedDamageBlock::PlayerFeedback)
    );
    assert_eq!(
        manager.apply_player_checked_damage(
            PlayerCheckedDamageRequest {
                delivery: FUN_0043F7C0_DAMAGE_DELIVERY,
                ..request
            },
            PlayerCheckedDamageFrame {
                hull: &mut hull,
                resources: &resources,
                world_fx: &mut fx,
                retail_tick: 0,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                extra_lives: 0
            }
        ),
        PlayerCheckedDamageOutcome::FilteredOut
    );
    assert_stamp(&manager);
}

#[v2k_test_support::retail_test]
fn suppression_and_foreign_targets_never_run_the_player_prefix() {
    let (mut manager, mut hull, mut fx, resources) = fixture(
        40_000,
        0,
        CHECKED_DAMAGE_ENABLED_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
    );
    let hit = impact(EntityHitEntry::Cured);
    assert_eq!(
        manager.apply_player_model_switch_particle_hit(
            ParticleEntityImpact {
                damage: None,
                ..hit
            },
            PlayerCheckedDamageFrame {
                hull: &mut hull,
                resources: &resources,
                world_fx: &mut fx,
                retail_tick: 0,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                extra_lives: 0
            }
        ),
        PlayerModelSwitchHitOutcome::Suppressed
    );
    assert_eq!(
        manager.apply_player_model_switch_particle_hit(
            ParticleEntityImpact {
                target_entity_id: 8,
                ..hit
            },
            PlayerCheckedDamageFrame {
                hull: &mut hull,
                resources: &resources,
                world_fx: &mut fx,
                retail_tick: 0,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                extra_lives: 0
            }
        ),
        PlayerModelSwitchHitOutcome::NotApplicable
    );
    assert_eq!(manager.player().unwrap().model_index, Some(75));
    assert_eq!(hull.health_raw, 40_000);
    assert!(fx.take_positional_sounds().is_empty());
    assert_stamp(&manager);
}

#[v2k_test_support::retail_test]
fn authored_class6_traversal_reaches_the_playing_player_cure_bridge() {
    use crate::playing_particle_host::{
        PlayingParticleActorEvent, PlayingParticleActorResponse, PlayingParticleHost,
        PlayingParticleRefresh,
    };
    use crate::world_fx::{
        DescriptorParticleRequest, EntityCollisionModel, ParticleCollisionCacheRefresh,
        ParticleEnvironment, ParticleOwnerAtBirth, ParticleTerrainResponse,
        ParticleTraversalTiming,
    };

    let data = v2k_test_support::retail_dir();
    let mut session = crate::session::GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    let (mut manager, mut hull, mut fx, _resources) = fixture(
        40_000,
        0,
        CHECKED_DAMAGE_ENABLED_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
    );
    // The authored local player repeats its normal/wreck models in the
    // infected slots. The synthetic selector test above deliberately uses
    // distinct models to expose high-bit selection independently.
    manager.player_mut().unwrap().model_slots = [Some(41), Some(67), Some(41), Some(67)];
    manager.player_mut().unwrap().model_index = Some(41);
    let collision_model = EntityCollisionModel {
        entity_id: 7,
        center_world: [0.0; 3],
        radius_raw: session.cache.global_model(41).unwrap().collision_radius_raw,
        model_id: 41,
        orientation_world_from_model: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        anim_vars: crate::player::PlayerCraft::new().anim_vars(),
        state_flags_at_0x08: Some(
            CHECKED_DAMAGE_ENABLED_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
        ),
        capability_flags_at_0x64: 5,
    };
    fx.materialize_descriptor_particle_request(
        DescriptorParticleRequest {
            source_class: 6,
            position_raw: [-2000, 0, 0],
            velocity_raw: [10000, 0, 0],
            // F7C0's model sweep requires the retained emitting handle.
            // This incoming emitter is distinct from the controlled target.
            owner: Some(ParticleOwnerAtBirth {
                entity_id: 99,
                entity_type: 46,
            }),
            suppresses_impact_damage: false,
        },
        ParticleEnvironment::Dry,
        0,
    )
    .unwrap();
    let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
    let mut applied = Vec::new();
    let mut host = PlayingParticleHost {
        cache: &mut session.cache,
        static_damage: &mut static_damage,
        owner_motions: Vec::new(),
        collision_models: vec![collision_model.clone()],
        attachment_owners: Vec::new(),
        on_actor_event: |cache: &mut crate::resource_cache::ResourceCache,
                         _: &mut crate::static_damage::StaticDamageScheduler,
                         fx: &mut WorldFx,
                         event| {
            let PlayingParticleActorEvent::Impact(impact) = event else {
                panic!("class6 must enter the direct impact bridge");
            };
            assert_eq!(impact.source_particle_class, 6);
            assert_eq!(
                impact.damage_delivery_record(),
                Some(FUN_0043F7C0_DAMAGE_DELIVERY)
            );
            applied.push(manager.apply_player_model_switch_particle_hit(
                impact,
                PlayerCheckedDamageFrame {
                    hull: &mut hull,
                    resources: cache,
                    world_fx: fx,
                    retail_tick: 0,
                    notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                    extra_lives: 0,
                },
            ));
            PlayingParticleActorResponse::Impact(PlayingParticleRefresh {
                collision: ParticleCollisionCacheRefresh::Replace(vec![EntityCollisionModel {
                    state_flags_at_0x08: Some(CHECKED_DAMAGE_ENABLED_STATE_BIT),
                    ..collision_model.clone()
                }]),
                owner_motions: Vec::new(),
                attachment_owners: Vec::new(),
            })
        },
        on_terrain_event: |_: &mut crate::static_damage::StaticDamageScheduler,
                           _: &mut WorldFx,
                           _| ParticleTerrainResponse::Unhandled,
    };
    for frame in 0..16 {
        let outcome = fx.update_with_traversal_host(
            ParticleTraversalTiming {
                elapsed_micros: 40000,
                retail_tick: frame * 2,
            },
            &mut host,
        );
        assert!(outcome.blocked_attached_updates.is_empty());
    }
    drop(host);
    assert!(
        matches!(
            applied.as_slice(),
            [PlayerModelSwitchHitOutcome::Applied {
                entry: EntityHitEntry::Cured,
                checked: PlayerCheckedDamageOutcome::Applied {
                    filtered_damage_raw: 800,
                    dying: false,
                    ..
                },
                ..
            }]
        ),
        "{applied:?}"
    );
    assert_eq!(hull.health_raw, 39_200);
    assert_eq!(
        manager.player().unwrap().collision.active_model_slot(),
        RetailRuntimeValue::Known(0)
    );
    assert_stamp(&manager);
    assert_eq!(
        fx.particle_count(),
        0,
        "consumed native parent must not replay"
    );
}
