use super::tests::{fixture, generic};
use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime, actor_task_owner::ActorTaskSlot,
    common_mover::type9_attitude::Type9BodyBasis, entity::EntityManager,
    entity_collision_state::RetailStateWord, session::GameSession, world_fx::WorldFx,
};

fn admit_callbacks(entity: &mut Entity, detailed: bool) {
    entity.collision.state_flags_at_0x08.overwrite(
        0x0202_0000,
        0x0002_0000 | if detailed { 0x0200_0000 } else { 0 },
    );
    // Detailed callbacks need no admission carry. For coarse mode, seed the
    // real 125-ms carry to force admission through 12DA0's capped branch.
    entity.collision.callback_scheduler_accumulator_us_at_0x6c =
        RetailRuntimeValue::Known(if detailed { 0 } else { 125_000 });
}

fn no_positive_beacons(manager: &mut EntityManager) {
    let ids: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
    for id in ids {
        manager
            .entity_mut(id)
            .unwrap()
            .authored_follow_beacon_priority_raw = Some(0);
    }
}

#[v2k_test_support::retail_test]
fn native_type17_both_live_births_move_in_both_modes_and_rebuild_full_euler_basis() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for spawn in INTRO2_TYPE17_SPAWN_INDICES {
        for detailed in [false, true] {
            let mut manager = generic(&session, &metadata);
            no_positive_beacons(&mut manager);
            let id = spawn as u32 + 1;
            let entity = manager.entity_mut(id).unwrap();
            publish_intro2_type17(
                entity,
                &metadata[17],
                &[],
                session.cache.terrain().unwrap(),
                &mut || 0,
            )
            .unwrap();
            let angles = [0x2bcd, 0x1000, -0x900];
            entity.set_rotation_heading_pitch_roll_raw(angles);
            let incoming_basis = Type9BodyBasis::from_angle_words(angles[0], angles[1], angles[2]);
            entity.physical_body_basis_q31 = RetailRuntimeValue::Known(incoming_basis);
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(19);
            let initial_position = entity.position_raw();
            let initial_health = entity.collision.health_raw;
            let mut owner = Intro2Type17Owner::adopt(&manager, id).unwrap();
            let mut fx = WorldFx::new();
            for frame_index in 0..2 {
                admit_callbacks(manager.entity_mut(id).unwrap(), detailed);
                let tick = tick_intro2_type17(
                    &mut manager,
                    owner,
                    Intro2Type17Frame {
                        capture_tasks: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler::default(),
                        notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                        resources: &session.cache,
                        world_fx: &mut fx,
                        elapsed_micros: 20_000,
                        retail_tick: 286 + frame_index,
                    },
                );
                assert!(
                    matches!(
                        tick.outcome,
                        Intro2Type17Outcome::Advanced {
                            callback_enabled: true,
                            ..
                        }
                    ),
                    "spawn={spawn} detailed={detailed} frame={frame_index}: {:?}",
                    tick.outcome
                );
                owner = tick
                    .retained_owner
                    .expect("completed actor remains scheduled");
                assert!(tick.replacement_common_dying_owner.is_none());
                let entity = manager.entity_mut(id).unwrap();
                assert_eq!(entity.mass_raw, if frame_index == 0 { 119 } else { 100 });
                assert_eq!(
                    entity.collision.animation_offset_at_0xb2,
                    RetailRuntimeValue::Known(0)
                );
                assert_eq!(entity.collision.health_raw, initial_health);
                assert!(matches!(
                    entity
                        .type17_sub_d_frame_owner
                        .unwrap()
                        .classifier_cache()
                        .origin(),
                    RetailRuntimeValue::Known(_)
                ));
                let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
                assert_eq!(entity.physical_body_basis_q31(), RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll)),
                    "the post-task F70 writer rebuilds all three axes from the resulting Euler words");
            }
            let entity = manager.entity_mut(id).unwrap();
            assert_ne!(entity.position_raw(), initial_position);
            assert_ne!(
                entity.physical_body_basis_q31(),
                RetailRuntimeValue::Known(incoming_basis)
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type17_follow_handoff_unwinds_secondary_and_starts_primary_next_pass() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let ids: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
    for id in ids {
        manager.entity_mut(id).unwrap().capability_flags &= !0xc01;
    }
    let entity = manager.entity_mut(5).unwrap();
    publish_intro2_type17(
        entity,
        &metadata[17],
        &[],
        session.cache.terrain().unwrap(),
        &mut || 0,
    )
    .unwrap();
    admit_callbacks(entity, true);
    let old_secondary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .unwrap();
    let initial_position = entity.position_raw();
    let mut owner = Intro2Type17Owner::adopt(&manager, 5).unwrap();
    let mut fx = WorldFx::new();
    let mut following_id = None;
    for frame_index in 0..4 {
        let tick = tick_intro2_type17(
            &mut manager,
            owner,
            Intro2Type17Frame {
                capture_tasks: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler::default(),
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                resources: &session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                retail_tick: 10_000 + frame_index,
            },
        );
        assert!(
            matches!(
                tick.outcome,
                Intro2Type17Outcome::Advanced {
                    callback_enabled: true,
                    ..
                }
            ),
            "{:?}",
            tick.outcome
        );
        owner = tick.retained_owner.unwrap();
        let entity = manager.entity_mut(5).unwrap();
        let task_id = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        if let Some(previous) = following_id {
            assert_eq!(task_id, previous);
        } else {
            following_id = Some(task_id);
        }
        let Some(ActorTaskRuntime::FollowBeaconsFollowing(task)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("following task missing");
        };
        assert_eq!(
            task.elapsed_ms(),
            frame_index * 20,
            "newborn Primary waits for the next actor visit"
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_tasks.wrapper_flags(old_secondary).is_none());
    }
    assert_ne!(
        manager.entity_mut(5).unwrap().position_raw(),
        initial_position
    );
}

fn targeted_birth(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
    class: u8,
) -> (EntityManager, u32) {
    let mut manager = generic(session, metadata);
    let mut prefix = generic(session, metadata);
    let owner = manager.entity_mut(5).unwrap();
    let candidate = prefix.entity_mut(1).unwrap();
    candidate.set_position_raw(owner.position_raw());
    candidate.capability_flags = if class == 9 { 0x800 } else { 1 };
    candidate.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
    let publication = publish_intro2_type17(
        owner,
        &metadata[17],
        std::slice::from_ref(candidate),
        session.cache.terrain().unwrap(),
        &mut || 0,
    )
    .unwrap();
    assert_eq!(publication.selection.program.class_id, class);
    admit_callbacks(owner, true);
    let position = owner.position_raw();
    let target_id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .unwrap()
        .id;
    let ids: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
    for id in ids {
        manager.entity_mut(id).unwrap().capability_flags &= !0xc03;
    }
    let target = manager.entity_mut(target_id).unwrap();
    target.capability_flags |= if class == 9 { 0x800 } else { 1 };
    target.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
    target.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
    target.set_position_raw([
        position[0].wrapping_add(700),
        position[1],
        position[2].wrapping_add(1100),
    ]);
    (manager, target_id)
}

#[v2k_test_support::retail_test]
fn native_type17_capture_and_run_away_publish_real_pursuit_and_advance_retained_tasks() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for class in [9, 10] {
        let (mut manager, target_id) = targeted_birth(&session, &metadata, class);
        let old_primary = manager
            .entity_mut(5)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let mut fx = WorldFx::new();
        let mut expected = WorldFx::new();
        expected.next_shared_retail_random_u16();
        behavior::secondary(&mut manager, 5, 20_000, 321, &mut fx).unwrap();
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16(),
            "each successful handoff has one 06070 suffix word"
        );
        let entity = manager.entity_mut(5).unwrap();
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        assert_ne!(primary, old_primary);
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert_eq!(context.style_table_index_raw_at_0x10(), 1);
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(target_id))
        );
        match entity.actor_task_state(ActorTaskSlot::Primary).unwrap() {
            ActorTaskRuntime::RunAway(task) => {
                assert_eq!(class, 10);
                assert_eq!(task.elapsed_ms(), 0);
            }
            ActorTaskRuntime::CapturePeoplePursuit(task) => {
                assert_eq!(class, 9);
                assert_eq!(task.elapsed_ms(), 0);
            }
            other => panic!("wrong pursuit: {other:?}"),
        }
        let mut owner = Intro2Type17Owner::adopt(&manager, 5).unwrap();
        for frame_index in 1..=2 {
            let tick = tick_intro2_type17(
                &mut manager,
                owner,
                Intro2Type17Frame {
                    capture_tasks: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler::default(),
                    notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                    resources: &session.cache,
                    world_fx: &mut fx,
                    elapsed_micros: 20_000,
                    retail_tick: 321 + frame_index,
                },
            );
            assert!(
                matches!(
                    tick.outcome,
                    Intro2Type17Outcome::Advanced {
                        callback_enabled: true,
                        ..
                    }
                ),
                "class={class}: {:?}",
                tick.outcome
            );
            owner = tick.retained_owner.unwrap();
            let entity = manager.entity_mut(5).unwrap();
            assert_eq!(
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                Some(primary)
            );
            match entity.actor_task_state(ActorTaskSlot::Primary).unwrap() {
                ActorTaskRuntime::RunAway(task) => {
                    assert_eq!(task.elapsed_ms(), frame_index * 20);
                    assert_eq!(task.target_id(), target_id);
                }
                ActorTaskRuntime::CapturePeoplePursuit(task) => {
                    assert_eq!(task.elapsed_ms(), frame_index * 20)
                }
                _ => panic!("pursuit must survive two nearby visits"),
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type17_incomplete_mover_retains_pending_cursor_without_replaying_or_hiding_replacement() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    no_positive_beacons(&mut manager);
    let entity = manager.entity_mut(5).unwrap();
    publish_intro2_type17(
        entity,
        &metadata[17],
        &[],
        session.cache.terrain().unwrap(),
        &mut || 0,
    )
    .unwrap();
    admit_callbacks(entity, true);
    entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    let owner = Intro2Type17Owner::adopt(&manager, 5).unwrap();
    let mut fx = WorldFx::new();
    let tick = tick_intro2_type17(
        &mut manager,
        owner,
        Intro2Type17Frame {
            capture_tasks: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler::default(),
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 286,
        },
    );
    assert!(
        matches!(
            tick.outcome,
            Intro2Type17Outcome::Blocked {
                prefix_committed: true,
                ..
            }
        ),
        "{:?}",
        tick.outcome
    );
    let pending = tick.retained_owner.unwrap();
    let before_tasks = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
        manager
            .entity_mut(5)
            .unwrap()
            .actor_task_state(slot)
            .cloned()
    });
    let before_state = manager.entity_mut(5).unwrap().collision.clone();
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let tick = tick_intro2_type17(
        &mut manager,
        pending,
        Intro2Type17Frame {
            capture_tasks: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler::default(),
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 999_000,
            retail_tick: 1000,
        },
    );
    assert!(matches!(tick.outcome, Intro2Type17Outcome::Pending { .. }));
    assert_eq!(
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| manager
            .entity_mut(5)
            .unwrap()
            .actor_task_state(slot)
            .cloned()),
        before_tasks
    );
    assert_eq!(manager.entity_mut(5).unwrap().collision, before_state);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
    manager
        .entity_mut(5)
        .unwrap()
        .actor_tasks
        .clear_slot_with_retirement(ActorTaskSlot::Secondary, |_| {});
    let tick = tick_intro2_type17(
        &mut manager,
        tick.retained_owner.unwrap(),
        Intro2Type17Frame {
            capture_tasks: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler::default(),
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 1001,
        },
    );
    assert!(matches!(tick.outcome, Intro2Type17Outcome::Dropped { .. }));
    assert!(tick.retained_owner.is_none());

    let entity = manager.entity_mut(5).unwrap();
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!()
    };
    entity.publish_behavior_initializer_failure_fallback(context);
    assert!(Intro2Type17Owner::adopt(&manager, 5).is_err());
    let fallback = Intro2Type17Owner::adopt_blocked_prefix(&manager, 5).unwrap();
    assert!(fallback.has_pending_prefix());
    let tick = tick_intro2_type17(
        &mut manager,
        fallback,
        Intro2Type17Frame {
            capture_tasks: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler::default(),
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 1002,
        },
    );
    assert!(
        matches!(tick.outcome, Intro2Type17Outcome::Pending { .. }),
        "the actual C6B0 fallback is retained without resuming an empty living graph"
    );
    assert!(tick.retained_owner.is_some());
}

#[v2k_test_support::retail_test]
fn native_type17_reselection_consumes_live_owner_relation_and_blocks_unknown_before_rng() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for unresolved in [false, true] {
        let (mut manager, target_id) = targeted_birth(&session, &metadata, 10);
        let entity = manager.entity_mut(5).unwrap();
        entity.collision.recent_relation_id_at_0x60 = if unresolved {
            RetailRuntimeValue::Unresolved
        } else {
            RetailRuntimeValue::Known(Some(target_id))
        };
        let before_context = entity.current_behavior_context;
        let before_primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
        let mut fx = WorldFx::new();
        let mut expected = WorldFx::new();
        let result = behavior::reselect(
            &mut manager,
            5,
            100,
            &mut fx,
            behavior::ReselectionEntry::Impact,
        );
        let entity = manager.entity_mut(5).unwrap();
        if unresolved {
            assert!(
                matches!(result, Err(Intro2Type17Block::Selection(_))),
                "{result:?}"
            );
            assert_eq!(entity.current_behavior_context, before_context);
            assert_eq!(
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                before_primary
            );
        } else {
            result.unwrap();
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                panic!()
            };
            assert_eq!(context.descriptor(), crate::entity_behavior::BehaviorDescriptorIdentity::Named(crate::entity_behavior::behavior_program(33).unwrap()),
                "the sole nearby player is the owner's excluded +60 relation, leaving only Always/Follow");
            for _ in 0..3 {
                expected.next_shared_retail_random_u16();
            }
        }
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn shared_type17_native_lease_rejects_foreign_live_hit_and_class12_entries() {
    use crate::{
        damage::PRIMARY_PROJECTILE_DAMAGE_PACKET,
        entity::EntityConstructionResources,
        intro2_common_dying::{
            publish_intro2_common_standard_death, tick_intro2_common_dying, Intro2CommonDyingBlock,
            Intro2CommonDyingFrame, Intro2CommonDyingOutcome, Intro2CommonDyingOwner,
        },
        specialized_actor_task_production::SpecializedActorTaskScheduler,
        world_fx::{BallisticDamageRequest, ParticleEntityImpact},
    };

    let Some((session, metadata)) = fixture() else {
        return;
    };
    let construct = || {
        EntityManager::from_native_intro2_frontend(
            session.cache.level_desc().unwrap(),
            &metadata,
            EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            0,
            &mut WorldFx::new(),
        )
        .unwrap()
    };
    let mut source = construct();
    let mut destination = construct();
    let id = 5;
    let owner = Intro2Type17Owner::adopt(&source, id).unwrap();
    let transaction = source.fork_for_main_base_abort_transaction();
    assert!(type17_manager_allocation_authenticates(&transaction, id));
    assert_eq!(Intro2Type17Owner::adopt(&transaction, id), Ok(owner));

    // Both bodies were constructed normally. Transplant only the native
    // receipt, proving that matching type/spawn/model words are insufficient.
    destination.entity_mut(id).unwrap().intro2_type17_runtime =
        source.entity_mut(id).unwrap().intro2_type17_runtime;
    assert!(intro2_type17_allocation_authenticates(
        destination.entity_mut(id).unwrap()
    ));
    assert!(!type17_manager_allocation_authenticates(&destination, id));
    assert_eq!(
        Intro2Type17Owner::adopt(&destination, id),
        Err(Intro2Type17Block::Allocation)
    );
    let before = destination.entity_mut(id).unwrap().collision.clone();
    let before_context = destination.entity_mut(id).unwrap().current_behavior_context;
    let mut fx = WorldFx::new();
    let mut expected = WorldFx::new();
    let tick = tick_intro2_type17(
        &mut destination,
        owner,
        Intro2Type17Frame {
            capture_tasks: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler::default(),
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 77,
        },
    );
    assert_eq!(
        tick.outcome,
        Intro2Type17Outcome::Blocked {
            entity_id: id,
            reason: Intro2Type17Block::Allocation,
            prefix_committed: false,
        }
    );
    assert!(tick.retained_owner.is_none());
    for source_particle_class in [5, 16] {
        let hit = super::impact::apply_intro2_type17_particle_hit(
            crate::shared_actor_impact::SharedActorImpactFrame {
                entities: &mut destination,
                resources: &session.cache,
                world_fx: &mut fx,
                scheduler: &mut SpecializedActorTaskScheduler::new(),
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                retail_tick: 77,
            },
            ParticleEntityImpact {
                source_particle_class,
                impact_position_argument_va: if source_particle_class == 5 {
                    0x004d_cf48
                } else {
                    0
                },
                target_entity_id: id,
                position_world: [0.0; 3],
                velocity_raw: [0, 0, 8192],
                damage: Some(BallisticDamageRequest {
                    packet: if source_particle_class == 5 {
                        crate::damage::FUN_0043F780_DAMAGE_PACKET
                    } else {
                        PRIMARY_PROJECTILE_DAMAGE_PACKET
                    },
                    source_entity_type_at_birth: Some(51),
                    source_owner_id: Some(0),
                }),
            },
        );
        assert_eq!(
            hit,
            super::impact::Intro2Type17ImpactOutcome::Blocked {
                reason: super::impact::Intro2Type17ImpactBlock::Runtime("native allocation"),
                committed_prefix: false,
            }
        );
    }
    assert_eq!(
        publish_intro2_common_standard_death(&mut destination, id, &mut fx),
        Err(Intro2CommonDyingBlock::UnauthenticatedAllocation),
    );
    let dying = publish_intro2_common_standard_death(&mut source, id, &mut WorldFx::new())
        .unwrap()
        .unwrap();
    assert_eq!(
        Intro2CommonDyingOwner::adopt(&destination, id),
        Err(Intro2CommonDyingBlock::UnauthenticatedAllocation),
    );
    let tick = tick_intro2_common_dying(
        &mut destination,
        dying,
        Intro2CommonDyingFrame {
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 77,
        },
    );
    assert_eq!(
        tick.outcome,
        Intro2CommonDyingOutcome::Blocked {
            entity_id: id,
            reason: Intro2CommonDyingBlock::UnauthenticatedAllocation,
            prefix_committed: false,
        }
    );
    assert!(tick.retained_owner.is_none());
    assert_eq!(destination.entity_mut(id).unwrap().collision, before);
    assert_eq!(
        destination.entity_mut(id).unwrap().current_behavior_context,
        before_context
    );
    assert_eq!(fx.pending_event_count(), 0);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn externally_entered_run_away2_stays_explicit_fail_closed() {
    use crate::entity_behavior::{
        audited_behavior_style, behavior_program, BehaviorContextRuntime,
    };

    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = INTRO2_TYPE17_SPAWN_INDICES[0] as u32 + 1;
    let entity = manager.entity_mut(id).unwrap();
    publish_intro2_type17(
        entity,
        &metadata[17],
        &[],
        session.cache.terrain().unwrap(),
        &mut || 0,
    )
    .unwrap();
    let previous = match entity.current_behavior_context {
        RetailRuntimeValue::Known(Some(context)) => context,
        _ => panic!("native Type17 publishes a current graph"),
    };
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(
        BehaviorContextRuntime::named_audited(
            behavior_program(10).expect("Run Away"),
            2,
            previous.choice_list_source(),
            previous.target_handle_at_0x08(),
            previous.auxiliary_word_at_0x0c(),
            *audited_behavior_style(10, 2).expect("Run Away2"),
        )
        .expect("class-10 variant 2"),
    ));
    assert_eq!(
        Intro2Type17Owner::adopt(&manager, id),
        Err(Intro2Type17Block::UnsupportedRunAway2 {
            entity_id: id,
            entity_type: 17,
            spawn_index: Some(INTRO2_TYPE17_SPAWN_INDICES[0]),
        })
    );
}
