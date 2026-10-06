//! Complete 12DA0 -> DCA0/E870 -> F70/E100/DF70/E370 -> master-motion visit.
use super::*;
use crate::{
    common_mover::{
        environment::{apply_common_wind_drag_raw, CommonWindDrag, CommonWindDragFrame},
        type9_attitude::Type9BodyBasis,
        type9_tail::{
            apply_type9_ground_snap_raw, plan_common_master_motion,
            COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
        },
    },
    entity::commit_common_master_motion,
    entity_collision_state::{BODY_BASIS_REBUILT_STATE_BIT, REMOTE_OWNED_STATE_BIT},
    entity_scheduler::*,
    intro2_common_dying::{run_actor_surface_with_death, Intro2ActorSurfaceFrame},
    resource_cache::ResourceCache,
    world_fx::{ParticleEnvironment, TerrainCollisionContext},
};

pub struct Intro2Type8Frame<'a> {
    pub resources: &'a ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
    pub retail_tick: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type8Outcome {
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
        reason: Intro2Type8Block,
        prefix_committed: bool,
    },
    Pending {
        entity_id: u32,
    },
    Dropped {
        entity_id: u32,
    },
}
impl Intro2Type8Outcome {
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
pub struct Intro2Type8Tick {
    pub outcome: Intro2Type8Outcome,
    pub retained_owner: Option<Intro2Type8Owner>,
}

pub fn tick_intro2_type8(
    manager: &mut EntityManager,
    mut owner: Intro2Type8Owner,
    frame: Intro2Type8Frame<'_>,
) -> Intro2Type8Tick {
    let id = owner.entity_id();
    // Check graph identity separately from pending: an unfinished visit stays
    // parked, while a destroyed or replaced allocation drops its old owner.
    let allocation_matches = manager
        .main_base_abort_actor_observation(id)
        .is_some_and(|observation| observation.lease == owner.allocation.allocation)
        && manager
            .iter_all()
            .find(|entity| entity.id == id)
            .is_some_and(|entity| {
                intro2_type8_allocation_authenticates(entity)
                    && entity
                        .intro2_type8_runtime
                        .is_some_and(|runtime| runtime.same_allocation(owner.allocation))
            });
    if !allocation_matches {
        return Intro2Type8Tick {
            outcome: Intro2Type8Outcome::Dropped { entity_id: id },
            retained_owner: None,
        };
    }
    if owner.pending {
        return Intro2Type8Tick {
            outcome: Intro2Type8Outcome::Pending { entity_id: id },
            retained_owner: Some(owner),
        };
    }
    let current = manager
        .iter_all()
        .find(|e| e.id == id)
        .and_then(Intro2Type8Owner::adopt_published);
    let matches = current.is_some_and(|current| {
        current.allocation.same_allocation(owner.allocation)
            && ((current.task_id == owner.task_id && current.context == owner.context)
                || current
                    .allocation
                    .relation_graph
                    .is_some_and(|publication| {
                        publication.matches(current.task_id, current.context)
                    }))
    });
    if !matches {
        return Intro2Type8Tick {
            outcome: Intro2Type8Outcome::Dropped { entity_id: id },
            retained_owner: None,
        };
    }
    // Only an exact attach/release graph receipt can transfer external task
    // publication. Arbitrary graph edits still drop the old linear owner.
    owner = current.unwrap();
    let mut prefix = false;
    match run_frame(manager, &mut owner, frame, &mut prefix) {
        Ok(outcome) => {
            let terminal = matches!(outcome, Intro2Type8Outcome::Advanced { terminal: true, .. });
            Intro2Type8Tick {
                outcome,
                retained_owner: (!terminal).then_some(owner),
            }
        }
        Err(reason) => {
            owner.pending = prefix;
            Intro2Type8Tick {
                outcome: Intro2Type8Outcome::Blocked {
                    entity_id: id,
                    reason,
                    prefix_committed: prefix,
                },
                retained_owner: Some(owner),
            }
        }
    }
}

fn run_frame(
    manager: &mut EntityManager,
    owner: &mut Intro2Type8Owner,
    frame: Intro2Type8Frame<'_>,
    prefix_committed: &mut bool,
) -> Result<Intro2Type8Outcome, Intro2Type8Block> {
    let id = owner.entity_id();
    let entity_type = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .map(|entity| entity.entity_type)
        .ok_or(Intro2Type8Block::Runtime("entity"))?;
    let metadata = manager
        .type_runtime_metadata(entity_type)
        .cloned()
        .ok_or(Intro2Type8Block::Runtime("metadata"))?;
    validate_worker_metadata(entity_type, &metadata)?;
    let terrain = frame
        .resources
        .level_terrain()
        .ok_or(Intro2Type8Block::Runtime("terrain"))?;
    let environment = CommonWindDrag::from_level(
        frame
            .resources
            .level_desc()
            .ok_or(Intro2Type8Block::Runtime("world wind descriptor"))?,
        manager.common_environment_physics(),
    )
    .map_err(Intro2Type8Block::Runtime)?;
    let particle_environment = ParticleEnvironment::Terrain(
        TerrainCollisionContext::from_current_level_cache(frame.resources)
            .ok_or(Intro2Type8Block::Runtime("surface environment"))?,
    );
    let model_id = NativeWorkerProfile::from_entity_type(entity_type)
        .ok_or(Intro2Type8Block::Runtime("worker model"))?
        .model_id();
    let model = frame
        .resources
        .global_model(model_id)
        .ok_or(Intro2Type8Block::Runtime("worker model"))?;
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let state = bits(
        entity,
        REMOTE_OWNED_STATE_BIT
            | 0x1000
            | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            | COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
        "outer state",
    )?;
    if state & REMOTE_OWNED_STATE_BIT != 0 {
        return Err(Intro2Type8Block::Runtime("local actor"));
    }
    let corpse_relation = if owner.kind == TaskKind::Exploding {
        Some(
            crate::intro2_type17::capture::prepare_native_corpse_relation(manager, id)
                .map_err(|block| Intro2Type8Block::Runtime(block.reason))?,
        )
    } else {
        None
    };
    if owner.kind == TaskKind::Carried {
        let parent_valid = entity.attached_to.and_then(|parent_id|
            manager.iter_all().find(|parent| parent.id == parent_id)).is_some_and(|parent|
                parent.active
                    && parent.collision.state_flags_at_0x08.masked(REMOTE_OWNED_STATE_BIT) == RetailRuntimeValue::Known(0)
                    && matches!(&parent.sub_j_attachment_runtime, RetailRuntimeValue::Known(Some(sub_j))
                        if sub_j.ordered_entity_ids().contains(&id)));
        if state & 0x1000 == 0 || !parent_valid {
            return Err(Intro2Type8Block::Runtime("carried relation membership"));
        }
    } else if corpse_relation.is_none() && (state & 0x1000 != 0 || entity.attached_to.is_some()) {
        return Err(Intro2Type8Block::Runtime("local unattached actor"));
    }
    if entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(0x2f) {
        return Err(Intro2Type8Block::Runtime("effective2F"));
    }
    let entity = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
            u32::from(frame.world_fx.next_shared_retail_random_u16())
        })
    else {
        return Err(Intro2Type8Block::Runtime("scheduler"));
    };
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    *prefix_committed = true;
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us: dt,
    } = prefix.flow
    else {
        return Ok(Intro2Type8Outcome::Waiting { entity_id: id });
    };
    if let Some(relation) = corpse_relation {
        crate::intro2_type17::capture::commit_native_corpse_relation(entity, relation);
    }
    let detailed = state & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0;
    if state & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT == 0 {
        commit_motion(entity, dt)?;
        return Ok(Intro2Type8Outcome::Advanced {
            entity_id: id,
            detailed,
            callback_elapsed_micros: dt,
            terminal: false,
        });
    }
    let RetailRuntimeValue::Known(mass) = common_scheduler_callback_mass(
        metadata.mass_raw,
        entity.collision.animation_offset_at_0xb2,
    ) else {
        return Err(Intro2Type8Block::Runtime("callback B2"));
    };
    entity.mass_raw = mass;
    let terminal = task::run_task(
        manager,
        owner,
        &task::TaskFrame {
            metadata: &metadata,
            terrain,
            elapsed_micros: dt,
            global_elapsed_micros: frame.global_elapsed_micros,
            scheduler_mode: if detailed { 0 } else { 1 },
        },
        frame.world_fx,
    )?;
    let entity = manager.entity_mut(id).unwrap();
    // DCA0's detailed sound is independently type-owned; both worker records
    // have no cue, but call the common planner so source gates stay explicit.
    if detailed {
        let record = frame
            .resources
            .global_entity_type(entity.entity_type as usize)
            .ok_or(Intro2Type8Block::Runtime("type record"))?;
        let RetailRuntimeValue::Known(health) = entity.collision.health_raw else {
            return Err(Intro2Type8Block::Runtime("health"));
        };
        if let Some(sound) = crate::actor_detailed_sound::plan_actor_detailed_sound(
            crate::actor_detailed_sound::ActorDetailedSoundFrame {
                type_record: record,
                health_raw: health,
                visible: bits(entity, 0x800, "visibility")? != 0,
                callback_elapsed_micros: dt,
            },
            &mut || u32::from(frame.world_fx.next_shared_retail_random_u16()),
        ) {
            frame
                .world_fx
                .queue_fixed_positional_sound_raw(sound, entity.position_raw());
        }
    }
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    let basis = Type9BodyBasis::from_angle_words(heading, pitch, roll);
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(basis);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    let mut velocity = entity.velocity_raw();
    let mut angles = [heading, pitch, roll];
    // E100's effective2F/AD retains drag bit8. EC60 uses the frame-owned
    // authored/current wind and changes angle words after F70; the physical
    // basis stays at F70 until the next source rebuild.
    apply_common_wind_drag_raw(
        &mut velocity,
        &mut angles,
        environment,
        CommonWindDragFrame {
            terrain,
            position_raw: entity.position_raw(),
            basis,
            callback_mass_raw: std::num::NonZeroU16::new(mass)
                .expect("scheduler promotes zero mass"),
            elapsed_micros: dt,
        },
    );
    entity.set_rotation_heading_pitch_roll_raw(angles);
    let mut position = entity.position_raw();
    let mut flags = 0;
    // Carrying styles7A08/87D0 use (2F|80)&!2 = AD. Their None task
    // and F70/E100/E370 still run, but DF70 must not pull cargo to ground.
    if owner.kind != TaskKind::Carried {
        apply_type9_ground_snap_raw(&mut position, &mut velocity, &mut flags, terrain);
    }
    entity.set_motion_raw(position, velocity);
    if owner.kind != TaskKind::Carried {
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x800000, flags & 0x800000);
    }
    let mut death = None;
    let surface = run_actor_surface_with_death(
        manager,
        id,
        Intro2ActorSurfaceFrame {
            metadata: &metadata,
            terrain,
            active_model_extent_raw: model.radius,
            elapsed_micros: dt,
            retail_tick: frame.retail_tick,
            particle_environment,
        },
        frame.world_fx,
        &mut death,
        intro2_type8_allocation_authenticates,
        |manager, id, fx| {
            impact::run_intro2_type8_standard_death(manager, id, fx)
                .map(|result| result.publication)
                .map_err(|_| {
                    crate::intro2_common_dying::Intro2CommonDyingBlock::Runtime(
                        "Type8 surface death",
                    )
                })
        },
    );
    // A new class14 graph survives a later surface/sound block. The next frame
    // must never repeat this callback or the surface death constructor draw.
    if let Some(death) = death {
        *owner = death;
    }
    surface.map_err(Intro2Type8Block::Surface)?;
    commit_motion(manager.entity_mut(id).unwrap(), dt)?;
    Ok(Intro2Type8Outcome::Advanced {
        entity_id: id,
        detailed,
        callback_elapsed_micros: dt,
        terminal,
    })
}

fn commit_motion(entity: &mut Entity, dt: u32) -> Result<(), Intro2Type8Block> {
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

#[cfg(test)]
#[path = "wind_tests.rs"]
mod wind_tests;
