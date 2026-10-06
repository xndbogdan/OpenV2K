//! Detached shared `"Chase Target"` task at `FUN_00403360/00403490`.
//!
//! Search-and-Attack's pursuing variant and Guard Location's pursuing style
//! both install this same 5,000-ms slot-0 family.  This module retains the
//! constructor suffix, callback decision matrix, mutable 0x24-byte target
//! record, and generic scheduler result ordering without attaching them to a
//! live entity.
//!
//! The common mover remains an explicit seven-argument adapter.  In
//! particular, this module does not replace it with direct target steering.

use crate::actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskVisit, PreparedActorTask};
use crate::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use crate::wander_near_location::{
    WanderNearLifetime, WanderNearLifetimeStatus, WanderNearPrivateState,
    WANDER_NEAR_NO_TRACKED_ENTITY,
};
use crate::wrapped_axis_range::{within_wrapped_axis_range, WrappedAxisRange};

pub mod live_primary;

pub const CHASE_TARGET_CONSTRUCTOR_ADDRESS: u32 = 0x0040_3360;
pub const CHASE_TARGET_TICK_ADDRESS: u32 = 0x0040_3490;
pub const CHASE_TARGET_COMMON_MOVER_ADDRESS: u32 = 0x0040_1430;
pub const CHASE_TARGET_ROUTE_PREDICATE_ADDRESS: u32 = 0x0042_3030;
pub const CHASE_TARGET_COMPONENT_MODE_WRITER_ADDRESS: u32 = 0x0042_4390;
pub const CHASE_TARGET_LIFETIME_MS: u32 = 5_000;

pub const CHASE_TARGET_INVALID_TARGET_SINGLETON_ADDRESS: u32 = 0x004B_E0D0;
pub const CHASE_TARGET_IN_RANGE_ZERO_MOVER_SINGLETON_ADDRESS: u32 = 0x004B_E0D8;
pub const CHASE_TARGET_OUT_OF_RANGE_SINGLETON_ADDRESS: u32 = 0x004B_E0E0;
pub const CHASE_TARGET_OWNER_TRANSITION_TAG: u32 = 0x0000_9C01;

pub const CHASE_TARGET_INNER_PROXIMITY_RAW: i32 = 0x0380;
pub const CHASE_TARGET_OUTER_PROXIMITY_RAW: i32 = 0x0400;
pub const CHASE_TARGET_ENABLED_COMPONENT_MODE: u8 = 1;
pub const CHASE_TARGET_ENABLED_COMPONENT_PHASE_RAW: u32 = 0x1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaseTargetConstructorSuffixError {
    UnresolvedComponentTopology,
    UnresolvedSubADescriptor,
    MissingSubADescriptor,
}

/// Successful post-allocation writes performed by `FUN_00403360`.
///
/// Disassembly of the retail executable disproves the earlier RNG hypothesis:
/// this constructor consumes no random word.  When Sub-A exists, it resets
/// runtime dword `+0x00` to signed `(descriptor[+4] * 4) / 3`.  When Sub-F
/// exists, it calls `FUN_00424390(component, 1)`, which writes mode byte one
/// and phase `0x1000`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChaseTargetConstructorSuffix {
    sub_a_target_speed_raw: Option<i32>,
    enable_sub_f: bool,
}

impl ChaseTargetConstructorSuffix {
    pub const fn sub_a_target_speed_raw(self) -> Option<i32> {
        self.sub_a_target_speed_raw
    }

    pub const fn enables_sub_f(self) -> bool {
        self.enable_sub_f
    }

    pub const fn sub_f_mode(self) -> Option<(u8, u32)> {
        if self.enable_sub_f {
            Some((
                CHASE_TARGET_ENABLED_COMPONENT_MODE,
                CHASE_TARGET_ENABLED_COMPONENT_PHASE_RAW,
            ))
        } else {
            None
        }
    }
}

fn chase_target_constructor_suffix(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<ChaseTargetConstructorSuffix, ChaseTargetConstructorSuffixError> {
    let topology = match metadata.common_mover_topology {
        RetailRuntimeValue::Known(topology) => topology,
        RetailRuntimeValue::Unresolved => {
            return Err(ChaseTargetConstructorSuffixError::UnresolvedComponentTopology);
        }
    };
    let sub_a_target_speed_raw = if topology.sub_a {
        let descriptor = match metadata.sub_a_propulsion_descriptor {
            RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
            RetailRuntimeValue::Known(None) => {
                return Err(ChaseTargetConstructorSuffixError::MissingSubADescriptor);
            }
            RetailRuntimeValue::Unresolved => {
                return Err(ChaseTargetConstructorSuffixError::UnresolvedSubADescriptor);
            }
        };
        let base = i32::from(descriptor.target_speed_base_raw);
        Some(base * 4 / 3)
    } else {
        None
    };

    Ok(ChaseTargetConstructorSuffix {
        sub_a_target_speed_raw,
        enable_sub_f: topology.sub_f,
    })
}

/// Successful task allocation paired with the still-unpublished constructor
/// suffix.
///
/// Call [`Self::apply_suffix`] exactly once.  It runs the proven component
/// writes before returning the prepared wrapper for publication, so callers
/// cannot accidentally publish first and reset components afterward. The
/// captured owner identity is forwarded with those writes so a live binder
/// cannot silently source metadata from one entity and mutate another.
#[derive(Debug)]
pub struct PreparedChaseTargetTask<T = ChaseTargetTaskState> {
    task: PreparedActorTask<T>,
    owner_id: u32,
    suffix: ChaseTargetConstructorSuffix,
}

impl PreparedChaseTargetTask {
    pub fn map_task<T>(
        self,
        map: impl FnOnce(ChaseTargetTaskState) -> T,
    ) -> PreparedChaseTargetTask<T> {
        PreparedChaseTargetTask {
            task: self.task.map(map),
            owner_id: self.owner_id,
            suffix: self.suffix,
        }
    }
}

impl<T> PreparedChaseTargetTask<T> {
    pub const fn owner_id(&self) -> u32 {
        self.owner_id
    }

    pub const fn suffix(&self) -> ChaseTargetConstructorSuffix {
        self.suffix
    }

    pub fn apply_suffix(
        self,
        apply: impl FnOnce(u32, ChaseTargetConstructorSuffix),
    ) -> PreparedActorTask<T> {
        apply(self.owner_id, self.suffix);
        self.task
    }
}

/// Exact callback-consumed state after task/private allocation succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChaseTargetTaskState {
    private_state: WanderNearPrivateState,
    lifetime: WanderNearLifetime,
}

impl ChaseTargetTaskState {
    /// Complete `FUN_00403360` after allocation succeeds.
    ///
    /// The private target begins at the controlled entity's current position,
    /// retains the explicit target handle, direction `+1`, and timer zero.
    /// Component topology is read only after allocation; unresolved consumed
    /// metadata therefore fails closed without publishing a task.
    pub fn prepare_after_allocation(
        owner_id: u32,
        current_position_raw: [i16; 3],
        target_id: u32,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<PreparedChaseTargetTask, ChaseTargetConstructorSuffixError> {
        let suffix = chase_target_constructor_suffix(metadata)?;
        Ok(PreparedChaseTargetTask {
            task: PreparedActorTask::new(Self {
                private_state: WanderNearPrivateState::tracked_entity(
                    current_position_raw,
                    target_id,
                ),
                lifetime: WanderNearLifetime::new(),
            }),
            owner_id,
            suffix,
        })
    }

    #[cfg(test)]
    const fn from_parts(private_state: WanderNearPrivateState, elapsed_ms: u32) -> Self {
        Self {
            private_state,
            lifetime: WanderNearLifetime::from_elapsed_ms(elapsed_ms),
        }
    }

    pub const fn private_state(self) -> WanderNearPrivateState {
        self.private_state
    }

    pub const fn target_id(self) -> u32 {
        self.private_state.tracked_entity_handle
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.lifetime.elapsed_ms()
    }

    pub fn before_callback(&mut self, elapsed_micros: u32) -> ChaseTargetCallbackPrefix {
        let lifetime_status = match self.lifetime.advance_frame(elapsed_micros) {
            WanderNearLifetimeStatus::Active => ChaseTargetLifetimeStatus::WithinLifetime,
            WanderNearLifetimeStatus::OwnerTransitionDue => {
                ChaseTargetLifetimeStatus::OwnerTransitionDue
            }
        };
        ChaseTargetCallbackPrefix { lifetime_status }
    }

    pub const fn stage_callback(self) -> ChaseTargetCallbackStage {
        ChaseTargetCallbackStage {
            private_state: self.private_state,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChaseTargetCallbackStage {
    private_state: WanderNearPrivateState,
}

impl ChaseTargetCallbackStage {
    pub const fn private_state(self) -> WanderNearPrivateState {
        self.private_state
    }

    pub fn private_state_mut(&mut self) -> &mut WanderNearPrivateState {
        &mut self.private_state
    }

    pub fn commit(self, surviving_state: &mut ChaseTargetTaskState) {
        surviving_state.private_state = self.private_state;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaseTargetLifetimeStatus {
    WithinLifetime,
    OwnerTransitionDue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChaseTargetCallbackPrefix {
    pub lifetime_status: ChaseTargetLifetimeStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ChaseTargetTaggedSingleton {
    InvalidTarget = CHASE_TARGET_INVALID_TARGET_SINGLETON_ADDRESS,
    InRangeZeroMover = CHASE_TARGET_IN_RANGE_ZERO_MOVER_SINGLETON_ADDRESS,
    OutOfRange = CHASE_TARGET_OUT_OF_RANGE_SINGLETON_ADDRESS,
}

impl ChaseTargetTaggedSingleton {
    pub const fn address(self) -> u32 {
        self as u32
    }

    pub const fn tag(self) -> u32 {
        CHASE_TARGET_OWNER_TRANSITION_TAG
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaseTargetTargetRuntimeState {
    Live { position_raw: [i16; 3] },
    Missing,
    Inactive,
    Dying,
}

impl ChaseTargetTargetRuntimeState {
    const fn live_position_raw(self) -> Option<[i16; 3]> {
        match self {
            Self::Live { position_raw } => Some(position_raw),
            Self::Missing | Self::Inactive | Self::Dying => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaseTargetCommonMoverReturn {
    Zero,
    NonZero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaseTargetCommonMoverPath {
    InRange,
    OutOfRange,
}

/// Exact presence cases tested before the X/Z proximity writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaseTargetProximityControl {
    Disabled,
    EnabledWithoutSource,
    EnabledWithSource { source_value_raw: i16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChaseTargetControllerWrite {
    pub primary_raw: i32,
    pub secondary_raw: Option<i32>,
}

#[derive(Debug)]
pub struct ChaseTargetCommonMoverRequest<'a, MovementState, ControllerContext> {
    pub visit: ActorTaskVisit,
    pub entity_id: u32,
    pub movement_state: &'a mut MovementState,
    pub controller_context: &'a mut ControllerContext,
    pub target_state: &'a mut WanderNearPrivateState,
    pub elapsed_micros: u32,
    pub scheduler_mode: u32,
}

#[derive(Debug)]
pub struct ChaseTargetCallbackRequest<'a, MovementState, ControllerContext> {
    pub visit: ActorTaskVisit,
    pub entity_id: u32,
    pub owner_position_raw: [i16; 3],
    pub route_range: WrappedAxisRange,
    pub proximity_control: ChaseTargetProximityControl,
    pub movement_state: &'a mut MovementState,
    pub controller_context: &'a mut ControllerContext,
    pub elapsed_micros: u32,
    pub scheduler_mode: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChaseTargetCallbackCall {
    pub visit: ActorTaskVisit,
    pub entity_id: u32,
    pub target_id: u32,
    pub elapsed_micros: u32,
    pub scheduler_mode: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChaseTargetPostMoverPositions {
    pub owner_position_raw: [i16; 3],
    pub target_position_raw: [i16; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChaseTargetCallbackError<TargetError, MoverError> {
    TargetValidation {
        call: ChaseTargetCallbackCall,
        error: TargetError,
    },
    CommonMover {
        call: ChaseTargetCallbackCall,
        path: ChaseTargetCommonMoverPath,
        error: MoverError,
    },
    PostMoverPositionLookup {
        call: ChaseTargetCallbackCall,
        error: TargetError,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaseTargetCallbackResult {
    Continue {
        controller_write: Option<ChaseTargetControllerWrite>,
    },
    Tagged(ChaseTargetTaggedSingleton),
}

/// Evaluate `FUN_00403490` against one detached callback stage.
///
/// The route predicate is the already-closed `FUN_00423030` strict wrapped
/// cube.  An out-of-range target still reaches the common mover, but its
/// Boolean return is ignored before returning the out-of-range singleton.
/// Only an in-range, nonzero mover result reaches fresh owner/target position
/// lookups and the X/Z proximity writes. Reusing the pre-mover positions here
/// is observably wrong because the mover may change either entity first.
///
/// Portable adapters must supply detached movement/controller staging values
/// and commit them only when this evaluator returns `Ok`. That keeps a
/// fail-closed lookup/mover error from leaking external mutations while the
/// heterogeneous dispatcher discards this task-private stage.
pub fn evaluate_chase_target_callback<MovementState, ControllerContext, TargetError, MoverError>(
    stage: &mut ChaseTargetCallbackStage,
    request: ChaseTargetCallbackRequest<'_, MovementState, ControllerContext>,
    mut validate_target: impl FnMut(u32) -> Result<ChaseTargetTargetRuntimeState, TargetError>,
    mut common_mover: impl FnMut(
        ChaseTargetCommonMoverRequest<'_, MovementState, ControllerContext>,
    ) -> Result<ChaseTargetCommonMoverReturn, MoverError>,
    mut resolve_post_mover_positions: impl FnMut(
        u32,
        u32,
    )
        -> Result<ChaseTargetPostMoverPositions, TargetError>,
) -> Result<ChaseTargetCallbackResult, ChaseTargetCallbackError<TargetError, MoverError>> {
    let ChaseTargetCallbackRequest {
        visit,
        entity_id,
        owner_position_raw,
        route_range,
        proximity_control,
        movement_state,
        controller_context,
        elapsed_micros,
        scheduler_mode,
    } = request;
    let target_id = stage.private_state.tracked_entity_handle;
    let call = ChaseTargetCallbackCall {
        visit,
        entity_id,
        target_id,
        elapsed_micros,
        scheduler_mode,
    };

    if target_id == WANDER_NEAR_NO_TRACKED_ENTITY {
        return Ok(ChaseTargetCallbackResult::Tagged(
            ChaseTargetTaggedSingleton::InvalidTarget,
        ));
    }
    let target_state = validate_target(target_id)
        .map_err(|error| ChaseTargetCallbackError::TargetValidation { call, error })?;
    let Some(target_position_raw) = target_state.live_position_raw() else {
        return Ok(ChaseTargetCallbackResult::Tagged(
            ChaseTargetTaggedSingleton::InvalidTarget,
        ));
    };

    let path = if within_wrapped_axis_range(route_range, owner_position_raw, target_position_raw) {
        ChaseTargetCommonMoverPath::InRange
    } else {
        ChaseTargetCommonMoverPath::OutOfRange
    };
    let mover_return = common_mover(ChaseTargetCommonMoverRequest {
        visit,
        entity_id,
        movement_state,
        controller_context,
        target_state: stage.private_state_mut(),
        elapsed_micros,
        scheduler_mode,
    })
    .map_err(|error| ChaseTargetCallbackError::CommonMover { call, path, error })?;

    if path == ChaseTargetCommonMoverPath::OutOfRange {
        return Ok(ChaseTargetCallbackResult::Tagged(
            ChaseTargetTaggedSingleton::OutOfRange,
        ));
    }
    if mover_return == ChaseTargetCommonMoverReturn::Zero {
        return Ok(ChaseTargetCallbackResult::Tagged(
            ChaseTargetTaggedSingleton::InRangeZeroMover,
        ));
    }

    if proximity_control == ChaseTargetProximityControl::Disabled {
        return Ok(ChaseTargetCallbackResult::Continue {
            controller_write: None,
        });
    }
    let post_mover_positions = resolve_post_mover_positions(entity_id, target_id)
        .map_err(|error| ChaseTargetCallbackError::PostMoverPositionLookup { call, error })?;
    let controller_write = chase_target_controller_write(
        stage,
        proximity_control,
        post_mover_positions.owner_position_raw,
        post_mover_positions.target_position_raw,
    );
    Ok(ChaseTargetCallbackResult::Continue { controller_write })
}

fn chase_target_controller_write(
    stage: &mut ChaseTargetCallbackStage,
    proximity_control: ChaseTargetProximityControl,
    owner_position_raw: [i16; 3],
    target_position_raw: [i16; 3],
) -> Option<ChaseTargetControllerWrite> {
    let ChaseTargetProximityControl::EnabledWithSource { source_value_raw } = proximity_control
    else {
        return None;
    };
    let delta_x = wrapped_absolute_delta(owner_position_raw[0], target_position_raw[0]);
    let delta_z = wrapped_absolute_delta(owner_position_raw[2], target_position_raw[2]);
    let source_value_raw = i32::from(source_value_raw);

    if delta_x < CHASE_TARGET_OUTER_PROXIMITY_RAW && delta_z < CHASE_TARGET_OUTER_PROXIMITY_RAW {
        if delta_x < CHASE_TARGET_INNER_PROXIMITY_RAW && delta_z < CHASE_TARGET_INNER_PROXIMITY_RAW
        {
            stage.private_state.direction = -1;
            Some(ChaseTargetControllerWrite {
                primary_raw: source_value_raw,
                secondary_raw: Some(-1),
            })
        } else {
            Some(ChaseTargetControllerWrite {
                primary_raw: 1,
                secondary_raw: None,
            })
        }
    } else {
        Some(ChaseTargetControllerWrite {
            primary_raw: source_value_raw,
            secondary_raw: None,
        })
    }
}

fn wrapped_absolute_delta(first: i16, second: i16) -> i32 {
    i32::from(first.wrapping_sub(second)).abs()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaseTargetTransitionReason {
    TaggedCallbackResult(ChaseTargetTaggedSingleton),
    LifetimeExpired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChaseTargetTransitionRequest {
    pub slot: ActorTaskSlot,
    pub task_id: ActorTaskId,
    pub reason: ChaseTargetTransitionReason,
    pub committed_prefix: ChaseTargetCallbackPrefix,
}

pub const fn chase_target_transition_after_unwind(
    visit: ActorTaskVisit,
    committed_prefix: ChaseTargetCallbackPrefix,
    callback_result: ChaseTargetCallbackResult,
) -> Option<ChaseTargetTransitionRequest> {
    let reason = match callback_result {
        ChaseTargetCallbackResult::Tagged(singleton) => {
            ChaseTargetTransitionReason::TaggedCallbackResult(singleton)
        }
        ChaseTargetCallbackResult::Continue { .. } => match committed_prefix.lifetime_status {
            ChaseTargetLifetimeStatus::WithinLifetime => return None,
            ChaseTargetLifetimeStatus::OwnerTransitionDue => {
                ChaseTargetTransitionReason::LifetimeExpired
            }
        },
    };
    Some(ChaseTargetTransitionRequest {
        slot: visit.slot,
        task_id: visit.task_id,
        reason,
        committed_prefix,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_task_owner::ActorTaskOwner;
    use crate::entity_collision_state::CommonMoverComponentTopology;
    use std::cell::Cell;
    use v2k_formats::collision::SubAPropulsionDescriptor;

    fn metadata(
        topology: RetailRuntimeValue<CommonMoverComponentTopology>,
        descriptor: RetailRuntimeValue<Option<SubAPropulsionDescriptor>>,
    ) -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            common_mover_topology: topology,
            sub_a_propulsion_descriptor: descriptor,
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn prepared_state(current_position_raw: [i16; 3], target_id: u32) -> ChaseTargetTaskState {
        let metadata = metadata(
            RetailRuntimeValue::Known(CommonMoverComponentTopology::default()),
            RetailRuntimeValue::Unresolved,
        );
        let prepared = ChaseTargetTaskState::prepare_after_allocation(
            7,
            current_position_raw,
            target_id,
            &metadata,
        )
        .unwrap();
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            prepared.apply_suffix(|owner_id, suffix| {
                assert_eq!(owner_id, 7);
                assert_eq!(suffix.sub_a_target_speed_raw(), None);
                assert!(!suffix.enables_sub_f());
            }),
        );
        *owner.state_in_slot(ActorTaskSlot::Primary).unwrap()
    }

    fn visit() -> ActorTaskVisit {
        let mut owner = ActorTaskOwner::new();
        let task_id = owner.replace_prepared(ActorTaskSlot::Primary, PreparedActorTask::new(()));
        ActorTaskVisit {
            slot: ActorTaskSlot::Primary,
            task_id,
        }
    }

    fn request<'a, M, C>(
        movement_state: &'a mut M,
        controller_context: &'a mut C,
        owner_position_raw: [i16; 3],
        route_range: WrappedAxisRange,
        proximity_control: ChaseTargetProximityControl,
    ) -> ChaseTargetCallbackRequest<'a, M, C> {
        ChaseTargetCallbackRequest {
            visit: visit(),
            entity_id: 7,
            owner_position_raw,
            route_range,
            proximity_control,
            movement_state,
            controller_context,
            elapsed_micros: 20_000,
            scheduler_mode: 1,
        }
    }

    #[test]
    fn constructor_is_deterministic_and_pairs_suffix_before_publication() {
        let topology = CommonMoverComponentTopology {
            sub_a: true,
            sub_f: true,
            ..CommonMoverComponentTopology::default()
        };
        let metadata = metadata(
            RetailRuntimeValue::Known(topology),
            RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                acceleration_raw: 1,
                overspeed_correction_raw: 2,
                target_speed_base_raw: -2,
            })),
        );
        let prepared =
            ChaseTargetTaskState::prepare_after_allocation(7, [1, 2, 3], 9, &metadata).unwrap();
        assert_eq!(prepared.owner_id(), 7);
        assert_eq!(prepared.suffix().sub_a_target_speed_raw(), Some(-2));
        assert_eq!(
            prepared.suffix().sub_f_mode(),
            Some((1, CHASE_TARGET_ENABLED_COMPONENT_PHASE_RAW))
        );

        let suffix_applied = Cell::new(false);
        let task = prepared.apply_suffix(|owner_id, suffix| {
            assert_eq!(owner_id, 7);
            assert_eq!(suffix.sub_a_target_speed_raw(), Some(-2));
            suffix_applied.set(true);
        });
        assert!(suffix_applied.get());
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, task);
        let state = owner.state_in_slot(ActorTaskSlot::Primary).unwrap();
        assert_eq!(state.private_state().target_position_raw, [1, 2, 3]);
        assert_eq!(state.target_id(), 9);
        assert_eq!(state.private_state().direction, 1);
    }

    #[test]
    fn constructor_uses_signed_truncation_and_fails_closed_on_consumed_metadata() {
        let topology = CommonMoverComponentTopology {
            sub_a: true,
            ..CommonMoverComponentTopology::default()
        };
        let signed = metadata(
            RetailRuntimeValue::Known(topology),
            RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                acceleration_raw: 0,
                overspeed_correction_raw: 0,
                target_speed_base_raw: 0x0300,
            })),
        );
        assert_eq!(
            ChaseTargetTaskState::prepare_after_allocation(7, [0; 3], 1, &signed)
                .unwrap()
                .suffix()
                .sub_a_target_speed_raw(),
            Some(0x0400)
        );
        assert!(matches!(
            ChaseTargetTaskState::prepare_after_allocation(
                7,
                [0; 3],
                1,
                &metadata(
                    RetailRuntimeValue::Unresolved,
                    RetailRuntimeValue::Unresolved
                ),
            ),
            Err(ChaseTargetConstructorSuffixError::UnresolvedComponentTopology)
        ));
        assert!(matches!(
            ChaseTargetTaskState::prepare_after_allocation(
                7,
                [0; 3],
                1,
                &metadata(
                    RetailRuntimeValue::Known(topology),
                    RetailRuntimeValue::Unresolved,
                ),
            ),
            Err(ChaseTargetConstructorSuffixError::UnresolvedSubADescriptor)
        ));
        assert!(matches!(
            ChaseTargetTaskState::prepare_after_allocation(
                7,
                [0; 3],
                1,
                &metadata(
                    RetailRuntimeValue::Known(topology),
                    RetailRuntimeValue::Known(None),
                ),
            ),
            Err(ChaseTargetConstructorSuffixError::MissingSubADescriptor)
        ));
    }

    #[test]
    fn invalid_target_short_circuits_without_mover() {
        let state = prepared_state([0; 3], 0);
        let mut stage = state.stage_callback();
        let mut movement = ();
        let mut controller = ();
        let validation_calls = Cell::new(0);
        let mover_calls = Cell::new(0);

        let result = evaluate_chase_target_callback(
            &mut stage,
            request(
                &mut movement,
                &mut controller,
                [0; 3],
                WrappedAxisRange::Unbounded,
                ChaseTargetProximityControl::Disabled,
            ),
            |_target| {
                validation_calls.set(validation_calls.get() + 1);
                Ok::<_, ()>(ChaseTargetTargetRuntimeState::Live {
                    position_raw: [0; 3],
                })
            },
            |_request| {
                mover_calls.set(mover_calls.get() + 1);
                Ok::<_, ()>(ChaseTargetCommonMoverReturn::NonZero)
            },
            |_, _| panic!("an invalid target cannot reach post-mover lookups"),
        )
        .unwrap();

        assert_eq!(
            result,
            ChaseTargetCallbackResult::Tagged(ChaseTargetTaggedSingleton::InvalidTarget)
        );
        assert_eq!(validation_calls.get(), 0);
        assert_eq!(mover_calls.get(), 0);
    }

    #[test]
    fn out_of_range_calls_mover_but_ignores_its_boolean_result() {
        for mover_return in [
            ChaseTargetCommonMoverReturn::Zero,
            ChaseTargetCommonMoverReturn::NonZero,
        ] {
            let state = prepared_state([0; 3], 9);
            let mut stage = state.stage_callback();
            let mut movement = ();
            let mut controller = ();
            let mover_calls = Cell::new(0);

            let result = evaluate_chase_target_callback(
                &mut stage,
                request(
                    &mut movement,
                    &mut controller,
                    [0; 3],
                    WrappedAxisRange::strict(0x100).unwrap(),
                    ChaseTargetProximityControl::EnabledWithSource {
                        source_value_raw: 77,
                    },
                ),
                |_target| {
                    Ok::<_, ()>(ChaseTargetTargetRuntimeState::Live {
                        position_raw: [0x100, 0, 0],
                    })
                },
                |request| {
                    mover_calls.set(mover_calls.get() + 1);
                    request.target_state.target_position_raw = [7, 8, 9];
                    Ok::<_, ()>(mover_return)
                },
                |_, _| panic!("the out-of-range path returns before post-mover lookups"),
            )
            .unwrap();

            assert_eq!(
                result,
                ChaseTargetCallbackResult::Tagged(ChaseTargetTaggedSingleton::OutOfRange)
            );
            assert_eq!(mover_calls.get(), 1);
            assert_eq!(stage.private_state().target_position_raw, [7, 8, 9]);
        }
    }

    #[test]
    fn in_range_zero_mover_returns_its_distinct_singleton() {
        let state = prepared_state([0; 3], 9);
        let mut stage = state.stage_callback();
        let mut movement = ();
        let mut controller = ();
        let result = evaluate_chase_target_callback(
            &mut stage,
            request(
                &mut movement,
                &mut controller,
                [0; 3],
                WrappedAxisRange::Unbounded,
                ChaseTargetProximityControl::Disabled,
            ),
            |_target| {
                Ok::<_, ()>(ChaseTargetTargetRuntimeState::Live {
                    position_raw: [0; 3],
                })
            },
            |_request| Ok::<_, ()>(ChaseTargetCommonMoverReturn::Zero),
            |_, _| panic!("a zero mover return cannot reach post-mover lookups"),
        )
        .unwrap();

        assert_eq!(
            result,
            ChaseTargetCallbackResult::Tagged(ChaseTargetTaggedSingleton::InRangeZeroMover)
        );
    }

    #[test]
    fn proximity_bands_are_strict_and_ignore_y() {
        let cases = [
            (
                [0x037f, i16::MAX, 0x037f],
                ChaseTargetControllerWrite {
                    primary_raw: -3,
                    secondary_raw: Some(-1),
                },
                -1,
            ),
            (
                [0x0380, i16::MIN, 0],
                ChaseTargetControllerWrite {
                    primary_raw: 1,
                    secondary_raw: None,
                },
                1,
            ),
            (
                [0x03ff, 123, 0x03ff],
                ChaseTargetControllerWrite {
                    primary_raw: 1,
                    secondary_raw: None,
                },
                1,
            ),
            (
                [0x0400, 0, 0],
                ChaseTargetControllerWrite {
                    primary_raw: -3,
                    secondary_raw: None,
                },
                1,
            ),
        ];

        for (target_position_raw, expected_write, expected_direction) in cases {
            let state = prepared_state([0; 3], 9);
            let mut stage = state.stage_callback();
            let mut movement = ();
            let mut controller = ();
            let result = evaluate_chase_target_callback(
                &mut stage,
                request(
                    &mut movement,
                    &mut controller,
                    [0; 3],
                    WrappedAxisRange::Unbounded,
                    ChaseTargetProximityControl::EnabledWithSource {
                        source_value_raw: -3,
                    },
                ),
                |_target| {
                    Ok::<_, ()>(ChaseTargetTargetRuntimeState::Live {
                        position_raw: [0; 3],
                    })
                },
                |_request| Ok::<_, ()>(ChaseTargetCommonMoverReturn::NonZero),
                |entity_id, target_id| {
                    assert_eq!((entity_id, target_id), (7, 9));
                    Ok::<_, ()>(ChaseTargetPostMoverPositions {
                        owner_position_raw: [0; 3],
                        target_position_raw,
                    })
                },
            )
            .unwrap();
            assert_eq!(
                result,
                ChaseTargetCallbackResult::Continue {
                    controller_write: Some(expected_write),
                }
            );
            assert_eq!(stage.private_state().direction, expected_direction);
        }
    }

    #[test]
    fn proximity_uses_wrapped_xz_and_requires_both_component_pointers() {
        for proximity_control in [
            ChaseTargetProximityControl::Disabled,
            ChaseTargetProximityControl::EnabledWithoutSource,
        ] {
            let state = prepared_state([i16::MIN, 0, 0], 9);
            let mut stage = state.stage_callback();
            let mut movement = ();
            let mut controller = ();
            let post_mover_lookups = Cell::new(0);
            let result = evaluate_chase_target_callback(
                &mut stage,
                request(
                    &mut movement,
                    &mut controller,
                    [i16::MIN, 0, 0],
                    WrappedAxisRange::strict(2).unwrap(),
                    proximity_control,
                ),
                |_target| {
                    Ok::<_, ()>(ChaseTargetTargetRuntimeState::Live {
                        position_raw: [i16::MAX, 0, 0],
                    })
                },
                |_request| Ok::<_, ()>(ChaseTargetCommonMoverReturn::NonZero),
                |_, _| {
                    post_mover_lookups.set(post_mover_lookups.get() + 1);
                    Ok::<_, ()>(ChaseTargetPostMoverPositions {
                        owner_position_raw: [i16::MIN, 0, 0],
                        target_position_raw: [i16::MAX, 0, 0],
                    })
                },
            )
            .unwrap();
            assert_eq!(
                result,
                ChaseTargetCallbackResult::Continue {
                    controller_write: None,
                }
            );
            assert_eq!(stage.private_state().direction, 1);
            assert_eq!(
                post_mover_lookups.get(),
                usize::from(proximity_control != ChaseTargetProximityControl::Disabled)
            );
        }
    }

    #[test]
    fn mover_error_retains_call_and_path_for_fail_closed_dispatch() {
        let state = prepared_state([0; 3], 9);
        let mut stage = state.stage_callback();
        let mut movement = ();
        let mut controller = ();
        let result = evaluate_chase_target_callback(
            &mut stage,
            request(
                &mut movement,
                &mut controller,
                [0; 3],
                WrappedAxisRange::Unbounded,
                ChaseTargetProximityControl::Disabled,
            ),
            |_target| {
                Ok::<_, &'static str>(ChaseTargetTargetRuntimeState::Live {
                    position_raw: [0; 3],
                })
            },
            |request| {
                request.target_state.direction = -1;
                Err::<ChaseTargetCommonMoverReturn, _>("unresolved mover")
            },
            |_, _| panic!("a mover error cannot reach post-mover lookups"),
        );
        assert!(matches!(
            result,
            Err(ChaseTargetCallbackError::CommonMover {
                path: ChaseTargetCommonMoverPath::InRange,
                error: "unresolved mover",
                ..
            })
        ));
        // The evaluator exposes the staged write. The heterogeneous dispatcher
        // is responsible for discarding this stage on an adapter error.
        assert_eq!(stage.private_state().direction, -1);
    }

    #[test]
    fn post_mover_lookup_error_is_explicit_and_leaves_only_staged_mutation() {
        let state = prepared_state([0; 3], 9);
        let mut stage = state.stage_callback();
        let mut movement = ();
        let mut controller = ();
        let result = evaluate_chase_target_callback(
            &mut stage,
            request(
                &mut movement,
                &mut controller,
                [0; 3],
                WrappedAxisRange::Unbounded,
                ChaseTargetProximityControl::EnabledWithSource {
                    source_value_raw: 7,
                },
            ),
            |_target| {
                Ok::<_, &'static str>(ChaseTargetTargetRuntimeState::Live {
                    position_raw: [0; 3],
                })
            },
            |request| {
                request.target_state.direction = -1;
                Ok::<_, ()>(ChaseTargetCommonMoverReturn::NonZero)
            },
            |_, _| Err("post-mover position unavailable"),
        );
        assert!(matches!(
            result,
            Err(ChaseTargetCallbackError::PostMoverPositionLookup {
                call: ChaseTargetCallbackCall {
                    entity_id: 7,
                    target_id: 9,
                    ..
                },
                error: "post-mover position unavailable",
            })
        ));
        // The dispatcher discards this stage, while the adapter contract
        // likewise requires its generic movement/controller values to be
        // transactional staging values.
        assert_eq!(stage.private_state().direction, -1);
    }

    #[test]
    fn tagged_callback_preempts_an_expired_lifetime() {
        let visit = visit();
        let prefix = ChaseTargetCallbackPrefix {
            lifetime_status: ChaseTargetLifetimeStatus::OwnerTransitionDue,
        };
        let request = chase_target_transition_after_unwind(
            visit,
            prefix,
            ChaseTargetCallbackResult::Tagged(ChaseTargetTaggedSingleton::OutOfRange),
        )
        .unwrap();
        assert_eq!(
            request.reason,
            ChaseTargetTransitionReason::TaggedCallbackResult(
                ChaseTargetTaggedSingleton::OutOfRange
            )
        );
    }

    #[test]
    fn lifetime_is_strictly_more_than_five_seconds() {
        let private_state = WanderNearPrivateState::tracked_entity([0; 3], 9);
        let mut state = ChaseTargetTaskState::from_parts(private_state, 4_999);
        assert_eq!(
            state.before_callback(1_000).lifetime_status,
            ChaseTargetLifetimeStatus::WithinLifetime
        );
        assert_eq!(state.elapsed_ms(), CHASE_TARGET_LIFETIME_MS);
        let prefix = state.before_callback(1_000);
        assert_eq!(
            prefix.lifetime_status,
            ChaseTargetLifetimeStatus::OwnerTransitionDue
        );
        let request = chase_target_transition_after_unwind(
            visit(),
            prefix,
            ChaseTargetCallbackResult::Continue {
                controller_write: None,
            },
        )
        .unwrap();
        assert_eq!(request.reason, ChaseTargetTransitionReason::LifetimeExpired);
    }

    #[test]
    fn singleton_addresses_remain_pointer_distinct_but_share_the_tag() {
        let values = [
            ChaseTargetTaggedSingleton::InvalidTarget,
            ChaseTargetTaggedSingleton::InRangeZeroMover,
            ChaseTargetTaggedSingleton::OutOfRange,
        ];
        assert_eq!(
            values.map(ChaseTargetTaggedSingleton::address),
            [0x004B_E0D0, 0x004B_E0D8, 0x004B_E0E0]
        );
        assert_eq!(
            values.map(ChaseTargetTaggedSingleton::tag),
            [CHASE_TARGET_OWNER_TRANSITION_TAG; 3]
        );
    }
}
