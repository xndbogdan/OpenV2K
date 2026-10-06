//! Actual aquatic cohorts enter14AE0 with current native task/allocation custody.

use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    entity_collision_state::{RetailRuntimeValue, CHECKED_DAMAGE_ENABLED_STATE_BIT},
    gameplay_notifications::GameplayNotificationPhase,
    native_type122::construction_tests::native_fixture_with_player,
    session::GameSession,
    shared_fish::{death::SharedFishDeathBlock, SharedFishOutcome},
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
    },
    static_damage::StaticDamageScheduler,
};

struct Fixture {
    session: GameSession,
    manager: EntityManager,
    scheduler: SpecializedActorTaskScheduler,
    fx: WorldFx,
    notifications: GameplayNotifications,
    hull: PlayerHull,
}

fn fixture(world: u32) -> Fixture {
    let (session, mut manager, fx) = native_fixture_with_player(world);
    manager.cleanup_pending_actor_deferred_destroys();
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert!(scheduler.adopt_shared_fish(&manager) > 0);
    Fixture {
        session,
        manager,
        scheduler,
        fx,
        notifications: GameplayNotifications::new(),
        hull: PlayerHull::default(),
    }
}

fn template() -> RadialDamageTemplate {
    RadialDamageTemplate {
        inner_radius_raw: 1000,
        outer_radius_raw: 2000,
        impulse_raw: 400,
        packet: DamagePacket {
            channels: [1, 0],
            amounts_raw: [100_000, 0],
        },
        trailing_raw: [-5, 0],
    }
}

fn isolate(f: &mut Fixture, id: u32) {
    let ids: Vec<_> = f.manager.iter_all().map(|entity| entity.id).collect();
    for current in ids {
        f.manager
            .entity_mut(current)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(
                CHECKED_DAMAGE_ENABLED_STATE_BIT,
                if current == id {
                    CHECKED_DAMAGE_ENABLED_STATE_BIT
                } else {
                    0
                },
            );
    }
}

fn deliver(f: &mut Fixture, origin_raw: [i16; 3]) -> PlayingRadialOutcome {
    f.scheduler.apply_playing_radial_damage(PlayingRadialFrame {
        entities: &mut f.manager,
        player_hull: &mut f.hull,
        extra_lives: RetailRuntimeValue::Known(2),
        origin_raw,
        template: template(),
        world_fx: &mut f.fx,
        notifications: &mut f.notifications,
        retail_tick: 2000,
        resources: &mut f.session.cache,
        static_damage: &mut StaticDamageScheduler::new(),
        active_terminal_calls: Vec::new(),
    })
}

#[v2k_test_support::retail_test]
fn playing_radial_quiet_death_covers_every_authored_class2_fish_after_live_visits() {
    let mut census = Vec::new();
    for world in [13, 14, 18, 22, 23, 30, 34, 36] {
        let mut f = fixture(world);
        let victims: Vec<_> = f
            .manager
            .iter_all()
            .filter(|entity| matches!(entity.entity_type, 22 | 23 | 24 | 62))
            .map(|entity| entity.id)
            .collect();
        let survivors: Vec<_> = f
            .manager
            .iter_all()
            .filter(|entity| entity.entity_type == 124)
            .map(|entity| entity.id)
            .collect();
        for tick in 1001..=1010 {
            let outcome = f.scheduler.tick(
                &mut f.manager,
                SpecializedActorTaskProductionFrame {
                    world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase: GameplayNotificationPhase::Playing,
                    resources: &mut f.session.cache,
                    world_fx: &mut f.fx,
                    static_damage: &mut StaticDamageScheduler::new(),
                    elapsed_micros: 20_000,
                    global_elapsed_micros: 20_000,
                    retail_tick: tick,
                    main_base_abort_active: false,
                },
                &mut f.notifications,
            );
            assert!(outcome.block.is_none(), "world{world}: {outcome:?}");
            assert!(outcome.outcomes.iter().all(|outcome| matches!(
                outcome,
                SpecializedActorTaskProductionOutcome::SharedFish(
                    SharedFishOutcome::Advanced { .. } | SharedFishOutcome::Waiting { .. }
                )
            )));
        }
        for &id in &victims {
            isolate(&mut f, id);
            let entity = f.manager.entity_mut(id).unwrap();
            let position = entity.position_raw();
            let stamp = entity.collision.last_hit_presentation_tick_at_0x34;
            let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
            let outcome = deliver(&mut f, position);
            assert!(
                outcome.blocked.is_none(),
                "world{world} fish{id}: {outcome:?}"
            );
            assert_eq!(outcome.completed_target_ids, vec![id]);
            assert_eq!(outcome.accepted_targets, 1);
            assert!(crate::shared_fish::death::completed_shared_fish_death(
                &f.manager, id
            ));
            assert_eq!(f.scheduler.family_for(id), None);
            let entity = f.manager.entity_mut(id).unwrap();
            assert_eq!(entity.position_raw(), position);
            assert_eq!(entity.collision.last_hit_presentation_tick_at_0x34, stamp);
            assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
                .into_iter()
                .all(|slot| entity.actor_tasks.task_in_slot(slot).is_none()));
            assert_eq!(
                f.fx.next_shared_retail_random_u16(),
                expected_fx.next_shared_retail_random_u16()
            );
        }
        assert_eq!(f.manager.pending_actor_deferred_destroy_ids(), victims);
        assert_eq!(f.manager.cleanup_pending_actor_deferred_destroys(), victims);
        assert!(survivors
            .iter()
            .all(|id| f.scheduler.family_for(*id).is_some()));
        census.push((world, victims.len(), survivors.len()));
    }
    eprintln!("radial fish (world, class2, class63) census: {census:?}");
    assert_eq!(census.iter().map(|row| row.1 + row.2).sum::<usize>(), 114);
}

#[v2k_test_support::retail_test]
fn playing_radial_fish_falloff_retains_impulse_and_surviving_owner_without_reselection() {
    for kind in [22, 23, 24, 62] {
        let mut f = fixture(if kind == 62 { 13 } else { 30 });
        let id = f
            .manager
            .iter_all()
            .find(|entity| entity.entity_type == kind)
            .unwrap()
            .id;
        isolate(&mut f, id);
        let entity = f.manager.entity_mut(id).unwrap();
        let position = entity.position_raw();
        let velocity = entity.velocity_raw();
        let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
        let context = entity.current_behavior_context;
        let RetailRuntimeValue::Known(profile) = entity.collision.damage_profile else {
            panic!()
        };
        entity.collision.health_raw = RetailRuntimeValue::Known(1_000_000);
        let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
        let outcome = deliver(
            &mut f,
            [position[0].wrapping_sub(1500), position[1], position[2]],
        );
        assert!(outcome.blocked.is_none(), "kind{kind}: {outcome:?}");
        let entity = f.manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.velocity_raw(),
            [velocity[0].wrapping_add(200), velocity[1], velocity[2]]
        );
        assert_eq!(
            entity.collision.health_raw,
            RetailRuntimeValue::Known(
                1_000_000
                    - profile.filter(DamagePacket {
                        amounts_raw: [50_000, 0],
                        ..template().packet
                    })
            )
        );
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            primary
        );
        assert_eq!(entity.current_behavior_context, context);
        assert!(f.scheduler.shared_fish_completed_owner(&f.manager, id));
        assert!(f.manager.pending_actor_deferred_destroy_ids().is_empty());
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected_fx.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn playing_radial_fish_requires_live_scheduler_and_pending_prefix_custody_before_impulse() {
    for invalid in 0..3 {
        let mut f = fixture(30);
        let id = f
            .manager
            .iter_all()
            .find(|entity| entity.entity_type == 22)
            .unwrap()
            .id;
        isolate(&mut f, id);
        match invalid {
            0 => f.scheduler.retire_shared_fish(id),
            1 => {
                f.manager
                    .entity_mut(id)
                    .unwrap()
                    .shared_fish_runtime
                    .as_mut()
                    .unwrap()
                    .impact_prefix_pending = true
            }
            _ => {
                let foreign = fixture(30);
                f.manager.entity_mut(id).unwrap().shared_fish_runtime = foreign
                    .manager
                    .iter_all()
                    .find(|entity| entity.id == id)
                    .unwrap()
                    .shared_fish_runtime
                    .clone();
            }
        }
        let entity = f.manager.entity_mut(id).unwrap();
        let position = entity.position_raw();
        let before = (
            entity.collision.clone(),
            entity.velocity_raw(),
            entity.current_behavior_context,
        );
        let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
        let outcome = deliver(
            &mut f,
            [position[0].wrapping_sub(1500), position[1], position[2]],
        );
        assert!(
            matches!(
                outcome.blocked,
                Some(PlayingRadialBlock::Native(DynamicRadialLiveBlock {
                    target_prefix_committed: false,
                    reason: crate::entity::DynamicRadialLiveBlockReason::NativeActorMutationCustody,
                    ..
                }))
            ),
            "{outcome:?}"
        );
        let entity = f.manager.entity_mut(id).unwrap();
        assert_eq!(
            (
                entity.collision.clone(),
                entity.velocity_raw(),
                entity.current_behavior_context
            ),
            before
        );
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected_fx.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn playing_radial_completed_fish_uses_its_buffer_until_sweep_but_requires_terminal_receipt() {
    let mut f = fixture(30);
    let id = f
        .manager
        .iter_all()
        .find(|entity| entity.entity_type == 22)
        .unwrap()
        .id;
    isolate(&mut f, id);
    let position = f.manager.entity_mut(id).unwrap().position_raw();
    assert!(deliver(&mut f, position).blocked.is_none());
    assert_eq!(f.scheduler.family_for(id), None);
    let entity = f.manager.entity_mut(id).unwrap();
    let context = entity.current_behavior_context;
    let RetailRuntimeValue::Known(profile) = entity.collision.damage_profile else {
        panic!()
    };
    entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(200_000);
    let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
    let repeated = deliver(&mut f, position);
    assert!(repeated.blocked.is_none(), "{repeated:?}");
    assert_eq!(repeated.completed_target_ids, vec![id]);
    let entity = f.manager.entity_mut(id).unwrap();
    assert_eq!(
        entity.collision.pre_health_damage_buffer_raw,
        RetailRuntimeValue::Known(200_000 - profile.filter(template().packet))
    );
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(entity.current_behavior_context, context);
    entity
        .shared_fish_runtime
        .as_mut()
        .unwrap()
        .quiet_death_context = None;
    let buffer = entity.collision.pre_health_damage_buffer_raw;
    let forged = deliver(&mut f, position);
    assert!(
        matches!(
            forged.blocked,
            Some(PlayingRadialBlock::Native(DynamicRadialLiveBlock {
                target_prefix_committed: false,
                reason: crate::entity::DynamicRadialLiveBlockReason::NativeActorMutationCustody,
                ..
            }))
        ),
        "{forged:?}"
    );
    assert_eq!(
        f.manager
            .entity_mut(id)
            .unwrap()
            .collision
            .pre_health_damage_buffer_raw,
        buffer
    );
    assert_eq!(
        f.fx.next_shared_retail_random_u16(),
        expected_fx.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn playing_radial_type124_keeps_the_lethal_prefix_and_named_class63_boundary() {
    let mut f = fixture(30);
    let id = f
        .manager
        .iter_all()
        .find(|entity| entity.entity_type == 124)
        .unwrap()
        .id;
    isolate(&mut f, id);
    let entity = f.manager.entity_mut(id).unwrap();
    let position = entity.position_raw();
    let context = entity.current_behavior_context;
    entity.collision.health_raw = RetailRuntimeValue::Known(1);
    let outcome = deliver(&mut f, position);
    assert!(
        matches!(outcome.blocked, Some(PlayingRadialBlock::Native(DynamicRadialLiveBlock {
            target_prefix_committed: true,
            reason: crate::entity::DynamicRadialLiveBlockReason::Checked(ref checked), ..
        })) if matches!(checked.reason, crate::live_actor_checked_damage::LiveActorDamageBlock::Death(
            crate::entity::DynamicRadialLiveBlockReason::Fish(SharedFishDeathBlock::UnsupportedDeathProgram {
                entity_type:124, alternate_behavior_class:63
            })))),
        "{outcome:?}"
    );
    let entity = f.manager.entity_mut(id).unwrap();
    assert!(matches!(entity.collision.health_raw, RetailRuntimeValue::Known(health) if health < 0));
    assert_eq!(entity.current_behavior_context, context);
    assert!(f.manager.pending_actor_deferred_destroy_ids().is_empty());
    assert!(f.scheduler.has_native_contact_prefix(id));
}
