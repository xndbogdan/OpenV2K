//! Actual carried allocations distinguish DAC0+28 from DA00+20.
use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    damage::{DamagePacket, FUN_0043F780_DAMAGE_DELIVERY},
    native_actor_capture::tests::Fixture,
    shared_actor_impact::{
        apply_shared_actor_particle_hit, SharedActorImpactFrame, SharedActorImpactOutcome,
    },
    specialized_actor_task_production::SpecializedActorTaskFamily,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact},
};

fn hit(f: &mut Fixture, infected: bool, amount: i32) -> SharedActorImpactOutcome {
    apply_shared_actor_particle_hit(
        SharedActorImpactFrame {
            resources: &f._session.cache,
            entities: &mut f.manager,
            world_fx: &mut f.fx,
            scheduler: &mut f.tasks,
            notifications: &mut f.notifications,
            retail_tick: 5000,
        },
        ParticleEntityImpact {
            source_particle_class: if infected { 5 } else { 16 },
            impact_position_argument_va: if infected { 0x004D_CF48 } else { 0 },
            target_entity_id: f.parent,
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
                source_entity_type_at_birth: Some(34),
                source_owner_id: Some(35),
            }),
        },
    )
    .unwrap()
}

#[v2k_test_support::retail_test]
fn type122_carry_primary_cleans_relation_but_infected_keeps_null_slot_and_task() {
    for variant in 2..=5 {
        for infected in [false, true] {
            let mut f = Fixture::new(8);
            f.attach().unwrap();
            f.carry(variant);
            let entity = f.manager.entity_mut(f.parent).unwrap();
            entity.collision.health_raw = RetailRuntimeValue::Known(1_000_000);
            entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
            let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
            let receipt = entity.native_type122_runtime;
            let outcome = hit(&mut f, infected, 6000);
            assert!(
                matches!(
                    outcome,
                    SharedActorImpactOutcome::Type122(impact::Type122ImpactOutcome::Applied(_))
                ),
                "{outcome:?}"
            );
            let entity = f.manager.entity_mut(f.parent).unwrap();
            assert_eq!(entity.native_type122_runtime, receipt);
            assert_eq!(
                entity.collision.last_hit_presentation_tick_at_0x34,
                RetailRuntimeValue::Known(if infected { 17 } else { 5000 })
            );
            assert_eq!(entity.native_capture_relation.is_some(), infected);
            assert_eq!(
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) == primary,
                infected
            );
            assert_eq!(
                f.manager.entity_mut(f.child).unwrap().attached_to,
                infected.then_some(f.parent)
            );
            assert!(Type122Owner::adopt(&f.manager, f.parent).is_ok());
            assert_eq!(
                f.tasks.family_for(f.parent),
                Some(SpecializedActorTaskFamily::NativeType122)
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn type122_lethal_primary_releases_child_but_authored_infected_immunity_keeps_it() {
    for infected in [false, true] {
        let mut f = Fixture::new(9);
        f.attach().unwrap();
        f.carry(2);
        f.manager.entity_mut(f.parent).unwrap().collision.health_raw = RetailRuntimeValue::Known(1);
        let outcome = hit(&mut f, infected, 12000);
        let SharedActorImpactOutcome::Type122(impact::Type122ImpactOutcome::Applied(applied)) =
            outcome
        else {
            panic!("{outcome:?}")
        };
        if infected {
            // Actual F780 channel6 has authored multiplier0 for Type122.
            // DA00+20 is null, and checked damage never enters lethal DB80.
            assert_eq!(applied.filtered_damage_raw, 0);
            let parent = f.manager.entity_mut(f.parent).unwrap();
            assert_eq!(parent.collision.health_raw, RetailRuntimeValue::Known(1));
            assert!(parent.native_capture_relation.is_some());
            assert_eq!(
                f.manager.entity_mut(f.child).unwrap().attached_to,
                Some(f.parent)
            );
            assert!(Type122Owner::adopt(&f.manager, f.parent).is_ok());
            continue;
        }
        assert_eq!(applied.filtered_damage_raw, 8000);
        assert_eq!(f.manager.entity_mut(f.child).unwrap().attached_to, None);
        assert!(f.manager.entity_mut(f.child).unwrap().active);
        let parent = f.manager.entity_mut(f.parent).unwrap();
        assert!(parent.native_capture_relation.is_none());
        assert_eq!(parent.collision.health_raw, RetailRuntimeValue::Known(0));
        assert!(
            crate::intro2_common_dying::Intro2CommonDyingOwner::adopt(&f.manager, f.parent).is_ok()
        );
        assert_eq!(
            f.tasks.family_for(f.parent),
            Some(SpecializedActorTaskFamily::Intro2CommonDying)
        );
    }
}

#[v2k_test_support::retail_test]
fn type122_parked_actor_rejects_both_hit_entries_before_prefix_or_relation_writes() {
    for infected in [false, true] {
        let mut f = Fixture::new(8);
        f.attach().unwrap();
        f.carry(2);
        let mut owner = Type122Owner::adopt(&f.manager, f.parent).unwrap();
        owner.park_external_prefix();
        f.tasks.register_type122(owner);
        let entity = f.manager.entity_mut(f.parent).unwrap();
        let before = (
            entity.collision.clone(),
            entity.current_behavior_context,
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            entity.native_capture_relation,
        );
        let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
        let outcome = hit(&mut f, infected, 12000);
        assert!(
            matches!(
                outcome,
                SharedActorImpactOutcome::Type122(impact::Type122ImpactOutcome::Blocked {
                    reason: impact::Type122ImpactBlock::Runtime(
                        "completed native allocation/task custody"
                    ),
                    committed_prefix: false,
                })
            ),
            "{outcome:?}"
        );
        let entity = f.manager.entity_mut(f.parent).unwrap();
        assert_eq!(
            (
                entity.collision.clone(),
                entity.current_behavior_context,
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                entity.native_capture_relation
            ),
            before
        );
        assert_eq!(
            f.manager.entity_mut(f.child).unwrap().attached_to,
            Some(f.parent)
        );
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected_fx.next_shared_retail_random_u16()
        );
    }
}
