//! Native Type26 stag-beetle construction and common scheduler ownership.
//!
//! Fresh births consume the Sub-A constructor word before weighted selection:
//! Furniture Nearby selects class26, while Always contributes class33 and
//! class4. The native owner executes 12DA0 and its ordered Primary, Secondary,
//! Tertiary and world-tail phases. Detailed/coarse callback mode comes from the
//! current 02000000 presentation bit; Sub-H geometry is written only by actual
//! visible model submission. Each authored allocation retains its own accepted
//! Sub-D classifier receipt. Ordinary births use the process allocation counter;
//! captured Intro2 fixtures retain their accepted first-query reset receipts.
//!
//! [`Intro2Type26BirthSelection::CapturedDefecate`] preserves the explicit
//! captured class4 fixture without replaying an invented selector word. The
//! detached `tick_intro2_type26_defecate_virus_*` functions below support that
//! fixture; production uses [`tick_intro2_type26_world`]. Its Detailed 02850
//! callback emits class5 carriers, whereas Coarse consumes two words and applies
//! the direct terrain infection cell. Class4's primary tag and strict 2000-ms
//! timeout route through C690; class26 shares the Furniture constructor/callback.

mod authored;
#[cfg(test)]
mod authored_tests;
mod following;
mod impact;
mod mover;
mod native;
#[cfg(test)]
mod native_tests;
mod world;
pub(crate) use authored::{publish_authored_type26, Type26AuthoredConstruction};
pub(crate) use impact::apply_intro2_type26_particle_hit;
pub use impact::{Intro2Type26ImpactBlock, Intro2Type26ImpactOutcome};
pub(crate) use native::publish_intro2_type26;
pub use native::{Intro2Type26BirthSelection, Intro2Type26Publication};
pub use world::{
    tick_intro2_type26_world, Intro2Type26WorldBlock, Intro2Type26WorldFrame,
    Intro2Type26WorldOutcome, Intro2Type26WorldOwner, Intro2Type26WorldTick,
};

pub(crate) fn reselect_after_static_contact(
    frame: &mut crate::intro2_contacts::Intro2ContactFrame<'_>,
    id: u32,
) -> Result<(), &'static str> {
    world::reselect_behavior(
        frame.entities,
        id,
        world::Type26ReselectionFrame {
            resources: frame.resources,
            world_fx: frame.world_fx,
            entry: world::Type26ReselectionEntry::Impact,
        },
    )
    .map_err(|_| "Type26 static C690 suffix")?;
    let owner = Intro2Type26WorldOwner::adopt(frame.entities, id)
        .map_err(|_| "Type26 post-static owner")?;
    frame.actor_tasks.register_intro2_type26(owner);
    Ok(())
}

use v2k_formats::collision::{
    BehaviorChoice, CommonAxisDescriptor, SubAPropulsionDescriptor, SubBLateralDescriptor,
    SubCLiftDescriptor,
};
use v2k_formats::terrain::TerrainGrid;

use crate::actor_task_dispatcher::{
    prepare_defecate_virus_runtime_task, ActorTaskRuntime, DefecateVirusRuntimePreparationError,
};
use crate::actor_task_owner::{ActorTaskSlot, ActorTaskVisit};
use crate::chase_target::ChaseTargetCommonMoverReturn;
use crate::common_mover::sub_d::{intro2_type26_first_query_owner_for_seed, Type9SubDFrameOwner};
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::component_update::ComponentUpdateMode;
use crate::defecate_virus::{
    defecate_virus_terrain_after_unwind, defecate_virus_wander_after_unwind,
    plan_defecate_virus_callback, DefecateVirusCallbackPlan, DefecateVirusCallbackRequest,
    DefecateVirusTerrainCallbackPrefix, DefecateVirusWanderCallbackPrefix,
    DefecateVirusWanderPostUnwind, DefecateVirusWanderRetarget,
    DEFECATE_VIRUS_BEHAVIOR_DESCRIPTOR_ADDRESS, DEFECATE_VIRUS_INITIALIZER_ADDRESS,
    DEFECATE_VIRUS_MODEL_SLOT_STATE_BITS, DEFECATE_VIRUS_PROTOTYPE_ADDRESS,
    DEFECATE_VIRUS_SUPPRESSION_STATE_BIT,
};
use crate::infection_evolution::InfectionCellWrite;
use crate::type26_common_mover::{
    evaluate_type26_common_mover, Type26CommonMoverBlock, Type26CommonMoverRequest,
};
use crate::wander_near_location::WanderNearCommonMoverReturn;
use crate::world_fx::WorldFx;

pub use crate::common_mover::sub_d::INTRO2_TYPE26_SUB_D;
use crate::defecate_virus_owner::{
    apply_defecate_virus_setup, DefecateVirusSetupError, DefecateVirusSetupRequest,
    DefecateVirusSubATopology,
};
use crate::entity::{commit_known_common_master_motion, Entity};
use crate::entity_behavior::{behavior_program, BehaviorContextRuntime, BehaviorSelection};
use crate::entity_collision_state::{
    CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
};
use crate::entity_view_detail::{RetailViewDetail, RetailViewDetailContext};

pub const INTRO2_DEFECATE_VIRUS_ENTITY_TYPE: u32 = 26;
pub const INTRO2_DEFECATE_VIRUS_SPAWN_INDEX: usize = 25;
pub const INTRO2_DEFECATE_VIRUS_RETAIL_HANDLE: u32 = 0x047E_0001;
pub const INTRO2_DEFECATE_VIRUS_MODEL_ID: usize = 267;
pub const INTRO2_DEFECATE_VIRUS_CHOICE_INDEX: usize = 2;
pub const INTRO2_DEFECATE_VIRUS_BEHAVIOR_CLASS_ID: u8 = 4;
pub const INTRO2_DEFECATE_VIRUS_TERRAIN_LIFETIME_MS: u16 = 67;

pub const INTRO2_TYPE26_BEHAVIOR_CHOICES: [BehaviorChoice; 3] = [
    BehaviorChoice {
        weight_rule_id: 8,
        weight_multiplier: 2,
        behavior_class_id: 26,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 4,
        behavior_class_id: 33,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 3,
        behavior_class_id: 4,
    },
];

pub const INTRO2_TYPE26_COMMON_MOVER_TOPOLOGY: CommonMoverComponentTopology =
    CommonMoverComponentTopology {
        sub_a: true,
        sub_b: true,
        sub_c: true,
        sub_d: true,
        sub_e: false,
        sub_f: false,
        sub_g: false,
        sub_h: true,
        sub_i: false,
        sub_j: false,
        sub_k: false,
        sub_l: false,
        sub_m: false,
        sub_n: false,
        sub_o: false,
    };

/// Section-12 Sub-H record count for type 26 / stag model 267 (`(0,6)..(5,11)`).
pub const TYPE26_SUB_H_RECORD_COUNT: usize = 6;

pub const INTRO2_TYPE26_SUB_A: SubAPropulsionDescriptor = SubAPropulsionDescriptor {
    acceleration_raw: 1_500,
    overspeed_correction_raw: -3_000,
    target_speed_base_raw: 200,
};

pub const INTRO2_TYPE26_SUB_B: SubBLateralDescriptor = SubBLateralDescriptor {
    projection_threshold_rate_raw: 10_000,
    correction_rate_raw: 10_000,
};

/// Terrain-only Sub-C. `surface_mode_raw = 0` does not select waves.
pub const INTRO2_TYPE26_SUB_C: SubCLiftDescriptor = SubCLiftDescriptor {
    base_clearance_raw: 75,
    lift_range_raw: 75,
    strength_raw: 3_145_728,
    near_boost_range_raw: 100,
    damping_range_raw: 200,
    surface_mode_raw: 0,
    offset_sample_raw: 0,
    reserved_at_0x0e: [0, 0],
};

/// Spawn-25 / `C200/0E00` process seed from `V200001.run`.
pub const INTRO2_TYPE26_SPAWN25_SUB_D_SEED: u8 = 0x15;

/// Authored-index-10 / `BE00/0F00` process seed from `V200001.run`.
pub const INTRO2_TYPE26_BE00_0F00_SUB_D_SEED: u8 = 0x0A;

pub const INTRO2_TYPE26_SPAWN10_INDEX: usize = 10;
pub const INTRO2_TYPE26_SPAWN25_INDEX: usize = 25;
pub const INTRO2_TYPE26_SPAWN_INDICES: [usize; 2] = [10, 25];

/// TTD `V200001.run` retail handle for spawn 10 (`BE00/0F00`).
pub const INTRO2_TYPE26_SPAWN10_RETAIL_HANDLE: u32 = 0x04F2_0001;
/// TTD `V200001.run` retail handle for spawn 25 (`C200/0E00`).
pub const INTRO2_TYPE26_SPAWN25_RETAIL_HANDLE: u32 = 0x04E3_0001;

pub const fn intro2_type26_seed_for_spawn(spawn_index: usize) -> Option<u8> {
    match spawn_index {
        INTRO2_TYPE26_SPAWN10_INDEX => Some(INTRO2_TYPE26_BE00_0F00_SUB_D_SEED),
        INTRO2_TYPE26_SPAWN25_INDEX => Some(INTRO2_TYPE26_SPAWN25_SUB_D_SEED),
        _ => None,
    }
}

/// The native constructor publishes both retained Sub-D owners only after its
/// exact allocation and metadata admission. They survive task/context changes,
/// including class12; mutable classifier contents are not a birth identity.
pub(crate) fn intro2_type26_allocation_authenticates(entity: &Entity) -> bool {
    entity.active
        && entity.entity_type == INTRO2_DEFECATE_VIRUS_ENTITY_TYPE
        && match entity.native_type26_allocation {
            Some(receipt) => {
                receipt.allocation.entity_id == entity.id
                    && entity.authored_spawn_index == Some(receipt.spawn_index)
            }
            None => entity
                .authored_spawn_index
                .and_then(intro2_type26_seed_for_spawn)
                .is_some(),
        }
        && entity.model_slots == [Some(INTRO2_DEFECATE_VIRUS_MODEL_ID); 4]
        && matches!(entity.initial_behavior, RetailRuntimeValue::Known(Some(selection))
            if matches!(selection.program.class_id, 4 | 26 | 33))
        && entity.intro2_type26_sub_d_frame_owner.is_some()
        && entity.intro2_type26_sub_d_runtime.is_some()
}

/// Ordinary 104B0 publication authenticates the manager generation, not pose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeType26Allocation {
    allocation: crate::main_base_abort::MainBaseAbortActorLease,
    spawn_index: usize,
}

pub(crate) fn type26_manager_allocation_authenticates(
    manager: &crate::entity::EntityManager,
    id: u32,
) -> bool {
    let Some(entity) = manager.iter_all().find(|e| e.id == id) else {
        return false;
    };
    if !intro2_type26_allocation_authenticates(entity) {
        return false;
    }
    entity.native_type26_allocation.is_none_or(|receipt| {
        manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|observation| observation.lease == receipt.allocation)
    })
}

pub const fn intro2_type26_retail_handle_for_spawn(spawn_index: usize) -> Option<u32> {
    match spawn_index {
        INTRO2_TYPE26_SPAWN10_INDEX => Some(INTRO2_TYPE26_SPAWN10_RETAIL_HANDLE),
        INTRO2_TYPE26_SPAWN25_INDEX => Some(INTRO2_DEFECATE_VIRUS_RETAIL_HANDLE),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type26DefecateVirusAdmission {
    selection: BehaviorSelection,
    captured_retail_handle: u32,
    terrain_task_lifetime_ms: u16,
    sub_d_frame_owner: Type9SubDFrameOwner,
}

impl Intro2Type26DefecateVirusAdmission {
    pub const fn selection(self) -> BehaviorSelection {
        self.selection
    }

    /// Allocation handle observed at the accepted spawn-25 constructor
    /// boundary. This is evidence identity, not the port's local `Entity::id`.
    pub const fn captured_retail_handle(self) -> u32 {
        self.captured_retail_handle
    }

    pub const fn terrain_task_lifetime_ms(self) -> u16 {
        self.terrain_task_lifetime_ms
    }

    pub const fn sub_d_frame_owner(self) -> Type9SubDFrameOwner {
        self.sub_d_frame_owner
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type26DefecateVirusPublicationError {
    TerrainUnavailable,
    EntityIdentityMismatch,
    MetadataMismatch,
    InitialBehaviorAlreadyResolved,
    CurrentBehaviorAlreadyResolved,
    TaskTableNotEmpty,
    SubARuntimeUnavailable,
    SubHRuntimeUnavailable,
    BehaviorContextUnavailable,
    AxisUnavailable,
    Furniture(crate::trash_furniture::TrashFurniturePublicationError),
    Setup(DefecateVirusSetupError<DefecateVirusRuntimePreparationError>),
}

/// Authenticate the accepted choice-2/class-4 receipt without consuming RNG.
pub(crate) fn authenticate_intro2_type26_defecate_virus(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<Intro2Type26DefecateVirusAdmission, Intro2Type26DefecateVirusPublicationError> {
    let spawn_index = entity
        .authored_spawn_index
        .ok_or(Intro2Type26DefecateVirusPublicationError::EntityIdentityMismatch)?;
    let seed = intro2_type26_seed_for_spawn(spawn_index)
        .ok_or(Intro2Type26DefecateVirusPublicationError::EntityIdentityMismatch)?;
    let captured_retail_handle = intro2_type26_retail_handle_for_spawn(spawn_index)
        .ok_or(Intro2Type26DefecateVirusPublicationError::EntityIdentityMismatch)?;
    if !entity.active
        || entity.entity_type != INTRO2_DEFECATE_VIRUS_ENTITY_TYPE
        || entity.model_slots != [Some(INTRO2_DEFECATE_VIRUS_MODEL_ID); 4]
        || entity.model_index != Some(INTRO2_DEFECATE_VIRUS_MODEL_ID)
    {
        return Err(Intro2Type26DefecateVirusPublicationError::EntityIdentityMismatch);
    }
    if entity.initial_behavior != RetailRuntimeValue::Unresolved {
        return Err(Intro2Type26DefecateVirusPublicationError::InitialBehaviorAlreadyResolved);
    }
    if entity.current_behavior_context != RetailRuntimeValue::Unresolved {
        return Err(Intro2Type26DefecateVirusPublicationError::CurrentBehaviorAlreadyResolved);
    }
    if ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .any(|slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Intro2Type26DefecateVirusPublicationError::TaskTableNotEmpty);
    }

    let program = authenticate_metadata(metadata)?;

    Ok(Intro2Type26DefecateVirusAdmission {
        selection: BehaviorSelection {
            choice_index: INTRO2_DEFECATE_VIRUS_CHOICE_INDEX,
            program,
        },
        captured_retail_handle,
        terrain_task_lifetime_ms: match metadata.terrain_contact_task_lifetime_ms {
            RetailRuntimeValue::Known(lifetime_ms) => lifetime_ms,
            RetailRuntimeValue::Unresolved => unreachable!("metadata admission checked +0xA2"),
        },
        sub_d_frame_owner: intro2_type26_first_query_owner_for_seed(seed)
            .expect("spawn seed is one of the two TTD identities"),
    })
}

pub(crate) fn authenticate_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<
    &'static crate::entity_behavior::BehaviorProgram,
    Intro2Type26DefecateVirusPublicationError,
> {
    let Some(initializer) = metadata.initializer.as_ref() else {
        return Err(Intro2Type26DefecateVirusPublicationError::MetadataMismatch);
    };
    let Some(program) = behavior_program(u32::from(INTRO2_DEFECATE_VIRUS_BEHAVIOR_CLASS_ID)) else {
        return Err(Intro2Type26DefecateVirusPublicationError::MetadataMismatch);
    };
    if metadata.model_slots != [INTRO2_DEFECATE_VIRUS_MODEL_ID as u16; 4]
        || metadata.mass_raw != 400
        || metadata.capability_flags != 8
        || metadata.initial_health_raw != Some(5_000)
        || metadata.terrain_contact_task_lifetime_ms
            != RetailRuntimeValue::Known(INTRO2_DEFECATE_VIRUS_TERRAIN_LIFETIME_MS)
        || metadata.sub_a_propulsion_descriptor
            != RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_A))
        || metadata.sub_b_lateral_descriptor != RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_B))
        || metadata.sub_c_lift_descriptor != RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_C))
        || metadata.common_mover_topology
            != RetailRuntimeValue::Known(INTRO2_TYPE26_COMMON_MOVER_TOPOLOGY)
        || metadata.sub_d_steering_descriptor
            != RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_D))
        || initializer.initializer_state_flags_raw != 0x0439
        || initializer.common_axis_descriptor
            != (CommonAxisDescriptor {
                strict_axis_limit_raw: 0x0F00,
                raw_word_at_0x04: 0x0225,
            })
        || initializer.behavior_choices.as_ref() != INTRO2_TYPE26_BEHAVIOR_CHOICES
        || initializer.behavior_rule_ref != 1
        || initializer.alternate_behavior_class_ref != 12
        || program.class_id != INTRO2_DEFECATE_VIRUS_BEHAVIOR_CLASS_ID
        || program.descriptor_address != DEFECATE_VIRUS_BEHAVIOR_DESCRIPTOR_ADDRESS
        || program.style_table_base_address != DEFECATE_VIRUS_PROTOTYPE_ADDRESS
        || program.initial_style.frame_address != DEFECATE_VIRUS_PROTOTYPE_ADDRESS
        || program.initializer_callback_address != DEFECATE_VIRUS_INITIALIZER_ADDRESS
    {
        return Err(Intro2Type26DefecateVirusPublicationError::MetadataMismatch);
    }

    Ok(program)
}

/// Publish the captured initializer success and its exact three-slot topology.
pub fn publish_intro2_type26_defecate_virus(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<Intro2Type26DefecateVirusAdmission, Intro2Type26DefecateVirusPublicationError> {
    let admission = authenticate_intro2_type26_defecate_virus(entity, metadata)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(admission.selection())
        .ok_or(Intro2Type26DefecateVirusPublicationError::BehaviorContextUnavailable)?;
    let anchor = entity.position_raw();
    let RetailRuntimeValue::Known(Some(sub_a)) = &mut entity.sub_a_propulsion_runtime else {
        return Err(Intro2Type26DefecateVirusPublicationError::SubARuntimeUnavailable);
    };

    apply_defecate_virus_setup(
        &mut entity.actor_tasks,
        DefecateVirusSetupRequest {
            terrain_task_lifetime_ms: admission.terrain_task_lifetime_ms(),
        },
        DefecateVirusSubATopology::Authored(sub_a),
        |preparation| prepare_defecate_virus_runtime_task(preparation, anchor),
    )
    .map_err(Intro2Type26DefecateVirusPublicationError::Setup)?;

    entity.initial_behavior = RetailRuntimeValue::Known(Some(admission.selection()));
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    entity.intro2_type26_sub_d_frame_owner = Some(admission.sub_d_frame_owner());
    entity.intro2_type26_sub_d_runtime =
        Some(crate::common_mover::sub_d::Type9SubDRuntime::from_constructor());
    Ok(admission)
}

/// Install a retained Q31 matrix for the spawn-25 visit.
///
/// This is not `FUN_00413F70`. Callers must already own a Known basis.
pub fn retain_intro2_type26_physical_body_basis(entity: &mut Entity, basis: Type9BodyBasis) {
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(basis);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type26PrimaryVisitBlock {
    EntityIdentityMismatch,
    WanderVisitUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type26PrimaryVisitResult {
    Continue,
    TaggedZeroMover,
    CommonMoverBlocked(Type26CommonMoverBlock),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type26PrimaryVisit {
    pub lifetime_ms: u32,
    pub retarget: DefecateVirusWanderRetarget,
    pub result: Intro2Type26PrimaryVisitResult,
    pub owner_transition: Intro2Type26OwnerTransition,
}

/// Run one spawn-25 Primary `FUN_00402BA0` visit through the detached type-26
/// `FUN_00401430` bind.
///
/// An unresolved physical matrix is forwarded as
/// [`Type26CommonMoverBlock::BodyBasisUnavailable`]; this visit does not
/// invent `FUN_00413F70`. Type 13 and Level-1 Type-47 never enter.
pub fn tick_intro2_type26_defecate_virus_primary(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
    terrain: Option<&TerrainGrid>,
    _model_records: Option<&[[i16; 4]]>,
    elapsed_micros: u32,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type26PrimaryVisit, Intro2Type26PrimaryVisitBlock> {
    let Some(spawn_index) = entity.authored_spawn_index else {
        return Err(Intro2Type26PrimaryVisitBlock::EntityIdentityMismatch);
    };
    let Some(sub_d_stagger_seed) = intro2_type26_seed_for_spawn(spawn_index) else {
        return Err(Intro2Type26PrimaryVisitBlock::EntityIdentityMismatch);
    };
    if !entity.active || entity.entity_type != INTRO2_DEFECATE_VIRUS_ENTITY_TYPE {
        return Err(Intro2Type26PrimaryVisitBlock::EntityIdentityMismatch);
    }
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) else {
        return Err(Intro2Type26PrimaryVisitBlock::WanderVisitUnavailable);
    };
    if !matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::DefecateVirusWander(_))
    ) {
        return Err(Intro2Type26PrimaryVisitBlock::WanderVisitUnavailable);
    }
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let Some(lifetime_ms) = entity.actor_tasks.begin_exact_visit_with(visit, |runtime| {
        let ActorTaskRuntime::DefecateVirusWander(state) = runtime else {
            unreachable!("Primary identity already authenticated as DefecateVirusWander")
        };
        state.before_callback(elapsed_micros)
    }) else {
        return Err(Intro2Type26PrimaryVisitBlock::WanderVisitUnavailable);
    };

    let position_raw = entity.position_raw();
    let velocity_raw = entity.velocity_raw();
    let heading_raw = entity.heading_raw();
    let roll_raw = entity.rotation_heading_pitch_roll_raw()[2] as u16;
    let sub_a_runtime = entity.sub_a_propulsion_runtime;
    let (body_right_q31, body_forward_q31, body_up_q31) = match entity.physical_body_basis_q31 {
        RetailRuntimeValue::Known(Type9BodyBasis {
            lateral,
            forward,
            up,
        }) => (
            RetailRuntimeValue::Known(lateral),
            RetailRuntimeValue::Known(forward),
            RetailRuntimeValue::Known(up),
        ),
        RetailRuntimeValue::Unresolved => (
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
        ),
    };
    let mut stage = match entity.actor_tasks.task_state_mut(visit.task_id) {
        Some(ActorTaskRuntime::DefecateVirusWander(state)) => {
            state.stage_callback(position_raw, || next_random() as u16)
        }
        _ => {
            finish_intro2_type26_primary_visit(entity, visit);
            return Err(Intro2Type26PrimaryVisitBlock::WanderVisitUnavailable);
        }
    };
    let mover = evaluate_type26_common_mover(
        Type26CommonMoverRequest {
            entity_id: entity.id,
            entity_type: entity.entity_type,
            metadata,
            position_raw,
            velocity_raw,
            heading_raw,
            roll_raw,
            sub_a_runtime,
            sub_d_stagger_seed: Some(sub_d_stagger_seed),
            sub_d_frame_owner: entity.intro2_type26_sub_d_frame_owner.as_mut(),
            sub_d_runtime: entity.intro2_type26_sub_d_runtime.as_mut(),
            body_right_q31,
            body_forward_q31,
            body_up_q31,
            terrain,
            global_elapsed_micros: elapsed_micros,
            target_private: stage.private_state_mut(),
            tracked_target: RetailRuntimeValue::Known(None),
            elapsed_micros,
        },
        || next_random(),
    );
    let result = match mover {
        Ok(outcome) => {
            if matches!(
                outcome.result,
                ChaseTargetCommonMoverReturn::NonZero | ChaseTargetCommonMoverReturn::Zero
            ) {
                if let Some(ActorTaskRuntime::DefecateVirusWander(surviving)) =
                    entity.actor_tasks.task_state_mut(visit.task_id)
                {
                    stage.commit(surviving);
                }
                entity.set_heading_raw(outcome.heading_raw);
                let [heading_raw, pitch_raw, roll_raw] = entity.rotation_heading_pitch_roll_raw();
                entity.physical_body_basis_q31 = RetailRuntimeValue::Known(
                    Type9BodyBasis::from_angle_words(heading_raw, pitch_raw, roll_raw),
                );
                entity.set_motion_raw(outcome.position_raw, outcome.velocity_raw);
                commit_known_common_master_motion(entity, elapsed_micros);
            }
            match outcome.result {
                ChaseTargetCommonMoverReturn::NonZero => Intro2Type26PrimaryVisitResult::Continue,
                ChaseTargetCommonMoverReturn::Zero => {
                    Intro2Type26PrimaryVisitResult::TaggedZeroMover
                }
            }
        }
        Err(block) => Intro2Type26PrimaryVisitResult::CommonMoverBlocked(block),
    };
    finish_intro2_type26_primary_visit(entity, visit);
    let owner_transition = resolve_intro2_type26_wander_owner_transition(
        entity,
        visit,
        DefecateVirusWanderCallbackPrefix {
            elapsed_ms: lifetime_ms,
            retarget: stage.retarget(),
        },
        result,
    );
    Ok(Intro2Type26PrimaryVisit {
        lifetime_ms,
        retarget: stage.retarget(),
        result,
        owner_transition,
    })
}

fn finish_intro2_type26_primary_visit(entity: &mut Entity, visit: ActorTaskVisit) {
    finish_intro2_type26_exact_visit(entity, visit);
}

fn finish_intro2_type26_exact_visit(entity: &mut Entity, visit: ActorTaskVisit) {
    if entity
        .actor_tasks
        .wrapper_flags(visit.task_id)
        .is_some_and(|flags| flags.in_callback)
    {
        let _ = entity.actor_tasks.finish_exact_visit(visit);
    }
}

const TYPE26_DETAILED_UPDATE_STATE_BIT: u32 = 0x0200_0000;
const TYPE26_OWNER_SIGN_STATE_BIT: u32 = 0x8000_0000;
const TYPE26_TERRAIN_CALLBACK_STATE_MASK: u32 = DEFECATE_VIRUS_SUPPRESSION_STATE_BIT
    | DEFECATE_VIRUS_MODEL_SLOT_STATE_BITS
    | TYPE26_OWNER_SIGN_STATE_BIT
    | TYPE26_DETAILED_UPDATE_STATE_BIT;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type26TertiaryVisitBlock {
    EntityIdentityMismatch,
    TerrainVisitUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type26TertiaryVisitResult {
    Planned(DefecateVirusCallbackPlan),
    StateFlagsUnavailable,
    UpdateModeUnavailable,
    BodyBasisUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type26OwnerTransition {
    NotRequested,
    CallbackAbsent,
    SuppressedByEntityState,
    StateFlagsUnavailable,
    StyleUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type26TertiaryVisit {
    pub prefix: DefecateVirusTerrainCallbackPrefix,
    pub result: Intro2Type26TertiaryVisitResult,
    pub owner_transition: Intro2Type26OwnerTransition,
}

/// Resolve unsigned Section-8 header `+0x08` words for entity slots
/// `+0xA8/+0xAA/+0xAC/+0xAE`.
///
/// Missing slots or missing model records stay `None`. The lookup must return
/// header `+0x08`, not `+0x0A` collision radius.
pub fn intro2_type26_model_extent_raw_by_state(
    entity: &Entity,
    mut header_extent_raw: impl FnMut(usize) -> Option<u16>,
) -> [Option<u16>; 4] {
    core::array::from_fn(|slot| entity.model_in_slot(slot).and_then(&mut header_extent_raw))
}

/// Run one retained Type26 Tertiary `FUN_00402850` visit.
///
/// Detailed mode reads the constructor `FUN_00413F70` forward column when
/// generic `FUN_0040D720` already wrote it. This visit does not invent that
/// matrix. Missing model extents stay
/// [`DefecateVirusCallbackPlan::DetailedModelExtentUnavailable`].
pub fn tick_intro2_type26_defecate_virus_tertiary(
    entity: &mut Entity,
    model_extent_raw_by_state: [Option<u16>; 4],
    elapsed_micros: u32,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type26TertiaryVisit, Intro2Type26TertiaryVisitBlock> {
    if !intro2_type26_allocation_authenticates(entity) {
        return Err(Intro2Type26TertiaryVisitBlock::EntityIdentityMismatch);
    }
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary) else {
        return Err(Intro2Type26TertiaryVisitBlock::TerrainVisitUnavailable);
    };
    if !matches!(
        entity.actor_task_state(ActorTaskSlot::Tertiary),
        Some(ActorTaskRuntime::DefecateVirusTerrain(_))
    ) {
        return Err(Intro2Type26TertiaryVisitBlock::TerrainVisitUnavailable);
    }
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Tertiary,
        task_id,
    };
    let Some(prefix) = entity.actor_tasks.begin_exact_visit_with(visit, |runtime| {
        let ActorTaskRuntime::DefecateVirusTerrain(state) = runtime else {
            unreachable!("Tertiary identity already authenticated as DefecateVirusTerrain")
        };
        state.before_callback(elapsed_micros)
    }) else {
        return Err(Intro2Type26TertiaryVisitBlock::TerrainVisitUnavailable);
    };

    let result = match intro2_type26_terrain_callback_request(
        entity,
        model_extent_raw_by_state,
        elapsed_micros,
    ) {
        Ok(request) => {
            Intro2Type26TertiaryVisitResult::Planned(plan_defecate_virus_callback(request, || {
                next_random() as u16
            }))
        }
        Err(result) => result,
    };
    finish_intro2_type26_exact_visit(entity, visit);
    let owner_transition = resolve_intro2_type26_terrain_owner_transition(entity, visit, prefix);
    Ok(Intro2Type26TertiaryVisit {
        prefix,
        result,
        owner_transition,
    })
}

fn resolve_intro2_type26_wander_owner_transition(
    entity: &Entity,
    visit: ActorTaskVisit,
    prefix: DefecateVirusWanderCallbackPrefix,
    result: Intro2Type26PrimaryVisitResult,
) -> Intro2Type26OwnerTransition {
    let mover_return = match result {
        Intro2Type26PrimaryVisitResult::Continue => WanderNearCommonMoverReturn::NonZero,
        Intro2Type26PrimaryVisitResult::TaggedZeroMover => WanderNearCommonMoverReturn::Zero,
        Intro2Type26PrimaryVisitResult::CommonMoverBlocked(_) => {
            return Intro2Type26OwnerTransition::NotRequested;
        }
    };
    match defecate_virus_wander_after_unwind(visit, prefix, mover_return) {
        DefecateVirusWanderPostUnwind::Transition(_) => {
            resolve_intro2_type26_class4_owner_transition(entity)
        }
        DefecateVirusWanderPostUnwind::Continue
        | DefecateVirusWanderPostUnwind::UnresolvedCommonMover => {
            Intro2Type26OwnerTransition::NotRequested
        }
    }
}

fn resolve_intro2_type26_terrain_owner_transition(
    entity: &Entity,
    visit: ActorTaskVisit,
    prefix: DefecateVirusTerrainCallbackPrefix,
) -> Intro2Type26OwnerTransition {
    if defecate_virus_terrain_after_unwind(visit, prefix).is_none() {
        return Intro2Type26OwnerTransition::NotRequested;
    }
    resolve_intro2_type26_class4_owner_transition(entity)
}

fn resolve_intro2_type26_class4_owner_transition(entity: &Entity) -> Intro2Type26OwnerTransition {
    match entity
        .collision
        .state_flags_at_0x08
        .masked(DEFECATE_VIRUS_SUPPRESSION_STATE_BIT)
    {
        RetailRuntimeValue::Unresolved => Intro2Type26OwnerTransition::StateFlagsUnavailable,
        RetailRuntimeValue::Known(bits) if bits != 0 => {
            Intro2Type26OwnerTransition::SuppressedByEntityState
        }
        RetailRuntimeValue::Known(_) => {
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                return Intro2Type26OwnerTransition::StyleUnavailable;
            };
            if context.active_style().style_address() != DEFECATE_VIRUS_PROTOTYPE_ADDRESS {
                return Intro2Type26OwnerTransition::StyleUnavailable;
            }
            Intro2Type26OwnerTransition::CallbackAbsent
        }
    }
}

fn intro2_type26_terrain_callback_request(
    entity: &Entity,
    model_extent_raw_by_state: [Option<u16>; 4],
    elapsed_micros: u32,
) -> Result<DefecateVirusCallbackRequest, Intro2Type26TertiaryVisitResult> {
    let flags = entity.collision.state_flags_at_0x08;
    let RetailRuntimeValue::Known(known_flags) = flags.masked(TYPE26_TERRAIN_CALLBACK_STATE_MASK)
    else {
        return Err(Intro2Type26TertiaryVisitResult::StateFlagsUnavailable);
    };
    let RetailRuntimeValue::Known(update_bit) = flags.masked(TYPE26_DETAILED_UPDATE_STATE_BIT)
    else {
        return Err(Intro2Type26TertiaryVisitResult::UpdateModeUnavailable);
    };
    let update_mode = if update_bit != 0 {
        ComponentUpdateMode::Detailed
    } else {
        ComponentUpdateMode::Coarse
    };
    let forward_q31 = match (update_mode, entity.physical_body_basis_q31) {
        (ComponentUpdateMode::Detailed, RetailRuntimeValue::Unresolved) => {
            return Err(Intro2Type26TertiaryVisitResult::BodyBasisUnavailable);
        }
        (ComponentUpdateMode::Detailed, RetailRuntimeValue::Known(basis)) => basis.forward,
        (ComponentUpdateMode::Coarse, _) => [0; 3],
    };
    Ok(DefecateVirusCallbackRequest {
        update_mode,
        elapsed_micros,
        entity_state_flags: known_flags,
        position_raw: entity.position_raw(),
        forward_q31,
        model_extent_raw_by_state,
        owner_entity_handle: entity.id,
    })
}

pub fn apply_intro2_type26_tertiary_plan(
    world_fx: &mut WorldFx,
    infection_writes: &mut Vec<InfectionCellWrite>,
    visit: &Intro2Type26TertiaryVisit,
) {
    let Intro2Type26TertiaryVisitResult::Planned(plan) = visit.result else {
        return;
    };
    match plan {
        DefecateVirusCallbackPlan::DetailedParticle(emission) => {
            // FUN_00440DC0 only allocates. Ordinary-surface infection writes
            // wait for the later FUN_0043E180 traversal.
            world_fx.emit_defecate_virus_particle_raw(emission);
        }
        DefecateVirusCallbackPlan::CoarseTerrainMutation(write) => {
            infection_writes.push(write);
        }
        DefecateVirusCallbackPlan::SuppressedByEntityState
        | DefecateVirusCallbackPlan::DetailedChanceRejected
        | DefecateVirusCallbackPlan::DetailedModelExtentUnavailable { .. } => {}
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type26ViewDetailBlock {
    EntityIdentityMismatch,
}

/// Apply spawn-25's `FUN_00411400` view-detail subset write.
///
/// This replaces only `0x06000000`. A known-clear or unresolved `0x800` gate
/// leaves the word untouched. Type 13 and Level-1 Type-47 never enter.
pub fn publish_intro2_type26_view_detail(
    entity: &mut Entity,
    context: RetailViewDetailContext,
) -> Result<RetailRuntimeValue<Option<RetailViewDetail>>, Intro2Type26ViewDetailBlock> {
    if !entity.active
        || entity
            .authored_spawn_index
            .and_then(intro2_type26_seed_for_spawn)
            .is_none()
        || entity.entity_type != INTRO2_DEFECATE_VIRUS_ENTITY_TYPE
    {
        return Err(Intro2Type26ViewDetailBlock::EntityIdentityMismatch);
    }
    Ok(context.publish(
        entity.position_raw(),
        &mut entity.collision.state_flags_at_0x08,
    ))
}

/// Publish view-detail bits for all published Intro2 type-26 entities.
pub fn publish_captured_intro2_type26_view_detail(
    manager: &mut crate::entity::EntityManager,
    context: RetailViewDetailContext,
) -> Result<RetailRuntimeValue<Option<RetailViewDetail>>, Intro2Type26ViewDetailBlock> {
    let ids = manager
        .iter_all()
        .filter(|entity| {
            entity.active
                && entity.entity_type == INTRO2_DEFECATE_VIRUS_ENTITY_TYPE
                && entity
                    .authored_spawn_index
                    .and_then(intro2_type26_seed_for_spawn)
                    .is_some()
        })
        .map(|entity| (entity.id, entity.authored_spawn_index))
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return Err(Intro2Type26ViewDetailBlock::EntityIdentityMismatch);
    }
    let mut spawn25_result = None;
    let mut last_result = None;
    for (entity_id, spawn_index) in ids {
        let Some(entity) = manager.intro2_type26_entity_mut(entity_id) else {
            return Err(Intro2Type26ViewDetailBlock::EntityIdentityMismatch);
        };
        let res = publish_intro2_type26_view_detail(entity, context)?;
        if spawn_index == Some(INTRO2_DEFECATE_VIRUS_SPAWN_INDEX) {
            spawn25_result = Some(res);
        }
        last_result = Some(res);
    }
    spawn25_result
        .or(last_result)
        .ok_or(Intro2Type26ViewDetailBlock::EntityIdentityMismatch)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type26SchedulerOwner {
    entity_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type26SchedulerAdoptionError {
    EntityUnavailable,
    GraphUnavailable { entity_id: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type26SchedulerProductionDrop {
    EntityUnavailable,
    GraphMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type26SchedulerProductionOutcome {
    Primary {
        entity_id: u32,
        visit: Intro2Type26PrimaryVisit,
        tertiary: Intro2Type26TertiaryVisit,
    },
    Dropped {
        entity_id: u32,
        reason: Intro2Type26SchedulerProductionDrop,
    },
}

impl Intro2Type26SchedulerProductionOutcome {
    pub const fn entity_id(self) -> u32 {
        match self {
            Self::Primary { entity_id, .. } | Self::Dropped { entity_id, .. } => entity_id,
        }
    }
}

pub struct Intro2Type26SchedulerOwnerTick {
    pub outcome: Intro2Type26SchedulerProductionOutcome,
    pub retained_owner: Option<Intro2Type26SchedulerOwner>,
}

impl Intro2Type26SchedulerOwner {
    pub const fn entity_id(self) -> u32 {
        self.entity_id
    }

    /// Admit only the captured spawn-25 publication graph.
    pub fn adopt(
        manager: &crate::entity::EntityManager,
    ) -> Result<Self, Intro2Type26SchedulerAdoptionError> {
        Self::adopt_spawn(manager, INTRO2_DEFECATE_VIRUS_SPAWN_INDEX)
    }

    /// Admit only the specified captured type-26 spawn index publication graph.
    pub fn adopt_spawn(
        manager: &crate::entity::EntityManager,
        spawn_index: usize,
    ) -> Result<Self, Intro2Type26SchedulerAdoptionError> {
        let Some(entity) = manager.iter_all().find(|entity| {
            entity.active
                && entity.entity_type == INTRO2_DEFECATE_VIRUS_ENTITY_TYPE
                && entity.authored_spawn_index == Some(spawn_index)
        }) else {
            return Err(Intro2Type26SchedulerAdoptionError::EntityUnavailable);
        };
        Self::adopt_published(entity)
    }

    pub fn adopt_published(entity: &Entity) -> Result<Self, Intro2Type26SchedulerAdoptionError> {
        if !intro2_type26_primary_graph_authenticates(entity) {
            return Err(Intro2Type26SchedulerAdoptionError::GraphUnavailable {
                entity_id: entity.id,
            });
        }
        Ok(Self {
            entity_id: entity.id,
        })
    }
}

pub fn tick_intro2_type26_scheduler_owner(
    manager: &mut crate::entity::EntityManager,
    owner: Intro2Type26SchedulerOwner,
    terrain: Option<&TerrainGrid>,
    model_records: Option<&[[i16; 4]]>,
    model_extent_raw_by_state: [Option<u16>; 4],
    elapsed_micros: u32,
    next_random: &mut impl FnMut() -> u32,
) -> Intro2Type26SchedulerOwnerTick {
    let metadata = manager
        .type_runtime_metadata(INTRO2_DEFECATE_VIRUS_ENTITY_TYPE)
        .cloned();
    let Some(entity) = manager.intro2_type26_entity_mut(owner.entity_id()) else {
        return Intro2Type26SchedulerOwnerTick {
            outcome: Intro2Type26SchedulerProductionOutcome::Dropped {
                entity_id: owner.entity_id(),
                reason: Intro2Type26SchedulerProductionDrop::EntityUnavailable,
            },
            retained_owner: None,
        };
    };
    tick_intro2_type26_scheduler_owner_on_entity(
        entity,
        metadata.as_ref(),
        terrain,
        model_records,
        model_extent_raw_by_state,
        elapsed_micros,
        next_random,
        owner,
    )
}

pub fn tick_intro2_type26_scheduler_owner_on_entity(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
    terrain: Option<&TerrainGrid>,
    model_records: Option<&[[i16; 4]]>,
    model_extent_raw_by_state: [Option<u16>; 4],
    elapsed_micros: u32,
    next_random: &mut impl FnMut() -> u32,
    owner: Intro2Type26SchedulerOwner,
) -> Intro2Type26SchedulerOwnerTick {
    let entity_id = owner.entity_id();
    if entity.id != entity_id || !intro2_type26_primary_graph_authenticates(entity) {
        return Intro2Type26SchedulerOwnerTick {
            outcome: Intro2Type26SchedulerProductionOutcome::Dropped {
                entity_id,
                reason: Intro2Type26SchedulerProductionDrop::GraphMismatch,
            },
            retained_owner: None,
        };
    }
    match tick_intro2_type26_defecate_virus_primary(
        entity,
        metadata,
        terrain,
        model_records,
        elapsed_micros,
        next_random,
    ) {
        Ok(visit) => {
            match tick_intro2_type26_defecate_virus_tertiary(
                entity,
                model_extent_raw_by_state,
                elapsed_micros,
                next_random,
            ) {
                Ok(tertiary) => Intro2Type26SchedulerOwnerTick {
                    outcome: Intro2Type26SchedulerProductionOutcome::Primary {
                        entity_id,
                        visit,
                        tertiary,
                    },
                    retained_owner: Some(owner),
                },
                Err(_) => Intro2Type26SchedulerOwnerTick {
                    outcome: Intro2Type26SchedulerProductionOutcome::Dropped {
                        entity_id,
                        reason: Intro2Type26SchedulerProductionDrop::GraphMismatch,
                    },
                    retained_owner: None,
                },
            }
        }
        Err(
            Intro2Type26PrimaryVisitBlock::EntityIdentityMismatch
            | Intro2Type26PrimaryVisitBlock::WanderVisitUnavailable,
        ) => Intro2Type26SchedulerOwnerTick {
            outcome: Intro2Type26SchedulerProductionOutcome::Dropped {
                entity_id,
                reason: Intro2Type26SchedulerProductionDrop::GraphMismatch,
            },
            retained_owner: None,
        },
    }
}

fn intro2_type26_primary_graph_authenticates(entity: &Entity) -> bool {
    entity.active
        && entity.entity_type == INTRO2_DEFECATE_VIRUS_ENTITY_TYPE
        && entity
            .authored_spawn_index
            .and_then(intro2_type26_seed_for_spawn)
            .is_some()
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::DefecateVirusWander(_))
        )
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Tertiary),
            Some(ActorTaskRuntime::DefecateVirusTerrain(_))
        )
        && entity.intro2_type26_sub_d_frame_owner.is_some()
        && entity.intro2_type26_sub_d_runtime.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_task_dispatcher::ActorTaskRuntime;
    use crate::common_mover::sub_d::Type9SubDRuntime;
    use crate::common_mover::SubAPropulsionRuntime;
    use crate::defecate_virus::DEFECATE_VIRUS_WANDER_LIFETIME_MS;
    use crate::defecate_virus_owner::DEFECATE_VIRUS_TERRAIN_MODE;
    use crate::entity::EntityKind;
    use crate::entity_collision_state::EntityInitializerSpec;

    fn metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [INTRO2_DEFECATE_VIRUS_MODEL_ID as u16; 4],
            mass_raw: 400,
            capability_flags: 8,
            initial_health_raw: Some(5_000),
            terrain_contact_task_lifetime_ms: RetailRuntimeValue::Known(67),
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_A)),
            sub_b_lateral_descriptor: RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_B)),
            sub_c_lift_descriptor: RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_C)),
            common_mover_topology: RetailRuntimeValue::Known(INTRO2_TYPE26_COMMON_MOVER_TOPOLOGY),
            sub_d_steering_descriptor: RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_D)),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: 0x0439,
                common_axis_descriptor: CommonAxisDescriptor {
                    strict_axis_limit_raw: 0x0F00,
                    raw_word_at_0x04: 0x0225,
                },
                behavior_choices: INTRO2_TYPE26_BEHAVIOR_CHOICES.to_vec().into_boxed_slice(),
                behavior_rule_ref: 1,
                alternate_behavior_class_ref: 12,
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn entity() -> Entity {
        let mut entity = Entity::unresolved_port_entity(
            77,
            EntityKind::Unknown(INTRO2_DEFECATE_VIRUS_ENTITY_TYPE),
            INTRO2_DEFECATE_VIRUS_ENTITY_TYPE,
        );
        entity.authored_spawn_index = Some(INTRO2_DEFECATE_VIRUS_SPAWN_INDEX);
        entity.model_slots = [Some(INTRO2_DEFECATE_VIRUS_MODEL_ID); 4];
        entity.model_index = Some(INTRO2_DEFECATE_VIRUS_MODEL_ID);
        entity.position = [194.0, 0.0, 14.0];
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Unresolved, 1, 100),
        ));
        entity
    }

    #[test]
    fn view_detail_publish_rejects_type13() {
        let mut type13 = entity();
        type13.entity_type = 13;
        assert_eq!(
            publish_intro2_type26_view_detail(
                &mut type13,
                RetailViewDetailContext::from_raw([0, 0, 0], 0, (52, 30)),
            ),
            Err(Intro2Type26ViewDetailBlock::EntityIdentityMismatch)
        );
    }

    #[test]
    fn captured_receipt_publishes_exact_selection_slots_and_sub_a_reset() {
        let mut entity = entity();
        let admission = publish_intro2_type26_defecate_virus(&mut entity, &metadata())
            .expect("exact capture fixture");

        assert_eq!(admission.selection().choice_index, 2);
        assert_eq!(admission.selection().program.class_id, 4);
        assert_eq!(admission.captured_retail_handle(), 0x047E_0001);
        assert_ne!(entity.id, admission.captured_retail_handle());
        assert_eq!(
            admission.selection().program.initial_style.frame_address,
            0x004C_7E88
        );
        assert_eq!(
            entity.initial_behavior,
            RetailRuntimeValue::Known(Some(admission.selection()))
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("captured initializer success must publish its context")
        };
        assert_eq!(context.active_style().style_address(), 0x004C_7E88);
        let Some(ActorTaskRuntime::DefecateVirusWander(wander)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("Primary must own the duration-2000 Wander task")
        };
        assert_eq!(DEFECATE_VIRUS_WANDER_LIFETIME_MS, 2_000);
        assert_eq!(
            wander.private_state().target_position_raw,
            entity.position_raw()
        );
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Secondary), None);
        let Some(ActorTaskRuntime::DefecateVirusTerrain(terrain)) =
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        else {
            panic!("Tertiary must own the mode-5 terrain task")
        };
        assert_eq!(DEFECATE_VIRUS_TERRAIN_MODE, 5);
        assert_eq!(terrain.lifetime_ms(), 67);
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("Sub-A runtime must survive publication")
        };
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(1));
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(sub_a.drive_scale_percent(), 100);
        assert_eq!(
            admission
                .sub_d_frame_owner()
                .classifier_cache()
                .stagger_counter(),
            INTRO2_TYPE26_SPAWN25_SUB_D_SEED
        );
        assert!(admission
            .sub_d_frame_owner()
            .classifier_cache()
            .can_classify());
        assert_eq!(
            entity.intro2_type26_sub_d_frame_owner,
            Some(admission.sub_d_frame_owner())
        );
        assert_eq!(
            entity.intro2_type26_sub_d_runtime,
            Some(Type9SubDRuntime::from_constructor())
        );
    }

    #[test]
    fn spawn10_publishes_be00_seed_and_retail_handle() {
        let mut entity = entity();
        entity.authored_spawn_index = Some(INTRO2_TYPE26_SPAWN10_INDEX);
        entity.position = [-206.0, 0.0, 15.0];
        let admission = publish_intro2_type26_defecate_virus(&mut entity, &metadata())
            .expect("spawn 10 capture fixture");

        assert_eq!(admission.selection().choice_index, 2);
        assert_eq!(admission.selection().program.class_id, 4);
        assert_eq!(
            admission.captured_retail_handle(),
            INTRO2_TYPE26_SPAWN10_RETAIL_HANDLE
        );
        assert_eq!(
            admission
                .sub_d_frame_owner()
                .classifier_cache()
                .stagger_counter(),
            INTRO2_TYPE26_BE00_0F00_SUB_D_SEED
        );
        assert!(intro2_type26_primary_graph_authenticates(&entity));
    }

    #[test]
    fn near_matches_fail_closed_without_mutating_the_task_table() {
        let cases = [
            (
                {
                    let mut entity = entity();
                    entity.authored_spawn_index = Some(11);
                    entity
                },
                metadata(),
            ),
            (entity(), {
                let mut metadata = metadata();
                metadata.terrain_contact_task_lifetime_ms = RetailRuntimeValue::Known(68);
                metadata
            }),
        ];

        for (mut entity, metadata) in cases {
            assert!(publish_intro2_type26_defecate_virus(&mut entity, &metadata).is_err());
            assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
                .into_iter()
                .all(|slot| entity.actor_task_state(slot).is_none()));
            assert_eq!(entity.initial_behavior, RetailRuntimeValue::Unresolved);
            assert_eq!(
                entity.current_behavior_context,
                RetailRuntimeValue::Unresolved
            );
        }
    }
}
