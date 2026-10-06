//! Native Type26 construction: 20450, 416650/4230C0, 425680, then the selected initializer.

use super::*;
use crate::{
    actor_task_dispatcher::{
        prepare_follow_beacons_acquiring_runtime_task, SharedGenericConstructorEffect,
    },
    common_mover::SubAPropulsionRuntime,
    entity_behavior::{initial_behavior_state_policy, select_initial_behavior, BehaviorWeightRule},
    follow_beacons::apply_follow_beacons_acquiring_task_setup,
    sub_h_external_frame::SubHRuntimeState,
    trash_furniture::{find_furniture, publish_trash_furniture},
};
use v2k_formats::anim_frames::TerrainObjectTable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Intro2Type26BirthSelection {
    #[default]
    Weighted,
    CapturedDefecate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type26Publication {
    pub selection: BehaviorSelection,
    pub furniture_nearby: bool,
    pub selector_word: Option<u32>,
}

pub(crate) fn publish_intro2_type26(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    terrain: Option<&TerrainGrid>,
    objects: Option<&TerrainObjectTable>,
    policy: Intro2Type26BirthSelection,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type26Publication, Intro2Type26DefecateVirusPublicationError> {
    use Intro2Type26DefecateVirusPublicationError as Error;
    let admission = authenticate_intro2_type26_defecate_virus(entity, metadata)?;
    if policy == Intro2Type26BirthSelection::CapturedDefecate {
        let admission = publish_intro2_type26_defecate_virus(entity, metadata)?;
        return Ok(Intro2Type26Publication {
            selection: admission.selection(),
            furniture_nearby: false,
            selector_word: None,
        });
    }
    let terrain = terrain.ok_or(Error::TerrainUnavailable)?;
    let spawn = entity.authored_spawn_index.unwrap();
    let xz = match spawn {
        10 => [0xBE00u16 as i16, 0x0F00],
        25 => [0xC200u16 as i16, 0x0E00],
        _ => unreachable!(),
    };
    if [entity.position_raw()[0], entity.position_raw()[2]] != xz {
        return Err(Error::EntityIdentityMismatch);
    }
    publish_retained_birth(
        entity,
        metadata,
        admission.sub_d_frame_owner(),
        terrain,
        objects,
        next_random,
    )
}

/// Both authored and captured entries have authenticated construction before
/// entering 20450/AC60. The process Sub-D receipt is retained without reseeding.
pub(super) fn publish_retained_birth(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    sub_d: Type9SubDFrameOwner,
    terrain: &TerrainGrid,
    objects: Option<&TerrainObjectTable>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type26Publication, Intro2Type26DefecateVirusPublicationError> {
    use Intro2Type26DefecateVirusPublicationError as Error;
    let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
        return Err(Error::AxisUnavailable);
    };
    let RetailRuntimeValue::Known(Some(_)) = entity.sub_a_propulsion_runtime else {
        return Err(Error::SubARuntimeUnavailable);
    };
    let h =
        SubHRuntimeState::new(TYPE26_SUB_H_RECORD_COUNT).map_err(|_| Error::MetadataMismatch)?;
    let mut position = entity.position_raw();
    position[1] = terrain.bilinear_height_raw(position[0], position[2]);
    entity.set_position_raw(position);
    entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(h));
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
        SubAPropulsionRuntime::from_20450_constructor(INTRO2_TYPE26_SUB_A, next_random() as u16),
    ));
    let furniture_nearby = find_furniture(
        terrain,
        objects,
        position,
        i32::from(axis.strict_axis_limit_raw) / 4,
        -1,
    )
    .is_some();
    let mut selector_word = 0;
    let selection = select_initial_behavior(
        &INTRO2_TYPE26_BEHAVIOR_CHOICES,
        |rule| match rule {
            BehaviorWeightRule::Always => 1,
            BehaviorWeightRule::FurnitureNearby => i32::from(furniture_nearby),
            _ => unreachable!(),
        },
        || {
            selector_word = next_random();
            selector_word
        },
    )
    .map_err(|_| Error::BehaviorContextUnavailable)?
    .ok_or(Error::BehaviorContextUnavailable)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Error::BehaviorContextUnavailable)?;
    entity.intro2_type26_sub_d_frame_owner = Some(sub_d);
    entity.intro2_type26_sub_d_runtime =
        Some(crate::common_mover::sub_d::Type9SubDRuntime::from_constructor());
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    publish_selection(entity, metadata, selection, context, next_random)?;
    let state = initial_behavior_state_policy(selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(state.set_bits | state.clear_bits, state.set_bits);
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(
        metadata
            .initializer
            .as_ref()
            .unwrap()
            .common_axis_descriptor,
    );
    // Native deterministic initialization of the allocator's unwritten B2;
    // later mass/animation contributions are owned by the scheduler.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    Ok(Intro2Type26Publication {
        selection,
        furniture_nearby,
        selector_word: Some(selector_word),
    })
}

pub(super) fn publish_selection(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    selection: BehaviorSelection,
    context: BehaviorContextRuntime,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), Intro2Type26DefecateVirusPublicationError> {
    use Intro2Type26DefecateVirusPublicationError as Error;
    let position = entity.position_raw();
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let state = initial_behavior_state_policy(selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(state.set_bits | state.clear_bits, state.set_bits);
    if selection.program.class_id == 4 {
        let RetailRuntimeValue::Known(Some(a)) = &mut entity.sub_a_propulsion_runtime else {
            return Err(Error::SubARuntimeUnavailable);
        };
        apply_defecate_virus_setup(
            &mut entity.actor_tasks,
            DefecateVirusSetupRequest {
                terrain_task_lifetime_ms: INTRO2_DEFECATE_VIRUS_TERRAIN_LIFETIME_MS,
            },
            DefecateVirusSubATopology::Authored(a),
            |preparation| prepare_defecate_virus_runtime_task(preparation, position),
        )
        .map_err(Error::Setup)?;
    } else {
        if selection.program.class_id == 33 {
            let RetailRuntimeValue::Known(Some(a)) = &mut entity.sub_a_propulsion_runtime else {
                return Err(Error::SubARuntimeUnavailable);
            };
            let RetailRuntimeValue::Known(Some(h)) = &mut entity.sub_h_external_frame_runtime
            else {
                return Err(Error::SubHRuntimeUnavailable);
            };
            let mut suffix = |effect| match effect {
                SharedGenericConstructorEffect::WriteSubHState08 { value } => {
                    h.set_enabled(value != 0)
                }
                SharedGenericConstructorEffect::WriteSubADirection {
                    direction_multiplier,
                } => a.set_direction_multiplier(direction_multiplier),
                SharedGenericConstructorEffect::WriteSubATargetSpeed {
                    target_speed_raw, ..
                } => a.apply_shared_initializer_target_speed_write(target_speed_raw),
            };
            apply_follow_beacons_acquiring_task_setup(&mut entity.actor_tasks, |preparation| {
                prepare_follow_beacons_acquiring_runtime_task(preparation, position, metadata)
                    .map(|prepared| prepared.apply_suffix(&mut *next_random, &mut suffix))
            })
            .map_err(|_| Error::BehaviorContextUnavailable)?;
        } else {
            publish_trash_furniture(
                entity,
                metadata,
                -1,
                crate::trash_furniture::TrashFurnitureTargetHeightPolicy::SourceUnresolved,
                next_random,
            )
            .map_err(Error::Furniture)?;
        }
    }

    Ok(())
}
