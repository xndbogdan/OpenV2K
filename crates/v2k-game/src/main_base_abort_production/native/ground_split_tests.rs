use super::*;
use crate::{
    gameplay_notifications::GameplayNotifications,
    specialized_actor_task_production::{
        SpecializedActorTaskFamily, SpecializedActorTaskScheduler,
    },
};

fn abort_visit(
    entities: &mut EntityManager,
    fx: &mut WorldFx,
    tasks: &mut SpecializedActorTaskScheduler,
    publications: &mut MainBaseAbortPublicationCounts,
    resources: &crate::resource_cache::ResourceCache,
    id: u32,
) -> MainBaseAbortActorDisposition {
    let actor = entities.main_base_abort_actor_observation(id).unwrap();
    let mut notifications = GameplayNotifications::new();
    match dispatch_native_actor(
        actor,
        entities,
        fx,
        tasks,
        publications,
        &mut MainBaseAbortGameplayContext {
            extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            notifications: &mut notifications,
            retail_tick: 400,
        },
        resources,
    )
    .expect("a native ground actor")
    {
        Ok(success) => success.disposition,
        Err(failure) => panic!("actor {id}: {:?}", failure.callback_error),
    }
}

fn ids(entities: &EntityManager, entity_type: u32) -> Vec<u32> {
    entities
        .iter_all()
        .filter(|entity| entity.entity_type == entity_type)
        .map(|entity| entity.id)
        .collect()
}

/// 170A0's 10C10 takes Type40 to class18: C080 appends its eight Type56
/// children behind the cursor, which then die quietly through class2.
#[v2k_test_support::retail_test]
fn type40_abort_splits_into_type56s_that_die_quietly() {
    let (session, mut entities, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(45);
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert!(tasks.adopt_type40(&entities) > 0);
    tasks.adopt_type56(&entities);
    let mut publications = MainBaseAbortPublicationCounts::default();
    let parents = ids(&entities, 40);
    let before = ids(&entities, 56).len();
    for &id in &parents {
        assert_eq!(
            abort_visit(
                &mut entities,
                &mut fx,
                &mut tasks,
                &mut publications,
                &session.cache,
                id
            ),
            MainBaseAbortActorDisposition::Type40Death
        );
        assert!(entities.pending_actor_deferred_destroy_ids().contains(&id));
        assert_eq!(tasks.family_for(id), None);
    }
    let children = ids(&entities, 56);
    assert_eq!(children.len() - before, 8 * parents.len());
    assert_eq!(publications.appended_type56_actors, 8 * parents.len());
    for id in children {
        assert_eq!(
            abort_visit(
                &mut entities,
                &mut fx,
                &mut tasks,
                &mut publications,
                &session.cache,
                id
            ),
            MainBaseAbortActorDisposition::QuietDeath
        );
        assert!(entities.pending_actor_deferred_destroy_ids().contains(&id));
    }
}

/// Type30's 10C10 takes class12 once, as a lethal hit's publisher does.
#[v2k_test_support::retail_test]
fn type30_abort_takes_class12_once() {
    let (session, mut entities, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(17);
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert!(tasks.adopt_type30(&entities) > 0);
    let mut publications = MainBaseAbortPublicationCounts::default();
    let id = ids(&entities, 30)[0];
    for _ in 0..2 {
        assert_eq!(
            abort_visit(
                &mut entities,
                &mut fx,
                &mut tasks,
                &mut publications,
                &session.cache,
                id
            ),
            MainBaseAbortActorDisposition::Type30Death
        );
    }
    assert_eq!(publications.type30_common_dying, 1);
    assert_eq!(
        tasks.family_for(id),
        Some(SpecializedActorTaskFamily::Intro2CommonDying)
    );
}

/// A Type27 boulder's class18 split appends two Type3s behind the cursor.
#[v2k_test_support::retail_test]
fn type27_abort_splits_into_two_type3s() {
    let (session, mut entities, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(27);
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert!(tasks.adopt_rolling_boulders(&entities) > 0);
    let mut publications = MainBaseAbortPublicationCounts::default();
    let id = ids(&entities, 27)[0];
    let before = ids(&entities, 3).len();
    assert_eq!(
        abort_visit(
            &mut entities,
            &mut fx,
            &mut tasks,
            &mut publications,
            &session.cache,
            id
        ),
        MainBaseAbortActorDisposition::Type27Death
    );
    assert!(entities.pending_actor_deferred_destroy_ids().contains(&id));
    assert_eq!(ids(&entities, 3).len() - before, 2);
    assert_eq!(publications.appended_type3_actors, 2);
}
