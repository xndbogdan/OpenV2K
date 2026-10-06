//! 12760 physical suffix with genuine ground allocations and bare15040 delivery.
use super::*;
use crate::{
    gameplay_notifications::GameplayNotifications,
    native_ground_actor::NativeGroundTaskCustody,
    player_active_contact::{
        classify_oriented_active_pair_contact, OrientedActivePairContactRequest,
    },
    player_hull::{HullDamageProfile, PlayerHull},
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    split_and_explode::SplitChildRequest,
    static_damage::StaticDamageScheduler,
};

fn lethal_physical_pair(kind: u32, playing: bool) {
    let (mut session, mut entities, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(17);
    let mut tasks = SpecializedActorTaskScheduler::new();
    let pair = if kind == 40 {
        let ids = entities
            .iter_all()
            .filter(|e| e.entity_type == 40)
            .map(|e| e.id)
            .collect::<Vec<_>>();
        assert_eq!(ids.len(), 2, "canonical Alpine authored parents");
        assert_eq!(tasks.adopt_type40(&entities), 2);
        [ids[0], ids[1]]
    } else {
        std::array::from_fn(|_| {
            let birth = entities
                .construct_native_type56(
                    SplitChildRequest {
                        requested_entity_handle_raw: 0,
                        entity_type: 56,
                        position_raw: [4096, 6000, 4096],
                        objective: false,
                        velocity_raw: [0; 3],
                        rotation_heading_pitch_roll_raw: [0; 3],
                    },
                    &session.cache,
                    &mut fx,
                    4793,
                )
                .unwrap();
            let id = birth.owner.entity_id();
            tasks.register_split_type56_child(birth.owner).unwrap();
            id
        })
    };
    let center = [4096i16, 6000, 4096];
    let ids = entities.retail_live_order_ids().collect::<Vec<_>>();
    for id in ids {
        entities.entity_mut(id).unwrap().set_motion_raw(
            if pair.contains(&id) {
                center
            } else {
                [16000, 16000, 16000]
            },
            [0; 3],
        );
    }
    let mut contact = None;
    'search: for dy in [-150i16, -75, 0, 75, 150] {
        for dx in [-240i16, -160, -80, 0, 80, 160, 240] {
            for dz in [-240i16, -160, -80, 0, 80, 160, 240] {
                entities.entity_mut(pair[1]).unwrap().set_position_raw([
                    center[0].wrapping_add(dx),
                    center[1].wrapping_add(dy),
                    center[2].wrapping_add(dz),
                ]);
                let subject = actor(&entities, pair[0]).unwrap();
                let entry = active_pair_body_from_entity(subject, &session.cache);
                let found = classify_oriented_active_pair_contact(
                    OrientedActivePairContactRequest {
                        subject,
                        candidate: actor(&entities, pair[1]).unwrap(),
                        subject_entry: &entry,
                        retail_tick: 4794,
                    },
                    &session.cache,
                )
                .unwrap();
                if let Some(found) = found.filter(|c| c.penetration_raw >= 32) {
                    contact = Some(found);
                    break 'search;
                }
            }
        }
    }
    let contact = contact.expect("actual retained bases and canonical solids overlap");
    entities
        .entity_mut(pair[0])
        .unwrap()
        .set_velocity_raw([4096, 0, 0]);
    entities
        .entity_mut(pair[1])
        .unwrap()
        .set_velocity_raw([-4096, 0, 0]);
    let before_positions = pair.map(|id| actor(&entities, id).unwrap().position_raw());
    let before_ticks = pair.map(|id| {
        actor(&entities, id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34
    });
    let plan = plan_active_pair_response_and_damage_cap(
        active_pair_body_from_entity(actor(&entities, pair[0]).unwrap(), &session.cache),
        active_pair_body_from_entity(actor(&entities, pair[1]).unwrap(), &session.cache),
        contact,
    )
    .unwrap();
    for id in pair {
        assert!(tasks.prepare_native_actor_mutation(&entities, id));
        let e = actor(&entities, id).unwrap();
        let RetailRuntimeValue::Known(health) = e.collision.health_raw else {
            panic!("native health")
        };
        assert_eq!(health, if kind == 40 { 8000 } else { 6000 });
        assert!(
            entities
                .type_runtime_metadata(kind)
                .unwrap()
                .damage_profile
                .unwrap()
                .filter(crate::damage::DamagePacket::collision(
                    plan.capped_pair_damage_raw
                ))
                >= health
        );
    }
    let mut hull = PlayerHull::new(HullDamageProfile::from_type_record(
        session.cache.global_entity_type(46).unwrap(),
    ));
    hull.readback_entity_damage_state(&entities.player().unwrap().collision);
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut committed = false;
    let mut stages = Vec::new();
    physical(
        &mut Intro2ContactFrame {
            entities: &mut entities,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: 4794,
            actor_tasks: &mut tasks,
        },
        pair[0],
        pair[1],
        contact,
        playing.then_some(PlayingPlayerContact {
            hull: &mut hull,
            extra_lives: RetailRuntimeValue::Known(3),
        }),
        &mut committed,
        &mut stages,
    )
    .expect("native checked terminal at the physical suffix");
    assert!(committed);
    assert_eq!(
        stages,
        vec![NativeCaptorPairStage::Physical {
            capped_damage_raw: plan.capped_pair_damage_raw,
        }]
    );
    assert_ne!(
        pair.map(|id| actor(&entities, id).unwrap().position_raw()),
        before_positions
    );
    assert_eq!(
        pair.map(|id| actor(&entities, id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34),
        before_ticks,
        "bare15040 must not stamp a10EB0 particle hit"
    );
    for id in pair {
        assert_eq!(
            actor(&entities, id).unwrap().collision.health_raw,
            RetailRuntimeValue::Known(0)
        );
        assert!(tasks.family_for(id).is_none());
        assert!(if kind == 40 {
            crate::native_type40::death::finished_terminal_authenticates(&entities, id)
        } else {
            crate::native_type56::death::finished_terminal_authenticates(&entities, id)
        });
    }
    assert_eq!(
        entities.iter_all().filter(|e| e.entity_type == 56).count(),
        if kind == 40 { 16 } else { 2 }
    );
}

#[v2k_test_support::retail_test]
fn type40_physical_lethal_suffix_uses_inline_class18_in_both_world_contexts() {
    for playing in [false, true] {
        lethal_physical_pair(40, playing);
    }
}

#[v2k_test_support::retail_test]
fn type56_physical_lethal_suffix_uses_quiet_class2_in_both_world_contexts() {
    for playing in [false, true] {
        lethal_physical_pair(56, playing);
    }
}
