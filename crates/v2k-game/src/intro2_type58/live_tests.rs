use super::native::tests::{fixture, generic};
use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::{EntityConstructionResources, EntityManager, Intro2BirthSelection},
    entity_collision_state::RetailStateWord,
    intro2_common_dying::{
        tick_intro2_common_dying, Intro2CommonDyingFrame, Intro2CommonDyingOutcome,
    },
    world_fx::WorldFx,
};

fn admit_callbacks(entity: &mut Entity, detailed: bool) {
    entity.collision.state_flags_at_0x08.overwrite(
        0x0202_0000,
        0x0002_0000 | if detailed { 0x0200_0000 } else { 0 },
    );
    entity.collision.callback_scheduler_accumulator_us_at_0x6c =
        RetailRuntimeValue::Known(if detailed { 0 } else { 125_000 });
}

#[v2k_test_support::retail_test]
fn native_type58_varied_frames_keep_native_constructor_rng_and_living_or_surface_death_custody() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    // Each bounded Type58 run starts at a different real shared-stream entry,
    // constructs the entire authored prefix in order, then continues that RNG.
    // Other actors are stationary in this unit fixture; full-scene integration
    // also runs their owners and all presentation/audio phases.
    for offset in [0, 17, 53] {
        for detailed in [false, true] {
            let mut fx = WorldFx::new();
            for _ in 0..offset {
                fx.next_shared_retail_random_u16();
            }
            let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
                session.cache.level_desc().unwrap(),
                &metadata,
                EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| {
                        session.cache.global_model(id).map(|model| model.radius)
                    }),
                },
                Intro2BirthSelection::default(),
                &mut || u32::from(fx.next_shared_retail_random_u16()),
            )
            .unwrap();
            let id = manager
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(40))
                .unwrap()
                .id;
            let entity = manager.entity_mut(id).unwrap();
            admit_callbacks(entity, detailed);
            let initial_position = entity.position_raw();
            let native_seed = entity
                .intro2_type58_runtime
                .unwrap()
                .sub_d_owner
                .classifier_cache()
                .stagger_counter();
            assert_eq!(native_seed, 0x19);
            let mut living = Some(Intro2Type58Owner::adopt(&manager, id).unwrap());
            let mut dying = None;
            let mut callbacks = 0;
            let mut total = 0;
            for index in 0..240 {
                let dt = [16_667, 33_333, 20_000, 50_000, 1_000, 125_000][index % 6];
                fx.advance_frame_pacing(dt);
                if let Some(owner) = living.take() {
                    let tick = tick_intro2_type58(
                        &mut manager,
                        owner,
                        Intro2Type58Frame {
                            resources: &mut session.cache,
                            world_fx: &mut fx,
                            elapsed_micros: dt,
                            retail_tick: 286 + total / 20_000,
                        },
                    );
                    match &tick.outcome {
                        Intro2Type58Outcome::Waiting { .. } => {}
                        Intro2Type58Outcome::Advanced {
                            callback_enabled: true,
                            ..
                        } => callbacks += 1,
                        other => {
                            panic!("entry{offset} detailed{detailed} elapsed{total}: {other:?}")
                        }
                    }
                    if let Some(owner) = tick.replacement_common_dying_owner {
                        assert!(tick.retained_owner.is_none());
                        assert_eq!(
                            manager.entity_mut(id).unwrap().collision.health_raw,
                            RetailRuntimeValue::Known(0)
                        );
                        dying = Some(owner);
                    } else {
                        let owner = tick
                            .retained_owner
                            .expect("living custody cannot silently disappear");
                        assert!(!owner.has_pending_prefix());
                        living = Some(owner);
                    }
                } else if let Some(owner) = dying.take() {
                    let tick = tick_intro2_common_dying(
                        &mut manager,
                        owner,
                        Intro2CommonDyingFrame {
                            resources: &session.cache,
                            world_fx: &mut fx,
                            elapsed_micros: dt,
                            retail_tick: 286 + total / 20_000,
                        },
                    );
                    assert!(
                        matches!(
                            tick.outcome,
                            Intro2CommonDyingOutcome::Waiting { .. }
                                | Intro2CommonDyingOutcome::Advanced { .. }
                        ),
                        "{:?}",
                        tick.outcome
                    );
                    dying = tick.retained_owner;
                    if dying.is_none() {
                        assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
                        break;
                    }
                } else {
                    panic!("lost both native owners");
                }
                fx.process_pending();
                total += dt;
            }
            assert!(callbacks > 0);
            assert_ne!(
                manager.entity_mut(id).unwrap().position_raw(),
                initial_position
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type58_search_handoff_starts_chase_next_pass_and_aim_in_current_pass() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let mut earlier = generic(&session, &metadata);
    let id = 41;
    let entity = manager.entity_mut(id).unwrap();
    let nearby = earlier.entity_mut(1).unwrap();
    nearby.set_position_raw(entity.position_raw());
    nearby.capability_flags = 1;
    nearby.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
    let mut words = [0x1700, 0xffff, 0x1234, 0x9876].into_iter();
    let birth = publish_intro2_type58(
        entity,
        &metadata[58],
        std::slice::from_ref(nearby),
        session.cache.terrain().unwrap(),
        session.cache.terrain_objects().unwrap(),
        &mut || words.next().expect("A, selector, two task suffixes"),
    )
    .unwrap();
    assert_eq!(birth.selection.program.class_id, 7);
    admit_callbacks(entity, true);
    let position = entity.position_raw();
    let old_primary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let old_secondary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .unwrap();
    let target_id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .unwrap()
        .id;
    let ids: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
    for candidate_id in ids {
        let candidate = manager.entity_mut(candidate_id).unwrap();
        candidate.capability_flags &= !0xc03;
        if candidate_id != id {
            candidate.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
        }
    }
    let target = manager.entity_mut(target_id).unwrap();
    target.capability_flags |= 1;
    target.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
    target.set_position_raw([
        position[0].wrapping_add(700),
        position[1],
        position[2].wrapping_add(1100),
    ]);
    let mut owner = Intro2Type58Owner::adopt(&manager, id).unwrap();
    let mut fx = WorldFx::new();
    let mut primary = None;
    for index in 0..3 {
        let tick = tick_intro2_type58(
            &mut manager,
            owner,
            Intro2Type58Frame {
                resources: &mut session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                retail_tick: 321 + index,
            },
        );
        assert!(
            matches!(
                tick.outcome,
                Intro2Type58Outcome::Advanced {
                    callback_enabled: true,
                    ..
                }
            ),
            "{:?}",
            tick.outcome
        );
        assert!(tick.replacement_common_dying_owner.is_none());
        owner = tick.retained_owner.unwrap();
        let entity = manager.entity_mut(id).unwrap();
        let current = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        assert_ne!(current, old_primary);
        if let Some(previous) = primary {
            assert_eq!(current, previous);
        } else {
            primary = Some(current);
        }
        assert!(entity.actor_tasks.wrapper_flags(old_primary).is_none());
        assert!(entity.actor_tasks.wrapper_flags(old_secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        let Some(ActorTaskRuntime::ChaseTarget(chase)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("native Chase")
        };
        assert_eq!(chase.elapsed_ms(), index * 20);
        let Some(ActorTaskRuntime::AimAndFire(aim)) =
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        else {
            panic!("native Aim")
        };
        assert_eq!(aim.elapsed_ms(), (index + 1) * 20);
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(target_id))
        );
    }
}
