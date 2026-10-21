//! Native Type83: 104B0/D4A0, AC60 -> C560, and 12DA0's visit of 405610.
//!
//! The 14 authored rows (worlds 30, 39, 40, 42, 46 and 47) carry Sub-K alone.
//! C560 zeroes the Euler words, clears Secondary and Tertiary and installs
//! 405540's lifetime-0 Primary; the birth suffix then builds the identity
//! basis. Class60's style has zero +34/+38, so `+C0` 0x25021 is also the
//! effective word: DCA0 and E870 both walk A800 (0x2000 clear), keep the basis
//! (0x4000), apply E100 gravity (0x4 clear) without wind, ground snap or water,
//! and only decay E370's timer. Retail's world-30 radar keeps its birth pose:
//! its state never gains master motion 0x40000.

use super::{
    destroy_portable_radar, tick_portable_radar, PortableRadarFrame, PortableRadarTickOutcome,
};
use crate::entity::NativeCreationRecord;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskVisit, PreparedActorTask},
    common_mover::{
        type9_attitude::Type9BodyBasis,
        type9_surface::{decay_actor_surface_timer_ms, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT},
        type9_tail::{plan_common_master_motion, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK},
    },
    damage::DamageProfile,
    entity::{
        apply_common_gravity_and_underwater_raw, commit_common_master_motion,
        CommonUnderwaterFrame, Entity, EntityManager,
    },
    entity_behavior::{
        audited_behavior_program, behavior_program, initial_behavior_state_policy,
        select_initial_behavior, BehaviorChoiceListSource, BehaviorContextRuntime,
        BehaviorDescriptorIdentity, BehaviorSelection, BehaviorWeightRule,
    },
    entity_collision_state::{
        CommonMoverComponentTopology, CommonMoverGklPayloads, CommonWorldEffectProfile,
        EntityTypeRuntimeMetadata, RetailRuntimeValue, DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
        SURFACE_STATE_MASK,
    },
    entity_scheduler::*,
    gameplay_radar::TerrainRadarError,
    main_base_abort::MainBaseAbortActorLease,
    resource_cache::{RadarCoverage, ResourceCache},
    world_fx::WorldFx,
};
use v2k_formats::{
    collision::{BehaviorChoice, CommonAxisDescriptor},
    terrain::TerrainGrid,
};

pub const ENTITY_TYPE: u32 = 83;
pub const MODEL: usize = 79;
pub const MASS: u16 = 50;
pub const HEALTH: i32 = 30_000;
pub const CAPABILITY: u32 = 0x1004;
/// Section-12 `+C0`: D4A0 ground (0x20), terrain lane (0x01), a scheduler
/// callback (0x20000), retained basis (0x4000) and 0x1000, which DCA0 reads
/// only alongside 0x2000.
pub const DEFAULT_FLAGS: u32 = 0x0002_5021;
pub const AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 512,
    raw_word_at_0x04: 8,
};
/// Class60's only style: attach C690, release D1C0, initializer C560.
pub const STYLE: u32 = 0x004c_7300;
pub const DEATH_SOUND: u16 = 62;
pub const ALTERNATE_BEHAVIOR_CLASS: u32 = 49;
const BEHAVIOR_CLASS: u32 = 60;
pub const DAMAGE: DamageProfile = DamageProfile {
    thresholds_raw: [0, 5000, 1000, 1000, 200, 0, 0],
    multipliers_q8: [0, 256, 256, 256, 0, 0, 0],
};
pub const INITIAL_CHOICES: [BehaviorChoice; 1] = [BehaviorChoice {
    weight_rule_id: 1,
    weight_multiplier: 1,
    behavior_class_id: 60,
}];

/// The 104B0 receipt and the entity-owned Sub-K word 1 (model selector 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortableRadarRuntime {
    pub(crate) allocation: MainBaseAbortActorLease,
    /// 405610 and 4055C0 share this word; no task keeps a copy.
    pub(crate) sub_k1: u16,
    /// A visit committed its 12DA0 prefix or K1 write, then stopped.
    pub(crate) pending_prefix: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortableRadarError {
    Identity,
    Metadata,
    AlreadyPublished,
    Selection,
    Allocation,
    Graph,
    Runtime(&'static str),
    Coverage(TerrainRadarError),
}

pub fn authenticate_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), PortableRadarError> {
    use RetailRuntimeValue::Known;
    let Some(initializer) = &metadata.initializer else {
        return Err(PortableRadarError::Metadata);
    };
    if metadata.model_slots != [MODEL as u16; 4]
        || metadata.model_variable_count_raw != Known(1)
        || metadata.mass_raw != MASS
        || metadata.capability_flags != CAPABILITY
        || metadata.initial_health_raw != Some(HEALTH)
        || metadata.damage_profile != Some(DAMAGE)
        || metadata.common_mover_topology
            != Known(CommonMoverComponentTopology {
                sub_k: true,
                ..CommonMoverComponentTopology::default()
            })
        || metadata.common_mover_gkl_payloads
            != Known(CommonMoverGklPayloads {
                sub_g: None,
                sub_k: Some([1, 0]),
                sub_l: None,
            })
        || metadata.sub_a_propulsion_descriptor != Known(None)
        || metadata.sub_b_lateral_descriptor != Known(None)
        || metadata.sub_c_lift_descriptor != Known(None)
        || metadata.sub_d_steering_descriptor != Known(None)
        || metadata.sub_f_swimming_descriptor != Known(None)
        || metadata.projectile_emitter_descriptor != Known(None)
        || metadata.actor_animation_descriptor != Known(None)
        || metadata.sub_h_external_frame_descriptor != Known(None)
        || metadata.sub_j_attachment_descriptor != Known(None)
        || metadata.status_component_descriptor != Known(None)
        || metadata.sub_n_payload.is_some()
        || metadata.constructor_sound_attachment_id != Known(None)
        || metadata.accepted_hit_presentation_sound_id != Known(None)
        || metadata.infected_model_presentation_sound_id != Known(None)
        || metadata.generic_hit_sound_id != Known(None)
        || metadata.death_sound_id != Known(Some(DEATH_SOUND))
        || metadata.target_warning_sound_id != Known(None)
        || metadata.terrain_contact_task_lifetime_ms != Known(0)
        || metadata.common_world_effects != Known(CommonWorldEffectProfile::default())
        || initializer.initializer_state_flags_raw != DEFAULT_FLAGS
        || initializer.common_axis_descriptor != AXIS
        || initializer.behavior_choices.as_ref() != INITIAL_CHOICES
        || initializer.behavior_rule_ref != 1
        || initializer.alternate_behavior_class_ref != ALTERNATE_BEHAVIOR_CLASS
    {
        return Err(PortableRadarError::Metadata);
    }
    Ok(())
}

pub(crate) fn allocation_authenticates(entity: &Entity) -> bool {
    entity.portable_radar_runtime.is_some_and(|runtime| {
        entity.active
            && runtime.allocation.entity_id == entity.id
            && entity.entity_type == ENTITY_TYPE
            && entity.model_slots == [Some(MODEL); 4]
    })
}

/// The receipt must also match its issuing manager generation.
pub(crate) fn manager_allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    manager.iter_all().any(|entity| {
        entity.id == id
            && allocation_authenticates(entity)
            && manager
                .main_base_abort_actor_observation(id)
                .zip(entity.portable_radar_runtime)
                .is_some_and(|(observation, runtime)| observation.lease == runtime.allocation)
    })
}

/// Ordinary 104B0 publication of one Type83 creation record: an authored row
/// or 451C00's zeroed campaign-cargo record.
pub(crate) struct PortableRadarConstruction<'a> {
    pub entity: &'a mut Entity,
    pub allocation: MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub record: NativeCreationRecord,
    pub terrain: &'a TerrainGrid,
    pub constructor_surface_bits: u32,
}

pub(crate) fn publish_portable_radar(
    request: PortableRadarConstruction<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), PortableRadarError> {
    let PortableRadarConstruction {
        entity,
        allocation,
        metadata,
        record,
        terrain,
        constructor_surface_bits,
    } = request;
    let [x, _, z] = record.position_raw;
    // D4A0's +C0 bit20 grounds at the record's X/Z; bit40 is absent.
    let grounded = [x, terrain.bilinear_height_raw(x, z), z];
    if !entity.active
        || entity.entity_type != ENTITY_TYPE
        || record.entity_type != ENTITY_TYPE
        || entity.id != allocation.entity_id
        || entity.authored_spawn_index != record.authored_spawn_index
        || entity.model_slots != [Some(MODEL); 4]
        || record
            .model_overrides
            .iter()
            .any(|&slot| slot != 0 && slot as usize != MODEL)
        || ![record.position_raw, grounded].contains(&entity.position_raw())
        || entity.rotation_heading_pitch_roll_raw() != record.rotation.map(|word| word as i16)
        || !record.plain
    {
        return Err(PortableRadarError::Identity);
    }
    authenticate_metadata(metadata)?;
    authenticate_birth_storage(entity, record.initial_damage_buffer_raw)?;
    if constructor_surface_bits & !SURFACE_STATE_MASK != 0 {
        return Err(PortableRadarError::Identity);
    }
    entity.set_position_raw(grounded);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(SURFACE_STATE_MASK, constructor_surface_bits);
    // Common type-vtable+30 and 104B0's initial +44 modifier are null.
    entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Known(None);
    entity
        .collision
        .pair_callbacks
        .damage_modifier_identity_context_empty = RetailRuntimeValue::Known(true);
    entity.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Known(None);
    // 09A80 zeroes the one-word model bank; Sub-K's payload starts K1 at 0.
    entity.portable_radar_runtime = Some(PortableRadarRuntime {
        allocation,
        sub_k1: 0,
        pending_prefix: false,
    });
    // Explicit native policy for 104B0's unwritten transient B2 heap residue.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    let selection = select(next_random)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(PortableRadarError::Selection)?;
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    publish_context(entity, context);
    c560_prefix(entity);
    store_primary(entity);
    // The birth suffix (D720/13F70) builds the basis from C560's zero Euler.
    entity.apply_d720_euler_body_basis();
    Ok(())
}

fn authenticate_birth_storage(
    entity: &Entity,
    damage_buffer_raw: i32,
) -> Result<(), PortableRadarError> {
    use RetailRuntimeValue::Known;
    if entity.portable_radar_runtime.is_some()
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .any(|slot| entity.actor_task_state(slot).is_some())
    {
        return Err(PortableRadarError::AlreadyPublished);
    }
    let expected = BehaviorSelection {
        choice_index: 0,
        program: behavior_program(BEHAVIOR_CLASS).ok_or(PortableRadarError::Selection)?,
    };
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    if entity.initial_behavior != Known(Some(expected))
        || entity.actor_common_axis_descriptor != Known(AXIS)
        || entity.physical_body_basis_q31
            != Known(Type9BodyBasis::from_angle_words(heading, pitch, roll))
        || entity.collision.default_state_flags_at_0xc8 != Known(DEFAULT_FLAGS)
        || entity.collision.health_raw != Known(HEALTH)
        || entity.collision.pre_health_damage_buffer_raw != Known(damage_buffer_raw)
        || entity.collision.last_hit_presentation_tick_at_0x34 != Known(0)
        || entity.capability_flags != CAPABILITY
        || entity.mass_raw != MASS
    {
        return Err(PortableRadarError::AlreadyPublished);
    }
    Ok(())
}

/// 25680 consumes a word even for the singleton Always list.
fn select(next_random: &mut impl FnMut() -> u32) -> Result<BehaviorSelection, PortableRadarError> {
    select_initial_behavior(
        &INITIAL_CHOICES,
        |rule| i32::from(rule == BehaviorWeightRule::Always),
        next_random,
    )
    .map_err(|_| PortableRadarError::Selection)?
    .ok_or(PortableRadarError::Selection)
}

fn publish_context(entity: &mut Entity, context: BehaviorContextRuntime) {
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(behavior_program(BEHAVIOR_CLASS).expect("class60"));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
}

/// C560's prefix: zero +A2/+A4/+A6, then clear Secondary and Tertiary.
fn c560_prefix(entity: &mut Entity) {
    entity.set_rotation_heading_pitch_roll_raw([0; 3]);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
}

/// 405540 -> 406030: no optional component, so 406070 draws nothing.
fn store_primary(entity: &mut Entity) {
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Primary,
        PreparedActorTask::new(ActorTaskRuntime::PortableRadar),
    );
}

/// The retiring Primary's 4055C0, before its slot is cleared or replaced:
/// a saturated K1 removes the footprint at the current position. A coverage
/// failure keeps both the task and K1.
pub(crate) fn retire_primary(
    entity: &mut Entity,
    coverage: &mut impl RadarCoverage,
    next_random: &mut impl FnMut() -> u16,
) -> Result<usize, PortableRadarError> {
    let Some(primary) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) else {
        return Ok(0);
    };
    if !matches!(
        entity.actor_tasks.task_state(primary),
        Some(ActorTaskRuntime::PortableRadar)
    ) {
        return Err(PortableRadarError::Graph);
    }
    let position = entity.position_raw();
    let runtime = entity
        .portable_radar_runtime
        .as_mut()
        .ok_or(PortableRadarError::Allocation)?;
    destroy_portable_radar(&mut runtime.sub_k1, position, coverage, next_random)
        .map_err(PortableRadarError::Coverage)
}

/// A860 for a Portable Radar: Primary's 4055C0, then all three slots.
pub(crate) fn clear_all_tasks(
    entity: &mut Entity,
    coverage: &mut impl RadarCoverage,
    next_random: &mut impl FnMut() -> u16,
) -> Result<usize, PortableRadarError> {
    let draws = retire_primary(entity, coverage, next_random)?;
    for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        entity.actor_tasks.clear_slot(slot);
    }
    Ok(draws)
}

/// AC60 -> C560 from a living relation callback (C690 attach or D1C0
/// release): the singleton selector word, C560's prefix, then 40A7A0, which
/// runs the old Primary's destructor before storing the replacement.
pub(crate) fn reselect(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    coverage: &mut impl RadarCoverage,
    world_fx: &mut WorldFx,
) -> Result<usize, PortableRadarError> {
    authenticate_metadata(metadata)?;
    if !allocation_authenticates(entity)
        || entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
            != RetailRuntimeValue::Known(0)
    {
        return Err(PortableRadarError::Graph);
    }
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(PortableRadarError::Graph);
    };
    let program = audited_behavior_program(BEHAVIOR_CLASS).ok_or(PortableRadarError::Graph)?;
    let context = previous
        .reselect_named_type_default(program, 0, program.initial_style)
        .ok_or(PortableRadarError::Graph)?;
    select(&mut || u32::from(world_fx.next_shared_retail_random_u16()))?;
    publish_context(entity, context);
    c560_prefix(entity);
    let draws = retire_primary(entity, coverage, &mut || {
        world_fx.next_shared_retail_random_u16()
    })?;
    store_primary(entity);
    Ok(draws)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortableRadarOwner {
    allocation: MainBaseAbortActorLease,
    context: BehaviorContextRuntime,
    primary: ActorTaskId,
}

impl PortableRadarOwner {
    pub const fn entity_id(self) -> u32 {
        self.allocation.entity_id
    }
    pub const fn allocation(self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub(crate) const fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }

    pub fn adopt(manager: &EntityManager, id: u32) -> Result<Self, PortableRadarError> {
        if !manager_allocation_authenticates(manager, id) {
            return Err(PortableRadarError::Allocation);
        }
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(PortableRadarError::Allocation)?;
        let runtime = entity
            .portable_radar_runtime
            .ok_or(PortableRadarError::Allocation)?;
        if runtime.pending_prefix {
            return Err(PortableRadarError::Runtime(
                "committed Portable Radar prefix",
            ));
        }
        let program = audited_behavior_program(BEHAVIOR_CLASS).ok_or(PortableRadarError::Graph)?;
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(PortableRadarError::Graph);
        };
        if context.descriptor() != BehaviorDescriptorIdentity::Named(program)
            || context.active_style().style_address() != STYLE
            || context.choice_list_source()
                != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
            || context.target_handle_at_0x08() != RetailRuntimeValue::Known(None)
            || context.auxiliary_word_at_0x0c() != RetailRuntimeValue::Known(0)
            || bits(entity, DYING_STATE_BIT)? != 0
        {
            return Err(PortableRadarError::Graph);
        }
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .ok_or(PortableRadarError::Graph)?;
        let wrapper = entity
            .actor_tasks
            .wrapper_flags(primary)
            .ok_or(PortableRadarError::Graph)?;
        if !wrapper.alive
            || wrapper.in_callback
            || !matches!(
                entity.actor_tasks.task_state(primary),
                Some(ActorTaskRuntime::PortableRadar)
            )
            || entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Secondary)
                .is_some()
            || entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Tertiary)
                .is_some()
        {
            return Err(PortableRadarError::Graph);
        }
        Ok(Self {
            allocation: runtime.allocation,
            context,
            primary,
        })
    }
}

pub struct PortableRadarActorFrame<'a> {
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortableRadarActorOutcome {
    Waiting {
        entity_id: u32,
    },
    Advanced {
        entity_id: u32,
        callback_enabled: bool,
        detailed: bool,
        callback_elapsed_micros: u32,
        radar: Option<PortableRadarTickOutcome>,
    },
    Blocked {
        entity_id: u32,
        reason: PortableRadarError,
        prefix_committed: bool,
    },
    Pending {
        entity_id: u32,
    },
    Dropped {
        entity_id: u32,
    },
}

impl PortableRadarActorOutcome {
    pub const fn entity_id(self) -> u32 {
        match self {
            Self::Waiting { entity_id }
            | Self::Advanced { entity_id, .. }
            | Self::Blocked { entity_id, .. }
            | Self::Pending { entity_id }
            | Self::Dropped { entity_id } => entity_id,
        }
    }
}

pub struct PortableRadarActorTick {
    pub outcome: PortableRadarActorOutcome,
    pub retained_owner: Option<PortableRadarOwner>,
}

pub(crate) fn has_pending_prefix(manager: &EntityManager, id: u32) -> bool {
    manager
        .iter_all()
        .find(|entity| entity.id == id)
        .and_then(|entity| entity.portable_radar_runtime)
        .is_some_and(|runtime| runtime.pending_prefix)
}

fn set_pending_prefix(manager: &mut EntityManager, id: u32, pending: bool) -> bool {
    let Some(runtime) = manager
        .entity_mut(id)
        .and_then(|entity| entity.portable_radar_runtime.as_mut())
    else {
        return false;
    };
    runtime.pending_prefix = pending;
    true
}

pub fn tick_portable_radar_owner(
    manager: &mut EntityManager,
    owner: PortableRadarOwner,
    mut frame: PortableRadarActorFrame<'_>,
) -> PortableRadarActorTick {
    let id = owner.entity_id();
    if !manager_allocation_authenticates(manager, id)
        || !manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|now| now.lease == owner.allocation)
    {
        return PortableRadarActorTick {
            outcome: PortableRadarActorOutcome::Dropped { entity_id: id },
            retained_owner: None,
        };
    }
    if has_pending_prefix(manager, id) {
        return PortableRadarActorTick {
            outcome: PortableRadarActorOutcome::Pending { entity_id: id },
            retained_owner: Some(owner),
        };
    }
    if PortableRadarOwner::adopt(manager, id) != Ok(owner) {
        return PortableRadarActorTick {
            outcome: PortableRadarActorOutcome::Dropped { entity_id: id },
            retained_owner: None,
        };
    }
    match run_frame(manager, owner, &mut frame) {
        Ok(outcome) => {
            set_pending_prefix(manager, id, false);
            PortableRadarActorTick {
                outcome,
                retained_owner: PortableRadarOwner::adopt(manager, id).ok(),
            }
        }
        Err(reason) => PortableRadarActorTick {
            outcome: PortableRadarActorOutcome::Blocked {
                entity_id: id,
                reason,
                prefix_committed: has_pending_prefix(manager, id),
            },
            retained_owner: Some(owner),
        },
    }
}

fn bits(entity: &Entity, mask: u32) -> Result<u32, PortableRadarError> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(PortableRadarError::Runtime("state bits")),
    }
}

fn run_frame(
    manager: &mut EntityManager,
    owner: PortableRadarOwner,
    frame: &mut PortableRadarActorFrame<'_>,
) -> Result<PortableRadarActorOutcome, PortableRadarError> {
    let id = owner.entity_id();
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(PortableRadarError::Allocation)?;
    let metadata = manager
        .type_runtime_metadata(ENTITY_TYPE)
        .cloned()
        .ok_or(PortableRadarError::Metadata)?;
    authenticate_metadata(&metadata)?;
    let state = bits(
        entity,
        REMOTE_OWNED_STATE_BIT
            | 0x1000
            | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    )?;
    if state & REMOTE_OWNED_STATE_BIT != 0 {
        return Err(PortableRadarError::Runtime("remote-owned Portable Radar"));
    }
    let callback_enabled = state & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT != 0;
    let detailed = state & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0;
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
            u32::from(frame.world_fx.next_shared_retail_random_u16())
        })
    else {
        return Err(PortableRadarError::Runtime("scheduler state"));
    };
    if !set_pending_prefix(manager, id, true) {
        return Err(PortableRadarError::Allocation);
    }
    let entity = manager
        .entity_mut(id)
        .ok_or(PortableRadarError::Allocation)?;
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us: dt,
    } = prefix.flow
    else {
        return Ok(PortableRadarActorOutcome::Waiting { entity_id: id });
    };
    // 12DA0 resolves a carried radar's parent after the age write and
    // before the callback-enable branch.
    if state & 0x1000 != 0 {
        resolve_relation_prefix(manager, id)?;
    } else if entity.attached_to.is_some() {
        return Err(PortableRadarError::Runtime("relation state"));
    }
    let mut radar = None;
    if callback_enabled {
        let entity = manager
            .entity_mut(id)
            .ok_or(PortableRadarError::Allocation)?;
        let RetailRuntimeValue::Known(mass) = common_scheduler_callback_mass(
            metadata.mass_raw,
            entity.collision.animation_offset_at_0xb2,
        ) else {
            return Err(PortableRadarError::Runtime("callback B2"));
        };
        entity.mass_raw = mass;
        radar = Some(run_callback(manager, owner, frame, dt, detailed)?);
    }
    let entity = manager
        .entity_mut(id)
        .ok_or(PortableRadarError::Allocation)?;
    commit_common_scheduler_post_callback(&mut entity.collision);
    if entity.collision.constructor_sound_attachment_id_at_0x8c != RetailRuntimeValue::Known(None) {
        return Err(PortableRadarError::Runtime("positional sound attachment"));
    }
    let motion = plan_common_master_motion(
        entity.position_raw(),
        entity.velocity_raw(),
        bits(entity, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)?,
        dt,
    );
    commit_common_master_motion(entity, motion);
    Ok(PortableRadarActorOutcome::Advanced {
        entity_id: id,
        callback_enabled,
        detailed,
        callback_elapsed_micros: dt,
        radar,
    })
}

/// 12DA0's relation repair: a remote parent lends its position and 0x20; a
/// local one must still list the radar in its Sub-J.
fn resolve_relation_prefix(manager: &mut EntityManager, id: u32) -> Result<(), PortableRadarError> {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(PortableRadarError::Allocation)?;
    let parent_id = entity
        .attached_to
        .ok_or(PortableRadarError::Runtime("missing relation handle"))?;
    let parent = manager
        .iter_all()
        .find(|entity| entity.id == parent_id)
        .ok_or(PortableRadarError::Runtime(
            "missing-parent relation repair",
        ))?;
    if bits(parent, REMOTE_OWNED_STATE_BIT)? != 0 {
        let position = parent.position_raw();
        let entity = manager
            .entity_mut(id)
            .ok_or(PortableRadarError::Allocation)?;
        entity.set_position_raw(position);
        entity.collision.state_flags_at_0x08.overwrite(0x20, 0x20);
        return Ok(());
    }
    if !matches!(&parent.sub_j_attachment_runtime, RetailRuntimeValue::Known(Some(sub_j))
        if sub_j.ordered_entity_ids().contains(&id))
    {
        return Err(PortableRadarError::Runtime(
            "missing-member relation release",
        ));
    }
    Ok(())
}

/// DCA0 (detailed) or E870 (coarse): both walk A800 for this profile.
fn run_callback(
    manager: &mut EntityManager,
    owner: PortableRadarOwner,
    frame: &mut PortableRadarActorFrame<'_>,
    dt: u32,
    detailed: bool,
) -> Result<PortableRadarTickOutcome, PortableRadarError> {
    let id = owner.entity_id();
    let program = audited_behavior_program(BEHAVIOR_CLASS).ok_or(PortableRadarError::Graph)?;
    let entity = manager
        .entity_mut(id)
        .ok_or(PortableRadarError::Allocation)?;
    if entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(DEFAULT_FLAGS) {
        return Err(PortableRadarError::Runtime("default environment flags"));
    }
    let effective =
        (DEFAULT_FLAGS | program.authored_enable_policy) & !program.authored_disable_policy;
    // The branch bits DCA0/E870 consume: walk (0x2000 clear, 0x1000 inert),
    // no E640 (0x10), keep basis (0x4000), gravity (0x4 clear), no wind (0x8),
    // no DF70 (0x2) and no infected ground (0x800).
    if effective & (0x3000 | 0x10 | 0x4000 | 4 | 8 | 2 | 0x800) != 0x5000 {
        return Err(PortableRadarError::Runtime("environment callback profile"));
    }
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id: owner.primary,
    };
    entity
        .actor_tasks
        .begin_exact_visit_with(visit, |_| ())
        .ok_or(PortableRadarError::Graph)?;
    let state_flags_raw = bits(entity, u32::MAX)?;
    let position_raw = entity.position_raw();
    let runtime = entity
        .portable_radar_runtime
        .as_mut()
        .ok_or(PortableRadarError::Allocation)?;
    // 405610 commits K1 before its coverage call: a coverage failure leaves
    // the visit's prefix parked rather than retried as a fresh visit.
    let radar = tick_portable_radar(
        &mut runtime.sub_k1,
        PortableRadarFrame {
            state_flags_raw,
            elapsed_micros: dt,
            position_raw,
        },
        frame.resources,
        &mut || frame.world_fx.next_shared_retail_random_u16(),
    )
    .map_err(PortableRadarError::Coverage)?;
    // 405610 always returns zero: the lifetime-0 wrapper survives.
    if !entity.actor_tasks.finish_exact_visit(visit) {
        return Err(PortableRadarError::Graph);
    }
    if detailed {
        let record = frame
            .resources
            .global_entity_type(ENTITY_TYPE as usize)
            .ok_or(PortableRadarError::Metadata)?;
        let RetailRuntimeValue::Known(health_raw) = entity.collision.health_raw else {
            return Err(PortableRadarError::Runtime("health"));
        };
        let visible = bits(entity, 0x800)? != 0;
        if let Some(sound) = crate::actor_detailed_sound::plan_actor_detailed_sound(
            crate::actor_detailed_sound::ActorDetailedSoundFrame {
                type_record: record,
                health_raw,
                visible,
                callback_elapsed_micros: dt,
            },
            &mut || u32::from(frame.world_fx.next_shared_retail_random_u16()),
        ) {
            frame
                .world_fx
                .queue_networked_positional_sound_raw(sound, entity.position_raw());
        }
    }
    // E100: gravity only; Sub-C is absent, so there is no water response.
    let mut velocity = entity.velocity_raw();
    apply_common_gravity_and_underwater_raw(
        &mut velocity,
        dt,
        CommonUnderwaterFrame {
            effective_environment_flags: effective,
            water_response_enabled: false,
            position_y_raw: entity.position_raw()[1],
            solid_or_sea_y_raw: 0,
            self_mass_raw: entity.mass_raw,
            attached_cargo_mass: 0,
        },
    );
    entity.set_velocity_raw(velocity);
    // E370 with both +72/+73 selectors zero only decays the timer.
    if bits(entity, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT)? == 0 {
        let RetailRuntimeValue::Known(timer) = entity.surface_lifetime_timer_ms_at_0x48 else {
            return Err(PortableRadarError::Runtime("surface timer"));
        };
        entity.surface_lifetime_timer_ms_at_0x48 =
            RetailRuntimeValue::Known(decay_actor_surface_timer_ms(timer, dt));
    }
    Ok(radar)
}

/// World teardown: 42E980 queues every actor's deferred removal in live
/// order, then 14990's sweep runs each A860. A deployed radar's 4055C0
/// removes its footprint while the old world's raster and the process RNG
/// are still resident, before resources unload. Returns the raster draws.
pub fn retire_world_radars(
    manager: &mut EntityManager,
    resources: &mut ResourceCache,
    world_fx: &mut WorldFx,
) -> Result<usize, PortableRadarError> {
    let ids: Vec<u32> = manager.retail_live_order_ids().collect();
    let mut draws = 0;
    for id in ids {
        let Some(entity) = manager
            .entity_mut(id)
            .filter(|entity| allocation_authenticates(entity))
        else {
            continue;
        };
        draws += clear_all_tasks(entity, resources, &mut || {
            world_fx.next_shared_retail_random_u16()
        })?;
    }
    Ok(draws)
}
