//! Authenticated task dispatch for an already-published Level-1 type-17
//! Common-Dying task.
//!
//! The accepted type-17 captures prove the lethal handoff into class 12 for
//! four authored Level-1/model-256 actors. Retail and demo C then close the
//! detailed task tail: update the wrapper age, emit the terrain effect, run the
//! common mover, damp post-mover X/Z, and select the terminal style only after
//! the strict 9,000-ms timeout. Static retail/demo C also closes the distinct
//! coarse dispatch: scheduler mode 1 returns the class-12 `0x9C01` tag without
//! effect lookup or mover work, so its owner transition occurs immediately.
//! The terminal initializer clears all three task slots, marks the entity, and
//! records its identity for the later centralized sweep owned by
//! `EntityManager`.
//!
//! This seam deliberately starts after task publication. It does not perform
//! pre-impact behavior reselection, impact reaction, checked damage, class-12
//! construction/shared RNG, production backend lookup, or E870's suffix after
//! the coarse task returns. Detailed callers must supply effect inputs and
//! concrete effect/common-mover backends.

use crate::actor_death::{
    COMMON_ACTOR_DYING_ACTIVE_STYLE, COMMON_ACTOR_DYING_COMPLETION_DISABLE_POLICY,
    COMMON_ACTOR_DYING_COMPLETION_STYLE,
};
use crate::actor_task_dispatcher::{ActorTaskRuntime, ActorTaskRuntimeFamily};
use crate::actor_task_owner::{
    ActorTaskSlot, ActorTaskVisit, ActorTaskVisitControl, ActorTaskWrapperFlags,
};
use crate::common_dying::{
    common_dying_after_unwind, tick_common_dying, CommonDyingAfterUnwindOutcome,
    CommonDyingCallbackError, CommonDyingCallbackPrefix, CommonDyingCallbackResolution,
    CommonDyingCallbackResult, CommonDyingEffectInputs, CommonDyingFrameRequest,
    CommonDyingTaggedResult, InvokeCommonDyingEffect, InvokeCommonDyingMover,
};
use crate::entity::{raw_position_world, Entity, EntityManager};
use crate::entity_behavior::{
    audited_behavior_program, translate_state_policy, ActiveBehaviorStyle, BehaviorContextRuntime,
    BehaviorDescriptorIdentity,
};
use crate::entity_collision_state::{
    EntityTypeRuntimeMetadata, RetailRuntimeValue, ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
    DEFERRED_DESTROY_PENDING_STATE_BIT,
};
use crate::sub_h_external_frame::SubHRuntimeState;

pub const FRESH_LEVEL1_TYPE17_COMMON_DYING_SPAWN_INDICES: [usize; 4] = [17, 18, 19, 20];
pub const TYPE17_COMMON_DYING_ENTITY_TYPE: u32 = 17;
pub const TYPE17_COMMON_DYING_MODEL_ID: usize = 256;
pub const COMMON_DYING_BEHAVIOR_CLASS_ID: u32 = 12;

const COMMON_DYING_COARSE_SCHEDULER_MODE: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Type17CommonDyingTaskMode {
    Detailed,
    Coarse,
}

impl Type17CommonDyingTaskMode {
    const fn scheduler_mode(self) -> u32 {
        match self {
            Self::Detailed => 0,
            Self::Coarse => COMMON_DYING_COARSE_SCHEDULER_MODE,
        }
    }
}

fn common_dying_behavior_context(
    style: crate::entity_behavior::BehaviorStyle,
    previous: Option<BehaviorContextRuntime>,
) -> BehaviorContextRuntime {
    let program = audited_behavior_program(COMMON_DYING_BEHAVIOR_CLASS_ID)
        .expect("class-12 alternate behavior program is statically catalogued");
    let style_offset = style
        .frame_address
        .checked_sub(program.style_table_base_address)
        .expect("class-12 live style follows its descriptor table base");
    assert_eq!(
        style_offset % 0x48,
        0,
        "class-12 live style is aligned to an exact 0x48-byte table record"
    );
    let style_table_index_raw = style_offset / 0x48;
    let (source, target, auxiliary) = previous.map_or(
        (
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
        ),
        |context| {
            (
                context.choice_list_source(),
                context.target_handle_at_0x08(),
                context.auxiliary_word_at_0x0c(),
            )
        },
    );
    BehaviorContextRuntime::named_audited(
        program,
        style_table_index_raw,
        source,
        target,
        auxiliary,
        style,
    )
    .expect("class-12 live styles are statically audited")
}

/// Apply the class-12 variant-one transition shared by authenticated actor
/// owners after a Common-Dying callback requests its tagged transition.
///
/// The manager-owned identity/count is queued after the entity borrow ends;
/// this helper owns only the entity-local style, policy, task-slot, and mark
/// writes performed before that queue publication.
pub(crate) fn stage_common_dying_terminal_transition(entity: &mut Entity) {
    // FUN_0040EA10 publishes variant 1 and applies its reversed +0x38 state
    // policy before invoking terminal initializer FUN_0040C470.
    let previous_context = match entity.current_behavior_context {
        RetailRuntimeValue::Known(Some(context)) => context,
        _ => unreachable!("the owner receipt authenticated its live context"),
    };
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(
        common_dying_behavior_context(COMMON_ACTOR_DYING_COMPLETION_STYLE, Some(previous_context)),
    ));
    let disabled = translate_state_policy(COMMON_ACTOR_DYING_COMPLETION_DISABLE_POLICY);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(disabled.set_bits | disabled.clear_bits, disabled.clear_bits);
    // FUN_0040A860 clears physical slots 0, 1, 2 in this order.
    for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        entity.actor_tasks.clear_slot(slot);
    }
    // FUN_00410B70 stages, but does not perform, the later centralized
    // intrusive-list sweep. The manager records the matching identity after
    // the entity borrow ends.
    entity.mark_actor_deferred_destroy_pending();
}

/// Why an already-published task cannot be authenticated as the captured
/// Level-1 type-17 normal tail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type17CommonDyingLiveOwnerError {
    NotRetailFirstWorld,
    EntityInactive,
    UnauthenticatedSpawn,
    WrongEntityType,
    MetadataModelMismatch,
    AlternateBehaviorClassMismatch,
    EntityModelMismatch,
    WrongActiveModelSlot,
    HealthNotZero,
    ActiveStyleMismatch,
    DeferredDestroyAlreadyPending,
    WrongTaskSlot,
    AdditionalPublishedTask { slot: ActorTaskSlot },
    TaskLeaseUnavailable { visit: ActorTaskVisit },
    TaskFamilyMismatch { visit: ActorTaskVisit },
    TaskWrapperNotRunnable { visit: ActorTaskVisit },
}

/// Entity-specific receipt for one exact, already-published primary task.
///
/// The task lease is revalidated before every frame, so completion or any
/// external replacement makes the receipt stale instead of replayable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelOneType17CommonDyingOwner {
    entity_id: u32,
    authored_spawn_index: usize,
    visit: ActorTaskVisit,
}

impl LevelOneType17CommonDyingOwner {
    /// Build the immediate tail owner from the successful authenticated
    /// class-12 publication transaction.
    ///
    /// The publisher has already proved the authored allocation, metadata,
    /// style, task family, runnable wrapper, and absence of auxiliary tasks.
    /// Keeping this construction infallible prevents future validation changes
    /// from introducing a mutation-then-panic path.
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

    pub fn from_entity_metadata(
        retail_first_world: bool,
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
        visit: ActorTaskVisit,
    ) -> Result<Self, Type17CommonDyingLiveOwnerError> {
        if !retail_first_world {
            return Err(Type17CommonDyingLiveOwnerError::NotRetailFirstWorld);
        }
        if metadata.model_slots[0] as usize != TYPE17_COMMON_DYING_MODEL_ID
            || metadata.model_slots[1] as usize != TYPE17_COMMON_DYING_MODEL_ID
        {
            return Err(Type17CommonDyingLiveOwnerError::MetadataModelMismatch);
        }
        if metadata
            .initializer
            .as_ref()
            .map(|initializer| initializer.alternate_behavior_class_ref)
            != Some(COMMON_DYING_BEHAVIOR_CLASS_ID)
        {
            return Err(Type17CommonDyingLiveOwnerError::AlternateBehaviorClassMismatch);
        }
        let authored_spawn_index = entity
            .authored_spawn_index
            .ok_or(Type17CommonDyingLiveOwnerError::UnauthenticatedSpawn)?;
        validate_live_entity(entity, authored_spawn_index, visit)?;
        Ok(Self {
            entity_id: entity.id,
            authored_spawn_index,
            visit,
        })
    }

    pub const fn entity_id(self) -> u32 {
        self.entity_id
    }

    pub const fn visit(self) -> ActorTaskVisit {
        self.visit
    }

    /// Tick one captured normal-mode frame through its manager-owned deferred
    /// sweep bookkeeping.
    ///
    /// Detailed mode is represented by this method, not by a caller-controlled
    /// numeric sentinel. A callback-evidence error is reported with its
    /// already-committed elapsed prefix and must not be retried for the same
    /// frame.
    pub fn tick_published_normal<TypeRuntime, ComponentRuntime, MoverReturn>(
        self,
        manager: &mut EntityManager,
        request: Type17CommonDyingLiveFrameRequest<TypeRuntime, ComponentRuntime>,
        resolve_effect_inputs: impl FnOnce() -> CommonDyingEffectInputs,
        invoke_effect: impl FnMut(InvokeCommonDyingEffect),
        invoke_common_mover: impl FnOnce(
            InvokeCommonDyingMover<ActorTaskVisit, TypeRuntime, ComponentRuntime>,
            &mut [i16; 3],
        ) -> MoverReturn,
    ) -> Result<Type17CommonDyingLiveFrameOutcome, Type17CommonDyingLiveFrameError> {
        self.tick_published_with_optional_commit(
            manager,
            request,
            Type17CommonDyingTaskMode::Detailed,
            resolve_effect_inputs,
            invoke_effect,
            invoke_common_mover,
            None,
        )
    }

    /// Tick the exact coarse-owner task dispatch selected by `FUN_0040E870`.
    ///
    /// The named method keeps scheduler mode 1 out of caller-controlled data.
    /// Retail commits the wrapper-age prefix, returns the class-12 `0x9C01`
    /// tag without resolving an effect or invoking the common mover, unwinds
    /// the wrapper, and only then evaluates the owner-transition gate.  The
    /// enclosing E870 suffix remains with the production adapter because it
    /// executes after this transition.
    pub(crate) fn tick_published_coarse(
        self,
        manager: &mut EntityManager,
        elapsed_micros: u32,
    ) -> Result<Type17CommonDyingLiveFrameOutcome, Type17CommonDyingLiveFrameError> {
        self.tick_published_with_optional_commit(
            manager,
            Type17CommonDyingLiveFrameRequest {
                visit: self.visit,
                elapsed_micros,
                type_runtime: (),
                component_runtime: (),
            },
            Type17CommonDyingTaskMode::Coarse,
            || unreachable!("mode 1 returns before Common-Dying effect resolution"),
            |_| unreachable!("mode 1 returns before the Common-Dying effect"),
            |_, _| -> () { unreachable!("mode 1 returns before the common mover") },
            None,
        )
    }

    /// Tick one preflighted production frame and commit its auxiliary entity
    /// state at the callback boundary.
    ///
    /// Common-Dying's shared effect and mover mutate more than linear
    /// velocity.  The bounded production adapter plans those mutations before
    /// entering the task wrapper, then supplies them here so position, angle
    /// words, and disabled Sub-H state commit after mover damping but before
    /// wrapper unwind or a terminal owner transition.  Keeping this seam
    /// crate-private prevents detached callers from manufacturing live state.
    pub(crate) fn tick_published_normal_with_preplanned_commit<
        TypeRuntime,
        ComponentRuntime,
        MoverReturn,
    >(
        self,
        manager: &mut EntityManager,
        request: Type17CommonDyingLiveFrameRequest<TypeRuntime, ComponentRuntime>,
        resolve_effect_inputs: impl FnOnce() -> CommonDyingEffectInputs,
        invoke_effect: impl FnMut(InvokeCommonDyingEffect),
        invoke_common_mover: impl FnOnce(
            InvokeCommonDyingMover<ActorTaskVisit, TypeRuntime, ComponentRuntime>,
            &mut [i16; 3],
        ) -> MoverReturn,
        frame_commit: Type17CommonDyingLiveFrameCommit,
    ) -> Result<Type17CommonDyingLiveFrameOutcome, Type17CommonDyingLiveFrameError> {
        self.tick_published_with_optional_commit(
            manager,
            request,
            Type17CommonDyingTaskMode::Detailed,
            resolve_effect_inputs,
            invoke_effect,
            invoke_common_mover,
            Some(frame_commit),
        )
    }

    fn tick_published_with_optional_commit<TypeRuntime, ComponentRuntime, MoverReturn>(
        self,
        manager: &mut EntityManager,
        request: Type17CommonDyingLiveFrameRequest<TypeRuntime, ComponentRuntime>,
        task_mode: Type17CommonDyingTaskMode,
        resolve_effect_inputs: impl FnOnce() -> CommonDyingEffectInputs,
        invoke_effect: impl FnMut(InvokeCommonDyingEffect),
        invoke_common_mover: impl FnOnce(
            InvokeCommonDyingMover<ActorTaskVisit, TypeRuntime, ComponentRuntime>,
            &mut [i16; 3],
        ) -> MoverReturn,
        frame_commit: Option<Type17CommonDyingLiveFrameCommit>,
    ) -> Result<Type17CommonDyingLiveFrameOutcome, Type17CommonDyingLiveFrameError> {
        let outcome = {
            let entity = manager
                .common_actor_dying_entity_mut(self.entity_id)
                .ok_or(Type17CommonDyingLiveFrameError::EntityUnavailable {
                    entity_id: self.entity_id,
                })?;
            self.tick_entity_with_optional_commit(
                entity,
                request,
                task_mode,
                resolve_effect_inputs,
                invoke_effect,
                invoke_common_mover,
                frame_commit,
            )
        }?;
        if matches!(
            outcome,
            Type17CommonDyingLiveFrameOutcome::DeferredDestroyStaged { .. }
        ) {
            manager.queue_actor_deferred_destroy(self.entity_id);
        }
        Ok(outcome)
    }

    #[cfg(test)]
    fn tick_entity_normal<TypeRuntime, ComponentRuntime, MoverReturn>(
        self,
        entity: &mut Entity,
        request: Type17CommonDyingLiveFrameRequest<TypeRuntime, ComponentRuntime>,
        resolve_effect_inputs: impl FnOnce() -> CommonDyingEffectInputs,
        invoke_effect: impl FnMut(InvokeCommonDyingEffect),
        invoke_common_mover: impl FnOnce(
            InvokeCommonDyingMover<ActorTaskVisit, TypeRuntime, ComponentRuntime>,
            &mut [i16; 3],
        ) -> MoverReturn,
    ) -> Result<Type17CommonDyingLiveFrameOutcome, Type17CommonDyingLiveFrameError> {
        self.tick_entity_with_optional_commit(
            entity,
            request,
            Type17CommonDyingTaskMode::Detailed,
            resolve_effect_inputs,
            invoke_effect,
            invoke_common_mover,
            None,
        )
    }

    fn tick_entity_with_optional_commit<TypeRuntime, ComponentRuntime, MoverReturn>(
        self,
        entity: &mut Entity,
        request: Type17CommonDyingLiveFrameRequest<TypeRuntime, ComponentRuntime>,
        task_mode: Type17CommonDyingTaskMode,
        resolve_effect_inputs: impl FnOnce() -> CommonDyingEffectInputs,
        invoke_effect: impl FnMut(InvokeCommonDyingEffect),
        invoke_common_mover: impl FnOnce(
            InvokeCommonDyingMover<ActorTaskVisit, TypeRuntime, ComponentRuntime>,
            &mut [i16; 3],
        ) -> MoverReturn,
        frame_commit: Option<Type17CommonDyingLiveFrameCommit>,
    ) -> Result<Type17CommonDyingLiveFrameOutcome, Type17CommonDyingLiveFrameError> {
        if entity.id != self.entity_id {
            return Err(Type17CommonDyingLiveFrameError::OwnerRuntimeMismatch {
                planned_owner_id: self.entity_id,
                runtime_owner_id: entity.id,
            });
        }
        if request.visit != self.visit {
            return Err(Type17CommonDyingLiveFrameError::VisitMismatch {
                planned: self.visit,
                requested: request.visit,
            });
        }
        validate_live_entity(entity, self.authored_spawn_index, self.visit)
            .map_err(Type17CommonDyingLiveFrameError::RuntimeStateChanged)?;

        let owner_entity_id = entity.id;
        let linear_velocity_raw = entity.velocity_raw();
        let internal = {
            let Entity {
                actor_tasks,
                velocity,
                position,
                pitch_roll_raw,
                sub_h_external_frame_runtime,
                ..
            } = entity;
            let mut context = Type17CommonDyingFrameContext {
                owner_entity_id,
                expected_visit: self.visit,
                task_mode,
                request: Some(request),
                linear_velocity_raw,
                velocity,
                resolve_effect_inputs: Some(resolve_effect_inputs),
                invoke_effect,
                invoke_common_mover: Some(invoke_common_mover),
                frame_commit,
                position,
                pitch_roll_raw,
                sub_h_external_frame_runtime,
            };
            actor_tasks
                .visit_slots_fresh_phased_with(
                    &mut context,
                    |context, state, visit| context.before_callback(state, visit),
                    |context, _owner, visit| context.callback(visit),
                    |context, _owner, visit, prefix, callback| {
                        context.after_unwind(visit, prefix, callback)
                    },
                )
                .ok_or(Type17CommonDyingLiveFrameError::ExpectedVisitNotTicked)?
        }?;

        match internal {
            Type17CommonDyingInternalOutcome::Continue {
                committed_prefix,
                callback_evidence,
                linear_velocity_raw,
            } => Ok(Type17CommonDyingLiveFrameOutcome::Continue {
                visit: self.visit,
                committed_prefix,
                callback_evidence,
                linear_velocity_raw,
            }),
            Type17CommonDyingInternalOutcome::RequestOwnerTransition {
                committed_prefix,
                callback_evidence,
                linear_velocity_raw,
            } => match entity
                .collision
                .state_flags_at_0x08
                .masked(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT)
            {
                RetailRuntimeValue::Known(0) => {
                    self.stage_terminal_transition(entity);
                    Ok(Type17CommonDyingLiveFrameOutcome::DeferredDestroyStaged {
                        entity_id: self.entity_id,
                        committed_prefix,
                        callback_evidence,
                        linear_velocity_raw,
                    })
                }
                RetailRuntimeValue::Known(_) => {
                    Ok(Type17CommonDyingLiveFrameOutcome::TransitionSuppressed {
                        visit: self.visit,
                        committed_prefix,
                        callback_evidence,
                        linear_velocity_raw,
                    })
                }
                RetailRuntimeValue::Unresolved => {
                    Err(Type17CommonDyingLiveFrameError::TransitionGateUnresolved {
                        visit: self.visit,
                        committed_prefix,
                        callback_evidence,
                        linear_velocity_raw,
                    })
                }
            },
        }
    }

    /// Apply the shared variant-one transition after either a detailed timeout
    /// or the coarse mode-1 tag has survived callback unwind.
    fn stage_terminal_transition(self, entity: &mut Entity) {
        stage_common_dying_terminal_transition(entity);
    }
}

/// Inputs outside the live [`Entity`] allocation.
#[derive(Debug)]
pub struct Type17CommonDyingLiveFrameRequest<TypeRuntime, ComponentRuntime> {
    pub visit: ActorTaskVisit,
    pub elapsed_micros: u32,
    pub type_runtime: TypeRuntime,
    pub component_runtime: ComponentRuntime,
}

/// Auxiliary live state preplanned by the bounded Type-17 production adapter.
///
/// Linear velocity remains owned by [`tick_common_dying`], because its final
/// X/Z damping occurs after the shared mover returns.  These remaining fields
/// commit immediately afterwards, while the task wrapper is still inside its
/// callback and before any terminal owner transition is observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Type17CommonDyingLiveFrameCommit {
    pub position_raw: [i16; 3],
    pub pitch_raw: i16,
    pub roll_raw: i16,
    pub sub_h_runtime: SubHRuntimeState,
}

impl Type17CommonDyingLiveFrameCommit {
    fn apply(
        self,
        position: &mut [f32; 3],
        pitch_roll_raw: &mut [i16; 2],
        sub_h_external_frame_runtime: &mut RetailRuntimeValue<Option<SubHRuntimeState>>,
    ) {
        *position = raw_position_world(self.position_raw);
        position[0] = v2k_core::world::wrap(position[0]);
        position[2] = v2k_core::world::wrap(position[2]);
        *pitch_roll_raw = [self.pitch_raw, self.roll_raw];
        *sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(self.sub_h_runtime));
    }
}

/// Evidence identifying which exact Common-Dying callback branch completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type17CommonDyingCallbackEvidence {
    Detailed {
        selected_effect_raw: i32,
    },
    Coarse {
        tagged_result: CommonDyingTaggedResult,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type17CommonDyingLiveFrameOutcome {
    Continue {
        visit: ActorTaskVisit,
        committed_prefix: CommonDyingCallbackPrefix,
        callback_evidence: Type17CommonDyingCallbackEvidence,
        linear_velocity_raw: [i16; 3],
    },
    /// The terminal entity mark and manager-owned deferred identity/count have
    /// both been committed. The later manager sweep has not run yet.
    DeferredDestroyStaged {
        entity_id: u32,
        committed_prefix: CommonDyingCallbackPrefix,
        callback_evidence: Type17CommonDyingCallbackEvidence,
        linear_velocity_raw: [i16; 3],
    },
    /// `FUN_00416410` observed owner state bit `0x1000` after callback
    /// unwind. The frame is consumed, but the task and active style survive.
    TransitionSuppressed {
        visit: ActorTaskVisit,
        committed_prefix: CommonDyingCallbackPrefix,
        callback_evidence: Type17CommonDyingCallbackEvidence,
        linear_velocity_raw: [i16; 3],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type17CommonDyingLiveFrameError {
    EntityUnavailable {
        entity_id: u32,
    },
    OwnerRuntimeMismatch {
        planned_owner_id: u32,
        runtime_owner_id: u32,
    },
    VisitMismatch {
        planned: ActorTaskVisit,
        requested: ActorTaskVisit,
    },
    RuntimeStateChanged(Type17CommonDyingLiveOwnerError),
    FrameBlocked {
        visit: ActorTaskVisit,
        committed_prefix: CommonDyingCallbackPrefix,
        reason: CommonDyingCallbackError,
    },
    /// Effect, mover, velocity, and elapsed prefix are already committed, but
    /// the post-unwind transition gate cannot be decided from retained state.
    TransitionGateUnresolved {
        visit: ActorTaskVisit,
        committed_prefix: CommonDyingCallbackPrefix,
        callback_evidence: Type17CommonDyingCallbackEvidence,
        linear_velocity_raw: [i16; 3],
    },
    ExpectedVisitNotTicked,
}

#[derive(Debug, Clone, Copy)]
enum Type17CommonDyingInternalOutcome {
    Continue {
        committed_prefix: CommonDyingCallbackPrefix,
        callback_evidence: Type17CommonDyingCallbackEvidence,
        linear_velocity_raw: [i16; 3],
    },
    RequestOwnerTransition {
        committed_prefix: CommonDyingCallbackPrefix,
        callback_evidence: Type17CommonDyingCallbackEvidence,
        linear_velocity_raw: [i16; 3],
    },
}

struct Type17CommonDyingFrameContext<
    'entity,
    TypeRuntime,
    ComponentRuntime,
    ResolveEffect,
    Effect,
    Mover,
> {
    owner_entity_id: u32,
    expected_visit: ActorTaskVisit,
    task_mode: Type17CommonDyingTaskMode,
    request: Option<Type17CommonDyingLiveFrameRequest<TypeRuntime, ComponentRuntime>>,
    linear_velocity_raw: [i16; 3],
    velocity: &'entity mut [f32; 3],
    resolve_effect_inputs: Option<ResolveEffect>,
    invoke_effect: Effect,
    invoke_common_mover: Option<Mover>,
    frame_commit: Option<Type17CommonDyingLiveFrameCommit>,
    position: &'entity mut [f32; 3],
    pitch_roll_raw: &'entity mut [i16; 2],
    sub_h_external_frame_runtime: &'entity mut RetailRuntimeValue<Option<SubHRuntimeState>>,
}

impl<TypeRuntime, ComponentRuntime, ResolveEffect, Effect, Mover, MoverReturn>
    Type17CommonDyingFrameContext<'_, TypeRuntime, ComponentRuntime, ResolveEffect, Effect, Mover>
where
    ResolveEffect: FnOnce() -> CommonDyingEffectInputs,
    Effect: FnMut(InvokeCommonDyingEffect),
    Mover: FnOnce(
        InvokeCommonDyingMover<ActorTaskVisit, TypeRuntime, ComponentRuntime>,
        &mut [i16; 3],
    ) -> MoverReturn,
{
    fn before_callback(
        &mut self,
        state: &mut ActorTaskRuntime,
        visit: ActorTaskVisit,
    ) -> CommonDyingCallbackPrefix {
        debug_assert_eq!(visit, self.expected_visit);
        let ActorTaskRuntime::CommonDying(state) = state else {
            unreachable!("the exact Common-Dying visit was prevalidated")
        };
        let elapsed_micros = self
            .request
            .as_ref()
            .expect("the single frame request must still be present")
            .elapsed_micros;
        state.before_callback(elapsed_micros)
    }

    fn callback(
        &mut self,
        visit: ActorTaskVisit,
    ) -> Result<CommonDyingCallbackResolution, CommonDyingCallbackError> {
        debug_assert_eq!(visit, self.expected_visit);
        let request = self
            .request
            .take()
            .expect("the prevalidated task is the frame's only visit");
        let resolve_effect_inputs = self
            .resolve_effect_inputs
            .take()
            .expect("the effect resolver is one-shot");
        let invoke_common_mover = self
            .invoke_common_mover
            .take()
            .expect("the common mover is one-shot");
        let resolution = tick_common_dying(
            CommonDyingFrameRequest {
                task_wrapper: visit,
                owner_entity_id: self.owner_entity_id,
                type_runtime: request.type_runtime,
                component_runtime: request.component_runtime,
                elapsed_micros: request.elapsed_micros,
                scheduler_mode: self.task_mode.scheduler_mode(),
                linear_velocity_raw: self.linear_velocity_raw,
            },
            resolve_effect_inputs,
            &mut self.invoke_effect,
            invoke_common_mover,
        )?;
        // Retail writes the mover-produced/damped words in the callback,
        // before wrapper unwind and any terminal owner transition.
        *self.velocity = raw_position_world(resolution.linear_velocity_raw);
        if let Some(frame_commit) = self.frame_commit.take() {
            frame_commit.apply(
                self.position,
                self.pitch_roll_raw,
                self.sub_h_external_frame_runtime,
            );
        }
        Ok(resolution)
    }

    fn after_unwind(
        &mut self,
        visit: ActorTaskVisit,
        committed_prefix: CommonDyingCallbackPrefix,
        callback: Result<CommonDyingCallbackResolution, CommonDyingCallbackError>,
    ) -> ActorTaskVisitControl<
        Result<Type17CommonDyingInternalOutcome, Type17CommonDyingLiveFrameError>,
    > {
        let resolution = match callback {
            Ok(resolution) => resolution,
            Err(reason) => {
                return ActorTaskVisitControl::Propagate(Err(
                    Type17CommonDyingLiveFrameError::FrameBlocked {
                        visit,
                        committed_prefix,
                        reason,
                    },
                ));
            }
        };
        let callback_evidence = match self.task_mode {
            Type17CommonDyingTaskMode::Detailed => Type17CommonDyingCallbackEvidence::Detailed {
                selected_effect_raw: resolution
                    .selected_effect_raw
                    .expect("detailed mode always selects an effect value"),
            },
            Type17CommonDyingTaskMode::Coarse => {
                debug_assert_eq!(resolution.selected_effect_raw, None);
                let CommonDyingCallbackResult::TaggedOwnerTransition(tagged_result) =
                    resolution.callback_result
                else {
                    unreachable!("coarse mode always returns the class-12 transition tag")
                };
                Type17CommonDyingCallbackEvidence::Coarse { tagged_result }
            }
        };
        let outcome = match common_dying_after_unwind(committed_prefix, resolution.callback_result)
        {
            CommonDyingAfterUnwindOutcome::Continue => Type17CommonDyingInternalOutcome::Continue {
                committed_prefix,
                callback_evidence,
                linear_velocity_raw: resolution.linear_velocity_raw,
            },
            CommonDyingAfterUnwindOutcome::RequestOwnerTransition { .. } => {
                Type17CommonDyingInternalOutcome::RequestOwnerTransition {
                    committed_prefix,
                    callback_evidence,
                    linear_velocity_raw: resolution.linear_velocity_raw,
                }
            }
        };
        ActorTaskVisitControl::Propagate(Ok(outcome))
    }
}

fn validate_live_entity(
    entity: &Entity,
    authored_spawn_index: usize,
    visit: ActorTaskVisit,
) -> Result<(), Type17CommonDyingLiveOwnerError> {
    if !entity.active {
        return Err(Type17CommonDyingLiveOwnerError::EntityInactive);
    }
    if entity.authored_spawn_index != Some(authored_spawn_index)
        || !FRESH_LEVEL1_TYPE17_COMMON_DYING_SPAWN_INDICES.contains(&authored_spawn_index)
    {
        return Err(Type17CommonDyingLiveOwnerError::UnauthenticatedSpawn);
    }
    if entity.entity_type != TYPE17_COMMON_DYING_ENTITY_TYPE {
        return Err(Type17CommonDyingLiveOwnerError::WrongEntityType);
    }
    if entity.model_slots[0] != Some(TYPE17_COMMON_DYING_MODEL_ID)
        || entity.model_slots[1] != Some(TYPE17_COMMON_DYING_MODEL_ID)
        || entity.model_index != Some(TYPE17_COMMON_DYING_MODEL_ID)
    {
        return Err(Type17CommonDyingLiveOwnerError::EntityModelMismatch);
    }
    if entity.collision.active_model_slot() != RetailRuntimeValue::Known(1) {
        return Err(Type17CommonDyingLiveOwnerError::WrongActiveModelSlot);
    }
    if entity.collision.health_raw != RetailRuntimeValue::Known(0) {
        return Err(Type17CommonDyingLiveOwnerError::HealthNotZero);
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Type17CommonDyingLiveOwnerError::ActiveStyleMismatch);
    };
    let program = audited_behavior_program(COMMON_DYING_BEHAVIOR_CLASS_ID)
        .expect("class-12 alternate behavior program is statically catalogued");
    if context.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || context.active_style() != ActiveBehaviorStyle::Audited(COMMON_ACTOR_DYING_ACTIVE_STYLE)
    {
        return Err(Type17CommonDyingLiveOwnerError::ActiveStyleMismatch);
    }
    if entity
        .collision
        .state_flags_at_0x08
        .masked(DEFERRED_DESTROY_PENDING_STATE_BIT)
        != RetailRuntimeValue::Known(0)
    {
        return Err(Type17CommonDyingLiveOwnerError::DeferredDestroyAlreadyPending);
    }
    if visit.slot != ActorTaskSlot::Primary {
        return Err(Type17CommonDyingLiveOwnerError::WrongTaskSlot);
    }
    for slot in [ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary] {
        if entity.actor_tasks.task_in_slot(slot).is_some() {
            return Err(Type17CommonDyingLiveOwnerError::AdditionalPublishedTask { slot });
        }
    }
    if entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) != Some(visit.task_id) {
        return Err(Type17CommonDyingLiveOwnerError::TaskLeaseUnavailable { visit });
    }
    match entity.actor_tasks.task_state(visit.task_id) {
        Some(state) if state.family() == ActorTaskRuntimeFamily::CommonDying => {}
        Some(_) => return Err(Type17CommonDyingLiveOwnerError::TaskFamilyMismatch { visit }),
        None => return Err(Type17CommonDyingLiveOwnerError::TaskLeaseUnavailable { visit }),
    }
    if entity.actor_tasks.wrapper_flags(visit.task_id)
        != Some(ActorTaskWrapperFlags {
            alive: true,
            in_callback: false,
        })
    {
        return Err(Type17CommonDyingLiveOwnerError::TaskWrapperNotRunnable { visit });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;
    use crate::actor_task_owner::PreparedActorTask;
    use crate::common_dying::{
        damp_common_dying_axis, CommonDyingComponentDescriptors, CommonDyingConstructorEffect,
        CommonDyingTaskState,
    };
    use crate::entity::{EntityKind, EntityManager};
    use crate::entity_collision_state::{
        CommonMoverComponentTopology, EntityInitializerSpec, DEFERRED_DESTROY_STATE_WRITE_MASK,
    };
    use crate::session::GameSession;
    use crate::world_fx::WorldFx;
    use v2k_formats::collision::CommonAxisDescriptor;

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [TYPE17_COMMON_DYING_MODEL_ID as u16; 4],
            common_mover_topology: RetailRuntimeValue::Known(
                CommonMoverComponentTopology::default(),
            ),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: 0,
                common_axis_descriptor: CommonAxisDescriptor {
                    strict_axis_limit_raw: 0,
                    raw_word_at_0x04: 0,
                },
                behavior_choices: Box::new([]),
                behavior_rule_ref: 0,
                alternate_behavior_class_ref: COMMON_DYING_BEHAVIOR_CLASS_ID,
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn prepared_task(entity_id: u32) -> PreparedActorTask<ActorTaskRuntime> {
        let metadata = exact_metadata();
        CommonDyingTaskState::prepare_after_allocation(
            entity_id,
            &metadata,
            CommonDyingComponentDescriptors::default(),
        )
        .unwrap()
        .map_task(ActorTaskRuntime::CommonDying)
        .apply_suffix(
            || panic!("the component-free fixture cannot consume RNG"),
            |effect| {
                assert_eq!(
                    effect,
                    CommonDyingConstructorEffect::WriteOwnerVerticalVelocity { velocity_raw: 500 }
                )
            },
        )
    }

    fn exact_entity(entity_id: u32) -> (Entity, ActorTaskVisit) {
        let mut entity = Entity::unresolved_port_entity(
            entity_id,
            EntityKind::Enemy,
            TYPE17_COMMON_DYING_ENTITY_TYPE,
        );
        entity.authored_spawn_index = Some(FRESH_LEVEL1_TYPE17_COMMON_DYING_SPAWN_INDICES[0]);
        entity.model_slots = [Some(TYPE17_COMMON_DYING_MODEL_ID); 4];
        entity.model_index = Some(TYPE17_COMMON_DYING_MODEL_ID);
        entity.collision =
            crate::entity_collision_state::EntityCollisionRuntimeState::unresolved_port_entity(1);
        entity.collision.health_raw = RetailRuntimeValue::Known(0);
        entity.collision.state_flags_at_0x08.overwrite(
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT | DEFERRED_DESTROY_STATE_WRITE_MASK,
            0,
        );
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(
            common_dying_behavior_context(COMMON_ACTOR_DYING_ACTIVE_STYLE, None),
        ));
        let task_id = entity
            .actor_tasks
            .replace_prepared(ActorTaskSlot::Primary, prepared_task(entity_id));
        (
            entity,
            ActorTaskVisit {
                slot: ActorTaskSlot::Primary,
                task_id,
            },
        )
    }

    fn owner(entity: &Entity, visit: ActorTaskVisit) -> LevelOneType17CommonDyingOwner {
        LevelOneType17CommonDyingOwner::from_entity_metadata(true, entity, &exact_metadata(), visit)
            .unwrap()
    }

    fn request<TypeRuntime, ComponentRuntime>(
        visit: ActorTaskVisit,
        elapsed_micros: u32,
        type_runtime: TypeRuntime,
        component_runtime: ComponentRuntime,
    ) -> Type17CommonDyingLiveFrameRequest<TypeRuntime, ComponentRuntime> {
        Type17CommonDyingLiveFrameRequest {
            visit,
            elapsed_micros,
            type_runtime,
            component_runtime,
        }
    }

    #[test]
    fn normal_frame_orders_effect_before_mover_and_commits_post_mover_velocity() {
        let (mut entity, visit) = exact_entity(0x04AA_0001);
        entity.set_velocity_raw([100, -25, -200]);
        let owner = owner(&entity, visit);
        let events = RefCell::new(Vec::new());

        let outcome = owner
            .tick_entity_normal(
                &mut entity,
                request(visit, 20_000, 0x00AB_CDEF_u32, "component-runtime"),
                || CommonDyingEffectInputs {
                    descriptor_effect_raw: RetailRuntimeValue::Known(Some(7)),
                    attached_mass: RetailRuntimeValue::Known(149),
                },
                |effect| {
                    events.borrow_mut().push("effect");
                    assert_eq!(effect.owner_entity_id, 0x04AA_0001);
                    assert_eq!(effect.effect_kind, 0x100);
                    assert_eq!(effect.signed_effect_raw, 7);
                    assert_eq!(effect.enabled, 1);
                    assert_eq!(effect.elapsed_micros, 20_000);
                },
                |mover, velocity| {
                    events.borrow_mut().push("mover");
                    assert_eq!(mover.task_wrapper, visit);
                    assert_eq!(mover.owner_entity_id, 0x04AA_0001);
                    assert_eq!(mover.type_runtime, 0x00AB_CDEF);
                    assert_eq!(mover.component_runtime, "component-runtime");
                    assert_eq!(mover.explicit_target_raw, None);
                    assert_eq!(mover.elapsed_micros, 20_000);
                    assert_eq!(mover.scheduler_mode, 0);
                    *velocity = [200, 123, -400];
                    "ignored mover return"
                },
            )
            .unwrap();

        let expected_velocity = [
            damp_common_dying_axis(200, 20_000),
            123,
            damp_common_dying_axis(-400, 20_000),
        ];
        assert_eq!(events.into_inner(), ["effect", "mover"]);
        assert_eq!(entity.velocity_raw(), expected_velocity);
        assert_eq!(
            outcome,
            Type17CommonDyingLiveFrameOutcome::Continue {
                visit,
                committed_prefix: CommonDyingCallbackPrefix {
                    elapsed_ms: 20,
                    lifetime_status: crate::common_dying::CommonDyingLifetimeStatus::WithinLifetime,
                },
                callback_evidence: Type17CommonDyingCallbackEvidence::Detailed {
                    selected_effect_raw: 7,
                },
                linear_velocity_raw: expected_velocity,
            }
        );
        let Some(ActorTaskRuntime::CommonDying(state)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("the normal frame must retain its Common-Dying task")
        };
        assert_eq!(state.elapsed_ms(), 20);
        assert_eq!(
            entity
                .current_behavior_context
                .map(|context| { context.map(BehaviorContextRuntime::active_style) }),
            RetailRuntimeValue::Known(Some(ActiveBehaviorStyle::Audited(
                COMMON_ACTOR_DYING_ACTIVE_STYLE
            )))
        );
    }

    #[test]
    fn coarse_frame_uses_exact_tag_and_transitions_below_the_timeout() {
        let (mut entity, visit) = exact_entity(0x04AA_0021);
        entity.set_velocity_raw([2258, 0, 658]);
        let receipt = owner(&entity, visit);
        let mut manager = EntityManager::from_entities_for_test(vec![entity]);

        let outcome = receipt.tick_published_coarse(&mut manager, 20_000).unwrap();

        assert_eq!(
            outcome,
            Type17CommonDyingLiveFrameOutcome::DeferredDestroyStaged {
                entity_id: 0x04AA_0021,
                committed_prefix: CommonDyingCallbackPrefix {
                    elapsed_ms: 20,
                    lifetime_status: crate::common_dying::CommonDyingLifetimeStatus::WithinLifetime,
                },
                callback_evidence: Type17CommonDyingCallbackEvidence::Coarse {
                    tagged_result: CommonDyingTaggedResult {
                        singleton_address: 0x004B_E170,
                        tag: 0x9C01,
                    },
                },
                linear_velocity_raw: [2258, 0, 658],
            }
        );
        let staged = manager.iter_all().next().unwrap();
        assert_eq!(staged.velocity_raw(), [2258, 0, 658]);
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| staged.actor_task_state(slot).is_none()));
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), [0x04AA_0021]);
    }

    #[test]
    fn coarse_transition_suppression_consumes_age_but_retains_the_task() {
        let (mut entity, visit) = exact_entity(0x04AA_0022);
        entity.collision.state_flags_at_0x08.overwrite(
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
        );
        let receipt = owner(&entity, visit);
        let mut manager = EntityManager::from_entities_for_test(vec![entity]);

        let outcome = receipt.tick_published_coarse(&mut manager, 20_000).unwrap();

        assert!(matches!(
            outcome,
            Type17CommonDyingLiveFrameOutcome::TransitionSuppressed {
                visit: actual_visit,
                committed_prefix: CommonDyingCallbackPrefix { elapsed_ms: 20, .. },
                callback_evidence: Type17CommonDyingCallbackEvidence::Coarse {
                    tagged_result: CommonDyingTaggedResult { tag: 0x9C01, .. },
                },
                ..
            } if actual_visit == visit
        ));
        let retained = manager.iter_all().next().unwrap();
        let Some(ActorTaskRuntime::CommonDying(state)) =
            retained.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("suppression must retain the Common-Dying task")
        };
        assert_eq!(state.elapsed_ms(), 20);
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
    }

    #[test]
    fn preplanned_auxiliary_state_commits_before_terminal_and_same_tick_sweep() {
        let (mut entity, visit) = exact_entity(0x04AA_0017);
        entity.set_velocity_raw([10, 20, 30]);
        let receipt = owner(&entity, visit);
        let mut sub_h_runtime = SubHRuntimeState::new(1).unwrap();
        sub_h_runtime.set_enabled(false);
        sub_h_runtime.records_mut()[0].flags_raw = 1;
        let expected_sub_h = sub_h_runtime.clone();
        let mut manager = EntityManager::from_entities_for_test(vec![entity]);

        let outcome = receipt
            .tick_published_normal_with_preplanned_commit(
                &mut manager,
                request(visit, 9_001_000, (), ()),
                || CommonDyingEffectInputs {
                    descriptor_effect_raw: RetailRuntimeValue::Known(None),
                    attached_mass: RetailRuntimeValue::Unresolved,
                },
                |_| {},
                |_, velocity_raw| {
                    *velocity_raw = [100, 200, 300];
                    1
                },
                Type17CommonDyingLiveFrameCommit {
                    position_raw: [-1, 321, 2],
                    pitch_raw: -123,
                    roll_raw: 456,
                    sub_h_runtime,
                },
            )
            .unwrap();
        assert!(matches!(
            outcome,
            Type17CommonDyingLiveFrameOutcome::DeferredDestroyStaged {
                entity_id: 0x04AA_0017,
                ..
            }
        ));

        let committed = manager
            .entity_mut_for_test(0x04AA_0017)
            .expect("terminal entity remains until the later manager sweep");
        assert_eq!(committed.position_raw(), [-1, 321, 2]);
        assert_eq!(
            committed.rotation_heading_pitch_roll_raw()[1..],
            [-123, 456]
        );
        assert_eq!(
            committed.sub_h_external_frame_runtime,
            RetailRuntimeValue::Known(Some(expected_sub_h))
        );
        assert!(committed.actor_task_state(ActorTaskSlot::Primary).is_none());
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), [0x04AA_0017]);

        assert_eq!(
            manager.cleanup_pending_actor_deferred_destroys(),
            [0x04AA_0017]
        );
        assert!(manager
            .retail_live_order_ids()
            .all(|entity_id| entity_id != 0x04AA_0017));
    }

    #[test]
    fn exact_9000_survives_and_9001_runs_callback_before_terminal_mark() {
        let (mut entity, visit) = exact_entity(0x04AA_0001);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x0006_0000, 0x0006_0000);
        let owner = owner(&entity, visit);
        let effects = Cell::new(0_u32);
        let movers = Cell::new(0_u32);

        let exact = owner
            .tick_entity_normal(
                &mut entity,
                request(visit, 9_000_000, (), ()),
                || CommonDyingEffectInputs {
                    descriptor_effect_raw: RetailRuntimeValue::Known(None),
                    attached_mass: RetailRuntimeValue::Unresolved,
                },
                |_| effects.set(effects.get() + 1),
                |_, velocity| {
                    movers.set(movers.get() + 1);
                    *velocity = [100, 200, 300];
                    0
                },
            )
            .unwrap();
        assert!(matches!(
            exact,
            Type17CommonDyingLiveFrameOutcome::Continue {
                committed_prefix: CommonDyingCallbackPrefix {
                    elapsed_ms: 9_000,
                    ..
                },
                ..
            }
        ));
        assert_eq!((effects.get(), movers.get()), (1, 1));

        let terminal = owner
            .tick_entity_normal(
                &mut entity,
                request(visit, 1_000, (), ()),
                || CommonDyingEffectInputs {
                    descriptor_effect_raw: RetailRuntimeValue::Known(None),
                    attached_mass: RetailRuntimeValue::Unresolved,
                },
                |_| effects.set(effects.get() + 1),
                |_, velocity| {
                    movers.set(movers.get() + 1);
                    *velocity = [-500, 77, 900];
                    false
                },
            )
            .unwrap();
        let expected_velocity = [
            damp_common_dying_axis(-500, 1_000),
            77,
            damp_common_dying_axis(900, 1_000),
        ];
        assert_eq!((effects.get(), movers.get()), (2, 2));
        assert_eq!(entity.velocity_raw(), expected_velocity);
        assert_eq!(
            terminal,
            Type17CommonDyingLiveFrameOutcome::DeferredDestroyStaged {
                entity_id: 0x04AA_0001,
                committed_prefix: CommonDyingCallbackPrefix {
                    elapsed_ms: 9_001,
                    lifetime_status:
                        crate::common_dying::CommonDyingLifetimeStatus::OwnerTransitionDue,
                },
                callback_evidence: Type17CommonDyingCallbackEvidence::Detailed {
                    selected_effect_raw: 0,
                },
                linear_velocity_raw: expected_velocity,
            }
        );
        assert_eq!(
            entity
                .current_behavior_context
                .map(|context| { context.map(BehaviorContextRuntime::active_style) }),
            RetailRuntimeValue::Known(Some(ActiveBehaviorStyle::Audited(
                COMMON_ACTOR_DYING_COMPLETION_STYLE
            )))
        );
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_task_state(slot).is_none()));
        assert_eq!(entity.actor_tasks.wrapper_flags(visit.task_id), None);
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x0006_0000),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(0x0001_0000 | DEFERRED_DESTROY_PENDING_STATE_BIT),
            RetailRuntimeValue::Known(0x0001_0000 | DEFERRED_DESTROY_PENDING_STATE_BIT)
        );
        assert!(
            entity.active,
            "the terminal initializer does not unlink inline"
        );

        let replay_effects = Cell::new(0);
        assert!(matches!(
            owner.tick_entity_normal(
                &mut entity,
                request(visit, 1_000, (), ()),
                || CommonDyingEffectInputs {
                    descriptor_effect_raw: RetailRuntimeValue::Known(None),
                    attached_mass: RetailRuntimeValue::Unresolved,
                },
                |_| replay_effects.set(replay_effects.get() + 1),
                |_, _| replay_effects.set(replay_effects.get() + 1),
            ),
            Err(Type17CommonDyingLiveFrameError::RuntimeStateChanged(_))
        ));
        assert_eq!(replay_effects.get(), 0);
    }

    #[test]
    fn unresolved_effect_evidence_reports_the_consumed_nonretryable_prefix() {
        let (mut entity, visit) = exact_entity(77);
        entity.set_velocity_raw([11, 22, 33]);
        let owner = owner(&entity, visit);
        let effect_calls = Cell::new(0);
        let mover_calls = Cell::new(0);

        assert_eq!(
            owner.tick_entity_normal(
                &mut entity,
                request(visit, 1_999, (), ()),
                || CommonDyingEffectInputs {
                    descriptor_effect_raw: RetailRuntimeValue::Unresolved,
                    attached_mass: RetailRuntimeValue::Known(0),
                },
                |_| effect_calls.set(effect_calls.get() + 1),
                |_, _| mover_calls.set(mover_calls.get() + 1),
            ),
            Err(Type17CommonDyingLiveFrameError::FrameBlocked {
                visit,
                committed_prefix: CommonDyingCallbackPrefix {
                    elapsed_ms: 1,
                    lifetime_status: crate::common_dying::CommonDyingLifetimeStatus::WithinLifetime,
                },
                reason: CommonDyingCallbackError::UnresolvedEffectDescriptor,
            })
        );
        assert_eq!((effect_calls.get(), mover_calls.get()), (0, 0));
        assert_eq!(entity.velocity_raw(), [11, 22, 33]);
        let Some(ActorTaskRuntime::CommonDying(state)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("blocked callback must retain the task")
        };
        assert_eq!(state.elapsed_ms(), 1);
        assert_eq!(
            entity.actor_tasks.wrapper_flags(visit.task_id),
            Some(ActorTaskWrapperFlags {
                alive: true,
                in_callback: false,
            })
        );
    }

    #[test]
    fn owner_rejects_unproven_or_stale_dimensions_before_frame_side_effects() {
        let metadata = exact_metadata();
        let (entity, visit) = exact_entity(1);
        assert_eq!(
            LevelOneType17CommonDyingOwner::from_entity_metadata(false, &entity, &metadata, visit),
            Err(Type17CommonDyingLiveOwnerError::NotRetailFirstWorld)
        );

        let (mut extra, extra_visit) = exact_entity(3);
        extra
            .actor_tasks
            .replace_prepared(ActorTaskSlot::Secondary, prepared_task(extra.id));
        assert_eq!(
            LevelOneType17CommonDyingOwner::from_entity_metadata(
                true,
                &extra,
                &metadata,
                extra_visit,
            ),
            Err(Type17CommonDyingLiveOwnerError::AdditionalPublishedTask {
                slot: ActorTaskSlot::Secondary,
            })
        );

        let (mut replaced, old_visit) = exact_entity(4);
        let replacement_id = replaced
            .actor_tasks
            .replace_prepared(ActorTaskSlot::Primary, prepared_task(replaced.id));
        assert_ne!(replacement_id, old_visit.task_id);
        assert_eq!(
            LevelOneType17CommonDyingOwner::from_entity_metadata(
                true, &replaced, &metadata, old_visit,
            ),
            Err(Type17CommonDyingLiveOwnerError::TaskLeaseUnavailable { visit: old_visit })
        );
    }

    #[test]
    fn post_unwind_gate_suppresses_only_transition_after_consuming_the_frame() {
        let metadata = exact_metadata();
        let (mut entity, visit) = exact_entity(2);
        entity.collision.state_flags_at_0x08.overwrite(
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
        );
        let owner =
            LevelOneType17CommonDyingOwner::from_entity_metadata(true, &entity, &metadata, visit)
                .unwrap();
        let effects = Cell::new(0);
        let movers = Cell::new(0);

        let outcome = owner
            .tick_entity_normal(
                &mut entity,
                request(visit, 9_001_000, (), ()),
                || CommonDyingEffectInputs {
                    descriptor_effect_raw: RetailRuntimeValue::Known(None),
                    attached_mass: RetailRuntimeValue::Unresolved,
                },
                |_| effects.set(effects.get() + 1),
                |_, velocity| {
                    movers.set(movers.get() + 1);
                    *velocity = [40, 50, 60];
                },
            )
            .unwrap();
        let expected_velocity = [
            damp_common_dying_axis(40, 9_001_000),
            50,
            damp_common_dying_axis(60, 9_001_000),
        ];
        assert_eq!((effects.get(), movers.get()), (1, 1));
        assert_eq!(entity.velocity_raw(), expected_velocity);
        assert_eq!(
            outcome,
            Type17CommonDyingLiveFrameOutcome::TransitionSuppressed {
                visit,
                committed_prefix: CommonDyingCallbackPrefix {
                    elapsed_ms: 9_001,
                    lifetime_status:
                        crate::common_dying::CommonDyingLifetimeStatus::OwnerTransitionDue,
                },
                callback_evidence: Type17CommonDyingCallbackEvidence::Detailed {
                    selected_effect_raw: 0,
                },
                linear_velocity_raw: expected_velocity,
            }
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_some());
        assert_eq!(
            entity
                .current_behavior_context
                .map(|context| { context.map(BehaviorContextRuntime::active_style) }),
            RetailRuntimeValue::Known(Some(ActiveBehaviorStyle::Audited(
                COMMON_ACTOR_DYING_ACTIVE_STYLE
            )))
        );
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(DEFERRED_DESTROY_PENDING_STATE_BIT),
            RetailRuntimeValue::Known(0)
        );
    }

    #[test]
    fn unresolved_post_unwind_gate_reports_committed_effect_mover_and_velocity() {
        let metadata = exact_metadata();
        let (mut entity, visit) = exact_entity(5);
        entity
            .collision
            .state_flags_at_0x08
            .invalidate(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT);
        let owner =
            LevelOneType17CommonDyingOwner::from_entity_metadata(true, &entity, &metadata, visit)
                .unwrap();
        let effects = Cell::new(0);
        let movers = Cell::new(0);

        let result = owner.tick_entity_normal(
            &mut entity,
            request(visit, 9_001_000, (), ()),
            || CommonDyingEffectInputs {
                descriptor_effect_raw: RetailRuntimeValue::Known(None),
                attached_mass: RetailRuntimeValue::Unresolved,
            },
            |_| effects.set(effects.get() + 1),
            |_, velocity| {
                movers.set(movers.get() + 1);
                *velocity = [-40, -50, -60];
            },
        );
        let expected_velocity = [
            damp_common_dying_axis(-40, 9_001_000),
            -50,
            damp_common_dying_axis(-60, 9_001_000),
        ];
        assert_eq!(
            result,
            Err(Type17CommonDyingLiveFrameError::TransitionGateUnresolved {
                visit,
                committed_prefix: CommonDyingCallbackPrefix {
                    elapsed_ms: 9_001,
                    lifetime_status:
                        crate::common_dying::CommonDyingLifetimeStatus::OwnerTransitionDue,
                },
                callback_evidence: Type17CommonDyingCallbackEvidence::Detailed {
                    selected_effect_raw: 0,
                },
                linear_velocity_raw: expected_velocity,
            })
        );
        assert_eq!((effects.get(), movers.get()), (1, 1));
        assert_eq!(entity.velocity_raw(), expected_velocity);
        assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_some());
        assert_eq!(
            entity
                .current_behavior_context
                .map(|context| { context.map(BehaviorContextRuntime::active_style) }),
            RetailRuntimeValue::Known(Some(ActiveBehaviorStyle::Audited(
                COMMON_ACTOR_DYING_ACTIVE_STYLE
            )))
        );
    }

    #[v2k_test_support::retail_test]
    fn retail_level_one_authenticates_exactly_four_type17_model256_receipts() {
        let dir = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&dir).expect("init session");
        session
            .load_auxiliary_ovl(3, 1)
            .expect("load normal-tier first-world resources");
        session
            .load_level_by_id(13, 1)
            .expect("load normal-tier Level 1");
        let type_models = session.cache.global_entity_model_table();
        let type_metadata: Vec<_> = type_models
            .iter()
            .copied()
            .enumerate()
            .map(|(entity_type, model_slots)| {
                session
                    .cache
                    .global_entity_type(entity_type)
                    .map(EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(EntityTypeRuntimeMetadata {
                        model_slots,
                        ..EntityTypeRuntimeMetadata::default()
                    })
            })
            .collect();
        let metadata = &type_metadata[TYPE17_COMMON_DYING_ENTITY_TYPE as usize];
        assert_eq!(
            metadata
                .initializer
                .as_ref()
                .map(|initializer| initializer.alternate_behavior_class_ref),
            Some(COMMON_DYING_BEHAVIOR_CLASS_ID)
        );
        assert_eq!(
            session
                .cache
                .level_desc()
                .expect("Level-1 descriptor")
                .entities
                .iter()
                .filter(|spawn| spawn.entity_type == TYPE17_COMMON_DYING_ENTITY_TYPE)
                .map(|spawn| spawn.index)
                .collect::<Vec<_>>(),
            FRESH_LEVEL1_TYPE17_COMMON_DYING_SPAWN_INDICES
        );

        let mut world_fx = WorldFx::new();
        let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
            session.cache.level_desc().expect("Level-1 descriptor"),
            &type_metadata,
            session.cache.terrain(),
            0,
            &mut world_fx,
        )
        .expect("fresh type-17 birth publication");
        let ids = manager
            .iter_all()
            .filter(|entity| entity.entity_type == TYPE17_COMMON_DYING_ENTITY_TYPE)
            .map(|entity| entity.id)
            .collect::<Vec<_>>();
        assert_eq!(ids.len(), 4);
        let first_id = ids[0];
        let mut first_receipt = None;
        for entity_id in ids {
            let entity = manager
                .entity_mut_for_test(entity_id)
                .expect("captured type-17 allocation");
            assert_eq!(
                entity.select_active_model_slot(1),
                Some(TYPE17_COMMON_DYING_MODEL_ID)
            );
            entity.collision.health_raw = RetailRuntimeValue::Known(0);
            entity.collision.state_flags_at_0x08.overwrite(
                ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT | DEFERRED_DESTROY_STATE_WRITE_MASK,
                0,
            );
            entity.current_behavior_context = RetailRuntimeValue::Known(Some(
                common_dying_behavior_context(COMMON_ACTOR_DYING_ACTIVE_STYLE, None),
            ));
            // The real class-12 transition has already cleared the two birth
            // initializer tasks before publishing its sole Primary owner.
            entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
            entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
            let task_id = entity
                .actor_tasks
                .replace_prepared(ActorTaskSlot::Primary, prepared_task(entity.id));
            let visit = ActorTaskVisit {
                slot: ActorTaskSlot::Primary,
                task_id,
            };
            let receipt =
                LevelOneType17CommonDyingOwner::from_entity_metadata(true, entity, metadata, visit)
                    .expect("exact authored receipt");
            if entity_id == first_id {
                first_receipt = Some((receipt, visit));
            }
        }

        let (receipt, visit) = first_receipt.expect("first captured receipt");
        assert!(matches!(
            receipt
                .tick_published_normal(
                    &mut manager,
                    request(visit, 9_001_000, (), ()),
                    || CommonDyingEffectInputs {
                        descriptor_effect_raw: RetailRuntimeValue::Known(None),
                        attached_mass: RetailRuntimeValue::Unresolved,
                    },
                    |_| {},
                    |_, _| 0,
                )
                .unwrap(),
            Type17CommonDyingLiveFrameOutcome::DeferredDestroyStaged {
                entity_id,
                ..
            } if entity_id == first_id
        ));
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), [first_id]);
        assert!(manager.iter_all().any(|entity| entity.id == first_id));
        assert_eq!(
            manager.cleanup_pending_actor_deferred_destroys(),
            [first_id]
        );
        assert!(manager.iter_all().all(|entity| entity.id != first_id));
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
    }
}
