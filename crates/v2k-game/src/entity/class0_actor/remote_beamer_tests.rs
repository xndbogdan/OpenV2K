//! Type74, the class59 Remote Beamer of worlds 21, 31 and 36.

use super::*;
use crate::main_base_type54_abort::TYPE74_ENTITY_TYPE;
use crate::specialized_actor_task_production::{
    SpecializedActorTaskProductionFrame, SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
};

/// Retail's world-21 Type74 reads `(-14080, 3008, 32512)`, 50000 health and
/// `0x08468805` at its first visit. 40C530 publishes class59's Primary, whose
/// 054C0 charges Sub-K word 1 by `dt >> 4` to 0xFFFF; the model reads it in
/// selector 1 and the 50-Hz clock in selector 0.
#[v2k_test_support::retail_test]
fn world21_type74_charges_its_sub_k_word() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(21);
    let ids: Vec<u32> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == TYPE74_ENTITY_TYPE)
        .map(|entity| entity.id)
        .collect();
    assert_eq!(ids.len(), 1);
    let id = ids[0];
    let body = manager.iter_all().find(|entity| entity.id == id).unwrap();
    assert_eq!(body.position_raw(), [-14080, 3008, 32512]);
    assert_eq!(body.collision.health_raw, RetailRuntimeValue::Known(50_000));
    assert_eq!(
        body.collision.state_flags_at_0x08.masked(
            !(crate::entity_view_detail::BROADER_DETAIL_STATE_BIT
                | crate::entity_view_detail::FULL_DETAIL_STATE_BIT)
        ),
        RetailRuntimeValue::Known(0x0846_8805)
    );
    assert_eq!(
        body.actor_task_state(ActorTaskSlot::Primary),
        Some(&ActorTaskRuntime::RemoteBeamer(0))
    );
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert!(tasks.adopt_class0_actors(&manager) > 0);
    for retail_tick in 1..=60 {
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
                global_elapsed_micros: retail_tick * 20_000,
                retail_tick,
                main_base_abort_active: false,
            },
            &mut crate::gameplay_notifications::GameplayNotifications::new(),
        );
        assert!(pass.block.is_none(), "tick{retail_tick}: {:?}", pass.block);
        let body = manager.iter_all().find(|entity| entity.id == id).unwrap();
        let Some(&ActorTaskRuntime::RemoteBeamer(k1)) =
            body.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("tick{retail_tick}: class59 Primary");
        };
        // Every visit charges the 20 ms since the last, coarse or detailed.
        assert_eq!(u32::from(k1), (retail_tick * 20_000 >> 4).min(0xFFFF));
        let vars = body.presentation_anim_vars(retail_tick);
        assert_eq!(vars.dynamic[0], retail_tick as i32);
        assert_eq!(vars.dynamic[1], i32::from(k1));
    }
    let body = manager.iter_all().find(|entity| entity.id == id).unwrap();
    assert_eq!(
        body.actor_task_state(ActorTaskSlot::Primary),
        Some(&ActorTaskRuntime::RemoteBeamer(0xFFFF)),
        "60 ticks of 1250 saturate the word"
    );
}
