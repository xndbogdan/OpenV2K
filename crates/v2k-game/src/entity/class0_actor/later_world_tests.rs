//! Later worlds' class0 rows: world 17's Type72 and world 43's Type85 (Type54's
//! class0 body with a running callback and an explicit Change Sea Level delta)
//! and the modelless Type106 of worlds 37, 38 and 40.

use super::*;
use crate::main_base_type54_abort::{
    MainBaseType54DeathAdvance, MainBaseType54DeathOutcome, MainBaseType54NetworkSession,
    MainBaseType54SeaDeltaSource, TYPE72_ENTITY_TYPE, TYPE85_ENTITY_TYPE,
};
use crate::specialized_actor_task_production::{
    SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
    SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
};

fn ids(manager: &EntityManager, entity_type: u32) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| entity.entity_type == entity_type)
        .map(|entity| entity.id)
        .collect()
}

// Less the per-frame detail bits and the birth's surface pair.
const MASK: u32 = !(0x0600_0000 | crate::entity_collision_state::SURFACE_STATE_MASK);

fn state(manager: &EntityManager, id: u32) -> RetailRuntimeValue<u32> {
    manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .masked(MASK)
}

fn set_detail(manager: &mut EntityManager, ids: &[u32], bits: u32) {
    for &id in ids {
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x0600_0000, bits);
    }
}

/// Tick class0 owners for 200 visits; return each actor's state after its
/// first callback, or `None` for one whose callback never ran.
fn first_callback_states(
    session: &mut crate::session::GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    ids: &[u32],
) -> Vec<Option<RetailRuntimeValue<u32>>> {
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert!(tasks.adopt_class0_actors(manager) >= ids.len());
    let mut states = vec![None; ids.len()];
    for tick in 1..=200 {
        let pass = tasks.tick(
            manager,
            SpecializedActorTaskProductionFrame {
                world: SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut session.cache,
                world_fx: fx,
                static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
                elapsed_micros: 20_000,
                global_elapsed_micros: tick * 20_000,
                retail_tick: tick,
                main_base_abort_active: false,
            },
            &mut crate::gameplay_notifications::GameplayNotifications::new(),
        );
        assert!(pass.block.is_none(), "tick{tick}: {:?}", pass.block);
        for outcome in &pass.outcomes {
            match outcome {
                SpecializedActorTaskProductionOutcome::Class0Actor(
                    Class0ActorOutcome::Advanced {
                        entity_id,
                        callback_enabled,
                        ..
                    },
                ) => {
                    if let Some(index) = ids.iter().position(|id| id == entity_id) {
                        if *callback_enabled && states[index].is_none() {
                            states[index] = Some(state(manager, *entity_id));
                        }
                    }
                }
                SpecializedActorTaskProductionOutcome::Class0Actor(
                    Class0ActorOutcome::Blocked { reason, .. },
                ) => panic!("tick{tick}: {reason:?}"),
                _ => {}
            }
        }
    }
    states
}

/// Retail's world-17 Type72s read `0x08468805` at their first 12DA0 visit
/// and `0x08428805` after their first callback.
#[v2k_test_support::retail_test]
fn world17_type72s_run_class0_from_the_retail_state() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(17);
    let ids = ids(&manager, TYPE72_ENTITY_TYPE);
    assert_eq!(ids.len(), 3);
    for &id in &ids {
        assert!(manager.native_class0_allocation_authenticates(id));
        assert_eq!(
            manager
                .iter_all()
                .find(|entity| entity.id == id)
                .unwrap()
                .main_base_type54_sea_delta_source,
            RetailRuntimeValue::Known(MainBaseType54SeaDeltaSource::ExplicitPreShiftWords(0xb4))
        );
        assert_eq!(
            state(&manager, id),
            RetailRuntimeValue::Known(0x0846_8805 & MASK)
        );
    }
    // Retail's presentation classified these far bodies coarse (no 411400
    // detail bits), so their first callback is E870's: class0's style sets
    // 0x2000, which clears motion 0x40000 and skips the walk and DF70.
    set_detail(&mut manager, &ids, 0);
    assert_eq!(
        first_callback_states(&mut session, &mut manager, &mut fx, &ids),
        vec![Some(RetailRuntimeValue::Known(0x0842_8805 & MASK)); 3]
    );
}

/// Retail's world-43 Type85 reads `0x0E468005` at its first 12DA0 visit,
/// grounded at Y 2112, and `0x0EC68005` after its first callback, a
/// detailed one whose DF70 sets grounded 0x800000.
#[v2k_test_support::retail_test]
fn world43_type85_runs_class0_from_the_retail_state() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(43);
    let ids = ids(&manager, TYPE85_ENTITY_TYPE);
    assert_eq!(ids.len(), 1);
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == ids[0])
        .unwrap();
    assert!(manager.native_class0_allocation_authenticates(ids[0]));
    assert_eq!(
        entity.main_base_type54_sea_delta_source,
        RetailRuntimeValue::Known(MainBaseType54SeaDeltaSource::ExplicitPreShiftWords(0x1b0))
    );
    assert_eq!(entity.position_raw(), [32512, 2112, -32256]);
    assert_eq!(
        state(&manager, ids[0]),
        RetailRuntimeValue::Known(0x0E46_8005 & MASK)
    );
    set_detail(&mut manager, &ids, 0x0600_0000);
    assert_eq!(
        first_callback_states(&mut session, &mut manager, &mut fx, &ids),
        vec![Some(RetailRuntimeValue::Known(0x0EC6_8005 & MASK))]
    );
}

/// Retail's Type106 keeps `0x06400005` from birth: no callback enable, so
/// its visits age the scheduler and run master motion only.
#[v2k_test_support::retail_test]
fn type106_keeps_its_retail_state_without_a_callback() {
    for (level_id, position) in [
        (37, [-32000, 0, -32768]),
        (38, [32000, 0, -31232]),
        (40, [14336, 0, 31232]),
    ] {
        let (mut session, mut manager, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level_id);
        let ids = ids(&manager, 106);
        assert_eq!(ids.len(), 1, "world{level_id}");
        assert!(manager.native_class0_allocation_authenticates(ids[0]));
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ids[0])
            .unwrap();
        assert_eq!(entity.model_slots, [None; 4]);
        assert_eq!(entity.position_raw(), position, "world{level_id}");
        let expected = RetailRuntimeValue::Known(0x0640_0005 & MASK);
        assert_eq!(state(&manager, ids[0]), expected, "world{level_id}");
        assert_eq!(
            first_callback_states(&mut session, &mut manager, &mut fx, &ids),
            vec![None]
        );
        assert_eq!(state(&manager, ids[0]), expected, "world{level_id}");
        assert_eq!(
            manager
                .iter_all()
                .find(|entity| entity.id == ids[0])
                .unwrap()
                .position_raw(),
            position
        );
    }
}

/// 405040 takes a nonzero `+88` as is: the class38 delta is the shifted word,
/// not the derived height difference. 40BDC0's burst reads the model extent.
#[v2k_test_support::retail_test]
fn explicit_rows_abort_change_the_sea_by_their_shifted_word() {
    for (level_id, entity_type, expected_extent, expected_delta) in [
        (17, TYPE72_ENTITY_TYPE, 272, 0xb400),
        (43, TYPE85_ENTITY_TYPE, 150, 0x1_b000),
    ] {
        let (session, mut manager, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level_id);
        let id = ids(&manager, entity_type)[0];
        let lease = manager.main_base_abort_actor_observation(id).unwrap().lease;
        let extent = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .and_then(|entity| entity.model_index)
            .and_then(|model| session.cache.global_model(model))
            .unwrap()
            .radius;
        assert_eq!(extent, expected_extent, "type{entity_type}'s header+08");
        let advance = manager
            .apply_main_base_abort_type54_death(
                lease,
                session.cache.terrain().unwrap(),
                extent,
                MainBaseType54NetworkSession::SoloNetworkingDisabled,
                &mut fx,
            )
            .unwrap();
        let outcome = match advance {
            MainBaseType54DeathAdvance::Advanced { outcome, .. }
            | MainBaseType54DeathAdvance::SuccessorUnavailableAfterCommit { outcome } => outcome,
        };
        assert!(
            matches!(
                outcome,
                MainBaseType54DeathOutcome::SeaLevelTaskPublished {
                    entity_id,
                    sea_delta_raw,
                    ..
                } if entity_id == id && sea_delta_raw == expected_delta
            ),
            "{outcome:?}"
        );
    }
}
