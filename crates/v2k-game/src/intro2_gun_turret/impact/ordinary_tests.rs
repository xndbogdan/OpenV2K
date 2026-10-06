use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    damage::{DamagePacket, FUN_0043F780_DAMAGE_PACKET},
    player_hull::PlayerHull,
    session::GameSession,
    shared_actor_impact::{
        apply_playing_actor_particle_hit, PlayingActorImpactFrame, SharedActorImpactOutcome,
    },
    world_fx::BallisticDamageRequest,
};

pub(crate) struct World {
    pub session: GameSession,
    pub manager: EntityManager,
    pub fx: WorldFx,
    pub scheduler: SpecializedActorTaskScheduler,
    pub static_damage: StaticDamageScheduler,
    pub notifications: GameplayNotifications,
    pub player_hull: PlayerHull,
    pub id: u32,
}
impl World {
    pub(crate) fn new(level: u32) -> Self {
        let (session, mut manager, mut fx) =
            crate::intro2_gun_turret::authored_tests::fixture(level);
        // Isolate later hit/death queues after the ordinary post-load sweep.
        manager.cleanup_pending_actor_deferred_destroys();
        let id = manager.iter_all().find(|e| e.entity_type == 97).unwrap().id;
        let ids: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
        for current in ids {
            let entity = manager.entity_mut(current).unwrap();
            // Isolate contact geometry without undoing the authored infection
            // that408EA0 already used to publish this task and live capability.
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(!0x2000, if entity.id == id { 0x0c06_8000 } else { 0 });
        }
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
        entity.set_position_raw([100, 10000, -300]);
        let mut scheduler = SpecializedActorTaskScheduler::default();
        assert!(scheduler.adopt_intro2_gun_turret(&manager) > 0);
        for _ in 0..8 {
            fx.advance_frame_pacing(20000);
        }
        Self {
            session,
            manager,
            fx,
            scheduler,
            static_damage: StaticDamageScheduler::new(),
            notifications: GameplayNotifications::new(),
            player_hull: PlayerHull::default(),
            id,
        }
    }

    fn hit(&mut self, class: u8, amount: i32) -> Intro2GunTurretImpactOutcome {
        let outcome = self.shared_hit(class, amount);
        let Some(SharedActorImpactOutcome::GunTurret(outcome)) = outcome else {
            panic!("{outcome:?}")
        };
        outcome
    }

    fn shared_hit(&mut self, class: u8, amount: i32) -> Option<SharedActorImpactOutcome> {
        apply_playing_actor_particle_hit(
            PlayingActorImpactFrame {
                extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                resources: &mut self.session.cache,
                entities: &mut self.manager,
                world_fx: &mut self.fx,
                scheduler: &mut self.scheduler,
                notifications: &mut self.notifications,
                static_damage: &mut self.static_damage,
                player_hull: &mut self.player_hull,
                retail_tick: 501,
            },
            ParticleEntityImpact {
                source_particle_class: class,
                impact_position_argument_va: if class == 5 { 0x004d_cf48 } else { 0 },
                target_entity_id: self.id,
                position_world: [0.0; 3],
                velocity_raw: [0, 0, 8192],
                damage: Some(BallisticDamageRequest {
                    packet: if class == 5 {
                        FUN_0043F780_DAMAGE_PACKET
                    } else {
                        DamagePacket {
                            channels: [2, 0],
                            amounts_raw: [amount, 0],
                        }
                    },
                    source_entity_type_at_birth: Some(34),
                    source_owner_id: Some(35),
                }),
            },
        )
    }
}

#[v2k_test_support::retail_test]
fn ordinary_type97_primary_and_infected_keep_distinct_slots_and_actual_components() {
    for level in [31, 42, 46, 47] {
        for infected in [false, true] {
            let mut f = World::new(level);
            let entity = f.manager.entity_mut(f.id).unwrap();
            let runtime = entity.intro2_gun_turret_runtime;
            let task = entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary);
            let capability = entity.capability_flags;
            let mut expected = f.fx.fork_for_main_base_abort_transaction();
            if infected {
                expected.next_shared_retail_random_u16();
            }
            let result = f.hit(if infected { 5 } else { 16 }, 3100);
            let Intro2GunTurretImpactOutcome::Applied(applied) = result else {
                panic!("{result:?}")
            };
            assert_eq!(applied.filtered_damage_raw, if infected { 0 } else { 1000 });
            let entity = f.manager.entity_mut(f.id).unwrap();
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(if infected { 5000 } else { 4000 })
            );
            assert_eq!(
                entity.collision.last_hit_presentation_tick_at_0x34,
                RetailRuntimeValue::Known(if infected { 17 } else { 501 })
            );
            assert_eq!(entity.intro2_gun_turret_runtime, runtime);
            assert_eq!(
                task == entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary),
                !infected
            );
            assert_eq!(
                entity.capability_flags,
                if infected {
                    (capability & !0x1004) | 8
                } else {
                    capability
                },
                "world{level}, infected entry{infected}: primary+28 is null"
            );
            assert!(f
                .scheduler
                .intro2_gun_turret_completed_owner(&f.manager, f.id));
            assert_eq!(
                f.fx.next_shared_retail_random_u16(),
                expected.next_shared_retail_random_u16()
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn ordinary_type97_playing_lethal_hit_finishes_class49_and_rehit_cannot_replay_blast() {
    for level in [31, 42, 46, 47] {
        let mut f = World::new(level);
        let result = f.hit(16, 7100);
        assert!(
            matches!(result, Intro2GunTurretImpactOutcome::Applied(_)),
            "{result:?}"
        );
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &f.manager, f.id
        ));
        assert_eq!(f.manager.pending_actor_deferred_destroy_ids(), &[f.id]);
        assert_eq!(
            f.manager
                .iter_all()
                .filter(|entity| entity.entity_type == 60)
                .count(),
            1
        );
        let particles = f.fx.particle_count();
        let entity = f.manager.entity_mut(f.id).unwrap();
        let terminal = entity.class49_death_runtime;
        let suffix_count = if entity.capability_flags & 8 != 0 {
            2
        } else {
            0
        };
        let mut expected = f.fx.fork_for_main_base_abort_transaction();
        let result = f.hit(16, 7100);
        let Intro2GunTurretImpactOutcome::Applied(checked) = result else {
            panic!("{result:?}")
        };
        assert!(checked.death_publication.is_none());
        assert_eq!(
            f.manager.entity_mut(f.id).unwrap().class49_death_runtime,
            terminal
        );
        assert_eq!(f.manager.pending_actor_deferred_destroy_ids(), &[f.id]);
        assert_eq!(
            f.manager
                .iter_all()
                .filter(|entity| entity.entity_type == 60)
                .count(),
            1
        );
        //10EB0's independent live capability8 suffix survives dying. Full-rate
        //40DC0 emits exactly two class5 carriers, advancing directions, not RNG.
        assert_eq!(f.fx.particle_count(), particles + suffix_count);
        assert!(f.fx.test_particles_in_virgin_birth_order()[particles..]
            .iter()
            .all(|particle| particle.source_class == 5));
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn ordinary_type97_nested_radial_death_uses_the_receipt_backed_playing_owner() {
    let mut f = World::new(42);
    let second = f
        .manager
        .iter_all()
        .find(|e| e.entity_type == 97 && e.id != f.id)
        .unwrap()
        .id;
    let entity = f.manager.entity_mut(second).unwrap();
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(!0x2000, 0x0c06_8000);
    entity.collision.health_raw = RetailRuntimeValue::Known(3800);
    entity.set_position_raw([200, 10000, -300]);
    let result = f.hit(16, 7100);
    assert!(
        matches!(result, Intro2GunTurretImpactOutcome::Applied(_)),
        "{result:?}"
    );
    assert_eq!(
        f.manager.pending_actor_deferred_destroy_ids(),
        &[second, f.id]
    );
    for id in [second, f.id] {
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &f.manager, id
        ));
    }
    assert_eq!(
        f.manager
            .iter_all()
            .filter(|entity| entity.entity_type == 60)
            .count(),
        2
    );
}

#[v2k_test_support::retail_test]
fn ordinary_type97_foreign_manager_and_411180_entries_reject_before_hit_prefix() {
    for class in [5, 16, 52, 68, 85] {
        let mut f = World::new(31);
        if matches!(class, 5 | 16) {
            let mut other = World::new(31);
            std::mem::swap(
                f.manager.entity_mut(f.id).unwrap(),
                other.manager.entity_mut(other.id).unwrap(),
            );
        }
        let before = f.manager.entity_mut(f.id).unwrap().collision.clone();
        let mut expected = f.fx.fork_for_main_base_abort_transaction();
        let result = f.shared_hit(class, 7100);
        if matches!(class, 52 | 68 | 85) {
            assert!(
                matches!(
                    result,
                    Some(SharedActorImpactOutcome::UnsupportedStaticRouteTarget {
                        entity_type: 97
                    })
                ),
                "{result:?}"
            );
        } else {
            assert!(
                matches!(
                    result,
                    Some(SharedActorImpactOutcome::GunTurret(
                        Intro2GunTurretImpactOutcome::Blocked {
                            committed_prefix: false,
                            ..
                        }
                    ))
                ),
                "{result:?}"
            );
        }
        assert_eq!(f.manager.entity_mut(f.id).unwrap().collision, before);
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
    }
}
