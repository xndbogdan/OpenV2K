use super::*;
use crate::{
    actor_task_owner::PreparedActorTask,
    intro2_gun_turret::native::tests::{fixture, generic, publish},
    type60_exploding_ring::{Type60ExplodingRingTaskState, Type60InitializerDisposition},
};

fn prepare(manager: &mut EntityManager, id: u32) {
    let entity = manager.entity_mut(id).unwrap();
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x0806_8000);
    entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
    entity.set_motion_raw([100, 10_000, -300], [70, -80, 90]);
}

fn full_rate_fx() -> WorldFx {
    let mut fx = WorldFx::new();
    for _ in 0..8 {
        fx.advance_frame_pacing(20_000);
    }
    fx
}

fn castle_fixture() -> (crate::session::GameSession, EntityManager, WorldFx) {
    let (session, mut manager, fx) = crate::intro2_gun_turret::authored_tests::fixture(15);
    // World construction retires two terrain exit helpers. Complete that
    // existing next-frame sweep before isolating later actor death custody.
    let pending: Vec<_> = manager
        .iter_all()
        .filter(|entity| {
            manager
                .pending_actor_deferred_destroy_ids()
                .contains(&entity.id)
        })
        .map(|entity| (entity.id, entity.entity_type, entity.authored_spawn_index))
        .collect();
    assert_eq!(
        pending.iter().map(|row| (row.1, row.2)).collect::<Vec<_>>(),
        [(111, None), (111, None)]
    );
    assert_eq!(
        manager.cleanup_pending_actor_deferred_destroys(),
        pending.iter().map(|row| row.0).collect::<Vec<_>>()
    );
    (session, manager, fx)
}

#[v2k_test_support::retail_test]
fn virus_flower_class1_runs_baf0_then_removes_without_a_ring() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 61);
    prepare(&mut manager, id);
    let tasks_before = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
        manager
            .entity_mut(id)
            .unwrap()
            .actor_tasks
            .task_in_slot(slot)
    });
    assert!(tasks_before.iter().any(Option::is_some));
    let mut fx = full_rate_fx();
    let receipt = begin_class49_standard_death(&mut manager, id, &session.cache, &mut fx, 1)
        .unwrap()
        .unwrap();
    assert_eq!(receipt.profile.policy(), NativeExplosionPolicy::Class1);
    assert_eq!(receipt.context.active_style().style_address(), 0x004c_7150);
    assert_eq!(
        fx.particle_count(),
        11,
        "ten class37 scatter particles plus the burst"
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot)),
        tasks_before,
        "BAF0 precedes A860"
    );
    assert!(claim_class49_terminal(&mut manager, &receipt));
    let result = finish_class49_terminal(&mut manager, receipt, &session.cache, &mut fx).unwrap();
    assert!(result.ring.is_none());
    assert!(result.ring_owner.is_none());
    assert!(!manager.iter_all().any(|entity| entity.entity_type == 60));
    assert_eq!(manager.pending_actor_deferred_destroy_ids(), &[id]);
    let entity = manager.entity_mut(id).unwrap();
    assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .all(|slot| entity.actor_task_state(slot).is_none()));
    assert!(finished_terminal_hit_authenticates(&manager, id));
    let count = fx.particle_count();
    assert_eq!(
        begin_class49_standard_death(&mut manager, id, &session.cache, &mut fx, 2).unwrap(),
        None
    );
    assert_eq!(fx.particle_count(), count);
}

#[v2k_test_support::retail_test]
fn castle_flowers_and_power_ups_keep_their_distinct_terminal_suffixes() {
    let (session, mut manager, _) = castle_fixture();
    let sources: Vec<_> = manager
        .iter_all()
        .filter(|entity| matches!(entity.entity_type, 61 | 115))
        .map(|entity| {
            (
                entity.id,
                entity.entity_type,
                entity.authored_spawn_index.unwrap(),
            )
        })
        .collect();
    assert_eq!(
        sources.iter().map(|row| row.2).collect::<Vec<_>>(),
        [2, 4, 51, 52]
    );
    let mut fx = full_rate_fx();
    for (id, entity_type, _) in sources {
        prepare(&mut manager, id);
        let old_sound = manager
            .entity_mut(id)
            .unwrap()
            .collision
            .constructor_sound_attachment_id_at_0x8c;
        assert_eq!(
            old_sound,
            RetailRuntimeValue::Known((entity_type == 61).then_some(44))
        );
        let particles_before = fx.particle_count();
        let receipt = begin_class49_standard_death(&mut manager, id, &session.cache, &mut fx, 1)
            .unwrap()
            .unwrap();
        let class1 = entity_type == 115;
        assert_eq!(
            receipt.profile.policy(),
            if class1 {
                NativeExplosionPolicy::Class1
            } else {
                NativeExplosionPolicy::Class49
            }
        );
        assert_eq!(
            fx.particle_count() - particles_before,
            if class1 { 11 } else { 17 }
        );
        assert_eq!(
            manager
                .entity_mut(id)
                .unwrap()
                .collision
                .constructor_sound_attachment_id_at_0x8c,
            RetailRuntimeValue::Known(None)
        );
        assert!(claim_class49_terminal(&mut manager, &receipt));
        let completion =
            finish_class49_terminal(&mut manager, receipt, &session.cache, &mut fx).unwrap();
        assert_eq!(completion.ring.is_some(), !class1);
        assert_eq!(completion.ring_owner.is_some(), !class1);
        assert!(finished_terminal_hit_authenticates(&manager, id));
    }
    assert_eq!(manager.pending_actor_deferred_destroy_ids().len(), 4);
    assert_eq!(
        manager
            .iter_all()
            .filter(|entity| entity.entity_type == 60)
            .count(),
        2
    );
}

#[v2k_test_support::retail_test]
fn native_power_up_pending_pickup_keeps_its_existing_deferred_owner_after_class49() {
    let (session, mut manager, _) = castle_fixture();
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(2))
        .unwrap()
        .id;
    assert!(crate::native_type61::allocation_authenticates(&manager, id));
    prepare(&mut manager, id);
    manager.queue_power_up_destroys(&[id]);
    let mut fx = full_rate_fx();
    let receipt = begin_class49_standard_death(&mut manager, id, &session.cache, &mut fx, 1)
        .unwrap()
        .unwrap();
    assert_eq!(
        receipt.deferred_destroy_entry,
        MainBaseDeferredDestroyEntry::AlreadyPending(
            MainBaseExternalDeferredDestroyOwner::Type61DeferredQueue
        )
    );
    assert!(claim_class49_terminal(&mut manager, &receipt));
    let completion =
        finish_class49_terminal(&mut manager, receipt, &session.cache, &mut fx).unwrap();
    assert!(completion.ring.is_some());
    assert_eq!(manager.pending_power_up_destroy_ids(), &[id]);
    assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
    assert!(finished_terminal_hit_authenticates(&manager, id));
    assert_eq!(
        begin_class49_standard_death(&mut manager, id, &session.cache, &mut fx, 2).unwrap(),
        None
    );
    assert_eq!(manager.cleanup_pending_power_up_destroys(), vec![id]);
    assert!(manager.cleanup_pending_power_up_destroys().is_empty());
    assert_eq!(
        manager
            .iter_all()
            .filter(|entity| entity.entity_type == 60)
            .count(),
        1
    );
}

#[v2k_test_support::retail_test]
fn native_type102_class49_emits_own_baf0_branch_before_slot_clear_and_ring() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for spawn in [53, 54] {
        let mut manager = generic(&session, &metadata);
        let id = publish(&mut manager, &session, &metadata, spawn);
        prepare(&mut manager, id);
        let entity = manager.entity_mut(id).unwrap();
        entity
            .intro2_gun_turret_runtime
            .as_mut()
            .unwrap()
            .sub_e_runtime
            .cadence_raw = 12345;
        let components = entity.intro2_gun_turret_runtime;
        let slots =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot));
        let mut fx = full_rate_fx();
        let mut oracle = full_rate_fx();
        // BAF0's direction table does not draw the process RNG; 440950 sound62 does.
        oracle.next_shared_retail_random_u16();
        let receipt = begin_class49_standard_death(&mut manager, id, &session.cache, &mut fx, 71)
            .unwrap()
            .unwrap();
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            state_bits(entity, DYING_STATE_BIT).unwrap(),
            DYING_STATE_BIT
        );
        assert_eq!(entity.intro2_gun_turret_runtime, components);
        assert_eq!(entity.velocity_raw(), [70, -80, 90]);
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot)),
            slots
        );
        assert!(
            matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(context))
            if context.active_style().style_address() == 0x004c71e0)
        );
        assert_eq!(
            receipt.radial_damage,
            RadialDamageTemplate {
                inner_radius_raw: 512,
                outer_radius_raw: 1024,
                impulse_raw: 2000,
                packet: DamagePacket {
                    channels: [1, 3],
                    amounts_raw: [4000, 4000]
                },
                trailing_raw: [102, id as i32],
            }
        );
        assert_eq!(fx.particle_count(), 17);
        // Authenticated Type102 authors no 10C10 header+90 cue. Only BAF0's
        // independent randomized sound62 is emitted, including after flush.
        assert_eq!(
            session
                .cache
                .global_entity_type(102)
                .unwrap()
                .death_sound_id(),
            None
        );
        fx.process_pending();
        let sounds = fx.take_positional_sounds();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].sound_id, 62);
        let particles = fx
            .prepare_presentation([640, 480], 0x3000, |_| {
                v2k_render::ParticleCenterProjection {
                    screen: [320, 240],
                    depth_raw: 1000,
                    clip: 0,
                }
            })
            .particles()
            .map(|prepared| prepared.particle)
            .collect::<Vec<_>>();
        for class in [94, 95] {
            assert_eq!(
                particles.iter().filter(|p| p.source_class == class).count(),
                8
            );
        }
        assert!(particles
            .iter()
            .all(|p| p.owner_id == Some(id) && p.source_entity_type_at_birth == Some(102)));
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
        assert_eq!(
            finish_class49_terminal(&mut manager, receipt, &session.cache, &mut fx),
            Err(Class49DeathBlock::TerminalReceipt)
        );
        assert_eq!(
            begin_class49_standard_death(&mut manager, id, &session.cache, &mut fx, 72).unwrap(),
            None
        );
        assert_eq!(fx.particle_count(), 17);
        fx.process_pending();
        assert!(fx.take_positional_sounds().is_empty());
    }
}

#[v2k_test_support::retail_test]
fn native_class49_remote_or_already_dying_skips_all_death_presentation() {
    let (session, metadata) = fixture().expect("retail corpus required");
    for bypass in [REMOTE_OWNED_STATE_BIT, DYING_STATE_BIT] {
        let mut manager = generic(&session, &metadata);
        let id = publish(&mut manager, &session, &metadata, 53);
        prepare(&mut manager, id);
        let source = manager.entity_mut(id).unwrap();
        source
            .collision
            .state_flags_at_0x08
            .overwrite(bypass, bypass);
        let health = source.collision.health_raw;
        let context = source.current_behavior_context;
        let slots =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| source.actor_tasks.task_in_slot(slot));
        let mut fx = full_rate_fx();
        let mut oracle = full_rate_fx();
        assert_eq!(
            begin_class49_standard_death(&mut manager, id, &session.cache, &mut fx, 1),
            Ok(None)
        );
        let source = manager.entity_mut(id).unwrap();
        assert_eq!(source.collision.health_raw, health);
        assert_eq!(source.current_behavior_context, context);
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| source.actor_tasks.task_in_slot(slot)),
            slots
        );
        assert!(source.class49_death_runtime.is_none());
        assert_eq!(fx.particle_count(), 0);
        fx.process_pending();
        assert!(fx.take_positional_sounds().is_empty());
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn class49_nonzero_header_death_cue_precedes_scatter_sound_without_rng() {
    use crate::{
        entity::{AuthoredPlayerArrival, AuthoredWorldConstruction, EntityConstructionResources},
        entity_collision_state::EntityTypeRuntimeMetadata,
        world_fx::PositionalSoundEvent,
    };

    let (mut session, _) = fixture().expect("retail corpus required");
    session.load_level_by_id(15, 1).unwrap();
    // Current native rover/turret records author no fixed cue. Exercise the
    // generic 10C10 prefix with Type83's proven nonzero cue on a native rover
    // fixture, editing only parsed test data, never the original OVL bytes.
    assert_eq!(
        session
            .cache
            .global_entity_type(49)
            .unwrap()
            .death_sound_id(),
        None
    );
    let cue = session
        .cache
        .global_entity_type(83)
        .unwrap()
        .death_sound_id()
        .unwrap();
    assert_eq!(cue, 62);
    let mut common = session.load_ovl_by_id(3, 1).unwrap();
    common.system_level = Some(3);
    let record = &mut common.collision.as_mut().unwrap().entries[49 - 2];
    assert_eq!(record.model_ids, [266; 4]);
    record.raw_header[0x90..0x92].copy_from_slice(&cue.to_le_bytes());
    session.cache.add_auxiliary(common);
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    assert_eq!(
        metadata[49].death_sound_id,
        RetailRuntimeValue::Known(Some(cue))
    );
    let mut construction_fx = WorldFx::new();
    let mut manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: 3,
            type_metadata: &metadata,
            player_arrival: Some(AuthoredPlayerArrival {
                position_raw: [8192, 0, 8192],
                heading_raw: 0,
            }),
            retail_tick: 1000,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|model| {
                    session.cache.global_model(model).map(|model| model.radius)
                }),
            },
        },
        &mut construction_fx,
    )
    .unwrap();
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 49)
        .unwrap()
        .id;
    prepare(&mut manager, id);
    let mut fx = full_rate_fx();
    let mut oracle = full_rate_fx();
    let expected_rate = 0x1_0000 + u32::from(oracle.next_shared_retail_random_u16() >> 3);
    assert!(
        begin_class49_standard_death(&mut manager, id, &session.cache, &mut fx, 1)
            .unwrap()
            .is_some()
    );
    assert_eq!(fx.particle_count(), 11);
    fx.process_pending();
    assert_eq!(
        fx.take_positional_sounds(),
        vec![
            PositionalSoundEvent::fixed(62, [100.0 / 256.0, 10_000.0 / 256.0, 65_236.0 / 256.0]),
            PositionalSoundEvent {
                sound_id: 62,
                position: [100.0 / 256.0, 10_000.0 / 256.0, 65_236.0 / 256.0],
                frequency_q16: expected_rate,
            },
        ]
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
    assert_eq!(
        begin_class49_standard_death(&mut manager, id, &session.cache, &mut fx, 2),
        Ok(None)
    );
    fx.process_pending();
    assert!(fx.take_positional_sounds().is_empty());
    assert_eq!(fx.particle_count(), 11);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type102_terminal_claim_is_one_use_and_finish_reads_post_radial_pose() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 53);
    prepare(&mut manager, id);
    let mut fx = full_rate_fx();
    let receipt = begin_class49_standard_death(&mut manager, id, &session.cache, &mut fx, 1)
        .unwrap()
        .unwrap();
    let mut forged = receipt;
    forged.radial_damage.impulse_raw += 1;
    assert!(!claim_class49_terminal(&mut manager, &forged));
    assert!(claim_class49_terminal(&mut manager, &receipt));
    assert!(!claim_class49_terminal(&mut manager, &receipt));
    assert!(active_terminal_receipt(&manager, &receipt));
    let source = manager.entity_mut(id).unwrap();
    source.set_position_raw([900, 7000, -800]);
    source.collision.health_raw = RetailRuntimeValue::Known(-19);
    let result = finish_class49_terminal(&mut manager, receipt, &session.cache, &mut fx).unwrap();
    let Some(Type60ConstructionOutcome::ActorLinked(ring)) = result.ring else {
        panic!("{result:?}")
    };
    let owner = result.ring_owner.unwrap();
    let ring_entity = manager.entity_mut(owner.entity_id()).unwrap();
    assert_eq!(ring_entity.entity_type, 60);
    assert_eq!(ring_entity.position_raw(), [900, 7000, -800]);
    assert_eq!(ring_entity.model_slots, [Some(243); 4]);
    assert_eq!(Some(owner.task_lease()), ring.primary_task_lease());
    let source = manager.entity_mut(id).unwrap();
    assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .all(|slot| source.actor_task_state(slot).is_none()));
    assert_eq!(source.collision.health_raw, RetailRuntimeValue::Known(-19));
    assert_eq!(manager.pending_actor_deferred_destroy_ids(), &[id]);
    assert_eq!(
        finish_class49_terminal(&mut manager, receipt, &session.cache, &mut fx),
        Err(Class49DeathBlock::TerminalReceipt)
    );
}

#[v2k_test_support::retail_test]
fn native_type102_terminal_rereads_remote_gate_after_radial_and_still_removes_source() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 54);
    prepare(&mut manager, id);
    let mut fx = full_rate_fx();
    let receipt = begin_class49_standard_death(&mut manager, id, &session.cache, &mut fx, 1)
        .unwrap()
        .unwrap();
    assert!(claim_class49_terminal(&mut manager, &receipt));
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(REMOTE_OWNED_STATE_BIT, REMOTE_OWNED_STATE_BIT);
    let mut oracle = fx.fork_for_main_base_abort_transaction();
    let result = finish_class49_terminal(&mut manager, receipt, &session.cache, &mut fx).unwrap();
    assert_eq!(result.ring, None);
    assert_eq!(result.ring_owner, None);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
    assert_eq!(manager.pending_actor_deferred_destroy_ids(), &[id]);
}

#[v2k_test_support::retail_test]
fn native_type102_baf0_uses_resolved_owner_and_dangling_relation_fallback() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for dangling in [false, true] {
        let mut manager = generic(&session, &metadata);
        let id = publish(&mut manager, &session, &metadata, 53);
        prepare(&mut manager, id);
        let other = manager.iter_all().find(|e| e.entity_type == 9).unwrap().id;
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .recent_relation_id_at_0x60 =
            RetailRuntimeValue::Known(Some(if dangling { u32::MAX } else { other }));
        let mut fx = full_rate_fx();
        let receipt = begin_class49_standard_death(&mut manager, id, &session.cache, &mut fx, 1)
            .unwrap()
            .unwrap();
        assert_eq!(
            receipt.radial_damage.trailing_raw,
            if dangling {
                [102, id as i32]
            } else {
                [9, other as i32]
            }
        );
    }
}

struct FailingAllocator(u8);
impl Type60Allocator for FailingAllocator {
    fn prepare_components(&mut self, _: Type60ConstructionRequest) -> bool {
        self.0 != 0
    }
    fn prepare_behavior_context(&mut self) -> bool {
        self.0 != 1
    }
    fn prepare_primary_task(
        &mut self,
        _: Type60ExplodingRingTaskState,
    ) -> Option<PreparedActorTask<Type60ExplodingRingTaskState>> {
        None
    }
}

#[v2k_test_support::retail_test]
fn native_type102_ring_allocation_failures_consume_source_return_and_preserve_rng_order() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for stage in 0..3 {
        let mut manager = generic(&session, &metadata);
        let id = publish(&mut manager, &session, &metadata, 54);
        prepare(&mut manager, id);
        let mut fx = full_rate_fx();
        let receipt = begin_class49_standard_death(&mut manager, id, &session.cache, &mut fx, 1)
            .unwrap()
            .unwrap();
        assert!(claim_class49_terminal(&mut manager, &receipt));
        let mut oracle = fx.fork_for_main_base_abort_transaction();
        let selector = (stage != 0).then(|| oracle.next_shared_retail_random_u16());
        let result = finish_with_allocator(
            &mut manager,
            receipt,
            &session.cache,
            &mut fx,
            &mut FailingAllocator(stage),
        )
        .unwrap();
        match (stage, result.ring.unwrap()) {
            (0, Type60ConstructionOutcome::RejectedBeforeSelector) => (),
            (1, Type60ConstructionOutcome::RejectedAfterSelector { selector_rng_word }) => {
                assert_eq!(Some(selector_rng_word), selector)
            }
            (2, Type60ConstructionOutcome::ActorLinked(ring)) => {
                assert!(matches!(
                    ring.initializer(),
                    Type60InitializerDisposition::FallbackPublishedAfterPrimaryAllocationFailure { .. }
                ));
            }
            unexpected => panic!("{unexpected:?}"),
        }
        assert!(result.ring_owner.is_none());
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), &[id]);
    }
}

#[v2k_test_support::retail_test]
fn native_type102_terminal_late_unknown_state_keeps_clear_prefix_and_cannot_reenter() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 53);
    prepare(&mut manager, id);
    let mut fx = full_rate_fx();
    let receipt = begin_class49_standard_death(&mut manager, id, &session.cache, &mut fx, 1)
        .unwrap()
        .unwrap();
    assert!(claim_class49_terminal(&mut manager, &receipt));
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08 = crate::entity_collision_state::RetailStateWord::unknown();
    assert_eq!(
        finish_class49_terminal(&mut manager, receipt, &session.cache, &mut fx),
        Err(Class49DeathBlock::Runtime("state bits"))
    );
    let entity = manager.entity_mut(id).unwrap();
    assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .all(|slot| entity.actor_task_state(slot).is_none()));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x0806_c000);
    assert!(!claim_class49_terminal(&mut manager, &receipt));
    assert_eq!(
        finish_class49_terminal(&mut manager, receipt, &session.cache, &mut fx),
        Err(Class49DeathBlock::TerminalReceipt)
    );
    assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
}

#[v2k_test_support::retail_test]
fn type13_search_and_aimless_class1_prefix_retains_slots_until_radial_completion() {
    use crate::type13_initial_behavior::{
        plan_type13_initial_behavior, publish_type13_reselected_initial_behavior,
    };
    for selector in [0u32, 0x4000] {
        let Some((session, mut manager, _)) =
            crate::intro2_type47_live::world::native_intro2_fixture()
        else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(0))
            .unwrap()
            .id;
        let metadata = manager.type_runtime_metadata(13).unwrap().clone();
        assert_eq!(
            metadata.accepted_hit_presentation_sound_id,
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            metadata.generic_hit_sound_id,
            RetailRuntimeValue::Known(None)
        );
        let record = session.cache.global_entity_type(13).unwrap();
        assert_eq!(record.model_ids, [291; 4]);
        assert_eq!(
            metadata.death_sound_id,
            RetailRuntimeValue::Known(record.death_sound_id())
        );
        assert_eq!(
            metadata.constructor_sound_attachment_id,
            RetailRuntimeValue::Known(record.constructor_sound_attachment_id())
        );
        let planned = plan_type13_initial_behavior(&metadata, || selector).unwrap();
        publish_type13_reselected_initial_behavior(
            manager.entity_mut(id).unwrap(),
            &metadata,
            planned,
            || 0x1234,
        )
        .unwrap();
        let entity = manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(u32::MAX, 0x0747_8825);
        entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        entity.set_motion_raw([100, 10_000, -300], [70, -80, 90]);
        let context_before = entity.current_behavior_context;
        assert!(
            matches!(context_before, RetailRuntimeValue::Known(Some(context))
            if context.active_style().death_callback_policy() == crate::entity_behavior::DeathCallbackPolicy::None)
        );
        let tasks_before =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot));
        let sub_g_before = entity.sub_g_06070_runtime;
        let mut fx = full_rate_fx();
        let receipt = begin_class49_standard_death(&mut manager, id, &session.cache, &mut fx, 25)
            .unwrap()
            .unwrap();
        assert_eq!(receipt.profile, NativeExplosionSourceProfile::Intro2Type13);
        assert_eq!(receipt.profile.policy(), NativeExplosionPolicy::Class1);
        assert_eq!(receipt.context.active_style().style_address(), 0x4c7150);
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            entity.collision.constructor_sound_attachment_id_at_0x8c,
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(entity.sub_g_06070_runtime, sub_g_before);
        assert_eq!(entity.velocity_raw(), [70, -80, 90]);
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot)),
            tasks_before,
            "BAF0 must retain both real predecessor graphs until its radial traversal returns"
        );
        assert_eq!(
            fx.test_particles_in_virgin_birth_order()
                .iter()
                .filter(|particle| particle.source_class == 37
                    && particle.source_entity_type_at_birth == Some(13))
                .count(),
            10
        );
        let record = session.cache.global_entity_type(13).unwrap();
        assert_eq!(
            receipt.radial_damage.inner_radius_raw,
            i16::from_le_bytes(record.raw_header[0x50..0x52].try_into().unwrap())
        );
        assert_eq!(receipt.radial_damage.trailing_raw, [13, id as i32]);
        assert!(claim_class49_terminal(&mut manager, &receipt));
        assert!(!claim_class49_terminal(&mut manager, &receipt));
        let completed =
            finish_class49_terminal(&mut manager, receipt, &session.cache, &mut fx).unwrap();
        assert!(completed.ring.is_none());
        assert!(completed.ring_owner.is_none());
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), &[id]);
        assert!(finished_terminal_hit_authenticates(&manager, id));
        assert_eq!(
            finish_class49_terminal(&mut manager, receipt, &session.cache, &mut fx),
            Err(Class49DeathBlock::TerminalReceipt)
        );
        let count = fx.particle_count();
        let mut expected_rng = fx.fork_for_main_base_abort_transaction();
        assert_eq!(
            begin_class49_standard_death(&mut manager, id, &session.cache, &mut fx, 26),
            Ok(None)
        );
        assert_eq!(fx.particle_count(), count);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected_rng.next_shared_retail_random_u16()
        );
    }
}
