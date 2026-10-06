use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::process::{plausible_data_pointer, plausible_heap_pointer, u32_at, Process};

const ENTRY_BYTES: usize = 0x14;
const MAX_SOUND_RESOURCES: usize = 4096;
const MAX_ALIAS_DEPTH: usize = 16;
const MAX_PCM_BYTES: usize = 16 * 1024 * 1024;

type SoundResolution<'a> = Result<(Vec<u16>, &'a RawSoundResource), (Vec<u16>, String)>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SoundResource {
    pub global_id: u16,
    pub source_level: u32,
    pub source_local_id: u32,
    pub source_name: String,
    pub record_pointer: u32,
    pub raw_type: u32,
    pub field_04: u32,
    pub field_08: u32,
    pub frequency_multiplier_16_16: u32,
    pub volume_multiplier_16_16: u32,
    pub alias_target_global_id: Option<u16>,
    pub resolution_chain: Vec<u16>,
    pub resolved_pcm_global_id: Option<u16>,
    pub pcm_pointer: u32,
    pub pcm_bytes: u32,
    pub pcm_fnv1a64: Option<String>,
    pub pcm_hash_error: Option<String>,
    pub resolution_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SoundResourceCatalog {
    pub table_pointer: u32,
    pub slot_count: u32,
    pub resources: Vec<SoundResource>,
}

#[derive(Debug, Clone)]
struct RawSoundResource {
    global_id: u16,
    record_pointer: u32,
    raw_type: u32,
    field_04: u32,
    field_08: u32,
    frequency_multiplier_16_16: u32,
    volume_multiplier_16_16: u32,
}

pub fn read(
    process: &Process,
    table_pointer: u32,
    slot_count: u32,
) -> Result<SoundResourceCatalog, String> {
    let slot_count_usize = slot_count as usize;
    if slot_count_usize > MAX_SOUND_RESOURCES {
        return Err(format!(
            "sound resource count {slot_count} exceeds safety limit {MAX_SOUND_RESOURCES}"
        ));
    }
    if !plausible_heap_pointer(table_pointer as usize) {
        return Err(format!(
            "sound resource table pointer {table_pointer:08X} is implausible"
        ));
    }

    let table = process.read_bytes(table_pointer as usize, slot_count_usize * 4)?;
    let mut raw = Vec::new();
    for global_id in 0..slot_count_usize {
        let record_pointer = u32_at(&table, global_id * 4);
        if record_pointer == 0 {
            continue;
        }
        if !plausible_heap_pointer(record_pointer as usize) {
            return Err(format!(
                "sound {global_id} record pointer {record_pointer:08X} is implausible"
            ));
        }
        let bytes = process.read_bytes(record_pointer as usize, ENTRY_BYTES)?;
        raw.push(RawSoundResource {
            global_id: global_id as u16,
            record_pointer,
            raw_type: u32_at(&bytes, 0x00),
            field_04: u32_at(&bytes, 0x04),
            field_08: u32_at(&bytes, 0x08),
            frequency_multiplier_16_16: u32_at(&bytes, 0x0C),
            volume_multiplier_16_16: u32_at(&bytes, 0x10),
        });
    }

    let by_id: HashMap<u16, &RawSoundResource> =
        raw.iter().map(|entry| (entry.global_id, entry)).collect();
    let by_pointer: HashMap<u32, u16> = raw
        .iter()
        .map(|entry| (entry.record_pointer, entry.global_id))
        .collect();
    let mut pcm_hashes: HashMap<(u32, u32), Result<String, String>> = HashMap::new();
    let mut resources = Vec::with_capacity(raw.len());

    for entry in &raw {
        let alias_target_global_id = (entry.raw_type == 5)
            .then(|| by_pointer.get(&entry.field_04).copied())
            .flatten();
        let resolution = resolve_pcm(entry.global_id, &by_id, &by_pointer);
        let (resolution_chain, resolved_pcm_global_id, pcm_pointer, pcm_bytes, resolution_error) =
            match resolution {
                Ok((chain, pcm)) => (chain, Some(pcm.global_id), pcm.field_04, pcm.field_08, None),
                Err((chain, error)) => (chain, None, 0, 0, Some(error)),
            };
        let (pcm_fnv1a64, pcm_hash_error) = if resolved_pcm_global_id.is_some() {
            let result = pcm_hashes
                .entry((pcm_pointer, pcm_bytes))
                .or_insert_with(|| {
                    if pcm_bytes as usize > MAX_PCM_BYTES {
                        return Err(format!(
                            "PCM size {pcm_bytes:#x} exceeds safety limit {MAX_PCM_BYTES:#x}"
                        ));
                    }
                    if !plausible_data_pointer(pcm_pointer as usize) {
                        return Err(format!(
                            "PCM data pointer {pcm_pointer:08X} is outside 32-bit user space"
                        ));
                    }
                    process
                        .read_bytes(pcm_pointer as usize, pcm_bytes as usize)
                        .map(|bytes| format!("{:016X}", fnv1a64(&bytes)))
                });
            match result {
                Ok(hash) => (Some(hash.clone()), None),
                Err(error) => (None, Some(error.clone())),
            }
        } else {
            (None, None)
        };
        let (source_level, source_local_id, source_name) = source_identity(entry.global_id);
        resources.push(SoundResource {
            global_id: entry.global_id,
            source_level,
            source_local_id,
            source_name,
            record_pointer: entry.record_pointer,
            raw_type: entry.raw_type,
            field_04: entry.field_04,
            field_08: entry.field_08,
            frequency_multiplier_16_16: entry.frequency_multiplier_16_16,
            volume_multiplier_16_16: entry.volume_multiplier_16_16,
            alias_target_global_id,
            resolution_chain,
            resolved_pcm_global_id,
            pcm_pointer,
            pcm_bytes,
            pcm_fnv1a64,
            pcm_hash_error,
            resolution_error,
        });
    }

    Ok(SoundResourceCatalog {
        table_pointer,
        slot_count,
        resources,
    })
}

fn resolve_pcm<'a>(
    start: u16,
    by_id: &HashMap<u16, &'a RawSoundResource>,
    by_pointer: &HashMap<u32, u16>,
) -> SoundResolution<'a> {
    let mut chain = Vec::new();
    let mut visited = HashSet::new();
    let mut current = start;
    for _ in 0..MAX_ALIAS_DEPTH {
        if !visited.insert(current) {
            return Err((
                chain,
                format!("alias cycle returned to global sound {current}"),
            ));
        }
        chain.push(current);
        let Some(entry) = by_id.get(&current).copied() else {
            return Err((chain, format!("global sound {current} is not loaded")));
        };
        match entry.raw_type {
            1 => return Ok((chain, entry)),
            5 => {
                let Some(target) = by_pointer.get(&entry.field_04).copied() else {
                    return Err((
                        chain,
                        format!(
                            "alias global sound {current} targets unknown pointer {:08X}",
                            entry.field_04
                        ),
                    ));
                };
                current = target;
            }
            other => {
                return Err((
                    chain,
                    format!("global sound {current} has unsupported type {other}"),
                ));
            }
        }
    }
    Err((
        chain,
        format!("alias chain exceeds safety depth {MAX_ALIAS_DEPTH}"),
    ))
}

fn source_identity(global_id: u16) -> (u32, u32, String) {
    if global_id < 7 {
        let local = u32::from(global_id);
        (2, local, format!("sec11_2XX_{local:03}"))
    } else {
        let local = u32::from(global_id - 7);
        (3, local, format!("sec11_3XX_{local:03}"))
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xCBF2_9CE4_8422_2325u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::source_identity;

    #[test]
    fn global_sound_ids_map_to_the_two_contributing_system_levels() {
        assert_eq!(source_identity(6), (2, 6, "sec11_2XX_006".into()));
        assert_eq!(source_identity(7), (3, 0, "sec11_3XX_000".into()));
        assert_eq!(source_identity(50), (3, 43, "sec11_3XX_043".into()));
    }
}
