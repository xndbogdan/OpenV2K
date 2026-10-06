//! Native living class-0 visits: 12DA0 -> DCA0/E870 -> A800/C690.
//!
//! The living timeout consumes the type's singleton selector word. Factory
//! wrecks retain their separate dying no-draw reselection owner. The manager
//! retains a committed-prefix marker so an interrupted visit cannot be adopted
//! afresh or silently discarded by Main Base abort.

use super::{authenticate_metadata, reselect_class0_actor, Class0ActorError};
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskVisit},
    common_mover::{
        type9_surface::{decay_actor_surface_timer_ms, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT},
        type9_tail::{
            apply_type9_ground_snap_raw, plan_common_master_motion,
            COMMON_MASTER_MOTION_REQUIRED_STATE_MASK, MASTER_GROUNDED_STATE_BIT,
        },
    },
    entity::{commit_common_master_motion, Entity, EntityManager},
    entity_behavior::{audited_behavior_program, BehaviorChoiceListSource, BehaviorContextRuntime},
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
    },
    entity_scheduler::*,
    main_base_abort::MainBaseAbortActorLease,
    resource_cache::ResourceCache,
    world_fx::WorldFx,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Class0ActorOwner {
    allocation: MainBaseAbortActorLease,
    context: BehaviorContextRuntime,
    primary: ActorTaskId,
}

impl Class0ActorOwner {
    pub const fn entity_id(self) -> u32 {
        self.allocation.entity_id
    }
    pub const fn allocation(self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub(crate) const fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }

    pub fn adopt(manager: &EntityManager, id: u32) -> Result<Self, Class0ActorError> {
        if !manager.native_class0_allocation_authenticates(id) {
            return Err(Class0ActorError::Allocation);
        }
        if manager.native_class0_has_pending_prefix(id) {
            return Err(Class0ActorError::Runtime("committed class0 prefix"));
        }
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(Class0ActorError::Allocation)?;
        let program = audited_behavior_program(0).ok_or(Class0ActorError::Graph)?;
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Class0ActorError::Graph);
        };
        if context.active_style().style_address() != program.initial_style.frame_address
            || context.choice_list_source()
                != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
            || context.target_handle_at_0x08() != RetailRuntimeValue::Known(None)
            || context.auxiliary_word_at_0x0c() != RetailRuntimeValue::Known(0)
            || bits(entity, DYING_STATE_BIT)? != 0
        {
            return Err(Class0ActorError::Graph);
        }
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .ok_or(Class0ActorError::Graph)?;
        let wrapper = entity
            .actor_tasks
            .wrapper_flags(primary)
            .ok_or(Class0ActorError::Graph)?;
        if !wrapper.alive
            || wrapper.in_callback
            || !matches!(
                entity.actor_tasks.task_state(primary),
                Some(ActorTaskRuntime::Class0Timer(_))
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
            return Err(Class0ActorError::Graph);
        }
        let allocation = manager
            .main_base_abort_actor_observation(id)
            .ok_or(Class0ActorError::Allocation)?
            .lease;
        Ok(Self {
            allocation,
            context,
            primary,
        })
    }
}

pub struct Class0ActorFrame<'a> {
    pub resources: &'a ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class0ActorOutcome {
    Waiting {
        entity_id: u32,
    },
    Advanced {
        entity_id: u32,
        callback_enabled: bool,
        detailed: bool,
        callback_elapsed_micros: u32,
    },
    Blocked {
        entity_id: u32,
        reason: Class0ActorError,
        prefix_committed: bool,
    },
    Pending {
        entity_id: u32,
    },
    Dropped {
        entity_id: u32,
    },
}

impl Class0ActorOutcome {
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

pub struct Class0ActorTick {
    pub outcome: Class0ActorOutcome,
    pub retained_owner: Option<Class0ActorOwner>,
}

pub fn tick_class0_actor_owner(
    manager: &mut EntityManager,
    owner: Class0ActorOwner,
    mut frame: Class0ActorFrame<'_>,
) -> Class0ActorTick {
    let id = owner.entity_id();
    if !manager.native_class0_allocation_authenticates(id)
        || !manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|now| now.lease == owner.allocation)
    {
        return Class0ActorTick {
            outcome: Class0ActorOutcome::Dropped { entity_id: id },
            retained_owner: None,
        };
    }
    if manager.native_class0_has_pending_prefix(id) {
        return Class0ActorTick {
            outcome: Class0ActorOutcome::Pending { entity_id: id },
            retained_owner: Some(owner),
        };
    }
    if Class0ActorOwner::adopt(manager, id) != Ok(owner) {
        return Class0ActorTick {
            outcome: Class0ActorOutcome::Dropped { entity_id: id },
            retained_owner: None,
        };
    }
    let result = run_frame(manager, owner, &mut frame);
    match result {
        Ok(outcome) => {
            manager.set_native_class0_pending_prefix(id, false);
            Class0ActorTick {
                outcome,
                retained_owner: Class0ActorOwner::adopt(manager, id).ok(),
            }
        }
        Err(reason) => Class0ActorTick {
            outcome: Class0ActorOutcome::Blocked {
                entity_id: id,
                reason,
                prefix_committed: manager.native_class0_has_pending_prefix(id),
            },
            retained_owner: Some(owner),
        },
    }
}

fn bits(entity: &Entity, mask: u32) -> Result<u32, Class0ActorError> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        _ => Err(Class0ActorError::Runtime("state bits")),
    }
}

fn run_frame(
    manager: &mut EntityManager,
    owner: Class0ActorOwner,
    frame: &mut Class0ActorFrame<'_>,
) -> Result<Class0ActorOutcome, Class0ActorError> {
    let id = owner.entity_id();
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Class0ActorError::Allocation)?;
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .cloned()
        .ok_or(Class0ActorError::Metadata)?;
    authenticate_metadata(entity.entity_type, &metadata)?;
    let state = bits(
        entity,
        REMOTE_OWNED_STATE_BIT
            | 0x1000
            | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    )?;
    if state & REMOTE_OWNED_STATE_BIT != 0 {
        return Err(Class0ActorError::Runtime("remote-owned class0 actor"));
    }
    let callback_enabled = state & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT != 0;
    let detailed = state & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0;
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
            u32::from(frame.world_fx.next_shared_retail_random_u16())
        })
    else {
        return Err(Class0ActorError::Runtime("scheduler state"));
    };
    if !manager.set_native_class0_pending_prefix(id, true) {
        return Err(Class0ActorError::Allocation);
    }
    let entity = manager.entity_mut(id).ok_or(Class0ActorError::Allocation)?;
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us: dt,
    } = prefix.flow
    else {
        return Ok(Class0ActorOutcome::Waiting { entity_id: id });
    };
    // 12DA0 resolves membership after the scheduler age write and before the
    // callback-enable branch. Valid local attachments preserve the class0 task.
    // 413039 explicitly excludes Type111 from parent/member resolution:
    // 416FF0's ordinary gate has +80=self, without a SubJ parent relation.
    if entity.entity_type == 111 {
        if entity.attached_to.is_some() {
            return Err(Class0ActorError::Runtime("gate has cargo attachment"));
        }
        if state & 0x1000 != 0 && manager.native_gate_self_relation(id) != Some(id) {
            return Err(Class0ActorError::Runtime("gate self relation"));
        }
    } else if state & 0x1000 != 0 {
        resolve_relation_prefix(manager, id)?;
    } else if entity.attached_to.is_some() {
        return Err(Class0ActorError::Runtime("relation state"));
    }
    if callback_enabled {
        let entity = manager.entity_mut(id).ok_or(Class0ActorError::Allocation)?;
        let RetailRuntimeValue::Known(mass) = common_scheduler_callback_mass(
            metadata.mass_raw,
            entity.collision.animation_offset_at_0xb2,
        ) else {
            return Err(Class0ActorError::Runtime("callback B2"));
        };
        entity.mass_raw = mass;
        run_callback(manager, owner, &metadata, frame, dt, detailed)?;
    }
    let entity = manager.entity_mut(id).ok_or(Class0ActorError::Allocation)?;
    commit_common_scheduler_post_callback(&mut entity.collision);
    if !matches!(
        metadata.constructor_sound_attachment_id,
        RetailRuntimeValue::Known(attachment)
            if entity.collision.constructor_sound_attachment_id_at_0x8c
                == RetailRuntimeValue::Known(attachment)
    ) {
        return Err(Class0ActorError::Runtime("positional sound attachment"));
    }
    // 413151..41316C follows +96 after the callback, before master motion,
    // including coarse visits and visits with the callback disabled.
    crate::entity_positional_audio::publish_constructor_sound_follow_position(entity);
    let motion = plan_common_master_motion(
        entity.position_raw(),
        entity.velocity_raw(),
        bits(entity, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)?,
        dt,
    );
    commit_common_master_motion(entity, motion);
    Ok(Class0ActorOutcome::Advanced {
        entity_id: id,
        callback_enabled,
        detailed,
        callback_elapsed_micros: dt,
    })
}

fn resolve_relation_prefix(manager: &mut EntityManager, id: u32) -> Result<(), Class0ActorError> {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Class0ActorError::Allocation)?;
    let parent_id = entity
        .attached_to
        .ok_or(Class0ActorError::Runtime("missing relation handle"))?;
    let parent = manager
        .iter_all()
        .find(|entity| entity.id == parent_id)
        .ok_or(Class0ActorError::Runtime("missing-parent relation repair"))?;
    if bits(parent, REMOTE_OWNED_STATE_BIT)? != 0 {
        let position = parent.position_raw();
        let entity = manager.entity_mut(id).ok_or(Class0ActorError::Allocation)?;
        entity.set_position_raw(position);
        entity.collision.state_flags_at_0x08.overwrite(0x20, 0x20);
        return Ok(());
    }
    if !matches!(&parent.sub_j_attachment_runtime, RetailRuntimeValue::Known(Some(sub_j))
        if sub_j.ordered_entity_ids().contains(&id))
    {
        return Err(Class0ActorError::Runtime("missing-member relation release"));
    }
    Ok(())
}

fn run_callback(
    manager: &mut EntityManager,
    owner: Class0ActorOwner,
    metadata: &EntityTypeRuntimeMetadata,
    frame: &mut Class0ActorFrame<'_>,
    dt: u32,
    detailed: bool,
) -> Result<(), Class0ActorError> {
    let id = owner.entity_id();
    let program = audited_behavior_program(0).ok_or(Class0ActorError::Graph)?;
    let entity = manager.entity_mut(id).ok_or(Class0ActorError::Allocation)?;
    let RetailRuntimeValue::Known(default_flags) = entity.collision.default_state_flags_at_0xc8
    else {
        return Err(Class0ActorError::Runtime("default environment flags"));
    };
    if metadata
        .initializer
        .as_ref()
        .map(|initializer| initializer.initializer_state_flags_raw)
        != Some(default_flags)
    {
        return Err(Class0ActorError::Runtime("default environment profile"));
    }
    let effective =
        (default_flags | program.authored_enable_policy) & !program.authored_disable_policy;
    if !detailed && effective & 0x2000 != 0 {
        entity.collision.state_flags_at_0x08.overwrite(0x40000, 0);
        return Ok(());
    }
    if detailed && effective & 0x3000 == 0x2000 {
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x40000, 0x40000);
    }
    let Some(ActorTaskRuntime::Class0Timer(task)) =
        entity.actor_tasks.task_state_mut(owner.primary)
    else {
        return Err(Class0ActorError::Graph);
    };
    let expired = task.advance_prefix(dt);
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id: owner.primary,
    };
    entity
        .actor_tasks
        .begin_exact_visit_with(visit, |_| ())
        .ok_or(Class0ActorError::Graph)?;
    let survived = entity.actor_tasks.finish_exact_visit(visit);
    if survived && expired && bits(entity, 0x1000)? == 0 {
        reselect_class0_actor(entity, metadata, &mut || {
            u32::from(frame.world_fx.next_shared_retail_random_u16())
        })?;
    }
    if detailed {
        let record = frame
            .resources
            .global_entity_type(entity.entity_type as usize)
            .ok_or(Class0ActorError::Metadata)?;
        let RetailRuntimeValue::Known(health_raw) = entity.collision.health_raw else {
            return Err(Class0ActorError::Runtime("health"));
        };
        let initial_health = i32::from_le_bytes(record.raw_header[0x14..0x18].try_into().unwrap());
        let visible = health_raw >= initial_health / 2 && bits(entity, 0x800)? != 0;
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
                .queue_fixed_positional_sound_raw(sound, entity.position_raw());
        }
    }
    // These admitted class0 profiles omit E640, F70, gravity/drag and infected-ground
    // damage. This checks the consumed branch bits, not surface position.
    if effective & (0x10 | 0x4000 | 4 | 8 | 0x800) != 0x4004 {
        return Err(Class0ActorError::Runtime("environment callback profile"));
    }
    if effective & 2 != 0 {
        let terrain = frame
            .resources
            .level_terrain()
            .ok_or(Class0ActorError::Runtime("terrain"))?;
        let mut position = entity.position_raw();
        let mut velocity = entity.velocity_raw();
        let mut grounded = 0;
        apply_type9_ground_snap_raw(&mut position, &mut velocity, &mut grounded, terrain);
        entity.set_position_raw(position);
        entity.set_velocity_raw(velocity);
        if effective & 0x40 != 0 {
            let RetailRuntimeValue::Known(slot) = entity.collision.active_model_slot() else {
                return Err(Class0ActorError::Runtime("active model selector"));
            };
            let model_id = entity
                .model_in_slot(usize::from(slot))
                .ok_or(Class0ActorError::Runtime("active model"))?;
            let model = frame
                .resources
                .global_model(model_id)
                .ok_or(Class0ActorError::Runtime("active model header08"))?;
            position[1] = position[1].wrapping_add(model.radius as i16);
        }
        entity.set_position_raw(position);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(MASTER_GROUNDED_STATE_BIT, grounded);
    }
    if bits(entity, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT)? == 0 {
        let RetailRuntimeValue::Known(profile) = metadata.common_world_effects else {
            return Err(Class0ActorError::Runtime("surface profile"));
        };
        if profile.surface_selectors != [0, 0] {
            return Err(Class0ActorError::Runtime("surface selectors"));
        }
        let RetailRuntimeValue::Known(timer) = entity.surface_lifetime_timer_ms_at_0x48 else {
            return Err(Class0ActorError::Runtime("surface timer"));
        };
        entity.surface_lifetime_timer_ms_at_0x48 =
            RetailRuntimeValue::Known(decay_actor_surface_timer_ms(timer, dt));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
