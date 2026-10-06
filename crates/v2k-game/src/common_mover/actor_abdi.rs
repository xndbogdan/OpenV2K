//! Shared actor D → I → A → B common-mover kernel.
//!
//! Normal-detail `1X3XX.OVL` proves that cumulative type 8 (scientist) and
//! cumulative type 9 (peasant) author the same common-mover component route:
//! Sub-D steering, Sub-I animation, Sub-A propulsion, then Sub-B lateral
//! damping. Their Sub-I sound words differ, so admission retains each record's
//! exact descriptors instead of treating the two entity types as aliases.
//! Ordinary Type116 (`vulc2`) shares the scientist's A/B/D/I topology with its
//! own model and attention-stop cue, so it joins this kernel as a distinct
//! identity rather than borrowing the Type8 receipt.
//! Workers79/90/91 and people78/86/95/123 select the same component order with
//! the D32 descriptor. Each identity retains its own Section12 Sub-I record.
//! Type7's diver also selects D32, but its flags0 descriptor skips terrain
//! classification while retaining the allocation's Sub-D frame cadence.
//!
//! This is an atomic evidence oracle, not a live entity scheduler. Task-family
//! setup, lifetime/transition policy, the ordinary type-9 surface/attitude
//! suffix, and process-order-sensitive Sub-D allocation remain with their
//! existing owners.

use super::component_dispatch::{CommonMoverDispatchMode, CommonMoverDispatchPlan};
use super::post_dispatch::{
    CommonMoverPostDispatchBlock, CommonMoverPostDispatchPhase, CommonMoverPostDispatchPlan,
};
use super::sub_d::{
    apply_type9_sub_d, Type9SubDFrameOwner, Type9SubDRuntime, Type9SubDStep, ORDINARY_TYPE7_SUB_D,
    ORDINARY_TYPE90_SUB_D, ORDINARY_TYPE9_SUB_D,
};
use super::target_prelude::{
    plan_common_mover_target_prelude, CommonMoverPreludeSubA, CommonMoverPreludeSubD,
    CommonMoverTargetPreludeBlock, CommonMoverTargetPreludeOutcome,
    CommonMoverTargetPreludeRequest, CommonMoverTargetPreludeTopology,
    CommonMoverTargetPreludeZero, CommonMoverTrackedTargetSnapshot,
};
use super::{apply_sub_a_propulsion_raw, apply_sub_b_lateral_raw, SubAPropulsionRuntime};
use crate::actor_animation::ActorAnimationController;
use crate::entity_collision_state::{
    CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
};
use crate::wander_near_location::WanderNearPrivateState;
use v2k_formats::collision::{
    ActorAnimationDescriptor, SubAPropulsionDescriptor, SubBLateralDescriptor,
    SubDSteeringDescriptor,
};
use v2k_formats::terrain::TerrainGrid;

/// Cumulative first-world scientist entity type.
pub const FIRST_WORLD_SCIENTIST_ENTITY_TYPE: u16 = 8;
/// Cumulative first-world peasant entity type.
pub const FIRST_WORLD_PEASANT_ENTITY_TYPE: u16 = 9;
/// Ordinary diver worker entity type (four-choice Class6/54/45/10 graph).
pub const ORDINARY_TYPE7_ENTITY_TYPE: u16 = 7;
/// Ordinary vulcan worker entity type (worker-equivalent to Type8).
pub const ORDINARY_VULCAN_WORKER_ENTITY_TYPE: u16 = 116;
/// Ordinary desert worker entity type (Type90, GoToJob/Wander with own sounds).
pub const ORDINARY_TYPE90_ENTITY_TYPE: u16 = 90;
/// Ordinary Type123 person entity type (Attract45/Wander with own sounds).
pub const ORDINARY_TYPE123_ENTITY_TYPE: u16 = 123;
/// Ordinary Type86 person entity type (Attract45/RunAway10/GoToJob54/Wander).
pub const ORDINARY_TYPE86_ENTITY_TYPE: u16 = 86;

/// Authored entity identities admitted by the shared actor kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorAbdiEntityKind {
    Scientist,
    Peasant,
    Type7Worker,
    VulcanWorker,
    Type90Worker,
    Type79Worker,
    Type91Worker,
    Type123Person,
    Type86Person,
    Type78Person,
    Type95Person,
}

impl ActorAbdiEntityKind {
    pub const fn from_entity_type(entity_type: u16) -> Option<Self> {
        match entity_type {
            FIRST_WORLD_SCIENTIST_ENTITY_TYPE => Some(Self::Scientist),
            FIRST_WORLD_PEASANT_ENTITY_TYPE => Some(Self::Peasant),
            ORDINARY_TYPE7_ENTITY_TYPE => Some(Self::Type7Worker),
            ORDINARY_VULCAN_WORKER_ENTITY_TYPE => Some(Self::VulcanWorker),
            ORDINARY_TYPE90_ENTITY_TYPE => Some(Self::Type90Worker),
            79 => Some(Self::Type79Worker),
            91 => Some(Self::Type91Worker),
            ORDINARY_TYPE123_ENTITY_TYPE => Some(Self::Type123Person),
            ORDINARY_TYPE86_ENTITY_TYPE => Some(Self::Type86Person),
            78 => Some(Self::Type78Person),
            95 => Some(Self::Type95Person),
            _ => None,
        }
    }

    pub const fn entity_type(self) -> u16 {
        match self {
            Self::Scientist => FIRST_WORLD_SCIENTIST_ENTITY_TYPE,
            Self::Peasant => FIRST_WORLD_PEASANT_ENTITY_TYPE,
            Self::Type7Worker => ORDINARY_TYPE7_ENTITY_TYPE,
            Self::VulcanWorker => ORDINARY_VULCAN_WORKER_ENTITY_TYPE,
            Self::Type90Worker => ORDINARY_TYPE90_ENTITY_TYPE,
            Self::Type79Worker => 79,
            Self::Type91Worker => 91,
            Self::Type123Person => ORDINARY_TYPE123_ENTITY_TYPE,
            Self::Type86Person => ORDINARY_TYPE86_ENTITY_TYPE,
            Self::Type78Person => 78,
            Self::Type95Person => 95,
        }
    }
}

/// Ordered component phases proven for both admitted entity records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorAbdiPhase {
    SubD,
    SubI,
    SubA,
    SubB,
}

pub const ACTOR_ABDI_PHASE_ORDER: [ActorAbdiPhase; 4] = [
    ActorAbdiPhase::SubD,
    ActorAbdiPhase::SubI,
    ActorAbdiPhase::SubA,
    ActorAbdiPhase::SubB,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorAbdiRequiredComponent {
    SubA,
    SubB,
    SubD,
    SubI,
}

/// Why exact Section-12 metadata cannot authorize the shared actor route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorAbdiTopologyError {
    UnsupportedEntityType { actual: u16 },
    UnresolvedComponentTopology,
    UnsupportedComponentTopology,
    UnresolvedDescriptor(ActorAbdiRequiredComponent),
    MissingRequiredComponent(ActorAbdiRequiredComponent),
    UnsupportedSubDDescriptor,
}

/// Section-12 proof that an admitted actor selects D → I → A → B.
///
/// Construction is available only through [`Self::from_metadata`]. Supplying
/// four convenient descriptors is insufficient because F/H/G/K/L can pre-empt
/// or supplement Sub-I, while C/G can alter the post-dispatch tail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorAbdiTopology {
    entity_kind: ActorAbdiEntityKind,
    component_topology: CommonMoverComponentTopology,
    sub_a: SubAPropulsionDescriptor,
    sub_b: SubBLateralDescriptor,
    sub_d: SubDSteeringDescriptor,
    actor_animation: ActorAnimationDescriptor,
}

impl ActorAbdiTopology {
    pub fn from_metadata(
        entity_type: u16,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<Self, ActorAbdiTopologyError> {
        let entity_kind = ActorAbdiEntityKind::from_entity_type(entity_type).ok_or(
            ActorAbdiTopologyError::UnsupportedEntityType {
                actual: entity_type,
            },
        )?;
        let component_topology = match metadata.common_mover_topology {
            RetailRuntimeValue::Known(value) => value,
            RetailRuntimeValue::Unresolved => {
                return Err(ActorAbdiTopologyError::UnresolvedComponentTopology);
            }
        };
        if !component_topology.sub_a
            || !component_topology.sub_b
            || component_topology.sub_c
            || !component_topology.sub_d
            || component_topology.sub_g
            || !CommonMoverDispatchPlan::from_topology(
                component_topology,
                CommonMoverDispatchMode::Normal,
            )
            .is_sub_i_only()
        {
            return Err(ActorAbdiTopologyError::UnsupportedComponentTopology);
        }

        let sub_a = required_descriptor(
            metadata.sub_a_propulsion_descriptor,
            ActorAbdiRequiredComponent::SubA,
        )?;
        let sub_b = required_descriptor(
            metadata.sub_b_lateral_descriptor,
            ActorAbdiRequiredComponent::SubB,
        )?;
        let sub_d = required_descriptor(
            metadata.sub_d_steering_descriptor,
            ActorAbdiRequiredComponent::SubD,
        )?;
        let expected_sub_d = match entity_kind {
            ActorAbdiEntityKind::Type7Worker => ORDINARY_TYPE7_SUB_D,
            ActorAbdiEntityKind::Type90Worker
            | ActorAbdiEntityKind::Type79Worker
            | ActorAbdiEntityKind::Type91Worker
            | ActorAbdiEntityKind::Type123Person
            | ActorAbdiEntityKind::Type86Person
            | ActorAbdiEntityKind::Type78Person
            | ActorAbdiEntityKind::Type95Person => ORDINARY_TYPE90_SUB_D,
            ActorAbdiEntityKind::Scientist
            | ActorAbdiEntityKind::Peasant
            | ActorAbdiEntityKind::VulcanWorker => ORDINARY_TYPE9_SUB_D,
        };
        if sub_d != expected_sub_d {
            return Err(ActorAbdiTopologyError::UnsupportedSubDDescriptor);
        }
        let actor_animation = required_descriptor(
            metadata.actor_animation_descriptor,
            ActorAbdiRequiredComponent::SubI,
        )?;

        Ok(Self {
            entity_kind,
            component_topology,
            sub_a,
            sub_b,
            sub_d,
            actor_animation,
        })
    }

    pub const fn entity_kind(self) -> ActorAbdiEntityKind {
        self.entity_kind
    }

    pub const fn component_topology(self) -> CommonMoverComponentTopology {
        self.component_topology
    }

    pub const fn actor_animation_descriptor(self) -> ActorAnimationDescriptor {
        self.actor_animation
    }

    pub const fn sub_d_descriptor(self) -> SubDSteeringDescriptor {
        self.sub_d
    }

    pub const fn phase_order(self) -> [ActorAbdiPhase; 4] {
        ACTOR_ABDI_PHASE_ORDER
    }
}

fn required_descriptor<T: Copy>(
    value: RetailRuntimeValue<Option<T>>,
    component: ActorAbdiRequiredComponent,
) -> Result<T, ActorAbdiTopologyError> {
    match value {
        RetailRuntimeValue::Known(Some(value)) => Ok(value),
        RetailRuntimeValue::Known(None) => {
            Err(ActorAbdiTopologyError::MissingRequiredComponent(component))
        }
        RetailRuntimeValue::Unresolved => {
            Err(ActorAbdiTopologyError::UnresolvedDescriptor(component))
        }
    }
}

/// Per-allocation state changed by D and I during one mover frame.
///
/// The Sub-D owner names retain their original type-9 provenance. The type-8
/// and type-9 Section-12 records author the identical descriptor bytes, so the
/// shared kernel can use that recovered implementation without duplicating it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorAbdiFrameState {
    pub sub_d_frame_owner: Type9SubDFrameOwner,
    pub sub_d_runtime: Type9SubDRuntime,
    pub sub_a_runtime: SubAPropulsionRuntime,
    pub actor_animation: ActorAnimationController,
}

/// Inputs finalized by the task and entity owners before `FUN_00401430`.
#[derive(Debug)]
pub struct ActorAbdiFrameRequest<'a> {
    pub topology: ActorAbdiTopology,
    pub target_state: WanderNearPrivateState,
    /// Current lookup result for `target_state.tracked_entity_handle`.
    pub tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    pub terrain: &'a TerrainGrid,
    pub position_raw: [i16; 3],
    /// Entity basis read before Sub-D changes the heading word.
    pub pre_mover_right_q31: [i32; 3],
    /// Entity basis read before Sub-D changes the heading word.
    pub pre_mover_forward_q31: [i32; 3],
    pub heading_raw: u16,
    pub velocity_raw: [i16; 3],
    pub elapsed_micros: u32,
    /// Retail `DAT_004D04E4`, independently consumed by Sub-D yaw integration.
    pub global_elapsed_micros: u32,
    /// Retail dispatcher mode: zero selects Sub-I for this topology, while
    /// every nonzero value suppresses it. Sub-D and the Sub-A/B tail remain.
    pub scheduler_mode: i32,
}

/// Exact deterministic result of one accepted shared actor frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorAbdiFrameStep {
    pub target_state: WanderNearPrivateState,
    pub heading_raw: u16,
    pub velocity_raw: [i16; 3],
    pub yaw_step_raw: i16,
    pub actor_animation_selector: u16,
    pub propulsion_applied: bool,
}

/// Runtime outcome is distinct from an evidence block: retail intentionally
/// returns zero when a tracked target is inactive or dying.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorAbdiFrameOutcome {
    Applied(ActorAbdiFrameStep),
    ReturnedZero(CommonMoverTargetPreludeZero),
}

/// Explicit owner proof for the Sub-I call inside the shared D/I/A/B route.
///
/// Most ordinary actors use the neutral policy. The two bounded exceptions
/// own complete local state: class-14 Exploding Person proves special mode,
/// while selected Attract Attention proves forced stop with neither special
/// mode nor a linked target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActorAbdiAnimationPolicy {
    Neutral,
    ExplodingPersonSpecial,
    AttractAttentionForcedStop,
}

/// Fail-closed boundaries whose owners are intentionally outside this oracle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorAbdiFrameBlock {
    TargetPrelude(CommonMoverTargetPreludeBlock),
    UnresolvedSubATargetSpeed,
    ActorAnimationDescriptorMismatch,
    NonNeutralActorAnimationRequiresOwner,
    ExplodingPersonAnimationStateMismatch,
    AttractAttentionAnimationStateMismatch,
    SubD(Type9SubDStep),
}

/// Apply D → heading commit → optional I → A → B atomically.
///
/// A/B consume the basis passed in the request. Retail changes entity heading
/// during D but does not rebuild the Q31 basis before the later propulsion
/// phases. State is staged locally and committed only after every phase
/// succeeds.
pub fn advance_actor_abdi_common_mover(
    state: &mut ActorAbdiFrameState,
    request: ActorAbdiFrameRequest<'_>,
    next_random: impl FnMut() -> u32,
) -> Result<ActorAbdiFrameOutcome, ActorAbdiFrameBlock> {
    advance_actor_abdi_common_mover_with_animation_policy(
        state,
        request,
        ActorAbdiAnimationPolicy::Neutral,
        next_random,
    )
}

/// Apply the same atomic common-mover route under one explicit Sub-I owner.
///
/// This entry point does not weaken the neutral API. The special policy is
/// accepted only for class-14's constructor-proven state shape.
pub(crate) fn advance_actor_abdi_common_mover_with_animation_policy(
    state: &mut ActorAbdiFrameState,
    request: ActorAbdiFrameRequest<'_>,
    animation_policy: ActorAbdiAnimationPolicy,
    mut next_random: impl FnMut() -> u32,
) -> Result<ActorAbdiFrameOutcome, ActorAbdiFrameBlock> {
    let dispatch_plan = CommonMoverDispatchPlan::from_topology(
        request.topology.component_topology,
        CommonMoverDispatchMode::from_retail_word(request.scheduler_mode),
    );
    let dispatches_sub_i = dispatch_plan.is_sub_i_only();
    debug_assert!(
        dispatches_sub_i || dispatch_plan.phases() == [None, None, None],
        "the admitted D/I/A/B topology has no restricted-mode F/G callback"
    );
    CommonMoverPostDispatchPlan::from_topology(
        request.topology.component_topology,
        state.sub_a_runtime.target_speed_raw(),
    )
    .map_err(|block| match block {
        CommonMoverPostDispatchBlock::UnresolvedSubATargetSpeed => {
            ActorAbdiFrameBlock::UnresolvedSubATargetSpeed
        }
    })?;
    if dispatches_sub_i {
        if state.actor_animation.descriptor() != request.topology.actor_animation {
            return Err(ActorAbdiFrameBlock::ActorAnimationDescriptorMismatch);
        }
        match animation_policy {
            ActorAbdiAnimationPolicy::Neutral if !state.actor_animation.is_neutral_runtime() => {
                return Err(ActorAbdiFrameBlock::NonNeutralActorAnimationRequiresOwner);
            }
            ActorAbdiAnimationPolicy::ExplodingPersonSpecial
                if !state.actor_animation.special_mode()
                    || state.actor_animation.linked_handle().is_some() =>
            {
                return Err(ActorAbdiFrameBlock::ExplodingPersonAnimationStateMismatch);
            }
            ActorAbdiAnimationPolicy::AttractAttentionForcedStop
                if state.actor_animation.special_mode()
                    || !state.actor_animation.forced_stop()
                    || state.actor_animation.linked_handle().is_some() =>
            {
                return Err(ActorAbdiFrameBlock::AttractAttentionAnimationStateMismatch);
            }
            ActorAbdiAnimationPolicy::Neutral
            | ActorAbdiAnimationPolicy::ExplodingPersonSpecial
            | ActorAbdiAnimationPolicy::AttractAttentionForcedStop => {}
        }
    }
    if request.topology.sub_d_descriptor().classifier_flags != 0
        && !state.sub_d_frame_owner.classifier_cache().can_classify()
    {
        return Err(ActorAbdiFrameBlock::SubD(
            Type9SubDStep::UnresolvedClassifierCache,
        ));
    }

    let mut next = *state;
    let prelude = plan_common_mover_target_prelude(
        CommonMoverTargetPreludeRequest {
            target_state: request.target_state,
            tracked_target: request.tracked_target,
            controlled_position_raw: request.position_raw,
            heading_raw: request.heading_raw,
            // Both exact actor records prove Sub-I, so the no-I Sub-D reversal
            // path cannot consume or change this placeholder.
            roll_raw: 0,
            elapsed_micros: request.elapsed_micros,
            topology: RetailRuntimeValue::Known(CommonMoverTargetPreludeTopology {
                sub_a: Some(CommonMoverPreludeSubA {
                    descriptor: Some(request.topology.sub_a),
                    runtime: next.sub_a_runtime,
                }),
                sub_d: Some(CommonMoverPreludeSubD {
                    steering_divisor_raw: request.topology.sub_d_descriptor().steering_divisor_raw,
                    couple_yaw_into_roll: request
                        .topology
                        .sub_d_descriptor()
                        .couple_yaw_into_roll_raw
                        != 0,
                }),
                sub_f: false,
                sub_g: false,
                sub_i: true,
                sub_l: false,
            }),
        },
        &mut next_random,
    )
    .map_err(ActorAbdiFrameBlock::TargetPrelude)?;
    let prelude = match prelude {
        CommonMoverTargetPreludeOutcome::Continue(plan) => plan,
        CommonMoverTargetPreludeOutcome::ReturnZero(reason) => {
            return Ok(ActorAbdiFrameOutcome::ReturnedZero(reason));
        }
    };
    debug_assert!(prelude.sub_d_reversal_write.is_none());
    debug_assert_eq!(prelude.sub_f_reverse_write, None);
    debug_assert_eq!(prelude.sub_g_reverse_write, None);
    debug_assert_eq!(prelude.sub_l_target_write, None);
    next.sub_a_runtime = prelude
        .sub_a_runtime
        .expect("shared actor topology proves Sub-A");
    let post_dispatch_plan = CommonMoverPostDispatchPlan::from_topology(
        request.topology.component_topology,
        next.sub_a_runtime.target_speed_raw(),
    )
    .expect("resolved actor target speed cannot become unresolved");
    let target_speed_raw = match next.sub_a_runtime.target_speed_raw() {
        RetailRuntimeValue::Known(value) => value,
        RetailRuntimeValue::Unresolved => {
            unreachable!("resolved actor target speed cannot become unresolved")
        }
    };
    let heading_raw = prelude.heading_raw;
    let target_state = prelude.target_state;
    let evidence = next.sub_d_frame_owner.evidence_for_frame_with_descriptor(
        request.topology.sub_d_descriptor(),
        request.terrain,
        request.position_raw,
        target_state.target_position_raw,
        request.pre_mover_right_q31,
        request.pre_mover_forward_q31,
        target_state.direction,
    );
    let yaw_step_raw = match apply_type9_sub_d(
        request.topology.sub_d_descriptor(),
        &mut next.sub_d_runtime,
        evidence,
        request.elapsed_micros,
        request.global_elapsed_micros,
    ) {
        Type9SubDStep::Applied { yaw_step_raw } => yaw_step_raw,
        blocked => return Err(ActorAbdiFrameBlock::SubD(blocked)),
    };

    let heading_raw = heading_raw.wrapping_sub(yaw_step_raw as u16);
    let mut forced_zero_velocity = false;
    if dispatches_sub_i {
        match animation_policy {
            ActorAbdiAnimationPolicy::Neutral => next
                .actor_animation
                .advance_neutral(request.elapsed_micros, heading_raw),
            ActorAbdiAnimationPolicy::ExplodingPersonSpecial => {
                let selection =
                    next.actor_animation
                        .advance(request.elapsed_micros, heading_raw, false);
                debug_assert!(
                    !selection.zero_velocity,
                    "special Sub-I mode wins before the preserved forced-stop byte"
                );
            }
            ActorAbdiAnimationPolicy::AttractAttentionForcedStop => {
                let selection =
                    next.actor_animation
                        .advance(request.elapsed_micros, heading_raw, false);
                debug_assert!(selection.zero_velocity);
                forced_zero_velocity = selection.zero_velocity;
            }
        }
    }

    let mut velocity_raw = if forced_zero_velocity {
        [0; 3]
    } else {
        request.velocity_raw
    };
    let mut propulsion_applied = false;
    for phase in post_dispatch_plan.phases().into_iter().flatten() {
        match phase {
            CommonMoverPostDispatchPhase::SubC => {
                unreachable!("shared actor topology proves authored Sub-C absence")
            }
            CommonMoverPostDispatchPhase::SubA => {
                propulsion_applied = apply_sub_a_propulsion_raw(
                    request.topology.sub_a,
                    target_speed_raw,
                    next.sub_a_runtime.direction_multiplier(),
                    next.sub_a_runtime.drive_scale_percent(),
                    request.pre_mover_forward_q31,
                    &mut velocity_raw,
                    request.elapsed_micros,
                );
            }
            CommonMoverPostDispatchPhase::SubB => apply_sub_b_lateral_raw(
                request.topology.sub_b,
                request.pre_mover_right_q31,
                &mut velocity_raw,
                request.elapsed_micros,
            ),
        }
    }

    let actor_animation_selector = next.actor_animation.output();
    *state = next;
    Ok(ActorAbdiFrameOutcome::Applied(ActorAbdiFrameStep {
        target_state,
        heading_raw,
        velocity_raw,
        yaw_step_raw,
        actor_animation_selector,
        propulsion_applied,
    }))
}
