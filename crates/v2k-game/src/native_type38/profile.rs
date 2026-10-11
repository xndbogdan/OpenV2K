//! Sealed Type38/Type129 receipts over the shared ground task/mover/hit host.

use super::*;
use crate::{
    common_mover::component_dispatch::CommonMoverDispatchMode,
    live_actor_checked_damage::LiveActorDeathResult,
    native_ground_actor::{
        NativeGroundActorBlock, NativeGroundActorProfile, NativeGroundAllocationOrigin,
        NativeGroundSubDState, NativeGroundTerminalPublication,
    },
};

fn graph_authenticates<P: NativeGroundActorProfile>(entity: &Entity) -> bool {
    use crate::{
        actor_task_dispatcher::ActorTaskRuntime as Task, actor_task_owner::ActorTaskSlot as Slot,
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
        0x4c7a98 => shared::search::pursuing_graph_authenticates::<P>(entity),
        _ => false,
    }
}

fn sub_d_state(entity: &Entity) -> Option<NativeGroundSubDState> {
    entity
        .native_type38_runtime
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
    let r = entity.native_type38_runtime.as_mut().unwrap();
    r.sub_d_runtime = state.runtime;
    r.sub_d_owner = state.owner;
}

fn update_attachment(manager: &mut EntityManager, id: u32) -> Result<(), NativeGroundActorBlock> {
    let entity = manager
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(NativeGroundActorBlock::Allocation)?;
    if entity.sub_j_attachment_runtime == RetailRuntimeValue::Known(None) {
        Ok(())
    } else {
        Err(NativeGroundActorBlock::Runtime("Type38 absent Sub-J"))
    }
}

macro_rules! type38_profile {
    ($name:ident, $row:expr, $variant:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name;
        impl NativeGroundActorProfile for $name {}
        impl shared::sealed::Sealed for $name {
            const ENTITY_TYPE: u32 = $row.entity_type();
            const CAPTURE_POLICY: shared::NativeCapturePolicy =
                shared::NativeCapturePolicy::NoCapture;
            const DEFAULT_FLAGS: u32 = DEFAULT_FLAGS;
            const TOPOLOGY: crate::entity_collision_state::CommonMoverComponentTopology = TOPOLOGY;
            const AXIS: CommonAxisDescriptor = $row.axis();
            const CHOICES: &'static [BehaviorChoice] = &CHOICES;
            const LIVING_CLASSES: &'static [u8] = &[5, 7];
            const TERMINAL_DEATH: bool = true;
            fn register_owner(
                scheduler: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
                owner: shared::NativeGroundActorOwner<Self>,
            ) {
                scheduler.register_type38_family(Type38FamilyOwner::$variant(owner));
            }
            fn allocation_authenticates(entity: &Entity) -> bool {
                type38_row(entity) == Some($row)
            }
            fn manager_authenticates(manager: &EntityManager, id: u32) -> bool {
                manager_allocation_authenticates(manager, id)
                    && manager
                        .iter_all()
                        .find(|entity| entity.id == id)
                        .and_then(type38_row)
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
            fn graph_authenticates(entity: &Entity) -> bool {
                graph_authenticates::<Self>(entity)
            }
            fn sub_d_state(entity: &Entity) -> Option<NativeGroundSubDState> {
                sub_d_state(entity)
            }
            fn store_sub_d_state(entity: &mut Entity, state: NativeGroundSubDState) {
                store_sub_d_state(entity, state)
            }
            fn kl_components(entity: &Entity) -> Option<Intro2KlComponents> {
                entity
                    .native_type38_runtime
                    .as_ref()
                    .map(|r| r.kl_components.clone())
            }
            fn store_kl_components(entity: &mut Entity, components: Intro2KlComponents) {
                entity.native_type38_runtime.as_mut().unwrap().kl_components = components;
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
                    .type_runtime_metadata($row.entity_type())
                    .cloned()
                    .ok_or(NativeGroundActorBlock::Metadata)?;
                super::aim::tick_type38_family_aim(mode, manager, fx, id, dt, Some(&metadata))
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
                update_attachment(manager, id)
            }
        }
    };
}

type38_profile!(Type38Profile, Type38Row::Type38, Type38);
type38_profile!(Type129Profile, Type38Row::Type129, Type129);
