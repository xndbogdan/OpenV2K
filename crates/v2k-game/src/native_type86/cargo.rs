//! Native people's 16700/DBF0 carrying and 409030/16750/CE90 release.
//!
//! Classes 10/45 share the CE70 variant-2 carrying selection while classes
//! 6/54 use variant1, mirroring the Type9 carrying table. Attach, release
//! and death retain each person's receipt, Sub-I and four-branch root.
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
pub(crate) struct Type86AttachPlan {
    attachment: NativeActorAttachmentPlan,
}

pub(crate) fn prepare_attach(
    manager: &EntityManager,
    id: u32,
    parent: u32,
) -> Result<Type86AttachPlan, Type86Block> {
    if !native_type86_manager_allocation_authenticates(manager, id) {
        return Err(Type86Block::Runtime("Type86 attach allocation"));
    }
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .ok_or(Type86Block::Runtime("Type86 attach metadata"))?;
    birth::validate_metadata(entity.native_type86_runtime.unwrap().profile, metadata)?;
    let (_, context, kind) =
        Type86Owner::published_graph(entity).ok_or(Type86Block::Runtime("Type86 attach graph"))?;
    if entity.attached_to.is_some()
        || !matches!(
            kind,
            TaskKind::Wander
                | TaskKind::GoToJob
                | TaskKind::AttractAcquiring
                | TaskKind::AttractTarget
                | TaskKind::RunAwayAcquiring
                | TaskKind::RunAwayFleeing
                | TaskKind::Exploding
        )
    {
        return Err(Type86Block::Runtime("Type86 attach style"));
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
        return Err(Type86Block::Runtime("Type86 local relation"));
    }
    let BehaviorDescriptorIdentity::Named(program) = context.descriptor() else {
        return Err(Type86Block::Runtime("Type86 attach context"));
    };
    if context.choice_list_source()
        != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
    {
        return Err(Type86Block::Runtime("Type86 attach choice list"));
    }
    // CD50 selects variant1 for class6/54; CE70 selects variant2 for10/45.
    // Class14 also selects variant1, whose C470 retires the actor immediately.
    let carrying_variant: u8 = match program.class_id {
        6 | 54 | 14 => 1,
        10 | 45 => 2,
        _ => return Err(Type86Block::Runtime("Type86 carrying class")),
    };
    let carrying = if kind == TaskKind::Exploding {
        crate::main_base_type9_abort::exploding_person_terminal_context(context)
            .ok_or(Type86Block::Runtime("Type86 terminal attach style"))?
    } else {
        BehaviorContextRuntime::named_audited(
            program,
            u32::from(carrying_variant),
            context.choice_list_source(),
            context.target_handle_at_0x08(),
            context.auxiliary_word_at_0x0c(),
            *audited_behavior_style(u32::from(program.class_id), carrying_variant)
                .ok_or(Type86Block::Runtime("Type86 carrying style"))?,
        )
        .ok_or(Type86Block::Runtime("Type86 carrying context"))?
    };
    let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
        return Err(Type86Block::Runtime("Type86 attach Sub-I"));
    };
    if metadata.actor_animation_descriptor
        != RetailRuntimeValue::Known(Some(animation.descriptor()))
        || (kind != TaskKind::Exploding && animation.special_mode())
    {
        return Err(Type86Block::Runtime("Type86 attach Sub-I descriptor"));
    }
    Ok(Type86AttachPlan {
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
        .map_err(Type86Block::Runtime)?,
    })
}

/// Called immediately after 16700's relation-bit and parent writes. The
/// caller has already reserved the Sub-J row, so no fallible allocation remains.
pub(crate) fn commit_attach(
    manager: &mut EntityManager,
    id: u32,
    plan: Type86AttachPlan,
    fx: &mut WorldFx,
) -> NativeActorAttachOutcome {
    assert!(native_type86_manager_allocation_authenticates(manager, id));
    let outcome = plan.attachment.commit(manager, id, fx);
    let entity = manager.entity_mut(id).unwrap();
    match outcome {
        NativeActorAttachOutcome::RetainedGraph => retain_graph(entity),
        NativeActorAttachOutcome::DeferredDestroy => {
            let runtime = entity.native_type86_runtime.as_mut().unwrap();
            runtime.relation_graph = None;
            runtime.birth_pending = false;
        }
    }
    outcome
}

pub(crate) struct Type86ReleasePlan {
    callback: Type86ReleaseCallback,
    position: Type9CargoReleasePosition,
    task_id: ActorTaskId,
    context: BehaviorContextRuntime,
}

enum Type86ReleaseCallback {
    /// CE90 reselects the four-branch root against current world candidates
    /// at commit time, after the 18500 pop and anchor update.
    CarryingReselect,
    /// Standard death may replace carrying while retaining physical Sub-J.
    /// Class14 has no DC50 style callback, so 16750 keeps its current task.
    None,
}

/// Resolve every fallible read before 18500 removes the Sub-J row. RNG draws
/// stay at commit time: CE90 invokes AC60 only after the fixed relation
/// release writes.
pub(crate) fn prepare_release(
    manager: &EntityManager,
    id: u32,
    parent: u32,
    position: Type9CargoReleasePosition,
) -> Result<Type86ReleasePlan, Type86Block> {
    if !native_type86_manager_allocation_authenticates(manager, id) {
        return Err(Type86Block::Runtime("Type86 release allocation"));
    }
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .ok_or(Type86Block::Runtime("Type86 release metadata"))?;
    birth::validate_metadata(entity.native_type86_runtime.unwrap().profile, metadata)?;
    let (task_id, context, kind) =
        Type86Owner::published_graph(entity).ok_or(Type86Block::Runtime("Type86 release graph"))?;
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
                && metadata.actor_animation_descriptor
                    == RetailRuntimeValue::Known(Some(animation.descriptor())))
    {
        return Err(Type86Block::Runtime("Type86 release components"));
    }
    let callback = if kind == TaskKind::Carried {
        let mut state = entity.collision.state_flags_at_0x08;
        let release_position = match position {
            Type9CargoReleasePosition::Retained => entity.position_raw(),
            Type9CargoReleasePosition::Materialiser(raw) => {
                state.overwrite(0x40000, 0x40000);
                raw
            }
        };
        state = relation_release_state_word_after(state, 0x2f);
        state.overwrite(0x8000, 0x8000);
        birth::preflight_release_root(manager, id, release_position, state)?;
        Type86ReleaseCallback::CarryingReselect
    } else {
        Type86ReleaseCallback::None
    };
    Ok(Type86ReleasePlan {
        callback,
        position,
        task_id,
        context,
    })
}

pub(crate) fn commit_release(
    manager: &mut EntityManager,
    id: u32,
    plan: Type86ReleasePlan,
    fx: &mut WorldFx,
) -> Type86Owner {
    let entity = manager.entity_mut(id).expect("release prepared its entity");
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
        entity.native_type86_anchor_raw_at_0x90 = RetailRuntimeValue::Known(raw);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x40000, 0x40000);
    }
    entity.collision.state_flags_at_0x08 =
        relation_release_state_word_after(entity.collision.state_flags_at_0x08, 0x2f);
    entity.collision.default_state_flags_at_0xc8 = RetailRuntimeValue::Known(0x2f);
    entity.attached_to = None;
    if matches!(plan.callback, Type86ReleaseCallback::CarryingReselect) {
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x8000, 0x8000);
        let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
            unreachable!("release prepared Sub-I");
        };
        animation.apply_relation_release();
        // CE90 reselects the four-branch root after the row is gone. Prepare
        // validated metadata and graph, so this cannot newly fail.
        let owner =
            birth::reselect(manager, id, fx).expect("release preflighted the four-branch root");
        retain_graph(manager.entity_mut(id).unwrap());
        return owner;
    }
    retain_graph(manager.entity_mut(id).unwrap());
    Type86Owner::adopt_published(manager.iter_all().find(|e| e.id == id).unwrap())
        .expect("release prepared its graph")
}

fn retain_graph(entity: &mut Entity) {
    let (task_id, context, _) = Type86Owner::published_graph(entity)
        .expect("relation transaction completed its actual task graph");
    let runtime = entity.native_type86_runtime.as_mut().unwrap();
    runtime.relation_graph = Some(RelationGraphPublication { task_id, context });
    // A tail-born person can attach before its first adoption. The exact
    // relation graph authorizes that transfer without rewriting birth evidence.
}
