//! Correlated actor, lifecycle, effects, input, and terrain evidence.
//!
//! This is the reusable read-only substrate for AI, factory, hive/virus, and
//! entity-death capture protocols. Scenario names remain metadata; uncertain
//! notions such as friendly/enemy are never inferred from a coincidental flag.

use std::collections::{HashMap, HashSet};
use std::fs::{create_dir_all, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use serde::Serialize;
use serde_json::json;

use crate::entity::EntityRecord;
use crate::event_stream::{AudioTimelineState, ParticleTimelineState};
use crate::process::{BuildFingerprint, Process};
use crate::terrain_infection::TerrainInfectionState;
use crate::timeline::{ensure_distinct_capture_paths, ensure_stop_file_absent, run_stop_aware};

const SHARED_RNG_STATE_ADDRESS: usize = 0x004F_7308;
const MAX_EXPLICIT_FILTERS: usize = 128;
const MAX_NEAREST_ACTORS: usize = 64;

pub struct WorldTimelineRequest<'a> {
    pub scenario: &'a str,
    pub entity_types: &'a [u32],
    pub model_ids: &'a [u32],
    pub handles: &'a [u32],
    pub nearest_player: usize,
    pub seconds: f64,
    pub hz: u32,
    pub context_hz: u32,
    pub evidence_hz: u32,
    pub terrain_hz: u32,
    pub capture_effects: bool,
    pub capture_terrain_infection: bool,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct SoundCatalogIdentity {
    sounds_pointer: u32,
    sound_count: u32,
    loaded_levels: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TransitionResourceContext {
    pools: crate::resources::ResourcePools,
    catalog: crate::resources::ResourceCatalog,
}

#[derive(Debug, Serialize)]
struct CompactActorState {
    intrusive_list_index: usize,
    pointer: u32,
    entity_type: u32,
    handle: u32,
    flags: u32,
    parent_handle: Option<u32>,
    raw_dword_at_0x80: u32,
    recent_relation_handle_at_0x60: u32,
    recent_relation_elapsed_us_at_0x68: u32,
    remaining_lifetime_us_at_0x74: u32,
    anchor_raw_8_8_at_0x90: [i16; 3],
    position_raw_8_8: [i16; 3],
    velocity_raw_8_8: [i16; 3],
    body_basis_raw: [i32; 9],
    rotation: [u16; 3],
    models: [u16; 4],
    active_slot: usize,
    active_model: u16,
    active_model_resource: u32,
    collision_health_at_0x30: i32,
    last_damage_tick_at_0x34: u32,
    damage_buffer_at_0x50: i32,
    component_root: u32,
    behavior: u32,
    behavior_context_at_0xc0: u32,
    environment_flags_at_0xc8: u32,
}

#[derive(Debug, Clone, Copy)]
struct ActorSelectionCandidate {
    intrusive_list_index: usize,
    pointer: u32,
    entity_type: u32,
    active_model: u16,
    handle: u32,
    position_raw_8_8: [i16; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExistingPointerEvent {
    None,
    StateChange,
    PointerReuse,
}

impl From<&EntityRecord> for CompactActorState {
    fn from(record: &EntityRecord) -> Self {
        Self {
            intrusive_list_index: record.intrusive_list_index,
            pointer: record.pointer,
            entity_type: record.entity_type,
            handle: record.handle,
            flags: record.flags,
            parent_handle: record.parent_handle,
            raw_dword_at_0x80: record.raw_dword_at_0x80,
            recent_relation_handle_at_0x60: record.recent_relation_handle_at_0x60,
            recent_relation_elapsed_us_at_0x68: record.recent_relation_elapsed_us_at_0x68,
            remaining_lifetime_us_at_0x74: record.remaining_lifetime_us_at_0x74,
            anchor_raw_8_8_at_0x90: record.anchor_raw_8_8_at_0x90,
            position_raw_8_8: record.position_raw_8_8,
            velocity_raw_8_8: record.velocity_raw_8_8,
            body_basis_raw: record.body_basis_raw,
            rotation: record.rotation,
            models: record.models,
            active_slot: record.active_slot,
            active_model: record.active_model,
            active_model_resource: record.active_model_resource,
            collision_health_at_0x30: record.collision_health_at_0x30,
            last_damage_tick_at_0x34: record.last_damage_tick_at_0x34,
            damage_buffer_at_0x50: record.damage_buffer_at_0x50,
            component_root: record.component_root,
            behavior: record.behavior,
            behavior_context_at_0xc0: record.behavior_context_at_0xc0,
            environment_flags_at_0xc8: record.environment_flags_at_0xc8,
        }
    }
}

pub fn capture(process: &Process, request: WorldTimelineRequest<'_>) -> Result<(), String> {
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
                command: "world-behavior-timeline",
                process_id: process.process_id,
                executable: &process.exe_name,
                build: &process.build,
            },
            "scenario": request.scenario,
            "entity_types": request.entity_types,
            "model_ids": request.model_ids,
            "handles": request.handles,
            "nearest_player": request.nearest_player,
            "seconds": request.seconds,
            "hz": request.hz,
            "context_hz": request.context_hz,
            "evidence_hz": request.evidence_hz,
            "terrain_hz": request.capture_terrain_infection.then_some(request.terrain_hz),
            "capture_effects": request.capture_effects,
            "capture_terrain_infection": request.capture_terrain_infection,
            "stop_file": request.stop_file.map(|path| path.display().to_string()),
            "transition_context_policy": "at up to 10 Hz, retain only duplicate-equal resource-pool/loaded-overlay reads and tick/stack/context-bracketed frontend/result state; unstable observations are explicit and never advance the canonical baseline; raw completion fields have no assigned semantics",
            "selection_policy": "explicit handles, types, and active-model IDs are always retained; when nearest-player is nonzero the player plus the closest distinct non-player actors by toroidal X/Z distance are retained; no friendly/enemy semantics are inferred",
            "lifecycle_policy": "birth, death/unlink, pointer reuse, and non-motion state changes are retained globally for the complete stable intrusive list; type/handle/nearest filters bound compact actor samples and deep runtime evidence only",
            "rng_policy": "0x004F7308 is a sampled shared RNG endpoint only; samples cannot prove the number or order of draws between observations",
            "callback_policy": "callback addresses and bounded state are read but never invoked; retail memory is never written",
            "input_policy": "only named scenario controls are sampled, and only while the attached V2000 process owns the foreground window",
            "terrain_policy": request.capture_terrain_infection.then_some(
                "duplicate full Section-10 reads retain exact bounded three-byte cell diffs, 0x10/0x08 counters, derived retail percentages, and resource-boundary resets"
            ),
        }),
    )?;
    writer.flush().map_err(|error| error.to_string())?;

    let mut sound_catalog_identity = None;
    let mut previous_sound_catalog_error = None;
    let mut particle_state = ParticleTimelineState::new(false);
    let mut audio_state = AudioTimelineState::default();
    let mut terrain_state = TerrainInfectionState::new();
    let mut previous_entities: HashMap<u32, EntityRecord> = HashMap::new();
    let mut previous_input = None;
    let mut previous_foreground = None;
    let mut previous_resource_pools = None;
    let mut previous_resource_catalog = None;
    let mut previous_menu_state = None;
    let mut previous_resource_error = None;
    let mut previous_menu_error = None;
    let mut previous_entity_error: Option<String> = None;
    let mut previous_terrain_error: Option<String> = None;

    let outcome = run_stop_aware(
        request.seconds,
        request.hz,
        request.stop_file,
        |sample, elapsed_ms| {
            let input = crate::foreground_input::poll_scenario_foreground(process.process_id);
            let foreground = input.is_some();
            if previous_foreground != Some(foreground) {
                write_json_line(
                    &mut writer,
                    &json!({
                        "record_kind": "scenario_input_scope",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "v2000_foreground": foreground,
                    }),
                )?;
                previous_foreground = Some(foreground);
            }
            if let (Some(before), Some(current)) = (previous_input, input) {
                for edge in current.edges_from(before) {
                    write_json_line(
                        &mut writer,
                        &json!({
                            "record_kind": "scenario_input_event",
                            "sample": sample,
                            "elapsed_ms": elapsed_ms,
                            "edge": edge,
                            "input_state": current,
                        }),
                    )?;
                }
            }
            previous_input = input;

            let transition_due = cadence_due(sample, request.hz, request.evidence_hz.min(10));
            let stable_resources = if transition_due {
                sample_transition_context(
                    process,
                    &mut writer,
                    sample,
                    elapsed_ms,
                    &mut previous_resource_pools,
                    &mut previous_resource_catalog,
                    &mut previous_menu_state,
                    &mut previous_resource_error,
                    &mut previous_menu_error,
                )?
            } else {
                None
            };

            if request.capture_effects && transition_due {
                refresh_sound_resource_catalog(
                    process,
                    &mut writer,
                    sample,
                    elapsed_ms,
                    stable_resources.as_ref(),
                    &mut sound_catalog_identity,
                    &mut previous_sound_catalog_error,
                )?;
            }

            if request.capture_effects {
                particle_state.sample(process, &mut writer, sample, elapsed_ms)?;
                audio_state.sample(process, &mut writer, sample, elapsed_ms)?;
            }

            if request.capture_terrain_infection
                && cadence_due(sample, request.hz, request.terrain_hz)
            {
                match terrain_state.sample(process, &mut writer, sample, elapsed_ms) {
                    Ok(()) => previous_terrain_error = None,
                    Err(error) => {
                        if previous_terrain_error.as_deref() != Some(error.as_str()) {
                            write_json_line(
                                &mut writer,
                                &json!({
                                    "record_kind": "terrain_infection_error",
                                    "sample": sample,
                                    "elapsed_ms": elapsed_ms,
                                    "error": &error,
                                }),
                            )?;
                        }
                        previous_terrain_error = Some(error);
                    }
                }
            }

            if cadence_due(sample, request.hz, request.context_hz) {
                match crate::entity::read_snapshot(process) {
                    Ok(snapshot) => {
                        previous_entity_error = None;
                        if !(snapshot.tick_stable && snapshot.topology_stable) {
                            write_json_line(
                                &mut writer,
                                &json!({
                                    "record_kind": "entity_snapshot_unstable",
                                    "sample": sample,
                                    "elapsed_ms": elapsed_ms,
                                    "tick_before": snapshot.tick_before,
                                    "tick_after": snapshot.tick_after,
                                    "head_before": snapshot.head_before,
                                    "head_after": snapshot.head_after,
                                    "tail_before": snapshot.tail_before,
                                    "tail_after": snapshot.tail_after,
                                    "warnings": snapshot.warnings,
                                }),
                            )?;
                        } else {
                            emit_lifecycle_events(
                                &mut writer,
                                sample,
                                elapsed_ms,
                                &previous_entities,
                                &snapshot.records,
                            )?;
                            let selected = select_actors(
                                &snapshot.records,
                                request.entity_types,
                                request.model_ids,
                                request.handles,
                                request.nearest_player,
                            );
                            let actors = selected
                                .iter()
                                .map(|record| CompactActorState::from(*record))
                                .collect::<Vec<_>>();
                            write_json_line(
                                &mut writer,
                                &json!({
                                    "record_kind": "actor_sample",
                                    "sample": sample,
                                    "elapsed_ms": elapsed_ms,
                                    "tick": snapshot.tick_after,
                                    "sampled_shared_rng_state_address": SHARED_RNG_STATE_ADDRESS as u32,
                                    "sampled_shared_rng_state": process.read_u32(SHARED_RNG_STATE_ADDRESS).ok(),
                                    "entity_count": snapshot.records.len(),
                                    "selected_count": actors.len(),
                                    "actors": actors,
                                    "warnings": snapshot.warnings,
                                }),
                            )?;

                            if cadence_due(sample, request.hz, request.evidence_hz) {
                                for record in selected {
                                    write_json_line(
                                        &mut writer,
                                        &json!({
                                            "record_kind": "actor_runtime_evidence",
                                            "sample": sample,
                                            "elapsed_ms": elapsed_ms,
                                            "tick": snapshot.tick_after,
                                            "entity_pointer": record.pointer,
                                            "entity_handle": record.handle,
                                            "entity_type": record.entity_type,
                                            "evidence": crate::entity::read_runtime_evidence(process, record),
                                        }),
                                    )?;
                                }
                            }
                            previous_entities = snapshot
                                .records
                                .into_iter()
                                .map(|record| (record.pointer, record))
                                .collect();
                        }
                    }
                    Err(error) => {
                        if previous_entity_error.as_deref() != Some(error.as_str()) {
                            write_json_line(
                                &mut writer,
                                &json!({
                                    "record_kind": "entity_snapshot_error",
                                    "sample": sample,
                                    "elapsed_ms": elapsed_ms,
                                    "error": &error,
                                }),
                            )?;
                        }
                        previous_entity_error = Some(error);
                    }
                }
            }

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
    eprintln!(
        "wrote world behavior timeline to {}",
        request.output.display()
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn sample_transition_context(
    process: &Process,
    writer: &mut impl Write,
    sample: u64,
    elapsed_ms: f64,
    previous_pools: &mut Option<crate::resources::ResourcePools>,
    previous_catalog: &mut Option<crate::resources::ResourceCatalog>,
    previous_menu: &mut Option<crate::menu::MenuEventState>,
    previous_resource_error: &mut Option<String>,
    previous_menu_error: &mut Option<String>,
) -> Result<Option<TransitionResourceContext>, String> {
    let stable_resources = match read_transition_resources(process) {
        Ok(first) => match read_transition_resources(process) {
            Ok(second) if first == second => {
                if previous_pools.as_ref() != Some(&second.pools) {
                    write_json_line(
                        writer,
                        &json!({
                            "record_kind": "resource_pools",
                            "sample": sample,
                            "elapsed_ms": elapsed_ms,
                            "previous": previous_pools.as_ref(),
                            "current": &second.pools,
                        }),
                    )?;
                    *previous_pools = Some(second.pools.clone());
                }
                if previous_catalog.as_ref() != Some(&second.catalog) {
                    write_json_line(
                        writer,
                        &json!({
                            "record_kind": "resource_catalog",
                            "sample": sample,
                            "elapsed_ms": elapsed_ms,
                            "loaded_levels": second.catalog.loaded_level_ids(),
                            "previous": previous_catalog.as_ref(),
                            "current": &second.catalog,
                        }),
                    )?;
                    *previous_catalog = Some(second.catalog.clone());
                }
                *previous_resource_error = None;
                Some(second)
            }
            Ok(second) => {
                write_json_line(
                    writer,
                    &json!({
                        "record_kind": "transition_context_unstable",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "domain": "resources",
                        "first": {
                            "pools": &first.pools,
                            "catalog": &first.catalog,
                        },
                        "second": {
                            "pools": &second.pools,
                            "catalog": &second.catalog,
                        },
                    }),
                )?;
                *previous_resource_error = None;
                None
            }
            Err(error) => {
                emit_transition_context_error(
                    writer,
                    sample,
                    elapsed_ms,
                    "resources_second_read",
                    error,
                    previous_resource_error,
                )?;
                None
            }
        },
        Err(error) => {
            emit_transition_context_error(
                writer,
                sample,
                elapsed_ms,
                "resources_first_read",
                error,
                previous_resource_error,
            )?;
            None
        }
    };

    match crate::menu::read_snapshot(process) {
        Ok(snapshot) => {
            let state = snapshot.event_state();
            let stable = snapshot.tick_stable && snapshot.stack_stable && snapshot.context_stable;
            if previous_menu.as_ref() != Some(&state) || !stable {
                write_json_line(
                    writer,
                    &json!({
                        "record_kind": if stable { "menu_event" } else { "menu_event_unstable" },
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "snapshot": &snapshot,
                    }),
                )?;
            }
            if stable {
                *previous_menu = Some(state);
            }
            *previous_menu_error = None;
        }
        Err(error) => emit_transition_context_error(
            writer,
            sample,
            elapsed_ms,
            "menu",
            error,
            previous_menu_error,
        )?,
    }

    Ok(stable_resources)
}

fn read_transition_resources(process: &Process) -> Result<TransitionResourceContext, String> {
    let pools = crate::resources::read(process)?;
    let catalog = crate::resources::read_catalog(process)?;
    Ok(TransitionResourceContext { pools, catalog })
}

fn emit_transition_context_error(
    writer: &mut impl Write,
    sample: u64,
    elapsed_ms: f64,
    domain: &'static str,
    error: String,
    previous: &mut Option<String>,
) -> Result<(), String> {
    if previous.as_deref() == Some(error.as_str()) {
        return Ok(());
    }
    write_json_line(
        writer,
        &json!({
            "record_kind": "transition_context_error",
            "sample": sample,
            "elapsed_ms": elapsed_ms,
            "domain": domain,
            "error": &error,
        }),
    )?;
    *previous = Some(error);
    Ok(())
}

fn emit_lifecycle_events(
    writer: &mut impl Write,
    sample: u64,
    elapsed_ms: f64,
    previous: &HashMap<u32, EntityRecord>,
    current: &[EntityRecord],
) -> Result<(), String> {
    let current_by_pointer = current
        .iter()
        .map(|record| (record.pointer, record))
        .collect::<HashMap<_, _>>();
    if previous.is_empty() {
        write_json_line(
            writer,
            &json!({
                "record_kind": "entity_lifecycle_baseline",
                "sample": sample,
                "elapsed_ms": elapsed_ms,
                "entities": current,
            }),
        )?;
        return Ok(());
    }

    for record in current {
        match previous.get(&record.pointer) {
            None => write_json_line(
                writer,
                &json!({
                    "record_kind": "entity_lifecycle_event",
                    "event": "birth",
                    "sample": sample,
                    "elapsed_ms": elapsed_ms,
                    "entity": record,
                }),
            )?,
            Some(before) => match classify_existing_pointer(before, record) {
                ExistingPointerEvent::PointerReuse => {
                    write_json_line(
                        writer,
                        &json!({
                            "record_kind": "entity_lifecycle_event",
                            "event": "death_or_unlink",
                            "reason": "pointer_reused",
                            "sample": sample,
                            "elapsed_ms": elapsed_ms,
                            "entity": before,
                        }),
                    )?;
                    write_json_line(
                        writer,
                        &json!({
                            "record_kind": "entity_lifecycle_event",
                            "event": "birth",
                            "reason": "pointer_reused",
                            "sample": sample,
                            "elapsed_ms": elapsed_ms,
                            "entity": record,
                        }),
                    )?;
                }
                ExistingPointerEvent::StateChange => write_json_line(
                    writer,
                    &json!({
                        "record_kind": "entity_lifecycle_event",
                        "event": "state_change",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "changed_offsets": crate::entity::changed_offsets(&before.raw_hex, &record.raw_hex),
                        "previous_signature": before.signature(),
                        "entity": record,
                    }),
                )?,
                ExistingPointerEvent::None => {}
            },
        }
    }
    for (pointer, record) in previous {
        if !current_by_pointer.contains_key(pointer) {
            write_json_line(
                writer,
                &json!({
                    "record_kind": "entity_lifecycle_event",
                    "event": "death_or_unlink",
                    "sample": sample,
                    "elapsed_ms": elapsed_ms,
                    "entity": record,
                }),
            )?;
        }
    }
    Ok(())
}

fn classify_existing_pointer(before: &EntityRecord, after: &EntityRecord) -> ExistingPointerEvent {
    classify_existing_pointer_values(
        before.handle,
        before.entity_type,
        after.handle,
        after.entity_type,
        lifecycle_changed(before, after),
    )
}

fn classify_existing_pointer_values(
    before_handle: u32,
    before_type: u32,
    after_handle: u32,
    after_type: u32,
    state_changed: bool,
) -> ExistingPointerEvent {
    if before_handle != after_handle || before_type != after_type {
        ExistingPointerEvent::PointerReuse
    } else if state_changed {
        ExistingPointerEvent::StateChange
    } else {
        ExistingPointerEvent::None
    }
}

fn lifecycle_changed(before: &EntityRecord, after: &EntityRecord) -> bool {
    before.signature() != after.signature()
        || before.collision_health_at_0x30 != after.collision_health_at_0x30
        || before.last_damage_tick_at_0x34 != after.last_damage_tick_at_0x34
        || before.damage_buffer_at_0x50 != after.damage_buffer_at_0x50
        || before.recent_relation_handle_at_0x60 != after.recent_relation_handle_at_0x60
        || before.raw_dword_at_0x80 != after.raw_dword_at_0x80
        || before.component_root != after.component_root
        || before.behavior_context_at_0xc0 != after.behavior_context_at_0xc0
}

fn select_actors<'a>(
    records: &'a [EntityRecord],
    types: &[u32],
    model_ids: &[u32],
    handles: &[u32],
    nearest_player: usize,
) -> Vec<&'a EntityRecord> {
    let candidates = records
        .iter()
        .map(|record| ActorSelectionCandidate {
            intrusive_list_index: record.intrusive_list_index,
            pointer: record.pointer,
            entity_type: record.entity_type,
            active_model: record.active_model,
            handle: record.handle,
            position_raw_8_8: record.position_raw_8_8,
        })
        .collect::<Vec<_>>();
    let selected_pointers =
        select_actor_pointers(&candidates, types, model_ids, handles, nearest_player);
    let by_pointer = records
        .iter()
        .map(|record| (record.pointer, record))
        .collect::<HashMap<_, _>>();
    selected_pointers
        .into_iter()
        .filter_map(|pointer| by_pointer.get(&pointer).copied())
        .collect()
}

fn select_actor_pointers(
    candidates: &[ActorSelectionCandidate],
    types: &[u32],
    model_ids: &[u32],
    handles: &[u32],
    nearest_player: usize,
) -> Vec<u32> {
    let mut selected = Vec::new();
    let mut pointers = HashSet::new();
    for candidate in candidates {
        if (types.contains(&candidate.entity_type)
            || model_ids.contains(&u32::from(candidate.active_model))
            || handles.contains(&candidate.handle))
            && pointers.insert(candidate.pointer)
        {
            selected.push(*candidate);
        }
    }

    if nearest_player != 0 {
        if let Some(player) = candidates
            .iter()
            .find(|candidate| candidate.entity_type == 46)
        {
            if pointers.insert(player.pointer) {
                selected.push(*player);
            }
            let mut nearest = candidates
                .iter()
                .filter(|candidate| candidate.pointer != player.pointer)
                .map(|candidate| {
                    let dx = i64::from(
                        candidate.position_raw_8_8[0].wrapping_sub(player.position_raw_8_8[0]),
                    );
                    let dz = i64::from(
                        candidate.position_raw_8_8[2].wrapping_sub(player.position_raw_8_8[2]),
                    );
                    (dx * dx + dz * dz, candidate.pointer, candidate)
                })
                .collect::<Vec<_>>();
            nearest.sort_unstable_by_key(|(distance, pointer, _)| (*distance, *pointer));
            for (_, _, candidate) in nearest.into_iter().take(nearest_player) {
                if pointers.insert(candidate.pointer) {
                    selected.push(*candidate);
                }
            }
        }
    }
    selected.sort_unstable_by_key(|candidate| candidate.intrusive_list_index);
    selected
        .into_iter()
        .map(|candidate| candidate.pointer)
        .collect()
}

fn refresh_sound_resource_catalog(
    process: &Process,
    writer: &mut impl Write,
    sample: u64,
    elapsed_ms: f64,
    resources: Option<&TransitionResourceContext>,
    previous_identity: &mut Option<SoundCatalogIdentity>,
    previous_error: &mut Option<String>,
) -> Result<(), String> {
    let Some(resources) = resources else {
        return Ok(());
    };
    let identity = SoundCatalogIdentity {
        sounds_pointer: resources.pools.sounds,
        sound_count: resources.catalog.section_totals[11],
        loaded_levels: resources.catalog.loaded_level_ids(),
    };
    if previous_identity.as_ref() == Some(&identity) {
        *previous_error = None;
        return Ok(());
    }

    match crate::sound_resource::read(
        process,
        resources.pools.sounds,
        resources.catalog.section_totals[11],
    ) {
        Ok(resources) => {
            write_json_line(
                writer,
                &json!({
                    "record_kind": "sound_resource_catalog",
                    "sample": sample,
                    "elapsed_ms": elapsed_ms,
                    "identity": {
                        "sounds_pointer": identity.sounds_pointer,
                        "sound_count": identity.sound_count,
                        "loaded_levels": &identity.loaded_levels,
                    },
                    "current": resources,
                }),
            )?;
            *previous_identity = Some(identity);
            *previous_error = None;
            Ok(())
        }
        Err(error) if previous_error.as_deref() == Some(error.as_str()) => Ok(()),
        Err(error) => {
            *previous_error = Some(error.clone());
            write_json_line(
                writer,
                &json!({
                    "record_kind": "sound_resource_catalog_error",
                    "sample": sample,
                    "elapsed_ms": elapsed_ms,
                    "error": error,
                }),
            )
        }
    }
}

fn validate_request(request: &WorldTimelineRequest<'_>) -> Result<(), String> {
    if !request.seconds.is_finite() || !(0.05..=3600.0).contains(&request.seconds) {
        return Err("--seconds must be between 0.05 and 3600".into());
    }
    if !(1..=1000).contains(&request.hz) {
        return Err("--hz must be between 1 and 1000".into());
    }
    for (name, rate) in [
        ("--context-hz", request.context_hz),
        ("--evidence-hz", request.evidence_hz),
        ("--terrain-hz", request.terrain_hz),
    ] {
        if rate == 0 || rate > request.hz {
            return Err(format!("{name} must be between 1 and --hz"));
        }
    }
    if request.entity_types.len() > MAX_EXPLICIT_FILTERS
        || request.model_ids.len() > MAX_EXPLICIT_FILTERS
        || request.handles.len() > MAX_EXPLICIT_FILTERS
    {
        return Err(format!(
            "at most {MAX_EXPLICIT_FILTERS} explicit type, model, or handle filters are allowed"
        ));
    }
    if request
        .entity_types
        .iter()
        .any(|entity_type| *entity_type > 129)
    {
        return Err("--type must be between 0 and 129".into());
    }
    if request
        .model_ids
        .iter()
        .any(|model_id| *model_id > u32::from(u16::MAX))
    {
        return Err("--model must be between 0 and 65535".into());
    }
    if request.nearest_player > MAX_NEAREST_ACTORS {
        return Err(format!(
            "--nearest-player must not exceed {MAX_NEAREST_ACTORS}"
        ));
    }
    Ok(())
}

fn cadence_due(sample: u64, primary_hz: u32, secondary_hz: u32) -> bool {
    sample == 0
        || sample * u64::from(secondary_hz) / u64::from(primary_hz)
            != (sample - 1) * u64::from(secondary_hz) / u64::from(primary_hz)
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
    use super::{
        cadence_due, classify_existing_pointer_values, lifecycle_changed, select_actor_pointers,
        select_actors, ActorSelectionCandidate, ExistingPointerEvent,
    };
    use crate::entity::EntityRecord;

    #[test]
    fn cadence_crossings_work_for_non_divisible_rates() {
        let due = (0..10)
            .filter(|sample| cadence_due(*sample, 10, 3))
            .collect::<Vec<_>>();
        assert_eq!(due, vec![0, 4, 7]);
    }

    #[test]
    fn nearest_selection_retains_player_and_uses_wrapping_distance() {
        let candidates = [
            ActorSelectionCandidate {
                intrusive_list_index: 0,
                pointer: 1,
                entity_type: 46,
                active_model: 41,
                handle: 10,
                position_raw_8_8: [32_760, 0, 0],
            },
            ActorSelectionCandidate {
                intrusive_list_index: 1,
                pointer: 2,
                entity_type: 17,
                active_model: 256,
                handle: 20,
                position_raw_8_8: [-32_760, 0, 0],
            },
            ActorSelectionCandidate {
                intrusive_list_index: 2,
                pointer: 3,
                entity_type: 47,
                active_model: 302,
                handle: 30,
                position_raw_8_8: [32_660, 0, 0],
            },
        ];

        assert_eq!(
            select_actor_pointers(&candidates, &[], &[], &[], 1),
            vec![1, 2]
        );
        assert_eq!(
            select_actor_pointers(&candidates, &[], &[302], &[], 0),
            vec![3]
        );
    }

    #[test]
    fn pointer_reuse_takes_precedence_over_an_ordinary_state_change() {
        assert_eq!(
            classify_existing_pointer_values(10, 17, 11, 17, true),
            ExistingPointerEvent::PointerReuse
        );
        assert_eq!(
            classify_existing_pointer_values(10, 17, 10, 17, true),
            ExistingPointerEvent::StateChange
        );
        assert_eq!(
            classify_existing_pointer_values(10, 17, 10, 17, false),
            ExistingPointerEvent::None
        );
    }

    // Keep this compile-time bridge independent of a fabricated 0xCC-byte
    // record; the pure selection and lifecycle policies are tested above.
    #[allow(dead_code)]
    fn type_check_helpers(records: &[EntityRecord]) {
        let _ = select_actors(records, &[46], &[], &[], 4);
        if let Some(record) = records.first() {
            let _ = lifecycle_changed(record, record);
        }
    }
}
