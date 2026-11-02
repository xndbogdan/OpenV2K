//! Type29's class21 Mobbing Target (descriptor `4C8910`).
//!
//! * Style `4C7C48` (`FUN_0040B460`) is Guard Location's `B5A0` with slot0
//!   `02B10`'s 5000-ms retarget instead of `02E20`: the axis `+04` copy,
//!   slot2 cleared, slot1 `01F80`'s search, then slot0, each through `06070`.
//!   `01FB0`'s one-in-four gate hands its first eligible candidate to `C7D0`.
//! * Style `4C7C90` (`FUN_0040B2C0`) clears slots 1 and 2 and, with a Sub-G,
//!   installs `03900`'s lifetime-0 approach on the context target. Its
//!   `03A00` tick runs `01430` first, then: a zero mover, a missing, dying or
//!   zero-state target, or `23030`'s axis cube failing returns a `9C01`
//!   singleton (`C690`); within 0x200 in X and Z, more than 300 above the
//!   target and `1E930`'s forward half-space passing, `4BE108` (`9C00`,
//!   `C770` into the dive); otherwise `1B940` sets G `+38` to the target's
//!   Y + 500.
//! * Style `4C7CD8` (`FUN_0040B340`) plays type `+9A` (none for Type29),
//!   clears slots 1 and 2 and installs `04BE0`'s lifetime-0 dive: `06030`,
//!   then `1B980(G, 1)`. Its `04D50` tick returns `9C00` (`C750`, back to the
//!   approach) when the target is gone, when the body's underside is within
//!   200 of the waved surface or below the target's Y, or after `01430` when
//!   the target is more than 0x300 away in X or Z. The record's `+1C` cue is
//!   type `+9C`, zero for Type29, so its random gate never draws.
//! * Both later styles' `+18` pair hook `40CF00` gives a capability-bit0 body
//!   a checked `{1, 6000}` hit of this row's kind ([`pair_hit_packet`]).

use super::{
    intro2_type10_allocation_authenticates, mover::MoverFrame, type10_profile, Intro2Type10Block,
    Type10Profile,
};
use crate::{
    actor_task_dispatcher::{prepare_guard_location_acquisition_runtime_task, ActorTaskRuntime},
    actor_task_owner::{ActorTaskOwner, ActorTaskSlot, ActorTaskVisit, PreparedActorTask},
    common_mover::target_prelude::CommonMoverTrackedTargetSnapshot,
    damage::DamagePacket,
    entity::{Entity, EntityManager},
    entity_behavior::{
        audited_behavior_style, behavior_program, BehaviorContextRuntime,
        BehaviorDescriptorIdentity,
    },
    entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT},
    guard_location_owner::{
        apply_guard_location_setup,
        native::{tick_native_guard_acquisition_with, NativeGuardBlock},
        GuardLocationPrimaryContext, GuardLocationSetupRequest, GuardLocationTaskConstructorInputs,
        GuardLocationTaskRole,
    },
    shared_retarget_mover::SharedRetargetTaskState,
    wander_near_location::{WanderNearPrivateState, WANDER_NEAR_NO_TRACKED_ENTITY},
    world_fx::WorldFx,
    wrapped_axis_range::{within_wrapped_axis_range, WrappedAxisRange},
};
use v2k_formats::collision::CommonAxisDescriptor;

pub(crate) const MOBBING_CLASS_ID: u32 = 21;
pub(crate) const SEARCHING_STYLE_ADDRESS: u32 = 0x004c_7c48;
pub(crate) const APPROACH_STYLE_ADDRESS: u32 = 0x004c_7c90;
pub(crate) const DIVE_STYLE_ADDRESS: u32 = 0x004c_7cd8;
/// `B460`'s slot0 `02B10` lifetime.
pub(crate) const SEARCHING_RETARGET_LIFETIME_MS: u32 = 5000;
/// `03A00`: the dive starts within this X/Z box, more than 300 above.
const DIVE_ENTRY_BOX_RAW: i32 = 0x200;
const DIVE_ENTRY_HEIGHT_RAW: i32 = 300;
/// `03A00`: G `+38` holds this far above the target.
const APPROACH_HOVER_RAW: i32 = 500;
/// `04D50`: the dive keeps going within this X/Z box.
const DIVE_KEEP_BOX_RAW: i32 = 0x300;
/// `04D50`: the dive ends within this clearance of the surface.
const DIVE_SURFACE_CLEARANCE_RAW: i32 = 200;
/// `40CF00`'s packet: channel 1, 6000.
pub(crate) const PAIR_HIT_DAMAGE_RAW: i32 = 6000;

/// The `0x24`-byte `01350` record shared by `03900` and `04BE0`: the tracked
/// target and its route point. Lifetime 0: neither task ages out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MobbingTaskState {
    private_state: WanderNearPrivateState,
}

impl MobbingTaskState {
    const fn new(current_position_raw: [i16; 3], target_id: u32) -> Self {
        Self {
            private_state: WanderNearPrivateState::tracked_entity(current_position_raw, target_id),
        }
    }
    pub const fn private_state(self) -> WanderNearPrivateState {
        self.private_state
    }
    pub const fn target_id(self) -> u32 {
        self.private_state.tracked_entity_handle
    }
    /// The approach's `02CA0` static redirect writes its route record.
    pub(crate) fn replace_private_state(&mut self, private_state: WanderNearPrivateState) {
        self.private_state = private_state;
    }
}

/// `40CF00` against a capability-bit0 body: `{ch 1, 6000}` of `kind`.
pub(crate) const fn pair_hit_packet() -> DamagePacket {
    DamagePacket {
        channels: [1, 0],
        amounts_raw: [PAIR_HIT_DAMAGE_RAW, 0],
    }
}

fn context_style(entity: &Entity) -> Option<(u32, u32)> {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return None;
    };
    let program = behavior_program(MOBBING_CLASS_ID)?;
    (context.descriptor() == BehaviorDescriptorIdentity::Named(program)).then(|| {
        (
            context.style_table_index_raw_at_0x10(),
            context.active_style().style_address(),
        )
    })
}

/// `B460`'s graph: the retarget in slot0 and the search in slot1.
pub(super) fn searching_graph_authenticates(entity: &Entity) -> bool {
    intro2_type10_allocation_authenticates(entity)
        && context_style(entity) == Some((0, SEARCHING_STYLE_ADDRESS))
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(_))
        )
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::GuardLocationAcquisition(_))
        )
        && entity.actor_task_state(ActorTaskSlot::Tertiary).is_none()
}

/// `B2C0`'s or `B340`'s graph: one task on the context target.
pub(crate) fn pursuing_graph_authenticates(entity: &Entity) -> bool {
    if !intro2_type10_allocation_authenticates(entity) {
        return false;
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return false;
    };
    let RetailRuntimeValue::Known(Some(target)) = context.target_handle_at_0x08() else {
        return false;
    };
    let task = match (
        context_style(entity),
        entity.actor_task_state(ActorTaskSlot::Primary),
    ) {
        (Some((1, APPROACH_STYLE_ADDRESS)), Some(ActorTaskRuntime::MobbingApproach(task)))
        | (Some((2, DIVE_STYLE_ADDRESS)), Some(ActorTaskRuntime::MobbingDive(task))) => task,
        _ => return false,
    };
    task.target_id() == target
        && entity.actor_task_state(ActorTaskSlot::Secondary).is_none()
        && entity.actor_task_state(ActorTaskSlot::Tertiary).is_none()
}

/// `B460` after its axis `+04` copy: slot2 cleared, slot1 `01F80`, slot0
/// `02B10`, each through `suffix` (`06070`). Guard's phase list has the same
/// slots, order and 5000-ms lifetime; only slot0's constructor differs.
pub(super) fn apply_searching_setup(
    actor_tasks: &mut ActorTaskOwner<ActorTaskRuntime>,
    axis: CommonAxisDescriptor,
    position: [i16; 3],
    mut suffix: impl FnMut(
        PreparedActorTask<ActorTaskRuntime>,
    ) -> Result<PreparedActorTask<ActorTaskRuntime>, ()>,
) -> Result<(), ()> {
    apply_guard_location_setup(
        actor_tasks,
        &mut GuardLocationPrimaryContext::new(0),
        GuardLocationSetupRequest {
            authored_primary_context_word: axis.raw_word_at_0x04,
            // Style 4C7C48's +44 is zero: 01F80 stores no filter override.
            search_task_context_word: 0,
        },
        |preparation| {
            let prepared = match (preparation.task.role, preparation.constructor_inputs) {
                (GuardLocationTaskRole::AcquireCandidate, _) => {
                    prepare_guard_location_acquisition_runtime_task(preparation).map_err(drop)?
                }
                (
                    GuardLocationTaskRole::WanderAroundAnchor,
                    GuardLocationTaskConstructorInputs::WanderAroundAnchor {
                        lifetime_ms: SEARCHING_RETARGET_LIFETIME_MS,
                    },
                ) => PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                    SharedRetargetTaskState::new(position, SEARCHING_RETARGET_LIFETIME_MS),
                )),
                _ => return Err(()),
            };
            suffix(prepared)
        },
    )
    .map_err(drop)
}

/// `C6B0(variant)` from a living Mobbing context: the same target and
/// auxiliary word, then that style's initializer (`B2C0` or `B340`).
fn switch_style(
    entity: &mut Entity,
    variant: u8,
    target_id: Option<u32>,
    world_fx: &mut WorldFx,
) -> Result<(), Intro2Type10Block> {
    use Intro2Type10Block as Block;
    let profile = type10_profile(entity).ok_or(Block::Allocation)?;
    let RetailRuntimeValue::Known(Some(current)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    let program = behavior_program(MOBBING_CLASS_ID).ok_or(Block::Graph)?;
    if current.descriptor() != BehaviorDescriptorIdentity::Named(program) {
        return Err(Block::Graph);
    }
    let style = *audited_behavior_style(MOBBING_CLASS_ID, variant).ok_or(Block::Graph)?;
    let target = match target_id {
        Some(target) => RetailRuntimeValue::Known(Some(target)),
        None => current.target_handle_at_0x08(),
    };
    let RetailRuntimeValue::Known(Some(target_handle)) = target else {
        return Err(Block::Behavior("Mobbing context target"));
    };
    let context = BehaviorContextRuntime::named_audited(
        program,
        u32::from(variant),
        current.choice_list_source(),
        target,
        current.auxiliary_word_at_0x0c(),
        style,
    )
    .ok_or(Block::Graph)?;
    // C6B0 publishes the context, then the initializer. Both styles' +34/+38
    // masks are zero.
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let position = entity.position_raw();
    let Entity {
        actor_tasks,
        sub_g_06070_runtime,
        ..
    } = entity;
    let RetailRuntimeValue::Known(Some(sub_g)) = sub_g_06070_runtime else {
        return Err(Block::Runtime("Sub-G allocation"));
    };
    // Type29's +9A cue is absent (authenticated), so B340 plays nothing.
    actor_tasks.clear_slot(ActorTaskSlot::Secondary);
    actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
    let task = MobbingTaskState::new(position, target_handle);
    // 06030 -> 06070: G1B970(0), G1B940(0) draws one word, G1B980(0).
    let word = world_fx.next_shared_retail_random_u16();
    let g = profile.sub_g();
    sub_g.apply_shared_06070_sub_g_branch(
        profile.sub_g_randomized_target_base() + i32::from(word >> 8),
        u32::from_le_bytes([g[0], g[1], g[2], g[3]]),
    );
    let runtime = match variant {
        1 => ActorTaskRuntime::MobbingApproach(task),
        2 => {
            // 04BE0's suffix after 06030: no H; G1B980(1); no K.
            sub_g.apply_1b980_mode(g, 1);
            ActorTaskRuntime::MobbingDive(task)
        }
        _ => return Err(Block::Graph),
    };
    actor_tasks.replace_prepared(ActorTaskSlot::Primary, PreparedActorTask::new(runtime));
    Ok(())
}

/// `C7D0` from `01FB0`'s handoff: target `+08`, then `C6B0(1)`.
fn publish_approach(
    entity: &mut Entity,
    target_id: u32,
    world_fx: &mut WorldFx,
) -> Result<(), NativeGuardBlock> {
    if context_style(entity) != Some((0, SEARCHING_STYLE_ADDRESS)) {
        return Err(NativeGuardBlock::Graph);
    }
    switch_style(entity, 1, Some(target_id), world_fx).map_err(|_| NativeGuardBlock::Pursuit)
}

/// One `01FB0` visit of the Mobbing search; returns whether `C7D0` took a
/// candidate into the approach.
pub(super) fn acquire(
    manager: &mut EntityManager,
    id: u32,
    world_fx: &mut WorldFx,
) -> Result<bool, Intro2Type10Block> {
    use Intro2Type10Block as Block;
    if !manager
        .iter_all()
        .find(|entity| entity.id == id)
        .is_some_and(searching_graph_authenticates)
    {
        return Err(Block::Graph);
    }
    let taken = tick_native_guard_acquisition_with(manager, id, world_fx, publish_approach)
        .map_err(|reason| match reason {
            NativeGuardBlock::Graph => Block::Graph,
            NativeGuardBlock::Axis => Block::Runtime("Mobbing search axis"),
            NativeGuardBlock::CandidateState => Block::Runtime("Mobbing candidate state"),
            NativeGuardBlock::Pursuit => Block::Behavior("B2C0 task publication"),
        })?;
    if taken
        && !manager
            .iter_all()
            .find(|entity| entity.id == id)
            .is_some_and(pursuing_graph_authenticates)
    {
        return Err(Block::Behavior("B2C0 task publication"));
    }
    Ok(taken)
}

/// How a Mobbing task's visit ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Next {
    Continue,
    /// A `9C01` singleton: style `+00`, `C690`.
    Reselect,
    /// A `9C00` singleton: style `+04`, `C770` or `C750`.
    Switch(u8),
}

fn tracked(
    manager: &EntityManager,
    target_id: u32,
) -> RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>> {
    RetailRuntimeValue::Known(
        manager
            .iter_all()
            .find(|entity| entity.id == target_id && entity.active)
            .map(|entity| CommonMoverTrackedTargetSnapshot {
                state_flags: entity.collision.state_flags_at_0x08,
                position_raw: entity.position_raw(),
                velocity_raw: entity.velocity_raw(),
            }),
    )
}

/// `03A00`'s post-mover target test: present, not dying, nonzero state.
fn live_target(
    manager: &EntityManager,
    target_id: u32,
) -> Result<Option<[i16; 3]>, Intro2Type10Block> {
    if target_id == WANDER_NEAR_NO_TRACKED_ENTITY {
        return Ok(None);
    }
    let Some(target) = manager
        .iter_all()
        .find(|entity| entity.id == target_id && entity.active)
    else {
        return Ok(None);
    };
    let flags = target.collision.state_flags_at_0x08;
    match flags.masked(DYING_STATE_BIT) {
        RetailRuntimeValue::Known(0) => {}
        RetailRuntimeValue::Known(_) => return Ok(None),
        RetailRuntimeValue::Unresolved => {
            return Err(Intro2Type10Block::Runtime("Mobbing target dying bit"))
        }
    }
    if flags.known_value_bits() != 0 {
        Ok(Some(target.position_raw()))
    } else if flags.known_mask() == u32::MAX {
        Ok(None)
    } else {
        Err(Intro2Type10Block::Runtime("Mobbing target state"))
    }
}

fn wrapped_abs(first: i16, second: i16) -> i32 {
    i32::from(first.wrapping_sub(second)).abs()
}

/// One visit of the Primary `03A00` or `04D50`. Returns whether the root's
/// `C690` must reselect; a `9C00` style switch has already run.
pub(super) fn tick_primary(
    manager: &mut EntityManager,
    id: u32,
    frame: MoverFrame<'_>,
    world_fx: &mut WorldFx,
) -> Result<bool, Intro2Type10Block> {
    use Intro2Type10Block as Block;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    if !pursuing_graph_authenticates(entity) {
        return Err(Block::Graph);
    }
    let dive = matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::MobbingDive(_))
    );
    let next = if dive {
        tick_dive(manager, id, frame, world_fx)?
    } else {
        tick_approach(manager, id, frame, world_fx)?
    };
    match next {
        Next::Continue => Ok(false),
        Next::Reselect => Ok(true),
        Next::Switch(variant) => {
            let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
            switch_style(entity, variant, None, world_fx)?;
            Ok(false)
        }
    }
}

fn begin(entity: &mut Entity) -> Result<(ActorTaskVisit, MobbingTaskState), Intro2Type10Block> {
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(Intro2Type10Block::Graph)?;
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let task = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| match runtime {
            ActorTaskRuntime::MobbingApproach(task) | ActorTaskRuntime::MobbingDive(task) => *task,
            _ => unreachable!("authenticated Mobbing Primary"),
        })
        .ok_or(Intro2Type10Block::Graph)?;
    Ok((visit, task))
}

fn finish(
    entity: &mut Entity,
    visit: ActorTaskVisit,
    private_state: WanderNearPrivateState,
) -> Result<(), Intro2Type10Block> {
    if let Some(ActorTaskRuntime::MobbingApproach(task) | ActorTaskRuntime::MobbingDive(task)) =
        entity.actor_tasks.task_state_mut(visit.task_id)
    {
        task.private_state = private_state;
    }
    if !entity.actor_tasks.finish_exact_visit(visit) {
        return Err(Intro2Type10Block::Graph);
    }
    Ok(())
}

fn tick_approach(
    manager: &mut EntityManager,
    id: u32,
    frame: MoverFrame<'_>,
    world_fx: &mut WorldFx,
) -> Result<Next, Intro2Type10Block> {
    use Intro2Type10Block as Block;
    let target_id = match manager
        .iter_all()
        .find(|entity| entity.id == id)
        .and_then(|entity| entity.actor_task_state(ActorTaskSlot::Primary))
    {
        Some(ActorTaskRuntime::MobbingApproach(task)) => task.target_id(),
        _ => return Err(Block::Graph),
    };
    let tracked = tracked(manager, target_id);
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    let (visit, task) = begin(entity)?;
    let mut private_state = task.private_state;
    // 03A00 calls 01430 before any target read.
    let moved = super::mover::run(entity, frame, &mut private_state, tracked, world_fx);
    finish(entity, visit, private_state)?;
    if !moved? {
        return Ok(Next::Reselect); // 4BE0F8
    }
    let Some(target_position) = live_target(manager, target_id)? else {
        return Ok(Next::Reselect); // 4BE100
    };
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
        return Err(Block::Runtime("Mobbing axis"));
    };
    let position = entity.position_raw();
    if !within_wrapped_axis_range(
        WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
        position,
        target_position,
    ) {
        return Ok(Next::Reselect); // 4BE110
    }
    if wrapped_abs(position[0], target_position[0]) < DIVE_ENTRY_BOX_RAW
        && wrapped_abs(position[2], target_position[2]) < DIVE_ENTRY_BOX_RAW
        && i32::from(target_position[1]) + DIVE_ENTRY_HEIGHT_RAW < i32::from(position[1])
    {
        let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
            return Err(Block::Runtime("Mobbing forward basis"));
        };
        // 1E930 projects the record's route point, not the target body.
        let displacement: [i16; 3] = std::array::from_fn(|axis| {
            private_state.target_position_raw[axis].wrapping_sub(position[axis])
        });
        if crate::hover::dot_q31(basis.forward, displacement) >= 0 {
            return Ok(Next::Switch(2)); // 4BE108
        }
    }
    let profile = type10_profile(entity).ok_or(Block::Allocation)?;
    let RetailRuntimeValue::Known(Some(sub_g)) = &mut entity.sub_g_06070_runtime else {
        return Err(Block::Runtime("Sub-G allocation"));
    };
    sub_g.apply_1b940_target(
        profile.sub_g(),
        i32::from(target_position[1]) + APPROACH_HOVER_RAW,
        || u32::from(world_fx.next_shared_retail_random_u16()),
    );
    Ok(Next::Continue)
}

fn tick_dive(
    manager: &mut EntityManager,
    id: u32,
    frame: MoverFrame<'_>,
    world_fx: &mut WorldFx,
) -> Result<Next, Intro2Type10Block> {
    use Intro2Type10Block as Block;
    let target_id = match manager
        .iter_all()
        .find(|entity| entity.id == id)
        .and_then(|entity| entity.actor_task_state(ActorTaskSlot::Primary))
    {
        Some(ActorTaskRuntime::MobbingDive(task)) => task.target_id(),
        _ => return Err(Block::Graph),
    };
    let tracked = tracked(manager, target_id);
    // 04D50's lookup accepts a dying target; only a missing one ends it.
    let RetailRuntimeValue::Known(Some(target)) = tracked else {
        return Ok(Next::Switch(1)); // 4BE178
    };
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    let (visit, task) = begin(entity)?;
    let position = entity.position_raw();
    let terrain = frame.terrain;
    let height = terrain.bilinear_height_raw(position[0], position[2]);
    let sea = terrain.sea_level_raw();
    let surface = if frame.waves_enabled {
        v2k_formats::terrain::wave_surface_raw(
            position[0],
            position[2],
            frame.retail_tick as i32,
            sea,
            height,
        )
    } else {
        height.max(sea)
    };
    if i32::from(position[1]) - i32::from(frame.active_model_extent_raw) - i32::from(surface)
        < DIVE_SURFACE_CLEARANCE_RAW
        || position[1] < target.position_raw[1]
    {
        finish(entity, visit, task.private_state)?;
        return Ok(Next::Switch(1)); // 4BE180
    }
    let mut private_state = task.private_state;
    // 04D50 ignores the mover's result.
    let moved = super::mover::run(entity, frame, &mut private_state, tracked, world_fx);
    finish(entity, visit, private_state)?;
    moved?;
    let target_position = manager
        .iter_all()
        .find(|entity| entity.id == target_id)
        .ok_or(Block::Runtime("Mobbing dive target"))?
        .position_raw();
    let position = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?
        .position_raw();
    if wrapped_abs(position[0], target_position[0]) <= DIVE_KEEP_BOX_RAW
        && wrapped_abs(position[2], target_position[2]) <= DIVE_KEEP_BOX_RAW
    {
        Ok(Next::Continue)
    } else {
        Ok(Next::Switch(1)) // 4BE188
    }
}

/// The dive style's `+10`/`+14`/`+1C` contact hooks: `C750`, `C6B0(1)`.
pub(crate) fn dive_contact_returns_to_approach(
    entity: &mut Entity,
    world_fx: &mut WorldFx,
) -> Result<(), Intro2Type10Block> {
    if !pursuing_graph_authenticates(entity)
        || context_style(entity) != Some((2, DIVE_STYLE_ADDRESS))
    {
        return Err(Intro2Type10Block::Graph);
    }
    switch_style(entity, 1, None, world_fx)
}

/// Whether the row is Type29 on this owner (the only class21 row).
pub(crate) fn is_type29(entity: &Entity) -> bool {
    type10_profile(entity) == Some(Type10Profile::Type29)
}

#[cfg(test)]
pub(crate) use test_support::*;
#[cfg(test)]
mod test_support {
    use super::*;
    /// Test seam: run `C6B0(variant)` on a living Mobbing context.
    pub(crate) fn switch_style_for_test(
        entity: &mut Entity,
        variant: u8,
        target_id: Option<u32>,
        world_fx: &mut WorldFx,
    ) -> Result<(), Intro2Type10Block> {
        switch_style(entity, variant, target_id, world_fx)
    }
}
