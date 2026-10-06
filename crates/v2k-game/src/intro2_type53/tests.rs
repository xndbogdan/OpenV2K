use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    common_mover::shared_initializer_target_speed_raw,
    entity::{EntityConstructionResources, EntityManager},
    entity_collision_state::RetailStateWord,
    session::GameSession,
};

pub(super) fn fixture() -> Option<(GameSession, Vec<EntityTypeRuntimeMetadata>)> {
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
fn weighted_birth_keeps_sub_a_constructor_then_selector_and_both_suffixes() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    // One already-linked candidate supplies each explicit capability predicate.
    // The zero selector remains in Always's bucket even when both are present.
    for (capability, selector, class, choice) in [
        (0, 0xffff, 33, 0),
        (0xc00, 0xffff, 9, 2),
        (1, 0xffff, 7, 3),
        (0xc01, 0, 33, 0),
    ] {
        let mut manager = generic(&session, &metadata);
        let mut preceding = generic(&session, &metadata);
        let owner = manager.entity_mut(21).unwrap();
        let person = preceding.entity_mut(1).unwrap();
        person.set_position_raw(owner.position_raw());
        person.capability_flags = capability;
        person.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
        let mut words = [0x5300, selector, 0x1234, 0x9876].into_iter();
        let mut draws = 0;
        let result = publish_intro2_type53(
            owner,
            &metadata[53],
            std::slice::from_ref(person),
            session.cache.terrain().unwrap(),
            &mut || {
                draws += 1;
                words.next().unwrap()
            },
        )
        .unwrap();
        assert_eq!(draws, 4);
        assert_eq!(result.selector_word, selector);
        assert!(!result.initializer_fallback);
        assert_eq!(result.selection.choice_index, choice);
        assert_eq!(result.selection.program.class_id, class);
        assert_eq!(result.people_nearby, capability & 0xc00 != 0);
        assert_eq!(result.player_nearby, capability & 1 != 0);
        assert!(matches!(
            owner.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(_))
        ));
        match owner.actor_task_state(ActorTaskSlot::Secondary).unwrap() {
            ActorTaskRuntime::FollowBeaconAcquisition(_) => assert_eq!(class, 33),
            ActorTaskRuntime::TargetAcquisition(task) => {
                assert_ne!(class, 33);
                assert_eq!(
                    task.constructor_filter_override_raw(),
                    if class == 9 { 0xc00 } else { 0 }
                );
            }
            other => panic!("wrong acquiring task: {other:?}"),
        }
        assert_eq!(owner.actor_task_state(ActorTaskSlot::Tertiary), None);
        let RetailRuntimeValue::Known(Some(sub_a)) = owner.sub_a_propulsion_runtime else {
            panic!()
        };
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(shared_initializer_target_speed_raw(400, 0x9876))
        );
        let RetailRuntimeValue::Known(Some(sub_h)) = &owner.sub_h_external_frame_runtime else {
            panic!()
        };
        assert!(sub_h.is_enabled());
        assert_eq!(sub_h.records().len(), 6);
    }
}

#[v2k_test_support::retail_test]
fn selection_rejects_future_allocations_before_rng_or_publication() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let mut future = generic(&session, &metadata);
    let owner = manager.entity_mut(21).unwrap();
    let future_actor = future.entity_mut(39).unwrap();
    future_actor.capability_flags = 0xc01;
    future_actor.set_position_raw(owner.position_raw());
    future_actor.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
    let mut draws = 0;
    assert_eq!(
        publish_intro2_type53(
            owner,
            &metadata[53],
            std::slice::from_ref(future_actor),
            session.cache.terrain().unwrap(),
            &mut || {
                draws += 1;
                0xffff
            }
        ),
        Err(Intro2Type53Error::Prefix)
    );
    assert_eq!(draws, 0);
    assert!(owner.intro2_type53_runtime.is_none());
    assert_eq!(
        owner.current_behavior_context,
        RetailRuntimeValue::Unresolved
    );
}

#[v2k_test_support::retail_test]
fn native_birth_retains_inactive_components_and_only_its_own_sub_d_query_receipt() {
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
    for (spawn, seed) in [(20, 0x13), (26, 0x16), (38, 0x18), (41, 0x1a)] {
        let entity = manager.entity_mut(spawn as u32 + 1).unwrap();
        assert!(intro2_type53_allocation_authenticates(entity));
        let runtime = entity.intro2_type53_runtime.unwrap();
        assert_eq!(
            runtime.sub_d_owner.classifier_cache().stagger_counter(),
            seed
        );
        assert_eq!(
            runtime.sub_d_owner.classifier_cache().origin(),
            RetailRuntimeValue::Unresolved
        );
        assert!(
            runtime.sub_d_owner.classifier_cache().can_classify(),
            "each allocation has its own accepted first-query chain"
        );
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
                .wrapping_add(50)
        );
        // A retained native allocation is not authenticated by a fixed pose.
        entity.set_position_raw([x.wrapping_add(17), y.wrapping_add(9), z]);
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(19);
        assert!(intro2_type53_allocation_authenticates(entity));
    }
    // Type122 shares choice data, but has another model, health and Sub-A profile.
    assert!(manager
        .entity_mut(22)
        .unwrap()
        .intro2_type53_runtime
        .is_none());
}

#[v2k_test_support::retail_test]
fn generic_or_wrong_type_profile_cannot_create_native_custody() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let entity = manager.entity_mut(21).unwrap();
    assert!(!intro2_type53_allocation_authenticates(entity));
    let mut draws = 0;
    assert_eq!(
        publish_intro2_type53(
            entity,
            &metadata[122],
            &[],
            session.cache.terrain().unwrap(),
            &mut || {
                draws += 1;
                0
            }
        ),
        Err(Intro2Type53Error::Metadata)
    );
    assert_eq!(draws, 0);
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Unresolved
    );
}

#[v2k_test_support::retail_test]
fn repeated_birth_cannot_clear_later_transient_mass_or_tasks() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let entity = manager.entity_mut(21).unwrap();
    publish_intro2_type53(
        entity,
        &metadata[53],
        &[],
        session.cache.terrain().unwrap(),
        &mut || 0,
    )
    .unwrap();
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(27);
    let task = entity.actor_task_state(ActorTaskSlot::Primary).cloned();
    assert_eq!(
        publish_intro2_type53(
            entity,
            &metadata[53],
            &[],
            session.cache.terrain().unwrap(),
            &mut || panic!("a second birth cannot consume RNG")
        ),
        Err(Intro2Type53Error::AlreadyPublished)
    );
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(27)
    );
    assert_eq!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        task.as_ref()
    );
}

#[v2k_test_support::retail_test]
fn callback_disabled_uses_entity_flags_without_clock_activation_or_sub_d_query() {
    use crate::{common_mover::type9_tail::apply_common_master_motion_raw, world_fx::WorldFx};
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let entity = manager.entity_mut(21).unwrap();
    publish_intro2_type53(
        entity,
        &metadata[53],
        &[],
        session.cache.terrain().unwrap(),
        &mut || 0,
    )
    .unwrap();
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x0202_0000, 0x0200_0000);
    entity.set_velocity_raw([400, 50, -300]);
    let mut expected_position = entity.position_raw();
    let mut expected_velocity = entity.velocity_raw();
    let mut expected_flags = entity.collision.state_flags_at_0x08.known_value_bits();
    apply_common_master_motion_raw(
        &mut expected_position,
        &mut expected_velocity,
        &mut expected_flags,
        20_000,
        0,
    );
    let task = entity.actor_task_state(ActorTaskSlot::Primary).copied();
    let owner = Intro2Type53Owner::adopt(&manager, 21).unwrap();
    let mut world_fx = WorldFx::default();
    let tick = tick_intro2_type53(
        &mut manager,
        owner,
        Intro2Type53Frame {
            resources: &session.cache,
            world_fx: &mut world_fx,
            elapsed_micros: 20_000,
            retail_tick: 10_000,
        },
    );
    assert!(matches!(
        tick.outcome,
        Intro2Type53Outcome::Advanced {
            callback_enabled: false,
            ..
        }
    ));
    assert!(tick.retained_owner.is_some());
    let entity = manager.entity_mut(21).unwrap();
    assert_eq!(entity.position_raw(), expected_position);
    assert_eq!(entity.velocity_raw(), expected_velocity);
    assert_eq!(
        entity.actor_task_state(ActorTaskSlot::Primary).copied(),
        task
    );
    assert_eq!(
        entity.collision.state_flags_at_0x08.masked(0x20000),
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(
        entity
            .intro2_type53_runtime
            .unwrap()
            .sub_d_owner
            .classifier_cache()
            .origin(),
        RetailRuntimeValue::Unresolved
    );
}

#[v2k_test_support::retail_test]
fn first_query_block_retains_retarget_prefix_without_replaying_time_or_rng() {
    use crate::world_fx::WorldFx;
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let entity = manager.entity_mut(21).unwrap();
    publish_intro2_type53(
        entity,
        &metadata[53],
        &[],
        session.cache.terrain().unwrap(),
        &mut || 0,
    )
    .unwrap();
    // Exercise an explicitly unwitnessed owner. The native spawn-20 birth
    // now has its own accepted first-query receipt, removed only in this
    // failure-prefix fixture so it cannot mask unresolved-origin behavior.
    entity.intro2_type53_runtime.as_mut().unwrap().sub_d_owner =
        Type9SubDFrameOwner::pending_constructor_origin(0x13);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x0202_0000, 0x0202_0000);
    let owner = Intro2Type53Owner::adopt(&manager, 21).unwrap();
    let mut world_fx = WorldFx::default();
    let mut expected_rng = WorldFx::default();
    expected_rng.next_shared_retail_random_u16();
    expected_rng.next_shared_retail_random_u16();
    let tick = tick_intro2_type53(
        &mut manager,
        owner,
        Intro2Type53Frame {
            resources: &session.cache,
            world_fx: &mut world_fx,
            elapsed_micros: 20_000,
            retail_tick: 0,
        },
    );
    assert_eq!(
        tick.outcome,
        Intro2Type53Outcome::Blocked {
            entity_id: 21,
            reason: Intro2Type53Block::SubDFirstQuery {
                origin: crate::native_ground_actor::NativeGroundAllocationOrigin::Authored {
                    spawn_index: 20
                },
                seed: 0x13
            },
            prefix_committed: true
        }
    );
    let task = manager
        .entity_mut(21)
        .unwrap()
        .actor_task_state(ActorTaskSlot::Primary)
        .copied();
    assert!(
        matches!(task, Some(ActorTaskRuntime::SharedRetarget(task)) if task.elapsed_ms() == 20)
    );
    let tick = tick_intro2_type53(
        &mut manager,
        tick.retained_owner.unwrap(),
        Intro2Type53Frame {
            resources: &session.cache,
            world_fx: &mut world_fx,
            elapsed_micros: 20_000,
            retail_tick: 1,
        },
    );
    assert_eq!(tick.outcome, Intro2Type53Outcome::Pending { entity_id: 21 });
    assert_eq!(
        manager
            .entity_mut(21)
            .unwrap()
            .actor_task_state(ActorTaskSlot::Primary)
            .copied(),
        task
    );
    assert_eq!(
        world_fx.next_shared_retail_random_u16(),
        expected_rng.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn witnessed_native_acquiring_callbacks_finish_before_the_later_contact_phase() {
    use crate::world_fx::WorldFx;
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for (spawn, detailed) in [
        (20, false),
        (20, true),
        (26, false),
        (26, true),
        (38, false),
        (38, true),
        (41, false),
        (41, true),
    ] {
        let mut manager = generic(&session, &metadata);
        // Keep this two-frame test in acquisition: these controlled beacon
        // scores exercise the selector's no-positive-result branch. A
        // separate test below covers native successful Secondary retirement.
        let beacon_ids: Vec<_> = manager
            .iter_all()
            .filter(|entity| entity.capability_flags & 0x100 != 0)
            .map(|entity| entity.id)
            .collect();
        for id in beacon_ids {
            manager
                .entity_mut(id)
                .unwrap()
                .authored_follow_beacon_priority_raw = Some(0);
        }
        let id = spawn + 1;
        let entity = manager.entity_mut(id).unwrap();
        publish_intro2_type53(
            entity,
            &metadata[53],
            &[],
            session.cache.terrain().unwrap(),
            &mut || 0,
        )
        .unwrap();
        entity.collision.state_flags_at_0x08.overwrite(
            0x0202_0000,
            0x0002_0000 | if detailed { 0x0200_0000 } else { 0 },
        );
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(19);
        let initial_position = entity.position_raw();
        let initial_health = entity.collision.health_raw;
        let mut owner = Intro2Type53Owner::adopt(&manager, id).unwrap();
        let mut fx = WorldFx::new();
        for frame_index in 0..2 {
            // Force the real callback admission without choosing a random
            // endpoint; 12DA0 still supplies its bounded 125-ms duration.
            let entity = manager.entity_mut(id).unwrap();
            entity.collision.callback_scheduler_accumulator_us_at_0x6c =
                RetailRuntimeValue::Known(125_000);
            let tick = tick_intro2_type53(
                &mut manager,
                owner,
                Intro2Type53Frame {
                    resources: &session.cache,
                    world_fx: &mut fx,
                    elapsed_micros: 20_000,
                    retail_tick: 286 + frame_index,
                },
            );
            assert!(
                matches!(
                    tick.outcome,
                    Intro2Type53Outcome::Advanced {
                        callback_enabled: true,
                        ..
                    }
                ),
                "spawn={spawn}, detailed={detailed}, frame={frame_index}: {:?}",
                tick.outcome
            );
            owner = tick
                .retained_owner
                .expect("completed actor remains scheduled");
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(entity.mass_raw, if frame_index == 0 { 119 } else { 100 });
            assert_eq!(
                entity.collision.animation_offset_at_0xb2,
                RetailRuntimeValue::Known(0)
            );
            assert_eq!(
                entity.collision.health_raw, initial_health,
                "contacts run later"
            );
            assert!(matches!(
                entity
                    .intro2_type53_runtime
                    .unwrap()
                    .sub_d_owner
                    .classifier_cache()
                    .origin(),
                RetailRuntimeValue::Known(_)
            ));
        }
        assert_ne!(
            manager.entity_mut(id).unwrap().position_raw(),
            initial_position
        );
    }
}

#[v2k_test_support::retail_test]
fn native_beacon_handoff_retires_its_secondary_without_aborting_the_world_tail() {
    use crate::world_fx::WorldFx;
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let entity = manager.entity_mut(21).unwrap();
    publish_intro2_type53(
        entity,
        &metadata[53],
        &[],
        session.cache.terrain().unwrap(),
        &mut || 0,
    )
    .unwrap();
    let secondary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .unwrap();
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x0202_0000, 0x0202_0000);
    let owner = Intro2Type53Owner::adopt(&manager, 21).unwrap();
    let mut fx = WorldFx::new();
    let result = tick_intro2_type53(
        &mut manager,
        owner,
        Intro2Type53Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 286,
        },
    );
    assert!(
        matches!(
            result.outcome,
            Intro2Type53Outcome::Advanced {
                callback_enabled: true,
                ..
            }
        ),
        "{:?}",
        result.outcome
    );
    assert!(result.retained_owner.is_some());
    let entity = manager.entity_mut(21).unwrap();
    assert!(entity.actor_tasks.wrapper_flags(secondary).is_none());
    assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::FollowBeaconsFollowing(_))
    ));
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0)
    );
}

#[v2k_test_support::retail_test]
fn native_following_continues_after_handoff_and_timeout_reselects_in_the_same_pass() {
    use crate::world_fx::WorldFx;
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    // Isolate the class33 owner transition from unrelated Capture/SearchAttack
    // choices. Authored beacons retain their positions, priority and capability.
    let ids: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
    for id in ids {
        manager.entity_mut(id).unwrap().capability_flags &= !0xc01;
    }
    let entity = manager.entity_mut(21).unwrap();
    publish_intro2_type53(
        entity,
        &metadata[53],
        &[],
        session.cache.terrain().unwrap(),
        &mut || 0,
    )
    .unwrap();
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x0202_0000, 0x0202_0000);
    let initial_position = entity.position_raw();
    let mut owner = Intro2Type53Owner::adopt(&manager, 21).unwrap();
    let mut fx = WorldFx::new();
    let mut following_id = None;
    for frame_index in 0..4 {
        let result = tick_intro2_type53(
            &mut manager,
            owner,
            Intro2Type53Frame {
                resources: &session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                retail_tick: 10_000 + frame_index,
            },
        );
        assert!(
            matches!(
                result.outcome,
                Intro2Type53Outcome::Advanced {
                    callback_enabled: true,
                    ..
                }
            ),
            "frame {frame_index}: {:?}",
            result.outcome
        );
        owner = result.retained_owner.unwrap();
        let entity = manager.entity_mut(21).unwrap();
        let task_id = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        if let Some(previous) = following_id {
            assert_eq!(
                task_id, previous,
                "a live beacon retains its following wrapper"
            );
        } else {
            following_id = Some(task_id);
        }
        let Some(ActorTaskRuntime::FollowBeaconsFollowing(task)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("following task absent");
        };
        assert_eq!(
            task.elapsed_ms(),
            frame_index * 20,
            "a Primary installed by Secondary first runs on the next pass"
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
    }
    let entity = manager.entity_mut(21).unwrap();
    assert_ne!(entity.position_raw(), initial_position);
    let task_id = following_id.unwrap();
    let Some(ActorTaskRuntime::FollowBeaconsFollowing(task)) =
        entity.actor_tasks.task_state_mut(task_id)
    else {
        panic!();
    };
    // Controlled accumulated age reaches, but does not exceed, the strict
    // 9000-ms limit. The real next callback must age, move, unwind, then C690.
    task.before_callback((9_000 - task.elapsed_ms()) * 1_000);
    let result = tick_intro2_type53(
        &mut manager,
        owner,
        Intro2Type53Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 10_004,
        },
    );
    assert!(
        matches!(
            result.outcome,
            Intro2Type53Outcome::Advanced {
                callback_enabled: true,
                ..
            }
        ),
        "{:?}",
        result.outcome
    );
    let entity = manager.entity_mut(21).unwrap();
    assert!(entity.actor_tasks.wrapper_flags(task_id).is_none());
    assert_ne!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        Some(task_id)
    );
    let Some(ActorTaskRuntime::FollowBeaconsFollowing(task)) =
        entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("same-pass new Secondary must publish following");
    };
    assert_eq!(
        task.elapsed_ms(),
        0,
        "the new Primary cannot run twice in one pass"
    );
    assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
}
