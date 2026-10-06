//! Resumable detached orchestration for retail `FUN_00401430`.
//!
//! The common mover interleaves exact arithmetic with opaque component
//! callbacks, live resource lookups, an effect materializer, and writes into
//! component allocations. A one-shot pure planner would be incorrect because
//! the later gates consume state changed by those callbacks. This module
//! therefore stops at every such boundary. The adapter must perform the
//! requested action and return a fresh snapshot before the machine selects the
//! next retail phase.
//!
//! No action in this module invokes a callback or mutates live game state.

use super::component_dispatch::{
    CommonMoverDispatchMode, CommonMoverDispatchPhase, CommonMoverDispatchPlan,
};
use super::post_dispatch::{
    CommonMoverPostDispatchBlock, CommonMoverPostDispatchPhase, CommonMoverPostDispatchPlan,
};
use super::sub_c::{
    apply_sub_c_lift_raw, sub_c_surface_sample_point_raw, sub_c_uses_wave_surface, SubCLiftOutcome,
    SubCSurfaceSample,
};
use super::target_correction::{
    plan_primary_common_mover_target_correction, plan_secondary_common_mover_target_correction,
    CommonMoverPrimaryTargetCorrectionPlan, CommonMoverSecondaryTargetCorrectionPlan,
    CommonMoverTargetCorrectionEntity, CommonMoverTargetCorrectionRequest,
};
use super::target_prelude::{
    plan_common_mover_post_sub_d_writes, plan_common_mover_target_prelude,
    CommonMoverPostSubDWritePlan, CommonMoverPostSubDWriteRequest, CommonMoverPreludeSubA,
    CommonMoverPreludeSubD, CommonMoverTargetPreludeBlock, CommonMoverTargetPreludeOutcome,
    CommonMoverTargetPreludePlan, CommonMoverTargetPreludeRequest,
    CommonMoverTargetPreludeTopology, CommonMoverTargetPreludeZero,
    CommonMoverTrackedTargetSnapshot,
};
use super::{apply_sub_a_propulsion_raw, apply_sub_b_lateral_raw, SubAPropulsionRuntime};
use crate::entity_collision_state::{CommonMoverComponentTopology, RetailRuntimeValue};
use crate::hover::{q31_mul, HoverLiftConfig};
use crate::wander_near_location::WanderNearPrivateState;
use v2k_formats::collision::{SubAPropulsionDescriptor, SubBLateralDescriptor};

/// Static inputs which remain stable throughout one retail mover call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverFrameConfiguration {
    pub controlled_entity_handle: u32,
    pub topology: CommonMoverComponentTopology,
    pub dispatch_mode: CommonMoverDispatchMode,
    pub elapsed_micros: u32,
    pub sub_a_descriptor: Option<SubAPropulsionDescriptor>,
    pub sub_b_descriptor: Option<SubBLateralDescriptor>,
    pub sub_c_descriptor: Option<HoverLiftConfig>,
    pub sub_d_descriptor: Option<CommonMoverPreludeSubD>,
    /// Whether the initial target-handle resource lookup returned non-null.
    ///
    /// This is independent of the controlled-entity lookup used by the two
    /// target-correction calls.
    pub target_resource_context_present: bool,
}

/// Result of the controlled-entity lookup consumed by target correction.
///
/// Position is deliberately not repeated here: a successful lookup refers to
/// the same controlled entity represented by [`CommonMoverFrameSnapshot`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverControlledEntityLookup {
    pub attitude_raw_28: i32,
}

/// Live state which must be sampled again after every callback or committed
/// write boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverFrameSnapshot {
    pub controlled_entity_lookup: Option<CommonMoverControlledEntityLookup>,
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub heading_raw: u16,
    pub roll_raw: u16,
    pub body_up_q31: [i32; 3],
    pub body_right_q31: [i32; 3],
    pub body_forward_q31: [i32; 3],
    pub attached_cargo_mass: u32,
    pub sub_a_runtime: RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
    pub sub_f_smoothed_raw: Option<i32>,
    pub sub_g_runtime_angle_raw: i16,
    pub sub_g_state_byte_3c: u8,
    pub sub_k_smoothed_raw: Option<i32>,
    pub sub_n_accumulator_raw: Option<i16>,
    pub sub_o_link_raw: u32,
}

/// Initial call inputs. A missing target models retail `param_5 == NULL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverFrameRequest {
    pub configuration: CommonMoverFrameConfiguration,
    pub initial_snapshot: CommonMoverFrameSnapshot,
    pub target_private: Option<WanderNearPrivateState>,
    /// Lookup result for `target_private.tracked_entity_handle`.
    ///
    /// Ignored when there is no target private state or its handle is zero.
    pub tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
}

/// Exact deterministic write whose completion is being acknowledged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonMoverFrameCommitPhase {
    TargetPrelude,
    PostSubDWrites,
    PrimaryTargetCorrection,
    SubNAccumulator,
    SubC,
    SubA,
    SubB,
    SecondaryTargetCorrection,
}

/// One explicit adapter boundary in retail order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonMoverFrameAction {
    CommitTargetPrelude(CommonMoverTargetPreludePlan),
    InvokeSubD {
        controlled_entity_handle: u32,
        target_position_raw: [i16; 3],
        elapsed_micros: u32,
    },
    CommitPostSubDWrites(CommonMoverPostSubDWritePlan),
    CommitPrimaryTargetCorrection(CommonMoverPrimaryTargetCorrectionPlan),
    InvokePrimaryCorrectionEffect {
        controlled_entity_handle: u32,
    },
    CommitSubNAccumulator {
        before_raw: i16,
        after_raw: i16,
    },
    InvokeComponent {
        phase: CommonMoverDispatchPhase,
        controlled_entity_handle: u32,
        elapsed_micros: u32,
    },
    SampleSubCSurface {
        point_raw: [i16; 2],
        use_wave_surface: bool,
    },
    CommitSubC {
        position_raw: [i16; 3],
        velocity_raw: [i16; 3],
        outcome: SubCLiftOutcome,
    },
    CommitSubA {
        velocity_raw: [i16; 3],
        propulsion_applied: bool,
    },
    CommitSubB {
        velocity_raw: [i16; 3],
    },
    CommitSecondaryTargetCorrection(CommonMoverSecondaryTargetCorrectionPlan),
}

/// Terminal or pending state exposed by [`CommonMoverFrameMachine::poll`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonMoverFramePoll {
    Action(CommonMoverFrameAction),
    Blocked(CommonMoverFrameBlock),
    ReturnZero(CommonMoverTargetPreludeZero),
    ReturnOne,
}

/// Result supplied after completing the currently requested adapter action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonMoverFrameResume {
    /// A deterministic write plan has been committed to live state.
    Committed {
        phase: CommonMoverFrameCommitPhase,
        snapshot: CommonMoverFrameSnapshot,
    },
    /// The opaque Sub-D callback returned its signed low word.
    SubDReturned {
        result_raw: i16,
        snapshot: CommonMoverFrameSnapshot,
    },
    /// One F/H/I/G/K/L callback returned.
    ComponentReturned {
        phase: CommonMoverDispatchPhase,
        snapshot: CommonMoverFrameSnapshot,
    },
    /// The primary correction's opaque effect materializer returned.
    PrimaryCorrectionEffectReturned(CommonMoverFrameSnapshot),
    /// The caller sampled the requested terrain or wave surface.
    SubCSurfaceSampled(SubCSurfaceSample),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonMoverFrameRequiredDescriptor {
    SubA,
    SubB,
    SubC,
    SubD,
}

/// Evidence or runtime-state boundary which prevents exact advancement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonMoverFrameBlock {
    TargetPrelude(CommonMoverTargetPreludeBlock),
    NullTargetSecondaryCorrectionUsesIndeterminateContext,
    PostDispatch(CommonMoverPostDispatchBlock),
    MissingRequiredDescriptor(CommonMoverFrameRequiredDescriptor),
    MissingSubARuntime,
    UnresolvedSubARuntime,
    MissingSubFRuntime,
    MissingSubKRuntime,
    MissingSubNRuntime,
}

/// Invalid adapter response for the currently pending retail action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonMoverFrameProtocolError {
    ExpectedCommitResult {
        expected: CommonMoverFrameCommitPhase,
        actual: Option<CommonMoverFrameCommitPhase>,
    },
    ExpectedSubDResult,
    ExpectedComponentResult {
        expected: CommonMoverDispatchPhase,
        actual: Option<CommonMoverDispatchPhase>,
    },
    ExpectedPrimaryCorrectionEffectResult,
    ExpectedSubCSurfaceSample,
    SurfaceKindMismatch {
        expected_wave: bool,
        actual_wave: bool,
    },
    MachineBlocked(CommonMoverFrameBlock),
    AlreadyComplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonMoverFrameAdvanceError {
    Block(CommonMoverFrameBlock),
    Protocol(CommonMoverFrameProtocolError),
}

impl From<CommonMoverFrameBlock> for CommonMoverFrameAdvanceError {
    fn from(value: CommonMoverFrameBlock) -> Self {
        Self::Block(value)
    }
}

impl From<CommonMoverFrameProtocolError> for CommonMoverFrameAdvanceError {
    fn from(value: CommonMoverFrameProtocolError) -> Self {
        Self::Protocol(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MachineStage {
    CommitPrelude(CommonMoverTargetPreludePlan),
    InvokeSubD {
        target_position_raw: [i16; 3],
    },
    CommitPostSubDWrites(CommonMoverPostSubDWritePlan),
    CommitPrimaryCorrection(CommonMoverPrimaryTargetCorrectionPlan),
    InvokePrimaryCorrectionEffect,
    CommitSubN {
        before_raw: i16,
        after_raw: i16,
    },
    InvokeComponent {
        phase: CommonMoverDispatchPhase,
        next_index: usize,
    },
    SampleSubC {
        point_raw: [i16; 2],
        expected_wave: bool,
        next_tail_index: usize,
    },
    CommitSubC {
        position_raw: [i16; 3],
        velocity_raw: [i16; 3],
        outcome: SubCLiftOutcome,
        next_tail_index: usize,
    },
    CommitSubA {
        velocity_raw: [i16; 3],
        propulsion_applied: bool,
        next_tail_index: usize,
    },
    CommitSubB {
        velocity_raw: [i16; 3],
        next_tail_index: usize,
    },
    CommitSecondaryCorrection(CommonMoverSecondaryTargetCorrectionPlan),
    Blocked(CommonMoverFrameBlock),
    ReturnZero(CommonMoverTargetPreludeZero),
    ReturnOne,
}

/// Exact, detached, resumable `FUN_00401430` orchestration.
#[derive(Debug, PartialEq, Eq)]
pub struct CommonMoverFrameMachine {
    configuration: CommonMoverFrameConfiguration,
    snapshot: CommonMoverFrameSnapshot,
    target_position_raw: Option<[i16; 3]>,
    dispatch_phases: [Option<CommonMoverDispatchPhase>; 3],
    stage: MachineStage,
}

impl CommonMoverFrameMachine {
    /// Start one mover call and evaluate the exact target/reversal prelude.
    ///
    /// All prelude evidence gates are resolved before RNG is consumed. A null
    /// target skips the complete prefix exactly as retail does.
    pub fn start(
        request: CommonMoverFrameRequest,
        next_random: impl FnMut() -> u32,
    ) -> Result<Self, CommonMoverFrameBlock> {
        let dispatch_phases = CommonMoverDispatchPlan::from_topology(
            request.configuration.topology,
            request.configuration.dispatch_mode,
        )
        .phases();
        let mut machine = Self {
            configuration: request.configuration,
            snapshot: request.initial_snapshot,
            target_position_raw: None,
            dispatch_phases,
            stage: MachineStage::ReturnOne,
        };

        if let Some(target_private) = request.target_private {
            // Retail rejects a dying/inactive tracked target before it reads
            // Section-12 topology or Sub-A's runtime allocation. Run that
            // prefix against an intentionally unresolved topology first so a
            // rejected target can still terminate when later evidence is
            // unavailable. This pass cannot consume RNG.
            let validation_request = machine.target_prelude_request_with_topology(
                target_private,
                request.tracked_target,
                RetailRuntimeValue::Unresolved,
            );
            match plan_common_mover_target_prelude(validation_request, || {
                unreachable!("target validation cannot consume RNG before topology resolution")
            }) {
                Ok(CommonMoverTargetPreludeOutcome::ReturnZero(reason)) => {
                    machine.stage = MachineStage::ReturnZero(reason);
                    return Ok(machine);
                }
                Err(CommonMoverTargetPreludeBlock::UnresolvedTopology) => {}
                Err(block) => return Err(CommonMoverFrameBlock::TargetPrelude(block)),
                Ok(CommonMoverTargetPreludeOutcome::Continue(_)) => {
                    unreachable!("an unresolved topology cannot produce a continuation plan")
                }
            }

            let prelude_request =
                machine.target_prelude_request(target_private, request.tracked_target)?;
            match plan_common_mover_target_prelude(prelude_request, next_random)
                .map_err(CommonMoverFrameBlock::TargetPrelude)?
            {
                CommonMoverTargetPreludeOutcome::Continue(plan) => {
                    machine.target_position_raw = Some(plan.target_state.target_position_raw);
                    machine.stage = MachineStage::CommitPrelude(plan);
                }
                CommonMoverTargetPreludeOutcome::ReturnZero(reason) => {
                    machine.stage = MachineStage::ReturnZero(reason);
                }
            }
        } else {
            machine.enter_after_prelude()?;
        }

        Ok(machine)
    }

    pub const fn poll(&self) -> CommonMoverFramePoll {
        match self.stage {
            MachineStage::CommitPrelude(plan) => {
                CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitTargetPrelude(plan))
            }
            MachineStage::InvokeSubD {
                target_position_raw,
            } => CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeSubD {
                controlled_entity_handle: self.configuration.controlled_entity_handle,
                target_position_raw,
                elapsed_micros: self.configuration.elapsed_micros,
            }),
            MachineStage::CommitPostSubDWrites(plan) => {
                CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitPostSubDWrites(plan))
            }
            MachineStage::CommitPrimaryCorrection(plan) => CommonMoverFramePoll::Action(
                CommonMoverFrameAction::CommitPrimaryTargetCorrection(plan),
            ),
            MachineStage::InvokePrimaryCorrectionEffect => CommonMoverFramePoll::Action(
                CommonMoverFrameAction::InvokePrimaryCorrectionEffect {
                    controlled_entity_handle: self.configuration.controlled_entity_handle,
                },
            ),
            MachineStage::CommitSubN {
                before_raw,
                after_raw,
            } => CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubNAccumulator {
                before_raw,
                after_raw,
            }),
            MachineStage::InvokeComponent { phase, .. } => {
                CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeComponent {
                    phase,
                    controlled_entity_handle: self.configuration.controlled_entity_handle,
                    elapsed_micros: self.configuration.elapsed_micros,
                })
            }
            MachineStage::SampleSubC {
                point_raw,
                expected_wave,
                ..
            } => CommonMoverFramePoll::Action(CommonMoverFrameAction::SampleSubCSurface {
                point_raw,
                use_wave_surface: expected_wave,
            }),
            MachineStage::CommitSubC {
                position_raw,
                velocity_raw,
                outcome,
                ..
            } => CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubC {
                position_raw,
                velocity_raw,
                outcome,
            }),
            MachineStage::CommitSubA {
                velocity_raw,
                propulsion_applied,
                ..
            } => CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubA {
                velocity_raw,
                propulsion_applied,
            }),
            MachineStage::CommitSubB { velocity_raw, .. } => {
                CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubB { velocity_raw })
            }
            MachineStage::CommitSecondaryCorrection(plan) => CommonMoverFramePoll::Action(
                CommonMoverFrameAction::CommitSecondaryTargetCorrection(plan),
            ),
            MachineStage::Blocked(block) => CommonMoverFramePoll::Blocked(block),
            MachineStage::ReturnZero(reason) => CommonMoverFramePoll::ReturnZero(reason),
            MachineStage::ReturnOne => CommonMoverFramePoll::ReturnOne,
        }
    }

    /// Resume after exactly one pending adapter boundary.
    pub fn resume(
        &mut self,
        completion: CommonMoverFrameResume,
    ) -> Result<(), CommonMoverFrameAdvanceError> {
        match self.stage {
            MachineStage::CommitPrelude(_) => {
                let snapshot = committed(completion, CommonMoverFrameCommitPhase::TargetPrelude)?;
                self.snapshot = snapshot;
                let transition = self.enter_after_prelude();
                self.finish_transition(transition)?;
            }
            MachineStage::InvokeSubD { .. } => {
                let CommonMoverFrameResume::SubDReturned {
                    result_raw,
                    snapshot,
                } = completion
                else {
                    return Err(CommonMoverFrameProtocolError::ExpectedSubDResult.into());
                };
                self.snapshot = snapshot;
                let transition = self.enter_after_sub_d(result_raw);
                self.finish_transition(transition)?;
            }
            MachineStage::CommitPostSubDWrites(_) => {
                self.snapshot = committed(completion, CommonMoverFrameCommitPhase::PostSubDWrites)?;
                let transition = self.enter_primary_correction();
                self.finish_transition(transition)?;
            }
            MachineStage::CommitPrimaryCorrection(_) => {
                self.snapshot = committed(
                    completion,
                    CommonMoverFrameCommitPhase::PrimaryTargetCorrection,
                )?;
                self.stage = MachineStage::InvokePrimaryCorrectionEffect;
            }
            MachineStage::InvokePrimaryCorrectionEffect => {
                let CommonMoverFrameResume::PrimaryCorrectionEffectReturned(snapshot) = completion
                else {
                    return Err(
                        CommonMoverFrameProtocolError::ExpectedPrimaryCorrectionEffectResult.into(),
                    );
                };
                self.snapshot = snapshot;
                let transition = self.enter_after_primary_effect();
                self.finish_transition(transition)?;
            }
            MachineStage::CommitSubN { .. } => {
                self.snapshot =
                    committed(completion, CommonMoverFrameCommitPhase::SubNAccumulator)?;
                let transition = self.enter_dispatch(0);
                self.finish_transition(transition)?;
            }
            MachineStage::InvokeComponent { phase, next_index } => {
                let CommonMoverFrameResume::ComponentReturned {
                    phase: actual,
                    snapshot,
                } = completion
                else {
                    return Err(CommonMoverFrameProtocolError::ExpectedComponentResult {
                        expected: phase,
                        actual: None,
                    }
                    .into());
                };
                if actual != phase {
                    return Err(CommonMoverFrameProtocolError::ExpectedComponentResult {
                        expected: phase,
                        actual: Some(actual),
                    }
                    .into());
                }
                self.snapshot = snapshot;
                let transition = self.enter_dispatch(next_index);
                self.finish_transition(transition)?;
            }
            MachineStage::SampleSubC {
                expected_wave,
                next_tail_index,
                ..
            } => {
                let CommonMoverFrameResume::SubCSurfaceSampled(surface) = completion else {
                    return Err(CommonMoverFrameProtocolError::ExpectedSubCSurfaceSample.into());
                };
                let actual_wave = matches!(surface, SubCSurfaceSample::Wave { .. });
                if actual_wave != expected_wave {
                    return Err(CommonMoverFrameProtocolError::SurfaceKindMismatch {
                        expected_wave,
                        actual_wave,
                    }
                    .into());
                }
                let Some(descriptor) = self.configuration.sub_c_descriptor else {
                    return self.block(CommonMoverFrameBlock::MissingRequiredDescriptor(
                        CommonMoverFrameRequiredDescriptor::SubC,
                    ));
                };
                let mut position_raw = self.snapshot.position_raw;
                let mut velocity_raw = self.snapshot.velocity_raw;
                let outcome = apply_sub_c_lift_raw(
                    descriptor,
                    &mut position_raw,
                    &mut velocity_raw,
                    self.configuration.elapsed_micros,
                    surface,
                    self.snapshot.body_up_q31,
                );
                self.stage = MachineStage::CommitSubC {
                    position_raw,
                    velocity_raw,
                    outcome,
                    next_tail_index,
                };
            }
            MachineStage::CommitSubC {
                next_tail_index, ..
            } => {
                self.snapshot = committed(completion, CommonMoverFrameCommitPhase::SubC)?;
                let transition = self.enter_tail(next_tail_index);
                self.finish_transition(transition)?;
            }
            MachineStage::CommitSubA {
                next_tail_index, ..
            } => {
                self.snapshot = committed(completion, CommonMoverFrameCommitPhase::SubA)?;
                let transition = self.enter_tail(next_tail_index);
                self.finish_transition(transition)?;
            }
            MachineStage::CommitSubB {
                next_tail_index, ..
            } => {
                self.snapshot = committed(completion, CommonMoverFrameCommitPhase::SubB)?;
                let transition = self.enter_tail(next_tail_index);
                self.finish_transition(transition)?;
            }
            MachineStage::CommitSecondaryCorrection(_) => {
                self.snapshot = committed(
                    completion,
                    CommonMoverFrameCommitPhase::SecondaryTargetCorrection,
                )?;
                self.stage = MachineStage::ReturnOne;
            }
            MachineStage::Blocked(block) => {
                return Err(CommonMoverFrameProtocolError::MachineBlocked(block).into());
            }
            MachineStage::ReturnZero(_) | MachineStage::ReturnOne => {
                return Err(CommonMoverFrameProtocolError::AlreadyComplete.into());
            }
        }
        Ok(())
    }

    fn enter_after_prelude(&mut self) -> Result<(), CommonMoverFrameBlock> {
        if let Some(target_position_raw) = self
            .target_position_raw
            .filter(|_| self.configuration.topology.sub_d)
        {
            self.stage = MachineStage::InvokeSubD {
                target_position_raw,
            };
        } else {
            self.enter_primary_correction()?;
        }
        Ok(())
    }

    fn enter_after_sub_d(&mut self, result_raw: i16) -> Result<(), CommonMoverFrameBlock> {
        self.preflight_post_sub_d_runtime()?;
        let target_position_raw = self
            .target_position_raw
            .ok_or(CommonMoverFrameBlock::NullTargetSecondaryCorrectionUsesIndeterminateContext)?;
        let plan = plan_common_mover_post_sub_d_writes(CommonMoverPostSubDWriteRequest {
            sub_d_result_raw: result_raw,
            target_position_raw,
            sub_f_smoothed_raw: self
                .configuration
                .topology
                .sub_f
                .then_some(self.snapshot.sub_f_smoothed_raw)
                .flatten(),
            sub_k_smoothed_raw: self
                .configuration
                .topology
                .sub_k
                .then_some(self.snapshot.sub_k_smoothed_raw)
                .flatten(),
            sub_l: self.configuration.topology.sub_l,
        });
        if plan.sub_f.is_some()
            || plan.sub_k_smoothed_raw.is_some()
            || plan.sub_l_exact_raw.is_some()
        {
            self.stage = MachineStage::CommitPostSubDWrites(plan);
        } else {
            self.enter_primary_correction()?;
        }
        Ok(())
    }

    fn enter_primary_correction(&mut self) -> Result<(), CommonMoverFrameBlock> {
        if self.target_position_raw.is_some()
            && self.configuration.topology.sub_o
            && self.configuration.topology.sub_g
        {
            let plan = plan_primary_common_mover_target_correction(self.correction_request()?);
            self.stage = MachineStage::CommitPrimaryCorrection(plan);
        } else {
            self.enter_dispatch(0)?;
        }
        Ok(())
    }

    fn enter_after_primary_effect(&mut self) -> Result<(), CommonMoverFrameBlock> {
        if self.configuration.topology.sub_n {
            let before_raw = self
                .snapshot
                .sub_n_accumulator_raw
                .ok_or(CommonMoverFrameBlock::MissingSubNRuntime)?;
            let elapsed_q14 = (self.configuration.elapsed_micros as i32).wrapping_shl(14);
            let delta_raw = q31_mul(elapsed_q14, 0x8000) as i16;
            self.stage = MachineStage::CommitSubN {
                before_raw,
                after_raw: before_raw.wrapping_add(delta_raw),
            };
            return Ok(());
        }
        self.enter_dispatch(0)
    }

    fn enter_dispatch(&mut self, from_index: usize) -> Result<(), CommonMoverFrameBlock> {
        for index in from_index..self.dispatch_phases.len() {
            if let Some(phase) = self.dispatch_phases[index] {
                self.stage = MachineStage::InvokeComponent {
                    phase,
                    next_index: index + 1,
                };
                return Ok(());
            }
        }
        self.enter_tail(0)
    }

    fn enter_tail(&mut self, from_index: usize) -> Result<(), CommonMoverFrameBlock> {
        let next = match CommonMoverPostDispatchPlan::next_phase(
            self.configuration.topology,
            from_index,
            RetailRuntimeValue::Unresolved,
        ) {
            Ok(next) => next,
            Err(CommonMoverPostDispatchBlock::UnresolvedSubATargetSpeed) => {
                CommonMoverPostDispatchPlan::next_phase(
                    self.configuration.topology,
                    from_index,
                    self.sub_a_target_speed_for_tail()?,
                )
                .map_err(CommonMoverFrameBlock::PostDispatch)?
            }
        };
        let Some((phase, next_tail_index)) = next else {
            return self.enter_secondary_correction();
        };

        match phase {
            CommonMoverPostDispatchPhase::SubC => {
                let descriptor = self.configuration.sub_c_descriptor.ok_or(
                    CommonMoverFrameBlock::MissingRequiredDescriptor(
                        CommonMoverFrameRequiredDescriptor::SubC,
                    ),
                )?;
                self.stage = MachineStage::SampleSubC {
                    point_raw: sub_c_surface_sample_point_raw(
                        self.snapshot.position_raw,
                        self.snapshot.body_up_q31,
                        descriptor.offset_sample,
                    ),
                    expected_wave: sub_c_uses_wave_surface(
                        descriptor,
                        self.snapshot.attached_cargo_mass,
                    ),
                    next_tail_index,
                };
            }
            CommonMoverPostDispatchPhase::SubA => {
                let descriptor = self.configuration.sub_a_descriptor.ok_or(
                    CommonMoverFrameBlock::MissingRequiredDescriptor(
                        CommonMoverFrameRequiredDescriptor::SubA,
                    ),
                )?;
                let runtime = self.resolved_sub_a_runtime()?;
                let target_speed_raw = match runtime.target_speed_raw() {
                    RetailRuntimeValue::Known(value) => value,
                    RetailRuntimeValue::Unresolved => {
                        return Err(CommonMoverFrameBlock::PostDispatch(
                            CommonMoverPostDispatchBlock::UnresolvedSubATargetSpeed,
                        ));
                    }
                };
                let mut velocity_raw = self.snapshot.velocity_raw;
                let propulsion_applied = apply_sub_a_propulsion_raw(
                    descriptor,
                    target_speed_raw,
                    runtime.direction_multiplier(),
                    runtime.drive_scale_percent(),
                    self.snapshot.body_forward_q31,
                    &mut velocity_raw,
                    self.configuration.elapsed_micros,
                );
                self.stage = MachineStage::CommitSubA {
                    velocity_raw,
                    propulsion_applied,
                    next_tail_index,
                };
            }
            CommonMoverPostDispatchPhase::SubB => {
                let descriptor = self.configuration.sub_b_descriptor.ok_or(
                    CommonMoverFrameBlock::MissingRequiredDescriptor(
                        CommonMoverFrameRequiredDescriptor::SubB,
                    ),
                )?;
                let mut velocity_raw = self.snapshot.velocity_raw;
                apply_sub_b_lateral_raw(
                    descriptor,
                    self.snapshot.body_right_q31,
                    &mut velocity_raw,
                    self.configuration.elapsed_micros,
                );
                self.stage = MachineStage::CommitSubB {
                    velocity_raw,
                    next_tail_index,
                };
            }
        }
        Ok(())
    }

    fn enter_secondary_correction(&mut self) -> Result<(), CommonMoverFrameBlock> {
        if self.configuration.topology.sub_o && self.configuration.topology.sub_g {
            let Some(_) = self.target_position_raw else {
                return Err(
                    CommonMoverFrameBlock::NullTargetSecondaryCorrectionUsesIndeterminateContext,
                );
            };
            let plan = plan_secondary_common_mover_target_correction(
                self.correction_request()?,
                self.snapshot.sub_o_link_raw,
                self.configuration.elapsed_micros,
                self.snapshot.sub_g_state_byte_3c,
            );
            self.stage = MachineStage::CommitSecondaryCorrection(plan);
        } else {
            self.stage = MachineStage::ReturnOne;
        }
        Ok(())
    }

    fn preflight_post_sub_d_runtime(&self) -> Result<(), CommonMoverFrameBlock> {
        if self.configuration.topology.sub_f && self.snapshot.sub_f_smoothed_raw.is_none() {
            return Err(CommonMoverFrameBlock::MissingSubFRuntime);
        }
        if self.configuration.topology.sub_k && self.snapshot.sub_k_smoothed_raw.is_none() {
            return Err(CommonMoverFrameBlock::MissingSubKRuntime);
        }
        Ok(())
    }

    fn sub_a_target_speed_for_tail(
        &self,
    ) -> Result<RetailRuntimeValue<i32>, CommonMoverFrameBlock> {
        Ok(self.resolved_sub_a_runtime()?.target_speed_raw())
    }

    fn resolved_sub_a_runtime(&self) -> Result<SubAPropulsionRuntime, CommonMoverFrameBlock> {
        match self.snapshot.sub_a_runtime {
            RetailRuntimeValue::Known(Some(runtime)) => Ok(runtime),
            RetailRuntimeValue::Known(None) => Err(CommonMoverFrameBlock::MissingSubARuntime),
            RetailRuntimeValue::Unresolved => Err(CommonMoverFrameBlock::UnresolvedSubARuntime),
        }
    }

    fn target_prelude_request(
        &self,
        target_state: WanderNearPrivateState,
        tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    ) -> Result<CommonMoverTargetPreludeRequest, CommonMoverFrameBlock> {
        let topology = self.configuration.topology;
        let sub_a = if topology.sub_a {
            Some(CommonMoverPreludeSubA {
                descriptor: self.configuration.sub_a_descriptor,
                runtime: self.resolved_sub_a_runtime()?,
            })
        } else {
            None
        };
        let sub_d = topology
            .sub_d
            .then_some(self.configuration.sub_d_descriptor)
            .flatten();
        let mismatch_needs_sub_d = topology.sub_d
            && !topology.sub_i
            && sub_a.is_some_and(|value| {
                target_state.direction != value.runtime.direction_multiplier()
            });
        if mismatch_needs_sub_d && sub_d.is_none() {
            return Err(CommonMoverFrameBlock::MissingRequiredDescriptor(
                CommonMoverFrameRequiredDescriptor::SubD,
            ));
        }

        Ok(self.target_prelude_request_with_topology(
            target_state,
            tracked_target,
            RetailRuntimeValue::Known(CommonMoverTargetPreludeTopology {
                sub_a,
                sub_d,
                sub_f: topology.sub_f,
                sub_g: topology.sub_g,
                sub_i: topology.sub_i,
                sub_l: topology.sub_l,
            }),
        ))
    }

    fn target_prelude_request_with_topology(
        &self,
        target_state: WanderNearPrivateState,
        tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
        topology: RetailRuntimeValue<CommonMoverTargetPreludeTopology>,
    ) -> CommonMoverTargetPreludeRequest {
        CommonMoverTargetPreludeRequest {
            target_state,
            tracked_target,
            controlled_position_raw: self.snapshot.position_raw,
            heading_raw: self.snapshot.heading_raw,
            roll_raw: self.snapshot.roll_raw,
            elapsed_micros: self.configuration.elapsed_micros,
            topology,
        }
    }

    fn correction_request(
        &self,
    ) -> Result<CommonMoverTargetCorrectionRequest, CommonMoverFrameBlock> {
        Ok(CommonMoverTargetCorrectionRequest {
            entity: self.snapshot.controlled_entity_lookup.map(|lookup| {
                CommonMoverTargetCorrectionEntity {
                    position_raw: self.snapshot.position_raw,
                    attitude_raw_28: lookup.attitude_raw_28,
                }
            }),
            target_context_present: self.configuration.target_resource_context_present,
            target_position_raw: self.target_position_raw.ok_or(
                CommonMoverFrameBlock::NullTargetSecondaryCorrectionUsesIndeterminateContext,
            )?,
            sub_g_runtime_angle_raw: self.snapshot.sub_g_runtime_angle_raw,
        })
    }

    fn finish_transition(
        &mut self,
        transition: Result<(), CommonMoverFrameBlock>,
    ) -> Result<(), CommonMoverFrameAdvanceError> {
        match transition {
            Ok(()) => Ok(()),
            Err(block) => self.block(block),
        }
    }

    fn block<T>(
        &mut self,
        block: CommonMoverFrameBlock,
    ) -> Result<T, CommonMoverFrameAdvanceError> {
        self.stage = MachineStage::Blocked(block);
        Err(CommonMoverFrameAdvanceError::Block(block))
    }
}

fn committed(
    completion: CommonMoverFrameResume,
    expected: CommonMoverFrameCommitPhase,
) -> Result<CommonMoverFrameSnapshot, CommonMoverFrameAdvanceError> {
    match completion {
        CommonMoverFrameResume::Committed { phase, snapshot } if phase == expected => Ok(snapshot),
        CommonMoverFrameResume::Committed { phase, .. } => {
            Err(CommonMoverFrameProtocolError::ExpectedCommitResult {
                expected,
                actual: Some(phase),
            }
            .into())
        }
        _ => Err(CommonMoverFrameProtocolError::ExpectedCommitResult {
            expected,
            actual: None,
        }
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUB_A: SubAPropulsionDescriptor = SubAPropulsionDescriptor {
        acceleration_raw: 1_500,
        overspeed_correction_raw: -3_000,
        target_speed_base_raw: 250,
    };
    const SUB_B: SubBLateralDescriptor = SubBLateralDescriptor {
        projection_threshold_rate_raw: 10_000,
        correction_rate_raw: 1_000,
    };
    const SUB_C: HoverLiftConfig = HoverLiftConfig {
        base_clearance_raw: 150,
        lift_range_raw: 125,
        strength_raw: 0x0090_0000,
        near_boost_range_raw: 100,
        damping_range_raw: 200,
        use_wave_surface: false,
        offset_sample: false,
    };
    const SUB_D: CommonMoverPreludeSubD = CommonMoverPreludeSubD {
        steering_divisor_raw: 20,
        couple_yaw_into_roll: false,
    };

    fn snapshot() -> CommonMoverFrameSnapshot {
        CommonMoverFrameSnapshot {
            controlled_entity_lookup: Some(CommonMoverControlledEntityLookup {
                attitude_raw_28: 0,
            }),
            position_raw: [100, 300, 200],
            velocity_raw: [0; 3],
            heading_raw: 0,
            roll_raw: 0,
            body_up_q31: [0, i32::MAX, 0],
            body_right_q31: [i32::MAX, 0, 0],
            body_forward_q31: [0, 0, i32::MAX],
            attached_cargo_mass: 0,
            sub_a_runtime: RetailRuntimeValue::Known(Some(
                SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(250), 1, 100),
            )),
            sub_f_smoothed_raw: Some(0),
            sub_g_runtime_angle_raw: 100,
            sub_g_state_byte_3c: 0,
            sub_k_smoothed_raw: Some(0),
            sub_n_accumulator_raw: Some(10),
            sub_o_link_raw: 0x1234_5678,
        }
    }

    fn target_private() -> WanderNearPrivateState {
        WanderNearPrivateState {
            target_position_raw: [400, 300, 500],
            tracked_entity_handle: 0,
            direction: 1,
            reversal_timer_ms: 0,
        }
    }

    fn request(topology: CommonMoverComponentTopology, target: bool) -> CommonMoverFrameRequest {
        CommonMoverFrameRequest {
            configuration: CommonMoverFrameConfiguration {
                controlled_entity_handle: 0x04f4_0001,
                topology,
                dispatch_mode: CommonMoverDispatchMode::Normal,
                elapsed_micros: 20_000,
                sub_a_descriptor: topology.sub_a.then_some(SUB_A),
                sub_b_descriptor: topology.sub_b.then_some(SUB_B),
                sub_c_descriptor: topology.sub_c.then_some(SUB_C),
                sub_d_descriptor: topology.sub_d.then_some(SUB_D),
                target_resource_context_present: target,
            },
            initial_snapshot: snapshot(),
            target_private: target.then(target_private),
            tracked_target: RetailRuntimeValue::Known(None),
        }
    }

    fn commit(machine: &mut CommonMoverFrameMachine) {
        commit_with(machine, snapshot());
    }

    fn commit_with(machine: &mut CommonMoverFrameMachine, snapshot: CommonMoverFrameSnapshot) {
        let phase = match machine.poll() {
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitTargetPrelude(_)) => {
                CommonMoverFrameCommitPhase::TargetPrelude
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitPostSubDWrites(_)) => {
                CommonMoverFrameCommitPhase::PostSubDWrites
            }
            CommonMoverFramePoll::Action(
                CommonMoverFrameAction::CommitPrimaryTargetCorrection(_),
            ) => CommonMoverFrameCommitPhase::PrimaryTargetCorrection,
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubNAccumulator {
                ..
            }) => CommonMoverFrameCommitPhase::SubNAccumulator,
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubC { .. }) => {
                CommonMoverFrameCommitPhase::SubC
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubA { .. }) => {
                CommonMoverFrameCommitPhase::SubA
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubB { .. }) => {
                CommonMoverFrameCommitPhase::SubB
            }
            CommonMoverFramePoll::Action(
                CommonMoverFrameAction::CommitSecondaryTargetCorrection(_),
            ) => CommonMoverFrameCommitPhase::SecondaryTargetCorrection,
            other => panic!("expected pending commit, got {other:?}"),
        };
        machine
            .resume(CommonMoverFrameResume::Committed { phase, snapshot })
            .unwrap();
    }

    fn component_action(phase: CommonMoverDispatchPhase) -> CommonMoverFramePoll {
        CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeComponent {
            phase,
            controlled_entity_handle: 0x04f4_0001,
            elapsed_micros: 20_000,
        })
    }

    #[test]
    fn target_rejection_is_the_only_zero_terminal() {
        let topology = CommonMoverComponentTopology::default();
        let mut rejected_request = request(topology, true);
        rejected_request
            .target_private
            .as_mut()
            .unwrap()
            .tracked_entity_handle = 7;
        rejected_request.tracked_target =
            RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: crate::entity_collision_state::RetailStateWord::exact(0),
                position_raw: [0; 3],
                velocity_raw: [0; 3],
            }));

        let machine =
            CommonMoverFrameMachine::start(rejected_request, || panic!("unexpected RNG")).unwrap();
        assert_eq!(
            machine.poll(),
            CommonMoverFramePoll::ReturnZero(CommonMoverTargetPreludeZero::TrackedEntityInactive)
        );

        let machine =
            CommonMoverFrameMachine::start(request(topology, false), || panic!("unexpected RNG"))
                .unwrap();
        assert_eq!(machine.poll(), CommonMoverFramePoll::ReturnOne);
    }

    #[test]
    fn inactive_target_returns_zero_before_unresolved_sub_a_is_read() {
        let topology = CommonMoverComponentTopology {
            sub_a: true,
            ..CommonMoverComponentTopology::default()
        };
        let mut rejected_request = request(topology, true);
        rejected_request.initial_snapshot.sub_a_runtime = RetailRuntimeValue::Unresolved;
        rejected_request
            .target_private
            .as_mut()
            .unwrap()
            .tracked_entity_handle = 7;
        rejected_request.tracked_target =
            RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: crate::entity_collision_state::RetailStateWord::exact(0),
                position_raw: [0; 3],
                velocity_raw: [0; 3],
            }));

        let machine =
            CommonMoverFrameMachine::start(rejected_request, || panic!("unexpected RNG")).unwrap();

        assert_eq!(
            machine.poll(),
            CommonMoverFramePoll::ReturnZero(CommonMoverTargetPreludeZero::TrackedEntityInactive)
        );
    }

    #[test]
    fn d_dispatch_and_cab_tail_pause_in_exact_order() {
        let topology = CommonMoverComponentTopology {
            sub_a: true,
            sub_b: true,
            sub_c: true,
            sub_d: true,
            sub_f: true,
            sub_k: true,
            sub_l: true,
            ..CommonMoverComponentTopology::default()
        };
        let mut machine =
            CommonMoverFrameMachine::start(request(topology, true), || panic!("unexpected RNG"))
                .unwrap();

        assert!(matches!(
            machine.poll(),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitTargetPrelude(_))
        ));
        commit(&mut machine);
        assert!(matches!(
            machine.poll(),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeSubD { .. })
        ));
        machine
            .resume(CommonMoverFrameResume::SubDReturned {
                result_raw: 160,
                snapshot: snapshot(),
            })
            .unwrap();
        assert!(matches!(
            machine.poll(),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitPostSubDWrites(_))
        ));
        commit(&mut machine);

        for expected in [
            CommonMoverDispatchPhase::SubF,
            CommonMoverDispatchPhase::SubK,
            CommonMoverDispatchPhase::SubL,
        ] {
            assert_eq!(machine.poll(), component_action(expected));
            machine
                .resume(CommonMoverFrameResume::ComponentReturned {
                    phase: expected,
                    snapshot: snapshot(),
                })
                .unwrap();
        }

        assert!(matches!(
            machine.poll(),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::SampleSubCSurface {
                use_wave_surface: false,
                ..
            })
        ));
        machine
            .resume(CommonMoverFrameResume::SubCSurfaceSampled(
                SubCSurfaceSample::Terrain { terrain_y_raw: 0 },
            ))
            .unwrap();
        assert!(matches!(
            machine.poll(),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubC { .. })
        ));
        commit(&mut machine);
        assert!(matches!(
            machine.poll(),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubA { .. })
        ));
        commit(&mut machine);
        assert!(matches!(
            machine.poll(),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubB { .. })
        ));
        commit(&mut machine);
        assert_eq!(machine.poll(), CommonMoverFramePoll::ReturnOne);
    }

    #[test]
    fn g_o_route_orders_both_corrections_and_suppresses_cab_tail() {
        let topology = CommonMoverComponentTopology {
            sub_g: true,
            sub_n: true,
            sub_o: true,
            ..CommonMoverComponentTopology::default()
        };
        let mut machine =
            CommonMoverFrameMachine::start(request(topology, true), || panic!("unexpected RNG"))
                .unwrap();
        commit(&mut machine);

        assert!(matches!(
            machine.poll(),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitPrimaryTargetCorrection(_))
        ));
        commit(&mut machine);
        assert!(matches!(
            machine.poll(),
            CommonMoverFramePoll::Action(
                CommonMoverFrameAction::InvokePrimaryCorrectionEffect { .. }
            )
        ));
        machine
            .resume(CommonMoverFrameResume::PrimaryCorrectionEffectReturned(
                snapshot(),
            ))
            .unwrap();
        assert!(matches!(
            machine.poll(),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubNAccumulator { .. })
        ));
        commit(&mut machine);

        assert_eq!(
            machine.poll(),
            component_action(CommonMoverDispatchPhase::SubG)
        );
        machine
            .resume(CommonMoverFrameResume::ComponentReturned {
                phase: CommonMoverDispatchPhase::SubG,
                snapshot: snapshot(),
            })
            .unwrap();
        assert!(matches!(
            machine.poll(),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSecondaryTargetCorrection(
                _
            ))
        ));
        commit(&mut machine);
        assert_eq!(machine.poll(), CommonMoverFramePoll::ReturnOne);
    }

    #[test]
    fn null_target_with_g_o_fails_closed_at_indeterminate_retail_local() {
        let topology = CommonMoverComponentTopology {
            sub_g: true,
            sub_o: true,
            ..CommonMoverComponentTopology::default()
        };
        let mut machine =
            CommonMoverFrameMachine::start(request(topology, false), || panic!("unexpected RNG"))
                .unwrap();
        assert_eq!(
            machine.poll(),
            component_action(CommonMoverDispatchPhase::SubG)
        );
        let error = machine
            .resume(CommonMoverFrameResume::ComponentReturned {
                phase: CommonMoverDispatchPhase::SubG,
                snapshot: snapshot(),
            })
            .unwrap_err();
        assert_eq!(
            error,
            CommonMoverFrameAdvanceError::Block(
                CommonMoverFrameBlock::NullTargetSecondaryCorrectionUsesIndeterminateContext
            )
        );
        let block = CommonMoverFrameBlock::NullTargetSecondaryCorrectionUsesIndeterminateContext;
        assert_eq!(machine.poll(), CommonMoverFramePoll::Blocked(block));
        assert_eq!(
            machine.resume(CommonMoverFrameResume::ComponentReturned {
                phase: CommonMoverDispatchPhase::SubG,
                snapshot: snapshot(),
            }),
            Err(CommonMoverFrameAdvanceError::Protocol(
                CommonMoverFrameProtocolError::MachineBlocked(block)
            ))
        );
    }

    #[test]
    fn callback_resume_requires_the_exact_pending_phase_and_fresh_snapshot() {
        let topology = CommonMoverComponentTopology {
            sub_h: true,
            ..CommonMoverComponentTopology::default()
        };
        let mut machine =
            CommonMoverFrameMachine::start(request(topology, false), || panic!("unexpected RNG"))
                .unwrap();
        assert_eq!(
            machine.poll(),
            component_action(CommonMoverDispatchPhase::SubH)
        );
        assert_eq!(
            machine.resume(CommonMoverFrameResume::ComponentReturned {
                phase: CommonMoverDispatchPhase::SubI,
                snapshot: snapshot(),
            }),
            Err(CommonMoverFrameAdvanceError::Protocol(
                CommonMoverFrameProtocolError::ExpectedComponentResult {
                    expected: CommonMoverDispatchPhase::SubH,
                    actual: Some(CommonMoverDispatchPhase::SubI),
                }
            ))
        );
    }

    #[test]
    fn sub_d_runs_before_initial_f_runtime_is_required() {
        let topology = CommonMoverComponentTopology {
            sub_d: true,
            sub_f: true,
            sub_k: true,
            ..CommonMoverComponentTopology::default()
        };
        let mut initial_request = request(topology, true);
        initial_request.initial_snapshot.sub_f_smoothed_raw = None;
        initial_request.initial_snapshot.sub_k_smoothed_raw = None;
        let mut machine =
            CommonMoverFrameMachine::start(initial_request, || panic!("unexpected RNG")).unwrap();
        commit_with(&mut machine, {
            let mut before_sub_d = snapshot();
            before_sub_d.sub_f_smoothed_raw = None;
            before_sub_d.sub_k_smoothed_raw = None;
            before_sub_d
        });
        assert!(matches!(
            machine.poll(),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeSubD { .. })
        ));

        machine
            .resume(CommonMoverFrameResume::SubDReturned {
                result_raw: 160,
                snapshot: snapshot(),
            })
            .unwrap();
        assert!(matches!(
            machine.poll(),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitPostSubDWrites(_))
        ));
    }

    #[test]
    fn post_sub_d_evidence_block_is_durable_and_never_replays_callback() {
        let topology = CommonMoverComponentTopology {
            sub_d: true,
            sub_f: true,
            ..CommonMoverComponentTopology::default()
        };
        let mut machine =
            CommonMoverFrameMachine::start(request(topology, true), || panic!("unexpected RNG"))
                .unwrap();
        commit(&mut machine);
        assert!(matches!(
            machine.poll(),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeSubD { .. })
        ));

        let mut after_sub_d = snapshot();
        after_sub_d.sub_f_smoothed_raw = None;
        let block = CommonMoverFrameBlock::MissingSubFRuntime;
        assert_eq!(
            machine.resume(CommonMoverFrameResume::SubDReturned {
                result_raw: 160,
                snapshot: after_sub_d,
            }),
            Err(CommonMoverFrameAdvanceError::Block(block))
        );
        assert_eq!(machine.poll(), CommonMoverFramePoll::Blocked(block));
        assert_eq!(
            machine.resume(CommonMoverFrameResume::SubDReturned {
                result_raw: 160,
                snapshot: snapshot(),
            }),
            Err(CommonMoverFrameAdvanceError::Protocol(
                CommonMoverFrameProtocolError::MachineBlocked(block)
            ))
        );
    }

    #[test]
    fn wrong_commit_tag_preserves_the_pending_action() {
        let topology = CommonMoverComponentTopology::default();
        let mut machine =
            CommonMoverFrameMachine::start(request(topology, true), || panic!("unexpected RNG"))
                .unwrap();
        let pending = machine.poll();

        assert_eq!(
            machine.resume(CommonMoverFrameResume::Committed {
                phase: CommonMoverFrameCommitPhase::SubA,
                snapshot: snapshot(),
            }),
            Err(CommonMoverFrameAdvanceError::Protocol(
                CommonMoverFrameProtocolError::ExpectedCommitResult {
                    expected: CommonMoverFrameCommitPhase::TargetPrelude,
                    actual: Some(CommonMoverFrameCommitPhase::SubA),
                }
            ))
        );
        assert_eq!(machine.poll(), pending);
        commit(&mut machine);
        assert_eq!(machine.poll(), CommonMoverFramePoll::ReturnOne);
    }

    #[test]
    fn sub_g_suppresses_tail_without_reading_unresolved_sub_a() {
        let topology = CommonMoverComponentTopology {
            sub_a: true,
            sub_g: true,
            ..CommonMoverComponentTopology::default()
        };
        let mut initial_request = request(topology, false);
        initial_request.initial_snapshot.sub_a_runtime = RetailRuntimeValue::Unresolved;
        let mut machine =
            CommonMoverFrameMachine::start(initial_request, || panic!("unexpected RNG")).unwrap();

        assert_eq!(
            machine.poll(),
            component_action(CommonMoverDispatchPhase::SubG)
        );
        let mut after_sub_g = snapshot();
        after_sub_g.sub_a_runtime = RetailRuntimeValue::Unresolved;
        machine
            .resume(CommonMoverFrameResume::ComponentReturned {
                phase: CommonMoverDispatchPhase::SubG,
                snapshot: after_sub_g,
            })
            .unwrap();
        assert_eq!(machine.poll(), CommonMoverFramePoll::ReturnOne);
    }

    #[test]
    fn sub_a_gate_is_rechecked_after_sub_c_commits() {
        let topology = CommonMoverComponentTopology {
            sub_a: true,
            sub_c: true,
            ..CommonMoverComponentTopology::default()
        };
        let mut machine =
            CommonMoverFrameMachine::start(request(topology, false), || panic!("unexpected RNG"))
                .unwrap();
        assert!(matches!(
            machine.poll(),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::SampleSubCSurface { .. })
        ));
        machine
            .resume(CommonMoverFrameResume::SubCSurfaceSampled(
                SubCSurfaceSample::Terrain { terrain_y_raw: 0 },
            ))
            .unwrap();
        assert!(matches!(
            machine.poll(),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubC { .. })
        ));

        let mut after_sub_c = snapshot();
        after_sub_c.sub_a_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(0), 1, 100),
        ));
        commit_with(&mut machine, after_sub_c);

        assert_eq!(machine.poll(), CommonMoverFramePoll::ReturnOne);
    }

    #[test]
    fn missing_sub_n_after_primary_effect_blocks_without_replaying_effect() {
        let topology = CommonMoverComponentTopology {
            sub_g: true,
            sub_n: true,
            sub_o: true,
            ..CommonMoverComponentTopology::default()
        };
        let mut machine =
            CommonMoverFrameMachine::start(request(topology, true), || panic!("unexpected RNG"))
                .unwrap();
        commit(&mut machine);
        commit(&mut machine);
        assert!(matches!(
            machine.poll(),
            CommonMoverFramePoll::Action(
                CommonMoverFrameAction::InvokePrimaryCorrectionEffect { .. }
            )
        ));

        let mut after_effect = snapshot();
        after_effect.sub_n_accumulator_raw = None;
        let block = CommonMoverFrameBlock::MissingSubNRuntime;
        assert_eq!(
            machine.resume(CommonMoverFrameResume::PrimaryCorrectionEffectReturned(
                after_effect
            )),
            Err(CommonMoverFrameAdvanceError::Block(block))
        );
        assert_eq!(machine.poll(), CommonMoverFramePoll::Blocked(block));
        assert_eq!(
            machine.resume(CommonMoverFrameResume::PrimaryCorrectionEffectReturned(
                snapshot()
            )),
            Err(CommonMoverFrameAdvanceError::Protocol(
                CommonMoverFrameProtocolError::MachineBlocked(block)
            ))
        );
    }

    #[test]
    fn callback_snapshot_can_suppress_a_later_sub_a_phase() {
        let topology = CommonMoverComponentTopology {
            sub_a: true,
            sub_h: true,
            ..CommonMoverComponentTopology::default()
        };
        let mut machine =
            CommonMoverFrameMachine::start(request(topology, false), || panic!("unexpected RNG"))
                .unwrap();
        assert_eq!(
            machine.poll(),
            component_action(CommonMoverDispatchPhase::SubH)
        );

        let mut after_sub_h = snapshot();
        after_sub_h.sub_a_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(0), 1, 100),
        ));
        machine
            .resume(CommonMoverFrameResume::ComponentReturned {
                phase: CommonMoverDispatchPhase::SubH,
                snapshot: after_sub_h,
            })
            .unwrap();
        assert_eq!(machine.poll(), CommonMoverFramePoll::ReturnOne);
    }
}
