//! Allocation-authenticated production owner for Type 60's class-48 task.
//!
//! Retail `FUN_00406DC0` owns only the exploding-ring callback.  The common
//! actor scheduler supplies its already-resolved elapsed value; the callback
//! itself consumes no RNG.  It sets entity state bit `0x80`, clamps bound
//! control output one to `0x1000..=0xd000`, and subtracts
//! `((elapsed >> 12) * effective) >> 7`.  A subtraction landing exactly on
//! zero continues.  Only a strictly larger decrement returns the `0x9c00`
//! transition token, after first publishing zero.
//!
//! The task wrapper then selects Type 60's class-2 alternate.  That terminal
//! clears all three task slots and stages ordinary deferred destruction while
//! preserving health 1 and the clear generic-dying bit: natural task expiry
//! does not replay generic death.
//!
//! Class 49's `FUN_0040BD20` and hard-water `FUN_004141D0` create the same
//! class-48 task through `FUN_00438080`. Neither its initializer nor callback
//! branches on the caller or model. Their constructor provenance instead owns
//! model selection: default 243 for the explosion tail, four explicit 130/132
//! overrides for water entry. Both retain exact task and presentation custody.

use crate::actor_task_dispatcher::{ActorTaskRuntime, ActorTaskRuntimeFamily};
use crate::actor_task_owner::{ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags};
use crate::entity::{world_position_raw, Entity, EntityManager};
use crate::entity_behavior::{
    behavior_program, ActiveBehaviorStyle, BehaviorChoiceListSource, BehaviorDescriptorIdentity,
    DeathCallbackPolicy, QUIET_DEATH_BEHAVIOR_PROGRAM,
};
use crate::entity_collision_state::{
    RetailRuntimeValue, DEFERRED_DESTROY_PENDING_STATE_BIT, DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
};
use crate::entity_scheduler::COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT;
use crate::main_base_abort::MainBaseAbortActorLease;
use crate::type60_exploding_ring::{
    exact_type60_ring_metadata, Type60ConstructionProvenance, Type60ExplodingRingTaskLease,
    TYPE60_RING_CAPABILITY_FLAGS, TYPE60_RING_ENTITY_TYPE, TYPE60_RING_INITIAL_BEHAVIOR_CLASS,
    TYPE60_RING_INITIAL_HEALTH_RAW, TYPE60_RING_MASS_RAW,
};
use crate::world_fx::WorldFx;

pub const TYPE60_EXPLODING_RING_CONTROL_MIN_RAW: u32 = 0x1000;
pub const TYPE60_EXPLODING_RING_CONTROL_MAX_RAW: u32 = 0xd000;
pub const TYPE60_EXPLODING_RING_CALLBACK_STATE_BIT: u32 = 0x80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type60ExplodingRingProductionBlock {
    TypeMetadataUnavailable,
    TypeMetadataMismatch,
    CurrentBehaviorContextUnresolved,
    SchedulerOwnerStateUnresolved,
    RemoteSchedulerOwnerUnsupported,
    SchedulerCallbackDisabled,
    DeferredDestroyStateUnresolved,
    CallbackSequenceExhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type60ExplodingRingProductionDrop {
    EntityUnavailable,
    ActorLeaseMismatch {
        expected: MainBaseAbortActorLease,
        actual: MainBaseAbortActorLease,
    },
    EntityInactive,
    WrongEntityType {
        actual: u32,
    },
    UnsupportedConstructionProvenance {
        actual: Option<Type60ConstructionProvenance>,
    },
    UnexpectedMassOrCapabilities,
    LiveModelMismatch,
    ConstructorStateMismatch,
    InitialBehaviorMismatch,
    CurrentBehaviorContextMismatch,
    UnexpectedRelationAttachment {
        actual: Option<u32>,
    },
    AlreadyDying,
    DeferredDestroyAlreadyPending,
    DeferredDestroyAlreadyQueued,
    PrimaryTaskMissingOrReplaced,
    WrongTaskFamily {
        actual: ActorTaskRuntimeFamily,
    },
    TaskAllocationMismatch,
    TaskSequenceMismatch {
        expected: u64,
        actual: u64,
    },
    AdditionalPublishedTask {
        slot: ActorTaskSlot,
    },
    TaskWrapperNotRunnable,
    PresentationStateMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type60ExplodingRingProductionOutcome {
    Continuing {
        entity_id: u32,
        callback_sequence: u64,
        callback_elapsed_micros: u32,
        control_before_raw: u16,
        effective_control_raw: u32,
        decrement_raw: u32,
        control_after_raw: u16,
    },
    TerminalClass2 {
        entity_id: u32,
        callback_sequence: u64,
        callback_elapsed_micros: u32,
        control_before_raw: u16,
        effective_control_raw: u32,
        decrement_raw: u32,
    },
    Blocked {
        entity_id: u32,
        reason: Type60ExplodingRingProductionBlock,
    },
    Dropped {
        entity_id: u32,
        reason: Type60ExplodingRingProductionDrop,
    },
}

/// Linear custody for one exact class-48 Primary allocation.
/// Deliberately neither `Clone` nor `Copy`.
#[derive(Debug, PartialEq, Eq)]
pub struct Type60ExplodingRingProductionOwner {
    task_lease: Type60ExplodingRingTaskLease,
    next_callback_sequence: u64,
}

impl Type60ExplodingRingProductionOwner {
    pub(crate) const fn adopt(task_lease: Type60ExplodingRingTaskLease) -> Self {
        Self {
            task_lease,
            next_callback_sequence: 1,
        }
    }

    pub(crate) const fn entity_id(&self) -> u32 {
        self.task_lease.actor().entity_id
    }

    pub(crate) const fn actor_lease(&self) -> MainBaseAbortActorLease {
        self.task_lease.actor()
    }

    pub(crate) const fn task_lease(&self) -> Type60ExplodingRingTaskLease {
        self.task_lease
    }

    pub(crate) const fn next_callback_sequence(&self) -> u64 {
        self.next_callback_sequence
    }

    /// Duplicate linear custody only inside the isolated Main Base abort
    /// transaction, paired with the matching forked EntityManager.
    pub(crate) const fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            task_lease: self.task_lease,
            next_callback_sequence: self.next_callback_sequence,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Type60ExplodingRingProductionTick {
    pub outcome: Type60ExplodingRingProductionOutcome,
    pub retained_owner: Option<Type60ExplodingRingProductionOwner>,
}

struct PreparedType60Frame {
    callback_sequence: u64,
    callback_elapsed_micros: u32,
    control_before_raw: u16,
    effective_control_raw: u32,
    decrement_raw: u32,
    control_after_raw: u16,
    terminal: bool,
    quiet_context: crate::entity_behavior::BehaviorContextRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Type60ControlAdvance {
    effective_control_raw: u32,
    decrement_raw: u32,
    control_after_raw: u16,
    terminal: bool,
}

enum Type60AdmissionFailure {
    Blocked(Type60ExplodingRingProductionBlock),
    Dropped(Type60ExplodingRingProductionDrop),
}

pub(crate) fn tick_type60_exploding_ring_production_owner(
    manager: &mut EntityManager,
    owner: Type60ExplodingRingProductionOwner,
    callback_elapsed_micros: u32,
    world_fx: &mut WorldFx,
) -> Type60ExplodingRingProductionTick {
    let entity_id = owner.entity_id();
    let prepared = match prepare_type60_frame(manager, &owner, callback_elapsed_micros, world_fx) {
        Ok(prepared) => prepared,
        Err(Type60AdmissionFailure::Blocked(reason)) => {
            return Type60ExplodingRingProductionTick {
                outcome: Type60ExplodingRingProductionOutcome::Blocked { entity_id, reason },
                retained_owner: Some(owner),
            };
        }
        Err(Type60AdmissionFailure::Dropped(reason)) => {
            return Type60ExplodingRingProductionTick {
                outcome: Type60ExplodingRingProductionOutcome::Dropped { entity_id, reason },
                retained_owner: None,
            };
        }
    };
    commit_type60_frame(manager, owner, prepared, world_fx)
}

fn prepare_type60_frame(
    manager: &EntityManager,
    owner: &Type60ExplodingRingProductionOwner,
    callback_elapsed_micros: u32,
    world_fx: &WorldFx,
) -> Result<PreparedType60Frame, Type60AdmissionFailure> {
    let entity_id = owner.entity_id();
    let Some(observation) = manager.main_base_abort_actor_observation(entity_id) else {
        return Err(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::EntityUnavailable,
        ));
    };
    if observation.lease != owner.actor_lease() {
        return Err(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::ActorLeaseMismatch {
                expected: observation.lease,
                actual: owner.actor_lease(),
            },
        ));
    }
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .expect("an observed Type-60 allocation remains in the immutable live list");
    authenticate_type60_entity(manager, entity, owner, world_fx)?;

    let Some(ActorTaskRuntime::ExplodingRing(task_state)) =
        entity.actor_tasks.task_state(owner.task_lease.task_id())
    else {
        unreachable!("task-family authentication retains the exploding-ring state")
    };
    let control_before_raw = task_state.control_output_1_raw();
    let advance = advance_type60_control(control_before_raw, callback_elapsed_micros);
    let context = match entity.current_behavior_context {
        RetailRuntimeValue::Known(Some(context)) => context,
        RetailRuntimeValue::Known(None) => {
            return Err(Type60AdmissionFailure::Dropped(
                Type60ExplodingRingProductionDrop::CurrentBehaviorContextMismatch,
            ))
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type60AdmissionFailure::Blocked(
                Type60ExplodingRingProductionBlock::CurrentBehaviorContextUnresolved,
            ))
        }
    };
    let quiet_context = context
        .reselect_audited_type_default(
            &QUIET_DEATH_BEHAVIOR_PROGRAM,
            QUIET_DEATH_BEHAVIOR_PROGRAM.initial_style_table_index_raw,
            QUIET_DEATH_BEHAVIOR_PROGRAM.initial_style,
        )
        .ok_or(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::CurrentBehaviorContextMismatch,
        ))?;

    Ok(PreparedType60Frame {
        callback_sequence: owner.next_callback_sequence,
        callback_elapsed_micros,
        control_before_raw,
        effective_control_raw: advance.effective_control_raw,
        decrement_raw: advance.decrement_raw,
        control_after_raw: advance.control_after_raw,
        terminal: advance.terminal,
        quiet_context,
    })
}

fn advance_type60_control(
    control_before_raw: u16,
    callback_elapsed_micros: u32,
) -> Type60ControlAdvance {
    let effective_control_raw = u32::from(control_before_raw).clamp(
        TYPE60_EXPLODING_RING_CONTROL_MIN_RAW,
        TYPE60_EXPLODING_RING_CONTROL_MAX_RAW,
    );
    let decrement_raw = (callback_elapsed_micros >> 12).wrapping_mul(effective_control_raw) >> 7;
    let terminal = decrement_raw > u32::from(control_before_raw);
    let control_after_raw = if terminal {
        0
    } else {
        control_before_raw.wrapping_sub(decrement_raw as u16)
    };
    Type60ControlAdvance {
        effective_control_raw,
        decrement_raw,
        control_after_raw,
        terminal,
    }
}

fn authenticate_type60_entity(
    manager: &EntityManager,
    entity: &Entity,
    owner: &Type60ExplodingRingProductionOwner,
    world_fx: &WorldFx,
) -> Result<(), Type60AdmissionFailure> {
    if !entity.active {
        return Err(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::EntityInactive,
        ));
    }
    if entity.entity_type != TYPE60_RING_ENTITY_TYPE || entity.authored_spawn_index.is_some() {
        return Err(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::WrongEntityType {
                actual: entity.entity_type,
            },
        ));
    }
    let Some(provenance) = entity.type60_construction_provenance() else {
        return Err(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::UnsupportedConstructionProvenance { actual: None },
        ));
    };
    let metadata = manager
        .type_runtime_metadata(TYPE60_RING_ENTITY_TYPE)
        .ok_or(Type60AdmissionFailure::Blocked(
            Type60ExplodingRingProductionBlock::TypeMetadataUnavailable,
        ))?;
    if !exact_type60_ring_metadata(metadata) {
        return Err(Type60AdmissionFailure::Blocked(
            Type60ExplodingRingProductionBlock::TypeMetadataMismatch,
        ));
    }
    if entity.mass_raw != TYPE60_RING_MASS_RAW
        || entity.capability_flags != TYPE60_RING_CAPABILITY_FLAGS
    {
        return Err(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::UnexpectedMassOrCapabilities,
        ));
    }
    // BD20 leaves all four overrides zero. Resolve that default from the
    // authenticated Type-60 row, while 141D0 retains its explicit water
    // overrides. The selected model alone never proves constructor custody.
    let overrides = provenance.model_overrides();
    let expected_slots = std::array::from_fn(|slot| {
        overrides[slot].or(Some(usize::from(metadata.model_slots[slot])))
    });
    let expected_model = Some(provenance.presentation_model_id());
    if entity.model_slots != expected_slots || entity.model_index != expected_model {
        return Err(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::LiveModelMismatch,
        ));
    }
    if entity.collision.health_raw != RetailRuntimeValue::Known(TYPE60_RING_INITIAL_HEALTH_RAW)
        || entity.collision.pre_health_damage_buffer_raw != RetailRuntimeValue::Known(0)
        || entity.collision.constructor_sound_attachment_id_at_0x8c
            != RetailRuntimeValue::Known(None)
        || entity.actor_common_axis_descriptor
            != RetailRuntimeValue::Known(
                metadata
                    .initializer
                    .as_ref()
                    .expect("exact Type-60 metadata retains its initializer")
                    .common_axis_descriptor,
            )
    {
        return Err(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::ConstructorStateMismatch,
        ));
    }
    let RetailRuntimeValue::Known(Some(initial_behavior)) = entity.initial_behavior else {
        return Err(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::InitialBehaviorMismatch,
        ));
    };
    if initial_behavior.choice_index != 0
        || initial_behavior.program.class_id != TYPE60_RING_INITIAL_BEHAVIOR_CLASS
    {
        return Err(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::InitialBehaviorMismatch,
        ));
    }
    match entity.current_behavior_context {
        RetailRuntimeValue::Known(Some(context)) if type60_named_context_matches(context) => {}
        RetailRuntimeValue::Known(_) => {
            return Err(Type60AdmissionFailure::Dropped(
                Type60ExplodingRingProductionDrop::CurrentBehaviorContextMismatch,
            ))
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type60AdmissionFailure::Blocked(
                Type60ExplodingRingProductionBlock::CurrentBehaviorContextUnresolved,
            ))
        }
    }
    let scheduler_bits = entity.collision.state_flags_at_0x08.masked(
        REMOTE_OWNED_STATE_BIT | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | DYING_STATE_BIT,
    );
    let RetailRuntimeValue::Known(scheduler_bits) = scheduler_bits else {
        return Err(Type60AdmissionFailure::Blocked(
            Type60ExplodingRingProductionBlock::SchedulerOwnerStateUnresolved,
        ));
    };
    if scheduler_bits & REMOTE_OWNED_STATE_BIT != 0 {
        return Err(Type60AdmissionFailure::Blocked(
            Type60ExplodingRingProductionBlock::RemoteSchedulerOwnerUnsupported,
        ));
    }
    if scheduler_bits & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT == 0 {
        return Err(Type60AdmissionFailure::Blocked(
            Type60ExplodingRingProductionBlock::SchedulerCallbackDisabled,
        ));
    }
    if scheduler_bits & DYING_STATE_BIT != 0 {
        return Err(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::AlreadyDying,
        ));
    }
    if entity.attached_to.is_some() {
        return Err(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::UnexpectedRelationAttachment {
                actual: entity.attached_to,
            },
        ));
    }
    match entity
        .collision
        .state_flags_at_0x08
        .masked(DEFERRED_DESTROY_PENDING_STATE_BIT)
    {
        RetailRuntimeValue::Known(0) => {}
        RetailRuntimeValue::Known(_) => {
            return Err(Type60AdmissionFailure::Dropped(
                Type60ExplodingRingProductionDrop::DeferredDestroyAlreadyPending,
            ))
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type60AdmissionFailure::Blocked(
                Type60ExplodingRingProductionBlock::DeferredDestroyStateUnresolved,
            ))
        }
    }
    if manager
        .pending_actor_deferred_destroy_ids()
        .contains(&entity.id)
    {
        return Err(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::DeferredDestroyAlreadyQueued,
        ));
    }

    let task_lease = owner.task_lease;
    if entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) != Some(task_lease.task_id()) {
        return Err(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::PrimaryTaskMissingOrReplaced,
        ));
    }
    for slot in [ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary] {
        if entity.actor_tasks.task_in_slot(slot).is_some() {
            return Err(Type60AdmissionFailure::Dropped(
                Type60ExplodingRingProductionDrop::AdditionalPublishedTask { slot },
            ));
        }
    }
    let Some(task) = entity.actor_tasks.task_state(task_lease.task_id()) else {
        return Err(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::PrimaryTaskMissingOrReplaced,
        ));
    };
    let ActorTaskRuntime::ExplodingRing(task_state) = task else {
        return Err(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::WrongTaskFamily {
                actual: task.family(),
            },
        ));
    };
    if task_state.next_callback_sequence() != owner.next_callback_sequence {
        return Err(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::TaskSequenceMismatch {
                expected: owner.next_callback_sequence,
                actual: task_state.next_callback_sequence(),
            },
        ));
    }
    if owner.next_callback_sequence == u64::MAX {
        return Err(Type60AdmissionFailure::Blocked(
            Type60ExplodingRingProductionBlock::CallbackSequenceExhausted,
        ));
    }
    if entity.actor_tasks.wrapper_flags(task_lease.task_id())
        != Some(ActorTaskWrapperFlags {
            alive: true,
            in_callback: false,
        })
    {
        return Err(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::TaskWrapperNotRunnable,
        ));
    }
    let associated = world_fx
        .exploding_rings()
        .iter()
        .filter(|ring| ring.associated_entity_id() == entity.id)
        .collect::<Vec<_>>();
    // Both BD20 and 141D0 publish wrapping position words. Entity and WorldFx use
    // signed and unsigned X/Z images of those same words, so compare in the
    // retail domain (as in the Type-60 Main Base abort admission). Comparing
    // floats drops the animation owner whenever either word has bit 15 set.
    if associated.len() != 1
        || associated[0].model_id != provenance.presentation_model_id()
        || world_position_raw(associated[0].position) != entity.position_raw()
        || associated[0].control_output_1_raw() != task_state.control_output_1_raw()
    {
        return Err(Type60AdmissionFailure::Dropped(
            Type60ExplodingRingProductionDrop::PresentationStateMismatch,
        ));
    }
    Ok(())
}

fn commit_type60_frame(
    manager: &mut EntityManager,
    mut owner: Type60ExplodingRingProductionOwner,
    prepared: PreparedType60Frame,
    world_fx: &mut WorldFx,
) -> Type60ExplodingRingProductionTick {
    let entity_id = owner.entity_id();
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id: owner.task_lease.task_id(),
    };
    {
        let entity = manager
            .type60_exploding_ring_entity_mut(entity_id)
            .expect("preflighted Type-60 allocation remains live and authenticated");
        let began = entity.actor_tasks.begin_exact_visit_with(visit, |_| ());
        assert_eq!(
            began,
            Some(()),
            "preflighted Type-60 wrapper must enter its exact Primary callback"
        );
        entity.collision.state_flags_at_0x08.overwrite(
            TYPE60_EXPLODING_RING_CALLBACK_STATE_BIT,
            TYPE60_EXPLODING_RING_CALLBACK_STATE_BIT,
        );
        let Some(ActorTaskRuntime::ExplodingRing(state)) =
            entity.actor_tasks.exact_callback_state_mut(visit)
        else {
            unreachable!("begun exact Type-60 visit exposes its class-48 callback state")
        };
        assert!(state.commit_callback_control_output_1_raw(prepared.control_after_raw));
    }
    let presentation_updated = world_fx.set_type60_exploding_ring_control_raw(
        entity_id,
        prepared.control_before_raw,
        prepared.control_after_raw,
    );
    debug_assert!(presentation_updated);
    {
        let entity = manager
            .type60_exploding_ring_entity_mut(entity_id)
            .expect("Type-60 allocation remains live until callback unwind");
        assert!(
            entity.actor_tasks.finish_exact_visit(visit),
            "preflighted Type-60 wrapper must survive callback unwind"
        );
    }

    if prepared.terminal {
        {
            let entity = manager
                .type60_exploding_ring_entity_mut(entity_id)
                .expect("terminal Type-60 allocation remains live until the deferred sweep");
            entity.current_behavior_context =
                RetailRuntimeValue::Known(Some(prepared.quiet_context));
            for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
                entity.actor_tasks.clear_slot(slot);
            }
            entity.mark_actor_deferred_destroy_pending();
        }
        let removed = world_fx.remove_type60_exploding_ring(entity_id);
        debug_assert!(removed);
        manager.queue_actor_deferred_destroy(entity_id);
        Type60ExplodingRingProductionTick {
            outcome: Type60ExplodingRingProductionOutcome::TerminalClass2 {
                entity_id,
                callback_sequence: prepared.callback_sequence,
                callback_elapsed_micros: prepared.callback_elapsed_micros,
                control_before_raw: prepared.control_before_raw,
                effective_control_raw: prepared.effective_control_raw,
                decrement_raw: prepared.decrement_raw,
            },
            retained_owner: None,
        }
    } else {
        owner.next_callback_sequence += 1;
        Type60ExplodingRingProductionTick {
            outcome: Type60ExplodingRingProductionOutcome::Continuing {
                entity_id,
                callback_sequence: prepared.callback_sequence,
                callback_elapsed_micros: prepared.callback_elapsed_micros,
                control_before_raw: prepared.control_before_raw,
                effective_control_raw: prepared.effective_control_raw,
                decrement_raw: prepared.decrement_raw,
                control_after_raw: prepared.control_after_raw,
            },
            retained_owner: Some(owner),
        }
    }
}

fn type60_named_context_matches(context: crate::entity_behavior::BehaviorContextRuntime) -> bool {
    let Some(program) = behavior_program(u32::from(TYPE60_RING_INITIAL_BEHAVIOR_CLASS)) else {
        return false;
    };
    context.choice_list_source() == RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        && context.descriptor() == BehaviorDescriptorIdentity::Named(program)
        && context.active_style() == ActiveBehaviorStyle::Audited(program.initial_style)
        && context.active_style().death_callback_policy() == DeathCallbackPolicy::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_collision_state::{EntityInitializerSpec, EntityTypeRuntimeMetadata};
    use crate::gameplay_notifications::GameplayNotifications;
    use crate::level::LevelState;
    use crate::resource_cache::ResourceCache;
    use crate::specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler, Type60ExplodingRingCustodyTakeBlock,
    };
    use crate::type60_exploding_ring::{
        HardWaterType60ConstructionRequest, HardWaterType60Severity, HostType60Allocator,
        Type60ConstructionOutcome, Type60ConstructionRequest, Type60ConstructorRngDisposition,
        TYPE60_COMPONENT_TOPOLOGY, TYPE60_RING_ALTERNATE_BEHAVIOR_CLASS,
        TYPE60_RING_COMMON_AXIS_DESCRIPTOR, TYPE60_RING_DAMAGE_PROFILE,
        TYPE60_RING_INITIALIZER_STATE_RAW, TYPE60_RING_INITIAL_CONTROL_OUTPUT_1_RAW,
        TYPE60_RING_MODEL_ID,
    };
    use v2k_formats::collision::BehaviorChoice;
    use v2k_formats::levels::LevelDescriptor;
    use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};

    fn exact_type60_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            cured_model_presentation_sound_id: RetailRuntimeValue::Unresolved,
            common_world_effects: RetailRuntimeValue::Unresolved,
            detailed_sound_policy: RetailRuntimeValue::Unresolved,
            model_slots: [TYPE60_RING_MODEL_ID as u16; 4],
            mass_raw: TYPE60_RING_MASS_RAW,
            capability_flags: TYPE60_RING_CAPABILITY_FLAGS,
            initial_health_raw: Some(TYPE60_RING_INITIAL_HEALTH_RAW),
            damage_profile: Some(TYPE60_RING_DAMAGE_PROFILE),
            accepted_hit_presentation_sound_id: RetailRuntimeValue::Known(None),
            infected_model_presentation_sound_id: RetailRuntimeValue::Known(None),
            death_sound_id: RetailRuntimeValue::Known(None),
            target_warning_sound_id: RetailRuntimeValue::Unresolved,
            constructor_sound_attachment_id: RetailRuntimeValue::Known(None),
            generic_hit_sound_id: RetailRuntimeValue::Known(None),
            search_attack_optional_prelude_sound_id: RetailRuntimeValue::Unresolved,
            search_attack_aim_sound_id: RetailRuntimeValue::Unresolved,
            search_attack_aim_sound_period_raw: RetailRuntimeValue::Unresolved,
            run_away_optional_sound_id: RetailRuntimeValue::Unresolved,
            run_away_sound_period_raw: RetailRuntimeValue::Unresolved,
            terrain_contact_task_lifetime_ms: RetailRuntimeValue::Unresolved,
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(None),
            sub_b_lateral_descriptor: RetailRuntimeValue::Known(None),
            sub_c_lift_descriptor: RetailRuntimeValue::Known(None),
            sub_d_steering_descriptor: RetailRuntimeValue::Known(None),
            sub_f_swimming_descriptor: RetailRuntimeValue::Known(None),
            model_variable_count_raw: RetailRuntimeValue::Unresolved,
            projectile_emitter_descriptor: RetailRuntimeValue::Known(None),
            sub_n_payload: None,
            status_component_descriptor: RetailRuntimeValue::Known(None),
            actor_animation_descriptor: RetailRuntimeValue::Known(None),
            sub_h_external_frame_descriptor: RetailRuntimeValue::Known(None),
            sub_j_attachment_descriptor: RetailRuntimeValue::Known(None),
            common_mover_topology: RetailRuntimeValue::Known(TYPE60_COMPONENT_TOPOLOGY),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: TYPE60_RING_INITIALIZER_STATE_RAW,
                common_axis_descriptor: TYPE60_RING_COMMON_AXIS_DESCRIPTOR,
                behavior_choices: vec![BehaviorChoice {
                    weight_rule_id: 1,
                    weight_multiplier: 1,
                    behavior_class_id: u32::from(TYPE60_RING_INITIAL_BEHAVIOR_CLASS),
                }]
                .into_boxed_slice(),
                behavior_rule_ref: 1,
                alternate_behavior_class_ref: u32::from(TYPE60_RING_ALTERNATE_BEHAVIOR_CLASS),
            }),
            common_mover_gkl_payloads: RetailRuntimeValue::Unresolved,
        }
    }

    fn exact_type60_manager() -> EntityManager {
        let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 61];
        metadata[TYPE60_RING_ENTITY_TYPE as usize] = exact_type60_metadata();
        EntityManager::from_entities_with_type_metadata_for_test(Vec::new(), metadata, true)
    }

    fn flat_terrain() -> TerrainGrid {
        TerrainGrid {
            header: [0; 5],
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

    fn strict_resources() -> ResourceCache {
        let mut resources = ResourceCache::new(Vec::new());
        resources.load_level(LevelState {
            source_path: "type60-production-test.ovl".into(),
            system_level: None,
            fixup_data: None,
            fixup_code: None,
            strings: Vec::new(),
            sprites: None,
            params: None,
            display_modes: None,
            fog_gradient: None,
            color_palettes: None,
            models: None,
            anim_frames: None,
            terrain: Some(flat_terrain()),
            anim_sound: None,
            collision: None,
            level: Some(LevelDescriptor {
                raw_header: [0; 0xd0],
                name: "type60-production-test".into(),
                world_style: 0,
                terrain_sprite_base: 0,
                sky_color_index: 0,
                sky_model: 0,
                main_base_abort_sky_color_index: 0,
                main_base_abort_sky_model: 0,
                terrain_draw_depth: 0,
                sub_count: 0,
                campaign_record_count: 0,
                entities: Vec::new(),
                campaign_records: Vec::new(),
            }),
            linkage: None,
        });
        resources
    }

    fn construct_hard_water_ring(
        manager: &mut EntityManager,
        world_fx: &mut WorldFx,
        terrain: &TerrainGrid,
        position_raw: [i16; 3],
        severity: HardWaterType60Severity,
    ) -> Type60ExplodingRingTaskLease {
        let outcome = manager.construct_hard_water_type60_ring(
            HardWaterType60ConstructionRequest::new(position_raw, severity),
            terrain,
            world_fx,
        );
        let Type60ConstructionOutcome::ActorLinked(receipt) = outcome else {
            panic!("the host allocator publishes an exact hard-water Type-60 actor")
        };
        receipt
            .primary_task_lease()
            .expect("the host allocator publishes class-48 Primary")
    }

    fn construct_class49_ring(
        manager: &mut EntityManager,
        world_fx: &mut WorldFx,
        terrain: &TerrainGrid,
        position_raw: [i16; 3],
    ) -> Type60ExplodingRingTaskLease {
        // Use the actual shared constructor called after BD20's radial pass,
        // not a hard-water actor relabelled with a different provenance.
        let outcome = manager.construct_type60_exploding_ring_with_allocator(
            Type60ConstructionRequest::class49_at(position_raw),
            terrain,
            world_fx,
            &mut HostType60Allocator,
        );
        outcome
            .primary_task_lease()
            .expect("the class-49 tail publishes class-48 Primary")
    }

    fn exploding_ring_state(
        manager: &EntityManager,
        lease: Type60ExplodingRingTaskLease,
    ) -> crate::type60_exploding_ring::Type60ExplodingRingTaskState {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == lease.actor().entity_id)
            .unwrap();
        let Some(ActorTaskRuntime::ExplodingRing(state)) =
            entity.actor_tasks.task_state(lease.task_id())
        else {
            panic!("the exact Type-60 Primary remains installed")
        };
        *state
    }

    #[test]
    fn callback_arithmetic_clamps_wraps_and_keeps_exact_zero_alive() {
        assert_eq!(
            advance_type60_control(0, 1 << 12),
            Type60ControlAdvance {
                effective_control_raw: TYPE60_EXPLODING_RING_CONTROL_MIN_RAW,
                decrement_raw: 32,
                control_after_raw: 0,
                terminal: true,
            }
        );
        assert_eq!(
            advance_type60_control(u16::MAX, 1 << 12),
            Type60ControlAdvance {
                effective_control_raw: TYPE60_EXPLODING_RING_CONTROL_MAX_RAW,
                decrement_raw: 416,
                control_after_raw: u16::MAX - 416,
                terminal: false,
            }
        );

        let equality = advance_type60_control(0x1000, 128 << 12);
        assert_eq!(equality.decrement_raw, 0x1000);
        assert_eq!(equality.control_after_raw, 0);
        assert!(!equality.terminal);
        assert!(advance_type60_control(equality.control_after_raw, 1 << 12).terminal);

        let elapsed = u32::MAX;
        let expected_wrapped =
            ((elapsed >> 12).wrapping_mul(TYPE60_EXPLODING_RING_CONTROL_MAX_RAW)) >> 7;
        assert_eq!(
            advance_type60_control(u16::MAX, elapsed).decrement_raw,
            expected_wrapped
        );
    }

    #[test]
    fn continuing_visit_commits_inside_exact_wrapper_then_unwinds_without_rng() {
        let terrain = flat_terrain();
        let mut manager = exact_type60_manager();
        let mut world_fx = WorldFx::new();
        let lease = construct_hard_water_ring(
            &mut manager,
            &mut world_fx,
            &terrain,
            [10, 20, 30],
            HardWaterType60Severity::Moderate,
        );
        let mut rng_oracle = world_fx.fork_for_main_base_abort_transaction();

        let tick = tick_type60_exploding_ring_production_owner(
            &mut manager,
            Type60ExplodingRingProductionOwner::adopt(lease),
            1 << 12,
            &mut world_fx,
        );
        assert_eq!(
            tick.outcome,
            Type60ExplodingRingProductionOutcome::Continuing {
                entity_id: lease.actor().entity_id,
                callback_sequence: 1,
                callback_elapsed_micros: 1 << 12,
                control_before_raw: TYPE60_RING_INITIAL_CONTROL_OUTPUT_1_RAW,
                effective_control_raw: TYPE60_EXPLODING_RING_CONTROL_MAX_RAW,
                decrement_raw: 416,
                control_after_raw: TYPE60_RING_INITIAL_CONTROL_OUTPUT_1_RAW - 416,
            }
        );
        let retained = tick
            .retained_owner
            .expect("a continuing task retains custody");
        assert_eq!(retained.next_callback_sequence, 2);
        let state = exploding_ring_state(&manager, lease);
        assert_eq!(state.next_callback_sequence(), 2);
        assert_eq!(state.control_output_1_raw(), u16::MAX - 416);

        let entity = manager
            .iter_all()
            .find(|entity| entity.id == lease.actor().entity_id)
            .unwrap();
        assert_eq!(
            entity.actor_tasks.wrapper_flags(lease.task_id()),
            Some(ActorTaskWrapperFlags {
                alive: true,
                in_callback: false,
            })
        );
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(TYPE60_EXPLODING_RING_CALLBACK_STATE_BIT),
            RetailRuntimeValue::Known(TYPE60_EXPLODING_RING_CALLBACK_STATE_BIT)
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            world_fx.exploding_rings()[0].control_output_1_raw(),
            u16::MAX - 416
        );
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            rng_oracle.next_shared_retail_random_u16(),
            "the class-48 callback consumes no shared RNG"
        );
    }

    #[test]
    fn terminal_visit_unwinds_before_class2_clears_slots_and_stages_destroy() {
        let terrain = flat_terrain();
        let mut manager = exact_type60_manager();
        let mut world_fx = WorldFx::new();
        let lease = construct_hard_water_ring(
            &mut manager,
            &mut world_fx,
            &terrain,
            [10, 20, 30],
            HardWaterType60Severity::Severe,
        );

        let elapsed_micros = 158 << 12;
        let tick = tick_type60_exploding_ring_production_owner(
            &mut manager,
            Type60ExplodingRingProductionOwner::adopt(lease),
            elapsed_micros,
            &mut world_fx,
        );
        assert!(matches!(
            tick.outcome,
            Type60ExplodingRingProductionOutcome::TerminalClass2 {
                entity_id,
                callback_sequence: 1,
                callback_elapsed_micros,
                control_before_raw: TYPE60_RING_INITIAL_CONTROL_OUTPUT_1_RAW,
                effective_control_raw: TYPE60_EXPLODING_RING_CONTROL_MAX_RAW,
                decrement_raw: 65_728,
            } if entity_id == lease.actor().entity_id && callback_elapsed_micros == elapsed_micros
        ));
        assert!(tick.retained_owner.is_none());

        let entity = manager
            .iter_all()
            .find(|entity| entity.id == lease.actor().entity_id)
            .expect("deferred destruction retains the actor through this task pass");
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(1));
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(DEFERRED_DESTROY_PENDING_STATE_BIT),
            RetailRuntimeValue::Known(DEFERRED_DESTROY_PENDING_STATE_BIT)
        );
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot)),
            [None; 3]
        );
        assert_eq!(entity.actor_tasks.wrapper_flags(lease.task_id()), None);
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("terminal Type-60 selects a resolved quiet context")
        };
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::Named(&QUIET_DEATH_BEHAVIOR_PROGRAM)
        );
        assert_eq!(
            manager.pending_actor_deferred_destroy_ids(),
            &[lease.actor().entity_id]
        );
        assert!(world_fx.exploding_rings().is_empty());
    }

    #[test]
    fn class49_tail_default_model_runs_to_natural_class2_without_extra_rng() {
        let terrain = flat_terrain();
        let mut manager = exact_type60_manager();
        let mut world_fx = WorldFx::new();
        let mut rng_oracle = world_fx.fork_for_main_base_abort_transaction();
        let selector_word = rng_oracle.next_shared_retail_random_u16();
        let position_raw = [i16::MIN, -847, -1];
        let outcome = manager.construct_type60_exploding_ring_with_allocator(
            Type60ConstructionRequest::class49_at(position_raw),
            &terrain,
            &mut world_fx,
            &mut HostType60Allocator,
        );
        assert_eq!(
            outcome.constructor_rng(),
            Type60ConstructorRngDisposition::SelectorWord(selector_word)
        );
        let lease = outcome.primary_task_lease().unwrap();
        let entity_id = lease.actor().entity_id;
        let mut owner = Some(Type60ExplodingRingProductionOwner::adopt(lease));
        let mut terminal = false;
        for callback_index in 0..512 {
            let tick = tick_type60_exploding_ring_production_owner(
                &mut manager,
                owner
                    .take()
                    .expect("the live class-49 ring retains its task"),
                20_000,
                &mut world_fx,
            );
            match tick.outcome {
                Type60ExplodingRingProductionOutcome::Continuing {
                    control_before_raw,
                    control_after_raw,
                    ..
                } => {
                    if callback_index == 0 {
                        assert_eq!(control_before_raw, 0xffff);
                        assert_eq!(control_after_raw, 0xf97f);
                    }
                    assert!(control_after_raw < control_before_raw);
                    let [presentation] = world_fx.exploding_rings() else {
                        panic!("the tail retains exactly one entity-backed presentation")
                    };
                    assert_eq!(presentation.model_id, TYPE60_RING_MODEL_ID);
                    assert_eq!(world_position_raw(presentation.position), position_raw);
                    assert_eq!(
                        presentation.anim_vars().dynamic[1],
                        i32::from(control_after_raw)
                    );
                    assert_eq!(
                        exploding_ring_state(&manager, lease).control_output_1_raw(),
                        control_after_raw
                    );
                    owner = tick.retained_owner;
                }
                Type60ExplodingRingProductionOutcome::TerminalClass2 { .. } => {
                    assert!(tick.retained_owner.is_none());
                    terminal = true;
                    break;
                }
                outcome => {
                    panic!("the constructor-backed explosion tail lost its task: {outcome:?}")
                }
            }
        }
        assert!(
            terminal,
            "the explosion tail must expire rather than remain frozen"
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert_eq!(entity.model_slots, [Some(TYPE60_RING_MODEL_ID); 4]);
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(1));
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(0)
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("natural expiry retains the class-2 context")
        };
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::Named(&QUIET_DEATH_BEHAVIOR_PROGRAM)
        );
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot)),
            [None; 3]
        );
        assert!(world_fx.exploding_rings().is_empty());
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), &[entity_id]);
        assert_eq!(
            manager.cleanup_pending_actor_deferred_destroys(),
            vec![entity_id]
        );
        assert!(manager.iter_all().all(|entity| entity.id != entity_id));
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            rng_oracle.next_shared_retail_random_u16()
        );
    }

    #[test]
    fn class49_admission_requires_constructor_custody_and_every_default_model_slot() {
        let terrain = flat_terrain();
        for changed_field in 0..6 {
            let mut manager = exact_type60_manager();
            let mut world_fx = WorldFx::new();
            let lease = construct_class49_ring(&mut manager, &mut world_fx, &terrain, [1, 2, 3]);
            let task_before = exploding_ring_state(&manager, lease);
            let presentation_before = world_fx.exploding_rings()[0];
            let entity = manager
                .type60_exploding_ring_entity_mut(lease.actor().entity_id)
                .unwrap();
            let reason = match changed_field {
                0..=3 => {
                    entity.model_slots[changed_field] =
                        Some(crate::world_fx::HARD_WATER_ENTRY_SPLASH_MODEL_ID);
                    Type60ExplodingRingProductionDrop::LiveModelMismatch
                }
                4 => {
                    entity.model_index = Some(crate::world_fx::HARD_WATER_ENTRY_SPLASH_MODEL_ID);
                    Type60ExplodingRingProductionDrop::LiveModelMismatch
                }
                _ => {
                    entity.type60_construction_provenance = None;
                    Type60ExplodingRingProductionDrop::UnsupportedConstructionProvenance {
                        actual: None,
                    }
                }
            };
            let tick = tick_type60_exploding_ring_production_owner(
                &mut manager,
                Type60ExplodingRingProductionOwner::adopt(lease),
                20_000,
                &mut world_fx,
            );
            assert_eq!(
                tick.outcome,
                Type60ExplodingRingProductionOutcome::Dropped {
                    entity_id: lease.actor().entity_id,
                    reason
                }
            );
            assert_eq!(exploding_ring_state(&manager, lease), task_before);
            assert_eq!(world_fx.exploding_rings(), &[presentation_before]);
            assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
        }
    }

    #[test]
    fn hard_water_ring_lifecycle_crosses_signed_xz_boundaries() {
        let terrain = flat_terrain();
        let boundary_words = [i16::MAX, i16::MIN, -1];
        for severity in [
            HardWaterType60Severity::Moderate,
            HardWaterType60Severity::Severe,
        ] {
            for x_raw in boundary_words {
                for z_raw in boundary_words {
                    let position_raw = [x_raw, -847, z_raw];
                    let mut manager = exact_type60_manager();
                    let mut world_fx = WorldFx::new();
                    let lease = construct_hard_water_ring(
                        &mut manager,
                        &mut world_fx,
                        &terrain,
                        position_raw,
                        severity,
                    );
                    let entity_id = lease.actor().entity_id;
                    let mut owner = Some(Type60ExplodingRingProductionOwner::adopt(lease));
                    let mut terminal = false;
                    for _ in 0..512 {
                        let tick = tick_type60_exploding_ring_production_owner(
                            &mut manager,
                            owner.take().expect("a continuing ring retains its owner"),
                            20_000,
                            &mut world_fx,
                        );
                        match tick.outcome {
                            Type60ExplodingRingProductionOutcome::Continuing {
                                control_before_raw,
                                control_after_raw,
                                ..
                            } => {
                                assert!(control_after_raw < control_before_raw);
                                assert_eq!(
                                    world_fx.exploding_rings()[0].control_output_1_raw(),
                                    control_after_raw
                                );
                                assert_eq!(
                                    crate::entity::world_position_raw(world_fx.exploding_rings()[0].position),
                                    position_raw
                                );
                                assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
                                owner = tick.retained_owner;
                            }
                            Type60ExplodingRingProductionOutcome::TerminalClass2 { .. } => {
                                assert!(tick.retained_owner.is_none());
                                terminal = true;
                                break;
                            }
                            outcome => panic!(
                                "hard-water {severity:?} at {position_raw:?} lost its lifecycle: {outcome:?}"
                            ),
                        }
                    }
                    assert!(terminal, "bounded hard-water lifecycle at {position_raw:?}");
                    assert!(world_fx.exploding_rings().is_empty());
                    assert_eq!(manager.pending_actor_deferred_destroy_ids(), &[entity_id]);
                    let entity = manager
                        .iter_all()
                        .find(|entity| entity.id == entity_id)
                        .unwrap();
                    assert_eq!(
                        ActorTaskSlot::IN_RETAIL_TICK_ORDER
                            .map(|slot| entity.actor_tasks.task_in_slot(slot)),
                        [None; 3]
                    );
                    assert_eq!(
                        manager.cleanup_pending_actor_deferred_destroys(),
                        vec![entity_id]
                    );
                    assert!(manager.iter_all().all(|entity| entity.id != entity_id));
                    assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
                }
            }
        }
    }

    #[test]
    fn hard_water_ring_rejects_one_raw_unit_position_mismatch_without_advancing() {
        let terrain = flat_terrain();
        for axis in 0..3 {
            let mut manager = exact_type60_manager();
            let mut world_fx = WorldFx::new();
            let position_raw = [i16::MIN, -847, -1];
            let lease = construct_hard_water_ring(
                &mut manager,
                &mut world_fx,
                &terrain,
                position_raw,
                HardWaterType60Severity::Moderate,
            );
            let ring_before = world_fx.exploding_rings()[0];
            let task_before = exploding_ring_state(&manager, lease);
            let mut changed_position = position_raw;
            changed_position[axis] = changed_position[axis].wrapping_add(1);
            manager
                .type60_exploding_ring_entity_mut(lease.actor().entity_id)
                .unwrap()
                .set_position_raw(changed_position);

            let tick = tick_type60_exploding_ring_production_owner(
                &mut manager,
                Type60ExplodingRingProductionOwner::adopt(lease),
                20_000,
                &mut world_fx,
            );
            assert_eq!(
                tick.outcome,
                Type60ExplodingRingProductionOutcome::Dropped {
                    entity_id: lease.actor().entity_id,
                    reason: Type60ExplodingRingProductionDrop::PresentationStateMismatch,
                }
            );
            assert!(tick.retained_owner.is_none());
            assert_eq!(world_fx.exploding_rings(), &[ring_before]);
            assert_eq!(exploding_ring_state(&manager, lease), task_before);
            assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
        }
    }

    #[test]
    fn scheduler_ticks_type60_in_live_order_and_drops_stale_pending_custody() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let terrain = flat_terrain();
        let mut manager = exact_type60_manager();
        let mut world_fx = WorldFx::new();
        let first = construct_hard_water_ring(
            &mut manager,
            &mut world_fx,
            &terrain,
            [1, 2, 3],
            HardWaterType60Severity::Moderate,
        );
        let second = construct_hard_water_ring(
            &mut manager,
            &mut world_fx,
            &terrain,
            [4, 5, 6],
            HardWaterType60Severity::Severe,
        );
        let third = construct_class49_ring(&mut manager, &mut world_fx, &terrain, [7, 8, 9]);
        let mut scheduler = SpecializedActorTaskScheduler::default();
        assert!(scheduler
            .register_type60_exploding_ring(third)
            .unwrap()
            .is_none());
        assert!(scheduler
            .register_type60_exploding_ring(second)
            .unwrap()
            .is_none());
        assert!(scheduler
            .register_type60_exploding_ring(first)
            .unwrap()
            .is_none());
        let mut resources = strict_resources();
        let mut notifications = GameplayNotifications::new();
        let pass = scheduler.tick(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut resources,
                world_fx: &mut world_fx,
                static_damage: &mut static_damage,
                elapsed_micros: 1 << 12,
                global_elapsed_micros: 1 << 12,
                retail_tick: 7,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        assert_eq!(pass.block, None);
        assert_eq!(
            pass.outcomes
                .iter()
                .map(SpecializedActorTaskProductionOutcome::entity_id)
                .collect::<Vec<_>>(),
            vec![
                first.actor().entity_id,
                second.actor().entity_id,
                third.actor().entity_id
            ]
        );
        assert!(pass.outcomes.iter().all(|outcome| matches!(
            outcome,
            SpecializedActorTaskProductionOutcome::Type60ExplodingRing(
                Type60ExplodingRingProductionOutcome::Continuing { .. }
            )
        )));
        assert_eq!(scheduler.registered_len(), 3);

        let mut source_manager = exact_type60_manager();
        let mut source_world_fx = WorldFx::new();
        let stale = construct_hard_water_ring(
            &mut source_manager,
            &mut source_world_fx,
            &terrain,
            [7, 8, 9],
            HardWaterType60Severity::Moderate,
        );
        let mut stale_scheduler = SpecializedActorTaskScheduler::default();
        stale_scheduler
            .register_type60_exploding_ring(stale)
            .unwrap();
        let mut empty_manager = exact_type60_manager();
        let mut resources = strict_resources();
        let stale_pass = stale_scheduler.tick(
            &mut empty_manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut resources,
                world_fx: &mut source_world_fx,
                static_damage: &mut static_damage,
                elapsed_micros: 1 << 12,
                global_elapsed_micros: 1 << 12,
                retail_tick: 8,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        assert_eq!(
            stale_pass.outcomes,
            vec![SpecializedActorTaskProductionOutcome::Type60ExplodingRing(
                Type60ExplodingRingProductionOutcome::Dropped {
                    entity_id: stale.actor().entity_id,
                    reason: Type60ExplodingRingProductionDrop::EntityUnavailable,
                }
            )]
        );
        assert!(stale_scheduler.is_empty());
    }

    #[test]
    fn abort_custody_take_is_exact_and_retains_stale_allocation_claims() {
        let terrain = flat_terrain();
        let mut source_manager = exact_type60_manager();
        let mut source_fx = WorldFx::new();
        let source = construct_hard_water_ring(
            &mut source_manager,
            &mut source_fx,
            &terrain,
            [1, 2, 3],
            HardWaterType60Severity::Moderate,
        );
        let mut current_manager = exact_type60_manager();
        let mut current_fx = WorldFx::new();
        let current = construct_hard_water_ring(
            &mut current_manager,
            &mut current_fx,
            &terrain,
            [1, 2, 3],
            HardWaterType60Severity::Moderate,
        );
        assert_eq!(source.actor().entity_id, current.actor().entity_id);
        assert_ne!(source.actor(), current.actor());

        let mut scheduler = SpecializedActorTaskScheduler::default();
        scheduler.register_type60_exploding_ring(source).unwrap();
        assert_eq!(
            scheduler.take_type60_exploding_ring_for_main_base_abort(current.actor()),
            Err(Type60ExplodingRingCustodyTakeBlock::ActorLeaseMismatch {
                expected: source.actor(),
                actual: current.actor(),
            })
        );
        assert_eq!(scheduler.registered_len(), 1);

        let owner = scheduler
            .take_type60_exploding_ring_for_main_base_abort(source.actor())
            .unwrap()
            .expect("the exact allocation transfers custody");
        assert!(scheduler.is_empty());
        assert!(scheduler
            .restore_type60_exploding_ring_after_main_base_abort_noop(owner)
            .unwrap()
            .is_none());
        assert_eq!(scheduler.registered_len(), 1);
        assert!(SpecializedActorTaskScheduler::default()
            .take_type60_exploding_ring_for_main_base_abort(source.actor())
            .unwrap()
            .is_none());
    }
}
