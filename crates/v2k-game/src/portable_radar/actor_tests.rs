//! Native Type83 births and deployment over the authored worlds.

use super::actor::*;
use super::PortableRadarTickOutcome;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::{Entity, EntityManager},
    entity_collision_state::{RetailRuntimeValue, SURFACE_STATE_MASK},
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
    },
    static_damage::StaticDamageScheduler,
};

/// Worlds with authored Type83 rows, and their birth counts (14 in all).
const WORLDS: [(u32, usize); 6] = [(30, 1), (39, 1), (40, 3), (42, 4), (46, 3), (47, 2)];

fn radar_ids(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| entity.entity_type == ENTITY_TYPE)
        .map(|entity| entity.id)
        .collect()
}

fn entity(manager: &EntityManager, id: u32) -> &Entity {
    manager.iter_all().find(|entity| entity.id == id).unwrap()
}

#[v2k_test_support::retail_test]
fn portable_radar_births_install_the_c560_primary() {
    let mut births = 0;
    for (level, expected) in WORLDS {
        let (session, manager, _) = crate::intro2_gun_turret::authored_tests::fixture(level);
        let terrain = session.cache.terrain().unwrap();
        let ids = radar_ids(&manager);
        assert_eq!(ids.len(), expected, "world{level}");
        for &id in &ids {
            births += 1;
            let entity = entity(&manager, id);
            assert!(manager_allocation_authenticates(&manager, id));
            assert_eq!(entity.portable_radar_runtime.unwrap().sub_k1, 0);
            // D4A0 grounds at the authored X/Z without a model offset.
            let [x, y, z] = entity.position_raw();
            assert_eq!(y, terrain.bilinear_height_raw(x, z), "world{level} id{id}");
            // C560 zeroes Euler; the birth suffix builds the identity basis.
            assert_eq!(entity.rotation_heading_pitch_roll_raw(), [0; 3]);
            assert_eq!(
                entity.physical_body_basis_q31,
                RetailRuntimeValue::Known(
                    crate::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(0, 0, 0)
                )
            );
            assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::PortableRadar)
            ));
            assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
            assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
            // Retail's world-30 radar at its first 12DA0 visit, less the
            // detail bits 411400 publishes each frame and the birth's surface
            // pair: no master motion 0x40000, so it holds its birth pose.
            let mask = !(0x0600_0000 | SURFACE_STATE_MASK);
            assert_eq!(
                entity.collision.state_flags_at_0x08.masked(mask),
                RetailRuntimeValue::Known(0x0822_8805 & mask),
                "world{level} id{id}"
            );
            PortableRadarOwner::adopt(&manager, id).unwrap();
        }
    }
    assert_eq!(births, 14);
}

#[v2k_test_support::retail_test]
fn world30_radar_deploys_after_53_ticks_of_callbacks_with_retail_raster_draws() {
    let (mut session, mut manager, mut fx) = crate::intro2_gun_turret::authored_tests::fixture(30);
    session
        .cache
        .initialize_level_terrain_radar(&mut || fx.next_shared_retail_random_u16())
        .unwrap();
    let [id] = radar_ids(&manager)[..] else {
        panic!("one world-30 radar");
    };
    let birth = entity(&manager, id).position_raw();
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_portable_radars(&manager), 1);
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut callback_micros = 0u32;
    let mut deployed = None;
    for tick in 1..=200u32 {
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
                global_elapsed_micros: tick * 20_000,
                retail_tick: tick,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        assert!(pass.block.is_none(), "tick{tick}: {:?}", pass.block);
        for outcome in &pass.outcomes {
            let SpecializedActorTaskProductionOutcome::PortableRadar(outcome) = outcome else {
                continue;
            };
            match *outcome {
                PortableRadarActorOutcome::Advanced {
                    callback_elapsed_micros,
                    radar: Some(radar),
                    ..
                } => {
                    callback_micros += callback_elapsed_micros;
                    match radar {
                        PortableRadarTickOutcome::Deploying => {
                            assert!(callback_micros < 53 * 20_000, "tick{tick}");
                        }
                        PortableRadarTickOutcome::Deployed { random_draws } => {
                            assert!(deployed.is_none());
                            // 1250 a tick saturates in the 53rd tick's time.
                            assert!(callback_micros >= 53 * 20_000, "tick{tick}");
                            deployed = Some((tick, random_draws));
                        }
                        PortableRadarTickOutcome::AlreadyDeployed => {
                            assert!(deployed.is_some(), "tick{tick}");
                        }
                        PortableRadarTickOutcome::Carried => panic!("tick{tick}: carried"),
                    }
                }
                PortableRadarActorOutcome::Advanced { radar: None, .. }
                | PortableRadarActorOutcome::Waiting { .. } => {}
                other => panic!("tick{tick}: {other:?}"),
            }
        }
    }
    // Retail's lab run deployed world 30's radar 54 ticks after arrival, and
    // 44A5C0 drew 8307 words for the cells it left covered.
    let (tick, draws) = deployed.expect("deployment");
    assert_eq!(draws, 8307, "tick{tick}");
    let entity = entity(&manager, id);
    assert_eq!(entity.portable_radar_runtime.unwrap().sub_k1, u16::MAX);
    // 40D320: selector 0 is the 50-Hz clock, selector 1 the deployment word.
    let vars = entity.presentation_anim_vars(1234);
    assert_eq!(vars.dynamic[..2], [1234, 0xFFFF]);
    assert_eq!(entity.position_raw(), birth, "no master motion");
}

fn tick_radars(
    tasks: &mut SpecializedActorTaskScheduler,
    manager: &mut EntityManager,
    session: &mut crate::session::GameSession,
    fx: &mut crate::world_fx::WorldFx,
    tick: u32,
) -> Vec<PortableRadarActorOutcome> {
    let pass = tasks.tick(
        manager,
        SpecializedActorTaskProductionFrame {
            world: SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase: GameplayNotificationPhase::Playing,
            resources: &mut session.cache,
            world_fx: fx,
            static_damage: &mut StaticDamageScheduler::new(),
            elapsed_micros: 20_000,
            global_elapsed_micros: tick * 20_000,
            retail_tick: tick,
            main_base_abort_active: false,
        },
        &mut GameplayNotifications::new(),
    );
    assert!(pass.block.is_none(), "tick{tick}: {:?}", pass.block);
    pass.outcomes
        .into_iter()
        .filter_map(|outcome| match outcome {
            SpecializedActorTaskProductionOutcome::PortableRadar(outcome) => Some(outcome),
            _ => None,
        })
        .collect()
}

fn lethal_hit(id: u32) -> crate::world_fx::ParticleEntityImpact {
    crate::world_fx::ParticleEntityImpact {
        source_particle_class: 16,
        impact_position_argument_va: 0,
        target_entity_id: id,
        position_world: [0.0; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(crate::world_fx::BallisticDamageRequest {
            packet: crate::damage::DamagePacket {
                channels: [2, 0],
                amounts_raw: [100_000, 0],
            },
            source_entity_type_at_birth: Some(34),
            source_owner_id: Some(35),
        }),
    }
}

/// BD20 keeps the footprint through BAF0's blast and radial; A860's 4055C0
/// then removes it at the current position before the Type60 ring.
#[v2k_test_support::retail_test]
fn a_lethal_hit_removes_deployed_coverage_inside_class49() {
    for deploy in [false, true] {
        let (mut session, mut manager, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(30);
        manager.cleanup_pending_actor_deferred_destroys();
        session
            .cache
            .initialize_level_terrain_radar(&mut || fx.next_shared_retail_random_u16())
            .unwrap();
        let before = session
            .cache
            .level_terrain_radar()
            .unwrap()
            .packed_coverage()
            .to_vec();
        let [id] = radar_ids(&manager)[..] else {
            panic!("one world-30 radar");
        };
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_portable_radars(&manager), 1);
        let ticks = if deploy { 120 } else { 10 };
        for tick in 1..=ticks {
            tick_radars(&mut tasks, &mut manager, &mut session, &mut fx, tick);
        }
        let k1 = entity(&manager, id).portable_radar_runtime.unwrap().sub_k1;
        assert_eq!(k1 == u16::MAX, deploy);
        assert_eq!(
            session
                .cache
                .level_terrain_radar()
                .unwrap()
                .packed_coverage()
                != before,
            deploy
        );
        let rings = manager
            .iter_all()
            .filter(|entity| entity.entity_type == 60)
            .count();
        let outcome = crate::shared_actor_impact::apply_playing_actor_particle_hit(
            crate::shared_actor_impact::PlayingActorImpactFrame {
                extra_lives: RetailRuntimeValue::Known(3),
                resources: &mut session.cache,
                entities: &mut manager,
                world_fx: &mut fx,
                scheduler: &mut tasks,
                notifications: &mut GameplayNotifications::new(),
                static_damage: &mut StaticDamageScheduler::new(),
                player_hull: &mut crate::player_hull::PlayerHull::default(),
                retail_tick: ticks + 1,
            },
            lethal_hit(id),
        );
        assert!(
            matches!(
                &outcome,
                Some(
                    crate::shared_actor_impact::SharedActorImpactOutcome::PortableRadar(
                        super::impact::PortableRadarImpactOutcome::Applied(_)
                    )
                )
            ),
            "{outcome:?}"
        );
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &manager, id
        ));
        assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
        let radar = entity(&manager, id);
        assert_eq!(radar.portable_radar_runtime.unwrap().sub_k1, 0);
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| radar.actor_tasks.task_in_slot(slot).is_none()));
        assert_eq!(
            session
                .cache
                .level_terrain_radar()
                .unwrap()
                .packed_coverage(),
            &before[..],
            "deploy={deploy}"
        );
        assert_eq!(
            manager
                .iter_all()
                .filter(|entity| entity.entity_type == 60)
                .count(),
            rings + 1
        );
        // The living owner retires; the finished corpse is never revisited.
        assert!(
            tick_radars(&mut tasks, &mut manager, &mut session, &mut fx, ticks + 2)
                .iter()
                .all(|outcome| !matches!(outcome, PortableRadarActorOutcome::Advanced { .. }))
        );
    }
}
