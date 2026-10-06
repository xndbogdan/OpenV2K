//! Native Type13 incoming hit custody, C690 reselection, impulse, and checked damage.
//!
//! The second half covers the `FUN_00442950` static route (`FUN_00441180`):
//! pre-callback sound, DAC0 dispatch, eightfold reaction force, result-blind
//! checked damage, and the fail-closed attached-class boundary.

use super::*;
use crate::{
    damage::{DamagePacket, CLASS68_STATIC_ROUTE_DAMAGE_PACKET},
    entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT},
    intro2_type13_live::{
        impact::{
            apply_intro2_type13_particle_hit, apply_intro2_type13_static_hit,
            Intro2Type13ImpactOutcome, Intro2Type13StaticBlock, Intro2Type13StaticOutcome,
        },
        INTRO2_TYPE13_SPAWN_INDEX, TYPE13_ENTITY_TYPE,
    },
    intro2_type47_live::world::native_intro2_fixture,
    world_fx::{attached_impact_offset_raw, BallisticDamageRequest, ParticleEntityImpact, WorldFx},
};

/// Class68 impact shaped like Type57 bat fire: the descriptor-selected
/// `0x004CC108` packet with Type57 birth provenance.
fn class68_impact(target_entity_id: u32, amount_scale: i32) -> ParticleEntityImpact {
    let packet = DamagePacket {
        channels: CLASS68_STATIC_ROUTE_DAMAGE_PACKET.channels,
        amounts_raw: [
            CLASS68_STATIC_ROUTE_DAMAGE_PACKET.amounts_raw[0] * amount_scale,
            CLASS68_STATIC_ROUTE_DAMAGE_PACKET.amounts_raw[1] * amount_scale,
        ],
    };
    ParticleEntityImpact {
        source_particle_class: 68,
        impact_position_argument_va: 0,
        target_entity_id,
        position_world: [0.0, 1000.0, 0.0],
        velocity_raw: [254, -471, 508],
        damage: Some(BallisticDamageRequest {
            packet,
            source_entity_type_at_birth: Some(57),
            source_owner_id: Some(53),
        }),
    }
}

#[v2k_test_support::retail_test]
fn type13_tick1473_incoming_hit_applies_c690_impulse_and_checked_damage() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_type13_search_attack(&manager), 1);
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
        .expect("Intro2 spawn 0 is Type13")
        .id;

    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert_eq!(entity.entity_type, TYPE13_ENTITY_TYPE);
    let initial_health = match entity.collision.health_raw {
        RetailRuntimeValue::Known(h) => h,
        _ => panic!("Type13 initial health must be known"),
    };
    let initial_velocity = entity.velocity_raw();

    let mut fx = WorldFx::new();
    // Fragment at tick 1473: class 30, channel 1, amount 2500, raw velocity [254, -471, 508]
    let impact = ParticleEntityImpact {
        source_particle_class: 30,
        impact_position_argument_va: 0,
        target_entity_id: id,
        position_world: [0.0, 1000.0, 0.0],
        velocity_raw: [254, -471, 508],
        damage: Some(BallisticDamageRequest {
            packet: DamagePacket {
                channels: [1, 0],
                amounts_raw: [2500, 0],
            },
            source_entity_type_at_birth: Some(102),
            source_owner_id: Some(53),
        }),
    };

    let result = apply_intro2_type13_particle_hit(
        crate::intro2_type13_live::impact::Intro2Type13ImpactFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            scheduler: &mut scheduler,
            static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            retail_tick: 1473,
        },
        impact,
    );

    let outcome = match result {
        Intro2Type13ImpactOutcome::Applied(outcome) => outcome,
        other => panic!("expected applied hit outcome: {other:?}"),
    };
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert_eq!(outcome.death_publication, None);
    // Type 13 authored damage profile has a 10,000 threshold on channel 1 (multipliers_q8: 256),
    // so the 2,500 raw amount fragment at tick 1473 is absorbed without health damage.
    assert_eq!(outcome.filtered_damage_raw, 0);
    assert_eq!(
        entity.collision.health_raw,
        RetailRuntimeValue::Known(initial_health)
    );
    assert_eq!(
        entity.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(1473)
    );
    assert_ne!(entity.velocity_raw(), initial_velocity);
    assert_eq!(
        entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
        RetailRuntimeValue::Known(0)
    );
    assert!(scheduler.intro2_type13_completed_owner(&manager, id));
}

#[v2k_test_support::retail_test]
fn type13_above_threshold_hit_reduces_health() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_type13_search_attack(&manager), 1);
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
        .expect("Intro2 spawn 0 is Type13")
        .id;

    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    let initial_health = match entity.collision.health_raw {
        RetailRuntimeValue::Known(h) => h,
        _ => panic!("Type13 initial health must be known"),
    };

    let mut fx = WorldFx::new();
    // Hit with 15,000 on channel 1 (threshold is 10,000, Q8 multiplier is 256 -> 5,000 filtered damage)
    let impact = ParticleEntityImpact {
        source_particle_class: 30,
        impact_position_argument_va: 0,
        target_entity_id: id,
        position_world: [0.0, 1000.0, 0.0],
        velocity_raw: [254, -471, 508],
        damage: Some(BallisticDamageRequest {
            packet: DamagePacket {
                channels: [1, 0],
                amounts_raw: [15_000, 0],
            },
            source_entity_type_at_birth: Some(102),
            source_owner_id: Some(53),
        }),
    };

    let result = apply_intro2_type13_particle_hit(
        crate::intro2_type13_live::impact::Intro2Type13ImpactFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            scheduler: &mut scheduler,
            static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            retail_tick: 1473,
        },
        impact,
    );

    let outcome = match result {
        Intro2Type13ImpactOutcome::Applied(outcome) => outcome,
        other => panic!("expected applied hit outcome: {other:?}"),
    };
    assert_eq!(outcome.death_publication, None);
    assert_eq!(outcome.filtered_damage_raw, 5000);

    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert_eq!(
        entity.collision.health_raw,
        RetailRuntimeValue::Known(initial_health - 5000)
    );
}

#[v2k_test_support::retail_test]
fn type13_hit_not_applicable_for_other_entities() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut scheduler = SpecializedActorTaskScheduler::new();
    let id_other = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(1))
        .expect("Intro2 spawn 1 exists")
        .id;

    let mut fx = WorldFx::new();
    let impact = ParticleEntityImpact {
        source_particle_class: 30,
        impact_position_argument_va: 0,
        target_entity_id: id_other,
        position_world: [0.0, 1000.0, 0.0],
        velocity_raw: [254, -471, 508],
        damage: Some(BallisticDamageRequest {
            packet: DamagePacket {
                channels: [1, 0],
                amounts_raw: [2500, 0],
            },
            source_entity_type_at_birth: Some(102),
            source_owner_id: Some(53),
        }),
    };

    let result = apply_intro2_type13_particle_hit(
        crate::intro2_type13_live::impact::Intro2Type13ImpactFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            scheduler: &mut scheduler,
            static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            retail_tick: 1473,
        },
        impact,
    );

    assert_eq!(result, Intro2Type13ImpactOutcome::NotApplicable);
}

#[v2k_test_support::retail_test]
fn type13_lethal_hit_finishes_authored_class1_synchronously() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_type13_search_attack(&manager), 1);
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
        .expect("Intro2 spawn 0 is Type13")
        .id;

    // Other real allocations retain their native construction but are not
    // eligible for this source-only terminal regression's radial scan.
    let ids: Vec<_> = manager.retail_live_order_ids().collect();
    for actor in ids {
        if actor != id {
            manager
                .entity_mut(actor)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(0x8000, 0);
        }
    }
    manager
        .entity_mut(id)
        .unwrap()
        .set_position_raw([100, 10_000, -300]);
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
    let mut fx = WorldFx::new();
    for _ in 0..8 {
        fx.advance_frame_pacing(20_000);
    }
    // Fatal damage (e.g. 100,000)
    let impact = ParticleEntityImpact {
        source_particle_class: 30,
        impact_position_argument_va: 0,
        target_entity_id: id,
        position_world: [0.0, 1000.0, 0.0],
        velocity_raw: [0, 0, 8192],
        damage: Some(BallisticDamageRequest {
            packet: DamagePacket {
                channels: [1, 0],
                amounts_raw: [100_000, 0],
            },
            source_entity_type_at_birth: Some(102),
            source_owner_id: Some(53),
        }),
    };

    let result = apply_intro2_type13_particle_hit(
        crate::intro2_type13_live::impact::Intro2Type13ImpactFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            scheduler: &mut scheduler,
            static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            retail_tick: 1473,
        },
        impact,
    );

    match result {
        Intro2Type13ImpactOutcome::Applied(outcome) => {
            assert!(outcome.filtered_damage_raw > 0);
            assert!(outcome.death_publication.is_none());
        }
        other => panic!("expected synchronous native Class1 death: {other:?}"),
    }
    assert!(crate::class49_death::finished_terminal_hit_authenticates(
        &manager, id
    ));
    assert!(scheduler.family_for(id).is_none());
    assert_eq!(manager.pending_actor_deferred_destroy_ids(), &[id]);
    assert!(!manager.iter_all().any(|entity| entity.entity_type == 60));
    assert_eq!(
        fx.test_particles_in_virgin_birth_order()
            .iter()
            .filter(|particle| particle.source_class == 37
                && particle.source_entity_type_at_birth == Some(13))
            .count(),
        10
    );
    // Resolve the active global Type13 record through the resource cache;
    // local Section12 row13 belongs to global Type15, not this allocation.
    let mut expected_sounds: Vec<_> = session
        .cache
        .global_entity_type(TYPE13_ENTITY_TYPE as usize)
        .unwrap()
        .death_sound_id()
        .into_iter()
        .map(usize::from)
        .collect();
    expected_sounds.push(62);
    assert_eq!(
        fx.take_positional_sounds()
            .into_iter()
            .map(|sound| sound.sound_id)
            .collect::<Vec<_>>(),
        expected_sounds,
        "active authored+90, when present, precedes440950's cue62 synchronously",
    );
}

#[v2k_test_support::retail_test]
fn type13_class68_static_hit_applies_411180_prefix_before_traversal_effects() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_type13_search_attack(&manager), 1);
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
        .expect("Intro2 spawn 0 is Type13")
        .id;

    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert_eq!(entity.entity_type, TYPE13_ENTITY_TYPE);
    let initial_health = match entity.collision.health_raw {
        RetailRuntimeValue::Known(h) => h,
        _ => panic!("Type13 initial health must be known"),
    };
    let initial_velocity = entity.velocity_raw();

    let mut fx = WorldFx::new();
    let result = apply_intro2_type13_static_hit(
        crate::intro2_type13_live::impact::Intro2Type13ImpactFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            scheduler: &mut scheduler,
            static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            retail_tick: 1473,
        },
        class68_impact(id, 1),
    );

    let applied = match result {
        Intro2Type13StaticOutcome::Applied(applied) => applied,
        other => panic!("expected applied static hit outcome: {other:?}"),
    };
    assert_eq!(
        fx.particle_count(),
        0,
        "the traversal emits F610 and attached84 after damage"
    );
    assert_eq!(applied.checked.death_publication, None);
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert_eq!(
        entity.collision.health_raw,
        RetailRuntimeValue::Known(initial_health - applied.checked.filtered_damage_raw)
    );
    assert_eq!(
        entity.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(1473)
    );
    assert_ne!(entity.velocity_raw(), initial_velocity);
    assert!(scheduler.intro2_type13_completed_owner(&manager, id));
}

#[test]
fn type13_attached_offset_words_match_retail_truncation() {
    // Plain impact-minus-target differences without capability gating.
    assert_eq!(
        attached_impact_offset_raw([100, 200, 300], [10, 20, 30], 0),
        [90, 180, 270]
    );
    // Capability-1: zero Y, trunc(3v/4) XZ with signed wrapping.
    assert_eq!(
        attached_impact_offset_raw([100, 200, 300], [10, 20, 30], 1),
        [67, 0, 202]
    );
    assert_eq!(
        attached_impact_offset_raw([0, 0, 0], [9, 9, 9], 1),
        [-6, 0, -6]
    );
    // Wrapping subtraction reproduces the retail short arithmetic.
    assert_eq!(
        attached_impact_offset_raw([i16::MIN, 0, i16::MAX], [1, 0, -1], 0),
        [i16::MAX, 0, i16::MIN]
    );
}

#[v2k_test_support::retail_test]
fn type13_static_hit_uses_eightfold_reaction_force_not_primary_force() {
    // Identical fixtures through both entries: shared-RNG draws stay aligned
    // through C690, so any velocity difference is the 411180 eightfold force.
    let run_primary_velocity = |impact| {
        let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
            return None;
        };
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_intro2_type13_search_attack(&manager), 1);
        let id = manager
            .iter_all()
            .find(|e| e.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
            .expect("Intro2 spawn 0 is Type13")
            .id;
        let mut fx = WorldFx::new();
        match apply_intro2_type13_particle_hit(
            crate::intro2_type13_live::impact::Intro2Type13ImpactFrame {
                entities: &mut manager,
                resources: &mut session.cache,
                world_fx: &mut fx,
                scheduler: &mut scheduler,
                static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                retail_tick: 1473,
            },
            impact,
        ) {
            Intro2Type13ImpactOutcome::Applied(_) => Some(
                manager
                    .iter_all()
                    .find(|e| e.id == id)
                    .unwrap()
                    .velocity_raw(),
            ),
            _ => None,
        }
    };
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_type13_search_attack(&manager), 1);
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
        .expect("Intro2 spawn 0 is Type13")
        .id;
    let mut fx = WorldFx::new();
    let static_result = apply_intro2_type13_static_hit(
        crate::intro2_type13_live::impact::Intro2Type13ImpactFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            scheduler: &mut scheduler,
            static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            retail_tick: 1473,
        },
        class68_impact(id, 1),
    );
    assert!(matches!(
        static_result,
        Intro2Type13StaticOutcome::Applied(_)
    ));
    let static_velocity = manager
        .iter_all()
        .find(|e| e.id == id)
        .unwrap()
        .velocity_raw();
    let primary_velocity = run_primary_velocity(class68_impact(id, 1))
        .expect("primary entry still accepts the class68 impact shape");
    assert_ne!(
        static_velocity, primary_velocity,
        "411180 must strike 11030 with 8x the 25590 sum, not the primary force"
    );
}

#[v2k_test_support::retail_test]
fn type13_static_hit_missing_packet_has_no_delivery() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_type13_search_attack(&manager), 1);
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
        .expect("Intro2 spawn 0 is Type13")
        .id;

    let mut impact = class68_impact(id, 1);
    impact.damage = None;
    let mut fx = WorldFx::new();
    assert_eq!(
        apply_intro2_type13_static_hit(
            crate::intro2_type13_live::impact::Intro2Type13ImpactFrame {
                entities: &mut manager,
                resources: &mut session.cache,
                world_fx: &mut fx,
                scheduler: &mut scheduler,
                static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                retail_tick: 1473,
            },
            impact,
        ),
        Intro2Type13StaticOutcome::Blocked {
            reason: Intro2Type13StaticBlock::MissingPacket,
            committed_prefix: false,
        }
    );
}

#[v2k_test_support::retail_test]
fn type13_static_hit_not_applicable_for_other_sources() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_type13_search_attack(&manager), 1);
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
        .expect("Intro2 spawn 0 is Type13")
        .id;

    // Class30 stays on the primary entry; the static route must not claim it.
    let mut impact = class68_impact(id, 1);
    impact.source_particle_class = 30;
    let mut fx = WorldFx::new();
    assert_eq!(
        apply_intro2_type13_static_hit(
            crate::intro2_type13_live::impact::Intro2Type13ImpactFrame {
                entities: &mut manager,
                resources: &mut session.cache,
                world_fx: &mut fx,
                scheduler: &mut scheduler,
                static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                retail_tick: 1473,
            },
            impact,
        ),
        Intro2Type13StaticOutcome::NotApplicable
    );
}

#[v2k_test_support::retail_test]
fn type13_static_hit_unsupported_target_fails_closed() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut scheduler = SpecializedActorTaskScheduler::new();
    let other = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(1))
        .expect("Intro2 spawn 1 exists");
    let (other_id, other_type) = (other.id, other.entity_type);

    let mut fx = WorldFx::new();
    assert_eq!(
        apply_intro2_type13_static_hit(
            crate::intro2_type13_live::impact::Intro2Type13ImpactFrame {
                entities: &mut manager,
                resources: &mut session.cache,
                world_fx: &mut fx,
                scheduler: &mut scheduler,
                static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                retail_tick: 1473,
            },
            class68_impact(other_id, 1),
        ),
        Intro2Type13StaticOutcome::Blocked {
            reason: Intro2Type13StaticBlock::UnsupportedStaticRouteTarget {
                entity_type: Some(other_type)
            },
            committed_prefix: false,
        }
    );
}

#[v2k_test_support::retail_test]
fn type13_static_lethal_hit_finishes_authored_class1_synchronously() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_type13_search_attack(&manager), 1);
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
        .expect("Intro2 spawn 0 is Type13")
        .id;

    // Other real allocations retain their native construction but are not
    // eligible for this source-only terminal regression's radial scan.
    let ids: Vec<_> = manager.retail_live_order_ids().collect();
    for actor in ids {
        if actor != id {
            manager
                .entity_mut(actor)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(0x8000, 0);
        }
    }
    manager
        .entity_mut(id)
        .unwrap()
        .set_position_raw([100, 10_000, -300]);
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
    let mut fx = WorldFx::new();
    for _ in 0..8 {
        fx.advance_frame_pacing(20_000);
    }
    match apply_intro2_type13_static_hit(
        crate::intro2_type13_live::impact::Intro2Type13ImpactFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            scheduler: &mut scheduler,
            static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            retail_tick: 1473,
        },
        class68_impact(id, 50),
    ) {
        Intro2Type13StaticOutcome::Applied(applied) => {
            assert!(applied.checked.filtered_damage_raw > 0);
            assert!(applied.checked.death_publication.is_none());
        }
        other => panic!("expected synchronous native Class1 death: {other:?}"),
    }
    assert!(crate::class49_death::finished_terminal_hit_authenticates(
        &manager, id
    ));
    assert!(scheduler.family_for(id).is_none());
    assert_eq!(manager.pending_actor_deferred_destroy_ids(), &[id]);
    assert!(!manager.iter_all().any(|entity| entity.entity_type == 60));
    assert_eq!(
        fx.test_particles_in_virgin_birth_order()
            .iter()
            .filter(|particle| particle.source_class == 37
                && particle.source_entity_type_at_birth == Some(13))
            .count(),
        10
    );
    // Resolve the active global Type13 record through the resource cache;
    // local Section12 row13 belongs to global Type15, not this allocation.
    let mut expected_sounds: Vec<_> = session
        .cache
        .global_entity_type(TYPE13_ENTITY_TYPE as usize)
        .unwrap()
        .death_sound_id()
        .into_iter()
        .map(usize::from)
        .collect();
    expected_sounds.push(62);
    assert_eq!(
        fx.take_positional_sounds()
            .into_iter()
            .map(|sound| sound.sound_id)
            .collect::<Vec<_>>(),
        expected_sounds,
        "active authored+90, when present, precedes440950's cue62 synchronously",
    );
}
