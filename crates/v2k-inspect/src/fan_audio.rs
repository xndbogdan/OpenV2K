//! Read-only evidence for the shared Hover/VTOL fan controller.
//!
//! `FUN_00420A50` resolves the player entity from the controller handle, then
//! reaches the 0x4C-byte Sub-O runtime block through
//! `entity+0x4C -> root+0x0C -> component table+0x38`. `FUN_00420920` updates
//! the RPM and gain-envelope dwords at +0x20/+0x24 before applying them to the
//! two persistent positional voices. This module passively follows that exact
//! chain; it never invokes the retail resource lookup or either callback.

use serde::Serialize;

use crate::audio::{SoundRecord, SoundSnapshot};
use crate::controller::ControllerMotionEvidence;
use crate::entity::TICK_50HZ;
use crate::process::{plausible_heap_pointer, u32_at, Process};
use crate::resources::{resolve_resource_handle, ResourceCacheResolution};
use crate::sound_resource::SoundResource;

const GLOBAL_DELTA_US: usize = 0x004D_04E4;
const TARGET_GLOBAL_SOUNDS: [u16; 2] = [47, 31];

const ENTITY_WINDOW_BYTES: usize = 0x90;
const ENTITY_COMPONENT_ROOT_OFFSET: usize = 0x4C;
const ENTITY_TYPE_OFFSET: usize = 0x58;
const ENTITY_HANDLE_OFFSET: usize = 0x5C;
const ENTITY_LOWER_FAN_HANDLE_OFFSET: usize = 0x8C;
const PLAYER_ENTITY_TYPE: u32 = 46;

const COMPONENT_TABLE_OFFSET: usize = 0x0C;
const FAN_STATE_SLOT_OFFSET: usize = 0x38;
const FAN_STATE_BYTES: usize = 0x4C;
const UPPER_FAN_HANDLE_OFFSET: usize = 0x1C;
const RPM_STATE_OFFSET: usize = 0x20;
const GAIN_ENVELOPE_OFFSET: usize = 0x24;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FanDriveValues {
    pub upper_voice_handle_at_0x1c: u32,
    /// State consumed by the visual fan and by the upper voice's playback rate.
    pub rpm_state_at_0x20: i32,
    pub rpm_excess_above_0x10000: i32,
    pub gain_envelope_at_0x24: i32,
    /// Exact next `FUN_0044C830/FUN_0044C8E0` rate argument for global 47.
    pub upper_rate_16_16: i32,
    /// Creation receives the envelope directly; an existing loop receives /6.
    pub upper_gain_argument_16_16: i32,
    /// Exact creation/update gain argument for the fixed-rate global-31 layer.
    pub lower_gain_argument_16_16: i32,
    pub lower_rate_16_16: i32,
    pub lower_voice_handle_at_entity_0x8c: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct FanDriveEvidence {
    pub tick_before: Option<u32>,
    pub tick_after: Option<u32>,
    pub tick_stable: Option<bool>,
    pub controller_entity_handle: Option<u32>,
    pub resource_cache_resolution: Option<ResourceCacheResolution>,
    pub entity_pointer: Option<u32>,
    pub component_root_pointer: Option<u32>,
    pub component_table_pointer: Option<u32>,
    pub fan_state_pointer: Option<u32>,
    pub fan_state_raw_hex: Option<String>,
    pub values: Option<FanDriveValues>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TargetVoiceObservation {
    pub global_sound: u16,
    pub pcm_pointer: u32,
    pub pcm_bytes: u32,
    pub voices: Vec<SoundRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FanAudioMemorySample {
    /// One means the first observation was stable; two means it was discarded
    /// and the complete controller/fan/audio sample was retried once.
    pub attempts: u32,
    pub prior_attempt_instability: Vec<String>,
    pub outer_tick_before: Option<u32>,
    pub outer_tick_after: Option<u32>,
    pub global_delta_us: Option<u32>,
    pub controller_motion: ControllerMotionEvidence,
    pub fan_drive: FanDriveEvidence,
    pub audio_snapshot: Option<SoundSnapshot>,
    pub audio_snapshot_error: Option<String>,
    pub target_voices: Vec<TargetVoiceObservation>,
    pub observationally_stable: bool,
    pub instability_reasons: Vec<String>,
}

pub fn read_target_resources(process: &Process) -> Result<Vec<SoundResource>, String> {
    let pools = crate::resources::read(process)?;
    let catalog = crate::resources::read_catalog(process)?;
    let slot_count = catalog.section_totals[11];
    let sound_catalog = crate::sound_resource::read(process, pools.sounds, slot_count)?;
    let mut targets = Vec::with_capacity(TARGET_GLOBAL_SOUNDS.len());
    for global_sound in TARGET_GLOBAL_SOUNDS {
        let resource = sound_catalog
            .resources
            .iter()
            .find(|resource| resource.global_id == global_sound)
            .ok_or_else(|| format!("live sound catalog has no global sound {global_sound}"))?;
        if resource.raw_type != 1
            || resource.resolved_pcm_global_id != Some(global_sound)
            || resource.pcm_pointer == 0
            || resource.pcm_bytes == 0
            || resource.pcm_fnv1a64.is_none()
        {
            return Err(format!(
                "global sound {global_sound} did not resolve to one hashable direct PCM resource: {resource:?}"
            ));
        }
        targets.push(resource.clone());
    }
    Ok(targets)
}

pub fn read_memory_sample(
    process: &Process,
    target_resources: &[SoundResource],
) -> FanAudioMemorySample {
    let first = read_memory_sample_once(process, target_resources);
    if first.observationally_stable {
        return first;
    }
    let first_instability = first.instability_reasons;
    let mut retry = read_memory_sample_once(process, target_resources);
    retry.attempts = 2;
    retry.prior_attempt_instability = first_instability;
    retry
}

pub fn read(process: &Process, controller_entity_handle: Option<u32>) -> FanDriveEvidence {
    let mut warnings = Vec::new();
    let tick_before = read_u32(
        process,
        TICK_50HZ,
        "50 Hz tick before fan read",
        &mut warnings,
    );
    let resource_cache_resolution = controller_entity_handle.and_then(|handle| {
        resolve_resource_handle(process, handle)
            .map_err(|error| warnings.push(error))
            .ok()
    });
    let entity_pointer = resource_cache_resolution
        .as_ref()
        .map(|resolution| resolution.object_pointer);

    let mut component_root_pointer = None;
    let mut component_table_pointer = None;
    let mut fan_state_pointer = None;
    let mut fan_state_raw_hex = None;
    let mut values = None;

    if let Some(entity_pointer) = entity_pointer {
        match process.read_bytes(entity_pointer as usize, ENTITY_WINDOW_BYTES) {
            Ok(entity) => {
                let entity_type = u32_at(&entity, ENTITY_TYPE_OFFSET);
                let stored_handle = u32_at(&entity, ENTITY_HANDLE_OFFSET);
                if entity_type != PLAYER_ENTITY_TYPE {
                    warnings.push(format!(
                        "controller handle resolved to entity type {entity_type}, expected {PLAYER_ENTITY_TYPE}"
                    ));
                } else if Some(stored_handle) != controller_entity_handle {
                    warnings.push(format!(
                        "resolved entity handle {stored_handle:08X} does not match controller handle {:08X}",
                        controller_entity_handle.unwrap_or_default()
                    ));
                } else {
                    let root = u32_at(&entity, ENTITY_COMPONENT_ROOT_OFFSET);
                    component_root_pointer =
                        plausible_pointer(root, "component root", &mut warnings);
                    if let Some(root) = component_root_pointer {
                        let table = read_pointer_at(
                            process,
                            root as usize + COMPONENT_TABLE_OFFSET,
                            "component table",
                            &mut warnings,
                        );
                        component_table_pointer = table;
                        if let Some(table) = table {
                            let state = read_pointer_at(
                                process,
                                table as usize + FAN_STATE_SLOT_OFFSET,
                                "fan Sub-O runtime state",
                                &mut warnings,
                            );
                            fan_state_pointer = state;
                            if let Some(state) = state {
                                match process.read_bytes(state as usize, FAN_STATE_BYTES) {
                                    Ok(state_bytes) => {
                                        let upper_handle =
                                            u32_at(&state_bytes, UPPER_FAN_HANDLE_OFFSET);
                                        let rpm_state = u32_at(&state_bytes, RPM_STATE_OFFSET) as i32;
                                        let envelope =
                                            u32_at(&state_bytes, GAIN_ENVELOPE_OFFSET) as i32;
                                        let lower_handle =
                                            u32_at(&entity, ENTITY_LOWER_FAN_HANDLE_OFFSET);
                                        fan_state_raw_hex = Some(encode_hex(&state_bytes));
                                        values = Some(derive_values(
                                            upper_handle,
                                            rpm_state,
                                            envelope,
                                            lower_handle,
                                        ));
                                    }
                                    Err(error) => warnings.push(format!(
                                        "could not read 0x{FAN_STATE_BYTES:X}-byte fan state at {state:08X}: {error}"
                                    )),
                                }
                            }
                        }
                    }
                }
            }
            Err(error) => warnings.push(format!(
                "could not read player entity window at {entity_pointer:08X}: {error}"
            )),
        }
    }

    let tick_after = read_u32(
        process,
        TICK_50HZ,
        "50 Hz tick after fan read",
        &mut warnings,
    );
    FanDriveEvidence {
        tick_before,
        tick_after,
        tick_stable: tick_before
            .zip(tick_after)
            .map(|(before, after)| before == after),
        controller_entity_handle,
        resource_cache_resolution,
        entity_pointer,
        component_root_pointer,
        component_table_pointer,
        fan_state_pointer,
        fan_state_raw_hex,
        values,
        warnings,
    }
}

fn read_memory_sample_once(
    process: &Process,
    target_resources: &[SoundResource],
) -> FanAudioMemorySample {
    let mut instability_reasons = Vec::new();
    let outer_tick_before = match process.read_u32(TICK_50HZ) {
        Ok(value) => Some(value),
        Err(error) => {
            instability_reasons.push(format!("could not read outer tick before sample: {error}"));
            None
        }
    };
    let controller_motion = crate::controller::read_motion_evidence(process, None);
    let controller_entity_handle = controller_motion
        .location
        .as_ref()
        .and_then(|location| location.entity_handle_at_0x68);
    let fan_drive = read(process, controller_entity_handle);
    let global_delta_us = match process.read_u32(GLOBAL_DELTA_US) {
        Ok(value) => Some(value),
        Err(error) => {
            instability_reasons.push(format!("could not read global delta-us: {error}"));
            None
        }
    };
    let (audio_snapshot, audio_snapshot_error) = match crate::audio::read_snapshot(process) {
        Ok(snapshot) => (Some(snapshot), None),
        Err(error) => {
            instability_reasons.push(format!("could not read active DirectSound list: {error}"));
            (None, Some(error))
        }
    };
    let outer_tick_after = match process.read_u32(TICK_50HZ) {
        Ok(value) => Some(value),
        Err(error) => {
            instability_reasons.push(format!("could not read outer tick after sample: {error}"));
            None
        }
    };

    if outer_tick_before
        .zip(outer_tick_after)
        .is_none_or(|(before, after)| before != after)
    {
        instability_reasons.push("outer 50-Hz tick changed or was unreadable".into());
    }
    if controller_motion.tick_stable != Some(true) {
        instability_reasons.push("controller read crossed a 50-Hz tick or was incomplete".into());
    }
    if controller_motion.motion_vector.is_none() || controller_entity_handle.is_none() {
        instability_reasons.push("controller motion vector or player handle is unavailable".into());
    }
    if fan_drive.tick_stable != Some(true) || fan_drive.values.is_none() {
        instability_reasons.push("fan Sub-O state crossed a 50-Hz tick or is unavailable".into());
    }
    if let Some(snapshot) = audio_snapshot.as_ref() {
        if !snapshot.topology_stable {
            instability_reasons.push("active-sound topology changed during traversal".into());
        }
        if snapshot.tick_before != snapshot.tick_after {
            instability_reasons.push("active-sound traversal crossed a 50-Hz tick".into());
        }
        if !snapshot.warnings.is_empty() {
            instability_reasons.push(format!(
                "active-sound traversal warnings: {}",
                snapshot.warnings.join("; ")
            ));
        }
    }

    let target_voices = target_resources
        .iter()
        .map(|resource| {
            let voices = voices_matching_pcm(
                audio_snapshot.as_ref(),
                resource.pcm_pointer,
                resource.pcm_bytes,
            );
            if voices.is_empty() {
                instability_reasons.push(format!(
                    "no active voice matches global sound {} PCM {:08X}/{}",
                    resource.global_id, resource.pcm_pointer, resource.pcm_bytes
                ));
            }
            TargetVoiceObservation {
                global_sound: resource.global_id,
                pcm_pointer: resource.pcm_pointer,
                pcm_bytes: resource.pcm_bytes,
                voices,
            }
        })
        .collect();

    FanAudioMemorySample {
        attempts: 1,
        prior_attempt_instability: Vec::new(),
        outer_tick_before,
        outer_tick_after,
        global_delta_us,
        controller_motion,
        fan_drive,
        audio_snapshot,
        audio_snapshot_error,
        target_voices,
        observationally_stable: instability_reasons.is_empty(),
        instability_reasons,
    }
}

fn derive_values(
    upper_handle: u32,
    rpm_state: i32,
    envelope: i32,
    lower_handle: u32,
) -> FanDriveValues {
    FanDriveValues {
        upper_voice_handle_at_0x1c: upper_handle,
        rpm_state_at_0x20: rpm_state,
        rpm_excess_above_0x10000: rpm_state.saturating_sub(0x10000).max(0),
        gain_envelope_at_0x24: envelope,
        upper_rate_16_16: rpm_state.saturating_mul(3) / 2,
        upper_gain_argument_16_16: if upper_handle == 0 {
            envelope
        } else {
            envelope / 6
        },
        lower_gain_argument_16_16: envelope / 2 + 0x2000,
        lower_rate_16_16: 0x8000,
        lower_voice_handle_at_entity_0x8c: lower_handle,
    }
}

fn voices_matching_pcm(
    snapshot: Option<&SoundSnapshot>,
    pcm_pointer: u32,
    pcm_bytes: u32,
) -> Vec<SoundRecord> {
    snapshot
        .map(|snapshot| {
            snapshot
                .records
                .iter()
                .filter(|voice| voice.pcm_pointer == pcm_pointer && voice.pcm_bytes == pcm_bytes)
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

fn read_pointer_at(
    process: &Process,
    address: usize,
    label: &str,
    warnings: &mut Vec<String>,
) -> Option<u32> {
    let pointer = read_u32(process, address, label, warnings)?;
    plausible_pointer(pointer, label, warnings)
}

fn plausible_pointer(pointer: u32, label: &str, warnings: &mut Vec<String>) -> Option<u32> {
    if plausible_heap_pointer(pointer as usize) {
        Some(pointer)
    } else {
        warnings.push(format!("{label} pointer {pointer:08X} is implausible"));
        None
    }
}

fn read_u32(
    process: &Process,
    address: usize,
    label: &str,
    warnings: &mut Vec<String>,
) -> Option<u32> {
    match process.read_u32(address) {
        Ok(value) => Some(value),
        Err(error) => {
            warnings.push(format!("could not read {label} at {address:08X}: {error}"));
            None
        }
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02X}");
    }
    output
}

#[cfg(test)]
mod tests {
    use crate::audio::{SoundRecord, SoundSnapshot};

    use super::{derive_values, voices_matching_pcm};

    #[test]
    fn fan_values_preserve_creation_and_update_gain_asymmetry() {
        let creating = derive_values(0, 0x10000, 0x2000, 0);
        assert_eq!(creating.upper_rate_16_16, 0x18000);
        assert_eq!(creating.upper_gain_argument_16_16, 0x2000);
        assert_eq!(creating.lower_gain_argument_16_16, 0x3000);

        let powered = derive_values(1, 0x18000, 0x10000, 2);
        assert_eq!(powered.rpm_excess_above_0x10000, 0x8000);
        assert_eq!(powered.upper_rate_16_16, 0x24000);
        assert_eq!(powered.upper_gain_argument_16_16, 0x10000 / 6);
        assert_eq!(powered.lower_gain_argument_16_16, 0xA000);
        assert_eq!(powered.lower_rate_16_16, 0x8000);
    }

    #[test]
    fn target_voice_matching_uses_pcm_pointer_and_length_together() {
        fn voice(pointer: u32, pcm_pointer: u32, pcm_bytes: u32) -> SoundRecord {
            SoundRecord {
                pointer,
                next: 0,
                previous_link: 0,
                volume_16_16: 0,
                frequency_16_16: 0,
                frequency_hz: 0,
                pan: 0,
                aux_param: 0,
                resolved_section11_type: 1,
                pcm_pointer,
                pcm_bytes,
                shared_buffer: 0,
                reserved_entry_word: 0,
                play_or_ref_state: 0,
                directsound_play_flags: 1,
                auto_gc: 0,
                duplicate_buffer: 0,
            }
        }

        let snapshot = SoundSnapshot {
            tick_before: 10,
            tick_after: 10,
            head_before: 1,
            head_after: 1,
            tail_before: 2,
            tail_after: 2,
            topology_stable: true,
            master_volume_16_16: 0x10000,
            records: vec![voice(1, 0x1000, 19014), voice(2, 0x2000, 19014)],
            warnings: Vec::new(),
        };

        let matched = voices_matching_pcm(Some(&snapshot), 0x2000, 19014);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].pointer, 2);
    }
}
