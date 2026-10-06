use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::{AuthoredWorldConstruction, EntityConstructionResources},
    entity_collision_state::EntityTypeRuntimeMetadata,
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskFamily,
};

struct Fixture {
    session: GameSession,
    entities: EntityManager,
    fx: WorldFx,
    scheduler: SpecializedActorTaskScheduler,
}

fn fixture() -> Option<Fixture> {
    fixture_for_world(25)
}

fn fixture_for_world(world: u32) -> Option<Fixture> {
    let data = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    // World25 contains all three native families and thirteen real factories,
    // including allocations whose spawn/pose/template differ from Level1.
    session.load_level_by_id(world, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(kind, _)| {
            EntityTypeRuntimeMetadata::from_section12(
                session.cache.global_entity_type(kind).unwrap(),
            )
        })
        .collect();
    let mut fx = WorldFx::new();
    let mut entities = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            logical_world_index: (world - 12) as i32,
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
            retail_tick: 0,
        },
        &mut fx,
    )
    .unwrap();
    assert!(!entities.is_fresh_new_game_first_world());
    let mut scheduler = SpecializedActorTaskScheduler::default();
    scheduler
        .adopt_fresh_level1_type9_selected(&mut entities)
        .unwrap();
    scheduler.adopt_intro2_type8(&mut entities);
    scheduler.adopt_intro2_type17(&entities);
    scheduler.adopt_intro2_type47_guards(&entities);
    scheduler.adopt_intro2_type66(&entities);
    fx.process_pending();
    fx.take_positional_sounds();
    Some(Fixture {
        session,
        entities,
        fx,
        scheduler,
    })
}

fn id(fixture: &Fixture, kind: u32) -> u32 {
    fixture
        .entities
        .iter_all()
        .find(|entity| entity.entity_type == kind)
        .unwrap()
        .id
}

fn invoke(
    fixture: &mut Fixture,
    id: u32,
) -> (
    Result<DispatchSuccess, DispatchFailure>,
    MainBaseAbortPublicationCounts,
) {
    let actor = fixture
        .entities
        .main_base_abort_actor_observation(id)
        .unwrap();
    let geometry = snapshot_main_base_abort_terrain_geometry(&fixture.session.cache).unwrap();
    let mut static_damage = StaticDamageScheduler::default();
    let mut effects =
        MainBaseAbortWorldEffects::new(&geometry, &mut fixture.session.cache, &mut static_damage)
            .unwrap();
    let mut publications = MainBaseAbortPublicationCounts::default();
    // Exercise the actual production switch, not only the native helper.
    let result = dispatch_ordinary_actor(
        actor,
        &mut None,
        &mut fixture.entities,
        geometry.terrain(),
        &mut PlayerHull::default(),
        &mut fixture.fx,
        &mut effects,
        &mut fixture.scheduler,
        &mut publications,
        &mut MainBaseAbortGameplayContext {
            extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            retail_tick: 400,
        },
    );
    (result, publications)
}

fn require_success(result: Result<DispatchSuccess, DispatchFailure>) -> DispatchSuccess {
    match result {
        Ok(result) => result,
        Err(failure) => panic!(
            "abort callback blocked: {:?}, {:?}",
            failure.callback_error, failure.diagnostic
        ),
    }
}

#[v2k_test_support::retail_test]
fn later_world_abort_adopts_shared_worker_peasant_and_factory_death_graphs() {
    let Some(mut fixture) = fixture() else {
        return;
    };
    for kind in [8, 9, 66] {
        let id = id(&fixture, kind);
        let entity = fixture.entities.entity_mut(id).unwrap();
        let before = (
            entity.position_raw(),
            entity.physical_body_basis_q31,
            entity.model_slots,
        );
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let next = fixture
            .entities
            .main_base_abort_successor_after_callback(id)
            .unwrap();
        let mut oracle = fixture.fx.fork_for_main_base_abort_transaction();
        oracle.next_shared_retail_random_u16(); // C3A0 Sub-A or257C0 selector, exactly once.
        let (result, publications) = invoke(&mut fixture, id);
        let result = require_success(result);
        assert!(result.successor_available);
        assert_eq!(result.successor, next);
        assert_eq!(publications.specialized_total(), 1);
        assert_eq!(
            fixture.fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
        let entity = fixture.entities.entity_mut(id).unwrap();
        assert_eq!(
            (
                entity.position_raw(),
                entity.physical_body_basis_q31,
                entity.model_slots
            ),
            before
        );
        assert_ne!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            Some(primary)
        );
        assert_eq!(entity.actor_tasks.wrapper_flags(primary), None);
        if kind == 66 {
            assert_eq!(
                result.disposition,
                MainBaseAbortActorDisposition::Type66Death
            );
            assert_eq!(publications.type66_production, 1);
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(10_000_000)
            );
            let RetailRuntimeValue::Known(Some(factory)) = entity.base_factory_runtime else {
                panic!()
            };
            assert_eq!(factory.progressive_death.elapsed_micros_raw, 1);
            assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::WorkingFactory(_))
            ));
            assert_eq!(
                fixture.scheduler.family_for(id),
                Some(SpecializedActorTaskFamily::Intro2Type66)
            );
        } else {
            assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
            assert_eq!(
                entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
                RetailRuntimeValue::Known(DYING_STATE_BIT)
            );
            assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::SharedRetarget(_))
            ));
            assert_eq!(
                fixture.scheduler.family_for(id),
                Some(if kind == 8 {
                    SpecializedActorTaskFamily::Intro2Type8
                } else {
                    SpecializedActorTaskFamily::OrdinaryType9Class14
                })
            );
            assert_eq!(publications.type8_exploding, usize::from(kind == 8));
            assert_eq!(publications.type9_exploding, usize::from(kind == 9));
        }
        assert_eq!(fixture.fx.pending_event_count(), usize::from(kind != 8));
        // 4F450 queues the logical request; the normal FX phase materializes
        // it before the audio layer drains ready positional sounds.
        fixture.fx.process_pending();
        let sounds = fixture.fx.take_positional_sounds();
        let expected_sound = match kind {
            8 => None,
            9 => Some(35),
            66 => Some(62),
            _ => unreachable!(),
        };
        let expected: Vec<_> = expected_sound
            .into_iter()
            .map(|sound_id| {
                crate::world_fx::PositionalSoundEvent::fixed(
                    sound_id,
                    [
                        f32::from(before.0[0] as u16) / 256.0,
                        f32::from(before.0[1]) / 256.0,
                        f32::from(before.0[2] as u16) / 256.0,
                    ],
                )
            })
            .collect();
        assert_eq!(sounds, expected);
    }
}

#[v2k_test_support::retail_test]
fn native_abort_revisit_preserves_corpses_and_remote_factory_without_republishing() {
    let Some(mut fixture) = fixture() else {
        return;
    };
    for kind in [8, 9, 66] {
        let id = id(&fixture, kind);
        if kind == 66 {
            fixture
                .entities
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(REMOTE_OWNED_STATE_BIT, REMOTE_OWNED_STATE_BIT);
        } else {
            require_success(invoke(&mut fixture, id).0);
        }
        fixture.fx.process_pending();
        fixture.fx.take_positional_sounds();
        let entity = fixture.entities.entity_mut(id).unwrap();
        let before = (
            entity.collision.clone(),
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            entity.base_factory_runtime,
        );
        let family = fixture.scheduler.family_for(id);
        let mut oracle = fixture.fx.fork_for_main_base_abort_transaction();
        let (result, publications) = invoke(&mut fixture, id);
        require_success(result);
        assert_eq!(publications, MainBaseAbortPublicationCounts::default());
        assert_eq!(fixture.scheduler.family_for(id), family);
        let entity = fixture.entities.entity_mut(id).unwrap();
        assert_eq!(
            (
                entity.collision.clone(),
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                entity.base_factory_runtime
            ),
            before
        );
        assert!(fixture.fx.take_positional_sounds().is_empty());
        assert_eq!(fixture.fx.pending_event_count(), 0);
        assert_eq!(
            fixture.fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn native_abort_rejects_foreign_receipts_and_pending_factory_custody_before_writes() {
    let Some(mut fixture) = fixture() else {
        return;
    };
    let Some(foreign) = self::fixture() else {
        return;
    };
    for kind in [8, 9, 66] {
        let id = id(&fixture, kind);
        let other_id = self::id(&foreign, kind);
        let other = foreign
            .entities
            .iter_all()
            .find(|entity| entity.id == other_id)
            .unwrap();
        let entity = fixture.entities.entity_mut(id).unwrap();
        match kind {
            8 => entity.intro2_type8_runtime = other.intro2_type8_runtime,
            9 => entity.ordinary_type9_native_receipt = other.ordinary_type9_native_receipt,
            66 => entity.intro2_type66_runtime = other.intro2_type66_runtime,
            _ => unreachable!(),
        }
        // No-op admission bypasses task custody, so the callback itself must
        // reject the foreign manager receipt before its generic-death prefix.
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(REMOTE_OWNED_STATE_BIT, REMOTE_OWNED_STATE_BIT);
        let collision = entity.collision.clone();
        let mut oracle = fixture.fx.fork_for_main_base_abort_transaction();
        let (result, publications) = invoke(&mut fixture, id);
        assert!(matches!(
            result,
            Err(DispatchFailure {
                callback_error: Some(MainBaseAbortActorCallbackBlock::Native(_)),
                ..
            })
        ));
        assert_eq!(publications, MainBaseAbortPublicationCounts::default());
        assert_eq!(
            fixture.entities.entity_mut(id).unwrap().collision,
            collision
        );
        assert!(fixture.fx.take_positional_sounds().is_empty());
        assert_eq!(fixture.fx.pending_event_count(), 0);
        assert_eq!(
            fixture.fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }

    let Some(mut fixture) = self::fixture() else {
        return;
    };
    let id = id(&fixture, 66);
    let pending =
        crate::intro2_type66::Intro2Type66Owner::adopt_blocked_prefix(&fixture.entities, id)
            .unwrap();
    fixture.scheduler.register_intro2_type66(pending);
    let before = fixture.entities.entity_mut(id).unwrap().collision.clone();
    let (result, _) = invoke(&mut fixture, id);
    assert!(matches!(
        result,
        Err(DispatchFailure {
            callback_error: Some(MainBaseAbortActorCallbackBlock::Native(
                NativeMainBaseAbortDeathBlock::TaskCustodyUnavailable
            )),
            ..
        })
    ));
    assert_eq!(fixture.entities.entity_mut(id).unwrap().collision, before);
}

#[v2k_test_support::retail_test]
fn later_world_type17_abort_replaces_the_actual_living_graph_with_common_dying() {
    // These are the three later authored worlds containing Type17. Their
    // construction, selected graph and classifier owner come from the actual
    // Section13 rows, without the captured Level1 spawn17..20 predecessor.
    for world in [14, 20, 35] {
        let Some(mut fixture) = fixture_for_world(world) else {
            return;
        };
        let id = id(&fixture, 17);
        assert!(
            crate::intro2_type17::type17_manager_allocation_authenticates(&fixture.entities, id)
        );
        assert_eq!(
            fixture.scheduler.family_for(id),
            Some(SpecializedActorTaskFamily::Intro2Type17)
        );
        let entity = fixture.entities.entity_mut(id).unwrap();
        let before = (
            entity.position_raw(),
            entity.model_slots,
            entity.physical_body_basis_q31,
            entity.construction_stamp_at_0xb4,
            entity.intro2_type17_runtime,
            entity.type17_sub_d_frame_owner,
            entity.type17_sub_d_runtime,
        );
        let old_tasks =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot));
        assert!(old_tasks.iter().any(Option::is_some));
        let old_velocity = entity.velocity_raw();
        let next = fixture
            .entities
            .main_base_abort_successor_after_callback(id)
            .unwrap();
        let mut oracle = fixture.fx.fork_for_main_base_abort_transaction();
        oracle.next_shared_retail_random_u16(); // C620's real Sub-A speed draw.
        let (result, publications) = invoke(&mut fixture, id);
        let result = require_success(result);
        assert_eq!(
            result.disposition,
            MainBaseAbortActorDisposition::Type17Death
        );
        assert_eq!(result.successor, next);
        assert!(result.successor_available);
        assert_eq!(publications.type17_common_dying, 1);
        assert_eq!(publications.specialized_total(), 1);
        assert_eq!(
            fixture.scheduler.family_for(id),
            Some(SpecializedActorTaskFamily::Intro2CommonDying)
        );
        let entity = fixture.entities.entity_mut(id).unwrap();
        assert_eq!(
            (
                entity.position_raw(),
                entity.model_slots,
                entity.physical_body_basis_q31,
                entity.construction_stamp_at_0xb4,
                entity.intro2_type17_runtime,
                entity.type17_sub_d_frame_owner,
                entity.type17_sub_d_runtime,
            ),
            before,
            "class12's null target preserves the existing native Sub-D owner"
        );
        assert_eq!(
            entity.velocity_raw(),
            [old_velocity[0], 500, old_velocity[2]]
        );
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
        for task in old_tasks.into_iter().flatten() {
            assert_eq!(entity.actor_tasks.wrapper_flags(task), None);
        }
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::CommonDying(task)) if task.elapsed_ms() == 0
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        fixture.fx.process_pending();
        assert_eq!(
            fixture
                .fx
                .take_positional_sounds()
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>(),
            [94]
        );
        assert_eq!(
            fixture.fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type17_abort_noops_preserve_remote_graphs_and_existing_corpse_elapsed() {
    for remote in [true, false] {
        let Some(mut fixture) = fixture_for_world(14) else {
            return;
        };
        let id = id(&fixture, 17);
        if remote {
            let entity = fixture.entities.entity_mut(id).unwrap();
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(REMOTE_OWNED_STATE_BIT, REMOTE_OWNED_STATE_BIT);
            entity.current_behavior_context = RetailRuntimeValue::Unresolved;
            // 10C10's remote return does not require a local task owner or
            // resolve the downstream behavior callback.
            fixture.scheduler = SpecializedActorTaskScheduler::default();
        } else {
            require_success(invoke(&mut fixture, id).0);
            let entity = fixture.entities.entity_mut(id).unwrap();
            let primary = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            let Some(ActorTaskRuntime::CommonDying(task)) =
                entity.actor_tasks.task_state_mut(primary)
            else {
                panic!("published common dying task")
            };
            task.before_callback(123_000);
        }
        fixture.fx.process_pending();
        fixture.fx.take_positional_sounds();
        let entity = fixture.entities.entity_mut(id).unwrap();
        let before_collision = entity.collision.clone();
        let before_context = entity.current_behavior_context;
        let before_tasks = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
            (
                entity.actor_tasks.task_in_slot(slot),
                entity.actor_task_state(slot).cloned(),
            )
        });
        let before_family = fixture.scheduler.family_for(id);
        let mut oracle = fixture.fx.fork_for_main_base_abort_transaction();
        let (result, publications) = invoke(&mut fixture, id);
        assert_eq!(
            require_success(result).disposition,
            MainBaseAbortActorDisposition::Type17Death
        );
        assert_eq!(publications, MainBaseAbortPublicationCounts::default());
        let entity = fixture.entities.entity_mut(id).unwrap();
        assert_eq!(entity.collision, before_collision);
        assert_eq!(entity.current_behavior_context, before_context);
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
                (
                    entity.actor_tasks.task_in_slot(slot),
                    entity.actor_task_state(slot).cloned(),
                )
            }),
            before_tasks
        );
        assert_eq!(fixture.scheduler.family_for(id), before_family);
        assert_eq!(fixture.fx.pending_event_count(), 0);
        assert_eq!(
            fixture.fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type17_abort_rejects_foreign_allocation_even_before_remote_noop() {
    let (Some(mut fixture), Some(foreign)) = (fixture_for_world(14), fixture_for_world(14)) else {
        return;
    };
    let id = id(&fixture, 17);
    let foreign_id = self::id(&foreign, 17);
    let entity = fixture.entities.entity_mut(id).unwrap();
    entity.intro2_type17_runtime = foreign
        .entities
        .iter_all()
        .find(|entity| entity.id == foreign_id)
        .unwrap()
        .intro2_type17_runtime;
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(REMOTE_OWNED_STATE_BIT, REMOTE_OWNED_STATE_BIT);
    let collision = entity.collision.clone();
    let mut oracle = fixture.fx.fork_for_main_base_abort_transaction();
    let (result, publications) = invoke(&mut fixture, id);
    assert!(matches!(
        result,
        Err(DispatchFailure {
            callback_error: Some(MainBaseAbortActorCallbackBlock::Native(
                NativeMainBaseAbortDeathBlock::Type17(
                    crate::intro2_common_dying::Intro2CommonDyingBlock::UnauthenticatedAllocation
                )
            )),
            ..
        })
    ));
    assert_eq!(publications, MainBaseAbortPublicationCounts::default());
    assert_eq!(
        fixture.entities.entity_mut(id).unwrap().collision,
        collision
    );
    assert_eq!(fixture.fx.pending_event_count(), 0);
    assert_eq!(
        fixture.fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type17_abort_requires_completed_current_scheduler_custody() {
    use crate::actor_task_owner::{ActorTaskVisit, PreparedActorTask};
    for defect in ["missing", "executing", "stale", "pending"] {
        let Some(mut fixture) = fixture_for_world(14) else {
            return;
        };
        let id = id(&fixture, 17);
        match defect {
            "missing" => fixture.scheduler = SpecializedActorTaskScheduler::default(),
            "executing" => {
                let entity = fixture.entities.entity_mut(id).unwrap();
                let slot = ActorTaskSlot::IN_RETAIL_TICK_ORDER
                    .into_iter()
                    .find(|slot| entity.actor_tasks.task_in_slot(*slot).is_some())
                    .unwrap();
                let task_id = entity.actor_tasks.task_in_slot(slot).unwrap();
                assert!(entity
                    .actor_tasks
                    .begin_exact_visit_with(ActorTaskVisit { slot, task_id }, |_| ())
                    .is_some());
            }
            "stale" => {
                let entity = fixture.entities.entity_mut(id).unwrap();
                let slot = ActorTaskSlot::IN_RETAIL_TICK_ORDER
                    .into_iter()
                    .find(|slot| entity.actor_task_state(*slot).is_some())
                    .unwrap();
                let state = entity.actor_task_state(slot).unwrap().clone();
                entity
                    .actor_tasks
                    .replace_prepared(slot, PreparedActorTask::new(state));
            }
            "pending" => {
                let entity = fixture.entities.entity_mut(id).unwrap();
                entity
                    .collision
                    .state_flags_at_0x08
                    .overwrite(0x0202_0000, 0x0202_0000);
                entity.collision.callback_scheduler_accumulator_us_at_0x6c =
                    RetailRuntimeValue::Known(0);
                entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
                let owner =
                    crate::intro2_type17::Intro2Type17Owner::adopt(&fixture.entities, id).unwrap();
                let tick = crate::intro2_type17::tick_intro2_type17(
                    &mut fixture.entities,
                    owner,
                    crate::intro2_type17::Intro2Type17Frame {
                        capture_tasks: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler::default(),
                        notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                        resources: &fixture.session.cache,
                        world_fx: &mut fixture.fx,
                        elapsed_micros: 20_000,
                        retail_tick: 1,
                    },
                );
                assert!(
                    matches!(
                        tick.outcome,
                        crate::intro2_type17::Intro2Type17Outcome::Blocked {
                            prefix_committed: true,
                            ..
                        }
                    ),
                    "{:?}",
                    tick.outcome
                );
                fixture
                    .scheduler
                    .register_intro2_type17(tick.retained_owner.unwrap());
            }
            _ => unreachable!(),
        }
        fixture.fx.process_pending();
        fixture.fx.take_positional_sounds();
        let entity = fixture.entities.entity_mut(id).unwrap();
        let before_collision = entity.collision.clone();
        let before_context = entity.current_behavior_context;
        let before_tasks = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
            (
                entity.actor_tasks.task_in_slot(slot),
                entity.actor_task_state(slot).cloned(),
            )
        });
        let mut oracle = fixture.fx.fork_for_main_base_abort_transaction();
        let (result, publications) = invoke(&mut fixture, id);
        assert!(
            matches!(
                result,
                Err(DispatchFailure {
                    callback_error: Some(MainBaseAbortActorCallbackBlock::Native(
                        NativeMainBaseAbortDeathBlock::TaskCustodyUnavailable
                    )),
                    ..
                })
            ),
            "defect={defect}"
        );
        assert_eq!(publications, MainBaseAbortPublicationCounts::default());
        let entity = fixture.entities.entity_mut(id).unwrap();
        assert_eq!(entity.collision, before_collision);
        assert_eq!(entity.current_behavior_context, before_context);
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
                (
                    entity.actor_tasks.task_in_slot(slot),
                    entity.actor_task_state(slot).cloned(),
                )
            }),
            before_tasks
        );
        assert_eq!(fixture.fx.pending_event_count(), 0);
        assert_eq!(
            fixture.fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn later_world_type47_abort_replaces_native_graph_and_preserves_constructor_custody() {
    for world in [14, 15, 31] {
        let Some(mut f) = fixture_for_world(world) else {
            return;
        };
        let id = id(&f, 47);
        assert!(crate::shared_type47::type47_manager_allocation_authenticates(&f.entities, id));
        let entity = f.entities.entity_mut(id).unwrap();
        let receipt = entity.native_type47_construction;
        let sub_d = (
            entity.intro2_type47_sub_d_frame_owner,
            entity.intro2_type47_sub_d_runtime,
        );
        let tasks =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot));
        let mut expected = f.fx.fork_for_main_base_abort_transaction();
        expected.next_shared_retail_random_u16(); // C620 Sub-A suffix, no selector/Sub-D query.
        let (result, publications) = invoke(&mut f, id);
        assert_eq!(
            require_success(result).disposition,
            MainBaseAbortActorDisposition::Type47Death
        );
        assert_eq!(publications.type47_common_dying, 1);
        assert_eq!(
            f.scheduler.family_for(id),
            Some(SpecializedActorTaskFamily::Intro2CommonDying)
        );
        let entity = f.entities.entity_mut(id).unwrap();
        assert_eq!(entity.native_type47_construction, receipt);
        assert_eq!(
            (
                entity.intro2_type47_sub_d_frame_owner,
                entity.intro2_type47_sub_d_runtime
            ),
            sub_d
        );
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        for task in tasks.into_iter().flatten() {
            assert_eq!(entity.actor_tasks.wrapper_flags(task), None);
        }
        assert!(
            matches!(entity.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::CommonDying(task)) if task.elapsed_ms() == 0)
        );
        f.fx.process_pending();
        assert_eq!(
            f.fx.take_positional_sounds()
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>(),
            [75]
        );
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
        let primary = f
            .entities
            .entity_mut(id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary);
        let (result, publications) = invoke(&mut f, id);
        require_success(result);
        assert_eq!(publications.specialized_total(), 0);
        assert_eq!(
            f.entities
                .entity_mut(id)
                .unwrap()
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary),
            primary
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type47_abort_rejects_foreign_receipt_before_remote_noop_and_pending_graph() {
    for foreign in [true, false] {
        let Some(mut f) = fixture_for_world(14) else {
            return;
        };
        let id = id(&f, 47);
        if foreign {
            let other = fixture_for_world(14).unwrap();
            let other_id = self::id(&other, 47);
            let receipt = other
                .entities
                .iter_all()
                .find(|entity| entity.id == other_id)
                .unwrap()
                .native_type47_construction;
            let entity = f.entities.entity_mut(id).unwrap();
            entity.native_type47_construction = receipt;
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(REMOTE_OWNED_STATE_BIT, REMOTE_OWNED_STATE_BIT);
        } else {
            f.scheduler.park_native_type47_external_prefix(id);
        }
        let before = f.entities.entity_mut(id).unwrap().collision.clone();
        let mut expected = f.fx.fork_for_main_base_abort_transaction();
        let (result, publications) = invoke(&mut f, id);
        assert!(matches!(
            result,
            Err(DispatchFailure {
                callback_error: Some(MainBaseAbortActorCallbackBlock::Native(_)),
                ..
            })
        ));
        assert_eq!(publications.specialized_total(), 0);
        assert_eq!(f.entities.entity_mut(id).unwrap().collision, before);
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn later_world_type53_abort_replaces_native_graph_and_preserves_constructor_custody() {
    // Worlds 14 and 15 author three Type53 actors each; no other ordinary
    // world does. Their construction, selected graph and Sub-D owner come
    // from the actual Section13 rows.
    for world in [14, 15] {
        let Some(mut f) = fixture_for_world(world) else {
            return;
        };
        assert_eq!(f.scheduler.adopt_intro2_type53(&f.entities), 3);
        let id = id(&f, 53);
        assert!(crate::intro2_type53::type53_manager_allocation_authenticates(&f.entities, id));
        let entity = f.entities.entity_mut(id).unwrap();
        let receipt = entity.intro2_type53_runtime;
        let sub_d = entity
            .intro2_type53_runtime
            .map(|runtime| (runtime.sub_d_owner, runtime.sub_d_runtime));
        let tasks =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot));
        let before_position = entity.position_raw();
        let mut expected = f.fx.fork_for_main_base_abort_transaction();
        expected.next_shared_retail_random_u16(); // C620 Sub-A suffix, no selector/Sub-D query.
        let (result, publications) = invoke(&mut f, id);
        assert_eq!(
            require_success(result).disposition,
            MainBaseAbortActorDisposition::Type53Death
        );
        assert_eq!(publications.type53_common_dying, 1);
        assert_eq!(
            f.scheduler.family_for(id),
            Some(SpecializedActorTaskFamily::Intro2CommonDying)
        );
        let entity = f.entities.entity_mut(id).unwrap();
        assert_eq!(entity.intro2_type53_runtime, receipt);
        assert_eq!(
            entity
                .intro2_type53_runtime
                .map(|runtime| (runtime.sub_d_owner, runtime.sub_d_runtime,)),
            sub_d
        );
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        for task in tasks.into_iter().flatten() {
            assert_eq!(entity.actor_tasks.wrapper_flags(task), None);
        }
        assert!(
            matches!(entity.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::CommonDying(task)) if task.elapsed_ms() == 0)
        );
        f.fx.process_pending();
        let expected_sound = match f
            .entities
            .type_runtime_metadata(53)
            .expect("Type53 metadata")
            .death_sound_id
        {
            RetailRuntimeValue::Known(Some(sound)) => Some(usize::from(sound)),
            RetailRuntimeValue::Known(None) => None,
            RetailRuntimeValue::Unresolved => panic!("Type53 death sound"),
        };
        assert_eq!(
            f.fx.take_positional_sounds()
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>(),
            expected_sound.into_iter().collect::<Vec<_>>()
        );
        let _ = before_position;
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
        let primary = f
            .entities
            .entity_mut(id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary);
        let (result, publications) = invoke(&mut f, id);
        require_success(result);
        assert_eq!(publications.specialized_total(), 0);
        assert_eq!(
            f.entities
                .entity_mut(id)
                .unwrap()
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary),
            primary
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type53_abort_rejects_foreign_receipt_and_missing_custody_before_writes() {
    // Foreign allocation: authentication precedes the remote-owned no-op, so a
    // transplanted receipt fails closed even with the remote bit set.
    let (Some(mut f), Some(other)) = (fixture_for_world(14), fixture_for_world(14)) else {
        return;
    };
    f.scheduler.adopt_intro2_type53(&f.entities);
    let actor_id = id(&f, 53);
    let other_id = self::id(&other, 53);
    let receipt = other
        .entities
        .iter_all()
        .find(|entity| entity.id == other_id)
        .unwrap()
        .intro2_type53_runtime;
    f.entities
        .entity_mut(actor_id)
        .unwrap()
        .intro2_type53_runtime = receipt;
    f.entities
        .entity_mut(actor_id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(REMOTE_OWNED_STATE_BIT, REMOTE_OWNED_STATE_BIT);
    let before = f.entities.entity_mut(actor_id).unwrap().collision.clone();
    let mut oracle = f.fx.fork_for_main_base_abort_transaction();
    let (result, publications) = invoke(&mut f, actor_id);
    assert!(matches!(
        result,
        Err(DispatchFailure {
            callback_error: Some(MainBaseAbortActorCallbackBlock::Native(
                NativeMainBaseAbortDeathBlock::Type53(
                    crate::intro2_common_dying::Intro2CommonDyingBlock::UnauthenticatedAllocation
                )
            )),
            ..
        })
    ));
    assert_eq!(publications, MainBaseAbortPublicationCounts::default());
    assert_eq!(f.entities.entity_mut(actor_id).unwrap().collision, before);
    assert!(f.fx.take_positional_sounds().is_empty());
    assert_eq!(f.fx.pending_event_count(), 0);
    assert_eq!(
        f.fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );

    // Missing scheduler custody: the shared fixture adopts every native family
    // except Type53, so the abort must block before any death write.
    let Some(mut missing) = fixture_for_world(14) else {
        return;
    };
    let missing_id = self::id(&missing, 53);
    let before = missing
        .entities
        .entity_mut(missing_id)
        .unwrap()
        .collision
        .clone();
    let mut oracle = missing.fx.fork_for_main_base_abort_transaction();
    let (result, publications) = invoke(&mut missing, missing_id);
    assert!(matches!(
        result,
        Err(DispatchFailure {
            callback_error: Some(MainBaseAbortActorCallbackBlock::Native(
                NativeMainBaseAbortDeathBlock::TaskCustodyUnavailable
            )),
            ..
        })
    ));
    assert_eq!(publications, MainBaseAbortPublicationCounts::default());
    assert_eq!(
        missing.entities.entity_mut(missing_id).unwrap().collision,
        before
    );
    assert!(missing.fx.take_positional_sounds().is_empty());
    assert_eq!(missing.fx.pending_event_count(), 0);
    assert_eq!(
        missing.fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
}
#[v2k_test_support::retail_test]
fn later_world_type58_abort_replaces_native_graph_and_preserves_constructor_custody() {
    // The actual later-world Section13 actors retain their own constructor receipts.
    for world in [14, 24, 31] {
        let Some(mut f) = fixture_for_world(world) else {
            return;
        };
        assert!(f.scheduler.adopt_intro2_type58(&f.entities) > 0);
        let id = id(&f, 58);
        assert!(crate::intro2_type58::type58_manager_allocation_authenticates(&f.entities, id));
        let entity = f.entities.entity_mut(id).unwrap();
        let receipt = entity.intro2_type58_runtime;
        let sub_d = entity
            .intro2_type58_runtime
            .map(|runtime| (runtime.sub_d_owner, runtime.sub_d_runtime));
        let tasks =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot));
        let before_position = entity.position_raw();
        let mut expected = f.fx.fork_for_main_base_abort_transaction();
        expected.next_shared_retail_random_u16(); // C620 Sub-A suffix, no selector/Sub-D query.
        let (result, publications) = invoke(&mut f, id);
        assert_eq!(
            require_success(result).disposition,
            MainBaseAbortActorDisposition::Type58Death
        );
        assert_eq!(publications.type58_common_dying, 1);
        assert_eq!(
            f.scheduler.family_for(id),
            Some(SpecializedActorTaskFamily::Intro2CommonDying)
        );
        let entity = f.entities.entity_mut(id).unwrap();
        assert_eq!(entity.intro2_type58_runtime, receipt);
        assert_eq!(
            entity
                .intro2_type58_runtime
                .map(|runtime| (runtime.sub_d_owner, runtime.sub_d_runtime,)),
            sub_d
        );
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        for task in tasks.into_iter().flatten() {
            assert_eq!(entity.actor_tasks.wrapper_flags(task), None);
        }
        assert!(
            matches!(entity.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::CommonDying(task)) if task.elapsed_ms() == 0)
        );
        f.fx.process_pending();
        let expected_sound = match f
            .entities
            .type_runtime_metadata(58)
            .expect("Type58 metadata")
            .death_sound_id
        {
            RetailRuntimeValue::Known(Some(sound)) => Some(usize::from(sound)),
            RetailRuntimeValue::Known(None) => None,
            RetailRuntimeValue::Unresolved => panic!("Type58 death sound"),
        };
        assert_eq!(
            f.fx.take_positional_sounds()
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>(),
            expected_sound.into_iter().collect::<Vec<_>>()
        );
        let _ = before_position;
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
        let primary = f
            .entities
            .entity_mut(id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary);
        let (result, publications) = invoke(&mut f, id);
        require_success(result);
        assert_eq!(publications.specialized_total(), 0);
        assert_eq!(
            f.entities
                .entity_mut(id)
                .unwrap()
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary),
            primary
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type58_abort_rejects_foreign_receipt_and_missing_custody_before_writes() {
    // Foreign allocation: authentication precedes the remote-owned no-op, so a
    // transplanted receipt fails closed even with the remote bit set.
    let (Some(mut f), Some(other)) = (fixture_for_world(14), fixture_for_world(14)) else {
        return;
    };
    f.scheduler.adopt_intro2_type58(&f.entities);
    let actor_id = id(&f, 58);
    let other_id = self::id(&other, 58);
    let receipt = other
        .entities
        .iter_all()
        .find(|entity| entity.id == other_id)
        .unwrap()
        .intro2_type58_runtime;
    f.entities
        .entity_mut(actor_id)
        .unwrap()
        .intro2_type58_runtime = receipt;
    f.entities
        .entity_mut(actor_id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(REMOTE_OWNED_STATE_BIT, REMOTE_OWNED_STATE_BIT);
    let before = f.entities.entity_mut(actor_id).unwrap().collision.clone();
    let mut oracle = f.fx.fork_for_main_base_abort_transaction();
    let (result, publications) = invoke(&mut f, actor_id);
    assert!(matches!(
        result,
        Err(DispatchFailure {
            callback_error: Some(MainBaseAbortActorCallbackBlock::Native(
                NativeMainBaseAbortDeathBlock::Type58(
                    crate::intro2_common_dying::Intro2CommonDyingBlock::UnauthenticatedAllocation
                )
            )),
            ..
        })
    ));
    assert_eq!(publications, MainBaseAbortPublicationCounts::default());
    assert_eq!(f.entities.entity_mut(actor_id).unwrap().collision, before);
    assert!(f.fx.take_positional_sounds().is_empty());
    assert_eq!(f.fx.pending_event_count(), 0);
    assert_eq!(
        f.fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );

    // Missing scheduler custody: the shared fixture adopts every native family
    // except Type58, so the abort must block before any death write.
    let Some(mut missing) = fixture_for_world(14) else {
        return;
    };
    let missing_id = self::id(&missing, 58);
    let before = missing
        .entities
        .entity_mut(missing_id)
        .unwrap()
        .collision
        .clone();
    let mut oracle = missing.fx.fork_for_main_base_abort_transaction();
    let (result, publications) = invoke(&mut missing, missing_id);
    assert!(matches!(
        result,
        Err(DispatchFailure {
            callback_error: Some(MainBaseAbortActorCallbackBlock::Native(
                NativeMainBaseAbortDeathBlock::TaskCustodyUnavailable
            )),
            ..
        })
    ));
    assert_eq!(publications, MainBaseAbortPublicationCounts::default());
    assert_eq!(
        missing.entities.entity_mut(missing_id).unwrap().collision,
        before
    );
    assert!(missing.fx.take_positional_sounds().is_empty());
    assert_eq!(missing.fx.pending_event_count(), 0);
    assert_eq!(
        missing.fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
}
