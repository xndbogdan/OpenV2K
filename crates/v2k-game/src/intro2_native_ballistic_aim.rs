//! Native Intro2 ballistic Aim and presentation-time projectile FIFO.
//!
//! The shared 2300/24650 transaction owns the action order. Unlike the flyer
//! adapter, cadence belongs to the allocation's native Sub-E; this host keeps
//! only immutable descriptor custody, queued 1EFD0 commands and transaction IDs.

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
    drain_intro2_flyer_transient_request, drain_type13_transient_request,
    evaluate_type47_aim_error, evaluate_type47_forward_half_space, resolve_ballistic_target_launch,
    Type47ShotMathError,
};
use crate::projectile_emitter::projectile_class_row;
use crate::world_fx::{
    Class38ParticleRequest, DescriptorParticleRequest, ParticleEnvironment, ParticleOwnerAtBirth,
    WorldFx,
};

const IMPACT_SUPPRESSION_STATE_BIT: u32 = 0x8000_0000;

/// Source-equivalent target emitter profiles; the FIFO records its allocation's
/// profile independently of descriptor equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeBallisticProfileId {
    Type10,
    /// Ordinary Type5 rows on the Type10 owner: method 10 at table speed.
    Type5,
    /// Ordinary Type80/126 carriers on the Type10 owner: Type10's override.
    Type80,
    Type126,
    /// Ordinary emitter-only Type43 shooters: method 10 at Type10's override.
    Type43,
    /// Ordinary Type38 ground shooters and the silent Type129 carriers.
    Type38,
    Type129,
    Type16,
    /// Ordinary Type128 carriers on the Type16 owner: a silent emitter.
    Type128,
    Type58,
    Type94,
    Type57,
    Type122,
    Type30,
    Type56,
    Type40,
    Type18,
}

impl NativeBallisticProfileId {
    const fn method(self) -> u32 {
        match self {
            Self::Type10
            | Self::Type5
            | Self::Type80
            | Self::Type126
            | Self::Type43
            | Self::Type38
            | Self::Type129 => 10,
            Self::Type16
            | Self::Type128
            | Self::Type58
            | Self::Type94
            | Self::Type30
            | Self::Type18 => 20,
            Self::Type57 | Self::Type122 => 24,
            Self::Type56 => 30,
            Self::Type40 => 1,
        }
    }
    const fn speed_override(self) -> i16 {
        match self {
            Self::Type10
            | Self::Type80
            | Self::Type126
            | Self::Type43
            | Self::Type38
            | Self::Type129 => 1500,
            Self::Type5
            | Self::Type16
            | Self::Type128
            | Self::Type58
            | Self::Type94
            | Self::Type57
            | Self::Type122
            | Self::Type30
            | Self::Type56
            | Self::Type40
            | Self::Type18 => 0,
        }
    }
    fn table_speed(self) -> i16 {
        projectile_class_row(self.method())
            .expect("native ballistic method has a retail table row")
            .speed_raw as i16
    }
    fn launch_speed(self) -> i16 {
        match self.speed_override() {
            0 => self.table_speed(),
            speed => speed,
        }
    }
    fn particle_class(self) -> u8 {
        projectile_class_row(self.method())
            .expect("native ballistic method has a retail table row")
            .particle_class
    }
    const fn sound(self) -> u16 {
        match self {
            Self::Type40 | Self::Type128 => 0,
            Self::Type10 | Self::Type5 | Self::Type80 => 81,
            Self::Type126 => 82,
            Self::Type43 => 68,
            Self::Type38 => 82,
            Self::Type129 => 0,
            Self::Type16 => 81,
            Self::Type58 => 70,
            Self::Type94 => 69,
            Self::Type57 => 93,
            Self::Type122 | Self::Type56 => 70,
            Self::Type30 => 75,
            Self::Type18 => 69,
        }
    }
    const fn source_magic(self) -> u64 {
        match self {
            Self::Type10 => 0x5459_5031_3053_5243,
            Self::Type5 => 0x5459_5030_3553_5243,
            Self::Type80 => 0x5459_5038_3053_5243,
            Self::Type126 => 0x5459_3132_3653_5243,
            Self::Type43 => 0x5459_5034_3353_5243,
            Self::Type38 => 0x5459_5033_3853_5243,
            Self::Type129 => 0x5459_3132_3953_5243,
            Self::Type16 => 0x5459_5031_3653_5243,
            Self::Type128 => 0x5459_3132_3853_5243,
            Self::Type58 => 0x5459_5035_3853_5243,
            Self::Type94 => 0x5459_5039_3453_5243,
            Self::Type57 => 0x5459_5035_3753_5243,
            Self::Type122 => 0x5459_3132_3253_5243,
            Self::Type56 => 0x5459_5035_3653_5243,
            Self::Type40 => 0x5459_5034_3053_5243,
            Self::Type30 => 0x5459_5033_3053_5243,
            Self::Type18 => 0x5459_5031_3853_5243,
        }
    }
    const fn target_magic(self) -> u64 {
        match self {
            Self::Type10 => 0x5459_5031_3054_4754,
            Self::Type5 => 0x5459_5030_3554_4754,
            Self::Type80 => 0x5459_5038_3054_4754,
            Self::Type126 => 0x5459_3132_3654_4754,
            Self::Type43 => 0x5459_5034_3354_4754,
            Self::Type38 => 0x5459_5033_3854_4754,
            Self::Type129 => 0x5459_3132_3954_4754,
            Self::Type16 => 0x5459_5031_3654_4754,
            Self::Type128 => 0x5459_3132_3854_4754,
            Self::Type58 => 0x5459_5035_3854_4754,
            Self::Type94 => 0x5459_5039_3454_4754,
            Self::Type57 => 0x5459_5035_3754_4754,
            Self::Type122 => 0x5459_3132_3254_4754,
            Self::Type56 => 0x5459_5035_3654_4754,
            Self::Type40 => 0x5459_5034_3054_4754,
            Self::Type30 => 0x5459_5033_3054_4754,
            Self::Type18 => 0x5459_5031_3854_4754,
        }
    }
    fn accepts(self, descriptor: ProjectileEmitterDescriptor) -> bool {
        let (interval, spread, raw_word, threshold, axis) = match self {
            Self::Type10 => (300_000, 128, 44, 12_000, 3840),
            Self::Type5 => (300_000, 512, 48, 40_000, 5120),
            Self::Type80 | Self::Type126 => (300_000, 128, 44, 24_000, 5120),
            Self::Type43 => (300_000, 1024, 0, 65_535, 7680),
            Self::Type38 | Self::Type129 => (600_000, 256, 122, 4000, 5120),
            Self::Type16 | Self::Type128 => (300_000, 100, 158, 16_000, 2560),
            Self::Type58 => (400_000, 256, 150, 16_000, 2560),
            Self::Type94 => (700_000, 256, 102, 12_000, 2304),
            Self::Type57 => (600_000, 256, 42, 16_000, 7680),
            Self::Type122 => (400_000, 100, 150, 12_000, 2560),
            Self::Type56 => (300_000, 64, 96, 32_000, 2560),
            Self::Type40 => (600_000, 256, 0, 16_000, 5120),
            Self::Type30 => (500_000, 512, 50, 25_000, 3840),
            Self::Type18 => (500_000, 100, 156, 30_000, 3072),
        };
        descriptor.projectile_method == self.method()
            && descriptor.sound_id == self.sound()
            && descriptor.random_interval_us == interval
            && descriptor.spread_raw == spread
            && descriptor.aim_threshold_raw == threshold
            && descriptor.target_axis_tolerance_raw == axis
            && descriptor.raw_word_at_0x12 == raw_word
            && descriptor.stochastic_gate_mode == 0
            && descriptor.variable_bindings == [0; 4]
            && descriptor.alternate_emitter_raw == 0
            && descriptor.auxiliary_command == 0
            && descriptor.speed_override_raw == self.speed_override()
    }
}

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// Implemented only by the actor-owned facades, which can access their private
/// native component receipt. No live cadence is reconstructed by this host.
pub(crate) trait NativeBallisticProfile: sealed::Sealed {
    const ID: NativeBallisticProfileId;
    fn allocation_authenticates(entity: &Entity) -> bool;
    fn graph_authenticates(entity: &Entity) -> bool;
    fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool;
    fn emitter(entity: &Entity) -> Option<&GenericEmitterRuntime>;
    fn emitter_mut(entity: &mut Entity) -> Option<&mut GenericEmitterRuntime>;
    fn queue(entity: &Entity) -> Option<&NativeBallisticAimRuntime>;
    fn queue_slot_mut(entity: &mut Entity) -> &mut Option<NativeBallisticAimRuntime>;
}

/// Allocation-owned transient FIFO. Native Sub-E remains the sole cadence owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeBallisticAimRuntime {
    profile: NativeBallisticProfileId,
    projectile_descriptor: ProjectileEmitterDescriptor,
    transient_shots: Vec<GenericEmitterAppendRequest>,
    next_transaction_id: u64,
}

impl NativeBallisticAimRuntime {
    fn from_descriptor(
        profile: NativeBallisticProfileId,
        descriptor: ProjectileEmitterDescriptor,
    ) -> Option<Self> {
        if !profile.accepts(descriptor) {
            return None;
        }
        Some(Self {
            profile,
            projectile_descriptor: descriptor,
            transient_shots: Vec::new(),
            next_transaction_id: 1,
        })
    }

    pub const fn projectile_descriptor(&self) -> ProjectileEmitterDescriptor {
        self.projectile_descriptor
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
pub enum NativeBallisticAimError {
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
pub struct NativeBallisticAimTickOutcome {
    pub resolution: AimAndFireFrameResolution,
    pub queued_shots_added: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBallisticShotDrainError {
    EntityUnavailable,
    RuntimeUnavailable,
    RuntimeContractMismatch,
    MalformedTransientRequest { index: usize },
    ShotMath(Type47ShotMathError),
    ImpactSuppressionStateUnresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeBallisticShotDrainOutcome {
    pub consumed_requests: usize,
    pub materialized_particle_classes: Vec<u8>,
}

pub(crate) fn ensure_native_aim_runtime<P: NativeBallisticProfile>(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<(), NativeBallisticAimError> {
    let Some(metadata) = metadata else {
        return Err(NativeBallisticAimError::DescriptorMismatch);
    };
    let RetailRuntimeValue::Known(Some(descriptor)) = metadata.projectile_emitter_descriptor else {
        return Err(NativeBallisticAimError::DescriptorMismatch);
    };
    if !P::allocation_authenticates(entity) {
        return Err(NativeBallisticAimError::GraphMismatch);
    }
    if !P::metadata_authenticates(metadata) {
        return Err(NativeBallisticAimError::DescriptorMismatch);
    }
    if !native_emitter_shape::<P>(entity) {
        return Err(NativeBallisticAimError::RuntimeContractMismatch);
    }
    if let Some(runtime) = P::queue(entity) {
        return if runtime.profile == P::ID
            && runtime.projectile_descriptor == descriptor
            && P::ID.accepts(descriptor)
        {
            Ok(())
        } else {
            Err(NativeBallisticAimError::RuntimeContractMismatch)
        };
    }
    *P::queue_slot_mut(entity) = Some(
        NativeBallisticAimRuntime::from_descriptor(P::ID, descriptor)
            .ok_or(NativeBallisticAimError::DescriptorMismatch)?,
    );
    Ok(())
}

pub(crate) fn tick_native_aim<P: NativeBallisticProfile>(
    dispatch_mode: CommonMoverDispatchMode,
    manager: &mut crate::entity::EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    elapsed_micros: u32,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<NativeBallisticAimTickOutcome, NativeBallisticAimError> {
    // E870 passes mode 1: the generic wrapper still ages Aim, but 2300
    // returns before reading its target or any projectile/emitter state.
    if dispatch_mode == CommonMoverDispatchMode::Restricted {
        let entity = manager
            .entity_mut(entity_id)
            .ok_or(NativeBallisticAimError::EntityUnavailable)?;
        authenticate_aim_owner::<P>(entity, false)?;
        let task_id = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary)
            .ok_or(NativeBallisticAimError::VisitUnavailable)?;
        let visit = ActorTaskVisit {
            slot: ActorTaskSlot::Tertiary,
            task_id,
        };
        let prefix = entity
            .actor_tasks
            .begin_exact_visit_with(visit, |task| {
                let ActorTaskRuntime::AimAndFire(task) = task else {
                    unreachable!("authenticated native Tertiary is AimAndFire");
                };
                task.before_callback(elapsed_micros)
            })
            .ok_or(NativeBallisticAimError::VisitUnavailable)?;
        if !entity.actor_tasks.finish_exact_visit(visit) {
            return Err(NativeBallisticAimError::CallbackWrapperDidNotSurvive);
        }
        return Ok(NativeBallisticAimTickOutcome {
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
        let Some(entity) = manager.entity_mut(entity_id) else {
            return Err(NativeBallisticAimError::EntityUnavailable);
        };
        authenticate_aim_owner::<P>(entity, false)?;
        ensure_native_aim_runtime::<P>(entity, metadata)?;
        let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary) else {
            return Err(NativeBallisticAimError::VisitUnavailable);
        };
        let task_visit = ActorTaskVisit {
            slot: ActorTaskSlot::Tertiary,
            task_id,
        };
        let Some(ActorTaskRuntime::AimAndFire(state)) =
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        else {
            return Err(NativeBallisticAimError::GraphMismatch);
        };
        let target_entity_id = state.private_state().target_entity_id();
        let owner_position_raw = entity.position_raw();
        let runtime = P::queue_slot_mut(entity)
            .as_mut()
            .ok_or(NativeBallisticAimError::RuntimeUnavailable)?;
        if runtime.profile != P::ID || !P::ID.accepts(runtime.projectile_descriptor) {
            return Err(NativeBallisticAimError::RuntimeContractMismatch);
        }
        let (parent_id, child_id) = runtime.take_transaction_pair();
        let descriptor = runtime.projectile_descriptor;
        let queued_before = runtime.transient_shots.len();
        let emitter_runtime =
            *P::emitter(entity).ok_or(NativeBallisticAimError::RuntimeUnavailable)?;
        let source_allocation_identity = transaction_scoped_allocation_identity(
            parent_id.get(),
            entity_id,
            P::ID.source_magic(),
        );
        let target_allocation_identity = transaction_scoped_allocation_identity(
            child_id.get(),
            target_entity_id,
            P::ID.target_magic(),
        );
        let (prefix, private_state) = entity
            .actor_tasks
            .begin_exact_visit_with(task_visit, |task| {
                let ActorTaskRuntime::AimAndFire(task) = task else {
                    unreachable!("authenticated native Tertiary is AimAndFire");
                };
                let private_state = task.private_state();
                (task.before_callback(elapsed_micros), private_state)
            })
            .ok_or(NativeBallisticAimError::VisitUnavailable)?;
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
        let target_state = aim_target_runtime_state(manager, target_entity_id)?;
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
        .map_err(NativeBallisticAimError::MachineStart)?;
        drive_aim_machine::<P>(
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
        .entity_mut(entity_id)
        .is_some_and(|entity| entity.actor_tasks.finish_exact_visit(task_visit));
    let completion = match callback {
        Ok(result) => result?,
        Err(payload) => std::panic::resume_unwind(payload),
    };
    if !survived {
        return Err(NativeBallisticAimError::CallbackWrapperDidNotSurvive);
    }
    let queued_after = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .and_then(|entity| P::queue(entity))
        .map(|runtime| runtime.transient_shots.len())
        .unwrap_or(queued_before);
    Ok(NativeBallisticAimTickOutcome {
        resolution: aim_and_fire_after_unwind(
            completion.committed_prefix,
            completion.callback_result,
        ),
        queued_shots_added: queued_after.saturating_sub(queued_before),
    })
}

/// Drain the entity-owned FIFO and materialize the authored projectile class.
///
/// `FUN_00411400` consumes every queued command even when `FUN_00440A60`
/// rejects a birth. Style, task wrappers, and later C690
/// graphs are irrelevant once the transient exists. The source allocation
/// retains its own descriptor and cadence across task replacement.
pub(crate) fn drain_native_shots<P: NativeBallisticProfile>(
    manager: &mut crate::entity::EntityManager,
    world_fx: &mut WorldFx,
    source_entity_id: u32,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Result<NativeBallisticShotDrainOutcome, NativeBallisticShotDrainError> {
    let (solutions, suppresses_impact_damage, owner_entity_type) = {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == source_entity_id)
            .ok_or(NativeBallisticShotDrainError::EntityUnavailable)?;
        if !P::allocation_authenticates(entity) || !native_emitter_shape::<P>(entity) {
            return Err(NativeBallisticShotDrainError::RuntimeContractMismatch);
        }
        let runtime = P::queue(entity).ok_or(NativeBallisticShotDrainError::RuntimeUnavailable)?;
        if runtime.profile != P::ID || !P::ID.accepts(runtime.projectile_descriptor) {
            return Err(NativeBallisticShotDrainError::RuntimeContractMismatch);
        }
        if runtime.transient_shots.is_empty() {
            return Ok(NativeBallisticShotDrainOutcome {
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
                return Err(NativeBallisticShotDrainError::ImpactSuppressionStateUnresolved);
            }
        };
        let mut solutions = Vec::with_capacity(runtime.transient_shots.len());
        for (index, request) in runtime.transient_shots.iter().copied().enumerate() {
            if !is_exact_transient_request::<P>(source_entity_id, request) {
                return Err(NativeBallisticShotDrainError::MalformedTransientRequest { index });
            }
            solutions.push(
                match P::ID {
                    // Method 10's leading dword is 1: 4E770 applies only
                    // positive closing-speed boost, without adding source velocity.
                    NativeBallisticProfileId::Type10
                    | NativeBallisticProfileId::Type5
                    | NativeBallisticProfileId::Type80
                    | NativeBallisticProfileId::Type126
                    | NativeBallisticProfileId::Type43
                    | NativeBallisticProfileId::Type38
                    | NativeBallisticProfileId::Type129 => drain_type13_transient_request(
                        request,
                        entity.position_raw(),
                        entity.velocity_raw(),
                    ),
                    NativeBallisticProfileId::Type16
                    | NativeBallisticProfileId::Type128
                    | NativeBallisticProfileId::Type58
                    | NativeBallisticProfileId::Type94
                    | NativeBallisticProfileId::Type57
                    | NativeBallisticProfileId::Type122
                    | NativeBallisticProfileId::Type30
                    | NativeBallisticProfileId::Type56
                    | NativeBallisticProfileId::Type40
                    | NativeBallisticProfileId::Type18 => drain_intro2_flyer_transient_request(
                        request,
                        entity.position_raw(),
                        entity.velocity_raw(),
                    ),
                }
                .map_err(NativeBallisticShotDrainError::ShotMath)?,
            );
        }
        (solutions, suppression, entity.entity_type as u8)
    };

    let queued = {
        let entity = manager
            .entity_mut(source_entity_id)
            .ok_or(NativeBallisticShotDrainError::EntityUnavailable)?;
        let runtime = P::queue_slot_mut(entity)
            .as_mut()
            .ok_or(NativeBallisticShotDrainError::RuntimeUnavailable)?;
        std::mem::take(&mut runtime.transient_shots)
    };
    debug_assert_eq!(queued.len(), solutions.len());

    let mut materialized_particle_classes = Vec::with_capacity(solutions.len());
    for solution in solutions {
        let particle_class = match P::ID {
            // 410B0 precedes 40A60 for method 10. At/below the sea plane
            // it substitutes class 46 and subtracts 100 from the 40A60
            // input; class46's +0x28 bias restores those units.
            NativeBallisticProfileId::Type10
            | NativeBallisticProfileId::Type5
            | NativeBallisticProfileId::Type80
            | NativeBallisticProfileId::Type126
            | NativeBallisticProfileId::Type43
            | NativeBallisticProfileId::Type38
            | NativeBallisticProfileId::Type129 => world_fx
                .materialize_class_38_request(
                    Class38ParticleRequest {
                        position_raw: solution.position_raw,
                        velocity_raw: solution.velocity_raw,
                        owner_entity_id: source_entity_id,
                        owner_entity_type,
                        suppresses_impact_damage,
                    },
                    environment,
                    retail_tick,
                )
                .map(|birth| birth.particle_class),
            NativeBallisticProfileId::Type16
            | NativeBallisticProfileId::Type128
            | NativeBallisticProfileId::Type58
            | NativeBallisticProfileId::Type94
            | NativeBallisticProfileId::Type57
            | NativeBallisticProfileId::Type122
            | NativeBallisticProfileId::Type30
            | NativeBallisticProfileId::Type56
            | NativeBallisticProfileId::Type40
            | NativeBallisticProfileId::Type18 => world_fx
                .materialize_descriptor_particle_request(
                    DescriptorParticleRequest {
                        source_class: P::ID.particle_class(),
                        position_raw: solution.position_raw,
                        velocity_raw: solution.velocity_raw,
                        owner: Some(ParticleOwnerAtBirth {
                            entity_id: source_entity_id,
                            entity_type: owner_entity_type,
                        }),
                        suppresses_impact_damage,
                    },
                    environment,
                    retail_tick,
                )
                .map(|_| P::ID.particle_class()),
        };
        if let Some(particle_class) = particle_class {
            materialized_particle_classes.push(particle_class);
        }
    }
    Ok(NativeBallisticShotDrainOutcome {
        consumed_requests: queued.len(),
        materialized_particle_classes,
    })
}

fn drive_aim_machine<P: NativeBallisticProfile>(
    machine: &mut AimAndFireEmitterMachine,
    manager: &mut crate::entity::EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    task_visit: ActorTaskVisit,
    child_transaction_id: GenericEmitterTransactionId,
    source_allocation_identity: u64,
    target_allocation_identity: u64,
) -> Result<crate::aim_and_fire_transaction::AimAndFireEmitterCompletion, NativeBallisticAimError> {
    loop {
        match machine.poll() {
            AimAndFireEmitterPoll::Action(issued) => {
                let action_phase = issued.action.phase();
                let expected_lease = AimAndFireEmitterLease {
                    owner_entity_id: entity_id,
                    task_visit,
                };
                if issued.action.lease() != expected_lease {
                    return Err(NativeBallisticAimError::TaskLeaseMismatch);
                }
                revalidate_active_owner::<P>(manager, entity_id, task_visit)?;
                let response = match issued.action {
                    AimAndFireEmitterAction::GenericEmitter {
                        phase,
                        child_transaction_id: actual_child,
                        action,
                        ..
                    } if actual_child == child_transaction_id
                        && phase == AimAndFireEmitterPhase::GenericEmitter(action.phase()) =>
                    {
                        let response = handle_generic_action::<P>(
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
                    _ => return Err(NativeBallisticAimError::ActionContractMismatch),
                };
                let _ = action_phase;
                machine
                    .resume(issued.receipt, response)
                    .map_err(|failure| NativeBallisticAimError::MachineProtocol(failure.error))?;
            }
            AimAndFireEmitterPoll::Awaiting(_) => {
                return Err(NativeBallisticAimError::ActionContractMismatch);
            }
            AimAndFireEmitterPoll::Blocked(block) => {
                return Err(NativeBallisticAimError::MachineBlocked(block));
            }
            AimAndFireEmitterPoll::Complete(completion) => return Ok(completion),
        }
    }
}

fn handle_generic_action<P: NativeBallisticProfile>(
    manager: &mut crate::entity::EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    task_visit: ActorTaskVisit,
    action: GenericEmitterAction,
    source_allocation_identity: u64,
    target_allocation_identity: u64,
) -> Result<GenericEmitterResume, NativeBallisticAimError> {
    let phase = action.phase();
    let source = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .ok_or(NativeBallisticAimError::EntityUnavailable)?;
    let descriptor = P::queue(source)
        .ok_or(NativeBallisticAimError::RuntimeUnavailable)?
        .projectile_descriptor;
    let target_entity_id = {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .ok_or(NativeBallisticAimError::EntityUnavailable)?;
        let Some(ActorTaskRuntime::AimAndFire(state)) =
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        else {
            return Err(NativeBallisticAimError::GraphMismatch);
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
                let target = manager
                    .iter_all()
                    .find(|entity| entity.id == handle && entity.active);
                target.map(|entity| entity_snapshot(entity, target_allocation_identity))
            } else {
                return Err(NativeBallisticAimError::ActionContractMismatch);
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
                .entity_mut(entity_id)
                .ok_or(NativeBallisticAimError::EntityUnavailable)?;
            authenticate_aim_owner::<P>(entity, true)?;
            let runtime =
                P::emitter_mut(entity).ok_or(NativeBallisticAimError::RuntimeUnavailable)?;
            if runtime.cadence_raw != write.before_raw || write.after_raw != expected_after {
                return Err(NativeBallisticAimError::RuntimeContractMismatch);
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
                return Err(NativeBallisticAimError::ActionContractMismatch);
            }
            let entity = manager
                .iter_all()
                .find(|entity| entity.id == entity_id && entity.active)
                .ok_or(NativeBallisticAimError::EntityUnavailable)?;
            authenticate_aim_owner::<P>(entity, true)?;
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
            let source = source_for_math::<P>(manager, entity_id, source_handle, task_visit)?;
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
            let source = source_for_math::<P>(manager, entity_id, source_handle, task_visit)?;
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
                || request.projectile_method != P::ID.method()
                || request.speed_override_raw != P::ID.speed_override()
                || request.current_target.is_some_and(|target| {
                    target.handle != target_entity_id
                        || target.allocation_identity != target_allocation_identity
                })
            {
                return Err(NativeBallisticAimError::ActionContractMismatch);
            }
            let solution = resolve_ballistic_target_launch(request, P::ID.table_speed())
                .map_err(NativeBallisticAimError::ShotMath)?;
            Ok(GenericEmitterResume::TargetLaunchResolved { phase, solution })
        }
        GenericEmitterAction::AppendTransient { request, .. } => {
            if !is_exact_transient_request::<P>(entity_id, request) {
                return Err(NativeBallisticAimError::ActionContractMismatch);
            }
            let entity = manager
                .entity_mut(entity_id)
                .ok_or(NativeBallisticAimError::EntityUnavailable)?;
            authenticate_aim_owner::<P>(entity, true)?;
            P::queue_slot_mut(entity)
                .as_mut()
                .ok_or(NativeBallisticAimError::RuntimeUnavailable)?
                .transient_shots
                .push(request);
            Ok(GenericEmitterResume::AppendReturned {
                phase,
                result_raw: 0,
            })
        }
        GenericEmitterAction::CommitTargetCadence { write, .. } => {
            if write.interval_us != descriptor.random_interval_us
                || write.cadence_after_raw
                    != write
                        .cadence_before_raw
                        .wrapping_add(write.interval_us as i32)
                || write.time_offset_after_raw
                    != write
                        .time_offset_before_raw
                        .wrapping_add(write.interval_us as i32)
            {
                return Err(NativeBallisticAimError::RuntimeContractMismatch);
            }
            let entity = manager
                .entity_mut(entity_id)
                .ok_or(NativeBallisticAimError::EntityUnavailable)?;
            authenticate_aim_owner::<P>(entity, true)?;
            let runtime =
                P::emitter_mut(entity).ok_or(NativeBallisticAimError::RuntimeUnavailable)?;
            if runtime.cadence_raw != write.cadence_before_raw {
                return Err(NativeBallisticAimError::RuntimeContractMismatch);
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
                .ok_or(NativeBallisticAimError::EntityUnavailable)?;
            authenticate_aim_owner::<P>(source, true)?;
            let expected_position = source.position_raw();
            if sound_id != u32::from(P::ID.sound())
                || source_position_raw != Some(expected_position)
                || frequency_multiplier_16_16 != 0x0001_0000
                || volume_multiplier_16_16 != 0x0001_0000
            {
                return Err(NativeBallisticAimError::ActionContractMismatch);
            }
            world_fx.queue_fixed_positional_sound_raw(P::ID.sound(), expected_position);
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
            Err(NativeBallisticAimError::ActionContractMismatch)
        }
    }
}

fn authenticate_aim_owner<P: NativeBallisticProfile>(
    entity: &Entity,
    expected_in_callback: bool,
) -> Result<(), NativeBallisticAimError> {
    if !entity.active || !P::allocation_authenticates(entity) || !P::graph_authenticates(entity) {
        return Err(NativeBallisticAimError::GraphMismatch);
    }
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary) else {
        return Err(NativeBallisticAimError::VisitUnavailable);
    };
    let flags = entity
        .actor_tasks
        .wrapper_flags(task_id)
        .ok_or(NativeBallisticAimError::VisitUnavailable)?;
    if !flags.alive || flags.in_callback != expected_in_callback {
        return Err(NativeBallisticAimError::VisitUnavailable);
    }
    Ok(())
}

fn revalidate_active_owner<P: NativeBallisticProfile>(
    manager: &crate::entity::EntityManager,
    entity_id: u32,
    task_visit: ActorTaskVisit,
) -> Result<(), NativeBallisticAimError> {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .ok_or(NativeBallisticAimError::EntityUnavailable)?;
    if entity.actor_tasks.task_in_slot(task_visit.slot) != Some(task_visit.task_id) {
        return Err(NativeBallisticAimError::TaskLeaseMismatch);
    }
    authenticate_aim_owner::<P>(entity, true)
}

fn aim_target_runtime_state(
    manager: &crate::entity::EntityManager,
    target_entity_id: u32,
) -> Result<AimAndFireTargetRuntimeState, NativeBallisticAimError> {
    let Some(target) = manager
        .iter_all()
        .find(|entity| entity.id == target_entity_id && entity.active)
    else {
        return Ok(AimAndFireTargetRuntimeState::Missing);
    };
    let state = target.collision.state_flags_at_0x08;
    let dying = match state.masked(AIM_AND_FIRE_DYING_STATE_BIT) {
        RetailRuntimeValue::Known(value) => value,
        RetailRuntimeValue::Unresolved => {
            return Err(NativeBallisticAimError::TargetStateUnresolved)
        }
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
    Err(NativeBallisticAimError::TargetStateUnresolved)
}

fn source_for_math<'a, P: NativeBallisticProfile>(
    manager: &'a crate::entity::EntityManager,
    entity_id: u32,
    source_handle: u32,
    _task_visit: ActorTaskVisit,
) -> Result<&'a Entity, NativeBallisticAimError> {
    if source_handle != entity_id {
        return Err(NativeBallisticAimError::ActionContractMismatch);
    }
    let source = manager
        .iter_all()
        .find(|entity| entity.id == source_handle && entity.active)
        .ok_or(NativeBallisticAimError::EntityUnavailable)?;
    authenticate_aim_owner::<P>(source, true)?;
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

fn retained_aim_basis(entity: &Entity) -> Result<Type9BodyBasis, NativeBallisticAimError> {
    // FUN_0041E4D0 reads retained lateral +0x0C..+0x14 and forward
    // +0x24..+0x2C; FUN_0041E930 reads that same forward column. Chase can
    // already have changed the angle words, but DCA0/E870 calls FUN_00413F70
    // only after the whole A800 traversal, including this same-pass Aim.
    match entity.physical_body_basis_q31() {
        RetailRuntimeValue::Known(basis) => Ok(basis),
        RetailRuntimeValue::Unresolved => Err(NativeBallisticAimError::BodyBasisUnresolved),
    }
}

fn native_emitter_shape<P: NativeBallisticProfile>(entity: &Entity) -> bool {
    P::emitter(entity).is_some_and(|emitter| {
        let emitter = *emitter;
        emitter
            == GenericEmitterRuntime {
                joint_bindings: [None; 2],
                projectile_method: P::ID.method(),
                emitter_selector: 0,
                sound_id: u32::from(P::ID.sound()),
                direct_mode: 0,
                remaining_time_raw: 0,
                manual_step_raw: 0,
                remaining_bursts_raw: 0,
                cadence_raw: emitter.cadence_raw,
                basis_adjustment_identity: None,
            }
    })
}

fn is_exact_transient_request<P: NativeBallisticProfile>(
    source_entity_id: u32,
    request: GenericEmitterAppendRequest,
) -> bool {
    request.source_handle == source_entity_id
        && request.owner_handle == source_entity_id
        && request.projectile_method == P::ID.method()
        && request.emitter_selector == 0
        && request.speed_field == GenericEmitterSpeedField::Explicit(P::ID.launch_speed())
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
