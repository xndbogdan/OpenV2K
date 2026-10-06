//! Live Section-10/Section-9/model resolution for static-object damage.
//!
//! Static damage must reacquire the current terrain cell before every hit:
//! terrain mutations can change both the descriptor attribute and the model
//! slot selected by terrain-type bits 3..4. Unlike the general layered cache
//! accessors, this lookup deliberately requires the current gameplay level's
//! mutable Section-10 allocation.

use crate::resource_cache::ResourceCache;
use crate::static_damage::{StaticDamageTarget, StaticDamageTargetState};
use v2k_formats::anim_frames::TerrainObjectTable;
use v2k_formats::models::CollisionModelPool;
use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

/// Current authored inputs consumed by the static-object damage callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentStaticDamageSnapshot {
    pub target: StaticDamageTarget,
    pub attribute: u8,
    pub model_id: u16,
}

/// A non-empty live Section-10 cell could not be resolved faithfully.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentStaticDamageLookupError {
    BurnCallback(crate::static_terrain_burn::StaticTerrainBurnFailure),
    MissingCurrentLevelTerrain,
    IncompleteCurrentLevelTerrain {
        cell: [u8; 2],
        required_index: usize,
        cell_count: usize,
    },
    MissingTerrainObjectTable,
    MissingTerrainObjectDescriptor {
        attribute: u8,
        record_count: usize,
    },
    MissingGlobalModel {
        model_id: u16,
    },
}

/// Resolve the live Section-10 cell, its Section-9 descriptor, and its
/// currently selected global Section-8 model.
///
/// `Ok(None)` is reserved for an authored empty cell (`attribute == 0`). Any
/// unavailable input for a non-empty cell is reported explicitly so callers
/// that need atomic preflight can distinguish missing evidence from absence.
pub fn resolve_current_static_damage_snapshot(
    cache: &ResourceCache,
    cell: [u8; 2],
) -> Result<Option<CurrentStaticDamageSnapshot>, CurrentStaticDamageLookupError> {
    let terrain = cache
        .level()
        .and_then(|level| level.terrain.as_ref())
        .ok_or(CurrentStaticDamageLookupError::MissingCurrentLevelTerrain)?;
    let required_index = usize::from(cell[0]) * GRID_SIZE + usize::from(cell[1]);
    let terrain_cell = terrain.cells.get(required_index).copied().ok_or(
        CurrentStaticDamageLookupError::IncompleteCurrentLevelTerrain {
            cell,
            required_index,
            cell_count: terrain.cells.len(),
        },
    )?;
    if terrain_cell.attribute == 0 {
        return Ok(None);
    }
    let table = cache
        .terrain_objects()
        .ok_or(CurrentStaticDamageLookupError::MissingTerrainObjectTable)?;
    resolve_static_damage_snapshot_from_cell(cell, terrain_cell, table, cache)
}

/// Resolve one already-selected Section-10 cell through the active Section-9
/// table and global model pool.
///
/// Particle collision keeps synchronous Section-10 writes in a compact
/// overlay while the gameplay cache is immutably borrowed. Passing the
/// effective cell through this shared resolver lets `FUN_0043F800` re-read the
/// current descriptor/model after its F610 visual without reintroducing a
/// second, subtly different target decoder.
pub(crate) fn resolve_static_damage_snapshot_from_cell<P: CollisionModelPool + ?Sized>(
    cell: [u8; 2],
    terrain_cell: TerrainCell,
    table: &TerrainObjectTable,
    model_pool: &P,
) -> Result<Option<CurrentStaticDamageSnapshot>, CurrentStaticDamageLookupError> {
    if terrain_cell.attribute == 0 {
        return Ok(None);
    }

    let descriptor = table
        .records
        .get(usize::from(terrain_cell.attribute))
        .ok_or(
            CurrentStaticDamageLookupError::MissingTerrainObjectDescriptor {
                attribute: terrain_cell.attribute,
                record_count: table.records.len(),
            },
        )?;
    let model_id = descriptor.model_id_for(terrain_cell.terrain_type);
    let model = model_pool
        .collision_model(usize::from(model_id))
        .ok_or(CurrentStaticDamageLookupError::MissingGlobalModel { model_id })?;

    Ok(Some(CurrentStaticDamageSnapshot {
        target: StaticDamageTarget {
            cell,
            state: StaticDamageTargetState {
                kind_index: descriptor.kind_index,
                terrain_type: terrain_cell.terrain_type,
                terrain_height_byte: terrain_cell.height,
                collision_radius_raw: model.collision_radius_raw,
                effect_extent_raw: model.radius,
            },
        },
        attribute: terrain_cell.attribute,
        model_id,
    }))
}

/// Resolve only the live target consumed by ordinary static-damage callers.
pub fn resolve_current_static_damage_target(
    cache: &ResourceCache,
    cell: [u8; 2],
) -> Result<Option<StaticDamageTarget>, CurrentStaticDamageLookupError> {
    resolve_current_static_damage_snapshot(cache, cell)
        .map(|snapshot| snapshot.map(|snapshot| snapshot.target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::LevelState;
    use v2k_formats::anim_frames::{ModelSlotPattern, TerrainObjectDescriptor, TerrainObjectTable};
    use v2k_formats::models::{ModelCollection, ModelEntry, StreamStats};
    use v2k_formats::terrain::{TerrainCell, TerrainGrid};

    fn empty_state(source_path: &str, system_level: Option<u32>) -> LevelState {
        LevelState {
            source_path: source_path.to_owned(),
            system_level,
            fixup_data: None,
            fixup_code: None,
            strings: Vec::new(),
            sprites: None,
            params: None,
            display_modes: None,
            fog_gradient: None,
            color_palettes: None,
            models: None,
            anim_frames: None,
            terrain: None,
            anim_sound: None,
            collision: None,
            level: None,
            linkage: None,
        }
    }

    fn terrain_with_cell(cell: [u8; 2], terrain_cell: TerrainCell) -> TerrainGrid {
        let mut cells = vec![
            TerrainCell {
                height: 0,
                attribute: 0,
                terrain_type: 0,
            };
            GRID_SIZE * GRID_SIZE
        ];
        cells[usize::from(cell[0]) * GRID_SIZE + usize::from(cell[1])] = terrain_cell;
        TerrainGrid {
            header: [0; 5],
            cells,
        }
    }

    fn descriptor(model_ids: [u16; 4], kind_index: u32) -> TerrainObjectDescriptor {
        TerrainObjectDescriptor {
            model_ids,
            kind_index,
            pattern: ModelSlotPattern::Varied,
        }
    }

    fn model(index: usize, radius: u16, collision_radius_raw: u16) -> ModelEntry {
        ModelEntry {
            index,
            cmd_word_count: 0,
            extra_count: 0,
            flags: 0x40,
            slot_count: 0,
            face_val: 2,
            radius,
            collision_radius_raw,
            collision_program: Vec::new(),
            records: Vec::new(),
            normal_pool: Vec::new(),
            cmd_words: Vec::new(),
            has_view_commands: false,
            vertices: Vec::new(),
            vertex_type_flags: Vec::new(),
            vertex_projection: Vec::new(),
            vertex_clip: Vec::new(),
            vertex_surface_origin: Vec::new(),
            triangles: Vec::new(),
            face_vertices: Vec::new(),
            normals: Vec::new(),
            face_cull: Vec::new(),
            face_materials: Vec::new(),
            face_uvs: Vec::new(),
            face_corner_normals: Vec::new(),
            face_shading: Vec::new(),
            shadow_triangles: Vec::new(),
            edges: Vec::new(),
            billboards: Vec::new(),
            instances: Vec::new(),
            name: None,
        }
    }

    #[test]
    fn lookup_requires_current_level_terrain_instead_of_layer_fallback() {
        let cell = [4, 7];
        let mut preload = empty_state("preload", Some(2));
        preload.terrain = Some(terrain_with_cell(
            cell,
            TerrainCell {
                height: 1,
                attribute: 0,
                terrain_type: 2,
            },
        ));
        let mut cache = ResourceCache::new(vec![preload]);

        assert_eq!(
            resolve_current_static_damage_snapshot(&cache, cell),
            Err(CurrentStaticDamageLookupError::MissingCurrentLevelTerrain)
        );

        cache.load_level(empty_state("level-without-terrain", None));
        assert_eq!(
            resolve_current_static_damage_snapshot(&cache, cell),
            Err(CurrentStaticDamageLookupError::MissingCurrentLevelTerrain)
        );
    }

    #[test]
    fn lookup_reserves_none_for_an_authored_empty_cell() {
        let cell = [4, 7];
        let mut level = empty_state("level", None);
        level.terrain = Some(terrain_with_cell(
            cell,
            TerrainCell {
                height: 91,
                attribute: 0,
                terrain_type: 0x18,
            },
        ));
        let mut cache = ResourceCache::new(Vec::new());
        cache.load_level(level);

        assert_eq!(
            resolve_current_static_damage_snapshot(&cache, cell),
            Ok(None),
            "an empty attribute must not require Section 9 or a model"
        );
    }

    #[test]
    fn lookup_reports_each_unresolved_nonempty_input() {
        let cell = [1, 2];
        let mut level = empty_state("incomplete-level", None);
        level.terrain = Some(TerrainGrid {
            header: [0; 5],
            cells: vec![TerrainCell {
                height: 0,
                attribute: 0,
                terrain_type: 0,
            }],
        });
        let mut cache = ResourceCache::new(Vec::new());
        cache.load_level(level);
        assert_eq!(
            resolve_current_static_damage_snapshot(&cache, cell),
            Err(
                CurrentStaticDamageLookupError::IncompleteCurrentLevelTerrain {
                    cell,
                    required_index: usize::from(cell[0]) * GRID_SIZE + usize::from(cell[1]),
                    cell_count: 1,
                }
            )
        );

        let mut level = empty_state("nonempty-level", None);
        level.terrain = Some(terrain_with_cell(
            cell,
            TerrainCell {
                height: 11,
                attribute: 5,
                terrain_type: 0,
            },
        ));
        cache.load_level(level);
        assert_eq!(
            resolve_current_static_damage_snapshot(&cache, cell),
            Err(CurrentStaticDamageLookupError::MissingTerrainObjectTable)
        );

        let mut table_state = empty_state("short-section-9", None);
        table_state.anim_frames = Some(TerrainObjectTable {
            records: vec![descriptor([0; 4], 0); 5],
        });
        cache.add_auxiliary(table_state);
        assert_eq!(
            resolve_current_static_damage_snapshot(&cache, cell),
            Err(
                CurrentStaticDamageLookupError::MissingTerrainObjectDescriptor {
                    attribute: 5,
                    record_count: 5,
                }
            )
        );

        let mut table_state = empty_state("complete-section-9", None);
        table_state.anim_frames = Some(TerrainObjectTable {
            records: vec![descriptor([37; 4], 3); 256],
        });
        cache.add_auxiliary(table_state);
        assert_eq!(
            resolve_current_static_damage_snapshot(&cache, cell),
            Err(CurrentStaticDamageLookupError::MissingGlobalModel { model_id: 37 })
        );
    }

    #[test]
    fn lookup_reacquires_the_live_model_slot_and_target_shape() {
        let cell = [13, 29];
        let mut resources = empty_state("system-2", Some(2));
        resources.models = Some(ModelCollection {
            sub_blocks: Vec::new(),
            all_entries: vec![
                model(0, 10, 100),
                model(1, 20, 200),
                model(2, 30, 300),
                model(3, 40, 400),
            ],
            stats: StreamStats::default(),
        });
        resources.anim_frames = Some(TerrainObjectTable {
            records: vec![descriptor([0, 1, 2, 3], 11); 256],
        });
        let mut cache = ResourceCache::new(vec![resources]);
        let mut level = empty_state("level", None);
        level.terrain = Some(terrain_with_cell(
            cell,
            TerrainCell {
                height: 77,
                attribute: 9,
                terrain_type: 0x18,
            },
        ));
        cache.load_level(level);

        let snapshot = resolve_current_static_damage_snapshot(&cache, cell)
            .expect("all current inputs resolve")
            .expect("attribute is nonzero");
        assert_eq!(snapshot.attribute, 9);
        assert_eq!(snapshot.model_id, 3);
        assert_eq!(
            snapshot.target,
            StaticDamageTarget {
                cell,
                state: StaticDamageTargetState {
                    kind_index: 11,
                    terrain_type: 0x18,
                    terrain_height_byte: 77,
                    collision_radius_raw: 400,
                    effect_extent_raw: 40,
                },
            }
        );
        assert_eq!(
            resolve_current_static_damage_target(&cache, cell),
            Ok(Some(snapshot.target))
        );
    }
}
