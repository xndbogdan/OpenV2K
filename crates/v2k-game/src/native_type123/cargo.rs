//! Type123's 16700/DBF0/CD50/CE70 attachment and 409030/16750/CE90 release.
//!
//! Class45 variant0/variant1 and class6 share the CE70 variant-2 / variant-1
//! carrying selection with Type9, but use their own receipt, Sub-I descriptor
//! and Always-only root. No Type9 allocation is borrowed.

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
pub(crate) struct Type123AttachPlan {
    attachment: NativeActorAttachmentPlan,
}

pub(crate) fn prepare_attach(
    manager: &EntityManager,
    id: u32,
    parent: u32,
) -> Result<Type123AttachPlan, Type123Block> {
    if !native_type123_manager_allocation_authenticates(manager, id) {
        return Err(Type123Block::Runtime("Type123 attach allocation"));
    }
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .ok_or(Type123Block::Runtime("Type123 attach metadata"))?;
    birth::validate_metadata(metadata)?;
    let (_, context, kind) = Type123Owner::published_graph(entity)
        .ok_or(Type123Block::Runtime("Type123 attach graph"))?;
    if entity.attached_to.is_some()
        || !matches!(
            kind,
            TaskKind::Wander
                | TaskKind::AttractAcquiring
                | TaskKind::AttractTarget
                | TaskKind::Exploding
        )
    {
        return Err(Type123Block::Runtime("Type123 attach style"));
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
        return Err(Type123Block::Runtime("Type123 local relation"));
    }
    let BehaviorDescriptorIdentity::Named(program) = context.descriptor() else {
        return Err(Type123Block::Runtime("Type123 attach context"));
    };
    if context.choice_list_source()
        != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
    {
        return Err(Type123Block::Runtime("Type123 attach choice list"));
    }
    // CD50 selects variant1 for class6 (ADB0) or class14 (terminal C470);
    // CE70 selects variant2 for class45 Attract (ADB0).
    let carrying_variant: u8 = match program.class_id {
        6 | 14 => 1,
        45 => 2,
        _ => return Err(Type123Block::Runtime("Type123 carrying class")),
    };
    let carrying = if kind == TaskKind::Exploding {
        crate::main_base_type9_abort::exploding_person_terminal_context(context)
            .ok_or(Type123Block::Runtime("Type123 terminal attach style"))?
    } else {
        BehaviorContextRuntime::named_audited(
            program,
            u32::from(carrying_variant),
            context.choice_list_source(),
            context.target_handle_at_0x08(),
            context.auxiliary_word_at_0x0c(),
            *audited_behavior_style(u32::from(program.class_id), carrying_variant)
                .ok_or(Type123Block::Runtime("Type123 carrying style"))?,
        )
        .ok_or(Type123Block::Runtime("Type123 carrying context"))?
    };
    let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
        return Err(Type123Block::Runtime("Type123 attach Sub-I"));
    };
    if metadata.actor_animation_descriptor
        != RetailRuntimeValue::Known(Some(animation.descriptor()))
        || (kind != TaskKind::Exploding && animation.special_mode())
    {
        return Err(Type123Block::Runtime("Type123 attach Sub-I descriptor"));
    }
    Ok(Type123AttachPlan {
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
        .map_err(Type123Block::Runtime)?,
    })
}

/// Called immediately after 16700's relation-bit and parent writes. The
/// caller has already reserved the Sub-J row, so no fallible allocation remains.
pub(crate) fn commit_attach(
    manager: &mut EntityManager,
    id: u32,
    plan: Type123AttachPlan,
    fx: &mut WorldFx,
) -> NativeActorAttachOutcome {
    assert!(native_type123_manager_allocation_authenticates(manager, id));
    let outcome = plan.attachment.commit(manager, id, fx);
    let entity = manager.entity_mut(id).unwrap();
    match outcome {
        NativeActorAttachOutcome::RetainedGraph => retain_graph(entity),
        NativeActorAttachOutcome::DeferredDestroy => {
            let runtime = entity.native_type123_runtime.as_mut().unwrap();
            runtime.relation_graph = None;
            runtime.birth_pending = false;
        }
    }
    outcome
}

pub(crate) struct Type123ReleasePlan {
    callback: Type123ReleaseCallback,
    position: Type9CargoReleasePosition,
    task_id: ActorTaskId,
    context: BehaviorContextRuntime,
}

enum Type123ReleaseCallback {
    /// CE90 reselects the Always-only root against current world candidates
    /// at commit time, after the 18500 pop and anchor update.
    CarryingReselect,
    /// Standard death may replace carrying while retaining physical Sub-J.
    /// Class14 has no DC50 style callback, so 16750 keeps its current task.
    None,
}

/// Resolve every fallible read before 18500 removes the Sub-J row. RNG draws
/// stay at commit time: CE90 invokes AC60 only after the fixed relation
/// release writes, and this profile's Always-only root reads no candidates
/// at prepare time.
pub(crate) fn prepare_release(
    manager: &EntityManager,
    id: u32,
    parent: u32,
    position: Type9CargoReleasePosition,
) -> Result<Type123ReleasePlan, Type123Block> {
    if !native_type123_manager_allocation_authenticates(manager, id) {
        return Err(Type123Block::Runtime("Type123 release allocation"));
    }
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .ok_or(Type123Block::Runtime("Type123 release metadata"))?;
    birth::validate_metadata(metadata)?;
    let (task_id, context, kind) = Type123Owner::published_graph(entity)
        .ok_or(Type123Block::Runtime("Type123 release graph"))?;
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
        return Err(Type123Block::Runtime("Type123 release components"));
    }
    let callback = if kind == TaskKind::Carried {
        Type123ReleaseCallback::CarryingReselect
    } else {
        Type123ReleaseCallback::None
    };
    Ok(Type123ReleasePlan {
        callback,
        position,
        task_id,
        context,
    })
}

pub(crate) fn commit_release(
    manager: &mut EntityManager,
    id: u32,
    plan: Type123ReleasePlan,
    fx: &mut WorldFx,
) -> Type123Owner {
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
        entity.native_type123_anchor_raw_at_0x90 = RetailRuntimeValue::Known(raw);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x40000, 0x40000);
    }
    entity.collision.state_flags_at_0x08 =
        relation_release_state_word_after(entity.collision.state_flags_at_0x08, 0x2f);
    entity.collision.default_state_flags_at_0xc8 = RetailRuntimeValue::Known(0x2f);
    entity.attached_to = None;
    if matches!(plan.callback, Type123ReleaseCallback::CarryingReselect) {
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x8000, 0x8000);
        let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
            unreachable!("release prepared Sub-I");
        };
        animation.apply_relation_release();
        // CE90 reselects the Always-only root after the row is gone. Prepare
        // validated metadata and graph, so this cannot newly fail.
        let owner =
            birth::reselect(manager, id, fx).expect("release preflighted the Always-only root");
        retain_graph(manager.entity_mut(id).unwrap());
        return owner;
    }
    retain_graph(manager.entity_mut(id).unwrap());
    Type123Owner::adopt_published(manager.iter_all().find(|e| e.id == id).unwrap())
        .expect("release prepared its graph")
}

fn retain_graph(entity: &mut Entity) {
    let (task_id, context, _) = Type123Owner::published_graph(entity)
        .expect("relation transaction completed its actual task graph");
    let runtime = entity.native_type123_runtime.as_mut().unwrap();
    runtime.relation_graph = Some(RelationGraphPublication { task_id, context });
    // A tail-born person can attach before its first adoption. The exact
    // relation graph authorizes that transfer without rewriting birth evidence.
}
