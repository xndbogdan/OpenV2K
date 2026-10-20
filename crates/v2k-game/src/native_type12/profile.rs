//! Sealed Type12/Type127 receipts over the shared ground task/mover/hit host,
//! with Type40's true absent-J capture policy and the class49 terminals.

use super::*;
use crate::{
    common_mover::component_dispatch::CommonMoverDispatchMode,
    intro2_capture_pursuit::{
        self as capture, Intro2CaptureBlock, Intro2CapturePrimaryFrame, Intro2CaptureProfile,
    },
    live_actor_checked_damage::LiveActorDeathResult,
    native_ground_actor::{
        NativeGroundActorBlock, NativeGroundActorProfile, NativeGroundAllocationOrigin,
        NativeGroundSubDState, NativeGroundTerminalPublication,
    },
};

fn sub_d_state(entity: &Entity) -> Option<NativeGroundSubDState> {
    entity
        .native_type12_runtime
        .as_ref()
        .map(|r| NativeGroundSubDState {
            origin: match r.spawn_index {
                Some(spawn_index) => NativeGroundAllocationOrigin::Authored { spawn_index },
                None => NativeGroundAllocationOrigin::Dynamic {
                    allocation: r.allocation,
                },
            },
            runtime: r.sub_d_runtime,
            owner: r.sub_d_owner,
        })
}

fn store_sub_d_state(entity: &mut Entity, state: NativeGroundSubDState) {
    let r = entity.native_type12_runtime.as_mut().unwrap();
    r.sub_d_runtime = state.runtime;
    r.sub_d_owner = state.owner;
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

macro_rules! type12_profile {
    ($name:ident, $row:expr, $variant:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name;
        impl NativeGroundActorProfile for $name {}
        impl shared::sealed::Sealed for $name {
            const ENTITY_TYPE: u32 = $row.entity_type();
            const CAPTURE_POLICY: shared::NativeCapturePolicy =
                shared::NativeCapturePolicy::AbsentJ;
            const DEFAULT_FLAGS: u32 = DEFAULT_FLAGS;
            const TOPOLOGY: crate::entity_collision_state::CommonMoverComponentTopology = TOPOLOGY;
            const AXIS: CommonAxisDescriptor = $row.axis();
            const CHOICES: &'static [BehaviorChoice] = &CHOICES;
            const LIVING_CLASSES: &'static [u8] = &[7, 9, 15, 33];
            const TERMINAL_DEATH: bool = true;
            fn register_owner(
                scheduler: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
                owner: shared::NativeGroundActorOwner<Self>,
            ) {
                scheduler.register_type12_family(Type12FamilyOwner::$variant(owner));
            }
            fn allocation_authenticates(entity: &Entity) -> bool {
                type12_row(entity) == Some($row)
            }
            fn manager_authenticates(manager: &EntityManager, id: u32) -> bool {
                manager_allocation_authenticates(manager, id)
                    && manager
                        .iter_all()
                        .find(|entity| entity.id == id)
                        .and_then(type12_row)
                        == Some($row)
            }
            fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
                authenticate_metadata($row, metadata).is_ok()
            }
            // BAC0 cleared the class1 corpse's tasks and BC90 kept the
            // carrier's: either way only the Finished receipt remains until
            // 14990, and both completion styles have null hit callbacks.
            fn completed_terminal_authenticates(manager: &EntityManager, id: u32) -> bool {
                crate::class49_death::finished_terminal_hit_authenticates(manager, id)
            }
            fn sub_d_state(entity: &Entity) -> Option<NativeGroundSubDState> {
                sub_d_state(entity)
            }
            fn store_sub_d_state(entity: &mut Entity, state: NativeGroundSubDState) {
                store_sub_d_state(entity, state)
            }
            fn kl_components(entity: &Entity) -> Option<Intro2KlComponents> {
                entity
                    .native_type12_runtime
                    .as_ref()
                    .map(|r| r.kl_components.clone())
            }
            fn store_kl_components(entity: &mut Entity, components: Intro2KlComponents) {
                entity.native_type12_runtime.as_mut().unwrap().kl_components = components;
            }
            fn publish_acquiring(
                entity: &mut Entity,
                metadata: &EntityTypeRuntimeMetadata,
                selection: BehaviorSelection,
                context: BehaviorContextRuntime,
                next_random: &mut impl FnMut() -> u32,
            ) -> bool {
                super::construction::publish_acquiring(
                    entity,
                    metadata,
                    selection,
                    context,
                    next_random,
                )
            }
            fn publish_standard_death(
                manager: &mut EntityManager,
                id: u32,
                context: &mut shared::NativeGroundDeathContext<'_>,
            ) -> Result<
                LiveActorDeathResult<NativeGroundTerminalPublication>,
                crate::intro2_common_dying::Intro2CommonDyingBlock,
            > {
                context.run_class49_terminal(manager, id)
            }
            fn publish_direct_surface_death(
                _: &mut EntityManager,
                _: u32,
                _: &mut shared::NativeGroundDeathContext<'_>,
            ) -> Result<
                LiveActorDeathResult<NativeGroundTerminalPublication>,
                crate::intro2_common_dying::Intro2CommonDyingBlock,
            > {
                // E370's 162B0 lifecycle would run BAC0/BC90 inside the tick,
                // whose surface frame still borrows the static world BAF0
                // mutates. Held until the tick can lend that world.
                Err(crate::intro2_common_dying::Intro2CommonDyingBlock::Runtime(
                    "E370 terminal death needs the tick's static world",
                ))
            }
            fn acquire_capture(
                manager: &mut EntityManager,
                id: u32,
                tick: u32,
                fx: &mut WorldFx,
            ) -> Result<(), NativeGroundActorBlock> {
                capture::acquire(manager, id, tick, fx, Intro2CaptureProfile::Type12Family)
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
                    .type_runtime_metadata($row.entity_type())
                    .cloned()
                    .ok_or(NativeGroundActorBlock::Metadata)?;
                super::aim::tick_type12_family_aim(mode, manager, fx, id, dt, Some(&metadata))
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
                    Err(NativeGroundActorBlock::Runtime("Type12 true absent Sub-J"))
                }
            }
        }
    };
}

type12_profile!(Type12Profile, Type12Row::Type12, Type12);
type12_profile!(Type127Profile, Type12Row::Type127, Type127);
