//! Type83 cargo: 16700's style+08 C690 and 16750's style+0C D1C0 both reach
//! AC60 -> C560. Attachment replaces a deployed Primary, so its 4055C0 removes
//! the footprint at the pickup position; a carried radar's K1 stays zero.

use super::actor::{
    authenticate_metadata, reselect, PortableRadarError, PortableRadarOwner, ENTITY_TYPE,
};
use crate::{
    common_mover::type9_attitude::Type9BodyBasis,
    entity::EntityManager,
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, REMOTE_OWNED_STATE_BIT,
    },
    entity_relation_release::relation_release_state_word_after,
    gameplay_radar::{RadarCoverageChange, TerrainRadarError},
    ordinary_type9_cargo::Type9CargoReleasePosition,
    resource_cache::RadarCoverage,
    terrain_crater::terrain_aligned_actor_basis,
    world_fx::WorldFx,
};
use v2k_formats::terrain::TerrainGrid;

pub(crate) struct PortableRadarAttachPlan {
    pub cargo: u32,
    pub parent: u32,
    owner: PortableRadarOwner,
    metadata: EntityTypeRuntimeMetadata,
}

pub(crate) struct PortableRadarReleasePlan {
    pub cargo: u32,
    owner: PortableRadarOwner,
    metadata: EntityTypeRuntimeMetadata,
    position: Type9CargoReleasePosition,
    body_basis: Type9BodyBasis,
}

/// No raster lent: only a retiring Primary whose K1 is not saturated.
pub(crate) struct NoRadarCoverage;

impl RadarCoverage for NoRadarCoverage {
    fn mutate_radar_coverage(
        &mut self,
        _: [i16; 2],
        _: RadarCoverageChange,
        _: &mut impl FnMut() -> u16,
    ) -> Result<usize, TerrainRadarError> {
        Err(TerrainRadarError::Uninitialized)
    }
}

fn relation_owner(
    manager: &EntityManager,
    cargo: u32,
    parent: u32,
) -> Result<(PortableRadarOwner, EntityTypeRuntimeMetadata), PortableRadarError> {
    let owner = PortableRadarOwner::adopt(manager, cargo)?;
    let metadata = manager
        .type_runtime_metadata(ENTITY_TYPE)
        .cloned()
        .ok_or(PortableRadarError::Metadata)?;
    authenticate_metadata(&metadata)?;
    let local = |id| {
        manager.iter_all().any(|entity| {
            entity.id == id
                && entity.active
                && entity
                    .collision
                    .state_flags_at_0x08
                    .masked(REMOTE_OWNED_STATE_BIT)
                    == RetailRuntimeValue::Known(0)
        })
    };
    if !local(cargo)
        || !local(parent)
        || !manager.iter_all().any(|entity| {
            entity.id == parent
                && matches!(
                    entity.sub_j_attachment_runtime,
                    RetailRuntimeValue::Known(Some(_))
                )
        })
    {
        return Err(PortableRadarError::Runtime("local radar relation"));
    }
    Ok((owner, metadata))
}

/// `raster_resident`: the caller lends coverage a deployed radar can mutate.
pub(crate) fn prepare_attach(
    manager: &EntityManager,
    cargo: u32,
    parent: u32,
    raster_resident: bool,
) -> Result<PortableRadarAttachPlan, PortableRadarError> {
    let (owner, metadata) = relation_owner(manager, cargo, parent)?;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == cargo)
        .unwrap();
    if entity.attached_to.is_some()
        || entity.collision.state_flags_at_0x08.masked(0x1000) != RetailRuntimeValue::Known(0)
    {
        return Err(PortableRadarError::Runtime("radar attach relation"));
    }
    if entity.portable_radar_runtime.unwrap().sub_k1 == u16::MAX && !raster_resident {
        return Err(PortableRadarError::Runtime(
            "deployed radar attach without its raster",
        ));
    }
    Ok(PortableRadarAttachPlan {
        cargo,
        parent,
        owner,
        metadata,
    })
}

/// After the caller's Sub-J append and 16700's 0x1000/+80 projection:
/// DBF0 -> C690 -> AC60 -> C560.
pub(crate) fn commit_attach(
    manager: &mut EntityManager,
    plan: PortableRadarAttachPlan,
    coverage: &mut impl RadarCoverage,
    world_fx: &mut WorldFx,
) {
    assert_eq!(
        PortableRadarOwner::adopt(manager, plan.cargo),
        Ok(plan.owner)
    );
    reselect(
        manager.entity_mut(plan.cargo).unwrap(),
        &plan.metadata,
        coverage,
        world_fx,
    )
    .expect("preflighted synchronous radar attach");
}

pub(crate) fn prepare_release(
    manager: &EntityManager,
    cargo: u32,
    parent: u32,
    position: Type9CargoReleasePosition,
    terrain: &TerrainGrid,
) -> Result<PortableRadarReleasePlan, PortableRadarError> {
    let (owner, metadata) = relation_owner(manager, cargo, parent)?;
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
        return Err(PortableRadarError::Runtime("radar release relation"));
    }
    // 405610 zeroes K1 on every carried visit and C690 retired the old one.
    if entity.portable_radar_runtime.unwrap().sub_k1 == u16::MAX {
        return Err(PortableRadarError::Runtime("carried radar deployed"));
    }
    let position_raw = match position {
        Type9CargoReleasePosition::Retained => entity.position_raw(),
        Type9CargoReleasePosition::Materialiser(raw) => raw,
    };
    Ok(PortableRadarReleasePlan {
        cargo,
        owner,
        metadata,
        position,
        body_basis: terrain_aligned_actor_basis(terrain, position_raw),
    })
}

pub(crate) fn commit_release(
    manager: &mut EntityManager,
    plan: PortableRadarReleasePlan,
    world_fx: &mut WorldFx,
) {
    assert_eq!(
        PortableRadarOwner::adopt(manager, plan.cargo),
        Ok(plan.owner)
    );
    let entity = manager.entity_mut(plan.cargo).unwrap();
    if let Type9CargoReleasePosition::Materialiser(raw) = plan.position {
        // 409030 copies the proxy's XYZ into +96 and +90, then enables
        // master motion before entering 16750.
        entity.set_position_raw(raw);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x40000, 0x40000);
    }
    let defaults = super::actor::DEFAULT_FLAGS;
    entity.collision.state_flags_at_0x08 =
        relation_release_state_word_after(entity.collision.state_flags_at_0x08, defaults);
    entity.collision.default_state_flags_at_0xc8 = RetailRuntimeValue::Known(defaults);
    entity.attached_to = None;
    // D1C0: 16AC0 writes the terrain basis and 0x28, never position or
    // Euler; AC60 -> C560 then zeroes Euler without rebuilding that basis.
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(plan.body_basis);
    entity.collision.state_flags_at_0x08.overwrite(0x28, 0x28);
    reselect(entity, &plan.metadata, &mut NoRadarCoverage, world_fx)
        .expect("preflighted synchronous radar release");
}
