//! Type63 births, visits, the hold walk and its death on world 13.

use super::*;
use crate::damage::{DamageDeliveryRecord, DamagePacket};
use crate::entity::{
    CampaignCargoRestoreContext, PlayerCheckedDamageFrame, PlayerCheckedDamageOutcome,
    PlayerCheckedDamageRequest, PlayerDamageEntry,
};

fn birth_ball(
    session: &crate::session::GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    tasks: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
) -> u32 {
    manager
        .dispatch_controller_operation_0x38(
            ENTITY_TYPE,
            &mut CampaignCargoRestoreContext {
                level: session.cache.level_desc().unwrap(),
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
                },
                retail_tick: 791,
                scheduler: tasks,
                world_fx: fx,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            },
        )
        .unwrap();
    *manager.auxiliary_owned_entity_ids().last().unwrap()
}

fn hull_of(manager: &EntityManager) -> crate::player_hull::PlayerHull {
    let player = manager.player().unwrap();
    let mut hull = crate::player_hull::PlayerHull::default();
    hull.health_raw = match player.collision.health_raw {
        RetailRuntimeValue::Known(health) => health,
        RetailRuntimeValue::Unresolved => panic!("player health"),
    };
    hull.pre_health_damage_buffer_raw = match player.collision.pre_health_damage_buffer_raw {
        RetailRuntimeValue::Known(buffer) => buffer,
        RetailRuntimeValue::Unresolved => panic!("player buffer"),
    };
    hull
}

fn hit_player(
    session: &crate::session::GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    hull: &mut crate::player_hull::PlayerHull,
) -> PlayerCheckedDamageOutcome {
    let player = manager.player().unwrap().id;
    manager.apply_player_checked_damage(
        PlayerCheckedDamageRequest {
            entry: PlayerDamageEntry::Checked,
            target_id: player,
            delivery: DamageDeliveryRecord {
                packet: DamagePacket {
                    channels: [1, 0],
                    amounts_raw: [40_000, 0],
                },
                source_entity_type_raw: 29,
                owner_handle: 0,
            },
            ratio_numerator: 0,
            ratio_denominator: 0,
        },
        PlayerCheckedDamageFrame {
            hull,
            resources: &session.cache,
            world_fx: fx,
            retail_tick: 800,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            extra_lives: 3,
        },
    )
}

/// Operation 0x38 puts the ball in the hold at the player; class50 draws its
/// pitch, and each visit pins it to the player and spins it.
#[v2k_test_support::retail_test]
fn the_ball_joins_the_hold_and_sits_on_the_player_spinning() {
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(13);
    let mut tasks = super::super::queen_tests::adopt_world(&mut manager);
    let expected_pitch = {
        // The singleton selector draws first, then 07070's pitch.
        let mut state = *fx.entity_construction_state().0;
        crate::retail_rng::retail_random_u16(&mut state);
        crate::retail_rng::retail_random_u16(&mut state) as i16
    };
    let ball = birth_ball(&session, &mut manager, &mut fx, &mut tasks);
    let player = manager.player().unwrap();
    let (player_id, player_position) = (player.id, player.position_raw());
    let body = manager.iter_all().find(|entity| entity.id == ball).unwrap();
    assert_eq!(body.entity_type, ENTITY_TYPE);
    assert_eq!(body.collision.health_raw, RetailRuntimeValue::Known(1));
    assert_eq!(
        body.collision.recent_relation_id_at_0x60,
        RetailRuntimeValue::Known(Some(player_id))
    );
    // 0x404 lacks 0x80, so the ball keeps checked damage.
    assert_eq!(
        body.collision.state_flags_at_0x08.masked(0x8000),
        RetailRuntimeValue::Known(0x8000)
    );
    assert_eq!(body.rotation_heading_pitch_roll_raw()[1], expected_pitch);
    assert!(matches!(
        body.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::DefenderBall)
    ));
    let [heading, pitch, roll] = body.rotation_heading_pitch_roll_raw();
    manager
        .entity_mut(ball)
        .unwrap()
        .set_velocity_raw([5, 6, 7]);
    manager.defender_ball_visit(ball, 20_000).unwrap();
    let body = manager.iter_all().find(|entity| entity.id == ball).unwrap();
    assert_eq!(body.position_raw(), player_position);
    assert_eq!(body.velocity_raw(), [0; 3]);
    assert_eq!(
        body.rotation_heading_pitch_roll_raw(),
        [
            heading,
            pitch.wrapping_add((20_000 >> 7) as i16),
            roll.wrapping_add((20_000 >> 5) + (20_000 >> 4))
        ]
    );
}

/// 4484A0: a live ball takes the player's packet and dies (health 1, class2),
/// the craft takes none; the next packet, with the ball dead, reaches the
/// craft.
#[v2k_test_support::retail_test]
fn a_live_ball_takes_the_players_hit_then_the_craft_takes_the_next() {
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(13);
    let mut tasks = super::super::queen_tests::adopt_world(&mut manager);
    let ball = birth_ball(&session, &mut manager, &mut fx, &mut tasks);
    let mut hull = hull_of(&manager);
    let health = hull.health_raw;
    assert_eq!(
        manager.player_pair_damage_modifier_context_empty(),
        RetailRuntimeValue::Known(false)
    );
    let outcome = hit_player(&session, &mut manager, &mut fx, &mut hull);
    assert!(
        matches!(
            outcome,
            PlayerCheckedDamageOutcome::Applied { dying: false, .. }
        ),
        "{outcome:?}"
    );
    assert_eq!(hull.health_raw, health);
    let body = manager.iter_all().find(|entity| entity.id == ball).unwrap();
    assert_eq!(body.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        body.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
        RetailRuntimeValue::Known(DYING_STATE_BIT)
    );
    assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .all(|slot| body.actor_task_state(slot).is_none()));
    assert_eq!(
        manager.player_pair_damage_modifier_context_empty(),
        RetailRuntimeValue::Known(true)
    );
    let outcome = hit_player(&session, &mut manager, &mut fx, &mut hull);
    assert!(
        matches!(outcome, PlayerCheckedDamageOutcome::Applied { .. }),
        "{outcome:?}"
    );
    assert!(hull.health_raw < health);
}

/// Retail lab (Level 1, F9, firing every weapon): at tick 1668 the player's
/// own blast reaches the craft first in the live list; 4484A0 hands the ball
/// 19516 and the craft takes 0, then the dying ball's own radial visit only
/// meets its buffer. The port's playing radial runs the same order.
#[v2k_test_support::retail_test]
fn a_blast_on_the_player_kills_the_ball_first_and_spares_the_craft() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(13);
    let mut tasks = super::super::queen_tests::adopt_world(&mut manager);
    let ball = birth_ball(&session, &mut manager, &mut fx, &mut tasks);
    let mut hull = hull_of(&manager);
    let health = hull.health_raw;
    let player = manager.player().unwrap();
    let (player_id, origin) = (player.id, player.position_raw());
    let outcome = tasks.apply_playing_radial_damage(
        crate::specialized_actor_task_production::PlayingRadialFrame {
            entities: &mut manager,
            player_hull: &mut hull,
            extra_lives: RetailRuntimeValue::Known(3),
            origin_raw: origin,
            template: crate::megablast::radial_template(46, player_id),
            world_fx: &mut fx,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            retail_tick: 1668,
            resources: &mut session.cache,
            static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
            active_terminal_calls: Vec::new(),
        },
    );
    assert_eq!(outcome.blocked, None);
    assert!(outcome.completed_target_ids.contains(&player_id));
    assert!(outcome.completed_target_ids.contains(&ball));
    assert_eq!(hull.health_raw, health);
    let body = manager.iter_all().find(|entity| entity.id == ball).unwrap();
    assert_eq!(body.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        body.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
        RetailRuntimeValue::Known(DYING_STATE_BIT)
    );
}

/// The Main Base abort's FUN_00410C10 on the hold's ball and a Megablast
/// pickup pod: each takes class2 (cue 62, tasks cleared, removal staged).
#[v2k_test_support::retail_test]
fn the_main_base_abort_kills_the_ball_and_a_pod_through_class2() {
    use crate::main_base_abort::{classify_main_base_abort_actor, MainBaseAbortActorRoute};
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(13);
    let mut tasks = super::super::queen_tests::adopt_world(&mut manager);
    let terrain = session.cache.terrain().unwrap().clone();
    let ball = birth_ball(&session, &mut manager, &mut fx, &mut tasks);
    let pod = crate::megablast::MegablastWorld {
        entities: &mut manager,
        scheduler: &mut tasks,
        terrain: &terrain,
        waves_enabled: false,
        world_fx: &mut fx,
        notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
        retail_tick: 791,
        player_rotation_raw: [0, 0, 0],
    }
    .pickup()
    .unwrap();
    for id in [ball, pod] {
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        let entity_type = entity.entity_type;
        let observation = manager.main_base_abort_actor_observation(id).unwrap();
        let route =
            classify_main_base_abort_actor(crate::main_base_abort::MainBaseAbortActorFacts {
                entity_id: id,
                entity_type,
                capability_flags: entity.capability_flags,
                state_flags: entity
                    .collision
                    .state_flags_at_0x08
                    .masked(crate::main_base_abort::ORDINARY_DEATH_STATE_SKIP_MASK),
            });
        assert_eq!(
            route,
            MainBaseAbortActorRoute::OrdinaryDeath { entity_id: id }
        );
        let advance = manager
            .apply_main_base_abort_quiet_death(observation.lease, &mut fx)
            .unwrap();
        assert!(
            matches!(
                advance,
                crate::entity::MainBaseAbortQuietDeathAdvance::Advanced {
                    outcome: crate::entity::MainBaseAbortQuietDeathOutcome::DeferredDestroyStaged {
                        death_sound_id: Some(62),
                        ..
                    },
                    ..
                }
            ),
            "Type{entity_type}: {advance:?}"
        );
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_task_state(slot).is_none()));
    }
}
