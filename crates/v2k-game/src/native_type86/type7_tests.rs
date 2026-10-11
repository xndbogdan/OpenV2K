//! Canonical diver allocations use the complete four-choice program, with
//! their own query-free steering, worker economics, sound and relation policy.
use super::*;
use crate::{
    entity_relation_release::relation_attach_state_word_after,
    entity_scheduler::{
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    },
    native_actor_attachment::NativeActorAttachOutcome,
    native_type122::construction_tests::native_fixture_with_player,
    ordinary_type9_cargo::Type9CargoReleasePosition,
    session::GameSession,
};

fn actor(manager: &EntityManager, id: u32) -> &Entity {
    manager.iter_all().find(|entity| entity.id == id).unwrap()
}

fn fixture(level: u32) -> (GameSession, EntityManager, WorldFx, u32) {
    let (session, mut manager, mut fx) = native_fixture_with_player(level);
    manager.cleanup_pending_actor_deferred_destroys();
    fx.process_pending();
    fx.take_positional_sounds();
    let id = manager.iter_all().find(|e| e.entity_type == 7).unwrap().id;
    (session, manager, fx, id)
}

fn detailed(entity: &mut Entity, visible: bool) {
    let flags =
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT;
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(flags | 0x800, flags | if visible { 0x800 } else { 0 });
    entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
}

/// Only candidate poses/staffing are controlled. Every body, component and
/// predicate still belongs to its actual world34 allocation.
fn isolate(manager: &mut EntityManager, id: u32) {
    let position = actor(manager, id).position_raw();
    let ids = manager.iter_all().map(|e| e.id).collect::<Vec<_>>();
    for other in ids {
        if other != id {
            manager.entity_mut(other).unwrap().set_position_raw([
                position[0].wrapping_add(16000),
                position[1],
                position[2].wrapping_add(16000),
            ]);
        }
    }
}

fn near(manager: &mut EntityManager, id: u32, candidate: u32) {
    let [x, y, z] = actor(manager, id).position_raw();
    manager
        .entity_mut(candidate)
        .unwrap()
        .set_position_raw([x.wrapping_add(1024), y, z]);
}

fn choose(manager: &mut EntityManager, fx: &mut WorldFx, id: u32, kind: TaskKind) -> Type86Owner {
    for _ in 0..512 {
        let owner = birth::reselect(manager, id, fx).unwrap();
        if owner.kind == kind {
            return owner;
        }
    }
    panic!("real Type7 candidate set never selected {kind:?}");
}

fn run_task(
    session: &GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    owner: &mut Type86Owner,
    dt: u32,
    mode: i32,
) {
    let metadata = manager.type_runtime_metadata(7).unwrap().clone();
    assert!(!task::run_task(
        manager,
        owner,
        &task::TaskFrame {
            metadata: &metadata,
            terrain: session.cache.level_terrain().unwrap(),
            elapsed_micros: dt,
            global_elapsed_micros: dt,
            scheduler_mode: mode,
        },
        fx
    )
    .unwrap());
}

#[v2k_test_support::retail_test]
fn type7_exact_profile_rejects_people_worker_and_querying_descriptor_aliases() {
    let (session, _, _, _) = fixture(34);
    let metadata =
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(7).unwrap());
    birth::validate_metadata(NativeFourChoiceProfile::DiverWorker, &metadata).unwrap();
    assert_eq!(NativePersonProfile::from_entity_type(7), None);
    assert_eq!(
        crate::intro2_type8::NativeWorkerProfile::from_entity_type(7),
        None
    );
    assert_eq!(metadata.model_slots, [1249; 4]);
    assert_eq!(
        (
            metadata.mass_raw,
            metadata.capability_flags,
            metadata.initial_health_raw
        ),
        (10, 0x1404, Some(1500))
    );
    let spec = metadata.initializer.as_ref().unwrap();
    assert_eq!(
        (
            spec.common_axis_descriptor.strict_axis_limit_raw,
            spec.common_axis_descriptor.raw_word_at_0x04
        ),
        (3072, 0xa1)
    );
    assert_eq!(
        spec.behavior_choices
            .iter()
            .map(|choice| (
                choice.weight_rule_id,
                choice.weight_multiplier,
                choice.behavior_class_id
            ))
            .collect::<Vec<_>>(),
        [(1, 1, 6), (13, 200, 54), (6, 20, 45), (7, 10, 10)]
    );
    let RetailRuntimeValue::Known(Some(d)) = metadata.sub_d_steering_descriptor else {
        panic!()
    };
    assert_eq!(
        (
            d.steering_divisor_raw,
            d.forward_probe_raw,
            d.lateral_probe_raw,
            d.classifier_flags
        ),
        (32, 512, 256, 0)
    );
    let RetailRuntimeValue::Known(Some(i)) = metadata.actor_animation_descriptor else {
        panic!()
    };
    assert_eq!(
        (
            i.capability_bit_3_sound_id,
            i.capability_mask_0x201_sound_id,
            i.attention_stop_sound_id
        ),
        (0, 0, 106)
    );
    let damage = metadata.damage_profile.unwrap();
    assert_eq!(damage.thresholds_raw, [0, 4000, 200, 0, 200, 0, 0]);
    assert_eq!(damage.multipliers_q8, [0, 256, 256, 512, 128, 0, 256]);
    assert_eq!(metadata.death_sound_id, RetailRuntimeValue::Known(Some(35)));
    assert!(birth::validate_metadata(
        NativeFourChoiceProfile::Person(NativePersonProfile::Type95),
        &metadata
    )
    .is_err());
    let mut wrong = metadata.clone();
    wrong.initializer.as_mut().unwrap().behavior_choices[1].weight_rule_id = 12;
    assert!(birth::validate_metadata(NativeFourChoiceProfile::DiverWorker, &wrong).is_err());
    let mut wrong = metadata.clone();
    let mut d = d;
    d.classifier_flags = 17;
    wrong.sub_d_steering_descriptor = RetailRuntimeValue::Known(Some(d));
    assert!(birth::validate_metadata(NativeFourChoiceProfile::DiverWorker, &wrong).is_err());
    let mut wrong = metadata;
    let mut i = i;
    i.attention_stop_sound_id = 72;
    wrong.actor_animation_descriptor = RetailRuntimeValue::Known(Some(i));
    assert!(birth::validate_metadata(NativeFourChoiceProfile::DiverWorker, &wrong).is_err());
}

#[v2k_test_support::retail_test]
fn type7_all_eight_wet_authored_births_keep_own_receipts_and_run_in_both_worlds() {
    for (level, spawns) in [(22, vec![25, 26, 27]), (34, vec![12, 13, 14, 15, 16])] {
        let mut fx = WorldFx::new();
        let (session, mut manager) = tests::load(level, &mut fx).expect("canonical retail corpus");
        let ids = manager
            .iter_all()
            .filter(|e| e.entity_type == 7)
            .map(|e| e.id)
            .collect::<Vec<_>>();
        assert_eq!(
            ids.iter()
                .map(|id| actor(&manager, *id).authored_spawn_index.unwrap())
                .collect::<Vec<_>>(),
            spawns
        );
        let before = ids
            .iter()
            .map(|id| {
                (
                    *id,
                    actor(&manager, *id).position_raw(),
                    actor(&manager, *id).native_type86_runtime.unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let mut owners = Vec::new();
        for &id in &ids {
            let entity = actor(&manager, id);
            assert!(native_type86_manager_allocation_authenticates(&manager, id));
            assert_eq!(
                entity.native_type86_runtime.unwrap().profile,
                NativeFourChoiceProfile::DiverWorker
            );
            assert!(entity.intro2_type8_runtime.is_none());
            assert_eq!(entity.model_slots, [Some(1249); 4]);
            assert_eq!(
                entity.capability_flags & 0xc00,
                0x400,
                "factory worker, never Base person"
            );
            let cache = *entity.type8_sub_d_frame_owner.unwrap().classifier_cache();
            assert_eq!(cache.origin(), RetailRuntimeValue::Unresolved);
            let spawn =
                &session.cache.level_desc().unwrap().entities[entity.authored_spawn_index.unwrap()];
            let [x, _, z] = spawn.position_raw();
            assert_eq!(spawn.param, 0);
            assert_eq!(spawn.rotation, [0; 3]);
            assert!(
                session
                    .cache
                    .level_terrain()
                    .unwrap()
                    .bilinear_height_raw(x, z)
                    < session.cache.level_terrain().unwrap().sea_level_raw()
            );
            detailed(manager.entity_mut(id).unwrap(), false);
            owners.push(Type86Owner::take_birth(manager.entity_mut(id).unwrap()).unwrap());
        }
        for step in 1..=20 {
            for owner in &mut owners {
                let tick = tick_type86(
                    &mut manager,
                    *owner,
                    Type86Frame {
                        resources: &session.cache,
                        world_fx: &mut fx,
                        elapsed_micros: 100_000,
                        global_elapsed_micros: 100_000,
                        retail_tick: step * 5,
                    },
                );
                assert!(
                    matches!(
                        tick.outcome,
                        Type86Outcome::Advanced {
                            terminal: false,
                            ..
                        }
                    ),
                    "world{level}: {:?}",
                    tick.outcome
                );
                *owner = tick.retained_owner.unwrap();
            }
        }
        for (id, position, receipt) in before {
            assert!(actor(&manager, id)
                .native_type86_runtime
                .unwrap()
                .same_allocation(receipt));
            assert_eq!(
                actor(&manager, id)
                    .type8_sub_d_frame_owner
                    .unwrap()
                    .classifier_cache()
                    .origin(),
                RetailRuntimeValue::Unresolved,
                "classifier0 never queries terrain cache"
            );
            assert_ne!(
                actor(&manager, id).position_raw(),
                position,
                "world{level} diver{id} never moved"
            );
            assert_eq!(
                actor(&manager, id).collision.health_raw,
                RetailRuntimeValue::Known(1500),
                "surface0 does not run ordinary-worker drowning"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn type7_query_free_d_keeps_process_seed_and_advances_in_detailed_and_coarse_tasks() {
    for mode in [0, 1] {
        let (session, mut manager, mut fx, id) = fixture(34);
        isolate(&mut manager, id);
        let mut owner = choose(&mut manager, &mut fx, id, TaskKind::Wander);
        let receipt = actor(&manager, id).native_type86_runtime.unwrap();
        let before = *actor(&manager, id)
            .type8_sub_d_frame_owner
            .unwrap()
            .classifier_cache();
        let animation = actor(&manager, id).actor_animation_runtime;
        for step in 1..=16 {
            run_task(&session, &mut manager, &mut fx, &mut owner, 20_000, mode);
            let cache = *actor(&manager, id)
                .type8_sub_d_frame_owner
                .unwrap()
                .classifier_cache();
            assert_eq!(
                cache.stagger_counter(),
                before.stagger_counter().wrapping_add(step)
            );
            assert_eq!(cache.origin(), before.origin());
            assert_eq!(cache.rows(), before.rows());
            assert!(actor(&manager, id)
                .native_type86_runtime
                .unwrap()
                .same_allocation(receipt));
        }
        if mode == 1 {
            assert_eq!(
                actor(&manager, id).actor_animation_runtime,
                animation,
                "coarse ABDI skips I"
            );
        } else {
            assert_ne!(
                actor(&manager, id).actor_animation_runtime,
                animation,
                "detailed ABDI advances I"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn type7_job_rule_uses_real_free_factory_capacity_and_not_a_nearby_base() {
    let (_, mut manager, mut fx, id) = fixture(34);
    isolate(&mut manager, id);
    let base = manager
        .iter_all()
        .find(|e| e.entity_type == 6)
        .expect("DarkReef Main Base")
        .id;
    near(&mut manager, id, base);
    for _ in 0..16 {
        assert_eq!(
            birth::reselect(&mut manager, id, &mut fx).unwrap().kind,
            TaskKind::Wander
        );
    }
    let factory = manager.iter_all().find(|e| e.entity_type == 66).unwrap().id;
    near(&mut manager, id, factory);
    let RetailRuntimeValue::Known(Some(runtime)) =
        &mut manager.entity_mut(factory).unwrap().base_factory_runtime
    else {
        panic!()
    };
    assert!(runtime.required_scientists > 0, "authored world34 capacity");
    runtime.current_scientists = 0;
    runtime.production.as_mut().unwrap().current_scientists_raw = 0;
    let owner = choose(&mut manager, &mut fx, id, TaskKind::GoToJob);
    assert!(
        matches!(actor(&manager,id).actor_tasks.state_in_slot(ActorTaskSlot::Primary),Some(ActorTaskRuntime::GoToJob(task)) if task.target_id()==Some(factory))
    );
    assert_eq!(owner.kind, TaskKind::GoToJob);
    let RetailRuntimeValue::Known(Some(runtime)) =
        &mut manager.entity_mut(factory).unwrap().base_factory_runtime
    else {
        panic!()
    };
    runtime.current_scientists = runtime.required_scientists;
    runtime.production.as_mut().unwrap().current_scientists_raw =
        i32::from(runtime.required_scientists);
    for _ in 0..16 {
        assert_eq!(
            birth::reselect(&mut manager, id, &mut fx).unwrap().kind,
            TaskKind::Wander
        );
    }
}

#[v2k_test_support::retail_test]
fn type7_real_player_selects_both_class45_parities_and_preserves_cue106() {
    let (session, mut manager, mut fx, id) = fixture(34);
    isolate(&mut manager, id);
    let player = manager.player().unwrap().id;
    near(&mut manager, id, player);
    let mut parities = [false; 2];
    let mut acquiring = None;
    for _ in 0..128 {
        fx.process_pending();
        fx.take_positional_sounds();
        let mut oracle = fx.fork_for_main_base_abort_transaction();
        oracle.next_shared_retail_random_u16(); //425680 selector.
        let owner = birth::reselect(&mut manager, id, &mut fx).unwrap();
        if owner.kind == TaskKind::AttractAcquiring {
            let parity = oracle.next_shared_retail_random_u16() & 1;
            let candidate = actor(&manager, id)
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Secondary)
                .is_some();
            assert_eq!(candidate, parity != 0);
            for _ in 0..(2 + usize::from(candidate)) {
                oracle.next_shared_retail_random_u16();
            }
            parities[usize::from(candidate)] = true;
            fx.process_pending();
            assert_eq!(
                fx.take_positional_sounds()
                    .iter()
                    .map(|s| s.sound_id)
                    .collect::<Vec<_>>(),
                [106],
                "BA40 plays the actual Sub-I+04 cue; 72 belongs to Type9"
            );
            if candidate {
                acquiring = Some(owner);
            }
        } else {
            assert_eq!(owner.kind, TaskKind::Wander);
            oracle.next_shared_retail_random_u16();
        }
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16(),
            "selector/parity/successful constructors retain exact RNG order"
        );
        if parities == [true, true] && acquiring.is_some_and(|o| o.task_id == owner.task_id) {
            break;
        }
    }
    assert_eq!(parities, [true, true]);
    let mut owner = acquiring.unwrap();
    for _ in 0..64 {
        run_task(&session, &mut manager, &mut fx, &mut owner, 20_000, 0);
        if owner.kind == TaskKind::AttractTarget {
            break;
        }
    }
    assert_eq!(owner.kind, TaskKind::AttractTarget);
    assert!(
        matches!(actor(&manager,id).actor_tasks.state_in_slot(ActorTaskSlot::Primary),Some(ActorTaskRuntime::AttractAttentionTargetRoute(task)) if task.target_id()==Some(player))
    );
    assert!(actor(&manager, id)
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .is_none());
    assert!(actor(&manager, id)
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Tertiary)
        .is_none());
}

#[v2k_test_support::retail_test]
fn type7_world22_arrival_handoff_retains_the_new_graph_through_the_next_scheduler_frame() {
    use crate::{
        campaign_transition::CampaignArrivalCatalog,
        entity::{AuthoredPlayerArrival, AuthoredWorldConstruction, EntityConstructionResources},
        gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
        specialized_actor_task_production::{
            SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
            SpecializedActorTaskScheduler,
        },
        static_damage::StaticDamageScheduler,
    };

    let (mut session, _, _) = native_fixture_with_player(22);
    let arrival = CampaignArrivalCatalog::load(&session, 1)
        .unwrap()
        .direct_world_entry(22)
        .unwrap()
        .arrival();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, &model_slots)| {
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
    let mut fx = WorldFx::new();
    let mut manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: 10,
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: Some(AuthoredPlayerArrival {
                position_raw: arrival.position_raw,
                heading_raw: arrival.heading_raw,
            }),
            retail_tick: 4793,
        },
        &mut fx,
    )
    .unwrap();
    manager.cleanup_pending_actor_deferred_destroys();
    let player = manager.player().unwrap().id;
    let ids: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 7)
        .map(|entity| entity.id)
        .collect();
    assert_eq!(ids.len(), 3);
    for &id in &ids {
        // Match the real birth-render observation without changing any
        // position, task, scheduler policy, candidate or RNG state.
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x800, 0x800);
        // Each birth draws its root from the shared RNG, after every earlier
        // native constructor (world 22's Type18 and Type28 included): any
        // published living graph is a valid birth.
        assert!(matches!(
            Type86Owner::adopt_published(actor(&manager, id))
                .unwrap()
                .kind,
            TaskKind::AttractAcquiring | TaskKind::Wander
        ));
    }
    let id = ids[0];
    let mut scheduler = SpecializedActorTaskScheduler::default();
    assert_eq!(scheduler.adopt_native_type86(&mut manager), 3);
    // Class45's acquisition Secondary is chosen by a real parity draw.
    // Earlier authored constructors share this RNG, so the handoff fixture
    // must not assume a particular birth parity. Re-enter the native selector
    // against these unchanged world22 candidates, retaining the first graph
    // that can perform the Secondary -> target-route transition under test.
    let acquiring = (0..128)
        .find_map(|_| {
            let owner = birth::reselect(&mut manager, id, &mut fx).unwrap();
            (owner.kind == TaskKind::AttractAcquiring
                && actor(&manager, id)
                    .actor_tasks
                    .task_in_slot(ActorTaskSlot::Secondary)
                    .is_some())
            .then_some(owner)
        })
        .expect("real world22 arrival must select acquisition with a Secondary");
    scheduler.register_native_type86(acquiring);
    let mut notifications = GameplayNotifications::new();
    let mut static_damage = StaticDamageScheduler::new();
    let mut published_primary = None;
    let mut next_frame_verified = false;
    // The installed Secondary independently gates acquisition one time in
    // four. Observe its real handoff, then check exactly the following pass;
    // neither gate may depend on unrelated constructors' RNG consumption.
    for step in 1..=128 {
        let before = Type86Owner::adopt_published(actor(&manager, id)).unwrap();
        let old_secondary = actor(&manager, id)
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Secondary);
        manager.advance_environment_frame(&mut fx, 100_000);
        let pass = scheduler.tick(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase: GameplayNotificationPhase::Playing,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                elapsed_micros: 100_000,
                global_elapsed_micros: 100_000,
                retail_tick: 4793 + step * 5,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        assert!(pass.block.is_none(), "frame{step}: {:?}", pass.block);
        assert_eq!(pass.outcomes.len(), 3);
        assert!(
            pass.outcomes.iter().all(|outcome| matches!(
                outcome,
                SpecializedActorTaskProductionOutcome::NativeType86(Type86Outcome::Advanced {
                    terminal: false,
                    ..
                })
            )),
            "frame{step}: {:?}",
            pass.outcomes
        );
        assert_eq!(scheduler.registered_len(), 3);
        assert!(scheduler.main_base_person_completed_owner(&manager, id));
        let entity = actor(&manager, id);
        let owner = Type86Owner::adopt_published(entity).unwrap();
        if published_primary.is_none() && owner.kind != TaskKind::AttractTarget {
            fx.process_pending();
            continue;
        }
        assert_eq!(owner.kind, TaskKind::AttractTarget);
        assert_eq!(
            owner.context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(player))
        );
        assert!(entity
            .actor_tasks
            .state_in_slot(ActorTaskSlot::Secondary)
            .is_none());
        assert!(entity
            .actor_tasks
            .state_in_slot(ActorTaskSlot::Tertiary)
            .is_none());
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_tasks.state_in_slot(ActorTaskSlot::Primary)
        else {
            panic!("frame{step}: missing target route")
        };
        if let Some(primary) = published_primary {
            assert_eq!(owner.task_id, primary);
            assert_eq!(route.elapsed_ms(), 100);
            next_frame_verified = true;
            break;
        } else {
            // AF50's newborn Primary must not run in the old Secondary visit.
            assert_eq!(before.kind, TaskKind::AttractAcquiring);
            assert_ne!(owner.task_id, before.task_id);
            assert_eq!(
                entity.actor_tasks.wrapper_flags(
                    old_secondary.expect("the handoff must retire its acquisition Secondary")
                ),
                None
            );
            assert_eq!(route.elapsed_ms(), 0);
            published_primary = Some(owner.task_id);
        }
        fx.process_pending();
    }
    assert!(
        next_frame_verified,
        "world22 must reach acquisition and retain its new Primary on the following frame"
    );
    assert_eq!(scheduler.adopt_native_type86(&mut manager), 0);
}

#[v2k_test_support::retail_test]
fn type7_class10_uses_own_axis_filter_after_baddie_root_selection() {
    let (session, mut manager, mut fx, id) = fixture(34);
    isolate(&mut manager, id);
    let hostile = manager
        .iter_all()
        .find(|e| e.capability_flags & 8 != 0)
        .expect("authored hostile")
        .id;
    near(&mut manager, id, hostile);
    let mut owner = choose(&mut manager, &mut fx, id, TaskKind::RunAwayAcquiring);
    assert!(matches!(
        actor(&manager, id)
            .actor_tasks
            .state_in_slot(ActorTaskSlot::Secondary),
        Some(ActorTaskRuntime::TargetAcquisition(task))
            if task.filter().raw() == 0xa1
                && task.radius().raw() == 3072
                && task.constructor_filter_override_raw() == 0
    ));
    assert_eq!(actor(&manager, hostile).capability_flags & 0xa1, 0);
    fx.process_pending();
    fx.take_positional_sounds();
    // Rule7's mask8 witnesses a nearby baddie, but B6C0 retains the actor's
    // distinct +04 maskA1. That witness is not a preselected flee target.
    for _ in 0..20 {
        run_task(&session, &mut manager, &mut fx, &mut owner, 20_000, 0);
        assert_eq!(owner.kind, TaskKind::RunAwayAcquiring);
    }
    let player = manager.player().unwrap().id;
    assert_ne!(actor(&manager, player).capability_flags & 0xa1, 0);
    near(&mut manager, id, player);
    run_task(&session, &mut manager, &mut fx, &mut owner, 20_000, 0);
    assert_eq!(owner.kind, TaskKind::RunAwayFleeing);
    assert!(
        matches!(actor(&manager,id).actor_tasks.state_in_slot(ActorTaskSlot::Primary),Some(ActorTaskRuntime::RunAway(task)) if task.target_id()==player)
    );
    fx.process_pending();
    assert!(fx.take_positional_sounds().is_empty());
}

#[v2k_test_support::retail_test]
fn type7_live_sound40_uses_healthy_visible_gate_after_actual_task_rng() {
    let (session, _, _, _) = fixture(34);
    let record = session.cache.global_entity_type(7).unwrap();
    let threshold = (100_000u32 << 6) / 2929;
    for (word, expected) in [(threshold, Some(40)), (threshold + 1, None)] {
        let mut draws = [word].into_iter();
        assert_eq!(
            crate::actor_detailed_sound::plan_actor_detailed_sound(
                crate::actor_detailed_sound::ActorDetailedSoundFrame {
                    type_record: record,
                    health_raw: 750,
                    visible: true,
                    callback_elapsed_micros: 100_000,
                },
                &mut || draws.next().expect("exactly one chance word"),
            ),
            expected,
            "canonical sound40 accepts equality without an alternate draw"
        );
        assert!(draws.next().is_none());
    }
    for (health, visible) in [(1500, true), (750, true), (749, true), (1500, false)] {
        let (session, mut manager, mut fx, id) = fixture(34);
        let (_, mut oracle, mut oracle_fx, oracle_id) = fixture(34);
        assert_eq!(id, oracle_id);
        isolate(&mut manager, id);
        isolate(&mut oracle, id);
        let owner = choose(&mut manager, &mut fx, id, TaskKind::Wander);
        let mut oracle_owner = choose(&mut oracle, &mut oracle_fx, id, TaskKind::Wander);
        for actors in [&mut manager, &mut oracle] {
            detailed(actors.entity_mut(id).unwrap(), visible);
            actors.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(health);
        }
        fx.process_pending();
        fx.take_positional_sounds();
        run_task(
            &session,
            &mut oracle,
            &mut oracle_fx,
            &mut oracle_owner,
            100_000,
            0,
        );
        let expected = if health >= 750 && visible {
            (u32::from(oracle_fx.next_shared_retail_random_u16()) <= (100_000u32 << 6) / 2929)
                .then_some(40)
        } else {
            None
        };
        let tick = tick_type86(
            &mut manager,
            owner,
            Type86Frame {
                resources: &session.cache,
                world_fx: &mut fx,
                elapsed_micros: 100_000,
                global_elapsed_micros: 100_000,
                retail_tick: 5000,
            },
        );
        assert!(
            matches!(tick.outcome, Type86Outcome::Advanced { detailed: true, .. }),
            "{:?}",
            tick.outcome
        );
        fx.process_pending();
        assert_eq!(
            fx.take_positional_sounds()
                .iter()
                .map(|s| s.sound_id)
                .collect::<Vec<_>>(),
            expected.into_iter().collect::<Vec<_>>()
        );
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle_fx.next_shared_retail_random_u16(),
            "health{health} visible{visible}"
        );
    }
}

#[v2k_test_support::retail_test]
fn type7_carry_release_retains_each_living_graph_and_distinct_allocation() {
    for kind in [
        TaskKind::Wander,
        TaskKind::GoToJob,
        TaskKind::AttractAcquiring,
        TaskKind::RunAwayAcquiring,
    ] {
        let (_, mut manager, mut fx, id) = fixture(34);
        isolate(&mut manager, id);
        let parent = manager.player().unwrap().id;
        match kind {
            TaskKind::GoToJob => {
                let factory = manager.iter_all().find(|e| e.entity_type == 66).unwrap().id;
                near(&mut manager, id, factory);
                let RetailRuntimeValue::Known(Some(base)) =
                    &mut manager.entity_mut(factory).unwrap().base_factory_runtime
                else {
                    panic!()
                };
                assert!(base.required_scientists > 0);
                base.current_scientists = 0;
                base.production.as_mut().unwrap().current_scientists_raw = 0;
            }
            TaskKind::AttractAcquiring => near(&mut manager, id, parent),
            TaskKind::RunAwayAcquiring => {
                let hostile = manager
                    .iter_all()
                    .find(|e| e.capability_flags & 8 != 0)
                    .unwrap()
                    .id;
                near(&mut manager, id, hostile);
            }
            _ => {}
        }
        choose(&mut manager, &mut fx, id, kind);
        let receipt = actor(&manager, id).native_type86_runtime.unwrap();
        let d = actor(&manager, id).type8_sub_d_frame_owner;
        let plan = cargo::prepare_attach(&manager, id, parent).unwrap();
        let RetailRuntimeValue::Known(Some(rows)) =
            &mut manager.entity_mut(parent).unwrap().sub_j_attachment_runtime
        else {
            panic!()
        };
        rows.append(id).unwrap();
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.state_flags_at_0x08 =
            relation_attach_state_word_after(entity.collision.state_flags_at_0x08);
        entity.attached_to = Some(parent);
        assert_eq!(
            cargo::commit_attach(&mut manager, id, plan, &mut fx),
            NativeActorAttachOutcome::RetainedGraph
        );
        let carried = Type86Owner::adopt_published(actor(&manager, id)).unwrap();
        assert_eq!(carried.kind, TaskKind::Carried);
        assert!(matches!(
            actor(&manager, id)
                .actor_tasks
                .state_in_slot(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::None)
        ));
        assert_eq!(actor(&manager, id).type8_sub_d_frame_owner, d);
        let release =
            cargo::prepare_release(&manager, id, parent, Type9CargoReleasePosition::Retained)
                .unwrap();
        let RetailRuntimeValue::Known(Some(rows)) =
            &mut manager.entity_mut(parent).unwrap().sub_j_attachment_runtime
        else {
            panic!()
        };
        assert_eq!(rows.pop_last(), Some(id));
        let released = cargo::commit_release(&mut manager, id, release, &mut fx);
        assert_ne!(released.kind, TaskKind::Carried);
        assert_eq!(actor(&manager, id).attached_to, None);
        assert!(actor(&manager, id)
            .native_type86_runtime
            .unwrap()
            .same_allocation(receipt));
        assert_eq!(actor(&manager, id).type8_sub_d_frame_owner, d);
        assert!(native_type86_manager_allocation_authenticates(&manager, id));
        let RetailRuntimeValue::Known(Some(animation)) =
            actor(&manager, id).actor_animation_runtime
        else {
            panic!()
        };
        assert_eq!(animation.descriptor().attention_stop_sound_id, 106);
    }
}

#[v2k_test_support::retail_test]
fn type7_foreign_receipt_or_parked_owner_cannot_mutate_body_graph_or_rng() {
    let (session, mut manager, mut fx, id) = fixture(34);
    let (_, foreign, _, _) = fixture(34);
    let owner = Type86Owner::take_birth(manager.entity_mut(id).unwrap()).unwrap();
    let before = (
        actor(&manager, id).collision.clone(),
        actor(&manager, id).position_raw(),
        actor(&manager, id).type8_sub_d_frame_owner,
    );
    let mut oracle = fx.fork_for_main_base_abort_transaction();
    manager.entity_mut(id).unwrap().native_type86_runtime =
        actor(&foreign, id).native_type86_runtime;
    assert!(!native_type86_manager_allocation_authenticates(
        &manager, id
    ));
    let tick = tick_type86(
        &mut manager,
        owner,
        Type86Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 100_000,
            global_elapsed_micros: 100_000,
            retail_tick: 5000,
        },
    );
    assert!(matches!(tick.outcome, Type86Outcome::Dropped { .. }));
    assert_eq!(
        (
            actor(&manager, id).collision.clone(),
            actor(&manager, id).position_raw(),
            actor(&manager, id).type8_sub_d_frame_owner
        ),
        before
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );

    let (session, mut manager, mut fx, id) = fixture(34);
    let mut owner = Type86Owner::take_birth(manager.entity_mut(id).unwrap()).unwrap();
    owner.pending = true;
    let before = (
        actor(&manager, id).collision.clone(),
        actor(&manager, id).position_raw(),
        actor(&manager, id).type8_sub_d_frame_owner,
    );
    let mut oracle = fx.fork_for_main_base_abort_transaction();
    for _ in 0..2 {
        let tick = tick_type86(
            &mut manager,
            owner,
            Type86Frame {
                resources: &session.cache,
                world_fx: &mut fx,
                elapsed_micros: 100_000,
                global_elapsed_micros: 100_000,
                retail_tick: 5000,
            },
        );
        assert!(matches!(tick.outcome, Type86Outcome::Pending { .. }));
        owner = tick.retained_owner.unwrap();
        assert_eq!(
            (
                actor(&manager, id).collision.clone(),
                actor(&manager, id).position_raw(),
                actor(&manager, id).type8_sub_d_frame_owner
            ),
            before
        );
    }
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
}
