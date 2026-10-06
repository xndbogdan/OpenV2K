//! Type8's 16700/DBF0/CD50 carrying and 409030/16750/CE90 release.
//!
//! Its classes6/54 share these styles with Type9, but use their own receipt,
//! actual Sub-I descriptor and two-way root. The parent's capabilities select
//! the child's attach cue; its position supplies the sound source. No Type9
//! allocation is borrowed.

use super::*;
use crate::{
    entity_behavior::{
        audited_behavior_style, BehaviorChoiceListSource, BehaviorDescriptorIdentity,
    },
    entity_relation_release::relation_release_state_word_after,
    native_actor_attachment::{NativeActorAttachOutcome, NativeActorAttachmentPlan},
    ordinary_type9_cargo::Type9CargoReleasePosition,
};

/// Exact graph published synchronously outside the actor-list callback. Only
/// the attach/release transactions below can mint this transfer receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Type8RelationGraphPublication {
    task_id: ActorTaskId,
    context: BehaviorContextRuntime,
}

impl Type8RelationGraphPublication {
    pub(super) fn matches(self, task_id: ActorTaskId, context: BehaviorContextRuntime) -> bool {
        self.task_id == task_id && self.context == context
    }
}

pub(crate) struct Type8AttachPlan {
    attachment: NativeActorAttachmentPlan,
}

pub(crate) fn prepare_attach(
    manager: &EntityManager,
    id: u32,
    parent: u32,
) -> Result<Type8AttachPlan, Intro2Type8Block> {
    if !intro2_type8_manager_allocation_authenticates(manager, id) {
        return Err(Intro2Type8Block::Runtime("Type8 attach allocation"));
    }
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .ok_or(Intro2Type8Block::Runtime("Type8 attach metadata"))?;
    validate_worker_metadata(entity.entity_type, metadata)?;
    let (_, context, kind) = Intro2Type8Owner::published_graph(entity)
        .ok_or(Intro2Type8Block::Runtime("Type8 attach graph"))?;
    if entity.attached_to.is_some()
        || !matches!(
            kind,
            TaskKind::Wander | TaskKind::GoToJob | TaskKind::Exploding
        )
    {
        return Err(Intro2Type8Block::Runtime("Type8 attach style"));
    }
    if entity
        .collision
        .state_flags_at_0x08
        .masked(crate::entity_collision_state::REMOTE_OWNED_STATE_BIT)
        != RetailRuntimeValue::Known(0)
        || !manager.iter_all().any(|entity| {
            entity.id == parent
                && entity.active
                && entity
                    .collision
                    .state_flags_at_0x08
                    .masked(crate::entity_collision_state::REMOTE_OWNED_STATE_BIT)
                    == RetailRuntimeValue::Known(0)
        })
    {
        return Err(Intro2Type8Block::Runtime("Type8 local relation"));
    }
    let BehaviorDescriptorIdentity::Named(program) = context.descriptor() else {
        return Err(Intro2Type8Block::Runtime("Type8 attach context"));
    };
    if context.choice_list_source()
        != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
    {
        return Err(Intro2Type8Block::Runtime("Type8 attach choice list"));
    }
    let carrying = if kind == TaskKind::Exploding {
        crate::main_base_type9_abort::exploding_person_terminal_context(context)
            .ok_or(Intro2Type8Block::Runtime("Type8 terminal attach style"))?
    } else {
        BehaviorContextRuntime::named_audited(
            program,
            1,
            context.choice_list_source(),
            context.target_handle_at_0x08(),
            context.auxiliary_word_at_0x0c(),
            *audited_behavior_style(u32::from(program.class_id), 1)
                .ok_or(Intro2Type8Block::Runtime("Type8 carrying style"))?,
        )
        .ok_or(Intro2Type8Block::Runtime("Type8 carrying context"))?
    };
    let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
        return Err(Intro2Type8Block::Runtime("Type8 attach Sub-I"));
    };
    if metadata.actor_animation_descriptor
        != RetailRuntimeValue::Known(Some(animation.descriptor()))
        || (kind != TaskKind::Exploding && animation.special_mode())
    {
        return Err(Intro2Type8Block::Runtime("Type8 attach Sub-I descriptor"));
    }
    Ok(Type8AttachPlan {
        attachment: NativeActorAttachmentPlan::prepare(
            manager,
            id,
            parent,
            carrying,
            if kind == TaskKind::Exploding {
                NativeActorAttachOutcome::DeferredDestroy
            } else {
                NativeActorAttachOutcome::RetainedGraph
            },
        )
        .map_err(Intro2Type8Block::Runtime)?,
    })
}

/// Called immediately after 16700's relation-bit and parent writes. The
/// caller has already reserved the Sub-J row, so no fallible allocation remains.
pub(crate) fn commit_attach(
    manager: &mut EntityManager,
    id: u32,
    plan: Type8AttachPlan,
    fx: &mut WorldFx,
) -> NativeActorAttachOutcome {
    assert!(intro2_type8_manager_allocation_authenticates(manager, id));
    let outcome = plan.attachment.commit(manager, id, fx);
    let entity = manager.entity_mut(id).unwrap();
    match outcome {
        NativeActorAttachOutcome::RetainedGraph => retain_graph(entity),
        NativeActorAttachOutcome::DeferredDestroy => {
            let runtime = entity.intro2_type8_runtime.as_mut().unwrap();
            runtime.relation_graph = None;
            runtime.birth_pending = false;
        }
    }
    outcome
}

pub(crate) struct Type8ReleasePlan {
    metadata: EntityTypeRuntimeMetadata,
    callback: Type8ReleaseCallback,
    position: Type9CargoReleasePosition,
    task_id: ActorTaskId,
    context: BehaviorContextRuntime,
}

enum Type8ReleaseCallback {
    CarryingReselect(birth::RootPlan),
    /// Standard death may replace carrying while retaining physical Sub-J.
    /// Class14 has no DC50 style callback, so 16750 keeps its current task.
    None,
}

/// Resolve every root/constructor read before 18500 removes the Sub-J row.
pub(crate) fn prepare_release(
    manager: &EntityManager,
    id: u32,
    parent: u32,
    position: Type9CargoReleasePosition,
) -> Result<Type8ReleasePlan, Intro2Type8Block> {
    if !intro2_type8_manager_allocation_authenticates(manager, id) {
        return Err(Intro2Type8Block::Runtime("Type8 release allocation"));
    }
    let entity_type = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .map(|entity| entity.entity_type)
        .ok_or(Intro2Type8Block::Runtime("Type8 release entity"))?;
    let metadata = manager
        .type_runtime_metadata(entity_type)
        .ok_or(Intro2Type8Block::Runtime("Type8 release metadata"))?
        .clone();
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let (task_id, context, kind) = Intro2Type8Owner::published_graph(entity)
        .ok_or(Intro2Type8Block::Runtime("Type8 release graph"))?;
    if !matches!(kind, TaskKind::Carried | TaskKind::Exploding)
        || entity.attached_to != Some(parent)
        || entity
            .collision
            .state_flags_at_0x08
            .masked(crate::entity_collision_state::REMOTE_OWNED_STATE_BIT)
            != RetailRuntimeValue::Known(0)
        || !manager.iter_all().any(|entity| {
            entity.id == parent
                && entity.active
                && entity
                    .collision
                    .state_flags_at_0x08
                    .masked(crate::entity_collision_state::REMOTE_OWNED_STATE_BIT)
                    == RetailRuntimeValue::Known(0)
        })
        || context.choice_list_source()
            != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        || !matches!(entity.sub_a_propulsion_runtime, RetailRuntimeValue::Known(Some(sub_a))
            if matches!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(_)))
        || !matches!(entity.actor_animation_runtime, RetailRuntimeValue::Known(Some(animation))
            if (kind == TaskKind::Exploding || !animation.special_mode())
                && metadata.actor_animation_descriptor == RetailRuntimeValue::Known(Some(animation.descriptor())))
    {
        return Err(Intro2Type8Block::Runtime("Type8 release components"));
    }
    let candidates = manager
        .retail_live_order_ids()
        .filter_map(|id| manager.iter_all().find(|entity| entity.id == id))
        .map(birth::candidate)
        .collect::<Vec<_>>();
    let position_raw = match position {
        Type9CargoReleasePosition::Retained => entity.position_raw(),
        Type9CargoReleasePosition::Materialiser(raw) => raw,
    };
    let callback = if kind == TaskKind::Carried {
        Type8ReleaseCallback::CarryingReselect(birth::plan_root(
            entity,
            &metadata,
            &candidates,
            position_raw,
        )?)
    } else {
        Type8ReleaseCallback::None
    };
    Ok(Type8ReleasePlan {
        metadata,
        callback,
        position,
        task_id,
        context,
    })
}

pub(crate) fn commit_release(entity: &mut Entity, plan: Type8ReleasePlan, fx: &mut WorldFx) {
    assert_eq!(
        entity.current_behavior_context,
        RetailRuntimeValue::Known(Some(plan.context))
    );
    assert_eq!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        Some(plan.task_id)
    );
    if let Type9CargoReleasePosition::Materialiser(raw) = plan.position {
        entity.set_position_raw(raw);
        entity.type8_wander_anchor_raw_at_0x90 = RetailRuntimeValue::Known(raw);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x40000, 0x40000);
    }
    entity.collision.state_flags_at_0x08 =
        relation_release_state_word_after(entity.collision.state_flags_at_0x08, 0x2f);
    entity.collision.default_state_flags_at_0xc8 = RetailRuntimeValue::Known(0x2f);
    entity.attached_to = None;
    if let Type8ReleaseCallback::CarryingReselect(root) = plan.callback {
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x8000, 0x8000);
        let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
            unreachable!("release prepared Sub-I");
        };
        animation.apply_relation_release();
        birth::apply_root(entity, &plan.metadata, root, &mut || {
            u32::from(fx.next_shared_retail_random_u16())
        })
        .expect("release preflights the two supported root constructors");
    }
    retain_graph(entity);
}

fn retain_graph(entity: &mut Entity) {
    let (task_id, context, _) = Intro2Type8Owner::published_graph(entity)
        .expect("relation transaction completed its actual task graph");
    let runtime = entity.intro2_type8_runtime.as_mut().unwrap();
    runtime.relation_graph = Some(Type8RelationGraphPublication { task_id, context });
    // A tail-born worker can attach before its first adoption. The exact
    // relation graph authorizes that transfer without rewriting birth evidence.
}
