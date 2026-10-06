//! Type53 receipt binding for the common native particle-hit sequence.
pub use crate::native_ground_actor::impact::{
    NativeGroundImpactBlock as Intro2Type53ImpactBlock,
    NativeGroundImpactOutcome as Intro2Type53ImpactOutcome,
};
pub(crate) fn apply_intro2_type53_particle_hit(
    frame: crate::shared_actor_impact::SharedActorImpactFrame<'_>,
    impact: crate::world_fx::ParticleEntityImpact,
) -> Intro2Type53ImpactOutcome {
    crate::native_ground_actor::impact::apply_native_ground_particle_hit::<
        super::profile::Type53Profile,
    >(frame, impact)
}
#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;
    use crate::{
        damage::{DamagePacket, FUN_0043F780_DAMAGE_DELIVERY},
        intro2_common_dying::Intro2CommonDyingOwner,
        intro2_type47_live::world::native_intro2_fixture,
        specialized_actor_task_production::SpecializedActorTaskScheduler,
        world_fx::{BallisticDamageRequest, ParticleEntityImpact, WorldFx},
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
    fn native_type53_primary_and_infected_hits_reselect_without_losing_scheduler_custody() {
        for infected in [false, true] {
            let Some((_session, mut manager, _)) = native_intro2_fixture() else {
                return;
            };
            let id = manager
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(20))
                .unwrap()
                .id;
            let entity = manager.entity_mut(id).unwrap();
            entity.collision.health_raw = RetailRuntimeValue::Known(1_000_000);
            entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
            let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
            let mut scheduler = SpecializedActorTaskScheduler::new();
            scheduler.adopt_intro2_type53(&manager);
            let result = apply_intro2_type53_particle_hit(
                crate::shared_actor_impact::SharedActorImpactFrame {
                    resources: &_session.cache,
                    entities: &mut manager,
                    world_fx: &mut WorldFx::new(),
                    scheduler: &mut scheduler,
                    notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                    retail_tick: 251,
                },
                impact(id, infected, 2_500),
            );
            assert!(
                matches!(result, Intro2Type53ImpactOutcome::Applied(_)),
                "{result:?}"
            );
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(
                entity.collision.last_hit_presentation_tick_at_0x34,
                RetailRuntimeValue::Known(if infected { 17 } else { 251 })
            );
            assert_ne!(
                primary,
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary)
            );
            assert!(Intro2Type53Owner::adopt(&manager, id).is_ok());
            assert_eq!(
                scheduler.adopt_intro2_type53(&manager),
                0,
                "replacement already retained"
            );
        }
    }

    #[v2k_test_support::retail_test]
    fn native_type53_lethal_fragment_publishes_class12_before_primary_suffix() {
        let Some((_session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(38))
            .unwrap()
            .id;
        manager.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(1);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.adopt_intro2_type53(&manager);
        let result = apply_intro2_type53_particle_hit(
            crate::shared_actor_impact::SharedActorImpactFrame {
                resources: &_session.cache,
                entities: &mut manager,
                world_fx: &mut WorldFx::new(),
                scheduler: &mut scheduler,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                retail_tick: 251,
            },
            impact(id, false, 2_500),
        );
        let Intro2Type53ImpactOutcome::Applied(result) = result else {
            panic!("{result:?}")
        };
        assert!(result.filtered_damage_raw > 0);
        assert!(result.death_publication.is_some());
        assert!(Intro2CommonDyingOwner::adopt(&manager, id).is_ok());
        assert_eq!(
            manager.entity_mut(id).unwrap().collision.health_raw,
            RetailRuntimeValue::Known(0)
        );
    }
}
