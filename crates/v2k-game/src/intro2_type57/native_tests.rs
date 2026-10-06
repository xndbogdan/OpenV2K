use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    common_mover::sub_d::Type9SubDRuntime,
    entity::{EntityConstructionResources, EntityManager},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    session::GameSession,
    sub_g_runtime::SubG06070RuntimeState,
};

pub(in crate::intro2_type57) fn fixture() -> Option<(GameSession, Vec<EntityTypeRuntimeMetadata>)> {
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

pub(in crate::intro2_type57) fn generic(
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
fn native_type57_birth_owns_four_draws_authored_pose_and_deglk_allocations() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    authenticate_metadata(&metadata[57]).unwrap();
    let mut manager = generic(&session, &metadata);
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(1))
        .unwrap()
        .id;
    let entity = manager.entity_mut(id).unwrap();
    let initial_basis = entity.physical_body_basis_q31();
    let initial_flags = entity.collision.state_flags_at_0x08;
    let birth_b2 = entity.collision.animation_offset_at_0xb2;
    let words = [0x1100, 0xffff, 0x2200, 0x3300];
    let mut draws = 0;
    let publication = publish_intro2_type57(entity, &metadata[57], &mut || {
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
    assert!(intro2_type57_allocation_authenticates(entity));
    assert!(crate::opening::intro2_uses_live_actor_pose(entity));
    assert_eq!(entity.position_raw(), [0xB400u16 as i16, 0x0200, 0x0600]);
    assert_eq!(entity.physical_body_basis_q31(), initial_basis);
    assert_eq!(entity.rotation_heading_pitch_roll_raw(), [0, 0, 0]);
    assert_eq!(entity.collision.animation_offset_at_0xb2, birth_b2);
    assert_eq!(entity.collision.state_flags_at_0x08, initial_flags);
    let mut expected_g = SubG06070RuntimeState::from_1b8c0_constructor(&SUB_G, 0x1100);
    expected_g.apply_shared_06070_sub_g_branch(700 + 0x22, 0);
    expected_g.apply_shared_06070_sub_g_branch(700 + 0x33, 0);
    assert_eq!(
        entity.sub_g_06070_runtime,
        RetailRuntimeValue::Known(Some(expected_g))
    );
    let runtime = entity.intro2_type57_runtime.unwrap();
    assert_eq!(runtime.sub_d_runtime, Type9SubDRuntime::from_constructor());
    assert_eq!(
        runtime
            .sub_d_frame_owner
            .classifier_cache()
            .stagger_counter(),
        0x01
    );
    assert_eq!(runtime.sub_e_runtime.projectile_method, 24);
    assert_eq!(runtime.sub_e_runtime.sound_id, 93);
    assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_some());
    assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_some());
    assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
}

#[v2k_test_support::retail_test]
fn native_type57_cannot_borrow_type10_identity() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let dragon = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(55))
        .unwrap()
        .id;
    assert_eq!(
        publish_intro2_type57(
            manager.entity_mut(dragon).unwrap(),
            &metadata[57],
            &mut || 0
        ),
        Err(Intro2Type57Error::Identity)
    );
    assert!(!intro2_type57_allocation_authenticates(
        manager.entity_mut(dragon).unwrap()
    ));
}
