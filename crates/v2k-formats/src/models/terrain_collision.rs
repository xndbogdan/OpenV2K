//! Retail `415160` sphere/heightfield callback. Its collision surface is one
//! averaged Q12 plane per cell, followed by the four perimeter edges. It is
//! independent of the renderer's two triangles and the bilinear ride surface.

use super::{collision_integer_sqrt, ModelCollisionHit};
use crate::terrain::{TerrainGrid, GRID_SIZE};

pub(super) fn terrain_sphere_hit(
    terrain: &TerrainGrid,
    center: [f64; 3],
    radius: f64,
) -> Option<ModelCollisionHit> {
    if radius < 0.0 || !radius.is_finite() || center.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let center = center.map(|v| v.trunc() as i32);
    let radius = radius as i32;
    let (min_x, max_x) = cell_range(center[0], radius);
    let (min_z, max_z) = cell_range(center[2], radius);
    let mut best: Option<([i16; 3], i32)> = None;
    // 415160 visits Z outside X; equal penetrations retain the first cell.
    for z in min_z..=max_z {
        for x in min_x..=max_x {
            let heights = [
                height(terrain, x, z)?,
                height(terrain, x, z + 1)?,
                height(terrain, x + 1, z)?,
                height(terrain, x + 1, z + 1)?,
            ];
            let Some(hit) = cell_hit(center, radius, [x * 256, z * 256], heights) else {
                continue;
            };
            if best.is_none_or(|old| hit.1 > old.1) {
                best = Some(hit);
            }
        }
    }
    best.map(|(normal, penetration)| ModelCollisionHit {
        normal: normal.map(|v| f64::from(v) / 4096.0),
        penetration_raw: f64::from(penetration),
    })
}

fn cell_range(center: i32, radius: i32) -> (i32, i32) {
    let min = (i64::from(center) - i64::from(radius)) >> 8;
    let max = (i64::from(center) + i64::from(radius)) >> 8;
    if max - min < GRID_SIZE as i64 {
        (min as i32, max as i32)
    } else {
        // Preserve the existing bounded query for oversized non-retail probes.
        let cell = center >> 8;
        (cell - 128, cell + 127)
    }
}

fn height(terrain: &TerrainGrid, x: i32, z: i32) -> Option<i32> {
    terrain
        .cell(x.rem_euclid(256) as usize, z.rem_euclid(256) as usize)
        .map(|c| i32::from(c.height as i8) * 32)
}

fn cell_hit(
    center: [i32; 3],
    radius: i32,
    [x, z]: [i32; 2],
    [h00, h01, h10, h11]: [i32; 4],
) -> Option<([i16; 3], i32)> {
    if h00.max(h01).max(h10).max(h11) <= center[1] - radius {
        return None;
    }
    // 415770 normalizes each cross separately, then 415610 normalizes their
    // sum. Averaging raw cross products would weight the two slopes differently.
    let first = normalize_scaled([-256 * (h10 - h00), 65536, -256 * (h01 - h00)]);
    let second = normalize_scaled([256 * (h01 - h11), 65536, 256 * (h10 - h11)]);
    let normal = normalize(std::array::from_fn(|i| {
        i32::from(first[i]) + i32::from(second[i])
    }));
    let distance = dot(normal, [center[0] - x, center[1] - h00, center[2] - z]);
    if distance >= radius {
        return None;
    }
    // 4158C0 stores the projected displacement in signed words. 415416/430
    // compare the wrapped cell bytes, not a finite triangle's barycentrics.
    let displacement = normal.map(|n| ((i32::from(n).wrapping_mul(distance)) >> 12) as i16);
    let projected_x = center[0].wrapping_sub(i32::from(displacement[0]));
    let projected_z = center[2].wrapping_sub(i32::from(displacement[2]));
    if ((projected_x ^ x) & 0xff00) == 0 && ((projected_z ^ z) & 0xff00) == 0 {
        return Some((normal, radius - distance));
    }

    // 415900: four perimeter segments, clockwise from (x,z). There is no
    // diagonal edge. Strictly closer candidates replace the retained point.
    let corners = [
        [x, h00, z],
        [x + 256, h10, z],
        [x + 256, h11, z + 256],
        [x, h01, z + 256],
    ];
    let mut closest = corners[0];
    let mut distance_squared = u32::MAX;
    for i in 0..4 {
        let point = closest_on_edge(center, corners[i], corners[(i + 1) % 4]);
        let delta = std::array::from_fn(|axis| center[axis] - point[axis]);
        let squared = squared_length(delta);
        if squared < distance_squared {
            closest = point;
            distance_squared = squared;
        }
    }
    // Edge selection uses unsigned distances, but 4154CF's radius admission
    // is a signed comparison. Keep the two policies distinct on overflow.
    if (distance_squared as i32) >= radius.wrapping_mul(radius) {
        return None;
    }
    let delta = std::array::from_fn(|i| center[i] - closest[i]);
    let normal = if center[1] > closest[1] {
        normalize_scaled(delta)
    } else if center[0] == closest[0] && center[2] == closest[0] {
        [4096, 0, 0]
    } else {
        // Retail deliberately suppresses downward edge normals; its special
        // equality branch also compares center Z with closest X (41551F).
        normalize_scaled([delta[0], 0, delta[2]])
    };
    // 457730 returns zero for a nonpositive signed argument before its
    // otherwise unsigned square-root loop (45773D/457752).
    let distance = if (distance_squared as i32) > 0 {
        i32::from(collision_integer_sqrt(distance_squared) as i16)
    } else {
        0
    };
    Some((normal, radius - distance))
}

fn closest_on_edge(center: [i32; 3], a: [i32; 3], b: [i32; 3]) -> [i32; 3] {
    let edge = std::array::from_fn(|i| b[i] - a[i]);
    let length = i32::from(collision_integer_sqrt(squared_length(edge)) as i16);
    // 46ABA0 receives the already-computed length for these 256-wide edges.
    let direction = edge.map(|v| (v.wrapping_shl(12) / length) as i16);
    let along = dot(direction, std::array::from_fn(|i| center[i] - a[i]));
    if along < 0 {
        a
    } else if along > length {
        b
    } else {
        std::array::from_fn(|i| a[i] + i32::from(((i32::from(direction[i]) * along) >> 12) as i16))
    }
}

fn dot(normal: [i16; 3], vector: [i32; 3]) -> i32 {
    (0..3).fold(0i32, |sum, i| {
        sum.wrapping_add(i32::from(normal[i]).wrapping_mul(vector[i]))
    }) >> 12
}

fn squared_length(vector: [i32; 3]) -> u32 {
    vector
        .iter()
        .fold(0u32, |sum, v| sum.wrapping_add(v.wrapping_mul(*v) as u32))
}

fn normalize_scaled(vector: [i32; 3]) -> [i16; 3] {
    let bits = vector.iter().fold(0u32, |bits, v| bits | v.unsigned_abs());
    let shift = match 31 - bits.max(1).leading_zeros() {
        0..=13 => 0,
        14..=18 => 5,
        19..=23 => 10,
        24..=28 => 15,
        _ => 20,
    };
    normalize(vector.map(|v| v >> shift))
}

fn normalize(vector: [i32; 3]) -> [i16; 3] {
    let length = i32::from(collision_integer_sqrt(squared_length(vector)) as i16);
    if length == 0 {
        [4096, 0, 0]
    } else {
        vector.map(|v| (v.wrapping_shl(12) / length) as i16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::TerrainCell;

    #[test]
    fn perimeter_below_center_preserves_retail_cross_axis_equality() {
        // Actual 415160/415900 execution on Level50 at [8192,-2007,8192],
        // radius458 retains this cell. 41551F compares center Z to closest X,
        // selecting +X; an ordinary horizontal normalization would select +Z.
        assert_eq!(
            cell_hit(
                [8192, -2007, 8192],
                458,
                [7936, 7936],
                [-1888, -1856, -1888, -1856]
            ),
            Some(([4096, 0, 0], 310)),
        );
    }

    #[test]
    fn warped_cell_and_perimeter_match_machine_code_oracles() {
        let mut terrain = TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0
                };
                GRID_SIZE * GRID_SIZE
            ],
        };
        terrain.cells[GRID_SIZE + 1].height = 8;
        // Synthetic inputs evaluated by unmodified retail415160 in an x86
        // emulator. Include the flat rendered half, deep burial, and a sphere
        // beyond the raised corner; these are not triangle-derived answers.
        for (center, radius, normal, depth) in [
            ([64, 3, 64], 2, [-1331, 3638, -1331], 41),
            ([192, 129, 192], 2, [-1331, 3638, -1331], 13),
            ([128, -100, 128], 20, [-1331, 3638, -1331], 193),
            ([300, 100, 300], 90, [1331, 3638, 1331], 200),
        ] {
            let hit =
                terrain_sphere_hit(&terrain, center.map(f64::from), f64::from(radius)).unwrap();
            assert_eq!(hit.normal, normal.map(|n| f64::from(n) / 4096.0));
            assert_eq!(hit.penetration_raw, f64::from(depth));
        }
        terrain.cells[GRID_SIZE + 1].height = 0;
        terrain.cells[1].height = 8;
        terrain.cells[GRID_SIZE].height = 8;
        assert!(terrain_sphere_hit(&terrain, [128.0, 20.0, 128.0], 20.0).is_none());
    }

    #[test]
    fn equal_depth_keeps_z_outer_cell_and_first_edge() {
        let terrain = TerrainGrid {
            header: [0; 5],
            cells: (0..GRID_SIZE * GRID_SIZE)
                .map(|i| TerrainCell {
                    height: if (i / GRID_SIZE + i % GRID_SIZE) & 1 == 0 {
                        (-31i8) as u8
                    } else {
                        31
                    },
                    attribute: 0,
                    terrain_type: 0,
                })
                .collect(),
        };
        // Unmodified 415160 oracles: swapping the cell-loop nesting changes
        // the first normal to [0,2730,-4096]; accepting equal-depth replacements
        // changes the second to [0,4096,0]. Both are observable corrections.
        for (center, radius, normal, depth) in [
            ([0, -961, 0], 31, [-4096, 2730, 0], 28),
            ([0, -992, 0], 1, [4096, 0, 0], 1),
        ] {
            let hit =
                terrain_sphere_hit(&terrain, center.map(f64::from), f64::from(radius)).unwrap();
            assert_eq!(hit.normal, normal.map(|n| f64::from(n) / 4096.0));
            assert_eq!(hit.penetration_raw, f64::from(depth));
        }
    }

    #[test]
    fn edge_squared_distance_keeps_distinct_unsigned_selection_and_signed_admission() {
        // A model-local offset can take the materialized sphere outside the
        // entity's signed-word range. These original-instruction oracles pin
        // 415900's unsigned closest selection, 4154CF's signed radius test,
        // and 457730's zero result for a nonpositive signed squared distance.
        let heights = [-1024, -960, -992, -928];
        assert_eq!(
            cell_hit([123, -50000, 45], 31, [0, 0], heights),
            Some(([3875, 0, 1417], 31)),
        );
        assert_eq!(cell_hit([123, -47000, 45], 31, [0, 0], heights), None);
    }
}
