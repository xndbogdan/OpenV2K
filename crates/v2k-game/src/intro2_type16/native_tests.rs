use super::*;
use crate::{
    common_mover::shared_initializer_target_speed_raw,
    entity::{EntityConstructionResources, EntityManager},
    entity_behavior::behavior_program,
    entity_collision_state::RetailStateWord,
    session::GameSession,
};

pub(in crate::intro2_type16) fn fixture() -> Option<(GameSession, Vec<EntityTypeRuntimeMetadata>)> {
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

pub(in crate::intro2_type16) fn generic(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
) -> EntityManager {
    EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    )
}

#[v2k_test_support::retail_test]
fn native_type16_birth_uses_own_weighted_graph_and_each_initializers_draw_count() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    // Fresh +34=0 disables the first authored choice even if the actor is
    // born after tick249. The two Always choices remain separate intervals.
    for (capability, selector, class, choice, expected_draws) in [
        (0, 0, 5, 3, 3),
        (0, 0xffff, 4, 4, 2),
        (0xc00, 0, 9, 1, 4),
        (1, 0, 7, 2, 4),
        (0xc01, 0, 9, 1, 4),
        (0xc01, 0x8000, 7, 2, 4),
    ] {
        let mut manager = generic(&session, &metadata);
        let mut earlier = generic(&session, &metadata);
        let owner = manager.entity_mut(6).unwrap();
        let nearby = earlier.entity_mut(1).unwrap();
        nearby.set_position_raw(owner.position_raw());
        nearby.capability_flags = capability;
        nearby.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
        let words = [0x1700, selector, 0x1234, 0x9876];
        let mut draws = 0;
        let publication = publish_intro2_type16(
            owner,
            &metadata[16],
            std::slice::from_ref(nearby),
            session.cache.terrain().unwrap(),
            &mut || {
                let word = words[draws];
                draws += 1;
                word
            },
        )
        .unwrap();
        assert_eq!(draws, expected_draws);
        assert_eq!(publication.selector_word, selector);
        assert_eq!(publication.selection.program.class_id, class);
        assert_eq!(publication.selection.choice_index, choice);
        assert_eq!(publication.people_nearby, capability & 0xc00 != 0);
        assert_eq!(publication.player_nearby, capability & 1 != 0);
        assert!(!publication.initializer_fallback);
        assert!(intro2_type16_allocation_authenticates(owner));
        match (
            class,
            owner.actor_task_state(ActorTaskSlot::Primary).unwrap(),
        ) {
            (4, ActorTaskRuntime::DefecateVirusWander(_)) => {
                assert!(matches!(
                    owner.actor_task_state(ActorTaskSlot::Tertiary),
                    Some(ActorTaskRuntime::DefecateVirusTerrain(_))
                ));
                assert!(owner.actor_task_state(ActorTaskSlot::Secondary).is_none());
            }
            (5 | 7 | 9, ActorTaskRuntime::SharedRetarget(task)) => {
                assert_eq!(task.elapsed_ms(), 0);
                assert_eq!(task.lifetime_ms(), if class == 5 { 5000 } else { 500 });
                assert!(owner.actor_task_state(ActorTaskSlot::Tertiary).is_none());
                if class == 5 {
                    assert!(owner.actor_task_state(ActorTaskSlot::Secondary).is_none());
                } else {
                    let Some(ActorTaskRuntime::TargetAcquisition(task)) =
                        owner.actor_task_state(ActorTaskSlot::Secondary)
                    else {
                        panic!()
                    };
                    assert_eq!(
                        task.constructor_filter_override_raw(),
                        if class == 9 { 0xc00 } else { 0 }
                    );
                }
            }
            (_, other) => panic!("wrong initial task for class{class}: {other:?}"),
        }
        let RetailRuntimeValue::Known(Some(a)) = owner.sub_a_propulsion_runtime else {
            panic!()
        };
        assert_eq!(
            a.target_speed_raw(),
            RetailRuntimeValue::Known(if class == 4 {
                1
            } else {
                shared_initializer_target_speed_raw(200, words[expected_draws - 1] as u16)
            })
        );
        assert_eq!(a.direction_multiplier(), 1);
        assert_eq!(a.drive_scale_percent(), 100);
        let RetailRuntimeValue::Known(Some(h)) = &owner.sub_h_external_frame_runtime else {
            panic!()
        };
        assert_eq!(h.records().len(), 8);
        assert!(h.is_enabled());
        assert!(h.records().iter().all(|record| record.flags_raw == 0));
        let e = owner.intro2_type16_runtime.unwrap().sub_e_runtime;
        assert_eq!(
            e,
            GenericEmitterRuntime {
                joint_bindings: [None; 2],
                projectile_method: 20,
                emitter_selector: 0,
                sound_id: 81,
                direct_mode: 0,
                remaining_time_raw: 0,
                manual_step_raw: 0,
                remaining_bursts_raw: 0,
                cadence_raw: 0,
                basis_adjustment_identity: None,
            }
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type16_rejects_wrong_profile_future_prefix_and_unknown_storage_before_rng() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for invalid in 0..4 {
        let mut manager = generic(&session, &metadata);
        let future = generic(&session, &metadata);
        let mut profile = metadata[16].clone();
        if invalid == 0 {
            let RetailRuntimeValue::Known(Some(mut e)) = profile.projectile_emitter_descriptor
            else {
                panic!()
            };
            e.variable_bindings[0] = 158;
            profile.projectile_emitter_descriptor = RetailRuntimeValue::Known(Some(e));
        }
        let owner = manager.entity_mut(6).unwrap();
        if invalid == 2 {
            owner.actor_common_axis_descriptor = RetailRuntimeValue::Unresolved;
        }
        if invalid == 3 {
            owner.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Unresolved;
        }
        let future_owner = future
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(42))
            .unwrap();
        let before_pose = owner.position_raw();
        let before_a = owner.sub_a_propulsion_runtime;
        let before_flags = owner.collision.state_flags_at_0x08;
        let mut draws = 0;
        assert_eq!(
            publish_intro2_type16(
                owner,
                &profile,
                if invalid == 1 {
                    std::slice::from_ref(future_owner)
                } else {
                    &[]
                },
                session.cache.terrain().unwrap(),
                &mut || {
                    draws += 1;
                    0
                }
            ),
            Err(match invalid {
                0 => Intro2Type16Error::Metadata,
                1 => Intro2Type16Error::Prefix,
                _ => Intro2Type16Error::ComponentStorage,
            })
        );
        assert_eq!(draws, 0);
        assert_eq!(owner.position_raw(), before_pose);
        assert_eq!(owner.sub_a_propulsion_runtime, before_a);
        assert_eq!(owner.collision.state_flags_at_0x08, before_flags);
        assert_eq!(owner.intro2_type16_runtime, None);
        assert_eq!(owner.initial_behavior, RetailRuntimeValue::Unresolved);
    }
}

#[v2k_test_support::retail_test]
fn native_type16_own_allocation_seeds_do_not_borrow_type17_and_birth_is_not_replayed() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    for (spawn, seed) in [(5, 0x05), (42, 0x1b)] {
        let entity = manager.entity_mut(spawn as u32 + 1).unwrap();
        publish_intro2_type16(
            entity,
            &metadata[16],
            &[],
            session.cache.terrain().unwrap(),
            &mut || 0,
        )
        .unwrap();
        let runtime = entity.intro2_type16_runtime.unwrap();
        assert_eq!(
            runtime.sub_d_owner.classifier_cache().stagger_counter(),
            seed
        );
        assert_eq!(
            runtime.sub_d_owner.classifier_cache().origin(),
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(runtime.sub_d_runtime, Type9SubDRuntime::from_constructor());
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        let [x, y, z] = entity.position_raw();
        assert_eq!(
            y,
            session
                .cache
                .terrain()
                .unwrap()
                .bilinear_height_raw(x, z)
                .wrapping_add(75)
        );
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(73);
        assert_eq!(
            publish_intro2_type16(
                entity,
                &metadata[16],
                &[],
                session.cache.terrain().unwrap(),
                &mut || panic!("birth replay")
            ),
            Err(Intro2Type16Error::AlreadyPublished)
        );
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(73)
        );
        entity.set_position_raw([x.wrapping_add(456), y, z.wrapping_sub(789)]);
        assert!(intro2_type16_allocation_authenticates(entity));
        entity.id += 1000;
        assert!(!intro2_type16_allocation_authenticates(entity));
    }
}

#[v2k_test_support::retail_test]
fn native_type16_initial_style_reentry_preserves_d_emitter_b2_and_uncopied_axis() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let entity = manager.entity_mut(6).unwrap();
    let first = publish_intro2_type16(
        entity,
        &metadata[16],
        &[],
        session.cache.terrain().unwrap(),
        &mut || 0,
    )
    .unwrap();
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0xffed);
    entity
        .intro2_type16_runtime
        .as_mut()
        .unwrap()
        .sub_e_runtime
        .cadence_raw = -8123;
    let receipt = entity.intro2_type16_runtime;
    for (class, expected_draws) in [(4, 0), (5, 1), (7, 2), (9, 2), (4, 0)] {
        let altered_axis = v2k_formats::collision::CommonAxisDescriptor {
            strict_axis_limit_raw: 1792,
            raw_word_at_0x04: 0x100,
        };
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(altered_axis);
        let selection = BehaviorSelection {
            program: behavior_program(class).unwrap(),
            ..first.selection
        };
        let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection).unwrap();
        let mut draws = 0;
        assert!(publish_initial_style(
            entity,
            &metadata[16],
            selection,
            context,
            &mut || {
                draws += 1;
                0x5555
            }
        ));
        assert_eq!(draws, expected_draws);
        assert_eq!(entity.intro2_type16_runtime, receipt);
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0xffed)
        );
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(v2k_formats::collision::CommonAxisDescriptor {
                strict_axis_limit_raw: altered_axis.strict_axis_limit_raw,
                raw_word_at_0x04: if matches!(class, 7 | 9) {
                    AXIS.raw_word_at_0x04
                } else {
                    altered_axis.raw_word_at_0x04
                },
            })
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type16_selector_uses_actual_relation_and_under_attack_window() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let mut earlier = generic(&session, &metadata);
    let owner = manager.entity_mut(6).unwrap();
    let nearby = earlier.entity_mut(1).unwrap();
    nearby.set_position_raw(owner.position_raw());
    nearby.capability_flags = 0xc01;
    nearby.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
    let candidates = [behavior::candidate(nearby)];
    for (tick, last_hit, expected_choice) in [
        (249, 249, 1),
        (250, 250, 0),
        (500, 250, 1),
        (u32::MAX, u32::MAX, 1),
    ] {
        let evaluated =
            behavior::evaluate(behavior::candidate(owner), &candidates, tick, last_hit).unwrap();
        let mut draws = 0;
        let (selected, _) = evaluated
            .select(&mut || {
                draws += 1;
                0
            })
            .unwrap();
        assert_eq!(selected.choice_index, expected_choice);
        assert_eq!(draws, 1);
    }
    owner.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(Some(nearby.id));
    let evaluated = behavior::evaluate(behavior::candidate(owner), &candidates, 0, 0).unwrap();
    assert!(!evaluated.people_nearby);
    assert!(!evaluated.player_nearby);
    owner.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Unresolved;
    assert_eq!(
        behavior::evaluate(behavior::candidate(owner), &candidates, 0, 0),
        Err(Intro2Type16Error::NearbyEvidence)
    );
}
