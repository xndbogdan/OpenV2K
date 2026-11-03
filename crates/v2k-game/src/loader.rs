use std::path::Path;
use v2k_core::Result;
use v2k_formats::ovl::OvlFile;
use v2k_formats::sections;

use crate::level::LevelState;

/// Parse all sections from an already-loaded `OvlFile` into a `LevelState`.
///
/// The `source` label is stored for display purposes (file path, embedded index, etc.).
/// Retail's section reader `FUN_00493BF0` likewise runs every section's loader in turn.
pub fn load_ovl(ovl: &OvlFile, source: &str) -> Result<LevelState> {
    Ok(LevelState {
        source_path: source.to_string(),
        system_level: None,
        strings: sections::extract_strings(ovl),
        fixup_data: sections::parse_fixup_data(ovl),
        fixup_code: sections::parse_fixup_code(ovl),
        sprites: sections::parse_sprites(ovl).ok(),
        params: sections::parse_params(ovl).ok(),
        display_modes: sections::parse_display_modes(ovl).ok(),
        fog_gradient: sections::parse_fog_gradient(ovl).ok(),
        color_palettes: sections::parse_color_palettes(ovl).ok(),
        models: sections::parse_models(ovl).ok(),
        anim_frames: sections::parse_terrain_objects(ovl).ok(),
        terrain: sections::parse_terrain(ovl).ok(),
        anim_sound: sections::parse_anim_sound(ovl).ok(),
        collision: sections::parse_collision(ovl).ok(),
        level: sections::parse_level(ovl).ok(),
        linkage: sections::parse_linkage(ovl, 0).ok(),
    })
}

/// Load all sections from an OVL file on disk into a `LevelState`, as retail
/// `FUN_00493BA0` loads one overlay. Retail requests it through
/// `FUN_00493E40` and opens the file with `FUN_00493680`.
///
/// Only hard errors: file I/O failure or invalid OVL container format, which
/// retail's `FUN_00493DB0` turns into an "Error loading overlay" failure.
/// Individual section parse failures are silently converted to `None`.
pub fn load_level(ovl_path: &Path) -> Result<LevelState> {
    let data = std::fs::read(ovl_path)?;
    let ovl = OvlFile::parse(&data)?;
    load_ovl(&ovl, &ovl_path.display().to_string())
}
