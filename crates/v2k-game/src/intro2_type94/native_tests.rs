use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    common_mover::shared_initializer_target_speed_raw,
    entity::{EntityConstructionResources, EntityManager},
    entity_collision_state::RetailStateWord,
    session::GameSession,
};

pub(in crate::intro2_type94) fn fixture() -> Option<(GameSession, Vec<EntityTypeRuntimeMetadata>)> {
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

pub(in crate::intro2_type94) fn generic(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
) -> EntityManager {
    EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    )
}

#[test]
fn native_type94_weighted_boundaries_preserve_authored_order_and_low_word() {
    for (player, people, cases) in [
        (false, false, vec![(0, 33), (65535, 33)]),
        (
            true,
            false,
            vec![(0, 33), (5957, 33), (5958, 7), (65535, 7)],
        ),
        (
            false,
            true,
            vec![(0, 33), (7281, 33), (7282, 9), (65535, 9)],
        ),
        (
            true,
            true,
            vec![
                (0, 33),
                (3449, 33),
                (3450, 7),
                (37941, 7),
                (37942, 9),
                (65535, 9),
            ],
        ),
    ] {
        for (word, class) in cases {
            let mut draws = 0;
            let (selection, returned) = behavior::Evaluators {
                player_nearby: player,
                people_nearby: people,
            }
            .select(&mut || {
                draws += 1;
                0xabcd_0000 | word
            })
            .unwrap();
            assert_eq!(selection.program.class_id, class);
            assert_eq!(
                selection.choice_index,
                match class {
                    33 => 0,
                    7 => 1,
                    9 => 2,
                    _ => unreachable!(),
                }
            );
            assert_eq!(returned, 0xabcd_0000 | word);
            assert_eq!(draws, 1);
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type94_birth_constructs_water_h_a_e_and_exact_four_words_for_each_root() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for (capabilities, selector, class) in [(0, 65535, 33), (1, 65535, 7), (0x0c00, 65535, 9)] {
        let mut manager = generic(&session, &metadata);
        let mut other = generic(&session, &metadata);
        let owner = manager.entity_mut(44).unwrap();
        let nearby = other.entity_mut(1).unwrap();
        nearby.set_position_raw(owner.position_raw());
        nearby.capability_flags = capabilities;
        nearby.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
        let words = [0x1700, selector, 0x1234, 0x9876];
        let mut draws = 0;
        let publication = publish_intro2_type94(
            owner,
            &metadata[94],
            std::slice::from_ref(nearby),
            session.cache.terrain().unwrap(),
            &mut || {
                let word = words[draws];
                draws += 1;
                word
            },
        )
        .unwrap();
        assert_eq!(draws, 4);
        assert_eq!(publication.selection.program.class_id, class);
        assert!(!publication.initializer_fallback);
        assert!(intro2_type94_allocation_authenticates(owner));
        assert!(matches!(
            owner.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(_))
        ));
        assert!(owner.actor_task_state(ActorTaskSlot::Secondary).is_some());
        assert!(owner.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        let RetailRuntimeValue::Known(Some(a)) = owner.sub_a_propulsion_runtime else {
            panic!()
        };
        assert_eq!(
            a.target_speed_raw(),
            RetailRuntimeValue::Known(shared_initializer_target_speed_raw(400, 0x9876))
        );
        let RetailRuntimeValue::Known(Some(h)) = &owner.sub_h_external_frame_runtime else {
            panic!()
        };
        assert_eq!(
            h.surface_policy(),
            crate::sub_h_external_frame::SubHSurfacePolicy::TerrainAndWater
        );
        assert_eq!(h.records().len(), 6);
        assert!(h.is_enabled());
        let native = owner.intro2_type94_runtime.unwrap();
        assert_eq!(native.sub_e_runtime.sound_id, 69);
        assert_eq!(native.sub_e_runtime.projectile_method, 20);
        assert_eq!(native.sub_e_runtime.joint_bindings, [None; 2]);
        assert_eq!(native.sub_e_runtime.cadence_raw, 0);
        assert!(owner.intro2_type94_aim_runtime.is_none());
    }
}

#[v2k_test_support::retail_test]
fn native_type94_metadata_and_prefix_rejection_precede_shared_rng() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let owner = manager.entity_mut(44).unwrap();
    let before = (
        owner.position_raw(),
        owner.sub_a_propulsion_runtime,
        owner.current_behavior_context,
    );
    let mut wrong = metadata[94].clone();
    wrong.mass_raw = 101;
    assert_eq!(
        publish_intro2_type94(
            owner,
            &wrong,
            &[],
            session.cache.terrain().unwrap(),
            &mut || panic!("metadata preflight RNG")
        ),
        Err(Intro2Type94Error::Metadata)
    );
    assert_eq!(
        (
            owner.position_raw(),
            owner.sub_a_propulsion_runtime,
            owner.current_behavior_context
        ),
        before
    );
    assert!(owner.intro2_type94_runtime.is_none());
    assert!(owner.actor_task_state(ActorTaskSlot::Primary).is_none());
}

#[v2k_test_support::retail_test]
fn native_type94_people_and_player_evaluators_use_current_axis_and_wrapped_xz() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let mut owner = behavior::candidate(manager.entity_mut(44).unwrap());
    owner.position_raw = [i16::MAX - 99, 0, 0];
    owner.state_flags_raw = RetailStateWord::exact(4);
    let target = manager.entity_mut(1).unwrap();
    target.set_position_raw([i16::MIN + 100, 0, 0]);
    target.capability_flags = 0x0c01;
    target.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
    let candidates = [behavior::candidate(target)];
    for (range, near) in [(200, false), (201, true)] {
        let result = behavior::evaluate(
            owner,
            &candidates,
            CommonAxisDescriptor {
                strict_axis_limit_raw: range,
                raw_word_at_0x04: 37,
            },
        )
        .unwrap();
        assert_eq!(
            result,
            behavior::Evaluators {
                player_nearby: near,
                people_nearby: near
            }
        );
    }
}
