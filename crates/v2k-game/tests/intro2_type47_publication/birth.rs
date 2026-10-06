use super::*;
use v2k_game::intro2_type47_live::{
    publish_intro2_type47, world::Intro2Type47WorldOwner, Intro2Type47BirthSelection,
};

#[v2k_test_support::retail_test]
fn native_type47_birth_keeps_sub_a_then_selector_and_admits_guard_or_wander() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata = intro2_type_metadata(&session);
    for (words, class, style, target_speed) in [
        // ceil(9 * 65536 / 10) = 58983 = 0xe667: selector threshold.
        (
            vec![0xffff, 0x1234_e666, 0x1200, 0x3400],
            32,
            0x004c_7bb8,
            306,
        ),
        (vec![0, 0xe667, 0x5600], 6, 0x004c_79c0, 310),
    ] {
        let mut manager = EntityManager::from_level_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            EntityConstructionResources::new(
                session.cache.terrain(),
                session.cache.terrain_objects(),
            ),
        );
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(6))
            .unwrap()
            .id;
        let entity = manager.entity_mut(id).unwrap();
        let mut draws = words.into_iter();
        publish_intro2_type47(
            entity,
            &metadata[47],
            session.cache.terrain(),
            Intro2Type47BirthSelection::Weighted,
            &mut || {
                draws
                    .next()
                    .expect("Sub-A, selector, then each task's 06070 suffix")
            },
        )
        .unwrap();
        assert_eq!(draws.next(), None);
        let RetailRuntimeValue::Known(Some(selection)) = entity.initial_behavior else {
            panic!()
        };
        assert_eq!(selection.program.class_id, class);
        assert_eq!(selection.choice_index, usize::from(class == 6));
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert_eq!(context.active_style().style_address(), style);
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::OrdinaryType9Wander(_))
        ));
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
        if class == 32 {
            assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Secondary),
                Some(ActorTaskRuntime::GuardLocationAcquisition(_))
            ));
        } else {
            assert_eq!(entity.actor_task_state(ActorTaskSlot::Secondary), None);
        }
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!()
        };
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(target_speed)
        );
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(sub_a.drive_scale_percent(), 100);
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(71);
        assert_eq!(
            publish_intro2_type47(
                entity,
                &metadata[47],
                session.cache.terrain(),
                Intro2Type47BirthSelection::Weighted,
                &mut || panic!("a live allocation cannot replay its constructor"),
            ),
            Err(v2k_game::intro2_type47_live::Intro2Type47PublicationError::AlreadyPublished),
        );
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(71)
        );
        let owner = Intro2Type47SchedulerOwner::adopt_published(entity).unwrap();
        assert_eq!(
            Intro2Type47WorldOwner::adopt(&manager, id)
                .unwrap()
                .entity_id(),
            id
        );
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_intro2_type47_guards(&manager), 1);
        let peers: Vec<_> = manager
            .iter_all()
            .filter(|entity| entity.id != id)
            .map(|entity| entity.id)
            .collect();
        for peer in peers {
            manager.entity_mut(peer).unwrap().active = false;
        }
        let tick = tick_intro2_type47_scheduler_owner(
            &mut manager,
            owner,
            &mut WorldFx::new(),
            session.cache.terrain(),
            Intro2Type47CallbackFrame {
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
            },
            &mut |_| 1,
        );
        match tick.outcome {
            Intro2Type47SchedulerProductionOutcome::Primary { visit, .. } if class == 32 => {
                assert_ne!(
                    visit.result,
                    Intro2Type47PrimaryVisitResult::CommonMoverBlocked
                )
            }
            Intro2Type47SchedulerProductionOutcome::WanderNear { visit, .. } if class == 6 => {
                assert_ne!(
                    visit.result,
                    Intro2Type47PrimaryVisitResult::CommonMoverBlocked
                )
            }
            other => panic!("native class {class} must retain its initial live graph: {other:?}"),
        }
        assert_eq!(tick.retained_owner.unwrap().entity_id(), id);
    }
}

#[v2k_test_support::retail_test]
fn captured_guard_policy_keeps_only_its_two_observed_suffix_words() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata = intro2_type_metadata(&session);
    let mut manager = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    );
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(6))
        .unwrap()
        .id;
    let entity = manager.entity_mut(id).unwrap();
    let mut words = [0xffffu32; 2].into_iter();
    publish_intro2_type47(
        entity,
        &metadata[47],
        session.cache.terrain(),
        Intro2Type47BirthSelection::CapturedGuard,
        &mut || words.next().unwrap(),
    )
    .unwrap();
    assert_eq!(words.next(), None);
    assert!(
        matches!(entity.initial_behavior, RetailRuntimeValue::Known(Some(selection)) if selection.program.class_id == 32)
    );
}
