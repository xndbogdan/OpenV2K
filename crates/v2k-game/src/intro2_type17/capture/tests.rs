use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::{AuthoredWorldConstruction, EntityConstructionResources},
    entity_collision_state::EntityTypeRuntimeMetadata,
    main_base_type9_abort::MainBaseType9ResultScreenState,
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
};

pub(crate) struct Fixture {
    pub(crate) session: GameSession,
    pub(crate) manager: EntityManager,
    pub(crate) scheduler: SpecializedActorTaskScheduler,
    pub(crate) fx: WorldFx,
    pub(crate) notifications: GameplayNotifications,
    pub(crate) parent: u32,
    pub(crate) child: u32,
}

pub(crate) fn fixture() -> Option<Fixture> {
    let data = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(14, 1).unwrap();
    let metadata = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(kind, _)| {
            EntityTypeRuntimeMetadata::from_section12(
                session.cache.global_entity_type(kind).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let mut fx = WorldFx::new();
    let mut manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            logical_world_index: 2,
            level: session.cache.level_desc().unwrap(),
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: None,
            retail_tick: 300,
        },
        &mut fx,
    )
    .unwrap();
    let parent = manager
        .iter_all()
        .find(|entity| entity.entity_type == 17)
        .unwrap()
        .id;
    let child = manager
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .unwrap()
        .id;
    let mut scheduler = SpecializedActorTaskScheduler::default();
    scheduler
        .adopt_fresh_level1_type9_selected(&mut manager)
        .unwrap();
    scheduler.adopt_intro2_type8(&mut manager);
    scheduler.adopt_intro2_type17(&manager);
    let position = actor(&manager, parent).unwrap().position_raw();
    manager.entity_mut(child).unwrap().set_position_raw([
        position[0].wrapping_add(32),
        position[1],
        position[2].wrapping_add(32),
    ]);
    for _ in 0..32 {
        crate::intro2_type17::behavior::reselect(
            &mut manager,
            parent,
            300,
            &mut fx,
            crate::intro2_type17::behavior::ReselectionEntry::Impact,
        )
        .unwrap();
        if matches!(actor(&manager,parent).unwrap().current_behavior_context,RetailRuntimeValue::Known(Some(context)) if context.descriptor()==crate::entity_behavior::BehaviorDescriptorIdentity::Named(crate::entity_behavior::behavior_program(9).unwrap()))
        {
            break;
        }
    }
    assert!(
        matches!(actor(&manager,parent).unwrap().current_behavior_context,RetailRuntimeValue::Known(Some(context)) if context.descriptor()==crate::entity_behavior::BehaviorDescriptorIdentity::Named(crate::entity_behavior::behavior_program(9).unwrap())),
        "actual nearby peasant must reach Capture through the weighted selector"
    );
    scheduler.register_intro2_type17(
        crate::intro2_type17::Intro2Type17Owner::adopt(&manager, parent).unwrap(),
    );
    let mut notifications = GameplayNotifications::new();
    attach_capture_child(
        &mut manager,
        parent,
        child,
        &mut CaptureContext {
            resources: None,
            tasks: &mut scheduler,
            world_fx: &mut fx,
            notifications: &mut notifications,
            retail_tick: 300,
            result_screen: MainBaseType9ResultScreenState::NotShown,
            hive_dying: Default::default(),
        },
    )
    .unwrap();
    // Invoke the real post-C910 class2 constructor. The separate active-pair
    // suite supplies overlap/A300 and destination-search coverage.
    crate::intro2_type17::carry_tasks::publish_style(
        &mut manager,
        parent,
        2,
        crate::intro2_type17::carry_tasks::CaptureTargetWrite::Set(Some(child)),
        &mut fx,
    )
    .unwrap();
    scheduler.register_intro2_type17(
        crate::intro2_type17::Intro2Type17Owner::adopt(&manager, parent).unwrap(),
    );
    Some(Fixture {
        session,
        manager,
        scheduler,
        fx,
        notifications,
        parent,
        child,
    })
}

#[v2k_test_support::retail_test]
fn direct_capture_death_runs_nested_and_current_outer_c620() {
    let Some(mut f) = fixture() else {
        return;
    };
    let mut expected = f.manager.fork_for_main_base_abort_transaction();
    let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
    let mut expected_tasks = f.scheduler.fork_for_main_base_abort_transaction();
    let mut expected_notifications = GameplayNotifications::new();
    let parent = expected.entity_mut(f.parent).unwrap();
    parent.collision.health_raw = RetailRuntimeValue::Known(0);
    parent
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    let first = execute_capture_root(
        &mut expected,
        f.parent,
        CaptureRootCallback::Cleanup,
        &mut CaptureContext {
            resources: None,
            tasks: &mut expected_tasks,
            world_fx: &mut expected_fx,
            notifications: &mut expected_notifications,
            retail_tick: 301,
            result_screen: MainBaseType9ResultScreenState::NotShown,
            hive_dying: Default::default(),
        },
    )
    .unwrap();
    assert!(first.common_dying_owner.is_some());
    let first_primary = actor(&expected, f.parent)
        .unwrap()
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary);
    crate::intro2_common_dying::publish_intro2_common_dying_alternate(
        &mut expected,
        f.parent,
        &mut expected_fx,
    )
    .unwrap()
    .unwrap();
    let second_primary = actor(&expected, f.parent)
        .unwrap()
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary);
    assert_ne!(
        first_primary, second_primary,
        "outer DB80 replaces the nested Primary"
    );
    let owner = publish_type17_standard_death(
        &mut f.manager,
        f.parent,
        &mut CaptureContext {
            resources: None,
            tasks: &mut f.scheduler,
            world_fx: &mut f.fx,
            notifications: &mut f.notifications,
            retail_tick: 301,
            result_screen: MainBaseType9ResultScreenState::NotShown,
            hive_dying: Default::default(),
        },
    )
    .unwrap()
    .unwrap();
    f.scheduler.register_intro2_common_dying(owner);
    assert_eq!(
        actor(&f.manager, f.parent)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary),
        second_primary
    );
    assert_eq!(
        f.fx.next_shared_retail_random_u16(),
        expected_fx.next_shared_retail_random_u16(),
        "two parent constructors after the child release selector"
    );
    assert_eq!(actor(&f.manager, f.child).unwrap().attached_to, None);
    assert!(actor(&f.manager, f.parent)
        .unwrap()
        .native_capture_relation
        .is_none());
}

#[v2k_test_support::retail_test]
fn release_of_captured_corpse_keeps_its_exact_class14_task() {
    let Some(mut f) = fixture() else {
        return;
    };
    kill_capture_contact(
        &mut f.manager,
        f.child,
        &mut CaptureContext {
            resources: None,
            tasks: &mut f.scheduler,
            world_fx: &mut f.fx,
            notifications: &mut f.notifications,
            retail_tick: 301,
            result_screen: MainBaseType9ResultScreenState::NotShown,
            hive_dying: Default::default(),
        },
    )
    .unwrap();
    let child = actor(&f.manager, f.child).unwrap();
    assert_eq!(child.attached_to, Some(f.parent));
    let before = child.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
    let state = *child.actor_task_state(ActorTaskSlot::Primary).unwrap();
    let animation = child.actor_animation_runtime;
    publish_type17_standard_death(
        &mut f.manager,
        f.parent,
        &mut CaptureContext {
            resources: None,
            tasks: &mut f.scheduler,
            world_fx: &mut f.fx,
            notifications: &mut f.notifications,
            retail_tick: 302,
            result_screen: MainBaseType9ResultScreenState::NotShown,
            hive_dying: Default::default(),
        },
    )
    .unwrap()
    .unwrap();
    let child = actor(&f.manager, f.child).unwrap();
    assert_eq!(child.attached_to, None);
    assert_eq!(
        child.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        before
    );
    assert_eq!(child.actor_task_state(ActorTaskSlot::Primary), Some(&state));
    assert_eq!(child.actor_animation_runtime, animation);
}

#[v2k_test_support::retail_test]
fn primary_hit_cleans_up_while_living_before_checked_damage() {
    let Some(mut f) = fixture() else {
        return;
    };
    let old = actor(&f.manager, f.parent)
        .unwrap()
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary);
    let packet = crate::damage::PRIMARY_PROJECTILE_DAMAGE_PACKET;
    let impact = crate::world_fx::ParticleEntityImpact {
        source_particle_class: 1,
        impact_position_argument_va: 0,
        target_entity_id: f.parent,
        position_world: [0.0; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(crate::world_fx::BallisticDamageRequest {
            packet,
            source_entity_type_at_birth: Some(46),
            source_owner_id: Some(f.parent),
        }),
    };
    let result = crate::shared_actor_impact::apply_shared_actor_particle_hit(
        crate::shared_actor_impact::SharedActorImpactFrame {
            entities: &mut f.manager,
            resources: &f.session.cache,
            world_fx: &mut f.fx,
            scheduler: &mut f.scheduler,
            notifications: &mut f.notifications,
            retail_tick: 301,
        },
        impact,
    );
    assert!(
        matches!(
            result,
            Some(
                crate::shared_actor_impact::SharedActorImpactOutcome::Spider(
                    crate::intro2_type17::impact::Intro2Type17ImpactOutcome::Applied(_)
                )
            )
        ),
        "{result:?}"
    );
    let parent = actor(&f.manager, f.parent).unwrap();
    assert_ne!(parent.actor_tasks.task_in_slot(ActorTaskSlot::Primary), old);
    assert!(!matches!(
        parent.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::CommonDying(_))
    ));
    assert!(parent.native_capture_relation.is_none());
    assert_eq!(actor(&f.manager, f.child).unwrap().attached_to, None);
}

#[v2k_test_support::retail_test]
fn foreign_capture_row_lease_blocks_before_child_or_rng_mutation() {
    let Some(mut f) = fixture() else {
        return;
    };
    let other = f
        .manager
        .iter_all()
        .find(|entity| entity.entity_type == 9 && entity.id != f.child)
        .unwrap()
        .id;
    let foreign = f
        .manager
        .main_base_abort_actor_observation(other)
        .unwrap()
        .lease;
    f.manager
        .entity_mut(f.parent)
        .unwrap()
        .native_capture_relation
        .as_mut()
        .unwrap()
        .child = foreign;
    let mut expected = f.fx.fork_for_main_base_abort_transaction();
    let before = actor(&f.manager, f.child)
        .unwrap()
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary);
    let result = execute_capture_root(
        &mut f.manager,
        f.parent,
        CaptureRootCallback::Cleanup,
        &mut CaptureContext {
            resources: None,
            tasks: &mut f.scheduler,
            world_fx: &mut f.fx,
            notifications: &mut f.notifications,
            retail_tick: 301,
            result_screen: MainBaseType9ResultScreenState::NotShown,
            hive_dying: Default::default(),
        },
    );
    assert!(matches!(
        result,
        Err(CaptureBlock {
            reason: "capture relation allocation changed",
            committed_prefix: true
        })
    ));
    assert_eq!(
        actor(&f.manager, f.child).unwrap().attached_to,
        Some(f.parent)
    );
    assert_eq!(
        actor(&f.manager, f.child)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary),
        before
    );
    assert_eq!(
        f.fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn cf90_releases_then_uses_its_own_rng_branch_for_child_standard_death() {
    for kill in [false, true] {
        let Some(mut f) = fixture() else {
            return;
        };
        for _ in 0..32 {
            let mut probe = f.fx.fork_for_main_base_abort_transaction();
            if (probe.next_shared_retail_random_u16() & 3 == 0) == kill {
                break;
            }
            f.fx.next_shared_retail_random_u16();
        }
        let mut probe = f.fx.fork_for_main_base_abort_transaction();
        assert_eq!(probe.next_shared_retail_random_u16() & 3 == 0, kill);
        let before = actor(&f.manager, f.child).unwrap().collision.health_raw;
        let result = execute_capture_root(
            &mut f.manager,
            f.parent,
            CaptureRootCallback::ReleaseOrKill,
            &mut CaptureContext {
                resources: None,
                tasks: &mut f.scheduler,
                world_fx: &mut f.fx,
                notifications: &mut f.notifications,
                retail_tick: 301,
                result_screen: MainBaseType9ResultScreenState::NotShown,
                hive_dying: Default::default(),
            },
        )
        .unwrap();
        assert!(result.common_dying_owner.is_none());
        let child = actor(&f.manager, f.child).unwrap();
        assert_eq!(child.attached_to, None);
        assert_eq!(
            child.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(if kill { DYING_STATE_BIT } else { 0 })
        );
        assert_eq!(
            child.collision.health_raw,
            if kill {
                RetailRuntimeValue::Known(0)
            } else {
                before
            }
        );
        assert!(
            !f.manager
                .pending_actor_deferred_destroy_ids()
                .contains(&f.child),
            "CF90 uses10C10, not the destination's443D10 deferred callback"
        );
    }
}

#[v2k_test_support::retail_test]
fn d0b0_releases_and_defers_without_child_standard_death() {
    let Some(mut f) = fixture() else {
        return;
    };
    let destination = f
        .manager
        .iter_all()
        .find(|entity| entity.capability_flags & 0x10 != 0)
        .expect("authored Hive destination")
        .id;
    let before = actor(&f.manager, f.child).unwrap().collision.health_raw;
    execute_capture_delivery(
        &mut f.manager,
        f.parent,
        destination,
        &mut CaptureContext {
            resources: None,
            tasks: &mut f.scheduler,
            world_fx: &mut f.fx,
            notifications: &mut f.notifications,
            retail_tick: 301,
            result_screen: MainBaseType9ResultScreenState::NotShown,
            hive_dying: Default::default(),
        },
    )
    .unwrap();
    let child = actor(&f.manager, f.child).unwrap();
    assert_eq!(child.attached_to, None);
    assert_eq!(child.collision.health_raw, before);
    assert_eq!(
        child
            .collision
            .state_flags_at_0x08
            .masked(DYING_STATE_BIT | 0x100000),
        RetailRuntimeValue::Known(0x100000)
    );
    assert_eq!(
        f.manager
            .pending_actor_deferred_destroy_ids()
            .iter()
            .filter(|id| **id == f.child)
            .count(),
        1
    );
    assert!(actor(&f.manager, f.parent)
        .unwrap()
        .native_capture_relation
        .is_none());
}

#[v2k_test_support::retail_test]
fn compacted_captured_corpse_releases_on_its_next_retained_class14_visit() {
    use crate::intro2_type9_class14::{
        tick_intro2_type9_class14, Intro2Type9Class14Frame, Intro2Type9Class14Outcome,
        Intro2Type9Class14Owner,
    };
    for detailed in [false, true] {
        let Some(mut f) = fixture() else { return };
        kill_capture_contact(
            &mut f.manager,
            f.child,
            &mut CaptureContext {
                resources: None,
                tasks: &mut f.scheduler,
                world_fx: &mut f.fx,
                notifications: &mut f.notifications,
                retail_tick: 301,
                result_screen: MainBaseType9ResultScreenState::NotShown,
                hive_dying: Default::default(),
            },
        )
        .unwrap();
        let allocation = f
            .manager
            .main_base_abort_actor_observation(f.child)
            .unwrap()
            .lease;
        let lease = crate::ordinary_type9_outer_tail::outer_tail_class14_exploding_task_lease(
            &f.manager, allocation,
        )
        .unwrap();
        let primary = actor(&f.manager, f.child)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary);
        let mut owner = Intro2Type9Class14Owner::adopt(lease);
        for visit in 0..2 {
            if visit == 1 {
                crate::native_actor_capture::update_carried_pose(&mut f.manager, f.parent).unwrap();
                assert_eq!(
                    actor(&f.manager, f.child).unwrap().attached_to,
                    Some(f.parent),
                    "18640 compacts the row without invoking its release callback"
                );
                assert!(
                    !actor(&f.manager, f.parent)
                        .unwrap()
                        .native_capture_relation
                        .unwrap()
                        .row_present
                );
            }
            let child = f.manager.entity_mut(f.child).unwrap();
            // Deterministic detailed/coarse 12DA0 boundaries, as used by the
            // class14 world tests: no scheduler random wait and 125ms callback.
            child.collision.state_flags_at_0x08.overwrite(
                crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
                if detailed {
                    crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
                } else {
                    0
                },
            );
            child.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(250_000);
            child.collision.callback_scheduler_accumulator_us_at_0x6c =
                RetailRuntimeValue::Known(125_001);
            let before_animation = child.actor_animation_runtime;
            let tick = tick_intro2_type9_class14(
                &mut f.manager,
                owner,
                Intro2Type9Class14Frame {
                    resources: &f.session.cache,
                    world_fx: &mut f.fx,
                    elapsed_micros: 0,
                    global_elapsed_micros: 20_000,
                    retail_tick: 302 + visit,
                },
            );
            assert!(
                matches!(tick.outcome, Intro2Type9Class14Outcome::Advanced {
                detailed: actual, task: Some(_), ..
            } if actual == detailed),
                "{:?}",
                tick.outcome
            );
            owner = tick.retained_owner.unwrap();
            let child = actor(&f.manager, f.child).unwrap();
            assert_eq!(
                child.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                primary,
                "membership repair keeps the original class14 task"
            );
            assert_eq!(
                child.attached_to,
                if visit == 0 { Some(f.parent) } else { None }
            );
            if !detailed {
                assert_eq!(child.actor_animation_runtime, before_animation);
            }
            assert!(owner.completed_hit_boundary(&f.manager));
        }
    }
}
