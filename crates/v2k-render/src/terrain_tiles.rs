//! Original terrain transition-tile canonicalization (`FUN_00433180`).
//!
//! Each terrain quad has four corner material codes in 0..=4. There are
//! 5^4 = 625 possible quads, but rotation/reflection reduces them to 120
//! canonical sprites beginning at the level descriptor's terrain sprite base.

pub const MATERIAL_COUNT: usize = 5;
pub const COMBINATION_COUNT: usize = MATERIAL_COUNT.pow(4); // 625
pub const CANONICAL_FRAME_COUNT: usize = 120;
/// `FUN_00433180` places the five infection marching-square shapes directly
/// after the 120 opaque D4-canonical terrain frames. `FUN_00430430` selects
/// these with the same `DAT_004CACC8/CC` table used by shoreline water.
pub const INFECTION_BASE_OFFSET: u32 = CANONICAL_FRAME_COUNT as u32;
pub const INFECTION_FRAME_COUNT: usize = 5;

/// Reproduce `FUN_00433130` -> `FUN_004330D0`: the level-authored depth also
/// controls the 4:5-aspect terrain scan width, with the executable's hard
/// maximum of 52 columns by 30 rows.
pub fn scan_dimensions(authored_depth: u32) -> (u32, u32) {
    let columns = authored_depth
        .saturating_mul(4)
        .checked_div(5)
        .unwrap_or(0)
        .saturating_mul(2)
        .min(52);
    (columns, authored_depth.min(30))
}

/// Canonical sprite offset plus the texture-corner permutation required to
/// orient it back onto the authored four-corner combination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainTile {
    pub frame: u8,
    pub slots: [u8; 4],
}

use crate::renderer::TextureId;
use v2k_formats::fixed_math::retail_sine_q15;

fn scaled_retail_sine(angle: u32, amplitude: i32) -> i8 {
    let q31 = retail_sine_q15(angle).wrapping_mul(0x1_0001);
    (((i64::from(q31) * i64::from(amplitude)) >> 31) as i32) as i8
}

fn infection_motion_offsets(phase_a: u32, phase_b: u32, amplitude: i32) -> [i8; 16] {
    let pair = |phase| {
        let sine = scaled_retail_sine(phase, amplitude);
        let cosine = scaled_retail_sine(phase.wrapping_add(0x4000), amplitude);
        [
            sine,
            -sine,
            sine / 2,
            -(sine / 2),
            cosine,
            -cosine,
            cosine / 2,
            -(cosine / 2),
        ]
    };
    let mut offsets = [0; 16];
    offsets[..8].copy_from_slice(&pair(phase_a));
    offsets[8..].copy_from_slice(&pair(phase_b));
    offsets
}

/// One immutable terrain-build snapshot of `FUN_00433530`'s motion globals.
#[derive(Debug, Clone)]
pub(crate) struct InfectionMotionFrame {
    offsets: [i8; 16],
    selectors: [u8; 256],
}

impl InfectionMotionFrame {
    /// Context `+0x44`: the sixteen signed motion offsets.
    pub(crate) fn offsets(&self) -> [i8; 16] {
        self.offsets
    }

    /// `0x004CAB88`: the selector bytes.
    pub(crate) fn selectors(&self) -> &[u8; 256] {
        &self.selectors
    }

    /// Apply `FUN_00430140`'s obfuscated 16x16 selector lookup. Its expression
    /// reduces to low-nibble X in bits 0..3 and low-nibble Z in bits 4..7.
    pub(crate) fn vertex_offset_raw(&self, world_x_cell: i32, world_z_cell: i32) -> [i8; 2] {
        let selector =
            (world_x_cell.rem_euclid(16) as u8) | ((world_z_cell.rem_euclid(16) as u8) << 4);
        let choices = self.selectors[usize::from(selector)];
        [
            self.offsets[usize::from(choices & 0x0f)],
            self.offsets[usize::from(choices >> 4)],
        ]
    }
}

/// Process-lifetime infection-motion globals owned by `FUN_00433530`.
///
/// Retail advances these only when its terrain draw callback executes, using
/// that callback's elapsed-microsecond word. Keeping the state in the renderer
/// therefore preserves it across level loads while naturally freezing it in
/// menus, pauses, and other frames that do not draw terrain.
#[derive(Debug, Clone)]
pub(crate) struct InfectionTerrainAnimation {
    phase_a: u32,
    phase_b: u32,
    amplitude: i32,
    direction: i8,
    offsets: [i8; 16],
    selectors: [u8; 256],
    selector_rng_state: u32,
    #[cfg(test)]
    selector_refreshes: u32,
}

impl Default for InfectionTerrainAnimation {
    fn default() -> Self {
        Self {
            phase_a: 0,
            phase_b: 0,
            amplitude: 0,
            // Retail's initialized .data value is -1, so the first positive
            // elapsed draw clamps at zero and fills the selector table.
            direction: -1,
            offsets: [0; 16],
            selectors: [0; 256],
            selector_rng_state: 0,
            #[cfg(test)]
            selector_refreshes: 0,
        }
    }
}

impl InfectionTerrainAnimation {
    pub(crate) fn advance(&mut self, elapsed_micros: u32) {
        let amplitude_step = (elapsed_micros >> 6).min(i32::MAX as u32) as i32;
        self.phase_a = self.phase_a.wrapping_add(elapsed_micros >> 6);
        self.phase_b = self.phase_b.wrapping_add(elapsed_micros / 0x12);

        if self.direction < 1 {
            self.amplitude = self.amplitude.wrapping_sub(amplitude_step);
            if self.amplitude < 0 {
                self.amplitude = 0;
                self.direction = 1;
                self.refresh_selectors();
            }
        } else {
            self.amplitude = self.amplitude.wrapping_add(amplitude_step);
            if self.amplitude > 0xffff {
                self.amplitude = 0xffff;
                self.direction = -1;
            }
        }

        let amplitude = (self.amplitude >> 10).min(0x20);
        self.offsets = infection_motion_offsets(self.phase_a, self.phase_b, amplitude);
    }

    pub(crate) fn frame(&self) -> InfectionMotionFrame {
        InfectionMotionFrame {
            offsets: self.offsets,
            selectors: self.selectors,
        }
    }

    fn refresh_selectors(&mut self) {
        // Retail consumes 0x80 consecutive 16-bit Random_Next samples into
        // this little-endian byte table. This subsystem-local stream preserves
        // the exact recurrence and sequence between refreshes; its initial
        // state remains a deterministic stand-in until all port systems share
        // the recovered process-global call order.
        for pair in self.selectors.chunks_exact_mut(2) {
            self.selector_rng_state = self
                .selector_rng_state
                .wrapping_mul(214_013)
                .wrapping_add(2_531_011);
            pair.copy_from_slice(&((self.selector_rng_state >> 16) as u16).to_le_bytes());
        }
        #[cfg(test)]
        {
            self.selector_refreshes = self.selector_refreshes.wrapping_add(1);
        }
    }
}

/// GPU-resident canonical terrain tile set for one level. The 120 sprites are
/// packed into one texture; the 625-entry lookup supplies frame and UV
/// orientation for each four-corner material combination.
#[derive(Debug)]
pub struct TerrainFrames {
    /// Brightest-shade RGBA atlas used by the fixed-function fallback.
    pub texture: TextureId,
    /// Raw palette-index atlas (index stored in the red channel).
    pub index_texture: TextureId,
    /// 16 × (120*32) palette lookup texture: frame-major, then shade row.
    pub palette_texture: TextureId,
    /// Fixed-shade-row-28 RGBA infection shapes at terrain base +120..124.
    /// Frames 0..3 are zero-keyed; frame 4 is the authored opaque full tile.
    pub infection_texture: TextureId,
    pub infection_frame_uvs: [[f32; 4]; INFECTION_FRAME_COUNT],
    pub frame_uvs: Vec<[f32; 4]>,
    pub lookup: Vec<TerrainTile>,
    /// Per-frame scalar approximation retained for the fixed-function fallback.
    pub shade_scales: Vec<[f32; 8]>,
    /// Section-6 primary table: terrain/water shade input 0..7 maps to these
    /// actual 0..31 rows in each Section-3 shade palette.
    pub shade_rows: [u8; 8],
    /// Per-level opaque terrain footprint recovered from Section 13 +0x88.
    pub scan_columns: u32,
    pub scan_rows: u32,
    /// Native materials for backends that run the retail ground producer.
    pub native: Option<NativeTerrainMaterials>,
}

/// One registered Section-3 record: its material id and `+0x10/+0x12` size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeSpriteRef {
    pub id: u32,
    pub width: u16,
    pub height: u16,
}

/// The terrain level's retail ground inputs that are resources, not state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeTerrainMaterials {
    /// Global sprite index of canonical frame 0 (Section 13 `+0x4C`).
    pub tile_base: u32,
    /// The 120 canonical frames, the five infection shapes and the five
    /// shoreline shapes, in global sprite order from `tile_base`.
    pub sprites: Vec<NativeSpriteRef>,
    /// The first eight Section-6 dwords, as `FUN_00431890` copies them.
    pub shade_words: [u32; 8],
}

#[cfg(test)]
mod scan_tests {
    use super::scan_dimensions;

    #[test]
    fn original_scan_dimension_formula_and_caps() {
        assert_eq!(scan_dimensions(18), (28, 18));
        assert_eq!(scan_dimensions(22), (34, 22));
        assert_eq!(scan_dimensions(30), (48, 30));
        assert_eq!(scan_dimensions(40), (52, 30));
    }
}

impl TerrainFrames {
    pub fn tile(&self, corners: [u8; 4]) -> Option<(TerrainTile, [f32; 4])> {
        let tile = *self.lookup.get(corner_index(corners)?)?;
        let uv = *self.frame_uvs.get(tile.frame as usize)?;
        Some((tile, uv))
    }

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

    pub fn textures(&self) -> [TextureId; 4] {
        [
            self.texture,
            self.index_texture,
            self.palette_texture,
            self.infection_texture,
        ]
    }
}

/// Exact 0..=8 depth-darkness step from `FUN_004336E0`. The input and Section
/// 10 fields are all in the engine's 8.8 world-Y domain.
pub fn underwater_darkness(height_world: i16, darkness_start: i16, darkness_range: i16) -> u8 {
    let height = height_world as i32;
    let start = darkness_start as i32;
    let range = darkness_range as i32;
    if height >= start {
        0
    } else if range <= 0 || height < start - range {
        8
    } else {
        (((start - height) * 8) / range).clamp(0, 8) as u8
    }
}

/// Exact shade input established by `FUN_00430140` before the software
/// textured filler interpolates it. `height` is the authored signed terrain
/// height byte and `light_delta` is the signed 32×32 dynamic-light entry.
pub fn vertex_shade(
    terrain_type: u8,
    height: u8,
    light_delta: i8,
    darkness_start: i16,
    darkness_range: i16,
) -> u8 {
    let mut shade = (terrain_type >> 5) as i32 + light_delta as i32;
    let height_world = height as i8 as i16 * 32;
    shade -= underwater_darkness(height_world, darkness_start, darkness_range) as i32;
    shade.clamp(0, 7) as u8
}

// The eight square symmetries in the exact order evaluated by FUN_00433180.
// Input/output corner words are little-to-big base-5 digits [a,b,c,d].
const TRANSFORMS: [[usize; 4]; 8] = [
    [0, 1, 2, 3], // [a,b,c,d]
    [2, 1, 0, 3], // [c,b,a,d]
    [1, 2, 3, 0], // [b,c,d,a]
    [3, 2, 1, 0], // [d,c,b,a]
    [2, 3, 0, 1], // [c,d,a,b]
    [0, 3, 2, 1], // [a,d,c,b]
    [3, 0, 1, 2], // [d,a,b,c]
    [1, 0, 3, 2], // [b,a,d,c]
];

// DAT_004CAD48: transform index -> entry in the UV rotation table at
// 0x4CACA8. The decoded low nibbles are the destination texture slots.
const TRANSFORM_ROTATION: [usize; 8] = [7, 6, 1, 0, 2, 3, 4, 5];
const ROTATION_SLOTS: [[u8; 4]; 8] = [
    [0, 1, 2, 3],
    [0, 3, 2, 1],
    [1, 0, 3, 2],
    [3, 0, 1, 2],
    [2, 1, 0, 3],
    [2, 3, 0, 1],
    [1, 2, 3, 0],
    [3, 2, 1, 0],
];

fn decode(mut index: usize) -> [usize; 4] {
    let mut corners = [0; 4];
    for corner in &mut corners {
        *corner = index % MATERIAL_COUNT;
        index /= MATERIAL_COUNT;
    }
    corners
}

fn encode(corners: [usize; 4]) -> usize {
    corners[0]
        + MATERIAL_COUNT
            * (corners[1] + MATERIAL_COUNT * (corners[2] + MATERIAL_COUNT * corners[3]))
}

fn transformed_index(corners: [usize; 4], transform: [usize; 4]) -> usize {
    encode([
        corners[transform[0]],
        corners[transform[1]],
        corners[transform[2]],
        corners[transform[3]],
    ])
}

/// Build the exact runtime 625-entry lookup generated by FUN_00433180.
pub fn build_lookup() -> Vec<TerrainTile> {
    let mut canonical_frame = [u8::MAX; COMBINATION_COUNT];
    let mut next_frame = 0u8;

    // The engine walks combinations in numeric base-5 order and assigns a
    // sprite offset whenever the identity form is the smallest symmetry.
    for (index, frame) in canonical_frame.iter_mut().enumerate() {
        let corners = decode(index);
        let minimum = TRANSFORMS
            .iter()
            .map(|&transform| transformed_index(corners, transform))
            .min()
            .expect("eight terrain transforms");
        if minimum == index {
            *frame = next_frame;
            next_frame += 1;
        }
    }
    debug_assert_eq!(next_frame as usize, CANONICAL_FRAME_COUNT);

    (0..COMBINATION_COUNT)
        .map(|index| {
            let corners = decode(index);
            let mut minimum = usize::MAX;
            let mut selected_transform = 0;
            // Strict '<' preserves the first transform for symmetric cases,
            // matching the executable's tie behavior.
            for (transform_index, &transform) in TRANSFORMS.iter().enumerate() {
                let candidate = transformed_index(corners, transform);
                if candidate < minimum {
                    minimum = candidate;
                    selected_transform = transform_index;
                }
            }
            TerrainTile {
                frame: canonical_frame[minimum],
                slots: ROTATION_SLOTS[TRANSFORM_ROTATION[selected_transform]],
            }
        })
        .collect()
}

/// Encode four engine material codes into the lookup's base-5 index.
pub fn corner_index(corners: [u8; 4]) -> Option<usize> {
    corners
        .iter()
        .all(|&corner| corner < MATERIAL_COUNT as u8)
        .then(|| {
            encode([
                corners[0] as usize,
                corners[1] as usize,
                corners[2] as usize,
                corners[3] as usize,
            ])
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_material_quads_reduce_to_120_canonical_sprites() {
        let lookup = build_lookup();
        assert_eq!(lookup.len(), COMBINATION_COUNT);
        let mut seen = [false; CANONICAL_FRAME_COUNT];
        for tile in lookup {
            seen[tile.frame as usize] = true;
            let mut slots = tile.slots;
            slots.sort_unstable();
            assert_eq!(slots, [0, 1, 2, 3]);
        }
        assert!(seen.into_iter().all(|present| present));
    }

    #[test]
    fn identity_canonical_form_uses_executable_slot_order() {
        let lookup = build_lookup();
        let index = corner_index([0, 0, 0, 0]).unwrap();
        assert_eq!(lookup[index].frame, 0);
        // Symmetry ties select transform zero; DAT_004CAD48 maps it to
        // rotation entry seven.
        assert_eq!(lookup[index].slots, [3, 2, 1, 0]);
    }

    #[test]
    fn corner_codes_are_strictly_base_five() {
        assert_eq!(corner_index([0, 1, 2, 4]), Some(555));
        assert_eq!(corner_index([0, 1, 2, 5]), None);
    }

    #[test]
    fn vertex_shade_applies_light_and_underwater_ramp() {
        assert_eq!(vertex_shade(4 << 5, 0, 2, -1024, 512), 6);
        // height -128 * 32 lies below the full-dark threshold.
        assert_eq!(vertex_shade(7 << 5, 128, 0, -1024, 512), 0);
        // Halfway through the ramp subtracts four.
        assert_eq!(vertex_shade(7 << 5, 216, 0, -1024, 512), 3);
    }

    #[test]
    fn underwater_darkness_matches_original_piecewise_steps() {
        assert_eq!(underwater_darkness(-1024, -1024, 512), 0);
        assert_eq!(underwater_darkness(-1280, -1024, 512), 4);
        assert_eq!(underwater_darkness(-1536, -1024, 512), 8);
        assert_eq!(underwater_darkness(-1537, -1024, 512), 8);
    }

    #[test]
    fn infection_motion_reproduces_retail_triangle_envelope() {
        let mut animation = InfectionTerrainAnimation::default();
        animation.advance(20_000);
        assert_eq!(animation.amplitude, 0);
        assert_eq!(animation.direction, 1);
        assert_eq!(animation.selector_refreshes, 1);

        for _ in 0..210 {
            animation.advance(20_000);
        }
        assert_eq!(animation.amplitude, 65_520);
        animation.advance(20_000);
        assert_eq!(animation.amplitude, 0xffff);
        assert_eq!(animation.direction, -1);

        for _ in 0..210 {
            animation.advance(20_000);
        }
        assert_eq!(animation.amplitude, 15);
        animation.advance(20_000);
        assert_eq!(animation.amplitude, 0);
        assert_eq!(animation.direction, 1);
        assert_eq!(animation.selector_refreshes, 2);
    }

    #[test]
    fn infection_motion_is_bounded_and_repeats_its_spatial_selector() {
        let mut animation = InfectionTerrainAnimation::default();
        for _ in 0..150 {
            animation.advance(20_000);
        }
        let frame = animation.frame();
        let first = frame.vertex_offset_raw(3, 5);
        assert_eq!(first, frame.vertex_offset_raw(19, 21));
        assert_ne!(first, frame.vertex_offset_raw(4, 5));
        for x in 0..16 {
            for z in 0..16 {
                let [dx, dz] = frame.vertex_offset_raw(x, z);
                assert!(i16::from(dx).abs() <= 32);
                assert!(i16::from(dz).abs() <= 32);
            }
        }
    }

    #[test]
    fn infection_animation_freezes_without_a_terrain_draw() {
        let mut animation = InfectionTerrainAnimation::default();
        animation.advance(20_000);
        for _ in 0..40 {
            animation.advance(20_000);
        }
        let before = animation.frame();

        // A menu, pause, or synchronous load does not call `advance`, however
        // long its unrelated process clock runs.
        let after_gap = animation.frame();
        assert_eq!(before.offsets, after_gap.offsets);
        assert_eq!(before.selectors, after_gap.selectors);
        assert_eq!(animation.selector_refreshes, 1);

        animation.advance(20_000);
        assert_ne!(before.offsets, animation.frame().offsets);
    }
}
