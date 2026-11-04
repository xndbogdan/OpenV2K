//! Ordinary-world Type29 on the Type10 owner: class21 Mobbing Target, class10
//! Run Away and class7 Search.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    entity::EntityManager,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    intro2_contacts::Intro2ContactFrame,
    intro2_flyer_contacts::resolve_native_flyer_contacts,
    intro2_gun_turret::authored_tests::fixture,
    native_actor_capture::pair::{
        resolve_native_captor_active_contacts_with_playing, CaptureFeedbackPolicy,
        NativeCaptorPairBehaviorResult, NativeCaptorPairOutcome, NativeCaptorPairStage,
        PlayingPlayerContact,
    },
    player_hull::{HullDamageProfile, PlayerHull},
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

/// Every authored Type29: two in world 20, three in world 24.
const WORLDS: [(u32, usize); 2] = [(20, 2), (24, 3)];

fn ids(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| entity.entity_type == 29)
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
}

fn assert_advanced(outcomes: &[SpecializedActorTaskProductionOutcome], context: &str) {
    for outcome in outcomes {
        if let SpecializedActorTaskProductionOutcome::Intro2Type10(outcome) = outcome {
            assert!(
                matches!(
                    outcome,
                    Intro2Type10Outcome::Waiting { .. } | Intro2Type10Outcome::Advanced { .. }
                ),
                "{context}: {outcome:?}"
            );
        }
    }
}

#[test]
fn the_row_is_type5s_g_with_its_own_l_and_no_k() {
    let profile = Type10Profile::Type29;
    assert_eq!(profile.sub_g()[4], 125);
    assert_eq!(profile.sub_g()[..4], TYPE5_SUB_G[..4]);
    assert_eq!(profile.sub_g()[5..], TYPE5_SUB_G[5..]);
    assert_eq!(profile.sub_k(), None);
    assert!(!profile.topology().sub_k && profile.topology().sub_l);
    assert_eq!(profile.sub_l(), [6, 7, 64, 31, 160, 15]);
    assert_eq!(profile.emitter().projectile_method, 24);
    assert_eq!(profile.behavior_rule_ref(), 3);
    assert_eq!(mobbing::pair_hit_packet().amounts_raw, [6000, 0]);
}

/// AC60 draws one word over Always x1 Mobbing, UnderAttack x9 Run Away and
/// Always x4 Search; a newborn is never under attack.
#[v2k_test_support::retail_test]
fn type29_births_take_mobbing_or_search_on_the_type10_owner() {
    for (level, count) in WORLDS {
        let (session, manager, _) = fixture(level);
        let ids = ids(&manager);
        assert_eq!(ids.len(), count, "world{level}");
        let metadata = manager.type_runtime_metadata(29).unwrap();
        native::authenticate_metadata(Type10Profile::Type29, metadata).unwrap();
        let level_desc = session.cache.level_desc().unwrap();
        for &id in &ids {
            let entity = entity(&manager, id);
            assert!(type10_manager_allocation_authenticates(&manager, id));
            assert_eq!(type10_profile(entity), Some(Type10Profile::Type29));
            let runtime = entity.intro2_type10_runtime.unwrap();
            let spawn = &level_desc.entities[entity.authored_spawn_index.unwrap()];
            assert_eq!(runtime.anchor_raw, spawn.position_raw());
            assert_eq!(runtime.sub_e_runtime.projectile_method, 24);
            assert_eq!(runtime.sub_e_runtime.sound_id, 70);
            let RetailRuntimeValue::Known(Some(selection)) = entity.initial_behavior else {
                panic!("selection");
            };
            match selection.program.class_id {
                21 => {
                    assert_eq!(selection.choice_index, 0);
                    assert!(mobbing::searching_graph_authenticates(entity));
                    assert_eq!(style(&manager, id), 0x004c_7c48);
                    assert!(matches!(
                        entity.actor_task_state(ActorTaskSlot::Primary),
                        Some(ActorTaskRuntime::SharedRetarget(task))
                            if task.lifetime_ms() == 5000
                    ));
                }
                7 => {
                    assert_eq!(selection.choice_index, 2);
                    assert_eq!(style(&manager, id), 0x004c_7a50);
                }
                class => panic!("world{level}: class{class}"),
            }
            assert!(Intro2Type10Owner::adopt(&manager, id).is_ok());
        }
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_intro2_type10(&manager), count, "world{level}");
    }
}

/// Twenty seconds under the scheduler and the late flyer contacts.
#[v2k_test_support::retail_test]
fn type29_cohorts_fly_twenty_seconds_with_their_contacts() {
    for (level, _) in WORLDS {
        let (mut session, mut manager, mut fx) = fixture(level);
        let ids = ids(&manager);
        activate(&mut manager, &ids);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_intro2_type10(&manager), ids.len());
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let start: Vec<_> = ids
            .iter()
            .map(|&id| entity(&manager, id).position_raw())
            .collect();
        for t in 1..=1000u32 {
            let outcomes = tick(&mut tasks, &mut manager, &mut session, &mut fx, t);
            assert_advanced(&outcomes, &format!("world{level} tick{t}"));
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
        for (index, &id) in ids.iter().enumerate() {
            assert_ne!(entity(&manager, id).position_raw(), start[index]);
            assert!(Intro2Type10Owner::adopt(&manager, id).is_ok());
        }
    }
}

/// Put one Type29 into Mobbing's approach on the player, below and ahead.
fn approach_fixture() -> (
    crate::session::GameSession,
    EntityManager,
    WorldFx,
    SpecializedActorTaskScheduler,
    u32,
    u32,
) {
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(20);
    let ids = ids(&manager);
    activate(&mut manager, &ids);
    let id = ids[0];
    let player = manager.player().unwrap().id;
    let position = entity(&manager, id).position_raw();
    manager.entity_mut(player).unwrap().set_position_raw([
        position[0].wrapping_add(0x800),
        position[1].wrapping_sub(0x400),
        position[2],
    ]);
    let entity = manager.entity_mut(id).unwrap();
    if !matches!(
        entity.current_behavior_context,
        RetailRuntimeValue::Known(Some(context))
            if context.active_style().style_address() == mobbing::SEARCHING_STYLE_ADDRESS
    ) {
        let metadata = manager.type_runtime_metadata(29).cloned().unwrap();
        let entity = manager.entity_mut(id).unwrap();
        // Word zero takes Always x1 Mobbing with no hit on the clock.
        let mut words = [0x0000, 0, 0].into_iter();
        native::reselect_acquiring(entity, &metadata, 600, &mut || words.next().unwrap()).unwrap();
    }
    let entity = manager.entity_mut(id).unwrap();
    mobbing::switch_style_for_test(entity, 1, Some(player), &mut fx).unwrap();
    assert!(mobbing::pursuing_graph_authenticates(entity));
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_intro2_type10(&manager), ids.len());
    (session, manager, fx, tasks, id, player)
}

/// The approach climbs to 500 above the player, dives within 0x200 more than
/// 300 above it, and the dive returns to the approach.
#[v2k_test_support::retail_test]
fn mobbing_approaches_dives_and_returns_on_the_player() {
    let (mut session, mut manager, mut fx, mut tasks, id, _) = approach_fixture();
    assert!(matches!(
        entity(&manager, id).sub_g_06070_runtime,
        RetailRuntimeValue::Known(Some(g)) if g.mode_at_0x40() == RetailRuntimeValue::Known(0)
    ));
    let mut dived = None;
    let mut returned = None;
    for t in 1..=3000u32 {
        let outcomes = tick(&mut tasks, &mut manager, &mut session, &mut fx, t);
        assert_advanced(&outcomes, &format!("tick{t}"));
        let current = style(&manager, id);
        if current == mobbing::APPROACH_STYLE_ADDRESS {
            if dived.is_some() {
                returned = Some(t);
                break;
            }
        } else if current == mobbing::DIVE_STYLE_ADDRESS {
            if dived.is_none() {
                dived = Some(t);
                let RetailRuntimeValue::Known(Some(g)) = entity(&manager, id).sub_g_06070_runtime
                else {
                    panic!()
                };
                assert_eq!(g.mode_at_0x40(), RetailRuntimeValue::Known(1));
                assert_eq!(g.source_raw_at_0x20(), RetailRuntimeValue::Known(0));
            }
        } else {
            // Out of range or a zero mover: C690 reselects over the list.
            break;
        }
    }
    let dived = dived.expect("the approach reaches the dive box above the player");
    let returned = returned.expect("the dive returns to the approach");
    assert!(returned > dived);
}

/// The approach's style `+18` hits the player with a checked 6000 of kind
/// 29. The craft's channel-1 threshold is exactly 6000, so retail's filter
/// leaves its hull untouched; a lower threshold shows the packet arrives.
#[v2k_test_support::retail_test]
fn touching_an_approaching_mobber_hits_the_player_through_the_lane() {
    for threshold in [6000, 0] {
        touch_with_player_threshold(threshold);
    }
}

fn touch_with_player_threshold(threshold: i32) {
    let (mut session, mut manager, mut fx, mut tasks, id, player) = approach_fixture();
    let position = entity(&manager, id).position_raw();
    let body = manager.entity_mut(player).unwrap();
    let RetailRuntimeValue::Known(mut profile) = body.collision.damage_profile else {
        panic!("player damage profile");
    };
    assert_eq!(profile.thresholds_raw[1], 6000);
    profile.thresholds_raw[1] = threshold;
    body.collision.damage_profile = RetailRuntimeValue::Known(profile);
    body.set_position_raw(position);
    body.physical_body_basis_q31 = RetailRuntimeValue::Known(
        crate::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(0, 0, 0),
    );
    let mut hull = PlayerHull::new(HullDamageProfile::from_type_record(
        session.cache.global_entity_type(46).unwrap(),
    ));
    hull.readback_entity_damage_state(&manager.player().unwrap().collision);
    let before = (hull.health_raw, hull.pre_health_damage_buffer_raw);
    let result = resolve_native_captor_active_contacts_with_playing(
        &mut Intro2ContactFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut StaticDamageScheduler::new(),
            notifications: &mut GameplayNotifications::new(),
            retail_tick: 1002,
            actor_tasks: &mut tasks,
        },
        player,
        CaptureFeedbackPolicy::Gameplay,
        Some(PlayingPlayerContact {
            craft: None,
            hull: &mut hull,
            extra_lives: RetailRuntimeValue::Known(3),
        }),
    );
    let NativeCaptorPairOutcome::Resolved { visits } = result else {
        panic!("{result:?}")
    };
    let visit = visits
        .iter()
        .find(|visit| visit.candidate_id == id)
        .expect("player-Type29 pair");
    assert!(
        visit.stages.contains(&NativeCaptorPairStage::Behavior {
            owner: id,
            style: mobbing::APPROACH_STYLE_ADDRESS,
            result: NativeCaptorPairBehaviorResult::Null,
        }),
        "{:?}",
        visit.stages
    );
    let after = (hull.health_raw, hull.pre_health_damage_buffer_raw);
    if threshold == 6000 {
        assert_eq!(after, before, "{:?}", visit.stages);
    } else {
        assert_ne!(after, before, "{:?}", visit.stages);
    }
}

/// Rule2: a recent hit weighs Run Away x9; its B6C0 search finds the player
/// and C7D0's style `4C7660` runs `B3F0`'s flee with the G word.
#[v2k_test_support::retail_test]
fn under_attack_type29s_run_away_from_the_player() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(20);
    let ids = ids(&manager);
    activate(&mut manager, &ids);
    let id = ids[0];
    let metadata = manager.type_runtime_metadata(29).cloned().unwrap();
    let position = entity(&manager, id).position_raw();
    let player = manager.player().unwrap().id;
    manager.entity_mut(player).unwrap().set_position_raw([
        position[0].wrapping_add(0x400),
        position[1],
        position[2],
    ]);
    let entity_mut = manager.entity_mut(id).unwrap();
    entity_mut.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(500);
    // Weights 1/9/4: 0x3000 of 0x7FFF lands in Run Away's band.
    let mut words = [0x3000, 0x1100, 0x2200].into_iter();
    let publication =
        native::reselect_acquiring(entity_mut, &metadata, 600, &mut || words.next().unwrap())
            .unwrap();
    assert_eq!(publication.selection.program.class_id, 10);
    assert_eq!(words.next(), None, "selector then B6C0's two G words");
    assert_eq!(style(&manager, id), 0x004c_7618);
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_intro2_type10(&manager), ids.len());
    let mut fled = false;
    for t in 1..=50u32 {
        let outcomes = tick(&mut tasks, &mut manager, &mut session, &mut fx, t);
        assert_advanced(&outcomes, &format!("tick{t}"));
        if style(&manager, id) == 0x004c_7660 {
            assert!(matches!(
                entity(&manager, id).actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::RunAway(task)) if task.target_id() == player
            ));
            fled = true;
            break;
        }
    }
    assert!(fled, "the search hands the player to the flee");
}

/// A lethal hit mid-dive publishes class11 Tumble; 06070 clears the dive mode.
#[v2k_test_support::retail_test]
fn a_lethal_hit_mid_dive_tumbles() {
    let (mut session, mut manager, mut fx, _, id, _) = approach_fixture();
    let entity_mut = manager.entity_mut(id).unwrap();
    mobbing::switch_style_for_test(entity_mut, 2, None, &mut fx).unwrap();
    assert_eq!(style(&manager, id), mobbing::DIVE_STYLE_ADDRESS);
    let mut tasks = SpecializedActorTaskScheduler::new();
    tasks.adopt_intro2_type10(&manager);
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
        ParticleEntityImpact {
            no_packet: false,
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
        },
    );
    let Some(SharedActorImpactOutcome::Type10(impact::Intro2Type10ImpactOutcome::Applied(applied))) =
        &outcome
    else {
        panic!("{outcome:?}");
    };
    assert!(applied.death_publication.is_some());
    assert_eq!(style(&manager, id), 0x004c_7f60);
    assert!(matches!(
        entity(&manager, id).sub_g_06070_runtime,
        RetailRuntimeValue::Known(Some(g)) if g.mode_at_0x40() == RetailRuntimeValue::Known(0)
            && g.mode_at_0x3f() == RetailRuntimeValue::Known(1)
    ));
}
