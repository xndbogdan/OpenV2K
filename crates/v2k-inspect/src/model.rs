use serde::Serialize;

use crate::entity::MODEL_POOL_PTR;
use crate::process::{plausible_heap_pointer, u16_at, Process};

const ENTITY_TYPE_TABLE_PTR: usize = 0x004F_E650;
const MAX_ENTRY_BYTES: usize = 1024 * 1024;
const MAX_NAME_BYTES: usize = 128;

#[derive(Debug, Clone, Serialize)]
pub struct ModelResource {
    pub model_id: u16,
    pub table_pointer: u32,
    pub entry_pointer: u32,
    pub command_word_count: u16,
    pub extra_count: u8,
    pub flags: u8,
    pub slot_count: u16,
    pub vertex_count: u16,
    pub face_val: u16,
    pub normal_count: u16,
    pub radius: u16,
    pub radius2: u16,
    pub name_offset: u32,
    pub name: Option<String>,
    pub aligned_entry_length: u32,
    pub fnv1a64: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntityTypeDefaults {
    pub entity_type: u32,
    pub table_pointer: u32,
    pub record_pointer: u32,
    pub models: [u16; 4],
}

pub fn resolve(process: &Process, model_id: u16) -> Result<ModelResource, String> {
    let table_pointer = process.read_u32(MODEL_POOL_PTR)?;
    require_pointer(table_pointer, "model table")?;
    let entry_pointer = process.read_u32(table_pointer as usize + usize::from(model_id) * 4)?;
    require_pointer(entry_pointer, "model entry")?;
    let header = process.read_bytes(entry_pointer as usize, 12)?;
    let command_word_count = u16_at(&header, 0x00);
    let extra_count = header[0x02];
    let flags = header[0x03];
    let slot_count = u16_at(&header, 0x04);
    let face_val = u16_at(&header, 0x06);
    let radius = u16_at(&header, 0x08);
    let radius2 = u16_at(&header, 0x0A);
    if face_val < 2 {
        return Err(format!(
            "model {model_id} entry {entry_pointer:08X} has invalid face_val {face_val}"
        ));
    }
    let vertex_count = slot_count >> 1;
    let normal_count = (face_val - 2) >> 1;
    let name_offset = 12usize
        .checked_add(4usize * usize::from(extra_count))
        .and_then(|offset| offset.checked_add(8usize * usize::from(vertex_count + normal_count)))
        .and_then(|offset| offset.checked_add(2usize * usize::from(command_word_count)))
        .ok_or_else(|| format!("model {model_id} entry-size arithmetic overflow"))?;
    if name_offset > MAX_ENTRY_BYTES {
        return Err(format!(
            "model {model_id} name offset {name_offset:#x} exceeds safety limit"
        ));
    }

    let (name, unaligned_length) = if flags & 0x40 == 0 {
        let (name, byte_length) = read_c_string_bounded(
            process,
            entry_pointer as usize + name_offset,
            MAX_NAME_BYTES,
        )?;
        (Some(name), name_offset + byte_length + 1)
    } else {
        (None, name_offset)
    };
    let aligned_entry_length = (unaligned_length + 3) & !3;
    if aligned_entry_length > MAX_ENTRY_BYTES {
        return Err(format!(
            "model {model_id} entry length {aligned_entry_length:#x} exceeds safety limit"
        ));
    }
    let entry_bytes = process.read_bytes(entry_pointer as usize, aligned_entry_length)?;

    Ok(ModelResource {
        model_id,
        table_pointer,
        entry_pointer,
        command_word_count,
        extra_count,
        flags,
        slot_count,
        vertex_count,
        face_val,
        normal_count,
        radius,
        radius2,
        name_offset: name_offset as u32,
        name,
        aligned_entry_length: aligned_entry_length as u32,
        fnv1a64: format!("{:016X}", fnv1a64(&entry_bytes)),
    })
}

fn read_c_string_bounded(
    process: &Process,
    address: usize,
    maximum_bytes: usize,
) -> Result<(String, usize), String> {
    // Read one byte at a time so a short name near the end of a committed
    // page does not fail merely because a fixed-size bulk read crosses into
    // an uncommitted page.
    let mut bytes = Vec::with_capacity(maximum_bytes.min(32));
    for offset in 0..=maximum_bytes {
        let byte = process.read_bytes(address + offset, 1)?[0];
        if byte == 0 {
            let byte_length = bytes.len();
            return Ok((String::from_utf8_lossy(&bytes).into_owned(), byte_length));
        }
        bytes.push(byte);
    }
    Err(format!(
        "C string at {address:08X} is not terminated within {maximum_bytes} bytes"
    ))
}

pub fn resolve_type_defaults(
    process: &Process,
    entity_type: u32,
) -> Result<EntityTypeDefaults, String> {
    let table_pointer = process.read_u32(ENTITY_TYPE_TABLE_PTR)?;
    require_pointer(table_pointer, "entity-type table")?;
    let record_pointer = process.read_u32(table_pointer as usize + entity_type as usize * 4)?;
    require_pointer(record_pointer, "entity-type record")?;
    let bytes = process.read_bytes(record_pointer as usize + 0x0C, 8)?;
    Ok(EntityTypeDefaults {
        entity_type,
        table_pointer,
        record_pointer,
        models: [
            u16_at(&bytes, 0),
            u16_at(&bytes, 2),
            u16_at(&bytes, 4),
            u16_at(&bytes, 6),
        ],
    })
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xCBF2_9CE4_8422_2325u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

fn require_pointer(pointer: u32, label: &str) -> Result<(), String> {
    if plausible_heap_pointer(pointer as usize) {
        Ok(())
    } else {
        Err(format!("{label} pointer {pointer:08X} is implausible"))
    }
}

#[cfg(test)]
mod tests {
    use super::fnv1a64;

    #[test]
    fn fnv_matches_known_empty_and_a_vectors() {
        assert_eq!(fnv1a64(b""), 0xCBF2_9CE4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xAF63_DC4C_8601_EC8C);
    }
}
