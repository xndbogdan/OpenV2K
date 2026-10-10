//! Native Rolling Boulder class20 for ordinary Type3 and Type27 bodies.
//!
//! Both rows are component-less: 104B0/09A80 build no mover component, D4A0
//! applies default state `0x4060` (bit20 terrain snap plus bit40 model lift),
//! and the singleton Always1/class20 choice installs style0. The class20
//! descriptor `0x004C8908` owns two `0x48`-byte styles, read at runtime from
//! the faststart04 recording:
//!
//! | Style | `+04` (tag 9C00) | `+18` pair | `+28` impact | `+34`/`+38` | Initializer | Primary |
//! |---|---|---|---|---|---|---|
//! | 0 rolling `4C78A0` | `40C750` -> style1 | null | null | `0`/`1005` | `40B950` | `404580(slot0, 0)` |
//! | 1 resting `4C78E8` | `40C730` -> style0 | `40C730` | `40C730` | `1005`/`0` | `40B9B0` | `404B40(slot0, 0)` |
//!
//! `40EA10` installs a style by applying `+34` through `40D440` and `+38`
//! reversed, so rolling sets `0x10000 | 0x40000` (terrain/water admission and
//! master motion) and resting clears them. Both initializers first clear
//! Tertiary and Secondary through `40A7A0`. A zero task lifetime never times
//! out in `401120`: the rolling task ends only through its stationary 9C00
//! result, and the resting task through a detailed visit at speed above 100.
//! `416410` (state `0x1000`) gates the 9C00 style transition.
//!
//! The effective environment word `(+C8 | style+34) & ~style+38` is `0x4060`
//! while rolling (E100 gravity) and `0x5065` while resting (bit4 suppresses
//! gravity). Neither sets bit8, so 4EC60 drag never runs, and neither sets
//! bit2, so DF70 never snaps the body to the ground.
//!
//! Late 11AD0 contact is shared: [`crate::native_actor_surface_contact`] runs
//! the bare 141D0 terrain/water response while rolling, and
//! [`crate::native_ground_actor::contact`] the null-hook 11760 static response
//! in either style. [`impact`] owns the `10EB0/11250/11320` hits; a lethal
//! Type3 enters the shared class1 terminal and a lethal Type27 the class18
//! split in [`death`]. Pair contacts are not owned. See
//! `docs/re/ROLLING_BOULDER.md`.

use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskVisit, PreparedActorTask};
use crate::common_mover::type9_tail::{
    plan_common_master_motion, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
};
use crate::damage::DamageProfile;
use crate::entity::{
    apply_common_gravity_and_underwater_raw, apply_common_no_wind_drag_raw,
    commit_common_master_motion, CommonEnvironmentPhysics, CommonUnderwaterFrame, Entity,
    EntityManager,
};
use crate::entity_behavior::{
    audited_behavior_program, audited_behavior_style, select_initial_behavior,
    translate_state_policy, BehaviorContextRuntime, BehaviorProgram, BehaviorStyle,
    BehaviorWeightRule,
};
use crate::entity_collision_state::{
    CommonMoverComponentTopology, CommonWorldEffectProfile, EntityTypeRuntimeMetadata,
    RetailRuntimeValue,
};
use crate::entity_initializer::{
    resolve_entity_initializer_with_selected_behavior, EntityInitializerRequest,
    ResourceDomainRelation,
};
use crate::entity_scheduler::{
    commit_common_scheduler_post_callback, commit_common_scheduler_prefix,
    common_scheduler_callback_mass, plan_common_scheduler_prefix, CommonSchedulerPrefixFlow,
};
use crate::intro2_meteors::BoulderRollingTaskState;
use crate::main_base_abort::MainBaseAbortActorLease;
use crate::resource_cache::ResourceCache;
use crate::world_fx::WorldFx;
use v2k_formats::fixed_math::retail_integer_sqrt;
use v2k_formats::levels::EntitySpawn;
use v2k_formats::terrain::TerrainGrid;

pub const ROLLING_BOULDER_CLASS: u8 = 20;
pub const ROLLING_STYLE_ADDRESS: u32 = 0x004C_78A0;
pub const RESTING_STYLE_ADDRESS: u32 = 0x004C_78E8;
/// Style1 `+18` and `+28` select style0 through `40C730`.
pub const RESTING_WAKE_CALLBACK_ADDRESS: u32 = 0x0040_C730;
/// Type `+C0` for both rows: bit20 snap, bit40 model lift, bit4000.
pub const DEFAULT_STATE_POLICY: u32 = 0x4060;
/// The style policy word applied by `40EA10`.
const STYLE_POLICY_WORD: u32 = 0x1005;
/// `404B60` keeps resting while the narrowed speed is at most this value.
pub const RESTING_WAKE_SPEED_RAW: i32 = 100;
const CALLBACK_ENABLED_STATE_BIT: u32 = 0x0002_0000;
const DETAILED_STATE_BIT: u32 = 0x0200_0000;
const ATTACHED_STATE_BIT: u32 = 0x1000;
const BLOCKING_STATE_MASK: u32 = 0x8010_1000;
const TERMINAL_STATE_MASK: u32 = 0x8010_0000;

const EMPTY_TOPOLOGY: CommonMoverComponentTopology = CommonMoverComponentTopology {
    sub_a: false,
    sub_b: false,
    sub_c: false,
    sub_d: false,
    sub_e: false,
    sub_f: false,
    sub_g: false,
    sub_h: false,
    sub_i: false,
    sub_j: false,
    sub_k: false,
    sub_l: false,
    sub_m: false,
    sub_n: false,
    sub_o: false,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollingBoulderProfile {
    /// Type3 small boulder: model648, mass100, health5000, class1 Explode.
    Small,
    /// Type27 boulder: model647, mass200, health10000, class18 Split And
    /// Explode into two Type3 boulders.
    Large,
}

impl RollingBoulderProfile {
    pub const fn for_entity_type(entity_type: u32) -> Option<Self> {
        match entity_type {
            3 => Some(Self::Small),
            27 => Some(Self::Large),
            _ => None,
        }
    }

    pub const fn entity_type(self) -> u32 {
        match self {
            Self::Small => 3,
            Self::Large => 27,
        }
    }

    pub const fn model(self) -> u16 {
        match self {
            Self::Small => 648,
            Self::Large => 647,
        }
    }

    pub const fn mass(self) -> u16 {
        match self {
            Self::Small => 100,
            Self::Large => 200,
        }
    }

    pub const fn health(self) -> i32 {
        match self {
            Self::Small => 5_000,
            Self::Large => 10_000,
        }
    }

    pub const fn damage(self) -> DamageProfile {
        DamageProfile {
            thresholds_raw: [
                0,
                match self {
                    Self::Small => 8_000,
                    Self::Large => 10_000,
                },
                4_000,
                10_000,
                200,
                0,
                0,
            ],
            multipliers_q8: [0, 256, 256, 0, 128, 0, 0],
        }
    }

    /// Section-12 `+0x124`: class1 Explode or class18 Split And Explode.
    pub const fn alternate_behavior_class(self) -> u32 {
        match self {
            Self::Small => 1,
            Self::Large => 18,
        }
    }
}

/// Current class20 style; also the descriptor-local style table index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollingBoulderStyle {
    Rolling,
    Resting,
}

impl RollingBoulderStyle {
    pub const fn table_index(self) -> u32 {
        match self {
            Self::Rolling => 0,
            Self::Resting => 1,
        }
    }

    pub const fn style_address(self) -> u32 {
        match self {
            Self::Rolling => ROLLING_STYLE_ADDRESS,
            Self::Resting => RESTING_STYLE_ADDRESS,
        }
    }

    /// Style `+34`/`+38` words consumed by `40EA10` and the environment.
    pub const fn policy_words(self) -> (u32, u32) {
        match self {
            Self::Rolling => (0, STYLE_POLICY_WORD),
            Self::Resting => (STYLE_POLICY_WORD, 0),
        }
    }

    /// `(+C8 | style+34) & ~style+38`, as read by E100 and D920.
    pub const fn effective_policy(self) -> u32 {
        let (enable, disable) = self.policy_words();
        (DEFAULT_STATE_POLICY | enable) & !disable
    }

    /// The other style, reached through `+04` on a 9C00 result.
    pub const fn on_task_completion(self) -> Self {
        match self {
            Self::Rolling => Self::Resting,
            Self::Resting => Self::Rolling,
        }
    }

    fn from_style_address(address: u32) -> Option<Self> {
        match address {
            ROLLING_STYLE_ADDRESS => Some(Self::Rolling),
            RESTING_STYLE_ADDRESS => Some(Self::Resting),
            _ => None,
        }
    }

    fn audited_style(self) -> Option<BehaviorStyle> {
        audited_behavior_style(u32::from(ROLLING_BOULDER_CLASS), self.table_index() as u8).copied()
    }
}

/// `40EA10`: apply style `+34` through `40D440`, then `+38` reversed.
pub const fn apply_style_install_state(state: u32, style: RollingBoulderStyle) -> u32 {
    let (enable, disable) = style.policy_words();
    let enable = translate_state_policy(enable);
    let disable = translate_state_policy(disable);
    let state = (state | enable.set_bits) & !enable.clear_bits;
    (state | disable.clear_bits) & !disable.set_bits
}

/// `404B40 -> 405F80`'s task: the generic wrapper's millisecond counter only.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BoulderRestingTaskState {
    pub elapsed_ms: u32,
}

impl BoulderRestingTaskState {
    pub const fn new() -> Self {
        Self { elapsed_ms: 0 }
    }

    /// `404B60`. A restricted (coarse) visit returns zero untouched. A
    /// detailed visit returns the 9C00 singleton `4BE198` while the narrowed
    /// `457730` speed exceeds 100; otherwise it zeroes the velocity.
    pub fn step(
        &mut self,
        velocity_raw: &mut [i16; 3],
        elapsed_micros: u32,
        detailed: bool,
    ) -> bool {
        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1_000);
        if !detailed {
            return false;
        }
        let square = |value: i16| i32::from(value).wrapping_mul(i32::from(value));
        let sum = square(velocity_raw[1])
            .wrapping_add(square(velocity_raw[2]))
            .wrapping_add(square(velocity_raw[0]));
        let speed = i32::from(retail_integer_sqrt(sum) as u16 as i16);
        if speed > RESTING_WAKE_SPEED_RAW {
            return true;
        }
        *velocity_raw = [0; 3];
        false
    }
}

/// Construction receipt retained by the allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RollingBoulderRuntime {
    pub(crate) allocation: MainBaseAbortActorLease,
    pub(crate) profile: RollingBoulderProfile,
    /// D4A0's post-grounding `+90` copy.
    pub(crate) anchor_raw: [i16; 3],
    /// Type27's retained class18 split, from `10C10` until the sweep.
    pub(crate) split_terminal: Option<death::RollingBoulderSplitTerminal>,
}

impl RollingBoulderRuntime {
    pub const fn allocation(&self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub const fn profile(&self) -> RollingBoulderProfile {
        self.profile
    }
    pub const fn anchor_raw(&self) -> [i16; 3] {
        self.anchor_raw
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollingBoulderBlock {
    Allocation,
    Identity,
    Metadata,
    Graph,
    AlreadyPublished,
    Model,
    Runtime(&'static str),
}

pub fn authenticate_metadata(
    profile: RollingBoulderProfile,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), RollingBoulderBlock> {
    let Some(init) = &metadata.initializer else {
        return Err(RollingBoulderBlock::Metadata);
    };
    if metadata.model_slots != [profile.model(); 4]
        || metadata.mass_raw != profile.mass()
        || metadata.capability_flags != 0x2000
        || metadata.initial_health_raw != Some(profile.health())
        || metadata.damage_profile != Some(profile.damage())
        || metadata.common_mover_topology != RetailRuntimeValue::Known(EMPTY_TOPOLOGY)
        || metadata.common_world_effects
            != RetailRuntimeValue::Known(CommonWorldEffectProfile {
                surface_selectors: [0; 2],
                surface_lifetime_ms: 0,
                low_health_effect_words: [0; 3],
            })
        || metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
        || metadata.death_sound_id != RetailRuntimeValue::Known(None)
        || metadata.generic_hit_sound_id != RetailRuntimeValue::Known(None)
        || metadata.accepted_hit_presentation_sound_id != RetailRuntimeValue::Known(None)
        || metadata.terrain_contact_task_lifetime_ms != RetailRuntimeValue::Known(0)
        || metadata.model_variable_count_raw != RetailRuntimeValue::Known(0)
        || init.initializer_state_flags_raw != DEFAULT_STATE_POLICY
        || init.behavior_rule_ref != 3
        || init.alternate_behavior_class_ref != profile.alternate_behavior_class()
        || init.behavior_choices.len() != 1
        || init.behavior_choices[0].weight_rule_id != 1
        || init.behavior_choices[0].weight_multiplier != 1
        || init.behavior_choices[0].behavior_class_id != u32::from(ROLLING_BOULDER_CLASS)
    {
        return Err(RollingBoulderBlock::Metadata);
    }
    Ok(())
}

fn rolling_boulder_program() -> Result<&'static BehaviorProgram, RollingBoulderBlock> {
    audited_behavior_program(u32::from(ROLLING_BOULDER_CLASS)).ok_or(RollingBoulderBlock::Metadata)
}

/// Both rows keep the shared type vtable `4C8A30`, whose `+30` generic-hit
/// slot is null; `05FF0` leaves each Primary's `+18` component hook null and
/// 104B0 the instance `+44` modifier. The pair orientation is not owned yet.
fn class20_pair_callbacks() -> crate::entity_collision_state::EntityPairCallbackRuntimeState {
    crate::entity_collision_state::EntityPairCallbackRuntimeState::audited_local(
        None,
        RetailRuntimeValue::Unresolved,
    )
}

/// Ordinary 104B0 construction of one authored boulder.
pub(crate) struct RollingBoulderAuthoredConstruction<'a> {
    pub entity: &'a mut Entity,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub allocation: MainBaseAbortActorLease,
    pub spawn: &'a EntitySpawn,
    pub terrain: &'a TerrainGrid,
    pub model_extent_raw: Option<u16>,
    pub constructor_surface_bits: u32,
}

/// `104B0 -> 09A80 -> D4A0 -> AC60 -> 40B950`.
pub(crate) fn publish_authored_rolling_boulder(
    request: RollingBoulderAuthoredConstruction<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<RollingBoulderRuntime, RollingBoulderBlock> {
    let RollingBoulderAuthoredConstruction {
        entity,
        metadata,
        allocation,
        spawn,
        terrain,
        model_extent_raw,
        constructor_surface_bits,
    } = request;
    let profile = RollingBoulderProfile::for_entity_type(entity.entity_type)
        .ok_or(RollingBoulderBlock::Identity)?;
    authenticate_metadata(profile, metadata)?;
    if spawn.entity_type != entity.entity_type
        || allocation.entity_id != entity.id
        || entity.authored_spawn_index != Some(spawn.index)
        || entity.rotation_heading_pitch_roll_raw() != spawn.rotation.map(|value| value as i16)
        || spawn.model_overrides != [0; 4]
        || spawn.has_animation
        || spawn.animation.is_some()
        || spawn.has_config
        || spawn.config.is_some()
    {
        return Err(RollingBoulderBlock::Identity);
    }
    if entity.rolling_boulder_runtime.is_some()
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .any(|slot| entity.actor_task_state(slot).is_some())
    {
        return Err(RollingBoulderBlock::AlreadyPublished);
    }
    if constructor_surface_bits & !crate::entity_collision_state::SURFACE_STATE_MASK != 0 {
        return Err(RollingBoulderBlock::Runtime("constructor surface"));
    }
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(RollingBoulderBlock::Metadata)?;
    let program = rolling_boulder_program()?;
    let selection = crate::entity_behavior::BehaviorSelection {
        choice_index: 0,
        program,
    };
    let resolution = resolve_entity_initializer_with_selected_behavior(
        EntityInitializerRequest {
            metadata: Some(metadata),
            spawn_param: spawn.param,
            authored_position_raw: spawn.position_raw(),
            terrain: Some(terrain),
            resource_domain: ResourceDomainRelation::Current,
        },
        Some(selection),
    );
    let mut state = resolution.state_flags;
    state.overwrite(
        crate::entity_initializer::CONSTRUCTOR_SURFACE_STATE_MASK,
        constructor_surface_bits,
    );
    // D4A0 bit20 grounds on bilinear terrain; nested bit40 adds the selected
    // active model's header+08.
    let extent = model_extent_raw.ok_or(RollingBoulderBlock::Model)?;
    let mut grounded = spawn.position_raw();
    grounded[1] = terrain
        .bilinear_height_raw(grounded[0], grounded[2])
        .wrapping_add(extent as i16);
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(RollingBoulderBlock::Metadata)?;
    // Every fallible lookup precedes the singleton selector's one RNG word.
    let selected = select_initial_behavior(
        &initializer.behavior_choices,
        |rule| i32::from(rule == BehaviorWeightRule::Always),
        next_random,
    )
    .map_err(|_| RollingBoulderBlock::Metadata)?;
    if selected != Some(selection) {
        return Err(RollingBoulderBlock::Metadata);
    }
    entity.set_position_raw(grounded);
    entity.collision.state_flags_at_0x08 = state;
    entity.collision.pair_callbacks = class20_pair_callbacks();
    // Explicit native policy for 104B0's unwritten transient mass word.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.apply_d720_euler_body_basis();
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    install_style_tasks(entity, RollingBoulderStyle::Rolling);
    let runtime = RollingBoulderRuntime {
        allocation,
        profile,
        anchor_raw: grounded,
        split_terminal: None,
    };
    entity.rolling_boulder_runtime = Some(runtime);
    Ok(runtime)
}

/// A class18 split child after `438080 -> 104B0 -> D4A0`: AC60's singleton
/// choice, then `40EA10`/`40B950` install style0 as for an authored boulder.
/// The caller links the allocation and runs D720 afterwards.
pub(crate) fn publish_split_rolling_boulder(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    profile: RollingBoulderProfile,
    allocation: MainBaseAbortActorLease,
    anchor_raw: [i16; 3],
    next_random: &mut impl FnMut() -> u32,
) -> Result<RollingBoulderRuntime, RollingBoulderBlock> {
    authenticate_metadata(profile, metadata)?;
    if entity.entity_type != profile.entity_type() || allocation.entity_id != entity.id {
        return Err(RollingBoulderBlock::Identity);
    }
    if entity.rolling_boulder_runtime.is_some()
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .any(|slot| entity.actor_task_state(slot).is_some())
    {
        return Err(RollingBoulderBlock::AlreadyPublished);
    }
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(RollingBoulderBlock::Metadata)?;
    let program = rolling_boulder_program()?;
    let selection = crate::entity_behavior::BehaviorSelection {
        choice_index: 0,
        program,
    };
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(RollingBoulderBlock::Metadata)?;
    let RetailRuntimeValue::Known(state) = entity.collision.state_flags_at_0x08.masked(u32::MAX)
    else {
        return Err(RollingBoulderBlock::Runtime("birth state"));
    };
    // Every fallible lookup precedes the singleton selector's one RNG word.
    let selected = select_initial_behavior(
        &initializer.behavior_choices,
        |rule| i32::from(rule == BehaviorWeightRule::Always),
        next_random,
    )
    .map_err(|_| RollingBoulderBlock::Metadata)?;
    if selected != Some(selection) {
        return Err(RollingBoulderBlock::Metadata);
    }
    entity.set_position_raw(anchor_raw);
    entity.collision.state_flags_at_0x08.overwrite(
        u32::MAX,
        apply_style_install_state(state, RollingBoulderStyle::Rolling),
    );
    entity.collision.pair_callbacks = class20_pair_callbacks();
    // Explicit native policy for 104B0's unwritten transient mass word.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    install_style_tasks(entity, RollingBoulderStyle::Rolling);
    let runtime = RollingBoulderRuntime {
        allocation,
        profile,
        anchor_raw,
        split_terminal: None,
    };
    entity.rolling_boulder_runtime = Some(runtime);
    Ok(runtime)
}

/// The class20 initializers: `40A7A0` clears Tertiary and Secondary, then
/// `404580`/`404B40` publish the new Primary. Neither consumes RNG for a
/// component-less body.
fn install_style_tasks(entity: &mut Entity, style: RollingBoulderStyle) {
    entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
    let primary = match style {
        RollingBoulderStyle::Rolling => {
            ActorTaskRuntime::BoulderRolling(BoulderRollingTaskState::new(entity.position_raw()))
        }
        RollingBoulderStyle::Resting => {
            ActorTaskRuntime::BoulderResting(BoulderRestingTaskState::new())
        }
    };
    entity
        .actor_tasks
        .replace_prepared(ActorTaskSlot::Primary, PreparedActorTask::new(primary));
}

/// `40C750`/`40C730 -> 40C6B0`: replace the style within the same context,
/// apply `40EA10`'s state edit, then run the style's initializer.
fn switch_style(
    entity: &mut Entity,
    style: RollingBoulderStyle,
) -> Result<(), RollingBoulderBlock> {
    let program = rolling_boulder_program()?;
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(RollingBoulderBlock::Graph);
    };
    let audited = style.audited_style().ok_or(RollingBoulderBlock::Metadata)?;
    let replaced = BehaviorContextRuntime::named_audited(
        program,
        style.table_index(),
        context.choice_list_source(),
        context.target_handle_at_0x08(),
        context.auxiliary_word_at_0x0c(),
        audited,
    )
    .ok_or(RollingBoulderBlock::Metadata)?;
    let RetailRuntimeValue::Known(state) = entity.collision.state_flags_at_0x08.masked(u32::MAX)
    else {
        return Err(RollingBoulderBlock::Runtime("style install state"));
    };
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(replaced));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, apply_style_install_state(state, style));
    install_style_tasks(entity, style);
    Ok(())
}

/// Wake a resting boulder through style1's `+18` pair or `+28` impact hook.
/// Style0's slots are null, so a rolling boulder is left unchanged.
pub fn wake_resting_boulder(entity: &mut Entity) -> Result<bool, RollingBoulderBlock> {
    if current_style(entity)? != RollingBoulderStyle::Resting {
        return Ok(false);
    }
    switch_style(entity, RollingBoulderStyle::Rolling)?;
    Ok(true)
}

pub fn current_style(entity: &Entity) -> Result<RollingBoulderStyle, RollingBoulderBlock> {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(RollingBoulderBlock::Graph);
    };
    if context.descriptor_address() != 0x004C_8908 {
        return Err(RollingBoulderBlock::Graph);
    }
    let style = RollingBoulderStyle::from_style_address(context.active_style().style_address())
        .ok_or(RollingBoulderBlock::Graph)?;
    if context.style_table_index_raw_at_0x10() != style.table_index() {
        return Err(RollingBoulderBlock::Graph);
    }
    Ok(style)
}

pub fn rolling_boulder_allocation_authenticates(entity: &Entity) -> bool {
    entity.rolling_boulder_runtime.is_some_and(|runtime| {
        entity.active
            && runtime.allocation.entity_id == entity.id
            && entity.entity_type == runtime.profile.entity_type()
            && entity.model_slots == [Some(usize::from(runtime.profile.model())); 4]
    })
}

pub fn rolling_boulder_manager_allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    rolling_boulder_allocation_authenticates(entity)
        && manager
            .main_base_abort_actor_observation(id)
            .zip(entity.rolling_boulder_runtime)
            .is_some_and(|(observation, runtime)| observation.lease == runtime.allocation)
}

/// Scheduler custody: the actual allocation, current style and Primary task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RollingBoulderOwner {
    entity_id: u32,
    allocation: MainBaseAbortActorLease,
    style: RollingBoulderStyle,
    primary: ActorTaskId,
    /// A visit or an external callback committed a prefix and then blocked;
    /// the owner must not replay the visit or overwrite those effects.
    pending: bool,
}

impl RollingBoulderOwner {
    pub const fn entity_id(&self) -> u32 {
        self.entity_id
    }

    pub const fn style(&self) -> RollingBoulderStyle {
        self.style
    }

    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        *self
    }

    pub const fn allocation(&self) -> MainBaseAbortActorLease {
        self.allocation
    }

    pub const fn has_pending_prefix(&self) -> bool {
        self.pending
    }

    pub(crate) fn park_external_prefix(&mut self) {
        self.pending = true;
    }

    pub fn adopt(manager: &EntityManager, id: u32) -> Result<Self, RollingBoulderBlock> {
        if !rolling_boulder_manager_allocation_authenticates(manager, id) {
            return Err(RollingBoulderBlock::Allocation);
        }
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(RollingBoulderBlock::Allocation)?;
        let style = current_style(entity)?;
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .ok_or(RollingBoulderBlock::Graph)?;
        let primary_matches = matches!(
            (style, entity.actor_task_state(ActorTaskSlot::Primary)),
            (
                RollingBoulderStyle::Rolling,
                Some(ActorTaskRuntime::BoulderRolling(_))
            ) | (
                RollingBoulderStyle::Resting,
                Some(ActorTaskRuntime::BoulderResting(_))
            )
        );
        if !primary_matches
            || entity.actor_task_state(ActorTaskSlot::Secondary).is_some()
            || entity.actor_task_state(ActorTaskSlot::Tertiary).is_some()
        {
            return Err(RollingBoulderBlock::Graph);
        }
        Ok(Self {
            entity_id: id,
            allocation: entity.rolling_boulder_runtime.unwrap().allocation,
            style,
            primary,
            pending: false,
        })
    }

    pub(crate) fn completed_mutation_boundary(&self, manager: &EntityManager) -> bool {
        !self.pending && Self::adopt(manager, self.entity_id) == Ok(*self)
    }
}

pub struct RollingBoulderFrame<'a> {
    pub resources: &'a ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollingBoulderOutcome {
    Waiting {
        entity_id: u32,
    },
    Advanced {
        entity_id: u32,
        elapsed_micros: u32,
        style: RollingBoulderStyle,
        switched: bool,
    },
    Blocked {
        entity_id: u32,
        reason: RollingBoulderBlock,
    },
    Dropped {
        entity_id: u32,
    },
}

impl RollingBoulderOutcome {
    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::Waiting { entity_id }
            | Self::Advanced { entity_id, .. }
            | Self::Blocked { entity_id, .. }
            | Self::Dropped { entity_id } => *entity_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollingBoulderTick {
    pub outcome: RollingBoulderOutcome,
    pub retained_owner: Option<RollingBoulderOwner>,
}

pub fn tick_rolling_boulder(
    manager: &mut EntityManager,
    owner: RollingBoulderOwner,
    frame: RollingBoulderFrame<'_>,
) -> RollingBoulderTick {
    tick_with_random(manager, owner, frame, &mut |fx| {
        u32::from(fx.next_shared_retail_random_u16())
    })
}

pub(crate) fn tick_with_random(
    manager: &mut EntityManager,
    owner: RollingBoulderOwner,
    frame: RollingBoulderFrame<'_>,
    next_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> RollingBoulderTick {
    let id = owner.entity_id;
    let blocked = |reason| RollingBoulderTick {
        outcome: RollingBoulderOutcome::Blocked {
            entity_id: id,
            reason,
        },
        retained_owner: Some(owner),
    };
    let dropped = RollingBoulderTick {
        outcome: RollingBoulderOutcome::Dropped { entity_id: id },
        retained_owner: None,
    };
    if owner.pending {
        return blocked(RollingBoulderBlock::Runtime("parked prefix"));
    }
    if !owner.completed_mutation_boundary(manager) {
        return dropped;
    }
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    let profile = entity.rolling_boulder_runtime.unwrap().profile;
    let Some(metadata) = manager.type_runtime_metadata(profile.entity_type()) else {
        return blocked(RollingBoulderBlock::Metadata);
    };
    if let Err(error) = authenticate_metadata(profile, metadata) {
        return blocked(error);
    }
    let (wind_mode, drag) = manager.intro2_type13_environment();
    let Some(model) = frame.resources.global_model(usize::from(profile.model())) else {
        return blocked(RollingBoulderBlock::Model);
    };
    if model.radius == 0 {
        return blocked(RollingBoulderBlock::Model);
    }
    let RetailRuntimeValue::Known(flags) = entity.collision.state_flags_at_0x08.masked(
        BLOCKING_STATE_MASK
            | CALLBACK_ENABLED_STATE_BIT
            | DETAILED_STATE_BIT
            | COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
    ) else {
        return blocked(RollingBoulderBlock::Runtime("scheduler state"));
    };
    if flags & BLOCKING_STATE_MASK != 0 {
        return blocked(RollingBoulderBlock::Runtime(
            if flags & TERMINAL_STATE_MASK != 0 {
                "terminal or remote body"
            } else {
                "attached body"
            },
        ));
    }
    let RetailRuntimeValue::Known(mut basis) = entity.physical_body_basis_q31() else {
        return blocked(RollingBoulderBlock::Runtime("physical basis"));
    };
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
            next_random(frame.world_fx)
        })
    else {
        return blocked(RollingBoulderBlock::Runtime("scheduler prefix"));
    };
    let entity = manager.entity_mut(id).unwrap();
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    // From here the visit has mutated the body; a later block parks the
    // owner so the next pass cannot replay the committed prefix.
    let parked = |reason| RollingBoulderTick {
        outcome: RollingBoulderOutcome::Blocked {
            entity_id: id,
            reason,
        },
        retained_owner: Some(RollingBoulderOwner {
            pending: true,
            ..owner
        }),
    };
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us: dt,
    } = prefix.flow
    else {
        return RollingBoulderTick {
            outcome: RollingBoulderOutcome::Waiting { entity_id: id },
            retained_owner: Some(owner),
        };
    };
    let mut style = owner.style;
    let mut switched = false;
    if flags & CALLBACK_ENABLED_STATE_BIT != 0 {
        let RetailRuntimeValue::Known(mass) = common_scheduler_callback_mass(
            profile.mass(),
            entity.collision.animation_offset_at_0xb2,
        ) else {
            return parked(RollingBoulderBlock::Runtime("callback B2"));
        };
        entity.mass_raw = mass;
        let detailed = flags & DETAILED_STATE_BIT != 0;
        let position = entity.position_raw();
        let mut velocity = entity.velocity_raw();
        let visit = ActorTaskVisit {
            slot: ActorTaskSlot::Primary,
            task_id: owner.primary,
        };
        let Some(tagged) = entity
            .actor_tasks
            .begin_exact_visit_with(visit, |task| match task {
                ActorTaskRuntime::BoulderRolling(state) => state.step(
                    position,
                    &mut velocity,
                    &mut basis,
                    model.radius,
                    dt,
                    detailed,
                ),
                ActorTaskRuntime::BoulderResting(state) => state.step(&mut velocity, dt, detailed),
                _ => unreachable!("adopted boulder Primary"),
            })
        else {
            return parked(RollingBoulderBlock::Graph);
        };
        entity.set_velocity_raw(velocity);
        entity.physical_body_basis_q31 = RetailRuntimeValue::Known(basis);
        if !entity.actor_tasks.finish_exact_visit(visit) {
            return parked(RollingBoulderBlock::Graph);
        }
        // 401120: a 9C00 result reaches the non-null style +04 unless
        // 416410 reports state 0x1000. The zero lifetime never expires.
        if tagged {
            let RetailRuntimeValue::Known(attached) = entity
                .collision
                .state_flags_at_0x08
                .masked(ATTACHED_STATE_BIT)
            else {
                return parked(RollingBoulderBlock::Runtime("transition gate"));
            };
            if attached == 0 {
                style = style.on_task_completion();
                if let Err(error) = switch_style(entity, style) {
                    return parked(error);
                }
                switched = true;
            }
        }
        // DCA0/E870 -> E100 reads the effective policy after the callback.
        let effective = style.effective_policy();
        let mut velocity = entity.velocity_raw();
        apply_common_gravity_and_underwater_raw(
            &mut velocity,
            dt,
            CommonUnderwaterFrame {
                effective_environment_flags: effective,
                water_response_enabled: false,
                position_y_raw: 0,
                solid_or_sea_y_raw: 0,
                self_mass_raw: mass,
                attached_cargo_mass: 0,
            },
        );
        apply_common_no_wind_drag_raw(
            &mut velocity,
            dt,
            effective,
            CommonEnvironmentPhysics {
                runtime_wind_mode: wind_mode,
                drag_strength: drag,
                ..CommonEnvironmentPhysics::default()
            },
            mass,
        );
        entity.set_velocity_raw(velocity);
    }
    commit_common_scheduler_post_callback(&mut entity.collision);
    // 412DA0 rereads +08 for the master-motion tail; a style switch during
    // this visit has already changed the motion-enable bit.
    let RetailRuntimeValue::Known(motion_flags) = entity
        .collision
        .state_flags_at_0x08
        .masked(COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)
    else {
        return parked(RollingBoulderBlock::Runtime("master motion state"));
    };
    let motion = plan_common_master_motion(
        entity.position_raw(),
        entity.velocity_raw(),
        motion_flags,
        dt,
    );
    commit_common_master_motion(entity, motion);
    let retained = match RollingBoulderOwner::adopt(manager, id) {
        Ok(owner) => owner,
        Err(error) => return parked(error),
    };
    RollingBoulderTick {
        outcome: RollingBoulderOutcome::Advanced {
            entity_id: id,
            elapsed_micros: dt,
            style,
            switched,
        },
        retained_owner: Some(retained),
    }
}

pub mod death;
pub mod impact;

#[cfg(test)]
mod tests;
