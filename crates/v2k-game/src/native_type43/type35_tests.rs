//! Type35 meteor launchers and the Type34 meteors their method-11 shots birth.

use super::*;
use crate::{
    entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT},
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    intro2_contacts::Intro2ContactFrame,
    native_entity_weapons::{
        contact::{resolve_entity_weapon_contacts, EntityWeaponContactFrame},
        meteor::{METEOR_ENTITY_TYPE, METEOR_PRIMARY},
        EntityWeaponKind,
    },
    player_hull::PlayerHull,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
    },
    static_damage::StaticDamageScheduler,
    world_fx::{ParticleEnvironment, TerrainCollisionContext, WorldFx},
};

fn type35_ids(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| entity.entity_type == TYPE35_ENTITY_TYPE)
        .map(|entity| entity.id)
        .collect()
}

#[v2k_test_support::retail_test]
fn twelve_launchers_publish_class7_search_without_a_model() {
    let mut total = 0;
    for level in 41..=47 {
        let (_session, manager, _fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level);
        let ids = type35_ids(&manager);
        for &id in &ids {
            let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
            assert_eq!(type43_row(entity), Some(Type43Row::Type35), "world{level}");
            assert_eq!(entity.model_slots, [None; 4]);
            assert!(manager_allocation_authenticates(&manager, id));
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                panic!("world{level}: context");
            };
            assert_eq!(context.active_style().style_address(), 0x004c_7a50);
            assert_eq!(
                entity.collision.default_state_flags_at_0xc8,
                RetailRuntimeValue::Known(TYPE35_DEFAULT_FLAGS)
            );
            assert!(Type43Owner::adopt(&manager, id).is_ok());
        }
        total += ids.len();
    }
    assert_eq!(total, 12);
}

/// World43's launcher at the (32256, -32256) corner with the player written
/// beside it, as the retail lab run did.
struct LauncherWorld {
    session: crate::session::GameSession,
    manager: EntityManager,
    fx: WorldFx,
    tasks: SpecializedActorTaskScheduler,
    hull: PlayerHull,
    static_damage: StaticDamageScheduler,
    notifications: GameplayNotifications,
    launcher: u32,
}

impl LauncherWorld {
    fn new() -> Self {
        let (session, mut manager, fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(43);
        let launcher = manager
            .iter_all()
            .find(|entity| {
                entity.entity_type == TYPE35_ENTITY_TYPE
                    && [entity.position_raw()[0], entity.position_raw()[2]] == [32256, -32256]
            })
            .unwrap()
            .id;
        manager
            .player_mut()
            .unwrap()
            .set_motion_raw([32256 - 2500, 2500, -32256 + 2500], [0; 3]);
        let hull = PlayerHull::default();
        manager.sync_player_hull_collision_state(&hull);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type43(&manager), type35_ids(&manager).len());
        Self {
            session,
            manager,
            fx,
            tasks,
            hull,
            static_damage: StaticDamageScheduler::new(),
            notifications: GameplayNotifications::new(),
            launcher,
        }
    }

    /// One update: the task pass, the late contact walk over the meteors and
    /// the presentation drain of every launcher.
    fn step(&mut self, t: u32) {
        let pass = self.tasks.tick(
            &mut self.manager,
            SpecializedActorTaskProductionFrame {
                world: SpecializedActorTaskWorld::Playing {
                    player_hull: &mut self.hull,
                    extra_lives: RetailRuntimeValue::Known(3),
                },
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
            match outcome {
                SpecializedActorTaskProductionOutcome::NativeType43(outcome) => assert!(
                    !matches!(
                        outcome,
                        Type43Outcome::Blocked { .. } | Type43Outcome::Pending { .. }
                    ),
                    "tick{t}: {outcome:?}"
                ),
                SpecializedActorTaskProductionOutcome::NativeWeapon(outcome) => {
                    assert!(
                        !format!("{outcome:?}").contains("Err"),
                        "tick{t}: {outcome:?}"
                    )
                }
                _ => {}
            }
        }
        let meteors: Vec<u32> = self
            .manager
            .iter_all()
            .filter(|entity| entity.entity_type == METEOR_ENTITY_TYPE)
            .map(|entity| entity.id)
            .collect();
        for id in meteors {
            let Some(owner) = self.tasks.native_weapon_owner(&self.manager, id) else {
                continue;
            };
            let outcome = resolve_entity_weapon_contacts(
                EntityWeaponContactFrame {
                    common: Intro2ContactFrame {
                        entities: &mut self.manager,
                        resources: &mut self.session.cache,
                        world_fx: &mut self.fx,
                        static_damage: &mut self.static_damage,
                        notifications: &mut self.notifications,
                        retail_tick: t,
                        actor_tasks: &mut self.tasks,
                    },
                    world: SpecializedActorTaskWorld::Playing {
                        player_hull: &mut self.hull,
                        extra_lives: RetailRuntimeValue::Known(3),
                    },
                },
                owner,
            );
            assert!(outcome.is_ok(), "tick{t} meteor{id}: {outcome:?}");
        }
        let environment = TerrainCollisionContext::from_current_level_cache(&self.session.cache)
            .map_or(ParticleEnvironment::Dry, ParticleEnvironment::Terrain);
        for id in type35_ids(&self.manager) {
            if let Some(drain) = crate::intro2_projectiles::drain_intro2_projectile_source(
                &mut self.manager,
                &mut self.fx,
                &self.session.cache,
                environment,
                t,
                id,
            ) {
                assert!(drain.is_ok(), "tick{t} launcher{id}: {drain:?}");
            }
        }
    }
}

/// Both corner launchers reach the player across the wrap, as in retail.
#[v2k_test_support::retail_test]
fn launchers_lob_meteors_that_land_and_explode() {
    let mut world = LauncherWorld::new();
    let launchers: Vec<(u32, [i16; 3])> = world
        .manager
        .iter_all()
        .filter(|entity| entity.entity_type == TYPE35_ENTITY_TYPE)
        .map(|entity| (entity.id, entity.position_raw()))
        .collect();
    let mut births = Vec::new();
    let mut explosions = 0;
    for t in 1..=600u32 {
        let before: Vec<u32> = world.manager.iter_all().map(|entity| entity.id).collect();
        world.step(t);
        for entity in world.manager.iter_all() {
            if entity.entity_type != METEOR_ENTITY_TYPE || before.contains(&entity.id) {
                continue;
            }
            // 44E770: the launcher's X/Z, Y+300, and an upward floor 0x960;
            // +60 is the launcher.
            let RetailRuntimeValue::Known(Some(source)) =
                entity.collision.recent_relation_id_at_0x60
            else {
                panic!("meteor {} relation", entity.id);
            };
            let (_, launcher) = launchers.iter().find(|(id, _)| *id == source).unwrap();
            assert_eq!(
                entity.position_raw(),
                [launcher[0], launcher[1].wrapping_add(300), launcher[2]]
            );
            assert!(entity.velocity_raw()[1] >= 0x960);
            assert_eq!(
                entity
                    .native_entity_weapon_runtime
                    .map(|runtime| runtime.kind),
                Some(EntityWeaponKind::Meteor)
            );
            births.push((entity.id, source));
        }
        explosions += world
            .manager
            .iter_all()
            .filter(|entity| {
                entity.entity_type == METEOR_ENTITY_TYPE
                    && entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
                        == RetailRuntimeValue::Known(DYING_STATE_BIT)
            })
            .count();
    }
    assert!(births.len() >= 2, "births {births:?}");
    assert!(
        births.iter().any(|&(_, source)| source == world.launcher),
        "births {births:?}"
    );
    assert!(explosions > 0, "no meteor landed");
}

#[test]
fn a_meteor_ignores_its_stationary_tag_until_its_lifetime() {
    use crate::{
        common_mover::type9_attitude::Type9BodyBasis,
        intro2_meteors::BoulderRollingTaskState,
        native_entity_weapons::grenade::{
            step_rolling_primary, GrenadePrimaryOwnerTransition, GrenadePrimaryStepRequest,
        },
    };
    let mut task = BoulderRollingTaskState::new([0; 3]);
    let request = GrenadePrimaryStepRequest {
        position_raw: [0; 3],
        velocity_raw: [0; 3],
        body_basis: Type9BodyBasis::from_angle_words(0, 0, 0),
        model_extent_raw: 256,
        elapsed_micros: 20_000,
        detailed: true,
        owner_transition_suppressed: false,
    };
    let mut tagged = false;
    for _ in 0..250 {
        let step = step_rolling_primary(&mut task, request, METEOR_PRIMARY).unwrap();
        tagged |= step.stationary_tagged;
        assert_eq!(step.owner_transition, None);
    }
    assert!(tagged);
    let step = step_rolling_primary(&mut task, request, METEOR_PRIMARY).unwrap();
    assert_eq!(
        step.owner_transition,
        Some(GrenadePrimaryOwnerTransition::StrictLifetimeExpired)
    );
}

#[v2k_test_support::retail_test]
fn ten_c10_republishes_search_on_the_dying_launcher() {
    let mut world = LauncherWorld::new();
    let metadata = world
        .manager
        .type_runtime_metadata(TYPE35_ENTITY_TYPE)
        .cloned()
        .unwrap();
    let id = world.launcher;
    let next_word = world
        .fx
        .fork_for_main_base_abort_transaction()
        .next_shared_retail_random_u16();
    let entity = world.manager.entity_mut(id).unwrap();
    let published = publish_type35_standard_death(entity, &metadata).unwrap();
    assert!(published.is_some_and(|publication| !publication.initializer_fallback));
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
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!("context");
    };
    assert_eq!(context.active_style().style_address(), 0x004c_7a50);
    // 25660 draws no word.
    assert_eq!(
        world
            .fx
            .fork_for_main_base_abort_transaction()
            .next_shared_retail_random_u16(),
        next_word
    );
    // A second 10C10 returns at the dying bit.
    let entity = world.manager.entity_mut(id).unwrap();
    assert_eq!(publish_type35_standard_death(entity, &metadata), Ok(None));
    // The dying launcher keeps searching and firing.
    world
        .tasks
        .register_type43(Type43Owner::adopt(&world.manager, id).unwrap());
    let mut births = 0;
    for t in 1..=300u32 {
        let before = world.manager.iter_all().count();
        world.step(t);
        births += world.manager.iter_all().count().saturating_sub(before);
    }
    assert!(births > 0);
}
