//! Real dynamic body routes retain B4 separately from handles, Sub-D and RNG.

use super::*;
use crate::{
    main_base_conversion::MainBaseReplacementSpawn,
    main_base_conversion_runtime::{
        apply_prepared_first_world_main_base_replacement, prepare_first_world_main_base_replacement,
    },
    session::GameSession,
};

fn load() -> (GameSession, EntityManager, WorldFx) {
    let data = v2k_test_support::retail_dir();
    assert!(
        data.join("PRELOAD.DAT").is_file(),
        "normal-tier corpus required"
    );
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(14, 1).unwrap();
    let rows: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    let mut fx = WorldFx::new();
    let manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: 2,
            type_metadata: &rows,
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
    (session, manager, fx)
}

fn ordinal(manager: &EntityManager) -> u16 {
    let RetailRuntimeValue::Known(value) = manager.next_common_body_ordinal() else {
        panic!("native world owns the body counter");
    };
    value
}

fn assert_stamp(manager: &EntityManager, id: u32, attempted_ordinal: u16) {
    assert_eq!(
        manager
            .entities
            .iter()
            .find(|entity| entity.id == id)
            .unwrap()
            .construction_stamp_at_0xb4,
        RetailRuntimeValue::Known(0x0800u16.wrapping_add(attempted_ordinal))
    );
    assert_eq!(ordinal(manager), attempted_ordinal.wrapping_add(1));
}

fn factory(manager: &EntityManager) -> (FactoryProductionEntityVersion, [i16; 3]) {
    let entity = manager
        .entities
        .iter()
        .find(|entity| entity.entity_type == 66)
        .unwrap();
    let RetailRuntimeValue::Known(Some(runtime)) = entity.base_factory_runtime else {
        panic!("native factory context");
    };
    let live = runtime.live_owner.unwrap();
    (
        FactoryProductionEntityVersion {
            entity_id: entity.id,
            allocation_identity: live.allocation_identity,
            state_version: live.state_version,
        },
        entity.position_raw(),
    )
}

fn conversion(manager: &EntityManager) -> MainBaseReplacementSpawn {
    let source = manager
        .entities
        .iter()
        .find(|entity| entity.entity_type == 9)
        .unwrap();
    let base = manager
        .entities
        .iter()
        .find(|entity| entity.entity_type == 6)
        .unwrap();
    MainBaseReplacementSpawn {
        source_entity_id: source.id,
        main_base_entity_id: base.id,
        replacement_type: 8,
        position_raw: base.position_raw(),
    }
}

#[v2k_test_support::retail_test]
fn factory_pickup_worker_and_materialiser_charge_one_body_each_after_admission() {
    let (session, mut manager, mut fx) = load();
    let (factory, position_raw) = factory(&manager);
    let request = FactoryEntitySpawnRequest {
        requested_handle: FactoryRequestedEntityHandle::Allocate,
        entity_type: 61,
        position_raw,
        spawn_parameter_6: 0x0001_f412,
    };
    let before = ordinal(&manager);
    let seed = fx.next_sub_d_allocation_seed();
    let mut expected_fx = fx.fork_for_main_base_abort_transaction();
    let foreign = FactoryProductionEntityVersion {
        allocation_identity: factory.allocation_identity.wrapping_add(1),
        ..factory
    };
    assert!(manager
        .append_factory_pickup(foreign, request, &mut fx)
        .is_none());
    assert_eq!(
        ordinal(&manager),
        before,
        "foreign host admission is not104B0"
    );
    let selector = expected_fx.next_shared_retail_random_u16();
    let product = manager
        .append_factory_pickup(factory, request, &mut fx)
        .unwrap();
    assert_stamp(&manager, product.entity_id.get(), before);
    let entity = manager
        .entities
        .iter()
        .find(|entity| entity.id == product.entity_id.get())
        .unwrap();
    assert_eq!(
        entity
            .factory_type61_birth_provenance
            .unwrap()
            .selector_rng_word(),
        selector
    );
    assert_eq!(fx.next_sub_d_allocation_seed(), seed, "Type61 has no Sub-D");
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_fx.next_shared_retail_random_u16()
    );

    let request = FactoryEntitySpawnRequest {
        entity_type: 8,
        spawn_parameter_6: 0,
        ..request
    };
    let before = ordinal(&manager);
    let mut expected_fx = fx.fork_for_main_base_abort_transaction();
    assert!(manager
        .append_factory_converted_output(
            factory,
            FactoryEntitySpawnRequest {
                spawn_parameter_6: 1,
                ..request
            },
            0,
            session.cache.terrain().unwrap(),
            &mut fx
        )
        .is_none());
    assert_eq!(ordinal(&manager), before);
    let _sub_a = expected_fx.next_shared_retail_random_u16();
    let selector = expected_fx.next_shared_retail_random_u16();
    let task = expected_fx.next_shared_retail_random_u16();
    let worker = manager
        .append_factory_converted_output(
            factory,
            request,
            0,
            session.cache.terrain().unwrap(),
            &mut fx,
        )
        .unwrap();
    assert_stamp(&manager, worker.entity_id.get(), before);
    let birth = manager.factory_converted_output_births().last().unwrap();
    assert_eq!(birth.selector_rng_word, selector);
    assert_eq!(birth.constructor_rng_words, vec![task]);
    assert_eq!(fx.next_sub_d_allocation_seed(), seed.wrapping_add(1));
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_fx.next_shared_retail_random_u16()
    );
    assert!(
        crate::intro2_type8::intro2_type8_manager_allocation_authenticates(
            &manager,
            worker.entity_id.get()
        )
    );

    let request = FactoryEntitySpawnRequest {
        entity_type: 93,
        ..request
    };
    let before = ordinal(&manager);
    let seed = fx.next_sub_d_allocation_seed();
    let mut expected_fx = fx.fork_for_main_base_abort_transaction();
    assert!(manager
        .append_factory_output_materialiser(
            factory,
            FactoryEntitySpawnRequest {
                spawn_parameter_6: 1,
                ..request
            },
            session.cache.terrain().unwrap(),
            &mut fx
        )
        .is_none());
    assert_eq!(
        ordinal(&manager),
        before,
        "invalid Type93 admission creates no body"
    );
    expected_fx.next_shared_retail_random_u16(); //Type93 singleton selector.
    let materialiser = manager
        .append_factory_output_materialiser(
            factory,
            request,
            session.cache.terrain().unwrap(),
            &mut fx,
        )
        .unwrap();
    assert_stamp(&manager, materialiser.entity_id.get(), before);
    assert_eq!(fx.next_sub_d_allocation_seed(), seed, "Type93 has no Sub-D");
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_fx.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn main_base_plan_and_source_destruction_precede_one_replacement_body_stamp() {
    let (session, mut manager, mut fx) = load();
    let request = conversion(&manager);
    let before = ordinal(&manager);
    let source_stamp = manager
        .entities
        .iter()
        .find(|entity| entity.id == request.source_entity_id)
        .unwrap()
        .construction_stamp_at_0xb4;
    let plan =
        prepare_first_world_main_base_replacement(&manager, session.cache.terrain(), request, 0)
            .unwrap();
    assert_eq!(ordinal(&manager), before, "planning does not enter104B0");
    assert_eq!(
        manager.preflight_main_base_scientist_append(
            request.source_entity_id,
            request.main_base_entity_id
        ),
        Err(MainBaseScientistSpawnError::SourceDestroyNotPending {
            source_id: request.source_entity_id
        })
    );
    assert_eq!(ordinal(&manager), before);
    assert_eq!(
        manager.queue_main_base_conversion_destroy(request.source_entity_id),
        MainBaseConversionDestroyQueueOutcome::Queued {
            source_id: request.source_entity_id
        }
    );
    assert_eq!(
        ordinal(&manager),
        before,
        "the destruction prefix creates no replacement body yet"
    );
    let seed = fx.next_sub_d_allocation_seed();
    let mut expected_fx = fx.fork_for_main_base_abort_transaction();
    for _ in 0..3 {
        expected_fx.next_shared_retail_random_u16();
    }
    let spawned = apply_prepared_first_world_main_base_replacement(
        &mut manager,
        session.cache.terrain(),
        &mut fx,
        plan,
    );
    assert_stamp(&manager, spawned.replacement_id, before);
    assert!(manager
        .pending_main_base_conversion_destroy_ids()
        .contains(&request.source_entity_id));
    assert_eq!(
        manager
            .entities
            .iter()
            .find(|entity| entity.id == request.source_entity_id)
            .unwrap()
            .construction_stamp_at_0xb4,
        source_stamp
    );
    assert_eq!(
        manager
            .entities
            .iter()
            .find(|entity| entity.id == spawned.replacement_id)
            .unwrap()
            .collision
            .recent_relation_id_at_0x60,
        RetailRuntimeValue::Known(Some(request.main_base_entity_id))
    );
    assert_eq!(fx.next_sub_d_allocation_seed(), seed.wrapping_add(1));
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_fx.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn entered_worker_body_keeps_its_stamp_and_component_prefix_when_publication_stops() {
    let (session, mut manager, mut fx) = load();
    let request = conversion(&manager);
    let _plan =
        prepare_first_world_main_base_replacement(&manager, session.cache.terrain(), request, 0)
            .unwrap();
    manager.queue_main_base_conversion_destroy(request.source_entity_id);
    manager
        .preflight_main_base_scientist_append(request.source_entity_id, request.main_base_entity_id)
        .unwrap();
    let before = ordinal(&manager);
    let entity_count = manager.entities.len();
    let seed = fx.next_sub_d_allocation_seed();
    let mut expected_fx = fx.fork_for_main_base_abort_transaction();
    expected_fx.next_shared_retail_random_u16(); //20450, before selector/task.
    let body = manager.begin_type8_body_construction(8, &mut fx);
    assert!(body.is_for_entity_type(8));
    assert!(!body.is_for_entity_type(90));
    assert!(!body.is_for_entity_type(116));
    assert_eq!(
        body.construction_stamp(),
        RetailRuntimeValue::Known(0x0800u16.wrapping_add(before))
    );
    assert_eq!(body.native_sub_d().unwrap().seed, seed);
    // Controlled stop at the entered body/component boundary. This tests
    // retained construction ownership, not a retail malloc-failure oracle.
    drop(body);
    assert_eq!(ordinal(&manager), before.wrapping_add(1));
    assert_eq!(manager.entities.len(), entity_count);
    assert!(manager
        .pending_main_base_conversion_destroy_ids()
        .contains(&request.source_entity_id));
    assert_eq!(fx.next_sub_d_allocation_seed(), seed.wrapping_add(1));
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_fx.next_shared_retail_random_u16()
    );

    let rows = manager.type_metadata.clone();
    let mut generic = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &rows,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    );
    let body = generic.begin_type8_body_construction(8, &mut fx);
    assert_eq!(body.construction_stamp(), RetailRuntimeValue::Unresolved);
    assert_eq!(
        generic.next_common_body_ordinal(),
        RetailRuntimeValue::Unresolved
    );
    assert_eq!(
        Type8BodyConstruction::CapturedMainBaseFixture.construction_stamp(),
        RetailRuntimeValue::Unresolved
    );
}
