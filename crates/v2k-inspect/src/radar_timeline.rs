//! Read-only terrain-radar and fullscreen-map evidence capture.
//!
//! Retail `FUN_004494A0` rasterizes the right-hand terrain radar from the
//! 256x256 byte buffer at `DAT_004DF118` and the packed terrain-coverage nibbles at
//! `DAT_004EF130`, then calls `FUN_0044AFF0` once per live entity to add
//! markers.  This module retains those inputs rather than attempting to infer
//! radar pixels from a screenshot.  Raw session/controller windows around M
//! transitions are deliberately offset-named runtime evidence: no map-mode
//! field is claimed until a trace proves it.

use std::collections::VecDeque;
use std::fs::{create_dir_all, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use serde::Serialize;
use serde_json::json;

use crate::controller::{SESSION_CONTROLLER_OFFSET, SESSION_POINTER_GLOBAL};
use crate::entity::{EntityRecord, EntitySnapshot, TICK_50HZ};
use crate::foreground_input::{HoverControl, HoverInputState, InputEdge};
use crate::process::{plausible_data_pointer, plausible_heap_pointer, BuildFingerprint, Process};
use crate::timeline::{ensure_distinct_capture_paths, ensure_stop_file_absent, run_stop_aware};

const RADAR_WIDTH: usize = 0x004E_F128;
const RADAR_HEIGHT: usize = 0x004D_F110;
const RADAR_PALETTE_BASE_ID: usize = 0x004D_F114;
const RADAR_RENDER_PARAMETER: usize = 0x004E_F12C;
const RADAR_TERRAIN_BUFFER: usize = 0x004D_F118;
const RADAR_TERRAIN_BUFFER_BYTES: usize = 256 * 256;
const RADAR_COVERAGE_BUFFER: usize = 0x004E_F130;
const RADAR_COVERAGE_BUFFER_BYTES: usize = 256 * 256 / 2;
const RADAR_REBUILD_CURSOR: usize = 0x004F_7134;
const RADAR_REBUILD_AXIS: usize = 0x004F_7138;
const FULLSCREEN_MAP_ICON_PHASE: usize = 0x004E_F118;
const FULLSCREEN_MAP_ICON_TIMESTAMP_LOW: usize = 0x004E_F120;
const FULLSCREEN_MAP_ICON_TIMESTAMP_HIGH: usize = 0x004E_F124;
const FULLSCREEN_MAP_PASS_COUNTER: usize = 0x004F_72EC;
const FULLSCREEN_MAP_MODAL_GATE: usize = 0x004F_741C;
const DIRECT_INPUT_KEYBOARD_STATE: usize = 0x004F_ED80;
const DIRECT_INPUT_KEYBOARD_STATE_BYTES: usize = 256;
const DIK_M: usize = 0x32;
const DIK_LEFT: usize = 0xCB;
const DIK_RIGHT: usize = 0xCD;
const DIK_UP: usize = 0xC8;
const DIK_DOWN: usize = 0xD0;
const SETTINGS_BASE: usize = 0x004C_B3D8;
const TARGETTER_SETTING_OFFSET: usize = 0x34;
const HUD_SETTING_OFFSET: usize = 0x38;
const TARGETTER_OFFSET: usize = 0x22C;
const TARGETTER_BYTES: usize = 0x54;
const TARGETTER_CALLBACK_TABLE_POINTER_OFFSET: usize = 0x44;
const TARGETTER_CALLBACK_TABLE_BYTES: usize = 0x14;
/// Pointer to the runtime array consumed by `FUN_004494A0`.
///
/// Retail performs two indirections: first load the array pointer from this
/// global, then load the three resource pointers from `array + index * 4`.
const RADAR_PROJECTION_RESOURCE_TABLE_POINTER: usize = 0x004F_E658;
const RADAR_PROJECTION_RESOURCE_COUNT: usize = 3;
const RADAR_RESOURCE_HEADER_BYTES: usize = 0x20;
const RADAR_RESOURCE_DATA_POINTER_OFFSET: usize = 0x18;
const MASTER_COLOR_POINTER_TABLE: usize = 0x004F_E63C;
const TERRAIN_PALETTE_ENTRY_COUNT: usize = 35;
const GLOBE_SHADING_PALETTE_ENTRY_COUNT: usize = 16;
const MARKER_PALETTE_ENTRY_COUNT: usize = 7;
const GLOBAL_SPRITE_POINTER_TABLE: usize = 0x004F_E62C;
const ENTITY_TYPE_POINTER_TABLE: usize = 0x004F_E650;
const ENTITY_TYPE_COUNT: usize = 130;
const FULLSCREEN_MAP_ICON_SELECTOR_OFFSET: usize = 0x70;
const FULLSCREEN_MAP_ICON_HEADER_BYTES: usize = 0x20;
const SPRITE_META_POINTER_GLOBAL: usize = 0x004F_E620;
const SPRITE_META_WIDTH_POINTER_OFFSET: usize = 0x18;
const SPRITE_META_HEIGHT_POINTER_OFFSET: usize = 0x1C;
const SPRITE_META_LAYOUT_BYTES: usize = 0x40;
const RETAIL_FRAME_COUNTER: usize = 0x004F_ED60;
const SESSION_DISCOVERY_WINDOW_BYTES: usize = 0x400;
const CONTROLLER_DISCOVERY_WINDOW_BYTES: usize = 0x300;
const MAX_CHANGED_CELLS: usize = 128;

#[derive(Serialize)]
struct CaptureMeta<'a> {
    record_kind: &'static str,
    tool_version: &'static str,
    command: &'a str,
    process_id: u32,
    executable: &'a str,
    build: &'a BuildFingerprint,
}

#[derive(Debug, Clone)]
struct RadarBuffers {
    tick_before: u32,
    tick_after: u32,
    width: i32,
    height: i32,
    palette_base_id: i32,
    render_parameter: i32,
    rebuild_cursor: u32,
    rebuild_axis: u32,
    terrain_duplicate_stable: bool,
    coverage_duplicate_stable: bool,
    terrain: Vec<u8>,
    coverage: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ByteChange {
    index: u32,
    before: u8,
    after: u8,
}

#[derive(Debug, Clone, Serialize)]
struct RadarBufferSummary {
    tick_before: u32,
    tick_after: u32,
    tick_stable: bool,
    width_at_0x004ef128: i32,
    height_at_0x004df110: i32,
    palette_base_id_at_0x004df114: i32,
    render_parameter_at_0x004ef12c: i32,
    rebuild_cursor_at_0x004f7134: u32,
    rebuild_axis_at_0x004f7138: u32,
    terrain_buffer_address: u32,
    terrain_buffer_bytes: usize,
    terrain_duplicate_stable: bool,
    terrain_fnv1a64: String,
    terrain_value_histogram: Vec<u32>,
    terrain_changed_cells: usize,
    terrain_changed_cells_sample: Vec<ByteChange>,
    coverage_buffer_address: u32,
    coverage_buffer_bytes: usize,
    coverage_duplicate_stable: bool,
    coverage_fnv1a64: String,
    coverage_nibble_histogram: [u32; 16],
    coverage_changed_bytes: usize,
    coverage_changed_bytes_sample: Vec<ByteChange>,
    player_cell: Option<RadarCellEvidence>,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct RadarCellEvidence {
    x: u8,
    z: u8,
    linear_index: u32,
    terrain_raster_value: u8,
    terrain_coverage_nibble: u8,
}

#[derive(Debug, Clone, Serialize)]
struct RadarEntityEvidence {
    intrusive_list_index: usize,
    pointer: u32,
    handle: u32,
    entity_type: u32,
    flags_at_entity_0x08: u32,
    marker_flags_at_entity_0x64: u32,
    position_raw_8_8: [i16; 3],
    relative_x_raw_wrapped: Option<i16>,
    relative_z_raw_wrapped: Option<i16>,
    radial_distance_input_after_quartering: Option<u32>,
    radial_distance_raw_approx: Option<u32>,
    inside_retail_0x4000_marker_radius: Option<bool>,
    cell: RadarCellEvidence,
    rejected_by_initial_entity_gate: bool,
    has_any_supported_marker_flag_0x8ebd: bool,
    zero_coverage_random_visibility_gate_applies: bool,
    passes_pre_radius_flag_gates: bool,
}

#[derive(Debug, Clone, Serialize)]
struct RadarPlayerEvidence {
    pointer: u32,
    handle: u32,
    position_raw_8_8: [i16; 3],
    velocity_raw_8_8: [i16; 3],
    rotation: [u16; 3],
    body_basis_raw: [i32; 9],
    cell: RadarCellEvidence,
}

#[derive(Debug, Clone, Serialize)]
struct DuplicateMemoryWindow {
    address: u32,
    requested_bytes: usize,
    duplicate_stable: Option<bool>,
    fnv1a64: Option<String>,
    raw_hex: Option<String>,
    read_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct DirectInputEvidence {
    state: DuplicateMemoryWindow,
    dik_m_down: Option<bool>,
    dik_left_down: Option<bool>,
    dik_right_down: Option<bool>,
    dik_up_down: Option<bool>,
    dik_down_down: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
struct TargetterEvidence {
    base_at_controller_0x22c: u32,
    block: DuplicateMemoryWindow,
    current_target_handle_at_0x0c: Option<u32>,
    prior_target_handle_at_0x04: Option<u32>,
    ray_distance_or_parameter_at_0x14: Option<u32>,
    result_bits_at_0x34: Option<u32>,
    scan_radius_at_0x38: Option<u32>,
    callback_table_pointer_at_0x44: Option<u32>,
    callback_table: Option<DuplicateMemoryWindow>,
}

#[derive(Debug, Clone, Serialize)]
struct RuntimeRadarWindows {
    tick_before: Option<u32>,
    tick_after: Option<u32>,
    tick_stable: Option<bool>,
    session_pointer_global_address: u32,
    session_pointer: Option<u32>,
    session: Option<DuplicateMemoryWindow>,
    gameplay_state_byte_at_session_0x296: Option<u8>,
    controller_pointer_offset_at_session_0x27c: u32,
    controller_pointer: Option<u32>,
    controller: Option<DuplicateMemoryWindow>,
    targetter_setting_at_0x004cb40c: Option<u32>,
    hud_setting_at_0x004cb410: Option<u32>,
    targetter: Option<TargetterEvidence>,
    direct_input_keyboard: DirectInputEvidence,
    fullscreen_map_icon_phase_at_0x004ef118: Option<u32>,
    fullscreen_map_icon_timestamp_low_at_0x004ef120: Option<u32>,
    fullscreen_map_icon_timestamp_high_at_0x004ef124: Option<u32>,
    fullscreen_map_pass_counter_at_0x004f72ec: Option<u32>,
    fullscreen_map_modal_gate_at_0x004f741c: Option<u32>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct PendingFullSnapshot {
    reason: String,
    trigger_sample: u64,
    trigger_elapsed_ms: f64,
}

#[derive(Debug, Clone, Copy)]
struct FullSnapshotBoundary<'a> {
    sample: u64,
    elapsed_ms: f64,
    reason: &'a str,
    trigger_sample: u64,
    trigger_elapsed_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
struct RadarProjectionTableEvidence {
    role: &'static str,
    resource_table_pointer_global_address: u32,
    resource_table_pointer: Option<u32>,
    table_entry_address: Option<u32>,
    resource_pointer: Option<u32>,
    resource_header: Option<DuplicateMemoryWindow>,
    resource_header_after_payload: Option<DuplicateMemoryWindow>,
    data_pointer_at_resource_0x18: Option<u32>,
    derived_requested_bytes: Option<usize>,
    data: Option<DuplicateMemoryWindow>,
    read_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct RadarProjectionDimensionsEvidence {
    width_global_at_0x004ef128: i32,
    height_global_at_0x004df110: i32,
    sprite_meta_pointer_global_address: u32,
    sprite_meta_pointer: Option<u32>,
    sprite_meta_header: Option<DuplicateMemoryWindow>,
    width_pointer_at_sprite_meta_0x18: Option<u32>,
    height_pointer_at_sprite_meta_0x1c: Option<u32>,
    width_value: Option<DuplicateMemoryWindow>,
    height_value: Option<DuplicateMemoryWindow>,
    width_from_sprite_meta: Option<i32>,
    height_from_sprite_meta: Option<i32>,
    dimensions_match_globals: bool,
    derived_payload_lengths: Option<[usize; RADAR_PROJECTION_RESOURCE_COUNT]>,
    validation_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct RadarProjectionResourcesEvidence {
    resource_table_pointer_global_address: u32,
    resource_table_pointer: Option<u32>,
    resource_table: Option<DuplicateMemoryWindow>,
    resource_table_pointer_after_payloads: Option<u32>,
    resource_table_after_payloads: Option<DuplicateMemoryWindow>,
    dimensions: RadarProjectionDimensionsEvidence,
    chains: Vec<RadarProjectionTableEvidence>,
}

#[derive(Debug, Clone, Serialize)]
struct RadarColorEvidence {
    role: String,
    relative_slot_offset: u32,
    pointer_table_entry_address: Option<u32>,
    pointer_table_entry: Option<DuplicateMemoryWindow>,
    color_pointer: Option<u32>,
    packed_color_u32: Option<u32>,
    packed_color_u16: Option<u16>,
    color_value: Option<DuplicateMemoryWindow>,
    read_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct RadarColorTablesEvidence {
    pointer_table_global_address: u32,
    pointer_table_pointer: Option<u32>,
    pointer_table_pointer_after_entries: Option<u32>,
    palette_base_id_at_0x004df114: i32,
    terrain_palette: Vec<RadarColorEvidence>,
    globe_shading_palette: Vec<RadarColorEvidence>,
    marker_palette: Vec<RadarColorEvidence>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct FullscreenMapIconTypeEvidence {
    entity_type: u32,
    slot_state: &'static str,
    type_record_pointer: Option<u32>,
    selector_value: Option<DuplicateMemoryWindow>,
    selector_at_type_record_0x70: Option<u16>,
    sprite_pointer_table_entry_address: Option<u32>,
    sprite_pointer_table_entry: Option<DuplicateMemoryWindow>,
    sprite_resource_pointer: Option<u32>,
    sprite_width_at_resource_0x10: Option<u16>,
    sprite_height_at_resource_0x12: Option<u16>,
    sprite_resource_header: Option<DuplicateMemoryWindow>,
    read_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct FullscreenMapIconTableEvidence {
    entity_type_pointer_table_global_address: u32,
    entity_type_pointer_table: Option<u32>,
    entity_type_pointer_table_window: Option<DuplicateMemoryWindow>,
    entity_type_pointer_table_after_types: Option<DuplicateMemoryWindow>,
    entity_type_pointer_table_after_types_pointer: Option<u32>,
    sprite_pointer_table_global_address: u32,
    sprite_pointer_table: Option<u32>,
    sprite_pointer_table_after_types: Option<u32>,
    unloaded_type_slots: usize,
    loaded_type_records: usize,
    nonzero_icon_selectors: usize,
    types: Vec<FullscreenMapIconTypeEvidence>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct FullscreenMapLayoutEvidence {
    sprite_meta_pointer_global_address: u32,
    sprite_meta_pointer: Option<u32>,
    sprite_meta_pointer_after_values: Option<u32>,
    sprite_meta_layout: Option<DuplicateMemoryWindow>,
    origin_x_pointer_at_0x30: Option<u32>,
    origin_y_pointer_at_0x34: Option<u32>,
    width_pointer_at_0x38: Option<u32>,
    height_pointer_at_0x3c: Option<u32>,
    origin_x_value: Option<DuplicateMemoryWindow>,
    origin_y_value: Option<DuplicateMemoryWindow>,
    width_value: Option<DuplicateMemoryWindow>,
    height_value: Option<DuplicateMemoryWindow>,
    origin_x_from_pointer_at_0x30: Option<i16>,
    origin_y_from_pointer_at_0x34: Option<i16>,
    width_from_pointer_at_0x38: Option<i32>,
    height_from_pointer_at_0x3c: Option<i32>,
    warnings: Vec<String>,
}

pub fn capture(
    process: &Process,
    seconds: f64,
    hz: u32,
    context_hz: u32,
    buffer_hz: u32,
    stop_file: Option<&Path>,
    output: &Path,
) -> Result<(), String> {
    validate_timeline(seconds, hz, context_hz, buffer_hz)?;
    ensure_distinct_capture_paths(stop_file, output)?;
    ensure_stop_file_absent(stop_file)?;

    let mut writer = create_writer(output)?;
    write_json_line(
        &mut writer,
        &json!({
            "record_kind": "capture_meta",
            "meta": capture_meta(process, "radar-timeline"),
            "seconds": seconds,
            "hz": hz,
            "context_hz_requested": context_hz,
            "radar_buffer_hz_requested": buffer_hz,
            "sampling_schedule": "integer cadence crossing; exact average requested rate without rounded fixed-stride drift",
            "stop_file": stop_file.map(|path| path.display().to_string()),
            "retail_draw_provenance": {
                "terrain_raster": "FUN_004494A0 reads DAT_004DF118 and the terrain-coverage nibbles at DAT_004EF130",
                "entity_markers": "FUN_004494A0 walks the intrusive entity list and calls FUN_0044AFF0",
                "marker_inputs": "entity +0x08 flags, +0x58 type, +0x5C handle, +0x64 marker flags, +0x96 X, and +0x9A Z",
                "targetter_is_separate": true
            },
            "foreground_key_controls": [
                "space", "right_shift", "up", "down", "left", "right",
                "s", "x", "tab", "m"
            ],
            "foreground_protocol_controls": ["caps_lock"],
            "foreground_key_source": "Win32 GetAsyncKeyState is retained for guided-protocol timing only while V2000 owns focus; every context sample also captures the retail 256-byte DirectInput state at 0x004FED80",
            "fullscreen_map_input": "the retail table at 0x004C26D0 binds DIK_M 0x32 to FUN_00456610; while session/context +0x296 > 4 and DAT_004F741C == 0 it pushes descriptor 0x004D0A68 through FUN_00456680",
            "fullscreen_map_close_input": "the map-mode table at 0x004C4610 binds DIK_M 0x32 to FUN_00456650, which returns to gameplay descriptor 0x004D0950; no map pan or zoom binding exists",
            "fullscreen_map_state_policy": "The proven modal gate, icon phase/timestamp, render-pass counter, Targetter block, and menu stack are decoded. Duplicate-stable raw session[0x000..0x3FF] and controller[0x000..0x2FF] windows retain unknown screen-local state without guessing offsets.",
            "coverage_buffer_semantics": "DAT_004EF130 is a packed 4-bit terrain coverage/layer-count field built from material-type-8 footprints; it is not player discovery state or the radar's per-pixel spherical mask",
            "terrain_row_overlap": "0x004DF218 is exactly DAT_004DF118 + 0x100 (row 1), not a second radar buffer",
            "full_buffer_snapshot_policy": "complete terrain/coverage buffers at capture start/end, every M press/release, and 750 ms after M release; compact hashes/deltas between boundaries",
            "read_only": true,
            "callback_policy": "no retail callback is invoked and process memory is never written"
        }),
    )?;

    let mut latest_buffers = read_radar_buffers(process)?;
    let mut latest_player_position = crate::entity::read_snapshot(process)
        .ok()
        .and_then(|snapshot| select_player(&snapshot).map(|record| record.position_raw_8_8));
    write_full_buffers(
        process,
        &mut writer,
        FullSnapshotBoundary {
            sample: 0,
            elapsed_ms: 0.0,
            reason: "capture_start",
            trigger_sample: 0,
            trigger_elapsed_ms: 0.0,
        },
        &latest_buffers,
    )?;
    writer.flush().map_err(|error| error.to_string())?;

    let mut previous_input: Option<HoverInputState> = None;
    let mut previous_foreground = None;
    let mut pending_full_snapshots = VecDeque::new();
    let mut post_m_release_due_ms: Option<f64> = None;

    let outcome = run_stop_aware(seconds, hz, stop_file, |sample, elapsed_ms| {
        let input = crate::foreground_input::poll_hover_foreground(process.process_id);
        let foreground = input.is_some();
        let input_edges = match (previous_input, input) {
            (Some(before), Some(current)) => current.edges_from(before),
            _ => Vec::new(),
        };
        previous_input = input;

        if previous_foreground != Some(foreground) {
            write_json_line(
                &mut writer,
                &json!({
                    "record_kind": "radar_input_scope",
                    "sample": sample,
                    "elapsed_ms": elapsed_ms,
                    "v2000_foreground": foreground,
                }),
            )?;
            previous_foreground = Some(foreground);
        }

        for edge in &input_edges {
            if edge.control == HoverControl::M {
                let reason = match edge.edge {
                    InputEdge::Pressed => "m_pressed",
                    InputEdge::Released => "m_released",
                };
                pending_full_snapshots.push_back(PendingFullSnapshot {
                    reason: reason.to_owned(),
                    trigger_sample: sample,
                    trigger_elapsed_ms: elapsed_ms,
                });
                if edge.edge == InputEdge::Released {
                    post_m_release_due_ms = Some(elapsed_ms + 750.0);
                }
            }
        }
        if post_m_release_due_ms.is_some_and(|due| elapsed_ms >= due) {
            pending_full_snapshots.push_back(PendingFullSnapshot {
                reason: "post_m_release_750ms".to_owned(),
                trigger_sample: sample,
                trigger_elapsed_ms: elapsed_ms,
            });
            post_m_release_due_ms = None;
        }

        if !input_edges.is_empty() || sample % u64::from(hz) == 0 {
            write_json_line(
                &mut writer,
                &json!({
                    "record_kind": "radar_input_sample",
                    "sample": sample,
                    "elapsed_ms": elapsed_ms,
                    "v2000_foreground": foreground,
                    "foreground_input_state": input,
                    "foreground_input_edges": input_edges,
                }),
            )?;
        }

        let buffer_due = cadence_due(sample, hz, buffer_hz) || !pending_full_snapshots.is_empty();
        if buffer_due {
            match read_radar_buffers(process) {
                Ok(current) => {
                    let summary =
                        summarize_buffers(&current, Some(&latest_buffers), latest_player_position);
                    write_json_line(
                        &mut writer,
                        &json!({
                            "record_kind": "radar_buffer_sample",
                            "sample": sample,
                            "elapsed_ms": elapsed_ms,
                            "summary": summary,
                        }),
                    )?;
                    latest_buffers = current;
                    while let Some(pending) = pending_full_snapshots.pop_front() {
                        write_full_buffers(
                            process,
                            &mut writer,
                            FullSnapshotBoundary {
                                sample,
                                elapsed_ms,
                                reason: &pending.reason,
                                trigger_sample: pending.trigger_sample,
                                trigger_elapsed_ms: pending.trigger_elapsed_ms,
                            },
                            &latest_buffers,
                        )?;
                    }
                }
                Err(error) => write_json_line(
                    &mut writer,
                    &json!({
                        "record_kind": "radar_buffer_error",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "error": error,
                    }),
                )?,
            }
        }

        if cadence_due(sample, hz, context_hz) {
            let runtime_windows = read_runtime_windows(process);
            let menu_phase = crate::menu::read_phase_snapshot(process);
            let render = crate::render::read_snapshot(process);
            let entity_snapshot = crate::entity::read_snapshot(process);
            match entity_snapshot {
                Ok(snapshot) => {
                    let player = select_player(&snapshot);
                    if let Some(player) = player {
                        latest_player_position = Some(player.position_raw_8_8);
                    }
                    let player_evidence =
                        player.map(|record| player_evidence(record, &latest_buffers));
                    let (radar_entities, radar_entity_warnings) =
                        radar_entities(&snapshot, player, &latest_buffers);
                    let controller_motion = crate::controller::read_motion_evidence(
                        process,
                        player.map(|record| record.handle),
                    );
                    write_json_line(
                        &mut writer,
                        &json!({
                            "record_kind": "radar_context_sample",
                            "sample": sample,
                            "elapsed_ms": elapsed_ms,
                            "entity_tick_before": snapshot.tick_before,
                            "entity_tick_after": snapshot.tick_after,
                            "entity_tick_stable": snapshot.tick_stable,
                            "entity_topology_stable": snapshot.topology_stable,
                            "player": player_evidence,
                            "entities_in_intrusive_order": radar_entities,
                            "entity_snapshot_warnings": snapshot.warnings,
                            "radar_entity_warnings": radar_entity_warnings,
                            "controller_motion": controller_motion,
                            "render": render.as_ref().ok(),
                            "render_error": render.as_ref().err(),
                            "menu_phase": menu_phase.as_ref().ok(),
                            "menu_phase_error": menu_phase.as_ref().err(),
                            "runtime_radar_windows": runtime_windows,
                        }),
                    )?;
                }
                Err(error) => write_json_line(
                    &mut writer,
                    &json!({
                        "record_kind": "radar_context_error",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "entity_snapshot_error": error,
                        "render": render.as_ref().ok(),
                        "render_error": render.as_ref().err(),
                        "menu_phase": menu_phase.as_ref().ok(),
                        "menu_phase_error": menu_phase.as_ref().err(),
                        "runtime_radar_windows": runtime_windows,
                    }),
                )?,
            }
        }

        if !input_edges.is_empty() || sample % u64::from(hz) == 0 {
            writer.flush().map_err(|error| error.to_string())?;
        }
        Ok(())
    })?;

    match read_radar_buffers(process) {
        Ok(final_buffers) => write_full_buffers(
            process,
            &mut writer,
            FullSnapshotBoundary {
                sample: outcome.samples_completed,
                elapsed_ms: outcome.elapsed_ms,
                reason: "capture_end",
                trigger_sample: outcome.samples_completed,
                trigger_elapsed_ms: outcome.elapsed_ms,
            },
            &final_buffers,
        )?,
        Err(error) => write_json_line(
            &mut writer,
            &json!({
                "record_kind": "radar_buffer_error",
                "sample": outcome.samples_completed,
                "elapsed_ms": outcome.elapsed_ms,
                "reason": "capture_end",
                "error": error,
            }),
        )?,
    }
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
    eprintln!("wrote radar timeline to {}", output.display());
    Ok(())
}

/// Capture the static/runtime resources needed to reproduce the radar and M
/// map without repeating the guided movement protocol.
pub fn capture_resources(process: &Process, output: &Path) -> Result<(), String> {
    let buffers = read_radar_buffers(process)?;
    let projection_tables = read_projection_tables(process, buffers.width, buffers.height);
    let color_tables = read_radar_color_tables(process, buffers.palette_base_id);
    let fullscreen_map_icons = read_fullscreen_map_icons(process);
    let fullscreen_map_layout = read_fullscreen_map_layout(process);
    let mut validation_errors = projection_validation_errors(&projection_tables);
    validation_errors.extend(color_validation_errors(&color_tables));
    validation_errors.extend(fullscreen_map_icon_validation_errors(&fullscreen_map_icons));
    validation_errors.extend(fullscreen_map_layout_validation_errors(
        &fullscreen_map_layout,
    ));
    let complete = validation_errors.is_empty();
    let mut writer = create_writer(output)?;
    serde_json::to_writer_pretty(
        &mut writer,
        &json!({
            "record_kind": "radar_resource_snapshot",
            "complete": complete,
            "validation_errors": validation_errors,
            "meta": capture_meta(process, "radar-resources"),
            "radar_dimensions": {
                "width": buffers.width,
                "height": buffers.height,
                "palette_base_id": buffers.palette_base_id,
                "render_parameter": buffers.render_parameter,
            },
            "radar_projection_tables": projection_tables,
            "radar_color_tables": color_tables,
            "fullscreen_map_icons": fullscreen_map_icons,
            "fullscreen_map_layout": fullscreen_map_layout,
            "read_only": true,
            "callback_policy": "no retail callback is invoked and process memory is never written",
            "completion_contract": {
                "projection_tables": "all three payloads are exact, duplicate-stable, dimension-verified, and enclosed by unchanged outer-table and resource-header reads",
                "radar_colors": "all 35 terrain, 16 globe-shading, and 7 marker entries have stable pointer slots and stable packed values",
                "fullscreen_map_icons": "the type table is stable and every loaded nonzero selector resolves to a stable sprite header with nonzero dimensions",
                "fullscreen_map_layout": "the sprite-meta layout and its four indirect origin/size values are stable, with positive dimensions",
                "intentionally_not_captured": [
                    "fullscreen icon pixel payloads: the proven selector identifies the already-extracted overlay sprite; this capture closes selector/header/dimension linkage",
                    "unloaded zero entity-type slots: catalog absence is recorded as unloaded, not treated as a read failure"
                ]
            },
        }),
    )
    .map_err(|error| error.to_string())?;
    writeln!(writer).map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())?;
    if complete {
        Ok(())
    } else {
        Err(format!(
            "radar resource snapshot is incomplete: {}",
            validation_errors.join("; ")
        ))
    }
}

fn read_radar_buffers(process: &Process) -> Result<RadarBuffers, String> {
    let tick_before = process.read_u32(TICK_50HZ)?;
    let terrain_before = process.read_bytes(RADAR_TERRAIN_BUFFER, RADAR_TERRAIN_BUFFER_BYTES)?;
    let coverage_before = process.read_bytes(RADAR_COVERAGE_BUFFER, RADAR_COVERAGE_BUFFER_BYTES)?;
    let width = process.read_i32(RADAR_WIDTH)?;
    let height = process.read_i32(RADAR_HEIGHT)?;
    let palette_base_id = process.read_i32(RADAR_PALETTE_BASE_ID)?;
    let render_parameter = process.read_i32(RADAR_RENDER_PARAMETER)?;
    let rebuild_cursor = process.read_u32(RADAR_REBUILD_CURSOR)?;
    let rebuild_axis = process.read_u32(RADAR_REBUILD_AXIS)?;
    let terrain = process.read_bytes(RADAR_TERRAIN_BUFFER, RADAR_TERRAIN_BUFFER_BYTES)?;
    let coverage = process.read_bytes(RADAR_COVERAGE_BUFFER, RADAR_COVERAGE_BUFFER_BYTES)?;
    let tick_after = process.read_u32(TICK_50HZ)?;
    Ok(RadarBuffers {
        tick_before,
        tick_after,
        width,
        height,
        palette_base_id,
        render_parameter,
        rebuild_cursor,
        rebuild_axis,
        terrain_duplicate_stable: terrain_before == terrain,
        coverage_duplicate_stable: coverage_before == coverage,
        terrain,
        coverage,
    })
}

fn summarize_buffers(
    current: &RadarBuffers,
    previous: Option<&RadarBuffers>,
    player_position: Option<[i16; 3]>,
) -> RadarBufferSummary {
    let (terrain_changed_cells, terrain_changed_cells_sample) = previous.map_or_else(
        || (0, Vec::new()),
        |before| bounded_changes(&before.terrain, &current.terrain),
    );
    let (coverage_changed_bytes, coverage_changed_bytes_sample) = previous.map_or_else(
        || (0, Vec::new()),
        |before| bounded_changes(&before.coverage, &current.coverage),
    );
    let mut terrain_value_histogram = vec![0u32; 256];
    for value in &current.terrain {
        terrain_value_histogram[usize::from(*value)] += 1;
    }
    let mut coverage_nibble_histogram = [0u32; 16];
    for packed in &current.coverage {
        coverage_nibble_histogram[usize::from(packed & 0x0F)] += 1;
        coverage_nibble_histogram[usize::from(packed >> 4)] += 1;
    }
    RadarBufferSummary {
        tick_before: current.tick_before,
        tick_after: current.tick_after,
        tick_stable: current.tick_before == current.tick_after,
        width_at_0x004ef128: current.width,
        height_at_0x004df110: current.height,
        palette_base_id_at_0x004df114: current.palette_base_id,
        render_parameter_at_0x004ef12c: current.render_parameter,
        rebuild_cursor_at_0x004f7134: current.rebuild_cursor,
        rebuild_axis_at_0x004f7138: current.rebuild_axis,
        terrain_buffer_address: RADAR_TERRAIN_BUFFER as u32,
        terrain_buffer_bytes: current.terrain.len(),
        terrain_duplicate_stable: current.terrain_duplicate_stable,
        terrain_fnv1a64: format!("{:016X}", fnv1a64(&current.terrain)),
        terrain_value_histogram,
        terrain_changed_cells,
        terrain_changed_cells_sample,
        coverage_buffer_address: RADAR_COVERAGE_BUFFER as u32,
        coverage_buffer_bytes: current.coverage.len(),
        coverage_duplicate_stable: current.coverage_duplicate_stable,
        coverage_fnv1a64: format!("{:016X}", fnv1a64(&current.coverage)),
        coverage_nibble_histogram,
        coverage_changed_bytes,
        coverage_changed_bytes_sample,
        player_cell: player_position.map(|position| cell_evidence(position, current)),
    }
}

fn write_full_buffers(
    process: &Process,
    writer: &mut impl Write,
    boundary: FullSnapshotBoundary<'_>,
    buffers: &RadarBuffers,
) -> Result<(), String> {
    let projection_tables = read_projection_tables(process, buffers.width, buffers.height);
    let color_tables = read_radar_color_tables(process, buffers.palette_base_id);
    let fullscreen_map_icons = read_fullscreen_map_icons(process);
    let fullscreen_map_layout = read_fullscreen_map_layout(process);
    write_json_line(
        writer,
        &json!({
            "record_kind": "radar_full_buffer_snapshot",
            "sample": boundary.sample,
            "elapsed_ms": boundary.elapsed_ms,
            "reason": boundary.reason,
            "trigger_sample": boundary.trigger_sample,
            "trigger_elapsed_ms": boundary.trigger_elapsed_ms,
            "summary": summarize_buffers(buffers, None, None),
            "terrain_buffer_hex": encode_hex(&buffers.terrain),
            "coverage_buffer_hex": encode_hex(&buffers.coverage),
            "radar_projection_tables": projection_tables,
            "radar_color_tables": color_tables,
            "fullscreen_map_icons": fullscreen_map_icons,
            "fullscreen_map_layout": fullscreen_map_layout,
        }),
    )
}

fn read_projection_tables(
    process: &Process,
    width: i32,
    height: i32,
) -> RadarProjectionResourcesEvidence {
    let roles = [
        "per-pixel globe base-shading palette index",
        "four signed source-coordinate pairs per half-resolution grid point",
        "four-neighbour blend/edge mask per half-resolution grid point",
    ];
    let dimensions = read_projection_dimensions(process, width, height);
    let lengths = dimensions.derived_payload_lengths;
    let (resource_table_pointer, table_error) =
        match process.read_u32(RADAR_PROJECTION_RESOURCE_TABLE_POINTER) {
            Ok(pointer) if plausible_heap_pointer(pointer as usize) => (Some(pointer), None),
            Ok(pointer) => (
                None,
                Some(format!(
                    "projection resource-table pointer {pointer:08X} is implausible"
                )),
            ),
            Err(error) => (
                None,
                Some(format!(
                    "projection resource-table pointer read failed: {error}"
                )),
            ),
        };
    let (resource_table, stable_table_bytes) =
        resource_table_pointer.map_or((None, None), |table| {
            let (window, bytes) = read_duplicate_window_with_bytes(
                process,
                table as usize,
                RADAR_PROJECTION_RESOURCE_COUNT * std::mem::size_of::<u32>(),
            );
            let bytes = window_is_stable_exact(
                &window,
                RADAR_PROJECTION_RESOURCE_COUNT * std::mem::size_of::<u32>(),
            )
            .then_some(bytes)
            .flatten();
            (Some(window), bytes)
        });
    let chains = (0..RADAR_PROJECTION_RESOURCE_COUNT)
        .map(|index| {
            let requested_bytes = lengths.and_then(|lengths| lengths.get(index).copied());
            let Some(table_pointer) = resource_table_pointer else {
                return RadarProjectionTableEvidence {
                    role: roles[index],
                    resource_table_pointer_global_address: RADAR_PROJECTION_RESOURCE_TABLE_POINTER
                        as u32,
                    resource_table_pointer,
                    table_entry_address: None,
                    resource_pointer: None,
                    resource_header: None,
                    resource_header_after_payload: None,
                    data_pointer_at_resource_0x18: None,
                    derived_requested_bytes: requested_bytes,
                    data: None,
                    read_error: table_error.clone(),
                };
            };
            let entry = projection_resource_entry_address(table_pointer, index);
            let resource_pointer = stable_table_bytes
                .as_deref()
                .and_then(|bytes| pointer_from_table_bytes(bytes, index));
            match resource_pointer {
                Some(resource_pointer) if plausible_heap_pointer(resource_pointer as usize) => {
                    let (resource_header, header_bytes) = read_duplicate_window_with_bytes(
                        process,
                        resource_pointer as usize,
                        RADAR_RESOURCE_HEADER_BYTES,
                    );
                    let stable_header_bytes = window_is_stable_exact(
                        &resource_header,
                        RADAR_RESOURCE_HEADER_BYTES,
                    )
                    .then_some(header_bytes)
                    .flatten();
                    let data_pointer = stable_header_bytes.as_deref().and_then(|bytes| {
                        u32_from_slice(bytes, RADAR_RESOURCE_DATA_POINTER_OFFSET)
                    });
                    match data_pointer {
                        Some(data_pointer) if plausible_data_pointer(data_pointer as usize) => {
                            let data = requested_bytes.map(|bytes| {
                                read_duplicate_window(process, data_pointer as usize, bytes)
                            });
                            RadarProjectionTableEvidence {
                                role: roles[index],
                                resource_table_pointer_global_address:
                                    RADAR_PROJECTION_RESOURCE_TABLE_POINTER as u32,
                                resource_table_pointer,
                                table_entry_address: Some(entry),
                                resource_pointer: Some(resource_pointer),
                                resource_header: Some(resource_header),
                                resource_header_after_payload: None,
                                data_pointer_at_resource_0x18: Some(data_pointer),
                                derived_requested_bytes: requested_bytes,
                                data,
                                read_error: if stable_header_bytes.is_none() {
                                    Some("resource header was not duplicate-stable".into())
                                } else {
                                    requested_bytes.is_none().then(|| {
                                        format!(
                                            "unverified or invalid radar dimensions {width}x{height}"
                                        )
                                    })
                                },
                            }
                        }
                        Some(data_pointer) => RadarProjectionTableEvidence {
                            role: roles[index],
                            resource_table_pointer_global_address:
                                RADAR_PROJECTION_RESOURCE_TABLE_POINTER as u32,
                            resource_table_pointer,
                            table_entry_address: Some(entry),
                            resource_pointer: Some(resource_pointer),
                            resource_header: Some(resource_header),
                            resource_header_after_payload: None,
                            data_pointer_at_resource_0x18: Some(data_pointer),
                            derived_requested_bytes: requested_bytes,
                            data: None,
                            read_error: Some(format!(
                                "resource data pointer {data_pointer:08X} is implausible"
                            )),
                        },
                        None => RadarProjectionTableEvidence {
                            role: roles[index],
                            resource_table_pointer_global_address:
                                RADAR_PROJECTION_RESOURCE_TABLE_POINTER as u32,
                            resource_table_pointer,
                            table_entry_address: Some(entry),
                            resource_pointer: Some(resource_pointer),
                            resource_header: Some(resource_header),
                            resource_header_after_payload: None,
                            data_pointer_at_resource_0x18: None,
                            derived_requested_bytes: requested_bytes,
                            data: None,
                            read_error: Some(
                                "resource data pointer could not be decoded from a stable header"
                                    .into(),
                            ),
                        },
                    }
                }
                Some(resource_pointer) => RadarProjectionTableEvidence {
                    role: roles[index],
                    resource_table_pointer_global_address: RADAR_PROJECTION_RESOURCE_TABLE_POINTER
                        as u32,
                    resource_table_pointer,
                    table_entry_address: Some(entry),
                    resource_pointer: Some(resource_pointer),
                    resource_header: None,
                    resource_header_after_payload: None,
                    data_pointer_at_resource_0x18: None,
                    derived_requested_bytes: requested_bytes,
                    data: None,
                    read_error: Some(format!(
                        "resource pointer {resource_pointer:08X} is implausible"
                    )),
                },
                None => RadarProjectionTableEvidence {
                    role: roles[index],
                    resource_table_pointer_global_address: RADAR_PROJECTION_RESOURCE_TABLE_POINTER
                        as u32,
                    resource_table_pointer,
                    table_entry_address: Some(entry),
                    resource_pointer: None,
                    resource_header: None,
                    resource_header_after_payload: None,
                    data_pointer_at_resource_0x18: None,
                    derived_requested_bytes: requested_bytes,
                    data: None,
                    read_error: Some(
                        "resource pointer could not be decoded from the stable outer table".into(),
                    ),
                },
            }
        })
        .collect::<Vec<_>>();
    let resource_table_pointer_after_payloads = process
        .read_u32(RADAR_PROJECTION_RESOURCE_TABLE_POINTER)
        .ok();
    let resource_table_after_payloads = resource_table_pointer_after_payloads
        .filter(|pointer| plausible_heap_pointer(*pointer as usize))
        .map(|pointer| {
            read_duplicate_window(
                process,
                pointer as usize,
                RADAR_PROJECTION_RESOURCE_COUNT * std::mem::size_of::<u32>(),
            )
        });
    let chains = chains
        .into_iter()
        .map(|mut chain| {
            chain.resource_header_after_payload = chain.resource_pointer.map(|pointer| {
                read_duplicate_window(process, pointer as usize, RADAR_RESOURCE_HEADER_BYTES)
            });
            chain
        })
        .collect();
    RadarProjectionResourcesEvidence {
        resource_table_pointer_global_address: RADAR_PROJECTION_RESOURCE_TABLE_POINTER as u32,
        resource_table_pointer,
        resource_table,
        resource_table_pointer_after_payloads,
        resource_table_after_payloads,
        dimensions,
        chains,
    }
}

fn read_projection_dimensions(
    process: &Process,
    width: i32,
    height: i32,
) -> RadarProjectionDimensionsEvidence {
    let mut validation_errors = Vec::new();
    let sprite_meta_pointer = match process.read_u32(SPRITE_META_POINTER_GLOBAL) {
        Ok(pointer) if plausible_heap_pointer(pointer as usize) => Some(pointer),
        Ok(pointer) => {
            validation_errors.push(format!("sprite-meta pointer {pointer:08X} is implausible"));
            None
        }
        Err(error) => {
            validation_errors.push(format!("sprite-meta pointer read failed: {error}"));
            None
        }
    };
    let (sprite_meta_header, stable_header_bytes) =
        sprite_meta_pointer.map_or((None, None), |pointer| {
            let (window, bytes) = read_duplicate_window_with_bytes(
                process,
                pointer as usize,
                RADAR_RESOURCE_HEADER_BYTES,
            );
            let stable = window_is_stable_exact(&window, RADAR_RESOURCE_HEADER_BYTES);
            if !stable {
                validation_errors.push(
                    "sprite-meta header was not duplicate-stable at the exact requested length"
                        .into(),
                );
            }
            (Some(window), stable.then_some(bytes).flatten())
        });
    let width_pointer = stable_header_bytes
        .as_deref()
        .and_then(|bytes| u32_from_slice(bytes, SPRITE_META_WIDTH_POINTER_OFFSET));
    let height_pointer = stable_header_bytes
        .as_deref()
        .and_then(|bytes| u32_from_slice(bytes, SPRITE_META_HEIGHT_POINTER_OFFSET));
    let (width_value, width_from_sprite_meta) = read_stable_i32_pointer_value(
        process,
        width_pointer,
        "sprite-meta width",
        &mut validation_errors,
    );
    let (height_value, height_from_sprite_meta) = read_stable_i32_pointer_value(
        process,
        height_pointer,
        "sprite-meta height",
        &mut validation_errors,
    );
    let dimensions_match_globals =
        width_from_sprite_meta == Some(width) && height_from_sprite_meta == Some(height);
    if !dimensions_match_globals {
        validation_errors.push(format!(
            "sprite-meta dimensions {:?}x{:?} do not match radar globals {width}x{height}",
            width_from_sprite_meta, height_from_sprite_meta
        ));
    }
    let derived_payload_lengths = dimensions_match_globals
        .then(|| projection_table_lengths(width, height))
        .flatten();
    if dimensions_match_globals && derived_payload_lengths.is_none() {
        validation_errors.push(format!(
            "radar dimensions {width}x{height} do not yield bounded projection payload lengths"
        ));
    }
    RadarProjectionDimensionsEvidence {
        width_global_at_0x004ef128: width,
        height_global_at_0x004df110: height,
        sprite_meta_pointer_global_address: SPRITE_META_POINTER_GLOBAL as u32,
        sprite_meta_pointer,
        sprite_meta_header,
        width_pointer_at_sprite_meta_0x18: width_pointer,
        height_pointer_at_sprite_meta_0x1c: height_pointer,
        width_value,
        height_value,
        width_from_sprite_meta,
        height_from_sprite_meta,
        dimensions_match_globals,
        derived_payload_lengths,
        validation_errors,
    }
}

fn read_stable_i32_pointer_value(
    process: &Process,
    pointer: Option<u32>,
    role: &str,
    validation_errors: &mut Vec<String>,
) -> (Option<DuplicateMemoryWindow>, Option<i32>) {
    let Some(pointer) = pointer else {
        validation_errors.push(format!("{role} pointer is unavailable"));
        return (None, None);
    };
    if !plausible_heap_pointer(pointer as usize) {
        validation_errors.push(format!("{role} pointer {pointer:08X} is implausible"));
        return (None, None);
    }
    let (window, bytes) = read_duplicate_window_with_bytes(process, pointer as usize, 4);
    let value = window_is_stable_exact(&window, 4)
        .then_some(bytes)
        .flatten()
        .as_deref()
        .and_then(|bytes| i32_from_slice(bytes, 0));
    if value.is_none() {
        validation_errors.push(format!(
            "{role} value was not duplicate-stable at the exact requested length"
        ));
    }
    (Some(window), value)
}

fn projection_validation_errors(evidence: &RadarProjectionResourcesEvidence) -> Vec<String> {
    let mut errors = evidence.dimensions.validation_errors.clone();
    if evidence.resource_table_pointer.is_none() {
        errors.push("projection outer-table pointer is unavailable".into());
    }
    match evidence.resource_table.as_ref() {
        Some(window)
            if window_is_stable_exact(
                window,
                RADAR_PROJECTION_RESOURCE_COUNT * std::mem::size_of::<u32>(),
            ) => {}
        _ => errors
            .push("projection outer table was not duplicate-stable at exactly 12 bytes".into()),
    }
    if evidence.resource_table_pointer_after_payloads != evidence.resource_table_pointer {
        errors.push(format!(
            "projection outer-table pointer changed across payload traversal: {:?} -> {:?}",
            evidence.resource_table_pointer, evidence.resource_table_pointer_after_payloads
        ));
    }
    if !stable_windows_equal_exact(
        evidence.resource_table.as_ref(),
        evidence.resource_table_after_payloads.as_ref(),
        RADAR_PROJECTION_RESOURCE_COUNT * std::mem::size_of::<u32>(),
    ) {
        errors.push("projection outer table changed across payload traversal".into());
    }
    if evidence.chains.len() != RADAR_PROJECTION_RESOURCE_COUNT {
        errors.push(format!(
            "expected {RADAR_PROJECTION_RESOURCE_COUNT} projection chains, captured {}",
            evidence.chains.len()
        ));
    }
    let expected_lengths = evidence.dimensions.derived_payload_lengths;
    for (index, chain) in evidence.chains.iter().enumerate() {
        let label = format!("projection chain {index} ({})", chain.role);
        if let Some(error) = &chain.read_error {
            errors.push(format!("{label}: {error}"));
        }
        if chain.resource_pointer.is_none() {
            errors.push(format!("{label}: resource pointer is unavailable"));
        }
        if !chain
            .resource_header
            .as_ref()
            .is_some_and(|window| window_is_stable_exact(window, RADAR_RESOURCE_HEADER_BYTES))
        {
            errors.push(format!(
                "{label}: resource header is not duplicate-stable at exactly {RADAR_RESOURCE_HEADER_BYTES} bytes"
            ));
        }
        if !stable_windows_equal_exact(
            chain.resource_header.as_ref(),
            chain.resource_header_after_payload.as_ref(),
            RADAR_RESOURCE_HEADER_BYTES,
        ) {
            errors.push(format!(
                "{label}: resource header changed across payload traversal"
            ));
        }
        if chain.data_pointer_at_resource_0x18.is_none() {
            errors.push(format!("{label}: data pointer is unavailable"));
        }
        let expected = expected_lengths.and_then(|lengths| lengths.get(index).copied());
        if chain.derived_requested_bytes != expected || expected.is_none() {
            errors.push(format!(
                "{label}: requested payload length {:?} does not equal verified expected length {:?}",
                chain.derived_requested_bytes, expected
            ));
        }
        if !chain
            .data
            .as_ref()
            .zip(expected)
            .is_some_and(|(window, bytes)| window_is_stable_exact(window, bytes))
        {
            errors.push(format!(
                "{label}: payload is not duplicate-stable at exactly {:?} bytes",
                expected
            ));
        }
    }
    errors
}

fn window_is_stable_exact(window: &DuplicateMemoryWindow, expected_bytes: usize) -> bool {
    window.requested_bytes == expected_bytes
        && window.duplicate_stable == Some(true)
        && window.read_error.is_none()
        && window
            .raw_hex
            .as_ref()
            .is_some_and(|raw| raw.len() == expected_bytes.saturating_mul(2))
}

fn stable_windows_equal_exact(
    before: Option<&DuplicateMemoryWindow>,
    after: Option<&DuplicateMemoryWindow>,
    expected_bytes: usize,
) -> bool {
    before.zip(after).is_some_and(|(before, after)| {
        window_is_stable_exact(before, expected_bytes)
            && window_is_stable_exact(after, expected_bytes)
            && before.raw_hex == after.raw_hex
    })
}

fn projection_resource_entry_address(table_pointer: u32, index: usize) -> u32 {
    table_pointer.wrapping_add((index * std::mem::size_of::<u32>()) as u32)
}

fn projection_table_lengths(width: i32, height: i32) -> Option<[usize; 3]> {
    const MAX_CAPTURE_BYTES: usize = 16 * 1024 * 1024;
    let width = usize::try_from(width).ok().filter(|value| *value > 0)?;
    let height = usize::try_from(height).ok().filter(|value| *value > 0)?;
    let pixels = width.checked_mul(height)?;
    let half_grid_cells = width.div_ceil(2).checked_mul(height.div_ceil(2))?;
    let half_grid_offsets = half_grid_cells.checked_mul(16)?;
    (pixels <= MAX_CAPTURE_BYTES && half_grid_offsets <= MAX_CAPTURE_BYTES).then_some([
        pixels,
        half_grid_offsets,
        half_grid_cells,
    ])
}

fn read_radar_color_tables(process: &Process, palette_base_id: i32) -> RadarColorTablesEvidence {
    let mut warnings = Vec::new();
    let pointer_table_pointer = read_pointer_table_global(
        process,
        MASTER_COLOR_POINTER_TABLE,
        "master-color pointer table",
        &mut warnings,
    );
    let Some(table) = pointer_table_pointer else {
        return RadarColorTablesEvidence {
            pointer_table_global_address: MASTER_COLOR_POINTER_TABLE as u32,
            pointer_table_pointer,
            pointer_table_pointer_after_entries: None,
            palette_base_id_at_0x004df114: palette_base_id,
            terrain_palette: Vec::new(),
            globe_shading_palette: Vec::new(),
            marker_palette: Vec::new(),
            warnings,
        };
    };
    let Some(palette_base_offset) = usize::try_from(palette_base_id)
        .ok()
        .and_then(|index| index.checked_mul(4))
    else {
        warnings.push(format!(
            "palette base ID {palette_base_id} cannot address the color table"
        ));
        return RadarColorTablesEvidence {
            pointer_table_global_address: MASTER_COLOR_POINTER_TABLE as u32,
            pointer_table_pointer,
            pointer_table_pointer_after_entries: None,
            palette_base_id_at_0x004df114: palette_base_id,
            terrain_palette: Vec::new(),
            globe_shading_palette: Vec::new(),
            marker_palette: Vec::new(),
            warnings,
        };
    };

    let mut terrain_palette = (0..32)
        .map(|index| {
            read_color_evidence(
                process,
                table,
                palette_base_offset + index * 4,
                format!("terrain_raster_index_{index}"),
            )
        })
        .collect::<Vec<_>>();
    for (index, offset) in [(32, 0x7C), (33, 0x88), (34, 0x84)] {
        terrain_palette.push(read_color_evidence(
            process,
            table,
            offset,
            format!("terrain_raster_index_{index}_special"),
        ));
    }

    let globe_shading_palette = (0..16)
        .map(|index| {
            read_color_evidence(
                process,
                table,
                0x80 + palette_base_offset + index * 4,
                format!("globe_base_shading_index_{index}"),
            )
        })
        .collect();
    let marker_palette = [
        (0x60, "marker_flags_0x0001"),
        (0x64, "marker_flags_0x0008"),
        (0x68, "marker_flags_0x0200"),
        (0x70, "marker_flags_0x00a0"),
        (0x74, "marker_flags_0x0010"),
        (0x78, "marker_flags_0x0004_or_0x8000"),
        (0x7C, "marker_flags_0x0c00"),
    ]
    .into_iter()
    .map(|(offset, role)| {
        read_color_evidence(
            process,
            table,
            offset + palette_base_offset,
            role.to_owned(),
        )
    })
    .collect();

    RadarColorTablesEvidence {
        pointer_table_global_address: MASTER_COLOR_POINTER_TABLE as u32,
        pointer_table_pointer,
        pointer_table_pointer_after_entries: process.read_u32(MASTER_COLOR_POINTER_TABLE).ok(),
        palette_base_id_at_0x004df114: palette_base_id,
        terrain_palette,
        globe_shading_palette,
        marker_palette,
        warnings,
    }
}

fn read_color_evidence(
    process: &Process,
    pointer_table: u32,
    relative_slot_offset: usize,
    role: String,
) -> RadarColorEvidence {
    let entry = pointer_table.wrapping_add(relative_slot_offset as u32);
    let (pointer_table_entry, entry_bytes) =
        read_duplicate_window_with_bytes(process, entry as usize, 4);
    let color_pointer = window_is_stable_exact(&pointer_table_entry, 4)
        .then_some(entry_bytes)
        .flatten()
        .as_deref()
        .and_then(|bytes| u32_from_slice(bytes, 0));
    match color_pointer {
        Some(color_pointer) if plausible_heap_pointer(color_pointer as usize) => {
            let (color_value, bytes) =
                read_duplicate_window_with_bytes(process, color_pointer as usize, 4);
            let packed_color_u32 = window_is_stable_exact(&color_value, 4)
                .then_some(bytes)
                .flatten()
                .as_deref()
                .and_then(|bytes| u32_from_slice(bytes, 0));
            RadarColorEvidence {
                role,
                relative_slot_offset: relative_slot_offset as u32,
                pointer_table_entry_address: Some(entry),
                pointer_table_entry: Some(pointer_table_entry),
                color_pointer: Some(color_pointer),
                packed_color_u32,
                packed_color_u16: packed_color_u32.map(|value| value as u16),
                color_value: Some(color_value),
                read_error: None,
            }
        }
        Some(color_pointer) => RadarColorEvidence {
            role,
            relative_slot_offset: relative_slot_offset as u32,
            pointer_table_entry_address: Some(entry),
            pointer_table_entry: Some(pointer_table_entry),
            color_pointer: Some(color_pointer),
            packed_color_u32: None,
            packed_color_u16: None,
            color_value: None,
            read_error: Some(format!("color pointer {color_pointer:08X} is implausible")),
        },
        None => RadarColorEvidence {
            role,
            relative_slot_offset: relative_slot_offset as u32,
            pointer_table_entry_address: Some(entry),
            pointer_table_entry: Some(pointer_table_entry),
            color_pointer: None,
            packed_color_u32: None,
            packed_color_u16: None,
            color_value: None,
            read_error: Some(
                "color pointer could not be decoded from a stable pointer-table entry".into(),
            ),
        },
    }
}

fn color_validation_errors(evidence: &RadarColorTablesEvidence) -> Vec<String> {
    let mut errors = evidence.warnings.clone();
    if evidence.pointer_table_pointer.is_none() {
        errors.push("radar master-color pointer table is unavailable".into());
    }
    if evidence.pointer_table_pointer_after_entries != evidence.pointer_table_pointer {
        errors.push(format!(
            "radar master-color pointer changed across entry traversal: {:?} -> {:?}",
            evidence.pointer_table_pointer, evidence.pointer_table_pointer_after_entries
        ));
    }
    for (label, entries, expected_count) in [
        (
            "terrain palette",
            evidence.terrain_palette.as_slice(),
            TERRAIN_PALETTE_ENTRY_COUNT,
        ),
        (
            "globe-shading palette",
            evidence.globe_shading_palette.as_slice(),
            GLOBE_SHADING_PALETTE_ENTRY_COUNT,
        ),
        (
            "marker palette",
            evidence.marker_palette.as_slice(),
            MARKER_PALETTE_ENTRY_COUNT,
        ),
    ] {
        if entries.len() != expected_count {
            errors.push(format!(
                "{label} contains {} requested entries instead of {expected_count}",
                entries.len()
            ));
        }
        for entry in entries {
            let role = format!(
                "{label} entry {} ({})",
                entry.relative_slot_offset, entry.role
            );
            if let Some(error) = &entry.read_error {
                errors.push(format!("{role}: {error}"));
            }
            if !entry
                .pointer_table_entry
                .as_ref()
                .is_some_and(|window| window_is_stable_exact(window, 4))
            {
                errors.push(format!("{role}: pointer-table entry is not stable"));
            }
            if entry.color_pointer.is_none()
                || entry.packed_color_u32.is_none()
                || !entry
                    .color_value
                    .as_ref()
                    .is_some_and(|window| window_is_stable_exact(window, 4))
            {
                errors.push(format!("{role}: packed color value is not stable"));
            }
        }
    }
    errors
}

fn read_fullscreen_map_icons(process: &Process) -> FullscreenMapIconTableEvidence {
    let mut warnings = Vec::new();
    let type_table = read_pointer_table_global(
        process,
        ENTITY_TYPE_POINTER_TABLE,
        "entity-type pointer table",
        &mut warnings,
    );
    let sprite_table = read_pointer_table_global(
        process,
        GLOBAL_SPRITE_POINTER_TABLE,
        "global-sprite pointer table",
        &mut warnings,
    );
    let (type_table_window, type_table_bytes) = type_table.map_or((None, None), |pointer| {
        let (window, bytes) = read_duplicate_window_with_bytes(
            process,
            pointer as usize,
            ENTITY_TYPE_COUNT * std::mem::size_of::<u32>(),
        );
        let bytes = window_is_stable_exact(&window, ENTITY_TYPE_COUNT * std::mem::size_of::<u32>())
            .then_some(bytes)
            .flatten();
        (Some(window), bytes)
    });
    let mut loaded_type_records = 0;
    let mut unloaded_type_slots = 0;
    let mut nonzero_icon_selectors = 0;
    let mut types = Vec::new();
    if let Some(type_pointers) = type_table_bytes.as_deref() {
        for entity_type in 0..ENTITY_TYPE_COUNT {
            let Some(type_record) = pointer_from_table_bytes(type_pointers, entity_type) else {
                warnings.push(format!(
                    "entity-type pointer table ended before type {entity_type}"
                ));
                break;
            };
            if type_record == 0 {
                unloaded_type_slots += 1;
                types.push(FullscreenMapIconTypeEvidence {
                    entity_type: entity_type as u32,
                    slot_state: "unloaded_type_slot",
                    type_record_pointer: Some(0),
                    selector_value: None,
                    selector_at_type_record_0x70: None,
                    sprite_pointer_table_entry_address: None,
                    sprite_pointer_table_entry: None,
                    sprite_resource_pointer: None,
                    sprite_width_at_resource_0x10: None,
                    sprite_height_at_resource_0x12: None,
                    sprite_resource_header: None,
                    read_error: None,
                });
                continue;
            }
            if !plausible_heap_pointer(type_record as usize) {
                types.push(FullscreenMapIconTypeEvidence {
                    entity_type: entity_type as u32,
                    slot_state: "error",
                    type_record_pointer: Some(type_record),
                    selector_value: None,
                    selector_at_type_record_0x70: None,
                    sprite_pointer_table_entry_address: None,
                    sprite_pointer_table_entry: None,
                    sprite_resource_pointer: None,
                    sprite_width_at_resource_0x10: None,
                    sprite_height_at_resource_0x12: None,
                    sprite_resource_header: None,
                    read_error: Some(format!(
                        "type-record pointer {type_record:08X} is implausible"
                    )),
                });
                continue;
            }
            loaded_type_records += 1;
            let (selector_value, selector_bytes) = read_duplicate_window_with_bytes(
                process,
                type_record as usize + FULLSCREEN_MAP_ICON_SELECTOR_OFFSET,
                2,
            );
            let selector = window_is_stable_exact(&selector_value, 2)
                .then_some(selector_bytes)
                .flatten()
                .as_deref()
                .and_then(|bytes| u16_from_slice(bytes, 0));
            let Some(selector) = selector else {
                types.push(FullscreenMapIconTypeEvidence {
                    entity_type: entity_type as u32,
                    slot_state: "error",
                    type_record_pointer: Some(type_record),
                    selector_value: Some(selector_value),
                    selector_at_type_record_0x70: None,
                    sprite_pointer_table_entry_address: None,
                    sprite_pointer_table_entry: None,
                    sprite_resource_pointer: None,
                    sprite_width_at_resource_0x10: None,
                    sprite_height_at_resource_0x12: None,
                    sprite_resource_header: None,
                    read_error: Some(
                        "icon selector was not duplicate-stable at exactly 2 bytes".into(),
                    ),
                });
                continue;
            };
            if selector == 0 {
                types.push(FullscreenMapIconTypeEvidence {
                    entity_type: entity_type as u32,
                    slot_state: "loaded_no_icon",
                    type_record_pointer: Some(type_record),
                    selector_value: Some(selector_value),
                    selector_at_type_record_0x70: Some(selector),
                    sprite_pointer_table_entry_address: None,
                    sprite_pointer_table_entry: None,
                    sprite_resource_pointer: None,
                    sprite_width_at_resource_0x10: None,
                    sprite_height_at_resource_0x12: None,
                    sprite_resource_header: None,
                    read_error: None,
                });
                continue;
            }
            nonzero_icon_selectors += 1;
            types.push(read_fullscreen_map_icon_type(
                process,
                sprite_table,
                entity_type as u32,
                type_record,
                selector,
                selector_value,
            ));
        }
    }
    let entity_type_pointer_table_after_types_pointer =
        process.read_u32(ENTITY_TYPE_POINTER_TABLE).ok();
    let entity_type_pointer_table_after_types = entity_type_pointer_table_after_types_pointer
        .filter(|pointer| plausible_heap_pointer(*pointer as usize))
        .map(|pointer| {
            read_duplicate_window(
                process,
                pointer as usize,
                ENTITY_TYPE_COUNT * std::mem::size_of::<u32>(),
            )
        });
    FullscreenMapIconTableEvidence {
        entity_type_pointer_table_global_address: ENTITY_TYPE_POINTER_TABLE as u32,
        entity_type_pointer_table: type_table,
        entity_type_pointer_table_window: type_table_window,
        entity_type_pointer_table_after_types,
        entity_type_pointer_table_after_types_pointer,
        sprite_pointer_table_global_address: GLOBAL_SPRITE_POINTER_TABLE as u32,
        sprite_pointer_table: sprite_table,
        sprite_pointer_table_after_types: process.read_u32(GLOBAL_SPRITE_POINTER_TABLE).ok(),
        unloaded_type_slots,
        loaded_type_records,
        nonzero_icon_selectors,
        types,
        warnings,
    }
}

fn read_fullscreen_map_icon_type(
    process: &Process,
    sprite_table: Option<u32>,
    entity_type: u32,
    type_record: u32,
    selector: u16,
    selector_value: DuplicateMemoryWindow,
) -> FullscreenMapIconTypeEvidence {
    let Some(sprite_table) = sprite_table else {
        return FullscreenMapIconTypeEvidence {
            entity_type,
            slot_state: "error",
            type_record_pointer: Some(type_record),
            selector_value: Some(selector_value),
            selector_at_type_record_0x70: Some(selector),
            sprite_pointer_table_entry_address: None,
            sprite_pointer_table_entry: None,
            sprite_resource_pointer: None,
            sprite_width_at_resource_0x10: None,
            sprite_height_at_resource_0x12: None,
            sprite_resource_header: None,
            read_error: Some("global-sprite pointer table is unavailable".into()),
        };
    };
    let entry = sprite_table.wrapping_add(u32::from(selector) * 4);
    let (sprite_pointer_table_entry, entry_bytes) =
        read_duplicate_window_with_bytes(process, entry as usize, 4);
    let resource = window_is_stable_exact(&sprite_pointer_table_entry, 4)
        .then_some(entry_bytes)
        .flatten()
        .as_deref()
        .and_then(|bytes| u32_from_slice(bytes, 0));
    match resource {
        Some(resource) if plausible_heap_pointer(resource as usize) => {
            let (header, bytes) = read_duplicate_window_with_bytes(
                process,
                resource as usize,
                FULLSCREEN_MAP_ICON_HEADER_BYTES,
            );
            let stable_bytes = window_is_stable_exact(&header, FULLSCREEN_MAP_ICON_HEADER_BYTES)
                .then_some(bytes)
                .flatten();
            FullscreenMapIconTypeEvidence {
                entity_type,
                slot_state: "loaded_icon",
                type_record_pointer: Some(type_record),
                selector_value: Some(selector_value),
                selector_at_type_record_0x70: Some(selector),
                sprite_pointer_table_entry_address: Some(entry),
                sprite_pointer_table_entry: Some(sprite_pointer_table_entry),
                sprite_resource_pointer: Some(resource),
                sprite_width_at_resource_0x10: stable_bytes
                    .as_deref()
                    .and_then(|bytes| u16_from_slice(bytes, 0x10)),
                sprite_height_at_resource_0x12: stable_bytes
                    .as_deref()
                    .and_then(|bytes| u16_from_slice(bytes, 0x12)),
                sprite_resource_header: Some(header),
                read_error: stable_bytes.is_none().then(|| {
                    "fullscreen-map sprite header was not duplicate-stable at exactly 32 bytes"
                        .into()
                }),
            }
        }
        Some(resource) => FullscreenMapIconTypeEvidence {
            entity_type,
            slot_state: "error",
            type_record_pointer: Some(type_record),
            selector_value: Some(selector_value),
            selector_at_type_record_0x70: Some(selector),
            sprite_pointer_table_entry_address: Some(entry),
            sprite_pointer_table_entry: Some(sprite_pointer_table_entry),
            sprite_resource_pointer: Some(resource),
            sprite_width_at_resource_0x10: None,
            sprite_height_at_resource_0x12: None,
            sprite_resource_header: None,
            read_error: Some(format!(
                "fullscreen-map sprite resource pointer {resource:08X} is implausible"
            )),
        },
        None => FullscreenMapIconTypeEvidence {
            entity_type,
            slot_state: "error",
            type_record_pointer: Some(type_record),
            selector_value: Some(selector_value),
            selector_at_type_record_0x70: Some(selector),
            sprite_pointer_table_entry_address: Some(entry),
            sprite_pointer_table_entry: Some(sprite_pointer_table_entry),
            sprite_resource_pointer: None,
            sprite_width_at_resource_0x10: None,
            sprite_height_at_resource_0x12: None,
            sprite_resource_header: None,
            read_error: Some(
                "sprite resource pointer could not be decoded from a stable table entry".into(),
            ),
        },
    }
}

fn fullscreen_map_icon_validation_errors(evidence: &FullscreenMapIconTableEvidence) -> Vec<String> {
    let mut errors = evidence.warnings.clone();
    if evidence.entity_type_pointer_table.is_none() {
        errors.push("fullscreen-map entity-type pointer table is unavailable".into());
    }
    if evidence.entity_type_pointer_table_after_types_pointer != evidence.entity_type_pointer_table
    {
        errors.push(format!(
            "entity-type pointer table changed across icon traversal: {:?} -> {:?}",
            evidence.entity_type_pointer_table,
            evidence.entity_type_pointer_table_after_types_pointer
        ));
    }
    if !stable_windows_equal_exact(
        evidence.entity_type_pointer_table_window.as_ref(),
        evidence.entity_type_pointer_table_after_types.as_ref(),
        ENTITY_TYPE_COUNT * std::mem::size_of::<u32>(),
    ) {
        errors.push("entity-type pointer table changed across icon traversal".into());
    }
    if evidence.sprite_pointer_table.is_none() {
        errors.push("fullscreen-map global-sprite pointer table is unavailable".into());
    }
    if evidence.sprite_pointer_table_after_types != evidence.sprite_pointer_table {
        errors.push(format!(
            "global-sprite pointer table changed across icon traversal: {:?} -> {:?}",
            evidence.sprite_pointer_table, evidence.sprite_pointer_table_after_types
        ));
    }
    if evidence.types.len() != ENTITY_TYPE_COUNT {
        errors.push(format!(
            "fullscreen-map type catalog contains {} slots instead of {ENTITY_TYPE_COUNT}",
            evidence.types.len()
        ));
    }
    if evidence.loaded_type_records == 0 {
        errors.push("fullscreen-map type catalog contains no loaded records".into());
    }
    if evidence.nonzero_icon_selectors == 0 {
        errors.push("fullscreen-map type catalog contains no nonzero icon selectors".into());
    }
    let counted_unloaded = evidence
        .types
        .iter()
        .filter(|entry| entry.slot_state == "unloaded_type_slot")
        .count();
    if counted_unloaded != evidence.unloaded_type_slots {
        errors.push(format!(
            "fullscreen-map unloaded-slot count {} does not match catalog count {counted_unloaded}",
            evidence.unloaded_type_slots
        ));
    }
    for entry in &evidence.types {
        let label = format!("fullscreen-map entity type {}", entry.entity_type);
        if let Some(error) = &entry.read_error {
            errors.push(format!("{label}: {error}"));
        }
        match entry.slot_state {
            "unloaded_type_slot" => {
                if entry.type_record_pointer != Some(0) || entry.read_error.is_some() {
                    errors.push(format!("{label}: malformed unloaded slot evidence"));
                }
            }
            "loaded_no_icon" => {
                if entry.selector_at_type_record_0x70 != Some(0)
                    || !entry
                        .selector_value
                        .as_ref()
                        .is_some_and(|window| window_is_stable_exact(window, 2))
                {
                    errors.push(format!("{label}: zero icon selector is not stable"));
                }
            }
            "loaded_icon" => {
                if entry.selector_at_type_record_0x70 == Some(0)
                    || !entry
                        .selector_value
                        .as_ref()
                        .is_some_and(|window| window_is_stable_exact(window, 2))
                {
                    errors.push(format!("{label}: nonzero icon selector is not stable"));
                }
                if !entry
                    .sprite_pointer_table_entry
                    .as_ref()
                    .is_some_and(|window| window_is_stable_exact(window, 4))
                {
                    errors.push(format!("{label}: sprite pointer-table entry is not stable"));
                }
                if entry.sprite_resource_pointer.is_none()
                    || !entry.sprite_resource_header.as_ref().is_some_and(|window| {
                        window_is_stable_exact(window, FULLSCREEN_MAP_ICON_HEADER_BYTES)
                    })
                {
                    errors.push(format!("{label}: sprite resource header is not stable"));
                }
                if !entry
                    .sprite_width_at_resource_0x10
                    .zip(entry.sprite_height_at_resource_0x12)
                    .is_some_and(|(width, height)| width > 0 && height > 0)
                {
                    errors.push(format!("{label}: sprite dimensions are absent or zero"));
                }
            }
            "error" => {
                if entry.read_error.is_none() {
                    errors.push(format!("{label}: error slot lacks a diagnostic"));
                }
            }
            state => errors.push(format!("{label}: unknown slot state {state}")),
        }
    }
    errors
}

fn read_pointer_table_global(
    process: &Process,
    global: usize,
    role: &str,
    warnings: &mut Vec<String>,
) -> Option<u32> {
    match process.read_u32(global) {
        Ok(pointer) if plausible_heap_pointer(pointer as usize) => Some(pointer),
        Ok(pointer) => {
            warnings.push(format!("{role} pointer {pointer:08X} is implausible"));
            None
        }
        Err(error) => {
            warnings.push(format!("{role} pointer read failed: {error}"));
            None
        }
    }
}

fn pointer_from_table_bytes(bytes: &[u8], index: usize) -> Option<u32> {
    u32_from_slice(bytes, index.checked_mul(std::mem::size_of::<u32>())?)
}

fn read_fullscreen_map_layout(process: &Process) -> FullscreenMapLayoutEvidence {
    let mut warnings = Vec::new();
    let sprite_meta_pointer = match process.read_u32(SPRITE_META_POINTER_GLOBAL) {
        Ok(pointer) if plausible_heap_pointer(pointer as usize) => Some(pointer),
        Ok(pointer) => {
            warnings.push(format!("sprite-meta pointer {pointer:08X} is implausible"));
            None
        }
        Err(error) => {
            warnings.push(format!("sprite-meta pointer read failed: {error}"));
            None
        }
    };
    let (layout, stable_layout_bytes) = sprite_meta_pointer.map_or((None, None), |pointer| {
        let (window, bytes) =
            read_duplicate_window_with_bytes(process, pointer as usize, SPRITE_META_LAYOUT_BYTES);
        let bytes = window_is_stable_exact(&window, SPRITE_META_LAYOUT_BYTES)
            .then_some(bytes)
            .flatten();
        (Some(window), bytes)
    });
    let pointer_at = |offset: usize| {
        stable_layout_bytes
            .as_deref()
            .and_then(|bytes| u32_from_slice(bytes, offset))
    };
    let origin_x_pointer = pointer_at(0x30);
    let origin_y_pointer = pointer_at(0x34);
    let width_pointer = pointer_at(0x38);
    let height_pointer = pointer_at(0x3C);
    let (origin_x_value, origin_x_bytes) = read_stable_indirect_value(
        process,
        origin_x_pointer,
        "fullscreen-map origin X",
        &mut warnings,
    );
    let (origin_y_value, origin_y_bytes) = read_stable_indirect_value(
        process,
        origin_y_pointer,
        "fullscreen-map origin Y",
        &mut warnings,
    );
    let (width_value, width_bytes) = read_stable_indirect_value(
        process,
        width_pointer,
        "fullscreen-map width",
        &mut warnings,
    );
    let (height_value, height_bytes) = read_stable_indirect_value(
        process,
        height_pointer,
        "fullscreen-map height",
        &mut warnings,
    );
    FullscreenMapLayoutEvidence {
        sprite_meta_pointer_global_address: SPRITE_META_POINTER_GLOBAL as u32,
        sprite_meta_pointer,
        sprite_meta_pointer_after_values: process.read_u32(SPRITE_META_POINTER_GLOBAL).ok(),
        sprite_meta_layout: layout,
        origin_x_pointer_at_0x30: origin_x_pointer,
        origin_y_pointer_at_0x34: origin_y_pointer,
        width_pointer_at_0x38: width_pointer,
        height_pointer_at_0x3c: height_pointer,
        origin_x_value,
        origin_y_value,
        width_value,
        height_value,
        origin_x_from_pointer_at_0x30: origin_x_bytes
            .as_deref()
            .and_then(|bytes| i16_from_slice(bytes, 0)),
        origin_y_from_pointer_at_0x34: origin_y_bytes
            .as_deref()
            .and_then(|bytes| i16_from_slice(bytes, 0)),
        width_from_pointer_at_0x38: width_bytes
            .as_deref()
            .and_then(|bytes| i32_from_slice(bytes, 0)),
        height_from_pointer_at_0x3c: height_bytes
            .as_deref()
            .and_then(|bytes| i32_from_slice(bytes, 0)),
        warnings,
    }
}

fn read_stable_indirect_value(
    process: &Process,
    pointer: Option<u32>,
    role: &str,
    warnings: &mut Vec<String>,
) -> (Option<DuplicateMemoryWindow>, Option<Vec<u8>>) {
    let Some(pointer) = pointer else {
        warnings.push(format!("{role} pointer is unavailable"));
        return (None, None);
    };
    if !plausible_heap_pointer(pointer as usize) {
        warnings.push(format!("{role} pointer {pointer:08X} is implausible"));
        return (None, None);
    }
    let (window, bytes) = read_duplicate_window_with_bytes(process, pointer as usize, 4);
    let stable_bytes = window_is_stable_exact(&window, 4)
        .then_some(bytes)
        .flatten();
    if stable_bytes.is_none() {
        warnings.push(format!("{role} value is not duplicate-stable"));
    }
    (Some(window), stable_bytes)
}

fn fullscreen_map_layout_validation_errors(evidence: &FullscreenMapLayoutEvidence) -> Vec<String> {
    let mut errors = evidence.warnings.clone();
    if evidence.sprite_meta_pointer.is_none() {
        errors.push("fullscreen-map sprite-meta pointer is unavailable".into());
    }
    if evidence.sprite_meta_pointer_after_values != evidence.sprite_meta_pointer {
        errors.push(format!(
            "fullscreen-map sprite-meta pointer changed across layout reads: {:?} -> {:?}",
            evidence.sprite_meta_pointer, evidence.sprite_meta_pointer_after_values
        ));
    }
    if !evidence
        .sprite_meta_layout
        .as_ref()
        .is_some_and(|window| window_is_stable_exact(window, SPRITE_META_LAYOUT_BYTES))
    {
        errors.push("fullscreen-map sprite-meta layout is not stable".into());
    }
    for (role, pointer, value) in [
        (
            "origin X",
            evidence.origin_x_pointer_at_0x30,
            evidence.origin_x_value.as_ref(),
        ),
        (
            "origin Y",
            evidence.origin_y_pointer_at_0x34,
            evidence.origin_y_value.as_ref(),
        ),
        (
            "width",
            evidence.width_pointer_at_0x38,
            evidence.width_value.as_ref(),
        ),
        (
            "height",
            evidence.height_pointer_at_0x3c,
            evidence.height_value.as_ref(),
        ),
    ] {
        if pointer.is_none() || !value.is_some_and(|window| window_is_stable_exact(window, 4)) {
            errors.push(format!(
                "fullscreen-map {role} pointer/value evidence is incomplete"
            ));
        }
    }
    if evidence.origin_x_from_pointer_at_0x30.is_none()
        || evidence.origin_y_from_pointer_at_0x34.is_none()
    {
        errors.push("fullscreen-map origin values are unavailable".into());
    }
    if !evidence
        .width_from_pointer_at_0x38
        .zip(evidence.height_from_pointer_at_0x3c)
        .is_some_and(|(width, height)| width > 0 && height > 0)
    {
        errors.push("fullscreen-map dimensions are unavailable or non-positive".into());
    }
    errors
}

fn radar_entities(
    snapshot: &EntitySnapshot,
    player: Option<&EntityRecord>,
    buffers: &RadarBuffers,
) -> (Vec<RadarEntityEvidence>, Vec<String>) {
    let records = snapshot
        .records
        .iter()
        .map(|record| {
            let marker_flags = record.radar_marker_flags_at_0x64;
            let relative_x = player
                .map(|player| record.position_raw_8_8[0].wrapping_sub(player.position_raw_8_8[0]));
            let relative_z = player
                .map(|player| record.position_raw_8_8[2].wrapping_sub(player.position_raw_8_8[2]));
            let distance_input = relative_x.zip(relative_z).map(|(x, z)| {
                let x = i64::from(x);
                let z = i64::from(z);
                (((x * x) >> 2) + ((z * z) >> 2)) as u32
            });
            let distance = distance_input.map(|value| integer_sqrt(u64::from(value)) as u32 * 2);
            let cell = cell_evidence(record.position_raw_8_8, buffers);
            let rejected_by_initial_entity_gate =
                (record.flags & 0x4000 != 0 || record.flags == 0) && marker_flags & 0x10 == 0;
            let has_any_supported_marker_flag = marker_flags & 0x8EBD != 0;
            let hidden_random_gate = cell.terrain_coverage_nibble == 0 && record.entity_type != 46;
            let passes_pre_radius_flag_gates =
                !rejected_by_initial_entity_gate && has_any_supported_marker_flag;
            RadarEntityEvidence {
                intrusive_list_index: record.intrusive_list_index,
                pointer: record.pointer,
                handle: record.handle,
                entity_type: record.entity_type,
                flags_at_entity_0x08: record.flags,
                marker_flags_at_entity_0x64: marker_flags,
                position_raw_8_8: record.position_raw_8_8,
                relative_x_raw_wrapped: relative_x,
                relative_z_raw_wrapped: relative_z,
                radial_distance_input_after_quartering: distance_input,
                radial_distance_raw_approx: distance,
                inside_retail_0x4000_marker_radius: distance.map(|value| value < 0x4000),
                cell,
                rejected_by_initial_entity_gate,
                has_any_supported_marker_flag_0x8ebd: has_any_supported_marker_flag,
                zero_coverage_random_visibility_gate_applies: hidden_random_gate,
                passes_pre_radius_flag_gates,
            }
        })
        .collect();
    (records, Vec::new())
}

fn select_player(snapshot: &EntitySnapshot) -> Option<&EntityRecord> {
    snapshot
        .records
        .iter()
        .find(|record| record.entity_type == 46)
}

fn player_evidence(player: &EntityRecord, buffers: &RadarBuffers) -> RadarPlayerEvidence {
    RadarPlayerEvidence {
        pointer: player.pointer,
        handle: player.handle,
        position_raw_8_8: player.position_raw_8_8,
        velocity_raw_8_8: player.velocity_raw_8_8,
        rotation: player.rotation,
        body_basis_raw: player.body_basis_raw,
        cell: cell_evidence(player.position_raw_8_8, buffers),
    }
}

fn cell_evidence(position: [i16; 3], buffers: &RadarBuffers) -> RadarCellEvidence {
    let x = (position[0] as u16 >> 8) as u8;
    let z = (position[2] as u16 >> 8) as u8;
    let linear_index = u32::from(z) * 256 + u32::from(x);
    let terrain_raster_value = buffers.terrain[linear_index as usize];
    let packed = buffers.coverage[linear_index as usize / 2];
    let terrain_coverage_nibble = if linear_index & 1 == 0 {
        packed & 0x0F
    } else {
        packed >> 4
    };
    RadarCellEvidence {
        x,
        z,
        linear_index,
        terrain_raster_value,
        terrain_coverage_nibble,
    }
}

fn read_runtime_windows(process: &Process) -> RuntimeRadarWindows {
    let mut warnings = Vec::new();
    let tick_before = process.read_u32(RETAIL_FRAME_COUNTER).ok();
    let session_pointer = match process.read_u32(SESSION_POINTER_GLOBAL) {
        Ok(pointer) if plausible_heap_pointer(pointer as usize) => Some(pointer),
        Ok(pointer) => {
            warnings.push(format!("session pointer {pointer:08X} is implausible"));
            None
        }
        Err(error) => {
            warnings.push(format!("session pointer read failed: {error}"));
            None
        }
    };
    let (session, session_bytes) = session_pointer.map_or((None, None), |pointer| {
        let (window, bytes) = read_duplicate_window_with_bytes(
            process,
            pointer as usize,
            SESSION_DISCOVERY_WINDOW_BYTES,
        );
        (Some(window), bytes)
    });
    let gameplay_state_byte = session_bytes
        .as_deref()
        .and_then(|bytes| bytes.get(0x296).copied());
    let controller_pointer = session_pointer.and_then(|pointer| {
        match process.read_u32(pointer as usize + SESSION_CONTROLLER_OFFSET) {
            Ok(controller) if plausible_heap_pointer(controller as usize) => Some(controller),
            Ok(controller) => {
                warnings.push(format!(
                    "controller pointer {controller:08X} is implausible"
                ));
                None
            }
            Err(error) => {
                warnings.push(format!("controller pointer read failed: {error}"));
                None
            }
        }
    });
    let (controller, targetter) = controller_pointer.map_or((None, None), |pointer| {
        let controller =
            read_duplicate_window(process, pointer as usize, CONTROLLER_DISCOVERY_WINDOW_BYTES);
        let targetter_base = pointer as usize + TARGETTER_OFFSET;
        let (block, bytes) =
            read_duplicate_window_with_bytes(process, targetter_base, TARGETTER_BYTES);
        let callback_pointer = bytes
            .as_deref()
            .and_then(|bytes| u32_from_slice(bytes, TARGETTER_CALLBACK_TABLE_POINTER_OFFSET));
        let callback_table = callback_pointer
            .filter(|pointer| plausible_heap_pointer(*pointer as usize))
            .map(|pointer| {
                read_duplicate_window(process, pointer as usize, TARGETTER_CALLBACK_TABLE_BYTES)
            });
        let targetter = TargetterEvidence {
            base_at_controller_0x22c: targetter_base as u32,
            prior_target_handle_at_0x04: bytes
                .as_deref()
                .and_then(|bytes| u32_from_slice(bytes, 0x04)),
            current_target_handle_at_0x0c: bytes
                .as_deref()
                .and_then(|bytes| u32_from_slice(bytes, 0x0C)),
            ray_distance_or_parameter_at_0x14: bytes
                .as_deref()
                .and_then(|bytes| u32_from_slice(bytes, 0x14)),
            result_bits_at_0x34: bytes
                .as_deref()
                .and_then(|bytes| u32_from_slice(bytes, 0x34)),
            scan_radius_at_0x38: bytes
                .as_deref()
                .and_then(|bytes| u32_from_slice(bytes, 0x38)),
            callback_table_pointer_at_0x44: callback_pointer,
            callback_table,
            block,
        };
        (Some(controller), Some(targetter))
    });
    let fullscreen_map_modal_gate = match process.read_u32(FULLSCREEN_MAP_MODAL_GATE) {
        Ok(value) => Some(value),
        Err(error) => {
            warnings.push(format!("fullscreen-map modal gate read failed: {error}"));
            None
        }
    };
    let tick_after = process.read_u32(RETAIL_FRAME_COUNTER).ok();
    let direct_input_keyboard = read_direct_input(process);
    RuntimeRadarWindows {
        tick_before,
        tick_after,
        tick_stable: tick_before
            .zip(tick_after)
            .map(|(before, after)| before == after),
        session_pointer_global_address: SESSION_POINTER_GLOBAL as u32,
        session_pointer,
        session,
        gameplay_state_byte_at_session_0x296: gameplay_state_byte,
        controller_pointer_offset_at_session_0x27c: SESSION_CONTROLLER_OFFSET as u32,
        controller_pointer,
        controller,
        targetter_setting_at_0x004cb40c: process
            .read_u32(SETTINGS_BASE + TARGETTER_SETTING_OFFSET)
            .ok(),
        hud_setting_at_0x004cb410: process.read_u32(SETTINGS_BASE + HUD_SETTING_OFFSET).ok(),
        targetter,
        direct_input_keyboard,
        fullscreen_map_icon_phase_at_0x004ef118: process.read_u32(FULLSCREEN_MAP_ICON_PHASE).ok(),
        fullscreen_map_icon_timestamp_low_at_0x004ef120: process
            .read_u32(FULLSCREEN_MAP_ICON_TIMESTAMP_LOW)
            .ok(),
        fullscreen_map_icon_timestamp_high_at_0x004ef124: process
            .read_u32(FULLSCREEN_MAP_ICON_TIMESTAMP_HIGH)
            .ok(),
        fullscreen_map_pass_counter_at_0x004f72ec: process
            .read_u32(FULLSCREEN_MAP_PASS_COUNTER)
            .ok(),
        fullscreen_map_modal_gate_at_0x004f741c: fullscreen_map_modal_gate,
        warnings,
    }
}

fn read_direct_input(process: &Process) -> DirectInputEvidence {
    let (state, bytes) = read_duplicate_window_with_bytes(
        process,
        DIRECT_INPUT_KEYBOARD_STATE,
        DIRECT_INPUT_KEYBOARD_STATE_BYTES,
    );
    DirectInputEvidence {
        state,
        dik_m_down: direct_input_key_down(bytes.as_deref(), DIK_M),
        dik_left_down: direct_input_key_down(bytes.as_deref(), DIK_LEFT),
        dik_right_down: direct_input_key_down(bytes.as_deref(), DIK_RIGHT),
        dik_up_down: direct_input_key_down(bytes.as_deref(), DIK_UP),
        dik_down_down: direct_input_key_down(bytes.as_deref(), DIK_DOWN),
    }
}

fn direct_input_key_down(bytes: Option<&[u8]>, index: usize) -> Option<bool> {
    bytes
        .and_then(|state| state.get(index))
        .map(|value| value & 0x80 != 0)
}

fn read_duplicate_window(process: &Process, address: usize, size: usize) -> DuplicateMemoryWindow {
    read_duplicate_window_with_bytes(process, address, size).0
}

fn read_duplicate_window_with_bytes(
    process: &Process,
    address: usize,
    size: usize,
) -> (DuplicateMemoryWindow, Option<Vec<u8>>) {
    match process.read_bytes(address, size) {
        Ok(before) => match process.read_bytes(address, size) {
            Ok(after) => {
                let window = DuplicateMemoryWindow {
                    address: address as u32,
                    requested_bytes: size,
                    duplicate_stable: Some(before == after),
                    fnv1a64: Some(format!("{:016X}", fnv1a64(&after))),
                    raw_hex: Some(encode_hex(&after)),
                    read_error: None,
                };
                (window, Some(after))
            }
            Err(error) => {
                let window = DuplicateMemoryWindow {
                    address: address as u32,
                    requested_bytes: size,
                    duplicate_stable: None,
                    fnv1a64: Some(format!("{:016X}", fnv1a64(&before))),
                    raw_hex: Some(encode_hex(&before)),
                    read_error: Some(format!("second duplicate read failed: {error}")),
                };
                (window, Some(before))
            }
        },
        Err(error) => (
            DuplicateMemoryWindow {
                address: address as u32,
                requested_bytes: size,
                duplicate_stable: None,
                fnv1a64: None,
                raw_hex: None,
                read_error: Some(error),
            },
            None,
        ),
    }
}

fn u32_from_slice(bytes: &[u8], offset: usize) -> Option<u32> {
    let value = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes(value.try_into().ok()?))
}

fn i32_from_slice(bytes: &[u8], offset: usize) -> Option<i32> {
    let value = bytes.get(offset..offset + 4)?;
    Some(i32::from_le_bytes(value.try_into().ok()?))
}

fn i16_from_slice(bytes: &[u8], offset: usize) -> Option<i16> {
    let value = bytes.get(offset..offset + 2)?;
    Some(i16::from_le_bytes(value.try_into().ok()?))
}

fn u16_from_slice(bytes: &[u8], offset: usize) -> Option<u16> {
    let value = bytes.get(offset..offset + 2)?;
    Some(u16::from_le_bytes(value.try_into().ok()?))
}

fn bounded_changes(before: &[u8], after: &[u8]) -> (usize, Vec<ByteChange>) {
    let mut count = 0;
    let mut sample = Vec::new();
    for (index, (&before, &after)) in before.iter().zip(after).enumerate() {
        if before != after {
            count += 1;
            if sample.len() < MAX_CHANGED_CELLS {
                sample.push(ByteChange {
                    index: index as u32,
                    before,
                    after,
                });
            }
        }
    }
    (count, sample)
}

fn integer_sqrt(value: u64) -> u64 {
    if value < 2 {
        return value;
    }
    let mut low = 1u64;
    let mut high = value.min(u64::from(u32::MAX)) + 1;
    while low + 1 < high {
        let middle = low + (high - low) / 2;
        if middle <= value / middle {
            low = middle;
        } else {
            high = middle;
        }
    }
    low
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xCBF2_9CE4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02X}");
    }
    output
}

fn cadence_due(sample: u64, hz: u32, requested_hz: u32) -> bool {
    sample == 0
        || sample.saturating_mul(u64::from(requested_hz)) / u64::from(hz)
            != sample
                .saturating_sub(1)
                .saturating_mul(u64::from(requested_hz))
                / u64::from(hz)
}

fn validate_timeline(seconds: f64, hz: u32, context_hz: u32, buffer_hz: u32) -> Result<(), String> {
    if !seconds.is_finite() || seconds <= 0.0 || seconds > 3600.0 {
        return Err("seconds must be finite and in the range 0 < seconds <= 3600".into());
    }
    if hz == 0 || hz > 1000 {
        return Err("hz must be in the range 1..=1000".into());
    }
    if context_hz == 0 || context_hz > hz {
        return Err("context-hz must be in the range 1..=hz".into());
    }
    if buffer_hz == 0 || buffer_hz > context_hz {
        return Err("buffer-hz must be in the range 1..=context-hz".into());
    }
    Ok(())
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
                    "could not create capture directory {}: {error}",
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
    use super::*;

    fn stable_window(address: u32, bytes: usize) -> DuplicateMemoryWindow {
        DuplicateMemoryWindow {
            address,
            requested_bytes: bytes,
            duplicate_stable: Some(true),
            fnv1a64: Some("0000000000000000".into()),
            raw_hex: Some("00".repeat(bytes)),
            read_error: None,
        }
    }

    fn complete_projection_evidence() -> RadarProjectionResourcesEvidence {
        let lengths = [15, 96, 6];
        RadarProjectionResourcesEvidence {
            resource_table_pointer_global_address: RADAR_PROJECTION_RESOURCE_TABLE_POINTER as u32,
            resource_table_pointer: Some(0x0200_0000),
            resource_table: Some(stable_window(0x0200_0000, 12)),
            resource_table_pointer_after_payloads: Some(0x0200_0000),
            resource_table_after_payloads: Some(stable_window(0x0200_0000, 12)),
            dimensions: RadarProjectionDimensionsEvidence {
                width_global_at_0x004ef128: 5,
                height_global_at_0x004df110: 3,
                sprite_meta_pointer_global_address: SPRITE_META_POINTER_GLOBAL as u32,
                sprite_meta_pointer: Some(0x0200_1000),
                sprite_meta_header: Some(stable_window(0x0200_1000, RADAR_RESOURCE_HEADER_BYTES)),
                width_pointer_at_sprite_meta_0x18: Some(0x0200_2000),
                height_pointer_at_sprite_meta_0x1c: Some(0x0200_2004),
                width_value: Some(stable_window(0x0200_2000, 4)),
                height_value: Some(stable_window(0x0200_2004, 4)),
                width_from_sprite_meta: Some(5),
                height_from_sprite_meta: Some(3),
                dimensions_match_globals: true,
                derived_payload_lengths: Some(lengths),
                validation_errors: Vec::new(),
            },
            chains: lengths
                .into_iter()
                .enumerate()
                .map(|(index, bytes)| RadarProjectionTableEvidence {
                    role: ["shade", "coordinates", "mask"][index],
                    resource_table_pointer_global_address: RADAR_PROJECTION_RESOURCE_TABLE_POINTER
                        as u32,
                    resource_table_pointer: Some(0x0200_0000),
                    table_entry_address: Some(0x0200_0000 + index as u32 * 4),
                    resource_pointer: Some(0x0200_3000 + index as u32 * 0x20),
                    resource_header: Some(stable_window(
                        0x0200_3000 + index as u32 * 0x20,
                        RADAR_RESOURCE_HEADER_BYTES,
                    )),
                    resource_header_after_payload: Some(stable_window(
                        0x0200_3000 + index as u32 * 0x20,
                        RADAR_RESOURCE_HEADER_BYTES,
                    )),
                    data_pointer_at_resource_0x18: Some(0x0200_4000 + index as u32 * 0x100),
                    derived_requested_bytes: Some(bytes),
                    data: Some(stable_window(0x0200_4000 + index as u32 * 0x100, bytes)),
                    read_error: None,
                })
                .collect(),
        }
    }

    fn complete_color_evidence() -> RadarColorTablesEvidence {
        let entry = |index: usize, role: &str| RadarColorEvidence {
            role: role.into(),
            relative_slot_offset: (index * 4) as u32,
            pointer_table_entry_address: Some(0x0201_0000 + index as u32 * 4),
            pointer_table_entry: Some(stable_window(0x0201_0000 + index as u32 * 4, 4)),
            color_pointer: Some(0x0202_0000 + index as u32 * 4),
            packed_color_u32: Some(index as u32),
            packed_color_u16: Some(index as u16),
            color_value: Some(stable_window(0x0202_0000 + index as u32 * 4, 4)),
            read_error: None,
        };
        RadarColorTablesEvidence {
            pointer_table_global_address: MASTER_COLOR_POINTER_TABLE as u32,
            pointer_table_pointer: Some(0x0201_0000),
            pointer_table_pointer_after_entries: Some(0x0201_0000),
            palette_base_id_at_0x004df114: 0,
            terrain_palette: (0..TERRAIN_PALETTE_ENTRY_COUNT)
                .map(|index| entry(index, "terrain"))
                .collect(),
            globe_shading_palette: (0..GLOBE_SHADING_PALETTE_ENTRY_COUNT)
                .map(|index| entry(index + 40, "globe"))
                .collect(),
            marker_palette: (0..MARKER_PALETTE_ENTRY_COUNT)
                .map(|index| entry(index + 60, "marker"))
                .collect(),
            warnings: Vec::new(),
        }
    }

    fn complete_icon_evidence() -> FullscreenMapIconTableEvidence {
        let mut types = (0..ENTITY_TYPE_COUNT)
            .map(|entity_type| FullscreenMapIconTypeEvidence {
                entity_type: entity_type as u32,
                slot_state: "unloaded_type_slot",
                type_record_pointer: Some(0),
                selector_value: None,
                selector_at_type_record_0x70: None,
                sprite_pointer_table_entry_address: None,
                sprite_pointer_table_entry: None,
                sprite_resource_pointer: None,
                sprite_width_at_resource_0x10: None,
                sprite_height_at_resource_0x12: None,
                sprite_resource_header: None,
                read_error: None,
            })
            .collect::<Vec<_>>();
        types[3] = FullscreenMapIconTypeEvidence {
            entity_type: 3,
            slot_state: "loaded_icon",
            type_record_pointer: Some(0x0203_0000),
            selector_value: Some(stable_window(0x0203_0070, 2)),
            selector_at_type_record_0x70: Some(12),
            sprite_pointer_table_entry_address: Some(0x0204_0030),
            sprite_pointer_table_entry: Some(stable_window(0x0204_0030, 4)),
            sprite_resource_pointer: Some(0x0205_0000),
            sprite_width_at_resource_0x10: Some(16),
            sprite_height_at_resource_0x12: Some(12),
            sprite_resource_header: Some(stable_window(
                0x0205_0000,
                FULLSCREEN_MAP_ICON_HEADER_BYTES,
            )),
            read_error: None,
        };
        FullscreenMapIconTableEvidence {
            entity_type_pointer_table_global_address: ENTITY_TYPE_POINTER_TABLE as u32,
            entity_type_pointer_table: Some(0x0206_0000),
            entity_type_pointer_table_window: Some(stable_window(
                0x0206_0000,
                ENTITY_TYPE_COUNT * 4,
            )),
            entity_type_pointer_table_after_types: Some(stable_window(
                0x0206_0000,
                ENTITY_TYPE_COUNT * 4,
            )),
            entity_type_pointer_table_after_types_pointer: Some(0x0206_0000),
            sprite_pointer_table_global_address: GLOBAL_SPRITE_POINTER_TABLE as u32,
            sprite_pointer_table: Some(0x0204_0000),
            sprite_pointer_table_after_types: Some(0x0204_0000),
            unloaded_type_slots: ENTITY_TYPE_COUNT - 1,
            loaded_type_records: 1,
            nonzero_icon_selectors: 1,
            types,
            warnings: Vec::new(),
        }
    }

    fn complete_layout_evidence() -> FullscreenMapLayoutEvidence {
        FullscreenMapLayoutEvidence {
            sprite_meta_pointer_global_address: SPRITE_META_POINTER_GLOBAL as u32,
            sprite_meta_pointer: Some(0x0207_0000),
            sprite_meta_pointer_after_values: Some(0x0207_0000),
            sprite_meta_layout: Some(stable_window(0x0207_0000, SPRITE_META_LAYOUT_BYTES)),
            origin_x_pointer_at_0x30: Some(0x0207_1000),
            origin_y_pointer_at_0x34: Some(0x0207_1004),
            width_pointer_at_0x38: Some(0x0207_1008),
            height_pointer_at_0x3c: Some(0x0207_100C),
            origin_x_value: Some(stable_window(0x0207_1000, 4)),
            origin_y_value: Some(stable_window(0x0207_1004, 4)),
            width_value: Some(stable_window(0x0207_1008, 4)),
            height_value: Some(stable_window(0x0207_100C, 4)),
            origin_x_from_pointer_at_0x30: Some(2),
            origin_y_from_pointer_at_0x34: Some(7),
            width_from_pointer_at_0x38: Some(464),
            height_from_pointer_at_0x3c: Some(464),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn packed_coverage_nibbles_follow_retail_even_low_odd_high_order() {
        let mut buffers = RadarBuffers {
            tick_before: 1,
            tick_after: 1,
            width: 64,
            height: 64,
            palette_base_id: 0,
            render_parameter: 0,
            rebuild_cursor: 0,
            rebuild_axis: 0,
            terrain_duplicate_stable: true,
            coverage_duplicate_stable: true,
            terrain: vec![0; RADAR_TERRAIN_BUFFER_BYTES],
            coverage: vec![0; RADAR_COVERAGE_BUFFER_BYTES],
        };
        let even_position = [0x1200, 0, 0x3400];
        let even_index = 0x34 * 256 + 0x12;
        buffers.terrain[even_index] = 0x29;
        buffers.coverage[even_index / 2] = 0xA5;
        let even = cell_evidence(even_position, &buffers);
        assert_eq!(even.linear_index, even_index as u32);
        assert_eq!(even.terrain_raster_value, 0x29);
        assert_eq!(even.terrain_coverage_nibble, 5);

        let odd_position = [0x1300, 0, 0x3400];
        let odd = cell_evidence(odd_position, &buffers);
        assert_eq!(odd.terrain_coverage_nibble, 0xA);
    }

    #[test]
    fn changed_cell_summary_is_bounded_without_losing_total() {
        let before = vec![0; MAX_CHANGED_CELLS + 20];
        let after = vec![1; MAX_CHANGED_CELLS + 20];
        let (count, sample) = bounded_changes(&before, &after);
        assert_eq!(count, MAX_CHANGED_CELLS + 20);
        assert_eq!(sample.len(), MAX_CHANGED_CELLS);
        assert_eq!(sample[0].index, 0);
        assert_eq!(sample[MAX_CHANGED_CELLS - 1].index, 127);
    }

    #[test]
    fn integer_sqrt_is_floor_exact_around_marker_thresholds() {
        assert_eq!(integer_sqrt(0), 0);
        assert_eq!(integer_sqrt(1), 1);
        assert_eq!(integer_sqrt(15), 3);
        assert_eq!(integer_sqrt(16), 4);
        assert_eq!(integer_sqrt(17), 4);
        assert_eq!(integer_sqrt(0x2000u64 * 0x2000), 0x2000);
    }

    #[test]
    fn rate_validation_keeps_expensive_buffer_reads_below_context_rate() {
        assert!(validate_timeline(120.0, 100, 20, 10).is_ok());
        assert!(validate_timeline(120.0, 100, 101, 10).is_err());
        assert!(validate_timeline(120.0, 100, 20, 21).is_err());
    }

    #[test]
    fn arbitrary_sampling_rates_use_cadence_crossings_without_stride_rounding() {
        let due = (0..10)
            .filter(|sample| cadence_due(*sample, 100, 30))
            .collect::<Vec<_>>();
        assert_eq!(due, vec![0, 4, 7]);
    }

    #[test]
    fn projection_table_sizes_follow_full_and_half_resolution_retail_loops() {
        assert_eq!(projection_table_lengths(5, 3), Some([15, 96, 6]));
        assert_eq!(projection_table_lengths(0, 3), None);
        assert_eq!(projection_table_lengths(-1, 3), None);
    }

    #[test]
    fn projection_resources_are_entries_beneath_the_global_outer_pointer() {
        let outer = 0x02AD_1650;
        assert_eq!(projection_resource_entry_address(outer, 0), 0x02AD_1650);
        assert_eq!(projection_resource_entry_address(outer, 1), 0x02AD_1654);
        assert_eq!(projection_resource_entry_address(outer, 2), 0x02AD_1658);
        assert_ne!(
            projection_resource_entry_address(outer, 1),
            RADAR_PROJECTION_RESOURCE_TABLE_POINTER as u32 + 4
        );

        // Exact outer-array bytes retained by the first radar capture.
        let captured = [
            0x08, 0xA0, 0x81, 0x13, 0x24, 0xA0, 0x81, 0x13, 0x40, 0xA0, 0x81, 0x13,
        ];
        assert_eq!(pointer_from_table_bytes(&captured, 0), Some(0x1381_A008));
        assert_eq!(pointer_from_table_bytes(&captured, 1), Some(0x1381_A024));
        assert_eq!(pointer_from_table_bytes(&captured, 2), Some(0x1381_A040));
        assert_eq!(pointer_from_table_bytes(&captured, 3), None);
    }

    #[test]
    fn projection_snapshot_requires_three_exact_duplicate_stable_chains() {
        let evidence = complete_projection_evidence();
        assert!(projection_validation_errors(&evidence).is_empty());

        let mut truncated = evidence.clone();
        truncated.chains[1]
            .data
            .as_mut()
            .expect("test payload")
            .raw_hex = Some("00".repeat(95));
        let errors = projection_validation_errors(&truncated);
        assert!(errors
            .iter()
            .any(|error| error.contains("projection chain 1") && error.contains("payload")));

        let mut unstable_outer = evidence;
        unstable_outer
            .resource_table
            .as_mut()
            .expect("test outer table")
            .duplicate_stable = Some(false);
        assert!(projection_validation_errors(&unstable_outer)
            .iter()
            .any(|error| error.contains("outer table")));
    }

    #[test]
    fn projection_snapshot_rejects_a_header_change_after_payload_reads() {
        let mut evidence = complete_projection_evidence();
        evidence.chains[2]
            .resource_header_after_payload
            .as_mut()
            .expect("post-payload header")
            .raw_hex = Some("01".repeat(RADAR_RESOURCE_HEADER_BYTES));
        assert!(projection_validation_errors(&evidence).iter().any(|error| {
            error.contains("projection chain 2") && error.contains("changed across payload")
        }));
    }

    #[test]
    fn visible_resource_contract_gates_colors_icons_and_layout() {
        let colors = complete_color_evidence();
        let icons = complete_icon_evidence();
        let layout = complete_layout_evidence();
        assert!(color_validation_errors(&colors).is_empty());
        assert!(fullscreen_map_icon_validation_errors(&icons).is_empty());
        assert!(fullscreen_map_layout_validation_errors(&layout).is_empty());

        let mut bad_colors = colors;
        bad_colors.marker_palette[0]
            .color_value
            .as_mut()
            .expect("marker color")
            .duplicate_stable = Some(false);
        assert!(color_validation_errors(&bad_colors)
            .iter()
            .any(|error| error.contains("packed color value")));

        let mut bad_icons = icons;
        bad_icons.types[3].sprite_width_at_resource_0x10 = Some(0);
        assert!(fullscreen_map_icon_validation_errors(&bad_icons)
            .iter()
            .any(|error| error.contains("dimensions")));

        let mut bad_layout = layout;
        bad_layout.width_from_pointer_at_0x38 = Some(0);
        assert!(fullscreen_map_layout_validation_errors(&bad_layout)
            .iter()
            .any(|error| error.contains("non-positive")));
    }

    #[test]
    fn projection_data_pointer_decodes_only_from_the_captured_header_offset() {
        let mut header = [0u8; RADAR_RESOURCE_HEADER_BYTES];
        header[RADAR_RESOURCE_DATA_POINTER_OFFSET
            ..RADAR_RESOURCE_DATA_POINTER_OFFSET + std::mem::size_of::<u32>()]
            .copy_from_slice(&0x1234_5678u32.to_le_bytes());
        assert_eq!(
            u32_from_slice(&header, RADAR_RESOURCE_DATA_POINTER_OFFSET),
            Some(0x1234_5678)
        );
    }

    #[test]
    fn fullscreen_icon_dimensions_decode_from_the_retail_sprite_header_offsets() {
        let mut header = [0u8; FULLSCREEN_MAP_ICON_HEADER_BYTES];
        header[0x10..0x12].copy_from_slice(&37u16.to_le_bytes());
        header[0x12..0x14].copy_from_slice(&19u16.to_le_bytes());
        assert_eq!(u16_from_slice(&header, 0x10), Some(37));
        assert_eq!(u16_from_slice(&header, 0x12), Some(19));
    }

    #[test]
    fn direct_input_decode_uses_the_retail_high_bit_and_exact_dik_index() {
        let mut state = [0u8; DIRECT_INPUT_KEYBOARD_STATE_BYTES];
        state[DIK_M] = 0x80;
        state[DIK_LEFT] = 0x01;
        assert_eq!(direct_input_key_down(Some(&state), DIK_M), Some(true));
        assert_eq!(direct_input_key_down(Some(&state), DIK_LEFT), Some(false));
        assert_eq!(direct_input_key_down(None, DIK_M), None);
    }
}
