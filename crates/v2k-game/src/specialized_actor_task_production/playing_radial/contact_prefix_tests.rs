use super::*;
use crate::{
    damage::DamagePacket, entity_collision_state::RetailRuntimeValue,
    intro2_radial::Intro2RadialTaskCustody, shared_actor_impact::type9_tests::initial_attract,
};

#[v2k_test_support::retail_test]
fn same_origin_radial_preserves_parked_peasant_and_allows_identity_noop() {
    for damage_raw in [2_100, 1_999] {
        let (mut world, id) = initial_attract();
        let entity = world
            .entities
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap();
        let origin_raw = entity.position_raw();
        let before_collision = entity.collision.clone();
        let before_velocity = entity.velocity_raw();
        assert_eq!(
            before_collision.health_raw,
            RetailRuntimeValue::Known(1_500)
        );
        assert_eq!(
            before_collision.pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(0)
        );
        let template = RadialDamageTemplate {
            inner_radius_raw: 0,
            outer_radius_raw: 1,
            impulse_raw: 256,
            packet: DamagePacket::collision(damage_raw),
            trailing_raw: [17, 0],
        };
        let scaled =
            crate::radial_damage::scale_radial_damage(template, origin_raw, origin_raw, true)
                .expect("same-origin packet is inside the strict radius");
        assert_eq!(scaled.impulse_vector_raw, Some([0; 3]));
        let RetailRuntimeValue::Known(profile) = before_collision.damage_profile else {
            panic!()
        };
        assert_eq!(
            scaled.packet.filtered_raw(Some(&profile)),
            if damage_raw == 2_100 { 100 } else { 0 }
        );
        assert!(world
            .scheduler
            .park_native_contact_prefix(&world.entities, id));
        let before_owner = format!(
            "{:?}",
            world
                .scheduler
                .owners
                .iter()
                .find(|owner| owner.entity_id() == id)
                .unwrap()
        );
        let mut expected_fx = world.fx.fork_for_main_base_abort_transaction();
        let before_notifications = format!("{:?}", world.notifications);
        let mut hull = PlayerHull::default();
        let result = world
            .scheduler
            .apply_playing_radial_damage(PlayingRadialFrame {
                resources: &mut world.session.cache,
                static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
                active_terminal_calls: Vec::new(),
                entities: &mut world.entities,
                player_hull: &mut hull,
                extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                origin_raw,
                template,
                world_fx: &mut world.fx,
                notifications: &mut world.notifications,
                retail_tick: world.tick,
            });
        if damage_raw == 2_100 {
            assert_eq!(
                result.blocked,
                Some(PlayingRadialBlock::Retained(DynamicRadialUnresolved {
                    target_id: id,
                    reason: DynamicRadialUnresolvedReason::SelectedType9MutationCustody,
                }))
            );
            assert_eq!(result.accepted_targets, 0);
        } else {
            assert_eq!(
                result.blocked, None,
                "the filtered-zero identity plan has no live effect"
            );
            assert_eq!(result.accepted_targets, 1);
        }
        let entity = world
            .entities
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap();
        assert_eq!(entity.position_raw(), origin_raw);
        assert_eq!(entity.velocity_raw(), before_velocity);
        assert_eq!(entity.collision, before_collision);
        assert_eq!(
            format!(
                "{:?}",
                world
                    .scheduler
                    .owners
                    .iter()
                    .find(|owner| owner.entity_id() == id)
                    .unwrap()
            ),
            before_owner
        );
        assert_eq!(format!("{:?}", world.notifications), before_notifications);
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            expected_fx.next_shared_retail_random_u16()
        );
        assert!(!world
            .scheduler
            .prepare_native_actor_mutation(&world.entities, id));
    }
}
