use serde::Serialize;

use crate::entity::TICK_50HZ;
use crate::process::{i16_at, u32_at, Process};

/// Seven intrusive priority lists begin here. List zero is the free pool;
/// lists one through six are the active draw priorities.
pub const PRIORITY_LISTS: usize = 0x004D_CEE8;
pub const PRIORITY_LIST_COUNT: usize = 7;
pub const PRIORITY_LIST_BYTES: usize = PRIORITY_LIST_COUNT * 0x0C;

/// FUN_00440120 scans this fixed array directly, from 0x004DCF40 through the
/// record at 0x004DE820. The next address, 0x004DE840, is not a particle: it
/// is the emission-table counter.
pub const PARTICLE_POOL: usize = 0x004D_CF40;
pub const PARTICLE_RECORD_BYTES: usize = 0x20;
pub const PARTICLE_CAPACITY: usize = 200;
pub const PARTICLE_POOL_BYTES: usize = PARTICLE_RECORD_BYTES * PARTICLE_CAPACITY;
pub const PARTICLE_DESCRIPTOR_TABLE: usize = 0x004C_C138;
pub const PARTICLE_DESCRIPTOR_BYTES: usize = 0x34;
pub const PARTICLE_DESCRIPTOR_COUNT: usize = 96;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParticleRecord {
    pub slot: usize,
    pub address: u32,
    pub next: u32,
    pub previous_link: u32,
    pub position_raw_8_8: [i16; 3],
    pub velocity_raw: [i16; 3],
    /// Entity handle copied by FUN_00440A60 from the spawn request at +0x0C.
    pub source_entity_handle: u32,
    /// Terrain/water height cache written by the common collision path.
    pub surface_height_raw_8_8: i16,
    /// Zero means that the slot belongs to the free list.
    pub class: u8,
    pub age: u8,
    /// Source entity type recovered by FUN_00440A60 through handle lookup.
    pub source_entity_type: u8,
    pub flags: u8,
    pub trailing_word: u16,
    pub raw_hex: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParticlePoolSnapshot {
    pub tick_before: u32,
    pub tick_after: u32,
    pub attempts: u8,
    pub duplicate_reads_identical: bool,
    pub topology_valid: bool,
    /// This is an observational stability test, not an atomic engine epoch:
    /// retail exposes no proven post-particle-update completion counter.
    pub observationally_stable: bool,
    pub topology_errors: Vec<String>,
    pub records: Vec<ParticleRecord>,
}

pub fn read_snapshot(
    process: &Process,
    maximum_attempts: u8,
) -> Result<ParticlePoolSnapshot, String> {
    let maximum_attempts = maximum_attempts.max(1);
    let descriptors = process.read_bytes(
        PARTICLE_DESCRIPTOR_TABLE,
        PARTICLE_DESCRIPTOR_BYTES * PARTICLE_DESCRIPTOR_COUNT,
    )?;
    let mut last = None;

    for attempt in 1..=maximum_attempts {
        let tick_before = process.read_u32(TICK_50HZ)?;
        let lists_before = process.read_bytes(PRIORITY_LISTS, PRIORITY_LIST_BYTES)?;
        let pool_before = process.read_bytes(PARTICLE_POOL, PARTICLE_POOL_BYTES)?;
        let lists_middle = process.read_bytes(PRIORITY_LISTS, PRIORITY_LIST_BYTES)?;
        let pool_after = process.read_bytes(PARTICLE_POOL, PARTICLE_POOL_BYTES)?;
        let lists_after = process.read_bytes(PRIORITY_LISTS, PRIORITY_LIST_BYTES)?;
        let tick_after = process.read_u32(TICK_50HZ)?;
        let duplicate_reads_identical = tick_before == tick_after
            && lists_before == lists_middle
            && lists_middle == lists_after
            && pool_before == pool_after;
        let topology_errors = validate_topology(&lists_after, &pool_after, &descriptors);
        let topology_valid = topology_errors.is_empty();
        let observationally_stable = duplicate_reads_identical && topology_valid;

        let snapshot = ParticlePoolSnapshot {
            tick_before,
            tick_after,
            attempts: attempt,
            duplicate_reads_identical,
            topology_valid,
            observationally_stable,
            topology_errors,
            records: decode_live_records(&pool_after),
        };
        if observationally_stable {
            return Ok(snapshot);
        }
        last = Some(snapshot);
    }

    Ok(last.expect("at least one particle-pool read attempt"))
}

pub fn generation_changed(before: &ParticleRecord, after: &ParticleRecord) -> bool {
    before.class != after.class || after.age < before.age
}

fn validate_topology(lists: &[u8], pool: &[u8], descriptors: &[u8]) -> Vec<String> {
    let mut errors = Vec::new();
    if lists.len() != PRIORITY_LIST_BYTES
        || pool.len() != PARTICLE_POOL_BYTES
        || descriptors.len() != PARTICLE_DESCRIPTOR_BYTES * PARTICLE_DESCRIPTOR_COUNT
    {
        errors.push("particle topology buffers have the wrong static size".into());
        return errors;
    }

    let mut visited = [false; PARTICLE_CAPACITY];
    for priority in 0..PRIORITY_LIST_COUNT {
        let sentinel = PRIORITY_LISTS + priority * 0x0C;
        let terminal = sentinel + 4;
        let list = &lists[priority * 0x0C..priority * 0x0C + 0x0C];
        if u32_at(list, 0x04) != 0 {
            errors.push(format!(
                "priority {priority} terminal {terminal:08X} is not zero"
            ));
        }

        let mut current = u32_at(list, 0x00) as usize;
        let mut expected_previous_link = sentinel;
        let mut traversed = 0usize;
        while current != terminal {
            let Some(slot) = pool_slot(current) else {
                errors.push(format!(
                    "priority {priority} contains non-pool pointer {current:08X}"
                ));
                break;
            };
            if visited[slot] {
                errors.push(format!(
                    "particle slot {slot} occurs more than once in the priority lists"
                ));
                break;
            }
            visited[slot] = true;
            traversed += 1;
            if traversed > PARTICLE_CAPACITY {
                errors.push(format!("priority {priority} list does not terminate"));
                break;
            }

            let record = &pool[slot * PARTICLE_RECORD_BYTES..(slot + 1) * PARTICLE_RECORD_BYTES];
            let previous_link = u32_at(record, 0x04) as usize;
            if previous_link != expected_previous_link {
                errors.push(format!(
                    "particle slot {slot} previous-link {previous_link:08X} != {expected_previous_link:08X}"
                ));
            }
            let class = usize::from(record[0x1A]);
            if priority == 0 {
                if class != 0 {
                    errors.push(format!(
                        "free-list particle slot {slot} has live class {class}"
                    ));
                }
            } else if class == 0 {
                errors.push(format!(
                    "active priority {priority} particle slot {slot} has class zero"
                ));
            } else if class >= PARTICLE_DESCRIPTOR_COUNT {
                errors.push(format!(
                    "active particle slot {slot} has out-of-range class {class}"
                ));
            } else {
                let descriptor_priority =
                    descriptors[class * PARTICLE_DESCRIPTOR_BYTES + 0x0F] as i8;
                if descriptor_priority != priority as i8 {
                    errors.push(format!(
                        "particle slot {slot} class {class} is in priority {priority}, descriptor says {descriptor_priority}"
                    ));
                }
            }

            expected_previous_link = current;
            current = u32_at(record, 0x00) as usize;
        }

        let tail_previous_link = u32_at(list, 0x08) as usize;
        if current == terminal && tail_previous_link != expected_previous_link {
            errors.push(format!(
                "priority {priority} tail previous-link {tail_previous_link:08X} != {expected_previous_link:08X}"
            ));
        }
    }

    let missing = visited.iter().filter(|present| !**present).count();
    if missing != 0 {
        errors.push(format!(
            "{missing} of {PARTICLE_CAPACITY} particle slots are absent from all priority lists"
        ));
    }
    errors
}

fn pool_slot(pointer: usize) -> Option<usize> {
    let offset = pointer.checked_sub(PARTICLE_POOL)?;
    (offset < PARTICLE_POOL_BYTES && offset % PARTICLE_RECORD_BYTES == 0)
        .then_some(offset / PARTICLE_RECORD_BYTES)
}

fn decode_live_records(pool: &[u8]) -> Vec<ParticleRecord> {
    debug_assert_eq!(pool.len(), PARTICLE_POOL_BYTES);
    pool.chunks_exact(PARTICLE_RECORD_BYTES)
        .enumerate()
        .filter_map(|(slot, bytes)| {
            let class = bytes[0x1A];
            (class != 0).then(|| ParticleRecord {
                slot,
                address: (PARTICLE_POOL + slot * PARTICLE_RECORD_BYTES) as u32,
                next: u32_at(bytes, 0x00),
                previous_link: u32_at(bytes, 0x04),
                position_raw_8_8: [
                    i16_at(bytes, 0x08),
                    i16_at(bytes, 0x0A),
                    i16_at(bytes, 0x0C),
                ],
                velocity_raw: [
                    i16_at(bytes, 0x0E),
                    i16_at(bytes, 0x10),
                    i16_at(bytes, 0x12),
                ],
                source_entity_handle: u32_at(bytes, 0x14),
                surface_height_raw_8_8: i16_at(bytes, 0x18),
                class,
                age: bytes[0x1B],
                source_entity_type: bytes[0x1C],
                flags: bytes[0x1D],
                trailing_word: u16::from_le_bytes([bytes[0x1E], bytes[0x1F]]),
                raw_hex: encode_hex(bytes),
            })
        })
        .collect()
}

fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(text, "{byte:02x}");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::{
        decode_live_records, generation_changed, validate_topology, PARTICLE_CAPACITY,
        PARTICLE_DESCRIPTOR_BYTES, PARTICLE_DESCRIPTOR_COUNT, PARTICLE_POOL, PARTICLE_POOL_BYTES,
        PARTICLE_RECORD_BYTES, PRIORITY_LISTS, PRIORITY_LIST_BYTES,
    };

    #[test]
    fn decodes_the_proven_0x20_byte_record_layout() {
        let mut pool = vec![0u8; PARTICLE_POOL_BYTES];
        let record = &mut pool[5 * PARTICLE_RECORD_BYTES..6 * PARTICLE_RECORD_BYTES];
        record[0x00..0x04].copy_from_slice(&0x004D_D020u32.to_le_bytes());
        record[0x04..0x08].copy_from_slice(&0x004D_CF80u32.to_le_bytes());
        record[0x08..0x0A].copy_from_slice(&(-256i16).to_le_bytes());
        record[0x0A..0x0C].copy_from_slice(&384i16.to_le_bytes());
        record[0x0C..0x0E].copy_from_slice(&512i16.to_le_bytes());
        record[0x0E..0x10].copy_from_slice(&12i16.to_le_bytes());
        record[0x10..0x12].copy_from_slice(&(-34i16).to_le_bytes());
        record[0x12..0x14].copy_from_slice(&56i16.to_le_bytes());
        record[0x14..0x18].copy_from_slice(&0x0000_9822u32.to_le_bytes());
        record[0x18..0x1A].copy_from_slice(&128i16.to_le_bytes());
        record[0x1A] = 18;
        record[0x1B] = 7;
        record[0x1C] = 67;
        record[0x1D] = 0x14;
        record[0x1E..0x20].copy_from_slice(&0xBEEFu16.to_le_bytes());

        let decoded = decode_live_records(&pool);
        assert_eq!(decoded.len(), 1);
        let decoded = &decoded[0];
        assert_eq!(decoded.slot, 5);
        assert_eq!(decoded.position_raw_8_8, [-256, 384, 512]);
        assert_eq!(decoded.velocity_raw, [12, -34, 56]);
        assert_eq!(decoded.source_entity_handle, 0x0000_9822);
        assert_eq!(decoded.class, 18);
        assert_eq!(decoded.age, 7);
        assert_eq!(decoded.source_entity_type, 67);
        assert_eq!(decoded.trailing_word, 0xBEEF);
    }

    #[test]
    fn class_change_or_age_reset_marks_slot_reuse() {
        let mut pool = vec![0u8; PARTICLE_POOL_BYTES];
        pool[0x1A] = 31;
        pool[0x1B] = 12;
        let before = decode_live_records(&pool).pop().unwrap();

        pool[0x1B] = 13;
        let continued = decode_live_records(&pool).pop().unwrap();
        assert!(!generation_changed(&before, &continued));

        pool[0x1B] = 0;
        let reset = decode_live_records(&pool).pop().unwrap();
        assert!(generation_changed(&continued, &reset));

        pool[0x1A] = 42;
        let changed_class = decode_live_records(&pool).pop().unwrap();
        assert!(generation_changed(&reset, &changed_class));
    }

    #[test]
    fn validates_a_complete_free_list_and_empty_active_lists() {
        let mut lists = vec![0u8; PRIORITY_LIST_BYTES];
        let mut pool = vec![0u8; PARTICLE_POOL_BYTES];
        let mut descriptors = vec![0u8; PARTICLE_DESCRIPTOR_BYTES * PARTICLE_DESCRIPTOR_COUNT];

        for priority in 0..7 {
            let sentinel = PRIORITY_LISTS + priority * 0x0C;
            let terminal = sentinel + 4;
            let list = &mut lists[priority * 0x0C..priority * 0x0C + 0x0C];
            list[0x00..0x04].copy_from_slice(&(terminal as u32).to_le_bytes());
            list[0x08..0x0C].copy_from_slice(&(sentinel as u32).to_le_bytes());
        }

        lists[0x00..0x04].copy_from_slice(&(PARTICLE_POOL as u32).to_le_bytes());
        let last = PARTICLE_POOL + (PARTICLE_CAPACITY - 1) * PARTICLE_RECORD_BYTES;
        lists[0x08..0x0C].copy_from_slice(&(last as u32).to_le_bytes());
        for slot in 0..PARTICLE_CAPACITY {
            let record =
                &mut pool[slot * PARTICLE_RECORD_BYTES..(slot + 1) * PARTICLE_RECORD_BYTES];
            let next = if slot + 1 == PARTICLE_CAPACITY {
                PRIORITY_LISTS + 4
            } else {
                PARTICLE_POOL + (slot + 1) * PARTICLE_RECORD_BYTES
            };
            let previous_link = if slot == 0 {
                PRIORITY_LISTS
            } else {
                PARTICLE_POOL + (slot - 1) * PARTICLE_RECORD_BYTES
            };
            record[0x00..0x04].copy_from_slice(&(next as u32).to_le_bytes());
            record[0x04..0x08].copy_from_slice(&(previous_link as u32).to_le_bytes());
        }

        assert_eq!(
            validate_topology(&lists, &pool, &descriptors),
            Vec::<String>::new()
        );

        // Move slot zero from the free list to active priority five and give
        // its class the matching authored descriptor priority.
        lists[0x00..0x04]
            .copy_from_slice(&((PARTICLE_POOL + PARTICLE_RECORD_BYTES) as u32).to_le_bytes());
        pool[PARTICLE_RECORD_BYTES + 0x04..PARTICLE_RECORD_BYTES + 0x08]
            .copy_from_slice(&(PRIORITY_LISTS as u32).to_le_bytes());
        let active_sentinel = PRIORITY_LISTS + 5 * 0x0C;
        let active_list = &mut lists[5 * 0x0C..6 * 0x0C];
        active_list[0x00..0x04].copy_from_slice(&(PARTICLE_POOL as u32).to_le_bytes());
        active_list[0x08..0x0C].copy_from_slice(&(PARTICLE_POOL as u32).to_le_bytes());
        pool[0x00..0x04].copy_from_slice(&((active_sentinel + 4) as u32).to_le_bytes());
        pool[0x04..0x08].copy_from_slice(&(active_sentinel as u32).to_le_bytes());
        pool[0x1A] = 31;
        descriptors[31 * PARTICLE_DESCRIPTOR_BYTES + 0x0F] = 5;

        assert_eq!(
            validate_topology(&lists, &pool, &descriptors),
            Vec::<String>::new()
        );
    }
}
