//! Section 10 terrain heightmap decoder.
//!
//! 20-byte header (5 × i32) followed by a 256×256×3 X-major grid.
//! Each cell: height (u8), attribute (u8), terrain_type (u8).
//!
//! Present in 38 game world levels.
//!
//! This Rust decoder is canonical; the earlier Python prototype is retired.

use v2k_core::{Result, V2kError};

use crate::fixed_math::retail_sine_q15;

/// Grid dimension (256×256).
pub const GRID_SIZE: usize = 256;

/// World-units-per-height-byte, expressed in port units (1 cell = 1.0).
///
/// The engine samples terrain as `(signed i8)height * 32` world units
/// (`FUN_00445860`), and 1 grid cell spans 256 world units. Adopting the
/// port convention of 1.0 unit per cell means dividing engine world-Y by
/// 256, so `32 / 256 = 0.125`. The old value (0.1, applied to an *unsigned*
/// byte) was invented and did not match the engine's coordinate space.
pub const HEIGHT_SCALE: f32 = 0.125;

/// Header size in bytes.
const HEADER_SIZE: usize = 20;

/// Bytes per terrain cell.
const BYTES_PER_CELL: usize = 3;

/// Expected total section data size.
pub const EXPECTED_SIZE: usize = HEADER_SIZE + GRID_SIZE * GRID_SIZE * BYTES_PER_CELL; // 196,628

/// A single terrain cell.
#[derive(Debug, Clone, Copy)]
pub struct TerrainCell {
    /// Height/elevation value (0-255).
    pub height: u8,
    /// Attribute byte.
    pub attribute: u8,
    /// Terrain type/texture ID.
    pub terrain_type: u8,
}

/// A decoded terrain heightmap grid.
#[derive(Debug, Clone)]
pub struct TerrainGrid {
    /// 5 header i32 values.
    pub header: [i32; 5],
    /// Flat array of 256×256 cells in the executable's X-major order:
    /// `index = x * 256 + z`.
    pub cells: Vec<TerrainCell>,
}

impl TerrainGrid {
    /// Engine-world Y where the underwater terrain-darkening ramp begins.
    /// Stored as the low signed word of Section 10 header dword 4.
    pub fn darkness_start_world_y(&self) -> i16 {
        self.header[4] as i16
    }

    /// Engine-world depth over which underwater darkness grows from 0 to 8.
    /// Stored as the high signed word of Section 10 header dword 4.
    pub fn darkness_range(&self) -> i16 {
        (self.header[4] >> 16) as i16
    }

    /// Access a cell by world-grid `(x, z)`, matching every original lookup
    /// (`FUN_00445860`: `x * 0x100 + z`). Returns None if out of bounds.
    pub fn cell(&self, x: usize, z: usize) -> Option<&TerrainCell> {
        if x < GRID_SIZE && z < GRID_SIZE {
            Some(&self.cells[x * GRID_SIZE + z])
        } else {
            None
        }
    }

    /// World-Y of the sea plane for this level, in port units.
    ///
    /// Section 10 header word 0 stores the sea level as `worldY * 256`
    /// (24.8 fixed point); the engine reads it everywhere as `>> 8`. In port
    /// units (1 cell = 1.0 = 256 world units) that is `header[0] / 65536`.
    /// The `FORMAT_DOCUMENTATION.md` name "world_y_offset" is a misnomer —
    /// this field is the per-level sea level.
    pub fn sea_level_world_y(&self) -> f32 {
        self.header[0] as f32 / 65536.0
    }

    /// Sea height in the signed engine-world word consumed by physics and
    /// `FUN_0041F470` (`Section10.header[0] >> 8`).
    pub fn sea_level_raw(&self) -> i16 {
        (self.header[0] >> 8) as i16
    }

    /// Whether this level renders a water plane.
    ///
    /// The engine gates the water pass on `(sea_level >> 8) > -0x1000`
    /// (`game_logic.c:23748`); "dry" levels park the plane at −6144, below
    /// the −4096 terrain floor, to disable it. `header[0] >> 8` is the sea
    /// level in world units, so the gate is `sea_world_units > -4096`.
    pub fn water_enabled(&self) -> bool {
        (self.header[0] >> 8) > -4096
    }

    /// Sample terrain height with bilinear interpolation.
    ///
    /// `x` and `z` are in grid coordinates; they wrap toroidally (the engine
    /// samples with `& 0xff` cell masking, so the grid is seamless — cell 255
    /// interpolates into cell 0). Returns height in port units
    /// (`signed i8 height * HEIGHT_SCALE`), matching `FUN_00445860`.
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        let x = x.rem_euclid(GRID_SIZE as f32);
        let z = z.rem_euclid(GRID_SIZE as f32);

        let x0 = (x as usize) % GRID_SIZE;
        let z0 = (z as usize) % GRID_SIZE;
        let x1 = (x0 + 1) % GRID_SIZE;
        let z1 = (z0 + 1) % GRID_SIZE;

        let fx = x - x.floor();
        let fz = z - z.floor();

        // Height byte is a signed i8 in the engine (relief −4096..+4064).
        let h00 = self.cell(x0, z0).unwrap().height as i8 as f32;
        let h10 = self.cell(x1, z0).unwrap().height as i8 as f32;
        let h01 = self.cell(x0, z1).unwrap().height as i8 as f32;
        let h11 = self.cell(x1, z1).unwrap().height as i8 as f32;

        let h = h00 * (1.0 - fx) * (1.0 - fz)
            + h10 * fx * (1.0 - fz)
            + h01 * (1.0 - fx) * fz
            + h11 * fx * fz;

        h * HEIGHT_SCALE
    }

    /// Exact signed-8.8 bilinear height lookup used by retail
    /// `FUN_0041DE60`, `FUN_00445860`, and their inlined copies.
    ///
    /// Both axes wrap on the 256-cell terrain torus. Height bytes are signed
    /// and expand by 32 before the two successive 8-bit interpolations.
    pub fn bilinear_height_raw(&self, x_raw: i16, z_raw: i16) -> i16 {
        let x_word = x_raw as u16;
        let z_word = z_raw as u16;
        let x0 = usize::from(x_word >> 8);
        let z0 = usize::from(z_word >> 8);
        let x1 = (x0 + 1) & 0xff;
        let z1 = (z0 + 1) & 0xff;
        let x_fraction = i32::from(x_word & 0xff);
        let z_fraction = i32::from(z_word & 0xff);
        let height = |x: usize, z: usize| {
            i32::from(
                self.cell(x, z)
                    .expect("retail terrain is a complete 256x256 torus")
                    .height as i8,
            ) << 5
        };

        let h00 = height(x0, z0);
        let h10 = height(x1, z0);
        let h01 = height(x0, z1);
        let h11 = height(x1, z1);
        let along_x0 = (((h10 - h00) * x_fraction) >> 8) + h00;
        let along_x1 = (((h11 - h01) * x_fraction) >> 8) + h01;
        ((((along_x1 - along_x0) * z_fraction) >> 8) + along_x0) as i16
    }

    /// Get the height range (min, max) across the entire grid.
    pub fn height_range(&self) -> (u8, u8) {
        let mut min = 255u8;
        let mut max = 0u8;
        for c in &self.cells {
            if c.height < min {
                min = c.height;
            }
            if c.height > max {
                max = c.height;
            }
        }
        (min, max)
    }
}

/// Wave-displaced water surface in signed engine-world units.
///
/// This is the integer wave half of `FUN_0041F470`; the caller supplies the
/// terrain floor selected by its own retail path. In particular,
/// `FUN_0043E8A0` and `FUN_0043E4F0` pass the coarse current-cell floor here,
/// then perform a separate bilinear eligibility test. All position and time
/// arithmetic wraps like the retail 32-bit code; every sine comes from the
/// exact `0x004D14D0` table; and both shifted sine terms narrow to signed words
/// before they are added. The returned surface is clamped to the supplied
/// floor.
pub fn wave_surface_raw(
    x_raw: i16,
    z_raw: i16,
    tick: i32,
    sea_y_raw: i16,
    terrain_y_raw: i16,
) -> i16 {
    if sea_y_raw <= terrain_y_raw {
        return terrain_y_raw;
    }

    let x = u32::from(x_raw as u16);
    let z = u32::from(z_raw as u16);
    let tick = tick as u32;

    let phase_1 = tick
        .wrapping_mul(0x40)
        .wrapping_add(x.wrapping_mul(0x0c))
        .wrapping_mul(2);
    let phase_2 = x
        .wrapping_add(z)
        .wrapping_mul(0x20)
        .wrapping_add(tick.wrapping_mul(0x140));
    let phase_3 = tick
        .wrapping_mul(0x40)
        .wrapping_add(z.wrapping_sub(x).wrapping_mul(6))
        .wrapping_mul(8);

    let wave_1 = retail_sine_q15(phase_1);
    let wave_2 = retail_sine_q15(phase_2);
    let wave_3 = retail_sine_q15(phase_3);
    let coarse = (wave_3 >> 6) as i16;
    let combined = (wave_2.wrapping_add(wave_1) >> 5) as i16;
    let wave_sum = coarse.wrapping_add(combined);
    let depth = i32::from(sea_y_raw) - i32::from(terrain_y_raw) + 0x200;
    let displacement = ((i32::from(wave_sum) * depth) >> 15) as i16;
    terrain_y_raw.max(sea_y_raw.wrapping_add(displacement))
}

/// Wave policy consumed by the water projector's `FUN_00445920` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaterAnimation {
    Static,
    Animated { tick: i32 },
}

/// Submitted water height from `FUN_00445920`, in signed engine-world units.
///
/// Dry shoreline vertices stay at the static sea height, even when that is
/// below the supplied terrain. The separate submerged bit and shoreline
/// sprite own coverage. This differs from [`wave_surface_raw`], whose callers
/// need a ride/contact surface that returns the terrain on dry ground.
/// Disabling waves also returns the static sea directly; only animated wet
/// vertices evaluate the shared sine expression and clamp troughs to terrain.
pub fn water_surface_raw(
    position_raw: [i16; 2],
    sea_y_raw: i16,
    terrain_y_raw: i16,
    animation: WaterAnimation,
) -> i16 {
    match animation {
        WaterAnimation::Static => sea_y_raw,
        WaterAnimation::Animated { .. } if sea_y_raw <= terrain_y_raw => sea_y_raw,
        WaterAnimation::Animated { tick } => wave_surface_raw(
            position_raw[0],
            position_raw[1],
            tick,
            sea_y_raw,
            terrain_y_raw,
        ),
    }
}

/// Wave-displaced water surface height at a point, in port units.
///
/// Float coordinates are only an adapter for render/effect callers. They are
/// first quantized to the signed position words and integer 50 Hz tick used by
/// the executable, then evaluated by [`wave_surface_raw`].
///
/// - `x_cell`, `z_cell`: horizontal position in grid cells.
/// - `tick`: the 50 Hz game tick (`elapsed_seconds * 50`).
/// - `sea_y`, `terrain_y`: sea level and ground height at this point, port units.
pub fn wave_surface_y(x_cell: f32, z_cell: f32, tick: f32, sea_y: f32, terrain_y: f32) -> f32 {
    if !x_cell.is_finite()
        || !z_cell.is_finite()
        || !tick.is_finite()
        || !sea_y.is_finite()
        || !terrain_y.is_finite()
    {
        return terrain_y;
    }
    let raw = |value: f32| ((value * 256.0).round() as i32) as i16;
    f32::from(wave_surface_raw(
        raw(x_cell),
        raw(z_cell),
        tick as i32,
        raw(sea_y),
        raw(terrain_y),
    )) / 256.0
}

/// Parse Section 10 terrain heightmap data.
pub fn parse_terrain(data: &[u8]) -> Result<TerrainGrid> {
    if data.len() != EXPECTED_SIZE {
        return Err(V2kError::section(
            10,
            format!(
                "section 10 size mismatch: {} bytes, expected {}",
                data.len(),
                EXPECTED_SIZE
            ),
        ));
    }

    // Parse header (5 × i32)
    let mut header = [0i32; 5];
    for (i, word) in header.iter_mut().enumerate() {
        let off = i * 4;
        *word = i32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]);
    }

    // Parse grid
    let mut cells = Vec::with_capacity(GRID_SIZE * GRID_SIZE);
    let mut offset = HEADER_SIZE;
    for _ in 0..GRID_SIZE * GRID_SIZE {
        cells.push(TerrainCell {
            height: data[offset],
            attribute: data[offset + 1],
            terrain_type: data[offset + 2],
        });
        offset += BYTES_PER_CELL;
    }

    Ok(TerrainGrid { header, cells })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_synthetic() {
        let mut data = vec![0u8; EXPECTED_SIZE];
        // Set header values
        data[0..4].copy_from_slice(&42i32.to_le_bytes());
        // Set cell (0,0) height to 100
        data[HEADER_SIZE] = 100;
        // Set cell (0,0) terrain_type to 5
        data[HEADER_SIZE + 2] = 5;

        let grid = parse_terrain(&data).unwrap();
        assert_eq!(grid.header[0], 42);
        let c = grid.cell(0, 0).unwrap();
        assert_eq!(c.height, 100);
        assert_eq!(c.terrain_type, 5);
    }

    #[test]
    fn height_at_exact_cell() {
        let mut data = vec![0u8; EXPECTED_SIZE];
        // Set world cell (x=5, z=3) to height 100 (X-major storage).
        let idx = HEADER_SIZE + (5 * GRID_SIZE + 3) * BYTES_PER_CELL;
        data[idx] = 100;
        let grid = parse_terrain(&data).unwrap();
        let h = grid.height_at(5.0, 3.0);
        assert!((h - 12.5).abs() < 0.01); // (i8)100 * 0.125
    }

    #[test]
    fn height_at_signed() {
        let mut data = vec![0u8; EXPECTED_SIZE];
        // Height byte 0x80 = -128 as i8 → deepest terrain (-4096 world = -16.0 port).
        let idx = HEADER_SIZE + (5 * GRID_SIZE + 3) * BYTES_PER_CELL;
        data[idx] = 0x80;
        let grid = parse_terrain(&data).unwrap();
        let h = grid.height_at(5.0, 3.0);
        assert!((h - (-16.0)).abs() < 0.01); // -128 * 0.125
    }

    #[test]
    fn wave_surface_behaviour() {
        // No water (sea at/below terrain) → returns terrain, never displaced.
        assert_eq!(wave_surface_y(10.0, 10.0, 5.0, -5.0, -5.0), -5.0);
        assert_eq!(wave_surface_y(10.0, 10.0, 5.0, -5.0, 2.0), 2.0);

        // Deep water: the surface oscillates around the sea level over time and
        // stays at or above the terrain floor.
        let sea = 16.0;
        let floor = -16.0;
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for i in 0..200 {
            let tick = i as f32;
            let s = wave_surface_y(40.0, 72.0, tick, sea, floor);
            assert!(s >= floor - 1e-3, "surface {s} dipped below floor");
            min = min.min(s);
            max = max.max(s);
        }
        // Deep water must actually move (waves), bracketing the sea level.
        assert!(
            max - min > 1.0,
            "deep water barely waved: range {}",
            max - min
        );
        assert!(
            min < sea && max > sea,
            "waves should straddle the sea level"
        );

        // Shallower water waves with smaller amplitude than deep water.
        let deep_range = {
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            for i in 0..200 {
                let s = wave_surface_y(40.0, 72.0, i as f32, sea, -16.0);
                lo = lo.min(s);
                hi = hi.max(s);
            }
            hi - lo
        };
        let shallow_range = {
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            for i in 0..200 {
                let s = wave_surface_y(40.0, 72.0, i as f32, sea, 14.0);
                lo = lo.min(s);
                hi = hi.max(s);
            }
            hi - lo
        };
        assert!(
            shallow_range < deep_range,
            "shallow range {shallow_range} should be < deep range {deep_range}"
        );
    }

    #[test]
    fn raw_wave_matches_retail_integer_narrowing() {
        assert_eq!(wave_surface_raw(0, 0, 0, 4096, -4096), 4096);
        assert_eq!(wave_surface_raw(0, 0, 1, 4096, -4096), 4114);
        assert_eq!(
            wave_surface_raw(0x1234, 0xabcdu16 as i16, 4793, 4096, -1024),
            4401
        );
        assert_eq!(wave_surface_raw(-14_189, 19_589, 4793, 4096, -1024), 4371);
        assert_eq!(wave_surface_raw(0, 0, 1, 4096, 5000), 5000);
    }

    #[test]
    fn rendered_water_matches_original_445920_shoreline_heights() {
        // Executed directly from the original PE at 445920. These include
        // the local hut crater and its dry neighbours across the i16 Z seam.
        let position = [-28672, -32768];
        for (terrain, expected) in [
            (-896, -855),
            (-864, -855),
            (-847, -847),
            (-832, -847),
            (-800, -847),
            (-256, -847),
            (0, -847),
            (1024, -847),
        ] {
            assert_eq!(
                water_surface_raw(
                    position,
                    -847,
                    terrain,
                    WaterAnimation::Animated { tick: 342 },
                ),
                expected,
            );
            assert_eq!(
                water_surface_raw(position, -847, terrain, WaterAnimation::Static),
                -847,
            );
        }
        // The physics/contact policy must retain dry terrain as its floor.
        assert_eq!(
            wave_surface_raw(position[0], position[1], 342, -847, -256),
            -256
        );
    }

    #[test]
    fn rendered_water_preserves_wet_wave_displacement_and_static_mode() {
        // Independent 445920 PE outputs, including signed coordinate wraps.
        for (position, tick, terrain, expected) in [
            ([0, 0], 0, -4096, 4096),
            ([0, 0], 1, -4096, 4114),
            ([0x1234, 0xabcdu16 as i16], 4793, -1024, 4401),
            ([-14189, 19589], 4793, -1024, 4371),
        ] {
            assert_eq!(
                water_surface_raw(position, 4096, terrain, WaterAnimation::Animated { tick }),
                expected,
            );
            assert_eq!(
                water_surface_raw(position, 4096, terrain, WaterAnimation::Static),
                4096,
            );
        }
    }

    #[test]
    fn height_at_wraps_at_seam() {
        let mut data = vec![0u8; EXPECTED_SIZE];
        // Z=0: X=255 has height 80 and X=0 has height 0.
        let cell = |x: usize, z: usize| HEADER_SIZE + (x * GRID_SIZE + z) * BYTES_PER_CELL;
        data[cell(255, 0)] = 80;
        data[cell(0, 0)] = 0;
        let grid = parse_terrain(&data).unwrap();
        // Halfway across the 255→0 seam should be the midpoint of the two,
        // not a clamp to cell 255. (i8)80 * 0.125 = 10.0, midpoint = 5.0.
        let h = grid.height_at(255.5, 0.0);
        assert!((h - 5.0).abs() < 0.01, "seam sample = {h}, expected 5.0");
        // Sampling past the edge wraps back into the grid.
        assert!((grid.height_at(256.0, 0.0) - grid.height_at(0.0, 0.0)).abs() < 0.01);
    }

    #[test]
    fn water_accessors() {
        let mut data = vec![0u8; EXPECTED_SIZE];
        // Water(22): header[0] = 1048576 → sea plane 16.0, enabled.
        data[0..4].copy_from_slice(&1_048_576i32.to_le_bytes());
        let grid = parse_terrain(&data).unwrap();
        assert!((grid.sea_level_world_y() - 16.0).abs() < 0.001);
        assert_eq!(grid.sea_level_raw(), 4096);
        assert!(grid.water_enabled());

        // Dry (VSpread 26): header[0] = -1572864 → plane -24.0, disabled.
        data[0..4].copy_from_slice(&(-1_572_864i32).to_le_bytes());
        let grid = parse_terrain(&data).unwrap();
        assert!(!grid.water_enabled());

        // Alien2(46): header[0] = -1048576 → sea = -4096 world, strict '>' → disabled.
        data[0..4].copy_from_slice(&(-1_048_576i32).to_le_bytes());
        let grid = parse_terrain(&data).unwrap();
        assert!(!grid.water_enabled());
    }

    #[test]
    fn height_at_interpolated() {
        let mut data = vec![0u8; EXPECTED_SIZE];
        // Set a 2x2 patch: (0,0)=0, (1,0)=100, (0,1)=0, (1,1)=100
        let cell = |x: usize, z: usize| HEADER_SIZE + (x * GRID_SIZE + z) * BYTES_PER_CELL;
        data[cell(0, 0)] = 0;
        data[cell(1, 0)] = 100;
        data[cell(0, 1)] = 0;
        data[cell(1, 1)] = 100;
        let grid = parse_terrain(&data).unwrap();
        // Midpoint along x at z=0 should be (i8)50 * 0.125 = 6.25
        let h = grid.height_at(0.5, 0.0);
        assert!((h - 6.25).abs() < 0.01);
    }

    #[test]
    fn bilinear_height_raw_preserves_signed_bytes_and_toroidal_seams() {
        let mut data = vec![0u8; EXPECTED_SIZE];
        let set_height = |data: &mut [u8], x: usize, z: usize, height: i8| {
            data[HEADER_SIZE + (x * GRID_SIZE + z) * BYTES_PER_CELL] = height as u8;
        };
        set_height(&mut data, 0, 0, -8);
        set_height(&mut data, 1, 0, 8);
        set_height(&mut data, 0, 1, 0);
        set_height(&mut data, 1, 1, 16);
        set_height(&mut data, 255, 0, 24);
        let grid = parse_terrain(&data).unwrap();

        assert_eq!(grid.bilinear_height_raw(0, 0), -256);
        assert_eq!(grid.bilinear_height_raw(0x0080, 0x0080), 128);
        assert_eq!(
            grid.bilinear_height_raw(-128, 0),
            256,
            "cell 255 must interpolate into cell 0 across the toroidal seam"
        );
    }

    #[test]
    fn wrong_size() {
        let data = vec![0u8; 1000];
        assert!(parse_terrain(&data).is_err());
    }

    #[test]
    fn height_range() {
        let mut data = vec![0u8; EXPECTED_SIZE];
        data[HEADER_SIZE] = 10; // first cell height
        data[HEADER_SIZE + 3] = 200; // second cell height
        let grid = parse_terrain(&data).unwrap();
        let (min, max) = grid.height_range();
        assert_eq!(min, 0);
        assert_eq!(max, 200);
    }

    #[test]
    fn darkness_words_share_header_dword_four() {
        let mut data = vec![0u8; EXPECTED_SIZE];
        data[16..20].copy_from_slice(&(i32::from(-1298i16 as u16) | (790i32 << 16)).to_le_bytes());
        let terrain = parse_terrain(&data).unwrap();
        assert_eq!(terrain.darkness_start_world_y(), -1298);
        assert_eq!(terrain.darkness_range(), 790);
    }

    #[test]
    fn cell_addressing_is_x_major_not_transposed() {
        let mut data = vec![0u8; EXPECTED_SIZE];
        data[HEADER_SIZE + (5 * GRID_SIZE + 3) * BYTES_PER_CELL] = 77;
        data[HEADER_SIZE + (3 * GRID_SIZE + 5) * BYTES_PER_CELL] = 22;
        let terrain = parse_terrain(&data).unwrap();
        assert_eq!(terrain.cell(5, 3).unwrap().height, 77);
        assert_eq!(terrain.cell(3, 5).unwrap().height, 22);
    }
}
