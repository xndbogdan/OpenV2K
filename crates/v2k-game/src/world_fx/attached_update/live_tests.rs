use super::*;
use crate::{
    attached_particle_damage::{
        apply_attached_particle_damage, AttachedParticleDamageBlock, AttachedParticleDamageFrame,
        AttachedParticleDamageWorld,
    },
    entity::EntityManager,
    gameplay_notifications::GameplayNotifications,
    intro2_type47_live::world::native_intro2_fixture,
    resource_cache::ResourceCache,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
};

#[derive(Clone, Copy)]
enum AfterDamage {
    None,
    Recycle(usize),
    Free(usize),
    Unlink,
    Rebind,
}

struct LiveHost<'a> {
    cache: &'a mut ResourceCache,
    entities: &'a mut EntityManager,
    scheduler: &'a mut SpecializedActorTaskScheduler,
    static_damage: &'a mut StaticDamageScheduler,
    notifications: &'a mut GameplayNotifications,
    after: AfterDamage,
    damage_calls: usize,
    cached_reads: usize,
}
impl ParticleTraversalHost for LiveHost<'_> {
    fn context(&self) -> ParticleTraversalContext<'_> {
        ParticleTraversalContext {
            environment: ParticleEnvironment::Dry,
            callbacks: Default::default(),
            particle_emitter: Default::default(),
        }
    }
    fn entity_impact(&mut self, _: &mut WorldFx, _: ParticleEntityImpact) {
        panic!("no contact");
    }
    fn static_impact(
        &mut self,
        _: &mut WorldFx,
        _: ParticleStaticImpact,
    ) -> Option<ParticleTerrainMutation> {
        panic!("no contact");
    }
    fn begin_attached_update(
        &mut self,
        _: &mut WorldFx,
        request: AttachedParticleOwnerRequest,
    ) -> AttachedParticleOwnerLookup {
        begin_attached_particle_owner_update(self.entities, request)
    }
    fn attached_damage(
        &mut self,
        fx: &mut WorldFx,
        request: AttachedParticleDamageRequest,
    ) -> Result<i32, AttachedParticleDamageBlock> {
        self.damage_calls += 1;
        let result = apply_attached_particle_damage(
            AttachedParticleDamageFrame {
                resources: self.cache,
                entities: self.entities,
                world_fx: fx,
                scheduler: self.scheduler,
                static_damage: self.static_damage,
                notifications: self.notifications,
                retail_tick: 21,
                world: AttachedParticleDamageWorld::Cinematic,
            },
            request,
        );
        if result.is_ok() {
            match self.after {
                AfterDamage::None => {}
                AfterDamage::Recycle(parent) => {
                    // Actual400A60 priority6 replacement. All other records
                    // have age0, leaving this aged physical parent as victim.
                    let mut replacement = fx.particles.slots[parent].unwrap();
                    replacement.source_class = 91;
                    replacement.age_ticks = 0.0;
                    replacement.lifetime_ticks = 60;
                    replacement.position = raw_position_to_world([1100, 2200, 3300]);
                    replacement.velocity = raw_velocity_to_world([71, 83, 97]);
                    replacement.owner_id = None;
                    replacement.attached_owner_handle = None;
                    replacement.attached_offset_raw = [0; 3];
                    replacement.source_entity_type_at_birth = Some(0);
                    replacement.suppresses_impact_damage = false;
                    replacement.pending_destruction = false;
                    // Descriptor91 is model-backed; this isolated record test
                    // proves allocator/callback custody, not its unowned draw.
                    let slot = fx.particles.allocate(replacement).unwrap();
                    assert_eq!(slot, parent);
                    // A controlled reentrant cleanup opens one lower callback
                    // allocation opportunity without freeing the current ESI.
                    let other = fx
                        .particles
                        .slots
                        .iter()
                        .enumerate()
                        .find(|(slot, particle)| *slot != parent && particle.is_some())
                        .unwrap()
                        .0;
                    fx.free_combat_particle(
                        other,
                        ParticleBirthContext {
                            environment: ParticleEnvironment::Dry,
                            retail_tick: 21,
                        },
                    );
                }
                AfterDamage::Free(slot) => {
                    fx.free_combat_particle(
                        slot,
                        ParticleBirthContext {
                            environment: ParticleEnvironment::Dry,
                            retail_tick: 21,
                        },
                    );
                }
                AfterDamage::Unlink => {
                    self.entities
                        .entity_mut(request.target_handle)
                        .unwrap()
                        .mark_actor_deferred_destroy_pending();
                    self.entities
                        .queue_actor_deferred_destroy(request.target_handle);
                    self.entities.cleanup_pending_actor_deferred_destroys();
                }
                AfterDamage::Rebind => {
                    *self.entities = native_intro2_fixture().unwrap().1;
                }
            }
        }
        result
    }
    fn attached_cascade_owner(
        &mut self,
        _: &mut WorldFx,
        request: crate::world_fx::AttachedParticleCascadeOwnerRequest,
    ) -> Result<AttachedParticleCascadeOwner, AttachedParticleUpdateBlock> {
        self.cached_reads += 1;
        sample_attached_particle_cached_owner(self.entities, request)
    }
}

fn parent(fx: &mut WorldFx, id: u32, class: u8, age: u8) -> usize {
    let slot = fx
        .emit_attached_static_particle_raw(AttachedStaticEmission {
            target_handle: id,
            position_world: [0.0, 20.0, 0.0],
            offset_raw: [0; 3],
            particle_class: class,
        })
        .unwrap()
        .slot;
    fx.particles.slots[slot].as_mut().unwrap().age_ticks = f32::from(age);
    slot
}
fn task_snapshot(
    entity: &crate::entity::Entity,
) -> Vec<(Option<crate::actor_task_owner::ActorTaskId>, String)> {
    crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .map(|slot| {
            let id = entity.actor_tasks.task_in_slot(slot);
            (
                id,
                format!(
                    "{:?}",
                    id.map(|id| (
                        entity.actor_tasks.task_state(id),
                        entity.actor_tasks.wrapper_flags(id)
                    ))
                ),
            )
        })
        .collect()
}
fn odd_after(fx: &mut WorldFx, draws: usize) {
    for seed in 1..10000 {
        let mut rng = seed;
        for _ in 0..draws {
            retail_random_u16(&mut rng);
        }
        if retail_random_u16(&mut rng) & 1 != 0 {
            fx.rng_state = seed;
            return;
        }
    }
    panic!("seed");
}
fn visit(fx: &mut WorldFx, host: &mut LiveHost<'_>, slot: usize) -> ParticleUpdateOutcome {
    let mut result = ParticleUpdateOutcome::default();
    fx.update_attached_particle_slot(
        slot,
        ParticleTraversalTiming {
            retail_tick: 21,
            elapsed_micros: 0,
        },
        16,
        host,
        &mut result,
    );
    result
}

#[v2k_test_support::retail_test]
fn enabled_attached_all_three_packets_use_direct_scaled_native_damage_without_hit_prefix() {
    for class in [83, 84, 86] {
        let Some((mut session, mut entities, _)) = native_intro2_fixture() else {
            return;
        };
        let id = entities.iter_all().find(|e| e.entity_type == 9).unwrap().id;
        let entity = entities.entity_mut(id).unwrap();
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        entity.collision.health_raw = RetailRuntimeValue::Known(50_000);
        entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
        entity.set_velocity_raw([-701, -203, 901]);
        let before_profile = entity.collision.damage_profile;
        let before_velocity = entity.velocity_raw();
        let before_context = entity.current_behavior_context;
        let before_tasks = task_snapshot(entity);
        let before_stamp = entity.collision.last_hit_presentation_tick_at_0x34;
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler.adopt_intro2_type9(&mut entities) > 0);
        let mut fx = WorldFx::new();
        let slot = parent(&mut fx, id, class, 32);
        odd_after(&mut fx, 0);
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut host = LiveHost {
            cache: &mut session.cache,
            entities: &mut entities,
            scheduler: &mut scheduler,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            after: AfterDamage::None,
            damage_calls: 0,
            cached_reads: 0,
        };
        let result = visit(&mut fx, &mut host, slot);
        assert!(result.blocked_attached_updates.is_empty(), "{result:?}");
        let [observed] = result.attached_updates.as_slice() else {
            panic!("{result:?}")
        };
        let delivery = observed.damage_request.delivery().unwrap();
        let RetailRuntimeValue::Known(profile) = before_profile else {
            panic!("profile")
        };
        let expected = delivery
            .packet
            .filtered_raw_with_ratio(Some(&profile), 16, 255);
        assert!(expected > 0);
        let after = host.entities.entity_mut(id).unwrap();
        assert_eq!(observed.filtered_damage_raw, expected);
        assert_eq!(
            after.collision.health_raw,
            RetailRuntimeValue::Known(50_000 - expected)
        );
        assert_eq!(after.velocity_raw(), before_velocity);
        assert_eq!(after.current_behavior_context, before_context);
        assert_eq!(task_snapshot(after), before_tasks);
        assert_eq!(
            after.collision.last_hit_presentation_tick_at_0x34,
            before_stamp
        );
        assert_eq!(delivery.source_entity_type_raw, (-5_i32) as u32);
        assert_eq!(
            after.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(56)
        );
        assert_eq!((host.damage_calls, host.cached_reads), (1, 1));
    }
}

#[v2k_test_support::retail_test]
fn enabled_attached_filtered_zero_keeps_buffer_and_graph_but_still_clamps_age() {
    for class in [83, 84, 86] {
        let Some((mut session, mut entities, _)) = native_intro2_fixture() else {
            return;
        };
        let id = entities
            .iter_all()
            .find(|e| e.entity_type == 26)
            .unwrap()
            .id;
        let entity = entities.entity_mut(id).unwrap();
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(100_000);
        let health = entity.collision.health_raw;
        let tasks = task_snapshot(entity);
        // Positive buffer chooses lifetime-age. At the authored lifetime255,
        // the exact ratio is zero for every descriptor's unchanged packet.
        //15040 cannot enter a mutating callback and needs no scheduler lease.
        let mut scheduler = SpecializedActorTaskScheduler::new();
        let mut fx = WorldFx::new();
        let slot = parent(&mut fx, id, class, 255);
        odd_after(&mut fx, 0);
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut host = LiveHost {
            cache: &mut session.cache,
            entities: &mut entities,
            scheduler: &mut scheduler,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            after: AfterDamage::None,
            damage_calls: 0,
            cached_reads: 0,
        };
        let result = visit(&mut fx, &mut host, slot);
        assert!(result.blocked_attached_updates.is_empty(), "{result:?}");
        let [observed] = result.attached_updates.as_slice() else {
            panic!("{result:?}")
        };
        assert_eq!(observed.filtered_damage_raw, 0);
        assert_eq!(observed.damage_request.ratio_numerator, 0);
        assert_eq!(observed.age_byte_after_damage, 255);
        let entity = host.entities.entity_mut(id).unwrap();
        assert_eq!(
            entity.collision.pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(100_000)
        );
        assert_eq!(entity.collision.health_raw, health);
        assert_eq!(task_snapshot(entity), tasks);
        assert_eq!(host.damage_calls, 1);
    }
}

#[v2k_test_support::retail_test]
fn native_class12_death_cascade_reads_postcallback_cached_velocity_and_retains_live_allocation() {
    let Some((mut session, mut entities, _)) = native_intro2_fixture() else {
        return;
    };
    let id = entities
        .iter_all()
        .find(|e| e.entity_type == 26)
        .unwrap()
        .id;
    let entity = entities.entity_mut(id).unwrap();
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.collision.health_raw = RetailRuntimeValue::Known(1);
    entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
    entity.set_velocity_raw([-701, -203, 901]);
    let rotated = match entity
        .collision
        .state_flags_at_0x08
        .masked(ATTACHED_FOLLOW_ROTATED_STATE_BIT)
    {
        RetailRuntimeValue::Known(flags) => flags != 0,
        _ => panic!("native follow state"),
    };
    let lease = entities
        .main_base_abort_actor_observation(id)
        .unwrap()
        .lease;
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert!(scheduler.adopt_intro2_type26(&entities) > 0);
    let mut fx = WorldFx::new();
    let slot = parent(&mut fx, id, 86, 32);
    // Rotated follow consumes its chance draw even at elapsed0, before direct
    // damage. Type26's authored Sub-A/null Sub-G C620 then consumes one draw.
    odd_after(&mut fx, usize::from(rotated) + 1);
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut host = LiveHost {
        cache: &mut session.cache,
        entities: &mut entities,
        scheduler: &mut scheduler,
        static_damage: &mut static_damage,
        notifications: &mut notifications,
        after: AfterDamage::None,
        damage_calls: 0,
        cached_reads: 0,
    };
    let result = visit(&mut fx, &mut host, slot);
    assert!(result.blocked_attached_updates.is_empty(), "{result:?}");
    assert_eq!(
        host.entities
            .main_base_abort_actor_observation(id)
            .unwrap()
            .lease,
        lease
    );
    assert_eq!(
        host.entities.entity_mut(id).unwrap().velocity_raw(),
        [-701, 500, 901]
    );
    assert_eq!(host.cached_reads, 1);
    let child = fx
        .particles
        .slots
        .iter()
        .flatten()
        .find(|p| p.source_class == 39)
        .unwrap();
    assert_eq!(child.velocity, raw_velocity_to_world([-350, 0, 450]));
    assert_eq!(child.owner_id, Some(id));
    assert_eq!(
        host.entities.entity_mut(id).unwrap().collision.health_raw,
        RetailRuntimeValue::Known(0)
    );
}

#[v2k_test_support::retail_test]
fn enabled_attached_does_not_freeze_unlinked_or_rebound_cached_entity_record() {
    for (after, reason) in [
        (
            AfterDamage::Unlink,
            AttachedParticleUpdateBlock::CachedOwnerAllocationMissing,
        ),
        (
            AfterDamage::Rebind,
            AttachedParticleUpdateBlock::CachedOwnerAllocationRebound,
        ),
    ] {
        let Some((mut session, mut entities, _)) = native_intro2_fixture() else {
            return;
        };
        let id = entities.iter_all().find(|e| e.entity_type == 9).unwrap().id;
        let entity = entities.entity_mut(id).unwrap();
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        entity.collision.health_raw = RetailRuntimeValue::Known(50_000);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.adopt_intro2_type9(&mut entities);
        let mut fx = WorldFx::new();
        let slot = parent(&mut fx, id, 84, 32);
        odd_after(&mut fx, 0);
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut host = LiveHost {
            cache: &mut session.cache,
            entities: &mut entities,
            scheduler: &mut scheduler,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            after,
            damage_calls: 0,
            cached_reads: 0,
        };
        let result = visit(&mut fx, &mut host, slot);
        assert_eq!(
            result.attached_updates.len(),
            1,
            "damage completed before suffix block"
        );
        let [diagnostic] = result.blocked_attached_updates.as_slice() else {
            panic!("{result:?}")
        };
        assert_eq!(diagnostic.reason, reason);
        assert!(diagnostic.committed_prefix);
        assert!(fx.particles.slots[slot].is_none());
        assert_eq!((host.damage_calls, host.cached_reads), (1, 1));
    }
}

#[v2k_test_support::retail_test]
fn enabled_attached_suffix_rereads_real_recycled_slot_and_never_restores_freed_memory() {
    for recycle in [true, false] {
        let Some((mut session, mut entities, _)) = native_intro2_fixture() else {
            return;
        };
        let id = entities.iter_all().find(|e| e.entity_type == 9).unwrap().id;
        let entity = entities.entity_mut(id).unwrap();
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        entity.collision.health_raw = RetailRuntimeValue::Known(50_000);
        entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(100_000);
        entity.set_velocity_raw([-701, -203, 901]);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.adopt_intro2_type9(&mut entities);
        let mut fx = WorldFx::new();
        let slot = parent(&mut fx, id, 84, 32);
        if recycle {
            for _ in 1..MAX_WORLD_PARTICLES {
                parent(&mut fx, id, 84, 0);
            }
        }
        odd_after(&mut fx, 0);
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut host = LiveHost {
            cache: &mut session.cache,
            entities: &mut entities,
            scheduler: &mut scheduler,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            after: if recycle {
                AfterDamage::Recycle(slot)
            } else {
                AfterDamage::Free(slot)
            },
            damage_calls: 0,
            cached_reads: 0,
        };
        let result = visit(&mut fx, &mut host, slot);
        if recycle {
            assert!(result.blocked_attached_updates.is_empty(), "{result:?}");
            let current = fx.particles.slots[slot].unwrap();
            assert_eq!(
                current.source_class, 91,
                "current class survives cached descriptor's age clamp"
            );
            assert_eq!(
                current.retail_age_byte(),
                255,
                "cached class84 lifetime, not class91 lifetime60"
            );
            assert_eq!(world_position_to_raw(current.position), [1100, 2200, 3300]);
            assert_eq!(current.velocity, raw_velocity_to_world([71, 83, 97]));
            assert_eq!(current.attached_owner_handle, None);
            assert_eq!(fx.particle_count(), MAX_WORLD_PARTICLES);
            let child = fx
                .particles
                .slots
                .iter()
                .flatten()
                .find(|p| p.source_class == 39)
                .unwrap();
            assert_eq!(
                child.owner_id, None,
                "current ESI+14, not original cached EDI"
            );
            assert_eq!(child.source_entity_type_at_birth, Some(0));
            assert_eq!(world_position_to_raw(child.position), [1100, 2200, 3300]);
            assert_eq!(child.velocity, raw_velocity_to_world([-350, -203, 450]));
        } else {
            let [diagnostic] = result.blocked_attached_updates.as_slice() else {
                panic!("{result:?}")
            };
            assert_eq!(
                diagnostic.reason,
                AttachedParticleUpdateBlock::PhysicalSlotFreedDuringDamage
            );
            assert!(diagnostic.committed_prefix);
            assert!(fx.particles.slots[slot].is_none());
            assert!(result.attached_updates.is_empty());
        }
        assert_eq!(host.damage_calls, 1);
    }
}
