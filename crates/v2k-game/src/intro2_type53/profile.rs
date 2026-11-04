//! Type53's own receipt and absent-emitter policy for the shared task kernel.

use super::*;
use crate::{
    common_mover::component_dispatch::CommonMoverDispatchMode,
    intro2_common_dying::Intro2CommonDyingOwner,
    native_ground_actor::{
        NativeGroundActorBlock, NativeGroundActorProfile, NativeGroundSubDState,
    },
    world_fx::WorldFx,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type53Profile;
impl NativeGroundActorProfile for Type53Profile {}

impl crate::native_ground_actor::sealed::Sealed for Type53Profile {
    const ENTITY_TYPE: u32 = 53;
    const CAPTURE_POLICY: crate::native_ground_actor::NativeCapturePolicy =
        crate::native_ground_actor::NativeCapturePolicy::Transport;
    const DEFAULT_FLAGS: u32 = 0x39;
    const TOPOLOGY: CommonMoverComponentTopology = super::TOPOLOGY;
    const AXIS: CommonAxisDescriptor = super::AXIS;
    const CHOICES: &'static [BehaviorChoice] = &super::CHOICES;
    const LIVING_CLASSES: &'static [u8] = &[7, 9, 33];

    fn register_owner(
        scheduler: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
        owner: crate::native_ground_actor::NativeGroundActorOwner<Self>,
    ) {
        scheduler.register_intro2_type53(owner);
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
        // DB80: a carrying style's D040 releases the child before class12.
        let crate::native_ground_actor::NativeGroundDeathContext::Capture { context, .. } = context
        else {
            return Err(crate::intro2_common_dying::Intro2CommonDyingBlock::Runtime(
                "Type53 Capture terminal custody",
            ));
        };
        crate::native_actor_capture::publish_native_captor_standard_death(manager, id, context)
            .map(common_terminal)
            .map_err(|error| {
                crate::intro2_common_dying::Intro2CommonDyingBlock::Runtime(error.reason)
            })
    }
    fn allocation_authenticates(entity: &Entity) -> bool {
        intro2_type53_allocation_authenticates(entity)
    }
    fn manager_authenticates(manager: &EntityManager, id: u32) -> bool {
        type53_manager_allocation_authenticates(manager, id)
    }
    fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
        authenticate_metadata(metadata).is_ok()
    }
    fn sub_d_state(entity: &Entity) -> Option<NativeGroundSubDState> {
        entity
            .intro2_type53_runtime
            .map(|runtime| NativeGroundSubDState {
                origin: crate::native_ground_actor::NativeGroundAllocationOrigin::Authored {
                    spawn_index: runtime.spawn_index,
                },
                runtime: runtime.sub_d_runtime,
                owner: runtime.sub_d_owner,
            })
    }
    fn store_sub_d_state(entity: &mut Entity, state: NativeGroundSubDState) {
        let runtime = entity.intro2_type53_runtime.as_mut().unwrap();
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
        super::capture_pursuit::acquire(manager, id, tick, fx)
    }
    fn tick_capture_primary(
        manager: &mut EntityManager,
        id: u32,
        frame: crate::native_ground_actor::mover::MoverFrame<'_>,
        fx: &mut WorldFx,
    ) -> Result<bool, NativeGroundActorBlock> {
        super::capture_pursuit::tick_primary(manager, id, frame, fx)
            .map(|transition| transition.is_some())
    }
    fn tick_aim(
        manager: &mut EntityManager,
        id: u32,
        dt: u32,
        mode: CommonMoverDispatchMode,
        fx: &mut WorldFx,
    ) -> Result<bool, NativeGroundActorBlock> {
        super::aim::tick_tertiary(manager, id, dt, mode, fx).map(|transition| transition.is_some())
    }
    fn update_attachment(
        manager: &mut EntityManager,
        id: u32,
    ) -> Result<(), NativeGroundActorBlock> {
        crate::native_actor_capture::update_carried_pose(manager, id)
            .map_err(NativeGroundActorBlock::Capture)
    }
}

fn common_terminal(
    owner: Option<Intro2CommonDyingOwner>,
) -> crate::live_actor_checked_damage::LiveActorDeathResult<
    crate::native_ground_actor::NativeGroundTerminalPublication,
> {
    crate::live_actor_checked_damage::LiveActorDeathResult {
        returned_nonzero: owner.is_some(),
        publication: owner
            .map(crate::native_ground_actor::NativeGroundTerminalPublication::CommonDying),
    }
}
