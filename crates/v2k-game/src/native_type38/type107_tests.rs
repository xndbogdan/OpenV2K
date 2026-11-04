//! Type107/Type121 mobile turrets: Search on the ground host, class29's
//! gun-turret task when a baddie or base is near, and the class49 terminal.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    intro2_contacts::Intro2ContactFrame,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskProductionPass, SpecializedActorTaskScheduler,
        SpecializedActorTaskWorld,
    },
    static_damage::StaticDamageScheduler,
    world_fx::{ParticleEnvironment, TerrainCollisionContext},
};

const TURRET_STYLE: u32 = 0x004c_8230;

fn turrets(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| matches!(entity.entity_type, 107 | 121))
        .map(|entity| entity.id)
        .collect()
}

fn entity(manager: &EntityManager, id: u32) -> &Entity {
    manager.iter_all().find(|entity| entity.id == id).unwrap()
}

fn style(manager: &EntityManager, id: u32) -> u32 {
    let RetailRuntimeValue::Known(Some(context)) = entity(manager, id).current_behavior_context
    else {
        panic!("context");
    };
    context.active_style().style_address()
}

struct World {
    session: crate::session::GameSession,
    manager: EntityManager,
    fx: WorldFx,
    tasks: SpecializedActorTaskScheduler,
    static_damage: StaticDamageScheduler,
    notifications: GameplayNotifications,
}

impl World {
    fn new(level: u32) -> Self {
        let (session, mut manager, fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level);
        for id in turrets(&manager) {
            let entity = manager.entity_mut(id).unwrap();
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(0x0202_8000, 0x0202_8000);
            entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        }
        let mut tasks = SpecializedActorTaskScheduler::new();
        tasks
            .adopt_fresh_level1_type9_selected(&mut manager)
            .unwrap();
        tasks.adopt_fresh_level1_type17_follow_beacons(&manager);
        tasks
            .adopt_fresh_level1_type47_scheduler(&mut manager)
            .unwrap();
        tasks.adopt_intro2_type13_search_attack(&manager);
        tasks.adopt_intro2_type26(&manager);
        tasks.adopt_intro2_type47_guards(&manager);
        tasks.adopt_intro2_flyers(&manager);
        tasks.adopt_intro2_type53(&manager);
        tasks.adopt_type122(&manager);
        tasks.adopt_type18(&manager);
        tasks.adopt_type28(&manager);
        tasks.adopt_type76_family(&manager);
        tasks.adopt_type30(&manager);
        tasks.adopt_rolling_boulders(&manager);
        tasks.adopt_type40(&manager);
        tasks.adopt_type56(&manager);
        tasks.adopt_type43(&manager);
        tasks.adopt_type38_family(&manager);
        tasks.adopt_type12_family(&manager);
        tasks.adopt_type31_family(&manager);
        tasks.adopt_shared_fish(&manager);
        tasks.adopt_cleansing_vehicle(&manager);
        tasks.adopt_intro2_type16(&manager);
        tasks.adopt_intro2_type58(&manager);
        tasks.adopt_intro2_type94(&manager);
        tasks.adopt_intro2_type66(&manager);
        tasks.adopt_class0_actors(&manager);
        tasks.adopt_portable_radars(&manager);
        tasks.adopt_main_base(&manager);
        tasks.adopt_intro2_type10(&manager);
        tasks.adopt_intro2_type57(&manager);
        tasks.adopt_intro2_gun_turret(&manager);
        tasks.adopt_intro2_type17(&manager);
        tasks.adopt_intro2_type8(&mut manager);
        tasks.adopt_intro2_type9(&mut manager);
        tasks.adopt_native_type123(&mut manager);
        tasks.adopt_native_type86(&mut manager);
        tasks.adopt_intro2_meteors(&manager);
        Self {
            session,
            manager,
            fx,
            tasks,
            static_damage: StaticDamageScheduler::new(),
            notifications: GameplayNotifications::new(),
        }
    }
    fn tick(&mut self, t: u32) -> SpecializedActorTaskProductionPass {
        let pass = self.tasks.tick(
            &mut self.manager,
            SpecializedActorTaskProductionFrame {
                world: SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase: GameplayNotificationPhase::Playing,
                resources: &mut self.session.cache,
                world_fx: &mut self.fx,
                static_damage: &mut self.static_damage,
                elapsed_micros: 20_000,
                global_elapsed_micros: t * 20_000,
                retail_tick: t,
                main_base_abort_active: false,
            },
            &mut self.notifications,
        );
        assert!(pass.block.is_none(), "tick{t}: {:?}", pass.block);
        for outcome in &pass.outcomes {
            if let SpecializedActorTaskProductionOutcome::NativeType38Family(outcome) = outcome {
                assert!(
                    !matches!(
                        outcome,
                        Type38Outcome::Blocked { .. } | Type38Outcome::Pending { .. }
                    ),
                    "tick{t}: {outcome:?}"
                );
            }
        }
        pass
    }
    fn contact(&mut self, t: u32, id: u32) {
        let outcome = contact::resolve_type38_family_static_contact(
            &mut Intro2ContactFrame {
                entities: &mut self.manager,
                resources: &mut self.session.cache,
                world_fx: &mut self.fx,
                static_damage: &mut self.static_damage,
                notifications: &mut self.notifications,
                retail_tick: t,
                actor_tasks: &mut self.tasks,
            },
            id,
            None,
        );
        assert!(
            !matches!(outcome, contact::Type38ContactOutcome::Blocked { .. }),
            "tick{t} id{id}: {outcome:?}"
        );
    }
    /// 44E770 for every queued command: the particle classes born.
    fn drain(&mut self, t: u32, id: u32) -> Vec<u8> {
        let environment = TerrainCollisionContext::from_current_level_cache(&self.session.cache)
            .map_or(ParticleEnvironment::Dry, ParticleEnvironment::Terrain);
        aim::drain_type38_family_shots(&mut self.manager, &mut self.fx, id, environment, t)
            .unwrap_or_else(|error| panic!("tick{t}: {error:?}"))
            .materialized_particle_classes
    }
    /// The player held `offset` from `id`, at rest.
    fn hold_player_near(&mut self, id: u32, offset: [i16; 3]) -> u32 {
        let player = self.manager.player().unwrap().id;
        let at = entity(&self.manager, id).position_raw();
        let held = std::array::from_fn(|axis| at[axis].wrapping_add(offset[axis]));
        let body = self.manager.entity_mut(player).unwrap();
        body.set_motion_raw(held, [0; 3]);
        // 408770 only weighs candidates with state 0x8000.
        body.collision.state_flags_at_0x08.overwrite(0x8000, 0x8000);
        player
    }
}

#[v2k_test_support::retail_test]
fn type107_and_type121_authenticate_their_canonical_metadata() {
    let (session, _) = crate::native_type122::construction_tests::fixture();
    for row in [Type38Row::Type107, Type38Row::Type121] {
        let metadata = EntityTypeRuntimeMetadata::from_section12(
            session
                .cache
                .global_entity_type(row.entity_type() as usize)
                .unwrap(),
        );
        authenticate_metadata(row, &metadata).unwrap();
    }
}

/// Worlds 26/31/40: each Type121 has a base in range and turrets at birth;
/// world 31's Type107s precede their base in the authored list and search.
#[v2k_test_support::retail_test]
fn mobile_turret_births_search_or_turret_with_an_l_only_bank() {
    for (level, expected) in [
        (26, &[(121, TURRET_STYLE)][..]),
        (
            31,
            &[(107, 0x004c_7a50), (107, 0x004c_7a50), (121, TURRET_STYLE)][..],
        ),
        (40, &[(121, TURRET_STYLE)][..]),
    ] {
        let (_, manager, _) = crate::intro2_gun_turret::authored_tests::fixture(level);
        let ids = turrets(&manager);
        let born: Vec<_> = ids
            .iter()
            .map(|&id| (entity(&manager, id).entity_type, style(&manager, id)))
            .collect();
        assert_eq!(born, expected, "world{level}");
        for &id in &ids {
            let body = entity(&manager, id);
            let row = type38_row(body).unwrap();
            assert!(row.turrets());
            assert_eq!(body.capability_flags, 0x1004);
            assert_eq!(
                body.sub_h_external_frame_runtime,
                RetailRuntimeValue::Known(None)
            );
            let runtime = body.native_type38_runtime.as_ref().unwrap();
            assert_eq!(runtime.sub_e_runtime.joint_bindings, [Some(3), Some(4)]);
            assert_eq!(
                runtime.sub_e_runtime.projectile_method,
                u32::from(row.emitter().projectile_method)
            );
            assert_eq!(runtime.kl_components.sub_l_output_selectors(), Some([1, 2]));
            if style(&manager, id) == TURRET_STYLE {
                // D190: no Primary or Secondary; 408BD0 in Tertiary.
                assert!(body.actor_task_state(ActorTaskSlot::Primary).is_none());
                assert!(body.actor_task_state(ActorTaskSlot::Secondary).is_none());
                assert!(matches!(
                    body.actor_task_state(ActorTaskSlot::Tertiary),
                    Some(ActorTaskRuntime::Intro2GunTurret(_))
                ));
            }
            assert!(Type38FamilyOwner::adopt(&manager, id).is_ok());
        }
    }
}

/// Ten seconds under the scheduler and the late static walk: the turrets
/// hold their X/Z, the searchers move.
#[v2k_test_support::retail_test]
fn mobile_turrets_run_ten_seconds_with_late_static_contact() {
    for level in [26, 31, 40] {
        let mut world = World::new(level);
        let ids = turrets(&world.manager);
        let start: Vec<_> = ids
            .iter()
            .map(|&id| {
                (
                    style(&world.manager, id),
                    entity(&world.manager, id).position_raw(),
                )
            })
            .collect();
        for t in 1..=500u32 {
            world.tick(t);
            for &id in &ids {
                world.contact(t, id);
            }
        }
        for (&id, (born_style, at)) in ids.iter().zip(start) {
            assert!(Type38FamilyOwner::adopt(&world.manager, id).is_ok());
            let now = entity(&world.manager, id).position_raw();
            if born_style == TURRET_STYLE && style(&world.manager, id) == TURRET_STYLE {
                assert_eq!([now[0], now[2]], [at[0], at[2]], "world{level} id{id}");
            } else {
                assert_ne!(now, at, "world{level} id{id}");
            }
        }
    }
}

/// World 40's Type121 turrets at a baddie (capability 8, 408770's top
/// priority): L tracks it and 425160 fires method13 class56 bolts, each
/// followed by E+17's method15 class32 auxiliary; 24EE0 flags bank word 4.
#[v2k_test_support::retail_test]
fn a_class29_type121_fires_bolts_and_auxiliaries_at_a_baddie() {
    let mut world = World::new(40);
    let turret = turrets(&world.manager)[0];
    assert_eq!(style(&world.manager, turret), TURRET_STYLE);
    let baddie = world
        .manager
        .iter_all()
        .find(|entity| entity.capability_flags == 8 && entity.id != turret)
        .unwrap()
        .id;
    let at = entity(&world.manager, turret).position_raw();
    let held = [at[0].wrapping_add(1500), at[1], at[2]];
    let mut born = Vec::new();
    for t in 1..=500u32 {
        let body = world.manager.entity_mut(baddie).unwrap();
        body.set_motion_raw(held, [0; 3]);
        body.collision.state_flags_at_0x08.overwrite(0x8000, 0x8000);
        world.tick(t);
        born.extend(world.drain(t, turret));
        if born.len() >= 4 {
            break;
        }
    }
    assert!(born.len() >= 4, "the turret fires: {born:?}");
    for pair in born.chunks(2) {
        assert!(matches!(pair[0], 56 | 82), "{born:?}");
        assert_eq!(pair[1], 32, "{born:?}");
    }
    let body = entity(&world.manager, turret);
    let Some(ActorTaskRuntime::Intro2GunTurret(task)) =
        body.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!("turret task");
    };
    assert_eq!(task.target_handle(), baddie);
    let kl = &body.native_type38_runtime.as_ref().unwrap().kl_components;
    assert_eq!(kl.model_word(4), Some(-1));
    assert_eq!(
        kl.model_word(1),
        Some((task.angles_raw()[0] as i16).wrapping_neg())
    );
    assert_eq!(body.velocity_raw(), [0; 3]);
}

/// World 31's searching Type107 fires method18 class2 shots through 24650,
/// each followed by the method15 auxiliary.
#[v2k_test_support::retail_test]
fn a_searching_type107_fires_method18_shots_and_auxiliaries() {
    let mut world = World::new(31);
    let searcher = turrets(&world.manager)[0];
    assert_eq!(entity(&world.manager, searcher).entity_type, 107);
    let mut born = Vec::new();
    for t in 1..=1500u32 {
        world.hold_player_near(searcher, [1200, 0, 0]);
        world.tick(t);
        born.extend(world.drain(t, searcher));
        if born.len() >= 4 {
            break;
        }
    }
    assert!(born.len() >= 4, "the searcher fires: {born:?}");
    for pair in born.chunks(2) {
        assert_eq!(pair, [2, 32], "{born:?}");
    }
}

/// A lethal hit runs the class49 terminal: BAF0, then BD20's Type60 ring.
#[v2k_test_support::retail_test]
fn a_dead_mobile_turret_takes_the_class49_terminal() {
    use crate::{
        damage::DamagePacket,
        player_hull::PlayerHull,
        shared_actor_impact::{apply_playing_actor_particle_hit, PlayingActorImpactFrame},
        world_fx::{BallisticDamageRequest, ParticleEntityImpact},
    };
    let mut world = World::new(40);
    let turret = turrets(&world.manager)[0];
    let player = world.manager.player().unwrap().id;
    let rings_before = world
        .manager
        .iter_all()
        .filter(|entity| entity.entity_type == 60)
        .count();
    let mut hull = PlayerHull::default();
    for t in 1..=10u32 {
        world.tick(t);
        if crate::class49_death::finished_terminal_hit_authenticates(&world.manager, turret) {
            break;
        }
        let outcome = apply_playing_actor_particle_hit(
            PlayingActorImpactFrame {
                extra_lives: RetailRuntimeValue::Known(3),
                resources: &mut world.session.cache,
                entities: &mut world.manager,
                world_fx: &mut world.fx,
                scheduler: &mut world.tasks,
                notifications: &mut world.notifications,
                static_damage: &mut world.static_damage,
                player_hull: &mut hull,
                retail_tick: t,
            },
            ParticleEntityImpact {
                no_packet: false,
                source_particle_class: 16,
                impact_position_argument_va: 0,
                target_entity_id: turret,
                position_world: [0.0; 3],
                velocity_raw: [0, 0, 0],
                damage: Some(BallisticDamageRequest {
                    packet: DamagePacket {
                        channels: [1, 0],
                        amounts_raw: [16_000, 0],
                    },
                    source_entity_type_at_birth: Some(46),
                    source_owner_id: Some(player),
                }),
            },
        );
        assert!(
            !matches!(
                outcome,
                Some(
                    crate::shared_actor_impact::SharedActorImpactOutcome::Type38Family(
                        impact::Type38ImpactOutcome::Blocked { .. }
                    )
                )
            ),
            "t{t}: {outcome:?}"
        );
    }
    assert!(crate::class49_death::finished_terminal_hit_authenticates(
        &world.manager,
        turret
    ));
    // BD20's ring, and the rings of any class49 neighbour its radial kills.
    assert!(
        world
            .manager
            .iter_all()
            .filter(|entity| entity.entity_type == 60)
            .count()
            > rings_before
    );
    assert_eq!(style(&world.manager, turret), 0x004c_71e0);
}
