use std::path::{Path, PathBuf};

use v2k_core::Result;
use v2k_formats::preload::PreloadDat;

use crate::loader::{load_level, load_ovl};
use crate::resource_cache::ResourceCache;

/// Top-level game state container.
///
/// Replaces the original game's cluster of globals: holds the resource cache
/// (preload system OVLs + current level), the PRELOAD.DAT resource grid,
/// and the data directory path for locating OVL files.
pub struct GameSession {
    pub cache: ResourceCache,
    /// Resource count matrix from PRELOAD.DAT (15 sections x 53 levels).
    pub preload_grid: Vec<Vec<u32>>,
    /// Root data directory containing PRELOAD.DAT and Overlay/.
    pub data_dir: PathBuf,
}

impl GameSession {
    /// Initialize a game session from a V2000 data directory.
    ///
    /// Reads PRELOAD.DAT, parses the 7 embedded OVLs into `LevelState`s,
    /// and creates the `ResourceCache` with the preload base layer.
    pub fn init(data_dir: &Path) -> Result<Self> {
        let preload_path = data_dir.join("PRELOAD.DAT");
        let preload_data = std::fs::read(&preload_path)?;
        let preload = PreloadDat::parse(&preload_data)?;

        let mut preload_states = Vec::with_capacity(preload.embedded_ovls.len());
        for emb in &preload.embedded_ovls {
            let source = format!("PRELOAD.DAT[{}]", emb.index);
            let mut state = load_ovl(&emb.ovl, &source)?;
            // Embedded OVL index → system level. Verified against the
            // PRELOAD.DAT resource-count matrix: indices 0-4 are levels
            // 0, 1, 2, 4, 5 (level 3 — the big common ?X3XX.OVL pool —
            // is loaded from disk, not embedded).
            state.system_level = match emb.index {
                0 => Some(0),
                1 => Some(1),
                2 => Some(2),
                3 => Some(4),
                4 => Some(5),
                _ => None,
            };
            preload_states.push(state);
        }

        Ok(Self {
            cache: ResourceCache::new(preload_states),
            preload_grid: preload.grid,
            data_dir: data_dir.to_path_buf(),
        })
    }

    /// Load a level by ID and variant into the cache ([`prepare_level`]).
    pub fn load_level_by_id(&mut self, level_id: u32, variant: u32) -> Result<()> {
        let prepared = prepare_level(&self.data_dir, level_id, variant)?;
        self.install_level(prepared);
        Ok(())
    }

    /// Install a level [`prepare_level`] read, replacing the resident one.
    pub fn install_level(&mut self, prepared: PreparedLevel) {
        // Drop world-resource auxiliaries from any previous level so they don't
        // shadow this level's palettes and sprites.
        self.cache.unload_world_auxiliaries();
        if let Some(auxiliary) = prepared.auxiliary {
            self.add_auxiliary(auxiliary);
        }
        self.cache.load_level(prepared.level);
    }

    /// Load any OVL by level ID and add it as an auxiliary layer.
    ///
    /// Auxiliary layers sit above preload but below the gameplay level
    /// in priority. Use this for OVLs that provide shared resources.
    pub fn load_auxiliary_ovl(&mut self, level_id: u32, variant: u32) -> Result<()> {
        let auxiliary = prepare_auxiliary_ovl(&self.data_dir, level_id, variant)?;
        self.add_auxiliary(auxiliary);
        Ok(())
    }

    fn add_auxiliary(&mut self, auxiliary: PreparedAuxiliary) {
        match auxiliary.receipt {
            Some(receipt) => self
                .cache
                .add_auxiliary_with_layout_receipt(auxiliary.state, receipt),
            None => self.cache.add_auxiliary(auxiliary.state),
        }
    }

    /// Read/validate both high layout candidates without touching resident
    /// resources. Dependent snapshots can be prepared through the result's
    /// SystemLayoutSource implementation before the cache commit.
    pub fn prepare_high_system_layout_refresh(
        &self,
        tier: crate::system_layout::HighSystemLayoutTier,
    ) -> std::result::Result<
        crate::system_layout::PreparedHighSystemLayouts,
        crate::system_layout::SystemLayoutRefreshError,
    > {
        use crate::system_layout::{SystemLayoutOrigin, SystemLayoutRefreshError as Error};
        let mut candidates = Vec::with_capacity(2);
        for system_level in [2, 3] {
            let path = self.data_dir.join("Overlay").join(format!(
                "{}X{}XX.OVL",
                tier.variant(),
                system_level
            ));
            let bytes = std::fs::read(&path).map_err(|source| Error::Read {
                system_level,
                path: path.clone(),
                source,
            })?;
            let ovl = v2k_formats::ovl::OvlFile::parse(&bytes).map_err(|source| Error::Parse {
                system_level,
                path: path.clone(),
                source,
            })?;
            candidates.push((
                ovl,
                SystemLayoutOrigin {
                    tier,
                    source_path: path,
                },
            ));
        }
        self.cache
            .prepare_high_system_layouts(candidates.try_into().unwrap())
    }

    /// Load an OVL by level ID without adding it to the cache.
    ///
    /// Returns the parsed LevelState for direct resource access.
    pub fn load_ovl_by_id(&self, level_id: u32, variant: u32) -> Result<crate::level::LevelState> {
        let filename = format!("{}X{}XX.OVL", variant, level_id);
        let path = self.data_dir.join("Overlay").join(&filename);
        load_level(&path)
    }

    /// Load an arbitrary OVL file into the cache as the current level.
    pub fn load_level_from_path(&mut self, path: &Path) -> Result<()> {
        let state = load_level(path)?;
        self.cache.load_level(state);
        Ok(())
    }

    /// Compare preload grid expectations vs actual loaded data.
    ///
    /// Reports which system levels (0-12) have resources in the grid
    /// and whether those resources are covered by the 7 embedded OVLs.
    pub fn system_coverage_report(&self) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        writeln!(s, "=== System OVL Coverage ===").unwrap();

        let section_names = [
            "fixup_data",
            "fixup_code",
            "strings",
            "sprites",
            "params",
            "display_modes",
            "fog",
            "palettes",
            "models",
            "anim_frames",
            "terrain",
            "anim_sound",
            "collision",
            "level_desc",
            "linkage",
        ];

        let preload_layers = self.cache.preload();
        writeln!(s, "Preload layers: {} embedded OVLs", preload_layers.len()).unwrap();

        // What sections the preload layers actually have
        let mut covered_sections: Vec<Vec<bool>> = Vec::new();
        for layer in &preload_layers {
            covered_sections.push(vec![
                layer.fixup_data.is_some(),
                layer.fixup_code.is_some(),
                !layer.strings.is_empty(),
                layer.sprites.is_some(),
                layer.params.is_some(),
                layer.display_modes.is_some(),
                layer.fog_gradient.is_some(),
                layer.color_palettes.is_some(),
                layer.models.is_some(),
                layer.anim_frames.is_some(),
                layer.terrain.is_some(),
                layer.anim_sound.is_some(),
                layer.collision.is_some(),
                layer.level.is_some(),
                layer.linkage.is_some(),
            ]);
        }

        // Summarize: which section types are present across all preload layers
        writeln!(s).unwrap();
        writeln!(s, "Preload section coverage:").unwrap();
        for (sec, name) in section_names.iter().enumerate() {
            let present = covered_sections.iter().any(|layer| layer[sec]);
            writeln!(
                s,
                "  [{:>2}] {:<14} {}",
                sec,
                name,
                if present { "YES" } else { "--" }
            )
            .unwrap();
        }

        // Grid expectations for system levels (0-12)
        writeln!(s).unwrap();
        writeln!(s, "Grid expectations (system levels 0-12):").unwrap();
        let n_levels = self.preload_grid.first().map(|r| r.len()).unwrap_or(0);
        let system_end = n_levels.min(13);
        for level in 0..system_end {
            let mut expected: Vec<usize> = Vec::new();
            for sec in 0..15.min(self.preload_grid.len()) {
                if self.preload_grid[sec].get(level).copied().unwrap_or(0) > 0 {
                    expected.push(sec);
                }
            }
            if !expected.is_empty() {
                let names: Vec<&str> = expected
                    .iter()
                    .map(|&i| section_names.get(i).copied().unwrap_or("?"))
                    .collect();
                writeln!(s, "  Level {:>2}: [{}]", level, names.join(", ")).unwrap();
            }
        }

        s
    }
}

/// Convert Section-13 `+0x48` into the exact auxiliary overlay requested by
/// `FUN_0042E570`. Values outside the six authored world styles are ignored.
pub fn world_resource_level(world_style: u32) -> Option<u32> {
    (1..=6).contains(&world_style).then_some(5 + world_style)
}

#[cfg(test)]
mod tests {
    use super::world_resource_level;

    #[test]
    fn section13_world_style_selects_the_retail_resource_overlay() {
        assert_eq!(world_resource_level(1), Some(6));
        assert_eq!(world_resource_level(6), Some(11));
        assert_eq!(world_resource_level(0), None);
        assert_eq!(world_resource_level(7), None);
    }
}

/// A level's overlays, read and parsed without touching the cache, so the
/// read can run beside the loading card's redraws.
pub struct PreparedLevel {
    level: crate::level::LevelState,
    auxiliary: Option<PreparedAuxiliary>,
}

/// An auxiliary overlay, read and parsed.
pub struct PreparedAuxiliary {
    state: crate::level::LevelState,
    receipt: Option<crate::system_layout::HighSystemLayerReceipt>,
}

/// Read a level by ID and variant: `Overlay/{variant}X{level_id}XX.OVL`
/// following the V2000 naming convention (e.g. `1X3XX.OVL` for level 3,
/// `1X13XX.OVL` for level 13, high-resolution variant 1). Variant 0 is the
/// explicit 320x240 low-resolution tier. Retail `FUN_00493640` formats the
/// same `overlay\%dx%dxx.ovl` name.
///
/// For game world levels (13-50), also reads the one system OVL selected
/// by Section-13 `+0x48`. `FUN_0042E570` performs exactly
/// `load_overlay(5 + world_style)` after loading the world itself.
pub fn prepare_level(data_dir: &Path, level_id: u32, variant: u32) -> Result<PreparedLevel> {
    // Parse the level OVL first so its Section-13 world-style field can
    // select the matching system resource pack.
    let filename = format!("{}X{}XX.OVL", variant, level_id);
    let path = data_dir.join("Overlay").join(&filename);
    let level = load_level(&path)?;

    // This one pack supplies the matching terrain palettes, transition and
    // shoreline sprites, Section-9 terrain-object descriptors, and model
    // range. Loading a second guessed "biome" pack lets its newest-first
    // sections shadow the authored set (the old Intro2 palms/pyramid bug).
    // Keep the existing graceful fallback for incomplete data sets; full
    // retail data always contains all six resource packs.
    let auxiliary = level
        .level
        .as_ref()
        .map(|l| l.world_style)
        .and_then(world_resource_level)
        .and_then(|resource_level| prepare_auxiliary_ovl(data_dir, resource_level, variant).ok());
    Ok(PreparedLevel { level, auxiliary })
}

/// Read and parse an auxiliary OVL by level ID (a system or biome level).
pub fn prepare_auxiliary_ovl(
    data_dir: &Path,
    level_id: u32,
    variant: u32,
) -> Result<PreparedAuxiliary> {
    let filename = format!("{}X{}XX.OVL", variant, level_id);
    let path = data_dir.join("Overlay").join(&filename);
    let bytes = std::fs::read(&path)?;
    let ovl = v2k_formats::ovl::OvlFile::parse(&bytes)?;
    let mut state = load_ovl(&ovl, &path.display().to_string())?;
    // Auxiliary OVLs are system/biome levels by construction.
    state.system_level = Some(level_id);
    let receipt = crate::system_layout::HighSystemLayoutTier::from_variant(variant)
        .filter(|_| matches!(level_id, 2 | 3))
        .map(|tier| {
            crate::system_layout::HighSystemLayerReceipt::from_loaded_ovl(&ovl, tier, path)
        });
    Ok(PreparedAuxiliary { state, receipt })
}
