//! Water producer: `FUN_00431A60` and its helpers.
//!
//! The water pass walks the same rows as the ground scan (lines of constant
//! world X, first toward -X and then toward +X, retiring leading points that
//! leave through the near plane or the walked-toward screen side) but builds
//! its own points: each sits on the sea surface, displaced by the wave sum
//! where the terrain below lies under the sea (`FUN_00445920`), and is
//! shaded by the surface's rise from the previous point of its row plus the
//! terrain light. Each cell with a submerged corner queues one shoreline
//! sprite chosen and rotated by its four submerged bits: a mapped quad
//! (`+0x10C0`, fogged `+0x10C4`) for the first and last cell of a strip,
//! whose edge UVs are interpolated by the eye's fractional Z, else a shaded
//! quad (`+0x10A8` / `+0x10AC`). Records are keyed by the strip depth word
//! plus `0x180`, so water at a cell's depth paints after its ground
//! (`+0x200`). Unlike the ground, no near-plane reprojection, bottom cap or
//! winding test runs.

use super::queue::{PrimitiveQueue, QueueError};
use super::slots::FillSlot;
use super::terrain::{edge_uvs, Edge, GroundPoint, GroundScene, MAX_POINTS, OUTCODE_VISIBLE};
use crate::water::SHORE_TABLE;
use v2k_formats::terrain::{water_surface_raw, WaterAnimation};

/// The water pass's inputs beyond the ground scan's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaterScene {
    /// Section-10 header dword 0 bits 8..23: the sea's world Y.
    pub sea_level: i16,
    /// The wave clock (`FUN_00431A60`'s second argument) when waves animate
    /// (`DAT_004FECE4`); a static sea lies flat at `sea_level`.
    pub animation: WaterAnimation,
    /// Global sprite index of shoreline shape 0 (`DAT_004DB26C`).
    pub shore_base: u32,
}

/// One 8-byte water point.
#[derive(Debug, Clone, Copy, Default)]
struct WaterPoint {
    point: GroundPoint,
    /// The terrain below lies under the sea (point byte 7).
    submerged: bool,
}

struct Pass<'s, 'a> {
    scene: &'s GroundScene<'a>,
    water: &'s WaterScene,
    queue: &'s mut PrimitiveQueue,
    /// Strip depth words written by the latest row (`u16` each).
    keys: [u16; MAX_POINTS],
}

type Row = [WaterPoint; MAX_POINTS];

impl Pass<'_, '_> {
    fn terrain(&self, x_cell: u32, z_cell: u32) -> i32 {
        i32::from(i16::from(self.scene.cell(x_cell, z_cell).0) << 5)
    }

    /// `FUN_00445920`: the sea surface over terrain height `terrain`.
    fn surface(&self, x_word: u32, z_word: u32, terrain: i32) -> i32 {
        i32::from(water_surface_raw(
            [x_word as i16, z_word as i16],
            self.water.sea_level,
            terrain as i16,
            self.water.animation,
        ))
    }

    /// Fill point `at` on the surface at `position`; returns the surface
    /// height, the next point's `previous`.
    #[allow(clippy::too_many_arguments)]
    fn point(
        &mut self,
        row: &mut Row,
        at: usize,
        previous: i32,
        surface: i32,
        terrain: i32,
        cells: [u32; 2],
        position: [u32; 2],
    ) -> i32 {
        let scene = self.scene;
        let water = &mut row[at];
        water.submerged = terrain < i32::from(self.water.sea_level);
        let shade = ((surface - previous) >> 5) + 3 + scene.light(cells[0], cells[1]);
        water.point.shade = shade.clamp(0, 7) as u8;
        let delta = [
            i32::from((position[0] as i16).wrapping_sub(scene.eye[0])),
            i32::from((surface as i16).wrapping_sub(scene.eye[1])),
            i32::from((position[1] as i16).wrapping_sub(scene.eye[2])),
        ];
        self.keys[at] = scene.projection.project_dry(&mut water.point, delta) as u16;
        surface
    }

    /// `FUN_004321E0`: an interior point at whole cell words.
    fn interior(&mut self, row: &mut Row, at: usize, previous: i32, x: u32, z: u32) -> i32 {
        let cells = [(x & 0xFFFF) >> 8, (z & 0xFFFF) >> 8];
        let terrain = self.terrain(cells[0], cells[1]);
        let surface = self.surface(x, z, terrain);
        self.point(row, at, previous, surface, terrain, cells, [x, z])
    }

    /// `FUN_004324B0`: a row's first or last point, `z_offset` along Z from
    /// cell word `z`; its terrain, type and light stay those of the cell.
    fn edge(
        &mut self,
        row: &mut Row,
        at: usize,
        previous: i32,
        x: u32,
        z: u32,
        z_offset: i32,
    ) -> i32 {
        let cells = [(x & 0xFFFF) >> 8, (z & 0xFFFF) >> 8];
        let terrain = self.terrain(cells[0], cells[1]);
        let z = (z_offset as u32).wrapping_add(z);
        let surface = self.surface(x, z, terrain);
        self.point(row, at, previous, surface, terrain, cells, [x, z])
    }

    /// `FUN_00431D20`: fill `row` for world column word `column` from point
    /// `start` on. The first point's `previous` surface takes its terrain
    /// one column back and its wave one cell back along Z.
    fn build_row(&mut self, row: &mut Row, column: u32, start: usize) {
        let scene = self.scene;
        let points = scene.points as usize;
        let z_sum = u32::from((scene.eye[2] as u16).wrapping_add(scene.lead as u16));
        let mut z_word = z_sum & 0xFF00;
        let back = ((column & 0xFFFF) >> 8).wrapping_sub(1) & 0xFF;
        let (mut index, mut previous) = if start == 0 {
            let terrain = self.terrain(back, z_sum >> 8);
            let previous = self.surface(column, z_sum.wrapping_sub(0x100), terrain);
            let previous = self.edge(row, 0, previous, column, z_word, (z_sum & 0xFF) as i32);
            z_word += 0x100;
            (1, previous)
        } else {
            z_word += start as u32 * 0x100;
            let z_back = z_word.wrapping_sub(0x100) & 0xFFFF;
            let terrain = self.terrain(back, z_back >> 8);
            (start, self.surface(column, z_back, terrain))
        };
        while index < points - 1 {
            previous = self.interior(row, index, previous, column, z_word);
            index += 1;
            z_word += 0x100;
        }
        let fraction = i32::from((scene.eye[2] as u8).wrapping_add(scene.lead as u8)) - 0x100;
        self.edge(row, index, previous, column, z_word, fraction);
    }

    /// `FUN_004327C0`: queue the strip between rows `a` (lower X) and `b`
    /// from cell `start`, keyed by the latest row's depth words.
    fn strip(&mut self, a: &Row, b: &Row, start: usize) -> Result<(), QueueError> {
        let last = self.scene.points as usize - 2;
        let mut cell = start;
        if start == 0 {
            self.cell(a, b, 0, Some(Edge::Leading))?;
            cell = 1;
        }
        while cell < last {
            self.cell(a, b, cell, None)?;
            cell += 1;
        }
        self.cell(a, b, last, Some(Edge::Trailing))
    }

    fn cell(
        &mut self,
        a: &Row,
        b: &Row,
        cell: usize,
        edge: Option<Edge>,
    ) -> Result<(), QueueError> {
        let corners = [a[cell], b[cell], b[cell + 1], a[cell + 1]];
        let code: usize = corners
            .iter()
            .enumerate()
            .map(|(bit, corner)| usize::from(corner.submerged) << bit)
            .sum();
        let outcode = corners
            .iter()
            .fold(0, |code, corner| code | corner.point.clip);
        let fades = corners
            .iter()
            .fold(0xFF, |fade, corner| fade & corner.point.fade);
        if code == 0 || OUTCODE_VISIBLE[usize::from(outcode)] == 0 || fades == 0xFF {
            return Ok(());
        }
        let fogged = corners.iter().any(|corner| corner.point.fade != 0);
        let (slot, bytes) = match (edge.is_some(), fogged) {
            (false, false) => (FillSlot::ShadedQuad, 0x28),
            (false, true) => (FillSlot::ShadedFogQuad, 0x30),
            (true, false) => (FillSlot::MappedShadedQuad, 0x48),
            (true, true) => (FillSlot::MappedShadedFogQuad, 0x50),
        };
        let key = i32::from(self.keys[cell]) + 0x180;
        let entry = SHORE_TABLE[code];
        let material = (self.scene.sprite)(self.water.shore_base + u32::from(entry.frame));
        let slots = entry.slot.map(usize::from);
        let at = self.queue.allocate_sorted(key, slot, bytes)?;
        let shade_words = self.scene.shade_words;
        let fog_colour = self.scene.fog_colour;
        let (colour, fades) = if edge.is_some() {
            (0x48, 0x4C)
        } else {
            (0x28, 0x2C)
        };
        let payload = self.queue.payload_mut(at);
        for (corner, slot) in corners.iter().zip(slots) {
            let point = corner.point;
            payload[4 * slot..4 * slot + 4].copy_from_slice(&point.screen_dword().to_le_bytes());
            let shade = shade_words[usize::from(point.shade)];
            payload[0x14 + 4 * slot..0x18 + 4 * slot].copy_from_slice(&shade.to_le_bytes());
            if fogged {
                payload[fades + slot] = point.fade;
            }
        }
        payload[0x10..0x14].copy_from_slice(&material.id.to_le_bytes());
        payload[0x24..0x28].copy_from_slice(&0u32.to_le_bytes());
        if let Some(edge) = edge {
            let fraction = i32::from((self.scene.eye[2] as u8).wrapping_add(self.scene.lead as u8));
            for (index, [u, v]) in edge_uvs(material, slots, edge, fraction)
                .into_iter()
                .enumerate()
            {
                payload[0x28 + 8 * index..0x2C + 8 * index].copy_from_slice(&u.to_le_bytes());
                payload[0x2C + 8 * index..0x30 + 8 * index].copy_from_slice(&v.to_le_bytes());
            }
        }
        if fogged {
            payload[colour..colour + 4].copy_from_slice(&fog_colour.to_le_bytes());
        }
        Ok(())
    }
}

/// `FUN_00431A60`: queue the water around the eye. `scene` supplies the
/// ground scan's grid, projection, eye, lead, counts, light, shade dwords,
/// fog colour and sprite records; its tile and infection inputs are unused.
pub fn draw_water(
    queue: &mut PrimitiveQueue,
    scene: &GroundScene<'_>,
    water: &WaterScene,
) -> Result<(), QueueError> {
    assert!(
        (3..=MAX_POINTS as u32).contains(&scene.points),
        "water rows hold 3..=30 points"
    );
    let mut pass = Pass {
        scene,
        water,
        queue,
        keys: [0; MAX_POINTS],
    };
    let points = scene.points as usize;
    let origin = (u32::from(scene.eye[0] as u16) | (u32::from(scene.eye[1] as u16) << 16)) & !0xFF;
    let mut rows = [[WaterPoint::default(); MAX_POINTS]; 2];
    for (step, retire) in [(-0x100i32, 0x41u8), (0x100, 0x44)] {
        let mut column = origin;
        let mut current = 0;
        let mut row = rows[current];
        pass.build_row(&mut row, column, 0);
        rows[current] = row;
        let mut start = 0usize;
        let mut walked = 0;
        while start < points - 1 && walked < scene.rows {
            column = column.wrapping_add_signed(step);
            let new = 1 - current;
            let mut row = rows[new];
            pass.build_row(&mut row, column, start);
            rows[new] = row;
            // The strip takes the lower-X row first in both passes.
            let (a, b) = if step < 0 {
                (&rows[new], &rows[current])
            } else {
                (&rows[current], &rows[new])
            };
            pass.strip(a, b, start)?;
            while start < points - 1 && rows[new][start + 1].point.clip & retire != 0 {
                start += 1;
            }
            current = new;
            walked += 1;
        }
    }
    Ok(())
}
