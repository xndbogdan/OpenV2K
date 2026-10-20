//! Type25's class12 "Flip Over And Die" on its B/D/F allocation.
//!
//! `10C10 -> DB80 -> AC60` alternate12 runs `C620`: clear slots 1/2, then
//! `04120`'s 9,000-ms task, whose `06070` resets Sub-F, and vertical velocity
//! 500. `04220` on a coarse visit takes `4BE170` (tag 9C01, the terminal); on
//! a detailed one it flips the body (`1EC70`, no Sub-C so mode 0), runs
//! `01430` with a null target (Sub-F's dying arm, then Sub-B) and damps X/Z.
//! The dying style clears 2015 from C8 `0xC`, so E100 now applies gravity.

use super::live::{bits, SharedFishBlock as Block, SharedFishFrame, SharedFishOutcome};
use crate::{
    actor_death::COMMON_ACTOR_DYING_ACTIVE_STYLE,
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit},
    common_dying::{
        common_dying_after_unwind, damp_common_dying_axis, plan_common_dying_setup,
        CommonDyingAfterUnwindOutcome, CommonDyingCallbackResult, CommonDyingComponentDescriptors,
        CommonDyingConstructorEffect, CommonDyingTaggedResult, CommonDyingTaskState,
        COMMON_DYING_OWNER_TRANSITION_TAG, COMMON_DYING_SCHEDULER_SINGLETON_ADDRESS,
    },
    common_mover::{
        component_dispatch::CommonMoverDispatchMode,
        type9_attitude::{
            plan_common_dying_terrain_attitude_raw, CommonDyingTerrainAttitudeInput, Type9BodyBasis,
        },
    },
    entity::{apply_common_gravity_and_underwater_raw, CommonUnderwaterFrame, EntityManager},
    entity_behavior::{audited_behavior_program, initial_behavior_state_policy},
    entity_collision_state::{
        RetailRuntimeValue, ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
        BODY_BASIS_REBUILT_STATE_BIT, DEFERRED_DESTROY_PENDING_STATE_BIT, DYING_STATE_BIT,
        REMOTE_OWNED_STATE_BIT,
    },
    entity_scheduler::*,
    live_actor_checked_damage::LiveActorDeathResult,
    world_fx::WorldFx,
};

use super::death::SharedFishDeathBlock;

/// Class12's style masks: clear 2015, set nothing.
const DYING_CLEAR_MASK: u32 = 0x2015;

/// Whether the live context is the shark's class12 dying graph.
pub(crate) fn dying_graph(entity: &crate::entity::Entity) -> bool {
    super::is_shark(entity.entity_type)
        && matches!(entity.current_behavior_context,
            RetailRuntimeValue::Known(Some(context))
                if context.active_style().style_address()
                    == COMMON_ACTOR_DYING_ACTIVE_STYLE.frame_address)
}

/// 10C10's prefix and AC60's alternate12 for a living shark. Remote and
/// already-dying entries are 10C10's no-ops.
pub(crate) fn publish_shark_death(
    manager: &mut EntityManager,
    id: u32,
    fx: &mut WorldFx,
) -> Result<LiveActorDeathResult<()>, SharedFishDeathBlock> {
    use SharedFishDeathBlock as Error;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Error::Allocation)?;
    let state = |mask| match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        RetailRuntimeValue::Unresolved => Err(Error::Runtime("death entry state")),
    };
    if state(REMOTE_OWNED_STATE_BIT)? != 0 {
        return Ok(LiveActorDeathResult {
            returned_nonzero: false,
            publication: None,
        });
    }
    if state(DYING_STATE_BIT)? != 0 {
        return Ok(LiveActorDeathResult {
            returned_nonzero: true,
            publication: None,
        });
    }
    if state(DEFERRED_DESTROY_PENDING_STATE_BIT)? != 0 {
        return Err(Error::Runtime("deferred destroy already pending"));
    }
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .cloned()
        .ok_or(Error::Metadata)?;
    super::authenticate_metadata(&metadata).map_err(|_| Error::Metadata)?;
    if metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
        || entity.collision.constructor_sound_attachment_id_at_0x8c
            != RetailRuntimeValue::Known(None)
    {
        return Err(Error::Runtime("constructor sound attachment"));
    }
    let RetailRuntimeValue::Known(death_sound) = metadata.death_sound_id else {
        return Err(Error::Metadata);
    };
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Error::Graph);
    };
    // Styles 4C7930/79C0/7D20 hold null +2C death hooks.
    if !matches!(
        context.active_style().style_address(),
        0x4c7930 | 0x4c79c0 | 0x4c7d20
    ) {
        return Err(Error::Graph);
    }
    let program = audited_behavior_program(12).unwrap();
    let selected = context
        .reselect_audited_type_default(program, 0, COMMON_ACTOR_DYING_ACTIVE_STYLE)
        .ok_or(Error::Graph)?;
    let prepared = CommonDyingTaskState::prepare_after_allocation(
        id,
        &metadata,
        CommonDyingComponentDescriptors::default(),
    )
    .map_err(|_| Error::Runtime("class12 constructor"))?
    .map_task(ActorTaskRuntime::CommonDying);
    let entity = manager.entity_mut(id).unwrap();
    let position = entity.position_raw();
    let mut velocity = entity.velocity_raw();
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    if let Some(sound) = death_sound {
        fx.emit_fixed_positional_sound_raw(sound, position);
    }
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(selected));
    let policy = initial_behavior_state_policy(program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    let crate::entity::Entity {
        actor_tasks,
        shared_fish_runtime,
        ..
    } = entity;
    let fish = shared_fish_runtime.as_mut().unwrap();
    let mut prepared = Some(prepared);
    plan_common_dying_setup(id)
        .apply(
            actor_tasks,
            || u32::from(fx.next_shared_retail_random_u16()),
            |effect| match effect {
                CommonDyingConstructorEffect::WriteSubFPhase3c { value } => {
                    fish.sub_f.set_reversal_raw(value)
                }
                CommonDyingConstructorEffect::ResetSubFModeAndAccumulator { mode_3d, .. } => {
                    fish.sub_f.set_follow_target_mode_raw(mode_3d)
                }
                CommonDyingConstructorEffect::WriteOwnerVerticalVelocity { velocity_raw } => {
                    velocity[1] = velocity_raw
                }
                _ => unreachable!("authenticated B/D/F class12 constructor"),
            },
            |_| Ok::<_, ()>(prepared.take().unwrap()),
        )
        .expect("host task allocation and prevalidated suffix");
    entity.set_velocity_raw(velocity);
    Ok(LiveActorDeathResult {
        returned_nonzero: true,
        publication: Some(()),
    })
}

fn visit(entity: &crate::entity::Entity) -> Result<ActorTaskVisit, Block> {
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(Block::Graph)?;
    if !matches!(
        entity.actor_tasks.task_state(task_id),
        Some(ActorTaskRuntime::CommonDying(_))
    ) || entity.actor_task_state(ActorTaskSlot::Secondary).is_some()
        || entity.actor_task_state(ActorTaskSlot::Tertiary).is_some()
    {
        return Err(Block::Graph);
    }
    Ok(ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    })
}

/// 12DA0 -> DCA0/E870 with 04220 in Primary. Returns whether the terminal ran.
pub(super) fn run_frame(
    manager: &mut EntityManager,
    id: u32,
    frame: SharedFishFrame<'_>,
    prefix_committed: &mut bool,
) -> Result<SharedFishOutcome, Block> {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Allocation)?;
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .cloned()
        .ok_or(Block::Metadata)?;
    super::authenticate_metadata(&metadata).map_err(|_| Block::Metadata)?;
    let RetailRuntimeValue::Known(default_flags @ 0xc) =
        entity.collision.default_state_flags_at_0xc8
    else {
        return Err(Block::Runtime("shark default environment"));
    };
    let effective_flags = default_flags & !DYING_CLEAR_MASK;
    let state = bits(
        entity,
        REMOTE_OWNED_STATE_BIT
            | 0x1000
            | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            | crate::common_mover::type9_tail::COMMON_MASTER_MOTION_REQUIRED_STATE_MASK
            | ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
    )?;
    if state & (REMOTE_OWNED_STATE_BIT | 0x1000) != 0 || entity.attached_to.is_some() {
        return Err(Block::Runtime("local unattached dying shark"));
    }
    let visit = visit(entity)?;
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
            u32::from(frame.world_fx.next_shared_retail_random_u16())
        })
    else {
        return Err(Block::Runtime("dying shark scheduler"));
    };
    let entity = manager.entity_mut(id).unwrap();
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    *prefix_committed = true;
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us: dt,
    } = prefix.flow
    else {
        return Ok(SharedFishOutcome::Waiting { entity_id: id });
    };
    let advanced = SharedFishOutcome::Advanced {
        entity_id: id,
        callback_enabled: state & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT != 0,
        callback_elapsed_micros: dt,
    };
    if state & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT == 0 {
        super::live::commit_motion(entity, dt)?;
        return Ok(advanced);
    }
    let RetailRuntimeValue::Known(mass) = common_scheduler_callback_mass(
        metadata.mass_raw,
        entity.collision.animation_offset_at_0xb2,
    ) else {
        return Err(Block::Runtime("dying shark callback mass"));
    };
    entity.mass_raw = mass;
    let detailed = state & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0;
    // 01280/E640's wave flag is the active descriptor's +84.
    let waves_enabled = frame
        .resources
        .level_desc()
        .and_then(|level| level.raw_u32(0x84))
        .ok_or(Block::Runtime("dying shark wave flag"))?
        != 0;
    let terrain = frame
        .resources
        .level_terrain()
        .ok_or(Block::Runtime("dying shark terrain"))?;
    let extent = entity
        .model_index
        .and_then(|model| frame.resources.global_model(model))
        .ok_or(Block::Runtime("dying shark model"))?
        .radius;
    let task_prefix = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |task| {
            let ActorTaskRuntime::CommonDying(state) = task else {
                unreachable!()
            };
            state.before_callback(dt)
        })
        .ok_or(Block::Graph)?;
    let callback_result = if detailed {
        let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
            entity.actor_tasks.finish_exact_visit(visit);
            return Err(Block::Runtime("dying shark basis"));
        };
        let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
        // 1EC70 writes pitch/roll; 01430 then reads them over the old basis.
        let attitude = plan_common_dying_terrain_attitude_raw(CommonDyingTerrainAttitudeInput {
            terrain,
            position_raw: entity.position_raw(),
            pitch_raw: pitch,
            roll_raw: roll,
            lateral_basis_q31: basis.lateral,
            forward_basis_q31: basis.forward,
            body_up_y_q31: basis.up[1],
            state_flags: state,
            resolved_surface_mode_raw: 0,
            water_enabled: waves_enabled,
            wave_tick_50hz: frame.retail_tick as i32,
            effective_elapsed_micros: dt,
        });
        entity.set_rotation_heading_pitch_roll_raw([
            heading,
            attitude.pitch_raw,
            attitude.roll_raw,
        ]);
        let moved = super::mover::run(
            entity,
            super::mover::FishMoverFrame {
                metadata: &metadata,
                terrain,
                dispatch_mode: CommonMoverDispatchMode::Normal,
                elapsed_micros: dt,
                global_elapsed_micros: frame.global_elapsed_micros,
                active_model_extent_raw: extent,
            },
            None,
            RetailRuntimeValue::Unresolved,
            &mut || panic!("class12's null target draws no word"),
        );
        if let Err(error) = moved {
            entity.actor_tasks.finish_exact_visit(visit);
            return Err(error);
        }
        let mut velocity = entity.velocity_raw();
        velocity[0] = damp_common_dying_axis(velocity[0], dt);
        velocity[2] = damp_common_dying_axis(velocity[2], dt);
        entity.set_velocity_raw(velocity);
        CommonDyingCallbackResult::Continue
    } else {
        CommonDyingCallbackResult::TaggedOwnerTransition(CommonDyingTaggedResult {
            singleton_address: COMMON_DYING_SCHEDULER_SINGLETON_ADDRESS,
            tag: COMMON_DYING_OWNER_TRANSITION_TAG,
        })
    };
    let entity = manager.entity_mut(id).unwrap();
    assert!(entity.actor_tasks.finish_exact_visit(visit));
    let terminal = matches!(
        common_dying_after_unwind(task_prefix, callback_result),
        CommonDyingAfterUnwindOutcome::RequestOwnerTransition { .. }
    ) && bits(entity, ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT)? == 0;
    if terminal {
        crate::common_dying_live::stage_common_dying_terminal_transition(entity);
        manager.queue_actor_deferred_destroy(id);
    }
    let entity = manager.entity_mut(id).unwrap();
    if detailed {
        // DCA0's detailed sound reads the zero health; Type25 authors none.
        let record = frame
            .resources
            .global_entity_type(entity.entity_type as usize)
            .ok_or(Block::Metadata)?;
        if let Some(sound) = crate::actor_detailed_sound::plan_actor_detailed_sound(
            crate::actor_detailed_sound::ActorDetailedSoundFrame {
                type_record: record,
                health_raw: 0,
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
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    let mut velocity = entity.velocity_raw();
    // E100 without Sub-C: gravity and no water response.
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
    entity.set_velocity_raw(velocity);
    if effective_flags & 8 != 0 {
        super::live::apply_environment(manager, id, &frame, dt)?;
    }
    let entity = manager.entity_mut(id).unwrap();
    super::surface::run(
        entity,
        metadata.common_world_effects,
        terrain.sea_level_raw(),
        dt,
    )?;
    super::live::commit_motion(entity, dt)?;
    Ok(advanced)
}
