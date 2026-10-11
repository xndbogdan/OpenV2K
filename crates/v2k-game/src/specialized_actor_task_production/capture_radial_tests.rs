use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    entity_collision_state::CHECKED_DAMAGE_ENABLED_STATE_BIT,
    intro2_radial::{apply_intro2_radial_damage, Intro2RadialFrame, Intro2RadialReport},
    intro2_type17::capture::{
        publish_type17_standard_death,
        tests::{fixture, Fixture},
        CaptureContext,
    },
    main_base_type9_abort::MainBaseType9ResultScreenState,
    player_hull::PlayerHull,
    radial_damage::RadialDamageTemplate,
    static_damage::StaticDamageScheduler,
};

const BLAST: RadialDamageTemplate = RadialDamageTemplate {
    inner_radius_raw: 8,
    outer_radius_raw: 16,
    impulse_raw: 2000,
    packet: DamagePacket {
        channels: [1, 3],
        amounts_raw: [20_000, 20_000],
    },
    trailing_raw: [0, 0],
};

#[derive(Clone, Copy, Debug)]
enum Route {
    Playing,
    Intro2,
    Cursor { child_visited: bool },
}

fn origin(f: &mut Fixture) -> [i16; 3] {
    let mut origin = f.manager.entity_mut(f.parent).unwrap().position_raw();
    origin[0] = origin[0].wrapping_sub(1);
    origin
}

fn isolate_target(f: &mut Fixture) {
    let ids: Vec<_> = f
        .manager
        .iter_all()
        .filter(|entity| entity.id != f.parent)
        .map(|entity| entity.id)
        .collect();
    for id in ids {
        f.manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(CHECKED_DAMAGE_ENABLED_STATE_BIT, 0);
    }
}

fn blast(f: &mut Fixture, route: Route) -> Option<crate::entity::DynamicRadialLiveBlock> {
    let origin_raw = origin(f);
    if matches!(route, Route::Playing) {
        let outcome = f.scheduler.apply_playing_radial_damage(PlayingRadialFrame {
            extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            resources: &mut f.session.cache,
            static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
            active_terminal_calls: Vec::new(),
            entities: &mut f.manager,
            player_hull: &mut PlayerHull::default(),
            origin_raw,
            template: BLAST,
            world_fx: &mut f.fx,
            notifications: &mut f.notifications,
            retail_tick: 301,
        });
        return match outcome.blocked {
            None => None,
            Some(PlayingRadialBlock::Native(error)) => Some(error),
            Some(error) => panic!("{error:?}"),
        };
    }
    let mut pending = Vec::new();
    let mut retained = Vec::new();
    let mut remaining = Vec::new();
    if let Route::Cursor { child_visited } = route {
        for owner in std::mem::take(&mut f.scheduler.owners) {
            if child_visited && owner.entity_id() == f.child {
                retained.push(owner);
            } else {
                remaining.push(owner.entity_id());
                pending.push(owner);
            }
        }
    }
    let mut cursor = Intro2RadialCursorCustody {
        pending: &mut pending,
        retained: &mut retained,
        remaining_live_ids: &remaining,
    };
    let custody: &mut dyn crate::intro2_radial::Intro2RadialTaskCustody =
        if matches!(route, Route::Intro2) {
            &mut f.scheduler
        } else {
            &mut cursor
        };
    let report = apply_intro2_radial_damage(
        &mut Intro2RadialFrame {
            active_terminal_calls: Vec::new(),
            entities: &mut f.manager,
            resources: &mut f.session.cache,
            world_fx: &mut f.fx,
            static_damage: &mut StaticDamageScheduler::new(),
            notifications: &mut f.notifications,
            retail_tick: 301,
            actor_tasks: custody,
        },
        origin_raw,
        BLAST,
    );
    let Intro2RadialReport::Applied { dynamic, .. } = report else {
        panic!("{report:?}")
    };
    if let Route::Cursor { child_visited } = route {
        assert_eq!(
            retained.iter().any(|owner| owner.entity_id() == f.child),
            child_visited
        );
        assert_eq!(
            pending.iter().any(|owner| owner.entity_id() == f.child),
            !child_visited
        );
        assert!(pending.iter().any(|owner| owner.entity_id() == f.parent));
        f.scheduler.owners.extend(pending);
        f.scheduler.owners.extend(retained);
    }
    dynamic.blocked
}

#[v2k_test_support::retail_test]
fn radial_capture_death_releases_child_and_replaces_parent_in_each_live_custody() {
    for route in [
        Route::Playing,
        Route::Intro2,
        Route::Cursor {
            child_visited: false,
        },
        Route::Cursor {
            child_visited: true,
        },
    ] {
        let Some(mut f) = fixture() else {
            return;
        };
        isolate_target(&mut f);
        let mut expected_manager = f.manager.fork_for_main_base_abort_transaction();
        let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
        let mut expected_scheduler = f.scheduler.fork_for_main_base_abort_transaction();
        publish_type17_standard_death(
            &mut expected_manager,
            f.parent,
            &mut CaptureContext {
                resources: None,
                tasks: &mut expected_scheduler,
                world_fx: &mut expected_fx,
                notifications: &mut GameplayNotifications::new(),
                retail_tick: 301,
                result_screen: MainBaseType9ResultScreenState::NotShown,
                hive_dying: Default::default(),
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(blast(&mut f, route), None, "{route:?}");
        let parent = f.manager.entity_mut(f.parent).unwrap();
        assert!(parent.native_capture_relation.is_none());
        assert_eq!(parent.collision.health_raw, RetailRuntimeValue::Known(0));
        assert!(matches!(
            parent.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::CommonDying(_))
        ));
        let primary = parent.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
        assert_eq!(
            primary,
            expected_manager
                .entity_mut(f.parent)
                .unwrap()
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
        );
        let child = f.manager.entity_mut(f.child).unwrap();
        assert_eq!(child.attached_to, None);
        assert_eq!(
            child.current_behavior_context,
            expected_manager
                .entity_mut(f.child)
                .unwrap()
                .current_behavior_context
        );
        assert!(f.scheduler.owners.iter().any(|owner| matches!(owner,
            SpecializedActorTaskOwner::Intro2CommonDying(owner) if owner.entity_id() == f.parent)));
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected_fx.next_shared_retail_random_u16(),
            "{route:?}: child release then nested and outer C620"
        );
        let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
        assert_eq!(
            blast(&mut f, route),
            None,
            "{route:?}: already dying repeat"
        );
        assert_eq!(
            f.manager
                .entity_mut(f.parent)
                .unwrap()
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary),
            primary
        );
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected_fx.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn radial_capture_child_custody_failure_parks_the_committed_parent_prefix() {
    for route in [Route::Playing, Route::Intro2] {
        let Some(mut f) = fixture() else {
            return;
        };
        isolate_target(&mut f);
        f.scheduler
            .owners
            .retain(|owner| owner.entity_id() != f.child);
        let child_before = f.manager.entity_mut(f.child).unwrap().collision.clone();
        let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
        let error = blast(&mut f, route).expect("actual child ownership is required");
        assert_eq!(error.target_id, f.parent);
        assert_eq!(error.phase, crate::entity::DynamicRadialLivePhase::Death);
        assert!(error.target_prefix_committed);
        assert_eq!(
            f.manager.entity_mut(f.parent).unwrap().collision.health_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            f.manager.entity_mut(f.child).unwrap().attached_to,
            Some(f.parent)
        );
        assert_eq!(
            f.manager.entity_mut(f.child).unwrap().collision,
            child_before
        );
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected_fx.next_shared_retail_random_u16(),
            "D040 does not draw before child cleanup"
        );
        let parent_before = f.manager.entity_mut(f.parent).unwrap().collision.clone();
        let event_count = f.fx.pending_event_count();
        let error = blast(&mut f, route).expect("a parked death cannot replay its prefix");
        assert_eq!(
            error.phase,
            crate::entity::DynamicRadialLivePhase::MutationCustody
        );
        assert!(!error.target_prefix_committed);
        assert_eq!(
            f.manager.entity_mut(f.parent).unwrap().collision,
            parent_before
        );
        assert_eq!(f.fx.pending_event_count(), event_count);
    }
}

#[v2k_test_support::retail_test]
fn type122_radial_late_child_custody_failure_parks_each_live_storage_without_replay() {
    for child_type in [8, 9] {
        for route in [
            Route::Playing,
            Route::Intro2,
            Route::Cursor {
                child_visited: false,
            },
            Route::Cursor {
                child_visited: true,
            },
        ] {
            let mut native = crate::native_actor_capture::tests::Fixture::new(child_type);
            native.attach().unwrap();
            native.carry(2);
            let mut f = Fixture {
                session: native._session,
                manager: native.manager,
                scheduler: native.tasks,
                fx: native.fx,
                notifications: native.notifications,
                parent: native.parent,
                child: native.child,
            };
            isolate_target(&mut f);
            // Retain the actual child and its owner in its cursor storage, but
            // park an earlier external prefix so D040 cannot mutate it again.
            assert!(f.scheduler.park_native_contact_prefix(&f.manager, f.child));
            let child_before = f.manager.entity_mut(f.child).unwrap().collision.clone();
            let child_primary = f
                .manager
                .entity_mut(f.child)
                .unwrap()
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary);
            let parent_primary = f
                .manager
                .entity_mut(f.parent)
                .unwrap()
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary);
            let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
            let error =
                blast(&mut f, route).expect("actual parked child ownership must block D040");
            assert_eq!(error.target_id, f.parent, "{route:?} child{child_type}");
            assert_eq!(error.phase, crate::entity::DynamicRadialLivePhase::Death);
            assert!(error.target_prefix_committed);
            assert_eq!(
                f.manager.entity_mut(f.parent).unwrap().collision.health_raw,
                RetailRuntimeValue::Known(0)
            );
            assert_eq!(
                f.manager.entity_mut(f.child).unwrap().attached_to,
                Some(f.parent)
            );
            assert_eq!(
                f.manager.entity_mut(f.child).unwrap().collision,
                child_before
            );
            assert_eq!(
                f.manager
                    .entity_mut(f.child)
                    .unwrap()
                    .actor_tasks
                    .task_in_slot(ActorTaskSlot::Primary),
                child_primary
            );
            assert_eq!(
                f.manager
                    .entity_mut(f.parent)
                    .unwrap()
                    .actor_tasks
                    .task_in_slot(ActorTaskSlot::Primary),
                parent_primary
            );
            assert_eq!(
                f.fx.next_shared_retail_random_u16(),
                expected_fx.next_shared_retail_random_u16(),
                "D040 blocks before child release or parent C620 draws"
            );
            let parent_before = f.manager.entity_mut(f.parent).unwrap().collision.clone();
            let events = f.fx.pending_event_count();
            let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
            let error = blast(&mut f, route).expect("parked parent prefix must not replay");
            assert_eq!(
                error.phase,
                crate::entity::DynamicRadialLivePhase::MutationCustody
            );
            assert!(!error.target_prefix_committed);
            assert_eq!(
                f.manager.entity_mut(f.parent).unwrap().collision,
                parent_before
            );
            assert_eq!(
                f.manager.entity_mut(f.child).unwrap().collision,
                child_before
            );
            assert_eq!(
                f.manager
                    .entity_mut(f.parent)
                    .unwrap()
                    .actor_tasks
                    .task_in_slot(ActorTaskSlot::Primary),
                parent_primary
            );
            assert_eq!(
                f.manager
                    .entity_mut(f.child)
                    .unwrap()
                    .actor_tasks
                    .task_in_slot(ActorTaskSlot::Primary),
                child_primary
            );
            assert_eq!(f.fx.pending_event_count(), events);
            assert_eq!(
                f.fx.next_shared_retail_random_u16(),
                expected_fx.next_shared_retail_random_u16()
            );
        }
    }
}
