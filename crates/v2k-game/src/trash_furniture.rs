//! Shared class26 B7C0/1CA0/1D30 construction and 1DA0 furniture callback.

use crate::{
    actor_task_dispatcher::{
        prepare_shared_generic_constructor_suffix, ActorTaskRuntime,
        SharedGenericConstructorEffect, SharedGenericConstructorSuffixError,
        SharedGenericConstructorTopology,
    },
    actor_task_owner::{ActorTaskOwner, ActorTaskSlot, ActorTaskVisit, PreparedActorTask},
    entity::Entity,
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    wander_near_location::WanderNearPrivateState,
};
use v2k_formats::{
    anim_frames::TerrainObjectTable, collision::CommonAxisDescriptor, terrain::TerrainGrid,
};

/// 4230C0 preserves low coordinate bytes and searches its asymmetric edges
/// in this exact order (EXE 423170/4232D8). Current and outer-radius cells
/// are excluded. This scanner has no RNG or actor-family policy.
pub(crate) fn find_furniture(
    terrain: &TerrainGrid,
    objects: Option<&TerrainObjectTable>,
    position_raw: [i16; 3],
    radius_raw: i32,
    kind_filter: i32,
) -> Option<[i16; 2]> {
    let objects = objects?;
    for ring in 1..radius_raw / 256 {
        for step in 0..ring * 2 {
            for [dx, dz] in [
                [-ring, -ring - step],
                [ring, ring - step],
                [-ring - step, ring],
                [ring - step, -ring],
            ] {
                let x = position_raw[0].wrapping_add((dx * 256) as i16);
                let z = position_raw[2].wrapping_add((dz * 256) as i16);
                let cell = terrain.cell(usize::from(x as u16 >> 8), usize::from(z as u16 >> 8))?;
                if cell.attribute == 0 || cell.terrain_type & 8 != 0 {
                    continue;
                }
                let Some(object) = objects.records.get(usize::from(cell.attribute)) else {
                    continue;
                };
                if kind_filter == -1 || object.kind_index == kind_filter as u32 {
                    return Some([x, z]);
                }
            }
        }
    }
    None
}

/// 1D30 writes X/Z, tracked handle, kind, direction and reversal timer, but
/// leaves target Y as allocator residue. Only a successful scan resolves it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrashFurnitureTargetHeightPolicy {
    SourceUnresolved,
    /// Native Type30's unconditional Furniture choice must complete the failed
    /// scan's 01430 prefix. Until allocator residue is owned, use body Y only
    /// at construction. ALPINE_INSECTS owns this replaceable approximation;
    /// it is not retail acceptance. A successful scan replaces it with retail Y.
    InitialActorHeightApproximation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrashFurnitureTaskState {
    target: WanderNearPrivateState,
    target_y: RetailRuntimeValue<i16>,
    target_y_is_approximation: bool,
    kind_filter: i32,
    elapsed_ms: u32,
}

impl TrashFurnitureTaskState {
    fn new(
        kind_filter: i32,
        policy: TrashFurnitureTargetHeightPolicy,
        actor_height_raw: i16,
    ) -> Self {
        let approximate =
            policy == TrashFurnitureTargetHeightPolicy::InitialActorHeightApproximation;
        Self {
            target: WanderNearPrivateState::ordinary_type9([
                0,
                if approximate { actor_height_raw } else { 0 },
                0,
            ]),
            target_y: if approximate {
                RetailRuntimeValue::Known(actor_height_raw)
            } else {
                RetailRuntimeValue::Unresolved
            },
            target_y_is_approximation: approximate,
            kind_filter,
            elapsed_ms: 0,
        }
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.elapsed_ms
    }
    pub const fn kind_filter(self) -> i32 {
        self.kind_filter
    }
    pub const fn target_y(self) -> RetailRuntimeValue<i16> {
        self.target_y
    }
    pub const fn target_y_is_approximation(self) -> bool {
        self.target_y_is_approximation
    }
    pub const fn target_position_raw(self) -> [i16; 3] {
        self.target.target_position_raw
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrashFurniturePublicationError {
    Constructor(SharedGenericConstructorSuffixError),
    SubAUnavailable,
    SubHUnavailable,
}

/// B7C0's destructive prefix precedes the fallible 1CA0 allocation. On
/// failure the old Primary remains until C6B0's initializer fallback.
fn apply_setup<E>(
    tasks: &mut ActorTaskOwner<ActorTaskRuntime>,
    prepare: impl FnOnce() -> Result<PreparedActorTask<ActorTaskRuntime>, E>,
) -> Result<(), E> {
    tasks.clear_slot(ActorTaskSlot::Tertiary);
    tasks.clear_slot(ActorTaskSlot::Secondary);
    let prepared = prepare()?;
    tasks.replace_prepared(ActorTaskSlot::Primary, prepared);
    Ok(())
}

/// Successful 401D03 -> 406030 -> 406070 enables H, then consumes one A
/// constructor word before publishing Primary. E is not a 6070 consumer.
pub(crate) fn publish_trash_furniture(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    kind_filter: i32,
    target_height_policy: TrashFurnitureTargetHeightPolicy,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), TrashFurniturePublicationError> {
    let prepared = prepare_shared_generic_constructor_suffix(
        PreparedActorTask::new(ActorTaskRuntime::TrashFurniture(
            TrashFurnitureTaskState::new(
                kind_filter,
                target_height_policy,
                entity.position_raw()[1],
            ),
        )),
        metadata,
    )
    .map_err(TrashFurniturePublicationError::Constructor)?;
    let RetailRuntimeValue::Known(Some(a)) = &mut entity.sub_a_propulsion_runtime else {
        return Err(TrashFurniturePublicationError::SubAUnavailable);
    };
    let h = match &mut entity.sub_h_external_frame_runtime {
        RetailRuntimeValue::Known(Some(h)) => Some(h),
        _ if prepared.topology() == SharedGenericConstructorTopology::SubAOnly => None,
        _ => return Err(TrashFurniturePublicationError::SubHUnavailable),
    };
    let mut h = h;
    apply_setup(&mut entity.actor_tasks, || {
        Ok(prepared.apply_suffix(next_random, |effect| match effect {
            SharedGenericConstructorEffect::WriteSubHState08 { value } => {
                h.as_mut().unwrap().set_enabled(value != 0)
            }
            SharedGenericConstructorEffect::WriteSubADirection {
                direction_multiplier,
            } => a.set_direction_multiplier(direction_multiplier),
            SharedGenericConstructorEffect::WriteSubATargetSpeed {
                target_speed_raw, ..
            } => a.apply_shared_initializer_target_speed_write(target_speed_raw),
        }))
    })
}

pub(crate) struct TrashFurnitureFrame<'a> {
    pub terrain: &'a TerrainGrid,
    pub objects: Option<&'a TerrainObjectTable>,
    pub elapsed_micros: u32,
    pub axis: CommonAxisDescriptor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrashFurnitureTick {
    Retired,
    Continue,
    ScanFailed,
    TimedOut,
}

impl TrashFurnitureTick {
    pub const fn requests_reselection(self) -> bool {
        matches!(self, Self::ScanFailed | Self::TimedOut)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrashFurnitureError<E> {
    Graph,
    UnresolvedTargetY,
    Mover(E),
}

/// Exact 01120 visit around 1DA0. A failed scan still calls 01430 before
/// returning 9C01; the mover Boolean is discarded. Callback errors preserve
/// elapsed/scan/mover prefixes and always unwind. Only a surviving wrapper
/// can request the tag or strict >2000-ms transition.
pub(crate) fn tick_trash_furniture<E>(
    entity: &mut Entity,
    frame: TrashFurnitureFrame<'_>,
    next_random: &mut impl FnMut() -> u32,
    mover: impl FnOnce(
        &mut Entity,
        &mut WanderNearPrivateState,
        &mut dyn FnMut() -> u32,
    ) -> Result<bool, E>,
) -> Result<TrashFurnitureTick, TrashFurnitureError<E>> {
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(TrashFurnitureError::Graph)?;
    if !matches!(
        entity.actor_tasks.task_state(task_id),
        Some(ActorTaskRuntime::TrashFurniture(_))
    ) {
        return Err(TrashFurnitureError::Graph);
    }
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let mut task = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::TrashFurniture(task) = runtime else {
                unreachable!()
            };
            task.elapsed_ms = task.elapsed_ms.wrapping_add(frame.elapsed_micros / 1000);
            *task
        })
        .ok_or(TrashFurnitureError::Graph)?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let scan = task.target.target_position_raw[0] == 0
            && task.target.target_position_raw[2] == 0
            || (next_random() as u16) < 0x0fff;
        let mut failed = false;
        if scan {
            if let Some([x, z]) = find_furniture(
                frame.terrain,
                frame.objects,
                entity.position_raw(),
                i32::from(frame.axis.strict_axis_limit_raw),
                task.kind_filter,
            ) {
                let y = frame.terrain.bilinear_height_raw(x, z);
                task.target.target_position_raw = [x, y, z];
                task.target_y = RetailRuntimeValue::Known(y);
                task.target_y_is_approximation = false;
            } else {
                failed = true;
            }
        }
        if task.target_y == RetailRuntimeValue::Unresolved {
            return Err(TrashFurnitureError::UnresolvedTargetY);
        }
        let result = mover(entity, &mut task.target, next_random);
        task.target_y = RetailRuntimeValue::Known(task.target.target_position_raw[1]);
        result.map_err(TrashFurnitureError::Mover)?;
        Ok(if failed {
            TrashFurnitureTick::ScanFailed
        } else if task.elapsed_ms > 2000 {
            TrashFurnitureTick::TimedOut
        } else {
            TrashFurnitureTick::Continue
        })
    }));
    if let Some(ActorTaskRuntime::TrashFurniture(surviving)) =
        entity.actor_tasks.task_state_mut(task_id)
    {
        *surviving = task;
    }
    let survived = entity.actor_tasks.finish_exact_visit(visit);
    match result {
        Ok(result) => result.map(|outcome| {
            if survived {
                outcome
            } else {
                TrashFurnitureTick::Retired
            }
        }),
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

#[cfg(test)]
#[path = "trash_furniture_tests.rs"]
mod tests;
