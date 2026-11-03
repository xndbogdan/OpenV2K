use super::*;
use crate::network::framing::Record;
use crate::network::protocol::GameRecord;
use v2k_formats::terrain::TerrainCell;

fn grid() -> TerrainGrid {
    TerrainGrid {
        header: [0x0012_3456, 0, 0, 0, 0],
        cells: vec![
            TerrainCell {
                height: 0,
                attribute: 0,
                terrain_type: 0
            };
            GRID_SIZE * GRID_SIZE
        ],
    }
}

fn set(terrain: &mut TerrainGrid, x: usize, z: usize, flags: u8, attribute: u8) {
    let cell = &mut terrain.cells[x * GRID_SIZE + z];
    cell.terrain_type = flags;
    cell.attribute = attribute;
}

#[test]
fn a_run_ends_at_an_infection_change_and_leaves_the_cursor_there() {
    let mut terrain = grid();
    for x in 0..5 {
        set(&mut terrain, x, 0, 0x10, 0);
    }
    let mut cursor = [0, 0];
    assert_eq!(measure_run(&terrain, &mut cursor), (5, 1));
    assert_eq!(cursor, [5, 0]);
}

#[test]
fn a_run_holds_255_cells_and_wraps_x_into_z() {
    let terrain = grid();
    let mut cursor = [200, 255];
    assert_eq!(measure_run(&terrain, &mut cursor), (255, 0));
    // 255 cells from x 200: x 200..=255 on row 255, then row 0 wraps.
    assert_eq!(cursor, [199, 0]);
}

#[test]
fn burned_cells_compare_the_attribute_the_retail_way() {
    // A burned first cell with no attribute byte (flags 2 | 4): a later
    // burned cell joins only when its attribute byte is *set*.
    let mut terrain = grid();
    set(&mut terrain, 0, 0, 0x08, 0);
    set(&mut terrain, 1, 0, 0x08, 7);
    set(&mut terrain, 2, 0, 0x08, 0);
    let mut cursor = [0, 0];
    assert_eq!(measure_run(&terrain, &mut cursor), (2, 6));
    assert_eq!(cursor, [2, 0]);
    // A burned first cell with an attribute byte (flags 2): a later burned
    // cell joins only when its byte is clear, an unburned object never.
    let mut terrain = grid();
    set(&mut terrain, 0, 0, 0x08, 3);
    set(&mut terrain, 1, 0, 0x08, 0);
    set(&mut terrain, 2, 0, 0x08, 4);
    let mut cursor = [0, 0];
    assert_eq!(measure_run(&terrain, &mut cursor), (2, 2));
    let mut terrain = grid();
    set(&mut terrain, 1, 0, 0, 9);
    let mut cursor = [0, 0];
    assert_eq!(measure_run(&terrain, &mut cursor), (1, 0));
}

#[test]
fn kind_3_packs_to_the_record_limit_and_advances_the_cursor() {
    let mut terrain = grid();
    // Alternate infection so every run is one cell long.
    for x in (0..256).step_by(2) {
        set(&mut terrain, x, 0, 0x10, 0);
    }
    let record = TerrainRuns {
        terrain: &terrain,
        cursor: Cell::new([0, 0]),
    };
    let mut room = [0u8; 200];
    assert_eq!(record.pack(&mut room), Some(0x60));
    assert_eq!(&room[..8], &[0x60, 0, 0x34, 0x12, 0, 0, 0, 0]);
    assert_eq!(&room[8..12], &[1, 1, 1, 0]);
    // 44 runs of one cell.
    assert_eq!(record.cursor.get(), [44, 0]);
    let (sea, start, runs) = parse_runs(&room[..0x60]).unwrap();
    assert_eq!((sea, start, runs.len()), (0x1234, [0, 0], 44));

    // A small room stops early; 4,096 cells stop it too.
    let record = TerrainRuns {
        terrain: &grid(),
        cursor: Cell::new([0, 0]),
    };
    let mut room = [0u8; 13];
    assert_eq!(record.pack(&mut room), Some(12));
    let mut room = [0u8; 200];
    let size = record.pack(&mut room).unwrap();
    assert_eq!(size, 8 + 2 * 17);
    assert_eq!(record.cursor.get(), [(255 * 19) % 256, (255 * 19) / 256]);
}

#[test]
fn notifications_become_the_hosts_cells_and_the_clients_requests() {
    let change = |list, value, attribute| NetworkCellChange {
        list,
        cell: [3, 4],
        value,
        flags: 0x18,
        attribute,
    };
    assert_eq!(
        cell_change_record(change(0, 1, 9), true),
        GameRecord::Cell([3, 4, 0x18, 9])
    );
    assert_eq!(
        cell_change_record(change(0, 1, 9), false),
        GameRecord::CellRequest([3, 4, 0, 0])
    );
    assert_eq!(
        cell_change_record(change(0, 0, 9), false),
        GameRecord::CellRequest([3, 4, 1, 0])
    );
    assert_eq!(
        cell_change_record(change(1, 1, 9), false),
        GameRecord::CellRequest([3, 4, 2, 0])
    );
    assert_eq!(
        cell_change_record(change(1, 0, 9), false),
        GameRecord::CellRequest([3, 4, 3, 0])
    );
    assert_eq!(
        cell_change_record(change(2, 5, 9), false),
        GameRecord::CellRequest([3, 4, 4, 9])
    );
}

#[test]
fn a_short_terrain_record_has_no_runs() {
    assert_eq!(parse_runs(&[4, 0, 0, 0]), None);
    assert_eq!(
        parse_runs(&[6, 0, 1, 0, 0, 0, 2, 3]),
        Some((1, [2, 3], vec![]))
    );
}
