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
    // Follow/Capture, Class5 row4C7930, Furniture26 row4C7738 and
    // Buildings15 row4C76F0 have zero +34/+38 masks; Search clears21080/80. These source masks
    // preserve each admitted profile's own authored C8. Class5 row4C7978
    // instead sets80/clears2 and is not this ordinary living task graph.
    let carrying = P::CAPTURE_POLICY == NativeCapturePolicy::Transport
        && crate::native_actor_capture::carry_tasks::carrying_variant(entity).is_some();
    // Wander Near Location 4C79C0 clears 21000; both Guard styles are zero.
    if context.active_style().style_address() == 0x4c79c0 {
        return Ok(P::DEFAULT_FLAGS & !0x21000);
    }
    // Class29's Gun Turret 4C8230 sets 4027: no gravity, a kept basis and
    // DF70's grounding.
    if context.active_style().style_address() == 0x4c8230 {
        return Ok(P::DEFAULT_FLAGS | 0x4027);
    }
    if !carrying
        && !matches!(
            context.active_style().style_address(),
            0x4c7bb8
                | 0x4c7c00
                | 0x4c7b28
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
                | 0x4c76f0
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
        // Sub-C `+0C`: mode1 probes take the waved sea over water.
        resolved_surface_mode_raw: sub_c.surface_mode_raw,
        water_enabled: waves_enabled,
        wave_tick_50hz: frame.retail_tick as i32,
        effective_elapsed_micros: dt,
    });
    entity.set_rotation_heading_pitch_roll_raw([heading, attitude.pitch_raw, attitude.roll_raw]);
    // 13F70 rebuilds the matrix unless the effective policy keeps it (4000).
    let basis = if effective_flags & 0x4000 == 0 {
        let rebuilt = attitude.rebuild_body_basis(heading);
        entity.physical_body_basis_q31 = RetailRuntimeValue::Known(rebuilt);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
        rebuilt
    } else {
        basis
    };
    let mut velocity = entity.velocity_raw();
    // 31/39/439/431 have gravity and no ground-snap bit2. E100's water
    // response reads Sub-C +0C against the solid-or-sea cell height.
    let [x, _, z] = entity.position_raw();
    apply_common_gravity_and_underwater_raw(
        &mut velocity,
        dt,
        CommonUnderwaterFrame {
            effective_environment_flags: effective_flags,
            water_response_enabled: sub_c.surface_mode_raw != 0,
            position_y_raw: entity.position_raw()[1],
            solid_or_sea_y_raw: crate::entity::common_environment_surface_raw(terrain, x, z),
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
                basis,
                callback_mass_raw: std::num::NonZeroU16::new(entity.mass_raw)
                    .ok_or(Block::Runtime("zero environment mass"))?,
                elapsed_micros: dt,
            },
        );
    }
    entity.set_velocity_raw(velocity);
    entity.set_rotation_heading_pitch_roll_raw(angles);
    // DF70 grounds the body at its current X/Z (effective bit 2).
    if effective_flags & 2 != 0 {
        let mut position = entity.position_raw();
        let mut velocity = entity.velocity_raw();
        let mut grounded = 0;
        crate::common_mover::type9_tail::apply_type9_ground_snap_raw(
            &mut position,
            &mut velocity,
            &mut grounded,
            terrain,
        );
        entity.set_position_raw(position);
        entity.set_velocity_raw(velocity);
        entity.collision.state_flags_at_0x08.overwrite(
            crate::common_mover::type9_tail::MASTER_GROUNDED_STATE_BIT,
            grounded,
        );
    }
    let surface_frame = crate::intro2_common_dying::Intro2ActorSurfaceFrame {
        metadata,
        terrain,
        active_model_extent_raw: model.radius,
        elapsed_micros: dt,
        retail_tick: frame.retail_tick,
        particle_environment: crate::world_fx::ParticleEnvironment::Terrain(
            crate::world_fx::TerrainCollisionContext::from_current_level_cache(&frame.resources)
                .ok_or(Block::Runtime("particle terrain context"))?,
        ),
    };
    if frame.terminal.is_some() {
        if !matches!(
            frame.capture,
            NativeCaptureDispatch::NoCapture | NativeCaptureDispatch::Turret { .. }
        ) || !P::manager_authenticates(manager, id)
        {
            return Err(Block::Allocation);
        }
        let extent = surface_frame.active_model_extent_raw;
        let head = crate::intro2_common_dying::run_actor_surface_head(
            manager,
            id,
            &surface_frame,
            P::allocation_authenticates,
        )
        .map_err(Block::Surface)?;
        run_lent_terminal_surface::<P>(manager, id, metadata, extent, head, frame, death)?;
    } else {
        run_surface::<P>(
            manager,
            id,
            surface_frame,
            frame.world_fx,
            death,
            &mut frame.capture,
            &frame.resources,
        )?;
    }
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

/// E370 with the lent world, after its timer write: `162B0` runs `16750`,
/// then `10C10`'s terminal (`BAC0`/`BC90`, `BAF0`'s radial) inside the tick,
/// before the bubble and sound tail, which meets the terrain that radial may
/// have cratered.
fn run_lent_terminal_surface<P: NativeGroundActorProfile>(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    extent: u16,
    head: crate::intro2_common_dying::ActorSurfaceHead,
    frame: &mut NativeGroundActorFrame<'_>,
    death: &mut Option<NativeGroundTerminalPublication>,
) -> Result<(), NativeGroundActorBlock> {
    use crate::intro2_common_dying::{
        release_surface_relation, run_actor_surface_tail, ActorSurfaceHead,
    };
    use NativeGroundActorBlock as Block;
    let remaining = match head {
        ActorSurfaceHead::Tail(remaining) => remaining,
        ActorSurfaceHead::Lifecycle(after) => {
            release_surface_relation(manager, id, metadata).map_err(Block::Surface)?;
            let (NativeGroundResources::Mutable(resources), Some(lent)) =
                (&mut frame.resources, frame.terminal.as_mut())
            else {
                return Err(Block::Runtime("E370 terminal death without the lent world"));
            };
            let player = match &mut lent.world {
                crate::specialized_actor_task_production::SpecializedActorTaskWorld::Playing {
                    player_hull,
                    extra_lives,
                } => Some(crate::native_actor_capture::pair::PlayingPlayerContact {
                    hull: player_hull,
                    extra_lives: *extra_lives,
                }),
                crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic => {
                    None
                }
            };
            *death = P::publish_direct_surface_death(
                manager,
                id,
                &mut NativeGroundDeathContext::Terminal {
                    resources,
                    fx: frame.world_fx,
                    static_damage: lent.static_damage,
                    notifications: lent.notifications,
                    retail_tick: frame.retail_tick,
                    tasks: lent.scheduler,
                    player,
                },
            )
            .map_err(Block::Surface)?
            .publication;
            (after < 75).then_some(after)
        }
    };
    let context =
        crate::world_fx::TerrainCollisionContext::from_current_level_cache(&frame.resources)
            .ok_or(Block::Runtime("particle terrain context"))?;
    run_actor_surface_tail(
        manager,
        id,
        remaining,
        extent,
        crate::world_fx::ParticleEnvironment::Terrain(context),
        frame.retail_tick,
        frame.world_fx,
    )
    .map_err(Block::Surface)
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
        NativeCaptureDispatch::NoCapture
            | NativeCaptureDispatch::PursuitOnly
            | NativeCaptureDispatch::Turret { .. }
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
