//! Type27's `10C10 -> DB80 -> AC60 -> 40C080` class18 split into two Type3
//! boulders. Type3 instead dies through the shared class1 terminal in
//! [`crate::class49_death`].
//!
//! `10C10` zeroes health and sets the dying bit; both class20 styles have a
//! null `+2C`, so DB80 reaches AC60, which selects alternate class18 without a
//! `425680` draw. C080's `440950` burst tests A/B/N/G before choosing its
//! class byte: a boulder has no components, so both bytes are `0x10` (16), with
//! the fixed count 2 (`40C0B3..40C0F5`). The Type27->Type3/count2 row of its
//! type switch then builds each child through [`crate::split_and_explode`].

use super::*;
use crate::{
    entity_behavior::BehaviorDescriptorIdentity,
    entity_collision_state::{
        active_model_slot_from_state_flags, DEFERRED_DESTROY_PENDING_STATE_BIT, DYING_STATE_BIT,
        REMOTE_OWNED_STATE_BIT,
    },
    live_actor_checked_damage::LiveActorDeathResult,
    native_ground_actor::{
        NativeGroundDeferredDeathReceipt, NativeGroundTaskCustody, NativeGroundTerminalPublication,
    },
    split_and_explode::{
        SplitAndExplodeBlock, SplitAndExplodeCompletion, SplitAndExplodeExecution,
        SplitAndExplodeHost, SplitAndExplodePhase, SplitAndExplodeProgress, SplitChildAttempt,
        SplitChildRequest, SplitLaunchSource, SplitSource, SplitSourceOwnership,
    },
    world_fx::ExplodeWithRingBurstRequest,
};
use std::convert::Infallible;

/// Class18 style `0x004C73D8`; its `+04..+38` words are all zero.
pub const SPLIT_STYLE_ADDRESS: u32 = 0x004C_73D8;
/// `440950` scatter class for a body without A/B/N/G components.
pub const SPLIT_BURST_CLASS: u8 = 0x10;
pub const SPLIT_BURST_COUNT: i32 = 2;

pub struct RollingBoulderSplitFrame<'a> {
    pub resources: &'a ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub retail_tick: u32,
    pub tasks: &'a mut dyn NativeGroundTaskCustody,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollingBoulderSplitStatus {
    Issued,
    Completed(SplitAndExplodeCompletion),
    Blocked(SplitAndExplodePhase),
}

/// The linear kernel's committed prefix after its one synchronous visit. It
/// never grants another constructor visit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RollingBoulderSplitTerminal {
    pub context: BehaviorContextRuntime,
    pub progress: SplitAndExplodeProgress,
    pub status: RollingBoulderSplitStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollingBoulderSplitHostBlock {
    Allocation,
    Metadata,
    Runtime(&'static str),
    Child(RollingBoulderBlock),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollingBoulderDeathBlock {
    Allocation,
    Metadata,
    Graph,
    Runtime(&'static str),
    Split(SplitAndExplodeBlock<RollingBoulderSplitHostBlock>),
}

/// A completed split leaves the deferred, task-less parent linked until the
/// next sweep. Its class18 style has null hooks and its health buffer remains.
pub fn finished_split_authenticates(manager: &EntityManager, id: u32) -> bool {
    rolling_boulder_manager_allocation_authenticates(manager, id)
        && manager.pending_actor_deferred_destroy_ids().contains(&id)
        && manager.iter_all().find(|e| e.id == id).is_some_and(|e| {
            let Some(terminal) = e.rolling_boulder_runtime.and_then(|r| r.split_terminal) else {
                return false;
            };
            let Some(program) = audited_behavior_program(18) else {
                return false;
            };
            matches!(terminal.status, RollingBoulderSplitStatus::Completed(_))
                && terminal.progress.source_deferred_destroyed
                && e.current_behavior_context == RetailRuntimeValue::Known(Some(terminal.context))
                && terminal.context.descriptor() == BehaviorDescriptorIdentity::Named(program)
                && terminal.context.active_style().style_address() == SPLIT_STYLE_ADDRESS
                && e.collision
                    .state_flags_at_0x08
                    .masked(DYING_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT)
                    == RetailRuntimeValue::Known(
                        DYING_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT,
                    )
                && ActorTaskSlot::IN_RETAIL_TICK_ORDER
                    .into_iter()
                    .all(|slot| e.actor_tasks.task_in_slot(slot).is_none())
        })
}

/// Type27 `10C10` followed by the synchronous class18 initializer.
pub fn begin_rolling_boulder_split(
    manager: &mut EntityManager,
    id: u32,
    frame: RollingBoulderSplitFrame<'_>,
) -> Result<LiveActorDeathResult<NativeGroundTerminalPublication>, RollingBoulderDeathBlock> {
    use RollingBoulderDeathBlock as B;
    if !rolling_boulder_manager_allocation_authenticates(manager, id) {
        return Err(B::Allocation);
    }
    let e = manager
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(B::Allocation)?;
    if e.rolling_boulder_runtime.unwrap().profile != RollingBoulderProfile::Large {
        return Err(B::Metadata);
    }
    let bits = |mask| match e.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(v) => Ok(v),
        _ => Err(B::Runtime("terminal entry state")),
    };
    if bits(REMOTE_OWNED_STATE_BIT)? != 0 {
        return Ok(LiveActorDeathResult {
            returned_nonzero: false,
            publication: None,
        });
    }
    if bits(DYING_STATE_BIT)? != 0 {
        if !finished_split_authenticates(manager, id) {
            return Err(B::Graph);
        }
        return Ok(LiveActorDeathResult {
            returned_nonzero: true,
            publication: None,
        });
    }
    let owner = RollingBoulderOwner::adopt(manager, id).map_err(|_| B::Graph)?;
    if !owner.completed_mutation_boundary(manager) {
        return Err(B::Runtime("completed native task custody"));
    }
    let metadata = manager.type_runtime_metadata(27).ok_or(B::Metadata)?;
    authenticate_metadata(RollingBoulderProfile::Large, metadata).map_err(|_| B::Metadata)?;
    let actual = frame
        .resources
        .global_entity_type(27)
        .map(EntityTypeRuntimeMetadata::from_section12)
        .ok_or(B::Metadata)?;
    if *metadata != actual {
        return Err(B::Metadata);
    }
    if e.collision.death_sound_id != RetailRuntimeValue::Known(None)
        || e.collision.constructor_sound_attachment_id_at_0x8c != RetailRuntimeValue::Known(None)
        || bits(DEFERRED_DESTROY_PENDING_STATE_BIT)? != 0
        || manager.pending_actor_deferred_destroy_ids().contains(&id)
        || e.rolling_boulder_runtime.unwrap().split_terminal.is_some()
    {
        return Err(B::Runtime("terminal source fields"));
    }
    // Both class20 styles carry a null +2C, so DB80 has no style callback.
    let style = current_style(e).map_err(|_| B::Graph)?;
    if style
        .audited_style()
        .is_none_or(|style| style.death_callback_address.is_some())
    {
        return Err(B::Graph);
    }
    let RetailRuntimeValue::Known(Some(context)) = e.current_behavior_context else {
        return Err(B::Graph);
    };
    let program = audited_behavior_program(18).ok_or(B::Metadata)?;
    let terminal = context
        .reselect_audited_type_default(
            program,
            program.initial_style_table_index_raw,
            program.initial_style,
        )
        .ok_or(B::Graph)?;
    let allocation = e.rolling_boulder_runtime.unwrap().allocation;
    let e = manager.entity_mut(id).unwrap();
    e.collision.health_raw = RetailRuntimeValue::Known(0);
    e.collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    // Type27 header+90 and +8C are null; the class18 words +34/+38 are zero.
    e.current_behavior_context = RetailRuntimeValue::Known(Some(terminal));
    e.rolling_boulder_runtime.as_mut().unwrap().split_terminal =
        Some(RollingBoulderSplitTerminal {
            context: terminal,
            progress: SplitAndExplodeProgress::default(),
            status: RollingBoulderSplitStatus::Issued,
        });
    let mut execution = SplitAndExplodeExecution::new(id);
    let result = execution.execute(&mut RollingBoulderSplitHost { manager, frame });
    let progress = execution.progress();
    let e = manager.entity_mut(id).ok_or(B::Allocation)?;
    let retained = e
        .rolling_boulder_runtime
        .as_mut()
        .unwrap()
        .split_terminal
        .as_mut()
        .unwrap();
    retained.progress = progress;
    match result {
        Ok(completion) => {
            retained.status = RollingBoulderSplitStatus::Completed(completion);
            if !progress.source_deferred_destroyed {
                return Err(B::Runtime("nonzero440950 exit has no deferred publication"));
            }
            Ok(LiveActorDeathResult {
                returned_nonzero: true,
                publication: Some(NativeGroundTerminalPublication::Deferred(
                    NativeGroundDeferredDeathReceipt::new(allocation, terminal),
                )),
            })
        }
        Err(error) => {
            retained.status = RollingBoulderSplitStatus::Blocked(match &error {
                SplitAndExplodeBlock::Host { phase, .. } => *phase,
                SplitAndExplodeBlock::AlreadyVisited => SplitAndExplodePhase::Burst,
            });
            Err(B::Split(error))
        }
    }
}

struct RollingBoulderSplitHost<'a, 'b> {
    manager: &'a mut EntityManager,
    frame: RollingBoulderSplitFrame<'b>,
}

impl SplitAndExplodeHost for RollingBoulderSplitHost<'_, '_> {
    type Block = RollingBoulderSplitHostBlock;
    type ConstructorError = Infallible;

    fn emit_split_burst(&mut self, id: u32) -> Result<u32, Self::Block> {
        let e = self
            .manager
            .iter_all()
            .find(|e| e.id == id)
            .ok_or(Self::Block::Allocation)?;
        let RetailRuntimeValue::Known(flags) = e.collision.state_flags_at_0x08.masked(u32::MAX)
        else {
            return Err(Self::Block::Runtime("burst state"));
        };
        let extent = e.model_slots[active_model_slot_from_state_flags(flags)]
            .and_then(|model| self.frame.resources.global_model(model))
            .ok_or(Self::Block::Metadata)?
            .radius;
        self.frame
            .world_fx
            .emit_explode_with_ring_burst_raw(ExplodeWithRingBurstRequest {
                position_raw: e.position_raw(),
                source_extent_raw: extent,
                sea_level_raw: self
                    .frame
                    .resources
                    .level_terrain()
                    .map(|terrain| terrain.sea_level_raw()),
                logical_owner_entity_id: id,
                logical_owner_entity_type: 27,
                scatter_count: SPLIT_BURST_COUNT,
                scatter_classes: [SPLIT_BURST_CLASS; 2],
                suppresses_impact_damage: flags & 0x8000_0000 != 0,
            });
        // The solo network path returns zero; 440950 delivers no radial.
        Ok(0)
    }

    fn clear_source_tasks(&mut self, id: u32) -> Result<(), Self::Block> {
        let e = self.manager.entity_mut(id).ok_or(Self::Block::Allocation)?;
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            e.actor_tasks.clear_slot(slot);
        }
        Ok(())
    }

    fn source_after_task_clear(&mut self, id: u32) -> Result<SplitSourceOwnership, Self::Block> {
        let e = self
            .manager
            .iter_all()
            .find(|e| e.id == id)
            .ok_or(Self::Block::Allocation)?;
        match e
            .collision
            .state_flags_at_0x08
            .masked(REMOTE_OWNED_STATE_BIT)
        {
            RetailRuntimeValue::Known(0) => Ok(SplitSourceOwnership::Local(SplitSource {
                entity_type: 27,
                requested_entity_handle_raw: 0,
            })),
            RetailRuntimeValue::Known(_) => Ok(SplitSourceOwnership::Remote),
            _ => Err(Self::Block::Runtime("post-clear ownership")),
        }
    }

    fn native_entity_list_count_raw(&mut self) -> Result<i32, Self::Block> {
        // 468D00 skips the first actor and counts the sentinel: the full
        // allocation count, including deferred actors and cargo.
        Ok(self.manager.iter_all().count() as u32 as i32)
    }

    fn current_launch_source(&mut self, id: u32) -> Result<SplitLaunchSource, Self::Block> {
        let e = self
            .manager
            .iter_all()
            .find(|e| e.id == id)
            .ok_or(Self::Block::Allocation)?;
        let objective = match e.collision.state_flags_at_0x08.masked(0x0100_0000) {
            RetailRuntimeValue::Known(v) => v != 0,
            _ => return Err(Self::Block::Runtime("birth objective")),
        };
        Ok(SplitLaunchSource {
            position_raw: e.position_raw(),
            heading_raw: e.rotation_heading_pitch_roll_raw()[0] as u16,
            objective,
        })
    }

    fn next_shared_random_u16(&mut self) -> u16 {
        self.frame.world_fx.next_shared_retail_random_u16()
    }

    fn construct_split_child(
        &mut self,
        request: SplitChildRequest,
    ) -> Result<SplitChildAttempt<Infallible>, Self::Block> {
        let owner = self
            .manager
            .construct_native_rolling_boulder(
                request,
                self.frame.resources,
                self.frame.world_fx,
                self.frame.retail_tick,
            )
            .map_err(Self::Block::Child)?;
        self.frame
            .tasks
            .register_split_rolling_boulder_child(owner)
            .map_err(Self::Block::Runtime)?;
        Ok(SplitChildAttempt::Created)
    }

    fn dispose_constructor_error(&mut self, error: Infallible) -> Result<(), Self::Block> {
        match error {}
    }

    fn mark_source_deferred_destroy(&mut self, id: u32) -> Result<(), Self::Block> {
        self.manager
            .entity_mut(id)
            .ok_or(Self::Block::Allocation)?
            .mark_actor_deferred_destroy_pending();
        self.manager.queue_actor_deferred_destroy(id);
        Ok(())
    }
}
