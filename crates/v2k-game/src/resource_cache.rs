use std::collections::HashSet;
use std::fmt::Write;

use v2k_formats::anim_frames::TerrainObjectTable;
use v2k_formats::anim_sound::AnimSoundTable;
use v2k_formats::collision::CollisionModel;
use v2k_formats::levels::LevelDescriptor;
use v2k_formats::linkage::{LinkageTable, SpriteDependencies};
use v2k_formats::models::ModelCollection;
use v2k_formats::params::ParamTable;
use v2k_formats::radar_projection::RadarProjectionTables;
use v2k_formats::sprites::SpriteAtlas;
use v2k_formats::system::{DisplayMode, FixupTable, FogGradientEntry, PaletteEntry};
use v2k_formats::terrain::TerrainGrid;

use crate::infection_evolution::{InfectionCellWrite, INFECTION_TERRAIN_TYPE_BIT};
use crate::level::LevelState;

mod system_layout;

/// Multi-OVL resource cache: stores all loaded OVLs in a flat list
/// ordered by priority. Lookup scans all layers in reverse order
/// (most recently loaded first).
///
/// This mirrors the original game's resource model where OVLs can be
/// loaded arbitrarily — a weapon sound from level 7 might be used on
/// level 3. The layer system supports preload (from PRELOAD.DAT),
/// auxiliary OVLs (persistent disk-loaded OVLs), and the current
/// gameplay level.
pub struct ResourceCache {
    /// All loaded OVLs, ordered by priority (last = highest priority).
    /// Level OVL goes on top, system OVLs in the middle, preload at bottom.
    layers: Vec<CacheLayer>,
    /// `FUN_00456960` may apply the `FUN_00433E30` Section-10 abort transform
    /// only once during one loaded world. This is reset with the level layer.
    level_abort_terrain_transformed: bool,
    /// Persistent 4AFB0/4AB20 terrain raster; its refreshes share world RNG.
    level_terrain_radar: Option<crate::gameplay_radar::TerrainRadar>,
    /// A live material/light write requires the next terrain submission to
    /// rebuild its cached mesh, including after a later radar refresh block.
    level_terrain_presentation_dirty: bool,
}

/// Panic-safe current-level write boundary for one speculative Main Base
/// abort.
///
/// Parsed OVL sections are intentionally not generally cloneable.  The abort
/// body mutates only the live Section-10 allocation and its one-shot latch, so
/// this guard parks the original terrain, installs an exact working copy, and
/// restores both on every uncommitted return (including unwinding).
pub(crate) struct MainBaseAbortResourceTransaction<'a> {
    cache: &'a mut ResourceCache,
    level_layer_index: Option<usize>,
    original_terrain: Option<TerrainGrid>,
    original_transform_latch: bool,
    original_terrain_radar: Option<crate::gameplay_radar::TerrainRadar>,
    original_presentation_dirty: bool,
    committed: bool,
}

impl std::ops::Deref for MainBaseAbortResourceTransaction<'_> {
    type Target = ResourceCache;

    fn deref(&self) -> &Self::Target {
        self.cache
    }
}

impl std::ops::DerefMut for MainBaseAbortResourceTransaction<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.cache
    }
}

impl MainBaseAbortResourceTransaction<'_> {
    /// Retain the installed working terrain and one-shot latch.
    pub(crate) fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for MainBaseAbortResourceTransaction<'_> {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        if let Some(index) = self.level_layer_index {
            if let Some(layer) = self.cache.layers.get_mut(index) {
                layer.state.terrain = self.original_terrain.take();
            }
        }
        self.cache.level_abort_terrain_transformed = self.original_transform_latch;
        self.cache.level_terrain_radar = self.original_terrain_radar.take();
        self.cache.level_terrain_presentation_dirty = self.original_presentation_dirty;
    }
}

/// Observable Section-10 changes made by retail `FUN_00433E30`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseAbortTerrainOutcome {
    /// Cells whose high three-bit authored light value changed.
    pub darkened_light_cells: usize,
    /// Qualifying object cells whose live state bit 0x08 changed from clear.
    pub newly_activated_object_cells: usize,
    /// Total `FUN_004337E0(..., 1)` calls, including kind-10 repetitions.
    pub object_activation_calls: usize,
    /// Shared `Random_Next` samples consumed by kind-10 descriptors.
    pub kind_10_rng_draws: usize,
}

/// A single OVL in the cache with its parsed sections and role tag.
struct CacheLayer {
    state: LevelState,
    kind: LayerKind,
    system_layout_receipt: Option<crate::system_layout::HighSystemLayerReceipt>,
}

/// What role this OVL serves in the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayerKind {
    /// From PRELOAD.DAT embedded OVLs.
    Preload,
    /// Disk-loaded auxiliary OVL (sits above preload, below gameplay level).
    Auxiliary,
    /// Current gameplay level.
    Level,
}

/// Canonical bases in the executable's `DAT_004FE640` global Section-8 pool.
/// Unloaded biome ranges remain holes because authored model ids are absolute.
const GLOBAL_MODEL_LAYERS: &[(u32, usize)] = &[
    (2, 0),
    (3, 13),
    (5, 315),
    (6, 324),
    (7, 563),
    (8, 693),
    (9, 973),
    (10, 1108),
    (11, 1186),
];

/// Canonical bases in the executable's `DAT_004FE63C` global Section-7 pool.
///
/// PRELOAD.DAT's count matrix fixes levels 2 and 4 at 86 and 1 entries, then
/// reserves 48 entries for every mutually exclusive world-style pack 6..=11.
/// Only the active world pack is resident, so computing bases from resident
/// layers would wrongly compress style 6 (level 11) from global 327 to 87.
const GLOBAL_PALETTE_LAYERS: &[(u32, usize)] = &[
    (2, 0),
    (4, 86),
    (6, 87),
    (7, 135),
    (8, 183),
    (9, 231),
    (10, 279),
    (11, 327),
];

/// Generate a section accessor that scans layers in reverse order (newest first).
///
/// For `Option<T>` fields: returns the first `Some` found.
macro_rules! section_accessor {
    ($name:ident, $field:ident, $ty:ty) => {
        pub fn $name(&self) -> Option<&$ty> {
            for layer in self.layers.iter().rev() {
                if let Some(ref v) = layer.state.$field {
                    return Some(v);
                }
            }
            None
        }
    };
}

impl ResourceCache {
    /// Create a new cache pre-populated with system OVLs from PRELOAD.DAT.
    pub fn new(preload: Vec<LevelState>) -> Self {
        let layers = preload
            .into_iter()
            .map(|state| CacheLayer {
                state,
                kind: LayerKind::Preload,
                system_layout_receipt: None,
            })
            .collect();
        Self {
            layers,
            level_abort_terrain_transformed: false,
            level_terrain_radar: None,
            level_terrain_presentation_dirty: false,
        }
    }

    /// Install an exact working copy of the only resource state mutated by a
    /// speculative Main Base abort.  Dropping the returned guard rolls back;
    /// [`MainBaseAbortResourceTransaction::commit`] retains the working copy.
    pub(crate) fn begin_main_base_abort_resource_transaction(
        &mut self,
    ) -> MainBaseAbortResourceTransaction<'_> {
        let original_transform_latch = self.level_abort_terrain_transformed;
        let original_terrain_radar = self.level_terrain_radar.clone();
        let original_presentation_dirty = self.level_terrain_presentation_dirty;
        let level_layer_index = self
            .layers
            .iter()
            .rposition(|layer| layer.kind == LayerKind::Level);
        let original_terrain =
            level_layer_index.and_then(|index| self.layers[index].state.terrain.take());
        if let Some(index) = level_layer_index {
            self.layers[index].state.terrain =
                original_terrain.as_ref().map(|terrain| TerrainGrid {
                    header: terrain.header,
                    cells: terrain.cells.clone(),
                });
        }
        MainBaseAbortResourceTransaction {
            cache: self,
            level_layer_index,
            original_terrain,
            original_transform_latch,
            original_terrain_radar,
            original_presentation_dirty,
            committed: false,
        }
    }

    /// Add an auxiliary OVL (persistent, above preload, below level).
    ///
    /// Auxiliary OVLs are inserted after all Preload layers but before
    /// any Level layer, so they have higher priority than preload
    /// but lower than the gameplay level.
    pub fn add_auxiliary(&mut self, state: LevelState) {
        self.insert_auxiliary(state, None);
    }

    fn insert_auxiliary(
        &mut self,
        state: LevelState,
        system_layout_receipt: Option<crate::system_layout::HighSystemLayerReceipt>,
    ) {
        // Find insertion point: after last Preload or Auxiliary, before Level
        let insert_pos = self
            .layers
            .iter()
            .rposition(|l| l.kind == LayerKind::Preload || l.kind == LayerKind::Auxiliary)
            .map(|i| i + 1)
            .unwrap_or(0);
        self.layers.insert(
            insert_pos,
            CacheLayer {
                state,
                kind: LayerKind::Auxiliary,
                system_layout_receipt,
            },
        );
    }

    /// Load a level into the cache, replacing any previously loaded level.
    ///
    /// The level layer is always pushed to the end (highest priority).
    pub fn load_level(&mut self, state: LevelState) {
        self.unload_level();
        self.layers.push(CacheLayer {
            state,
            kind: LayerKind::Level,
            system_layout_receipt: None,
        });
        self.level_abort_terrain_transformed = false;
    }

    /// Unload the current level, keeping preload and system resources.
    pub fn unload_level(&mut self) {
        self.layers.retain(|l| l.kind != LayerKind::Level);
        self.level_abort_terrain_transformed = false;
        self.level_terrain_radar = None;
        self.level_terrain_presentation_dirty = false;
    }

    /// Borrow the live world raster shared by terrain updates, the HUD and M map.
    pub fn level_terrain_radar(&self) -> Option<crate::gameplay_radar::TerrainRadarView<'_>> {
        self.level_terrain_radar.as_ref().map(|radar| radar.view())
    }

    /// Successful 2E570 calls 4AFB0 after all authored actor constructors and
    /// completed-world terrain writes, before 51710 restores cargo via 51C00.
    /// Resume does not reconstruct this allocation or consume its RNG again.
    pub fn initialize_level_terrain_radar(
        &mut self,
        next_random: &mut impl FnMut() -> u16,
    ) -> Result<usize, crate::gameplay_radar::TerrainRadarError> {
        if self.level_terrain_radar.is_some() {
            return Ok(0);
        }
        let mut draws = 0;
        let radar = crate::gameplay_radar::TerrainRadar::from_cache(self, &mut || {
            draws += 1;
            next_random()
        })?;
        self.level_terrain_radar = Some(radar);
        Ok(draws)
    }

    /// 4A890 refreshes preserve the original coverage map and input order.
    pub(crate) fn refresh_level_terrain_radar(
        &mut self,
        positions: &[[i16; 2]],
        next_random: &mut impl FnMut() -> u16,
    ) -> Result<usize, crate::gameplay_radar::TerrainRadarError> {
        let mut radar = self
            .level_terrain_radar
            .take()
            .ok_or(crate::gameplay_radar::TerrainRadarError::Uninitialized)?;
        let result = radar.refresh_cells(self, positions, next_random);
        self.level_terrain_radar = Some(radar);
        result
    }

    /// `44A8D0` mutates the resident radar in source order. This never rebuilds
    /// static coverage and is rolled back with the other Main Base resources.
    pub(crate) fn mutate_level_terrain_radar_coverage(
        &mut self,
        position: [i16; 2],
        change: crate::gameplay_radar::RadarCoverageChange,
        next_random: &mut impl FnMut() -> u16,
    ) -> Result<usize, crate::gameplay_radar::TerrainRadarError> {
        let mut radar = self
            .level_terrain_radar
            .take()
            .ok_or(crate::gameplay_radar::TerrainRadarError::Uninitialized)?;
        let result = radar.mutate_coverage(self, position, change, next_random);
        self.level_terrain_radar = Some(radar);
        result
    }

    /// Reference to the current level state, if loaded.
    pub fn level(&self) -> Option<&LevelState> {
        self.layers
            .iter()
            .rev()
            .find(|l| l.kind == LayerKind::Level)
            .map(|l| &l.state)
    }

    /// References to the preload system OVLs.
    pub fn preload(&self) -> Vec<&LevelState> {
        self.layers
            .iter()
            .filter(|l| l.kind == LayerKind::Preload)
            .map(|l| &l.state)
            .collect()
    }

    /// Find the active system/menu OVL with display modes (Section 5).
    ///
    /// Retail replaces PRELOAD's variant-0 presentation pack with the selected
    /// on-disk display tier. Auxiliary layers therefore take priority over the
    /// embedded fallback.
    pub fn menu_ovl(&self) -> Option<&LevelState> {
        self.layers
            .iter()
            .rev()
            .find(|l| l.state.display_modes.is_some())
            .map(|l| &l.state)
    }

    // --- Section accessors (scan layers newest-first) ---

    section_accessor!(fixup_data, fixup_data, FixupTable);
    section_accessor!(fixup_code, fixup_code, FixupTable);
    section_accessor!(sprites, sprites, SpriteAtlas);
    section_accessor!(params, params, ParamTable);
    section_accessor!(display_modes, display_modes, Vec<DisplayMode>);
    section_accessor!(fog_gradient, fog_gradient, Vec<FogGradientEntry>);
    section_accessor!(color_palettes, color_palettes, Vec<PaletteEntry>);
    section_accessor!(models, models, ModelCollection);
    section_accessor!(anim_frames, anim_frames, TerrainObjectTable);
    /// Active biome's Section-9 static terrain-object descriptor table.
    pub fn terrain_objects(&self) -> Option<&TerrainObjectTable> {
        self.anim_frames()
    }
    section_accessor!(terrain, terrain, TerrainGrid);

    /// Current gameplay level's Section-10 terrain allocation.
    ///
    /// Unlike [`Self::terrain`], this accessor never falls back to an
    /// auxiliary or PRELOAD layer. Runtime systems that mutate or pair terrain
    /// with the current level's Section-13 descriptor must use this strict
    /// ownership boundary.
    pub fn level_terrain(&self) -> Option<&TerrainGrid> {
        self.level()?.terrain.as_ref()
    }

    /// Mutable current gameplay level's Section-10 terrain allocation.
    ///
    /// Missing level terrain remains missing even when a lower-priority cache
    /// layer contains Section 10.
    pub fn level_terrain_mut(&mut self) -> Option<&mut TerrainGrid> {
        self.layers
            .iter_mut()
            .rev()
            .find(|layer| layer.kind == LayerKind::Level)?
            .state
            .terrain
            .as_mut()
    }

    /// Take and clear one live Section-10 static-object attribute byte.
    ///
    /// Retail mutates the decompressed level allocation after an accepted
    /// pickup. Restricting this write to the current level preserves immutable
    /// preload resources while automatically synchronizing every terrain
    /// consumer: rendering, static collision, particles, camera probes, and
    /// projectile queries.
    ///
    /// `Some(old_attribute)` distinguishes an existing zero-valued cell from
    /// a missing current-level terrain allocation or cell.
    pub fn take_level_terrain_object_attribute(&mut self, cell: [u8; 2]) -> Option<u8> {
        let terrain = self
            .layers
            .iter_mut()
            .rev()
            .find(|layer| layer.kind == LayerKind::Level)
            .and_then(|layer| layer.state.terrain.as_mut())?;
        let index = usize::from(cell[0]) * v2k_formats::terrain::GRID_SIZE + usize::from(cell[1]);
        let terrain_cell = terrain.cells.get_mut(index)?;
        let old_attribute = terrain_cell.attribute;
        terrain_cell.attribute = 0;
        Some(old_attribute)
    }

    /// Clear one live Section-10 static-object attribute byte.
    ///
    /// Returns whether a nonzero attribute was present. Use
    /// [`Self::take_level_terrain_object_attribute`] when the old byte or the
    /// distinction between a zero byte and missing level terrain is required.
    pub fn clear_level_terrain_object_attribute(&mut self, cell: [u8; 2]) -> bool {
        matches!(
            self.take_level_terrain_object_attribute(cell),
            Some(attribute) if attribute != 0
        )
    }

    /// OR bits into one live Section-10 terrain-type byte.
    ///
    /// Static-object programs address the toroidal terrain grid through
    /// wrapped byte coordinates and mutate the decompressed current-level
    /// allocation in place. Keeping the write on that allocation makes the
    /// changed model selection immediately visible to every terrain consumer
    /// without maintaining a parallel object-state map. Returns whether the
    /// terrain-type byte actually changed.
    pub fn or_level_terrain_type_bits(&mut self, cell: [u8; 2], bits: u8) -> bool {
        let Some(terrain) = self
            .layers
            .iter_mut()
            .rev()
            .find(|layer| layer.kind == LayerKind::Level)
            .and_then(|layer| layer.state.terrain.as_mut())
        else {
            return false;
        };
        let index = usize::from(cell[0]) * v2k_formats::terrain::GRID_SIZE + usize::from(cell[1]);
        let Some(terrain_cell) = terrain.cells.get_mut(index) else {
            return false;
        };
        let old_terrain_type = terrain_cell.terrain_type;
        terrain_cell.terrain_type |= bits;
        let changed = terrain_cell.terrain_type != old_terrain_type;
        self.level_terrain_presentation_dirty |= changed;
        changed
    }

    /// Consume the terrain rebuild request made by a live program write.
    pub fn take_level_terrain_presentation_dirty(&mut self) -> bool {
        std::mem::take(&mut self.level_terrain_presentation_dirty)
    }

    /// Static opcode11 -> 37100 lowers the current cell's high three light
    /// bits, clamps at zero and preserves all five low terrain-state bits.
    ///
    /// Only a positive amount and a nonzero old light reach the write and
    /// 4A890 refresh. The 281A0 position is X=cell*256+128, Z=cell*256;
    /// retaining its half-cell X is necessary for 4A5C0 material rounding.
    /// A refresh failure retains the terrain write and presentation request,
    /// so callers must report it without replaying the consumed opcode.
    pub fn lower_level_terrain_light(
        &mut self,
        cell: [u8; 2],
        amount: i32,
        next_random: &mut impl FnMut() -> u16,
    ) -> Result<bool, crate::gameplay_radar::TerrainRadarError> {
        use crate::gameplay_radar::TerrainRadarError;

        if amount <= 0 {
            return Ok(false);
        }
        let terrain = self
            .level_terrain_mut()
            .ok_or(TerrainRadarError::ResourceUnavailable(
                "current level terrain",
            ))?;
        let index = usize::from(cell[0]) * v2k_formats::terrain::GRID_SIZE + usize::from(cell[1]);
        let terrain_cell =
            terrain
                .cells
                .get_mut(index)
                .ok_or(TerrainRadarError::ResourceUnavailable(
                    "current level terrain cell",
                ))?;
        let light = i32::from(terrain_cell.terrain_type >> 5);
        if light == 0 {
            return Ok(false);
        }
        let remaining = (light - amount).max(0) as u8;
        terrain_cell.terrain_type = (remaining << 5) | (terrain_cell.terrain_type & 0x1f);
        self.level_terrain_presentation_dirty = true;
        let position = [
            ((u16::from(cell[0]) << 8).wrapping_add(0x80)) as i16,
            (u16::from(cell[1]) << 8) as i16,
        ];
        self.refresh_level_terrain_radar(&[position], next_random)?;
        Ok(true)
    }

    /// Apply an ordered batch of exact infection-bit set/clear writes to the
    /// mutable current-level Section-10 allocation.
    ///
    /// The evolution snapshot already observes each preceding write during
    /// the component pass. Replaying the compact mutation log in the same
    /// order preserves that result while releasing the immutable terrain
    /// borrow before cache mutation. Other terrain-type bits are untouched.
    /// Returns `None` when no live level terrain exists and otherwise the
    /// number of bytes whose infection bit actually changed.
    pub fn apply_level_infection_writes(&mut self, writes: &[InfectionCellWrite]) -> Option<usize> {
        let terrain = self
            .layers
            .iter_mut()
            .rev()
            .find(|layer| layer.kind == LayerKind::Level)
            .and_then(|layer| layer.state.terrain.as_mut())?;
        let mut changed = 0;
        for write in writes {
            let index = usize::from(write.cell[0]) * v2k_formats::terrain::GRID_SIZE
                + usize::from(write.cell[1]);
            let Some(cell) = terrain.cells.get_mut(index) else {
                continue;
            };
            let old = cell.terrain_type;
            if write.infected {
                cell.terrain_type |= INFECTION_TERRAIN_TYPE_BIT;
            } else {
                cell.terrain_type &= !INFECTION_TERRAIN_TYPE_BIT;
            }
            changed += usize::from(cell.terrain_type != old);
        }
        Some(changed)
    }

    /// Commit the physical-particle traversal's ordered Section-10 writes.
    /// Collision and F800 already consumed this same journal as a synchronous
    /// overlay while borrowing terrain; the cache now receives the exact final
    /// bytes before other world callbacks and presentation can observe them.
    pub fn apply_particle_terrain_mutations(
        &mut self,
        writes: &[crate::world_fx::ParticleTerrainMutation],
    ) -> Option<usize> {
        let terrain = self.level_terrain_mut()?;
        let mut changed = 0;
        for &write in writes {
            let cell = write.cell();
            let index =
                usize::from(cell[0]) * v2k_formats::terrain::GRID_SIZE + usize::from(cell[1]);
            let Some(cell) = terrain.cells.get_mut(index) else {
                continue;
            };
            let old = cell.terrain_type;
            cell.terrain_type = write.apply(old);
            changed += usize::from(cell.terrain_type != old);
        }
        self.level_terrain_presentation_dirty |= changed != 0;
        Some(changed)
    }

    /// Apply `FUN_00427950`'s already-burned kind-10 transition.
    ///
    /// Retail first calls `FUN_004337E0(..., 0)`, clearing only terrain-type
    /// bit `0x08`, and then `FUN_004338C0`, which increments the live
    /// Section-10 attribute byte with byte wrapping. Keeping both writes on
    /// the current level allocation makes the new descriptor/model visible to
    /// every renderer and collision consumer in the same frame.
    pub fn apply_burned_kind_10_transition(&mut self, cell: [u8; 2]) -> bool {
        let Some(terrain) = self
            .layers
            .iter_mut()
            .rev()
            .find(|layer| layer.kind == LayerKind::Level)
            .and_then(|layer| layer.state.terrain.as_mut())
        else {
            return false;
        };
        let index = usize::from(cell[0]) * v2k_formats::terrain::GRID_SIZE + usize::from(cell[1]);
        let Some(terrain_cell) = terrain.cells.get_mut(index) else {
            return false;
        };
        terrain_cell.terrain_type &= !0x08;
        terrain_cell.attribute = terrain_cell.attribute.wrapping_add(1);
        self.level_terrain_presentation_dirty = true;
        true
    }

    /// Apply `FUN_00433E30`'s one-shot Main Base abort transform to the live
    /// Section-10 allocation.
    ///
    /// The high three bits of each terrain-type byte are an authored light
    /// value. Retail subtracts five with a zero floor while preserving the low
    /// five material bits. `FUN_00456960` reaches this path once after Main
    /// Base terminal destruction; keeping the latch beside the mutable level
    /// allocation prevents a repeated host event from darkening twice.
    /// Before darkening each cell, retail resolves its nonzero Section-10
    /// attribute through the active Section-9 descriptor table. Every object
    /// kind except 8 receives `FUN_004337E0(..., 1)`, which sets terrain-type
    /// bit 0x08. Kind 10 additionally consumes one shared `Random_Next` sample
    /// and repeats that idempotent call `sample % 6` times. The callback/count
    /// side effects inside `FUN_004337E0` happen only when the bit changes.
    /// This method owns the authoritative terrain bit and exact RNG cadence;
    /// retail's separate global active-count/subscriber notification remains
    /// outside this terrain-state bridge.
    ///
    /// Returns an outcome only for the first application to a loaded terrain.
    /// `None` means either no level terrain is resident or the transform was
    /// already applied.
    pub fn apply_main_base_abort_terrain_transform(
        &mut self,
        mut next_random_u16: impl FnMut() -> u16,
    ) -> Option<MainBaseAbortTerrainOutcome> {
        if self.level_abort_terrain_transformed {
            return None;
        }
        let level_layer_index = self
            .layers
            .iter()
            .rposition(|layer| layer.kind == LayerKind::Level)?;
        let descriptor_kinds = self.layers[level_layer_index]
            .state
            .anim_frames
            .as_ref()
            .map(|table| {
                table
                    .records
                    .iter()
                    .map(|descriptor| descriptor.kind_index)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let terrain = self.layers[level_layer_index].state.terrain.as_mut()?;

        let mut outcome = MainBaseAbortTerrainOutcome {
            darkened_light_cells: 0,
            newly_activated_object_cells: 0,
            object_activation_calls: 0,
            kind_10_rng_draws: 0,
        };
        for cell in &mut terrain.cells {
            if cell.attribute != 0 {
                if let Some(&kind_index) = descriptor_kinds.get(usize::from(cell.attribute)) {
                    if kind_index != 8 {
                        outcome.object_activation_calls += 1;
                        let previous = cell.terrain_type;
                        cell.terrain_type |= 0x08;
                        outcome.newly_activated_object_cells +=
                            usize::from(cell.terrain_type != previous);
                        if kind_index == 10 {
                            outcome.kind_10_rng_draws += 1;
                            outcome.object_activation_calls += usize::from(next_random_u16() % 6);
                        }
                    }
                }
            }
            let old = cell.terrain_type;
            let light = old >> 5;
            let darkened = light.saturating_sub(5);
            cell.terrain_type = (darkened << 5) | (old & 0x1f);
            outcome.darkened_light_cells += usize::from(cell.terrain_type != old);
        }
        self.level_abort_terrain_transformed = true;
        Some(outcome)
    }
    section_accessor!(anim_sound, anim_sound, AnimSoundTable);
    section_accessor!(collision, collision, CollisionModel);
    section_accessor!(level_desc, level, LevelDescriptor);
    section_accessor!(linkage, linkage, LinkageTable);

    /// Get the active menu-graphics OVL (system level 5).
    ///
    /// The selected on-disk display tier shadows PRELOAD's low-resolution
    /// variant. Matching by the assigned system level avoids confusing a
    /// gameplay model pack with menu graphics.
    pub fn menu_graphics_ovl(&self) -> Option<&LevelState> {
        self.system_layer(5)
    }

    /// Global sprite ids required by the current level's type-1 Section-14
    /// descriptor. Section 14 was previously (incorrectly) treated as an
    /// entity-type → model map; entity models actually come from the global
    /// Section-12 type-record pool.
    pub fn sprite_dependencies(&self) -> Option<SpriteDependencies> {
        self.linkage()?.sprite_dependencies()
    }

    /// Layer tagged with the given system level (preload or auxiliary).
    fn system_layer(&self, level: u32) -> Option<&LevelState> {
        self.layers
            .iter()
            .rev()
            .find(|l| l.state.system_level == Some(level))
            .map(|l| &l.state)
    }

    /// Packed signed screen-layout point authored in one system OVL's
    /// Section-1 table.
    ///
    /// The executable appends these tables to `DAT_004FE624`; callers that
    /// already know the owning system level should use the local index here so
    /// an unloaded preceding level cannot shift the lookup. Despite the
    /// section's historical "code fixup" name, the menu and gameplay HUD read
    /// its low/high words as signed `(x, y)` pixel coordinates.
    pub fn system_layout_point(&self, level: u32, local_index: usize) -> Option<(i16, i16)> {
        let raw = *self
            .system_layer(level)?
            .fixup_code
            .as_ref()?
            .entries
            .get(local_index)?;
        Some((raw as u16 as i16, (raw >> 16) as u16 as i16))
    }

    /// One scalar authored in a fixed system OVL's Section-0 table.
    ///
    /// The historic `fixup_data` name describes the loader mechanism, but
    /// several retail subsystems consume these entries directly as dimensions,
    /// radii, and screen rectangles. Using the owning system level plus a local
    /// index preserves the executable's pool boundaries.
    pub fn system_data_value(&self, level: u32, local_index: usize) -> Option<u32> {
        self.system_layer(level)?
            .fixup_data
            .as_ref()?
            .entries
            .get(local_index)
            .copied()
    }

    /// Decode the three radar projection resources authored in one system
    /// OVL's Section 14.
    ///
    /// Unlike the general newest-layer [`Self::linkage`] accessor, this keeps
    /// ownership explicit and propagates malformed-resource errors instead of
    /// silently substituting another level's linkage table.
    pub fn system_radar_projection_tables(
        &self,
        level: u32,
    ) -> v2k_core::Result<RadarProjectionTables> {
        let linkage = self.system_linkage(level).ok_or_else(|| {
            v2k_core::V2kError::section(
                14,
                format!("system level {level} has no Section-14 linkage table"),
            )
        })?;
        linkage.radar_projection_tables()
    }

    /// Section-14 linkage table authored by one fixed system OVL.
    pub fn system_linkage(&self, level: u32) -> Option<&LinkageTable> {
        self.system_layer(level)?.linkage.as_ref()
    }

    /// Section-7 palette authored by one fixed system/resource level.
    ///
    /// Normal runtime rendering should continue to use [`Self::color_palettes`],
    /// which reflects the currently active level stack. Diagnostics that keep
    /// several mutually exclusive biome packs resident at once can use this
    /// accessor to preview a model against its owning pack without allowing the
    /// last loaded pack to colour every model in the browser.
    pub fn system_color_palette(&self, level: u32) -> Option<&[PaletteEntry]> {
        self.system_layer(level)?.color_palettes.as_deref()
    }

    /// Resolve one entry in the executable's cumulative global Section-7
    /// palette-pointer pool.
    ///
    /// Missing mutually exclusive world-style OVLs retain their canonical
    /// holes. Cache insertion priority and residency are deliberately
    /// irrelevant: loading style 6 alone must still resolve its authored base
    /// 327 instead of compacting it behind the master palettes at base 87.
    pub fn global_palette_entry(&self, global_index: usize) -> Option<&PaletteEntry> {
        self.global_palette_entry_with_source(global_index)
            .map(|(_, _, entry)| entry)
    }

    /// Resolve a global palette entry together with its owning system level
    /// and local Section-7 index.
    pub fn global_palette_entry_with_source(
        &self,
        global_index: usize,
    ) -> Option<(u32, usize, &PaletteEntry)> {
        for &(level, base) in GLOBAL_PALETTE_LAYERS {
            let Some(palette) = self.system_color_palette(level) else {
                continue;
            };
            let Some(local_index) = global_index.checked_sub(base) else {
                continue;
            };
            if local_index < palette.len() {
                return Some((level, local_index, &palette[local_index]));
            }
        }
        None
    }

    /// Convert one system OVL's local Section-7 index into its current global
    /// palette-pool index. Returns `None` when the level is not resident or the
    /// local entry is out of range.
    pub fn global_palette_index(&self, system_level: u32, local_index: usize) -> Option<usize> {
        let &(_, base) = GLOBAL_PALETTE_LAYERS
            .iter()
            .find(|&&(level, _)| level == system_level)?;
        let palette = self.system_color_palette(system_level)?;
        (local_index < palette.len()).then(|| base + local_index)
    }

    /// System-level-2 master RGB555 palette (`DAT_004FE63C`). World
    /// background colour indices and other core render colours address this
    /// table directly; they must not be resolved against the active biome's
    /// later 48-entry terrain palette.
    pub fn master_color_palette(&self) -> Option<&[v2k_formats::system::PaletteEntry]> {
        self.system_layer(2)?.color_palettes.as_deref()
    }

    /// Cumulative global string pool, matching the runtime array at
    /// DAT_004FE628: Section 2 of system levels 2-5 concatenated in level
    /// order (L2 ids 0-120, L3 121-302, L4 303-304, L5 305-309). Level 3
    /// must have been loaded as an auxiliary layer (it is not embedded in
    /// PRELOAD.DAT); missing levels contribute nothing, shifting later ids,
    /// so callers should prefer `global_string` with fallback labels.
    pub fn global_strings(&self) -> Vec<&str> {
        let mut pool = Vec::new();
        for level in 2..=5 {
            if let Some(layer) = self.system_layer(level) {
                pool.extend(layer.strings.iter().map(|s| s.as_str()));
            }
        }
        pool
    }

    /// Look up a global string id in the cumulative pool.
    pub fn global_string(&self, id: usize) -> Option<&str> {
        let mut base = 0usize;
        for level in 2..=5 {
            if let Some(layer) = self.system_layer(level) {
                if id < base + layer.strings.len() {
                    return Some(layer.strings[id - base].as_str());
                }
                base += layer.strings.len();
            }
        }
        None
    }

    /// Look up a sprite by its GLOBAL pool index, matching the runtime pool at
    /// DAT_004FE62C. Each Section 3 entry stores its global slot in its
    /// `index` field (verified: biome 6 entries are 1322 + entry_idx), so the
    /// cumulative pool can be searched directly without cumulative bases.
    /// Scans layers newest-first.
    pub fn global_sprite(
        &self,
        global_index: u16,
    ) -> Option<(
        &v2k_formats::sprites::SpriteAtlas,
        &v2k_formats::sprites::SpriteEntry,
    )> {
        for layer in self.layers.iter().rev() {
            if let Some(atlas) = &layer.state.sprites {
                if let Some(entry) = atlas.entries.iter().find(|e| e.index == global_index) {
                    return Some((atlas, entry));
                }
            }
        }
        None
    }

    /// Sorted ids currently addressable through the runtime's global sprite
    /// pool. Newer cache layers win exactly as they do in [`Self::global_sprite`].
    /// This is primarily useful to diagnostics which must browse the same
    /// resources the renderer can actually resolve.
    pub fn global_sprite_ids(&self) -> Vec<u16> {
        let mut seen = HashSet::new();
        let mut ids = Vec::new();
        for layer in self.layers.iter().rev() {
            if let Some(atlas) = &layer.state.sprites {
                for entry in &atlas.entries {
                    if seen.insert(entry.index) {
                        ids.push(entry.index);
                    }
                }
            }
        }
        ids.sort_unstable();
        ids
    }

    /// Section 11 tables forming the GLOBAL sound pool, mirroring the
    /// runtime array at DAT_004FE64C: system level 2 (global ids 0-6) then
    /// level 3 (ids 7-109). All other levels contribute nothing (verified
    /// against the PRELOAD.DAT count matrix). Feed to
    /// `SoundManager::from_tables` in this order.
    pub fn global_sound_tables(&self) -> Vec<&AnimSoundTable> {
        [2u32, 3]
            .iter()
            .filter_map(|&lv| self.system_layer(lv).and_then(|s| s.anim_sound.as_ref()))
            .collect()
    }

    /// Remove per-level world-resource auxiliary layers (system levels 6-11),
    /// keeping persistent auxiliaries like the level-3 common pool. Called on
    /// level load so consecutive loads don't stack stale world-style layers (which
    /// would shadow the new level's palettes and sprites).
    pub fn unload_world_auxiliaries(&mut self) {
        self.layers.retain(|l| {
            !(l.kind == LayerKind::Auxiliary && matches!(l.state.system_level, Some(6..=11)))
        });
    }

    /// Look up a global model id in the fixed Section-8 pool at DAT_004FE640.
    /// Missing biome layers leave their canonical ranges unresolved instead
    /// of compressing every later absolute id toward 324.
    pub fn global_model(&self, id: usize) -> Option<&v2k_formats::models::ModelEntry> {
        for &(level, base) in GLOBAL_MODEL_LAYERS {
            if let Some(layer) = self.system_layer(level) {
                if let Some(models) = &layer.models {
                    if let Some(local_id) = id.checked_sub(base) {
                        if local_id < models.all_entries.len() {
                            return Some(&models.all_entries[local_id]);
                        }
                    }
                }
            }
        }
        None
    }

    /// System/resource level that owns a currently addressable global model.
    /// The lookup follows the same fixed pool ranges as [`Self::global_model`].
    pub fn global_model_system_level(&self, id: usize) -> Option<u32> {
        for &(level, base) in GLOBAL_MODEL_LAYERS {
            let Some(models) = self
                .system_layer(level)
                .and_then(|layer| layer.models.as_ref())
            else {
                continue;
            };
            let Some(local_id) = id.checked_sub(base) else {
                continue;
            };
            if local_id < models.all_entries.len() {
                return Some(level);
            }
        }
        None
    }

    /// Sorted ids currently addressable through the fixed global model pool.
    /// Holes remain holes; ids are never compressed around an unloaded pack.
    pub fn global_model_ids(&self) -> Vec<usize> {
        let mut ids = Vec::new();
        for &(level, base) in GLOBAL_MODEL_LAYERS {
            let Some(models) = self
                .system_layer(level)
                .and_then(|layer| layer.models.as_ref())
            else {
                continue;
            };
            ids.extend(base..base + models.all_entries.len());
        }
        ids
    }

    /// Look up an entity type record in the cumulative Section-12 pool.
    /// System level 2 contributes menu-only types 0-1; level 3 contributes
    /// gameplay types 2-129. This mirrors `DAT_004FE650[type]`.
    pub fn global_entity_type(
        &self,
        entity_type: usize,
    ) -> Option<&v2k_formats::collision::CollisionEntry> {
        let mut base = 0usize;
        for level in [2u32, 3] {
            if let Some(layer) = self.system_layer(level) {
                if let Some(records) = &layer.collision {
                    if entity_type < base + records.entries.len() {
                        return Some(&records.entries[entity_type - base]);
                    }
                    base += records.entries.len();
                }
            }
        }
        None
    }

    /// Materialize the exact type → model-id table used when entities spawn.
    pub fn global_entity_model_table(&self) -> Vec<[u16; 4]> {
        let mut table = Vec::new();
        for level in [2u32, 3] {
            if let Some(layer) = self.system_layer(level) {
                if let Some(records) = &layer.collision {
                    table.extend(records.entries.iter().map(|record| record.model_ids));
                }
            }
        }
        table
    }

    /// Find a global-pool model by embedded name, returning its global id and
    /// entry. Uses the same active cumulative pool as [`Self::global_model`].
    pub fn global_model_by_name(
        &self,
        name: &str,
    ) -> Option<(usize, &v2k_formats::models::ModelEntry)> {
        for &(level, base) in GLOBAL_MODEL_LAYERS {
            if let Some(layer) = self.system_layer(level) {
                if let Some(models) = &layer.models {
                    for (i, e) in models.all_entries.iter().enumerate() {
                        if e.name.as_deref() == Some(name) {
                            return Some((base + i, e));
                        }
                    }
                }
            }
        }
        None
    }

    /// String table accessor — merges strings from all layers.
    ///
    /// Returns strings from highest-priority layers first.
    pub fn strings(&self) -> Vec<&str> {
        let mut result = Vec::new();
        for layer in self.layers.iter().rev() {
            for s in &layer.state.strings {
                result.push(s.as_str());
            }
        }
        result
    }

    /// Summary of what's available in the cache.
    pub fn status(&self) -> String {
        let mut s = String::new();

        writeln!(s, "=== Resource Cache ===").unwrap();

        let preload_count = self
            .layers
            .iter()
            .filter(|l| l.kind == LayerKind::Preload)
            .count();
        let aux_count = self
            .layers
            .iter()
            .filter(|l| l.kind == LayerKind::Auxiliary)
            .count();

        writeln!(s, "  Preload OVLs:  {}", preload_count).unwrap();
        for (i, layer) in self
            .layers
            .iter()
            .filter(|l| l.kind == LayerKind::Preload)
            .enumerate()
        {
            writeln!(
                s,
                "    [{}] {} — {}/15 sections",
                i,
                layer.state.source_path,
                layer.state.populated_count()
            )
            .unwrap();
        }

        if aux_count > 0 {
            writeln!(s, "  Auxiliary OVLs: {}", aux_count).unwrap();
            for (i, layer) in self
                .layers
                .iter()
                .filter(|l| l.kind == LayerKind::Auxiliary)
                .enumerate()
            {
                writeln!(
                    s,
                    "    [{}] {} — {}/15 sections",
                    i,
                    layer.state.source_path,
                    layer.state.populated_count()
                )
                .unwrap();
            }
        }

        match self.level() {
            Some(lv) => writeln!(
                s,
                "  Level:         {} — {}/15 sections",
                lv.source_path,
                lv.populated_count()
            )
            .unwrap(),
            None => writeln!(s, "  Level:         (none)").unwrap(),
        }

        // Show which section types are available after merging
        writeln!(s).unwrap();
        writeln!(s, "  Available sections (merged):").unwrap();
        writeln!(
            s,
            "    fixup_data:     {}",
            if self.fixup_data().is_some() {
                "yes"
            } else {
                "--"
            }
        )
        .unwrap();
        writeln!(
            s,
            "    fixup_code:     {}",
            if self.fixup_code().is_some() {
                "yes"
            } else {
                "--"
            }
        )
        .unwrap();
        writeln!(s, "    strings:        {}", {
            let n = self.strings().len();
            if n > 0 {
                format!("{} entries", n)
            } else {
                "--".into()
            }
        })
        .unwrap();
        writeln!(
            s,
            "    sprites:        {}",
            if self.sprites().is_some() {
                "yes"
            } else {
                "--"
            }
        )
        .unwrap();
        writeln!(
            s,
            "    params:         {}",
            if self.params().is_some() { "yes" } else { "--" }
        )
        .unwrap();
        writeln!(
            s,
            "    display_modes:  {}",
            if self.display_modes().is_some() {
                "yes"
            } else {
                "--"
            }
        )
        .unwrap();
        writeln!(
            s,
            "    fog_gradient:   {}",
            if self.fog_gradient().is_some() {
                "yes"
            } else {
                "--"
            }
        )
        .unwrap();
        writeln!(
            s,
            "    color_palettes: {}",
            if self.color_palettes().is_some() {
                "yes"
            } else {
                "--"
            }
        )
        .unwrap();
        writeln!(
            s,
            "    models:         {}",
            if self.models().is_some() { "yes" } else { "--" }
        )
        .unwrap();
        writeln!(
            s,
            "    anim_frames:    {}",
            if self.anim_frames().is_some() {
                "yes"
            } else {
                "--"
            }
        )
        .unwrap();
        writeln!(
            s,
            "    terrain:        {}",
            if self.terrain().is_some() {
                "yes"
            } else {
                "--"
            }
        )
        .unwrap();
        writeln!(
            s,
            "    anim_sound:     {}",
            if self.anim_sound().is_some() {
                "yes"
            } else {
                "--"
            }
        )
        .unwrap();
        writeln!(
            s,
            "    collision:      {}",
            if self.collision().is_some() {
                "yes"
            } else {
                "--"
            }
        )
        .unwrap();
        writeln!(
            s,
            "    level_desc:     {}",
            if self.level_desc().is_some() {
                "yes"
            } else {
                "--"
            }
        )
        .unwrap();
        writeln!(
            s,
            "    linkage:        {}",
            if self.linkage().is_some() {
                "yes"
            } else {
                "--"
            }
        )
        .unwrap();

        s
    }
}

#[cfg(test)]
mod tests {
    mod terrain_light;

    #[v2k_test_support::retail_test]
    fn native_terrain_radar_initializes_once_per_world_and_restores_on_abort_rollback() {
        let dir = v2k_test_support::retail_dir();
        let mut session = crate::session::GameSession::init(&dir).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(50, 1).unwrap();
        let mut draws = 0;
        let initialized = session
            .cache
            .initialize_level_terrain_radar(&mut || {
                draws += 1;
                1
            })
            .unwrap();
        assert!(initialized > 0);
        assert_eq!(initialized, draws);
        let original = session.cache.level_terrain_radar().unwrap();
        let original_revision = original.revision();
        let original_indices = original.indices().to_vec();
        let original_coverage = original.packed_coverage().to_vec();
        assert_eq!(
            session
                .cache
                .initialize_level_terrain_radar(&mut || panic!("resume repeated raster RNG")),
            Ok(0)
        );
        {
            let mut transaction = session.cache.begin_main_base_abort_resource_transaction();
            // An aborted terrain operation must restore the public raster and
            // its revision, not just keep an initialized allocation present.
            transaction
                .refresh_level_terrain_radar(
                    &[[64 << 8, 235_u16.wrapping_shl(8) as i16]],
                    &mut || 0,
                )
                .unwrap();
            transaction
                .mutate_level_terrain_radar_coverage(
                    [64 << 8, 235_u16.wrapping_shl(8) as i16],
                    crate::gameplay_radar::RadarCoverageChange::Add,
                    &mut || 0,
                )
                .unwrap();
            assert_ne!(
                transaction.level_terrain_radar().unwrap().packed_coverage(),
                original_coverage
            );
            assert_ne!(
                transaction.level_terrain_radar().unwrap().revision(),
                original_revision
            );
        }
        let restored = session.cache.level_terrain_radar().unwrap();
        assert_eq!(restored.revision(), original_revision);
        assert_eq!(restored.indices(), original_indices);
        assert_eq!(restored.packed_coverage(), original_coverage);
        assert_eq!(
            session
                .cache
                .initialize_level_terrain_radar(&mut || panic!("rollback lost persistent raster")),
            Ok(0)
        );
        session.cache.level_terrain_mut().unwrap().cells.clear();
        assert!(session
            .cache
            .mutate_level_terrain_radar_coverage(
                [0, 0],
                crate::gameplay_radar::RadarCoverageChange::Remove,
                &mut || panic!("missing terrain drew RNG"),
            )
            .is_err());
        let retained = session.cache.level_terrain_radar().unwrap();
        assert_eq!(retained.revision(), original_revision);
        assert_eq!(retained.indices(), original_indices);
        assert_eq!(retained.packed_coverage(), original_coverage);
        session.cache.unload_level();
        assert_eq!(
            session
                .cache
                .refresh_level_terrain_radar(&[[0, 0]], &mut || panic!("unloaded world drew RNG")),
            Err(crate::gameplay_radar::TerrainRadarError::Uninitialized)
        );
        assert_eq!(
            session.cache.mutate_level_terrain_radar_coverage(
                [0, 0],
                crate::gameplay_radar::RadarCoverageChange::Add,
                &mut || panic!("unloaded world coverage drew RNG"),
            ),
            Err(crate::gameplay_radar::TerrainRadarError::Uninitialized)
        );
        assert!(session
            .cache
            .initialize_level_terrain_radar(&mut || panic!("invalid world drew RNG"))
            .is_err());
    }

    use super::*;
    use v2k_formats::anim_frames::{ModelSlotPattern, TerrainObjectDescriptor, TerrainObjectTable};
    use v2k_formats::linkage::{LinkageRecord, LinkageTable};
    use v2k_formats::system::{FixupTable, PaletteEntry};
    use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

    fn empty_system_state(system_level: u32) -> LevelState {
        LevelState {
            source_path: format!("system-{system_level}.ovl"),
            system_level: Some(system_level),
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

    fn palette_entry(rgb555: u16) -> PaletteEntry {
        PaletteEntry {
            rgb555,
            r: 0,
            g: 0,
            b: 0,
        }
    }

    fn minimal_radar_linkage() -> LinkageTable {
        const DESCRIPTORS_END: usize = 3 * 28;
        let mut data = vec![0_u8; 169];
        data[DESCRIPTORS_END..DESCRIPTORS_END + 4].copy_from_slice(&2_u32.to_le_bytes());
        data[DESCRIPTORS_END + 4..DESCRIPTORS_END + 8].copy_from_slice(&2_u32.to_le_bytes());
        data[100..104].copy_from_slice(&1_u32.to_le_bytes());
        data[104..108].copy_from_slice(&1_u32.to_le_bytes());
        data[156..160].copy_from_slice(&1_u32.to_le_bytes());
        data[160..164].copy_from_slice(&1_u32.to_le_bytes());
        data[96..100].copy_from_slice(&[1, 2, 3, 4]);
        data[168] = 0x0f;

        LinkageTable {
            records: vec![
                LinkageRecord {
                    record_type: 2,
                    layer_count: 1,
                    entry_count: 4,
                    entry_size: 1,
                    region1_ptr: 84,
                    region2_ptr: 92,
                    region3_ptr: 96,
                },
                LinkageRecord {
                    record_type: 2,
                    layer_count: 8,
                    entry_count: 1,
                    entry_size: 16,
                    region1_ptr: 100,
                    region2_ptr: 108,
                    region3_ptr: 140,
                },
                LinkageRecord {
                    record_type: 2,
                    layer_count: 1,
                    entry_count: 1,
                    entry_size: 1,
                    region1_ptr: 156,
                    region2_ptr: 164,
                    region3_ptr: 168,
                },
            ],
            data,
        }
    }

    fn level_state_with_cell(cell: [u8; 2], terrain_cell: TerrainCell) -> LevelState {
        let mut cells = vec![
            TerrainCell {
                height: 0,
                attribute: 0,
                terrain_type: 0,
            };
            GRID_SIZE * GRID_SIZE
        ];
        cells[usize::from(cell[0]) * GRID_SIZE + usize::from(cell[1])] = terrain_cell;
        LevelState {
            source_path: "test.ovl".into(),
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
            terrain: Some(TerrainGrid {
                header: [0; 5],
                cells,
            }),
            anim_sound: None,
            collision: None,
            level: None,
            linkage: None,
        }
    }

    #[test]
    fn main_base_abort_resource_transaction_rolls_back_during_unwind() {
        let cell = [0x31, 0x47];
        let original = TerrainCell {
            height: 2,
            attribute: 0xb3,
            terrain_type: 0x61,
        };
        let mut cache = ResourceCache::new(Vec::new());
        cache.load_level(level_state_with_cell(cell, original));

        let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut transaction = cache.begin_main_base_abort_resource_transaction();
            assert_eq!(
                transaction.take_level_terrain_object_attribute(cell),
                Some(original.attribute)
            );
            assert!(transaction
                .apply_main_base_abort_terrain_transform(|| 0)
                .is_some());
            panic!("exercise Main Base resource rollback guard");
        }));
        assert!(unwind.is_err());

        let restored = cache
            .level_terrain()
            .unwrap()
            .cell(usize::from(cell[0]), usize::from(cell[1]))
            .unwrap();
        assert_eq!(restored.height, original.height);
        assert_eq!(restored.attribute, original.attribute);
        assert_eq!(restored.terrain_type, original.terrain_type);
        assert!(
            cache
                .apply_main_base_abort_terrain_transform(|| 0)
                .is_some(),
            "unwind must also restore the one-shot transform latch"
        );
    }

    fn terrain_object_table(kinds: &[(u8, u32)]) -> TerrainObjectTable {
        let mut records = vec![
            TerrainObjectDescriptor {
                model_ids: [0; 4],
                kind_index: 0,
                pattern: ModelSlotPattern::Static,
            };
            256
        ];
        for &(attribute, kind_index) in kinds {
            records[usize::from(attribute)].kind_index = kind_index;
        }
        TerrainObjectTable { records }
    }

    #[test]
    fn terrain_object_attribute_take_returns_old_byte_and_clears_only_the_level() {
        let cell = [0x31, 0x47];
        let preload = level_state_with_cell(
            cell,
            TerrainCell {
                height: 1,
                attribute: 0x34,
                terrain_type: 0x40,
            },
        );
        let level = level_state_with_cell(
            cell,
            TerrainCell {
                height: 2,
                attribute: 0xb3,
                terrain_type: 0x61,
            },
        );
        let mut cache = ResourceCache::new(vec![preload]);
        cache.load_level(level);

        assert_eq!(cache.take_level_terrain_object_attribute(cell), Some(0xb3));
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
        assert_eq!(
            cache.preload()[0]
                .terrain
                .as_ref()
                .unwrap()
                .cell(usize::from(cell[0]), usize::from(cell[1]))
                .unwrap()
                .attribute,
            0x34
        );
        assert_eq!(
            cache.take_level_terrain_object_attribute(cell),
            Some(0),
            "an existing zero byte remains distinguishable from missing level terrain"
        );
    }

    #[test]
    fn strict_level_terrain_accessors_never_fall_back_to_auxiliary_or_preload() {
        let cell = [0x31, 0x47];
        let preload = level_state_with_cell(
            cell,
            TerrainCell {
                height: 1,
                attribute: 0x11,
                terrain_type: 0x21,
            },
        );
        let mut auxiliary = level_state_with_cell(
            cell,
            TerrainCell {
                height: 2,
                attribute: 0x22,
                terrain_type: 0x42,
            },
        );
        auxiliary.source_path = "auxiliary.ovl".into();
        let mut cache = ResourceCache::new(vec![preload]);
        cache.add_auxiliary(auxiliary);

        assert_eq!(
            cache
                .terrain()
                .unwrap()
                .cell(usize::from(cell[0]), usize::from(cell[1]))
                .unwrap()
                .height,
            2,
            "the compatibility accessor still follows layered cache priority"
        );
        assert!(cache.level_terrain().is_none());
        assert!(cache.level_terrain_mut().is_none());

        let mut level_without_terrain = level_state_with_cell(
            cell,
            TerrainCell {
                height: 3,
                attribute: 0x33,
                terrain_type: 0x63,
            },
        );
        level_without_terrain.terrain = None;
        cache.load_level(level_without_terrain);

        assert_eq!(
            cache
                .terrain()
                .unwrap()
                .cell(usize::from(cell[0]), usize::from(cell[1]))
                .unwrap()
                .height,
            2,
            "the compatibility accessor may still fall back through the level hole"
        );
        assert!(cache.level_terrain().is_none());
        assert!(cache.level_terrain_mut().is_none());

        let level = level_state_with_cell(
            cell,
            TerrainCell {
                height: 3,
                attribute: 0x33,
                terrain_type: 0x63,
            },
        );
        cache.load_level(level);
        let level_terrain = cache.level_terrain_mut().unwrap();
        level_terrain.cells[usize::from(cell[0]) * GRID_SIZE + usize::from(cell[1])].height = 4;

        assert_eq!(
            cache
                .level_terrain()
                .unwrap()
                .cell(usize::from(cell[0]), usize::from(cell[1]))
                .unwrap()
                .height,
            4
        );
        assert_eq!(
            cache.preload()[0]
                .terrain
                .as_ref()
                .unwrap()
                .cell(usize::from(cell[0]), usize::from(cell[1]))
                .unwrap()
                .height,
            1
        );
    }

    #[test]
    fn clear_terrain_object_attribute_preserves_nonzero_bool_contract() {
        let cell = [0x31, 0x47];
        let level = level_state_with_cell(
            cell,
            TerrainCell {
                height: 2,
                attribute: 0xb3,
                terrain_type: 0x61,
            },
        );
        let mut cache = ResourceCache::new(Vec::new());
        cache.load_level(level);

        assert!(cache.clear_level_terrain_object_attribute(cell));
        assert!(!cache.clear_level_terrain_object_attribute(cell));
    }

    #[test]
    fn terrain_object_attribute_take_does_not_fall_back_when_level_data_is_missing() {
        let cell = [0x31, 0x47];
        let preload = level_state_with_cell(
            cell,
            TerrainCell {
                height: 1,
                attribute: 0x34,
                terrain_type: 0x40,
            },
        );
        let mut cache = ResourceCache::new(vec![preload]);

        assert_eq!(cache.take_level_terrain_object_attribute(cell), None);

        let mut level_without_terrain = level_state_with_cell(
            cell,
            TerrainCell {
                height: 2,
                attribute: 0xb3,
                terrain_type: 0x61,
            },
        );
        level_without_terrain.terrain = None;
        cache.load_level(level_without_terrain);
        assert_eq!(cache.take_level_terrain_object_attribute(cell), None);

        let mut level_without_cell = level_state_with_cell(
            cell,
            TerrainCell {
                height: 2,
                attribute: 0xb3,
                terrain_type: 0x61,
            },
        );
        level_without_cell.terrain.as_mut().unwrap().cells.clear();
        cache.load_level(level_without_cell);
        assert_eq!(cache.take_level_terrain_object_attribute(cell), None);
        assert_eq!(
            cache.preload()[0]
                .terrain
                .as_ref()
                .unwrap()
                .cell(usize::from(cell[0]), usize::from(cell[1]))
                .unwrap()
                .attribute,
            0x34,
            "missing current-level data must never redirect the write to preload terrain"
        );
    }

    #[test]
    fn terrain_type_bits_mutate_only_the_live_level_cell() {
        let cell = [0xff, 0x02];
        let preload = level_state_with_cell(
            cell,
            TerrainCell {
                height: 0x12,
                attribute: 0x34,
                terrain_type: 0x40,
            },
        );
        let level = level_state_with_cell(
            cell,
            TerrainCell {
                height: 0x92,
                attribute: 0xb3,
                terrain_type: 0x61,
            },
        );
        let mut cache = ResourceCache::new(vec![preload]);
        cache.load_level(level);

        assert!(cache.or_level_terrain_type_bits(cell, 0x08));
        let changed = cache
            .level()
            .unwrap()
            .terrain
            .as_ref()
            .unwrap()
            .cell(usize::from(cell[0]), usize::from(cell[1]));
        let changed = changed.unwrap();
        assert_eq!(changed.height, 0x92);
        assert_eq!(changed.attribute, 0xb3);
        assert_eq!(changed.terrain_type, 0x69);

        let preload = cache.preload()[0]
            .terrain
            .as_ref()
            .unwrap()
            .cell(usize::from(cell[0]), usize::from(cell[1]));
        assert_eq!(preload.unwrap().terrain_type, 0x40);
        assert!(!cache.or_level_terrain_type_bits(cell, 0x08));
    }

    #[test]
    fn infection_writes_replay_in_order_and_preserve_other_type_bits() {
        let cell = [0x31, 0x47];
        let level = level_state_with_cell(
            cell,
            TerrainCell {
                height: 2,
                attribute: 0xb3,
                terrain_type: 0x69,
            },
        );
        let mut cache = ResourceCache::new(Vec::new());
        cache.load_level(level);

        let writes = [
            InfectionCellWrite {
                cell,
                infected: true,
            },
            InfectionCellWrite {
                cell,
                infected: true,
            },
            InfectionCellWrite {
                cell,
                infected: false,
            },
        ];
        assert_eq!(cache.apply_level_infection_writes(&writes), Some(2));
        let changed = cache
            .level()
            .unwrap()
            .terrain
            .as_ref()
            .unwrap()
            .cell(usize::from(cell[0]), usize::from(cell[1]))
            .unwrap();
        assert_eq!(changed.height, 2);
        assert_eq!(changed.attribute, 0xb3);
        assert_eq!(
            changed.terrain_type, 0x69,
            "bit 0x10 returns to its authored clear state while unrelated bits survive"
        );
    }

    #[test]
    fn burned_kind_10_transition_clears_only_bit_3_and_advances_attribute() {
        let cell = [0x31, 0x47];
        let preload = level_state_with_cell(
            cell,
            TerrainCell {
                height: 1,
                attribute: 7,
                terrain_type: 0x68,
            },
        );
        let level = level_state_with_cell(
            cell,
            TerrainCell {
                height: 2,
                attribute: 0xff,
                terrain_type: 0x7d,
            },
        );
        let mut cache = ResourceCache::new(vec![preload]);
        cache.load_level(level);

        assert!(cache.apply_burned_kind_10_transition(cell));
        let changed = cache
            .level()
            .unwrap()
            .terrain
            .as_ref()
            .unwrap()
            .cell(usize::from(cell[0]), usize::from(cell[1]))
            .unwrap();
        assert_eq!(changed.height, 2);
        assert_eq!(changed.attribute, 0);
        assert_eq!(changed.terrain_type, 0x75);

        let preload = cache.preload()[0]
            .terrain
            .as_ref()
            .unwrap()
            .cell(usize::from(cell[0]), usize::from(cell[1]))
            .unwrap();
        assert_eq!(preload.attribute, 7);
        assert_eq!(preload.terrain_type, 0x68);
    }

    #[test]
    fn particle_burn_and_infection_journal_preserves_order_and_invalidates_live_terrain() {
        use crate::world_fx::ParticleTerrainMutation as Write;
        let cell = [0x31, 0x47];
        let authored = TerrainCell {
            height: 2,
            attribute: 7,
            terrain_type: 0x65,
        };
        let preload = level_state_with_cell(cell, authored);
        let mut cache = ResourceCache::new(vec![preload]);
        let writes = [
            Write::Infection {
                cell,
                infected: true,
            },
            Write::ImmediateBurn { cell },
            Write::Infection {
                cell,
                infected: false,
            },
            Write::ImmediateBurn { cell },
        ];
        assert_eq!(cache.apply_particle_terrain_mutations(&writes), None);
        assert!(!cache.take_level_terrain_presentation_dirty());
        cache.load_level(level_state_with_cell(cell, authored));
        assert_eq!(cache.apply_particle_terrain_mutations(&writes), Some(3));
        assert!(cache.take_level_terrain_presentation_dirty());
        assert!(!cache.take_level_terrain_presentation_dirty());
        let changed = cache
            .level_terrain()
            .unwrap()
            .cell(usize::from(cell[0]), usize::from(cell[1]))
            .unwrap();
        assert_eq!(changed.terrain_type, 0x6d);
        assert_eq!(changed.height, authored.height);
        assert_eq!(changed.attribute, authored.attribute);
        assert_eq!(
            cache.preload()[0]
                .terrain
                .as_ref()
                .unwrap()
                .cell(usize::from(cell[0]), usize::from(cell[1]))
                .unwrap()
                .terrain_type,
            authored.terrain_type
        );
        assert_eq!(
            cache.apply_particle_terrain_mutations(&[Write::ImmediateBurn { cell }]),
            Some(0)
        );
        assert!(!cache.take_level_terrain_presentation_dirty());
        assert!(cache.apply_burned_kind_10_transition(cell));
        assert!(cache.take_level_terrain_presentation_dirty());
        assert!(cache.or_level_terrain_type_bits(cell, 0x08));
        assert!(cache.take_level_terrain_presentation_dirty());
        assert!(!cache.or_level_terrain_type_bits(cell, 0x08));
        assert!(!cache.take_level_terrain_presentation_dirty());
    }

    #[test]
    fn main_base_abort_transform_darkens_only_live_light_bits_once() {
        let cell = [4, 7];
        let preload = level_state_with_cell(
            cell,
            TerrainCell {
                height: 0x12,
                attribute: 0x34,
                terrain_type: 0xff,
            },
        );
        let mut level = level_state_with_cell(
            cell,
            TerrainCell {
                height: 0x92,
                attribute: 0xb3,
                terrain_type: 0,
            },
        );
        let terrain = level.terrain.as_mut().unwrap();
        for (light, terrain_cell) in terrain.cells.iter_mut().take(8).enumerate() {
            terrain_cell.terrain_type = ((light as u8) << 5) | 0x15;
        }

        let mut cache = ResourceCache::new(vec![preload]);
        cache.load_level(level);

        let outcome = cache
            .apply_main_base_abort_terrain_transform(|| {
                panic!("no kind-10 descriptor should consume RNG")
            })
            .unwrap();
        assert_eq!(outcome.darkened_light_cells, 7);
        assert_eq!(outcome.newly_activated_object_cells, 0);
        assert_eq!(outcome.object_activation_calls, 0);
        assert_eq!(outcome.kind_10_rng_draws, 0);
        let terrain = cache.level().unwrap().terrain.as_ref().unwrap();
        let actual = terrain
            .cells
            .iter()
            .take(8)
            .map(|terrain_cell| terrain_cell.terrain_type)
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            [0x15, 0x15, 0x15, 0x15, 0x15, 0x15, 0x35, 0x55],
            "high lights [0..7] become [0,0,0,0,0,0,1,2] and preserve low bits"
        );
        assert_eq!(
            cache.apply_main_base_abort_terrain_transform(|| 0),
            None,
            "the loaded-world latch prevents cumulative darkening"
        );
        assert_eq!(
            cache.preload()[0]
                .terrain
                .as_ref()
                .unwrap()
                .cell(usize::from(cell[0]), usize::from(cell[1]))
                .unwrap()
                .terrain_type,
            0xff,
            "the mutable world operation must not alter preload resources"
        );

        cache.load_level(level_state_with_cell(
            cell,
            TerrainCell {
                height: 0,
                attribute: 0,
                terrain_type: 0xf3,
            },
        ));
        assert_eq!(
            cache
                .apply_main_base_abort_terrain_transform(|| 0)
                .unwrap()
                .darkened_light_cells,
            1,
            "loading a new world resets the one-shot latch"
        );
        assert_eq!(
            cache
                .level()
                .unwrap()
                .terrain
                .as_ref()
                .unwrap()
                .cell(usize::from(cell[0]), usize::from(cell[1]))
                .unwrap()
                .terrain_type,
            0x53
        );
    }

    #[test]
    fn main_base_abort_transform_activates_attributes_and_preserves_rng_cadence() {
        let regular_cell = [0_u8, 0];
        let skipped_cell = [0_u8, 1];
        let repeated_cell = [0_u8, 2];
        let already_active_cell = [0_u8, 3];
        let mut level = level_state_with_cell(
            regular_cell,
            TerrainCell {
                height: 0,
                attribute: 1,
                terrain_type: 0x60,
            },
        );
        let terrain = level.terrain.as_mut().unwrap();
        terrain.cells[1] = TerrainCell {
            height: 0,
            attribute: 2,
            terrain_type: 0x40,
        };
        terrain.cells[2] = TerrainCell {
            height: 0,
            attribute: 3,
            terrain_type: 0x20,
        };
        terrain.cells[3] = TerrainCell {
            height: 0,
            attribute: 3,
            terrain_type: 0x08,
        };
        level.anim_frames = Some(terrain_object_table(&[(1, 7), (2, 8), (3, 10)]));

        let mut cache = ResourceCache::new(Vec::new());
        cache.load_level(level);
        let mut draws = [11_u16, 4_u16].into_iter();
        let outcome = cache
            .apply_main_base_abort_terrain_transform(|| draws.next().unwrap())
            .unwrap();

        assert_eq!(
            outcome,
            MainBaseAbortTerrainOutcome {
                darkened_light_cells: 3,
                newly_activated_object_cells: 2,
                object_activation_calls: 12,
                kind_10_rng_draws: 2,
            }
        );
        assert!(
            draws.next().is_none(),
            "one draw is consumed per kind-10 cell"
        );
        let terrain = cache.level().unwrap().terrain.as_ref().unwrap();
        assert_eq!(
            terrain
                .cell(usize::from(regular_cell[0]), usize::from(regular_cell[1]))
                .unwrap()
                .terrain_type,
            0x08,
            "ordinary non-kind-8 attributes set bit 0x08 before darkening"
        );
        assert_eq!(
            terrain
                .cell(usize::from(skipped_cell[0]), usize::from(skipped_cell[1]))
                .unwrap()
                .terrain_type,
            0,
            "kind 8 skips the attribute continuation"
        );
        assert_eq!(
            terrain
                .cell(usize::from(repeated_cell[0]), usize::from(repeated_cell[1]))
                .unwrap()
                .terrain_type,
            0x08,
            "kind 10 repeats are visually idempotent"
        );
        assert_eq!(
            terrain
                .cell(
                    usize::from(already_active_cell[0]),
                    usize::from(already_active_cell[1])
                )
                .unwrap()
                .terrain_type,
            0x08,
            "an existing active bit still consumes kind-10 RNG and calls"
        );
    }

    #[test]
    fn main_base_abort_transform_never_borrows_descriptors_from_another_layer() {
        let cell = [9_u8, 12];
        let mut preload = level_state_with_cell(
            cell,
            TerrainCell {
                height: 0,
                attribute: 3,
                terrain_type: 0,
            },
        );
        preload.anim_frames = Some(terrain_object_table(&[(3, 10)]));
        let level = level_state_with_cell(
            cell,
            TerrainCell {
                height: 0,
                attribute: 3,
                terrain_type: 0,
            },
        );
        let mut cache = ResourceCache::new(vec![preload]);
        cache.load_level(level);

        let outcome = cache
            .apply_main_base_abort_terrain_transform(|| {
                panic!("a descriptor from PRELOAD must not drive live level state")
            })
            .unwrap();

        assert_eq!(outcome.newly_activated_object_cells, 0);
        assert_eq!(outcome.object_activation_calls, 0);
        assert_eq!(outcome.kind_10_rng_draws, 0);
        assert_eq!(
            cache
                .level()
                .unwrap()
                .terrain
                .as_ref()
                .unwrap()
                .cell(usize::from(cell[0]), usize::from(cell[1]))
                .unwrap()
                .terrain_type,
            0
        );
    }

    #[test]
    fn system_layout_point_decodes_signed_section_one_words() {
        let state = LevelState {
            source_path: "layout.ovl".into(),
            system_level: Some(3),
            fixup_data: None,
            fixup_code: Some(FixupTable {
                entries: vec![0xfff9_0007, 0x0002_ffd8],
            }),
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
        };
        let cache = ResourceCache::new(vec![state]);

        assert_eq!(cache.system_layout_point(3, 0), Some((7, -7)));
        assert_eq!(cache.system_layout_point(3, 1), Some((-40, 2)));
        assert_eq!(cache.system_layout_point(3, 2), None);
        assert_eq!(cache.system_layout_point(2, 0), None);
    }

    #[test]
    fn system_data_values_keep_section_zero_local_indices() {
        let mut state = empty_system_state(3);
        state.fixup_data = Some(FixupTable {
            entries: vec![96, 96, 4, 5, 3, 50],
        });
        let cache = ResourceCache::new(vec![state]);

        assert_eq!(cache.system_data_value(3, 0), Some(96));
        assert_eq!(cache.system_data_value(3, 5), Some(50));
        assert_eq!(cache.system_data_value(3, 6), None);
        assert_eq!(cache.system_data_value(2, 0), None);
    }

    #[test]
    fn selected_auxiliary_system_tier_overrides_preload_fallback() {
        let mut low = empty_system_state(5);
        low.fixup_data = Some(FixupTable { entries: vec![32] });
        let mut high = empty_system_state(5);
        high.fixup_data = Some(FixupTable { entries: vec![64] });

        let mut cache = ResourceCache::new(vec![low]);
        assert_eq!(cache.system_data_value(5, 0), Some(32));

        cache.add_auxiliary(high);
        assert_eq!(cache.system_data_value(5, 0), Some(64));
    }

    #[test]
    fn system_radar_projection_decodes_only_the_owning_level() {
        let mut radar = empty_system_state(3);
        radar.linkage = Some(minimal_radar_linkage());
        let cache = ResourceCache::new(vec![radar]);

        let projection = cache.system_radar_projection_tables(3).unwrap();
        assert_eq!((projection.width, projection.height), (2, 2));
        assert_eq!(projection.shade_grid, [1, 2, 3, 4]);
        assert_eq!(projection.mask_grid, [0x0f]);
        assert!(cache.system_radar_projection_tables(2).is_err());
    }

    #[test]
    fn global_palette_pool_preserves_canonical_holes_and_ignores_cache_order() {
        let mut level_4 = empty_system_state(4);
        level_4.color_palettes = Some(vec![palette_entry(0x0400)]);
        let mut level_2 = empty_system_state(2);
        level_2.color_palettes = Some(vec![palette_entry(0x0200), palette_entry(0x0201)]);
        let mut level_7 = empty_system_state(7);
        level_7.color_palettes = Some(vec![palette_entry(0x0700), palette_entry(0x0701)]);

        // PRELOAD physical order and later auxiliary priority must not reorder
        // the executable's cumulative Section-7 pointer pool.
        let mut cache = ResourceCache::new(vec![level_4, level_2]);
        cache.add_auxiliary(level_7);

        assert_eq!(cache.global_palette_entry(0).unwrap().rgb555, 0x0200);
        assert!(cache.global_palette_entry(2).is_none());
        assert_eq!(cache.global_palette_entry(86).unwrap().rgb555, 0x0400);
        assert_eq!(cache.global_palette_entry(135).unwrap().rgb555, 0x0700);
        assert_eq!(cache.global_palette_entry(136).unwrap().rgb555, 0x0701);
        assert!(cache.global_palette_entry(137).is_none());
        assert_eq!(cache.global_palette_index(2, 1), Some(1));
        assert_eq!(cache.global_palette_index(4, 0), Some(86));
        assert_eq!(cache.global_palette_index(7, 1), Some(136));
        assert_eq!(cache.global_palette_index(7, 2), None);
        assert_eq!(cache.global_palette_index(6, 0), None);
        assert_eq!(
            cache
                .global_palette_entry_with_source(135)
                .map(|(level, local, entry)| (level, local, entry.rgb555)),
            Some((7, 0, 0x0700))
        );
    }
}
