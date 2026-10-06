use v2k_game::actor_task_dispatcher::ActorTaskRuntime;
use v2k_game::actor_task_owner::ActorTaskSlot;
use v2k_game::entity::{EntityManager, MainBaseConversionDestroyQueueOutcome, PlayerUpdateRequest};
use v2k_game::entity_collision_state::{
    EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord,
    DEFERRED_DESTROY_PENDING_STATE_BIT,
};
use v2k_game::factory_activation_live::{
    apply_prepared_factory_scientist_arrival, prepare_factory_scientist_arrival,
    retain_level_one_factory_remaining_stock, spawn_factory_converted_output,
    splice_level_one_factory_spawned_pickup, tick_level_one_factory_owner,
    FactoryActivationLiveError, FactoryActivationLiveEvent,
};
use v2k_game::factory_pair_suffix::{FactoryPairCandidateBehavior, FactoryPairPhysical};
use v2k_game::factory_production::FactoryProductionPhase;
use v2k_game::factory_production_live::{
    converted_output_entity_type, FactoryEntitySpawnRequest, FactoryPickupPresence,
    FactoryRequestedEntityHandle, FactorySpawnedEntity,
};
use v2k_game::gameplay_notifications::{GameplayNotifications, TextTypewriterCadence};
use v2k_game::hover::HoverFrameForces;
use v2k_game::main_base_conversion::MainBaseReplacementSpawn;
use v2k_game::main_base_conversion_runtime::{
    apply_prepared_first_world_main_base_replacement, prepare_first_world_main_base_replacement,
};
use v2k_game::session::GameSession;
use v2k_game::vtol::{VehicleFrameForces, VtolBoost, VtolHeightPolicy};
use v2k_game::world_fx::WorldFx;

const FACTORY_SPAWN_INDEX: usize = 23;
const MAIN_BASE_SPAWN_INDEX: usize = 6;
const FACTORY_POSITION_RAW: [i16; 3] = [0x5700, -0x0300, 0x3A00];
const FRAME_MICROS: u32 = 20_000;

fn fresh_level_one() -> (GameSession, EntityManager) {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("initialize V2000 retail corpus");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load Level-1 auxiliary overlay");
    let type_models = session.cache.global_entity_model_table();
    let type_metadata = type_models
        .iter()
        .copied()
        .enumerate()
        .map(|(entity_type, model_slots)| {
            session
                .cache
                .global_entity_type(entity_type)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..EntityTypeRuntimeMetadata::default()
                })
        })
        .collect::<Vec<_>>();
    session
        .load_level_by_id(13, 1)
        .expect("load Level 1 from retail corpus");
    let mut world_fx = WorldFx::new();
    let manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
        session
            .cache
            .level_desc()
            .expect("loaded Level-1 descriptor"),
        &type_metadata,
        Some(session.cache.terrain().expect("loaded Level-1 terrain")),
        0,
        &mut world_fx,
    )
    .expect("fresh type-17 birth publication");
    (session, manager)
}

fn spawn_two_factory_bound_scientists(
    session: &GameSession,
    entities: &mut EntityManager,
    world_fx: &mut WorldFx,
    factory_id: u32,
    main_base_id: u32,
) -> [u32; 2] {
    let sources = entities
        .iter_all()
        .filter(|entity| entity.entity_type == 9)
        .map(|entity| entity.id)
        .collect::<Vec<_>>();
    let mut scientists = Vec::new();
    for source_id in sources {
        let request = MainBaseReplacementSpawn {
            source_entity_id: source_id,
            main_base_entity_id: main_base_id,
            replacement_type: 8,
            // The live movement consumer is deliberately outside this test.
            // Inject the explicit accepted-arrival position while retaining
            // the real Main Base allocator and Go-To-Job constructor.
            position_raw: FACTORY_POSITION_RAW,
        };
        let Ok(plan) = prepare_first_world_main_base_replacement(
            entities,
            Some(session.cache.terrain().expect("Level-1 terrain")),
            request,
            0,
        ) else {
            continue;
        };
        assert_eq!(
            entities.queue_main_base_conversion_destroy(source_id),
            MainBaseConversionDestroyQueueOutcome::Queued { source_id }
        );
        let expected_sub_d_seed = world_fx.next_sub_d_allocation_seed();
        let spawned = apply_prepared_first_world_main_base_replacement(
            entities,
            Some(session.cache.terrain().expect("Level-1 terrain")),
            world_fx,
            plan,
        );
        let replacement = entities
            .iter_all()
            .find(|entity| entity.id == spawned.replacement_id)
            .expect("published Main Base replacement");
        assert_eq!(
            replacement
                .type8_sub_d_frame_owner
                .expect("Main Base conversion retains its process allocation")
                .classifier_cache()
                .stagger_counter(),
            expected_sub_d_seed
        );
        assert!(replacement.type8_sub_d_runtime.is_some());
        assert_eq!(
            replacement.physical_body_basis_q31(),
            RetailRuntimeValue::Known(
                v2k_game::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(0, 0, 0)
            ),
        );
        if matches!(
            replacement.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::GoToJob(state)) if state.target_id() == Some(factory_id)
        ) {
            scientists.push(spawned.replacement_id);
            if scientists.len() == 2 {
                break;
            }
        }
    }
    scientists
        .try_into()
        .expect("retail Level 1 supplies two factory-bound scientist replacements")
}

fn staff_and_deliver_first_level_one_product(
    session: &GameSession,
    entities: &mut EntityManager,
    notifications: &mut GameplayNotifications,
    world_fx: &mut WorldFx,
    factory_id: u32,
    main_base_id: u32,
) -> FactorySpawnedEntity {
    let terrain = session.cache.terrain().expect("Level-1 terrain");
    let [first_scientist, second_scientist] =
        spawn_two_factory_bound_scientists(session, entities, world_fx, factory_id, main_base_id);
    let first = prepare_factory_scientist_arrival(entities, factory_id, first_scientist)
        .expect("first arrival");
    apply_prepared_factory_scientist_arrival(entities, world_fx, notifications, 100, first)
        .expect("first delivery");
    tick_level_one_factory_owner(
        entities,
        terrain,
        world_fx,
        notifications,
        101,
        factory_id,
        FRAME_MICROS,
    )
    .expect("1/2 tail");
    let second = prepare_factory_scientist_arrival(entities, factory_id, second_scientist)
        .expect("second arrival");
    apply_prepared_factory_scientist_arrival(entities, world_fx, notifications, 102, second)
        .expect("second delivery");
    let mut product = None;
    for frame in 1..=300_u32 {
        let outcome = tick_level_one_factory_owner(
            entities,
            terrain,
            world_fx,
            notifications,
            102 + frame,
            factory_id,
            FRAME_MICROS,
        )
        .expect("phase-0 frame");
        if frame == 300 {
            product = outcome.product;
        }
    }
    let product = product.expect("six-second type-61");
    for frame in 1..=250_u32 {
        let outcome = tick_level_one_factory_owner(
            entities,
            terrain,
            world_fx,
            notifications,
            402 + frame,
            factory_id,
            FRAME_MICROS,
        )
        .expect("phase-1 frame");
        assert_eq!(
            outcome.production.phase,
            if frame < 250 {
                FactoryProductionPhase::Delivering
            } else {
                FactoryProductionPhase::WaitingForPickup
            }
        );
    }
    product
}

#[v2k_test_support::retail_test]
fn real_level_one_arrivals_complete_delivery_and_both_finite_worker_ejections() {
    let (session, mut entities) = fresh_level_one();
    let factory_id = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(FACTORY_SPAWN_INDEX))
        .expect("authored Level-1 Working Factory")
        .id;
    let main_base_id = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(MAIN_BASE_SPAWN_INDEX))
        .expect("authored Level-1 Main Base")
        .id;
    let mut world_fx = WorldFx::new();
    let mut notifications = GameplayNotifications::new();
    let terrain = session.cache.terrain().expect("Level-1 terrain");
    let [first_scientist, second_scientist] = spawn_two_factory_bound_scientists(
        &session,
        &mut entities,
        &mut world_fx,
        factory_id,
        main_base_id,
    );
    let first_arrival_position_raw = entities
        .iter_all()
        .find(|entity| entity.id == first_scientist)
        .expect("first live scientist")
        .position_raw();
    let second_arrival_position_raw = entities
        .iter_all()
        .find(|entity| entity.id == second_scientist)
        .expect("second live scientist")
        .position_raw();
    world_fx.process_pending();
    let particles_before_delivery = world_fx.particle_count();
    assert_eq!(world_fx.pending_event_count(), 0);

    // Two independently prepared plans reserve the same factory generation.
    // Applying the first must make the second stale before it can replay the
    // class-0x33 effect or staffing mutation.
    let first = prepare_factory_scientist_arrival(&entities, factory_id, first_scientist)
        .expect("first explicit arrival");
    let stale = prepare_factory_scientist_arrival(&entities, factory_id, first_scientist)
        .expect("parallel preflight at the same generation");
    let first_outcome = apply_prepared_factory_scientist_arrival(
        &mut entities,
        &mut world_fx,
        &mut notifications,
        100,
        first,
    )
    .expect("first scientist delivery");
    assert_eq!(first_outcome.factory_runtime.current_scientists_raw, 1);
    assert_eq!(
        first_outcome.visit_suffix.candidate_behavior,
        FactoryPairCandidateBehavior::SkippedNull
    );
    assert_eq!(
        first_outcome.visit_suffix.physical,
        FactoryPairPhysical::ClearedByA300
    );
    assert_eq!(
        first_outcome.events,
        [
            FactoryActivationLiveEvent::Operation33 {
                position_raw: first_arrival_position_raw,
                source_factory_id: factory_id,
            },
            FactoryActivationLiveEvent::DeliveryHudResource { event_id: 5 },
            FactoryActivationLiveEvent::StaffingCommitted {
                before: 0,
                after: 1,
            },
            FactoryActivationLiveEvent::ScientistDestroyQueued {
                scientist_id: first_scientist,
            },
        ]
    );
    assert_eq!(world_fx.pending_event_count(), 1);
    assert_eq!(
        apply_prepared_factory_scientist_arrival(
            &mut entities,
            &mut world_fx,
            &mut notifications,
            100,
            stale,
        ),
        Err(FactoryActivationLiveError::FactoryLeaseStale { factory_id })
    );
    assert_eq!(world_fx.pending_event_count(), 1);
    world_fx.process_pending();
    assert_eq!(world_fx.particle_count(), particles_before_delivery + 1);

    // The under-capacity owner takes its common status tail without advancing
    // phase-0 production, publishing the first occupied pole as 1/2.
    let understaffed = tick_level_one_factory_owner(
        &mut entities,
        terrain,
        &mut world_fx,
        &mut notifications,
        101,
        factory_id,
        FRAME_MICROS,
    )
    .expect("one-scientist owner frame");
    assert_eq!(understaffed.production.production_progress_micros_raw, 0);
    let factory = entities
        .iter_all()
        .find(|entity| entity.id == factory_id)
        .expect("live factory");
    let RetailRuntimeValue::Known(Some(factory_state)) = factory.base_factory_runtime else {
        panic!("live factory runtime");
    };
    assert_eq!(
        (
            factory_state.current_scientists,
            factory_state.required_scientists
        ),
        (1, 2)
    );

    let second = prepare_factory_scientist_arrival(&entities, factory_id, second_scientist)
        .expect("second explicit arrival");
    let second_outcome = apply_prepared_factory_scientist_arrival(
        &mut entities,
        &mut world_fx,
        &mut notifications,
        102,
        second,
    )
    .expect("second scientist delivery");
    assert_eq!(second_outcome.factory_runtime.current_scientists_raw, 2);
    assert_eq!(
        second_outcome.events,
        [
            FactoryActivationLiveEvent::Operation33 {
                position_raw: second_arrival_position_raw,
                source_factory_id: factory_id,
            },
            FactoryActivationLiveEvent::DeliveryHudResource { event_id: 5 },
            FactoryActivationLiveEvent::StaffingCommitted {
                before: 1,
                after: 2,
            },
            FactoryActivationLiveEvent::CapacityHudResource { event_id: 2 },
            FactoryActivationLiveEvent::ScientistDestroyQueued {
                scientist_id: second_scientist,
            },
        ]
    );
    let mut capacity_cadence = TextTypewriterCadence::default();
    let capacity_presentation =
        notifications.presentation(142, &mut capacity_cadence, |id| match id {
            0xE3 => Some("\t\t<*,3000, 4,30, *>Factory fully staffed"),
            _ => None,
        });
    assert_eq!(
        capacity_presentation
            .lines
            .iter()
            .map(|line| line.string_id)
            .collect::<Vec<_>>(),
        [0xE3]
    );
    let factory = entities
        .iter_all()
        .find(|entity| entity.id == factory_id)
        .expect("live factory");
    let RetailRuntimeValue::Known(Some(factory_state)) = factory.base_factory_runtime else {
        panic!("live factory runtime");
    };
    assert_eq!(
        factory_state.current_scientists, 1,
        "delivery mutates production staffing; the next owner tail publishes the pole"
    );
    assert_eq!(
        entities.pending_factory_scientist_destroy_ids(),
        [first_scientist, second_scientist]
    );
    for scientist_id in [first_scientist, second_scientist] {
        let scientist = entities
            .iter_all()
            .find(|entity| entity.id == scientist_id)
            .expect("queued scientist remains live through callback unwind");
        assert_eq!(
            scientist
                .collision
                .state_flags_at_0x08
                .masked(DEFERRED_DESTROY_PENDING_STATE_BIT),
            RetailRuntimeValue::Known(DEFERRED_DESTROY_PENDING_STATE_BIT)
        );
    }
    entities.update(PlayerUpdateRequest {
        body_pitch_roll_raw: [0; 2],
        elapsed_micros: 0,
        terrain: session.cache.terrain().expect("Level-1 terrain"),
        sea_level: None,
        retail_tick: 0,
        water_response_selectors: [6; 8],
        attached_cargo_mass: 0,
        active_model_half_radius_raw: 0,
        active_model_collision_radius_raw: 0,
        frame: VehicleFrameForces::Hover(HoverFrameForces::default()),
        vtol_boost: VtolBoost::Inactive,
        vtol_height_policy: VtolHeightPolicy::RetailAttenuation,
    });
    assert!(entities.pending_factory_scientist_destroy_ids().is_empty());
    assert!(entities
        .iter_all()
        .all(|entity| entity.id != first_scientist && entity.id != second_scientist));

    let product_count_before = entities
        .iter_all()
        .filter(|entity| entity.entity_type == 61)
        .count();
    let mut production_world_fx = WorldFx::new();
    let mut production_rng_oracle = WorldFx::new();
    let expected_product_selector_word = production_rng_oracle.next_shared_retail_random_u16();
    for frame in 1..=299_u32 {
        let outcome = tick_level_one_factory_owner(
            &mut entities,
            terrain,
            &mut production_world_fx,
            &mut notifications,
            102 + frame,
            factory_id,
            FRAME_MICROS,
        )
        .expect("captured phase-0 owner frame");
        assert!(outcome.product.is_none());
        assert_eq!(
            outcome.production.production_progress_micros_raw,
            frame as i32 * FRAME_MICROS as i32
        );
        if frame == 1 {
            let factory = entities
                .iter_all()
                .find(|entity| entity.id == factory_id)
                .expect("live factory");
            let RetailRuntimeValue::Known(Some(state)) = factory.base_factory_runtime else {
                panic!("live factory runtime");
            };
            assert_eq!(
                (state.current_scientists, state.required_scientists),
                (2, 2)
            );
        }
    }

    let product_frame = tick_level_one_factory_owner(
        &mut entities,
        terrain,
        &mut production_world_fx,
        &mut notifications,
        402,
        factory_id,
        FRAME_MICROS,
    )
    .expect("six-second product frame");
    let product = product_frame.product.expect("one real product allocation");
    assert_eq!(product_frame.production.remaining_stock_raw, 0);
    assert_eq!(
        product_frame.production.production_progress_micros_raw,
        6_000_000
    );
    assert_eq!(
        product_frame.production.phase,
        FactoryProductionPhase::Delivering
    );
    assert_eq!(
        entities
            .iter_all()
            .filter(|entity| entity.entity_type == 61)
            .count(),
        product_count_before + 1
    );
    assert_eq!(
        entities.retail_live_order_ids().last(),
        Some(product.entity_id.get())
    );
    let product_entity = entities
        .iter_all()
        .find(|entity| entity.id == product.entity_id.get())
        .expect("tail-appended factory product");
    assert_eq!(product_entity.power_up_payload_packed, Some(0x0001_F412));
    assert_eq!(product_entity.position_raw(), FACTORY_POSITION_RAW);
    assert_eq!(
        product_entity.collision.state_flags_at_0x08,
        RetailStateWord::exact(0x0E40_8805)
    );
    assert_eq!(
        product_entity.collision.recent_relation_id_at_0x60,
        RetailRuntimeValue::Known(Some(factory_id))
    );
    let birth = product_entity
        .factory_type61_birth_provenance()
        .expect("factory product retains its exact constructor receipt");
    assert_eq!(birth.selector_rng_word(), expected_product_selector_word);
    assert_eq!(birth.source_factory().entity_id, factory_id);
    let spawn_request = birth.spawn_request();
    assert_eq!(
        spawn_request.requested_handle,
        FactoryRequestedEntityHandle::Allocate
    );
    assert_eq!(spawn_request.entity_type, 61);
    assert_eq!(spawn_request.position_raw, FACTORY_POSITION_RAW);
    assert_eq!(spawn_request.spawn_parameter_6, 0x0001_F412);
    assert_eq!(
        production_world_fx.next_shared_retail_random_u16(),
        production_rng_oracle.next_shared_retail_random_u16(),
        "299 non-spawn frames draw nothing and the product constructor draws once"
    );

    let factory = entities
        .iter_all()
        .find(|entity| entity.id == factory_id)
        .expect("live factory");
    let RetailRuntimeValue::Known(Some(factory_state)) = factory.base_factory_runtime else {
        panic!("live factory runtime");
    };
    let owner = factory_state.live_owner.expect("live owner sidecar");
    assert_eq!(
        birth.source_factory().allocation_identity,
        owner.allocation_identity
    );
    assert_eq!(
        owner.state_version,
        birth.source_factory().state_version + 1,
        "the retained source version is the product-frame entry lease"
    );
    assert_eq!(owner.animation_state_raw, 3);
    assert_eq!(owner.lifetime_at_0x74_raw, 0);
    assert_eq!(owner.entry_flags_at_0x84_raw & 4, 4);
    assert_eq!(
        factory.collision.state_flags_at_0x08.masked(0x80),
        RetailRuntimeValue::Known(0x80)
    );
    assert!(product_frame
        .events
        .contains(&FactoryActivationLiveEvent::DirectText { string_id: 0xD1 }));
    assert!(product_frame
        .events
        .contains(&FactoryActivationLiveEvent::ProductOwnerLinked {
            product_id: product.entity_id.get(),
            factory_id,
        }));

    let mut cadence = TextTypewriterCadence::default();
    let presentation = notifications.presentation(442, &mut cadence, |id| match id {
        0xD1 => Some("\t\t<*,3000, 4,30, *>Power-up ready at factory"),
        _ => None,
    });
    assert_eq!(
        presentation
            .lines
            .iter()
            .map(|line| line.string_id)
            .collect::<Vec<_>>(),
        [0xD1]
    );

    let entity_count_after_product = entities.iter_all().count();
    for frame in 1..=249_u32 {
        let outcome = tick_level_one_factory_owner(
            &mut entities,
            terrain,
            &mut production_world_fx,
            &mut notifications,
            402 + frame,
            factory_id,
            FRAME_MICROS,
        )
        .expect("captured phase-1 delivery frame");
        assert!(outcome.product.is_none());
        assert_eq!(outcome.production.phase, FactoryProductionPhase::Delivering);
        assert_eq!(
            outcome.production.delivery_progress_micros_raw,
            frame as i32 * FRAME_MICROS as i32
        );
        assert_eq!(outcome.production.remaining_stock_raw, 0);
        assert!(outcome.status_published);
        assert!(outcome.dirty_flag_written);
        assert!(matches!(
            outcome.events.as_slice(),
            [
                FactoryActivationLiveEvent::StatusPublished(_),
                FactoryActivationLiveEvent::EntityMarkedDirty { flag: 0x80 }
            ]
        ));
    }

    let delivery_completion = tick_level_one_factory_owner(
        &mut entities,
        terrain,
        &mut production_world_fx,
        &mut notifications,
        652,
        factory_id,
        FRAME_MICROS,
    )
    .expect("250th delivery frame enters phase 2");
    assert!(delivery_completion.product.is_none());
    assert_eq!(
        delivery_completion.production.delivery_progress_micros_raw,
        5_000_000
    );
    assert_eq!(
        delivery_completion.production.phase,
        FactoryProductionPhase::WaitingForPickup
    );
    assert_eq!(delivery_completion.production.remaining_stock_raw, 0);
    assert!(delivery_completion.status_published);
    assert!(delivery_completion.dirty_flag_written);
    assert!(matches!(
        delivery_completion.events.as_slice(),
        [
            FactoryActivationLiveEvent::AnimationTransition(transition),
            FactoryActivationLiveEvent::StatusPublished(_),
            FactoryActivationLiveEvent::EntityMarkedDirty { flag: 0x80 }
        ] if transition.previous_state_raw == 3 && transition.next_state_raw == 1
    ));
    assert_eq!(entities.iter_all().count(), entity_count_after_product);
    assert_eq!(
        entities
            .iter_all()
            .filter(|entity| entity.entity_type == 61)
            .count(),
        product_count_before + 1
    );
    let product_entity = entities
        .iter_all()
        .find(|entity| entity.id == product.entity_id.get())
        .expect("the delivered product remains the same live allocation");
    assert_eq!(
        product_entity.collision.recent_relation_id_at_0x60,
        RetailRuntimeValue::Known(Some(factory_id))
    );
    let factory = entities
        .iter_all()
        .find(|entity| entity.id == factory_id)
        .expect("live factory after delivery");
    let RetailRuntimeValue::Known(Some(factory_state)) = factory.base_factory_runtime else {
        panic!("live factory runtime after delivery");
    };
    assert_eq!(
        factory_state.production,
        Some(delivery_completion.production)
    );
    let owner_after_delivery = factory_state.live_owner.expect("live owner after delivery");
    assert_eq!(owner_after_delivery.animation_state_raw, 1);
    assert_eq!(
        owner_after_delivery.state_version,
        owner.state_version + 250
    );
    assert_eq!(
        owner_after_delivery.next_transaction_id_raw,
        owner.next_transaction_id_raw + 500
    );
    assert_eq!(
        production_world_fx.next_shared_retail_random_u16(),
        production_rng_oracle.next_shared_retail_random_u16(),
        "phase-1 delivery consumes no constructor or presentation RNG"
    );

    for frame in 1..=176_u32 {
        let outcome = tick_level_one_factory_owner(
            &mut entities,
            terrain,
            &mut production_world_fx,
            &mut notifications,
            652 + frame,
            factory_id,
            FRAME_MICROS,
        )
        .expect("pre-conversion finite-stock phase-2 frame");
        assert!(outcome.product.is_none());
        assert_eq!(
            outcome.production.phase,
            FactoryProductionPhase::WaitingForPickup
        );
        assert_eq!(
            outcome.production.production_progress_micros_raw,
            6_000_000 + frame as i32 * FRAME_MICROS as i32
        );
        assert_eq!(outcome.production.delivery_progress_micros_raw, 5_000_000);
        assert_eq!(outcome.production.remaining_stock_raw, 0);
        assert_eq!(outcome.production.current_scientists_raw, 2);
        assert_eq!(outcome.production.scientist_capacity_raw, 2);
        assert!(outcome.status_published);
        assert!(!outcome.dirty_flag_written);
        assert!(matches!(
            outcome.events.as_slice(),
            [FactoryActivationLiveEvent::StatusPublished(_)]
        ));
    }

    let factory = entities
        .iter_all()
        .find(|entity| entity.id == factory_id)
        .expect("live factory at the conversion-action boundary");
    let RetailRuntimeValue::Known(Some(phase2_state)) = factory.base_factory_runtime else {
        panic!("live factory runtime at the conversion-action boundary");
    };
    let phase2_production = phase2_state
        .production
        .expect("phase-2 production state remains live");
    assert_eq!(phase2_production.production_progress_micros_raw, 9_520_000);
    let owner_after_wait = phase2_state
        .live_owner
        .expect("live owner after phase-2 wait");
    assert_eq!(owner_after_wait.animation_state_raw, 1);
    assert_eq!(
        owner_after_wait.state_version,
        owner_after_delivery.state_version + 176
    );
    assert_eq!(
        owner_after_wait.next_transaction_id_raw,
        owner_after_delivery.next_transaction_id_raw + 352
    );
    assert_eq!(entities.iter_all().count(), entity_count_after_product);
    assert!(entities
        .iter_all()
        .any(|entity| entity.id == product.entity_id.get() && entity.entity_type == 61));
    assert_eq!(
        production_world_fx.next_shared_retail_random_u16(),
        production_rng_oracle.next_shared_retail_random_u16(),
        "the finite-stock conversion wait consumes no RNG"
    );

    let type8_count_before = entities
        .iter_all()
        .filter(|entity| entity.entity_type == 8)
        .count();
    let type93_count_before = entities
        .iter_all()
        .filter(|entity| entity.entity_type == 93)
        .count();
    let births_before = entities.factory_converted_output_births().len();
    let request_position_raw = [0x5700, -0x0300, 0x3906];
    let request_position_world = request_position_raw.map(|raw| f32::from(raw) / 256.0);

    for ejection_index in 0..2_usize {
        let expected_sub_d_seed = production_world_fx.next_sub_d_allocation_seed();
        let _component_sub_a_word = production_rng_oracle.next_shared_retail_random_u16();
        let expected_selector_word = production_rng_oracle.next_shared_retail_random_u16();
        let expected_constructor_word = production_rng_oracle.next_shared_retail_random_u16();
        let _expected_materialiser_selector_word =
            production_rng_oracle.next_shared_retail_random_u16();
        let outcome = tick_level_one_factory_owner(
            &mut entities,
            terrain,
            &mut production_world_fx,
            &mut notifications,
            829 + ejection_index as u32,
            factory_id,
            FRAME_MICROS,
        )
        .expect("finite-stock worker ejection");
        let output = outcome
            .converted_output
            .expect("successful Type-8 output allocation");
        let materialiser = outcome
            .materialiser
            .expect("successful Type-93 materialiser allocation");
        assert!(outcome.product.is_none());
        assert_eq!(
            outcome.production.phase,
            FactoryProductionPhase::WaitingForPickup
        );
        assert_eq!(outcome.production.remaining_stock_raw, 0);
        assert_eq!(outcome.production.production_progress_micros_raw, 9_520_000);
        assert_eq!(
            outcome.production.current_scientists_raw,
            1 - ejection_index as i32
        );
        assert_eq!(
            outcome.production.scientist_capacity_raw,
            1 - ejection_index as i32
        );
        assert!(outcome.status_published);
        assert!(!outcome.dirty_flag_written);
        assert_eq!(
            outcome.events[0],
            FactoryActivationLiveEvent::ConversionHudResource { resource_id: 0x16 }
        );
        assert_eq!(
            outcome.events[1],
            FactoryActivationLiveEvent::ConvertedOutputSpawned { output }
        );
        assert_eq!(
            outcome.events[2],
            FactoryActivationLiveEvent::ConvertedOutputOwnerLinked {
                output_id: output.entity_id.get(),
                factory_id,
            }
        );
        assert_eq!(
            outcome.events[3],
            FactoryActivationLiveEvent::MaterialiserSpawned { materialiser }
        );
        assert_eq!(
            outcome.events[4],
            FactoryActivationLiveEvent::MaterialiserOutputLinked {
                materialiser_id: materialiser.entity_id.get(),
                output_id: output.entity_id.get(),
            }
        );
        assert_eq!(
            outcome.events[5],
            FactoryActivationLiveEvent::ConversionPositionalSound {
                sound_id: 8,
                position_raw: request_position_raw,
            }
        );
        assert!(matches!(
            outcome.events.get(6),
            Some(FactoryActivationLiveEvent::StatusPublished(_))
        ));
        assert_eq!(outcome.events.len(), 7);

        let birth = &entities.factory_converted_output_births()[births_before + ejection_index];
        assert_eq!(birth.entity_id(), output.entity_id.get());
        assert_eq!(birth.source_factory().entity_id, factory_id);
        assert_eq!(birth.spawn_request().position_raw, request_position_raw);
        assert_eq!(birth.selector_rng_word(), expected_selector_word);
        assert_eq!(birth.constructor_rng_words(), &[expected_constructor_word]);
        assert_eq!(birth.selected_behavior_class_id(), 6);
        let output_entity = entities
            .iter_all()
            .find(|entity| entity.id == output.entity_id.get())
            .expect("factory-ejected Type-8 remains live");
        assert_eq!(
            output_entity
                .type8_sub_d_frame_owner
                .expect("native factory worker allocation")
                .classifier_cache()
                .stagger_counter(),
            expected_sub_d_seed,
        );
        assert!(output_entity.type8_sub_d_runtime.is_some());
        assert_eq!(
            output_entity.physical_body_basis_q31(),
            RetailRuntimeValue::Known(
                v2k_game::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(0, 0, 0)
            ),
        );
        assert_eq!(output_entity.position_raw(), request_position_raw);
        assert_eq!(
            output_entity.attached_to,
            Some(materialiser.entity_id.get())
        );
        assert_eq!(
            output_entity.collision.recent_relation_id_at_0x60,
            RetailRuntimeValue::Known(Some(factory_id))
        );
        assert_eq!(
            output_entity
                .collision
                .state_flags_at_0x08
                .masked(0x0000_1000),
            RetailRuntimeValue::Known(0x0000_1000)
        );
        assert_eq!(
            output_entity
                .collision
                .state_flags_at_0x08
                .masked(0x0004_0000),
            RetailRuntimeValue::Known(0)
        );
        assert!(matches!(
            output_entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::None)
        ));
        let materialiser_entity = entities
            .iter_all()
            .find(|entity| entity.id == materialiser.entity_id.get())
            .expect("factory Type-93 remains live");
        assert_eq!(
            materialiser_entity.position_raw(),
            [0x5700, -0x02ff, 0x3906]
        );
        assert_eq!(
            materialiser_entity.model_slots[0],
            output_entity.model_index
        );
        assert_eq!(
            materialiser_entity.model_slots[2],
            output_entity.model_index
        );
        let RetailRuntimeValue::Known(Some(sub_j)) = &materialiser_entity.sub_j_attachment_runtime
        else {
            panic!("factory Type-93 owns exact Sub-J custody")
        };
        assert_eq!(sub_j.ordered_entity_ids(), [output.entity_id.get()]);
        assert_eq!(
            production_world_fx.next_shared_retail_random_u16(),
            production_rng_oracle.next_shared_retail_random_u16(),
            "each successful ejection consumes Type-8 selector/suffix then Type-93 selector"
        );
    }

    assert_eq!(
        entities
            .iter_all()
            .filter(|entity| entity.entity_type == 8)
            .count(),
        type8_count_before + 2
    );
    assert_eq!(
        entities
            .iter_all()
            .filter(|entity| entity.entity_type == 93)
            .count(),
        type93_count_before + 2
    );
    assert_eq!(entities.iter_all().count(), entity_count_after_product + 4);
    assert_eq!(production_world_fx.pending_event_count(), 2);
    production_world_fx.process_pending();
    let conversion_sounds = production_world_fx.take_positional_sounds();
    assert_eq!(conversion_sounds.len(), 2);
    assert!(conversion_sounds.iter().all(|sound| {
        sound.sound_id == 8
            && sound.position == request_position_world
            && sound.frequency_q16 == 0x1_0000
    }));

    let idle = tick_level_one_factory_owner(
        &mut entities,
        terrain,
        &mut production_world_fx,
        &mut notifications,
        831,
        factory_id,
        FRAME_MICROS,
    )
    .expect("zero-capacity factory enters phase 4");
    assert_eq!(idle.production.phase, FactoryProductionPhase::IdleEmpty);
    assert_eq!(idle.production.current_scientists_raw, 0);
    assert_eq!(idle.production.scientist_capacity_raw, 0);
    assert_eq!(idle.production.production_progress_micros_raw, 0);
    assert!(idle.converted_output.is_none());
    assert!(idle.materialiser.is_none());
    assert!(matches!(
        idle.events.as_slice(),
        [FactoryActivationLiveEvent::StatusPublished(_)]
    ));
    assert_eq!(
        production_world_fx.next_shared_retail_random_u16(),
        production_rng_oracle.next_shared_retail_random_u16(),
        "phase-4 entry consumes no constructor RNG"
    );

    let mut conversion_cadence = TextTypewriterCadence::default();
    let conversion_presentation =
        notifications.presentation(871, &mut conversion_cadence, |id| match id {
            0xF6 => Some("\t\t<*,3000, 4,30, *>Worker materialised"),
            _ => None,
        });
    assert_eq!(
        conversion_presentation
            .lines
            .iter()
            .map(|line| line.string_id)
            .collect::<Vec<_>>(),
        [0xF6]
    );
}

#[v2k_test_support::retail_test]
fn factory_terminal_progressive_death_queues_the_live_type61_crate() {
    let (session, mut entities) = fresh_level_one();
    let factory_id = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(FACTORY_SPAWN_INDEX))
        .expect("authored Level-1 Working Factory")
        .id;
    let main_base_id = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(MAIN_BASE_SPAWN_INDEX))
        .expect("authored Level-1 Main Base")
        .id;
    let mut world_fx = WorldFx::new();
    let mut notifications = GameplayNotifications::new();
    let terrain = session.cache.terrain().expect("Level-1 terrain");
    let [first_scientist, second_scientist] = spawn_two_factory_bound_scientists(
        &session,
        &mut entities,
        &mut world_fx,
        factory_id,
        main_base_id,
    );
    let first = prepare_factory_scientist_arrival(&entities, factory_id, first_scientist)
        .expect("first arrival");
    apply_prepared_factory_scientist_arrival(
        &mut entities,
        &mut world_fx,
        &mut notifications,
        100,
        first,
    )
    .expect("first delivery");
    tick_level_one_factory_owner(
        &mut entities,
        terrain,
        &mut world_fx,
        &mut notifications,
        101,
        factory_id,
        FRAME_MICROS,
    )
    .expect("1/2 tail");
    let second = prepare_factory_scientist_arrival(&entities, factory_id, second_scientist)
        .expect("second arrival");
    apply_prepared_factory_scientist_arrival(
        &mut entities,
        &mut world_fx,
        &mut notifications,
        102,
        second,
    )
    .expect("second delivery");
    for frame in 1..=300_u32 {
        let outcome = tick_level_one_factory_owner(
            &mut entities,
            terrain,
            &mut world_fx,
            &mut notifications,
            102 + frame,
            factory_id,
            FRAME_MICROS,
        )
        .expect("phase-0 frame");
        if frame == 300 {
            let product = outcome.product.expect("six-second type-61");
            let staffed_animation = entities
                .iter_all()
                .find(|entity| entity.id == factory_id)
                .and_then(|entity| match entity.base_factory_runtime {
                    RetailRuntimeValue::Known(Some(state)) => state.live_owner,
                    _ => None,
                })
                .expect("staffed live owner")
                .animation_state_raw;
            assert_ne!(
                staffed_animation, 0,
                "2/2 production must have left +0xB4 set"
            );
            assert!(entities
                .arm_live_factories_for_main_base_abort()
                .iter()
                .any(|started| started.target_id == factory_id));
            let first_death = entities.advance_base_factory_progressive_deaths(1);
            assert!(first_death.iter().all(|event| !matches!(
                event,
                v2k_game::entity::BaseFactoryProgressionEvent::TerminalDeath { .. }
            )));
            let factory = entities
                .iter_all()
                .find(|entity| entity.id == factory_id)
                .expect("dying factory");
            assert_eq!(factory.model_index, Some(227));
            let RetailRuntimeValue::Known(Some(state)) = factory.base_factory_runtime else {
                panic!("factory runtime");
            };
            assert_eq!(
                state.live_owner.expect("live owner").animation_state_raw,
                0,
                "FUN_00419B50 must idle +0xB4 on the first dying frame"
            );
            let events = entities.advance_base_factory_progressive_deaths(3_200_000);
            assert!(events.iter().any(|event| matches!(
                event,
                v2k_game::entity::BaseFactoryProgressionEvent::TerminalDeath {
                    target_id,
                    full_frame_sequence_requested: true,
                    ..
                } if *target_id == factory_id
            )));
            assert!(
                entities.is_power_up_destroy_pending(product.entity_id.get()),
                "stage 32 must queue the crate still sitting on the lift"
            );
            let factory = entities
                .iter_all()
                .find(|entity| entity.id == factory_id)
                .expect("dead factory remains allocated");
            let RetailRuntimeValue::Known(Some(state)) = factory.base_factory_runtime else {
                panic!("factory runtime");
            };
            let production = state.production.expect("Level-1 production");
            assert_eq!(production.current_scientists_raw, 0);
            assert_eq!(production.scientist_capacity_raw, 0);
            let live_owner = state.live_owner.expect("Level-1 live owner");
            let reset = v2k_game::factory_production_live::FactoryAnimationRange {
                start_raw_16_16: 0,
                end_raw_16_16: 0x0001_0000,
            };
            assert_eq!(live_owner.primary_animation_range, reset);
            assert_eq!(live_owner.secondary_animation_range, reset);
            assert_eq!(live_owner.animation_state_raw, 0);
            assert_eq!(state.current_scientists, 0);
            assert_eq!(state.required_scientists, 0);
        }
    }
}

#[v2k_test_support::retail_test]
fn staffed_factory_repeats_after_live_pickup_disappears() {
    let (session, mut entities) = fresh_level_one();
    let factory_id = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(FACTORY_SPAWN_INDEX))
        .expect("authored Level-1 Working Factory")
        .id;
    let main_base_id = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(MAIN_BASE_SPAWN_INDEX))
        .expect("authored Level-1 Main Base")
        .id;
    let mut world_fx = WorldFx::new();
    let mut notifications = GameplayNotifications::new();
    let terrain = session.cache.terrain().expect("Level-1 terrain");
    let first_product = staff_and_deliver_first_level_one_product(
        &session,
        &mut entities,
        &mut notifications,
        &mut world_fx,
        factory_id,
        main_base_id,
    );
    retain_level_one_factory_remaining_stock(&mut entities, factory_id, 1)
        .expect("positive remaining stock admits pickup presence");

    let active = tick_level_one_factory_owner(
        &mut entities,
        terrain,
        &mut world_fx,
        &mut notifications,
        700,
        factory_id,
        FRAME_MICROS,
    )
    .expect("active pickup presence frame");
    assert_eq!(
        active.production.phase,
        FactoryProductionPhase::WaitingForPickup
    );
    assert_eq!(active.production.remaining_stock_raw, 1);
    assert!(active
        .events
        .contains(&FactoryActivationLiveEvent::PickupPresence {
            pickup_id: first_product.entity_id.get(),
            presence: FactoryPickupPresence::Active,
        }));
    assert!(entities
        .iter_all()
        .any(|entity| entity.id == first_product.entity_id.get()));

    assert_eq!(
        splice_level_one_factory_spawned_pickup(&mut entities, factory_id)
            .expect("completed pickup splice"),
        first_product.entity_id.get()
    );
    assert!(entities
        .iter_all()
        .all(|entity| entity.id != first_product.entity_id.get()));

    let gone = tick_level_one_factory_owner(
        &mut entities,
        terrain,
        &mut world_fx,
        &mut notifications,
        701,
        factory_id,
        FRAME_MICROS,
    )
    .expect("gone pickup starts cooldown");
    assert_eq!(gone.production.phase, FactoryProductionPhase::Cooldown);
    assert_eq!(gone.production.cooldown_remaining_micros_raw, -1_000);
    assert_eq!(gone.production.production_progress_micros_raw, 0);
    assert_eq!(gone.production.delivery_progress_micros_raw, 0);
    assert!(gone
        .events
        .contains(&FactoryActivationLiveEvent::PickupPresence {
            pickup_id: first_product.entity_id.get(),
            presence: FactoryPickupPresence::Gone,
        }));

    let restart = tick_level_one_factory_owner(
        &mut entities,
        terrain,
        &mut world_fx,
        &mut notifications,
        702,
        factory_id,
        FRAME_MICROS,
    )
    .expect("negative cooldown expires immediately");
    assert_eq!(restart.production.phase, FactoryProductionPhase::Producing);
    assert_eq!(restart.production.cooldown_remaining_micros_raw, 0);
    assert_eq!(restart.production.production_progress_micros_raw, 0);

    let mut second_product = None;
    for frame in 1..=300_u32 {
        let outcome = tick_level_one_factory_owner(
            &mut entities,
            terrain,
            &mut world_fx,
            &mut notifications,
            702 + frame,
            factory_id,
            FRAME_MICROS,
        )
        .expect("repeat manufacture frame");
        if frame == 300 {
            second_product = outcome.product;
            assert_eq!(outcome.production.phase, FactoryProductionPhase::Delivering);
            assert_eq!(outcome.production.remaining_stock_raw, 0);
        }
    }
    let second_product = second_product.expect("repeat type-61 product");
    assert_ne!(second_product.entity_id, first_product.entity_id);
    assert!(entities
        .iter_all()
        .any(|entity| entity.id == second_product.entity_id.get() && entity.entity_type == 61));
}

#[v2k_test_support::retail_test]
fn unsupported_converted_output_class_fails_closed_without_type8_birth() {
    let (session, mut entities) = fresh_level_one();
    let factory_id = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(FACTORY_SPAWN_INDEX))
        .expect("authored Level-1 Working Factory")
        .id;
    let terrain = session.cache.terrain().expect("Level-1 terrain");
    let mut world_fx = WorldFx::new();
    let type8_before = entities
        .iter_all()
        .filter(|entity| entity.entity_type == 8)
        .count();
    for class in [0] {
        let entity_type = converted_output_entity_type(class);
        assert_ne!(entity_type, 8);
        let error = spawn_factory_converted_output(
            &mut entities,
            terrain,
            &mut world_fx,
            0,
            factory_id,
            FactoryEntitySpawnRequest {
                requested_handle: FactoryRequestedEntityHandle::Allocate,
                entity_type,
                position_raw: FACTORY_POSITION_RAW,
                spawn_parameter_6: 0,
            },
        )
        .expect_err("an unmapped output class cannot borrow a worker constructor");
        assert_eq!(
            error,
            FactoryActivationLiveError::UnsupportedConvertedOutput { entity_type }
        );
        assert_eq!(
            entities
                .iter_all()
                .filter(|entity| entity.entity_type == 8)
                .count(),
            type8_before,
            "class {class} must not borrow Type8"
        );
        assert!(
            !entities
                .iter_all()
                .any(|entity| entity.entity_type == entity_type
                    && entity.authored_spawn_index.is_none()),
            "class {class} must not invent a live {entity_type} birth"
        );
    }
}
