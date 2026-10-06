use super::*;
use crate::{
    entity::{EntityConstructionResources, EntityManager},
    entity_behavior::audited_behavior_style,
    session::GameSession,
};

pub(in crate::intro2_type10) fn fixture() -> Option<(GameSession, Vec<EntityTypeRuntimeMetadata>)> {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata = session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(id, model_slots)| {
            session
                .cache
                .global_entity_type(id)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..Default::default()
                })
        })
        .collect();
    Some((session, metadata))
}

pub(in crate::intro2_type10) fn generic(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
) -> EntityManager {
    EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    )
}

pub(in crate::intro2_type10) fn publish(
    manager: &mut EntityManager,
    metadata: &[EntityTypeRuntimeMetadata],
    spawn: usize,
) -> u32 {
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(spawn))
        .unwrap()
        .id;
    publish_intro2_type10(manager.entity_mut(id).unwrap(), &metadata[10], &mut || 0).unwrap();
    id
}

#[v2k_test_support::retail_test]
fn native_type10_birth_owns_four_draws_authored_pose_and_deglk_allocations() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for (spawn, expected_position, seed) in [
        (55, [0x7300, 0x0700, 0x0900], 0x22),
        (56, [0x5a00, 0x0600, 0x0800], 0x23),
    ] {
        let mut manager = generic(&session, &metadata);
        let id = manager
            .iter_all()
            .find(|e| e.authored_spawn_index == Some(spawn))
            .unwrap()
            .id;
        let entity = manager.entity_mut(id).unwrap();
        let initial_basis = entity.physical_body_basis_q31();
        let initial_flags = entity.collision.state_flags_at_0x08;
        let words = [0x1100, 0xffff, 0x2200, 0x3300];
        let mut draws = 0;
        let publication = publish_intro2_type10(entity, &metadata[10], &mut || {
            let word = words[draws];
            draws += 1;
            word
        })
        .unwrap();
        assert_eq!(draws, 4);
        assert_eq!(publication.selector_word, 0xffff);
        assert_eq!(publication.selection.choice_index, 0);
        assert_eq!(publication.selection.program.class_id, 7);
        assert!(!publication.initializer_fallback);
        assert!(intro2_type10_allocation_authenticates(entity));
        assert_eq!(entity.position_raw(), expected_position);
        assert_eq!(entity.physical_body_basis_q31(), initial_basis);
        assert_eq!(
            entity.rotation_heading_pitch_roll_raw(),
            [0xc000u16 as i16, 0, 0]
        );
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(entity.collision.state_flags_at_0x08, initial_flags);
        let mut expected_g = SubG06070RuntimeState::from_1b8c0_constructor(&SUB_G, 0x1100);
        expected_g.apply_shared_06070_sub_g_branch(700 + 0x22, 0);
        expected_g.apply_shared_06070_sub_g_branch(700 + 0x33, 0);
        assert_eq!(
            entity.sub_g_06070_runtime,
            RetailRuntimeValue::Known(Some(expected_g))
        );
        let runtime = entity.intro2_type10_runtime.unwrap();
        assert_eq!(runtime.sub_d_runtime, Type9SubDRuntime::from_constructor());
        assert_eq!(
            runtime
                .sub_d_frame_owner
                .classifier_cache()
                .stagger_counter(),
            seed
        );
        assert_eq!(
            runtime.sub_d_frame_owner.classifier_cache().origin(),
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(runtime.sub_d_frame_owner.classifier_cache().rows(), [0; 8]);
        assert_eq!(runtime.sub_e_runtime.projectile_method, 10);
        assert_eq!(runtime.sub_e_runtime.sound_id, 81);
        assert_eq!(runtime.sub_e_runtime.cadence_raw, 0);
        assert_eq!(runtime.sub_e_runtime.joint_bindings, [None; 2]);
        assert_eq!(runtime.sub_k_smoothed_raw, 0);
        assert_eq!(runtime.sub_k_output_raw, [0; 2]);
        assert_eq!(runtime.sub_l_target_raw, [0; 3]);
        assert_eq!(runtime.sub_l_exact_raw, 0);
        assert_eq!(runtime.sub_l_output_raw, [0; 2]);
        let Some(ActorTaskRuntime::SharedRetarget(primary)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("B6C0 Primary");
        };
        assert_eq!(primary.lifetime_ms(), 500);
        assert_eq!(primary.elapsed_ms(), 0);
        let Some(ActorTaskRuntime::TargetAcquisition(secondary)) =
            entity.actor_task_state(ActorTaskSlot::Secondary)
        else {
            panic!("B6C0 Secondary");
        };
        assert_eq!(secondary.radius(), SearchAttackRadius::from_raw(7680));
        assert_eq!(secondary.filter().raw(), 0xc85);
        assert_eq!(secondary.constructor_filter_override_raw(), 0);
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
    }
}

#[v2k_test_support::retail_test]
fn native_type10_constructor_rejection_precedes_every_word_and_component_write() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for case in 0..7 {
        let mut manager = generic(&session, &metadata);
        let entity = manager.entity_mut(56).unwrap();
        let mut wrong_metadata = metadata[10].clone();
        match case {
            0 => entity.authored_spawn_index = Some(54),
            1 => entity.set_position_raw([0x7300, 0, 0x0900]),
            2 => entity.sub_g_06070_runtime = RetailRuntimeValue::Unresolved,
            3 => wrong_metadata.common_mover_gkl_payloads = RetailRuntimeValue::Unresolved,
            4 => {
                wrong_metadata.sub_d_steering_descriptor =
                    RetailRuntimeValue::Known(Some(crate::common_mover::sub_d::FLYER_SUB_D))
            }
            5 => entity.actor_common_axis_descriptor = RetailRuntimeValue::Unresolved,
            6 => {
                publish_intro2_type10(entity, &metadata[10], &mut || 7).unwrap();
            }
            _ => unreachable!(),
        }
        let before = (
            entity.intro2_type10_runtime,
            entity.sub_g_06070_runtime,
            entity.collision.clone(),
            entity.current_behavior_context,
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot)),
        );
        let result = publish_intro2_type10(entity, &wrong_metadata, &mut || {
            panic!("preflight must consume no word")
        });
        assert!(result.is_err(), "case {case}");
        assert_eq!(
            before,
            (
                entity.intro2_type10_runtime,
                entity.sub_g_06070_runtime,
                entity.collision.clone(),
                entity.current_behavior_context,
                ActorTaskSlot::IN_RETAIL_TICK_ORDER
                    .map(|slot| entity.actor_tasks.task_in_slot(slot))
            )
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type10_reselection_preserves_mutable_components_context_and_pose() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &metadata, 55);
    let entity = manager.entity_mut(id).unwrap();
    entity.set_position_raw([-1200, 4321, 5678]);
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0xfff3);
    let mut runtime = entity.intro2_type10_runtime.unwrap();
    runtime.sub_d_runtime = Type9SubDRuntime {
        yaw_rate_raw: -321,
        last_yaw_step_raw: 123,
    };
    runtime
        .sub_d_frame_owner
        .evidence_for_classifier_free_frame(
            SUB_D,
            [0; 3],
            [0, 0, 1000],
            [i32::MAX, 0, 0],
            [0, 0, i32::MAX],
        )
        .unwrap();
    runtime.sub_e_runtime.cadence_raw = 234567;
    runtime.sub_k_smoothed_raw = -890;
    runtime.sub_k_output_raw = [11, -12];
    runtime.sub_l_target_raw = [123, 456, -789];
    runtime.sub_l_exact_raw = 45;
    runtime.sub_l_output_raw = [-13, 14];
    entity.intro2_type10_runtime = Some(runtime);
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(CommonAxisDescriptor {
        strict_axis_limit_raw: 1700,
        raw_word_at_0x04: 0x77,
    });
    let program = behavior_program(7).unwrap();
    let previous = BehaviorContextRuntime::named_audited(
        program,
        1,
        RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
        RetailRuntimeValue::Known(Some(52)),
        RetailRuntimeValue::Known(0x11223344),
        *audited_behavior_style(7, 1).unwrap(),
    )
    .unwrap();
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(previous));
    let old_tasks =
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot));
    let position = entity.position_raw();
    let basis = entity.physical_body_basis_q31();
    let birth_selection = entity.initial_behavior;
    let mut draws = 0;
    let publication = reselect_acquiring(entity, &metadata[10], &mut || {
        draws += 1;
        [0x8000, 0xaa00, 0xbb00][draws - 1]
    })
    .unwrap();
    assert_eq!(draws, 3);
    assert_eq!(publication.selector_word, 0x8000);
    assert!(!publication.initializer_fallback);
    assert_eq!(entity.intro2_type10_runtime, Some(runtime));
    let vars = entity.presentation_anim_vars(0);
    for (selector, output) in SUB_K
        .into_iter()
        .zip(runtime.sub_k_output_raw)
        .chain(SUB_L[..2].iter().copied().zip(runtime.sub_l_output_raw))
    {
        assert_eq!(vars.dynamic[usize::from(selector)], i32::from(output));
    }
    assert_eq!(entity.position_raw(), position);
    assert_eq!(entity.physical_body_basis_q31(), basis);
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0xfff3)
    );
    assert_eq!(entity.initial_behavior, birth_selection);
    assert_eq!(
        entity.actor_common_axis_descriptor,
        RetailRuntimeValue::Known(CommonAxisDescriptor {
            strict_axis_limit_raw: 1700,
            raw_word_at_0x04: 0xc85
        })
    );
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!()
    };
    assert_eq!(
        context.target_handle_at_0x08(),
        previous.target_handle_at_0x08()
    );
    assert_eq!(
        context.auxiliary_word_at_0x0c(),
        previous.auxiliary_word_at_0x0c()
    );
    assert_eq!(context.active_style().style_address(), 0x4c7a50);
    let Some(ActorTaskRuntime::TargetAcquisition(acquisition)) =
        entity.actor_task_state(ActorTaskSlot::Secondary)
    else {
        panic!()
    };
    assert_eq!(acquisition.radius(), SearchAttackRadius::from_raw(1700));
    assert_ne!(
        old_tasks,
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot))
    );
    let Some(ActorTaskRuntime::SharedRetarget(primary)) =
        entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!()
    };
    assert_eq!(primary.elapsed_ms(), 0);
    let RetailRuntimeValue::Known(Some(g)) = entity.sub_g_06070_runtime else {
        panic!()
    };
    assert_eq!(
        g.randomized_target_raw_at_0x38(),
        RetailRuntimeValue::Known(700 + 0xbb)
    );
}

#[v2k_test_support::retail_test]
fn native_type10_corpus_binds_model351_and_own_query_free_sub_d_seeds() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let record = session.cache.global_entity_type(10).unwrap();
    assert_eq!(
        u16::from_le_bytes(record.raw_header[0x110..0x112].try_into().unwrap()),
        11
    );
    assert_eq!(SUB_G[0x24..0x2b], SUB_G_ANIMATION_BINDINGS.map(|i| i as u8));
    let model = session
        .cache
        .global_model(MODEL)
        .expect("normal-tier dragon model351");
    assert!(!model.cmd_words.is_empty());
    assert!(model.radius > 0);
    let geometry = model.materialize(&Default::default());
    assert!(!geometry.triangles.is_empty() || !geometry.instances.is_empty());
    let mut manager = generic(&session, &metadata);
    for (spawn, seed) in [(55, 0x22), (56, 0x23)] {
        let preceding_d_allocations = session.cache.level_desc().unwrap().entities[..spawn].iter()
            .filter(|s| matches!(metadata[s.entity_type as usize].common_mover_topology, RetailRuntimeValue::Known(t) if t.sub_d)).count();
        assert_eq!(preceding_d_allocations, seed);
        let id = publish(&mut manager, &metadata, spawn);
        let entity = manager.entity_mut(id).unwrap();
        let runtime = entity.intro2_type10_runtime.as_mut().unwrap();
        for _ in 0..35 {
            assert!(runtime
                .sub_d_frame_owner
                .evidence_for_classifier_free_frame(
                    SUB_D,
                    [0; 3],
                    [0, 0, 1000],
                    [i32::MAX, 0, 0],
                    [0, 0, i32::MAX]
                )
                .is_some());
        }
        assert_eq!(
            runtime
                .sub_d_frame_owner
                .classifier_cache()
                .stagger_counter(),
            seed as u8 + 35
        );
        assert_eq!(
            runtime.sub_d_frame_owner.classifier_cache().origin(),
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(runtime.sub_d_frame_owner.classifier_cache().rows(), [0; 8]);
    }
}

#[v2k_test_support::retail_test]
fn native_type10_reselection_rejects_unowned_or_dying_inputs_before_selector() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for case in 0..3 {
        let mut manager = generic(&session, &metadata);
        let id = publish(&mut manager, &metadata, 55);
        let entity = manager.entity_mut(id).unwrap();
        match case {
            0 => entity
                .collision
                .state_flags_at_0x08
                .overwrite(DYING_STATE_BIT, DYING_STATE_BIT),
            1 => entity.actor_common_axis_descriptor = RetailRuntimeValue::Unresolved,
            2 => entity.current_behavior_context = RetailRuntimeValue::Unresolved,
            _ => unreachable!(),
        }
        let before = (
            entity.intro2_type10_runtime,
            entity.sub_g_06070_runtime,
            entity.collision.clone(),
            entity.current_behavior_context,
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot)),
        );
        assert!(reselect_acquiring(entity, &metadata[10], &mut || panic!(
            "invalid reentry must not draw"
        ))
        .is_err());
        assert_eq!(
            before,
            (
                entity.intro2_type10_runtime,
                entity.sub_g_06070_runtime,
                entity.collision.clone(),
                entity.current_behavior_context,
                ActorTaskSlot::IN_RETAIL_TICK_ORDER
                    .map(|slot| entity.actor_tasks.task_in_slot(slot))
            )
        );
    }
}
