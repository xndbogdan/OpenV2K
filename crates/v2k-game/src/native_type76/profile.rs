//! Sealed Type76/Type77 receipts over the shared ground task/mover/hit host.

use super::*;
use crate::{
    common_mover::component_dispatch::CommonMoverDispatchMode,
    intro2_common_dying::Intro2CommonDyingOwner,
    native_ground_actor::{
        NativeGroundActorBlock, NativeGroundActorProfile, NativeGroundSubDState,
        NativeGroundTerminalPublication,
    },
};

fn sub_d_state(entity: &Entity) -> Option<NativeGroundSubDState> {
    entity
        .native_type76_runtime
        .map(|runtime| NativeGroundSubDState {
            origin: shared::NativeGroundAllocationOrigin::Authored {
                spawn_index: runtime.spawn_index,
            },
            runtime: runtime.sub_d_runtime,
            owner: runtime.sub_d_owner,
        })
}

fn store_sub_d_state(entity: &mut Entity, state: NativeGroundSubDState) {
    let runtime = entity.native_type76_runtime.as_mut().unwrap();
    runtime.sub_d_runtime = state.runtime;
    runtime.sub_d_owner = state.owner;
}

fn update_attachment(manager: &mut EntityManager, id: u32) -> Result<(), NativeGroundActorBlock> {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(NativeGroundActorBlock::Allocation)?;
    if entity.sub_j_attachment_runtime == RetailRuntimeValue::Known(None) {
        Ok(())
    } else {
        Err(NativeGroundActorBlock::Runtime(
            "Type76-family absent Sub-J",
        ))
    }
}

fn common_terminal(
    owner: Option<Intro2CommonDyingOwner>,
) -> crate::live_actor_checked_damage::LiveActorDeathResult<NativeGroundTerminalPublication> {
    crate::live_actor_checked_damage::LiveActorDeathResult {
        returned_nonzero: owner.is_some(),
        publication: owner.map(NativeGroundTerminalPublication::CommonDying),
    }
}

macro_rules! type76_profile {
    ($name:ident, $row:expr, $variant:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name;
        impl NativeGroundActorProfile for $name {}
        impl shared::sealed::Sealed for $name {
            const ENTITY_TYPE: u32 = $row.entity_type();
            const CAPTURE_POLICY: shared::NativeCapturePolicy =
                shared::NativeCapturePolicy::NoCapture;
            const DEFAULT_FLAGS: u32 = $row.default_flags();
            const TOPOLOGY: CommonMoverComponentTopology = TOPOLOGY;
            const AXIS: CommonAxisDescriptor = $row.axis();
            const CHOICES: &'static [BehaviorChoice] = $row.choices();
            const LIVING_CLASSES: &'static [u8] = $row.living_classes();
            fn register_owner(
                scheduler: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
                owner: shared::NativeGroundActorOwner<Self>,
            ) {
                scheduler.register_type76_family(Type76FamilyOwner::$variant(owner));
            }
            fn allocation_authenticates(entity: &Entity) -> bool {
                type76_row(entity) == Some($row)
            }
            fn manager_authenticates(manager: &EntityManager, id: u32) -> bool {
                manager_allocation_authenticates(manager, id)
                    && manager
                        .iter_all()
                        .find(|entity| entity.id == id)
                        .and_then(type76_row)
                        == Some($row)
            }
            fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
                authenticate_metadata($row, metadata).is_ok()
            }
            fn sub_d_state(entity: &Entity) -> Option<NativeGroundSubDState> {
                sub_d_state(entity)
            }
            fn store_sub_d_state(entity: &mut Entity, state: NativeGroundSubDState) {
                store_sub_d_state(entity, state)
            }
            fn publish_acquiring(
                entity: &mut Entity,
                metadata: &EntityTypeRuntimeMetadata,
                selection: BehaviorSelection,
                context: BehaviorContextRuntime,
                rng: &mut impl FnMut() -> u32,
            ) -> bool {
                super::publish_acquiring(entity, metadata, selection, context, rng)
            }
            fn publish_standard_death(
                manager: &mut EntityManager,
                id: u32,
                context: &mut shared::NativeGroundDeathContext<'_>,
            ) -> Result<
                crate::live_actor_checked_damage::LiveActorDeathResult<
                    NativeGroundTerminalPublication,
                >,
                crate::intro2_common_dying::Intro2CommonDyingBlock,
            > {
                crate::intro2_common_dying::publish_intro2_common_standard_death(
                    manager,
                    id,
                    context.world_fx(),
                )
                .map(common_terminal)
            }
            // E370's 2000-ms lifecycle death is the same class12.
            fn publish_direct_surface_death(
                manager: &mut EntityManager,
                id: u32,
                context: &mut shared::NativeGroundDeathContext<'_>,
            ) -> Result<
                crate::live_actor_checked_damage::LiveActorDeathResult<
                    NativeGroundTerminalPublication,
                >,
                crate::intro2_common_dying::Intro2CommonDyingBlock,
            > {
                crate::intro2_common_dying::publish_intro2_common_standard_death(
                    manager,
                    id,
                    context.world_fx(),
                )
                .map(common_terminal)
            }
            // Neither row captures.
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
                super::aim::tick_type76_family_aim(mode, manager, fx, id, dt)
                    .map_err(NativeGroundActorBlock::Aim)
                    .map(|outcome| {
                        matches!(
                            outcome.resolution.outcome,
                            crate::aim_and_fire::AimAndFireFrameOutcome::RequestOwnerTransition { .. }
                        )
                    })
            }
            fn update_attachment(
                manager: &mut EntityManager,
                id: u32,
            ) -> Result<(), NativeGroundActorBlock> {
                update_attachment(manager, id)
            }
        }
    };
}

type76_profile!(Type76Profile, Type76Row::Type76, Type76);
type76_profile!(Type77Profile, Type76Row::Type77, Type77);
