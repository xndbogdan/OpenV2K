use super::*;
use crate::{
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit},
    gameplay_notifications::GameplayNotifications,
    native_type122::construction_tests::native_fixture_with_player,
};

#[v2k_test_support::retail_test]
fn casualty_castle_shared_abort_completes_the_authored_actor_walk() {
    use crate::power_up_contact::PlayerCampaignProgress;
    use std::num::NonZeroU64;

    let (mut session, mut manager, mut fx) = native_fixture_with_player(15);
    let mut tasks = SpecializedActorTaskScheduler::new();
    tasks
        .adopt_fresh_level1_type9_selected(&mut manager)
        .unwrap();
    tasks.adopt_fresh_level1_type17_follow_beacons(&manager);
    tasks
        .adopt_fresh_level1_type47_scheduler(&mut manager)
        .unwrap();
    tasks.adopt_intro2_type13_search_attack(&manager);
    tasks.adopt_intro2_type26(&manager);
    tasks.adopt_intro2_type47_guards(&manager);
    tasks.adopt_intro2_flyers(&manager);
    tasks.adopt_intro2_type53(&manager);
    tasks.adopt_type122(&manager);
    tasks.adopt_shared_fish(&manager);
    tasks.adopt_cleansing_vehicle(&manager);
    tasks.adopt_intro2_type16(&manager);
    tasks.adopt_intro2_type58(&manager);
    tasks.adopt_intro2_type94(&manager);
    tasks.adopt_intro2_type66(&manager);
    tasks.adopt_class0_actors(&manager);
    tasks.adopt_main_base(&manager);
    tasks.adopt_intro2_type10(&manager);
    tasks.adopt_intro2_type57(&manager);
    tasks.adopt_intro2_gun_turret(&manager);
    tasks.adopt_intro2_type17(&manager);
    tasks.adopt_intro2_type8(&mut manager);
    tasks.adopt_intro2_type9(&mut manager);
    tasks.adopt_native_type123(&mut manager);
    tasks.adopt_native_type86(&mut manager);
    tasks.adopt_intro2_meteors(&manager);
    let count_before = manager.iter_all().count();
    let mut progress = PlayerCampaignProgress::new();
    progress.set_current_control_slot(Some(3));
    let (mut controller, _) = MainBaseAbortControllerStorage::from_loaded_level(
        NonZeroU64::new(15).unwrap(),
        session.cache.level_desc().unwrap(),
        false,
        &mut progress,
    )
    .unwrap();
    let result = execute_campaign_abort(
        MainBaseAbortTransactionId::new(15).unwrap(),
        None,
        &mut controller,
        &mut manager,
        &mut session.cache,
        &mut StaticDamageScheduler::new(),
        &mut PlayerHull::default(),
        &mut fx,
        &mut tasks,
        MainBaseAbortGameplayContext {
            extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            notifications: &mut GameplayNotifications::new(),
            retail_tick: 0x1dcb,
        },
    )
    .unwrap_or_else(|failure| panic!("Castle casualty abort: {:?}", failure.diagnostic));
    assert!(result.processed_actors.len() >= count_before);
    assert_eq!(result.publications.main_base_production, 1);
    assert!(result.completion.full_frame_submitted);
    assert_eq!(controller.player_state(), RetailRuntimeValue::Known(5));
}

#[v2k_test_support::retail_test]
fn casualty_walk_starts_native_main_base_progression_and_replaces_its_task_owner() {
    let (_, mut manager, mut fx) = native_fixture_with_player(15);
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 6)
        .unwrap()
        .id;
    let primary_before = manager
        .entity_mut(id)
        .unwrap()
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary);
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_main_base(&manager), 1);
    let mut oracle = fx.fork_for_main_base_abort_transaction();
    oracle.next_shared_retail_random_u16(); // AC60's singleton reselector.
    let mut publications = MainBaseAbortPublicationCounts::default();
    let observation = manager.main_base_abort_actor_observation(id).unwrap();
    let result = dispatch_native_actor(
        observation,
        &mut manager,
        &mut fx,
        &mut tasks,
        &mut publications,
        &mut MainBaseAbortGameplayContext {
            extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            notifications: &mut GameplayNotifications::new(),
            retail_tick: 0x1dcb,
        },
    )
    .unwrap()
    .unwrap_or_else(|failure| panic!("{:?}", failure.callback_error));
    assert_eq!(
        result.disposition,
        MainBaseAbortActorDisposition::MainBaseDeath
    );
    assert_eq!(publications.main_base_production, 1);
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity.collision.health_raw,
        RetailRuntimeValue::Known(10_000_000)
    );
    assert_eq!(
        entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
        RetailRuntimeValue::Known(0)
    );
    let RetailRuntimeValue::Known(Some(base)) = entity.base_factory_runtime else {
        panic!()
    };
    assert_eq!(base.progressive_death.elapsed_micros_raw, 1);
    assert_ne!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        primary_before
    );
    assert!(
        tasks.prepare_native_actor_mutation(&manager, id),
        "replacement task owns the next-frame callback"
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn casualty_main_base_rejects_missing_or_executing_owner_before_death_prefix() {
    for inside_callback in [false, true] {
        let (_, mut manager, mut fx) = native_fixture_with_player(15);
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 6)
            .unwrap()
            .id;
        let mut tasks = SpecializedActorTaskScheduler::new();
        if inside_callback {
            assert_eq!(tasks.adopt_main_base(&manager), 1);
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
        }
        let before = manager.entity_mut(id).unwrap().collision.clone();
        let mut oracle = fx.fork_for_main_base_abort_transaction();
        let observation = manager.main_base_abort_actor_observation(id).unwrap();
        let result = dispatch_native_actor(
            observation,
            &mut manager,
            &mut fx,
            &mut tasks,
            &mut MainBaseAbortPublicationCounts::default(),
            &mut MainBaseAbortGameplayContext {
                extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                notifications: &mut GameplayNotifications::new(),
                retail_tick: 0x1dcb,
            },
        )
        .unwrap();
        assert!(result.is_err());
        assert_eq!(manager.entity_mut(id).unwrap().collision, before);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }
}
