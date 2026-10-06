use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    entity::AuthoredWorldConstruction,
    entity_collision_state::EntityTypeRuntimeMetadata,
    hive_birth::{
        HiveBirthAttempt, HiveBirthHost, HiveBirthRequest, HiveEjectionRequest, HiveEjectionSlot,
    },
    level::LevelState,
    session::GameSession,
    static_damage::StaticDamageScheduler,
};
use v2k_formats::terrain::TerrainGrid;

fn fixture() -> (GameSession, EntityManager, WorldFx, u32) {
    let data = v2k_test_support::retail_dir();
    assert!(
        data.join("PRELOAD.DAT").is_file(),
        "normal-tier corpus required"
    );
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(13, 1).unwrap();
    let metadata: Vec<_> = session
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
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.level_terrain(),
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
        .iter_all()
        .find(|entity| entity.entity_type == 67)
        .unwrap()
        .id;
    (session, manager, fx, hive)
}

fn incomplete_cache(terrain: Option<TerrainGrid>) -> ResourceCache {
    let mut cache = ResourceCache::new(Vec::new());
    cache.load_level(LevelState {
        source_path: "explicit-missing-collision-sections.ovl".into(),
        system_level: None,
        fixup_data: None,
        fixup_code: None,
        strings: Vec::new(),
        sprites: None,
        params: None,
        display_modes: None,
        fog_gradient: None,
        color_palettes: None,
        models: None,
        anim_frames: None,
        terrain,
        anim_sound: None,
        collision: None,
        level: None,
        linkage: None,
    });
    cache
}

#[v2k_test_support::retail_test]
fn missing_actor_resources_keep_native_custody_while_hive_final_ejection_restores_pairs() {
    for has_terrain in [false, true] {
        let (session, mut manager, mut fx, hive) = fixture();
        let others: Vec<_> = manager
            .iter_all()
            .filter(|entity| entity.id != hive)
            .map(|entity| entity.id)
            .collect();
        for id in others {
            manager.entity_mut(id).unwrap().active = false;
        }
        let position = manager.entity_mut(hive).unwrap().position_raw();
        let mut runtime = manager
            .entity_mut(hive)
            .unwrap()
            .authored_radial_emitter
            .as_mut()
            .unwrap()
            .take_birth_runtime()
            .unwrap();
        let extent = |id| session.cache.global_model(id).map(|model| model.radius);
        let mut host = HiveBirthManagerHost::new(
            &mut manager,
            HiveBirthManagerContext {
                resources: EntityConstructionResources {
                    terrain: session.cache.level_terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&extent),
                },
                retail_tick: 0,
                waves_enabled: RetailRuntimeValue::Known(true),
            },
            &mut fx,
        );
        // A real native constructor publishes the child and linear task owner.
        // Nonobjective birth isolates the independent fallback clocks from
        // infection's separately source-backed shared-RNG work.
        let HiveBirthAttempt::Created(child) = host
            .construct_hive_child(HiveBirthRequest {
                entity_type: 15,
                position_raw: position,
                objective: false,
                source_id: hive,
            })
            .unwrap()
        else {
            panic!()
        };
        runtime
            .eject_child(
                HiveEjectionRequest {
                    source_id: hive,
                    child_handle: child,
                },
                &mut host,
            )
            .unwrap();
        let owners = host.take_newborn_owners();
        assert_eq!(owners.len(), 1);
        drop(host);
        let parent = manager.entity_mut(hive).unwrap();
        parent.collision.health_raw = RetailRuntimeValue::Known(0);
        parent
            .collision
            .state_flags_at_0x08
            .overwrite(0x4000, 0x4000);
        let emitter = parent.authored_radial_emitter.as_mut().unwrap();
        emitter.restore_birth_runtime(runtime);
        emitter.enter_dying_slot0();
        emitter.arm_wreck_suction();
        emitter.advance_wreck_timer(6_000_000);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.register_intro2_flyer(owners.into_iter().next().unwrap());
        let child_entity = manager.entity_mut(child).unwrap();
        let before_tasks: Vec<_> = ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .map(|slot| {
                (
                    child_entity.actor_tasks.task_in_slot(slot),
                    child_entity.actor_task_state(slot).copied(),
                )
            })
            .collect();
        let before_motion = (
            child_entity.position_raw(),
            child_entity.velocity_raw(),
            child_entity.physical_body_basis_q31(),
        );
        assert_eq!(
            child_entity.collision.state_flags_at_0x08.masked(0x8000),
            RetailRuntimeValue::Known(0)
        );
        let before_count = manager.iter_all().count();
        let before_ordinal = manager.next_common_body_ordinal();
        let before_seed = fx.next_sub_d_allocation_seed();
        let mut expected = fx.fork_for_main_base_abort_transaction();
        let mut cache =
            incomplete_cache(has_terrain.then(|| session.cache.level_terrain().unwrap().clone()));
        let mut notifications = GameplayNotifications::new();
        let mut tally = WorldCompleteTally::default();
        let mut static_damage = StaticDamageScheduler::new();
        let pass = scheduler.tick(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: Some(HiveComponentProductionContext {
                    world_complete_tally: &mut tally,
                    view_detail: RetailViewDetailContext::from_raw(
                        position.map(i32::from),
                        0,
                        (52, 30),
                    ),
                }),
                resources: &mut cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                elapsed_micros: 500_000,
                global_elapsed_micros: 500_000,
                retail_tick: 25,
                notification_phase: GameplayNotificationPhase::Playing,
                main_base_abort_active: true,
            },
            &mut notifications,
        );
        assert_eq!(
            pass.block,
            Some(if has_terrain {
                SpecializedActorTaskProductionBlock::CurrentLevelCollisionContextUnavailable
            } else {
                SpecializedActorTaskProductionBlock::CurrentLevelTerrainUnavailable
            })
        );
        assert!(
            pass.outcomes.is_empty(),
            "blocked actor was not visited or dropped"
        );
        assert_eq!(scheduler.registered_len(), 1);
        let child_entity = manager.entity_mut(child).unwrap();
        let after_tasks: Vec<_> = ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .map(|slot| {
                (
                    child_entity.actor_tasks.task_in_slot(slot),
                    child_entity.actor_task_state(slot).copied(),
                )
            })
            .collect();
        assert_eq!(
            after_tasks, before_tasks,
            "actor preflight retains task identities and clocks"
        );
        assert_eq!(
            (
                child_entity.position_raw(),
                child_entity.velocity_raw(),
                child_entity.physical_body_basis_q31()
            ),
            before_motion
        );
        assert_eq!(
            child_entity.collision.state_flags_at_0x08.masked(0x8000),
            RetailRuntimeValue::Known(0x8000)
        );
        let emitter = manager
            .entity_mut(hive)
            .unwrap()
            .authored_radial_emitter
            .as_ref()
            .unwrap();
        assert_eq!(emitter.suction_timer_us(), 500_000);
        assert_eq!(emitter.sub_k_output(), 1_953);
        assert_eq!(
            emitter.birth_runtime().unwrap().ejection_slots(),
            &[HiveEjectionSlot::default(); 2]
        );
        assert_eq!(manager.iter_all().count(), before_count);
        assert_eq!(manager.next_common_body_ordinal(), before_ordinal);
        assert_eq!(fx.next_sub_d_allocation_seed(), before_seed);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16(),
            "final CA90 and dead-state clocks consume no RNG"
        );
        assert!(
            has_terrain
                || scheduler
                    .hive_diagnostics
                    .iter()
                    .any(|message| message.contains("MissingTerrain"))
        );
    }
}
