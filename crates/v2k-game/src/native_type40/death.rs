//! Type40's10C10/DB80/AC60 and synchronous C080 split; source-inline births.
use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    entity_behavior::{audited_behavior_program, BehaviorDescriptorIdentity},
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

/// Source solo dispatcher retains the global default/null handle value.
/// A nonzero handle override is deliberately rejected by the Type56 allocator.
pub const SPLIT_DEFAULT_REQUESTED_ENTITY_HANDLE_RAW: u32 = 0;

pub struct Type40Class18Frame<'a> {
    pub resources: &'a ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub retail_tick: u32,
    pub tasks: &'a mut dyn NativeGroundTaskCustody,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type40SplitStatus {
    Issued,
    Completed(SplitAndExplodeCompletion),
    Blocked(SplitAndExplodePhase),
}
/// Retains the linear kernel's committed prefix after its one synchronous visit.
/// It never contains an executable continuation or grants a fresh constructor visit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type40SplitTerminalState {
    pub context: BehaviorContextRuntime,
    pub progress: SplitAndExplodeProgress,
    pub status: Type40SplitStatus,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type40SplitHostBlock {
    Allocation,
    Metadata,
    Runtime(&'static str),
    Child(crate::native_type56::Type56Block),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type40DeathBlock {
    Allocation,
    Metadata,
    Graph,
    Runtime(&'static str),
    Split(SplitAndExplodeBlock<Type40SplitHostBlock>),
}

pub fn finished_terminal_authenticates(manager: &EntityManager, id: u32) -> bool {
    manager_allocation_authenticates(manager, id)
        && manager.pending_actor_deferred_destroy_ids().contains(&id)
        && manager.iter_all().find(|e| e.id == id).is_some_and(|e| {
            let Some(t) = e.native_type40_runtime.and_then(|r| r.split_terminal) else {
                return false;
            };
            let Some(p) = audited_behavior_program(18) else {
                return false;
            };
            matches!(t.status, Type40SplitStatus::Completed(_))
                && t.progress.source_deferred_destroyed
                && e.current_behavior_context == RetailRuntimeValue::Known(Some(t.context))
                && t.context.descriptor() == BehaviorDescriptorIdentity::Named(p)
                && t.context.active_style().style_address() == 0x004c73d8
                && e.collision
                    .state_flags_at_0x08
                    .masked(DYING_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT)
                    == RetailRuntimeValue::Known(
                        DYING_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT,
                    )
                && ActorTaskSlot::IN_RETAIL_TICK_ORDER
                    .into_iter()
                    .all(|s| e.actor_tasks.task_in_slot(s).is_none())
        })
}

pub fn begin_type40_standard_death(
    manager: &mut EntityManager,
    id: u32,
    frame: Type40Class18Frame<'_>,
) -> Result<LiveActorDeathResult<NativeGroundTerminalPublication>, Type40DeathBlock> {
    use Type40DeathBlock as B;
    if !manager_allocation_authenticates(manager, id) {
        return Err(B::Allocation);
    }
    let e = manager
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(B::Allocation)?;
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
        if !finished_terminal_authenticates(manager, id) {
            return Err(B::Graph);
        }
        return Ok(LiveActorDeathResult {
            returned_nonzero: true,
            publication: None,
        });
    }
    let owner = Type40Owner::adopt(manager, id).map_err(|_| B::Graph)?;
    if !owner.completed_mutation_boundary(manager) {
        return Err(B::Runtime("completed native task custody"));
    }
    let metadata = manager.type_runtime_metadata(40).ok_or(B::Metadata)?;
    authenticate_metadata(metadata).map_err(|_| B::Metadata)?;
    let actual = frame
        .resources
        .global_entity_type(40)
        .map(EntityTypeRuntimeMetadata::from_section12)
        .ok_or(B::Metadata)?;
    if *metadata != actual {
        return Err(B::Metadata);
    }
    if e.collision.death_sound_id != RetailRuntimeValue::Known(None)
        || e.collision.constructor_sound_attachment_id_at_0x8c != RetailRuntimeValue::Known(None)
        || bits(DEFERRED_DESTROY_PENDING_STATE_BIT)? != 0
        || manager.pending_actor_deferred_destroy_ids().contains(&id)
        || e.native_type40_runtime.unwrap().split_terminal.is_some()
    {
        return Err(B::Runtime("terminal source fields"));
    }
    let RetailRuntimeValue::Known(Some(c)) = e.current_behavior_context else {
        return Err(B::Graph);
    };
    let BehaviorDescriptorIdentity::Named(p) = c.descriptor() else {
        return Err(B::Graph);
    };
    let s = c.active_style().audited().ok_or(B::Graph)?;
    if !matches!((p.class_id, s.variant), (7 | 9, 0 | 1)) || s.death_callback_address.is_some() {
        return Err(B::Graph);
    }
    let program = audited_behavior_program(18).ok_or(B::Metadata)?;
    let terminal = c
        .reselect_audited_type_default(
            program,
            program.initial_style_table_index_raw,
            program.initial_style,
        )
        .ok_or(B::Graph)?;
    let allocation = e.native_type40_runtime.unwrap().allocation;
    let e = manager.entity_mut(id).unwrap();
    e.collision.health_raw = RetailRuntimeValue::Known(0);
    e.collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    // Type40 header+90 and+8C are null. AC60 chooses rule1/class18 directly
    // once dying, so this alternate publication consumes no425680 RNG word.
    e.current_behavior_context = RetailRuntimeValue::Known(Some(terminal));
    e.native_type40_runtime.as_mut().unwrap().split_terminal = Some(Type40SplitTerminalState {
        context: terminal,
        progress: SplitAndExplodeProgress::default(),
        status: Type40SplitStatus::Issued,
    });
    let mut execution = SplitAndExplodeExecution::new(id);
    let result = execution.execute(&mut Type40SplitHost { manager, frame });
    let progress = execution.progress();
    let e = manager.entity_mut(id).ok_or(B::Allocation)?;
    let retained = e
        .native_type40_runtime
        .as_mut()
        .unwrap()
        .split_terminal
        .as_mut()
        .unwrap();
    retained.progress = progress;
    match result {
        Ok(completion) => {
            retained.status = Type40SplitStatus::Completed(completion);
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
            retained.status = Type40SplitStatus::Blocked(match &error {
                SplitAndExplodeBlock::Host { phase, .. } => *phase,
                SplitAndExplodeBlock::AlreadyVisited => SplitAndExplodePhase::Burst,
            });
            Err(B::Split(error))
        }
    }
}

struct Type40SplitHost<'a, 'b> {
    manager: &'a mut EntityManager,
    frame: Type40Class18Frame<'b>,
}
impl SplitAndExplodeHost for Type40SplitHost<'_, '_> {
    type Block = Type40SplitHostBlock;
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
        // C080 A/B/N/G test chooses37 because this real type owns A and B;
        // source WORDburstcount2 and both class bytes37 are distinct from49.
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
                logical_owner_entity_type: 40,
                scatter_count: 2,
                scatter_classes: [37; 2],
                suppresses_impact_damage: flags & 0x80000000 != 0,
            });
        // The source solo network path returns0. No radial delivery occurs in440950.
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
                entity_type: 40,
                requested_entity_handle_raw: SPLIT_DEFAULT_REQUESTED_ENTITY_HANDLE_RAW,
            })),
            RetailRuntimeValue::Known(_) => Ok(SplitSourceOwnership::Remote),
            _ => Err(Self::Block::Runtime("post-clear ownership")),
        }
    }
    fn native_entity_list_count_raw(&mut self) -> Result<i32, Self::Block> {
        //468D00 skips first actor/counts sentinel: full allocation count,
        //including deferred actors and cargo, numerically without an extra1.
        Ok(self.manager.iter_all().count() as u32 as i32)
    }
    fn current_launch_source(&mut self, id: u32) -> Result<SplitLaunchSource, Self::Block> {
        let e = self
            .manager
            .iter_all()
            .find(|e| e.id == id)
            .ok_or(Self::Block::Allocation)?;
        let objective = match e.collision.state_flags_at_0x08.masked(0x01000000) {
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
        let birth = self
            .manager
            .construct_native_type56(
                request,
                self.frame.resources,
                self.frame.world_fx,
                self.frame.retail_tick,
            )
            .map_err(Self::Block::Child)?;
        self.frame
            .tasks
            .register_split_type56_child(birth.owner)
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
