//! Type10's10C10 -> alternate11 -> C660/404360, then404460 and C750/BAC0.
//! A falling Primary has lifetime zero. Only the contact callbacks select the
//! explosion variant; neither elapsed time nor a guessed ground Y ends it.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit, PreparedActorTask},
    common_mover::type9_tail::COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
    damage::DamagePacket,
    entity::EntityManager,
    entity_behavior::{
        audited_behavior_program, initial_behavior_state_policy, translate_state_policy,
        TUMBLE_OUT_OF_SKY_COMPLETION_STYLE,
    },
    entity_collision_state::{DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT},
    entity_scheduler::*,
    main_base_abort::MainBaseAbortActorLease,
    radial_damage::RadialDamageTemplate,
    resource_cache::ResourceCache,
    world_fx::WorldFx,
};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type10DeathBlock {
    Allocation,
    Graph,
    Metadata,
    Runtime(&'static str),
    World(Intro2Type10Block),
    SubG(crate::sub_g_runtime::Type13SubGFrameBlock),
    /// A class63 row's death is the shared BAF0/BC90 terminal, which needs the
    /// caller's radial owner; it never publishes this row's class11 Tumble.
    AutoPilotCarrier,
    /// The class63 terminal blocked after its committed prefix.
    AutoPilot(Box<crate::class49_terminal::Class49TerminalBlock>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type10TumbleTask {
    elapsed_ms: u32,
    terminal: Option<TerminalPhase>,
}
impl Intro2Type10TumbleTask {
    pub const fn elapsed_ms(self) -> u32 {
        self.elapsed_ms
    }
    fn new() -> Self {
        Self {
            elapsed_ms: 0,
            terminal: None,
        }
    }
    fn advance(&mut self, elapsed_micros: u32) -> i32 {
        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1000);
        // 4044B5 uses signed division of the retained dword, not frame dt.
        3i32.wrapping_sub((self.elapsed_ms as i32) / 1000).max(1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type10TumbleOwner {
    allocation: MainBaseAbortActorLease,
    visit: ActorTaskVisit,
    pending: bool,
}
impl Intro2Type10TumbleOwner {
    pub(crate) const fn actor_lease(self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub const fn entity_id(self) -> u32 {
        self.allocation.entity_id
    }
    pub(crate) const fn has_pending_prefix(self) -> bool {
        self.pending
    }
    pub(crate) fn park_contact_prefix(&mut self) {
        self.pending = true;
    }
    pub(crate) const fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }
    pub fn adopt(manager: &EntityManager, id: u32) -> Result<Self, Intro2Type10DeathBlock> {
        let entity = manager
            .iter_all()
            .find(|e| e.id == id)
            .ok_or(Intro2Type10DeathBlock::Allocation)?;
        if !intro2_type10_allocation_authenticates(entity) {
            return Err(Intro2Type10DeathBlock::Allocation);
        }
        let visit = current_visit(entity)?;
        Ok(Self {
            allocation: manager
                .main_base_abort_actor_observation(id)
                .ok_or(Intro2Type10DeathBlock::Allocation)?
                .lease,
            visit,
            pending: false,
        })
    }
}

fn current_visit(entity: &Entity) -> Result<ActorTaskVisit, Intro2Type10DeathBlock> {
    use Intro2Type10DeathBlock as Block;
    if !matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(context))
        if context.descriptor_address() == 0x004c88d8
            && matches!(context.active_style().style_address(), 0x004c7f60 | 0x004c7fa8))
        || entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Secondary)
            .is_some()
        || entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary)
            .is_some()
    {
        return Err(Block::Graph);
    }
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(Block::Graph)?;
    if !matches!(
        entity.actor_tasks.task_state(task_id),
        Some(ActorTaskRuntime::TumbleOutOfSky(_))
    ) {
        return Err(Block::Graph);
    }
    Ok(ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    })
}

fn bits(entity: &Entity, mask: u32) -> Result<u32, Intro2Type10DeathBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        _ => Err(Intro2Type10DeathBlock::Runtime("state bits")),
    }
}

/// No-op death gates precede metadata/component reads. All constructor inputs
/// are authenticated before health0/dying; the alternate uses no selector draw.
pub fn publish_intro2_type10_standard_death(
    manager: &mut EntityManager,
    id: u32,
    world_fx: &mut WorldFx,
) -> Result<Option<Intro2Type10TumbleOwner>, Intro2Type10DeathBlock> {
    use Intro2Type10DeathBlock as Block;
    let entity = manager
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(Block::Allocation)?;
    if !intro2_type10_allocation_authenticates(entity) {
        return Err(Block::Allocation);
    }
    if bits(entity, REMOTE_OWNED_STATE_BIT | DYING_STATE_BIT)? != 0 {
        return Ok(None);
    }
    let profile = super::type10_profile(entity).ok_or(Block::Allocation)?;
    if profile.alternate_behavior_class() != 11 {
        return Err(Block::AutoPilotCarrier);
    }
    let metadata = manager
        .type_runtime_metadata(profile.entity_type())
        .ok_or(Block::Metadata)?;
    authenticate_metadata(profile, metadata).map_err(|_| Block::Metadata)?;
    if entity.collision.constructor_sound_attachment_id_at_0x8c != RetailRuntimeValue::Known(None) {
        return Err(Block::Runtime("constructor sound attachment"));
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    // Exact +2C is null for these living/failed Search styles, and both
    // alternate styles. An arbitrary null-looking foreign style is not admitted.
    if !matches!(
        context.active_style().style_address(),
        0x4c7a50 | 0x4c7a98 | 0x4c7ae0 | 0x4c74f8
    ) {
        return Err(Block::Runtime("unaudited death style"));
    }
    let program = audited_behavior_program(11).ok_or(Block::Metadata)?;
    let selected = context
        .reselect_audited_type_default(program, 0, program.initial_style)
        .ok_or(Block::Graph)?;
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(selected));
    let policy = initial_behavior_state_policy(program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
    let RetailRuntimeValue::Known(Some(sub_g)) = &mut entity.sub_g_06070_runtime else {
        unreachable!("authenticated native G allocation")
    };
    // Successful6030/06070 before404360-specific G1B970(1). E/K/L/D and
    // transient B2 survive; there is no class12 +500 vertical-velocity write.
    let word = world_fx.next_shared_retail_random_u16();
    sub_g.apply_shared_06070_sub_g_branch(700 + i32::from(word >> 8), 0);
    sub_g.enter_tumble();
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Primary,
        PreparedActorTask::new(ActorTaskRuntime::TumbleOutOfSky(
            Intro2Type10TumbleTask::new(),
        )),
    );
    Intro2Type10TumbleOwner::adopt(manager, id).map(Some)
}

pub type Intro2Type10TumbleFrame<'a> = Intro2Type10Frame<'a>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type10TumbleOutcome {
    Waiting {
        entity_id: u32,
    },
    Advanced {
        entity_id: u32,
        detailed: bool,
        callback_elapsed_micros: u32,
    },
    Pending {
        entity_id: u32,
    },
    Dropped {
        entity_id: u32,
    },
    Blocked {
        entity_id: u32,
        reason: Intro2Type10DeathBlock,
        prefix_committed: bool,
    },
}
impl Intro2Type10TumbleOutcome {
    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::Waiting { entity_id }
            | Self::Advanced { entity_id, .. }
            | Self::Pending { entity_id }
            | Self::Dropped { entity_id }
            | Self::Blocked { entity_id, .. } => *entity_id,
        }
    }
}
pub struct Intro2Type10TumbleTick {
    pub outcome: Intro2Type10TumbleOutcome,
    pub retained_owner: Option<Intro2Type10TumbleOwner>,
}

pub fn tick_intro2_type10_tumble(
    manager: &mut EntityManager,
    mut owner: Intro2Type10TumbleOwner,
    frame: Intro2Type10TumbleFrame<'_>,
) -> Intro2Type10TumbleTick {
    let id = owner.entity_id();
    if !Intro2Type10TumbleOwner::adopt(manager, id)
        .ok()
        .is_some_and(|current| {
            current.allocation == owner.allocation && current.visit == owner.visit
        })
    {
        return Intro2Type10TumbleTick {
            outcome: Intro2Type10TumbleOutcome::Dropped { entity_id: id },
            retained_owner: None,
        };
    }
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    let terminal = matches!(entity.actor_tasks.task_state(owner.visit.task_id),
        Some(ActorTaskRuntime::TumbleOutOfSky(task)) if task.terminal.is_some());
    if owner.pending || terminal {
        return Intro2Type10TumbleTick {
            outcome: Intro2Type10TumbleOutcome::Pending { entity_id: id },
            retained_owner: Some(owner),
        };
    }
    let mut committed = false;
    let outcome = match run_frame(manager, owner, frame, &mut committed) {
        Ok(outcome) => outcome,
        Err(reason) => {
            owner.pending = committed;
            Intro2Type10TumbleOutcome::Blocked {
                entity_id: id,
                reason,
                prefix_committed: committed,
            }
        }
    };
    Intro2Type10TumbleTick {
        outcome,
        retained_owner: Some(owner),
    }
}

fn run_frame(
    manager: &mut EntityManager,
    owner: Intro2Type10TumbleOwner,
    mut frame: Intro2Type10TumbleFrame<'_>,
    committed: &mut bool,
) -> Result<Intro2Type10TumbleOutcome, Intro2Type10DeathBlock> {
    use Intro2Type10DeathBlock as Block;
    let id = owner.entity_id();
    let profile = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .and_then(super::type10_profile)
        .ok_or(Block::Allocation)?;
    let metadata = manager
        .type_runtime_metadata(profile.entity_type())
        .cloned()
        .ok_or(Block::Metadata)?;
    authenticate_metadata(profile, &metadata).map_err(|_| Block::Metadata)?;
    let entity = manager.entity_mut(id).unwrap();
    let state = bits(
        entity,
        REMOTE_OWNED_STATE_BIT
            | 0x1000
            | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            | COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
    )?;
    if state & (REMOTE_OWNED_STATE_BIT | 0x1000) != 0 || entity.attached_to.is_some() {
        return Err(Block::Runtime("local unattached owner"));
    }
    if entity.collision.constructor_sound_attachment_id_at_0x8c != RetailRuntimeValue::Known(None)
        || entity.sub_j_attachment_runtime != RetailRuntimeValue::Known(None)
    {
        return Err(Block::Runtime("absent voice/Sub-J"));
    }
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
            u32::from(frame.world_fx.next_shared_retail_random_u16())
        })
    else {
        return Err(Block::Runtime("scheduler"));
    };
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    *committed = true;
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us: dt,
    } = prefix.flow
    else {
        return Ok(Intro2Type10TumbleOutcome::Waiting { entity_id: id });
    };
    let detailed = state & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0;
    if state & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT == 0 {
        super::live::commit_motion(entity, dt).map_err(Block::World)?;
    } else {
        let RetailRuntimeValue::Known(mass) = common_scheduler_callback_mass(
            metadata.mass_raw,
            entity.collision.animation_offset_at_0xb2,
        ) else {
            return Err(Block::Runtime("callback B2"));
        };
        entity.mass_raw = mass;
        let effective = super::world::effective_flags(entity).map_err(Block::World)?;
        let multiplier = entity
            .actor_tasks
            .begin_exact_visit_with(owner.visit, |runtime| {
                let ActorTaskRuntime::TumbleOutOfSky(task) = runtime else {
                    unreachable!()
                };
                task.advance(dt)
            })
            .ok_or(Block::Graph)?;
        let result = if detailed {
            tick_detailed_components(entity, multiplier, dt)
        } else {
            Ok(())
        };
        if !entity.actor_tasks.finish_exact_visit(owner.visit) {
            return Err(Block::Graph);
        }
        result?;
        super::world::finish(manager, id, &metadata, &mut frame, dt, state, effective)
            .map_err(Block::World)?;
    }
    Ok(Intro2Type10TumbleOutcome::Advanced {
        entity_id: id,
        detailed,
        callback_elapsed_micros: dt,
    })
}

fn tick_detailed_components(
    entity: &mut Entity,
    multiplier: i32,
    dt: u32,
) -> Result<(), Intro2Type10DeathBlock> {
    use Intro2Type10DeathBlock as Block;
    let [mut heading, mut pitch, mut roll] = entity.rotation_heading_pitch_roll_raw();
    let yaw_step = multiplier.wrapping_mul((dt >> 8) as i32);
    let roll_step = multiplier.wrapping_mul((dt >> 7) as i32);
    roll = roll.wrapping_add(roll_step as i16);
    heading = heading.wrapping_add(yaw_step as i16);
    pitch = pitch.wrapping_add(roll_step.wrapping_add(yaw_step) as i16);
    entity.set_rotation_heading_pitch_roll_raw([heading, pitch, roll]);
    // Direct018A0 has no target prelude, Sub-D, 019C0 or14E0/1780 writes.
    let RetailRuntimeValue::Known(Some(sub_g)) = &mut entity.sub_g_06070_runtime else {
        return Err(Block::Runtime("Sub-G"));
    };
    sub_g.advance_tumble_mode().map_err(Block::SubG)?;
    let velocity_y = entity.velocity_raw()[1];
    let position = entity.position_raw();
    let runtime = entity
        .intro2_type10_runtime
        .as_mut()
        .ok_or(Block::Allocation)?;
    runtime.sub_k_output_raw = crate::gkl_common_mover::component_outputs::advance_sub_k(
        runtime.sub_k_output_raw,
        runtime.sub_k_smoothed_raw,
        velocity_y,
    );
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31 else {
        return Err(Block::Runtime("Sub-L retained basis"));
    };
    runtime.sub_l_output_raw = crate::gkl_common_mover::component_outputs::advance_sub_l(
        runtime.sub_l_output_raw,
        runtime.sub_l_exact_raw,
        position,
        runtime.sub_l_target_raw,
        basis,
        SUB_L,
    );
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type10TumbleContact {
    Terrain,
    Static,
    Water,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type10TerminalReceipt {
    pub entity_id: u32,
    pub position_raw: [i16; 3],
    pub radial_damage: RadialDamageTemplate,
    visit: ActorTaskVisit,
    continuation_id: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalPhase {
    Issued(Intro2Type10TerminalReceipt),
    Claimed(Intro2Type10TerminalReceipt),
}
static NEXT_TERMINAL_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn terminal_is_pending(entity: &Entity) -> bool {
    matches!(entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::TumbleOutOfSky(task)) if task.terminal.is_some())
}

/// A source-authenticated +10/+14/+1C callback already won contact admission.
/// Pair style+18 is null and intentionally has no entry in this enum.
pub fn begin_intro2_type10_tumble_contact(
    manager: &mut EntityManager,
    id: u32,
    resources: &ResourceCache,
    fx: &mut WorldFx,
    _contact: Intro2Type10TumbleContact,
) -> Result<Option<Intro2Type10TerminalReceipt>, Intro2Type10DeathBlock> {
    use Intro2Type10DeathBlock as Block;
    let owner = Intro2Type10TumbleOwner::adopt(manager, id)?;
    let entity = manager.entity_mut(id).unwrap();
    let ActorTaskRuntime::TumbleOutOfSky(task) =
        entity.actor_tasks.task_state(owner.visit.task_id).unwrap()
    else {
        unreachable!()
    };
    if task.terminal.is_some() {
        return Ok(None);
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    if context.active_style().style_address() != 0x4c7f60 {
        return Err(Block::Graph);
    }
    let flags = bits(entity, 0x8000_6000)?;
    if flags & REMOTE_OWNED_STATE_BIT != 0 {
        return Err(Block::Runtime("local terminal owner"));
    }
    if entity.capability_flags != 8 {
        return Err(Block::Metadata);
    }
    let model_slot = crate::entity_collision_state::active_model_slot_from_state_flags(flags);
    let extent = entity.model_slots[model_slot]
        .and_then(|id| resources.global_model(id))
        .ok_or(Block::Metadata)?
        .radius;
    let own_type = entity.entity_type;
    let record = resources
        .global_entity_type(own_type as usize)
        .ok_or(Block::Metadata)?;
    let header = &record.raw_header;
    let word = |at: usize| i16::from_le_bytes(header[at..at + 2].try_into().unwrap());
    let dword = |at: usize| i32::from_le_bytes(header[at..at + 4].try_into().unwrap());
    let selected = context
        .reselect_audited_type_default(
            audited_behavior_program(11).unwrap(),
            1,
            TUMBLE_OUT_OF_SKY_COMPLETION_STYLE,
        )
        .ok_or(Block::Graph)?;
    let receipt = Intro2Type10TerminalReceipt {
        entity_id: id,
        position_raw: entity.position_raw(),
        visit: owner.visit,
        continuation_id: NEXT_TERMINAL_ID.fetch_add(1, Ordering::Relaxed),
        radial_damage: RadialDamageTemplate {
            inner_radius_raw: word(0x50),
            outer_radius_raw: word(0x52),
            impulse_raw: dword(0x54),
            packet: DamagePacket {
                channels: [dword(0x58), dword(0x5c)],
                amounts_raw: [dword(0x60), dword(0x64)],
            },
            trailing_raw: [own_type as i32, id as i32],
        },
    };
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(selected));
    let policy = translate_state_policy(0x11);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.clear_bits);
    let ActorTaskRuntime::TumbleOutOfSky(task) = entity
        .actor_tasks
        .task_state_mut(owner.visit.task_id)
        .unwrap()
    else {
        unreachable!()
    };
    task.terminal = Some(TerminalPhase::Issued(receipt));
    fx.emit_intro2_type10_terminal_raw(
        receipt.position_raw,
        id,
        extent,
        resources.level_terrain().map(|t| t.sea_level_raw()),
    );
    Ok(Some(receipt))
}

fn receipt_current(manager: &EntityManager, receipt: &Intro2Type10TerminalReceipt) -> bool {
    manager
        .iter_all()
        .find(|e| e.id == receipt.entity_id)
        .is_some_and(|entity| {
            intro2_type10_allocation_authenticates(entity)
                && current_visit(entity).ok() == Some(receipt.visit)
                && matches!(entity.current_behavior_context,RetailRuntimeValue::Known(Some(context))
            if context.active_style().style_address()==0x4c7fa8)
                && entity.position_raw() == receipt.position_raw
        })
}
pub fn claim_intro2_type10_terminal(
    manager: &mut EntityManager,
    receipt: &Intro2Type10TerminalReceipt,
) -> bool {
    if !receipt_current(manager, receipt) {
        return false;
    }
    let entity = manager.entity_mut(receipt.entity_id).unwrap();
    let ActorTaskRuntime::TumbleOutOfSky(task) = entity
        .actor_tasks
        .task_state_mut(receipt.visit.task_id)
        .unwrap()
    else {
        unreachable!()
    };
    if task.terminal != Some(TerminalPhase::Issued(*receipt)) {
        return false;
    }
    task.terminal = Some(TerminalPhase::Claimed(*receipt));
    true
}
pub(crate) fn active_terminal_receipt(
    manager: &EntityManager,
    receipt: &Intro2Type10TerminalReceipt,
) -> bool {
    receipt_current(manager,receipt) && manager.iter_all().find(|e|e.id==receipt.entity_id)
        .is_some_and(|entity|matches!(entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::TumbleOutOfSky(task)) if task.terminal==Some(TerminalPhase::Claimed(*receipt))))
}
pub fn finish_intro2_type10_terminal(
    manager: &mut EntityManager,
    receipt: Intro2Type10TerminalReceipt,
) -> bool {
    if !receipt_current(manager, &receipt) {
        return false;
    }
    let entity = manager.entity_mut(receipt.entity_id).unwrap();
    if !matches!(entity.actor_tasks.task_state(receipt.visit.task_id),Some(ActorTaskRuntime::TumbleOutOfSky(task))
        if task.terminal==Some(TerminalPhase::Claimed(receipt)))
    {
        return false;
    }
    for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        entity.actor_tasks.clear_slot(slot);
    }
    entity.mark_actor_deferred_destroy_pending();
    manager.queue_actor_deferred_destroy(receipt.entity_id);
    true
}

#[cfg(test)]
mod tests;
