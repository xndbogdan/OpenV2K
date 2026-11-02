//! World 47's Type2 tractor-beam walkers and their Type69 beams.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    intro2_contacts::Intro2ContactFrame,
    native_type69::{self, Type69Error, Type69Outcome},
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
    },
    static_damage::StaticDamageScheduler,
};

fn ids(manager: &EntityManager, entity_type: u32) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| entity.entity_type == entity_type)
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

fn activate(manager: &mut EntityManager, ids: &[u32]) {
    for &id in ids {
        let entity = manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x0202_8000, 0x0202_8000);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    }
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
    fn new() -> Self {
        let (session, mut manager, fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(47);
        let walkers = ids(&manager, 2);
        activate(&mut manager, &walkers);
        let mut tasks = SpecializedActorTaskScheduler::new();
        tasks.adopt_type38_family(&manager);
        Self {
            session,
            manager,
            fx,
            tasks,
            static_damage: StaticDamageScheduler::new(),
            notifications: GameplayNotifications::new(),
        }
    }
    fn tick(
        &mut self,
        t: u32,
    ) -> crate::specialized_actor_task_production::SpecializedActorTaskProductionPass {
        self.tasks.tick(
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
        )
    }
    /// A beam from the first Type2, at `position`, at rest.
    fn beam_at(&mut self, position_raw: [i16; 3]) -> u32 {
        let shooter = ids(&self.manager, 2)[0];
        let owner = self
            .manager
            .construct_native_type69(
                crate::entity::Type69BirthRequest {
                    shooter_id: shooter,
                    position_raw,
                    velocity_raw: [0; 3],
                    rotation_raw: [0; 3],
                },
                &self.session.cache,
                &mut self.fx,
                600,
            )
            .unwrap();
        self.manager.queue_born_native_type69(owner);
        activate(&mut self.manager, &[owner.entity_id()]);
        owner.entity_id()
    }
    fn beam_outcome(
        pass: &crate::specialized_actor_task_production::SpecializedActorTaskProductionPass,
        id: u32,
    ) -> Option<Type69Outcome> {
        pass.outcomes.iter().find_map(|outcome| match outcome {
            SpecializedActorTaskProductionOutcome::NativeType69(outcome)
                if outcome.entity_id() == id =>
            {
                Some(*outcome)
            }
            _ => None,
        })
    }
}

#[v2k_test_support::retail_test]
fn type2_and_type69_authenticate_their_canonical_metadata() {
    let (session, _) = crate::native_type122::construction_tests::fixture();
    let metadata = |id: usize| {
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
    };
    authenticate_metadata(Type38Row::Type2, &metadata(2)).unwrap();
    native_type69::authenticate_metadata(&metadata(69)).unwrap();
}

#[v2k_test_support::retail_test]
fn world47s_type2s_publish_defecate_or_aimless_with_a_k_only_bank() {
    let (_, manager, _) = crate::intro2_gun_turret::authored_tests::fixture(47);
    let walkers = ids(&manager, 2);
    assert_eq!(walkers.len(), 2);
    for &id in &walkers {
        let walker = entity(&manager, id);
        assert_eq!(type38_row(walker), Some(Type38Row::Type2));
        assert!(matches!(style(&manager, id), 0x004c_7e88 | 0x004c_7930));
        let runtime = walker.native_type38_runtime.as_ref().unwrap();
        assert_eq!(runtime.sub_e_runtime.projectile_method, 25);
        assert!(Type38FamilyOwner::adopt(&manager, id).is_ok());
    }
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert!(tasks.adopt_type38_family(&manager) >= walkers.len());
}

/// Ten seconds under the scheduler and the late static walk.
#[v2k_test_support::retail_test]
fn type2s_run_ten_seconds_with_late_static_contact() {
    let mut world = World::new();
    let walkers = ids(&world.manager, 2);
    let start: Vec<_> = walkers
        .iter()
        .map(|&id| entity(&world.manager, id).position_raw())
        .collect();
    for t in 1..=500u32 {
        let pass = world.tick(t);
        assert!(pass.block.is_none(), "tick{t}: {:?}", pass.block);
        for outcome in &pass.outcomes {
            if let SpecializedActorTaskProductionOutcome::NativeType38Family(outcome) = outcome {
                if walkers.contains(&outcome.entity_id()) {
                    assert!(
                        !matches!(
                            outcome,
                            Type38Outcome::Blocked { .. } | Type38Outcome::Pending { .. }
                        ),
                        "tick{t}: {outcome:?}"
                    );
                }
            }
        }
        for &id in &walkers {
            let outcome = contact::resolve_type38_family_static_contact(
                &mut Intro2ContactFrame {
                    entities: &mut world.manager,
                    resources: &mut world.session.cache,
                    world_fx: &mut world.fx,
                    static_damage: &mut world.static_damage,
                    notifications: &mut world.notifications,
                    retail_tick: t,
                    actor_tasks: &mut world.tasks,
                },
                id,
                None,
            );
            assert!(
                !matches!(outcome, contact::Type38ContactOutcome::Blocked { .. }),
                "tick{t} id{id}: {outcome:?}"
            );
        }
    }
    for (&id, start) in walkers.iter().zip(start) {
        assert!(Type38FamilyOwner::adopt(&world.manager, id).is_ok());
        assert_ne!(entity(&world.manager, id).position_raw(), start);
    }
}

/// A beam with nothing in its box dies when its 1000-ms retarget times out:
/// the 51st 20-ms visit, quietly, with sound 62.
#[v2k_test_support::retail_test]
fn an_empty_beam_times_out_at_its_fifty_first_visit() {
    let mut world = World::new();
    let player = world.manager.player().unwrap().id;
    let far = entity(&world.manager, player).position_raw();
    let beam = world.beam_at([far[0].wrapping_add(8000), far[1], far[2].wrapping_add(8000)]);
    assert_eq!(style(&world.manager, beam), native_type69::SEARCH_STYLE);
    assert_eq!(
        entity(&world.manager, beam)
            .collision
            .recent_relation_id_at_0x60,
        RetailRuntimeValue::Known(Some(ids(&world.manager, 2)[0]))
    );
    let mut died_at = None;
    for t in 601..=700u32 {
        let pass = world.tick(t);
        assert!(pass.block.is_none(), "tick{t}: {:?}", pass.block);
        if let Some(Type69Outcome::Advanced { died: true, .. }) = World::beam_outcome(&pass, beam) {
            died_at = Some(t - 600);
            break;
        }
    }
    assert_eq!(died_at, Some(51));
    assert!(native_type69::finished_terminal_authenticates(
        &world.manager,
        beam
    ));
}

/// A beam that finds the player sits on it; beyond 1024 units from the
/// shooter it pulls both, beyond 2560 it dies.
#[v2k_test_support::retail_test]
fn a_beam_holds_and_pulls_the_player_then_breaks() {
    let mut world = World::new();
    let shooter = ids(&world.manager, 2)[0];
    let player = world.manager.player().unwrap().id;
    let at = entity(&world.manager, shooter).position_raw();
    let held = [at[0].wrapping_add(1500), at[1], at[2]];
    world
        .manager
        .entity_mut(player)
        .unwrap()
        .set_position_raw(held);
    let beam = world.beam_at([held[0].wrapping_add(100), held[1], held[2]]);
    let mut grabbed = false;
    for t in 601..=700u32 {
        world
            .manager
            .entity_mut(player)
            .unwrap()
            .set_motion_raw(held, [0; 3]);
        let pass = world.tick(t);
        assert!(pass.block.is_none(), "tick{t}: {:?}", pass.block);
        if style(&world.manager, beam) == native_type69::TRACTOR_STYLE {
            grabbed = true;
            assert!(matches!(
                entity(&world.manager, beam).actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::TractorBeam(task)) if task.target_id() == player
            ));
            break;
        }
    }
    assert!(grabbed, "the player is inside the beam's box");
    world
        .manager
        .entity_mut(player)
        .unwrap()
        .set_motion_raw(held, [0; 3]);
    let shooter_velocity = entity(&world.manager, shooter).velocity_raw();
    world.tick(701);
    assert_eq!(entity(&world.manager, beam).position_raw()[0], held[0]);
    let pulled = entity(&world.manager, player).velocity_raw();
    assert!(
        pulled[0] < 0,
        "the player is pulled toward the shooter: {pulled:?}"
    );
    assert_ne!(
        entity(&world.manager, shooter).velocity_raw(),
        shooter_velocity
    );
    // Beyond 2560 units the beam lets go and dies.
    let far = [at[0].wrapping_add(3000), at[1], at[2]];
    world
        .manager
        .entity_mut(player)
        .unwrap()
        .set_motion_raw(far, [0; 3]);
    let pass = world.tick(702);
    assert!(matches!(
        World::beam_outcome(&pass, beam),
        Some(Type69Outcome::Advanced { died: true, .. })
    ));
}

/// Retail crashes reading a gone shooter's position; the port stops the beam.
#[v2k_test_support::retail_test]
fn a_held_beam_whose_shooter_is_gone_stops_where_retail_crashes() {
    let mut world = World::new();
    let shooter = ids(&world.manager, 2)[0];
    let player = world.manager.player().unwrap().id;
    let at = entity(&world.manager, shooter).position_raw();
    let held = [at[0].wrapping_add(300), at[1], at[2]];
    world
        .manager
        .entity_mut(player)
        .unwrap()
        .set_position_raw(held);
    let beam = world.beam_at(held);
    for t in 601..=700u32 {
        world
            .manager
            .entity_mut(player)
            .unwrap()
            .set_motion_raw(held, [0; 3]);
        world.tick(t);
        if style(&world.manager, beam) == native_type69::TRACTOR_STYLE {
            break;
        }
    }
    assert_eq!(style(&world.manager, beam), native_type69::TRACTOR_STYLE);
    world.manager.remove_entity_for_test(shooter);
    let pass = world.tick(701);
    assert!(matches!(
        World::beam_outcome(&pass, beam),
        Some(Type69Outcome::Blocked {
            reason: Type69Error::RetailCrashOwnerGone { .. },
            ..
        })
    ));
}

/// A hit Type2 weighs Search in; Search's method-25 Aim drains through
/// 44E770's actor branch into a Type69 whose +60 is the shooter.
#[v2k_test_support::retail_test]
fn a_searching_type2_fires_beams_at_the_player() {
    use crate::{
        damage::DamagePacket,
        player_hull::PlayerHull,
        shared_actor_impact::{apply_playing_actor_particle_hit, PlayingActorImpactFrame},
        world_fx::{
            BallisticDamageRequest, ParticleEntityImpact, ParticleEnvironment,
            TerrainCollisionContext,
        },
    };
    let mut world = World::new();
    let shooter = ids(&world.manager, 2)[0];
    let player = world.manager.player().unwrap().id;
    let at = entity(&world.manager, shooter).position_raw();
    let near = [at[0].wrapping_add(1200), at[1], at[2]];
    world
        .manager
        .entity_mut(player)
        .unwrap()
        .set_position_raw(near);
    let mut hull = PlayerHull::default();
    let mut t = 600;
    while !matches!(style(&world.manager, shooter), 0x004c_7a50 | 0x004c_7a98) {
        t += 1;
        assert!(t < 640, "a hit selects Search within a few tries");
        apply_playing_actor_particle_hit(
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
                source_particle_class: 16,
                impact_position_argument_va: 0,
                target_entity_id: shooter,
                position_world: [0.0; 3],
                velocity_raw: [0, 0, 0],
                damage: Some(BallisticDamageRequest {
                    packet: DamagePacket {
                        channels: [1, 0],
                        amounts_raw: [8100, 0],
                    },
                    source_entity_type_at_birth: Some(46),
                    source_owner_id: Some(player),
                }),
            },
        );
        world.tick(t);
    }
    let mut beams = Vec::new();
    for t in t + 1..t + 600 {
        world
            .manager
            .entity_mut(player)
            .unwrap()
            .set_motion_raw(near, [0; 3]);
        let pass = world.tick(t);
        assert!(pass.block.is_none(), "tick{t}: {:?}", pass.block);
        let environment = TerrainCollisionContext::from_current_level_cache(&world.session.cache)
            .map_or(ParticleEnvironment::Dry, ParticleEnvironment::Terrain);
        if let Some(drain) = crate::intro2_projectiles::drain_intro2_projectile_source(
            &mut world.manager,
            &mut world.fx,
            &world.session.cache,
            environment,
            t,
            shooter,
        ) {
            assert!(drain.is_ok(), "tick{t}: {drain:?}");
        }
        beams = ids(&world.manager, native_type69::ENTITY_TYPE);
        if !beams.is_empty() {
            break;
        }
    }
    assert!(!beams.is_empty(), "Search's Aim fires a beam");
    let beam = entity(&world.manager, beams[0]);
    assert_eq!(beam.native_type69_runtime.unwrap().shooter_id(), shooter);
    assert_eq!(
        beam.collision.recent_relation_id_at_0x60,
        RetailRuntimeValue::Known(Some(shooter))
    );
    assert_eq!(style(&world.manager, beams[0]), native_type69::SEARCH_STYLE);
}
