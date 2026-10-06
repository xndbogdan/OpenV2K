//! Ordinary Type-9 relation callbacks, distinct from Type-68 release.
//!
//! `16700 -> DBF0 -> CD50/CE70 -> CD70` switches the existing behavior
//! context to its carrying style. `ADB0` clears slots 2/1 and constructs a
//! real None Primary (`3230`, tick `3250`). The paired `16750 -> DC50 -> CE90`
//! resets Sub-I and invokes AC60 from context +0, the retained type-default
//! choice list. It does not resume the task graph destroyed at collection.

use crate::{
    actor_animation::ActorAnimationController,
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{
        ActorTaskOwner, ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags, PreparedActorTask,
    },
    entity::Entity,
    entity_behavior::{
        audited_behavior_style, ActiveBehaviorStyle, BehaviorChoiceListSource,
        BehaviorContextRuntime, BehaviorDescriptorIdentity,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    main_base_abort::MainBaseAbortActorLease,
    main_base_type9_abort::exact_level_one_type9_metadata,
    ordinary_type9_live::{
        OrdinaryType9SelectedComponentRuntime, OrdinaryType9SelectedRuntimeKind,
    },
};

use crate::{
    attract_attention::{
        AttractAttentionPositionalSoundRequest, AttractAttentionResourceTextRequest,
    },
    entity_relation_release::relation_release_state_word_after,
    ordinary_type9_attract_attention_initializer::OrdinaryType9AttractAttentionAllocationDecision,
    ordinary_type9_current_task::OrdinaryType9CurrentTaskPublication,
    ordinary_type9_go_to_job_initializer::{
        OrdinaryType9GoToJobAllocationDecision, OrdinaryType9GoToJobCandidateEvidence,
    },
    ordinary_type9_root_attract_attention_application::{
        apply_ordinary_type9_root_attract_attention_from_predecessor,
        OrdinaryType9RootAttractAttentionApplicationOutcome,
        OrdinaryType9RootAttractAttentionPredecessorKind,
    },
    ordinary_type9_root_go_to_job_application::{
        apply_ordinary_type9_root_go_to_job_from_predecessor,
        OrdinaryType9RootGoToJobApplicationOutcome, OrdinaryType9RootGoToJobPredecessorKind,
    },
    ordinary_type9_root_reselection::{
        plan_ordinary_type9_root_reselection, OrdinaryType9RootEntityRef,
        OrdinaryType9RootReselectionPlan, OrdinaryType9RootReselectionRequest,
        OrdinaryType9RootSelection,
    },
    ordinary_type9_root_run_away_application::{
        apply_ordinary_type9_root_run_away_from_predecessor,
        OrdinaryType9RootRunAwayApplicationOutcome, OrdinaryType9RootRunAwayPredecessorKind,
    },
    ordinary_type9_root_wander_application::{
        apply_ordinary_type9_root_wander, OrdinaryType9RootWanderApplicationOutcome,
    },
    ordinary_type9_run_away_initializer::OrdinaryType9RunAwayAllocationDecision,
    ordinary_type9_wander_initializer::OrdinaryType9WanderAllocationDecision,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Type9CargoBlock {
    UnsupportedEntity,
    MetadataUnavailable,
    ContextUnavailable,
    UnsupportedStyle,
    ComponentUnavailable,
    TaskGraphUnavailable,
    StateChanged,
    CandidateEvidenceUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Type9CargoReleasePosition {
    /// `443D30` releases the old player relation before attaching the new
    /// proxy. Neither position nor the old wander anchor changes here.
    Retained,
    /// `409030` copies the proxy position to both entity +96 and +90.
    Materialiser([i16; 3]),
}

pub(crate) struct Type9CargoReleaseRequest<'a> {
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub candidates: &'a [OrdinaryType9RootEntityRef],
    pub job_evidence: &'a [OrdinaryType9GoToJobCandidateEvidence],
    pub position: Type9CargoReleasePosition,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Type9CargoReleasePrepared {
    before_position: [i16; 3],
    before_state: crate::entity_collision_state::RetailStateWord,
    before_axis: RetailRuntimeValue<v2k_formats::collision::CommonAxisDescriptor>,
    before_sub_a: RetailRuntimeValue<Option<crate::common_mover::SubAPropulsionRuntime>>,
    before_animation: RetailRuntimeValue<Option<ActorAnimationController>>,
    position: Type9CargoReleasePosition,
    after_owner: OrdinaryType9RootEntityRef,
    candidates: Vec<OrdinaryType9RootEntityRef>,
    job_evidence: Vec<OrdinaryType9GoToJobCandidateEvidence>,
}

/// Validate the release at the position its caller actually publishes. This
/// preflight evaluates the nearby queries without advancing process RNG; the
/// actual selector draw remains after the ordered 16750/CE90 writes.
pub(crate) fn prepare_release(
    entity: &Entity,
    carried: &Type9CarriedOwner,
    request: &Type9CargoReleaseRequest<'_>,
) -> Result<Type9CargoReleasePrepared, Type9CargoBlock> {
    if !carried.authenticates(entity) {
        return Err(Type9CargoBlock::StateChanged);
    }
    if !exact_level_one_type9_metadata(request.metadata) {
        return Err(Type9CargoBlock::MetadataUnavailable);
    }
    if entity.model_index != Some(558)
        || entity.model_slots != [Some(558); 4]
        || !matches!(
            entity.collision.active_model_slot(),
            RetailRuntimeValue::Known(0 | 2)
        )
        || entity.ordinary_type9_pending_initial_selection.is_some()
        || entity.main_base_type9_death_component_runtime.is_some()
    {
        return Err(Type9CargoBlock::UnsupportedEntity);
    }
    let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
        return Err(Type9CargoBlock::ComponentUnavailable);
    };
    if !matches!(
        entity.sub_a_propulsion_runtime,
        RetailRuntimeValue::Known(Some(sub_a)) if matches!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(_))
    ) || !matches!(entity.actor_animation_runtime, RetailRuntimeValue::Known(Some(animation)) if !animation.special_mode())
        || matches!(
            carried.context.target_handle_at_0x08(),
            RetailRuntimeValue::Unresolved
        )
        || matches!(
            carried.context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Unresolved
        )
    {
        return Err(Type9CargoBlock::ComponentUnavailable);
    }
    let before_position = position_raw(entity);
    let mut state = entity.collision.state_flags_at_0x08;
    let position = match request.position {
        Type9CargoReleasePosition::Retained => before_position,
        Type9CargoReleasePosition::Materialiser(raw) => {
            state.overwrite(0x40000, 0x40000);
            raw
        }
    };
    state = relation_release_state_word_after(state, 0x2f);
    state.overwrite(0x8000, 0x8000); // CE90 before AC60
    let after_owner = OrdinaryType9RootEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: position,
        state_flags_raw: state,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
    };
    let mut candidates = request.candidates.to_vec();
    let mut job_evidence = request.job_evidence.to_vec();
    for candidate in &mut candidates {
        if candidate.id == entity.id {
            *candidate = after_owner;
        }
    }
    for evidence in &mut job_evidence {
        if evidence.candidate_id == entity.id {
            evidence.state_flags = state;
        }
    }
    let probe = plan_ordinary_type9_root_reselection(
        OrdinaryType9RootReselectionRequest {
            active_model_id: 558,
            metadata: request.metadata,
            owner: after_owner,
            current_context: entity.current_behavior_context,
            candidates_in_intrusive_order: &candidates,
        },
        || 0,
    )
    .map_err(|_| Type9CargoBlock::CandidateEvidenceUnavailable)?;
    let OrdinaryType9RootSelection::Weighted {
        evaluator_evidence, ..
    } = probe.selection()
    else {
        return Err(Type9CargoBlock::UnsupportedEntity);
    };
    // The separate class-54 scan may be selected by the later real RNG word.
    // Resolve its fallible capacity/target evidence now as well.
    let job_owner = crate::go_to_job::GoToJobOwner::from_type_metadata(
        entity.id,
        position,
        RetailRuntimeValue::Known(entity.capability_flags),
        9,
        request.metadata,
    );
    if evaluator_evidence.base_nearby.weight() != 0 {
        crate::ordinary_type9_go_to_job_initializer::plan_ordinary_type9_go_to_job_task_transaction(
        job_owner,
        &candidates,
        &job_evidence,
        crate::wrapped_axis_range::WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
    )
        .map_err(|_| Type9CargoBlock::CandidateEvidenceUnavailable)?;
    }
    Ok(Type9CargoReleasePrepared {
        before_position,
        before_state: entity.collision.state_flags_at_0x08,
        before_axis: entity.actor_common_axis_descriptor,
        before_sub_a: entity.sub_a_propulsion_runtime,
        before_animation: entity.actor_animation_runtime,
        position: request.position,
        after_owner,
        candidates,
        job_evidence,
    })
}

/// Commit the complete fixed release prefix and the carrying-style callback.
/// The exact prepared snapshot makes the second selector on a beam/drop cycle
/// observe the settled position, while the first still observes the old one.
pub(crate) fn commit_release(
    entity: &mut Entity,
    carried: Type9CarriedOwner,
    prepared: Type9CargoReleasePrepared,
    request: Type9CargoReleaseRequest<'_>,
    mut next_rng: impl FnMut() -> u32,
    emit_resource_text: impl FnMut(AttractAttentionResourceTextRequest),
    emit_positional_sound: impl FnMut(AttractAttentionPositionalSoundRequest),
) -> Result<OrdinaryType9CurrentTaskPublication, Type9CarriedOwner> {
    // Revalidate the complete immutable request before the first prefix write.
    // This also rejects changed component words, model admission, or candidate
    // evidence without consuming process RNG or surrendering carrying custody.
    if prepare_release(entity, &carried, &request).as_ref() != Ok(&prepared) {
        return Err(carried);
    }
    if let Type9CargoReleasePosition::Materialiser(raw) = prepared.position {
        entity.position = raw.map(|word| f32::from(word) / 256.0);
        entity
            .ordinary_type9_selected_component_runtime
            .as_mut()
            .unwrap()
            .apply_materialiser_release_anchor(raw);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x40000, 0x40000);
    }
    entity.collision.state_flags_at_0x08 =
        relation_release_state_word_after(entity.collision.state_flags_at_0x08, 0x2f);
    entity.collision.default_state_flags_at_0xc8 = RetailRuntimeValue::Known(0x2f);
    entity.attached_to = None;
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x8000, 0x8000);
    let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
        unreachable!("prepared carrying graph retains Sub-I")
    };
    animation.apply_relation_release();
    let plan = plan_ordinary_type9_root_reselection(
        OrdinaryType9RootReselectionRequest {
            active_model_id: 558,
            metadata: request.metadata,
            owner: prepared.after_owner,
            current_context: entity.current_behavior_context,
            candidates_in_intrusive_order: &prepared.candidates,
        },
        &mut next_rng,
    )
    .expect("immutable release preflight resolved every selector read");
    Ok(apply_release_selection(
        entity,
        request.metadata,
        plan,
        &prepared.job_evidence,
        next_rng,
        emit_resource_text,
        emit_positional_sound,
    ))
}

fn apply_release_selection(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    plan: OrdinaryType9RootReselectionPlan,
    job_evidence: &[OrdinaryType9GoToJobCandidateEvidence],
    mut next_rng: impl FnMut() -> u32,
    emit_resource_text: impl FnMut(AttractAttentionResourceTextRequest),
    emit_positional_sound: impl FnMut(AttractAttentionPositionalSoundRequest),
) -> OrdinaryType9CurrentTaskPublication {
    let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
        unreachable!("release preflight retains the common axis")
    };
    let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
        unreachable!("release preflight retains Sub-A")
    };
    let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
        unreachable!("release preflight retains Sub-I")
    };
    let OrdinaryType9RootSelection::Weighted { selection, .. } = plan.selection() else {
        unreachable!("release preflight rejects the unsupported dying branch")
    };
    let task_visits = visits(entity);
    match selection.program.class_id {
        6 => apply_ordinary_type9_root_wander(
            entity,
            metadata,
            plan,
            task_visits,
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            &mut next_rng,
        )
        .map(|outcome| match outcome {
            OrdinaryType9RootWanderApplicationOutcome::Published { owner, .. } => {
                OrdinaryType9CurrentTaskPublication::Wander(owner)
            }
            _ => unreachable!("infallible allocation publishes Wander"),
        })
        .expect("preflighted carrying graph publishes the canonical Wander constructor"),
        10 => apply_ordinary_type9_root_run_away_from_predecessor(
            entity,
            metadata,
            plan,
            task_visits,
            sub_a,
            OrdinaryType9RootRunAwayPredecessorKind::CargoRelease {
                actor_common_axis: axis,
            },
            |_| OrdinaryType9RunAwayAllocationDecision::Prepared,
            &mut next_rng,
        )
        .map(|outcome| match outcome {
            OrdinaryType9RootRunAwayApplicationOutcome::Published { owner, .. } => {
                OrdinaryType9CurrentTaskPublication::RunAway(owner)
            }
            _ => unreachable!("infallible allocation publishes Run Away"),
        })
        .expect("preflighted carrying graph publishes the canonical Run Away constructor"),
        54 => apply_ordinary_type9_root_go_to_job_from_predecessor(
            entity,
            metadata,
            plan,
            task_visits,
            axis,
            sub_a,
            job_evidence,
            OrdinaryType9RootGoToJobPredecessorKind::CargoRelease,
            |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
            &mut next_rng,
        )
        .map(|outcome| match outcome {
            OrdinaryType9RootGoToJobApplicationOutcome::Published { owner, .. } => {
                OrdinaryType9CurrentTaskPublication::GoToJob(owner)
            }
            _ => unreachable!("infallible allocation publishes Go To Job"),
        })
        .expect("preflighted carrying graph publishes the canonical Go To Job constructor"),
        45 => apply_ordinary_type9_root_attract_attention_from_predecessor(
            entity,
            metadata,
            plan,
            task_visits,
            axis,
            sub_a,
            animation,
            OrdinaryType9RootAttractAttentionPredecessorKind::CargoRelease,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            &mut next_rng,
            emit_resource_text,
            emit_positional_sound,
        )
        .map(|outcome| match outcome {
            OrdinaryType9RootAttractAttentionApplicationOutcome::Published {
                publication,
                owners,
                ..
            } => OrdinaryType9CurrentTaskPublication::AttractAttention {
                publication,
                initial_owners: owners,
            },
            _ => unreachable!("infallible allocation publishes Attract Attention"),
        })
        .expect("preflighted carrying graph publishes the canonical Attract Attention constructor"),
        _ => unreachable!("exact Type-9 list contains only four weighted branches"),
    }
}

fn position_raw(entity: &Entity) -> [i16; 3] {
    entity.position_raw()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Type9RelationOwner {
    pub id: u32,
    pub capability_flags: u32,
    pub position_raw: [i16; 3],
}

/// All fallible attach reads, completed before relation or task publication.
#[derive(Debug)]
pub(crate) struct Type9CargoAttachPlan {
    entity_id: u32,
    authored_spawn_index: Option<usize>,
    initial_behavior: RetailRuntimeValue<Option<crate::entity_behavior::BehaviorSelection>>,
    before_context: BehaviorContextRuntime,
    carrying_context: BehaviorContextRuntime,
    selected: OrdinaryType9SelectedComponentRuntime,
    animation: ActorAnimationController,
    visits: [Option<ActorTaskVisit>; 3],
    task_states: [Option<ActorTaskRuntime>; 3],
    relation_owner: Type9RelationOwner,
}

impl Type9CargoAttachPlan {
    /// Retail emits this at the relation owner's position before writing
    /// Sub-I's linked handle or switching the behavior style.
    pub(crate) fn sound(&self) -> Option<(u16, [i16; 3])> {
        self.animation
            .relation_attach_sound_id(self.relation_owner.capability_flags)
            .map(|sound| (sound, self.relation_owner.position_raw))
    }
}

/// Linear custody of the carrying style and its actual None wrapper.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Type9CarriedOwner {
    pub(crate) actor: MainBaseAbortActorLease,
    authored_spawn_index: Option<usize>,
    initial_behavior: RetailRuntimeValue<Option<crate::entity_behavior::BehaviorSelection>>,
    pub(crate) context: BehaviorContextRuntime,
    pub(crate) selected: OrdinaryType9SelectedComponentRuntime,
    pub(crate) visit: ActorTaskVisit,
    pub(crate) linked_owner: u32,
    animation_linked_owner: Option<u32>,
}

impl Type9CarriedOwner {
    pub(crate) const fn entity_id(&self) -> u32 {
        self.actor.entity_id
    }

    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            actor: self.actor,
            authored_spawn_index: self.authored_spawn_index,
            initial_behavior: self.initial_behavior,
            context: self.context,
            selected: self.selected,
            visit: self.visit,
            linked_owner: self.linked_owner,
            animation_linked_owner: self.animation_linked_owner,
        }
    }
    pub(crate) fn authenticates(&self, entity: &Entity) -> bool {
        entity.id == self.actor.entity_id
            && entity.active
            && entity.entity_type == 9
            && entity.authored_spawn_index == self.authored_spawn_index
            && entity.initial_behavior == self.initial_behavior
            && entity.current_behavior_context == RetailRuntimeValue::Known(Some(self.context))
            && entity.ordinary_type9_selected_component_runtime == Some(self.selected)
            && visits(entity) == [Some(self.visit), None, None]
            && carrying_graph_matches(&entity.actor_tasks, self.context, self.selected.kind())
            && matches!(entity.actor_animation_runtime, RetailRuntimeValue::Known(Some(animation)) if
                animation.linked_handle() == self.animation_linked_owner)
    }
}

pub(crate) fn plan_attach(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    relation_owner: Type9RelationOwner,
) -> Result<Type9CargoAttachPlan, Type9CargoBlock> {
    if !entity.active || entity.entity_type != 9 || entity.capability_flags != 0x1804 {
        return Err(Type9CargoBlock::UnsupportedEntity);
    }
    if !exact_level_one_type9_metadata(metadata) {
        return Err(Type9CargoBlock::MetadataUnavailable);
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Type9CargoBlock::ContextUnavailable);
    };
    let carrying_context = carrying_context(context).ok_or(Type9CargoBlock::UnsupportedStyle)?;
    let selected = entity
        .ordinary_type9_selected_component_runtime
        .ok_or(Type9CargoBlock::ComponentUnavailable)?;
    if matches!(
        selected.kind(),
        OrdinaryType9SelectedRuntimeKind::Carried
            | OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
    ) {
        return Err(Type9CargoBlock::TaskGraphUnavailable);
    }
    let task_visits = visits(entity);
    if task_visits[0].is_none()
        || task_visits.iter().flatten().any(|visit| {
            entity.actor_tasks.wrapper_flags(visit.task_id)
                != Some(ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: false,
                })
        })
    {
        return Err(Type9CargoBlock::TaskGraphUnavailable);
    }
    let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
        return Err(Type9CargoBlock::ComponentUnavailable);
    };
    if metadata.actor_animation_descriptor
        != RetailRuntimeValue::Known(Some(animation.descriptor()))
    {
        return Err(Type9CargoBlock::ComponentUnavailable);
    }
    Ok(Type9CargoAttachPlan {
        entity_id: entity.id,
        authored_spawn_index: entity.authored_spawn_index,
        initial_behavior: entity.initial_behavior,
        before_context: context,
        carrying_context,
        selected,
        animation,
        visits: task_visits,
        task_states: ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .map(|slot| entity.actor_task_state(slot).copied()),
        relation_owner,
    })
}

/// The caller consumes completed scheduler custody before this commit. The
/// fixed `16700` bit/parent writes and the optional sound remain caller-owned.
pub(crate) fn commit_attach(
    entity: &mut Entity,
    actor: MainBaseAbortActorLease,
    plan: Type9CargoAttachPlan,
) -> Result<Type9CarriedOwner, Type9CargoBlock> {
    if entity.id != actor.entity_id
        || entity.id != plan.entity_id
        || entity.authored_spawn_index != plan.authored_spawn_index
        || entity.initial_behavior != plan.initial_behavior
        || !entity.active
        || entity.current_behavior_context != RetailRuntimeValue::Known(Some(plan.before_context))
        || entity.ordinary_type9_selected_component_runtime != Some(plan.selected)
        || entity.actor_animation_runtime != RetailRuntimeValue::Known(Some(plan.animation))
        || visits(entity) != plan.visits
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).copied())
            != plan.task_states
        || plan.visits.iter().flatten().any(|visit| {
            entity.actor_tasks.wrapper_flags(visit.task_id)
                != Some(ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: false,
                })
        })
    {
        return Err(Type9CargoBlock::StateChanged);
    }
    let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
        unreachable!("preflight retained Sub-I");
    };
    animation.apply_relation_attach(plan.relation_owner.id);
    // CD70 explicitly clears 8000 before C6B0; carrying style +34=80 repeats
    // that clear. Its +38=2 contributes no D440 state-mask bits.
    entity.collision.state_flags_at_0x08.overwrite(0x8000, 0);
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(plan.carrying_context));
    entity
        .actor_tasks
        .clear_slot_with_retirement(ActorTaskSlot::Tertiary, |task| {
            task.retire_animation(animation)
        });
    entity
        .actor_tasks
        .clear_slot_with_retirement(ActorTaskSlot::Secondary, |task| {
            task.retire_animation(animation)
        });
    entity.actor_tasks.replace_prepared_with_retirement(
        ActorTaskSlot::Primary,
        PreparedActorTask::new(ActorTaskRuntime::None),
        |task| task.retire_animation(animation),
    );
    // CD70 attaches Sub-I before ADB0 retires the old tasks. A retired cue's
    // 402AC0 destructor can therefore clear that animation link again while
    // the physical Sub-J parent relation remains attached.
    let animation_linked_owner = animation.linked_handle();
    let mut selected = plan.selected;
    selected.set_kind(OrdinaryType9SelectedRuntimeKind::Carried);
    entity.ordinary_type9_selected_component_runtime = Some(selected);
    let owner = Type9CarriedOwner {
        actor,
        authored_spawn_index: plan.authored_spawn_index,
        initial_behavior: plan.initial_behavior,
        context: plan.carrying_context,
        selected,
        visit: visits(entity)[0].expect("None constructor publishes Primary"),
        linked_owner: plan.relation_owner.id,
        animation_linked_owner,
    };
    debug_assert!(owner.authenticates(entity));
    Ok(owner)
}

fn carrying_context(context: BehaviorContextRuntime) -> Option<BehaviorContextRuntime> {
    let BehaviorDescriptorIdentity::Named(program) = context.descriptor() else {
        return None;
    };
    if context.choice_list_source()
        != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
    {
        return None;
    }
    let (accepted_variants, carrying_variant): (&[u8], u8) = match program.class_id {
        6 | 54 => (&[0], 1),
        10 | 45 => (&[0, 1], 2),
        _ => return None,
    };
    let ActiveBehaviorStyle::Audited(style) = context.active_style() else {
        return None;
    };
    if !accepted_variants.contains(&style.variant) {
        return None;
    }
    BehaviorContextRuntime::named_audited(
        program,
        u32::from(carrying_variant),
        context.choice_list_source(),
        context.target_handle_at_0x08(),
        context.auxiliary_word_at_0x0c(),
        *audited_behavior_style(u32::from(program.class_id), carrying_variant)?,
    )
}

pub(crate) fn carrying_graph_matches(
    tasks: &ActorTaskOwner<ActorTaskRuntime>,
    context: BehaviorContextRuntime,
    kind: OrdinaryType9SelectedRuntimeKind,
) -> bool {
    let BehaviorDescriptorIdentity::Named(program) = context.descriptor() else {
        return false;
    };
    let variant = match program.class_id {
        6 | 54 => 1,
        10 | 45 => 2,
        _ => return false,
    };
    kind == OrdinaryType9SelectedRuntimeKind::Carried
        && context.choice_list_source()
            == RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        && context.style_table_index_raw_at_0x10() == u32::from(variant)
        && audited_behavior_style(u32::from(program.class_id), variant)
            .is_some_and(|style| context.active_style() == ActiveBehaviorStyle::Audited(*style))
        && matches!(
            tasks.state_in_slot(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::None)
        )
        && tasks.task_in_slot(ActorTaskSlot::Secondary).is_none()
        && tasks.task_in_slot(ActorTaskSlot::Tertiary).is_none()
        && tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .is_some_and(|id| {
                tasks.wrapper_flags(id)
                    == Some(ActorTaskWrapperFlags {
                        alive: true,
                        in_callback: false,
                    })
            })
}

fn visits(entity: &Entity) -> [Option<ActorTaskVisit>; 3] {
    ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
        entity
            .actor_tasks
            .task_in_slot(slot)
            .map(|task_id| ActorTaskVisit { slot, task_id })
    })
}

#[cfg(test)]
mod tests;
