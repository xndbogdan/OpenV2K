//! Post-task DCA0/E870 phases and the enclosing 12DA0 motion suffix.

use super::*;
use crate::{
    common_mover::type9_attitude::*,
    entity::{apply_type13_common_environment_raw, EntityManager},
    entity_collision_state::BODY_BASIS_REBUILT_STATE_BIT,
};

pub(super) fn finish(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    frame: &mut Intro2Type17Frame<'_>,
    dt: u32,
    entry_state: u32,
    death: &mut Option<crate::intro2_common_dying::Intro2CommonDyingOwner>,
) -> Result<(), Intro2Type17Block> {
    use Intro2Type17Block as Block;
    let environment = manager.intro2_type13_environment();
    if environment.0 != 0 {
        return Err(Block::Runtime("wind mode"));
    }
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    if entry_state & crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0 {
        let record = frame
            .resources
            .global_entity_type(17)
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
    if entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(0x39) {
        return Err(Block::Runtime("default flags"));
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    // Exact executable +34/+38 masks are zero for these Follow, Capture,
    // and Run Away styles. Each leaves authored C8=39 unchanged.
    if !matches!(
        context.active_style().style_address(),
        0x4c7b28
            | 0x4c7b70
            | 0x4c7ff0
            | 0x4c8038
            | 0x4c8080
            | 0x4c80c8
            | 0x4c8110
            | 0x4c8158
            | 0x4c7618
            | 0x4c7660
    ) {
        return Err(Block::Runtime("effective style flags"));
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
        state_flags: entry_state,
        effective_flags: 0x39,
        active_model_extent_raw: model.radius,
        resolved_surface_mode_raw: 0,
        water_enabled: frame
            .resources
            .level_desc()
            .and_then(|level| level.raw_u32(0x84))
            .ok_or(Block::Runtime("level wave flag"))?
            != 0,
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
    // 39 has gravity/drag, no ground-snap bit2, and C+0c is terrain-only.
    apply_type13_common_environment_raw(
        &mut velocity,
        dt,
        entity.mass_raw,
        environment.0,
        environment.1,
    );
    entity.set_velocity_raw(velocity);
    let mut capture_error = None;
    let surface = crate::intro2_common_dying::run_actor_surface_with_death(
        manager,
        id,
        crate::intro2_common_dying::Intro2ActorSurfaceFrame {
            metadata,
            terrain,
            active_model_extent_raw: model.radius,
            elapsed_micros: dt,
            retail_tick: frame.retail_tick,
            particle_environment: crate::world_fx::ParticleEnvironment::Terrain(
                crate::world_fx::TerrainCollisionContext::from_current_level_cache(frame.resources)
                    .ok_or(Block::Runtime("particle terrain context"))?,
            ),
        },
        frame.world_fx,
        death,
        super::intro2_type17_allocation_authenticates,
        |manager, id, world_fx| {
            // 162B0's direct 10C10 enters the same DB80 cleanup as radial
            // death. Child release precedes the nested and outer C620 tasks.
            super::capture::publish_type17_standard_death(
                manager,
                id,
                &mut super::capture::CaptureContext {
                    tasks: &mut *frame.capture_tasks,
                    world_fx,
                    notifications: &mut *frame.notifications,
                    retail_tick: frame.retail_tick,
                    result_screen:
                        crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
                    hive_dying: Default::default(),
                },
            )
            .map_err(|error| {
                capture_error = Some(error);
                crate::intro2_common_dying::Intro2CommonDyingBlock::Runtime("Capture surface death")
            })
        },
    );
    if let Some(error) = capture_error {
        return Err(Block::Capture(error));
    }
    surface.map_err(Block::Surface)?;
    // 413135..413149 invokes Sub-J after the complete type callback and
    // before the parent's master integration. Child positions use that
    // pre-integration origin and the newly rebuilt basis.
    crate::native_actor_capture::update_carried_pose(manager, id).map_err(Block::Capture)?;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    super::live::commit_motion(entity, dt)?;
    // 12DA0 returns after master motion. 44FFA0 later calls 11A80, whose
    // intrusive walk owns 11AD0 terrain/static/pair contacts after particles.
    // A missing contact adapter must not freeze this earlier actor callback.
    Ok(())
}

#[cfg(test)]
mod capture_surface_tests;
