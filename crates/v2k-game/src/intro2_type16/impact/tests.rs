use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    damage::{DamagePacket, FUN_0043F780_DAMAGE_DELIVERY},
    entity_behavior::{
        audited_behavior_style, behavior_program, BehaviorChoiceListSource, BehaviorContextRuntime,
    },
    intro2_type47_live::world::native_intro2_fixture,
    live_actor_checked_damage::LiveActorDamagePhase,
    specialized_actor_task_production::SpecializedActorTaskFamily,
    world_fx::BallisticDamageRequest,
};

fn impact(id: u32, infected: bool, amount: i32) -> ParticleEntityImpact {
    ParticleEntityImpact {
        source_particle_class: if infected { 5 } else { 16 },
        impact_position_argument_va: if infected { 0x004D_CF48 } else { 0 },
        target_entity_id: id,
        position_world: [0.0; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(BallisticDamageRequest {
            packet: if infected {
                FUN_0043F780_DAMAGE_DELIVERY.packet
            } else {
                DamagePacket {
                    channels: [1, 0],
                    amounts_raw: [amount, 0],
                }
            },
            // F780 owns a static zero/zero delivery independently of these.
            source_entity_type_at_birth: Some(34),
            source_owner_id: Some(35),
        }),
    }
}

fn spawn_id(manager: &EntityManager, spawn: usize) -> u32 {
    manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(spawn))
        .unwrap()
        .id
}

fn prepare_target(manager: &mut EntityManager, spawn: usize) -> u32 {
    let id = spawn_id(manager, spawn);
    let entity = manager.entity_mut(id).unwrap();
    assert!(crate::intro2_type16::intro2_type16_allocation_authenticates(entity));
    // Controlled local reaction/admission flags; native tasks, metadata and
    // component receipts come from the actual authored birth.
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x0406_8000);
    entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
    entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
    id
}

fn set_style(entity: &mut Entity, class: u32, variant: u8) {
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(
        BehaviorContextRuntime::named_audited(
            behavior_program(class).unwrap(),
            u32::from(variant),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(0),
            *audited_behavior_style(class, variant).unwrap(),
        )
        .unwrap(),
    ));
}

fn full_rate_fx() -> WorldFx {
    let mut fx = WorldFx::new();
    for _ in 0..8 {
        fx.advance_frame_pacing(20_000);
    }
    fx
}

#[test]
fn native_type16_authenticates_primary_and_infected_style_asymmetries() {
    for (class, variant, infected, primary) in [
        (
            4,
            0,
            ImpactCallbackPolicy::ReselectBehavior,
            ImpactCallbackPolicy::ReselectBehavior,
        ),
        (
            5,
            0,
            ImpactCallbackPolicy::ReselectBehavior,
            ImpactCallbackPolicy::ReselectBehavior,
        ),
        (
            7,
            0,
            ImpactCallbackPolicy::ReselectBehavior,
            ImpactCallbackPolicy::ReselectBehavior,
        ),
        (
            7,
            1,
            ImpactCallbackPolicy::ReselectBehavior,
            ImpactCallbackPolicy::ReselectBehavior,
        ),
        (7, 2, ImpactCallbackPolicy::None, ImpactCallbackPolicy::None),
        (
            9,
            0,
            ImpactCallbackPolicy::ReselectBehavior,
            ImpactCallbackPolicy::ReselectBehavior,
        ),
        (
            9,
            1,
            ImpactCallbackPolicy::ReselectBehavior,
            ImpactCallbackPolicy::ReselectBehavior,
        ),
        (
            9,
            2,
            ImpactCallbackPolicy::None,
            ImpactCallbackPolicy::CapturePeopleCleanup,
        ),
        (
            9,
            3,
            ImpactCallbackPolicy::None,
            ImpactCallbackPolicy::CapturePeopleCleanup,
        ),
        (
            9,
            4,
            ImpactCallbackPolicy::None,
            ImpactCallbackPolicy::CapturePeopleCleanup,
        ),
        (
            9,
            5,
            ImpactCallbackPolicy::None,
            ImpactCallbackPolicy::CapturePeopleCleanup,
        ),
        (
            12,
            0,
            ImpactCallbackPolicy::None,
            ImpactCallbackPolicy::None,
        ),
        (
            12,
            1,
            ImpactCallbackPolicy::None,
            ImpactCallbackPolicy::None,
        ),
    ] {
        let style = ActiveBehaviorStyle::Audited(*audited_behavior_style(class, variant).unwrap());
        assert_eq!(callback_policy(style, HitEntry::Infected), Some(infected));
        assert_eq!(callback_policy(style, HitEntry::Primary), Some(primary));
    }
    // Exact executable Move5 completion row; independent of whether the live
    // task owner currently exposes that completion through the shared catalog.
    let move_completion = ActiveBehaviorStyle::Audited(crate::entity_behavior::BehaviorStyle {
        class_id: 5,
        variant: 1,
        frame_address: 0x004C_7978,
        release_callback_address: Some(0x0040_CE90),
        pair_contact_callback_address: None,
        impact_callback_address: None,
        death_callback_address: None,
    });
    for entry in [HitEntry::Primary, HitEntry::Infected] {
        assert_eq!(
            callback_policy(move_completion, entry),
            Some(ImpactCallbackPolicy::None)
        );
        assert_eq!(
            callback_policy(ActiveBehaviorStyle::InitializerFailureFallback, entry),
            Some(ImpactCallbackPolicy::None)
        );
        for class in [10, 33] {
            assert_eq!(
                callback_policy(
                    ActiveBehaviorStyle::Audited(*audited_behavior_style(class, 0).unwrap()),
                    entry
                ),
                None
            );
        }
    }
}
#[v2k_test_support::retail_test]
fn native_type16_both_births_reselect_before_primary_or_filtered_zero_infected_damage() {
    for spawn in [5, 42] {
        for (infected, amount, filtered) in [(false, 2500, 500), (false, 2000, 0), (true, 2000, 0)]
        {
            let Some((session, mut manager, _)) = native_intro2_fixture() else {
                return;
            };
            let id = prepare_target(&mut manager, spawn);
            let entity = manager.entity_mut(id).unwrap();
            let primary_before = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
            let position = entity.position_raw();
            let mut scheduler = SpecializedActorTaskScheduler::new();
            assert_eq!(scheduler.adopt_intro2_type16(&manager), 2);
            let mut fx = full_rate_fx();
            let delivery = impact(id, infected, amount);
            if infected {
                assert_eq!(
                    delivery.damage_delivery_record(),
                    Some(FUN_0043F780_DAMAGE_DELIVERY)
                );
            }
            let result = apply_intro2_type16_particle_hit(
                &mut manager,
                &session.cache,
                &mut fx,
                &mut scheduler,
                delivery,
                251,
            );
            let Intro2Type16ImpactOutcome::Applied(result) = result else {
                panic!("spawn{spawn} infected{infected}: {result:?}")
            };
            assert_eq!(result.filtered_damage_raw, filtered);
            assert!(result.death_publication.is_none());
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(12000 - filtered)
            );
            assert_eq!(
                entity.collision.last_hit_presentation_tick_at_0x34,
                RetailRuntimeValue::Known(if infected { 17 } else { 251 })
            );
            assert_ne!(
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                primary_before
            );
            assert_eq!(
                state_bits(entity, 0x2000).unwrap(),
                if infected { 0x2000 } else { 0 }
            );
            assert_eq!(
                scheduler.adopt_intro2_type16(&manager),
                0,
                "C690 already transferred its replacement owner"
            );
            fx.process_pending();
            let sounds = fx.take_positional_sounds();
            assert_eq!(
                sounds
                    .iter()
                    .map(|sound| sound.sound_id)
                    .collect::<Vec<_>>(),
                if infected {
                    vec![]
                } else if filtered != 0 {
                    vec![84]
                } else {
                    vec![]
                }
            );
            assert!(sounds.iter().all(|sound| sound.frequency_q16 == 0x10000));
            let particles = fx.prepare_presentation([640, 480], 0x3000, |_| {
                v2k_render::ParticleCenterProjection {
                    screen: [320, 240],
                    depth_raw: 1000,
                    clip: 0,
                }
            });
            assert_eq!(
                particles.particles().len(),
                if !infected && filtered != 0 { 2 } else { 0 }
            );
            let extent = session.cache.global_model(257).unwrap().radius;
            for particle in particles.particles() {
                assert_eq!(particle.particle.source_class, 5);
                assert_eq!(particle.particle.owner_id, Some(id));
                assert_eq!(
                    particle.particle.position,
                    [
                        f32::from(position[0] as u16) / 256.0,
                        f32::from(position[1]) / 256.0,
                        f32::from(position[2].wrapping_sub(extent as i16) as u16) / 256.0
                    ]
                );
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type16_capture_attached_styles_block_primary_but_allow_zero_infected_reaction() {
    for variant in 2..=5 {
        for infected in [false, true] {
            let Some((session, mut manager, _)) = native_intro2_fixture() else {
                return;
            };
            let id = prepare_target(&mut manager, 42);
            let entity = manager.entity_mut(id).unwrap();
            // Only the raw style is controlled. Infected entry has no hook, so
            // it must not inspect or invent Capture's attached task state.
            set_style(entity, 9, variant);
            let before_context = entity.current_behavior_context;
            let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
            let angles = entity.rotation_heading_pitch_roll_raw();
            let velocity = entity.velocity_raw();
            let mut fx = WorldFx::new();
            let result = apply_intro2_type16_particle_hit(
                &mut manager,
                &session.cache,
                &mut fx,
                &mut SpecializedActorTaskScheduler::new(),
                impact(id, infected, 2500),
                251,
            );
            if infected {
                let Intro2Type16ImpactOutcome::Applied(result) = result else {
                    panic!("{result:?}")
                };
                assert_eq!(result.filtered_damage_raw, 0);
                assert_eq!(
                    fx.next_shared_retail_random_u16(),
                    2437,
                    "zero-strength11030 still consumes three words"
                );
            } else {
                assert!(
                    matches!(
                        result,
                        Intro2Type16ImpactOutcome::Blocked {
                            reason: Intro2Type16ImpactBlock::Callback(
                                ImpactCallbackPolicy::CapturePeopleCleanup
                            ),
                            committed_prefix: true
                        }
                    ),
                    "{result:?}"
                );
                assert_eq!(fx.next_shared_retail_random_u16(), 38);
            }
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(12000)
            );
            assert_eq!(entity.current_behavior_context, before_context);
            assert_eq!(
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                primary
            );
            assert_eq!(entity.velocity_raw(), velocity);
            assert_eq!(
                entity.rotation_heading_pitch_roll_raw(),
                if infected {
                    angles.map(|a| a.wrapping_sub(0x400))
                } else {
                    angles
                }
            );
            assert_eq!(
                entity.collision.last_hit_presentation_tick_at_0x34,
                RetailRuntimeValue::Known(if infected { 17 } else { 251 })
            );
            fx.process_pending();
            assert!(fx.take_positional_sounds().is_empty());
            assert_eq!(fx.particle_count(), 0);
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type16_lethal_primary_publishes_class12_before_later_hits() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = prepare_target(&mut manager, 5);
    manager.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(1);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type16(&manager);
    let mut fx = full_rate_fx();
    let first = apply_intro2_type16_particle_hit(
        &mut manager,
        &session.cache,
        &mut fx,
        &mut scheduler,
        impact(id, false, 2500),
        251,
    );
    let Intro2Type16ImpactOutcome::Applied(first) = first else {
        panic!("{first:?}")
    };
    assert!(first.death_publication.is_some());
    assert_eq!(
        scheduler.family_for(id),
        Some(SpecializedActorTaskFamily::Intro2CommonDying)
    );
    assert!(Intro2CommonDyingOwner::adopt(&manager, id).is_ok());
    assert_eq!(
        fx.particle_count(),
        2,
        "lethal delivery retains capability8 suffix"
    );
    fx.process_pending();
    assert_eq!(
        fx.take_positional_sounds()
            .iter()
            .map(|sound| sound.sound_id)
            .collect::<Vec<_>>(),
        [86]
    );
    let primary = manager
        .entity_mut(id)
        .unwrap()
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary);
    let second = apply_intro2_type16_particle_hit(
        &mut manager,
        &session.cache,
        &mut fx,
        &mut scheduler,
        impact(id, false, 2500),
        252,
    );
    let Intro2Type16ImpactOutcome::Applied(second) = second else {
        panic!("{second:?}")
    };
    assert!(second.death_publication.is_none());
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        primary
    );
    assert_eq!(fx.particle_count(), 4);
    fx.process_pending();
    assert!(fx.take_positional_sounds().is_empty());
}

#[v2k_test_support::retail_test]
fn native_type16_late_checked_block_preserves_reselected_scheduler_owner() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = prepare_target(&mut manager, 42);
    let entity = manager.entity_mut(id).unwrap();
    let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
    entity.collision.health_raw = RetailRuntimeValue::Unresolved;
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type16(&manager);
    let result = apply_intro2_type16_particle_hit(
        &mut manager,
        &session.cache,
        &mut WorldFx::new(),
        &mut scheduler,
        impact(id, false, 2500),
        251,
    );
    assert!(
        matches!(
            result,
            Intro2Type16ImpactOutcome::Blocked {
                reason: Intro2Type16ImpactBlock::Damage(LiveActorDamageError {
                    phase: LiveActorDamagePhase::Health,
                    ..
                }),
                committed_prefix: true,
            }
        ),
        "{result:?}"
    );
    assert_ne!(
        manager
            .entity_mut(id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary),
        primary
    );
    assert_eq!(scheduler.adopt_intro2_type16(&manager), 0);
}

#[v2k_test_support::retail_test]
fn native_type16_pending_actor_prefix_rejects_both_hits_without_mutation() {
    for spawn in [5, 42] {
        for infected in [false, true] {
            let Some((session, mut manager, _)) = native_intro2_fixture() else {
                return;
            };
            let id = prepare_target(&mut manager, spawn);
            let entity = manager.entity_mut(id).unwrap();
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                panic!("native birth must publish its current style")
            };
            assert_eq!(
                callback_policy(context.active_style(), HitEntry::Primary),
                Some(ImpactCallbackPolicy::ReselectBehavior),
                "the pending gate must precede an otherwise valid C690 callback"
            );
            let collision = entity.collision.clone();
            let body = (
                entity.position_raw(),
                entity.velocity_raw(),
                entity.rotation_heading_pitch_roll_raw(),
                entity.mass_raw,
            );
            let tasks = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
                (
                    entity.actor_tasks.task_in_slot(slot),
                    entity.actor_task_state(slot).copied(),
                )
            });
            let a = entity.sub_a_propulsion_runtime;
            let h = entity.sub_h_external_frame_runtime.clone();
            let axis = entity.actor_common_axis_descriptor;
            let identity = entity.intro2_type16_runtime;
            let owner = Intro2Type16Owner::adopt_blocked_prefix(&manager, id).unwrap();
            let mut scheduler = SpecializedActorTaskScheduler::new();
            assert_eq!(scheduler.adopt_intro2_type16(&manager), 2);
            scheduler.register_intro2_type16(owner);
            assert!(scheduler.intro2_type16_has_pending_prefix(id));
            let mut fx = WorldFx::new();
            let result = apply_intro2_type16_particle_hit(
                &mut manager,
                &session.cache,
                &mut fx,
                &mut scheduler,
                impact(id, infected, 2500),
                251,
            );
            assert_eq!(
                result,
                Intro2Type16ImpactOutcome::Blocked {
                    reason: Intro2Type16ImpactBlock::Runtime("pending actor prefix"),
                    committed_prefix: false,
                }
            );
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(entity.collision, collision);
            assert_eq!(
                entity.current_behavior_context,
                RetailRuntimeValue::Known(Some(context))
            );
            assert_eq!(
                (
                    entity.position_raw(),
                    entity.velocity_raw(),
                    entity.rotation_heading_pitch_roll_raw(),
                    entity.mass_raw,
                ),
                body
            );
            assert_eq!(
                ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
                    (
                        entity.actor_tasks.task_in_slot(slot),
                        entity.actor_task_state(slot).copied(),
                    )
                }),
                tasks
            );
            assert_eq!(entity.sub_a_propulsion_runtime, a);
            assert_eq!(entity.sub_h_external_frame_runtime, h);
            assert_eq!(entity.actor_common_axis_descriptor, axis);
            assert_eq!(entity.intro2_type16_runtime, identity);
            assert!(scheduler.intro2_type16_has_pending_prefix(id));
            assert_eq!(
                scheduler.family_for(id),
                Some(SpecializedActorTaskFamily::Intro2Type16)
            );
            assert_eq!(scheduler.adopt_intro2_type16(&manager), 0);
            assert!(scheduler.intro2_type16_has_pending_prefix(id));
            assert_eq!(fx.next_shared_retail_random_u16(), 38);
            assert_eq!(fx.pending_event_count(), 0);
            assert!(fx.take_positional_sounds().is_empty());
            assert_eq!(fx.particle_count(), 0);
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type16_wrong_allocation_cannot_commit_either_hit_prefix() {
    for infected in [false, true] {
        let Some((session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = prepare_target(&mut manager, 5);
        let entity = manager.entity_mut(id).unwrap();
        entity.intro2_type16_runtime = None;
        let flags = entity.collision.state_flags_at_0x08;
        let mut fx = WorldFx::new();
        let result = apply_intro2_type16_particle_hit(
            &mut manager,
            &session.cache,
            &mut fx,
            &mut SpecializedActorTaskScheduler::new(),
            impact(id, infected, 2500),
            251,
        );
        assert!(
            matches!(
                result,
                Intro2Type16ImpactOutcome::Blocked {
                    reason: Intro2Type16ImpactBlock::Runtime("native allocation"),
                    committed_prefix: false
                }
            ),
            "{result:?}"
        );
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(17)
        );
        assert_eq!(entity.collision.state_flags_at_0x08, flags);
        assert_eq!(fx.next_shared_retail_random_u16(), 38);
        assert_eq!(fx.pending_event_count(), 0);
    }
}
