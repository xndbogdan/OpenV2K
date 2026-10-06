//! Particle presentation-gate evidence paired with the complete live pool.

use std::fs::{create_dir_all, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use serde::Serialize;
use serde_json::json;

use crate::event_stream::ParticleTimelineState;
use crate::process::{BuildFingerprint, Process};
use crate::timeline::{ensure_distinct_capture_paths, ensure_stop_file_absent, run_stop_aware};

const PRESENTATION_CONTEXT_ADDRESS: usize = 0x004D_04E8;
const PRESENTATION_CONTEXT_BYTES: usize = 0x80;
const ACTIVE_PROJECTION_ADDRESS: usize = 0x004F_EED0;
const ACTIVE_PROJECTION_BYTES: usize = 0x24;

#[derive(Serialize)]
struct CaptureMeta<'a> {
    record_kind: &'static str,
    tool_version: &'static str,
    command: &'static str,
    process_id: u32,
    executable: &'a str,
    build: &'a BuildFingerprint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct PresentationContext {
    tick_before: u32,
    tick_after: u32,
    duplicate_stable: bool,
    context_address: u32,
    context_raw_hex: String,
    active_projection_address: u32,
    active_projection_raw_hex: String,
}

pub fn capture(
    process: &Process,
    scene: &str,
    seconds: f64,
    hz: u32,
    stop_file: Option<&Path>,
    output: &Path,
) -> Result<(), String> {
    if !seconds.is_finite() || !(0.05..=3600.0).contains(&seconds) {
        return Err("--seconds must be between 0.05 and 3600".into());
    }
    if !(1..=500).contains(&hz) {
        return Err("--hz must be between 1 and 500".into());
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
                command: "particle-render-gate-timeline",
                process_id: process.process_id,
                executable: &process.exe_name,
                build: &process.build,
            },
            "scene": scene,
            "seconds": seconds,
            "hz": hz,
            "stop_file": stop_file.map(|path| path.display().to_string()),
            "context_window": {
                "address": PRESENTATION_CONTEXT_ADDRESS as u32,
                "bytes": PRESENTATION_CONTEXT_BYTES,
                "notable_offsets": {
                    "display_center_x": "0x5C",
                    "display_center_y": "0x60",
                    "fog_near": "0x74",
                    "fog_far": "0x78"
                }
            },
            "active_projection_window": {
                "address": ACTIVE_PROJECTION_ADDRESS as u32,
                "bytes": ACTIVE_PROJECTION_BYTES,
                "range": "0x004FEED0..0x004FEEF3"
            },
            "sample_order": "tick, duplicate raw presentation windows, decoded render/camera snapshot, duplicate-read complete particle pool",
            "read_only": true,
        }),
    )?;
    writer.flush().map_err(|error| error.to_string())?;

    let mut particles = ParticleTimelineState::new(true);
    let outcome = run_stop_aware(seconds, hz, stop_file, |sample, elapsed_ms| {
        let context = read_presentation_context(process)?;
        let render = crate::render::read_snapshot(process);
        write_json_line(
            &mut writer,
            &json!({
                "record_kind": "particle_presentation_context",
                "sample": sample,
                "elapsed_ms": elapsed_ms,
                "context": context,
                "render_snapshot": render.as_ref().ok(),
                "render_error": render.as_ref().err(),
            }),
        )?;
        particles.sample(process, &mut writer, sample, elapsed_ms)?;
        if sample % u64::from(hz) == 0 {
            writer.flush().map_err(|error| error.to_string())?;
        }
        Ok(())
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
    eprintln!(
        "wrote particle render-gate timeline to {}",
        output.display()
    );
    Ok(())
}

fn read_presentation_context(process: &Process) -> Result<PresentationContext, String> {
    let tick_before = process.read_u32(crate::entity::TICK_50HZ)?;
    let context_before =
        process.read_bytes(PRESENTATION_CONTEXT_ADDRESS, PRESENTATION_CONTEXT_BYTES)?;
    let projection_before =
        process.read_bytes(ACTIVE_PROJECTION_ADDRESS, ACTIVE_PROJECTION_BYTES)?;
    let context_after =
        process.read_bytes(PRESENTATION_CONTEXT_ADDRESS, PRESENTATION_CONTEXT_BYTES)?;
    let projection_after =
        process.read_bytes(ACTIVE_PROJECTION_ADDRESS, ACTIVE_PROJECTION_BYTES)?;
    let tick_after = process.read_u32(crate::entity::TICK_50HZ)?;
    Ok(PresentationContext {
        tick_before,
        tick_after,
        duplicate_stable: tick_before == tick_after
            && context_before == context_after
            && projection_before == projection_after,
        context_address: PRESENTATION_CONTEXT_ADDRESS as u32,
        context_raw_hex: encode_hex(&context_after),
        active_projection_address: ACTIVE_PROJECTION_ADDRESS as u32,
        active_projection_raw_hex: encode_hex(&projection_after),
    })
}

fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(text, "{byte:02X}");
    }
    text
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

#[cfg(test)]
mod tests {
    use super::{encode_hex, ACTIVE_PROJECTION_BYTES, PRESENTATION_CONTEXT_BYTES};

    #[test]
    fn raw_windows_have_stable_lossless_encoding() {
        assert_eq!(PRESENTATION_CONTEXT_BYTES, 0x80);
        assert_eq!(ACTIVE_PROJECTION_BYTES, 0x24);
        assert_eq!(encode_hex(&[0x00, 0xA5, 0xFF]), "00A5FF");
    }
}
