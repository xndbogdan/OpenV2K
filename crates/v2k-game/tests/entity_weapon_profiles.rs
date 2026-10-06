use std::time::Duration;
use v2k_formats::anim_sound::SoundPool;
use v2k_game::{
    primary_weapon::{
        fire_profile_from_descriptor, PlayerProjectileDelivery, PrimaryFireGeometry,
        PrimaryGunMounts, PrimaryLaunchBasis, PrimaryShotBudget, PrimaryTriggerInput,
        PrimaryWeapon,
    },
    session::GameSession,
    weapon_inventory::{Ammunition, PowerUpPayload, WeaponInventory},
    world_fx::WorldFx,
};

fn session() -> GameSession {
    let data = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data).expect("retail corpus required");
    session.load_auxiliary_ovl(2, 1).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(17, 1).unwrap();
    session
}

fn events(
    selector: u8,
    origin: [f32; 3],
    rounds: u32,
) -> Vec<v2k_game::primary_weapon::PrimaryFireEvent> {
    let mut inventory = WeaponInventory::new();
    inventory.acquire_weapon(PowerUpPayload {
        selector,
        amount: rounds as i32,
    });
    let profile = fire_profile_from_descriptor(inventory.selected_descriptor()).unwrap();
    PrimaryWeapon::new().update_with_profile(
        Duration::ZERO,
        PrimaryTriggerInput {
            source_a: true,
            source_b: false,
        },
        PrimaryFireGeometry {
            launch_basis: PrimaryLaunchBasis {
                origin_world: origin,
                direction_unit: [0.0, 0.0, 1.0],
            },
            gun_mounts: PrimaryGunMounts::Unresolved,
            shooter_origin_world: origin,
            shooter_id: 7,
            shooter_entity_type_at_birth: Some(46),
            shooter_velocity_world: [0.0; 3],
            sound_origin_world: origin,
        },
        profile,
        PrimaryShotBudget::Limited(rounds),
    )
}

#[v2k_test_support::retail_test]
fn native_profiles_keep_models_cadence_ammo_and_pcm_aliases() {
    let session = session();
    let tables = session.cache.global_sound_tables();
    let sounds = SoundPool::from_tables(&tables);
    for (selector, entity_type, model, cue, pcm, multiplier, variance) in [
        (3, 59, 128, 61, 7, 32768, 6553),
        (4, 59, 128, 88, 7, 104857, 0),
        (5, 42, 240, 79, 19, 65536, 6553),
    ] {
        let mut inventory = WeaponInventory::new();
        inventory.acquire_weapon(PowerUpPayload {
            selector,
            amount: 2,
        });
        let descriptor = inventory.selected_descriptor();
        let profile = fire_profile_from_descriptor(descriptor).unwrap();
        assert_eq!(
            profile.delivery,
            PlayerProjectileDelivery::NativeEntityWeapon { entity_type }
        );
        assert_eq!(profile.repeat_interval, Duration::from_secs(1));
        assert_eq!(descriptor.ammunition(), Ammunition::Finite(2));
        assert!(profile.emits_auxiliary);
        assert_eq!(profile.sound_id, cue);
        let sound = sounds.resolve(cue).unwrap();
        assert_eq!(sound.pcm_global_id, pcm);
        assert_eq!(sound.alias_hops[0].frequency_multiplier_16_16, multiplier);
        assert_eq!(sound.alias_hops[0].frequency_variance_16_16, variance);
        let record = session
            .cache
            .global_entity_type(usize::from(entity_type))
            .unwrap();
        assert_eq!(record.model_ids, [model; 4]);
        assert!(session.cache.global_model(usize::from(model)).is_some());
        let raw = &record.raw_header;
        assert_eq!(i16::from_le_bytes(raw[0x50..0x52].try_into().unwrap()), 512);
        assert_eq!(
            i16::from_le_bytes(raw[0x52..0x54].try_into().unwrap()),
            if entity_type == 42 { 1024 } else { 2048 }
        );
        assert_eq!(
            i32::from_le_bytes(raw[0x60..0x64].try_into().unwrap()),
            14000
        );
        assert_eq!(
            i32::from_le_bytes(raw[0x64..0x68].try_into().unwrap()),
            12000
        );
    }
}

#[test]
fn body_feedback_keeps_alias_and_only_auxiliary_particle() {
    let mut fx = WorldFx::new();
    let events = events(5, [0.0, 60.0, 0.0], 1);
    fx.queue_primary_fire_batch(&events, None);
    fx.process_pending();
    assert_eq!(fx.particle_count(), 1);
    let sounds = fx.take_positional_sounds();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].sound_id, 79);
}
