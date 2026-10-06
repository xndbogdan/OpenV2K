//! Type56 sealed dynamic ABCDEH/class7/4/10 binding to the shared living walk.
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
pub struct Type56Profile;
impl NativeGroundActorProfile for Type56Profile {}
impl shared::sealed::Sealed for Type56Profile {
    const ENTITY_TYPE: u32 = 56;
    const CAPTURE_POLICY: shared::NativeCapturePolicy = shared::NativeCapturePolicy::NoCapture;
    const DEFAULT_FLAGS: u32 = 0x39;
    const TOPOLOGY: CommonMoverComponentTopology = super::TOPOLOGY;
    const AXIS: CommonAxisDescriptor = super::AXIS;
    const CHOICES: &'static [BehaviorChoice] = &super::CHOICES;
    const LIVING_CLASSES: &'static [u8] = &[4, 7, 10];
    fn completed_terminal_authenticates(manager: &EntityManager, id: u32) -> bool {
        super::death::finished_terminal_authenticates(manager, id)
    }
    fn register_owner(
        scheduler: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
        owner: shared::NativeGroundActorOwner<Self>,
    ) {
        scheduler.register_type56(owner);
    }
    fn publish_standard_death(
        manager: &mut EntityManager,
        id: u32,
        context: &mut crate::native_ground_actor::NativeGroundDeathContext<'_>,
    ) -> Result<
        LiveActorDeathResult<NativeGroundTerminalPublication>,
        crate::intro2_common_dying::Intro2CommonDyingBlock,
    > {
        super::death::begin_type56_standard_death(manager, id, context.world_fx()).map_err(|_| {
            crate::intro2_common_dying::Intro2CommonDyingBlock::Runtime(
                "Type56 quiet death custody",
            )
        })
    }
    fn publish_direct_surface_death(
        manager: &mut EntityManager,
        id: u32,
        context: &mut crate::native_ground_actor::NativeGroundDeathContext<'_>,
    ) -> Result<
        LiveActorDeathResult<NativeGroundTerminalPublication>,
        crate::intro2_common_dying::Intro2CommonDyingBlock,
    > {
        super::death::begin_type56_standard_death(manager, id, context.world_fx()).map_err(|_| {
            crate::intro2_common_dying::Intro2CommonDyingBlock::Runtime(
                "Type56 quiet surface custody",
            )
        })
    }
    fn graph_authenticates(entity: &Entity) -> bool {
        use crate::{
            actor_task_dispatcher::ActorTaskRuntime as Task,
            actor_task_owner::ActorTaskSlot as Slot,
        };
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return false;
        };
        let primary = entity.actor_task_state(Slot::Primary);
        let secondary = entity.actor_task_state(Slot::Secondary);
        let tertiary = entity.actor_task_state(Slot::Tertiary);
        match context.active_style().style_address() {
            0x4c7e88 => {
                matches!(primary, Some(Task::DefecateVirusWander(_)))
                    && secondary.is_none()
                    && matches!(tertiary, Some(Task::DefecateVirusTerrain(task)) if task.lifetime_ms() == 0)
            }
            0x4c7a50 | 0x4c7618 => {
                matches!(primary, Some(Task::SharedRetarget(_)))
                    && matches!(secondary, Some(Task::TargetAcquisition(_)))
                    && tertiary.is_none()
            }
            0x4c7a98 => shared::search::pursuing_graph_authenticates::<Self>(entity),
            0x4c7660 => {
                matches!((primary, context.target_handle_at_0x08()), (Some(Task::RunAway(task)), RetailRuntimeValue::Known(Some(target))) if task.target_id() == target)
                    && secondary.is_none()
                    && tertiary.is_none()
            }
            _ => false,
        }
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
    fn sub_d_state(entity: &Entity) -> Option<NativeGroundSubDState> {
        entity
            .native_type56_runtime
            .map(|runtime| NativeGroundSubDState {
                origin: NativeGroundAllocationOrigin::Dynamic {
                    allocation: runtime.allocation,
                },
                runtime: runtime.sub_d_runtime,
                owner: runtime.sub_d_owner,
            })
    }
    fn store_sub_d_state(entity: &mut Entity, state: NativeGroundSubDState) {
        let runtime = entity.native_type56_runtime.as_mut().unwrap();
        runtime.sub_d_runtime = state.runtime;
        runtime.sub_d_owner = state.owner;
    }
    fn publish_acquiring(
        entity: &mut Entity,
        metadata: &EntityTypeRuntimeMetadata,
        selection: BehaviorSelection,
        context: BehaviorContextRuntime,
        next_random: &mut impl FnMut() -> u32,
    ) -> bool {
        super::behavior::publish_selection(entity, metadata, selection, context, next_random)
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
            .type_runtime_metadata(56)
            .cloned()
            .ok_or(NativeGroundActorBlock::Metadata)?;
        let outcome = super::aim::tick_type56_aim(mode, manager, fx, id, dt, Some(&metadata))
            .map_err(NativeGroundActorBlock::Aim)?;
        Ok(matches!(
            outcome.resolution.outcome,
            crate::aim_and_fire::AimAndFireFrameOutcome::RequestOwnerTransition { .. }
        ))
    }
    fn update_attachment(
        manager: &mut EntityManager,
        id: u32,
    ) -> Result<(), NativeGroundActorBlock> {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(NativeGroundActorBlock::Allocation)?;
        if entity.sub_j_attachment_runtime == RetailRuntimeValue::Known(None) {
            Ok(())
        } else {
            Err(NativeGroundActorBlock::Runtime("Type56 true absent SubJ"))
        }
    }
}
