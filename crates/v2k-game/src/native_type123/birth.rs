//! Authored 20450 -> 425680 -> BA40/AD10 construction and C690 reselection.
use super::*;
use crate::{
    actor_task_owner::PreparedActorTask,
    attract_attention::{
        execute_attract_attention_initial_setup, AttractAttentionInitialSetupAdapter,
        AttractAttentionInitialTaskPreparation, AttractAttentionPositionalSoundRequest,
        AttractAttentionResourceTextRequest,
    },
    common_mover::{
        actor_abdi::ActorAbdiTopology,
        sub_d::{NativeSubDConstruction, Type9SubDRuntime, ORDINARY_TYPE90_SUB_D},
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
    ordinary_type9_wander_owner::plan_ordinary_type9_wander_setup,
};
use std::convert::Infallible;
use v2k_formats::{levels::EntitySpawn, terrain::TerrainGrid};

/// The shared process constructor supplies an actual successful Sub-D allocation
/// and `104B0`'s surface comparison before `D4A0` grounds the authored person.
pub(crate) struct Type123AuthoredConstructionRequest<'a> {
    pub entity: &'a mut Entity,
    pub allocation: crate::main_base_abort::MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub spawn: &'a EntitySpawn,
    pub preceding: &'a [Entity],
    pub terrain: &'a TerrainGrid,
    pub sub_d: NativeSubDConstruction,
    pub constructor_surface_bits: u32,
}

pub(crate) fn publish_authored_type123(
    request: Type123AuthoredConstructionRequest<'_>,
    world_fx: &mut WorldFx,
) -> Result<Option<AttractAttentionResourceTextRequest>, Type123Block> {
    let Type123AuthoredConstructionRequest {
        entity,
        allocation,
        metadata,
        spawn,
        preceding,
        terrain,
        sub_d,
        constructor_surface_bits,
    } = request;
    validate_metadata(metadata)?;
    if !entity.active
        || entity.id != allocation.entity_id
        || entity.entity_type != 123
        || spawn.entity_type != 123
        || entity.authored_spawn_index != Some(spawn.index)
        || entity.model_slots != [Some(MODEL); 4]
        || entity.capability_flags != CAPABILITY
        || entity.rotation_heading_pitch_roll_raw() != spawn.rotation.map(|word| word as i16)
        || entity.native_type123_runtime.is_some()
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || entity.initial_behavior != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .iter()
            .any(|&s| entity.actor_tasks.task_in_slot(s).is_some())
        || preceding
            .iter()
            .any(|e| e.id == entity.id || e.authored_spawn_index.is_some_and(|i| i >= spawn.index))
        || preceding.windows(2).any(|pair| {
            matches!(
                (pair[0].authored_spawn_index, pair[1].authored_spawn_index),
                (Some(a), Some(b)) if a >= b
            )
        })
    {
        return Err(Type123Block::Runtime("authored Type123 birth"));
    }
    if constructor_surface_bits & !SURFACE_STATE_MASK != 0 {
        return Err(Type123Block::Runtime("constructor surface bits"));
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
        return Err(Type123Block::Runtime(
            "unconsumed native Type123 Sub-A constructor",
        ));
    }
    let [x, _, z] = spawn.position_raw();
    let cell = terrain
        .cell(usize::from((x as u16) >> 8), usize::from((z as u16) >> 8))
        .ok_or(Type123Block::Runtime("authored terrain cell"))?;
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
    entity.model_index = Some(MODEL);
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(
        metadata
            .initializer
            .as_ref()
            .unwrap()
            .common_axis_descriptor,
    );
    entity.native_type123_anchor_raw_at_0x90 = RetailRuntimeValue::Known(anchor);
    entity.type8_sub_d_frame_owner = Some(sub_d.frame_owner);
    entity.type8_sub_d_runtime = Some(sub_d.runtime);
    // 20450 consumes the first constructor word, before 425680 evaluates its
    // Always-weighted predicates and BA40/AD10 draws the selected task's words.
    let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_a_propulsion_descriptor else {
        unreachable!();
    };
    entity.sub_a_propulsion_runtime =
        RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_20450_constructor(
            descriptor,
            u32::from(world_fx.next_shared_retail_random_u16()) as u16,
        )));
    let selection = apply_root(entity, metadata, world_fx)?;
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection.selection));
    let receipt = selection.resource_text;
    retain_native_type123_runtime(entity, allocation, metadata, sub_d)?;
    Ok(receipt)
}

/// 451C00 miss path: the same Always-weighted BA40/AD10 birth as the authored
/// owner, grounded at the zero-record pose instead of an authored spawn. The
/// zero body already carries zero Euler words, the pending Sub-A constructor,
/// the Sub-I controller and authored health through the shared
/// `zero_record_body`; authored submission order has no meaning without a
/// spawn index and is not consulted. RNG order (20450, selector, branch
/// words) matches the authored birth exactly, and the optional BA40
/// resource-text receipt returns for the shared canonical-owner vec.
pub(crate) struct Type123ZeroRecordConstruction<'a> {
    pub entity: &'a mut Entity,
    pub allocation: crate::main_base_abort::MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub terrain: &'a TerrainGrid,
    pub sub_d: NativeSubDConstruction,
    pub constructor_surface_bits: u32,
}

pub(crate) fn publish_zero_record_type123(
    request: Type123ZeroRecordConstruction<'_>,
    world_fx: &mut WorldFx,
) -> Result<Option<AttractAttentionResourceTextRequest>, Type123Block> {
    let Type123ZeroRecordConstruction {
        entity,
        allocation,
        metadata,
        terrain,
        sub_d,
        constructor_surface_bits,
    } = request;
    validate_metadata(metadata)?;
    if !entity.active
        || entity.id != allocation.entity_id
        || entity.entity_type != 123
        || entity.authored_spawn_index.is_some()
        || entity.model_slots != [Some(MODEL); 4]
        || entity.capability_flags != CAPABILITY
        || entity.native_type123_runtime.is_some()
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || entity.initial_behavior != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .iter()
            .any(|&s| entity.actor_tasks.task_in_slot(s).is_some())
    {
        return Err(Type123Block::Runtime("zero-record Type123 birth"));
    }
    if constructor_surface_bits & !SURFACE_STATE_MASK != 0 {
        return Err(Type123Block::Runtime("constructor surface bits"));
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
        return Err(Type123Block::Runtime(
            "unconsumed native Type123 Sub-A constructor",
        ));
    }
    let anchor = [0, terrain.bilinear_height_raw(0, 0), 0];
    let cell = terrain
        .cell(0, 0)
        .ok_or(Type123Block::Runtime("zero-record terrain cell"))?;
    let mut state = RetailStateWord::exact(0x0607_8801);
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
    entity.model_index = Some(MODEL);
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(
        metadata
            .initializer
            .as_ref()
            .unwrap()
            .common_axis_descriptor,
    );
    entity.native_type123_anchor_raw_at_0x90 = RetailRuntimeValue::Known(anchor);
    entity.type8_sub_d_frame_owner = Some(sub_d.frame_owner);
    entity.type8_sub_d_runtime = Some(sub_d.runtime);
    // 20450 consumes the first constructor word, before 425680 evaluates its
    // Always-weighted predicates and BA40/AD10 draws the selected task's words.
    let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_a_propulsion_descriptor else {
        unreachable!();
    };
    entity.sub_a_propulsion_runtime =
        RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_20450_constructor(
            descriptor,
            u32::from(world_fx.next_shared_retail_random_u16()) as u16,
        )));
    let selection = apply_root(entity, metadata, world_fx)?;
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection.selection));
    let receipt = selection.resource_text;
    retain_native_type123_runtime(entity, allocation, metadata, sub_d)?;
    Ok(receipt)
}

fn validate_native_components(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    sub_d: NativeSubDConstruction,
) -> Result<(), Type123Block> {
    if sub_d.descriptor != ORDINARY_TYPE90_SUB_D
        || sub_d.runtime != Type9SubDRuntime::from_constructor()
        || sub_d.frame_owner.classifier_cache().stagger_counter() != sub_d.seed
        || entity.collision.health_raw != RetailRuntimeValue::Known(HEALTH)
        || !matches!(
            entity.sub_a_propulsion_runtime,
            RetailRuntimeValue::Known(Some(_))
        )
        || !matches!(entity.actor_animation_runtime, RetailRuntimeValue::Known(Some(animation))
            if RetailRuntimeValue::Known(Some(animation.descriptor())) == metadata.actor_animation_descriptor)
    {
        return Err(Type123Block::Runtime("native Type123 components"));
    }
    Ok(())
}

fn retain_native_type123_runtime(
    entity: &mut Entity,
    allocation: crate::main_base_abort::MainBaseAbortActorLease,
    metadata: &EntityTypeRuntimeMetadata,
    sub_d: NativeSubDConstruction,
) -> Result<(), Type123Block> {
    validate_metadata(metadata)?;
    let (task_id, context, kind) = Type123Owner::published_graph(entity)
        .ok_or(Type123Block::Runtime("completed native Type123 graph"))?;
    if !entity.active
        || entity.id != allocation.entity_id
        || entity.entity_type != 123
        || entity.model_slots != [Some(MODEL); 4]
        || entity.capability_flags != CAPABILITY
        || entity.native_type123_runtime.is_some()
        || kind == TaskKind::Exploding
        || entity.type8_sub_d_frame_owner != Some(sub_d.frame_owner)
        || entity.type8_sub_d_runtime != Some(sub_d.runtime)
        || entity.native_type123_anchor_raw_at_0x90
            != RetailRuntimeValue::Known(entity.position_raw())
    {
        return Err(Type123Block::Runtime("completed native Type123 allocation"));
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
    // initialize that transient storage. No captured first-scheduler state is
    // copied into this path.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.native_type123_runtime = Some(NativeType123Runtime {
        allocation,
        entity_id: entity.id,
        spawn_index: entity.authored_spawn_index,
        anchor_raw: entity.position_raw(),
        model_slots: entity.model_slots,
        sub_d_seed: sub_d.seed,
        birth_task_id: task_id,
        birth_context: context,
        birth_pending: true,
        relation_graph: None,
    });
    Ok(())
}

pub(super) fn validate_metadata(metadata: &EntityTypeRuntimeMetadata) -> Result<(), Type123Block> {
    let Some(init) = metadata.initializer.as_ref() else {
        return Err(Type123Block::Runtime("initializer"));
    };
    let animation_cues = match metadata.actor_animation_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => Some((
            descriptor.capability_bit_3_sound_id,
            descriptor.capability_mask_0x201_sound_id,
            descriptor.attention_stop_sound_id,
        )),
        _ => None,
    };
    if metadata.model_slots != [MODEL as u16; 4]
        || animation_cues != Some((0, 0, ATTENTION_STOP_SOUND))
        || metadata.capability_flags != CAPABILITY
        || metadata.mass_raw != 10
        || metadata.initial_health_raw != Some(HEALTH as i32)
        || init.initializer_state_flags_raw != 0x2f
        || init.behavior_rule_ref != 1
        || init.alternate_behavior_class_ref != 14
        || init.common_axis_descriptor.strict_axis_limit_raw != 0x600
        || init.common_axis_descriptor.raw_word_at_0x04 != 0x04
        || init
            .behavior_choices
            .iter()
            .map(|c| (c.weight_rule_id, c.weight_multiplier, c.behavior_class_id))
            .collect::<Vec<_>>()
            != [(1, 3, 45), (1, 1, 6)]
        || metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
        || metadata.death_sound_id != RetailRuntimeValue::Known(Some(DEATH_SOUND))
        || metadata.accepted_hit_presentation_sound_id
            != RetailRuntimeValue::Known(Some(ACCEPTED_HIT_SOUND))
        || ActorAbdiTopology::from_metadata(123, metadata).is_err()
    {
        return Err(Type123Block::Runtime("canonical Type123 metadata"));
    }
    Ok(())
}

struct RootApplication {
    selection: BehaviorSelection,
    resource_text: Option<AttractAttentionResourceTextRequest>,
}

fn apply_root(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
) -> Result<RootApplication, Type123Block> {
    // Each RNG scope below borrows the shared stream separately so the
    // selector word, BA40 parity/suffix words and Wander constructor words
    // land in exact retail order without overlapping borrows.
    let selection = {
        let mut next_random = || u32::from(world_fx.next_shared_retail_random_u16());
        select_initial_behavior(
            &metadata.initializer.as_ref().unwrap().behavior_choices,
            |rule| match rule {
                BehaviorWeightRule::Always => 1,
                _ => unreachable!("exact Type123 Always-only choices"),
            },
            &mut next_random,
        )
        .map_err(|_| Type123Block::Runtime("root selection"))?
        .ok_or(Type123Block::Runtime("empty root"))?
    };
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
    .ok_or(Type123Block::Runtime("root context"))?;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    let resource_text = match selection.program.class_id {
        45 => apply_attract(entity, metadata, world_fx)?,
        6 => {
            let mut next_random = || u32::from(world_fx.next_shared_retail_random_u16());
            apply_wander(entity, metadata, &mut next_random)?;
            None
        }
        _ => unreachable!("exact Type123 choices"),
    };
    Ok(RootApplication {
        selection,
        resource_text,
    })
}

fn apply_wander(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), Type123Block> {
    // AD10 installs only the Primary task. Retire any stale Secondary/
    // Tertiary tasks from a previous Attract variant first: the cue
    // destructor (2AC0 -> 20830) clears the forced-stop byte, which is what
    // makes the Wander mover neutral again after an Attract graph.
    let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
        return Err(Type123Block::Runtime("Sub-I"));
    };
    for slot in [ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary] {
        entity
            .actor_tasks
            .clear_slot_with_retirement(slot, |task| task.retire_animation(animation));
    }
    let position = entity.position_raw();
    let RetailRuntimeValue::Known(Some(sub_a)) = &mut entity.sub_a_propulsion_runtime else {
        return Err(Type123Block::Runtime("Sub-A"));
    };
    let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_a_propulsion_descriptor else {
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
    Ok(())
}

struct BirthAdapter<'a> {
    position_raw: [i16; 3],
    resource_text: Option<AttractAttentionResourceTextRequest>,
    world_fx: &'a mut WorldFx,
}

impl AttractAttentionInitialSetupAdapter<ActorTaskRuntime> for BirthAdapter<'_> {
    type PrepareError = Infallible;

    fn retire_task(
        &mut self,
        task: &ActorTaskRuntime,
        animation: &mut crate::actor_animation::ActorAnimationController,
    ) {
        task.retire_animation(animation);
    }

    fn next_shared_random_u16(&mut self) -> u16 {
        self.world_fx.next_shared_retail_random_u16()
    }

    fn prepare_task(
        &mut self,
        preparation: AttractAttentionInitialTaskPreparation,
    ) -> Result<PreparedActorTask<ActorTaskRuntime>, Self::PrepareError> {
        Ok(prepare_attract_task(self.position_raw, preparation))
    }

    fn dispatch_resource_text(&mut self, request: AttractAttentionResourceTextRequest) {
        debug_assert!(self.resource_text.is_none());
        self.resource_text = Some(request);
    }

    fn emit_positional_sound(&mut self, request: AttractAttentionPositionalSoundRequest) {
        self.world_fx
            .queue_fixed_positional_sound_raw(request.global_sound_id, request.position_raw);
    }
}

pub(super) fn prepare_attract_task(
    position_raw: [i16; 3],
    preparation: AttractAttentionInitialTaskPreparation,
) -> PreparedActorTask<ActorTaskRuntime> {
    use crate::attract_attention::{
        AttractAttentionTaskConstructorInputs, AttractAttentionTaskRole,
        ATTRACT_ATTENTION_CANDIDATE_TASK, ATTRACT_ATTENTION_CUE_TASK,
        ATTRACT_ATTENTION_WANDER_TASK,
    };
    use crate::shared_retarget_mover::SharedRetargetTaskState;
    match preparation.task.role {
        AttractAttentionTaskRole::AcquireCandidate => {
            debug_assert_eq!(preparation.task, ATTRACT_ATTENTION_CANDIDATE_TASK);
            let AttractAttentionTaskConstructorInputs::AcquireCandidate {
                constructor_context_raw,
                ..
            } = preparation.constructor_inputs
            else {
                unreachable!();
            };
            PreparedActorTask::new(ActorTaskRuntime::AttractAttentionCandidate(
                crate::attract_attention::AttractAttentionCandidateTaskState::new(
                    constructor_context_raw,
                ),
            ))
        }
        AttractAttentionTaskRole::AttentionCue => {
            debug_assert_eq!(preparation.task, ATTRACT_ATTENTION_CUE_TASK);
            let AttractAttentionTaskConstructorInputs::AttentionCue { lifetime_ms } =
                preparation.constructor_inputs
            else {
                unreachable!();
            };
            PreparedActorTask::new(ActorTaskRuntime::AttractAttentionCue(
                crate::attract_attention::AttractAttentionCueTaskState::new(lifetime_ms),
            ))
        }
        AttractAttentionTaskRole::LocalWander => {
            debug_assert_eq!(preparation.task, ATTRACT_ATTENTION_WANDER_TASK);
            PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                SharedRetargetTaskState::new(position_raw, 1000),
            ))
        }
        AttractAttentionTaskRole::RouteToTarget => {
            unreachable!("initial style cannot prepare target-route tasks")
        }
    }
}

fn apply_attract(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
) -> Result<Option<AttractAttentionResourceTextRequest>, Type123Block> {
    use crate::attract_attention::AttractAttentionInitialExecutionRequest;
    let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_a_propulsion_descriptor else {
        unreachable!();
    };
    let position_raw = entity.position_raw();
    let RetailRuntimeValue::Known(Some(sub_a)) = &mut entity.sub_a_propulsion_runtime else {
        return Err(Type123Block::Runtime("Sub-A"));
    };
    let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
        return Err(Type123Block::Runtime("Sub-I"));
    };
    // The BA40 adapter draws parity + suffix words from the shared stream in
    // retail order; the outer selector word was already consumed above.
    // Sounds queue to WorldFx (no phase gate); the resource-text event is
    // retained for the canonical load drain, which admits ordinary native
    // Section-13 construction as gameplay phase 5.
    let mut adapter = BirthAdapter {
        position_raw,
        resource_text: None,
        world_fx,
    };
    execute_attract_attention_initial_setup(
        &mut entity.actor_tasks,
        sub_a,
        animation,
        AttractAttentionInitialExecutionRequest {
            sub_a_target_speed_base_raw: descriptor.target_speed_base_raw,
            position_raw,
        },
        &mut adapter,
    )
    .map_err(|_| Type123Block::Runtime("attract initializer"))?;
    Ok(adapter.resource_text)
}

pub(super) fn reselect(
    manager: &mut EntityManager,
    id: u32,
    world_fx: &mut WorldFx,
) -> Result<Type123Owner, Type123Block> {
    let metadata = manager
        .type_runtime_metadata(123)
        .cloned()
        .ok_or(Type123Block::Runtime("metadata"))?;
    let entity = manager
        .entity_mut(id)
        .ok_or(Type123Block::Runtime("entity"))?;
    apply_root(entity, &metadata, world_fx)?;
    Type123Owner::adopt_published(entity).ok_or(Type123Block::Runtime("root publication"))
}
