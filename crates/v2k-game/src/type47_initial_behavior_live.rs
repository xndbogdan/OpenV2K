//! Fresh-Level-1 type-47 weighted birth publication.
//!
//! Retail `FUN_004104B0` runs `FUN_0040AC60 -> FUN_00425680 -> FUN_0040C6B0`
//! before the allocation is appended. Type 47 authors only Always x9 -> class
//! 32 Guard Location and Always x1 -> class 6 Wander Near, so the selector
//! consumes one shared word and needs no nearby walks. Each successful task
//! allocation then runs the shared `FUN_00406070` Sub-H-then-Sub-A suffix.
//! Style `+0x44` is the EXE-proven zero initializer argument for both
//! programs.

use std::convert::Infallible;

use crate::{
    actor_task_dispatcher::{
        prepare_guard_location_acquisition_runtime_task, prepare_shared_generic_constructor_suffix,
        ActorTaskRuntime, PreparedSharedGenericRuntimeTask, SharedGenericConstructorEffect,
    },
    actor_task_owner::ActorTaskSlot,
    common_mover::SubAPropulsionRuntime,
    entity::Entity,
    entity_behavior::{
        behavior_program, initial_behavior_state_policy, select_initial_behavior,
        BehaviorContextRuntime, BehaviorSelection, BehaviorSelectionError, BehaviorWeightRule,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    guard_location_owner::{
        apply_guard_location_setup, GuardLocationPrimaryContext, GuardLocationSetupError,
        GuardLocationSetupRequest, GuardLocationTaskPreparation, GuardLocationTaskRole,
    },
    ordinary_type47_death_live::{
        TYPE47_COMMON_DYING_BEHAVIOR_CHOICES, TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR,
        TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR,
    },
    ordinary_type47_live::{
        FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY, FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID,
        FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES,
        FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT,
    },
    ordinary_type9_wander_owner::OrdinaryType9WanderTaskState,
    sub_h_external_frame::SubHRuntimeState,
    world_fx::WorldFx,
};

pub const TYPE47_GUARD_BEHAVIOR_CLASS_ID: u8 = 32;
pub const TYPE47_WANDER_BEHAVIOR_CLASS_ID: u8 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Type47ReselectionCohort {
    FreshLevel1,
    Intro2,
    NativeConstruction,
}

/// Retained allocation provenance for the shared Type-47 hit/death path.
/// Intro2 uses the same Section-12 body and generic callbacks, but its moving
/// allocation must retain the authenticated emitter/Sub-D custody. A matching
/// intro spawn number or model alone never grants that ownership.
pub(crate) fn live_type47_cohort(entity: &Entity) -> Option<Type47ReselectionCohort> {
    if let Some(receipt) = entity.native_type47_construction.as_ref() {
        return receipt
            .entity_authenticates(entity)
            .then_some(Type47ReselectionCohort::NativeConstruction);
    }
    if crate::intro2_type47_live::intro2_type47_cohort_runtime_authenticates(entity) {
        Some(Type47ReselectionCohort::Intro2)
    } else if entity.entity_type == 47
        && entity
            .authored_spawn_index
            .is_some_and(|spawn| FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES.contains(&spawn))
    {
        Some(Type47ReselectionCohort::FreshLevel1)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type47GenericConstructorEvidence {
    pub random_sample_low16: u16,
    pub sub_a_target_speed_raw: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type47InitializerFailure {
    pub phase_index: usize,
    pub slot: ActorTaskSlot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshType47InitializerPublication {
    GuardLocation {
        constructors_by_phase: [Type47GenericConstructorEvidence; 2],
    },
    WanderNear {
        constructor: Type47GenericConstructorEvidence,
    },
    InitializerFallback {
        failure: Type47InitializerFailure,
        constructors_by_phase: [Option<Type47GenericConstructorEvidence>; 2],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshType47WeightedSelection {
    pub selection: BehaviorSelection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshType47InitialBehaviorPublication {
    pub entity_id: u32,
    pub authored_spawn_index: usize,
    pub weighted: FreshType47WeightedSelection,
    pub initializer: FreshType47InitializerPublication,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreshType47InitialBehaviorError {
    EntityInactive,
    MissingAuthoredSpawnIndex,
    UnsupportedAuthoredSpawnIndex { actual: usize },
    UnexpectedEntityType { actual: u32 },
    UnexpectedModelSlots { actual: [Option<usize>; 4] },
    UnexpectedActiveModel { actual: Option<usize> },
    InitialBehaviorAlreadyResolved,
    CurrentBehaviorContextAlreadyResolved,
    CurrentBehaviorContextUnresolved,
    ActorCommonAxisDescriptorUnresolved,
    UnexpectedActorCommonAxisDescriptor,
    ComponentTopologyUnresolved,
    UnexpectedComponentTopology,
    MissingInitializer,
    UnexpectedMetadataCommonAxisDescriptor,
    UnexpectedBehaviorChoices,
    SubADescriptorUnresolved,
    SubADescriptorAbsent,
    UnexpectedSubATargetSpeedBase { actual: i16 },
    SubARuntimeUnresolved,
    SubARuntimeAbsent,
    SubHDescriptorUnresolved,
    SubHDescriptorAbsent,
    UnexpectedSubHDescriptorRecordCount { actual: usize },
    SubHRuntimeUnresolved,
    SubHRuntimeAbsent,
    UnexpectedSubHRuntimeRecordCount { actual: usize },
    BirthTaskTableNotEmpty,
    SelectedInitializerResolutionMismatch,
    CanonicalFreshContextUnavailable,
    UnexpectedSelectedClass { actual: u8 },
    Plan(BehaviorSelectionError),
}

/// Validate the exact model-302 publication surface and consume one selector
/// word. No entity state changes before this returns successfully.
pub fn plan_fresh_type47_initial_behavior(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
) -> Result<FreshType47WeightedSelection, FreshType47InitialBehaviorError> {
    validate_fresh_publication_surface(entity, metadata)?;
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(FreshType47InitialBehaviorError::MissingInitializer)?;
    let selection = select_initial_behavior(
        &initializer.behavior_choices,
        |rule| i32::from(rule == BehaviorWeightRule::Always),
        || u32::from(world_fx.next_shared_retail_random_u16()),
    )
    .map_err(FreshType47InitialBehaviorError::Plan)?
    .ok_or(FreshType47InitialBehaviorError::Plan(
        BehaviorSelectionError::UnknownBehaviorClass { raw: 0 },
    ))?;
    if !matches!(
        selection.program.class_id,
        TYPE47_GUARD_BEHAVIOR_CLASS_ID | TYPE47_WANDER_BEHAVIOR_CLASS_ID
    ) {
        return Err(FreshType47InitialBehaviorError::UnexpectedSelectedClass {
            actual: selection.program.class_id,
        });
    }
    Ok(FreshType47WeightedSelection { selection })
}

/// Publish the selected class-32 or class-6 initializer, or the unnamed
/// outer fallback. Production context construction is infallible once the
/// selector has been authenticated; only task allocation remains fallible.
/// Publish class-32 variant-0 Guard or class-6 Wander after C690 reselection.
///
/// Uses `FUN_0040ABB0`'s TypeDefault replacement: preserve target `+0x08` and
/// auxiliary `+0x0C`. This is the initial style, not Pursuing ADE0.
pub(crate) fn publish_type47_reselected_initial_behavior(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    cohort: Type47ReselectionCohort,
    selection: BehaviorSelection,
    world_fx: &mut WorldFx,
) -> Result<FreshType47InitializerPublication, FreshType47InitialBehaviorError> {
    validate_common_publication_surface_allowing_live_tasks(entity, metadata, cohort)?;
    let RetailRuntimeValue::Known(Some(current)) = entity.current_behavior_context else {
        return Err(FreshType47InitialBehaviorError::CurrentBehaviorContextUnresolved);
    };
    let selected_context = current
        .reselect_named_type_default(
            selection.program,
            selection.program.initial_style_table_index_raw,
            selection.program.initial_style,
        )
        .ok_or(FreshType47InitialBehaviorError::CanonicalFreshContextUnavailable)?;
    Ok(match selection.program.class_id {
        TYPE47_GUARD_BEHAVIOR_CLASS_ID => {
            publish_type47_guard_location(entity, selected_context, metadata, &mut || {
                u32::from(world_fx.next_shared_retail_random_u16())
            })
        }
        TYPE47_WANDER_BEHAVIOR_CLASS_ID => {
            publish_type47_wander_near(entity, selected_context, metadata, &mut || {
                u32::from(world_fx.next_shared_retail_random_u16())
            })
        }
        actual => {
            return Err(FreshType47InitialBehaviorError::UnexpectedSelectedClass { actual });
        }
    })
}

pub fn publish_fresh_type47_initial_behavior(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    weighted: FreshType47WeightedSelection,
    world_fx: &mut WorldFx,
) -> Result<FreshType47InitialBehaviorPublication, FreshType47InitialBehaviorError> {
    validate_selected_publication_surface(entity, metadata, weighted.selection)?;
    let selected_context =
        BehaviorContextRuntime::from_fresh_weighted_selection(weighted.selection)
            .ok_or(FreshType47InitialBehaviorError::CanonicalFreshContextUnavailable)?;
    let initializer = match weighted.selection.program.class_id {
        TYPE47_GUARD_BEHAVIOR_CLASS_ID => {
            publish_type47_guard_location(entity, selected_context, metadata, &mut || {
                u32::from(world_fx.next_shared_retail_random_u16())
            })
        }
        TYPE47_WANDER_BEHAVIOR_CLASS_ID => {
            publish_type47_wander_near(entity, selected_context, metadata, &mut || {
                u32::from(world_fx.next_shared_retail_random_u16())
            })
        }
        actual => return Err(FreshType47InitialBehaviorError::UnexpectedSelectedClass { actual }),
    };

    Ok(FreshType47InitialBehaviorPublication {
        entity_id: entity.id,
        authored_spawn_index: entity
            .authored_spawn_index
            .expect("fresh publication preflight retained the authored spawn"),
        weighted,
        initializer,
    })
}

pub(crate) fn publish_type47_guard_location(
    entity: &mut Entity,
    selected_context: BehaviorContextRuntime,
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> FreshType47InitializerPublication {
    publish_selected_context(entity, selected_context);
    restore_type_authored_common_axis_filter(entity, metadata);

    let authored_primary_context_word = metadata
        .initializer
        .as_ref()
        .expect("preflight authenticated the type initializer")
        .common_axis_descriptor
        .raw_word_at_0x04;
    let RetailRuntimeValue::Known(0) = behavior_program(u32::from(TYPE47_GUARD_BEHAVIOR_CLASS_ID))
        .expect("Guard Location is an authored named program")
        .initializer_argument_raw
    else {
        unreachable!("Guard Location style +0x44 is the EXE-proven zero word")
    };

    let owner_position_raw = entity.position_raw();
    let mut constructors_by_phase = [None; 2];
    let mut primary_context = GuardLocationPrimaryContext::new(0);
    let setup_result: Result<(), GuardLocationSetupError<Infallible>> = {
        let Entity {
            actor_tasks,
            sub_a_propulsion_runtime,
            sub_h_external_frame_runtime,
            ..
        } = entity;
        let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
            unreachable!("live preflight retained exact Sub-A storage")
        };
        let RetailRuntimeValue::Known(Some(sub_h)) = sub_h_external_frame_runtime else {
            unreachable!("live preflight retained exact Sub-H storage")
        };
        apply_guard_location_setup(
            actor_tasks,
            &mut primary_context,
            GuardLocationSetupRequest {
                authored_primary_context_word,
                search_task_context_word: 0,
            },
            |preparation| {
                let phase_index = preparation.phase_index;
                let prepared =
                    prepare_type47_guard_runtime_task(preparation, owner_position_raw, metadata)
                        .expect("authenticated type-47 Guard metadata must prepare both phases");
                Ok(prepared.apply_suffix(
                    || next_random(),
                    |effect| {
                        apply_type47_constructor_effect(
                            effect,
                            sub_h,
                            sub_a,
                            &mut constructors_by_phase[phase_index],
                        )
                    },
                ))
            },
        )
    };

    match setup_result {
        Ok(()) => FreshType47InitializerPublication::GuardLocation {
            constructors_by_phase: constructors_by_phase.map(|evidence| {
                evidence.expect("successful Guard setup applied both constructor suffixes")
            }),
        },
        Err(error) => {
            entity.publish_behavior_initializer_failure_fallback(selected_context);
            FreshType47InitializerPublication::InitializerFallback {
                failure: Type47InitializerFailure {
                    phase_index: error.phase_index,
                    slot: error.slot,
                },
                constructors_by_phase,
            }
        }
    }
}

pub(crate) fn publish_type47_wander_near(
    entity: &mut Entity,
    selected_context: BehaviorContextRuntime,
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> FreshType47InitializerPublication {
    publish_selected_context(entity, selected_context);

    let owner_position_raw = entity.position_raw();
    let mut constructors_by_phase = [None; 2];
    let setup_result: Result<(), Type47InitializerFailure> = {
        let Entity {
            actor_tasks,
            sub_a_propulsion_runtime,
            sub_h_external_frame_runtime,
            ..
        } = entity;
        let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
            unreachable!("live preflight retained exact Sub-A storage")
        };
        let RetailRuntimeValue::Known(Some(sub_h)) = sub_h_external_frame_runtime else {
            unreachable!("live preflight retained exact Sub-H storage")
        };
        actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
        actor_tasks.clear_slot(ActorTaskSlot::Secondary);
        let prepared = prepare_type47_wander_runtime_task(owner_position_raw, metadata)
            .expect("authenticated type-47 Wander metadata must prepare the Primary");
        let published = prepared.apply_suffix(
            || next_random(),
            |effect| {
                apply_type47_constructor_effect(effect, sub_h, sub_a, &mut constructors_by_phase[0])
            },
        );
        actor_tasks.replace_prepared(ActorTaskSlot::Primary, published);
        Ok(())
    };

    match setup_result {
        Ok(()) => FreshType47InitializerPublication::WanderNear {
            constructor: constructors_by_phase[0]
                .expect("successful Wander setup applied its constructor suffix"),
        },
        Err(failure) => {
            entity.publish_behavior_initializer_failure_fallback(selected_context);
            FreshType47InitializerPublication::InitializerFallback {
                failure,
                constructors_by_phase,
            }
        }
    }
}

pub(crate) fn publish_selected_context(
    entity: &mut Entity,
    selected_context: BehaviorContextRuntime,
) {
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(selected_context));
    let program = match selected_context.descriptor() {
        crate::entity_behavior::BehaviorDescriptorIdentity::Named(program) => program,
        crate::entity_behavior::BehaviorDescriptorIdentity::InitializerFailureFallback => {
            unreachable!("fresh type-47 publication starts from a named program")
        }
    };
    let policy = initial_behavior_state_policy(program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
}

pub(crate) fn restore_type_authored_common_axis_filter(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
) {
    let RetailRuntimeValue::Known(mut actor_common_axis_descriptor) =
        entity.actor_common_axis_descriptor
    else {
        unreachable!("preflight retained exact actor-local common-axis storage")
    };
    actor_common_axis_descriptor.raw_word_at_0x04 = metadata
        .initializer
        .as_ref()
        .expect("preflight authenticated the type initializer")
        .common_axis_descriptor
        .raw_word_at_0x04;
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(actor_common_axis_descriptor);
}

pub(crate) fn prepare_type47_guard_runtime_task(
    preparation: GuardLocationTaskPreparation,
    owner_position_raw: [i16; 3],
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<PreparedSharedGenericRuntimeTask, Infallible> {
    let allocated = match preparation.task.role {
        GuardLocationTaskRole::AcquireCandidate => {
            prepare_guard_location_acquisition_runtime_task(preparation)
                .expect("authenticated Guard phase 0 matches FUN_00401F80")
        }
        GuardLocationTaskRole::WanderAroundAnchor => {
            prepare_type47_wander_allocated_task(owner_position_raw)
        }
    };
    Ok(
        prepare_shared_generic_constructor_suffix(allocated, metadata)
            .expect("authenticated type-47 topology is Sub-H then Sub-A"),
    )
}

fn prepare_type47_wander_runtime_task(
    owner_position_raw: [i16; 3],
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<PreparedSharedGenericRuntimeTask, Infallible> {
    Ok(prepare_shared_generic_constructor_suffix(
        prepare_type47_wander_allocated_task(owner_position_raw),
        metadata,
    )
    .expect("authenticated type-47 topology is Sub-H then Sub-A"))
}

fn prepare_type47_wander_allocated_task(
    owner_position_raw: [i16; 3],
) -> crate::actor_task_owner::PreparedActorTask<ActorTaskRuntime> {
    crate::actor_task_owner::PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(
        OrdinaryType9WanderTaskState::from_allocated_anchor(owner_position_raw),
    ))
}

pub(crate) fn apply_type47_constructor_effect(
    effect: SharedGenericConstructorEffect,
    sub_h: &mut SubHRuntimeState,
    sub_a: &mut SubAPropulsionRuntime,
    constructor: &mut Option<Type47GenericConstructorEvidence>,
) {
    match effect {
        SharedGenericConstructorEffect::WriteSubHState08 { value } => {
            debug_assert_eq!(value, 1);
            sub_h.set_enabled(value != 0);
        }
        SharedGenericConstructorEffect::WriteSubADirection {
            direction_multiplier,
        } => sub_a.set_direction_multiplier(direction_multiplier),
        SharedGenericConstructorEffect::WriteSubATargetSpeed {
            target_speed_raw,
            random_sample_low16,
        } => {
            sub_a.apply_shared_initializer_target_speed_write(target_speed_raw);
            *constructor = Some(Type47GenericConstructorEvidence {
                random_sample_low16,
                sub_a_target_speed_raw: target_speed_raw,
            });
        }
    }
}

fn validate_fresh_publication_surface(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), FreshType47InitialBehaviorError> {
    validate_common_publication_surface(entity, metadata)?;
    if entity.initial_behavior != RetailRuntimeValue::Unresolved {
        return Err(FreshType47InitialBehaviorError::InitialBehaviorAlreadyResolved);
    }
    if entity.current_behavior_context != RetailRuntimeValue::Unresolved {
        return Err(FreshType47InitialBehaviorError::CurrentBehaviorContextAlreadyResolved);
    }
    Ok(())
}

fn validate_selected_publication_surface(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    expected_selection: BehaviorSelection,
) -> Result<(), FreshType47InitialBehaviorError> {
    validate_common_publication_surface(entity, metadata)?;
    if entity.initial_behavior != RetailRuntimeValue::Known(Some(expected_selection))
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
    {
        return Err(FreshType47InitialBehaviorError::SelectedInitializerResolutionMismatch);
    }
    Ok(())
}

fn validate_common_publication_surface_allowing_live_tasks(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    cohort: Type47ReselectionCohort,
) -> Result<(), FreshType47InitialBehaviorError> {
    validate_common_publication_surface_identity(entity, metadata, cohort)
}

fn validate_common_publication_surface(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), FreshType47InitialBehaviorError> {
    validate_common_publication_surface_identity(
        entity,
        metadata,
        Type47ReselectionCohort::FreshLevel1,
    )?;
    if entity_has_birth_tasks(entity) {
        return Err(FreshType47InitialBehaviorError::BirthTaskTableNotEmpty);
    }
    Ok(())
}

fn validate_common_publication_surface_identity(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    cohort: Type47ReselectionCohort,
) -> Result<(), FreshType47InitialBehaviorError> {
    if !entity.active {
        return Err(FreshType47InitialBehaviorError::EntityInactive);
    }
    let spawn_index = entity
        .authored_spawn_index
        .ok_or(FreshType47InitialBehaviorError::MissingAuthoredSpawnIndex)?;
    let spawn_admitted = match cohort {
        Type47ReselectionCohort::FreshLevel1 => {
            FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES.contains(&spawn_index)
        }
        Type47ReselectionCohort::Intro2 => {
            crate::intro2_type47_live::INTRO2_TYPE47_SPAWN_INDICES.contains(&spawn_index)
        }
        Type47ReselectionCohort::NativeConstruction => entity
            .native_type47_construction
            .as_ref()
            .is_some_and(|receipt| receipt.entity_authenticates(entity)),
    };
    if !spawn_admitted {
        return Err(
            FreshType47InitialBehaviorError::UnsupportedAuthoredSpawnIndex {
                actual: spawn_index,
            },
        );
    }
    if entity.entity_type != 47 {
        return Err(FreshType47InitialBehaviorError::UnexpectedEntityType {
            actual: entity.entity_type,
        });
    }
    let expected_model = Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID);
    if cohort != Type47ReselectionCohort::NativeConstruction
        && entity.model_slots != [expected_model; 4]
    {
        return Err(FreshType47InitialBehaviorError::UnexpectedModelSlots {
            actual: entity.model_slots,
        });
    }
    if cohort != Type47ReselectionCohort::NativeConstruction && entity.model_index != expected_model
    {
        return Err(FreshType47InitialBehaviorError::UnexpectedActiveModel {
            actual: entity.model_index,
        });
    }

    match entity.actor_common_axis_descriptor {
        RetailRuntimeValue::Known(actual)
            if actual == TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR => {}
        RetailRuntimeValue::Known(_) => {
            return Err(FreshType47InitialBehaviorError::UnexpectedActorCommonAxisDescriptor)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(FreshType47InitialBehaviorError::ActorCommonAxisDescriptorUnresolved)
        }
    }
    match metadata.common_mover_topology {
        RetailRuntimeValue::Known(actual)
            if actual == FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY => {}
        RetailRuntimeValue::Known(_) => {
            return Err(FreshType47InitialBehaviorError::UnexpectedComponentTopology)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(FreshType47InitialBehaviorError::ComponentTopologyUnresolved)
        }
    }
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(FreshType47InitialBehaviorError::MissingInitializer)?;
    if initializer.common_axis_descriptor != TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR {
        return Err(FreshType47InitialBehaviorError::UnexpectedMetadataCommonAxisDescriptor);
    }
    if initializer.behavior_choices.as_ref() != TYPE47_COMMON_DYING_BEHAVIOR_CHOICES.as_slice() {
        return Err(FreshType47InitialBehaviorError::UnexpectedBehaviorChoices);
    }
    let sub_a_descriptor = match metadata.sub_a_propulsion_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
        RetailRuntimeValue::Known(None) => {
            return Err(FreshType47InitialBehaviorError::SubADescriptorAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(FreshType47InitialBehaviorError::SubADescriptorUnresolved)
        }
    };
    if sub_a_descriptor.target_speed_base_raw
        != TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR.target_speed_base_raw
    {
        return Err(
            FreshType47InitialBehaviorError::UnexpectedSubATargetSpeedBase {
                actual: sub_a_descriptor.target_speed_base_raw,
            },
        );
    }
    match entity.sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(_)) => {}
        RetailRuntimeValue::Known(None) => {
            return Err(FreshType47InitialBehaviorError::SubARuntimeAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(FreshType47InitialBehaviorError::SubARuntimeUnresolved)
        }
    }
    let sub_h_descriptor = match &metadata.sub_h_external_frame_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
        RetailRuntimeValue::Known(None) => {
            return Err(FreshType47InitialBehaviorError::SubHDescriptorAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(FreshType47InitialBehaviorError::SubHDescriptorUnresolved)
        }
    };
    if sub_h_descriptor.records.len() != FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT {
        return Err(
            FreshType47InitialBehaviorError::UnexpectedSubHDescriptorRecordCount {
                actual: sub_h_descriptor.records.len(),
            },
        );
    }
    let sub_h_runtime = match &entity.sub_h_external_frame_runtime {
        RetailRuntimeValue::Known(Some(runtime)) => runtime,
        RetailRuntimeValue::Known(None) => {
            return Err(FreshType47InitialBehaviorError::SubHRuntimeAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(FreshType47InitialBehaviorError::SubHRuntimeUnresolved)
        }
    };
    if sub_h_runtime.records().len() != FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT {
        return Err(
            FreshType47InitialBehaviorError::UnexpectedSubHRuntimeRecordCount {
                actual: sub_h_runtime.records().len(),
            },
        );
    }
    Ok(())
}

fn entity_has_birth_tasks(entity: &Entity) -> bool {
    [
        ActorTaskSlot::Primary,
        ActorTaskSlot::Secondary,
        ActorTaskSlot::Tertiary,
    ]
    .into_iter()
    .any(|slot| entity.actor_task_state(slot).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        actor_task_dispatcher::ActorTaskRuntimeFamily,
        common_mover::shared_initializer_target_speed_raw,
        entity::EntityKind,
        entity_behavior::{ActiveBehaviorStyle, BehaviorChoiceListSource},
        entity_collision_state::{EntityInitializerSpec, RetailStateWord},
        ordinary_type47_death_live::TYPE47_COMMON_DYING_SUB_H_RECORDS,
        retail_rng::retail_random_u16,
    };
    use v2k_formats::collision::SubHExternalFrameDescriptor;

    const ENTITY_ID: u32 = 0x042F_000B;

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID as u16; 4],
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR,
            )),
            sub_h_external_frame_descriptor: RetailRuntimeValue::Known(Some(
                SubHExternalFrameDescriptor {
                    completion_sound_id: None,
                    records: TYPE47_COMMON_DYING_SUB_H_RECORDS.to_vec(),
                },
            )),
            common_mover_topology: RetailRuntimeValue::Known(
                FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY,
            ),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: 0x2039,
                common_axis_descriptor: TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR,
                behavior_choices: TYPE47_COMMON_DYING_BEHAVIOR_CHOICES
                    .to_vec()
                    .into_boxed_slice(),
                behavior_rule_ref: 0,
                alternate_behavior_class_ref: 12,
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn exact_entity(spawn_index: usize) -> Entity {
        let mut entity = Entity::unresolved_port_entity(ENTITY_ID, EntityKind::Enemy, 47);
        entity.authored_spawn_index = Some(spawn_index);
        entity.model_slots = [Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID); 4];
        entity.model_index = Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID);
        entity.actor_common_axis_descriptor =
            RetailRuntimeValue::Known(TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR);
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(777), -1, 100),
        ));
        entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(
            SubHRuntimeState::new(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT).unwrap(),
        ));
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0x2039);
        entity
    }

    fn publish_cold_stream(
        spawn_index: usize,
    ) -> (
        Entity,
        FreshType47InitialBehaviorPublication,
        WorldFx,
        u16,
        u16,
    ) {
        let metadata = exact_metadata();
        let mut entity = exact_entity(spawn_index);
        let mut world_fx = WorldFx::new();
        let weighted =
            plan_fresh_type47_initial_behavior(&entity, &metadata, &mut world_fx).unwrap();
        entity.initial_behavior = RetailRuntimeValue::Known(Some(weighted.selection));
        let publication =
            publish_fresh_type47_initial_behavior(&mut entity, &metadata, weighted, &mut world_fx)
                .unwrap();
        let mut oracle = WorldFx::new();
        let selector = oracle.next_shared_retail_random_u16();
        let first_constructor = oracle.next_shared_retail_random_u16();
        (entity, publication, world_fx, selector, first_constructor)
    }

    #[test]
    fn always_x9_selects_guard_and_publishes_both_suffixes() {
        let (entity, publication, mut world_fx, selector, first_constructor) =
            publish_cold_stream(11);
        assert_eq!(selector, 0x0026);
        assert_eq!(publication.weighted.selection.choice_index, 0);
        assert_eq!(
            publication.weighted.selection.program.class_id,
            TYPE47_GUARD_BEHAVIOR_CLASS_ID
        );
        let FreshType47InitializerPublication::GuardLocation {
            constructors_by_phase,
        } = publication.initializer
        else {
            panic!("Always x9 must publish Guard Location")
        };
        assert_eq!(
            constructors_by_phase[0],
            Type47GenericConstructorEvidence {
                random_sample_low16: first_constructor,
                sub_a_target_speed_raw: shared_initializer_target_speed_raw(
                    TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR.target_speed_base_raw,
                    first_constructor,
                ),
            }
        );
        assert_eq!(
            entity
                .actor_task_state(ActorTaskSlot::Secondary)
                .map(ActorTaskRuntime::family),
            Some(ActorTaskRuntimeFamily::GuardLocationAcquisition)
        );
        assert_eq!(
            entity
                .actor_task_state(ActorTaskSlot::Primary)
                .map(ActorTaskRuntime::family),
            Some(ActorTaskRuntimeFamily::OrdinaryType9Wander)
        );
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("Guard birth must publish its context")
        };
        assert!(matches!(
            context.active_style(),
            ActiveBehaviorStyle::Audited(style) if style.class_id == 32 && style.variant == 0
        ));
        assert_eq!(
            context.choice_list_source(),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        );
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("Guard birth must retain Sub-A")
        };
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(constructors_by_phase[1].sub_a_target_speed_raw)
        );
        let mut expected = 0;
        for _ in 0..3 {
            retail_random_u16(&mut expected);
        }
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            retail_random_u16(&mut expected)
        );
    }

    #[test]
    fn always_x1_selects_wander_from_a_high_selector_word() {
        let metadata = exact_metadata();
        let mut entity = exact_entity(13);
        let mut world_fx = WorldFx::new();
        // Advance until the selector threshold lands in the last tenth.
        // total=10, threshold = (low16 * 10) >> 16; Guard occupies 0..8.
        let mut rng_state = 0;
        let mut prefix_draws: u32 = 0;
        loop {
            let sample = retail_random_u16(&mut rng_state);
            prefix_draws += 1;
            let threshold = (u32::from(sample).wrapping_mul(10)) >> 16;
            if threshold >= 9 {
                break;
            }
        }
        for _ in 0..prefix_draws.saturating_sub(1) {
            let _ = world_fx.next_shared_retail_random_u16();
        }
        let weighted =
            plan_fresh_type47_initial_behavior(&entity, &metadata, &mut world_fx).unwrap();
        assert_eq!(weighted.selection.choice_index, 1);
        assert_eq!(
            weighted.selection.program.class_id,
            TYPE47_WANDER_BEHAVIOR_CLASS_ID
        );
        entity.initial_behavior = RetailRuntimeValue::Known(Some(weighted.selection));
        let publication =
            publish_fresh_type47_initial_behavior(&mut entity, &metadata, weighted, &mut world_fx)
                .unwrap();
        assert!(matches!(
            publication.initializer,
            FreshType47InitializerPublication::WanderNear { .. }
        ));
        assert_eq!(
            entity
                .actor_task_state(ActorTaskSlot::Primary)
                .map(ActorTaskRuntime::family),
            Some(ActorTaskRuntimeFamily::OrdinaryType9Wander)
        );
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Secondary), None);
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
    }

    #[test]
    fn unresolved_plus_b2_is_left_untouched() {
        let (entity, _, _, _, _) = publish_cold_stream(12);
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Unresolved
        );
    }
}
