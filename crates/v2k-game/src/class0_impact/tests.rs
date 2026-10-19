//! Class0 hits on real worlds, against the retail lab's Level 1 grock.

use super::*;
use crate::{
    entity::Class0ActorOutcome,
    gameplay_notifications::GameplayNotifications,
    shared_actor_impact::{apply_playing_actor_particle_hit, SharedActorImpactOutcome},
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskWorld,
    },
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

// Less the per-frame detail bits and the birth's surface pair.
const MASK: u32 = !(0x0600_0000 | crate::entity_collision_state::SURFACE_STATE_MASK);

fn first(manager: &EntityManager, entity_type: u32) -> u32 {
    manager
        .iter_all()
        .find(|entity| entity.entity_type == entity_type)
        .unwrap()
        .id
}

fn entity(manager: &EntityManager, id: u32) -> &Entity {
    manager.iter_all().find(|entity| entity.id == id).unwrap()
}

fn hit(id: u32, channel: i32, amount_raw: i32) -> ParticleEntityImpact {
    ParticleEntityImpact {
        source_particle_class: 16,
        impact_position_argument_va: 0,
        target_entity_id: id,
        position_world: [0.0; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(crate::world_fx::BallisticDamageRequest {
            packet: crate::damage::DamagePacket {
                channels: [channel, 0],
                amounts_raw: [amount_raw, 0],
            },
            source_entity_type_at_birth: Some(46),
            // The player's own shot.
            source_owner_id: Some(1),
        }),
    }
}

fn tick(
    tasks: &mut SpecializedActorTaskScheduler,
    manager: &mut EntityManager,
    session: &mut crate::session::GameSession,
    fx: &mut WorldFx,
    tick: u32,
) -> Vec<SpecializedActorTaskProductionOutcome> {
    let pass = tasks.tick(
        manager,
        SpecializedActorTaskProductionFrame {
            world: SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase: crate::gameplay_notifications::GameplayNotificationPhase::Playing,
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

fn shoot(
    session: &mut crate::session::GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    tasks: &mut SpecializedActorTaskScheduler,
    impact: ParticleEntityImpact,
    retail_tick: u32,
) -> Class0ImpactOutcome {
    match apply_playing_actor_particle_hit(
        PlayingActorImpactFrame {
            resources: &mut session.cache,
            entities: manager,
            world_fx: fx,
            scheduler: tasks,
            notifications: &mut GameplayNotifications::new(),
            static_damage: &mut StaticDamageScheduler::new(),
            player_hull: &mut crate::player_hull::PlayerHull::default(),
            extra_lives: RetailRuntimeValue::Known(3),
            retail_tick,
        },
        impact,
    ) {
        Some(SharedActorImpactOutcome::Class0(outcome)) => outcome,
        other => panic!("{other:?}"),
    }
}

/// Level 1, coarse, at the retail lab's pre-hit state `0x00428805`.
fn level1_grock() -> (
    crate::session::GameSession,
    EntityManager,
    WorldFx,
    SpecializedActorTaskScheduler,
    u32,
) {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(13);
    let id = first(&manager, 54);
    assert!(manager.native_class0_allocation_authenticates(id));
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x0600_0000, 0);
    let mut tasks = SpecializedActorTaskScheduler::new();
    tasks.adopt_class0_actors(&manager);
    let mut called = false;
    for t in 1..=200 {
        called = tick(&mut tasks, &mut manager, &mut session, &mut fx, t)
            .iter()
            .any(|outcome| {
                matches!(outcome, SpecializedActorTaskProductionOutcome::Class0Actor(
                    Class0ActorOutcome::Advanced { entity_id, callback_enabled: true, .. })
                    if *entity_id == id)
            });
        if called {
            break;
        }
    }
    assert!(called);
    assert_eq!(
        entity(&manager, id)
            .collision
            .state_flags_at_0x08
            .masked(MASK),
        RetailRuntimeValue::Known(0x0042_8805 & MASK)
    );
    (session, manager, fx, tasks, id)
}

/// Retail's lethal 14E90 on the Level 1 grock reads `0x0042C005`, health 0,
/// then class38 lowers the sea word from `0xFFFCB100` to `0xFFFA2000`.
#[v2k_test_support::retail_test]
fn a_lethal_hit_on_the_level1_grock_lowers_the_sea_as_retail() {
    let (mut session, mut manager, mut fx, mut tasks, id) = level1_grock();
    assert_eq!(
        session.cache.terrain().unwrap().header[0] as u32,
        0xFFFC_B100
    );
    let outcome = shoot(
        &mut session,
        &mut manager,
        &mut fx,
        &mut tasks,
        hit(id, 2, 100_000),
        300,
    );
    let Class0ImpactOutcome::Applied(applied) = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(
        applied.death_publication,
        Some(Class0Death::SeaLevel {
            sea_delta_raw: -168_192
        })
    );
    let grock = entity(&manager, id);
    assert_eq!(grock.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        grock.collision.state_flags_at_0x08.masked(MASK),
        RetailRuntimeValue::Known(0x0042_C005 & MASK)
    );
    assert!(!tasks.prepare_class0_actor_external_mutation(&manager, id));
    let mut finished = None;
    for t in 301..=1300 {
        let outcomes = tick(&mut tasks, &mut manager, &mut session, &mut fx, t);
        if outcomes.iter().any(|outcome| {
            matches!(
                outcome,
                SpecializedActorTaskProductionOutcome::MainBaseType54SeaLevel(
                    crate::main_base_type54_abort::MainBaseType54SeaLevelProductionOutcome::DeferredDestroyStaged { .. }
                )
            )
        }) {
            finished = Some(t);
            break;
        }
    }
    // Retail's sea writes run from the hit's own pass 1300 through 1989.
    assert_eq!(finished, Some(300 + 690), "the sea's 690 paced updates");
    assert_eq!(
        session.cache.terrain().unwrap().header[0] as u32,
        0xFFFA_2000
    );
    assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
}

/// A light hit stamps `+34` and takes the filtered damage; the coarse grock
/// is not jolted, the detailed one is, and its class0 owner stays.
#[v2k_test_support::retail_test]
fn a_light_hit_damages_the_grock_and_jolts_it_only_when_detailed() {
    for detailed in [false, true] {
        let (mut session, mut manager, mut fx, mut tasks, id) = level1_grock();
        if detailed {
            manager
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(0x0400_0000, 0x0400_0000);
        }
        let velocity = entity(&manager, id).velocity_raw();
        let mut control = fx.fork_for_main_base_abort_transaction();
        let outcome = shoot(
            &mut session,
            &mut manager,
            &mut fx,
            &mut tasks,
            hit(id, 2, 500),
            300,
        );
        let Class0ImpactOutcome::Applied(applied) = outcome else {
            panic!("{outcome:?}");
        };
        assert_eq!(applied.filtered_damage_raw, 300);
        assert_eq!(applied.death_publication, None);
        let grock = entity(&manager, id);
        assert_eq!(grock.collision.health_raw, RetailRuntimeValue::Known(700));
        assert_eq!(
            grock.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(300)
        );
        assert_eq!(grock.velocity_raw() != velocity, detailed);
        // 11030 draws its three spin words only when it jolts.
        let draws = if detailed { 3 } else { 0 };
        for _ in 0..draws {
            control.next_shared_retail_random_u16();
        }
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            control.next_shared_retail_random_u16()
        );
        assert!(tasks.prepare_class0_actor_external_mutation(&manager, id));
    }
}

/// Flags take no damage: every multiplier is zero.
#[v2k_test_support::retail_test]
fn a_flag_shrugs_off_a_hit() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(13);
    let id = first(&manager, 52);
    let mut tasks = SpecializedActorTaskScheduler::new();
    tasks.adopt_class0_actors(&manager);
    let health = entity(&manager, id).collision.health_raw;
    let outcome = shoot(
        &mut session,
        &mut manager,
        &mut fx,
        &mut tasks,
        hit(id, 2, 100_000),
        5,
    );
    let Class0ImpactOutcome::Applied(applied) = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(applied.filtered_damage_raw, 0);
    assert_eq!(entity(&manager, id).collision.health_raw, health);
    assert!(tasks.prepare_class0_actor_external_mutation(&manager, id));
}
