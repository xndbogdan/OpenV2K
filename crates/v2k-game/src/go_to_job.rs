//! Exact read-only class-54 `"Go To Job"` target and task-install plan.
//!
//! Retail initializer `FUN_0040AF90` first calls target selector
//! `FUN_004235F0`, then clears task slots 1 and 2, and finally installs slot 0
//! through `FUN_00403650` with a 5,000-ms lifetime and the selected handle.
//! This module closes that selection/setup contract and the successful
//! first-world type-8/type-9 constructor suffix through the shared
//! mutation-safe three-slot owner. [`crate::go_to_job_owner`] models the exact
//! post-allocation private state plus detached callback and post-unwind phases.
//! The heterogeneous live dispatcher and common-mover bridge remain separate.

use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::actor_task_owner::{
    ActorTaskMutation, ActorTaskOwner, ActorTaskPrepareError, ActorTaskSlot, PreparedActorTask,
};
use crate::common_mover::{
    actor_abdi::{ActorAbdiEntityKind, FIRST_WORLD_SCIENTIST_ENTITY_TYPE},
    SubAPropulsionRuntime,
};
use crate::entity::Entity;
use crate::entity_collision_state::{
    CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord,
};
use crate::job_nearby::{
    candidate_accepts_job, candidate_state_is_live, JobCapacityState, JobNearbyCandidate,
    JobNearbyEvaluationError,
};
use crate::shared_target_route::shared_target_route_speed_raw;
use crate::wrapped_axis_range::{within_wrapped_axis_range, WrappedAxisRange};
use v2k_formats::collision::SubAPropulsionDescriptor;

pub const GO_TO_JOB_INITIALIZER_ADDRESS: u32 = 0x0040_AF90;
pub const GO_TO_JOB_TARGET_SELECTOR_ADDRESS: u32 = 0x0042_35F0;
pub const GO_TO_JOB_TASK_CONSTRUCTOR_ADDRESS: u32 = 0x0040_3650;
pub const GO_TO_JOB_TASK_INITIALIZER_ADDRESS: u32 = 0x0040_1350;
pub const GO_TO_JOB_TASK_TICK_ADDRESS: u32 = 0x0040_3780;
pub const GO_TO_JOB_TASK_LIFETIME_MS: u32 = 5_000;
pub const GO_TO_JOB_FAST_OWNER_CAPABILITY_BIT: u32 = 0x0000_0800;
pub const GO_TO_JOB_FAST_TARGET_CAPABILITY_BIT: u32 = 0x0000_0020;
pub const GO_TO_JOB_FIRST_WORLD_SCIENTIST_TYPE: u16 = FIRST_WORLD_SCIENTIST_ENTITY_TYPE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoToJobConstructorSuffixError {
    WrongEntityType { actual: u16 },
    UnresolvedComponentTopology,
    MissingSubA,
    SubHPresent,
    SubGPresent,
    SubFPresent,
    UnresolvedSubADescriptor,
    MissingSubADescriptor,
}

/// Exact successful `FUN_00403650` component suffix for the admitted actors'
/// A/B/D/I topology.
///
/// After task preparation, shared `FUN_00406070` consumes one RNG word and
/// writes Sub-A direction `+1` plus its randomized target speed. The
/// class-specific tail then overwrites only target speed with signed
/// `(descriptor[+4] * 4) / 3` before publishing the prepared wrapper. The
/// The admitted authored records have Sub-A and none of the preceding
/// Sub-H/G/F branches, so accepting any of those branches would silently skip
/// a real constructor side effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoToJobConstructorSuffix {
    sub_a_target_speed_base_raw: i16,
    sub_a_target_speed_raw: i32,
}

impl GoToJobConstructorSuffix {
    fn from_owner(owner: GoToJobOwner) -> Result<Self, GoToJobConstructorSuffixError> {
        if !matches!(
            ActorAbdiEntityKind::from_entity_type(owner.entity_type),
            Some(
                ActorAbdiEntityKind::Scientist
                    | ActorAbdiEntityKind::Peasant
                    | ActorAbdiEntityKind::Type7Worker
                    | ActorAbdiEntityKind::VulcanWorker
                    | ActorAbdiEntityKind::Type90Worker
                    | ActorAbdiEntityKind::Type79Worker
                    | ActorAbdiEntityKind::Type91Worker
                    | ActorAbdiEntityKind::Type86Person
                    | ActorAbdiEntityKind::Type78Person
                    | ActorAbdiEntityKind::Type95Person
            )
        ) {
            return Err(GoToJobConstructorSuffixError::WrongEntityType {
                actual: owner.entity_type,
            });
        }

        let topology = match owner.common_mover_topology {
            RetailRuntimeValue::Known(topology) => topology,
            RetailRuntimeValue::Unresolved => {
                return Err(GoToJobConstructorSuffixError::UnresolvedComponentTopology);
            }
        };
        if !topology.sub_a {
            return Err(GoToJobConstructorSuffixError::MissingSubA);
        }
        if topology.sub_h {
            return Err(GoToJobConstructorSuffixError::SubHPresent);
        }
        if topology.sub_g {
            return Err(GoToJobConstructorSuffixError::SubGPresent);
        }
        if topology.sub_f {
            return Err(GoToJobConstructorSuffixError::SubFPresent);
        }

        let descriptor = match owner.sub_a_propulsion_descriptor {
            RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
            RetailRuntimeValue::Known(None) => {
                return Err(GoToJobConstructorSuffixError::MissingSubADescriptor);
            }
            RetailRuntimeValue::Unresolved => {
                return Err(GoToJobConstructorSuffixError::UnresolvedSubADescriptor);
            }
        };
        Ok(Self {
            sub_a_target_speed_base_raw: descriptor.target_speed_base_raw,
            sub_a_target_speed_raw: shared_target_route_speed_raw(descriptor.target_speed_base_raw),
        })
    }

    pub const fn sub_a_target_speed_raw(self) -> i32 {
        self.sub_a_target_speed_raw
    }
}

/// One externally visible Sub-A write from the successful generic and
/// class-specific constructor suffixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GoToJobConstructorEffect {
    WriteSubADirection {
        direction_multiplier: i32,
    },
    WriteGenericSubATargetSpeed {
        target_speed_raw: i32,
        random_sample_low16: u16,
    },
    WriteGoToJobSubATargetSpeed {
        target_speed_raw: i32,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct GoToJobOwner {
    pub id: u32,
    pub position_raw: [i16; 3],
    pub capability_flags: RetailRuntimeValue<u32>,
    entity_type: u16,
    sub_a_propulsion_descriptor: RetailRuntimeValue<Option<SubAPropulsionDescriptor>>,
    common_mover_topology: RetailRuntimeValue<CommonMoverComponentTopology>,
}

impl GoToJobOwner {
    /// Bind selector-visible runtime state and constructor-visible Section-12
    /// evidence to one controlled entity snapshot.
    pub fn from_type_metadata(
        id: u32,
        position_raw: [i16; 3],
        capability_flags: RetailRuntimeValue<u32>,
        entity_type: u16,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Self {
        Self {
            id,
            position_raw,
            capability_flags,
            entity_type,
            sub_a_propulsion_descriptor: metadata.sub_a_propulsion_descriptor,
            common_mover_topology: metadata.common_mover_topology,
        }
    }

    /// Bind the controlled entity's task owner and mutable Sub-A runtime.
    ///
    /// The crate-internal live entity adapter must source both references from
    /// the entity record that produced this snapshot. Keeping this raw binder
    /// private prevents external callers from manufacturing a split binding.
    #[cfg(test)]
    pub(crate) fn bind_runtime<'a, T>(
        self,
        tasks: &'a mut ActorTaskOwner<T>,
        sub_a: &'a mut SubAPropulsionRuntime,
    ) -> GoToJobBoundRuntime<'a, T> {
        GoToJobBoundRuntime {
            owner_id: self.id,
            tasks,
            sub_a,
        }
    }

    /// Bind the task table and Sub-A runtime through one live entity borrow.
    ///
    /// The constructor transaction must not accept independently sourced
    /// mutable references: doing so could install a task on one entity while
    /// resetting another entity's propulsion component. The entity identity
    /// check also closes a preflight plan that became stale before apply.
    pub(crate) fn bind_entity_runtime(
        self,
        entity: &mut Entity,
    ) -> Result<GoToJobBoundRuntime<'_, ActorTaskRuntime>, GoToJobRuntimeBindError> {
        if entity.id != self.id {
            return Err(GoToJobRuntimeBindError::OwnerRuntimeMismatch {
                planned_owner_id: self.id,
                runtime_owner_id: entity.id,
            });
        }
        let Entity {
            id,
            actor_tasks,
            sub_a_propulsion_runtime,
            ..
        } = entity;
        let sub_a = match sub_a_propulsion_runtime {
            RetailRuntimeValue::Known(Some(sub_a)) => sub_a,
            RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => {
                return Err(GoToJobRuntimeBindError::SubARuntimeUnavailable);
            }
        };
        Ok(GoToJobBoundRuntime {
            owner_id: *id,
            tasks: actor_tasks,
            sub_a,
        })
    }

    /// Bind already-split task/Sub-A borrows after an owning adapter has
    /// authenticated that both came from `runtime_owner_id`.
    ///
    /// The explicit id check preserves the same pre-mutation mismatch gate
    /// as [`Self::bind_entity_runtime`]. The root adapter keeps construction of
    /// its component bundle crate-private so arbitrary callers cannot use this
    /// seam to claim split-reference provenance.
    pub(crate) fn bind_authenticated_parts_runtime<'a, T>(
        self,
        runtime_owner_id: u32,
        tasks: &'a mut ActorTaskOwner<T>,
        sub_a: &'a mut SubAPropulsionRuntime,
    ) -> Result<GoToJobBoundRuntime<'a, T>, GoToJobRuntimeBindError> {
        if runtime_owner_id != self.id {
            return Err(GoToJobRuntimeBindError::OwnerRuntimeMismatch {
                planned_owner_id: self.id,
                runtime_owner_id,
            });
        }
        Ok(GoToJobBoundRuntime {
            owner_id: runtime_owner_id,
            tasks,
            sub_a,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GoToJobCandidate {
    pub id: u32,
    pub position_raw: [i16; 3],
    pub state_flags: RetailStateWord,
    pub capability_flags: RetailRuntimeValue<u32>,
    pub capacity: RetailRuntimeValue<Option<JobCapacityState>>,
}

#[derive(Debug, Clone, Copy)]
pub struct GoToJobSetupRequest<'a> {
    pub owner: GoToJobOwner,
    /// Resolved entries in retail intrusive-list order, without the terminal
    /// sentinel. Equal-distance candidates retain this order.
    pub candidates_in_intrusive_order: &'a [GoToJobCandidate],
    pub range: WrappedAxisRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoToJobSetupError {
    OwnerCapabilityUnresolved,
    CandidateStateUnresolved { id: u32 },
    CandidateCapabilityUnresolved { id: u32 },
    CandidateCapacityUnresolved { id: u32 },
    ConstructorSuffix(GoToJobConstructorSuffixError),
}

#[derive(Debug, PartialEq, Eq)]
pub enum GoToJobApplyError<E> {
    OwnerRuntimeMismatch {
        planned_owner_id: u32,
        runtime_owner_id: u32,
    },
    Prepare(ActorTaskPrepareError<E>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GoToJobRuntimeBindError {
    OwnerRuntimeMismatch {
        planned_owner_id: u32,
        runtime_owner_id: u32,
    },
    SubARuntimeUnavailable,
}

/// Mutable task and Sub-A state carried together for one controlled entity.
///
/// The crate-internal live entity adapter must source both references from the
/// same entity record that supplied [`GoToJobOwner`]. The apply transaction
/// additionally checks the snapshot identity before either state owner can
/// mutate; that numeric check does not itself prove reference provenance.
pub struct GoToJobBoundRuntime<'a, T> {
    owner_id: u32,
    tasks: &'a mut ActorTaskOwner<T>,
    sub_a: &'a mut SubAPropulsionRuntime,
}

/// Proven inputs to the class-54 task constructor/initializer.
///
/// Construction remains private to [`plan_go_to_job_setup`], preventing a
/// caller from desynchronizing the selected target from the install payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoToJobTaskSpec {
    initial_position_raw: [i16; 3],
    target_id: Option<u32>,
}

impl GoToJobTaskSpec {
    pub const fn initial_position_raw(self) -> [i16; 3] {
        self.initial_position_raw
    }

    pub const fn target_id(self) -> Option<u32> {
        self.target_id
    }

    pub const fn constructor_address(self) -> u32 {
        GO_TO_JOB_TASK_CONSTRUCTOR_ADDRESS
    }

    pub const fn initializer_address(self) -> u32 {
        GO_TO_JOB_TASK_INITIALIZER_ADDRESS
    }

    pub const fn tick_address(self) -> u32 {
        GO_TO_JOB_TASK_TICK_ADDRESS
    }

    pub const fn lifetime_ms(self) -> u32 {
        GO_TO_JOB_TASK_LIFETIME_MS
    }
}

/// One-shot retail-ordered task transaction.
///
/// Its mutations are private and the apply method consumes the plan so a
/// caller cannot reorder, replay, or forge the clear/clear/install sequence.
#[derive(Debug, PartialEq, Eq)]
pub struct GoToJobSetupPlan {
    /// Result of the read-only selector, which retail computes before any task
    /// slot is cleared.
    target_id: Option<u32>,
    /// Mutations following target selection in exact retail order.
    ordered_mutations: [ActorTaskMutation<GoToJobTaskSpec>; 3],
    /// Successful post-allocation component suffix proven from the same
    /// controlled entity's exact admitted Section-12 metadata.
    constructor_suffix: GoToJobConstructorSuffix,
    owner_id: u32,
}

impl GoToJobSetupPlan {
    pub const fn target_id(&self) -> Option<u32> {
        self.target_id
    }

    /// Exact Sub-A target-speed reset proven by the constructor suffix.
    ///
    /// This read-only witness lets corpus tests pin authored Section-12
    /// metadata without exposing the crate-internal mutable binding seam.
    pub const fn constructor_sub_a_target_speed_raw(&self) -> i32 {
        self.constructor_suffix.sub_a_target_speed_raw
    }

    /// Apply clear-secondary, clear-tertiary, then try-install-primary without
    /// rollback.
    ///
    /// `prepare` owns allocation and class-54 task-state initialization. If it
    /// fails, the old primary task survives, both earlier clears remain
    /// committed, and Sub-A/RNG are untouched. Success runs the shared
    /// one-draw Sub-A suffix, then the class-specific target-speed overwrite,
    /// after preparation and before `FUN_0040A7A0` publishes the new primary.
    pub fn apply<T, E>(
        self,
        runtime: GoToJobBoundRuntime<'_, T>,
        next_shared_random: impl FnMut() -> u32,
        prepare: impl FnMut(GoToJobTaskSpec) -> Result<PreparedActorTask<T>, E>,
    ) -> Result<(), GoToJobApplyError<E>> {
        self.apply_with_retirement(runtime, next_shared_random, prepare, |_| {})
    }

    pub fn apply_with_retirement<T, E>(
        self,
        runtime: GoToJobBoundRuntime<'_, T>,
        next_shared_random: impl FnMut() -> u32,
        prepare: impl FnMut(GoToJobTaskSpec) -> Result<PreparedActorTask<T>, E>,
        retire: impl FnMut(&T),
    ) -> Result<(), GoToJobApplyError<E>> {
        let GoToJobBoundRuntime {
            owner_id,
            tasks,
            sub_a,
        } = runtime;
        self.apply_with_constructor_effects_and_retirement(
            tasks,
            owner_id,
            next_shared_random,
            |effect| match effect {
                GoToJobConstructorEffect::WriteSubADirection {
                    direction_multiplier,
                } => sub_a.set_direction_multiplier(direction_multiplier),
                GoToJobConstructorEffect::WriteGenericSubATargetSpeed {
                    target_speed_raw, ..
                } => sub_a.apply_shared_initializer_target_speed_write(target_speed_raw),
                GoToJobConstructorEffect::WriteGoToJobSubATargetSpeed { target_speed_raw } => {
                    sub_a.apply_go_to_job_reset(target_speed_raw);
                }
            },
            prepare,
            retire,
        )
    }

    #[cfg(test)]
    fn apply_with_constructor_effects<T, E>(
        self,
        owner: &mut ActorTaskOwner<T>,
        runtime_owner_id: u32,
        next_shared_random: impl FnMut() -> u32,
        apply_effect: impl FnMut(GoToJobConstructorEffect),
        prepare: impl FnMut(GoToJobTaskSpec) -> Result<PreparedActorTask<T>, E>,
    ) -> Result<(), GoToJobApplyError<E>> {
        self.apply_with_constructor_effects_and_retirement(
            owner,
            runtime_owner_id,
            next_shared_random,
            apply_effect,
            prepare,
            |_| {},
        )
    }

    fn apply_with_constructor_effects_and_retirement<T, E>(
        self,
        owner: &mut ActorTaskOwner<T>,
        runtime_owner_id: u32,
        mut next_shared_random: impl FnMut() -> u32,
        mut apply_effect: impl FnMut(GoToJobConstructorEffect),
        mut prepare: impl FnMut(GoToJobTaskSpec) -> Result<PreparedActorTask<T>, E>,
        mut retire: impl FnMut(&T),
    ) -> Result<(), GoToJobApplyError<E>> {
        if runtime_owner_id != self.owner_id {
            return Err(GoToJobApplyError::OwnerRuntimeMismatch {
                planned_owner_id: self.owner_id,
                runtime_owner_id,
            });
        }

        let [clear_secondary, clear_tertiary, install_primary] = self.ordered_mutations;
        let ActorTaskMutation::Clear {
            slot: ActorTaskSlot::Secondary,
        } = clear_secondary
        else {
            unreachable!("private Go-To-Job plan changed its first action")
        };
        owner.clear_slot_with_retirement(ActorTaskSlot::Secondary, &mut retire);
        let ActorTaskMutation::Clear {
            slot: ActorTaskSlot::Tertiary,
        } = clear_tertiary
        else {
            unreachable!("private Go-To-Job plan changed its second action")
        };
        owner.clear_slot_with_retirement(ActorTaskSlot::Tertiary, &mut retire);
        let ActorTaskMutation::TryInstall {
            slot: ActorTaskSlot::Primary,
            specification,
        } = install_primary
        else {
            unreachable!("private Go-To-Job plan changed its final action")
        };

        let prepared = prepare(specification).map_err(|error| {
            GoToJobApplyError::Prepare(ActorTaskPrepareError {
                action_index: 2,
                slot: ActorTaskSlot::Primary,
                error,
            })
        })?;

        // FUN_00406030 calls FUN_00406070 only after preparation succeeds.
        // Both admitted first-world actor records reach only the Sub-A branch:
        // draw first, then direction and randomized target. FUN_00403650
        // immediately overwrites only target speed with its signed 4/3 value
        // before publication.
        let random_sample_low16 = next_shared_random() as u16;
        let random_byte = i32::from(random_sample_low16 >> 8);
        let base = i32::from(self.constructor_suffix.sub_a_target_speed_base_raw);
        let randomized_target_speed_raw =
            base.wrapping_add(random_byte.wrapping_mul(base) / 0x0A00);
        apply_effect(GoToJobConstructorEffect::WriteSubADirection {
            direction_multiplier: 1,
        });
        apply_effect(GoToJobConstructorEffect::WriteGenericSubATargetSpeed {
            target_speed_raw: randomized_target_speed_raw,
            random_sample_low16,
        });
        apply_effect(GoToJobConstructorEffect::WriteGoToJobSubATargetSpeed {
            target_speed_raw: self.constructor_suffix.sub_a_target_speed_raw,
        });
        owner.replace_prepared_with_retirement(ActorTaskSlot::Primary, prepared, &mut retire);
        Ok(())
    }
}

/// Select the nearest compatible job and return the exact subsequent slot and
/// proven first-world type-8/type-9 constructor transaction. No task graph or
/// component runtime is changed by this planning function.
pub fn plan_go_to_job_setup(
    request: GoToJobSetupRequest<'_>,
) -> Result<GoToJobSetupPlan, GoToJobSetupError> {
    let owner_capabilities = match request.owner.capability_flags {
        RetailRuntimeValue::Known(flags) => flags,
        RetailRuntimeValue::Unresolved => {
            return Err(GoToJobSetupError::OwnerCapabilityUnresolved);
        }
    };
    let mut best = None;
    let mut best_distance = i32::MAX;

    for candidate in request.candidates_in_intrusive_order {
        if candidate.id == request.owner.id {
            continue;
        }
        if !candidate_state_is_live(candidate.id, candidate.state_flags).map_err(map_job_error)? {
            continue;
        }
        if !within_wrapped_axis_range(
            request.range,
            request.owner.position_raw,
            candidate.position_raw,
        ) {
            continue;
        }

        let fast_owner = owner_capabilities & GO_TO_JOB_FAST_OWNER_CAPABILITY_BIT != 0;
        let fast_target = if fast_owner {
            match candidate.capability_flags {
                RetailRuntimeValue::Known(flags) => {
                    flags & GO_TO_JOB_FAST_TARGET_CAPABILITY_BIT != 0
                }
                RetailRuntimeValue::Unresolved => {
                    return Err(GoToJobSetupError::CandidateCapabilityUnresolved {
                        id: candidate.id,
                    });
                }
            }
        } else {
            false
        };
        let accepts_job = if fast_target {
            true
        } else {
            candidate_accepts_job(
                &JobNearbyCandidate {
                    id: candidate.id,
                    position_raw: candidate.position_raw,
                    state_flags: candidate.state_flags,
                    capacity: candidate.capacity,
                },
                RetailRuntimeValue::Known(owner_capabilities),
            )
            .map_err(map_job_error)?
        };
        if !accepts_job {
            continue;
        }

        let distance =
            wrapped_quarter_squared_distance(request.owner.position_raw, candidate.position_raw);
        if distance < best_distance {
            best_distance = distance;
            best = Some(candidate.id);
        }
    }

    let constructor_suffix = GoToJobConstructorSuffix::from_owner(request.owner)
        .map_err(GoToJobSetupError::ConstructorSuffix)?;

    Ok(GoToJobSetupPlan {
        target_id: best,
        constructor_suffix,
        owner_id: request.owner.id,
        ordered_mutations: [
            ActorTaskMutation::Clear {
                slot: ActorTaskSlot::Secondary,
            },
            ActorTaskMutation::Clear {
                slot: ActorTaskSlot::Tertiary,
            },
            ActorTaskMutation::TryInstall {
                slot: ActorTaskSlot::Primary,
                specification: GoToJobTaskSpec {
                    initial_position_raw: request.owner.position_raw,
                    target_id: best,
                },
            },
        ],
    })
}

fn map_job_error(error: JobNearbyEvaluationError) -> GoToJobSetupError {
    match error {
        JobNearbyEvaluationError::CandidateStateUnresolved { id } => {
            GoToJobSetupError::CandidateStateUnresolved { id }
        }
        JobNearbyEvaluationError::CandidateCapacityUnresolved { id } => {
            GoToJobSetupError::CandidateCapacityUnresolved { id }
        }
        JobNearbyEvaluationError::OwnerCapabilityUnresolved => {
            GoToJobSetupError::OwnerCapabilityUnresolved
        }
    }
}

/// Exact `FUN_00422DD0` ordering metric: each wrapped signed-16 delta is
/// squared and shifted right by two before the three terms are added.
fn wrapped_quarter_squared_distance(source: [i16; 3], target: [i16; 3]) -> i32 {
    source
        .into_iter()
        .zip(target)
        .map(|(source, target)| {
            let delta = i32::from(source.wrapping_sub(target));
            delta * delta >> 2
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::convert::Infallible;
    use std::rc::Rc;

    use super::*;
    use crate::entity_collision_state::{CommonMoverComponentTopology, DYING_STATE_BIT};
    use v2k_formats::collision::SubAPropulsionDescriptor;

    #[derive(Debug, PartialEq, Eq)]
    enum TestTask {
        OldPrimary,
        OldSecondary,
        OldTertiary,
        GoToJob(Option<u32>),
    }

    fn populated_task_owner() -> ActorTaskOwner<TestTask> {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(TestTask::OldPrimary),
        );
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(TestTask::OldSecondary),
        );
        owner.replace_prepared(
            ActorTaskSlot::Tertiary,
            PreparedActorTask::new(TestTask::OldTertiary),
        );
        owner
    }

    fn exact_suffix_metadata(target_speed_base_raw: i16) -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                SubAPropulsionDescriptor {
                    acceleration_raw: 1,
                    overspeed_correction_raw: 2,
                    target_speed_base_raw,
                },
            )),
            common_mover_topology: RetailRuntimeValue::Known(CommonMoverComponentTopology {
                sub_a: true,
                ..CommonMoverComponentTopology::default()
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn owner_with_metadata(
        capabilities: RetailRuntimeValue<u32>,
        entity_type: u16,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> GoToJobOwner {
        GoToJobOwner::from_type_metadata(7, [0, 0, 0], capabilities, entity_type, metadata)
    }

    fn owner(capabilities: RetailRuntimeValue<u32>) -> GoToJobOwner {
        owner_with_metadata(
            capabilities,
            GO_TO_JOB_FIRST_WORLD_SCIENTIST_TYPE,
            &exact_suffix_metadata(0x0300),
        )
    }

    fn type9_owner(capabilities: RetailRuntimeValue<u32>) -> GoToJobOwner {
        owner_with_metadata(
            capabilities,
            crate::common_mover::actor_abdi::FIRST_WORLD_PEASANT_ENTITY_TYPE,
            &exact_suffix_metadata(0x0300),
        )
    }

    fn candidate(
        id: u32,
        position_raw: [i16; 3],
        capabilities: RetailRuntimeValue<u32>,
        capacity: RetailRuntimeValue<Option<JobCapacityState>>,
    ) -> GoToJobCandidate {
        GoToJobCandidate {
            id,
            position_raw,
            state_flags: RetailStateWord::exact(1),
            capability_flags: capabilities,
            capacity,
        }
    }

    fn open_capacity() -> RetailRuntimeValue<Option<JobCapacityState>> {
        RetailRuntimeValue::Known(Some(JobCapacityState {
            current_jobs_raw: 1,
            capacity_raw: 2,
        }))
    }

    fn request<'a>(
        owner: GoToJobOwner,
        candidates: &'a [GoToJobCandidate],
    ) -> GoToJobSetupRequest<'a> {
        GoToJobSetupRequest {
            owner,
            candidates_in_intrusive_order: candidates,
            range: WrappedAxisRange::strict(0x0F00).unwrap(),
        }
    }

    fn constructor_suffix(
        target_speed_base_raw: i16,
    ) -> Result<GoToJobConstructorSuffix, GoToJobConstructorSuffixError> {
        GoToJobConstructorSuffix::from_owner(owner_with_metadata(
            RetailRuntimeValue::Known(0),
            GO_TO_JOB_FIRST_WORLD_SCIENTIST_TYPE,
            &exact_suffix_metadata(target_speed_base_raw),
        ))
    }

    fn exact_plan(request: GoToJobSetupRequest<'_>) -> Result<GoToJobSetupPlan, GoToJobSetupError> {
        plan_go_to_job_setup(request)
    }

    fn sub_a_runtime() -> SubAPropulsionRuntime {
        SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(77), -1, 73)
    }

    #[test]
    fn constructor_suffix_uses_signed_truncation_toward_zero() {
        assert_eq!(
            constructor_suffix(0x0300).unwrap().sub_a_target_speed_raw(),
            0x0400
        );
        assert_eq!(constructor_suffix(-2).unwrap().sub_a_target_speed_raw(), -2);
    }

    #[test]
    fn constructor_suffix_entity_type_gate_keeps_the_admitted_abdi_job_families() {
        let metadata = exact_suffix_metadata(0x0300);
        for entity_type in [
            GO_TO_JOB_FIRST_WORLD_SCIENTIST_TYPE,
            crate::common_mover::actor_abdi::FIRST_WORLD_PEASANT_ENTITY_TYPE,
            7,
            79,
            90,
            91,
            116,
            78,
            86,
            95,
        ] {
            assert!(GoToJobConstructorSuffix::from_owner(owner_with_metadata(
                RetailRuntimeValue::Known(0),
                entity_type,
                &metadata,
            ))
            .is_ok());
        }

        for actual in [0, 10, 123, u16::MAX] {
            assert_eq!(
                GoToJobConstructorSuffix::from_owner(owner_with_metadata(
                    RetailRuntimeValue::Known(0),
                    actual,
                    &metadata,
                )),
                Err(GoToJobConstructorSuffixError::WrongEntityType { actual })
            );
        }
    }

    #[test]
    fn constructor_suffix_fails_closed_without_exact_a_without_h_g_f() {
        let unresolved = EntityTypeRuntimeMetadata::default();
        assert_eq!(
            GoToJobConstructorSuffix::from_owner(owner_with_metadata(
                RetailRuntimeValue::Known(0),
                GO_TO_JOB_FIRST_WORLD_SCIENTIST_TYPE,
                &unresolved,
            )),
            Err(GoToJobConstructorSuffixError::UnresolvedComponentTopology)
        );

        let mut metadata = EntityTypeRuntimeMetadata {
            common_mover_topology: RetailRuntimeValue::Known(CommonMoverComponentTopology {
                sub_a: true,
                sub_h: true,
                ..CommonMoverComponentTopology::default()
            }),
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                SubAPropulsionDescriptor {
                    acceleration_raw: 1,
                    overspeed_correction_raw: 2,
                    target_speed_base_raw: 3,
                },
            )),
            ..EntityTypeRuntimeMetadata::default()
        };
        assert_eq!(
            GoToJobConstructorSuffix::from_owner(owner_with_metadata(
                RetailRuntimeValue::Known(0),
                GO_TO_JOB_FIRST_WORLD_SCIENTIST_TYPE,
                &metadata,
            )),
            Err(GoToJobConstructorSuffixError::SubHPresent)
        );

        metadata.common_mover_topology = RetailRuntimeValue::Known(CommonMoverComponentTopology {
            sub_a: true,
            sub_g: true,
            ..CommonMoverComponentTopology::default()
        });
        assert_eq!(
            GoToJobConstructorSuffix::from_owner(owner_with_metadata(
                RetailRuntimeValue::Known(0),
                GO_TO_JOB_FIRST_WORLD_SCIENTIST_TYPE,
                &metadata,
            )),
            Err(GoToJobConstructorSuffixError::SubGPresent)
        );

        metadata.common_mover_topology = RetailRuntimeValue::Known(CommonMoverComponentTopology {
            sub_a: true,
            sub_f: true,
            ..CommonMoverComponentTopology::default()
        });
        assert_eq!(
            GoToJobConstructorSuffix::from_owner(owner_with_metadata(
                RetailRuntimeValue::Known(0),
                GO_TO_JOB_FIRST_WORLD_SCIENTIST_TYPE,
                &metadata,
            )),
            Err(GoToJobConstructorSuffixError::SubFPresent)
        );

        metadata.common_mover_topology = RetailRuntimeValue::Known(CommonMoverComponentTopology {
            sub_a: true,
            ..CommonMoverComponentTopology::default()
        });
        metadata.sub_a_propulsion_descriptor = RetailRuntimeValue::Known(None);
        assert_eq!(
            GoToJobConstructorSuffix::from_owner(owner_with_metadata(
                RetailRuntimeValue::Known(0),
                GO_TO_JOB_FIRST_WORLD_SCIENTIST_TYPE,
                &metadata,
            )),
            Err(GoToJobConstructorSuffixError::MissingSubADescriptor)
        );
        metadata.sub_a_propulsion_descriptor = RetailRuntimeValue::Unresolved;
        assert_eq!(
            GoToJobConstructorSuffix::from_owner(owner_with_metadata(
                RetailRuntimeValue::Known(0),
                GO_TO_JOB_FIRST_WORLD_SCIENTIST_TYPE,
                &metadata,
            )),
            Err(GoToJobConstructorSuffixError::UnresolvedSubADescriptor)
        );
        metadata.common_mover_topology =
            RetailRuntimeValue::Known(CommonMoverComponentTopology::default());
        assert_eq!(
            GoToJobConstructorSuffix::from_owner(owner_with_metadata(
                RetailRuntimeValue::Known(0),
                GO_TO_JOB_FIRST_WORLD_SCIENTIST_TYPE,
                &metadata,
            )),
            Err(GoToJobConstructorSuffixError::MissingSubA)
        );
    }

    #[test]
    fn selector_keeps_strictly_nearest_candidate_and_first_equal_distance() {
        let candidates = [
            candidate(
                8,
                [0x0200, 0, 0],
                RetailRuntimeValue::Known(0),
                open_capacity(),
            ),
            candidate(
                9,
                [0x0100, 0, 0],
                RetailRuntimeValue::Known(0),
                open_capacity(),
            ),
            candidate(
                10,
                [-0x0100, 0, 0],
                RetailRuntimeValue::Known(0),
                open_capacity(),
            ),
        ];
        let plan = exact_plan(request(
            owner(RetailRuntimeValue::Known(0x0400)),
            &candidates,
        ))
        .unwrap();
        assert_eq!(plan.target_id(), Some(9));
        assert_eq!(
            wrapped_quarter_squared_distance([0; 3], [0x0100, 0, 0]),
            0x4000
        );
    }

    #[test]
    fn selector_rejects_self_inactive_dying_range_full_and_incompatible_entries() {
        let mut inactive = candidate(8, [0, 0, 0], RetailRuntimeValue::Known(0), open_capacity());
        inactive.state_flags = RetailStateWord::exact(0);
        let mut dying = inactive;
        dying.id = 9;
        dying.state_flags = RetailStateWord::exact(1 | DYING_STATE_BIT);
        let candidates = [
            candidate(
                7,
                [0, 0, 0],
                RetailRuntimeValue::Known(GO_TO_JOB_FAST_TARGET_CAPABILITY_BIT),
                open_capacity(),
            ),
            inactive,
            dying,
            candidate(
                10,
                [0x0F00, 0, 0],
                RetailRuntimeValue::Known(GO_TO_JOB_FAST_TARGET_CAPABILITY_BIT),
                open_capacity(),
            ),
            candidate(
                11,
                [0, 0, 0],
                RetailRuntimeValue::Known(0),
                RetailRuntimeValue::Known(Some(JobCapacityState {
                    current_jobs_raw: 2,
                    capacity_raw: 2,
                })),
            ),
            candidate(12, [0, 0, 0], RetailRuntimeValue::Known(0), open_capacity()),
        ];
        let plan = exact_plan(request(
            owner(RetailRuntimeValue::Known(0x0800)),
            &candidates,
        ))
        .unwrap();
        assert_eq!(plan.target_id(), None);
    }

    #[test]
    fn type9_fast_pair_is_candidate_bit_0x20_not_0x2000() {
        let candidates = [candidate(
            8,
            [0, 0, 0],
            RetailRuntimeValue::Known(GO_TO_JOB_FAST_TARGET_CAPABILITY_BIT),
            RetailRuntimeValue::Unresolved,
        )];
        let plan = exact_plan(request(
            type9_owner(RetailRuntimeValue::Known(0x1804)),
            &candidates,
        ))
        .unwrap();
        assert_eq!(plan.target_id(), Some(8));

        let old_misread = [candidate(
            9,
            [0, 0, 0],
            RetailRuntimeValue::Known(0x2000),
            RetailRuntimeValue::Unresolved,
        )];
        assert_eq!(
            exact_plan(request(
                type9_owner(RetailRuntimeValue::Known(0x1804)),
                &old_misread,
            )),
            Err(GoToJobSetupError::CandidateCapacityUnresolved { id: 9 })
        );
    }

    #[test]
    fn fast_owner_without_fast_target_falls_back_to_capacity_and_job_capability() {
        let candidates = [candidate(
            8,
            [0, 0, 0],
            RetailRuntimeValue::Known(0),
            open_capacity(),
        )];
        assert_eq!(
            exact_plan(request(
                owner(RetailRuntimeValue::Known(0x0800)),
                &candidates,
            ))
            .unwrap()
            .target_id(),
            None
        );
        assert_eq!(
            exact_plan(request(
                owner(RetailRuntimeValue::Known(0x0C00)),
                &candidates,
            ))
            .unwrap()
            .target_id(),
            Some(8)
        );
    }

    #[test]
    fn only_consumed_unknown_fields_fail_closed() {
        let candidate = candidate(
            8,
            [0, 0, 0],
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
        );
        assert_eq!(
            exact_plan(request(owner(RetailRuntimeValue::Unresolved), &[candidate],)),
            Err(GoToJobSetupError::OwnerCapabilityUnresolved)
        );
        assert_eq!(
            exact_plan(request(
                owner(RetailRuntimeValue::Known(0x0800)),
                &[candidate],
            )),
            Err(GoToJobSetupError::CandidateCapabilityUnresolved { id: 8 })
        );
        assert_eq!(
            exact_plan(request(
                owner(RetailRuntimeValue::Known(0x0400)),
                &[candidate],
            )),
            Err(GoToJobSetupError::CandidateCapacityUnresolved { id: 8 })
        );
    }

    #[test]
    fn target_selection_precedes_exact_clear_clear_install_action_order() {
        let plan = exact_plan(request(owner(RetailRuntimeValue::Known(0x0400)), &[])).unwrap();
        assert_eq!(plan.target_id(), None);
        assert_eq!(
            plan.ordered_mutations,
            [
                ActorTaskMutation::Clear {
                    slot: ActorTaskSlot::Secondary,
                },
                ActorTaskMutation::Clear {
                    slot: ActorTaskSlot::Tertiary,
                },
                ActorTaskMutation::TryInstall {
                    slot: ActorTaskSlot::Primary,
                    specification: GoToJobTaskSpec {
                        initial_position_raw: [0, 0, 0],
                        target_id: None,
                    },
                },
            ]
        );
    }

    #[test]
    fn selector_failure_leaves_the_task_owner_untouched() {
        let candidate = candidate(8, [0, 0, 0], RetailRuntimeValue::Known(0), open_capacity());
        let task_owner = populated_task_owner();

        assert_eq!(
            exact_plan(request(owner(RetailRuntimeValue::Unresolved), &[candidate],)),
            Err(GoToJobSetupError::OwnerCapabilityUnresolved)
        );
        assert_eq!(
            task_owner.state_in_slot(ActorTaskSlot::Primary),
            Some(&TestTask::OldPrimary)
        );
        assert_eq!(
            task_owner.state_in_slot(ActorTaskSlot::Secondary),
            Some(&TestTask::OldSecondary)
        );
        assert_eq!(
            task_owner.state_in_slot(ActorTaskSlot::Tertiary),
            Some(&TestTask::OldTertiary)
        );
    }

    #[test]
    fn applying_the_plan_clears_later_slots_and_prepares_one_exact_primary_task() {
        let plan = exact_plan(request(owner(RetailRuntimeValue::Known(0x0400)), &[])).unwrap();
        let mut task_owner = populated_task_owner();
        let mut sub_a = sub_a_runtime();
        let mut prepared_specs = Vec::new();
        let draws = Cell::new(0);

        plan.apply(
            owner(RetailRuntimeValue::Known(0x0400)).bind_runtime(&mut task_owner, &mut sub_a),
            || {
                draws.set(draws.get() + 1);
                0xDEAD_ABCD
            },
            |spec| {
                prepared_specs.push(spec);
                Ok::<_, &'static str>(PreparedActorTask::new(TestTask::GoToJob(spec.target_id())))
            },
        )
        .unwrap();

        assert_eq!(prepared_specs.len(), 1);
        let spec = prepared_specs[0];
        assert_eq!(spec.target_id(), None);
        assert_eq!(spec.constructor_address(), 0x0040_3650);
        assert_eq!(spec.initializer_address(), 0x0040_1350);
        assert_eq!(spec.tick_address(), 0x0040_3780);
        assert_eq!(spec.lifetime_ms(), 5_000);
        assert_eq!(
            task_owner.state_in_slot(ActorTaskSlot::Primary),
            Some(&TestTask::GoToJob(None))
        );
        assert_eq!(task_owner.state_in_slot(ActorTaskSlot::Secondary), None);
        assert_eq!(task_owner.state_in_slot(ActorTaskSlot::Tertiary), None);
        assert_eq!(draws.get(), 1);
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(0x0400));
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(sub_a.drive_scale_percent(), 73);
    }

    #[test]
    fn type7_job_suffix_consumes_one_word_and_keeps_its_sub_a_speed() {
        let diver = owner_with_metadata(
            RetailRuntimeValue::Known(0x1404),
            crate::common_mover::actor_abdi::ORDINARY_TYPE7_ENTITY_TYPE,
            &exact_suffix_metadata(250),
        );
        let plan = exact_plan(request(diver, &[])).unwrap();
        let mut tasks = populated_task_owner();
        let mut sub_a = sub_a_runtime();
        let draws = Cell::new(0);
        plan.apply(
            diver.bind_runtime(&mut tasks, &mut sub_a),
            || {
                draws.set(draws.get() + 1);
                0xDEAD_ABCD
            },
            |spec| Ok::<_, Infallible>(PreparedActorTask::new(TestTask::GoToJob(spec.target_id()))),
        )
        .unwrap();
        assert_eq!(draws.get(), 1, "406070 runs once before the 03650 override");
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(333));
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(sub_a.drive_scale_percent(), 73);
        assert_eq!(
            tasks.state_in_slot(ActorTaskSlot::Primary),
            Some(&TestTask::GoToJob(None))
        );
        assert_eq!(tasks.state_in_slot(ActorTaskSlot::Secondary), None);
        assert_eq!(tasks.state_in_slot(ActorTaskSlot::Tertiary), None);
    }

    #[test]
    fn type9_failed_primary_preparation_is_lazy_and_preserves_primary_after_both_clears() {
        let type9_owner = type9_owner(RetailRuntimeValue::Known(0x0400));
        let plan = exact_plan(request(type9_owner, &[])).unwrap();
        let mut task_owner = populated_task_owner();
        let mut sub_a = sub_a_runtime();

        let error = plan
            .apply(
                type9_owner.bind_runtime(&mut task_owner, &mut sub_a),
                || panic!("failed preparation must not consume constructor RNG"),
                |_| Err::<PreparedActorTask<TestTask>, _>("allocation failed"),
            )
            .unwrap_err();

        assert_eq!(
            error,
            GoToJobApplyError::Prepare(ActorTaskPrepareError {
                action_index: 2,
                slot: ActorTaskSlot::Primary,
                error: "allocation failed",
            })
        );
        assert_eq!(
            task_owner.state_in_slot(ActorTaskSlot::Primary),
            Some(&TestTask::OldPrimary)
        );
        assert_eq!(task_owner.state_in_slot(ActorTaskSlot::Secondary), None);
        assert_eq!(task_owner.state_in_slot(ActorTaskSlot::Tertiary), None);
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(77));
        assert_eq!(sub_a.direction_multiplier(), -1);
        assert_eq!(sub_a.drive_scale_percent(), 73);
    }

    #[test]
    fn mismatched_bound_runtime_fails_before_any_mutation() {
        let plan = exact_plan(request(owner(RetailRuntimeValue::Known(0x0400)), &[])).unwrap();
        let mut task_owner = populated_task_owner();
        let mut sub_a = sub_a_runtime();

        let error = plan
            .apply(
                GoToJobBoundRuntime {
                    owner_id: 99,
                    tasks: &mut task_owner,
                    sub_a: &mut sub_a,
                },
                || panic!("identity mismatch must fail before constructor RNG"),
                |_| -> Result<PreparedActorTask<TestTask>, Infallible> {
                    panic!("identity mismatch must fail before preparation")
                },
            )
            .unwrap_err();

        assert_eq!(
            error,
            GoToJobApplyError::OwnerRuntimeMismatch {
                planned_owner_id: 7,
                runtime_owner_id: 99,
            }
        );
        assert_eq!(
            task_owner.state_in_slot(ActorTaskSlot::Primary),
            Some(&TestTask::OldPrimary)
        );
        assert_eq!(
            task_owner.state_in_slot(ActorTaskSlot::Secondary),
            Some(&TestTask::OldSecondary)
        );
        assert_eq!(
            task_owner.state_in_slot(ActorTaskSlot::Tertiary),
            Some(&TestTask::OldTertiary)
        );
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(77));
    }

    #[test]
    fn authenticated_parts_binder_rejects_wrong_owner_before_mutation() {
        let planned_owner = owner(RetailRuntimeValue::Known(0x0400));
        let mut task_owner = populated_task_owner();
        let mut sub_a = sub_a_runtime();

        let error =
            match planned_owner.bind_authenticated_parts_runtime(99, &mut task_owner, &mut sub_a) {
                Ok(_) => panic!("mismatched split runtime must not bind"),
                Err(error) => error,
            };

        assert_eq!(
            error,
            GoToJobRuntimeBindError::OwnerRuntimeMismatch {
                planned_owner_id: 7,
                runtime_owner_id: 99,
            }
        );
        assert_eq!(
            task_owner.state_in_slot(ActorTaskSlot::Primary),
            Some(&TestTask::OldPrimary)
        );
        assert_eq!(
            task_owner.state_in_slot(ActorTaskSlot::Secondary),
            Some(&TestTask::OldSecondary)
        );
        assert_eq!(
            task_owner.state_in_slot(ActorTaskSlot::Tertiary),
            Some(&TestTask::OldTertiary)
        );
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(77));
    }

    #[test]
    fn type9_success_orders_prepare_rng_generic_suffix_custom_overwrite_then_publish() {
        #[derive(Debug, PartialEq, Eq)]
        enum Event {
            Marker(&'static str),
            Effect(GoToJobConstructorEffect),
        }

        #[derive(Debug)]
        struct LoggedTask {
            retired_as: &'static str,
            events: Rc<RefCell<Vec<Event>>>,
        }

        impl Drop for LoggedTask {
            fn drop(&mut self) {
                self.events
                    .borrow_mut()
                    .push(Event::Marker(self.retired_as));
            }
        }

        let events = Rc::new(RefCell::new(Vec::new()));
        let logged_task = |retired_as| {
            PreparedActorTask::new(LoggedTask {
                retired_as,
                events: Rc::clone(&events),
            })
        };
        let mut task_owner = ActorTaskOwner::new();
        task_owner.replace_prepared(ActorTaskSlot::Primary, logged_task("retire-primary"));
        task_owner.replace_prepared(ActorTaskSlot::Secondary, logged_task("clear-secondary"));
        task_owner.replace_prepared(ActorTaskSlot::Tertiary, logged_task("clear-tertiary"));

        let plan =
            exact_plan(request(type9_owner(RetailRuntimeValue::Known(0x0400)), &[])).unwrap();
        plan.apply_with_constructor_effects(
            &mut task_owner,
            7,
            || {
                events.borrow_mut().push(Event::Marker("rng"));
                0x1234_ABCD
            },
            |effect| events.borrow_mut().push(Event::Effect(effect)),
            |specification| {
                assert_eq!(
                    specification.target_id(),
                    None,
                    "retail still installs the zero target sentinel"
                );
                events.borrow_mut().push(Event::Marker("prepare"));
                Ok::<_, Infallible>(logged_task("retire-new-primary"))
            },
        )
        .unwrap();

        assert_eq!(
            events.borrow().as_slice(),
            [
                Event::Marker("clear-secondary"),
                Event::Marker("clear-tertiary"),
                Event::Marker("prepare"),
                Event::Marker("rng"),
                Event::Effect(GoToJobConstructorEffect::WriteSubADirection {
                    direction_multiplier: 1,
                }),
                Event::Effect(GoToJobConstructorEffect::WriteGenericSubATargetSpeed {
                    target_speed_raw: 819,
                    random_sample_low16: 0xABCD,
                }),
                Event::Effect(GoToJobConstructorEffect::WriteGoToJobSubATargetSpeed {
                    target_speed_raw: 0x0400,
                }),
                Event::Marker("retire-primary"),
            ]
        );
        assert!(task_owner.state_in_slot(ActorTaskSlot::Primary).is_some());
    }
}
