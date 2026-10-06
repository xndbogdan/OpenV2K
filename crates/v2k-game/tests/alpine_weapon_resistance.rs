//! Alpine's authored damage filters, independently of live actor ownership.
//!
//! These tests establish neither Type30/40 task publication nor their death
//! programs. They connect the real OVL17 cohort and the canonical executable
//! weapon/particle tables to the shared `FUN_004255E0` arithmetic.

use v2k_game::{
    damage::{
        DamagePacket, DamageProfile, CLASS49_TURRET_BOLT_DAMAGE_PACKET,
        CLASS56_TURRET_BOLT_DAMAGE_PACKET, DRAGON_FIREBALL_DAMAGE_PACKET,
        FUN_0043F6E0_DAMAGE_PACKET, FUN_0043F780_DAMAGE_PACKET, PRIMARY_PROJECTILE_DAMAGE_PACKET,
        TURRET_BOLT_DAMAGE_PACKET,
    },
    entity_collision_state::EntityTypeRuntimeMetadata,
    particle_descriptors::particle_descriptor,
    primary_weapon::fire_profile_from_descriptor,
    session::GameSession,
    weapon_inventory::{WeaponDescriptor, WEAPON_MASTER_TABLE_RAW},
};

fn alpine() -> GameSession {
    let data = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data).expect("retail corpus required");
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(17, 1).unwrap();
    session
}

fn damage_profile(session: &GameSession, entity_type: usize) -> DamageProfile {
    let record = session.cache.global_entity_type(entity_type).unwrap();
    EntityTypeRuntimeMetadata::from_section12(record)
        .damage_profile
        .unwrap()
}

#[v2k_test_support::retail_test]
fn alpine_insect_cohort_authors_two_distinct_resistance_and_death_profiles() {
    let session = alpine();
    let level = session.cache.level_desc().unwrap();
    assert_eq!(level.name, "Alpine");
    for (entity_type, model_id, name, health, alternate, indices, thresholds, multipliers) in [
        (
            30,
            1042,
            "icecurly",
            12_000,
            12, // Flip Over And Die, unlike Type40's Split And Explode.
            vec![10, 11, 12, 13, 14, 15, 16],
            [0, 4_000, 4_000, 500, 200, 0, 0],
            [0, 256, 256, 256, 128, 0, 0],
        ),
        (
            40,
            1040,
            "icelous",
            8_000,
            18,
            vec![2, 23],
            [0, 2_000, 3_000, 0, 200, 0, 0],
            [0, 256, 256, 512, 128, 0, 0],
        ),
    ] {
        let record = session.cache.global_entity_type(entity_type).unwrap();
        let metadata = EntityTypeRuntimeMetadata::from_section12(record);
        assert_eq!(record.model_ids, [model_id; 4]);
        assert_eq!(
            session
                .cache
                .global_model(usize::from(model_id))
                .unwrap()
                .name
                .as_deref(),
            Some(name)
        );
        assert_eq!(metadata.initial_health_raw, Some(health));
        assert_eq!(metadata.capability_flags, 8);
        assert_eq!(record.alternate_behavior_class_ref, alternate);
        assert_eq!(
            metadata.damage_profile,
            Some(DamageProfile {
                thresholds_raw: thresholds,
                multipliers_q8: multipliers,
            })
        );
        let cohort: Vec<_> = level
            .entities
            .iter()
            .filter(|spawn| spawn.entity_type as usize == entity_type)
            .collect();
        assert_eq!(
            cohort.iter().map(|spawn| spawn.index).collect::<Vec<_>>(),
            indices
        );
        assert!(cohort
            .iter()
            .all(|spawn| spawn.initial_damage_buffer_raw == 0));
    }
}

#[v2k_test_support::retail_test]
fn canonical_weapon_packets_explain_machine_gun_immunity_without_a_plasma_only_gate() {
    let session = alpine();
    let profiles = [damage_profile(&session, 30), damage_profile(&session, 40)];

    // Expected results are per direct hit, before any optional delivery ratio,
    // buffer, or death callback. Explosion/splash totals are separate programs.
    for (selector, class, packet_va, packet, expected) in [
        (1, 1, 0x004c_bf70, PRIMARY_PROJECTILE_DAMAGE_PACKET, [0, 0]),
        (2, 3, 0x004c_bf88, PRIMARY_PROJECTILE_DAMAGE_PACKET, [0, 0]),
        (
            0x12,
            2,
            0x004c_bf70,
            PRIMARY_PROJECTILE_DAMAGE_PACKET,
            [0, 0],
        ),
        (
            0x0e,
            55,
            0x004c_c0a8,
            TURRET_BOLT_DAMAGE_PACKET,
            [500, 2_000],
        ),
        (
            0x0d,
            56,
            0x004c_c090,
            CLASS56_TURRET_BOLT_DAMAGE_PACKET,
            [1_000, 4_000],
        ),
        (
            0x0c,
            49,
            0x004c_c078,
            CLASS49_TURRET_BOLT_DAMAGE_PACKET,
            [3_500, 7_000],
        ),
        (
            6,
            4,
            0x004c_bfa0,
            FUN_0043F6E0_DAMAGE_PACKET,
            [6_000, 7_000],
        ),
        (
            0x13,
            54,
            0x004c_bfa0,
            FUN_0043F6E0_DAMAGE_PACKET,
            [6_000, 7_000],
        ),
        (
            0x0a,
            38,
            0x004c_c048,
            DRAGON_FIREBALL_DAMAGE_PACKET,
            [5_500, 12_000],
        ),
        (
            0x16,
            39,
            0x004c_c048,
            DRAGON_FIREBALL_DAMAGE_PACKET,
            [5_500, 12_000],
        ),
        (8, 5, 0x004c_bfd0, FUN_0043F780_DAMAGE_PACKET, [0, 0]),
    ] {
        let descriptor = WEAPON_MASTER_TABLE_RAW
            .into_iter()
            .map(WeaponDescriptor::from_raw)
            .find(|descriptor| descriptor.selector() == selector)
            .unwrap();
        let fire = fire_profile_from_descriptor(&descriptor).unwrap();
        assert_eq!(
            fire.projectile.projectile_class, class,
            "selector {selector:#x}"
        );
        let particle = particle_descriptor(class).unwrap();
        assert_eq!(particle.raw_u32(0x20), packet_va, "selector {selector:#x}");
        assert_eq!(
            profiles.map(|profile| profile.filter(packet)),
            expected,
            "selector {selector:#x}, class {class}"
        );
    }
}

#[v2k_test_support::retail_test]
fn resistance_uses_strict_per_slot_thresholds_and_scales_only_the_excess() {
    let session = alpine();
    for entity_type in [30, 40] {
        let profile = damage_profile(&session, entity_type);
        for channel in 1..=4 {
            let threshold = profile.thresholds_raw[channel];
            for amount in [threshold - 1, threshold] {
                assert_eq!(
                    profile.filter(DamagePacket {
                        channels: [channel as i32, 0],
                        amounts_raw: [amount, 0],
                    }),
                    0,
                    "Type{entity_type}, channel{channel}, amount{amount}"
                );
            }
            assert_eq!(
                profile.filter(DamagePacket {
                    channels: [channel as i32, 0],
                    amounts_raw: [threshold + 256, 0],
                }),
                profile.multipliers_q8[channel],
                "the same channel can hurt when a stronger packet exceeds its threshold"
            );
        }
        let packet = DamagePacket {
            channels: [2, 3],
            amounts_raw: [profile.thresholds_raw[2], profile.thresholds_raw[3] + 500],
        };
        let expected = if entity_type == 30 { 500 } else { 1_000 };
        assert_eq!(
            profile.filter(packet),
            expected,
            "blocked first slot does not block second"
        );
        assert_eq!(
            profile.filter(DamagePacket {
                channels: [3, 2],
                amounts_raw: [packet.amounts_raw[1], packet.amounts_raw[0]],
            }),
            expected
        );
    }
}
