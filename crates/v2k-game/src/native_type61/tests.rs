use super::*;
use crate::native_type122::construction_tests::native_fixture_with_player;

#[v2k_test_support::retail_test]
fn native_power_up_castle_and_alpine_keep_authored_payloads_and_models() {
    for world in [13, 15, 17, 18, 37] {
        let (session, manager, _) = native_fixture_with_player(world);
        let level = session.cache.level_desc().unwrap();
        let mut count = 0;
        for entity in manager.iter_all().filter(|e| e.entity_type == 61) {
            count += 1;
            assert!(allocation_authenticates(&manager, entity.id));
            let spawn = &level.entities[entity.authored_spawn_index.unwrap()];
            assert_eq!(entity.position_raw(), spawn.position_raw());
            assert_eq!(
                entity.power_up_payload_packed,
                Some(u32::from_le_bytes(spawn.extra[8..12].try_into().unwrap()))
            );
            assert_eq!(
                entity.model_slots,
                spawn
                    .model_overrides
                    .map(|id| Some(if id == 0 { 82 } else { id as usize }))
            );
            assert!(
                matches!(entity.initial_behavior, RetailRuntimeValue::Known(Some(selection)) if selection.choice_index == 0 && selection.program.class_id == 23)
            );
            assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
                .into_iter()
                .all(|slot| entity.actor_task_state(slot).is_none()));
            assert_eq!(
                entity.collision.constructor_sound_attachment_id_at_0x8c,
                RetailRuntimeValue::Known(Some(44))
            );
        }
        assert_eq!(
            count,
            level
                .entities
                .iter()
                .filter(|s| s.entity_type == 61)
                .count()
        );
        assert!(count > 0, "world {world} exercises an authored pickup");
    }
}

#[v2k_test_support::retail_test]
fn native_power_up_receipt_survives_live_state_but_rejects_another_world_allocation() {
    let (_, mut first, _) = native_fixture_with_player(15);
    let (_, mut second, _) = native_fixture_with_player(15);
    let id = first.iter_all().find(|e| e.entity_type == 61).unwrap().id;
    let original = first.entity_mut(id).unwrap().native_type61_allocation;
    let foreign = second.entity_mut(id).unwrap().native_type61_allocation;
    assert_ne!(original, foreign);
    first.entity_mut(id).unwrap().native_type61_allocation = foreign;
    assert!(!allocation_authenticates(&first, id));
    let entity = first.entity_mut(id).unwrap();
    entity.native_type61_allocation = original;
    entity.set_position_raw([123, -400, 500]);
    entity.collision.health_raw = RetailRuntimeValue::Known(800_000);
    assert!(allocation_authenticates(&first, id));
}
