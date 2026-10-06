use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    common_mover::shared_initializer_target_speed_raw,
    entity::{EntityConstructionResources, EntityManager},
    entity_behavior::{behavior_program, BehaviorContextRuntime},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord},
    session::GameSession,
};

pub(crate) fn fixture() -> Option<(GameSession, Vec<EntityTypeRuntimeMetadata>)> {
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

pub(super) fn generic(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
) -> EntityManager {
    EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
    )
}

#[v2k_test_support::retail_test]
fn native_type17_birth_draws_sub_a_selector_then_both_real_task_suffixes() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for (capability, selector, class, choice) in [
        (0, 0xffff, 33, 3),
        (0xc00, 0, 9, 0),
        (1, 0, 10, 1),
        (0xc01, 0xffff, 33, 3),
    ] {
        let mut manager = generic(&session, &metadata);
        let mut earlier = generic(&session, &metadata);
        let owner = manager.entity_mut(5).unwrap();
        let candidate = earlier.entity_mut(1).unwrap();
        candidate.set_position_raw(owner.position_raw());
        candidate.capability_flags = capability;
        candidate.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
        let mut words = [0x1700, selector, 0x1234, 0x9876].into_iter();
        let mut draws = 0;
        let publication = publish_intro2_type17(
            owner,
            &metadata[17],
            std::slice::from_ref(candidate),
            session.cache.terrain().unwrap(),
            &mut || {
                draws += 1;
                words.next().unwrap()
            },
        )
        .unwrap();
        assert_eq!(draws, 4);
        assert_eq!(publication.selector_word, selector);
        assert_eq!(publication.selection.program.class_id, class);
        assert_eq!(publication.selection.choice_index, choice);
        assert_eq!(publication.people_nearby, capability & 0xc00 != 0);
        assert_eq!(publication.player_nearby, capability & 1 != 0);
        assert!(!publication.initializer_fallback);
        assert!(matches!(
            owner.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(_))
        ));
        match owner.actor_task_state(ActorTaskSlot::Secondary).unwrap() {
            ActorTaskRuntime::FollowBeaconAcquisition(_) => assert_eq!(class, 33),
            ActorTaskRuntime::TargetAcquisition(task) => {
                assert!(matches!(class, 9 | 10));
                assert_eq!(
                    task.constructor_filter_override_raw(),
                    if class == 9 { 0xc00 } else { 0 }
                );
            }
            task => panic!("wrong native acquiring task: {task:?}"),
        }
        assert_eq!(owner.actor_task_state(ActorTaskSlot::Tertiary), None);
        let RetailRuntimeValue::Known(Some(sub_a)) = owner.sub_a_propulsion_runtime else {
            panic!()
        };
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(shared_initializer_target_speed_raw(250, 0x9876))
        );
        let RetailRuntimeValue::Known(Some(sub_h)) = &owner.sub_h_external_frame_runtime else {
            panic!()
        };
        assert!(sub_h.is_enabled());
        assert_eq!(sub_h.records().len(), 8);
        assert!(sub_h.records().iter().all(|record| record.flags_raw == 0));
    }
}

#[v2k_test_support::retail_test]
fn native_type17_birth_rejects_wrong_metadata_and_future_prefix_without_rng() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for invalid_metadata in [true, false] {
        let mut manager = generic(&session, &metadata);
        let future = generic(&session, &metadata);
        let mut profile = metadata[17].clone();
        if invalid_metadata {
            profile
                .initializer
                .as_mut()
                .unwrap()
                .common_axis_descriptor
                .raw_word_at_0x04 ^= 1;
        }
        let owner = manager.entity_mut(5).unwrap();
        let future_entity = future
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(30))
            .unwrap();
        let before_flags = owner.collision.state_flags_at_0x08;
        let before_a = owner.sub_a_propulsion_runtime;
        let mut draws = 0;
        let result = publish_intro2_type17(
            owner,
            &profile,
            if invalid_metadata {
                &[]
            } else {
                std::slice::from_ref(future_entity)
            },
            session.cache.terrain().unwrap(),
            &mut || {
                draws += 1;
                0
            },
        );
        assert_eq!(
            result,
            Err(if invalid_metadata {
                Intro2Type17Error::Metadata
            } else {
                Intro2Type17Error::Prefix
            })
        );
        assert_eq!(draws, 0);
        assert_eq!(owner.collision.state_flags_at_0x08, before_flags);
        assert_eq!(owner.sub_a_propulsion_runtime, before_a);
        assert!(owner.intro2_type17_runtime.is_none());
        assert!(owner.type17_sub_d_frame_owner.is_none());
    }
}

#[v2k_test_support::retail_test]
fn native_type17_both_allocations_retain_their_own_first_query_and_birth_only_mass() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
        crate::entity::Intro2BirthSelection::default(),
        &mut || 0,
    )
    .unwrap();
    for (spawn, seed) in [(4, 0x04), (30, 0x17)] {
        let entity = manager.entity_mut(spawn as u32 + 1).unwrap();
        assert!(intro2_type17_allocation_authenticates(entity));
        let owner = entity.type17_sub_d_frame_owner.unwrap();
        assert_eq!(owner.classifier_cache().stagger_counter(), seed);
        assert_eq!(
            owner.classifier_cache().origin(),
            RetailRuntimeValue::Unresolved
        );
        assert!(owner.classifier_cache().can_classify());
        assert!(entity.type17_sub_d_runtime.is_some());
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
        let mut draws = 0;
        assert_eq!(
            publish_intro2_type17(
                entity,
                &metadata[17],
                &[],
                session.cache.terrain().unwrap(),
                &mut || {
                    draws += 1;
                    0
                }
            ),
            Err(Intro2Type17Error::AlreadyPublished)
        );
        assert_eq!(draws, 0);
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(73)
        );
        entity.set_position_raw([x.wrapping_add(456), y, z.wrapping_sub(789)]);
        assert!(intro2_type17_allocation_authenticates(entity));
        entity.id += 1000;
        assert!(!intro2_type17_allocation_authenticates(entity));
    }
}

#[v2k_test_support::retail_test]
fn native_type17_reselection_reuses_components_and_preserves_follow_axis() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let entity = manager.entity_mut(5).unwrap();
    let publication = publish_intro2_type17(
        entity,
        &metadata[17],
        &[],
        session.cache.terrain().unwrap(),
        &mut || 0,
    )
    .unwrap();
    let receipt = entity.intro2_type17_runtime;
    let sub_d = entity.type17_sub_d_runtime;
    let sub_d_owner = entity.type17_sub_d_frame_owner;
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0xffed);
    let live_axis = CommonAxisDescriptor {
        strict_axis_limit_raw: 1792,
        raw_word_at_0x04: 0x100,
    };
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(live_axis);
    for class in [33, 10, 9] {
        let program = behavior_program(class).unwrap();
        let selection = BehaviorSelection {
            program,
            ..publication.selection
        };
        let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection).unwrap();
        let mut draws = 0;
        assert!(native::publish_acquiring(
            entity,
            &metadata[17],
            selection,
            context,
            &mut || {
                draws += 1;
                0x5555
            }
        ));
        assert_eq!(draws, 2);
        assert_eq!(entity.intro2_type17_runtime, receipt);
        assert_eq!(entity.type17_sub_d_runtime, sub_d);
        assert_eq!(entity.type17_sub_d_frame_owner, sub_d_owner);
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0xffed)
        );
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(CommonAxisDescriptor {
                strict_axis_limit_raw: live_axis.strict_axis_limit_raw,
                raw_word_at_0x04: if class == 33 {
                    0x100
                } else {
                    AXIS.raw_word_at_0x04
                },
            })
        );
    }
}
