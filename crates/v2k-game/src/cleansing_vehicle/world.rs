//! DCA0/E870 suffix; E100 rereads the style after tasks may replace it.
use super::*;
use crate::{
    common_mover::{
        type9_attitude::*,
        type9_surface::{decay_actor_surface_timer_ms, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT},
        type9_tail::apply_type9_ground_snap_raw,
    },
    entity::{
        apply_common_gravity_and_underwater_raw, apply_common_no_wind_drag_raw,
        common_environment_surface_raw, CommonEnvironmentPhysics, CommonUnderwaterFrame,
    },
    entity_collision_state::BODY_BASIS_REBUILT_STATE_BIT,
};

pub(super) fn effective_flags(entity: &Entity) -> Result<u32, CleansingVehicleBlock> {
    use CleansingVehicleBlock as Block;
    if entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(0x8039) {
        return Err(Block::Runtime("cleansing default environment"));
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    let BehaviorDescriptorIdentity::Named(program) = context.descriptor() else {
        return Err(Block::Graph);
    };
    // Type49 +FC=8039; style+34/+38: AD50=(0,0), ADB0=(80,2), C490=(A023,0).
    match (program.class_id, context.style_table_index_raw_at_0x10()) {
        (42, 0) => Ok(0x8039),
        (42, 1) => Ok((0x8039 | 0x80) & !2),
        (68, 0) => Ok(0x8039 | 0xa023),
        _ => Err(Block::Graph),
    }
}

pub(super) fn finish(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    frame: &mut CleansingVehicleFrame<'_>,
    dt: u32,
    detailed: bool,
    entry_flags: u32,
) -> Result<(), CleansingVehicleBlock> {
    use CleansingVehicleBlock as Block;
    let environment = manager.intro2_type13_environment();
    let waves_enabled = manager.common_environment_physics().waves_enabled;
    if environment.0 != 0 {
        return Err(Block::Runtime("cleansing nonzero wind mode"));
    }
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    if detailed {
        let record = frame
            .resources
            .global_entity_type(49)
            .ok_or(Block::Metadata)?;
        let RetailRuntimeValue::Known(health_raw) = entity.collision.health_raw else {
            return Err(Block::Runtime("cleansing health"));
        };
        if let Some(sound) = crate::actor_detailed_sound::plan_actor_detailed_sound(
            crate::actor_detailed_sound::ActorDetailedSoundFrame {
                type_record: record,
                health_raw,
                visible: bits(entity, 0x800)? != 0,
                callback_elapsed_micros: dt,
            },
            &mut || u32::from(frame.world_fx.next_shared_retail_random_u16()),
        ) {
            frame
                .world_fx
                .queue_fixed_positional_sound_raw(sound, entity.position_raw());
        }
    }
    let terrain = frame
        .resources
        .level_terrain()
        .ok_or(Block::Runtime("cleansing terrain"))?;
    let model = entity
        .model_index
        .and_then(|id| frame.resources.global_model(id))
        .ok_or(Block::Runtime("cleansing model"))?;
    let [heading, mut pitch, mut roll] = entity.rotation_heading_pitch_roll_raw();
    if entry_flags & 0x10 != 0 {
        let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
            return Err(Block::Runtime("cleansing basis"));
        };
        let attitude = plan_type9_terrain_attitude_raw(Type9TerrainAttitudeInput {
            terrain,
            position_raw: entity.position_raw(),
            pitch_raw: pitch,
            roll_raw: roll,
            lateral_basis_q31: basis.lateral,
            forward_basis_q31: basis.forward,
            state_flags: bits(entity, TERRAIN_ATTITUDE_BILINEAR_STATE_BIT)?,
            effective_flags: entry_flags,
            active_model_extent_raw: model.radius,
            resolved_surface_mode_raw: 1,
            water_enabled: waves_enabled,
            wave_tick_50hz: frame.retail_tick as i32,
            effective_elapsed_micros: dt,
        });
        pitch = attitude.pitch_raw;
        roll = attitude.roll_raw;
        entity.set_rotation_heading_pitch_roll_raw([heading, pitch, roll]);
    }
    if entry_flags & 0x4000 == 0 {
        entity.physical_body_basis_q31 =
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    }
    let current_flags = effective_flags(entity)?;
    let mut position = entity.position_raw();
    let mut velocity = entity.velocity_raw();
    apply_common_gravity_and_underwater_raw(
        &mut velocity,
        dt,
        CommonUnderwaterFrame {
            effective_environment_flags: current_flags,
            water_response_enabled: true,
            position_y_raw: position[1],
            solid_or_sea_y_raw: common_environment_surface_raw(terrain, position[0], position[2]),
            self_mass_raw: entity.mass_raw,
            attached_cargo_mass: 0,
        },
    );
    apply_common_no_wind_drag_raw(
        &mut velocity,
        dt,
        current_flags,
        CommonEnvironmentPhysics {
            runtime_wind_mode: environment.0,
            drag_strength: environment.1,
            ..CommonEnvironmentPhysics::default()
        },
        entity.mass_raw,
    );
    if entry_flags & 2 != 0 {
        // Every admitted post-task mask has40 clear: DF70 has no model-radius suffix.
        if current_flags & 0x40 != 0 {
            return Err(Block::Runtime("cleansing snap extent"));
        }
        let mut state = 0;
        apply_type9_ground_snap_raw(&mut position, &mut velocity, &mut state, terrain);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x800000, 0x800000);
        entity.set_position_raw(position);
    }
    entity.set_velocity_raw(velocity);
    if entry_flags & 0x800 != 0 {
        return Err(Block::Runtime("cleansing hot-terrain damage"));
    }
    // Type49 authors zero surface selectors; E370 only decays its timer.
    let RetailRuntimeValue::Known(effects) = metadata.common_world_effects else {
        return Err(Block::Metadata);
    };
    if effects.surface_selectors != [0, 0] {
        return Err(Block::Metadata);
    }
    if bits(entity, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT)? == 0 {
        let RetailRuntimeValue::Known(timer) = entity.surface_lifetime_timer_ms_at_0x48 else {
            return Err(Block::Runtime("cleansing surface timer"));
        };
        entity.surface_lifetime_timer_ms_at_0x48 =
            RetailRuntimeValue::Known(decay_actor_surface_timer_ms(timer, dt));
    }
    super::commit_motion(entity, dt)
}
