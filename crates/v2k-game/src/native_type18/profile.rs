//! Type18's own ABCDEHJ receipt, method20 Aim and capture policy.

use super::*;
use crate::{
    common_mover::component_dispatch::CommonMoverDispatchMode,
    intro2_capture_pursuit::{
        self as capture, Intro2CaptureBlock, Intro2CapturePrimaryFrame, Intro2CaptureProfile,
    },
    intro2_common_dying::Intro2CommonDyingOwner,
    native_ground_actor::{
        NativeGroundActorBlock, NativeGroundActorProfile, NativeGroundSubDState,
    },
    world_fx::WorldFx,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type18Profile;
impl NativeGroundActorProfile for Type18Profile {}

impl shared::sealed::Sealed for Type18Profile {
    const ENTITY_TYPE: u32 = ENTITY_TYPE;
    const CAPTURE_POLICY: shared::NativeCapturePolicy = shared::NativeCapturePolicy::Transport;
    const DEFAULT_FLAGS: u32 = DEFAULT_FLAGS;
    const TOPOLOGY: CommonMoverComponentTopology = TOPOLOGY;
    const AXIS: CommonAxisDescriptor = AXIS;
    const CHOICES: &'static [BehaviorChoice] = &CHOICES;
    const LIVING_CLASSES: &'static [u8] = &[4, 7, 9, 26, 33];
    fn register_owner(
        scheduler: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
        owner: shared::NativeGroundActorOwner<Self>,
    ) {
        scheduler.register_type18(owner);
    }
    fn publish_standard_death(
        manager: &mut EntityManager,
        id: u32,
        context: &mut shared::NativeGroundDeathContext<'_>,
    ) -> Result<
        crate::live_actor_checked_damage::LiveActorDeathResult<
            shared::NativeGroundTerminalPublication,
        >,
        crate::intro2_common_dying::Intro2CommonDyingBlock,
    > {
        let shared::NativeGroundDeathContext::Capture { context, .. } = context else {
            return Err(crate::intro2_common_dying::Intro2CommonDyingBlock::Runtime(
                "Type18 Capture terminal custody",
            ));
        };
        crate::native_actor_capture::publish_native_captor_standard_death(manager, id, context)
            .map(common_terminal)
            .map_err(|error| {
                crate::intro2_common_dying::Intro2CommonDyingBlock::Runtime(error.reason)
            })
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
            .native_type18_runtime
            .map(|runtime| NativeGroundSubDState {
                origin: shared::NativeGroundAllocationOrigin::Authored {
                    spawn_index: runtime.spawn_index,
                },
                runtime: runtime.sub_d_runtime,
                owner: runtime.sub_d_owner,
            })
    }
    fn store_sub_d_state(entity: &mut Entity, state: NativeGroundSubDState) {
        let runtime = entity.native_type18_runtime.as_mut().unwrap();
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
        super::publish_acquiring(entity, metadata, selection, context, next_random)
    }
    fn acquire_capture(
        manager: &mut EntityManager,
        id: u32,
        tick: u32,
        fx: &mut WorldFx,
    ) -> Result<(), NativeGroundActorBlock> {
        capture::acquire(manager, id, tick, fx, Intro2CaptureProfile::Type18)
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
            .type_runtime_metadata(ENTITY_TYPE)
            .cloned()
            .ok_or(NativeGroundActorBlock::Metadata)?;
        let outcome = super::aim::tick_type18_aim(mode, manager, fx, id, dt, Some(&metadata))
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
        crate::native_actor_capture::update_carried_pose(manager, id)
            .map_err(NativeGroundActorBlock::Capture)
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

fn common_terminal(
    owner: Option<Intro2CommonDyingOwner>,
) -> crate::live_actor_checked_damage::LiveActorDeathResult<shared::NativeGroundTerminalPublication>
{
    crate::live_actor_checked_damage::LiveActorDeathResult {
        returned_nonzero: owner.is_some(),
        publication: owner.map(shared::NativeGroundTerminalPublication::CommonDying),
    }
}
