use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    common_mover::shared_initializer_target_speed_raw,
    entity::{EntityConstructionResources, EntityManager},
    entity_behavior::behavior_program,
    entity_collision_state::RetailStateWord,
    session::GameSession,
};

pub(in crate::intro2_type58) fn fixture() -> Option<(GameSession, Vec<EntityTypeRuntimeMetadata>)> {
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

pub(in crate::intro2_type58) fn generic(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
) -> EntityManager {
    EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    )
}

fn controlled_terrain(session: &GameSession, furniture: bool) -> TerrainGrid {
    let original = session.cache.terrain().unwrap();
    let mut terrain = TerrainGrid {
        header: original.header,
        cells: original.cells.clone(),
    };
    for cell in &mut terrain.cells {
        cell.attribute = 0;
    }
    if furniture {
        assert!(session.cache.terrain_objects().unwrap().records.len() > 1);
        // Spawn40 is BC00/0D00. Its first4230C0 corner is BB00/0C00.
        let cell = &mut terrain.cells[0xbb * 256 + 0x0c];
        cell.attribute = 1;
        cell.terrain_type &= !8;
    }
    terrain
}

#[test]
fn native_type58_weighted_intervals_keep_authored_order_and_one_low_word_draw() {
    for (furniture, player, cases) in [
        (false, false, vec![(0, 33), (65535, 33)]),
        (
            true,
            false,
            vec![(0, 33), (10922, 33), (10923, 26), (65535, 26)],
        ),
        (
            false,
            true,
            vec![(0, 33), (5957, 33), (5958, 7), (65535, 7)],
        ),
        (
            true,
            true,
            vec![
                (0, 33),
                (4095, 33),
                (4096, 26),
                (24575, 26),
                (24576, 7),
                (65535, 7),
            ],
        ),
    ] {
        for (word, class) in cases {
            let sample = 0xabcd_0000 | word;
            let mut draws = 0;
            let (selection, returned) = behavior::Evaluators {
                furniture_nearby: furniture,
                player_nearby: player,
            }
            .select(&mut || {
                draws += 1;
                sample
            })
            .unwrap();
            assert_eq!(selection.program.class_id, class);
            assert_eq!(
                selection.choice_index,
                match class {
                    33 => 0,
                    26 => 1,
                    7 => 2,
                    _ => unreachable!(),
                }
            );
            assert_eq!(returned, sample);
            assert_eq!(draws, 1);
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type58_birth_consumes_a_selector_then_exact_task_suffix_and_keeps_native_e() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for (furniture, player, selector, class, expected_draws) in [
        (false, false, 0xffff, 33, 4),
        (true, false, 0xffff, 26, 3),
        (false, true, 0xffff, 7, 4),
        (true, true, 0x4000, 26, 3),
    ] {
        let terrain = controlled_terrain(&session, furniture);
        let mut manager = generic(&session, &metadata);
        let mut earlier = generic(&session, &metadata);
        let owner = manager.entity_mut(41).unwrap();
        let nearby = earlier.entity_mut(1).unwrap();
        nearby.set_position_raw(owner.position_raw());
        nearby.capability_flags = u32::from(player);
        nearby.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
        let words = [0x1700, selector, 0x1234, 0x9876];
        let mut draws = 0;
        let publication = publish_intro2_type58(
            owner,
            &metadata[58],
            std::slice::from_ref(nearby),
            &terrain,
            session.cache.terrain_objects().unwrap(),
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
        assert_eq!(publication.furniture_nearby, furniture);
        assert_eq!(publication.player_nearby, player);
        assert!(!publication.initializer_fallback);
        assert!(intro2_type58_allocation_authenticates(owner));
        match (
            class,
            owner.actor_task_state(ActorTaskSlot::Primary).unwrap(),
        ) {
            (26, ActorTaskRuntime::TrashFurniture(task)) => {
                assert_eq!(task.elapsed_ms(), 0);
                assert_eq!(task.target_position_raw(), [0; 3]);
                assert_eq!(task.target_y(), RetailRuntimeValue::Unresolved);
                assert_eq!(task.kind_filter(), -1);
                assert!(owner.actor_task_state(ActorTaskSlot::Secondary).is_none());
            }
            (7 | 33, ActorTaskRuntime::SharedRetarget(task)) => {
                assert_eq!(task.elapsed_ms(), 0);
                assert!(owner.actor_task_state(ActorTaskSlot::Secondary).is_some());
            }
            (_, other) => panic!("class{class}: {other:?}"),
        }
        assert!(owner.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        let RetailRuntimeValue::Known(Some(a)) = owner.sub_a_propulsion_runtime else {
            panic!()
        };
        assert_eq!(
            a.target_speed_raw(),
            RetailRuntimeValue::Known(shared_initializer_target_speed_raw(
                300,
                words[expected_draws - 1] as u16
            ))
        );
        assert_eq!(a.direction_multiplier(), 1);
        assert_eq!(a.drive_scale_percent(), 100);
        let RetailRuntimeValue::Known(Some(h)) = &owner.sub_h_external_frame_runtime else {
            panic!()
        };
        assert_eq!(h.records().len(), 6);
        assert!(h.is_enabled());
        assert!(h.records().iter().all(|record| record.flags_raw == 0));
        assert_eq!(
            owner.intro2_type58_runtime.unwrap().sub_e_runtime,
            GenericEmitterRuntime {
                joint_bindings: [None; 2],
                projectile_method: 20,
                emitter_selector: 0,
                sound_id: 70,
                direct_mode: 0,
                remaining_time_raw: 0,
                manual_step_raw: 0,
                remaining_bursts_raw: 0,
                cadence_raw: 0,
                basis_adjustment_identity: None,
            }
        );
        assert!(owner.intro2_type58_aim_runtime.is_none());
    }
}

#[v2k_test_support::retail_test]
fn native_type58_evaluators_read_live_axis_for_quarter_radius_furniture_and_full_player_range() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let mut earlier = generic(&session, &metadata);
    let owner = manager.entity_mut(41).unwrap();
    let mut owner_ref = behavior::candidate(owner);
    owner_ref.position_raw = [0x1234, 0, 0x5678];
    let target = earlier.entity_mut(1).unwrap();
    target.set_position_raw([0x1234 + 700, 0, 0x5678]);
    target.capability_flags = 1;
    target.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
    let candidates = [behavior::candidate(target)];
    let mut terrain = controlled_terrain(&session, false);
    terrain.cells[0x11 * 256 + 0x55].attribute = 1;
    terrain.cells[0x11 * 256 + 0x55].terrain_type &= !8;
    for (axis_raw, furniture, player) in [
        (512, false, false),
        (1024, false, true),
        (2048, true, true),
        (4096, true, true),
    ] {
        let result = behavior::evaluate(
            owner_ref,
            &candidates,
            CommonAxisDescriptor {
                strict_axis_limit_raw: axis_raw,
                raw_word_at_0x04: 5,
            },
            &terrain,
            session.cache.terrain_objects().unwrap(),
        )
        .unwrap();
        assert_eq!(result.furniture_nearby, furniture, "live axis{axis_raw}");
        assert_eq!(result.player_nearby, player, "live axis{axis_raw}");
    }
}

#[v2k_test_support::retail_test]
fn native_type58_preflight_rejects_foreign_descriptor_future_prefix_and_missing_storage_before_rng()
{
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for invalid in 0..5 {
        let mut manager = generic(&session, &metadata);
        let future = generic(&session, &metadata);
        let mut profile = metadata[58].clone();
        if invalid == 0 {
            let RetailRuntimeValue::Known(Some(mut e)) = profile.projectile_emitter_descriptor
            else {
                panic!()
            };
            e.variable_bindings[0] = 150;
            profile.projectile_emitter_descriptor = RetailRuntimeValue::Known(Some(e));
        }
        let owner = manager.entity_mut(41).unwrap();
        if invalid == 2 {
            owner.actor_common_axis_descriptor = RetailRuntimeValue::Unresolved;
        }
        if invalid == 3 {
            owner.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Unresolved;
        }
        if invalid == 4 {
            owner.sub_h_external_frame_runtime = RetailRuntimeValue::Unresolved;
        }
        let future_owner = future
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(42))
            .unwrap();
        let before = (
            owner.position_raw(),
            owner.sub_a_propulsion_runtime,
            owner.collision.state_flags_at_0x08,
        );
        let mut draws = 0;
        assert_eq!(
            publish_intro2_type58(
                owner,
                &profile,
                if invalid == 1 {
                    std::slice::from_ref(future_owner)
                } else {
                    &[]
                },
                session.cache.terrain().unwrap(),
                session.cache.terrain_objects().unwrap(),
                &mut || {
                    draws += 1;
                    0
                }
            ),
            Err(match invalid {
                0 => Intro2Type58Error::Metadata,
                1 => Intro2Type58Error::Prefix,
                _ => Intro2Type58Error::ComponentStorage,
            })
        );
        assert_eq!(draws, 0);
        assert_eq!(
            (
                owner.position_raw(),
                owner.sub_a_propulsion_runtime,
                owner.collision.state_flags_at_0x08
            ),
            before
        );
        assert_eq!(owner.intro2_type58_runtime, None);
        assert_eq!(owner.initial_behavior, RetailRuntimeValue::Unresolved);
    }
}

#[v2k_test_support::retail_test]
fn native_type58_reentry_preserves_own_d_e_b2_and_axis_and_birth_cannot_replay() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let entity = manager.entity_mut(41).unwrap();
    let first = publish_intro2_type58(
        entity,
        &metadata[58],
        &[],
        session.cache.terrain().unwrap(),
        session.cache.terrain_objects().unwrap(),
        &mut || 0,
    )
    .unwrap();
    assert_eq!(
        entity
            .intro2_type58_runtime
            .unwrap()
            .sub_d_owner
            .classifier_cache()
            .stagger_counter(),
        0x19
    );
    assert_eq!(
        entity.intro2_type58_runtime.unwrap().sub_d_runtime,
        Type9SubDRuntime::from_constructor()
    );
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0)
    );
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0xffed);
    entity
        .intro2_type58_runtime
        .as_mut()
        .unwrap()
        .sub_e_runtime
        .cadence_raw = -8123;
    let receipt = entity.intro2_type58_runtime;
    assert_eq!(
        publish_intro2_type58(
            entity,
            &metadata[58],
            &[],
            session.cache.terrain().unwrap(),
            session.cache.terrain_objects().unwrap(),
            &mut || panic!("birth replay")
        ),
        Err(Intro2Type58Error::AlreadyPublished)
    );
    for (class, count) in [(26, 1), (33, 2), (7, 2), (26, 1)] {
        let axis = CommonAxisDescriptor {
            strict_axis_limit_raw: 1792,
            raw_word_at_0x04: 0x100,
        };
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
        let selection = BehaviorSelection {
            program: behavior_program(class).unwrap(),
            ..first.selection
        };
        let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection).unwrap();
        let mut draws = 0;
        assert!(publish_initial_style(
            entity,
            &metadata[58],
            selection,
            context,
            &mut || {
                draws += 1;
                0x5555
            }
        ));
        assert_eq!(draws, count);
        assert_eq!(entity.intro2_type58_runtime, receipt);
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0xffed)
        );
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(CommonAxisDescriptor {
                strict_axis_limit_raw: axis.strict_axis_limit_raw,
                raw_word_at_0x04: if class == 7 {
                    AXIS.raw_word_at_0x04
                } else {
                    axis.raw_word_at_0x04
                },
            })
        );
    }
}
