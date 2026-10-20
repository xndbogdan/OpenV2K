//! 10C10/DB80/AC60 terminals: Type31's synchronous class18 `C080` split into
//! eight Type105 children, and Type105's quiet class2 `C470`.

use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    entity_behavior::{
        audited_behavior_program, BehaviorDescriptorIdentity, QUIET_DEATH_BEHAVIOR_PROGRAM,
    },
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
        SplitLaunchSource, SplitSource, SplitSourceOwnership,
    },
    world_fx::ExplodeWithRingBurstRequest,
};
use std::convert::Infallible;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type31SplitStatus {
    Issued,
    Completed(SplitAndExplodeCompletion),
    Blocked(SplitAndExplodePhase),
}

/// The linear kernel's committed prefix after its one synchronous visit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type31SplitTerminalState {
    pub context: BehaviorContextRuntime,
    pub progress: SplitAndExplodeProgress,
    pub status: Type31SplitStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type31DeathBlock {
    Allocation,
    Metadata,
    Graph,
    Runtime(&'static str),
    Split(SplitAndExplodeBlock<Type31SplitHostBlock>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type31SplitHostBlock {
    Allocation,
    Metadata,
    Runtime(&'static str),
    Child(Type31Block),
}

/// A completed terminal keeps only its context until 14990 sweeps it.
pub fn finished_terminal_authenticates(manager: &EntityManager, id: u32) -> bool {
    manager_allocation_authenticates(manager, id)
        && manager.pending_actor_deferred_destroy_ids().contains(&id)
        && manager.iter_all().find(|e| e.id == id).is_some_and(|e| {
            let runtime = e.native_type31_runtime.unwrap();
            let terminal = match runtime.row {
                Type31Row::Type31 => runtime.split_terminal.and_then(|t| {
                    let program = audited_behavior_program(18)?;
                    (matches!(t.status, Type31SplitStatus::Completed(_))
                        && t.progress.source_deferred_destroyed
                        && t.context.descriptor() == BehaviorDescriptorIdentity::Named(program)
                        && t.context.active_style().style_address() == 0x004c73d8)
                        .then_some(t.context)
                }),
                Type31Row::Type105 => runtime.quiet_death_context.filter(|context| {
                    context.descriptor()
                        == BehaviorDescriptorIdentity::Named(&QUIET_DEATH_BEHAVIOR_PROGRAM)
                }),
            };
            terminal.is_some_and(|context| {
                e.current_behavior_context == RetailRuntimeValue::Known(Some(context))
            }) && e
                .collision
                .state_flags_at_0x08
                .masked(DYING_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT)
                == RetailRuntimeValue::Known(DYING_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT)
                && ActorTaskSlot::IN_RETAIL_TICK_ORDER
                    .into_iter()
                    .all(|slot| e.actor_tasks.task_in_slot(slot).is_none())
        })
}

/// 10C10's entry shared by both rows: remote and dying returns, completed
/// custody, exact metadata and a living style without a death callback.
fn living_entry(
    manager: &EntityManager,
    id: u32,
    row: Type31Row,
) -> Result<Option<(BehaviorContextRuntime, MainBaseAbortActorLease)>, Type31DeathBlock> {
    use Type31DeathBlock as B;
    if !manager_allocation_authenticates(manager, id) {
        return Err(B::Allocation);
    }
    let e = manager
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(B::Allocation)?;
    if type31_row(e) != Some(row) {
        return Err(B::Allocation);
    }
    let bits = |mask| match e.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(v) => Ok(v),
        _ => Err(B::Runtime("terminal entry state")),
    };
    if bits(REMOTE_OWNED_STATE_BIT)? != 0 {
        return Ok(None);
    }
    if bits(DYING_STATE_BIT)? != 0 {
        return if finished_terminal_authenticates(manager, id) {
            Ok(None)
        } else {
            Err(B::Graph)
        };
    }
    let owner = Type31FamilyOwner::adopt(manager, id).map_err(|_| B::Graph)?;
    if !owner.completed_mutation_boundary(manager) {
        return Err(B::Runtime("completed native task custody"));
    }
    let metadata = manager
        .type_runtime_metadata(row.entity_type())
        .ok_or(B::Metadata)?;
    authenticate_metadata(row, metadata).map_err(|_| B::Metadata)?;
    let runtime = e.native_type31_runtime.unwrap();
    // Both rows' +90 death sound and +8C attachment are null.
    if e.collision.death_sound_id != RetailRuntimeValue::Known(None)
        || e.collision.constructor_sound_attachment_id_at_0x8c != RetailRuntimeValue::Known(None)
        || bits(DEFERRED_DESTROY_PENDING_STATE_BIT)? != 0
        || manager.pending_actor_deferred_destroy_ids().contains(&id)
        || runtime.split_terminal.is_some()
        || runtime.quiet_death_context.is_some()
    {
        return Err(B::Runtime("terminal source fields"));
    }
    let RetailRuntimeValue::Known(Some(context)) = e.current_behavior_context else {
        return Err(B::Graph);
    };
    let BehaviorDescriptorIdentity::Named(program) = context.descriptor() else {
        return Err(B::Graph);
    };
    let style = context.active_style().audited().ok_or(B::Graph)?;
    if !matches!(
        (program.class_id, style.variant),
        (4, 0) | (5, 0) | (10, 0 | 1)
    ) || !row
        .choices()
        .iter()
        .any(|choice| u32::from(choice.behavior_class_id) == u32::from(program.class_id))
        || style.death_callback_address.is_some()
    {
        return Err(B::Graph);
    }
    Ok(Some((context, runtime.allocation)))
}

fn already_returned(
    manager: &EntityManager,
    id: u32,
) -> LiveActorDeathResult<NativeGroundTerminalPublication> {
    // A remote source returns zero; an authentic completed terminal one.
    LiveActorDeathResult {
        returned_nonzero: manager.iter_all().any(|e| {
            e.id == id
                && e.collision
                    .state_flags_at_0x08
                    .masked(REMOTE_OWNED_STATE_BIT)
                    == RetailRuntimeValue::Known(0)
        }),
        publication: None,
    }
}

/// Type105: class2 has no task; C470 clears the slots and marks 14990.
pub fn begin_type105_standard_death(
    manager: &mut EntityManager,
    id: u32,
) -> Result<LiveActorDeathResult<NativeGroundTerminalPublication>, Type31DeathBlock> {
    let Some((context, allocation)) = living_entry(manager, id, Type31Row::Type105)? else {
        return Ok(already_returned(manager, id));
    };
    let terminal = context
        .reselect_audited_type_default(
            &QUIET_DEATH_BEHAVIOR_PROGRAM,
            QUIET_DEATH_BEHAVIOR_PROGRAM.initial_style_table_index_raw,
            QUIET_DEATH_BEHAVIOR_PROGRAM.initial_style,
        )
        .ok_or(Type31DeathBlock::Graph)?;
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    // Alternate rule1/class2 is selected directly after dying; no 25680 draw.
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(terminal));
    for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        entity.actor_tasks.clear_slot(slot);
    }
    entity.mark_actor_deferred_destroy_pending();
    entity
        .native_type31_runtime
        .as_mut()
        .unwrap()
        .quiet_death_context = Some(terminal);
    manager.queue_actor_deferred_destroy(id);
    Ok(LiveActorDeathResult {
        returned_nonzero: true,
        publication: Some(NativeGroundTerminalPublication::Deferred(
            NativeGroundDeferredDeathReceipt::new(allocation, terminal),
        )),
    })
}

pub struct Type31Class18Frame<'a> {
    pub resources: &'a ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub retail_tick: u32,
    pub tasks: &'a mut dyn NativeGroundTaskCustody,
}

/// Type31: AC60 selects class18 once dying (no 25680 draw) and `C080`
/// splits the body into eight Type105s before the source is marked.
pub fn begin_type31_standard_death(
    manager: &mut EntityManager,
    id: u32,
    frame: Type31Class18Frame<'_>,
) -> Result<LiveActorDeathResult<NativeGroundTerminalPublication>, Type31DeathBlock> {
    use Type31DeathBlock as B;
    let Some((context, allocation)) = living_entry(manager, id, Type31Row::Type31)? else {
        return Ok(already_returned(manager, id));
    };
    let actual = frame
        .resources
        .global_entity_type(31)
        .map(EntityTypeRuntimeMetadata::from_section12)
        .ok_or(B::Metadata)?;
    if manager.type_runtime_metadata(31) != Some(&actual) {
        return Err(B::Metadata);
    }
    let program = audited_behavior_program(18).ok_or(B::Metadata)?;
    let terminal = context
        .reselect_audited_type_default(
            program,
            program.initial_style_table_index_raw,
            program.initial_style,
        )
        .ok_or(B::Graph)?;
    let e = manager.entity_mut(id).unwrap();
    e.collision.health_raw = RetailRuntimeValue::Known(0);
    e.collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    e.current_behavior_context = RetailRuntimeValue::Known(Some(terminal));
    e.native_type31_runtime.as_mut().unwrap().split_terminal = Some(Type31SplitTerminalState {
        context: terminal,
        progress: SplitAndExplodeProgress::default(),
        status: Type31SplitStatus::Issued,
    });
    let mut execution = SplitAndExplodeExecution::new(id);
    let result = execution.execute(&mut Type31SplitHost { manager, frame });
    let progress = execution.progress();
    let e = manager.entity_mut(id).ok_or(B::Allocation)?;
    let retained = e
        .native_type31_runtime
        .as_mut()
        .unwrap()
        .split_terminal
        .as_mut()
        .unwrap();
    retained.progress = progress;
    match result {
        Ok(completion) => {
            retained.status = Type31SplitStatus::Completed(completion);
            if !progress.source_deferred_destroyed {
                return Err(B::Runtime(
                    "nonzero 440950 exit has no deferred publication",
                ));
            }
            Ok(LiveActorDeathResult {
                returned_nonzero: true,
                publication: Some(NativeGroundTerminalPublication::Deferred(
                    NativeGroundDeferredDeathReceipt::new(allocation, terminal),
                )),
            })
        }
        Err(error) => {
            retained.status = Type31SplitStatus::Blocked(match &error {
                SplitAndExplodeBlock::Host { phase, .. } => *phase,
                SplitAndExplodeBlock::AlreadyVisited => SplitAndExplodePhase::Burst,
            });
            Err(B::Split(error))
        }
    }
}

struct Type31SplitHost<'a, 'b> {
    manager: &'a mut EntityManager,
    frame: Type31Class18Frame<'b>,
}

impl SplitAndExplodeHost for Type31SplitHost<'_, '_> {
    type Block = Type31SplitHostBlock;
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
            .and_then(|m| self.frame.resources.global_model(m))
            .ok_or(Self::Block::Metadata)?
            .radius;
        // C080's A/B/N/G test picks class 37: Type31 owns A and B.
        self.frame
            .world_fx
            .emit_explode_with_ring_burst_raw(ExplodeWithRingBurstRequest {
                position_raw: e.position_raw(),
                source_extent_raw: extent,
                sea_level_raw: self
                    .frame
                    .resources
                    .level_terrain()
                    .map(|t| t.sea_level_raw()),
                logical_owner_entity_id: id,
                logical_owner_entity_type: 31,
                scatter_count: 2,
                scatter_classes: [37; 2],
                suppresses_impact_damage: flags & 0x8000_0000 != 0,
            });
        // The solo network path returns 0; 440950 delivers no radial.
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
                entity_type: 31,
                requested_entity_handle_raw: 0,
            })),
            RetailRuntimeValue::Known(_) => Ok(SplitSourceOwnership::Remote),
            _ => Err(Self::Block::Runtime("post-clear ownership")),
        }
    }
    fn native_entity_list_count_raw(&mut self) -> Result<i32, Self::Block> {
        // 468D00 counts every allocation, deferred actors and cargo included.
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
        request: crate::split_and_explode::SplitChildRequest,
    ) -> Result<SplitChildAttempt<Infallible>, Self::Block> {
        let birth = self
            .manager
            .construct_native_type105(
                request,
                self.frame.resources,
                self.frame.world_fx,
                self.frame.retail_tick,
            )
            .map_err(Self::Block::Child)?;
        self.frame
            .tasks
            .register_split_type105_child(birth.owner)
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
