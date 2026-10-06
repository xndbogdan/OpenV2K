use super::*;
use crate::{
    native_type122::construction_tests::native_fixture_with_player,
    type60_exploding_ring::{
        HardWaterType60ConstructionRequest, HardWaterType60Severity, HostType60Allocator,
        Type60ConstructionRequest, Type60InitializerDisposition,
    },
    type60_exploding_ring_production::tick_type60_exploding_ring_production_owner,
};

#[v2k_test_support::retail_test]
fn ordinary_world_abort_preserves_the_ring_custody_decision_matrix() {
    for kind in 0..3 {
        for progressed in [false, true] {
            for supplied in [false, true] {
                let (session, mut manager, mut fx) = native_fixture_with_player(15);
                let request = match kind {
                    0 => Type60ConstructionRequest::class49_at([100, 10000, -300]),
                    1 => HardWaterType60ConstructionRequest::new(
                        [100, 10000, -300],
                        HardWaterType60Severity::Moderate,
                    )
                    .into_generic(),
                    _ => HardWaterType60ConstructionRequest::new(
                        [100, 10000, -300],
                        HardWaterType60Severity::Severe,
                    )
                    .into_generic(),
                };
                let Type60ConstructionOutcome::ActorLinked(receipt) = manager
                    .construct_type60_exploding_ring_with_allocator(
                        request,
                        session.cache.terrain().unwrap(),
                        &mut fx,
                        &mut HostType60Allocator,
                    )
                else {
                    panic!("real ring allocation")
                };
                let Type60InitializerDisposition::PrimaryPublished { task_lease, .. } =
                    receipt.initializer()
                else {
                    panic!("real primary")
                };
                let mut owner = Type60ExplodingRingProductionOwner::adopt(task_lease);
                if progressed {
                    owner = tick_type60_exploding_ring_production_owner(
                        &mut manager,
                        owner,
                        20_000,
                        &mut fx,
                    )
                    .retained_owner
                    .expect("one nonterminal ring callback");
                    assert_eq!(owner.next_callback_sequence(), 2);
                }
                let before = manager
                    .entity_mut(receipt.actor().entity_id)
                    .unwrap()
                    .collision
                    .clone();
                let result = manager.apply_main_base_abort_type60_ring_death(
                    receipt.actor(),
                    supplied.then_some(&owner),
                    &mut fx,
                );
                if supplied || (kind == 0 && !progressed) {
                    assert!(matches!(
                        result,
                        Ok(MainBaseType60RingDeathAdvance::Advanced {
                            outcome: MainBaseType60RingDeathOutcome::DeferredDestroyStaged { .. },
                            ..
                        })
                    ));
                    assert!(manager
                        .entity_mut(receipt.actor().entity_id)
                        .unwrap()
                        .actor_tasks
                        .task_in_slot(ActorTaskSlot::Primary)
                        .is_none());
                    assert!(fx
                        .exploding_rings()
                        .iter()
                        .all(|ring| ring.associated_entity_id() != receipt.actor().entity_id));
                } else {
                    let expected = if kind == 0 {
                        MainBaseType60RingDeathBlock::InitializerTaskStateMismatch
                    } else {
                        MainBaseType60RingDeathBlock::SchedulerOwnerMissing
                    };
                    assert_eq!(result, Err(expected));
                    assert_eq!(
                        manager
                            .entity_mut(receipt.actor().entity_id)
                            .unwrap()
                            .collision,
                        before
                    );
                }
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn class49_ring_rejects_a_stale_owner_sequence_and_an_executing_wrapper() {
    use crate::actor_task_owner::ActorTaskVisit;
    let (session, mut manager, mut fx) = native_fixture_with_player(15);
    let Type60ConstructionOutcome::ActorLinked(receipt) = manager
        .construct_type60_exploding_ring_with_allocator(
            Type60ConstructionRequest::class49_at([100, 10000, -300]),
            session.cache.terrain().unwrap(),
            &mut fx,
            &mut HostType60Allocator,
        )
    else {
        panic!()
    };
    let Type60InitializerDisposition::PrimaryPublished { task_lease, .. } = receipt.initializer()
    else {
        panic!()
    };
    let stale = Type60ExplodingRingProductionOwner::adopt(task_lease);
    let owner = tick_type60_exploding_ring_production_owner(
        &mut manager,
        stale.fork_for_main_base_abort_transaction(),
        20_000,
        &mut fx,
    )
    .retained_owner
    .unwrap();
    let id = receipt.actor().entity_id;
    let before = manager.entity_mut(id).unwrap().collision.clone();
    assert!(matches!(
        manager.apply_main_base_abort_type60_ring_death(receipt.actor(), Some(&stale), &mut fx),
        Err(MainBaseType60RingDeathBlock::SchedulerOwnerSequenceMismatch { .. })
    ));
    let entity = manager.entity_mut(id).unwrap();
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    entity
        .actor_tasks
        .begin_exact_visit_with(
            ActorTaskVisit {
                slot: ActorTaskSlot::Primary,
                task_id,
            },
            |_| (),
        )
        .unwrap();
    assert_eq!(
        manager.apply_main_base_abort_type60_ring_death(receipt.actor(), Some(&owner), &mut fx),
        Err(MainBaseType60RingDeathBlock::InitializerTaskStateMismatch)
    );
    assert_eq!(manager.entity_mut(id).unwrap().collision, before);
}
