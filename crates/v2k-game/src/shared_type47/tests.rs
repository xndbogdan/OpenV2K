use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    common_mover::{
        shared_initializer_target_speed_raw,
        sub_d::{construct_native_sub_d, SubDAllocationCounter},
    },
    session::GameSession,
};

fn fixture() -> Option<(GameSession, Vec<EntityTypeRuntimeMetadata>)> {
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

fn generic(session: &GameSession, metadata: &[EntityTypeRuntimeMetadata]) -> EntityManager {
    EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    )
}

fn native_sub_d(metadata: &EntityTypeRuntimeMetadata) -> NativeSubDConstruction {
    let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_d_steering_descriptor else {
        panic!("canonical Type47 Sub-D");
    };
    construct_native_sub_d(&mut SubDAllocationCounter::from_next_seed(0xd3), descriptor)
}

/// Real canonical Section12/13 through the production constructor, with an
/// explicit native process counter. No captured seed or hand-built receipt.
pub(crate) fn native_fixture() -> Option<(GameSession, EntityManager, u32)> {
    let (session, metadata) = fixture()?;
    let mut manager = generic(&session, &metadata);
    let id = 7;
    let allocation = manager.main_base_abort_actor_observation(id).unwrap().lease;
    publish_authored_type47(
        Type47AuthoredConstructionRequest {
            entity: manager.entity_mut(id).unwrap(),
            allocation,
            metadata: &metadata[47],
            spawn: &session.cache.level_desc().unwrap().entities[6],
            resources: EntityConstructionResources::new(
                session.cache.terrain(),
                session.cache.terrain_objects(),
            ),
            constructor_surface_bits: 0,
            sub_d: native_sub_d(&metadata[47]),
        },
        &mut || 0,
    )
    .unwrap();
    Some((session, manager, id))
}

#[v2k_test_support::retail_test]
fn native_birth_uses_actual_authored_anchor_angles_seed_and_guard_or_wander_draws() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    let mut spawn = session.cache.level_desc().unwrap().entities[6].clone();
    // Deliberately outside every captured cohort, with actual authored Euler
    // and terrain changes. The shared constructor must not replay spawn6.
    spawn.index = 99;
    spawn.rotation = [0x9000, 0x0800, 0xfc00];
    let [x, _, z] = spawn.position_raw();
    let x = x.wrapping_add(29);
    let z = z.wrapping_sub(37);
    spawn.pos_data_1[..2].copy_from_slice(&x.to_le_bytes());
    spawn.pos_data_2[..2].copy_from_slice(&z.to_le_bytes());
    let index = usize::from((x as u16) >> 8) * 256 + usize::from((z as u16) >> 8);
    session.cache.level_terrain_mut().unwrap().cells[index].terrain_type |= 0x10;
    for (selector, class, expected_draws, last_word) in [(0, 32, 4, 0x9876), (0xffff, 6, 3, 0x1234)]
    {
        let mut manager = generic(&session, &metadata);
        let allocation = manager.main_base_abort_actor_observation(7).unwrap().lease;
        let entity = manager.entity_mut(7).unwrap();
        entity.authored_spawn_index = Some(spawn.index);
        entity.set_rotation_heading_pitch_roll_raw(spawn.rotation.map(|word| word as i16));
        let sub_d = native_sub_d(&metadata[47]);
        let words = [0x1700, selector, 0x1234, 0x9876];
        let mut draws = 0;
        let admission = publish_authored_type47(
            Type47AuthoredConstructionRequest {
                entity,
                allocation,
                metadata: &metadata[47],
                spawn: &spawn,
                resources: EntityConstructionResources::new(
                    session.cache.terrain(),
                    session.cache.terrain_objects(),
                ),
                constructor_surface_bits: 0,
                sub_d,
            },
            &mut || {
                let word = words[draws];
                draws += 1;
                word
            },
        )
        .unwrap();
        assert_eq!(draws, expected_draws);
        assert_eq!(
            admission,
            Intro2Type47Admission {
                spawn_index: 99,
                seed: 0xd3
            }
        );
        assert_eq!(
            entity.intro2_type47_sub_d_frame_owner,
            Some(sub_d.frame_owner)
        );
        assert_eq!(entity.intro2_type47_sub_d_runtime, Some(sub_d.runtime));
        assert_eq!(
            entity.type47_immutable_anchor_raw_at_0x90,
            RetailRuntimeValue::Known([
                x,
                session
                    .cache
                    .terrain()
                    .unwrap()
                    .bilinear_height_raw(x, z)
                    .wrapping_add(50),
                z
            ])
        );
        assert_eq!(
            entity.collision.active_model_slot(),
            RetailRuntimeValue::Known(2)
        );
        let [h, p, r] = spawn.rotation.map(|word| word as i16);
        assert_eq!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(h, p, r))
        );
        let RetailRuntimeValue::Known(Some(selection)) = entity.initial_behavior else {
            panic!()
        };
        assert_eq!(selection.program.class_id, class);
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!()
        };
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(shared_initializer_target_speed_raw(300, last_word))
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::OrdinaryType9Wander(_))
        ));
        assert_eq!(
            entity.actor_task_state(ActorTaskSlot::Secondary).is_some(),
            class == 32
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        assert!(crate::intro2_type47_live::intro2_type47_cohort_runtime_authenticates(entity));
        assert!(type47_manager_allocation_authenticates(&manager, 7));
    }
}

#[v2k_test_support::retail_test]
fn native_receipt_rejects_manager_transplant_and_birth_retry_preserves_live_words() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let spawn = &session.cache.level_desc().unwrap().entities[6];
    let mut manager = generic(&session, &metadata);
    let mut foreign = generic(&session, &metadata);
    let allocation = manager.main_base_abort_actor_observation(7).unwrap().lease;
    let sub_d = native_sub_d(&metadata[47]);
    publish_authored_type47(
        Type47AuthoredConstructionRequest {
            entity: manager.entity_mut(7).unwrap(),
            allocation,
            metadata: &metadata[47],
            spawn,
            resources: EntityConstructionResources::new(
                session.cache.terrain(),
                session.cache.terrain_objects(),
            ),
            constructor_surface_bits: 0,
            sub_d,
        },
        &mut || 0,
    )
    .unwrap();
    let receipt = manager
        .entity_mut(7)
        .unwrap()
        .native_type47_construction
        .unwrap();
    foreign.entity_mut(7).unwrap().native_type47_construction = Some(receipt);
    foreign
        .entity_mut(7)
        .unwrap()
        .type47_immutable_anchor_raw_at_0x90 = manager
        .entity_mut(7)
        .unwrap()
        .type47_immutable_anchor_raw_at_0x90;
    assert!(receipt.entity_authenticates(foreign.entity_mut(7).unwrap()));
    assert!(!type47_manager_allocation_authenticates(&foreign, 7));
    assert!(type47_manager_allocation_authenticates(&manager, 7));
    let entity = manager.entity_mut(7).unwrap();
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(73);
    let tasks = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
        (
            entity.actor_tasks.task_in_slot(slot),
            entity.actor_task_state(slot).cloned(),
        )
    });
    let result = publish_authored_type47(
        Type47AuthoredConstructionRequest {
            entity,
            allocation,
            metadata: &metadata[47],
            spawn,
            resources: EntityConstructionResources::new(
                session.cache.terrain(),
                session.cache.terrain_objects(),
            ),
            constructor_surface_bits: 0,
            sub_d,
        },
        &mut || panic!("replay consumes no word"),
    );
    assert_eq!(result, Err(Intro2Type47PublicationError::AlreadyPublished));
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(73)
    );
    assert_eq!(
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| (
            entity.actor_tasks.task_in_slot(slot),
            entity.actor_task_state(slot).cloned(),
        )),
        tasks
    );
}

#[v2k_test_support::retail_test]
fn unsupported_native_model_and_missing_terrain_fail_before_body_or_rng_prefix() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for wrong_model in [false, true] {
        let mut spawn = session.cache.level_desc().unwrap().entities[6].clone();
        if wrong_model {
            spawn.model_overrides[0] = 303;
        }
        let mut manager = generic(&session, &metadata);
        let allocation = manager.main_base_abort_actor_observation(7).unwrap().lease;
        let entity = manager.entity_mut(7).unwrap();
        let state = entity.collision.state_flags_at_0x08;
        let sub_a = entity.sub_a_propulsion_runtime;
        let result = publish_authored_type47(
            Type47AuthoredConstructionRequest {
                entity,
                allocation,
                metadata: &metadata[47],
                spawn: &spawn,
                resources: EntityConstructionResources::new(None, session.cache.terrain_objects()),
                constructor_surface_bits: 0,
                sub_d: native_sub_d(&metadata[47]),
            },
            &mut || panic!("preflight consumes no word"),
        );
        assert!(result.is_err());
        assert_eq!(entity.collision.state_flags_at_0x08, state);
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a);
        assert!(entity.native_type47_construction.is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_none());
    }
}

#[v2k_test_support::retail_test]
fn native_guard_stops_after_mover_block_before_acquisition_rng() {
    use crate::intro2_type47_live::{
        tick_intro2_type47_scheduler_owner, Intro2Type47CallbackFrame, Intro2Type47SchedulerOwner,
        Intro2Type47SchedulerProductionBlock, Intro2Type47SchedulerProductionOutcome,
    };
    let Some((_session, mut manager, id)) = native_fixture() else {
        return;
    };
    let entity = manager.entity_mut(id).unwrap();
    let owner = Intro2Type47SchedulerOwner::adopt_published(entity).unwrap();
    let secondary = entity.actor_task_state(ActorTaskSlot::Secondary).cloned();
    let mut fx = crate::world_fx::WorldFx::new();
    let mut draws = 0;
    let tick = tick_intro2_type47_scheduler_owner(
        &mut manager,
        owner,
        &mut fx,
        None,
        Intro2Type47CallbackFrame {
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
        },
        &mut |_| {
            draws += 1;
            1
        },
    );
    assert!(matches!(
        tick.outcome,
        Intro2Type47SchedulerProductionOutcome::Blocked {
            reason: Intro2Type47SchedulerProductionBlock::CommonMoverUnavailable,
            ..
        }
    ));
    assert!(tick.retained_owner.is_some());
    assert_eq!(draws, 1, "02EB0 ran; blocked01430 must not enter01FB0");
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .actor_task_state(ActorTaskSlot::Secondary)
            .cloned(),
        secondary
    );
}

#[v2k_test_support::retail_test]
fn guard_and_wander_primary_expire_strictly_after_five_seconds_and_visit_new_later_slots() {
    use crate::intro2_type47_live::{
        tick_intro2_type47_scheduler_owner, Intro2Type47CallbackFrame, Intro2Type47SchedulerOwner,
        Intro2Type47SchedulerProductionOutcome,
    };
    use crate::type47_scheduler_production::Type47SchedulerC690Transition;
    let Some((session, mut manager, id)) = native_fixture() else {
        return;
    };
    let metadata =
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(47).unwrap());
    let mut owner =
        Intro2Type47SchedulerOwner::adopt_published(manager.entity_mut(id).unwrap()).unwrap();
    let mut fx = crate::world_fx::WorldFx::new();
    let frame = Intro2Type47CallbackFrame {
        elapsed_micros: 20_000,
        global_elapsed_micros: 20_000,
    };
    // Run actual callbacks to the exact boundary. A1 at every retarget and
    // acquisition gate avoids extra retarget words and acquired entities.
    for expected_class in [32, 6] {
        let old_primary = manager
            .entity_mut(id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        for _ in 0..250 {
            let tick = tick_intro2_type47_scheduler_owner(
                &mut manager,
                owner,
                &mut fx,
                session.cache.terrain(),
                frame,
                &mut |_| 1,
            );
            assert!(
                matches!(
                    tick.outcome,
                    Intro2Type47SchedulerProductionOutcome::Primary { .. }
                        | Intro2Type47SchedulerProductionOutcome::WanderNear { .. }
                ),
                "{:?}",
                tick.outcome
            );
            owner = tick.retained_owner.expect("completed native owner");
        }
        let entity = manager.entity_mut(id).unwrap();
        let Some(ActorTaskRuntime::OrdinaryType9Wander(task)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!()
        };
        assert_eq!(task.elapsed_ms(), 5000);
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            Some(old_primary)
        );
        let mut supplied = if expected_class == 32 {
            vec![1, 0xffff]
        } else {
            vec![1, 0, 1]
        }
        .into_iter();
        let tick = tick_intro2_type47_scheduler_owner(
            &mut manager,
            owner,
            &mut fx,
            session.cache.terrain(),
            frame,
            &mut |_| {
                supplied
                    .next()
                    .expect("retarget, C690 selector, then new Guard acquisition only")
            },
        );
        assert!(supplied.next().is_none());
        let Intro2Type47SchedulerProductionOutcome::RootTransition {
            transition,
            post_primary_guard,
            ..
        } = tick.outcome
        else {
            panic!(
                "expected strict Primary timeout transition: {:?}",
                tick.outcome
            )
        };
        if expected_class == 32 {
            assert!(matches!(
                transition,
                Type47SchedulerC690Transition::WanderPublished { .. }
            ));
            assert!(post_primary_guard.is_none());
        } else {
            assert!(matches!(
                transition,
                Type47SchedulerC690Transition::GuardPublished { .. }
            ));
            assert!(
                post_primary_guard.is_some(),
                "new Secondary must run this pass"
            );
        }
        owner = tick
            .retained_owner
            .expect("new graph retains native custody");
        let entity = manager.entity_mut(id).unwrap();
        let new_primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        assert_ne!(new_primary, old_primary);
        assert!(
            entity.actor_tasks.wrapper_flags(old_primary).is_none(),
            "old callback unwound before replacement"
        );
        let Some(ActorTaskRuntime::OrdinaryType9Wander(task)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!()
        };
        assert_eq!(
            task.elapsed_ms(),
            0,
            "new Primary cannot run twice in the same physical-slot pass"
        );
        assert_eq!(
            entity.native_type47_construction.unwrap().sub_d_seed(),
            0xd3
        );
        assert_ne!(
            entity.intro2_type47_sub_d_frame_owner,
            Some(native_sub_d(&metadata).frame_owner),
            "C690 must retain progressed Sub-D cache"
        );
    }
}
