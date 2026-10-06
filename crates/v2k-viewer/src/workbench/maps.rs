//! Section-10 map discovery and diagnostic rasterization.
//!
//! These images expose raw terrain data for reverse-engineering work. They
//! deliberately do not attempt to reproduce the retail world renderer.

use std::path::{Path, PathBuf};
use v2k_formats::terrain::{TerrainGrid, GRID_SIZE};

pub(super) const MAP_SIZE: u32 = GRID_SIZE as u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MapLayer {
    Height,
    TerrainTypes,
    RawChannels,
}

impl MapLayer {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Height => Self::TerrainTypes,
            Self::TerrainTypes => Self::RawChannels,
            Self::RawChannels => Self::Height,
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Height => "HEIGHT",
            Self::TerrainTypes => "FALSE COLOR TYPES",
            Self::RawChannels => "RAW H A T",
        }
    }
}

pub(super) struct MapAsset {
    path: PathBuf,
    grid: TerrainGrid,
    height_range: (u8, u8),
}

impl MapAsset {
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn source_name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("OVL")
    }

    pub(super) fn height_range(&self) -> (u8, u8) {
        self.height_range
    }

    pub(super) fn diagnostic_rgba(&self, layer: MapLayer) -> Vec<u8> {
        diagnostic_rgba(&self.grid, layer)
    }
}

pub(super) fn discover(
    data_dir: &Path,
    initial: Option<&Path>,
) -> Result<Vec<MapAsset>, Box<dyn std::error::Error>> {
    let mut paths = Vec::new();
    if let Some(path) = initial.filter(|path| path.is_file()) {
        paths.push(path.to_path_buf());
    }
    let overlay = data_dir.join("Overlay");
    if overlay.is_dir() {
        let mut discovered: Vec<_> = std::fs::read_dir(overlay)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("ovl"))
            })
            .collect();
        discovered.sort_by_key(|path| path.to_string_lossy().to_ascii_lowercase());
        for path in discovered {
            if !paths.iter().any(|existing| same_path(existing, &path)) {
                paths.push(path);
            }
        }
    }

    let mut maps = Vec::new();
    for path in paths {
        let Ok(data) = std::fs::read(&path) else {
            continue;
        };
        let Ok(ovl) = v2k_formats::ovl::OvlFile::parse(&data) else {
            continue;
        };
        let Ok(grid) = v2k_formats::sections::parse_terrain(&ovl) else {
            continue;
        };
        let height_range = grid.height_range();
        maps.push(MapAsset {
            path,
            grid,
            height_range,
        });
    }
    Ok(maps)
}

pub(super) fn same_path(left: &Path, right: &Path) -> bool {
    match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn diagnostic_rgba(grid: &TerrainGrid, layer: MapLayer) -> Vec<u8> {
    let mut rgba = vec![0u8; GRID_SIZE * GRID_SIZE * 4];
    for z in 0..GRID_SIZE {
        for x in 0..GRID_SIZE {
            let cell = grid.cell(x, z).expect("bounded terrain cell");
            let [red, green, blue] = match layer {
                MapLayer::Height => [cell.height; 3],
                MapLayer::TerrainTypes => terrain_type_color(cell.terrain_type, cell.height),
                MapLayer::RawChannels => [cell.height, cell.attribute, cell.terrain_type],
            };
            let offset = (z * GRID_SIZE + x) * 4;
            rgba[offset..offset + 4].copy_from_slice(&[red, green, blue, 255]);
        }
    }
    rgba
}

fn terrain_type_color(terrain_type: u8, height: u8) -> [u8; 3] {
    // Deliberate false colour for distinguishing raw type ids. This is a
    // diagnostic classification, not a claim about retail terrain colour.
    let seed = terrain_type as u32;
    let base = [
        ((seed * 73 + 41) & 0xff) as u8,
        ((seed * 151 + 89) & 0xff) as u8,
        ((seed * 199 + 17) & 0xff) as u8,
    ];
    let light = 0.45 + height as f32 / 255.0 * 0.55;
    [
        (base[0] as f32 * light) as u8,
        (base[1] as f32 * light) as u8,
        (base[2] as f32 * light) as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::terrain::TerrainCell;

    #[test]
    fn map_layers_cycle_in_toolbar_order() {
        assert_eq!(MapLayer::Height.next(), MapLayer::TerrainTypes);
        assert_eq!(MapLayer::TerrainTypes.next(), MapLayer::RawChannels);
        assert_eq!(MapLayer::RawChannels.next(), MapLayer::Height);
    }

    #[test]
    fn terrain_type_coloring_is_stable_and_height_sensitive() {
        assert_eq!(terrain_type_color(7, 100), terrain_type_color(7, 100));
        assert_ne!(terrain_type_color(7, 10), terrain_type_color(7, 240));
        assert_ne!(terrain_type_color(7, 100), terrain_type_color(8, 100));
    }

    #[test]
    fn raw_channel_raster_preserves_section_10_axis_order() {
        let mut cells = vec![
            TerrainCell {
                height: 0,
                attribute: 0,
                terrain_type: 0,
            };
            GRID_SIZE * GRID_SIZE
        ];
        cells[3 * GRID_SIZE + 5] = TerrainCell {
            height: 17,
            attribute: 23,
            terrain_type: 42,
        };
        let grid = TerrainGrid {
            header: [0; 5],
            cells,
        };

        let rgba = diagnostic_rgba(&grid, MapLayer::RawChannels);
        let output_offset = (5 * GRID_SIZE + 3) * 4;
        assert_eq!(&rgba[output_offset..output_offset + 4], &[17, 23, 42, 255]);
    }
}
