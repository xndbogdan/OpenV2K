use super::*;
use crate::native_type122::construction_tests::native_fixture_with_player;

#[v2k_test_support::retail_test]
fn native_power_up_radial_custody_requires_its_own_allocation_and_absent_tasks() {
    let (_, mut manager, _) = native_fixture_with_player(15);
    let (_, mut foreign, _) = native_fixture_with_player(15);
    let id = manager.iter_all().find(|e| e.entity_type == 61).unwrap().id;
    let foreign_id = foreign.iter_all().find(|e| e.entity_type == 61).unwrap().id;
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert!(scheduler.prepare_native_actor_mutation(&manager, id));

    std::mem::swap(
        manager.entity_mut(id).unwrap(),
        foreign.entity_mut(foreign_id).unwrap(),
    );
    assert!(!scheduler.prepare_native_actor_mutation(&manager, id));
    std::mem::swap(
        manager.entity_mut(id).unwrap(),
        foreign.entity_mut(foreign_id).unwrap(),
    );

    let base_id = manager.iter_all().find(|e| e.entity_type == 6).unwrap().id;
    let task_graph = std::mem::take(&mut manager.entity_mut(base_id).unwrap().actor_tasks);
    manager.entity_mut(id).unwrap().actor_tasks = task_graph;
    assert!(!scheduler.prepare_native_actor_mutation(&manager, id));
}
