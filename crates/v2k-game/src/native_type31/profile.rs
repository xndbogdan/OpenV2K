//! Sealed Type31/Type105 receipts over the shared ground task/mover/hit host.
//! Neither row captures: the AbsentJ dispatch lends the task custody class18
//! needs to publish its children inline; Type105's quiet death uses none.

use super::*;
use crate::{
    common_mover::component_dispatch::CommonMoverDispatchMode,
    live_actor_checked_damage::LiveActorDeathResult,
    native_ground_actor::{
        NativeGroundActorBlock, NativeGroundActorProfile, NativeGroundAllocationOrigin,
        NativeGroundSubDState, NativeGroundTerminalPublication,
    },
};

fn sub_d_state(entity: &Entity) -> Option<NativeGroundSubDState> {
    entity.native_type31_runtime.map(|r| NativeGroundSubDState {
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
    let r = entity.native_type31_runtime.as_mut().unwrap();
    r.sub_d_runtime = state.runtime;
    r.sub_d_owner = state.owner;
}

/// Living graphs of classes 4 (`4C7E88`), 5 (`4C7930`) and 10 (acquiring
/// `4C7618`, running `4C7660`), restricted to the row's own choices.
fn graph_authenticates(row: Type31Row, entity: &Entity) -> bool {
    use crate::{
        actor_task_dispatcher::ActorTaskRuntime as Task, actor_task_owner::ActorTaskSlot as Slot,
    };
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return false;
    };
    if entity.sub_j_attachment_runtime != RetailRuntimeValue::Known(None) {
        return false;
    }
    let primary = entity.actor_task_state(Slot::Primary);
    let secondary = entity.actor_task_state(Slot::Secondary);
    let tertiary = entity.actor_task_state(Slot::Tertiary);
    match context.active_style().style_address() {
        0x004c7e88 => {
            matches!(primary, Some(Task::DefecateVirusWander(_)))
                && secondary.is_none()
                && matches!(tertiary, Some(Task::DefecateVirusTerrain(task)) if task.lifetime_ms() == 0)
        }
        0x004c7930 => {
            row == Type31Row::Type31
                && matches!(primary, Some(Task::SharedRetarget(_)))
                && secondary.is_none()
                && tertiary.is_none()
        }
        0x004c7618 => {
            matches!(primary, Some(Task::SharedRetarget(_)))
                && matches!(secondary, Some(Task::TargetAcquisition(_)))
                && tertiary.is_none()
        }
        0x004c7660 => {
            matches!((primary, context.target_handle_at_0x08()),
                (Some(Task::RunAway(task)), RetailRuntimeValue::Known(Some(target)))
                    if task.target_id() == target)
                && secondary.is_none()
                && tertiary.is_none()
        }
        _ => false,
    }
}

macro_rules! type31_profile {
    ($name:ident, $row:expr, $variant:ident, $classes:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name;
        impl NativeGroundActorProfile for $name {}
        impl shared::sealed::Sealed for $name {
            const ENTITY_TYPE: u32 = $row.entity_type();
            const CAPTURE_POLICY: shared::NativeCapturePolicy =
                shared::NativeCapturePolicy::AbsentJ;
            const DEFAULT_FLAGS: u32 = $row.default_flags();
            const TOPOLOGY: CommonMoverComponentTopology = $row.topology();
            const AXIS: CommonAxisDescriptor = $row.axis();
            const CHOICES: &'static [BehaviorChoice] = $row.choices();
            const LIVING_CLASSES: &'static [u8] = $classes;
            fn register_owner(
                scheduler: &mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
                owner: shared::NativeGroundActorOwner<Self>,
            ) {
                scheduler.register_type31_family(Type31FamilyOwner::$variant(owner));
            }
            fn graph_authenticates(entity: &Entity) -> bool {
                graph_authenticates($row, entity)
            }
            fn completed_terminal_authenticates(manager: &EntityManager, id: u32) -> bool {
                super::death::finished_terminal_authenticates(manager, id)
            }
            fn allocation_authenticates(entity: &Entity) -> bool {
                type31_row(entity) == Some($row)
            }
            fn manager_authenticates(manager: &EntityManager, id: u32) -> bool {
                manager_allocation_authenticates(manager, id)
                    && manager
                        .iter_all()
                        .find(|entity| entity.id == id)
                        .and_then(type31_row)
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
                publish_death($row, manager, id, context)
            }
            fn publish_direct_surface_death(
                manager: &mut EntityManager,
                id: u32,
                context: &mut shared::NativeGroundDeathContext<'_>,
            ) -> Result<
                LiveActorDeathResult<NativeGroundTerminalPublication>,
                crate::intro2_common_dying::Intro2CommonDyingBlock,
            > {
                publish_death($row, manager, id, context)
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
            // No class7: the allocated E never aims.
            fn tick_aim(
                _: &mut EntityManager,
                _: u32,
                _: u32,
                _: CommonMoverDispatchMode,
                _: &mut WorldFx,
            ) -> Result<bool, NativeGroundActorBlock> {
                Err(NativeGroundActorBlock::Graph)
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
                    Err(NativeGroundActorBlock::Runtime("Type31 true absent Sub-J"))
                }
            }
        }
    };
}

type31_profile!(Type31Profile, Type31Row::Type31, Type31, &[4, 5, 10]);
type31_profile!(Type105Profile, Type31Row::Type105, Type105, &[4, 10]);

fn publish_death(
    row: Type31Row,
    manager: &mut EntityManager,
    id: u32,
    context: &mut shared::NativeGroundDeathContext<'_>,
) -> Result<
    LiveActorDeathResult<NativeGroundTerminalPublication>,
    crate::intro2_common_dying::Intro2CommonDyingBlock,
> {
    use crate::intro2_common_dying::Intro2CommonDyingBlock as Block;
    match row {
        Type31Row::Type105 => super::death::begin_type105_standard_death(manager, id)
            .map_err(|_| Block::Runtime("Type105 quiet death custody")),
        Type31Row::Type31 => {
            let shared::NativeGroundDeathContext::Split {
                resources,
                fx,
                tick,
                tasks,
            } = context
            else {
                return Err(Block::Runtime("Type31 inline split custody"));
            };
            super::death::begin_type31_standard_death(
                manager,
                id,
                super::death::Type31Class18Frame {
                    resources,
                    world_fx: fx,
                    retail_tick: *tick,
                    tasks: &mut **tasks,
                },
            )
            .map_err(|_| Block::Runtime("Type31 split terminal prefix"))
        }
    }
}
