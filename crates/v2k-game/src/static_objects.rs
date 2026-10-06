//! Static terrain-object placement for the world rendering pass.
//!
//! Section 10 stores an object-descriptor index in each cell's attribute
//! byte. Section 9 supplies four model states for that descriptor. Retail
//! places the selected model at raw cell centre `0x80` and at the average of
//! the four signed terrain-corner heights (`FUN_0042F650`). Collision has a
//! deliberately distinct `0x7f` centre (`FUN_00427410`).

use v2k_formats::anim_frames::TerrainObjectTable;
use v2k_formats::terrain::{TerrainGrid, GRID_SIZE};

const RENDER_CELL_CENTER: f32 = 0.5;

#[derive(Debug, Clone, Copy)]
struct TerrainViewFootprint {
    camera_xz: [f32; 2],
    forward: [f32; 2],
    right: [f32; 2],
    half_width: f32,
    near: f32,
    far: f32,
}

impl TerrainViewFootprint {
    fn for_free_camera_entities(
        camera_position: [f32; 3],
        camera_forward: [f32; 3],
        scan_dimensions: (u32, u32),
    ) -> Self {
        let forward = horizontal_forward(camera_forward);
        let near = v2k_core::render_scan::terrain_row_lead(camera_forward[1]);
        Self {
            camera_xz: [camera_position[0], camera_position[2]],
            forward,
            right: [-forward[1], forward[0]],
            half_width: scan_dimensions.0 as f32 * 0.5,
            near,
            far: near + scan_dimensions.1 as f32,
        }
    }

    /// Static objects use `FUN_0042F530`'s deliberately inset traversal rather
    /// than every cell covered by opaque terrain.  Its X loop starts at the
    /// terrain start plus one and stops two cells before the terminal column;
    /// its row loop starts one cell after the terrain lead and shares the
    /// terrain terminal row.  That is `(columns - 3) x (rows - 1)` cells,
    /// centred laterally and inset by one row at the near edge.
    fn for_static_objects(
        camera_position: [f32; 3],
        camera_forward: [f32; 3],
        scan_dimensions: (u32, u32),
    ) -> Self {
        let forward = horizontal_forward(camera_forward);
        let terrain_near = v2k_core::render_scan::terrain_row_lead(camera_forward[1]);
        let columns = scan_dimensions.0.saturating_sub(3);
        let rows = scan_dimensions.1.saturating_sub(1);
        let near = terrain_near + 1.0;
        Self {
            camera_xz: [camera_position[0], camera_position[2]],
            forward,
            right: [-forward[1], forward[0]],
            half_width: columns as f32 * 0.5,
            near,
            far: near + rows as f32,
        }
    }

    fn projected(self, position: [f32; 3]) -> (f32, f32) {
        let dx = position[0] - self.camera_xz[0];
        let dz = position[2] - self.camera_xz[1];
        let along = dx * self.forward[0] + dz * self.forward[1];
        let across = dx * self.right[0] + dz * self.right[1];
        (along, across)
    }

    fn contains_terrain(self, position: [f32; 3]) -> bool {
        let (along, across) = self.projected(position);
        (self.near..self.far).contains(&along) && across.abs() < self.half_width
    }
}

/// Port-only scan window for the developer free camera. This retains the
/// existing camera-relative horizontal footprint when looking in arbitrary
/// directions; it is not retail `FUN_00411400`'s world-axis main-model gate.
/// `position` must already be the nearest toroidal image of the entity.
pub fn free_camera_entity_view_contains_position(
    camera_position: [f32; 3],
    camera_forward: [f32; 3],
    scan_dimensions: (u32, u32),
    position: [f32; 3],
) -> bool {
    let footprint = TerrainViewFootprint::for_free_camera_entities(
        camera_position,
        camera_forward,
        scan_dimensions,
    );
    let (along, across) = footprint.projected(position);
    along <= scan_dimensions.1 as f32 && across.abs() <= footprint.half_width
}

/// One visible static terrain-object instance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StaticTerrainObjectInstance {
    /// Global Section-8 model id selected by terrain-type bits 3-4.
    pub model_id: u16,
    /// Index into retail's 44-byte static-object kind table.
    pub kind_index: u32,
    /// Section-10 descriptor index. Zero is never emitted.
    pub attribute: u8,
    /// Authored Section-10 terrain-type byte for this cell. Besides selecting
    /// one of the descriptor's four models, bit 3 drives the retail static
    /// smoke/fire emitter in `FUN_0042F650`.
    pub terrain_type: u8,
    /// Unwrapped world-cell coordinate. Sampling this cell uses modulo 256,
    /// while retaining this image keeps models adjacent to the camera at the
    /// world seam.
    pub cell: [i32; 2],
    /// Unwrapped port-world position at retail's authored tile centre.
    pub position: [f32; 3],
}

impl StaticTerrainObjectInstance {
    /// Exact world point returned by the static draw context's type-14
    /// callback at `0x00427050`.
    ///
    /// The callback ignores the vertex record's three operands, copies the
    /// current cell centre X/Z, and forces Y to absolute world zero. It is not
    /// the type-13 view pin used by tree trunks and other organic supports.
    pub fn external_frame_world_point(self) -> [f32; 3] {
        [self.position[0], 0.0, self.position[2]]
    }
}

/// Collect static terrain objects in retail's inset camera-relative traversal.
/// `FUN_0042F530` intentionally visits three fewer columns and one fewer row
/// than the opaque terrain scan; it is not a second full terrain footprint.
///
/// `camera_forward` may include pitch; its XZ projection is normalized in the
/// same way as `GlRenderer::set_camera`. A vertical/degenerate direction uses
/// the renderer's `[0, 0, -1]` fallback.
pub fn collect_static_terrain_objects(
    terrain: &TerrainGrid,
    objects: &TerrainObjectTable,
    camera_position: [f32; 3],
    camera_forward: [f32; 3],
    scan_dimensions: (u32, u32),
) -> Vec<StaticTerrainObjectInstance> {
    let footprint =
        TerrainViewFootprint::for_static_objects(camera_position, camera_forward, scan_dimensions);
    let half_width = footprint.half_width;
    let max_along = footprint.near.abs().max(footprint.far.abs());
    let max_radius = (half_width * half_width + max_along * max_along)
        .sqrt()
        .ceil() as i32
        + 1;
    let center_x = camera_position[0].floor() as i32;
    let center_z = camera_position[2].floor() as i32;
    let mut instances = Vec::new();

    // Preserve the terrain renderer's Z-major visit order so later sorting or
    // deterministic diagnostics observe the same footprint ordering.
    for world_z in (center_z - max_radius)..=(center_z + max_radius) {
        for world_x in (center_x - max_radius)..=(center_x + max_radius) {
            let cell_center = [
                world_x as f32 + RENDER_CELL_CENTER,
                camera_position[1],
                world_z as f32 + RENDER_CELL_CENTER,
            ];
            if !footprint.contains_terrain(cell_center) {
                continue;
            }

            let x = world_x.rem_euclid(GRID_SIZE as i32) as usize;
            let z = world_z.rem_euclid(GRID_SIZE as i32) as usize;
            let cell = terrain.cell(x, z).expect("wrapped terrain cell");
            if cell.attribute == 0 {
                continue;
            }
            let Some(descriptor) = objects.records.get(usize::from(cell.attribute)) else {
                continue;
            };

            let xn = (x + 1) % GRID_SIZE;
            let zn = (z + 1) % GRID_SIZE;
            let height_sum_raw = [(x, z), (xn, z), (x, zn), (xn, zn)]
                .map(|(corner_x, corner_z)| {
                    let corner = terrain
                        .cell(corner_x, corner_z)
                        .expect("wrapped terrain corner");
                    i32::from(corner.height as i8) * 32
                })
                .into_iter()
                .sum::<i32>();
            // Rust's signed division truncates toward zero, matching the
            // correction around retail's arithmetic shift for negative sums.
            let center_y_raw = height_sum_raw / 4;

            instances.push(StaticTerrainObjectInstance {
                model_id: descriptor.model_id_for(cell.terrain_type),
                kind_index: descriptor.kind_index,
                attribute: cell.attribute,
                terrain_type: cell.terrain_type,
                cell: [world_x, world_z],
                position: [
                    world_x as f32 + RENDER_CELL_CENTER,
                    center_y_raw as f32 / 256.0,
                    world_z as f32 + RENDER_CELL_CENTER,
                ],
            });
        }
    }

    instances
}

fn horizontal_forward(forward: [f32; 3]) -> [f32; 2] {
    let length = (forward[0] * forward[0] + forward[2] * forward[2]).sqrt();
    if length > 1.0e-5 {
        [forward[0] / length, forward[2] / length]
    } else {
        [0.0, -1.0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::anim_frames::{ModelSlotPattern, TerrainObjectDescriptor};
    use v2k_formats::terrain::TerrainCell;

    fn flat_terrain() -> TerrainGrid {
        TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn object_table(attribute: u8, model_ids: [u16; 4], kind_index: u32) -> TerrainObjectTable {
        let empty = TerrainObjectDescriptor {
            model_ids: [0; 4],
            kind_index: 0,
            pattern: ModelSlotPattern::Static,
        };
        let mut records = vec![empty; 256];
        records[usize::from(attribute)] = TerrainObjectDescriptor {
            model_ids,
            kind_index,
            pattern: ModelSlotPattern::Varied,
        };
        TerrainObjectTable { records }
    }

    fn cell_mut(terrain: &mut TerrainGrid, x: usize, z: usize) -> &mut TerrainCell {
        &mut terrain.cells[x * GRID_SIZE + z]
    }

    #[test]
    fn selects_model_and_places_at_retail_four_corner_center() {
        let mut terrain = flat_terrain();
        *cell_mut(&mut terrain, 10, 7) = TerrainCell {
            height: (-8_i8) as u8,
            attribute: 5,
            terrain_type: 0x10,
        };
        cell_mut(&mut terrain, 11, 7).height = 4;
        cell_mut(&mut terrain, 10, 8).height = 8;
        cell_mut(&mut terrain, 11, 8).height = 12;
        let table = object_table(5, [100, 101, 102, 103], 29);

        let instances = collect_static_terrain_objects(
            &terrain,
            &table,
            [10.0, 3.0, 10.5],
            [0.0, -0.5, -2.0],
            (8, 8),
        );

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].model_id, 102);
        assert_eq!(instances[0].kind_index, 29);
        assert_eq!(instances[0].attribute, 5);
        assert_eq!(instances[0].cell, [10, 7]);
        assert_eq!(
            instances[0].position,
            [10.0 + RENDER_CELL_CENTER, 0.5, 7.0 + RENDER_CELL_CENTER]
        );
        assert_eq!(
            instances[0].external_frame_world_point(),
            [10.0 + RENDER_CELL_CENTER, 0.0, 7.0 + RENDER_CELL_CENTER]
        );
    }

    #[test]
    fn wraps_samples_but_keeps_the_camera_near_unwrapped_image() {
        let mut terrain = flat_terrain();
        *cell_mut(&mut terrain, 0, 10) = TerrainCell {
            height: 8,
            attribute: 7,
            terrain_type: 0x08,
        };
        cell_mut(&mut terrain, 1, 10).height = 8;
        cell_mut(&mut terrain, 0, 11).height = 8;
        cell_mut(&mut terrain, 1, 11).height = 8;
        let table = object_table(7, [200, 201, 202, 203], 9);

        let instances = collect_static_terrain_objects(
            &terrain,
            &table,
            [253.0, 0.0, 10.0],
            [4.0, 0.0, 0.0],
            (8, 8),
        );

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].cell, [256, 10]);
        assert_eq!(instances[0].model_id, 201);
        assert_eq!(
            instances[0].position,
            [256.0 + RENDER_CELL_CENTER, 1.0, 10.0 + RENDER_CELL_CENTER]
        );
    }

    #[test]
    fn matches_retail_inset_static_scan_boundaries() {
        let mut terrain = flat_terrain();
        let table = object_table(3, [77; 4], 1);
        for (x, z) in [(20, 18), (20, 17), (20, 14), (21, 17), (22, 17), (23, 17)] {
            cell_mut(&mut terrain, x, z).attribute = 3;
        }

        // Forward -Z with a terrain scan of 8x8 has terrain near=2.  Retail's
        // static loop starts one row later, retains seven rows, and visits the
        // centred five-column interval.  z=18 lies before the inset edge;
        // z=14/17 and x=20/21/22 remain, while x=23 is outside the five
        // columns.
        let instances = collect_static_terrain_objects(
            &terrain,
            &table,
            [20.5, 0.0, 20.5],
            [0.0, 0.0, -1.0],
            (8, 8),
        );

        assert_eq!(
            instances
                .iter()
                .map(|instance| instance.cell)
                .collect::<Vec<_>>(),
            vec![[20, 14], [20, 17], [21, 17], [22, 17]]
        );
    }

    #[test]
    fn tiny_authored_scans_saturate_without_underflow() {
        let footprint =
            TerrainViewFootprint::for_static_objects([0.0, 0.0, 0.0], [0.0, 0.0, -1.0], (2, 1));

        assert_eq!(footprint.half_width, 0.0);
        assert_eq!(footprint.near, footprint.far);
    }

    #[test]
    fn free_camera_entity_visibility_preserves_its_rotated_scan_bounds() {
        let camera = [20.5, 4.0, 20.5];
        let forward = [0.0, -0.4, -1.0];
        let scan = (4, 4);

        assert!(free_camera_entity_view_contains_position(
            camera,
            forward,
            scan,
            [20.5, 100.0, 17.5]
        ));
        assert!(free_camera_entity_view_contains_position(
            camera,
            forward,
            scan,
            [20.5, 0.0, 19.0]
        ));
        assert!(free_camera_entity_view_contains_position(
            camera,
            forward,
            scan,
            [20.5, 0.0, 16.5]
        ));
        assert!(!free_camera_entity_view_contains_position(
            camera,
            forward,
            scan,
            [20.5, 0.0, 16.49]
        ));
        assert!(free_camera_entity_view_contains_position(
            camera,
            forward,
            scan,
            [22.5, 0.0, 17.5]
        ));
        assert!(!free_camera_entity_view_contains_position(
            camera,
            forward,
            scan,
            [22.51, 0.0, 17.5]
        ));
    }

    #[test]
    fn vertical_camera_direction_uses_retail_rearward_lead_and_renderer_fallback() {
        let mut terrain = flat_terrain();
        cell_mut(&mut terrain, 3, 4).attribute = 2;
        let table = object_table(2, [55; 4], 4);

        let instances = collect_static_terrain_objects(
            &terrain,
            &table,
            [3.0, 10.0, 3.0],
            [0.0, 1.0, 0.0],
            (8, 8),
        );

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].cell, [3, 4]);
    }
}
