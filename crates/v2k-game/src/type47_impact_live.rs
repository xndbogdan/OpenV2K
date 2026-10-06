//! Live Type-47 primary `40DAC0`/+28 and infected `40DA00`/+20 hit callbacks.
//!
//! `FUN_00410EB0` stamps `+0x34` then invokes the common type callback. This
//! owner applies the detached C690 plan to a live entity: TypeDefault null
//! word selects Always x9 Guard or Always x1 Wander and publishes those
//! *initial* styles. It does not invent a Pursuing/Chase/Aim install.
//! Dying-bit `0x4000` is `FUN_00425660` / `FUN_00438340(entity, 0, *+0x124)`
//! then `FUN_0040C620`. That is not the generic death prefix.

use crate::actor_task_dispatcher::ActorTaskRuntimeFamily;
use crate::actor_task_owner::ActorTaskSlot;
use crate::damage::EntityHitEntry;
use crate::entity::{Entity, EntityManager};
use crate::entity_behavior::ActiveBehaviorStyle;
use crate::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use crate::ordinary_type47_death_live::{
    publish_type47_c690_alternate_class12, Type47CommonDyingPublication,
    Type47CommonDyingPublicationError,
};
use crate::type47_c690::{
    plan_type47_impact_c690, Type47ImpactC690Error, Type47ImpactC690Outcome,
    Type47ImpactC690Request,
};
use crate::type47_initial_behavior_live::{
    publish_type47_reselected_initial_behavior, FreshType47InitialBehaviorError,
    FreshType47InitializerPublication, TYPE47_GUARD_BEHAVIOR_CLASS_ID,
    TYPE47_WANDER_BEHAVIOR_CLASS_ID,
};
use crate::world_fx::WorldFx;

const ORDINARY_TYPE47_ENTITY_TYPE: u32 = 47;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type47ImpactLiveOutcome {
    NotApplicable,
    CallbackAbsent,
    GuardPublished {
        planned: Type47ImpactC690Outcome,
        initializer: FreshType47InitializerPublication,
    },
    WanderPublished {
        planned: Type47ImpactC690Outcome,
        initializer: FreshType47InitializerPublication,
    },
    Class12Published {
        planned: Type47ImpactC690Outcome,
        publication: Type47CommonDyingPublication,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type47ImpactLiveRequest {
    pub entry: EntityHitEntry,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type47ImpactLiveError {
    AllocationCohortUnavailable,
    Plan(Type47ImpactC690Error),
    Publish(FreshType47InitialBehaviorError),
    AlternateClass12(Type47CommonDyingPublicationError),
    CurrentBehaviorContextUnresolved,
    CurrentBehaviorContextAbsent,
}

/// Apply the selected type trampoline before impulse and checked damage.
pub fn apply_type47_impact_c690_live(
    manager: &mut EntityManager,
    entity_id: u32,
    world_fx: &mut WorldFx,
    request: Type47ImpactLiveRequest,
) -> Result<Type47ImpactLiveOutcome, Type47ImpactLiveError> {
    let Some(metadata) = manager.type_runtime_metadata(47).cloned() else {
        return Ok(Type47ImpactLiveOutcome::NotApplicable);
    };
    apply_type47_impact_c690_live_with_random(
        manager,
        entity_id,
        &metadata,
        world_fx,
        request,
        |fx| u32::from(fx.next_shared_retail_random_u16()),
    )
}

pub(crate) fn apply_type47_impact_c690_live_with_random(
    manager: &mut EntityManager,
    entity_id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
    request: Type47ImpactLiveRequest,
    mut next_selector_random: impl FnMut(&mut WorldFx) -> u32,
) -> Result<Type47ImpactLiveOutcome, Type47ImpactLiveError> {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return Ok(Type47ImpactLiveOutcome::NotApplicable);
    };
    if entity.entity_type != ORDINARY_TYPE47_ENTITY_TYPE {
        return Ok(Type47ImpactLiveOutcome::NotApplicable);
    }
    if entity.native_type47_construction.is_some()
        && !crate::shared_type47::type47_manager_allocation_authenticates(manager, entity_id)
    {
        return Err(Type47ImpactLiveError::AllocationCohortUnavailable);
    }
    let cohort = crate::type47_initial_behavior_live::live_type47_cohort(entity)
        .ok_or(Type47ImpactLiveError::AllocationCohortUnavailable)?;
    let current_style = match entity.current_behavior_context {
        RetailRuntimeValue::Unresolved => {
            return Err(Type47ImpactLiveError::CurrentBehaviorContextUnresolved);
        }
        RetailRuntimeValue::Known(None) => {
            return Err(Type47ImpactLiveError::CurrentBehaviorContextAbsent);
        }
        RetailRuntimeValue::Known(Some(context)) => {
            RetailRuntimeValue::Known(Some(context.active_style()))
        }
    };
    let choice_list_source = match entity.current_behavior_context {
        RetailRuntimeValue::Known(Some(context)) => context.choice_list_source(),
        _ => RetailRuntimeValue::Unresolved,
    };
    // AC60 / 425660 consumes only the dying selector. Unrelated surface
    // writers may still own unknown bits on a native Intro2 allocation.
    let state_flags_raw = entity
        .collision
        .state_flags_at_0x08
        .masked(crate::type47_c690::TYPE47_IMPACT_DYING_STATE_BIT);

    let Some(entity) = manager.ordinary_type47_entity_mut(entity_id) else {
        return Ok(Type47ImpactLiveOutcome::NotApplicable);
    };
    if request.entry == EntityHitEntry::PrimaryProjectile {
        entity.collision.last_hit_presentation_tick_at_0x34 =
            RetailRuntimeValue::Known(request.retail_tick);
    }

    let planned = plan_type47_impact_c690(
        Type47ImpactC690Request {
            entry: request.entry,
            entity_type: ORDINARY_TYPE47_ENTITY_TYPE,
            current_style,
            choice_list_source,
            state_flags_raw,
            metadata,
        },
        || next_selector_random(world_fx),
    )
    .map_err(Type47ImpactLiveError::Plan)?;
    let Some(planned) = planned else {
        return Ok(Type47ImpactLiveOutcome::CallbackAbsent);
    };

    match planned {
        Type47ImpactC690Outcome::AlternateCommonDying { .. } => {
            let publication = publish_type47_c690_alternate_class12(entity, metadata, world_fx)
                .map_err(Type47ImpactLiveError::AlternateClass12)?;
            Ok(Type47ImpactLiveOutcome::Class12Published {
                planned,
                publication,
            })
        }
        Type47ImpactC690Outcome::Weighted { selection, .. } => {
            let initializer = publish_type47_reselected_initial_behavior(
                entity, metadata, cohort, selection, world_fx,
            )
            .map_err(Type47ImpactLiveError::Publish)?;
            Ok(match selection.program.class_id {
                TYPE47_GUARD_BEHAVIOR_CLASS_ID => Type47ImpactLiveOutcome::GuardPublished {
                    planned,
                    initializer,
                },
                TYPE47_WANDER_BEHAVIOR_CLASS_ID => Type47ImpactLiveOutcome::WanderPublished {
                    planned,
                    initializer,
                },
                _ => unreachable!("C690 planner only returns class 32 or 6"),
            })
        }
    }
}

pub fn type47_live_graph_is_initial_guard(entity: &Entity) -> bool {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return false;
    };
    matches!(
        context.active_style(),
        ActiveBehaviorStyle::Audited(style) if style.class_id == 32 && style.variant == 0
    ) && entity
        .actor_task_state(ActorTaskSlot::Secondary)
        .is_some_and(|task| task.family() == ActorTaskRuntimeFamily::GuardLocationAcquisition)
        && entity
            .actor_task_state(ActorTaskSlot::Primary)
            .is_some_and(|task| task.family() == ActorTaskRuntimeFamily::OrdinaryType9Wander)
}

pub fn type47_live_graph_is_initial_wander(entity: &Entity) -> bool {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return false;
    };
    matches!(
        context.active_style(),
        ActiveBehaviorStyle::Audited(style) if style.class_id == 6
    ) && entity.actor_task_state(ActorTaskSlot::Secondary).is_none()
        && entity
            .actor_task_state(ActorTaskSlot::Primary)
            .is_some_and(|task| task.family() == ActorTaskRuntimeFamily::OrdinaryType9Wander)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common_mover::SubAPropulsionRuntime;
    use crate::entity::EntityKind;
    use crate::entity_collision_state::{EntityInitializerSpec, RetailStateWord};
    use crate::ordinary_type47_death_live::{
        TYPE47_COMMON_DYING_BEHAVIOR_CHOICES, TYPE47_COMMON_DYING_BEHAVIOR_CLASS_ID,
        TYPE47_COMMON_DYING_CAPABILITY_FLAGS, TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR,
        TYPE47_COMMON_DYING_DEATH_SOUND_ID, TYPE47_COMMON_DYING_INITIALIZER_STATE_FLAGS_RAW,
        TYPE47_COMMON_DYING_INITIAL_HEALTH_RAW, TYPE47_COMMON_DYING_MASS_RAW,
        TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR, TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR,
        TYPE47_COMMON_DYING_SUB_H_RECORDS,
    };
    use crate::ordinary_type47_live::{
        FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY, FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID,
        FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR, FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D,
        FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT,
    };
    use crate::specialized_actor_task_production::{
        SpecializedActorTaskFamily, SpecializedActorTaskScheduler,
    };
    use crate::sub_h_external_frame::SubHRuntimeState;
    use crate::type47_c690::TYPE47_IMPACT_DYING_STATE_BIT;
    use crate::type47_initial_behavior_live::{
        plan_fresh_type47_initial_behavior, publish_fresh_type47_initial_behavior,
        FreshType47InitializerPublication,
    };
    use v2k_formats::collision::SubHExternalFrameDescriptor;

    const ENTITY_ID: u32 = 0x042F_000B;

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID as u16; 4],
            mass_raw: TYPE47_COMMON_DYING_MASS_RAW,
            capability_flags: TYPE47_COMMON_DYING_CAPABILITY_FLAGS,
            initial_health_raw: Some(TYPE47_COMMON_DYING_INITIAL_HEALTH_RAW),
            death_sound_id: RetailRuntimeValue::Known(Some(TYPE47_COMMON_DYING_DEATH_SOUND_ID)),
            constructor_sound_attachment_id: RetailRuntimeValue::Known(None),
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR,
            )),
            sub_c_lift_descriptor: RetailRuntimeValue::Known(Some(
                TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR,
            )),
            sub_h_external_frame_descriptor: RetailRuntimeValue::Known(Some(
                SubHExternalFrameDescriptor {
                    completion_sound_id: None,
                    records: TYPE47_COMMON_DYING_SUB_H_RECORDS.to_vec(),
                },
            )),
            projectile_emitter_descriptor: RetailRuntimeValue::Known(Some(
                FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR,
            )),
            common_mover_topology: RetailRuntimeValue::Known(
                FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY,
            ),
            sub_d_steering_descriptor: RetailRuntimeValue::Known(Some(
                FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D,
            )),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: TYPE47_COMMON_DYING_INITIALIZER_STATE_FLAGS_RAW,
                common_axis_descriptor: TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR,
                behavior_choices: TYPE47_COMMON_DYING_BEHAVIOR_CHOICES
                    .to_vec()
                    .into_boxed_slice(),
                behavior_rule_ref: 0,
                alternate_behavior_class_ref: TYPE47_COMMON_DYING_BEHAVIOR_CLASS_ID,
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn publish_guard() -> (EntityManager, EntityTypeRuntimeMetadata) {
        let metadata = exact_metadata();
        let mut entity = Entity::unresolved_port_entity(ENTITY_ID, EntityKind::Enemy, 47);
        entity.authored_spawn_index = Some(11);
        entity.model_slots = [Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID); 4];
        entity.model_index = Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID);
        entity.mass_raw = TYPE47_COMMON_DYING_MASS_RAW;
        entity.capability_flags = TYPE47_COMMON_DYING_CAPABILITY_FLAGS;
        entity.collision.health_raw =
            RetailRuntimeValue::Known(TYPE47_COMMON_DYING_INITIAL_HEALTH_RAW);
        entity.collision.default_state_flags_at_0xc8 =
            RetailRuntimeValue::Known(TYPE47_COMMON_DYING_INITIALIZER_STATE_FLAGS_RAW);
        entity.actor_common_axis_descriptor =
            RetailRuntimeValue::Known(TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR);
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(777), -1, 100),
        ));
        entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(
            SubHRuntimeState::new(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT).unwrap(),
        ));
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0x2039);
        let mut world_fx = WorldFx::new();
        let weighted =
            plan_fresh_type47_initial_behavior(&entity, &metadata, &mut world_fx).unwrap();
        entity.initial_behavior = RetailRuntimeValue::Known(Some(weighted.selection));
        let publication =
            publish_fresh_type47_initial_behavior(&mut entity, &metadata, weighted, &mut world_fx)
                .unwrap();
        assert!(matches!(
            publication.initializer,
            FreshType47InitializerPublication::GuardLocation { .. }
        ));
        let mut manager = EntityManager::from_entities_for_test(vec![entity]);
        manager.retain_fresh_level1_type47_initial_production(publication);
        (manager, metadata)
    }

    #[test]
    fn live_hit_selector_zero_republishes_initial_guard_not_pursuing() {
        let (mut manager, metadata) = publish_guard();
        let mut world_fx = WorldFx::new();
        let outcome = apply_type47_impact_c690_live_with_random(
            &mut manager,
            ENTITY_ID,
            &metadata,
            &mut world_fx,
            Type47ImpactLiveRequest {
                entry: EntityHitEntry::PrimaryProjectile,
                retail_tick: 250,
            },
            |_| 0,
        )
        .unwrap();
        assert!(matches!(
            outcome,
            Type47ImpactLiveOutcome::GuardPublished {
                initializer: FreshType47InitializerPublication::GuardLocation { .. },
                ..
            }
        ));
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert!(type47_live_graph_is_initial_guard(entity));
        assert!(!type47_live_graph_is_initial_wander(entity));
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(250)
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("reselection keeps a context");
        };
        assert!(matches!(
            context.active_style(),
            ActiveBehaviorStyle::Audited(style) if style.class_id == 32 && style.variant == 0
        ));
    }

    #[test]
    fn live_hit_high_word_publishes_wander_not_pursuing() {
        let (mut manager, metadata) = publish_guard();
        let mut world_fx = WorldFx::new();
        let outcome = apply_type47_impact_c690_live_with_random(
            &mut manager,
            ENTITY_ID,
            &metadata,
            &mut world_fx,
            Type47ImpactLiveRequest {
                entry: EntityHitEntry::PrimaryProjectile,
                retail_tick: 251,
            },
            |_| 0xFFFF,
        )
        .unwrap();
        assert!(matches!(
            outcome,
            Type47ImpactLiveOutcome::WanderPublished {
                initializer: FreshType47InitializerPublication::WanderNear { .. },
                ..
            }
        ));
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert!(type47_live_graph_is_initial_wander(entity));
        assert!(!type47_live_graph_is_initial_guard(entity));
    }

    #[test]
    fn infected_reselects_guard_then_initial_wander_without_replacing_primary_stamp() {
        let (mut manager, metadata) = publish_guard();
        manager
            .entity_mut_for_test(ENTITY_ID)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(19);
        let mut world_fx = WorldFx::new();
        for (tick, selector, expected_wander) in [(250, 0xFFFF, true), (251, 0, false)] {
            let mut draws = 0;
            let outcome = apply_type47_impact_c690_live_with_random(
                &mut manager,
                ENTITY_ID,
                &metadata,
                &mut world_fx,
                Type47ImpactLiveRequest {
                    entry: EntityHitEntry::Infected,
                    retail_tick: tick,
                },
                |_| {
                    draws += 1;
                    selector
                },
            )
            .unwrap();
            assert_eq!(draws, 1);
            assert_eq!(
                matches!(outcome, Type47ImpactLiveOutcome::WanderPublished { .. }),
                expected_wander
            );
            let entity = manager.entity_mut_for_test(ENTITY_ID).unwrap();
            assert_eq!(
                entity.collision.last_hit_presentation_tick_at_0x34,
                RetailRuntimeValue::Known(19)
            );
            assert_eq!(type47_live_graph_is_initial_wander(entity), expected_wander);
            assert_eq!(type47_live_graph_is_initial_guard(entity), !expected_wander);
        }
    }

    #[test]
    fn live_hit_dying_bit_installs_class12_without_death_prefix() {
        let (mut manager, metadata) = publish_guard();
        manager
            .entity_mut_for_test(ENTITY_ID)
            .unwrap()
            .collision
            .state_flags_at_0x08 = RetailStateWord::exact(0x2039 | TYPE47_IMPACT_DYING_STATE_BIT);
        let mut world_fx = WorldFx::new();
        let outcome = apply_type47_impact_c690_live_with_random(
            &mut manager,
            ENTITY_ID,
            &metadata,
            &mut world_fx,
            Type47ImpactLiveRequest {
                entry: EntityHitEntry::PrimaryProjectile,
                retail_tick: 252,
            },
            |_| 0,
        )
        .unwrap();
        assert!(matches!(
            outcome,
            Type47ImpactLiveOutcome::Class12Published { .. }
        ));
        assert!(world_fx.take_positional_sounds().is_empty());
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert!(!type47_live_graph_is_initial_guard(entity));
        assert!(!type47_live_graph_is_initial_wander(entity));
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("25660 keeps a context");
        };
        assert!(matches!(
            context.active_style(),
            ActiveBehaviorStyle::Audited(style)
                if style.class_id == 12 && style.variant == 0
        ));
        assert!(entity
            .actor_task_state(ActorTaskSlot::Primary)
            .is_some_and(|task| task.family() == ActorTaskRuntimeFamily::CommonDying));
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Secondary), None);
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
        assert_eq!(
            entity.collision.health_raw,
            RetailRuntimeValue::Known(TYPE47_COMMON_DYING_INITIAL_HEALTH_RAW)
        );
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(252)
        );
    }

    #[test]
    fn dying_bit_class12_receipt_replaces_type47_scheduler() {
        let (mut manager, metadata) = publish_guard();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(
            scheduler.adopt_fresh_level1_type47_scheduler(&mut manager),
            Ok(1)
        );
        assert_eq!(
            scheduler.family_for(ENTITY_ID),
            Some(SpecializedActorTaskFamily::OrdinaryType47Scheduler)
        );

        manager
            .entity_mut_for_test(ENTITY_ID)
            .unwrap()
            .collision
            .state_flags_at_0x08 = RetailStateWord::exact(0x2039 | TYPE47_IMPACT_DYING_STATE_BIT);
        let mut world_fx = WorldFx::new();
        let outcome = apply_type47_impact_c690_live_with_random(
            &mut manager,
            ENTITY_ID,
            &metadata,
            &mut world_fx,
            Type47ImpactLiveRequest {
                entry: EntityHitEntry::PrimaryProjectile,
                retail_tick: 253,
            },
            |_| 0,
        )
        .unwrap();
        let Type47ImpactLiveOutcome::Class12Published { publication, .. } = outcome else {
            panic!("dying-bit C690 publishes class 12");
        };
        scheduler
            .register_type47_common_dying(publication.owner)
            .unwrap();
        assert_eq!(
            scheduler.family_for(ENTITY_ID),
            Some(SpecializedActorTaskFamily::Type47CommonDying)
        );
        assert_eq!(scheduler.registered_len(), 1);
        assert_eq!(
            manager
                .iter_all()
                .find(|entity| entity.id == ENTITY_ID)
                .unwrap()
                .collision
                .health_raw,
            RetailRuntimeValue::Known(TYPE47_COMMON_DYING_INITIAL_HEALTH_RAW)
        );
    }
}
