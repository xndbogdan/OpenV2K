//! Detached first-world type-13 `FUN_00425680` birth plan and class-7
//! variant-0 `FUN_0040B6C0` publication.
//!
//! Type 13 authors Always x1 class 5 (`Move About Aimlessly`) and Always x3
//! class 7 (`Search And Attack Target`). Both evaluators are Always, so the
//! selector consumes one shared word and needs no nearby walks. A class-7
//! result publishes C6B0 style `0x004C7A50` and the B6C0 acquire/wander graph.
//! A class-5 result publishes C6B0 style `0x004C7930` and `FUN_0040ACD0`:
//! clear slots 2 then 1, then `FUN_00402B10(entity, 0, 5000)`.
//! Each successful B6C0 phase applies type 13's `FUN_00406070` Sub-G suffix
//! (`FUN_0041B970/1B940/1B980/00424380`) and consumes one shared word. It does
//! not publish ADE0/Chase/Aim (no target yet), and leaves `FUN_00401430` to
//! the live scheduler,
//! and does not invent Sub-A, unwritten Sub-G mover fields, or the class-12
//! `FUN_00404120` tail. Each successful suffix writes the recovered live
//! `FUN_00406070` Sub-G words. A later live
//! visit may drive the published B6C0 graph through recovered C7D0/ADE0.
//! Captured Intro2 production publishes spawn 0's class-7 B6C0 graph and can
//! later retain the class-5 graph selected by C690. The cohort scheduler owns
//! both SharedRetarget callbacks and their Type-13 common-mover transaction.
//! Detached metadata birth sequences `FUN_0041B8C0`, then `FUN_00425680`,
//! then B6C0/ACD0 from one caller RNG stream. It does not hook `WorldFx`.

use crate::actor_task_dispatcher::{prepare_target_acquisition_runtime_task, ActorTaskRuntime};
use crate::actor_task_owner::{ActorTaskId, ActorTaskSlot, PreparedActorTask};
use crate::entity::{Entity, EntityKind, EntityManager};
use crate::entity_behavior::{
    behavior_program, initial_behavior_state_policy, select_initial_behavior,
    BehaviorChoiceListSource, BehaviorContextRuntime, BehaviorDescriptorIdentity,
    BehaviorSelection, BehaviorSelectionError, BehaviorWeightRule,
};
use crate::entity_collision_state::{
    EntityCollisionRuntimeState, EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord,
};
use crate::search_attack::{
    search_attack_variant_setup, SearchAttackCandidateFilter, SearchAttackRadius,
    SearchAttackTaskLifetime, SearchAttackTaskRole, SearchAttackVariant,
    SEARCH_ATTACK_BEHAVIOR_CLASS_ID,
};
use crate::search_attack_live::{
    apply_search_attack_acquisition_live, SearchAttackLiveAcquisitionOutcome,
    SearchAttackLiveHandoffRequirement, SearchAttackLiveSamePassAim,
    TYPE13_SEARCH_ATTACK_ALTERNATE_BEHAVIOR_CLASS, TYPE13_SEARCH_ATTACK_BEHAVIOR_CHOICES,
    TYPE13_SEARCH_ATTACK_BEHAVIOR_RULE_REF, TYPE13_SEARCH_ATTACK_COMMON_AXIS,
    TYPE13_SEARCH_ATTACK_SUB_G, TYPE13_SEARCH_ATTACK_SUB_G_RANDOMIZED_TARGET_BASE_RAW_AT_0X0C,
    TYPE13_SEARCH_ATTACK_SUB_G_SOURCE_RAW_AT_0X00, TYPE13_SEARCH_ATTACK_TOPOLOGY,
};
use crate::search_attack_owner::{
    apply_search_attack_task_setup, SearchAttackTaskPreparation, SearchAttackTaskSetupError,
    SearchAttackTaskSetupRequest,
};
use crate::shared_retarget_mover::SharedRetargetTaskState;
use crate::sub_g_runtime::SubG06070RuntimeState;
use crate::world_fx::WorldFx;

pub const TYPE13_MOVE_ABOUT_AIMLESSLY_CLASS_ID: u8 = 5;
pub const TYPE13_MOVE_ABOUT_AIMLESSLY_STYLE_ADDRESS: u32 = 0x004C_7930;
pub const TYPE13_MOVE_ABOUT_AIMLESSLY_INITIALIZER_ADDRESS: u32 = 0x0040_ACD0;
pub const TYPE13_ENTITY_TYPE: u32 = 13;
const TYPE13_CLASS5_AIMLESS_WANDER_LIFETIME_MS: u32 = 5_000;
const TYPE13_CLASS7_ACQUIRING_WANDER_LIFETIME_MS: u32 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type13WeightedSelection {
    pub selection: BehaviorSelection,
    pub random_sample_low16: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type13B6c0SubGConstructorEvidence {
    pub random_sample_low16: u16,
    pub target_raw_at_0x38: i32,
    pub source_raw_at_0x00: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type13Class7AcquiringPublication {
    pub entity_id: u32,
    pub planned: Type13WeightedSelection,
    pub style_address: u32,
    pub secondary_task_id: ActorTaskId,
    pub primary_task_id: ActorTaskId,
    pub constructors_by_phase: [Type13B6c0SubGConstructorEvidence; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type13Class5AimlessPublication {
    pub entity_id: u32,
    pub planned: Type13WeightedSelection,
    pub style_address: u32,
    pub primary_task_id: ActorTaskId,
    pub constructor: Type13B6c0SubGConstructorEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type13InitialBehaviorPublication {
    MoveAboutAimlessly(Type13Class5AimlessPublication),
    SearchAndAttack(Type13Class7AcquiringPublication),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type13InitialBehaviorError {
    MissingInitializer,
    UnexpectedBehaviorChoices,
    UnexpectedBehaviorRuleRef { actual: u32 },
    UnexpectedAlternateBehaviorClass { actual: u32 },
    UnexpectedSelectedClass { actual: u8 },
    Plan(BehaviorSelectionError),
}

/// Detached type-13 spawn: `FUN_0041B8C0`, then `FUN_00425680`, then B6C0/ACD0.
pub struct Type13MetadataBirth {
    pub entity: Entity,
    pub constructor_sample_low16: u16,
    pub publication: Type13InitialBehaviorPublication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type13MetadataBirthError {
    Construction(Type13Class7AcquiringError),
    Plan(Type13InitialBehaviorError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type13Class7AcquiringError {
    EntityInactive,
    UnexpectedEntityType { actual: u32 },
    CurrentBehaviorContextAlreadyResolved,
    CurrentBehaviorContextUnresolved,
    CurrentBehaviorContextAbsent,
    CurrentBehaviorContextNotTypeDefault,
    ActorCommonAxisDescriptorUnresolved,
    MissingInitializer,
    UnexpectedMetadataCommonAxis,
    ComponentTopologyUnresolved,
    UnexpectedComponentTopology,
    MoveAboutAimlesslyUninvented,
    UnexpectedSelectedClass { actual: u8 },
    CanonicalFreshContextUnavailable,
    CanonicalReselectedContextUnavailable,
    UnexpectedInitializerArgument,
    SubGPayloadUnresolved,
    SubGPayloadAbsent,
    UnexpectedSubGPayload,
    SubGRuntimeUnresolved,
    SubGRuntimeAbsent,
    TaskContractMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type13Class7LiveAcquisitionError {
    MissingOwner,
    UnexpectedEntityType { actual: u32 },
    UnpublishedB6c0Graph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Type13Class5GraphError {
    UnexpectedEntityType { actual: u32 },
    UnpublishedAimlessGraph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Type13Class7PursuingGraphError {
    UnexpectedEntityType { actual: u32 },
    UnpublishedPursuingGraph,
}

/// Authenticate the recovered type-13 list and consume one selector word.
///
/// No entity state changes. The Always x1 / Always x3 pair maps `r16 < 0x4000`
/// to class 5 and every larger low word to class 7.
pub fn plan_type13_initial_behavior(
    metadata: &EntityTypeRuntimeMetadata,
    mut next_random: impl FnMut() -> u32,
) -> Result<Type13WeightedSelection, Type13InitialBehaviorError> {
    authenticate_type13_behavior_list(metadata)?;
    let initializer = metadata
        .initializer
        .as_ref()
        .expect("type-13 behavior-list preflight retained the initializer");

    let mut sample = 0_u16;
    let selection = select_initial_behavior(
        &initializer.behavior_choices,
        |rule| i32::from(rule == BehaviorWeightRule::Always),
        || {
            let word = next_random();
            sample = word as u16;
            word
        },
    )
    .map_err(Type13InitialBehaviorError::Plan)?
    .ok_or(Type13InitialBehaviorError::Plan(
        BehaviorSelectionError::UnknownBehaviorClass { raw: 0 },
    ))?;
    if !matches!(
        selection.program.class_id,
        TYPE13_MOVE_ABOUT_AIMLESSLY_CLASS_ID | SEARCH_ATTACK_BEHAVIOR_CLASS_ID
    ) {
        return Err(Type13InitialBehaviorError::UnexpectedSelectedClass {
            actual: selection.program.class_id,
        });
    }
    Ok(Type13WeightedSelection {
        selection,
        random_sample_low16: sample,
    })
}

/// Metadata-backed type-13 construction through `FUN_0041B8C0`.
///
/// Authenticates the recovered Section-12 type-13 record, then applies the
/// Sub-G birth constructor with one caller-supplied word. Behavior
/// publication and `FUN_00401430` remain caller-owned.
pub fn construct_type13_from_metadata(
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    position_raw: [i16; 3],
    random_sample_low16: u16,
) -> Result<Entity, Type13Class7AcquiringError> {
    authenticate_type13_metadata(metadata)?;
    let initializer = metadata
        .initializer
        .as_ref()
        .expect("type-13 metadata preflight retained the initializer");
    let mut entity = Entity::unresolved_port_entity(id, EntityKind::Enemy, TYPE13_ENTITY_TYPE);
    entity.set_position_raw(position_raw);
    entity.mass_raw = metadata.mass_raw;
    entity.capability_flags = metadata.capability_flags;
    entity.model_slots = metadata.model_slots.map(|model| {
        let model = usize::from(model);
        (model != 0).then_some(model)
    });
    entity.collision =
        EntityCollisionRuntimeState::from_constructor(Some(metadata), 0, RetailStateWord::exact(1));
    entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
    entity.actor_common_axis_descriptor =
        RetailRuntimeValue::Known(initializer.common_axis_descriptor);
    entity.sub_g_06070_runtime =
        RetailRuntimeValue::Known(Some(SubG06070RuntimeState::from_1b8c0_constructor(
            &TYPE13_SEARCH_ATTACK_SUB_G,
            random_sample_low16,
        )));
    Ok(entity)
}

/// Detached type-13 birth: `FUN_0041B8C0`, then `FUN_00425680`, then B6C0/ACD0.
///
/// One caller RNG stream supplies the constructor word, the selector word,
/// and each successful `FUN_00406070` suffix word. Class 5 consumes three
/// words; class 7 consumes four. The function does not take `WorldFx` and
/// does not invent `FUN_00401430`.
pub fn birth_type13_from_metadata(
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    position_raw: [i16; 3],
    mut next_random: impl FnMut() -> u32,
) -> Result<Type13MetadataBirth, Type13MetadataBirthError> {
    authenticate_type13_metadata(metadata).map_err(Type13MetadataBirthError::Construction)?;
    authenticate_type13_behavior_list(metadata).map_err(Type13MetadataBirthError::Plan)?;
    let constructor_sample_low16 = next_random() as u16;
    let mut entity =
        construct_type13_from_metadata(id, metadata, position_raw, constructor_sample_low16)
            .map_err(Type13MetadataBirthError::Construction)?;
    let planned = plan_type13_initial_behavior(metadata, &mut next_random)
        .map_err(Type13MetadataBirthError::Plan)?;
    let publication =
        publish_type13_initial_behavior(&mut entity, metadata, planned, &mut next_random)
            .map_err(Type13MetadataBirthError::Construction)?;
    Ok(Type13MetadataBirth {
        entity,
        constructor_sample_low16,
        publication,
    })
}

pub(crate) fn authenticate_type13_behavior_list(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type13InitialBehaviorError> {
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(Type13InitialBehaviorError::MissingInitializer)?;
    if initializer.behavior_choices.as_ref() != TYPE13_SEARCH_ATTACK_BEHAVIOR_CHOICES.as_slice() {
        return Err(Type13InitialBehaviorError::UnexpectedBehaviorChoices);
    }
    if initializer.behavior_rule_ref != TYPE13_SEARCH_ATTACK_BEHAVIOR_RULE_REF {
        return Err(Type13InitialBehaviorError::UnexpectedBehaviorRuleRef {
            actual: initializer.behavior_rule_ref,
        });
    }
    if initializer.alternate_behavior_class_ref != TYPE13_SEARCH_ATTACK_ALTERNATE_BEHAVIOR_CLASS {
        return Err(
            Type13InitialBehaviorError::UnexpectedAlternateBehaviorClass {
                actual: initializer.alternate_behavior_class_ref,
            },
        );
    }
    Ok(())
}

pub(crate) fn authenticate_type13_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type13Class7AcquiringError> {
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(Type13Class7AcquiringError::MissingInitializer)?;
    if initializer.common_axis_descriptor != TYPE13_SEARCH_ATTACK_COMMON_AXIS {
        return Err(Type13Class7AcquiringError::UnexpectedMetadataCommonAxis);
    }
    match metadata.common_mover_topology {
        RetailRuntimeValue::Unresolved => {
            return Err(Type13Class7AcquiringError::ComponentTopologyUnresolved);
        }
        RetailRuntimeValue::Known(topology) if topology == TYPE13_SEARCH_ATTACK_TOPOLOGY => {}
        RetailRuntimeValue::Known(_) => {
            return Err(Type13Class7AcquiringError::UnexpectedComponentTopology);
        }
    }
    match metadata.common_mover_gkl_payloads {
        RetailRuntimeValue::Unresolved => Err(Type13Class7AcquiringError::SubGPayloadUnresolved),
        RetailRuntimeValue::Known(payloads) => match payloads.sub_g {
            None => Err(Type13Class7AcquiringError::SubGPayloadAbsent),
            Some(bytes) if bytes == TYPE13_SEARCH_ATTACK_SUB_G => Ok(()),
            Some(_) => Err(Type13Class7AcquiringError::UnexpectedSubGPayload),
        },
    }
}

/// Publish the selected type-13 class-5 or class-7 birth graph.
pub fn publish_type13_initial_behavior(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    planned: Type13WeightedSelection,
    next_random: impl FnMut() -> u32,
) -> Result<Type13InitialBehaviorPublication, Type13Class7AcquiringError> {
    match planned.selection.program.class_id {
        TYPE13_MOVE_ABOUT_AIMLESSLY_CLASS_ID => {
            publish_type13_class5_aimlessly(entity, metadata, planned, next_random)
                .map(Type13InitialBehaviorPublication::MoveAboutAimlessly)
        }
        SEARCH_ATTACK_BEHAVIOR_CLASS_ID => {
            publish_type13_class7_acquiring(entity, metadata, planned, next_random)
                .map(Type13InitialBehaviorPublication::SearchAndAttack)
        }
        actual => Err(Type13Class7AcquiringError::UnexpectedSelectedClass { actual }),
    }
}

/// Publish a C690/AC60 TypeDefault reselection into the entity's existing
/// behavior-context allocation.
///
/// `FUN_0040ABB0` replaces descriptor/style identity while retaining context
/// target `+0x08` and auxiliary `+0x0C`. The selected initializer then owns
/// the same state, Sub-G, RNG, and task mutation order as fresh publication.
/// `initial_behavior` remains birth provenance and is never rewritten here.
pub(crate) fn publish_type13_reselected_initial_behavior(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    planned: Type13WeightedSelection,
    next_random: impl FnMut() -> u32,
) -> Result<Type13InitialBehaviorPublication, Type13Class7AcquiringError> {
    if !matches!(
        planned.selection.program.class_id,
        TYPE13_MOVE_ABOUT_AIMLESSLY_CLASS_ID | SEARCH_ATTACK_BEHAVIOR_CLASS_ID
    ) {
        return Err(Type13Class7AcquiringError::UnexpectedSelectedClass {
            actual: planned.selection.program.class_id,
        });
    }
    let current_context = authenticate_type13_reselected_publication_surface(entity, metadata)?;

    match planned.selection.program.class_id {
        TYPE13_MOVE_ABOUT_AIMLESSLY_CLASS_ID => {
            authenticate_type13_class5_selection(planned)?;
            let selected_context = current_context
                .reselect_named_type_default(
                    planned.selection.program,
                    planned.selection.program.initial_style_table_index_raw,
                    planned.selection.program.initial_style,
                )
                .ok_or(Type13Class7AcquiringError::CanonicalReselectedContextUnavailable)?;
            publish_type13_class5_aimlessly_with_context(
                entity,
                planned,
                selected_context,
                next_random,
            )
            .map(Type13InitialBehaviorPublication::MoveAboutAimlessly)
        }
        SEARCH_ATTACK_BEHAVIOR_CLASS_ID => {
            authenticate_type13_class7_selection(planned)?;
            let selected_context = current_context
                .reselect_named_type_default(
                    planned.selection.program,
                    planned.selection.program.initial_style_table_index_raw,
                    planned.selection.program.initial_style,
                )
                .ok_or(Type13Class7AcquiringError::CanonicalReselectedContextUnavailable)?;
            publish_type13_class7_acquiring_with_context(
                entity,
                metadata,
                planned,
                selected_context,
                next_random,
            )
            .map(Type13InitialBehaviorPublication::SearchAndAttack)
        }
        _ => unreachable!("type-13 reselection admitted only class 5 or class 7"),
    }
}

fn authenticate_type13_class5_selection(
    planned: Type13WeightedSelection,
) -> Result<(), Type13Class7AcquiringError> {
    if planned.selection.program.class_id != TYPE13_MOVE_ABOUT_AIMLESSLY_CLASS_ID {
        return Err(Type13Class7AcquiringError::UnexpectedSelectedClass {
            actual: planned.selection.program.class_id,
        });
    }
    if planned.selection.program
        != behavior_program(u32::from(TYPE13_MOVE_ABOUT_AIMLESSLY_CLASS_ID)).ok_or(
            Type13Class7AcquiringError::UnexpectedSelectedClass {
                actual: TYPE13_MOVE_ABOUT_AIMLESSLY_CLASS_ID,
            },
        )?
    {
        return Err(Type13Class7AcquiringError::UnexpectedSelectedClass {
            actual: planned.selection.program.class_id,
        });
    }
    if planned.selection.program.initializer_callback_address
        != TYPE13_MOVE_ABOUT_AIMLESSLY_INITIALIZER_ADDRESS
        || planned.selection.program.initializer_argument_raw != RetailRuntimeValue::Known(0)
    {
        return Err(Type13Class7AcquiringError::UnexpectedInitializerArgument);
    }
    Ok(())
}

fn authenticate_type13_class7_selection(
    planned: Type13WeightedSelection,
) -> Result<(), Type13Class7AcquiringError> {
    match planned.selection.program.class_id {
        TYPE13_MOVE_ABOUT_AIMLESSLY_CLASS_ID => {
            return Err(Type13Class7AcquiringError::MoveAboutAimlesslyUninvented);
        }
        SEARCH_ATTACK_BEHAVIOR_CLASS_ID => {}
        actual => {
            return Err(Type13Class7AcquiringError::UnexpectedSelectedClass { actual });
        }
    }
    if planned.selection.program
        != behavior_program(u32::from(SEARCH_ATTACK_BEHAVIOR_CLASS_ID)).ok_or(
            Type13Class7AcquiringError::UnexpectedSelectedClass {
                actual: SEARCH_ATTACK_BEHAVIOR_CLASS_ID,
            },
        )?
    {
        return Err(Type13Class7AcquiringError::UnexpectedSelectedClass {
            actual: planned.selection.program.class_id,
        });
    }
    if planned.selection.program.initializer_argument_raw != RetailRuntimeValue::Known(0) {
        return Err(Type13Class7AcquiringError::UnexpectedInitializerArgument);
    }
    Ok(())
}

/// Publish class-5 `FUN_0040ACD0` after a class-5 plan.
///
/// Retail clears slot 2, clears slot 1, then calls
/// `FUN_00402B10(entity, 0, 5000)`. That constructor runs `FUN_00406030`
/// (including type 13's Sub-G `FUN_00406070`) and publishes Primary.
/// It does not copy common-axis `+0x04`. `FUN_00401430` belongs to a later
/// live scheduler visit of this Primary, not to publication.
pub fn publish_type13_class5_aimlessly(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    planned: Type13WeightedSelection,
    mut next_random: impl FnMut() -> u32,
) -> Result<Type13Class5AimlessPublication, Type13Class7AcquiringError> {
    authenticate_type13_class7_publication_surface(entity, metadata)?;
    authenticate_type13_class5_selection(planned)?;
    let selected_context = BehaviorContextRuntime::from_fresh_weighted_selection(planned.selection)
        .ok_or(Type13Class7AcquiringError::CanonicalFreshContextUnavailable)?;
    debug_assert_eq!(
        selected_context.active_style().style_address(),
        TYPE13_MOVE_ABOUT_AIMLESSLY_STYLE_ADDRESS
    );

    publish_type13_class5_aimlessly_with_context(
        entity,
        planned,
        selected_context,
        &mut next_random,
    )
}

fn publish_type13_class5_aimlessly_with_context(
    entity: &mut Entity,
    planned: Type13WeightedSelection,
    selected_context: BehaviorContextRuntime,
    mut next_random: impl FnMut() -> u32,
) -> Result<Type13Class5AimlessPublication, Type13Class7AcquiringError> {
    debug_assert_eq!(
        selected_context.active_style().style_address(),
        TYPE13_MOVE_ABOUT_AIMLESSLY_STYLE_ADDRESS
    );

    entity.current_behavior_context = RetailRuntimeValue::Known(Some(selected_context));
    let policy = initial_behavior_state_policy(planned.selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);

    let owner_position_raw = entity.position_raw();
    let constructor = {
        let Entity {
            actor_tasks,
            sub_g_06070_runtime,
            ..
        } = entity;
        let RetailRuntimeValue::Known(Some(sub_g)) = sub_g_06070_runtime else {
            unreachable!("type-13 class-5 preflight retained Sub-G 06070 storage");
        };
        actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
        actor_tasks.clear_slot(ActorTaskSlot::Secondary);
        let evidence = apply_type13_b6c0_06070_sub_g_suffix(&mut next_random);
        sub_g.apply_shared_06070_sub_g_branch(
            evidence.target_raw_at_0x38,
            evidence.source_raw_at_0x00,
        );
        actor_tasks.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                SharedRetargetTaskState::new(
                    owner_position_raw,
                    TYPE13_CLASS5_AIMLESS_WANDER_LIFETIME_MS,
                ),
            )),
        );
        evidence
    };
    let primary_task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(Type13Class7AcquiringError::TaskContractMismatch)?;
    if !matches!(
        entity.actor_tasks.state_in_slot(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::SharedRetarget(_))
    ) || entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .is_some()
        || entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary)
            .is_some()
    {
        return Err(Type13Class7AcquiringError::TaskContractMismatch);
    }

    Ok(Type13Class5AimlessPublication {
        entity_id: entity.id,
        planned,
        style_address: TYPE13_MOVE_ABOUT_AIMLESSLY_STYLE_ADDRESS,
        primary_task_id,
        constructor,
    })
}

/// Publish class-7 variant-0 `FUN_0040B6C0` after a class-7 plan.
///
/// Retail copies type `+0x04` into the actor-local common-axis word, clears
/// slot 2, installs `FUN_00402050/00402080` in slot 1, and only then installs
/// duration-500 `FUN_00402B10/00402BA0` in slot 0. Style `+0x44` is the EXE
/// zero word. Each successful phase then runs type 13's `FUN_00406070` Sub-G
/// branch before publication. Class 5 stays fail-closed.
pub fn publish_type13_class7_acquiring(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    planned: Type13WeightedSelection,
    mut next_random: impl FnMut() -> u32,
) -> Result<Type13Class7AcquiringPublication, Type13Class7AcquiringError> {
    authenticate_type13_class7_publication_surface(entity, metadata)?;
    authenticate_type13_class7_selection(planned)?;
    let selected_context = BehaviorContextRuntime::from_fresh_weighted_selection(planned.selection)
        .ok_or(Type13Class7AcquiringError::CanonicalFreshContextUnavailable)?;
    debug_assert_eq!(
        selected_context.active_style().style_address(),
        SearchAttackVariant::Acquiring.style_address()
    );

    publish_type13_class7_acquiring_with_context(
        entity,
        metadata,
        planned,
        selected_context,
        &mut next_random,
    )
}

fn publish_type13_class7_acquiring_with_context(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    planned: Type13WeightedSelection,
    selected_context: BehaviorContextRuntime,
    mut next_random: impl FnMut() -> u32,
) -> Result<Type13Class7AcquiringPublication, Type13Class7AcquiringError> {
    debug_assert_eq!(
        selected_context.active_style().style_address(),
        SearchAttackVariant::Acquiring.style_address()
    );

    entity.current_behavior_context = RetailRuntimeValue::Known(Some(selected_context));
    let policy = initial_behavior_state_policy(planned.selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    copy_type_authored_common_axis_filter(entity, metadata);

    let initializer = metadata
        .initializer
        .as_ref()
        .expect("type-13 class-7 preflight retained the initializer");
    let radius =
        SearchAttackRadius::from_raw(initializer.common_axis_descriptor.strict_axis_limit_raw);
    let type_authored_filter =
        SearchAttackCandidateFilter::from_raw(initializer.common_axis_descriptor.raw_word_at_0x04);
    let constructor_filter_override_raw = 0;
    let owner_position_raw = entity.position_raw();
    let mut constructors_by_phase = [None; 2];
    let setup_result = {
        let Entity {
            actor_tasks,
            sub_g_06070_runtime,
            ..
        } = entity;
        let RetailRuntimeValue::Known(Some(sub_g)) = sub_g_06070_runtime else {
            unreachable!("type-13 class-7 preflight retained Sub-G 06070 storage");
        };
        apply_search_attack_task_setup(
            actor_tasks,
            SearchAttackTaskSetupRequest::Acquiring,
            |preparation| {
                let prepared = prepare_type13_class7_acquiring_task(
                    preparation,
                    owner_position_raw,
                    radius,
                    type_authored_filter,
                    constructor_filter_override_raw,
                )?;
                let evidence = apply_type13_b6c0_06070_sub_g_suffix(&mut next_random);
                sub_g.apply_shared_06070_sub_g_branch(
                    evidence.target_raw_at_0x38,
                    evidence.source_raw_at_0x00,
                );
                constructors_by_phase[preparation.phase_index] = Some(evidence);
                Ok(prepared)
            },
        )
    };
    setup_result.map_err(
        |_: SearchAttackTaskSetupError<Type13Class7AcquiringError>| {
            Type13Class7AcquiringError::TaskContractMismatch
        },
    )?;

    let secondary_task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .ok_or(Type13Class7AcquiringError::TaskContractMismatch)?;
    let primary_task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(Type13Class7AcquiringError::TaskContractMismatch)?;
    if !matches!(
        entity.actor_tasks.state_in_slot(ActorTaskSlot::Secondary),
        Some(ActorTaskRuntime::TargetAcquisition(_))
    ) || !matches!(
        entity.actor_tasks.state_in_slot(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::SharedRetarget(_))
    ) || entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Tertiary)
        .is_some()
    {
        return Err(Type13Class7AcquiringError::TaskContractMismatch);
    }

    Ok(Type13Class7AcquiringPublication {
        entity_id: entity.id,
        planned,
        style_address: SearchAttackVariant::Acquiring.style_address(),
        secondary_task_id,
        primary_task_id,
        constructors_by_phase: [
            constructors_by_phase[0].expect("successful B6C0 phase 0 applied the Sub-G suffix"),
            constructors_by_phase[1].expect("successful B6C0 phase 1 applied the Sub-G suffix"),
        ],
    })
}

/// Drive one live `FUN_00402080` visit from an already-published type-13 B6C0
/// graph.
///
/// Only the recovered acquire radius/filter/override and 500-ms wander are
/// admitted. The existing class-7 C7D0/ADE0 owner then runs; `FUN_00401430`
/// is not invented. Same-pass Aim is skipped so `intro2_type13_aim` can visit
/// the newborn Tertiary once.
pub fn apply_type13_published_class7_acquisition_live(
    manager: &mut EntityManager,
    owner_id: u32,
    elapsed_micros: u32,
    world_fx: &mut WorldFx,
) -> Result<SearchAttackLiveAcquisitionOutcome, Type13Class7LiveAcquisitionError> {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == owner_id)
        .ok_or(Type13Class7LiveAcquisitionError::MissingOwner)?;
    authenticate_published_type13_class7_b6c0_graph(entity)?;
    Ok(apply_search_attack_acquisition_live(
        manager,
        owner_id,
        elapsed_micros,
        world_fx,
        SearchAttackLiveHandoffRequirement::RequiredClass7Variant0,
        SearchAttackLiveSamePassAim::Skip,
    ))
}

pub(crate) fn authenticate_published_type13_class5_aimless_graph(
    entity: &Entity,
) -> Result<(), Type13Class5GraphError> {
    authenticate_published_type13_class5_identity(entity)?;
    authenticate_published_type13_class5_context(entity)?;
    authenticate_published_type13_class5_aimless_tasks(entity)
}

pub(crate) fn authenticate_published_type13_class5_aimless_task_graph(
    entity: &Entity,
) -> Result<(), Type13Class5GraphError> {
    authenticate_published_type13_class5_identity(entity)?;
    authenticate_published_type13_class5_aimless_tasks(entity)
}

fn authenticate_published_type13_class5_identity(
    entity: &Entity,
) -> Result<(), Type13Class5GraphError> {
    if entity.entity_type != TYPE13_ENTITY_TYPE {
        return Err(Type13Class5GraphError::UnexpectedEntityType {
            actual: entity.entity_type,
        });
    }
    Ok(())
}

fn authenticate_published_type13_class5_context(
    entity: &Entity,
) -> Result<(), Type13Class5GraphError> {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Type13Class5GraphError::UnpublishedAimlessGraph);
    };
    let Some(program) = behavior_program(u32::from(TYPE13_MOVE_ABOUT_AIMLESSLY_CLASS_ID)) else {
        return Err(Type13Class5GraphError::UnpublishedAimlessGraph);
    };
    if context.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || context.style_table_index_raw_at_0x10() != 0
        || context.active_style().style_address() != TYPE13_MOVE_ABOUT_AIMLESSLY_STYLE_ADDRESS
    {
        return Err(Type13Class5GraphError::UnpublishedAimlessGraph);
    }
    Ok(())
}

fn authenticate_published_type13_class5_aimless_tasks(
    entity: &Entity,
) -> Result<(), Type13Class5GraphError> {
    let Some(ActorTaskRuntime::SharedRetarget(wander)) =
        entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        return Err(Type13Class5GraphError::UnpublishedAimlessGraph);
    };
    if wander.lifetime_ms() != TYPE13_CLASS5_AIMLESS_WANDER_LIFETIME_MS
        || entity.actor_task_state(ActorTaskSlot::Secondary).is_some()
        || entity.actor_task_state(ActorTaskSlot::Tertiary).is_some()
    {
        return Err(Type13Class5GraphError::UnpublishedAimlessGraph);
    }
    Ok(())
}

pub(crate) fn authenticate_published_type13_class7_b6c0_graph(
    entity: &Entity,
) -> Result<(), Type13Class7LiveAcquisitionError> {
    authenticate_published_type13_class7_identity(entity)?;
    authenticate_published_type13_class7_context(entity)?;
    authenticate_published_type13_class7_b6c0_tasks(entity)
}

pub(crate) fn authenticate_published_type13_class7_b6c0_task_graph(
    entity: &Entity,
) -> Result<(), Type13Class7LiveAcquisitionError> {
    authenticate_published_type13_class7_identity(entity)?;
    authenticate_published_type13_class7_b6c0_tasks(entity)
}

fn authenticate_published_type13_class7_identity(
    entity: &Entity,
) -> Result<(), Type13Class7LiveAcquisitionError> {
    if entity.entity_type != TYPE13_ENTITY_TYPE {
        return Err(Type13Class7LiveAcquisitionError::UnexpectedEntityType {
            actual: entity.entity_type,
        });
    }
    Ok(())
}

fn authenticate_published_type13_class7_context(
    entity: &Entity,
) -> Result<(), Type13Class7LiveAcquisitionError> {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Type13Class7LiveAcquisitionError::UnpublishedB6c0Graph);
    };
    let Some(program) = behavior_program(u32::from(SEARCH_ATTACK_BEHAVIOR_CLASS_ID)) else {
        return Err(Type13Class7LiveAcquisitionError::UnpublishedB6c0Graph);
    };
    if context.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || context.style_table_index_raw_at_0x10() != 0
    {
        return Err(Type13Class7LiveAcquisitionError::UnpublishedB6c0Graph);
    }
    Ok(())
}

pub(crate) fn authenticate_published_type13_class7_pursuing_graph(
    entity: &Entity,
) -> Result<(), Type13Class7PursuingGraphError> {
    authenticate_published_type13_class7_pursuing_identity(entity)?;
    let target_id = authenticate_published_type13_class7_pursuing_context(entity)?;
    authenticate_published_type13_class7_pursuing_tasks(entity, target_id)
}

fn authenticate_published_type13_class7_pursuing_identity(
    entity: &Entity,
) -> Result<(), Type13Class7PursuingGraphError> {
    if entity.entity_type != TYPE13_ENTITY_TYPE {
        return Err(Type13Class7PursuingGraphError::UnexpectedEntityType {
            actual: entity.entity_type,
        });
    }
    Ok(())
}

fn authenticate_published_type13_class7_pursuing_context(
    entity: &Entity,
) -> Result<u32, Type13Class7PursuingGraphError> {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Type13Class7PursuingGraphError::UnpublishedPursuingGraph);
    };
    let Some(program) = behavior_program(u32::from(SEARCH_ATTACK_BEHAVIOR_CLASS_ID)) else {
        return Err(Type13Class7PursuingGraphError::UnpublishedPursuingGraph);
    };
    if context.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || context.choice_list_source()
            != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        || context.style_table_index_raw_at_0x10() != 1
        || context.active_style().style_address() != SearchAttackVariant::Pursuing.style_address()
    {
        return Err(Type13Class7PursuingGraphError::UnpublishedPursuingGraph);
    }
    match context.target_handle_at_0x08() {
        RetailRuntimeValue::Known(Some(target_id)) => Ok(target_id),
        RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => {
            Err(Type13Class7PursuingGraphError::UnpublishedPursuingGraph)
        }
    }
}

fn authenticate_published_type13_class7_pursuing_tasks(
    entity: &Entity,
    target_id: u32,
) -> Result<(), Type13Class7PursuingGraphError> {
    let Some(ActorTaskRuntime::ChaseTarget(chase)) =
        entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        return Err(Type13Class7PursuingGraphError::UnpublishedPursuingGraph);
    };
    let Some(ActorTaskRuntime::AimAndFire(aim)) = entity.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        return Err(Type13Class7PursuingGraphError::UnpublishedPursuingGraph);
    };
    let aim_private = aim.private_state();
    if entity.actor_task_state(ActorTaskSlot::Secondary).is_some()
        || chase.target_id() != target_id
        || aim_private.target_entity_id() != target_id
        || aim_private.direction() != 1
        || aim_private.reversal_timer_ms() != 0
        || aim_private.optional_sound_id_raw() != 0
        || aim_private.sound_period_us_raw() != 0
    {
        return Err(Type13Class7PursuingGraphError::UnpublishedPursuingGraph);
    }
    Ok(())
}

fn authenticate_published_type13_class7_b6c0_tasks(
    entity: &Entity,
) -> Result<(), Type13Class7LiveAcquisitionError> {
    let Some(ActorTaskRuntime::TargetAcquisition(acquire)) =
        entity.actor_task_state(ActorTaskSlot::Secondary)
    else {
        return Err(Type13Class7LiveAcquisitionError::UnpublishedB6c0Graph);
    };
    if acquire.radius()
        != SearchAttackRadius::from_raw(TYPE13_SEARCH_ATTACK_COMMON_AXIS.strict_axis_limit_raw)
        || acquire.filter()
            != SearchAttackCandidateFilter::from_raw(
                TYPE13_SEARCH_ATTACK_COMMON_AXIS.raw_word_at_0x04,
            )
        || acquire.constructor_filter_override_raw() != 0
    {
        return Err(Type13Class7LiveAcquisitionError::UnpublishedB6c0Graph);
    }
    let Some(ActorTaskRuntime::SharedRetarget(wander)) =
        entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        return Err(Type13Class7LiveAcquisitionError::UnpublishedB6c0Graph);
    };
    if wander.lifetime_ms() != TYPE13_CLASS7_ACQUIRING_WANDER_LIFETIME_MS
        || entity.actor_task_state(ActorTaskSlot::Tertiary).is_some()
    {
        return Err(Type13Class7LiveAcquisitionError::UnpublishedB6c0Graph);
    }
    Ok(())
}

fn authenticate_type13_class7_publication_surface(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type13Class7AcquiringError> {
    authenticate_type13_publication_identity(entity)?;
    match entity.current_behavior_context {
        RetailRuntimeValue::Unresolved | RetailRuntimeValue::Known(None) => {}
        RetailRuntimeValue::Known(Some(_)) => {
            return Err(Type13Class7AcquiringError::CurrentBehaviorContextAlreadyResolved);
        }
    }
    authenticate_type13_publication_storage(entity, metadata)
}

fn authenticate_type13_reselected_publication_surface(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<BehaviorContextRuntime, Type13Class7AcquiringError> {
    authenticate_type13_publication_identity(entity)?;
    let current_context = match entity.current_behavior_context {
        RetailRuntimeValue::Known(Some(context)) => context,
        RetailRuntimeValue::Known(None) => {
            return Err(Type13Class7AcquiringError::CurrentBehaviorContextAbsent);
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type13Class7AcquiringError::CurrentBehaviorContextUnresolved);
        }
    };
    if current_context.choice_list_source()
        != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
    {
        return Err(Type13Class7AcquiringError::CurrentBehaviorContextNotTypeDefault);
    }
    authenticate_type13_publication_storage(entity, metadata)?;
    Ok(current_context)
}

fn authenticate_type13_publication_identity(
    entity: &Entity,
) -> Result<(), Type13Class7AcquiringError> {
    if !entity.active {
        return Err(Type13Class7AcquiringError::EntityInactive);
    }
    if entity.entity_type != TYPE13_ENTITY_TYPE {
        return Err(Type13Class7AcquiringError::UnexpectedEntityType {
            actual: entity.entity_type,
        });
    }
    Ok(())
}

fn authenticate_type13_publication_storage(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type13Class7AcquiringError> {
    if !matches!(
        entity.actor_common_axis_descriptor,
        RetailRuntimeValue::Known(_)
    ) {
        return Err(Type13Class7AcquiringError::ActorCommonAxisDescriptorUnresolved);
    }
    authenticate_type13_metadata(metadata)?;
    match entity.sub_g_06070_runtime {
        RetailRuntimeValue::Unresolved => Err(Type13Class7AcquiringError::SubGRuntimeUnresolved),
        RetailRuntimeValue::Known(None) => Err(Type13Class7AcquiringError::SubGRuntimeAbsent),
        RetailRuntimeValue::Known(Some(_)) => Ok(()),
    }
}

fn copy_type_authored_common_axis_filter(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
) {
    let RetailRuntimeValue::Known(mut descriptor) = entity.actor_common_axis_descriptor else {
        unreachable!("type-13 class-7 preflight retained actor-local common-axis storage");
    };
    descriptor.raw_word_at_0x04 = metadata
        .initializer
        .as_ref()
        .expect("type-13 class-7 preflight retained the initializer")
        .common_axis_descriptor
        .raw_word_at_0x04;
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(descriptor);
}

/// Shared `FUN_00406070` Sub-G branch for type-13 B6C0.
///
/// Order is `FUN_0041B970(..., 0)`, `FUN_0041B940(..., 0)` (one word),
/// `FUN_0041B980(..., 0)`, then `FUN_00424380(..., 0)`. Sub-A, Sub-H, Sub-F,
/// and the class-12 `FUN_00404120` tail are not present on this topology.
/// The caller applies this receipt to the entity-owned Sub-G allocation.
fn apply_type13_b6c0_06070_sub_g_suffix(
    next_random: &mut impl FnMut() -> u32,
) -> Type13B6c0SubGConstructorEvidence {
    let random_sample_low16 = next_random() as u16;
    let target_raw_at_0x38 =
        i32::from(TYPE13_SEARCH_ATTACK_SUB_G_RANDOMIZED_TARGET_BASE_RAW_AT_0X0C)
            .wrapping_add(i32::from(random_sample_low16 >> 8));
    Type13B6c0SubGConstructorEvidence {
        random_sample_low16,
        target_raw_at_0x38,
        source_raw_at_0x00: TYPE13_SEARCH_ATTACK_SUB_G_SOURCE_RAW_AT_0X00,
    }
}

fn prepare_type13_class7_acquiring_task(
    preparation: SearchAttackTaskPreparation,
    owner_position_raw: [i16; 3],
    radius: SearchAttackRadius,
    type_authored_filter: SearchAttackCandidateFilter,
    constructor_filter_override_raw: u32,
) -> Result<PreparedActorTask<ActorTaskRuntime>, Type13Class7AcquiringError> {
    match preparation.task.role {
        SearchAttackTaskRole::AcquireTarget => prepare_target_acquisition_runtime_task(
            preparation,
            radius,
            type_authored_filter,
            constructor_filter_override_raw,
        )
        .map_err(|_| Type13Class7AcquiringError::TaskContractMismatch),
        SearchAttackTaskRole::Wander => {
            let setup = search_attack_variant_setup(SearchAttackVariant::Acquiring);
            if preparation.request != SearchAttackTaskSetupRequest::Acquiring
                || preparation.phase_index != 1
                || preparation.task != setup.ordered_phases[1].install
                || preparation.task.lifetime
                    != SearchAttackTaskLifetime::FixedMilliseconds(
                        TYPE13_CLASS7_ACQUIRING_WANDER_LIFETIME_MS,
                    )
            {
                return Err(Type13Class7AcquiringError::TaskContractMismatch);
            }
            Ok(PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                SharedRetargetTaskState::new(
                    owner_position_raw,
                    TYPE13_CLASS7_ACQUIRING_WANDER_LIFETIME_MS,
                ),
            )))
        }
        SearchAttackTaskRole::AimAndFire
        | SearchAttackTaskRole::PursueTarget
        | SearchAttackTaskRole::ExternalEvent => {
            Err(Type13Class7AcquiringError::TaskContractMismatch)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aim_and_fire::AimAndFireTaskState;
    use crate::entity::{EntityKind, EntityManager};
    use crate::entity_behavior::{
        apply_initial_behavior_state, audited_behavior_style, BehaviorChoiceListSource,
        BehaviorDescriptorIdentity,
    };
    use crate::entity_collision_state::{
        EntityInitializerSpec, RetailRuntimeValue, RetailStateWord,
    };
    use crate::search_attack::{
        SearchAttackCandidateFilter, SearchAttackRadius, SearchAttackTarget,
    };
    use crate::search_attack_acquisition::{
        TargetAcquisitionCallbackPrefix, TargetAcquisitionCallbackResult,
        TargetAcquisitionTaggedSingleton, TargetAcquisitionTaskState,
    };
    use crate::search_attack_live::{
        SearchAttackLiveAcquisitionOutcome, TYPE13_SEARCH_ATTACK_COMMON_AXIS,
        TYPE13_SEARCH_ATTACK_GKL, TYPE13_SEARCH_ATTACK_PROJECTILE_DESCRIPTOR,
        TYPE13_SEARCH_ATTACK_SUB_D, TYPE13_SEARCH_ATTACK_SUB_G_1B8C0_TABLE_BYTES,
    };
    use crate::world_fx::WorldFx;
    use v2k_formats::collision::CommonAxisDescriptor;

    fn type13_metadata() -> EntityTypeRuntimeMetadata {
        let mut metadata = EntityTypeRuntimeMetadata::default();
        metadata.common_mover_topology = RetailRuntimeValue::Known(TYPE13_SEARCH_ATTACK_TOPOLOGY);
        metadata.common_mover_gkl_payloads = RetailRuntimeValue::Known(TYPE13_SEARCH_ATTACK_GKL);
        metadata.initializer = Some(EntityInitializerSpec {
            initializer_state_flags_raw: 0x8,
            common_axis_descriptor: TYPE13_SEARCH_ATTACK_COMMON_AXIS,
            behavior_choices: Box::from(TYPE13_SEARCH_ATTACK_BEHAVIOR_CHOICES),
            behavior_rule_ref: TYPE13_SEARCH_ATTACK_BEHAVIOR_RULE_REF,
            alternate_behavior_class_ref: TYPE13_SEARCH_ATTACK_ALTERNATE_BEHAVIOR_CLASS,
        });
        metadata.search_attack_optional_prelude_sound_id = RetailRuntimeValue::Known(None);
        metadata.search_attack_aim_sound_id = RetailRuntimeValue::Known(None);
        metadata.search_attack_aim_sound_period_raw = RetailRuntimeValue::Known(0);
        metadata.projectile_emitter_descriptor =
            RetailRuntimeValue::Known(Some(TYPE13_SEARCH_ATTACK_PROJECTILE_DESCRIPTOR));
        metadata.sub_d_steering_descriptor =
            RetailRuntimeValue::Known(Some(TYPE13_SEARCH_ATTACK_SUB_D));
        metadata
    }

    fn type13_entity() -> Entity {
        construct_type13_from_metadata(0x0497_0001, &type13_metadata(), [0; 3], 0x1100).unwrap()
    }

    fn type13_context_with_words(
        selection: BehaviorSelection,
        choice_list_source: RetailRuntimeValue<BehaviorChoiceListSource>,
        target_handle_at_0x08: RetailRuntimeValue<Option<u32>>,
        auxiliary_word_at_0x0c: RetailRuntimeValue<u32>,
    ) -> BehaviorContextRuntime {
        BehaviorContextRuntime::named_audited(
            selection.program,
            selection.program.initial_style_table_index_raw,
            choice_list_source,
            target_handle_at_0x08,
            auxiliary_word_at_0x0c,
            selection.program.initial_style,
        )
        .expect("canonical type-13 context")
    }

    #[test]
    fn metadata_backed_construction_applies_1b8c0() {
        let entity =
            construct_type13_from_metadata(0x0497_0001, &type13_metadata(), [8, 0, 4], 0x1100)
                .unwrap();
        assert_eq!(entity.entity_type, TYPE13_ENTITY_TYPE);
        assert_eq!(entity.position_raw(), [8, 0, 4]);
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(TYPE13_SEARCH_ATTACK_COMMON_AXIS)
        );
        let RetailRuntimeValue::Known(Some(sub_g)) = entity.sub_g_06070_runtime else {
            panic!("metadata-backed construction must apply FUN_0041B8C0");
        };
        assert_eq!(
            sub_g.table_bytes_at_0x28(),
            RetailRuntimeValue::Known(TYPE13_SEARCH_ATTACK_SUB_G_1B8C0_TABLE_BYTES)
        );
        assert_eq!(
            sub_g.randomized_target_raw_at_0x38(),
            RetailRuntimeValue::Known(200 + 0x11)
        );
        assert_eq!(sub_g.word_at_0x1c(), RetailRuntimeValue::Known(0x5000));
        assert_eq!(sub_g.angle_raw_at_0x1e(), RetailRuntimeValue::Known(0));
        assert_eq!(sub_g.byte_at_0x41(), RetailRuntimeValue::Known(1));
        assert_eq!(
            entity.current_behavior_context,
            RetailRuntimeValue::Unresolved
        );
    }

    #[test]
    fn metadata_backed_construction_rejects_unexpected_sub_g() {
        let mut metadata = type13_metadata();
        metadata.common_mover_gkl_payloads =
            RetailRuntimeValue::Known(crate::entity_collision_state::CommonMoverGklPayloads {
                sub_g: Some([0; 104]),
                sub_k: None,
                sub_l: None,
            });
        assert!(matches!(
            construct_type13_from_metadata(1, &metadata, [0; 3], 0),
            Err(Type13Class7AcquiringError::UnexpectedSubGPayload)
        ));
    }

    #[test]
    fn metadata_birth_sequences_1b8c0_then_class5_acd0_without_world_fx() {
        let metadata = type13_metadata();
        let mut draws = 0;
        let birth = birth_type13_from_metadata(0x0497_0001, &metadata, [8, 0, 4], || {
            let word = match draws {
                0 => 0x1100,
                1 => 0x3FFF,
                2 => 0x2200,
                _ => panic!("class-5 birth consumes constructor, selector, and one suffix word"),
            };
            draws += 1;
            word
        })
        .unwrap();
        assert_eq!(draws, 3);
        assert_eq!(birth.constructor_sample_low16, 0x1100);
        let Type13InitialBehaviorPublication::MoveAboutAimlessly(published) = birth.publication
        else {
            panic!("0x3FFF must publish FUN_0040ACD0");
        };
        assert_eq!(published.style_address, 0x004C_7930);
        assert_eq!(published.planned.random_sample_low16, 0x3FFF);
        assert_eq!(
            published.constructor,
            Type13B6c0SubGConstructorEvidence {
                random_sample_low16: 0x2200,
                target_raw_at_0x38: 200 + 0x22,
                source_raw_at_0x00: 0,
            }
        );

        let entity = birth.entity;
        assert_eq!(entity.entity_type, TYPE13_ENTITY_TYPE);
        assert_eq!(entity.position_raw(), [8, 0, 4]);
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(TYPE13_SEARCH_ATTACK_COMMON_AXIS)
        );
        let Some(ActorTaskRuntime::SharedRetarget(wander)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("ACD0 must install slot-0 SharedRetarget");
        };
        assert_eq!(
            wander.lifetime_ms(),
            TYPE13_CLASS5_AIMLESS_WANDER_LIFETIME_MS
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        let RetailRuntimeValue::Known(Some(sub_g)) = entity.sub_g_06070_runtime else {
            panic!("class-5 birth must keep live Sub-G storage");
        };
        assert_eq!(
            sub_g.randomized_target_raw_at_0x38(),
            RetailRuntimeValue::Known(200 + 0x22)
        );
        assert_eq!(
            sub_g.table_bytes_at_0x28(),
            RetailRuntimeValue::Known(TYPE13_SEARCH_ATTACK_SUB_G_1B8C0_TABLE_BYTES)
        );
        assert_eq!(sub_g.word_at_0x1c(), RetailRuntimeValue::Known(0x5000));
        assert_eq!(sub_g.angle_raw_at_0x1e(), RetailRuntimeValue::Known(0));
        assert_eq!(sub_g.byte_at_0x41(), RetailRuntimeValue::Known(1));
    }

    #[test]
    fn metadata_birth_sequences_1b8c0_then_class7_b6c0_without_world_fx() {
        let metadata = type13_metadata();
        let mut draws = 0;
        let birth = birth_type13_from_metadata(0x0497_0001, &metadata, [8, 0, 4], || {
            let word = match draws {
                0 => 0x1100,
                1 => 0x4000,
                2 => 0x2200,
                3 => 0x3300,
                _ => panic!("class-7 birth consumes constructor, selector, and two suffix words"),
            };
            draws += 1;
            word
        })
        .unwrap();
        assert_eq!(draws, 4);
        assert_eq!(birth.constructor_sample_low16, 0x1100);
        let Type13InitialBehaviorPublication::SearchAndAttack(published) = birth.publication else {
            panic!("0x4000 must publish FUN_0040B6C0");
        };
        assert_eq!(published.style_address, 0x004C_7A50);
        assert_eq!(published.planned.random_sample_low16, 0x4000);
        assert_eq!(
            published.constructors_by_phase,
            [
                Type13B6c0SubGConstructorEvidence {
                    random_sample_low16: 0x2200,
                    target_raw_at_0x38: 200 + 0x22,
                    source_raw_at_0x00: 0,
                },
                Type13B6c0SubGConstructorEvidence {
                    random_sample_low16: 0x3300,
                    target_raw_at_0x38: 200 + 0x33,
                    source_raw_at_0x00: 0,
                },
            ]
        );

        let entity = birth.entity;
        let Some(ActorTaskRuntime::TargetAcquisition(_)) =
            entity.actor_task_state(ActorTaskSlot::Secondary)
        else {
            panic!("B6C0 must install slot-1 acquisition");
        };
        let Some(ActorTaskRuntime::SharedRetarget(wander)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("B6C0 must install slot-0 SharedRetarget");
        };
        assert_eq!(
            wander.lifetime_ms(),
            TYPE13_CLASS7_ACQUIRING_WANDER_LIFETIME_MS
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        let RetailRuntimeValue::Known(Some(sub_g)) = entity.sub_g_06070_runtime else {
            panic!("class-7 birth must keep live Sub-G storage");
        };
        assert_eq!(
            sub_g.randomized_target_raw_at_0x38(),
            RetailRuntimeValue::Known(200 + 0x33)
        );
        assert_eq!(
            sub_g.table_bytes_at_0x28(),
            RetailRuntimeValue::Known(TYPE13_SEARCH_ATTACK_SUB_G_1B8C0_TABLE_BYTES)
        );
        assert_eq!(sub_g.word_at_0x1c(), RetailRuntimeValue::Known(0x5000));
        assert_eq!(sub_g.angle_raw_at_0x1e(), RetailRuntimeValue::Known(0));
        assert_eq!(sub_g.byte_at_0x41(), RetailRuntimeValue::Known(1));
    }

    #[test]
    fn metadata_birth_rejects_unexpected_payload_without_rng() {
        let mut metadata = type13_metadata();
        metadata.common_mover_gkl_payloads =
            RetailRuntimeValue::Known(crate::entity_collision_state::CommonMoverGklPayloads {
                sub_g: Some([0; 104]),
                sub_k: None,
                sub_l: None,
            });
        let mut draws = 0;
        assert!(matches!(
            birth_type13_from_metadata(1, &metadata, [0; 3], || {
                draws += 1;
                0
            }),
            Err(Type13MetadataBirthError::Construction(
                Type13Class7AcquiringError::UnexpectedSubGPayload
            ))
        ));
        assert_eq!(draws, 0);

        metadata = type13_metadata();
        metadata.initializer.as_mut().unwrap().behavior_choices = Box::new([]);
        assert!(matches!(
            birth_type13_from_metadata(1, &metadata, [0; 3], || {
                draws += 1;
                0
            }),
            Err(Type13MetadataBirthError::Plan(
                Type13InitialBehaviorError::UnexpectedBehaviorChoices
            ))
        ));
        assert_eq!(draws, 0);
    }

    #[test]
    fn missing_initializer_fails_closed() {
        assert_eq!(
            plan_type13_initial_behavior(&EntityTypeRuntimeMetadata::default(), || 0),
            Err(Type13InitialBehaviorError::MissingInitializer)
        );
    }

    #[test]
    fn unexpected_choices_fail_closed_without_rng() {
        let mut metadata = type13_metadata();
        metadata.initializer.as_mut().unwrap().behavior_choices = Box::new([]);
        let mut draws = 0;
        assert_eq!(
            plan_type13_initial_behavior(&metadata, || {
                draws += 1;
                0
            }),
            Err(Type13InitialBehaviorError::UnexpectedBehaviorChoices)
        );
        assert_eq!(draws, 0);
    }

    #[test]
    fn low_word_below_0x4000_selects_move_about_aimlessly() {
        let planned = plan_type13_initial_behavior(&type13_metadata(), || 0x3FFF).unwrap();
        assert_eq!(planned.random_sample_low16, 0x3FFF);
        assert_eq!(planned.selection.choice_index, 0);
        assert_eq!(
            planned.selection.program.class_id,
            TYPE13_MOVE_ABOUT_AIMLESSLY_CLASS_ID
        );
        assert_eq!(planned.selection.program.name, "Move About Aimlessly");
    }

    #[test]
    fn low_word_at_0x4000_selects_search_and_attack() {
        let planned = plan_type13_initial_behavior(&type13_metadata(), || 0x4000).unwrap();
        assert_eq!(planned.random_sample_low16, 0x4000);
        assert_eq!(planned.selection.choice_index, 1);
        assert_eq!(
            planned.selection.program.class_id,
            SEARCH_ATTACK_BEHAVIOR_CLASS_ID
        );
        assert_eq!(planned.selection.program.name, "Search And Attack Target");
    }

    #[test]
    fn consumes_exactly_one_selector_word() {
        let mut draws = 0;
        let planned = plan_type13_initial_behavior(&type13_metadata(), || {
            draws += 1;
            0xFFFF
        })
        .unwrap();
        assert_eq!(draws, 1);
        assert_eq!(
            planned.selection.program.class_id,
            SEARCH_ATTACK_BEHAVIOR_CLASS_ID
        );
    }

    #[test]
    fn class5_plan_does_not_publish_move_about_aimlessly() {
        let planned = plan_type13_initial_behavior(&type13_metadata(), || 0x3FFF).unwrap();
        let mut entity = type13_entity();
        assert_eq!(
            publish_type13_class7_acquiring(&mut entity, &type13_metadata(), planned, || {
                panic!("class 5 must not consume a B6C0 suffix word")
            }),
            Err(Type13Class7AcquiringError::MoveAboutAimlesslyUninvented)
        );
        assert_eq!(
            entity.current_behavior_context,
            RetailRuntimeValue::Unresolved
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
    }

    #[test]
    fn class5_plan_publishes_acd0_5000ms_shared_retarget_without_01430() {
        let metadata = type13_metadata();
        let planned = plan_type13_initial_behavior(&metadata, || 0x3FFF).unwrap();
        let mut entity = type13_entity();
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(CommonAxisDescriptor {
            strict_axis_limit_raw: TYPE13_SEARCH_ATTACK_COMMON_AXIS.strict_axis_limit_raw,
            raw_word_at_0x04: 0xDEAD,
        });
        entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                SharedRetargetTaskState::new([1, 2, 3], 9),
            )),
        );
        entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                SharedRetargetTaskState::new([4, 5, 6], 10),
            )),
        );

        let mut suffix_draws = 0;
        let published = publish_type13_class5_aimlessly(&mut entity, &metadata, planned, || {
            suffix_draws += 1;
            0x1100
        })
        .unwrap();
        assert_eq!(suffix_draws, 1);
        assert_eq!(published.style_address, 0x004C_7930);
        assert_eq!(
            published.constructor,
            Type13B6c0SubGConstructorEvidence {
                random_sample_low16: 0x1100,
                target_raw_at_0x38: 200 + 0x11,
                source_raw_at_0x00: 0,
            }
        );

        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("class-5 ACD0 must publish style 0x004C7930");
        };
        assert_eq!(context.style_table_index_raw_at_0x10(), 0);
        assert_eq!(
            context.active_style().style_address(),
            TYPE13_MOVE_ABOUT_AIMLESSLY_STYLE_ADDRESS
        );
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(CommonAxisDescriptor {
                strict_axis_limit_raw: TYPE13_SEARCH_ATTACK_COMMON_AXIS.strict_axis_limit_raw,
                raw_word_at_0x04: 0xDEAD,
            }),
            "FUN_0040ACD0 does not copy type +0x04"
        );

        let Some(ActorTaskRuntime::SharedRetarget(wander)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("ACD0 must install slot-0 SharedRetarget");
        };
        assert_eq!(
            wander.lifetime_ms(),
            TYPE13_CLASS5_AIMLESS_WANDER_LIFETIME_MS
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        let RetailRuntimeValue::Known(Some(sub_g)) = entity.sub_g_06070_runtime else {
            panic!("ACD0 02B10 must write live 06070 Sub-G storage");
        };
        assert_eq!(
            sub_g.randomized_target_raw_at_0x38(),
            RetailRuntimeValue::Known(200 + 0x11)
        );
        assert_eq!(
            sub_g.table_bytes_at_0x28(),
            RetailRuntimeValue::Known(TYPE13_SEARCH_ATTACK_SUB_G_1B8C0_TABLE_BYTES)
        );
        assert_eq!(sub_g.word_at_0x1c(), RetailRuntimeValue::Known(0x5000));
        assert_eq!(sub_g.angle_raw_at_0x1e(), RetailRuntimeValue::Known(0));
        assert_eq!(sub_g.byte_at_0x41(), RetailRuntimeValue::Known(1));
        assert!(!matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::TargetAcquisition(_))
        ));
    }

    #[test]
    fn dispatcher_selects_class5_or_class7_from_the_weighted_plan() {
        let metadata = type13_metadata();
        let class5 = plan_type13_initial_behavior(&metadata, || 0x3FFF).unwrap();
        let mut entity = type13_entity();
        assert!(matches!(
            publish_type13_initial_behavior(&mut entity, &metadata, class5, || 0).unwrap(),
            Type13InitialBehaviorPublication::MoveAboutAimlessly(_)
        ));

        let class7 = plan_type13_initial_behavior(&metadata, || 0x4000).unwrap();
        let mut entity = type13_entity();
        assert!(matches!(
            publish_type13_initial_behavior(&mut entity, &metadata, class7, || 0).unwrap(),
            Type13InitialBehaviorPublication::SearchAndAttack(_)
        ));
    }

    #[test]
    fn class5_reselection_preserves_context_birth_provenance_and_rng_cadence() {
        const RETAINED_TARGET: u32 = 0x047F_0001;
        const RETAINED_AUXILIARY: u32 = 0xCAFE_BABE;

        let metadata = type13_metadata();
        let predecessor = plan_type13_initial_behavior(&metadata, || 0x4000).unwrap();
        let planned = plan_type13_initial_behavior(&metadata, || 0x3FFF).unwrap();
        let mut entity = type13_entity();
        entity.initial_behavior = RetailRuntimeValue::Known(Some(predecessor.selection));
        entity.current_behavior_context =
            RetailRuntimeValue::Known(Some(type13_context_with_words(
                predecessor.selection,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(Some(RETAINED_TARGET)),
                RetailRuntimeValue::Known(RETAINED_AUXILIARY),
            )));
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(CommonAxisDescriptor {
            strict_axis_limit_raw: TYPE13_SEARCH_ATTACK_COMMON_AXIS.strict_axis_limit_raw,
            raw_word_at_0x04: 0xDEAD,
        });
        for (slot, lifetime_ms) in [
            (ActorTaskSlot::Primary, 101),
            (ActorTaskSlot::Secondary, 202),
            (ActorTaskSlot::Tertiary, 303),
        ] {
            entity.actor_tasks.replace_prepared(
                slot,
                PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                    SharedRetargetTaskState::new([1, 2, 3], lifetime_ms),
                )),
            );
        }
        let initial_behavior = entity.initial_behavior;
        let state_before = entity.collision.state_flags_at_0x08.known_value_bits();

        let mut suffix_draws = 0;
        let publication =
            publish_type13_reselected_initial_behavior(&mut entity, &metadata, planned, || {
                suffix_draws += 1;
                0x12FF
            })
            .unwrap();
        let Type13InitialBehaviorPublication::MoveAboutAimlessly(published) = publication else {
            panic!("class-5 reselection must publish ACD0");
        };
        assert_eq!(suffix_draws, 1);
        assert_eq!(
            published.constructor,
            Type13B6c0SubGConstructorEvidence {
                random_sample_low16: 0x12FF,
                target_raw_at_0x38: 200 + 0x12,
                source_raw_at_0x00: 0,
            }
        );
        assert_eq!(entity.initial_behavior, initial_behavior);
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("class-5 reselection must retain the allocated context");
        };
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::Named(planned.selection.program)
        );
        assert_eq!(
            context.choice_list_source(),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        );
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(RETAINED_TARGET))
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(RETAINED_AUXILIARY)
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.known_value_bits(),
            apply_initial_behavior_state(state_before, planned.selection.program)
        );
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(CommonAxisDescriptor {
                strict_axis_limit_raw: TYPE13_SEARCH_ATTACK_COMMON_AXIS.strict_axis_limit_raw,
                raw_word_at_0x04: 0xDEAD,
            }),
            "FUN_0040ACD0 reselection does not copy type +0x04"
        );
        let Some(ActorTaskRuntime::SharedRetarget(primary)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("class-5 reselection must replace Primary");
        };
        assert_eq!(
            primary.lifetime_ms(),
            TYPE13_CLASS5_AIMLESS_WANDER_LIFETIME_MS
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        let RetailRuntimeValue::Known(Some(sub_g)) = entity.sub_g_06070_runtime else {
            panic!("class-5 reselection must retain Sub-G storage");
        };
        assert_eq!(
            sub_g.randomized_target_raw_at_0x38(),
            RetailRuntimeValue::Known(200 + 0x12)
        );
    }

    #[test]
    fn class7_reselection_replaces_live_tasks_and_consumes_two_suffix_words() {
        const RETAINED_TARGET: u32 = 0x047F_0002;
        const RETAINED_AUXILIARY: u32 = 0x1357_2468;

        let metadata = type13_metadata();
        let predecessor = plan_type13_initial_behavior(&metadata, || 0x3FFF).unwrap();
        let planned = plan_type13_initial_behavior(&metadata, || 0x4000).unwrap();
        let mut entity = type13_entity();
        entity.initial_behavior = RetailRuntimeValue::Known(Some(predecessor.selection));
        entity.current_behavior_context =
            RetailRuntimeValue::Known(Some(type13_context_with_words(
                predecessor.selection,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(Some(RETAINED_TARGET)),
                RetailRuntimeValue::Known(RETAINED_AUXILIARY),
            )));
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(CommonAxisDescriptor {
            strict_axis_limit_raw: TYPE13_SEARCH_ATTACK_COMMON_AXIS.strict_axis_limit_raw,
            raw_word_at_0x04: 0xDEAD,
        });
        for (slot, lifetime_ms) in [
            (ActorTaskSlot::Primary, 101),
            (ActorTaskSlot::Secondary, 202),
            (ActorTaskSlot::Tertiary, 303),
        ] {
            entity.actor_tasks.replace_prepared(
                slot,
                PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                    SharedRetargetTaskState::new([4, 5, 6], lifetime_ms),
                )),
            );
        }
        let old_primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let old_secondary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Secondary)
            .unwrap();
        let initial_behavior = entity.initial_behavior;
        let state_before = entity.collision.state_flags_at_0x08.known_value_bits();

        let suffix_words = [0x1100, 0x22FF];
        let mut suffix_draws = 0;
        let publication =
            publish_type13_reselected_initial_behavior(&mut entity, &metadata, planned, || {
                let word = suffix_words[suffix_draws];
                suffix_draws += 1;
                word
            })
            .unwrap();
        let Type13InitialBehaviorPublication::SearchAndAttack(published) = publication else {
            panic!("class-7 reselection must publish B6C0");
        };
        assert_eq!(suffix_draws, 2);
        assert_eq!(
            published.constructors_by_phase,
            [
                Type13B6c0SubGConstructorEvidence {
                    random_sample_low16: 0x1100,
                    target_raw_at_0x38: 200 + 0x11,
                    source_raw_at_0x00: 0,
                },
                Type13B6c0SubGConstructorEvidence {
                    random_sample_low16: 0x22FF,
                    target_raw_at_0x38: 200 + 0x22,
                    source_raw_at_0x00: 0,
                },
            ]
        );
        assert_ne!(published.primary_task_id, old_primary);
        assert_ne!(published.secondary_task_id, old_secondary);
        assert_eq!(entity.initial_behavior, initial_behavior);
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("class-7 reselection must retain the allocated context");
        };
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::Named(planned.selection.program)
        );
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(RETAINED_TARGET))
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(RETAINED_AUXILIARY)
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.known_value_bits(),
            apply_initial_behavior_state(state_before, planned.selection.program)
        );
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(TYPE13_SEARCH_ATTACK_COMMON_AXIS)
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::TargetAcquisition(_))
        ));
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(state))
                if state.lifetime_ms() == TYPE13_CLASS7_ACQUIRING_WANDER_LIFETIME_MS
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        let RetailRuntimeValue::Known(Some(sub_g)) = entity.sub_g_06070_runtime else {
            panic!("class-7 reselection must retain Sub-G storage");
        };
        assert_eq!(
            sub_g.randomized_target_raw_at_0x38(),
            RetailRuntimeValue::Known(200 + 0x22)
        );
    }

    #[test]
    fn reselection_context_preflight_is_precise_and_mutation_free() {
        let metadata = type13_metadata();
        let predecessor = plan_type13_initial_behavior(&metadata, || 0x4000).unwrap();
        let planned = plan_type13_initial_behavior(&metadata, || 0x3FFF).unwrap();

        let mut entity = type13_entity();
        assert_eq!(
            publish_type13_reselected_initial_behavior(&mut entity, &metadata, planned, || panic!(
                "unresolved context must block before suffix RNG"
            ),),
            Err(Type13Class7AcquiringError::CurrentBehaviorContextUnresolved)
        );
        entity.current_behavior_context = RetailRuntimeValue::Known(None);
        assert_eq!(
            publish_type13_reselected_initial_behavior(&mut entity, &metadata, planned, || panic!(
                "absent context must block before suffix RNG"
            ),),
            Err(Type13Class7AcquiringError::CurrentBehaviorContextAbsent)
        );

        entity.initial_behavior = RetailRuntimeValue::Known(Some(predecessor.selection));
        entity.current_behavior_context =
            RetailRuntimeValue::Known(Some(type13_context_with_words(
                predecessor.selection,
                RetailRuntimeValue::Unresolved,
                RetailRuntimeValue::Known(Some(0x047F_0003)),
                RetailRuntimeValue::Known(0xABCD_EF01),
            )));
        for (slot, lifetime_ms) in [
            (ActorTaskSlot::Primary, 101),
            (ActorTaskSlot::Secondary, 202),
            (ActorTaskSlot::Tertiary, 303),
        ] {
            entity.actor_tasks.replace_prepared(
                slot,
                PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                    SharedRetargetTaskState::new([7, 8, 9], lifetime_ms),
                )),
            );
        }
        let context_before = entity.current_behavior_context;
        let initial_behavior_before = entity.initial_behavior;
        let state_before = entity.collision.state_flags_at_0x08;
        let axis_before = entity.actor_common_axis_descriptor;
        let sub_g_before = entity.sub_g_06070_runtime;
        let task_ids_before = [
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Secondary),
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary),
        ];
        let task_states_before = [
            entity.actor_task_state(ActorTaskSlot::Primary).copied(),
            entity.actor_task_state(ActorTaskSlot::Secondary).copied(),
            entity.actor_task_state(ActorTaskSlot::Tertiary).copied(),
        ];
        let mut suffix_draws = 0;

        assert_eq!(
            publish_type13_reselected_initial_behavior(&mut entity, &metadata, planned, || {
                suffix_draws += 1;
                0
            },),
            Err(Type13Class7AcquiringError::CurrentBehaviorContextNotTypeDefault)
        );
        assert_eq!(suffix_draws, 0);
        assert_eq!(entity.current_behavior_context, context_before);
        assert_eq!(entity.initial_behavior, initial_behavior_before);
        assert_eq!(entity.collision.state_flags_at_0x08, state_before);
        assert_eq!(entity.actor_common_axis_descriptor, axis_before);
        assert_eq!(entity.sub_g_06070_runtime, sub_g_before);
        assert_eq!(
            [
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Secondary),
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary),
            ],
            task_ids_before
        );
        assert_eq!(
            [
                entity.actor_task_state(ActorTaskSlot::Primary).copied(),
                entity.actor_task_state(ActorTaskSlot::Secondary).copied(),
                entity.actor_task_state(ActorTaskSlot::Tertiary).copied(),
            ],
            task_states_before
        );
    }

    #[test]
    fn class7_plan_publishes_b6c0_acquire_and_500ms_wander_without_ade0() {
        let metadata = type13_metadata();
        let planned = plan_type13_initial_behavior(&metadata, || 0x4000).unwrap();
        let mut entity = type13_entity();
        entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                SharedRetargetTaskState::new([1, 2, 3], 9),
            )),
        );

        let mut suffix_draws = 0;
        let published = publish_type13_class7_acquiring(&mut entity, &metadata, planned, || {
            let word = match suffix_draws {
                0 => 0x1100,
                1 => 0x22FF,
                _ => panic!("B6C0 Sub-G suffix consumes exactly two words"),
            };
            suffix_draws += 1;
            word
        })
        .unwrap();
        assert_eq!(suffix_draws, 2);
        assert_eq!(published.entity_id, entity.id);
        assert_eq!(published.style_address, 0x004C_7A50);
        assert_eq!(published.planned.random_sample_low16, 0x4000);
        assert_eq!(
            published.constructors_by_phase,
            [
                Type13B6c0SubGConstructorEvidence {
                    random_sample_low16: 0x1100,
                    target_raw_at_0x38: 200 + 0x11,
                    source_raw_at_0x00: 0,
                },
                Type13B6c0SubGConstructorEvidence {
                    random_sample_low16: 0x22FF,
                    target_raw_at_0x38: 200 + 0x22,
                    source_raw_at_0x00: 0,
                },
            ]
        );
        let RetailRuntimeValue::Known(Some(sub_g)) = entity.sub_g_06070_runtime else {
            panic!("B6C0 Sub-G suffix must write live 06070 storage");
        };
        assert_eq!(sub_g.mode_at_0x3f(), RetailRuntimeValue::Known(0));
        assert_eq!(
            sub_g.randomized_target_raw_at_0x38(),
            RetailRuntimeValue::Known(200 + 0x22)
        );
        assert_eq!(sub_g.mode_at_0x40(), RetailRuntimeValue::Known(0));
        assert_eq!(
            sub_g.accumulator_raw_at_0x24(),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(sub_g.source_raw_at_0x20(), RetailRuntimeValue::Known(0));
        assert_eq!(sub_g.invert_target_at_0x3c(), RetailRuntimeValue::Known(0));
        assert_eq!(
            sub_g.table_bytes_at_0x28(),
            RetailRuntimeValue::Known(TYPE13_SEARCH_ATTACK_SUB_G_1B8C0_TABLE_BYTES)
        );
        assert_eq!(sub_g.word_at_0x1c(), RetailRuntimeValue::Known(0x5000));
        assert_eq!(sub_g.angle_raw_at_0x1e(), RetailRuntimeValue::Known(0));
        assert_eq!(sub_g.byte_at_0x41(), RetailRuntimeValue::Known(1));

        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("class-7 B6C0 must publish a fresh variant-0 context");
        };
        let program = behavior_program(u32::from(SEARCH_ATTACK_BEHAVIOR_CLASS_ID)).unwrap();
        let style = *audited_behavior_style(u32::from(SEARCH_ATTACK_BEHAVIOR_CLASS_ID), 0).unwrap();
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::Named(program)
        );
        assert_eq!(context.style_table_index_raw_at_0x10(), 0);
        assert_eq!(context.active_style().style_address(), 0x004C_7A50);
        assert_eq!(context.active_style().audited(), Some(style));
        assert_eq!(
            context.choice_list_source(),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        );
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(TYPE13_SEARCH_ATTACK_COMMON_AXIS)
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.known_value_bits(),
            apply_initial_behavior_state(1, planned.selection.program)
        );

        let Some(ActorTaskRuntime::TargetAcquisition(acquire)) =
            entity.actor_task_state(ActorTaskSlot::Secondary)
        else {
            panic!("B6C0 must install slot-1 acquisition");
        };
        assert_eq!(
            *acquire,
            TargetAcquisitionTaskState::new(
                SearchAttackRadius::from_raw(0x1900),
                SearchAttackCandidateFilter::from_raw(0x0C05),
                0,
            )
        );
        let Some(ActorTaskRuntime::SharedRetarget(wander)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("B6C0 must install slot-0 SharedRetarget");
        };
        assert_eq!(
            wander.lifetime_ms(),
            TYPE13_CLASS7_ACQUIRING_WANDER_LIFETIME_MS
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        assert!(!matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::ChaseTarget(_))
        ));
        assert!(!matches!(
            entity.actor_task_state(ActorTaskSlot::Tertiary),
            Some(ActorTaskRuntime::AimAndFire(_))
        ));
    }

    #[test]
    fn unresolved_axis_fails_closed_without_publication() {
        let metadata = type13_metadata();
        let planned = plan_type13_initial_behavior(&metadata, || 0x4000).unwrap();
        let mut entity = type13_entity();
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Unresolved;
        assert_eq!(
            publish_type13_class7_acquiring(&mut entity, &metadata, planned, || {
                panic!("unresolved axis must not consume a B6C0 suffix word")
            }),
            Err(Type13Class7AcquiringError::ActorCommonAxisDescriptorUnresolved)
        );
        assert_eq!(
            entity.current_behavior_context,
            RetailRuntimeValue::Unresolved
        );
    }

    #[test]
    fn unexpected_topology_fails_closed_without_inventing_suffix() {
        let mut metadata = type13_metadata();
        metadata.common_mover_topology = RetailRuntimeValue::Known(Default::default());
        let planned = plan_type13_initial_behavior(&metadata, || 0x4000).unwrap();
        let mut entity = type13_entity();
        assert_eq!(
            publish_type13_class7_acquiring(&mut entity, &metadata, planned, || {
                panic!("unexpected topology must not consume a B6C0 suffix word")
            }),
            Err(Type13Class7AcquiringError::UnexpectedComponentTopology)
        );
    }

    #[test]
    fn unresolved_sub_g_fails_closed_without_suffix_rng() {
        let mut metadata = type13_metadata();
        metadata.common_mover_gkl_payloads = RetailRuntimeValue::Unresolved;
        let planned = plan_type13_initial_behavior(&metadata, || 0x4000).unwrap();
        let mut entity = type13_entity();
        assert_eq!(
            publish_type13_class7_acquiring(&mut entity, &metadata, planned, || {
                panic!("unresolved Sub-G must not consume a B6C0 suffix word")
            }),
            Err(Type13Class7AcquiringError::SubGPayloadUnresolved)
        );
        assert_eq!(
            entity.current_behavior_context,
            RetailRuntimeValue::Unresolved
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_none());
    }

    #[test]
    fn published_b6c0_graph_drives_live_c7d0_ade0_without_type13_mover() {
        const NEAR_ID: u32 = 0x047F_0001;
        let metadata = type13_metadata();
        let planned = plan_type13_initial_behavior(&metadata, || 0x4000).unwrap();
        let mut owner = type13_entity();
        publish_type13_class7_acquiring(&mut owner, &metadata, planned, || 0).unwrap();

        let mut near = Entity::unresolved_port_entity(NEAR_ID, EntityKind::Enemy, 17);
        near.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        near.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        near.set_position_raw([8, 0, 0]);
        near.capability_flags = TYPE13_SEARCH_ATTACK_COMMON_AXIS.raw_word_at_0x04;

        let mut table = vec![EntityTypeRuntimeMetadata::default(); 18];
        table[13] = metadata;
        let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![owner, near],
            table,
            false,
        );
        let mut world_fx = WorldFx::new();
        let outcome = apply_type13_published_class7_acquisition_live(
            &mut manager,
            0x0497_0001,
            20_000,
            &mut world_fx,
        )
        .unwrap();
        assert_eq!(
            outcome,
            SearchAttackLiveAcquisitionOutcome::Applied {
                entity_id: 0x0497_0001,
                prefix: TargetAcquisitionCallbackPrefix {
                    radius: SearchAttackRadius::from_raw(0x1900),
                    filter: SearchAttackCandidateFilter::from_raw(0x0C05),
                },
                result: TargetAcquisitionCallbackResult::TaggedTargetAccepted {
                    target: SearchAttackTarget {
                        id: NEAR_ID,
                        scaled_distance_squared_raw: 16,
                    },
                    singleton: TargetAcquisitionTaggedSingleton::TargetAccepted,
                },
                same_pass_aim: None,
            }
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == 0x0497_0001)
            .unwrap();
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("C7D0 must publish class-7 variant 1");
        };
        assert_eq!(context.style_table_index_raw_at_0x10(), 1);
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(NEAR_ID))
        );
        let Some(ActorTaskRuntime::AimAndFire(aim)) =
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        else {
            panic!("ADE0 publishes Aim");
        };
        assert_eq!(
            aim.elapsed_ms(),
            0,
            "Type-13 acquisition skips without_emitter so Aim is not visited twice"
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::ChaseTarget(chase)) if chase.target_id() == NEAR_ID
                && chase.elapsed_ms() == 0
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
    }

    #[test]
    fn pursuing_authentication_joins_context_chase_and_authored_aim_private_state() {
        const TARGET_ID: u32 = 0x047F_0001;
        const FOREIGN_TARGET_ID: u32 = 0x047F_0002;
        let metadata = type13_metadata();
        let planned = plan_type13_initial_behavior(&metadata, || 0x4000).unwrap();
        let mut owner = type13_entity();
        publish_type13_class7_acquiring(&mut owner, &metadata, planned, || 0).unwrap();

        let mut target = Entity::unresolved_port_entity(TARGET_ID, EntityKind::Enemy, 17);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        target.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        target.set_position_raw([8, 0, 0]);
        target.capability_flags = TYPE13_SEARCH_ATTACK_COMMON_AXIS.raw_word_at_0x04;
        let mut table = vec![EntityTypeRuntimeMetadata::default(); 18];
        table[TYPE13_ENTITY_TYPE as usize] = metadata.clone();
        let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
            vec![owner, target],
            table,
            false,
        );
        apply_type13_published_class7_acquisition_live(
            &mut manager,
            0x0497_0001,
            20_000,
            &mut WorldFx::new(),
        )
        .unwrap();
        let owner = manager.entity_mut(0x0497_0001).unwrap();
        assert_eq!(
            authenticate_published_type13_class7_pursuing_graph(owner),
            Ok(())
        );
        let RetailRuntimeValue::Known(Some(context)) = owner.current_behavior_context else {
            panic!("ADE0 published the pursuing context");
        };
        let program = behavior_program(u32::from(SEARCH_ATTACK_BEHAVIOR_CLASS_ID)).unwrap();
        let style = context
            .active_style()
            .audited()
            .expect("pursuing style is audited");
        owner.current_behavior_context = RetailRuntimeValue::Known(Some(
            BehaviorContextRuntime::named_audited(
                program,
                1,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(Some(FOREIGN_TARGET_ID)),
                context.auxiliary_word_at_0x0c(),
                style,
            )
            .unwrap(),
        ));
        assert_eq!(
            authenticate_published_type13_class7_pursuing_graph(owner),
            Err(Type13Class7PursuingGraphError::UnpublishedPursuingGraph)
        );
        owner.current_behavior_context = RetailRuntimeValue::Known(Some(context));

        let foreign_target_aim = AimAndFireTaskState::prepare_after_allocation(
            owner.id,
            owner.position_raw(),
            FOREIGN_TARGET_ID,
            0,
            0,
            &metadata,
        )
        .unwrap()
        .map_task(ActorTaskRuntime::AimAndFire)
        .apply_suffix(|_, _| {});
        owner
            .actor_tasks
            .replace_prepared(ActorTaskSlot::Tertiary, foreign_target_aim);
        assert_eq!(
            authenticate_published_type13_class7_pursuing_graph(owner),
            Err(Type13Class7PursuingGraphError::UnpublishedPursuingGraph)
        );

        let foreign_audio_aim = AimAndFireTaskState::prepare_after_allocation(
            owner.id,
            owner.position_raw(),
            TARGET_ID,
            70,
            0x400,
            &metadata,
        )
        .unwrap()
        .map_task(ActorTaskRuntime::AimAndFire)
        .apply_suffix(|_, _| {});
        owner
            .actor_tasks
            .replace_prepared(ActorTaskSlot::Tertiary, foreign_audio_aim);
        assert_eq!(
            authenticate_published_type13_class7_pursuing_graph(owner),
            Err(Type13Class7PursuingGraphError::UnpublishedPursuingGraph)
        );
    }

    #[test]
    fn unpublished_graph_does_not_enter_the_type13_live_owner() {
        let mut owner = type13_entity();
        owner.actor_tasks.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(ActorTaskRuntime::TargetAcquisition(
                TargetAcquisitionTaskState::new(
                    SearchAttackRadius::from_raw(0x1900),
                    SearchAttackCandidateFilter::from_raw(0x0C05),
                    0,
                ),
            )),
        );
        let mut manager = EntityManager::from_entities_for_test(vec![owner]);
        assert_eq!(
            apply_type13_published_class7_acquisition_live(
                &mut manager,
                0x0497_0001,
                0,
                &mut WorldFx::new(),
            ),
            Err(Type13Class7LiveAcquisitionError::UnpublishedB6c0Graph)
        );
    }

    #[test]
    fn unresolved_sub_g_runtime_fails_closed_without_suffix_rng() {
        let metadata = type13_metadata();
        let planned = plan_type13_initial_behavior(&metadata, || 0x4000).unwrap();
        let mut entity = type13_entity();
        entity.sub_g_06070_runtime = RetailRuntimeValue::Unresolved;
        assert_eq!(
            publish_type13_class7_acquiring(&mut entity, &metadata, planned, || {
                panic!("unresolved Sub-G runtime must not consume a B6C0 suffix word")
            }),
            Err(Type13Class7AcquiringError::SubGRuntimeUnresolved)
        );
        assert_eq!(
            entity.current_behavior_context,
            RetailRuntimeValue::Unresolved
        );
    }
}
