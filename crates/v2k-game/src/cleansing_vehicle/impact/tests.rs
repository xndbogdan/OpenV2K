use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    damage::{DamagePacket, FUN_0043F780_DAMAGE_DELIVERY},
    entity_collision_state::DEFERRED_DESTROY_PENDING_STATE_BIT,
    world_fx::BallisticDamageRequest,
};

fn impact(id: u32, infected: bool, packet: DamagePacket) -> ParticleEntityImpact {
    ParticleEntityImpact {
        source_particle_class: if infected { 5 } else { 55 },
        impact_position_argument_va: if infected { 0x004d_cf48 } else { 0 },
        target_entity_id: id,
        position_world: [0.; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(BallisticDamageRequest {
            packet,
            source_entity_type_at_birth: Some(46),
            source_owner_id: Some(0x04c9_0001),
        }),
    }
}

#[v2k_test_support::retail_test]
fn primary_and_infected_null_hooks_keep_current_rover_tasks_and_distinct_stamps() {
    for (infected, newant) in [(false, false), (true, false), (false, true)] {
        let (mut session, mut manager, mut fx) = super::super::tests::fixture(15);
        let id = manager.iter_all().find(|e| e.entity_type == 49).unwrap().id;
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.health_raw = RetailRuntimeValue::Known(10_000);
        entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
        let tasks =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot));
        let context = entity.current_behavior_context;
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.adopt_cleansing_vehicle(&manager);
        let mut notifications = GameplayNotifications::new();
        let mut static_damage = StaticDamageScheduler::new();
        let mut hull = PlayerHull::default();
        let packet = if infected {
            FUN_0043F780_DAMAGE_DELIVERY.packet
        } else if newant {
            DamagePacket {
                channels: [2, 6],
                amounts_raw: [1000, 1000],
            }
        } else {
            DamagePacket {
                channels: [2, 0],
                amounts_raw: [1000, 0],
            }
        };
        let mut hit = impact(id, infected, packet);
        if newant {
            hit.source_particle_class = 87;
            hit.damage.as_mut().unwrap().source_entity_type_at_birth = Some(47);
        }
        let result = apply_cleansing_vehicle_particle_hit(
            CleansingVehicleImpactFrame {
                extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                entities: &mut manager,
                resources: &mut session.cache,
                static_damage: &mut static_damage,
                notifications: &mut notifications,
                world_fx: &mut fx,
                scheduler: &mut scheduler,
                player_hull: &mut hull,
                retail_tick: 251,
            },
            hit,
        );
        let CleansingVehicleImpactOutcome::Applied(damage) = result else {
            panic!("{result:?}")
        };
        let expected_damage = if infected {
            500
        } else if newant {
            1050
        } else {
            800
        };
        assert_eq!(damage.filtered_damage_raw, expected_damage);
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(if infected { 17 } else { 251 })
        );
        assert_eq!(entity.current_behavior_context, context);
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot)),
            tasks
        );
        assert_eq!(
            entity.collision.health_raw,
            RetailRuntimeValue::Known(10_000 - expected_damage)
        );
        assert!(scheduler.begin_cleansing_vehicle_external_mutation(&manager, id));
    }
}

#[v2k_test_support::retail_test]
fn recorded_player_class55_hit_finishes_class49_before_a_second_physical_hit() {
    let (mut session, mut manager, mut fx) = super::super::tests::fixture(15);
    let id = manager.iter_all().find(|e| e.entity_type == 49).unwrap().id;
    // Keep this test's radial footprint away from unrelated authored actors
    // and terrain objects; the full Playing traversal still visits its source.
    let entity = manager.entity_mut(id).unwrap();
    entity.set_position_raw([0, -30_000, 0]);
    entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_cleansing_vehicle(&manager);
    let mut notifications = GameplayNotifications::new();
    let mut static_damage = StaticDamageScheduler::new();
    let mut hull = PlayerHull::default();
    for _ in 0..8 {
        fx.advance_frame_pacing(20_000);
    }
    // NoCD02 49FB4F:369, tick644: packet4CC0A8 from class55, player46.
    // 415040 computes4800: 2000 -> -2800, then10C10 zeroes health.
    let packet = DamagePacket {
        channels: [2, 3],
        amounts_raw: [3000, 1000],
    };
    let hit = impact(id, false, packet);
    let result = apply_cleansing_vehicle_particle_hit(
        CleansingVehicleImpactFrame {
            extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            entities: &mut manager,
            resources: &mut session.cache,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            world_fx: &mut fx,
            scheduler: &mut scheduler,
            player_hull: &mut hull,
            retail_tick: 0x644,
        },
        hit,
    );
    let CleansingVehicleImpactOutcome::Applied(damage) = result else {
        panic!("{result:?}")
    };
    assert_eq!(damage.filtered_damage_raw, 4800);
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        entity
            .collision
            .state_flags_at_0x08
            .masked(DYING_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT),
        RetailRuntimeValue::Known(DYING_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT)
    );
    assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .all(|slot| entity.actor_tasks.task_in_slot(slot).is_none()));
    assert!(finished_terminal_hit_authenticates(&manager, id));
    assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
    let ring_count = manager.iter_all().filter(|e| e.entity_type == 60).count();
    assert_eq!(ring_count, 1);
    let particles = fx
        .prepare_presentation([640, 480], 0x3000, |_| {
            v2k_render::ParticleCenterProjection {
                screen: [320, 240],
                depth_raw: 1000,
                clip: 0,
            }
        })
        .particles()
        .map(|p| p.particle)
        .collect::<Vec<_>>();
    assert_eq!(
        particles.iter().filter(|p| p.source_class == 37).count(),
        10
    );
    let before_fx = fx.particle_count();
    let result = apply_cleansing_vehicle_particle_hit(
        CleansingVehicleImpactFrame {
            extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            entities: &mut manager,
            resources: &mut session.cache,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            world_fx: &mut fx,
            scheduler: &mut scheduler,
            player_hull: &mut hull,
            retail_tick: 0x645,
        },
        hit,
    );
    assert!(
        matches!(result, CleansingVehicleImpactOutcome::Applied(_)),
        "{result:?}"
    );
    assert_eq!(fx.particle_count(), before_fx);
    assert_eq!(
        manager.iter_all().filter(|e| e.entity_type == 60).count(),
        ring_count
    );
}
