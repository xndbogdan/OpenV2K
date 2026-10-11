//! Post-task DCA0/E870 phases and the enclosing 12DA0 motion suffix.

use super::*;
use crate::{
    common_mover::{environment::*, type9_attitude::*},
    entity::{apply_common_gravity_and_underwater_raw, CommonUnderwaterFrame, EntityManager},
    entity_collision_state::BODY_BASIS_REBUILT_STATE_BIT,
    intro2_common_dying::Intro2ActorSurfaceFrame,
};

pub(crate) fn effective_flags<P: NativeGroundActorProfile>(
    entity: &Entity,
) -> Result<u32, NativeGroundActorBlock> {
    use NativeGroundActorBlock as Block;
    if entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(P::DEFAULT_FLAGS) {
        return Err(Block::Runtime("default flags"));
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Graph);
    };
    // Follow/Capture, Class5 row4C7930 and Furniture26 row4C7738 have
    // zero +34/+38 masks; Search clears21080/80. These source masks
    // preserve each admitted profile's own authored C8. Class5 row4C7978
    // instead sets80/clears2 and is not this ordinary living task graph.
    let carrying = P::CAPTURE_POLICY == NativeCapturePolicy::Transport
        && crate::native_actor_capture::carry_tasks::carrying_variant(entity).is_some();
    if !carrying
        && !matches!(
            context.active_style().style_address(),
            0x4c7b28
                | 0x4c7b70
                | 0x4c7ff0
                | 0x4c8038
                | 0x4c7a50
                | 0x4c7a98
                | 0x4c7e88
                | 0x4c7618
                | 0x4c7660
                | 0x4c7930
                | 0x4c7738
        )
    {
        return Err(Block::Runtime("effective style flags"));
    }
    Ok(P::DEFAULT_FLAGS)
}

pub(crate) fn finish<P: NativeGroundActorProfile>(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    frame: &mut NativeGroundActorFrame<'_>,
    dt: u32,
    entry_state: u32,
    effective_flags: u32,
    death: &mut Option<NativeGroundTerminalPublication>,
) -> Result<(), NativeGroundActorBlock> {
    use NativeGroundActorBlock as Block;
    let environment = CommonWindDrag::from_level(
        frame
            .resources
            .level_desc()
            .ok_or(Block::Runtime("world wind descriptor"))?,
        manager.common_environment_physics(),
    )
    .map_err(Block::Runtime)?;
    let waves_enabled = manager.common_environment_physics().waves_enabled;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    if entry_state & crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0 {
        let record = frame
            .resources
            .global_entity_type(P::ENTITY_TYPE as usize)
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
    // 39/439/431 have gravity, no ground-snap bit2, and C+0c is terrain-only.
    apply_common_gravity_and_underwater_raw(
        &mut velocity,
        dt,
        CommonUnderwaterFrame {
            effective_environment_flags: effective_flags,
            water_response_enabled: false,
            position_y_raw: entity.position_raw()[1],
            solid_or_sea_y_raw: 0,
            self_mass_raw: entity.mass_raw,
            attached_cargo_mass: 0,
        },
    );
    let mut angles = entity.rotation_heading_pitch_roll_raw();
    // 40E354 calls 44EC60 only for effective bit8: 39/439 drag, 431 does not.
    if effective_flags & 0x08 != 0 {
        apply_common_wind_drag_raw(
            &mut velocity,
            &mut angles,
            environment,
            CommonWindDragFrame {
                terrain,
                position_raw: entity.position_raw(),
                basis: attitude.rebuild_body_basis(heading),
                callback_mass_raw: std::num::NonZeroU16::new(entity.mass_raw)
                    .ok_or(Block::Runtime("zero environment mass"))?,
                elapsed_micros: dt,
            },
        );
    }
    entity.set_velocity_raw(velocity);
    entity.set_rotation_heading_pitch_roll_raw(angles);
    run_surface::<P>(
        manager,
        id,
        crate::intro2_common_dying::Intro2ActorSurfaceFrame {
            metadata,
            terrain,
            active_model_extent_raw: model.radius,
            elapsed_micros: dt,
            retail_tick: frame.retail_tick,
            particle_environment: crate::world_fx::ParticleEnvironment::Terrain(
                crate::world_fx::TerrainCollisionContext::from_current_level_cache(
                    &frame.resources,
                )
                .ok_or(Block::Runtime("particle terrain context"))?,
            ),
        },
        frame.world_fx,
        death,
        &mut frame.capture,
        &frame.resources,
    )?;
    // 413135..413149 runs18640 with the rebuilt basis and pre-integration
    // parent origin, after the complete type callback and before12DA0 motion.
    P::update_attachment(manager, id)?;
    let entity = manager.entity_mut(id).ok_or(Block::Allocation)?;
    super::live::commit_motion(entity, dt)?;
    // 12DA0 returns after master motion. 44FFA0 later calls 11A80, whose
    // intrusive walk owns 11AD0 terrain/static/pair contacts after particles.
    // A missing contact adapter must not freeze this earlier actor callback.
    Ok(())
}

fn run_surface<P: NativeGroundActorProfile>(
    manager: &mut EntityManager,
    id: u32,
    frame: Intro2ActorSurfaceFrame<'_>,
    fx: &mut WorldFx,
    death: &mut Option<NativeGroundTerminalPublication>,
    capture: &mut NativeCaptureDispatch<'_>,
    resources: &crate::resource_cache::ResourceCache,
) -> Result<(), NativeGroundActorBlock> {
    if matches!(
        capture,
        NativeCaptureDispatch::NoCapture | NativeCaptureDispatch::PursuitOnly
    ) {
        if !P::manager_authenticates(manager, id) {
            return Err(NativeGroundActorBlock::Allocation);
        }
        return crate::intro2_common_dying::run_actor_surface_with_death(
            manager,
            id,
            frame,
            fx,
            death,
            P::allocation_authenticates,
            |manager, id, fx| {
                P::publish_direct_surface_death(
                    manager,
                    id,
                    &mut NativeGroundDeathContext::Basic { resources, fx },
                )
                .map(|result| result.publication)
            },
        )
        .map_err(NativeGroundActorBlock::Surface);
    }
    match capture {
        NativeCaptureDispatch::Transport {
            tasks,
            notifications,
        } => {
            let tick = frame.retail_tick;
            crate::intro2_common_dying::run_actor_surface_with_death(manager, id, frame, fx, death,
                P::allocation_authenticates, |manager, id, world_fx| P::publish_standard_death(manager, id,
                    &mut NativeGroundDeathContext::Capture { resources,
                        context: crate::native_actor_capture::CaptureContext {
                            resources: Some(resources),
                            tasks: &mut **tasks, world_fx, notifications: &mut **notifications, retail_tick: tick,
                            result_screen: crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
                            hive_dying: Default::default(),
                        }
                    }).map(|result| result.publication))
                .map_err(NativeGroundActorBlock::Surface)
        }
        NativeCaptureDispatch::AbsentJ { tasks, .. } => {
            let tick = frame.retail_tick;
            crate::intro2_common_dying::run_actor_surface_with_death(
                manager,
                id,
                frame,
                fx,
                death,
                P::allocation_authenticates,
                |manager, id, fx| {
                    P::publish_standard_death(
                        manager,
                        id,
                        &mut NativeGroundDeathContext::Split {
                            resources,
                            fx,
                            tick,
                            tasks: &mut **tasks,
                        },
                    )
                    .map(|result| result.publication)
                },
            )
            .map_err(NativeGroundActorBlock::Surface)
        }
        _ => Err(NativeGroundActorBlock::Runtime(
            "native ground surface task custody",
        )),
    }
}
