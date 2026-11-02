//! Class16 Generate Flares: Type36's Primary, `FUN_0040B8B0 -> FUN_00406300`.
//!
//! The task has no timeout and no private data. Each visit of
//! `FUN_00406390` draws a point around the player (`FUN_00456F90`): X up to
//! 0x1000 either side, Z from 0x400 behind to 0xBFF ahead. A point on a
//! material whose ground response selector is 7, or over a cell under the
//! sea, becomes a still class-47 flare on the terrain or the (waving) water
//! surface, owned by the generator.

use v2k_formats::terrain::TerrainGrid;

/// `FUN_00406390`'s flare particle class.
pub const FLARE_PARTICLE_CLASS: u8 = 47;

/// The level inputs `FUN_00406390` reads besides the terrain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlareWorld {
    /// `FUN_0042EA30`'s ground response map (Section-13 `+0xA8..`).
    pub ground_response_selectors: [u8; 8],
    /// `DAT_004FECE4`, Section-13 `+0x84`.
    pub waves_enabled: bool,
    /// The 50-Hz clock `DAT_004FED60` the waves read.
    pub wave_tick: i32,
}

/// One visit's two draws and, on selector-7 ground or under the sea, the
/// flare's birth point.
pub fn flare_site(
    player_position_raw: [i16; 3],
    terrain: &TerrainGrid,
    world: FlareWorld,
    next_random: &mut impl FnMut() -> u16,
) -> Option<[i16; 3]> {
    let x = (player_position_raw[0] as u16)
        .wrapping_add(((next_random() >> 3) & 0x1FFF).wrapping_sub(0x1000));
    let z = (player_position_raw[2] as u16)
        .wrapping_add(((next_random() >> 4) & 0xFFF).wrapping_sub(0x400));
    let sea = terrain.header[0] >> 8;
    // The material reads the nearest cell; the depth test the containing one.
    let material = terrain
        .cell(
            usize::from(x.wrapping_add(0x80) >> 8),
            usize::from(z.wrapping_add(0x80) >> 8),
        )
        .map_or(0, |cell| cell.terrain_type & 7);
    let depth = terrain
        .cell(usize::from(x >> 8), usize::from(z >> 8))
        .map_or(0, |cell| i32::from(cell.height as i8) * 0x20);
    if world.ground_response_selectors[usize::from(material)] != 7 && depth >= sea {
        return None;
    }
    let (x, z) = (x as i16, z as i16);
    let ground = terrain.bilinear_height_raw(x, z);
    let y = if world.waves_enabled {
        v2k_formats::terrain::wave_surface_raw(x, z, world.wave_tick, sea as i16, ground)
    } else {
        ground.max(sea as i16)
    };
    Some([x, y, z])
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::terrain::TerrainCell;

    fn terrain(height: i8, terrain_type: u8, sea: i32) -> TerrainGrid {
        TerrainGrid {
            header: [sea << 8, 0, 0, 0, 0],
            cells: vec![
                TerrainCell {
                    height: height as u8,
                    attribute: 0,
                    terrain_type,
                };
                256 * 256
            ],
        }
    }

    fn world(selector7: bool) -> FlareWorld {
        FlareWorld {
            ground_response_selectors: [if selector7 { 7 } else { 0 }; 8],
            waves_enabled: false,
            wave_tick: 0,
        }
    }

    #[test]
    fn flares_rise_from_selector7_ground_or_the_sea_and_nowhere_else() {
        // 0x8000 draws are the window's midpoint: X +0, Z +0x400.
        let mut words = [0x8000u16, 0x8000].into_iter();
        let dry = terrain(10, 0, -100);
        assert_eq!(
            flare_site([0, 0, 0], &dry, world(false), &mut || words.next().unwrap()),
            None
        );
        let mut words = [0x8000u16, 0x8000].into_iter();
        assert_eq!(
            flare_site([0, 0, 0], &dry, world(true), &mut || words.next().unwrap()),
            Some([0, 320, 0x400])
        );
        // A drowned cell: the flare sits on the still sea.
        let wet = terrain(-10, 0, -100);
        let mut words = [0u16, 0xFFFF].into_iter();
        assert_eq!(
            flare_site([0x100, 0, 0x100], &wet, world(false), &mut || words
                .next()
                .unwrap()),
            Some([0x100 - 0x1000, -100, 0x100 + 0xBFF])
        );
    }
}
