//! The diver's own damage table and native receipt reach the Playing dispatcher.
use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    damage::{DamagePacket, FUN_0043F780_DAMAGE_PACKET},
    entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT},
    native_type122::construction_tests::native_fixture_with_player,
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskFamily,
    world_fx::BallisticDamageRequest,
};

struct Fixture {
    session: GameSession,
    entities: EntityManager,
    fx: WorldFx,
    scheduler: SpecializedActorTaskScheduler,
    notifications: GameplayNotifications,
    id: u32,
}

impl Fixture {
    fn new() -> Self {
        let (session, mut entities, mut fx) = native_fixture_with_player(34);
        entities.cleanup_pending_actor_deferred_destroys();
        let id = entities.iter_all().find(|e| e.entity_type == 7).unwrap().id;
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_native_type86(&mut entities), 5);
        fx.process_pending();
        fx.take_positional_sounds();
        Self {
            session,
            entities,
            fx,
            scheduler,
            notifications: GameplayNotifications::new(),
            id,
        }
    }

    fn hit(&mut self, infected: bool, amount: i32, source_type: u8) -> SharedActorImpactOutcome {
        let source_owner = if source_type == 46 {
            self.entities.player().unwrap().id
        } else {
            0x045f_0001
        };
        apply_shared_actor_particle_hit(
            SharedActorImpactFrame {
                resources: &self.session.cache,
                entities: &mut self.entities,
                world_fx: &mut self.fx,
                scheduler: &mut self.scheduler,
                notifications: &mut self.notifications,
                retail_tick: 5000,
            },
            ParticleEntityImpact {
                source_particle_class: if infected { 5 } else { 16 },
                impact_position_argument_va: if infected { 0x004d_cf48 } else { 0 },
                target_entity_id: self.id,
                position_world: [0.0; 3],
                velocity_raw: [0, 0, 8192],
                damage: Some(BallisticDamageRequest {
                    packet: if infected {
                        FUN_0043F780_DAMAGE_PACKET
                    } else {
                        DamagePacket {
                            channels: [1, 0],
                            amounts_raw: [amount, 0],
                        }
                    },
                    source_entity_type_at_birth: Some(source_type),
                    source_owner_id: Some(source_owner),
                }),
            },
        )
        .expect("native Type7 must select its shared four-choice owner")
    }
}

#[v2k_test_support::retail_test]
fn type7_player_kill_feedback_uses_post_death_infected_state_without_replaying_death() {
    for source in [34, 46] {
        for infected_model in [false, true] {
            let mut f = Fixture::new();
            let actor = f.entities.entity_mut(f.id).unwrap();
            assert_eq!(actor.collision.health_raw, RetailRuntimeValue::Known(1500));
            actor
                .collision
                .state_flags_at_0x08
                .overwrite(0x0100_0000, if infected_model { 0x0100_0000 } else { 0 });
            let receipt = actor.native_type86_runtime;
            let result = f.hit(false, 7000, source);
            assert!(
                matches!(result, SharedActorImpactOutcome::Type86(NativeType86ImpactOutcome::Applied(ref applied))
                if applied.filtered_damage_raw == 3000 && applied.death_publication.is_some()),
                "{result:?}"
            );
            let actor = f.entities.entity_mut(f.id).unwrap();
            assert_eq!(actor.collision.health_raw, RetailRuntimeValue::Known(0));
            assert_eq!(actor.native_type86_runtime, receipt);
            let death_task = actor
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            assert!(
                matches!(actor.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::SharedRetarget(task)) if task.lifetime_ms() == 1000)
            );
            let expected = if source == 46 && infected_model {
                1 << 4
            } else {
                0
            };
            assert_eq!(f.notifications.save_tail_seen_mask(), expected);
            let mut expected_notifications = GameplayNotifications::new();
            if expected != 0 {
                expected_notifications.queue_player_kill(5000);
            }
            assert_eq!(f.notifications, expected_notifications);
            assert!(f
                .scheduler
                .begin_native_type86_external_mutation(&f.entities, f.id));

            // A second projectile sees the real already-dying Class14 owner;
            // 14E90 cannot re-enter death or refresh the notification timestamp.
            let result = f.hit(false, 7000, source);
            assert!(
                matches!(result, SharedActorImpactOutcome::Type86(NativeType86ImpactOutcome::Applied(ref applied))
                if applied.death_publication.is_none()),
                "{result:?}"
            );
            assert_eq!(
                f.entities
                    .entity_mut(f.id)
                    .unwrap()
                    .actor_tasks
                    .task_in_slot(ActorTaskSlot::Primary),
                Some(death_task)
            );
            assert_eq!(f.notifications, expected_notifications);
        }
    }
}

#[v2k_test_support::retail_test]
fn type7_primary_and_infected_hits_use_own_filter_and_entry_prefix() {
    for infected in [false, true] {
        let mut f = Fixture::new();
        let actor = f.entities.entity_mut(f.id).unwrap();
        let receipt = actor.native_type86_runtime;
        actor.collision.health_raw = RetailRuntimeValue::Known(5000);
        actor.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
        let result = f.hit(infected, 4500, 34);
        let SharedActorImpactOutcome::Type86(NativeType86ImpactOutcome::Applied(applied)) = result
        else {
            panic!("{result:?}");
        };
        // Actual Type7 channel1 threshold4000/multiplier256 and channel6
        // threshold0/multiplier256 differ from the person profile's immunity.
        let damage = if infected { 2000 } else { 500 };
        assert_eq!(applied.filtered_damage_raw, damage);
        let actor = f.entities.entity_mut(f.id).unwrap();
        assert_eq!(
            actor.collision.health_raw,
            RetailRuntimeValue::Known(5000 - damage)
        );
        assert_eq!(actor.native_type86_runtime, receipt);
        assert_eq!(actor.model_slots, [Some(1249); 4]);
        assert_eq!(actor.capability_flags, 0x1404);
        assert_eq!(
            actor.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(if infected { 17 } else { 5000 })
        );
        assert_eq!(
            f.scheduler.family_for(f.id),
            Some(SpecializedActorTaskFamily::NativeType86)
        );
        assert!(f
            .scheduler
            .begin_native_type86_external_mutation(&f.entities, f.id));
    }
}

#[v2k_test_support::retail_test]
fn type7_lethal_primary_and_infected_hits_publish_class14_and_own_death_cue() {
    for infected in [false, true] {
        let mut f = Fixture::new();
        let receipt = f.entities.entity_mut(f.id).unwrap().native_type86_runtime;
        let result = f.hit(infected, 7000, 34);
        let SharedActorImpactOutcome::Type86(NativeType86ImpactOutcome::Applied(applied)) = result
        else {
            panic!("{result:?}");
        };
        assert_eq!(
            applied.filtered_damage_raw,
            if infected { 2000 } else { 3000 }
        );
        let actor = f.entities.entity_mut(f.id).unwrap();
        assert_eq!(actor.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            actor.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
        assert_eq!(actor.native_type86_runtime, receipt);
        assert!(
            matches!(actor.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::SharedRetarget(task)) if task.lifetime_ms() == 1000)
        );
        assert!(f
            .scheduler
            .begin_native_type86_external_mutation(&f.entities, f.id));
        f.fx.process_pending();
        assert_eq!(
            f.fx.take_positional_sounds()
                .iter()
                .filter(|sound| sound.sound_id == 35)
                .count(),
            1
        );
    }
}

#[v2k_test_support::retail_test]
fn type7_foreign_or_parked_owner_rejects_both_hit_entries_before_writes() {
    for infected in [false, true] {
        for foreign in [false, true] {
            let mut f = Fixture::new();
            if foreign {
                let other = Fixture::new();
                f.entities.entity_mut(f.id).unwrap().native_type86_runtime = other
                    .entities
                    .iter_all()
                    .find(|e| e.id == other.id)
                    .unwrap()
                    .native_type86_runtime;
            } else {
                f.scheduler.park_native_type86_external_prefix(f.id);
            }
            let actor = f.entities.entity_mut(f.id).unwrap();
            let before = (
                actor.collision.clone(),
                actor.current_behavior_context,
                actor.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                actor.actor_animation_runtime,
            );
            let pending = f.fx.pending_event_count();
            let mut oracle = f.fx.fork_for_main_base_abort_transaction();
            let result = f.hit(infected, 7000, 34);
            assert!(
                matches!(
                    result,
                    SharedActorImpactOutcome::Type86(NativeType86ImpactOutcome::Blocked {
                        committed_prefix: false,
                        ..
                    })
                ),
                "{result:?}"
            );
            let actor = f.entities.entity_mut(f.id).unwrap();
            assert_eq!(
                (
                    actor.collision.clone(),
                    actor.current_behavior_context,
                    actor.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                    actor.actor_animation_runtime
                ),
                before
            );
            assert_eq!(f.fx.pending_event_count(), pending);
            assert_eq!(
                f.fx.next_shared_retail_random_u16(),
                oracle.next_shared_retail_random_u16()
            );
        }
    }
}
