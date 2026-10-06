//! Production-shaped world ownership for the Main Base abort sweep.
//!
//! The retail sweep shares one mutable Section-10 allocation, one static
//! damage scheduler, and one process RNG stream across its actors.  This
//! adapter keeps those authorities together: Type-67 cleanup publishes the
//! exact old attribute byte from the live level, while each Type-61 actor
//! preflights its static radial against that same allocation and commits its
//! hits in retail scan order through [`WorldFx`]'s shared RNG.

use v2k_formats::terrain::{TerrainGrid, GRID_SIZE};

use crate::entity::MainBaseAbortAlternateCleanupEffects;
use crate::main_base_type61_abort::MainBaseType61StaticRadialEffects;
use crate::radial_damage::RadialDamageTemplate;
use crate::resource_cache::ResourceCache;
use crate::static_damage::{
    scan_static_radial, static_damage_kind_is_admitted, StaticDamageOutcome, StaticDamageScheduler,
    StaticRadialHit,
};
use crate::static_damage_live::{
    resolve_current_static_damage_snapshot, CurrentStaticDamageLookupError,
};
use crate::world_fx::WorldFx;

const COMPLETE_TERRAIN_CELL_COUNT: usize = GRID_SIZE * GRID_SIZE;

/// Why the shared world-effects owner could not be established safely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseAbortWorldEffectsConstructorBlock {
    MissingCurrentLevelTerrain,
    CurrentLevelTerrainCellCount {
        expected: usize,
        actual: usize,
    },
    GeometrySnapshotCellCount {
        expected: usize,
        actual: usize,
    },
    GeometrySnapshotHeaderMismatch,
    GeometrySnapshotStableCellMismatch {
        cell: [u8; 2],
        current_height: u8,
        snapshot_height: u8,
        current_type_bit_10: u8,
        snapshot_type_bit_10: u8,
    },
}

/// Opaque copy of the stable Section-10 inputs used during one abort sweep.
#[derive(Debug)]
pub struct MainBaseAbortTerrainGeometrySnapshot {
    terrain: TerrainGrid,
}

impl MainBaseAbortTerrainGeometrySnapshot {
    /// Borrow the geometry for entity callbacks which read terrain height,
    /// header fields, or terrain-type bit `0x10` during the same sweep.
    pub const fn terrain(&self) -> &TerrainGrid {
        &self.terrain
    }
}

/// Copy the stable terrain inputs needed while the abort adapter owns the
/// mutable cache borrow.
///
/// Type-67 changes only object attributes and Type-61's immediate kind-10
/// branch changes an attribute plus terrain-type bit `0x08`. The later Type-60
/// constructor reads heights, the header, and terrain-type bit `0x10`, so this
/// snapshot remains authoritative for that bounded sweep. Static-object
/// presence, descriptor/model selection, and damage state must still be read
/// from the adapter's live cache.
pub fn snapshot_main_base_abort_terrain_geometry(
    cache: &ResourceCache,
) -> Result<MainBaseAbortTerrainGeometrySnapshot, MainBaseAbortWorldEffectsConstructorBlock> {
    let terrain = cache
        .level()
        .and_then(|level| level.terrain.as_ref())
        .ok_or(MainBaseAbortWorldEffectsConstructorBlock::MissingCurrentLevelTerrain)?;
    if terrain.cells.len() != COMPLETE_TERRAIN_CELL_COUNT {
        return Err(
            MainBaseAbortWorldEffectsConstructorBlock::CurrentLevelTerrainCellCount {
                expected: COMPLETE_TERRAIN_CELL_COUNT,
                actual: terrain.cells.len(),
            },
        );
    }
    Ok(MainBaseAbortTerrainGeometrySnapshot {
        terrain: TerrainGrid {
            header: terrain.header,
            cells: terrain.cells.clone(),
        },
    })
}

/// Exact old-byte publication made by one Type-67 cleanup callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseAbortTerrainAttributePublication {
    pub cell: [u8; 2],
    pub old_attribute: u8,
}

/// One Type-61 static hit and the scheduler decision it produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseType61StaticRadialOutcome {
    pub hit: StaticRadialHit,
    pub outcome: StaticDamageOutcome,
    /// True only when the outcome requested kind 10's synchronous live
    /// Section-10 transition and that transition was applied.
    pub immediate_terrain_transition_applied: bool,
}

/// First unresolved live-static condition observed in retail scan order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType61StaticRadialPreflightBlock {
    CurrentStaticDamageLookup(CurrentStaticDamageLookupError),
    UnsupportedKind {
        cell: [u8; 2],
        attribute: u8,
        kind_index: u32,
    },
}

/// Read-only plan admitted before any Type-61 death or presentation mutation.
#[derive(Debug, PartialEq, Eq)]
pub struct MainBaseType61StaticRadialPlan {
    hits: Vec<StaticRadialHit>,
}

impl MainBaseType61StaticRadialPlan {
    pub fn hits(&self) -> &[StaticRadialHit] {
        &self.hits
    }

    pub fn is_empty(&self) -> bool {
        self.hits.is_empty()
    }
}

/// Compact observable state retained by the shared effects owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseAbortWorldEffectsSummary {
    pub terrain_attribute_publication_count: usize,
    pub static_radial_outcome_count: usize,
    pub terrain_dirty: bool,
}

/// Owned results returned when the adapter releases its cache/scheduler borrows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainBaseAbortWorldEffectsReport {
    terrain_attribute_publications: Vec<MainBaseAbortTerrainAttributePublication>,
    static_radial_outcomes: Vec<MainBaseType61StaticRadialOutcome>,
    terrain_dirty: bool,
}

impl MainBaseAbortWorldEffectsReport {
    pub fn terrain_attribute_publications(&self) -> &[MainBaseAbortTerrainAttributePublication] {
        &self.terrain_attribute_publications
    }

    pub fn static_radial_outcomes(&self) -> &[MainBaseType61StaticRadialOutcome] {
        &self.static_radial_outcomes
    }

    pub const fn terrain_dirty(&self) -> bool {
        self.terrain_dirty
    }

    pub fn summary(&self) -> MainBaseAbortWorldEffectsSummary {
        MainBaseAbortWorldEffectsSummary {
            terrain_attribute_publication_count: self.terrain_attribute_publications.len(),
            static_radial_outcome_count: self.static_radial_outcomes.len(),
            terrain_dirty: self.terrain_dirty,
        }
    }
}

/// Shared mutable world state required from the first Type-67 cleanup through
/// the final Type-61 static scan.
pub struct MainBaseAbortWorldEffects<'a> {
    geometry_snapshot: &'a TerrainGrid,
    cache: &'a mut ResourceCache,
    static_damage: &'a mut StaticDamageScheduler,
    terrain_attribute_publications: Vec<MainBaseAbortTerrainAttributePublication>,
    static_radial_outcomes: Vec<MainBaseType61StaticRadialOutcome>,
    terrain_dirty: bool,
    last_static_radial_preflight_block: Option<MainBaseType61StaticRadialPreflightBlock>,
}

/// The abort transaction's actual Playing authorities for a synchronous
/// class49 callback; resource/static ownership stays in the world-effects owner.
pub(crate) struct MainBaseAbortClass49Frame<'a> {
    pub entities: &'a mut crate::entity::EntityManager,
    pub world_fx: &'a mut WorldFx,
    pub notifications: &'a mut crate::gameplay_notifications::GameplayNotifications,
    pub retail_tick: u32,
    pub scheduler: &'a mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
    pub player_hull: &'a mut crate::player_hull::PlayerHull,
    pub extra_lives: crate::entity_collision_state::RetailRuntimeValue<u8>,
}

impl<'a> MainBaseAbortWorldEffects<'a> {
    pub(crate) fn run_class49_standard_death(
        &mut self,
        id: u32,
        frame: MainBaseAbortClass49Frame<'_>,
    ) -> Result<
        crate::live_actor_checked_damage::LiveActorDeathResult<()>,
        crate::class49_terminal::Class49TerminalBlock,
    > {
        let result = crate::class49_terminal::run_class49_standard_death(
            crate::class49_terminal::Class49TerminalFrame {
                entities: frame.entities,
                resources: self.cache,
                world_fx: frame.world_fx,
                static_damage: self.static_damage,
                notifications: frame.notifications,
                retail_tick: frame.retail_tick,
                world: crate::class49_terminal::Class49WorldContext::Playing {
                    scheduler: frame.scheduler,
                    player_hull: frame.player_hull,
                    extra_lives: frame.extra_lives,
                    active_terminal_calls: Vec::new(),
                },
            },
            id,
        );
        // The nested static pass owns immediate terrain writes. Invalidate the
        // abort's presentation after a completed blast, including nested ones.
        self.terrain_dirty |= result.as_ref().is_ok_and(|result| result.returned_nonzero);
        result
    }

    /// Bind a complete, independent geometry snapshot to the authoritative
    /// mutable level allocation and the bound static scheduler.
    pub fn new(
        geometry_snapshot: &'a MainBaseAbortTerrainGeometrySnapshot,
        cache: &'a mut ResourceCache,
        static_damage: &'a mut StaticDamageScheduler,
    ) -> Result<Self, MainBaseAbortWorldEffectsConstructorBlock> {
        let current_terrain = cache
            .level()
            .and_then(|level| level.terrain.as_ref())
            .ok_or(MainBaseAbortWorldEffectsConstructorBlock::MissingCurrentLevelTerrain)?;
        let current_cell_count = current_terrain.cells.len();
        if current_cell_count != COMPLETE_TERRAIN_CELL_COUNT {
            return Err(
                MainBaseAbortWorldEffectsConstructorBlock::CurrentLevelTerrainCellCount {
                    expected: COMPLETE_TERRAIN_CELL_COUNT,
                    actual: current_cell_count,
                },
            );
        }
        let geometry_snapshot = geometry_snapshot.terrain();
        if geometry_snapshot.cells.len() != COMPLETE_TERRAIN_CELL_COUNT {
            return Err(
                MainBaseAbortWorldEffectsConstructorBlock::GeometrySnapshotCellCount {
                    expected: COMPLETE_TERRAIN_CELL_COUNT,
                    actual: geometry_snapshot.cells.len(),
                },
            );
        }
        if current_terrain.header != geometry_snapshot.header {
            return Err(MainBaseAbortWorldEffectsConstructorBlock::GeometrySnapshotHeaderMismatch);
        }
        for (index, (current, snapshot)) in current_terrain
            .cells
            .iter()
            .zip(&geometry_snapshot.cells)
            .enumerate()
        {
            let current_type_bit_10 = current.terrain_type & 0x10;
            let snapshot_type_bit_10 = snapshot.terrain_type & 0x10;
            if current.height != snapshot.height || current_type_bit_10 != snapshot_type_bit_10 {
                return Err(
                    MainBaseAbortWorldEffectsConstructorBlock::GeometrySnapshotStableCellMismatch {
                        cell: [(index / GRID_SIZE) as u8, (index % GRID_SIZE) as u8],
                        current_height: current.height,
                        snapshot_height: snapshot.height,
                        current_type_bit_10,
                        snapshot_type_bit_10,
                    },
                );
            }
        }

        Ok(Self {
            geometry_snapshot,
            cache,
            static_damage,
            terrain_attribute_publications: Vec::new(),
            static_radial_outcomes: Vec::new(),
            terrain_dirty: false,
            last_static_radial_preflight_block: None,
        })
    }

    pub fn terrain_attribute_publications(&self) -> &[MainBaseAbortTerrainAttributePublication] {
        &self.terrain_attribute_publications
    }

    pub fn static_radial_outcomes(&self) -> &[MainBaseType61StaticRadialOutcome] {
        &self.static_radial_outcomes
    }

    pub const fn terrain_dirty(&self) -> bool {
        self.terrain_dirty
    }

    pub const fn last_static_radial_preflight_block(
        &self,
    ) -> Option<MainBaseType61StaticRadialPreflightBlock> {
        self.last_static_radial_preflight_block
    }

    pub fn summary(&self) -> MainBaseAbortWorldEffectsSummary {
        MainBaseAbortWorldEffectsSummary {
            terrain_attribute_publication_count: self.terrain_attribute_publications.len(),
            static_radial_outcome_count: self.static_radial_outcomes.len(),
            terrain_dirty: self.terrain_dirty,
        }
    }

    /// Release the mutable world borrows while retaining all ordered outputs.
    pub fn finish(self) -> MainBaseAbortWorldEffectsReport {
        MainBaseAbortWorldEffectsReport {
            terrain_attribute_publications: self.terrain_attribute_publications,
            static_radial_outcomes: self.static_radial_outcomes,
            terrain_dirty: self.terrain_dirty,
        }
    }
}

impl MainBaseAbortAlternateCleanupEffects for MainBaseAbortWorldEffects<'_> {
    fn clear_and_publish_old_terrain_attribute(&mut self, cell: [u8; 2]) {
        let old_attribute = self
            .cache
            .take_level_terrain_object_attribute(cell)
            .expect("constructor authenticated a complete current-level Section-10 allocation");
        self.terrain_attribute_publications
            .push(MainBaseAbortTerrainAttributePublication {
                cell,
                old_attribute,
            });
        self.terrain_dirty |= old_attribute != 0;
    }
}

impl MainBaseType61StaticRadialEffects for MainBaseAbortWorldEffects<'_> {
    type Prepared = MainBaseType61StaticRadialPlan;

    fn preflight_static_radial(
        &mut self,
        origin_raw: [i16; 3],
        template: RadialDamageTemplate,
    ) -> Option<Self::Prepared> {
        // FUN_00427F20 resolves the descriptor and selected model before its
        // radial helper, because model collision radius participates in vertical
        // placement. It reaches FUN_00427950's kind-specific damage closure
        // only when that helper accepts the candidate.
        let mut resolved_candidates = Vec::new();
        let hits = scan_static_radial(self.geometry_snapshot, origin_raw, template, |cell| {
            match resolve_current_static_damage_snapshot(self.cache, cell) {
                Ok(None) => None,
                Ok(Some(snapshot)) => {
                    resolved_candidates.push((
                        cell,
                        Ok((snapshot.attribute, snapshot.target.state.kind_index)),
                    ));
                    Some(snapshot.target.state)
                }
                Err(error) => {
                    resolved_candidates.push((cell, Err(error)));
                    None
                }
            }
        });
        let first_block = resolved_candidates
            .into_iter()
            .find_map(|(cell, resolution)| {
                let (attribute, kind_index) = match resolution {
                    Ok(resolved) => resolved,
                    Err(error) => {
                        return Some(
                            MainBaseType61StaticRadialPreflightBlock::CurrentStaticDamageLookup(
                                error,
                            ),
                        );
                    }
                };
                (hits.iter().any(|hit| hit.target.cell == cell)
                    && !static_damage_kind_is_admitted(kind_index))
                .then_some(MainBaseType61StaticRadialPreflightBlock::UnsupportedKind {
                    cell,
                    attribute,
                    kind_index,
                })
            });

        self.last_static_radial_preflight_block = first_block;
        first_block
            .is_none()
            .then_some(MainBaseType61StaticRadialPlan { hits })
    }

    fn hive_dying_burst_model_extent_raw(&self, model_id: usize) -> Option<u16> {
        self.cache.global_model(model_id).map(|model| model.radius)
    }

    fn commit_static_radial(&mut self, world_fx: &mut WorldFx, prepared: Self::Prepared) {
        for hit in prepared.hits {
            let outcome = self
                .static_damage
                .submit_hit(hit.target, hit.packet, &mut || {
                    world_fx.next_shared_retail_random_u16()
                });
            let immediate_terrain_transition_applied = match outcome {
                StaticDamageOutcome::ImmediateBurn { cell, .. } => {
                    let result = crate::static_terrain_burn::apply_immediate_static_burn(
                        cell, self.cache, world_fx,
                    );
                    let outcome = result.unwrap_or_else(|error| {
                        eprintln!("Main Base static burn callback blocked: {error:?}");
                        error.outcome
                    });
                    let applied = outcome.terrain_changed;
                    self.terrain_dirty |= applied;
                    applied
                }
                StaticDamageOutcome::BurnedKind10Transition { cell, .. } => {
                    let applied = self.cache.apply_burned_kind_10_transition(cell);
                    assert!(
                        applied,
                        "constructor authenticated a complete current-level Section-10 allocation"
                    );
                    self.terrain_dirty = true;
                    applied
                }
                _ => false,
            };
            self.static_radial_outcomes
                .push(MainBaseType61StaticRadialOutcome {
                    hit,
                    outcome,
                    immediate_terrain_transition_applied,
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::damage::DamagePacket;
    use crate::level::LevelState;
    use crate::main_base_type61_abort::TYPE61_RADIAL_DAMAGE_BASE;
    use crate::retail_rng::retail_random_u16;
    use crate::static_damage::{
        StaticDamageChanceSample, StaticDamageTarget, StaticDamageTargetState,
        BURNED_TERRAIN_TYPE_BIT, KIND_0_STATIC_OBJECT, KIND_10_STATIC_OBJECT, KIND_9_STATIC_OBJECT,
    };
    use v2k_formats::anim_frames::{ModelSlotPattern, TerrainObjectDescriptor, TerrainObjectTable};
    use v2k_formats::models::{ModelCollection, ModelEntry, StreamStats};
    use v2k_formats::terrain::TerrainCell;

    fn terrain(cell_count: usize) -> TerrainGrid {
        TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0,
                };
                cell_count
            ],
        }
    }

    fn geometry_snapshot(cell_count: usize) -> MainBaseAbortTerrainGeometrySnapshot {
        MainBaseAbortTerrainGeometrySnapshot {
            terrain: terrain(cell_count),
        }
    }

    fn level_state(terrain: Option<TerrainGrid>) -> LevelState {
        LevelState {
            source_path: "main-base-world-effects-test.ovl".into(),
            system_level: None,
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
            terrain,
            anim_sound: None,
            collision: None,
            level: None,
            linkage: None,
        }
    }

    fn cache_with_level(terrain: TerrainGrid) -> ResourceCache {
        let mut cache = ResourceCache::new(Vec::new());
        cache.load_level(level_state(Some(terrain)));
        cache
    }

    fn static_model() -> ModelEntry {
        ModelEntry {
            index: 0,
            cmd_word_count: 0,
            extra_count: 0,
            flags: 0x40,
            slot_count: 0,
            face_val: 2,
            radius: 100,
            collision_radius_raw: 100,
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

    fn cache_with_static_level(
        live_terrain: TerrainGrid,
        static_kinds: &[(u8, u32)],
    ) -> ResourceCache {
        let mut resources = level_state(None);
        resources.system_level = Some(2);
        resources.models = Some(ModelCollection {
            sub_blocks: Vec::new(),
            all_entries: vec![static_model()],
            stats: StreamStats::default(),
        });
        let mut records = vec![
            TerrainObjectDescriptor {
                model_ids: [0; 4],
                kind_index: 0,
                pattern: ModelSlotPattern::Static,
            };
            256
        ];
        for &(attribute, kind_index) in static_kinds {
            records[usize::from(attribute)].kind_index = kind_index;
        }
        resources.anim_frames = Some(TerrainObjectTable { records });

        let mut cache = ResourceCache::new(vec![resources]);
        cache.load_level(level_state(Some(live_terrain)));
        cache
    }

    fn cell_mut(terrain: &mut TerrainGrid, cell: [u8; 2]) -> &mut TerrainCell {
        &mut terrain.cells[usize::from(cell[0]) * GRID_SIZE + usize::from(cell[1])]
    }

    #[test]
    fn constructor_requires_two_complete_section_10_allocations() {
        let geometry = geometry_snapshot(COMPLETE_TERRAIN_CELL_COUNT);
        let mut no_level = ResourceCache::new(Vec::new());
        let mut scheduler = StaticDamageScheduler::new();
        assert!(matches!(
            MainBaseAbortWorldEffects::new(&geometry, &mut no_level, &mut scheduler),
            Err(MainBaseAbortWorldEffectsConstructorBlock::MissingCurrentLevelTerrain)
        ));

        let mut incomplete_cache = cache_with_level(terrain(12));
        assert!(matches!(
            MainBaseAbortWorldEffects::new(&geometry, &mut incomplete_cache, &mut scheduler),
            Err(
                MainBaseAbortWorldEffectsConstructorBlock::CurrentLevelTerrainCellCount {
                    expected: COMPLETE_TERRAIN_CELL_COUNT,
                    actual: 12,
                }
            )
        ));

        let incomplete_geometry = geometry_snapshot(34);
        let mut cache = cache_with_level(terrain(COMPLETE_TERRAIN_CELL_COUNT));
        assert!(matches!(
            MainBaseAbortWorldEffects::new(&incomplete_geometry, &mut cache, &mut scheduler),
            Err(
                MainBaseAbortWorldEffectsConstructorBlock::GeometrySnapshotCellCount {
                    expected: COMPLETE_TERRAIN_CELL_COUNT,
                    actual: 34,
                }
            )
        ));

        let mut header_mismatch = geometry_snapshot(COMPLETE_TERRAIN_CELL_COUNT);
        header_mismatch.terrain.header[2] = 1;
        assert!(matches!(
            MainBaseAbortWorldEffects::new(&header_mismatch, &mut cache, &mut scheduler),
            Err(MainBaseAbortWorldEffectsConstructorBlock::GeometrySnapshotHeaderMismatch)
        ));

        let mut stable_cell_mismatch = geometry_snapshot(COMPLETE_TERRAIN_CELL_COUNT);
        stable_cell_mismatch.terrain.cells[GRID_SIZE + 2].height = 3;
        stable_cell_mismatch.terrain.cells[GRID_SIZE + 2].terrain_type = 0x10;
        let Err(stable_cell_block) =
            MainBaseAbortWorldEffects::new(&stable_cell_mismatch, &mut cache, &mut scheduler)
        else {
            panic!("mismatched stable terrain inputs must reject custody")
        };
        assert_eq!(
            stable_cell_block,
            MainBaseAbortWorldEffectsConstructorBlock::GeometrySnapshotStableCellMismatch {
                cell: [1, 2],
                current_height: 0,
                snapshot_height: 3,
                current_type_bit_10: 0,
                snapshot_type_bit_10: 0x10,
            }
        );
    }

    #[test]
    fn type67_publishes_zero_and_nonzero_old_bytes_but_dirties_only_for_change() {
        let cell = [10, 20];
        let geometry = geometry_snapshot(COMPLETE_TERRAIN_CELL_COUNT);
        let mut live_terrain = terrain(COMPLETE_TERRAIN_CELL_COUNT);
        cell_mut(&mut live_terrain, cell).attribute = 0xb3;
        let mut cache = cache_with_level(live_terrain);
        let mut scheduler = StaticDamageScheduler::new();

        let report = {
            let mut effects =
                MainBaseAbortWorldEffects::new(&geometry, &mut cache, &mut scheduler).unwrap();
            effects.clear_and_publish_old_terrain_attribute(cell);
            effects.clear_and_publish_old_terrain_attribute(cell);
            assert_eq!(
                effects.summary(),
                MainBaseAbortWorldEffectsSummary {
                    terrain_attribute_publication_count: 2,
                    static_radial_outcome_count: 0,
                    terrain_dirty: true,
                }
            );
            effects.finish()
        };

        assert_eq!(
            report.terrain_attribute_publications(),
            [
                MainBaseAbortTerrainAttributePublication {
                    cell,
                    old_attribute: 0xb3,
                },
                MainBaseAbortTerrainAttributePublication {
                    cell,
                    old_attribute: 0,
                },
            ]
        );
        assert!(report.terrain_dirty());
        assert_eq!(
            cache
                .level()
                .unwrap()
                .terrain
                .as_ref()
                .unwrap()
                .cell(usize::from(cell[0]), usize::from(cell[1]))
                .unwrap()
                .attribute,
            0
        );
    }

    #[test]
    fn type67_clear_is_visible_to_the_later_type61_static_preflight() {
        let cell = [10, 20];
        let mut live_terrain = terrain(COMPLETE_TERRAIN_CELL_COUNT);
        cell_mut(&mut live_terrain, cell).attribute = 5;
        let mut cache = cache_with_static_level(live_terrain, &[(5, KIND_0_STATIC_OBJECT)]);
        let geometry = snapshot_main_base_abort_terrain_geometry(&cache).unwrap();
        let mut scheduler = StaticDamageScheduler::new();

        let report = {
            let mut effects =
                MainBaseAbortWorldEffects::new(&geometry, &mut cache, &mut scheduler).unwrap();
            effects.clear_and_publish_old_terrain_attribute(cell);
            let plan = effects
                .preflight_static_radial(
                    [i16::from(cell[0]) << 8, 0, i16::from(cell[1]) << 8],
                    TYPE61_RADIAL_DAMAGE_BASE,
                )
                .expect("the cleared live cell is an ordinary empty candidate");
            assert!(plan.is_empty());
            effects.finish()
        };

        assert_eq!(
            report.terrain_attribute_publications(),
            [MainBaseAbortTerrainAttributePublication {
                cell,
                old_attribute: 5,
            }]
        );
        assert!(report.static_radial_outcomes().is_empty());
        assert_eq!(scheduler.active_program_count(), 0);
    }

    #[test]
    fn preflight_fails_closed_for_any_unresolved_nonempty_candidate_without_mutation() {
        let cell = [0, 0];
        let geometry = geometry_snapshot(COMPLETE_TERRAIN_CELL_COUNT);
        let mut live_terrain = terrain(COMPLETE_TERRAIN_CELL_COUNT);
        cell_mut(&mut live_terrain, cell).attribute = 1;
        let mut cache = cache_with_level(live_terrain);
        let mut scheduler = StaticDamageScheduler::new();
        let mut effects =
            MainBaseAbortWorldEffects::new(&geometry, &mut cache, &mut scheduler).unwrap();

        assert_eq!(
            effects.preflight_static_radial([0; 3], TYPE61_RADIAL_DAMAGE_BASE),
            None
        );
        assert_eq!(
            effects.last_static_radial_preflight_block(),
            Some(
                MainBaseType61StaticRadialPreflightBlock::CurrentStaticDamageLookup(
                    CurrentStaticDamageLookupError::MissingTerrainObjectTable,
                )
            )
        );
        assert!(!effects.terrain_dirty());
        assert!(effects.terrain_attribute_publications().is_empty());
        assert!(effects.static_radial_outcomes().is_empty());
    }

    #[test]
    fn preflight_resolves_an_admitted_live_target_and_commit_starts_it() {
        let cell = [3, 4];
        let geometry = geometry_snapshot(COMPLETE_TERRAIN_CELL_COUNT);
        let mut live_terrain = terrain(COMPLETE_TERRAIN_CELL_COUNT);
        cell_mut(&mut live_terrain, cell).attribute = 5;
        let mut cache = cache_with_static_level(live_terrain, &[(5, KIND_0_STATIC_OBJECT)]);
        let mut scheduler = StaticDamageScheduler::new();
        let mut world_fx = WorldFx::new();

        let report = {
            let mut effects =
                MainBaseAbortWorldEffects::new(&geometry, &mut cache, &mut scheduler).unwrap();
            let plan = effects
                .preflight_static_radial(
                    [i16::from(cell[0]) << 8, 0, i16::from(cell[1]) << 8],
                    TYPE61_RADIAL_DAMAGE_BASE,
                )
                .expect("the live kind-0 target has a complete admitted closure");
            assert_eq!(plan.hits().len(), 1);
            assert_eq!(plan.hits()[0].target.cell, cell);
            effects.commit_static_radial(&mut world_fx, plan);
            effects.finish()
        };

        assert_eq!(report.static_radial_outcomes().len(), 1);
        assert!(matches!(
            report.static_radial_outcomes()[0].outcome,
            StaticDamageOutcome::Started { .. }
        ));
        assert_eq!(scheduler.active_program_count(), 1);
    }

    #[test]
    fn unsupported_live_candidate_blocks_the_whole_static_preflight() {
        let cell = [3, 4];
        let geometry = geometry_snapshot(COMPLETE_TERRAIN_CELL_COUNT);
        let mut live_terrain = terrain(COMPLETE_TERRAIN_CELL_COUNT);
        cell_mut(&mut live_terrain, cell).attribute = 5;
        let mut cache = cache_with_static_level(live_terrain, &[(5, 5)]);
        let mut scheduler = StaticDamageScheduler::new();
        let mut effects =
            MainBaseAbortWorldEffects::new(&geometry, &mut cache, &mut scheduler).unwrap();

        assert_eq!(
            effects.preflight_static_radial(
                [i16::from(cell[0]) << 8, 0, i16::from(cell[1]) << 8],
                TYPE61_RADIAL_DAMAGE_BASE,
            ),
            None
        );
        assert_eq!(
            effects.last_static_radial_preflight_block(),
            Some(MainBaseType61StaticRadialPreflightBlock::UnsupportedKind {
                cell,
                attribute: 5,
                kind_index: 5,
            })
        );
        assert!(effects.static_radial_outcomes().is_empty());
    }

    #[test]
    fn unsupported_candidate_outside_the_radial_bound_does_not_block() {
        let cell = [3, 4];
        let mut live_terrain = terrain(COMPLETE_TERRAIN_CELL_COUNT);
        for corner in [cell, [4, 4], [3, 5], [4, 5]] {
            cell_mut(&mut live_terrain, corner).height = 127;
        }
        cell_mut(&mut live_terrain, cell).attribute = 5;
        let mut cache = cache_with_static_level(live_terrain, &[(5, 5)]);
        let geometry = snapshot_main_base_abort_terrain_geometry(&cache).unwrap();
        let mut scheduler = StaticDamageScheduler::new();
        let mut effects =
            MainBaseAbortWorldEffects::new(&geometry, &mut cache, &mut scheduler).unwrap();

        let plan = effects
            .preflight_static_radial(
                [i16::from(cell[0]) << 8, 0, i16::from(cell[1]) << 8],
                TYPE61_RADIAL_DAMAGE_BASE,
            )
            .expect("retail never submits an out-of-range static candidate");
        assert!(plan.is_empty());
        assert_eq!(effects.last_static_radial_preflight_block(), None);
    }

    #[test]
    fn kind10_transition_is_applied_synchronously_and_marks_terrain_dirty() {
        let cell = [7, 9];
        let geometry = geometry_snapshot(COMPLETE_TERRAIN_CELL_COUNT);
        let mut live_terrain = terrain(COMPLETE_TERRAIN_CELL_COUNT);
        let live_cell = cell_mut(&mut live_terrain, cell);
        live_cell.attribute = 7;
        live_cell.terrain_type = BURNED_TERRAIN_TYPE_BIT;
        let mut cache = cache_with_static_level(
            live_terrain,
            &[(7, KIND_10_STATIC_OBJECT), (8, KIND_0_STATIC_OBJECT)],
        );
        let mut scheduler = StaticDamageScheduler::new();
        let mut world_fx = WorldFx::new();
        let target = StaticDamageTarget {
            cell,
            state: StaticDamageTargetState {
                kind_index: KIND_10_STATIC_OBJECT,
                terrain_type: BURNED_TERRAIN_TYPE_BIT,
                terrain_height_byte: 0,
                collision_radius_raw: 100,
                effect_extent_raw: 100,
            },
        };
        let hit = StaticRadialHit {
            target,
            position_raw: [0; 3],
            distance_raw: 0,
            packet: DamagePacket::collision(9_999),
            trailing_raw: [0; 2],
        };

        let report = {
            let mut effects =
                MainBaseAbortWorldEffects::new(&geometry, &mut cache, &mut scheduler).unwrap();
            effects.commit_static_radial(
                &mut world_fx,
                MainBaseType61StaticRadialPlan { hits: vec![hit] },
            );
            let next_actor_plan = effects
                .preflight_static_radial(
                    [i16::from(cell[0]) << 8, 0, i16::from(cell[1]) << 8],
                    TYPE61_RADIAL_DAMAGE_BASE,
                )
                .expect("the next actor sees kind 10's live descriptor transition");
            assert_eq!(next_actor_plan.hits().len(), 1);
            assert_eq!(next_actor_plan.hits()[0].target.cell, cell);
            assert_eq!(
                next_actor_plan.hits()[0].target.state.kind_index,
                KIND_0_STATIC_OBJECT
            );
            assert_eq!(
                next_actor_plan.hits()[0].target.state.terrain_type & BURNED_TERRAIN_TYPE_BIT,
                0
            );
            effects.finish()
        };

        assert_eq!(report.static_radial_outcomes().len(), 1);
        assert_eq!(report.static_radial_outcomes()[0].hit, hit);
        let outcome = report.static_radial_outcomes()[0].outcome;
        assert!(
            matches!(
                outcome,
                StaticDamageOutcome::BurnedKind10Transition { cell: outcome_cell, .. }
                    if outcome_cell == cell
            ),
            "unexpected kind-10 radial outcome: {outcome:?}"
        );
        assert!(report.static_radial_outcomes()[0].immediate_terrain_transition_applied);
        assert!(report.terrain_dirty());
        assert_eq!(scheduler.active_program_count(), 0);
        let changed = cache
            .level()
            .unwrap()
            .terrain
            .as_ref()
            .unwrap()
            .cell(usize::from(cell[0]), usize::from(cell[1]))
            .unwrap();
        assert_eq!(changed.attribute, 8);
        assert_eq!(changed.terrain_type & BURNED_TERRAIN_TYPE_BIT, 0);
    }

    #[test]
    fn kind4_null_program_burn_is_applied_before_the_next_radial_actor() {
        use crate::static_damage::KIND_4_STATIC_OBJECT;
        let cell = [7, 9];
        let mut live_terrain = terrain(COMPLETE_TERRAIN_CELL_COUNT);
        let live_cell = cell_mut(&mut live_terrain, cell);
        live_cell.attribute = 7;
        live_cell.terrain_type = 0x10;
        let mut geometry = geometry_snapshot(COMPLETE_TERRAIN_CELL_COUNT);
        *cell_mut(&mut geometry.terrain, cell) = *live_cell;
        let mut cache = cache_with_static_level(live_terrain, &[(7, KIND_4_STATIC_OBJECT)]);
        let mut scheduler = StaticDamageScheduler::new();
        let mut world_fx = WorldFx::new();
        let mut effects =
            MainBaseAbortWorldEffects::new(&geometry, &mut cache, &mut scheduler).unwrap();
        let origin = [i16::from(cell[0]) << 8, 0, i16::from(cell[1]) << 8];
        let first = effects
            .preflight_static_radial(origin, TYPE61_RADIAL_DAMAGE_BASE)
            .expect("kind4 has an authenticated damage descriptor");
        assert_eq!(first.hits().len(), 1);
        effects.commit_static_radial(&mut world_fx, first);
        let second = effects
            .preflight_static_radial(origin, TYPE61_RADIAL_DAMAGE_BASE)
            .unwrap();
        assert_eq!(second.hits().len(), 1);
        assert_eq!(second.hits()[0].target.state.terrain_type, 0x18);
        effects.commit_static_radial(&mut world_fx, second);
        let report = effects.finish();
        assert!(
            matches!(report.static_radial_outcomes()[0].outcome, StaticDamageOutcome::ImmediateBurn { cell: actual, .. } if actual == cell)
        );
        assert!(report.static_radial_outcomes()[0].immediate_terrain_transition_applied);
        assert!(matches!(
            report.static_radial_outcomes()[1].outcome,
            StaticDamageOutcome::BurnedIgnored { .. }
        ));
        assert!(!report.static_radial_outcomes()[1].immediate_terrain_transition_applied);
        assert!(report.terrain_dirty());
        assert!(cache.take_level_terrain_presentation_dirty());
        assert_eq!(scheduler.active_program_count(), 0);
        assert_eq!(
            world_fx.particle_count(),
            3,
            "unprimed governor admits one class93 plus the two underwater tails"
        );
        assert_eq!(
            world_fx
                .take_positional_sounds()
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>(),
            [62]
        );
    }

    #[test]
    fn static_commit_preserves_hit_order_and_the_shared_rng_stream() {
        let geometry = geometry_snapshot(COMPLETE_TERRAIN_CELL_COUNT);
        let mut cache = cache_with_level(terrain(COMPLETE_TERRAIN_CELL_COUNT));
        let mut scheduler = StaticDamageScheduler::new();
        let mut world_fx = WorldFx::new();
        let hits = [[7, 9], [8, 9]].map(|cell| StaticRadialHit {
            target: StaticDamageTarget {
                cell,
                state: StaticDamageTargetState {
                    kind_index: KIND_9_STATIC_OBJECT,
                    terrain_type: 0,
                    terrain_height_byte: 0,
                    collision_radius_raw: 100,
                    effect_extent_raw: 100,
                },
            },
            position_raw: [0; 3],
            distance_raw: 0,
            packet: DamagePacket::collision(4_250),
            trailing_raw: [0; 2],
        });
        let mut expected_rng_state = 0;
        let expected_rolls = [
            retail_random_u16(&mut expected_rng_state),
            retail_random_u16(&mut expected_rng_state),
        ];
        let expected_next = retail_random_u16(&mut expected_rng_state);

        let report = {
            let mut effects =
                MainBaseAbortWorldEffects::new(&geometry, &mut cache, &mut scheduler).unwrap();
            effects.commit_static_radial(
                &mut world_fx,
                MainBaseType61StaticRadialPlan {
                    hits: hits.to_vec(),
                },
            );
            effects.finish()
        };

        assert_eq!(
            report
                .static_radial_outcomes()
                .iter()
                .map(|record| record.hit.target.cell)
                .collect::<Vec<_>>(),
            hits.map(|hit| hit.target.cell)
        );
        assert_eq!(
            report
                .static_radial_outcomes()
                .iter()
                .map(|record| match record.outcome {
                    StaticDamageOutcome::Started {
                        sample: Some(StaticDamageChanceSample { roll, .. }),
                        ..
                    } => roll,
                    other => panic!("expected a chance-gated started program, got {other:?}"),
                })
                .collect::<Vec<_>>(),
            expected_rolls
        );
        assert_eq!(world_fx.next_shared_retail_random_u16(), expected_next);
        assert_eq!(scheduler.active_program_count(), 2);
    }
}
