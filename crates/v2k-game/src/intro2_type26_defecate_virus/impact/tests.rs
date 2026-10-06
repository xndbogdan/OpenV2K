use super::*;
use crate::{
    damage::{DamagePacket, FUN_0043F780_DAMAGE_DELIVERY},
    entity_behavior::{audited_behavior_style, behavior_program},
    intro2_type47_live::world::native_intro2_fixture,
    world_fx::BallisticDamageRequest,
};

fn impact(id: u32, infected: bool, amount: i32) -> ParticleEntityImpact {
    ParticleEntityImpact {
        source_particle_class: if infected { 5 } else { 1 },
        impact_position_argument_va: if infected { 0x004D_CF48 } else { 0 },
        target_entity_id: id,
        position_world: [0.0; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(BallisticDamageRequest {
            packet: if infected {
                FUN_0043F780_DAMAGE_DELIVERY.packet
            } else {
                DamagePacket {
                    channels: [2, 0],
                    amounts_raw: [amount, 0],
                }
            },
            // F780 must replace these particle-birth values by zero/zero.
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

fn full_rate_fx() -> WorldFx {
    let mut fx = WorldFx::new();
    // 44FA30's cold governor starts at minimum-one. Eight real frame writes
    // replace that history; 40DC0 then attempts both scale0800 carriers.
    for _ in 0..8 {
        fx.advance_frame_pacing(20_000);
    }
    fx
}

fn presented_particles(fx: &mut WorldFx) -> Vec<crate::world_fx::WorldParticle> {
    fx.prepare_presentation([640, 480], 0x3000, |_| {
        v2k_render::ParticleCenterProjection {
            screen: [320, 240],
            depth_raw: 1000,
            clip: 0,
        }
    })
    .particles()
    .map(|prepared| prepared.particle)
    .collect()
}

#[test]
fn native_type26_authenticates_each_primary_and_infected_style_word() {
    for (class, variant) in [(4, 0), (26, 0), (33, 0), (33, 1), (12, 0)] {
        let style = ActiveBehaviorStyle::Audited(*audited_behavior_style(class, variant).unwrap());
        for entry in [HitEntry::Primary, HitEntry::Infected] {
            assert_eq!(
                callback_policy(style, entry),
                Some(if class == 12 {
                    ImpactCallbackPolicy::None
                } else {
                    ImpactCallbackPolicy::ReselectBehavior
                })
            );
        }
    }
    for entry in [HitEntry::Primary, HitEntry::Infected] {
        assert_eq!(
            callback_policy(ActiveBehaviorStyle::InitializerFailureFallback, entry),
            Some(ImpactCallbackPolicy::None)
        );
        let unrelated = ActiveBehaviorStyle::Audited(*audited_behavior_style(9, 1).unwrap());
        assert_eq!(
            unrelated.audited().unwrap().impact_callback_policy(),
            ImpactCallbackPolicy::ReselectBehavior
        );
        assert_eq!(
            callback_policy(unrelated, entry),
            None,
            "same callback is not Type26 style authority"
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type26_hits_reselect_both_births_and_keep_infected_prefix_distinct() {
    for spawn in [10, 25] {
        for infected in [false, true] {
            let Some((session, mut manager, _)) = native_intro2_fixture() else {
                return;
            };
            let id = spawn_id(&manager, spawn);
            let metadata = manager.type_runtime_metadata(26).unwrap().clone();
            let entity = manager.entity_mut(id).unwrap();
            entity.collision.health_raw = RetailRuntimeValue::Known(100_000);
            entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(10_000);
            entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(0x3000, 0x1000);
            let old_primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
            let position = entity.position_raw();
            let mut scheduler = SpecializedActorTaskScheduler::new();
            scheduler.adopt_intro2_type26(&manager);
            let mut fx = full_rate_fx();
            let delivery = impact(id, infected, 2000);
            if infected {
                assert_eq!(
                    delivery.damage_delivery_record(),
                    Some(FUN_0043F780_DAMAGE_DELIVERY)
                );
            }
            let result = apply_intro2_type26_particle_hit(
                &mut manager,
                &session.cache,
                &mut fx,
                &mut scheduler,
                delivery,
                251,
            );
            let Intro2Type26ImpactOutcome::Applied(result) = result else {
                panic!("spawn{spawn} infected{infected}: {result:?}")
            };
            assert_eq!(result.filtered_damage_raw, if infected { 0 } else { 200 });
            assert_eq!(result.damage_after_buffer_raw, 0);
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(100_000)
            );
            assert_eq!(
                entity.collision.last_hit_presentation_tick_at_0x34,
                RetailRuntimeValue::Known(if infected { 17 } else { 251 })
            );
            assert_ne!(
                old_primary,
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                "impact C690 bypasses 16410 suppression"
            );
            assert_eq!(
                state_bits(entity, 0x2000).unwrap(),
                if infected { 0x2000 } else { 0 }
            );
            assert_eq!(
                scheduler.adopt_intro2_type26(&manager),
                0,
                "hit retained the replacement graph"
            );
            let mut expected_sounds = Vec::new();
            if infected {
                if let RetailRuntimeValue::Known(Some(sound)) =
                    metadata.infected_model_presentation_sound_id
                {
                    expected_sounds.push(sound as usize);
                }
            }
            if !infected {
                if let RetailRuntimeValue::Known(Some(sound)) = metadata.generic_hit_sound_id {
                    expected_sounds.push(sound as usize);
                }
            }
            if !infected {
                if let RetailRuntimeValue::Known(Some(sound)) =
                    metadata.accepted_hit_presentation_sound_id
                {
                    expected_sounds.push(sound as usize);
                }
            }
            fx.process_pending();
            let sounds = fx.take_positional_sounds();
            assert_eq!(
                sounds
                    .iter()
                    .map(|sound| sound.sound_id)
                    .collect::<Vec<_>>(),
                expected_sounds
            );
            assert!(sounds.iter().all(|sound| sound.frequency_q16 == 0x10000));
            let particles = presented_particles(&mut fx);
            assert_eq!(particles.len(), if infected { 0 } else { 2 });
            let extent = session.cache.global_model(267).unwrap().radius;
            // Emitter words wrap before conversion into the canonical world:
            // X/Z are unsigned toroidal coordinates; only Y stays signed.
            let expected_position = [
                f32::from(position[0] as u16) / 256.0,
                f32::from(position[1]) / 256.0,
                f32::from(position[2].wrapping_sub(extent as i16) as u16) / 256.0,
            ];
            for particle in particles {
                assert_eq!(particle.source_class, 5);
                assert_eq!(particle.position, expected_position);
                assert_eq!(particle.owner_id, Some(id));
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type26_negative_primary_return_still_runs_sound_and_capability_suffix() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = spawn_id(&manager, 25);
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.health_raw = RetailRuntimeValue::Known(100_000);
    entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
    let mut fx = full_rate_fx();
    let result = apply_intro2_type26_particle_hit(
        &mut manager,
        &session.cache,
        &mut fx,
        &mut SpecializedActorTaskScheduler::new(),
        // Positive input passes the strict threshold; the signed Q8 product
        // wraps at 0x80000000 and therefore returns -0x00800000.
        impact(id, false, 1800 + 0x0080_0000),
        700,
    );
    let Intro2Type26ImpactOutcome::Applied(result) = result else {
        panic!("{result:?}")
    };
    assert_eq!(result.filtered_damage_raw, -0x0080_0000);
    assert_eq!(
        manager.entity_mut(id).unwrap().collision.health_raw,
        RetailRuntimeValue::Known(100_000 - result.filtered_damage_raw)
    );
    assert_eq!(fx.particle_count(), 2);
    let RetailRuntimeValue::Known(Some(sound)) = manager
        .type_runtime_metadata(26)
        .unwrap()
        .accepted_hit_presentation_sound_id
    else {
        panic!("Type26 authored +80 cue");
    };
    fx.process_pending();
    assert_eq!(
        fx.take_positional_sounds().last().unwrap().sound_id,
        sound as usize
    );
}

#[v2k_test_support::retail_test]
fn native_type26_lethal_primary_transfers_class12_and_later_hits_keep_its_task() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = spawn_id(&manager, 10);
    manager.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(1);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type26(&manager);
    let mut fx = full_rate_fx();
    let first = apply_intro2_type26_particle_hit(
        &mut manager,
        &session.cache,
        &mut fx,
        &mut scheduler,
        impact(id, false, 2000),
        251,
    );
    let Intro2Type26ImpactOutcome::Applied(first) = first else {
        panic!("{first:?}")
    };
    assert!(first.death_publication.is_some());
    assert!(Intro2CommonDyingOwner::adopt(&manager, id).is_ok());
    assert_eq!(
        manager.entity_mut(id).unwrap().collision.health_raw,
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(
        fx.particle_count(),
        2,
        "dying skips cue, not capability suffix"
    );
    fx.process_pending();
    fx.take_positional_sounds();
    let primary = manager
        .entity_mut(id)
        .unwrap()
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary);
    let second = apply_intro2_type26_particle_hit(
        &mut manager,
        &session.cache,
        &mut fx,
        &mut scheduler,
        impact(id, false, 2000),
        252,
    );
    let Intro2Type26ImpactOutcome::Applied(second) = second else {
        panic!("{second:?}")
    };
    assert!(second.death_publication.is_none());
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary),
        primary
    );
    assert_eq!(fx.particle_count(), 4);
    fx.process_pending();
    assert!(fx.take_positional_sounds().is_empty());
}

#[v2k_test_support::retail_test]
fn native_type26_authored_fragment_filters_zero_after_the_primary_callback() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = spawn_id(&manager, 25);
    let profile = manager
        .type_runtime_metadata(26)
        .unwrap()
        .damage_profile
        .unwrap();
    assert_eq!(profile.thresholds_raw, [0, 6000, 1800, 2000, 200, 0, 0]);
    assert_eq!(profile.multipliers_q8, [0, 256, 256, 256, 128, 0, 0]);
    let entity = manager.entity_mut(id).unwrap();
    let health = entity.collision.health_raw;
    let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
    let mut fragment = impact(id, false, 0);
    fragment.source_particle_class = 16;
    fragment.damage.as_mut().unwrap().packet = crate::damage::BALLISTIC_PARTICLE_DAMAGE_PACKET;
    let mut fx = WorldFx::new();
    let result = apply_intro2_type26_particle_hit(
        &mut manager,
        &session.cache,
        &mut fx,
        &mut SpecializedActorTaskScheduler::new(),
        fragment,
        91,
    );
    let Intro2Type26ImpactOutcome::Applied(result) = result else {
        panic!("{result:?}")
    };
    assert_eq!(result.filtered_damage_raw, 0);
    assert!(result.death_publication.is_none());
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.collision.health_raw, health);
    assert_eq!(
        entity.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(91)
    );
    assert_ne!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        primary
    );
    assert_eq!(fx.particle_count(), 0);
    assert_eq!(
        fx.pending_event_count(),
        0,
        "filtered-zero skips the primary suffix"
    );
}

#[v2k_test_support::retail_test]
fn native_type26_infected_unaudited_hook_preserves_only_11250_prefix() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = spawn_id(&manager, 25);
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.state_flags_at_0x08.overwrite(0x2000, 0);
    entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(
        BehaviorContextRuntime::from_fresh_weighted_selection(BehaviorSelection {
            choice_index: 0,
            program: behavior_program(9).unwrap(),
        })
        .unwrap(),
    ));
    let health = entity.collision.health_raw;
    let velocity = entity.velocity_raw();
    let mut fx = WorldFx::new();
    let result = apply_intro2_type26_particle_hit(
        &mut manager,
        &session.cache,
        &mut fx,
        &mut SpecializedActorTaskScheduler::new(),
        impact(id, true, 2000),
        251,
    );
    assert_eq!(
        result,
        Intro2Type26ImpactOutcome::Blocked {
            reason: Intro2Type26ImpactBlock::Runtime("unaudited style hit slot"),
            committed_prefix: true
        }
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(state_bits(entity, 0x2000).unwrap(), 0x2000);
    assert_eq!(
        entity.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(17)
    );
    assert_eq!(entity.collision.health_raw, health);
    assert_eq!(entity.velocity_raw(), velocity);
    assert_eq!(fx.particle_count(), 0);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        WorldFx::new().next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type26_root_failure_keeps_committed_context_without_fabricating_fallback() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = spawn_id(&manager, 25);
    session
        .cache
        .level_terrain_mut()
        .unwrap()
        .cells
        .iter_mut()
        .for_each(|cell| cell.attribute = 0);
    let entity = manager.entity_mut(id).unwrap();
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Unresolved;
    let health = entity.collision.health_raw;
    let velocity = entity.velocity_raw();
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type26(&manager);
    let mut fx = WorldFx::new();
    let result = apply_intro2_type26_particle_hit(
        &mut manager,
        &session.cache,
        &mut fx,
        &mut scheduler,
        impact(id, false, 2000),
        251,
    );
    assert_eq!(
        result,
        Intro2Type26ImpactOutcome::Blocked {
            reason: Intro2Type26ImpactBlock::Behavior(Intro2Type26WorldBlock::Publication(
                Intro2Type26DefecateVirusPublicationError::SubARuntimeUnavailable
            )),
            committed_prefix: true,
        }
    );
    let entity = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!("committed selection");
    };
    assert_eq!(context.active_style().style_address(), 0x004C_7B28);
    assert_eq!(entity.collision.health_raw, health);
    assert_eq!(entity.velocity_raw(), velocity);
    assert_eq!(
        entity.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(251)
    );
    assert_eq!(scheduler.adopt_intro2_type26(&manager), 0);
    let mut expected = WorldFx::new();
    expected.next_shared_retail_random_u16(); // C690's selector; no constructor/impulse suffix.
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
    let owner = Intro2Type26WorldOwner::adopt_blocked_prefix(&manager, id).unwrap();
    let result = tick_intro2_type26_world(
        &mut manager,
        owner,
        Intro2Type26WorldFrame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            retail_tick: 252,
        },
    );
    assert!(matches!(
        result.outcome,
        Intro2Type26WorldOutcome::Pending { .. }
    ));
}

#[v2k_test_support::retail_test]
fn native_type26_task_result_suppression_preserves_rng_and_existing_graph() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = spawn_id(&manager, 25);
    let entity = manager.entity_mut(id).unwrap();
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x1000, 0x1000);
    let context = entity.current_behavior_context;
    let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
    let mut fx = WorldFx::new();
    super::super::world::reselect_behavior(
        &mut manager,
        id,
        super::super::world::Type26ReselectionFrame {
            resources: &session.cache,
            world_fx: &mut fx,
            entry: super::super::world::Type26ReselectionEntry::TaskResult,
        },
    )
    .unwrap();
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.current_behavior_context, context);
    assert_eq!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        primary
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        WorldFx::new().next_shared_retail_random_u16()
    );
}

fn static_route_hit(id: u32, class: u8) -> ParticleEntityImpact {
    let packet = match class {
        52 => crate::damage::TYPE_47_PROJECTILE_DAMAGE_PACKET,
        68 => crate::damage::CLASS68_STATIC_ROUTE_DAMAGE_PACKET,
        85 => crate::damage::DRAGON_FIREBALL_DAMAGE_PACKET,
        _ => unreachable!(),
    };
    ParticleEntityImpact {
        source_particle_class: class,
        impact_position_argument_va: 0x004D_EFC8,
        target_entity_id: id,
        position_world: [179.28014, 0.8351769, 23.153553],
        velocity_raw: [1110, -3812, 1010],
        damage: Some(BallisticDamageRequest {
            packet,
            source_entity_type_at_birth: Some(57),
            source_owner_id: Some(2),
        }),
    }
}

#[v2k_test_support::retail_test]
fn native_type26_static_route_uses_11180_for_all_three_packets_without_primary_suffix() {
    for spawn in [10, 25] {
        for (class, expected_damage) in [(52, 0), (68, 200), (85, 4000)] {
            let Some((session, mut manager, _)) = native_intro2_fixture() else {
                return;
            };
            let id = spawn_id(&manager, spawn);
            let entity = manager.entity_mut(id).unwrap();
            entity.collision.health_raw = RetailRuntimeValue::Known(100_000);
            entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
            entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
            let old_primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
            let mut scheduler = SpecializedActorTaskScheduler::new();
            scheduler.adopt_intro2_type26(&manager);
            let mut fx = full_rate_fx();
            let result = apply_intro2_type26_particle_hit(
                &mut manager,
                &session.cache,
                &mut fx,
                &mut scheduler,
                static_route_hit(id, class),
                2428,
            );
            let Intro2Type26ImpactOutcome::Applied(applied) = result else {
                panic!("spawn{spawn} class{class}: {result:?}");
            };
            assert_eq!(applied.filtered_damage_raw, expected_damage);
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(100_000 - expected_damage)
            );
            assert_eq!(
                entity.collision.last_hit_presentation_tick_at_0x34,
                RetailRuntimeValue::Known(2428)
            );
            assert_ne!(
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                old_primary,
                "DAC0/C690 precedes the damage filter even for zero filtered damage"
            );
            assert_eq!(
                fx.particle_count(),
                0,
                "11180 has no capability8 class5 suffix"
            );
            fx.process_pending();
            let expected: Vec<_> = session
                .cache
                .global_entity_type(26)
                .unwrap()
                .accepted_hit_presentation_sound_id()
                .into_iter()
                .map(usize::from)
                .collect();
            assert_eq!(
                fx.take_positional_sounds()
                    .into_iter()
                    .map(|sound| sound.sound_id)
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type26_static_route_impulse_has_eightfold_primary_force() {
    let mut velocities = Vec::new();
    for static_route in [false, true] {
        let Some((session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = spawn_id(&manager, 10);
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.mass_raw, 400, "actual authored Type26 reaction mass");
        entity.set_velocity_raw([0; 3]);
        entity.collision.health_raw = RetailRuntimeValue::Known(100_000);
        entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
        entity.collision.state_flags_at_0x08.overwrite(
            IMPACT_REACTION_ENABLED_STATE_BIT
                | IMPACT_REACTION_SUPPRESSED_STATE_BIT
                | IMPACT_REACTION_NETWORKED_STATE_BIT,
            IMPACT_REACTION_ENABLED_STATE_BIT,
        );
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.adopt_intro2_type26(&manager);
        let mut fx = full_rate_fx();
        let mut delivery = static_route_hit(id, 68);
        delivery.velocity_raw = [0, 0, 8192];
        if !static_route {
            delivery.source_particle_class = 1;
        }
        let result = apply_intro2_type26_particle_hit(
            &mut manager,
            &session.cache,
            &mut fx,
            &mut scheduler,
            delivery,
            2428,
        );
        assert!(
            matches!(result, Intro2Type26ImpactOutcome::Applied(_)),
            "{result:?}"
        );
        velocities.push(manager.entity_mut(id).unwrap().velocity_raw());
    }
    // 11030 first divides0x400000 by authored mass400, yielding10485.
    // The Q16 scales are319 for the original2000 impact and2559 for
    // 11180's16000 impact; direction8192 then gives Q15 deltas79 and639.
    // Multiplying the already quantized primary79 by8 would give632 and
    // would place the11180 multiplier after the native fixed-point stages.
    assert_eq!(
        velocities[0],
        [0, 0, 79],
        "native primary force through11030"
    );
    assert_eq!(
        velocities[1],
        [0, 0, 639],
        "11180 multiplies25590 before11030's two shifts"
    );
}

#[v2k_test_support::retail_test]
fn native_type26_static_route_lethal_and_later_hits_retain_actual_class12() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = spawn_id(&manager, 10);
    manager.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(1);
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type26(&manager);
    let mut fx = full_rate_fx();
    let result = apply_intro2_type26_particle_hit(
        &mut manager,
        &session.cache,
        &mut fx,
        &mut scheduler,
        static_route_hit(id, 68),
        2428,
    );
    let Intro2Type26ImpactOutcome::Applied(applied) = result else {
        panic!("{result:?}");
    };
    assert!(applied.death_publication.is_some());
    assert!(Intro2CommonDyingOwner::adopt(&manager, id).is_ok());
    assert_eq!(
        manager.entity_mut(id).unwrap().collision.health_raw,
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(fx.particle_count(), 0);
    let record = session.cache.global_entity_type(26).unwrap();
    let expected: Vec<_> = record
        .accepted_hit_presentation_sound_id()
        .into_iter()
        .chain(record.death_sound_id())
        .map(usize::from)
        .collect();
    fx.process_pending();
    assert_eq!(
        fx.take_positional_sounds()
            .into_iter()
            .map(|sound| sound.sound_id)
            .collect::<Vec<_>>(),
        expected
    );
    let primary = manager
        .entity_mut(id)
        .unwrap()
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary);
    let result = apply_intro2_type26_particle_hit(
        &mut manager,
        &session.cache,
        &mut fx,
        &mut scheduler,
        static_route_hit(id, 68),
        2429,
    );
    let Intro2Type26ImpactOutcome::Applied(applied) = result else {
        panic!("{result:?}");
    };
    assert!(applied.death_publication.is_none());
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary),
        primary
    );
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(2429)
    );
    assert_eq!(fx.particle_count(), 0);
    fx.process_pending();
    assert!(fx.take_positional_sounds().is_empty());
}

#[v2k_test_support::retail_test]
fn native_type26_static_route_commits_cue_before_c690_failure_without_replay() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = spawn_id(&manager, 10);
    let health = manager.entity_mut(id).unwrap().collision.health_raw;
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type26(&manager);
    let mut fx = WorldFx::new();
    // The actual allocation and metadata remain intact; this detached caller
    // supplies no terrain for the reached C690 constructor preflight.
    let resources = ResourceCache::new(Vec::new());
    let result = apply_intro2_type26_particle_hit(
        &mut manager,
        &resources,
        &mut fx,
        &mut scheduler,
        static_route_hit(id, 68),
        2428,
    );
    assert!(
        matches!(
            result,
            Intro2Type26ImpactOutcome::Blocked {
                reason: Intro2Type26ImpactBlock::Behavior(_),
                committed_prefix: true,
            }
        ),
        "{result:?}"
    );
    assert_eq!(manager.entity_mut(id).unwrap().collision.health_raw, health);
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(2428)
    );
    fx.process_pending();
    let expected: Vec<_> = session
        .cache
        .global_entity_type(26)
        .unwrap()
        .accepted_hit_presentation_sound_id()
        .into_iter()
        .map(usize::from)
        .collect();
    assert_eq!(
        fx.take_positional_sounds()
            .into_iter()
            .map(|sound| sound.sound_id)
            .collect::<Vec<_>>(),
        expected
    );
    let result = apply_intro2_type26_particle_hit(
        &mut manager,
        &session.cache,
        &mut fx,
        &mut scheduler,
        static_route_hit(id, 68),
        2429,
    );
    assert!(
        matches!(
            result,
            Intro2Type26ImpactOutcome::Blocked {
                reason: Intro2Type26ImpactBlock::Runtime("completed actor custody"),
                committed_prefix: false,
            }
        ),
        "{result:?}"
    );
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(2428)
    );
    assert_eq!(fx.pending_event_count(), 0);
}
