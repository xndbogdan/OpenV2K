//! Remaining `FUN_00411AD0` visit after factory `FUN_00425850` returns `0xA300`.
//!
//! Retail dispatches that tagged object immediately, normalizes it to null,
//! and clears only the physical/damage suffix. Candidate behavior still runs,
//! then subject component slots 0--2, then candidate component slots 0--2.
//!
//! A receipt-backed Working Factory / Go-To-Job scientist pair closes that
//! remainder without inventing physical response:
//!
//! - class-54 style `+0x18` is null, so candidate behavior is skipped;
//! - type 66 has constructor-owned null component callbacks;
//! - `FUN_00403650` writes component-slot-0 pair callback `FUN_00402DA0`,
//!   and the constructor already cleared slots 1 and 2;
//! - the scientist's authored Sub-I topology takes the zero-RNG heading
//!   `+0x2000` / retained-direction branch.
//!
//! The detached delivery machine still reports
//! [`crate::factory_delivery::FactoryDeliveryPairSuffix::Unclaimed`]. This
//! module is the live coordinator that claims the remaining visit.

use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::actor_task_owner::ActorTaskSlot;
use crate::common_mover::target_prelude::{
    CommonMoverPreludeSubA, CommonMoverPreludeSubD, CommonMoverTargetPreludeTopology,
};
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::common_mover::SubAPropulsionRuntime;
use crate::descriptor_contact::{
    plan_descriptor_contact, DescriptorContactBlock, DescriptorContactMiss,
    DescriptorContactOutcome, DescriptorContactRequest, DescriptorContactSourceSnapshot,
    DescriptorContactTargetSnapshot,
};
use crate::entity::{Entity, EntityManager};
use crate::entity_behavior::PairContactCallbackPolicy;
use crate::entity_collision_state::{
    PairComponentContactPolicy, PairOrientationPolicy, RetailRuntimeValue,
};
use crate::wander_near_location::WanderNearPrivateState;

const LEVEL_ONE_FACTORY_ENTITY_TYPE: u32 = 66;
const FACTORY_COMPONENT_SLOTS: [PairComponentContactPolicy; 3] = [
    PairComponentContactPolicy::None,
    PairComponentContactPolicy::None,
    PairComponentContactPolicy::None,
];
const SCIENTIST_COMPONENT_SLOTS: [PairComponentContactPolicy; 3] = [
    PairComponentContactPolicy::DescriptorContact,
    PairComponentContactPolicy::None,
    PairComponentContactPolicy::None,
];

/// The two native worker graphs retain separate constructor receipts. Only the
/// older Type8 fixture may reach this callback without a native receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FactoryScientistRuntime {
    CapturedType8,
    TwoChoice(crate::intro2_type8::Intro2Type8Runtime),
    DiverWorker(crate::native_type86::NativeType86Runtime),
}

impl FactoryScientistRuntime {
    pub(crate) fn from_entity(entity: &Entity) -> Option<Self> {
        if entity.entity_type == 7 {
            return entity.native_type86_runtime.map(Self::DiverWorker);
        }
        if crate::intro2_type8::NativeWorkerProfile::from_entity_type(entity.entity_type).is_none()
        {
            return None;
        }
        match entity.intro2_type8_runtime {
            Some(runtime) => Some(Self::TwoChoice(runtime)),
            None if entity.entity_type == 8 => Some(Self::CapturedType8),
            None => None,
        }
    }
}

/// Why the remaining factory pair visit could not be closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryPairSuffixUnresolved {
    FactoryUnavailable { factory_id: u32 },
    ScientistUnavailable { scientist_id: u32 },
    FactoryComponentUnavailable { factory_id: u32 },
    FactoryComponentMismatch { factory_id: u32 },
    CandidateBehaviorUnavailable { scientist_id: u32 },
    CandidateBehaviorMismatch { scientist_id: u32 },
    CandidateComponentUnavailable { scientist_id: u32 },
    CandidateComponentMismatch { scientist_id: u32 },
    OrientationUnavailable { scientist_id: u32 },
    OrientationMismatch { scientist_id: u32 },
    ScientistMetadataUnavailable,
    SubAUnavailable { scientist_id: u32 },
    PrimaryTaskUnavailable { scientist_id: u32 },
    Descriptor(DescriptorContactBlock),
}

/// A preflighted plan became stale before the remaining visit committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryPairSuffixCommitError {
    FactoryChanged { factory_id: u32 },
    ScientistChanged { scientist_id: u32 },
}

/// Known-null class-54 pair callback. Unresolved or non-null styles fail
/// closed before this value exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryPairCandidateBehavior {
    SkippedNull,
}

/// Working Factory component slots are all null.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryPairSubjectComponents {
    None,
}

/// Scientist slot-0 `FUN_00402DA0` after the factory callback queued destroy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryPairCandidateComponents {
    Miss {
        forward_projection_raw: i32,
    },
    Applied {
        forward_projection_raw: i32,
        heading_raw_before: u16,
        heading_raw_after: u16,
        direction_multiplier_before: i32,
        direction_multiplier_after: i32,
    },
}

/// `0xA300` already cleared `FUN_00411AD0`'s physical/damage flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryPairPhysical {
    ClearedByA300,
}

/// Immutable preflight for the remaining factory/scientist pair visit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryPairSuffixPlan {
    factory_id: u32,
    scientist_id: u32,
    factory_allocation: crate::main_base_abort::MainBaseAbortActorLease,
    scientist_allocation: crate::main_base_abort::MainBaseAbortActorLease,
    expected_scientist_type: u32,
    expected_scientist_runtime: FactoryScientistRuntime,
    expected_factory_position_raw: [i16; 3],
    expected_scientist_primary: crate::actor_task_owner::ActorTaskId,
    expected_scientist_private: WanderNearPrivateState,
    expected_scientist_behavior:
        RetailRuntimeValue<Option<crate::entity_behavior::BehaviorContextRuntime>>,
    expected_scientist_position_raw: [i16; 3],
    expected_pose: ScientistPose,
    expected_sub_a: SubAPropulsionRuntime,
    resolved_heading_raw: u16,
    resolved_sub_a: SubAPropulsionRuntime,
    candidate_components: FactoryPairCandidateComponentPlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FactoryPairCandidateComponentPlan {
    Miss { forward_projection_raw: i32 },
    Apply { forward_projection_raw: i32 },
}

/// The retained F70 matrix can intentionally lag a subsequent heading write.
/// Journal it independently from the current angle words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScientistPose {
    rotation: [u16; 3],
    basis: Type9BodyBasis,
}

/// Auditable result of the remaining visit. Physical response is absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryPairSuffixOutcome {
    pub factory_id: u32,
    pub scientist_id: u32,
    pub candidate_behavior: FactoryPairCandidateBehavior,
    pub subject_components: FactoryPairSubjectComponents,
    pub candidate_components: FactoryPairCandidateComponents,
    pub physical: FactoryPairPhysical,
}

/// Close candidate behavior, both component chains, and the A300 physical skip
/// before the delivery journal mutates either allocation.
pub fn plan_factory_pair_suffix(
    entities: &EntityManager,
    factory_id: u32,
    scientist_id: u32,
) -> Result<FactoryPairSuffixPlan, FactoryPairSuffixUnresolved> {
    crate::factory_activation_live::validate_factory_delivery_owner(entities, factory_id)
        .map_err(|_| FactoryPairSuffixUnresolved::FactoryUnavailable { factory_id })?;
    let factory = entities
        .iter_all()
        .find(|entity| entity.id == factory_id)
        .ok_or(FactoryPairSuffixUnresolved::FactoryUnavailable { factory_id })?;
    let scientist = entities
        .iter_all()
        .find(|entity| entity.id == scientist_id)
        .ok_or(FactoryPairSuffixUnresolved::ScientistUnavailable { scientist_id })?;
    if !scientist_allocation_authenticates(entities, scientist_id) {
        return Err(FactoryPairSuffixUnresolved::ScientistUnavailable { scientist_id });
    }
    validate_factory_subject_components(factory, factory_id)?;
    validate_candidate_behavior_callback(scientist_id, scientist.current_behavior_context)?;
    validate_scientist_component_slots(scientist, scientist_id)?;
    let pose = scientist_pose(scientist, scientist_id)?;
    let [heading_raw, _, roll_raw] = pose.rotation;

    let private_state = go_to_job_private_state(scientist, scientist_id)?;
    let sub_a = match scientist.sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(runtime)) => runtime,
        RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => {
            return Err(FactoryPairSuffixUnresolved::SubAUnavailable { scientist_id });
        }
    };
    let topology = scientist_descriptor_topology(entities, scientist.entity_type, sub_a)?;
    let source = DescriptorContactSourceSnapshot {
        entity_id: scientist.id,
        position_raw: scientist.position_raw(),
        forward_q31: pose.basis.forward,
        heading_raw,
        roll_raw,
    };
    let outcome = plan_descriptor_contact(
        DescriptorContactRequest {
            source: RetailRuntimeValue::Known(source),
            target: RetailRuntimeValue::Known(DescriptorContactTargetSnapshot {
                entity_id: factory.id,
                position_raw: factory.position_raw(),
            }),
            private_state: RetailRuntimeValue::Known(private_state),
            topology: RetailRuntimeValue::Known(topology),
        },
        || unreachable!("the authenticated worker Sub-I effect consumes no RNG"),
    )
    .map_err(FactoryPairSuffixUnresolved::Descriptor)?;

    let (candidate_components, resolved_heading_raw, resolved_sub_a) = match outcome {
        DescriptorContactOutcome::Miss(DescriptorContactMiss {
            forward_projection_raw,
            ..
        }) => (
            FactoryPairCandidateComponentPlan::Miss {
                forward_projection_raw,
            },
            heading_raw,
            sub_a,
        ),
        DescriptorContactOutcome::Apply(plan) => {
            debug_assert_eq!(plan.effect.rng_draw_count, 0);
            debug_assert_eq!(plan.effect.target_state, private_state);
            debug_assert_eq!(plan.effect.roll_raw, roll_raw);
            debug_assert!(plan.effect.sub_d_reversal_write.is_none());
            debug_assert_eq!(plan.effect.sub_f_reverse_write, None);
            debug_assert_eq!(plan.effect.sub_g_reverse_write, None);
            let resolved_sub_a = plan.effect.sub_a_runtime.unwrap_or(sub_a);
            (
                FactoryPairCandidateComponentPlan::Apply {
                    forward_projection_raw: plan.forward_projection_raw,
                },
                plan.effect.heading_raw,
                resolved_sub_a,
            )
        }
    };

    Ok(FactoryPairSuffixPlan {
        factory_id,
        scientist_id,
        factory_allocation: entities
            .main_base_abort_actor_observation(factory_id)
            .ok_or(FactoryPairSuffixUnresolved::FactoryUnavailable { factory_id })?
            .lease,
        scientist_allocation: entities
            .main_base_abort_actor_observation(scientist_id)
            .ok_or(FactoryPairSuffixUnresolved::ScientistUnavailable { scientist_id })?
            .lease,
        expected_scientist_type: scientist.entity_type,
        expected_scientist_runtime: FactoryScientistRuntime::from_entity(scientist)
            .expect("scientist allocation authenticated"),
        expected_factory_position_raw: factory.position_raw(),
        expected_scientist_primary: scientist
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .ok_or(FactoryPairSuffixUnresolved::PrimaryTaskUnavailable { scientist_id })?,
        expected_scientist_private: private_state,
        expected_scientist_behavior: scientist.current_behavior_context,
        expected_scientist_position_raw: scientist.position_raw(),
        expected_pose: pose,
        expected_sub_a: sub_a,
        resolved_heading_raw,
        resolved_sub_a,
        candidate_components,
    })
}

/// Commit the remaining visit after deferred scientist destruction is queued.
///
/// Destroy-pending flags are expected to change. Position, heading, Sub-A,
/// and pair-callback identity must still match the preflight.
pub fn apply_factory_pair_suffix(
    entities: &mut EntityManager,
    plan: FactoryPairSuffixPlan,
) -> Result<FactoryPairSuffixOutcome, FactoryPairSuffixCommitError> {
    validate_factory_pair_suffix_plan(entities, &plan)?;
    let scientist = entities
        .entity_mut(plan.scientist_id)
        .expect("validated worker");
    let candidate_components = match plan.candidate_components {
        FactoryPairCandidateComponentPlan::Miss {
            forward_projection_raw,
        } => FactoryPairCandidateComponents::Miss {
            forward_projection_raw,
        },
        FactoryPairCandidateComponentPlan::Apply {
            forward_projection_raw,
        } => {
            scientist.set_heading_raw(plan.resolved_heading_raw);
            scientist.sub_a_propulsion_runtime =
                RetailRuntimeValue::Known(Some(plan.resolved_sub_a));
            FactoryPairCandidateComponents::Applied {
                forward_projection_raw,
                heading_raw_before: plan.expected_pose.rotation[0],
                heading_raw_after: plan.resolved_heading_raw,
                direction_multiplier_before: plan.expected_sub_a.direction_multiplier(),
                direction_multiplier_after: plan.resolved_sub_a.direction_multiplier(),
            }
        }
    };
    Ok(FactoryPairSuffixOutcome {
        factory_id: plan.factory_id,
        scientist_id: plan.scientist_id,
        candidate_behavior: FactoryPairCandidateBehavior::SkippedNull,
        subject_components: FactoryPairSubjectComponents::None,
        candidate_components,
        physical: FactoryPairPhysical::ClearedByA300,
    })
}

/// Revalidate before delivery emits operation33 or changes staffing, as well
/// as immediately before the component suffix after deferred destruction.
pub(crate) fn validate_factory_pair_suffix_plan(
    entities: &EntityManager,
    plan: &FactoryPairSuffixPlan,
) -> Result<(), FactoryPairSuffixCommitError> {
    if entities
        .main_base_abort_actor_observation(plan.factory_id)
        .is_none_or(|observed| observed.lease != plan.factory_allocation)
        || crate::factory_activation_live::validate_factory_delivery_owner(
            entities,
            plan.factory_id,
        )
        .is_err()
    {
        return Err(FactoryPairSuffixCommitError::FactoryChanged {
            factory_id: plan.factory_id,
        });
    }
    let factory_unchanged = entities
        .iter_all()
        .find(|entity| entity.id == plan.factory_id)
        .is_some_and(|factory| {
            factory.entity_type == LEVEL_ONE_FACTORY_ENTITY_TYPE
                && factory.position_raw() == plan.expected_factory_position_raw
                && factory.collision.pair_callbacks.component_contact
                    == RetailRuntimeValue::Known(FACTORY_COMPONENT_SLOTS)
        });
    if !factory_unchanged {
        return Err(FactoryPairSuffixCommitError::FactoryChanged {
            factory_id: plan.factory_id,
        });
    }

    if entities
        .main_base_abort_actor_observation(plan.scientist_id)
        .is_none_or(|observed| observed.lease != plan.scientist_allocation)
        || !scientist_allocation_authenticates(entities, plan.scientist_id)
    {
        return Err(FactoryPairSuffixCommitError::ScientistChanged {
            scientist_id: plan.scientist_id,
        });
    }
    let scientist = entities
        .iter_all()
        .find(|entity| entity.id == plan.scientist_id)
        .ok_or(FactoryPairSuffixCommitError::ScientistChanged {
            scientist_id: plan.scientist_id,
        })?;
    if scientist.entity_type != plan.expected_scientist_type
        || FactoryScientistRuntime::from_entity(scientist) != Some(plan.expected_scientist_runtime)
        || scientist.actor_tasks.task_in_slot(ActorTaskSlot::Primary)
            != Some(plan.expected_scientist_primary)
        || go_to_job_private_state(scientist, plan.scientist_id)
            != Ok(plan.expected_scientist_private)
        || scientist.current_behavior_context != plan.expected_scientist_behavior
        || scientist_pose(scientist, plan.scientist_id) != Ok(plan.expected_pose)
        || scientist.position_raw() != plan.expected_scientist_position_raw
        || scientist.sub_a_propulsion_runtime
            != RetailRuntimeValue::Known(Some(plan.expected_sub_a))
        || scientist.collision.pair_callbacks.component_contact
            != RetailRuntimeValue::Known(SCIENTIST_COMPONENT_SLOTS)
    {
        return Err(FactoryPairSuffixCommitError::ScientistChanged {
            scientist_id: plan.scientist_id,
        });
    }

    Ok(())
}

fn validate_factory_subject_components(
    factory: &Entity,
    factory_id: u32,
) -> Result<(), FactoryPairSuffixUnresolved> {
    match factory.collision.pair_callbacks.component_contact {
        RetailRuntimeValue::Known(actual) if actual == FACTORY_COMPONENT_SLOTS => Ok(()),
        RetailRuntimeValue::Known(_) => {
            Err(FactoryPairSuffixUnresolved::FactoryComponentMismatch { factory_id })
        }
        RetailRuntimeValue::Unresolved => {
            Err(FactoryPairSuffixUnresolved::FactoryComponentUnavailable { factory_id })
        }
    }
}

fn validate_candidate_behavior_callback(
    entity_id: u32,
    behavior: RetailRuntimeValue<Option<crate::entity_behavior::BehaviorContextRuntime>>,
) -> Result<(), FactoryPairSuffixUnresolved> {
    match behavior {
        RetailRuntimeValue::Known(None) => Ok(()),
        RetailRuntimeValue::Known(Some(context))
            if context.active_style().pair_contact_callback_policy()
                == PairContactCallbackPolicy::None =>
        {
            Ok(())
        }
        RetailRuntimeValue::Known(Some(_)) => {
            Err(FactoryPairSuffixUnresolved::CandidateBehaviorMismatch {
                scientist_id: entity_id,
            })
        }
        RetailRuntimeValue::Unresolved => {
            Err(FactoryPairSuffixUnresolved::CandidateBehaviorUnavailable {
                scientist_id: entity_id,
            })
        }
    }
}

fn validate_scientist_component_slots(
    scientist: &Entity,
    scientist_id: u32,
) -> Result<(), FactoryPairSuffixUnresolved> {
    match scientist.collision.pair_callbacks.component_contact {
        RetailRuntimeValue::Known(actual) if actual == SCIENTIST_COMPONENT_SLOTS => Ok(()),
        RetailRuntimeValue::Known(_) => {
            Err(FactoryPairSuffixUnresolved::CandidateComponentMismatch { scientist_id })
        }
        RetailRuntimeValue::Unresolved => {
            Err(FactoryPairSuffixUnresolved::CandidateComponentUnavailable { scientist_id })
        }
    }
}

fn scientist_pose(
    scientist: &Entity,
    scientist_id: u32,
) -> Result<ScientistPose, FactoryPairSuffixUnresolved> {
    if scientist.entity_type == 7
        || scientist
            .intro2_type8_runtime
            .is_some_and(|runtime| runtime.is_native_construction())
    {
        let RetailRuntimeValue::Known(basis) = scientist.physical_body_basis_q31() else {
            return Err(FactoryPairSuffixUnresolved::OrientationUnavailable { scientist_id });
        };
        return Ok(ScientistPose {
            rotation: scientist
                .rotation_heading_pitch_roll_raw()
                .map(|angle| angle as u16),
            basis,
        });
    }
    // Captured/legacy Type8 adapters still require their authored pose proof.
    let (heading, pitch, roll) = validate_scientist_orientation(scientist, scientist_id)?;
    Ok(ScientistPose {
        rotation: [heading, pitch, roll],
        basis: Type9BodyBasis::from_angle_words(heading as i16, pitch as i16, roll as i16),
    })
}

fn validate_scientist_orientation(
    scientist: &Entity,
    scientist_id: u32,
) -> Result<(u16, u16, u16), FactoryPairSuffixUnresolved> {
    let PairOrientationPolicy::LiveHeadingWithAuthoredPitchRoll {
        pitch_raw,
        roll_raw,
    } = match scientist.collision.pair_callbacks.orientation_policy {
        RetailRuntimeValue::Known(policy) => policy,
        RetailRuntimeValue::Unresolved => {
            return Err(FactoryPairSuffixUnresolved::OrientationUnavailable { scientist_id });
        }
    };
    let rotation = scientist.rotation_heading_pitch_roll_raw();
    let actual_pitch_roll_raw = [rotation[1] as u16, rotation[2] as u16];
    if actual_pitch_roll_raw != [pitch_raw, roll_raw] {
        return Err(FactoryPairSuffixUnresolved::OrientationMismatch { scientist_id });
    }
    Ok((scientist.heading_raw(), pitch_raw, roll_raw))
}

fn go_to_job_private_state(
    scientist: &Entity,
    scientist_id: u32,
) -> Result<WanderNearPrivateState, FactoryPairSuffixUnresolved> {
    match scientist.actor_task_state(ActorTaskSlot::Primary) {
        Some(ActorTaskRuntime::GoToJob(state)) => Ok(state.private_state()),
        _ => Err(FactoryPairSuffixUnresolved::PrimaryTaskUnavailable { scientist_id }),
    }
}

fn scientist_descriptor_topology(
    entities: &EntityManager,
    entity_type: u32,
    sub_a_runtime: SubAPropulsionRuntime,
) -> Result<CommonMoverTargetPreludeTopology, FactoryPairSuffixUnresolved> {
    let metadata = entities
        .type_runtime_metadata(entity_type)
        .ok_or(FactoryPairSuffixUnresolved::ScientistMetadataUnavailable)?;
    if !scientist_metadata_authenticates(entity_type, metadata) {
        return Err(FactoryPairSuffixUnresolved::ScientistMetadataUnavailable);
    }
    let sub_a_descriptor = match metadata.sub_a_propulsion_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
        _ => return Err(FactoryPairSuffixUnresolved::ScientistMetadataUnavailable),
    };
    let sub_d_descriptor = match metadata.sub_d_steering_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
        _ => return Err(FactoryPairSuffixUnresolved::ScientistMetadataUnavailable),
    };
    Ok(CommonMoverTargetPreludeTopology {
        sub_a: Some(CommonMoverPreludeSubA {
            descriptor: Some(sub_a_descriptor),
            runtime: sub_a_runtime,
        }),
        sub_d: Some(CommonMoverPreludeSubD {
            steering_divisor_raw: sub_d_descriptor.steering_divisor_raw,
            couple_yaw_into_roll: sub_d_descriptor.couple_yaw_into_roll_raw != 0,
        }),
        sub_f: false,
        sub_g: false,
        sub_i: true,
        sub_l: false,
    })
}

/// Native worker profiles share 25850/03650/02DA0, but retain
/// different model, Sub-D and Sub-I records. A copied receipt cannot authorize
/// another manager. The retained Type8 conversion fixture keeps its existing
/// separate construction path; later families require their native receipt.
pub(crate) fn scientist_allocation_authenticates(entities: &EntityManager, id: u32) -> bool {
    let Some(scientist) = entities.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    let Some(metadata) = entities.type_runtime_metadata(scientist.entity_type) else {
        return false;
    };
    if !scientist_metadata_authenticates(scientist.entity_type, metadata) {
        return false;
    }
    if scientist.entity_type == 7 {
        return crate::native_type86::native_type86_manager_allocation_authenticates(entities, id)
            && crate::native_type86::Type86Owner::adopt_published(scientist).is_some();
    }
    if scientist.intro2_type8_runtime.is_some() {
        crate::intro2_type8::intro2_type8_manager_allocation_authenticates(entities, id)
            && crate::intro2_type8::Intro2Type8Owner::adopt_published(scientist).is_some()
    } else {
        scientist.active && scientist.entity_type == 8 && scientist.model_slots == [Some(559); 4]
    }
}

fn scientist_metadata_authenticates(
    entity_type: u32,
    metadata: &crate::entity_collision_state::EntityTypeRuntimeMetadata,
) -> bool {
    if entity_type == 7 {
        crate::native_type86::validate_four_choice_metadata(entity_type, metadata).is_ok()
    } else {
        crate::intro2_type8::validate_worker_metadata(entity_type, metadata).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_behavior::{
        audited_behavior_style, behavior_program, BehaviorContextRuntime, BehaviorSelection,
        PairContactCallbackPolicy,
    };

    const SCIENTIST_ID: u32 = 0x04c2_0001;

    #[test]
    fn captured_go_to_job_style_and_known_absence_skip_candidate_behavior() {
        let go_to_job = behavior_program(54).expect("captured class-54 program");
        assert_eq!(go_to_job.initial_style.frame_address, 0x004c_8788);
        assert_eq!(
            go_to_job.initial_style.pair_contact_callback_policy(),
            PairContactCallbackPolicy::None
        );
        assert_eq!(
            validate_candidate_behavior_callback(
                SCIENTIST_ID,
                RetailRuntimeValue::Known(Some(
                    BehaviorContextRuntime::from_published_weighted_selection(BehaviorSelection {
                        choice_index: 0,
                        program: go_to_job,
                    },)
                    .expect("Go To Job is a weighted catalog behavior"),
                )),
            ),
            Ok(())
        );
        assert_eq!(
            validate_candidate_behavior_callback(SCIENTIST_ID, RetailRuntimeValue::Known(None)),
            Ok(())
        );
    }

    #[test]
    fn unresolved_or_non_null_candidate_behavior_fails_closed() {
        assert_eq!(
            validate_candidate_behavior_callback(SCIENTIST_ID, RetailRuntimeValue::Unresolved),
            Err(FactoryPairSuffixUnresolved::CandidateBehaviorUnavailable {
                scientist_id: SCIENTIST_ID
            })
        );

        let non_null = BehaviorContextRuntime::named_audited(
            behavior_program(9).expect("Capture People behavior"),
            1,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
            *audited_behavior_style(9, 1).expect("Capture People variant 1"),
        )
        .expect("Capture People variant 1 is audited");
        assert_eq!(
            validate_candidate_behavior_callback(
                SCIENTIST_ID,
                RetailRuntimeValue::Known(Some(non_null)),
            ),
            Err(FactoryPairSuffixUnresolved::CandidateBehaviorMismatch {
                scientist_id: SCIENTIST_ID
            })
        );
    }
}
