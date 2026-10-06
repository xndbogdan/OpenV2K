use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    common_mover::{
        shared_initializer_target_speed_raw,
        sub_d::{construct_native_sub_d, NativeSubDConstruction, SubDAllocationCounter},
        type9_attitude::Type9BodyBasis,
        SubAPropulsionRuntime,
    },
    entity::EntityConstructionResources,
    entity_collision_state::{
        RetailStateWord, ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT,
    },
};

fn native_sub_d(metadata: &EntityTypeRuntimeMetadata) -> NativeSubDConstruction {
    let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_d_steering_descriptor else {
        panic!("canonical Type17 Sub-D");
    };
    construct_native_sub_d(&mut SubDAllocationCounter::from_next_seed(0xd3), descriptor)
}

#[v2k_test_support::retail_test]
fn authored_birth_retains_actual_pose_native_sub_d_and_all_three_initial_programs() {
    let Some((mut session, metadata)) = tests::fixture() else {
        return;
    };
    let mut spawn = session.cache.level_desc().unwrap().entities[4].clone();
    // Controlled authored-input variation: no captured spawn/angle/seed policy
    // can authorize this request. The canonical table and actual manager lease
    // still own its components and publication.
    spawn.index = 99;
    spawn.rotation = [0x9000, 0x0800, 0xfc00];
    let x = spawn.position_raw()[0].wrapping_add(19);
    let z = spawn.position_raw()[2].wrapping_sub(37);
    spawn.pos_data_1[..2].copy_from_slice(&x.to_le_bytes());
    spawn.pos_data_2[..2].copy_from_slice(&z.to_le_bytes());
    let cell_index = usize::from((x as u16) >> 8) * 256 + usize::from((z as u16) >> 8);
    session.cache.level_terrain_mut().unwrap().cells[cell_index].terrain_type |= 0x10;
    for (param, capability, selector, class, choice) in [
        (0, 0, 0xffff, 33, 3),
        (1, 0xc00, 0, 9, 0),
        (0, 1, 0, 10, 1),
        (1, 0xc01, 0xffff, 33, 3),
    ] {
        spawn.param = param;
        let mut manager = tests::generic(&session, &metadata);
        let allocation = manager.main_base_abort_actor_observation(5).unwrap().lease;
        let mut predecessor_manager = tests::generic(&session, &metadata);
        let candidate = predecessor_manager.entity_mut(1).unwrap();
        // A persistent player has no Section13 index; the nearby evaluator
        // must nevertheless see it in the genuine preceding list position.
        candidate.authored_spawn_index = None;
        candidate.set_position_raw([
            x,
            session.cache.terrain().unwrap().bilinear_height_raw(x, z),
            z,
        ]);
        candidate.capability_flags = capability;
        candidate.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
        let entity = manager.entity_mut(5).unwrap();
        entity.authored_spawn_index = Some(spawn.index);
        entity.set_rotation_heading_pitch_roll_raw(spawn.rotation.map(|word| word as i16));
        let sub_d = native_sub_d(&metadata[17]);
        let mut words = [0x1700, selector, 0x1234, 0x9876].into_iter();
        let publication = publish_authored_type17(
            Type17AuthoredConstructionRequest {
                entity,
                allocation,
                metadata: &metadata[17],
                spawn: &spawn,
                preceding: std::slice::from_ref(candidate),
                resources: EntityConstructionResources::new(
                    session.cache.terrain(),
                    session.cache.terrain_objects(),
                ),
                constructor_surface_bits: 0,
                retail_tick: 0xffff_ffed,
                sub_d,
            },
            &mut || words.next().expect("exact four constructor words"),
        )
        .unwrap();
        assert!(words.next().is_none());
        assert_eq!(publication.selector_word, selector);
        assert_eq!(publication.selection.program.class_id, class);
        assert_eq!(publication.selection.choice_index, choice);
        assert!(!publication.initializer_fallback);
        assert_eq!(publication.player_nearby, capability & 1 != 0);
        assert_eq!(publication.people_nearby, capability & 0xc00 != 0);
        assert_eq!(entity.type17_sub_d_frame_owner, Some(sub_d.frame_owner));
        assert_eq!(entity.type17_sub_d_runtime, Some(sub_d.runtime));
        assert_eq!(
            entity.intro2_type17_runtime.unwrap().anchor_raw,
            entity.position_raw()
        );
        assert_eq!(
            entity.collision.active_model_slot(),
            RetailRuntimeValue::Known(2)
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x0100_0000),
            RetailRuntimeValue::Known(if param != 0 { 0x0100_0000 } else { 0 })
        );
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT),
            RetailRuntimeValue::Known(
                BODY_BASIS_REBUILT_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT
            )
        );
        let angles = spawn.rotation.map(|word| word as i16);
        assert_eq!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
                angles[0], angles[1], angles[2]
            ))
        );
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!()
        };
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(shared_initializer_target_speed_raw(250, 0x9876))
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(_))
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_some());
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
        assert!(type17_manager_allocation_authenticates(&manager, 5));
    }
}

#[v2k_test_support::retail_test]
fn native_receipt_requires_the_issuing_manager_and_reentry_never_resets_animation() {
    let Some((session, metadata)) = tests::fixture() else {
        return;
    };
    let spawn = &session.cache.level_desc().unwrap().entities[4];
    let mut first = tests::generic(&session, &metadata);
    let mut second = tests::generic(&session, &metadata);
    let allocation = first.main_base_abort_actor_observation(5).unwrap().lease;
    let sub_d = native_sub_d(&metadata[17]);
    publish_authored_type17(
        Type17AuthoredConstructionRequest {
            entity: first.entity_mut(5).unwrap(),
            allocation,
            metadata: &metadata[17],
            spawn,
            preceding: &[],
            resources: EntityConstructionResources::new(
                session.cache.terrain(),
                session.cache.terrain_objects(),
            ),
            constructor_surface_bits: 0,
            retail_tick: 327,
            sub_d,
        },
        &mut || 0,
    )
    .unwrap();
    let entity = first.entity_mut(5).unwrap();
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(73);
    let receipt = entity.intro2_type17_runtime;
    second.entity_mut(5).unwrap().intro2_type17_runtime = receipt;
    assert!(intro2_type17_allocation_authenticates(
        second.entity_mut(5).unwrap()
    ));
    assert!(!type17_manager_allocation_authenticates(&second, 5));
    assert!(type17_manager_allocation_authenticates(&first, 5));
    let mut draws = 0;
    assert_eq!(
        publish_authored_type17(
            Type17AuthoredConstructionRequest {
                entity: first.entity_mut(5).unwrap(),
                allocation,
                metadata: &metadata[17],
                spawn,
                preceding: &[],
                resources: EntityConstructionResources::new(
                    session.cache.terrain(),
                    session.cache.terrain_objects()
                ),
                constructor_surface_bits: 0,
                retail_tick: 328,
                sub_d,
            },
            &mut || {
                draws += 1;
                0
            }
        ),
        Err(Intro2Type17Error::AlreadyPublished)
    );
    assert_eq!(draws, 0);
    assert_eq!(
        first
            .entity_mut(5)
            .unwrap()
            .collision
            .animation_offset_at_0xb2,
        RetailRuntimeValue::Known(73)
    );
}

#[v2k_test_support::retail_test]
fn native_birth_preflight_is_read_only_but_late_selector_failure_retains_20450() {
    let Some((session, metadata)) = tests::fixture() else {
        return;
    };
    let spawn = &session.cache.level_desc().unwrap().entities[4];
    for missing_terrain in [true, false] {
        let mut manager = tests::generic(&session, &metadata);
        let allocation = manager.main_base_abort_actor_observation(5).unwrap().lease;
        let mut prefix = tests::generic(&session, &metadata);
        let candidate = prefix.entity_mut(1).unwrap();
        candidate.capability_flags = 1;
        candidate.collision.state_flags_at_0x08 = RetailStateWord::unknown();
        let entity = manager.entity_mut(5).unwrap();
        let before_state = entity.collision.state_flags_at_0x08;
        let before_a = entity.sub_a_propulsion_runtime;
        let sub_d = native_sub_d(&metadata[17]);
        let mut draws = 0;
        let result = publish_authored_type17(
            Type17AuthoredConstructionRequest {
                entity,
                allocation,
                metadata: &metadata[17],
                spawn,
                preceding: std::slice::from_ref(candidate),
                resources: EntityConstructionResources::new(
                    if missing_terrain {
                        None
                    } else {
                        session.cache.terrain()
                    },
                    session.cache.terrain_objects(),
                ),
                constructor_surface_bits: 0,
                retail_tick: 111,
                sub_d,
            },
            &mut || {
                draws += 1;
                0x4321
            },
        );
        if missing_terrain {
            assert_eq!(
                result,
                Err(Intro2Type17Error::Runtime("constructor terrain"))
            );
            assert_eq!(draws, 0);
            assert_eq!(entity.collision.state_flags_at_0x08, before_state);
            assert_eq!(entity.sub_a_propulsion_runtime, before_a);
            assert!(entity.type17_sub_d_runtime.is_none());
        } else {
            assert!(
                matches!(result, Err(Intro2Type17Error::Selection(_))),
                "{result:?}"
            );
            assert_eq!(draws, 1);
            let RetailRuntimeValue::Known(Some(descriptor)) =
                metadata[17].sub_a_propulsion_descriptor
            else {
                panic!()
            };
            assert_eq!(
                entity.sub_a_propulsion_runtime,
                RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_20450_constructor(
                    descriptor, 0x4321
                )))
            );
            assert_eq!(entity.type17_sub_d_runtime, Some(sub_d.runtime));
            assert_eq!(entity.type17_sub_d_frame_owner, Some(sub_d.frame_owner));
        }
        assert!(entity.intro2_type17_runtime.is_none());
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_task_state(slot).is_none()));
    }
}
