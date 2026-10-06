//! Static `4337E0` burn transitions and `427760`'s synchronous crater callback.
//!
//! `426FC5 -> 4339E0` registers the callback on the `4DC628` change list.
//! A repeated burn does not dispatch it. Kind27's authored +0C payload owns
//! the second village crater; no cinematic time or actor identity selects it.

use crate::{
    entity::EntityManager,
    resource_cache::ResourceCache,
    static_damage::StaticDamageScheduler,
    static_kind_catalog::static_kind_descriptor,
    terrain_crater::{
        apply_world_terrain_crater, crater_material_for_world_style, TerrainCraterRequest,
    },
    world_fx::{ExplodeWithRingBurstRequest, WorldFx},
};
use v2k_formats::{
    anim_frames::TerrainObjectTable, models::CollisionModelPool, terrain::TerrainGrid,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StaticTerrainBurnOutcome {
    pub burn_changed: bool,
    pub crater_applied: bool,
    pub burst_emitted: bool,
    pub terrain_changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticTerrainBurnFailure {
    pub phase: &'static str,
    pub outcome: StaticTerrainBurnOutcome,
}

/// Constructor-time null-program burns use a private terrain allocation while
/// global models remain borrowed. A crater needs the mutable world owner and
/// is rejected explicitly; the common scatter suffix is shared with live burn.
pub(crate) fn apply_constructor_static_burn<P: CollisionModelPool + ?Sized>(
    cell: [u8; 2],
    terrain: &mut TerrainGrid,
    objects: &TerrainObjectTable,
    models: &P,
    world_fx: &mut WorldFx,
) -> Result<(), &'static str> {
    apply_borrowed_static_burn(
        cell,
        terrain,
        objects,
        |model| {
            models
                .collision_model(usize::from(model))
                .map(|model| model.radius)
        },
        world_fx,
    )
}

/// The same null-program listener while the crater owns the mutable terrain.
/// Model extents are intrinsic asset words and may be borrowed independently
/// of terrain; no sampled position or level-specific rule enters this path.
pub(crate) fn apply_borrowed_static_burn(
    cell: [u8; 2],
    terrain: &mut TerrainGrid,
    objects: &TerrainObjectTable,
    model_extent: impl Fn(u16) -> Option<u16>,
    world_fx: &mut WorldFx,
) -> Result<(), &'static str> {
    let current = *terrain
        .cell(cell[0] as usize, cell[1] as usize)
        .ok_or("burn terrain")?;
    if current.terrain_type & 8 != 0 {
        return Ok(());
    }
    if current.attribute == 0 {
        terrain.cells[cell[0] as usize * 256 + cell[1] as usize].terrain_type |= 8;
        return Ok(());
    }
    let object = objects
        .records
        .get(current.attribute as usize)
        .ok_or("burn static descriptor")?;
    let descriptor = static_kind_descriptor(object.kind_index).ok_or("burn static kind")?;
    if descriptor.burn_crater().is_some() {
        return Err("constructor crater needs mutable world owner");
    }
    terrain.cells[cell[0] as usize * 256 + cell[1] as usize].terrain_type |= 8;
    if let Some(burst) = descriptor.burn_burst() {
        let extent = model_extent(object.model_id_for(current.terrain_type | 8))
            .ok_or("burn burst model")?;
        emit_static_burn_burst(
            world_fx,
            burn_position_raw(cell, current.height),
            extent,
            Some(terrain.sea_level_raw()),
            burst,
        );
    }
    Ok(())
}

fn burn_position_raw(cell: [u8; 2], height: u8) -> [i16; 3] {
    [
        (u16::from(cell[0]) << 8).wrapping_add(0x80) as i16,
        i16::from(height as i8) * 32,
        (u16::from(cell[1]) << 8).wrapping_add(0x80) as i16,
    ]
}

fn emit_static_burn_burst(
    world_fx: &mut WorldFx,
    position_raw: [i16; 3],
    source_extent_raw: u16,
    sea_level_raw: Option<i16>,
    burst: crate::static_kind_catalog::StaticBurnBurst,
) {
    world_fx.emit_explode_with_ring_burst_raw(ExplodeWithRingBurstRequest {
        position_raw,
        source_extent_raw,
        sea_level_raw,
        logical_owner_entity_id: 0,
        logical_owner_entity_type: 0,
        scatter_count: i32::from(burst.scatter_count),
        scatter_classes: [burst.particle_class; 2],
        suppresses_impact_damage: false,
    });
}

/// `27950`'s null-program branch still enters `4337E0 -> 427760`.
/// These descriptors have no crater, so particle/contact/radial callers need
/// no entity-manager borrow. The changed bit, burned model selection, cached
/// cell centre and scatter/sound suffix are the same as a timed opcode10.
pub fn apply_immediate_static_burn(
    cell: [u8; 2],
    resources: &mut ResourceCache,
    world_fx: &mut WorldFx,
) -> Result<StaticTerrainBurnOutcome, StaticTerrainBurnFailure> {
    let mut outcome = StaticTerrainBurnOutcome::default();
    let failure = |phase, outcome| StaticTerrainBurnFailure { phase, outcome };
    let current = *resources
        .level_terrain()
        .and_then(|terrain| terrain.cell(usize::from(cell[0]), usize::from(cell[1])))
        .ok_or_else(|| failure("burn terrain", outcome))?;
    if current.terrain_type & 8 != 0 {
        return Ok(outcome);
    }
    resources.or_level_terrain_type_bits(cell, 8);
    outcome.burn_changed = true;
    outcome.terrain_changed = true;
    if current.attribute == 0 {
        return Ok(outcome);
    }
    let object = resources
        .terrain_objects()
        .and_then(|objects| objects.records.get(usize::from(current.attribute)))
        .ok_or_else(|| failure("burn static descriptor", outcome))?;
    let descriptor = static_kind_descriptor(object.kind_index)
        .ok_or_else(|| failure("burn static kind", outcome))?;
    if descriptor.burn_crater().is_some() {
        return Err(failure("immediate burn cannot own a crater", outcome));
    }
    if let Some(burst) = descriptor.burn_burst() {
        let source_extent_raw = resources
            .global_model(usize::from(object.model_id_for(current.terrain_type | 8)))
            .ok_or_else(|| failure("burn burst model", outcome))?
            .radius;
        emit_static_burn_burst(
            world_fx,
            burn_position_raw(cell, current.height),
            source_extent_raw,
            resources
                .level_terrain()
                .map(|terrain| terrain.sea_level_raw()),
            burst,
        );
        outcome.burst_emitted = true;
    }
    Ok(outcome)
}

/// Apply one opcode10 burn while its source node remains registered. Crater
/// child programs use the same scheduler and cannot restart the in-flight cell.
/// The caller must apply this before the next action in that source program.
/// `4DC9B4` suppresses presentation only inside serialized-terrain application
/// (`437910`/`437B60`); ordinary scheduler callbacks run outside that scope.
pub fn apply_static_terrain_burn(
    cell: [u8; 2],
    resources: &mut ResourceCache,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    scheduler: &mut StaticDamageScheduler,
) -> Result<StaticTerrainBurnOutcome, StaticTerrainBurnFailure> {
    let mut outcome = StaticTerrainBurnOutcome::default();
    let failure = |phase, outcome| StaticTerrainBurnFailure { phase, outcome };
    let current = *resources
        .level_terrain()
        .and_then(|terrain| terrain.cell(usize::from(cell[0]), usize::from(cell[1])))
        .ok_or_else(|| failure("burn terrain", outcome))?;
    if current.terrain_type & 0x08 != 0 {
        return Ok(outcome);
    }
    // 4337E0 commits the bit before invoking any subscriber.
    resources.or_level_terrain_type_bits(cell, 0x08);
    outcome.burn_changed = true;
    outcome.terrain_changed = true;
    if current.attribute == 0 {
        return Ok(outcome);
    }
    let static_object = resources
        .terrain_objects()
        .and_then(|objects| objects.records.get(usize::from(current.attribute)))
        .ok_or_else(|| failure("burn static descriptor", outcome))?;
    let kind = static_object.kind_index;
    let burned_model_id = static_object.model_id_for(current.terrain_type | 0x08);
    let descriptor =
        static_kind_descriptor(kind).ok_or_else(|| failure("burn static kind", outcome))?;
    // 427845..42787A aligns BOTH coordinates to cell centre. This differs
    // from the timed program's base, whose Z retains the north cell edge.
    let position_raw = burn_position_raw(cell, current.height);
    if let Some(crater) = descriptor.burn_crater() {
        let world_style = resources
            .level_desc()
            .ok_or_else(|| failure("crater world", outcome))?
            .world_style;
        apply_world_terrain_crater(
            manager,
            resources,
            world_fx,
            scheduler,
            TerrainCraterRequest {
                position_raw,
                radius_raw: crater.radius_raw,
                depth_height_units: crater.depth_height_units,
                replacement_material: crater_material_for_world_style(world_style),
            },
        )
        .map_err(|error| failure(error.phase, outcome))?;
        outcome.crater_applied = true;
    }
    if let Some(burst) = descriptor.burn_burst() {
        let source_extent_raw = resources
            .global_model(usize::from(burned_model_id))
            .ok_or_else(|| failure("burn burst model", outcome))?
            .radius;
        // 42789C follows the crater, retaining its cached pre-crater origin
        // and burned model slot. DAT4DCA00 is the known zero owner; A60's
        // failed lookup gives the known zero source-type byte as well.
        emit_static_burn_burst(
            world_fx,
            position_raw,
            source_extent_raw,
            resources
                .level_terrain()
                .map(|terrain| terrain.sea_level_raw()),
            burst,
        );
        outcome.burst_emitted = true;
    }
    Ok(outcome)
}

#[cfg(test)]
#[path = "static_terrain_burn_tests.rs"]
mod tests;
