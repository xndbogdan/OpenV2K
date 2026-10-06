//! Bounded standard-death and class-12 publication for ordinary
//! fresh-Level-1 type 47.
//!
//! Retail's generic death prefix commits zero health, selects the dying model
//! slot, requests sound 75, observes the authored null `+0xB4` attachment, and
//! reaches the ordinary null death hook. Matched full-game/demo code then
//! selects the Section-12 alternate class and runs `FUN_0040C620`
//! synchronously. This module owns both the complete transition and the
//! independently useful post-prefix publication for the three exact
//! type-47/model-302 Level-1 allocations.
//!
//! Every fallible metadata, live-component, context, task-wrapper, and Common-
//! Dying constructor check completes before behavior, task, component,
//! velocity, or shared-RNG mutation.  Main Base abort routing and subsequent
//! class-12 scheduling deliberately remain with their owning adapters.

use v2k_formats::collision::{
    BehaviorChoice, CommonAxisDescriptor, SubAPropulsionDescriptor, SubBLateralDescriptor,
    SubCLiftDescriptor, SubHExternalFrameRecord,
};

use crate::{
    actor_death::COMMON_ACTOR_DYING_ACTIVE_STYLE,
    actor_task_dispatcher::{ActorTaskRuntime, ActorTaskRuntimeFamily},
    actor_task_owner::{
        ActorTaskPrepareError, ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags,
    },
    common_dying::{
        plan_common_dying_setup, CommonDyingComponentDescriptors, CommonDyingConstructorEffect,
        CommonDyingConstructorError, CommonDyingTaskState, PreparedCommonDyingTask,
        COMMON_DYING_TASK_LIFETIME_MS,
    },
    common_mover::SubAPropulsionRuntime,
    entity::{raw_position_world, Entity, EntityManager},
    entity_behavior::{
        audited_behavior_program, initial_behavior_state_policy, ActiveBehaviorStyle,
        BehaviorChoiceListSource, BehaviorContextRuntime, BehaviorDescriptorIdentity,
        DeathCallbackPolicy,
    },
    entity_collision_state::{
        CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
        ACTIVE_MODEL_SLOT_LOW_STATE_BIT, DEFERRED_DESTROY_PENDING_STATE_BIT,
        REMOTE_OWNED_STATE_BIT,
    },
    ordinary_type47_live::{
        FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY, FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID,
        FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES,
        FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT,
    },
    sub_h_external_frame::SubHRuntimeState,
    world_fx::WorldFx,
};

pub const TYPE47_COMMON_DYING_ENTITY_TYPE: u32 = 47;
pub const TYPE47_COMMON_DYING_BEHAVIOR_CLASS_ID: u32 =
    COMMON_ACTOR_DYING_ACTIVE_STYLE.class_id as u32;
pub const TYPE47_COMMON_DYING_INITIAL_HEALTH_RAW: i32 = 3_000;
pub const TYPE47_COMMON_DYING_MASS_RAW: u16 = 100;
pub const TYPE47_COMMON_DYING_DRIVE_SCALE_PERCENT: i32 = 100;
pub const TYPE47_COMMON_DYING_CAPABILITY_FLAGS: u32 = 0x0000_0008;
pub const TYPE47_COMMON_DYING_INITIALIZER_STATE_FLAGS_RAW: u32 = 0x0000_2039;
pub const TYPE47_COMMON_DYING_DEATH_SOUND_ID: u16 = 75;
pub const TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 1_792,
    raw_word_at_0x04: 5,
};
pub const TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR: SubAPropulsionDescriptor =
    SubAPropulsionDescriptor {
        acceleration_raw: 1_500,
        overspeed_correction_raw: -3_000,
        target_speed_base_raw: 300,
    };
pub const TYPE47_COMMON_DYING_SUB_B_DESCRIPTOR: SubBLateralDescriptor = SubBLateralDescriptor {
    projection_threshold_rate_raw: 10_000,
    correction_rate_raw: 1_000,
};
pub const TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR: SubCLiftDescriptor = SubCLiftDescriptor {
    base_clearance_raw: 50,
    lift_range_raw: 75,
    strength_raw: 0x0030_0000,
    near_boost_range_raw: 100,
    damping_range_raw: 200,
    surface_mode_raw: 0,
    offset_sample_raw: 0,
    reserved_at_0x0e: [0; 2],
};
pub const TYPE47_COMMON_DYING_BEHAVIOR_CHOICES: [BehaviorChoice; 2] = [
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 9,
        behavior_class_id: 32,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 6,
    },
];
pub const TYPE47_COMMON_DYING_SUB_H_RECORDS: [SubHExternalFrameRecord; 6] = [
    SubHExternalFrameRecord {
        resolver_flags_raw: 0x2000_0000,
        phase_rate_raw: 0x3000_0000,
        vertex_refs: [86, 88, 94],
        axis_mode_raw: 0,
        dependencies: [1, 2, 3, 4],
    },
    SubHExternalFrameRecord {
        resolver_flags_raw: 0x2000_0000,
        phase_rate_raw: 0x3000_0000,
        vertex_refs: [87, 89, 95],
        axis_mode_raw: 0,
        dependencies: [0, 2, 3, 5],
    },
    SubHExternalFrameRecord {
        resolver_flags_raw: 0x2000_0000,
        phase_rate_raw: 0x3000_0000,
        vertex_refs: [148, 90, 96],
        axis_mode_raw: 0,
        dependencies: [0, 1, 4, 5],
    },
    SubHExternalFrameRecord {
        resolver_flags_raw: 0x2000_0000,
        phase_rate_raw: 0x3000_0000,
        vertex_refs: [149, 91, 97],
        axis_mode_raw: 0,
        dependencies: [0, 1, 4, 5],
    },
    SubHExternalFrameRecord {
        resolver_flags_raw: 0x2000_0000,
        phase_rate_raw: 0x3000_0000,
        vertex_refs: [2, 92, 98],
        axis_mode_raw: 0,
        dependencies: [0, 2, 3, 5],
    },
    SubHExternalFrameRecord {
        resolver_flags_raw: 0x2000_0000,
        phase_rate_raw: 0x3000_0000,
        vertex_refs: [3, 93, 99],
        axis_mode_raw: 0,
        dependencies: [1, 2, 3, 4],
    },
];

const TYPE47_WANDER_BEHAVIOR_CLASS_ID: u8 = 6;
const TYPE47_GUARD_BEHAVIOR_CLASS_ID: u8 = 32;

/// The one process-RNG-derived write in the exact type-47 class-12 suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type47CommonDyingConstructorEvidence {
    pub random_sample_low16: u16,
    pub sub_a_target_speed_raw: i32,
}

/// Receipt for the exact primary class-12 wrapper published by this call.
///
/// No scheduler methods live here: the production Common-Dying owner must
/// independently authenticate and adopt this receipt before ticking it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshLevelOneType47CommonDyingOwner {
    entity_id: u32,
    authored_spawn_index: usize,
    visit: ActorTaskVisit,
}

impl FreshLevelOneType47CommonDyingOwner {
    pub(crate) const fn from_authenticated_publication(
        entity_id: u32,
        authored_spawn_index: usize,
        visit: ActorTaskVisit,
    ) -> Self {
        Self {
            entity_id,
            authored_spawn_index,
            visit,
        }
    }

    pub const fn entity_id(self) -> u32 {
        self.entity_id
    }

    pub const fn authored_spawn_index(self) -> usize {
        self.authored_spawn_index
    }

    pub const fn visit(self) -> ActorTaskVisit {
        self.visit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type47CommonDyingPublication {
    pub owner: FreshLevelOneType47CommonDyingOwner,
    pub constructor: Type47CommonDyingConstructorEvidence,
}

/// Result of entering retail's generic standard-death helper for the bounded
/// fresh-Level-1 type-47 cohort.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type47StandardDeathOutcome {
    /// Remote ownership wins even when the dying selector is also set.
    RemoteOwnedNoOp,
    /// A local entity already using the dying selector is left untouched.
    AlreadyDyingNoOp,
    /// The generic prefix and exact class-12 suffix committed atomically.
    Published(Type47CommonDyingPublication),
}

/// Why the post-prefix transaction cannot be admitted without guessing.
///
/// Every public error is returned before mutation or shared-RNG consumption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type47CommonDyingPublicationError {
    NotFreshNewGameFirstWorld,
    EntityUnavailable {
        entity_id: u32,
    },
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
    HealthNotZero,
    DyingStateUnresolved,
    DyingStateNotSet,
    RemoteOwnerStateUnresolved,
    RemoteOwned,
    DeferredDestroyStateUnresolved,
    DeferredDestroyAlreadyPending,
    DeferredDestroyAlreadyQueued,
    DeathSoundMetadataMismatch,
    DeathSoundRuntimeMismatch,
    ConstructorSoundAttachmentNotExactNull,
    MissingInitializer,
    UnexpectedInitializerStateFlags {
        actual: u32,
    },
    DefaultStateFlagsMismatch {
        actual: RetailRuntimeValue<u32>,
    },
    UnexpectedBehaviorChoices,
    AlternateBehaviorClassMismatch {
        actual: u32,
    },
    UnexpectedCommonAxisDescriptor {
        actual: CommonAxisDescriptor,
    },
    ActorCommonAxisDescriptorMismatch {
        actual: RetailRuntimeValue<CommonAxisDescriptor>,
    },
    ComponentTopologyUnresolved,
    UnexpectedComponentTopology {
        actual: CommonMoverComponentTopology,
    },
    SubADescriptorUnresolved,
    SubADescriptorAbsent,
    UnexpectedSubADescriptor {
        actual: SubAPropulsionDescriptor,
    },
    SubCDescriptorUnresolved,
    SubCDescriptorAbsent,
    UnexpectedSubCDescriptor {
        actual: SubCLiftDescriptor,
    },
    SubHDescriptorUnresolved,
    SubHDescriptorAbsent,
    UnexpectedSubHDescriptorRecordCount {
        actual: usize,
    },
    UnexpectedSubHDescriptorCompletionSound {
        actual: Option<u16>,
    },
    UnexpectedSubHDescriptorRecords,
    SubARuntimeUnresolved,
    SubARuntimeAbsent,
    UnexpectedSubADriveScale {
        actual: i32,
    },
    SubHRuntimeUnresolved,
    SubHRuntimeAbsent,
    UnexpectedSubHRuntimeRecordCount {
        actual: usize,
    },
    CurrentBehaviorContextUnresolved,
    CurrentBehaviorContextAbsent,
    CurrentBehaviorChoiceSourceMismatch,
    CurrentBehaviorDescriptorStyleMismatch,
    CurrentBehaviorClassNotAuthoredForType47 {
        actual: u8,
    },
    CurrentBehaviorDeathCallbackPolicy(DeathCallbackPolicy),
    MissingTask {
        slot: ActorTaskSlot,
    },
    UnexpectedTask {
        slot: ActorTaskSlot,
    },
    TaskFamilyMismatch {
        slot: ActorTaskSlot,
        expected: ActorTaskRuntimeFamily,
        actual: ActorTaskRuntimeFamily,
    },
    TaskWrapperNotRunnable {
        slot: ActorTaskSlot,
        flags: ActorTaskWrapperFlags,
    },
    Constructor(CommonDyingConstructorError),
}

#[derive(Debug)]
struct Type47CommonDyingPreflight {
    authored_spawn_index: usize,
    selected_context: BehaviorContextRuntime,
    prepared_task: PreparedCommonDyingTask<ActorTaskRuntime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Type47DeathPrefixState {
    Fresh,
    AlreadyCommitted,
    /// Impact C690 / `FUN_00425660`: dying bit only. Not the death prefix.
    C690Alternate,
}

#[derive(Debug)]
struct PreparedType47StandardDeath {
    sound_position_raw: [i16; 3],
    suffix: Type47CommonDyingPreflight,
}

/// Enter the complete generic standard-death transition and synchronously
/// publish the exact type-47 class-12 task.
///
/// Every fallible cohort, metadata, component, behavior, and task check is
/// completed against the untouched entity before health, state, sound, RNG,
/// or task mutation. The remaining transaction is infallible in production.
pub fn publish_fresh_level_one_type47_standard_death(
    manager: &mut EntityManager,
    entity_id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
) -> Result<Type47StandardDeathOutcome, Type47CommonDyingPublicationError> {
    authenticate_present_native_allocation(manager, entity_id)?;
    publish_fresh_level_one_type47_standard_death_with_provenance(
        manager,
        entity_id,
        metadata,
        world_fx,
        manager.is_fresh_new_game_first_world()
            || manager
                .iter_all()
                .find(|entity| entity.id == entity_id)
                .is_some_and(crate::intro2_type47_live::intro2_type47_cohort_runtime_authenticates),
    )
}

fn publish_fresh_level_one_type47_standard_death_with_provenance(
    manager: &mut EntityManager,
    entity_id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
    fresh_new_game_first_world: bool,
) -> Result<Type47StandardDeathOutcome, Type47CommonDyingPublicationError> {
    if !fresh_new_game_first_world {
        return Err(Type47CommonDyingPublicationError::NotFreshNewGameFirstWorld);
    }
    let deferred_destroy_already_queued = manager
        .pending_actor_deferred_destroy_ids()
        .contains(&entity_id);
    let entity = manager
        .common_actor_dying_entity_mut(entity_id)
        .ok_or(Type47CommonDyingPublicationError::EntityUnavailable { entity_id })?;
    let authored_spawn_index = authenticate_fresh_level_one_type47_owner(entity, metadata)?;

    match entity
        .collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT)
    {
        RetailRuntimeValue::Known(REMOTE_OWNED_STATE_BIT) => {
            return Ok(Type47StandardDeathOutcome::RemoteOwnedNoOp)
        }
        RetailRuntimeValue::Known(0) => {}
        RetailRuntimeValue::Known(_) => unreachable!("a one-bit mask has only two values"),
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CommonDyingPublicationError::RemoteOwnerStateUnresolved)
        }
    }
    match entity
        .collision
        .state_flags_at_0x08
        .masked(ACTIVE_MODEL_SLOT_LOW_STATE_BIT)
    {
        RetailRuntimeValue::Known(ACTIVE_MODEL_SLOT_LOW_STATE_BIT) => {
            return Ok(Type47StandardDeathOutcome::AlreadyDyingNoOp)
        }
        RetailRuntimeValue::Known(0) => {}
        RetailRuntimeValue::Known(_) => unreachable!("a one-bit mask has only two values"),
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CommonDyingPublicationError::DyingStateUnresolved)
        }
    }

    if deferred_destroy_already_queued {
        return Err(Type47CommonDyingPublicationError::DeferredDestroyAlreadyQueued);
    }
    let suffix = preflight_type47_common_dying_publication_after_owner_auth(
        entity,
        metadata,
        authored_spawn_index,
        Type47DeathPrefixState::Fresh,
    )?;
    let prepared = PreparedType47StandardDeath {
        sound_position_raw: entity.position_raw(),
        suffix,
    };
    Ok(Type47StandardDeathOutcome::Published(
        apply_prepared_type47_standard_death(entity, prepared, world_fx),
    ))
}

/// Install Section-12 `+0x124` class 12 after impact C690 saw bit `0x4000`.
///
/// Retail `FUN_0040AC60` calls `FUN_00425660(entity, type+0x11C)`, which is
/// `FUN_00438340(entity, 0, *+0x124)`. An existing context uses
/// `FUN_0040ABB0` (preserve `+0x08/+0x0C`) then `FUN_0040C6B0` /
/// `FUN_0040C620`. This is not the generic death prefix: health, the dying
/// model write, and sound 75 stay with
/// [`publish_fresh_level_one_type47_standard_death`].
pub fn publish_type47_c690_alternate_class12(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
) -> Result<Type47CommonDyingPublication, Type47CommonDyingPublicationError> {
    let authored_spawn_index = authenticate_fresh_level_one_type47_owner(entity, metadata)?;
    let preflight = preflight_type47_common_dying_publication_after_owner_auth(
        entity,
        metadata,
        authored_spawn_index,
        Type47DeathPrefixState::C690Alternate,
    )?;
    Ok(apply_type47_common_dying_publication(
        entity,
        preflight,
        world_fx,
        &mut |task| Ok(task),
        &mut |_| {},
    )
    .expect("the prepared production transaction has no fallible dispatch step"))
}

/// Publish the exact type-47 alternate class after the generic death prefix.
pub fn publish_fresh_level_one_type47_common_dying(
    manager: &mut EntityManager,
    entity_id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
) -> Result<Type47CommonDyingPublication, Type47CommonDyingPublicationError> {
    authenticate_present_native_allocation(manager, entity_id)?;
    publish_fresh_level_one_type47_common_dying_with_prepare(
        manager,
        entity_id,
        metadata,
        world_fx,
        manager.is_fresh_new_game_first_world()
            || manager
                .iter_all()
                .find(|entity| entity.id == entity_id)
                .is_some_and(crate::intro2_type47_live::intro2_type47_cohort_runtime_authenticates),
        Ok,
        |_| {},
    )
    .map(|result| {
        result.expect("an already-prepared production task cannot fail final preparation")
    })
}

fn authenticate_present_native_allocation(
    manager: &EntityManager,
    id: u32,
) -> Result<(), Type47CommonDyingPublicationError> {
    if let Some(entity) = manager.iter_all().find(|entity| entity.id == id) {
        if entity.native_type47_construction.is_some()
            && !crate::shared_type47::type47_manager_allocation_authenticates(manager, id)
        {
            return Err(Type47CommonDyingPublicationError::UnauthenticatedSpawn {
                actual: entity.authored_spawn_index,
            });
        }
    }
    Ok(())
}

fn publish_fresh_level_one_type47_common_dying_with_prepare(
    manager: &mut EntityManager,
    entity_id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
    fresh_new_game_first_world: bool,
    mut prepare_primary: impl FnMut(
        PreparedCommonDyingTask<ActorTaskRuntime>,
    ) -> Result<PreparedCommonDyingTask<ActorTaskRuntime>, ()>,
    mut observe_effect: impl FnMut(CommonDyingConstructorEffect),
) -> Result<
    Result<Type47CommonDyingPublication, ActorTaskPrepareError<()>>,
    Type47CommonDyingPublicationError,
> {
    if !fresh_new_game_first_world {
        return Err(Type47CommonDyingPublicationError::NotFreshNewGameFirstWorld);
    }
    if manager
        .pending_actor_deferred_destroy_ids()
        .contains(&entity_id)
    {
        return Err(Type47CommonDyingPublicationError::DeferredDestroyAlreadyQueued);
    }
    let entity = manager
        .common_actor_dying_entity_mut(entity_id)
        .ok_or(Type47CommonDyingPublicationError::EntityUnavailable { entity_id })?;
    let preflight = preflight_type47_common_dying_publication(entity, metadata)?;
    Ok(apply_type47_common_dying_publication(
        entity,
        preflight,
        world_fx,
        &mut prepare_primary,
        &mut observe_effect,
    ))
}

fn preflight_type47_common_dying_publication(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<Type47CommonDyingPreflight, Type47CommonDyingPublicationError> {
    let authored_spawn_index = authenticate_fresh_level_one_type47_owner(entity, metadata)?;
    preflight_type47_common_dying_publication_after_owner_auth(
        entity,
        metadata,
        authored_spawn_index,
        Type47DeathPrefixState::AlreadyCommitted,
    )
}

fn authenticate_fresh_level_one_type47_owner(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<usize, Type47CommonDyingPublicationError> {
    if !entity.active {
        return Err(Type47CommonDyingPublicationError::EntityInactive);
    }
    let spawn = entity.authored_spawn_index;
    // Preserve the legacy spawn -> type -> model validation order. The
    // reselection cohort helper also checks the type, which would hide a
    // wrong-type error at an otherwise authenticated Level-1 spawn.
    let Some(authored_spawn_index) = spawn.filter(|spawn| {
        FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES.contains(spawn)
            || crate::intro2_type47_live::intro2_type47_cohort_runtime_authenticates(entity)
    }) else {
        return Err(Type47CommonDyingPublicationError::UnauthenticatedSpawn { actual: spawn });
    };
    if entity.entity_type != TYPE47_COMMON_DYING_ENTITY_TYPE {
        return Err(Type47CommonDyingPublicationError::UnexpectedEntityType {
            actual: entity.entity_type,
        });
    }
    let expected_model = Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID);
    if entity.model_slots != [expected_model; 4] {
        return Err(
            Type47CommonDyingPublicationError::UnexpectedEntityModelSlots {
                actual: entity.model_slots,
            },
        );
    }
    if entity.model_index != expected_model {
        return Err(Type47CommonDyingPublicationError::UnexpectedEntityModel {
            actual: entity.model_index,
        });
    }
    let expected_metadata_models = [FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID as u16; 4];
    if metadata.model_slots != expected_metadata_models {
        return Err(
            Type47CommonDyingPublicationError::UnexpectedMetadataModelSlots {
                actual: metadata.model_slots,
            },
        );
    }
    if metadata.capability_flags != TYPE47_COMMON_DYING_CAPABILITY_FLAGS {
        return Err(
            Type47CommonDyingPublicationError::UnexpectedMetadataCapabilityFlags {
                actual: metadata.capability_flags,
            },
        );
    }
    if entity.capability_flags != TYPE47_COMMON_DYING_CAPABILITY_FLAGS {
        return Err(
            Type47CommonDyingPublicationError::UnexpectedEntityCapabilityFlags {
                actual: entity.capability_flags,
            },
        );
    }
    if metadata.mass_raw != TYPE47_COMMON_DYING_MASS_RAW {
        return Err(Type47CommonDyingPublicationError::UnexpectedMetadataMass {
            actual: metadata.mass_raw,
        });
    }
    if entity.mass_raw != TYPE47_COMMON_DYING_MASS_RAW {
        return Err(Type47CommonDyingPublicationError::UnexpectedEntityMass {
            actual: entity.mass_raw,
        });
    }
    if metadata.initial_health_raw != Some(TYPE47_COMMON_DYING_INITIAL_HEALTH_RAW) {
        return Err(Type47CommonDyingPublicationError::UnexpectedInitialHealth {
            actual: metadata.initial_health_raw,
        });
    }
    if metadata.death_sound_id
        != RetailRuntimeValue::Known(Some(TYPE47_COMMON_DYING_DEATH_SOUND_ID))
    {
        return Err(Type47CommonDyingPublicationError::DeathSoundMetadataMismatch);
    }
    if metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None) {
        return Err(Type47CommonDyingPublicationError::ConstructorSoundAttachmentNotExactNull);
    }
    Ok(authored_spawn_index)
}

fn preflight_type47_common_dying_publication_after_owner_auth(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    authored_spawn_index: usize,
    prefix_state: Type47DeathPrefixState,
) -> Result<Type47CommonDyingPreflight, Type47CommonDyingPublicationError> {
    match prefix_state {
        Type47DeathPrefixState::Fresh => {}
        Type47DeathPrefixState::AlreadyCommitted => {
            if entity.collision.health_raw != RetailRuntimeValue::Known(0) {
                return Err(Type47CommonDyingPublicationError::HealthNotZero);
            }
            match entity
                .collision
                .state_flags_at_0x08
                .masked(ACTIVE_MODEL_SLOT_LOW_STATE_BIT)
            {
                RetailRuntimeValue::Known(ACTIVE_MODEL_SLOT_LOW_STATE_BIT) => {}
                RetailRuntimeValue::Known(_) => {
                    return Err(Type47CommonDyingPublicationError::DyingStateNotSet)
                }
                RetailRuntimeValue::Unresolved => {
                    return Err(Type47CommonDyingPublicationError::DyingStateUnresolved)
                }
            }
        }
        Type47DeathPrefixState::C690Alternate => {
            match entity
                .collision
                .state_flags_at_0x08
                .masked(ACTIVE_MODEL_SLOT_LOW_STATE_BIT)
            {
                RetailRuntimeValue::Known(ACTIVE_MODEL_SLOT_LOW_STATE_BIT) => {}
                RetailRuntimeValue::Known(_) => {
                    return Err(Type47CommonDyingPublicationError::DyingStateNotSet)
                }
                RetailRuntimeValue::Unresolved => {
                    return Err(Type47CommonDyingPublicationError::DyingStateUnresolved)
                }
            }
        }
    }
    match entity
        .collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT)
    {
        RetailRuntimeValue::Known(0) => {}
        RetailRuntimeValue::Known(_) => return Err(Type47CommonDyingPublicationError::RemoteOwned),
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CommonDyingPublicationError::RemoteOwnerStateUnresolved)
        }
    }
    match entity
        .collision
        .state_flags_at_0x08
        .masked(DEFERRED_DESTROY_PENDING_STATE_BIT)
    {
        RetailRuntimeValue::Known(0) => {}
        RetailRuntimeValue::Known(_) => {
            return Err(Type47CommonDyingPublicationError::DeferredDestroyAlreadyPending)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CommonDyingPublicationError::DeferredDestroyStateUnresolved)
        }
    }
    if prefix_state != Type47DeathPrefixState::C690Alternate
        && entity.collision.death_sound_id
            != RetailRuntimeValue::Known(Some(TYPE47_COMMON_DYING_DEATH_SOUND_ID))
    {
        return Err(Type47CommonDyingPublicationError::DeathSoundRuntimeMismatch);
    }

    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(Type47CommonDyingPublicationError::MissingInitializer)?;
    if initializer.initializer_state_flags_raw != TYPE47_COMMON_DYING_INITIALIZER_STATE_FLAGS_RAW {
        return Err(
            Type47CommonDyingPublicationError::UnexpectedInitializerStateFlags {
                actual: initializer.initializer_state_flags_raw,
            },
        );
    }
    if entity.collision.default_state_flags_at_0xc8
        != RetailRuntimeValue::Known(TYPE47_COMMON_DYING_INITIALIZER_STATE_FLAGS_RAW)
    {
        return Err(
            Type47CommonDyingPublicationError::DefaultStateFlagsMismatch {
                actual: entity.collision.default_state_flags_at_0xc8,
            },
        );
    }
    if initializer.behavior_choices.as_ref() != TYPE47_COMMON_DYING_BEHAVIOR_CHOICES {
        return Err(Type47CommonDyingPublicationError::UnexpectedBehaviorChoices);
    }
    if initializer.alternate_behavior_class_ref != TYPE47_COMMON_DYING_BEHAVIOR_CLASS_ID {
        return Err(
            Type47CommonDyingPublicationError::AlternateBehaviorClassMismatch {
                actual: initializer.alternate_behavior_class_ref,
            },
        );
    }
    if initializer.common_axis_descriptor != TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR {
        return Err(
            Type47CommonDyingPublicationError::UnexpectedCommonAxisDescriptor {
                actual: initializer.common_axis_descriptor,
            },
        );
    }
    if entity.actor_common_axis_descriptor
        != RetailRuntimeValue::Known(TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR)
    {
        return Err(
            Type47CommonDyingPublicationError::ActorCommonAxisDescriptorMismatch {
                actual: entity.actor_common_axis_descriptor,
            },
        );
    }
    let topology = match metadata.common_mover_topology {
        RetailRuntimeValue::Known(topology) => topology,
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CommonDyingPublicationError::ComponentTopologyUnresolved)
        }
    };
    if topology != FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY {
        return Err(
            Type47CommonDyingPublicationError::UnexpectedComponentTopology { actual: topology },
        );
    }
    let sub_a_descriptor = match metadata.sub_a_propulsion_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
        RetailRuntimeValue::Known(None) => {
            return Err(Type47CommonDyingPublicationError::SubADescriptorAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CommonDyingPublicationError::SubADescriptorUnresolved)
        }
    };
    if sub_a_descriptor != TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR {
        return Err(
            Type47CommonDyingPublicationError::UnexpectedSubADescriptor {
                actual: sub_a_descriptor,
            },
        );
    }
    let sub_c_descriptor = match metadata.sub_c_lift_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
        RetailRuntimeValue::Known(None) => {
            return Err(Type47CommonDyingPublicationError::SubCDescriptorAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CommonDyingPublicationError::SubCDescriptorUnresolved)
        }
    };
    if sub_c_descriptor != TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR {
        return Err(
            Type47CommonDyingPublicationError::UnexpectedSubCDescriptor {
                actual: sub_c_descriptor,
            },
        );
    }
    let sub_h_descriptor = match &metadata.sub_h_external_frame_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
        RetailRuntimeValue::Known(None) => {
            return Err(Type47CommonDyingPublicationError::SubHDescriptorAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CommonDyingPublicationError::SubHDescriptorUnresolved)
        }
    };
    if sub_h_descriptor.records.len() != FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT {
        return Err(
            Type47CommonDyingPublicationError::UnexpectedSubHDescriptorRecordCount {
                actual: sub_h_descriptor.records.len(),
            },
        );
    }
    if sub_h_descriptor.completion_sound_id.is_some() {
        return Err(
            Type47CommonDyingPublicationError::UnexpectedSubHDescriptorCompletionSound {
                actual: sub_h_descriptor.completion_sound_id,
            },
        );
    }
    if sub_h_descriptor.records.as_slice() != TYPE47_COMMON_DYING_SUB_H_RECORDS {
        return Err(Type47CommonDyingPublicationError::UnexpectedSubHDescriptorRecords);
    }
    let sub_a_runtime = match entity.sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(runtime)) => runtime,
        RetailRuntimeValue::Known(None) => {
            return Err(Type47CommonDyingPublicationError::SubARuntimeAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CommonDyingPublicationError::SubARuntimeUnresolved)
        }
    };
    if sub_a_runtime.drive_scale_percent() != TYPE47_COMMON_DYING_DRIVE_SCALE_PERCENT {
        return Err(
            Type47CommonDyingPublicationError::UnexpectedSubADriveScale {
                actual: sub_a_runtime.drive_scale_percent(),
            },
        );
    }
    let sub_h_runtime = match &entity.sub_h_external_frame_runtime {
        RetailRuntimeValue::Known(Some(runtime)) => runtime,
        RetailRuntimeValue::Known(None) => {
            return Err(Type47CommonDyingPublicationError::SubHRuntimeAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CommonDyingPublicationError::SubHRuntimeUnresolved)
        }
    };
    if sub_h_runtime.records().len() != FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT {
        return Err(
            Type47CommonDyingPublicationError::UnexpectedSubHRuntimeRecordCount {
                actual: sub_h_runtime.records().len(),
            },
        );
    }

    let current_context = match entity.current_behavior_context {
        RetailRuntimeValue::Known(Some(context)) => context,
        RetailRuntimeValue::Known(None) => {
            return Err(Type47CommonDyingPublicationError::CurrentBehaviorContextAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CommonDyingPublicationError::CurrentBehaviorContextUnresolved)
        }
    };
    if current_context.choice_list_source()
        != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
    {
        return Err(Type47CommonDyingPublicationError::CurrentBehaviorChoiceSourceMismatch);
    }
    let current_class_id = match (current_context.descriptor(), current_context.active_style()) {
        (BehaviorDescriptorIdentity::Named(program), ActiveBehaviorStyle::Audited(style))
            if style.class_id == program.class_id
                && (prefix_state == Type47DeathPrefixState::C690Alternate
                    || style.variant == 0) =>
        {
            program.class_id
        }
        _ => return Err(Type47CommonDyingPublicationError::CurrentBehaviorDescriptorStyleMismatch),
    };
    match prefix_state {
        Type47DeathPrefixState::C690Alternate => {
            if current_class_id != TYPE47_GUARD_BEHAVIOR_CLASS_ID {
                return Err(
                    Type47CommonDyingPublicationError::CurrentBehaviorClassNotAuthoredForType47 {
                        actual: current_class_id,
                    },
                );
            }
        }
        Type47DeathPrefixState::Fresh | Type47DeathPrefixState::AlreadyCommitted => {
            if !matches!(
                current_class_id,
                TYPE47_GUARD_BEHAVIOR_CLASS_ID | TYPE47_WANDER_BEHAVIOR_CLASS_ID
            ) {
                return Err(
                    Type47CommonDyingPublicationError::CurrentBehaviorClassNotAuthoredForType47 {
                        actual: current_class_id,
                    },
                );
            }
        }
    }
    let death_policy = current_context.active_style().death_callback_policy();
    if death_policy != DeathCallbackPolicy::None {
        return Err(
            Type47CommonDyingPublicationError::CurrentBehaviorDeathCallbackPolicy(death_policy),
        );
    }
    if prefix_state != Type47DeathPrefixState::C690Alternate {
        authenticate_type47_task_graph(entity, current_class_id)?;
    }

    let common_dying_program = audited_behavior_program(TYPE47_COMMON_DYING_BEHAVIOR_CLASS_ID)
        .expect("class-12 alternate behavior program is statically audited");
    let selected_context = current_context
        .reselect_audited_type_default(common_dying_program, 0, COMMON_ACTOR_DYING_ACTIVE_STYLE)
        .expect("class-12 variant zero is an exact audited style");
    let prepared_task = CommonDyingTaskState::prepare_after_allocation(
        entity.id,
        metadata,
        CommonDyingComponentDescriptors::default(),
    )
    .map_err(Type47CommonDyingPublicationError::Constructor)?
    .map_task(ActorTaskRuntime::CommonDying);

    Ok(Type47CommonDyingPreflight {
        authored_spawn_index,
        selected_context,
        prepared_task,
    })
}

fn authenticate_type47_task_graph(
    entity: &Entity,
    current_class_id: u8,
) -> Result<(), Type47CommonDyingPublicationError> {
    authenticate_task_family(
        entity,
        ActorTaskSlot::Primary,
        ActorTaskRuntimeFamily::OrdinaryType9Wander,
    )?;
    match current_class_id {
        TYPE47_GUARD_BEHAVIOR_CLASS_ID => authenticate_task_family(
            entity,
            ActorTaskSlot::Secondary,
            ActorTaskRuntimeFamily::GuardLocationAcquisition,
        )?,
        TYPE47_WANDER_BEHAVIOR_CLASS_ID => {
            authenticate_empty_slot(entity, ActorTaskSlot::Secondary)?
        }
        _ => unreachable!("the current class was admitted immediately above"),
    }
    authenticate_empty_slot(entity, ActorTaskSlot::Tertiary)
}

fn authenticate_task_family(
    entity: &Entity,
    slot: ActorTaskSlot,
    expected: ActorTaskRuntimeFamily,
) -> Result<(), Type47CommonDyingPublicationError> {
    let task_id = entity
        .actor_tasks
        .task_in_slot(slot)
        .ok_or(Type47CommonDyingPublicationError::MissingTask { slot })?;
    let task = entity
        .actor_tasks
        .task_state(task_id)
        .ok_or(Type47CommonDyingPublicationError::MissingTask { slot })?;
    let actual = task.family();
    if actual != expected {
        return Err(Type47CommonDyingPublicationError::TaskFamilyMismatch {
            slot,
            expected,
            actual,
        });
    }
    let flags = entity
        .actor_tasks
        .wrapper_flags(task_id)
        .ok_or(Type47CommonDyingPublicationError::MissingTask { slot })?;
    if flags
        != (ActorTaskWrapperFlags {
            alive: true,
            in_callback: false,
        })
    {
        return Err(Type47CommonDyingPublicationError::TaskWrapperNotRunnable { slot, flags });
    }
    Ok(())
}

fn authenticate_empty_slot(
    entity: &Entity,
    slot: ActorTaskSlot,
) -> Result<(), Type47CommonDyingPublicationError> {
    if entity.actor_tasks.task_in_slot(slot).is_some() {
        return Err(Type47CommonDyingPublicationError::UnexpectedTask { slot });
    }
    Ok(())
}

fn apply_prepared_type47_standard_death(
    entity: &mut Entity,
    prepared: PreparedType47StandardDeath,
    world_fx: &mut WorldFx,
) -> Type47CommonDyingPublication {
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity.collision.state_flags_at_0x08.overwrite(
        ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
        ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
    );
    world_fx.queue_fixed_positional_sound_raw(
        TYPE47_COMMON_DYING_DEATH_SOUND_ID,
        prepared.sound_position_raw,
    );
    // Section-12 +0xB4 was authenticated as an exact null before the prefix,
    // so retail's intervening release call is an exact no-op.
    apply_type47_common_dying_publication(
        entity,
        prepared.suffix,
        world_fx,
        &mut |task| Ok(task),
        &mut |_| {},
    )
    .expect("the prepared production transaction has no fallible dispatch step")
}

fn apply_type47_common_dying_publication(
    entity: &mut Entity,
    preflight: Type47CommonDyingPreflight,
    world_fx: &mut WorldFx,
    prepare_primary: &mut impl FnMut(
        PreparedCommonDyingTask<ActorTaskRuntime>,
    ) -> Result<PreparedCommonDyingTask<ActorTaskRuntime>, ()>,
    observe_effect: &mut impl FnMut(CommonDyingConstructorEffect),
) -> Result<Type47CommonDyingPublication, ActorTaskPrepareError<()>> {
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(preflight.selected_context));
    let program = audited_behavior_program(TYPE47_COMMON_DYING_BEHAVIOR_CLASS_ID)
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
                observe_effect(effect);
                apply_type47_common_dying_constructor_effect(
                    effect,
                    sub_h,
                    sub_a,
                    velocity,
                    &mut velocity_raw,
                    &mut constructor,
                );
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
    setup?;

    // Every accepted in-memory Type-47 death sample retains the normal
    // scheduler's animation offset at exact zero. The generic constructor
    // cannot prove that allocator-owned word, so custody is promoted only
    // after the authenticated class-12 primary handoff succeeds; the coarse
    // owner later computes callback mass from this retained value.
    // Native Intro2 already owns its constructor residue policy and later
    // contributions. C620 has no B2 write, so that live word survives here.
    if !crate::intro2_type47_live::intro2_type47_cohort_runtime_authenticates(entity) {
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    }

    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .expect("successful class-12 setup publishes primary last");
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    Ok(Type47CommonDyingPublication {
        owner: FreshLevelOneType47CommonDyingOwner::from_authenticated_publication(
            entity.id,
            preflight.authored_spawn_index,
            visit,
        ),
        constructor: constructor
            .expect("exact type-47 Sub-A topology consumes one constructor draw"),
    })
}

fn apply_type47_common_dying_constructor_effect(
    effect: CommonDyingConstructorEffect,
    sub_h: &mut SubHRuntimeState,
    sub_a: &mut SubAPropulsionRuntime,
    velocity: &mut [f32; 3],
    velocity_raw: &mut [i16; 3],
    constructor: &mut Option<Type47CommonDyingConstructorEvidence>,
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
            *constructor = Some(Type47CommonDyingConstructorEvidence {
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
            unreachable!("exact type-47 A/B/C/D/E/H/J topology has no Sub-F/Sub-G effects")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use v2k_formats::collision::SubHExternalFrameDescriptor;

    use super::*;
    use crate::{
        actor_task_owner::PreparedActorTask,
        entity::{EntityKind, EntityManager},
        entity_behavior::{audited_behavior_style, behavior_program},
        entity_collision_state::{
            EntityInitializerSpec, RetailStateWord, ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
        },
        guard_location_owner::acquisition::GuardLocationAcquisitionTaskState,
        ordinary_type9_wander_owner::plan_ordinary_type9_wander_setup,
        session::GameSession,
    };

    const ENTITY_ID: u32 = 100;
    const INITIAL_VELOCITY_RAW: [i16; 3] = [11, -22, 33];
    const BASE_LIVE_STATE: u32 = 0x0142_8805 | ACTIVE_MODEL_SLOT_LOW_STATE_BIT;

    fn captured_death_position_raw(spawn: usize) -> [i16; 3] {
        match spawn {
            11 => [-18_688, -462, 32_512],
            12 => [-16_896, -398, -31_488],
            13 => [-16_896, -430, 32_000],
            _ => panic!("no captured type-47 death position for spawn {spawn}"),
        }
    }

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID as u16; 4],
            mass_raw: TYPE47_COMMON_DYING_MASS_RAW,
            capability_flags: TYPE47_COMMON_DYING_CAPABILITY_FLAGS,
            initial_health_raw: Some(TYPE47_COMMON_DYING_INITIAL_HEALTH_RAW),
            accepted_hit_presentation_sound_id: RetailRuntimeValue::Known(Some(92)),
            death_sound_id: RetailRuntimeValue::Known(Some(TYPE47_COMMON_DYING_DEATH_SOUND_ID)),
            constructor_sound_attachment_id: RetailRuntimeValue::Known(None),
            generic_hit_sound_id: RetailRuntimeValue::Known(None),
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
            common_mover_topology: RetailRuntimeValue::Known(
                FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY,
            ),
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

    fn exact_context(class_id: u8) -> BehaviorContextRuntime {
        let program = behavior_program(u32::from(class_id)).expect("weighted type-47 behavior");
        let style = *audited_behavior_style(u32::from(class_id), 0)
            .expect("type-47 initial behavior style");
        BehaviorContextRuntime::named_audited(
            program,
            0,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(0),
            style,
        )
        .unwrap()
    }

    fn install_wander_primary(entity: &mut Entity) {
        let position_raw = entity.position_raw();
        let Entity {
            actor_tasks,
            sub_a_propulsion_runtime,
            ..
        } = entity;
        let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
            panic!("fixture must retain Sub-A")
        };
        plan_ordinary_type9_wander_setup(
            position_raw,
            TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR.target_speed_base_raw,
        )
        .apply(actor_tasks, sub_a, |specification| {
            Ok::<_, Infallible>(
                specification
                    .prepare_after_allocation(|| 0x1234)
                    .map_task(ActorTaskRuntime::OrdinaryType9Wander),
            )
        })
        .unwrap();
    }

    fn exact_entity(spawn: usize, class_id: u8, high_selector: bool) -> Entity {
        let mut entity = Entity::unresolved_port_entity(
            ENTITY_ID,
            EntityKind::Enemy,
            TYPE47_COMMON_DYING_ENTITY_TYPE,
        );
        entity.authored_spawn_index = Some(spawn);
        entity.model_slots = [Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID); 4];
        entity.model_index = Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID);
        entity.mass_raw = TYPE47_COMMON_DYING_MASS_RAW;
        entity.capability_flags = TYPE47_COMMON_DYING_CAPABILITY_FLAGS;
        entity.collision.health_raw = RetailRuntimeValue::Known(0);
        entity.collision.death_sound_id =
            RetailRuntimeValue::Known(Some(TYPE47_COMMON_DYING_DEATH_SOUND_ID));
        entity.collision.default_state_flags_at_0xc8 =
            RetailRuntimeValue::Known(TYPE47_COMMON_DYING_INITIALIZER_STATE_FLAGS_RAW);
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(
            BASE_LIVE_STATE
                | if high_selector {
                    ACTIVE_MODEL_SLOT_HIGH_STATE_BIT
                } else {
                    0
                },
        );
        entity.actor_common_axis_descriptor =
            RetailRuntimeValue::Known(TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR);
        entity.sub_a_propulsion_runtime =
            RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_retail_words(
                RetailRuntimeValue::Known(777),
                -1,
                TYPE47_COMMON_DYING_DRIVE_SCALE_PERCENT,
            )));
        entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(
            SubHRuntimeState::new(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT).unwrap(),
        ));
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(exact_context(class_id)));
        entity.set_velocity_raw(INITIAL_VELOCITY_RAW);
        install_wander_primary(&mut entity);
        if class_id == TYPE47_GUARD_BEHAVIOR_CLASS_ID {
            entity.actor_tasks.replace_prepared(
                ActorTaskSlot::Secondary,
                PreparedActorTask::new(ActorTaskRuntime::GuardLocationAcquisition(
                    GuardLocationAcquisitionTaskState::new(0),
                )),
            );
        }
        entity
    }

    fn exact_manager(spawn: usize, class_id: u8, high_selector: bool) -> EntityManager {
        EntityManager::from_entities_for_test(vec![exact_entity(spawn, class_id, high_selector)])
    }

    fn exact_fresh_manager(
        spawn: usize,
        class_id: u8,
        high_selector: bool,
        health: RetailRuntimeValue<i32>,
    ) -> EntityManager {
        let mut manager = exact_manager(spawn, class_id, high_selector);
        let entity = manager.entity_mut_for_test(ENTITY_ID).unwrap();
        entity.collision.health_raw = health;
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(ACTIVE_MODEL_SLOT_LOW_STATE_BIT, 0);
        entity.set_position_raw(captured_death_position_raw(spawn));
        manager
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct LiveSnapshot {
        context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
        state: RetailStateWord,
        health: RetailRuntimeValue<i32>,
        velocity_raw: [i16; 3],
        sub_a: RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
        sub_h: RetailRuntimeValue<Option<SubHRuntimeState>>,
        task_ids: [Option<crate::actor_task_owner::ActorTaskId>; 3],
    }

    fn snapshot(manager: &EntityManager) -> LiveSnapshot {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        LiveSnapshot {
            context: entity.current_behavior_context,
            state: entity.collision.state_flags_at_0x08,
            health: entity.collision.health_raw,
            velocity_raw: entity.velocity_raw(),
            sub_a: entity.sub_a_propulsion_runtime,
            sub_h: entity.sub_h_external_frame_runtime.clone(),
            task_ids: ActorTaskSlot::IN_RETAIL_TICK_ORDER
                .map(|slot| entity.actor_tasks.task_in_slot(slot)),
        }
    }

    fn publish_for_test(
        manager: &mut EntityManager,
        metadata: &EntityTypeRuntimeMetadata,
        world_fx: &mut WorldFx,
        observe_effect: impl FnMut(CommonDyingConstructorEffect),
    ) -> Result<
        Result<Type47CommonDyingPublication, ActorTaskPrepareError<()>>,
        Type47CommonDyingPublicationError,
    > {
        publish_fresh_level_one_type47_common_dying_with_prepare(
            manager,
            ENTITY_ID,
            metadata,
            world_fx,
            true,
            Ok,
            observe_effect,
        )
    }

    fn publish_standard_death_for_test(
        manager: &mut EntityManager,
        metadata: &EntityTypeRuntimeMetadata,
        world_fx: &mut WorldFx,
    ) -> Result<Type47StandardDeathOutcome, Type47CommonDyingPublicationError> {
        publish_fresh_level_one_type47_standard_death_with_provenance(
            manager, ENTITY_ID, metadata, world_fx, true,
        )
    }

    #[v2k_test_support::retail_test]
    fn normal_tier_retail_record_matches_the_transition_profile() {
        let data_dir = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&data_dir).expect("init retail session");
        session
            .load_auxiliary_ovl(3, 1)
            .expect("load normal-tier first-world resources");
        session
            .load_level_by_id(13, 1)
            .expect("load normal-tier Level 1");
        let metadata = EntityTypeRuntimeMetadata::from_section12(
            session
                .cache
                .global_entity_type(TYPE47_COMMON_DYING_ENTITY_TYPE as usize)
                .expect("cumulative type-47 record"),
        );
        let initializer = metadata.initializer.as_ref().unwrap();
        assert_eq!(metadata.model_slots, [302; 4]);
        assert_eq!(metadata.mass_raw, TYPE47_COMMON_DYING_MASS_RAW);
        assert_eq!(
            metadata.capability_flags,
            TYPE47_COMMON_DYING_CAPABILITY_FLAGS
        );
        assert_eq!(
            metadata.initial_health_raw,
            Some(TYPE47_COMMON_DYING_INITIAL_HEALTH_RAW)
        );
        assert_eq!(
            metadata.death_sound_id,
            RetailRuntimeValue::Known(Some(TYPE47_COMMON_DYING_DEATH_SOUND_ID))
        );
        assert_eq!(
            metadata.constructor_sound_attachment_id,
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            initializer.initializer_state_flags_raw,
            TYPE47_COMMON_DYING_INITIALIZER_STATE_FLAGS_RAW
        );
        assert_eq!(
            initializer.behavior_choices.as_ref(),
            TYPE47_COMMON_DYING_BEHAVIOR_CHOICES
        );
        assert_eq!(
            initializer.alternate_behavior_class_ref,
            TYPE47_COMMON_DYING_BEHAVIOR_CLASS_ID
        );
        assert_eq!(
            initializer.common_axis_descriptor,
            TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR
        );
        assert_eq!(
            metadata.sub_a_propulsion_descriptor,
            RetailRuntimeValue::Known(Some(TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR))
        );
        assert_eq!(
            metadata.sub_c_lift_descriptor,
            RetailRuntimeValue::Known(Some(TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR))
        );
        let RetailRuntimeValue::Known(Some(sub_h)) = metadata.sub_h_external_frame_descriptor
        else {
            panic!("type 47 must author Sub-H")
        };
        assert_eq!(sub_h.completion_sound_id, None);
        assert_eq!(sub_h.records, TYPE47_COMMON_DYING_SUB_H_RECORDS);
    }

    #[test]
    fn standard_death_publishes_spawn13_guard_once_with_exact_prefix_and_suffix() {
        let metadata = exact_metadata();
        let mut manager = exact_fresh_manager(
            13,
            TYPE47_GUARD_BEHAVIOR_CLASS_ID,
            true,
            RetailRuntimeValue::Known(1_377),
        );
        let mut world_fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
        let expected_sample = oracle.next_shared_retail_random_u16();
        let expected_next = oracle.next_shared_retail_random_u16();
        let expected_target = 300 + i32::from(expected_sample >> 8) * 300 / 0x0A00;

        let Type47StandardDeathOutcome::Published(publication) =
            publish_standard_death_for_test(&mut manager, &metadata, &mut world_fx).unwrap()
        else {
            panic!("fresh local spawn 13 must publish")
        };

        assert_eq!(publication.owner.authored_spawn_index(), 13);
        assert_eq!(
            publication.constructor,
            Type47CommonDyingConstructorEvidence {
                random_sample_low16: expected_sample,
                sub_a_target_speed_raw: expected_target,
            }
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0),
            "the authenticated capture-backed handoff owns callback-mass provenance"
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08,
            RetailStateWord::exact(0x0143_E805)
        );
        assert_eq!(entity.velocity_raw(), [11, 500, 33]);
        assert_eq!(world_fx.pending_event_count(), 1);
        let after_first = snapshot(&manager);

        assert_eq!(
            publish_standard_death_for_test(&mut manager, &metadata, &mut world_fx).unwrap(),
            Type47StandardDeathOutcome::AlreadyDyingNoOp
        );
        assert_eq!(snapshot(&manager), after_first);
        assert_eq!(world_fx.pending_event_count(), 1);
        assert_eq!(world_fx.next_shared_retail_random_u16(), expected_next);
        world_fx.process_pending();
        let expected_position_raw = captured_death_position_raw(13);
        assert_eq!(
            world_fx.take_positional_sounds(),
            vec![crate::world_fx::PositionalSoundEvent::fixed(
                usize::from(TYPE47_COMMON_DYING_DEATH_SOUND_ID),
                [
                    f32::from(expected_position_raw[0] as u16) / 256.0,
                    f32::from(expected_position_raw[1]) / 256.0,
                    f32::from(expected_position_raw[2] as u16) / 256.0,
                ],
            )]
        );
    }

    #[test]
    fn standard_death_accepts_wander_and_overwrites_unresolved_health() {
        let metadata = exact_metadata();
        let mut manager = exact_fresh_manager(
            12,
            TYPE47_WANDER_BEHAVIOR_CLASS_ID,
            false,
            RetailRuntimeValue::Unresolved,
        );
        let mut world_fx = WorldFx::new();

        let Type47StandardDeathOutcome::Published(publication) =
            publish_standard_death_for_test(&mut manager, &metadata, &mut world_fx).unwrap()
        else {
            panic!("fresh local Wander must publish")
        };

        assert_eq!(publication.owner.authored_spawn_index(), 12);
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            entity.collision.state_flags_at_0x08,
            RetailStateWord::exact(0x0143_C805)
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::CommonDying(_))
        ));
        assert_eq!(world_fx.pending_event_count(), 1);
    }

    #[test]
    fn guard_publication_preserves_high_selector_and_orders_constructor_effects() {
        let metadata = exact_metadata();
        let mut manager = exact_manager(13, TYPE47_GUARD_BEHAVIOR_CLASS_ID, true);
        let old_primary = snapshot(&manager).task_ids[ActorTaskSlot::Primary as usize].unwrap();
        let mut world_fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
        let expected_sample = oracle.next_shared_retail_random_u16();
        let expected_next = oracle.next_shared_retail_random_u16();
        let random_byte = i32::from(expected_sample >> 8);
        let expected_target = 300 + random_byte * 300 / 0x0A00;
        let mut effects = Vec::new();

        let publication = publish_for_test(&mut manager, &metadata, &mut world_fx, |effect| {
            effects.push(effect)
        })
        .unwrap()
        .unwrap();

        assert_eq!(publication.owner.entity_id(), ENTITY_ID);
        assert_eq!(publication.owner.authored_spawn_index(), 13);
        assert_eq!(publication.owner.visit().slot, ActorTaskSlot::Primary);
        assert_ne!(publication.owner.visit().task_id, old_primary);
        assert_eq!(
            publication.constructor,
            Type47CommonDyingConstructorEvidence {
                random_sample_low16: expected_sample,
                sub_a_target_speed_raw: expected_target,
            }
        );
        assert_eq!(
            effects,
            vec![
                CommonDyingConstructorEffect::WriteSubHState08 { value: 1 },
                CommonDyingConstructorEffect::WriteSubADirection {
                    direction_multiplier: 1,
                },
                CommonDyingConstructorEffect::WriteSubATargetSpeed {
                    target_speed_raw: expected_target,
                    random_sample_low16: expected_sample,
                },
                CommonDyingConstructorEffect::WriteOwnerVerticalVelocity { velocity_raw: 500 },
                CommonDyingConstructorEffect::WriteSubADirection {
                    direction_multiplier: 1,
                },
                CommonDyingConstructorEffect::WriteSubHState08 { value: 0 },
            ]
        );
        assert_eq!(world_fx.next_shared_retail_random_u16(), expected_next);

        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert_eq!(entity.velocity_raw(), [11, 500, 33]);
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(ACTIVE_MODEL_SLOT_LOW_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT),
            RetailRuntimeValue::Known(
                ACTIVE_MODEL_SLOT_LOW_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT
            )
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08,
            RetailStateWord::exact(0x0143_E805)
        );
        assert_eq!(
            entity
                .current_behavior_context
                .map(|context| { context.map(|context| context.active_style()) }),
            RetailRuntimeValue::Known(Some(ActiveBehaviorStyle::Audited(
                COMMON_ACTOR_DYING_ACTIVE_STYLE
            )))
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::CommonDying(_))
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("Sub-A disappeared")
        };
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(expected_target)
        );
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(sub_a.drive_scale_percent(), 100);
        let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime else {
            panic!("Sub-H disappeared")
        };
        assert!(!sub_h.is_enabled());
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
    }

    #[test]
    fn wander_publication_accepts_the_primary_only_graph() {
        let metadata = exact_metadata();
        let mut manager = exact_manager(12, TYPE47_WANDER_BEHAVIOR_CLASS_ID, false);
        let mut world_fx = WorldFx::new();

        let publication = publish_for_test(&mut manager, &metadata, &mut world_fx, |_| {})
            .unwrap()
            .unwrap();

        assert_eq!(publication.owner.authored_spawn_index(), 12);
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::CommonDying(_))
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        assert_eq!(
            entity.collision.state_flags_at_0x08,
            RetailStateWord::exact(0x0143_C805)
        );
    }

    #[test]
    fn standard_death_no_ops_are_strict_and_remote_wins_over_dying() {
        let mut metadata = exact_metadata();
        metadata.common_mover_topology = RetailRuntimeValue::Unresolved;

        let mut remote = exact_fresh_manager(
            11,
            TYPE47_GUARD_BEHAVIOR_CLASS_ID,
            false,
            RetailRuntimeValue::Known(2_000),
        );
        {
            let entity = remote.entity_mut_for_test(ENTITY_ID).unwrap();
            entity.actor_tasks.clear_slot(ActorTaskSlot::Primary);
            entity.collision.state_flags_at_0x08.overwrite(
                REMOTE_OWNED_STATE_BIT | ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
                REMOTE_OWNED_STATE_BIT | ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
            );
        }
        let remote_before = snapshot(&remote);
        let mut remote_fx = WorldFx::new();
        let mut remote_oracle = WorldFx::new();
        assert_eq!(
            publish_standard_death_for_test(&mut remote, &metadata, &mut remote_fx).unwrap(),
            Type47StandardDeathOutcome::RemoteOwnedNoOp
        );
        assert_eq!(snapshot(&remote), remote_before);
        assert_eq!(remote_fx.pending_event_count(), 0);
        assert_eq!(
            remote_fx.next_shared_retail_random_u16(),
            remote_oracle.next_shared_retail_random_u16()
        );

        let mut already_dying = exact_fresh_manager(
            12,
            TYPE47_WANDER_BEHAVIOR_CLASS_ID,
            false,
            RetailRuntimeValue::Known(-27),
        );
        {
            let entity = already_dying.entity_mut_for_test(ENTITY_ID).unwrap();
            entity.actor_tasks.clear_slot(ActorTaskSlot::Primary);
            entity.collision.state_flags_at_0x08.overwrite(
                REMOTE_OWNED_STATE_BIT | ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
                ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
            );
        }
        let dying_before = snapshot(&already_dying);
        let mut dying_fx = WorldFx::new();
        let mut dying_oracle = WorldFx::new();
        assert_eq!(
            publish_standard_death_for_test(&mut already_dying, &metadata, &mut dying_fx).unwrap(),
            Type47StandardDeathOutcome::AlreadyDyingNoOp
        );
        assert_eq!(snapshot(&already_dying), dying_before);
        assert_eq!(dying_fx.pending_event_count(), 0);
        assert_eq!(
            dying_fx.next_shared_retail_random_u16(),
            dying_oracle.next_shared_retail_random_u16()
        );
    }

    #[test]
    fn standard_death_no_op_still_requires_exact_type47_custody() {
        let metadata = exact_metadata();
        let mut manager = exact_fresh_manager(
            11,
            TYPE47_GUARD_BEHAVIOR_CLASS_ID,
            false,
            RetailRuntimeValue::Known(2_000),
        );
        let entity = manager.entity_mut_for_test(ENTITY_ID).unwrap();
        entity.entity_type = 48;
        entity.collision.state_flags_at_0x08.overwrite(
            REMOTE_OWNED_STATE_BIT | ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
            REMOTE_OWNED_STATE_BIT | ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
        );
        let before = snapshot(&manager);
        let mut world_fx = WorldFx::new();

        assert_eq!(
            publish_standard_death_for_test(&mut manager, &metadata, &mut world_fx).unwrap_err(),
            Type47CommonDyingPublicationError::UnexpectedEntityType { actual: 48 }
        );
        assert_eq!(snapshot(&manager), before);
        assert_eq!(world_fx.pending_event_count(), 0);
    }

    #[test]
    fn standard_death_dispatch_rejection_is_zero_mutation_audio_and_rng() {
        let metadata = exact_metadata();
        let mut manager = exact_fresh_manager(
            11,
            TYPE47_GUARD_BEHAVIOR_CLASS_ID,
            false,
            RetailRuntimeValue::Known(2_000),
        );
        manager
            .entity_mut_for_test(ENTITY_ID)
            .unwrap()
            .actor_tasks
            .clear_slot(ActorTaskSlot::Primary);
        let before = snapshot(&manager);
        let mut world_fx = WorldFx::new();
        let mut oracle = WorldFx::new();

        assert_eq!(
            publish_standard_death_for_test(&mut manager, &metadata, &mut world_fx).unwrap_err(),
            Type47CommonDyingPublicationError::MissingTask {
                slot: ActorTaskSlot::Primary,
            }
        );
        assert_eq!(snapshot(&manager), before);
        assert_eq!(world_fx.pending_event_count(), 0);
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }

    #[test]
    fn metadata_rejection_is_zero_mutation_and_zero_rng() {
        let mut metadata = exact_metadata();
        metadata.sub_h_external_frame_descriptor =
            RetailRuntimeValue::Known(Some(SubHExternalFrameDescriptor {
                completion_sound_id: None,
                records: {
                    let mut records = TYPE47_COMMON_DYING_SUB_H_RECORDS.to_vec();
                    records[3].vertex_refs[0] = 150;
                    records
                },
            }));
        let mut manager = exact_manager(11, TYPE47_GUARD_BEHAVIOR_CLASS_ID, false);
        let before = snapshot(&manager);
        let mut world_fx = WorldFx::new();
        let mut oracle = WorldFx::new();

        let error = publish_for_test(&mut manager, &metadata, &mut world_fx, |_| {}).unwrap_err();

        assert_eq!(
            error,
            Type47CommonDyingPublicationError::UnexpectedSubHDescriptorRecords
        );
        assert_eq!(snapshot(&manager), before);
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }

    #[test]
    fn wrong_constructor_drive_scale_is_zero_mutation_and_zero_rng() {
        let metadata = exact_metadata();
        let mut manager = exact_manager(11, TYPE47_GUARD_BEHAVIOR_CLASS_ID, false);
        manager
            .entity_mut_for_test(ENTITY_ID)
            .unwrap()
            .sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(777), -1, 73),
        ));
        let before = snapshot(&manager);
        let mut world_fx = WorldFx::new();
        let mut oracle = WorldFx::new();

        let error = publish_for_test(&mut manager, &metadata, &mut world_fx, |_| {}).unwrap_err();

        assert_eq!(
            error,
            Type47CommonDyingPublicationError::UnexpectedSubADriveScale { actual: 73 }
        );
        assert_eq!(snapshot(&manager), before);
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }

    #[test]
    fn absent_and_mismatched_task_graphs_are_rejected_before_rng() {
        let metadata = exact_metadata();
        let mut absent = exact_manager(11, TYPE47_GUARD_BEHAVIOR_CLASS_ID, false);
        absent
            .entity_mut_for_test(ENTITY_ID)
            .unwrap()
            .actor_tasks
            .clear_slot(ActorTaskSlot::Primary);
        let absent_before = snapshot(&absent);
        let mut absent_fx = WorldFx::new();
        assert_eq!(
            publish_for_test(&mut absent, &metadata, &mut absent_fx, |_| {}).unwrap_err(),
            Type47CommonDyingPublicationError::MissingTask {
                slot: ActorTaskSlot::Primary
            }
        );
        assert_eq!(snapshot(&absent), absent_before);

        let mut mismatched = exact_manager(11, TYPE47_GUARD_BEHAVIOR_CLASS_ID, false);
        let replacement = *mismatched
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap()
            .actor_task_state(ActorTaskSlot::Primary)
            .unwrap();
        mismatched
            .entity_mut_for_test(ENTITY_ID)
            .unwrap()
            .actor_tasks
            .replace_prepared(
                ActorTaskSlot::Secondary,
                PreparedActorTask::new(replacement),
            );
        assert!(matches!(
            publish_for_test(&mut mismatched, &metadata, &mut WorldFx::new(), |_| {}),
            Err(Type47CommonDyingPublicationError::TaskFamilyMismatch {
                slot: ActorTaskSlot::Secondary,
                expected: ActorTaskRuntimeFamily::GuardLocationAcquisition,
                actual: ActorTaskRuntimeFamily::OrdinaryType9Wander,
            })
        ));
    }

    #[test]
    fn in_callback_wrapper_is_rejected_without_disturbing_the_wrapper() {
        let metadata = exact_metadata();
        let mut manager = exact_manager(12, TYPE47_WANDER_BEHAVIOR_CLASS_ID, false);
        let primary = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let visit = ActorTaskVisit {
            slot: ActorTaskSlot::Primary,
            task_id: primary,
        };
        manager
            .entity_mut_for_test(ENTITY_ID)
            .unwrap()
            .actor_tasks
            .begin_exact_visit_with(visit, |_| ())
            .unwrap();

        let error =
            publish_for_test(&mut manager, &metadata, &mut WorldFx::new(), |_| {}).unwrap_err();
        assert_eq!(
            error,
            Type47CommonDyingPublicationError::TaskWrapperNotRunnable {
                slot: ActorTaskSlot::Primary,
                flags: ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: true,
                },
            }
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert_eq!(
            entity.actor_tasks.wrapper_flags(primary),
            Some(ActorTaskWrapperFlags {
                alive: true,
                in_callback: true,
            })
        );
    }

    #[test]
    fn injected_final_prepare_failure_clears_secondary_but_preserves_primary_and_rng() {
        let metadata = exact_metadata();
        let mut manager = exact_manager(11, TYPE47_GUARD_BEHAVIOR_CLASS_ID, false);
        let old_primary = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let animation_offset_before = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap()
            .collision
            .animation_offset_at_0xb2;
        let mut world_fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        let mut effects = Vec::new();

        let failure = publish_fresh_level_one_type47_common_dying_with_prepare(
            &mut manager,
            ENTITY_ID,
            &metadata,
            &mut world_fx,
            true,
            |_| Err(()),
            |effect| effects.push(effect),
        )
        .unwrap()
        .unwrap_err();

        assert_eq!(failure.action_index, 2);
        assert_eq!(failure.slot, ActorTaskSlot::Primary);
        assert!(effects.is_empty());
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ENTITY_ID)
            .unwrap();
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            Some(old_primary)
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::OrdinaryType9Wander(_))
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        assert_eq!(entity.velocity_raw(), INITIAL_VELOCITY_RAW);
        assert_eq!(
            entity.collision.animation_offset_at_0xb2, animation_offset_before,
            "a failed setup never publishes callback-mass provenance"
        );
        let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime else {
            panic!("Sub-H disappeared")
        };
        assert!(sub_h.is_enabled());
        assert_eq!(
            entity
                .current_behavior_context
                .map(|context| { context.map(|context| context.active_style()) }),
            RetailRuntimeValue::Known(Some(ActiveBehaviorStyle::Audited(
                COMMON_ACTOR_DYING_ACTIVE_STYLE
            )))
        );
    }

    #[test]
    fn public_entry_rejects_a_generic_manager_before_mutation() {
        let metadata = exact_metadata();
        let mut manager = exact_manager(11, TYPE47_GUARD_BEHAVIOR_CLASS_ID, false);
        let before = snapshot(&manager);
        let mut world_fx = WorldFx::new();
        let mut oracle = WorldFx::new();

        let error = publish_fresh_level_one_type47_common_dying(
            &mut manager,
            ENTITY_ID,
            &metadata,
            &mut world_fx,
        )
        .unwrap_err();

        assert_eq!(
            error,
            Type47CommonDyingPublicationError::NotFreshNewGameFirstWorld
        );
        assert_eq!(snapshot(&manager), before);
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }
}
