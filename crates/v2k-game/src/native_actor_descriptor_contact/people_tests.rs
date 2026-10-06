//! Actual later-world people retain the same zero-RNG Sub-I contact branch.
use super::*;
use crate::{
    gameplay_notifications::GameplayNotifications, intro2_gun_turret::authored_tests::fixture,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
};

#[v2k_test_support::retail_test]
fn native_worker_and_person_profiles_contact_through_their_own_allocations() {
    for (world, kinds) in [
        (19, &[79, 78][..]),
        (40, &[91, 95][..]),
        (24, &[90, 86][..]),
        (34, &[7][..]),
        (42, &[116][..]),
        (49, &[123][..]),
    ] {
        let (mut session, mut entities, mut fx) = fixture(world);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.adopt_intro2_type8(&mut entities);
        scheduler.adopt_native_type86(&mut entities);
        scheduler.adopt_native_type123(&mut entities);
        for &kind in kinds {
            let (id, slot, task_id) = entities
                .iter_all()
                .find_map(|entity| {
                    if entity.entity_type != kind {
                        return None;
                    }
                    ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .into_iter()
                        .find_map(|slot| {
                            let task_id = entity.actor_tasks.task_in_slot(slot)?;
                            task_has_descriptor_contact(entity.actor_tasks.task_state(task_id)?)
                                .then_some((entity.id, slot, task_id))
                        })
                })
                .unwrap_or_else(|| panic!("world{world} type{kind}: no authored descriptor task"));
            let source = actor(&entities, id).unwrap();
            let task = *source.actor_tasks.task_state(task_id).unwrap();
            let private = private_state(&task).unwrap();
            let position = source.position_raw();
            let orientation = source.rotation_heading_pitch_roll_raw();
            let animation = source.actor_animation_runtime;
            let basis = source.physical_body_basis_q31();
            let sub_d = source.type8_sub_d_runtime;
            let opposite = entities
                .iter_all()
                .find(|e| e.id != id && e.active)
                .unwrap()
                .id;
            entities
                .entity_mut(opposite)
                .unwrap()
                .set_position_raw(position);
            let next_word = fx
                .fork_for_main_base_abort_transaction()
                .next_shared_retail_random_u16();
            let mut damage = StaticDamageScheduler::new();
            let mut notifications = GameplayNotifications::new();
            let mut frame = Intro2ContactFrame {
                entities: &mut entities,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut damage,
                notifications: &mut notifications,
                retail_tick: 0,
                actor_tasks: &mut scheduler,
            };
            assert_eq!(
                resolve_native_actor_descriptor_contact(&mut frame, id, opposite, slot),
                Ok(NativeDescriptorContactOutcome::Applied { rng_draws: 0 }),
                "world{world} type{kind}"
            );
            let source = actor(&entities, id).unwrap();
            assert_eq!(source.actor_tasks.task_state(task_id), Some(&task));
            assert_eq!(source.actor_animation_runtime, animation);
            assert_eq!(source.physical_body_basis_q31(), basis);
            assert_eq!(source.type8_sub_d_runtime, sub_d);
            assert_eq!(source.position_raw(), position);
            assert_eq!(
                source.rotation_heading_pitch_roll_raw(),
                [
                    orientation[0].wrapping_add(0x2000),
                    orientation[1],
                    orientation[2]
                ]
            );
            let RetailRuntimeValue::Known(Some(sub_a)) = source.sub_a_propulsion_runtime else {
                panic!()
            };
            assert_eq!(sub_a.direction_multiplier(), i32::from(private.direction));
            assert_eq!(
                fx.fork_for_main_base_abort_transaction()
                    .next_shared_retail_random_u16(),
                next_word
            );
            assert!(scheduler.prepare_native_actor_mutation(&entities, id));
        }
    }
}
