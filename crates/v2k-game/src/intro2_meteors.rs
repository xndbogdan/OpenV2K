//! Authenticated Intro2 class-19 Boulder Trailing Fire task and world owner.
//!
//! The actor's physical Q31 matrix is mutated by displacement in 404690;
//! neither a cinematic clock nor Euler reconstruction owns this rotation.
//! Terrain contact is a separate 411AD0 phase after the complete mover pass.

pub mod impact;

use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskVisit, PreparedActorTask};
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::common_mover::type9_tail::{
    plan_common_master_motion, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
};
use crate::damage::{
    generic_entity_damage_transition, DamagePacket, DamageProfile, GenericEntityDamageStage,
    GenericEntityDamageState,
};
use crate::entity::{
    apply_type13_common_environment_raw, commit_common_master_motion, Entity, EntityManager,
};
use crate::entity_behavior::{
    audited_behavior_program, select_initial_behavior, BehaviorContextRuntime, BehaviorWeightRule,
};
use crate::entity_collision_state::{
    CommonMoverComponentTopology, CommonWorldEffectProfile, EntityTypeRuntimeMetadata,
    RetailRuntimeValue, DYING_STATE_BIT,
};
use crate::entity_scheduler::{
    commit_common_scheduler_post_callback, commit_common_scheduler_prefix,
    common_scheduler_callback_mass, plan_common_scheduler_prefix, CommonSchedulerPrefixFlow,
};
use crate::radial_damage::RadialDamageTemplate;
use crate::resource_cache::ResourceCache;
use crate::terrain_contact::TerrainModelContact;
use crate::whole_body_surface::nearest_cell_material_code;
use crate::world_fx::{TerrainCollisionContext, WorldFx};
use std::sync::atomic::{AtomicU64, Ordering};
use v2k_formats::fixed_math::retail_sine_q15;
use v2k_formats::models::AnimVars;

static NEXT_METEOR_CONTINUATION_ID: AtomicU64 = AtomicU64::new(1);

pub const INTRO2_METEOR_TYPE: u32 = 34;
pub const INTRO2_METEOR_MODEL: usize = 560;
pub const INTRO2_METEOR_SPAWN_INDICES: [usize; 4] = [31, 33, 34, 35];
pub const BOULDER_ROLLING_LIFETIME_MS: u32 = 5_000;
pub const METEOR_TRAIL_CLASS: u8 = 0x28;
const METEOR_FLAGS: u32 = 0x4008;
const METEOR_DAMAGE_PROFILE: DamageProfile = DamageProfile {
    thresholds_raw: [0, 2_000, 200, 0, 200, 0, 0],
    multipliers_q8: [0, 256, 256, 512, 128, 0, 0],
};

/// Retained fields read/written by 401350 -> 4012E0 -> 404690.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoulderRollingTaskState {
    pub previous_position_raw: [i16; 3],
    pub stationary_visits: i32,
    pub direction_raw_at_0x10: i32,
    pub movement_words_at_0x14: [u16; 2],
    pub elapsed_ms: u32,
    /// Port continuation custody, separate from the retail task words. A
    /// radial callback may commit a prefix before reporting missing support;
    /// retaining its claim prevents a copied receipt from replaying that work.
    radial_continuation: Option<MeteorRadialContinuation>,
}

impl BoulderRollingTaskState {
    pub const fn new(position_raw: [i16; 3]) -> Self {
        Self {
            previous_position_raw: position_raw,
            stationary_visits: 0,
            direction_raw_at_0x10: 1,
            movement_words_at_0x14: [0; 2],
            elapsed_ms: 0,
            radial_continuation: None,
        }
    }

    /// One callback only; the shared wrapper tests lifetime after unwind.
    pub fn step(
        &mut self,
        position_raw: [i16; 3],
        velocity_raw: &mut [i16; 3],
        basis: &mut Type9BodyBasis,
        extent_raw: u16,
        elapsed_micros: u32,
        detailed: bool,
    ) -> bool {
        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1_000);
        if self.previous_position_raw == position_raw {
            self.stationary_visits = self.stationary_visits.wrapping_add(1);
            if self.stationary_visits > 10 {
                self.movement_words_at_0x14 = [0; 2];
                return true; // singleton 004BE190, tag 9C00
            }
        } else {
            if detailed {
                roll_displaced_body(basis, self.previous_position_raw, position_raw, extent_raw);
            }
            self.stationary_visits = 0;
            self.previous_position_raw = position_raw;
        }
        let damping = (elapsed_micros >> 13) as i32;
        for axis in [0, 2] {
            let old = i32::from(velocity_raw[axis]);
            velocity_raw[axis] = if old > 0 {
                (old - damping).max(0)
            } else {
                (old + damping).min(0)
            } as i16;
        }
        // 401430 has no component work for this exact all-null topology.
        false
    }
}

/// 4069E0's private payload is a class number, not an allocated structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrailingFireTaskState {
    pub particle_class: u8,
    pub elapsed_ms: u32,
    /// Type34 has no other live consumer of this callback's entity +84 bit.
    pub emitted_state_bit_at_0x84: bool,
}

impl TrailingFireTaskState {
    pub const fn new() -> Self {
        Self {
            particle_class: METEOR_TRAIL_CLASS,
            elapsed_ms: 0,
            emitted_state_bit_at_0x84: false,
        }
    }
}

impl Default for TrailingFireTaskState {
    fn default() -> Self {
        Self::new()
    }
}

fn q31(lhs: i32, rhs: i32) -> i32 {
    ((i64::from(lhs) * i64::from(rhs)) >> 31) as i32
}

/// 457A90/457C30/457DD0 duplicate the unsigned table word into both halves.
fn sine_q31(angle: i32) -> i32 {
    let sine = retail_sine_q15(angle as u32);
    let word = sine.unsigned_abs();
    let duplicated = (word | (word << 16)) as i32;
    if sine < 0 {
        duplicated.wrapping_neg()
    } else {
        duplicated
    }
}

fn rotate_pair(first: &mut [i32; 3], second: &mut [i32; 3], angle: i32) {
    let sine = sine_q31(angle);
    let cosine = sine_q31(angle.wrapping_add(0x4000));
    let old_first = *first;
    let old_second = *second;
    for axis in 0..3 {
        first[axis] = q31(cosine, old_first[axis]).wrapping_add(q31(sine, old_second[axis]));
        second[axis] =
            q31(cosine, old_second[axis]).wrapping_add(q31(sine.wrapping_neg(), old_first[axis]));
    }
}

fn roll_displaced_body(
    basis: &mut Type9BodyBasis,
    previous: [i16; 3],
    current: [i16; 3],
    extent_raw: u16,
) {
    assert_ne!(extent_raw, 0, "404690 divides by the selected model extent");
    let x = q31(
        (i32::from(previous[2].wrapping_sub(current[2])) << 14) / i32::from(extent_raw),
        i32::MAX,
    )
    .wrapping_neg();
    let z = q31(
        (i32::from(previous[0].wrapping_sub(current[0])) << 14) / i32::from(extent_raw),
        i32::MAX,
    );
    let angles = [basis.lateral, basis.up, basis.forward]
        .map(|axis| q31(x, axis[0]).wrapping_add(q31(z, axis[2])));
    if angles[2] != 0 {
        rotate_pair(&mut basis.lateral, &mut basis.up, angles[2]);
    }
    if angles[0] != 0 {
        rotate_pair(&mut basis.up, &mut basis.forward, angles[0]);
    }
    if angles[1] != 0 {
        rotate_pair(
            &mut basis.lateral,
            &mut basis.forward,
            angles[1].wrapping_neg(),
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2MeteorError {
    Identity,
    Metadata,
    AlreadyPublished,
    TaskChanged,
    RuntimeState,
    Model,
    Wind,
    CollisionProgram,
    DeathPending,
}

/// A receipt identifies exact task wrappers, so replacing a slot invalidates it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2MeteorOwner {
    entity_id: u32,
    spawn_index: usize,
    primary: ActorTaskId,
    tertiary: ActorTaskId,
}

impl Intro2MeteorOwner {
    pub const fn entity_id(&self) -> u32 {
        self.entity_id
    }
    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        *self
    }
    pub fn adopt_published(entity: &Entity) -> Result<Self, Intro2MeteorError> {
        let spawn_index = authenticate_identity(entity)?;
        if !matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::BoulderRolling(_))
        ) || !matches!(
            entity.actor_task_state(ActorTaskSlot::Tertiary),
            Some(ActorTaskRuntime::TrailingFire(_))
        ) || entity.actor_task_state(ActorTaskSlot::Secondary).is_some()
        {
            return Err(Intro2MeteorError::TaskChanged);
        }
        Ok(Self {
            entity_id: entity.id,
            spawn_index,
            primary: entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap(),
            tertiary: entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Tertiary)
                .unwrap(),
        })
    }
}

fn authenticate_identity(entity: &Entity) -> Result<usize, Intro2MeteorError> {
    let Some(spawn) = entity.authored_spawn_index else {
        return Err(Intro2MeteorError::Identity);
    };
    if !entity.active
        || entity.entity_type != INTRO2_METEOR_TYPE
        || !INTRO2_METEOR_SPAWN_INDICES.contains(&spawn)
        || entity.model_slots != [Some(INTRO2_METEOR_MODEL); 4]
        || entity.attached_to.is_some()
    {
        return Err(Intro2MeteorError::Identity);
    }
    Ok(spawn)
}

fn authenticate_metadata(metadata: &EntityTypeRuntimeMetadata) -> Result<(), Intro2MeteorError> {
    let Some(init) = &metadata.initializer else {
        return Err(Intro2MeteorError::Metadata);
    };
    let empty = CommonMoverComponentTopology {
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
    if metadata.mass_raw != 100
        || metadata.capability_flags != 0
        || metadata.initial_health_raw != Some(1_000)
        || metadata.damage_profile != Some(METEOR_DAMAGE_PROFILE)
        || metadata.common_mover_topology != RetailRuntimeValue::Known(empty)
        || metadata.common_world_effects
            != RetailRuntimeValue::Known(CommonWorldEffectProfile {
                surface_selectors: [0; 2],
                surface_lifetime_ms: 0,
                low_health_effect_words: [0; 3],
            })
        || metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
        || metadata.death_sound_id != RetailRuntimeValue::Known(None)
        || metadata.generic_hit_sound_id != RetailRuntimeValue::Known(None)
        || init.initializer_state_flags_raw != METEOR_FLAGS
        || init.behavior_rule_ref != 3
        || init.alternate_behavior_class_ref != 1
        || init.behavior_choices.len() != 1
        || init.behavior_choices[0].weight_rule_id != 1
        || init.behavior_choices[0].weight_multiplier != 1
        || init.behavior_choices[0].behavior_class_id != 19
    {
        return Err(Intro2MeteorError::Metadata);
    }
    Ok(())
}

/// Call after common construction and before 451710's launch transform.
pub fn publish_intro2_meteor(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2MeteorOwner, Intro2MeteorError> {
    let spawn = authenticate_identity(entity)?;
    authenticate_metadata(metadata)?;
    let expected = match spawn {
        31 => [-30976, 6400, -28672],
        33 => [-29184, 5120, -26112],
        34 => [32256, 5120, -26112],
        35 => [-30208, 5120, -26112],
        _ => unreachable!(),
    };
    if entity.position_raw() != expected
        || entity.rotation_heading_pitch_roll_raw() != [0; 3]
        || entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .is_some()
        || entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary)
            .is_some()
    {
        return Err(Intro2MeteorError::AlreadyPublished);
    }
    let selection = select_initial_behavior(
        &metadata.initializer.as_ref().unwrap().behavior_choices,
        |rule| {
            debug_assert_eq!(rule, BehaviorWeightRule::Always);
            1
        },
        next_random,
    )
    .map_err(|_| Intro2MeteorError::Metadata)?
    .ok_or(Intro2MeteorError::Metadata)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Intro2MeteorError::Metadata)?;
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    // B910 publishes Tertiary, clears Secondary, then constructs Primary.
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Tertiary,
        PreparedActorTask::new(ActorTaskRuntime::TrailingFire(TrailingFireTaskState::new())),
    );
    entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Primary,
        PreparedActorTask::new(ActorTaskRuntime::BoulderRolling(
            BoulderRollingTaskState::new(expected),
        )),
    );
    // +B2 is not constructor-written in retail. The native fresh allocation
    // initializes this heap-owned transient once, as for the Intro2 flyers.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    // 40D720 -> 413F70 writes the parked constructor matrix. 4519E0 then
    // disables movement/callback/subject bits before applying launch words;
    // the four Section-2 opcode-2 events subsequently re-enable these bits.
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0, 0, 0));
    entity.collision.state_flags_at_0x08.overwrite(0x68000, 0);
    let mut launched = expected;
    launched[0] = launched[0].wrapping_sub(0x500);
    launched[2] = launched[2].wrapping_add(0xa00);
    entity.set_motion_raw(launched, [1500, -400, -3000]);
    Intro2MeteorOwner::adopt_published(entity)
}

pub struct Intro2MeteorFrame<'a> {
    pub resources: &'a ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeteorTerminalReceipt {
    pub entity_id: u32,
    pub position_raw: [i16; 3],
    pub radial_damage: RadialDamageTemplate,
    primary: ActorTaskId,
    tertiary: ActorTaskId,
    callback_suffix: Option<[u32; 3]>,
    /// Port allocation identity; unrelated to retail RNG or entity state bits.
    continuation_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MeteorRadialContinuation {
    Issued(MeteorTerminalReceipt),
    Claimed(MeteorTerminalReceipt),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2MeteorOutcome {
    Waiting {
        entity_id: u32,
    },
    Advanced {
        entity_id: u32,
        elapsed_micros: u32,
        trail_attempts: usize,
    },
    Terminal {
        entity_id: u32,
        receipt: MeteorTerminalReceipt,
    },
    Blocked {
        entity_id: u32,
        reason: Intro2MeteorError,
    },
    Dropped {
        entity_id: u32,
    },
}
impl Intro2MeteorOutcome {
    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::Waiting { entity_id }
            | Self::Advanced { entity_id, .. }
            | Self::Terminal { entity_id, .. }
            | Self::Blocked { entity_id, .. }
            | Self::Dropped { entity_id } => *entity_id,
        }
    }
}
pub struct Intro2MeteorTick {
    pub outcome: Intro2MeteorOutcome,
    pub retained_owner: Option<Intro2MeteorOwner>,
}

fn checked_owner(entity: &Entity, owner: Intro2MeteorOwner) -> bool {
    Intro2MeteorOwner::adopt_published(entity) == Ok(owner)
        && entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) == Some(owner.primary)
        && entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary) == Some(owner.tertiary)
        && entity.actor_task_state(ActorTaskSlot::Secondary).is_none()
}

/// One exact live-list 12DA0 visit; no nominal frame subdivision is introduced.
pub fn tick_intro2_meteor(
    manager: &mut EntityManager,
    owner: Intro2MeteorOwner,
    frame: Intro2MeteorFrame<'_>,
) -> Intro2MeteorTick {
    tick_intro2_meteor_with_random(manager, owner, frame, &mut |fx| {
        u32::from(fx.next_shared_retail_random_u16())
    })
}

pub(crate) fn tick_intro2_meteor_with_random(
    manager: &mut EntityManager,
    owner: Intro2MeteorOwner,
    frame: Intro2MeteorFrame<'_>,
    next_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Intro2MeteorTick {
    let id = owner.entity_id;
    let blocked = |reason| Intro2MeteorTick {
        outcome: Intro2MeteorOutcome::Blocked {
            entity_id: id,
            reason,
        },
        retained_owner: Some(owner),
    };
    let Some(entity) = manager.iter_all().find(|e| e.id == id) else {
        return Intro2MeteorTick {
            outcome: Intro2MeteorOutcome::Dropped { entity_id: id },
            retained_owner: None,
        };
    };
    if !checked_owner(entity, owner) {
        return Intro2MeteorTick {
            outcome: Intro2MeteorOutcome::Dropped { entity_id: id },
            retained_owner: None,
        };
    }
    let Some(metadata) = manager.type_runtime_metadata(INTRO2_METEOR_TYPE) else {
        return blocked(Intro2MeteorError::Metadata);
    };
    if let Err(error) = authenticate_metadata(metadata) {
        return blocked(error);
    }
    let (wind, drag) = manager.intro2_type13_environment();
    if wind != 0 {
        return blocked(Intro2MeteorError::Wind);
    }
    let Some(model) = frame.resources.global_model(INTRO2_METEOR_MODEL) else {
        return blocked(Intro2MeteorError::Model);
    };
    if model.radius == 0 {
        return blocked(Intro2MeteorError::Model);
    }
    let RetailRuntimeValue::Known(flags) = entity
        .collision
        .state_flags_at_0x08
        .masked(0x8212_1000 | COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)
    else {
        return blocked(Intro2MeteorError::RuntimeState);
    };
    if flags & 0x8010_1000 != 0 {
        return blocked(Intro2MeteorError::DeathPending);
    }
    let RetailRuntimeValue::Known(mut basis) = entity.physical_body_basis_q31 else {
        return blocked(Intro2MeteorError::RuntimeState);
    };
    if entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(METEOR_FLAGS)
        || entity.surface_lifetime_timer_ms_at_0x48 != RetailRuntimeValue::Known(0)
        || entity.collision.constructor_sound_attachment_id_at_0x8c
            != RetailRuntimeValue::Known(None)
    {
        return blocked(Intro2MeteorError::RuntimeState);
    }
    if !matches!(entity.current_behavior_context,RetailRuntimeValue::Known(Some(context))
        if context.descriptor_address()==0x004c8900 && context.style_table_index_raw_at_0x10()==0)
    {
        return blocked(Intro2MeteorError::TaskChanged);
    }
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
            next_random(frame.world_fx)
        })
    else {
        return blocked(Intro2MeteorError::RuntimeState);
    };
    let entity = manager.entity_mut(id).unwrap();
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us: dt,
    } = prefix.flow
    else {
        return Intro2MeteorTick {
            outcome: Intro2MeteorOutcome::Waiting { entity_id: id },
            retained_owner: Some(owner),
        };
    };
    let mut attempts = 0;
    if flags & 0x20000 != 0 {
        let RetailRuntimeValue::Known(mass) =
            common_scheduler_callback_mass(100, entity.collision.animation_offset_at_0xb2)
        else {
            return blocked(Intro2MeteorError::RuntimeState);
        };
        entity.mass_raw = mass;
        let detailed = flags & 0x0200_0000 != 0;
        let position = entity.position_raw();
        let mut velocity = entity.velocity_raw();
        let visit = ActorTaskVisit {
            slot: ActorTaskSlot::Primary,
            task_id: owner.primary,
        };
        let terminal = entity
            .actor_tasks
            .begin_exact_visit_with(visit, |task| {
                let ActorTaskRuntime::BoulderRolling(state) = task else {
                    unreachable!()
                };
                let tagged = state.step(
                    position,
                    &mut velocity,
                    &mut basis,
                    model.radius,
                    dt,
                    detailed,
                );
                tagged || state.elapsed_ms > BOULDER_ROLLING_LIFETIME_MS
            })
            .unwrap();
        entity.set_velocity_raw(velocity);
        entity.physical_body_basis_q31 = RetailRuntimeValue::Known(basis);
        assert!(entity.actor_tasks.finish_exact_visit(visit));
        if terminal {
            let receipt = begin_meteor_death(
                entity,
                frame.resources,
                frame.world_fx,
                Some([dt, wind, drag]),
            );
            return Intro2MeteorTick {
                outcome: Intro2MeteorOutcome::Terminal {
                    entity_id: id,
                    receipt,
                },
                retained_owner: None,
            };
        }
        let visit = ActorTaskVisit {
            slot: ActorTaskSlot::Tertiary,
            task_id: owner.tertiary,
        };
        entity
            .actor_tasks
            .begin_exact_visit_with(visit, |task| {
                let ActorTaskRuntime::TrailingFire(state) = task else {
                    unreachable!()
                };
                state.elapsed_ms = state.elapsed_ms.wrapping_add(dt / 1_000);
            })
            .unwrap();
        let speed_squared = velocity.iter().fold(0i32, |sum, &v| {
            sum.wrapping_add(i32::from(v).wrapping_mul(i32::from(v)))
        });
        if detailed && speed_squared > 0x77a10 {
            attempts = emit_meteor_trail(frame.world_fx, id, position, velocity, dt, next_random);
            let ActorTaskRuntime::TrailingFire(state) =
                entity.actor_tasks.exact_callback_state_mut(visit).unwrap()
            else {
                unreachable!()
            };
            state.emitted_state_bit_at_0x84 = true;
        }
        assert!(entity.actor_tasks.finish_exact_visit(visit));
        apply_type13_common_environment_raw(&mut velocity, dt, mass, wind, drag);
        entity.set_velocity_raw(velocity);
    }
    commit_common_scheduler_post_callback(&mut entity.collision);
    let motion = plan_common_master_motion(
        entity.position_raw(),
        entity.velocity_raw(),
        flags & COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
        dt,
    );
    commit_common_master_motion(entity, motion);
    Intro2MeteorTick {
        outcome: Intro2MeteorOutcome::Advanced {
            entity_id: id,
            elapsed_micros: dt,
            trail_attempts: attempts,
        },
        retained_owner: Some(owner),
    }
}

fn emit_meteor_trail(
    fx: &mut WorldFx,
    id: u32,
    mut position: [i16; 3],
    velocity: [i16; 3],
    dt: u32,
    random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> usize {
    let mut remaining = dt as i32;
    let mut vertical = velocity[1];
    let mut attempts = 0;
    loop {
        vertical = vertical.wrapping_sub(43);
        fx.emit_intro2_meteor_trail_raw(position, id);
        attempts += 1;
        remaining = remaining.wrapping_sub(30_000);
        for (axis, speed) in [
            i32::from(velocity[0]),
            i32::from(vertical),
            i32::from(velocity[2]),
        ]
        .into_iter()
        .enumerate()
        {
            let jitter = ((random(fx) as u16) >> 11) as i16 - 16;
            position[axis] = position[axis]
                .wrapping_add(jitter)
                .wrapping_add(q31(speed, 0x03a9_8000) as i16);
        }
        if remaining <= 0 {
            break;
        }
    }
    attempts
}

fn begin_meteor_death(
    entity: &mut Entity,
    resources: &ResourceCache,
    fx: &mut WorldFx,
    callback_suffix: Option<[u32; 3]>,
) -> MeteorTerminalReceipt {
    let primary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let tertiary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Tertiary)
        .unwrap();
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    entity.select_active_model_slot(1);
    let program = audited_behavior_program(1).expect("audited class-1 Explode descriptor");
    let RetailRuntimeValue::Known(Some(previous_context)) = entity.current_behavior_context else {
        unreachable!("authenticated class-19 context");
    };
    let context = previous_context
        .reselect_audited_type_default(program, 0, program.initial_style)
        .unwrap();
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let position_raw = entity.position_raw();
    fx.emit_intro2_meteor_impact_raw(
        position_raw,
        entity.id,
        resources.global_model(INTRO2_METEOR_MODEL).unwrap().radius,
        resources.level_terrain().map(|t| t.sea_level_raw()),
    );
    let header = &resources
        .global_entity_type(INTRO2_METEOR_TYPE as usize)
        .unwrap()
        .raw_header;
    let word = |at: usize| i16::from_le_bytes(header[at..at + 2].try_into().unwrap());
    let dword = |at: usize| i32::from_le_bytes(header[at..at + 4].try_into().unwrap());
    let receipt = MeteorTerminalReceipt {
        entity_id: entity.id,
        position_raw,
        primary,
        tertiary,
        callback_suffix,
        continuation_id: NEXT_METEOR_CONTINUATION_ID.fetch_add(1, Ordering::Relaxed),
        radial_damage: RadialDamageTemplate {
            inner_radius_raw: word(0x50),
            outer_radius_raw: word(0x52),
            impulse_raw: dword(0x54),
            packet: DamagePacket {
                channels: [dword(0x58), dword(0x5c)],
                amounts_raw: [dword(0x60), dword(0x64)],
            },
            trailing_raw: [INTRO2_METEOR_TYPE as i32, entity.id as i32],
        },
    };
    let ActorTaskRuntime::BoulderRolling(task) =
        entity.actor_tasks.task_state_mut(primary).unwrap()
    else {
        unreachable!()
    };
    task.radial_continuation = Some(MeteorRadialContinuation::Issued(receipt));
    receipt
}

/// Claim the exact callback continuation before the static/dynamic radial
/// prefix. A rejected/blocked suffix remains claimed until its owner supplies
/// an explicit continuation; a copied receipt is never a retry authorization.
pub fn claim_intro2_meteor_radial(
    manager: &mut EntityManager,
    receipt: &MeteorTerminalReceipt,
) -> bool {
    if !meteor_terminal_receipt_is_current(manager, receipt) {
        return false;
    }
    let entity = manager.entity_mut(receipt.entity_id).unwrap();
    let ActorTaskRuntime::BoulderRolling(task) =
        entity.actor_tasks.task_state_mut(receipt.primary).unwrap()
    else {
        unreachable!()
    };
    task.radial_continuation = Some(MeteorRadialContinuation::Claimed(*receipt));
    true
}

/// BAC0's suffix, called exactly once after the receipt's 4566E0 radial work.
pub fn finish_intro2_meteor_death(
    manager: &mut EntityManager,
    receipt: MeteorTerminalReceipt,
) -> bool {
    if !meteor_terminal_receipt_identity_is_current(manager, &receipt) {
        return false;
    }
    let entity = manager.entity_mut(receipt.entity_id).unwrap();
    if !matches!(entity.actor_tasks.task_state(receipt.primary), Some(ActorTaskRuntime::BoulderRolling(task))
        if task.radial_continuation == Some(MeteorRadialContinuation::Claimed(receipt)))
    {
        return false;
    }
    for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        entity.actor_tasks.clear_slot(slot);
    }
    entity.mark_actor_deferred_destroy_pending();
    if let Some([dt, wind, drag]) = receipt.callback_suffix {
        let mut velocity = entity.velocity_raw();
        apply_type13_common_environment_raw(&mut velocity, dt, entity.mass_raw, wind, drag);
        entity.set_velocity_raw(velocity);
        commit_common_scheduler_post_callback(&mut entity.collision);
        // 10B70 cleared master-enable before 12DA0 re-reads it.
        crate::entity::commit_known_common_master_motion(entity, dt);
    }
    manager.queue_actor_deferred_destroy(receipt.entity_id);
    true
}

/// Check the receipt before *any* radial mutation or repeated callback work.
pub(crate) fn meteor_terminal_receipt_is_current(
    manager: &EntityManager,
    receipt: &MeteorTerminalReceipt,
) -> bool {
    if !meteor_terminal_receipt_identity_is_current(manager, receipt) {
        return false;
    }
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == receipt.entity_id)
        .unwrap();
    matches!(entity.actor_tasks.task_state(receipt.primary), Some(ActorTaskRuntime::BoulderRolling(task))
        if task.radial_continuation == Some(MeteorRadialContinuation::Issued(*receipt)))
}

fn meteor_terminal_receipt_identity_is_current(
    manager: &EntityManager,
    receipt: &MeteorTerminalReceipt,
) -> bool {
    let Some(entity) = manager.iter_all().find(|e| e.id == receipt.entity_id) else {
        return false;
    };
    if authenticate_identity(entity).is_err()
        || entity.position_raw() != receipt.position_raw
        || entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
            != RetailRuntimeValue::Known(DYING_STATE_BIT)
        || entity.collision.state_flags_at_0x08.masked(0x100000) != RetailRuntimeValue::Known(0)
        || entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) != Some(receipt.primary)
        || entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary) != Some(receipt.tertiary)
        || entity.collision.health_raw != RetailRuntimeValue::Known(0)
        || !matches!(entity.current_behavior_context,RetailRuntimeValue::Known(Some(context))
            if context.active_style().style_address()==0x004c7150)
    {
        return false;
    }
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeteorTerrainResponse {
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub particle_scale_raw: u32,
    pub collision_damage_raw: i32,
}

/// 4141D0 mode 1: separate, emit, correct velocity, then channel-1 damage.
pub fn plan_meteor_terrain_response(
    position: [i16; 3],
    velocity: [i16; 3],
    mass: u16,
    contact: TerrainModelContact,
) -> MeteorTerrainResponse {
    let mut result = MeteorTerrainResponse {
        position_raw: position,
        velocity_raw: velocity,
        particle_scale_raw: 0,
        collision_damage_raw: 0,
    };
    for axis in 0..3 {
        result.position_raw[axis] = position[axis].wrapping_add(
            (i32::from(contact.normal_q12[axis]).wrapping_mul(contact.penetration_raw) >> 12)
                as i16,
        );
    }
    let inward = contact
        .normal_q12
        .iter()
        .zip(velocity)
        .fold(0i32, |sum, (&n, v)| {
            sum.wrapping_add(i32::from(n).wrapping_mul(i32::from(v)))
        })
        >> 12;
    if inward < 0 {
        let speed = inward.wrapping_neg();
        let response = speed.wrapping_add(q31(speed, i32::MAX));
        result.particle_scale_raw = response.wrapping_mul(2) as u32;
        for (axis, &normal) in contact.normal_q12.iter().enumerate() {
            result.velocity_raw[axis] = velocity[axis]
                .wrapping_add((i32::from(normal).wrapping_mul(response >> 1) >> 12) as i16);
        }
        let squared =
            velocity
                .into_iter()
                .zip(result.velocity_raw)
                .fold(0i32, |sum, (before, after)| {
                    let delta = i32::from(before.wrapping_sub(after));
                    sum.wrapping_add(delta.wrapping_mul(delta))
                });
        result.collision_damage_raw = (squared >> 7).wrapping_mul(i32::from(mass)) >> 10;
    }
    result
}

/// Bare-terrain part of the separate post-mover 411AD0 scan.
pub fn resolve_intro2_meteor_terrain_contact(
    manager: &mut EntityManager,
    entity_id: u32,
    resources: &ResourceCache,
    fx: &mut WorldFx,
) -> Result<Option<MeteorTerminalReceipt>, Intro2MeteorError> {
    let entity = manager
        .entity_mut(entity_id)
        .ok_or(Intro2MeteorError::Identity)?;
    authenticate_identity(entity)?;
    let RetailRuntimeValue::Known(flags) = entity.collision.state_flags_at_0x08.masked(0x8811_d000)
    else {
        return Err(Intro2MeteorError::RuntimeState);
    };
    if flags & 0x8000 == 0
        || flags & 0x8810_5000 != 0
        || entity.collision.subject_scan_gate_at_0x70 != RetailRuntimeValue::Known(0)
        || flags & 0x10000 == 0
    {
        return Ok(None);
    }
    Intro2MeteorOwner::adopt_published(entity)?;
    if !matches!(entity.current_behavior_context,RetailRuntimeValue::Known(Some(context))
        if context.descriptor_address()==0x004c8900 && context.style_table_index_raw_at_0x10()==0)
    {
        return Err(Intro2MeteorError::TaskChanged);
    }
    let context = TerrainCollisionContext::from_current_level_cache(resources)
        .ok_or(Intro2MeteorError::Model)?;
    let model = resources
        .global_model(INTRO2_METEOR_MODEL)
        .ok_or(Intro2MeteorError::Model)?;
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31 else {
        return Err(Intro2MeteorError::RuntimeState);
    };
    let RetailRuntimeValue::Known(health) = entity.collision.health_raw else {
        return Err(Intro2MeteorError::RuntimeState);
    };
    let RetailRuntimeValue::Known(buffer) = entity.collision.pre_health_damage_buffer_raw else {
        return Err(Intro2MeteorError::RuntimeState);
    };
    let axes = [basis.lateral, basis.up, basis.forward];
    let matrix = std::array::from_fn(|world| {
        std::array::from_fn(|local| f64::from(axes[local][world]) / 2147483648.0)
    });
    let Some(hit) = model
        .collide_terrain_raw_oriented(
            context.terrain,
            entity.position_raw(),
            matrix,
            &AnimVars::default(),
        )
        .map_err(|_| Intro2MeteorError::CollisionProgram)?
    else {
        return Ok(None);
    };
    // 12870 packs material from the original entity center into contact+10.
    // 141D0 uses that retained selector after separating the hull; crossing a
    // cell boundary during separation must not select a different effect.
    let position_before = entity.position_raw();
    let material =
        nearest_cell_material_code(context.terrain, position_before[0], position_before[2]);
    let response = plan_meteor_terrain_response(
        entity.position_raw(),
        entity.velocity_raw(),
        entity.mass_raw,
        TerrainModelContact {
            normal_q12: hit.normal.map(|n| (n * 4096.0).round() as i16),
            penetration_raw: hit.penetration_raw as i32,
        },
    );
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x800000, 0x800000);
    entity.set_position_raw(response.position_raw);
    if response.particle_scale_raw != 0 {
        let [x, _, z] = response.position_raw;
        fx.emit_intro2_meteor_ground_raw(
            [x, context.terrain.bilinear_height_raw(x, z), z],
            entity.id,
            context.ground_response_selectors[usize::from(material)],
            response.particle_scale_raw,
        );
    }
    entity.set_velocity_raw(response.velocity_raw);
    let filtered = DamagePacket::collision(response.collision_damage_raw)
        .filtered_raw(Some(&METEOR_DAMAGE_PROFILE));
    if filtered != 0 {
        let damage = generic_entity_damage_transition(
            GenericEntityDamageState {
                health_raw: health,
                pre_health_buffer_raw: buffer,
                already_dying: false,
            },
            filtered,
        );
        entity.collision.health_raw =
            RetailRuntimeValue::Known(damage.health_after_subtraction_raw);
        entity.collision.pre_health_damage_buffer_raw =
            RetailRuntimeValue::Known(damage.pre_health_buffer_after_raw);
        if damage.stage == GenericEntityDamageStage::DeathDispatchRequired {
            return Ok(Some(begin_meteor_death(entity, resources, fx, None)));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stationary_tag_is_the_eleventh_visit_and_skips_damping() {
        let mut state = BoulderRollingTaskState::new([0; 3]);
        let mut basis = Type9BodyBasis::from_angle_words(0, 0, 0);
        let mut velocity = [100, 200, -100];
        for _ in 0..10 {
            assert!(!state.step([0; 3], &mut velocity, &mut basis, 256, 8192, false));
        }
        assert_eq!(velocity, [90, 200, -90]);
        state.movement_words_at_0x14 = [4, 5];
        assert!(state.step([0; 3], &mut velocity, &mut basis, 256, 8192, false));
        assert_eq!(velocity, [90, 200, -90]);
        assert_eq!(state.movement_words_at_0x14, [0; 2]);
    }

    #[test]
    fn coarse_visit_advances_previous_position_without_rotating() {
        let original = Type9BodyBasis::from_angle_words(0, 0, 0);
        let mut basis = original;
        let mut state = BoulderRollingTaskState::new([0; 3]);
        state.stationary_visits = 9;
        let mut velocity = [1500, -400, -3000];
        assert!(!state.step([8, 1, 16], &mut velocity, &mut basis, 256, 48000, false));
        assert_eq!(basis, original);
        assert_eq!(state.previous_position_raw, [8, 1, 16]);
        assert_eq!(state.stationary_visits, 0);
        assert_eq!(velocity, [1495, -400, -2995]);
        assert!(!state.step([8, 1, 16], &mut velocity, &mut basis, 256, 8000, true));
        assert_eq!(
            basis, original,
            "entering detail cannot replay the earlier coarse displacement"
        );
    }

    #[test]
    fn rolling_horizontal_deltas_wrap_as_words_and_vertical_only_motion_does_not_roll() {
        let initial = Type9BodyBasis::from_angle_words(0, 0, 0);
        let mut seam = initial;
        let mut ordinary = initial;
        roll_displaced_body(&mut seam, [32760, 0, 32760], [-32760, 0, -32760], 256);
        roll_displaced_body(&mut ordinary, [0; 3], [16, 0, 16], 256);
        assert_eq!(seam, ordinary);
        assert_ne!(ordinary, initial);
        let mut vertical = initial;
        roll_displaced_body(&mut vertical, [0; 3], [0, 20, 0], 256);
        assert_eq!(vertical, initial);
    }

    #[test]
    fn rotation_lookup_duplicates_the_word_and_keeps_low_angle_mask() {
        assert_eq!(sine_q31(0), 0);
        assert_eq!(sine_q31(4), 12 | (12 << 16));
        assert_eq!(sine_q31(7), sine_q31(4));
        assert_eq!(sine_q31(0x8004), -sine_q31(4));
        assert_eq!(sine_q31(0x4000), sine_q31(0x3ffc));
    }

    #[test]
    fn trail_attempts_keep_post_final_rng_and_signed_word_vertical_wrap() {
        for (dt, attempts) in [(0, 1), (30000, 1), (30001, 2), (125000, 5)] {
            let mut fx = WorldFx::new();
            let mut draws = 0;
            let emitted = emit_meteor_trail(
                &mut fx,
                77,
                [i16::MAX; 3],
                [0, i16::MIN, 0],
                dt,
                &mut |_| {
                    draws += 1;
                    0
                },
            );
            assert_eq!(emitted, attempts);
            assert_eq!(draws, attempts * 3);
            assert_eq!(fx.particle_count(), attempts);
        }
    }

    #[test]
    fn plane_response_removes_inward_speed_then_applies_mass_filter() {
        let contact = TerrainModelContact {
            normal_q12: [0, 4096, 0],
            penetration_raw: 10,
        };
        let response = plan_meteor_terrain_response([0, -10, 0], [100, -4000, 200], 100, contact);
        assert_eq!(response.position_raw, [0; 3]);
        // MAX Q31 gain is one raw unit below an exact doubled response.
        assert_eq!(response.velocity_raw, [100, -1, 200]);
        assert_eq!(response.particle_scale_raw, 15998);
        assert_eq!(response.collision_damage_raw, 12200);
        assert_eq!(
            DamagePacket::collision(response.collision_damage_raw)
                .filtered_raw(Some(&METEOR_DAMAGE_PROFILE)),
            10200
        );
        let separating = plan_meteor_terrain_response([0; 3], [0, 4000, 0], 100, contact);
        assert_eq!(separating.velocity_raw, [0, 4000, 0]);
        assert_eq!(separating.particle_scale_raw, 0);
        assert_eq!(separating.collision_damage_raw, 0);
    }
}
