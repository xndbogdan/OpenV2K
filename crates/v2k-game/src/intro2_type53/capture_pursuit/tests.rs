use super::super::tests::{fixture, generic};
use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    entity_collision_state::DYING_STATE_BIT,
    intro2_capture_pursuit::{publish_handoff, warn_target},
    shared_target_route::{SharedTargetRouteTaggedSingleton, SharedTargetRouteTransitionReason},
};
const PURSUIT_STYLE_ADDRESS: u32 = 0x004C_8038;
use crate::{
    common_mover::component_dispatch::CommonMoverDispatchMode,
    entity_collision_state::RetailStateWord, session::GameSession,
};

fn capture_fixture(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
    spawn: usize,
) -> (EntityManager, u32, u32) {
    let mut manager = generic(session, metadata);
    let id = spawn as u32 + 1;
    let mut preceding = generic(session, metadata);
    let witness = preceding.entity_mut(1).unwrap();
    let entity = manager.entity_mut(id).unwrap();
    witness.set_position_raw(entity.position_raw());
    witness.capability_flags = 0xc00;
    witness.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
    let mut words = [0, 0xffff, 0, 0].into_iter();
    let publication = publish_intro2_type53(
        entity,
        &metadata[53],
        std::slice::from_ref(witness),
        session.cache.terrain().unwrap(),
        &mut || words.next().unwrap(),
    )
    .unwrap();
    assert_eq!(publication.selection.program.class_id, 9);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x0202_0000, 0x0202_0000);
    let owner_position = entity.position_raw();
    let target_id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .unwrap()
        .id;
    // Retain the real corpus actors and geometry, but isolate one target so
    // ordering/timeout assertions do not depend on another actor's AI visit.
    let ids: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
    for candidate in ids {
        manager.entity_mut(candidate).unwrap().capability_flags &= !0xc01;
    }
    let target = manager.entity_mut(target_id).unwrap();
    target.capability_flags |= 0x800;
    target.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
    target.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
    target.set_position_raw([
        owner_position[0].wrapping_add(700),
        owner_position[1],
        owner_position[2].wrapping_add(1100),
    ]);
    (manager, id, target_id)
}

fn frame<'a>(
    session: &'a GameSession,
    metadata: &'a [EntityTypeRuntimeMetadata],
) -> super::super::mover::MoverFrame<'a> {
    super::super::mover::MoverFrame {
        metadata: &metadata[53],
        terrain: session.cache.terrain().unwrap(),
        dispatch_mode: CommonMoverDispatchMode::Normal,
        elapsed_micros: 20_000,
        global_elapsed_micros: 20_000,
    }
}

#[v2k_test_support::retail_test]
fn four_native_capture_handoffs_warn_once_and_publish_fresh_pursuit() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for spawn in INTRO2_TYPE53_SPAWN_INDICES {
        let (mut manager, id, target_id) = capture_fixture(&session, &metadata, spawn);
        let entity = manager.entity_mut(id).unwrap();
        let old_primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let old_secondary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Secondary)
            .unwrap();
        let RetailRuntimeValue::Known(Some(h)) = &mut entity.sub_h_external_frame_runtime else {
            panic!()
        };
        h.set_enabled(false);
        let target_health = manager.entity_mut(target_id).unwrap().collision.health_raw;
        let mut fx = WorldFx::new();
        let mut expected = WorldFx::new();
        expected.next_shared_retail_random_u16();
        acquire(&mut manager, id, 321, &mut fx).unwrap();
        let target = manager.entity_mut(target_id).unwrap();
        assert_eq!(
            target.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(321)
        );
        assert_eq!(
            target.collision.health_raw, target_health,
            "416360 does not damage the target"
        );
        let warning_position = target.position_raw().map(|raw| f32::from(raw) / 256.0);
        assert_eq!(
            fx.pending_event_count(),
            usize::from(matches!(
                metadata[9].target_warning_sound_id,
                RetailRuntimeValue::Known(Some(_))
            ))
        );
        fx.process_pending();
        let sounds = fx.take_positional_sounds();
        if let RetailRuntimeValue::Known(Some(sound)) = metadata[9].target_warning_sound_id {
            assert_eq!(sounds.len(), 1);
            assert_eq!(sounds[0].sound_id, usize::from(sound));
            assert_eq!(sounds[0].position, warning_position);
            assert_eq!(sounds[0].frequency_q16, 0x10000);
        } else {
            assert!(sounds.is_empty());
        }
        let entity = manager.entity_mut(id).unwrap();
        assert!(entity.actor_tasks.wrapper_flags(old_primary).is_none());
        assert!(entity.actor_tasks.wrapper_flags(old_secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        let Some(ActorTaskRuntime::CapturePeoplePursuit(task)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!()
        };
        assert_eq!(task.elapsed_ms(), 0);
        assert_eq!(task.target_id(), Some(target_id));
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert_eq!(
            context.active_style().style_address(),
            PURSUIT_STYLE_ADDRESS
        );
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(target_id))
        );
        let RetailRuntimeValue::Known(Some(a)) = entity.sub_a_propulsion_runtime else {
            panic!()
        };
        assert_eq!(a.target_speed_raw(), RetailRuntimeValue::Known(533));
        assert!(
            matches!(&entity.sub_h_external_frame_runtime, RetailRuntimeValue::Known(Some(h)) if h.is_enabled())
        );
        assert!(
            matches!(&entity.sub_j_attachment_runtime, RetailRuntimeValue::Known(Some(j)) if j.is_empty())
        );
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16(),
            "only the successful 06070 suffix draws a word"
        );
    }
}

#[v2k_test_support::retail_test]
fn pursuit_runs_next_frame_and_timeout_reselects_then_visits_new_secondary() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for spawn in INTRO2_TYPE53_SPAWN_INDICES {
        let (mut manager, id, target_id) = capture_fixture(&session, &metadata, spawn);
        let mut owner = Intro2Type53Owner::adopt(&manager, id).unwrap();
        let mut fx = WorldFx::new();
        let mut pursuit_id = None;
        for pass in 0..3 {
            let tick = tick_intro2_type53(
                &mut manager,
                owner,
                Intro2Type53Frame {
                    resources: &session.cache,
                    world_fx: &mut fx,
                    elapsed_micros: 20_000,
                    retail_tick: 300 + pass,
                },
            );
            assert!(
                matches!(tick.outcome, Intro2Type53Outcome::Advanced { .. }),
                "spawn={spawn}, pass={pass}: {:?}",
                tick.outcome
            );
            owner = tick.retained_owner.unwrap();
            let entity = manager.entity_mut(id).unwrap();
            let current_id = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            if let Some(previous) = pursuit_id {
                assert_eq!(current_id, previous);
            } else {
                pursuit_id = Some(current_id);
            }
            let Some(ActorTaskRuntime::CapturePeoplePursuit(task)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!()
            };
            assert_eq!(
                task.elapsed_ms(),
                pass * 20,
                "Secondary cannot rerun the new Primary"
            );
            assert_eq!(
                entity.collision.animation_offset_at_0xb2,
                RetailRuntimeValue::Known(0)
            );
            // Keep a moving live target within the route cube.
            let position = entity.position_raw();
            manager.entity_mut(target_id).unwrap().set_position_raw([
                position[0].wrapping_add(700),
                position[1],
                position[2].wrapping_add(1100),
            ]);
        }
        let entity = manager.entity_mut(id).unwrap();
        let old_task_id = pursuit_id.unwrap();
        let Some(ActorTaskRuntime::CapturePeoplePursuit(task)) =
            entity.actor_tasks.task_state_mut(old_task_id)
        else {
            panic!()
        };
        task.before_callback((4980 - task.elapsed_ms()) * 1000);
        let boundary = tick_intro2_type53(
            &mut manager,
            owner,
            Intro2Type53Frame {
                resources: &session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                retail_tick: 304,
            },
        );
        assert!(
            matches!(boundary.outcome, Intro2Type53Outcome::Advanced { .. }),
            "{:?}",
            boundary.outcome
        );
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            Some(old_task_id)
        );
        let Some(ActorTaskRuntime::CapturePeoplePursuit(task)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!()
        };
        assert_eq!(task.elapsed_ms(), 5000, "equality must not reselect");
        // Remove people from the weighted root, so timeout selects the actual
        // authored Always Follow choice and its Secondary runs in this pass.
        manager.entity_mut(target_id).unwrap().capability_flags &= !0xc00;
        let result = tick_intro2_type53(
            &mut manager,
            boundary.retained_owner.unwrap(),
            Intro2Type53Frame {
                resources: &session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                retail_tick: 305,
            },
        );
        assert!(
            matches!(result.outcome, Intro2Type53Outcome::Advanced { .. }),
            "{:?}",
            result.outcome
        );
        let entity = manager.entity_mut(id).unwrap();
        assert!(entity.actor_tasks.wrapper_flags(old_task_id).is_none());
        assert_ne!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            Some(old_task_id)
        );
        match entity.actor_task_state(ActorTaskSlot::Primary) {
            Some(ActorTaskRuntime::FollowBeaconsFollowing(task)) => {
                assert_eq!(task.elapsed_ms(), 0)
            }
            Some(ActorTaskRuntime::SharedRetarget(task)) => assert_eq!(task.elapsed_ms(), 0),
            other => panic!("new authored Follow graph: {other:?}"),
        }
        assert!(result.retained_owner.is_some());
    }
}

#[v2k_test_support::retail_test]
fn invalid_targets_skip_mover_but_route_zero_commits_mover_before_tag() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for invalid in ["missing", "inactive", "dying", "unknown", "route"] {
        let (mut manager, id, target_id) = capture_fixture(&session, &metadata, 20);
        let mut fx = WorldFx::new();
        acquire(&mut manager, id, 321, &mut fx).unwrap();
        let entity = manager.entity_mut(id).unwrap();
        let origin_before = entity
            .intro2_type53_runtime
            .unwrap()
            .sub_d_owner
            .classifier_cache()
            .origin();
        assert_eq!(origin_before, RetailRuntimeValue::Unresolved);
        let target = manager.entity_mut(target_id).unwrap();
        match invalid {
            "missing" => target.active = false,
            "inactive" => target.collision.state_flags_at_0x08 = RetailStateWord::exact(0),
            "dying" => target
                .collision
                .state_flags_at_0x08
                .overwrite(DYING_STATE_BIT, DYING_STATE_BIT),
            "unknown" => {
                target.collision.state_flags_at_0x08 = RetailStateWord::from_known_bits(4, 4)
            }
            "route" => target.set_position_raw([25000, 0, 25000]),
            _ => unreachable!(),
        }
        let result = tick_primary(&mut manager, id, frame(&session, &metadata), &mut fx);
        if invalid == "unknown" {
            assert_eq!(
                result,
                Err(Intro2Type53Block::Runtime("Capture target state"))
            );
        } else {
            assert_eq!(
                result.unwrap().unwrap().reason,
                SharedTargetRouteTransitionReason::TaggedCallbackResult(if invalid == "route" {
                    SharedTargetRouteTaggedSingleton::ZeroPredicate
                } else {
                    SharedTargetRouteTaggedSingleton::InvalidTarget
                })
            );
        }
        let entity = manager.entity_mut(id).unwrap();
        let Some(ActorTaskRuntime::CapturePeoplePursuit(task)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!()
        };
        assert_eq!(task.elapsed_ms(), 20);
        assert_eq!(
            entity
                .actor_tasks
                .wrapper_flags(
                    entity
                        .actor_tasks
                        .task_in_slot(ActorTaskSlot::Primary)
                        .unwrap()
                )
                .unwrap()
                .in_callback,
            false
        );
        let origin_after = entity
            .intro2_type53_runtime
            .unwrap()
            .sub_d_owner
            .classifier_cache()
            .origin();
        if invalid == "route" {
            assert!(matches!(origin_after, RetailRuntimeValue::Known(_)));
        } else {
            assert_eq!(origin_after, origin_before);
        }
    }
}

#[v2k_test_support::retail_test]
fn failed_af50_preparation_keeps_warning_and_clears_without_constructor_rng() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let (mut manager, id, target_id) = capture_fixture(&session, &metadata, 20);
    let mut fx = WorldFx::new();
    let mut expected = WorldFx::new();
    let before_speed = manager.entity_mut(id).unwrap().sub_a_propulsion_runtime;
    let result = publish_handoff(
        &mut manager,
        id,
        target_id,
        321,
        &mut fx,
        Intro2CaptureProfile::Type53,
        |_, _, _| Err(Intro2CaptureBlock::Runtime("allocation denied")),
    );
    assert_eq!(
        result,
        Err(Intro2CaptureBlock::Runtime("allocation denied"))
    );
    let entity = manager.entity_mut(id).unwrap();
    for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        assert!(entity.actor_task_state(slot).is_none());
    }
    assert_eq!(entity.sub_a_propulsion_runtime, before_speed);
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!()
    };
    assert_ne!(
        context.active_style().style_address(),
        PURSUIT_STYLE_ADDRESS,
        "C6B0 installs its initializer-failure fallback"
    );
    assert_eq!(
        manager
            .entity_mut(target_id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(321)
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn target_warning_dying_gate_follows_stamp_and_skips_unknown_sound() {
    let Some((session, mut metadata)) = fixture() else {
        return;
    };
    metadata[9].target_warning_sound_id = RetailRuntimeValue::Unresolved;
    let (mut manager, _, target_id) = capture_fixture(&session, &metadata, 20);
    let mut fx = WorldFx::new();
    manager
        .entity_mut(target_id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    warn_target(&mut manager, target_id, u32::MAX, &mut fx).unwrap();
    assert_eq!(
        manager
            .entity_mut(target_id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(u32::MAX)
    );
    assert_eq!(fx.pending_event_count(), 0);
    manager
        .entity_mut(target_id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, 0);
    assert_eq!(
        warn_target(&mut manager, target_id, 0, &mut fx),
        Err(Intro2CaptureBlock::Runtime("Capture target warning sound"))
    );
    assert_eq!(
        manager
            .entity_mut(target_id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(0)
    );
}

#[v2k_test_support::retail_test]
fn unresolved_warning_parks_the_committed_context_without_replaying_handoff() {
    let Some((session, mut metadata)) = fixture() else {
        return;
    };
    metadata[9].target_warning_sound_id = RetailRuntimeValue::Unresolved;
    let (mut manager, id, target_id) = capture_fixture(&session, &metadata, 20);
    let owner = Intro2Type53Owner::adopt(&manager, id).unwrap();
    let mut fx = WorldFx::new();
    let blocked = tick_intro2_type53(
        &mut manager,
        owner,
        Intro2Type53Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 321,
        },
    );
    assert_eq!(
        blocked.outcome,
        Intro2Type53Outcome::Blocked {
            entity_id: id,
            reason: Intro2Type53Block::Runtime("Capture target warning sound"),
            prefix_committed: true,
        }
    );
    let entity = manager.entity_mut(id).unwrap();
    let context = entity.current_behavior_context;
    assert!(
        matches!(context, RetailRuntimeValue::Known(Some(context)) if context.active_style().style_address() == PURSUIT_STYLE_ADDRESS)
    );
    let primary = entity.actor_task_state(ActorTaskSlot::Primary).copied();
    let secondary = entity.actor_task_state(ActorTaskSlot::Secondary).copied();
    let before = (
        entity.position_raw(),
        entity.velocity_raw(),
        entity.collision.callback_scheduler_accumulator_us_at_0x6c,
    );
    let pending = tick_intro2_type53(
        &mut manager,
        blocked.retained_owner.unwrap(),
        Intro2Type53Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 125_000,
            retail_tick: 322,
        },
    );
    assert_eq!(
        pending.outcome,
        Intro2Type53Outcome::Pending { entity_id: id }
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        (
            entity.position_raw(),
            entity.velocity_raw(),
            entity.collision.callback_scheduler_accumulator_us_at_0x6c
        ),
        before
    );
    assert_eq!(entity.current_behavior_context, context);
    assert_eq!(
        entity.actor_task_state(ActorTaskSlot::Primary).copied(),
        primary
    );
    assert_eq!(
        entity.actor_task_state(ActorTaskSlot::Secondary).copied(),
        secondary
    );
    assert_eq!(
        manager
            .entity_mut(target_id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(321)
    );
    assert_eq!(fx.pending_event_count(), 0);
}
