use std::collections::HashSet;

use serde::Serialize;

use crate::entity::TICK_50HZ;
use crate::process::{plausible_heap_pointer, u32_at, Process};

const SOUND_HEAD_PTR: usize = 0x004F_BE60;
const SOUND_END_SENTINEL: usize = 0x004F_BE64;
const SOUND_TAIL_GUARD: usize = 0x004F_BE68;
const MASTER_VOLUME: usize = 0x004F_BE6C;
const SOUND_NODE_BYTES: usize = 0x3C;
const MAX_SOUNDS: usize = 24;

#[derive(Debug, Clone, Serialize)]
pub struct SoundRecord {
    pub pointer: u32,
    pub next: u32,
    pub previous_link: u32,
    pub volume_16_16: i32,
    pub frequency_16_16: i32,
    pub frequency_hz: i64,
    pub pan: i32,
    pub aux_param: u32,
    pub resolved_section11_type: u32,
    pub pcm_pointer: u32,
    pub pcm_bytes: u32,
    pub shared_buffer: u32,
    pub reserved_entry_word: u32,
    pub play_or_ref_state: i32,
    pub directsound_play_flags: u32,
    pub auto_gc: u32,
    pub duplicate_buffer: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SoundSignature {
    pub volume_16_16: i32,
    pub frequency_16_16: i32,
    pub pan: i32,
    pub aux_param: u32,
    pub resolved_section11_type: u32,
    pub pcm_pointer: u32,
    pub pcm_bytes: u32,
    pub play_or_ref_state: i32,
    pub directsound_play_flags: u32,
    pub auto_gc: u32,
}

impl SoundRecord {
    pub fn signature(&self) -> SoundSignature {
        SoundSignature {
            volume_16_16: self.volume_16_16,
            frequency_16_16: self.frequency_16_16,
            pan: self.pan,
            aux_param: self.aux_param,
            resolved_section11_type: self.resolved_section11_type,
            pcm_pointer: self.pcm_pointer,
            pcm_bytes: self.pcm_bytes,
            play_or_ref_state: self.play_or_ref_state,
            directsound_play_flags: self.directsound_play_flags,
            auto_gc: self.auto_gc,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SoundSnapshot {
    pub tick_before: u32,
    pub tick_after: u32,
    pub head_before: u32,
    pub head_after: u32,
    pub tail_before: u32,
    pub tail_after: u32,
    pub topology_stable: bool,
    pub master_volume_16_16: u32,
    pub records: Vec<SoundRecord>,
    pub warnings: Vec<String>,
}

pub fn read_snapshot(process: &Process) -> Result<SoundSnapshot, String> {
    let tick_before = process.read_u32(TICK_50HZ)?;
    let head_before = process.read_u32(SOUND_HEAD_PTR)?;
    let tail_before = process.read_u32(SOUND_TAIL_GUARD)?;
    let master_volume_16_16 = process.read_u32(MASTER_VOLUME)?;
    let mut records = Vec::new();
    let mut warnings = Vec::new();
    let mut visited = HashSet::new();
    let mut current = head_before as usize;

    while current != 0 && current != SOUND_END_SENTINEL && records.len() <= MAX_SOUNDS {
        if !plausible_heap_pointer(current) {
            warnings.push(format!("implausible active-sound pointer {current:08X}"));
            break;
        }
        if !visited.insert(current) {
            warnings.push(format!("active-sound list cycle at {current:08X}"));
            break;
        }
        let bytes = match process.read_bytes(current, SOUND_NODE_BYTES) {
            Ok(bytes) => bytes,
            Err(error) => {
                warnings.push(format!("unreadable sound node {current:08X}: {error}"));
                break;
            }
        };
        let next = u32_at(&bytes, 0x00);
        let frequency_16_16 = u32_at(&bytes, 0x0C) as i32;
        records.push(SoundRecord {
            pointer: current as u32,
            next,
            previous_link: u32_at(&bytes, 0x04),
            volume_16_16: u32_at(&bytes, 0x08) as i32,
            frequency_16_16,
            frequency_hz: frequency_16_16 as i64 * 22_050 / 65_536,
            pan: u32_at(&bytes, 0x10) as i32,
            aux_param: u32_at(&bytes, 0x14),
            resolved_section11_type: u32_at(&bytes, 0x18),
            pcm_pointer: u32_at(&bytes, 0x1C),
            pcm_bytes: u32_at(&bytes, 0x20),
            shared_buffer: u32_at(&bytes, 0x24),
            reserved_entry_word: u32_at(&bytes, 0x28),
            play_or_ref_state: u32_at(&bytes, 0x2C) as i32,
            directsound_play_flags: u32_at(&bytes, 0x30),
            auto_gc: u32_at(&bytes, 0x34),
            duplicate_buffer: u32_at(&bytes, 0x38),
        });
        current = next as usize;
    }
    if records.len() > MAX_SOUNDS {
        warnings.push(format!(
            "more than the proven maximum of {MAX_SOUNDS} active sounds"
        ));
    }

    let head_after = process.read_u32(SOUND_HEAD_PTR)?;
    let tail_after = process.read_u32(SOUND_TAIL_GUARD)?;
    let tick_after = process.read_u32(TICK_50HZ)?;
    Ok(SoundSnapshot {
        tick_before,
        tick_after,
        head_before,
        head_after,
        tail_before,
        tail_after,
        topology_stable: head_before == head_after && tail_before == tail_after,
        master_volume_16_16,
        records,
        warnings,
    })
}
