use super::*;
use crate::particle_descriptors::PARTICLE_DESCRIPTOR_STRIDE;
use crate::primary_weapon::fire_profile_from_descriptor;
use crate::weapon_inventory::{PowerUpPayload, WeaponInventory};

fn rapid_event(origin_y: f32) -> PrimaryFireEvent {
    let mut inventory = WeaponInventory::new();
    inventory.acquire_weapon(PowerUpPayload::from_runtime_word(0x0001_f412));
    let profile = fire_profile_from_descriptor(inventory.selected_descriptor()).unwrap();
    PrimaryWeapon::new().update_with_profile(
        Duration::ZERO,
        PrimaryTriggerInput {
            source_a: true,
            source_b: false,
        },
        primary_geometry(origin_y, [0.0; 3]),
        profile,
        PrimaryShotBudget::Unlimited,
    )[0]
}

#[test]
fn factory_bullet_descriptor_differs_only_in_draw_scale() {
    let default = particle_descriptor(PRIMARY_BULLET_PARTICLE_CLASS).unwrap();
    let rapid = particle_descriptor(RAPID_PRIMARY_PARTICLE_CLASS).unwrap();
    assert_eq!(default.draw_scale_raw(), 0x300);
    assert_eq!(rapid.draw_scale_raw(), 0x400);
    for offset in 0..PARTICLE_DESCRIPTOR_STRIDE {
        if !matches!(offset, 0x0a | 0x0b) {
            assert_eq!(
                default.raw_byte(offset),
                rapid.raw_byte(offset),
                "offset {offset:#x}"
            );
        }
    }
    assert!(particle_uses_primary_collision_dispatch(
        RAPID_PRIMARY_PARTICLE_CLASS
    ));
}

#[test]
fn rapid_primary_catch_up_emits_every_shot_but_one_callback_sound() {
    let mut inventory = WeaponInventory::new();
    inventory.acquire_weapon(PowerUpPayload::from_runtime_word(0x0001_f412));
    let profile = fire_profile_from_descriptor(inventory.selected_descriptor()).unwrap();
    let mut weapon = PrimaryWeapon::new();
    let trigger = PrimaryTriggerInput {
        source_a: true,
        source_b: false,
    };
    let geometry = primary_geometry(2.0, [0.0; 3]);
    weapon.update_with_profile(
        Duration::ZERO,
        trigger,
        geometry,
        profile,
        PrimaryShotBudget::Unlimited,
    );
    let shots = weapon.update_with_profile(
        Duration::from_millis(120),
        trigger,
        geometry,
        profile,
        PrimaryShotBudget::Unlimited,
    );
    assert_eq!(shots.len(), 3);
    let mut fx = WorldFx::new();
    fx.queue_primary_fire_batch(&shots, None);
    fx.process_pending();
    let particles = fx.test_particles_in_virgin_birth_order();
    let bullets: Vec<_> = particles
        .iter()
        .filter(|p| p.source_class == RAPID_PRIMARY_PARTICLE_CLASS)
        .collect();
    assert_eq!(bullets.len(), 3);
    assert!(bullets
        .iter()
        .all(|p| p.draw_scale_raw == 0x400 && p.current_sprite_id() == 705));
    assert_eq!(
        particles
            .iter()
            .filter(|p| p.source_class == PRIMARY_MUZZLE_PARTICLE_CLASS)
            .count(),
        3
    );
    let sounds = fx.take_positional_sounds();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].sound_id, 88);
    fx.queue_primary_fire_batch(&[], None);
    fx.process_pending();
    assert!(fx.take_positional_sounds().is_empty());
}

#[test]
fn factory_bullet_keeps_default_air_water_and_slow_free_semantics() {
    for water in [None, Some(-10.0), Some(3.0)] {
        let mut pair = [WorldFx::new(), WorldFx::new()];
        for (fx, event) in pair
            .iter_mut()
            .zip([primary_event(2.0, [0.0; 3]), rapid_event(2.0)])
        {
            fx.queue_primary_fire_batch(&[event], water);
            fx.process_pending();
            fx.update(match water {
                Some(y) => ParticleUpdateRequest::flat_water(20_000, 1, y),
                None => ParticleUpdateRequest::dry(20_000, 1),
            });
        }
        let default = pair[0].test_particles_in_virgin_birth_order()[0];
        let mut rapid = pair[1].test_particles_in_virgin_birth_order()[0];
        assert_eq!(rapid.source_class, RAPID_PRIMARY_PARTICLE_CLASS);
        assert_eq!(rapid.draw_scale_raw, 0x400);
        rapid.source_class = default.source_class;
        rapid.draw_scale_raw = default.draw_scale_raw;
        assert_eq!(rapid, default);
    }
    let mut fx = WorldFx::new();
    fx.queue_primary_fire_batch(&[rapid_event(2.0)], None);
    fx.process_pending();
    fx.particles[0].velocity = [0.0; 3];
    fx.update(ParticleUpdateRequest::dry(20_000, 1));
    assert!(!fx
        .particles
        .slots
        .iter()
        .flatten()
        .any(|p| p.source_class == RAPID_PRIMARY_PARTICLE_CLASS));
}

#[test]
fn factory_bullet_applies_primary_entity_damage_and_is_consumed() {
    let mut fx = WorldFx::new();
    fx.queue_primary_fire_batch(&[rapid_event(2.0)], None);
    fx.process_pending();
    let pool = solid_sphere_pool(100);
    let entities = [collision_entity(9, [4.0, 2.0, 5.0], 80)];
    let outcome = fx.update(ParticleUpdateRequest::dry(0, 0).with_callbacks(
        ParticleCallbackContext {
            owner_motions: &[],
            collision: Some(ParticleCollisionContext {
                entities: &entities,
                model_pool: &pool,
            }),
        },
    ));
    assert_eq!(outcome.entity_impacts.len(), 1);
    let impact = outcome.entity_impacts[0];
    assert_eq!(impact.source_particle_class, RAPID_PRIMARY_PARTICLE_CLASS);
    assert_eq!(impact.target_entity_id, 9);
    assert_eq!(
        impact.damage,
        Some(BallisticDamageRequest {
            packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
            source_entity_type_at_birth: Some(46),
            source_owner_id: Some(7),
        })
    );
    assert!(!fx
        .particles
        .slots
        .iter()
        .flatten()
        .any(|p| p.source_class == RAPID_PRIMARY_PARTICLE_CLASS));
}

#[test]
fn factory_bullet_surface_callback_survives_only_water_entry_selector_six() {
    for selector in 0..=15 {
        let mut particle = descriptor_test_particle(
            RAPID_PRIMARY_PARTICLE_CLASS,
            [4.0, 2.0, 5.0],
            [1000, -40, 0],
            Some(7),
        );
        let mut effects = Vec::new();
        let keep = apply_primary_surface_callback(
            &mut particle,
            selector,
            0.0,
            [1.0; 3],
            [2.0; 3],
            &mut effects,
        );
        assert_eq!(keep, selector == 6);
        assert!(matches!(effects.as_slice(), [PrimaryImpactEffect::Surface {
            sound_position, effect_position, response_selector,
        }] if *sound_position == [1.0; 3] && *effect_position == [2.0; 3]
            && *response_selector == selector));
    }
}
