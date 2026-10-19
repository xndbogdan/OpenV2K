//! World 17's Type72: Type54's class0 body with a running callback and an
//! explicit Change Sea Level delta.

use super::*;
use crate::main_base_type54_abort::{
    MainBaseType54DeathAdvance, MainBaseType54DeathOutcome, MainBaseType54NetworkSession,
    MainBaseType54SeaDeltaSource, TYPE72_ENTITY_TYPE,
};
use crate::specialized_actor_task_production::{
    SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
    SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
};

fn type72_ids(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| entity.entity_type == TYPE72_ENTITY_TYPE)
        .map(|entity| entity.id)
        .collect()
}

fn state(manager: &EntityManager, id: u32) -> RetailRuntimeValue<u32> {
    // Less the per-frame detail bits and the birth's surface pair.
    let mask = !(0x0600_0000 | crate::entity_collision_state::SURFACE_STATE_MASK);
    manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .masked(mask)
}

/// Retail's world-17 Type72s read `0x08468805` at their first 12DA0 visit
/// and `0x08428805` after their first callback.
#[v2k_test_support::retail_test]
fn world17_type72s_run_class0_from_the_retail_state() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(17);
    let ids = type72_ids(&manager);
    assert_eq!(ids.len(), 3);
    let mask = !(0x0600_0000 | crate::entity_collision_state::SURFACE_STATE_MASK);
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
            RetailRuntimeValue::Known(0x0846_8805 & mask)
        );
    }
    // Retail's presentation classified these far bodies coarse (no 411400
    // detail bits), so their first callback is E870's: class0's style sets
    // 0x2000, which clears motion 0x40000 and skips the walk and DF70.
    for &id in &ids {
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x0600_0000, 0);
    }
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert!(tasks.adopt_class0_actors(&manager) >= ids.len());
    let mut called = std::collections::BTreeSet::new();
    for tick in 1..=200 {
        let pass = tasks.tick(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world: SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut session.cache,
                world_fx: &mut fx,
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
                        callback_enabled: true,
                        ..
                    },
                ) if ids.contains(entity_id) => {
                    if called.insert(*entity_id) {
                        assert_eq!(
                            state(&manager, *entity_id),
                            RetailRuntimeValue::Known(0x0842_8805 & mask),
                            "tick{tick}"
                        );
                    }
                }
                SpecializedActorTaskProductionOutcome::Class0Actor(
                    Class0ActorOutcome::Blocked { reason, .. },
                ) => panic!("tick{tick}: {reason:?}"),
                _ => {}
            }
        }
    }
    assert_eq!(called.len(), ids.len());
}

/// 405040 takes Type72's nonzero `+88` as is: the class38 delta is
/// `0xB4 << 8`, not the derived height difference.
#[v2k_test_support::retail_test]
fn type72_abort_changes_the_sea_by_its_explicit_delta() {
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(17);
    let id = type72_ids(&manager)[0];
    let lease = manager.main_base_abort_actor_observation(id).unwrap().lease;
    let extent = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .and_then(|entity| entity.model_index)
        .and_then(|model| session.cache.global_model(model))
        .unwrap()
        .radius;
    assert_eq!(extent, 272, "model 1104's header+08 feeds 40BDC0's burst");
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
                sea_delta_raw: 0xb400,
                ..
            } if entity_id == id
        ),
        "{outcome:?}"
    );
}
