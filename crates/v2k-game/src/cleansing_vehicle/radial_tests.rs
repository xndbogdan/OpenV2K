use crate::{
    actor_task_owner::ActorTaskSlot,
    class49_death::finished_terminal_hit_authenticates,
    class49_terminal::{run_class49_standard_death, Class49TerminalFrame, Class49WorldContext},
    damage::DamagePacket,
    entity_collision_state::RetailRuntimeValue,
    gameplay_notifications::GameplayNotifications,
    player_hull::PlayerHull,
    radial_damage::RadialDamageTemplate,
    specialized_actor_task_production::{PlayingRadialFrame, SpecializedActorTaskScheduler},
    static_damage::StaticDamageScheduler,
};

#[v2k_test_support::retail_test]
fn playing_radial_hit_runs_machine_death_and_its_reentrant_self_visit() {
    let (mut session, mut manager, mut fx) = super::tests::fixture(15);
    // This hit occurs after the load's queued gate-helper removals.
    manager.cleanup_pending_actor_deferred_destroys();
    let id = manager.iter_all().find(|e| e.entity_type == 49).unwrap().id;
    let origin_raw = [0, -30_000, 0];
    let entity = manager.entity_mut(id).unwrap();
    entity.set_position_raw(origin_raw);
    entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_cleansing_vehicle(&manager);
    let mut hull = PlayerHull::default();
    let mut notifications = GameplayNotifications::new();
    let mut static_damage = StaticDamageScheduler::new();
    let result = scheduler.apply_playing_radial_damage(PlayingRadialFrame {
        extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
        entities: &mut manager,
        player_hull: &mut hull,
        origin_raw,
        template: RadialDamageTemplate {
            inner_radius_raw: 8,
            outer_radius_raw: 16,
            impulse_raw: 0,
            packet: DamagePacket {
                channels: [2, 3],
                amounts_raw: [3000, 1000],
            },
            trailing_raw: [0, 0],
        },
        world_fx: &mut fx,
        notifications: &mut notifications,
        retail_tick: 0x644,
        resources: &mut session.cache,
        static_damage: &mut static_damage,
        active_terminal_calls: Vec::new(),
    });
    assert!(result.blocked.is_none(), "{result:?}");
    assert!(result.completed_target_ids.contains(&id));
    assert!(finished_terminal_hit_authenticates(&manager, id));
    assert_eq!(manager.pending_actor_deferred_destroy_ids(), &[id]);
    assert_eq!(
        manager.iter_all().filter(|e| e.entity_type == 60).count(),
        1
    );
}

#[v2k_test_support::retail_test]
fn playing_machine_late_radial_block_parks_source_without_replaying_blast() {
    let (mut session, mut manager, mut fx) = super::tests::fixture(15);
    // This hit occurs after the load's queued gate-helper removals.
    manager.cleanup_pending_actor_deferred_destroys();
    let id = manager.iter_all().find(|e| e.entity_type == 49).unwrap().id;
    let origin_raw = [0, -30_000, 0];
    let entity = manager.entity_mut(id).unwrap();
    entity.set_position_raw(origin_raw);
    entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
    let before_tasks =
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot));
    let player_id = manager.player().unwrap().id;
    let player = manager.entity_mut(player_id).unwrap();
    player.set_position_raw(origin_raw);
    player.collision.health_raw = RetailRuntimeValue::Unresolved;
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_cleansing_vehicle(&manager);
    let mut hull = PlayerHull::default();
    let mut notifications = GameplayNotifications::new();
    let mut static_damage = StaticDamageScheduler::new();
    let result = run_class49_standard_death(
        Class49TerminalFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: 0x644,
            world: Class49WorldContext::Playing {
                extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                scheduler: &mut scheduler,
                player_hull: &mut hull,
                active_terminal_calls: Vec::new(),
            },
        },
        id,
    );
    assert!(
        result.is_err(),
        "the unresolved player must stop the radial suffix"
    );
    assert!(scheduler.has_native_contact_prefix(id));
    assert!(!scheduler.begin_cleansing_vehicle_external_mutation(&manager, id));
    assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot)),
        before_tasks
    );
    let particles = fx.particle_count();
    assert!(particles > 0);
    let mut expected_rng = fx.fork_for_main_base_abort_transaction();
    let replay = run_class49_standard_death(
        Class49TerminalFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: 0x645,
            world: Class49WorldContext::Playing {
                extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                scheduler: &mut scheduler,
                player_hull: &mut hull,
                active_terminal_calls: Vec::new(),
            },
        },
        id,
    )
    .unwrap();
    assert!(!replay.returned_nonzero);
    assert_eq!(fx.particle_count(), particles);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_rng.next_shared_retail_random_u16()
    );
}
