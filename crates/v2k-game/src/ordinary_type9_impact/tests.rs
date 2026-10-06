//! Intro2 cohort controls for the shared native Type9 hit path.
use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    damage::{DamagePacket, FUN_0043F780_DAMAGE_PACKET},
    entity_collision_state::SURFACE_STATE_MASK,
    intro2_type47_live::world::native_intro2_fixture,
    specialized_actor_task_production::{
        SpecializedActorTaskFamily, SpecializedActorTaskProductionFrame,
    },
    static_damage::StaticDamageScheduler,
    world_fx::BallisticDamageRequest,
};

fn hit(id: u32, infected: bool, amount: i32) -> ParticleEntityImpact {
    ParticleEntityImpact {
        source_particle_class: if infected { 5 } else { 16 },
        impact_position_argument_va: if infected { 0x004d_cf48 } else { 0 },
        target_entity_id: id,
        position_world: [0.0; 3],
        velocity_raw: [700, -200, 900],
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
    }
}

fn class(entity: &Entity) -> u8 {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!()
    };
    context.active_style().audited().unwrap().class_id
}

#[v2k_test_support::retail_test]
fn native_type9_antidote_reselects_run_away_then_filters_damage_without_primary_stamp_or_suffix() {
    let Some((_session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 9 && class(entity) == 10)
        .unwrap()
        .id;
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type9(&mut manager);
    let metadata = manager.type_runtime_metadata(9).unwrap().clone();
    let RetailRuntimeValue::Known(cue) = metadata.cured_model_presentation_sound_id else {
        panic!()
    };
    let entity = manager.entity_mut(id).unwrap();
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x2000, 0x2000);
    entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
    let RetailRuntimeValue::Known(health) = entity.collision.health_raw else {
        panic!()
    };
    let tasks = task_visits(entity);
    let mut impact = hit(id, false, 1000);
    impact.source_particle_class = 6;
    impact.damage.as_mut().unwrap().packet = crate::damage::FUN_0043F7C0_DAMAGE_PACKET;
    let mut fx = WorldFx::new();
    let result = apply_native_type9_particle_hit(
        &mut manager,
        &mut fx,
        &mut scheduler,
        &mut GameplayNotifications::new(),
        impact,
        251,
    );
    let NativeType9ImpactOutcome::Applied(result) = result else {
        panic!("{result:?}")
    };
    assert_eq!(result.filtered_damage_raw, 600);
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity.collision.health_raw,
        RetailRuntimeValue::Known(health - 600)
    );
    assert_eq!(
        entity.collision.state_flags_at_0x08.masked(0x2000),
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(
        entity.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(17)
    );
    assert_ne!(
        task_visits(entity),
        tasks,
        "Run Away+24 invokesC690 before15040"
    );
    assert!(scheduler
        .begin_native_type9_external_mutation(&manager, id)
        .is_some());
    fx.process_pending();
    assert_eq!(
        fx.take_positional_sounds()
            .iter()
            .map(|sound| sound.sound_id)
            .collect::<Vec<_>>(),
        cue.into_iter().map(usize::from).collect::<Vec<_>>()
    );
    assert_eq!(fx.particle_count(), 0, "11320 has no10EB0 class5 suffix");
}

#[v2k_test_support::retail_test]
fn native_type9_repeated_particle_hits_retain_current_graph_and_infected_bits() {
    let Some((_session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let ids: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 9)
        .map(|entity| entity.id)
        .collect();
    assert_eq!(ids.len(), 13);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_type9(&mut manager), 13);
    let mut fx = WorldFx::new();
    let mut notifications = GameplayNotifications::new();
    for id in ids {
        manager.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(1_000_000);
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
        for (tick, infected) in [(25, false), (26, true), (27, true), (28, false)] {
            let entity = manager.entity_mut(id).unwrap();
            let expected_reselection = calls_c690(
                entity,
                if infected {
                    EntityHitEntry::Infected
                } else {
                    EntityHitEntry::PrimaryProjectile
                },
            )
            .unwrap();
            let old_visits = super::task_visits(entity);
            let old_stamp = entity.collision.last_hit_presentation_tick_at_0x34;
            let old_anchor = entity
                .ordinary_type9_selected_component_runtime
                .unwrap()
                .components()
                .immutable_anchor_raw_at_0x90();
            let result = apply_native_type9_particle_hit(
                &mut manager,
                &mut fx,
                &mut scheduler,
                &mut notifications,
                hit(id, infected, 1000),
                tick,
            );
            assert!(
                matches!(result, NativeType9ImpactOutcome::Applied(_)),
                "id{id} infected{infected}: {result:?}"
            );
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(
                super::task_visits(entity) != old_visits,
                expected_reselection
            );
            assert_eq!(
                entity.collision.last_hit_presentation_tick_at_0x34,
                if infected {
                    old_stamp
                } else {
                    RetailRuntimeValue::Known(tick)
                }
            );
            assert_eq!(
                entity
                    .collision
                    .state_flags_at_0x08
                    .masked(SURFACE_STATE_MASK),
                RetailRuntimeValue::Unresolved
            );
            assert_eq!(
                entity.collision.active_model_slot(),
                RetailRuntimeValue::Known(if tick == 25 { 0 } else { 2 })
            );
            assert_eq!(entity.model_slots, [Some(558); 4]);
            assert_eq!(
                entity
                    .ordinary_type9_selected_component_runtime
                    .unwrap()
                    .components()
                    .immutable_anchor_raw_at_0x90(),
                old_anchor
            );
            assert!(
                scheduler
                    .begin_native_type9_external_mutation(&manager, id)
                    .is_some(),
                "new graph must be immediately reusable"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type9_run_away_primary_zero_impact_draws_three_words_without_reselection() {
    let Some((_session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 9 && class(entity) == 10)
        .expect("authored prefix selects Run Away")
        .id;
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type9(&mut manager);
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.state_flags_at_0x08.overwrite(
        IMPACT_REACTION_ENABLED_STATE_BIT | IMPACT_REACTION_SUPPRESSED_STATE_BIT,
        IMPACT_REACTION_ENABLED_STATE_BIT,
    );
    let old_tasks =
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).cloned());
    let old_pose = (
        entity.velocity_raw(),
        entity.rotation_heading_pitch_roll_raw(),
    );
    let mut fx = WorldFx::new();
    let mut control = WorldFx::new();
    let result = apply_native_type9_particle_hit(
        &mut manager,
        &mut fx,
        &mut scheduler,
        &mut GameplayNotifications::new(),
        hit(id, false, 0),
        25,
    );
    assert!(
        matches!(result, NativeType9ImpactOutcome::Applied(_)),
        "{result:?}"
    );
    for _ in 0..3 {
        control.next_shared_retail_random_u16();
    }
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        control.next_shared_retail_random_u16()
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).cloned()),
        old_tasks
    );
    assert_eq!(
        (
            entity.velocity_raw(),
            entity.rotation_heading_pitch_roll_raw()
        ),
        (
            old_pose.0,
            old_pose.1.map(|angle| angle.wrapping_sub(0x400))
        )
    );
    assert!(fx.take_positional_sounds().is_empty());
    assert_eq!(fx.particle_count(), 0);
}

#[v2k_test_support::retail_test]
fn native_type9_lethal_primary_and_infected_hits_keep_class14_custody() {
    for infected in [false, true] {
        let Some((_session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 9)
            .unwrap()
            .id;
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.adopt_intro2_type9(&mut manager);
        let mut fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let result = apply_native_type9_particle_hit(
            &mut manager,
            &mut fx,
            &mut scheduler,
            &mut notifications,
            hit(id, infected, 20_000),
            25,
        );
        let NativeType9ImpactOutcome::Applied(result) = result else {
            panic!("{result:?}")
        };
        let lease = result
            .death_publication
            .expect("lethal callback retains an exact task receipt");
        assert_eq!(lease.actor().entity_id, id);
        assert_eq!(
            scheduler.family_for(id),
            Some(SpecializedActorTaskFamily::Intro2Type9Class14)
        );
        assert_eq!(
            manager
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
        fx.process_pending();
        let sounds = fx.take_positional_sounds();
        assert!(sounds.iter().any(|sound| sound.sound_id == 35));
        assert!(
            sounds.iter().all(|sound| sound.sound_id != 95),
            "accepted cue observes final dying state"
        );
        let primary = manager
            .entity_mut(id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary);
        let again = apply_native_type9_particle_hit(
            &mut manager,
            &mut fx,
            &mut scheduler,
            &mut notifications,
            hit(id, infected, 20_000),
            26,
        );
        assert!(
            matches!(again, NativeType9ImpactOutcome::Applied(_)),
            "{again:?}"
        );
        assert_eq!(
            manager
                .entity_mut(id)
                .unwrap()
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary),
            primary
        );
        assert_eq!(
            scheduler.family_for(id),
            Some(SpecializedActorTaskFamily::Intro2Type9Class14)
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type9_hit_after_completed_mover_consumes_no_extra_actor_visit() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 9 && class(entity) == 10)
        .unwrap()
        .id;
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type9(&mut manager);
    let mut fx = WorldFx::new();
    let mut notifications = GameplayNotifications::new();
    let mut static_damage = StaticDamageScheduler::new();
    let pass = scheduler.tick(
        &mut manager,
        SpecializedActorTaskProductionFrame {
            world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase:
                crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            retail_tick: 6,
            main_base_abort_active: false,
        },
        &mut notifications,
    );
    assert_eq!(pass.block, None);
    let entity = manager.entity_mut(id).unwrap();
    let before = (
        entity.position_raw(),
        entity.collision.callback_scheduler_accumulator_us_at_0x6c,
        entity
            .ordinary_type9_selected_component_runtime
            .unwrap()
            .components()
            .sub_d_frame_owner
            .classifier_cache()
            .stagger_counter(),
    );
    let result = apply_native_type9_particle_hit(
        &mut manager,
        &mut fx,
        &mut scheduler,
        &mut notifications,
        hit(id, false, 0),
        6,
    );
    assert!(
        matches!(result, NativeType9ImpactOutcome::Applied(_)),
        "{result:?}"
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        (
            entity.position_raw(),
            entity.collision.callback_scheduler_accumulator_us_at_0x6c,
            entity
                .ordinary_type9_selected_component_runtime
                .unwrap()
                .components()
                .sub_d_frame_owner
                .classifier_cache()
                .stagger_counter()
        ),
        before
    );
    assert!(scheduler
        .begin_native_type9_external_mutation(&manager, id)
        .is_some());
}

#[v2k_test_support::retail_test]
fn native_type9_missing_custody_rejects_before_timestamp_or_rng() {
    let Some((_session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .unwrap()
        .id;
    let before = manager.entity_mut(id).unwrap().collision.clone();
    let mut fx = WorldFx::new();
    let mut control = WorldFx::new();
    let result = apply_native_type9_particle_hit(
        &mut manager,
        &mut fx,
        &mut SpecializedActorTaskScheduler::new(),
        &mut GameplayNotifications::new(),
        hit(id, true, 0),
        25,
    );
    assert!(matches!(
        result,
        NativeType9ImpactOutcome::Blocked {
            committed_prefix: false,
            ..
        }
    ));
    assert_eq!(manager.entity_mut(id).unwrap().collision, before);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        control.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type9_unavailable_initializer_input_retains_infected_prefix_and_old_graph() {
    let Some((_session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 9 && class(entity) == 10)
        .unwrap()
        .id;
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type9(&mut manager);
    let entity = manager.entity_mut(id).unwrap();
    let visits = super::task_visits(entity);
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Unresolved;
    entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
    let mut fx = WorldFx::new();
    let mut control = WorldFx::new();
    let result = apply_native_type9_particle_hit(
        &mut manager,
        &mut fx,
        &mut scheduler,
        &mut GameplayNotifications::new(),
        hit(id, true, 0),
        25,
    );
    assert!(
        matches!(
            result,
            NativeType9ImpactOutcome::Blocked {
                reason: NativeType9ImpactBlock::Runtime("common axis"),
                committed_prefix: true
            }
        ),
        "{result:?}"
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(super::task_visits(entity), visits);
    assert_eq!(
        entity.collision.active_model_slot(),
        RetailRuntimeValue::Known(2)
    );
    assert_eq!(
        entity.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(17)
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        control.next_shared_retail_random_u16()
    );
    assert_eq!(
        scheduler.family_for(id),
        Some(SpecializedActorTaskFamily::OrdinaryType9RunAway)
    );
}

#[v2k_test_support::retail_test]
fn native_type9_primary_nonzero_return_keeps_cue_when_buffered_or_negative() {
    for negative in [false, true] {
        let Some((_session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 9 && class(entity) == 10)
            .unwrap()
            .id;
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.adopt_intro2_type9(&mut manager);
        let entity = manager.entity_mut(id).unwrap();
        let before_health = entity.collision.health_raw;
        if negative {
            let RetailRuntimeValue::Known(mut profile) = entity.collision.damage_profile else {
                panic!()
            };
            profile.multipliers_q8[2] = -256;
            entity.collision.damage_profile = RetailRuntimeValue::Known(profile);
        } else {
            entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(1_000_000);
        }
        let mut fx = WorldFx::new();
        let result = apply_native_type9_particle_hit(
            &mut manager,
            &mut fx,
            &mut scheduler,
            &mut GameplayNotifications::new(),
            hit(id, false, 1_000),
            25,
        );
        let NativeType9ImpactOutcome::Applied(result) = result else {
            panic!("{result:?}")
        };
        if negative {
            assert!(result.filtered_damage_raw < 0);
            assert!(result.damage_after_buffer_raw < 0);
        } else {
            assert!(result.filtered_damage_raw > 0);
            assert_eq!(result.damage_after_buffer_raw, 0);
            assert_eq!(
                manager.entity_mut(id).unwrap().collision.health_raw,
                before_health
            );
        }
        assert!(result.death_publication.is_none());
        fx.process_pending();
        assert_eq!(
            fx.take_positional_sounds()
                .iter()
                .filter(|sound| sound.sound_id == 95)
                .count(),
            1,
            "10EB0 tests the signed nonzero filtered return, not post-buffer health loss"
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type9_actual_acquisition_then_primary_and_infected_hits_use_fleeing_style() {
    use crate::ordinary_type9_live::OrdinaryType9SelectedRuntimeKind;
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 9 && class(entity) == 10)
        .unwrap()
        .id;
    manager.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(1_000_000);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type9(&mut manager);
    let mut fx = WorldFx::new();
    let mut notifications = GameplayNotifications::new();
    let mut static_damage = StaticDamageScheduler::new();
    let mut acquired = false;
    for tick in 1..=100 {
        let pass = scheduler.tick(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: tick,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        assert_eq!(pass.block, None);
        let entity = manager.entity_mut(id).unwrap();
        if entity
            .ordinary_type9_selected_component_runtime
            .is_some_and(|selected| {
                selected.kind() == OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished
            })
        {
            acquired = true;
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                panic!()
            };
            assert_eq!(context.active_style().style_address(), 0x004c_7660);
            break;
        }
    }
    assert!(
        acquired,
        "the authored Run Away graph must acquire a real baddie"
    );
    let visits = super::task_visits(manager.entity_mut(id).unwrap());
    let primary = apply_native_type9_particle_hit(
        &mut manager,
        &mut fx,
        &mut scheduler,
        &mut notifications,
        hit(id, false, 0),
        101,
    );
    assert!(
        matches!(primary, NativeType9ImpactOutcome::Applied(_)),
        "{primary:?}"
    );
    assert_eq!(super::task_visits(manager.entity_mut(id).unwrap()), visits);
    let infected = apply_native_type9_particle_hit(
        &mut manager,
        &mut fx,
        &mut scheduler,
        &mut notifications,
        hit(id, true, 0),
        102,
    );
    assert!(
        matches!(infected, NativeType9ImpactOutcome::Applied(_)),
        "{infected:?}"
    );
    assert_ne!(super::task_visits(manager.entity_mut(id).unwrap()), visits);
    assert!(scheduler
        .begin_native_type9_external_mutation(&manager, id)
        .is_some());
}
