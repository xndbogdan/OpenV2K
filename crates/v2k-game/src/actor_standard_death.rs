//! Detached standard death installation for ordinary type-17/type-47 actors.
//!
//! Retail reaches this path after the accepted primary-hit damage branch
//! re-resolves the live entity and dispatches the current type runtime's
//! alternate behavior entry. The live re-resolution, component pointer binding,
//! and process RNG owner are still outside this module. This module only joins
//! the authenticated lethal-hit plan to the exact class-12 task-owner
//! transaction recovered in [`crate::common_dying`].

use crate::{
    actor_death::{
        plan_type_17_or_47_primary_lethal_hit, ActorHitDeathPlanError, ActorPrimaryHitDeathPlan,
        ActorPrimaryLethalHitRequest,
    },
    actor_task_owner::{ActorTaskOwner, ActorTaskPrepareError},
    common_dying::{
        plan_common_dying_setup, CommonDyingConstructorEffect, CommonDyingTaskSpec,
        PreparedCommonDyingTask,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandardActorDeathInstallRequest {
    pub owner_entity_id: u32,
    pub lethal_hit: ActorPrimaryLethalHitRequest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StandardActorDeathInstallError<E> {
    Plan(ActorHitDeathPlanError),
    Preparation(ActorTaskPrepareError<E>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandardActorDeathInstallOutcome {
    pub lethal_plan: ActorPrimaryHitDeathPlan,
}

/// Apply the recovered detached standard-dying setup for one authenticated
/// type-17/type-47 lethal hit.
///
/// The lethal-hit request is intentionally re-planned internally. Accepting a
/// caller-supplied [`ActorPrimaryHitDeathPlan`] would make this function trust a
/// cloneable value instead of the original behavior-style and damage evidence.
/// Successful preparation runs the complete class-12 constructor suffix before
/// the primary slot is published; preparation failure preserves the old primary
/// while keeping the earlier secondary/tertiary clears.
pub fn install_standard_actor_death<T, E>(
    request: StandardActorDeathInstallRequest,
    owner: &mut ActorTaskOwner<T>,
    next_shared_random: impl FnMut() -> u32,
    apply_effect: impl FnMut(CommonDyingConstructorEffect),
    prepare: impl FnMut(CommonDyingTaskSpec) -> Result<PreparedCommonDyingTask<T>, E>,
) -> Result<StandardActorDeathInstallOutcome, StandardActorDeathInstallError<E>> {
    let lethal_plan = plan_type_17_or_47_primary_lethal_hit(request.lethal_hit)
        .map_err(StandardActorDeathInstallError::Plan)?;

    plan_common_dying_setup(request.owner_entity_id)
        .apply(owner, next_shared_random, apply_effect, prepare)
        .map_err(StandardActorDeathInstallError::Preparation)?;

    Ok(StandardActorDeathInstallOutcome { lethal_plan })
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;
    use crate::{
        actor_death::{CommonEnemyActorType, COMMON_ACTOR_DYING_PROGRAM},
        actor_task_owner::{ActorTaskSlot, PreparedActorTask},
        common_dying::{
            CommonDyingComponentDescriptors, CommonDyingSubGDescriptorSnapshot,
            CommonDyingTaskState,
        },
        damage::GenericEntityDamageState,
        entity_behavior::{audited_behavior_style, BehaviorStyle, DeathCallbackPolicy},
        entity_collision_state::{
            CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
        },
    };
    use v2k_formats::collision::SubAPropulsionDescriptor;

    fn style(class_id: u32, variant: u8) -> BehaviorStyle {
        *audited_behavior_style(class_id, variant).unwrap()
    }

    fn lethal_request(
        actor_type: CommonEnemyActorType,
        pre_impact_style: BehaviorStyle,
        post_impact_style: BehaviorStyle,
    ) -> StandardActorDeathInstallRequest {
        StandardActorDeathInstallRequest {
            owner_entity_id: 0x0497_0001,
            lethal_hit: ActorPrimaryLethalHitRequest {
                actor_type,
                pre_impact_style,
                post_impact_style,
                generic_damage_state: GenericEntityDamageState {
                    health_raw: 1_400,
                    pre_health_buffer_raw: 0,
                    already_dying: false,
                },
                accepted_damage_raw: 1_800,
            },
        }
    }

    fn metadata(topology: CommonMoverComponentTopology) -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            common_mover_topology: RetailRuntimeValue::Known(topology),
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(None),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn prepare_dying<T>(
        owner_entity_id: u32,
        metadata: &EntityTypeRuntimeMetadata,
        descriptors: CommonDyingComponentDescriptors,
        map: impl FnOnce(CommonDyingTaskState) -> T,
    ) -> PreparedCommonDyingTask<T> {
        CommonDyingTaskState::prepare_after_allocation(owner_entity_id, metadata, descriptors)
            .unwrap()
            .map_task(map)
    }

    fn seed_owner() -> ActorTaskOwner<&'static str> {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new("old-primary"),
        );
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new("old-secondary"),
        );
        owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new("old-tertiary"),
        );
        owner
    }

    #[test]
    fn valid_type_17_and_type_47_hits_install_common_dying_primary() {
        for (actor_type, pre, post) in [
            (CommonEnemyActorType::SpiderType17, style(9, 0), style(9, 0)),
            (
                CommonEnemyActorType::NewantType47,
                style(6, 0),
                style(32, 0),
            ),
        ] {
            let mut owner = seed_owner();
            let effects = RefCell::new(Vec::new());
            let draw_count = Cell::new(0);
            let metadata = metadata(CommonMoverComponentTopology::default());

            let outcome = install_standard_actor_death(
                lethal_request(actor_type, pre, post),
                &mut owner,
                || {
                    draw_count.set(draw_count.get() + 1);
                    0
                },
                |effect| effects.borrow_mut().push(effect),
                |specification| {
                    assert_eq!(specification.owner_entity_id(), 0x0497_0001);
                    assert_eq!(specification.lifetime_ms(), 9_000);
                    Ok::<PreparedCommonDyingTask<&'static str>, ()>(prepare_dying(
                        specification.owner_entity_id(),
                        &metadata,
                        CommonDyingComponentDescriptors::default(),
                        |_| "dying",
                    ))
                },
            )
            .unwrap();

            assert_eq!(outcome.lethal_plan.actor_type, actor_type);
            assert_eq!(
                outcome.lethal_plan.post_impact_death_policy,
                DeathCallbackPolicy::None
            );
            assert_eq!(owner.state_in_slot(ActorTaskSlot::Primary), Some(&"dying"));
            assert!(owner.task_in_slot(ActorTaskSlot::Secondary).is_none());
            assert!(owner.task_in_slot(ActorTaskSlot::Tertiary).is_none());
            assert_eq!(draw_count.get(), 0);
            assert_eq!(
                *effects.borrow(),
                [CommonDyingConstructorEffect::WriteOwnerVerticalVelocity {
                    velocity_raw: COMMON_ACTOR_DYING_PROGRAM.initial_vertical_velocity_raw,
                }]
            );
        }
    }

    #[test]
    fn primary_prepare_failure_keeps_primary_after_auxiliary_clears() {
        let mut owner = seed_owner();
        let draw_count = Cell::new(0);
        let effects = Cell::new(0);

        let result = install_standard_actor_death(
            lethal_request(CommonEnemyActorType::SpiderType17, style(9, 0), style(9, 0)),
            &mut owner,
            || {
                draw_count.set(draw_count.get() + 1);
                0
            },
            |_| effects.set(effects.get() + 1),
            |_| Err::<PreparedCommonDyingTask<&'static str>, _>("allocation failed"),
        );

        assert_eq!(
            result,
            Err(StandardActorDeathInstallError::Preparation(
                ActorTaskPrepareError {
                    action_index: 2,
                    slot: ActorTaskSlot::Primary,
                    error: "allocation failed",
                }
            ))
        );
        assert_eq!(
            owner.state_in_slot(ActorTaskSlot::Primary),
            Some(&"old-primary")
        );
        assert!(owner.task_in_slot(ActorTaskSlot::Secondary).is_none());
        assert!(owner.task_in_slot(ActorTaskSlot::Tertiary).is_none());
        assert_eq!(draw_count.get(), 0);
        assert_eq!(effects.get(), 0);
    }

    #[test]
    fn authored_sub_g_then_sub_a_suffix_order_is_preserved() {
        let mut topology = CommonMoverComponentTopology::default();
        topology.sub_a = true;
        topology.sub_g = true;
        let mut metadata = metadata(topology);
        metadata.sub_a_propulsion_descriptor =
            RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                acceleration_raw: 0,
                overspeed_correction_raw: 0,
                target_speed_base_raw: 2_560,
            }));
        let descriptors = CommonDyingComponentDescriptors {
            sub_g: RetailRuntimeValue::Known(Some(CommonDyingSubGDescriptorSnapshot {
                source_raw_at_0x00: 0xABCD,
                randomized_target_base_raw_at_0x0c: -5,
            })),
        };
        let mut owner = ActorTaskOwner::new();
        let draws = Cell::new(0);
        let effects = RefCell::new(Vec::new());

        install_standard_actor_death(
            lethal_request(
                CommonEnemyActorType::NewantType47,
                style(6, 0),
                style(32, 0),
            ),
            &mut owner,
            || {
                let draw = match draws.get() {
                    0 => 0xCAFE_1200,
                    1 => 0xCAFE_FF00,
                    _ => panic!("unexpected extra RNG draw"),
                };
                draws.set(draws.get() + 1);
                draw
            },
            |effect| effects.borrow_mut().push(effect),
            |specification| {
                Ok::<PreparedCommonDyingTask<&'static str>, ()>(prepare_dying(
                    specification.owner_entity_id(),
                    &metadata,
                    descriptors,
                    |_| "dying",
                ))
            },
        )
        .unwrap();

        assert_eq!(draws.get(), 2);
        assert_eq!(
            *effects.borrow(),
            [
                CommonDyingConstructorEffect::WriteSubGMode3f { value: 0 },
                CommonDyingConstructorEffect::WriteSubGRandomizedTarget38 {
                    target_raw: 13,
                    random_sample_low16: 0x1200,
                    descriptor_base_raw: -5,
                },
                CommonDyingConstructorEffect::ResetSubGModeAndAccumulatorThenWriteSource {
                    source_raw: 0xABCD,
                    accumulator_raw: 0,
                    mode_40: 0,
                },
                CommonDyingConstructorEffect::WriteSubGPhase3c { value: 0 },
                CommonDyingConstructorEffect::WriteSubADirection {
                    direction_multiplier: 1,
                },
                CommonDyingConstructorEffect::WriteSubATargetSpeed {
                    target_speed_raw: 2_815,
                    random_sample_low16: 0xFF00,
                },
                CommonDyingConstructorEffect::WriteOwnerVerticalVelocity { velocity_raw: 500 },
                CommonDyingConstructorEffect::WriteSubADirection {
                    direction_multiplier: 1,
                },
                CommonDyingConstructorEffect::WriteSubGMode3f { value: 1 },
            ]
        );
    }

    #[test]
    fn unsupported_capture_death_and_nonlethal_requests_do_not_mutate_owner() {
        for mut request in [
            lethal_request(CommonEnemyActorType::SpiderType17, style(9, 4), style(9, 4)),
            lethal_request(CommonEnemyActorType::SpiderType17, style(9, 0), style(9, 0)),
        ] {
            if request.lethal_hit.pre_impact_style.variant == 0 {
                request.lethal_hit.accepted_damage_raw = 10;
            }
            let mut owner = seed_owner();
            let draws = Cell::new(0);
            let effects = Cell::new(0);
            let preparations = Cell::new(0);

            assert!(matches!(
                install_standard_actor_death(
                    request,
                    &mut owner,
                    || {
                        draws.set(draws.get() + 1);
                        0
                    },
                    |_| effects.set(effects.get() + 1),
                    |_| {
                        preparations.set(preparations.get() + 1);
                        Ok::<PreparedCommonDyingTask<&'static str>, ()>(prepare_dying(
                            0x0497_0001,
                            &metadata(CommonMoverComponentTopology::default()),
                            CommonDyingComponentDescriptors::default(),
                            |_| "dying",
                        ))
                    },
                ),
                Err(StandardActorDeathInstallError::Plan(_))
            ));
            assert_eq!(
                owner.state_in_slot(ActorTaskSlot::Primary),
                Some(&"old-primary")
            );
            assert_eq!(
                owner.state_in_slot(ActorTaskSlot::Secondary),
                Some(&"old-secondary")
            );
            assert_eq!(
                owner.state_in_slot(ActorTaskSlot::Tertiary),
                Some(&"old-tertiary")
            );
            assert_eq!(draws.get(), 0);
            assert_eq!(effects.get(), 0);
            assert_eq!(preparations.get(), 0);
        }
    }
}
