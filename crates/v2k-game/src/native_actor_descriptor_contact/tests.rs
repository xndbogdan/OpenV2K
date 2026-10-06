use super::*;
use crate::{
    common_mover::target_prelude::{
        plan_common_mover_descriptor_effect, CommonMoverDescriptorEffectRequest,
    },
    intro2_type17::pair::tests::{ChildOrder, Fixture},
};

fn invoke(
    fixture: &mut Fixture,
    id: u32,
    opposite: u32,
    slot: ActorTaskSlot,
) -> Result<NativeDescriptorContactOutcome, NativeDescriptorContactBlock> {
    resolve_native_actor_descriptor_contact(
        &mut Intro2ContactFrame {
            entities: &mut fixture.entities,
            resources: &mut fixture.session.cache,
            world_fx: &mut fixture.fx,
            static_damage: &mut fixture.static_damage,
            notifications: &mut fixture.notifications,
            retail_tick: fixture.tick,
            actor_tasks: &mut fixture.scheduler,
        },
        id,
        opposite,
        slot,
    )
}

fn primary_with_descriptor(fixture: &Fixture, kind: u32) -> u32 {
    fixture
        .entities
        .iter_all()
        .find_map(|entity| {
            let task = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary)?;
            (entity.entity_type == kind
                && task_has_descriptor_contact(entity.actor_tasks.task_state(task)?))
            .then_some(entity.id)
        })
        .expect("actual native descriptor Primary")
}

fn opposite_id(fixture: &Fixture, id: u32) -> u32 {
    if id == fixture.spider {
        fixture.child
    } else {
        fixture.spider
    }
}

fn next_word(fixture: &Fixture) -> u16 {
    fixture
        .fx
        .fork_for_main_base_abort_transaction()
        .next_shared_retail_random_u16()
}

fn place_at_gate(fixture: &mut Fixture, id: u32, opposite: u32, behind: bool) {
    let source = actor(&fixture.entities, id).unwrap();
    let mut position = source.position_raw();
    if behind {
        let RetailRuntimeValue::Known(basis) = source.physical_body_basis_q31() else {
            panic!()
        };
        let axis = (0..3)
            .max_by_key(|&axis| i64::from(basis.forward[axis]).abs())
            .unwrap();
        assert_ne!(basis.forward[axis], 0);
        position[axis] =
            position[axis].wrapping_sub(if basis.forward[axis] > 0 { 64 } else { -64 });
    }
    fixture
        .entities
        .entity_mut(opposite)
        .unwrap()
        .set_position_raw(position);
}

#[v2k_test_support::retail_test]
fn native_people_contact_changes_heading_without_advancing_task_or_animation() {
    // Both the ordinary loader and Intro2 use real native allocations; the
    // callback phase is controlled independently of natural collision timing.
    for (world, kinds) in [(50, &[8_u32, 9][..]), (14, &[9_u32][..])] {
        let mut fixture = Fixture::ready(world, None, ChildOrder::First);
        for &kind in kinds {
            let id = primary_with_descriptor(&fixture, kind);
            let opposite = opposite_id(&fixture, id);
            place_at_gate(&mut fixture, id, opposite, false);
            let before = actor(&fixture.entities, id).unwrap();
            let task_id = before
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            let task = *before.actor_tasks.task_state(task_id).unwrap();
            let private = private_state(&task).unwrap();
            let orientation = before.rotation_heading_pitch_roll_raw();
            let animation = before.actor_animation_runtime.clone();
            let basis = before.physical_body_basis_q31();
            let position = before.position_raw();
            let velocity = before.velocity_raw();
            let RetailRuntimeValue::Known(Some(sub_a)) = before.sub_a_propulsion_runtime else {
                panic!()
            };
            let rng = next_word(&fixture);

            assert_eq!(
                invoke(&mut fixture, id, opposite, ActorTaskSlot::Primary),
                Ok(NativeDescriptorContactOutcome::Applied { rng_draws: 0 }),
                "world={world}, type={kind}"
            );
            let after = actor(&fixture.entities, id).unwrap();
            assert_eq!(
                after.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                Some(task_id)
            );
            assert_eq!(after.actor_tasks.task_state(task_id), Some(&task));
            assert_eq!(after.actor_animation_runtime, animation);
            assert_eq!(after.physical_body_basis_q31(), basis);
            assert_eq!(
                (after.position_raw(), after.velocity_raw()),
                (position, velocity)
            );
            assert_eq!(
                after.rotation_heading_pitch_roll_raw(),
                [
                    orientation[0].wrapping_add(0x2000),
                    orientation[1],
                    orientation[2]
                ]
            );
            let RetailRuntimeValue::Known(Some(after_a)) = after.sub_a_propulsion_runtime else {
                panic!()
            };
            assert_eq!(after_a.target_speed_raw(), sub_a.target_speed_raw());
            assert_eq!(after_a.drive_scale_percent(), sub_a.drive_scale_percent());
            assert_eq!(after_a.direction_multiplier(), i32::from(private.direction));
            assert_eq!(next_word(&fixture), rng);
            assert!(fixture
                .scheduler
                .prepare_native_actor_mutation(&fixture.entities, id));
        }
    }
}

#[v2k_test_support::retail_test]
fn native_no_i_actors_reverse_using_actual_descriptor_and_shared_rng() {
    let mut fixture = Fixture::ready(50, None, ChildOrder::First);
    assert!(
        fixture
            .scheduler
            .adopt_intro2_type47_guards(&fixture.entities)
            > 0
    );
    fixture.scheduler.adopt_intro2_type53(&fixture.entities);
    fixture.scheduler.adopt_intro2_type94(&fixture.entities);
    for kind in [17, 47, 53, 94] {
        let id = primary_with_descriptor(&fixture, kind);
        let opposite = opposite_id(&fixture, id);
        // Repeated contacts also prove that the Type47 completed graph is
        // refreshed without replacing its scheduler owner or task allocation.
        for _ in 0..2 {
            place_at_gate(&mut fixture, id, opposite, false);
            let before = actor(&fixture.entities, id).unwrap();
            let task_id = before
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            let task = *before.actor_tasks.task_state(task_id).unwrap();
            let private = private_state(&task).unwrap();
            let basis = before.physical_body_basis_q31();
            let position = before.position_raw();
            let velocity = before.velocity_raw();
            let [heading, pitch, roll] = before.rotation_heading_pitch_roll_raw();
            let metadata = fixture.entities.type_runtime_metadata(kind).unwrap();
            let RetailRuntimeValue::Known(Some(a)) = metadata.sub_a_propulsion_descriptor else {
                panic!()
            };
            let RetailRuntimeValue::Known(Some(d)) = metadata.sub_d_steering_descriptor else {
                panic!()
            };
            let RetailRuntimeValue::Known(Some(sub_a)) = before.sub_a_propulsion_runtime else {
                panic!()
            };
            let mut expected_fx = fixture.fx.fork_for_main_base_abort_transaction();
            let expected = plan_common_mover_descriptor_effect(
                CommonMoverDescriptorEffectRequest {
                    target_state: private,
                    controlled_position_raw: position,
                    heading_raw: heading as u16,
                    roll_raw: roll as u16,
                    topology: CommonMoverTargetPreludeTopology {
                        sub_a: Some(CommonMoverPreludeSubA {
                            descriptor: Some(a),
                            runtime: sub_a,
                        }),
                        sub_d: Some(CommonMoverPreludeSubD {
                            steering_divisor_raw: d.steering_divisor_raw,
                            couple_yaw_into_roll: d.couple_yaw_into_roll_raw != 0,
                        }),
                        sub_i: false,
                        sub_f: false,
                        sub_g: false,
                        sub_l: false,
                    },
                },
                || u32::from(expected_fx.next_shared_retail_random_u16()),
            )
            .unwrap();
            assert!(expected.rng_draw_count > 0);

            assert_eq!(
                invoke(&mut fixture, id, opposite, ActorTaskSlot::Primary),
                Ok(NativeDescriptorContactOutcome::Applied {
                    rng_draws: expected.rng_draw_count
                })
            );
            let after = actor(&fixture.entities, id).unwrap();
            let after_task = after.actor_tasks.task_state(task_id).unwrap();
            assert_eq!(private_state(after_task), Some(expected.target_state));
            assert_eq!(
                expected.target_state.direction,
                private.direction.wrapping_neg()
            );
            assert_eq!(expected.target_state.reversal_timer_ms, 1_500);
            assert_eq!(
                after.sub_a_propulsion_runtime,
                RetailRuntimeValue::Known(expected.sub_a_runtime)
            );
            assert_eq!(
                after.rotation_heading_pitch_roll_raw(),
                [expected.heading_raw as i16, pitch, expected.roll_raw as i16]
            );
            let sub_d = match kind {
                17 => after.type17_sub_d_runtime.unwrap(),
                47 => after.intro2_type47_sub_d_runtime.unwrap(),
                53 => after.intro2_type53_runtime.unwrap().sub_d_runtime,
                94 => after.intro2_type94_runtime.unwrap().sub_d_runtime,
                _ => unreachable!(),
            };
            assert_eq!(
                sub_d.last_yaw_step_raw,
                expected.sub_d_reversal_write.unwrap().step_raw as i16
            );
            assert_eq!(after.physical_body_basis_q31(), basis);
            assert_eq!(
                (after.position_raw(), after.velocity_raw()),
                (position, velocity)
            );
            match (task, *after_task) {
                (
                    ActorTaskRuntime::CapturePeoplePursuit(before),
                    ActorTaskRuntime::CapturePeoplePursuit(after),
                ) => {
                    assert_eq!(
                        (after.elapsed_ms(), after.lifetime()),
                        (before.elapsed_ms(), before.lifetime())
                    );
                }
                (
                    ActorTaskRuntime::SharedRetarget(before),
                    ActorTaskRuntime::SharedRetarget(after),
                ) => {
                    assert_eq!(
                        (after.elapsed_ms(), after.lifetime_ms()),
                        (before.elapsed_ms(), before.lifetime_ms())
                    );
                }
                (
                    ActorTaskRuntime::OrdinaryType9Wander(before),
                    ActorTaskRuntime::OrdinaryType9Wander(after),
                ) => {
                    assert_eq!(after.elapsed_ms(), before.elapsed_ms());
                }
                _ => panic!("unexpected actual initial native task: {task:?}"),
            }
            assert_eq!(
                next_word(&fixture),
                expected_fx.next_shared_retail_random_u16()
            );
            assert!(fixture
                .scheduler
                .prepare_native_actor_mutation(&fixture.entities, id));
        }
    }
}

#[v2k_test_support::retail_test]
fn negative_forward_halfspace_does_not_require_descriptor_runtime_or_draw_rng() {
    let mut fixture = Fixture::ready(50, None, ChildOrder::First);
    fixture
        .scheduler
        .adopt_intro2_type47_guards(&fixture.entities);
    fixture.scheduler.adopt_intro2_type53(&fixture.entities);
    fixture.scheduler.adopt_intro2_type94(&fixture.entities);
    for kind in [8, 9, 17, 47, 53, 94] {
        let id = primary_with_descriptor(&fixture, kind);
        let opposite = opposite_id(&fixture, id);
        place_at_gate(&mut fixture, id, opposite, true);
        let entity = fixture.entities.entity_mut(id).unwrap();
        let a = entity.sub_a_propulsion_runtime;
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Unresolved;
        let task_id = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let task = *entity.actor_tasks.task_state(task_id).unwrap();
        let rotation = entity.rotation_heading_pitch_roll_raw();
        let rng = next_word(&fixture);
        assert_eq!(
            invoke(&mut fixture, id, opposite, ActorTaskSlot::Primary),
            Ok(NativeDescriptorContactOutcome::Behind),
            "type={kind}"
        );
        let entity = fixture.entities.entity_mut(id).unwrap();
        assert_eq!(
            entity.sub_a_propulsion_runtime,
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(entity.actor_tasks.task_state(task_id), Some(&task));
        assert_eq!(entity.rotation_heading_pitch_roll_raw(), rotation);
        entity.sub_a_propulsion_runtime = a;
        assert_eq!(next_word(&fixture), rng);
    }
}

#[v2k_test_support::retail_test]
fn current_null_acquisition_and_pending_descriptor_owner_are_distinct() {
    let mut fixture = Fixture::ready(50, None, ChildOrder::First);
    fixture
        .scheduler
        .adopt_intro2_type47_guards(&fixture.entities);
    let (id, slot) = fixture
        .entities
        .iter_all()
        .find_map(|entity| {
            if !matches!(entity.entity_type, 8 | 9 | 17 | 47) {
                return None;
            }
            [ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary]
                .into_iter()
                .find_map(|slot| {
                    let task_id = entity.actor_tasks.task_in_slot(slot)?;
                    task_has_null_contact(entity.actor_tasks.task_state(task_id)?)
                        .then_some((entity.id, slot))
                })
        })
        .expect("canonical acquisition/cue/Aim task");
    let opposite = opposite_id(&fixture, id);
    let rng = next_word(&fixture);
    assert_eq!(
        invoke(&mut fixture, id, opposite, slot),
        Ok(NativeDescriptorContactOutcome::Null)
    );
    assert_eq!(next_word(&fixture), rng);

    let id = fixture.spider;
    let opposite = fixture.child;
    place_at_gate(&mut fixture, id, opposite, false);
    let entity = actor(&fixture.entities, id).unwrap();
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let task = *entity.actor_tasks.task_state(task_id).unwrap();
    let a = entity.sub_a_propulsion_runtime;
    fixture.scheduler.park_intro2_type17_external_prefix(id);
    let failure = invoke(&mut fixture, id, opposite, ActorTaskSlot::Primary).unwrap_err();
    assert!(!failure.committed_prefix);
    assert_eq!(
        failure.reason,
        NativeDescriptorContactError::Runtime("completed descriptor owner")
    );
    let entity = actor(&fixture.entities, id).unwrap();
    assert_eq!(entity.actor_tasks.task_state(task_id), Some(&task));
    assert_eq!(entity.sub_a_propulsion_runtime, a);
    assert_eq!(next_word(&fixture), rng);
}

#[v2k_test_support::retail_test]
fn live_and_dying_hive_templates_have_null_contact_without_target_or_rng_reads() {
    let mut fixture = Fixture::ready(50, None, ChildOrder::First);
    let id = fixture
        .entities
        .iter_all()
        .find(|entity| entity.entity_type == 67)
        .unwrap()
        .id;
    for dying in [false, true] {
        if dying {
            fixture
                .entities
                .entity_mut(id)
                .unwrap()
                .apply_hive_dying_initializer()
                .expect("actual Class46 dying publication");
        }
        let entity = actor(&fixture.entities, id).unwrap();
        let task_id = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let task = *entity.actor_tasks.task_state(task_id).unwrap();
        assert!(matches!(task, ActorTaskRuntime::HiveRadial(state) if state.is_dying() == dying));
        let body = (
            entity.position_raw(),
            entity.velocity_raw(),
            entity.rotation_heading_pitch_roll_raw(),
            entity.physical_body_basis_q31(),
        );
        let rng = next_word(&fixture);
        // A null task+18 does not dereference the opposite entity or enter
        // the common-mover descriptor/owner path.
        assert_eq!(
            invoke(&mut fixture, id, u32::MAX, ActorTaskSlot::Primary),
            Ok(NativeDescriptorContactOutcome::Null)
        );
        let entity = actor(&fixture.entities, id).unwrap();
        assert_eq!(entity.actor_tasks.task_state(task_id), Some(&task));
        assert_eq!(
            (
                entity.position_raw(),
                entity.velocity_raw(),
                entity.rotation_heading_pitch_roll_raw(),
                entity.physical_body_basis_q31(),
            ),
            body
        );
        assert_eq!(next_word(&fixture), rng);
    }
}
