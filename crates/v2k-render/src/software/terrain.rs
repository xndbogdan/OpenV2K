//! Opaque ground producer: `FUN_0042F980` and its helpers.
//!
//! Retail draws the ground as strips of cells between two rows of projected
//! points. Rows are lines of constant world X, one cell apart; every row
//! holds `points` vertices along +Z, starting at the eye's Z plus a signed
//! lead. The scan first walks rows toward -X from the eye's row, then
//! restarts at the eye's row and walks toward +X, retiring leading points
//! once they fall off the near plane or the screen edge it walks toward.
//! Each strip queues:
//!
//! - a flat bottom cap (`FUN_00431970`) under its first point pair;
//! - its first and last cells as explicit-UV mapped quads (`+0x10C0`, fogged
//!   `+0x10C4`) whose leading/trailing edge UVs are interpolated by the
//!   eye's fractional Z, since those cells are only partly in the strip;
//! - every other cell as a shaded quad (`+0x10A8`, fogged `+0x10AC`) whose
//!   corners are permuted onto the canonical transition sprite;
//! - an infection overlay after any cell with an infected corner.
//!
//! Points come from the dry or wet world projector at render-context
//! `+0xB4` (`FUN_0046D1E0`, `FUN_00436440`); points behind the near plane
//! are reprojected at depth 0x40 by `FUN_0042FFF0`. The record layouts,
//! keys (strip-depth word + 0x200) and allocation order are byte-exact.

use super::fixed::mul_q31;
use super::material::MaterialId;
use super::queue::{PrimitiveQueue, QueueError};
use super::slots::FillSlot;
use crate::terrain_tiles::TerrainTile;
use crate::water::SHORE_TABLE;
use v2k_formats::fixed_math::retail_sine_q15;
use v2k_formats::terrain::TerrainGrid;

/// `DAT_004C5268`: non-zero when a primitive with this OR of point outcodes
/// may reach the screen.
pub(crate) const OUTCODE_VISIBLE: [u8; 256] = outcode_table();

/// Points per row and rows are capped by `FUN_004330D0`.
pub const MAX_POINTS: usize = 30;

/// The world projection words published at `0x004FEEA0..0x004FEEF0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroundProjection {
    /// View X, view Y and depth rows, each a dot product with the delta.
    pub axes_q31: [[i32; 3]; 3],
    /// `0x004FEEC4/C8/CC`.
    pub translation: [i32; 3],
    /// `0x004FEED0/D4`.
    pub focal: [i32; 2],
    /// `0x004FEED8/DC`: unsigned outcode bounds (viewport width, height).
    pub bounds: [u32; 2],
    /// `0x004FEEE0/E4`.
    pub centre: [i32; 2],
    /// `0x004FEEE8` (`0x10000 * 255 / (far - near)`), `0x004FEEEC`,
    /// `0x004FEEF0`: the fade byte ramp in depth units.
    pub fade: [i32; 3],
    /// `Some(clock)` selects the wet projector `FUN_00436440`, which wobbles
    /// every point by the presentation clock at `0x004FED60`.
    pub wet_clock: Option<u32>,
}

/// One projected ground point: the 8-byte record `FUN_0042FCC0` fills.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GroundPoint {
    pub screen: [i16; 2],
    pub clip: u8,
    pub fade: u8,
    /// Index into the scene's eight shade dwords.
    pub shade: u8,
    /// The Section-10 cell's type byte.
    pub cell: u8,
}

impl GroundPoint {
    pub(crate) fn screen_dword(self) -> u32 {
        u32::from(self.screen[0] as u16) | (u32::from(self.screen[1] as u16) << 16)
    }
}

/// A bound transition or infection sprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroundMaterial {
    pub id: MaterialId,
    /// Sprite record `+0x10/+0x12`.
    pub width: u16,
    pub height: u16,
}

/// Everything the ground scan reads.
pub struct GroundScene<'a> {
    pub grid: &'a TerrainGrid,
    pub projection: GroundProjection,
    /// Viewport origin words (context `+0x30/+0x32/+0x34`).
    pub eye: [i16; 3],
    /// Signed row lead (context `+0x3C`, see [`ground_lead`]).
    pub lead: i16,
    /// Logical screen height (context `+0x2E`).
    pub screen_height: i16,
    /// The scene's first eight Section-6 dwords (context `+0x00`).
    pub shade_words: [u32; 8],
    /// Fog colour dword (context `+0x28`).
    pub fog_colour: u32,
    /// First dword of system-2 Section-7 entry 11: the bottom-cap colour.
    pub cap_colour: u32,
    /// `DAT_004CAB70`: rows walked in each X direction.
    pub rows: u32,
    /// `DAT_004CAB74`: points per row (2..=30).
    pub points: u32,
    /// `0x004FE820`: 32×32 signed light window, X-major.
    pub light: &'a [i8; 1024],
    /// Light window origin cells `(0x004FEC20, 0x004FE818)`.
    pub light_origin: [u8; 2],
    /// `FUN_00433530`'s sixteen signed motion offsets (context `+0x44`).
    pub infection_offsets: [i8; 16],
    /// Its selector bytes (`0x004CAB88`).
    pub infection_selectors: &'a [u8; 256],
    /// The 625-entry transition table built by `FUN_00433180`.
    pub tiles: &'a [TerrainTile],
    /// Global sprite index of canonical frame 0 (`DAT_004DB270` adds it).
    pub tile_base: u32,
    /// Global sprite index of infection shape 0 (`DAT_004DB268`).
    pub infection_base: u32,
    /// Global sprite records (`DAT_004FE62C`).
    pub sprite: &'a dyn Fn(u32) -> GroundMaterial,
}

/// `FUN_00431890`'s signed row lead from camera basis word 12 (VIEW depth's
/// Y component): `0x200`, or `0x200 + ((word + 0x58000000) * 0x1C00 >> 31)`
/// when that sum is negative.
pub fn ground_lead(depth_y_q31: i32) -> i16 {
    let sum = depth_y_q31.wrapping_add(0x5800_0000);
    if sum < 0 {
        (mul_q31(sum, 0x1C00) as i16).wrapping_add(0x200)
    } else {
        0x200
    }
}

/// `FUN_004330D0`: rows walked per direction (`min(2 * columns, 52)`) and
/// points per row (`min(points, 30)`).
pub fn scan_counts(columns: i32, points: i32) -> (u32, u32) {
    let rows = if columns.wrapping_mul(2) < 0x35 {
        columns.wrapping_mul(2)
    } else {
        0x34
    };
    let points = if points < 0x1F { points } else { 0x1E };
    (rows as u32, points as u32)
}

impl GroundProjection {
    fn view(&self, delta: [i32; 3]) -> [i32; 3] {
        let [dx, dy, dz] = delta;
        let a = self.axes_q31;
        let t = self.translation;
        let depth = mul_q31(a[2][2], dz)
            .wrapping_add(mul_q31(a[2][1], dy))
            .wrapping_add(mul_q31(a[2][0], dx))
            .wrapping_add(t[2]);
        let x = mul_q31(a[0][2], dz)
            .wrapping_add(mul_q31(a[0][1], dy))
            .wrapping_add(mul_q31(a[0][0], dx))
            .wrapping_add(t[0]);
        let y = mul_q31(a[1][1], dy)
            .wrapping_add(mul_q31(a[1][2], dz))
            .wrapping_add(mul_q31(a[1][0], dx))
            .wrapping_add(t[1]);
        [x, y, depth]
    }

    /// The fade byte for `depth`.
    fn fade_byte(&self, depth: i32) -> u8 {
        let [scale, near, far] = self.fade;
        if depth < near {
            0
        } else if depth < far {
            (depth.wrapping_sub(near).wrapping_mul(scale) as u32 >> 16) as u8
        } else {
            0xFF
        }
    }

    /// Perspective, the 0x1FFF screen cap and the fade byte, shared by the
    /// projectors and `FUN_0042FFF0`; returns the outcode without bit 0x40.
    fn finish(&self, point: &mut GroundPoint, x: i32, y: i32, depth: i32) -> u8 {
        let mut sx = quotient(x, self.focal[0], depth).wrapping_add(self.centre[0]);
        let mut sy = self.centre[1].wrapping_sub(quotient(y, self.focal[1], depth));
        let mut control = sx.wrapping_abs() | sy.wrapping_abs();
        while control > 0x1FFF {
            control >>= 1;
            sx >>= 1;
            sy >>= 1;
        }
        point.fade = self.fade_byte(depth);
        if let Some(clock) = self.wet_clock {
            let phase = clock
                .wrapping_add((sy as u32).wrapping_add(sx as u32).wrapping_mul(2))
                .wrapping_mul(0x200);
            sx = sx.wrapping_add(wobble(phase.wrapping_add(0x4000)));
            sy = sy.wrapping_add(wobble(phase));
        }
        point.screen = [sx as i16, sy as i16];
        outcode(sx, sy, self.bounds)
    }

    /// `FUN_0046D1E0` / `FUN_00436440`: project `delta` (point minus eye);
    /// returns the depth.
    pub(crate) fn project(&self, point: &mut GroundPoint, delta: [i32; 3]) -> i32 {
        let [x, y, depth] = self.view(delta);
        if depth < 0x40 {
            point.clip = 0x40;
            return depth;
        }
        point.clip = self.finish(point, x, y, depth);
        depth
    }

    /// The dry projector the water pass inlines (`FUN_004321E0`) or calls
    /// (`FUN_0042FFF0`): a point behind the near plane only takes the 0x40
    /// outcode and keeps its stale screen point and fade. Returns the depth.
    pub(crate) fn project_dry(&self, point: &mut GroundPoint, delta: [i32; 3]) -> i32 {
        let [x, y, depth] = self.view(delta);
        if depth < 0x40 {
            point.clip = 0x40;
        } else {
            let dry = Self {
                wet_clock: None,
                ..*self
            };
            point.clip = dry.finish(point, x, y, depth);
        }
        depth
    }

    /// `FUN_0046D010`: the dry projector without a fade byte. Behind the near
    /// plane it returns 0x40 and leaves `screen` as it was.
    pub(crate) fn project_plain(&self, screen: &mut [i16; 2], delta: [i32; 3]) -> u8 {
        let [x, y, depth] = self.view(delta);
        if depth < 0x40 {
            return 0x40;
        }
        let mut point = GroundPoint::default();
        let dry = Self {
            wet_clock: None,
            ..*self
        };
        let clip = dry.finish(&mut point, x, y, depth);
        *screen = point.screen;
        clip
    }

    /// Project an already transformed VIEW point: outcode (0x40 behind the
    /// near plane), screen point and fade byte. The model vertex cache uses
    /// this with its node's own axes.
    pub(crate) fn project_view(&self, view: [i32; 3]) -> GroundPoint {
        let mut point = GroundPoint::default();
        let [x, y, depth] = view;
        if depth < 0x40 {
            point.clip = 0x40;
        } else {
            point.clip = self.finish(&mut point, x, y, depth);
        }
        point
    }

    /// `FUN_004594C0`: half a projected size in screen pixels on each axis,
    /// halved together until both fit below 0x2000.
    pub(crate) fn half_width(&self, value: i32, depth: i32) -> (i16, i16) {
        let mut x = quotient(value, self.focal[0], depth);
        let mut y = quotient(value, self.focal[1], depth);
        let mut control = x | y;
        while control > 0x1FFF {
            control >>= 1;
            x >>= 1;
            y >>= 1;
        }
        (x as i16, y as i16)
    }

    /// `FUN_0042FFF0`: reproject a point behind the near plane at depth 0x40.
    /// The dry formula is used even under the wet projector.
    fn reproject_near(&self, point: &mut GroundPoint, delta: [i32; 3]) -> u8 {
        let [x, y, _] = self.view(delta);
        let dry = Self {
            wet_clock: None,
            ..*self
        };
        dry.finish(point, x, y, 0x40)
    }
}

/// The signed perspective quotient: divide first when the product could
/// overflow (`|v| >> 12` beyond the depth).
fn quotient(value: i32, focal: i32, depth: i32) -> i32 {
    let divide_first = if value < 0 {
        value >> 12 < depth.wrapping_neg()
    } else {
        depth < value >> 12
    };
    if divide_first {
        (value / depth).wrapping_mul(focal)
    } else {
        value.wrapping_mul(focal) / depth
    }
}

/// `FUN_00436440`'s wobble: the duplicated quarter-sine word as Q31, `>> 30`.
fn wobble(angle: u32) -> i32 {
    retail_sine_q15(angle & 0xFFFF).wrapping_mul(0x1_0001) >> 30
}

/// X bits 1 (left) / 2 (inside) / 4 (right), Y bits 8 (above) / 0x10
/// (inside) / 0x20 (below), from unsigned comparisons with the bounds.
pub(crate) fn outcode_of(x: i32, y: i32, bounds: [u32; 2]) -> u8 {
    outcode(x, y, bounds)
}

fn outcode(x: i32, y: i32, bounds: [u32; 2]) -> u8 {
    let horizontal = if (x as u32) < bounds[0] {
        2
    } else if x >= 0 {
        4
    } else {
        1
    };
    let vertical = if (y as u32) < bounds[1] {
        0x10
    } else if y >= 0 {
        0x20
    } else {
        8
    };
    horizontal | vertical
}

/// `DAT_004C5268`: visible unless every point shares an outside side or the
/// near flag is set without an inside point.
const fn outcode_table() -> [u8; 256] {
    // Rebuilt from its definition: a primitive is rejected when its OR of
    // outcodes lacks an inside bit on either axis or contains the near flag.
    let mut table = [0u8; 256];
    let mut code = 0;
    while code < 256 {
        let horizontal_inside = code & 0x02 != 0 || (code & 0x01 != 0 && code & 0x04 != 0);
        let vertical_inside = code & 0x10 != 0 || (code & 0x08 != 0 && code & 0x20 != 0);
        table[code] = (horizontal_inside && vertical_inside && code & 0x40 == 0) as u8;
        code += 1;
    }
    table
}

/// Mutable state of one scan: the two point rows and the depth words.
struct Scan<'s, 'a> {
    scene: &'s GroundScene<'a>,
    queue: &'s mut PrimitiveQueue,
    /// Strip depth words written by the latest row (`u16` each).
    keys: [u16; MAX_POINTS],
}

impl GroundScene<'_> {
    pub(crate) fn cell(&self, x: u32, z: u32) -> (i8, u8) {
        let cell = self
            .grid
            .cell((x & 0xFF) as usize, (z & 0xFF) as usize)
            .expect("256x256 grid");
        (cell.height as i8, cell.terrain_type)
    }

    pub(crate) fn light(&self, x_cell: u32, z_cell: u32) -> i32 {
        let x = x_cell.wrapping_sub(u32::from(self.light_origin[0])) & 0xFF;
        let z = z_cell.wrapping_sub(u32::from(self.light_origin[1])) & 0xFF;
        if x < 0x20 && z < 0x20 {
            i32::from(self.light[(z + x * 0x20) as usize])
        } else {
            0
        }
    }

    /// Shade index 0..=7: type bits 5..7 plus light, darkened below the
    /// darkness start by `(height - start) * 8 / range` (or 8 past range).
    fn shade(&self, cell: u8, light: i32, height: i32) -> u8 {
        let mut shade = i32::from(cell >> 5) + light;
        let start = i32::from(self.grid.darkness_start_world_y());
        let range = i32::from(self.grid.darkness_range());
        if height < start {
            if height < start - range {
                shade -= 8;
            } else {
                shade += (height - start) * 8 / range;
            }
        }
        shade.clamp(0, 7) as u8
    }

    /// Project a point, reprojecting it at the near plane when it is behind;
    /// returns its strip depth word.
    fn project(&self, point: &mut GroundPoint, delta: [i32; 3]) -> u16 {
        let depth = self.projection.project(point, delta);
        if point.clip & 0x40 != 0 {
            point.clip = self.projection.reproject_near(point, delta);
            0x40
        } else {
            depth as u16
        }
    }

    /// `FUN_00430140`: a point at the fractional cell position
    /// `(column + x_offset, z_word + z_offset)`, its height bilinear.
    fn fractional_point(
        &self,
        point: &mut GroundPoint,
        column: u32,
        x_offset: i32,
        z_word: u32,
        z_offset: i32,
    ) -> u16 {
        let x_position = (x_offset as u32).wrapping_add(column);
        let x_cell = (x_position & 0xFFFF) >> 8;
        let x_fraction = (x_position & 0xFF) as i32;
        let z_position = (z_offset as u32).wrapping_add(z_word);
        let z_cell = (z_position & 0xFFFF) >> 8;
        let z_fraction = (z_position & 0xFF) as i32;
        let height = |x: u32, z: u32| i32::from(self.cell(x, z).0) * 0x20;
        let near = height(x_cell, z_cell);
        let along_near = (((height(x_cell + 1, z_cell) - near) * x_fraction) >> 8) + near;
        let far = height(x_cell, z_cell + 1);
        let along_far = (((height(x_cell + 1, z_cell + 1) - far) * x_fraction) >> 8) + far;
        let height = i32::from(
            ((((along_far - along_near) * z_fraction) >> 8) as i16).wrapping_add(along_near as i16),
        );
        // Type, light and motion use the whole cells of the un-offset words.
        let x_cell = (column & 0xFFFF) >> 8;
        let z_cell = (z_word & 0xFFFF) >> 8;
        let cell = self.cell(x_cell, z_cell).1;
        point.cell = cell;
        point.shade = self.shade(cell, self.light(x_cell, z_cell), height);
        let mut x_word = column as i16;
        let mut z_word = z_word as i16;
        if cell & 0x10 != 0 {
            let b = (z_word >> 4) as u16 as u32;
            let index = (((column >> 8) ^ b) & 0xF) ^ (b & 0xFF);
            let selector = self.infection_selectors[index as usize];
            x_word = x_word.wrapping_add(i16::from(
                self.infection_offsets[usize::from(selector & 0xF)],
            ));
            z_word = z_word.wrapping_add(i16::from(
                self.infection_offsets[usize::from(selector >> 4)],
            ));
        }
        let delta = [
            i32::from(x_word.wrapping_add((x_offset as i16).wrapping_sub(self.eye[0]))),
            height - i32::from(self.eye[1]),
            i32::from(z_word.wrapping_add((z_offset as i16).wrapping_sub(self.eye[2]))),
        ];
        self.project(point, delta)
    }
}

impl Scan<'_, '_> {
    /// `FUN_0042FCC0`: fill `row` for world column word `column` from point
    /// `start` on, writing each point's strip depth word.
    fn build_row(&mut self, row: &mut [GroundPoint; MAX_POINTS], column: u32, start: usize) {
        let scene = self.scene;
        let points = scene.points as usize;
        let z_sum = (scene.eye[2] as u16).wrapping_add(scene.lead as u16);
        let z_base = u32::from(z_sum) & 0xFF00;
        let (mut index, mut z_word) = if start == 0 {
            self.keys[0] =
                scene.fractional_point(&mut row[0], column, 0, z_base, i32::from(z_sum & 0xFF));
            (1, z_base + 0x100)
        } else {
            (start, z_base + start as u32 * 0x100)
        };
        let x_cell = (column & 0xFFFF) >> 8;
        while index < points - 1 {
            let z_cell = (z_word & 0xFFFF) >> 8;
            let (height_byte, cell) = scene.cell(x_cell, z_cell);
            let height = i32::from(i16::from(height_byte) << 5);
            let point = &mut row[index];
            point.cell = cell;
            point.shade = scene.shade(cell, scene.light(x_cell, z_cell), height);
            let mut x_word = column as i16;
            let mut z = z_word as i16;
            if cell & 0x10 != 0 {
                let index = ((column >> 8) & 0xF) ^ (((z >> 4) as u16 as u32) & 0xFF);
                let selector = scene.infection_selectors[index as usize];
                x_word = x_word.wrapping_add(i16::from(
                    scene.infection_offsets[usize::from(selector & 0xF)],
                ));
                z = z.wrapping_add(i16::from(
                    scene.infection_offsets[usize::from(selector >> 4)],
                ));
            }
            let delta = [
                i32::from(x_word.wrapping_sub(scene.eye[0])),
                height - i32::from(scene.eye[1]),
                i32::from(z.wrapping_sub(scene.eye[2])),
            ];
            self.keys[index] = scene.project(point, delta);
            index += 1;
            z_word += 0x100;
        }
        let fraction = i32::from((scene.eye[2] as u8).wrapping_add(scene.lead as u8)) - 0x100;
        self.keys[index] = scene.fractional_point(&mut row[index], column, 0, z_word, fraction);
    }

    /// `FUN_00431970`: the flat cap from the strip's first points `a`
    /// (lower X) and `b` down to the screen bottom, keyed zero.
    fn cap(&mut self, a: GroundPoint, b: GroundPoint) -> Result<(), QueueError> {
        if OUTCODE_VISIBLE[usize::from(b.clip | a.clip | 0x20)] == 0 {
            return Ok(());
        }
        let height = self.scene.screen_height;
        let payload = self.queue.push_sorted(0, FillSlot::FlatQuad, 0x18)?;
        let corners = [
            (a.screen[0], height),
            (b.screen[0], height),
            (b.screen[0], b.screen[1]),
            (a.screen[0], a.screen[1]),
        ];
        // Retail stores the words in a different order; the result is the
        // same 0x18 bytes.
        for (index, (x, y)) in corners.into_iter().enumerate() {
            payload[4 * index..4 * index + 2].copy_from_slice(&x.to_le_bytes());
            payload[4 * index + 2..4 * index + 4].copy_from_slice(&y.to_le_bytes());
        }
        payload[0x10..0x14].copy_from_slice(&self.scene.cap_colour.to_le_bytes());
        payload[0x14..0x18].copy_from_slice(&0u32.to_le_bytes());
        Ok(())
    }

    /// `FUN_00430430`: queue the strip between rows `a` (lower X) and `b`
    /// from cell `start`, keyed by the latest row's depth words.
    fn strip(
        &mut self,
        a: &[GroundPoint; MAX_POINTS],
        b: &[GroundPoint; MAX_POINTS],
        start: usize,
    ) -> Result<(), QueueError> {
        let last = self.scene.points as usize - 2;
        let mut cell = start;
        if start == 0 {
            self.edge_cell(a, b, 0, Edge::Leading)?;
            cell = 1;
        }
        while cell < last {
            self.middle_cell(a, b, cell)?;
            cell += 1;
        }
        self.edge_cell(a, b, last, Edge::Trailing)
    }

    fn corners(
        a: &[GroundPoint; MAX_POINTS],
        b: &[GroundPoint; MAX_POINTS],
        cell: usize,
    ) -> [GroundPoint; 4] {
        [a[cell], b[cell], b[cell + 1], a[cell + 1]]
    }

    /// Visible, not fully faded, and wound clockwise on screen (`FUN_004709B0`
    /// false).
    fn admitted(corners: &[GroundPoint; 4]) -> bool {
        let outcode = corners.iter().fold(0, |code, point| code | point.clip);
        let fades = corners.iter().fold(0xFF, |fade, point| fade & point.fade);
        OUTCODE_VISIBLE[usize::from(outcode)] != 0
            && fades != 0xFF
            && !crossed(corners[0], corners[1], corners[2], corners[3])
    }

    fn fogged(corners: &[GroundPoint; 4]) -> bool {
        corners.iter().any(|point| point.fade != 0)
    }

    fn tile(&self, corners: &[GroundPoint; 4]) -> (GroundMaterial, [usize; 4]) {
        let digit = |point: &GroundPoint| usize::from(point.cell & 7);
        let index = ((digit(&corners[3]) * 5 + digit(&corners[2])) * 5 + digit(&corners[1])) * 5
            + digit(&corners[0]);
        let tile = self.scene.tiles[index];
        let material = (self.scene.sprite)(self.scene.tile_base + u32::from(tile.frame));
        (material, tile.slots.map(usize::from))
    }

    fn overlay(&self, mask: usize) -> (GroundMaterial, [usize; 4]) {
        let entry = SHORE_TABLE[mask];
        let material = (self.scene.sprite)(self.scene.infection_base + u32::from(entry.frame));
        (material, entry.slot.map(usize::from))
    }

    fn infection_mask(corners: &[GroundPoint; 4]) -> usize {
        corners
            .iter()
            .enumerate()
            .map(|(bit, point)| usize::from((point.cell & 0x10) >> 4) << bit)
            .sum()
    }

    /// The first (leading) or last (trailing) cell of a strip: a mapped quad
    /// whose partial edge UVs are interpolated by the eye's fraction.
    fn edge_cell(
        &mut self,
        a: &[GroundPoint; MAX_POINTS],
        b: &[GroundPoint; MAX_POINTS],
        cell: usize,
        edge: Edge,
    ) -> Result<(), QueueError> {
        let corners = Self::corners(a, b, cell);
        if !Self::admitted(&corners) {
            return Ok(());
        }
        let fogged = Self::fogged(&corners);
        let (slot, bytes) = if fogged {
            (FillSlot::MappedShadedFogQuad, 0x50)
        } else {
            (FillSlot::MappedShadedQuad, 0x48)
        };
        let key = i32::from(self.keys[cell]) + 0x200;
        let fraction = i32::from((self.scene.eye[2] as u8).wrapping_add(self.scene.lead as u8));
        let base = self.queue.allocate_sorted(key, slot, bytes)?;
        let (material, slots) = self.tile(&corners);
        self.write_material_and_uvs(base, material, slots, edge, fraction);
        let mask = Self::infection_mask(&corners);
        let overlay = if mask == 0 {
            None
        } else {
            let at = self.queue.allocate_sorted(key, slot, bytes)?;
            let (material, slots) = self.overlay(mask);
            self.write_material_and_uvs(at, material, slots, edge, fraction);
            Some((at, slots))
        };
        self.write_corners(base, &corners, slots, fogged, 0x4C, 0x48);
        if let Some((at, slots)) = overlay {
            self.write_corners(at, &corners, slots, fogged, 0x4C, 0x48);
        }
        Ok(())
    }

    fn write_material_and_uvs(
        &mut self,
        at: usize,
        material: GroundMaterial,
        slots: [usize; 4],
        edge: Edge,
        fraction: i32,
    ) {
        let out = edge_uvs(material, slots, edge, fraction);
        let payload = self.queue.payload_mut(at);
        payload[0x10..0x14].copy_from_slice(&material.id.to_le_bytes());
        payload[0x24..0x28].copy_from_slice(&0u32.to_le_bytes());
        for (index, [u, v]) in out.into_iter().enumerate() {
            payload[0x28 + 8 * index..0x2C + 8 * index].copy_from_slice(&u.to_le_bytes());
            payload[0x2C + 8 * index..0x30 + 8 * index].copy_from_slice(&v.to_le_bytes());
        }
    }

    /// Corners, shade dwords and (fogged) fade bytes and colour, permuted by
    /// `slots`; `fades`/`colour` are payload offsets.
    fn write_corners(
        &mut self,
        at: usize,
        corners: &[GroundPoint; 4],
        slots: [usize; 4],
        fogged: bool,
        fades: usize,
        colour: usize,
    ) {
        let shade_words = self.scene.shade_words;
        let fog_colour = self.scene.fog_colour;
        let payload = self.queue.payload_mut(at);
        for (point, slot) in corners.iter().zip(slots) {
            payload[4 * slot..4 * slot + 4].copy_from_slice(&point.screen_dword().to_le_bytes());
            let shade = shade_words[usize::from(point.shade)];
            payload[0x14 + 4 * slot..0x18 + 4 * slot].copy_from_slice(&shade.to_le_bytes());
            if fogged {
                payload[fades + slot] = point.fade;
            }
        }
        if fogged {
            payload[colour..colour + 4].copy_from_slice(&fog_colour.to_le_bytes());
        }
    }

    /// An interior cell: a shaded quad with implicit UVs.
    fn middle_cell(
        &mut self,
        a: &[GroundPoint; MAX_POINTS],
        b: &[GroundPoint; MAX_POINTS],
        cell: usize,
    ) -> Result<(), QueueError> {
        let corners = Self::corners(a, b, cell);
        if !Self::admitted(&corners) {
            return Ok(());
        }
        let fogged = Self::fogged(&corners);
        let (slot, bytes) = if fogged {
            (FillSlot::ShadedFogQuad, 0x30)
        } else {
            (FillSlot::ShadedQuad, 0x28)
        };
        let key = i32::from(self.keys[cell]) + 0x200;
        let mask = Self::infection_mask(&corners);
        if mask == 0xF {
            // Fully infected: only the full infection shape, unrotated.
            let at = self.queue.allocate_sorted(key, slot, bytes)?;
            let material =
                (self.scene.sprite)(self.scene.infection_base + u32::from(SHORE_TABLE[0xF].frame));
            self.write_shaded_material(at, material);
            self.write_corners(at, &corners, [0, 1, 2, 3], fogged, 0x2C, 0x28);
            return Ok(());
        }
        let base = self.queue.allocate_sorted(key, slot, bytes)?;
        let (material, slots) = self.tile(&corners);
        self.write_shaded_material(base, material);
        let overlay = if mask == 0 {
            None
        } else {
            let at = self.queue.allocate_sorted(key, slot, bytes)?;
            let (material, slots) = self.overlay(mask);
            self.write_shaded_material(at, material);
            Some((at, slots))
        };
        self.write_corners(base, &corners, slots, fogged, 0x2C, 0x28);
        if let Some((at, slots)) = overlay {
            self.write_corners(at, &corners, slots, fogged, 0x2C, 0x28);
        }
        Ok(())
    }

    fn write_shaded_material(&mut self, at: usize, material: GroundMaterial) {
        let payload = self.queue.payload_mut(at);
        payload[0x10..0x14].copy_from_slice(&material.id.to_le_bytes());
        payload[0x24..0x28].copy_from_slice(&0u32.to_le_bytes());
    }
}

/// The first or last cell of a strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Edge {
    Leading,
    Trailing,
}

/// A strip edge cell's 16.16 UVs per sprite slot: the cell is only partly
/// in the strip, so its leading (or trailing) slots are interpolated toward
/// the far ones by the eye's fractional Z.
pub(crate) fn edge_uvs(
    material: GroundMaterial,
    slots: [usize; 4],
    edge: Edge,
    fraction: i32,
) -> [[i32; 2]; 4] {
    let width = (i32::from(material.width) << 16).wrapping_sub(1);
    let height = (i32::from(material.height) << 16).wrapping_sub(1);
    let uv = [[0, 0], [width, 0], [width, height], [0, height]];
    let blend = |from: [i32; 2], to: [i32; 2]| -> [i32; 2] {
        std::array::from_fn(|axis| {
            (to[axis].wrapping_sub(from[axis]))
                .wrapping_mul(fraction)
                .wrapping_add(from[axis].wrapping_mul(0x100))
                >> 8
        })
    };
    let [s0, s1, s2, s3] = slots;
    let mut out = [[0i32; 2]; 4];
    match edge {
        Edge::Leading => {
            out[s2] = uv[s2];
            out[s3] = uv[s3];
            out[s0] = blend(uv[s0], uv[s3]);
            out[s1] = blend(uv[s1], uv[s2]);
        }
        Edge::Trailing => {
            out[s0] = uv[s0];
            out[s1] = uv[s1];
            out[s2] = blend(uv[s1], uv[s2]);
            out[s3] = blend(uv[s0], uv[s3]);
        }
    }
    out
}

/// `FUN_004709B0`: the quad's screen winding is counter-clockwise on both
/// of its triangles.
fn crossed(p1: GroundPoint, p2: GroundPoint, p3: GroundPoint, p4: GroundPoint) -> bool {
    let [x1, y1] = p1.screen.map(i32::from);
    let [x2, y2] = p2.screen.map(i32::from);
    let [x3, y3] = p3.screen.map(i32::from);
    let [x4, y4] = p4.screen.map(i32::from);
    if (x2 - x1) * (y3 - y1) - (y2 - y1) * (x3 - x1) < 1 {
        return false;
    }
    (y4 - y1) * (x3 - x1) - (x4 - x1) * (y3 - y1) > 0
}

/// `FUN_0042F980`: queue the ground around the eye.
pub fn draw_ground(queue: &mut PrimitiveQueue, scene: &GroundScene<'_>) -> Result<(), QueueError> {
    // Two-point rows make retail read a cell past the row; no level uses them.
    assert!(
        (3..=MAX_POINTS as u32).contains(&scene.points),
        "ground rows hold 3..=30 points"
    );
    let mut scan = Scan {
        scene,
        queue,
        keys: [0; MAX_POINTS],
    };
    let points = scene.points as usize;
    // The eye's X and Y words as one dword with X's fraction cleared; only
    // the low word reaches the row builder.
    let origin = (u32::from(scene.eye[0] as u16) | (u32::from(scene.eye[1] as u16) << 16)) & !0xFF;
    let mut rows = [[GroundPoint::default(); MAX_POINTS]; 2];
    for (step, retire) in [(-0x100i32, 0x41u8), (0x100, 0x44)] {
        let mut column = origin;
        let mut current = 0;
        scan.build_row(&mut rows[current], column, 0);
        let mut start = 0usize;
        let mut walked = 0;
        while start < points - 1 && walked < scene.rows {
            column = column.wrapping_add_signed(step);
            let new = 1 - current;
            scan.build_row(&mut rows[new], column, start);
            // The cap and strip take the lower-X row first in both passes.
            let (a, b) = if step < 0 {
                (rows[new], rows[current])
            } else {
                (rows[current], rows[new])
            };
            if start == 0 {
                scan.cap(a[0], b[0])?;
            }
            scan.strip(&a, &b, start)?;
            // Retire leading points that left through the near plane or the
            // screen side this pass walks toward.
            while start < points - 1 && rows[new][start + 1].clip & retire != 0 {
                start += 1;
            }
            current = new;
            walked += 1;
        }
    }
    Ok(())
}
