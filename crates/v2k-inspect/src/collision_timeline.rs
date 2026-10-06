//! Compact collision/health timeline orchestration.
//!
//! Keeping the high-rate sampling loop beside its evidence schema prevents the
//! CLI dispatcher from accumulating capture-specific state machines. The
//! retail process remains strictly read-only throughout this module.

use std::fs::{create_dir_all, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::json;

use crate::process::{BuildFingerprint, Process};

#[derive(Serialize)]
struct CaptureMeta<'a> {
    record_kind: &'static str,
    tool_version: &'static str,
    command: &'a str,
    process_id: u32,
    executable: &'a str,
    build: &'a BuildFingerprint,
}

pub fn capture(process: &Process, seconds: f64, hz: u32, output: &Path) -> Result<(), String> {
    validate_timeline(seconds, hz, 200)?;
    let mut writer = create_writer(output)?;
    let context_stride = ((hz as f64 / 20.0).round() as u64).max(1);
    let context_hz_approx = hz as f64 / context_stride as f64;
    write_json_line(
        &mut writer,
        &json!({
            "record_kind": "capture_meta",
            "meta": capture_meta(process, "collision-health-timeline"),
            "seconds": seconds,
            "hz": hz,
            "nearby_context_hz_approx": context_hz_approx,
            "foreground_key_source": "Win32 GetAsyncKeyState high bit while the attached V2000 process owns the foreground window; correlation evidence only",
            "authoritative_hull_health": "signed i32 at type-46 player entity +0x30",
            "hull_health_mirror": "signed i32 at validated controller +0x8C; FUN_00443440 mirrors entity +0x30",
            "visible_trailing_hull_health": "signed i32 at absolute retail global 0x004DB1C8",
            "pre_health_buffer": "signed i32 at entity +0x50; drained before hull health and not drawn as the hull bar",
            "normalization_policy": "Q16 and percent-x100 values are inspector comparisons against the proven type-46 initial/full value 40000, not remote retail fields",
            "stop_policy": "only after a stable entity snapshot has observed type 46, a later stable snapshot has no type 46, and the exact stable retail main ring persists for at least 1000 ms",
            "process_unreadable_policy": "retain the partial JSONL and emit collision_capture_end instead of failing the command",
        }),
    )?;
    writer.flush().map_err(|error| error.to_string())?;

    let total_samples = (seconds * hz as f64).ceil().max(1.0) as u64;
    let interval = Duration::from_secs_f64(1.0 / hz as f64);
    let start = Instant::now();
    let mut last_flush = start;
    let mut previous_input: Option<crate::foreground_input::HoverInputState> = None;
    let mut previous_presence: Option<bool> = None;
    let mut player_seen = false;
    let mut stable_main_ring_since_ms = None;
    let mut last_stable_main_ring_observed_ms = None;
    let mut last_menu_phase = None;
    let mut samples_completed = 0u64;
    let mut end_reason = "safety_timeout";

    for sample in 0..total_samples {
        let target = start + interval.mul_f64(sample as f64);
        let now = Instant::now();
        if target > now {
            sleep(target - now);
        }
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        let process_read_error = match process.read_bytes(crate::process::RETAIL_IMAGE_BASE, 2) {
            Ok(bytes) if bytes == b"MZ" => None,
            Ok(_) => Some("retail image no longer has its DOS MZ header".to_owned()),
            Err(error) => Some(error),
        };
        if let Some(error) = process_read_error {
            write_json_line(
                &mut writer,
                &json!({
                    "record_kind": "collision_sample",
                    "sample": sample,
                    "elapsed_ms": elapsed_ms,
                    "process_read_error": error,
                }),
            )?;
            samples_completed = sample + 1;
            end_reason = "process_unreadable";
            break;
        }

        let foreground_input = crate::foreground_input::poll_hover_foreground(process.process_id);
        let foreground_input_edges = match (previous_input, foreground_input) {
            (Some(before), Some(current)) => current.edges_from(before),
            _ => Vec::new(),
        };
        previous_input = foreground_input;
        let mut reached_stable_main_ring = false;

        match crate::entity::read_snapshot(process) {
            Ok(snapshot) => {
                let player = snapshot
                    .records
                    .iter()
                    .find(|record| record.entity_type == 46);
                let snapshot_stable = snapshot.tick_stable && snapshot.topology_stable;
                if snapshot_stable {
                    let present = player.is_some();
                    if present {
                        player_seen = true;
                    }
                    if previous_presence != Some(present) {
                        write_json_line(
                            &mut writer,
                            &json!({
                                "record_kind": "player_presence",
                                "sample": sample,
                                "elapsed_ms": elapsed_ms,
                                "present": present,
                                "player_seen_before_or_now": player_seen,
                                "entity_pointer": player.map(|record| record.pointer),
                                "entity_handle": player.map(|record| record.handle),
                            }),
                        )?;
                        previous_presence = Some(present);
                    }
                }

                let stable_player_absent = snapshot_stable && player.is_none();
                let (menu_phase, menu_phase_error) = if player_seen && stable_player_absent {
                    match crate::menu::read_phase_snapshot(process) {
                        Ok(phase) => (Some(phase), None),
                        Err(error) => (None, Some(error)),
                    }
                } else {
                    (None, None)
                };
                if let Some(phase) = &menu_phase {
                    last_menu_phase = Some(phase.clone());
                }
                let stable_main_ring = menu_phase
                    .as_ref()
                    .is_some_and(|phase| phase.stable_main_ring);
                last_stable_main_ring_observed_ms = stable_main_ring.then_some(elapsed_ms);
                reached_stable_main_ring = update_return_to_menu_stability(
                    player_seen,
                    stable_player_absent,
                    stable_main_ring,
                    elapsed_ms,
                    &mut stable_main_ring_since_ms,
                );

                let collision_health =
                    player.map(|player| crate::collision_health::read(process, player));
                let context_sampled = sample % context_stride == 0;
                let nearby_dynamic_entities = context_sampled
                    .then(|| {
                        player
                            .map(|player| crate::entity::nearby_entities(player, &snapshot.records))
                    })
                    .flatten();
                let nearby_static_collision_candidates = context_sampled
                    .then(|| {
                        player.map(|player| {
                            crate::physics::read_nearby_static_collision_candidates(process, player)
                        })
                    })
                    .flatten();

                write_json_line(
                    &mut writer,
                    &json!({
                        "record_kind": "collision_sample",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "entity_tick_before": snapshot.tick_before,
                        "entity_tick_after": snapshot.tick_after,
                        "entity_tick_stable": snapshot.tick_stable,
                        "entity_topology_stable": snapshot.topology_stable,
                        "v2000_foreground": foreground_input.is_some(),
                        "foreground_input_state": foreground_input,
                        "foreground_input_edges": foreground_input_edges,
                        "collision_health": collision_health,
                        "nearby_context_sampled": context_sampled,
                        "nearby_dynamic_entities": nearby_dynamic_entities,
                        "nearby_static_collision_candidates": nearby_static_collision_candidates,
                        "menu_phase": menu_phase,
                        "menu_phase_error": menu_phase_error,
                        "entity_warnings": snapshot.warnings,
                    }),
                )?;
            }
            Err(error) => {
                stable_main_ring_since_ms = None;
                last_stable_main_ring_observed_ms = None;
                write_json_line(
                    &mut writer,
                    &json!({
                        "record_kind": "collision_sample",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "v2000_foreground": foreground_input.is_some(),
                        "foreground_input_state": foreground_input,
                        "foreground_input_edges": foreground_input_edges,
                        "entity_snapshot_error": error,
                    }),
                )?;
            }
        }

        samples_completed = sample + 1;
        if last_flush.elapsed() >= Duration::from_secs(1) {
            writer.flush().map_err(|error| error.to_string())?;
            last_flush = Instant::now();
        }
        if reached_stable_main_ring {
            end_reason = "return_to_main_menu_stable";
            break;
        }
    }

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let stable_main_ring_duration_ms = stable_main_ring_since_ms
        .zip(last_stable_main_ring_observed_ms)
        .map(|(since, observed)| (observed - since).max(0.0));
    write_json_line(
        &mut writer,
        &json!({
            "record_kind": "collision_capture_end",
            "reason": end_reason,
            "samples_completed": samples_completed,
            "samples_requested": total_samples,
            "elapsed_ms": elapsed_ms,
            "player_seen": player_seen,
            "last_stable_player_present": previous_presence,
            "stable_main_ring_duration_ms": stable_main_ring_duration_ms,
            "last_menu_phase": last_menu_phase,
        }),
    )?;
    writer.flush().map_err(|error| error.to_string())?;
    eprintln!("wrote collision/health timeline to {}", output.display());
    Ok(())
}

fn update_return_to_menu_stability(
    player_seen: bool,
    stable_player_absent: bool,
    stable_main_ring: bool,
    elapsed_ms: f64,
    stable_since_ms: &mut Option<f64>,
) -> bool {
    if !(player_seen && stable_player_absent && stable_main_ring) {
        *stable_since_ms = None;
        return false;
    }
    let since = *stable_since_ms.get_or_insert(elapsed_ms);
    elapsed_ms - since >= 1_000.0
}

fn capture_meta<'a>(process: &'a Process, command: &'a str) -> CaptureMeta<'a> {
    CaptureMeta {
        record_kind: "capture_meta",
        tool_version: env!("CARGO_PKG_VERSION"),
        command,
        process_id: process.process_id,
        executable: &process.exe_name,
        build: &process.build,
    }
}

fn create_writer(path: &Path) -> Result<BufWriter<File>, String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            create_dir_all(parent).map_err(|error| {
                format!(
                    "could not create output directory {}: {error}",
                    parent.display()
                )
            })?;
        }
    }
    File::create(path)
        .map(BufWriter::new)
        .map_err(|error| format!("could not create {}: {error}", path.display()))
}

fn write_json_line(writer: &mut impl Write, value: &impl Serialize) -> Result<(), String> {
    serde_json::to_writer(&mut *writer, value).map_err(|error| error.to_string())?;
    writeln!(writer).map_err(|error| error.to_string())
}

fn validate_timeline(seconds: f64, hz: u32, maximum_hz: u32) -> Result<(), String> {
    if !seconds.is_finite() || !(0.05..=3600.0).contains(&seconds) {
        return Err("--seconds must be between 0.05 and 3600".into());
    }
    if !(1..=maximum_hz).contains(&hz) {
        return Err(format!("--hz must be between 1 and {maximum_hz}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::update_return_to_menu_stability;

    #[test]
    fn requires_seen_player_then_one_second_of_exact_menu_stability() {
        let mut since = None;
        assert!(!update_return_to_menu_stability(
            false, true, true, 100.0, &mut since
        ));
        assert_eq!(since, None);
        assert!(!update_return_to_menu_stability(
            true, true, true, 200.0, &mut since
        ));
        assert_eq!(since, Some(200.0));
        assert!(!update_return_to_menu_stability(
            true, true, false, 700.0, &mut since
        ));
        assert_eq!(since, None);
        assert!(!update_return_to_menu_stability(
            true, true, true, 900.0, &mut since
        ));
        assert!(update_return_to_menu_stability(
            true, true, true, 1_900.0, &mut since
        ));
    }
}
