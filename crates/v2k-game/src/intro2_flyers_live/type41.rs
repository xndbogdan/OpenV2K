//! Type41's D/G graphs: class6 Wander (`AD10`), class10 Run Away (`B6C0`,
//! then `C7D0`/`B3F0`'s flee) and alternate class12 (`C620`/`04120`/`04220`).
//! Its class4 is Type14's `B9E0` and its class5 the shared aimless graph.
//!
//! Every task constructor reaches `406070`, whose Sub-G branch draws one RNG
//! word. Run Away is weighted by UnderAttack (rule 2). Class12's `04120`
//! sets vertical velocity 500 and then `1B970(G, 1)`: Sub-G's flight
//! controller stops, so `04220`'s null-target `01430` leaves only the zero
//! force words; a detailed visit first flips the body (`1EC70`, mode 0
//! without Sub-C) and afterwards damps X and Z.

use super::*;
use crate::actor_death::{COMMON_ACTOR_DYING_ACTIVE_STYLE, COMMON_ACTOR_DYING_COMPLETION_STYLE};
use crate::common_dying::{
    common_dying_after_unwind, damp_common_dying_axis, plan_common_dying_setup,
    CommonDyingAfterUnwindOutcome, CommonDyingCallbackResult, CommonDyingComponentDescriptors,
    CommonDyingConstructorEffect, CommonDyingSubGDescriptorSnapshot, CommonDyingTaggedResult,
    CommonDyingTaskState, COMMON_DYING_OWNER_TRANSITION_TAG,
    COMMON_DYING_SCHEDULER_SINGLETON_ADDRESS,
};
use crate::common_mover::type9_attitude::{
    plan_common_dying_terrain_attitude_raw, CommonDyingTerrainAttitudeInput,
};
use crate::entity_behavior::{audited_behavior_program, BehaviorWeightRule};
use crate::entity_collision_state::{
    ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT, DEFERRED_DESTROY_PENDING_STATE_BIT,
    DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
};
use crate::live_actor_checked_damage::LiveActorDeathResult;
use crate::native_ground_actor::run_away;
use crate::ordinary_type9_wander_owner::{
    ordinary_type9_wander_after_unwind, OrdinaryType9WanderCallbackPrefix,
    OrdinaryType9WanderPostUnwind, OrdinaryType9WanderTaskState,
};
use crate::wander_near_location::map_common_mover_return;

pub(super) const WANDER_CLASS_ID: u8 = 6;
pub(super) const RUN_AWAY_CLASS_ID: u8 = 10;
const WANDER_STYLE_ADDRESS: u32 = 0x004C_79C0;
const RUN_AWAY_ACQUIRING_STYLE_ADDRESS: u32 = 0x004C_7618;
const RUN_AWAY_FLEEING_STYLE_ADDRESS: u32 = 0x004C_7660;

/// 425680's rule 2: a hit stamped `+34` within the last 250 ticks, once the
/// clock has passed 249. A newborn's `+34` is zero.
pub(super) fn under_attack(entity: &Entity, retail_tick: u32) -> Option<bool> {
    let RetailRuntimeValue::Known(last_hit) = entity.collision.last_hit_presentation_tick_at_0x34
    else {
        return None;
    };
    Some(retail_tick.wrapping_sub(last_hit) < 250 && (retail_tick as i32) > 249)
}

/// Whether the row authors rule 2.
pub(super) fn weighs_under_attack(metadata: &EntityTypeRuntimeMetadata) -> bool {
    metadata.initializer.as_ref().is_some_and(|initializer| {
        initializer.behavior_choices.iter().any(|choice| {
            BehaviorWeightRule::from_raw(choice.weight_rule_id)
                == Some(BehaviorWeightRule::UnderAttack)
        })
    })
}

fn is_type41(entity: &Entity) -> bool {
    entity.entity_type == TYPE41_ENTITY_TYPE
}

/// A Type41 style that `C690` may replace: Wander, Run Away or Defecate
/// Virus (the aimless style is the shared one).
pub(super) fn context_authenticates(entity: &Entity) -> bool {
    is_type41(entity)
        && (type14::named_context(entity, WANDER_CLASS_ID, 0, WANDER_STYLE_ADDRESS)
            || type14::named_context(
                entity,
                RUN_AWAY_CLASS_ID,
                0,
                RUN_AWAY_ACQUIRING_STYLE_ADDRESS,
            )
            || type14::named_context(entity, RUN_AWAY_CLASS_ID, 1, RUN_AWAY_FLEEING_STYLE_ADDRESS)
            || type14::named_context(entity, DEFECATE_VIRUS_CLASS_ID, 0, 0x004C_7E88))
}

/// `AD10`: the 5000-ms anchor wander alone.
pub(super) fn wander_graph_authenticates(entity: &Entity) -> bool {
    is_type41(entity)
        && flyer_identity_authenticates(entity)
        && type14::named_context(entity, WANDER_CLASS_ID, 0, WANDER_STYLE_ADDRESS)
        && flyer_wrappers_live(entity)
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::OrdinaryType9Wander(_))
        )
        && entity.actor_task_state(ActorTaskSlot::Secondary).is_none()
        && entity.actor_task_state(ActorTaskSlot::Tertiary).is_none()
}

/// `B6C0` under `4C7618`: the 500-ms retarget and the search.
pub(super) fn acquiring_graph_authenticates(entity: &Entity) -> bool {
    is_type41(entity)
        && flyer_identity_authenticates(entity)
        && type14::named_context(
            entity,
            RUN_AWAY_CLASS_ID,
            0,
            RUN_AWAY_ACQUIRING_STYLE_ADDRESS,
        )
        && flyer_wrappers_live(entity)
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(_))
        )
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::TargetAcquisition(_))
        )
        && entity.actor_task_state(ActorTaskSlot::Tertiary).is_none()
}

/// `B3F0` under `4C7660`: the flee from the context's target alone.
pub(super) fn fleeing_graph_authenticates(entity: &Entity) -> bool {
    if !is_type41(entity)
        || !flyer_identity_authenticates(entity)
        || !type14::named_context(entity, RUN_AWAY_CLASS_ID, 1, RUN_AWAY_FLEEING_STYLE_ADDRESS)
        || !flyer_wrappers_live(entity)
        || entity.actor_task_state(ActorTaskSlot::Secondary).is_some()
        || entity.actor_task_state(ActorTaskSlot::Tertiary).is_some()
    {
        return false;
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return false;
    };
    matches!(entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::RunAway(task))
            if context.target_handle_at_0x08() == RetailRuntimeValue::Known(Some(task.target_id())))
}

/// `C620`'s graph: the 9000-ms dying task alone under `4C7ED0`.
pub(crate) fn dying_graph_authenticates(entity: &Entity) -> bool {
    is_type41(entity)
        && flyer_identity_authenticates(entity)
        && matches!(entity.current_behavior_context,
            RetailRuntimeValue::Known(Some(context))
                if context.active_style().style_address()
                    == COMMON_ACTOR_DYING_ACTIVE_STYLE.frame_address)
        && flyer_wrappers_live(entity)
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::CommonDying(_))
        )
        && entity.actor_task_state(ActorTaskSlot::Secondary).is_none()
        && entity.actor_task_state(ActorTaskSlot::Tertiary).is_none()
}

/// The completed class12 terminal: `C470`'s style with every slot cleared.
pub(crate) fn finished_dying_authenticates(entity: &Entity) -> bool {
    is_type41(entity)
        && flyer_identity_authenticates(entity)
        && matches!(entity.current_behavior_context,
            RetailRuntimeValue::Known(Some(context))
                if context.active_style().style_address()
                    == COMMON_ACTOR_DYING_COMPLETION_STYLE.frame_address)
        && ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_task_state(slot).is_none())
}

/// `AD10`: clear slots 2 then 1, then `402E20`'s 5000-ms wander around the
/// immutable anchor, whose 06030 runs the Sub-G 06070 branch.
pub(super) fn publish_wander(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    context: BehaviorContextRuntime,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), Intro2FlyerPublicationError> {
    let descriptor = authenticate_flyer_mover_metadata(entity.entity_type, metadata)
        .map_err(|_| Intro2FlyerPublicationError::MetadataMismatch)?;
    let RetailRuntimeValue::Known(Some(mut sub_g)) = entity.sub_g_06070_runtime else {
        return Err(Intro2FlyerPublicationError::SubGRuntimeUnavailable);
    };
    type14::enter_class(entity, WANDER_CLASS_ID, context)?;
    entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
    apply_task_constructor_sub_g_reset(&mut sub_g, descriptor, next_random);
    entity.sub_g_06070_runtime = RetailRuntimeValue::Known(Some(sub_g));
    let position = entity.position_raw();
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Primary,
        PreparedActorTask::new(ActorTaskRuntime::OrdinaryType9Wander(
            OrdinaryType9WanderTaskState::from_allocated_anchor(position),
        )),
    );
    Ok(())
}

/// One `402EB0` visit: the anchor retarget, then the flyer mover.
pub(super) fn tick_wander(
    entity: &mut Entity,
    frame: FlyerTaskFrame<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(bool, Option<Type13SubGSound>), Intro2FlyerRuntimeBlock> {
    let anchor = entity
        .intro2_flyer_frame_owner
        .ok_or(Intro2FlyerRuntimeBlock::MetadataMismatch)?
        .anchor_raw;
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let (lifetime_status, mut stage) = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::OrdinaryType9Wander(task) = runtime else {
                unreachable!()
            };
            (
                task.before_callback(frame.elapsed_micros),
                task.stage_callback(anchor, &mut *next_random),
            )
        })
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let request = frame.request(
        entity,
        *stage.private_state_mut(),
        RetailRuntimeValue::Known(None),
    );
    let evaluated = request.and_then(|request| {
        evaluate_intro2_flyer_common_mover(request, &mut *next_random)
            .map_err(Intro2FlyerRuntimeBlock::CommonMover)
    });
    if let Ok(outcome) = &evaluated {
        *stage.private_state_mut() = outcome.target_private;
    }
    if let Some(ActorTaskRuntime::OrdinaryType9Wander(task)) =
        entity.actor_tasks.task_state_mut(task_id)
    {
        // The anchor retarget precedes the mover's first boundary.
        stage.commit(task);
    }
    if !entity.actor_tasks.finish_exact_visit(visit) {
        return Err(Intro2FlyerRuntimeBlock::TaskUnavailable);
    }
    let outcome = evaluated?;
    commit_intro2_flyer_mover_pose(entity, outcome);
    let prefix = OrdinaryType9WanderCallbackPrefix {
        lifetime_status,
        retarget: stage.retarget(),
    };
    let result = map_common_mover_return(match outcome.result {
        ChaseTargetCommonMoverReturn::NonZero => WanderNearCommonMoverReturn::NonZero,
        ChaseTargetCommonMoverReturn::Zero => WanderNearCommonMoverReturn::Zero,
    });
    // 4C79C0 +00/+04 both point to C690.
    match ordinary_type9_wander_after_unwind(visit, prefix, result) {
        OrdinaryType9WanderPostUnwind::Continue => Ok((false, outcome.sound)),
        OrdinaryType9WanderPostUnwind::Transition(_) => Ok((true, outcome.sound)),
        OrdinaryType9WanderPostUnwind::UnresolvedCommonMover => {
            Err(Intro2FlyerRuntimeBlock::TaskUnavailable)
        }
    }
}

fn map_run_away<E>(error: run_away::NativeRunAwayBlock<E>) -> Intro2FlyerRuntimeBlock
where
    E: Into<Intro2FlyerRuntimeBlock>,
{
    match error {
        run_away::NativeRunAwayBlock::Acquisition(reason) => {
            Intro2FlyerRuntimeBlock::Acquisition(reason)
        }
        run_away::NativeRunAwayBlock::Mover(reason) => reason.into(),
        run_away::NativeRunAwayBlock::Allocation
        | run_away::NativeRunAwayBlock::Graph
        | run_away::NativeRunAwayBlock::Runtime(_) => Intro2FlyerRuntimeBlock::TaskUnavailable,
    }
}

impl From<std::convert::Infallible> for Intro2FlyerRuntimeBlock {
    fn from(never: std::convert::Infallible) -> Self {
        match never {}
    }
}

/// One `02080` visit of `4C7618`'s search; a found body reaches `C7D0`,
/// whose `4C7660` runs `B3F0` with `06070`'s Sub-G word.
pub(super) fn acquire(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
) -> Result<(), Intro2FlyerRuntimeBlock> {
    run_away::acquire(manager, id, metadata, world_fx).map_err(map_run_away)
}

/// One `403F40` visit around the flyer mover; returns whether `C690` must
/// reselect, and Sub-G's wing beat.
pub(super) fn tick_flee(
    manager: &mut EntityManager,
    id: u32,
    frame: FlyerTaskFrame<'_>,
    world_fx: &mut WorldFx,
) -> Result<(bool, Option<Type13SubGSound>), Intro2FlyerRuntimeBlock> {
    let mut sound = None;
    let transition = run_away::tick_primary(
        manager,
        id,
        run_away::NativeRunAwayFrame {
            elapsed_micros: frame.elapsed_micros,
            dispatch_mode: frame.dispatch_mode,
        },
        world_fx,
        |request| {
            let outcome = evaluate_intro2_flyer_common_mover(
                frame.request(request.entity, *request.target, request.tracked_target)?,
                || u32::from(request.world_fx.next_shared_retail_random_u16()),
            )
            .map_err(Intro2FlyerRuntimeBlock::CommonMover)?;
            *request.target = outcome.target_private;
            commit_intro2_flyer_mover_pose(request.entity, outcome);
            sound = outcome.sound;
            Ok::<_, Intro2FlyerRuntimeBlock>(matches!(
                outcome.result,
                ChaseTargetCommonMoverReturn::NonZero
            ))
        },
    )
    .map_err(map_run_away)?;
    Ok((transition, sound))
}

/// `10C10`'s prefix and `AC60`'s alternate12 (`C620`) for a living Type41.
/// Remote and already-dying entries are `10C10`'s no-ops.
pub(crate) fn publish_standard_death(
    manager: &mut EntityManager,
    id: u32,
    world_fx: &mut WorldFx,
) -> Result<LiveActorDeathResult<Intro2FlyerSchedulerOwner>, &'static str> {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or("class12 allocation")?;
    if !flyer_manager_identity_authenticates(manager, id) || !is_type41(entity) {
        return Err("class12 native flyer allocation");
    }
    let RetailRuntimeValue::Known(flags) = entity
        .collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT | DYING_STATE_BIT | DEFERRED_DESTROY_PENDING_STATE_BIT)
    else {
        return Err("class12 entry state");
    };
    if flags & REMOTE_OWNED_STATE_BIT != 0 {
        return Ok(LiveActorDeathResult {
            returned_nonzero: false,
            publication: None,
        });
    }
    if flags & DYING_STATE_BIT != 0 {
        return Ok(LiveActorDeathResult {
            returned_nonzero: true,
            publication: None,
        });
    }
    if flags & DEFERRED_DESTROY_PENDING_STATE_BIT != 0 {
        return Err("class12 deferred destroy pending");
    }
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .cloned()
        .ok_or("class12 metadata")?;
    let descriptor = authenticate_flyer_mover_metadata(entity.entity_type, &metadata)
        .map_err(|_| "class12 D/G descriptors")?;
    if metadata
        .initializer
        .as_ref()
        .map(|init| init.alternate_behavior_class_ref)
        != Some(12)
        || metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
        || entity.collision.constructor_sound_attachment_id_at_0x8c
            != RetailRuntimeValue::Known(None)
    {
        return Err("class12 authored profile");
    }
    let RetailRuntimeValue::Known(death_sound) = metadata.death_sound_id else {
        return Err("class12 death sound");
    };
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err("class12 context");
    };
    // Styles 4C7930/79C0/7E88/7618/7660 hold null +2C death hooks.
    if !matches!(
        context.active_style().style_address(),
        0x4c7930 | 0x4c79c0 | 0x4c7e88 | 0x4c7618 | 0x4c7660
    ) {
        return Err("class12 death hook");
    }
    let RetailRuntimeValue::Known(Some(mut sub_g)) = entity.sub_g_06070_runtime else {
        return Err("class12 Sub-G");
    };
    let program = audited_behavior_program(12).unwrap();
    let selected = context
        .reselect_audited_type_default(program, 0, COMMON_ACTOR_DYING_ACTIVE_STYLE)
        .ok_or("class12 context")?;
    let prepared = CommonDyingTaskState::prepare_after_allocation(
        id,
        &metadata,
        CommonDyingComponentDescriptors {
            sub_g: RetailRuntimeValue::Known(Some(CommonDyingSubGDescriptorSnapshot {
                source_raw_at_0x00: u32::from_le_bytes(descriptor[0..4].try_into().unwrap()),
                randomized_target_base_raw_at_0x0c: i16::from_le_bytes(
                    descriptor[12..14].try_into().unwrap(),
                ),
            })),
        },
    )
    .map_err(|_| "class12 constructor")?
    .map_task(ActorTaskRuntime::CommonDying);
    let entity = manager.intro2_flyer_entity_mut(id).unwrap();
    let position = entity.position_raw();
    let mut velocity = entity.velocity_raw();
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    if let Some(sound) = death_sound {
        world_fx.emit_fixed_positional_sound_raw(sound, position);
    }
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(selected));
    let policy = initial_behavior_state_policy(program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    let mut prepared = Some(prepared);
    let mut target = 0;
    plan_common_dying_setup(id)
        .apply(
            &mut entity.actor_tasks,
            || u32::from(world_fx.next_shared_retail_random_u16()),
            |effect| match effect {
                CommonDyingConstructorEffect::WriteSubGMode3f { value: 0 } => {}
                CommonDyingConstructorEffect::WriteSubGRandomizedTarget38 {
                    target_raw, ..
                } => target = target_raw,
                CommonDyingConstructorEffect::ResetSubGModeAndAccumulatorThenWriteSource {
                    source_raw,
                    ..
                } => sub_g.apply_shared_06070_sub_g_branch(target, source_raw),
                CommonDyingConstructorEffect::WriteSubGPhase3c { value: 0 } => {}
                CommonDyingConstructorEffect::WriteOwnerVerticalVelocity { velocity_raw } => {
                    velocity[1] = velocity_raw
                }
                // 404120's own 1B970(G, 1) after the +500 write.
                CommonDyingConstructorEffect::WriteSubGMode3f { value: 1 } => sub_g.enter_tumble(),
                _ => unreachable!("authenticated D/G class12 constructor"),
            },
            |_| Ok::<_, ()>(prepared.take().unwrap()),
        )
        .expect("host task allocation and prevalidated suffix");
    entity.sub_g_06070_runtime = RetailRuntimeValue::Known(Some(sub_g));
    entity.set_velocity_raw(velocity);
    let owner = Intro2FlyerSchedulerOwner::adopt_published(entity)
        .map_err(|_| "class12 published graph")?;
    Ok(LiveActorDeathResult {
        returned_nonzero: true,
        publication: Some(owner),
    })
}

/// One `04220` visit. A coarse visit takes `4BE170` (tag 9C01): the terminal
/// stages removal at once. Returns whether the terminal ran.
pub(super) fn tick_dying(
    manager: &mut EntityManager,
    id: u32,
    frame: FlyerTaskFrame<'_>,
) -> Result<bool, Intro2FlyerRuntimeBlock> {
    let entity = manager
        .intro2_flyer_entity_mut(id)
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    // 1EC70 reads only the bilinear bit of the state word.
    let RetailRuntimeValue::Known(state) = entity
        .collision
        .state_flags_at_0x08
        .masked(crate::common_mover::type9_attitude::TERRAIN_ATTITUDE_BILINEAR_STATE_BIT)
    else {
        return Err(Intro2FlyerRuntimeBlock::StateUnavailable);
    };
    let waves_enabled = frame
        .resources
        .level_desc()
        .and_then(|level| level.raw_u32(0x84))
        .ok_or(Intro2FlyerRuntimeBlock::FrameUnavailable)?
        != 0;
    let task_prefix = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |task| {
            let ActorTaskRuntime::CommonDying(state) = task else {
                unreachable!()
            };
            state.before_callback(frame.elapsed_micros)
        })
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let callback_result = if frame.dispatch_mode == CommonMoverDispatchMode::Normal {
        let detailed = (|| {
            let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
                return Err(Intro2FlyerRuntimeBlock::StateUnavailable);
            };
            let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
            // 1EC70 writes pitch/roll; 01430 then runs over the old basis.
            let attitude =
                plan_common_dying_terrain_attitude_raw(CommonDyingTerrainAttitudeInput {
                    terrain: frame.terrain,
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
                    effective_elapsed_micros: frame.elapsed_micros,
                });
            entity.set_rotation_heading_pitch_roll_raw([
                heading,
                attitude.pitch_raw,
                attitude.roll_raw,
            ]);
            // 01430's null target skips the prelude and D; 018A0's G phase
            // in mode 1 writes only its zero force words.
            let RetailRuntimeValue::Known(Some(mut sub_g)) = entity.sub_g_06070_runtime else {
                return Err(Intro2FlyerRuntimeBlock::CommonMover(
                    Intro2FlyerCommonMoverBlock::SubGRuntimeUnavailable,
                ));
            };
            sub_g.advance_tumble_mode().map_err(|error| {
                Intro2FlyerRuntimeBlock::CommonMover(Intro2FlyerCommonMoverBlock::SubG(error))
            })?;
            entity.sub_g_06070_runtime = RetailRuntimeValue::Known(Some(sub_g));
            let mut velocity = entity.velocity_raw();
            velocity[0] = damp_common_dying_axis(velocity[0], frame.elapsed_micros);
            velocity[2] = damp_common_dying_axis(velocity[2], frame.elapsed_micros);
            entity.set_velocity_raw(velocity);
            Ok(())
        })();
        if let Err(error) = detailed {
            entity.actor_tasks.finish_exact_visit(visit);
            return Err(error);
        }
        CommonDyingCallbackResult::Continue
    } else {
        CommonDyingCallbackResult::TaggedOwnerTransition(CommonDyingTaggedResult {
            singleton_address: COMMON_DYING_SCHEDULER_SINGLETON_ADDRESS,
            tag: COMMON_DYING_OWNER_TRANSITION_TAG,
        })
    };
    if !entity.actor_tasks.finish_exact_visit(visit) {
        return Err(Intro2FlyerRuntimeBlock::TaskUnavailable);
    }
    let RetailRuntimeValue::Known(suppressed) = entity
        .collision
        .state_flags_at_0x08
        .masked(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT)
    else {
        return Err(Intro2FlyerRuntimeBlock::StateUnavailable);
    };
    let terminal = matches!(
        common_dying_after_unwind(task_prefix, callback_result),
        CommonDyingAfterUnwindOutcome::RequestOwnerTransition { .. }
    ) && suppressed == 0;
    if terminal {
        crate::common_dying_live::stage_common_dying_terminal_transition(entity);
        manager.queue_actor_deferred_destroy(id);
    }
    Ok(terminal)
}

/// Type41's task walk: Primary (with `C690`), then the freshly read
/// Secondary and Tertiary, or class12's `04220` alone.
pub(super) fn tick_tasks(
    manager: &mut EntityManager,
    owner: Intro2FlyerSchedulerOwner,
    frame: FlyerTaskFrame<'_>,
    world_fx: &mut WorldFx,
    next_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Intro2FlyerSchedulerOwnerTick {
    let entity_id = owner.entity_id;
    let visit = |reselected, infection_write, terminal| Intro2FlyerSchedulerOwnerTick {
        outcome: Intro2FlyerSchedulerProductionOutcome::Type41Visit {
            entity_id,
            reselected,
            infection_write,
            terminal,
        },
        retained_owner: (!terminal).then_some(owner),
    };
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .unwrap();
    if dying_graph_authenticates(entity) {
        return match tick_dying(manager, entity_id, frame) {
            Ok(terminal) => visit(false, None, terminal),
            Err(reason) => park_flyer(owner, reason),
        };
    }
    let aimless = intro2_flyer_aimless_graph_authenticates(entity);
    let acquiring = acquiring_graph_authenticates(entity);
    let wandering = wander_graph_authenticates(entity);
    let fleeing = fleeing_graph_authenticates(entity);
    let defecating = type14::defecate_graph_authenticates(entity);
    if !aimless && !acquiring && !wandering && !fleeing && !defecating {
        return Intro2FlyerSchedulerOwnerTick {
            outcome: Intro2FlyerSchedulerProductionOutcome::Dropped {
                entity_id,
                reason: Intro2FlyerSchedulerProductionDrop::GraphMismatch,
            },
            retained_owner: None,
        };
    }
    let primary = if aimless || acquiring {
        tick_flyer_shared_retarget(
            manager.intro2_flyer_entity_mut(entity_id).unwrap(),
            frame,
            &mut || next_random(world_fx),
        )
        .map(|visit| {
            (
                matches!(visit.post_unwind, SharedRetargetPostUnwind::Transition(_)),
                visit.sound,
            )
        })
    } else if wandering {
        tick_wander(
            manager.intro2_flyer_entity_mut(entity_id).unwrap(),
            frame,
            &mut || next_random(world_fx),
        )
    } else if fleeing {
        tick_flee(manager, entity_id, frame, world_fx)
    } else {
        type14::tick_defecate_wander(
            manager.intro2_flyer_entity_mut(entity_id).unwrap(),
            frame,
            &mut || next_random(world_fx),
        )
    };
    let (transition_due, sound) = match primary {
        Ok(primary) => primary,
        Err(reason) => return park_flyer(owner, reason),
    };
    if let Some(sound) = sound {
        world_fx.queue_networked_positional_sound_raw_at_rate(
            sound.sound_id,
            sound.position_raw,
            sound.rate_q16,
        );
    }
    let mut reselected = false;
    if transition_due {
        match reselect_flyer(
            manager.intro2_flyer_entity_mut(entity_id).unwrap(),
            frame.metadata,
            frame.retail_tick,
            &mut || next_random(world_fx),
        ) {
            Ok(applied) => reselected = applied,
            Err(reason) => return park_flyer(owner, reason),
        }
    }
    // Slots 1 and 2 are read fresh after Primary and C690: a graph C690 just
    // published runs its Secondary or Tertiary in this same pass.
    if manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .is_some_and(acquiring_graph_authenticates)
    {
        if let Err(reason) = acquire(manager, entity_id, frame.metadata, world_fx) {
            return park_flyer(owner, reason);
        }
    }
    let entity = manager.intro2_flyer_entity_mut(entity_id).unwrap();
    let mut infection_write = None;
    if type14::defecate_graph_authenticates(entity) {
        match type14::tick_virus(entity, frame, world_fx, next_random) {
            Ok(write) => infection_write = write,
            Err(reason) => return park_flyer(owner, reason),
        }
    }
    visit(reselected, infection_write, false)
}

#[cfg(test)]
mod tests;
