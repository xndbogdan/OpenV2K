use super::*;
use crate::{
    common_mover::type9_attitude::Type9BodyBasis, intro2_type47_live::world::native_intro2_fixture,
};

fn spawn_id(manager: &crate::entity::EntityManager, spawn: usize) -> u32 {
    manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(spawn))
        .unwrap()
        .id
}
fn prepare(manager: &mut EntityManager, id: u32) {
    let entity = manager.entity_mut(id).unwrap();
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x0206_8000);
    entity.set_motion_raw([0, 10_000, 0], [100, 200, -300]);
    entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
}

struct ActiveTerminalCall(Intro2Type10TerminalReceipt);
impl crate::entity::DynamicRadialLiveCallbacks for ActiveTerminalCall {
    fn before_native_actor_mutation(&mut self, _: &EntityManager, _: u32) -> bool {
        panic!("the active terminal allocation retains its callback custody");
    }
    fn active_terminal_call(&self, manager: &EntityManager, id: u32) -> bool {
        id == self.0.entity_id && active_terminal_receipt(manager, &self.0)
    }
}

#[v2k_test_support::retail_test]
fn native_type10_death_uses_own_class11_constructor_preserving_component_cadence_and_velocity() {
    for spawn in [55, 56] {
        let Some((_session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = spawn_id(&manager, spawn);
        prepare(&mut manager, id);
        let entity = manager.entity_mut(id).unwrap();
        entity
            .intro2_type10_runtime
            .as_mut()
            .unwrap()
            .sub_e_runtime
            .cadence_raw = 12345;
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
        let before = entity.intro2_type10_runtime.unwrap();
        let old_slots =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot));
        let mut fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        let word = oracle.next_shared_retail_random_u16();
        let owner = publish_intro2_type10_standard_death(&mut manager, id, &mut fx)
            .unwrap()
            .unwrap();
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.intro2_type10_runtime, Some(before));
        assert_eq!(entity.velocity_raw(), [100, 200, -300]);
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(7)
        );
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(bits(entity, 0x1c000).unwrap(), 0x1c000);
        assert_ne!(Some(owner.visit.task_id), old_slots[0]);
        assert!(entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Secondary)
            .is_none());
        assert!(entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary)
            .is_none());
        let RetailRuntimeValue::Known(Some(g)) = entity.sub_g_06070_runtime else {
            panic!()
        };
        assert_eq!(g.mode_at_0x3f(), RetailRuntimeValue::Known(1));
        assert_eq!(
            g.randomized_target_raw_at_0x38(),
            RetailRuntimeValue::Known(700 + i32::from(word >> 8))
        );
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
        assert_eq!(fx.pending_event_count(), 0);
        assert_eq!(
            publish_intro2_type10_standard_death(&mut manager, id, &mut fx).unwrap(),
            None
        );
    }
}

#[test]
fn native_type10_tumble_signed_age_and_no_lifetime_boundary() {
    let mut task = Intro2Type10TumbleTask::new();
    assert_eq!(task.advance(999_000), 3);
    assert_eq!(task.advance(1000), 2);
    assert_eq!(task.advance(1_000_000), 1);
    assert_eq!(task.advance(20_000_000), 1);
    assert_eq!(task.terminal, None);
    task.elapsed_ms = 0xffff_fc18; // signed -1000 ms
    assert_eq!(task.advance(0), 4);
}

#[v2k_test_support::retail_test]
fn native_type10_tumble_direct_components_preserve_d_and_use_old_matrix_then_outer_rebuild() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = spawn_id(&manager, 55);
    prepare(&mut manager, id);
    let mut fx = WorldFx::new();
    let owner = publish_intro2_type10_standard_death(&mut manager, id, &mut fx)
        .unwrap()
        .unwrap();
    let entity = manager.entity_mut(id).unwrap();
    entity.set_rotation_heading_pitch_roll_raw([0x1234, -0x345, 0x678]);
    let old_basis = Type9BodyBasis::from_angle_words(-0x2200, 0x1100, 0x3300);
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(old_basis);
    let runtime = entity.intro2_type10_runtime.as_mut().unwrap();
    runtime.sub_k_smoothed_raw = 77;
    runtime.sub_l_target_raw = [500, 10_500, 800];
    let before = *runtime;
    let expected_l = crate::gkl_common_mover::component_outputs::advance_sub_l(
        before.sub_l_output_raw,
        before.sub_l_exact_raw,
        [0, 10_000, 0],
        before.sub_l_target_raw,
        old_basis,
        SUB_L,
    );
    let tick = tick_intro2_type10_tumble(
        &mut manager,
        owner,
        Intro2Type10Frame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 1,
        },
    );
    assert!(
        matches!(
            tick.outcome,
            Intro2Type10TumbleOutcome::Advanced { detailed: true, .. }
        ),
        "{:?}",
        tick.outcome
    );
    let entity = manager.entity_mut(id).unwrap();
    let after = entity.intro2_type10_runtime.unwrap();
    assert_eq!(after.sub_d_runtime, before.sub_d_runtime);
    assert_eq!(after.sub_d_frame_owner, before.sub_d_frame_owner);
    assert_eq!(after.sub_k_smoothed_raw, 77);
    assert_eq!(after.sub_l_target_raw, before.sub_l_target_raw);
    assert_eq!(after.sub_l_exact_raw, before.sub_l_exact_raw);
    assert_eq!(after.sub_l_output_raw, expected_l);
    let expected_angles = [
        0x1234i16.wrapping_add(3 * (20_000 >> 8)),
        (-0x345i16).wrapping_add(3 * ((20_000 >> 7) + (20_000 >> 8))),
        0x678i16.wrapping_add(3 * (20_000 >> 7)),
    ];
    assert_eq!(entity.rotation_heading_pitch_roll_raw(), expected_angles);
    assert_eq!(
        entity.physical_body_basis_q31,
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
            expected_angles[0],
            expected_angles[1],
            expected_angles[2]
        ))
    );
    assert_ne!(entity.position_raw(), [0, 10_000, 0]);
    assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
}

#[v2k_test_support::retail_test]
fn native_type10_tumble_coarse_age_only_and_late_prefix_never_replays() {
    for detailed in [false, true] {
        let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = spawn_id(&manager, 56);
        prepare(&mut manager, id);
        let mut fx = WorldFx::new();
        let owner = publish_intro2_type10_standard_death(&mut manager, id, &mut fx)
            .unwrap()
            .unwrap();
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.state_flags_at_0x08.overwrite(
            SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            if detailed {
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            } else {
                0
            },
        );
        entity.collision.callback_scheduler_accumulator_us_at_0x6c =
            RetailRuntimeValue::Known(125001);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(250000);
        let angles = entity.rotation_heading_pitch_roll_raw();
        if detailed {
            entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
        }
        let before = entity.intro2_type10_runtime.unwrap();
        let tick = tick_intro2_type10_tumble(
            &mut manager,
            owner,
            Intro2Type10Frame {
                resources: &mut session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                retail_tick: 1,
            },
        );
        if detailed {
            assert!(
                matches!(
                    tick.outcome,
                    Intro2Type10TumbleOutcome::Blocked {
                        reason: Intro2Type10DeathBlock::Runtime("Sub-L retained basis"),
                        prefix_committed: true,
                        ..
                    }
                ),
                "{:?}",
                tick.outcome
            );
            let entity = manager.entity_mut(id).unwrap();
            let angle_after = entity.rotation_heading_pitch_roll_raw();
            assert_ne!(angle_after, angles);
            let Some(ActorTaskRuntime::TumbleOutOfSky(task)) =
                entity.actor_tasks.task_state(owner.visit.task_id)
            else {
                panic!()
            };
            let age = task.elapsed_ms();
            assert!(
                !entity
                    .actor_tasks
                    .wrapper_flags(owner.visit.task_id)
                    .unwrap()
                    .in_callback
            );
            let next = tick_intro2_type10_tumble(
                &mut manager,
                tick.retained_owner.unwrap(),
                Intro2Type10Frame {
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    elapsed_micros: 20_000,
                    retail_tick: 2,
                },
            );
            assert!(matches!(
                next.outcome,
                Intro2Type10TumbleOutcome::Pending { .. }
            ));
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(entity.rotation_heading_pitch_roll_raw(), angle_after);
            let Some(ActorTaskRuntime::TumbleOutOfSky(task)) =
                entity.actor_tasks.task_state(owner.visit.task_id)
            else {
                panic!()
            };
            assert_eq!(task.elapsed_ms(), age);
        } else {
            assert!(
                matches!(
                    tick.outcome,
                    Intro2Type10TumbleOutcome::Advanced {
                        detailed: false,
                        ..
                    }
                ),
                "{:?}",
                tick.outcome
            );
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(entity.rotation_heading_pitch_roll_raw(), angles);
            assert_eq!(entity.intro2_type10_runtime, Some(before));
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type10_terminal_contact_is_one_use_class37_then_radial_then_destroy() {
    for contact in [
        Intro2Type10TumbleContact::Terrain,
        Intro2Type10TumbleContact::Static,
        Intro2Type10TumbleContact::Water,
    ] {
        let Some((session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = spawn_id(&manager, 55);
        prepare(&mut manager, id);
        let mut fx = WorldFx::new();
        for _ in 0..8 {
            fx.advance_frame_pacing(20_000);
        }
        let _ = publish_intro2_type10_standard_death(&mut manager, id, &mut fx).unwrap();
        let receipt =
            begin_intro2_type10_tumble_contact(&mut manager, id, &session.cache, &mut fx, contact)
                .unwrap()
                .unwrap();
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
                trailing_raw: [10, id as i32],
            }
        );
        let count = fx.particle_count();
        assert!(count > 0);
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
        assert!(particles.iter().any(|particle| particle.source_class == 37));
        assert!(particles.iter().any(|particle| particle.source_class == 18));
        assert!(particles
            .iter()
            .all(|particle| matches!(particle.source_class, 37 | 18)
                && particle.owner_id == Some(id)
                && particle.source_entity_type_at_birth == Some(10)));
        assert!(!finish_intro2_type10_terminal(&mut manager, receipt));
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
        assert_eq!(
            begin_intro2_type10_tumble_contact(&mut manager, id, &session.cache, &mut fx, contact)
                .unwrap(),
            None
        );
        assert_eq!(fx.particle_count(), count);
        assert!(claim_intro2_type10_terminal(&mut manager, &receipt));
        assert!(!claim_intro2_type10_terminal(&mut manager, &receipt));
        let other_ids = manager
            .iter_all()
            .filter(|e| e.id != id)
            .map(|e| e.id)
            .collect::<Vec<_>>();
        for other_id in other_ids {
            manager
                .entity_mut(other_id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(
                    crate::entity_collision_state::CHECKED_DAMAGE_ENABLED_STATE_BIT,
                    0,
                );
        }
        let mut notifications = crate::gameplay_notifications::GameplayNotifications::new();
        // Even the exact same origin and packet cannot replay a parked
        // terminal outside its currently executing callback stack.
        let blocked =
            manager.apply_dynamic_radial_damage_live(crate::entity::DynamicRadialLiveRequest {
                origin_raw: receipt.position_raw,
                template: receipt.radial_damage,
                world_fx: &mut fx,
                retail_tick: 1,
                notifications: &mut notifications,
                callbacks: &mut |_: &crate::entity::EntityManager, _: u32| {
                    panic!("pending terminal cannot transfer")
                },
            });
        assert!(matches!(
            blocked.blocked,
            Some(crate::entity::DynamicRadialLiveBlock {
                phase: crate::entity::DynamicRadialLivePhase::MutationCustody,
                target_prefix_committed: false,
                ..
            })
        ));
        let applied =
            manager.apply_dynamic_radial_damage_live(crate::entity::DynamicRadialLiveRequest {
                origin_raw: receipt.position_raw,
                template: receipt.radial_damage,
                world_fx: &mut fx,
                retail_tick: 1,
                notifications: &mut notifications,
                callbacks: &mut ActiveTerminalCall(receipt),
            });
        assert!(applied.completed(), "{applied:?}");
        assert_eq!(applied.completed_target_ids, [id]);
        assert_eq!(
            begin_intro2_type10_tumble_contact(&mut manager, id, &session.cache, &mut fx, contact)
                .unwrap(),
            None
        );
        assert!(finish_intro2_type10_terminal(&mut manager, receipt));
        assert!(!finish_intro2_type10_terminal(&mut manager, receipt));
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), [id]);
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| manager
                .entity_mut(id)
                .unwrap()
                .actor_tasks
                .task_in_slot(slot)
                .is_none()));
    }
}
