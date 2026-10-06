use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    damage::{DamagePacket, FUN_0043F780_DAMAGE_PACKET},
    intro2_type47_live::world::native_intro2_fixture,
    session::GameSession,
    world_fx::BallisticDamageRequest,
};

struct World {
    session: GameSession,
    manager: EntityManager,
    fx: WorldFx,
    scheduler: SpecializedActorTaskScheduler,
    static_damage: StaticDamageScheduler,
    notifications: GameplayNotifications,
    id: u32,
}
impl World {
    fn new(spawn: usize) -> Option<Self> {
        let (session, mut manager, _) = native_intro2_fixture()?;
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn))
            .unwrap()
            .id;
        let ids: Vec<_> = manager.retail_live_order_ids().collect();
        for other in ids {
            if other != id {
                manager
                    .entity_mut(other)
                    .unwrap()
                    .collision
                    .state_flags_at_0x08
                    .overwrite(u32::MAX, 0);
            }
        }
        let entity = manager.entity_mut(id).unwrap();
        // The native Intro2 constructor owns Section-12's retained null
        // type-hit hook; generic from-level construction leaves it unresolved.
        assert_eq!(
            entity.collision.pair_callbacks.type_hit_callback_address,
            RetailRuntimeValue::Known(None)
        );
        //Keep the native allocation/tasks/profile. This controlled local
        //fixed state isolates the complete hit chain from unrelated actors.
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(u32::MAX, 0x0c06_8000);
        entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
        entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
        entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        entity.set_position_raw([100, 10_000, -300]);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.register_intro2_gun_turret(Intro2GunTurretOwner::adopt(&manager, id).unwrap());
        Some(Self {
            session,
            manager,
            fx: WorldFx::new(),
            scheduler,
            static_damage: StaticDamageScheduler::new(),
            notifications: GameplayNotifications::default(),
            id,
        })
    }
    fn hit(&mut self, infected: bool, amount: i32, tick: u32) -> Intro2GunTurretImpactOutcome {
        apply_intro2_gun_turret_particle_hit(
            Intro2GunTurretImpactFrame {
                world: GunTurretImpactWorld::Cinematic,
                entities: &mut self.manager,
                resources: &mut self.session.cache,
                static_damage: &mut self.static_damage,
                notifications: &mut self.notifications,
                world_fx: &mut self.fx,
                scheduler: &mut self.scheduler,
                retail_tick: tick,
            },
            ParticleEntityImpact {
                source_particle_class: if infected { 5 } else { 16 },
                impact_position_argument_va: if infected { 0x004dcf48 } else { 0 },
                target_entity_id: self.id,
                position_world: [0.0; 3],
                velocity_raw: [0, 0, 8192],
                damage: Some(BallisticDamageRequest {
                    packet: if infected {
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
    fn task(&mut self) -> crate::actor_task_owner::ActorTaskId {
        self.manager
            .entity_mut(self.id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary)
            .unwrap()
    }
}

#[v2k_test_support::retail_test]
fn turret_primary_null_hook_retains_task_and_rng_on_filtered_and_accepted_hits() {
    for amount in [0, 2100, 2101] {
        let Some(mut world) = World::new(53) else {
            return;
        };
        let task = world.task();
        let runtime = world
            .manager
            .entity_mut(world.id)
            .unwrap()
            .intro2_gun_turret_runtime;
        let mut expected = world.fx.fork_for_main_base_abort_transaction();
        let result = world.hit(false, amount, 91);
        let Intro2GunTurretImpactOutcome::Applied(result) = result else {
            panic!("{result:?}")
        };
        let filtered = if amount > 2100 { amount - 2100 } else { 0 };
        assert_eq!(result.filtered_damage_raw, filtered);
        assert_eq!(world.task(), task);
        let entity = world.manager.entity_mut(world.id).unwrap();
        assert_eq!(entity.intro2_gun_turret_runtime, runtime);
        assert_eq!(
            entity.collision.health_raw,
            RetailRuntimeValue::Known(4000 - filtered)
        );
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(91)
        );
        assert!(world
            .scheduler
            .intro2_gun_turret_completed_owner(&world.manager, world.id));
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
        world.fx.process_pending();
        assert!(world.fx.take_positional_sounds().is_empty());
    }
}

#[v2k_test_support::retail_test]
fn turret_infected_zero_damage_still_reselects_then_primary_uses_live_capability_suffix() {
    let Some(mut world) = World::new(54) else {
        return;
    };
    let task = world.task();
    let runtime = world
        .manager
        .entity_mut(world.id)
        .unwrap()
        .intro2_gun_turret_runtime;
    let mut expected = world.fx.fork_for_main_base_abort_transaction();
    expected.next_shared_retail_random_u16(); //DA00/C690 one AC60 selector
    let result = world.hit(true, 0, 92);
    assert!(
        matches!(result,Intro2GunTurretImpactOutcome::Applied(ref checked) if checked.filtered_damage_raw==0),
        "{result:?}"
    );
    assert_ne!(world.task(), task);
    let entity = world.manager.entity_mut(world.id).unwrap();
    assert_eq!(entity.capability_flags, 0x48);
    assert_eq!(entity.intro2_gun_turret_runtime, runtime);
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(4000));
    assert_eq!(
        entity.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(17)
    );
    assert_eq!(
        world.fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
    assert!(world
        .scheduler
        .intro2_gun_turret_completed_owner(&world.manager, world.id));
    //An ordinary accepted hit after infection reads livecap48, not birth44.
    for _ in 0..8 {
        world.fx.advance_frame_pacing(20_000);
    }
    let task = world.task();
    let count = world.fx.particle_count();
    let result = world.hit(false, 2101, 93);
    assert!(
        matches!(result, Intro2GunTurretImpactOutcome::Applied(_)),
        "{result:?}"
    );
    assert_eq!(world.task(), task);
    assert_eq!(world.fx.particle_count(), count + 2);
    let prepared = world.fx.prepare_presentation([640, 480], 0x3000, |_| {
        v2k_render::ParticleCenterProjection {
            screen: [320, 240],
            depth_raw: 1000,
            clip: 0,
        }
    });
    assert!(prepared.particles().any(|p| p.particle.source_class == 5));
}

#[v2k_test_support::retail_test]
fn turret_pending_owner_rejects_both_entries_before_timestamp_rng_or_reselection() {
    for infected in [false, true] {
        let Some(mut world) = World::new(53) else {
            return;
        };
        let owner = Intro2GunTurretOwner::adopt_blocked_prefix(&world.manager, world.id).unwrap();
        world.scheduler.register_intro2_gun_turret(owner);
        let task = world.task();
        let before = world
            .manager
            .entity_mut(world.id)
            .unwrap()
            .intro2_gun_turret_runtime;
        let mut expected = world.fx.fork_for_main_base_abort_transaction();
        assert_eq!(
            world.hit(infected, 2101, 99),
            Intro2GunTurretImpactOutcome::Blocked {
                reason: Intro2GunTurretImpactBlock::Runtime("pending actor prefix"),
                committed_prefix: false
            }
        );
        assert_eq!(world.task(), task);
        let entity = world.manager.entity_mut(world.id).unwrap();
        assert_eq!(entity.intro2_gun_turret_runtime, before);
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(17)
        );
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(4000));
        assert!(world
            .scheduler
            .intro2_gun_turret_has_pending_prefix(world.id));
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn turret_lethal_primary_completes_blast_ring_and_retirement_inside_hit() {
    let Some(mut world) = World::new(53) else {
        return;
    };
    for _ in 0..8 {
        world.fx.advance_frame_pacing(20_000);
    }
    let result = world.hit(false, 10_000, 111);
    let Intro2GunTurretImpactOutcome::Applied(checked) = result else {
        panic!("{result:?}")
    };
    assert_eq!(checked.filtered_damage_raw, 7_900);
    assert_eq!(
        world.manager.pending_actor_deferred_destroy_ids(),
        &[world.id]
    );
    let entity = world.manager.entity_mut(world.id).unwrap();
    assert_eq!(
        entity.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(111)
    );
    assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .all(|slot| entity.actor_task_state(slot).is_none()));
    assert!(!world
        .scheduler
        .intro2_gun_turret_completed_owner(&world.manager, world.id));
    let rings: Vec<_> = world
        .manager
        .iter_all()
        .filter(|e| e.entity_type == 60)
        .map(|e| e.id)
        .collect();
    assert_eq!(rings.len(), 1);
    assert!(world.scheduler.family_for(rings[0]).is_some());
    assert!(world.fx.particle_count() >= 17);
    let count = world.fx.particle_count();
    let mut expected = world.fx.fork_for_main_base_abort_transaction();
    let repeat = world.hit(false, 10_000, 112);
    assert!(
        matches!(repeat, Intro2GunTurretImpactOutcome::Applied(ref checked)
        if checked.filtered_damage_raw == 7_900),
        "{repeat:?}"
    );
    assert_eq!(
        world
            .manager
            .entity_mut(world.id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(112)
    );
    assert_eq!(world.fx.particle_count(), count);
    assert_eq!(
        world.fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn finished_type92_and_type102_accept_both_hit_wrappers_without_repeating_terminal_work() {
    for spawn in [52, 53, 54] {
        let Some(mut world) = World::new(spawn) else {
            return;
        };
        for _ in 0..8 {
            world.fx.advance_frame_pacing(20_000);
        }
        let first = world.hit(false, 10_000, 111);
        assert!(
            matches!(first, Intro2GunTurretImpactOutcome::Applied(_)),
            "{first:?}"
        );
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &world.manager,
            world.id
        ));
        let entity = world.manager.entity_mut(world.id).unwrap();
        let terminal = entity.class49_death_runtime;
        let context = entity.current_behavior_context;
        let capability = entity.capability_flags;
        let runtime = entity.intro2_gun_turret_runtime;
        let motion = (
            entity.position_raw(),
            entity.velocity_raw(),
            entity.rotation_heading_pitch_roll_raw(),
        );
        let ring_ids: Vec<_> = world
            .manager
            .iter_all()
            .filter(|entity| entity.entity_type == 60)
            .map(|entity| entity.id)
            .collect();
        assert_eq!(ring_ids.len(), 1);
        world.fx.process_pending();
        world.fx.take_positional_sounds();
        let count = world.fx.particle_count();
        let mut expected = world.fx.fork_for_main_base_abort_transaction();
        for (infected, tick, expected_stamp) in
            [(false, 112, 112), (true, 113, 112), (false, 114, 114)]
        {
            let result = world.hit(infected, 10_000, tick);
            let Intro2GunTurretImpactOutcome::Applied(checked) = result else {
                panic!("spawn{spawn} infected{infected}: {result:?}");
            };
            assert_eq!(
                checked.filtered_damage_raw,
                if infected {
                    0
                } else if spawn == 52 {
                    8_200
                } else {
                    7_900
                }
            );
            assert!(checked.death_publication.is_none());
            let entity = world.manager.entity_mut(world.id).unwrap();
            assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
            assert_eq!(
                entity.collision.last_hit_presentation_tick_at_0x34,
                RetailRuntimeValue::Known(expected_stamp)
            );
            if tick >= 113 {
                assert_eq!(
                    entity.collision.state_flags_at_0x08.masked(0x2000),
                    RetailRuntimeValue::Known(0x2000)
                );
            }
            assert_eq!(entity.class49_death_runtime, terminal);
            assert_eq!(entity.current_behavior_context, context);
            assert_eq!(entity.intro2_gun_turret_runtime, runtime);
            assert_eq!(
                entity.capability_flags, capability,
                "null49+20 must not execute living C690"
            );
            assert_eq!(
                (
                    entity.position_raw(),
                    entity.velocity_raw(),
                    entity.rotation_heading_pitch_roll_raw()
                ),
                motion
            );
            assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
                .into_iter()
                .all(|slot| entity.actor_tasks.task_in_slot(slot).is_none()));
            assert!(crate::class49_death::finished_terminal_hit_authenticates(
                &world.manager,
                world.id
            ));
            assert_eq!(
                world.manager.pending_actor_deferred_destroy_ids(),
                &[world.id]
            );
            assert!(!world
                .scheduler
                .intro2_gun_turret_has_pending_prefix(world.id));
        }
        assert_eq!(
            world
                .manager
                .iter_all()
                .filter(|entity| entity.entity_type == 60)
                .map(|entity| entity.id)
                .collect::<Vec<_>>(),
            ring_ids
        );
        assert_eq!(world.fx.particle_count(), count);
        world.fx.process_pending();
        assert!(world.fx.take_positional_sounds().is_empty());
        assert_eq!(
            world.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn incomplete_and_foreign_terminal_receipts_cannot_admit_repeated_hits() {
    use crate::class49_death::{
        begin_class49_standard_death, claim_class49_terminal, finish_class49_terminal,
        finished_terminal_hit_authenticates,
    };
    // Exercise actual Issued, Claimed and a committed Finishing failure, as
    // well as a completed receipt copied onto another manager's allocation.
    for phase in 0..4 {
        for infected in [false, true] {
            let Some(mut world) = World::new(53) else {
                return;
            };
            if phase < 3 {
                let receipt = begin_class49_standard_death(
                    &mut world.manager,
                    world.id,
                    &world.session.cache,
                    &mut world.fx,
                    111,
                )
                .unwrap()
                .unwrap();
                if phase >= 1 {
                    assert!(claim_class49_terminal(&mut world.manager, &receipt));
                }
                if phase == 2 {
                    world
                        .manager
                        .entity_mut(world.id)
                        .unwrap()
                        .collision
                        .state_flags_at_0x08
                        .invalidate(0x8000_0000);
                    assert!(finish_class49_terminal(
                        &mut world.manager,
                        receipt,
                        &world.session.cache,
                        &mut world.fx
                    )
                    .is_err());
                    world
                        .manager
                        .entity_mut(world.id)
                        .unwrap()
                        .collision
                        .state_flags_at_0x08
                        .overwrite(0x8000_0000, 0);
                }
            } else {
                let Some(mut source) = World::new(53) else {
                    return;
                };
                assert!(matches!(
                    source.hit(false, 10_000, 111),
                    Intro2GunTurretImpactOutcome::Applied(_)
                ));
                let source = source.manager.entity_mut(source.id).unwrap();
                let entity = world.manager.entity_mut(world.id).unwrap();
                entity.class49_death_runtime = source.class49_death_runtime;
                entity.current_behavior_context = source.current_behavior_context;
                entity.collision = source.collision.clone();
                for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
                    entity.actor_tasks.clear_slot(slot);
                }
            }
            assert!(!finished_terminal_hit_authenticates(
                &world.manager,
                world.id
            ));
            let entity = world.manager.entity_mut(world.id).unwrap();
            let collision = entity.collision.clone();
            let terminal = entity.class49_death_runtime;
            let context = entity.current_behavior_context;
            let slots = ActorTaskSlot::IN_RETAIL_TICK_ORDER
                .map(|slot| entity.actor_tasks.task_in_slot(slot));
            let count = world.fx.particle_count();
            let mut expected = world.fx.fork_for_main_base_abort_transaction();
            assert_eq!(
                world.hit(infected, 10_000, 112),
                Intro2GunTurretImpactOutcome::Blocked {
                    reason: Intro2GunTurretImpactBlock::Runtime("pending actor prefix"),
                    committed_prefix: false,
                }
            );
            let entity = world.manager.entity_mut(world.id).unwrap();
            assert_eq!(entity.collision, collision);
            assert_eq!(entity.class49_death_runtime, terminal);
            assert_eq!(entity.current_behavior_context, context);
            assert_eq!(
                ActorTaskSlot::IN_RETAIL_TICK_ORDER
                    .map(|slot| entity.actor_tasks.task_in_slot(slot)),
                slots
            );
            assert_eq!(world.fx.particle_count(), count);
            assert_eq!(
                world.fx.next_shared_retail_random_u16(),
                expected.next_shared_retail_random_u16()
            );
        }
    }
}
