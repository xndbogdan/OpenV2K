//! Native Type54 construction and Change-Sea-Level abort from ordinary loads.

use v2k_formats::levels::LevelDescriptor;
use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::{
        AuthoredPlayerArrival, AuthoredWorldConstruction, CampaignCargoControllerState,
        CampaignCargoRestoreContext, CampaignCargoRestoreError, Class0ActorOwner,
        EntityConstructionResources, EntityManager,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue, DYING_STATE_BIT},
    gameplay_notifications::GameplayNotifications,
    main_base_abort_production::{
        apply_type54_abort_with_scheduler, MainBaseAbortPublicationCounts,
    },
    main_base_type54_abort::{
        change_sea_level_delta_raw, MainBaseType54DeathAdvance, MainBaseType54DeathBlock,
        MainBaseType54DeathOutcome, MainBaseType54NetworkSession, MainBaseType54SeaDeltaSource,
        LEVEL_ONE_TYPE54_MODEL_EXTENT_RAW,
    },
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskFamily, SpecializedActorTaskScheduler,
    },
    world_fx::WorldFx,
};

struct World {
    session: GameSession,
    manager: EntityManager,
    fx: WorldFx,
}

impl World {
    fn load(level_id: u32) -> Self {
        Self::load_with_spawn_edit(level_id, |_| {})
    }

    fn load_with_spawn_edit(level_id: u32, edit: impl FnOnce(&mut LevelDescriptor)) -> Self {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").is_file(),
            "normal-tier corpus required"
        );
        let mut session = GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(level_id, 1).unwrap();
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(id, slots)| {
                session
                    .cache
                    .global_entity_type(id)
                    .map(EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(EntityTypeRuntimeMetadata {
                        model_slots: *slots,
                        ..Default::default()
                    })
            })
            .collect();
        let bytes =
            std::fs::read(data.join("Overlay").join(format!("1X{level_id}XX.OVL"))).unwrap();
        let overlay = v2k_formats::ovl::OvlFile::parse(&bytes).unwrap();
        let mut level =
            v2k_formats::levels::parse_level(&overlay.section(13).unwrap().data).unwrap();
        edit(&mut level);
        let mut fx = WorldFx::new();
        let manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                logical_world_index: (level_id - 12) as i32,
                level: &level,
                type_metadata: &metadata,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| {
                        session.cache.global_model(id).map(|model| model.radius)
                    }),
                },
                player_arrival: Some(AuthoredPlayerArrival {
                    position_raw: [19_712, -500, 14_848],
                    heading_raw: 0x4000,
                }),
                retail_tick: 4793,
            },
            &mut fx,
        )
        .unwrap();
        Self {
            session,
            manager,
            fx,
        }
    }

    fn first(&self, entity_type: u32) -> u32 {
        self.manager
            .iter_all()
            .find(|entity| entity.entity_type == entity_type)
            .unwrap_or_else(|| panic!("authored type{entity_type} required"))
            .id
    }

    fn model_extent(&self, model: usize) -> u16 {
        self.session
            .cache
            .global_model(model)
            .map(|model| model.radius)
            .expect("active model header08")
    }
}

fn abort_type54(world: &mut World, id: u32) -> MainBaseType54DeathAdvance {
    let (model, y, sea_source, primary_is_timer) = {
        let entity = world
            .manager
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap();
        (
            entity.model_index.expect("active Type54 model"),
            entity.position_raw()[1],
            entity.main_base_type54_sea_delta_source,
            matches!(
                entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::Class0Timer(_))
            ),
        )
    };
    assert!(primary_is_timer);
    assert_eq!(
        sea_source,
        RetailRuntimeValue::Known(MainBaseType54SeaDeltaSource::DeriveFromActorModelAndSea)
    );
    assert_eq!(
        Class0ActorOwner::adopt(&world.manager, id)
            .unwrap()
            .entity_id(),
        id
    );
    let extent = world.model_extent(model);
    let observation = world.manager.main_base_abort_actor_observation(id).unwrap();
    let terrain = world.session.cache.terrain().unwrap();
    let expected_delta = change_sea_level_delta_raw(y, extent, terrain.header[0]);
    let advance = world
        .manager
        .apply_main_base_abort_type54_death(
            observation.lease,
            terrain,
            extent,
            MainBaseType54NetworkSession::SoloNetworkingDisabled,
            &mut world.fx,
        )
        .expect("native Type54 abort");
    let MainBaseType54DeathAdvance::Advanced { outcome, .. } = &advance else {
        panic!("post-callback successor remains authenticated");
    };
    let MainBaseType54DeathOutcome::SeaLevelTaskPublished {
        entity_id,
        sea_delta_raw,
        ..
    } = outcome
    else {
        panic!("class 38 published Change Sea Level");
    };
    assert_eq!(*entity_id, id);
    assert_eq!(*sea_delta_raw, expected_delta);
    let entity = world
        .manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap();
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
        RetailRuntimeValue::Known(DYING_STATE_BIT)
    );
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::ChangeSeaLevel(_))
    ));
    advance
}

#[v2k_test_support::retail_test]
fn overlay13_native_type54_constructs_and_aborts_without_spawn_index_admission() {
    let mut authored = World::load(13);
    let authored_id = authored.first(54);
    let authored_spawn = authored
        .manager
        .iter_all()
        .find(|entity| entity.id == authored_id)
        .unwrap()
        .authored_spawn_index;
    assert_eq!(authored_spawn, Some(8));
    abort_type54(&mut authored, authored_id);

    let mut rotated = World::load_with_spawn_edit(13, |level| {
        let spawn = level
            .entities
            .iter_mut()
            .find(|spawn| spawn.entity_type == 54)
            .unwrap();
        spawn.rotation = [0x1234, 0x5678, 0x9abc];
    });
    let rotated_id = rotated.first(54);
    assert_eq!(
        rotated
            .manager
            .iter_all()
            .find(|entity| entity.id == rotated_id)
            .unwrap()
            .rotation_heading_pitch_roll_raw(),
        [0x1234, 0x5678_u16 as i16, 0x9abc_u16 as i16]
    );
    abort_type54(&mut rotated, rotated_id);
}

#[v2k_test_support::retail_test]
fn later_world_type54_constructs_outside_overlay13_spawn_tables() {
    let mut world = World::load_with_spawn_edit(14, |level| {
        let spawn = level
            .entities
            .iter_mut()
            .find(|spawn| spawn.entity_type == 62)
            .expect("Medaeval authors Type62");
        spawn.entity_type = 54;
        spawn.model_overrides = [560, 560, 560, 560];
    });
    let id = world.first(54);
    let entity = world
        .manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap();
    assert_eq!(entity.authored_spawn_index, Some(1));
    assert_eq!(entity.model_slots, [Some(560); 4]);
    abort_type54(&mut world, id);
}

fn production_abort_type54(world: &mut World, id: u32, adopt_class0: bool) {
    let mut scheduler = SpecializedActorTaskScheduler::new();
    if adopt_class0 {
        assert!(scheduler.adopt_class0_actors(&world.manager) >= 1);
        assert!(Class0ActorOwner::adopt(&world.manager, id).is_ok());
        assert_eq!(
            scheduler.family_for(id),
            Some(SpecializedActorTaskFamily::Class0Actor)
        );
    } else {
        assert_eq!(scheduler.family_for(id), None);
        assert!(Class0ActorOwner::adopt(&world.manager, id).is_ok());
    }
    let y = world
        .manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .position_raw()[1];
    let observation = world.manager.main_base_abort_actor_observation(id).unwrap();
    let mut publications = MainBaseAbortPublicationCounts::default();
    let terrain = world.session.cache.terrain().unwrap();
    let expected_delta =
        change_sea_level_delta_raw(y, LEVEL_ONE_TYPE54_MODEL_EXTENT_RAW, terrain.header[0]);
    apply_type54_abort_with_scheduler(
        observation.lease,
        &mut world.manager,
        terrain,
        &mut world.fx,
        &mut scheduler,
        &mut publications,
    )
    .expect("production Type54 abort");
    let entity = world
        .manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap();
    match entity.actor_task_state(ActorTaskSlot::Primary) {
        Some(ActorTaskRuntime::ChangeSeaLevel(state)) => {
            assert_eq!(state.remaining_delta_raw(), expected_delta);
        }
        other => panic!("expected Change Sea Level, got {other:?}"),
    }
    assert_eq!(publications.type54_sea_level, 1);
    assert!(Class0ActorOwner::adopt(&world.manager, id).is_err());
    assert_eq!(
        scheduler.family_for(id),
        Some(SpecializedActorTaskFamily::MainBaseType54SeaLevel)
    );
    assert!(matches!(
        world
            .manager
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap()
            .actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::ChangeSeaLevel(_))
    ));
}

#[v2k_test_support::retail_test]
fn production_type54_abort_takes_class0_then_registers_change_sea_level() {
    let mut world = World::load(13);
    let id = world.first(54);
    production_abort_type54(&mut world, id, true);
}

#[v2k_test_support::retail_test]
fn production_type54_abort_without_class0_owner_still_registers() {
    let mut world = World::load(13);
    let id = world.first(54);
    production_abort_type54(&mut world, id, false);
}

#[v2k_test_support::retail_test]
fn later_world_production_type54_abort_does_not_conflict_with_class0() {
    let mut world = World::load_with_spawn_edit(14, |level| {
        let spawn = level
            .entities
            .iter_mut()
            .find(|spawn| spawn.entity_type == 62)
            .expect("Medaeval authors Type62");
        spawn.entity_type = 54;
        spawn.model_overrides = [560, 560, 560, 560];
    });
    let id = world.first(54);
    production_abort_type54(&mut world, id, true);
}

#[v2k_test_support::retail_test]
fn native_type54_model_override_uses_the_selected_header08_extent() {
    let mut world = World::load_with_spawn_edit(13, |level| {
        let spawn = level
            .entities
            .iter_mut()
            .find(|spawn| spawn.entity_type == 54)
            .unwrap();
        spawn.model_overrides = [145, 81, 145, 81];
    });
    let id = world.first(54);
    let entity = world
        .manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap();
    assert_eq!(
        entity.model_slots,
        [Some(145), Some(81), Some(145), Some(81)]
    );
    abort_type54(&mut world, id);
}

#[v2k_test_support::retail_test]
fn quiet_type62_later_world_abort_uses_native_construction() {
    let mut world = World::load(14);
    let id = world.first(62);
    let spawn = world
        .manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .authored_spawn_index;
    assert_eq!(spawn, Some(1));
    let observation = world.manager.main_base_abort_actor_observation(id).unwrap();
    world
        .manager
        .apply_main_base_abort_quiet_death(observation.lease, &mut world.fx)
        .expect("native Type62 quiet abort");
}

#[v2k_test_support::retail_test]
fn campaign_cargo_type74_miss_still_fail_closes() {
    let World {
        session,
        mut manager,
        mut fx,
        ..
    } = World::load(14);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_class0_actors(&manager);
    let mut notifications = GameplayNotifications::new();
    let extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let error = manager
        .restore_campaign_cargo_controller_state(
            CampaignCargoControllerState::from_packed(5, [0x7777_004a]),
            CampaignCargoRestoreContext {
                level: session.cache.level_desc().unwrap(),
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&extent),
                },
                retail_tick: 4793,
                scheduler: &mut scheduler,
                world_fx: &mut fx,
                notifications: &mut notifications,
            },
        )
        .unwrap_err();
    assert!(matches!(
        error,
        CampaignCargoRestoreError::UnsupportedConstructor { entity_type: 74 }
    ));
}

#[v2k_test_support::retail_test]
fn native_type54_nonzero_plus88_constructs_but_abort_fail_closes() {
    let mut world = World::load_with_spawn_edit(13, |level| {
        let spawn = level
            .entities
            .iter_mut()
            .find(|spawn| spawn.entity_type == 54)
            .unwrap();
        spawn.extra[8..12].copy_from_slice(&1_i32.to_le_bytes());
    });
    let id = world.first(54);
    let entity = world
        .manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap();
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::Class0Timer(_))
    ));
    assert_eq!(
        entity.main_base_type54_sea_delta_source,
        RetailRuntimeValue::Known(MainBaseType54SeaDeltaSource::ExplicitPreShiftWords(1))
    );
    let observation = world.manager.main_base_abort_actor_observation(id).unwrap();
    let error = world
        .manager
        .apply_main_base_abort_type54_death(
            observation.lease,
            world.session.cache.terrain().unwrap(),
            LEVEL_ONE_TYPE54_MODEL_EXTENT_RAW,
            MainBaseType54NetworkSession::SoloNetworkingDisabled,
            &mut world.fx,
        )
        .expect_err("explicit +0x88 remains unsupported");
    assert_eq!(
        error,
        MainBaseType54DeathBlock::SeaDeltaSourceUnsupported(RetailRuntimeValue::Known(
            MainBaseType54SeaDeltaSource::ExplicitPreShiftWords(1)
        ))
    );
}
