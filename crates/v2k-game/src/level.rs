use std::fmt::Write;
use v2k_formats::anim_frames::TerrainObjectTable;
use v2k_formats::anim_sound::AnimSoundTable;
use v2k_formats::collision::CollisionModel;
use v2k_formats::levels::LevelDescriptor;
use v2k_formats::linkage::LinkageTable;
use v2k_formats::models::ModelCollection;
use v2k_formats::params::ParamTable;
use v2k_formats::sprites::SpriteAtlas;
use v2k_formats::system::{DisplayMode, FixupTable, FogGradientEntry, PaletteEntry};
use v2k_formats::terrain::TerrainGrid;

/// All parsed data from a single OVL file.
///
/// Uses `Option<T>` for sections that may not exist — not every OVL has every
/// section (terrain/entities only in game-world levels, params only in certain
/// OVLs, etc.).
pub struct LevelState {
    pub source_path: String,

    /// System level id (0-11) when this OVL is a known system/biome level.
    /// Used to assemble the cumulative global resource pools (strings,
    /// models, sounds) in system-level order, matching the runtime arrays
    /// at DAT_004FE628/DAT_004FE640.
    pub system_level: Option<u32>,

    // Section 0 & 1: pointer fixup tables
    pub fixup_data: Option<FixupTable>,
    pub fixup_code: Option<FixupTable>,

    // Section 2: string table (always present, empty vec if missing)
    pub strings: Vec<String>,

    // Section 3: sprite atlas
    pub sprites: Option<SpriteAtlas>,

    // Section 4: menu sprite-font records (legacy ParamTable type name)
    pub params: Option<ParamTable>,

    // Section 5: display modes
    pub display_modes: Option<Vec<DisplayMode>>,

    // Section 6: fog gradient
    pub fog_gradient: Option<Vec<FogGradientEntry>>,

    // Section 7: color palettes
    pub color_palettes: Option<Vec<PaletteEntry>>,

    // Section 8: 3D models
    pub models: Option<ModelCollection>,

    // Section 9: static terrain-object descriptors (legacy field name).
    pub anim_frames: Option<TerrainObjectTable>,

    // Section 10: terrain heightmap
    pub terrain: Option<TerrainGrid>,

    // Section 11: sound-effect PCM and parametric aliases
    pub anim_sound: Option<AnimSoundTable>,

    // Section 12: collision volumes
    pub collision: Option<CollisionModel>,

    // Section 13: level descriptor
    pub level: Option<LevelDescriptor>,

    // Section 14: entity linkage
    pub linkage: Option<LinkageTable>,
}

impl LevelState {
    /// Count how many of the 15 sections are populated.
    pub fn populated_count(&self) -> usize {
        let mut n = 0;
        if self.fixup_data.is_some() {
            n += 1;
        }
        if self.fixup_code.is_some() {
            n += 1;
        }
        if !self.strings.is_empty() {
            n += 1;
        }
        if self.sprites.is_some() {
            n += 1;
        }
        if self.params.is_some() {
            n += 1;
        }
        if self.display_modes.is_some() {
            n += 1;
        }
        if self.fog_gradient.is_some() {
            n += 1;
        }
        if self.color_palettes.is_some() {
            n += 1;
        }
        if self.models.is_some() {
            n += 1;
        }
        if self.anim_frames.is_some() {
            n += 1;
        }
        if self.terrain.is_some() {
            n += 1;
        }
        if self.anim_sound.is_some() {
            n += 1;
        }
        if self.collision.is_some() {
            n += 1;
        }
        if self.level.is_some() {
            n += 1;
        }
        if self.linkage.is_some() {
            n += 1;
        }
        n
    }

    /// Human-readable section-by-section summary.
    pub fn summary(&self) -> String {
        let mut s = String::new();

        writeln!(s, "=== Level: {} ===", self.source_path).unwrap();
        writeln!(s).unwrap();

        // Section 2: strings
        if self.strings.is_empty() {
            writeln!(s, "  Strings (sec 2):        --").unwrap();
        } else {
            writeln!(
                s,
                "  Strings (sec 2):        {} entries",
                self.strings.len()
            )
            .unwrap();
        }

        // Section 0: fixup data
        match &self.fixup_data {
            Some(f) => {
                writeln!(s, "  Fixup data (sec 0):     {} entries", f.entries.len()).unwrap()
            }
            None => writeln!(s, "  Fixup data (sec 0):     --").unwrap(),
        }

        // Section 1: fixup code
        match &self.fixup_code {
            Some(f) => {
                writeln!(s, "  Fixup code (sec 1):     {} entries", f.entries.len()).unwrap()
            }
            None => writeln!(s, "  Fixup code (sec 1):     --").unwrap(),
        }

        // Section 3: sprites
        match &self.sprites {
            Some(a) => writeln!(
                s,
                "  Sprites (sec 3):        {} entries, {} rects, {}x{} atlas",
                a.entries.len(),
                a.rects.len(),
                a.width,
                a.height,
            )
            .unwrap(),
            None => writeln!(s, "  Sprites (sec 3):        --").unwrap(),
        }

        // Section 4: params
        match &self.params {
            Some(p) => {
                writeln!(s, "  Params (sec 4):         {} records", p.records.len()).unwrap()
            }
            None => writeln!(s, "  Params (sec 4):         --").unwrap(),
        }

        // Section 5: display modes
        match &self.display_modes {
            Some(m) => writeln!(s, "  Display modes (sec 5):  {} modes", m.len()).unwrap(),
            None => writeln!(s, "  Display modes (sec 5):  --").unwrap(),
        }

        // Section 6: fog gradient
        match &self.fog_gradient {
            Some(f) => writeln!(s, "  Fog gradient (sec 6):   {} entries", f.len()).unwrap(),
            None => writeln!(s, "  Fog gradient (sec 6):   --").unwrap(),
        }

        // Section 7: color palettes
        match &self.color_palettes {
            Some(p) => writeln!(s, "  Color palettes (sec 7): {} entries", p.len()).unwrap(),
            None => writeln!(s, "  Color palettes (sec 7): --").unwrap(),
        }

        // Section 8: models
        match &self.models {
            Some(m) => writeln!(
                s,
                "  Models (sec 8):         {} entries, {} verts, {} tris",
                m.all_entries.len(),
                m.total_vertices(),
                m.total_triangles(),
            )
            .unwrap(),
            None => writeln!(s, "  Models (sec 8):         --").unwrap(),
        }

        // Section 9: anim frames
        match &self.anim_frames {
            Some(a) => {
                writeln!(s, "  Anim frames (sec 9):    {} records", a.records.len()).unwrap()
            }
            None => writeln!(s, "  Anim frames (sec 9):    --").unwrap(),
        }

        // Section 10: terrain
        match &self.terrain {
            Some(t) => {
                let size = (t.cells.len() as f64).sqrt() as usize;
                let (h_min, h_max) = t.height_range();
                writeln!(
                    s,
                    "  Terrain (sec 10):       {}x{} grid, height [{}-{}]",
                    size, size, h_min, h_max
                )
                .unwrap();
            }
            None => writeln!(s, "  Terrain (sec 10):       --").unwrap(),
        }

        // Section 11: anim/sound
        match &self.anim_sound {
            Some(a) => writeln!(
                s,
                "  Anim/sound (sec 11):    {} entries ({} blobs, {} aliases)",
                a.entries.len(),
                a.type1_count(),
                a.type5_count(),
            )
            .unwrap(),
            None => writeln!(s, "  Anim/sound (sec 11):    --").unwrap(),
        }

        // Section 12: collision
        match &self.collision {
            Some(c) => {
                writeln!(s, "  Collision (sec 12):     {} entries", c.entries.len()).unwrap()
            }
            None => writeln!(s, "  Collision (sec 12):     --").unwrap(),
        }

        // Section 13: level descriptor
        match &self.level {
            Some(l) => writeln!(
                s,
                "  Level desc (sec 13):    \"{}\" ({} entities, {} campaign records)",
                l.name,
                l.entities.len(),
                l.campaign_records.len(),
            )
            .unwrap(),
            None => writeln!(s, "  Level desc (sec 13):    --").unwrap(),
        }

        // Section 14: linkage
        match &self.linkage {
            Some(l) => {
                writeln!(s, "  Linkage (sec 14):       {} records", l.records.len()).unwrap()
            }
            None => writeln!(s, "  Linkage (sec 14):       --").unwrap(),
        }

        writeln!(s).unwrap();
        writeln!(s, "  Sections populated: {}/15", self.populated_count()).unwrap();

        s
    }
}
