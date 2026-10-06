//! Shared authenticated prefix for fresh-Level-1 Type-9 selected initializers.
//!
//! Retail selects and initializes an ordinary actor before linking it into the
//! live entity list and before `FUN_00413F70` finalizes its physical basis and
//! state bit 4. Branch adapters use this module to authenticate that common
//! pre-publication boundary once, then consume the returned token to publish
//! the chosen descriptor/context and move component custody. Task mutation,
//! allocation failure, outer initializer fallback, entity linking, and wrapper
//! finalization remain explicit at the branch/outer-owner seams.
//!
//! Native `FUN_0040ABE0` allocates the fresh behavior context before the style
//! switch enters any branch initializer. Rust stores that context inline, so
//! this shared token represents only the successful-allocation path. A failed
//! native context allocation is a terminal outer entity-construction abort,
//! not a retryable preflight rejection or the unnamed initializer fallback;
//! the production composer owns that earlier decision and provisional-entity
//! teardown before calling [`OrdinaryType9SelectedInitializerPreflight::publish`].

use crate::{
    actor_animation::ActorAnimationController,
    actor_task_owner::ActorTaskSlot,
    common_mover::SubAPropulsionRuntime,
    entity::Entity,
    entity_behavior::{initial_behavior_state_policy, BehaviorContextRuntime, BehaviorSelection},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    main_base_type9_abort::{
        exact_level_one_type9_metadata, LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
        LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR, LEVEL_ONE_TYPE9_ENTITY_TYPE,
        LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW, LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW,
        LEVEL_ONE_TYPE9_MASS_RAW, LEVEL_ONE_TYPE9_MODEL_ID, LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR,
        LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR,
    },
    ordinary_type9_initial_selection::{
        FreshLevel1Type9EntityRef, FreshLevel1Type9EvaluatorEvidence,
        FreshLevel1Type9WeightedSelection,
    },
    ordinary_type9_live::{
        fresh_level1_ordinary_type9_pre_publication_state_matches,
        OrdinaryType9PendingInitialSelection, OrdinaryType9SelectedComponentRuntime,
        OrdinaryType9SelectedRuntimeKind,
    },
};

/// Copyable diagnostics from one selected receipt, carrying no initializer
/// authority or component custody.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9SelectedEvidence {
    pub owner_id: u32,
    pub authored_spawn_index: usize,
    /// Exact constructor-final birth position copied into selected component
    /// `+0x90`; retained after task-private target state begins evolving.
    pub immutable_anchor_raw_at_0x90: [i16; 3],
    pub selection: BehaviorSelection,
    pub evaluator_evidence: FreshLevel1Type9EvaluatorEvidence,
    pub selector_random_word: u32,
}

impl OrdinaryType9SelectedEvidence {
    pub(crate) const fn from_receipt(receipt: &FreshLevel1Type9WeightedSelection) -> Self {
        Self {
            owner_id: receipt.owner_id(),
            authored_spawn_index: receipt.authored_spawn_index(),
            immutable_anchor_raw_at_0x90: receipt.owner_snapshot().position_raw,
            selection: receipt.selection(),
            evaluator_evidence: receipt.evaluator_evidence(),
            selector_random_word: receipt.random_word(),
        }
    }
}

/// Common stale/inauthentic entity evidence rejected before branch mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9SelectedInitializerPreflightError {
    EntityInactive,
    OwnerSnapshotMismatch {
        expected: FreshLevel1Type9EntityRef,
        actual: FreshLevel1Type9EntityRef,
    },
    AuthoredSpawnMismatch {
        expected: usize,
        actual: Option<usize>,
    },
    UnexpectedModelSlots {
        actual: [Option<usize>; 4],
    },
    UnexpectedActiveModel {
        actual: Option<usize>,
    },
    UnexpectedActiveModelSlot,
    UnexpectedMass {
        actual: u16,
    },
    UnexpectedCapabilityFlags {
        actual: u32,
    },
    UnexpectedHealth,
    UnexpectedPhysicalBodyBasis,
    UnexpectedAuthoredRotation,
    UnexpectedDefaultStateFlags,
    UnexpectedRecentRelation,
    BirthAnchorUnavailable,
    OwnerPositionDoesNotMatchBirthAnchor {
        expected: [i16; 3],
        actual: [i16; 3],
    },
    UnexpectedPrePublicationState,
    MetadataNotExact,
    InitialBehaviorAlreadyResolved,
    CurrentBehaviorContextAlreadyResolved,
    BirthTaskTableNotEmpty,
    PendingComponentCustodyUnavailable,
    PendingComponentCustodyMismatch,
    SelectedComponentCustodyAlreadyPresent,
    DeathComponentCustodyAlreadyPresent,
    UnexpectedActorCommonAxis,
    UnexpectedSubARuntime,
    UnexpectedSubHRuntime,
    UnexpectedActorAnimationRuntime,
    CanonicalFreshContextUnavailable,
}

/// Linear proof that the common entity prefix still matches one selector
/// receipt. Branch code must authenticate its own class/choice identity before
/// requesting this token.
#[derive(Debug)]
pub(crate) struct OrdinaryType9SelectedInitializerPreflight {
    selection: BehaviorSelection,
    selected_context: BehaviorContextRuntime,
}

/// Common prefix committed before entering one selected initializer body.
pub(crate) struct OrdinaryType9PublishedSelectedPrefix {
    components: OrdinaryType9PendingInitialSelection,
    selected_context: BehaviorContextRuntime,
}

impl OrdinaryType9PublishedSelectedPrefix {
    /// Consume component custody into one successful selected branch.
    pub(crate) fn finish_success(
        self,
        entity: &mut Entity,
        kind: OrdinaryType9SelectedRuntimeKind,
    ) {
        entity.ordinary_type9_selected_component_runtime = Some(
            OrdinaryType9SelectedComponentRuntime::new(self.components, kind),
        );
    }

    /// Consume component custody into the terminal outer C6B0 fallback.
    pub(crate) fn finish_initializer_fallback(self, entity: &mut Entity) {
        entity.publish_behavior_initializer_failure_fallback(self.selected_context);
        entity.ordinary_type9_selected_component_runtime =
            Some(OrdinaryType9SelectedComponentRuntime::new(
                self.components,
                OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished,
            ));
    }
}

impl OrdinaryType9SelectedInitializerPreflight {
    /// Consume the authenticated token and publish the outer chosen-behavior
    /// prefix. No fallible branch operation may precede this call unless it is
    /// read-only: after publication retail owns terminal success or fallback.
    /// The caller must already be on native fresh-context allocation's success
    /// path; this inline Rust representation cannot express allocator failure.
    pub(crate) fn publish(self, entity: &mut Entity) -> OrdinaryType9PublishedSelectedPrefix {
        let components = entity
            .ordinary_type9_pending_initial_selection
            .take()
            .expect("preflight retained exact pending Type-9 custody");
        entity.initial_behavior = RetailRuntimeValue::Known(Some(self.selection));
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(self.selected_context));
        let selected_policy = initial_behavior_state_policy(self.selection.program);
        entity.collision.state_flags_at_0x08.overwrite(
            selected_policy.set_bits | selected_policy.clear_bits,
            selected_policy.set_bits,
        );
        OrdinaryType9PublishedSelectedPrefix {
            components,
            selected_context: self.selected_context,
        }
    }
}

/// Authenticate the common fresh-Level-1 pre-publication entity boundary.
///
/// The caller must first prove that `receipt` names the branch it is about to
/// execute. This function and the returned token perform no allocation, RNG,
/// task mutation, entity linking, or wrapper finalization.
pub(crate) fn preflight_fresh_level1_type9_selected_initializer(
    entity: &Entity,
    receipt: &FreshLevel1Type9WeightedSelection,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<OrdinaryType9SelectedInitializerPreflight, OrdinaryType9SelectedInitializerPreflightError>
{
    let native = receipt.native_receipt();
    if native.is_some_and(|native| !native.authenticates_birth(entity)) {
        return Err(OrdinaryType9SelectedInitializerPreflightError::UnexpectedPrePublicationState);
    }
    if !entity.active {
        return Err(OrdinaryType9SelectedInitializerPreflightError::EntityInactive);
    }
    if entity.ordinary_type9_selected_component_runtime.is_some() {
        return Err(
            OrdinaryType9SelectedInitializerPreflightError::SelectedComponentCustodyAlreadyPresent,
        );
    }
    if entity.main_base_type9_death_component_runtime.is_some() {
        return Err(
            OrdinaryType9SelectedInitializerPreflightError::DeathComponentCustodyAlreadyPresent,
        );
    }
    let actual_owner = FreshLevel1Type9EntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
    };
    let expected_owner = receipt.owner_snapshot();
    if actual_owner != expected_owner {
        return Err(
            OrdinaryType9SelectedInitializerPreflightError::OwnerSnapshotMismatch {
                expected: expected_owner,
                actual: actual_owner,
            },
        );
    }
    if entity.authored_spawn_index != Some(receipt.authored_spawn_index()) {
        return Err(
            OrdinaryType9SelectedInitializerPreflightError::AuthoredSpawnMismatch {
                expected: receipt.authored_spawn_index(),
                actual: entity.authored_spawn_index,
            },
        );
    }
    if entity.model_slots != [Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)); 4] {
        return Err(
            OrdinaryType9SelectedInitializerPreflightError::UnexpectedModelSlots {
                actual: entity.model_slots,
            },
        );
    }
    if entity.model_index != Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)) {
        return Err(
            OrdinaryType9SelectedInitializerPreflightError::UnexpectedActiveModel {
                actual: entity.model_index,
            },
        );
    }
    if entity.collision.active_model_slot()
        != RetailRuntimeValue::Known(native.map_or(0, |native| native.active_slot()))
    {
        return Err(OrdinaryType9SelectedInitializerPreflightError::UnexpectedActiveModelSlot);
    }
    if !native.map_or_else(
        || {
            fresh_level1_ordinary_type9_pre_publication_state_matches(
                entity.collision.state_flags_at_0x08,
            )
        },
        |native| native.pre_publication_state() == entity.collision.state_flags_at_0x08,
    ) {
        return Err(OrdinaryType9SelectedInitializerPreflightError::UnexpectedPrePublicationState);
    }
    if entity.collision.default_state_flags_at_0xc8
        != RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW)
    {
        return Err(OrdinaryType9SelectedInitializerPreflightError::UnexpectedDefaultStateFlags);
    }
    if entity.collision.recent_relation_id_at_0x60 != RetailRuntimeValue::Known(None) {
        return Err(OrdinaryType9SelectedInitializerPreflightError::UnexpectedRecentRelation);
    }
    if entity.rotation_heading_pitch_roll_raw()
        != native.map_or([0; 3], |native| native.rotation_raw())
    {
        return Err(OrdinaryType9SelectedInitializerPreflightError::UnexpectedAuthoredRotation);
    }
    if entity.mass_raw != LEVEL_ONE_TYPE9_MASS_RAW {
        return Err(
            OrdinaryType9SelectedInitializerPreflightError::UnexpectedMass {
                actual: entity.mass_raw,
            },
        );
    }
    if entity.capability_flags != LEVEL_ONE_TYPE9_CAPABILITY_FLAGS {
        return Err(
            OrdinaryType9SelectedInitializerPreflightError::UnexpectedCapabilityFlags {
                actual: entity.capability_flags,
            },
        );
    }
    if entity.collision.health_raw != RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW)
    {
        return Err(OrdinaryType9SelectedInitializerPreflightError::UnexpectedHealth);
    }
    if entity.physical_body_basis_q31 != RetailRuntimeValue::Unresolved {
        return Err(OrdinaryType9SelectedInitializerPreflightError::UnexpectedPhysicalBodyBasis);
    }
    if entity.entity_type != LEVEL_ONE_TYPE9_ENTITY_TYPE
        || !exact_level_one_type9_metadata(metadata)
    {
        return Err(OrdinaryType9SelectedInitializerPreflightError::MetadataNotExact);
    }
    if entity.initial_behavior != RetailRuntimeValue::Unresolved {
        return Err(OrdinaryType9SelectedInitializerPreflightError::InitialBehaviorAlreadyResolved);
    }
    if entity.current_behavior_context != RetailRuntimeValue::Unresolved {
        return Err(
            OrdinaryType9SelectedInitializerPreflightError::CurrentBehaviorContextAlreadyResolved,
        );
    }
    if [
        ActorTaskSlot::Primary,
        ActorTaskSlot::Secondary,
        ActorTaskSlot::Tertiary,
    ]
    .into_iter()
    .any(|slot| entity.actor_tasks.task_in_slot(slot).is_some())
    {
        return Err(OrdinaryType9SelectedInitializerPreflightError::BirthTaskTableNotEmpty);
    }
    let Some(pending) = entity.ordinary_type9_pending_initial_selection else {
        return Err(
            OrdinaryType9SelectedInitializerPreflightError::PendingComponentCustodyUnavailable,
        );
    };
    if pending != receipt.expected_pending_initial_selection() {
        return Err(
            OrdinaryType9SelectedInitializerPreflightError::PendingComponentCustodyMismatch,
        );
    }
    let RetailRuntimeValue::Known(birth_anchor) = pending.immutable_anchor_raw_at_0x90() else {
        return Err(OrdinaryType9SelectedInitializerPreflightError::BirthAnchorUnavailable);
    };
    if entity.position_raw() != birth_anchor {
        return Err(
            OrdinaryType9SelectedInitializerPreflightError::OwnerPositionDoesNotMatchBirthAnchor {
                expected: birth_anchor,
                actual: entity.position_raw(),
            },
        );
    }
    if entity.actor_common_axis_descriptor
        != RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR)
    {
        return Err(OrdinaryType9SelectedInitializerPreflightError::UnexpectedActorCommonAxis);
    }
    if entity.sub_a_propulsion_runtime
        != RetailRuntimeValue::Known(Some(native.map_or_else(
            || SubAPropulsionRuntime::pending_constructor_rng(LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR),
            |native| native.initial_sub_a(),
        )))
    {
        return Err(OrdinaryType9SelectedInitializerPreflightError::UnexpectedSubARuntime);
    }
    if entity.sub_h_external_frame_runtime != RetailRuntimeValue::Known(None) {
        return Err(OrdinaryType9SelectedInitializerPreflightError::UnexpectedSubHRuntime);
    }
    if entity.actor_animation_runtime
        != RetailRuntimeValue::Known(Some(
            ActorAnimationController::from_descriptor(LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR)
                .expect("exact Type-9 Sub-I descriptor is valid"),
        ))
    {
        return Err(
            OrdinaryType9SelectedInitializerPreflightError::UnexpectedActorAnimationRuntime,
        );
    }

    let selected_context = BehaviorContextRuntime::from_fresh_weighted_selection(
        receipt.selection(),
    )
    .ok_or(OrdinaryType9SelectedInitializerPreflightError::CanonicalFreshContextUnavailable)?;
    Ok(OrdinaryType9SelectedInitializerPreflight {
        selection: receipt.selection(),
        selected_context,
    })
}
