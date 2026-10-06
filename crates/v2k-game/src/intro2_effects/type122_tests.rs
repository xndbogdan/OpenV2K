//! The cinematic host dispatches Type122 before its generic damage fallback.

use super::*;
use crate::{
    damage::{DamagePacket, FUN_0043F780_DAMAGE_PACKET},
    native_type122::{contact_tests::Fixture, impact::Type122ImpactOutcome, Type122Owner},
    world_fx::BallisticDamageRequest,
};

fn hit(id: u32, infected: bool) -> ParticleEntityImpact {
    ParticleEntityImpact {
        source_particle_class: if infected { 5 } else { 16 },
        impact_position_argument_va: if infected { 0x004d_cf48 } else { 0 },
        target_entity_id: id,
        position_world: [0.0; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(BallisticDamageRequest {
            packet: if infected {
                FUN_0043F780_DAMAGE_PACKET
            } else {
                DamagePacket {
                    channels: [1, 0],
                    amounts_raw: [6000, 0],
                }
            },
            source_entity_type_at_birth: Some(34),
            source_owner_id: Some(35),
        }),
    }
}

#[v2k_test_support::retail_test]
fn cinematic_type122_primary_and_infected_hits_reach_native_owner_and_preserve_entry_order() {
    for infected in [false, true] {
        let mut f = Fixture::new(50);
        let id = f.id;
        let entity = f.entities.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(u32::MAX, 0x68000);
        entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
        entity.set_motion_raw([0, 10_000, 0], [50, -80, 100]);
        let allocation = entity.native_type122_runtime;
        let steps = deliver_entity_impact(
            &mut f.session.cache,
            &mut f.entities,
            &mut f.fx,
            &mut f.damage,
            &mut f.tasks,
            &mut f.notifications,
            hit(id, infected),
            500,
        );
        let [Intro2EntityImpactStep::Type122Damage(Type122ImpactOutcome::Applied(applied))] =
            steps.as_slice()
        else {
            panic!("infected{infected}: {steps:?}")
        };
        assert_eq!(applied.filtered_damage_raw, if infected { 0 } else { 2000 });
        let entity = f.entities.entity_mut(id).unwrap();
        assert_eq!(
            entity.collision.health_raw,
            RetailRuntimeValue::Known(if infected { 7000 } else { 5000 })
        );
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(if infected { 17 } else { 500 })
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x2000),
            RetailRuntimeValue::Known(if infected { 0x2000 } else { 0 })
        );
        assert_eq!(entity.native_type122_runtime, allocation);
        assert!(Type122Owner::adopt(&f.entities, id).is_ok());
        assert_eq!(
            f.tasks.adopt_type122(&f.entities),
            0,
            "the same cinematic owner retains current graph custody"
        );
    }
}

#[v2k_test_support::retail_test]
fn cinematic_foreign_type122_receipt_blocks_before_generic_damage_stamp_and_rng() {
    let mut f = Fixture::new(50);
    let mut other = Fixture::new(50);
    let id = f.id;
    std::mem::swap(
        f.entities.entity_mut(id).unwrap(),
        other.entities.entity_mut(id).unwrap(),
    );
    let before = f.entities.entity_mut(id).unwrap().collision.clone();
    let mut expected = f.fx.fork_for_main_base_abort_transaction();
    let steps = deliver_entity_impact(
        &mut f.session.cache,
        &mut f.entities,
        &mut f.fx,
        &mut f.damage,
        &mut f.tasks,
        &mut f.notifications,
        hit(id, false),
        500,
    );
    assert!(
        matches!(
            steps.as_slice(),
            [Intro2EntityImpactStep::Type122Damage(
                Type122ImpactOutcome::Blocked {
                    committed_prefix: false,
                    ..
                }
            )]
        ),
        "{steps:?}"
    );
    assert_eq!(f.entities.entity_mut(id).unwrap().collision, before);
    assert_eq!(
        f.fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}
