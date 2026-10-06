use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    aim_and_fire::{AimAndFireFrameOutcome, AimAndFireTransitionReason},
    entity::{EntityConstructionResources, EntityManager},
    entity_behavior::{BehaviorContextRuntime, BehaviorDescriptorIdentity},
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord, DYING_STATE_BIT,
    },
    gameplay_notifications::GameplayNotifications,
    generic_projectile_emitter::GenericEmitterSpeedField,
    intro2_type13_aim::{
        drain_live_intro2_type13_shot_queues, tick_intro2_type13_aim, Type13AimError,
        Type13AimTickOutcome,
    },
    intro2_type13_live::{
        tick_intro2_type13_class5_primary, tick_intro2_type13_primary,
        tick_intro2_type13_scheduler_owner, tick_intro2_type13_scheduler_owner_with_random,
        Intro2Type13C690Block, Intro2Type13C690Transition, Intro2Type13ChaseVisitResult,
        Intro2Type13PrimaryFrame, Intro2Type13PrimaryVisitBlock, Intro2Type13PrimaryVisitResult,
        Intro2Type13PursuingC690Attempt, Intro2Type13PursuingC690Reason,
        Intro2Type13PursuingC690Request, Intro2Type13SchedulerOwner,
        Intro2Type13SchedulerProductionDrop, Intro2Type13SchedulerProductionOutcome,
        INTRO2_TYPE13_MODEL_ID, INTRO2_TYPE13_POSITION_RAW, INTRO2_TYPE13_SPAWN_INDEX,
        TYPE13_ENTITY_TYPE,
    },
    intro2_type47_live::INTRO2_TYPE47_SPAWN_INDICES,
    opening::intro2_uses_live_actor_pose,
    projectile_emitter::{FIRST_WORLD_SHOOTER_PARTICLE_CLASS, TYPE13_PROJECTILE_PARTICLE_CLASS},
    search_attack::SEARCH_ATTACK_BEHAVIOR_CLASS_ID,
    search_attack_live::{
        SearchAttackLiveAcquisitionOutcome, TYPE13_SEARCH_ATTACK_PROJECTILE_DESCRIPTOR,
    },
    session::GameSession,
    shared_retarget_mover::{
        SharedRetarget, SharedRetargetPostUnwind, SharedRetargetTransitionReason,
        SharedRetargetTrigger, SHARED_RETARGET_RADIUS_RAW,
    },
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler,
    },
    sub_g_runtime::Type13SubGFrameBlock,
    type13_c690::{Type13C690ReselectionBlock, TYPE13_C690_SUPPRESS_STATE_BIT},
    type13_common_mover::GklCommonMoverBlock,
    type13_initial_behavior::{
        apply_type13_published_class7_acquisition_live, Type13Class7AcquiringError,
        Type13InitialBehaviorPublication,
    },
    world_fx::{ParticleEnvironment, WorldFx},
};

#[path = "intro2_type13_publication/aim_basis.rs"]
mod aim_basis;
#[path = "intro2_type13_publication/aim_targets.rs"]
mod aim_targets;
#[path = "intro2_type13_publication/birth.rs"]
mod birth;
#[path = "intro2_type13_publication/callback_failure.rs"]
mod callback_failure;
#[path = "intro2_type13_publication/coarse_dispatch.rs"]
mod coarse_dispatch;
#[path = "intro2_type13_publication/post_task_basis.rs"]
mod post_task_basis;
#[path = "intro2_type13_publication/world_update.rs"]
mod world_update;

fn intro2_type_metadata(session: &GameSession) -> Vec<EntityTypeRuntimeMetadata> {
    session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(entity_type, model_slots)| {
            session
                .cache
                .global_entity_type(entity_type)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..EntityTypeRuntimeMetadata::default()
                })
        })
        .collect()
}

fn primary_frame(session: &GameSession, elapsed_micros: u32) -> Intro2Type13PrimaryFrame<'_> {
    Intro2Type13PrimaryFrame {
        dispatch_mode: v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
        terrain: session.cache.level_terrain().expect("Intro2 Section 10"),
        active_model_extent_raw: session
            .cache
            .global_model(INTRO2_TYPE13_MODEL_ID)
            .expect("Ptersect model 291")
            .radius,
        attached_cargo_mass: 0,
        elapsed_micros,
        global_elapsed_micros: elapsed_micros,
        retail_tick: 0,
    }
}

fn silence_non_owner_peers(manager: &mut EntityManager, owner_id: u32) {
    let other_ids: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.id != owner_id)
        .map(|entity| entity.id)
        .collect();
    for entity_id in other_ids {
        let peer = manager.entity_mut(entity_id).expect("captured peer");
        peer.active = false;
        peer.collision.state_flags_at_0x08 = RetailStateWord::exact(0);
    }
}

fn captured_intro2_type13(
    session: &GameSession,
) -> (
    Vec<EntityTypeRuntimeMetadata>,
    EntityManager,
    Intro2Type13SchedulerOwner,
) {
    let type_metadata = intro2_type_metadata(session);
    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection {
            type13: v2k_game::intro2_type13_live::Intro2Type13BirthSelection::CapturedClass7,
            ..Default::default()
        },
        &mut || 0,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");
    let owner = Intro2Type13SchedulerOwner::adopt_published(
        manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
            .expect("published type-13"),
    )
    .expect("published B6C0 graph");
    silence_non_owner_peers(&mut manager, owner.entity_id());
    (type_metadata, manager, owner)
}

fn captured_intro2_type13_pursuing(
    session: &GameSession,
) -> (EntityManager, Intro2Type13SchedulerOwner) {
    let type_metadata = intro2_type_metadata(session);
    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection {
            type13: v2k_game::intro2_type13_live::Intro2Type13BirthSelection::CapturedClass7,
            ..Default::default()
        },
        &mut || 0,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");
    let owner = Intro2Type13SchedulerOwner::adopt_published(
        manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
            .expect("published type-13"),
    )
    .expect("published B6C0 graph");
    let first = tick_intro2_type13_scheduler_owner(
        &mut manager,
        owner,
        &mut WorldFx::new(),
        Some(primary_frame(session, 20_000)),
    );
    assert!(matches!(
        first.outcome,
        Intro2Type13SchedulerProductionOutcome::B6c0Visit {
            acquisition: SearchAttackLiveAcquisitionOutcome::Applied { .. },
            ..
        }
    ));
    (
        manager,
        first
            .retained_owner
            .expect("ADE0 retains the pursuing owner"),
    )
}

fn make_pursuing_chase_target_inactive(manager: &mut EntityManager, owner_id: u32) {
    let target_id = match manager
        .iter_all()
        .find(|entity| entity.id == owner_id)
        .and_then(|entity| entity.actor_task_state(ActorTaskSlot::Primary))
    {
        Some(ActorTaskRuntime::ChaseTarget(chase)) => chase.target_id(),
        other => panic!("pursuing Primary must be Chase, got {other:?}"),
    };
    let target = manager.entity_mut(target_id).expect("pursuing target");
    target.active = false;
    target.collision.state_flags_at_0x08 = RetailStateWord::exact(0);
}

fn tick_pursuing_aim_out_of_band(
    session: &GameSession,
    manager: &mut EntityManager,
    owner_id: u32,
    elapsed_micros: u32,
) -> Type13AimTickOutcome {
    let metadata = intro2_type_metadata(session);
    tick_intro2_type13_aim(
        v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
        manager,
        &mut WorldFx::new(),
        owner_id,
        elapsed_micros,
        metadata.get(TYPE13_ENTITY_TYPE as usize),
    )
    .expect("out-of-band Aim visit")
}

fn prime_distant_shared_retarget(
    session: &GameSession,
    manager: &mut EntityManager,
    owner: Intro2Type13SchedulerOwner,
) -> Intro2Type13SchedulerOwner {
    let mut words = [0xffff_u32, 0xffff].into_iter();
    let prime = tick_intro2_type13_scheduler_owner_with_random(
        manager,
        owner,
        &mut WorldFx::new(),
        Some(primary_frame(session, 0)),
        &mut |_| words.next().expect("two retarget words"),
    );
    assert!(words.next().is_none());
    prime
        .retained_owner
        .expect("distant retarget retains the owner")
}

fn reselect_class5_from_b6c0(
    session: &GameSession,
    manager: &mut EntityManager,
    owner: Intro2Type13SchedulerOwner,
) -> Intro2Type13SchedulerOwner {
    let owner = prime_distant_shared_retarget(session, manager, owner);
    let mut words = [1_u32, 0x3fff, 0x1234].into_iter();
    let transition = tick_intro2_type13_scheduler_owner_with_random(
        manager,
        owner,
        &mut WorldFx::new(),
        Some(Intro2Type13PrimaryFrame {
            dispatch_mode:
                v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
            retail_tick: 2,
            global_elapsed_micros: 501_000,
            ..primary_frame(session, 501_000)
        }),
        &mut |_| words.next().expect("class-5 C690 suffix"),
    );
    assert!(words.next().is_none());
    match transition.outcome {
        Intro2Type13SchedulerProductionOutcome::C690Transition {
            transition:
                Intro2Type13C690Transition::Published(
                    Type13InitialBehaviorPublication::MoveAboutAimlessly(_),
                ),
            acquisition: None,
            ..
        } => {}
        other => panic!("B6C0 timeout must publish class 5, got {other:?}"),
    }
    transition
        .retained_owner
        .expect("class-5 publication retains the scheduler owner")
}

#[v2k_test_support::retail_test]
fn captured_intro2_route_commits_spawn0_b6c0_normal_sub_g() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");

    let type_metadata = intro2_type_metadata(&session);
    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let direct = EntityManager::from_level_with_type_metadata(level, &type_metadata, resources);
    let entity = direct
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
        .expect("Intro2 authored type-13 spawn");
    assert_eq!(entity.entity_type, TYPE13_ENTITY_TYPE);
    assert_eq!(entity.position_raw(), INTRO2_TYPE13_POSITION_RAW);
    assert_eq!(entity.model_slots, [Some(INTRO2_TYPE13_MODEL_ID); 4]);
    assert_eq!(entity.initial_behavior, RetailRuntimeValue::Unresolved);
    assert_eq!(
        entity.current_behavior_context,
        RetailRuntimeValue::Unresolved
    );
    for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        assert_eq!(entity.actor_task_state(slot), None);
    }

    let mut captured = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection {
            type13: v2k_game::intro2_type13_live::Intro2Type13BirthSelection::CapturedClass7,
            ..Default::default()
        },
        &mut || 0,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");

    let entity = captured
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
        .expect("captured Intro2 type-13 spawn");
    assert_eq!(entity.entity_type, TYPE13_ENTITY_TYPE);
    let RetailRuntimeValue::Known(Some(selection)) = entity.initial_behavior else {
        panic!("captured route must publish the class-7 B6C0 selection")
    };
    assert_eq!(selection.choice_index, 1);
    assert_eq!(selection.program.class_id, SEARCH_ATTACK_BEHAVIOR_CLASS_ID);
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::SharedRetarget(wander)) if wander.lifetime_ms() == 500
    ));
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Secondary),
        Some(ActorTaskRuntime::TargetAcquisition(_))
    ));
    assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
    assert!(entity.intro2_type26_sub_d_frame_owner.is_none());
    assert!(entity.intro2_type47_sub_d_frame_owner.is_none());
    assert!(intro2_uses_live_actor_pose(entity));

    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_type13_search_attack(&captured), 1);
    let mut empty = SpecializedActorTaskScheduler::new();
    assert_eq!(empty.adopt_intro2_type13_search_attack(&direct), 0);
    assert_eq!(empty.adopt_intro2_type47_guards(&direct), 0);

    let owner = Intro2Type13SchedulerOwner::adopt_published(
        captured
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
            .expect("published type-13"),
    )
    .expect("published B6C0 graph");
    let entity_before = captured
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("published type-13 owner");
    assert_eq!(entity_before.mass_raw, 100);
    assert_eq!(entity_before.capability_flags, 8);
    assert_eq!(entity_before.model_index, Some(INTRO2_TYPE13_MODEL_ID));
    assert_eq!(
        session
            .cache
            .global_model(INTRO2_TYPE13_MODEL_ID)
            .expect("Ptersect model 291")
            .radius,
        480
    );
    assert!(matches!(
        entity_before.sub_j_attachment_runtime,
        RetailRuntimeValue::Known(None)
    ));
    let position_before = entity_before.position_raw();
    let velocity_before = entity_before.velocity_raw();
    let rotation_before = entity_before.rotation_heading_pitch_roll_raw();
    let basis_before = entity_before.physical_body_basis_q31();
    let sub_g_before = entity_before.sub_g_06070_runtime;

    let mut world_fx = WorldFx::new();
    let tick = tick_intro2_type13_scheduler_owner(
        &mut captured,
        owner,
        &mut world_fx,
        Some(primary_frame(&session, 20_000)),
    );
    match tick.outcome {
        Intro2Type13SchedulerProductionOutcome::B6c0Visit { primary, .. } => {
            assert_eq!(primary.result, Intro2Type13PrimaryVisitResult::Continue);
            assert_eq!(primary.sound, None);
        }
        other => panic!("expected B6C0 visit, got {other:?}"),
    }
    assert!(
        tick.retained_owner.is_some(),
        "captured ADE0 retains the pursuing Chase/Aim graph while component allocations survive",
    );

    let entity_after = captured
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("surviving type-13 owner");
    assert_eq!(entity_after.position_raw(), position_before);
    assert_eq!(entity_after.velocity_raw(), velocity_before);
    assert_ne!(
        entity_after.rotation_heading_pitch_roll_raw(),
        rotation_before
    );
    let [heading, pitch, roll] = entity_after.rotation_heading_pitch_roll_raw();
    assert_eq!(
        entity_after.physical_body_basis_q31(),
        RetailRuntimeValue::Known(
            v2k_game::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(
                heading, pitch, roll,
            ),
        ),
    );
    assert_ne!(entity_after.physical_body_basis_q31(), basis_before);
    assert_ne!(entity_after.sub_g_06070_runtime, sub_g_before);
    assert_eq!(
        entity_after
            .intro2_type13_common_mover_runtime
            .expect("entity-owned common-mover allocation survives acquisition")
            .sub_d_frame_owner
            .classifier_cache()
            .stagger_counter(),
        1
    );
    let RetailRuntimeValue::Known(Some(sub_g_after)) = entity_after.sub_g_06070_runtime else {
        panic!("normal Sub-G commit must retain its entity-owned runtime");
    };
    assert!(matches!(
        sub_g_after.rate_raw_at_0x34(),
        RetailRuntimeValue::Known(50..=130)
    ));
    let RetailRuntimeValue::Known(outputs) = sub_g_after.animation_outputs_raw() else {
        panic!("normal Sub-G must publish all seven entity-bank outputs");
    };
    let vars = entity_after.presentation_anim_vars(0);
    for (selector, output) in
        v2k_game::search_attack_live::TYPE13_SEARCH_ATTACK_SUB_G_ANIMATION_BINDINGS
            .into_iter()
            .zip(outputs)
    {
        assert_eq!(vars.dynamic[selector], i32::from(output));
    }
    assert!(intro2_uses_live_actor_pose(entity_after));

    let mut primary_captured = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection {
            type13: v2k_game::intro2_type13_live::Intro2Type13BirthSelection::CapturedClass7,
            ..Default::default()
        },
        &mut || 0,
    )
    .expect("independent captured fixture for the direct primary visit");
    let primary_owner = Intro2Type13SchedulerOwner::adopt_published(
        primary_captured
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
            .expect("independently published type-13"),
    )
    .expect("independently published B6C0 graph");
    let entity_before_primary = primary_captured
        .iter_all()
        .find(|entity| entity.id == primary_owner.entity_id())
        .expect("independent type-13 owner");
    let primary_position_before = entity_before_primary.position_raw();
    let primary_velocity_before = entity_before_primary.velocity_raw();
    let primary_rotation_before = entity_before_primary.rotation_heading_pitch_roll_raw();
    let primary_basis_before = entity_before_primary.physical_body_basis_q31();
    let primary_sub_g_before = entity_before_primary.sub_g_06070_runtime;
    let primary_before = entity_before_primary
        .actor_task_state(ActorTaskSlot::Primary)
        .copied();
    let primary_only = tick_intro2_type13_primary(
        primary_captured
            .entity_mut(primary_owner.entity_id())
            .expect("independent type-13 owner"),
        type_metadata.get(TYPE13_ENTITY_TYPE as usize),
        primary_frame(&session, 20_000),
        &mut || 0,
    )
    .expect("published primary callback");
    assert_eq!(
        primary_only.result,
        Intro2Type13PrimaryVisitResult::Continue
    );
    assert_eq!(primary_only.post_unwind, SharedRetargetPostUnwind::Continue);
    let entity_after_primary = primary_captured
        .iter_all()
        .find(|entity| entity.id == primary_owner.entity_id())
        .expect("surviving independent type-13 owner");
    let RetailRuntimeValue::Known(Some(first_sub_g)) = entity_after_primary.sub_g_06070_runtime
    else {
        panic!("first primary visit must retain Sub-G state");
    };
    let RetailRuntimeValue::Known(first_phases) = first_sub_g.table_bytes_at_0x28() else {
        panic!("first primary visit must retain oscillator phases");
    };
    let RetailRuntimeValue::Known(first_rate_raw) = first_sub_g.rate_raw_at_0x34() else {
        panic!("first primary visit must retain its derived rate");
    };
    assert_eq!(
        entity_after_primary
            .intro2_type13_common_mover_runtime
            .expect("entity-owned common-mover allocation")
            .sub_d_frame_owner
            .classifier_cache()
            .stagger_counter(),
        1
    );
    assert_eq!(entity_after_primary.position_raw(), primary_position_before);
    assert_eq!(entity_after_primary.velocity_raw(), primary_velocity_before);
    assert_ne!(
        entity_after_primary.rotation_heading_pitch_roll_raw(),
        primary_rotation_before
    );
    assert_eq!(
        entity_after_primary.physical_body_basis_q31(),
        primary_basis_before
    );
    assert_ne!(
        entity_after_primary.sub_g_06070_runtime,
        primary_sub_g_before
    );
    let Some(ActorTaskRuntime::SharedRetarget(before_state)) = primary_before else {
        panic!("published primary state changed shape before its visit");
    };
    let Some(ActorTaskRuntime::SharedRetarget(after_state)) = entity_after_primary
        .actor_task_state(ActorTaskSlot::Primary)
        .copied()
    else {
        panic!("successful primary visit must retain its SharedRetarget task");
    };
    assert_eq!(
        after_state.private_state().target_position_raw,
        match primary_only.prefix.retarget {
            v2k_game::shared_retarget_mover::SharedRetarget::Retained => {
                before_state.private_state().target_position_raw
            }
            v2k_game::shared_retarget_mover::SharedRetarget::Replaced {
                target_position_raw,
                ..
            } => target_position_raw,
        }
    );
    assert_eq!(after_state.elapsed_ms(), before_state.elapsed_ms() + 20);
    assert_eq!(after_state.lifetime_ms(), before_state.lifetime_ms());

    // This second call verifies component custody only. The enclosing
    // DCA0/E870 basis refresh remains a separate production boundary.
    let second_primary = tick_intro2_type13_primary(
        primary_captured
            .entity_mut(primary_owner.entity_id())
            .expect("independent type-13 owner on its second visit"),
        type_metadata.get(TYPE13_ENTITY_TYPE as usize),
        Intro2Type13PrimaryFrame {
            dispatch_mode:
                v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
            global_elapsed_micros: 20_000,
            retail_tick: 1,
            ..primary_frame(&session, 20_000)
        },
        &mut || 0,
    )
    .expect("second published primary callback");
    assert_eq!(
        second_primary.result,
        Intro2Type13PrimaryVisitResult::Continue
    );
    assert_eq!(
        second_primary.post_unwind,
        SharedRetargetPostUnwind::Continue
    );
    let second_entity = primary_captured
        .iter_all()
        .find(|entity| entity.id == primary_owner.entity_id())
        .expect("independent type-13 owner after its second visit");
    assert_eq!(
        second_entity
            .intro2_type13_common_mover_runtime
            .expect("entity-owned common-mover allocation after two visits")
            .sub_d_frame_owner
            .classifier_cache()
            .stagger_counter(),
        2,
        "the entity-owned common-mover allocation must survive between visits"
    );
    let RetailRuntimeValue::Known(Some(second_sub_g)) = second_entity.sub_g_06070_runtime else {
        panic!("second primary visit must retain Sub-G state");
    };
    let RetailRuntimeValue::Known(second_phases) = second_sub_g.table_bytes_at_0x28() else {
        panic!("second primary visit must retain oscillator phases");
    };
    let expected_phase_step = first_rate_raw.wrapping_mul((20_000u32 >> 12) as i32) >> 6;
    assert_eq!(
        second_phases[0],
        first_phases[0].wrapping_add(expected_phase_step as u8),
        "AA60 must advance from the prior frame's entity-owned rate"
    );
    let Some(ActorTaskRuntime::SharedRetarget(second_state)) = second_entity
        .actor_task_state(ActorTaskSlot::Primary)
        .copied()
    else {
        panic!("second primary visit must retain its SharedRetarget task");
    };
    assert_eq!(second_state.elapsed_ms(), before_state.elapsed_ms() + 40);

    let common_before_block = second_entity.intro2_type13_common_mover_runtime;
    let sub_g_before_block = second_entity.sub_g_06070_runtime;
    let rotation_before_block = second_entity.rotation_heading_pitch_roll_raw();
    let velocity_before_block = second_entity.velocity_raw();
    primary_captured
        .entity_mut(primary_owner.entity_id())
        .expect("type-13 owner before blocked visit")
        .capability_flags |= 1;
    let blocked = tick_intro2_type13_primary(
        primary_captured
            .entity_mut(primary_owner.entity_id())
            .expect("type-13 owner for blocked visit"),
        type_metadata.get(TYPE13_ENTITY_TYPE as usize),
        Intro2Type13PrimaryFrame {
            dispatch_mode:
                v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
            global_elapsed_micros: 20_000,
            retail_tick: 2,
            ..primary_frame(&session, 20_000)
        },
        &mut || 0,
    )
    .expect("evidence block is a detached callback result");
    assert!(matches!(
        blocked.result,
        Intro2Type13PrimaryVisitResult::CommonMoverBlocked(GklCommonMoverBlock::SubG(
            Type13SubGFrameBlock::PlayerProjectionUnsupported
        ))
    ));
    assert_eq!(
        blocked.post_unwind,
        SharedRetargetPostUnwind::UnresolvedCommonMover
    );
    let blocked_entity = primary_captured
        .iter_all()
        .find(|entity| entity.id == primary_owner.entity_id())
        .expect("type-13 owner after blocked visit");
    let Some(ActorTaskRuntime::SharedRetarget(blocked_task)) =
        blocked_entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("blocked callback retains SharedRetarget");
    };
    assert_eq!(blocked_task.elapsed_ms(), second_state.elapsed_ms() + 20);
    if let SharedRetarget::Replaced {
        target_position_raw,
        ..
    } = blocked.prefix.retarget
    {
        assert_eq!(
            blocked_task.private_state().target_position_raw,
            target_position_raw
        );
    }
    assert_eq!(
        blocked_entity.intro2_type13_common_mover_runtime,
        common_before_block
    );
    assert_eq!(blocked_entity.sub_g_06070_runtime, sub_g_before_block);
    assert_eq!(
        blocked_entity.rotation_heading_pitch_roll_raw(),
        rotation_before_block
    );
    assert_eq!(blocked_entity.velocity_raw(), velocity_before_block);

    let primary_before_scheduler_block = blocked_entity
        .actor_task_state(ActorTaskSlot::Primary)
        .copied();
    let secondary_before_scheduler_block = blocked_entity
        .actor_task_state(ActorTaskSlot::Secondary)
        .copied();
    let context_before_scheduler_block = blocked_entity.current_behavior_context;
    let mut blocked_world_fx = WorldFx::new();
    let scheduler_blocked = tick_intro2_type13_scheduler_owner(
        &mut primary_captured,
        primary_owner,
        &mut blocked_world_fx,
        Some(Intro2Type13PrimaryFrame {
            dispatch_mode:
                v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
            retail_tick: 3,
            ..primary_frame(&session, 20_000)
        }),
    );
    match scheduler_blocked.outcome {
        Intro2Type13SchedulerProductionOutcome::PrimaryBlocked { primary, .. } => {
            assert!(matches!(
                primary.result,
                Intro2Type13PrimaryVisitResult::CommonMoverBlocked(GklCommonMoverBlock::SubG(
                    Type13SubGFrameBlock::PlayerProjectionUnsupported
                ))
            ));
            assert_eq!(
                primary.post_unwind,
                SharedRetargetPostUnwind::UnresolvedCommonMover
            );
            assert_eq!(primary.sound, None);
        }
        other => panic!("expected a Primary-only evidence block, got {other:?}"),
    }
    assert!(scheduler_blocked.retained_owner.is_some());
    assert_ne!(scheduler_blocked.retained_owner, Some(primary_owner));
    let after_scheduler_block = primary_captured
        .iter_all()
        .find(|entity| entity.id == primary_owner.entity_id())
        .expect("type-13 owner after scheduler-level block");
    let Some(ActorTaskRuntime::SharedRetarget(before_block)) = primary_before_scheduler_block
    else {
        panic!("SharedRetarget before scheduler block");
    };
    let Some(ActorTaskRuntime::SharedRetarget(after_block)) =
        after_scheduler_block.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("SharedRetarget after scheduler block");
    };
    assert_eq!(after_block.elapsed_ms(), before_block.elapsed_ms() + 20);
    assert_eq!(
        after_scheduler_block
            .actor_task_state(ActorTaskSlot::Secondary)
            .copied(),
        secondary_before_scheduler_block,
        "a blocked Primary must stop the retail slot walk before acquisition"
    );
    assert_eq!(
        after_scheduler_block.current_behavior_context,
        context_before_scheduler_block
    );
    blocked_world_fx.process_pending();
    assert!(blocked_world_fx.take_positional_sounds().is_empty());

    let type47_published = captured
        .iter_all()
        .filter(|entity| {
            entity
                .authored_spawn_index
                .is_some_and(|index| INTRO2_TYPE47_SPAWN_INDICES.contains(&index))
        })
        .count();
    assert_eq!(type47_published, INTRO2_TYPE47_SPAWN_INDICES.len());
}

#[v2k_test_support::retail_test]
fn ade0_retains_pursuing_owner_and_commits_chase_01430() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let type_metadata = intro2_type_metadata(&session);
    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection {
            type13: v2k_game::intro2_type13_live::Intro2Type13BirthSelection::CapturedClass7,
            ..Default::default()
        },
        &mut || 0,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");
    let owner = Intro2Type13SchedulerOwner::adopt_published(
        manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
            .expect("published type-13"),
    )
    .expect("published B6C0 graph");

    let mut world_fx = WorldFx::new();
    let first = tick_intro2_type13_scheduler_owner(
        &mut manager,
        owner,
        &mut world_fx,
        Some(primary_frame(&session, 20_000)),
    );
    let ade0_queued = match &first.outcome {
        Intro2Type13SchedulerProductionOutcome::B6c0Visit {
            acquisition, aim, ..
        } => {
            assert!(
                matches!(
                    acquisition,
                    SearchAttackLiveAcquisitionOutcome::Applied {
                        same_pass_aim: None,
                        ..
                    }
                ),
                "Type-13 ADE0 must skip generic without_emitter, got {acquisition:?}"
            );
            assert_type13_aim_24650(aim.as_ref())
        }
        other => panic!("expected captured ADE0, got {other:?}"),
    };
    let owner = first
        .retained_owner
        .expect("ADE0 must keep the pursuing owner");

    let entity_before = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("pursuing type-13");
    assert!(matches!(
        entity_before.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::ChaseTarget(_))
    ));
    assert!(entity_before
        .actor_task_state(ActorTaskSlot::Secondary)
        .is_none());
    let Some(ActorTaskRuntime::AimAndFire(aim_before)) =
        entity_before.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!("ADE0 publishes Aim");
    };
    assert_eq!(
        aim_before.elapsed_ms(),
        20,
        "same-pass Aim visits newborn Tertiary once"
    );
    let queued_before = entity_before
        .intro2_type13_aim_runtime
        .as_ref()
        .map(|runtime| runtime.queued_shot_count())
        .unwrap_or(0);
    assert_eq!(queued_before, ade0_queued);
    assert_type13_method10_runtime(&entity_before, queued_before, ade0_queued, &mut world_fx);
    let position_before = entity_before.position_raw();
    let rotation_before = entity_before.rotation_heading_pitch_roll_raw();
    let sub_g_before = entity_before.sub_g_06070_runtime;
    let stagger_before = entity_before
        .intro2_type13_common_mover_runtime
        .expect("entity-owned common-mover allocation")
        .sub_d_frame_owner
        .classifier_cache()
        .stagger_counter();

    let mut world_fx = WorldFx::new();
    let chase = tick_intro2_type13_scheduler_owner(
        &mut manager,
        owner,
        &mut world_fx,
        Some(Intro2Type13PrimaryFrame {
            dispatch_mode:
                v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
            retail_tick: 1,
            global_elapsed_micros: 20_000,
            ..primary_frame(&session, 20_000)
        }),
    );
    let queued_shots_added = match chase.outcome {
        Intro2Type13SchedulerProductionOutcome::PursuingVisit {
            chase: Some(chase),
            post_chase_plus00_c690: None,
            post_aim_plus04_c690: None,
            post_aim_plus00_c690: None,
            aim,
            ..
        } => {
            assert_eq!(chase.result, Intro2Type13ChaseVisitResult::Continue);
            assert_type13_aim_24650(aim.as_ref())
        }
        other => panic!("expected Type-13 Chase 01430, got {other:?}"),
    };
    assert!(chase.retained_owner.is_some());

    let entity_after = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("pursuing type-13 after Chase");
    assert_eq!(entity_after.position_raw(), position_before);
    assert_ne!(
        entity_after.rotation_heading_pitch_roll_raw(),
        rotation_before
    );
    assert_ne!(entity_after.sub_g_06070_runtime, sub_g_before);
    assert_eq!(
        entity_after
            .intro2_type13_common_mover_runtime
            .expect("entity-owned common-mover allocation")
            .sub_d_frame_owner
            .classifier_cache()
            .stagger_counter(),
        stagger_before.wrapping_add(1)
    );
    let Some(ActorTaskRuntime::ChaseTarget(state)) =
        entity_after.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("Chase remains after 01430");
    };
    assert_eq!(state.elapsed_ms(), 20);
    let Some(ActorTaskRuntime::AimAndFire(aim_after)) =
        entity_after.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!("Aim remains after later FUN_00424650");
    };
    assert_eq!(
        aim_after.elapsed_ms(),
        40,
        "ADE0 then later Aim each visit slot 2 once"
    );

    let mut queued_total = queued_before + queued_shots_added;
    assert_type13_method10_runtime(
        &entity_after,
        queued_total,
        queued_shots_added,
        &mut world_fx,
    );

    if queued_total == 0 {
        // Cadence alone does not make the captured target lie inside the
        // retained post-task firing cone. Place this fixture target there;
        // the next Chase can change angles, but same-pass Aim still consumes
        // this matrix before the enclosing F70 refresh. The lateral offset
        // avoids the exact-collinear Q31 dot-overflow boundary.
        let target_id = state.target_id();
        let RetailRuntimeValue::Known(basis) = entity_after.physical_body_basis_q31() else {
            panic!("completed Chase retains its body basis");
        };
        let position = entity_after.position_raw();
        let target_position = std::array::from_fn(|axis| {
            position[axis].wrapping_add(
                ((i64::from(basis.forward[axis]) * 2048 + i64::from(basis.lateral[axis]) * 512)
                    >> 31) as i16,
            )
        });
        manager
            .entity_mut(target_id)
            .expect("retained Chase target")
            .set_motion_raw(target_position, [0; 3]);
        let owner = chase
            .retained_owner
            .expect("later Aim retains the pursuing owner");
        let cadence = tick_intro2_type13_scheduler_owner(
            &mut manager,
            owner,
            &mut world_fx,
            Some(Intro2Type13PrimaryFrame {
                dispatch_mode:
                    v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
                retail_tick: 2,
                global_elapsed_micros: 600_000,
                ..primary_frame(&session, 600_000)
            }),
        );
        queued_total = match cadence.outcome {
            Intro2Type13SchedulerProductionOutcome::PursuingVisit { aim, .. } => {
                assert_type13_aim_24650(aim.as_ref())
            }
            other => panic!("cadence visit must remain pursuing, got {other:?}"),
        };
    }

    assert!(
        queued_total > 0,
        "method-10 cadence must queue a class-38 transient"
    );
    let drained = drain_live_intro2_type13_shot_queues(
        &mut manager,
        &mut world_fx,
        ParticleEnvironment::Dry,
        0,
    );
    assert_eq!(drained.len(), 1);
    let drain = drained[0].1.as_ref().expect("queued Type-13 shots drain");
    assert_eq!(drain.consumed_requests, queued_total);
    assert_eq!(drain.materialized_particle_classes.len(), queued_total);
    assert!(
        drain
            .materialized_particle_classes
            .iter()
            .all(|&class| class == TYPE13_PROJECTILE_PARTICLE_CLASS),
        "method 10 must materialize class 38, got {:?}",
        drain.materialized_particle_classes
    );
    assert!(
        drain
            .materialized_particle_classes
            .iter()
            .all(|&class| class != FIRST_WORLD_SHOOTER_PARTICLE_CLASS),
        "Type-13 drain must not reuse class 87"
    );
    assert_eq!(world_fx.particle_count(), queued_total);
    let replay = drain_live_intro2_type13_shot_queues(
        &mut manager,
        &mut world_fx,
        ParticleEnvironment::Dry,
        0,
    );
    assert!(replay.is_empty());
    let entity_drained = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("type-13 after drain");
    assert_eq!(
        entity_drained
            .intro2_type13_aim_runtime
            .as_ref()
            .map(|runtime| runtime.queued_shot_count())
            .unwrap_or(0),
        0
    );
}

fn assert_type13_aim_24650(aim: Option<&Result<Type13AimTickOutcome, Type13AimError>>) -> usize {
    match aim {
        Some(Ok(Type13AimTickOutcome {
            resolution,
            queued_shots_added,
        })) => {
            assert!(
                matches!(
                    resolution.outcome,
                    AimAndFireFrameOutcome::Continue
                        | AimAndFireFrameOutcome::ReturnGenericEmitterResult(_)
                        | AimAndFireFrameOutcome::RequestOwnerTransition { .. }
                ),
                "Type-13 Aim must reach FUN_00424650, got {:?}",
                resolution.outcome
            );
            *queued_shots_added
        }
        other => panic!("Type-13 Aim must run FUN_00424650, got {other:?}"),
    }
}

fn assert_type13_method10_runtime(
    entity: &v2k_game::entity::Entity,
    queued_total: usize,
    queued_this_visit: usize,
    world_fx: &mut WorldFx,
) {
    let runtime = entity
        .intro2_type13_aim_runtime
        .as_ref()
        .expect("Type-13 Sub-E sidecar");
    assert_eq!(
        runtime.projectile_descriptor(),
        TYPE13_SEARCH_ATTACK_PROJECTILE_DESCRIPTOR
    );
    assert_eq!(runtime.emitter_runtime().projectile_method, 10);
    assert_eq!(runtime.emitter_runtime().sound_id, 75);
    assert_eq!(runtime.emitter_runtime().direct_mode, 0);
    assert_eq!(runtime.queued_shot_count(), queued_total);
    if queued_total > 0 {
        let shot = runtime.transient_shots()[0];
        assert_eq!(shot.projectile_method, 10);
        assert_eq!(shot.emitter_selector, 0);
        assert_eq!(shot.speed_field, GenericEmitterSpeedField::Explicit(2400));
        assert!(!shot.auxiliary);
    }
    world_fx.process_pending();
    let sounds = world_fx.take_positional_sounds();
    assert!(
        sounds.iter().all(|sound| sound.sound_id != 70),
        "Type-13 Aim must not queue Type-47 sound 70"
    );
    if queued_this_visit > 0 {
        assert!(
            sounds.iter().any(|sound| sound.sound_id == 75),
            "a queued method-10 shot must play sound 75"
        );
    }
}

#[v2k_test_support::retail_test]
fn pursuing_chase_timeout_applies_c690_from_variant1() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    for (selector, suffixes, expected_class) in [
        (0x3fff_u32, vec![0x1234], 5_u8),
        (0x4000_u32, vec![0x1234, 0xabcd], 7_u8),
    ] {
        let type_metadata = intro2_type_metadata(&session);
        let level = session.cache.level_desc().expect("Intro2 Section 13");
        let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
        let resources = EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&model_extent),
        };
        let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
            level,
            &type_metadata,
            resources,
            v2k_game::entity::Intro2BirthSelection {
                type13: v2k_game::intro2_type13_live::Intro2Type13BirthSelection::CapturedClass7,
                ..Default::default()
            },
            &mut || 0,
        )
        .expect("accepted frontend-to-Intro2 constructor receipt");
        let owner = Intro2Type13SchedulerOwner::adopt_published(
            manager
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
                .expect("published type-13"),
        )
        .expect("published B6C0 graph");
        let first = tick_intro2_type13_scheduler_owner(
            &mut manager,
            owner,
            &mut WorldFx::new(),
            Some(primary_frame(&session, 20_000)),
        );
        let owner = first
            .retained_owner
            .expect("ADE0 retains the pursuing owner");

        let entity_before = manager
            .iter_all()
            .find(|entity| entity.id == owner.entity_id())
            .expect("pursuing type-13");
        let initial_behavior_before = entity_before.initial_behavior;
        let RetailRuntimeValue::Known(Some(context_before)) =
            entity_before.current_behavior_context
        else {
            panic!("pursuing context")
        };

        let aim_c690_words: Vec<u32> = if expected_class == 7 {
            vec![0x4000, 0x2222, 0x3333]
        } else {
            Vec::new()
        };
        let mut root_words = [selector]
            .into_iter()
            .chain(suffixes.iter().copied())
            .chain(aim_c690_words.iter().copied());
        let mut root_draws = 0;
        let transition = tick_intro2_type13_scheduler_owner_with_random(
            &mut manager,
            owner,
            &mut WorldFx::new(),
            Some(Intro2Type13PrimaryFrame {
                dispatch_mode:
                    v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
                retail_tick: 1,
                global_elapsed_micros: 5_001_000,
                ..primary_frame(&session, 5_001_000)
            }),
            &mut |_| {
                root_draws += 1;
                root_words.next().expect("pursuing C690 RNG suffix")
            },
        );
        let (publication, acquisition, chase, aim) = match transition.outcome {
            Intro2Type13SchedulerProductionOutcome::PursuingVisit {
                chase: Some(chase),
                post_chase_plus00_c690: Some(chase_attempt),
                acquisition,
                aim,
                post_aim_plus04_c690: None,
                post_aim_plus00_c690,
                ..
            } => {
                let Intro2Type13C690Transition::Published(chase_publication) =
                    chase_attempt.transition
                else {
                    panic!("Chase +0x00 C690 must publish, got {chase_attempt:?}");
                };
                assert_eq!(
                    chase.prefix.lifetime_status,
                    v2k_game::chase_target::ChaseTargetLifetimeStatus::OwnerTransitionDue
                );
                let publication = if expected_class == 5 {
                    assert_eq!(post_aim_plus00_c690, None);
                    chase_publication
                } else {
                    assert!(matches!(
                        chase_publication,
                        Type13InitialBehaviorPublication::SearchAndAttack(_)
                    ));
                    match post_aim_plus00_c690 {
                        Some(attempt) => match attempt.transition {
                            Intro2Type13C690Transition::Published(publication) => publication,
                            other => panic!("expired Tertiary Aim must publish, got {other:?}"),
                        },
                        other => panic!("expired Tertiary Aim must use +0x00 C690, got {other:?}"),
                    }
                };
                (publication, acquisition, chase, aim)
            }
            other => panic!("Chase timeout must apply pursuing C690, got {other:?}"),
        };
        assert!(matches!(
            chase.result,
            Intro2Type13ChaseVisitResult::Continue | Intro2Type13ChaseVisitResult::Tagged(_)
        ));
        assert_eq!(root_draws, 1 + suffixes.len() + aim_c690_words.len());
        assert!(root_words.next().is_none());

        match (expected_class, publication) {
            (5, Type13InitialBehaviorPublication::MoveAboutAimlessly(publication)) => {
                assert_eq!(publication.planned.random_sample_low16, 0x3fff);
                assert_eq!(publication.constructor.random_sample_low16, 0x1234);
                assert_eq!(acquisition, None);
                assert!(aim.is_none());
            }
            (7, Type13InitialBehaviorPublication::SearchAndAttack(publication)) => {
                assert_eq!(publication.planned.random_sample_low16, 0x4000);
                assert_eq!(
                    publication
                        .constructors_by_phase
                        .map(|constructor| constructor.random_sample_low16),
                    [0x2222, 0x3333],
                    "Tertiary Aim timeout C690 publishes a new class-7 graph"
                );
                assert!(matches!(
                    acquisition,
                    Some(SearchAttackLiveAcquisitionOutcome::Applied {
                        same_pass_aim: None,
                        ..
                    })
                ));
                assert_type13_aim_24650(aim.as_ref());
            }
            (_, other) => panic!("selector chose the wrong Type-13 root: {other:?}"),
        }
        assert!(transition.retained_owner.is_some());

        let entity_after = manager
            .iter_all()
            .find(|entity| entity.id == owner.entity_id())
            .expect("reselected type-13");
        assert_eq!(entity_after.initial_behavior, initial_behavior_before);
        let RetailRuntimeValue::Known(Some(context_after)) = entity_after.current_behavior_context
        else {
            panic!("reselected context")
        };
        assert_eq!(
            context_after.target_handle_at_0x08(),
            context_before.target_handle_at_0x08()
        );
        assert_eq!(
            context_after.auxiliary_word_at_0x0c(),
            context_before.auxiliary_word_at_0x0c()
        );
        if expected_class == 5 {
            let Some(ActorTaskRuntime::SharedRetarget(primary)) =
                entity_after.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("class-5 C690 must publish ACD0 Primary")
            };
            assert_eq!(primary.elapsed_ms(), 0);
            assert_eq!(primary.lifetime_ms(), 5_000);
            assert_eq!(
                entity_after.actor_task_state(ActorTaskSlot::Secondary),
                None
            );
        } else {
            let Some(ActorTaskRuntime::SharedRetarget(primary)) =
                entity_after.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("Aim timeout C690 class-7 must publish B6C0 Primary")
            };
            assert_eq!(primary.elapsed_ms(), 0);
            assert_eq!(primary.lifetime_ms(), 500);
            assert!(
                matches!(
                    entity_after.actor_task_state(ActorTaskSlot::Secondary),
                    Some(ActorTaskRuntime::TargetAcquisition(_))
                ),
                "Tertiary Aim C690 must not ADE0 the newborn Secondary"
            );
            assert!(entity_after
                .actor_task_state(ActorTaskSlot::Tertiary)
                .is_none());
        }
    }
}

#[v2k_test_support::retail_test]
fn pursuing_chase_rejects_a_partially_known_zero_target_state() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let (mut manager, owner) = captured_intro2_type13_pursuing(&session);
    let target_id = match manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .and_then(|entity| entity.actor_task_state(ActorTaskSlot::Primary))
    {
        Some(ActorTaskRuntime::ChaseTarget(chase)) => chase.target_id(),
        other => panic!("pursuing Primary must be Chase, got {other:?}"),
    };
    manager
        .entity_mut(target_id)
        .expect("pursuing target")
        .collision
        .state_flags_at_0x08 = RetailStateWord::from_known_bits(0, DYING_STATE_BIT);
    let entity_before = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("pursuing type-13");
    let tasks_before = ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .map(|slot| entity_before.actor_task_state(slot).copied());
    let context_before = entity_before.current_behavior_context;
    let mover_before = entity_before.intro2_type13_common_mover_runtime;
    let sub_g_before = entity_before.sub_g_06070_runtime;

    let mut draws = 0;
    let blocked = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut WorldFx::new(),
        Some(primary_frame(&session, 20_000)),
        &mut |_| {
            draws += 1;
            0
        },
    );
    assert!(matches!(
        blocked.outcome,
        Intro2Type13SchedulerProductionOutcome::PursuingChaseBlocked {
            reason: v2k_game::intro2_type13_live::Intro2Type13ChaseBlock::TargetStateUnresolved,
            ..
        }
    ));
    assert!(blocked.retained_owner.is_some());
    assert_eq!(
        draws, 0,
        "unresolved target state must precede Chase/C690 RNG"
    );
    let entity_after = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("pursuing type-13");
    assert_eq!(
        ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .map(|slot| entity_after.actor_task_state(slot).copied()),
        tasks_before
    );
    assert_eq!(entity_after.current_behavior_context, context_before);
    assert_eq!(
        entity_after.intro2_type13_common_mover_runtime,
        mover_before
    );
    assert_eq!(entity_after.sub_g_06070_runtime, sub_g_before);
}

#[v2k_test_support::retail_test]
fn pursuing_graph_rejects_foreign_aim_audio_before_the_aim_visit() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let mut type_metadata = intro2_type_metadata(&session);
    type_metadata[TYPE13_ENTITY_TYPE as usize].search_attack_aim_sound_id =
        RetailRuntimeValue::Known(Some(70));
    type_metadata[TYPE13_ENTITY_TYPE as usize].search_attack_aim_sound_period_raw =
        RetailRuntimeValue::Known(0x400);
    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection {
            type13: v2k_game::intro2_type13_live::Intro2Type13BirthSelection::CapturedClass7,
            ..Default::default()
        },
        &mut || 0,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");
    let entity_id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
        .expect("published type-13")
        .id;
    let mut world_fx = WorldFx::new();
    let acquisition = apply_type13_published_class7_acquisition_live(
        &mut manager,
        entity_id,
        20_000,
        &mut world_fx,
    )
    .expect("ADE0 publishes the deliberately foreign Aim graph");
    assert!(matches!(
        acquisition,
        SearchAttackLiveAcquisitionOutcome::Applied { .. }
    ));
    assert!(Intro2Type13SchedulerOwner::adopt_published(
        manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("foreign pursuing graph")
    )
    .is_err());
    assert_eq!(
        tick_intro2_type13_aim(
            v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
            &mut manager,
            &mut world_fx,
            entity_id,
            20_000,
            type_metadata.get(TYPE13_ENTITY_TYPE as usize),
        ),
        Err(Type13AimError::GraphMismatch)
    );
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .expect("type-13 with rejected foreign Aim");
    let Some(ActorTaskRuntime::AimAndFire(aim)) = entity.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!("ADE0 published the deliberately foreign Aim task");
    };
    assert_eq!(
        aim.elapsed_ms(),
        0,
        "authentication precedes Aim accounting"
    );
    assert_eq!(
        entity
            .intro2_type13_aim_runtime
            .as_ref()
            .map(|runtime| runtime.queued_shot_count()),
        None,
        "rejected graph cannot allocate the Type-13 Aim runtime"
    );
    assert!(world_fx.take_positional_sounds().is_empty());
    let mut untouched_rng = WorldFx::new();
    assert_eq!(
        world_fx.next_shared_retail_random_u16(),
        untouched_rng.next_shared_retail_random_u16(),
        "foreign optional audio cannot consume the shared stream"
    );
}

#[v2k_test_support::retail_test]
fn pursuing_chase_common_mover_block_stops_before_aim() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let (mut manager, owner) = captured_intro2_type13_pursuing(&session);
    let entity_before = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("pursuing type-13");
    let aim_before = entity_before
        .actor_task_state(ActorTaskSlot::Tertiary)
        .copied();
    let queued_before = entity_before
        .intro2_type13_aim_runtime
        .as_ref()
        .map(|runtime| runtime.queued_shot_count());
    manager
        .entity_mut(owner.entity_id())
        .expect("pursuing type-13")
        .capability_flags |= 1;

    let mut draws = 0;
    let mut world_fx = WorldFx::new();
    let blocked = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut world_fx,
        Some(primary_frame(&session, 20_000)),
        &mut |_| {
            draws += 1;
            0
        },
    );
    assert!(matches!(
        blocked.outcome,
        Intro2Type13SchedulerProductionOutcome::PursuingVisit {
            chase: Some(v2k_game::intro2_type13_live::Intro2Type13ChaseVisit {
                result: Intro2Type13ChaseVisitResult::CommonMoverBlocked(
                    GklCommonMoverBlock::SubG(Type13SubGFrameBlock::PlayerProjectionUnsupported)
                ),
                ..
            }),
            post_chase_plus00_c690: None,
            acquisition: None,
            aim: None,
            post_aim_plus04_c690: None,
            post_aim_plus00_c690: None,
            ..
        }
    ));
    assert!(blocked.retained_owner.is_some());
    assert_ne!(blocked.retained_owner, Some(owner));
    assert_eq!(draws, 0);
    assert!(world_fx.take_positional_sounds().is_empty());
    assert_eq!(world_fx.particle_count(), 0);
    let entity_after = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("pursuing type-13 after block");
    assert_eq!(
        entity_after
            .actor_task_state(ActorTaskSlot::Tertiary)
            .copied(),
        aim_before
    );
    assert_eq!(
        entity_after
            .intro2_type13_aim_runtime
            .as_ref()
            .map(|runtime| runtime.queued_shot_count()),
        queued_before
    );
}

#[v2k_test_support::retail_test]
fn pursuing_c690_plan_retry_rejects_a_changed_post_callback_predecessor() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let (mut manager, running_owner) = captured_intro2_type13_pursuing(&session);
    make_pursuing_chase_target_inactive(&mut manager, running_owner.entity_id());
    let state_before = manager
        .iter_all()
        .find(|entity| entity.id == running_owner.entity_id())
        .expect("pursuing type-13")
        .collision
        .state_flags_at_0x08;
    manager
        .entity_mut(running_owner.entity_id())
        .expect("pursuing type-13")
        .collision
        .state_flags_at_0x08
        .invalidate(TYPE13_C690_SUPPRESS_STATE_BIT);

    let blocked = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        running_owner,
        &mut WorldFx::new(),
        Some(primary_frame(&session, 20_000)),
        &mut |_| panic!("an unresolved suppression bit must precede C690 RNG"),
    );
    assert!(matches!(
        blocked.outcome,
        Intro2Type13SchedulerProductionOutcome::PursuingC690Blocked {
            reason: Intro2Type13C690Block::Plan(Type13C690ReselectionBlock::StateFlagsUnresolved),
            ..
        }
    ));
    let pending_owner = blocked
        .retained_owner
        .expect("pending pursuing C690 plan retains custody");
    let changed = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        running_owner,
        &mut WorldFx::new(),
        Some(primary_frame(&session, 20_000)),
        &mut |_| panic!("a second unresolved suppression bit must precede C690 RNG"),
    );
    assert!(matches!(
        changed.outcome,
        Intro2Type13SchedulerProductionOutcome::PursuingC690Blocked {
            reason: Intro2Type13C690Block::Plan(Type13C690ReselectionBlock::StateFlagsUnresolved),
            ..
        }
    ));
    manager
        .entity_mut(running_owner.entity_id())
        .expect("pursuing type-13")
        .collision
        .state_flags_at_0x08 = state_before;
    let tasks_before_retry = {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == running_owner.entity_id())
            .expect("pursuing type-13");
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).copied())
    };

    let mut draws = 0;
    let dropped = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        pending_owner,
        &mut WorldFx::new(),
        None,
        &mut |_| {
            draws += 1;
            0
        },
    );
    assert!(matches!(
        dropped.outcome,
        Intro2Type13SchedulerProductionOutcome::Dropped {
            reason: Intro2Type13SchedulerProductionDrop::GraphMismatch,
            ..
        }
    ));
    assert_eq!(dropped.retained_owner, None);
    assert_eq!(draws, 0, "a stale predecessor cannot consume the selector");
    let entity_after = manager
        .iter_all()
        .find(|entity| entity.id == running_owner.entity_id())
        .expect("pursuing type-13");
    assert_eq!(
        ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .map(|slot| entity_after.actor_task_state(slot).copied()),
        tasks_before_retry,
        "dropping stale custody must not replace the changed graph"
    );
}

#[v2k_test_support::retail_test]
fn pursuing_c690_plan_retry_rejects_a_changed_predecessor_context_without_rng() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let (mut manager, owner) = captured_intro2_type13_pursuing(&session);
    make_pursuing_chase_target_inactive(&mut manager, owner.entity_id());
    let state_before = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("pursuing type-13")
        .collision
        .state_flags_at_0x08;
    manager
        .entity_mut(owner.entity_id())
        .expect("pursuing type-13")
        .collision
        .state_flags_at_0x08
        .invalidate(TYPE13_C690_SUPPRESS_STATE_BIT);
    let blocked = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut WorldFx::new(),
        Some(primary_frame(&session, 20_000)),
        &mut |_| panic!("an unresolved suppression bit must precede C690 RNG"),
    );
    let pending_owner = blocked
        .retained_owner
        .expect("pending pursuing C690 plan retains custody");

    let entity = manager
        .entity_mut(owner.entity_id())
        .expect("pursuing type-13");
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!("pursuing context is known");
    };
    let BehaviorDescriptorIdentity::Named(program) = context.descriptor() else {
        panic!("pursuing context has a named descriptor");
    };
    let changed_auxiliary = match context.auxiliary_word_at_0x0c() {
        RetailRuntimeValue::Known(value) => RetailRuntimeValue::Known(value.wrapping_add(1)),
        RetailRuntimeValue::Unresolved => RetailRuntimeValue::Known(0),
    };
    let changed_context = BehaviorContextRuntime::named_audited(
        program,
        context.style_table_index_raw_at_0x10(),
        context.choice_list_source(),
        context.target_handle_at_0x08(),
        changed_auxiliary,
        context
            .active_style()
            .audited()
            .expect("pursuing style is audited"),
    )
    .expect("changed context remains a valid class-7 pursuing context");
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(changed_context));
    entity.collision.state_flags_at_0x08 = state_before;
    let tasks_before_retry =
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).copied());

    let mut draws = 0;
    let dropped = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        pending_owner,
        &mut WorldFx::new(),
        None,
        &mut |_| {
            draws += 1;
            0
        },
    );
    assert!(matches!(
        dropped.outcome,
        Intro2Type13SchedulerProductionOutcome::Dropped {
            reason: Intro2Type13SchedulerProductionDrop::GraphMismatch,
            ..
        }
    ));
    assert_eq!(dropped.retained_owner, None);
    assert_eq!(draws, 0, "a changed context cannot consume selector RNG");
    let entity_after = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("pursuing type-13");
    assert_eq!(
        entity_after.current_behavior_context,
        RetailRuntimeValue::Known(Some(changed_context))
    );
    assert_eq!(
        ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .map(|slot| entity_after.actor_task_state(slot).copied()),
        tasks_before_retry
    );
}

#[v2k_test_support::retail_test]
fn pursuing_c690_plan_retry_freshly_reads_a_changed_later_aim() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let (mut manager, owner) = captured_intro2_type13_pursuing(&session);
    make_pursuing_chase_target_inactive(&mut manager, owner.entity_id());
    manager
        .entity_mut(owner.entity_id())
        .expect("pursuing type-13")
        .collision
        .state_flags_at_0x08
        .invalidate(TYPE13_C690_SUPPRESS_STATE_BIT);
    let blocked = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut WorldFx::new(),
        Some(primary_frame(&session, 20_000)),
        &mut |_| panic!("an unresolved suppression bit must precede C690 RNG"),
    );
    assert!(matches!(
        blocked.outcome,
        Intro2Type13SchedulerProductionOutcome::PursuingC690Blocked {
            reason: Intro2Type13C690Block::Plan(Type13C690ReselectionBlock::StateFlagsUnresolved),
            ..
        }
    ));
    let owner = blocked
        .retained_owner
        .expect("pending pursuing C690 plan retains custody");
    let advanced = tick_pursuing_aim_out_of_band(&session, &mut manager, owner.entity_id(), 1_000);
    assert!(matches!(
        advanced.resolution.outcome,
        AimAndFireFrameOutcome::RequestOwnerTransition { .. }
    ));
    manager
        .entity_mut(owner.entity_id())
        .expect("pursuing type-13")
        .collision
        .state_flags_at_0x08 = RetailStateWord::exact(TYPE13_C690_SUPPRESS_STATE_BIT);

    let mut draws = 0;
    let resumed = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut WorldFx::new(),
        None,
        &mut |_| {
            draws += 1;
            0
        },
    );
    assert!(matches!(
        resumed.outcome,
        Intro2Type13SchedulerProductionOutcome::PursuingVisit {
            post_chase_plus00_c690: Some(Intro2Type13PursuingC690Attempt {
                request: Intro2Type13PursuingC690Request {
                    reason: Intro2Type13PursuingC690Reason::ChaseTagged(_),
                    ..
                },
                transition: Intro2Type13C690Transition::SuppressedByEntityState,
            }),
            post_aim_plus04_c690: Some(Intro2Type13PursuingC690Attempt {
                request: Intro2Type13PursuingC690Request {
                    reason: Intro2Type13PursuingC690Reason::AimTaggedInvalidTarget {
                        lifetime_due: false,
                        ..
                    },
                    ..
                },
                transition: Intro2Type13C690Transition::SuppressedByEntityState,
            }),
            post_aim_plus00_c690: None,
            ..
        }
    ));
    assert!(resumed.retained_owner.is_some());
    assert_eq!(draws, 0, "both suppressed C690 calls consume no selector");
}

#[v2k_test_support::retail_test]
fn pursuing_aim_lifetime_uses_plus00_c690() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let (mut manager, owner) = captured_intro2_type13_pursuing(&session);
    let advanced =
        tick_pursuing_aim_out_of_band(&session, &mut manager, owner.entity_id(), 5_001_000);
    assert!(matches!(
        advanced.resolution.outcome,
        AimAndFireFrameOutcome::RequestOwnerTransition {
            reason: AimAndFireTransitionReason::LifetimeExpired
        }
    ));

    let mut words = [0_u32, 0x1234].into_iter();
    let mut draws = 0;
    let transition = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut WorldFx::new(),
        Some(primary_frame(&session, 20_000)),
        &mut |_| {
            draws += 1;
            words
                .next()
                .expect("Aim +0x00 selector and constructor word")
        },
    );
    let attempt = match transition.outcome {
        Intro2Type13SchedulerProductionOutcome::PursuingVisit {
            post_chase_plus00_c690: None,
            post_aim_plus04_c690: None,
            post_aim_plus00_c690: Some(attempt),
            ..
        } => attempt,
        other => panic!("Aim lifetime must route through +0x00, got {other:?}"),
    };
    assert_eq!(
        attempt.request.reason,
        Intro2Type13PursuingC690Reason::AimLifetimeExpired
    );
    assert!(matches!(
        attempt.transition,
        Intro2Type13C690Transition::Published(
            Type13InitialBehaviorPublication::MoveAboutAimlessly(_)
        )
    ));
    assert_eq!(draws, 2);
    assert!(words.next().is_none());
}

#[v2k_test_support::retail_test]
fn pursuing_aim_c690_block_reports_the_committed_aim_visit() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let (mut manager, owner) = captured_intro2_type13_pursuing(&session);
    let advanced =
        tick_pursuing_aim_out_of_band(&session, &mut manager, owner.entity_id(), 5_001_000);
    assert!(matches!(
        advanced.resolution.outcome,
        AimAndFireFrameOutcome::RequestOwnerTransition {
            reason: AimAndFireTransitionReason::LifetimeExpired
        }
    ));
    manager
        .entity_mut(owner.entity_id())
        .expect("pursuing type-13")
        .collision
        .state_flags_at_0x08
        .invalidate(TYPE13_C690_SUPPRESS_STATE_BIT);

    let mut draws = 0;
    let blocked = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut WorldFx::new(),
        Some(primary_frame(&session, 20_000)),
        &mut |_| {
            draws += 1;
            0
        },
    );
    let aim = match blocked.outcome {
        Intro2Type13SchedulerProductionOutcome::PursuingC690Blocked {
            chase: Some(_),
            aim: Some(Ok(aim)),
            acquisition: None,
            request:
                Intro2Type13PursuingC690Request {
                    reason: Intro2Type13PursuingC690Reason::AimLifetimeExpired,
                    ..
                },
            reason: Intro2Type13C690Block::Plan(Type13C690ReselectionBlock::StateFlagsUnresolved),
            ..
        } => aim,
        other => panic!("blocked Aim continuation must report its committed visit, got {other:?}"),
    };
    assert!(matches!(
        aim.resolution.outcome,
        AimAndFireFrameOutcome::RequestOwnerTransition {
            reason: AimAndFireTransitionReason::LifetimeExpired
        }
    ));
    assert_eq!(aim.queued_shots_added, 0);
    assert_eq!(draws, 0);
    assert!(blocked.retained_owner.is_some());
}

#[v2k_test_support::retail_test]
fn pursuing_aim_c690_plan_retry_rejects_a_changed_primary_prefix() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let (mut manager, running_owner) = captured_intro2_type13_pursuing(&session);
    let advanced =
        tick_pursuing_aim_out_of_band(&session, &mut manager, running_owner.entity_id(), 5_001_000);
    assert!(matches!(
        advanced.resolution.outcome,
        AimAndFireFrameOutcome::RequestOwnerTransition {
            reason: AimAndFireTransitionReason::LifetimeExpired
        }
    ));
    let state_before = manager
        .iter_all()
        .find(|entity| entity.id == running_owner.entity_id())
        .expect("pursuing type-13")
        .collision
        .state_flags_at_0x08;
    manager
        .entity_mut(running_owner.entity_id())
        .expect("pursuing type-13")
        .collision
        .state_flags_at_0x08
        .invalidate(TYPE13_C690_SUPPRESS_STATE_BIT);
    let blocked = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        running_owner,
        &mut WorldFx::new(),
        Some(primary_frame(&session, 20_000)),
        &mut |_| panic!("an unresolved suppression bit must precede C690 RNG"),
    );
    let pending_owner = blocked
        .retained_owner
        .expect("pending Aim C690 plan retains custody");
    let (primary_before_stale_visit, tertiary_before_stale_visit) = {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == running_owner.entity_id())
            .expect("pursuing type-13");
        (
            entity.actor_task_state(ActorTaskSlot::Primary).copied(),
            entity.actor_task_state(ActorTaskSlot::Tertiary).copied(),
        )
    };
    manager
        .entity_mut(running_owner.entity_id())
        .expect("pursuing type-13")
        .capability_flags |= 1;
    let stale_visit = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        running_owner,
        &mut WorldFx::new(),
        Some(primary_frame(&session, 20_000)),
        &mut |_| panic!("the forced common-mover block precedes RNG"),
    );
    assert!(matches!(
        stale_visit.outcome,
        Intro2Type13SchedulerProductionOutcome::PursuingVisit {
            chase: Some(v2k_game::intro2_type13_live::Intro2Type13ChaseVisit {
                result: Intro2Type13ChaseVisitResult::CommonMoverBlocked(_),
                ..
            }),
            aim: None,
            ..
        }
    ));
    let entity = manager
        .entity_mut(running_owner.entity_id())
        .expect("pursuing type-13");
    assert_ne!(
        entity.actor_task_state(ActorTaskSlot::Primary).copied(),
        primary_before_stale_visit,
        "the stale owner advanced the committed Primary prefix"
    );
    assert_eq!(
        entity.actor_task_state(ActorTaskSlot::Tertiary).copied(),
        tertiary_before_stale_visit,
        "the forced block stopped before Tertiary"
    );
    entity.capability_flags &= !1;
    entity.collision.state_flags_at_0x08 = state_before;

    let mut draws = 0;
    let dropped = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        pending_owner,
        &mut WorldFx::new(),
        None,
        &mut |_| {
            draws += 1;
            0
        },
    );
    assert!(matches!(
        dropped.outcome,
        Intro2Type13SchedulerProductionOutcome::Dropped {
            reason: Intro2Type13SchedulerProductionDrop::GraphMismatch,
            ..
        }
    ));
    assert_eq!(dropped.retained_owner, None);
    assert_eq!(draws, 0, "a changed committed prefix cannot consume RNG");
}

#[v2k_test_support::retail_test]
fn suppressed_due_invalid_aim_falls_through_from_plus04_to_plus00() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let (mut manager, owner) = captured_intro2_type13_pursuing(&session);
    let advanced =
        tick_pursuing_aim_out_of_band(&session, &mut manager, owner.entity_id(), 5_001_000);
    assert!(matches!(
        advanced.resolution.outcome,
        AimAndFireFrameOutcome::RequestOwnerTransition {
            reason: AimAndFireTransitionReason::LifetimeExpired
        }
    ));
    make_pursuing_chase_target_inactive(&mut manager, owner.entity_id());
    manager
        .entity_mut(owner.entity_id())
        .expect("pursuing type-13")
        .collision
        .state_flags_at_0x08 = RetailStateWord::exact(TYPE13_C690_SUPPRESS_STATE_BIT);

    let mut draws = 0;
    let transition = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut WorldFx::new(),
        Some(primary_frame(&session, 20_000)),
        &mut |_| {
            draws += 1;
            0
        },
    );
    let (chase_attempt, tagged_attempt, lifetime_attempt) = match transition.outcome {
        Intro2Type13SchedulerProductionOutcome::PursuingVisit {
            post_chase_plus00_c690: Some(chase_attempt),
            post_aim_plus04_c690: Some(tagged_attempt),
            post_aim_plus00_c690: Some(lifetime_attempt),
            ..
        } => (chase_attempt, tagged_attempt, lifetime_attempt),
        other => panic!("due invalid Aim must fall through after suppression, got {other:?}"),
    };
    assert!(matches!(
        chase_attempt.request.reason,
        Intro2Type13PursuingC690Reason::ChaseTagged(_)
    ));
    assert!(matches!(
        tagged_attempt.request.reason,
        Intro2Type13PursuingC690Reason::AimTaggedInvalidTarget {
            lifetime_due: true,
            ..
        }
    ));
    assert_eq!(
        lifetime_attempt.request.reason,
        Intro2Type13PursuingC690Reason::AimLifetimeExpired
    );
    for attempt in [chase_attempt, tagged_attempt, lifetime_attempt] {
        assert_eq!(
            attempt.transition,
            Intro2Type13C690Transition::SuppressedByEntityState
        );
    }
    assert_eq!(draws, 0, "all three suppressed C690 calls consume no RNG");
    assert!(transition.retained_owner.is_some());
}

#[v2k_test_support::retail_test]
fn pursuing_c690_plan_retry_suppresses_before_context_preflight() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let (mut manager, owner) = captured_intro2_type13_pursuing(&session);
    make_pursuing_chase_target_inactive(&mut manager, owner.entity_id());
    manager
        .entity_mut(owner.entity_id())
        .expect("pursuing type-13")
        .collision
        .state_flags_at_0x08
        .invalidate(TYPE13_C690_SUPPRESS_STATE_BIT);
    let blocked = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut WorldFx::new(),
        Some(primary_frame(&session, 20_000)),
        &mut |_| panic!("an unresolved suppression bit must precede C690 RNG"),
    );
    let owner = blocked
        .retained_owner
        .expect("pending pursuing C690 plan retains custody");
    let entity = manager
        .entity_mut(owner.entity_id())
        .expect("pursuing type-13");
    let basis_before_retry = entity.physical_body_basis_q31();
    entity.current_behavior_context = RetailRuntimeValue::Unresolved;
    entity.collision.state_flags_at_0x08 = RetailStateWord::exact(TYPE13_C690_SUPPRESS_STATE_BIT);

    let mut draws = 0;
    let resumed = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut WorldFx::new(),
        None,
        &mut |_| {
            draws += 1;
            0
        },
    );
    assert!(matches!(
        resumed.outcome,
        Intro2Type13SchedulerProductionOutcome::PursuingVisit {
            post_chase_plus00_c690: Some(Intro2Type13PursuingC690Attempt {
                request: Intro2Type13PursuingC690Request {
                    reason: Intro2Type13PursuingC690Reason::ChaseTagged(_),
                    ..
                },
                transition: Intro2Type13C690Transition::SuppressedByEntityState,
            }),
            aim: Some(Err(Type13AimError::GraphMismatch)),
            ..
        }
    ));
    assert_eq!(draws, 0, "suppression must not consult context or RNG");
    assert_eq!(
        manager
            .entity_mut(owner.entity_id())
            .expect("pursuing type-13")
            .physical_body_basis_q31(),
        basis_before_retry,
        "an Aim error after suppressed C690 must stop before the basis rebuild"
    );
}

#[v2k_test_support::retail_test]
fn b6c0_scheduler_applies_c690_class5_and_class7_roots_without_parking() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    for (selector, suffixes, expected_class) in [
        (0x3fff_u32, vec![0x1234], 5_u8),
        (0x4000_u32, vec![0x1234, 0xabcd], 7_u8),
    ] {
        let type_metadata = intro2_type_metadata(&session);
        let level = session.cache.level_desc().expect("Intro2 Section 13");
        let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
        let resources = EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&model_extent),
        };
        let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
            level,
            &type_metadata,
            resources,
            v2k_game::entity::Intro2BirthSelection {
                type13: v2k_game::intro2_type13_live::Intro2Type13BirthSelection::CapturedClass7,
                ..Default::default()
            },
            &mut || 0,
        )
        .expect("accepted frontend-to-Intro2 constructor receipt");
        let mut owner = Intro2Type13SchedulerOwner::adopt_published(
            manager
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
                .expect("published type-13"),
        )
        .expect("published B6C0 graph");
        let other_ids: Vec<_> = manager
            .iter_all()
            .filter(|entity| entity.id != owner.entity_id())
            .map(|entity| entity.id)
            .collect();
        for entity_id in other_ids {
            let peer = manager.entity_mut(entity_id).expect("captured peer");
            peer.active = false;
            peer.collision.state_flags_at_0x08 = RetailStateWord::exact(0);
        }

        let entity_before = manager
            .iter_all()
            .find(|entity| entity.id == owner.entity_id())
            .expect("published type-13");
        let initial_behavior_before = entity_before.initial_behavior;
        let RetailRuntimeValue::Known(Some(context_before)) =
            entity_before.current_behavior_context
        else {
            panic!("published class-7 context")
        };

        // A zero-duration callback installs a deliberately distant wander
        // target. The later timeout therefore consumes no prefix RNG, making
        // the root selector/suffix cadence exact and independently visible.
        let mut prime_words = [0xffff_u32, 0xffff].into_iter();
        let mut prime_draws = 0;
        let prime = tick_intro2_type13_scheduler_owner_with_random(
            &mut manager,
            owner,
            &mut WorldFx::new(),
            Some(primary_frame(&session, 0)),
            &mut |_| {
                prime_draws += 1;
                prime_words.next().expect("two retarget words")
            },
        );
        assert_eq!(prime_draws, 2);
        assert!(matches!(
            prime.outcome,
            Intro2Type13SchedulerProductionOutcome::B6c0Visit { .. }
        ));
        owner = prime
            .retained_owner
            .expect("no-target acquisition retains class-7 custody");

        let mut boundary_words = [1_u32].into_iter();
        let at_lifetime = tick_intro2_type13_scheduler_owner_with_random(
            &mut manager,
            owner,
            &mut WorldFx::new(),
            Some(Intro2Type13PrimaryFrame {
                dispatch_mode:
                    v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
                retail_tick: 1,
                ..primary_frame(&session, 500_000)
            }),
            &mut |_| boundary_words.next().expect("far-target gate word"),
        );
        match at_lifetime.outcome {
            Intro2Type13SchedulerProductionOutcome::B6c0Visit { primary, .. } => {
                assert_eq!(primary.prefix.elapsed_ms, 500);
                assert_eq!(primary.prefix.lifetime_ms, 500);
                assert_eq!(primary.post_unwind, SharedRetargetPostUnwind::Continue);
            }
            other => panic!("exact lifetime must remain in B6C0: {other:?}"),
        }
        assert!(boundary_words.next().is_none());
        owner = at_lifetime
            .retained_owner
            .expect("strict equality retains class-7 custody");

        // Both axes are far, so the old Primary first owns its one-in-64
        // retarget gate. A non-passing word isolates the C690 root suffix.
        let mut root_words = [1_u32, selector]
            .into_iter()
            .chain(suffixes.iter().copied());
        let mut root_draws = 0;
        let mut world_fx = WorldFx::new();
        let transition = tick_intro2_type13_scheduler_owner_with_random(
            &mut manager,
            owner,
            &mut world_fx,
            Some(Intro2Type13PrimaryFrame {
                dispatch_mode:
                    v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
                retail_tick: 2,
                global_elapsed_micros: 501_000,
                ..primary_frame(&session, 1_000)
            }),
            &mut |_| {
                root_draws += 1;
                root_words.next().expect("audited C690 RNG suffix")
            },
        );
        let (request, publication, acquisition) = match transition.outcome {
            Intro2Type13SchedulerProductionOutcome::C690Transition {
                request,
                transition: Intro2Type13C690Transition::Published(publication),
                primary: Some(primary),
                acquisition,
                ..
            } => {
                assert_eq!(primary.prefix.elapsed_ms, 501);
                assert_eq!(primary.prefix.lifetime_ms, 500);
                assert_eq!(primary.result, Intro2Type13PrimaryVisitResult::Continue);
                assert_eq!(
                    primary.post_unwind,
                    SharedRetargetPostUnwind::Transition(request)
                );
                (request, publication, acquisition)
            }
            other => panic!("strict timeout must apply C690 inline, got {other:?}"),
        };
        assert_eq!(request.slot, ActorTaskSlot::Primary);
        assert_eq!(
            request.reason,
            SharedRetargetTransitionReason::LifetimeExpired
        );
        assert_eq!(root_draws, 2 + suffixes.len());
        assert!(root_words.next().is_none());

        match (expected_class, publication) {
            (5, Type13InitialBehaviorPublication::MoveAboutAimlessly(publication)) => {
                assert_eq!(publication.planned.random_sample_low16, 0x3fff);
                assert_eq!(publication.constructor.random_sample_low16, 0x1234);
                assert_eq!(acquisition, None);
                assert!(
                    transition.retained_owner.is_some(),
                    "class 5 has no later slot but keeps the 5,000-ms Primary owner"
                );
            }
            (7, Type13InitialBehaviorPublication::SearchAndAttack(publication)) => {
                assert_eq!(publication.planned.random_sample_low16, 0x4000);
                assert_eq!(
                    publication
                        .constructors_by_phase
                        .map(|constructor| constructor.random_sample_low16),
                    [0x1234, 0xabcd]
                );
                assert!(acquisition.is_some());
                assert!(transition.retained_owner.is_some());
            }
            (_, other) => panic!("selector chose the wrong Type-13 root: {other:?}"),
        }

        let entity_after = manager
            .iter_all()
            .find(|entity| entity.id == owner.entity_id())
            .expect("reselected type-13");
        assert_eq!(entity_after.initial_behavior, initial_behavior_before);
        let RetailRuntimeValue::Known(Some(context_after)) = entity_after.current_behavior_context
        else {
            panic!("reselected context")
        };
        assert_eq!(
            context_after.target_handle_at_0x08(),
            context_before.target_handle_at_0x08()
        );
        assert_eq!(
            context_after.auxiliary_word_at_0x0c(),
            context_before.auxiliary_word_at_0x0c()
        );
        let Some(ActorTaskRuntime::SharedRetarget(primary)) =
            entity_after.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("reselection must publish a fresh Primary")
        };
        assert_eq!(primary.elapsed_ms(), 0, "newborn Primary is not revisited");
        assert_eq!(
            primary.lifetime_ms(),
            if expected_class == 5 { 5_000 } else { 500 }
        );
        assert_eq!(
            entity_after
                .actor_task_state(ActorTaskSlot::Secondary)
                .is_some(),
            expected_class == 7
        );
        assert_eq!(entity_after.actor_task_state(ActorTaskSlot::Tertiary), None);
    }
}

#[v2k_test_support::retail_test]
fn class5_scheduler_commits_01430_without_acquisition() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let (type_metadata, mut manager, owner) = captured_intro2_type13(&session);

    let class5_on_b6c0 = tick_intro2_type13_class5_primary(
        manager
            .entity_mut(owner.entity_id())
            .expect("published B6C0 type-13"),
        type_metadata.get(TYPE13_ENTITY_TYPE as usize),
        primary_frame(&session, 0),
        &mut || panic!("class-5 Primary must not run against a B6C0 graph"),
    );
    assert_eq!(
        class5_on_b6c0,
        Err(Intro2Type13PrimaryVisitBlock::GraphMismatch)
    );

    let owner = reselect_class5_from_b6c0(&session, &mut manager, owner);
    let class7_on_aimless = tick_intro2_type13_primary(
        manager
            .entity_mut(owner.entity_id())
            .expect("class-5 type-13"),
        type_metadata.get(TYPE13_ENTITY_TYPE as usize),
        primary_frame(&session, 0),
        &mut || panic!("B6C0 Primary must not run against a class-5 graph"),
    );
    assert_eq!(
        class7_on_aimless,
        Err(Intro2Type13PrimaryVisitBlock::GraphMismatch)
    );

    let entity_before = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("class-5 type-13");
    let position_before = entity_before.position_raw();
    let velocity_before = entity_before.velocity_raw();
    let rotation_before = entity_before.rotation_heading_pitch_roll_raw();
    let basis_before = entity_before.physical_body_basis_q31();
    let sub_g_before = entity_before.sub_g_06070_runtime;
    let mut near_axis_words = [0x0000_u32, 0xffff].into_iter();
    let primary = tick_intro2_type13_class5_primary(
        manager
            .entity_mut(owner.entity_id())
            .expect("class-5 type-13"),
        type_metadata.get(TYPE13_ENTITY_TYPE as usize),
        primary_frame(&session, 20_000),
        &mut || near_axis_words.next().expect("near-axis X then Z"),
    )
    .expect("class-5 Primary callback");
    assert!(near_axis_words.next().is_none());
    assert_eq!(primary.result, Intro2Type13PrimaryVisitResult::Continue);
    assert_eq!(primary.post_unwind, SharedRetargetPostUnwind::Continue);
    assert_eq!(primary.prefix.elapsed_ms, 20);
    assert_eq!(primary.prefix.lifetime_ms, 5_000);
    let [owner_x, owner_y, owner_z] = INTRO2_TYPE13_POSITION_RAW;
    let expected_target = [
        owner_x.wrapping_add((0x0000_u16 >> 3).wrapping_sub(SHARED_RETARGET_RADIUS_RAW) as i16),
        owner_y,
        owner_z.wrapping_add((0xffff_u16 >> 3).wrapping_sub(SHARED_RETARGET_RADIUS_RAW) as i16),
    ];
    assert_eq!(
        primary.prefix.retarget,
        SharedRetarget::Replaced {
            trigger: SharedRetargetTrigger::NearTargetAxis,
            target_position_raw: expected_target,
        }
    );

    let entity_after = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("class-5 type-13 after 01430");
    assert_eq!(entity_after.position_raw(), position_before);
    assert_eq!(entity_after.velocity_raw(), velocity_before);
    assert_ne!(
        entity_after.rotation_heading_pitch_roll_raw(),
        rotation_before
    );
    assert_eq!(entity_after.physical_body_basis_q31(), basis_before);
    assert_ne!(entity_after.sub_g_06070_runtime, sub_g_before);
    assert_eq!(
        entity_after.actor_task_state(ActorTaskSlot::Secondary),
        None
    );
    assert_eq!(entity_after.actor_task_state(ActorTaskSlot::Tertiary), None);
    let Some(ActorTaskRuntime::SharedRetarget(state)) =
        entity_after.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("class-5 Primary remains SharedRetarget");
    };
    assert_eq!(state.elapsed_ms(), 20);
    assert_eq!(state.lifetime_ms(), 5_000);
    assert_eq!(state.private_state().target_position_raw, expected_target);

    let mut followup_words = [1_u32].into_iter();
    let tick = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut WorldFx::new(),
        Some(Intro2Type13PrimaryFrame {
            dispatch_mode:
                v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
            retail_tick: 1,
            global_elapsed_micros: 20_000,
            ..primary_frame(&session, 20_000)
        }),
        &mut |_| followup_words.next().expect("far-target gate word"),
    );
    match tick.outcome {
        Intro2Type13SchedulerProductionOutcome::Class5AimlessVisit { primary, .. } => {
            assert_eq!(primary.result, Intro2Type13PrimaryVisitResult::Continue);
            assert_eq!(primary.post_unwind, SharedRetargetPostUnwind::Continue);
            assert_eq!(primary.prefix.elapsed_ms, 40);
            assert_eq!(primary.prefix.retarget, SharedRetarget::Retained);
        }
        other => panic!("class-5 Continue must not visit acquisition, got {other:?}"),
    }
    assert!(followup_words.next().is_none());
    assert!(tick.retained_owner.is_some());
    let entity_followup = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("retained class-5 type-13");
    assert_eq!(
        entity_followup.actor_task_state(ActorTaskSlot::Secondary),
        None
    );
}

#[v2k_test_support::retail_test]
fn class5_scheduler_applies_c690_from_aimless_predecessor() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    for (selector, suffixes, expected_class) in [
        (0x3fff_u32, vec![0x1234], 5_u8),
        (0x4000_u32, vec![0x1234, 0xabcd], 7_u8),
    ] {
        let (_, mut manager, owner) = captured_intro2_type13(&session);
        let mut owner = reselect_class5_from_b6c0(&session, &mut manager, owner);
        owner = prime_distant_shared_retarget(&session, &mut manager, owner);

        let entity_before = manager
            .iter_all()
            .find(|entity| entity.id == owner.entity_id())
            .expect("class-5 type-13");
        let initial_behavior_before = entity_before.initial_behavior;
        let RetailRuntimeValue::Known(Some(context_before)) =
            entity_before.current_behavior_context
        else {
            panic!("published class-5 context")
        };

        let mut boundary_words = [1_u32].into_iter();
        let at_lifetime = tick_intro2_type13_scheduler_owner_with_random(
            &mut manager,
            owner,
            &mut WorldFx::new(),
            Some(Intro2Type13PrimaryFrame {
                dispatch_mode:
                    v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
                retail_tick: 1,
                ..primary_frame(&session, 5_000_000)
            }),
            &mut |_| boundary_words.next().expect("far-target gate word"),
        );
        match at_lifetime.outcome {
            Intro2Type13SchedulerProductionOutcome::Class5AimlessVisit { primary, .. } => {
                assert_eq!(primary.prefix.elapsed_ms, 5_000);
                assert_eq!(primary.prefix.lifetime_ms, 5_000);
                assert_eq!(primary.post_unwind, SharedRetargetPostUnwind::Continue);
            }
            other => panic!("exact class-5 lifetime must remain in ACD0: {other:?}"),
        }
        assert!(boundary_words.next().is_none());
        owner = at_lifetime
            .retained_owner
            .expect("strict equality retains class-5 custody");

        let mut root_words = [1_u32, selector]
            .into_iter()
            .chain(suffixes.iter().copied());
        let mut root_draws = 0;
        let transition = tick_intro2_type13_scheduler_owner_with_random(
            &mut manager,
            owner,
            &mut WorldFx::new(),
            Some(Intro2Type13PrimaryFrame {
                dispatch_mode:
                    v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
                retail_tick: 2,
                global_elapsed_micros: 5_001_000,
                ..primary_frame(&session, 1_000)
            }),
            &mut |_| {
                root_draws += 1;
                root_words.next().expect("audited class-5 C690 RNG suffix")
            },
        );
        let (request, publication, acquisition) = match transition.outcome {
            Intro2Type13SchedulerProductionOutcome::C690Transition {
                request,
                transition: Intro2Type13C690Transition::Published(publication),
                primary: Some(primary),
                acquisition,
                ..
            } => {
                assert_eq!(primary.prefix.elapsed_ms, 5_001);
                assert_eq!(primary.prefix.lifetime_ms, 5_000);
                assert_eq!(primary.result, Intro2Type13PrimaryVisitResult::Continue);
                assert_eq!(
                    primary.post_unwind,
                    SharedRetargetPostUnwind::Transition(request)
                );
                (request, publication, acquisition)
            }
            other => panic!("class-5 strict timeout must apply C690 inline, got {other:?}"),
        };
        assert_eq!(request.slot, ActorTaskSlot::Primary);
        assert_eq!(
            request.reason,
            SharedRetargetTransitionReason::LifetimeExpired
        );
        assert_eq!(root_draws, 2 + suffixes.len());
        assert!(root_words.next().is_none());

        match (expected_class, publication) {
            (5, Type13InitialBehaviorPublication::MoveAboutAimlessly(publication)) => {
                assert_eq!(publication.planned.random_sample_low16, 0x3fff);
                assert_eq!(publication.constructor.random_sample_low16, 0x1234);
                assert_eq!(acquisition, None);
                assert!(transition.retained_owner.is_some());
            }
            (7, Type13InitialBehaviorPublication::SearchAndAttack(publication)) => {
                assert_eq!(publication.planned.random_sample_low16, 0x4000);
                assert_eq!(
                    publication
                        .constructors_by_phase
                        .map(|constructor| constructor.random_sample_low16),
                    [0x1234, 0xabcd]
                );
                assert!(acquisition.is_some());
                assert!(transition.retained_owner.is_some());
            }
            (_, other) => panic!("selector chose the wrong Type-13 root: {other:?}"),
        }

        let entity_after = manager
            .iter_all()
            .find(|entity| entity.id == owner.entity_id())
            .expect("reselected type-13");
        assert_eq!(entity_after.initial_behavior, initial_behavior_before);
        let RetailRuntimeValue::Known(Some(context_after)) = entity_after.current_behavior_context
        else {
            panic!("reselected context")
        };
        assert_eq!(
            context_after.target_handle_at_0x08(),
            context_before.target_handle_at_0x08()
        );
        assert_eq!(
            context_after.auxiliary_word_at_0x0c(),
            context_before.auxiliary_word_at_0x0c()
        );
        let Some(ActorTaskRuntime::SharedRetarget(primary)) =
            entity_after.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("reselection must publish a fresh Primary")
        };
        assert_eq!(primary.elapsed_ms(), 0, "newborn Primary is not revisited");
        assert_eq!(
            primary.lifetime_ms(),
            if expected_class == 5 { 5_000 } else { 500 }
        );
        assert_eq!(
            entity_after
                .actor_task_state(ActorTaskSlot::Secondary)
                .is_some(),
            expected_class == 7
        );
        assert_eq!(entity_after.actor_task_state(ActorTaskSlot::Tertiary), None);
    }
}

#[v2k_test_support::retail_test]
fn class5_suppression_skips_secondary() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let (_, mut manager, owner) = captured_intro2_type13(&session);
    let mut owner = reselect_class5_from_b6c0(&session, &mut manager, owner);
    owner = prime_distant_shared_retarget(&session, &mut manager, owner);

    manager
        .entity_mut(owner.entity_id())
        .expect("class-5 type-13")
        .collision
        .state_flags_at_0x08 = RetailStateWord::from_known_bits(
        TYPE13_C690_SUPPRESS_STATE_BIT,
        TYPE13_C690_SUPPRESS_STATE_BIT,
    );

    let mut transition_words = [1_u32].into_iter();
    let mut draws = 0;
    let suppressed = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut WorldFx::new(),
        Some(primary_frame(&session, 5_001_000)),
        &mut |_| {
            draws += 1;
            transition_words
                .next()
                .expect("suppressed class-5 C690 must not draw a selector or suffix")
        },
    );
    match suppressed.outcome {
        Intro2Type13SchedulerProductionOutcome::C690Transition {
            transition: Intro2Type13C690Transition::SuppressedByEntityState,
            primary: Some(primary),
            acquisition: None,
            ..
        } => {
            assert_eq!(primary.prefix.elapsed_ms, 5_001);
            assert_eq!(primary.prefix.lifetime_ms, 5_000);
        }
        other => panic!("class-5 suppression must skip Secondary, got {other:?}"),
    }
    assert_eq!(draws, 1, "only the old Primary gate may consume RNG");
    assert!(transition_words.next().is_none());
    assert!(
        suppressed.retained_owner.is_some(),
        "the unchanged class-5 graph remains schedulable"
    );
    let entity_after = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("suppressed class-5 type-13");
    assert!(matches!(
        entity_after.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::SharedRetarget(primary))
            if primary.elapsed_ms() == 5_001 && primary.lifetime_ms() == 5_000
    ));
    assert_eq!(
        entity_after.actor_task_state(ActorTaskSlot::Secondary),
        None
    );
}

#[v2k_test_support::retail_test]
fn c690_publication_retry_freezes_selector_context_and_dispatch_cursor() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let type_metadata = intro2_type_metadata(&session);
    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection {
            type13: v2k_game::intro2_type13_live::Intro2Type13BirthSelection::CapturedClass7,
            ..Default::default()
        },
        &mut || 0,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");
    let mut owner = Intro2Type13SchedulerOwner::adopt_published(
        manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
            .expect("published type-13"),
    )
    .expect("published B6C0 graph");
    let peer_states: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.id != owner.entity_id())
        .map(|entity| {
            (
                entity.id,
                entity.active,
                entity.collision.state_flags_at_0x08,
            )
        })
        .collect();
    for (entity_id, _, _) in &peer_states {
        let peer = manager.entity_mut(*entity_id).expect("captured peer");
        peer.active = false;
        peer.collision.state_flags_at_0x08 = RetailStateWord::exact(0);
    }

    let mut prime_words = [0xffff_u32, 0xffff].into_iter();
    let prime = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut WorldFx::new(),
        Some(primary_frame(&session, 0)),
        &mut |_| prime_words.next().expect("two retarget words"),
    );
    owner = prime.retained_owner.expect("no-target B6C0 owner");
    assert!(prime_words.next().is_none());

    let entity_before = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("published type-13");
    let RetailRuntimeValue::Known(Some(predecessor_context)) =
        entity_before.current_behavior_context
    else {
        panic!("published class-7 context")
    };
    let actor_axis = entity_before.actor_common_axis_descriptor;
    let basis_before = entity_before.physical_body_basis_q31();
    manager
        .entity_mut(owner.entity_id())
        .expect("published type-13")
        .actor_common_axis_descriptor = RetailRuntimeValue::Unresolved;

    let mut planned_words = [1_u32, 0x4000].into_iter();
    let mut planned_draws = 0;
    let mut world_fx = WorldFx::new();
    let blocked = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut world_fx,
        Some(primary_frame(&session, 501_000)),
        &mut |_| {
            planned_draws += 1;
            planned_words
                .next()
                .expect("Primary gate then one root selector")
        },
    );
    let request = match blocked.outcome {
        Intro2Type13SchedulerProductionOutcome::C690Blocked {
            request,
            reason:
                Intro2Type13C690Block::Publication(
                    Type13Class7AcquiringError::ActorCommonAxisDescriptorUnresolved,
                ),
            primary: Some(primary),
            ..
        } => {
            assert_eq!(
                primary.post_unwind,
                SharedRetargetPostUnwind::Transition(request)
            );
            request
        }
        other => panic!("publication preflight must park after planning: {other:?}"),
    };
    assert_eq!(planned_draws, 2);
    assert!(planned_words.next().is_none());
    owner = blocked
        .retained_owner
        .expect("publication block retains linear custody");
    assert_eq!(
        manager
            .entity_mut(owner.entity_id())
            .expect("parked type-13")
            .physical_body_basis_q31(),
        basis_before,
        "a retained publication plan must stop before the post-task basis rebuild"
    );
    // This is still the same outer visit. A changed current default policy
    // must not replace the effective flags latched before its old Primary.
    manager
        .entity_mut(owner.entity_id())
        .unwrap()
        .collision
        .default_state_flags_at_0xc8 = RetailRuntimeValue::Unresolved;
    manager
        .entity_mut(owner.entity_id())
        .expect("parked type-13")
        .actor_common_axis_descriptor = actor_axis;
    for (entity_id, active, _) in peer_states {
        let peer = manager.entity_mut(entity_id).expect("captured peer");
        peer.active = active;
        peer.collision.state_flags_at_0x08 = RetailStateWord::exact(u32::from(active));
    }

    let mut suffix_words = [0x1234_u32, 0xabcd].into_iter();
    let mut suffix_draws = 0;
    let resumed = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut world_fx,
        None,
        &mut |_| {
            suffix_draws += 1;
            suffix_words
                .next()
                .expect("frozen class-7 constructor suffix")
        },
    );
    let acquisition = match resumed.outcome {
        Intro2Type13SchedulerProductionOutcome::C690Transition {
            request: resumed_request,
            transition:
                Intro2Type13C690Transition::Published(
                    Type13InitialBehaviorPublication::SearchAndAttack(publication),
                ),
            primary: None,
            acquisition: Some(acquisition),
            ..
        } => {
            assert_eq!(resumed_request, request);
            assert_eq!(publication.planned.random_sample_low16, 0x4000);
            assert_eq!(
                publication
                    .constructors_by_phase
                    .map(|constructor| constructor.random_sample_low16),
                [0x1234, 0xabcd]
            );
            acquisition
        }
        other => panic!("publication retry must resume after the selector: {other:?}"),
    };
    assert_eq!(suffix_draws, 2, "the selector must not be replayed");
    assert!(suffix_words.next().is_none());
    assert!(
        resumed.retained_owner.is_some(),
        "a target enters pursuit and keeps the specialized owner"
    );

    let entity_after = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("pursuing type-13");
    let [heading, pitch, roll] = entity_after.rotation_heading_pitch_roll_raw();
    assert_eq!(
        entity_after.physical_body_basis_q31(),
        RetailRuntimeValue::Known(
            v2k_game::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(
                heading, pitch, roll,
            ),
        ),
        "a completed None-frame C690 retry must rebuild from the post-task angle words"
    );
    let RetailRuntimeValue::Known(Some(context_after)) = entity_after.current_behavior_context
    else {
        panic!("pursuing context")
    };
    assert!(matches!(
        context_after.target_handle_at_0x08(),
        RetailRuntimeValue::Known(Some(_))
    ));
    assert_eq!(
        context_after.auxiliary_word_at_0x0c(),
        predecessor_context.auxiliary_word_at_0x0c()
    );
    let Some(ActorTaskRuntime::ChaseTarget(chase)) =
        entity_after.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("acquisition publishes newborn Chase")
    };
    let Some(ActorTaskRuntime::AimAndFire(aim)) =
        entity_after.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!("acquisition publishes Aim")
    };
    assert_eq!(chase.elapsed_ms(), 0, "newborn Chase is not revisited");
    assert_eq!(
        aim.elapsed_ms(),
        501,
        "same-pass Aim uses the originating dispatch duration: {acquisition:?}"
    );
}

#[v2k_test_support::retail_test]
fn c690_suppression_retry_consumes_no_root_random_and_preserves_primary() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let type_metadata = intro2_type_metadata(&session);
    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection {
            type13: v2k_game::intro2_type13_live::Intro2Type13BirthSelection::CapturedClass7,
            ..Default::default()
        },
        &mut || 0,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");
    let mut owner = Intro2Type13SchedulerOwner::adopt_published(
        manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
            .expect("published type-13"),
    )
    .expect("published B6C0 graph");
    let other_ids: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.id != owner.entity_id())
        .map(|entity| entity.id)
        .collect();
    for entity_id in other_ids {
        let peer = manager.entity_mut(entity_id).expect("captured peer");
        peer.active = false;
        peer.collision.state_flags_at_0x08 = RetailStateWord::exact(0);
    }

    let mut world_fx = WorldFx::new();
    let mut prime_words = [0xffff_u32, 0xffff].into_iter();
    let prime = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut world_fx,
        Some(primary_frame(&session, 0)),
        &mut |_| prime_words.next().expect("two retarget words"),
    );
    owner = prime.retained_owner.expect("no-target B6C0 owner");
    assert!(prime_words.next().is_none());

    manager
        .entity_mut(owner.entity_id())
        .expect("published type-13")
        .collision
        .state_flags_at_0x08
        .invalidate(TYPE13_C690_SUPPRESS_STATE_BIT);
    let mut transition_words = [1_u32].into_iter();
    let blocked = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut world_fx,
        Some(primary_frame(&session, 501_000)),
        &mut |_| {
            transition_words
                .next()
                .expect("only the old Primary far-target gate")
        },
    );
    let request = match blocked.outcome {
        Intro2Type13SchedulerProductionOutcome::C690Blocked {
            request,
            reason: Intro2Type13C690Block::Plan(Type13C690ReselectionBlock::StateFlagsUnresolved),
            primary: Some(primary),
            ..
        } => {
            assert_eq!(
                primary.post_unwind,
                SharedRetargetPostUnwind::Transition(request)
            );
            request
        }
        other => panic!("unresolved suppression bit must park before planning: {other:?}"),
    };
    assert!(transition_words.next().is_none());
    owner = blocked.retained_owner.expect("pending C690 plan owner");

    let entity_before = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("pending type-13");
    let primary_before = entity_before
        .actor_task_state(ActorTaskSlot::Primary)
        .copied();
    let initial_behavior_before = entity_before.initial_behavior;
    let context_before = entity_before.current_behavior_context;
    manager
        .entity_mut(owner.entity_id())
        .expect("pending type-13")
        .collision
        .state_flags_at_0x08 = RetailStateWord::from_known_bits(
        TYPE13_C690_SUPPRESS_STATE_BIT,
        TYPE13_C690_SUPPRESS_STATE_BIT,
    );

    let mut retry_draws = 0;
    let resumed = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut world_fx,
        None,
        &mut |_| {
            retry_draws += 1;
            0
        },
    );
    match resumed.outcome {
        Intro2Type13SchedulerProductionOutcome::C690Transition {
            request: resumed_request,
            transition: Intro2Type13C690Transition::SuppressedByEntityState,
            primary: None,
            acquisition: Some(_),
            ..
        } => assert_eq!(resumed_request, request),
        other => panic!("suppressed retry must continue at Secondary: {other:?}"),
    }
    assert_eq!(retry_draws, 0, "suppression must precede root RNG");
    assert!(
        resumed.retained_owner.is_some(),
        "the unchanged class-7 graph remains schedulable"
    );
    let entity_after = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("suppressed type-13");
    assert_eq!(
        entity_after
            .actor_task_state(ActorTaskSlot::Primary)
            .copied(),
        primary_before,
        "a pending retry must not revisit or replace the old Primary"
    );
    assert_eq!(entity_after.initial_behavior, initial_behavior_before);
    assert_eq!(entity_after.current_behavior_context, context_before);
}

#[v2k_test_support::retail_test]
fn c690_publication_retry_drops_a_stale_predecessor_context_without_random() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let type_metadata = intro2_type_metadata(&session);
    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection {
            type13: v2k_game::intro2_type13_live::Intro2Type13BirthSelection::CapturedClass7,
            ..Default::default()
        },
        &mut || 0,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");
    let mut owner = Intro2Type13SchedulerOwner::adopt_published(
        manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
            .expect("published type-13"),
    )
    .expect("published B6C0 graph");
    let other_ids: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.id != owner.entity_id())
        .map(|entity| entity.id)
        .collect();
    for entity_id in other_ids {
        let peer = manager.entity_mut(entity_id).expect("captured peer");
        peer.active = false;
        peer.collision.state_flags_at_0x08 = RetailStateWord::exact(0);
    }

    let mut world_fx = WorldFx::new();
    let mut prime_words = [0xffff_u32, 0xffff].into_iter();
    let prime = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut world_fx,
        Some(primary_frame(&session, 0)),
        &mut |_| prime_words.next().expect("two retarget words"),
    );
    owner = prime.retained_owner.expect("no-target B6C0 owner");
    assert!(prime_words.next().is_none());

    let actor_axis = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("published type-13")
        .actor_common_axis_descriptor;
    manager
        .entity_mut(owner.entity_id())
        .expect("published type-13")
        .actor_common_axis_descriptor = RetailRuntimeValue::Unresolved;
    let mut planned_words = [1_u32, 0x4000].into_iter();
    let blocked = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut world_fx,
        Some(primary_frame(&session, 501_000)),
        &mut |_| {
            planned_words
                .next()
                .expect("Primary gate then one root selector")
        },
    );
    match blocked.outcome {
        Intro2Type13SchedulerProductionOutcome::C690Blocked {
            reason:
                Intro2Type13C690Block::Publication(
                    Type13Class7AcquiringError::ActorCommonAxisDescriptorUnresolved,
                ),
            primary: Some(_),
            ..
        } => {}
        other => panic!("publication preflight must retain its exact context: {other:?}"),
    }
    assert!(planned_words.next().is_none());
    owner = blocked
        .retained_owner
        .expect("publication block retains linear custody");

    let entity_before = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("pending type-13");
    let tasks_before = ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .map(|slot| entity_before.actor_task_state(slot).copied());
    let initial_behavior_before = entity_before.initial_behavior;
    let sub_g_before = entity_before.sub_g_06070_runtime;
    let parked_entity = manager
        .entity_mut(owner.entity_id())
        .expect("pending type-13");
    parked_entity.actor_common_axis_descriptor = actor_axis;
    parked_entity.current_behavior_context = RetailRuntimeValue::Unresolved;
    let _ = world_fx.take_positional_sounds();

    let mut retry_draws = 0;
    let dropped = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut world_fx,
        None,
        &mut |_| {
            retry_draws += 1;
            0
        },
    );
    assert!(matches!(
        dropped.outcome,
        Intro2Type13SchedulerProductionOutcome::Dropped {
            reason: Intro2Type13SchedulerProductionDrop::GraphMismatch,
            ..
        }
    ));
    assert_eq!(dropped.retained_owner, None);
    assert_eq!(retry_draws, 0, "a stale continuation cannot consume RNG");
    assert!(
        world_fx.take_positional_sounds().is_empty(),
        "a stale continuation cannot replay the completed Primary's sound"
    );

    let entity_after = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .expect("stale type-13");
    assert_eq!(
        ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .map(|slot| entity_after.actor_task_state(slot).copied()),
        tasks_before,
        "dropping stale custody must not mutate the task graph"
    );
    assert_eq!(entity_after.initial_behavior, initial_behavior_before);
    assert_eq!(entity_after.sub_g_06070_runtime, sub_g_before);
    assert_eq!(
        entity_after.current_behavior_context,
        RetailRuntimeValue::Unresolved
    );
}

#[v2k_test_support::retail_test]
fn central_scheduler_blocks_unresolved_or_live_type13_relation_prelude_before_world_effects() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("normal-tier cumulative Section-12 records");
    session.load_level_by_id(50, 1).expect("Intro2");

    for relation_known in [true, false] {
        let (_, mut manager, task_owner) = captured_intro2_type13(&session);
        let entity_id = task_owner.entity_id();
        manager.set_authored_behavior_components_enabled(INTRO2_TYPE13_SPAWN_INDEX, true);
        let subject = manager.entity_mut(entity_id).expect("published type-13");
        // This activated fixture has already passed Intro2's dormant B2 clear.
        subject.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        if relation_known {
            subject.collision.state_flags_at_0x08.overwrite(
                TYPE13_C690_SUPPRESS_STATE_BIT,
                TYPE13_C690_SUPPRESS_STATE_BIT,
            );
        } else {
            subject
                .collision
                .state_flags_at_0x08
                .invalidate(TYPE13_C690_SUPPRESS_STATE_BIT);
        }
        let before_collision = subject.collision.clone();
        let before_motion = (
            subject.position_raw(),
            subject.velocity_raw(),
            subject.physical_body_basis_q31(),
            subject.mass_raw,
        );
        let before_tasks =
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| subject.actor_task_state(slot).copied());
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_intro2_type13_search_attack(&manager), 1);
        let mut world_fx = WorldFx::new();
        let mut static_damage = v2k_game::static_damage::StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let blocked = scheduler.tick(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    v2k_game::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                resources: &mut session.cache,
                world_fx: &mut world_fx,
                static_damage: &mut static_damage,
                elapsed_micros: 501_000,
                global_elapsed_micros: 501_000,
                retail_tick: 0,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        assert_eq!(blocked.block, None);
        assert!(
            matches!(
                blocked.outcomes.as_slice(),
                [SpecializedActorTaskProductionOutcome::Intro2Type13SearchAttack(
                    v2k_game::intro2_type13_live::Intro2Type13WorldOutcome::Blocked {
                        entity_id: actual_entity_id,
                        reason: v2k_game::intro2_type13_live::Intro2Type13WorldBlock::UnsupportedRelation,
                    },
                )] if *actual_entity_id == entity_id
            ),
            "relation prelude must stop the full world visit: {:?}",
            blocked.outcomes
        );
        let after = manager.entity_mut(entity_id).expect("retained type-13");
        assert_eq!(after.collision, before_collision);
        assert_eq!(
            (
                after.position_raw(),
                after.velocity_raw(),
                after.physical_body_basis_q31(),
                after.mass_raw
            ),
            before_motion,
        );
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| after.actor_task_state(slot).copied()),
            before_tasks,
        );
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            WorldFx::new().next_shared_retail_random_u16(),
            "relation preflight must precede scheduler and task RNG",
        );
        assert!(world_fx.take_positional_sounds().is_empty());
    }
}

#[v2k_test_support::retail_test]
fn b6c0_scheduler_threads_component_state_and_delivers_one_shot_sound() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");
    let type_metadata = intro2_type_metadata(&session);
    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection {
            type13: v2k_game::intro2_type13_live::Intro2Type13BirthSelection::CapturedClass7,
            ..Default::default()
        },
        &mut || 0,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");
    let mut owner = Intro2Type13SchedulerOwner::adopt_published(
        manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX))
            .expect("published type-13"),
    )
    .expect("published B6C0 graph");

    // Keep slot 1 in its no-target B6C0 state so the returned scheduler owner
    // can be threaded across enough callbacks to cross AA60's sound band.
    // These repeated calls include the post-task basis refresh; shared outer
    // scheduler/environment/master-motion phases remain outside this test.
    let other_ids: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.id != owner.entity_id())
        .map(|entity| entity.id)
        .collect();
    for entity_id in other_ids {
        let peer = manager.entity_mut(entity_id).expect("captured peer");
        peer.active = false;
        peer.collision.state_flags_at_0x08 = RetailStateWord::exact(0);
    }
    manager
        .entity_mut(owner.entity_id())
        .expect("published type-13")
        .collision
        .state_flags_at_0x08 = RetailStateWord::from_known_bits(
        TYPE13_C690_SUPPRESS_STATE_BIT,
        TYPE13_C690_SUPPRESS_STATE_BIT,
    );

    let mut world_fx = WorldFx::new();
    let mut heard_sound = false;
    let mut awaiting_latched_followup = false;
    let mut saw_latched_followup = false;
    for frame_index in 0..32u32 {
        let tick = tick_intro2_type13_scheduler_owner(
            &mut manager,
            owner,
            &mut world_fx,
            Some(Intro2Type13PrimaryFrame {
                dispatch_mode:
                    v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
                global_elapsed_micros: 20_000,
                retail_tick: frame_index,
                ..primary_frame(&session, 20_000)
            }),
        );
        let primary_sound = match &tick.outcome {
            Intro2Type13SchedulerProductionOutcome::B6c0Visit { primary, .. } => {
                assert_eq!(primary.result, Intro2Type13PrimaryVisitResult::Continue);
                primary.sound
            }
            Intro2Type13SchedulerProductionOutcome::C690Transition {
                transition: Intro2Type13C690Transition::SuppressedByEntityState,
                primary: Some(primary),
                acquisition: Some(_),
                ..
            } => {
                assert_eq!(primary.result, Intro2Type13PrimaryVisitResult::Continue);
                primary.sound
            }
            other => panic!("expected retained B6C0 visit, got {other:?}"),
        };
        owner = tick
            .retained_owner
            .expect("no-target acquisition must retain the scheduler owner");

        let entity = manager
            .iter_all()
            .find(|entity| entity.id == owner.entity_id())
            .expect("retained type-13 entity");
        if frame_index < 2 {
            assert_eq!(
                entity
                    .intro2_type13_common_mover_runtime
                    .expect("entity-owned common-mover allocation")
                    .sub_d_frame_owner
                    .classifier_cache()
                    .stagger_counter(),
                (frame_index + 1) as u8
            );
        }

        world_fx.process_pending();
        let sounds = world_fx.take_positional_sounds();
        match primary_sound {
            Some(sound) => {
                assert!(!heard_sound, "the first phase-band entry emits once");
                assert_eq!(sound.sound_id, 69);
                assert_eq!(sound.rate_q16, 0x0000_aaaa);
                assert_eq!(sounds.len(), 1);
                assert_eq!(sounds[0].sound_id, 69);
                assert_eq!(sounds[0].frequency_q16, 0x0000_aaaa);
                heard_sound = true;
                awaiting_latched_followup = true;
            }
            None => {
                assert!(sounds.is_empty());
                if awaiting_latched_followup {
                    saw_latched_followup = true;
                    break;
                }
            }
        }
    }
    assert!(
        heard_sound,
        "AA60 phase zero must reach its authored sound band"
    );
    assert!(
        saw_latched_followup,
        "the scheduler must execute one latch-suppressed follow-up"
    );
}
