use super::*;
use v2k_game::intro2_type13_live::{
    publish_intro2_type13, Intro2Type13BirthSelection, Intro2Type13WorldOwner,
};

#[v2k_test_support::retail_test]
fn native_birth_draws_the_selector_after_1b8c0_and_retains_either_live_graph() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata = intro2_type_metadata(&session);

    for (words, class_id, lifetime_ms, target, style) in [
        // The constructor would select class 7 if mistaken for the selector.
        (
            vec![0xffff, 0x1234_3fff, 0x2200],
            5,
            5_000,
            234,
            0x004c_7930,
        ),
        // The real second word is the first value selecting class 7.
        (vec![0, 0x4000, 0x2200, 0x3300], 7, 500, 251, 0x004c_7a50),
    ] {
        let mut manager = EntityManager::from_level_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
        );
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
            .unwrap()
            .id;
        let entity = manager.entity_mut(id).unwrap();
        let mut draws = words.into_iter();
        publish_intro2_type13(
            entity,
            &metadata[TYPE13_ENTITY_TYPE as usize],
            Intro2Type13BirthSelection::Weighted,
            &mut || {
                draws
                    .next()
                    .expect("only the authored constructor/selector/suffix draws")
            },
        )
        .unwrap();
        assert_eq!(draws.next(), None);
        let RetailRuntimeValue::Known(Some(selection)) = entity.initial_behavior else {
            panic!("birth selection must be retained");
        };
        assert_eq!(selection.program.class_id, class_id);
        assert_eq!(selection.choice_index, usize::from(class_id == 7));
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("birth must publish its live behavior context");
        };
        assert_eq!(context.active_style().style_address(), style);
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(task)) if task.lifetime_ms() == lifetime_ms
        ));
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
        if class_id == 5 {
            assert_eq!(entity.actor_task_state(ActorTaskSlot::Secondary), None);
        } else {
            assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Secondary),
                Some(ActorTaskRuntime::TargetAcquisition(_))
            ));
        }
        let RetailRuntimeValue::Known(Some(sub_g)) = entity.sub_g_06070_runtime else {
            panic!("native Sub-G storage");
        };
        assert_eq!(
            sub_g.randomized_target_raw_at_0x38(),
            RetailRuntimeValue::Known(target)
        );
        assert!(intro2_uses_live_actor_pose(entity));
        let owner = Intro2Type13SchedulerOwner::adopt_published(entity).unwrap();
        assert_eq!(
            Intro2Type13WorldOwner::adopt(&manager).unwrap().entity_id(),
            id
        );
        let mut production = SpecializedActorTaskScheduler::new();
        assert_eq!(production.adopt_intro2_type13_search_attack(&manager), 1);
        silence_non_owner_peers(&mut manager, id);

        let mut target_words = [0u32, 0xffff].into_iter();
        let tick = tick_intro2_type13_scheduler_owner_with_random(
            &mut manager,
            owner,
            &mut WorldFx::new(),
            Some(primary_frame(&session, 20_000)),
            &mut |_| target_words.next().expect("first near-axis target X/Z"),
        );
        assert_eq!(target_words.next(), None);
        match tick.outcome {
            Intro2Type13SchedulerProductionOutcome::Class5AimlessVisit { primary, .. }
                if class_id == 5 =>
            {
                assert_eq!(primary.result, Intro2Type13PrimaryVisitResult::Continue)
            }
            Intro2Type13SchedulerProductionOutcome::B6c0Visit { primary, .. } if class_id == 7 => {
                assert_eq!(primary.result, Intro2Type13PrimaryVisitResult::Continue)
            }
            other => panic!("native class {class_id} must keep its first live visit: {other:?}"),
        }
        assert_eq!(tick.retained_owner.unwrap().entity_id(), id);
    }
}

#[v2k_test_support::retail_test]
fn captured_birth_explicitly_retains_class7_without_replaying_the_selector() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata = intro2_type_metadata(&session);
    let mut manager = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
    );
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(0))
        .unwrap()
        .id;
    let entity = manager.entity_mut(id).unwrap();
    let mut draws = [0u32; 3].into_iter();
    publish_intro2_type13(
        entity,
        &metadata[TYPE13_ENTITY_TYPE as usize],
        Intro2Type13BirthSelection::CapturedClass7,
        &mut || {
            draws
                .next()
                .expect("1B8C0 and the two captured B6C0 suffix words")
        },
    )
    .unwrap();
    assert_eq!(draws.next(), None);
    assert!(
        matches!(entity.initial_behavior, RetailRuntimeValue::Known(Some(selection)) if selection.program.class_id == 7)
    );
    assert!(Intro2Type13SchedulerOwner::adopt_published(entity).is_ok());
}

#[v2k_test_support::retail_test]
fn native_full_constructor_enters_first_world_callback_and_preserves_later_mass_contributions() {
    use v2k_game::entity_collision_state::{
        FULLY_ABOVE_SURFACE_STATE_BIT, FULLY_BELOW_SURFACE_STATE_BIT,
    };
    use v2k_game::entity_initializer::CONSTRUCTOR_SURFACE_STATE_MASK;
    use v2k_game::intro2_type13_live::{
        tick_intro2_type13_world_owner_with_random, Intro2Type13WorldOutcome,
    };
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata = intro2_type_metadata(&session);
    let level = session.cache.level_desc().unwrap();
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let generic = EntityManager::from_level_with_type_metadata(level, &metadata, resources);
    let generic_actor = generic
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(0))
        .unwrap();
    assert_eq!(
        generic_actor.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Unresolved
    );
    assert_eq!(
        generic_actor
            .collision
            .state_flags_at_0x08
            .masked(CONSTRUCTOR_SURFACE_STATE_MASK),
        RetailRuntimeValue::Unresolved,
        "generic construction does not claim native 104B0 surface ownership"
    );

    // 104B0 classifies the authored center before D4A0/component placement.
    // A dry integer cell bypasses waves, so this captured constructor needs
    // no invented clock and no injected exact state word to own these bits.
    let [x, y, z] = level
        .entities
        .iter()
        .find(|spawn| spawn.index == INTRO2_TYPE13_SPAWN_INDEX)
        .unwrap()
        .position_raw();
    let terrain = session.cache.terrain().unwrap();
    let cell = terrain
        .cell(usize::from((x as u16) >> 8), usize::from((z as u16) >> 8))
        .unwrap();
    let sea = terrain.sea_level_raw();
    assert!(i16::from(cell.height as i8) * 32 >= sea);
    let expected_surface_bits = match y.cmp(&sea) {
        std::cmp::Ordering::Less => FULLY_BELOW_SURFACE_STATE_BIT,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => FULLY_ABOVE_SURFACE_STATE_BIT,
    };
    let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &metadata,
        resources,
        Default::default(),
        &mut || 0,
    )
    .unwrap();
    let owner = Intro2Type13WorldOwner::adopt(&manager).unwrap();
    let id = owner.entity_id();
    let before = manager.iter_all().find(|entity| entity.id == id).unwrap();
    assert_eq!(
        before.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(
        before
            .collision
            .state_flags_at_0x08
            .masked(CONSTRUCTOR_SURFACE_STATE_MASK),
        RetailRuntimeValue::Known(expected_surface_bits),
        "the native dry birth must retain its source-derived surface classification"
    );
    let before_position = before.position_raw();
    let mut fx = WorldFx::new();
    let first = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut fx,
        owner,
        Some(primary_frame(&session, 20_000)),
        &mut |_| 0,
    );
    assert!(
        matches!(
            first.outcome,
            Intro2Type13WorldOutcome::Task {
                completed: true,
                ..
            }
        ),
        "native first callback must complete: {:?}",
        first.outcome
    );
    let after = manager.iter_all().find(|entity| entity.id == id).unwrap();
    assert_ne!(after.position_raw(), before_position);
    assert_eq!(after.mass_raw, 100);
    let subject = manager.entity_mut(id).unwrap();
    subject.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(77);
    let next = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut fx,
        first.retained_owner.unwrap(),
        Some(primary_frame(&session, 20_000)),
        &mut |_| 0,
    );
    assert!(
        matches!(
            next.outcome,
            Intro2Type13WorldOutcome::Task {
                completed: true,
                ..
            }
        ),
        "later callback must complete: {:?}",
        next.outcome
    );
    let after = manager.iter_all().find(|entity| entity.id == id).unwrap();
    assert_eq!(
        after.mass_raw, 177,
        "admission must retain the later +B2 contribution"
    );
    assert_eq!(
        after.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0),
        "the ordinary post-callback phase owns clearing that contribution"
    );
}
