//! Frame-transient 32×32 signed terrain-light window (`DAT_004FE820`).
//!
//! The original rebuilds this table for every presented world frame. Its
//! `FUN_004383F0` anchor is the camera's signed 8.8 X coordinate and its Z
//! coordinate plus `0x0A00`, placing the 32-cell window ten cells ahead of the
//! camera along retail's terrain scan. Samples are X-major:
//! `values[x * 32 + z]`, matching the terrain grid.

pub const LIGHT_WINDOW_SIZE: usize = 32;
const LIGHT_WINDOW_LEN: usize = LIGHT_WINDOW_SIZE * LIGHT_WINDOW_SIZE;

#[derive(Debug, Clone)]
pub struct TerrainLightWindow {
    origin_x: u8,
    origin_z: u8,
    values: [i8; LIGHT_WINDOW_LEN],
    revision: u64,
}

impl Default for TerrainLightWindow {
    fn default() -> Self {
        Self {
            origin_x: 0,
            origin_z: 0,
            values: [0; LIGHT_WINDOW_LEN],
            revision: 0,
        }
    }
}

impl TerrainLightWindow {
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn clear(&mut self) {
        if self.values.iter().any(|&value| value != 0) {
            self.values.fill(0);
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// Begin one presented retail world frame.
    ///
    /// `FUN_004530D0` first calls `FUN_004383D0` to clear all 0x400 signed
    /// light cells. `FUN_00453570` then calls
    /// `FUN_004383F0(camera_x_raw, camera_z_raw + 0x0A00)` before particles
    /// contribute their frame-local radial lights. The executable converts a
    /// signed 8.8 coordinate to a cell with truncation toward zero, not an
    /// arithmetic floor; `retail_fixed_cell` preserves that distinction at the
    /// signed world seam.
    pub fn begin_world_frame(&mut self, camera_x_raw: i32, camera_z_raw: i32) {
        self.clear();
        self.scroll_to_center(
            retail_fixed_cell(camera_x_raw),
            retail_fixed_cell(camera_z_raw.wrapping_add(0x0a00)),
        );
    }

    /// Scroll around a camera cell, preserving the overlapping world samples
    /// and zeroing newly exposed strips. A jump larger than 31 cells naturally
    /// has no overlap and clears the complete window.
    pub fn scroll_to_center(&mut self, center_x: i32, center_z: i32) {
        let new_origin_x = (center_x - 16).rem_euclid(256) as u8;
        let new_origin_z = (center_z - 16).rem_euclid(256) as u8;
        if (new_origin_x, new_origin_z) == (self.origin_x, self.origin_z) {
            return;
        }

        let mut next = [0i8; LIGHT_WINDOW_LEN];
        for local_x in 0..LIGHT_WINDOW_SIZE {
            let world_x = new_origin_x.wrapping_add(local_x as u8);
            let old_x = world_x.wrapping_sub(self.origin_x) as usize;
            if old_x >= LIGHT_WINDOW_SIZE {
                continue;
            }
            for local_z in 0..LIGHT_WINDOW_SIZE {
                let world_z = new_origin_z.wrapping_add(local_z as u8);
                let old_z = world_z.wrapping_sub(self.origin_z) as usize;
                if old_z < LIGHT_WINDOW_SIZE {
                    next[local_x * LIGHT_WINDOW_SIZE + local_z] =
                        self.values[old_x * LIGHT_WINDOW_SIZE + old_z];
                }
            }
        }
        // A changed origin changes the world mapping even if a repeating
        // numeric pattern happens to compare equal. All-zero data is the only
        // case that cannot affect a cached mesh.
        let changed =
            self.values.iter().any(|&value| value != 0) || next.iter().any(|&value| value != 0);
        self.values = next;
        self.origin_x = new_origin_x;
        self.origin_z = new_origin_z;
        if changed {
            self.revision = self.revision.wrapping_add(1);
        }
    }

    pub fn sample(&self, world_x: i32, world_z: i32) -> i8 {
        let local_x = (world_x.rem_euclid(256) as u8).wrapping_sub(self.origin_x) as usize;
        let local_z = (world_z.rem_euclid(256) as u8).wrapping_sub(self.origin_z) as usize;
        if local_x < LIGHT_WINDOW_SIZE && local_z < LIGHT_WINDOW_SIZE {
            self.values[local_x * LIGHT_WINDOW_SIZE + local_z]
        } else {
            0
        }
    }

    /// Exact upper-saturated point writer from `FUN_00441610`. Its callers use
    /// positive deltas; retaining an `i32` input also preserves the executable's
    /// signed-char wrapping behavior if negative effect writers are connected.
    pub fn add_point(&mut self, world_x: i32, world_z: i32, delta: i32) {
        let local_x = (world_x.rem_euclid(256) as u8).wrapping_sub(self.origin_x) as usize;
        let local_z = (world_z.rem_euclid(256) as u8).wrapping_sub(self.origin_z) as usize;
        if local_x >= LIGHT_WINDOW_SIZE || local_z >= LIGHT_WINDOW_SIZE {
            return;
        }
        let slot = &mut self.values[local_x * LIGHT_WINDOW_SIZE + local_z];
        let sum = *slot as i32 + delta;
        let next = if sum > i8::MAX as i32 {
            i8::MAX
        } else {
            sum as i8
        };
        if next != *slot {
            *slot = next;
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// Radial sprite/effect writer from `FUN_004385E0`. Coordinates and radius
    /// use the engine's 8.8 cell domain. The core adds +7; the outer half uses
    /// the original integer `13 - distance²*12/radius²` falloff.
    pub fn add_radial(&mut self, x_8_8: i32, z_8_8: i32, radius_8_8: i32) {
        if radius_8_8 <= 0xff {
            return;
        }
        let center_x = x_8_8 + 0x80;
        let center_z = z_8_8 + 0x80;
        let start_x = (center_x - radius_8_8) & !0xff;
        let end_x = (center_x + radius_8_8 + 0xff) & !0xff;
        let start_z = (center_z - radius_8_8) & !0xff;
        let end_z = (center_z + radius_8_8 + 0xff) & !0xff;
        let radius_sq = radius_8_8 as i64 * radius_8_8 as i64;

        let mut fixed_x = start_x;
        while fixed_x != end_x {
            let mut fixed_z = start_z;
            while fixed_z != end_z {
                let dx = fixed_x as i64 - center_x as i64;
                let dz = fixed_z as i64 - center_z as i64;
                let distance_sq = dx * dx + dz * dz;
                let delta = if distance_sq < radius_sq / 2 {
                    Some(7)
                } else if distance_sq <= radius_sq {
                    Some(13 - (distance_sq * 12 / radius_sq) as i32)
                } else {
                    None
                };
                if let Some(delta) = delta {
                    self.add_point(fixed_x >> 8, fixed_z >> 8, delta);
                }
                fixed_z += 0x100;
            }
            fixed_x += 0x100;
        }
    }

    /// Four-way symmetric explosion writer from `FUN_00441420`. This retains
    /// the original repeated writes on the center axes before delegating its
    /// softer +2 contribution to the point writer (`FUN_00441610`).
    pub fn add_explosion(&mut self, x_8_8: i32, z_8_8: i32, radius_8_8: i32) {
        let radius = (radius_8_8 + ((radius_8_8 >> 31) & 0xff)) >> 8;
        if radius <= 0 {
            return;
        }
        let mut x_offset = 0;
        let mut left_x = x_8_8;
        let mut right_x = x_8_8;
        while x_offset < radius {
            let mut z_offset = 0;
            let mut upper_z = z_8_8;
            let mut lower_z = z_8_8;
            while z_offset < radius {
                let metric = z_offset * z_offset + radius - x_offset * x_offset;
                let points = [
                    (right_x, upper_z),
                    (right_x, lower_z),
                    (left_x, lower_z),
                    (left_x, upper_z),
                ];
                if metric > 1 {
                    for &(x, z) in &points {
                        self.add_point(x >> 8, z >> 8, 7);
                    }
                }
                if metric > 0 {
                    for &(x, z) in &points {
                        self.add_point(x >> 8, z >> 8, 2);
                    }
                }
                z_offset += 1;
                upper_z += 0x100;
                lower_z -= 0x100;
            }
            x_offset += 1;
            left_x -= 0x100;
            right_x += 0x100;
        }
    }
}

/// Exact signed division by 256 used by `FUN_004383F0`.
///
/// Adding 255 before the arithmetic shift for negative values makes the result
/// truncate toward zero, matching the original x86 integer sequence.
fn retail_fixed_cell(raw_8_8: i32) -> i32 {
    raw_8_8.wrapping_add((raw_8_8 >> 31) & 0xff) >> 8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_samples_are_x_major_and_upper_saturated() {
        let mut lights = TerrainLightWindow::default();
        lights.scroll_to_center(16, 16);
        lights.add_point(3, 7, 5);
        assert_eq!(lights.sample(3, 7), 5);
        assert_eq!(lights.sample(7, 3), 0);
        lights.add_point(3, 7, 200);
        assert_eq!(lights.sample(3, 7), 127);
    }

    #[test]
    fn scrolling_preserves_overlap_and_clears_exposed_cells() {
        let mut lights = TerrainLightWindow::default();
        lights.scroll_to_center(16, 16);
        lights.add_point(10, 10, 7);
        lights.scroll_to_center(17, 18);
        assert_eq!(lights.sample(10, 10), 7);
        assert_eq!(lights.sample(0, 0), 0);
        lights.scroll_to_center(100, 100);
        assert_eq!(lights.sample(10, 10), 0);
    }

    #[test]
    fn scrolling_wraps_at_the_256_cell_world_seam() {
        let mut lights = TerrainLightWindow::default();
        lights.scroll_to_center(250, 250);
        lights.add_point(255, 1, 4);
        lights.scroll_to_center(1, 1);
        assert_eq!(lights.sample(255, 1), 4);
    }

    #[test]
    fn beginning_a_world_frame_clears_previous_particle_light() {
        let mut lights = TerrainLightWindow::default();
        lights.begin_world_frame(16 << 8, 6 << 8);
        lights.add_point(16, 16, 7);
        assert_eq!(lights.sample(16, 16), 7);

        lights.begin_world_frame(16 << 8, 6 << 8);
        assert_eq!(lights.sample(16, 16), 0);
    }

    #[test]
    fn beginning_a_world_frame_applies_the_retail_ten_cell_z_lead() {
        let mut lights = TerrainLightWindow::default();
        lights.begin_world_frame(16 << 8, 16 << 8);
        assert_eq!(lights.origin_x, 0);
        assert_eq!(lights.origin_z, 10);
    }

    #[test]
    fn beginning_a_world_frame_truncates_negative_raw_coordinates_toward_zero() {
        let mut lights = TerrainLightWindow::default();
        // X is -1/256. Z becomes -1/256 after the retail +0x0A00 lead.
        // Both truncate to cell zero; arithmetic floor would incorrectly
        // produce cell -1 and origin 239.
        lights.begin_world_frame(-1, -0x0a01);
        assert_eq!(lights.origin_x, 240);
        assert_eq!(lights.origin_z, 240);

        assert_eq!(retail_fixed_cell(-0x101), -1);
        assert_eq!(retail_fixed_cell(-0x100), -1);
        assert_eq!(retail_fixed_cell(-1), 0);
        assert_eq!(retail_fixed_cell(0xff), 0);
        assert_eq!(retail_fixed_cell(0x100), 1);
    }

    #[test]
    fn radial_writer_has_seven_step_core_and_integer_falloff() {
        let mut lights = TerrainLightWindow::default();
        lights.scroll_to_center(10, 10);
        lights.add_radial(10 << 8, 10 << 8, 2 << 8);
        assert_eq!(lights.sample(10, 10), 7);
        assert_eq!(lights.sample(11, 11), 7);
        assert_eq!(lights.sample(10, 9), 6);
        assert_eq!(lights.sample(8, 8), 0);
    }

    #[test]
    fn explosion_writer_preserves_center_axis_overdraw() {
        let mut lights = TerrainLightWindow::default();
        lights.scroll_to_center(10, 10);
        lights.add_explosion(10 << 8, 10 << 8, 2 << 8);
        assert_eq!(lights.sample(10, 10), 36);
        assert_eq!(lights.sample(10, 9), 18);
        assert_eq!(lights.sample(9, 10), 4);
        assert_eq!(lights.sample(8, 8), 0);
    }
}
