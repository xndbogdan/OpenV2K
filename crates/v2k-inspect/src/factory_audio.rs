//! Passive Working Factory manufacturing-audio timeline.
//!
//! Retail observation: a positional loop plays only during type-66 phase 0
//! (manufacture) after the factory is staffed. Type 66 authors no constructor
//! `+0xB4` attachment, and `FUN_00419010` phase 0 does not submit a cue.
//! Type-61's `sound_044` loop cannot be that voice: the product is allocated
//! only when manufacture ends.
//!
//! This inspector records every live DirectSound voice plus the entity `+0x8C`
//! logical-handle join for the factory, its product, the player, Main Base,
//! and nearby type-8/93 actors. It does not start, stop, or retune voices.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use serde::Serialize;
use serde_json::json;

use crate::entity::{EntityRecord, EntitySnapshot};
use crate::event_stream::AudioTimelineState;
use crate::process::{plausible_heap_pointer, u32_at, Process};
use crate::timeline::{ensure_distinct_capture_paths, ensure_stop_file_absent, run_stop_aware};

const FACTORY_ENTITY_TYPE: u32 = 66;
const PLAYER_ENTITY_TYPE: u32 = 46;
const POWER_UP_ENTITY_TYPE: u32 = 61;
const MAIN_BASE_ENTITY_TYPE: u32 = 6;
const SCIENTIST_ENTITY_TYPE: u32 = 8;
const MATERIALISER_ENTITY_TYPE: u32 = 93;
const FACTORY_STATUS_BYTES: usize = 0xB8;
const COMPONENT_ROOT_BYTES: usize = 0x10;
const COMPONENT_TABLE_BYTES: usize = 0x34;

const FOCUSED_ENTITY_TYPES: [u32; 6] = [
    MAIN_BASE_ENTITY_TYPE,
    SCIENTIST_ENTITY_TYPE,
    PLAYER_ENTITY_TYPE,
    POWER_UP_ENTITY_TYPE,
    FACTORY_ENTITY_TYPE,
    MATERIALISER_ENTITY_TYPE,
];

#[derive(Debug, Clone, Serialize)]
struct CaptureMeta<'a> {
    record_kind: &'static str,
    tool_version: &'static str,
    command: &'static str,
    process_id: u32,
    executable: &'a str,
    build: &'a crate::process::BuildFingerprint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
struct FactoryStatusSnapshot {
    state_pointer: u32,
    phase_at_0xb0: u32,
    production_progress_micros_at_0x60: i32,
    production_threshold_micros_at_0x08: i32,
    current_scientists_at_0x68: i32,
    remaining_stock_at_0x58: i32,
    required_scientists_at_0x04: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct VoiceOwner {
    entity_pointer: u32,
    entity_type: u32,
    handle: u32,
    sound_attachment_handle_at_0x8c: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct FactoryAudioCensus {
    factory_handle: Option<u32>,
    factory_status: Option<FactoryStatusSnapshot>,
    type61_count: usize,
    owners: Vec<VoiceOwner>,
}

pub fn capture(
    process: &Process,
    seconds: f64,
    hz: u32,
    stop_file: Option<&Path>,
    output: &Path,
) -> Result<(), String> {
    if !seconds.is_finite() || !(1.0..=120.0).contains(&seconds) {
        return Err("--seconds must be between 1 and 120".into());
    }
    if !(1..=200).contains(&hz) {
        return Err("--hz must be between 1 and 200".into());
    }
    ensure_distinct_capture_paths(stop_file, output)?;
    ensure_stop_file_absent(stop_file)?;

    let mut writer = create_writer(output)?;
    write_json_line(
        &mut writer,
        &json!({
            "meta": CaptureMeta {
                record_kind: "capture_meta",
                tool_version: env!("CARGO_PKG_VERSION"),
                command: "factory-manufacturing-audio",
                process_id: process.process_id,
                executable: &process.exe_name,
                build: &process.build,
            },
            "seconds": seconds,
            "hz": hz,
            "stop_file": stop_file.map(|path| path.display().to_string()),
            "stop_file_policy": stop_file.map(|_| json!({
                "ownership": "external; the inspector never creates, removes, or modifies the marker",
                "startup": "refuse capture if the marker already exists",
                "polling": "check after each sample deadline and stop before taking the next sample when the marker exists",
                "read_error": "abort capture rather than silently ignore an unreadable marker path",
            })),
            "focused_entity_types": FOCUSED_ENTITY_TYPES,
            "sample_order": "tick-bracketed entity list, factory status through entity+0x4C -> root+0x0C -> table+0x30, live DirectSound list, +0x8C handle join",
            "voice_join": "entity +0x8C is FUN_004104B0's retained logical voice; player type 46 also uses it as the lower fan handle",
            "read_only": true,
        }),
    )?;
    writer.flush().map_err(|error| error.to_string())?;

    let mut audio = AudioTimelineState::default();
    let mut previous_census: Option<FactoryAudioCensus> = None;
    let outcome = run_stop_aware(seconds, hz, stop_file, |sample, elapsed_ms| {
        let entities = match crate::entity::read_snapshot(process) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                write_json_line(
                    &mut writer,
                    &json!({
                        "record_kind": "factory_audio_entity_error",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "error": error,
                    }),
                )?;
                audio.sample(process, &mut writer, sample, elapsed_ms)?;
                return Ok(());
            }
        };
        let census = census_from_entities(process, &entities);
        if previous_census.as_ref() != Some(&census) {
            write_json_line(
                &mut writer,
                &json!({
                    "record_kind": "factory_audio_census",
                    "sample": sample,
                    "elapsed_ms": elapsed_ms,
                    "tick_after": entities.tick_after,
                    "entity_list_stable": entities.tick_stable && entities.topology_stable,
                    "census": &census,
                }),
            )?;
            previous_census = Some(census);
        }
        audio.sample(process, &mut writer, sample, elapsed_ms)
    })?;

    write_json_line(
        &mut writer,
        &json!({
            "record_kind": "timeline_end",
            "stop_reason": outcome.reason.as_str(),
            "samples_completed": outcome.samples_completed,
            "samples_requested": outcome.samples_requested,
            "elapsed_ms": outcome.elapsed_ms,
        }),
    )?;
    writer.flush().map_err(|error| error.to_string())?;
    eprintln!("wrote factory manufacturing audio to {}", output.display());
    Ok(())
}

fn census_from_entities(process: &Process, snapshot: &EntitySnapshot) -> FactoryAudioCensus {
    let mut owners = Vec::new();
    let mut factory_handle = None;
    let mut factory_status = None;
    let mut type61_count = 0;
    for record in &snapshot.records {
        if record.entity_type == POWER_UP_ENTITY_TYPE {
            type61_count += 1;
        }
        if !FOCUSED_ENTITY_TYPES.contains(&record.entity_type) {
            continue;
        }
        owners.push(VoiceOwner {
            entity_pointer: record.pointer,
            entity_type: record.entity_type,
            handle: record.handle,
            sound_attachment_handle_at_0x8c: record.sound_attachment_handle_at_0x8c,
        });
        if record.entity_type == FACTORY_ENTITY_TYPE && factory_status.is_none() {
            factory_handle = Some(record.handle);
            factory_status = read_factory_status(process, record);
        }
    }
    owners.sort_by_key(|owner| (owner.entity_type, owner.handle));
    FactoryAudioCensus {
        factory_handle,
        factory_status,
        type61_count,
        owners,
    }
}

fn read_factory_status(process: &Process, factory: &EntityRecord) -> Option<FactoryStatusSnapshot> {
    if !plausible_heap_pointer(factory.component_root as usize) {
        return None;
    }
    let root = process
        .read_bytes(factory.component_root as usize, COMPONENT_ROOT_BYTES)
        .ok()?;
    let table_pointer = u32_at(&root, 0x0C);
    if !plausible_heap_pointer(table_pointer as usize) {
        return None;
    }
    let table = process
        .read_bytes(table_pointer as usize, COMPONENT_TABLE_BYTES)
        .ok()?;
    let state_pointer = u32_at(&table, 0x30);
    if !plausible_heap_pointer(state_pointer as usize) {
        return None;
    }
    let state = process
        .read_bytes(state_pointer as usize, FACTORY_STATUS_BYTES)
        .ok()?;
    Some(decode_factory_status(state_pointer, &state)?)
}

fn decode_factory_status(state_pointer: u32, state: &[u8]) -> Option<FactoryStatusSnapshot> {
    if state.len() < FACTORY_STATUS_BYTES {
        return None;
    }
    Some(FactoryStatusSnapshot {
        state_pointer,
        phase_at_0xb0: u32_at(state, 0xB0),
        production_progress_micros_at_0x60: u32_at(state, 0x60) as i32,
        production_threshold_micros_at_0x08: u32_at(state, 0x08) as i32,
        current_scientists_at_0x68: u32_at(state, 0x68) as i32,
        remaining_stock_at_0x58: u32_at(state, 0x58) as i32,
        required_scientists_at_0x04: u32_at(state, 0x04) as i32,
    })
}

fn create_writer(path: &Path) -> Result<BufWriter<File>, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    File::create(path)
        .map(BufWriter::new)
        .map_err(|error| error.to_string())
}

fn write_json_line(writer: &mut impl Write, value: &impl Serialize) -> Result<(), String> {
    serde_json::to_writer(&mut *writer, value).map_err(|error| error.to_string())?;
    writeln!(writer).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn factory_status_decode_uses_the_0xb8_production_block() {
        let mut state = vec![0_u8; FACTORY_STATUS_BYTES];
        state[0x04..0x08].copy_from_slice(&2_i32.to_le_bytes());
        state[0x08..0x0C].copy_from_slice(&6_000_000_i32.to_le_bytes());
        state[0x58..0x5C].copy_from_slice(&1_i32.to_le_bytes());
        state[0x60..0x64].copy_from_slice(&1_500_000_i32.to_le_bytes());
        state[0x68..0x6C].copy_from_slice(&2_i32.to_le_bytes());
        state[0xB0..0xB4].copy_from_slice(&0_u32.to_le_bytes());
        let decoded = decode_factory_status(0x0BAD_F00D, &state).expect("full block");
        assert_eq!(decoded.phase_at_0xb0, 0);
        assert_eq!(decoded.required_scientists_at_0x04, 2);
        assert_eq!(decoded.production_threshold_micros_at_0x08, 6_000_000);
        assert_eq!(decoded.remaining_stock_at_0x58, 1);
        assert_eq!(decoded.production_progress_micros_at_0x60, 1_500_000);
        assert_eq!(decoded.current_scientists_at_0x68, 2);
    }

    #[test]
    fn focused_type_set_is_the_factory_cycle_cast() {
        let types = FOCUSED_ENTITY_TYPES.into_iter().collect::<BTreeSet<_>>();
        assert_eq!(types, BTreeSet::from([6, 8, 46, 61, 66, 93]));
    }
}
