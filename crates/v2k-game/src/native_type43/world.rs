//! DCA0/E870 after the task walk: detail sound, E640, 413F70, E100, DF70 and
//! E370. The enclosing 12DA0 then integrates X/Z: Search cleared 0x08000000.

use super::{live::Type43Block, *};
use crate::{
    common_mover::{
        environment::{apply_common_wind_drag_raw, CommonWindDrag, CommonWindDragFrame},
        type9_attitude::{
            plan_type9_terrain_attitude_raw, Type9TerrainAttitudeInput,
            TERRAIN_ATTITUDE_BILINEAR_STATE_BIT, TERRAIN_ATTITUDE_EFFECTIVE_FLAG,
        },
        type9_surface::{decay_actor_surface_timer_ms, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT},
        type9_tail::{apply_type9_ground_snap_raw, MASTER_GROUNDED_STATE_BIT},
    },
    entity::{apply_common_gravity_and_underwater_raw, CommonUnderwaterFrame},
    entity_collision_state::{RetailRuntimeValue, BODY_BASIS_REBUILT_STATE_BIT},
    resource_cache::ResourceCache,
    world_fx::WorldFx,
};

/// DCA0/E870 read `(C8 | style+34) & !style+38` once, before the task walk.
/// Both class7 styles have a zero +34; Search 4C7A50's +38 is 0x21080 and
/// Pursuit 4C7A98's is 0x80. Neither result has 0x1000/0x2000 (motion and
/// coarse skip), 0x0800 (terrain material), 0x4000 (keep basis) or 0x40.
pub(super) fn effective_flags(entity: &Entity) -> Result<u32, Type43Block> {
    if entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(DEFAULT_FLAGS) {
        return Err(Type43Block::Runtime("default flags"));
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Type43Block::Graph);
    };
    match context.active_style().style_address() {
        0x004c_7a50 => Ok(DEFAULT_FLAGS & !0x0002_1080),
        0x004c_7a98 => Ok(DEFAULT_FLAGS & !0x0000_0080),
        _ => Err(Type43Block::Runtime("effective style flags")),
    }
}

pub(super) struct FinishFrame<'a> {
    pub resources: &'a ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub retail_tick: u32,
    pub detailed: bool,
}

pub(super) fn finish(
    manager: &mut EntityManager,
    id: u32,
    frame: FinishFrame<'_>,
    dt: u32,
    entry_flags: u32,
) -> Result<(), Type43Block> {
    use Type43Block as Block;
    let wind = CommonWindDrag::from_level(
        frame
            .resources
            .level_desc()
            .ok_or(Block::Runtime("world wind descriptor"))?,
        manager.common_environment_physics(),
    )
    .map_err(Block::Runtime)?;
    let waves_enabled = manager.common_environment_physics().waves_enabled;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    if frame.detailed {
        let record = frame
            .resources
            .global_entity_type(ENTITY_TYPE as usize)
            .ok_or(Block::Metadata)?;
        let RetailRuntimeValue::Known(health) = entity.collision.health_raw else {
            return Err(Block::Runtime("detailed sound health"));
        };
        let visible = super::live::bits(entity, 0x800)? != 0;
        if let Some(sound) = crate::actor_detailed_sound::plan_actor_detailed_sound(
            crate::actor_detailed_sound::ActorDetailedSoundFrame {
                type_record: record,
                health_raw: health,
                visible,
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
        .ok_or(Block::Runtime("terrain"))?;
    let model = entity
        .model_index
        .and_then(|model| frame.resources.global_model(model))
        .ok_or(Block::Runtime("active model"))?;
    let [heading, mut pitch, mut roll] = entity.rotation_heading_pitch_roll_raw();
    if entry_flags & TERRAIN_ATTITUDE_EFFECTIVE_FLAG != 0 {
        let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
            return Err(Block::Runtime("physical basis"));
        };
        // E640 reads Sub-C+0C only when a C descriptor exists: terrain probes.
        let attitude = plan_type9_terrain_attitude_raw(Type9TerrainAttitudeInput {
            terrain,
            position_raw: entity.position_raw(),
            pitch_raw: pitch,
            roll_raw: roll,
            lateral_basis_q31: basis.lateral,
            forward_basis_q31: basis.forward,
            state_flags: super::live::bits(entity, TERRAIN_ATTITUDE_BILINEAR_STATE_BIT)?,
            effective_flags: entry_flags,
            active_model_extent_raw: model.radius,
            resolved_surface_mode_raw: 0,
            water_enabled: waves_enabled,
            wave_tick_50hz: frame.retail_tick as i32,
            effective_elapsed_micros: dt,
        });
        pitch = attitude.pitch_raw;
        roll = attitude.roll_raw;
        entity.set_rotation_heading_pitch_roll_raw([heading, pitch, roll]);
    }
    // 413F70: neither effective word has 0x4000.
    let basis =
        crate::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(heading, pitch, roll);
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(basis);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    // DCA0's E640/413F70/DF70 gates keep its entry word, but E100 and DF70
    // re-read the current style's masks after the walk. Search and Pursuit
    // differ only in 0x20000, which neither reads.
    let current_flags = effective_flags(entity)?;
    // E100: gravity (no 0x04); the water response reads Sub-C+0C, absent.
    let mut velocity = entity.velocity_raw();
    apply_common_gravity_and_underwater_raw(
        &mut velocity,
        dt,
        CommonUnderwaterFrame {
            effective_environment_flags: current_flags,
            water_response_enabled: false,
            position_y_raw: entity.position_raw()[1],
            solid_or_sea_y_raw: 0,
            self_mass_raw: entity.mass_raw,
            attached_cargo_mass: 0,
        },
    );
    let mut angles = entity.rotation_heading_pitch_roll_raw();
    if current_flags & 0x08 != 0 {
        apply_common_wind_drag_raw(
            &mut velocity,
            &mut angles,
            wind,
            CommonWindDragFrame {
                terrain,
                position_raw: entity.position_raw(),
                basis,
                callback_mass_raw: std::num::NonZeroU16::new(entity.mass_raw)
                    .ok_or(Block::Runtime("zero environment mass"))?,
                elapsed_micros: dt,
            },
        );
    }
    entity.set_velocity_raw(velocity);
    entity.set_rotation_heading_pitch_roll_raw(angles);
    // DF70 at the current X/Z. Its own re-read 0x40 would add the model
    // origin; neither class7 style sets it.
    if entry_flags & 0x02 != 0 {
        if current_flags & 0x40 != 0 {
            return Err(Block::Runtime("DF70 model origin"));
        }
        let mut position = entity.position_raw();
        let mut velocity = entity.velocity_raw();
        let mut grounded = 0;
        apply_type9_ground_snap_raw(&mut position, &mut velocity, &mut grounded, terrain);
        entity.set_position_raw(position);
        entity.set_velocity_raw(velocity);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(MASTER_GROUNDED_STATE_BIT, grounded);
    }
    // E370 with both +72/+73 selectors zero only decays the timer.
    if super::live::bits(entity, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT)? == 0 {
        let RetailRuntimeValue::Known(timer) = entity.surface_lifetime_timer_ms_at_0x48 else {
            return Err(Block::Runtime("surface timer"));
        };
        entity.surface_lifetime_timer_ms_at_0x48 =
            RetailRuntimeValue::Known(decay_actor_surface_timer_ms(timer, dt));
    }
    Ok(())
}
