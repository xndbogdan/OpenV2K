//! Type68's null class0 attach and D1C0 terrain-align/reselection release.
//!
//! DC50 at40DC76/40DC85 passes entity+C0 as the second callback argument;
//! the packaged parent is the third argument. D1C0 at40D1CB/40D1D2 reads
//! that second argument's context[0], so AC60 receives the type-default
//! choice source. No packed handle is interpreted as a choice-list address.

use super::*;
use crate::{
    entity_relation_release::relation_release_state_word_after,
    ordinary_type9_cargo::Type9CargoReleasePosition, terrain_crater::terrain_aligned_actor_basis,
};

pub(crate) struct Class0AttachPlan {
    pub cargo: u32,
    pub parent: u32,
    owner: Class0ActorOwner,
}

pub(crate) struct Class0ReleasePlan {
    pub cargo: u32,
    owner: Class0ActorOwner,
    metadata: EntityTypeRuntimeMetadata,
    position: Type9CargoReleasePosition,
    body_basis: Type9BodyBasis,
}

fn relation_owner(
    manager: &EntityManager,
    cargo: u32,
    parent: u32,
) -> Result<Class0ActorOwner, Class0ActorError> {
    let owner = Class0ActorOwner::adopt(manager, cargo)?;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == cargo)
        .ok_or(Class0ActorError::Allocation)?;
    if entity.entity_type != 68 {
        return Err(Class0ActorError::Runtime("class0 cargo type"));
    }
    authenticate_metadata(
        entity.entity_type,
        manager
            .type_runtime_metadata(entity.entity_type)
            .ok_or(Class0ActorError::Metadata)?,
    )?;
    if entity
        .collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT)
        != RetailRuntimeValue::Known(0)
        || !manager.iter_all().any(|entity| {
            entity.id == parent
                && entity.active
                && entity
                    .collision
                    .state_flags_at_0x08
                    .masked(REMOTE_OWNED_STATE_BIT)
                    == RetailRuntimeValue::Known(0)
                && matches!(
                    entity.sub_j_attachment_runtime,
                    RetailRuntimeValue::Known(Some(_))
                )
        })
    {
        return Err(Class0ActorError::Runtime("local class0 relation"));
    }
    Ok(owner)
}

pub(crate) fn prepare_attach(
    manager: &EntityManager,
    cargo: u32,
    parent: u32,
) -> Result<Class0AttachPlan, Class0ActorError> {
    let owner = relation_owner(manager, cargo, parent)?;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == cargo)
        .unwrap();
    if entity.attached_to.is_some()
        || entity.collision.state_flags_at_0x08.masked(0x1000) != RetailRuntimeValue::Known(0)
    {
        return Err(Class0ActorError::Runtime("class0 attach relation"));
    }
    Ok(Class0AttachPlan {
        cargo,
        parent,
        owner,
    })
}

pub(crate) fn commit_attach(
    manager: &mut EntityManager,
    plan: Class0AttachPlan,
    publish_sub_j: impl FnOnce(&mut EntityManager),
) {
    assert_eq!(Class0ActorOwner::adopt(manager, plan.cargo), Ok(plan.owner));
    publish_sub_j(manager);
    // 16700 writes membership/parent. Class0 style4C7468+08 is null:
    // DBF0 returns zero without replacing the timer or adding a Sub-I task.
    project_relation_attach(manager.entity_mut(plan.cargo).unwrap(), plan.parent);
}

pub(crate) fn prepare_release(
    manager: &EntityManager,
    cargo: u32,
    parent: u32,
    position: Type9CargoReleasePosition,
    terrain: &TerrainGrid,
) -> Result<Class0ReleasePlan, Class0ActorError> {
    let owner = relation_owner(manager, cargo, parent)?;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == cargo)
        .unwrap();
    if entity.attached_to != Some(parent)
        || entity.collision.state_flags_at_0x08.masked(0x1000) != RetailRuntimeValue::Known(0x1000)
        || !manager.iter_all().any(|entity| entity.id == parent
            && matches!(&entity.sub_j_attachment_runtime, RetailRuntimeValue::Known(Some(runtime))
                if runtime.ordered_entity_ids().contains(&cargo)))
    {
        return Err(Class0ActorError::Runtime("class0 release relation"));
    }
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .unwrap()
        .clone();
    let position_raw = match position {
        Type9CargoReleasePosition::Retained => entity.position_raw(),
        Type9CargoReleasePosition::Materialiser(raw) => raw,
    };
    Ok(Class0ReleasePlan {
        cargo,
        owner,
        metadata,
        position,
        body_basis: terrain_aligned_actor_basis(terrain, position_raw),
    })
}

pub(crate) fn commit_release(
    manager: &mut EntityManager,
    plan: Class0ReleasePlan,
    world_fx: &mut WorldFx,
) {
    assert_eq!(Class0ActorOwner::adopt(manager, plan.cargo), Ok(plan.owner));
    if let Type9CargoReleasePosition::Materialiser(raw) = plan.position {
        // 409030 copies the proxy's current XYZ into both child+96 and+90,
        // then enables master motion before entering16750.
        manager
            .entity_mut(plan.cargo)
            .unwrap()
            .set_position_raw(raw);
        manager
            .native_class0_actors
            .get_mut(&plan.cargo)
            .unwrap()
            .anchor_raw = raw;
        manager
            .entity_mut(plan.cargo)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x40000, 0x40000);
    }
    let entity = manager.entity_mut(plan.cargo).unwrap();
    let defaults = plan
        .metadata
        .initializer
        .as_ref()
        .unwrap()
        .initializer_state_flags_raw;
    entity.collision.state_flags_at_0x08 =
        relation_release_state_word_after(entity.collision.state_flags_at_0x08, defaults);
    entity.collision.default_state_flags_at_0xc8 = RetailRuntimeValue::Known(defaults);
    entity.attached_to = None;
    // 16AC0 changes the physical columns and sets28, preserving Euler words
    // and position. D1C0 then takes AC60's living singleton selector once.
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(plan.body_basis);
    entity.collision.state_flags_at_0x08.overwrite(0x28, 0x28);
    reselect_class0_actor(entity, &plan.metadata, &mut || {
        u32::from(world_fx.next_shared_retail_random_u16())
    })
    .expect("preflighted synchronous class0 release");
}
