//! Exact dependencies of the existing camera-facing terrain cell selection.
//! This preserves the forced-rebuild geometry; it does not replace the scan
//! with a different retail algorithm or wrap its world-space lattice.

const TERRAIN_COVERAGE_MARGIN: f32 = 2.0;

#[derive(Debug, Clone, Copy)]
pub(crate) struct TerrainScan {
    pub(crate) camera_xz: [f32; 2],
    /// Normalized horizontal camera direction.
    pub(crate) forward: [f32; 2],
    pub(crate) row_lead_raw: i32,
    pub(crate) fog_far: Option<f32>,
    pub(crate) horizontal_half_fov_tan: f32,
}

impl TerrainScan {
    pub(crate) fn footprint(self, dimensions: [u32; 2]) -> TerrainFootprint {
        let [columns, authored_rows] = dimensions;
        let near = self.row_lead_raw as f32 / 256.0;
        let rows = fog_covered_scan_rows(authored_rows, self.fog_far, near);
        TerrainFootprint {
            origin: self.camera_xz,
            forward: self.forward,
            near,
            far: near + rows as f32,
            half_width: fog_covered_half_width(columns, self.fog_far, self.horizontal_half_fov_tan),
        }
    }
}

/// Cache identity and selection use the same resolved inputs. Fractional
/// motion and small yaw changes can change boundary cells even when their
/// previous integer-cell or yaw-bucket keys were equal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TerrainFootprint {
    origin: [f32; 2],
    forward: [f32; 2],
    near: f32,
    far: f32,
    half_width: f32,
}

impl TerrainFootprint {
    pub(crate) fn cells(self) -> impl Iterator<Item = [i32; 2]> {
        let max_along = self.near.abs().max(self.far.abs());
        let radius = (self.half_width * self.half_width + max_along * max_along)
            .sqrt()
            .ceil() as i32
            + 1;
        let center_x = self.origin[0].floor() as i32;
        let center_z = self.origin[1].floor() as i32;
        let right = [-self.forward[1], self.forward[0]];
        // Preserve the old Z-outer, X-inner submission order and strict side
        // boundary; geometry remains in the unwrapped world lattice.
        (center_z - radius..=center_z + radius).flat_map(move |world_z| {
            (center_x - radius..=center_x + radius).filter_map(move |world_x| {
                let dx = world_x as f32 + 0.5 - self.origin[0];
                let dz = world_z as f32 + 0.5 - self.origin[1];
                let along = dx * self.forward[0] + dz * self.forward[1];
                let across = dx * right[0] + dz * right[1];
                if !(self.near..self.far).contains(&along) || across.abs() >= self.half_width {
                    None
                } else {
                    Some([world_x, world_z])
                }
            })
        })
    }
}

fn fog_covered_scan_rows(scan_rows: u32, fog_far: Option<f32>, row_lead: f32) -> u32 {
    let Some(fog_far) = fog_far else {
        return scan_rows;
    };
    scan_rows.max(
        (fog_far - row_lead + TERRAIN_COVERAGE_MARGIN)
            .ceil()
            .max(0.0) as u32,
    )
}

fn fog_covered_half_width(
    scan_columns: u32,
    fog_far: Option<f32>,
    horizontal_half_fov_tan: f32,
) -> f32 {
    let authored = scan_columns as f32 * 0.5;
    let Some(fog_far) = fog_far else {
        return authored;
    };
    authored.max(fog_far * horizontal_half_fov_tan.max(0.0) + TERRAIN_COVERAGE_MARGIN)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan() -> TerrainScan {
        TerrainScan {
            camera_xz: [0.25, 0.25],
            forward: [0.0, -1.0],
            row_lead_raw: 0,
            fog_far: None,
            horizontal_half_fov_tan: 1.0,
        }
    }

    fn cells(footprint: TerrainFootprint) -> Vec<[i32; 2]> {
        footprint.cells().collect()
    }

    #[test]
    fn fog_coverage_reaches_the_retail_terminal_plane() {
        assert_eq!(fog_covered_scan_rows(21, Some(21.0), 2.0), 21);
        assert_eq!(2.0 + 21.0, 23.0);
        assert_eq!(fog_covered_scan_rows(21, None, 2.0), 21);
        assert_eq!(fog_covered_scan_rows(30, Some(21.0), 2.0), 30);
        assert_eq!(fog_covered_scan_rows(21, Some(21.0), -6.75), 30);
    }

    #[test]
    fn fog_coverage_expands_the_retail_4_3_scan_for_widescreen() {
        assert_eq!(fog_covered_half_width(32, None, 1.0), 16.0);
        assert_eq!(fog_covered_half_width(32, Some(21.0), 1.0), 23.0);
        assert_eq!(fog_covered_half_width(52, Some(21.0), 1.0), 26.0);
    }

    #[test]
    fn fractional_translation_inside_one_cell_changes_boundary_selection() {
        let a = TerrainScan {
            camera_xz: [0.49; 2],
            ..scan()
        }
        .footprint([2, 2]);
        let b = TerrainScan {
            camera_xz: [0.51; 2],
            ..scan()
        }
        .footprint([2, 2]);
        assert_eq!(a.origin.map(f32::floor), b.origin.map(f32::floor));
        assert_ne!(a, b);
        assert_ne!(cells(a), cells(b));
    }

    #[test]
    fn tiny_yaw_inside_one_previous_bucket_changes_boundary_selection() {
        let yaw = 0.005_f32;
        let a = TerrainScan {
            camera_xz: [0.5; 2],
            ..scan()
        }
        .footprint([4, 4]);
        let b = TerrainScan {
            camera_xz: [0.5; 2],
            forward: [yaw.sin(), -yaw.cos()],
            ..scan()
        }
        .footprint([4, 4]);
        let old_bucket = |f: TerrainFootprint| {
            (f.forward[0].atan2(-f.forward[1]) * 128.0 / std::f32::consts::TAU).round() as i16
        };
        assert_eq!(old_bucket(a), old_bucket(b));
        assert_ne!(a, b);
        assert_ne!(cells(a), cells(b));
    }

    #[test]
    fn dimensions_and_fog_width_invalidate_even_when_row_count_is_unchanged() {
        let narrow = scan().footprint([2, 30]);
        let wide = scan().footprint([4, 30]);
        assert_eq!([narrow.near, narrow.far], [wide.near, wide.far]);
        assert_ne!(narrow, wide);
        assert_ne!(cells(narrow), cells(wide));

        let near_fog = TerrainScan {
            fog_far: Some(10.0),
            ..scan()
        }
        .footprint([4, 30]);
        let far_fog = TerrainScan {
            fog_far: Some(10.75),
            ..scan()
        }
        .footprint([4, 30]);
        assert_eq!([near_fog.near, near_fog.far], [far_fog.near, far_fog.far]);
        assert_ne!(near_fog, far_fog);
        assert_ne!(cells(near_fog), cells(far_fog));
    }

    #[test]
    fn pitch_row_lead_changes_the_exact_near_and_far_boundaries() {
        let a = TerrainScan {
            camera_xz: [0.5; 2],
            ..scan()
        }
        .footprint([4, 30]);
        let b = TerrainScan {
            camera_xz: [0.5; 2],
            row_lead_raw: 64,
            ..scan()
        }
        .footprint([4, 30]);
        assert_eq!(b.near - a.near, 0.25);
        assert_eq!(b.far - a.far, 0.25);
        assert_ne!(a, b);
        assert_ne!(cells(a), cells(b));
    }

    #[test]
    fn world_translation_across_the_terrain_seam_does_not_wrap_geometry() {
        let a = TerrainScan {
            camera_xz: [127.75, -0.25],
            ..scan()
        }
        .footprint([4, 8]);
        let b = TerrainScan {
            camera_xz: [255.75, -128.25],
            ..scan()
        }
        .footprint([4, 8]);
        assert_ne!(a, b);
        let a_cells = cells(a);
        assert!(a_cells.iter().any(|&[x, z]| x >= 128 && z < 0));
        assert_eq!(
            a_cells
                .iter()
                .map(|&[x, z]| [x + 128, z - 128])
                .collect::<Vec<_>>(),
            cells(b)
        );
    }

    #[test]
    fn unchanged_effective_footprint_reuses_geometry() {
        let a = scan().footprint([32, 21]);
        assert_eq!(a, scan().footprint([32, 21]));
        // With fog disabled, FOV does not expand the authored selection.
        let b = TerrainScan {
            horizontal_half_fov_tan: 3.0,
            ..scan()
        }
        .footprint([32, 21]);
        assert_eq!(a, b);
        assert_eq!(cells(a), cells(b));
        // Different authored dimensions can resolve to the same fog coverage.
        let expanded = TerrainScan {
            fog_far: Some(40.0),
            ..scan()
        };
        assert_eq!(expanded.footprint([32, 21]), expanded.footprint([52, 30]));
    }

    #[test]
    fn selection_matches_the_existing_predicate_over_an_independent_search_box() {
        // The fixed reference box is deliberately larger than every sampled
        // footprint; it does not reuse the production search-radius formula.
        for origin in [[0.49, 0.49], [0.51, 0.51], [127.75, -0.25]] {
            for yaw in [0.0_f32, 0.005, 0.7] {
                for row_lead_raw in [-1728, 0, 511] {
                    for dimensions in [[4, 8], [32, 21], [52, 30]] {
                        for fog_far in [None, Some(21.0), Some(21.75)] {
                            let input = TerrainScan {
                                camera_xz: origin,
                                forward: [yaw.sin(), -yaw.cos()],
                                row_lead_raw,
                                fog_far,
                                horizontal_half_fov_tan: 1.2,
                            };
                            let near = row_lead_raw as f32 / 256.0;
                            let mut rows = dimensions[1];
                            let mut half_width = dimensions[0] as f32 * 0.5;
                            if let Some(far) = fog_far {
                                rows = rows.max((far - near + 2.0).ceil().max(0.0) as u32);
                                half_width = half_width.max(far * 1.2 + 2.0);
                            }
                            let far = near + rows as f32;
                            let [cx, cz] = origin.map(|coordinate| coordinate.floor() as i32);
                            let mut expected = Vec::new();
                            for z in cz - 96..=cz + 96 {
                                for x in cx - 96..=cx + 96 {
                                    let dx = x as f32 + 0.5 - origin[0];
                                    let dz = z as f32 + 0.5 - origin[1];
                                    let along = dx * input.forward[0] + dz * input.forward[1];
                                    let across = dx * -input.forward[1] + dz * input.forward[0];
                                    if (near..far).contains(&along) && across.abs() < half_width {
                                        expected.push([x, z]);
                                    }
                                }
                            }
                            assert_eq!(
                                cells(input.footprint(dimensions)),
                                expected,
                                "{input:?} {dimensions:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}
