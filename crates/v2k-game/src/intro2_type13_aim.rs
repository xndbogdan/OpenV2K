//! Intro2 Type-13 Aim `FUN_00424650` adapter.
//!
//! This is a Type-13 cohort wrapper, not the Type-47 firing core. Spawn 0
//! authenticates method 10, interval 600,000 us, sound 75, and table speed
//! 2400. ADE0 same-pass Aim and later pursuing visits share this adapter
//! and visit slot 2 once per owner pass. A later live-list drain models
//! `FUN_00411400` before materializing class 38 (not class 87). This module
//! commits cadence, transient append, positional sound 75, and that drain.

use v2k_formats::collision::ProjectileEmitterDescriptor;

use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::actor_task_owner::{ActorTaskSlot, ActorTaskVisit};
use crate::aim_and_fire::{
    aim_and_fire_after_unwind, AimAndFireCallbackResult, AimAndFireFrameRequest,
    AimAndFireFrameResolution, AimAndFireTargetRuntimeState, AIM_AND_FIRE_DYING_STATE_BIT,
};
use crate::aim_and_fire_transaction::{
    AimAndFireEmitterAction, AimAndFireEmitterBlock, AimAndFireEmitterLease,
    AimAndFireEmitterMachine, AimAndFireEmitterPhase, AimAndFireEmitterPoll,
    AimAndFireEmitterProtocolError, AimAndFireEmitterResume, AimAndFireEmitterStartError,
    AimAndFireEmitterTransactionId,
};
use crate::common_mover::component_dispatch::CommonMoverDispatchMode;
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::entity::Entity;
use crate::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use crate::generic_projectile_emitter::{
    GenericEmitterAction, GenericEmitterAppendRequest, GenericEmitterEntitySnapshot,
    GenericEmitterResume, GenericEmitterRuntime, GenericEmitterSourceRefreshPoint,
    GenericEmitterSpeedField, GenericEmitterTransactionId,
};
use crate::ordinary_type47_shot_math::{
    drain_type13_transient_request, evaluate_type47_aim_error, evaluate_type47_forward_half_space,
    resolve_ballistic_target_launch, Type47ShotMathError,
};
use crate::projectile_emitter::projectile_class_row;
use crate::search_attack_live::TYPE13_SEARCH_ATTACK_PROJECTILE_DESCRIPTOR;
use crate::type13_initial_behavior::{
    authenticate_published_type13_class7_pursuing_graph, TYPE13_ENTITY_TYPE,
};
use crate::world_fx::{Class38ParticleRequest, ParticleEnvironment, WorldFx};

const INTRO2_TYPE13_SPAWN_INDEX: usize = 0;
const INTRO2_TYPE13_MODEL_ID: usize = 291;
const TYPE13_PROJECTILE_SOUND_ID: u16 = 75;
const TYPE13_PROJECTILE_METHOD: u32 = 10;
const IMPACT_SUPPRESSION_STATE_BIT: u32 = 0x8000_0000;
const TYPE13_SOURCE_ALLOCATION_MAGIC: u64 = 0x5459_5045_3133_4149;
const TYPE13_TARGET_ALLOCATION_MAGIC: u64 = 0x5459_5045_3133_5447;

pub const TYPE13_EMITTER_RUNTIME: GenericEmitterRuntime = GenericEmitterRuntime {
    joint_bindings: [None; 2],
    projectile_method: TYPE13_PROJECTILE_METHOD,
    emitter_selector: 0,
    sound_id: TYPE13_PROJECTILE_SOUND_ID as u32,
    direct_mode: 0,
    remaining_time_raw: 0,
    manual_step_raw: 0,
    remaining_bursts_raw: 0,
    cadence_raw: 0,
    basis_adjustment_identity: None,
};

fn type13_table_speed_raw() -> i16 {
    projectile_class_row(TYPE13_PROJECTILE_METHOD)
        .expect("method 10 is an audited projectile-class row")
        .speed_raw as i16
}

/// Entity-owned Type-13 Sub-E cadence and later-drained transient FIFO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type13AimRuntime {
    projectile_descriptor: ProjectileEmitterDescriptor,
    emitter_runtime: GenericEmitterRuntime,
    transient_shots: Vec<GenericEmitterAppendRequest>,
    next_transaction_id: u64,
}

impl Type13AimRuntime {
    pub fn from_type13_descriptor(descriptor: ProjectileEmitterDescriptor) -> Option<Self> {
        if descriptor != TYPE13_SEARCH_ATTACK_PROJECTILE_DESCRIPTOR {
            return None;
        }
        Some(Self {
            projectile_descriptor: descriptor,
            emitter_runtime: TYPE13_EMITTER_RUNTIME,
            transient_shots: Vec::new(),
            next_transaction_id: 1,
        })
    }

    pub const fn projectile_descriptor(&self) -> ProjectileEmitterDescriptor {
        self.projectile_descriptor
    }

    pub const fn emitter_runtime(&self) -> GenericEmitterRuntime {
        self.emitter_runtime
    }

    pub fn queued_shot_count(&self) -> usize {
        self.transient_shots.len()
    }

    pub fn transient_shots(&self) -> &[GenericEmitterAppendRequest] {
        &self.transient_shots
    }

    fn take_transaction_pair(
        &mut self,
    ) -> (AimAndFireEmitterTransactionId, GenericEmitterTransactionId) {
        let parent = self.next_transaction_id.max(1);
        let child = parent.wrapping_add(1).max(1);
        self.next_transaction_id = child.wrapping_add(1).max(1);
        (
            AimAndFireEmitterTransactionId::new(parent).expect("parent id is nonzero"),
            GenericEmitterTransactionId::new(child).expect("child id is nonzero"),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type13AimError {
    EntityUnavailable,
    GraphMismatch,
    VisitUnavailable,
    TargetStateUnresolved,
    DescriptorMismatch,
    RuntimeUnavailable,
    RuntimeContractMismatch,
    BodyBasisUnresolved,
    TaskLeaseMismatch,
    MachineStart(AimAndFireEmitterStartError),
    MachineProtocol(AimAndFireEmitterProtocolError),
    MachineBlocked(AimAndFireEmitterBlock),
    ShotMath(Type47ShotMathError),
    ActionContractMismatch,
    CallbackWrapperDidNotSurvive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type13AimTickOutcome {
    pub resolution: AimAndFireFrameResolution,
    pub queued_shots_added: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type13ShotDrainError {
    EntityUnavailable,
    RuntimeUnavailable,
    RuntimeContractMismatch,
    MalformedTransientRequest { index: usize },
    ShotMath(Type47ShotMathError),
    ImpactSuppressionStateUnresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type13ShotDrainOutcome {
    pub consumed_requests: usize,
    pub materialized_particle_classes: Vec<u8>,
}

pub fn ensure_intro2_type13_aim_runtime(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<(), Type13AimError> {
    if entity.intro2_type13_aim_runtime.is_some() {
        return Ok(());
    }
    let Some(metadata) = metadata else {
        return Err(Type13AimError::DescriptorMismatch);
    };
    let RetailRuntimeValue::Known(Some(descriptor)) = metadata.projectile_emitter_descriptor else {
        return Err(Type13AimError::DescriptorMismatch);
    };
    entity.intro2_type13_aim_runtime = Some(
        Type13AimRuntime::from_type13_descriptor(descriptor)
            .ok_or(Type13AimError::DescriptorMismatch)?,
    );
    Ok(())
}

pub fn tick_intro2_type13_aim(
    dispatch_mode: CommonMoverDispatchMode,
    manager: &mut crate::entity::EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    elapsed_micros: u32,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<Type13AimTickOutcome, Type13AimError> {
    // E870 passes mode 1: the generic wrapper still ages Aim, but 2300
    // returns before reading its target or any projectile/emitter state.
    if dispatch_mode == CommonMoverDispatchMode::Restricted {
        let entity = manager
            .intro2_type13_entity_mut(entity_id)
            .ok_or(Type13AimError::EntityUnavailable)?;
        authenticate_intro2_type13_aim_owner(entity, false)?;
        let task_id = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary)
            .ok_or(Type13AimError::VisitUnavailable)?;
        let visit = ActorTaskVisit {
            slot: ActorTaskSlot::Tertiary,
            task_id,
        };
        let prefix = entity
            .actor_tasks
            .begin_exact_visit_with(visit, |task| {
                let ActorTaskRuntime::AimAndFire(task) = task else {
                    unreachable!("authenticated Type-13 Tertiary is AimAndFire");
                };
                task.before_callback(elapsed_micros)
            })
            .ok_or(Type13AimError::VisitUnavailable)?;
        if !entity.actor_tasks.finish_exact_visit(visit) {
            return Err(Type13AimError::CallbackWrapperDidNotSurvive);
        }
        return Ok(Type13AimTickOutcome {
            resolution: aim_and_fire_after_unwind(prefix, AimAndFireCallbackResult::Zero),
            queued_shots_added: 0,
        });
    }
    let (
        task_visit,
        target_entity_id,
        owner_position_raw,
        descriptor,
        emitter_runtime,
        queued_before,
        parent_id,
        child_id,
        prefix,
        private_state,
        source_allocation_identity,
        target_allocation_identity,
    ) = {
        let Some(entity) = manager.intro2_type13_entity_mut(entity_id) else {
            return Err(Type13AimError::EntityUnavailable);
        };
        authenticate_intro2_type13_aim_owner(entity, false)?;
        ensure_intro2_type13_aim_runtime(entity, metadata)?;
        let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary) else {
            return Err(Type13AimError::VisitUnavailable);
        };
        let task_visit = ActorTaskVisit {
            slot: ActorTaskSlot::Tertiary,
            task_id,
        };
        let Some(ActorTaskRuntime::AimAndFire(state)) =
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        else {
            return Err(Type13AimError::GraphMismatch);
        };
        let target_entity_id = state.private_state().target_entity_id();
        let owner_position_raw = entity.position_raw();
        let runtime = entity
            .intro2_type13_aim_runtime
            .as_mut()
            .ok_or(Type13AimError::RuntimeUnavailable)?;
        if !is_exact_runtime_shape(runtime) {
            return Err(Type13AimError::RuntimeContractMismatch);
        }
        let (parent_id, child_id) = runtime.take_transaction_pair();
        let descriptor = runtime.projectile_descriptor;
        let emitter_runtime = runtime.emitter_runtime;
        let queued_before = runtime.transient_shots.len();
        let source_allocation_identity = transaction_scoped_allocation_identity(
            parent_id.get(),
            entity_id,
            TYPE13_SOURCE_ALLOCATION_MAGIC,
        );
        let target_allocation_identity = transaction_scoped_allocation_identity(
            child_id.get(),
            target_entity_id,
            TYPE13_TARGET_ALLOCATION_MAGIC,
        );
        let (prefix, private_state) = entity
            .actor_tasks
            .begin_exact_visit_with(task_visit, |task| {
                let ActorTaskRuntime::AimAndFire(task) = task else {
                    unreachable!("authenticated Type-13 Tertiary is AimAndFire");
                };
                let private_state = task.private_state();
                (task.before_callback(elapsed_micros), private_state)
            })
            .ok_or(Type13AimError::VisitUnavailable)?;
        (
            task_visit,
            target_entity_id,
            owner_position_raw,
            descriptor,
            emitter_runtime,
            queued_before,
            parent_id,
            child_id,
            prefix,
            private_state,
            source_allocation_identity,
            target_allocation_identity,
        )
    };

    let callback = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let target_state = type13_aim_target_runtime_state(manager, target_entity_id)?;
        let lease = AimAndFireEmitterLease {
            owner_entity_id: entity_id,
            task_visit,
        };
        let mut machine = AimAndFireEmitterMachine::start(
            parent_id,
            child_id,
            lease,
            prefix,
            private_state,
            AimAndFireFrameRequest {
                owner_entity_id: entity_id,
                elapsed_micros,
                scheduler_mode: 0,
                target_state,
                owner_sound_position_raw: Some(owner_position_raw),
                sub_e_descriptor: Some(descriptor),
                emitter_runtime,
            },
        )
        .map_err(Type13AimError::MachineStart)?;
        drive_type13_aim_machine(
            &mut machine,
            manager,
            world_fx,
            entity_id,
            task_visit,
            child_id,
            source_allocation_identity,
            target_allocation_identity,
        )
    }));

    let survived = manager
        .intro2_type13_entity_mut(entity_id)
        .is_some_and(|entity| entity.actor_tasks.finish_exact_visit(task_visit));
    let completion = match callback {
        Ok(result) => result?,
        Err(payload) => std::panic::resume_unwind(payload),
    };
    if !survived {
        return Err(Type13AimError::CallbackWrapperDidNotSurvive);
    }
    let queued_after = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .and_then(|entity| entity.intro2_type13_aim_runtime.as_ref())
        .map(|runtime| runtime.transient_shots.len())
        .unwrap_or(queued_before);
    Ok(Type13AimTickOutcome {
        resolution: aim_and_fire_after_unwind(
            completion.committed_prefix,
            completion.callback_result,
        ),
        queued_shots_added: queued_after.saturating_sub(queued_before),
    })
}

/// Drain the entity-owned method-10 FIFO and materialize class 38.
///
/// `FUN_00411400` consumes every queued command even when `FUN_004410B0` /
/// `FUN_00440A60` rejects a birth. Style, task wrappers, and later C690
/// graphs are irrelevant once the transient exists. This path never reuses
/// method 30, class 87, sound 70, or the Type-47 Aim owner.
pub fn drain_intro2_type13_shots(
    manager: &mut crate::entity::EntityManager,
    world_fx: &mut WorldFx,
    source_entity_id: u32,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Result<Type13ShotDrainOutcome, Type13ShotDrainError> {
    let (solutions, suppresses_impact_damage) = {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == source_entity_id)
            .ok_or(Type13ShotDrainError::EntityUnavailable)?;
        if entity.entity_type != TYPE13_ENTITY_TYPE
            || entity.authored_spawn_index != Some(INTRO2_TYPE13_SPAWN_INDEX)
            || entity.model_index != Some(INTRO2_TYPE13_MODEL_ID)
        {
            return Err(Type13ShotDrainError::RuntimeContractMismatch);
        }
        let runtime = entity
            .intro2_type13_aim_runtime
            .as_ref()
            .ok_or(Type13ShotDrainError::RuntimeUnavailable)?;
        if !is_exact_runtime_shape(runtime) {
            return Err(Type13ShotDrainError::RuntimeContractMismatch);
        }
        if runtime.transient_shots.is_empty() {
            return Ok(Type13ShotDrainOutcome {
                consumed_requests: 0,
                materialized_particle_classes: Vec::new(),
            });
        }
        let suppression = match entity
            .collision
            .state_flags_at_0x08
            .masked(IMPACT_SUPPRESSION_STATE_BIT)
        {
            RetailRuntimeValue::Known(value) => value != 0,
            RetailRuntimeValue::Unresolved => {
                return Err(Type13ShotDrainError::ImpactSuppressionStateUnresolved);
            }
        };
        let mut solutions = Vec::with_capacity(runtime.transient_shots.len());
        for (index, request) in runtime.transient_shots.iter().copied().enumerate() {
            if !is_exact_transient_request(source_entity_id, request) {
                return Err(Type13ShotDrainError::MalformedTransientRequest { index });
            }
            solutions.push(
                drain_type13_transient_request(
                    request,
                    entity.position_raw(),
                    entity.velocity_raw(),
                )
                .map_err(Type13ShotDrainError::ShotMath)?,
            );
        }
        (solutions, suppression)
    };

    let queued = {
        let entity = manager
            .intro2_type13_entity_mut(source_entity_id)
            .ok_or(Type13ShotDrainError::EntityUnavailable)?;
        let runtime = entity
            .intro2_type13_aim_runtime
            .as_mut()
            .ok_or(Type13ShotDrainError::RuntimeUnavailable)?;
        std::mem::take(&mut runtime.transient_shots)
    };
    debug_assert_eq!(queued.len(), solutions.len());

    let mut materialized_particle_classes = Vec::with_capacity(solutions.len());
    for solution in solutions {
        if let Some(birth) = world_fx.materialize_class_38_request(
            Class38ParticleRequest {
                position_raw: solution.position_raw,
                velocity_raw: solution.velocity_raw,
                owner_entity_id: source_entity_id,
                owner_entity_type: TYPE13_ENTITY_TYPE as u8,
                suppresses_impact_damage,
            },
            environment,
            retail_tick,
        ) {
            materialized_particle_classes.push(birth.particle_class);
        }
    }
    Ok(Type13ShotDrainOutcome {
        consumed_requests: queued.len(),
        materialized_particle_classes,
    })
}

/// Presentation-phase `FUN_00411400` drain of every retained Type-13 FIFO.
///
/// Aim appends during the specialized scheduler visit. Retail materializes
/// those transients later in presentation, after particle traversal, in live-list order.
pub fn drain_live_intro2_type13_shot_queues(
    manager: &mut crate::entity::EntityManager,
    world_fx: &mut WorldFx,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Vec<(u32, Result<Type13ShotDrainOutcome, Type13ShotDrainError>)> {
    let source_ids = manager
        .iter_all()
        .filter(|entity| {
            entity
                .intro2_type13_aim_runtime
                .as_ref()
                .is_some_and(|runtime| runtime.queued_shot_count() > 0)
        })
        .map(|entity| entity.id)
        .collect::<Vec<_>>();
    source_ids
        .into_iter()
        .map(|source_entity_id| {
            (
                source_entity_id,
                drain_intro2_type13_shots(
                    manager,
                    world_fx,
                    source_entity_id,
                    environment,
                    retail_tick,
                ),
            )
        })
        .collect()
}

fn drive_type13_aim_machine(
    machine: &mut AimAndFireEmitterMachine,
    manager: &mut crate::entity::EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    task_visit: ActorTaskVisit,
    child_transaction_id: GenericEmitterTransactionId,
    source_allocation_identity: u64,
    target_allocation_identity: u64,
) -> Result<crate::aim_and_fire_transaction::AimAndFireEmitterCompletion, Type13AimError> {
    loop {
        match machine.poll() {
            AimAndFireEmitterPoll::Action(issued) => {
                let action_phase = issued.action.phase();
                let expected_lease = AimAndFireEmitterLease {
                    owner_entity_id: entity_id,
                    task_visit,
                };
                if issued.action.lease() != expected_lease {
                    return Err(Type13AimError::TaskLeaseMismatch);
                }
                revalidate_active_owner(manager, entity_id, task_visit)?;
                let response = match issued.action {
                    AimAndFireEmitterAction::GenericEmitter {
                        phase,
                        child_transaction_id: actual_child,
                        action,
                        ..
                    } if actual_child == child_transaction_id
                        && phase == AimAndFireEmitterPhase::GenericEmitter(action.phase()) =>
                    {
                        let response = handle_type13_generic_action(
                            manager,
                            world_fx,
                            entity_id,
                            task_visit,
                            action,
                            source_allocation_identity,
                            target_allocation_identity,
                        )?;
                        AimAndFireEmitterResume::GenericEmitter {
                            phase,
                            child_transaction_id,
                            response,
                        }
                    }
                    _ => return Err(Type13AimError::ActionContractMismatch),
                };
                let _ = action_phase;
                machine
                    .resume(issued.receipt, response)
                    .map_err(|failure| Type13AimError::MachineProtocol(failure.error))?;
            }
            AimAndFireEmitterPoll::Awaiting(_) => {
                return Err(Type13AimError::ActionContractMismatch);
            }
            AimAndFireEmitterPoll::Blocked(block) => {
                return Err(Type13AimError::MachineBlocked(block));
            }
            AimAndFireEmitterPoll::Complete(completion) => return Ok(completion),
        }
    }
}

fn handle_type13_generic_action(
    manager: &mut crate::entity::EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    task_visit: ActorTaskVisit,
    action: GenericEmitterAction,
    source_allocation_identity: u64,
    target_allocation_identity: u64,
) -> Result<GenericEmitterResume, Type13AimError> {
    let phase = action.phase();
    let target_entity_id = {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .ok_or(Type13AimError::EntityUnavailable)?;
        let Some(ActorTaskRuntime::AimAndFire(state)) =
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        else {
            return Err(Type13AimError::GraphMismatch);
        };
        state.private_state().target_entity_id()
    };
    match action {
        GenericEmitterAction::LookupEntity { handle, .. } => {
            let entity = if handle == entity_id {
                manager
                    .iter_all()
                    .find(|entity| entity.id == handle && entity.active)
                    .map(|entity| entity_snapshot(entity, source_allocation_identity))
            } else if handle == target_entity_id {
                // 42476A reads the selected allocation's raw XYZ; 424892
                // resolves it again for signed velocity lead. Neither lookup
                // consumes target type, model, or component metadata.
                manager
                    .iter_all()
                    .find(|entity| entity.id == handle && entity.active)
                    .map(|entity| entity_snapshot(entity, target_allocation_identity))
            } else {
                return Err(Type13AimError::ActionContractMismatch);
            };
            Ok(GenericEmitterResume::EntityLookupResolved { phase, entity })
        }
        GenericEmitterAction::CommitInitialCadence { write, .. } => {
            let expected_after = if write.before_raw < 1 {
                0i32.wrapping_sub(write.elapsed_us as i32)
            } else {
                write.before_raw.wrapping_sub(write.elapsed_us as i32)
            };
            let entity = manager
                .intro2_type13_entity_mut(entity_id)
                .ok_or(Type13AimError::EntityUnavailable)?;
            authenticate_intro2_type13_aim_owner(entity, true)?;
            let runtime = &mut entity
                .intro2_type13_aim_runtime
                .as_mut()
                .ok_or(Type13AimError::RuntimeUnavailable)?
                .emitter_runtime;
            if runtime.cadence_raw != write.before_raw || write.after_raw != expected_after {
                return Err(Type13AimError::RuntimeContractMismatch);
            }
            runtime.cadence_raw = write.after_raw;
            Ok(GenericEmitterResume::Acknowledged { phase })
        }
        GenericEmitterAction::DrawSharedRandom { .. } => Ok(GenericEmitterResume::RandomDrawn {
            phase,
            value: u32::from(world_fx.next_shared_retail_random_u16()),
        }),
        GenericEmitterAction::RefreshCachedSource {
            source_handle,
            allocation_identity,
            point,
            ..
        } => {
            if source_handle != entity_id
                || allocation_identity != source_allocation_identity
                || !matches!(
                    point,
                    GenericEmitterSourceRefreshPoint::TargetAxisGate
                        | GenericEmitterSourceRefreshPoint::TargetShot
                )
            {
                return Err(Type13AimError::ActionContractMismatch);
            }
            let entity = manager
                .iter_all()
                .find(|entity| entity.id == entity_id && entity.active)
                .ok_or(Type13AimError::EntityUnavailable)?;
            authenticate_intro2_type13_aim_owner(entity, true)?;
            Ok(GenericEmitterResume::CachedSourceRefreshed {
                phase,
                entity: Some(entity_snapshot(entity, source_allocation_identity)),
            })
        }
        GenericEmitterAction::EvaluateAim {
            source_handle,
            target_position_raw,
            ..
        } => {
            let source = source_for_math(manager, entity_id, source_handle, task_visit)?;
            let basis = retained_aim_basis(source)?;
            Ok(GenericEmitterResume::AimEvaluated {
                phase,
                error_raw: i32::from(evaluate_type47_aim_error(
                    source.position_raw(),
                    basis.lateral,
                    basis.forward,
                    target_position_raw,
                )),
            })
        }
        GenericEmitterAction::EvaluateForwardHalfSpace {
            source_handle,
            target_position_raw,
            ..
        } => {
            let source = source_for_math(manager, entity_id, source_handle, task_visit)?;
            let basis = retained_aim_basis(source)?;
            Ok(GenericEmitterResume::ForwardHalfSpaceEvaluated {
                phase,
                in_forward_half_space: evaluate_type47_forward_half_space(
                    source.position_raw(),
                    basis.forward,
                    target_position_raw,
                ),
            })
        }
        GenericEmitterAction::ResolveTargetLaunch { request, .. } => {
            if request.cached_source.handle != entity_id
                || request.cached_source.allocation_identity != source_allocation_identity
                || request.projectile_method != TYPE13_PROJECTILE_METHOD
                || request.speed_override_raw != 0
                || request.current_target.is_some_and(|target| {
                    target.handle != target_entity_id
                        || target.allocation_identity != target_allocation_identity
                })
            {
                return Err(Type13AimError::ActionContractMismatch);
            }
            let solution = resolve_ballistic_target_launch(request, type13_table_speed_raw())
                .map_err(Type13AimError::ShotMath)?;
            Ok(GenericEmitterResume::TargetLaunchResolved { phase, solution })
        }
        GenericEmitterAction::AppendTransient { request, .. } => {
            if !is_exact_transient_request(entity_id, request) {
                return Err(Type13AimError::ActionContractMismatch);
            }
            let entity = manager
                .intro2_type13_entity_mut(entity_id)
                .ok_or(Type13AimError::EntityUnavailable)?;
            authenticate_intro2_type13_aim_owner(entity, true)?;
            entity
                .intro2_type13_aim_runtime
                .as_mut()
                .ok_or(Type13AimError::RuntimeUnavailable)?
                .transient_shots
                .push(request);
            Ok(GenericEmitterResume::AppendReturned {
                phase,
                result_raw: 0,
            })
        }
        GenericEmitterAction::CommitTargetCadence { write, .. } => {
            if write.interval_us != TYPE13_SEARCH_ATTACK_PROJECTILE_DESCRIPTOR.random_interval_us
                || write.cadence_after_raw
                    != write
                        .cadence_before_raw
                        .wrapping_add(write.interval_us as i32)
                || write.time_offset_after_raw
                    != write
                        .time_offset_before_raw
                        .wrapping_add(write.interval_us as i32)
            {
                return Err(Type13AimError::RuntimeContractMismatch);
            }
            let entity = manager
                .intro2_type13_entity_mut(entity_id)
                .ok_or(Type13AimError::EntityUnavailable)?;
            authenticate_intro2_type13_aim_owner(entity, true)?;
            let runtime = &mut entity
                .intro2_type13_aim_runtime
                .as_mut()
                .ok_or(Type13AimError::RuntimeUnavailable)?
                .emitter_runtime;
            if runtime.cadence_raw != write.cadence_before_raw {
                return Err(Type13AimError::RuntimeContractMismatch);
            }
            runtime.cadence_raw = write.cadence_after_raw;
            Ok(GenericEmitterResume::Acknowledged { phase })
        }
        GenericEmitterAction::PlayPositionalSound {
            sound_id,
            source_position_raw,
            frequency_multiplier_16_16,
            volume_multiplier_16_16,
            ..
        } => {
            let source = manager
                .iter_all()
                .find(|entity| entity.id == entity_id && entity.active)
                .ok_or(Type13AimError::EntityUnavailable)?;
            authenticate_intro2_type13_aim_owner(source, true)?;
            let expected_position = source.position_raw();
            if sound_id != u32::from(TYPE13_PROJECTILE_SOUND_ID)
                || source_position_raw != Some(expected_position)
                || frequency_multiplier_16_16 != 0x0001_0000
                || volume_multiplier_16_16 != 0x0001_0000
            {
                return Err(Type13AimError::ActionContractMismatch);
            }
            world_fx
                .queue_fixed_positional_sound_raw(TYPE13_PROJECTILE_SOUND_ID, expected_position);
            Ok(GenericEmitterResume::Acknowledged { phase })
        }
        GenericEmitterAction::EmitMethod31Rejection { .. }
        | GenericEmitterAction::CommitMethod31Suppression { .. }
        | GenericEmitterAction::ReadManualAngleBinding { .. }
        | GenericEmitterAction::ResolveManualBasis { .. }
        | GenericEmitterAction::CommitGunJoint { .. }
        | GenericEmitterAction::ResolveManualLaunch { .. }
        | GenericEmitterAction::CommitAlternateSelector { .. }
        | GenericEmitterAction::CommitManualCounters { .. } => {
            Err(Type13AimError::ActionContractMismatch)
        }
    }
}

fn authenticate_intro2_type13_aim_owner(
    entity: &Entity,
    expected_in_callback: bool,
) -> Result<(), Type13AimError> {
    if !entity.active
        || entity.entity_type != TYPE13_ENTITY_TYPE
        || entity.authored_spawn_index != Some(INTRO2_TYPE13_SPAWN_INDEX)
        || entity.model_index != Some(INTRO2_TYPE13_MODEL_ID)
    {
        return Err(Type13AimError::GraphMismatch);
    }
    authenticate_published_type13_class7_pursuing_graph(entity)
        .map_err(|_| Type13AimError::GraphMismatch)?;
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary) else {
        return Err(Type13AimError::VisitUnavailable);
    };
    let flags = entity
        .actor_tasks
        .wrapper_flags(task_id)
        .ok_or(Type13AimError::VisitUnavailable)?;
    if !flags.alive || flags.in_callback != expected_in_callback {
        return Err(Type13AimError::VisitUnavailable);
    }
    Ok(())
}

fn revalidate_active_owner(
    manager: &crate::entity::EntityManager,
    entity_id: u32,
    _task_visit: ActorTaskVisit,
) -> Result<(), Type13AimError> {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .ok_or(Type13AimError::EntityUnavailable)?;
    authenticate_intro2_type13_aim_owner(entity, true)
}

fn type13_aim_target_runtime_state(
    manager: &crate::entity::EntityManager,
    target_entity_id: u32,
) -> Result<AimAndFireTargetRuntimeState, Type13AimError> {
    let Some(target) = manager
        .iter_all()
        .find(|entity| entity.id == target_entity_id && entity.active)
    else {
        return Ok(AimAndFireTargetRuntimeState::Missing);
    };
    // 402355..40236B validates only the tracked handle and state +08:
    // nonzero and not dying. The class-7 selector already chose the target;
    // Aim has no Type-9-only or other target-family admission rule.
    let state = target.collision.state_flags_at_0x08;
    let dying = match state.masked(AIM_AND_FIRE_DYING_STATE_BIT) {
        RetailRuntimeValue::Known(value) => value,
        RetailRuntimeValue::Unresolved => return Err(Type13AimError::TargetStateUnresolved),
    };
    if dying != 0 {
        return Ok(AimAndFireTargetRuntimeState::Present {
            state_flags: AIM_AND_FIRE_DYING_STATE_BIT,
        });
    }
    let known_value = state.known_value_bits();
    if known_value != 0 || state.known_mask() == u32::MAX {
        return Ok(AimAndFireTargetRuntimeState::Present {
            state_flags: known_value,
        });
    }
    Err(Type13AimError::TargetStateUnresolved)
}

fn source_for_math<'a>(
    manager: &'a crate::entity::EntityManager,
    entity_id: u32,
    source_handle: u32,
    _task_visit: ActorTaskVisit,
) -> Result<&'a Entity, Type13AimError> {
    if source_handle != entity_id {
        return Err(Type13AimError::ActionContractMismatch);
    }
    let source = manager
        .iter_all()
        .find(|entity| entity.id == source_handle && entity.active)
        .ok_or(Type13AimError::EntityUnavailable)?;
    authenticate_intro2_type13_aim_owner(source, true)?;
    Ok(source)
}

fn entity_snapshot(entity: &Entity, allocation_identity: u64) -> GenericEmitterEntitySnapshot {
    GenericEmitterEntitySnapshot {
        handle: entity.id,
        allocation_identity,
        state_flags_at_0x08: entity.collision.state_flags_at_0x08.known_value_bits(),
        position_raw: entity.position_raw(),
        velocity_raw: entity.velocity_raw(),
        orientation_raw: entity.rotation_heading_pitch_roll_raw(),
    }
}

fn retained_aim_basis(entity: &Entity) -> Result<Type9BodyBasis, Type13AimError> {
    // FUN_0041E4D0 reads retained lateral +0x0C..+0x14 and forward
    // +0x24..+0x2C; FUN_0041E930 reads that same forward column. Chase can
    // already have changed the angle words, but DCA0/E870 calls FUN_00413F70
    // only after the whole A800 traversal, including this same-pass Aim.
    match entity.physical_body_basis_q31() {
        RetailRuntimeValue::Known(basis) => Ok(basis),
        RetailRuntimeValue::Unresolved => Err(Type13AimError::BodyBasisUnresolved),
    }
}

fn is_exact_runtime_shape(runtime: &Type13AimRuntime) -> bool {
    runtime.projectile_descriptor == TYPE13_SEARCH_ATTACK_PROJECTILE_DESCRIPTOR
        && runtime.emitter_runtime
            == (GenericEmitterRuntime {
                cadence_raw: runtime.emitter_runtime.cadence_raw,
                ..TYPE13_EMITTER_RUNTIME
            })
}

fn is_exact_transient_request(source_entity_id: u32, request: GenericEmitterAppendRequest) -> bool {
    request.source_handle == source_entity_id
        && request.owner_handle == source_entity_id
        && request.projectile_method == TYPE13_PROJECTILE_METHOD
        && request.emitter_selector == 0
        && request.speed_field == GenericEmitterSpeedField::Explicit(type13_table_speed_raw())
        && !request.auxiliary
}

fn transaction_scoped_allocation_identity(seed: u64, handle: u32, discriminator: u64) -> u64 {
    let mut identity = seed.rotate_left(17) ^ discriminator;
    if identity == 0 || identity == u64::from(handle) {
        identity = identity.wrapping_add(discriminator | 1);
    }
    if identity == 0 || identity == u64::from(handle) {
        identity = u64::from(handle) ^ u64::MAX;
    }
    identity
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityKind;

    #[test]
    fn aim_basis_requires_a_retained_matrix_even_when_angles_are_available() {
        let mut entity = Entity::unresolved_port_entity(1, EntityKind::Enemy, TYPE13_ENTITY_TYPE);
        entity.set_rotation_heading_pitch_roll_raw([0x4000, 0x1000, -0x2000]);
        assert_eq!(
            retained_aim_basis(&entity),
            Err(Type13AimError::BodyBasisUnresolved),
        );
    }
}
