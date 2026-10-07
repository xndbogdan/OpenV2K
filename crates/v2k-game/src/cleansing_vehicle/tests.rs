use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    common_mover::sub_d::{construct_native_sub_d, SubDAllocationCounter},
    entity::{
        AuthoredWorldConstruction, CampaignCargoControllerState, CampaignCargoRestoreContext,
        EntityConstructionResources,
    },
    gameplay_notifications::GameplayNotifications,
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::WorldFx,
};
use v2k_formats::terrain::{TerrainCell, TerrainGrid};

pub(super) fn fixture(world: u32) -> (GameSession, EntityManager, WorldFx) {
    let path = v2k_test_support::retail_dir();
    assert!(path.join("PRELOAD.DAT").is_file(), "retail corpus required");
    let mut session = GameSession::init(&path).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(world, 1).unwrap();
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
            logical_world_index: world as i32 - 12,
            type_metadata: &metadata,
            player_arrival: Some(crate::entity::AuthoredPlayerArrival {
                position_raw: [8192, 0, 8192],
                heading_raw: 0,
            }),
            retail_tick: 1000,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
            },
        },
        &mut fx,
    )
    .unwrap();
    (session, manager, fx)
}

fn flat() -> TerrainGrid {
    TerrainGrid {
        header: [0; 5],
        cells: vec![
            TerrainCell {
                height: 0,
                attribute: 0,
                terrain_type: 0
            };
            256 * 256
        ],
    }
}

#[test]
fn infection_search_preserves_source_rng_order_divisions_and_wrapping() {
    let mut terrain = flat();
    let mut task = tasks::CleansingMovementTaskState::new([512, 77, 512]);
    terrain.cells[256 + 1].terrain_type = 0x10;
    let mut draws = [0, 0].into_iter();
    task.retarget([512, 77, 512], &terrain, &mut || {
        draws.next().expect("unexpected retention draw")
    });
    assert_eq!(task.private.target_position_raw, [256, 77, 256]);
    assert_eq!(draws.next(), None);
    let mut draws = [1].into_iter();
    task.retarget([3000, 9, 3000], &terrain, &mut || {
        draws.next().expect("infected target should remain")
    });
    assert_eq!(task.private.target_position_raw, [256, 77, 256]);
    assert_eq!(draws.next(), None);
    terrain.cells[257].terrain_type = 0;
    let mut count = 0;
    task.retarget([32760, 123, -32760], &terrain, &mut || {
        count += 1;
        0xffff
    });
    assert_eq!(count, 16);
    assert_eq!(task.private.target_position_raw, [-27788, 123, -27772]);
}

#[v2k_test_support::retail_test]
fn native_rover_constructor_draws_a_then_selector_then_both_task_suffixes() {
    let (session, _, _) = fixture(15);
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: None,
    };
    let mut manager = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        resources,
    );
    let id = manager.iter_all().find(|e| e.entity_type == 49).unwrap().id;
    let allocation = manager.main_base_abort_actor_observation(id).unwrap().lease;
    let entity = manager.entity_mut(id).unwrap();
    let spawn = &session.cache.level_desc().unwrap().entities[entity.authored_spawn_index.unwrap()];
    let terrain = session.cache.terrain().unwrap();
    let surface = crate::entity_initializer::constructor_surface_bits_at_tick(
        spawn.position_raw(),
        terrain,
        1000,
        true,
    )
    .unwrap();
    let sub_d = construct_native_sub_d(
        &mut SubDAllocationCounter::from_next_seed(0xd3),
        CLEANSING_VEHICLE_SUB_D,
    );
    let mut draws = [0x1000, 0, 0x2000, 0x3000].into_iter();
    publish_cleansing_vehicle(
        CleansingVehicleConstruction {
            entity,
            allocation,
            metadata: &metadata[49],
            preceding: &[],
            terrain,
            authored_position_raw: spawn.position_raw(),
            spawn_param_nonzero: spawn.param != 0,
            constructor_surface_bits: surface,
            sub_d,
        },
        &mut || draws.next().expect("unexpected constructor draw"),
    )
    .unwrap();
    assert_eq!(draws.next(), None);
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::CleansingLandscape(_))
    ));
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Tertiary),
        Some(ActorTaskRuntime::TerrainCleansing(_))
    ));
    let RetailRuntimeValue::Known(Some(a)) = entity.sub_a_propulsion_runtime else {
        panic!()
    };
    assert_eq!(a.target_speed_raw(), RetailRuntimeValue::Known(509));
    assert_eq!(
        entity
            .cleansing_vehicle_runtime
            .unwrap()
            .sub_d_owner
            .classifier_cache()
            .stagger_counter(),
        0xd3
    );
}

#[v2k_test_support::retail_test]
fn all_authored_rovers_get_native_construction_and_current_task_custody() {
    let mut total = 0;
    for world in [15, 21, 26, 31, 35, 39, 40] {
        let (_, manager, _) = fixture(world);
        for entity in manager.iter_all().filter(|e| e.entity_type == 49) {
            assert!(
                allocation_authenticates(&manager, entity.id),
                "world{world}"
            );
            CleansingVehicleOwner::adopt(&manager, entity.id).unwrap();
            total += 1;
        }
    }
    assert_eq!(total, 14);
}

#[v2k_test_support::retail_test]
fn campaign_missing_rover_constructs_then_attaches_with_saved_stamp() {
    let (session, mut manager, mut fx) = fixture(13);
    assert!(!manager.iter_all().any(|e| e.entity_type == 49));
    let mut scheduler = SpecializedActorTaskScheduler::new();
    manager
        .restore_campaign_cargo_controller_state(
            CampaignCargoControllerState::from_packed(3, [0x0c35_0031, 0]),
            CampaignCargoRestoreContext {
                level: session.cache.level_desc().unwrap(),
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
                },
                retail_tick: 1000,
                scheduler: &mut scheduler,
                world_fx: &mut fx,
                notifications: &mut GameplayNotifications::new(),
            },
        )
        .unwrap();
    let rover = manager.iter_all().find(|e| e.entity_type == 49).unwrap();
    assert_eq!(
        rover.construction_stamp_at_0xb4,
        RetailRuntimeValue::Known(0x0c35)
    );
    assert_eq!(rover.attached_to, manager.player().map(|p| p.id));
    assert!(allocation_authenticates(&manager, rover.id));
    CleansingVehicleOwner::adopt(&manager, rover.id).unwrap();
}

#[v2k_test_support::retail_test]
fn cleansing_rover_collect_drop_and_settle_retains_identity_and_republishes_tasks() {
    use crate::entity::{
        BeamCommand, BeamOutcome, CargoDropContext, LateTailMaterialiserFrame,
        PlayerCargoAttachmentFrame, PlayerCargoFrame,
    };
    let (session, mut manager, mut fx) = fixture(15);
    let id = manager.iter_all().find(|e| e.entity_type == 49).unwrap().id;
    let metadata = manager.type_runtime_metadata(49).unwrap().clone();
    let selection = tasks::select_with_base(false, &mut || 0).unwrap();
    let entity = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        panic!()
    };
    let context = previous
        .reselect_named_type_default(selection.program, 0, selection.program.initial_style)
        .unwrap();
    tasks::publish_selection(entity, &metadata, selection, context, &mut || 0).unwrap();
    let stamp = entity.construction_stamp_at_0xb4;
    let anchor = entity
        .cleansing_vehicle_runtime
        .unwrap()
        .immutable_anchor_raw;
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_cleansing_vehicle(&manager);
    let mut notifications = GameplayNotifications::new();
    let mut expected_rng = fx.fork_for_main_base_abort_transaction();
    manager
        .attach_player_cargo(
            id,
            PlayerCargoAttachmentFrame {
                scheduler: &mut scheduler,
                world_fx: &mut fx,
                notifications: &mut notifications,
                retail_tick: 1001,
            },
        )
        .unwrap();
    expected_rng.next_shared_retail_random_u16();
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_rng.next_shared_retail_random_u16()
    );
    let entity = manager.entity_mut(id).unwrap();
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::None)
    ));
    assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
    let [x, _, z] = entity.position;
    let floor = session.cache.terrain().unwrap().height_at(x + 2., z);
    manager.player_mut().unwrap().position = [x + 2., floor - 1., z + 400. / 256.];
    manager.queue_beam(BeamCommand::Drop);
    let mut proxy = None;
    for tick in 1002..1102 {
        let pass = manager.update_player_cargo(PlayerCargoFrame {
            elapsed_micros: 20_000,
            drop_context: Some(CargoDropContext {
                carrier_orientation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                terrain: session.cache.terrain().unwrap(),
            }),
            retail_tick: tick,
            scheduler: &mut scheduler,
            world_fx: &mut fx,
            notifications: &mut notifications,
        });
        assert!(pass.blocked.is_empty(), "{:?}", pass.blocked);
        if let Some(BeamOutcome::DropStarted { cargo_id, proxy_id }) = pass.beam {
            assert_eq!(cargo_id, id);
            proxy = Some(proxy_id);
            assert_eq!(
                manager
                    .entity_mut(id)
                    .unwrap()
                    .cleansing_vehicle_runtime
                    .unwrap()
                    .immutable_anchor_raw,
                anchor
            );
        }
        let blocked = manager.update_late_tail_materialisers(LateTailMaterialiserFrame {
            elapsed_micros: 20_000,
            terrain: session.cache.terrain().unwrap(),
            world_fx: &mut fx,
            scheduler: &mut scheduler,
            notifications: &mut notifications,
            retail_tick: tick,
        });
        assert!(blocked.is_empty(), "{blocked:?}");
        if proxy.is_some() && manager.entity_mut(id).unwrap().attached_to.is_none() {
            break;
        }
    }
    let proxy = proxy.expect("native Type93 drop");
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.attached_to, None);
    assert_eq!(entity.construction_stamp_at_0xb4, stamp);
    assert_eq!(
        entity
            .cleansing_vehicle_runtime
            .unwrap()
            .immutable_anchor_raw,
        entity.position_raw()
    );
    assert!(manager.iter_all().all(|e| e.id != proxy));
    assert!(scheduler.begin_cleansing_vehicle_external_mutation(&manager, id));
}
