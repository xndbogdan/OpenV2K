//! Ordinary-world Type55 guards and Type81 carriers on the Type10 owner.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    class49_death::{source_profile, NativeExplosionPolicy, NativeExplosionSourceProfile},
    damage::DamagePacket,
    entity::EntityManager,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    intro2_contacts::Intro2ContactFrame,
    intro2_flyer_contacts::resolve_native_flyer_contacts,
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
    world_fx::{BallisticDamageRequest, ParticleEntityImpact, WorldFx},
};

/// Every authored guard: Type55 in worlds 19, 29 and 32, Type81 in 38.
const WORLDS: [(u32, Type10Profile, usize); 4] = [
    (19, Type10Profile::Type55, 1),
    (29, Type10Profile::Type55, 5),
    (32, Type10Profile::Type55, 3),
    (38, Type10Profile::Type81, 3),
];

fn ids(manager: &EntityManager, profile: Type10Profile) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| entity.entity_type == profile.entity_type())
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
    tick: u32,
) -> Vec<SpecializedActorTaskProductionOutcome> {
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
            global_elapsed_micros: tick * 20_000,
            retail_tick: tick,
            main_base_abort_active: false,
        },
        notifications,
    );
    assert!(pass.block.is_none(), "tick{tick}: {:?}", pass.block);
    pass.outcomes
}

#[v2k_test_support::retail_test]
fn guard_births_publish_guard_location_on_the_type10_owner() {
    for (level, profile, count) in WORLDS {
        let (session, manager, _) = fixture(level);
        let ids = ids(&manager, profile);
        assert_eq!(ids.len(), count, "world{level}");
        let level_desc = session.cache.level_desc().unwrap();
        for &id in &ids {
            let entity = entity(&manager, id);
            assert!(type10_manager_allocation_authenticates(&manager, id));
            assert_eq!(type10_profile(entity), Some(profile));
            let runtime = entity.intro2_type10_runtime.unwrap();
            let spawn = &level_desc.entities[entity.authored_spawn_index.unwrap()];
            assert_eq!(runtime.anchor_raw, spawn.position_raw());
            assert_eq!(runtime.sub_e_runtime.projectile_method, 10);
            assert_eq!(
                runtime.sub_e_runtime.sound_id,
                u32::from(profile.emitter().sound_id)
            );
            // A newborn is never UnderAttack: AC60 takes Always Guard Location.
            assert!(matches!(
                entity.initial_behavior,
                RetailRuntimeValue::Known(Some(selection))
                    if selection.choice_index == 1 && selection.program.class_id == 32
            ));
            assert!(guard::acquiring_graph_authenticates(entity), "world{level}");
            assert_eq!(style(&manager, id), 0x004c_7bb8);
            assert_eq!(
                entity.actor_common_axis_descriptor,
                RetailRuntimeValue::Known(profile.axis())
            );
            assert_eq!(entity.collision.state_flags_at_0x08.known_mask(), u32::MAX);
            assert!(Intro2Type10Owner::adopt(&manager, id).is_ok());
            if profile == Type10Profile::Type81 {
                let source = source_profile(entity).unwrap();
                assert_eq!(source, NativeExplosionSourceProfile::Type10Carrier(profile));
                assert_eq!(source.policy(), NativeExplosionPolicy::Class63);
                assert_eq!(
                    entity.auto_pilot_payload_packed,
                    Some(u32::from_le_bytes(spawn.extra[8..12].try_into().unwrap()))
                );
            }
        }
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_intro2_type10(&manager), count, "world{level}");
    }
}

/// Twenty seconds under the real scheduler and the late flyer contacts. Each
/// 02EB0 retarget copies the `+90` anchor's Y and lands within 1024 on X/Z;
/// contact hooks may later redirect the same private target.
#[v2k_test_support::retail_test]
fn guard_cohorts_wander_near_their_anchors_for_twenty_seconds() {
    for (level, profile, _) in WORLDS {
        let (mut session, mut manager, mut fx) = fixture(level);
        let ids = ids(&manager, profile);
        activate(&mut manager, &ids);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_intro2_type10(&manager), ids.len());
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let start: Vec<_> = ids
            .iter()
            .map(|&id| entity(&manager, id).position_raw())
            .collect();
        let wander_target = |manager: &EntityManager, id| match entity(manager, id)
            .actor_task_state(ActorTaskSlot::Primary)
        {
            Some(ActorTaskRuntime::OrdinaryType9Wander(task)) => {
                Some(task.private_state().target_position_raw)
            }
            _ => None,
        };
        let mut retargets = 0;
        for t in 1..=1000u32 {
            let before: Vec<_> = ids.iter().map(|&id| wander_target(&manager, id)).collect();
            for outcome in tick(
                &mut tasks,
                &mut manager,
                &mut session,
                &mut fx,
                &mut static_damage,
                &mut notifications,
                t,
            ) {
                if let SpecializedActorTaskProductionOutcome::Intro2Type10(outcome) = &outcome {
                    assert!(
                        matches!(
                            outcome,
                            Intro2Type10Outcome::Waiting { .. }
                                | Intro2Type10Outcome::Advanced { .. }
                        ),
                        "world{level} tick{t}: {outcome:?}"
                    );
                }
            }
            for (index, &id) in ids.iter().enumerate() {
                let anchor = entity(&manager, id)
                    .intro2_type10_runtime
                    .unwrap()
                    .anchor_raw;
                if let (Some(old), Some(target)) = (before[index], wander_target(&manager, id)) {
                    if target != old && target[1] == anchor[1] {
                        retargets += 1;
                        assert!(
                            [0, 2].into_iter().all(|axis| {
                                target[axis].wrapping_sub(anchor[axis]).unsigned_abs() <= 1024
                            }),
                            "world{level} tick{t} id{id}: {target:?} {anchor:?}"
                        );
                    }
                }
            }
            for &id in &ids {
                let outcome = resolve_native_flyer_contacts(
                    &mut Intro2ContactFrame {
                        entities: &mut manager,
                        resources: &mut session.cache,
                        world_fx: &mut fx,
                        static_damage: &mut static_damage,
                        notifications: &mut notifications,
                        retail_tick: t,
                        actor_tasks: &mut tasks,
                    },
                    id,
                );
                assert!(
                    !outcome.blocks_later_contacts(),
                    "world{level} tick{t} id{id}: {outcome:?}"
                );
            }
        }
        assert!(retargets > 0, "world{level}");
        for (index, &id) in ids.iter().enumerate() {
            assert_ne!(entity(&manager, id).position_raw(), start[index]);
            assert!(
                Intro2Type10Owner::adopt(&manager, id).is_ok(),
                "world{level}"
            );
        }
    }
}

/// 425680's rule2: a hit within 250 ticks makes Search weigh in; Type55's
/// low selector word then takes class7 and B6C0, a quiet clock keeps Guard.
#[v2k_test_support::retail_test]
fn under_attack_reselection_weighs_search_beside_guard() {
    let (_, mut manager, _) = fixture(19);
    let metadata = manager.type_runtime_metadata(55).cloned().unwrap();
    let id = ids(&manager, Type10Profile::Type55)[0];
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(500);
    let mut words = [0x0000, 0x1100, 0x2200].into_iter();
    let publication =
        native::reselect_acquiring(entity, &metadata, 600, &mut || words.next().unwrap()).unwrap();
    assert_eq!(publication.selection.program.class_id, 7);
    assert_eq!(words.next(), None, "selector then B6C0's two G words");
    assert_eq!(style(&manager, id), 0x004c_7a50);
    let entity = manager.entity_mut(id).unwrap();
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::SharedRetarget(_))
    ));
    // Two hundred and fifty ticks later the same word keeps Guard Location.
    let mut words = [0x0000, 0x1100, 0x2200].into_iter();
    let publication =
        native::reselect_acquiring(entity, &metadata, 750, &mut || words.next().unwrap()).unwrap();
    assert_eq!(publication.selection.program.class_id, 32);
    assert!(guard::acquiring_graph_authenticates(
        manager.entity_mut(id).unwrap()
    ));
}

/// A player inside the strict cube is the first eligible candidate: C7D0
/// publishes `4C7C00` and ADE0's Chase/Aim, whose Aim fires that pass.
#[v2k_test_support::retail_test]
fn a_type81_guard_takes_the_player_into_chase_and_aim() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(38);
    let ids = ids(&manager, Type10Profile::Type81);
    activate(&mut manager, &ids);
    let id = ids[0];
    let anchor = entity(&manager, id).position_raw();
    let player = manager.player().unwrap().id;
    manager.entity_mut(player).unwrap().set_position_raw([
        anchor[0].wrapping_add(0x400),
        anchor[1],
        anchor[2],
    ]);
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_intro2_type10(&manager), ids.len());
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut pursued = None;
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
            pursued = Some(t);
            break;
        }
    }
    pursued.expect("the guard's one-in-four gate finds the player");
    let entity = entity(&manager, id);
    assert!(search::pursuing_graph_authenticates(entity));
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::ChaseTarget(task)) if task.target_id() == player
    ));
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

/// A guarding Type55 dies into class11 Tumble; a guarding Type81 into
/// class63's BC90, which keeps its tasks and drops its authored Type61.
#[v2k_test_support::retail_test]
fn lethal_hits_on_guards_run_tumble_and_class63() {
    for (level, profile) in [(19, Type10Profile::Type55), (38, Type10Profile::Type81)] {
        let (mut session, mut manager, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level);
        manager.cleanup_pending_actor_deferred_destroys();
        let ids = ids(&manager, profile);
        activate(&mut manager, &ids);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_intro2_type10(&manager), ids.len());
        let id = ids[0];
        let payload = entity(&manager, id).auto_pilot_payload_packed;
        let position = entity(&manager, id).position_raw();
        let newest = manager.iter_all().map(|entity| entity.id).max().unwrap();
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
        let Some(SharedActorImpactOutcome::Type10(impact::Intro2Type10ImpactOutcome::Applied(
            applied,
        ))) = &outcome
        else {
            panic!("world{level}: {outcome:?}");
        };
        match profile {
            Type10Profile::Type55 => {
                assert!(applied.death_publication.is_some());
                assert_eq!(style(&manager, id), 0x004c_7f60);
            }
            _ => {
                assert!(applied.death_publication.is_none());
                assert!(crate::class49_death::finished_terminal_hit_authenticates(
                    &manager, id
                ));
                assert_eq!(style(&manager, id), 0x004c_7198);
                assert!(matches!(
                    entity(&manager, id).actor_task_state(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::OrdinaryType9Wander(_))
                ));
                let drop = manager
                    .iter_all()
                    .find(|entity| entity.id > newest && entity.entity_type == 61)
                    .expect("Type61 drop");
                assert_eq!(drop.position_raw(), position);
                assert_eq!(drop.power_up_payload_packed, payload);
            }
        }
    }
}
