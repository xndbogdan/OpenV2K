//! Recovered weapon-selector projectile rows and `FUN_0044EA60` families.

use std::time::Duration;

use v2k_game::particle_descriptors::particle_descriptor;
use v2k_game::primary_weapon::{
    fire_profile_from_descriptor, PlayerProjectileDelivery, DEFAULT_PRIMARY_PROFILE,
};
use v2k_game::projectile_emitter::{
    projectile_class_row, FIRST_WORLD_SHOOTER_PARTICLE_CLASS, PROJECTILE_CLASS_COUNT,
    PROJECTILE_CLASS_STRIDE, PROJECTILE_CLASS_TABLE_VA, PROJECTILE_METHOD_30,
    PROJECTILE_METHOD_30_TABLE_VA,
};
use v2k_game::targetter::{fun_0044ea60, TargetterTrajectory};
use v2k_game::weapon_inventory::{
    Ammunition, WeaponDescriptor, DEFAULT_WEAPON_DESCRIPTOR_RAW, FACTORY_LEVEL_ONE_WEAPON_SELECTOR,
    WEAPON_MASTER_TABLE_RAW,
};
use v2k_game::world_fx::{
    particle_uses_fun_0043f780_entity_hit, particle_uses_fun_0043f7c0_entity_hit,
    particle_uses_primary_collision_dispatch, PRIMARY_BULLET_PARTICLE_CLASS,
    RAPID_PRIMARY_PARTICLE_CLASS, UPGRADED_PRIMARY_PARTICLE_CLASS,
};

#[test]
fn dat_004d02c0_rows_match_recovered_primary_and_method_30() {
    assert_eq!(
        PROJECTILE_CLASS_TABLE_VA + (30 * PROJECTILE_CLASS_STRIDE) as u32,
        PROJECTILE_METHOD_30_TABLE_VA
    );
    assert_eq!(PROJECTILE_CLASS_COUNT, 32);

    let row1 = projectile_class_row(1).expect("selector 1");
    assert_eq!(row1.speed_raw, 4_000);
    assert_eq!(row1.particle_class, 1);

    let row2 = projectile_class_row(2).expect("selector 2");
    assert_eq!(row2.speed_raw, 2_000);
    assert_eq!(row2.particle_class, 3);

    let row15 = projectile_class_row(15).expect("auxiliary class 15");
    assert_eq!(row15.speed_raw, 1);
    assert_eq!(row15.particle_class, 32);

    let factory = projectile_class_row(0x12).expect("factory selector 18");
    assert_eq!(factory.leading_raw, 0);
    assert_eq!(factory.speed_raw, 4_000);
    assert_eq!(factory.particle_class, RAPID_PRIMARY_PARTICLE_CLASS);
    assert_eq!(factory.trailing_raw, 0);

    let row30 = projectile_class_row(30).expect("method 30");
    assert_eq!(row30.speed_raw, PROJECTILE_METHOD_30.speed_raw);
    assert_eq!(row30.particle_class, PROJECTILE_METHOD_30.particle_class);
    assert_eq!(row30.leading_raw, PROJECTILE_METHOD_30.leading_raw);
    assert_eq!(row30.trailing_raw, PROJECTILE_METHOD_30.trailing_raw);

    assert!(projectile_class_row(32).is_none());
}

#[test]
fn fun_0044ea60_uses_the_weapon_selector_switch() {
    assert_eq!(fun_0044ea60(1), TargetterTrajectory::Ballistic);
    assert_eq!(fun_0044ea60(2), TargetterTrajectory::Ballistic);
    assert_eq!(fun_0044ea60(0x12), TargetterTrajectory::Ballistic);
    assert_eq!(fun_0044ea60(5), TargetterTrajectory::Straight);
    assert_eq!(fun_0044ea60(0x0c), TargetterTrajectory::Straight);
    assert_eq!(fun_0044ea60(0x1f), TargetterTrajectory::Straight);
}

#[test]
fn fire_profile_from_descriptor_keeps_selectors_1_and_2() {
    let selector1 = WeaponDescriptor::from_raw(DEFAULT_WEAPON_DESCRIPTOR_RAW);
    assert_eq!(
        fire_profile_from_descriptor(&selector1),
        Some(DEFAULT_PRIMARY_PROFILE)
    );
    let selector2 = WeaponDescriptor::from_raw(WEAPON_MASTER_TABLE_RAW[1]);
    assert_eq!(
        fire_profile_from_descriptor(&selector2).map(|profile| profile.selector),
        Some(2)
    );
}

#[test]
fn fire_profile_from_descriptor_admits_plasma_and_factory_fun_0043f590_rows() {
    let plasma_blue = WeaponDescriptor::from_raw(WEAPON_MASTER_TABLE_RAW[3]);
    assert_eq!(plasma_blue.selector(), 0x0c);
    let profile = fire_profile_from_descriptor(&plasma_blue).expect("selector 0xC uses class 49");
    assert_eq!(profile.selector, 0x0c);
    assert_eq!(profile.projectile.projectile_class, 49);
    assert_eq!(profile.projectile.speed_raw, 6_000);
    assert_eq!(profile.repeat_interval, Duration::from_micros(100_000));
    assert_eq!(profile.sound_id, 0x4c);
    assert!(!profile.emits_auxiliary);
    assert_eq!(profile.targetter_trajectory, TargetterTrajectory::Straight);

    let factory = WeaponDescriptor::from_raw(
        WEAPON_MASTER_TABLE_RAW
            .iter()
            .copied()
            .find(|raw| raw[0] == FACTORY_LEVEL_ONE_WEAPON_SELECTOR)
            .expect("selector 0x12 master row"),
    );
    let profile = fire_profile_from_descriptor(&factory).expect("selector 18 uses class 2");
    assert_eq!(profile.selector, FACTORY_LEVEL_ONE_WEAPON_SELECTOR);
    assert_eq!(
        profile.projectile.projectile_class,
        RAPID_PRIMARY_PARTICLE_CLASS
    );
    assert_eq!(profile.projectile.speed_raw, 4_000);
    assert_eq!(profile.projectile.sprite_id, 705);
    assert_eq!(profile.projectile.lifetime_ticks, 200);
    assert_eq!(profile.repeat_interval, Duration::from_micros(40_000));
    assert_eq!(profile.sound_id, 88);
    assert!(profile.emits_auxiliary);
    assert_eq!(profile.targetter_trajectory, TargetterTrajectory::Ballistic);
}

fn master_descriptor_for(selector: u8) -> WeaponDescriptor {
    WeaponDescriptor::from_raw(
        WEAPON_MASTER_TABLE_RAW
            .iter()
            .copied()
            .find(|raw| raw[0] == selector)
            .unwrap_or_else(|| panic!("selector {selector:#x} master row")),
    )
}

#[test]
fn fire_profile_from_descriptor_admits_fun_0043f6e0_rows() {
    let selector_6 = master_descriptor_for(0x06);
    let profile = fire_profile_from_descriptor(&selector_6).expect("selector 6 uses class 4");
    assert_eq!(profile.selector, 6);
    assert_eq!(profile.projectile.projectile_class, 4);
    assert_eq!(profile.projectile.speed_raw, 2_000);
    assert_eq!(profile.repeat_interval, Duration::from_micros(400_000));
    assert_eq!(profile.sound_id, 0x58);
    assert!(!profile.emits_auxiliary);
    assert_eq!(profile.targetter_trajectory, TargetterTrajectory::Ballistic);

    let virus_bomb = master_descriptor_for(0x1c);
    let profile = fire_profile_from_descriptor(&virus_bomb).expect("selector 0x1C uses class 77");
    assert_eq!(profile.projectile.projectile_class, 77);
    assert_eq!(profile.targetter_trajectory, TargetterTrajectory::Ballistic);

    let antidote_bomb = master_descriptor_for(0x1d);
    let profile =
        fire_profile_from_descriptor(&antidote_bomb).expect("selector 0x1D uses class 78");
    assert_eq!(profile.projectile.projectile_class, 78);
    // Existing F6E0 emission does not admit selector29's distinct 41850 tail
    // to the 11320 cure transaction implemented for selector7/class6.
    assert!(!particle_uses_fun_0043f7c0_entity_hit(78));
    assert_eq!(particle_descriptor(78).unwrap().raw_u32(0x2c), 0x0044_1850);

    let selector_0x13 = master_descriptor_for(0x13);
    let profile =
        fire_profile_from_descriptor(&selector_0x13).expect("selector 0x13 uses class 54");
    assert_eq!(profile.projectile.projectile_class, 54);
}

#[test]
fn fire_profile_from_descriptor_admits_fun_0043f780_selector_8() {
    let virus = master_descriptor_for(0x08);
    let profile = fire_profile_from_descriptor(&virus).expect("selector 8 uses class 5");
    assert_eq!(profile.selector, 8);
    assert_eq!(profile.projectile.projectile_class, 5);
    assert_eq!(profile.projectile.speed_raw, 2_000);
    assert_eq!(profile.repeat_interval, Duration::from_micros(80_000));
    assert_eq!(profile.sound_id, 0x40);
    assert!(!profile.emits_auxiliary);
    assert_eq!(profile.targetter_trajectory, TargetterTrajectory::Ballistic);
    assert!(particle_uses_fun_0043f780_entity_hit(5));
    assert!(!particle_uses_fun_0043f780_entity_hit(6));
    assert_eq!(
        particle_descriptor(5).expect("class 5").raw_u32(0x24),
        0x0043_f920
    );
}

#[test]
fn fire_profile_from_descriptor_admits_fun_0043f7c0_selector_7() {
    let antidote = master_descriptor_for(7);
    let profile = fire_profile_from_descriptor(&antidote).expect("selector7 uses Antidote class6");
    assert_eq!(profile.selector, 7);
    assert_eq!(profile.delivery, PlayerProjectileDelivery::Particle);
    assert_eq!(profile.projectile.projectile_class, 6);
    assert_eq!(profile.projectile.sprite_id, 917);
    assert_eq!(profile.projectile.speed_raw, 2_000);
    assert_eq!(profile.projectile.lifetime_ticks, 200);
    assert_eq!(profile.repeat_interval, Duration::from_micros(80_000));
    assert_eq!(profile.sound_id, 64);
    assert!(!profile.emits_auxiliary);
    assert_eq!(profile.targetter_trajectory, TargetterTrajectory::Ballistic);
    assert!(matches!(antidote.ammunition(), Ammunition::Finite(_)));

    // Retained DAT_004D0330 selects class6 descriptor 004CC270. Its distinct
    // entity/static/surface callbacks select 11320 and clear 33720(x,z,0).
    let row = projectile_class_row(7).unwrap();
    assert_eq!(row.speed_raw, 2_000);
    assert_eq!(row.particle_class, 6);
    let particle = particle_descriptor(6).unwrap();
    assert_eq!(particle.raw_u32(0x18), 0x0043_e1a0);
    assert_eq!(particle.raw_u32(0x1c), 0x0043_f7c0);
    assert_eq!(particle.raw_u32(0x20), 0x004c_bfe8);
    assert_eq!(particle.raw_u32(0x24), 0x0043_f950);
    assert!(particle_uses_fun_0043f7c0_entity_hit(6));
}

#[test]
fn default_and_factory_bullets_share_the_packet_and_callbacks_with_distinct_draw_scale() {
    let class1 = particle_descriptor(PRIMARY_BULLET_PARTICLE_CLASS).unwrap();
    let class2 = particle_descriptor(RAPID_PRIMARY_PARTICLE_CLASS).unwrap();
    let class3 = particle_descriptor(UPGRADED_PRIMARY_PARTICLE_CLASS).unwrap();
    let class87 = particle_descriptor(FIRST_WORLD_SHOOTER_PARTICLE_CLASS).unwrap();
    assert_eq!(class1.raw_u32(0x1c), 0x0043_f590);
    assert_eq!(class1.raw_u32(0x20), 0x004c_bf70);
    assert_eq!(class1.raw_u32(0x24), 0x0043_f800);
    // DAT_004CC16C and DAT_004CC1A0 differ only in draw-scale word +0x0A.
    for offset in 0..v2k_game::particle_descriptors::PARTICLE_DESCRIPTOR_STRIDE {
        if !(0x0a..0x0c).contains(&offset) {
            assert_eq!(
                class1.raw_byte(offset),
                class2.raw_byte(offset),
                "offset {offset:#x}"
            );
        }
    }
    assert_eq!(class1.draw_scale_raw(), 0x300);
    assert_eq!(class2.draw_scale_raw(), 0x400);
    assert_eq!(class3.raw_u32(0x1c), 0x0043_f590);
    assert_eq!(class3.raw_u32(0x20), 0x004c_bf88);
    assert_eq!(class3.raw_u32(0x24), 0x0043_f800);
    assert_eq!(class87.raw_u32(0x1c), 0x0043_f590);
    assert_eq!(class87.raw_u32(0x20), 0x004c_c0f0);
    assert_eq!(class87.raw_u32(0x24), 0x0042_e8e0);
    assert!(particle_uses_primary_collision_dispatch(
        PRIMARY_BULLET_PARTICLE_CLASS
    ));
    assert!(particle_uses_primary_collision_dispatch(
        RAPID_PRIMARY_PARTICLE_CLASS
    ));
    assert!(particle_uses_primary_collision_dispatch(
        UPGRADED_PRIMARY_PARTICLE_CLASS
    ));
    assert!(!particle_uses_primary_collision_dispatch(
        FIRST_WORLD_SHOOTER_PARTICLE_CLASS
    ));
}

#[test]
fn idle_unfireable_selector_keeps_ab_phase() {
    use v2k_game::primary_weapon::{
        PrimaryFireGeometry, PrimaryGunChannel, PrimaryGunMounts, PrimaryLaunchBasis,
        PrimaryShotBudget, PrimaryTriggerInput, PrimaryWeapon, DEFAULT_PRIMARY_PROFILE,
    };

    let geometry = PrimaryFireGeometry {
        launch_basis: PrimaryLaunchBasis {
            origin_world: [0.0; 3],
            direction_unit: [0.0, 0.0, 1.0],
        },
        gun_mounts: PrimaryGunMounts::Unresolved,
        shooter_origin_world: [0.0; 3],
        shooter_id: 46,
        shooter_entity_type_at_birth: Some(46),
        shooter_velocity_world: [0.0; 3],
        sound_origin_world: [0.0; 3],
    };
    let mut weapon = PrimaryWeapon::new();
    weapon.update(
        Duration::ZERO,
        PrimaryTriggerInput {
            source_a: true,
            source_b: false,
        },
        geometry,
    );
    assert_eq!(weapon.next_gun(), PrimaryGunChannel::B);
    weapon.idle(Duration::from_millis(200), 5, false);
    assert_eq!(weapon.next_gun(), PrimaryGunChannel::B);
    let resumed = weapon.update_with_profile(
        Duration::ZERO,
        PrimaryTriggerInput {
            source_a: true,
            source_b: false,
        },
        geometry,
        DEFAULT_PRIMARY_PROFILE,
        PrimaryShotBudget::Unlimited,
    );
    assert_eq!(resumed[0].gun_channel, PrimaryGunChannel::B);
}
