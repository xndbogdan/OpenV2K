use super::*;
use crate::{
    entity::{AuthoredWorldConstruction, EntityConstructionResources},
    entity_collision_state::EntityTypeRuntimeMetadata,
    main_base_conversion::{MainBaseConversionPhase, MainBaseConversionUnresolved},
    session::GameSession,
};

#[v2k_test_support::retail_test]
fn native_world13_missing_receipts_cannot_borrow_captured_main_base_conversion() {
    let data = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data).expect("retail PRELOAD required");
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(13, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    let mut fx = WorldFx::new();
    let mut entities = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: 1,
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
    assert!(entities.is_fresh_new_game_first_world());
    assert!(!entities.has_captured_first_world_construction());
    let base = entities
        .iter_all()
        .find(|entity| entity.entity_type == 6)
        .unwrap();
    let (base_id, base_position) = (base.id, base.position);
    let people: Vec<_> = entities
        .iter_all()
        .filter(|entity| entity.entity_type == 9)
        .map(|entity| entity.id)
        .collect();
    for &id in &people {
        entities.entity_mut(id).unwrap().position =
            [base_position[0] + 64.0, base_position[1], base_position[2]];
    }
    let source_id = people[0];
    let source = entities.entity_mut(source_id).unwrap();
    source.position = base_position;
    let source_receipt = source
        .ordinary_type9_native_receipt
        .take()
        .expect("actual native receipt");
    let source_collision = source.collision.clone();
    let source_graph = crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .map(|slot| source.actor_task_state(slot).copied());
    let before_order: Vec<_> = entities.retail_live_order_ids().collect();
    let before_ordinal = entities.next_common_body_ordinal();
    let mut oracle = fx.fork_for_main_base_abort_transaction();
    let mut tasks = SpecializedActorTaskScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let error = resolve_first_world_main_base_conversions(MainBaseConversionFrame {
        entities: &mut entities,
        actor_tasks: &mut tasks,
        model_pool: &session.cache,
        terrain: session.cache.terrain(),
        world_fx: &mut fx,
        notifications: &mut notifications,
        retail_tick: 0,
        world_style: RetailRuntimeValue::Known(1),
    })
    .expect_err("missing native source receipt must not fall through to captured Type9");
    assert!(
        matches!(error.unresolved, MainBaseConversionUnresolved::Host {
        candidate_id,
        phase: MainBaseConversionPhase::ExactPairContact,
        source: FirstWorldMainBaseConversionUnresolved::SourceAllocationUnavailable { entity_id },
    } if candidate_id == source_id && entity_id == source_id)
    );
    let source = entities.entity_mut(source_id).unwrap();
    assert_eq!(source.collision, source_collision);
    assert_eq!(
        crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .map(|slot| source.actor_task_state(slot).copied()),
        source_graph
    );
    source.ordinary_type9_native_receipt = Some(source_receipt);
    entities
        .entity_mut(base_id)
        .unwrap()
        .main_base_runtime
        .take()
        .expect("native Base receipt");
    assert_eq!(
        resolve_first_world_main_base_conversions(MainBaseConversionFrame {
            entities: &mut entities,
            actor_tasks: &mut tasks,
            model_pool: &session.cache,
            terrain: session.cache.terrain(),
            world_fx: &mut fx,
            notifications: &mut notifications,
            retail_tick: 0,
            world_style: RetailRuntimeValue::Known(1),
        }),
        Ok(FirstWorldMainBaseConversionPass::MainBaseAbsent),
        "native Base without its receipt is not the captured spawn6 fixture"
    );
    assert_eq!(
        entities.retail_live_order_ids().collect::<Vec<_>>(),
        before_order
    );
    assert_eq!(entities.next_common_body_ordinal(), before_ordinal);
    assert!(entities
        .pending_main_base_conversion_destroy_ids()
        .is_empty());
    assert_eq!(
        format!("{notifications:?}"),
        format!("{:?}", GameplayNotifications::new())
    );
    assert_eq!(fx.pending_event_count(), oracle.pending_event_count());
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
}
