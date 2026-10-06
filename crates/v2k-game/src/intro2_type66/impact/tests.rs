use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    damage::{DamagePacket, FUN_0043F780_DAMAGE_DELIVERY},
    intro2_type47_live::world::native_intro2_fixture,
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
            source_entity_type_at_birth: Some(34),
            source_owner_id: Some(35),
        }),
    }
}

#[v2k_test_support::retail_test]
fn both_native_factories_filter_hits_without_reselecting_or_jolting_fixed_bodies() {
    for spawn in [36, 51] {
        let Some((_session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn))
            .unwrap()
            .id;
        let entity = manager.entity_mut(id).unwrap();
        let before = (
            entity.position_raw(),
            entity.velocity_raw(),
            entity.rotation_heading_pitch_roll_raw(),
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        );
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_intro2_type66(&manager), 2);
        let mut fx = WorldFx::new();
        let result = apply_intro2_type66_particle_hit(
            &mut manager,
            &mut fx,
            &mut scheduler,
            impact(id, false, 2500),
            251,
        );
        let Intro2Type66ImpactOutcome::Applied(result) = result else {
            panic!("{result:?}");
        };
        assert_eq!(result.filtered_damage_raw, 500);
        assert!(result.death_publication.is_none());
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.collision.health_raw,
            RetailRuntimeValue::Known(99_499)
        );
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(251)
        );
        assert_eq!(
            (
                entity.position_raw(),
                entity.velocity_raw(),
                entity.rotation_heading_pitch_roll_raw(),
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary)
            ),
            before
        );
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            38,
            "fixed impact consumes no random words"
        );
    }
}

#[v2k_test_support::retail_test]
fn infected_zero_damage_selects_infected_model_without_primary_stamp_or_task_replacement() {
    let Some((_session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(36))
        .unwrap()
        .id;
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
    let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
    let mut fx = WorldFx::new();
    let result = apply_intro2_type66_particle_hit(
        &mut manager,
        &mut fx,
        &mut SpecializedActorTaskScheduler::new(),
        impact(id, true, 0),
        251,
    );
    let Intro2Type66ImpactOutcome::Applied(result) = result else {
        panic!("{result:?}");
    };
    assert_eq!(result.filtered_damage_raw, 0);
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity.collision.health_raw,
        RetailRuntimeValue::Known(99_999)
    );
    assert_eq!(
        entity.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(17)
    );
    assert_eq!(state_bits(entity, 0x2000).unwrap(), 0x2000);
    assert_eq!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        primary
    );
    assert_eq!(fx.next_shared_retail_random_u16(), 38);
}

#[v2k_test_support::retail_test]
fn fixed_factory_reaction_only_reads_gates_reached_before_zero_filter() {
    for disabled in [false, true] {
        for infected in [false, true] {
            let Some((_session, mut manager, _)) = native_intro2_fixture() else {
                return;
            };
            let id = manager
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(51))
                .unwrap()
                .id;
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(
                state_bits(
                    entity,
                    IMPACT_REACTION_ENABLED_STATE_BIT | IMPACT_REACTION_SUPPRESSED_STATE_BIT
                )
                .unwrap(),
                IMPACT_REACTION_ENABLED_STATE_BIT | IMPACT_REACTION_SUPPRESSED_STATE_BIT
            );
            // Canonical fixed factories enable reaction but suppress it. The
            // controlled disabled branch must not consult even suppression;
            // neither early return reaches the later network notification.
            let unread_mask = if disabled {
                entity
                    .collision
                    .state_flags_at_0x08
                    .overwrite(IMPACT_REACTION_ENABLED_STATE_BIT, 0);
                IMPACT_REACTION_SUPPRESSED_STATE_BIT | IMPACT_REACTION_NETWORKED_STATE_BIT
            } else {
                IMPACT_REACTION_NETWORKED_STATE_BIT
            };
            entity.collision.state_flags_at_0x08.invalidate(unread_mask);
            entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
            let before = (
                entity.position_raw(),
                entity.velocity_raw(),
                entity.rotation_heading_pitch_roll_raw(),
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            );
            let mut fx = WorldFx::new();
            let result = apply_intro2_type66_particle_hit(
                &mut manager,
                &mut fx,
                &mut SpecializedActorTaskScheduler::new(),
                impact(id, infected, 2000),
                251,
            );
            assert!(
                matches!(result, Intro2Type66ImpactOutcome::Applied(ref result) if result.filtered_damage_raw == 0),
                "{result:?}"
            );
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(
                (
                    entity.position_raw(),
                    entity.velocity_raw(),
                    entity.rotation_heading_pitch_roll_raw(),
                    entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary)
                ),
                before
            );
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(99_999)
            );
            assert_eq!(
                entity.collision.last_hit_presentation_tick_at_0x34,
                RetailRuntimeValue::Known(if infected { 17 } else { 251 })
            );
            assert_eq!(
                entity.collision.state_flags_at_0x08.masked(unread_mask),
                RetailRuntimeValue::Unresolved
            );
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                WorldFx::new().next_shared_retail_random_u16()
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn lethal_hit_restarts_working_factory_before_next_particle_and_keeps_the_same_allocation() {
    for spawn in [36, 51] {
        let Some((_session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn))
            .unwrap()
            .id;
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.health_raw = RetailRuntimeValue::Known(1);
        let before = (
            entity.intro2_type66_runtime,
            entity.position_raw(),
            entity.model_index,
        );
        let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.adopt_intro2_type66(&manager);
        let mut fx = WorldFx::new();
        let mut expected_rng = WorldFx::new();
        expected_rng.next_shared_retail_random_u16();
        let result = apply_intro2_type66_particle_hit(
            &mut manager,
            &mut fx,
            &mut scheduler,
            impact(id, false, 2500),
            251,
        );
        let Intro2Type66ImpactOutcome::Applied(result) = result else {
            panic!("{result:?}");
        };
        assert!(result.death_publication.is_some());
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.collision.health_raw,
            RetailRuntimeValue::Known(10_000_000)
        );
        assert_eq!(
            (
                entity.intro2_type66_runtime,
                entity.position_raw(),
                entity.model_index
            ),
            before
        );
        assert_ne!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            primary
        );
        let RetailRuntimeValue::Known(Some(factory)) = entity.base_factory_runtime else {
            panic!()
        };
        assert_eq!(factory.progressive_death.elapsed_micros_raw, 1);
        let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
        let result = apply_intro2_type66_particle_hit(
            &mut manager,
            &mut fx,
            &mut scheduler,
            impact(id, false, 2500),
            252,
        );
        assert!(
            matches!(result, Intro2Type66ImpactOutcome::Applied(ref result) if result.death_publication.is_none())
        );
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            primary
        );
        assert_eq!(
            entity.collision.health_raw,
            RetailRuntimeValue::Known(9_999_500)
        );
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected_rng.next_shared_retail_random_u16()
        );
    }
}
