use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::{EntityConstructionResources, EntityManager},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailStateWord},
    intro2_type13_live::{
        tick_intro2_type13_scheduler_owner_with_random, Intro2Type13C690Block,
        Intro2Type13PrimaryFrame, Intro2Type13SchedulerOwner,
        Intro2Type13SchedulerProductionOutcome, INTRO2_TYPE13_MODEL_ID, INTRO2_TYPE13_SPAWN_INDEX,
    },
    session::GameSession,
    shared_retarget_mover::SharedRetargetPostUnwind,
    type13_c690::{Type13C690ReselectionBlock, TYPE13_C690_DYING_STATE_BIT},
    world_fx::WorldFx,
};

fn intro2_type_metadata(session: &GameSession) -> Vec<EntityTypeRuntimeMetadata> {
    session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(entity_type, model_slots)| {
            session
                .cache
                .global_entity_type(entity_type)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..EntityTypeRuntimeMetadata::default()
                })
        })
        .collect()
}

fn primary_frame(session: &GameSession, elapsed_micros: u32) -> Intro2Type13PrimaryFrame<'_> {
    Intro2Type13PrimaryFrame {
        dispatch_mode: v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
        terrain: session.cache.level_terrain().expect("Intro2 Section 10"),
        active_model_extent_raw: session
            .cache
            .global_model(INTRO2_TYPE13_MODEL_ID)
            .expect("Ptersect model 291")
            .radius,
        attached_cargo_mass: 0,
        elapsed_micros,
        global_elapsed_micros: elapsed_micros,
        retail_tick: 0,
    }
}

#[v2k_test_support::retail_test]
fn dying_c690_fails_closed_before_selector_rng_or_graph_mutation() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let type_metadata = intro2_type_metadata(&session);
    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection {
            type13: v2k_game::intro2_type13_live::Intro2Type13BirthSelection::CapturedClass7,
            ..Default::default()
        },
        &mut || 0,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");
    let mut owner = Intro2Type13SchedulerOwner::adopt_published(
        manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
            .expect("published type-13"),
    )
    .expect("published B6C0 graph");
    let other_ids: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.id != owner.entity_id())
        .map(|entity| entity.id)
        .collect();
    for entity_id in other_ids {
        let peer = manager.entity_mut(entity_id).expect("captured peer");
        peer.active = false;
        peer.collision.state_flags_at_0x08 = RetailStateWord::exact(0);
    }

    // Install a distant target first so the strict-timeout callback consumes
    // only its one-in-64 gate word before reaching the root state gate.
    let mut world_fx = WorldFx::new();
    let mut prime_words = [0xffff_u32, 0xffff].into_iter();
    let prime = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut world_fx,
        Some(primary_frame(&session, 0)),
        &mut |_| prime_words.next().expect("two retarget words"),
    );
    owner = prime.retained_owner.expect("no-target B6C0 owner");
    assert!(prime_words.next().is_none());

    let entity_before = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("published type-13");
    let context_before = entity_before.current_behavior_context;
    let basis_before = entity_before.physical_body_basis_q31();
    let initial_behavior_before = entity_before.initial_behavior;
    let axis_before = entity_before.actor_common_axis_descriptor;
    let secondary_before = entity_before
        .actor_task_state(ActorTaskSlot::Secondary)
        .copied();
    let tertiary_before = entity_before
        .actor_task_state(ActorTaskSlot::Tertiary)
        .copied();
    let state_bits = entity_before
        .collision
        .state_flags_at_0x08
        .known_value_bits();
    manager
        .entity_mut(owner.entity_id())
        .expect("published type-13")
        .collision
        .state_flags_at_0x08 = RetailStateWord::exact(state_bits | TYPE13_C690_DYING_STATE_BIT);

    let mut draws = 0;
    let mut primary_gate = [1_u32].into_iter();
    let blocked = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut world_fx,
        Some(primary_frame(&session, 501_000)),
        &mut |_| {
            draws += 1;
            primary_gate
                .next()
                .expect("dying C690 must not draw a selector or suffix")
        },
    );
    let request = match blocked.outcome {
        Intro2Type13SchedulerProductionOutcome::C690Blocked {
            request,
            reason:
                Intro2Type13C690Block::Plan(Type13C690ReselectionBlock::AlternateBehaviorUnsupported {
                    class_id: 1,
                }),
            primary: Some(primary),
            ..
        } => {
            assert_eq!(
                primary.post_unwind,
                SharedRetargetPostUnwind::Transition(request)
            );
            request
        }
        other => panic!("dying Type-13 must fail closed at class 1: {other:?}"),
    };
    assert_eq!(draws, 1, "only the old Primary gate may consume RNG");
    assert!(primary_gate.next().is_none());
    owner = blocked
        .retained_owner
        .expect("blocked root retains custody");

    let entity_after = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("blocked type-13");
    assert_eq!(entity_after.current_behavior_context, context_before);
    assert_eq!(
        entity_after.physical_body_basis_q31(),
        basis_before,
        "blocked C690 planning must stop before the post-task basis rebuild"
    );
    assert_eq!(entity_after.initial_behavior, initial_behavior_before);
    assert_eq!(entity_after.actor_common_axis_descriptor, axis_before);
    assert_eq!(
        entity_after
            .actor_task_state(ActorTaskSlot::Secondary)
            .copied(),
        secondary_before
    );
    assert_eq!(
        entity_after
            .actor_task_state(ActorTaskSlot::Tertiary)
            .copied(),
        tertiary_before
    );
    assert!(matches!(
        entity_after.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::SharedRetarget(primary)) if primary.elapsed_ms() == 501
    ));
    let tasks_after_block = ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .map(|slot| entity_after.actor_task_state(slot).copied());
    let sub_g_after_block = entity_after.sub_g_06070_runtime;

    let mut retry_draws = 0;
    let retry = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut world_fx,
        None,
        &mut |_| {
            retry_draws += 1;
            0
        },
    );
    assert!(matches!(
        retry.outcome,
        Intro2Type13SchedulerProductionOutcome::C690Blocked {
            request: retry_request,
            reason: Intro2Type13C690Block::Plan(
                Type13C690ReselectionBlock::AlternateBehaviorUnsupported { class_id: 1 }
            ),
            primary: None,
            ..
        } if retry_request == request
    ));
    assert_eq!(retry_draws, 0);
    assert!(retry.retained_owner.is_some());
    let entity_after_retry = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("retried type-13");
    assert_eq!(
        ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .map(|slot| entity_after_retry.actor_task_state(slot).copied()),
        tasks_after_block
    );
    assert_eq!(entity_after_retry.sub_g_06070_runtime, sub_g_after_block);
    assert_eq!(entity_after_retry.current_behavior_context, context_before);
    assert_eq!(
        entity_after_retry.physical_body_basis_q31(),
        basis_before,
        "retrying the retained C690 request must not publish a basis while planning stays blocked"
    );
}
