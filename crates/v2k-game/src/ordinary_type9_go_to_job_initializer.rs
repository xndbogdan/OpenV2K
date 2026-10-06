//! Bounded already-selected Type-9 class-54 initializer publication.
//!
//! This adapter consumes only an exact fresh-Level-1 Go To Job receipt. The
//! class-54 target selector monotonically refines the receipt's exact ordered
//! live-list snapshot with later-consumed state, capability, and job-capacity
//! evidence; callers cannot substitute another list. Read-only target and
//! constructor planning is pulled ahead
//! of selected-prefix publication so unresolved port evidence can return the
//! linear receipt without mutation. The committed transaction still preserves
//! retail's selected prefix, Secondary -> Tertiary clears, fallible Primary
//! preparation, one-word constructor suffix, and terminal outer fallback.
//! Entity linking and `FUN_00413F70` wrapper finalization remain outside this
//! bounded owner.

use std::num::NonZeroU32;

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, PreparedActorTask},
    entity::Entity,
    entity_behavior::behavior_program,
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    go_to_job::{
        plan_go_to_job_setup, GoToJobApplyError, GoToJobCandidate, GoToJobOwner, GoToJobSetupError,
        GoToJobSetupPlan, GoToJobSetupRequest, GoToJobTaskSpec,
    },
    go_to_job_owner::GoToJobTaskState,
    guard_location_owner::acquisition::GuardLocationEntityRef,
    job_nearby::JobCapacityState,
    ordinary_type9_initial_selection::{
        FreshLevel1Type9InitializerIdentity, FreshLevel1Type9WeightedSelection,
        LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID,
    },
    ordinary_type9_live::OrdinaryType9SelectedRuntimeKind,
    ordinary_type9_selected_initializer::{
        preflight_fresh_level1_type9_selected_initializer, OrdinaryType9SelectedEvidence,
        OrdinaryType9SelectedInitializerPreflight, OrdinaryType9SelectedInitializerPreflightError,
    },
    wrapped_axis_range::WrappedAxisRange,
};

/// Caller decision at the exact Primary allocation/private-initialization seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9GoToJobAllocationDecision {
    Prepared,
    Failed,
}

pub type OrdinaryType9GoToJobSelectionEvidence = OrdinaryType9SelectedEvidence;

/// Class-54 evidence paired by index with the exact ordered entity snapshot
/// retained by the weighted-selection receipt.
///
/// State and capability evidence must monotonically refine the retained entry:
/// known state bits cannot be forgotten or contradicted, and a known
/// capability word cannot change. Capacity is class-54-only evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9GoToJobCandidateEvidence {
    pub candidate_id: u32,
    pub state_flags: crate::entity_collision_state::RetailStateWord,
    pub capability_flags: RetailRuntimeValue<u32>,
    pub capacity: RetailRuntimeValue<Option<JobCapacityState>>,
}

/// Read-only result of class-54's separate nearest-compatible-job scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9GoToJobTargetEvidence {
    pub target_id: NonZeroU32,
}

/// Successful one-word generic suffix plus class-54's fixed Sub-A overwrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9GoToJobConstructorEvidence {
    pub random_sample_low16: u16,
    pub sub_a_target_speed_raw: i32,
}

/// The failed class-54 task phase consumed by outer C6B0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9GoToJobInitializerFailure {
    pub action_index: usize,
    pub slot: ActorTaskSlot,
}

/// Terminal result after the class-54 descriptor has entered outer C6B0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9GoToJobInitializerOutcome {
    Published {
        selected: OrdinaryType9GoToJobSelectionEvidence,
        target: OrdinaryType9GoToJobTargetEvidence,
        constructor: OrdinaryType9GoToJobConstructorEvidence,
    },
    InitializerFallbackPublished {
        selected: OrdinaryType9GoToJobSelectionEvidence,
        target: OrdinaryType9GoToJobTargetEvidence,
        failure: OrdinaryType9GoToJobInitializerFailure,
    },
}

/// Rejection before selected-prefix publication, task mutation, allocation, or
/// constructor RNG. The failure receipt retains selector authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9GoToJobInitializerError {
    SelectedInitializerNotCanonicalGoToJob,
    EntityPreflight(OrdinaryType9SelectedInitializerPreflightError),
    CandidateEvidenceCountMismatch {
        expected: usize,
        actual: usize,
    },
    CandidateEvidenceOrderMismatch {
        index: usize,
        expected_id: u32,
        actual_id: u32,
    },
    CandidateStateEvidenceDoesNotRefineReceipt {
        id: u32,
    },
    CandidateCapabilityEvidenceDoesNotRefineReceipt {
        id: u32,
    },
    Setup(GoToJobSetupError),
    SelectedReceiptTargetUnavailable,
    SelectedReceiptTargetIsZero,
}

/// Receipt-neutral failure while refining a retained weighted-selection
/// candidate snapshot into class 54's later target/setup transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9GoToJobTaskPlanError {
    CandidateEvidenceCountMismatch {
        expected: usize,
        actual: usize,
    },
    CandidateEvidenceOrderMismatch {
        index: usize,
        expected_id: u32,
        actual_id: u32,
    },
    CandidateStateEvidenceDoesNotRefineSelection {
        id: u32,
    },
    CandidateCapabilityEvidenceDoesNotRefineSelection {
        id: u32,
    },
    Setup(GoToJobSetupError),
    SelectedTargetUnavailable,
    SelectedTargetIsZero,
}

#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9GoToJobInitializerFailureReceipt {
    pub error: OrdinaryType9GoToJobInitializerError,
    receipt: FreshLevel1Type9WeightedSelection,
}

impl OrdinaryType9GoToJobInitializerFailureReceipt {
    pub fn into_receipt(self) -> FreshLevel1Type9WeightedSelection {
        self.receipt
    }

    pub const fn receipt(&self) -> &FreshLevel1Type9WeightedSelection {
        &self.receipt
    }
}

#[derive(Debug)]
struct OrdinaryType9GoToJobPreflight {
    entity: OrdinaryType9SelectedInitializerPreflight,
    selected_evidence: OrdinaryType9GoToJobSelectionEvidence,
    transaction: OrdinaryType9GoToJobTaskTransaction,
}

/// Read-only target and one-shot S -> T -> fallible-P transaction shared by
/// fresh publication and an already-selected root replacement.
#[derive(Debug)]
pub(crate) struct OrdinaryType9GoToJobTaskTransaction {
    target_evidence: OrdinaryType9GoToJobTargetEvidence,
    owner: GoToJobOwner,
    setup: GoToJobSetupPlan,
}

impl OrdinaryType9GoToJobTaskTransaction {
    pub(crate) const fn target_evidence(&self) -> OrdinaryType9GoToJobTargetEvidence {
        self.target_evidence
    }
}

/// Publish one exact already-selected fresh-Level-1 Type-9 Go To Job task.
///
/// `candidate_evidence_in_intrusive_order` refines, but cannot
/// replace or reorder, the selector receipt's entity snapshot. `allocate` is
/// called once after Secondary and Tertiary clear. A
/// failed allocation terminally publishes the outer fallback and consumes no
/// constructor word. Success consumes exactly one word, applies the generic
/// Sub-A reset and class-54 overwrite, then publishes Primary.
pub fn apply_selected_ordinary_type9_go_to_job_initializer(
    entity: &mut Entity,
    receipt: FreshLevel1Type9WeightedSelection,
    metadata: &EntityTypeRuntimeMetadata,
    candidate_evidence_in_intrusive_order: &[OrdinaryType9GoToJobCandidateEvidence],
    mut allocate: impl FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    mut next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9GoToJobInitializerOutcome, OrdinaryType9GoToJobInitializerFailureReceipt> {
    let preflight = match preflight_selected_go_to_job(
        entity,
        &receipt,
        metadata,
        candidate_evidence_in_intrusive_order,
    ) {
        Ok(preflight) => preflight,
        Err(error) => {
            return Err(OrdinaryType9GoToJobInitializerFailureReceipt { error, receipt });
        }
    };

    let selected = preflight.selected_evidence;
    let target = preflight.transaction.target_evidence();
    let published = preflight.entity.publish(entity);
    let setup_result = apply_ordinary_type9_go_to_job_task_transaction(
        entity,
        preflight.transaction,
        &mut allocate,
        &mut next_constructor_word,
    );

    match setup_result {
        Ok(constructor) => {
            published.finish_success(entity, OrdinaryType9SelectedRuntimeKind::GoToJobPublished);
            Ok(OrdinaryType9GoToJobInitializerOutcome::Published {
                selected,
                target,
                constructor,
            })
        }
        Err(failure) => {
            published.finish_initializer_fallback(entity);
            Ok(
                OrdinaryType9GoToJobInitializerOutcome::InitializerFallbackPublished {
                    selected,
                    target,
                    failure,
                },
            )
        }
    }
}

/// Refine the exact candidate array retained by a weighted selection into the
/// later class-54 target and task transaction. This function has no fresh-
/// construction receipt or component-custody dependency.
pub(crate) fn plan_ordinary_type9_go_to_job_task_transaction(
    owner: GoToJobOwner,
    selected_candidates_in_intrusive_order: &[GuardLocationEntityRef],
    candidate_evidence_in_intrusive_order: &[OrdinaryType9GoToJobCandidateEvidence],
    range: WrappedAxisRange,
) -> Result<OrdinaryType9GoToJobTaskTransaction, OrdinaryType9GoToJobTaskPlanError> {
    if selected_candidates_in_intrusive_order.len() != candidate_evidence_in_intrusive_order.len() {
        return Err(
            OrdinaryType9GoToJobTaskPlanError::CandidateEvidenceCountMismatch {
                expected: selected_candidates_in_intrusive_order.len(),
                actual: candidate_evidence_in_intrusive_order.len(),
            },
        );
    }
    let mut job_candidates = Vec::with_capacity(selected_candidates_in_intrusive_order.len());
    for (index, (candidate, evidence)) in selected_candidates_in_intrusive_order
        .iter()
        .zip(candidate_evidence_in_intrusive_order)
        .enumerate()
    {
        if evidence.candidate_id != candidate.id {
            return Err(
                OrdinaryType9GoToJobTaskPlanError::CandidateEvidenceOrderMismatch {
                    index,
                    expected_id: candidate.id,
                    actual_id: evidence.candidate_id,
                },
            );
        }
        let retained_state = candidate.state_flags_raw;
        if evidence.state_flags.known_mask() & retained_state.known_mask()
            != retained_state.known_mask()
            || (evidence.state_flags.known_value_bits() ^ retained_state.known_value_bits())
                & retained_state.known_mask()
                != 0
        {
            return Err(
                OrdinaryType9GoToJobTaskPlanError::CandidateStateEvidenceDoesNotRefineSelection {
                    id: candidate.id,
                },
            );
        }
        let capability_flags = match (candidate.capability_flags, evidence.capability_flags) {
            (RetailRuntimeValue::Known(retained), RetailRuntimeValue::Known(refined))
                if retained == refined =>
            {
                RetailRuntimeValue::Known(retained)
            }
            (RetailRuntimeValue::Known(_), _) => {
                return Err(
                    OrdinaryType9GoToJobTaskPlanError::CandidateCapabilityEvidenceDoesNotRefineSelection {
                        id: candidate.id,
                    },
                );
            }
            (RetailRuntimeValue::Unresolved, refined) => refined,
        };
        job_candidates.push(GoToJobCandidate {
            id: candidate.id,
            position_raw: candidate.position_raw,
            state_flags: evidence.state_flags,
            capability_flags,
            capacity: evidence.capacity,
        });
    }
    let setup = plan_go_to_job_setup(GoToJobSetupRequest {
        owner,
        candidates_in_intrusive_order: &job_candidates,
        range,
    })
    .map_err(OrdinaryType9GoToJobTaskPlanError::Setup)?;
    let target_id = setup
        .target_id()
        .ok_or(OrdinaryType9GoToJobTaskPlanError::SelectedTargetUnavailable)?;
    let target_id = NonZeroU32::new(target_id)
        .ok_or(OrdinaryType9GoToJobTaskPlanError::SelectedTargetIsZero)?;
    Ok(OrdinaryType9GoToJobTaskTransaction {
        target_evidence: OrdinaryType9GoToJobTargetEvidence { target_id },
        owner,
        setup,
    })
}

/// Commit the exact class-54 S -> T -> fallible-P transaction against one
/// entity. Allocation failure returns before constructor RNG or Sub-A writes;
/// success consumes exactly one word and publishes only after the fixed 4/3
/// target-speed overwrite.
pub(crate) fn apply_ordinary_type9_go_to_job_task_transaction(
    entity: &mut Entity,
    transaction: OrdinaryType9GoToJobTaskTransaction,
    allocate: impl FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9GoToJobConstructorEvidence, OrdinaryType9GoToJobInitializerFailure> {
    let Entity {
        id,
        actor_tasks,
        sub_a_propulsion_runtime,
        ..
    } = entity;
    let sub_a = match sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(sub_a)) => sub_a,
        RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => {
            unreachable!("class-54 preflight retained live Type-9 Sub-A custody")
        }
    };
    apply_ordinary_type9_go_to_job_task_transaction_parts(
        *id,
        actor_tasks,
        sub_a,
        transaction,
        allocate,
        next_constructor_word,
        |_| {},
    )
}

/// Commit the class-54 task transaction through an authenticated split borrow
/// of one entity's task table and Sub-A runtime.
pub(crate) fn apply_ordinary_type9_go_to_job_task_transaction_parts(
    runtime_owner_id: u32,
    actor_tasks: &mut crate::actor_task_owner::ActorTaskOwner<ActorTaskRuntime>,
    sub_a: &mut crate::common_mover::SubAPropulsionRuntime,
    transaction: OrdinaryType9GoToJobTaskTransaction,
    mut allocate: impl FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    mut next_constructor_word: impl FnMut() -> u32,
    retire: impl FnMut(&ActorTaskRuntime),
) -> Result<OrdinaryType9GoToJobConstructorEvidence, OrdinaryType9GoToJobInitializerFailure> {
    let final_target_speed_raw = transaction.setup.constructor_sub_a_target_speed_raw();
    let runtime = transaction
        .owner
        .bind_authenticated_parts_runtime(runtime_owner_id, actor_tasks, sub_a)
        .expect("class-54 preflight retained matching split runtime custody");
    let mut constructor_word = None;
    let setup_result = transaction.setup.apply_with_retirement(
        runtime,
        || {
            let word = next_constructor_word();
            constructor_word = Some(word);
            word
        },
        |specification| {
            if allocate(specification) == OrdinaryType9GoToJobAllocationDecision::Failed {
                return Err(());
            }
            Ok(PreparedActorTask::new(ActorTaskRuntime::GoToJob(
                GoToJobTaskState::after_allocation(specification),
            )))
        },
        retire,
    );
    match setup_result {
        Ok(()) => Ok(OrdinaryType9GoToJobConstructorEvidence {
            random_sample_low16: constructor_word
                .expect("successful class-54 setup consumes exactly one suffix word")
                as u16,
            sub_a_target_speed_raw: final_target_speed_raw,
        }),
        Err(GoToJobApplyError::Prepare(error)) => {
            debug_assert!(constructor_word.is_none());
            Err(OrdinaryType9GoToJobInitializerFailure {
                action_index: error.action_index,
                slot: error.slot,
            })
        }
        Err(GoToJobApplyError::OwnerRuntimeMismatch { .. }) => {
            unreachable!("class-54 task plan and bound runtime share one entity owner")
        }
    }
}

fn map_task_plan_error(
    error: OrdinaryType9GoToJobTaskPlanError,
) -> OrdinaryType9GoToJobInitializerError {
    match error {
        OrdinaryType9GoToJobTaskPlanError::CandidateEvidenceCountMismatch { expected, actual } => {
            OrdinaryType9GoToJobInitializerError::CandidateEvidenceCountMismatch {
                expected,
                actual,
            }
        }
        OrdinaryType9GoToJobTaskPlanError::CandidateEvidenceOrderMismatch {
            index,
            expected_id,
            actual_id,
        } => OrdinaryType9GoToJobInitializerError::CandidateEvidenceOrderMismatch {
            index,
            expected_id,
            actual_id,
        },
        OrdinaryType9GoToJobTaskPlanError::CandidateStateEvidenceDoesNotRefineSelection { id } => {
            OrdinaryType9GoToJobInitializerError::CandidateStateEvidenceDoesNotRefineReceipt { id }
        }
        OrdinaryType9GoToJobTaskPlanError::CandidateCapabilityEvidenceDoesNotRefineSelection {
            id,
        } => {
            OrdinaryType9GoToJobInitializerError::CandidateCapabilityEvidenceDoesNotRefineReceipt {
                id,
            }
        }
        OrdinaryType9GoToJobTaskPlanError::Setup(error) => {
            OrdinaryType9GoToJobInitializerError::Setup(error)
        }
        OrdinaryType9GoToJobTaskPlanError::SelectedTargetUnavailable => {
            OrdinaryType9GoToJobInitializerError::SelectedReceiptTargetUnavailable
        }
        OrdinaryType9GoToJobTaskPlanError::SelectedTargetIsZero => {
            OrdinaryType9GoToJobInitializerError::SelectedReceiptTargetIsZero
        }
    }
}

fn preflight_selected_go_to_job(
    entity: &Entity,
    receipt: &FreshLevel1Type9WeightedSelection,
    metadata: &EntityTypeRuntimeMetadata,
    candidate_evidence_in_intrusive_order: &[OrdinaryType9GoToJobCandidateEvidence],
) -> Result<OrdinaryType9GoToJobPreflight, OrdinaryType9GoToJobInitializerError> {
    let canonical_program = behavior_program(LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID)
        .expect("class-54 Go To Job is statically audited");
    if receipt.initializer_identity() != FreshLevel1Type9InitializerIdentity::GoToJob
        || receipt.selection().choice_index != 2
        || receipt.selection().program != canonical_program
    {
        return Err(OrdinaryType9GoToJobInitializerError::SelectedInitializerNotCanonicalGoToJob);
    }
    let selected_entity =
        preflight_fresh_level1_type9_selected_initializer(entity, receipt, metadata)
            .map_err(OrdinaryType9GoToJobInitializerError::EntityPreflight)?;
    let owner = GoToJobOwner::from_type_metadata(
        entity.id,
        entity.position_raw(),
        RetailRuntimeValue::Known(entity.capability_flags),
        u16::try_from(entity.entity_type).expect("exact fresh-Level-1 Type-9 entity type fits u16"),
        metadata,
    );
    let initializer = metadata
        .initializer
        .as_ref()
        .expect("common exact-metadata preflight retained the Type-9 initializer");
    let transaction = plan_ordinary_type9_go_to_job_task_transaction(
        owner,
        receipt.candidates_in_intrusive_order(),
        candidate_evidence_in_intrusive_order,
        WrappedAxisRange::from_raw(initializer.common_axis_descriptor.strict_axis_limit_raw),
    )
    .map_err(map_task_plan_error)?;
    Ok(OrdinaryType9GoToJobPreflight {
        entity: selected_entity,
        selected_evidence: OrdinaryType9SelectedEvidence::from_receipt(receipt),
        transaction,
    })
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;
    use crate::{
        actor_animation::ActorAnimationController,
        common_mover::{sub_d::ORDINARY_TYPE9_SUB_D, SubAPropulsionRuntime},
        entity::{Entity, EntityKind},
        entity_behavior::{
            ActiveBehaviorStyle, BehaviorChoiceListSource, BehaviorDescriptorIdentity,
        },
        entity_collision_state::{EntityInitializerSpec, RetailStateWord},
        main_base_type9_abort::{
            LEVEL_ONE_TYPE9_BEHAVIOR_CHOICES, LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
            LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR, LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY,
            LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS, LEVEL_ONE_TYPE9_DEATH_SOUND_ID,
            LEVEL_ONE_TYPE9_ENTITY_TYPE, LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW,
            LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW, LEVEL_ONE_TYPE9_MASS_RAW, LEVEL_ONE_TYPE9_MODEL_ID,
            LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR, LEVEL_ONE_TYPE9_SUB_B_DESCRIPTOR,
            LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR,
        },
        ordinary_type9_initial_selection::{
            plan_fresh_level1_type9_weighted_selection, FreshLevel1Type9EntityRef,
            FreshLevel1Type9WeightedSelectionRequest, LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK,
        },
        ordinary_type9_live::{
            admit_fresh_level1_ordinary_type9, FreshLevel1OrdinaryType9SpawnFacts,
            OrdinaryType9SelectedComponentRuntime,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_MASK,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE,
        },
        shared_retarget_mover::SharedRetargetTaskState,
    };

    const OWNER_ID: u32 = 0x04A9_0001;
    const SPAWN_INDEX: usize = 9;
    const IMMUTABLE_ANCHOR: [i16; 3] = [111, 22, -333];
    const BASE_WITNESS_ID: u32 = 0x04A4_0001;
    const NEAREST_JOB_ID: u32 = 0x04A4_0002;
    const EQUAL_DISTANCE_JOB_ID: u32 = 0x04A4_0003;

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [LEVEL_ONE_TYPE9_MODEL_ID as u16; 4],
            mass_raw: LEVEL_ONE_TYPE9_MASS_RAW,
            capability_flags: LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
            initial_health_raw: Some(LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW),
            accepted_hit_presentation_sound_id: RetailRuntimeValue::Known(Some(95)),
            death_sound_id: RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE9_DEATH_SOUND_ID)),
            constructor_sound_attachment_id: RetailRuntimeValue::Known(None),
            generic_hit_sound_id: RetailRuntimeValue::Known(None),
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR,
            )),
            sub_b_lateral_descriptor: RetailRuntimeValue::Known(Some(
                LEVEL_ONE_TYPE9_SUB_B_DESCRIPTOR,
            )),
            sub_d_steering_descriptor: RetailRuntimeValue::Known(Some(ORDINARY_TYPE9_SUB_D)),
            actor_animation_descriptor: RetailRuntimeValue::Known(Some(
                LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR,
            )),
            common_mover_topology: RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW,
                common_axis_descriptor: LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR,
                behavior_choices: LEVEL_ONE_TYPE9_BEHAVIOR_CHOICES.to_vec().into_boxed_slice(),
                behavior_rule_ref: 1,
                alternate_behavior_class_ref: u32::from(LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS),
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn admission() -> crate::ordinary_type9_live::FreshLevel1OrdinaryType9Admission {
        admit_fresh_level1_ordinary_type9(FreshLevel1OrdinaryType9SpawnFacts {
            retail_first_world: true,
            authored_spawn_index: SPAWN_INDEX,
            entity_type: LEVEL_ONE_TYPE9_ENTITY_TYPE,
            active_model_slot: RetailRuntimeValue::Known(0),
            active_model: Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)),
            rotation: [0; 3],
            immutable_anchor_raw_at_0x90: RetailRuntimeValue::Known(IMMUTABLE_ANCHOR),
        })
        .expect("exact fresh Type-9 admission")
    }

    fn exact_entity() -> Entity {
        let mut entity = Entity::unresolved_port_entity(
            OWNER_ID,
            EntityKind::Unknown(LEVEL_ONE_TYPE9_ENTITY_TYPE),
            LEVEL_ONE_TYPE9_ENTITY_TYPE,
        );
        entity.authored_spawn_index = Some(SPAWN_INDEX);
        entity.model_slots = [Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)); 4];
        entity.model_index = Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID));
        entity.mass_raw = LEVEL_ONE_TYPE9_MASS_RAW;
        entity.capability_flags = LEVEL_ONE_TYPE9_CAPABILITY_FLAGS;
        entity.collision.state_flags_at_0x08 = RetailStateWord::from_known_bits(
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_MASK,
        );
        entity.collision.default_state_flags_at_0xc8 =
            RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW);
        entity.collision.health_raw = RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW);
        entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        entity.initial_behavior = RetailRuntimeValue::Unresolved;
        entity.current_behavior_context = RetailRuntimeValue::Unresolved;
        entity.actor_common_axis_descriptor =
            RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR);
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::pending_constructor_rng(LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR),
        ));
        entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(None);
        entity.actor_animation_runtime = RetailRuntimeValue::Known(Some(
            ActorAnimationController::from_descriptor(LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR).unwrap(),
        ));
        entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
        entity.ordinary_type9_pending_initial_selection =
            Some(admission().pending_initial_selection());
        entity.set_motion_raw(IMMUTABLE_ANCHOR, [0; 3]);
        entity
    }

    fn owner_snapshot(entity: &Entity) -> FreshLevel1Type9EntityRef {
        FreshLevel1Type9EntityRef {
            id: entity.id,
            entity_type: entity.entity_type,
            position_raw: entity.position_raw(),
            state_flags_raw: entity.collision.state_flags_at_0x08,
            capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
            attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
        }
    }

    fn selector_candidate(
        id: u32,
        position_raw: [i16; 3],
        capability_flags: u32,
    ) -> FreshLevel1Type9EntityRef {
        FreshLevel1Type9EntityRef {
            id,
            entity_type: 66,
            position_raw,
            state_flags_raw: RetailStateWord::exact(1),
            capability_flags: RetailRuntimeValue::Known(capability_flags),
            attached_entity_handle: RetailRuntimeValue::Known(None),
        }
    }

    fn selector_candidates() -> [FreshLevel1Type9EntityRef; 3] {
        [
            selector_candidate(
                BASE_WITNESS_ID,
                [879, 22, -333],
                LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK,
            ),
            selector_candidate(
                NEAREST_JOB_ID,
                [367, 22, -333],
                LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK,
            ),
            selector_candidate(
                EQUAL_DISTANCE_JOB_ID,
                [-145, 22, -333],
                LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK,
            ),
        ]
    }

    fn go_to_job_receipt(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> FreshLevel1Type9WeightedSelection {
        let candidates = selector_candidates();
        go_to_job_receipt_with_candidates(entity, metadata, &candidates)
    }

    fn go_to_job_receipt_with_candidates(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
        candidates: &[FreshLevel1Type9EntityRef],
    ) -> FreshLevel1Type9WeightedSelection {
        go_to_job_receipt_with_candidates_and_word(entity, metadata, candidates, 0xCAFE_0000)
    }

    fn go_to_job_receipt_with_candidates_and_word(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
        candidates: &[FreshLevel1Type9EntityRef],
        selector_word: u32,
    ) -> FreshLevel1Type9WeightedSelection {
        let selection = plan_fresh_level1_type9_weighted_selection(
            FreshLevel1Type9WeightedSelectionRequest {
                admission: admission(),
                authored_spawn_index: SPAWN_INDEX,
                active_model_id: usize::from(LEVEL_ONE_TYPE9_MODEL_ID),
                metadata,
                owner: owner_snapshot(entity),
                candidates_in_intrusive_order: candidates,
            },
            || selector_word,
        )
        .expect("Base Nearby selects Go To Job");
        assert_eq!(
            selection.initializer_identity(),
            FreshLevel1Type9InitializerIdentity::GoToJob
        );
        selection
    }

    fn wander_receipt(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> FreshLevel1Type9WeightedSelection {
        plan_fresh_level1_type9_weighted_selection(
            FreshLevel1Type9WeightedSelectionRequest {
                admission: admission(),
                authored_spawn_index: SPAWN_INDEX,
                active_model_id: usize::from(LEVEL_ONE_TYPE9_MODEL_ID),
                metadata,
                owner: owner_snapshot(entity),
                candidates_in_intrusive_order: &[],
            },
            || u32::MAX,
        )
        .expect("Always fallback selects Wander Near")
    }

    fn candidate_evidence(
        candidate: FreshLevel1Type9EntityRef,
    ) -> OrdinaryType9GoToJobCandidateEvidence {
        OrdinaryType9GoToJobCandidateEvidence {
            candidate_id: candidate.id,
            state_flags: candidate.state_flags_raw,
            capability_flags: candidate.capability_flags,
            capacity: RetailRuntimeValue::Unresolved,
        }
    }

    fn default_candidate_evidence() -> [OrdinaryType9GoToJobCandidateEvidence; 3] {
        selector_candidates().map(candidate_evidence)
    }

    fn selected_runtime(entity: &Entity) -> OrdinaryType9SelectedComponentRuntime {
        entity
            .ordinary_type9_selected_component_runtime
            .expect("terminal initializer must retain selected component custody")
    }

    fn assert_wrapper_pending(entity: &Entity, fixed_state: u32) {
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_MASK),
            RetailRuntimeValue::Known(fixed_state)
        );
        assert_eq!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Unresolved
        );
    }

    #[test]
    fn success_uses_the_receipt_snapshot_nearest_tie_then_one_lazy_constructor_word() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Event {
            Allocate(Option<u32>),
            Word(u32),
        }

        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let original_axis = entity.actor_common_axis_descriptor;
        let expected_components = entity.ordinary_type9_pending_initial_selection.unwrap();
        let receipt = go_to_job_receipt(&entity, &metadata);
        let capacities = default_candidate_evidence();
        let events = RefCell::new(Vec::new());
        let word = 0x1234_D2F6;
        let outcome = apply_selected_ordinary_type9_go_to_job_initializer(
            &mut entity,
            receipt,
            &metadata,
            &capacities,
            |specification| {
                events
                    .borrow_mut()
                    .push(Event::Allocate(specification.target_id()));
                assert_eq!(specification.initial_position_raw(), IMMUTABLE_ANCHOR);
                OrdinaryType9GoToJobAllocationDecision::Prepared
            },
            || {
                events.borrow_mut().push(Event::Word(word));
                word
            },
        )
        .unwrap();

        let OrdinaryType9GoToJobInitializerOutcome::Published {
            selected,
            target,
            constructor,
        } = outcome
        else {
            panic!("expected selected Go To Job publication: {outcome:?}")
        };
        assert_eq!(
            events.into_inner(),
            [Event::Allocate(Some(NEAREST_JOB_ID)), Event::Word(word)]
        );
        assert_eq!(selected.selection.choice_index, 2);
        assert_eq!(selected.selection.program.class_id, 54);
        assert_eq!(selected.selector_random_word, 0xCAFE_0000);
        assert_eq!(
            selected
                .evaluator_evidence
                .base_nearby
                .selected_candidate
                .unwrap()
                .id,
            BASE_WITNESS_ID
        );
        assert_ne!(BASE_WITNESS_ID, target.target_id.get());
        assert_eq!(target.target_id.get(), NEAREST_JOB_ID);
        assert_eq!(
            constructor,
            OrdinaryType9GoToJobConstructorEvidence {
                random_sample_low16: 0xD2F6,
                sub_a_target_speed_raw: 333,
            }
        );
        let Some(ActorTaskRuntime::GoToJob(primary)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("class 54 must publish one Go To Job Primary")
        };
        assert_eq!(primary.target_id(), Some(NEAREST_JOB_ID));
        assert_eq!(
            primary.private_state().target_position_raw,
            IMMUTABLE_ANCHOR
        );
        assert_eq!(primary.private_state().direction, 1);
        assert_eq!(primary.private_state().reversal_timer_ms, 0);
        assert_eq!(primary.elapsed_ms(), 0);
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Secondary), None);
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!()
        };
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(constructor.sub_a_target_speed_raw)
        );
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(sub_a.drive_scale_percent(), 100);
        assert_eq!(entity.actor_common_axis_descriptor, original_axis);
        assert_eq!(
            entity.initial_behavior,
            RetailRuntimeValue::Known(Some(selected.selection))
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::Named(selected.selection.program)
        );
        assert_eq!(context.active_style().audited().unwrap().variant, 0);
        assert_eq!(
            context.choice_list_source(),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        );
        assert_eq!(entity.ordinary_type9_pending_initial_selection, None);
        assert_eq!(selected_runtime(&entity).components(), expected_components);
        assert_eq!(
            selected_runtime(&entity).kind(),
            OrdinaryType9SelectedRuntimeKind::GoToJobPublished
        );
        assert_wrapper_pending(
            &entity,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE,
        );
    }

    #[test]
    fn mismatched_candidate_projection_returns_the_receipt_before_publication() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let pending_before = entity.ordinary_type9_pending_initial_selection;
        let receipt = go_to_job_receipt(&entity, &metadata);
        let calls = Cell::new(0);
        let failure = apply_selected_ordinary_type9_go_to_job_initializer(
            &mut entity,
            receipt,
            &metadata,
            &[],
            |_| {
                calls.set(calls.get() + 1);
                OrdinaryType9GoToJobAllocationDecision::Prepared
            },
            || {
                calls.set(calls.get() + 1);
                0
            },
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9GoToJobInitializerError::CandidateEvidenceCountMismatch {
                expected: 3,
                actual: 0,
            }
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(entity.initial_behavior, RetailRuntimeValue::Unresolved);
        assert_eq!(
            entity.ordinary_type9_pending_initial_selection,
            pending_before
        );

        let receipt = failure.into_receipt();
        let mut reordered = default_candidate_evidence();
        reordered.swap(0, 1);
        let failure = apply_selected_ordinary_type9_go_to_job_initializer(
            &mut entity,
            receipt,
            &metadata,
            &reordered,
            |_| panic!("reordered evidence must stop before allocation"),
            || panic!("reordered evidence must stop before RNG"),
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9GoToJobInitializerError::CandidateEvidenceOrderMismatch {
                index: 0,
                expected_id: BASE_WITNESS_ID,
                actual_id: NEAREST_JOB_ID,
            }
        );

        let receipt = failure.into_receipt();
        let capacities = default_candidate_evidence();
        assert!(apply_selected_ordinary_type9_go_to_job_initializer(
            &mut entity,
            receipt,
            &metadata,
            &capacities,
            |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
            || 0,
        )
        .is_ok());
    }

    #[test]
    fn zero_port_target_is_rejected_before_publication() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let candidate = [selector_candidate(
            0,
            [367, 22, -333],
            LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK,
        )];
        let receipt = go_to_job_receipt_with_candidates(&entity, &metadata, &candidate);
        let capacities = [candidate_evidence(candidate[0])];
        let failure = apply_selected_ordinary_type9_go_to_job_initializer(
            &mut entity,
            receipt,
            &metadata,
            &capacities,
            |_| panic!("zero target must stop before allocation"),
            || panic!("zero target must stop before constructor RNG"),
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9GoToJobInitializerError::SelectedReceiptTargetIsZero
        );
        assert_eq!(entity.initial_behavior, RetailRuntimeValue::Unresolved);
        assert!(entity.ordinary_type9_pending_initial_selection.is_some());
    }

    #[test]
    fn unresolved_target_evidence_returns_the_receipt_before_publication_and_can_retry() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let pending_before = entity.ordinary_type9_pending_initial_selection;
        let sub_a_before = entity.sub_a_propulsion_runtime;
        let unresolved_id = 0x04A4_0020;
        let target_id = 0x04A4_0021;
        let candidates = [
            selector_candidate(unresolved_id, IMMUTABLE_ANCHOR, 0),
            selector_candidate(
                target_id,
                [367, 22, -333],
                LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK,
            ),
        ];
        let receipt = go_to_job_receipt_with_candidates(&entity, &metadata, &candidates);
        let unresolved = candidates.map(candidate_evidence);
        let calls = Cell::new(0);
        let failure = apply_selected_ordinary_type9_go_to_job_initializer(
            &mut entity,
            receipt,
            &metadata,
            &unresolved,
            |_| {
                calls.set(calls.get() + 1);
                OrdinaryType9GoToJobAllocationDecision::Prepared
            },
            || {
                calls.set(calls.get() + 1);
                0
            },
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9GoToJobInitializerError::Setup(
                GoToJobSetupError::CandidateCapacityUnresolved { id: unresolved_id }
            )
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(entity.initial_behavior, RetailRuntimeValue::Unresolved);
        assert_eq!(
            entity.current_behavior_context,
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(
            entity.ordinary_type9_pending_initial_selection,
            pending_before
        );
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before);

        let receipt = failure.into_receipt();
        let mut repaired = candidates.map(candidate_evidence);
        repaired[0].capacity = RetailRuntimeValue::Known(None);
        assert!(apply_selected_ordinary_type9_go_to_job_initializer(
            &mut entity,
            receipt,
            &metadata,
            &repaired,
            |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
            || 0,
        )
        .is_ok());
    }

    #[test]
    fn later_unresolved_candidate_can_refine_the_same_receipt_without_redrawing_selection() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let witness_id = 0x04A4_0040;
        let nearer_id = 0x04A4_0041;
        let candidates = [
            selector_candidate(witness_id, [879, 22, -333], 0x29),
            FreshLevel1Type9EntityRef {
                id: nearer_id,
                entity_type: 66,
                position_raw: [367, 22, -333],
                state_flags_raw: RetailStateWord::unknown(),
                capability_flags: RetailRuntimeValue::Unresolved,
                attached_entity_handle: RetailRuntimeValue::Known(None),
            },
        ];
        let receipt = go_to_job_receipt_with_candidates_and_word(
            &entity,
            &metadata,
            &candidates,
            0xCAFE_0F8E,
        );
        let calls = Cell::new(0);
        let unresolved = candidates.map(candidate_evidence);
        let failure = apply_selected_ordinary_type9_go_to_job_initializer(
            &mut entity,
            receipt,
            &metadata,
            &unresolved,
            |_| {
                calls.set(calls.get() + 1);
                OrdinaryType9GoToJobAllocationDecision::Prepared
            },
            || {
                calls.set(calls.get() + 1);
                0
            },
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9GoToJobInitializerError::Setup(
                GoToJobSetupError::CandidateStateUnresolved { id: nearer_id }
            )
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(entity.initial_behavior, RetailRuntimeValue::Unresolved);

        let receipt = failure.into_receipt();
        let mut state_conflict = candidates.map(candidate_evidence);
        state_conflict[0].state_flags = RetailStateWord::exact(0);
        let failure = apply_selected_ordinary_type9_go_to_job_initializer(
            &mut entity,
            receipt,
            &metadata,
            &state_conflict,
            |_| panic!("contradictory state must stop before allocation"),
            || panic!("contradictory state must stop before RNG"),
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9GoToJobInitializerError::CandidateStateEvidenceDoesNotRefineReceipt {
                id: witness_id,
            }
        );

        let receipt = failure.into_receipt();
        let mut capability_conflict = candidates.map(candidate_evidence);
        capability_conflict[0].capability_flags = RetailRuntimeValue::Known(0x20);
        let failure = apply_selected_ordinary_type9_go_to_job_initializer(
            &mut entity,
            receipt,
            &metadata,
            &capability_conflict,
            |_| panic!("contradictory capability must stop before allocation"),
            || panic!("contradictory capability must stop before RNG"),
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9GoToJobInitializerError::CandidateCapabilityEvidenceDoesNotRefineReceipt {
                id: witness_id,
            }
        );

        let receipt = failure.into_receipt();
        let mut refined = candidates.map(candidate_evidence);
        refined[1].state_flags = RetailStateWord::exact(1);
        refined[1].capability_flags = RetailRuntimeValue::Known(0x20);
        let outcome = apply_selected_ordinary_type9_go_to_job_initializer(
            &mut entity,
            receipt,
            &metadata,
            &refined,
            |_| {
                calls.set(calls.get() + 1);
                OrdinaryType9GoToJobAllocationDecision::Prepared
            },
            || {
                calls.set(calls.get() + 1);
                0
            },
        )
        .unwrap();
        let OrdinaryType9GoToJobInitializerOutcome::Published {
            selected, target, ..
        } = outcome
        else {
            panic!("refined evidence must publish")
        };
        assert_eq!(selected.selector_random_word, 0xCAFE_0F8E);
        assert_eq!(target.target_id.get(), nearer_id);
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn allocation_failure_consumes_no_word_and_terminally_publishes_fallback() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let expected_components = entity.ordinary_type9_pending_initial_selection.unwrap();
        let initial_sub_a = entity.sub_a_propulsion_runtime;
        let receipt = go_to_job_receipt(&entity, &metadata);
        let capacities = default_candidate_evidence();
        let allocations = Cell::new(0);
        let draws = Cell::new(0);
        let outcome = apply_selected_ordinary_type9_go_to_job_initializer(
            &mut entity,
            receipt,
            &metadata,
            &capacities,
            |_| {
                allocations.set(allocations.get() + 1);
                OrdinaryType9GoToJobAllocationDecision::Failed
            },
            || {
                draws.set(draws.get() + 1);
                panic!("failed allocation must not consume constructor RNG")
            },
        )
        .unwrap();
        let OrdinaryType9GoToJobInitializerOutcome::InitializerFallbackPublished {
            selected,
            target,
            failure,
        } = outcome
        else {
            panic!("expected terminal fallback: {outcome:?}")
        };
        assert_eq!(selected.selection.program.class_id, 54);
        assert_eq!(target.target_id.get(), NEAREST_JOB_ID);
        assert_eq!(
            failure,
            OrdinaryType9GoToJobInitializerFailure {
                action_index: 2,
                slot: ActorTaskSlot::Primary,
            }
        );
        assert_eq!((allocations.get(), draws.get()), (1, 0));
        assert_eq!(entity.sub_a_propulsion_runtime, initial_sub_a);
        for slot in [
            ActorTaskSlot::Secondary,
            ActorTaskSlot::Tertiary,
            ActorTaskSlot::Primary,
        ] {
            assert_eq!(entity.actor_task_state(slot), None);
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::InitializerFailureFallback
        );
        assert_eq!(
            context.active_style(),
            ActiveBehaviorStyle::InitializerFailureFallback
        );
        assert_eq!(selected_runtime(&entity).components(), expected_components);
        assert_eq!(
            selected_runtime(&entity).kind(),
            OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
        );
        assert_wrapper_pending(&entity, 0x0600_0801);
    }

    #[test]
    fn wrong_branch_and_stale_entity_win_before_target_planning_or_callbacks() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let calls = Cell::new(0);
        let wrong_receipt = wander_receipt(&entity, &metadata);
        let failure = apply_selected_ordinary_type9_go_to_job_initializer(
            &mut entity,
            wrong_receipt,
            &metadata,
            &[],
            |_| {
                calls.set(calls.get() + 1);
                OrdinaryType9GoToJobAllocationDecision::Prepared
            },
            || {
                calls.set(calls.get() + 1);
                0
            },
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9GoToJobInitializerError::SelectedInitializerNotCanonicalGoToJob
        );
        assert_eq!(calls.get(), 0);

        let receipt = go_to_job_receipt(&entity, &metadata);
        entity.set_motion_raw([112, 22, -333], [0; 3]);
        let failure = apply_selected_ordinary_type9_go_to_job_initializer(
            &mut entity,
            receipt,
            &metadata,
            &[],
            |_| panic!(),
            || panic!(),
        )
        .unwrap_err();
        assert!(matches!(
            failure.error,
            OrdinaryType9GoToJobInitializerError::EntityPreflight(
                OrdinaryType9SelectedInitializerPreflightError::OwnerSnapshotMismatch { .. }
            )
        ));
        let receipt = failure.into_receipt();
        entity.set_motion_raw(IMMUTABLE_ANCHOR, [0; 3]);
        let capacities = default_candidate_evidence();
        assert!(apply_selected_ordinary_type9_go_to_job_initializer(
            &mut entity,
            receipt,
            &metadata,
            &capacities,
            |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
            || 0,
        )
        .is_ok());
    }

    #[test]
    fn success_and_fallback_both_reject_replay_before_target_planning() {
        for fail_allocation in [false, true] {
            let metadata = exact_metadata();
            let mut entity = exact_entity();
            let receipt = go_to_job_receipt(&entity, &metadata);
            let capacities = default_candidate_evidence();
            apply_selected_ordinary_type9_go_to_job_initializer(
                &mut entity,
                receipt,
                &metadata,
                &capacities,
                |_| {
                    if fail_allocation {
                        OrdinaryType9GoToJobAllocationDecision::Failed
                    } else {
                        OrdinaryType9GoToJobAllocationDecision::Prepared
                    }
                },
                || 0,
            )
            .unwrap();

            let snapshot = exact_entity();
            let replay_receipt = go_to_job_receipt(&snapshot, &metadata);
            let failure = apply_selected_ordinary_type9_go_to_job_initializer(
                &mut entity,
                replay_receipt,
                &metadata,
                &[],
                |_| panic!("replay must stop before allocation"),
                || panic!("replay must stop before RNG"),
            )
            .unwrap_err();
            assert_eq!(
                failure.error,
                OrdinaryType9GoToJobInitializerError::EntityPreflight(
                    OrdinaryType9SelectedInitializerPreflightError::SelectedComponentCustodyAlreadyPresent,
                )
            );
        }
    }

    #[test]
    fn existing_task_preflight_precedes_target_scan_and_surface_bits_survive() {
        let metadata = exact_metadata();
        let mut blocked = exact_entity();
        let receipt = go_to_job_receipt(&blocked, &metadata);
        blocked.actor_tasks.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                SharedRetargetTaskState::new(IMMUTABLE_ANCHOR, 500),
            )),
        );
        let failure = apply_selected_ordinary_type9_go_to_job_initializer(
            &mut blocked,
            receipt,
            &metadata,
            &[],
            |_| panic!(),
            || panic!(),
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9GoToJobInitializerError::EntityPreflight(
                OrdinaryType9SelectedInitializerPreflightError::BirthTaskTableNotEmpty,
            )
        );

        for surface_bits in [
            0,
            crate::entity_collision_state::FULLY_ABOVE_SURFACE_STATE_BIT,
            crate::entity_collision_state::FULLY_BELOW_SURFACE_STATE_BIT,
            crate::entity_collision_state::FULLY_ABOVE_SURFACE_STATE_BIT
                | crate::entity_collision_state::FULLY_BELOW_SURFACE_STATE_BIT,
        ] {
            let mut entity = exact_entity();
            entity.collision.state_flags_at_0x08 = RetailStateWord::exact(
                FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE | surface_bits,
            );
            let receipt = go_to_job_receipt(&entity, &metadata);
            let capacities = default_candidate_evidence();
            assert!(apply_selected_ordinary_type9_go_to_job_initializer(
                &mut entity,
                receipt,
                &metadata,
                &capacities,
                |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
                || 0,
            )
            .is_ok());
            assert_eq!(
                entity.collision.state_flags_at_0x08.masked(0x0060_0000),
                RetailRuntimeValue::Known(surface_bits)
            );
        }
    }
}
