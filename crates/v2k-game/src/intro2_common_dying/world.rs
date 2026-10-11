use super::*;
use crate::common_dying::{
    common_dying_after_unwind, damp_common_dying_axis, CommonDyingAfterUnwindOutcome,
    CommonDyingCallbackResult, CommonDyingTaggedResult, COMMON_DYING_OWNER_TRANSITION_TAG,
    COMMON_DYING_SCHEDULER_SINGLETON_ADDRESS,
};
use crate::common_mover::environment::*;
use crate::common_mover::type9_attitude::{
    plan_common_dying_terrain_attitude_raw, CommonDyingTerrainAttitudeInput, Type9BodyBasis,
};
use crate::common_mover::type9_surface::*;
use crate::common_mover::type9_tail::{
    plan_common_master_motion, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
};
use crate::entity::{
    apply_common_gravity_and_underwater_raw, commit_common_master_motion, CommonUnderwaterFrame,
};
use crate::entity_collision_state::{
    ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT,
};
use crate::entity_relation_release::relation_release_state_word_after;
use crate::entity_scheduler::*;
use crate::resource_cache::ResourceCache;
use crate::world_fx::{ParticleEnvironment, TerrainCollisionContext};

pub struct Intro2CommonDyingFrame<'a> {
    pub resources: &'a ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2CommonDyingOutcome {
    Waiting {
        entity_id: u32,
    },
    Advanced {
        entity_id: u32,
        detailed: bool,
        callback_elapsed_micros: u32,
        terminal: bool,
    },
    Blocked {
        entity_id: u32,
        reason: Intro2CommonDyingBlock,
        prefix_committed: bool,
    },
    Pending {
        entity_id: u32,
    },
    Dropped {
        entity_id: u32,
    },
}
impl Intro2CommonDyingOutcome {
    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::Waiting { entity_id }
            | Self::Advanced { entity_id, .. }
            | Self::Blocked { entity_id, .. }
            | Self::Pending { entity_id }
            | Self::Dropped { entity_id } => *entity_id,
        }
    }
}
pub struct Intro2CommonDyingTick {
    pub outcome: Intro2CommonDyingOutcome,
    pub retained_owner: Option<Intro2CommonDyingOwner>,
}

pub fn tick_intro2_common_dying(
    manager: &mut EntityManager,
    mut owner: Intro2CommonDyingOwner,
    frame: Intro2CommonDyingFrame<'_>,
) -> Intro2CommonDyingTick {
    let id = owner.entity_id();
    if manager.iter_all().any(|entity| {
        entity.id == id
            && entity.active
            && (entity.intro2_type17_runtime.is_some()
                || entity.native_type47_construction.is_some())
    }) && !native_manager_allocation(manager, id)
    {
        return Intro2CommonDyingTick {
            outcome: Intro2CommonDyingOutcome::Blocked {
                entity_id: id,
                reason: Intro2CommonDyingBlock::UnauthenticatedAllocation,
                prefix_committed: false,
            },
            retained_owner: None,
        };
    }
    if !native_manager_allocation(manager, id)
        || manager
            .main_base_abort_actor_observation(id)
            .map(|observation| observation.lease)
            != Some(owner.allocation)
    {
        return Intro2CommonDyingTick {
            outcome: Intro2CommonDyingOutcome::Dropped { entity_id: id },
            retained_owner: None,
        };
    }
    let current = Intro2CommonDyingOwner::adopt(manager, id).ok();
    if !current.is_some_and(|current| current.visit == owner.visit) {
        return Intro2CommonDyingTick {
            outcome: Intro2CommonDyingOutcome::Dropped { entity_id: id },
            retained_owner: None,
        };
    }
    if owner.pending {
        return Intro2CommonDyingTick {
            outcome: Intro2CommonDyingOutcome::Pending { entity_id: id },
            retained_owner: Some(owner),
        };
    }
    let mut prefix_committed = false;
    match run_frame(manager, owner, frame, &mut prefix_committed) {
        Ok(outcome) => {
            let terminal = matches!(
                outcome,
                Intro2CommonDyingOutcome::Advanced { terminal: true, .. }
            );
            Intro2CommonDyingTick {
                outcome,
                retained_owner: (!terminal).then_some(owner),
            }
        }
        Err(reason) => {
            owner.pending = prefix_committed;
            Intro2CommonDyingTick {
                outcome: Intro2CommonDyingOutcome::Blocked {
                    entity_id: id,
                    reason,
                    prefix_committed,
                },
                retained_owner: Some(owner),
            }
        }
    }
}

fn bits(entity: &Entity, mask: u32, label: &'static str) -> Result<u32, Intro2CommonDyingBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        _ => Err(Intro2CommonDyingBlock::Runtime(label)),
    }
}

fn run_frame(
    manager: &mut EntityManager,
    owner: Intro2CommonDyingOwner,
    frame: Intro2CommonDyingFrame<'_>,
    prefix_committed: &mut bool,
) -> Result<Intro2CommonDyingOutcome, Intro2CommonDyingBlock> {
    use Intro2CommonDyingBlock as Block;
    let id = owner.entity_id();
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .cloned()
        .ok_or(Block::Metadata("type"))?;
    authenticate_components(entity, &metadata)?;
    let terrain = frame
        .resources
        .level_terrain()
        .ok_or(Block::Metadata("terrain"))?;
    // 01280/E640 reads global4FECE4 from the active descriptor's +84.
    // Terrain water geometry and authored wave animation are independent.
    let wave_clock = crate::common_mover::sub_c::SubCWaveClock {
        wave_tick_50hz: frame.retail_tick as i32,
        waves_enabled: frame
            .resources
            .level_desc()
            .and_then(|level| level.raw_u32(0x84))
            .ok_or(Block::Metadata("world wave flag"))?
            != 0,
    };
    let particle_environment = ParticleEnvironment::Terrain(
        TerrainCollisionContext::from_current_level_cache(frame.resources)
            .ok_or(Block::Metadata("particle terrain context"))?,
    );
    let model = frame
        .resources
        .global_model(entity.model_index.ok_or(Block::Runtime("active model"))?)
        .ok_or(Block::Metadata("active model"))?;
    let environment = CommonWindDrag::from_level(
        frame
            .resources
            .level_desc()
            .ok_or(Block::Metadata("world wind descriptor"))?,
        manager.common_environment_physics(),
    )
    .map_err(Block::Runtime)?;
    let RetailRuntimeValue::Known(effects) = metadata.common_world_effects else {
        return Err(Block::Metadata("world effects"));
    };
    if effects.surface_selectors != [0, 0]
        && (effects.surface_selectors != [1, 0] || effects.surface_lifetime_ms == 0)
    {
        return Err(Block::Metadata("world effects profile"));
    }
    let RetailRuntimeValue::Known(default_flags) = entity.collision.default_state_flags_at_0xc8
    else {
        return Err(Block::Runtime("default flags"));
    };
    let authored_default = metadata
        .initializer
        .as_ref()
        .ok_or(Block::Metadata("initializer"))?
        .initializer_state_flags_raw;
    // Type26 retains 0400. It does not participate in E640/DF70's gates;
    // class12 still has no attitude/ground-snap policy and does run E100/E370.
    // Type18's 431 lacks drag bit8, so its class12 effective word is 420.
    if default_flags != authored_default || !matches!(default_flags & !0x2415, 0x28 | 0x20) {
        return Err(Block::Runtime("class12 effective flags"));
    }
    // Class12's style clears 2015 and sets nothing.
    let effective_flags = default_flags & !0x2015;
    let state = bits(
        entity,
        REMOTE_OWNED_STATE_BIT
            | 0x1000
            | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            | COMMON_MASTER_MOTION_REQUIRED_STATE_MASK
            | ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
        "outer state",
    )?;
    if state & (REMOTE_OWNED_STATE_BIT | 0x1000) != 0 || entity.attached_to.is_some() {
        return Err(Block::Runtime("local unattached owner"));
    }
    if entity.collision.constructor_sound_attachment_id_at_0x8c != RetailRuntimeValue::Known(None) {
        return Err(Block::Runtime("looping sound attachment"));
    }
    match (
        &metadata.sub_j_attachment_descriptor,
        &entity.sub_j_attachment_runtime,
    ) {
        (RetailRuntimeValue::Known(None), RetailRuntimeValue::Known(None)) => {}
        (RetailRuntimeValue::Known(Some(_)), RetailRuntimeValue::Known(Some(runtime)))
            if runtime.is_empty() && runtime.policy_raw_at_0x0c() == 0 => {}
        _ => return Err(Block::Runtime("empty Sub-J")),
    }
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
            u32::from(frame.world_fx.next_shared_retail_random_u16())
        })
    else {
        return Err(Block::Runtime("scheduler"));
    };
    let entity = manager.entity_mut(id).unwrap();
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    *prefix_committed = true;
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us: dt,
    } = prefix.flow
    else {
        return Ok(Intro2CommonDyingOutcome::Waiting { entity_id: id });
    };
    if state & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT == 0 {
        commit_motion(entity, dt)?;
        return Ok(Intro2CommonDyingOutcome::Advanced {
            entity_id: id,
            detailed: false,
            callback_elapsed_micros: dt,
            terminal: false,
        });
    }
    let RetailRuntimeValue::Known(mass) = common_scheduler_callback_mass(
        metadata.mass_raw,
        entity.collision.animation_offset_at_0xb2,
    ) else {
        return Err(Block::Runtime("callback B2"));
    };
    entity.mass_raw = mass;
    let detailed = state & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0;
    let before_rotation = entity.rotation_heading_pitch_roll_raw();
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31 else {
        return Err(Block::Runtime("physical body basis"));
    };
    let detailed_plan = if detailed {
        let RetailRuntimeValue::Known(Some(sub_c)) = metadata.sub_c_lift_descriptor else {
            return Err(Block::Metadata("Sub-C"));
        };
        let attitude = plan_common_dying_terrain_attitude_raw(CommonDyingTerrainAttitudeInput {
            terrain,
            position_raw: entity.position_raw(),
            pitch_raw: before_rotation[1],
            roll_raw: before_rotation[2],
            lateral_basis_q31: basis.lateral,
            forward_basis_q31: basis.forward,
            body_up_y_q31: basis.up[1],
            state_flags: state,
            resolved_surface_mode_raw: sub_c.surface_mode_raw,
            water_enabled: wave_clock.waves_enabled,
            wave_tick_50hz: wave_clock.wave_tick_50hz,
            effective_elapsed_micros: dt,
        });
        // EC70 changes angles only. The subsequent mover consumes the retained
        // pre-effect matrix, exactly as the retail call pair does.
        let mover = if entity.native_type30_runtime.is_some() {
            mover::MoverExecution::RetainedKl
        } else {
            mover::MoverExecution::Planned(mover::plan_mover(
                entity, &metadata, basis, terrain, dt, wave_clock,
            )?)
        };
        Some((attitude, mover))
    } else {
        None
    };
    let task_prefix = entity
        .actor_tasks
        .begin_exact_visit_with(owner.visit, |task| {
            let ActorTaskRuntime::CommonDying(state) = task else {
                unreachable!()
            };
            state.before_callback(dt)
        })
        .ok_or(Block::TaskUnavailable)?;
    if let Some((attitude, mover)) = detailed_plan {
        entity.set_rotation_heading_pitch_roll_raw([
            before_rotation[0],
            attitude.pitch_raw,
            attitude.roll_raw,
        ]);
        let (position, mut velocity) = match mover {
            mover::MoverExecution::Planned(mover) => {
                entity.sub_a_propulsion_runtime = mover.snapshot.sub_a_runtime;
                entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(mover.sub_h));
                (mover.snapshot.position_raw, mover.snapshot.velocity_raw)
            }
            mover::MoverExecution::RetainedKl => {
                // Counter and EC70 angles precede H -> K -> L -> C -> A -> B.
                // A late block retains that native prefix and unwinds once.
                let result = mover::run_retained_kl_mover(
                    entity,
                    &metadata,
                    terrain,
                    dt,
                    frame.elapsed_micros,
                );
                if let Err(error) = result {
                    entity.actor_tasks.finish_exact_visit(owner.visit);
                    return Err(error);
                }
                (entity.position_raw(), entity.velocity_raw())
            }
        };
        velocity[0] = damp_common_dying_axis(velocity[0], dt);
        velocity[2] = damp_common_dying_axis(velocity[2], dt);
        entity.set_motion_raw(position, velocity);
    }
    assert!(entity.actor_tasks.finish_exact_visit(owner.visit));
    let callback_result = if detailed {
        CommonDyingCallbackResult::Continue
    } else {
        CommonDyingCallbackResult::TaggedOwnerTransition(CommonDyingTaggedResult {
            singleton_address: COMMON_DYING_SCHEDULER_SINGLETON_ADDRESS,
            tag: COMMON_DYING_OWNER_TRANSITION_TAG,
        })
    };
    let terminal = matches!(
        common_dying_after_unwind(task_prefix, callback_result),
        CommonDyingAfterUnwindOutcome::RequestOwnerTransition { .. }
    ) && bits(
        entity,
        ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
        "transition state",
    )? == 0;
    if terminal {
        crate::common_dying_live::stage_common_dying_terminal_transition(entity);
        manager.queue_actor_deferred_destroy(id);
    }
    let entity = manager.entity_mut(id).unwrap();
    if detailed {
        // DCA0: A800 and its terminal callback return before DD2E reads the
        // current health. The A0 low-health cue is independent of dying4000;
        // 94/96 apply only at/above half health and with visible800 set.
        // E870 has no corresponding sound phase.
        let record = frame
            .resources
            .global_entity_type(entity.entity_type as usize)
            .ok_or(Block::Metadata("detailed sound type"))?;
        let RetailRuntimeValue::Known(health_raw) = entity.collision.health_raw else {
            return Err(Block::Runtime("detailed sound health"));
        };
        let visible = bits(entity, 0x800, "detailed sound visibility")? != 0;
        if let Some(sound) = crate::actor_detailed_sound::plan_actor_detailed_sound(
            crate::actor_detailed_sound::ActorDetailedSoundFrame {
                type_record: record,
                health_raw,
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
    // Entry effective 28/428/420 bypasses E640; DCA0 and E870 still publish
    // F70, then E100/E370 after a tagged/expired task's terminal callback.
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    let mut velocity = entity.velocity_raw();
    apply_common_gravity_and_underwater_raw(
        &mut velocity,
        dt,
        CommonUnderwaterFrame {
            effective_environment_flags: effective_flags,
            water_response_enabled: false,
            position_y_raw: entity.position_raw()[1],
            solid_or_sea_y_raw: 0,
            self_mass_raw: mass,
            attached_cargo_mass: 0,
        },
    );
    let mut angles = [heading, pitch, roll];
    // 40E354: 44EC60 runs only for effective bit8.
    if effective_flags & 0x08 != 0 {
        apply_common_wind_drag_raw(
            &mut velocity,
            &mut angles,
            environment,
            CommonWindDragFrame {
                terrain,
                position_raw: entity.position_raw(),
                basis: Type9BodyBasis::from_angle_words(heading, pitch, roll),
                callback_mass_raw: std::num::NonZeroU16::new(mass)
                    .ok_or(Block::Runtime("zero environment mass"))?,
                elapsed_micros: dt,
            },
        );
    }
    entity.set_velocity_raw(velocity);
    entity.set_rotation_heading_pitch_roll_raw(angles);
    run_surface(
        entity,
        &metadata,
        terrain,
        model.radius,
        dt,
        frame.retail_tick,
        particle_environment,
        frame.world_fx,
    )?;
    commit_motion(entity, dt)?;
    Ok(Intro2CommonDyingOutcome::Advanced {
        entity_id: id,
        detailed,
        callback_elapsed_micros: dt,
        terminal,
    })
}

fn commit_motion(entity: &mut Entity, dt: u32) -> Result<(), Intro2CommonDyingBlock> {
    let state = bits(
        entity,
        COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
        "master motion",
    )?;
    let motion = plan_common_master_motion(entity.position_raw(), entity.velocity_raw(), state, dt);
    commit_common_scheduler_post_callback(&mut entity.collision);
    commit_common_master_motion(entity, motion);
    Ok(())
}

pub(crate) fn run_surface(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    terrain: &v2k_formats::terrain::TerrainGrid,
    extent: u16,
    dt: u32,
    tick: u32,
    particle_environment: ParticleEnvironment<'_>,
    world_fx: &mut WorldFx,
) -> Result<(), Intro2CommonDyingBlock> {
    let state = bits(
        entity,
        ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
        "surface owner bit",
    )?;
    if state != 0 {
        return Ok(());
    }
    let RetailRuntimeValue::Known(timer) = entity.surface_lifetime_timer_ms_at_0x48 else {
        return Err(Intro2CommonDyingBlock::Runtime("surface timer"));
    };
    let RetailRuntimeValue::Known(effects) = metadata.common_world_effects else {
        unreachable!()
    };
    if effects.surface_selectors == [0, 0] {
        // E370's absent +72/+73 branches only decay +48; lifetime zero is
        // valid here and never enters 162B0's percentage calculation.
        entity.surface_lifetime_timer_ms_at_0x48 =
            RetailRuntimeValue::Known(timer.saturating_sub(dt / 1000));
        return Ok(());
    }
    let phase = classify_actor_surface_timer_phase(
        timer,
        ActorSurfaceTimerFrame {
            state_flags: state,
            position_y_raw: entity.position_raw()[1],
            active_model_extent_raw: extent,
            flat_surface_y_raw: terrain.sea_level_raw(),
            elapsed_us: dt,
            authored_lifetime_ms: effects.surface_lifetime_ms,
        },
    )
    .map_err(|_| Intro2CommonDyingBlock::Metadata("surface lifetime"))?;
    let (timer, remaining) = match phase {
        ActorSurfaceTimerPhase::OwnerDisabled => return Ok(()),
        ActorSurfaceTimerPhase::NonDeep { timer_after_ms }
        | ActorSurfaceTimerPhase::DeepBeforeRandomEffects { timer_after_ms, .. } => {
            (timer_after_ms, None)
        }
        ActorSurfaceTimerPhase::DeepRandomEffects {
            timer_after_ms,
            remaining_percent,
        } => (timer_after_ms, Some(remaining_percent)),
        ActorSurfaceTimerPhase::DeepLifecycle {
            timer_after_ms,
            remaining_percent_after_lifecycle,
        } => {
            let RetailRuntimeValue::Known(default) = entity.collision.default_state_flags_at_0xc8
            else {
                unreachable!()
            };
            // The admitted class12/class14 styles have null release hooks.
            // 16750 runs, then 10C10 sees the already-dying bit and returns.
            entity.collision.state_flags_at_0x08 =
                relation_release_state_word_after(entity.collision.state_flags_at_0x08, default);
            entity.attached_to = None;
            (
                timer_after_ms,
                (remaining_percent_after_lifecycle < 75)
                    .then_some(remaining_percent_after_lifecycle),
            )
        }
    };
    entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(timer);
    if let Some(remaining) = remaining {
        let state = bits(
            entity,
            REMOTE_OWNED_STATE_BIT | DYING_STATE_BIT | 1,
            "surface emission state",
        )?;
        let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31 else {
            unreachable!()
        };
        let surface_frame = Type9SurfaceFrame {
            entity_id: entity.id,
            entity_type: entity.entity_type as u8,
            state_flags: state,
            position_raw: entity.position_raw(),
            emission_axis_q31: basis.forward,
            active_model_extent_raw: extent,
            flat_surface_y_raw: terrain.sea_level_raw(),
            elapsed_us: dt,
            authored_lifetime_ms: effects.surface_lifetime_ms,
        };
        if let Some(bubble) = plan_actor_surface_bubble(
            remaining,
            ActorSurfaceBubbleFrame {
                entity_id: entity.id,
                entity_type: entity.entity_type as u8,
                state_flags: state,
                position_raw: entity.position_raw(),
                emission_axis_q31: basis.forward,
                active_model_extent_raw: extent,
            },
            &mut || u32::from(world_fx.next_shared_retail_random_u16()),
        ) {
            world_fx.materialize_actor_surface_bubble_request(bubble, particle_environment, tick);
        }
        if let Some(sound) = plan_ordinary_type9_surface_sound(surface_frame.into(), &mut || {
            u32::from(world_fx.next_shared_retail_random_u16())
        }) {
            world_fx.queue_fixed_positional_sound_raw(sound.sound_id, sound.position_raw);
        }
    }
    Ok(())
}
