//! Authored 20450 -> 425680 -> BA40/B6C0/AF90/AD10 construction and C690 reselection.
use super::*;
use crate::{
    actor_task_dispatcher::{
        prepare_shared_acquiring_runtime_task, SharedGenericConstructorEffect,
    },
    actor_task_owner::PreparedActorTask,
    attract_attention::{
        execute_attract_attention_initial_setup, AttractAttentionInitialSetupAdapter,
        AttractAttentionInitialTaskPreparation, AttractAttentionPositionalSoundRequest,
        AttractAttentionResourceTextRequest,
    },
    common_mover::{
        actor_abdi::ActorAbdiTopology,
        sub_d::{NativeSubDConstruction, Type9SubDRuntime},
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
    go_to_job::{plan_go_to_job_setup, GoToJobCandidate, GoToJobOwner, GoToJobSetupRequest},
    guard_location_owner::acquisition::{
        select_guard_location_candidate, GuardLocationCandidateFilter,
        GuardLocationCandidateRequest, GuardLocationEntityRef, GuardLocationSearchContext,
    },
    ordinary_type9_wander_owner::plan_ordinary_type9_wander_setup,
    run_away::{apply_run_away_task_setup_with_retirement, RunAwayTaskSetupRequest},
    wrapped_axis_range::WrappedAxisRange,
};
use std::convert::Infallible;
use v2k_formats::{levels::EntitySpawn, terrain::TerrainGrid};

/// Rule-7 BaddieNearby capability mask (`FUN_00416560`).
const BADDIE_NEARBY_MASK: u32 = 0x08;
/// Rule-6 PlayerNearby capability mask (`FUN_00416550`).
const PLAYER_NEARBY_MASK: u32 = 0x01;
/// Rule-12 BaseNearby capability mask (`FUN_004165B0`).
const BASE_NEARBY_MASK: u32 = 0x20;

/// The shared process constructor supplies an actual successful Sub-D allocation
/// and `104B0`'s surface comparison before `D4A0` grounds the authored person.
pub(crate) struct Type86AuthoredConstructionRequest<'a> {
    pub entity: &'a mut Entity,
    pub allocation: crate::main_base_abort::MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub spawn: &'a EntitySpawn,
    pub preceding: &'a [Entity],
    pub terrain: &'a TerrainGrid,
    pub sub_d: NativeSubDConstruction,
    pub constructor_surface_bits: u32,
}

pub(crate) fn publish_authored_type86(
    request: Type86AuthoredConstructionRequest<'_>,
    world_fx: &mut WorldFx,
) -> Result<Option<AttractAttentionResourceTextRequest>, Type86Block> {
    let Type86AuthoredConstructionRequest {
        entity,
        allocation,
        metadata,
        spawn,
        preceding,
        terrain,
        sub_d,
        constructor_surface_bits,
    } = request;
    let profile = NativeFourChoiceProfile::from_entity_type(entity.entity_type)
        .ok_or(Type86Block::Runtime("person profile"))?;
    validate_metadata(profile, metadata)?;
    if !entity.active
        || entity.id != allocation.entity_id
        || spawn.entity_type != profile.entity_type()
        || entity.authored_spawn_index != Some(spawn.index)
        || entity.model_slots != [Some(profile.model_id()); 4]
        || entity.capability_flags != profile.capability()
        || entity.rotation_heading_pitch_roll_raw() != spawn.rotation.map(|word| word as i16)
        || entity.native_type86_runtime.is_some()
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
        return Err(Type86Block::Runtime("authored Type86 birth"));
    }
    if constructor_surface_bits & !SURFACE_STATE_MASK != 0 {
        return Err(Type86Block::Runtime("constructor surface bits"));
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
        return Err(Type86Block::Runtime(
            "unconsumed native Type86 Sub-A constructor",
        ));
    }
    let [x, _, z] = spawn.position_raw();
    let cell = terrain
        .cell(usize::from((x as u16) >> 8), usize::from((z as u16) >> 8))
        .ok_or(Type86Block::Runtime("authored terrain cell"))?;
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
    entity.model_index = Some(profile.model_id());
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(
        metadata
            .initializer
            .as_ref()
            .unwrap()
            .common_axis_descriptor,
    );
    entity.native_type86_anchor_raw_at_0x90 = RetailRuntimeValue::Known(anchor);
    entity.type8_sub_d_frame_owner = Some(sub_d.frame_owner);
    entity.type8_sub_d_runtime = Some(sub_d.runtime);
    // 20450 consumes the first constructor word, before 425680 evaluates its
    // weighted Player/Baddie/Base/Always predicates and the selected branch
    // draws its constructor words.
    let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_a_propulsion_descriptor else {
        unreachable!();
    };
    entity.sub_a_propulsion_runtime =
        RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_20450_constructor(
            descriptor,
            u32::from(world_fx.next_shared_retail_random_u16()) as u16,
        )));
    let preceding_refs = preceding.iter().collect::<Vec<_>>();
    let selection = apply_root_with_preceding(entity, metadata, &preceding_refs, world_fx)?;
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection.selection));
    let receipt = selection.resource_text;
    retain_native_type86_runtime(entity, allocation, metadata, sub_d)?;
    Ok(receipt)
}

fn validate_native_components(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    sub_d: NativeSubDConstruction,
) -> Result<(), Type86Block> {
    if sub_d.descriptor
        != NativeFourChoiceProfile::from_entity_type(entity.entity_type)
            .ok_or(Type86Block::Runtime("four-choice profile"))?
            .sub_d()
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
        return Err(Type86Block::Runtime("native Type86 components"));
    }
    Ok(())
}

pub(crate) fn retain_native_type86_runtime(
    entity: &mut Entity,
    allocation: crate::main_base_abort::MainBaseAbortActorLease,
    metadata: &EntityTypeRuntimeMetadata,
    sub_d: NativeSubDConstruction,
) -> Result<(), Type86Block> {
    let profile = NativeFourChoiceProfile::from_entity_type(entity.entity_type)
        .ok_or(Type86Block::Runtime("person profile"))?;
    validate_metadata(profile, metadata)?;
    let (task_id, context, kind) = Type86Owner::published_graph(entity)
        .ok_or(Type86Block::Runtime("completed native Type86 graph"))?;
    if !entity.active
        || entity.id != allocation.entity_id
        || entity.model_slots != [Some(profile.model_id()); 4]
        || entity.capability_flags != profile.capability()
        || entity.native_type86_runtime.is_some()
        || kind == TaskKind::Exploding
        || entity.type8_sub_d_frame_owner != Some(sub_d.frame_owner)
        || entity.type8_sub_d_runtime != Some(sub_d.runtime)
        || entity.native_type86_anchor_raw_at_0x90
            != RetailRuntimeValue::Known(entity.position_raw())
    {
        return Err(Type86Block::Runtime("completed native Type86 allocation"));
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
    entity.native_type86_runtime = Some(NativeType86Runtime {
        profile,
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

pub(super) fn validate_metadata(
    profile: NativeFourChoiceProfile,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type86Block> {
    let Some(init) = metadata.initializer.as_ref() else {
        return Err(Type86Block::Runtime("initializer"));
    };
    let animation_cues = match metadata.actor_animation_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => Some((
            descriptor.capability_bit_3_sound_id,
            descriptor.capability_mask_0x201_sound_id,
            descriptor.attention_stop_sound_id,
            descriptor.variable_binding,
            descriptor.frames_per_direction,
        )),
        _ => None,
    };
    let expected_axis = profile.axis();
    if metadata.model_slots != [profile.model_id() as u16; 4]
        || animation_cues != Some(profile.animation_cues())
        || metadata.capability_flags != profile.capability()
        || metadata.mass_raw != 10
        || metadata.initial_health_raw != Some(HEALTH)
        || init.initializer_state_flags_raw != 0x2f
        || init.behavior_rule_ref != 1
        || init.alternate_behavior_class_ref != 14
        || init.common_axis_descriptor.strict_axis_limit_raw != expected_axis.0
        || init.common_axis_descriptor.raw_word_at_0x04 != expected_axis.1
        || init
            .behavior_choices
            .iter()
            .map(|c| (c.weight_rule_id, c.weight_multiplier, c.behavior_class_id))
            .collect::<Vec<_>>()
            != profile.choices()
        || metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
        || metadata.death_sound_id != RetailRuntimeValue::Known(Some(profile.death_sound()))
        || metadata.accepted_hit_presentation_sound_id
            != RetailRuntimeValue::Known(profile.accepted_hit_sound())
        || metadata.run_away_optional_sound_id != RetailRuntimeValue::Known(None)
        || metadata.run_away_sound_period_raw != RetailRuntimeValue::Known(0)
        || !matches!(metadata.sub_a_propulsion_descriptor, RetailRuntimeValue::Known(Some(a))
            if a.acceleration_raw == 1500
                && a.overspeed_correction_raw == -3000
                && a.target_speed_base_raw == 250)
        || !matches!(metadata.sub_b_lateral_descriptor, RetailRuntimeValue::Known(Some(b))
            if b.projection_threshold_rate_raw == 10000 && b.correction_rate_raw == 1000)
        || !matches!(metadata.common_mover_topology, RetailRuntimeValue::Known(t)
            if t.sub_a && t.sub_b && t.sub_d && t.sub_i
                && !t.sub_c && !t.sub_e && !t.sub_f && !t.sub_g && !t.sub_h
                && !t.sub_j && !t.sub_k && !t.sub_l && !t.sub_m && !t.sub_n && !t.sub_o)
        || !matches!(metadata.common_world_effects, RetailRuntimeValue::Known(effects)
            if effects.surface_selectors == profile.surface_selectors()
                && effects.surface_lifetime_ms == profile.surface_lifetime_ms()
                && effects.low_health_effect_words == profile.effect_words())
        || metadata.damage_profile != Some(profile.damage_profile())
        || ActorAbdiTopology::from_metadata(profile.entity_type() as u16, metadata).is_err()
    {
        return Err(Type86Block::Runtime("canonical person metadata"));
    }
    Ok(())
}

pub(crate) struct RootApplication {
    pub selection: BehaviorSelection,
    pub resource_text: Option<AttractAttentionResourceTextRequest>,
    pub selector_rng_word: u16,
    pub constructor_rng_words: Vec<u16>,
}

/// Read-only 425680 predicates and AF90 target plan. Dynamic callers close
/// this before committing 104B0's body attempt or conversion notifications.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct FourChoiceRootPlan {
    profile: NativeFourChoiceProfile,
    player_nearby: bool,
    baddie_nearby: bool,
    destination_nearby: bool,
    go_to_job: Option<crate::go_to_job::GoToJobSetupPlan>,
}

struct BirthEffects<'a> {
    world_fx: &'a mut WorldFx,
    constructor_rng_words: Vec<u16>,
}
impl BirthEffects<'_> {
    fn next_word(&mut self) -> u16 {
        let word = self.world_fx.next_shared_retail_random_u16();
        self.constructor_rng_words.push(word);
        word
    }
}

/// Evaluate the Player/Baddie/Base nearby predicates against already
/// snapshotted candidates in exact intrusive order, through the owner's own
/// authored axis range. Snapshots are owned so selection can precede the
/// exclusive entity borrow.
fn nearby_weights(
    owner: GuardLocationEntityRef,
    candidates: &[GuardLocationEntityRef],
    range: WrappedAxisRange,
    profile: NativeFourChoiceProfile,
) -> Result<(bool, bool, bool), Type86Block> {
    let nearby = |mask: u32| {
        let filter = GuardLocationCandidateFilter::CapabilityMask(
            std::num::NonZeroU32::new(mask).expect("nearby masks are nonzero"),
        );
        select_guard_location_candidate(GuardLocationCandidateRequest {
            owner,
            candidates_in_intrusive_order: candidates,
            search_context: GuardLocationSearchContext::new(range, filter),
        })
        .map(|selection| {
            matches!(
                selection,
                crate::guard_location_owner::acquisition::GuardLocationCandidateSelection::Selected(
                    _
                )
            )
        })
        .map_err(|_| Type86Block::Runtime("nearby evidence"))
    };
    Ok((
        nearby(PLAYER_NEARBY_MASK)?,
        nearby(BADDIE_NEARBY_MASK)?,
        match profile {
            NativeFourChoiceProfile::Person(_) => nearby(BASE_NEARBY_MASK)?,
            NativeFourChoiceProfile::DiverWorker => false,
        },
    ))
}

fn owner_nearby_ref(entity: &Entity) -> GuardLocationEntityRef {
    GuardLocationEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
    }
}

fn job_candidate(entity: &Entity) -> GoToJobCandidate {
    GoToJobCandidate {
        id: entity.id,
        position_raw: entity.position_raw(),
        state_flags: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        capacity: crate::ordinary_type9_wander_production::job_capacity_from_entity(entity),
    }
}

fn apply_wander(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), Type86Block> {
    // AD10 installs only the Primary task. Retire any stale Secondary/
    // Tertiary tasks from a previous multi-slot graph first.
    let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
        return Err(Type86Block::Runtime("Sub-I"));
    };
    for slot in [ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary] {
        entity
            .actor_tasks
            .clear_slot_with_retirement(slot, |task| task.retire_animation(animation));
    }
    let position = entity.position_raw();
    let RetailRuntimeValue::Known(Some(sub_a)) = &mut entity.sub_a_propulsion_runtime else {
        return Err(Type86Block::Runtime("Sub-A"));
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

fn apply_go_to_job(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    plan: crate::go_to_job::GoToJobSetupPlan,
    effects: &mut BirthEffects<'_>,
) -> Result<(), Type86Block> {
    use crate::go_to_job_owner::GoToJobTaskState;
    // AD10 installs only the Primary task. Retire stale auxiliaries first,
    // mirroring the Wander arm.
    let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
        return Err(Type86Block::Runtime("Sub-I"));
    };
    for slot in [ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary] {
        entity
            .actor_tasks
            .clear_slot_with_retirement(slot, |task| task.retire_animation(animation));
    }
    let owner = GoToJobOwner::from_type_metadata(
        entity.id,
        entity.position_raw(),
        RetailRuntimeValue::Known(entity.capability_flags),
        entity.entity_type as u16,
        metadata,
    );
    let mut next_random = || u32::from(effects.next_word());
    plan.apply(
        owner
            .bind_entity_runtime(entity)
            .map_err(|_| Type86Block::Runtime("Sub-A"))?,
        &mut next_random,
        |spec| {
            Ok::<_, Infallible>(PreparedActorTask::new(ActorTaskRuntime::GoToJob(
                GoToJobTaskState::after_allocation(spec),
            )))
        },
    )
    .expect("infallible native task allocation");
    Ok(())
}

fn apply_run_away_acquiring(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    effects: &mut BirthEffects<'_>,
) -> Result<(), Type86Block> {
    // B6C0 clears Tertiary, then installs the acquisition Secondary (phase0)
    // and the 500ms wander Primary (phase1), each followed by its 06070
    // Sub-A-only suffix word. Failed allocation preserves the phase prefix
    // for the outer fallback; the caller owns that policy.
    let position_raw = entity.position_raw();
    let RetailRuntimeValue::Known(Some(sub_a)) = &mut entity.sub_a_propulsion_runtime else {
        return Err(Type86Block::Runtime("Sub-A"));
    };
    let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
        return Err(Type86Block::Runtime("Sub-I"));
    };
    let mut next_word = || u32::from(effects.next_word());
    apply_run_away_task_setup_with_retirement(
        &mut entity.actor_tasks,
        RunAwayTaskSetupRequest::Acquiring,
        |preparation| -> Result<PreparedActorTask<ActorTaskRuntime>, ()> {
            let prepared =
                prepare_shared_acquiring_runtime_task(preparation, position_raw, metadata, 0)
                    .map_err(|_| ())?;
            debug_assert_eq!(
                prepared.topology(),
                crate::actor_task_dispatcher::SharedGenericConstructorTopology::SubAOnly
            );
            Ok(prepared.apply_suffix(&mut next_word, |effect| {
                apply_constructor_effect(effect, sub_a)
            }))
        },
        |task| task.retire_animation(animation),
    )
    .map_err(|_| Type86Block::Runtime("run-away setup"))?;
    Ok(())
}

fn apply_constructor_effect(
    effect: SharedGenericConstructorEffect,
    sub_a: &mut SubAPropulsionRuntime,
) {
    match effect {
        SharedGenericConstructorEffect::WriteSubHState08 { .. } => {
            unreachable!("Type86 topology has no Sub-H branch")
        }
        SharedGenericConstructorEffect::WriteSubADirection {
            direction_multiplier,
        } => sub_a.set_direction_multiplier(direction_multiplier),
        SharedGenericConstructorEffect::WriteSubATargetSpeed {
            target_speed_raw, ..
        } => sub_a.apply_shared_initializer_target_speed_write(target_speed_raw),
    }
}

struct BirthAdapter<'a, 'fx> {
    position_raw: [i16; 3],
    resource_text: Option<AttractAttentionResourceTextRequest>,
    effects: &'a mut BirthEffects<'fx>,
}

impl AttractAttentionInitialSetupAdapter<ActorTaskRuntime> for BirthAdapter<'_, '_> {
    type PrepareError = Infallible;

    fn retire_task(
        &mut self,
        task: &ActorTaskRuntime,
        animation: &mut crate::actor_animation::ActorAnimationController,
    ) {
        task.retire_animation(animation);
    }

    fn next_shared_random_u16(&mut self) -> u16 {
        self.effects.next_word()
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
        self.effects
            .world_fx
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
    effects: &mut BirthEffects<'_>,
) -> Result<Option<AttractAttentionResourceTextRequest>, Type86Block> {
    use crate::attract_attention::AttractAttentionInitialExecutionRequest;
    let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_a_propulsion_descriptor else {
        unreachable!();
    };
    let position_raw = entity.position_raw();
    let RetailRuntimeValue::Known(Some(sub_a)) = &mut entity.sub_a_propulsion_runtime else {
        return Err(Type86Block::Runtime("Sub-A"));
    };
    let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
        return Err(Type86Block::Runtime("Sub-I"));
    };
    // The BA40 adapter draws parity + suffix words from the shared stream in
    // retail order; the outer selector word was already consumed above.
    // Sounds queue to WorldFx (no phase gate); the resource-text event is
    // retained for the canonical load drain, which admits ordinary native
    // Section-13 construction as gameplay phase 5.
    let mut adapter = BirthAdapter {
        position_raw,
        resource_text: None,
        effects,
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
    .map_err(|_| Type86Block::Runtime("attract initializer"))?;
    Ok(adapter.resource_text)
}

pub(super) fn reselect(
    manager: &mut EntityManager,
    id: u32,
    world_fx: &mut WorldFx,
) -> Result<Type86Owner, Type86Block> {
    if !native_type86_manager_allocation_authenticates(manager, id) {
        return Err(Type86Block::Runtime("person reselection allocation"));
    }
    let entity_type = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .entity_type;
    let metadata = manager
        .type_runtime_metadata(entity_type)
        .cloned()
        .ok_or(Type86Block::Runtime("metadata"))?;
    // Snapshot owned candidate projections before the exclusive borrow.
    // Reselection evaluates nearby predicates against the live list, like
    // the birth prefix. The entity itself is skipped by the 22C10 walk.
    let entities = manager
        .retail_live_order_ids()
        .filter_map(|id| manager.iter_all().find(|e| e.id == id))
        .map(|e| (owner_nearby_ref(e), job_candidate(e)))
        .collect::<Vec<_>>();
    let (nearby_candidates, job_candidates): (Vec<_>, Vec<_>) = entities.into_iter().unzip();
    let entity = manager
        .entity_mut(id)
        .ok_or(Type86Block::Runtime("entity"))?;
    let owner_ref = owner_nearby_ref(entity);
    let application = apply_root_with_snapshots(
        entity,
        &metadata,
        owner_ref,
        &nearby_candidates,
        &job_candidates,
        world_fx,
    )?;
    let owner =
        Type86Owner::adopt_published(entity).ok_or(Type86Block::Runtime("root publication"))?;
    if let Some(receipt) = application.resource_text {
        manager.enqueue_native_attract_attention_receipt(receipt);
    }
    Ok(owner)
}

/// Close CE90's fallible world reads before the caller removes its Sub-J row.
/// The projected owner retains its real identity and +60 relation; only the
/// release-owned position/state writes are substituted. Discard the plan so
/// CE90 still selects against the current intrusive list after release.
pub(super) fn preflight_release_root(
    manager: &EntityManager,
    id: u32,
    position_raw: [i16; 3],
    state_flags_raw: RetailStateWord,
) -> Result<(), Type86Block> {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Type86Block::Runtime("release root entity"))?;
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .ok_or(Type86Block::Runtime("release root metadata"))?;
    let mut owner = owner_nearby_ref(entity);
    owner.position_raw = position_raw;
    owner.state_flags_raw = state_flags_raw;
    let (nearby, jobs): (Vec<_>, Vec<_>) = manager
        .retail_live_order_ids()
        .filter_map(|id| manager.iter_all().find(|entity| entity.id == id))
        .map(|entity| (owner_nearby_ref(entity), job_candidate(entity)))
        .unzip();
    prepare_root_plan(metadata, owner, &nearby, &jobs).map(|_| ())
}

fn apply_root_with_preceding(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    preceding: &[&Entity],
    world_fx: &mut WorldFx,
) -> Result<RootApplication, Type86Block> {
    // Snapshot owned candidate projections before any mutation: selection
    // and the class54 nearest scan both read the authored prefix.
    let owner_ref = owner_nearby_ref(entity);
    let nearby_candidates = preceding
        .iter()
        .map(|candidate| owner_nearby_ref(candidate))
        .collect::<Vec<_>>();
    let job_candidates = preceding
        .iter()
        .map(|candidate| job_candidate(candidate))
        .collect::<Vec<_>>();
    apply_root_with_snapshots(
        entity,
        metadata,
        owner_ref,
        &nearby_candidates,
        &job_candidates,
        world_fx,
    )
}

fn apply_root_with_snapshots(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    owner_ref: GuardLocationEntityRef,
    nearby_candidates: &[GuardLocationEntityRef],
    job_candidates: &[GoToJobCandidate],
    world_fx: &mut WorldFx,
) -> Result<RootApplication, Type86Block> {
    let plan = prepare_root_plan(metadata, owner_ref, nearby_candidates, job_candidates)?;
    apply_prepared_root(entity, metadata, plan, world_fx)
}

pub(crate) fn prepare_unpublished_root(
    id: u32,
    entity_type: u32,
    position_raw: [i16; 3],
    metadata: &EntityTypeRuntimeMetadata,
    preceding: &[&Entity],
) -> Result<FourChoiceRootPlan, Type86Block> {
    let owner = GuardLocationEntityRef {
        id,
        entity_type,
        position_raw,
        state_flags_raw: RetailStateWord::exact(0),
        capability_flags: RetailRuntimeValue::Known(metadata.capability_flags),
        attached_entity_handle: RetailRuntimeValue::Known(None),
    };
    let nearby = preceding
        .iter()
        .map(|e| owner_nearby_ref(e))
        .collect::<Vec<_>>();
    let jobs = preceding
        .iter()
        .map(|e| job_candidate(e))
        .collect::<Vec<_>>();
    prepare_root_plan(metadata, owner, &nearby, &jobs)
}

fn prepare_root_plan(
    metadata: &EntityTypeRuntimeMetadata,
    owner: GuardLocationEntityRef,
    nearby_candidates: &[GuardLocationEntityRef],
    job_candidates: &[GoToJobCandidate],
) -> Result<FourChoiceRootPlan, Type86Block> {
    use crate::job_nearby::{
        evaluate_job_nearby, JobNearbyCandidate, JobNearbyEvaluationRequest, JobNearbyOwner,
    };
    let profile = NativeFourChoiceProfile::from_entity_type(owner.entity_type)
        .ok_or(Type86Block::Runtime("four-choice profile"))?;
    validate_metadata(profile, metadata)?;
    let range = WrappedAxisRange::from_raw(profile.axis().0);
    let (player_nearby, baddie_nearby, base_nearby) =
        nearby_weights(owner, nearby_candidates, range, profile)?;
    let destination_nearby = match profile {
        NativeFourChoiceProfile::Person(_) => base_nearby,
        NativeFourChoiceProfile::DiverWorker => {
            let candidates = job_candidates
                .iter()
                .map(|c| JobNearbyCandidate {
                    id: c.id,
                    position_raw: c.position_raw,
                    state_flags: c.state_flags,
                    capacity: c.capacity,
                })
                .collect::<Vec<_>>();
            evaluate_job_nearby(JobNearbyEvaluationRequest {
                owner: JobNearbyOwner {
                    id: owner.id,
                    position_raw: owner.position_raw,
                    capability_flags: owner.capability_flags,
                },
                candidates_in_intrusive_order: &candidates,
                range,
            })
            .map_err(|_| Type86Block::Runtime("Job Nearby evidence"))?
        }
    };
    let go_to_job = destination_nearby
        .then(|| {
            plan_go_to_job_setup(GoToJobSetupRequest {
                owner: GoToJobOwner::from_type_metadata(
                    owner.id,
                    owner.position_raw,
                    owner.capability_flags,
                    owner.entity_type as u16,
                    metadata,
                ),
                candidates_in_intrusive_order: job_candidates,
                range,
            })
        })
        .transpose()
        .map_err(|_| Type86Block::Runtime("Go-To-Job evidence"))?;
    Ok(FourChoiceRootPlan {
        profile,
        player_nearby,
        baddie_nearby,
        destination_nearby,
        go_to_job,
    })
}

pub(crate) fn apply_prepared_root(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    plan: FourChoiceRootPlan,
    world_fx: &mut WorldFx,
) -> Result<RootApplication, Type86Block> {
    if plan.profile.entity_type() != entity.entity_type {
        return Err(Type86Block::Runtime("root profile"));
    }
    validate_metadata(plan.profile, metadata)?;
    let mut selector_rng_word = None;
    let selection = select_initial_behavior(
        &metadata.initializer.as_ref().unwrap().behavior_choices,
        |rule| match rule {
            BehaviorWeightRule::PlayerNearby => i32::from(plan.player_nearby),
            BehaviorWeightRule::BaddieNearby => i32::from(plan.baddie_nearby),
            BehaviorWeightRule::BaseNearby | BehaviorWeightRule::JobNearby => {
                i32::from(plan.destination_nearby)
            }
            BehaviorWeightRule::Always => 1,
            _ => unreachable!("exact four-choice rules"),
        },
        || {
            let word = world_fx.next_shared_retail_random_u16();
            assert!(selector_rng_word.replace(word).is_none());
            u32::from(word)
        },
    )
    .map_err(|_| Type86Block::Runtime("root selection"))?
    .ok_or(Type86Block::Runtime("empty root"))?;
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
    .ok_or(Type86Block::Runtime("root context"))?;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    let mut effects = BirthEffects {
        world_fx,
        constructor_rng_words: Vec::new(),
    };
    let resource_text = match selection.program.class_id {
        45 => apply_attract(entity, metadata, &mut effects)?,
        10 => {
            apply_run_away_acquiring(entity, metadata, &mut effects)?;
            None
        }
        54 => {
            apply_go_to_job(
                entity,
                metadata,
                plan.go_to_job.expect("selected class54 was preflighted"),
                &mut effects,
            )?;
            None
        }
        6 => {
            apply_wander(entity, metadata, &mut || u32::from(effects.next_word()))?;
            None
        }
        _ => unreachable!("exact four-choice classes"),
    };
    Ok(RootApplication {
        selection,
        resource_text,
        selector_rng_word: selector_rng_word.expect("425680 draws once"),
        constructor_rng_words: effects.constructor_rng_words,
    })
}
