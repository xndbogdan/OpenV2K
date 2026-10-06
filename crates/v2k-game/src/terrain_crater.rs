//! `36E00` signed-height crater and `EB20 -> 16AC0` grounded actor refresh.
//!
//! The caller performs the returned `4A890` radar refreshes after all terrain
//! writes and before refreshing actors. Renderer mesh/normal invalidation is
//! a separate port cache concern, reported even when a later phase blocks.

use crate::{
    common_mover::type9_attitude::Type9BodyBasis,
    entity::EntityManager,
    entity_collision_state::RetailRuntimeValue,
    ordinary_type47_shot_math::normalize_retail_vector_q31,
    resource_cache::ResourceCache,
    static_damage::{StaticCraterOutcome, StaticDamageScheduler},
    world_fx::WorldFx,
};
use v2k_formats::{
    anim_frames::TerrainObjectTable,
    fixed_math::retail_integer_sqrt,
    terrain::{TerrainGrid, GRID_SIZE},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TerrainCraterRequest {
    pub position_raw: [i16; 3],
    pub radius_raw: i32,
    /// Signed height-byte units, not raw world Y (one unit is 32 raw Y).
    pub depth_height_units: i32,
    /// `2EC30`: world style4 ->1, style5 ->none, every other style ->4.
    /// `36E00` caller param4==0 also selects None.
    pub replacement_material: Option<u8>,
}

pub(crate) const fn crater_material_for_world_style(world_style: u32) -> Option<u8> {
    match world_style {
        4 => Some(1),
        5 => None,
        _ => Some(4),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TerrainCraterReport {
    pub terrain_changed: bool,
    pub visited_cells: usize,
    pub started_static_cells: Vec<[u8; 2]>,
    /// Exact X-major second-pass raw points, retaining fractional origin.
    pub radar_refresh_positions_raw: Vec<[i16; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TerrainCraterBlock {
    InvalidRequest,
    MissingTerrain,
    MissingStaticDescriptor(u8),
    UnsupportedStaticKind(u32),
    ActorRuntime { entity_id: u32, field: &'static str },
    MissingModel(usize),
    BurnCallback(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TerrainCraterFailure {
    pub block: TerrainCraterBlock,
    pub report: TerrainCraterReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorldTerrainCraterFailure {
    pub phase: &'static str,
    pub terrain_changed: bool,
}

/// Complete `436E00`: terrain/static insertion, radar RNG, then grounded actors.
/// Both Sub-M death and static burn callbacks run this synchronously.
pub(crate) fn apply_world_terrain_crater(
    manager: &mut EntityManager,
    resources: &mut ResourceCache,
    world_fx: &mut WorldFx,
    static_damage: &mut StaticDamageScheduler,
    request: TerrainCraterRequest,
) -> Result<TerrainCraterReport, WorldTerrainCraterFailure> {
    let missing = |phase| WorldTerrainCraterFailure {
        phase,
        terrain_changed: false,
    };
    let objects = TerrainObjectTable {
        records: resources
            .terrain_objects()
            .ok_or_else(|| missing("crater objects"))?
            .records
            .clone(),
    };
    // Only model-header extents cross this borrow boundary. Each listener
    // still reads its current cell after that cell's crater height write.
    let extents: std::collections::BTreeMap<_, _> = objects
        .records
        .iter()
        .flat_map(|record| [record.model_id_for(8), record.model_id_for(24)])
        .filter_map(|id| {
            resources
                .global_model(usize::from(id))
                .map(|model| (id, model.radius))
        })
        .collect();
    let report = apply_terrain_crater_with_burn_listener(
        resources
            .level_terrain_mut()
            .ok_or_else(|| missing("crater terrain"))?,
        &objects,
        static_damage,
        request,
        |terrain, cell| {
            crate::static_terrain_burn::apply_borrowed_static_burn(
                cell,
                terrain,
                &objects,
                |id| extents.get(&id).copied(),
                world_fx,
            )
        },
    )
    .map_err(|failure| WorldTerrainCraterFailure {
        phase: "crater terrain/static program",
        terrain_changed: failure.report.terrain_changed,
    })?;
    let failed = |phase| WorldTerrainCraterFailure {
        phase,
        terrain_changed: report.terrain_changed,
    };
    resources
        .refresh_level_terrain_radar(&report.radar_refresh_positions_raw, &mut || {
            world_fx.next_shared_retail_random_u16()
        })
        .map_err(|_| failed("crater radar refresh"))?;
    refresh_crater_grounded_actors(manager, resources, request)
        .map_err(|_| failed("crater grounded actor refresh"))?;
    Ok(report)
}

/// Terrain writes and synchronous static-program insertion in retail order.
fn apply_terrain_crater_with_burn_listener(
    terrain: &mut TerrainGrid,
    objects: &TerrainObjectTable,
    static_damage: &mut StaticDamageScheduler,
    request: TerrainCraterRequest,
    mut burn_cell: impl FnMut(&mut TerrainGrid, [u8; 2]) -> Result<(), &'static str>,
) -> Result<TerrainCraterReport, TerrainCraterFailure> {
    let mut report = TerrainCraterReport::default();
    let radius = request.radius_raw;
    let depth = request.depth_height_units;
    // These bounds preserve the source's signed products/divisors and one
    // visit per wrapped cell. The admitted hut uses radius1280/depth16.
    if !(384..=32767).contains(&radius)
        || !(-32767..=32767).contains(&depth)
        || request.replacement_material.is_some_and(|value| value > 7)
        || terrain.cells.len() != GRID_SIZE * GRID_SIZE
    {
        return Err(TerrainCraterFailure {
            block: TerrainCraterBlock::InvalidRequest,
            report,
        });
    }
    let inner = (radius * 4) / 6;
    let inner_squared = inner * inner >> 16;
    let outer_squared = radius * radius >> 16;
    let static_radius = radius * 5 / 6;
    let static_squared = static_radius * static_radius >> 16;
    let [x, _, z] = request.position_raw;
    let cell_at = |dx: i32, dz: i32| {
        [
            (x.wrapping_add(dx as i16) as u16 >> 8) as u8,
            (z.wrapping_add(dz as i16) as u16 >> 8) as u8,
        ]
    };
    let center = cell_at(0, 0);
    let center_height = terrain
        .cell(center[0] as usize, center[1] as usize)
        .unwrap()
        .height as i8 as i32;
    for dx in (-radius..=radius).step_by(256) {
        for dz in (-radius..=radius).step_by(256) {
            let distance = (dx * dx >> 16) + (dz * dz >> 16);
            if distance >= outer_squared {
                continue;
            }
            let cell = cell_at(dx, dz);
            let index = cell[0] as usize * GRID_SIZE + cell[1] as usize;
            let depression = depth * (inner_squared - distance) / inner_squared + depth / 4;
            let previous = terrain.cells[index];
            let old_height = previous.height as i8 as i32;
            let new_height = if depression < 1 {
                old_height
                    - depression * (inner_squared - distance) / (outer_squared - inner_squared)
                    - depression
            } else if old_height >= center_height {
                old_height - depression
            } else {
                old_height.min(center_height - depression)
            }
            .clamp(-127, 127);
            terrain.cells[index].height = new_height as i8 as u8;
            terrain.cells[index].terrain_type &= !0x10;
            report.visited_cells += 1;
            report.terrain_changed |= previous.height != terrain.cells[index].height
                || previous.terrain_type != terrain.cells[index].terrain_type;
            if distance < static_squared && new_height < center_height + depth {
                let attribute = terrain.cells[index].attribute;
                if attribute != 0 {
                    let Some(descriptor) = objects.records.get(attribute as usize) else {
                        return Err(TerrainCraterFailure {
                            block: TerrainCraterBlock::MissingStaticDescriptor(attribute),
                            report,
                        });
                    };
                    match static_damage.submit_crater_destruction(cell, descriptor.kind_index) {
                        StaticCraterOutcome::BurnCell => {
                            let old = terrain.cells[index].terrain_type;
                            if let Err(reason) = burn_cell(terrain, cell) {
                                report.terrain_changed |= old != terrain.cells[index].terrain_type;
                                return Err(TerrainCraterFailure {
                                    block: TerrainCraterBlock::BurnCallback(reason),
                                    report,
                                });
                            }
                            report.terrain_changed |= old != terrain.cells[index].terrain_type;
                        }
                        StaticCraterOutcome::Started => report.started_static_cells.push(cell),
                        StaticCraterOutcome::Duplicate => {}
                        StaticCraterOutcome::UnsupportedKind { kind_index } => {
                            return Err(TerrainCraterFailure {
                                block: TerrainCraterBlock::UnsupportedStaticKind(kind_index),
                                report,
                            });
                        }
                    }
                }
                if let Some(material) = request.replacement_material {
                    let old = terrain.cells[index].terrain_type;
                    let light = ((i32::from(old >> 5) * distance / outer_squared) & 7) as u8;
                    terrain.cells[index].terrain_type = (old & 0x18) | (light << 5) | material;
                    report.terrain_changed |= old != terrain.cells[index].terrain_type;
                }
            }
        }
    }
    for dx in (-radius..=radius).step_by(256) {
        for dz in (-radius..=radius).step_by(256) {
            if (dx * dx >> 16) + (dz * dz >> 16) < outer_squared {
                report
                    .radar_refresh_positions_raw
                    .push([x.wrapping_add(dx as i16), z.wrapping_add(dz as i16)]);
            }
        }
    }
    Ok(report)
}

/// Geometry-only fixtures isolate the height oracle from subscriber effects.
#[cfg(test)]
pub(crate) fn apply_terrain_crater(
    terrain: &mut TerrainGrid,
    objects: &TerrainObjectTable,
    static_damage: &mut StaticDamageScheduler,
    request: TerrainCraterRequest,
) -> Result<TerrainCraterReport, TerrainCraterFailure> {
    apply_terrain_crater_with_burn_listener(
        terrain,
        objects,
        static_damage,
        request,
        |terrain, cell| {
            terrain.cells[usize::from(cell[0]) * GRID_SIZE + usize::from(cell[1])].terrain_type |=
                8;
            Ok(())
        },
    )
}

/// `16AC0` uses forward terrain differences, not Euler rotation or a central
/// gradient. It normalizes Q12 tangents, crosses them, expands then normalizes
/// each Q31 column through the existing exact57960 implementation.
pub(crate) fn terrain_aligned_actor_basis(
    terrain: &TerrainGrid,
    position_raw: [i16; 3],
) -> Type9BodyBasis {
    let [x, _, z] = position_raw;
    let base = terrain.bilinear_height_raw(x, z);
    let dz = terrain
        .bilinear_height_raw(x, z.wrapping_add(256))
        .wrapping_sub(base);
    let dx = terrain
        .bilinear_height_raw(x.wrapping_add(256), z)
        .wrapping_sub(base);
    let tangent = |delta: i16, along_x: bool| {
        let length = retail_integer_sqrt(i32::from(delta) * i32::from(delta) + 0x10000) as i16;
        let vector = if along_x {
            [256, i32::from(delta), 0]
        } else {
            [0, i32::from(delta), 256]
        };
        vector.map(|component| ((component << 12) / i32::from(length)) as i16)
    };
    let forward = tangent(dz, false);
    let lateral = tangent(dx, true);
    let cross = |a: [i16; 3], b: [i16; 3]| {
        std::array::from_fn(|axis| {
            let p = (axis + 1) % 3;
            let q = (axis + 2) % 3;
            ((i32::from(a[p]) * i32::from(b[q]) - i32::from(a[q]) * i32::from(b[p])) >> 12) as i16
        })
    };
    let expand = |vector: [i16; 3]| {
        normalize_retail_vector_q31(vector.map(|component| match component {
            4096.. => i32::MAX,
            ..=-4096 => -i32::MAX,
            _ => i32::from(component) << 19,
        }))
    };
    Type9BodyBasis {
        lateral: expand(lateral),
        up: expand(cross(forward, lateral)),
        forward: expand(forward),
    }
}

/// `EB20` runs after all crater/radar cells. Later failures keep earlier actor
/// refreshes committed, and unrelated unresolved state bits are never read.
pub(crate) fn refresh_crater_grounded_actors(
    manager: &mut EntityManager,
    resources: &ResourceCache,
    request: TerrainCraterRequest,
) -> Result<Vec<u32>, TerrainCraterBlock> {
    let terrain = resources
        .level_terrain()
        .ok_or(TerrainCraterBlock::MissingTerrain)?;
    let ids: Vec<_> = manager.retail_live_order_ids().collect();
    let mut refreshed = Vec::new();
    for id in ids {
        let entity = manager.entity_mut(id).unwrap();
        let state = entity.collision.state_flags_at_0x08;
        match state.masked(0x08009000) {
            RetailRuntimeValue::Known(0x08008000) => {}
            RetailRuntimeValue::Known(_) => continue,
            RetailRuntimeValue::Unresolved => {
                return Err(TerrainCraterBlock::ActorRuntime {
                    entity_id: id,
                    field: "fixed/active/hidden state",
                })
            }
        }
        let [x, _, z] = entity.position_raw();
        let dx = i32::from(x.wrapping_sub(request.position_raw[0]));
        let dz = i32::from(z.wrapping_sub(request.position_raw[2]));
        if (dx * dx).wrapping_add(dz * dz) as u32
            >= (request.radius_raw * request.radius_raw) as u32
        {
            continue;
        }
        let RetailRuntimeValue::Known(defaults) = entity.collision.default_state_flags_at_0xc8
        else {
            return Err(TerrainCraterBlock::ActorRuntime {
                entity_id: id,
                field: "default flags",
            });
        };
        if defaults & 0x20 == 0 {
            continue;
        }
        let mut y = terrain.bilinear_height_raw(x, z);
        // Y is committed before the optional selected model-header read.
        entity.set_position_raw([x, y, z]);
        if defaults & 0x40 != 0 {
            let RetailRuntimeValue::Known(model_state) = state.masked(0x6000) else {
                return Err(TerrainCraterBlock::ActorRuntime {
                    entity_id: id,
                    field: "active model state",
                });
            };
            let slot = ((model_state & 0x2000) >> 12 | (model_state & 0x4000) >> 14) as usize;
            let model_id = entity
                .model_in_slot(slot)
                .ok_or(TerrainCraterBlock::ActorRuntime {
                    entity_id: id,
                    field: "selected model",
                })?;
            let model = resources
                .global_model(model_id)
                .ok_or(TerrainCraterBlock::MissingModel(model_id))?;
            y = y.wrapping_add(model.radius as i16);
            entity.set_position_raw([x, y, z]);
        }
        entity.physical_body_basis_q31 =
            RetailRuntimeValue::Known(terrain_aligned_actor_basis(terrain, entity.position_raw()));
        entity.collision.state_flags_at_0x08.overwrite(0x28, 0x28);
        refreshed.push(id);
    }
    Ok(refreshed)
}

#[cfg(test)]
#[path = "terrain_crater_tests.rs"]
mod tests;
