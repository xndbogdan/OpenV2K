use super::*;
use crate::{
    entity::{EntityConstructionResources, EntityManager},
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

pub(crate) fn generic(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
) -> EntityManager {
    EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    )
}

pub(crate) fn publish(
    manager: &mut EntityManager,
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
    spawn: usize,
) -> u32 {
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(spawn))
        .unwrap()
        .id;
    let profile_metadata = &metadata[manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .entity_type as usize];
    publish_intro2_gun_turret(
        manager.entity_mut(id).unwrap(),
        profile_metadata,
        session.cache.terrain().unwrap(),
        &mut || 0,
    )
    .unwrap();
    id
}

#[v2k_test_support::retail_test]
fn native_type102_birth_publishes_one_word_tertiary_and_real_model_bindings() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for (spawn, xz) in [
        (53, [0x6400, 0xf500u16 as i16]),
        (54, [0x6800, 0xeb00u16 as i16]),
    ] {
        let mut manager = generic(&session, &metadata);
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn))
            .unwrap()
            .id;
        let entity = manager.entity_mut(id).unwrap();
        let old_basis = entity.physical_body_basis_q31;
        let old_flags = entity.collision.state_flags_at_0x08;
        let mut draws = 0;
        let publication = publish_intro2_gun_turret(
            entity,
            &metadata[102],
            session.cache.terrain().unwrap(),
            &mut || {
                draws += 1;
                0x8123
            },
        )
        .unwrap();
        assert_eq!(draws, 1);
        assert_eq!(publication.selector_word, 0x8123);
        assert_eq!(publication.selection.program.class_id, 29);
        assert!(intro2_gun_turret_allocation_authenticates(entity));
        assert_eq!(
            entity.position_raw(),
            [
                xz[0],
                session
                    .cache
                    .terrain()
                    .unwrap()
                    .bilinear_height_raw(xz[0], xz[1]),
                xz[1]
            ]
        );
        assert_eq!(entity.physical_body_basis_q31, old_basis);
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x128),
            RetailRuntimeValue::Known(0x128)
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(!0x128),
            old_flags.masked(!0x128)
        );
        assert_eq!(
            entity.collision.default_state_flags_at_0xc8,
            RetailRuntimeValue::Known(0x25025)
        );
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(
            matches!(entity.actor_task_state(ActorTaskSlot::Tertiary),Some(ActorTaskRuntime::Intro2GunTurret(state))
            if *state == task::GunTurretTaskState::from_408df0(0x44,[0;2],false))
        );
        let runtime = entity.intro2_gun_turret_runtime.unwrap();
        assert_eq!(runtime.sub_l_output_raw, [0; 2]);
        assert_eq!(runtime.sub_l_target_raw, [0; 3]);
        assert_eq!(runtime.sub_l_exact_raw, 0);
        assert_eq!(runtime.sub_e_joint_word_raw, 0);
        assert_eq!(runtime.sub_e_runtime.joint_bindings, [Some(3), None]);
        assert_eq!(runtime.sub_e_runtime.projectile_method, 14);
        assert_eq!(runtime.sub_e_runtime.sound_id, 78);
        assert_eq!(runtime.sub_e_runtime.cadence_raw, 0);
        assert_eq!(runtime.sub_e_runtime.direct_mode, 0);
    }
    let record = session.cache.global_entity_type(102).unwrap();
    //09A80 allocates type+110 words, and A950 maps selectors1..3 into them.
    assert_eq!(record.raw_header[0x110], 3);
    assert_eq!(metadata[102].model_slots, [162; 4]);
    assert_eq!(EMITTER.variable_bindings, [0, 3, 0, 0]);
    assert_eq!(&SUB_L[..2], &[1, 2]);
}

#[v2k_test_support::retail_test]
fn native_type102_reselection_replaces_wrapper_preserving_components_and_infected_prefix() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 53);
    let entity = manager.entity_mut(id).unwrap();
    let old_task = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Tertiary)
        .unwrap();
    let runtime = entity.intro2_gun_turret_runtime.as_mut().unwrap();
    runtime.sub_l_output_raw = [i16::MIN, 1234];
    runtime.sub_l_target_raw = [333, 444, 555];
    runtime.sub_l_exact_raw = -123;
    runtime.sub_e_joint_word_raw = 0x4321;
    runtime.sub_e_runtime.cadence_raw = -9988;
    let retained = *runtime;
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0xffed);
    entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(77);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x2000, 0x2000);
    let context = entity.current_behavior_context;
    let mut draws = 0;
    reselect_intro2_gun_turret(entity, &metadata[102], &mut || {
        draws += 1;
        0xffff
    })
    .unwrap();
    assert_eq!(draws, 1);
    assert_eq!(entity.intro2_gun_turret_runtime, Some(retained));
    assert_eq!(entity.current_behavior_context, context);
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0xffed)
    );
    assert_eq!(
        entity.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(77)
    );
    assert_eq!(entity.capability_flags, 0x48);
    assert!(entity.actor_tasks.task_state(old_task).is_none());
    assert_ne!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary),
        Some(old_task)
    );
    assert!(
        matches!(entity.actor_task_state(ActorTaskSlot::Tertiary),Some(ActorTaskRuntime::Intro2GunTurret(state))
        if *state == task::GunTurretTaskState::from_408df0(0x44,[i16::MIN,1234],false))
    );
    let old_task = entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary);
    reselect_intro2_gun_turret(entity, &metadata[102], &mut || 0).unwrap();
    assert_ne!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary),
        old_task
    );
    assert_eq!(entity.intro2_gun_turret_runtime, Some(retained));
    assert!(
        matches!(entity.actor_task_state(ActorTaskSlot::Tertiary),Some(ActorTaskRuntime::Intro2GunTurret(state))
        if *state == task::GunTurretTaskState::from_408df0(0x48,[i16::MIN,1234],false))
    );
}

#[v2k_test_support::retail_test]
fn native_type102_wrong_metadata_duplicate_and_dying_reentry_do_not_draw_or_replace() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(53))
        .unwrap()
        .id;
    let entity = manager.entity_mut(id).unwrap();
    let before_position = entity.position_raw();
    let mut wrong = metadata[102].clone();
    wrong.projectile_emitter_descriptor =
        RetailRuntimeValue::Known(Some(ProjectileEmitterDescriptor {
            variable_bindings: [0; 4],
            ..EMITTER
        }));
    assert_eq!(
        publish_intro2_gun_turret(
            entity,
            &wrong,
            session.cache.terrain().unwrap(),
            &mut || panic!("metadata rejection must precede RNG")
        ),
        Err(Intro2GunTurretError::Metadata)
    );
    assert_eq!(entity.position_raw(), before_position);
    assert!(entity.intro2_gun_turret_runtime.is_none());
    publish_intro2_gun_turret(
        entity,
        &metadata[102],
        session.cache.terrain().unwrap(),
        &mut || 0,
    )
    .unwrap();
    let task = entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary);
    assert_eq!(
        publish_intro2_gun_turret(
            entity,
            &metadata[102],
            session.cache.terrain().unwrap(),
            &mut || panic!("duplicate")
        ),
        Err(Intro2GunTurretError::AlreadyPublished)
    );
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x4000, 0x4000);
    assert_eq!(
        reselect_intro2_gun_turret(entity, &metadata[102], &mut || panic!(
            "alternate49 consumes no weighted word"
        )),
        Err(Intro2GunTurretError::AlternateBehavior)
    );
    assert_eq!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary),
        task
    );
    entity.authored_spawn_index = Some(52);
    assert!(!intro2_gun_turret_allocation_authenticates(entity));
}
