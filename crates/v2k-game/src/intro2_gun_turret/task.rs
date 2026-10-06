//! Native Gun Turret private state and the exact408770/408BD0 task visit.

use super::{aim, intro2_gun_turret_allocation_authenticates};
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit},
    entity::{Entity, EntityManager},
    entity_collision_state::{recent_relation_suppresses_pair, RetailRuntimeValue},
    world_fx::WorldFx,
};

mod math;

const NORMAL_PRIORITIES: [i32; 3] = [0x20000, 0x10000, 0]; //4C7098
const INFECTED_PRIORITIES: [i32; 3] = [0, 0x30000, 0x20000]; //4C70A8

/// The0x2C private block plus the independent01120 wrapper age.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GunTurretTaskState {
    elapsed_ms: u32,
    target_handle: u32,
    target_age_us: u32,
    yaw_raw: i32,
    pitch_raw: i32,
    selection_remaining_us: i32,
    tracking_disabled: bool,
    gravity_lead: bool,
    original_capability: u32,
    lost_target_remaining_us: u32,
    normal_uses_infected_priorities: bool,
}

impl GunTurretTaskState {
    pub const fn from_408df0(
        capability_flags: u32,
        sub_l_output_raw: [i16; 2],
        gravity_lead: bool,
    ) -> Self {
        Self {
            elapsed_ms: 0,
            target_handle: 0,
            target_age_us: 0,
            yaw_raw: -(sub_l_output_raw[0] as i32),
            pitch_raw: sub_l_output_raw[1] as i32,
            selection_remaining_us: 0,
            tracking_disabled: false,
            gravity_lead,
            original_capability: capability_flags,
            lost_target_remaining_us: 6_000_000,
            normal_uses_infected_priorities: capability_flags & 8 != 0,
        }
    }
    pub const fn elapsed_ms(&self) -> u32 {
        self.elapsed_ms
    }
    pub const fn target_handle(&self) -> u32 {
        self.target_handle
    }
    pub const fn target_age_us(&self) -> u32 {
        self.target_age_us
    }
    pub const fn angles_raw(&self) -> [i32; 2] {
        [self.yaw_raw, self.pitch_raw]
    }
    pub const fn selection_remaining_us(&self) -> i32 {
        self.selection_remaining_us
    }
    pub const fn tracking_disabled(&self) -> bool {
        self.tracking_disabled
    }
    pub const fn lost_target_remaining_us(&self) -> u32 {
        self.lost_target_remaining_us
    }
}

pub struct TurretTaskFrame<'a> {
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
    pub retail_tick: u32,
    pub notifications: &'a mut crate::gameplay_notifications::GameplayNotifications,
    pub notification_phase: crate::gameplay_notifications::GameplayNotificationPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurretTaskOutcome {
    Continue,
    RetargetRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurretTaskBlock {
    Allocation,
    Graph,
    Runtime(&'static str),
    Aim(aim::Intro2GunTurretAimError),
}

fn bits(entity: &Entity, mask: u32) -> Result<u32, TurretTaskBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        RetailRuntimeValue::Unresolved => Err(TurretTaskBlock::Runtime("turret state")),
    }
}

pub(crate) fn graph_authenticates(entity: &Entity) -> bool {
    intro2_gun_turret_allocation_authenticates(entity)
        && matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(context)) if context.active_style().style_address() == 0x004c8230)
        && entity.actor_task_state(ActorTaskSlot::Primary).is_none()
        && entity.actor_task_state(ActorTaskSlot::Secondary).is_none()
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Tertiary),
            Some(ActorTaskRuntime::Intro2GunTurret(_))
        )
}

///Type115's other authored choice is C490's real Primary wait. The same
///allocation retains E/L while the firing wrapper is absent.
pub(crate) fn idle_graph_authenticates(entity: &Entity) -> bool {
    intro2_gun_turret_allocation_authenticates(entity)
        && entity.entity_type == 115
        && matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(context))
            if context.active_style().style_address() == 0x004c7468)
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::Class0Timer(_))
        )
        && entity.actor_task_state(ActorTaskSlot::Secondary).is_none()
        && entity.actor_task_state(ActorTaskSlot::Tertiary).is_none()
}

pub(crate) fn tick_flower_idle_task(
    entity: &mut Entity,
    elapsed_micros: u32,
) -> Result<TurretTaskOutcome, TurretTaskBlock> {
    if !idle_graph_authenticates(entity) {
        return Err(TurretTaskBlock::Graph);
    }
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id: entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .ok_or(TurretTaskBlock::Graph)?,
    };
    let expired = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::Class0Timer(state) = runtime else {
                unreachable!()
            };
            state.advance_prefix(elapsed_micros)
        })
        .ok_or(TurretTaskBlock::Graph)?;
    let survived = entity.actor_tasks.finish_exact_visit(visit);
    Ok(if survived && expired && bits(entity, 0x1000)? == 0 {
        TurretTaskOutcome::RetargetRequired
    } else {
        TurretTaskOutcome::Continue
    })
}

pub fn tick_intro2_gun_turret_task(
    manager: &mut EntityManager,
    id: u32,
    mut frame: TurretTaskFrame<'_>,
    world_fx: &mut WorldFx,
) -> Result<TurretTaskOutcome, TurretTaskBlock> {
    if !super::intro2_gun_turret_manager_allocation_authenticates(manager, id) {
        return Err(TurretTaskBlock::Allocation);
    }
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(TurretTaskBlock::Allocation)?;
    if !graph_authenticates(entity) {
        return Err(TurretTaskBlock::Graph);
    }
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Tertiary,
        task_id: entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary)
            .ok_or(TurretTaskBlock::Graph)?,
    };
    let mut state = manager
        .entity_mut(id)
        .ok_or(TurretTaskBlock::Allocation)?
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::Intro2GunTurret(state) = runtime else {
                unreachable!()
            };
            state.elapsed_ms = state.elapsed_ms.wrapping_add(frame.elapsed_micros / 1000);
            *state
        })
        .ok_or(TurretTaskBlock::Graph)?;
    // Every partial callback prefix belongs to this exact wrapper, including
    // selection RNG and an emitter failure after cadence or joint writes.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        callback(manager, id, &mut state, &mut frame, world_fx)
    }));
    let entity = manager.entity_mut(id).ok_or(TurretTaskBlock::Allocation)?;
    if let Some(ActorTaskRuntime::Intro2GunTurret(current)) =
        entity.actor_tasks.task_state_mut(visit.task_id)
    {
        *current = state;
    }
    if entity
        .actor_tasks
        .wrapper_flags(visit.task_id)
        .is_none_or(|flags| !flags.in_callback)
    {
        return Err(TurretTaskBlock::Graph);
    }
    let survived = entity.actor_tasks.finish_exact_visit(visit);
    let outcome = match result {
        Ok(result) => result?,
        Err(payload) => std::panic::resume_unwind(payload),
    };
    if !survived {
        return Ok(TurretTaskOutcome::Continue);
    }
    //01120/16410 evaluates transition suppression after the callback unwinds.
    if outcome == TurretTaskOutcome::RetargetRequired && bits(entity, 0x1000)? != 0 {
        return Ok(TurretTaskOutcome::Continue);
    }
    Ok(outcome)
}

fn callback(
    manager: &mut EntityManager,
    id: u32,
    state: &mut GunTurretTaskState,
    frame: &mut TurretTaskFrame<'_>,
    world_fx: &mut WorldFx,
) -> Result<TurretTaskOutcome, TurretTaskBlock> {
    let entity = manager.entity_mut(id).ok_or(TurretTaskBlock::Allocation)?;
    entity.set_velocity_raw([0; 3]);
    if bits(entity, 0x1000)? != 0 {
        return Ok(TurretTaskOutcome::Continue);
    }
    if state.selection_remaining_us < frame.elapsed_micros as i32 {
        acquire(manager, id, state, world_fx)?;
    } else {
        state.selection_remaining_us = state
            .selection_remaining_us
            .wrapping_sub(frame.elapsed_micros as i32);
    }
    if state.tracking_disabled {
        if state.lost_target_remaining_us <= frame.elapsed_micros {
            return Ok(TurretTaskOutcome::RetargetRequired);
        }
        state.lost_target_remaining_us -= frame.elapsed_micros;
    }
    let target = manager
        .iter_all()
        .find(|entity| entity.id == state.target_handle)
        .map(|entity| (entity.position_raw(), entity.velocity_raw()));
    let mut direction = None;
    if let Some((position, velocity)) = target {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(TurretTaskBlock::Allocation)?;
        let RetailRuntimeValue::Known(body) = entity.physical_body_basis_q31() else {
            return Err(TurretTaskBlock::Runtime("turret body basis"));
        };
        let metadata = manager
            .type_runtime_metadata(entity.entity_type)
            .ok_or(TurretTaskBlock::Runtime("turret metadata"))?;
        let RetailRuntimeValue::Known(Some(emitter)) = metadata.projectile_emitter_descriptor
        else {
            return Err(TurretTaskBlock::Runtime("turret descriptor"));
        };
        let speed = emitter.speed_override_raw as u16;
        if speed == 0 {
            return Err(TurretTaskBlock::Runtime("turret lead divisor"));
        }
        let tracking = math::track(math::TrackingFrame {
            owner: entity.position_raw(),
            target: position,
            velocity,
            speed,
            body,
            yaw: state.yaw_raw,
            pitch: state.pitch_raw,
            gravity: state.gravity_lead,
            elapsed_us: frame.elapsed_micros,
            retail_tick: frame.retail_tick,
        });
        state.yaw_raw = tracking.yaw;
        state.pitch_raw = tracking.pitch;
        state.target_age_us = state.target_age_us.wrapping_add(frame.elapsed_micros);
        if tracking.aligned && !state.tracking_disabled {
            direction = Some(tracking.direction);
        }
    } else {
        if state.target_age_us != 0 {
            state.selection_remaining_us = 0;
            state.target_handle = 0;
        }
        state.target_age_us = 0;
        state.tracking_disabled = true;
    }
    aim::tick_intro2_gun_turret_aim(
        manager,
        id,
        direction,
        aim::Intro2GunTurretAimFrame {
            global_elapsed_micros: frame.global_elapsed_micros,
            world_fx,
            notifications: frame.notifications,
            notification_phase: frame.notification_phase,
            retail_tick: frame.retail_tick,
        },
    )
    .map_err(TurretTaskBlock::Aim)?;
    let entity = manager.entity_mut(id).ok_or(TurretTaskBlock::Allocation)?;
    let runtime = entity
        .intro2_gun_turret_runtime
        .as_mut()
        .ok_or(TurretTaskBlock::Allocation)?;
    runtime.sub_l_output_raw = [
        (state.yaw_raw as i16).wrapping_neg(),
        state.pitch_raw as i16,
    ];
    Ok(TurretTaskOutcome::Continue)
}

///408BD0 walks the live list, not the spatial index or renderer-visible set.
fn acquire(
    manager: &mut EntityManager,
    id: u32,
    state: &mut GunTurretTaskState,
    world_fx: &mut WorldFx,
) -> Result<(), TurretTaskBlock> {
    state.tracking_disabled = true;
    state.selection_remaining_us =
        500_000 + i32::from(world_fx.next_shared_retail_random_u16()) * 4;
    let owner = manager.entity_mut(id).ok_or(TurretTaskBlock::Allocation)?;
    let infected = bits(owner, 0x2000)? != 0;
    let priorities = if infected {
        owner.capability_flags = (owner.capability_flags & !0x1004) | 8;
        INFECTED_PRIORITIES
    } else {
        if owner.capability_flags != state.original_capability {
            owner.collision.state_flags_at_0x08.overwrite(0x100, 0x100);
        }
        owner.capability_flags = state.original_capability;
        if state.normal_uses_infected_priorities {
            INFECTED_PRIORITIES
        } else {
            NORMAL_PRIORITIES
        }
    };
    let owner_position = owner.position_raw();
    let owner_collision = owner.collision.clone();
    let mut best = state.target_handle;
    let mut score = 0_i32;
    for candidate_id in manager.retail_live_order_ids() {
        let Some(candidate) = manager.iter_all().find(|entity| entity.id == candidate_id) else {
            continue;
        };
        let capability = candidate.capability_flags;
        if capability & 0xd == 0 || candidate_id == id {
            continue;
        }
        if bits(candidate, 0x4000)? != 0 {
            continue;
        }
        let word = candidate.collision.state_flags_at_0x08;
        if word.known_value_bits() == 0 {
            if word.masked(u32::MAX) == RetailRuntimeValue::Known(0) {
                continue;
            }
            return Err(TurretTaskBlock::Runtime("turret target nonzero state"));
        }
        match recent_relation_suppresses_pair(
            candidate_id,
            &candidate.collision,
            id,
            &owner_collision,
        ) {
            RetailRuntimeValue::Known(true) => continue,
            RetailRuntimeValue::Known(false) => {}
            RetailRuntimeValue::Unresolved => {
                return Err(TurretTaskBlock::Runtime("turret target relation"))
            }
        }
        if bits(candidate, 0x8000)? == 0 {
            continue;
        }
        let (_, mut distance) = math::displacement(owner_position, candidate.position_raw());
        if candidate_id == state.target_handle {
            distance = distance.wrapping_mul(6) >> 2;
        }
        if distance >= 0x1400 {
            continue;
        }
        let index = if capability & 8 != 0 {
            0
        } else if capability & 1 != 0 {
            1
        } else {
            2
        };
        let next = (0x1400 - distance).wrapping_add(priorities[index]);
        if score < next {
            score = next;
            best = candidate_id;
            if next > 0x1ffff {
                state.selection_remaining_us =
                    (i32::from(world_fx.next_shared_retail_random_u16()) + 0x2625a) * 16;
            }
        }
    }
    state.target_handle = best;
    if score >= 0x20001 {
        state.tracking_disabled = false;
        state.lost_target_remaining_us = 6_000_000;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
