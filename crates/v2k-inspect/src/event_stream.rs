//! Reusable lifecycle reducers for the retail particle and DirectSound pools.
//!
//! Scenario captures deliberately share these reducers.  The data shape is
//! stable while each wrapper remains free to choose its own subject and
//! sampling cadence.

use std::collections::HashMap;
use std::io::Write;

use serde::Serialize;
use serde_json::json;

use crate::audio::{SoundRecord, SoundSignature};
use crate::particle::ParticleRecord;
use crate::process::Process;

pub(crate) struct ParticleTimelineState {
    previous: Option<Vec<Option<ParticleRecord>>>,
    unstable_samples_since_previous: u32,
    retain_active_samples: bool,
}

impl ParticleTimelineState {
    pub(crate) fn new(retain_active_samples: bool) -> Self {
        Self {
            previous: None,
            unstable_samples_since_previous: 0,
            retain_active_samples,
        }
    }

    pub(crate) fn sample(
        &mut self,
        process: &Process,
        writer: &mut impl Write,
        sample: u64,
        elapsed_ms: f64,
    ) -> Result<(), String> {
        let snapshot = match crate::particle::read_snapshot(process, 3) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.unstable_samples_since_previous += 1;
                write_json_line(
                    writer,
                    &json!({
                        "record_kind": "particle_pool_error",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "error": error,
                    }),
                )?;
                return Ok(());
            }
        };
        if !snapshot.observationally_stable {
            self.unstable_samples_since_previous += 1;
            write_json_line(
                writer,
                &json!({
                    "record_kind": "particle_pool_unstable",
                    "sample": sample,
                    "elapsed_ms": elapsed_ms,
                    "tick_before": snapshot.tick_before,
                    "tick_after": snapshot.tick_after,
                    "attempts": snapshot.attempts,
                    "duplicate_reads_identical": snapshot.duplicate_reads_identical,
                    "topology_valid": snapshot.topology_valid,
                    "topology_errors": snapshot.topology_errors,
                }),
            )?;
            return Ok(());
        }

        let mut current = vec![None; crate::particle::PARTICLE_CAPACITY];
        for record in snapshot.records {
            let slot = record.slot;
            current[slot] = Some(record);
        }
        if self.previous.is_none() {
            let active = current
                .iter()
                .filter_map(Option::as_ref)
                .collect::<Vec<_>>();
            write_json_line(
                writer,
                &json!({
                    "record_kind": "particle_pool_baseline",
                    "sample": sample,
                    "elapsed_ms": elapsed_ms,
                    "tick": snapshot.tick_after,
                    "attempts": snapshot.attempts,
                    "unstable_samples_before_baseline": self.unstable_samples_since_previous,
                    "active": active,
                }),
            )?;
            self.previous = Some(current);
            self.unstable_samples_since_previous = 0;
            return Ok(());
        }

        // The 50-Hz clock advances before particle processing. Two distinct
        // stable pool states can therefore share a tick and must be compared
        // by decoded contents rather than by clock value.
        if self.retain_active_samples
            && self
                .previous
                .as_ref()
                .is_some_and(|before| before != &current)
        {
            let active = current
                .iter()
                .filter_map(Option::as_ref)
                .collect::<Vec<_>>();
            write_json_line(
                writer,
                &json!({
                    "record_kind": "particle_pool_sample",
                    "sample": sample,
                    "elapsed_ms": elapsed_ms,
                    "tick": snapshot.tick_after,
                    "attempts": snapshot.attempts,
                    "unstable_samples_since_previous": self.unstable_samples_since_previous,
                    "active": active,
                }),
            )?;
        }

        if self.unstable_samples_since_previous != 0 {
            write_json_line(
                writer,
                &json!({
                    "record_kind": "particle_pool_gap_closed",
                    "sample": sample,
                    "elapsed_ms": elapsed_ms,
                    "tick": snapshot.tick_after,
                    "unstable_samples": self.unstable_samples_since_previous,
                    "active_count": current.iter().filter(|record| record.is_some()).count(),
                }),
            )?;
        }
        let before = self.previous.as_ref().expect("particle baseline exists");
        for slot in 0..crate::particle::PARTICLE_CAPACITY {
            match (&before[slot], &current[slot]) {
                (None, Some(record)) => write_particle_event(
                    writer,
                    "birth",
                    sample,
                    elapsed_ms,
                    snapshot.tick_after,
                    slot,
                    self.unstable_samples_since_previous,
                    None,
                    Some(record),
                )?,
                (Some(record), None) => write_particle_event(
                    writer,
                    "death",
                    sample,
                    elapsed_ms,
                    snapshot.tick_after,
                    slot,
                    self.unstable_samples_since_previous,
                    Some(record),
                    None,
                )?,
                (Some(old), Some(new)) if crate::particle::generation_changed(old, new) => {
                    write_particle_event(
                        writer,
                        "recycle",
                        sample,
                        elapsed_ms,
                        snapshot.tick_after,
                        slot,
                        self.unstable_samples_since_previous,
                        Some(old),
                        Some(new),
                    )?
                }
                (Some(old), Some(new))
                    if old.source_entity_handle != new.source_entity_handle
                        || old.source_entity_type != new.source_entity_type =>
                {
                    write_particle_event(
                        writer,
                        "source_change",
                        sample,
                        elapsed_ms,
                        snapshot.tick_after,
                        slot,
                        self.unstable_samples_since_previous,
                        Some(old),
                        Some(new),
                    )?
                }
                _ => {}
            }
        }
        self.previous = Some(current);
        self.unstable_samples_since_previous = 0;
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct AudioTimelineState {
    previous: HashMap<u32, SoundSignature>,
    previous_warnings: Vec<String>,
}

impl AudioTimelineState {
    pub(crate) fn sample(
        &mut self,
        process: &Process,
        writer: &mut impl Write,
        sample: u64,
        elapsed_ms: f64,
    ) -> Result<(), String> {
        let mut snapshot = match crate::audio::read_snapshot(process) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                write_json_line(
                    writer,
                    &json!({
                        "record_kind": "audio_snapshot_error",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "error": error,
                    }),
                )?;
                return Ok(());
            }
        };
        if !snapshot.topology_stable {
            if let Ok(retry) = crate::audio::read_snapshot(process) {
                snapshot = retry;
            }
        }
        if snapshot.warnings != self.previous_warnings {
            if !snapshot.warnings.is_empty() {
                write_json_line(
                    writer,
                    &json!({
                        "record_kind": "audio_snapshot_warning",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "tick_before": snapshot.tick_before,
                        "tick_after": snapshot.tick_after,
                        "topology_stable": snapshot.topology_stable,
                        "warnings": &snapshot.warnings,
                    }),
                )?;
            }
            self.previous_warnings = snapshot.warnings.clone();
        }
        if !snapshot.topology_stable {
            write_json_line(
                writer,
                &json!({
                    "record_kind": "audio_snapshot_unstable",
                    "sample": sample,
                    "elapsed_ms": elapsed_ms,
                    "tick_before": snapshot.tick_before,
                    "tick_after": snapshot.tick_after,
                    "head_before": snapshot.head_before,
                    "head_after": snapshot.head_after,
                    "tail_before": snapshot.tail_before,
                    "tail_after": snapshot.tail_after,
                    "warnings": &snapshot.warnings,
                }),
            )?;
            return Ok(());
        }

        let current: HashMap<u32, SoundSignature> = snapshot
            .records
            .iter()
            .map(|record| (record.pointer, record.signature()))
            .collect();
        for record in &snapshot.records {
            match self.previous.get(&record.pointer) {
                None => write_sound_event(
                    writer,
                    "start",
                    sample,
                    elapsed_ms,
                    snapshot.tick_after,
                    snapshot.master_volume_16_16,
                    record.pointer,
                    None,
                    Some(record),
                )?,
                Some(before) if before != &record.signature() => write_sound_event(
                    writer,
                    "update",
                    sample,
                    elapsed_ms,
                    snapshot.tick_after,
                    snapshot.master_volume_16_16,
                    record.pointer,
                    Some(before),
                    Some(record),
                )?,
                _ => {}
            }
        }
        for (pointer, before) in &self.previous {
            if !current.contains_key(pointer) {
                write_sound_event(
                    writer,
                    "stop",
                    sample,
                    elapsed_ms,
                    snapshot.tick_after,
                    snapshot.master_volume_16_16,
                    *pointer,
                    Some(before),
                    None,
                )?;
            }
        }
        self.previous = current;
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn write_particle_event(
    writer: &mut impl Write,
    event: &str,
    sample: u64,
    elapsed_ms: f64,
    tick: u32,
    slot: usize,
    unstable_samples_since_previous: u32,
    previous: Option<&ParticleRecord>,
    particle: Option<&ParticleRecord>,
) -> Result<(), String> {
    write_json_line(
        writer,
        &json!({
            "record_kind": "particle_event",
            "event": event,
            "sample": sample,
            "elapsed_ms": elapsed_ms,
            "tick": tick,
            "slot": slot,
            "unstable_samples_since_previous": unstable_samples_since_previous,
            "previous": previous,
            "particle": particle,
        }),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn write_sound_event(
    writer: &mut impl Write,
    event: &str,
    sample: u64,
    elapsed_ms: f64,
    tick: u32,
    master_volume_16_16: u32,
    pointer: u32,
    previous: Option<&SoundSignature>,
    sound: Option<&SoundRecord>,
) -> Result<(), String> {
    write_json_line(
        writer,
        &json!({
            "record_kind": "audio_event",
            "event": event,
            "sample": sample,
            "elapsed_ms": elapsed_ms,
            "tick": tick,
            "master_volume_16_16": master_volume_16_16,
            "pointer": pointer,
            "previous": previous,
            "sound": sound,
        }),
    )
}

fn write_json_line(writer: &mut impl Write, value: &impl Serialize) -> Result<(), String> {
    serde_json::to_writer(&mut *writer, value).map_err(|error| error.to_string())?;
    writeln!(writer).map_err(|error| error.to_string())
}
