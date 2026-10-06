use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    entity::{DynamicRadialLiveOutcome, DynamicRadialLivePhase, DynamicRadialLiveRequest, Entity},
    intro2_radial::{
        apply_intro2_radial_damage, Intro2RadialFrame, Intro2RadialReport, Intro2RadialTaskCustody,
    },
    intro2_type47_live::world::native_intro2_fixture,
    radial_damage::RadialDamageTemplate,
    session::GameSession,
    static_damage::StaticDamageScheduler,
};

const BLAST: RadialDamageTemplate = RadialDamageTemplate {
    inner_radius_raw: 8,
    outer_radius_raw: 16,
    impulse_raw: 128,
    packet: DamagePacket {
        channels: [2, 0],
        amounts_raw: [1_000, 0],
    },
    trailing_raw: [-1, 0],
};

fn fixture() -> Option<(
    GameSession,
    EntityManager,
    SpecializedActorTaskScheduler,
    u32,
)> {
    let (session, mut manager, _) = native_intro2_fixture()?;
    let id = manager
        .iter_all()
        .find(|entity| {
            entity.entity_type == 9
                && matches!(entity.current_behavior_context,
            RetailRuntimeValue::Known(Some(context))
            if context.active_style().audited().is_some_and(|style| style.class_id == 10))
        })
        .unwrap()
        .id;
    manager.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(1_000_000);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_type9(&mut manager), 13);
    // Keep the complete native manager for actual target resolution; isolate
    // only which callback the test asks the scheduler to advance.
    scheduler.owners.retain(|owner| owner.entity_id() == id);
    Some((session, manager, scheduler, id))
}

fn tasks(entity: &Entity) -> [Option<ActorTaskRuntime>; 3] {
    ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).cloned())
}

fn origin(entity: &Entity) -> [i16; 3] {
    let [x, y, z] = entity.position_raw();
    [x.wrapping_sub(8), y, z]
}

fn transaction(
    scheduler: &SpecializedActorTaskScheduler,
    manager: &crate::entity::EntityManager,
    id: u32,
) -> u64 {
    let mut shadow = SpecializedActorTaskScheduler {
        hive_diagnostics: Vec::new(),
        terminal_abort_origins: Vec::new(),
        owners: scheduler
            .owners
            .iter()
            .map(SpecializedActorTaskOwner::fork_for_main_base_abort_transaction)
            .collect(),
    };
    let Some(NativeType9HitCustody::Selected {
        next_transaction_id,
        ..
    }) = shadow.begin_native_type9_external_mutation(manager, id)
    else {
        panic!("expected completed native selected owner")
    };
    next_transaction_id
}

fn tick(
    session: &mut GameSession,
    manager: &mut EntityManager,
    scheduler: &mut SpecializedActorTaskScheduler,
    fx: &mut WorldFx,
    tick: u32,
) -> SpecializedActorTaskProductionPass {
    scheduler.tick(
        manager,
        SpecializedActorTaskProductionFrame {
            world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase:
                crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
            resources: &mut session.cache,
            world_fx: fx,
            static_damage: &mut StaticDamageScheduler::new(),
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            retail_tick: tick,
            main_base_abort_active: false,
        },
        &mut GameplayNotifications::new(),
    )
}

fn radial(
    manager: &mut EntityManager,
    scheduler: &mut SpecializedActorTaskScheduler,
    fx: &mut WorldFx,
    id: u32,
) -> DynamicRadialLiveOutcome {
    let origin_raw = origin(manager.entity_mut(id).unwrap());
    manager.apply_dynamic_radial_damage_live(DynamicRadialLiveRequest {
        origin_raw,
        template: BLAST,
        world_fx: fx,
        retail_tick: 51,
        notifications: &mut GameplayNotifications::new(),
        callbacks: &mut |manager: &crate::entity::EntityManager, id: u32| {
            scheduler.prepare_native_type9_external_mutation(manager, id)
        },
    })
}

#[v2k_test_support::retail_test]
fn native_type9_radial_survivor_and_repeated_hits_preserve_clocks_and_resume() {
    let Some((mut session, mut manager, mut scheduler, id)) = fixture() else {
        return;
    };
    let mut fx = WorldFx::new();
    let pass = tick(&mut session, &mut manager, &mut scheduler, &mut fx, 50);
    assert_eq!(pass.block, None);
    assert_eq!(scheduler.owners.len(), 1, "{pass:?}");
    let SpecializedActorTaskOwner::OrdinaryType9RunAway(owner) = &scheduler.owners[0] else {
        panic!("{pass:?}")
    };
    assert!(matches!(owner.state(), crate::ordinary_type9_run_away_production::OrdinaryType9RunAwayProductionState::PostBasisTailPending { .. }), "the regression needs an actual completed outer observation");
    let next_transaction = transaction(&scheduler, &manager, id);
    let entity = manager.entity_mut(id).unwrap();
    let before = (
        tasks(entity),
        entity.position_raw(),
        entity.current_behavior_context,
        entity.collision.callback_scheduler_accumulator_us_at_0x6c,
        entity.collision.last_hit_presentation_tick_at_0x34,
        entity
            .ordinary_type9_selected_component_runtime
            .unwrap()
            .components()
            .sub_d_frame_owner
            .classifier_cache()
            .stagger_counter(),
    );
    let before_velocity = entity.velocity_raw();
    let before_health = entity.collision.health_raw;
    let mut control_rng = fx.fork_for_main_base_abort_transaction();
    for count in 1..=2 {
        let result = radial(&mut manager, &mut scheduler, &mut fx, id);
        assert_eq!(result.blocked, None, "{result:?}");
        assert_eq!(result.completed_target_ids, [id]);
        assert!(result.death_publications.is_empty());
        assert_eq!(transaction(&scheduler, &manager, id), next_transaction);
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(
            (
                tasks(entity),
                entity.position_raw(),
                entity.current_behavior_context,
                entity.collision.callback_scheduler_accumulator_us_at_0x6c,
                entity.collision.last_hit_presentation_tick_at_0x34,
                entity
                    .ordinary_type9_selected_component_runtime
                    .unwrap()
                    .components()
                    .sub_d_frame_owner
                    .classifier_cache()
                    .stagger_counter()
            ),
            before
        );
        assert_eq!(
            entity.velocity_raw(),
            [
                before_velocity[0].wrapping_add(128 * count),
                before_velocity[1],
                before_velocity[2]
            ]
        );
        assert_ne!(entity.collision.health_raw, before_health);
    }
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        control_rng.next_shared_retail_random_u16(),
        "radial neither runs particle jitter nor actor callbacks"
    );
    let pass = tick(&mut session, &mut manager, &mut scheduler, &mut fx, 52);
    assert_eq!(pass.block, None);
    assert_eq!(scheduler.owners.len(), 1, "{pass:?}");
    assert!(
        scheduler.prepare_native_type9_external_mutation(&manager, id),
        "surviving actor must finish another visit: {pass:?}"
    );
    assert_ne!(manager.entity_mut(id).unwrap().position_raw(), before.1);
}

#[v2k_test_support::retail_test]
fn native_type66_radial_custody_rejects_pending_stale_and_missing_owners_without_mutation() {
    for already_visited in [false, true] {
        for case in 0..3 {
            let Some((_session, mut manager, _)) = native_intro2_fixture() else {
                return;
            };
            let id = manager
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(51))
                .unwrap()
                .id;
            let mut scheduler = SpecializedActorTaskScheduler::new();
            scheduler.adopt_intro2_type66(&manager);
            scheduler.owners.retain(|owner| owner.entity_id() == id);
            match case {
                0 => scheduler.register_intro2_type66(
                    crate::intro2_type66::Intro2Type66Owner::adopt_blocked_prefix(&manager, id)
                        .unwrap(),
                ),
                1 => {
                    let entity = manager.entity_mut(id).unwrap();
                    let runtime = *entity.actor_task_state(ActorTaskSlot::Primary).unwrap();
                    entity.actor_tasks.replace_prepared(
                        ActorTaskSlot::Primary,
                        crate::actor_task_owner::PreparedActorTask::new(runtime),
                    );
                }
                2 => scheduler.owners.clear(),
                _ => unreachable!(),
            }
            let entity = manager.entity_mut(id).unwrap();
            entity.collision.health_raw = RetailRuntimeValue::Known(1);
            let origin_raw = entity.position_raw();
            let before = (
                entity.collision.clone(),
                entity.base_factory_runtime,
                entity.current_behavior_context,
                tasks(entity),
            );
            let expected_owners = scheduler
                .owners
                .iter()
                .map(SpecializedActorTaskOwner::entity_id)
                .collect::<Vec<_>>();
            let mut pending = Vec::new();
            let mut retained = Vec::new();
            if already_visited {
                retained = scheduler.owners;
            } else {
                pending = scheduler.owners;
            }
            let remaining = if already_visited { vec![] } else { vec![id] };
            let mut cursor = Intro2RadialCursorCustody {
                pending: &mut pending,
                retained: &mut retained,
                remaining_live_ids: &remaining,
            };
            let mut fx = WorldFx::new();
            let result = manager.apply_dynamic_radial_damage_live(DynamicRadialLiveRequest {
                origin_raw,
                template: BLAST,
                world_fx: &mut fx,
                retail_tick: 51,
                notifications: &mut GameplayNotifications::new(),
                callbacks: &mut |manager: &crate::entity::EntityManager, id: u32| {
                    cursor.prepare_native_actor_mutation(manager, id)
                },
            });
            assert!(
                matches!(result.blocked, Some(crate::entity::DynamicRadialLiveBlock {
                target_id, phase: DynamicRadialLivePhase::MutationCustody, target_prefix_committed: false,
                reason: crate::entity::DynamicRadialLiveBlockReason::NativeActorMutationCustody,
            }) if target_id == id),
                "visited={already_visited} case={case}: {result:?}"
            );
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(
                (
                    entity.collision.clone(),
                    entity.base_factory_runtime,
                    entity.current_behavior_context,
                    tasks(entity)
                ),
                before
            );
            assert_eq!(
                pending
                    .iter()
                    .chain(retained.iter())
                    .map(SpecializedActorTaskOwner::entity_id)
                    .collect::<Vec<_>>(),
                expected_owners
            );
            assert!(result.death_publications.is_empty());
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                WorldFx::new().next_shared_retail_random_u16()
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type66_radial_lethal_replacement_is_adopted_before_a_second_radial_call() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(51))
        .unwrap()
        .id;
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type66(&manager);
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.health_raw = RetailRuntimeValue::Known(1);
    let origin_raw = entity.position_raw();
    let previous = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
    let mut fx = WorldFx::new();
    let mut expected = WorldFx::new();
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut primary = None;
    for visit in 0..2 {
        let report = apply_intro2_radial_damage(
            &mut Intro2RadialFrame {
                active_terminal_calls: Vec::new(),
                entities: &mut manager,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                notifications: &mut notifications,
                retail_tick: 51 + visit,
                actor_tasks: &mut scheduler,
            },
            origin_raw,
            BLAST,
        );
        let Intro2RadialReport::Applied { dynamic, .. } = report else {
            panic!("{report:?}")
        };
        assert!(dynamic.completed(), "{dynamic:?}");
        assert_eq!(dynamic.completed_target_ids, [id]);
        assert_eq!(dynamic.death_publications.len(), usize::from(visit == 0));
        assert!(scheduler.prepare_native_actor_mutation(&manager, id));
        let entity = manager.entity_mut(id).unwrap();
        if visit == 0 {
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(10_000_000)
            );
            primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
            assert_ne!(primary, previous);
            expected.next_shared_retail_random_u16();
        } else {
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(9_999_200)
            );
            assert_eq!(
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                primary
            );
        }
    }
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type9_radial_rejects_pending_owner_without_replacing_it_or_writing() {
    let Some((mut session, mut manager, mut scheduler, id)) = fixture() else {
        return;
    };
    // The task/mover and F70 publish before the outer owner reads the live
    // +48 surface timer. Removing that consumed field parks the actual tail;
    // the native task's immutable axis descriptor remains fully authenticated.
    let entity = manager.entity_mut(id).unwrap();
    let before_tasks = tasks(entity);
    entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Unresolved;
    let mut fx = WorldFx::new();
    let pass = tick(&mut session, &mut manager, &mut scheduler, &mut fx, 50);
    assert_eq!(pass.block, None);
    assert_eq!(scheduler.owners.len(), 1, "{pass:?}");
    assert!(matches!(pass.outcomes.as_slice(),
        [SpecializedActorTaskProductionOutcome::OrdinaryType9RunAway(
            crate::ordinary_type9_run_away_production::OrdinaryType9RunAwayProductionOutcome::Blocked {
                reason: crate::ordinary_type9_run_away_production::OrdinaryType9RunAwayProductionBlock::OuterOwnerStateUnavailable,
                ..
            }
        )]), "{pass:?}");
    assert_ne!(
        tasks(manager.entity_mut(id).unwrap()),
        before_tasks,
        "the callback must commit its task prefix before the outer-tail block"
    );
    assert!(
        !scheduler.prepare_native_type9_external_mutation(&manager, id),
        "{pass:?}"
    );
    let owner_before = format!("{:?}", scheduler.owners[0]);
    let entity = manager.entity_mut(id).unwrap();
    let before = (
        entity.collision.clone(),
        entity.velocity_raw(),
        tasks(entity),
    );
    let mut control_rng = fx.fork_for_main_base_abort_transaction();
    let result = radial(&mut manager, &mut scheduler, &mut fx, id);
    assert_eq!(
        result
            .blocked
            .as_ref()
            .map(|block| (block.phase, block.target_prefix_committed)),
        Some((DynamicRadialLivePhase::MutationCustody, false)),
        "{result:?}"
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        (
            entity.collision.clone(),
            entity.velocity_raw(),
            tasks(entity)
        ),
        before
    );
    assert_eq!(format!("{:?}", scheduler.owners[0]), owner_before);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        control_rng.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type9_radial_block_after_impulse_keeps_consumed_custody() {
    let Some((mut session, mut manager, mut scheduler, id)) = fixture() else {
        return;
    };
    let mut fx = WorldFx::new();
    tick(&mut session, &mut manager, &mut scheduler, &mut fx, 50);
    let next_transaction = transaction(&scheduler, &manager, id);
    let entity = manager.entity_mut(id).unwrap();
    let velocity = entity.velocity_raw();
    let health = entity.collision.health_raw;
    entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Unresolved;
    let result = radial(&mut manager, &mut scheduler, &mut fx, id);
    assert_eq!(
        result
            .blocked
            .as_ref()
            .map(|block| (block.phase, block.target_prefix_committed)),
        Some((DynamicRadialLivePhase::Modifier, true)),
        "{result:?}"
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity.velocity_raw(),
        [velocity[0].wrapping_add(128), velocity[1], velocity[2]]
    );
    assert_eq!(entity.collision.health_raw, health);
    assert_eq!(transaction(&scheduler, &manager, id), next_transaction);
}

#[v2k_test_support::retail_test]
fn native_type9_radial_cursor_preserves_visited_and_unvisited_owner_positions() {
    let Some((_session, mut manager, scheduler, id)) = fixture() else {
        return;
    };
    for already_visited in [false, true] {
        let original = scheduler.owners[0].fork_for_main_base_abort_transaction();
        let mut pending = Vec::new();
        let mut retained = Vec::new();
        if already_visited {
            retained.push(original);
        } else {
            pending.push(original);
        }
        let remaining = if already_visited {
            Vec::new()
        } else {
            vec![id]
        };
        let mut cursor = Intro2RadialCursorCustody {
            pending: &mut pending,
            retained: &mut retained,
            remaining_live_ids: &remaining,
        };
        let before = tasks(manager.entity_mut(id).unwrap());
        let origin_raw = origin(manager.entity_mut(id).unwrap());
        let velocity = manager.entity_mut(id).unwrap().velocity_raw();
        let result = manager.apply_dynamic_radial_damage_live(DynamicRadialLiveRequest {
            origin_raw,
            template: BLAST,
            world_fx: &mut WorldFx::new(),
            retail_tick: 51,
            notifications: &mut GameplayNotifications::new(),
            callbacks: &mut |manager: &crate::entity::EntityManager, id: u32| {
                cursor.prepare_native_actor_mutation(manager, id)
            },
        });
        assert_eq!(result.blocked, None, "{result:?}");
        assert_eq!(result.completed_target_ids, [id]);
        assert_eq!(
            manager.entity_mut(id).unwrap().velocity_raw(),
            [velocity[0].wrapping_add(128), velocity[1], velocity[2]]
        );
        assert_eq!(tasks(manager.entity_mut(id).unwrap()), before);
        assert_eq!(
            pending
                .iter()
                .map(SpecializedActorTaskOwner::entity_id)
                .collect::<Vec<_>>(),
            if already_visited { vec![] } else { vec![id] }
        );
        assert_eq!(
            retained
                .iter()
                .map(SpecializedActorTaskOwner::entity_id)
                .collect::<Vec<_>>(),
            if already_visited { vec![id] } else { vec![] }
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type9_radial_cursor_adopts_death_before_a_second_blast() {
    for already_visited in [false, true] {
        let Some((mut session, mut manager, mut scheduler, id)) = fixture() else {
            return;
        };
        let mut pending = Vec::new();
        let mut retained = Vec::new();
        let owner = scheduler.owners.remove(0);
        if already_visited {
            retained.push(owner);
        } else {
            pending.push(owner);
        }
        let remaining = if already_visited {
            Vec::new()
        } else {
            vec![id]
        };
        let origin_raw = origin(manager.entity_mut(id).unwrap());
        let mut fx = WorldFx::new();
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        for _ in 0..2 {
            let result = apply_intro2_radial_damage(
                &mut Intro2RadialFrame {
                    active_terminal_calls: Vec::new(),
                    entities: &mut manager,
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    static_damage: &mut static_damage,
                    notifications: &mut notifications,
                    retail_tick: 51,
                    actor_tasks: &mut Intro2RadialCursorCustody {
                        pending: &mut pending,
                        retained: &mut retained,
                        remaining_live_ids: &remaining,
                    },
                },
                origin_raw,
                RadialDamageTemplate {
                    packet: DamagePacket {
                        channels: [2, 0],
                        amounts_raw: [2_000_000, 0],
                    },
                    ..BLAST
                },
            );
            let Intro2RadialReport::Applied { dynamic, .. } = result else {
                panic!("{result:?}")
            };
            assert_eq!(dynamic.blocked, None, "{dynamic:?}");
            let owners = if already_visited { &retained } else { &pending };
            assert_eq!(owners.len(), 1);
            assert_eq!(
                owners[0].family(),
                SpecializedActorTaskFamily::Intro2Type9Class14
            );
            assert!(if already_visited {
                pending.is_empty()
            } else {
                retained.is_empty()
            });
        }
    }
}
