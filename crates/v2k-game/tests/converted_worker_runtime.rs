//! Dynamic worker construction is independent of authored worker admission.
//! Factory tests enter a controlled late production state on real allocations;
//! Main Base tests retain real people graphs and use the oriented contact pass.

use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::{AuthoredWorldConstruction, Entity, EntityConstructionResources, EntityManager},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    entity_scheduler::{
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    },
    factory_production::{FactoryProductionPhase, FACTORY_OUTPUT_CONVERSION_GATE_RAW},
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    intro2_type66::{
        tick_intro2_type66_owner, Intro2Type66Frame, Intro2Type66Outcome, Intro2Type66Owner,
    },
    intro2_type8::Intro2Type8Outcome,
    main_base_conversion::MainBaseConversionActionOutcome,
    main_base_conversion_live::{
        resolve_first_world_main_base_conversions, FirstWorldMainBaseConversionPass,
        MainBaseConversionFrame,
    },
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

struct World {
    level: u32,
    session: GameSession,
    entities: EntityManager,
    fx: WorldFx,
    tasks: SpecializedActorTaskScheduler,
    notifications: GameplayNotifications,
    static_damage: StaticDamageScheduler,
}

impl World {
    fn load(level: u32) -> Self {
        let data = v2k_test_support::retail_dir();
        assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
        let mut session = GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(level, 1).unwrap();
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(id, _)| {
                EntityTypeRuntimeMetadata::from_section12(
                    session.cache.global_entity_type(id).unwrap(),
                )
            })
            .collect();
        let mut fx = WorldFx::new();
        let entities = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index: (level - 12) as i32,
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
        Self {
            level,
            session,
            entities,
            fx,
            tasks: SpecializedActorTaskScheduler::new(),
            notifications: GameplayNotifications::new(),
            static_damage: StaticDamageScheduler::new(),
        }
    }

    fn entity(&self, id: u32) -> &Entity {
        self.entities
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap()
    }

    fn step(&mut self, tick: u32) {
        let pass = self.tasks.tick(
            &mut self.entities,
            SpecializedActorTaskProductionFrame {
                world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase: GameplayNotificationPhase::Playing,
                resources: &mut self.session.cache,
                world_fx: &mut self.fx,
                static_damage: &mut self.static_damage,
                elapsed_micros: 100_000,
                global_elapsed_micros: 100_000,
                retail_tick: tick,
                main_base_abort_active: false,
            },
            &mut self.notifications,
        );
        assert!(
            pass.block.is_none(),
            "world{}: {:?}",
            self.level,
            pass.block
        );
        for outcome in pass.outcomes {
            if let SpecializedActorTaskProductionOutcome::NativeType86(ref person) = outcome {
                assert!(
                    !matches!(
                        person,
                        v2k_game::native_type86::Type86Outcome::Blocked { .. }
                            | v2k_game::native_type86::Type86Outcome::Pending { .. }
                    ),
                    "world{}: {person:?}",
                    self.level
                );
            }
            if let SpecializedActorTaskProductionOutcome::Intro2Type8(outcome) = outcome {
                assert!(
                    matches!(
                        outcome,
                        Intro2Type8Outcome::Advanced { .. } | Intro2Type8Outcome::Waiting { .. }
                    ),
                    "world{}: {outcome:?}",
                    self.level
                );
            }
        }
    }

    fn ordinal(&self) -> u16 {
        let RetailRuntimeValue::Known(ordinal) = self.entities.next_common_body_ordinal() else {
            panic!("native body lineage");
        };
        ordinal
    }

    fn assert_stamp(&self, id: u32, ordinal: u16) {
        assert_eq!(
            self.entity(id).construction_stamp_at_0xb4,
            RetailRuntimeValue::Known(
                ((self.level - 12) as u16)
                    .wrapping_mul(0x400)
                    .wrapping_add(ordinal)
            )
        );
    }

    fn assert_worker(&self, id: u32, worker_type: u32) {
        let worker = self.entity(id);
        let record = self
            .session
            .cache
            .global_entity_type(worker_type as usize)
            .unwrap();
        let metadata = EntityTypeRuntimeMetadata::from_section12(record);
        assert_eq!(worker.entity_type, worker_type);
        assert!(
            worker.intro2_type8_runtime.is_some(),
            "actual worker allocation owner"
        );
        assert_eq!(
            worker.model_slots,
            metadata.model_slots.map(|id| Some(usize::from(id)))
        );
        assert_eq!(
            worker.collision.health_raw,
            RetailRuntimeValue::Known(metadata.initial_health_raw.unwrap())
        );
        let RetailRuntimeValue::Known(Some(animation)) = worker.actor_animation_runtime else {
            panic!("Sub-I")
        };
        assert_eq!(
            RetailRuntimeValue::Known(Some(animation.descriptor())),
            metadata.actor_animation_descriptor
        );
        let RetailRuntimeValue::Known(Some(steering)) = metadata.sub_d_steering_descriptor else {
            panic!("authored Sub-D")
        };
        assert_eq!(
            steering.steering_divisor_raw,
            if matches!(worker_type, 79 | 90 | 91) {
                32
            } else {
                20
            }
        );
        assert!(worker.type8_sub_d_runtime.is_some());
        assert!(matches!(
            worker.physical_body_basis_q31(),
            RetailRuntimeValue::Known(_)
        ));
    }
}

fn detailed(entity: &mut Entity) {
    let bits =
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT;
    entity.collision.state_flags_at_0x08.overwrite(bits, bits);
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
}

/// The authored template, native receipt and Class39 task stay intact. Only the
/// runtime staffing/stock/phase is arranged at the proven final-ejection edge.
fn prepare_ejection(world: &mut World, available_job: bool) -> u32 {
    let factory_ids: Vec<_> = world
        .entities
        .iter_all()
        .filter(|e| e.entity_type == 66)
        .map(|e| e.id)
        .collect();
    let source = factory_ids.iter().copied().find(|id| {
        matches!(world.entity(*id).base_factory_runtime, RetailRuntimeValue::Known(Some(base)) if base.required_scientists > 0)
    }).unwrap();
    assert!(matches!(
        world
            .entity(source)
            .actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::WorkingFactory(_))
    ));
    let vacant_target = available_job.then(|| {
        let output_type = v2k_game::factory_production_live::converted_output_entity_type(
            world.session.cache.level_desc().unwrap().world_style,
        );
        let metadata = EntityTypeRuntimeMetadata::from_section12(
            world.session.cache.global_entity_type(output_type as usize).unwrap(),
        );
        let range = metadata.initializer.unwrap().common_axis_descriptor.strict_axis_limit_raw;
        let mut output_position = world.entity(source).position_raw();
        output_position[2] = output_position[2].wrapping_add(
            v2k_game::factory_production_live::FACTORY_CONVERTED_OUTPUT_Z_OFFSET_RAW,
        );
        factory_ids.iter().copied().find(|id| *id != source
            && matches!(world.entity(*id).base_factory_runtime, RetailRuntimeValue::Known(Some(base)) if base.required_scientists > 0)
            && [0, 2].into_iter().all(|axis|
                i32::from(world.entity(*id).position_raw()[axis].wrapping_sub(output_position[axis])).abs() < range))
            .expect("available-job case needs a separate authored nearby factory")
    });
    for id in factory_ids {
        let entity = world.entities.entity_mut(id).unwrap();
        let RetailRuntimeValue::Known(Some(base)) = &mut entity.base_factory_runtime else {
            panic!("Sub-M")
        };
        base.current_scientists = base.required_scientists;
        let production = base.production.as_mut().unwrap();
        production.current_scientists_raw = i32::from(base.current_scientists);
        if Some(id) == vacant_target {
            base.current_scientists = 0;
            production.current_scientists_raw = 0;
        }
        if id == source {
            production.remaining_stock_raw = 0;
            production.phase = FactoryProductionPhase::WaitingForPickup;
            production.production_progress_micros_raw = production
                .production_threshold_micros_raw
                .wrapping_add(FACTORY_OUTPUT_CONVERSION_GATE_RAW)
                .wrapping_add(1);
            detailed(entity);
        }
    }
    source
}

#[v2k_test_support::retail_test]
fn native_worker_ejections_retain_own_birth_and_materialiser_lifetimes() {
    for (level, worker_type, available_job) in [
        (16, 91, false),
        (19, 79, false),
        (19, 79, true),
        (24, 90, false),
        (28, 90, false),
        (28, 90, true),
        (38, 79, false),
        (38, 79, true),
        (40, 91, false),
        (43, 116, false),
    ] {
        let mut world = World::load(level);
        let mut oracle = World::load(level);
        let source = prepare_ejection(&mut world, available_job);
        prepare_ejection(&mut oracle, available_job);
        let source_allocation = world.entity(source).intro2_type66_runtime;
        let before_order: Vec<_> = world.entities.retail_live_order_ids().collect();
        let ordinal = world.ordinal();
        let seed = world.fx.next_sub_d_allocation_seed();
        oracle.fx.next_shared_retail_random_u16(); //20450 Sub-A.
        let selector = oracle.fx.next_shared_retail_random_u16();
        let task_word = oracle.fx.next_shared_retail_random_u16();
        oracle.fx.next_shared_retail_random_u16(); //Type93 singleton selector.
        let style = world.session.cache.level_desc().unwrap().world_style;
        let tick = tick_intro2_type66_owner(
            &mut world.entities,
            Intro2Type66Owner::adopt(&oracle.entities, source).unwrap(),
            Intro2Type66Frame {
                resources: &mut world.session.cache,
                world_fx: &mut world.fx,
                notifications: &mut world.notifications,
                static_damage: &mut world.static_damage,
                elapsed_micros: 0,
                retail_tick: 100,
                world_style_raw: style,
                main_base_abort_active: false,
            },
        );
        // The same-id foreign manager must not own even the first body attempt.
        assert!(matches!(tick.outcome, Intro2Type66Outcome::Dropped { .. }));
        assert_eq!(world.ordinal(), ordinal);
        assert_eq!(world.fx.next_sub_d_allocation_seed(), seed);
        assert_eq!(
            world.entities.retail_live_order_ids().collect::<Vec<_>>(),
            before_order
        );
        let owner = Intro2Type66Owner::adopt(&world.entities, source).unwrap();
        let tick = tick_intro2_type66_owner(
            &mut world.entities,
            owner,
            Intro2Type66Frame {
                resources: &mut world.session.cache,
                world_fx: &mut world.fx,
                notifications: &mut world.notifications,
                static_damage: &mut world.static_damage,
                elapsed_micros: 0,
                retail_tick: 100,
                world_style_raw: style,
                main_base_abort_active: false,
            },
        );
        assert!(
            matches!(
                tick.outcome,
                Intro2Type66Outcome::Advanced {
                    callback_enabled: true,
                    ..
                }
            ),
            "world{level}: {:?}",
            tick.outcome
        );
        assert_eq!(
            world.entity(source).intro2_type66_runtime,
            source_allocation
        );
        let birth = world
            .entities
            .factory_converted_output_births()
            .last()
            .unwrap_or_else(|| panic!("world{level} available_job={available_job}: expected one dynamic birth; factory={:?}", world.entity(source).base_factory_runtime));
        assert_eq!(world.entities.factory_converted_output_births().len(), 1);
        let worker = birth.entity_id();
        assert_eq!(birth.source_factory().entity_id, source);
        assert_eq!(birth.spawn_request().entity_type, worker_type);
        assert_eq!(birth.selector_rng_word(), selector);
        assert_eq!(birth.constructor_rng_words(), &[task_word]);
        assert_eq!(
            birth.selected_behavior_class_id(),
            if available_job { 54 } else { 6 }
        );
        world.assert_worker(worker, worker_type);
        assert_eq!(world.entity(worker).authored_spawn_index, None);
        assert_eq!(
            world.entity(worker).collision.recent_relation_id_at_0x60,
            RetailRuntimeValue::Known(Some(source))
        );
        assert_eq!(
            world
                .entity(worker)
                .type8_sub_d_frame_owner
                .unwrap()
                .classifier_cache()
                .stagger_counter(),
            seed
        );
        let materialiser = world
            .entity(worker)
            .attached_to
            .expect("factory Type93 link");
        assert_eq!(world.entity(materialiser).entity_type, 93);
        assert_eq!(
            world.entity(materialiser).model_slots[0],
            world.entity(worker).model_index
        );
        let RetailRuntimeValue::Known(Some(sub_j)) =
            &world.entity(materialiser).sub_j_attachment_runtime
        else {
            panic!("Type93 Sub-J")
        };
        assert_eq!(sub_j.ordered_entity_ids(), [worker]);
        let order: Vec<_> = world.entities.retail_live_order_ids().collect();
        assert_eq!(&order[..before_order.len()], before_order);
        assert_eq!(&order[before_order.len()..], [worker, materialiser]);
        world.assert_stamp(worker, ordinal);
        world.assert_stamp(materialiser, ordinal.wrapping_add(1));
        assert_eq!(world.ordinal(), ordinal.wrapping_add(2));
        assert_eq!(world.fx.next_sub_d_allocation_seed(), seed.wrapping_add(1));
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            oracle.fx.next_shared_retail_random_u16(),
            "native ejection consumes Sub-A, selector, task, then Type93 selector only"
        );
        assert_ne!(world.notifications.save_tail_seen_mask() & (1 << 0x16), 0);
        assert!(matches!(
            world
                .entity(worker)
                .actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::None)
        ));

        let native_count = world
            .entities
            .iter_all()
            .filter(|e| e.intro2_type8_runtime.is_some())
            .count();
        assert_eq!(
            world.tasks.adopt_intro2_type8(&mut world.entities),
            native_count
        );
        for step in 0..20 {
            let blocked = world.entities.update_late_tail_materialisers(
                v2k_game::entity::LateTailMaterialiserFrame {
                    elapsed_micros: 100_000,
                    terrain: world.session.cache.terrain().unwrap(),
                    world_fx: &mut world.fx,
                    scheduler: &mut world.tasks,
                    notifications: &mut world.notifications,
                    retail_tick: 100 + step * 5,
                },
            );
            assert!(blocked.is_empty(), "{blocked:?}");
            if world.entity(worker).attached_to.is_none() {
                break;
            }
        }
        assert_eq!(
            world.entity(worker).attached_to,
            None,
            "normal Type93 release"
        );
        assert!(!world.entities.iter_all().any(|e| e.id == materialiser));
        world.assert_worker(worker, worker_type);
        oracle.fx.next_shared_retail_random_u16(); //CE90 root reselection.
        oracle.fx.next_shared_retail_random_u16(); //6/54 task construction.
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            oracle.fx.next_shared_retail_random_u16(),
            "materialiser release reselects once without rebuilding the worker body"
        );
        assert_eq!(world.ordinal(), ordinal.wrapping_add(2));
        assert_eq!(world.fx.next_sub_d_allocation_seed(), seed.wrapping_add(1));
        world.assert_stamp(worker, ordinal);
        let worker_ids: Vec<_> = world
            .entities
            .iter_all()
            .filter(|e| e.intro2_type8_runtime.is_some())
            .map(|e| e.id)
            .collect();
        for id in worker_ids {
            detailed(world.entities.entity_mut(id).unwrap());
        }
        assert_eq!(world.tasks.adopt_live_type8_go_to_job(&world.entities), 0);
        assert_eq!(world.tasks.registered_len(), native_count);
        assert_eq!(world.tasks.adopt_intro2_type8(&mut world.entities), 0);
        let before = world.entity(worker).position_raw();
        let mut moved = false;
        for step in 1..=30 {
            world.step(100 + step * 5);
            moved |= world.entity(worker).position_raw() != before;
        }
        assert!(
            moved,
            "world{level} dynamic worker never moved after materialiser release"
        );
    }
}

#[v2k_test_support::retail_test]
fn authored116_zero_capacity_and_factory_absence_are_not_invented_ejections() {
    for (level, workers, factories) in [(42, 3, 1), (47, 2, 0)] {
        let mut world = World::load(level);
        let ids: Vec<_> = world
            .entities
            .iter_all()
            .filter(|e| e.entity_type == 116)
            .map(|e| e.id)
            .collect();
        assert_eq!(ids.len(), workers);
        assert_eq!(
            world
                .entities
                .iter_all()
                .filter(|e| e.entity_type == 66)
                .count(),
            factories
        );
        assert!(world.entities.iter_all().filter(|e| e.entity_type == 66).all(|e|
            matches!(e.base_factory_runtime, RetailRuntimeValue::Known(Some(base)) if base.required_scientists == 0)));
        for id in ids {
            world.assert_worker(id, 116);
            assert!(matches!(
                world.entity(id).actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::OrdinaryType9Wander(_))
            ));
        }
        assert!(world.entities.factory_converted_output_births().is_empty());
        assert_eq!(world.tasks.adopt_intro2_type8(&mut world.entities), workers);
        assert_eq!(world.tasks.adopt_intro2_type8(&mut world.entities), 0);
        //47's existing nonzero-wind worker boundary is outside this birth seam.
        if level == 42 {
            world.step(5);
        }
    }
}

fn prepare_people_contact(world: &mut World) -> (u32, u32) {
    let ids: Vec<_> = world
        .entities
        .iter_all()
        .filter(|e| matches!(e.entity_type, 78 | 86 | 95))
        .map(|e| e.id)
        .collect();
    assert!(!ids.is_empty());
    for id in &ids {
        detailed(world.entities.entity_mut(*id).unwrap());
    }
    assert_eq!(
        world.tasks.adopt_native_type86(&mut world.entities),
        ids.len()
    );
    for tick in [5, 10, 15] {
        world.step(tick);
    }
    let base = world
        .entities
        .iter_all()
        .find(|e| e.entity_type == 6)
        .unwrap();
    let (base_id, position) = (base.id, base.position);
    for id in &ids {
        world.entities.entity_mut(*id).unwrap().position =
            [position[0] + 64.0, position[1], position[2]];
    }
    world.entities.entity_mut(ids[0]).unwrap().position = position;
    (base_id, ids[0])
}

#[v2k_test_support::retail_test]
fn native_person_cohorts_convert_through_actual_base_contact() {
    // WindyIce32 remains an explicit movement prerequisite; its sole factory
    // has zero authored capacity. Every other78/86/95 world uses its own graph.
    for (level, source_type, worker_type) in [
        (16, 95, 91),
        (17, 78, 79),
        (18, 86, 90),
        (19, 78, 79),
        (21, 86, 90),
        (24, 86, 90),
        (26, 86, 90),
        (27, 95, 91),
        (28, 86, 90),
        (31, 95, 91),
        (35, 95, 91),
        (36, 95, 91),
        (37, 86, 90),
        (38, 78, 79),
        (40, 95, 91),
    ] {
        let mut world = World::load(level);
        let mut oracle = World::load(level);
        let (base, source) = prepare_people_contact(&mut world);
        assert_eq!(world.entity(source).entity_type, source_type);
        prepare_people_contact(&mut oracle);
        let source_receipt = world.entity(source).native_type86_runtime;
        let source_stamp = world.entity(source).construction_stamp_at_0xb4;
        let source_graph = ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .map(|slot| world.entity(source).actor_task_state(slot).copied());
        let source_basis = world.entity(source).physical_body_basis_q31();
        let source_animation = world.entity(source).actor_animation_runtime;
        let seed = world.fx.next_sub_d_allocation_seed();
        let ordinal = world.ordinal();
        for _ in 0..3 {
            oracle.fx.next_shared_retail_random_u16();
        }
        let result = resolve_first_world_main_base_conversions(MainBaseConversionFrame {
            entities: &mut world.entities,
            actor_tasks: &mut world.tasks,
            model_pool: &world.session.cache,
            terrain: world.session.cache.terrain(),
            world_fx: &mut world.fx,
            notifications: &mut world.notifications,
            retail_tick: 20,
            world_style: RetailRuntimeValue::Known(
                world.session.cache.level_desc().unwrap().world_style,
            ),
        })
        .unwrap_or_else(|error| panic!("world{level}: {error:?}"));
        let FirstWorldMainBaseConversionPass::Resolved(pass) = result else {
            panic!("native Base")
        };
        let visit = pass.visits.iter().find(|v| v.entity_id == source).unwrap();
        let [MainBaseConversionActionOutcome::Applied, MainBaseConversionActionOutcome::Applied, MainBaseConversionActionOutcome::ReplacementSpawned {
            replacement_entity_id: worker,
        }] = visit.action_outcomes.as_slice()
        else {
            panic!("world{level}: {visit:?}")
        };
        let worker = *worker;
        let suffix = visit
            .suffix_outcome
            .as_ref()
            .expect("descriptor and physical suffix run after destruction is queued");
        assert!(
            matches!(
                suffix.component,
                v2k_game::main_base_conversion_live::MainBaseConversionComponentOutcome::Native(_)
            ),
            "native person must not borrow the captured Type9 component path"
        );
        assert_eq!(
            world.entity(source).velocity_raw(),
            suffix.source_velocity_after_raw
        );
        assert_eq!(world.entity(source).native_type86_runtime, source_receipt);
        assert_eq!(
            world.entity(source).construction_stamp_at_0xb4,
            source_stamp
        );
        assert!(
            source_graph[0].is_some(),
            "source ran its own completed native graph"
        );
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER
                .map(|slot| world.entity(source).actor_task_state(slot).copied()),
            source_graph,
            "02DA0 preserves the actual task private state"
        );
        assert_eq!(world.entity(source).physical_body_basis_q31(), source_basis);
        assert_eq!(
            world.entity(source).actor_animation_runtime,
            source_animation
        );
        assert_eq!(
            world.entities.pending_main_base_conversion_destroy_ids(),
            [source]
        );
        assert_eq!(
            world.entity(worker).collision.recent_relation_id_at_0x60,
            RetailRuntimeValue::Known(Some(base))
        );
        assert_eq!(world.entity(worker).attached_to, None);
        assert_eq!(world.entity(worker).authored_spawn_index, None);
        world.assert_worker(worker, worker_type);
        world.assert_stamp(worker, ordinal);
        assert_eq!(world.ordinal(), ordinal.wrapping_add(1));
        assert_eq!(world.fx.next_sub_d_allocation_seed(), seed.wrapping_add(1));
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            oracle.fx.next_shared_retail_random_u16()
        );
        assert_ne!(world.notifications.save_tail_seen_mask() & (1 << 1), 0);
        assert!(
            pass.visits.iter().any(|v| v.entity_id == worker),
            "saved-next walker reaches tail append"
        );
        assert_eq!(
            world
                .entities
                .cleanup_pending_main_base_conversion_destroys(),
            [source]
        );
        assert!(!world.entities.iter_all().any(|e| e.id == source));
        assert!(world.entities.iter_all().any(|e| e.id == worker));
        let native_count = world
            .entities
            .iter_all()
            .filter(|e| e.intro2_type8_runtime.is_some())
            .count();
        assert_eq!(
            world.tasks.adopt_intro2_type8(&mut world.entities),
            native_count
        );
        assert_eq!(world.tasks.adopt_intro2_type8(&mut world.entities), 0);
        detailed(world.entities.entity_mut(worker).unwrap());
        let before = world.entity(worker).position_raw();
        let mut moved = false;
        for tick in 1..=30 {
            world.step(20 + tick * 5);
            moved |= world.entity(worker).position_raw() != before;
        }
        assert!(
            moved,
            "world{level}: converted Type{worker_type} must execute its native graph"
        );
    }
}

#[v2k_test_support::retail_test]
fn foreign_person_receipt_cannot_commit_conversion_notification_or_birth() {
    for (level, style) in [(19, 4), (24, 3), (40, 2)] {
        let mut world = World::load(level);
        let mut foreign = World::load(level);
        let (_, source) = prepare_people_contact(&mut world);
        let (_, foreign_source) = prepare_people_contact(&mut foreign);
        assert_eq!(
            source, foreign_source,
            "same public handle does not prove allocation custody"
        );
        world
            .entities
            .entity_mut(source)
            .unwrap()
            .native_type86_runtime = foreign.entity(source).native_type86_runtime;
        let collision = world.entity(source).collision.clone();
        let graph = world
            .entity(source)
            .actor_task_state(ActorTaskSlot::Primary)
            .copied();
        let before_order: Vec<_> = world.entities.retail_live_order_ids().collect();
        let ordinal = world.ordinal();
        let seed = world.fx.next_sub_d_allocation_seed();
        let notifications = format!("{:?}", world.notifications);
        let effects = world.fx.pending_event_count();
        let result = resolve_first_world_main_base_conversions(MainBaseConversionFrame {
            entities: &mut world.entities,
            actor_tasks: &mut world.tasks,
            model_pool: &world.session.cache,
            terrain: world.session.cache.terrain(),
            world_fx: &mut world.fx,
            notifications: &mut world.notifications,
            retail_tick: 20,
            world_style: RetailRuntimeValue::Known(style),
        });
        assert!(
            result.is_err(),
            "foreign source receipt must block the reached contact"
        );
        assert_eq!(world.entity(source).collision, collision);
        assert_eq!(
            world
                .entity(source)
                .actor_task_state(ActorTaskSlot::Primary)
                .copied(),
            graph
        );
        assert_eq!(
            world.entities.retail_live_order_ids().collect::<Vec<_>>(),
            before_order
        );
        assert_eq!(world.ordinal(), ordinal);
        assert_eq!(world.fx.next_sub_d_allocation_seed(), seed);
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            foreign.fx.next_shared_retail_random_u16()
        );
        assert_eq!(world.fx.pending_event_count(), effects);
        assert_eq!(format!("{:?}", world.notifications), notifications);
        assert!(world
            .entities
            .pending_main_base_conversion_destroy_ids()
            .is_empty());
    }
}

#[v2k_test_support::retail_test]
fn windy_ice_authors_no_factory_ejection_while_native_people_keep_running() {
    let mut world = World::load(32);
    assert_eq!(
        world.session.cache.level_desc().unwrap().raw_u32(0x90),
        Some(1)
    );
    let factory = world
        .entities
        .iter_all()
        .find(|e| e.entity_type == 66)
        .unwrap()
        .id;
    assert!(matches!(world.entity(factory).base_factory_runtime,
        RetailRuntimeValue::Known(Some(base)) if base.required_scientists == 0));
    detailed(world.entities.entity_mut(factory).unwrap());
    let owner = Intro2Type66Owner::adopt(&world.entities, factory).unwrap();
    let outcome = tick_intro2_type66_owner(
        &mut world.entities,
        owner,
        Intro2Type66Frame {
            resources: &mut world.session.cache,
            world_fx: &mut world.fx,
            notifications: &mut world.notifications,
            static_damage: &mut world.static_damage,
            elapsed_micros: 100_000,
            retail_tick: 5,
            world_style_raw: 4,
            main_base_abort_active: false,
        },
    )
    .outcome;
    assert!(
        matches!(outcome, Intro2Type66Outcome::Advanced { .. }),
        "{outcome:?}"
    );
    assert!(world.entities.factory_converted_output_births().is_empty());
    let people: Vec<_> = world
        .entities
        .iter_all()
        .filter(|e| e.entity_type == 78)
        .map(|e| e.id)
        .collect();
    assert_eq!(people.len(), 3);
    for &id in &people {
        detailed(world.entities.entity_mut(id).unwrap());
    }
    assert_eq!(world.tasks.adopt_native_type86(&mut world.entities), 3);
    let pass = world.tasks.tick(
        &mut world.entities,
        SpecializedActorTaskProductionFrame {
            world:
                v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase: GameplayNotificationPhase::Playing,
            resources: &mut world.session.cache,
            world_fx: &mut world.fx,
            static_damage: &mut world.static_damage,
            elapsed_micros: 100_000,
            global_elapsed_micros: 100_000,
            retail_tick: 10,
            main_base_abort_active: false,
        },
        &mut world.notifications,
    );
    assert!(pass.block.is_none(), "{pass:?}");
    assert_eq!(pass.outcomes.len(), people.len());
    assert!(
        pass.outcomes.iter().all(|outcome| matches!(
            outcome,
            SpecializedActorTaskProductionOutcome::NativeType86(
                v2k_game::native_type86::Type86Outcome::Advanced {
                    terminal: false,
                    ..
                }
            )
        )),
        "{pass:?}"
    );
    assert!(world.entities.factory_converted_output_births().is_empty());
}

#[v2k_test_support::retail_test]
fn remote_native_type86_contact_skips_conversion_before_local_task_custody() {
    use v2k_game::entity_collision_state::REMOTE_OWNED_STATE_BIT;
    use v2k_game::main_base_conversion::{MainBaseConversionPhase, MainBaseConversionUnresolved};
    use v2k_game::main_base_conversion_live::FirstWorldMainBaseConversionUnresolved;

    let mut world = World::load(24);
    let mut oracle = World::load(24);
    let (_, source) = prepare_people_contact(&mut world);
    prepare_people_contact(&mut oracle);
    world.tasks = SpecializedActorTaskScheduler::new();
    let local = resolve_first_world_main_base_conversions(MainBaseConversionFrame {
        entities: &mut world.entities,
        actor_tasks: &mut world.tasks,
        model_pool: &world.session.cache,
        terrain: world.session.cache.terrain(),
        world_fx: &mut world.fx,
        notifications: &mut world.notifications,
        retail_tick: 20,
        world_style: RetailRuntimeValue::Known(3),
    })
    .expect_err("the real local contact requires its completed scheduler owner");
    assert!(
        matches!(local.unresolved, MainBaseConversionUnresolved::Host {
        candidate_id,
        phase: MainBaseConversionPhase::ExactPairContact,
        source: FirstWorldMainBaseConversionUnresolved::SourceTaskCustodyUnavailable { entity_id },
    } if candidate_id == source && entity_id == source)
    );
    world
        .entities
        .entity_mut(source)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(REMOTE_OWNED_STATE_BIT, REMOTE_OWNED_STATE_BIT);
    let collision = world.entity(source).collision.clone();
    let graph = ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .map(|slot| world.entity(source).actor_task_state(slot).copied());
    let before_order: Vec<_> = world.entities.retail_live_order_ids().collect();
    let ordinal = world.ordinal();
    let seed = world.fx.next_sub_d_allocation_seed();
    let notifications = format!("{:?}", world.notifications);
    let effects = world.fx.pending_event_count();
    let pass = resolve_first_world_main_base_conversions(MainBaseConversionFrame {
        entities: &mut world.entities,
        actor_tasks: &mut world.tasks,
        model_pool: &world.session.cache,
        terrain: world.session.cache.terrain(),
        world_fx: &mut world.fx,
        notifications: &mut world.notifications,
        retail_tick: 20,
        world_style: RetailRuntimeValue::Known(3),
    })
    .expect("4258DD remote gate precedes local task/metadata custody");
    let FirstWorldMainBaseConversionPass::Resolved(pass) = pass else {
        panic!("native Base");
    };
    let visit = pass
        .visits
        .iter()
        .find(|visit| visit.entity_id == source)
        .unwrap();
    assert!(visit.action_outcomes.is_empty());
    assert!(
        visit.suffix_outcome.is_none(),
        "bounded conversion does not own remote pair effects"
    );
    assert_eq!(world.entity(source).collision, collision);
    assert_eq!(
        ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .map(|slot| world.entity(source).actor_task_state(slot).copied()),
        graph
    );
    assert_eq!(
        world.entities.retail_live_order_ids().collect::<Vec<_>>(),
        before_order
    );
    assert_eq!(world.ordinal(), ordinal);
    assert_eq!(world.fx.next_sub_d_allocation_seed(), seed);
    assert_eq!(
        world.fx.next_shared_retail_random_u16(),
        oracle.fx.next_shared_retail_random_u16()
    );
    assert_eq!(world.fx.pending_event_count(), effects);
    assert_eq!(format!("{:?}", world.notifications), notifications);
    assert!(world
        .entities
        .pending_main_base_conversion_destroy_ids()
        .is_empty());
}
