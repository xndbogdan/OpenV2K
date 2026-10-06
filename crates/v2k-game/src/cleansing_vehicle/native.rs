//! Ordinary authored and zero-record49 construction:104B0/09A80/D4A0/AC60.
use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    common_mover::{
        sub_d::NativeSubDConstruction, type9_attitude::Type9BodyBasis, SubAPropulsionRuntime,
    },
    entity_behavior::{translate_state_policy, BehaviorContextRuntime},
    entity_collision_state::{
        RetailStateWord, ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT,
        SURFACE_STATE_MASK,
    },
};
use v2k_formats::terrain::TerrainGrid;

pub(crate) struct CleansingVehicleConstruction<'a> {
    pub entity: &'a mut Entity,
    pub allocation: MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub preceding: &'a [Entity],
    pub terrain: &'a TerrainGrid,
    pub authored_position_raw: [i16; 3],
    pub spawn_param_nonzero: bool,
    pub constructor_surface_bits: u32,
    pub sub_d: NativeSubDConstruction,
}

pub(crate) fn publish_cleansing_vehicle(
    request: CleansingVehicleConstruction<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), CleansingVehicleError> {
    let CleansingVehicleConstruction {
        entity,
        allocation,
        metadata,
        preceding,
        terrain,
        authored_position_raw,
        spawn_param_nonzero,
        constructor_surface_bits,
        sub_d,
    } = request;
    authenticate_metadata(metadata)?;
    if !entity.active
        || entity.entity_type != 49
        || entity.id != allocation.entity_id
        || entity.model_slots != [Some(266); 4]
        || entity.capability_flags != 0x1204
        || metadata.sub_d_steering_descriptor != RetailRuntimeValue::Known(Some(sub_d.descriptor))
        || constructor_surface_bits & !SURFACE_STATE_MASK != 0
    {
        return Err(CleansingVehicleError::Allocation);
    }
    if entity.cleansing_vehicle_runtime.is_some()
        || entity.initial_behavior != RetailRuntimeValue::Unresolved
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .any(|slot| entity.actor_task_state(slot).is_some())
    {
        return Err(CleansingVehicleError::AlreadyPublished);
    }
    let [x, _, z] = authored_position_raw;
    let cell = terrain
        .cell(usize::from(x as u16 >> 8), usize::from(z as u16 >> 8))
        .ok_or(CleansingVehicleError::Runtime("constructor cell"))?;
    let mut state =
        RetailStateWord::exact(0x0607_8801 | if spawn_param_nonzero { 0x0100_0000 } else { 0 });
    state.overwrite(SURFACE_STATE_MASK, constructor_surface_bits);
    state.overwrite(
        ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
        if cell.terrain_type & 0x10 != 0 {
            ACTIVE_MODEL_SLOT_HIGH_STATE_BIT
        } else {
            0
        },
    );
    let init = metadata.initializer.as_ref().unwrap();
    let policy = translate_state_policy(init.initializer_state_flags_raw);
    state.overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    entity.collision.state_flags_at_0x08 = state;
    entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Known(None);
    entity
        .collision
        .pair_callbacks
        .damage_modifier_identity_context_empty = RetailRuntimeValue::Known(true);
    entity.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Known(None);
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(AXIS);
    let RetailRuntimeValue::Known(Some(a)) = metadata.sub_a_propulsion_descriptor else {
        unreachable!()
    };
    //09A80: D construction precedes A20450's one random target-speed word.
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
        SubAPropulsionRuntime::from_20450_constructor(a, next_random() as u16),
    ));
    let RetailRuntimeValue::Known(position) = crate::entity_initializer::constructor_position_raw(
        metadata,
        authored_position_raw,
        Some(terrain),
        None,
    ) else {
        return Err(CleansingVehicleError::Runtime("D4A0 terrain alignment"));
    };
    entity.set_position_raw(position);
    entity.model_index = Some(266);
    entity.cleansing_vehicle_runtime = Some(CleansingVehicleRuntime {
        allocation,
        spawn_index: entity.authored_spawn_index,
        immutable_anchor_raw: authored_position_raw,
        sub_d_runtime: sub_d.runtime,
        sub_d_owner: sub_d.frame_owner,
    });
    let selection = tasks::select_behavior(entity, preceding, metadata, next_random)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(CleansingVehicleError::Graph)?;
    tasks::publish_selection(entity, metadata, selection, context, next_random)?;
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    Ok(())
}
