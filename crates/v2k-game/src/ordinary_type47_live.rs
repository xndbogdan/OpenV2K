//! Type-47 Aim-and-Fire coordinator with explicit native and replay custody.
//!
//! Fresh-New-Game construction retains the exact type-47/model-302 Sub-E
//! runtime for authored spawns 11/12/13. All three share that Section-12
//! record, the Guard/Wander birth graph, and the class-32 ADE0 Pursuing/Aim
//! lease. The accepted enemy-AI capture sampled only 11/12 in range; spawn 13
//! uses the same already-published coordinator rather than a second whitelist.
//! `20260831-215158` / `V200003.run` closes the first `FUN_0041FCB0` as
//! `full_reset` for seeds `0x2B/0x2C/0x2D`. Chase and Guard wander apply
//! that owner, then Sub-H and the terrain-only C -> A -> B tail.
//! Intro2 spawns 6/7/8 authenticate the same authored Sub-E payload and
//! emitter transaction through a distinct cohort adapter. Their targets are
//! captured type-9 allocations, while their `0x06/0x07/0x08` Sub-D pair stays
//! on the entity instead of this emitter sidecar.
//!
//! One admitted callback runs synchronously: no action or receipt escapes the
//! exclusive manager call. This lets the adapter mint a transaction-scoped
//! allocation token which is deliberately distinct from the reusable entity
//! handle. Cadence writes and transient appends commit to entity-owned state
//! before acknowledgement, sound 70 is queued after the successful append and
//! cadence suffix, and a later live-list drain models `FUN_00411400` before
//! materializing class 87. Both gameplay and Intro2 run that drain after the
//! specialized scheduler visit and before particle traversal. Fresh
//! construction publishes the birth Guard/Wander graph separately. The
//! specialized scheduler now adopts that graph and, after a live Guard
//! handoff, the pursuing bind. This
//! coordinator still ticks only after that bind authenticates. The
//! specialized scheduler now visits slot-0 Chase first and starts the
//! shared `FUN_00401430` frame machine. Fresh construction retains the
//! accepted constructor seeds `0x2B/0x2C/0x2D` and the authored Sub-D
//! descriptor (`classifier_flags` `0x13`). Those seeds are not Type-9's
//! `0x29..0x35` and do not authorize Type-9's first-query reset. Intro2
//! Type-47 full-reset (`0x06/0x07/0x08`) belongs to the cohort-specific
//! Guard owner rather than this shared Aim/emitter sidecar. Replay-level seeds
//! `0x3C/0x3D/0x3E` stay fail-closed.

use v2k_formats::collision::ProjectileEmitterDescriptor;

use crate::common_mover::sub_d::TYPE47_SUB_D;

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags},
    aim_and_fire::{
        aim_and_fire_after_unwind, AimAndFireFrameRequest, AimAndFireFrameResolution,
        AimAndFireTargetRuntimeState, AIM_AND_FIRE_DYING_STATE_BIT,
    },
    aim_and_fire_transaction::{
        AimAndFireEmitterAction, AimAndFireEmitterBlock, AimAndFireEmitterLease,
        AimAndFireEmitterMachine, AimAndFireEmitterPhase, AimAndFireEmitterPoll,
        AimAndFireEmitterProtocolError, AimAndFireEmitterResume, AimAndFireEmitterStartError,
        AimAndFireEmitterTransactionId,
    },
    entity::{Entity, EntityManager},
    entity_behavior::{
        audited_behavior_style, behavior_program, ActiveBehaviorStyle, BehaviorChoiceListSource,
        BehaviorDescriptorIdentity,
    },
    entity_collision_state::{
        CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
    generic_projectile_emitter::{
        GenericEmitterAction, GenericEmitterAppendRequest, GenericEmitterEntitySnapshot,
        GenericEmitterPhase, GenericEmitterResume, GenericEmitterRuntime, GenericEmitterSpeedField,
        GenericEmitterTransactionId,
    },
    hover::HoverBasis,
    ordinary_type47_shot_math::{
        drain_type47_transient_request, evaluate_type47_aim_error,
        evaluate_type47_forward_half_space, resolve_type47_method30_target_launch,
        Type47ShotMathError,
    },
    projectile_emitter::{
        FIRST_WORLD_SHOOTER_PROJECTILE_METHOD, FIRST_WORLD_SHOOTER_PROJECTILE_SPEED_RAW,
        FIRST_WORLD_SHOOTER_SOUND_ID,
    },
    world_fx::{DescriptorParticleRequest, ParticleEnvironment, ParticleOwnerAtBirth, WorldFx},
};

pub const FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES: [usize; 3] = [11, 12, 13];
/// Process `FUN_004203D0` seeds from accepted constructor transcripts
/// `20260730-034232` / `20260730-035135`, joined to spawn 11/12/13 by
/// authored X/Z. These are not Type-9 seeds `0x29..0x35`.
pub const FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS: [u8; 3] = [0x2B, 0x2C, 0x2D];
/// Exact normal-tier first-world Section-12 Sub-D descriptor. Probes match
/// the constructor-entry words; `classifier_flags` `0x13` is not Type-9
/// `0x17`.
pub const FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D: v2k_formats::collision::SubDSteeringDescriptor =
    TYPE47_SUB_D;
pub const FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID: usize = 302;
/// Exact normal-tier first-world Section-12 Sub-H record count.
pub const FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT: usize = 6;
/// Signed Section-12 Sub-C `+0x0C` value selected by Common-Dying.
pub const FRESH_LEVEL1_ORDINARY_TYPE47_SUB_C_EFFECT_RAW: i8 = 0;
const ORDINARY_TYPE47_ENTITY_TYPE: u32 = 47;
const TYPE47_PURSUING_BEHAVIOR_CLASS_ID: u32 = 32;
const TYPE47_PURSUING_STYLE_INDEX: u32 = 1;
const TYPE47_PLAYER_TARGET_ENTITY_TYPE: u32 = 46;
const INTRO2_TYPE47_TARGET_ENTITY_TYPE: u32 = 9;
const IMPACT_SUPPRESSION_STATE_BIT: u32 = 0x8000_0000;

/// Exact normal-tier first-world Sub-E payload for cumulative entity type 47.
pub const FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR: ProjectileEmitterDescriptor =
    ProjectileEmitterDescriptor {
        projectile_method: 30,
        random_interval_us: 750_000,
        spread_raw: 100,
        aim_threshold_raw: 16_000,
        speed_override_raw: 0,
        target_axis_tolerance_raw: 0x0600,
        sound_id: 70,
        raw_word_at_0x12: 150,
        alternate_emitter_raw: 0,
        stochastic_gate_mode: 0,
        auxiliary_command: 0,
        variable_bindings: [0; 4],
    };

/// Exact A/B/C/D/E/H/J component topology in the normal-tier type-47 record.
pub const FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY: CommonMoverComponentTopology =
    CommonMoverComponentTopology {
        sub_a: true,
        sub_b: true,
        sub_c: true,
        sub_d: true,
        sub_e: true,
        sub_f: false,
        sub_g: false,
        sub_h: true,
        sub_i: false,
        sub_j: true,
        sub_k: false,
        sub_l: false,
        sub_m: false,
        sub_n: false,
        sub_o: false,
    };

/// Exact `FUN_00424E30` Sub-E state after the all-zero allocation fill and
/// type-47 descriptor writes. No variable binding can install a joint or
/// manual/direct-mode input for this record.
pub const FRESH_LEVEL1_ORDINARY_TYPE47_EMITTER_RUNTIME: GenericEmitterRuntime =
    GenericEmitterRuntime {
        joint_bindings: [None; 2],
        projectile_method: FIRST_WORLD_SHOOTER_PROJECTILE_METHOD,
        emitter_selector: 0,
        sound_id: FIRST_WORLD_SHOOTER_SOUND_ID as u32,
        direct_mode: 0,
        remaining_time_raw: 0,
        manual_step_raw: 0,
        remaining_bursts_raw: 0,
        cadence_raw: 0,
        basis_adjustment_identity: None,
    };

/// Construction facts available without running a behavior callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshLevel1OrdinaryType47SpawnFacts {
    pub retail_first_world: bool,
    pub authored_spawn_index: usize,
    pub entity_type: u32,
    pub active_model: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshLevel1OrdinaryType47AdmissionBlock {
    NotRetailFirstWorld,
    UnauthenticatedSpawnIndex,
    WrongEntityType,
    WrongActiveModel,
    MissingExactTypeMetadata,
    UnresolvedProjectileDescriptor,
    AuthoredProjectileDescriptorAbsent,
    ProjectileDescriptorMismatch,
    UnresolvedComponentTopology,
    ComponentTopologyMismatch,
    UnresolvedSubDDescriptor,
    AuthoredSubDDescriptorAbsent,
    SubDDescriptorMismatch,
}

/// Private receipt proving that one fresh allocation belongs to the captured
/// ordinary Level-1 type-47 cohort.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshLevel1OrdinaryType47Admission {
    projectile_descriptor: ProjectileEmitterDescriptor,
    component_topology: CommonMoverComponentTopology,
    sub_d_stagger_seed: u8,
}

impl FreshLevel1OrdinaryType47Admission {
    /// Retain the exact mutable Sub-E state without publishing a task.
    pub fn aim_and_fire_runtime(self) -> OrdinaryType47AimAndFireRuntime {
        OrdinaryType47AimAndFireRuntime {
            pursuing_cohort: Type47AimAllocationPolicy::FreshLevel1,
            projectile_descriptor: self.projectile_descriptor,
            component_topology: self.component_topology,
            emitter_runtime: FRESH_LEVEL1_ORDINARY_TYPE47_EMITTER_RUNTIME,
            transient_shots: Vec::new(),
            sub_d_stagger_seed: self.sub_d_stagger_seed,
            sub_d_frame_owner: crate::common_mover::sub_d::level1_type47_first_query_owner_for_seed(
                self.sub_d_stagger_seed,
            ),
            sub_d_runtime: Some(crate::common_mover::sub_d::Type9SubDRuntime::from_constructor()),
        }
    }

    pub const fn sub_d_stagger_seed(self) -> u8 {
        self.sub_d_stagger_seed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Type47AimAllocationPolicy {
    FreshLevel1,
    Intro2,
    NativeConstruction,
}

impl Type47AimAllocationPolicy {
    const fn admits_target_type(self, entity_type: u32) -> bool {
        match self {
            Self::FreshLevel1 => entity_type == TYPE47_PLAYER_TARGET_ENTITY_TYPE,
            Self::Intro2 => entity_type == INTRO2_TYPE47_TARGET_ENTITY_TYPE,
            // The native Guard acquisition has already selected its target
            // using the actual common-axis filter. 07A10 and 15820 validate
            // that handle/state, not an Intro2 or first-world entity type.
            Self::NativeConstruction => true,
        }
    }
}

/// The source 0x2C FIFO record retains its +0E draw-stamped muzzle separately
/// from the generic emitter's append payload. None is the nonzero +2A branch
/// which 11400 reanchors to the actor centre before time correction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Type47TransientShot {
    request: GenericEmitterAppendRequest,
    draw_origin_raw: Option<[i16; 3]>,
}

/// Entity-owned exact Sub-E state and later-drained transient shot FIFO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrdinaryType47AimAndFireRuntime {
    pursuing_cohort: Type47AimAllocationPolicy,
    projectile_descriptor: ProjectileEmitterDescriptor,
    component_topology: CommonMoverComponentTopology,
    emitter_runtime: GenericEmitterRuntime,
    transient_shots: Vec<Type47TransientShot>,
    sub_d_stagger_seed: u8,
    /// Pending first `FUN_0041FCB0` reset for seeds `0x2B/0x2C/0x2D`.
    sub_d_frame_owner: Option<crate::common_mover::sub_d::Type9SubDFrameOwner>,
    sub_d_runtime: Option<crate::common_mover::sub_d::Type9SubDRuntime>,
}

impl OrdinaryType47AimAndFireRuntime {
    /// Native Sub-D custody remains on the shared actor allocation; Sub-E
    /// starts with the same Section-12 descriptor and 20370 runtime words.
    pub(crate) fn from_authenticated_native_metadata(
        metadata: &EntityTypeRuntimeMetadata,
        sub_d_stagger_seed: u8,
    ) -> Option<Self> {
        let mut runtime = Self::from_authenticated_intro2_metadata(metadata, sub_d_stagger_seed)?;
        runtime.pursuing_cohort = Type47AimAllocationPolicy::NativeConstruction;
        Some(runtime)
    }

    /// Retain the exact Type-47 Sub-E payload for one authenticated Intro2
    /// birth. Intro2 owns its `0x06/0x07/0x08` Sub-D query state separately;
    /// this sidecar therefore carries only the shared emitter state.
    pub(crate) fn from_authenticated_intro2_metadata(
        metadata: &EntityTypeRuntimeMetadata,
        sub_d_stagger_seed: u8,
    ) -> Option<Self> {
        let RetailRuntimeValue::Known(Some(projectile_descriptor)) =
            metadata.projectile_emitter_descriptor
        else {
            return None;
        };
        let RetailRuntimeValue::Known(component_topology) = metadata.common_mover_topology else {
            return None;
        };
        if projectile_descriptor != FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR
            || component_topology != FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY
        {
            return None;
        }
        Some(Self {
            pursuing_cohort: Type47AimAllocationPolicy::Intro2,
            projectile_descriptor,
            component_topology,
            emitter_runtime: FRESH_LEVEL1_ORDINARY_TYPE47_EMITTER_RUNTIME,
            transient_shots: Vec::new(),
            sub_d_stagger_seed,
            sub_d_frame_owner: None,
            sub_d_runtime: None,
        })
    }

    pub const fn projectile_descriptor(&self) -> ProjectileEmitterDescriptor {
        self.projectile_descriptor
    }

    pub const fn component_topology(&self) -> CommonMoverComponentTopology {
        self.component_topology
    }

    pub const fn emitter_runtime(&self) -> GenericEmitterRuntime {
        self.emitter_runtime
    }

    pub fn queued_shot_count(&self) -> usize {
        self.transient_shots.len()
    }

    /// 424F20 refreshes every matching emitter node after resolving its source
    /// slot, then clears +2A. The retained runtime authenticates the descriptor
    /// and every append before any mutation; this does not borrow an AI target.
    pub fn stamp_draw_emitter_origin(
        &mut self,
        source_entity_id: u32,
        point: crate::actor_emitter_external_frame::ActorEmitterExternalFramePoint,
    ) -> Result<usize, OrdinaryType47LiveError> {
        if !is_exact_runtime_shape(self) {
            return Err(OrdinaryType47LiveError::AimAndFireRuntimeContractMismatch);
        }
        let expected_slot = match point.emitter_index {
            0 => self.projectile_descriptor.raw_word_at_0x12,
            1 => self.projectile_descriptor.alternate_emitter_raw,
            _ => return Err(OrdinaryType47LiveError::DrawEmitterOriginContractMismatch),
        };
        if point.emitter_index
            >= u16::from(
                crate::actor_emitter_external_frame::emitter_external_selector_count(
                    self.projectile_descriptor,
                ),
            )
            || point.source_slot != expected_slot
            || point.position_words != point.position_raw.map(|value| value as i16)
        {
            return Err(OrdinaryType47LiveError::DrawEmitterOriginContractMismatch);
        }
        for (index, shot) in self.transient_shots.iter().enumerate() {
            if !is_exact_transient_request(source_entity_id, shot.request) {
                return Err(OrdinaryType47LiveError::MalformedTransientRequest { index });
            }
        }
        let mut stamped = 0;
        for shot in &mut self.transient_shots {
            if shot.request.emitter_selector == point.emitter_index {
                shot.draw_origin_raw = Some(point.position_words);
                stamped += 1;
            }
        }
        Ok(stamped)
    }

    pub(crate) fn authenticates_intro2_cohort(self: &Self, expected_seed: u8) -> bool {
        self.pursuing_cohort == Type47AimAllocationPolicy::Intro2
            && self.sub_d_stagger_seed == expected_seed
            && self.sub_d_frame_owner.is_none()
            && self.sub_d_runtime.is_none()
            && is_exact_runtime_shape(self)
    }

    pub(crate) fn authenticates_native_cohort(&self, expected_seed: u8) -> bool {
        self.pursuing_cohort == Type47AimAllocationPolicy::NativeConstruction
            && self.sub_d_stagger_seed == expected_seed
            && self.sub_d_frame_owner.is_none()
            && self.sub_d_runtime.is_none()
            && is_exact_runtime_shape(self)
    }

    /// Constructor `+0x3A` seed. `20260831-215158` joins `0x2B/0x2C/0x2D` to
    /// caller `0x0041F7A8` / frame `0x00401602` / `full_reset`.
    pub const fn sub_d_stagger_seed(&self) -> u8 {
        self.sub_d_stagger_seed
    }

    pub fn sub_d_frame_owner(&self) -> Option<&crate::common_mover::sub_d::Type9SubDFrameOwner> {
        self.sub_d_frame_owner.as_ref()
    }

    pub fn sub_d_pair_mut(
        &mut self,
    ) -> (
        Option<&mut crate::common_mover::sub_d::Type9SubDFrameOwner>,
        Option<&mut crate::common_mover::sub_d::Type9SubDRuntime>,
    ) {
        (self.sub_d_frame_owner.as_mut(), self.sub_d_runtime.as_mut())
    }
}

/// Admit only the exact captured fresh-Level-1 cohort and authored Sub-E data.
///
/// Success authorizes retaining descriptor/topology and exact zero-filled
/// Sub-E state. Firing still requires the live Pursuing/Aim/Chase binding
/// below; that bind now covers the same three authored spawns.
pub fn admit_fresh_level1_ordinary_type47(
    facts: FreshLevel1OrdinaryType47SpawnFacts,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<FreshLevel1OrdinaryType47Admission, FreshLevel1OrdinaryType47AdmissionBlock> {
    if !facts.retail_first_world {
        return Err(FreshLevel1OrdinaryType47AdmissionBlock::NotRetailFirstWorld);
    }
    if !FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES.contains(&facts.authored_spawn_index) {
        return Err(FreshLevel1OrdinaryType47AdmissionBlock::UnauthenticatedSpawnIndex);
    }
    if facts.entity_type != ORDINARY_TYPE47_ENTITY_TYPE {
        return Err(FreshLevel1OrdinaryType47AdmissionBlock::WrongEntityType);
    }
    if facts.active_model != Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID) {
        return Err(FreshLevel1OrdinaryType47AdmissionBlock::WrongActiveModel);
    }
    let metadata =
        metadata.ok_or(FreshLevel1OrdinaryType47AdmissionBlock::MissingExactTypeMetadata)?;
    let projectile_descriptor = match metadata.projectile_emitter_descriptor {
        RetailRuntimeValue::Unresolved => {
            return Err(FreshLevel1OrdinaryType47AdmissionBlock::UnresolvedProjectileDescriptor);
        }
        RetailRuntimeValue::Known(None) => {
            return Err(
                FreshLevel1OrdinaryType47AdmissionBlock::AuthoredProjectileDescriptorAbsent,
            );
        }
        RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
    };
    if projectile_descriptor != FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR {
        return Err(FreshLevel1OrdinaryType47AdmissionBlock::ProjectileDescriptorMismatch);
    }
    let component_topology = match metadata.common_mover_topology {
        RetailRuntimeValue::Known(topology) => topology,
        RetailRuntimeValue::Unresolved => {
            return Err(FreshLevel1OrdinaryType47AdmissionBlock::UnresolvedComponentTopology);
        }
    };
    if component_topology != FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY {
        return Err(FreshLevel1OrdinaryType47AdmissionBlock::ComponentTopologyMismatch);
    }
    let sub_d_descriptor = match metadata.sub_d_steering_descriptor {
        RetailRuntimeValue::Unresolved => {
            return Err(FreshLevel1OrdinaryType47AdmissionBlock::UnresolvedSubDDescriptor);
        }
        RetailRuntimeValue::Known(None) => {
            return Err(FreshLevel1OrdinaryType47AdmissionBlock::AuthoredSubDDescriptorAbsent);
        }
        RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
    };
    if sub_d_descriptor != FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D {
        return Err(FreshLevel1OrdinaryType47AdmissionBlock::SubDDescriptorMismatch);
    }
    let sub_d_stagger_seed = FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES
        .iter()
        .position(|&index| index == facts.authored_spawn_index)
        .map(|ordinal| FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS[ordinal])
        .expect("spawn index already authenticated");

    Ok(FreshLevel1OrdinaryType47Admission {
        projectile_descriptor,
        component_topology,
        sub_d_stagger_seed,
    })
}

/// Revalidated identity of one already-published captured Pursuing/Aim task.
///
/// This is not a detached machine lease. It can be reused for later frames only
/// while the exact entity allocation, behavior context, and task wrappers still
/// match; every live boundary checks those facts again before side effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType47AimAndFireOwner {
    source_entity_id: u32,
    task_visit: ActorTaskVisit,
    target_entity_id: u32,
    pursuing_cohort: Type47AimAllocationPolicy,
}

impl OrdinaryType47AimAndFireOwner {
    pub const fn source_entity_id(self) -> u32 {
        self.source_entity_id
    }

    pub const fn task_visit(self) -> ActorTaskVisit {
        self.task_visit
    }

    pub const fn target_entity_id(self) -> u32 {
        self.target_entity_id
    }
}

/// One synchronous callback identity. Callers must not reuse either
/// transaction id while another machine bearing that id remains live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType47AimAndFireFrameRequest {
    pub parent_transaction_id: AimAndFireEmitterTransactionId,
    pub child_transaction_id: GenericEmitterTransactionId,
    pub elapsed_micros: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType47AimAndFireTickOutcome {
    pub resolution: AimAndFireFrameResolution,
    pub queued_shots_added: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrdinaryType47ShotDrainOutcome {
    /// Requests consumed from the entity-owned FIFO, including allocator
    /// rejections just as retail frees every drained transient record.
    pub consumed_requests: usize,
    pub materialized_particle_slots: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType47LiveError {
    SourceEntityMissing,
    SourceInactive,
    WrongSourceEntityType,
    UncapturedPursuingSpawn,
    WrongActiveModel,
    MissingAimAndFireRuntime,
    AimAndFireRuntimeContractMismatch,
    BehaviorContextUnresolved,
    BehaviorContextContractMismatch,
    MissingPrimaryChaseTask,
    SecondaryTaskUnexpectedlyPresent,
    MissingTertiaryAimTask,
    TaskLeaseMismatch,
    TaskWrapperStateMismatch,
    ChaseTaskContractMismatch,
    AimTaskContractMismatch,
    TargetStateUnresolved,
    TargetAllocationTypeMismatch { actual: u32 },
    ParentAndChildTransactionIdsMustDiffer,
    MachineStart(AimAndFireEmitterStartError),
    MachineProtocol(AimAndFireEmitterProtocolError),
    MachineBlocked(AimAndFireEmitterBlock),
    MachineUnexpectedlyAwaiting(AimAndFireEmitterPhase),
    ParentActionContractMismatch(AimAndFireEmitterPhase),
    GenericActionContractMismatch(GenericEmitterPhase),
    RuntimeCommitMismatch(GenericEmitterPhase),
    ShotMath(Type47ShotMathError),
    MalformedTransientRequest { index: usize },
    DrawEmitterOriginContractMismatch,
    ImpactSuppressionStateUnresolved,
    CallbackWrapperDidNotSurvive,
}

/// Bind a fresh Level-1 ordinary shooter whose live graph is Guard Location
/// variant 1 with concurrent Primary Chase and Tertiary Aim-and-Fire targeting
/// the player. Authored spawns 11/12/13 share that contract.
pub fn bind_fresh_level1_ordinary_type47_aim_and_fire(
    entity: &Entity,
) -> Result<OrdinaryType47AimAndFireOwner, OrdinaryType47LiveError> {
    bind_captured_type47_aim_and_fire(entity, Type47AimAllocationPolicy::FreshLevel1)
}

/// Bind the shared native class-32 graph, or the explicit Intro2 replay graph.
/// A native receipt is authenticated before any captured identity fallback.
pub(crate) fn bind_intro2_type47_aim_and_fire(
    entity: &Entity,
) -> Result<OrdinaryType47AimAndFireOwner, OrdinaryType47LiveError> {
    let cohort = if entity.native_type47_construction.is_some() {
        Type47AimAllocationPolicy::NativeConstruction
    } else {
        Type47AimAllocationPolicy::Intro2
    };
    bind_captured_type47_aim_and_fire(entity, cohort)
}

fn bind_captured_type47_aim_and_fire(
    entity: &Entity,
    pursuing_cohort: Type47AimAllocationPolicy,
) -> Result<OrdinaryType47AimAndFireOwner, OrdinaryType47LiveError> {
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Tertiary)
        .ok_or(OrdinaryType47LiveError::MissingTertiaryAimTask)?;
    let task_visit = ActorTaskVisit {
        slot: ActorTaskSlot::Tertiary,
        task_id,
    };
    let target_entity_id = authenticate_live_owner(entity, task_visit, pursuing_cohort, false)?;
    Ok(OrdinaryType47AimAndFireOwner {
        source_entity_id: entity.id,
        task_visit,
        target_entity_id,
        pursuing_cohort,
    })
}

/// Run one exact already-published Aim-and-Fire callback synchronously.
///
/// Elapsed accounting commits before target validation. Every child action is
/// handled before this function returns, so the transaction-scoped allocation
/// identities never escape and cannot be mistaken for reusable entity handles.
/// The resulting class-87 request stays queued until the explicit later drain.
pub fn tick_fresh_level1_ordinary_type47_aim_and_fire(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    owner: OrdinaryType47AimAndFireOwner,
    frame: OrdinaryType47AimAndFireFrameRequest,
) -> Result<OrdinaryType47AimAndFireTickOutcome, OrdinaryType47LiveError> {
    if owner.pursuing_cohort != Type47AimAllocationPolicy::FreshLevel1 {
        return Err(OrdinaryType47LiveError::UncapturedPursuingSpawn);
    }
    tick_captured_type47_aim_and_fire(manager, world_fx, owner, frame)
}

pub(crate) fn tick_intro2_type47_aim_and_fire(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    owner: OrdinaryType47AimAndFireOwner,
    frame: OrdinaryType47AimAndFireFrameRequest,
) -> Result<OrdinaryType47AimAndFireTickOutcome, OrdinaryType47LiveError> {
    if !matches!(
        owner.pursuing_cohort,
        Type47AimAllocationPolicy::Intro2 | Type47AimAllocationPolicy::NativeConstruction
    ) {
        return Err(OrdinaryType47LiveError::UncapturedPursuingSpawn);
    }
    tick_captured_type47_aim_and_fire(manager, world_fx, owner, frame)
}

fn tick_captured_type47_aim_and_fire(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    owner: OrdinaryType47AimAndFireOwner,
    frame: OrdinaryType47AimAndFireFrameRequest,
) -> Result<OrdinaryType47AimAndFireTickOutcome, OrdinaryType47LiveError> {
    if owner.pursuing_cohort == Type47AimAllocationPolicy::NativeConstruction
        && !crate::shared_type47::type47_manager_allocation_authenticates(
            manager,
            owner.source_entity_id,
        )
    {
        return Err(OrdinaryType47LiveError::UncapturedPursuingSpawn);
    }
    if frame.parent_transaction_id.get() == frame.child_transaction_id.get() {
        return Err(OrdinaryType47LiveError::ParentAndChildTransactionIdsMustDiffer);
    }

    let source_allocation_identity = transaction_scoped_allocation_identity(
        frame.parent_transaction_id.get(),
        owner.source_entity_id,
        0x5352_4345_5F34_3701,
    );
    let target_allocation_identity = transaction_scoped_allocation_identity(
        frame.child_transaction_id.get(),
        owner.target_entity_id,
        0x5441_5247_4554_4702,
    );

    let (prefix, private_state, descriptor, emitter_runtime, owner_position_raw, queued_before) = {
        let entity = manager
            .ordinary_type47_entity_mut(owner.source_entity_id)
            .ok_or(OrdinaryType47LiveError::SourceEntityMissing)?;
        let target_entity_id =
            authenticate_live_owner(entity, owner.task_visit, owner.pursuing_cohort, false)?;
        if target_entity_id != owner.target_entity_id {
            return Err(OrdinaryType47LiveError::TaskLeaseMismatch);
        }
        let runtime = entity
            .ordinary_type47_aim_and_fire_runtime
            .as_ref()
            .expect("authenticated type-47 owner retains its sidecar");
        let descriptor = runtime.projectile_descriptor;
        let emitter_runtime = runtime.emitter_runtime;
        let queued_before = runtime.transient_shots.len();
        let owner_position_raw = entity.position_raw();
        let (prefix, private_state) = entity
            .actor_tasks
            .begin_exact_visit_with(owner.task_visit, |task| {
                let ActorTaskRuntime::AimAndFire(task) = task else {
                    unreachable!("authenticated tertiary task changed family in one borrow")
                };
                let private_state = task.private_state();
                (task.before_callback(frame.elapsed_micros), private_state)
            })
            .ok_or(OrdinaryType47LiveError::TaskWrapperStateMismatch)?;
        (
            prefix,
            private_state,
            descriptor,
            emitter_runtime,
            owner_position_raw,
            queued_before,
        )
    };

    let callback = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let target_state =
            aim_target_runtime_state(manager, owner.target_entity_id, owner.pursuing_cohort)?;
        let lease = AimAndFireEmitterLease {
            owner_entity_id: owner.source_entity_id,
            task_visit: owner.task_visit,
        };
        let mut machine = AimAndFireEmitterMachine::start(
            frame.parent_transaction_id,
            frame.child_transaction_id,
            lease,
            prefix,
            private_state,
            AimAndFireFrameRequest {
                owner_entity_id: owner.source_entity_id,
                elapsed_micros: frame.elapsed_micros,
                scheduler_mode: 0,
                target_state,
                owner_sound_position_raw: Some(owner_position_raw),
                sub_e_descriptor: Some(descriptor),
                emitter_runtime,
            },
        )
        .map_err(OrdinaryType47LiveError::MachineStart)?;

        drive_type47_machine(
            &mut machine,
            manager,
            world_fx,
            owner,
            frame.child_transaction_id,
            source_allocation_identity,
            target_allocation_identity,
        )
    }));

    let survived = manager
        .ordinary_type47_entity_mut(owner.source_entity_id)
        .is_some_and(|entity| entity.actor_tasks.finish_exact_visit(owner.task_visit));
    let completion = match callback {
        Ok(result) => result?,
        Err(payload) => std::panic::resume_unwind(payload),
    };
    if !survived {
        return Err(OrdinaryType47LiveError::CallbackWrapperDidNotSurvive);
    }

    let entity = manager
        .iter_all()
        .find(|entity| entity.id == owner.source_entity_id)
        .ok_or(OrdinaryType47LiveError::SourceEntityMissing)?;
    authenticate_live_owner(entity, owner.task_visit, owner.pursuing_cohort, false)?;
    let queued_after = entity
        .ordinary_type47_aim_and_fire_runtime
        .as_ref()
        .expect("authenticated type-47 owner retains its sidecar")
        .transient_shots
        .len();
    Ok(OrdinaryType47AimAndFireTickOutcome {
        resolution: aim_and_fire_after_unwind(
            completion.committed_prefix,
            completion.callback_result,
        ),
        queued_shots_added: queued_after.saturating_sub(queued_before),
    })
}

/// Drain the entity-owned transient FIFO in insertion order and materialize
/// the supported class-87 slice. The queue is consumed even when the fixed
/// particle pool or the request's impact-suppression flag rejects a birth.
/// Current behavior style, task wrappers, active model, and spawn admission are
/// intentionally irrelevant: a previously appended retail command survives
/// those later presentation/behavior changes while its source allocation does.
/// Despite the historical function name, this drain is cohort-agnostic after
/// an authenticated Level-1 or Intro2 owner has queued the exact request.
pub fn drain_fresh_level1_ordinary_type47_shots(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    source_entity_id: u32,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Result<OrdinaryType47ShotDrainOutcome, OrdinaryType47LiveError> {
    if manager
        .iter_all()
        .find(|entity| entity.id == source_entity_id)
        .is_some_and(|entity| entity.native_type47_construction.is_some())
        && !crate::shared_type47::type47_manager_allocation_authenticates(manager, source_entity_id)
    {
        return Err(OrdinaryType47LiveError::UncapturedPursuingSpawn);
    }
    let (solutions, suppresses_impact_damage) = {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == source_entity_id)
            .ok_or(OrdinaryType47LiveError::SourceEntityMissing)?;
        if entity.entity_type != ORDINARY_TYPE47_ENTITY_TYPE {
            return Err(OrdinaryType47LiveError::WrongSourceEntityType);
        }
        let runtime = entity
            .ordinary_type47_aim_and_fire_runtime
            .as_ref()
            .ok_or(OrdinaryType47LiveError::MissingAimAndFireRuntime)?;
        if !is_exact_runtime_shape(runtime) {
            return Err(OrdinaryType47LiveError::AimAndFireRuntimeContractMismatch);
        }
        if runtime.transient_shots.is_empty() {
            return Ok(OrdinaryType47ShotDrainOutcome {
                consumed_requests: 0,
                materialized_particle_slots: Vec::new(),
            });
        }
        let suppression = match entity
            .collision
            .state_flags_at_0x08
            .masked(IMPACT_SUPPRESSION_STATE_BIT)
        {
            RetailRuntimeValue::Known(value) => value != 0,
            RetailRuntimeValue::Unresolved => {
                return Err(OrdinaryType47LiveError::ImpactSuppressionStateUnresolved);
            }
        };
        let mut solutions = Vec::with_capacity(runtime.transient_shots.len());
        for (index, shot) in runtime.transient_shots.iter().copied().enumerate() {
            if !is_exact_transient_request(source_entity_id, shot.request) {
                return Err(OrdinaryType47LiveError::MalformedTransientRequest { index });
            }
            solutions.push(
                drain_type47_transient_request(
                    shot.request,
                    shot.draw_origin_raw
                        .unwrap_or_else(|| entity.position_raw()),
                    entity.velocity_raw(),
                )
                .map_err(OrdinaryType47LiveError::ShotMath)?,
            );
        }
        (solutions, suppression)
    };

    let queued = {
        let entity = manager
            .ordinary_type47_entity_mut(source_entity_id)
            .ok_or(OrdinaryType47LiveError::SourceEntityMissing)?;
        let runtime = entity
            .ordinary_type47_aim_and_fire_runtime
            .as_mut()
            .ok_or(OrdinaryType47LiveError::MissingAimAndFireRuntime)?;
        std::mem::take(&mut runtime.transient_shots)
    };
    debug_assert_eq!(queued.len(), solutions.len());

    let mut materialized_particle_slots = Vec::with_capacity(solutions.len());
    for solution in solutions {
        if let Some(slot) = world_fx.materialize_descriptor_particle_request(
            DescriptorParticleRequest {
                source_class: 87,
                position_raw: solution.position_raw,
                velocity_raw: solution.velocity_raw,
                owner: Some(ParticleOwnerAtBirth {
                    entity_id: source_entity_id,
                    entity_type: ORDINARY_TYPE47_ENTITY_TYPE as u8,
                }),
                suppresses_impact_damage,
            },
            environment,
            retail_tick,
        ) {
            materialized_particle_slots.push(slot);
        }
    }
    Ok(OrdinaryType47ShotDrainOutcome {
        consumed_requests: queued.len(),
        materialized_particle_slots,
    })
}

/// Presentation-phase `FUN_00411400` drain of every retained Type-47 FIFO.
///
/// Aim appends into the entity-owned queue during the specialized scheduler
/// visit. Retail materializes those transients later in presentation, after
/// particle traversal, in live-list order. Empty queues are skipped. A fail-closed
/// owner leaves its queue untouched and does not stop later siblings.
pub fn drain_live_ordinary_type47_shot_queues(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Vec<(
    u32,
    Result<OrdinaryType47ShotDrainOutcome, OrdinaryType47LiveError>,
)> {
    let source_ids = manager
        .iter_all()
        .filter(|entity| {
            entity
                .ordinary_type47_aim_and_fire_runtime
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
                drain_fresh_level1_ordinary_type47_shots(
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

fn drive_type47_machine(
    machine: &mut AimAndFireEmitterMachine,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    owner: OrdinaryType47AimAndFireOwner,
    child_transaction_id: GenericEmitterTransactionId,
    source_allocation_identity: u64,
    target_allocation_identity: u64,
) -> Result<crate::aim_and_fire_transaction::AimAndFireEmitterCompletion, OrdinaryType47LiveError> {
    loop {
        match machine.poll() {
            AimAndFireEmitterPoll::Action(issued) => {
                let action_phase = issued.action.phase();
                let action_lease = issued.action.lease();
                if action_lease
                    != (AimAndFireEmitterLease {
                        owner_entity_id: owner.source_entity_id,
                        task_visit: owner.task_visit,
                    })
                {
                    return Err(OrdinaryType47LiveError::TaskLeaseMismatch);
                }
                revalidate_active_owner(manager, owner)?;
                let response = match issued.action {
                    AimAndFireEmitterAction::GenericEmitter {
                        phase,
                        child_transaction_id: actual_child,
                        action,
                        ..
                    } if actual_child == child_transaction_id
                        && phase == AimAndFireEmitterPhase::GenericEmitter(action.phase()) =>
                    {
                        let response = handle_type47_generic_action(
                            manager,
                            world_fx,
                            owner,
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
                    _ => {
                        return Err(OrdinaryType47LiveError::ParentActionContractMismatch(
                            action_phase,
                        ));
                    }
                };
                machine
                    .resume(issued.receipt, response)
                    .map_err(|failure| OrdinaryType47LiveError::MachineProtocol(failure.error))?;
            }
            AimAndFireEmitterPoll::Awaiting(phase) => {
                return Err(OrdinaryType47LiveError::MachineUnexpectedlyAwaiting(phase));
            }
            AimAndFireEmitterPoll::Blocked(block) => {
                return Err(OrdinaryType47LiveError::MachineBlocked(block));
            }
            AimAndFireEmitterPoll::Complete(completion) => return Ok(completion),
        }
    }
}

fn handle_type47_generic_action(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    owner: OrdinaryType47AimAndFireOwner,
    action: GenericEmitterAction,
    source_allocation_identity: u64,
    target_allocation_identity: u64,
) -> Result<GenericEmitterResume, OrdinaryType47LiveError> {
    let phase = action.phase();
    match action {
        GenericEmitterAction::LookupEntity { handle, .. } => {
            let entity = if handle == owner.source_entity_id {
                manager
                    .iter_all()
                    .find(|entity| entity.id == handle && entity.active)
                    .map(|entity| entity_snapshot(entity, source_allocation_identity))
            } else if handle == owner.target_entity_id {
                let target = manager
                    .iter_all()
                    .find(|entity| entity.id == handle && entity.active);
                if let Some(target) = target {
                    if !owner.pursuing_cohort.admits_target_type(target.entity_type) {
                        return Err(OrdinaryType47LiveError::TargetAllocationTypeMismatch {
                            actual: target.entity_type,
                        });
                    }
                }
                target.map(|entity| entity_snapshot(entity, target_allocation_identity))
            } else {
                return Err(OrdinaryType47LiveError::GenericActionContractMismatch(
                    phase,
                ));
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
                .ordinary_type47_entity_mut(owner.source_entity_id)
                .ok_or(OrdinaryType47LiveError::SourceEntityMissing)?;
            authenticate_live_owner(entity, owner.task_visit, owner.pursuing_cohort, true)?;
            let runtime = &mut entity
                .ordinary_type47_aim_and_fire_runtime
                .as_mut()
                .expect("authenticated type-47 owner retains its sidecar")
                .emitter_runtime;
            if runtime.cadence_raw != write.before_raw || write.after_raw != expected_after {
                return Err(OrdinaryType47LiveError::RuntimeCommitMismatch(phase));
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
            if source_handle != owner.source_entity_id
                || allocation_identity != source_allocation_identity
                || !matches!(
                    point,
                    crate::generic_projectile_emitter::GenericEmitterSourceRefreshPoint::TargetAxisGate
                        | crate::generic_projectile_emitter::GenericEmitterSourceRefreshPoint::TargetShot
                )
            {
                return Err(OrdinaryType47LiveError::GenericActionContractMismatch(phase));
            }
            let entity = manager
                .iter_all()
                .find(|entity| entity.id == owner.source_entity_id && entity.active)
                .ok_or(OrdinaryType47LiveError::SourceEntityMissing)?;
            authenticate_live_owner(entity, owner.task_visit, owner.pursuing_cohort, true)?;
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
            let source = source_for_math(manager, owner, source_handle)?;
            let rotation = source.rotation_heading_pitch_roll_raw();
            let basis = HoverBasis::from_angle_words(rotation[0], rotation[1], rotation[2]);
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
            let source = source_for_math(manager, owner, source_handle)?;
            let rotation = source.rotation_heading_pitch_roll_raw();
            let basis = HoverBasis::from_angle_words(rotation[0], rotation[1], rotation[2]);
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
            if request.cached_source.handle != owner.source_entity_id
                || request.cached_source.allocation_identity != source_allocation_identity
                || request.projectile_method != FIRST_WORLD_SHOOTER_PROJECTILE_METHOD
                || request.speed_override_raw != 0
                || request.current_target.is_some_and(|target| {
                    target.handle != owner.target_entity_id
                        || target.allocation_identity != target_allocation_identity
                })
            {
                return Err(OrdinaryType47LiveError::GenericActionContractMismatch(
                    phase,
                ));
            }
            let solution = resolve_type47_method30_target_launch(request)
                .map_err(OrdinaryType47LiveError::ShotMath)?;
            Ok(GenericEmitterResume::TargetLaunchResolved { phase, solution })
        }
        GenericEmitterAction::AppendTransient { request, .. } => {
            if !is_exact_transient_request(owner.source_entity_id, request) {
                return Err(OrdinaryType47LiveError::GenericActionContractMismatch(
                    phase,
                ));
            }
            let entity = manager
                .ordinary_type47_entity_mut(owner.source_entity_id)
                .ok_or(OrdinaryType47LiveError::SourceEntityMissing)?;
            authenticate_live_owner(entity, owner.task_visit, owner.pursuing_cohort, true)?;
            entity
                .ordinary_type47_aim_and_fire_runtime
                .as_mut()
                .expect("authenticated type-47 owner retains its sidecar")
                .transient_shots
                .push(Type47TransientShot {
                    request,
                    draw_origin_raw: None,
                });
            Ok(GenericEmitterResume::AppendReturned {
                phase,
                result_raw: 0,
            })
        }
        GenericEmitterAction::CommitTargetCadence { write, .. } => {
            if write.interval_us
                != FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR.random_interval_us
                || write.cadence_after_raw
                    != write
                        .cadence_before_raw
                        .wrapping_add(write.interval_us as i32)
                || write.time_offset_after_raw
                    != write
                        .time_offset_before_raw
                        .wrapping_add(write.interval_us as i32)
            {
                return Err(OrdinaryType47LiveError::RuntimeCommitMismatch(phase));
            }
            let entity = manager
                .ordinary_type47_entity_mut(owner.source_entity_id)
                .ok_or(OrdinaryType47LiveError::SourceEntityMissing)?;
            authenticate_live_owner(entity, owner.task_visit, owner.pursuing_cohort, true)?;
            let runtime = &mut entity
                .ordinary_type47_aim_and_fire_runtime
                .as_mut()
                .expect("authenticated type-47 owner retains its sidecar")
                .emitter_runtime;
            if runtime.cadence_raw != write.cadence_before_raw {
                return Err(OrdinaryType47LiveError::RuntimeCommitMismatch(phase));
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
                .find(|entity| entity.id == owner.source_entity_id && entity.active)
                .ok_or(OrdinaryType47LiveError::SourceEntityMissing)?;
            authenticate_live_owner(source, owner.task_visit, owner.pursuing_cohort, true)?;
            let expected_position = source.position_raw();
            if sound_id != u32::from(FIRST_WORLD_SHOOTER_SOUND_ID)
                || source_position_raw != Some(expected_position)
                || frequency_multiplier_16_16 != 0x0001_0000
                || volume_multiplier_16_16 != 0x0001_0000
            {
                return Err(OrdinaryType47LiveError::GenericActionContractMismatch(
                    phase,
                ));
            }
            world_fx
                .queue_fixed_positional_sound_raw(FIRST_WORLD_SHOOTER_SOUND_ID, expected_position);
            Ok(GenericEmitterResume::Acknowledged { phase })
        }
        GenericEmitterAction::EmitMethod31Rejection { .. }
        | GenericEmitterAction::CommitMethod31Suppression { .. }
        | GenericEmitterAction::ReadManualAngleBinding { .. }
        | GenericEmitterAction::ResolveManualBasis { .. }
        | GenericEmitterAction::CommitGunJoint { .. }
        | GenericEmitterAction::ResolveManualLaunch { .. }
        | GenericEmitterAction::CommitAlternateSelector { .. }
        | GenericEmitterAction::CommitManualCounters { .. } => Err(
            OrdinaryType47LiveError::GenericActionContractMismatch(phase),
        ),
    }
}

fn authenticate_live_owner(
    entity: &Entity,
    task_visit: ActorTaskVisit,
    pursuing_cohort: Type47AimAllocationPolicy,
    expected_in_callback: bool,
) -> Result<u32, OrdinaryType47LiveError> {
    if !entity.active {
        return Err(OrdinaryType47LiveError::SourceInactive);
    }
    if entity.entity_type != ORDINARY_TYPE47_ENTITY_TYPE {
        return Err(OrdinaryType47LiveError::WrongSourceEntityType);
    }
    let native = entity.native_type47_construction.as_ref();
    // A present native receipt cannot fall back to a captured spawn match.
    if native.is_some() && pursuing_cohort != Type47AimAllocationPolicy::NativeConstruction {
        return Err(OrdinaryType47LiveError::UncapturedPursuingSpawn);
    }
    let spawn_admitted = entity
        .authored_spawn_index
        .is_some_and(|index| match pursuing_cohort {
            Type47AimAllocationPolicy::FreshLevel1 => {
                FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES.contains(&index)
            }
            Type47AimAllocationPolicy::Intro2 => {
                crate::intro2_type47_live::INTRO2_TYPE47_SPAWN_INDICES.contains(&index)
            }
            Type47AimAllocationPolicy::NativeConstruction => {
                native.is_some_and(|receipt| receipt.entity_authenticates(entity))
            }
        });
    if !spawn_admitted {
        return Err(OrdinaryType47LiveError::UncapturedPursuingSpawn);
    }
    if pursuing_cohort != Type47AimAllocationPolicy::NativeConstruction
        && entity.model_index != Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID)
    {
        return Err(OrdinaryType47LiveError::WrongActiveModel);
    }
    let runtime = entity
        .ordinary_type47_aim_and_fire_runtime
        .as_ref()
        .ok_or(OrdinaryType47LiveError::MissingAimAndFireRuntime)?;
    if !is_exact_runtime_shape(runtime) {
        return Err(OrdinaryType47LiveError::AimAndFireRuntimeContractMismatch);
    }
    if runtime.pursuing_cohort != pursuing_cohort {
        return Err(OrdinaryType47LiveError::AimAndFireRuntimeContractMismatch);
    }
    let Some(spawn_index) = entity.authored_spawn_index else {
        return Err(OrdinaryType47LiveError::UncapturedPursuingSpawn);
    };
    let runtime_cohort_matches = match pursuing_cohort {
        Type47AimAllocationPolicy::FreshLevel1 => {
            let expected_seed = FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES
                .iter()
                .position(|&index| index == spawn_index)
                .map(|ordinal| FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS[ordinal]);
            expected_seed == Some(runtime.sub_d_stagger_seed)
                && runtime.sub_d_frame_owner.is_some()
                && runtime.sub_d_runtime.is_some()
        }
        Type47AimAllocationPolicy::Intro2 => {
            crate::intro2_type47_live::intro2_type47_seed_for_spawn(spawn_index)
                .is_some_and(|seed| runtime.authenticates_intro2_cohort(seed))
        }
        Type47AimAllocationPolicy::NativeConstruction => {
            native.is_some_and(|receipt| runtime.authenticates_native_cohort(receipt.sub_d_seed()))
        }
    };
    if !runtime_cohort_matches {
        return Err(OrdinaryType47LiveError::AimAndFireRuntimeContractMismatch);
    }

    let context = match entity.current_behavior_context {
        RetailRuntimeValue::Known(Some(context)) => context,
        RetailRuntimeValue::Known(None) => {
            return Err(OrdinaryType47LiveError::BehaviorContextContractMismatch);
        }
        RetailRuntimeValue::Unresolved => {
            return Err(OrdinaryType47LiveError::BehaviorContextUnresolved);
        }
    };
    let program = behavior_program(TYPE47_PURSUING_BEHAVIOR_CLASS_ID)
        .expect("class-32 Guard Location is in the audited program table");
    let style = *audited_behavior_style(
        TYPE47_PURSUING_BEHAVIOR_CLASS_ID,
        TYPE47_PURSUING_STYLE_INDEX as u8,
    )
    .expect("class-32 pursuing style is in the audited style table");
    let target_entity_id = match context.target_handle_at_0x08() {
        RetailRuntimeValue::Known(Some(target)) => target,
        _ => return Err(OrdinaryType47LiveError::BehaviorContextContractMismatch),
    };
    if context.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || context.choice_list_source()
            != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        || context.auxiliary_word_at_0x0c() != RetailRuntimeValue::Known(0)
        || context.style_table_index_raw_at_0x10() != TYPE47_PURSUING_STYLE_INDEX
        || context.active_style() != ActiveBehaviorStyle::Audited(style)
    {
        return Err(OrdinaryType47LiveError::BehaviorContextContractMismatch);
    }

    let primary_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(OrdinaryType47LiveError::MissingPrimaryChaseTask)?;
    if entity.actor_tasks.wrapper_flags(primary_id)
        != Some(ActorTaskWrapperFlags {
            alive: true,
            in_callback: false,
        })
    {
        return Err(OrdinaryType47LiveError::TaskWrapperStateMismatch);
    }
    let Some(ActorTaskRuntime::ChaseTarget(chase)) =
        entity.actor_tasks.state_in_slot(ActorTaskSlot::Primary)
    else {
        return Err(OrdinaryType47LiveError::ChaseTaskContractMismatch);
    };
    if chase.target_id() != target_entity_id {
        return Err(OrdinaryType47LiveError::ChaseTaskContractMismatch);
    }
    if entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .is_some()
    {
        return Err(OrdinaryType47LiveError::SecondaryTaskUnexpectedlyPresent);
    }
    if task_visit.slot != ActorTaskSlot::Tertiary
        || entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary) != Some(task_visit.task_id)
    {
        return Err(OrdinaryType47LiveError::TaskLeaseMismatch);
    }
    if entity.actor_tasks.wrapper_flags(task_visit.task_id)
        != Some(ActorTaskWrapperFlags {
            alive: true,
            in_callback: expected_in_callback,
        })
    {
        return Err(OrdinaryType47LiveError::TaskWrapperStateMismatch);
    }
    let Some(ActorTaskRuntime::AimAndFire(aim)) =
        entity.actor_tasks.state_in_slot(ActorTaskSlot::Tertiary)
    else {
        return Err(OrdinaryType47LiveError::AimTaskContractMismatch);
    };
    let private = aim.private_state();
    if private.target_entity_id() != target_entity_id
        || private.direction() != 1
        || private.reversal_timer_ms() != 0
        || private.optional_sound_id_raw() != 0
        || private.sound_period_us_raw() != 0
    {
        return Err(OrdinaryType47LiveError::AimTaskContractMismatch);
    }
    Ok(target_entity_id)
}

fn revalidate_active_owner(
    manager: &EntityManager,
    owner: OrdinaryType47AimAndFireOwner,
) -> Result<(), OrdinaryType47LiveError> {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == owner.source_entity_id)
        .ok_or(OrdinaryType47LiveError::SourceEntityMissing)?;
    let target = authenticate_live_owner(entity, owner.task_visit, owner.pursuing_cohort, true)?;
    if target != owner.target_entity_id {
        return Err(OrdinaryType47LiveError::TaskLeaseMismatch);
    }
    Ok(())
}

fn aim_target_runtime_state(
    manager: &EntityManager,
    target_entity_id: u32,
    pursuing_cohort: Type47AimAllocationPolicy,
) -> Result<AimAndFireTargetRuntimeState, OrdinaryType47LiveError> {
    let Some(target) = manager
        .iter_all()
        .find(|entity| entity.id == target_entity_id && entity.active)
    else {
        return Ok(AimAndFireTargetRuntimeState::Missing);
    };
    if !pursuing_cohort.admits_target_type(target.entity_type) {
        return Err(OrdinaryType47LiveError::TargetAllocationTypeMismatch {
            actual: target.entity_type,
        });
    }
    let state = target.collision.state_flags_at_0x08;
    let dying = match state.masked(AIM_AND_FIRE_DYING_STATE_BIT) {
        RetailRuntimeValue::Known(value) => value,
        RetailRuntimeValue::Unresolved => {
            return Err(OrdinaryType47LiveError::TargetStateUnresolved);
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
    Err(OrdinaryType47LiveError::TargetStateUnresolved)
}

fn source_for_math(
    manager: &EntityManager,
    owner: OrdinaryType47AimAndFireOwner,
    source_handle: u32,
) -> Result<&Entity, OrdinaryType47LiveError> {
    if source_handle != owner.source_entity_id {
        return Err(OrdinaryType47LiveError::TaskLeaseMismatch);
    }
    let source = manager
        .iter_all()
        .find(|entity| entity.id == source_handle && entity.active)
        .ok_or(OrdinaryType47LiveError::SourceEntityMissing)?;
    authenticate_live_owner(source, owner.task_visit, owner.pursuing_cohort, true)?;
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

fn is_exact_runtime_shape(runtime: &OrdinaryType47AimAndFireRuntime) -> bool {
    runtime.projectile_descriptor == FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR
        && runtime.component_topology == FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY
        && runtime.emitter_runtime
            == (GenericEmitterRuntime {
                cadence_raw: runtime.emitter_runtime.cadence_raw,
                ..FRESH_LEVEL1_ORDINARY_TYPE47_EMITTER_RUNTIME
            })
}

fn is_exact_transient_request(source_entity_id: u32, request: GenericEmitterAppendRequest) -> bool {
    request.source_handle == source_entity_id
        && request.owner_handle == source_entity_id
        && request.projectile_method == FIRST_WORLD_SHOOTER_PROJECTILE_METHOD
        && request.emitter_selector == 0
        && request.speed_field
            == GenericEmitterSpeedField::Explicit(FIRST_WORLD_SHOOTER_PROJECTILE_SPEED_RAW as i16)
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
    use v2k_formats::collision::SubAPropulsionDescriptor;

    use super::*;
    use crate::{
        actor_task_dispatcher::{
            prepare_aim_and_fire_runtime_task, AimAndFireRuntimeTaskPreparation,
        },
        aim_and_fire::{
            AimAndFireFrameOutcome, AIM_AND_FIRE_CONSTRUCTOR_ADDRESS, AIM_AND_FIRE_LIFETIME_MS,
            AIM_AND_FIRE_TICK_ADDRESS,
        },
        chase_target::ChaseTargetTaskState,
        entity::EntityKind,
        entity_behavior::BehaviorContextRuntime,
        entity_collision_state::RetailStateWord,
    };

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            projectile_emitter_descriptor: RetailRuntimeValue::Known(Some(
                FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR,
            )),
            common_mover_topology: RetailRuntimeValue::Known(
                FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY,
            ),
            sub_d_steering_descriptor: RetailRuntimeValue::Known(Some(
                FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D,
            )),
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                SubAPropulsionDescriptor {
                    acceleration_raw: 1,
                    overspeed_correction_raw: 1,
                    target_speed_base_raw: 300,
                },
            )),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn exact_facts() -> FreshLevel1OrdinaryType47SpawnFacts {
        FreshLevel1OrdinaryType47SpawnFacts {
            retail_first_world: true,
            authored_spawn_index: 11,
            entity_type: ORDINARY_TYPE47_ENTITY_TYPE,
            active_model: Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID),
        }
    }

    #[test]
    fn native_sub_e_keeps_source_descriptor_without_a_captured_seed_or_target_type() {
        let metadata = exact_metadata();
        for seed in [0, 0x06, 0x2b, 0x91, 0xff] {
            let runtime = OrdinaryType47AimAndFireRuntime::from_authenticated_native_metadata(
                &metadata, seed,
            )
            .unwrap();
            assert!(runtime.authenticates_native_cohort(seed));
            assert!(!runtime.authenticates_intro2_cohort(seed));
            assert_eq!(
                runtime.emitter_runtime(),
                FRESH_LEVEL1_ORDINARY_TYPE47_EMITTER_RUNTIME
            );
        }
        let mut changed = metadata;
        let RetailRuntimeValue::Known(Some(ref mut descriptor)) =
            changed.projectile_emitter_descriptor
        else {
            unreachable!()
        };
        descriptor.projectile_method = 29;
        assert!(
            OrdinaryType47AimAndFireRuntime::from_authenticated_native_metadata(&changed, 0x91)
                .is_none()
        );

        let mut manager = live_manager(11);
        // This is a target-state adapter control, not a synthetic native birth.
        // Acquired targets are checked for lookup/state at 07A10; the source
        // does not apply the captured scene's Type9-versus-player restriction.
        manager.entity_mut(200).unwrap().entity_type = 8;
        assert_eq!(
            aim_target_runtime_state(&manager, 200, Type47AimAllocationPolicy::NativeConstruction),
            Ok(AimAndFireTargetRuntimeState::Present { state_flags: 4 })
        );
        assert!(matches!(
            aim_target_runtime_state(&manager, 200, Type47AimAllocationPolicy::Intro2),
            Err(OrdinaryType47LiveError::TargetAllocationTypeMismatch { actual: 8 })
        ));
        manager
            .entity_mut(200)
            .unwrap()
            .collision
            .state_flags_at_0x08 = RetailStateWord::exact(AIM_AND_FIRE_DYING_STATE_BIT);
        assert_eq!(
            aim_target_runtime_state(&manager, 200, Type47AimAllocationPolicy::NativeConstruction),
            Ok(AimAndFireTargetRuntimeState::Present {
                state_flags: AIM_AND_FIRE_DYING_STATE_BIT
            })
        );
    }

    fn live_source(authored_spawn_index: usize, target_id: u32) -> Entity {
        let metadata = exact_metadata();
        let mut source = Entity::unresolved_port_entity(100, EntityKind::Enemy, 47);
        source.authored_spawn_index = Some(authored_spawn_index);
        source.model_slots[0] = Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID);
        source.model_index = Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID);
        source.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
        source.set_motion_raw([0, 0, 0], [10, 0, 0]);
        source.set_rotation_heading_pitch_roll_raw([0; 3]);
        source.current_behavior_context = RetailRuntimeValue::Known(Some(
            BehaviorContextRuntime::named_audited(
                behavior_program(TYPE47_PURSUING_BEHAVIOR_CLASS_ID).unwrap(),
                TYPE47_PURSUING_STYLE_INDEX,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(Some(target_id)),
                RetailRuntimeValue::Known(0),
                *audited_behavior_style(
                    TYPE47_PURSUING_BEHAVIOR_CLASS_ID,
                    TYPE47_PURSUING_STYLE_INDEX as u8,
                )
                .unwrap(),
            )
            .unwrap(),
        ));
        source.ordinary_type47_aim_and_fire_runtime = Some(
            admit_fresh_level1_ordinary_type47(
                FreshLevel1OrdinaryType47SpawnFacts {
                    authored_spawn_index,
                    ..exact_facts()
                },
                Some(&metadata),
            )
            .unwrap()
            .aim_and_fire_runtime(),
        );

        let chase = ChaseTargetTaskState::prepare_after_allocation(
            source.id,
            source.position_raw(),
            target_id,
            &metadata,
        )
        .unwrap()
        .map_task(ActorTaskRuntime::ChaseTarget)
        .apply_suffix(|owner_id, suffix| {
            assert_eq!(owner_id, source.id);
            assert_eq!(suffix.sub_a_target_speed_raw(), Some(400));
            assert!(!suffix.enables_sub_f());
        });
        source
            .actor_tasks
            .replace_prepared(ActorTaskSlot::Primary, chase);

        let aim = prepare_aim_and_fire_runtime_task(
            AimAndFireRuntimeTaskPreparation {
                slot: ActorTaskSlot::Tertiary,
                constructor_address: AIM_AND_FIRE_CONSTRUCTOR_ADDRESS,
                tick_address: AIM_AND_FIRE_TICK_ADDRESS,
                lifetime_ms: AIM_AND_FIRE_LIFETIME_MS,
                target_id,
            },
            source.id,
            source.position_raw(),
            0,
            0,
            &metadata,
        )
        .unwrap()
        .apply_suffix(|owner_id, suffix| {
            assert_eq!(owner_id, source.id);
            assert!(!suffix.enables_sub_f());
        });
        source
            .actor_tasks
            .replace_prepared(ActorTaskSlot::Tertiary, aim);
        source
    }

    fn live_target(target_id: u32) -> Entity {
        let mut target = Entity::unresolved_port_entity(target_id, EntityKind::Player, 46);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
        target.set_motion_raw([1_000, 0, 0], [0; 3]);
        target
    }

    fn live_manager(authored_spawn_index: usize) -> EntityManager {
        let target_id = 200;
        EntityManager::from_entities_for_test(vec![
            live_source(authored_spawn_index, target_id),
            live_target(target_id),
        ])
    }

    fn bind_live_owner(manager: &EntityManager) -> OrdinaryType47AimAndFireOwner {
        bind_fresh_level1_ordinary_type47_aim_and_fire(
            manager.iter_all().find(|entity| entity.id == 100).unwrap(),
        )
        .unwrap()
    }

    fn frame(elapsed_micros: u32) -> OrdinaryType47AimAndFireFrameRequest {
        OrdinaryType47AimAndFireFrameRequest {
            parent_transaction_id: AimAndFireEmitterTransactionId::new(0x4700_0000_0000_0001)
                .unwrap(),
            child_transaction_id: GenericEmitterTransactionId::new(0x4700_0000_0000_0002).unwrap(),
            elapsed_micros,
        }
    }

    #[test]
    fn captured_spawn_fires_to_fifo_then_later_drains_class87_once() {
        for authored_spawn_index in FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES {
            captured_spawn_fires_once(authored_spawn_index);
        }
    }

    fn captured_spawn_fires_once(authored_spawn_index: usize) {
        let mut manager = live_manager(authored_spawn_index);
        let owner = bind_live_owner(&manager);
        let mut world_fx = WorldFx::new();

        let outcome = tick_fresh_level1_ordinary_type47_aim_and_fire(
            &mut manager,
            &mut world_fx,
            owner,
            frame(19_500),
        )
        .unwrap();
        assert_eq!(outcome.resolution.outcome, AimAndFireFrameOutcome::Continue);
        assert_eq!(outcome.queued_shots_added, 1);
        let runtime = manager
            .iter_all()
            .find(|entity| entity.id == owner.source_entity_id())
            .unwrap()
            .ordinary_type47_aim_and_fire_runtime
            .as_ref()
            .unwrap();
        assert_eq!(runtime.queued_shot_count(), 1);
        assert_eq!(runtime.emitter_runtime().cadence_raw, 730_500);
        assert_eq!(world_fx.particle_count(), 0);
        world_fx.process_pending();
        let sounds = world_fx.take_positional_sounds();
        assert_eq!(sounds.len(), 1);
        assert_eq!(
            sounds[0].sound_id,
            usize::from(FIRST_WORLD_SHOOTER_SOUND_ID)
        );

        let drained = drain_live_ordinary_type47_shot_queues(
            &mut manager,
            &mut world_fx,
            ParticleEnvironment::Dry,
            0,
        );
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].0, owner.source_entity_id());
        let drained = drained[0].1.as_ref().expect("queued spawn drains");
        assert_eq!(drained.consumed_requests, 1);
        assert_eq!(drained.materialized_particle_slots.len(), 1);
        assert_eq!(world_fx.particle_count(), 1);

        let replay = drain_fresh_level1_ordinary_type47_shots(
            &mut manager,
            &mut world_fx,
            owner.source_entity_id(),
            ParticleEnvironment::Dry,
            0,
        )
        .unwrap();
        assert_eq!(replay.consumed_requests, 0);
        assert!(replay.materialized_particle_slots.is_empty());
        assert_eq!(world_fx.particle_count(), 1);
    }

    #[test]
    fn live_list_drain_skips_empty_queues() {
        let mut manager = live_manager(11);
        let mut world_fx = WorldFx::new();
        assert!(drain_live_ordinary_type47_shot_queues(
            &mut manager,
            &mut world_fx,
            ParticleEnvironment::Dry,
            0,
        )
        .is_empty());
        assert_eq!(world_fx.particle_count(), 0);
    }

    #[test]
    fn cadence_rejection_commits_before_one_rng_draw_and_emits_nothing() {
        let mut manager = live_manager(12);
        let owner = bind_live_owner(&manager);
        let mut world_fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        assert_eq!(oracle.next_shared_retail_random_u16(), 38);
        let expected_next = oracle.next_shared_retail_random_u16();

        let outcome = tick_fresh_level1_ordinary_type47_aim_and_fire(
            &mut manager,
            &mut world_fx,
            owner,
            frame(20_000),
        )
        .unwrap();
        assert_eq!(outcome.resolution.outcome, AimAndFireFrameOutcome::Continue);
        assert_eq!(outcome.queued_shots_added, 0);
        let runtime = manager
            .iter_all()
            .find(|entity| entity.id == owner.source_entity_id())
            .unwrap()
            .ordinary_type47_aim_and_fire_runtime
            .as_ref()
            .unwrap();
        assert_eq!(runtime.emitter_runtime().cadence_raw, -20_000);
        assert_eq!(runtime.queued_shot_count(), 0);
        assert_eq!(world_fx.next_shared_retail_random_u16(), expected_next);
        world_fx.process_pending();
        assert!(world_fx.take_positional_sounds().is_empty());
        assert_eq!(world_fx.particle_count(), 0);
    }

    #[test]
    fn catch_up_appends_twice_but_keeps_one_sound_suffix() {
        let mut manager = live_manager(11);
        let owner = bind_live_owner(&manager);
        let mut world_fx = WorldFx::new();

        let outcome = tick_fresh_level1_ordinary_type47_aim_and_fire(
            &mut manager,
            &mut world_fx,
            owner,
            frame(800_000),
        )
        .unwrap();
        assert_eq!(outcome.resolution.outcome, AimAndFireFrameOutcome::Continue);
        assert_eq!(outcome.queued_shots_added, 2);
        let runtime = manager
            .iter_all()
            .find(|entity| entity.id == owner.source_entity_id())
            .unwrap()
            .ordinary_type47_aim_and_fire_runtime
            .as_ref()
            .unwrap();
        assert_eq!(runtime.queued_shot_count(), 2);
        assert_eq!(runtime.emitter_runtime().cadence_raw, 700_000);
        world_fx.process_pending();
        assert_eq!(world_fx.take_positional_sounds().len(), 1);
        assert_eq!(world_fx.particle_count(), 0);

        let drained = drain_fresh_level1_ordinary_type47_shots(
            &mut manager,
            &mut world_fx,
            owner.source_entity_id(),
            ParticleEnvironment::Dry,
            0,
        )
        .unwrap();
        assert_eq!(drained.consumed_requests, 2);
        assert_eq!(drained.materialized_particle_slots.len(), 2);
    }

    #[test]
    fn zero_state_target_commits_elapsed_but_skips_emitter_rng_and_effects() {
        let mut manager = live_manager(11);
        manager
            .entity_mut_for_test(200)
            .unwrap()
            .collision
            .state_flags_at_0x08 = RetailStateWord::exact(0);
        let owner = bind_live_owner(&manager);
        let mut world_fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        let first_random = oracle.next_shared_retail_random_u16();

        let outcome = tick_fresh_level1_ordinary_type47_aim_and_fire(
            &mut manager,
            &mut world_fx,
            owner,
            frame(20_000),
        )
        .unwrap();
        assert!(matches!(
            outcome.resolution.outcome,
            AimAndFireFrameOutcome::RequestOwnerTransition { .. }
        ));
        assert_eq!(outcome.queued_shots_added, 0);
        assert_eq!(world_fx.next_shared_retail_random_u16(), first_random);
        let source = manager
            .iter_all()
            .find(|entity| entity.id == owner.source_entity_id())
            .unwrap();
        let ActorTaskRuntime::AimAndFire(aim) =
            source.actor_task_state(ActorTaskSlot::Tertiary).unwrap()
        else {
            panic!("authenticated task changed family")
        };
        assert_eq!(aim.elapsed_ms(), 20);
        let runtime = source
            .ordinary_type47_aim_and_fire_runtime
            .as_ref()
            .unwrap();
        assert_eq!(runtime.emitter_runtime().cadence_raw, 0);
        assert_eq!(runtime.queued_shot_count(), 0);
        world_fx.process_pending();
        assert!(world_fx.take_positional_sounds().is_empty());
    }

    #[test]
    fn fresh_level1_aim_rejects_an_intro2_type9_target() {
        let mut manager = live_manager(11);
        manager.entity_mut_for_test(200).unwrap().entity_type = 9;
        let owner = bind_live_owner(&manager);
        let mut world_fx = WorldFx::new();
        assert_eq!(
            tick_fresh_level1_ordinary_type47_aim_and_fire(
                &mut manager,
                &mut world_fx,
                owner,
                frame(20_000),
            ),
            Err(OrdinaryType47LiveError::TargetAllocationTypeMismatch { actual: 9 })
        );
    }

    #[test]
    fn spawn13_binds_the_same_pursuing_aim_lease() {
        let manager = live_manager(13);
        assert!(bind_fresh_level1_ordinary_type47_aim_and_fire(
            manager.iter_all().find(|entity| entity.id == 100).unwrap()
        )
        .is_ok());
    }

    #[test]
    fn intro2_bind_rejects_a_fresh_level1_runtime_sidecar() {
        let mut source = live_source(11, 200);
        source.authored_spawn_index = Some(6);
        assert_eq!(
            bind_intro2_type47_aim_and_fire(&source),
            Err(OrdinaryType47LiveError::AimAndFireRuntimeContractMismatch)
        );
    }

    #[test]
    fn unauthenticated_spawn_and_a_replaced_tertiary_lease_fail_before_side_effects() {
        let mut manager = live_manager(11);
        manager
            .entity_mut_for_test(100)
            .unwrap()
            .authored_spawn_index = Some(10);
        assert_eq!(
            bind_fresh_level1_ordinary_type47_aim_and_fire(
                manager.iter_all().find(|entity| entity.id == 100).unwrap()
            ),
            Err(OrdinaryType47LiveError::UncapturedPursuingSpawn)
        );

        let mut manager = live_manager(11);
        let stale = bind_live_owner(&manager);
        let metadata = exact_metadata();
        let source = manager
            .entity_mut_for_test(stale.source_entity_id())
            .unwrap();
        let replacement = prepare_aim_and_fire_runtime_task(
            AimAndFireRuntimeTaskPreparation {
                slot: ActorTaskSlot::Tertiary,
                constructor_address: AIM_AND_FIRE_CONSTRUCTOR_ADDRESS,
                tick_address: AIM_AND_FIRE_TICK_ADDRESS,
                lifetime_ms: AIM_AND_FIRE_LIFETIME_MS,
                target_id: stale.target_entity_id(),
            },
            source.id,
            source.position_raw(),
            0,
            0,
            &metadata,
        )
        .unwrap()
        .apply_suffix(|_, _| {});
        source
            .actor_tasks
            .replace_prepared(ActorTaskSlot::Tertiary, replacement);
        let mut world_fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        let first_random = oracle.next_shared_retail_random_u16();
        assert_eq!(
            tick_fresh_level1_ordinary_type47_aim_and_fire(
                &mut manager,
                &mut world_fx,
                stale,
                frame(19_500),
            ),
            Err(OrdinaryType47LiveError::TaskLeaseMismatch)
        );
        assert_eq!(world_fx.next_shared_retail_random_u16(), first_random);
        let source = manager
            .iter_all()
            .find(|entity| entity.id == stale.source_entity_id())
            .unwrap();
        let ActorTaskRuntime::AimAndFire(replacement) =
            source.actor_task_state(ActorTaskSlot::Tertiary).unwrap()
        else {
            panic!("replacement changed family")
        };
        assert_eq!(replacement.elapsed_ms(), 0);
        assert_eq!(
            source
                .ordinary_type47_aim_and_fire_runtime
                .as_ref()
                .unwrap()
                .emitter_runtime()
                .cadence_raw,
            0
        );
    }

    #[test]
    fn exact_three_spawns_retain_the_exact_component_runtime() {
        let metadata = exact_metadata();
        for authored_spawn_index in FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES {
            let admission = admit_fresh_level1_ordinary_type47(
                FreshLevel1OrdinaryType47SpawnFacts {
                    authored_spawn_index,
                    ..exact_facts()
                },
                Some(&metadata),
            )
            .unwrap();
            let runtime = admission.aim_and_fire_runtime();
            assert_eq!(
                runtime.projectile_descriptor(),
                FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR
            );
            assert_eq!(
                runtime.component_topology(),
                FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY
            );
            assert_eq!(
                runtime.emitter_runtime(),
                FRESH_LEVEL1_ORDINARY_TYPE47_EMITTER_RUNTIME
            );
            assert_eq!(runtime.queued_shot_count(), 0);
            assert!(
                !runtime.authenticates_intro2_cohort(0x06),
                "Level-1 sidecar must not authenticate as the Intro2 cohort"
            );
            assert_eq!(
                runtime.sub_d_stagger_seed(),
                FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS
                    [FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES
                        .iter()
                        .position(|&index| index == authored_spawn_index)
                        .expect("authenticated spawn")]
            );
            assert!(
                !crate::ordinary_type9_live::FRESH_LEVEL1_ORDINARY_TYPE9_SUB_D_SEEDS
                    .contains(&runtime.sub_d_stagger_seed()),
                "Type-47 must not reuse Type-9 Sub-D seeds"
            );
        }
    }

    #[test]
    fn admission_rejects_every_unproven_birth_dimension() {
        let metadata = exact_metadata();
        let cases = [
            (
                FreshLevel1OrdinaryType47SpawnFacts {
                    retail_first_world: false,
                    ..exact_facts()
                },
                FreshLevel1OrdinaryType47AdmissionBlock::NotRetailFirstWorld,
            ),
            (
                FreshLevel1OrdinaryType47SpawnFacts {
                    authored_spawn_index: 10,
                    ..exact_facts()
                },
                FreshLevel1OrdinaryType47AdmissionBlock::UnauthenticatedSpawnIndex,
            ),
            (
                FreshLevel1OrdinaryType47SpawnFacts {
                    entity_type: 17,
                    ..exact_facts()
                },
                FreshLevel1OrdinaryType47AdmissionBlock::WrongEntityType,
            ),
            (
                FreshLevel1OrdinaryType47SpawnFacts {
                    active_model: Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID + 1),
                    ..exact_facts()
                },
                FreshLevel1OrdinaryType47AdmissionBlock::WrongActiveModel,
            ),
        ];
        for (facts, expected) in cases {
            assert_eq!(
                admit_fresh_level1_ordinary_type47(facts, Some(&metadata)),
                Err(expected)
            );
        }
        assert_eq!(
            admit_fresh_level1_ordinary_type47(exact_facts(), None),
            Err(FreshLevel1OrdinaryType47AdmissionBlock::MissingExactTypeMetadata)
        );
    }

    #[test]
    fn admission_rejects_unresolved_absent_or_corrupted_authored_data() {
        let exact = exact_metadata();

        let unresolved_descriptor = EntityTypeRuntimeMetadata {
            projectile_emitter_descriptor: RetailRuntimeValue::Unresolved,
            ..exact.clone()
        };
        assert_eq!(
            admit_fresh_level1_ordinary_type47(exact_facts(), Some(&unresolved_descriptor)),
            Err(FreshLevel1OrdinaryType47AdmissionBlock::UnresolvedProjectileDescriptor)
        );

        let absent_descriptor = EntityTypeRuntimeMetadata {
            projectile_emitter_descriptor: RetailRuntimeValue::Known(None),
            ..exact.clone()
        };
        assert_eq!(
            admit_fresh_level1_ordinary_type47(exact_facts(), Some(&absent_descriptor)),
            Err(FreshLevel1OrdinaryType47AdmissionBlock::AuthoredProjectileDescriptorAbsent)
        );

        let corrupted_descriptors = [
            ProjectileEmitterDescriptor {
                projectile_method: 31,
                ..FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR
            },
            ProjectileEmitterDescriptor {
                random_interval_us: 750_001,
                ..FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR
            },
            ProjectileEmitterDescriptor {
                spread_raw: 101,
                ..FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR
            },
            ProjectileEmitterDescriptor {
                aim_threshold_raw: 16_001,
                ..FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR
            },
            ProjectileEmitterDescriptor {
                speed_override_raw: 1,
                ..FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR
            },
            ProjectileEmitterDescriptor {
                target_axis_tolerance_raw: 0x0601,
                ..FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR
            },
            ProjectileEmitterDescriptor {
                sound_id: 71,
                ..FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR
            },
            ProjectileEmitterDescriptor {
                raw_word_at_0x12: 151,
                ..FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR
            },
            ProjectileEmitterDescriptor {
                alternate_emitter_raw: 1,
                ..FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR
            },
            ProjectileEmitterDescriptor {
                stochastic_gate_mode: 1,
                ..FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR
            },
            ProjectileEmitterDescriptor {
                auxiliary_command: 1,
                ..FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR
            },
            ProjectileEmitterDescriptor {
                variable_bindings: [1, 0, 0, 0],
                ..FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR
            },
        ];
        for descriptor in corrupted_descriptors {
            let corrupted_descriptor = EntityTypeRuntimeMetadata {
                projectile_emitter_descriptor: RetailRuntimeValue::Known(Some(descriptor)),
                ..exact.clone()
            };
            assert_eq!(
                admit_fresh_level1_ordinary_type47(exact_facts(), Some(&corrupted_descriptor)),
                Err(FreshLevel1OrdinaryType47AdmissionBlock::ProjectileDescriptorMismatch)
            );
        }

        let unresolved_topology = EntityTypeRuntimeMetadata {
            common_mover_topology: RetailRuntimeValue::Unresolved,
            ..exact.clone()
        };
        assert_eq!(
            admit_fresh_level1_ordinary_type47(exact_facts(), Some(&unresolved_topology)),
            Err(FreshLevel1OrdinaryType47AdmissionBlock::UnresolvedComponentTopology)
        );

        let unresolved_sub_d = EntityTypeRuntimeMetadata {
            sub_d_steering_descriptor: RetailRuntimeValue::Unresolved,
            ..exact.clone()
        };
        assert_eq!(
            admit_fresh_level1_ordinary_type47(exact_facts(), Some(&unresolved_sub_d)),
            Err(FreshLevel1OrdinaryType47AdmissionBlock::UnresolvedSubDDescriptor)
        );

        let absent_sub_d = EntityTypeRuntimeMetadata {
            sub_d_steering_descriptor: RetailRuntimeValue::Known(None),
            ..exact.clone()
        };
        assert_eq!(
            admit_fresh_level1_ordinary_type47(exact_facts(), Some(&absent_sub_d)),
            Err(FreshLevel1OrdinaryType47AdmissionBlock::AuthoredSubDDescriptorAbsent)
        );

        let type9_sub_d = EntityTypeRuntimeMetadata {
            sub_d_steering_descriptor: RetailRuntimeValue::Known(Some(
                crate::common_mover::sub_d::ORDINARY_TYPE9_SUB_D,
            )),
            ..exact.clone()
        };
        assert_eq!(
            admit_fresh_level1_ordinary_type47(exact_facts(), Some(&type9_sub_d)),
            Err(FreshLevel1OrdinaryType47AdmissionBlock::SubDDescriptorMismatch)
        );

        let missing_sub_e = EntityTypeRuntimeMetadata {
            common_mover_topology: RetailRuntimeValue::Known(
                CommonMoverComponentTopology::default(),
            ),
            ..exact
        };
        assert_eq!(
            admit_fresh_level1_ordinary_type47(exact_facts(), Some(&missing_sub_e)),
            Err(FreshLevel1OrdinaryType47AdmissionBlock::ComponentTopologyMismatch)
        );
    }
    fn controlled_muzzle_point(
        origin: [i16; 3],
    ) -> crate::actor_emitter_external_frame::ActorEmitterExternalFramePoint {
        crate::actor_emitter_external_frame::ActorEmitterExternalFramePoint {
            emitter_index: 0,
            source_slot: 150,
            position_raw: origin.map(i32::from),
            position_words: origin,
        }
    }

    #[test]
    fn type47_draw_stamp_is_used_before_shared_time_correction_and_fifo_drain() {
        for muzzle in [None, Some([314, -110, 168]), Some([32767, -32768, -32760])] {
            let mut manager = live_manager(11);
            let owner = bind_live_owner(&manager);
            let mut fx = WorldFx::new();
            let outcome = tick_fresh_level1_ordinary_type47_aim_and_fire(
                &mut manager,
                &mut fx,
                owner,
                frame(19_500),
            )
            .unwrap();
            assert_eq!(outcome.queued_shots_added, 1);
            if let Some(origin) = muzzle {
                let runtime = manager
                    .entity_mut_for_test(100)
                    .unwrap()
                    .ordinary_type47_aim_and_fire_runtime
                    .as_mut()
                    .unwrap();
                assert_eq!(
                    runtime.stamp_draw_emitter_origin(100, controlled_muzzle_point(origin)),
                    Ok(1)
                );
            }
            let source = manager.iter_all().find(|e| e.id == 100).unwrap();
            let shot = source
                .ordinary_type47_aim_and_fire_runtime
                .as_ref()
                .unwrap()
                .transient_shots[0];
            let expected = drain_type47_transient_request(
                shot.request,
                muzzle.unwrap_or_else(|| source.position_raw()),
                source.velocity_raw(),
            )
            .unwrap();
            let drained = drain_fresh_level1_ordinary_type47_shots(
                &mut manager,
                &mut fx,
                100,
                ParticleEnvironment::Dry,
                42,
            )
            .unwrap();
            assert_eq!(drained.consumed_requests, 1);
            let mut births = Vec::new();
            let _ = fx.prepare_presentation([640, 480], 100_000, |particle| {
                births.push(crate::world_fx::world_position_to_raw(particle.position));
                v2k_render::ParticleCenterProjection {
                    screen: [320, 240],
                    depth_raw: 4096,
                    clip: 0,
                }
            });
            // Class87's 442240 projects the live record first; its six
            // independent stack samples are presentation, not extra births.
            assert_eq!(births.first(), Some(&expected.position_raw));
            assert_eq!(fx.particle_count(), 1);
            assert_eq!(
                drain_fresh_level1_ordinary_type47_shots(
                    &mut manager,
                    &mut fx,
                    100,
                    ParticleEnvironment::Dry,
                    42,
                )
                .unwrap()
                .consumed_requests,
                0
            );
        }
    }

    #[test]
    fn type47_draw_stamp_refreshes_all_matching_queued_nodes_without_stamping_later_appends() {
        let mut manager = live_manager(11);
        let owner = bind_live_owner(&manager);
        let mut fx = WorldFx::new();
        assert_eq!(
            tick_fresh_level1_ordinary_type47_aim_and_fire(
                &mut manager,
                &mut fx,
                owner,
                frame(800_000),
            )
            .unwrap()
            .queued_shots_added,
            2
        );
        let runtime = manager
            .entity_mut_for_test(100)
            .unwrap()
            .ordinary_type47_aim_and_fire_runtime
            .as_mut()
            .unwrap();
        let first = runtime.transient_shots[0].request;
        assert_eq!(
            runtime.stamp_draw_emitter_origin(100, controlled_muzzle_point([14, -10, 168])),
            Ok(2)
        );
        assert!(runtime
            .transient_shots
            .iter()
            .all(|s| s.draw_origin_raw == Some([14, -10, 168])));
        runtime.transient_shots.push(Type47TransientShot {
            request: first,
            draw_origin_raw: None,
        });
        assert_eq!(
            runtime.transient_shots.last().unwrap().draw_origin_raw,
            None
        );
        assert_eq!(runtime.queued_shot_count(), 3);
    }

    #[test]
    fn type47_draw_stamp_rejects_wrong_descriptor_slot_and_malformed_fifo_atomically() {
        let mut manager = live_manager(11);
        let owner = bind_live_owner(&manager);
        let mut fx = WorldFx::new();
        tick_fresh_level1_ordinary_type47_aim_and_fire(&mut manager, &mut fx, owner, frame(19_500))
            .unwrap();
        let runtime = manager
            .entity_mut_for_test(100)
            .unwrap()
            .ordinary_type47_aim_and_fire_runtime
            .as_mut()
            .unwrap();
        let before = runtime.clone();
        let mut point = controlled_muzzle_point([14, -10, 168]);
        point.source_slot = 151;
        assert_eq!(
            runtime.stamp_draw_emitter_origin(100, point),
            Err(OrdinaryType47LiveError::DrawEmitterOriginContractMismatch)
        );
        assert_eq!(*runtime, before);
        runtime.transient_shots[0].request.source_handle = 999;
        let malformed = runtime.clone();
        assert_eq!(
            runtime.stamp_draw_emitter_origin(100, controlled_muzzle_point([14, -10, 168])),
            Err(OrdinaryType47LiveError::MalformedTransientRequest { index: 0 })
        );
        assert_eq!(*runtime, malformed);
    }
}
