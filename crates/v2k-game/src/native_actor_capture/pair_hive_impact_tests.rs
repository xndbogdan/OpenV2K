//! Authored boulders retain their boundary before a Hive latch can escape.

use super::*;
use crate::{
    gameplay_notifications::GameplayNotifications,
    native_type122::construction_tests::native_fixture,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
};

#[v2k_test_support::retail_test]
fn hive_pair_rejects_actual_type3_and_type27_before_an_unowned_suffix() {
    for (world, kind) in [(31, 3), (27, 27)] {
        let (mut session, mut entities, mut fx) = native_fixture(world);
        let hive = entities
            .iter_all()
            .find(|entity| entity.entity_type == 67)
            .unwrap()
            .id;
        let boulder = entities
            .iter_all()
            .find(|entity| entity.entity_type == kind)
            .unwrap()
            .id;
        let metadata = entities.type_runtime_metadata(kind).unwrap();
        assert_eq!(metadata.capability_flags, 0x2000);
        assert_eq!(
            metadata.initializer.as_ref().unwrap().behavior_choices[0].behavior_class_id,
            20
        );
        assert_eq!(
            hive_impact_counterpart(
                actor(&entities, hive).unwrap(),
                actor(&entities, boulder).unwrap()
            ),
            Some(boulder)
        );
        assert_eq!(
            hive_impact_counterpart(
                actor(&entities, boulder).unwrap(),
                actor(&entities, hive).unwrap()
            ),
            Some(boulder)
        );
        let before = actor(&entities, hive).unwrap().sub_n_runtime;
        let mut tasks = SpecializedActorTaskScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut static_damage = StaticDamageScheduler::new();
        let mut committed = false;
        let result = behavior(
            &mut Intro2ContactFrame {
                entities: &mut entities,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                notifications: &mut notifications,
                retail_tick: 5000,
                actor_tasks: &mut tasks,
            },
            hive,
            boulder,
            CaptureFeedbackPolicy::Gameplay,
            None,
            &mut committed,
        );
        assert_eq!(
            result,
            Err(NativeCaptorPairBlock::UnsupportedHiveImpactCounterpart {
                entity_id: boulder,
                entity_type: kind
            })
        );
        assert!(!committed);
        assert_eq!(actor(&entities, hive).unwrap().sub_n_runtime, before);
    }
}

#[v2k_test_support::retail_test]
fn hive_consumption_keeps_its_explicit_owner_boundary() {
    let (mut session, mut entities, mut fx) = native_fixture(13);
    let hive = entities
        .iter_all()
        .find(|entity| entity.entity_type == 67)
        .unwrap()
        .id;
    let villager = entities
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .unwrap()
        .id;
    assert_ne!(
        actor(&entities, villager).unwrap().capability_flags & 0x0c00,
        0
    );
    let before = actor(&entities, hive).unwrap().sub_n_runtime;
    let style = style_address(actor(&entities, hive).unwrap()).unwrap();
    let mut tasks = SpecializedActorTaskScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut static_damage = StaticDamageScheduler::new();
    let mut committed = false;
    let result = behavior(
        &mut Intro2ContactFrame {
            entities: &mut entities,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: 5000,
            actor_tasks: &mut tasks,
        },
        hive,
        villager,
        CaptureFeedbackPolicy::Gameplay,
        None,
        &mut committed,
    );
    assert_eq!(
        result,
        Err(NativeCaptorPairBlock::UnsupportedBehavior {
            entity_id: hive,
            style
        })
    );
    assert!(!committed);
    assert_eq!(actor(&entities, hive).unwrap().sub_n_runtime, before);
}

#[v2k_test_support::retail_test]
fn existing_native_gunner_hive_callback_stays_null() {
    let (mut session, mut entities, mut fx) = native_fixture(13);
    let hive = entities
        .iter_all()
        .find(|entity| entity.entity_type == 67)
        .unwrap()
        .id;
    let gunner = entities
        .iter_all()
        .find(|entity| entity.entity_type == 47)
        .unwrap()
        .id;
    assert_eq!(actor(&entities, gunner).unwrap().capability_flags, 8);
    let before = actor(&entities, hive).unwrap().sub_n_runtime;
    let mut tasks = SpecializedActorTaskScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut static_damage = StaticDamageScheduler::new();
    let mut committed = false;
    let result = behavior(
        &mut Intro2ContactFrame {
            entities: &mut entities,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: 5000,
            actor_tasks: &mut tasks,
        },
        hive,
        gunner,
        CaptureFeedbackPolicy::Gameplay,
        None,
        &mut committed,
    );
    assert_eq!(result, Ok(NativeCaptorPairBehaviorResult::Null));
    assert!(!committed);
    assert_eq!(actor(&entities, hive).unwrap().sub_n_runtime, before);
}
