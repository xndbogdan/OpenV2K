//! Actual constructor/cursor/master-walk ownership, independent of ID snapshots.

use super::super::{
    Intro2RadialCursorCustody, SpecializedActorTaskProductionFrame,
    SpecializedActorTaskProductionOutcome, SpecializedActorTaskScheduler,
    SpecializedActorTaskWorld,
};
use crate::{
    entity_collision_state::RetailRuntimeValue,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    native_ground_actor::{NativeGroundActorOutcome, NativeGroundTaskCustody},
    split_and_explode::SplitChildRequest,
    static_damage::StaticDamageScheduler,
};

#[v2k_test_support::retail_test]
fn cursor_registered_child_receives_real_mover_visit_without_historical_id_snapshot() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(19);
    // This is the cursor's actual pre-constructor snapshot. The new tail can
    // never qualify through this list, even though13500 must visit it.
    let remaining_live_ids = manager.retail_live_order_ids().collect::<Vec<_>>();
    let mut publication = None;
    for _ in 0..128 {
        let request = SplitChildRequest {
            requested_entity_handle_raw: 0,
            entity_type: 56,
            position_raw: manager.player().unwrap().position_raw(),
            objective: true,
            velocity_raw: [-2000, -3000, 2000],
            rotation_heading_pitch_roll_raw: [0x9234, 0x8123, 0x7654],
        };
        let child = manager
            .construct_native_type56(request, &session.cache, &mut fx, 4793)
            .unwrap();
        if child.publication.selection.program.class_id == 7 {
            publication = Some(child);
            break;
        }
    }
    let publication = publication.expect("genuine weighted constructor reaches class7");
    let id = publication.owner.entity_id();
    assert!(!remaining_live_ids.contains(&id));
    assert_eq!(manager.retail_live_order_ids().last(), Some(id));
    let age_before = manager
        .entity_mut(id)
        .unwrap()
        .collision
        .recent_relation_elapsed_us_at_0x68;
    assert_eq!(age_before, RetailRuntimeValue::Known(0));

    let mut pending = Vec::new();
    let mut retained = Vec::new();
    Intro2RadialCursorCustody {
        pending: &mut pending,
        retained: &mut retained,
        remaining_live_ids: &remaining_live_ids,
    }
    .register_split_type56_child(publication.owner)
    .unwrap();
    // Exercise the production walker using the cursor's real output. Merely
    // finding the new ID in a vector would not prove its native visit executes.
    let mut scheduler = SpecializedActorTaskScheduler {
        owners: pending,
        ..Default::default()
    };
    let mut notifications = GameplayNotifications::new();
    let mut static_damage = StaticDamageScheduler::new();
    let pass = scheduler.tick(
        &mut manager,
        SpecializedActorTaskProductionFrame {
            world: SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            elapsed_micros: 25_000,
            global_elapsed_micros: 25_000,
            retail_tick: 4794,
            notification_phase: GameplayNotificationPhase::NonGameplay,
            main_base_abort_active: false,
        },
        &mut notifications,
    );
    assert_eq!(pass.block, None);
    assert!(
        matches!(
            pass.outcomes.as_slice(),
            [SpecializedActorTaskProductionOutcome::NativeType56(
                NativeGroundActorOutcome::Advanced {
                    entity_id,
                    callback_enabled: true,
                    callback_elapsed_micros: 25_000,
                }
            )] if *entity_id == id
        ),
        "actual newborn callback must complete: {:?}",
        pass.outcomes
    );
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .recent_relation_elapsed_us_at_0x68,
        RetailRuntimeValue::Known(25_000)
    );
    assert_eq!(scheduler.owners.len(), 1);
    assert_eq!(scheduler.owners[0].entity_id(), id);
    assert!(retained.is_empty());
}
