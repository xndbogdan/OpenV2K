//! Section dispatch — high-level helpers for loading section data from OVL files.

use crate::anim_frames::{self, AnimFrameTable, TerrainObjectTable};
use crate::anim_sound::{self, AnimSoundTable};
use crate::collision::{self, CollisionModel};
use crate::levels::{self, LevelDescriptor};
use crate::linkage::{self, LinkageTable};
use crate::models::{self, ModelCollection};
use crate::ovl::OvlFile;
use crate::palette::ShadePalette;
use crate::params::{self, ParamTable};
use crate::sprites::{self, SpriteAtlas};
use crate::strings;
use crate::system::{self, DisplayMode, FixupTable, FogGradientEntry, PaletteEntry};
use crate::terrain::{self, TerrainGrid};
use v2k_core::{Result, V2kError};

/// Section indices (matching V2000's 15-section layout).
pub mod idx {
    pub const POINTER_FIXUP_DATA: usize = 0;
    pub const POINTER_FIXUP_CODE: usize = 1;
    pub const STRING_TABLE: usize = 2;
    pub const SPRITE_ATLAS: usize = 3;
    pub const PARAM_TABLES: usize = 4;
    pub const DISPLAY_MODES: usize = 5;
    pub const FOG_GRADIENT: usize = 6;
    pub const COLOR_PALETTES: usize = 7;
    pub const SPRITE_METADATA: usize = 8;
    pub const ANIM_FRAMES: usize = 9;
    pub const TERRAIN_HEIGHTMAP: usize = 10;
    pub const ANIM_SOUND: usize = 11;
    pub const COLLISION_MODELS: usize = 12;
    pub const LEVEL_DESCRIPTORS: usize = 13;
    pub const ENTITY_LINKAGE: usize = 14;
}

// ── Existing helpers ────────────────────────────────────────────────────────

/// Extract strings from Section 2 of an OVL file.
pub fn extract_strings(ovl: &OvlFile) -> Vec<String> {
    match ovl.section(idx::STRING_TABLE) {
        Some(sec) if !sec.data.is_empty() => strings::extract_strings(&sec.data),
        _ => Vec::new(),
    }
}

/// Parse the sprite atlas from Section 3 of an OVL file.
pub fn parse_sprites(ovl: &OvlFile) -> Result<SpriteAtlas> {
    let sec = ovl
        .section(idx::SPRITE_ATLAS)
        .ok_or_else(|| V2kError::section(3, "section 3 not found"))?;

    if sec.data.is_empty() {
        return Err(V2kError::section(3, "section 3 is empty"));
    }

    sprites::parse_section3(sec.header_value, &sec.data)
}

/// Try to extract a shade palette from Section 3's entry data.
/// Returns a grayscale fallback if the OVL has no palette data.
pub fn extract_palette(ovl: &OvlFile) -> Result<ShadePalette> {
    let atlas = parse_sprites(ovl)?;
    if atlas.entries.is_empty() {
        return Err(V2kError::section(3, "no sprite entries"));
    }
    let first = &atlas.entries[0];
    crate::palette::parse_palette(&atlas.entry_data, first.pal_offset as usize)
}

// ── Section 0 & 1: Pointer fixup tables ─────────────────────────────────────

/// Parse Section 0 (data pointer fixup table).
pub fn parse_fixup_data(ovl: &OvlFile) -> Option<FixupTable> {
    let sec = ovl.section(idx::POINTER_FIXUP_DATA)?;
    if sec.data.is_empty() {
        return None;
    }
    Some(system::parse_fixup_table(&sec.data))
}

/// Parse Section 1 (code pointer fixup table).
pub fn parse_fixup_code(ovl: &OvlFile) -> Option<FixupTable> {
    let sec = ovl.section(idx::POINTER_FIXUP_CODE)?;
    if sec.data.is_empty() {
        return None;
    }
    Some(system::parse_fixup_table(&sec.data))
}

// ── Section 4: Parameter lookup tables ──────────────────────────────────────

/// Parse Section 4 parameter tables.
pub fn parse_params(ovl: &OvlFile) -> Result<ParamTable> {
    let sec = ovl
        .section(idx::PARAM_TABLES)
        .ok_or_else(|| V2kError::section(4, "section 4 not found"))?;
    if sec.data.is_empty() {
        return Err(V2kError::section(4, "section 4 is empty"));
    }
    params::parse_params(&sec.data)
}

// ── Section 5: Display modes ────────────────────────────────────────────────

/// Parse Section 5 display modes.
pub fn parse_display_modes(ovl: &OvlFile) -> Result<Vec<DisplayMode>> {
    let sec = ovl
        .section(idx::DISPLAY_MODES)
        .ok_or_else(|| V2kError::section(5, "section 5 not found"))?;
    if sec.data.is_empty() {
        return Err(V2kError::section(5, "section 5 is empty"));
    }
    system::parse_display_modes(&sec.data)
}

// ── Section 6: render shade lookup (legacy fog_gradient API) ────────────────

/// Parse the Section 6 render shade lookup table.
pub fn parse_fog_gradient(ovl: &OvlFile) -> Result<Vec<FogGradientEntry>> {
    let sec = ovl
        .section(idx::FOG_GRADIENT)
        .ok_or_else(|| V2kError::section(6, "section 6 not found"))?;
    if sec.data.is_empty() {
        return Err(V2kError::section(6, "section 6 is empty"));
    }
    system::parse_fog_gradient(&sec.data)
}

// ── Section 7: Color palettes ───────────────────────────────────────────────

/// Parse Section 7 RGB555 color palettes.
pub fn parse_color_palettes(ovl: &OvlFile) -> Result<Vec<PaletteEntry>> {
    let sec = ovl
        .section(idx::COLOR_PALETTES)
        .ok_or_else(|| V2kError::section(7, "section 7 not found"))?;
    if sec.data.is_empty() {
        return Err(V2kError::section(7, "section 7 is empty"));
    }
    system::parse_color_palettes(&sec.data)
}

// ── Section 8: Model sub-blocks ─────────────────────────────────────────────

/// Parse Section 8 model sub-blocks (3D mesh data).
///
/// Section 8 is a chain of FGDK sub-blocks holding model entries with
/// 12-byte headers; geometry is decoded by interpreting each entry's command
/// stream (see `models`). The section's `header_value` is the first
/// sub-block's `[u16 alloc][u16 count]` header.
pub fn parse_models(ovl: &OvlFile) -> Result<ModelCollection> {
    let sec = ovl
        .section(idx::SPRITE_METADATA)
        .ok_or_else(|| V2kError::section(8, "section 8 not found"))?;
    if sec.data.is_empty() {
        return Err(V2kError::section(8, "section 8 is empty"));
    }
    models::parse_models_section(&sec.data, sec.header_value)
}

// ── Section 9: Static terrain-object descriptors ────────────────────────────

/// Parse the Section-9 static terrain-object descriptor table.
pub fn parse_terrain_objects(ovl: &OvlFile) -> Result<TerrainObjectTable> {
    let sec = ovl
        .section(idx::ANIM_FRAMES)
        .ok_or_else(|| V2kError::section(9, "section 9 not found"))?;
    if sec.data.is_empty() {
        return Err(V2kError::section(9, "section 9 is empty"));
    }
    anim_frames::parse_terrain_objects(&sec.data)
}

/// Legacy parser name retained for source compatibility.
pub fn parse_anim_frames(ovl: &OvlFile) -> Result<AnimFrameTable> {
    parse_terrain_objects(ovl)
}

// ── Section 10: Terrain heightmap ───────────────────────────────────────────

/// Parse Section 10 terrain heightmap grid.
pub fn parse_terrain(ovl: &OvlFile) -> Result<TerrainGrid> {
    let sec = ovl
        .section(idx::TERRAIN_HEIGHTMAP)
        .ok_or_else(|| V2kError::section(10, "section 10 not found"))?;
    if sec.data.is_empty() {
        return Err(V2kError::section(10, "section 10 is empty"));
    }
    terrain::parse_terrain(&sec.data)
}

// ── Section 11: Animation/sound data ────────────────────────────────────────

/// Parse Section 11 sound-effect entries.
pub fn parse_anim_sound(ovl: &OvlFile) -> Result<AnimSoundTable> {
    let sec = ovl
        .section(idx::ANIM_SOUND)
        .ok_or_else(|| V2kError::section(11, "section 11 not found"))?;
    if sec.data.is_empty() {
        return Err(V2kError::section(11, "section 11 is empty"));
    }
    anim_sound::parse_anim_sound(&sec.data)
}

// ── Section 12: Collision models ────────────────────────────────────────────

/// Parse Section 12 collision volume models.
pub fn parse_collision(ovl: &OvlFile) -> Result<CollisionModel> {
    let sec = ovl
        .section(idx::COLLISION_MODELS)
        .ok_or_else(|| V2kError::section(12, "section 12 not found"))?;
    if sec.data.is_empty() {
        return Err(V2kError::section(12, "section 12 is empty"));
    }
    collision::parse_collision_section(&sec.data)
}

// ── Section 13: Level descriptors ───────────────────────────────────────────

/// Parse Section 13 level descriptor.
pub fn parse_level(ovl: &OvlFile) -> Result<LevelDescriptor> {
    let sec = ovl
        .section(idx::LEVEL_DESCRIPTORS)
        .ok_or_else(|| V2kError::section(13, "section 13 not found"))?;
    if sec.data.is_empty() {
        return Err(V2kError::section(13, "section 13 is empty"));
    }
    levels::parse_level(&sec.data)
}

// ── Section 14: Entity linkage ──────────────────────────────────────────────

/// Parse Section 14 entity linkage records.
///
/// `count` is the number of 28-byte records (from PRELOAD.DAT).
/// Pass 0 to infer from data size.
pub fn parse_linkage(ovl: &OvlFile, count: usize) -> Result<LinkageTable> {
    let sec = ovl
        .section(idx::ENTITY_LINKAGE)
        .ok_or_else(|| V2kError::section(14, "section 14 not found"))?;
    if sec.data.is_empty() {
        return Err(V2kError::section(14, "section 14 is empty"));
    }
    linkage::parse_linkage(&sec.data, count)
}
