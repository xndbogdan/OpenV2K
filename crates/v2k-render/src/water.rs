//! Marching-squares shoreline texturing for the water pass.
//!
//! Ground truth: the engine's water-quad emitter `FUN_004327c0`
//! (bulk_clean/game_logic.c:25624) builds a 4-bit "corner submerged" code per
//! terrain cell and looks up a 16-entry table at EXE VA `0x4CACC8` (stride 8:
//! `{ i32 frame_offset, u32 rotation }`). `frame_offset` (0-4) selects one of
//! five shoreline SHAPE sprites (single-corner / edge / diagonal / 3-corner /
//! full water — they are NOT time-animation frames); `rotation` packs four
//! nibbles assigning each grid corner to a texture slot, orienting the sprite
//! toward the actual submerged corners. The sprite index is
//! `DAT_004db26c + frame_offset`, where `DAT_004db26c` = Section 13 descriptor
//! `terrain_sprite_base (+0x4C)` + 125, computed once at level load
//! (`FUN_00433180`).
//!
//! Corner bit assignment (game_logic.c:25662-25665): for the cell quad with
//! grid corners
//!   c0 = (x, z), c1 = (x, z+1), c2 = (x+1, z+1), c3 = (x+1, z)
//! the code is `b0 | b1<<1 | b2<<2 | b3<<3` where `b_i` = corner c_i submerged.
//! "Submerged" = `terrain_byte * 32 < sea_level_16` (`FUN_004321e0`:25383 —
//! the STATIC terrain height, not the wave-displaced surface).
//!
//! Texture slots are the sprite's corners in fixed order
//! (game_logic.c:25702-25712): slot0 = (0,0) top-left, slot1 = (W,0)
//! top-right, slot2 = (W,H) bottom-right, slot3 = (0,H) bottom-left. Corner
//! `c_i` uses the UV of slot `rotation_nibble[i]`.
//!
//! Table bytes verified directly against V2000.EXE (.data file offset 0xC94C8)
//! and cross-checked by the decoded sprites: shape 0 has water in its (0,H)
//! corner, matching code 8's identity rotation.

use crate::renderer::TextureId;

/// One marching-squares table entry: which of the 5 shoreline shape frames to
/// draw, and which texture slot each of the 4 quad corners maps to.
#[derive(Debug, Clone, Copy)]
pub struct ShoreEntry {
    /// Frame offset 0-4 added to the level's shoreline base sprite.
    pub frame: u8,
    /// `slot[i]` = texture slot (0-3) for grid corner `c_i`.
    pub slot: [u8; 4],
}

/// The 16-entry table at EXE VA 0x4CACC8/0x4CACCC, indexed by the 4-bit
/// corner-submerged code. Entry 0 (fully dry) is never drawn.
///
/// Rotation dwords decode as `slot[i] = (rot >> (8*i)) & 0xf`.
pub const SHORE_TABLE: [ShoreEntry; 16] = [
    ShoreEntry {
        frame: 0,
        slot: [0, 0, 0, 0],
    }, // 0b0000 dry — skipped
    ShoreEntry {
        frame: 0,
        slot: [3, 2, 1, 0],
    }, // 0b0001  rot 0x00112233
    ShoreEntry {
        frame: 0,
        slot: [0, 3, 2, 1],
    }, // 0b0010  rot 0x11223300
    ShoreEntry {
        frame: 1,
        slot: [3, 2, 1, 0],
    }, // 0b0011  rot 0x00112233
    ShoreEntry {
        frame: 0,
        slot: [1, 2, 3, 0],
    }, // 0b0100  rot 0x20130231
    ShoreEntry {
        frame: 2,
        slot: [3, 2, 1, 0],
    }, // 0b0101  rot 0x00112233 (diagonal)
    ShoreEntry {
        frame: 1,
        slot: [1, 2, 3, 0],
    }, // 0b0110  rot 0x20130231
    ShoreEntry {
        frame: 3,
        slot: [3, 2, 1, 0],
    }, // 0b0111  rot 0x00112233
    ShoreEntry {
        frame: 0,
        slot: [0, 1, 2, 3],
    }, // 0b1000  rot 0x33221100
    ShoreEntry {
        frame: 1,
        slot: [3, 0, 1, 2],
    }, // 0b1001  rot 0x02312013
    ShoreEntry {
        frame: 2,
        slot: [0, 3, 2, 1],
    }, // 0b1010  rot 0x11223300 (diagonal)
    ShoreEntry {
        frame: 3,
        slot: [2, 1, 0, 3],
    }, // 0b1011  rot 0x33001122
    ShoreEntry {
        frame: 1,
        slot: [0, 1, 2, 3],
    }, // 0b1100  rot 0x33221100
    ShoreEntry {
        frame: 3,
        slot: [1, 0, 3, 2],
    }, // 0b1101  rot 0x22330011
    ShoreEntry {
        frame: 3,
        slot: [0, 3, 2, 1],
    }, // 0b1110  rot 0x11223300
    ShoreEntry {
        frame: 4,
        slot: [3, 2, 1, 0],
    }, // 0b1111  rot 0x00112233 (full)
];

/// Normalized (u, v) of each texture slot within a frame rect:
/// slot0 top-left, slot1 top-right, slot2 bottom-right, slot3 bottom-left.
pub const SLOT_UV: [(f32, f32); 4] = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];

/// Offset from the Section 13 terrain sprite base to the first of the 5
/// shoreline shape sprites: at level load
/// `DAT_004db26c = base + local_288 + 5`, with `local_288 = 120`
/// (verified empirically across biomes 6, 7 and 9: bases 1616/1994/2734 map
/// to shoreline sets at global sprite indices 1741/2119/2859).
pub const SHORELINE_BASE_OFFSET: u32 = 125;

/// GPU-resident shoreline frame set for one level: the 5 shape sprites packed
/// into a single texture, plus each frame's UV sub-rect (u0, v0, u1, v1).
///
/// The indexed shader preserves the source texel and supplies alpha 0.5: the
/// engine's water span function computes `dst = texel + dst/2`, which is
/// exactly `BlendFunc(ONE, ONE_MINUS_SRC_ALPHA)` with source alpha 0.5. The
/// RGBA fallback stores the same alpha directly. Palette index zero is the
/// transparent dry portion of partial shoreline tiles; the full-water tile
/// is solid and retains index zero as authored surface detail.
#[derive(Debug)]
pub struct WaterFrames {
    /// Brightest-shade RGBA strip used by the fixed-function fallback.
    pub texture: TextureId,
    /// Raw palette indices for the exact indexed water shader.
    pub index_texture: TextureId,
    /// 16 × (5*32) frame-major Section-3 palette rows.
    pub palette_texture: TextureId,
    /// Per-shape UV rect in the packed texture, half-texel inset.
    pub frame_uvs: [[f32; 4]; 5],
    /// Per-shape approximation of palette shade rows 0..=7 relative to the
    /// brightest decoded row. Used by the fixed-function shoreline pass.
    pub shade_scales: [[f32; 8]; 5],
    /// Section-6 mapping from water shade input 0..7 to palette row 0..31.
    pub shade_rows: [u8; 8],
    /// The water pass traverses the same per-level bounded scan as terrain.
    pub scan_columns: u32,
    pub scan_rows: u32,
}

impl WaterFrames {
    pub fn shade_scale(&self, frame: u8, shade: u8) -> f32 {
        self.shade_scales
            .get(frame as usize)
            .and_then(|scales| scales.get(shade.min(7) as usize))
            .copied()
            .unwrap_or(1.0)
    }

    pub fn palette_shade(&self, shade: u8) -> u8 {
        self.shade_rows[shade.min(7) as usize]
    }

    pub fn textures(&self) -> [TextureId; 3] {
        [self.texture, self.index_texture, self.palette_texture]
    }
}

/// Evaluate one integer water-lattice point with the executable's signed
/// wrapping 50 Hz tick. Keeping the tick integral all the way into
/// `water_surface_raw` avoids the loss of individual ticks once a floating
/// elapsed-seconds clock grows beyond `f32` integer precision.
pub(crate) fn surface_y_at_cell(
    x_cell: i32,
    z_cell: i32,
    retail_tick: i32,
    sea_y: f32,
    terrain_y: f32,
) -> f32 {
    if !sea_y.is_finite() || !terrain_y.is_finite() {
        return terrain_y;
    }

    let position_raw = |cell: i32| cell.wrapping_mul(256) as i16;
    let height_raw = |height: f32| ((height * 256.0).round() as i32) as i16;
    f32::from(v2k_formats::terrain::water_surface_raw(
        [position_raw(x_cell), position_raw(z_cell)],
        height_raw(sea_y),
        height_raw(terrain_y),
        v2k_formats::terrain::WaterAnimation::Animated { tick: retail_tick },
    )) / 256.0
}

/// Water-vertex palette shade from `FUN_004321E0`:
/// `clamp(((wave_y - previous_wave_y) >> 5) + 3 + light, 0, 7)`.
/// Heights are signed engine world-Y values (one port cell = 256 units).
pub fn vertex_shade(wave_y: i16, previous_wave_y: i16, light_delta: i8) -> u8 {
    (((wave_y as i32 - previous_wave_y as i32) >> 5) + 3 + light_delta as i32).clamp(0, 7) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_vertex_shade_uses_slope_and_dynamic_light() {
        assert_eq!(vertex_shade(100, 100, 0), 3);
        assert_eq!(vertex_shade(132, 100, 0), 4);
        assert_eq!(vertex_shade(68, 100, 0), 2);
        assert_eq!(vertex_shade(100, 100, 4), 7);
        assert_eq!(vertex_shade(-1000, 1000, 0), 0);
    }

    #[test]
    fn water_surface_keeps_the_retail_tick_integral_past_f32_precision() {
        let exact_tick = 16_777_217;
        let rounded_tick = exact_tick as f32 as i32;
        assert_ne!(exact_tick, rounded_tick);

        let exact = surface_y_at_cell(0x12, 0x34, exact_tick, 16.0, -16.0);
        let rounded = surface_y_at_cell(0x12, 0x34, rounded_tick, 16.0, -16.0);
        assert_ne!(exact, rounded);
    }

    #[test]
    fn water_surface_tick_preserves_signed_wrapping_semantics() {
        let expected = v2k_formats::terrain::wave_surface_raw(
            (0x1234_i32 * 256) as i16,
            (-0x2345_i32 * 256) as i16,
            i32::MIN,
            (16_i32 * 256) as i16,
            (-16_i32 * 256) as i16,
        );
        assert_eq!(
            surface_y_at_cell(0x1234, -0x2345, i32::MIN, 16.0, -16.0),
            f32::from(expected) / 256.0
        );
    }

    #[test]
    fn dry_shoreline_corners_remain_on_the_sea_plane() {
        // Original PE 445920 at the hut crater's X/Z and tick 342 returns
        // -847 for these dry neighbours, not their terrain heights.
        let sea = -847.0 / 256.0;
        for terrain_raw in [-847, -832, -800, -256, 0, 1024] {
            assert_eq!(
                surface_y_at_cell(144, 128, 342, sea, terrain_raw as f32 / 256.0),
                sea,
            );
        }
        // The same PE call at the wet crater centre still displaces the sea.
        assert_eq!(
            surface_y_at_cell(144, 128, 342, sea, -896.0 / 256.0),
            -855.0 / 256.0,
        );
    }

    /// Frame class must match the popcount structure of the code:
    /// 1 corner → 0, adjacent pair → 1, diagonal pair → 2, 3 corners → 3,
    /// full → 4 (verified EXE data; this guards against typos).
    #[test]
    fn frame_offsets_match_shape_classes() {
        for (code, entry) in SHORE_TABLE.iter().enumerate().skip(1) {
            let bits = (code as u32).count_ones();
            let expected = match code {
                5 | 10 => 2, // diagonal pairs
                _ => match bits {
                    1 => 0,
                    2 => 1,
                    3 => 3,
                    4 => 4,
                    _ => unreachable!(),
                },
            };
            assert_eq!(entry.frame, expected, "code {code} frame class mismatch");
        }
    }

    /// Every drawn entry's rotation must be a permutation of slots 0-3
    /// (code 0 is never drawn; its raw table value is all-zero).
    #[test]
    fn rotations_are_permutations() {
        for (code, e) in SHORE_TABLE.iter().enumerate().skip(1) {
            let mut seen = [false; 4];
            for &s in &e.slot {
                assert!(s < 4, "code {code} slot out of range");
                seen[s as usize] = true;
            }
            assert!(seen.iter().all(|&b| b), "code {code} not a permutation");
        }
    }

    /// Submerged corners must land on texture slots that are water in the
    /// canonical sprites. Shape sprites (verified from the decoded PNGs):
    /// shape 0 water at slot 3 only; shape 1 water at slots 2,3; shape 2
    /// water at slots 1,3; shape 3 dry at slot 0 only; shape 4 all water.
    #[test]
    fn submerged_corners_map_to_wet_slots() {
        let wet_slots: [&[u8]; 5] = [&[3], &[2, 3], &[1, 3], &[1, 2, 3], &[0, 1, 2, 3]];
        for (code, e) in SHORE_TABLE.iter().enumerate().skip(1) {
            for corner in 0..4 {
                let submerged = code & (1 << corner) != 0;
                let slot = e.slot[corner];
                let wet = wet_slots[e.frame as usize].contains(&slot);
                assert_eq!(
                    submerged, wet,
                    "code {code} corner {corner} → slot {slot} (frame {}): wet/dry mismatch",
                    e.frame
                );
            }
        }
    }
}
