use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    intro2_flyers_live::{
        tick_intro2_flyer_scheduler_owner, Intro2FlyerFrame, Intro2FlyerSchedulerProductionOutcome,
    },
    native_flying_surface_contact::{
        publish_native_flying_standard_death, NativeFlyingSurfaceDeathPublication,
    },
    session::GameSession,
};

fn fixture() -> (GameSession, EntityManager, WorldFx, HiveBirthRequest) {
    let data = v2k_test_support::retail_dir();
    assert!(
        data.join("PRELOAD.DAT").is_file(),
        "normal-tier corpus required"
    );
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(13, 1).unwrap();
    let rows: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    let mut fx = WorldFx::new();
    let manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: 1,
            type_metadata: &rows,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: None,
            retail_tick: 0,
        },
        &mut fx,
    )
    .unwrap();
    let hive = manager
        .entities
        .iter()
        .find(|entity| entity.entity_type == 67)
        .unwrap();
    let request = HiveBirthRequest {
        entity_type: 15,
        position_raw: hive.position_raw(),
        objective: true,
        source_id: hive.id,
    };
    (session, manager, fx, request)
}

fn context(session: &GameSession) -> NativeHiveChildConstructionContext<'_> {
    NativeHiveChildConstructionContext {
        resources: EntityConstructionResources::new(
            session.cache.terrain(),
            session.cache.terrain_objects(),
        ),
        retail_tick: 123,
        waves_enabled: true,
    }
}

#[v2k_test_support::retail_test]
fn hive_wasp_actual_native_append_retains_body_process_history_and_class7() {
    let (session, mut manager, mut fx, request) = fixture();
    let RetailRuntimeValue::Known(ordinal) = manager.next_common_body_ordinal() else {
        panic!()
    };
    let seed = fx.next_sub_d_allocation_seed();
    let before_count = manager.entities.len();
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let words: Vec<_> = (0..4)
        .map(|_| expected.next_shared_retail_random_u16())
        .collect();
    let owner = manager
        .append_native_hive_child(request, context(&session), &mut fx)
        .unwrap();
    let child = manager.entities.last().unwrap();
    assert_eq!(child.id, owner.entity_id());
    assert_eq!(manager.entities.len(), before_count + 1);
    assert_eq!(child.authored_spawn_index, None);
    assert_eq!(child.position_raw(), request.position_raw);
    assert_eq!(child.velocity_raw(), [0; 3]);
    assert_eq!(child.rotation_heading_pitch_roll_raw(), [0; 3]);
    assert_eq!(
        child.construction_stamp_at_0xb4,
        RetailRuntimeValue::Known(0x400_u16.wrapping_add(ordinal))
    );
    assert_eq!(
        manager.next_common_body_ordinal(),
        RetailRuntimeValue::Known(ordinal.wrapping_add(1))
    );
    assert_eq!(fx.next_sub_d_allocation_seed(), seed.wrapping_add(1));
    assert_eq!(
        child
            .intro2_flyer_frame_owner
            .unwrap()
            .sub_d_frame_owner
            .classifier_cache()
            .stagger_counter(),
        seed
    );
    let RetailRuntimeValue::Known(Some(sub_g)) = child.sub_g_06070_runtime else {
        panic!()
    };
    assert_eq!(
        sub_g.randomized_target_raw_at_0x38(),
        RetailRuntimeValue::Known(100 + i32::from(words[3] >> 8))
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
    assert_eq!(
        child.collision.recent_relation_id_at_0x60,
        RetailRuntimeValue::Known(None),
        "1BEB0 source suffix has not run"
    );
    assert_eq!(child.attached_to, None);
    assert_eq!(
        child.collision.state_flags_at_0x08.masked(0x0100_0000),
        RetailRuntimeValue::Known(0x0100_0000)
    );
    assert!(matches!(
        child.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::SharedRetarget(_))
    ));
    assert!(matches!(
        child.actor_task_state(ActorTaskSlot::Secondary),
        Some(ActorTaskRuntime::TargetAcquisition(_))
    ));
    assert!(crate::intro2_flyers_live::flyer_manager_identity_authenticates(&manager, child.id));
}

#[v2k_test_support::retail_test]
fn hive_wasp_unowned_type_metadata_or_resource_rejects_without_body_sub_d_rng_or_list() {
    let (session, manager, mut fx, request) = fixture();
    for case in 0..4 {
        let mut manager = manager.fork_for_main_base_abort_transaction();
        let before_count = manager.entities.len();
        let before_stamp = manager.next_common_body_ordinal();
        let before_id = manager.next_entity_id;
        let before_seed = fx.next_sub_d_allocation_seed();
        let mut expected = fx.fork_for_main_base_abort_transaction();
        let mut request = request;
        let mut context = context(&session);
        match case {
            // Type37 has no native constructor.
            0 => request.entity_type = 37,
            1 => {
                manager.type_metadata[15].projectile_emitter_descriptor =
                    RetailRuntimeValue::Unresolved
            }
            2 => context.resources.terrain = None,
            3 => manager.common_body_stamps = None,
            _ => unreachable!(),
        }
        let after_override_stamp = manager.next_common_body_ordinal();
        assert!(
            manager
                .append_native_hive_child(request, context, &mut fx)
                .is_err(),
            "case {case}"
        );
        assert_eq!(manager.entities.len(), before_count);
        assert_eq!(manager.next_entity_id, before_id);
        assert_eq!(manager.next_common_body_ordinal(), after_override_stamp);
        if case != 3 {
            assert_eq!(after_override_stamp, before_stamp);
        }
        assert_eq!(fx.next_sub_d_allocation_seed(), before_seed);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn hive_wasp_native_source_suffix_runs_class7_and_quiet_death_clears_tasks() {
    let (session, mut manager, mut fx, request) = fixture();
    let owner = manager
        .append_native_hive_child(request, context(&session), &mut fx)
        .unwrap();
    let NativeHiveNewborn::Flyer(owner) = owner else {
        panic!("Type15 births a Wasp owner");
    };
    let id = owner.entity_id();
    for entity in &mut manager.entities {
        if entity.id != id {
            entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0);
        }
    }
    let child = manager.entity_mut(id).unwrap();
    child.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(Some(request.source_id));
    child.set_position_raw([request.position_raw[0], 400, request.position_raw[2]]);
    crate::entity_view_detail::RetailViewDetailContext::from_raw(
        child.position_raw().map(i32::from),
        0,
        (52, 30),
    )
    .publish(
        child.position_raw(),
        &mut child.collision.state_flags_at_0x08,
    );
    let tick = tick_intro2_flyer_scheduler_owner(
        &mut manager,
        owner,
        Intro2FlyerFrame {
            resources: &session.cache,
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            retail_tick: 123,
        },
        &mut fx,
    );
    assert!(
        matches!(
            tick.outcome,
            Intro2FlyerSchedulerProductionOutcome::B6c0Visit { .. }
        ),
        "{:?}",
        tick.outcome
    );
    assert!(tick.retained_owner.is_some());
    let child = manager.entity_mut(id).unwrap();
    assert_eq!(
        child.collision.recent_relation_id_at_0x60,
        RetailRuntimeValue::Known(Some(request.source_id))
    );
    let death = publish_native_flying_standard_death(&mut manager, id, &mut fx).unwrap();
    assert!(
        matches!(death.publication, Some(NativeFlyingSurfaceDeathPublication::QuietDeath(lease)) if lease.entity_id == id)
    );
    assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
    let child = manager.entity_mut(id).unwrap();
    assert_eq!(child.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        child.collision.constructor_sound_attachment_id_at_0x8c,
        RetailRuntimeValue::Known(None)
    );
    assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .all(|slot| child.actor_tasks.task_in_slot(slot).is_none()));
}

/// Each worker row's own Hive births its worker through the shared native
/// constructor, seeing the whole live list, and the owner keeps it ticking.
#[v2k_test_support::retail_test]
fn hive_workers_take_the_authored_constructor_with_a_zeroed_record() {
    use crate::{
        gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
        specialized_actor_task_production::{
            SpecializedActorTaskProductionFrame, SpecializedActorTaskScheduler,
            SpecializedActorTaskWorld,
        },
        static_damage::StaticDamageScheduler,
    };
    for (level, hive_spawn, entity_type) in [
        (25, 86, 8),
        (27, 40, 91),
        (28, 2, 90),
        (29, 0, 79),
        (41, 1, 116),
    ] {
        let (mut session, mut manager, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level);
        let hive = manager
            .entities
            .iter()
            .find(|entity| entity.authored_spawn_index == Some(hive_spawn))
            .unwrap();
        assert_eq!(hive.entity_type, 67, "world{level}");
        let request = HiveBirthRequest {
            entity_type,
            position_raw: hive.position_raw(),
            objective: false,
            source_id: hive.id,
        };
        let before = manager.entities.len();
        let newborn = manager
            .append_native_hive_child(request, context(&session), &mut fx)
            .unwrap_or_else(|block| panic!("world{level} Type{entity_type}: {block:?}"));
        let NativeHiveNewborn::Worker(owner) = newborn else {
            panic!("world{level}: {newborn:?}");
        };
        assert_eq!(manager.entities.len(), before + 1);
        let child = manager.entities.last().unwrap();
        assert_eq!(child.id, owner.entity_id());
        assert_eq!(child.entity_type, entity_type);
        assert_eq!(child.authored_spawn_index, None);
        assert_eq!(child.rotation_heading_pitch_roll_raw(), [0; 3]);
        assert!(
            crate::intro2_type8::intro2_type8_manager_allocation_authenticates(&manager, child.id)
        );
        let id = child.id;
        let mut tasks = SpecializedActorTaskScheduler::new();
        tasks.adopt_intro2_type8(&mut manager);
        tasks.register_intro2_type8(owner);
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        for t in 1..=100u32 {
            let pass = tasks.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase: GameplayNotificationPhase::Playing,
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    static_damage: &mut static_damage,
                    elapsed_micros: 20_000,
                    global_elapsed_micros: t * 20_000,
                    retail_tick: t,
                    main_base_abort_active: false,
                },
                &mut notifications,
            );
            assert!(
                pass.block.is_none(),
                "world{level} tick{t}: {:?}",
                pass.block
            );
            for outcome in &pass.outcomes {
                assert!(
                    outcome.entity_id() != id || !format!("{outcome:?}").contains("Block"),
                    "world{level} tick{t}: {outcome:?}"
                );
            }
        }
        assert!(manager.entities.iter().any(|entity| entity.id == id));
    }
}

/// World24's Hive30 births Type94 (an objective row) through its ordinary
/// constructor with a zeroed record; the owner adopts the allocation.
#[v2k_test_support::retail_test]
fn hive_type94_takes_its_ordinary_constructor_with_a_zeroed_record() {
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(24);
    let hive = manager
        .entities
        .iter()
        .find(|entity| entity.authored_spawn_index == Some(30))
        .unwrap();
    assert_eq!(hive.entity_type, 67);
    let request = HiveBirthRequest {
        entity_type: 94,
        position_raw: hive.position_raw(),
        objective: true,
        source_id: hive.id,
    };
    let newborn = manager
        .append_native_hive_child(request, context(&session), &mut fx)
        .unwrap();
    let NativeHiveNewborn::Type94(owner) = newborn else {
        panic!("{newborn:?}");
    };
    let child = manager.entities.last().unwrap();
    assert_eq!(child.id, owner.entity_id());
    assert_eq!(child.authored_spawn_index, None);
    assert!(crate::intro2_type94::type94_manager_allocation_authenticates(&manager, child.id));
    // The objective bit reaches 104B0's spawn-parameter state.
    assert_eq!(
        child.collision.state_flags_at_0x08.masked(0x0100_0000),
        RetailRuntimeValue::Known(0x0100_0000)
    );
}
