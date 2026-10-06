//! Type49's null Stationary attach and CD50/CE90/D1C0 relation callbacks.
use super::*;
use crate::{
    common_mover::type9_attitude::Type9BodyBasis,
    entity_behavior::{BehaviorContextRuntime, BehaviorDescriptorIdentity},
    entity_relation_release::relation_release_state_word_after,
    ordinary_type9_cargo::Type9CargoReleasePosition,
    terrain_crater::terrain_aligned_actor_basis,
    world_fx::WorldFx,
};
use v2k_formats::terrain::TerrainGrid;

pub(crate) struct AttachPlan {
    pub cargo: u32,
    pub parent: u32,
    metadata: EntityTypeRuntimeMetadata,
    carrying: bool,
}
pub(crate) struct ReleasePlan {
    pub cargo: u32,
    metadata: EntityTypeRuntimeMetadata,
    context: BehaviorContextRuntime,
    position: Type9CargoReleasePosition,
    aligned_basis: Option<Type9BodyBasis>,
    base_nearby: bool,
}

fn relation_owner(
    manager: &EntityManager,
    id: u32,
    parent: u32,
) -> Result<BehaviorContextRuntime, CleansingVehicleError> {
    if !allocation_authenticates(manager, id) {
        return Err(CleansingVehicleError::Allocation);
    }
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    if entity.collision.state_flags_at_0x08.masked(0x8000_4000) != RetailRuntimeValue::Known(0)
        || !manager.iter_all().any(|e| {
            e.id == parent
                && e.active
                && e.collision.state_flags_at_0x08.masked(0x8000_0000)
                    == RetailRuntimeValue::Known(0)
                && matches!(
                    e.sub_j_attachment_runtime,
                    RetailRuntimeValue::Known(Some(_))
                )
        })
    {
        return Err(CleansingVehicleError::Runtime(
            "local living cleansing relation",
        ));
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(CleansingVehicleError::Graph);
    };
    let BehaviorDescriptorIdentity::Named(program) = context.descriptor() else {
        return Err(CleansingVehicleError::Graph);
    };
    if !matches!(
        (program.class_id, context.style_table_index_raw_at_0x10()),
        (68, 0) | (42, 0 | 1)
    ) {
        return Err(CleansingVehicleError::Graph);
    }
    authenticate_metadata(
        manager
            .type_runtime_metadata(49)
            .ok_or(CleansingVehicleError::Metadata)?,
    )?;
    Ok(context)
}

pub(crate) fn prepare_attach(
    manager: &EntityManager,
    id: u32,
    parent: u32,
) -> Result<AttachPlan, CleansingVehicleError> {
    let context = relation_owner(manager, id, parent)?;
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    if entity.attached_to.is_some_and(|old| old != parent) {
        return Err(CleansingVehicleError::Runtime("cleansing old parent"));
    }
    Ok(AttachPlan {
        cargo: id,
        parent,
        metadata: manager.type_runtime_metadata(49).unwrap().clone(),
        carrying: context.active_style().style_address() == 0x004c85d8,
    })
}

/// Membership+parent are committed by16700's caller before this style callback.
pub(crate) fn commit_attach(entity: &mut Entity, plan: AttachPlan, world_fx: &mut WorldFx) {
    if plan.carrying {
        tasks::publish_carrying(entity, &plan.metadata, &mut || {
            u32::from(world_fx.next_shared_retail_random_u16())
        })
        .expect("preflighted Type49 carrying graph");
    }
}

pub(crate) fn prepare_release(
    manager: &EntityManager,
    id: u32,
    parent: u32,
    position: Type9CargoReleasePosition,
    terrain: &TerrainGrid,
) -> Result<ReleasePlan, CleansingVehicleError> {
    let context = relation_owner(manager, id, parent)?;
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    if entity.attached_to != Some(parent)
        || entity.collision.state_flags_at_0x08.masked(0x1000) != RetailRuntimeValue::Known(0x1000)
    {
        return Err(CleansingVehicleError::Runtime("cleansing release relation"));
    }
    let metadata = manager.type_runtime_metadata(49).unwrap().clone();
    let position_raw = match position {
        Type9CargoReleasePosition::Retained => entity.position_raw(),
        Type9CargoReleasePosition::Materialiser(raw) => raw,
    };
    let mut candidate = tasks::candidate(entity);
    candidate.position_raw = position_raw;
    candidate.state_flags_raw =
        relation_release_state_word_after(entity.collision.state_flags_at_0x08, 0x8039);
    let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
        return Err(CleansingVehicleError::Graph);
    };
    let candidates: Vec<_> = manager
        .retail_live_order_ids()
        .filter_map(|id| manager.iter_all().find(|e| e.id == id && e.active))
        .map(tasks::candidate)
        .collect();
    let base_nearby = tasks::base_nearby(candidate, &candidates, axis)?;
    Ok(ReleasePlan {
        cargo: id,
        metadata,
        context,
        position,
        base_nearby,
        aligned_basis: if context.active_style().release_callback_policy()
            == crate::entity_behavior::ReleaseCallbackPolicy::TerrainAlignAndReselect
        {
            Some(terrain_aligned_actor_basis(terrain, position_raw))
        } else {
            None
        },
    })
}

pub(crate) fn commit_release(
    manager: &mut EntityManager,
    plan: ReleasePlan,
    world_fx: &mut WorldFx,
) {
    let entity = manager.entity_mut(plan.cargo).unwrap();
    if let Type9CargoReleasePosition::Materialiser(raw) = plan.position {
        entity.set_position_raw(raw);
        entity
            .cleansing_vehicle_runtime
            .as_mut()
            .unwrap()
            .immutable_anchor_raw = raw;
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x40000, 0x40000);
    }
    entity.collision.state_flags_at_0x08 =
        relation_release_state_word_after(entity.collision.state_flags_at_0x08, 0x8039);
    entity.collision.default_state_flags_at_0xc8 = RetailRuntimeValue::Known(0x8039);
    entity.attached_to = None;
    if let Some(basis) = plan.aligned_basis {
        entity.physical_body_basis_q31 = RetailRuntimeValue::Known(basis);
        entity.collision.state_flags_at_0x08.overwrite(0x28, 0x28);
    } else {
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x8000, 0x8000);
    }
    let mut random = || u32::from(world_fx.next_shared_retail_random_u16());
    let selection = tasks::select_with_base(plan.base_nearby, &mut random)
        .expect("authenticated Type49 selector");
    let context = plan
        .context
        .reselect_named_type_default(selection.program, 0, selection.program.initial_style)
        .unwrap();
    tasks::publish_selection(entity, &plan.metadata, selection, context, &mut random)
        .expect("prepared release graph");
}
