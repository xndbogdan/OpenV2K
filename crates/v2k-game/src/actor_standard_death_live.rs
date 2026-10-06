//! Live class-12 publication at the checked-damage death callback.
//!
//! Retail `FUN_00410C10` commits health zero and the low model-selector bit,
//! then invokes type callback `FUN_0040DB80`. That callback first dispatches
//! the current style's `+0x2C` hook and, for the ordinary null-hook type-17
//! path, re-resolves the entity before selecting its Section-12 `+0x124`
//! alternate class. Full-game and demo code then run class-12 initializer
//! `FUN_0040C620` synchronously inside the same checked-damage receipt.
//!
//! This module binds that static chain to the exact four captured fresh-Level-1
//! type-17/model-256 allocations. It authenticates the public action payload
//! against the checked child's private outstanding callback before inspecting
//! live storage. Every fallible evidence check and constructor descriptor check
//! precedes behavior, task, component, velocity, or shared-RNG mutation.

use v2k_formats::collision::CommonAxisDescriptor;

use crate::{
    actor_death::COMMON_ACTOR_DYING_ACTIVE_STYLE,
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit},
    checked_damage::IssuedCheckedDamageAction,
    common_dying::{
        plan_common_dying_setup, CommonDyingComponentDescriptors, CommonDyingConstructorEffect,
        CommonDyingConstructorError, CommonDyingTaskState, PreparedCommonDyingTask,
        COMMON_DYING_TASK_LIFETIME_MS,
    },
    common_dying_live::{
        LevelOneType17CommonDyingOwner, COMMON_DYING_BEHAVIOR_CLASS_ID,
        FRESH_LEVEL1_TYPE17_COMMON_DYING_SPAWN_INDICES, TYPE17_COMMON_DYING_ENTITY_TYPE,
        TYPE17_COMMON_DYING_MODEL_ID,
    },
    common_mover::SubAPropulsionRuntime,
    entity::{raw_position_world, Entity, EntityManager, LEVEL_ONE_TYPE17_SELF_MASS_RAW},
    entity_behavior::{
        audited_behavior_program, initial_behavior_state_policy, ActiveBehaviorStyle,
        BehaviorChoiceListSource, BehaviorContextRuntime, BehaviorDescriptorIdentity,
        DeathCallbackPolicy,
    },
    entity_collision_state::{
        CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
        ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
        DEFERRED_DESTROY_PENDING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
    },
    primary_hit_checked_damage::{
        AuthenticatedCheckedDamageDeathCallback, PrimaryHitCheckedDamageCoordinator,
        PrimaryHitCheckedDamageDeathCallbackError,
    },
    sub_h_external_frame::SubHRuntimeState,
    type17_impact_live::{
        TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR, TYPE17_MODEL256_COMPONENT_TOPOLOGY,
        TYPE17_MODEL256_SUB_A_TARGET_SPEED_BASE_RAW, TYPE17_MODEL256_SUB_H_RECORD_COUNT,
    },
    type17_impact_reselection::{TYPE17_BEHAVIOR_RULE_REF, TYPE17_IMPACT_BEHAVIOR_CHOICES},
    world_fx::WorldFx,
};

pub const TYPE17_STANDARD_DEATH_CALLBACK_ADDRESS: u32 = 0x0040_DB80;
pub const TYPE17_STANDARD_DEATH_INITIAL_HEALTH_RAW: i32 = 5_000;
pub const TYPE17_STANDARD_DEATH_CAPABILITY_FLAGS: u32 = 0x0000_0008;
pub const TYPE17_STANDARD_DEATH_INITIALIZER_STATE_FLAGS_RAW: u32 = 0x0000_0039;
pub const TYPE17_STANDARD_DEATH_SOUND_ID: u16 = 94;

/// Evidence outside the checked child and manager-owned entity allocation.
#[derive(Debug, Clone, Copy)]
pub struct Type17CommonDyingPublicationRequest<'a> {
    pub retail_first_world: bool,
    pub metadata: &'a EntityTypeRuntimeMetadata,
}

/// The one process-RNG-derived write made by the exact type-17 constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17CommonDyingConstructorEvidence {
    pub random_sample_low16: u16,
    pub sub_a_target_speed_raw: i32,
}

/// Exact class-12 publication produced by the fresh generic-death path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17CommonDyingPublication {
    pub owner: LevelOneType17CommonDyingOwner,
    pub constructor: Type17CommonDyingConstructorEvidence,
}

/// Result of entering retail's generic standard-death helper for the bounded
/// fresh-Level-1 type-17 cohort.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type17StandardDeathOutcome {
    /// Remote ownership wins even when the dying selector is also set.
    RemoteOwnedNoOp,
    /// A local entity already using the dying selector is left untouched.
    AlreadyDyingNoOp,
    /// The generic prefix and exact class-12 suffix committed atomically.
    Published(Type17CommonDyingPublication),
}

/// The structural task action whose modeled preparation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17CommonDyingInitializerFailure {
    pub action_index: usize,
    pub slot: ActorTaskSlot,
}

/// Completed state of the authenticated death callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type17CommonDyingPublicationOutcome {
    Published {
        owner: LevelOneType17CommonDyingOwner,
        constructor: Type17CommonDyingConstructorEvidence,
    },
    /// Outer behavior initialization treats task preparation failure as a
    /// completed callback after publishing its terminal fallback state.
    InitializerFallbackPublished(Type17CommonDyingInitializerFailure),
}

/// Why the live callback cannot be committed without guessing or replaying.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type17CommonDyingPublicationError {
    CheckedDamage(PrimaryHitCheckedDamageDeathCallbackError),
    EntityUnavailable {
        entity_id: u32,
    },
    NotRetailFirstWorld,
    EntityInactive,
    UnauthenticatedSpawn {
        actual: Option<usize>,
    },
    UnexpectedEntityType {
        actual: u32,
    },
    UnexpectedEntityModelSlots {
        actual: [Option<usize>; 4],
    },
    UnexpectedEntityModel {
        actual: Option<usize>,
    },
    UnexpectedMetadataModelSlots {
        actual: [u16; 4],
    },
    UnexpectedMetadataCapabilityFlags {
        actual: u32,
    },
    UnexpectedEntityCapabilityFlags {
        actual: u32,
    },
    UnexpectedMetadataMass {
        actual: u16,
    },
    UnexpectedEntityMass {
        actual: u16,
    },
    UnexpectedInitialHealth {
        actual: Option<i32>,
    },
    DeathSoundMetadataMismatch,
    DeathSoundRuntimeMismatch,
    ConstructorSoundAttachmentNotExactNull,
    UnexpectedInitializerStateFlags {
        actual: u32,
    },
    DefaultStateFlagsMismatch {
        actual: RetailRuntimeValue<u32>,
    },
    UnexpectedBehaviorChoices,
    UnexpectedBehaviorRuleRef {
        actual: u32,
    },
    RemoteOwnerStateUnresolved,
    DyingStateUnresolved,
    HealthNotZero,
    ActiveModelSelectorUnresolved,
    WrongActiveModelSelector {
        actual: u32,
    },
    DeferredDestroyAlreadyPending,
    DeferredDestroyAlreadyQueued,
    CurrentBehaviorContextUnresolved,
    CurrentBehaviorContextAbsent,
    CurrentBehaviorChoiceSourceMismatch,
    CurrentBehaviorDescriptorStyleMismatch,
    CurrentBehaviorClassNotAuthoredForType17 {
        actual: u8,
    },
    CurrentBehaviorDeathCallbackPolicy(DeathCallbackPolicy),
    MissingInitializer,
    AlternateBehaviorClassMismatch {
        actual: u32,
    },
    UnexpectedCommonAxisDescriptor {
        actual: CommonAxisDescriptor,
    },
    ComponentTopologyUnresolved,
    UnexpectedComponentTopology {
        actual: CommonMoverComponentTopology,
    },
    SubADescriptorUnresolved,
    SubADescriptorAbsent,
    UnexpectedSubATargetSpeedBase {
        actual: i16,
    },
    SubHDescriptorUnresolved,
    SubHDescriptorAbsent,
    UnexpectedSubHDescriptorRecordCount {
        actual: usize,
    },
    SubARuntimeUnresolved,
    SubARuntimeAbsent,
    SubHRuntimeUnresolved,
    SubHRuntimeAbsent,
    UnexpectedSubHRuntimeRecordCount {
        actual: usize,
    },
    Constructor(CommonDyingConstructorError),
}

/// Recoverable rejection retaining the original issued action and receipt.
#[derive(Debug, PartialEq, Eq)]
pub struct Type17CommonDyingPublicationFailure {
    pub issued_action: IssuedCheckedDamageAction,
    pub error: Type17CommonDyingPublicationError,
}

#[derive(Debug)]
struct Type17CommonDyingPreflight {
    authored_spawn_index: usize,
    selected_context: BehaviorContextRuntime,
    prepared_task: PreparedCommonDyingTask<ActorTaskRuntime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Type17DeathPrefixState {
    Fresh,
    AlreadyCommitted,
}

#[derive(Debug)]
struct PreparedType17StandardDeath {
    sound_position_raw: [i16; 3],
    suffix: Type17CommonDyingPreflight,
}

/// Publish class 12 and acknowledge the same checked-damage child receipt.
pub fn publish_type17_common_dying_from_checked_death(
    coordinator: &mut PrimaryHitCheckedDamageCoordinator,
    issued_action: IssuedCheckedDamageAction,
    manager: &mut EntityManager,
    request: Type17CommonDyingPublicationRequest<'_>,
    world_fx: &mut WorldFx,
) -> Result<Type17CommonDyingPublicationOutcome, Type17CommonDyingPublicationFailure> {
    publish_type17_common_dying_from_checked_death_with_prepare(
        coordinator,
        issued_action,
        manager,
        request,
        world_fx,
        Ok,
    )
}

/// Enter the complete generic standard-death transition and synchronously
/// publish the exact fresh-Level-1 type-17 class-12 task.
///
/// The checked-damage entry above retains its private child receipt. This
/// sibling entry owns only the independently authenticated generic-death
/// prefix used by the Main Base abort sweep. Every fallible cohort, metadata,
/// context, component, and constructor check completes before health, state,
/// sound, task, component, velocity, or shared-RNG mutation.
pub fn publish_fresh_level_one_type17_standard_death(
    manager: &mut EntityManager,
    entity_id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
) -> Result<Type17StandardDeathOutcome, Type17CommonDyingPublicationError> {
    publish_fresh_level_one_type17_standard_death_with_provenance(
        manager,
        entity_id,
        metadata,
        world_fx,
        manager.is_fresh_new_game_first_world(),
    )
}

fn publish_fresh_level_one_type17_standard_death_with_provenance(
    manager: &mut EntityManager,
    entity_id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
    retail_first_world: bool,
) -> Result<Type17StandardDeathOutcome, Type17CommonDyingPublicationError> {
    let deferred_destroy_already_queued = manager
        .pending_actor_deferred_destroy_ids()
        .contains(&entity_id);
    let entity = manager
        .common_actor_dying_entity_mut(entity_id)
        .ok_or(Type17CommonDyingPublicationError::EntityUnavailable { entity_id })?;
    let authored_spawn_index =
        authenticate_type17_common_dying_owner(entity, metadata, retail_first_world)?;

    match entity
        .collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT)
    {
        RetailRuntimeValue::Known(REMOTE_OWNED_STATE_BIT) => {
            return Ok(Type17StandardDeathOutcome::RemoteOwnedNoOp);
        }
        RetailRuntimeValue::Known(0) => {}
        RetailRuntimeValue::Known(_) => unreachable!("a one-bit mask has only two values"),
        RetailRuntimeValue::Unresolved => {
            return Err(Type17CommonDyingPublicationError::RemoteOwnerStateUnresolved);
        }
    }
    match entity
        .collision
        .state_flags_at_0x08
        .masked(ACTIVE_MODEL_SLOT_LOW_STATE_BIT)
    {
        RetailRuntimeValue::Known(ACTIVE_MODEL_SLOT_LOW_STATE_BIT) => {
            return Ok(Type17StandardDeathOutcome::AlreadyDyingNoOp);
        }
        RetailRuntimeValue::Known(0) => {}
        RetailRuntimeValue::Known(_) => unreachable!("a one-bit mask has only two values"),
        RetailRuntimeValue::Unresolved => {
            return Err(Type17CommonDyingPublicationError::DyingStateUnresolved);
        }
    }

    if deferred_destroy_already_queued {
        return Err(Type17CommonDyingPublicationError::DeferredDestroyAlreadyQueued);
    }
    authenticate_fresh_type17_standard_death_surface(entity, metadata)?;
    let suffix = preflight_type17_common_dying_publication_after_owner_auth(
        entity,
        metadata,
        authored_spawn_index,
        Type17DeathPrefixState::Fresh,
    )?;
    let prepared = PreparedType17StandardDeath {
        sound_position_raw: entity.position_raw(),
        suffix,
    };
    Ok(Type17StandardDeathOutcome::Published(
        apply_prepared_type17_standard_death(entity, prepared, world_fx),
    ))
}

fn publish_type17_common_dying_from_checked_death_with_prepare(
    coordinator: &mut PrimaryHitCheckedDamageCoordinator,
    issued_action: IssuedCheckedDamageAction,
    manager: &mut EntityManager,
    request: Type17CommonDyingPublicationRequest<'_>,
    world_fx: &mut WorldFx,
    mut prepare_primary: impl FnMut(
        PreparedCommonDyingTask<ActorTaskRuntime>,
    ) -> Result<PreparedCommonDyingTask<ActorTaskRuntime>, ()>,
) -> Result<Type17CommonDyingPublicationOutcome, Type17CommonDyingPublicationFailure> {
    let authenticated = match coordinator
        .authenticate_death_callback_action(issued_action, TYPE17_STANDARD_DEATH_CALLBACK_ADDRESS)
    {
        Ok(authenticated) => authenticated,
        Err(failure) => {
            return Err(Type17CommonDyingPublicationFailure {
                issued_action: failure.issued_action,
                error: Type17CommonDyingPublicationError::CheckedDamage(failure.error),
            });
        }
    };
    let entity_id = authenticated.target_handle();
    if manager
        .pending_actor_deferred_destroy_ids()
        .contains(&entity_id)
    {
        return Err(reject_authenticated(
            authenticated,
            Type17CommonDyingPublicationError::DeferredDestroyAlreadyQueued,
        ));
    }

    let outcome = {
        let Some(entity) = manager.common_actor_dying_entity_mut(entity_id) else {
            return Err(reject_authenticated(
                authenticated,
                Type17CommonDyingPublicationError::EntityUnavailable { entity_id },
            ));
        };
        let preflight = match preflight_type17_common_dying_publication(
            entity,
            request.metadata,
            request.retail_first_world,
        ) {
            Ok(preflight) => preflight,
            Err(error) => return Err(reject_authenticated(authenticated, error)),
        };
        apply_type17_common_dying_publication(entity, preflight, world_fx, &mut prepare_primary)
    };

    authenticated.acknowledge();
    Ok(outcome)
}

fn reject_authenticated(
    authenticated: AuthenticatedCheckedDamageDeathCallback,
    error: Type17CommonDyingPublicationError,
) -> Type17CommonDyingPublicationFailure {
    Type17CommonDyingPublicationFailure {
        issued_action: authenticated.into_issued_action(),
        error,
    }
}

fn preflight_type17_common_dying_publication(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    retail_first_world: bool,
) -> Result<Type17CommonDyingPreflight, Type17CommonDyingPublicationError> {
    let authored_spawn_index =
        authenticate_type17_common_dying_owner(entity, metadata, retail_first_world)?;
    preflight_type17_common_dying_publication_after_owner_auth(
        entity,
        metadata,
        authored_spawn_index,
        Type17DeathPrefixState::AlreadyCommitted,
    )
}

fn authenticate_type17_common_dying_owner(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    retail_first_world: bool,
) -> Result<usize, Type17CommonDyingPublicationError> {
    if !retail_first_world {
        return Err(Type17CommonDyingPublicationError::NotRetailFirstWorld);
    }
    if !entity.active {
        return Err(Type17CommonDyingPublicationError::EntityInactive);
    }
    let spawn = entity.authored_spawn_index;
    let Some(authored_spawn_index) =
        spawn.filter(|spawn| FRESH_LEVEL1_TYPE17_COMMON_DYING_SPAWN_INDICES.contains(spawn))
    else {
        return Err(Type17CommonDyingPublicationError::UnauthenticatedSpawn { actual: spawn });
    };
    if entity.entity_type != TYPE17_COMMON_DYING_ENTITY_TYPE {
        return Err(Type17CommonDyingPublicationError::UnexpectedEntityType {
            actual: entity.entity_type,
        });
    }
    let expected_model = Some(TYPE17_COMMON_DYING_MODEL_ID);
    if entity.model_slots != [expected_model; 4] {
        return Err(
            Type17CommonDyingPublicationError::UnexpectedEntityModelSlots {
                actual: entity.model_slots,
            },
        );
    }
    if entity.model_index != expected_model {
        return Err(Type17CommonDyingPublicationError::UnexpectedEntityModel {
            actual: entity.model_index,
        });
    }
    let expected_metadata_models = [TYPE17_COMMON_DYING_MODEL_ID as u16; 4];
    if metadata.model_slots != expected_metadata_models {
        return Err(
            Type17CommonDyingPublicationError::UnexpectedMetadataModelSlots {
                actual: metadata.model_slots,
            },
        );
    }
    Ok(authored_spawn_index)
}

fn authenticate_fresh_type17_standard_death_surface(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type17CommonDyingPublicationError> {
    if metadata.capability_flags != TYPE17_STANDARD_DEATH_CAPABILITY_FLAGS {
        return Err(
            Type17CommonDyingPublicationError::UnexpectedMetadataCapabilityFlags {
                actual: metadata.capability_flags,
            },
        );
    }
    if entity.capability_flags != TYPE17_STANDARD_DEATH_CAPABILITY_FLAGS {
        return Err(
            Type17CommonDyingPublicationError::UnexpectedEntityCapabilityFlags {
                actual: entity.capability_flags,
            },
        );
    }
    if metadata.mass_raw != LEVEL_ONE_TYPE17_SELF_MASS_RAW {
        return Err(Type17CommonDyingPublicationError::UnexpectedMetadataMass {
            actual: metadata.mass_raw,
        });
    }
    if entity.mass_raw != LEVEL_ONE_TYPE17_SELF_MASS_RAW {
        return Err(Type17CommonDyingPublicationError::UnexpectedEntityMass {
            actual: entity.mass_raw,
        });
    }
    if metadata.initial_health_raw != Some(TYPE17_STANDARD_DEATH_INITIAL_HEALTH_RAW) {
        return Err(Type17CommonDyingPublicationError::UnexpectedInitialHealth {
            actual: metadata.initial_health_raw,
        });
    }
    if metadata.death_sound_id != RetailRuntimeValue::Known(Some(TYPE17_STANDARD_DEATH_SOUND_ID)) {
        return Err(Type17CommonDyingPublicationError::DeathSoundMetadataMismatch);
    }
    if entity.collision.death_sound_id
        != RetailRuntimeValue::Known(Some(TYPE17_STANDARD_DEATH_SOUND_ID))
    {
        return Err(Type17CommonDyingPublicationError::DeathSoundRuntimeMismatch);
    }
    if metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None) {
        return Err(Type17CommonDyingPublicationError::ConstructorSoundAttachmentNotExactNull);
    }
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(Type17CommonDyingPublicationError::MissingInitializer)?;
    if initializer.initializer_state_flags_raw != TYPE17_STANDARD_DEATH_INITIALIZER_STATE_FLAGS_RAW
    {
        return Err(
            Type17CommonDyingPublicationError::UnexpectedInitializerStateFlags {
                actual: initializer.initializer_state_flags_raw,
            },
        );
    }
    if entity.collision.default_state_flags_at_0xc8
        != RetailRuntimeValue::Known(TYPE17_STANDARD_DEATH_INITIALIZER_STATE_FLAGS_RAW)
    {
        return Err(
            Type17CommonDyingPublicationError::DefaultStateFlagsMismatch {
                actual: entity.collision.default_state_flags_at_0xc8,
            },
        );
    }
    if initializer.behavior_choices.as_ref() != TYPE17_IMPACT_BEHAVIOR_CHOICES {
        return Err(Type17CommonDyingPublicationError::UnexpectedBehaviorChoices);
    }
    if initializer.behavior_rule_ref != TYPE17_BEHAVIOR_RULE_REF {
        return Err(
            Type17CommonDyingPublicationError::UnexpectedBehaviorRuleRef {
                actual: initializer.behavior_rule_ref,
            },
        );
    }
    Ok(())
}

fn preflight_type17_common_dying_publication_after_owner_auth(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    authored_spawn_index: usize,
    prefix_state: Type17DeathPrefixState,
) -> Result<Type17CommonDyingPreflight, Type17CommonDyingPublicationError> {
    if prefix_state == Type17DeathPrefixState::AlreadyCommitted {
        if entity.collision.health_raw != RetailRuntimeValue::Known(0) {
            return Err(Type17CommonDyingPublicationError::HealthNotZero);
        }
        let selector_mask = ACTIVE_MODEL_SLOT_LOW_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT;
        match entity.collision.state_flags_at_0x08.masked(selector_mask) {
            RetailRuntimeValue::Known(ACTIVE_MODEL_SLOT_LOW_STATE_BIT) => {}
            RetailRuntimeValue::Known(actual) => {
                return Err(Type17CommonDyingPublicationError::WrongActiveModelSelector { actual });
            }
            RetailRuntimeValue::Unresolved => {
                return Err(Type17CommonDyingPublicationError::ActiveModelSelectorUnresolved);
            }
        }
    }
    if entity
        .collision
        .state_flags_at_0x08
        .masked(DEFERRED_DESTROY_PENDING_STATE_BIT)
        != RetailRuntimeValue::Known(0)
    {
        return Err(Type17CommonDyingPublicationError::DeferredDestroyAlreadyPending);
    }

    let current_context = match entity.current_behavior_context {
        RetailRuntimeValue::Known(Some(context)) => context,
        RetailRuntimeValue::Known(None) => {
            return Err(Type17CommonDyingPublicationError::CurrentBehaviorContextAbsent);
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17CommonDyingPublicationError::CurrentBehaviorContextUnresolved);
        }
    };
    if current_context.choice_list_source()
        != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
    {
        return Err(Type17CommonDyingPublicationError::CurrentBehaviorChoiceSourceMismatch);
    }
    match (current_context.descriptor(), current_context.active_style()) {
        (BehaviorDescriptorIdentity::Named(program), ActiveBehaviorStyle::Audited(style)) => {
            if !matches!(program.class_id, 9 | 10 | 33) {
                return Err(
                    Type17CommonDyingPublicationError::CurrentBehaviorClassNotAuthoredForType17 {
                        actual: program.class_id,
                    },
                );
            }
            if style.class_id != program.class_id {
                return Err(
                    Type17CommonDyingPublicationError::CurrentBehaviorDescriptorStyleMismatch,
                );
            }
        }
        (
            BehaviorDescriptorIdentity::InitializerFailureFallback,
            ActiveBehaviorStyle::InitializerFailureFallback,
        ) => {}
        _ => {
            return Err(Type17CommonDyingPublicationError::CurrentBehaviorDescriptorStyleMismatch);
        }
    }
    let death_policy = current_context.active_style().death_callback_policy();
    if death_policy != DeathCallbackPolicy::None {
        return Err(
            Type17CommonDyingPublicationError::CurrentBehaviorDeathCallbackPolicy(death_policy),
        );
    }

    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(Type17CommonDyingPublicationError::MissingInitializer)?;
    if initializer.alternate_behavior_class_ref != COMMON_DYING_BEHAVIOR_CLASS_ID {
        return Err(
            Type17CommonDyingPublicationError::AlternateBehaviorClassMismatch {
                actual: initializer.alternate_behavior_class_ref,
            },
        );
    }
    if initializer.common_axis_descriptor != TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR {
        return Err(
            Type17CommonDyingPublicationError::UnexpectedCommonAxisDescriptor {
                actual: initializer.common_axis_descriptor,
            },
        );
    }
    let topology = match metadata.common_mover_topology {
        RetailRuntimeValue::Known(topology) => topology,
        RetailRuntimeValue::Unresolved => {
            return Err(Type17CommonDyingPublicationError::ComponentTopologyUnresolved);
        }
    };
    if topology != TYPE17_MODEL256_COMPONENT_TOPOLOGY {
        return Err(
            Type17CommonDyingPublicationError::UnexpectedComponentTopology { actual: topology },
        );
    }
    let sub_a_descriptor = match metadata.sub_a_propulsion_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
        RetailRuntimeValue::Known(None) => {
            return Err(Type17CommonDyingPublicationError::SubADescriptorAbsent);
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17CommonDyingPublicationError::SubADescriptorUnresolved);
        }
    };
    if sub_a_descriptor.target_speed_base_raw != TYPE17_MODEL256_SUB_A_TARGET_SPEED_BASE_RAW {
        return Err(
            Type17CommonDyingPublicationError::UnexpectedSubATargetSpeedBase {
                actual: sub_a_descriptor.target_speed_base_raw,
            },
        );
    }
    let sub_h_descriptor = match &metadata.sub_h_external_frame_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
        RetailRuntimeValue::Known(None) => {
            return Err(Type17CommonDyingPublicationError::SubHDescriptorAbsent);
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17CommonDyingPublicationError::SubHDescriptorUnresolved);
        }
    };
    if sub_h_descriptor.records.len() != TYPE17_MODEL256_SUB_H_RECORD_COUNT {
        return Err(
            Type17CommonDyingPublicationError::UnexpectedSubHDescriptorRecordCount {
                actual: sub_h_descriptor.records.len(),
            },
        );
    }
    match entity.sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(_)) => {}
        RetailRuntimeValue::Known(None) => {
            return Err(Type17CommonDyingPublicationError::SubARuntimeAbsent);
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17CommonDyingPublicationError::SubARuntimeUnresolved);
        }
    }
    let sub_h_runtime = match &entity.sub_h_external_frame_runtime {
        RetailRuntimeValue::Known(Some(runtime)) => runtime,
        RetailRuntimeValue::Known(None) => {
            return Err(Type17CommonDyingPublicationError::SubHRuntimeAbsent);
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17CommonDyingPublicationError::SubHRuntimeUnresolved);
        }
    };
    if sub_h_runtime.records().len() != TYPE17_MODEL256_SUB_H_RECORD_COUNT {
        return Err(
            Type17CommonDyingPublicationError::UnexpectedSubHRuntimeRecordCount {
                actual: sub_h_runtime.records().len(),
            },
        );
    }

    let common_dying_program = audited_behavior_program(COMMON_DYING_BEHAVIOR_CLASS_ID)
        .expect("class-12 alternate behavior program is statically audited");
    let selected_context = current_context
        .reselect_audited_type_default(common_dying_program, 0, COMMON_ACTOR_DYING_ACTIVE_STYLE)
        .expect("class-12 variant zero is an exact audited style");
    let prepared_task = CommonDyingTaskState::prepare_after_allocation(
        entity.id,
        metadata,
        CommonDyingComponentDescriptors::default(),
    )
    .map_err(Type17CommonDyingPublicationError::Constructor)?
    .map_task(ActorTaskRuntime::CommonDying);

    Ok(Type17CommonDyingPreflight {
        authored_spawn_index,
        selected_context,
        prepared_task,
    })
}

fn apply_prepared_type17_standard_death(
    entity: &mut Entity,
    prepared: PreparedType17StandardDeath,
    world_fx: &mut WorldFx,
) -> Type17CommonDyingPublication {
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity.collision.state_flags_at_0x08.overwrite(
        ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
        ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
    );
    world_fx.queue_fixed_positional_sound_raw(
        TYPE17_STANDARD_DEATH_SOUND_ID,
        prepared.sound_position_raw,
    );
    // Section-12 +0xB4 was authenticated as an exact null before the prefix,
    // so retail's intervening release call is an exact no-op.
    match apply_type17_common_dying_publication(entity, prepared.suffix, world_fx, &mut |task| {
        Ok(task)
    }) {
        Type17CommonDyingPublicationOutcome::Published { owner, constructor } => {
            Type17CommonDyingPublication { owner, constructor }
        }
        Type17CommonDyingPublicationOutcome::InitializerFallbackPublished(_) => {
            unreachable!("the prepared production task has no fallible allocation step")
        }
    }
}

fn apply_type17_common_dying_publication(
    entity: &mut Entity,
    preflight: Type17CommonDyingPreflight,
    world_fx: &mut WorldFx,
    prepare_primary: &mut impl FnMut(
        PreparedCommonDyingTask<ActorTaskRuntime>,
    ) -> Result<PreparedCommonDyingTask<ActorTaskRuntime>, ()>,
) -> Type17CommonDyingPublicationOutcome {
    let selected_context = preflight.selected_context;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(selected_context));
    let program = audited_behavior_program(COMMON_DYING_BEHAVIOR_CLASS_ID)
        .expect("class-12 alternate behavior program is statically audited");
    let policy = initial_behavior_state_policy(program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);

    let mut prepared_task = Some(preflight.prepared_task);
    let mut constructor = None;
    let mut velocity_raw = entity.velocity_raw();
    let setup = {
        let Entity {
            actor_tasks,
            velocity,
            sub_a_propulsion_runtime,
            sub_h_external_frame_runtime,
            ..
        } = entity;
        let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
            unreachable!("preflight retained exact live Sub-A storage")
        };
        let RetailRuntimeValue::Known(Some(sub_h)) = sub_h_external_frame_runtime else {
            unreachable!("preflight retained exact live Sub-H storage")
        };
        plan_common_dying_setup(entity.id).apply(
            actor_tasks,
            || u32::from(world_fx.next_shared_retail_random_u16()),
            |effect| {
                apply_type17_common_dying_constructor_effect(
                    effect,
                    sub_h,
                    sub_a,
                    velocity,
                    &mut velocity_raw,
                    &mut constructor,
                )
            },
            |specification| {
                debug_assert_eq!(specification.owner_entity_id(), entity.id);
                debug_assert_eq!(specification.lifetime_ms(), COMMON_DYING_TASK_LIFETIME_MS);
                prepare_primary(
                    prepared_task
                        .take()
                        .expect("the exact setup plan prepares primary once"),
                )
            },
        )
    };

    match setup {
        Ok(()) => {
            // The lethal F590 publication runs in the later particle pass,
            // after this entity's normal FUN_00412DA0 visit. Every admitted
            // local visit either clears +0xB2 at its callback gate or after
            // callback return; the accepted Type-17 captures likewise retain
            // +0xB2 == 0. Publish that boundary fact without pretending the
            // allocator-indeterminate constructor field was authored zero.
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
            let task_id = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .expect("successful class-12 setup publishes primary last");
            let visit = ActorTaskVisit {
                slot: ActorTaskSlot::Primary,
                task_id,
            };
            let owner = LevelOneType17CommonDyingOwner::from_authenticated_publication(
                entity.id,
                preflight.authored_spawn_index,
                visit,
            );
            Type17CommonDyingPublicationOutcome::Published {
                owner,
                constructor: constructor
                    .expect("exact type-17 Sub-A topology consumes one constructor draw"),
            }
        }
        Err(error) => {
            entity.publish_behavior_initializer_failure_fallback(selected_context);
            Type17CommonDyingPublicationOutcome::InitializerFallbackPublished(
                Type17CommonDyingInitializerFailure {
                    action_index: error.action_index,
                    slot: error.slot,
                },
            )
        }
    }
}

fn apply_type17_common_dying_constructor_effect(
    effect: CommonDyingConstructorEffect,
    sub_h: &mut SubHRuntimeState,
    sub_a: &mut SubAPropulsionRuntime,
    velocity: &mut [f32; 3],
    velocity_raw: &mut [i16; 3],
    constructor: &mut Option<Type17CommonDyingConstructorEvidence>,
) {
    match effect {
        CommonDyingConstructorEffect::WriteSubHState08 { value } => {
            sub_h.set_enabled(value != 0);
        }
        CommonDyingConstructorEffect::WriteSubADirection {
            direction_multiplier,
        } => sub_a.set_direction_multiplier(direction_multiplier),
        CommonDyingConstructorEffect::WriteSubATargetSpeed {
            target_speed_raw,
            random_sample_low16,
        } => {
            sub_a.apply_shared_initializer_target_speed_write(target_speed_raw);
            *constructor = Some(Type17CommonDyingConstructorEvidence {
                random_sample_low16,
                sub_a_target_speed_raw: target_speed_raw,
            });
        }
        CommonDyingConstructorEffect::WriteOwnerVerticalVelocity { velocity_raw: y } => {
            velocity_raw[1] = y;
            *velocity = raw_position_world(*velocity_raw);
        }
        CommonDyingConstructorEffect::WriteSubGMode3f { .. }
        | CommonDyingConstructorEffect::WriteSubGRandomizedTarget38 { .. }
        | CommonDyingConstructorEffect::ResetSubGModeAndAccumulatorThenWriteSource { .. }
        | CommonDyingConstructorEffect::WriteSubGPhase3c { .. }
        | CommonDyingConstructorEffect::WriteSubFPhase3c { .. }
        | CommonDyingConstructorEffect::ResetSubFModeAndAccumulator { .. } => {
            unreachable!("exact type-17 A/B/C/D/H/J topology has no Sub-F/Sub-G effects")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        actor_task_owner::PreparedActorTask,
        checked_damage::{
            CachedTargetWriteObservation, CheckedDamageAction, CheckedDamageAdmissionTarget,
            CheckedDamageFilterBinding, CheckedDamageMachine, CheckedDamagePhase,
            CheckedDamagePoll, CheckedDamageRequest, CheckedDamageResume,
            CheckedDamageTransactionId, DeathAttachmentObservation, DeathCallbackObservation,
            DeathEntryObservation, DeathEntryTarget, DeathSoundObservation,
            GenericDamageEntryObservation, GenericDamageEntryTarget, GenericHealthObservation,
            GenericHitCallbackObservation, GenericHitSoundObservation,
        },
        damage::{DamageDeliveryRecord, DamagePacket, DamageProfile, DAMAGE_CHANNEL_COUNT},
        entity_behavior::{audited_behavior_style, behavior_program},
        entity_collision_state::{
            EntityInitializerSpec, RetailStateWord, CHECKED_DAMAGE_ENABLED_STATE_BIT,
        },
    };
    use v2k_formats::{
        collision::{
            SubAPropulsionDescriptor, SubCLiftDescriptor, SubHExternalFrameDescriptor,
            SubHExternalFrameRecord,
        },
        levels::{EntitySpawn, LevelDescriptor},
    };

    const ENTITY_TYPE: usize = TYPE17_COMMON_DYING_ENTITY_TYPE as usize;
    // Zero is null; this first-world fixture inserts the persistent player
    // before allocating its authored actors.
    const TARGET_HANDLE: u32 = FRESH_LEVEL1_TYPE17_COMMON_DYING_SPAWN_INDICES[0] as u32 + 2;
    const TARGET_CONTEXT_HANDLE: u32 = 0x047F_0001;
    const AUXILIARY_WORD: u32 = 0x1357_9BDF;
    const GENERIC_ALLOCATION: u64 = 0x6000;
    const GENERIC_TYPE: u64 = 0x7000;
    const DEATH_ALLOCATION: u64 = 0x8000;
    const DEATH_TYPE: u64 = 0x9000;

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [TYPE17_COMMON_DYING_MODEL_ID as u16; 4],
            initial_health_raw: Some(5_000),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: 0,
                common_axis_descriptor: TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR,
                behavior_choices: Box::new([]),
                behavior_rule_ref: 0,
                alternate_behavior_class_ref: COMMON_DYING_BEHAVIOR_CLASS_ID,
            }),
            common_mover_topology: RetailRuntimeValue::Known(TYPE17_MODEL256_COMPONENT_TOPOLOGY),
            // 09A80 derives H+0C from C+0C even when this later-death fixture
            // never executes Sub-C. Retain the authored terrain-only policy.
            sub_c_lift_descriptor: RetailRuntimeValue::Known(Some(SubCLiftDescriptor {
                base_clearance_raw: 75,
                lift_range_raw: 75,
                strength_raw: 0x300000,
                near_boost_range_raw: 100,
                damping_range_raw: 200,
                surface_mode_raw: 0,
                offset_sample_raw: 0,
                reserved_at_0x0e: [0, 0],
            })),
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                SubAPropulsionDescriptor {
                    acceleration_raw: 100,
                    overspeed_correction_raw: 200,
                    target_speed_base_raw: TYPE17_MODEL256_SUB_A_TARGET_SPEED_BASE_RAW,
                },
            )),
            sub_h_external_frame_descriptor: RetailRuntimeValue::Known(Some(
                SubHExternalFrameDescriptor {
                    completion_sound_id: None,
                    records: vec![
                        SubHExternalFrameRecord {
                            resolver_flags_raw: 0,
                            phase_rate_raw: 0,
                            vertex_refs: [0; 3],
                            axis_mode_raw: 0,
                            dependencies: [0; 4],
                        };
                        TYPE17_MODEL256_SUB_H_RECORD_COUNT
                    ],
                },
            )),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn exact_fresh_standard_death_metadata() -> EntityTypeRuntimeMetadata {
        let mut metadata = exact_metadata();
        metadata.mass_raw = LEVEL_ONE_TYPE17_SELF_MASS_RAW;
        metadata.capability_flags = TYPE17_STANDARD_DEATH_CAPABILITY_FLAGS;
        metadata.initial_health_raw = Some(TYPE17_STANDARD_DEATH_INITIAL_HEALTH_RAW);
        metadata.accepted_hit_presentation_sound_id = RetailRuntimeValue::Known(Some(84));
        metadata.death_sound_id = RetailRuntimeValue::Known(Some(TYPE17_STANDARD_DEATH_SOUND_ID));
        metadata.constructor_sound_attachment_id = RetailRuntimeValue::Known(None);
        metadata.generic_hit_sound_id = RetailRuntimeValue::Known(None);
        let initializer = metadata
            .initializer
            .as_mut()
            .expect("the exact Type-17 fixture has an initializer");
        initializer.initializer_state_flags_raw = TYPE17_STANDARD_DEATH_INITIALIZER_STATE_FLAGS_RAW;
        initializer.behavior_choices = TYPE17_IMPACT_BEHAVIOR_CHOICES.to_vec().into_boxed_slice();
        initializer.behavior_rule_ref = TYPE17_BEHAVIOR_RULE_REF;
        metadata
    }

    fn spawn(index: usize, entity_type: u32) -> EntitySpawn {
        EntitySpawn {
            index,
            entity_type,
            pos_data_1: [0; 4],
            pos_data_2: [0; 4],
            param: 0,
            rotation: [0; 3],
            extra: [0; 40],
            initial_damage_buffer_raw: 0,
            model_overrides: [0; 4],
            has_animation: false,
            anim_frames: 0,
            animation: None,
            has_config: false,
            config: None,
        }
    }

    fn first_world_level() -> LevelDescriptor {
        let mut entities = (0..35).map(|index| spawn(index, 0)).collect::<Vec<_>>();
        entities[6] = spawn(6, 6);
        entities[6].pos_data_1 = [0x00, 0x50, 0x00, 0x00];
        entities[6].pos_data_2[..2].copy_from_slice(&[0x00, 0x3C]);
        entities[FRESH_LEVEL1_TYPE17_COMMON_DYING_SPAWN_INDICES[0]] = spawn(
            FRESH_LEVEL1_TYPE17_COMMON_DYING_SPAWN_INDICES[0],
            TYPE17_COMMON_DYING_ENTITY_TYPE,
        );
        LevelDescriptor {
            raw_header: [0; 0xD0],
            name: String::new(),
            world_style: 0,
            terrain_sprite_base: 0,
            sky_color_index: 0,
            sky_model: 0,
            main_base_abort_sky_color_index: 0,
            main_base_abort_sky_model: 0,
            terrain_draw_depth: 0,
            sub_count: entities.len() as u32,
            campaign_record_count: 0,
            entities,
            campaign_records: Vec::new(),
        }
    }

    fn named_context(class_id: u32, variant: u8) -> BehaviorContextRuntime {
        let program = behavior_program(class_id).expect("weighted behavior program");
        let style = *audited_behavior_style(class_id, variant).expect("audited behavior style");
        BehaviorContextRuntime::named_audited(
            program,
            u32::from(variant),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(Some(TARGET_CONTEXT_HANDLE)),
            RetailRuntimeValue::Known(AUXILIARY_WORD),
            style,
        )
        .expect("coherent context fixture")
    }

    fn exact_manager(metadata: &EntityTypeRuntimeMetadata) -> EntityManager {
        let mut all_metadata = vec![EntityTypeRuntimeMetadata::default(); 47];
        all_metadata[ENTITY_TYPE] = metadata.clone();
        let level = first_world_level();
        // This fixture isolates the later death publication with deliberately
        // synthetic constructor metadata. Do not route it through the exact
        // retail birth owner, whose state/terrain/choice preflight is tested
        // separately against the real Level-1 corpus.
        let mut manager = EntityManager::from_level_with_type_metadata(&level, &all_metadata, None);
        let entity_id = manager
            .iter_all()
            .find(|entity| {
                entity.authored_spawn_index
                    == Some(FRESH_LEVEL1_TYPE17_COMMON_DYING_SPAWN_INDICES[0])
            })
            .expect("synthetic first-world type-17")
            .id;
        assert_eq!(entity_id, TARGET_HANDLE);
        let entity = manager
            .entity_mut_for_test(entity_id)
            .expect("synthetic type-17 allocation");
        assert_eq!(
            entity.select_active_model_slot(1),
            Some(TYPE17_COMMON_DYING_MODEL_ID)
        );
        entity.collision.health_raw = RetailRuntimeValue::Known(0);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(DEFERRED_DESTROY_PENDING_STATE_BIT, 0);
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(named_context(33, 1)));
        entity.set_velocity_raw([11, -22, 33]);
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Unresolved, -1, 73),
        ));
        manager
    }

    fn dummy_task(
        entity_id: u32,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> PreparedActorTask<ActorTaskRuntime> {
        CommonDyingTaskState::prepare_after_allocation(
            entity_id,
            metadata,
            CommonDyingComponentDescriptors::default(),
        )
        .unwrap()
        .map_task(ActorTaskRuntime::CommonDying)
        .apply_suffix(|| 0, |_| {})
    }

    fn issue(machine: &mut CheckedDamageMachine) -> IssuedCheckedDamageAction {
        match machine.poll() {
            CheckedDamagePoll::Action(issued) => issued,
            other => panic!("expected checked child action, got {other:?}"),
        }
    }

    fn child_at_death_callback() -> (
        PrimaryHitCheckedDamageCoordinator,
        IssuedCheckedDamageAction,
    ) {
        let profile = DamageProfile {
            thresholds_raw: [0; DAMAGE_CHANNEL_COUNT],
            multipliers_q8: [256; DAMAGE_CHANNEL_COUNT],
        };
        let mut child = CheckedDamageMachine::start(
            CheckedDamageTransactionId::new(0xC4EC_0012).unwrap(),
            CheckedDamageRequest {
                target_handle: TARGET_HANDLE,
                delivery: Some(DamageDeliveryRecord {
                    packet: DamagePacket {
                        channels: [1, 0],
                        amounts_raw: [1_000, 0],
                    },
                    source_entity_type_raw: 0x2F,
                    owner_handle: 0x04AA_0001,
                }),
                ratio_numerator: 0,
                ratio_denominator: 0,
                admission_target: RetailRuntimeValue::Known(Some(CheckedDamageAdmissionTarget {
                    target_allocation_identity: 0x5000,
                    state_flags_at_0x08: CHECKED_DAMAGE_ENABLED_STATE_BIT,
                    capability_flags_at_0x64: 0,
                    modifier_address: RetailRuntimeValue::Known(None),
                })),
                filter_binding: RetailRuntimeValue::Known(Some(CheckedDamageFilterBinding {
                    target_allocation_identity: 0x5000,
                    type_record_identity: GENERIC_TYPE,
                    profile: RetailRuntimeValue::Known(profile),
                })),
            },
        );

        loop {
            let issued = issue(&mut child);
            let completion = match issued.action {
                CheckedDamageAction::ResolveGenericDamageEntry { .. } => {
                    CheckedDamageResume::GenericDamageEntryResolved {
                        phase: CheckedDamagePhase::ResolveGenericDamageEntry,
                        observation: GenericDamageEntryObservation {
                            target: RetailRuntimeValue::Known(Some(GenericDamageEntryTarget {
                                target_allocation_identity: GENERIC_ALLOCATION,
                                state_flags_at_0x08: CHECKED_DAMAGE_ENABLED_STATE_BIT,
                                pre_health_buffer_raw: 0,
                            })),
                            type_record_identity: RetailRuntimeValue::Known(Some(GENERIC_TYPE)),
                            local_player_type_at_call: RetailRuntimeValue::Known(0x22),
                        },
                    }
                }
                CheckedDamageAction::SampleCachedGenericHitSound { .. } => {
                    CheckedDamageResume::GenericHitSoundSampled {
                        phase: CheckedDamagePhase::SampleGenericHitSound,
                        observation: RetailRuntimeValue::Known(Some(GenericHitSoundObservation {
                            target_allocation_identity: GENERIC_ALLOCATION,
                            type_record_identity: GENERIC_TYPE,
                            position_raw: [1, 2, 3],
                            sound_id: RetailRuntimeValue::Known(None),
                        })),
                    }
                }
                CheckedDamageAction::SampleCachedGenericHitCallback { .. } => {
                    CheckedDamageResume::GenericHitCallbackSampled {
                        phase: CheckedDamagePhase::SampleGenericHitCallback,
                        observation: RetailRuntimeValue::Known(Some(
                            GenericHitCallbackObservation {
                                type_record_identity: GENERIC_TYPE,
                                callback_address: RetailRuntimeValue::Known(None),
                            },
                        )),
                    }
                }
                CheckedDamageAction::SampleCachedGenericHealth { .. } => {
                    CheckedDamageResume::GenericHealthSampled {
                        phase: CheckedDamagePhase::SampleGenericHealth,
                        observation: RetailRuntimeValue::Known(Some(GenericHealthObservation {
                            target_allocation_identity: GENERIC_ALLOCATION,
                            health_raw: 500,
                        })),
                    }
                }
                CheckedDamageAction::CommitGenericHealth { .. } => {
                    CheckedDamageResume::GenericHealthCommitted {
                        phase: CheckedDamagePhase::CommitGenericHealth,
                        observation: RetailRuntimeValue::Known(Some(
                            CachedTargetWriteObservation {
                                target_allocation_identity: GENERIC_ALLOCATION,
                            },
                        )),
                    }
                }
                CheckedDamageAction::ResolveDeathEntry { .. } => {
                    CheckedDamageResume::DeathEntryResolved {
                        phase: CheckedDamagePhase::ResolveDeathEntry,
                        observation: DeathEntryObservation {
                            target: RetailRuntimeValue::Known(Some(DeathEntryTarget {
                                target_allocation_identity: DEATH_ALLOCATION,
                                state_flags_at_0x08: 0,
                            })),
                            type_record_identity: RetailRuntimeValue::Known(Some(DEATH_TYPE)),
                        },
                    }
                }
                CheckedDamageAction::CommitDeathState { .. } => {
                    CheckedDamageResume::DeathStateCommitted {
                        phase: CheckedDamagePhase::CommitDeathState,
                        observation: RetailRuntimeValue::Known(Some(
                            CachedTargetWriteObservation {
                                target_allocation_identity: DEATH_ALLOCATION,
                            },
                        )),
                    }
                }
                CheckedDamageAction::SampleCachedDeathSound { .. } => {
                    CheckedDamageResume::DeathSoundSampled {
                        phase: CheckedDamagePhase::SampleDeathSound,
                        observation: RetailRuntimeValue::Known(Some(DeathSoundObservation {
                            target_allocation_identity: DEATH_ALLOCATION,
                            type_record_identity: DEATH_TYPE,
                            position_raw: [4, 5, 6],
                            sound_id: RetailRuntimeValue::Known(None),
                        })),
                    }
                }
                CheckedDamageAction::SampleCachedDeathAttachment { .. } => {
                    CheckedDamageResume::DeathAttachmentSampled {
                        phase: CheckedDamagePhase::SampleDeathAttachment,
                        observation: RetailRuntimeValue::Known(Some(DeathAttachmentObservation {
                            target_allocation_identity: DEATH_ALLOCATION,
                            attached_resource_identity: RetailRuntimeValue::Known(None),
                        })),
                    }
                }
                CheckedDamageAction::SampleCachedDeathCallback { .. } => {
                    CheckedDamageResume::DeathCallbackSampled {
                        phase: CheckedDamagePhase::SampleDeathCallback,
                        observation: RetailRuntimeValue::Known(Some(DeathCallbackObservation {
                            type_record_identity: DEATH_TYPE,
                            callback_address: RetailRuntimeValue::Known(Some(
                                TYPE17_STANDARD_DEATH_CALLBACK_ADDRESS,
                            )),
                        })),
                    }
                }
                CheckedDamageAction::InvokeDeathCallback { .. } => {
                    let coordinator =
                        PrimaryHitCheckedDamageCoordinator::from_active_child_for_test(child);
                    return (coordinator, issued);
                }
                other => panic!("unexpected checked child action {other:?}"),
            };
            child.resume(issued.receipt, completion).unwrap();
        }
    }

    fn task_ids(entity: &Entity) -> [Option<crate::actor_task_owner::ActorTaskId>; 3] {
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot))
    }

    fn exact_fresh_standard_death_manager(
        metadata: &EntityTypeRuntimeMetadata,
        class_id: u32,
        variant: u8,
        health: RetailRuntimeValue<i32>,
    ) -> EntityManager {
        let mut manager = exact_manager(metadata);
        let entity = manager.entity_mut_for_test(TARGET_HANDLE).unwrap();
        entity.collision.health_raw = health;
        entity.collision.death_sound_id =
            RetailRuntimeValue::Known(Some(TYPE17_STANDARD_DEATH_SOUND_ID));
        entity.collision.default_state_flags_at_0xc8 =
            RetailRuntimeValue::Known(TYPE17_STANDARD_DEATH_INITIALIZER_STATE_FLAGS_RAW);
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0x0146_8825);
        entity.current_behavior_context =
            RetailRuntimeValue::Known(Some(named_context(class_id, variant)));
        entity.set_position_raw([256, -512, 768]);
        entity.set_velocity_raw([11, -22, 33]);
        manager
    }

    #[derive(Debug, Clone, PartialEq)]
    struct FreshStandardDeathSnapshot {
        collision: crate::entity_collision_state::EntityCollisionRuntimeState,
        context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
        velocity_raw: [i16; 3],
        sub_a: RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
        sub_h: RetailRuntimeValue<Option<SubHRuntimeState>>,
        task_ids: [Option<crate::actor_task_owner::ActorTaskId>; 3],
    }

    fn fresh_standard_death_snapshot(manager: &EntityManager) -> FreshStandardDeathSnapshot {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == TARGET_HANDLE)
            .unwrap();
        FreshStandardDeathSnapshot {
            collision: entity.collision.clone(),
            context: entity.current_behavior_context,
            velocity_raw: entity.velocity_raw(),
            sub_a: entity.sub_a_propulsion_runtime,
            sub_h: entity.sub_h_external_frame_runtime.clone(),
            task_ids: task_ids(entity),
        }
    }

    #[test]
    fn fresh_standard_death_publishes_captured_null_hook_contexts_once() {
        for (class_id, variant, health) in [
            (9_u32, 1_u8, RetailRuntimeValue::Unresolved),
            (33, 1, RetailRuntimeValue::Known(1_377)),
        ] {
            let metadata = exact_fresh_standard_death_metadata();
            let mut manager =
                exact_fresh_standard_death_manager(&metadata, class_id, variant, health);
            let mut world_fx = WorldFx::new();
            let mut oracle = WorldFx::new();
            let expected_sample = oracle.next_shared_retail_random_u16();
            let expected_next = oracle.next_shared_retail_random_u16();
            let expected_target = 250 + i32::from(expected_sample >> 8) * 250 / 0x0A00;

            let Type17StandardDeathOutcome::Published(publication) =
                publish_fresh_level_one_type17_standard_death_with_provenance(
                    &mut manager,
                    TARGET_HANDLE,
                    &metadata,
                    &mut world_fx,
                    true,
                )
                .unwrap()
            else {
                panic!("captured null-hook context must publish Common Dying")
            };
            assert_eq!(publication.owner.entity_id(), TARGET_HANDLE);
            assert_eq!(publication.owner.visit().slot, ActorTaskSlot::Primary);
            assert_eq!(publication.constructor.random_sample_low16, expected_sample);
            assert_eq!(
                publication.constructor.sub_a_target_speed_raw,
                expected_target
            );
            assert_eq!(world_fx.pending_event_count(), 1);
            assert_eq!(world_fx.next_shared_retail_random_u16(), expected_next);

            let entity = manager
                .iter_all()
                .find(|entity| entity.id == TARGET_HANDLE)
                .unwrap();
            assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
            assert_eq!(
                entity.collision.state_flags_at_0x08,
                RetailStateWord::exact(0x0147_C825)
            );
            assert_eq!(entity.velocity_raw(), [11, 500, 33]);
            assert_eq!(task_ids(entity)[1..], [None, None]);
            assert_eq!(task_ids(entity)[0], Some(publication.owner.visit().task_id));
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                panic!("class-12 context must be known")
            };
            assert_eq!(
                context.active_style(),
                ActiveBehaviorStyle::Audited(COMMON_ACTOR_DYING_ACTIVE_STYLE)
            );
            assert_eq!(
                context.target_handle_at_0x08(),
                RetailRuntimeValue::Known(Some(TARGET_CONTEXT_HANDLE))
            );
            assert_eq!(
                context.auxiliary_word_at_0x0c(),
                RetailRuntimeValue::Known(AUXILIARY_WORD)
            );
            assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::CommonDying(_))
            ));
            let after_first = fresh_standard_death_snapshot(&manager);

            assert_eq!(
                publish_fresh_level_one_type17_standard_death_with_provenance(
                    &mut manager,
                    TARGET_HANDLE,
                    &metadata,
                    &mut world_fx,
                    true,
                )
                .unwrap(),
                Type17StandardDeathOutcome::AlreadyDyingNoOp
            );
            assert_eq!(fresh_standard_death_snapshot(&manager), after_first);
            assert_eq!(world_fx.pending_event_count(), 1);
            world_fx.process_pending();
            assert_eq!(
                world_fx.take_positional_sounds(),
                vec![crate::world_fx::PositionalSoundEvent::fixed(
                    usize::from(TYPE17_STANDARD_DEATH_SOUND_ID),
                    [1.0, -2.0, 3.0],
                )]
            );
        }
    }

    #[test]
    fn fresh_standard_death_noops_are_atomic_and_remote_wins() {
        let mut metadata = exact_fresh_standard_death_metadata();
        metadata.mass_raw = 0;
        metadata.capability_flags = 0;
        metadata.death_sound_id = RetailRuntimeValue::Unresolved;
        metadata.constructor_sound_attachment_id = RetailRuntimeValue::Unresolved;
        metadata.common_mover_topology = RetailRuntimeValue::Unresolved;

        for (bits, expected) in [
            (
                REMOTE_OWNED_STATE_BIT | ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
                Type17StandardDeathOutcome::RemoteOwnedNoOp,
            ),
            (
                ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
                Type17StandardDeathOutcome::AlreadyDyingNoOp,
            ),
        ] {
            let mut manager = exact_fresh_standard_death_manager(
                &metadata,
                33,
                1,
                RetailRuntimeValue::Known(-27),
            );
            manager
                .entity_mut_for_test(TARGET_HANDLE)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(
                    REMOTE_OWNED_STATE_BIT | ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
                    bits,
                );
            let before = fresh_standard_death_snapshot(&manager);
            let mut world_fx = WorldFx::new();
            let mut oracle = WorldFx::new();

            assert_eq!(
                publish_fresh_level_one_type17_standard_death_with_provenance(
                    &mut manager,
                    TARGET_HANDLE,
                    &metadata,
                    &mut world_fx,
                    true,
                )
                .unwrap(),
                expected
            );
            assert_eq!(fresh_standard_death_snapshot(&manager), before);
            assert_eq!(world_fx.pending_event_count(), 0);
            assert_eq!(
                world_fx.next_shared_retail_random_u16(),
                oracle.next_shared_retail_random_u16()
            );
        }
    }

    #[test]
    fn fresh_standard_death_rejections_precede_audio_rng_and_mutation() {
        let metadata = exact_fresh_standard_death_metadata();
        let mut capture_manager =
            exact_fresh_standard_death_manager(&metadata, 9, 2, RetailRuntimeValue::Known(4_000));
        let capture_before = fresh_standard_death_snapshot(&capture_manager);
        let mut capture_fx = WorldFx::new();
        let mut capture_oracle = WorldFx::new();
        assert_eq!(
            publish_fresh_level_one_type17_standard_death_with_provenance(
                &mut capture_manager,
                TARGET_HANDLE,
                &metadata,
                &mut capture_fx,
                true,
            )
            .unwrap_err(),
            Type17CommonDyingPublicationError::CurrentBehaviorDeathCallbackPolicy(
                DeathCallbackPolicy::CapturePeopleCleanup
            )
        );
        assert_eq!(
            fresh_standard_death_snapshot(&capture_manager),
            capture_before
        );
        assert_eq!(capture_fx.pending_event_count(), 0);
        assert_eq!(
            capture_fx.next_shared_retail_random_u16(),
            capture_oracle.next_shared_retail_random_u16()
        );

        let mut malformed_metadata = metadata.clone();
        malformed_metadata.constructor_sound_attachment_id = RetailRuntimeValue::Known(Some(7));
        let mut malformed_manager =
            exact_fresh_standard_death_manager(&metadata, 33, 1, RetailRuntimeValue::Known(2_000));
        let malformed_before = fresh_standard_death_snapshot(&malformed_manager);
        let mut malformed_fx = WorldFx::new();
        assert_eq!(
            publish_fresh_level_one_type17_standard_death_with_provenance(
                &mut malformed_manager,
                TARGET_HANDLE,
                &malformed_metadata,
                &mut malformed_fx,
                true,
            )
            .unwrap_err(),
            Type17CommonDyingPublicationError::ConstructorSoundAttachmentNotExactNull
        );
        assert_eq!(
            fresh_standard_death_snapshot(&malformed_manager),
            malformed_before
        );
        assert_eq!(malformed_fx.pending_event_count(), 0);
    }

    #[test]
    fn genuine_callback_publishes_exact_class12_state_and_resumes_same_child() {
        let metadata = exact_metadata();
        let mut manager = exact_manager(&metadata);
        let (mut coordinator, issued) = child_at_death_callback();
        let transaction = issued.receipt.transaction_id();
        let sequence = issued.receipt.action_sequence();
        let mut world_fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        let expected_random = oracle.next_shared_retail_random_u16();
        let expected_next = oracle.next_shared_retail_random_u16();
        let expected_target = 250 + (i32::from(expected_random >> 8) * 250) / 0x0A00;

        let outcome = publish_type17_common_dying_from_checked_death(
            &mut coordinator,
            issued,
            &mut manager,
            Type17CommonDyingPublicationRequest {
                retail_first_world: true,
                metadata: &metadata,
            },
            &mut world_fx,
        )
        .unwrap();
        let Type17CommonDyingPublicationOutcome::Published { owner, constructor } = outcome else {
            panic!("expected successful class-12 publication")
        };
        assert_eq!(owner.entity_id(), TARGET_HANDLE);
        assert_eq!(constructor.random_sample_low16, expected_random);
        assert_eq!(constructor.sub_a_target_speed_raw, expected_target);
        assert_eq!(world_fx.next_shared_retail_random_u16(), expected_next);

        let entity = manager
            .iter_all()
            .find(|entity| entity.id == TARGET_HANDLE)
            .unwrap();
        assert_eq!(entity.velocity_raw(), [11, 500, 33]);
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0),
            "authenticated lethal publication follows the normal-owner clear"
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(
                0x0001_0000 | ACTIVE_MODEL_SLOT_LOW_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT
            ),
            RetailRuntimeValue::Known(0x0001_0000 | ACTIVE_MODEL_SLOT_LOW_STATE_BIT)
        );
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(DEFERRED_DESTROY_PENDING_STATE_BIT),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(task_ids(entity)[1..], [None, None]);
        assert_eq!(task_ids(entity)[0], Some(owner.visit().task_id));
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::CommonDying(_))
        ));
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("class-12 context must be known")
        };
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::Named(
                audited_behavior_program(COMMON_DYING_BEHAVIOR_CLASS_ID).unwrap()
            )
        );
        assert_eq!(
            context.active_style(),
            ActiveBehaviorStyle::Audited(COMMON_ACTOR_DYING_ACTIVE_STYLE)
        );
        assert_eq!(
            context.choice_list_source(),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        );
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(TARGET_CONTEXT_HANDLE))
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(AUXILIARY_WORD)
        );
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("Sub-A runtime lost")
        };
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(expected_target)
        );
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(sub_a.drive_scale_percent(), 73);
        let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime else {
            panic!("Sub-H runtime lost")
        };
        assert!(!sub_h.is_enabled());
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());

        let next = match coordinator.poll() {
            crate::primary_hit_checked_damage::PrimaryHitCheckedDamagePoll::Action(issued) => {
                issued
            }
            other => panic!("expected post-callback child action, got {other:?}"),
        };
        assert_eq!(next.receipt.transaction_id(), transaction);
        assert_eq!(next.receipt.action_sequence(), sequence + 1);
        assert!(matches!(
            next.action,
            CheckedDamageAction::SampleCachedDeathTargetAfterCallback { .. }
        ));
    }

    #[test]
    fn stale_live_evidence_returns_action_without_entity_or_rng_mutation() {
        let metadata = exact_metadata();
        let mut manager = exact_manager(&metadata);
        let entity = manager.entity_mut_for_test(TARGET_HANDLE).unwrap();
        entity.collision.health_raw = RetailRuntimeValue::Known(1);
        let context_before = entity.current_behavior_context;
        let state_before = entity.collision.state_flags_at_0x08;
        let velocity_before = entity.velocity_raw();
        let tasks_before = task_ids(entity);
        let sub_a_before = entity.sub_a_propulsion_runtime;
        let sub_h_before = entity.sub_h_external_frame_runtime.clone();
        let (mut coordinator, issued) = child_at_death_callback();
        let transaction = issued.receipt.transaction_id();
        let sequence = issued.receipt.action_sequence();
        let mut world_fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        let expected_first = oracle.next_shared_retail_random_u16();

        let failure = publish_type17_common_dying_from_checked_death(
            &mut coordinator,
            issued,
            &mut manager,
            Type17CommonDyingPublicationRequest {
                retail_first_world: true,
                metadata: &metadata,
            },
            &mut world_fx,
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            Type17CommonDyingPublicationError::HealthNotZero
        );
        assert_eq!(failure.issued_action.receipt.transaction_id(), transaction);
        assert_eq!(failure.issued_action.receipt.action_sequence(), sequence);
        assert_eq!(
            coordinator.poll(),
            crate::primary_hit_checked_damage::PrimaryHitCheckedDamagePoll::Awaiting(
                CheckedDamagePhase::DeathCallback
            )
        );
        assert_eq!(world_fx.next_shared_retail_random_u16(), expected_first);
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == TARGET_HANDLE)
            .unwrap();
        assert_eq!(entity.current_behavior_context, context_before);
        assert_eq!(entity.collision.state_flags_at_0x08, state_before);
        assert_eq!(entity.velocity_raw(), velocity_before);
        assert_eq!(task_ids(entity), tasks_before);
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before);
        assert_eq!(entity.sub_h_external_frame_runtime, sub_h_before);
    }

    #[test]
    fn modeled_primary_failure_publishes_terminal_fallback_without_rng() {
        let metadata = exact_metadata();
        let mut manager = exact_manager(&metadata);
        {
            let entity = manager.entity_mut_for_test(TARGET_HANDLE).unwrap();
            for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
                entity
                    .actor_tasks
                    .replace_prepared(slot, dummy_task(entity.id, &metadata));
            }
            entity.collision.state_flags_at_0x08 =
                RetailStateWord::exact(ACTIVE_MODEL_SLOT_LOW_STATE_BIT | 0x0006_8000);
        }
        let (mut coordinator, issued) = child_at_death_callback();
        let mut world_fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        let expected_first = oracle.next_shared_retail_random_u16();

        let outcome = publish_type17_common_dying_from_checked_death_with_prepare(
            &mut coordinator,
            issued,
            &mut manager,
            Type17CommonDyingPublicationRequest {
                retail_first_world: true,
                metadata: &metadata,
            },
            &mut world_fx,
            |_| Err(()),
        )
        .unwrap();
        assert_eq!(
            outcome,
            Type17CommonDyingPublicationOutcome::InitializerFallbackPublished(
                Type17CommonDyingInitializerFailure {
                    action_index: 2,
                    slot: ActorTaskSlot::Primary,
                }
            )
        );
        assert_eq!(world_fx.next_shared_retail_random_u16(), expected_first);
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == TARGET_HANDLE)
            .unwrap();
        assert_eq!(entity.velocity_raw(), [11, -22, 33]);
        assert_eq!(task_ids(entity), [None; 3]);
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x0006_8000),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08,
            RetailStateWord::exact(ACTIVE_MODEL_SLOT_LOW_STATE_BIT | 0x0001_0000)
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("fallback context must be known")
        };
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::InitializerFailureFallback
        );
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(TARGET_CONTEXT_HANDLE))
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(AUXILIARY_WORD)
        );
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("Sub-A runtime lost")
        };
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Unresolved);
        assert_eq!(sub_a.direction_multiplier(), -1);
        let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime else {
            panic!("Sub-H runtime lost")
        };
        assert!(sub_h.is_enabled());
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
        assert!(matches!(
            coordinator.poll(),
            crate::primary_hit_checked_damage::PrimaryHitCheckedDamagePoll::Action(
                IssuedCheckedDamageAction {
                    action: CheckedDamageAction::SampleCachedDeathTargetAfterCallback { .. },
                    ..
                }
            )
        ));
    }

    #[test]
    fn capture_cleanup_and_high_selector_are_zero_mutation_preflight_blocks() {
        let metadata = exact_metadata();
        let mut capture_manager = exact_manager(&metadata);
        capture_manager
            .entity_mut_for_test(TARGET_HANDLE)
            .unwrap()
            .current_behavior_context = RetailRuntimeValue::Known(Some(named_context(9, 2)));
        assert_eq!(
            preflight_type17_common_dying_publication(
                capture_manager
                    .iter_all()
                    .find(|entity| entity.id == TARGET_HANDLE)
                    .unwrap(),
                &metadata,
                true,
            )
            .unwrap_err(),
            Type17CommonDyingPublicationError::CurrentBehaviorDeathCallbackPolicy(
                DeathCallbackPolicy::CapturePeopleCleanup
            )
        );

        let mut selector_manager = exact_manager(&metadata);
        selector_manager
            .entity_mut_for_test(TARGET_HANDLE)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(
                ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
                ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
            );
        assert_eq!(
            preflight_type17_common_dying_publication(
                selector_manager
                    .iter_all()
                    .find(|entity| entity.id == TARGET_HANDLE)
                    .unwrap(),
                &metadata,
                true,
            )
            .unwrap_err(),
            Type17CommonDyingPublicationError::WrongActiveModelSelector {
                actual: ACTIVE_MODEL_SLOT_LOW_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
            }
        );
    }

    #[test]
    fn exact_initializer_fallback_is_a_valid_null_death_context() {
        let metadata = exact_metadata();
        let mut manager = exact_manager(&metadata);
        let entity = manager.entity_mut_for_test(TARGET_HANDLE).unwrap();
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(
            named_context(33, 1).with_initializer_failure_fallback(),
        ));
        let preflight = preflight_type17_common_dying_publication(entity, &metadata, true).unwrap();
        assert_eq!(
            preflight.selected_context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(TARGET_CONTEXT_HANDLE))
        );
        assert_eq!(
            preflight.selected_context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(AUXILIARY_WORD)
        );
        assert_eq!(
            preflight.selected_context.active_style(),
            ActiveBehaviorStyle::Audited(COMMON_ACTOR_DYING_ACTIVE_STYLE)
        );
    }
}
