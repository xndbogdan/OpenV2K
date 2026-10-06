use super::*;
use v2k_formats::{
    anim_frames::{ModelSlotPattern, TerrainObjectDescriptor},
    terrain::TerrainCell,
};

fn terrain(height: i8, terrain_type: u8) -> TerrainGrid {
    TerrainGrid {
        header: [0; 5],
        cells: vec![
            TerrainCell {
                height: height as u8,
                attribute: 0,
                terrain_type
            };
            GRID_SIZE * GRID_SIZE
        ],
    }
}
fn objects() -> TerrainObjectTable {
    TerrainObjectTable {
        records: (0..256)
            .map(|_| TerrainObjectDescriptor {
                model_ids: [0; 4],
                kind_index: 0,
                pattern: ModelSlotPattern::Static,
            })
            .collect(),
    }
}
fn request(x: i16, z: i16) -> TerrainCraterRequest {
    TerrainCraterRequest {
        position_raw: [x, 123, z],
        radius_raw: 1280,
        depth_height_units: 16,
        replacement_material: Some(4),
    }
}
fn height_at(terrain: &TerrainGrid, x: usize, z: usize) -> i8 {
    terrain.cell(x, z).unwrap().height as i8
}

#[test]
fn crater_36e00_uses_signed_integer_profile_strict_radii_and_material_light_tail() {
    let mut grid = terrain(-8, 0xf7);
    let mut scheduler = StaticDamageScheduler::default();
    let report =
        apply_terrain_crater(&mut grid, &objects(), &mut scheduler, request(0, 0)).unwrap();
    // radius^2>>16=25, inner(1280*4/6)^2>>16=11. Source
    // centre=16+4; at1cell=14+4; at3cells=2+4; at4cells=-7+4.
    assert_eq!(height_at(&grid, 0, 0), -28);
    assert_eq!(height_at(&grid, 1, 0), -26);
    assert_eq!(height_at(&grid, 3, 0), -14);
    assert_eq!(height_at(&grid, 4, 0), -6);
    assert_eq!(height_at(&grid, 5, 0), -8, "strict outer boundary");
    assert_eq!(grid.cell(0, 0).unwrap().terrain_type, 4);
    assert_eq!(grid.cell(1, 0).unwrap().terrain_type, 4);
    assert_eq!(grid.cell(3, 0).unwrap().terrain_type, 0x44);
    assert_eq!(grid.cell(4, 0).unwrap().terrain_type, 0x84);
    assert_eq!(grid.cell(5, 0).unwrap().terrain_type, 0xf7);
    assert!(report.terrain_changed);
    assert_eq!(report.visited_cells, 69);
    assert_eq!(report.radar_refresh_positions_raw.len(), 69);
    assert_eq!(report.radar_refresh_positions_raw[0], [-1024, -512]);
    assert_eq!(
        report.radar_refresh_positions_raw.last(),
        Some(&[1024, 512])
    );
}

#[test]
fn hut_crater_matches_retail_ttd_height_neighborhood_and_water_extent() {
    // V200001 FB636:1E7 -> FB641:13FB, tick342. The independent retail
    // before/after oracle covers x139..149, z123..133, including slopes and
    // the raised rim. Full triples and command line: 20260919-intro2-hut-
    // crater-ttd.{txt,ps1,windbg.cmd} in the accepted local capture ledger.
    let before: [[i8; 11]; 11] = [
        [-8, -8, -8, -8, -8, -8, -7, -6, -6, 1, 10],
        [-10, -9, -9, -9, -9, -8, -7, -6, -5, 1, 6],
        [-11, -10, -10, -10, -9, -8, -7, -6, -5, -1, 3],
        [-12, -11, -11, -10, -9, -8, -7, -6, -6, -5, -2],
        [-12, -11, -11, -10, -9, -8, -7, -6, -6, -4, 1],
        [-11, -10, -10, -10, -9, -8, -7, -6, -5, 5, 8],
        [-9, -9, -9, -9, -8, -8, -7, -6, -5, 2, 9],
        [-8, -8, -8, -8, -8, -7, -7, -6, -3, 4, 8],
        [-8, -8, -8, -8, -8, -7, -6, -5, -3, 2, 6],
        [-7, -6, -6, -6, -6, -6, -5, -4, -3, 1, 5],
        [-7, -7, -6, -6, -6, -5, -5, -4, -2, 0, 4],
    ];
    let after: [[i8; 11]; 11] = [
        [-8, -8, -8, -8, -8, -8, -7, -6, -6, 1, 10],
        [-10, -9, -9, -5, -6, -6, -4, -2, -5, 1, 6],
        [-11, -10, -7, -10, -13, -14, -12, -8, -2, -1, 3],
        [-12, -7, -11, -16, -20, -22, -19, -14, -8, -1, -2],
        [-12, -8, -13, -20, -25, -26, -24, -18, -11, -1, 1],
        [-11, -8, -14, -22, -26, -28, -25, -20, -11, 7, 8],
        [-9, -6, -13, -20, -25, -26, -24, -18, -10, 5, 9],
        [-8, -4, -10, -16, -20, -21, -19, -14, -5, 8, 8],
        [-8, -8, -5, -10, -13, -13, -11, -7, 0, 2, 6],
        [-7, -6, -6, -2, -3, -4, -2, 0, -3, 1, 5],
        [-7, -7, -6, -6, -6, -5, -5, -4, -2, 0, 4],
    ];
    let mut grid = terrain(0, 0);
    grid.header[0] = -216_832;
    for (dx, row) in before.iter().enumerate() {
        for (dz, &height) in row.iter().enumerate() {
            grid.cells[(139 + dx) * GRID_SIZE + 123 + dz].height = height as u8;
        }
    }
    let report = apply_terrain_crater(
        &mut grid,
        &objects(),
        &mut StaticDamageScheduler::default(),
        request(-28672, -32768),
    )
    .unwrap();
    assert_eq!(report.visited_cells, 69);
    let mut submerged = Vec::new();
    for (dx, row) in after.iter().enumerate() {
        for (dz, &expected) in row.iter().enumerate() {
            let cell = [139 + dx, 123 + dz];
            let actual = height_at(&grid, cell[0], cell[1]);
            assert_eq!(actual, expected, "retail cell {cell:?}");
            if i16::from(actual) * 32 < grid.sea_level_raw() {
                submerged.push(cell);
            }
        }
    }
    assert_eq!(grid.sea_level_raw(), -847, "crater does not move sea level");
    assert_eq!(
        submerged,
        [[144, 128]],
        "retail admits one submerged vertex"
    );
}

#[test]
fn crater_signed_clamp_and_lower_valley_branch_preserve_deeper_terrain() {
    let mut grid = terrain(-8, 0);
    grid.cells[GRID_SIZE].height = (-100_i8) as u8;
    grid.cells[2 * GRID_SIZE].height = (-10_i8) as u8;
    let mut scheduler = StaticDamageScheduler::default();
    apply_terrain_crater(&mut grid, &objects(), &mut scheduler, request(0, 0)).unwrap();
    assert_eq!(height_at(&grid, 1, 0), -100);
    assert_eq!(height_at(&grid, 2, 0), -22);
    let mut grid = terrain(-126, 0);
    apply_terrain_crater(&mut grid, &objects(), &mut scheduler, request(0, 0)).unwrap();
    assert_eq!(
        height_at(&grid, 0, 0),
        -127,
        "retail floor is -127, never -128"
    );
    let mut grid = terrain(126, 0);
    let mut upward = request(0, 0);
    upward.depth_height_units = -16;
    apply_terrain_crater(&mut grid, &objects(), &mut scheduler, upward).unwrap();
    assert_eq!(height_at(&grid, 0, 0), 127);
}

#[test]
fn crater_wraps_raw_point_keeps_fractional_phase_and_world_style_no_recolor() {
    assert_eq!(crater_material_for_world_style(4), Some(1));
    assert_eq!(crater_material_for_world_style(5), None);
    assert_eq!(crater_material_for_world_style(6), Some(4));
    let mut grid = terrain(0, 0xf7);
    let mut req = request(-64, 64);
    req.replacement_material = None;
    let report = apply_terrain_crater(
        &mut grid,
        &objects(),
        &mut StaticDamageScheduler::default(),
        req,
    )
    .unwrap();
    assert_eq!(height_at(&grid, 255, 0), -20);
    assert_eq!(height_at(&grid, 0, 0), -18);
    assert_eq!(
        grid.cell(255, 0).unwrap().terrain_type,
        0xe7,
        "clear infection even with no material override"
    );
    assert!(report
        .radar_refresh_positions_raw
        .iter()
        .all(|p| p[0] as u16 & 255 == 192 && p[1] as u16 & 255 == 64));
}

#[test]
fn crater_direct_static_entry_bypasses_burn_and_chance_but_deduplicates_fifo() {
    let mut grid = terrain(0, 0x08);
    grid.cells[0].attribute = 1;
    let mut scheduler = StaticDamageScheduler::default();
    let report =
        apply_terrain_crater(&mut grid, &objects(), &mut scheduler, request(0, 0)).unwrap();
    assert_eq!(
        report.started_static_cells,
        vec![[0, 0]],
        "27B20 accepts an already burned cell with no damage packet"
    );
    assert_eq!(scheduler.active_program_count(), 1);
    let report =
        apply_terrain_crater(&mut grid, &objects(), &mut scheduler, request(0, 0)).unwrap();
    assert!(report.started_static_cells.is_empty());
    assert_eq!(scheduler.active_program_count(), 1);
    let mut table = objects();
    table.records[1].kind_index = (0..32)
        .find(|kind| {
            crate::static_kind_catalog::static_kind_descriptor(*kind)
                .unwrap()
                .timed_program_va()
                == 0
        })
        .unwrap();
    let mut grid = terrain(0, 0x10);
    grid.cells[0].attribute = 1;
    let report = apply_terrain_crater(
        &mut grid,
        &table,
        &mut StaticDamageScheduler::default(),
        request(0, 0),
    )
    .unwrap();
    assert!(report.started_static_cells.is_empty());
    assert_eq!(
        grid.cell(0, 0).unwrap().terrain_type,
        0x0c,
        "null-program immediate burn survives subsequent material write"
    );
}

#[test]
fn crater_late_unknown_static_preserves_exact_height_prefix_without_fake_radar_completion() {
    let mut grid = terrain(0, 0);
    grid.cells[0].attribute = 1;
    let mut table = objects();
    table.records[1].kind_index = u32::MAX;
    let mut scheduler = StaticDamageScheduler::default();
    let failure =
        apply_terrain_crater(&mut grid, &table, &mut scheduler, request(0, 0)).unwrap_err();
    assert_eq!(
        failure.block,
        TerrainCraterBlock::UnsupportedStaticKind(u32::MAX)
    );
    assert!(failure.report.terrain_changed);
    assert_eq!(height_at(&grid, 0, 0), -20);
    assert_eq!(height_at(&grid, 1, 0), 0, "later rows are not committed");
    assert!(failure.report.radar_refresh_positions_raw.is_empty());
    let mut invalid = request(0, 0);
    invalid.radius_raw = i32::MAX;
    assert_eq!(
        apply_terrain_crater(&mut grid, &table, &mut scheduler, invalid)
            .unwrap_err()
            .block,
        TerrainCraterBlock::InvalidRequest
    );
}

#[test]
fn crater_16ac0_uses_forward_slope_not_heading_and_retains_integer_normalization() {
    let mut grid = terrain(0, 0);
    let flat = terrain_aligned_actor_basis(&grid, [0; 3]);
    // Q12 one maps toMAX, then57960 shifts it to16383 and adds1
    // after the square root. This differs from an Euler identity column.
    assert_eq!(flat.lateral, [2147352576, 0, 0]);
    assert_eq!(flat.up, [0, 2147352576, 0]);
    assert_eq!(flat.forward, [0, 0, 2147352576]);
    grid.cells[GRID_SIZE].height = 8; // +256 raw Y over one +X cell
    grid.cells[1].height = (-8_i8) as u8; // -256 raw Y over one +Z cell
    let basis = terrain_aligned_actor_basis(&grid, [0; 3]);
    assert_eq!(basis.lateral[0], basis.lateral[1]);
    assert_eq!(basis.lateral[2], 0);
    assert_eq!(basis.forward[0], 0);
    assert_eq!(basis.forward[1], -basis.forward[2]);
    assert!(basis.up[0] < 0 && basis.up[1] > 0 && basis.up[2] > 0);
    assert_ne!(basis, flat);
}
