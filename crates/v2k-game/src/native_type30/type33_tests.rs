//! Ordinary Type33 fireectos on the Type30 owner: Wander Near Location and
//! Guard Location on the ground host, a waved-sea Sub-C and class12 death.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    intro2_contacts::Intro2ContactFrame,
    intro2_gun_turret::authored_tests::fixture,
    player_hull::PlayerHull,
    shared_actor_impact::{
        apply_playing_actor_particle_hit, PlayingActorImpactFrame, SharedActorImpactOutcome,
    },
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
    },
    static_damage::StaticDamageScheduler,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact},
};

/// Every authored Type33: worlds 42, 43 and 47.
const WORLDS: [(u32, usize); 3] = [(42, 3), (43, 4), (47, 3)];

fn ids(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| entity.entity_type == 33)
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

fn tick(
    tasks: &mut SpecializedActorTaskScheduler,
    manager: &mut EntityManager,
    session: &mut crate::session::GameSession,
    fx: &mut WorldFx,
    static_damage: &mut StaticDamageScheduler,
    notifications: &mut GameplayNotifications,
    t: u32,
) {
    manager.advance_environment_frame(fx, 20_000);
    let pass = tasks.tick(
        manager,
        SpecializedActorTaskProductionFrame {
            world: SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase: GameplayNotificationPhase::Playing,
            resources: &mut session.cache,
            world_fx: fx,
            static_damage,
            elapsed_micros: 20_000,
            global_elapsed_micros: t * 20_000,
            retail_tick: t,
            main_base_abort_active: false,
        },
        notifications,
    );
    assert!(pass.block.is_none(), "tick{t}: {:?}", pass.block);
    for outcome in &pass.outcomes {
        if let SpecializedActorTaskProductionOutcome::NativeType30(outcome) = outcome {
            assert!(
                !matches!(
                    outcome,
                    Type30Outcome::Blocked { .. } | Type30Outcome::Pending { .. }
                ),
                "tick{t}: {outcome:?}"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn type33_births_publish_wander_or_guard_on_the_type30_owner() {
    for (level, count) in WORLDS {
        let (session, manager, _) = fixture(level);
        let ids = ids(&manager);
        assert_eq!(ids.len(), count, "world{level}");
        let terrain = session.cache.level_terrain().unwrap();
        for &id in &ids {
            let entity = entity(&manager, id);
            assert!(
                manager_allocation_authenticates(&manager, id),
                "world{level}"
            );
            assert_eq!(type30_row(entity), Some(Type30Row::Type33));
            let runtime = entity.native_type30_runtime.as_ref().unwrap();
            let [x, _, z] = runtime.anchor_raw;
            // D4A0's bit20 grounds the body with Sub-C's 220 clearance.
            assert_eq!(
                runtime.anchor_raw[1],
                terrain.bilinear_height_raw(x, z).wrapping_add(220)
            );
            assert_eq!(runtime.sub_e_runtime.projectile_method, 24);
            assert_eq!(runtime.sub_e_runtime.sound_id, 105);
            assert_eq!(runtime.kl_components.model_variables_raw().len(), 4);
            assert!(matches!(
                &entity.sub_h_external_frame_runtime,
                RetailRuntimeValue::Known(Some(h)) if h.records().len() == 4
                    && h.surface_policy()
                        == crate::sub_h_external_frame::SubHSurfacePolicy::TerrainAndWater
            ));
            assert!(
                matches!(style(&manager, id), 0x004c_79c0 | 0x004c_7bb8),
                "world{level}"
            );
            assert_eq!(entity.collision.state_flags_at_0x08.known_mask(), u32::MAX);
            assert!(
                Type30FamilyOwner::adopt(&manager, id).is_ok(),
                "world{level}"
            );
        }
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type30(&manager), count, "world{level}");
    }
}

/// Twenty seconds under the real scheduler, the native surface contact and
/// the late static walk.
#[v2k_test_support::retail_test]
fn type33_cohorts_run_twenty_seconds_with_late_contacts() {
    for (level, _) in WORLDS {
        let (mut session, mut manager, mut fx) = fixture(level);
        let ids = ids(&manager);
        activate(&mut manager, &ids);
        let mut tasks = SpecializedActorTaskScheduler::new();
        tasks.adopt_type30(&manager);
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let start: Vec<_> = ids
            .iter()
            .map(|&id| entity(&manager, id).position_raw())
            .collect();
        for t in 1..=1000u32 {
            tick(
                &mut tasks,
                &mut manager,
                &mut session,
                &mut fx,
                &mut static_damage,
                &mut notifications,
                t,
            );
            for &id in &ids {
                let mut frame = Intro2ContactFrame {
                    entities: &mut manager,
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    static_damage: &mut static_damage,
                    notifications: &mut notifications,
                    retail_tick: t,
                    actor_tasks: &mut tasks,
                };
                let surface =
                    crate::native_actor_surface_contact::resolve_native_actor_surface_contact(
                        &mut frame, id,
                    );
                assert!(
                    !matches!(
                        surface,
                        crate::native_actor_surface_contact::NativeActorSurfaceContactOutcome::Blocked { .. }
                    ),
                    "world{level} tick{t} id{id}: {surface:?}"
                );
                let outcome = contact::resolve_type30_static_contact(&mut frame, id);
                assert!(
                    !matches!(outcome, contact::Type30ContactOutcome::Blocked { .. }),
                    "world{level} tick{t} id{id}: {outcome:?}"
                );
            }
        }
        for (index, &id) in ids.iter().enumerate() {
            assert_ne!(entity(&manager, id).position_raw(), start[index]);
            assert!(
                Type30FamilyOwner::adopt(&manager, id).is_ok(),
                "world{level}"
            );
        }
    }
}

fn lethal_hit(id: u32) -> ParticleEntityImpact {
    ParticleEntityImpact {
        source_particle_class: 16,
        impact_position_argument_va: 0,
        target_entity_id: id,
        position_world: [0.0; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(BallisticDamageRequest {
            packet: DamagePacket {
                channels: [2, 0],
                amounts_raw: [1_000_000, 0],
            },
            source_entity_type_at_birth: Some(34),
            source_owner_id: Some(35),
        }),
    }
}

/// A lethal Playing hit takes the alternate class12: Flip Over And Die.
#[v2k_test_support::retail_test]
fn a_lethal_hit_flips_a_type33_into_class12() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(42);
    let ids = ids(&manager);
    activate(&mut manager, &ids);
    let mut tasks = SpecializedActorTaskScheduler::new();
    tasks.adopt_type30(&manager);
    let id = ids[0];
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut hull = PlayerHull::default();
    let outcome = apply_playing_actor_particle_hit(
        PlayingActorImpactFrame {
            extra_lives: RetailRuntimeValue::Known(3),
            resources: &mut session.cache,
            entities: &mut manager,
            world_fx: &mut fx,
            scheduler: &mut tasks,
            notifications: &mut notifications,
            static_damage: &mut static_damage,
            player_hull: &mut hull,
            retail_tick: 600,
        },
        lethal_hit(id),
    );
    assert!(
        matches!(&outcome, Some(SharedActorImpactOutcome::Type30(_))),
        "{outcome:?}"
    );
    assert_eq!(style(&manager, id), 0x004c_7ed0);
    assert!(matches!(
        entity(&manager, id).actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::CommonDying(_))
    ));
}

/// A player inside the strict cube is Guard's first eligible candidate:
/// `C7D0` publishes `4C7C00` and `ADE0`'s Chase and method24 Aim.
#[v2k_test_support::retail_test]
fn a_guarding_type33_takes_the_player_into_chase_and_aim() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(43);
    let ids = ids(&manager);
    activate(&mut manager, &ids);
    let id = *ids
        .iter()
        .find(|&&id| style(&manager, id) == 0x004c_7bb8)
        .expect("a guarding Type33");
    let anchor = entity(&manager, id).position_raw();
    let player = manager.player().unwrap().id;
    manager.entity_mut(player).unwrap().set_position_raw([
        anchor[0].wrapping_add(0x200),
        anchor[1],
        anchor[2],
    ]);
    let mut tasks = SpecializedActorTaskScheduler::new();
    tasks.adopt_type30(&manager);
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut pursued = false;
    for t in 1..=200u32 {
        tick(
            &mut tasks,
            &mut manager,
            &mut session,
            &mut fx,
            &mut static_damage,
            &mut notifications,
            t,
        );
        if style(&manager, id) == 0x004c_7c00 {
            pursued = true;
            break;
        }
        // A finished wander may reselect Wander Near Location instead.
        if style(&manager, id) != 0x004c_7bb8 {
            break;
        }
    }
    assert!(pursued, "the guard's one-in-four gate finds the player");
    assert!(matches!(
        entity(&manager, id).actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::ChaseTarget(task)) if task.target_id() == player
    ));
    assert!(matches!(
        entity(&manager, id).actor_task_state(ActorTaskSlot::Tertiary),
        Some(ActorTaskRuntime::AimAndFire(_))
    ));
}
