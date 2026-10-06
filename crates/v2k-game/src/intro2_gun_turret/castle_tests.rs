//! Castle's authored AA turrets and flowers use real allocation receipts.

use super::*;
use crate::{
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    specialized_actor_task_production::SpecializedActorTaskScheduler,
};

#[v2k_test_support::retail_test]
fn every_authored_type104_and_flower_row_matches_the_native_constructor_contract() {
    let (mut session, _, _) = authored_tests::fixture(15);
    let mut births = [0usize; 2];
    for level in 13..=50 {
        session.load_level_by_id(level, 1).unwrap();
        for (index, type_id) in [104u32, 115].into_iter().enumerate() {
            let profile = Intro2GunTurretProfile::for_native_authored(type_id).unwrap();
            let metadata = EntityTypeRuntimeMetadata::from_section12(
                session.cache.global_entity_type(type_id as usize).unwrap(),
            );
            native::authenticate_metadata(profile, &metadata)
                .unwrap_or_else(|error| panic!("world{level} type{type_id}: {error:?}"));
            for spawn in &session.cache.level_desc().unwrap().entities {
                if u32::from(spawn.entity_type) != type_id {
                    continue;
                }
                assert_eq!(
                    spawn.model_overrides, [0; 4],
                    "world{level} spawn{}",
                    spawn.index
                );
                assert!(
                    !spawn.has_animation && spawn.animation.is_none(),
                    "world{level} spawn{}",
                    spawn.index
                );
                assert!(
                    !spawn.has_config && spawn.config.is_none(),
                    "world{level} spawn{}",
                    spawn.index
                );
                births[index] += 1;
            }
        }
    }
    assert!(births.into_iter().all(|count| count > 0));
}

#[v2k_test_support::retail_test]
fn castle_type104_and_flowers_construct_their_distinct_native_profiles() {
    let (session, manager, _) = authored_tests::fixture(15);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_gun_turret(&manager), 5);
    for (type_id, profile, expected_spawns) in [
        (104, Intro2GunTurretProfile::Type104, vec![1, 49, 50]),
        (115, Intro2GunTurretProfile::Type115, vec![51, 52]),
    ] {
        let metadata = EntityTypeRuntimeMetadata::from_section12(
            session.cache.global_entity_type(type_id as usize).unwrap(),
        );
        native::authenticate_metadata(profile, &metadata).unwrap();
        let entities: Vec<_> = manager
            .iter_all()
            .filter(|e| e.entity_type == type_id)
            .collect();
        assert_eq!(
            entities
                .iter()
                .map(|e| e.authored_spawn_index.unwrap())
                .collect::<Vec<_>>(),
            expected_spawns
        );
        for entity in entities {
            let runtime = entity.intro2_gun_turret_runtime.unwrap();
            assert_eq!(runtime.profile, profile);
            assert!(matches!(
                runtime.origin,
                GunTurretConstructionOrigin::NativeOrdinary(_)
            ));
            assert!(intro2_gun_turret_manager_allocation_authenticates(
                &manager, entity.id
            ));
            assert!(scheduler.intro2_gun_turret_completed_owner(&manager, entity.id));
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(profile.health())
            );
            assert_eq!(entity.capability_flags, profile.capability());
            assert_eq!(
                entity.model_slots,
                profile.model_slots().map(|m| Some(usize::from(m)))
            );
            assert_eq!(
                runtime.sub_e_runtime.projectile_method,
                u32::from(profile.emitter().projectile_method)
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn castle_type104_profile_rejects_different_emitter_and_foreign_receipt() {
    let (session, mut manager, _) = authored_tests::fixture(15);
    let (_, mut foreign, _) = authored_tests::fixture(15);
    let mut metadata =
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(104).unwrap());
    metadata.projectile_emitter_descriptor =
        RetailRuntimeValue::Known(Some(Intro2GunTurretProfile::Type97.emitter()));
    assert_eq!(
        native::authenticate_metadata(Intro2GunTurretProfile::Type104, &metadata),
        Err(Intro2GunTurretError::Metadata)
    );
    let id = manager
        .iter_all()
        .find(|e| e.entity_type == 104)
        .unwrap()
        .id;
    let other_id = foreign
        .iter_all()
        .find(|e| e.entity_type == 104)
        .unwrap()
        .id;
    std::mem::swap(
        manager.entity_mut(id).unwrap(),
        foreign.entity_mut(other_id).unwrap(),
    );
    assert!(!intro2_gun_turret_manager_allocation_authenticates(
        &manager, id
    ));
    assert!(Intro2GunTurretOwner::adopt(&manager, id).is_err());
}
