//! Type40's own ABCDEH receipt, method1 Aim and true absent-J capture policy.

use super::*;
use crate::{
    common_mover::component_dispatch::CommonMoverDispatchMode,
    intro2_capture_pursuit::{
        self as capture, Intro2CaptureBlock, Intro2CapturePrimaryFrame, Intro2CaptureProfile,
    },
    native_ground_actor::{
        self as shared, NativeGroundActorBlock, NativeGroundActorProfile, NativeGroundSubDState,
    },
    world_fx::WorldFx,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type40Profile;
impl NativeGroundActorProfile for Type40Profile {}

impl shared::sealed::Sealed for Type40Profile {
    const ENTITY_TYPE: u32 = 40;
    const CAPTURE_POLICY: shared::NativeCapturePolicy = shared::NativeCapturePolicy::AbsentJ;
    const DEFAULT_FLAGS: u32 = 0x39;
    const TOPOLOGY: CommonMoverComponentTopology = super::TOPOLOGY;
    const AXIS: CommonAxisDescriptor = super::AXIS;
    const CHOICES: &'static [BehaviorChoice] = &super::CHOICES;
    const LIVING_CLASSES: &'static [u8] = &[7, 9];
    fn register_owner(
        scheduler: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
        owner: shared::NativeGroundActorOwner<Self>,
    ) {
        scheduler.register_type40(owner);
    }
    fn publish_standard_death(
        manager: &mut EntityManager,
        id: u32,
        context: &mut crate::native_ground_actor::NativeGroundDeathContext<'_>,
    ) -> Result<
        crate::live_actor_checked_damage::LiveActorDeathResult<
            crate::native_ground_actor::NativeGroundTerminalPublication,
        >,
        crate::intro2_common_dying::Intro2CommonDyingBlock,
    > {
        let shared::NativeGroundDeathContext::Split {
            resources,
            fx,
            tick,
            tasks,
        } = context
        else {
            return Err(crate::intro2_common_dying::Intro2CommonDyingBlock::Runtime(
                "Type40 inline split custody",
            ));
        };
        super::death::begin_type40_standard_death(
            manager,
            id,
            super::death::Type40Class18Frame {
                resources,
                world_fx: fx,
                retail_tick: *tick,
                tasks: &mut **tasks,
            },
        )
        .map_err(|_| {
            crate::intro2_common_dying::Intro2CommonDyingBlock::Runtime(
                "Type40 split terminal prefix",
            )
        })
    }
    fn completed_terminal_authenticates(manager: &EntityManager, id: u32) -> bool {
        super::death::finished_terminal_authenticates(manager, id)
    }
    fn graph_authenticates(entity: &Entity) -> bool {
        use crate::{
            actor_task_dispatcher::ActorTaskRuntime as Task,
            actor_task_owner::ActorTaskSlot as Slot,
        };
        let RetailRuntimeValue::Known(Some(c)) = entity.current_behavior_context else {
            return false;
        };
        if entity.sub_j_attachment_runtime != RetailRuntimeValue::Known(None) {
            return false;
        }
        match c.active_style().style_address() {
            0x004c7a50 | 0x004c7ff0 => {
                matches!(
                    entity.actor_task_state(Slot::Primary),
                    Some(Task::SharedRetarget(_))
                ) && matches!(
                    entity.actor_task_state(Slot::Secondary),
                    Some(Task::TargetAcquisition(_))
                ) && entity.actor_task_state(Slot::Tertiary).is_none()
            }
            0x004c7a98 => shared::search::pursuing_graph_authenticates::<Self>(entity),
            0x004c8038 => match (
                entity.actor_task_state(Slot::Primary),
                c.target_handle_at_0x08(),
            ) {
                (Some(Task::CapturePeoplePursuit(t)), RetailRuntimeValue::Known(target)) => t
                    .target_id()
                    == target
                    && t.lifetime()
                        == crate::shared_target_route::SharedTargetRouteLifetime::FixedMilliseconds(
                            5000,
                        )
                    && entity.actor_task_state(Slot::Secondary).is_none()
                    && entity.actor_task_state(Slot::Tertiary).is_none(),
                _ => false,
            },
            _ => false,
        }
    }
    fn allocation_authenticates(entity: &Entity) -> bool {
        allocation_authenticates(entity)
    }
    fn manager_authenticates(manager: &EntityManager, id: u32) -> bool {
        manager_allocation_authenticates(manager, id)
    }
    fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
        authenticate_metadata(metadata).is_ok()
    }
    fn sub_d_state(entity: &Entity) -> Option<NativeGroundSubDState> {
        entity
            .native_type40_runtime
            .map(|runtime| NativeGroundSubDState {
                origin: crate::native_ground_actor::NativeGroundAllocationOrigin::Authored {
                    spawn_index: runtime.spawn_index,
                },
                runtime: runtime.sub_d_runtime,
                owner: runtime.sub_d_owner,
            })
    }
    fn store_sub_d_state(entity: &mut Entity, state: NativeGroundSubDState) {
        let runtime = entity.native_type40_runtime.as_mut().unwrap();
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
        super::behavior::publish_acquiring(entity, metadata, selection, context, next_random)
    }
    fn acquire_capture(
        manager: &mut EntityManager,
        id: u32,
        tick: u32,
        fx: &mut WorldFx,
    ) -> Result<(), NativeGroundActorBlock> {
        capture::acquire(manager, id, tick, fx, Intro2CaptureProfile::Type40)
            .map_err(|error| map_capture(error, |never| match never {}))
    }
    fn tick_capture_primary(
        manager: &mut EntityManager,
        id: u32,
        frame: shared::mover::MoverFrame<'_>,
        fx: &mut WorldFx,
    ) -> Result<bool, NativeGroundActorBlock> {
        capture::tick_primary(
            manager,
            Intro2CapturePrimaryFrame {
                entity_id: id,
                elapsed_micros: frame.elapsed_micros,
                dispatch_mode: frame.dispatch_mode,
            },
            |entity, target, tracked| {
                shared::mover::run::<Self>(entity, frame, target, tracked, &mut || {
                    u32::from(fx.next_shared_retail_random_u16())
                })
            },
        )
        .map(|transition| transition.is_some())
        .map_err(|error| map_capture(error, |error| error))
    }
    fn tick_aim(
        manager: &mut EntityManager,
        id: u32,
        dt: u32,
        mode: CommonMoverDispatchMode,
        fx: &mut WorldFx,
    ) -> Result<bool, NativeGroundActorBlock> {
        let metadata = manager
            .type_runtime_metadata(40)
            .cloned()
            .ok_or(NativeGroundActorBlock::Metadata)?;
        let outcome = super::aim::tick_type40_aim(mode, manager, fx, id, dt, Some(&metadata))
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
            .find(|e| e.id == id)
            .ok_or(NativeGroundActorBlock::Allocation)?;
        if entity.sub_j_attachment_runtime == RetailRuntimeValue::Known(None) {
            Ok(())
        } else {
            Err(NativeGroundActorBlock::Runtime("Type40 true absent SubJ"))
        }
    }
}

fn map_capture<E>(
    error: Intro2CaptureBlock<E>,
    mover: impl FnOnce(E) -> NativeGroundActorBlock,
) -> NativeGroundActorBlock {
    match error {
        Intro2CaptureBlock::Allocation => NativeGroundActorBlock::Allocation,
        Intro2CaptureBlock::Graph => NativeGroundActorBlock::Graph,
        Intro2CaptureBlock::Metadata => NativeGroundActorBlock::Metadata,
        Intro2CaptureBlock::Runtime(reason) => NativeGroundActorBlock::Runtime(reason),
        Intro2CaptureBlock::Acquisition(reason) => NativeGroundActorBlock::Acquisition(reason),
        Intro2CaptureBlock::Mover(reason) => mover(reason),
    }
}
