//! Native Type69 tractor beams: the method-25 shots of world 47's Type2.
//!
//! `44E770`'s actor branch builds one through `438080` at the shot's
//! position and velocity, with the shooter at `+60`. The beam has no
//! components and model 78's radius is zero, so nothing touches it, and its
//! state never gains the damage-enabled bit, so blasts skip it. It dies only
//! through its own tasks or the Main Base abort, quietly, with sound 62.
//!
//! Class58 Tractor Beam (`4C8840`) has two styles:
//! - `4C7228` (search): `B500` copies the type's axis `+04` (8), clears
//!   Tertiary, installs `01F80`'s guard search in Secondary with filter
//!   override `0xC`, and `402B10`'s 1000-ms retarget in Primary. The
//!   retarget's `01430` has no component to move, so the wrapper times out
//!   after 1000 ms and style `+00` (`CEF0`) calls `10C10`. A candidate
//!   inside the 512-unit box goes through `C7D0` to the tractor style.
//! - `4C7270` (tractor): `C5E0` clears Secondary and Tertiary and installs
//!   `4051B0`'s tractor in Primary. Its `405240` (detailed visits only) puts
//!   the beam on its target; beyond 1024 units from the shooter it pulls the
//!   two together, beyond 2560 it calls `10C10`. A missing or dying target
//!   returns nonzero, and style `+00` (`C690`) searches again.
//!
//! Retail reads the shooter through `+60` without a null check, so a beam
//! still holding something after its Type2 is gone crashes (`004052F8`).
//! The port stops that beam with [`Type69Error::RetailCrashOwnerGone`].

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskVisit, PreparedActorTask},
    common_mover::{
        type9_surface::{decay_actor_surface_timer_ms, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT},
        type9_tail::{plan_common_master_motion, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK},
    },
    damage::DamageProfile,
    entity::{commit_common_master_motion, Entity, EntityManager},
    entity_behavior::{
        audited_behavior_program, audited_behavior_style, initial_behavior_state_policy,
        select_initial_behavior, BehaviorContextRuntime, BehaviorDescriptorIdentity,
        BehaviorSelection, BehaviorWeightRule, QUIET_DEATH_BEHAVIOR_PROGRAM,
    },
    entity_collision_state::{
        CommonMoverComponentTopology, CommonMoverGklPayloads, CommonWorldEffectProfile,
        EntityTypeRuntimeMetadata, RetailRuntimeValue, DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
    },
    entity_scheduler::*,
    guard_location_owner::acquisition::{
        evaluate_guard_location_acquisition_callback, GuardLocationAcquisitionCallbackPrefix,
        GuardLocationAcquisitionCallbackResult, GuardLocationAcquisitionTaskState,
        GuardLocationCandidateFilter, GuardLocationCandidateHandoff, GuardLocationSearchContext,
    },
    main_base_abort::MainBaseAbortActorLease,
    shared_retarget_mover::{
        shared_retarget_after_unwind, SharedRetargetCallbackPrefix, SharedRetargetPostUnwind,
        SharedRetargetTaskState,
    },
    wander_near_location::WanderNearCommonMoverReturn,
    world_fx::WorldFx,
    wrapped_axis_range::WrappedAxisRange,
};
use v2k_formats::collision::{BehaviorChoice, CommonAxisDescriptor};

#[cfg(test)]
mod tests;

pub const ENTITY_TYPE: u32 = 69;
pub const MODEL: usize = 78;
pub const MASS: u16 = 10;
pub const HEALTH: i32 = 1;
pub const DEATH_SOUND: u16 = 62;
/// `+C0` 0x85: no gravity (0x4), no drag, no E640, a rebuilt basis.
pub const DEFAULT_FLAGS: u32 = 0x85;
pub const AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 512,
    raw_word_at_0x04: 8,
};
pub const SEARCH_STYLE: u32 = 0x004c_7228;
pub const TRACTOR_STYLE: u32 = 0x004c_7270;
const BEHAVIOR_CLASS: u32 = 58;
/// Style `4C7228 +44`: `01F80`'s filter override, capability 4 or 8.
pub const SEARCH_FILTER_OVERRIDE_RAW: u32 = 0x0C;
pub const SEARCH_LIFETIME_MS: u32 = 1000;
/// Squared shooter-target distances: pull beyond 1024, break beyond 2560.
pub const PULL_DISTANCE_SQUARED: i32 = 0x10_0000;
pub const BREAK_DISTANCE_SQUARED: i32 = 0x64_0000;
const DAMAGE: DamageProfile = DamageProfile {
    thresholds_raw: [0, 2000, 200, 0, 200, 0, 0],
    multipliers_q8: [0, 256, 256, 512, 128, 0, 0],
};
const CHOICES: [BehaviorChoice; 1] = [BehaviorChoice {
    weight_rule_id: 1,
    weight_multiplier: 1,
    behavior_class_id: 58,
}];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type69Runtime {
    pub(crate) allocation: MainBaseAbortActorLease,
    /// The Type2 whose method-25 shot built this beam (`+60`).
    pub(crate) shooter_id: u32,
    /// Class2's context after `10C10`, kept until `14990`.
    pub(crate) quiet_death_context: Option<BehaviorContextRuntime>,
}

impl Type69Runtime {
    pub const fn shooter_id(self) -> u32 {
        self.shooter_id
    }
}

/// `4051B0`'s private record: the target handle (`ctx +08` at `C5E0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TractorBeamTaskState {
    target_id: u32,
}

impl TractorBeamTaskState {
    pub const fn target_id(self) -> u32 {
        self.target_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type69Error {
    Identity,
    Metadata,
    Allocation,
    Graph,
    Selection,
    Terrain,
    SourceAllocation,
    Runtime(&'static str),
    /// Retail reads the shooter's `+96` through a null pointer here.
    RetailCrashOwnerGone {
        beam_id: u32,
        shooter_id: u32,
    },
}

pub fn authenticate_metadata(metadata: &EntityTypeRuntimeMetadata) -> Result<(), Type69Error> {
    use RetailRuntimeValue::Known;
    let Some(initializer) = &metadata.initializer else {
        return Err(Type69Error::Metadata);
    };
    if metadata.model_slots != [MODEL as u16; 4]
        || metadata.model_variable_count_raw != Known(0)
        || metadata.mass_raw != MASS
        || metadata.capability_flags != 0
        || metadata.initial_health_raw != Some(HEALTH)
        || metadata.damage_profile != Some(DAMAGE)
        || metadata.common_mover_topology != Known(CommonMoverComponentTopology::default())
        || metadata.common_mover_gkl_payloads
            != Known(CommonMoverGklPayloads {
                sub_g: None,
                sub_k: None,
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
        || metadata.detailed_sound_policy
            != Known(crate::actor_detailed_sound::ActorDetailedSoundPolicy {
                full_health_raw: HEALTH,
                healthy_sounds: [0, 0],
                low_health_sound: 0,
                periods_raw: [0, 0],
            })
        || initializer.initializer_state_flags_raw != DEFAULT_FLAGS
        || initializer.common_axis_descriptor != AXIS
        || initializer.behavior_choices.as_ref() != CHOICES
        || initializer.behavior_rule_ref != 1
        || initializer.alternate_behavior_class_ref != 2
    {
        return Err(Type69Error::Metadata);
    }
    Ok(())
}

pub(crate) fn allocation_authenticates(entity: &Entity) -> bool {
    entity.native_type69_runtime.is_some_and(|runtime| {
        entity.active
            && entity.entity_type == ENTITY_TYPE
            && runtime.allocation.entity_id == entity.id
            && entity.authored_spawn_index.is_none()
            && entity.model_slots == [Some(MODEL); 4]
            && entity.capability_flags == 0
    })
}

pub(crate) fn manager_allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    manager.iter_all().any(|entity| {
        entity.id == id
            && allocation_authenticates(entity)
            && manager
                .main_base_abort_actor_observation(id)
                .zip(entity.native_type69_runtime)
                .is_some_and(|(observation, runtime)| observation.lease == runtime.allocation)
    })
}

/// `+08 == 0`: a freed or never-published body.
fn state_is_zero(state: crate::entity_collision_state::RetailStateWord) -> bool {
    state.known_mask() == u32::MAX && state.known_value_bits() == 0
}

fn bits(entity: &Entity, mask: u32) -> Result<u32, Type69Error> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        RetailRuntimeValue::Unresolved => Err(Type69Error::Runtime("state bits")),
    }
}

/// `25680` draws a word even for the singleton Always list.
pub(crate) fn select(
    next_random: &mut impl FnMut() -> u32,
) -> Result<BehaviorSelection, Type69Error> {
    select_initial_behavior(
        &CHOICES,
        |rule| i32::from(rule == BehaviorWeightRule::Always),
        next_random,
    )
    .map_err(|_| Type69Error::Selection)?
    .ok_or(Type69Error::Selection)
}

/// `B500` on a published search context: the axis `+04` copy, then Tertiary
/// cleared, `01F80` in Secondary and `402B10` in Primary. Neither task's
/// `06070` suffix has a component to write or a word to draw.
pub(crate) fn publish_search(entity: &mut Entity, context: BehaviorContextRuntime) {
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let program = audited_behavior_program(BEHAVIOR_CLASS).expect("class58");
    let policy = initial_behavior_state_policy(program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    if let RetailRuntimeValue::Known(mut axis) = entity.actor_common_axis_descriptor {
        axis.raw_word_at_0x04 = AXIS.raw_word_at_0x04;
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
    }
    let position = entity.position_raw();
    entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Secondary,
        PreparedActorTask::new(ActorTaskRuntime::GuardLocationAcquisition(
            GuardLocationAcquisitionTaskState::new(SEARCH_FILTER_OVERRIDE_RAW),
        )),
    );
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Primary,
        PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
            SharedRetargetTaskState::new(position, SEARCH_LIFETIME_MS),
        )),
    );
}

/// `C7D0`: the target to `ctx +08`, `C6B0` to variant 1, then `C5E0`.
fn publish_tractor(entity: &mut Entity, target_id: u32) -> Result<(), Type69Error> {
    let RetailRuntimeValue::Known(Some(current)) = entity.current_behavior_context else {
        return Err(Type69Error::Graph);
    };
    let program = audited_behavior_program(BEHAVIOR_CLASS).ok_or(Type69Error::Graph)?;
    let style = *audited_behavior_style(BEHAVIOR_CLASS, 1).ok_or(Type69Error::Graph)?;
    let tractor = BehaviorContextRuntime::named_audited(
        program,
        1,
        current.choice_list_source(),
        RetailRuntimeValue::Known(Some(target_id)),
        current.auxiliary_word_at_0x0c(),
        style,
    )
    .ok_or(Type69Error::Graph)?;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(tractor));
    entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Primary,
        PreparedActorTask::new(ActorTaskRuntime::TractorBeam(TractorBeamTaskState {
            target_id,
        })),
    );
    Ok(())
}

/// Style `4C7270 +00` = `C690`: `AC60` draws the singleton selector word and
/// runs `B500` again.
fn reselect(entity: &mut Entity, world_fx: &mut WorldFx) -> Result<(), Type69Error> {
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(Type69Error::Graph);
    };
    let program = audited_behavior_program(BEHAVIOR_CLASS).ok_or(Type69Error::Graph)?;
    let context = previous
        .reselect_named_type_default(program, 0, program.initial_style)
        .ok_or(Type69Error::Graph)?;
    select(&mut || u32::from(world_fx.next_shared_retail_random_u16()))?;
    publish_search(entity, context);
    Ok(())
}

/// `10C10`: a remote or already dying beam returns; otherwise health 0, the
/// dying bit, sound 62 at the beam (`44F480`), then `DB80 -> AC60` class2,
/// whose `C470` clears the tasks and queues `14990`.
pub fn begin_quiet_death(
    manager: &mut EntityManager,
    id: u32,
    world_fx: &mut WorldFx,
) -> Result<bool, Type69Error> {
    if !manager_allocation_authenticates(manager, id) {
        return Err(Type69Error::Allocation);
    }
    let entity = manager.entity_mut(id).ok_or(Type69Error::Allocation)?;
    if bits(entity, REMOTE_OWNED_STATE_BIT)? != 0 {
        return Ok(false);
    }
    if bits(entity, DYING_STATE_BIT)? != 0 {
        return Ok(true);
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Type69Error::Graph);
    };
    if context.descriptor()
        != BehaviorDescriptorIdentity::Named(
            audited_behavior_program(BEHAVIOR_CLASS).ok_or(Type69Error::Graph)?,
        )
    {
        return Err(Type69Error::Graph);
    }
    let terminal = context
        .reselect_audited_type_default(
            &QUIET_DEATH_BEHAVIOR_PROGRAM,
            QUIET_DEATH_BEHAVIOR_PROGRAM.initial_style_table_index_raw,
            QUIET_DEATH_BEHAVIOR_PROGRAM.initial_style,
        )
        .ok_or(Type69Error::Graph)?;
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    world_fx.queue_fixed_positional_sound_raw(DEATH_SOUND, entity.position_raw());
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(terminal));
    for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        entity.actor_tasks.clear_slot(slot);
    }
    entity.mark_actor_deferred_destroy_pending();
    entity
        .native_type69_runtime
        .as_mut()
        .unwrap()
        .quiet_death_context = Some(terminal);
    manager.queue_actor_deferred_destroy(id);
    Ok(true)
}

/// A completed quiet death keeps only its class2 context until `14990`.
pub fn finished_terminal_authenticates(manager: &EntityManager, id: u32) -> bool {
    manager_allocation_authenticates(manager, id)
        && manager.pending_actor_deferred_destroy_ids().contains(&id)
        && manager.iter_all().find(|e| e.id == id).is_some_and(|e| {
            e.native_type69_runtime
                .and_then(|runtime| runtime.quiet_death_context)
                .is_some_and(|context| {
                    e.current_behavior_context == RetailRuntimeValue::Known(Some(context))
                })
                && ActorTaskSlot::IN_RETAIL_TICK_ORDER
                    .into_iter()
                    .all(|slot| e.actor_tasks.task_in_slot(slot).is_none())
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Graph {
    Search {
        primary: ActorTaskId,
        secondary: ActorTaskId,
    },
    Tractor {
        primary: ActorTaskId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type69Owner {
    allocation: MainBaseAbortActorLease,
    context: BehaviorContextRuntime,
    graph: Graph,
}

impl Type69Owner {
    pub const fn entity_id(self) -> u32 {
        self.allocation.entity_id
    }
    pub const fn allocation(self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub(crate) const fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }

    pub fn adopt(manager: &EntityManager, id: u32) -> Result<Self, Type69Error> {
        if !manager_allocation_authenticates(manager, id) {
            return Err(Type69Error::Allocation);
        }
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(Type69Error::Allocation)?;
        let runtime = entity
            .native_type69_runtime
            .ok_or(Type69Error::Allocation)?;
        let program = audited_behavior_program(BEHAVIOR_CLASS).ok_or(Type69Error::Graph)?;
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Type69Error::Graph);
        };
        if runtime.quiet_death_context.is_some()
            || bits(entity, DYING_STATE_BIT)? != 0
            || context.descriptor() != BehaviorDescriptorIdentity::Named(program)
        {
            return Err(Type69Error::Graph);
        }
        let slot = |slot| entity.actor_tasks.task_in_slot(slot);
        let alive = |task: ActorTaskId| {
            entity.actor_tasks.wrapper_flags(task)
                == Some(crate::actor_task_owner::ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: false,
                })
        };
        let graph = match (
            context.active_style().style_address(),
            slot(ActorTaskSlot::Primary),
            slot(ActorTaskSlot::Secondary),
            slot(ActorTaskSlot::Tertiary),
        ) {
            (SEARCH_STYLE, Some(primary), Some(secondary), None)
                if matches!(
                    entity.actor_tasks.task_state(primary),
                    Some(ActorTaskRuntime::SharedRetarget(task))
                        if task.lifetime_ms() == SEARCH_LIFETIME_MS
                ) && matches!(
                    entity.actor_tasks.task_state(secondary),
                    Some(ActorTaskRuntime::GuardLocationAcquisition(task))
                        if task.constructor_filter_override_raw() == SEARCH_FILTER_OVERRIDE_RAW
                ) && alive(primary)
                    && alive(secondary) =>
            {
                Graph::Search { primary, secondary }
            }
            (TRACTOR_STYLE, Some(primary), None, None)
                if matches!(
                    entity.actor_tasks.task_state(primary),
                    Some(ActorTaskRuntime::TractorBeam(task))
                        if context.target_handle_at_0x08()
                            == RetailRuntimeValue::Known(Some(task.target_id))
                ) && alive(primary) =>
            {
                Graph::Tractor { primary }
            }
            _ => return Err(Type69Error::Graph),
        };
        Ok(Self {
            allocation: runtime.allocation,
            context,
            graph,
        })
    }
}

pub struct Type69Frame<'a> {
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type69Outcome {
    Waiting {
        entity_id: u32,
    },
    Advanced {
        entity_id: u32,
        callback_enabled: bool,
        detailed: bool,
        died: bool,
    },
    Blocked {
        entity_id: u32,
        reason: Type69Error,
        prefix_committed: bool,
    },
    Dropped {
        entity_id: u32,
    },
}

impl Type69Outcome {
    pub const fn entity_id(self) -> u32 {
        match self {
            Self::Waiting { entity_id }
            | Self::Advanced { entity_id, .. }
            | Self::Blocked { entity_id, .. }
            | Self::Dropped { entity_id } => entity_id,
        }
    }
}

pub struct Type69Tick {
    pub outcome: Type69Outcome,
    pub retained_owner: Option<Type69Owner>,
}

pub fn tick_type69(
    manager: &mut EntityManager,
    owner: Type69Owner,
    frame: Type69Frame<'_>,
) -> Type69Tick {
    let id = owner.entity_id();
    if Type69Owner::adopt(manager, id) != Ok(owner) {
        return Type69Tick {
            outcome: Type69Outcome::Dropped { entity_id: id },
            retained_owner: None,
        };
    }
    let mut committed = false;
    match run_frame(manager, owner, frame, &mut committed) {
        Ok(outcome) => Type69Tick {
            outcome,
            retained_owner: Type69Owner::adopt(manager, id).ok(),
        },
        // A committed prefix must not replay: the owner is not retained.
        Err(reason) => Type69Tick {
            outcome: Type69Outcome::Blocked {
                entity_id: id,
                reason,
                prefix_committed: committed,
            },
            retained_owner: (!committed).then_some(owner),
        },
    }
}

fn run_frame(
    manager: &mut EntityManager,
    owner: Type69Owner,
    frame: Type69Frame<'_>,
    committed: &mut bool,
) -> Result<Type69Outcome, Type69Error> {
    let id = owner.entity_id();
    let metadata = manager
        .type_runtime_metadata(ENTITY_TYPE)
        .cloned()
        .ok_or(Type69Error::Metadata)?;
    authenticate_metadata(&metadata)?;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Type69Error::Allocation)?;
    let state = bits(
        entity,
        REMOTE_OWNED_STATE_BIT
            | 0x1000
            | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    )?;
    if state & (REMOTE_OWNED_STATE_BIT | 0x1000) != 0 || entity.attached_to.is_some() {
        return Err(Type69Error::Runtime("local unattached beam"));
    }
    if entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(DEFAULT_FLAGS) {
        return Err(Type69Error::Runtime("default environment flags"));
    }
    let callback_enabled = state & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT != 0;
    let detailed = state & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0;
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
            u32::from(frame.world_fx.next_shared_retail_random_u16())
        })
    else {
        return Err(Type69Error::Runtime("scheduler state"));
    };
    let entity = manager.entity_mut(id).ok_or(Type69Error::Allocation)?;
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    *committed = true;
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us: dt,
    } = prefix.flow
    else {
        return Ok(Type69Outcome::Waiting { entity_id: id });
    };
    let mut died = false;
    if callback_enabled {
        let RetailRuntimeValue::Known(mass) = common_scheduler_callback_mass(
            metadata.mass_raw,
            entity.collision.animation_offset_at_0xb2,
        ) else {
            return Err(Type69Error::Runtime("callback B2"));
        };
        entity.mass_raw = mass;
        died = run_tasks(manager, owner, frame.world_fx, dt, detailed)?;
        // DCA0/E870 after the tasks, a death included, for policy 0x85: no
        // healthy sound (both words zero), 13F70 (0x4000 clear), no E640,
        // gravity or drag, no DF70, and E370's timer.
        let entity = manager.entity_mut(id).ok_or(Type69Error::Allocation)?;
        entity.apply_d720_euler_body_basis();
        if bits(entity, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT)? == 0 {
            let RetailRuntimeValue::Known(timer) = entity.surface_lifetime_timer_ms_at_0x48 else {
                return Err(Type69Error::Runtime("surface timer"));
            };
            entity.surface_lifetime_timer_ms_at_0x48 =
                RetailRuntimeValue::Known(decay_actor_surface_timer_ms(timer, dt));
        }
    }
    let entity = manager.entity_mut(id).ok_or(Type69Error::Allocation)?;
    commit_common_scheduler_post_callback(&mut entity.collision);
    let motion = plan_common_master_motion(
        entity.position_raw(),
        entity.velocity_raw(),
        bits(entity, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)?,
        dt,
    );
    commit_common_master_motion(entity, motion);
    Ok(Type69Outcome::Advanced {
        entity_id: id,
        callback_enabled,
        detailed,
        died,
    })
}

/// `0A800`: Primary, then Secondary. Returns whether `10C10` ran.
fn run_tasks(
    manager: &mut EntityManager,
    owner: Type69Owner,
    world_fx: &mut WorldFx,
    dt: u32,
    detailed: bool,
) -> Result<bool, Type69Error> {
    let id = owner.entity_id();
    match owner.graph {
        Graph::Search { primary, secondary } => {
            let entity = manager.entity_mut(id).ok_or(Type69Error::Allocation)?;
            let visit = ActorTaskVisit {
                slot: ActorTaskSlot::Primary,
                task_id: primary,
            };
            let position = entity.position_raw();
            let (prefix, mut stage) = entity
                .actor_tasks
                .begin_exact_visit_with(visit, |runtime| {
                    let ActorTaskRuntime::SharedRetarget(task) = runtime else {
                        unreachable!()
                    };
                    let lifetime = task.before_callback(dt);
                    let stage =
                        task.stage_callback(position, || world_fx.next_shared_retail_random_u16());
                    (
                        SharedRetargetCallbackPrefix::from_parts(lifetime, stage.retarget()),
                        stage,
                    )
                })
                .ok_or(Type69Error::Graph)?;
            // 01430 with no component: only a tracked target is read.
            let private = stage.private_state_mut();
            if private.reversal_timer_ms != 0 {
                entity.actor_tasks.finish_exact_visit(visit);
                return Err(Type69Error::Runtime("01430 countdown without components"));
            }
            let tracked = private.tracked_entity_handle;
            if let Some(ActorTaskRuntime::SharedRetarget(task)) =
                entity.actor_tasks.task_state_mut(primary)
            {
                task.commit_callback_stage(stage);
            }
            if !entity.actor_tasks.finish_exact_visit(visit) {
                return Err(Type69Error::Graph);
            }
            let tracked_dying = tracked != 0
                && manager
                    .iter_all()
                    .find(|e| e.id == tracked)
                    .is_some_and(|e| {
                        e.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
                            != RetailRuntimeValue::Known(0)
                            || state_is_zero(e.collision.state_flags_at_0x08)
                    });
            let result = if tracked_dying {
                WanderNearCommonMoverReturn::Zero
            } else {
                WanderNearCommonMoverReturn::NonZero
            };
            match shared_retarget_after_unwind(visit, prefix, result) {
                SharedRetargetPostUnwind::Continue => {}
                // Style +00 is CEF0: 10C10.
                SharedRetargetPostUnwind::Transition(_) => {
                    begin_quiet_death(manager, id, world_fx)?;
                    return Ok(true);
                }
                SharedRetargetPostUnwind::UnresolvedCommonMover => {
                    return Err(Type69Error::Runtime("retarget mover"))
                }
            }
            search(manager, id, secondary, world_fx)?;
            Ok(false)
        }
        Graph::Tractor { primary } => tractor(manager, id, primary, world_fx, dt, detailed),
    }
}

/// `01FB0`: one word, the one-in-four gate, the `0xC` override written to
/// axis `+04`, then `22C10`'s first eligible candidate inside the box.
fn search(
    manager: &mut EntityManager,
    id: u32,
    secondary: ActorTaskId,
    world_fx: &mut WorldFx,
) -> Result<(), Type69Error> {
    let candidates: Vec<_> = manager
        .iter_all()
        .map(crate::type47_scheduler_production::type47_guard_entity_ref)
        .collect();
    let entity = manager.entity_mut(id).ok_or(Type69Error::Allocation)?;
    if entity.actor_tasks.task_in_slot(ActorTaskSlot::Secondary) != Some(secondary) {
        return Err(Type69Error::Graph);
    }
    let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
        return Err(Type69Error::Runtime("search axis"));
    };
    let mut context = GuardLocationSearchContext::new(
        WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
        GuardLocationCandidateFilter::from_raw(axis.raw_word_at_0x04),
    );
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Secondary,
        task_id: secondary,
    };
    let prefix = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::GuardLocationAcquisition(task) = runtime else {
                unreachable!()
            };
            task.before_callback(
                &mut context,
                u32::from(world_fx.next_shared_retail_random_u16()),
            )
        })
        .ok_or(Type69Error::Graph)?;
    if let GuardLocationAcquisitionCallbackPrefix::Acquire {
        filter_write: Some(filter),
        ..
    } = prefix
    {
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(CommonAxisDescriptor {
            raw_word_at_0x04: filter.raw(),
            ..axis
        });
    }
    let owner_ref = crate::type47_scheduler_production::type47_guard_entity_ref(entity);
    let mut handoff_error = None;
    let result = evaluate_guard_location_acquisition_callback(
        prefix,
        owner_ref,
        &candidates,
        Some(|handoff: GuardLocationCandidateHandoff| {
            match publish_tractor(entity, handoff.candidate.id) {
                Ok(()) => 0,
                Err(reason) => {
                    handoff_error = Some(reason);
                    1
                }
            }
        }),
    );
    // C5E0 retired this search inside its own callback; unwind frees it.
    entity.actor_tasks.finish_exact_visit(visit);
    if let Some(reason) = handoff_error {
        return Err(reason);
    }
    match result.map_err(|_| Type69Error::Runtime("search candidate state"))? {
        GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted { .. }
        | GuardLocationAcquisitionCallbackResult::Zero(_) => Ok(()),
        GuardLocationAcquisitionCallbackResult::PropagateBehaviorResult { .. } => {
            unreachable!("the handoff reports its own failure")
        }
    }
}

/// `405240`, run only on a detailed visit (`0A800` mode 0).
fn tractor(
    manager: &mut EntityManager,
    id: u32,
    primary: ActorTaskId,
    world_fx: &mut WorldFx,
    dt: u32,
    detailed: bool,
) -> Result<bool, Type69Error> {
    let entity = manager
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(Type69Error::Allocation)?;
    let Some(ActorTaskRuntime::TractorBeam(task)) = entity.actor_tasks.task_state(primary).copied()
    else {
        return Err(Type69Error::Graph);
    };
    let shooter_id = entity
        .native_type69_runtime
        .ok_or(Type69Error::Allocation)?
        .shooter_id;
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id: primary,
    };
    let entity = manager.entity_mut(id).unwrap();
    entity
        .actor_tasks
        .begin_exact_visit_with(visit, |_| ())
        .ok_or(Type69Error::Graph)?;
    let finish = |manager: &mut EntityManager| {
        manager
            .entity_mut(id)
            .is_some_and(|entity| entity.actor_tasks.finish_exact_visit(visit))
    };
    if !detailed {
        return if finish(manager) {
            Ok(false)
        } else {
            Err(Type69Error::Graph)
        };
    }
    let target = manager
        .iter_all()
        .find(|e| e.id == task.target_id)
        .map(|e| (e.collision.state_flags_at_0x08, e.position_raw()));
    let live_target = match target {
        None => None,
        Some((state, position)) => match state.masked(DYING_STATE_BIT) {
            RetailRuntimeValue::Known(0) if !state_is_zero(state) => Some(position),
            RetailRuntimeValue::Known(_) => None,
            RetailRuntimeValue::Unresolved => {
                finish(manager);
                return Err(Type69Error::Runtime("tractor target state"));
            }
        },
    };
    let Some(target_position) = live_target else {
        // A nonzero result: style +00 (C690) searches again.
        if !finish(manager) {
            return Err(Type69Error::Graph);
        }
        let entity = manager.entity_mut(id).ok_or(Type69Error::Allocation)?;
        reselect(entity, world_fx)?;
        return Ok(false);
    };
    manager
        .entity_mut(id)
        .unwrap()
        .set_position_raw(target_position);
    let Some(shooter_position) = manager
        .iter_all()
        .find(|e| e.id == shooter_id)
        .map(Entity::position_raw)
    else {
        finish(manager);
        return Err(Type69Error::RetailCrashOwnerGone {
            beam_id: id,
            shooter_id,
        });
    };
    let delta: [i32; 3] = std::array::from_fn(|axis| {
        i32::from(target_position[axis]) - i32::from(shooter_position[axis])
    });
    let squared = delta
        .iter()
        .fold(0i32, |sum, &d| sum.wrapping_add(d.wrapping_mul(d)));
    if squared > BREAK_DISTANCE_SQUARED {
        if !finish(manager) {
            return Err(Type69Error::Graph);
        }
        begin_quiet_death(manager, id, world_fx)?;
        return Ok(true);
    }
    if squared > PULL_DISTANCE_SQUARED {
        let pull = tractor_pull_raw(delta, squared, dt);
        let target = manager
            .entity_mut(task.target_id)
            .ok_or(Type69Error::Runtime("tractor target"))?;
        let velocity = target.velocity_raw();
        target.set_velocity_raw(std::array::from_fn(|axis| {
            velocity[axis].wrapping_sub(pull[axis])
        }));
        let shooter = manager
            .entity_mut(shooter_id)
            .ok_or(Type69Error::Runtime("tractor shooter"))?;
        let velocity = shooter.velocity_raw();
        shooter.set_velocity_raw(std::array::from_fn(|axis| {
            velocity[axis].wrapping_add(pull[axis])
        }));
    }
    if finish(manager) {
        Ok(false)
    } else {
        Err(Type69Error::Graph)
    }
}

/// `405240`'s pull: `457960` normalises the shooter-to-target delta, the
/// strength is `(dt >> 9) * ((0x640000 - d²) / 20000) >> 8`, and each word is
/// the Q31 product's bits 31..46.
pub(crate) fn tractor_pull_raw(delta: [i32; 3], squared: i32, dt: u32) -> [i16; 3] {
    let unit = crate::ordinary_type47_shot_math::normalize_retail_vector_q31(delta);
    let strength =
        (dt >> 9).wrapping_mul(((BREAK_DISTANCE_SQUARED - squared) / 20_000) as u32) >> 8;
    unit.map(|component| ((i64::from(strength as i32) * i64::from(component)) >> 31) as i16)
}
