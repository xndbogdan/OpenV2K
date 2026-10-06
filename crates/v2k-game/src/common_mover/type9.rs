//! Ordinary type-9 adapter around the shared first-world actor mover.
//!
//! Type 8 and type 9 share the exact D → I → A → B component route. This
//! module preserves the ordinary type-9 public vocabulary (`wander`) and its
//! strict entity-type admission while delegating the atomic mover phases to
//! [`super::actor_abdi`]. Type-9 task installation, expiry, surface/attitude
//! suffix, and owner semantics remain separate.

use super::actor_abdi::{
    advance_actor_abdi_common_mover_with_animation_policy, ActorAbdiAnimationPolicy,
    ActorAbdiFrameBlock, ActorAbdiFrameOutcome, ActorAbdiFrameRequest, ActorAbdiFrameState,
    ActorAbdiRequiredComponent, ActorAbdiTopology, ActorAbdiTopologyError,
    FIRST_WORLD_PEASANT_ENTITY_TYPE,
};
use super::sub_d::Type9SubDStep;
use super::target_prelude::{
    CommonMoverTargetPreludeBlock, CommonMoverTargetPreludeZero, CommonMoverTrackedTargetSnapshot,
};
use crate::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use crate::wander_near_location::WanderNearPrivateState;
use v2k_formats::terrain::TerrainGrid;

/// Cumulative first-world peasant entity type.
pub const ORDINARY_TYPE9_ENTITY_TYPE: u16 = FIRST_WORLD_PEASANT_ENTITY_TYPE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9RequiredComponent {
    SubA,
    SubB,
    SubD,
    SubI,
}

/// Why exact Section-12 metadata cannot authorize the ordinary type-9 route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9TopologyError {
    WrongEntityType { actual: u16 },
    UnresolvedComponentTopology,
    UnsupportedComponentTopology,
    UnresolvedDescriptor(OrdinaryType9RequiredComponent),
    MissingRequiredComponent(OrdinaryType9RequiredComponent),
    UnsupportedSubDDescriptor,
}

/// Private proof that Section 12 selects D → I → A → B for cumulative type 9.
///
/// Construction is intentionally available only through
/// [`Self::from_metadata`]. A caller cannot prove this route merely by
/// supplying four convenient descriptors while F/H/G/K/L remain unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9Topology {
    shared: ActorAbdiTopology,
}

impl OrdinaryType9Topology {
    pub fn from_metadata(
        entity_type: u16,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<Self, OrdinaryType9TopologyError> {
        if entity_type != ORDINARY_TYPE9_ENTITY_TYPE {
            return Err(OrdinaryType9TopologyError::WrongEntityType {
                actual: entity_type,
            });
        }

        ActorAbdiTopology::from_metadata(entity_type, metadata)
            .map(|shared| Self { shared })
            .map_err(map_topology_error)
    }
}

fn map_topology_error(error: ActorAbdiTopologyError) -> OrdinaryType9TopologyError {
    match error {
        ActorAbdiTopologyError::UnsupportedEntityType { actual } => {
            OrdinaryType9TopologyError::WrongEntityType { actual }
        }
        ActorAbdiTopologyError::UnresolvedComponentTopology => {
            OrdinaryType9TopologyError::UnresolvedComponentTopology
        }
        ActorAbdiTopologyError::UnsupportedComponentTopology => {
            OrdinaryType9TopologyError::UnsupportedComponentTopology
        }
        ActorAbdiTopologyError::UnresolvedDescriptor(component) => {
            OrdinaryType9TopologyError::UnresolvedDescriptor(map_required_component(component))
        }
        ActorAbdiTopologyError::MissingRequiredComponent(component) => {
            OrdinaryType9TopologyError::MissingRequiredComponent(map_required_component(component))
        }
        ActorAbdiTopologyError::UnsupportedSubDDescriptor => {
            OrdinaryType9TopologyError::UnsupportedSubDDescriptor
        }
    }
}

const fn map_required_component(
    component: ActorAbdiRequiredComponent,
) -> OrdinaryType9RequiredComponent {
    match component {
        ActorAbdiRequiredComponent::SubA => OrdinaryType9RequiredComponent::SubA,
        ActorAbdiRequiredComponent::SubB => OrdinaryType9RequiredComponent::SubB,
        ActorAbdiRequiredComponent::SubD => OrdinaryType9RequiredComponent::SubD,
        ActorAbdiRequiredComponent::SubI => OrdinaryType9RequiredComponent::SubI,
    }
}

/// Per-allocation state changed by D and I during one mover frame.
pub type OrdinaryType9FrameState = ActorAbdiFrameState;

/// Inputs already finalized by the task and entity owners before
/// `FUN_00401430`.
#[derive(Debug)]
pub struct OrdinaryType9FrameRequest<'a> {
    pub topology: OrdinaryType9Topology,
    pub wander: WanderNearPrivateState,
    /// Current lookup result for `wander.tracked_entity_handle`.
    ///
    /// Ordinary Wander uses the zero static-target sentinel, but retaining the
    /// explicit lookup lets this bounded adapter exercise the shared prelude
    /// without fabricating a target if a task owner supplies one later.
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
    /// Retail dispatcher mode: zero selects Sub-I; every nonzero value skips
    /// it while retaining Sub-D and the Sub-A/B tail.
    pub scheduler_mode: i32,
}

/// Exact deterministic result of one accepted common-mover frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9FrameStep {
    pub wander: WanderNearPrivateState,
    pub heading_raw: u16,
    pub velocity_raw: [i16; 3],
    pub yaw_step_raw: i16,
    pub actor_animation_selector: u16,
    pub propulsion_applied: bool,
}

/// Runtime outcome is distinct from an evidence block: retail intentionally
/// returns zero when a tracked target is inactive or dying.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9FrameOutcome {
    Applied(OrdinaryType9FrameStep),
    ReturnedZero(CommonMoverTargetPreludeZero),
}

/// Fail-closed boundaries whose owners are intentionally outside this oracle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9FrameBlock {
    TargetPrelude(CommonMoverTargetPreludeBlock),
    UnresolvedSubATargetSpeed,
    ActorAnimationDescriptorMismatch,
    NonNeutralActorAnimationRequiresOwner,
    ExplodingPersonAnimationStateMismatch,
    AttractAttentionAnimationStateMismatch,
    SubD(Type9SubDStep),
}

/// Apply D → heading commit → I → A → B atomically.
///
/// A/B deliberately consume the basis passed in the request. Retail changes
/// entity heading during D but does not rebuild the entity's Q31 basis before
/// the later propulsion phases. The state is staged locally and committed only
/// after every phase succeeds.
pub fn advance_ordinary_type9_common_mover(
    state: &mut OrdinaryType9FrameState,
    request: OrdinaryType9FrameRequest<'_>,
    next_random: impl FnMut() -> u32,
) -> Result<OrdinaryType9FrameOutcome, OrdinaryType9FrameBlock> {
    advance_ordinary_type9_common_mover_with_animation_policy(
        state,
        request,
        ActorAbdiAnimationPolicy::Neutral,
        next_random,
    )
}

/// Execute the ordinary Type-9 route with a separately authenticated Sub-I
/// owner policy. The public ordinary entry point remains neutral-only.
pub(crate) fn advance_ordinary_type9_common_mover_with_animation_policy(
    state: &mut OrdinaryType9FrameState,
    request: OrdinaryType9FrameRequest<'_>,
    animation_policy: ActorAbdiAnimationPolicy,
    next_random: impl FnMut() -> u32,
) -> Result<OrdinaryType9FrameOutcome, OrdinaryType9FrameBlock> {
    let outcome = advance_actor_abdi_common_mover_with_animation_policy(
        state,
        actor_abdi_request(request),
        animation_policy,
        next_random,
    )
    .map_err(map_frame_block)?;
    Ok(map_frame_outcome(outcome))
}

fn actor_abdi_request(request: OrdinaryType9FrameRequest<'_>) -> ActorAbdiFrameRequest<'_> {
    ActorAbdiFrameRequest {
        topology: request.topology.shared,
        target_state: request.wander,
        tracked_target: request.tracked_target,
        terrain: request.terrain,
        position_raw: request.position_raw,
        pre_mover_right_q31: request.pre_mover_right_q31,
        pre_mover_forward_q31: request.pre_mover_forward_q31,
        heading_raw: request.heading_raw,
        velocity_raw: request.velocity_raw,
        elapsed_micros: request.elapsed_micros,
        global_elapsed_micros: request.global_elapsed_micros,
        scheduler_mode: request.scheduler_mode,
    }
}

fn map_frame_outcome(outcome: ActorAbdiFrameOutcome) -> OrdinaryType9FrameOutcome {
    match outcome {
        ActorAbdiFrameOutcome::Applied(step) => {
            OrdinaryType9FrameOutcome::Applied(OrdinaryType9FrameStep {
                wander: step.target_state,
                heading_raw: step.heading_raw,
                velocity_raw: step.velocity_raw,
                yaw_step_raw: step.yaw_step_raw,
                actor_animation_selector: step.actor_animation_selector,
                propulsion_applied: step.propulsion_applied,
            })
        }
        ActorAbdiFrameOutcome::ReturnedZero(reason) => {
            OrdinaryType9FrameOutcome::ReturnedZero(reason)
        }
    }
}

fn map_frame_block(block: ActorAbdiFrameBlock) -> OrdinaryType9FrameBlock {
    match block {
        ActorAbdiFrameBlock::TargetPrelude(block) => OrdinaryType9FrameBlock::TargetPrelude(block),
        ActorAbdiFrameBlock::UnresolvedSubATargetSpeed => {
            OrdinaryType9FrameBlock::UnresolvedSubATargetSpeed
        }
        ActorAbdiFrameBlock::ActorAnimationDescriptorMismatch => {
            OrdinaryType9FrameBlock::ActorAnimationDescriptorMismatch
        }
        ActorAbdiFrameBlock::NonNeutralActorAnimationRequiresOwner => {
            OrdinaryType9FrameBlock::NonNeutralActorAnimationRequiresOwner
        }
        ActorAbdiFrameBlock::ExplodingPersonAnimationStateMismatch => {
            OrdinaryType9FrameBlock::ExplodingPersonAnimationStateMismatch
        }
        ActorAbdiFrameBlock::AttractAttentionAnimationStateMismatch => {
            OrdinaryType9FrameBlock::AttractAttentionAnimationStateMismatch
        }
        ActorAbdiFrameBlock::SubD(block) => OrdinaryType9FrameBlock::SubD(block),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_animation::ActorAnimationController;
    use crate::common_mover::post_dispatch::{
        CommonMoverPostDispatchPhase, CommonMoverPostDispatchPlan,
    };
    use crate::common_mover::sub_d::{
        FreshLevel1Type9ClassifierAdmission, Type9SubDFrameOwner, Type9SubDRuntime,
        ORDINARY_TYPE9_SUB_D,
    };
    use crate::common_mover::SubAPropulsionRuntime;
    use crate::entity_collision_state::CommonMoverComponentTopology;
    use v2k_formats::collision::{
        ActorAnimationDescriptor, SubAPropulsionDescriptor, SubBLateralDescriptor,
        SubDSteeringDescriptor,
    };
    use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

    const SUB_A: SubAPropulsionDescriptor = SubAPropulsionDescriptor {
        acceleration_raw: 1_500,
        overspeed_correction_raw: -3_000,
        target_speed_base_raw: 250,
    };
    const SUB_B: SubBLateralDescriptor = SubBLateralDescriptor {
        projection_threshold_rate_raw: 10_000,
        correction_rate_raw: 1_000,
    };
    const SUB_I: ActorAnimationDescriptor = ActorAnimationDescriptor {
        capability_bit_3_sound_id: 72,
        capability_mask_0x201_sound_id: 0,
        attention_stop_sound_id: 72,
        variable_binding: 1,
        frames_per_direction: 4,
    };

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(SUB_A)),
            sub_b_lateral_descriptor: RetailRuntimeValue::Known(Some(SUB_B)),
            sub_d_steering_descriptor: RetailRuntimeValue::Known(Some(ORDINARY_TYPE9_SUB_D)),
            actor_animation_descriptor: RetailRuntimeValue::Known(Some(SUB_I)),
            common_mover_topology: RetailRuntimeValue::Known(CommonMoverComponentTopology {
                sub_a: true,
                sub_b: true,
                sub_d: true,
                sub_i: true,
                ..CommonMoverComponentTopology::default()
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn topology() -> OrdinaryType9Topology {
        OrdinaryType9Topology::from_metadata(ORDINARY_TYPE9_ENTITY_TYPE, &exact_metadata()).unwrap()
    }

    fn flat_terrain() -> TerrainGrid {
        TerrainGrid {
            header: [-1 << 8, 0, 0, 0, 0],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn state() -> OrdinaryType9FrameState {
        OrdinaryType9FrameState {
            sub_d_frame_owner: Type9SubDFrameOwner::from_retail_state([0; 8], [0, 0], 5),
            sub_d_runtime: Type9SubDRuntime::default(),
            sub_a_runtime: SubAPropulsionRuntime::from_retail_words(
                RetailRuntimeValue::Known(250),
                1,
                100,
            ),
            actor_animation: ActorAnimationController::from_descriptor(SUB_I).unwrap(),
        }
    }

    fn request<'a>(
        topology: OrdinaryType9Topology,
        terrain: &'a TerrainGrid,
    ) -> OrdinaryType9FrameRequest<'a> {
        OrdinaryType9FrameRequest {
            topology,
            wander: WanderNearPrivateState {
                target_position_raw: [0, 0, 100],
                tracked_entity_handle: 0,
                direction: 1,
                reversal_timer_ms: 0,
            },
            tracked_target: RetailRuntimeValue::Unresolved,
            terrain,
            position_raw: [0, 0, 0],
            pre_mover_right_q31: [0, 0, i32::MAX],
            pre_mover_forward_q31: [i32::MAX, 0, 0],
            heading_raw: 0x1000,
            velocity_raw: [0, 0, 0],
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            scheduler_mode: 0,
        }
    }

    fn applied(
        result: Result<OrdinaryType9FrameOutcome, OrdinaryType9FrameBlock>,
    ) -> OrdinaryType9FrameStep {
        let OrdinaryType9FrameOutcome::Applied(step) = result.unwrap() else {
            panic!("expected an applied ordinary type-9 frame");
        };
        step
    }

    fn advance_without_rng(
        state: &mut OrdinaryType9FrameState,
        request: OrdinaryType9FrameRequest<'_>,
    ) -> Result<OrdinaryType9FrameOutcome, OrdinaryType9FrameBlock> {
        advance_ordinary_type9_common_mover(state, request, || {
            panic!("this ordinary type-9 branch must not consume RNG")
        })
    }

    #[test]
    fn topology_gate_requires_exact_route_and_descriptors() {
        let metadata = exact_metadata();
        let exact = OrdinaryType9Topology::from_metadata(9, &metadata).unwrap();
        assert_eq!(
            CommonMoverPostDispatchPlan::from_topology(
                exact.shared.component_topology(),
                RetailRuntimeValue::Known(250),
            )
            .unwrap()
            .phases(),
            [
                None,
                Some(CommonMoverPostDispatchPhase::SubA),
                Some(CommonMoverPostDispatchPhase::SubB),
            ]
        );
        assert_eq!(
            OrdinaryType9Topology::from_metadata(8, &metadata),
            Err(OrdinaryType9TopologyError::WrongEntityType { actual: 8 })
        );

        let mut unresolved = metadata.clone();
        unresolved.common_mover_topology = RetailRuntimeValue::Unresolved;
        assert_eq!(
            OrdinaryType9Topology::from_metadata(9, &unresolved),
            Err(OrdinaryType9TopologyError::UnresolvedComponentTopology)
        );

        type TopologyMutation = fn(&mut CommonMoverComponentTopology);
        for mutate in [
            (|value: &mut CommonMoverComponentTopology| value.sub_a = false) as TopologyMutation,
            |value| value.sub_b = false,
            |value| value.sub_d = false,
            |value| value.sub_i = false,
            |value| value.sub_c = true,
            |value| value.sub_f = true,
            |value| value.sub_g = true,
            |value| value.sub_h = true,
            |value| value.sub_k = true,
            |value| value.sub_l = true,
        ] {
            let mut unsupported = metadata.clone();
            let RetailRuntimeValue::Known(mut components) = unsupported.common_mover_topology
            else {
                unreachable!();
            };
            mutate(&mut components);
            unsupported.common_mover_topology = RetailRuntimeValue::Known(components);
            assert_eq!(
                OrdinaryType9Topology::from_metadata(9, &unsupported),
                Err(OrdinaryType9TopologyError::UnsupportedComponentTopology)
            );
        }

        let mut inert_components = metadata.clone();
        let RetailRuntimeValue::Known(mut components) = inert_components.common_mover_topology
        else {
            unreachable!();
        };
        components.sub_e = true;
        components.sub_j = true;
        components.sub_m = true;
        components.sub_n = true;
        components.sub_o = true;
        inert_components.common_mover_topology = RetailRuntimeValue::Known(components);
        assert!(OrdinaryType9Topology::from_metadata(9, &inert_components).is_ok());

        let mut wrong_d = metadata;
        wrong_d.sub_d_steering_descriptor =
            RetailRuntimeValue::Known(Some(SubDSteeringDescriptor {
                steering_divisor_raw: 19,
                ..ORDINARY_TYPE9_SUB_D
            }));
        assert_eq!(
            OrdinaryType9Topology::from_metadata(9, &wrong_d),
            Err(OrdinaryType9TopologyError::UnsupportedSubDDescriptor)
        );
    }

    #[test]
    fn frame_uses_post_d_heading_but_pre_d_basis_for_a_and_b() {
        let terrain = flat_terrain();
        let mut state = state();
        let initial_stagger = state.sub_d_frame_owner.classifier_cache().stagger_counter();
        let step = applied(advance_without_rng(
            &mut state,
            request(topology(), &terrain),
        ));

        assert!(step.yaw_step_raw > 0);
        assert_eq!(
            step.heading_raw,
            0x1000u16.wrapping_sub(step.yaw_step_raw as u16)
        );
        assert_eq!(
            step.actor_animation_selector, 25,
            "post-D heading crosses from direction bin 5 to bin 6 before Sub-I"
        );
        assert!(step.propulsion_applied);
        assert!(step.velocity_raw[0] > 0);
        assert_eq!(
            step.velocity_raw[2], 0,
            "A/B retain the supplied pre-D basis instead of rebuilding from heading"
        );
        assert_eq!(
            state.actor_animation.output(),
            step.actor_animation_selector
        );
        assert_eq!(state.sub_d_runtime.last_yaw_step_raw, step.yaw_step_raw);
        assert_eq!(
            state.sub_d_frame_owner.classifier_cache().stagger_counter(),
            initial_stagger.wrapping_add(1),
            "one accepted mover frame advances the Sub-D allocation exactly once"
        );
    }

    #[test]
    fn every_nonzero_scheduler_mode_skips_sub_i_but_keeps_d_a_b() {
        let terrain = flat_terrain();
        let exact_topology = topology();
        let mut expected = None;

        for scheduler_mode in [1, 7, -1] {
            let mut restricted = state();
            restricted.actor_animation =
                ActorAnimationController::from_descriptor(ActorAnimationDescriptor {
                    frames_per_direction: 3,
                    ..SUB_I
                })
                .unwrap();
            restricted.actor_animation.apply_exploding_person_reset();
            let animation_before = restricted.actor_animation;
            let initial_stagger = restricted
                .sub_d_frame_owner
                .classifier_cache()
                .stagger_counter();
            let mut frame = request(exact_topology, &terrain);
            frame.scheduler_mode = scheduler_mode;

            let step = applied(advance_without_rng(&mut restricted, frame));

            assert!(step.yaw_step_raw > 0);
            assert!(step.propulsion_applied);
            assert!(step.velocity_raw[0] > 0);
            assert_eq!(step.velocity_raw[2], 0);
            assert_eq!(restricted.actor_animation, animation_before);
            assert_eq!(step.actor_animation_selector, animation_before.output());
            assert_eq!(
                restricted
                    .sub_d_frame_owner
                    .classifier_cache()
                    .stagger_counter(),
                initial_stagger.wrapping_add(1)
            );

            let result = (restricted, step);
            if let Some(expected) = expected {
                assert_eq!(result, expected);
            } else {
                expected = Some(result);
            }
        }
    }

    #[test]
    fn explicit_exploding_policy_advances_special_sub_i_without_weakening_neutral_api() {
        let terrain = flat_terrain();
        let mut special = state();
        special.actor_animation.apply_exploding_person_reset();
        let before = special;

        assert_eq!(
            advance_ordinary_type9_common_mover(
                &mut special,
                request(topology(), &terrain),
                || panic!("the exact special-mode frame consumes no mover RNG"),
            ),
            Err(OrdinaryType9FrameBlock::NonNeutralActorAnimationRequiresOwner)
        );
        assert_eq!(
            special, before,
            "the existing neutral entry point stays fail-closed and atomic"
        );

        let step = applied(advance_ordinary_type9_common_mover_with_animation_policy(
            &mut special,
            request(topology(), &terrain),
            ActorAbdiAnimationPolicy::ExplodingPersonSpecial,
            || panic!("the exact special-mode frame consumes no mover RNG"),
        ));
        assert!(special.actor_animation.special_mode());
        assert_eq!(special.actor_animation.linked_handle(), None);
        assert_eq!(special.actor_animation.phase(), 1);
        assert_eq!(special.actor_animation.output(), 39);
        assert_eq!(step.actor_animation_selector, 39);

        let mut neutral = state();
        let before = neutral;
        assert_eq!(
            advance_ordinary_type9_common_mover_with_animation_policy(
                &mut neutral,
                request(topology(), &terrain),
                ActorAbdiAnimationPolicy::ExplodingPersonSpecial,
                || panic!("a policy mismatch consumes no RNG"),
            ),
            Err(OrdinaryType9FrameBlock::ExplodingPersonAnimationStateMismatch)
        );
        assert_eq!(neutral, before);
    }

    #[test]
    fn explicit_attract_policy_persists_forced_stop_and_zeroes_velocity_before_a_b() {
        let terrain = flat_terrain();
        let mut attract = state();
        attract
            .actor_animation
            .apply_attract_attention_forced_stop();
        let mut frame = request(topology(), &terrain);
        frame.velocity_raw = [0, 1_200, -900];

        let step = applied(advance_ordinary_type9_common_mover_with_animation_policy(
            &mut attract,
            frame,
            ActorAbdiAnimationPolicy::AttractAttentionForcedStop,
            || panic!("the exact forced-stop frame consumes no mover RNG"),
        ));

        assert!(attract.actor_animation.forced_stop());
        assert!(!attract.actor_animation.special_mode());
        assert_eq!(attract.actor_animation.linked_handle(), None);
        assert_eq!(attract.actor_animation.phase(), 1);
        assert_eq!(step.actor_animation_selector, 33);
        assert!(step.velocity_raw[0] > 0, "Sub-A runs after the zero write");
        assert_eq!(step.velocity_raw[1], 0);
        assert_eq!(step.velocity_raw[2], 0);
    }

    #[test]
    fn neutral_api_stays_fail_closed_for_exact_attract_forced_stop() {
        let terrain = flat_terrain();
        let mut attract = state();
        attract
            .actor_animation
            .apply_attract_attention_forced_stop();
        let before = attract;

        assert_eq!(
            advance_ordinary_type9_common_mover(
                &mut attract,
                request(topology(), &terrain),
                || panic!("a neutral-policy rejection consumes no RNG"),
            ),
            Err(OrdinaryType9FrameBlock::NonNeutralActorAnimationRequiresOwner)
        );
        assert_eq!(attract, before);
    }

    #[test]
    fn attract_policy_rejects_special_or_missing_forced_stop_state() {
        let terrain = flat_terrain();
        for mut invalid in [
            state(),
            {
                let mut value = state();
                value.actor_animation.apply_attract_attention_forced_stop();
                value.actor_animation.apply_exploding_person_reset();
                value
            },
            {
                let mut value = state();
                value.actor_animation.apply_attract_attention_forced_stop();
                value
                    .actor_animation
                    .set_linked_handle_for_test(Some(0x04ac_0001));
                value
            },
        ] {
            let before = invalid;
            assert_eq!(
                advance_ordinary_type9_common_mover_with_animation_policy(
                    &mut invalid,
                    request(topology(), &terrain),
                    ActorAbdiAnimationPolicy::AttractAttentionForcedStop,
                    || panic!("a forced-stop policy mismatch consumes no RNG"),
                ),
                Err(OrdinaryType9FrameBlock::AttractAttentionAnimationStateMismatch)
            );
            assert_eq!(invalid, before);
        }
    }

    #[test]
    fn every_unresolved_prelude_gate_is_atomic() {
        let terrain = flat_terrain();
        let exact_topology = topology();

        let mut unresolved_speed = state();
        unresolved_speed.sub_a_runtime =
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Unresolved, 1, 100);
        let before = unresolved_speed;
        let mut unresolved_speed_request = request(exact_topology, &terrain);
        unresolved_speed_request.scheduler_mode = 1;
        assert_eq!(
            advance_without_rng(&mut unresolved_speed, unresolved_speed_request),
            Err(OrdinaryType9FrameBlock::UnresolvedSubATargetSpeed)
        );
        assert_eq!(unresolved_speed, before);

        let mut tracked = state();
        let before = tracked;
        let mut tracked_request = request(exact_topology, &terrain);
        tracked_request.wander.tracked_entity_handle = 0x04ac_0001;
        assert_eq!(
            advance_without_rng(&mut tracked, tracked_request),
            Err(OrdinaryType9FrameBlock::TargetPrelude(
                CommonMoverTargetPreludeBlock::UnresolvedTrackedTarget {
                    handle: 0x04ac_0001
                }
            ))
        );
        assert_eq!(tracked, before);

        let mut timed = state();
        let mut timed_request = request(exact_topology, &terrain);
        timed_request.wander.reversal_timer_ms = 500;
        let timed_step = applied(advance_without_rng(&mut timed, timed_request));
        assert_eq!(timed_step.wander.reversal_timer_ms, 480);

        let mut mismatched_animation = state();
        mismatched_animation.actor_animation =
            ActorAnimationController::from_descriptor(ActorAnimationDescriptor {
                frames_per_direction: 3,
                ..SUB_I
            })
            .unwrap();
        let before = mismatched_animation;
        assert_eq!(
            advance_without_rng(&mut mismatched_animation, request(exact_topology, &terrain),),
            Err(OrdinaryType9FrameBlock::ActorAnimationDescriptorMismatch)
        );
        assert_eq!(mismatched_animation, before);

        let mut unresolved_cache_origin = state();
        unresolved_cache_origin.sub_d_frame_owner =
            Type9SubDFrameOwner::pending_constructor_origin(5);
        let before = unresolved_cache_origin;
        let mut unresolved_cache_request = request(exact_topology, &terrain);
        unresolved_cache_request.scheduler_mode = 1;
        assert_eq!(
            advance_without_rng(&mut unresolved_cache_origin, unresolved_cache_request),
            Err(OrdinaryType9FrameBlock::SubD(
                Type9SubDStep::UnresolvedClassifierCache
            ))
        );
        assert_eq!(unresolved_cache_origin, before);
    }

    #[test]
    fn sub_i_direction_mismatch_turns_and_propagates_without_rng_or_basis_rebuild() {
        let terrain = flat_terrain();
        let mut reversed = state();
        let mut frame = request(topology(), &terrain);
        frame.wander.direction = -1;

        let step = applied(advance_without_rng(&mut reversed, frame));

        assert_eq!(
            step.heading_raw,
            0x3000u16.wrapping_sub(step.yaw_step_raw as u16),
            "Sub-I's mismatch branch adds 0x2000 before Sub-D commits yaw"
        );
        assert_eq!(reversed.sub_a_runtime.direction_multiplier(), -1);
        assert!(
            step.velocity_raw[0] < 0,
            "Sub-A receives the propagated reverse direction"
        );
        assert_eq!(
            step.velocity_raw[2], 0,
            "A/B retain the caller's pre-mismatch body basis"
        );
    }

    #[test]
    fn adapter_commits_tracked_extrapolation_before_sub_d() {
        let terrain = flat_terrain();
        let mut state = state();
        let mut frame = request(topology(), &terrain);
        frame.wander.tracked_entity_handle = 0x04ac_0001;
        frame.tracked_target = RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
            state_flags: crate::entity_collision_state::RetailStateWord::exact(1),
            position_raw: [100, 200, 300],
            velocity_raw: [256, -256, 0],
        }));

        let step = applied(advance_without_rng(&mut state, frame));

        assert_eq!(step.wander.target_position_raw, [109, 190, 300]);
        assert_eq!(step.wander.tracked_entity_handle, 0x04ac_0001);
    }

    #[test]
    fn adapter_returns_retail_zero_without_committing_frame_state() {
        let terrain = flat_terrain();
        let mut state = state();
        let before = state;
        let mut frame = request(topology(), &terrain);
        frame.wander.tracked_entity_handle = 0x04ac_0001;
        frame.tracked_target = RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
            state_flags: crate::entity_collision_state::RetailStateWord::exact(0),
            position_raw: [100, 200, 300],
            velocity_raw: [256, -256, 0],
        }));

        assert_eq!(
            advance_without_rng(&mut state, frame),
            Ok(OrdinaryType9FrameOutcome::ReturnedZero(
                CommonMoverTargetPreludeZero::TrackedEntityInactive
            ))
        );
        assert_eq!(state, before);
    }

    #[test]
    fn adapter_expires_timer_and_consumes_the_one_authored_speed_draw() {
        let terrain = flat_terrain();
        let mut state = state();
        state.sub_a_runtime =
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(200), -1, 73);
        let mut frame = request(topology(), &terrain);
        frame.wander.direction = -1;
        frame.wander.reversal_timer_ms = 20;
        let mut calls = 0;

        let outcome = advance_ordinary_type9_common_mover(&mut state, frame, || {
            calls += 1;
            0x0000_ff00
        })
        .unwrap();
        let OrdinaryType9FrameOutcome::Applied(step) = outcome else {
            panic!("expected applied timer-expiry frame");
        };

        assert_eq!(calls, 1);
        assert_eq!(step.wander.direction, 1);
        assert_eq!(step.wander.reversal_timer_ms, 0);
        assert_eq!(state.sub_a_runtime.direction_multiplier(), 1);
        assert_eq!(
            state.sub_a_runtime.target_speed_raw(),
            RetailRuntimeValue::Known(274)
        );
    }

    #[test]
    fn adapter_preflights_late_sub_d_boundary_before_timer_rng() {
        let terrain = flat_terrain();
        let mut state = state();
        state.sub_d_frame_owner = Type9SubDFrameOwner::pending_constructor_origin(5);
        let before = state;
        let mut frame = request(topology(), &terrain);
        frame.wander.reversal_timer_ms = 1;
        let mut calls = 0;

        assert_eq!(
            advance_ordinary_type9_common_mover(&mut state, frame, || {
                calls += 1;
                0
            }),
            Err(OrdinaryType9FrameBlock::SubD(
                Type9SubDStep::UnresolvedClassifierCache
            ))
        );
        assert_eq!(calls, 0);
        assert_eq!(state, before);
    }

    #[test]
    fn accepted_fresh_level1_first_query_passes_atomic_preflight() {
        let terrain = flat_terrain();
        let mut state = state();
        state.sub_d_frame_owner = Type9SubDFrameOwner::pending_fresh_level1_first_query_reset(
            0x2a,
            FreshLevel1Type9ClassifierAdmission::ACCEPTED_FIRST_CONSUMER_TRACE,
        );

        assert_eq!(
            state.sub_d_frame_owner.classifier_cache().origin(),
            RetailRuntimeValue::Unresolved
        );
        let step = applied(advance_without_rng(
            &mut state,
            request(topology(), &terrain),
        ));

        assert!(step.propulsion_applied);
        assert!(matches!(
            state.sub_d_frame_owner.classifier_cache().origin(),
            RetailRuntimeValue::Known(_)
        ));
        assert_eq!(
            state.sub_d_frame_owner.classifier_cache().stagger_counter(),
            0x2b,
            "the admitted first frame advances the captured per-allocation seed once"
        );
        assert!(
            state
                .sub_d_frame_owner
                .classifier_cache()
                .rows()
                .iter()
                .any(|row| *row != 0),
            "the accepted frame must commit the first-query reset and classifier fills"
        );
    }

    #[test]
    fn zero_sub_a_target_is_a_successful_no_op_before_sub_b() {
        let terrain = flat_terrain();
        let mut state = state();
        state.sub_a_runtime =
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(0), 1, 100);
        let mut frame = request(topology(), &terrain);
        frame.velocity_raw = [0, 0, 100];

        let step = applied(advance_without_rng(&mut state, frame));

        assert!(!step.propulsion_applied);
        assert_eq!(step.velocity_raw[0], 0);
        assert!(
            step.velocity_raw[2] < 100,
            "Sub-B still removes lateral velocity after Sub-A's zero-target gate"
        );
    }
}
