use super::*;
use crate::static_damage::StaticDamageScheduler;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime, actor_task_owner::ActorTaskSlot,
    entity_collision_state::RetailRuntimeValue, intro2_type47_live::world::native_intro2_fixture,
    intro2_type53::Intro2Type53Owner,
};

#[v2k_test_support::retail_test]
fn native_surface_death_changes_scheduler_family_without_same_pass_dying_tick() {
    for (spawn, kind) in [(38, 53), (4, 17), (30, 17)] {
        let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn))
            .unwrap()
            .id;
        let terrain = session.cache.level_terrain().unwrap();
        // Sub-C corrects penetration before E370. Use an actual deep-water point,
        // rather than placing the actor below dry land and expecting it to stay.
        let position = (0..256)
            .flat_map(|x| {
                (0..256).map(move |z| {
                    let x = (x << 8) as i16;
                    let z = (z << 8) as i16;
                    [x, terrain.bilinear_height_raw(x, z).saturating_add(256), z]
                })
            })
            .min_by_key(|position| position[1])
            .unwrap();
        assert!(position[1] < terrain.sea_level_raw() - 512);
        let entity = manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(u32::MAX, 0x0206_8000);
        entity.set_motion_raw(position, [0; 3]);
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(1_999);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(250_000);
        entity.collision.callback_scheduler_accumulator_us_at_0x6c =
            RetailRuntimeValue::Known(125_001);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        if kind == 17 {
            scheduler.register_intro2_type17(
                crate::intro2_type17::Intro2Type17Owner::adopt(&manager, id).unwrap(),
            );
        } else {
            scheduler.register_intro2_type53(Intro2Type53Owner::adopt(&manager, id).unwrap());
        }
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let mut static_damage = StaticDamageScheduler::new();
        for pass_index in 0..2 {
            let pass = scheduler.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase:
                        crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                    resources: &mut session.cache,
                    world_fx: &mut world_fx,
                    static_damage: &mut static_damage,
                    elapsed_micros: 125_000,
                    global_elapsed_micros: 125_000,
                    retail_tick: 17 + pass_index,
                    main_base_abort_active: false,
                },
                &mut notifications,
            );
            assert_eq!(pass.block, None);
            assert_eq!(
                pass.outcomes.len(),
                1,
                "a replacement never creates an extra same-pass callback"
            );
            assert_eq!(
                scheduler.family_for(id),
                Some(SpecializedActorTaskFamily::Intro2CommonDying),
                "{:?}",
                pass.outcomes
            );
            assert_eq!(scheduler.owners.len(), 1);
            let entity = manager.entity_mut(id).unwrap();
            let Some(ActorTaskRuntime::CommonDying(task)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("{:?}", pass.outcomes)
            };
            assert_eq!(
                task.elapsed_ms(),
                if pass_index == 0 { 0 } else { 125 },
                "{:?}",
                pass.outcomes
            );
            if pass_index == 0 {
                assert!(matches!(
                    pass.outcomes[0],
                    SpecializedActorTaskProductionOutcome::Intro2Type53(_)
                        | SpecializedActorTaskProductionOutcome::Intro2Type17(_)
                ));
            } else {
                assert!(matches!(
                    pass.outcomes[0],
                    SpecializedActorTaskProductionOutcome::Intro2CommonDying(_)
                ));
            }
        }
    }
}
