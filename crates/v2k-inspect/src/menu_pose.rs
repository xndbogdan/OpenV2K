//! Synchronized, passive frontend pose capture.
//!
//! Menu state, camera output, frontend entities, and Klaus's private state
//! normally live in separate retail structures. This timeline brackets all of
//! them with the shared retail tick so a stable sample can be compared without
//! debugger breakpoints or callback invocation.

use std::fs::{create_dir_all, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use serde::Serialize;
use serde_json::json;

use crate::entity::{EntityRecord, EntityRuntimeEvidence, TICK_50HZ};
use crate::process::{i16_at, plausible_heap_pointer, u16_at, u32_at, BuildFingerprint, Process};
use crate::timeline::{ensure_distinct_capture_paths, ensure_stop_file_absent, run_stop_aware};

const FRONTEND_GLOBALS_ADDRESS: usize = 0x004D_B1DC;
const FRONTEND_GLOBALS_BYTES: usize = 0x4C;
const KLAUS_HANDLE_OFFSET: usize = 0x00;
const FLAG_HANDLE_OFFSET: usize = 0x04;
const BILLBOARD_PROJECTION_OFFSET: usize = 0x18;
const BILLBOARD_PROJECTION_BYTES: usize = 0x0C;
const BILLBOARD_X_OFFSET: usize = 0x18;
const BILLBOARD_Y_OFFSET: usize = 0x1A;
const BILLBOARD_SCALE_OFFSET: usize = 0x20;
const GAME_STATE_OFFSET: usize = 0x24;
const FRAME_DELTA_US_OFFSET: usize = 0x30;
const MENU_ACTION_OFFSET: usize = 0x34;
const KLAUS_PRIVATE_STATE_POINTER_OFFSET: usize = 0x38;
const FRONTEND_FLAG_OFFSET: usize = 0x3C;
const MENU_SUB_MODE_OFFSET: usize = 0x48;
const KLAUS_PRIVATE_STATE_BYTES: usize = 0x24;

pub struct MenuPoseTimelineRequest<'a> {
    pub seconds: f64,
    pub hz: u32,
    pub stop_file: Option<&'a Path>,
    pub output: &'a Path,
}

#[derive(Serialize)]
struct CaptureMeta<'a> {
    record_kind: &'static str,
    tool_version: &'static str,
    command: &'static str,
    process_id: u32,
    executable: &'a str,
    build: &'a BuildFingerprint,
}

#[derive(Debug, Serialize)]
struct Captured<T> {
    value: Option<T>,
    error: Option<String>,
}

impl<T> Captured<T> {
    fn from_result(result: Result<T, String>) -> Self {
        match result {
            Ok(value) => Self {
                value: Some(value),
                error: None,
            },
            Err(error) => Self {
                value: None,
                error: Some(error),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct BillboardProjectionWindow {
    address: u32,
    requested_bytes: usize,
    raw_hex: String,
    x: i16,
    y: i16,
    unknown_dword_at_0x004d_b1f8: u32,
    scale: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct FrontendGlobals {
    address: u32,
    requested_bytes: usize,
    raw_hex: String,
    raw_dwords: [u32; FRONTEND_GLOBALS_BYTES / 4],
    klaus_handle: u32,
    flag_handle: u32,
    billboard_projection: BillboardProjectionWindow,
    game_state: i32,
    frame_delta_us: i32,
    menu_action: u32,
    klaus_private_state_pointer: u32,
    frontend_flag: u32,
    menu_sub_mode: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct KlausPrivateState {
    state: i32,
    primary_spin_phase: u32,
    idle_twitch_phase: u32,
    idle_twitch_rate: i32,
    unknown_dword_at_0x10: u32,
    background_zoom: u32,
    position_x_raw: i16,
    position_y_raw: i16,
    position_z_raw: i16,
    unknown_word_at_0x1e: u16,
    unknown_dword_at_0x20: u32,
    raw_dwords: [u32; KLAUS_PRIVATE_STATE_BYTES / 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct KlausPrivateStateWindow {
    pointer: u32,
    requested_bytes: usize,
    duplicate_stable: bool,
    raw_hex_before: Option<String>,
    raw_hex_after: Option<String>,
    value: Option<KlausPrivateState>,
    read_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct FrontendPoseSnapshot {
    tick_before: u32,
    tick_after: u32,
    tick_stable: bool,
    globals_duplicate_stable: bool,
    globals_before: FrontendGlobals,
    globals_after: FrontendGlobals,
    klaus_private_state: KlausPrivateStateWindow,
}

#[derive(Debug, Clone, Serialize)]
struct FrontendEntitySnapshot {
    tick_before: u32,
    tick_after: u32,
    tick_stable: bool,
    head_before: u32,
    head_after: u32,
    tail_before: u32,
    tail_after: u32,
    topology_stable: bool,
    model_pool_pointer: u32,
    warnings: Vec<String>,
    records: Vec<EntityRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RingFormulaInputs {
    top_screen: u32,
    main_ring_screen: bool,
    selected_index: Option<i16>,
    observed_model_item_count: usize,
    /// Exact visible-count input only for the seven-entry main ring, whose
    /// entries all use the model-item callback.
    main_ring_visible_count: Option<usize>,
    ring_spin: u16,
    horizontal_interp: i32,
    fly_clock: u32,
    transition_pending: u16,
    transition_push: u16,
    transition_screen: u32,
    transition_delta: i32,
}

#[derive(Debug, Serialize)]
struct MenuPoseSample {
    record_kind: &'static str,
    sample: u64,
    elapsed_ms: f64,
    outer_tick_before: u32,
    outer_tick_after: u32,
    outer_tick_stable: bool,
    frontend: Captured<FrontendPoseSnapshot>,
    menu: Captured<crate::menu::MenuSnapshot>,
    render: Captured<crate::render::RenderSnapshot>,
    entities: Captured<FrontendEntitySnapshot>,
    klaus_runtime_evidence: Captured<EntityRuntimeEvidence>,
    ring_formula_inputs: Option<RingFormulaInputs>,
}

pub fn capture(process: &Process, request: MenuPoseTimelineRequest<'_>) -> Result<(), String> {
    validate_request(&request)?;
    ensure_distinct_capture_paths(request.stop_file, request.output)?;
    ensure_stop_file_absent(request.stop_file)?;

    let mut writer = create_writer(request.output)?;
    write_json_line(
        &mut writer,
        &json!({
            "meta": CaptureMeta {
                record_kind: "capture_meta",
                tool_version: env!("CARGO_PKG_VERSION"),
                command: "menu-pose-timeline",
                process_id: process.process_id,
                executable: &process.exe_name,
                build: &process.build,
            },
            "seconds": request.seconds,
            "hz": request.hz,
            "stop_file": request.stop_file.map(|path| path.display().to_string()),
            "read_only": true,
            "coverage": [
                "menu_stack_selection_transition_and_ring_formula_inputs",
                "complete_render_camera_spring_view_and_projection_state",
                "complete_type_0_and_type_1_frontend_entity_records",
                "klaus_bounded_runtime_evidence",
                "fixed_frontend_handles_billboard_projection_and_private_state"
            ],
            "synchronization_policy": "each compound sample is bracketed by 0x004FED60; component readers retain their own tick/topology/duplicate-read stability, and unstable samples remain explicit",
            "frontend_policy": "0x004DB1DC..0x004DB227 and the 0x24-byte state named by 0x004DB214 are duplicate-read only; callbacks are never invoked and retail memory is never written",
            "ring_formula": {
                "source": "FUN_0043AA40_then_FUN_0043B410",
                "packed_angle": "floor(65535 / (visible_count * 200)) * (i16(horizontal_interp) + (item_index - selection) * 200)",
                "local_position_raw": "x=800*sin(angle), y=-320*cos(angle), z=y_in-1200*cos(angle)",
                "capture_boundary": "persistent formula inputs are retained exactly; y_in transition policy and final per-call matrices are not reconstructed here because they are transient renderer submissions; use a focused WinDbg draw-call hook only if those final matrices are required"
            },
        }),
    )?;
    // Guided PowerShell capture validates the attached PID and stop path from
    // the first line while this process is still running.
    writer.flush().map_err(|error| error.to_string())?;

    let outcome = run_stop_aware(
        request.seconds,
        request.hz,
        request.stop_file,
        |sample, elapsed_ms| {
            let value = read_compound_sample(process, sample, elapsed_ms)?;
            write_json_line(&mut writer, &value)?;
            if sample % u64::from(request.hz) == 0 {
                writer.flush().map_err(|error| error.to_string())?;
            }
            Ok(())
        },
    )?;
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
    eprintln!("wrote menu pose timeline to {}", request.output.display());
    Ok(())
}

fn read_compound_sample(
    process: &Process,
    sample: u64,
    elapsed_ms: f64,
) -> Result<MenuPoseSample, String> {
    let outer_tick_before = process.read_u32(TICK_50HZ)?;
    let frontend_result = read_frontend_pose(process);
    let menu_result = crate::menu::read_snapshot(process);
    let render_result = crate::render::read_snapshot(process);
    let entity_result = read_frontend_entities(process);

    let klaus_runtime_result = match (&frontend_result, &entity_result) {
        (Ok(frontend), Ok(entities))
            if frontend.tick_stable
                && frontend.globals_duplicate_stable
                && entities.tick_stable
                && entities.topology_stable =>
        {
            let handle = frontend.globals_after.klaus_handle;
            if handle == 0 {
                Err("frontend Klaus handle at 0x004DB1DC is null".into())
            } else {
                let matches = entities
                    .records
                    .iter()
                    .filter(|record| record.entity_type == 0 && record.handle == handle)
                    .collect::<Vec<_>>();
                match matches.as_slice() {
                    [record] => Ok(crate::entity::read_runtime_evidence(process, record)),
                    [] => Err(format!(
                        "no type-0 entity matches frontend Klaus handle 0x{handle:08X}"
                    )),
                    _ => Err(format!(
                        "multiple type-0 entities match frontend Klaus handle 0x{handle:08X}"
                    )),
                }
            }
        }
        (Ok(_), Ok(_)) => Err(
            "frontend globals or filtered entity topology changed during the sample; Klaus runtime evidence was not dereferenced"
                .into(),
        ),
        (Err(error), _) => Err(format!(
            "Klaus runtime evidence unavailable because frontend globals failed: {error}"
        )),
        (_, Err(error)) => Err(format!(
            "Klaus runtime evidence unavailable because entity snapshot failed: {error}"
        )),
    };
    let outer_tick_after = process.read_u32(TICK_50HZ)?;
    let ring_formula_inputs = menu_result.as_ref().ok().map(ring_formula_inputs);

    Ok(MenuPoseSample {
        record_kind: "menu_pose_sample",
        sample,
        elapsed_ms,
        outer_tick_before,
        outer_tick_after,
        outer_tick_stable: outer_tick_before == outer_tick_after,
        frontend: Captured::from_result(frontend_result),
        menu: Captured::from_result(menu_result),
        render: Captured::from_result(render_result),
        entities: Captured::from_result(entity_result),
        klaus_runtime_evidence: Captured::from_result(klaus_runtime_result),
        ring_formula_inputs,
    })
}

fn read_frontend_entities(process: &Process) -> Result<FrontendEntitySnapshot, String> {
    let snapshot = crate::entity::read_snapshot(process)?;
    Ok(FrontendEntitySnapshot {
        tick_before: snapshot.tick_before,
        tick_after: snapshot.tick_after,
        tick_stable: snapshot.tick_stable,
        head_before: snapshot.head_before,
        head_after: snapshot.head_after,
        tail_before: snapshot.tail_before,
        tail_after: snapshot.tail_after,
        topology_stable: snapshot.topology_stable,
        model_pool_pointer: snapshot.model_pool_pointer,
        warnings: snapshot.warnings,
        records: snapshot
            .records
            .into_iter()
            .filter(|record| matches!(record.entity_type, 0 | 1))
            .collect(),
    })
}

fn read_frontend_pose(process: &Process) -> Result<FrontendPoseSnapshot, String> {
    let tick_before = process.read_u32(TICK_50HZ)?;
    let globals_before_bytes =
        process.read_bytes(FRONTEND_GLOBALS_ADDRESS, FRONTEND_GLOBALS_BYTES)?;
    let globals_before = decode_frontend_globals(&globals_before_bytes);
    let klaus_private_state =
        read_klaus_private_state(process, globals_before.klaus_private_state_pointer);
    let globals_after_bytes =
        process.read_bytes(FRONTEND_GLOBALS_ADDRESS, FRONTEND_GLOBALS_BYTES)?;
    let globals_after = decode_frontend_globals(&globals_after_bytes);
    let tick_after = process.read_u32(TICK_50HZ)?;

    Ok(FrontendPoseSnapshot {
        tick_before,
        tick_after,
        tick_stable: tick_before == tick_after,
        globals_duplicate_stable: globals_before_bytes == globals_after_bytes,
        globals_before,
        globals_after,
        klaus_private_state,
    })
}

fn read_klaus_private_state(process: &Process, pointer: u32) -> KlausPrivateStateWindow {
    if pointer == 0 {
        return KlausPrivateStateWindow {
            pointer,
            requested_bytes: KLAUS_PRIVATE_STATE_BYTES,
            duplicate_stable: false,
            raw_hex_before: None,
            raw_hex_after: None,
            value: None,
            read_error: Some("null private-state pointer".into()),
        };
    }
    if !plausible_heap_pointer(pointer as usize) {
        return KlausPrivateStateWindow {
            pointer,
            requested_bytes: KLAUS_PRIVATE_STATE_BYTES,
            duplicate_stable: false,
            raw_hex_before: None,
            raw_hex_after: None,
            value: None,
            read_error: Some(format!("implausible private-state pointer 0x{pointer:08X}")),
        };
    }

    let before = match process.read_bytes(pointer as usize, KLAUS_PRIVATE_STATE_BYTES) {
        Ok(bytes) => bytes,
        Err(error) => {
            return KlausPrivateStateWindow {
                pointer,
                requested_bytes: KLAUS_PRIVATE_STATE_BYTES,
                duplicate_stable: false,
                raw_hex_before: None,
                raw_hex_after: None,
                value: None,
                read_error: Some(error),
            };
        }
    };
    let after = match process.read_bytes(pointer as usize, KLAUS_PRIVATE_STATE_BYTES) {
        Ok(bytes) => bytes,
        Err(error) => {
            return KlausPrivateStateWindow {
                pointer,
                requested_bytes: KLAUS_PRIVATE_STATE_BYTES,
                duplicate_stable: false,
                raw_hex_before: Some(hex_bytes(&before)),
                raw_hex_after: None,
                value: None,
                read_error: Some(error),
            };
        }
    };
    KlausPrivateStateWindow {
        pointer,
        requested_bytes: KLAUS_PRIVATE_STATE_BYTES,
        duplicate_stable: before == after,
        raw_hex_before: Some(hex_bytes(&before)),
        raw_hex_after: Some(hex_bytes(&after)),
        value: Some(decode_klaus_private_state(&after)),
        read_error: None,
    }
}

fn decode_frontend_globals(bytes: &[u8]) -> FrontendGlobals {
    debug_assert_eq!(bytes.len(), FRONTEND_GLOBALS_BYTES);
    let mut raw_dwords = [0; FRONTEND_GLOBALS_BYTES / 4];
    for (index, value) in raw_dwords.iter_mut().enumerate() {
        *value = u32_at(bytes, index * 4);
    }
    let projection = &bytes
        [BILLBOARD_PROJECTION_OFFSET..BILLBOARD_PROJECTION_OFFSET + BILLBOARD_PROJECTION_BYTES];
    FrontendGlobals {
        address: FRONTEND_GLOBALS_ADDRESS as u32,
        requested_bytes: FRONTEND_GLOBALS_BYTES,
        raw_hex: hex_bytes(bytes),
        raw_dwords,
        klaus_handle: u32_at(bytes, KLAUS_HANDLE_OFFSET),
        flag_handle: u32_at(bytes, FLAG_HANDLE_OFFSET),
        billboard_projection: BillboardProjectionWindow {
            address: (FRONTEND_GLOBALS_ADDRESS + BILLBOARD_PROJECTION_OFFSET) as u32,
            requested_bytes: BILLBOARD_PROJECTION_BYTES,
            raw_hex: hex_bytes(projection),
            x: i16_at(bytes, BILLBOARD_X_OFFSET),
            y: i16_at(bytes, BILLBOARD_Y_OFFSET),
            unknown_dword_at_0x004d_b1f8: u32_at(bytes, BILLBOARD_PROJECTION_OFFSET + 4),
            scale: u32_at(bytes, BILLBOARD_SCALE_OFFSET) as i32,
        },
        game_state: u32_at(bytes, GAME_STATE_OFFSET) as i32,
        frame_delta_us: u32_at(bytes, FRAME_DELTA_US_OFFSET) as i32,
        menu_action: u32_at(bytes, MENU_ACTION_OFFSET),
        klaus_private_state_pointer: u32_at(bytes, KLAUS_PRIVATE_STATE_POINTER_OFFSET),
        frontend_flag: u32_at(bytes, FRONTEND_FLAG_OFFSET),
        menu_sub_mode: u32_at(bytes, MENU_SUB_MODE_OFFSET),
    }
}

fn decode_klaus_private_state(bytes: &[u8]) -> KlausPrivateState {
    debug_assert_eq!(bytes.len(), KLAUS_PRIVATE_STATE_BYTES);
    let mut raw_dwords = [0; KLAUS_PRIVATE_STATE_BYTES / 4];
    for (index, value) in raw_dwords.iter_mut().enumerate() {
        *value = u32_at(bytes, index * 4);
    }
    KlausPrivateState {
        state: u32_at(bytes, 0x00) as i32,
        primary_spin_phase: u32_at(bytes, 0x04),
        idle_twitch_phase: u32_at(bytes, 0x08),
        idle_twitch_rate: u32_at(bytes, 0x0C) as i32,
        unknown_dword_at_0x10: u32_at(bytes, 0x10),
        background_zoom: u32_at(bytes, 0x14),
        position_x_raw: i16_at(bytes, 0x18),
        position_y_raw: i16_at(bytes, 0x1A),
        position_z_raw: i16_at(bytes, 0x1C),
        unknown_word_at_0x1e: u16_at(bytes, 0x1E),
        unknown_dword_at_0x20: u32_at(bytes, 0x20),
        raw_dwords,
    }
}

fn ring_formula_inputs(snapshot: &crate::menu::MenuSnapshot) -> RingFormulaInputs {
    RingFormulaInputs {
        top_screen: snapshot.top_screen,
        main_ring_screen: snapshot.top_screen == crate::menu::MAIN_RING_SCREEN,
        selected_index: snapshot.stack.last().map(|entry| entry.selection),
        observed_model_item_count: snapshot.model_items.len(),
        main_ring_visible_count: (snapshot.top_screen == crate::menu::MAIN_RING_SCREEN)
            .then_some(snapshot.model_items.len()),
        ring_spin: snapshot.ring_spin,
        horizontal_interp: snapshot.horizontal_interp,
        fly_clock: snapshot.fly_clock,
        transition_pending: snapshot.transition_pending,
        transition_push: snapshot.transition_push,
        transition_screen: snapshot.transition_screen,
        transition_delta: snapshot.transition_delta,
    }
}

fn validate_request(request: &MenuPoseTimelineRequest<'_>) -> Result<(), String> {
    if !request.seconds.is_finite() || request.seconds <= 0.0 {
        return Err("--seconds must be a finite positive number".into());
    }
    if request.hz == 0 || request.hz > 200 {
        return Err("--hz must be in 1..=200".into());
    }
    Ok(())
}

fn hex_bytes(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02X}").expect("writing to a String cannot fail");
    }
    output
}

fn create_writer(path: &Path) -> Result<BufWriter<File>, String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            create_dir_all(parent).map_err(|error| error.to_string())?;
        }
    }
    File::create(path)
        .map(BufWriter::new)
        .map_err(|error| error.to_string())
}

fn write_json_line(writer: &mut impl Write, value: &impl Serialize) -> Result<(), String> {
    serde_json::to_writer(&mut *writer, value).map_err(|error| error.to_string())?;
    writer.write_all(b"\n").map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontend_globals_decoder_retains_projection_and_handles() {
        let mut bytes = [0u8; FRONTEND_GLOBALS_BYTES];
        put_u32(&mut bytes, KLAUS_HANDLE_OFFSET, 0x04FE_0001);
        put_u32(&mut bytes, FLAG_HANDLE_OFFSET, 0x04FE_0002);
        put_u16(&mut bytes, BILLBOARD_X_OFFSET, (-123i16) as u16);
        put_u16(&mut bytes, BILLBOARD_Y_OFFSET, 456);
        put_u32(&mut bytes, BILLBOARD_PROJECTION_OFFSET + 4, 0xDEAD_BEEF);
        put_u32(&mut bytes, BILLBOARD_SCALE_OFFSET, 0x0001_2345);
        put_u32(&mut bytes, GAME_STATE_OFFSET, 2);
        put_u32(&mut bytes, FRAME_DELTA_US_OFFSET, 16_667);
        put_u32(&mut bytes, MENU_ACTION_OFFSET, 4);
        put_u32(&mut bytes, KLAUS_PRIVATE_STATE_POINTER_OFFSET, 0x0200_1000);
        put_u32(&mut bytes, FRONTEND_FLAG_OFFSET, 1);
        put_u32(&mut bytes, MENU_SUB_MODE_OFFSET, 7);

        let decoded = decode_frontend_globals(&bytes);
        assert_eq!(decoded.klaus_handle, 0x04FE_0001);
        assert_eq!(decoded.flag_handle, 0x04FE_0002);
        assert_eq!(decoded.billboard_projection.x, -123);
        assert_eq!(decoded.billboard_projection.y, 456);
        assert_eq!(
            decoded.billboard_projection.unknown_dword_at_0x004d_b1f8,
            0xDEAD_BEEF
        );
        assert_eq!(decoded.billboard_projection.scale, 0x0001_2345);
        assert_eq!(decoded.frame_delta_us, 16_667);
        assert_eq!(decoded.klaus_private_state_pointer, 0x0200_1000);
        assert_eq!(decoded.menu_sub_mode, 7);
    }

    #[test]
    fn klaus_private_state_decoder_preserves_packed_signed_positions() {
        let mut bytes = [0u8; KLAUS_PRIVATE_STATE_BYTES];
        put_u32(&mut bytes, 0x00, 1);
        put_u32(&mut bytes, 0x04, 0x6001);
        put_u32(&mut bytes, 0x08, 0x0123);
        put_u32(&mut bytes, 0x0C, 47);
        put_u32(&mut bytes, 0x10, 0xFFFF_FFFE);
        put_u32(&mut bytes, 0x14, 0xFFFF);
        put_u16(&mut bytes, 0x18, (-20i16) as u16);
        put_u16(&mut bytes, 0x1A, 0x0320);
        put_u16(&mut bytes, 0x1C, 0x1400);
        put_u16(&mut bytes, 0x1E, 0xABCD);
        put_u32(&mut bytes, 0x20, 0x1122_3344);

        let decoded = decode_klaus_private_state(&bytes);
        assert_eq!(decoded.state, 1);
        assert_eq!(decoded.primary_spin_phase, 0x6001);
        assert_eq!(decoded.idle_twitch_rate, 47);
        assert_eq!(decoded.unknown_dword_at_0x10, 0xFFFF_FFFE);
        assert_eq!(decoded.position_x_raw, -20);
        assert_eq!(decoded.position_y_raw, 0x0320);
        assert_eq!(decoded.position_z_raw, 0x1400);
        assert_eq!(decoded.unknown_word_at_0x1e, 0xABCD);
        assert_eq!(decoded.unknown_dword_at_0x20, 0x1122_3344);
    }

    #[test]
    fn request_validation_rejects_invalid_rates_and_durations() {
        let output = Path::new("capture.jsonl");
        let request = |seconds, hz| MenuPoseTimelineRequest {
            seconds,
            hz,
            stop_file: None,
            output,
        };
        assert!(validate_request(&request(180.0, 100)).is_ok());
        assert!(validate_request(&request(0.0, 100)).is_err());
        assert!(validate_request(&request(f64::NAN, 100)).is_err());
        assert!(validate_request(&request(180.0, 0)).is_err());
        assert!(validate_request(&request(180.0, 201)).is_err());
    }

    fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
}
