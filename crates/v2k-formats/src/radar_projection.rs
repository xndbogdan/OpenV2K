//! Typed decoder for the three radar-projection grids in system level 3.
//!
//! The first three Section-14 linkage records are, in order, the full-size
//! globe shade grid, a half-width/half-height grid of four mirrored source
//! coordinates, and a half-size edge-mask grid. System level 3 has a fourth
//! unrelated 38x7 linkage record; this decoder deliberately leaves it in the
//! generic [`LinkageTable`] and consumes only the first three records.

use std::ops::Range;

use crate::linkage::{LinkageRecord, LinkageTable, RECORD_SIZE};
use crate::ovl::read_u32;
use v2k_core::{Result, V2kError};

const RADAR_RECORD_COUNT: usize = 3;
const DIMENSION_HEADER_BYTES: usize = 8;
const DEFAULT_VALUE_BYTES: usize = 4;

/// One signed source coordinate in the radar globe's projection domain. The
/// four coordinates in each tuple retain their authored on-disk order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadarProjectionCoordinate {
    pub x: i16,
    pub z: i16,
}

/// Four terrain-footprint source coordinates for one projected globe pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadarProjectionCoordinateTuple {
    pub samples: [RadarProjectionCoordinate; 4],
}

/// The three authored resources used to shade and project the right-HUD radar
/// globe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadarProjectionTables {
    /// Full globe width from the first record's payload header.
    pub width: usize,
    /// Full globe height from the first record's payload header.
    pub height: usize,
    /// One palette/shade byte per full-resolution globe pixel.
    pub shade_grid: Vec<u8>,
    /// `(width / 2) * (height / 2)` four-coordinate tuples.
    pub coordinate_tuples: Vec<RadarProjectionCoordinateTuple>,
    /// One four-neighbour edge-mask byte per half-resolution grid point.
    pub mask_grid: Vec<u8>,
}

impl RadarProjectionTables {
    pub fn half_width(&self) -> usize {
        self.width / 2
    }

    pub fn half_height(&self) -> usize {
        self.height / 2
    }
}

impl LinkageTable {
    /// Decode system level 3's first three Section-14 records as radar
    /// projection resources. Any later generic linkage records remain intact.
    pub fn radar_projection_tables(&self) -> Result<RadarProjectionTables> {
        parse_radar_projection_tables(self)
    }
}

/// Decode and strictly validate system level 3's first three Section-14
/// linkage records.
pub fn parse_radar_projection_tables(table: &LinkageTable) -> Result<RadarProjectionTables> {
    if table.records.len() < RADAR_RECORD_COUNT {
        return Err(section_error(format!(
            "radar projection requires at least {RADAR_RECORD_COUNT} linkage records, found {}",
            table.records.len()
        )));
    }

    let shade = parse_grid_record(table, 0, "shade", 1, 1)?;
    let coordinates = parse_grid_record(table, 1, "coordinate", 8, 16)?;
    let mask = parse_grid_record(table, 2, "mask", 1, 1)?;

    if shade.width % 2 != 0 || shade.height % 2 != 0 {
        return Err(section_error(format!(
            "radar shade dimensions must be even, found {}x{}",
            shade.width, shade.height
        )));
    }
    let half_dimensions = [shade.width / 2, shade.height / 2];
    if [coordinates.width, coordinates.height] != half_dimensions {
        return Err(section_error(format!(
            "radar coordinate dimensions {}x{} do not match half shade dimensions {}x{}",
            coordinates.width, coordinates.height, half_dimensions[0], half_dimensions[1]
        )));
    }
    if [mask.width, mask.height] != half_dimensions {
        return Err(section_error(format!(
            "radar mask dimensions {}x{} do not match half shade dimensions {}x{}",
            mask.width, mask.height, half_dimensions[0], half_dimensions[1]
        )));
    }
    ensure_disjoint_payloads([&shade, &coordinates, &mask])?;

    let coordinate_tuples = coordinates
        .payload
        .chunks_exact(16)
        .map(|tuple| {
            let mut offset = 0;
            let samples = std::array::from_fn(|_| {
                let x = i16::from_le_bytes([tuple[offset], tuple[offset + 1]]);
                let z = i16::from_le_bytes([tuple[offset + 2], tuple[offset + 3]]);
                offset += 4;
                RadarProjectionCoordinate { x, z }
            });
            RadarProjectionCoordinateTuple { samples }
        })
        .collect();

    Ok(RadarProjectionTables {
        width: shade.width,
        height: shade.height,
        shade_grid: shade.payload.to_vec(),
        coordinate_tuples,
        mask_grid: mask.payload.to_vec(),
    })
}

struct ParsedGridRecord<'a> {
    width: usize,
    height: usize,
    payload: &'a [u8],
    payload_range: Range<usize>,
}

fn parse_grid_record<'a>(
    table: &'a LinkageTable,
    index: usize,
    role: &str,
    expected_values_per_record: u32,
    expected_stride: u32,
) -> Result<ParsedGridRecord<'a>> {
    let record = &table.records[index];
    validate_descriptor_shape(
        record,
        index,
        role,
        expected_values_per_record,
        expected_stride,
    )?;

    let descriptor_bytes = table
        .records
        .len()
        .checked_mul(RECORD_SIZE)
        .ok_or_else(|| section_error("linkage descriptor byte count overflow"))?;
    let dimensions_start = pointer_offset(record.region1_ptr, index, role, "dimensions")?;
    let dimensions_end = checked_end(
        dimensions_start,
        DIMENSION_HEADER_BYTES,
        table.data.len(),
        index,
        role,
        "dimensions",
    )?;
    let defaults_start = pointer_offset(record.region2_ptr, index, role, "defaults")?;
    let defaults_bytes = usize::try_from(record.layer_count)
        .ok()
        .and_then(|count| count.checked_mul(DEFAULT_VALUE_BYTES))
        .ok_or_else(|| section_error(format!("radar {role} record {index} defaults overflow")))?;
    let defaults_end = checked_end(
        defaults_start,
        defaults_bytes,
        table.data.len(),
        index,
        role,
        "defaults",
    )?;
    let payload_start = pointer_offset(record.region3_ptr, index, role, "payload")?;

    for (name, start) in [
        ("dimensions", dimensions_start),
        ("defaults", defaults_start),
        ("payload", payload_start),
    ] {
        if start < descriptor_bytes {
            return Err(section_error(format!(
                "radar {role} record {index} {name} pointer {start} aliases the {descriptor_bytes}-byte descriptor array"
            )));
        }
    }
    if dimensions_end > defaults_start || defaults_end > payload_start {
        return Err(section_error(format!(
            "radar {role} record {index} dimensions/defaults/payload regions overlap"
        )));
    }

    let width = usize::try_from(read_u32(&table.data, dimensions_start)?)
        .map_err(|_| section_error(format!("radar {role} width does not fit usize")))?;
    let height = usize::try_from(read_u32(&table.data, dimensions_start + 4)?)
        .map_err(|_| section_error(format!("radar {role} height does not fit usize")))?;
    if width == 0 || height == 0 {
        return Err(section_error(format!(
            "radar {role} record {index} has zero dimensions {width}x{height}"
        )));
    }
    let expected_entries = width.checked_mul(height).ok_or_else(|| {
        section_error(format!(
            "radar {role} record {index} dimension product overflows"
        ))
    })?;
    let entry_count = usize::try_from(record.entry_count)
        .map_err(|_| section_error(format!("radar {role} entry count does not fit usize")))?;
    if entry_count != expected_entries {
        return Err(section_error(format!(
            "radar {role} record {index} declares {entry_count} entries for {width}x{height} dimensions"
        )));
    }
    let stride = usize::try_from(record.entry_size)
        .map_err(|_| section_error(format!("radar {role} stride does not fit usize")))?;
    let payload_bytes = entry_count.checked_mul(stride).ok_or_else(|| {
        section_error(format!(
            "radar {role} record {index} payload byte count overflows"
        ))
    })?;
    let payload_end = checked_end(
        payload_start,
        payload_bytes,
        table.data.len(),
        index,
        role,
        "payload",
    )?;

    Ok(ParsedGridRecord {
        width,
        height,
        payload: &table.data[payload_start..payload_end],
        payload_range: payload_start..payload_end,
    })
}

fn validate_descriptor_shape(
    record: &LinkageRecord,
    index: usize,
    role: &str,
    expected_values_per_record: u32,
    expected_stride: u32,
) -> Result<()> {
    if record.record_type != 2
        || record.layer_count != expected_values_per_record
        || record.entry_size != expected_stride
    {
        return Err(section_error(format!(
            "radar {role} record {index} has descriptor shape type={}, values={}, stride={}; expected type=2, values={expected_values_per_record}, stride={expected_stride}",
            record.record_type, record.layer_count, record.entry_size
        )));
    }
    Ok(())
}

fn pointer_offset(pointer: u32, index: usize, role: &str, region: &str) -> Result<usize> {
    let offset = usize::try_from(pointer).map_err(|_| {
        section_error(format!(
            "radar {role} record {index} {region} pointer does not fit usize"
        ))
    })?;
    if offset % 4 != 0 {
        return Err(section_error(format!(
            "radar {role} record {index} {region} pointer {offset} is not 4-byte aligned"
        )));
    }
    Ok(offset)
}

fn checked_end(
    start: usize,
    bytes: usize,
    data_len: usize,
    index: usize,
    role: &str,
    region: &str,
) -> Result<usize> {
    let end = start.checked_add(bytes).ok_or_else(|| {
        section_error(format!(
            "radar {role} record {index} {region} range overflows"
        ))
    })?;
    if end > data_len {
        return Err(section_error(format!(
            "radar {role} record {index} {region} range {start}..{end} exceeds {} bytes",
            data_len
        )));
    }
    Ok(end)
}

fn ensure_disjoint_payloads(records: [&ParsedGridRecord<'_>; RADAR_RECORD_COUNT]) -> Result<()> {
    for left in 0..records.len() {
        for right in left + 1..records.len() {
            let a = &records[left].payload_range;
            let b = &records[right].payload_range;
            if a.start < b.end && b.start < a.end {
                return Err(section_error(format!(
                    "radar projection payloads {left} and {right} overlap"
                )));
            }
        }
    }
    Ok(())
}

fn section_error(message: impl Into<String>) -> V2kError {
    V2kError::section(14, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn append_record_payload(
        data: &mut Vec<u8>,
        dimensions: [u32; 2],
        defaults: &[u32],
        payload: &[u8],
        entry_size: u32,
    ) -> LinkageRecord {
        let region1_ptr = data.len() as u32;
        data.extend_from_slice(&dimensions[0].to_le_bytes());
        data.extend_from_slice(&dimensions[1].to_le_bytes());
        let region2_ptr = data.len() as u32;
        for value in defaults {
            data.extend_from_slice(&value.to_le_bytes());
        }
        let region3_ptr = data.len() as u32;
        data.extend_from_slice(payload);
        LinkageRecord {
            record_type: 2,
            layer_count: defaults.len() as u32,
            entry_count: dimensions[0] * dimensions[1],
            entry_size,
            region1_ptr,
            region2_ptr,
            region3_ptr,
        }
    }

    fn synthetic_table() -> LinkageTable {
        let mut data = vec![0; 4 * RECORD_SIZE];
        let shade = append_record_payload(&mut data, [4, 2], &[1], &[0, 1, 2, 3, 4, 5, 6, 7], 1);

        let mut coordinate_payload = Vec::new();
        for coordinates in [
            [(-1_i16, -2_i16), (3, 4), (-5, 6), (7, -8)],
            [(9, 10), (-11, 12), (13, -14), (-15, -16)],
        ] {
            for (x, z) in coordinates {
                coordinate_payload.extend_from_slice(&x.to_le_bytes());
                coordinate_payload.extend_from_slice(&z.to_le_bytes());
            }
        }
        let coordinates =
            append_record_payload(&mut data, [2, 1], &[2; 8], &coordinate_payload, 16);
        let mask = append_record_payload(&mut data, [2, 1], &[1], &[0x05, 0x0a], 1);
        let unrelated = append_record_payload(&mut data, [1, 1], &[1], &[0xff], 1);

        LinkageTable {
            records: vec![shade, coordinates, mask, unrelated],
            data,
        }
    }

    #[test]
    fn decodes_three_grids_and_ignores_fourth_linkage_record() {
        let table = synthetic_table();
        let projection = table.radar_projection_tables().unwrap();

        assert_eq!(table.records.len(), 4);
        assert_eq!((projection.width, projection.height), (4, 2));
        assert_eq!((projection.half_width(), projection.half_height()), (2, 1));
        assert_eq!(projection.shade_grid, [0, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(projection.mask_grid, [0x05, 0x0a]);
        assert_eq!(
            projection.coordinate_tuples,
            [
                RadarProjectionCoordinateTuple {
                    samples: [
                        RadarProjectionCoordinate { x: -1, z: -2 },
                        RadarProjectionCoordinate { x: 3, z: 4 },
                        RadarProjectionCoordinate { x: -5, z: 6 },
                        RadarProjectionCoordinate { x: 7, z: -8 },
                    ],
                },
                RadarProjectionCoordinateTuple {
                    samples: [
                        RadarProjectionCoordinate { x: 9, z: 10 },
                        RadarProjectionCoordinate { x: -11, z: 12 },
                        RadarProjectionCoordinate { x: 13, z: -14 },
                        RadarProjectionCoordinate { x: -15, z: -16 },
                    ],
                },
            ]
        );
    }

    #[test]
    fn rejects_descriptor_shape_and_dimension_mismatches() {
        let mut wrong_stride = synthetic_table();
        wrong_stride.records[1].entry_size = 8;
        assert!(wrong_stride.radar_projection_tables().is_err());

        let mut wrong_half_dimensions = synthetic_table();
        let dimensions = wrong_half_dimensions.records[2].region1_ptr as usize;
        wrong_half_dimensions.data[dimensions..dimensions + 4].copy_from_slice(&1u32.to_le_bytes());
        assert!(wrong_half_dimensions.radar_projection_tables().is_err());

        let mut odd_full_width = synthetic_table();
        let dimensions = odd_full_width.records[0].region1_ptr as usize;
        odd_full_width.data[dimensions..dimensions + 4].copy_from_slice(&3u32.to_le_bytes());
        odd_full_width.records[0].entry_count = 6;
        assert!(odd_full_width.radar_projection_tables().is_err());
    }

    #[test]
    fn rejects_truncated_aliasing_and_overlapping_payloads() {
        let mut truncated = synthetic_table();
        truncated.records[2].region3_ptr = truncated.data.len() as u32;
        assert!(truncated.radar_projection_tables().is_err());

        let mut descriptor_alias = synthetic_table();
        descriptor_alias.records[0].region1_ptr = 0;
        assert!(descriptor_alias.radar_projection_tables().is_err());

        let mut overlap = synthetic_table();
        overlap.records[0].region3_ptr = overlap.records[1].region3_ptr;
        let error = overlap.radar_projection_tables().unwrap_err();
        assert!(error.to_string().contains("payloads 0 and 1 overlap"));
    }
}
