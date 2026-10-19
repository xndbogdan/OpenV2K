//! Own Type122 damage profile and Class12 lifetime retain the native allocation.
use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    intro2_common_dying::{
        tick_intro2_common_dying, Intro2CommonDyingFrame, Intro2CommonDyingOutcome,
        Intro2CommonDyingOwner,
    },
    shared_actor_impact::{
        apply_shared_actor_particle_hit, SharedActorImpactFrame, SharedActorImpactOutcome,
    },
    specialized_actor_task_production::SpecializedActorTaskFamily,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact},
};

/// 442950's 11180 on a native Type122 differs from 10EB0 only where retail's
/// wrapper does: its living accepted-hit cue (92) precedes 15040, so a
/// filtered-zero packet still queues it; 11030 takes eight times the packet
/// sum; no class5 capability particle follows. Both stamp +34 and run C690.
#[v2k_test_support::retail_test]
fn static_route_hits_run_11180_on_native_type122() {
    let hit = |class: u8, packet: DamagePacket| {
        let mut f = super::contact_tests::Fixture::new(24);
        f.enable_contact();
        let entity = f.entities.entity_mut(f.id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(u32::MAX, 0x0406_8000);
        entity.set_motion_raw([0, 10_000, 0], [50, -80, 100]);
        let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
        let (events, particles) = (f.fx.pending_event_count(), f.fx.particle_count());
        let result = apply_shared_actor_particle_hit(
            SharedActorImpactFrame {
                resources: &f.session.cache,
                entities: &mut f.entities,
                world_fx: &mut f.fx,
                scheduler: &mut f.tasks,
                notifications: &mut f.notifications,
                retail_tick: 500,
            },
            ParticleEntityImpact {
                source_particle_class: class,
                impact_position_argument_va: 0,
                target_entity_id: f.id,
                position_world: [0.0; 3],
                velocity_raw: [0, 0, 8192],
                damage: Some(BallisticDamageRequest {
                    packet,
                    source_entity_type_at_birth: Some(122),
                    source_owner_id: Some(35),
                }),
            },
        );
        let Some(SharedActorImpactOutcome::Type122(impact::Type122ImpactOutcome::Applied(applied))) =
            result
        else {
            panic!("class{class}: {result:?}")
        };
        assert_eq!(
            f.entity().collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(500)
        );
        assert_ne!(
            f.entity().actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            primary
        );
        (
            applied.filtered_damage_raw,
            f.fx.pending_event_count() - events,
            f.fx.particle_count() - particles,
            f.entity().velocity_raw(),
        )
    };
    let damaging = DamagePacket {
        channels: [1, 0],
        amounts_raw: [6_000, 0],
    };
    // Type122's channel-6 multiplier is zero.
    let filtered_out = DamagePacket {
        channels: [6, 0],
        amounts_raw: [1_000, 0],
    };
    let primary = hit(16, damaging);
    assert_eq!((primary.0, primary.1, primary.2), (2000, 1, 1));
    assert_eq!(hit(16, filtered_out).1, 0);
    for class in [52, 68, 85] {
        let static_route = hit(class, damaging);
        assert_eq!(
            (static_route.0, static_route.1, static_route.2),
            (2000, 1, 0),
            "class{class}"
        );
        assert_ne!(static_route.3, primary.3, "class{class}: eightfold 11030");
        let filtered = hit(class, filtered_out);
        assert_eq!((filtered.0, filtered.1), (0, 1), "class{class}");
    }
}

#[v2k_test_support::retail_test]
fn native_primary_hits_use_type122_health_threshold_and_completed_replacement_custody() {
    for level in [24, 42, 46, 49, 50] {
        for lethal in [false, true] {
            let mut f = super::contact_tests::Fixture::new(level);
            f.enable_contact();
            // Keep the actor in the detailed local lane for the later dying
            // tick, independently of the map's activation/camera controller.
            let entity = f.entities.entity_mut(f.id).unwrap();
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(u32::MAX, 0x68000);
            entity.set_motion_raw([0, 10_000, 0], [50, -80, 100]);
            let allocation = entity.native_type122_runtime;
            let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
            let impact = ParticleEntityImpact {
                source_particle_class: 16,
                impact_position_argument_va: 0,
                target_entity_id: f.id,
                position_world: [0.0; 3],
                velocity_raw: [0, 0, 8192],
                damage: Some(BallisticDamageRequest {
                    packet: DamagePacket {
                        channels: [1, 0],
                        amounts_raw: [if lethal { 12_000 } else { 6_000 }, 0],
                    },
                    source_entity_type_at_birth: Some(34),
                    source_owner_id: Some(35),
                }),
            };
            let result = apply_shared_actor_particle_hit(
                SharedActorImpactFrame {
                    resources: &f.session.cache,
                    entities: &mut f.entities,
                    world_fx: &mut f.fx,
                    scheduler: &mut f.tasks,
                    notifications: &mut f.notifications,
                    retail_tick: 500,
                },
                impact,
            );
            let Some(SharedActorImpactOutcome::Type122(impact::Type122ImpactOutcome::Applied(
                applied,
            ))) = result
            else {
                panic!("world{level}: {result:?}")
            };
            assert_eq!(
                applied.filtered_damage_raw,
                if lethal { 8000 } else { 2000 }
            );
            assert_eq!(
                f.entity().collision.health_raw,
                RetailRuntimeValue::Known(if lethal { 0 } else { 5000 })
            );
            assert_eq!(
                f.entity().collision.last_hit_presentation_tick_at_0x34,
                RetailRuntimeValue::Known(500)
            );
            assert_eq!(f.entity().native_type122_runtime, allocation);
            assert_ne!(
                f.entity().actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                primary
            );
            if lethal {
                assert_eq!(
                    f.tasks.family_for(f.id),
                    Some(SpecializedActorTaskFamily::Intro2CommonDying)
                );
                let owner = Intro2CommonDyingOwner::adopt(&f.entities, f.id).unwrap();
                // C620 can restore the scheduler's random wait policy. Keep
                // this control on its first actual detailed Class12 visit.
                let no_wait = crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT;
                f.entities
                    .entity_mut(f.id)
                    .unwrap()
                    .collision
                    .state_flags_at_0x08
                    .overwrite(no_wait, no_wait);
                let tick = tick_intro2_common_dying(
                    &mut f.entities,
                    owner,
                    Intro2CommonDyingFrame {
                        resources: &f.session.cache,
                        world_fx: &mut f.fx,
                        elapsed_micros: 20_000,
                        retail_tick: 520,
                    },
                );
                assert!(
                    matches!(tick.outcome, Intro2CommonDyingOutcome::Advanced { .. }),
                    "{:#?}",
                    tick.outcome
                );
                assert!(tick.retained_owner.is_some());
                assert_eq!(
                    f.entity().collision.default_state_flags_at_0xc8,
                    RetailRuntimeValue::Known(0x439)
                );
            } else {
                assert!(Type122Owner::adopt(&f.entities, f.id).is_ok());
                assert_eq!(f.tasks.adopt_type122(&f.entities), 0);
            }
        }
    }
}
