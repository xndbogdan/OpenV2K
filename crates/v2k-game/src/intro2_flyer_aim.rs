//! Flyer (wasp/deathwas/Type108/Type39) Aim, using the shared `FUN_00424650` transaction.
//!
//! Their Sub-E descriptors own cadence, spread and sound; methods 30, 20 and 24
//! select classes 87, 52 and 68. All use the original ballistic target launch
//! and inherited-source-velocity drain, independently of the Ptersect method 10.

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
    GenericEmitterSpeedField, GenericEmitterTransactionId, GENERIC_PROJECTILE_AUXILIARY_METHOD,
};
use crate::intro2_flyers_live::intro2_flyer_pursuing_graph_authenticates;
use crate::ordinary_type47_shot_math::{
    drain_intro2_flyer_transient_request, evaluate_type47_aim_error,
    evaluate_type47_forward_half_space, resolve_ballistic_target_launch, Type47ShotMathError,
};
use crate::projectile_emitter::projectile_class_row;
use crate::world_fx::{
    DescriptorParticleRequest, ParticleEnvironment, ParticleOwnerAtBirth, WorldFx,
};

const IMPACT_SUPPRESSION_STATE_BIT: u32 = 0x8000_0000;
const FLYER_SOURCE_ALLOCATION_MAGIC: u64 = 0x464C_5945_5253_5243;
const FLYER_TARGET_ALLOCATION_MAGIC: u64 = 0x464C_5945_5254_4754;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlyerEmitterProfile {
    Wasp,
    Deathwas,
    Type108,
    Type39,
    /// The attract craft: type 46's Sub-E, which `FUN_004503C0` arms from
    /// the attract row on every visit (method, sound, cadence, auxiliary
    /// command and gate mode 1).
    Craft,
}

impl FlyerEmitterProfile {
    fn from_entity(entity: &Entity) -> Option<Self> {
        if !crate::intro2_flyers_live::flyer_identity_authenticates(entity) {
            return None;
        }
        Self::from_type(entity.entity_type)
    }

    const fn from_type(entity_type: u32) -> Option<Self> {
        match entity_type {
            15 => Some(Self::Wasp),
            87 => Some(Self::Deathwas),
            108 => Some(Self::Type108),
            39 => Some(Self::Type39),
            46 => Some(Self::Craft),
            _ => None,
        }
    }

    /// The authored method; the craft's runtime word is the row's.
    fn method(self) -> u32 {
        match self {
            Self::Wasp => 30,
            Self::Deathwas | Self::Type39 => 20,
            Self::Type108 => 24,
            Self::Craft => 18,
        }
    }
    fn sound(self) -> u16 {
        match self {
            Self::Wasp => 70,
            Self::Deathwas => 92,
            Self::Type108 | Self::Type39 => 105,
            Self::Craft => 88,
        }
    }
    /// Sub-E random interval and target-axis tolerance.
    fn cadence(self) -> (u32, u16) {
        match self {
            Self::Wasp | Self::Deathwas => (600_000, 2_560),
            Self::Type108 => (400_000, 3_072),
            Self::Type39 => (2_000_000, 2_560),
            Self::Craft => (75_000, 5_120),
        }
    }
    /// Sub-E `+0x0C`; zero takes the method's table speed.
    fn speed_override(self) -> i16 {
        match self {
            Self::Type39 => 512,
            _ => 0,
        }
    }
    fn particle_class(self) -> u8 {
        projectile_class_row(self.method()).unwrap().particle_class
    }
    fn emitter(self) -> GenericEmitterRuntime {
        GenericEmitterRuntime {
            joint_bindings: [None; 2],
            // FUN_00443560 replaces the craft's method with the session
            // record's active weapon: the boot record's slot 0 (selector 1).
            projectile_method: if self == Self::Craft {
                CRAFT_BOOT_RECORD_SELECTOR
            } else {
                self.method()
            },
            emitter_selector: 0,
            sound_id: u32::from(self.sound()),
            direct_mode: 0,
            remaining_time_raw: 0,
            manual_step_raw: 0,
            remaining_bursts_raw: 0,
            cadence_raw: 0,
            basis_adjustment_identity: None,
        }
    }
    fn accepts(self, descriptor: ProjectileEmitterDescriptor) -> bool {
        if self == Self::Craft {
            // Type 46's record; cadence, gate mode and auxiliary command are
            // the attract row's writes.
            return descriptor
                == ProjectileEmitterDescriptor {
                    random_interval_us: descriptor.random_interval_us,
                    stochastic_gate_mode: descriptor.stochastic_gate_mode,
                    auxiliary_command: descriptor.auxiliary_command,
                    ..CRAFT_SUB_E
                };
        }
        descriptor.projectile_method == self.method()
            && descriptor.sound_id == self.sound()
            && (
                descriptor.random_interval_us,
                descriptor.target_axis_tolerance_raw,
            ) == self.cadence()
            && descriptor.spread_raw == 256
            && descriptor.aim_threshold_raw == 4_000
            && descriptor.raw_word_at_0x12 == 0
            && descriptor.stochastic_gate_mode == 0
            && descriptor.variable_bindings == [0; 4]
            && descriptor.alternate_emitter_raw == 0
            && descriptor.auxiliary_command == 0
            && descriptor.speed_override_raw == self.speed_override()
    }
}

/// `DAT_004D0B30 + 0x1C`: the boot session record's first weapon, selector
/// 1 (slot byte `+0x122` is 0).
const CRAFT_BOOT_RECORD_SELECTOR: u32 = 1;

/// Type 46's authored Sub-E.
const CRAFT_SUB_E: ProjectileEmitterDescriptor = ProjectileEmitterDescriptor {
    projectile_method: 18,
    random_interval_us: 75_000,
    spread_raw: 0,
    aim_threshold_raw: 5_000,
    speed_override_raw: 0,
    target_axis_tolerance_raw: 5_120,
    sound_id: 88,
    raw_word_at_0x12: 18,
    alternate_emitter_raw: 20,
    stochastic_gate_mode: 0,
    auxiliary_command: 1,
    variable_bindings: [1, 2, 3, 4],
};

pub(crate) fn flyer_projectile_descriptor_authenticates(
    entity_type: u32,
    metadata: &EntityTypeRuntimeMetadata,
) -> bool {
    let Some(profile) = FlyerEmitterProfile::from_type(entity_type) else {
        return false;
    };
    matches!(metadata.projectile_emitter_descriptor,
        RetailRuntimeValue::Known(Some(descriptor)) if profile.accepts(descriptor))
}

/// Entity-owned flyer Sub-E cadence and later-drained transient FIFO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intro2FlyerAimRuntime {
    profile: FlyerEmitterProfile,
    projectile_descriptor: ProjectileEmitterDescriptor,
    emitter_runtime: GenericEmitterRuntime,
    transient_shots: Vec<GenericEmitterAppendRequest>,
    next_transaction_id: u64,
    /// The word Sub-E's `+0x0C` binding points at (model variable 4): the
    /// weapon record's `+0x08` byte, which selects the gun the craft shows.
    weapon_var_raw: i16,
}

/// `FUN_004503C0`'s writes for an attract row: the Sub-E runtime's method
/// (`+10`) and sound (`+20`), and type 46's shared Sub-E cadence (`+04`),
/// gate mode (`+16`, 1) and auxiliary command (`+17`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CraftEmitterArming {
    pub projectile_method: u32,
    pub sound_id: u32,
    pub cadence_us: u32,
    pub auxiliary_command: u8,
    /// Row `+03`, written through the runtime's `+0x0C` binding.
    pub weapon_var_raw: i16,
}

impl Intro2FlyerAimRuntime {
    pub fn arm_craft(&mut self, arming: CraftEmitterArming) {
        self.emitter_runtime.projectile_method = arming.projectile_method;
        self.emitter_runtime.sound_id = arming.sound_id;
        self.weapon_var_raw = arming.weapon_var_raw;
        self.restore_craft_record(arming);
    }

    pub const fn weapon_var_raw(&self) -> i16 {
        self.weapon_var_raw
    }

    /// The type record keeps an earlier demo's writes: a later craft starts
    /// from them until its own first `FUN_004503C0` visit.
    pub fn restore_craft_record(&mut self, arming: CraftEmitterArming) {
        self.projectile_descriptor.random_interval_us = arming.cadence_us;
        self.projectile_descriptor.stochastic_gate_mode = 1;
        self.projectile_descriptor.auxiliary_command = arming.auxiliary_command;
    }

    fn method(&self) -> u32 {
        self.emitter_runtime.projectile_method
    }

    /// `FUN_00401430`'s second correction clears runtime `+0x10` and may
    /// restore it.
    pub(crate) fn set_projectile_method(&mut self, method: u32) {
        self.emitter_runtime.projectile_method = method;
    }

    fn sound(&self) -> u16 {
        self.emitter_runtime.sound_id as u16
    }

    fn row(&self) -> Option<crate::projectile_emitter::ProjectileClassRow> {
        projectile_class_row(self.method())
    }

    /// Sub-E `+0x0C`, else the method's table speed.
    fn launch_speed(&self) -> Option<i16> {
        match self.projectile_descriptor.speed_override_raw {
            0 => self.row().map(|row| row.speed_raw as i16),
            speed => Some(speed),
        }
    }

    fn from_descriptor(
        profile: FlyerEmitterProfile,
        descriptor: ProjectileEmitterDescriptor,
    ) -> Option<Self> {
        if !profile.accepts(descriptor) {
            return None;
        }
        Some(Self {
            profile,
            projectile_descriptor: descriptor,
            emitter_runtime: profile.emitter(),
            transient_shots: Vec::new(),
            next_transaction_id: 1,
            // FUN_00443560: the boot record's slot-0 weapon record `+0x08`.
            weapon_var_raw: if profile == FlyerEmitterProfile::Craft {
                i16::from(
                    crate::weapon_inventory::WeaponInventory::new()
                        .selected_descriptor()
                        .callback_selector() as i8,
                )
            } else {
                0
            },
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
pub enum Intro2FlyerAimError {
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
pub struct Intro2FlyerAimTickOutcome {
    pub resolution: AimAndFireFrameResolution,
    pub queued_shots_added: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2FlyerShotDrainError {
    EntityUnavailable,
    RuntimeUnavailable,
    RuntimeContractMismatch,
    MalformedTransientRequest { index: usize },
    ShotMath(Type47ShotMathError),
    ImpactSuppressionStateUnresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intro2FlyerShotDrainOutcome {
    pub consumed_requests: usize,
    pub materialized_particle_classes: Vec<u8>,
}

pub fn ensure_intro2_flyer_aim_runtime(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<(), Intro2FlyerAimError> {
    let Some(metadata) = metadata else {
        return Err(Intro2FlyerAimError::DescriptorMismatch);
    };
    let RetailRuntimeValue::Known(Some(descriptor)) = metadata.projectile_emitter_descriptor else {
        return Err(Intro2FlyerAimError::DescriptorMismatch);
    };
    let profile =
        FlyerEmitterProfile::from_entity(entity).ok_or(Intro2FlyerAimError::GraphMismatch)?;
    if let Some(runtime) = &entity.intro2_flyer_aim_runtime {
        // The craft's copy carries the attract row's writes to the record.
        return if runtime.profile == profile
            && (runtime.projectile_descriptor == descriptor
                || profile == FlyerEmitterProfile::Craft)
            && is_exact_runtime_shape(runtime)
        {
            Ok(())
        } else {
            Err(Intro2FlyerAimError::RuntimeContractMismatch)
        };
    }
    entity.intro2_flyer_aim_runtime = Some(
        Intro2FlyerAimRuntime::from_descriptor(profile, descriptor)
            .ok_or(Intro2FlyerAimError::DescriptorMismatch)?,
    );
    Ok(())
}

pub fn tick_intro2_flyer_aim(
    dispatch_mode: CommonMoverDispatchMode,
    manager: &mut crate::entity::EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    elapsed_micros: u32,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<Intro2FlyerAimTickOutcome, Intro2FlyerAimError> {
    if !crate::intro2_flyers_live::flyer_manager_identity_authenticates(manager, entity_id) {
        return Err(Intro2FlyerAimError::GraphMismatch);
    }
    // E870 passes mode 1: the generic wrapper still ages Aim, but 2300
    // returns before reading its target or any projectile/emitter state.
    if dispatch_mode == CommonMoverDispatchMode::Restricted {
        let entity = manager
            .entity_mut(entity_id)
            .ok_or(Intro2FlyerAimError::EntityUnavailable)?;
        authenticate_intro2_flyer_aim_owner(entity, false)?;
        let task_id = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary)
            .ok_or(Intro2FlyerAimError::VisitUnavailable)?;
        let visit = ActorTaskVisit {
            slot: ActorTaskSlot::Tertiary,
            task_id,
        };
        let prefix = entity
            .actor_tasks
            .begin_exact_visit_with(visit, |task| {
                let ActorTaskRuntime::AimAndFire(task) = task else {
                    unreachable!("authenticated flyer Tertiary is AimAndFire");
                };
                task.before_callback(elapsed_micros)
            })
            .ok_or(Intro2FlyerAimError::VisitUnavailable)?;
        if !entity.actor_tasks.finish_exact_visit(visit) {
            return Err(Intro2FlyerAimError::CallbackWrapperDidNotSurvive);
        }
        return Ok(Intro2FlyerAimTickOutcome {
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
            return Err(Intro2FlyerAimError::EntityUnavailable);
        };
        authenticate_intro2_flyer_aim_owner(entity, false)?;
        ensure_intro2_flyer_aim_runtime(entity, metadata)?;
        let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary) else {
            return Err(Intro2FlyerAimError::VisitUnavailable);
        };
        let task_visit = ActorTaskVisit {
            slot: ActorTaskSlot::Tertiary,
            task_id,
        };
        let Some(ActorTaskRuntime::AimAndFire(state)) =
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        else {
            return Err(Intro2FlyerAimError::GraphMismatch);
        };
        let target_entity_id = state.private_state().target_entity_id();
        let owner_position_raw = entity.position_raw();
        let runtime = entity
            .intro2_flyer_aim_runtime
            .as_mut()
            .ok_or(Intro2FlyerAimError::RuntimeUnavailable)?;
        if !is_exact_runtime_shape(runtime) {
            return Err(Intro2FlyerAimError::RuntimeContractMismatch);
        }
        let (parent_id, child_id) = runtime.take_transaction_pair();
        let descriptor = runtime.projectile_descriptor;
        let emitter_runtime = runtime.emitter_runtime;
        let queued_before = runtime.transient_shots.len();
        let source_allocation_identity = transaction_scoped_allocation_identity(
            parent_id.get(),
            entity_id,
            FLYER_SOURCE_ALLOCATION_MAGIC,
        );
        let target_allocation_identity = transaction_scoped_allocation_identity(
            child_id.get(),
            target_entity_id,
            FLYER_TARGET_ALLOCATION_MAGIC,
        );
        let (prefix, private_state) = entity
            .actor_tasks
            .begin_exact_visit_with(task_visit, |task| {
                let ActorTaskRuntime::AimAndFire(task) = task else {
                    unreachable!("authenticated flyer Tertiary is AimAndFire");
                };
                let private_state = task.private_state();
                (task.before_callback(elapsed_micros), private_state)
            })
            .ok_or(Intro2FlyerAimError::VisitUnavailable)?;
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
        let target_state = flyer_aim_target_runtime_state(manager, target_entity_id)?;
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
        .map_err(Intro2FlyerAimError::MachineStart)?;
        drive_flyer_aim_machine(
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
        return Err(Intro2FlyerAimError::CallbackWrapperDidNotSurvive);
    }
    let queued_after = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .and_then(|entity| entity.intro2_flyer_aim_runtime.as_ref())
        .map(|runtime| runtime.transient_shots.len())
        .unwrap_or(queued_before);
    Ok(Intro2FlyerAimTickOutcome {
        resolution: aim_and_fire_after_unwind(
            completion.committed_prefix,
            completion.callback_result,
        ),
        queued_shots_added: queued_after.saturating_sub(queued_before),
    })
}

/// Drain the entity-owned method-20/30 FIFO and materialize the authored projectile class.
///
/// `FUN_00411400` consumes every queued command even when `FUN_00440A60`
/// rejects a birth. Style, task wrappers, and later C690
/// graphs are irrelevant once the transient exists. The source allocation
/// retains its own descriptor and cadence across task replacement.
pub fn drain_intro2_flyer_shots(
    manager: &mut crate::entity::EntityManager,
    world_fx: &mut WorldFx,
    source_entity_id: u32,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Result<Intro2FlyerShotDrainOutcome, Intro2FlyerShotDrainError> {
    if !crate::intro2_flyers_live::flyer_manager_identity_authenticates(manager, source_entity_id) {
        return Err(Intro2FlyerShotDrainError::RuntimeContractMismatch);
    }
    let (solutions, suppresses_impact_damage, profile, owner_entity_type) = {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == source_entity_id)
            .ok_or(Intro2FlyerShotDrainError::EntityUnavailable)?;
        let profile = FlyerEmitterProfile::from_entity(entity)
            .ok_or(Intro2FlyerShotDrainError::RuntimeContractMismatch)?;
        let runtime = entity
            .intro2_flyer_aim_runtime
            .as_ref()
            .ok_or(Intro2FlyerShotDrainError::RuntimeUnavailable)?;
        if !is_exact_runtime_shape(runtime) {
            return Err(Intro2FlyerShotDrainError::RuntimeContractMismatch);
        }
        if runtime.transient_shots.is_empty() {
            return Ok(Intro2FlyerShotDrainOutcome {
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
                return Err(Intro2FlyerShotDrainError::ImpactSuppressionStateUnresolved);
            }
        };
        let mut solutions = Vec::with_capacity(runtime.transient_shots.len());
        for (index, request) in runtime.transient_shots.iter().copied().enumerate() {
            if !is_exact_transient_request(runtime, source_entity_id, request) {
                return Err(Intro2FlyerShotDrainError::MalformedTransientRequest { index });
            }
            let origin = entity.emitter_muzzles[usize::from(request.emitter_selector)]
                .unwrap_or(entity.position_raw());
            solutions.push(if request.auxiliary {
                // 4E770's method-15 row: class 32 at the muzzle with the
                // source's velocity alone (FUN_004410B0 drops it at or below
                // the sea).
                FlyerShot::Auxiliary {
                    position_raw: origin,
                    velocity_raw: entity.velocity_raw(),
                }
            } else if profile == FlyerEmitterProfile::Craft {
                let row = projectile_class_row(request.projectile_method)
                    .ok_or(Intro2FlyerShotDrainError::RuntimeContractMismatch)?;
                let solution = crate::ordinary_type47_shot_math::drain_transient_with_method_row(
                    request,
                    origin,
                    entity.velocity_raw(),
                    row,
                )
                .map_err(Intro2FlyerShotDrainError::ShotMath)?;
                FlyerShot::Primary {
                    class: row.particle_class,
                    position_raw: solution.position_raw,
                    velocity_raw: solution.velocity_raw,
                }
            } else {
                let solution =
                    drain_intro2_flyer_transient_request(request, origin, entity.velocity_raw())
                        .map_err(Intro2FlyerShotDrainError::ShotMath)?;
                FlyerShot::Primary {
                    class: profile.particle_class(),
                    position_raw: solution.position_raw,
                    velocity_raw: solution.velocity_raw,
                }
            });
        }
        (solutions, suppression, profile, entity.entity_type as u8)
    };

    let queued = {
        let entity = manager
            .entity_mut(source_entity_id)
            .ok_or(Intro2FlyerShotDrainError::EntityUnavailable)?;
        let runtime = entity
            .intro2_flyer_aim_runtime
            .as_mut()
            .ok_or(Intro2FlyerShotDrainError::RuntimeUnavailable)?;
        std::mem::take(&mut runtime.transient_shots)
    };
    debug_assert_eq!(queued.len(), solutions.len());

    let _ = profile;
    let mut materialized_particle_classes = Vec::with_capacity(solutions.len());
    for solution in solutions {
        let owner = Some(ParticleOwnerAtBirth {
            entity_id: source_entity_id,
            entity_type: owner_entity_type,
        });
        match solution {
            FlyerShot::Primary {
                class,
                position_raw,
                velocity_raw,
            } => {
                if world_fx
                    .materialize_descriptor_particle_request(
                        DescriptorParticleRequest {
                            source_class: class,
                            position_raw,
                            velocity_raw,
                            owner,
                            suppresses_impact_damage,
                        },
                        environment,
                        retail_tick,
                    )
                    .is_some()
                {
                    materialized_particle_classes.push(class);
                }
            }
            FlyerShot::Auxiliary {
                position_raw,
                velocity_raw,
            } => {
                let class = projectile_class_row(GENERIC_PROJECTILE_AUXILIARY_METHOD)
                    .expect("method 15 is an authored row")
                    .particle_class;
                world_fx.materialize_combat_projectile_410b0(
                    DescriptorParticleRequest {
                        source_class: class,
                        position_raw,
                        velocity_raw,
                        owner,
                        suppresses_impact_damage,
                    },
                    environment,
                    retail_tick,
                );
                materialized_particle_classes.push(class);
            }
        }
    }
    Ok(Intro2FlyerShotDrainOutcome {
        consumed_requests: queued.len(),
        materialized_particle_classes,
    })
}

/// One drained command, from its row.
enum FlyerShot {
    Primary {
        class: u8,
        position_raw: [i16; 3],
        velocity_raw: [i16; 3],
    },
    Auxiliary {
        position_raw: [i16; 3],
        velocity_raw: [i16; 3],
    },
}

fn drive_flyer_aim_machine(
    machine: &mut AimAndFireEmitterMachine,
    manager: &mut crate::entity::EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    task_visit: ActorTaskVisit,
    child_transaction_id: GenericEmitterTransactionId,
    source_allocation_identity: u64,
    target_allocation_identity: u64,
) -> Result<crate::aim_and_fire_transaction::AimAndFireEmitterCompletion, Intro2FlyerAimError> {
    loop {
        match machine.poll() {
            AimAndFireEmitterPoll::Action(issued) => {
                let action_phase = issued.action.phase();
                let expected_lease = AimAndFireEmitterLease {
                    owner_entity_id: entity_id,
                    task_visit,
                };
                if issued.action.lease() != expected_lease {
                    return Err(Intro2FlyerAimError::TaskLeaseMismatch);
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
                        let response = handle_flyer_generic_action(
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
                    _ => return Err(Intro2FlyerAimError::ActionContractMismatch),
                };
                let _ = action_phase;
                machine
                    .resume(issued.receipt, response)
                    .map_err(|failure| Intro2FlyerAimError::MachineProtocol(failure.error))?;
            }
            AimAndFireEmitterPoll::Awaiting(_) => {
                return Err(Intro2FlyerAimError::ActionContractMismatch);
            }
            AimAndFireEmitterPoll::Blocked(block) => {
                return Err(Intro2FlyerAimError::MachineBlocked(block));
            }
            AimAndFireEmitterPoll::Complete(completion) => return Ok(completion),
        }
    }
}

fn handle_flyer_generic_action(
    manager: &mut crate::entity::EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    task_visit: ActorTaskVisit,
    action: GenericEmitterAction,
    source_allocation_identity: u64,
    target_allocation_identity: u64,
) -> Result<GenericEmitterResume, Intro2FlyerAimError> {
    let phase = action.phase();
    let source = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .ok_or(Intro2FlyerAimError::EntityUnavailable)?;
    FlyerEmitterProfile::from_entity(source).ok_or(Intro2FlyerAimError::GraphMismatch)?;
    let aim_runtime = source
        .intro2_flyer_aim_runtime
        .clone()
        .ok_or(Intro2FlyerAimError::RuntimeUnavailable)?;
    let descriptor = aim_runtime.projectile_descriptor;
    let target_entity_id = {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .ok_or(Intro2FlyerAimError::EntityUnavailable)?;
        let Some(ActorTaskRuntime::AimAndFire(state)) =
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        else {
            return Err(Intro2FlyerAimError::GraphMismatch);
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
                return Err(Intro2FlyerAimError::ActionContractMismatch);
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
                .ok_or(Intro2FlyerAimError::EntityUnavailable)?;
            authenticate_intro2_flyer_aim_owner(entity, true)?;
            let runtime = &mut entity
                .intro2_flyer_aim_runtime
                .as_mut()
                .ok_or(Intro2FlyerAimError::RuntimeUnavailable)?
                .emitter_runtime;
            if runtime.cadence_raw != write.before_raw || write.after_raw != expected_after {
                return Err(Intro2FlyerAimError::RuntimeContractMismatch);
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
                return Err(Intro2FlyerAimError::ActionContractMismatch);
            }
            let entity = manager
                .iter_all()
                .find(|entity| entity.id == entity_id && entity.active)
                .ok_or(Intro2FlyerAimError::EntityUnavailable)?;
            authenticate_intro2_flyer_aim_owner(entity, true)?;
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
                || request.projectile_method != aim_runtime.method()
                || request.speed_override_raw != descriptor.speed_override_raw
                || request.current_target.is_some_and(|target| {
                    target.handle != target_entity_id
                        || target.allocation_identity != target_allocation_identity
                })
            {
                return Err(Intro2FlyerAimError::ActionContractMismatch);
            }
            let table_speed = aim_runtime
                .row()
                .ok_or(Intro2FlyerAimError::RuntimeContractMismatch)?
                .speed_raw as i16;
            let solution = resolve_ballistic_target_launch(request, table_speed)
                .map_err(Intro2FlyerAimError::ShotMath)?;
            Ok(GenericEmitterResume::TargetLaunchResolved { phase, solution })
        }
        GenericEmitterAction::AppendTransient { request, .. } => {
            if !is_exact_transient_request(&aim_runtime, entity_id, request) {
                return Err(Intro2FlyerAimError::ActionContractMismatch);
            }
            let entity = manager
                .entity_mut(entity_id)
                .ok_or(Intro2FlyerAimError::EntityUnavailable)?;
            authenticate_intro2_flyer_aim_owner(entity, true)?;
            entity
                .intro2_flyer_aim_runtime
                .as_mut()
                .ok_or(Intro2FlyerAimError::RuntimeUnavailable)?
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
                return Err(Intro2FlyerAimError::RuntimeContractMismatch);
            }
            let entity = manager
                .entity_mut(entity_id)
                .ok_or(Intro2FlyerAimError::EntityUnavailable)?;
            authenticate_intro2_flyer_aim_owner(entity, true)?;
            let runtime = &mut entity
                .intro2_flyer_aim_runtime
                .as_mut()
                .ok_or(Intro2FlyerAimError::RuntimeUnavailable)?
                .emitter_runtime;
            if runtime.cadence_raw != write.cadence_before_raw {
                return Err(Intro2FlyerAimError::RuntimeContractMismatch);
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
                .ok_or(Intro2FlyerAimError::EntityUnavailable)?;
            authenticate_intro2_flyer_aim_owner(source, true)?;
            let expected_position = source.position_raw();
            if sound_id != u32::from(aim_runtime.sound())
                || source_position_raw != Some(expected_position)
                || frequency_multiplier_16_16 != 0x0001_0000
                || volume_multiplier_16_16 != 0x0001_0000
            {
                return Err(Intro2FlyerAimError::ActionContractMismatch);
            }
            world_fx.queue_networked_positional_sound_raw(aim_runtime.sound(), expected_position);
            Ok(GenericEmitterResume::Acknowledged { phase })
        }
        // A descriptor with `+0x14` alternates the A/B selector after each
        // shot (the craft's 20; the flyers author none).
        GenericEmitterAction::CommitAlternateSelector { write, .. }
            if descriptor.alternate_emitter_raw != 0 =>
        {
            let entity = manager
                .entity_mut(entity_id)
                .ok_or(Intro2FlyerAimError::EntityUnavailable)?;
            authenticate_intro2_flyer_aim_owner(entity, true)?;
            let runtime = &mut entity
                .intro2_flyer_aim_runtime
                .as_mut()
                .ok_or(Intro2FlyerAimError::RuntimeUnavailable)?
                .emitter_runtime;
            if runtime.emitter_selector != write.before_raw {
                return Err(Intro2FlyerAimError::RuntimeContractMismatch);
            }
            runtime.emitter_selector = write.after_raw;
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
            Err(Intro2FlyerAimError::ActionContractMismatch)
        }
    }
}

fn authenticate_intro2_flyer_aim_owner(
    entity: &Entity,
    expected_in_callback: bool,
) -> Result<(), Intro2FlyerAimError> {
    if !entity.active
        || FlyerEmitterProfile::from_entity(entity).is_none()
        || !intro2_flyer_pursuing_graph_authenticates(entity)
    {
        return Err(Intro2FlyerAimError::GraphMismatch);
    }
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary) else {
        return Err(Intro2FlyerAimError::VisitUnavailable);
    };
    let flags = entity
        .actor_tasks
        .wrapper_flags(task_id)
        .ok_or(Intro2FlyerAimError::VisitUnavailable)?;
    if !flags.alive || flags.in_callback != expected_in_callback {
        return Err(Intro2FlyerAimError::VisitUnavailable);
    }
    Ok(())
}

fn revalidate_active_owner(
    manager: &crate::entity::EntityManager,
    entity_id: u32,
    _task_visit: ActorTaskVisit,
) -> Result<(), Intro2FlyerAimError> {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .ok_or(Intro2FlyerAimError::EntityUnavailable)?;
    authenticate_intro2_flyer_aim_owner(entity, true)
}

fn flyer_aim_target_runtime_state(
    manager: &crate::entity::EntityManager,
    target_entity_id: u32,
) -> Result<AimAndFireTargetRuntimeState, Intro2FlyerAimError> {
    let Some(target) = manager
        .iter_all()
        .find(|entity| entity.id == target_entity_id && entity.active)
    else {
        return Ok(AimAndFireTargetRuntimeState::Missing);
    };
    let state = target.collision.state_flags_at_0x08;
    let dying = match state.masked(AIM_AND_FIRE_DYING_STATE_BIT) {
        RetailRuntimeValue::Known(value) => value,
        RetailRuntimeValue::Unresolved => return Err(Intro2FlyerAimError::TargetStateUnresolved),
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
    Err(Intro2FlyerAimError::TargetStateUnresolved)
}

fn source_for_math<'a>(
    manager: &'a crate::entity::EntityManager,
    entity_id: u32,
    source_handle: u32,
    _task_visit: ActorTaskVisit,
) -> Result<&'a Entity, Intro2FlyerAimError> {
    if source_handle != entity_id {
        return Err(Intro2FlyerAimError::ActionContractMismatch);
    }
    let source = manager
        .iter_all()
        .find(|entity| entity.id == source_handle && entity.active)
        .ok_or(Intro2FlyerAimError::EntityUnavailable)?;
    authenticate_intro2_flyer_aim_owner(source, true)?;
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

fn retained_aim_basis(entity: &Entity) -> Result<Type9BodyBasis, Intro2FlyerAimError> {
    // FUN_0041E4D0 reads retained lateral +0x0C..+0x14 and forward
    // +0x24..+0x2C; FUN_0041E930 reads that same forward column. Chase can
    // already have changed the angle words, but DCA0/E870 calls FUN_00413F70
    // only after the whole A800 traversal, including this same-pass Aim.
    match entity.physical_body_basis_q31() {
        RetailRuntimeValue::Known(basis) => Ok(basis),
        RetailRuntimeValue::Unresolved => Err(Intro2FlyerAimError::BodyBasisUnresolved),
    }
}

fn is_exact_runtime_shape(runtime: &Intro2FlyerAimRuntime) -> bool {
    let craft = runtime.profile == FlyerEmitterProfile::Craft;
    let base = runtime.profile.emitter();
    runtime.profile.accepts(runtime.projectile_descriptor)
        && runtime.emitter_runtime
            == (GenericEmitterRuntime {
                cadence_raw: runtime.emitter_runtime.cadence_raw,
                // The craft's method and sound are the row's; its selector
                // alternates.
                projectile_method: if craft {
                    runtime.emitter_runtime.projectile_method
                } else {
                    base.projectile_method
                },
                sound_id: if craft {
                    runtime.emitter_runtime.sound_id
                } else {
                    base.sound_id
                },
                emitter_selector: if craft {
                    runtime.emitter_runtime.emitter_selector
                } else {
                    base.emitter_selector
                },
                ..base
            })
}

fn is_exact_transient_request(
    runtime: &Intro2FlyerAimRuntime,
    source_entity_id: u32,
    request: GenericEmitterAppendRequest,
) -> bool {
    let shape = request.source_handle == source_entity_id
        && request.owner_handle == source_entity_id
        && request.emitter_selector <= 1
        && (runtime.projectile_descriptor.alternate_emitter_raw != 0
            || request.emitter_selector == 0);
    if request.auxiliary {
        return shape
            && runtime.projectile_descriptor.auxiliary_command != 0
            && request.projectile_method == GENERIC_PROJECTILE_AUXILIARY_METHOD
            && request.speed_field == GenericEmitterSpeedField::IndeterminateAuxiliaryStackWord;
    }
    shape
        && request.projectile_method == runtime.method()
        && runtime
            .launch_speed()
            .is_some_and(|speed| request.speed_field == GenericEmitterSpeedField::Explicit(speed))
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
