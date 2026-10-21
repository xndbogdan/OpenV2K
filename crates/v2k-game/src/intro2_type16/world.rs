//! Post-task DCA0/E870 phases and the enclosing 12DA0 motion suffix.

use super::*;
use crate::{
    common_mover::type9_attitude::*,
    entity::{apply_type13_common_environment_raw, EntityManager},
    entity_collision_state::BODY_BASIS_REBUILT_STATE_BIT,
};

pub(super) fn effective_flags(entity: &Entity) -> Result<u32, Intro2Type16Block> {
    use Intro2Type16Block as Block;
    if entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(0x439) {
        return Err(Block::Runtime("default flags"));
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    // Defecate/Aimless/Capture (carrying included) have zero masks. Search
    // clears21080 or80; all admitted styles leave the authored C8=439 unchanged.
    if !matches!(
        context.active_style().style_address(),
        0x4c7e88
            | 0x4c7930
            | 0x4c7a50
            | 0x4c7a98
            | 0x4c7ff0
            | 0x4c8038
            | 0x4c8080
            | 0x4c80c8
            | 0x4c8110
            | 0x4c8158
    ) {
        return Err(Block::Runtime("effective style flags"));
    }
    Ok(0x439)
}

pub(super) fn finish(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    frame: &mut Intro2Type16Frame<'_>,
    dt: u32,
    entry_state: u32,
    effective_flags: u32,
    death: &mut Option<crate::intro2_common_dying::Intro2CommonDyingOwner>,
) -> Result<(), Intro2Type16Block> {
    use Intro2Type16Block as Block;
    let environment = manager.intro2_type13_environment();
    let waves_enabled = manager.common_environment_physics().waves_enabled;
    if environment.0 != 0 {
        return Err(Block::Runtime("wind mode"));
    }
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    if entry_state & crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0 {
        let record = frame
            .resources
            .global_entity_type(16)
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
                .queue_networked_positional_sound_raw(sound, entity.position_raw());
        }
    }
    let terrain = frame
        .resources
        .level_terrain()
        .ok_or(Block::Runtime("terrain"))?;
    let model = entity
        .model_index
        .and_then(|id| frame.resources.global_model(id))
        .ok_or(Block::Runtime("active model"))?;
    let RetailRuntimeValue::Known(Some(sub_c)) = metadata.sub_c_lift_descriptor else {
        return Err(Block::Metadata);
    };
    if sub_c.surface_mode_raw != 0 {
        return Err(Block::Runtime("Sub-C surface mode"));
    }
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(Block::Runtime("physical basis"));
    };
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    let attitude = plan_type9_terrain_attitude_raw(Type9TerrainAttitudeInput {
        terrain,
        position_raw: entity.position_raw(),
        pitch_raw: pitch,
        roll_raw: roll,
        lateral_basis_q31: basis.lateral,
        forward_basis_q31: basis.forward,
        state_flags: super::live::bits(entity, TERRAIN_ATTITUDE_BILINEAR_STATE_BIT)?,
        effective_flags,
        active_model_extent_raw: model.radius,
        resolved_surface_mode_raw: 0,
        water_enabled: waves_enabled,
        wave_tick_50hz: frame.retail_tick as i32,
        effective_elapsed_micros: dt,
    });
    entity.set_rotation_heading_pitch_roll_raw([heading, attitude.pitch_raw, attitude.roll_raw]);
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(attitude.rebuild_body_basis(heading));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    let mut velocity = entity.velocity_raw();
    // 439 has gravity/drag, no ground-snap bit2, and C+0c is terrain-only.
    apply_type13_common_environment_raw(
        &mut velocity,
        dt,
        entity.mass_raw,
        environment.0,
        environment.1,
    );
    entity.set_velocity_raw(velocity);
    let surface_frame = crate::intro2_common_dying::Intro2ActorSurfaceFrame {
        metadata,
        terrain,
        active_model_extent_raw: model.radius,
        elapsed_micros: dt,
        retail_tick: frame.retail_tick,
        particle_environment: crate::world_fx::ParticleEnvironment::Terrain(
            crate::world_fx::TerrainCollisionContext::from_current_level_cache(frame.resources)
                .ok_or(Block::Runtime("particle terrain context"))?,
        ),
    };
    if super::type16_row(manager.iter_all().find(|entity| entity.id == id).unwrap())
        == Some(super::Type16Row::Type128)
    {
        // Type128's class63 drowning needs BAF0's lent world; the class12
        // publisher rejects its alternate and this step holds.
        crate::intro2_common_dying::run_living_actor_surface(
            manager,
            id,
            surface_frame,
            frame.world_fx,
            death,
        )
        .map_err(Block::Surface)?;
    } else {
        let mut capture_error = None;
        let tick = frame.retail_tick;
        let surface = crate::intro2_common_dying::run_actor_surface_with_death(
            manager,
            id,
            surface_frame,
            frame.world_fx,
            death,
            super::intro2_type16_allocation_authenticates,
            |manager, id, world_fx| {
                // 162B0's direct 10C10 enters DB80: a carrying style's D040
                // releases the child before the nested and outer C620.
                crate::native_actor_capture::publish_native_captor_standard_death(
                    manager,
                    id,
                    &mut crate::native_actor_capture::CaptureContext {
                        resources: None,
                        tasks: &mut *frame.capture_tasks,
                        world_fx,
                        notifications: &mut *frame.notifications,
                        retail_tick: tick,
                        result_screen:
                            crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
                        hive_dying: Default::default(),
                    },
                )
                .map_err(|error| {
                    capture_error = Some(error);
                    crate::intro2_common_dying::Intro2CommonDyingBlock::Runtime(
                        "Capture surface death",
                    )
                })
            },
        );
        if let Some(error) = capture_error {
            return Err(Block::Capture(error));
        }
        surface.map_err(Block::Surface)?;
    }
    // 413135..413149 invokes Sub-J after the complete type callback and
    // before the parent's master integration, at the rebuilt basis.
    crate::native_actor_capture::update_carried_pose(manager, id).map_err(Block::Capture)?;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    super::live::commit_motion(entity, dt)?;
    // 12DA0 returns after master motion. 44FFA0 later calls 11A80, whose
    // intrusive walk owns 11AD0 terrain/static/pair contacts after particles.
    // A missing contact adapter must not freeze this earlier actor callback.
    Ok(())
}
