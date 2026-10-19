//! World 30's radar through a real beam pickup, carry, drop and redeploy.

use super::actor::*;
use super::PortableRadarTickOutcome;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::{
        AuthoredPlayerArrival, AuthoredWorldConstruction, BeamCommand, BeamExpiry, BeamOutcome,
        CargoDropContext, CargoProxyEvent, Entity, EntityConstructionResources, EntityManager,
        LateTailMaterialiserFrame, PlayerCargoFrame,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
    },
    static_damage::StaticDamageScheduler,
    terrain_crater::terrain_aligned_actor_basis,
    world_fx::WorldFx,
};

struct World {
    session: GameSession,
    manager: EntityManager,
    scheduler: SpecializedActorTaskScheduler,
    fx: WorldFx,
    notifications: GameplayNotifications,
    tick: u32,
    radar: u32,
}

impl World {
    fn load() -> Self {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").is_file(),
            "normal-tier corpus required"
        );
        let mut session = GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(30, 1).unwrap();
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(id, _)| {
                EntityTypeRuntimeMetadata::from_section12(
                    session.cache.global_entity_type(id).unwrap(),
                )
            })
            .collect();
        let mut fx = WorldFx::new();
        let manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                logical_world_index: 30 - 12,
                level: session.cache.level_desc().unwrap(),
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
                retail_tick: 0,
            },
            &mut fx,
        )
        .unwrap();
        session
            .cache
            .initialize_level_terrain_radar(&mut || fx.next_shared_retail_random_u16())
            .unwrap();
        let radar = manager
            .iter_all()
            .find(|entity| entity.entity_type == ENTITY_TYPE)
            .unwrap()
            .id;
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_portable_radars(&manager), 1);
        Self {
            session,
            manager,
            scheduler,
            fx,
            notifications: GameplayNotifications::new(),
            tick: 1,
            radar,
        }
    }

    fn radar(&self) -> &Entity {
        self.manager
            .iter_all()
            .find(|entity| entity.id == self.radar)
            .unwrap()
    }

    fn k1(&self) -> u16 {
        self.radar().portable_radar_runtime.unwrap().sub_k1
    }

    fn primary(&self) -> crate::actor_task_owner::ActorTaskId {
        let radar = self.radar();
        let id = radar
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        assert!(matches!(
            radar.actor_tasks.task_state(id),
            Some(ActorTaskRuntime::PortableRadar)
        ));
        id
    }

    fn coverage(&self) -> Vec<u8> {
        self.session
            .cache
            .level_terrain_radar()
            .unwrap()
            .packed_coverage()
            .to_vec()
    }

    fn tick_actors(&mut self) -> Option<PortableRadarActorOutcome> {
        let pass = self.scheduler.tick(
            &mut self.manager,
            SpecializedActorTaskProductionFrame {
                world: SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase: GameplayNotificationPhase::Playing,
                resources: &mut self.session.cache,
                world_fx: &mut self.fx,
                static_damage: &mut StaticDamageScheduler::new(),
                elapsed_micros: 20_000,
                global_elapsed_micros: self.tick * 20_000,
                retail_tick: self.tick,
                main_base_abort_active: false,
            },
            &mut self.notifications,
        );
        assert!(pass.block.is_none(), "{:?}", pass.block);
        self.tick += 1;
        pass.outcomes.into_iter().find_map(|outcome| match outcome {
            SpecializedActorTaskProductionOutcome::PortableRadar(outcome)
                if outcome.entity_id() == self.radar =>
            {
                Some(outcome)
            }
            _ => None,
        })
    }

    fn deploy(&mut self) {
        for _ in 0..120 {
            if let Some(PortableRadarActorOutcome::Blocked { reason, .. }) = self.tick_actors() {
                panic!("{reason:?}");
            }
            if self.k1() == u16::MAX {
                return;
            }
        }
        panic!("the radar never deployed");
    }

    fn cargo_frame(&mut self) -> crate::entity::PlayerCargoFrameOutcome {
        let tick = self.tick;
        self.tick += 1;
        let Self {
            session,
            manager,
            scheduler,
            fx,
            notifications,
            ..
        } = self;
        session.cache.with_detached_terrain_radar(|cache, radar| {
            manager.update_player_cargo(PlayerCargoFrame {
                elapsed_micros: 20_000,
                beam_expiry: BeamExpiry::AtZero,
                drop_context: Some(CargoDropContext {
                    carrier_orientation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                    terrain: cache.terrain().unwrap(),
                }),
                retail_tick: tick,
                scheduler,
                world_fx: fx,
                notifications,
                radar: Some(radar),
            })
        })
    }

    fn place_player(&mut self, dx: f32) {
        let position = self.radar().position;
        let x = position[0] + dx;
        let z = position[2];
        let height = self.session.cache.terrain().unwrap().height_at(x, z);
        self.manager.player_mut().unwrap().position = [x + 0.25, height - 1., z + 400. / 256.];
    }
}

/// Picking up a deployed radar retires its Primary inside C690's C560, which
/// removes the footprint; it stays dormant while carried (405610 zeroes K1),
/// and the dropped radar redeploys at its new position.
#[v2k_test_support::retail_test]
fn a_carried_radar_retires_its_coverage_and_redeploys_where_it_lands() {
    let mut world = World::load();
    let before = world.coverage();
    world.deploy();
    let deployed = world.coverage();
    assert_ne!(deployed, before);
    let deployed_primary = world.primary();

    world.place_player(0.);
    world.manager.queue_beam(BeamCommand::Collect);
    let mut collected = false;
    for _ in 0..30 {
        let pass = world.cargo_frame();
        assert!(pass.blocked.is_empty(), "{:?}", pass.blocked);
        if pass.beam
            == Some(BeamOutcome::Collected {
                entity_id: world.radar,
            })
        {
            collected = true;
            break;
        }
    }
    assert!(collected, "the authored radar must be collected");
    assert_eq!(
        world.coverage(),
        before,
        "C560's 4055C0 removed the footprint"
    );
    assert_eq!(world.k1(), 0);
    assert_ne!(world.primary(), deployed_primary);
    let player = world.manager.player().unwrap().id;
    assert_eq!(world.radar().attached_to, Some(player));
    assert_eq!(world.radar().rotation_heading_pitch_roll_raw(), [0; 3]);
    assert!(world
        .scheduler
        .prepare_portable_radar_external_mutation(&world.manager, world.radar));

    // Carried: every callback that runs reports 405610's attachment branch.
    let mut carried = 0;
    for _ in 0..80 {
        match world.tick_actors() {
            Some(PortableRadarActorOutcome::Advanced {
                radar: Some(radar), ..
            }) => {
                assert_eq!(radar, PortableRadarTickOutcome::Carried);
                carried += 1;
            }
            Some(PortableRadarActorOutcome::Advanced { radar: None, .. })
            | Some(PortableRadarActorOutcome::Waiting { .. }) => {}
            other => panic!("{other:?}"),
        }
        assert_eq!(world.k1(), 0);
    }
    assert!(carried > 0);
    assert_eq!(world.coverage(), before);

    // Drop: the release from the player, then 08F00's attach to the proxy.
    world.place_player(8.);
    world.manager.queue_beam(BeamCommand::Drop);
    let mut proxy = None;
    for _ in 0..30 {
        let pass = world.cargo_frame();
        assert!(pass.blocked.is_empty(), "{:?}", pass.blocked);
        if let Some(BeamOutcome::DropStarted { cargo_id, proxy_id }) = pass.beam {
            assert_eq!(cargo_id, world.radar);
            proxy = Some(proxy_id);
            break;
        }
    }
    let proxy = proxy.expect("the drop constructs and attaches a Type93");
    assert_eq!(world.radar().attached_to, Some(proxy));
    world.manager.take_cargo_proxy_events();
    let mut released = None;
    for _ in 0..80 {
        let retail_tick = world.tick;
        let pass = world.cargo_frame();
        assert!(pass.blocked.is_empty(), "{:?}", pass.blocked);
        let blocked = world
            .manager
            .update_late_tail_materialisers(LateTailMaterialiserFrame {
                elapsed_micros: 20_000,
                terrain: world.session.cache.terrain().unwrap(),
                world_fx: &mut world.fx,
                scheduler: &mut world.scheduler,
                notifications: &mut world.notifications,
                retail_tick,
            });
        assert!(blocked.is_empty(), "{blocked:?}");
        for event in world.manager.take_cargo_proxy_events() {
            if let CargoProxyEvent::Released {
                cargo_id, position, ..
            } = event
            {
                assert_eq!(cargo_id, world.radar);
                released = Some(crate::entity::world_position_raw(position));
            }
        }
        if released.is_some() {
            break;
        }
    }
    let released = released.expect("the Type93 settles and releases the radar");
    let radar = world.radar();
    assert_eq!(radar.attached_to, None);
    assert_eq!(radar.position_raw(), released);
    // D1C0: 16AC0's terrain basis survives C560's Euler zeroing.
    assert_eq!(
        radar.physical_body_basis_q31,
        RetailRuntimeValue::Known(terrain_aligned_actor_basis(
            world.session.cache.terrain().unwrap(),
            released
        ))
    );
    assert_eq!(
        radar.collision.state_flags_at_0x08.masked(0x1000),
        RetailRuntimeValue::Known(0)
    );

    world.deploy();
    let redeployed = world.coverage();
    assert_ne!(redeployed, before);
    assert_ne!(redeployed, deployed, "the footprint follows the radar");
    assert_eq!(
        world.radar().position_raw(),
        released,
        "the fixed body stays put"
    );
}

fn restore_campaign_radar(
    session: &GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    scheduler: &mut SpecializedActorTaskScheduler,
    packed: u32,
) {
    manager
        .restore_campaign_cargo_controller_state(
            crate::entity::CampaignCargoControllerState::from_packed(3, [packed, 0]),
            crate::entity::CampaignCargoRestoreContext {
                level: session.cache.level_desc().unwrap(),
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
                },
                retail_tick: 1000,
                scheduler,
                world_fx: fx,
                notifications: &mut GameplayNotifications::new(),
            },
        )
        .unwrap();
}

/// 451C00: a miss constructs a zero-record radar, grounded at the origin
/// with the saved stamp; a match reuses the authored radar. Both attach.
#[v2k_test_support::retail_test]
fn campaign_cargo_restores_a_radar_by_miss_and_by_match() {
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(13);
    assert!(!manager.iter_all().any(|e| e.entity_type == ENTITY_TYPE));
    let mut scheduler = SpecializedActorTaskScheduler::new();
    restore_campaign_radar(&session, &mut manager, &mut fx, &mut scheduler, 0x0c35_0053);
    let radar = manager
        .iter_all()
        .find(|e| e.entity_type == ENTITY_TYPE)
        .unwrap();
    assert_eq!(
        radar.construction_stamp_at_0xb4,
        RetailRuntimeValue::Known(0x0c35)
    );
    assert_eq!(radar.attached_to, manager.player().map(|p| p.id));
    let terrain = session.cache.terrain().unwrap();
    assert_eq!(
        radar.position_raw(),
        [0, terrain.bilinear_height_raw(0, 0), 0]
    );
    assert_eq!(radar.portable_radar_runtime.unwrap().sub_k1, 0);
    assert!(scheduler.prepare_portable_radar_external_mutation(&manager, radar.id));

    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(30);
    let authored = manager
        .iter_all()
        .find(|e| e.entity_type == ENTITY_TYPE)
        .unwrap();
    let (id, RetailRuntimeValue::Known(stamp)) = (authored.id, authored.construction_stamp_at_0xb4)
    else {
        panic!("native stamp lineage");
    };
    let radars = |manager: &EntityManager| {
        manager
            .iter_all()
            .filter(|e| e.entity_type == ENTITY_TYPE)
            .count()
    };
    let count = radars(&manager);
    let style = session.cache.level_desc().unwrap().world_style;
    let mut scheduler = SpecializedActorTaskScheduler::new();
    restore_campaign_radar(
        &session,
        &mut manager,
        &mut fx,
        &mut scheduler,
        u32::from(stamp) << 16 | ENTITY_TYPE,
    );
    assert_eq!(
        radars(&manager),
        count,
        "a match constructs no radar (style {style})"
    );
    let radar = manager.iter_all().find(|e| e.id == id).unwrap();
    assert_eq!(radar.attached_to, manager.player().map(|p| p.id));
    assert!(scheduler.prepare_portable_radar_external_mutation(&manager, id));
}

/// 42E980 -> 14990: teardown's A860 removes a deployed footprint while the
/// old raster is resident; a partial deployment only resets K1.
#[v2k_test_support::retail_test]
fn world_teardown_retires_deployed_coverage() {
    let mut world = World::load();
    let before = world.coverage();
    for _ in 0..10 {
        world.tick_actors();
    }
    assert!(world.k1() != 0 && world.k1() != u16::MAX);
    let mut partial = World::load();
    for _ in 0..10 {
        partial.tick_actors();
    }
    let mut rng = partial.fx.fork_for_main_base_abort_transaction();
    assert_eq!(
        super::retire_world_radars(
            &mut partial.manager,
            &mut partial.session.cache,
            &mut partial.fx
        ),
        Ok(0)
    );
    assert_eq!(partial.k1(), 0);
    assert_eq!(
        partial.fx.next_shared_retail_random_u16(),
        rng.next_shared_retail_random_u16(),
        "a partial deployment draws nothing"
    );

    world.deploy();
    assert_ne!(world.coverage(), before);
    super::retire_world_radars(&mut world.manager, &mut world.session.cache, &mut world.fx)
        .unwrap();
    assert_eq!(world.coverage(), before);
    assert_eq!(world.k1(), 0);
    assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .all(|slot| world.radar().actor_tasks.task_in_slot(slot).is_none()));
}
