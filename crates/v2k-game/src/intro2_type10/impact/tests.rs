use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    damage::{DamagePacket, FUN_0043F780_DAMAGE_DELIVERY},
    entity_behavior::audited_behavior_style,
    intro2_type47_live::world::native_intro2_fixture,
    world_fx::BallisticDamageRequest,
};

fn impact(id: u32, infected: bool, packet: DamagePacket) -> ParticleEntityImpact {
    ParticleEntityImpact {
        source_particle_class: if infected { 5 } else { 16 },
        impact_position_argument_va: if infected { 0x004dcf48 } else { 0 },
        target_entity_id: id,
        position_world: [0.; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(BallisticDamageRequest {
            packet: if infected {
                FUN_0043F780_DAMAGE_DELIVERY.packet
            } else {
                packet
            },
            source_entity_type_at_birth: Some(34),
            source_owner_id: Some(35),
        }),
    }
}
fn setup(manager: &mut EntityManager, spawn: usize) -> (u32, SpecializedActorTaskScheduler) {
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(spawn))
        .unwrap()
        .id;
    let entity = manager.entity_mut(id).unwrap();
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x0606_8000);
    entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
    entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.register_intro2_type10(Intro2Type10Owner::adopt(manager, id).unwrap());
    (id, scheduler)
}

#[test]
fn native_type10_hit_authenticates_distinct_primary_and_infected_style_words() {
    for (class, variant, expected) in [
        (7, 0, ImpactCallbackPolicy::ReselectBehavior),
        (7, 1, ImpactCallbackPolicy::ReselectBehavior),
        (7, 2, ImpactCallbackPolicy::None),
        (11, 0, ImpactCallbackPolicy::None),
        (11, 1, ImpactCallbackPolicy::None),
    ] {
        let style = ActiveBehaviorStyle::Audited(*audited_behavior_style(class, variant).unwrap());
        assert_eq!(callback_policy(style, HitEntry::Primary), Some(expected));
        assert_eq!(callback_policy(style, HitEntry::Infected), Some(expected));
    }
    for entry in [HitEntry::Primary, HitEntry::Infected] {
        assert_eq!(
            callback_policy(ActiveBehaviorStyle::InitializerFailureFallback, entry),
            Some(ImpactCallbackPolicy::None)
        );
        assert_eq!(
            callback_policy(
                ActiveBehaviorStyle::Audited(*audited_behavior_style(12, 0).unwrap()),
                entry
            ),
            None
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type10_both_births_reselect_on_every_primary_or_infected_hit_in_source_order() {
    for spawn in [55, 56] {
        let Some((session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let (id, mut scheduler) = setup(&mut manager, spawn);
        let mut fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        for (tick, infected) in [(251, false), (252, true), (253, false), (254, true)] {
            let entity = manager.entity_mut(id).unwrap();
            let before = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
            let before_stamp = entity.collision.last_hit_presentation_tick_at_0x34;
            let before_health = entity.collision.health_raw;
            let particles = fx.particle_count();
            let result = apply_intro2_type10_particle_hit(
                &mut manager,
                &session.cache,
                &mut fx,
                &mut scheduler,
                impact(
                    id,
                    infected,
                    DamagePacket {
                        channels: [1, 0],
                        amounts_raw: [10000, 0],
                    },
                ),
                tick,
            );
            let Intro2Type10ImpactOutcome::Applied(result) = result else {
                panic!("{result:?}")
            };
            assert_eq!(result.filtered_damage_raw, if infected { 0 } else { 1000 });
            assert_eq!(result.death_publication, None);
            let entity = manager.entity_mut(id).unwrap();
            assert_ne!(
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                before
            );
            assert_eq!(
                entity.collision.last_hit_presentation_tick_at_0x34,
                if infected {
                    before_stamp
                } else {
                    RetailRuntimeValue::Known(tick)
                }
            );
            assert_eq!(
                entity.collision.health_raw,
                before_health.map(|health| health - if infected { 0 } else { 1000 })
            );
            assert_eq!(fx.particle_count() > particles, !infected);
            // Singleton selector, two successful6030 initializers, then
            //11030's three Euler words even for the filtered-zero F780 hit.
            for _ in 0..6 {
                oracle.next_shared_retail_random_u16();
            }
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                oracle.next_shared_retail_random_u16()
            );
            assert!(Intro2Type10Owner::adopt(&manager, id).is_ok());
        }
        assert_eq!(
            fx.pending_event_count(),
            0,
            "Type10 has no hit/infected cue"
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type10_nonzero_filtered_suffix_survives_buffer_negative_and_lethal_results() {
    for (packet, buffer, filtered, after_health, death) in [
        (
            DamagePacket {
                channels: [1, 0],
                amounts_raw: [10000, 0],
            },
            1000,
            1000,
            32000,
            false,
        ),
        (
            DamagePacket {
                channels: [5, 0],
                amounts_raw: [4194304, 0],
            },
            0,
            -8388608,
            8420608,
            false,
        ),
        (
            DamagePacket {
                channels: [1, 0],
                amounts_raw: [50000, 0],
            },
            0,
            41000,
            0,
            true,
        ),
    ] {
        let Some((session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let (id, mut scheduler) = setup(&mut manager, 55);
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .pre_health_damage_buffer_raw = RetailRuntimeValue::Known(buffer);
        let mut fx = WorldFx::new();
        let result = apply_intro2_type10_particle_hit(
            &mut manager,
            &session.cache,
            &mut fx,
            &mut scheduler,
            impact(id, false, packet),
            444,
        );
        let Intro2Type10ImpactOutcome::Applied(result) = result else {
            panic!("{result:?}")
        };
        assert_eq!(result.filtered_damage_raw, filtered);
        assert_eq!(result.death_publication.is_some(), death);
        assert_eq!(
            manager.entity_mut(id).unwrap().collision.health_raw,
            RetailRuntimeValue::Known(after_health)
        );
        assert!(
            fx.particle_count() > 0,
            "primary capability8 suffix is independent of surviving health and positive sign"
        );
        assert_eq!(fx.pending_event_count(), 0);
        if death {
            let entity = manager.entity_mut(id).unwrap();
            assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::TumbleOutOfSky(_))
            ));
            let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
            let again = apply_intro2_type10_particle_hit(
                &mut manager,
                &session.cache,
                &mut fx,
                &mut scheduler,
                impact(
                    id,
                    false,
                    DamagePacket {
                        channels: [1, 0],
                        amounts_raw: [10000, 0],
                    },
                ),
                445,
            );
            assert!(
                matches!(again, Intro2Type10ImpactOutcome::Applied(_)),
                "{again:?}"
            );
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                primary
            );
            assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type10_hit_unresolved_initializer_retains_stamp_and_blocks_without_rng_replay() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let (id, mut scheduler) = setup(&mut manager, 56);
    manager.entity_mut(id).unwrap().actor_common_axis_descriptor = RetailRuntimeValue::Unresolved;
    let mut fx = WorldFx::new();
    let mut oracle = WorldFx::new();
    let hit = impact(
        id,
        false,
        DamagePacket {
            channels: [1, 0],
            amounts_raw: [10000, 0],
        },
    );
    let result = apply_intro2_type10_particle_hit(
        &mut manager,
        &session.cache,
        &mut fx,
        &mut scheduler,
        hit,
        5,
    );
    assert!(
        matches!(
            result,
            Intro2Type10ImpactOutcome::Blocked {
                committed_prefix: true,
                ..
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
        RetailRuntimeValue::Known(5)
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
    assert!(scheduler.intro2_type10_has_pending_prefix(id));
    let second = apply_intro2_type10_particle_hit(
        &mut manager,
        &session.cache,
        &mut fx,
        &mut scheduler,
        hit,
        6,
    );
    assert!(matches!(
        second,
        Intro2Type10ImpactOutcome::Blocked {
            reason: Intro2Type10ImpactBlock::Runtime("pending actor prefix"),
            committed_prefix: false
        }
    ));
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(5)
    );
}

#[v2k_test_support::retail_test]
fn native_type10_hits_require_current_living_or_tumble_custody_before_any_prefix() {
    for dying in [false, true] {
        for stale in [false, true] {
            for infected in [false, true] {
                let Some((session, mut manager, _)) = native_intro2_fixture() else {
                    return;
                };
                let (id, mut scheduler) = setup(&mut manager, 55);
                let mut fx = WorldFx::new();
                if dying {
                    let owner = publish_intro2_type10_standard_death(&mut manager, id, &mut fx)
                        .unwrap()
                        .unwrap();
                    scheduler.register_intro2_type10_tumble(owner);
                }
                if stale {
                    // Replacing the actual wrapper invalidates retained custody even
                    // if the new task has equal private data and the same style.
                    let entity = manager.entity_mut(id).unwrap();
                    let state = entity
                        .actor_task_state(ActorTaskSlot::Primary)
                        .unwrap()
                        .clone();
                    entity.actor_tasks.replace_prepared(
                        ActorTaskSlot::Primary,
                        crate::actor_task_owner::PreparedActorTask::new(state),
                    );
                } else {
                    scheduler = SpecializedActorTaskScheduler::new();
                }
                let entity = manager.entity_mut(id).unwrap();
                let before = (
                    entity.position_raw(),
                    entity.velocity_raw(),
                    entity.rotation_heading_pitch_roll_raw(),
                    entity.collision.health_raw,
                    entity.collision.last_hit_presentation_tick_at_0x34,
                    entity.collision.state_flags_at_0x08,
                    entity.current_behavior_context,
                );
                let mut fx = WorldFx::new();
                let mut oracle = WorldFx::new();
                let result = apply_intro2_type10_particle_hit(
                    &mut manager,
                    &session.cache,
                    &mut fx,
                    &mut scheduler,
                    impact(
                        id,
                        infected,
                        DamagePacket {
                            channels: [1, 0],
                            amounts_raw: [10000, 0],
                        },
                    ),
                    777,
                );
                assert!(
                    matches!(
                        result,
                        Intro2Type10ImpactOutcome::Blocked {
                            reason: Intro2Type10ImpactBlock::Runtime("completed actor custody"),
                            committed_prefix: false,
                        }
                    ),
                    "{result:?}"
                );
                let entity = manager.entity_mut(id).unwrap();
                assert_eq!(
                    (
                        entity.position_raw(),
                        entity.velocity_raw(),
                        entity.rotation_heading_pitch_roll_raw(),
                        entity.collision.health_raw,
                        entity.collision.last_hit_presentation_tick_at_0x34,
                        entity.collision.state_flags_at_0x08,
                        entity.current_behavior_context
                    ),
                    before
                );
                assert_eq!(fx.particle_count(), 0);
                assert_eq!(fx.pending_event_count(), 0);
                assert_eq!(
                    fx.next_shared_retail_random_u16(),
                    oracle.next_shared_retail_random_u16()
                );
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type10_late_hit_failure_parks_the_committed_reaction_for_both_entries() {
    for dying in [false, true] {
        for infected in [false, true] {
            let Some((session, mut manager, _)) = native_intro2_fixture() else {
                return;
            };
            let (id, mut scheduler) = setup(&mut manager, 56);
            let mut fx = WorldFx::new();
            if dying {
                let owner = publish_intro2_type10_standard_death(&mut manager, id, &mut fx)
                    .unwrap()
                    .unwrap();
                scheduler.register_intro2_type10_tumble(owner);
            }
            manager
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(
                    IMPACT_REACTION_NETWORKED_STATE_BIT,
                    IMPACT_REACTION_NETWORKED_STATE_BIT,
                );
            let health = manager.entity_mut(id).unwrap().collision.health_raw;
            let mut fx = WorldFx::new();
            let mut oracle = WorldFx::new();
            let hit = impact(
                id,
                infected,
                DamagePacket {
                    channels: [1, 0],
                    amounts_raw: [10000, 0],
                },
            );
            let result = apply_intro2_type10_particle_hit(
                &mut manager,
                &session.cache,
                &mut fx,
                &mut scheduler,
                hit,
                901,
            );
            assert!(
                matches!(
                    result,
                    Intro2Type10ImpactOutcome::Blocked {
                        reason: Intro2Type10ImpactBlock::Runtime("network impact"),
                        committed_prefix: true,
                    }
                ),
                "{result:?}"
            );
            assert!(scheduler.intro2_type10_has_pending_prefix(id));
            assert_eq!(manager.entity_mut(id).unwrap().collision.health_raw, health);
            // Living C690 consumes three initializer/selection words; every
            // admitted11030 reaction consumes its three Euler words before the
            // late network field is consumed. Tumble's hit slots are null.
            for _ in 0..if dying { 3 } else { 6 } {
                oracle.next_shared_retail_random_u16();
            }
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                oracle.next_shared_retail_random_u16()
            );
            let entity = manager.entity_mut(id).unwrap();
            let prefix = (
                entity.velocity_raw(),
                entity.rotation_heading_pitch_roll_raw(),
                entity.collision.last_hit_presentation_tick_at_0x34,
                entity.current_behavior_context,
            );
            let again = apply_intro2_type10_particle_hit(
                &mut manager,
                &session.cache,
                &mut fx,
                &mut scheduler,
                hit,
                902,
            );
            assert!(
                matches!(
                    again,
                    Intro2Type10ImpactOutcome::Blocked {
                        reason: Intro2Type10ImpactBlock::Runtime("pending actor prefix"),
                        committed_prefix: false,
                    }
                ),
                "{again:?}"
            );
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(
                (
                    entity.velocity_raw(),
                    entity.rotation_heading_pitch_roll_raw(),
                    entity.collision.last_hit_presentation_tick_at_0x34,
                    entity.current_behavior_context
                ),
                prefix
            );
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                oracle.next_shared_retail_random_u16()
            );
        }
    }
}
