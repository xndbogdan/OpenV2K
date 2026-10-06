use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::{AuthoredWorldConstruction, EntityConstructionResources},
    entity_collision_state::EntityTypeRuntimeMetadata,
    session::GameSession,
};

fn fixture(level: u32) -> (EntityManager, WorldFx, SpecializedActorTaskScheduler) {
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
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    let mut fx = WorldFx::new();
    let mut entities = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: (level - 12) as i32,
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
            },
            player_arrival: None,
            retail_tick: 0,
        },
        &mut fx,
    )
    .unwrap();
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(
        tasks.adopt_intro2_type8(&mut entities),
        if level == 40 { 1 } else { 4 }
    );
    (entities, fx, tasks)
}

#[v2k_test_support::retail_test]
fn worker79_and91_abort_owns_all_nine_native_allocations_once() {
    for level in [19, 38, 40] {
        let (mut entities, mut fx, mut tasks) = fixture(level);
        let ids: Vec<_> = entities
            .iter_all()
            .filter(|e| matches!(e.entity_type, 79 | 91))
            .map(|e| e.id)
            .collect();
        let mut publications = MainBaseAbortPublicationCounts::default();
        for id in ids {
            let before = entities.entity_mut(id).unwrap();
            let retained = (
                before.intro2_type8_runtime,
                before.position_raw(),
                before.physical_body_basis_q31(),
                before.model_slots,
                before.construction_stamp_at_0xb4,
            );
            let ordinal = entities.next_common_body_ordinal();
            let seed = fx.next_sub_d_allocation_seed();
            let mut expected = fx.fork_for_main_base_abort_transaction();
            expected.next_shared_retail_random_u16();
            let actor = entities.main_base_abort_actor_observation(id).unwrap();
            let result = dispatch_native_actor(
                actor,
                &mut entities,
                &mut fx,
                &mut tasks,
                &mut publications,
                &mut MainBaseAbortGameplayContext {
                    extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                    notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                    retail_tick: 400,
                },
            )
            .unwrap();
            let success = match result {
                Ok(success) => success,
                Err(failure) => panic!("{:?}", failure.callback_error),
            };
            assert_eq!(
                success.disposition,
                MainBaseAbortActorDisposition::Type8Death
            );
            let actor = entities.entity_mut(id).unwrap();
            assert_eq!(actor.collision.health_raw, RetailRuntimeValue::Known(0));
            assert!(
                matches!(actor.actor_task_state(ActorTaskSlot::Primary),Some(ActorTaskRuntime::SharedRetarget(task)) if task.lifetime_ms()==1000)
            );
            assert_eq!(
                (
                    actor.intro2_type8_runtime,
                    actor.position_raw(),
                    actor.physical_body_basis_q31(),
                    actor.model_slots,
                    actor.construction_stamp_at_0xb4
                ),
                retained
            );
            assert_eq!(entities.next_common_body_ordinal(), ordinal);
            assert_eq!(fx.next_sub_d_allocation_seed(), seed);
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                expected.next_shared_retail_random_u16()
            );
            let before_publications = publications.clone();
            let mut expected = fx.fork_for_main_base_abort_transaction();
            let actor = entities.main_base_abort_actor_observation(id).unwrap();
            assert!(dispatch_native_actor(
                actor,
                &mut entities,
                &mut fx,
                &mut tasks,
                &mut publications,
                &mut MainBaseAbortGameplayContext {
                    extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                    notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                    retail_tick: 401
                }
            )
            .unwrap()
            .is_ok());
            assert_eq!(publications, before_publications);
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                expected.next_shared_retail_random_u16()
            );
        }
        assert_eq!(
            publications.type8_exploding,
            if level == 40 { 1 } else { 4 }
        );
    }
}

#[v2k_test_support::retail_test]
fn worker79_and91_abort_rejects_foreign_and_parked_custody_without_prefix() {
    for level in [19, 40] {
        for foreign in [false, true] {
            let (mut entities, mut fx, mut tasks) = fixture(level);
            let id = entities
                .iter_all()
                .find(|e| matches!(e.entity_type, 79 | 91))
                .unwrap()
                .id;
            if foreign {
                let (other, _, _) = fixture(level);
                entities.entity_mut(id).unwrap().intro2_type8_runtime = other
                    .iter_all()
                    .find(|e| e.id == id)
                    .unwrap()
                    .intro2_type8_runtime;
            } else {
                tasks.park_intro2_type8_external_prefix(id);
            }
            let collision = entities.entity_mut(id).unwrap().collision.clone();
            let graph = entities
                .entity_mut(id)
                .unwrap()
                .actor_task_state(ActorTaskSlot::Primary)
                .copied();
            let mut expected = fx.fork_for_main_base_abort_transaction();
            let mut publications = MainBaseAbortPublicationCounts::default();
            let actor = entities.main_base_abort_actor_observation(id).unwrap();
            assert!(dispatch_native_actor(
                actor,
                &mut entities,
                &mut fx,
                &mut tasks,
                &mut publications,
                &mut MainBaseAbortGameplayContext {
                    extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                    notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                    retail_tick: 400
                }
            )
            .unwrap()
            .is_err());
            assert_eq!(publications, MainBaseAbortPublicationCounts::default());
            assert_eq!(entities.entity_mut(id).unwrap().collision, collision);
            assert_eq!(
                entities
                    .entity_mut(id)
                    .unwrap()
                    .actor_task_state(ActorTaskSlot::Primary)
                    .copied(),
                graph
            );
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                expected.next_shared_retail_random_u16()
            );
        }
    }
}
