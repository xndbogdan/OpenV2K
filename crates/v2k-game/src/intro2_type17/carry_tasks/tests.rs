use super::*;
use crate::entity_collision_state::RetailStateWord;

fn capture_fixture() -> Option<(
    crate::session::GameSession,
    EntityManager,
    Vec<EntityTypeRuntimeMetadata>,
    u32,
)> {
    let (session, metadata) = super::super::tests::fixture()?;
    let mut manager = super::super::tests::generic(&session, &metadata);
    let mut candidates = super::super::tests::generic(&session, &metadata);
    let id = 5;
    let position = manager.entity_mut(id).unwrap().position_raw();
    let target = candidates.entity_mut(1).unwrap();
    target.capability_flags = 0xC00;
    target.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
    target.set_position_raw(position);
    super::super::native::publish_intro2_type17(
        manager.entity_mut(id).unwrap(),
        &metadata[17],
        std::slice::from_ref(target),
        session.cache.terrain().unwrap(),
        &mut || 0,
    )
    .unwrap();
    Some((session, manager, metadata, id))
}

#[v2k_test_support::retail_test]
fn carrying_initializers_publish_exact_lifetimes_slots_and_suffix_draws() {
    for (variant, draws) in [(2, 1), (3, 2), (4, 1), (5, 1)] {
        let Some((_session, mut manager, _metadata, id)) = capture_fixture() else {
            return;
        };
        let before = manager.entity_mut(id).unwrap().initial_behavior;
        let mut fx = WorldFx::new();
        let mut expected = WorldFx::new();
        for _ in 0..draws {
            expected.next_shared_retail_random_u16();
        }
        publish_style(
            &mut manager,
            id,
            variant,
            CaptureTargetWrite::Set(Some(1)),
            &mut fx,
        )
        .unwrap();
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.initial_behavior, before);
        assert_eq!(carrying_variant(entity), Some(variant));
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        match variant {
            2 => assert!(matches!(entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::CapturePeoplePursuit(task))
                    if task.lifetime() == SharedTargetRouteLifetime::Unlimited && task.target_id() == Some(1))),
            4 => assert!(matches!(entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::CapturePeopleFollowing(task)) if task.target_id() == 1 && task.elapsed_ms() == 0)),
            3 | 5 => assert!(matches!(entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::SharedRetarget(task)) if task.lifetime_ms() == if variant == 3 { 500 } else { 5_000 })),
            _ => unreachable!(),
        }
        if variant == 3 {
            assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Secondary),
                Some(ActorTaskRuntime::CaptureBeaconAcquisition)
            ));
        } else {
            assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        }
        if matches!(variant, 2 | 4) {
            let RetailRuntimeValue::Known(Some(a)) = entity.sub_a_propulsion_runtime else {
                panic!()
            };
            assert_eq!(a.target_speed_raw(), RetailRuntimeValue::Known(333));
        }
    }
}

#[v2k_test_support::retail_test]
fn constructor_failure_keeps_style_prefix_then_publishes_fallback_without_rng() {
    let Some((_session, mut manager, _metadata, id)) = capture_fixture() else {
        return;
    };
    manager.entity_mut(id).unwrap().sub_h_external_frame_runtime = RetailRuntimeValue::Known(None);
    let mut fx = WorldFx::new();
    let mut expected = WorldFx::new();
    assert!(publish_style(
        &mut manager,
        id,
        3,
        CaptureTargetWrite::Set(Some(77)),
        &mut fx
    )
    .is_err());
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
    let entity = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!()
    };
    assert_eq!(
        context.descriptor(),
        BehaviorDescriptorIdentity::InitializerFailureFallback
    );
    assert_eq!(
        context.target_handle_at_0x08(),
        RetailRuntimeValue::Known(Some(77))
    );
    for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        assert!(entity.actor_task_state(slot).is_none());
    }
}

#[v2k_test_support::retail_test]
fn beacon_search_uses_lowest_signed_score_and_keeps_sentinel_failure() {
    let Some((_session, mut manager, _metadata, id)) = capture_fixture() else {
        return;
    };
    let position = manager.entity_mut(id).unwrap().position_raw();
    let ids: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
    for candidate in ids {
        let entity = manager.entity_mut(candidate).unwrap();
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
        entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        entity.capability_flags = 0;
    }
    // Controlled selector records preserve the actual list order; only the
    // fields consumed by22F10 are varied, independently of the task publisher.
    for (candidate, score) in [(1, 50), (2, -7), (3, 0)] {
        let entity = manager.entity_mut(candidate).unwrap();
        entity.capability_flags = 0x100;
        entity.authored_follow_beacon_priority_raw = Some(score);
        entity.set_position_raw(position);
    }
    let axis = CommonAxisDescriptor {
        strict_axis_limit_raw: 0xA00,
        raw_word_at_0x04: 0x100,
    };
    let mut fx = WorldFx::new();
    let mut expected = WorldFx::new();
    assert_eq!(select_beacon(&manager, id, axis, &mut fx), Ok(Some(2)));
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16(),
        "distinct scores draw no tie RNG"
    );
    for candidate in [1, 2, 3] {
        manager
            .entity_mut(candidate)
            .unwrap()
            .authored_follow_beacon_priority_raw = Some(65_535);
    }
    let mut expected = fx.fork_for_main_base_abort_transaction();
    for _ in 0..3 {
        expected.next_shared_retail_random_u16();
    }
    assert_eq!(select_beacon(&manager, id, axis, &mut fx), Ok(None));
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}
