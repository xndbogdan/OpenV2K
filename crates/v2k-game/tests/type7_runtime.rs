//! Finite native factory production, Type93 release and subsequent diver
//! staffing are distinct from the eight authored Type7 allocations.
use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    entity::{
        AuthoredPlayerArrival, AuthoredWorldConstruction, Entity, EntityConstructionResources,
        EntityManager,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    entity_scheduler::{
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    },
    factory_activation_live::{
        apply_prepared_factory_scientist_arrival, prepare_factory_scientist_arrival,
    },
    factory_production::{FactoryProductionPhase, FACTORY_OUTPUT_CONVERSION_GATE_RAW},
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    intro2_type66::{
        tick_intro2_type66_owner, Intro2Type66Frame, Intro2Type66Outcome, Intro2Type66Owner,
    },
    native_type86::{impact::NativeType86ImpactOutcome, Type86Outcome},
    session::GameSession,
    shared_actor_impact::{
        apply_shared_actor_particle_hit, SharedActorImpactFrame, SharedActorImpactOutcome,
    },
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact, WorldFx},
};

struct World {
    session: GameSession,
    entities: EntityManager,
    fx: WorldFx,
    tasks: SpecializedActorTaskScheduler,
    notifications: GameplayNotifications,
    static_damage: StaticDamageScheduler,
}
impl World {
    fn load(player: bool) -> Self {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").is_file(),
            "canonical retail corpus required"
        );
        let mut session = GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(34, 1).unwrap();
        let metadata = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(id, _)| {
                EntityTypeRuntimeMetadata::from_section12(
                    session.cache.global_entity_type(id).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let mut fx = WorldFx::new();
        let mut entities = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index: 22,
                type_metadata: &metadata,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
                },
                player_arrival: player.then_some(AuthoredPlayerArrival {
                    position_raw: [0; 3],
                    heading_raw: 0,
                }),
                retail_tick: 0,
            },
            &mut fx,
        )
        .unwrap();
        entities.cleanup_pending_actor_deferred_destroys();
        Self {
            session,
            entities,
            fx,
            tasks: SpecializedActorTaskScheduler::new(),
            notifications: GameplayNotifications::new(),
            static_damage: StaticDamageScheduler::new(),
        }
    }
    fn actor(&self, id: u32) -> &Entity {
        self.entities.iter_all().find(|e| e.id == id).unwrap()
    }
    fn factory(&self) -> u32 {
        self.entities
            .iter_all()
            .find(|e| e.entity_type == 66)
            .unwrap()
            .id
    }
    fn ordinal(&self) -> u16 {
        let RetailRuntimeValue::Known(ordinal) = self.entities.next_common_body_ordinal() else {
            panic!()
        };
        ordinal
    }
    fn assert_diver(&self, id: u32) {
        let entity = self.actor(id);
        assert_eq!(entity.entity_type, 7);
        assert_eq!(entity.capability_flags, 0x1404);
        assert_eq!(entity.model_slots, [Some(1249); 4]);
        assert!(entity.native_type86_runtime.is_some());
        assert!(
            entity.intro2_type8_runtime.is_none(),
            "diver does not borrow two-choice worker receipt"
        );
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(1500));
        let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
            panic!()
        };
        assert_eq!(animation.descriptor().attention_stop_sound_id, 106);
        assert_eq!(
            entity
                .type8_sub_d_frame_owner
                .unwrap()
                .classifier_cache()
                .origin(),
            RetailRuntimeValue::Unresolved
        );
    }
    fn prepare_finite_ejection(&mut self) -> u32 {
        let id = self.factory();
        assert_eq!(self.actor(id).authored_spawn_index, Some(11));
        assert!(self.actor(id).intro2_type66_runtime.is_some());
        assert!(matches!(
            self.actor(id).actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::WorkingFactory(_))
        ));
        // Controlled final production state on the actual nonzero-capacity
        // factory; all nearby predicates remain current-list native objects.
        let position = self.actor(id).position_raw();
        let others = self
            .entities
            .iter_all()
            .filter(|e| e.id != id)
            .map(|e| e.id)
            .collect::<Vec<_>>();
        for other in others {
            self.entities.entity_mut(other).unwrap().position = [
                position[0].wrapping_add(16000),
                position[1],
                position[2].wrapping_add(16000),
            ]
            .map(|raw| f32::from(raw) / 256.0);
        }
        let entity = self.entities.entity_mut(id).unwrap();
        let RetailRuntimeValue::Known(Some(base)) = &mut entity.base_factory_runtime else {
            panic!()
        };
        assert!(
            base.required_scientists > 0,
            "DarkReef's actual authored factory capacity"
        );
        base.current_scientists = base.required_scientists;
        let production = base.production.as_mut().unwrap();
        production.current_scientists_raw = i32::from(base.current_scientists);
        production.remaining_stock_raw = 0;
        production.phase = FactoryProductionPhase::WaitingForPickup;
        production.production_progress_micros_raw = production
            .production_threshold_micros_raw
            .wrapping_add(FACTORY_OUTPUT_CONVERSION_GATE_RAW)
            .wrapping_add(1);
        detailed(entity);
        id
    }
    fn eject(&mut self, owner: Intro2Type66Owner) -> Intro2Type66Outcome {
        tick_intro2_type66_owner(
            &mut self.entities,
            owner,
            Intro2Type66Frame {
                resources: &mut self.session.cache,
                world_fx: &mut self.fx,
                notifications: &mut self.notifications,
                static_damage: &mut self.static_damage,
                elapsed_micros: 0,
                retail_tick: 100,
                world_style_raw: 6,
                main_base_abort_active: false,
            },
        )
        .outcome
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
        assert!(pass.block.is_none(), "{:?}", pass.block);
        for outcome in pass.outcomes {
            if let SpecializedActorTaskProductionOutcome::NativeType86(outcome) = outcome {
                assert!(
                    matches!(
                        outcome,
                        Type86Outcome::Advanced {
                            terminal: false,
                            ..
                        } | Type86Outcome::Waiting { .. }
                    ),
                    "{outcome:?}"
                );
            }
        }
    }
}
fn detailed(entity: &mut Entity) {
    let flags =
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT;
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(flags | 0x800, flags);
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
}

#[v2k_test_support::retail_test]
fn type7_native_factory_ejection_materialiser_release_and_staffing_keep_one_allocation() {
    let mut world = World::load(false);
    let mut oracle = World::load(false);
    let factory = world.prepare_finite_ejection();
    oracle.prepare_finite_ejection();
    let source_receipt = world.actor(factory).intro2_type66_runtime;
    let order = world.entities.retail_live_order_ids().collect::<Vec<_>>();
    let ordinal = world.ordinal();
    let seed = world.fx.next_sub_d_allocation_seed();
    // Even a matching public factory id in another native manager is foreign.
    let foreign = Intro2Type66Owner::adopt(&oracle.entities, factory).unwrap();
    assert!(matches!(
        world.eject(foreign),
        Intro2Type66Outcome::Dropped { .. }
    ));
    assert_eq!(world.ordinal(), ordinal);
    assert_eq!(
        world.entities.retail_live_order_ids().collect::<Vec<_>>(),
        order
    );
    let owner = Intro2Type66Owner::adopt(&world.entities, factory).unwrap();
    let outcome = world.eject(owner);
    assert!(
        matches!(
            outcome,
            Intro2Type66Outcome::Advanced {
                callback_enabled: true,
                ..
            }
        ),
        "{outcome:?}"
    );
    assert_eq!(world.actor(factory).intro2_type66_runtime, source_receipt);
    assert_eq!(world.entities.factory_converted_output_births().len(), 1);
    let birth = world
        .entities
        .factory_converted_output_births()
        .last()
        .unwrap();
    let id = birth.entity_id();
    assert_eq!(birth.source_factory().entity_id, factory);
    assert_eq!(birth.spawn_request().entity_type, 7);
    assert_eq!(
        birth.selected_behavior_class_id(),
        6,
        "full source and no other nearby candidates"
    );
    oracle.fx.next_shared_retail_random_u16(); //20450.
    assert_eq!(
        birth.selector_rng_word(),
        oracle.fx.next_shared_retail_random_u16()
    );
    oracle.fx.next_shared_retail_random_u16(); //AD10 successful constructor.
    oracle.fx.next_shared_retail_random_u16(); //Type93 singleton selector.
    assert_eq!(
        world.fx.next_shared_retail_random_u16(),
        oracle.fx.next_shared_retail_random_u16()
    );
    world.assert_diver(id);
    assert_eq!(world.actor(id).authored_spawn_index, None);
    assert_eq!(
        world
            .actor(id)
            .type8_sub_d_frame_owner
            .unwrap()
            .classifier_cache()
            .stagger_counter(),
        seed
    );
    let proxy = world.actor(id).attached_to.unwrap();
    assert_eq!(world.actor(proxy).entity_type, 93);
    assert_eq!(world.actor(proxy).model_slots[0], Some(1249));
    let RetailRuntimeValue::Known(Some(rows)) = &world.actor(proxy).sub_j_attachment_runtime else {
        panic!()
    };
    assert_eq!(rows.ordered_entity_ids(), [id]);
    let after = world.entities.retail_live_order_ids().collect::<Vec<_>>();
    assert_eq!(&after[..order.len()], order);
    assert_eq!(&after[order.len()..], [id, proxy]);
    assert_eq!(world.ordinal(), ordinal.wrapping_add(2));
    assert_eq!(world.fx.next_sub_d_allocation_seed(), seed.wrapping_add(1));
    assert_eq!(
        world.actor(id).construction_stamp_at_0xb4,
        RetailRuntimeValue::Known(22u16.wrapping_mul(0x400).wrapping_add(ordinal))
    );
    let stamp = world.actor(id).construction_stamp_at_0xb4;
    assert!(matches!(
        world.actor(id).actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::None)
    ));
    assert_eq!(world.tasks.adopt_native_type86(&mut world.entities), 6);
    assert_eq!(world.tasks.adopt_native_type86(&mut world.entities), 0);
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
        if world.actor(id).attached_to.is_none() {
            break;
        }
    }
    assert_eq!(world.actor(id).attached_to, None);
    assert!(!world.entities.iter_all().any(|e| e.id == proxy));
    world.assert_diver(id);
    oracle.fx.next_shared_retail_random_u16(); //CE90 root selector.
    oracle.fx.next_shared_retail_random_u16(); //AD10 constructor.
    assert_eq!(
        world.fx.next_shared_retail_random_u16(),
        oracle.fx.next_shared_retail_random_u16()
    );
    assert_eq!(world.ordinal(), ordinal.wrapping_add(2));
    assert_eq!(world.fx.next_sub_d_allocation_seed(), seed.wrapping_add(1));
    let ids = world
        .entities
        .iter_all()
        .filter(|e| e.native_type86_runtime.is_some())
        .map(|e| e.id)
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 6, "five authored and one real dynamic diver");
    for &worker in &ids {
        detailed(world.entities.entity_mut(worker).unwrap());
    }
    assert_eq!(world.tasks.registered_len(), 6);
    assert_eq!(world.tasks.adopt_native_type86(&mut world.entities), 0);
    let position = world.actor(id).position_raw();
    for tick in 1..=30 {
        world.step(100 + tick * 5);
    }
    assert_ne!(world.actor(id).position_raw(), position);
    assert_eq!(world.actor(id).construction_stamp_at_0xb4, stamp);

    // Open one real staffing vacancy after ejection. C690 enters the actual
    // four-choice selector; do not hand-install a class54 or Type8 task.
    let position = world.actor(factory).position_raw();
    world.entities.entity_mut(id).unwrap().position =
        [position[0].wrapping_add(512), position[1], position[2]].map(|raw| f32::from(raw) / 256.0);
    let RetailRuntimeValue::Known(Some(base)) = &mut world
        .entities
        .entity_mut(factory)
        .unwrap()
        .base_factory_runtime
    else {
        panic!()
    };
    base.current_scientists = 0;
    base.production.as_mut().unwrap().current_scientists_raw = 0;
    for attempt in 0..32 {
        let hit = apply_shared_actor_particle_hit(
            SharedActorImpactFrame {
                resources: &world.session.cache,
                entities: &mut world.entities,
                world_fx: &mut world.fx,
                scheduler: &mut world.tasks,
                notifications: &mut world.notifications,
                retail_tick: 300 + attempt,
            },
            ParticleEntityImpact {
                source_particle_class: 38,
                impact_position_argument_va: 0,
                target_entity_id: id,
                position_world: [0.; 3],
                velocity_raw: [0; 3],
                damage: Some(BallisticDamageRequest {
                    packet: DamagePacket {
                        channels: [2, 3],
                        amounts_raw: [0, 0],
                    },
                    source_entity_type_at_birth: Some(10),
                    source_owner_id: Some(0x045f_0001),
                }),
            },
        )
        .expect("native four-choice impact route");
        assert!(
            matches!(
                hit,
                SharedActorImpactOutcome::Type86(NativeType86ImpactOutcome::Applied(_))
            ),
            "{hit:?}"
        );
        if matches!(world.actor(id).actor_task_state(ActorTaskSlot::Primary),Some(ActorTaskRuntime::GoToJob(task)) if task.target_id()==Some(factory))
        {
            break;
        }
    }
    assert!(
        matches!(world.actor(id).actor_task_state(ActorTaskSlot::Primary),Some(ActorTaskRuntime::GoToJob(task)) if task.target_id()==Some(factory))
    );
    world.step(400);
    world.entities.entity_mut(id).unwrap().position = position.map(|raw| f32::from(raw) / 256.0);
    let prepared = prepare_factory_scientist_arrival(&world.entities, factory, id).unwrap();
    let result = apply_prepared_factory_scientist_arrival(
        &mut world.entities,
        &mut world.fx,
        &mut world.notifications,
        401,
        prepared,
    )
    .unwrap();
    assert_eq!(result.scientist_id, id);
    assert_eq!(result.factory_runtime.current_scientists_raw, 1);
    assert_eq!(world.entities.pending_factory_scientist_destroy_ids(), [id]);
    assert!(prepare_factory_scientist_arrival(&world.entities, factory, id).is_err());
    assert_eq!(
        world.entities.cleanup_pending_factory_scientist_destroys(),
        [id]
    );
    assert!(!world.entities.iter_all().any(|e| e.id == id));
    assert_eq!(world.actor(factory).intro2_type66_runtime, source_receipt);
}

#[v2k_test_support::retail_test]
fn type7_dynamic_class45_keeps_full_constructor_rng_and_live_notification_before_materialising() {
    use v2k_game::{
        attract_attention::AttractAttentionResourceTextRequest,
        entity_behavior::{select_initial_behavior, BehaviorWeightRule},
    };
    let mut world = World::load(true);
    let factory = world.prepare_finite_ejection();
    let player = world.entities.player().unwrap().id;
    let [x, y, z] = world.actor(factory).position_raw();
    world.entities.entity_mut(player).unwrap().position =
        [x.wrapping_add(512), y, z].map(|raw| f32::from(raw) / 256.0);
    // The fixture advances the real process stream to exercise each BA40
    // parity deterministically; it never changes type choices or task fields.
    let choices = world
        .session
        .cache
        .global_entity_type(7)
        .unwrap()
        .behavior_choices
        .clone();
    let mut selected = None;
    for offset in 0..128 {
        let mut oracle = World::load(true).fx;
        for _ in 0..offset {
            oracle.next_shared_retail_random_u16();
        }
        oracle.next_shared_retail_random_u16(); //20450 before selection.
        let selector = oracle.next_shared_retail_random_u16();
        let selection = select_initial_behavior(
            &choices,
            |rule| match rule {
                BehaviorWeightRule::Always | BehaviorWeightRule::PlayerNearby => 1,
                _ => 0,
            },
            || u32::from(selector),
        )
        .unwrap()
        .unwrap();
        if selection.program.class_id == 45 {
            let parity = oracle.next_shared_retail_random_u16();
            let mut words = vec![parity];
            for _ in 0..(2 + usize::from(parity & 1 != 0)) {
                words.push(oracle.next_shared_retail_random_u16());
            }
            oracle.next_shared_retail_random_u16(); //Type93 singleton selector.
            selected = Some((selector, words, oracle));
            break;
        }
        world.fx.next_shared_retail_random_u16();
    }
    let (selector, words, mut oracle) = selected.expect("real stream reaches class45");
    world.fx.process_pending();
    world.fx.take_positional_sounds();
    // Only the dynamic BA40 receipt is outstanding at this controlled edge.
    world
        .notifications
        .drain_attract_attention_receipts(&mut world.entities, 0)
        .unwrap();
    world.notifications = GameplayNotifications::new();
    let mut expected = world.notifications.clone();
    expected.queue_factory_output_conversion(100);
    expected
        .queue_attract_attention_resource_text(
            AttractAttentionResourceTextRequest {
                event: 0x10,
                global_resource_id: 0xf0,
            },
            100,
        )
        .unwrap();
    let owner = Intro2Type66Owner::adopt(&world.entities, factory).unwrap();
    let outcome = world.eject(owner);
    assert!(
        matches!(
            outcome,
            Intro2Type66Outcome::Advanced {
                callback_enabled: true,
                ..
            }
        ),
        "{outcome:?}"
    );
    let birth = world
        .entities
        .factory_converted_output_births()
        .last()
        .unwrap();
    let id = birth.entity_id();
    assert_eq!(birth.selected_behavior_class_id(), 45);
    assert_eq!(birth.selector_rng_word(), selector);
    assert_eq!(birth.constructor_rng_words(), words);
    assert_eq!(
        world.fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
    world.assert_diver(id);
    assert_eq!(
        world
            .actor(world.actor(id).attached_to.unwrap())
            .entity_type,
        93
    );
    assert!(matches!(
        world.actor(id).actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::None)
    ));
    assert!(world
        .actor(id)
        .actor_task_state(ActorTaskSlot::Secondary)
        .is_none());
    assert!(world
        .actor(id)
        .actor_task_state(ActorTaskSlot::Tertiary)
        .is_none());
    assert!(world
        .entities
        .pending_fresh_level1_type9_resource_text_receipts()
        .is_empty());
    assert_eq!(
        world.notifications, expected,
        "resource16 uses the live factory tick, not a load timestamp"
    );
    world.fx.process_pending();
    let sounds = world
        .fx
        .take_positional_sounds()
        .iter()
        .map(|s| s.sound_id)
        .collect::<Vec<_>>();
    assert!(sounds.contains(&106), "class45 Sub-I cue: {sounds:?}");
    assert!(
        !sounds.contains(&72),
        "Type9 cue cannot replace the actual Type7 Sub-I cue"
    );
    assert_eq!(world.tasks.adopt_native_type86(&mut world.entities), 6);
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
        if world.actor(id).attached_to.is_none() {
            break;
        }
    }
    assert_eq!(world.actor(id).attached_to, None);
    assert!(matches!(
        world.actor(id).actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::OrdinaryType9Wander(_) | ActorTaskRuntime::SharedRetarget(_))
    ));
    world.assert_diver(id);
}

#[v2k_test_support::retail_test]
fn type7_c690_reselection_drains_event16_at_the_hit_tick() {
    use v2k_game::{
        attract_attention::AttractAttentionResourceTextRequest,
        entity_behavior::{select_initial_behavior, BehaviorWeightRule},
        impact_reaction::{
            IMPACT_REACTION_ENABLED_STATE_BIT, IMPACT_REACTION_SUPPRESSED_STATE_BIT,
        },
    };
    const HIT_TICK: u32 = 812;
    let mut world = World::load(true);
    let id = world
        .entities
        .iter_all()
        .find(|e| {
            e.entity_type == 7
                && matches!(
                    e.actor_task_state(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::OrdinaryType9Wander(_) | ActorTaskRuntime::GoToJob(_))
                )
        })
        .expect("an authored diver with a real C690 primary-hit callback")
        .id;
    let player = world.entities.player().unwrap().id;
    let [x, y, z] = world.actor(id).position_raw();
    let other_ids = world
        .entities
        .iter_all()
        .filter(|e| e.id != id && e.id != player)
        .map(|e| e.id)
        .collect::<Vec<_>>();
    for other in other_ids {
        world.entities.entity_mut(other).unwrap().position =
            [x.wrapping_add(16000), y, z.wrapping_add(16000)].map(|raw| f32::from(raw) / 256.0);
    }
    world.entities.entity_mut(player).unwrap().position =
        [x.wrapping_add(512), y, z].map(|raw| f32::from(raw) / 256.0);
    assert_eq!(world.tasks.adopt_native_type86(&mut world.entities), 5);
    world
        .notifications
        .drain_attract_attention_receipts(&mut world.entities, 0)
        .unwrap();
    world.notifications = GameplayNotifications::new();
    world.fx.process_pending();
    world.fx.take_positional_sounds();

    // Reach BA40 through the real C690 selector and shared process stream.
    // Unlike a new 104B0 allocation, reselection has no preceding 20450 word.
    let choices = world
        .session
        .cache
        .global_entity_type(7)
        .unwrap()
        .behavior_choices
        .clone();
    let mut selected = None;
    for offset in 0..128 {
        let mut oracle = World::load(true).fx;
        for _ in 0..offset {
            oracle.next_shared_retail_random_u16();
        }
        let selector = oracle.next_shared_retail_random_u16();
        let selection = select_initial_behavior(
            &choices,
            |rule| match rule {
                BehaviorWeightRule::Always | BehaviorWeightRule::PlayerNearby => 1,
                _ => 0,
            },
            || u32::from(selector),
        )
        .unwrap()
        .unwrap();
        if selection.program.class_id == 45 {
            let parity = oracle.next_shared_retail_random_u16();
            for _ in 0..(2 + usize::from(parity & 1 != 0)) {
                oracle.next_shared_retail_random_u16();
            }
            // 11030 follows C690 even for a zero-damage packet. Its three
            // angular words belong to the hit, not the notification drain.
            let reaction =
                world.actor(id).collision.state_flags_at_0x08.masked(
                    IMPACT_REACTION_ENABLED_STATE_BIT | IMPACT_REACTION_SUPPRESSED_STATE_BIT,
                );
            if reaction == RetailRuntimeValue::Known(IMPACT_REACTION_ENABLED_STATE_BIT) {
                for _ in 0..3 {
                    oracle.next_shared_retail_random_u16();
                }
            }
            selected = Some(oracle);
            break;
        }
        world.fx.next_shared_retail_random_u16();
    }
    let mut oracle = selected.expect("real stream reaches class45");
    let mut expected = GameplayNotifications::new();
    expected
        .queue_attract_attention_resource_text(
            AttractAttentionResourceTextRequest {
                event: 0x10,
                global_resource_id: 0xf0,
            },
            HIT_TICK as i32,
        )
        .unwrap();
    let hit = apply_shared_actor_particle_hit(
        SharedActorImpactFrame {
            resources: &world.session.cache,
            entities: &mut world.entities,
            world_fx: &mut world.fx,
            scheduler: &mut world.tasks,
            notifications: &mut world.notifications,
            retail_tick: HIT_TICK,
        },
        ParticleEntityImpact {
            source_particle_class: 38,
            impact_position_argument_va: 0,
            target_entity_id: id,
            position_world: [0.; 3],
            velocity_raw: [0; 3],
            damage: Some(BallisticDamageRequest {
                packet: DamagePacket {
                    channels: [2, 3],
                    amounts_raw: [0, 0],
                },
                source_entity_type_at_birth: Some(10),
                source_owner_id: Some(0x045f_0001),
            }),
        },
    )
    .expect("native four-choice impact route");
    assert!(
        matches!(
            hit,
            SharedActorImpactOutcome::Type86(NativeType86ImpactOutcome::Applied(_))
        ),
        "{hit:?}"
    );
    assert!(matches!(
        world.actor(id).actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::SharedRetarget(_))
    ));
    assert!(matches!(
        world.actor(id).actor_task_state(ActorTaskSlot::Tertiary),
        Some(ActorTaskRuntime::AttractAttentionCue(_))
    ));
    assert_eq!(world.notifications, expected);
    assert!(world
        .entities
        .pending_fresh_level1_type9_resource_text_receipts()
        .is_empty());
    assert_eq!(
        world.fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16(),
        "notification transfer cannot draw from the shared RNG"
    );
    world.fx.process_pending();
    let sounds = world
        .fx
        .take_positional_sounds()
        .iter()
        .map(|sound| sound.sound_id)
        .collect::<Vec<_>>();
    assert!(sounds.contains(&106), "real Type7 Sub-I cue: {sounds:?}");
    assert!(!sounds.contains(&72));
}
