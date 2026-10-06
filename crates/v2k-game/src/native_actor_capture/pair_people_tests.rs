//! Native person/worker 15040 pair deaths and pre-callback body custody.

use super::*;
use crate::{
    damage::{DamageDeliveryRecord, DamagePacket},
    gameplay_notifications::GameplayNotifications,
    intro2_type17::pair::tests::Fixture,
    native_type122::construction_tests::native_fixture,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
};

#[v2k_test_support::retail_test]
fn native_worker_and_person_pair_packets_publish_their_own_class14_death() {
    // The real 11760 packet consumer is entered after the physical cap. No
    // projectile wrapper, synthesized actor or borrowed receipt is involved.
    for (world, entity_type) in [
        (25, 8),
        (19, 79),
        (24, 90),
        (40, 91),
        (42, 116),
        (17, 78),
        (24, 86),
        (16, 95),
        (49, 123),
    ] {
        let (mut session, mut entities, mut fx) = native_fixture(world);
        let id = entities
            .iter_all()
            .find(|e| e.entity_type == entity_type)
            .unwrap()
            .id;
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.adopt_intro2_type8(&mut entities);
        scheduler.adopt_native_type86(&mut entities);
        scheduler.adopt_native_type123(&mut entities);
        assert!(scheduler.prepare_native_actor_mutation(&entities, id));
        let e = entities.entity_mut(id).unwrap();
        e.collision.state_flags_at_0x08.overwrite(0x8000, 0x8000);
        let hit_tick = e.collision.last_hit_presentation_tick_at_0x34;
        let models = e.model_slots;
        let packet = DamagePacket::collision(10_000);
        let RetailRuntimeValue::Known(profile) = e.collision.damage_profile else {
            panic!("canonical damage")
        };
        let RetailRuntimeValue::Known(health) = e.collision.health_raw else {
            panic!("native health")
        };
        assert!(packet.filtered_raw(Some(&profile)) >= health);
        let mut notifications = GameplayNotifications::new();
        let mut static_damage = StaticDamageScheduler::new();
        apply_pair_checked_damage(
            &mut Intro2ContactFrame {
                entities: &mut entities,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                notifications: &mut notifications,
                retail_tick: 5000,
                actor_tasks: &mut scheduler,
            },
            id,
            DamageDeliveryRecord {
                packet,
                source_entity_type_raw: 122,
                owner_handle: 0,
            },
            None,
            &mut false,
        )
        .unwrap_or_else(|error| panic!("world{world} type{entity_type}: {error:?}"));
        let e = actor(&entities, id).unwrap();
        assert_eq!(e.entity_type, entity_type);
        assert_eq!(e.model_slots, models);
        assert_eq!(e.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            e.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
        assert_eq!(
            e.collision.last_hit_presentation_tick_at_0x34, hit_tick,
            "15040 does not enter 10EB0"
        );
        assert_eq!(style_address(e).unwrap(), 0x004c_70c0);
        assert!(
            matches!(e.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::SharedRetarget(task)) if task.lifetime_ms() == 1000)
        );
        assert!(scheduler.prepare_native_actor_mutation(&entities, id));
    }
}

fn person_pair() -> Fixture {
    let (session, mut entities, fx) = native_fixture(24);
    entities.cleanup_pending_actor_deferred_destroys();
    let spider = entities
        .iter_all()
        .find(|e| e.entity_type == 122)
        .unwrap()
        .id;
    let child = entities
        .iter_all()
        .find(|e| e.entity_type == 86)
        .unwrap()
        .id;
    let center = [
        32720,
        session.cache.terrain().unwrap().sea_level_raw() + 6000,
        16000,
    ];
    for id in [spider, child] {
        let entity = entities.entity_mut(id).unwrap();
        entity.set_motion_raw(center, [0; 3]);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x8000, 0x8000);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    }
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_type122(&entities);
    scheduler.adopt_native_type86(&mut entities);
    let order: Vec<_> = entities.retail_live_order_ids().collect();
    let (subject, candidate) =
        if order.iter().position(|id| *id == spider) < order.iter().position(|id| *id == child) {
            (spider, child)
        } else {
            (child, spider)
        };
    let mut f = Fixture {
        session,
        entities,
        scheduler,
        fx,
        notifications: GameplayNotifications::new(),
        static_damage: StaticDamageScheduler::new(),
        spider,
        child,
        subject,
        candidate,
        tick: 5000,
    };
    f.place_oriented_contact();
    f
}

#[v2k_test_support::retail_test]
fn native_person_pair_rejects_foreign_or_pending_owner_before_any_callback_write() {
    for foreign in [true, false] {
        let mut f = person_pair();
        assert!(requires_body_custody(actor(&f.entities, f.child).unwrap()));
        assert!(f
            .scheduler
            .prepare_native_actor_mutation(&f.entities, f.child));
        if foreign {
            let (_, other, _) = native_fixture(24);
            f.entities
                .entity_mut(f.child)
                .unwrap()
                .native_type86_runtime = actor(&other, f.child).unwrap().native_type86_runtime;
        } else {
            f.scheduler.park_native_type86_external_prefix(f.child);
        }
        let snapshot = |f: &Fixture| {
            format!(
                "{:?}",
                (
                    [f.spider, f.child].map(|id| {
                        let e = actor(&f.entities, id).unwrap();
                        (
                            e.position_raw(),
                            e.velocity_raw(),
                            e.collision.clone(),
                            e.current_behavior_context,
                            e.actor_animation_runtime,
                            e.sub_a_propulsion_runtime,
                            format!("{:?}", e.actor_tasks),
                        )
                    }),
                    &f.fx,
                    &f.notifications
                )
            )
        };
        let before = snapshot(&f);
        let result = f.pair(CaptureFeedbackPolicy::Gameplay);
        assert!(
            matches!(result, NativeCaptorPairOutcome::Blocked {
            reason: NativeCaptorPairBlock::BodyCustody { entity_id },
            committed_prefix: false, ref visits,
        } if entity_id == f.child && visits.is_empty()),
            "{result:?}"
        );
        assert_eq!(
            snapshot(&f),
            before,
            "no callback, health, physical, audio or RNG prefix"
        );
    }
}
