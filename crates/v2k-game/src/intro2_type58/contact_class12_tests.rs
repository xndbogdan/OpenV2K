//! The real native Class12 graph has null A8B0/D920 callbacks, then11760.

use super::*;
use crate::{
    intro2_radial::Intro2RadialTaskCustody,
    native_actor_surface_contact::{type58_tests::Fixture, NativeActorSurfaceContactOutcome},
    specialized_actor_task_production::SpecializedActorTaskFamily,
    static_damage::static_damage_kind_is_admitted,
};

/// Search authored static objects with the retained model273/basis, without
/// changing the terrain or manufacturing a contact plane.
fn place_overlap(f: &mut Fixture) -> StaticModelContact {
    let entity = f.entity();
    let terrain = f.session.cache.terrain().unwrap();
    let objects = f.session.cache.terrain_objects().unwrap();
    let model = f.session.cache.global_model(MODEL).unwrap();
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        panic!("native Class12 basis")
    };
    for x in 0..256 {
        for z in 0..256 {
            let cell = terrain.cell(x, z).unwrap();
            if cell.attribute == 0 {
                continue;
            }
            let descriptor = &objects.records[usize::from(cell.attribute)];
            if !static_damage_kind_is_admitted(descriptor.kind_index) {
                continue;
            }
            let x_raw = ((x as u16) << 8 | 0x80) as i16;
            let z_raw = ((z as u16) << 8 | 0x80) as i16;
            let floor = terrain.bilinear_height_raw(x_raw, z_raw);
            for dy in [0i16, 64, 128, 256, -64] {
                for (dx, dz) in [(0i16, 0i16), (-128, 0), (128, 0), (0, -128), (0, 128)] {
                    let position = [
                        x_raw.wrapping_add(dx),
                        floor.wrapping_add(dy),
                        z_raw.wrapping_add(dz),
                    ];
                    let contact = scan_deepest_static_contact(StaticContactQuery {
                        terrain,
                        terrain_objects: objects,
                        model_pool: &f.session.cache,
                        tick: 0,
                        active_model: model,
                        active_model_to_world_basis: basis
                            .orientation_world_from_model()
                            .map(|row| row.map(f64::from)),
                        active_anim_vars: &entity.presentation_anim_vars(0),
                        position_raw: position,
                    })
                    .unwrap();
                    if let Some(contact) =
                        contact.filter(|hit| static_damage_kind_is_admitted(hit.kind_index))
                    {
                        f.entities
                            .entity_mut(f.id)
                            .unwrap()
                            .set_motion_raw(position, [0; 3]);
                        return contact;
                    }
                }
            }
        }
    }
    panic!("actual authored static geometry must overlap retained model273");
}

fn run(f: &mut Fixture) -> Intro2Type58ContactOutcome {
    let id = f.id;
    resolve_intro2_type58_static_contact(&mut f.frame(0), id)
}

#[v2k_test_support::retail_test]
fn native_type58_class12_static_null_hooks_preserve_clock_and_rng_in_every_native_world() {
    for level in [14, 24, 31, 50] {
        let mut f = Fixture::new(level);
        f.die();
        let contact = place_overlap(&mut f);
        let before = f.snapshot();
        let mut expected_position = before.position;
        let mut expected_velocity = before.velocity;
        apply_contact_response_raw(&mut expected_position, &mut expected_velocity, contact);
        let mut rng = f.fx.fork_for_main_base_abort_transaction();
        let result = run(&mut f);
        let Intro2Type58ContactOutcome::Applied(applied) = result else {
            panic!("world{level}: {result:?}")
        };
        assert_eq!(applied.contact, contact);
        assert_eq!(applied.furniture_damage, None);
        assert_eq!(applied.collision_impact_raw, 0);
        assert_eq!(applied.collision_static_damage, None);
        assert_eq!(applied.actor_damage, None);
        let after = f.snapshot();
        assert_eq!(
            (after.position, after.velocity),
            (expected_position, expected_velocity)
        );
        after.assert_graph_and_components_retained(&before);
        assert_eq!(after.collision, before.collision);
        assert_eq!(
            f.tasks.family_for(f.id),
            Some(SpecializedActorTaskFamily::Intro2CommonDying)
        );
        assert!(f.tasks.prepare_native_actor_mutation(&f.entities, f.id));
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            rng.next_shared_retail_random_u16(),
            "Class12 has no02CA0/C690 draws"
        );
        assert_eq!(f.static_damage.active_program_count(), 0);
        assert_eq!(f.fx.particle_count(), 0);
    }
}

#[v2k_test_support::retail_test]
fn native_type58_class12_static_impact_uses_existing_dying_owner_and_checked_buffer() {
    let mut f = Fixture::new(14);
    f.die();
    let contact = place_overlap(&mut f);
    let entity = f.entities.entity_mut(f.id).unwrap();
    entity.set_velocity_raw(contact.normal_q12.map(|word| word.wrapping_mul(-2)));
    entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(37);
    let before = f.snapshot();
    let result = run(&mut f);
    let Intro2Type58ContactOutcome::Applied(applied) = result else {
        panic!("{result:?}")
    };
    assert_eq!(applied.contact, contact);
    assert!(applied.furniture_damage.is_none());
    assert!(applied.collision_impact_raw > 0);
    assert!(applied.collision_static_damage.is_some());
    let damage = applied
        .actor_damage
        .expect("11760 checked damage reaches existing dying state");
    assert!(damage.filtered_damage_raw > 37);
    assert_eq!(
        damage.damage_after_buffer_raw,
        damage.filtered_damage_raw - 37
    );
    assert!(damage.death_publication.is_none(), "no C850 reentry");
    let after = f.snapshot();
    assert_eq!(
        after.collision.pre_health_damage_buffer_raw,
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(after.collision.health_raw, before.collision.health_raw);
    after.assert_graph_and_components_retained(&before);
    assert_eq!(
        f.tasks.family_for(f.id),
        Some(SpecializedActorTaskFamily::Intro2CommonDying)
    );
    assert!(f.tasks.prepare_native_actor_mutation(&f.entities, f.id));
}

#[v2k_test_support::retail_test]
fn native_type58_class12_static_rejects_unknown_executing_stale_and_foreign_custody() {
    for case in 0..7 {
        let mut f = Fixture::new(14);
        f.die();
        place_overlap(&mut f);
        f.invalidate_custody(case);
        let before = f.snapshot();
        let mut rng = f.fx.fork_for_main_base_abort_transaction();
        let result = run(&mut f);
        assert!(
            matches!(
                result,
                Intro2Type58ContactOutcome::Blocked {
                    committed_prefix: false,
                    ..
                }
            ),
            "case{case}: {result:?}"
        );
        assert_eq!(f.snapshot(), before);
        assert_eq!(f.static_damage.active_program_count(), 0);
        assert_eq!(f.fx.particle_count(), 0);
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            rng.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type58_class12_late_static_damage_failure_parks_response_across_surface_entry() {
    let mut f = Fixture::new(14);
    f.die();
    let contact = place_overlap(&mut f);
    let entity = f.entities.entity_mut(f.id).unwrap();
    entity.set_velocity_raw(contact.normal_q12.map(|word| word.wrapping_mul(-2)));
    entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Unresolved;
    let before = f.snapshot();
    let mut position = before.position;
    let mut velocity = before.velocity;
    apply_contact_response_raw(&mut position, &mut velocity, contact);
    assert_ne!(velocity, before.velocity);
    let result = run(&mut f);
    assert!(
        matches!(
            result,
            Intro2Type58ContactOutcome::Blocked {
                reason: Intro2Type58ContactBlock::Damage(_),
                committed_prefix: true
            }
        ),
        "{result:?}"
    );
    let after = f.snapshot();
    assert_eq!((after.position, after.velocity), (position, velocity));
    after.assert_graph_and_components_retained(&before);
    assert_eq!(after.collision, before.collision);
    assert_eq!(
        f.tasks.family_for(f.id),
        Some(SpecializedActorTaskFamily::Intro2CommonDying)
    );
    assert!(!f.tasks.prepare_native_actor_mutation(&f.entities, f.id));
    f.tasks
        .register_intro2_common_dying(Intro2CommonDyingOwner::adopt(&f.entities, f.id).unwrap());
    assert!(!f.tasks.prepare_native_actor_mutation(&f.entities, f.id));
    let mut rng = f.fx.fork_for_main_base_abort_transaction();
    let programs = f.static_damage.active_program_count();
    assert!(matches!(
        run(&mut f),
        Intro2Type58ContactOutcome::Blocked {
            committed_prefix: false,
            ..
        }
    ));
    assert!(matches!(
        f.run(1),
        NativeActorSurfaceContactOutcome::Blocked {
            committed_prefix: false,
            ..
        }
    ));
    assert_eq!(f.snapshot(), after);
    assert_eq!(f.static_damage.active_program_count(), programs);
    assert_eq!(
        f.fx.next_shared_retail_random_u16(),
        rng.next_shared_retail_random_u16()
    );
}
