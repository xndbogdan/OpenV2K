//! Authored 20450 -> 425680 -> AD10/AF90 construction and C690 reselection.
use super::*;
use crate::{
    actor_task_owner::PreparedActorTask,
    common_mover::{
        sub_d::{type8_first_query_owner_for_seed, NativeSubDConstruction, Type9SubDRuntime},
        type9_attitude::Type9BodyBasis,
        SubAPropulsionRuntime,
    },
    entity_behavior::{
        initial_behavior_state_policy, select_initial_behavior, translate_state_policy,
        BehaviorSelection, BehaviorWeightRule,
    },
    entity_collision_state::{
        EntityPairCallbackRuntimeState, PairOrientationPolicy, RetailStateWord,
        ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT, SURFACE_STATE_MASK,
    },
    go_to_job::{
        plan_go_to_job_setup, GoToJobCandidate, GoToJobOwner, GoToJobSetupPlan, GoToJobSetupRequest,
    },
    go_to_job_owner::GoToJobTaskState,
    job_nearby::{
        evaluate_job_nearby, JobNearbyCandidate, JobNearbyEvaluationRequest, JobNearbyOwner,
    },
    ordinary_type9_wander_owner::plan_ordinary_type9_wander_setup,
    wrapped_axis_range::WrappedAxisRange,
};
use std::convert::Infallible;
use v2k_formats::{levels::EntitySpawn, terrain::TerrainGrid};

/// The shared process constructor supplies an actual successful Sub-D allocation
/// and `104B0`'s surface comparison before `D4A0` grounds the authored worker.
pub(crate) struct Type8AuthoredConstructionRequest<'a> {
    pub entity: &'a mut Entity,
    pub allocation: crate::main_base_abort::MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub spawn: &'a EntitySpawn,
    pub preceding: &'a [Entity],
    pub terrain: &'a TerrainGrid,
    pub sub_d: NativeSubDConstruction,
    pub constructor_surface_bits: u32,
}

pub(crate) fn publish_authored_type8(
    request: Type8AuthoredConstructionRequest<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), Intro2Type8Block> {
    let Type8AuthoredConstructionRequest {
        entity,
        allocation,
        metadata,
        spawn,
        preceding,
        terrain,
        sub_d,
        constructor_surface_bits,
    } = request;
    let entity_type = entity.entity_type;
    validate_worker_metadata(entity_type, metadata)?;
    let profile = NativeWorkerProfile::from_entity_type(entity_type)
        .ok_or(Intro2Type8Block::Runtime("worker entity type"))?;
    let expected_model = profile.model_id();
    if !entity.active || entity.id != allocation.entity_id
        || spawn.entity_type != entity_type || entity.authored_spawn_index != Some(spawn.index)
        || entity.model_slots != [Some(expected_model); 4] || entity.capability_flags != 0x1404
        || entity.rotation_heading_pitch_roll_raw() != spawn.rotation.map(|word| word as i16)
        || entity.intro2_type8_runtime.is_some()
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || entity.initial_behavior != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER.iter().any(|&s| entity.actor_tasks.task_in_slot(s).is_some())
        || preceding.iter().any(|e| e.id == entity.id || e.authored_spawn_index.is_some_and(|i| i >= spawn.index))
        || preceding.windows(2).any(|pair| matches!((pair[0].authored_spawn_index, pair[1].authored_spawn_index), (Some(a), Some(b)) if a >= b))
    {
        return Err(Intro2Type8Block::Runtime("authored Type8 birth"));
    }
    if constructor_surface_bits & !SURFACE_STATE_MASK != 0 {
        return Err(Intro2Type8Block::Runtime("constructor surface bits"));
    }
    validate_native_components(entity, metadata, sub_d)?;
    let RetailRuntimeValue::Known(Some(sub_a_descriptor)) = metadata.sub_a_propulsion_descriptor
    else {
        unreachable!();
    };
    if entity.sub_a_propulsion_runtime
        != RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::pending_constructor_rng(
            sub_a_descriptor,
        )))
    {
        return Err(Intro2Type8Block::Runtime(
            "unconsumed native Type8 Sub-A constructor",
        ));
    }
    let [x, _, z] = spawn.position_raw();
    let cell = terrain
        .cell(usize::from((x as u16) >> 8), usize::from((z as u16) >> 8))
        .ok_or(Intro2Type8Block::Runtime("authored terrain cell"))?;
    let anchor = [x, terrain.bilinear_height_raw(x, z), z];
    let mut state =
        RetailStateWord::exact(0x0607_8801 | if spawn.param != 0 { 0x0100_0000 } else { 0 });
    state.overwrite(SURFACE_STATE_MASK, constructor_surface_bits);
    state.overwrite(
        ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
        if cell.terrain_type & 0x10 != 0 {
            ACTIVE_MODEL_SLOT_HIGH_STATE_BIT
        } else {
            0
        },
    );
    let policy = translate_state_policy(
        metadata
            .initializer
            .as_ref()
            .unwrap()
            .initializer_state_flags_raw,
    );
    state.overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    entity.collision.state_flags_at_0x08 = state;
    entity.set_position_raw(anchor);
    entity.model_index = Some(expected_model);
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(
        metadata
            .initializer
            .as_ref()
            .unwrap()
            .common_axis_descriptor,
    );
    entity.type8_wander_anchor_raw_at_0x90 = RetailRuntimeValue::Known(anchor);
    entity.type8_sub_d_frame_owner = Some(sub_d.frame_owner);
    entity.type8_sub_d_runtime = Some(sub_d.runtime);
    // 20450 consumes the first constructor word, before 425680 evaluates its
    // weighted predicates and AD10/AF90 draws the selected task's word.
    let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_a_propulsion_descriptor else {
        unreachable!();
    };
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
        SubAPropulsionRuntime::from_20450_constructor(descriptor, next_random() as u16),
    ));
    let candidates = preceding.iter().map(candidate).collect::<Vec<_>>();
    let plan = plan_root(entity, metadata, &candidates, entity.position_raw())?;
    let selection = apply_root(entity, metadata, plan, next_random)?;
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    retain_native_type8_runtime(entity, allocation, metadata, sub_d)
}

fn validate_native_components(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    sub_d: NativeSubDConstruction,
) -> Result<(), Intro2Type8Block> {
    let profile = NativeWorkerProfile::from_entity_type(entity.entity_type)
        .ok_or(Intro2Type8Block::Runtime("worker entity type"))?;
    if sub_d.descriptor != profile.sub_d()
        || sub_d.runtime != Type9SubDRuntime::from_constructor()
        || sub_d.frame_owner.classifier_cache().stagger_counter() != sub_d.seed
        || entity.collision.health_raw != RetailRuntimeValue::Known(profile.health())
        || !matches!(
            entity.sub_a_propulsion_runtime,
            RetailRuntimeValue::Known(Some(_))
        )
        || !matches!(entity.actor_animation_runtime, RetailRuntimeValue::Known(Some(animation))
            if RetailRuntimeValue::Known(Some(animation.descriptor())) == metadata.actor_animation_descriptor)
    {
        return Err(Intro2Type8Block::Runtime("native Type8 components"));
    }
    Ok(())
}

/// Retain an already-completed native worker constructor, including dynamic
/// factory/Main Base births. This boundary consumes no RNG and selects no task.
/// The caller supplies the actual component allocation instead of a replay seed.
pub(crate) fn retain_native_type8_runtime(
    entity: &mut Entity,
    allocation: crate::main_base_abort::MainBaseAbortActorLease,
    metadata: &EntityTypeRuntimeMetadata,
    sub_d: NativeSubDConstruction,
) -> Result<(), Intro2Type8Block> {
    let entity_type = entity.entity_type;
    validate_worker_metadata(entity_type, metadata)?;
    validate_native_components(entity, metadata, sub_d)?;
    let (task_id, context, kind) = Intro2Type8Owner::published_graph(entity)
        .ok_or(Intro2Type8Block::Runtime("completed native Type8 graph"))?;
    let profile = NativeWorkerProfile::from_entity_type(entity_type)
        .ok_or(Intro2Type8Block::Runtime("worker entity type"))?;
    let expected_model = profile.model_id();
    if !entity.active
        || entity.id != allocation.entity_id
        || entity.model_slots != [Some(expected_model); 4]
        || entity.capability_flags != 0x1404
        || entity.intro2_type8_runtime.is_some()
        || kind == TaskKind::Exploding
        || entity.type8_sub_d_frame_owner != Some(sub_d.frame_owner)
        || entity.type8_sub_d_runtime != Some(sub_d.runtime)
        || entity.type8_wander_anchor_raw_at_0x90
            != RetailRuntimeValue::Known(entity.position_raw())
    {
        return Err(Intro2Type8Block::Runtime(
            "completed native Type8 allocation",
        ));
    }
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    entity.collision.pair_callbacks = EntityPairCallbackRuntimeState::audited_local(
        Some(crate::descriptor_contact::DESCRIPTOR_CONTACT_CALLBACK_ADDRESS),
        RetailRuntimeValue::Known(PairOrientationPolicy::LiveHeadingWithAuthoredPitchRoll {
            pitch_raw: pitch as u16,
            roll_raw: roll as u16,
        }),
    );
    // +B2 has no generic constructor writer. Native allocations deliberately
    // initialize that transient storage; the Intro2 replay keeps its separately
    // observed zero. No captured first-scheduler state is copied into this path.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.intro2_type8_runtime = Some(Intro2Type8Runtime {
        allocation,
        profile,
        entity_id: entity.id,
        spawn_index: entity.authored_spawn_index,
        anchor_raw: entity.position_raw(),
        model_slots: entity.model_slots,
        sub_d_seed: sub_d.seed,
        origin: Type8ConstructionOrigin::Native,
        birth_task_id: task_id,
        birth_context: context,
        birth_pending: true,
        relation_graph: None,
    });
    Ok(())
}

pub(super) fn validate_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Intro2Type8Block> {
    validate_worker_metadata(8, metadata)
}

pub(crate) fn publish_intro2_type8(
    entity: &mut Entity,
    allocation: crate::main_base_abort::MainBaseAbortActorLease,
    metadata: &EntityTypeRuntimeMetadata,
    preceding: &[Entity],
    terrain: &TerrainGrid,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), Intro2Type8Block> {
    validate_metadata(metadata)?;
    let spawn = entity
        .authored_spawn_index
        .ok_or(Intro2Type8Block::Runtime("authored spawn"))?;
    let seed = seed_for_spawn(spawn).ok_or(Intro2Type8Block::Runtime("authored Type8 spawn"))?;
    let xz = match spawn {
        12 => [83u16 << 8, 227u16 << 8],
        13 => [80u16 << 8, 228u16 << 8],
        15 => [87u16 << 8, 232u16 << 8],
        _ => unreachable!(),
    }
    .map(|v| v as i16);
    if !entity.active
        || entity.id != allocation.entity_id
        || entity.entity_type != 8
        || entity.model_slots != [Some(559); 4]
        || [entity.position_raw()[0], entity.position_raw()[2]] != xz
        || entity.rotation_heading_pitch_roll_raw() != [0; 3]
        || entity.intro2_type8_runtime.is_some()
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .iter()
            .any(|&s| entity.actor_tasks.task_in_slot(s).is_some())
        || preceding
            .iter()
            .any(|e| e.authored_spawn_index.is_none_or(|i| i >= spawn))
    {
        return Err(Intro2Type8Block::Runtime("native Type8 birth"));
    }
    let anchor = [xz[0], terrain.bilinear_height_raw(xz[0], xz[1]), xz[1]];
    entity.set_position_raw(anchor);
    let candidates = preceding.iter().map(candidate).collect::<Vec<_>>();
    let plan = plan_root(entity, metadata, &candidates, entity.position_raw())?;
    let RetailRuntimeValue::Known(Some(sub_a)) = metadata.sub_a_propulsion_descriptor else {
        unreachable!();
    };
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
        SubAPropulsionRuntime::from_20450_constructor(sub_a, next_random() as u16),
    ));
    let selection = apply_root(entity, metadata, plan, next_random)?;
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(
        metadata
            .initializer
            .as_ref()
            .unwrap()
            .common_axis_descriptor,
    );
    entity.type8_wander_anchor_raw_at_0x90 = RetailRuntimeValue::Known(anchor);
    entity.type8_sub_d_frame_owner = type8_first_query_owner_for_seed(seed);
    entity.type8_sub_d_runtime = Some(Type9SubDRuntime::from_constructor());
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0, 0, 0));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    // V200002: all three exact allocations retain B2=0 at constructor
    // return, first12DA0 and same-visit130F6 mass read (base AX=10).
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        unreachable!();
    };
    entity.intro2_type8_runtime = Some(Intro2Type8Runtime {
        allocation,
        profile: NativeWorkerProfile::Type8,
        entity_id: entity.id,
        spawn_index: Some(spawn),
        anchor_raw: anchor,
        model_slots: entity.model_slots,
        sub_d_seed: seed,
        origin: Type8ConstructionOrigin::CapturedIntro2,
        birth_task_id: entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap(),
        birth_context: context,
        birth_pending: true,
        relation_graph: None,
    });
    Ok(())
}

pub(super) fn candidate(entity: &Entity) -> GoToJobCandidate {
    GoToJobCandidate {
        id: entity.id,
        position_raw: entity.position_raw(),
        state_flags: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        capacity: crate::ordinary_type9_wander_production::job_capacity_from_entity(entity),
    }
}

pub(super) struct RootPlan {
    job_nearby: bool,
    job_owner: GoToJobOwner,
    job: Option<GoToJobSetupPlan>,
}

pub(super) fn plan_root(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    candidates: &[GoToJobCandidate],
    position_raw: [i16; 3],
) -> Result<RootPlan, Intro2Type8Block> {
    validate_worker_metadata(entity.entity_type, metadata)?;
    let range = WrappedAxisRange::from_raw(
        metadata
            .initializer
            .as_ref()
            .unwrap()
            .common_axis_descriptor
            .strict_axis_limit_raw,
    );
    let jobs = candidates
        .iter()
        .map(|c| JobNearbyCandidate {
            id: c.id,
            position_raw: c.position_raw,
            state_flags: c.state_flags,
            capacity: c.capacity,
        })
        .collect::<Vec<_>>();
    let job_nearby = evaluate_job_nearby(JobNearbyEvaluationRequest {
        owner: JobNearbyOwner {
            id: entity.id,
            position_raw,
            capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        },
        candidates_in_intrusive_order: &jobs,
        range,
    })
    .map_err(|_| Intro2Type8Block::Runtime("Job Nearby evidence"))?;
    let job_owner = GoToJobOwner::from_type_metadata(
        entity.id,
        position_raw,
        RetailRuntimeValue::Known(entity.capability_flags),
        entity.entity_type as u16,
        metadata,
    );
    let job = if job_nearby {
        Some(
            plan_go_to_job_setup(GoToJobSetupRequest {
                owner: job_owner,
                candidates_in_intrusive_order: candidates,
                range,
            })
            .map_err(|_| Intro2Type8Block::Runtime("Go-To-Job evidence"))?,
        )
    } else {
        None
    };
    Ok(RootPlan {
        job_nearby,
        job_owner,
        job,
    })
}

pub(super) fn apply_root(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    plan: RootPlan,
    next_random: &mut impl FnMut() -> u32,
) -> Result<BehaviorSelection, Intro2Type8Block> {
    let selection = select_initial_behavior(
        &metadata.initializer.as_ref().unwrap().behavior_choices,
        |rule| match rule {
            BehaviorWeightRule::Always => 1,
            BehaviorWeightRule::JobNearby => i32::from(plan.job_nearby),
            _ => unreachable!(),
        },
        &mut *next_random,
    )
    .map_err(|_| Intro2Type8Block::Runtime("root selection"))?
    .ok_or(Intro2Type8Block::Runtime("empty root"))?;
    let context = match entity.current_behavior_context {
        RetailRuntimeValue::Unresolved => {
            BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        }
        RetailRuntimeValue::Known(Some(context)) => context.reselect_named_type_default(
            selection.program,
            selection.program.initial_style_table_index_raw,
            selection.program.initial_style,
        ),
        _ => None,
    }
    .ok_or(Intro2Type8Block::Runtime("root context"))?;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    match selection.program.class_id {
        54 => {
            plan.job
                .unwrap()
                .apply(
                    plan.job_owner
                        .bind_entity_runtime(entity)
                        .map_err(|_| Intro2Type8Block::Runtime("Sub-A"))?,
                    &mut *next_random,
                    |spec| {
                        Ok::<_, Infallible>(PreparedActorTask::new(ActorTaskRuntime::GoToJob(
                            GoToJobTaskState::after_allocation(spec),
                        )))
                    },
                )
                .expect("infallible native task allocation");
        }
        6 => {
            let position = entity.position_raw();
            let RetailRuntimeValue::Known(Some(sub_a)) = &mut entity.sub_a_propulsion_runtime
            else {
                return Err(Intro2Type8Block::Runtime("Sub-A"));
            };
            let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_a_propulsion_descriptor
            else {
                unreachable!();
            };
            plan_ordinary_type9_wander_setup(position, descriptor.target_speed_base_raw)
                .apply(&mut entity.actor_tasks, sub_a, |spec| {
                    Ok::<_, Infallible>(
                        spec.prepare_after_allocation(&mut *next_random)
                            .map_task(ActorTaskRuntime::OrdinaryType9Wander),
                    )
                })
                .expect("infallible native task allocation");
        }
        _ => unreachable!(),
    }
    Ok(selection)
}

pub(super) fn reselect(
    manager: &mut EntityManager,
    id: u32,
    world_fx: &mut WorldFx,
) -> Result<Intro2Type8Owner, Intro2Type8Block> {
    let entity_type = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .map(|entity| entity.entity_type)
        .ok_or(Intro2Type8Block::Runtime("entity"))?;
    let metadata = manager
        .type_runtime_metadata(entity_type)
        .cloned()
        .ok_or(Intro2Type8Block::Runtime("metadata"))?;
    let candidates = manager
        .retail_live_order_ids()
        .filter_map(|id| manager.iter_all().find(|e| e.id == id))
        .map(candidate)
        .collect::<Vec<_>>();
    let entity = manager
        .entity_mut(id)
        .ok_or(Intro2Type8Block::Runtime("entity"))?;
    let plan = plan_root(entity, &metadata, &candidates, entity.position_raw())?;
    apply_root(entity, &metadata, plan, &mut || {
        u32::from(world_fx.next_shared_retail_random_u16())
    })?;
    Intro2Type8Owner::adopt_published(entity).ok_or(Intro2Type8Block::Runtime("root publication"))
}
