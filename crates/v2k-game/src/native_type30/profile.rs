//! Sealed Type30 receipt over the shared ground task/mover/checked-hit host.

use super::*;
use crate::{
    common_mover::component_dispatch::CommonMoverDispatchMode,
    live_actor_checked_damage::LiveActorDeathResult,
    native_ground_actor::{
        NativeGroundActorBlock, NativeGroundActorProfile, NativeGroundAllocationOrigin,
        NativeGroundSubDState, NativeGroundTerminalPublication,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type30Profile;
impl NativeGroundActorProfile for Type30Profile {}
impl shared::sealed::Sealed for Type30Profile {
    const ENTITY_TYPE: u32 = 30;
    const CAPTURE_POLICY: shared::NativeCapturePolicy = shared::NativeCapturePolicy::NoCapture;
    const DEFAULT_FLAGS: u32 = 0x439;
    const TOPOLOGY: crate::entity_collision_state::CommonMoverComponentTopology = super::TOPOLOGY;
    const AXIS: CommonAxisDescriptor = super::AXIS;
    const CHOICES: &'static [BehaviorChoice] = &super::CHOICES;
    const LIVING_CLASSES: &'static [u8] = &[5, 7, 26];
    fn register_owner(
        scheduler: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
        owner: Type30Owner,
    ) {
        scheduler.register_type30(owner);
    }
    fn allocation_authenticates(entity: &Entity) -> bool {
        super::allocation_authenticates(entity)
    }
    fn manager_authenticates(manager: &EntityManager, id: u32) -> bool {
        super::manager_allocation_authenticates(manager, id)
    }
    fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
        super::authenticate_metadata(metadata).is_ok()
    }
    fn graph_authenticates(entity: &Entity) -> bool {
        use crate::{
            actor_task_dispatcher::ActorTaskRuntime as Task,
            actor_task_owner::ActorTaskSlot as Slot,
        };
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return false;
        };
        let p = entity.actor_task_state(Slot::Primary);
        let s = entity.actor_task_state(Slot::Secondary);
        let t = entity.actor_task_state(Slot::Tertiary);
        match context.active_style().style_address() {
            0x4c7930 => {
                matches!(p, Some(Task::SharedRetarget(task)) if task.lifetime_ms() == 5000)
                    && s.is_none()
                    && t.is_none()
            }
            0x4c7a50 => {
                matches!(p, Some(Task::SharedRetarget(_)))
                    && matches!(s, Some(Task::TargetAcquisition(_)))
                    && t.is_none()
            }
            0x4c7a98 => shared::search::pursuing_graph_authenticates::<Self>(entity),
            0x4c7738 => {
                matches!(p, Some(Task::TrashFurniture(task)) if task.kind_filter() == -1)
                    && s.is_none()
                    && t.is_none()
            }
            _ => false,
        }
    }
    fn sub_d_state(entity: &Entity) -> Option<NativeGroundSubDState> {
        entity
            .native_type30_runtime
            .as_ref()
            .map(|r| NativeGroundSubDState {
                origin: NativeGroundAllocationOrigin::Authored {
                    spawn_index: r.spawn_index,
                },
                runtime: r.sub_d_runtime,
                owner: r.sub_d_owner,
            })
    }
    fn store_sub_d_state(entity: &mut Entity, state: NativeGroundSubDState) {
        let r = entity.native_type30_runtime.as_mut().unwrap();
        r.sub_d_runtime = state.runtime;
        r.sub_d_owner = state.owner;
    }
    fn kl_components(entity: &Entity) -> Option<Intro2KlComponents> {
        entity
            .native_type30_runtime
            .as_ref()
            .map(|r| r.kl_components.clone())
    }
    fn store_kl_components(entity: &mut Entity, components: Intro2KlComponents) {
        entity.native_type30_runtime.as_mut().unwrap().kl_components = components;
    }
    fn publish_acquiring(
        entity: &mut Entity,
        metadata: &EntityTypeRuntimeMetadata,
        selection: BehaviorSelection,
        context: BehaviorContextRuntime,
        rng: &mut impl FnMut() -> u32,
    ) -> bool {
        super::tasks::publish_acquiring(entity, metadata, selection, context, rng)
    }
    fn publish_standard_death(
        manager: &mut EntityManager,
        id: u32,
        context: &mut shared::NativeGroundDeathContext<'_>,
    ) -> Result<
        LiveActorDeathResult<NativeGroundTerminalPublication>,
        crate::intro2_common_dying::Intro2CommonDyingBlock,
    > {
        crate::intro2_common_dying::publish_intro2_common_standard_death(
            manager,
            id,
            context.world_fx(),
        )
        .map(terminal)
    }
    fn publish_direct_surface_death(
        manager: &mut EntityManager,
        id: u32,
        context: &mut shared::NativeGroundDeathContext<'_>,
    ) -> Result<
        LiveActorDeathResult<NativeGroundTerminalPublication>,
        crate::intro2_common_dying::Intro2CommonDyingBlock,
    > {
        crate::intro2_common_dying::publish_intro2_common_standard_death(
            manager,
            id,
            context.world_fx(),
        )
        .map(terminal)
    }
    fn acquire_capture(
        _: &mut EntityManager,
        _: u32,
        _: u32,
        _: &mut WorldFx,
    ) -> Result<(), NativeGroundActorBlock> {
        Err(NativeGroundActorBlock::Graph)
    }
    fn tick_capture_primary(
        _: &mut EntityManager,
        _: u32,
        _: shared::mover::MoverFrame<'_>,
        _: &mut WorldFx,
    ) -> Result<bool, NativeGroundActorBlock> {
        Err(NativeGroundActorBlock::Graph)
    }
    fn tick_aim(
        manager: &mut EntityManager,
        id: u32,
        dt: u32,
        mode: CommonMoverDispatchMode,
        fx: &mut WorldFx,
    ) -> Result<bool, NativeGroundActorBlock> {
        let metadata = manager
            .type_runtime_metadata(30)
            .cloned()
            .ok_or(NativeGroundActorBlock::Metadata)?;
        super::aim::tick_type30_aim(mode, manager, fx, id, dt, Some(&metadata))
            .map(|result| {
                matches!(
                    result.resolution.outcome,
                    crate::aim_and_fire::AimAndFireFrameOutcome::RequestOwnerTransition { .. }
                )
            })
            .map_err(NativeGroundActorBlock::Aim)
    }
    fn update_attachment(
        manager: &mut EntityManager,
        id: u32,
    ) -> Result<(), NativeGroundActorBlock> {
        let entity = manager
            .iter_all()
            .find(|e| e.id == id)
            .ok_or(NativeGroundActorBlock::Allocation)?;
        if entity.sub_j_attachment_runtime == RetailRuntimeValue::Known(None) {
            Ok(())
        } else {
            Err(NativeGroundActorBlock::Runtime("Type30 absent Sub-J"))
        }
    }
}
fn terminal(
    owner: Option<crate::intro2_common_dying::Intro2CommonDyingOwner>,
) -> LiveActorDeathResult<NativeGroundTerminalPublication> {
    LiveActorDeathResult {
        returned_nonzero: owner.is_some(),
        publication: owner.map(NativeGroundTerminalPublication::CommonDying),
    }
}
