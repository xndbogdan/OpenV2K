#[cfg(not(windows))]
fn main() {
    eprintln!("v2k-original-inspect is only supported on Windows");
    std::process::exit(1);
}

#[cfg(windows)]
mod audio;
#[cfg(windows)]
mod collision_health;
#[cfg(windows)]
mod collision_timeline;
#[cfg(windows)]
mod controller;
#[cfg(windows)]
mod entity;
#[cfg(windows)]
mod event_stream;
#[cfg(windows)]
mod factory_audio;
#[cfg(windows)]
mod fan_audio;
#[cfg(windows)]
mod fire_state;
#[cfg(windows)]
mod foreground_input;
#[cfg(windows)]
mod menu;
#[cfg(windows)]
mod menu_pose;
#[cfg(windows)]
mod model;
#[cfg(windows)]
mod particle;
#[cfg(windows)]
mod particle_gate;
#[cfg(windows)]
mod physics;
#[cfg(windows)]
mod process;
#[cfg(windows)]
mod radar_timeline;
#[cfg(windows)]
mod render;
#[cfg(windows)]
mod resources;
#[cfg(windows)]
mod sound_resource;
#[cfg(windows)]
mod terrain_infection;
#[cfg(windows)]
mod timeline;
#[cfg(windows)]
mod world_timeline;

#[cfg(windows)]
fn main() {
    if let Err(error) = app::run() {
        eprintln!("v2k-original-inspect: {error}");
        std::process::exit(1);
    }
}

#[cfg(windows)]
mod app {
    use std::collections::{HashMap, HashSet};
    use std::fs::{create_dir_all, File};
    use std::io::{BufWriter, Write};
    use std::path::{Path, PathBuf};
    use std::thread::sleep;
    use std::time::{Duration, Instant};

    use clap::{Parser, Subcommand, ValueEnum};
    use serde::Serialize;
    use serde_json::json;

    use crate::audio::SoundSignature;
    use crate::entity::{EntityRecord, EntitySignature};
    use crate::event_stream::{write_sound_event, AudioTimelineState, ParticleTimelineState};
    use crate::process::{BuildFingerprint, Process};
    use crate::timeline::{
        ensure_distinct_capture_paths, ensure_stop_file_absent, run as run_timeline, run_stop_aware,
    };

    type SoundResourceSource = (u32, u32, u32, u32, u32);
    type SoundResourceSourceKey = (u32, Vec<SoundResourceSource>);

    const TERRAIN_MATERIAL_SELECTOR_MAP: usize = 0x004F_EC48;
    const SURFACE_PARTICLE_CLASS_TABLE: usize = 0x004C_D750;
    const SURFACE_RESPONSE_TABLE_BYTES: usize = 0x20;

    #[derive(Debug, Serialize)]
    struct SurfaceMaterialResponseCatalog {
        runtime_material_to_selector_address: u32,
        runtime_material_to_selector: [i32; 8],
        selector_to_particle_class_address: u32,
        selector_to_particle_class: [i32; 8],
    }

    #[derive(Parser)]
    #[command(
        name = "v2k-original-inspect",
        about = "Read-only runtime probes for the fixed-base retail V2000.EXE"
    )]
    struct Cli {
        /// Select a process when more than one original V2000 instance is running.
        #[arg(long, global = true)]
        pid: Option<u32>,

        /// Wait this many seconds for a supported original process to appear.
        #[arg(long, global = true, default_value_t = 0.0)]
        wait_for_process_seconds: f64,

        /// Require an auto-detected process to remain readable this long.
        #[arg(long, global = true, default_value_t = 0)]
        stable_process_milliseconds: u64,

        #[command(subcommand)]
        command: Option<Command>,
    }

    #[derive(Subcommand)]
    enum Command {
        /// Take one complete live entity-list snapshot (the default command).
        Entities {
            #[arg(long, value_enum, default_value_t = SnapshotFormat::Text)]
            format: SnapshotFormat,
            #[arg(short, long)]
            output: Option<PathBuf>,
        },

        /// Sample one entity, its raw record, and bounded behavior/component state.
        TrackEntity {
            #[arg(long = "type", default_value_t = 46)]
            entity_type: u32,
            /// Optional decimal or 0x-prefixed entity handle filter.
            #[arg(long, value_parser = parse_u32)]
            handle: Option<u32>,
            #[arg(long, default_value_t = 20.0)]
            seconds: f64,
            #[arg(long, default_value_t = 50)]
            hz: u32,
            /// Correlate foreground Hover keys and the live controller motion vector.
            #[arg(long)]
            capture_player_controls: bool,
            /// Retain first/previous-sample VTOL evidence around the live Sub-G block.
            #[arg(long, requires = "capture_player_controls")]
            capture_vtol_evidence: bool,
            /// Retain the camera spring/output and active render settings at sample time.
            #[arg(long)]
            capture_render_evidence: bool,
            /// Stop before the next sample when this externally managed marker exists.
            #[arg(long)]
            stop_file: Option<PathBuf>,
            #[arg(short, long)]
            output: PathBuf,
        },

        /// Correlate skimmer/VTOL surface effects with particles, audio, and ride-surface state.
        VtolSurfaceEffectsTimeline {
            #[arg(long, default_value_t = 180.0)]
            seconds: f64,
            #[arg(long, default_value_t = 200)]
            hz: u32,
            /// Lower-rate entity, controller, and terrain/wave context sampling rate.
            #[arg(long, default_value_t = 100)]
            context_hz: u32,
            /// Stop before the next sample when this externally managed marker exists.
            #[arg(long)]
            stop_file: Option<PathBuf>,
            #[arg(short, long)]
            output: PathBuf,
        },

        /// Follow one entity with particles, audio, and bounded runtime evidence.
        EntityEffectsTimeline {
            #[arg(long = "type", value_parser = parse_u32)]
            entity_type: u32,
            #[arg(long, value_parser = parse_u32)]
            handle: Option<u32>,
            #[arg(long, default_value_t = 180.0)]
            seconds: f64,
            #[arg(long, default_value_t = 200)]
            hz: u32,
            #[arg(long, default_value_t = 50)]
            context_hz: u32,
            /// For type 46, retain craft controls and controller state.
            #[arg(long)]
            capture_player_controls: bool,
            /// For type 46, also retain Sub-G and ride-surface evidence.
            #[arg(long, requires = "capture_player_controls")]
            capture_vtol_evidence: bool,
            /// Retain the camera spring/output and active render settings at context cadence.
            #[arg(long)]
            capture_render_evidence: bool,
            #[arg(long)]
            stop_file: Option<PathBuf>,
            #[arg(short, long)]
            output: PathBuf,
        },

        /// Capture the type-46 lethal burst and wreck handoff with full effects.
        PlayerWreckTimeline {
            #[arg(long, default_value_t = 120.0)]
            seconds: f64,
            #[arg(long, default_value_t = 500)]
            hz: u32,
            #[arg(long, default_value_t = 100)]
            context_hz: u32,
            #[arg(long)]
            stop_file: Option<PathBuf>,
            #[arg(short, long)]
            output: PathBuf,
        },

        /// Correlate the right terrain radar and M fullscreen map with their retail buffers.
        RadarTimeline {
            #[arg(long, default_value_t = 300.0)]
            seconds: f64,
            #[arg(long, default_value_t = 100)]
            hz: u32,
            /// Entity, camera, menu, and raw session/controller context rate.
            #[arg(long, default_value_t = 20)]
            context_hz: u32,
            /// Duplicate-read/hash rate for the 96 KiB radar terrain/coverage inputs.
            #[arg(long, default_value_t = 10)]
            buffer_hz: u32,
            /// Stop before the next sample when this externally managed marker exists.
            #[arg(long)]
            stop_file: Option<PathBuf>,
            #[arg(short, long)]
            output: PathBuf,
        },

        /// Snapshot radar projection, color, and fullscreen-map icon resources.
        RadarResources {
            #[arg(short, long)]
            output: PathBuf,
        },

        /// Capture compact player collision/health evidence through destruction and menu return.
        CollisionHealthTimeline {
            #[arg(long, default_value_t = 300.0)]
            seconds: f64,
            #[arg(long, default_value_t = 100)]
            hz: u32,
            #[arg(short, long)]
            output: PathBuf,
        },

        /// Record entity births, deaths, model-slot changes, and behavior changes.
        EntityEvents {
            #[arg(long, default_value_t = 150.0)]
            seconds: f64,
            #[arg(long, default_value_t = 20)]
            hz: u32,
            #[arg(short, long)]
            output: PathBuf,
        },

        /// Sample the shared main-menu/submenu stack, transition clocks, and settings.
        MenuTimeline {
            #[arg(long, default_value_t = 45.0)]
            seconds: f64,
            #[arg(long, default_value_t = 60)]
            hz: u32,
            #[arg(short, long)]
            output: PathBuf,
        },

        /// Correlate menu state, Klaus/flag poses, camera output, and projection globals.
        MenuPoseTimeline {
            #[arg(long, default_value_t = 180.0)]
            seconds: f64,
            #[arg(long, default_value_t = 100)]
            hz: u32,
            /// Stop before the next sample when this externally managed marker exists.
            #[arg(long)]
            stop_file: Option<PathBuf>,
            #[arg(short, long)]
            output: PathBuf,
        },

        /// Sample fog planes, view transform, terrain scan bounds, and water selection.
        RenderTimeline {
            #[arg(long, default_value_t = 20.0)]
            seconds: f64,
            #[arg(long, default_value_t = 50)]
            hz: u32,
            #[arg(short, long)]
            output: PathBuf,
        },

        /// Record active DirectSound voice start/update/stop events.
        AudioEvents {
            #[arg(long, default_value_t = 20.0)]
            seconds: f64,
            #[arg(long, default_value_t = 200)]
            hz: u32,
            #[arg(short, long)]
            output: PathBuf,
        },

        /// Join live DirectSound voices to factory-cycle entity +0x8C handles.
        FactoryManufacturingAudio {
            #[arg(long, default_value_t = 20.0)]
            seconds: f64,
            #[arg(long, default_value_t = 50)]
            hz: u32,
            #[arg(long)]
            stop_file: Option<PathBuf>,
            #[arg(short, long)]
            output: PathBuf,
        },

        /// Correlate Hover thrust controls with every live DirectSound voice.
        FanAudioTimeline {
            #[arg(long, default_value_t = 90.0)]
            seconds: f64,
            #[arg(long, default_value_t = 200)]
            hz: u32,
            /// Stop before the next sample when this externally managed marker exists.
            #[arg(long)]
            stop_file: Option<PathBuf>,
            #[arg(short, long)]
            output: PathBuf,
        },

        /// Record stable births, deaths, and slot reuse in the fixed particle pool.
        ParticleEvents {
            #[arg(long, default_value_t = 30.0)]
            seconds: f64,
            #[arg(long, default_value_t = 200)]
            hz: u32,
            #[arg(short, long)]
            output: PathBuf,
        },

        /// Pair raw particle presentation state with every distinct stable pool state.
        ParticleRenderGateTimeline {
            /// Scene label such as Intro2, Level1, or OtherWorld.
            #[arg(long)]
            scene: String,
            #[arg(long, default_value_t = 180.0)]
            seconds: f64,
            #[arg(long, default_value_t = 200)]
            hz: u32,
            #[arg(long)]
            stop_file: Option<PathBuf>,
            #[arg(short, long)]
            output: PathBuf,
        },

        /// Correlate fire/selection controls with Power Up state, shots, entities, particles, and audio.
        FireEvents {
            #[arg(long, default_value_t = 20.0)]
            seconds: f64,
            /// High-frequency player shot-list/input sampling rate.
            #[arg(long, default_value_t = 1000)]
            hz: u32,
            /// Slower main-entity and DirectSound context sampling rate.
            #[arg(long, default_value_t = 200)]
            context_hz: u32,
            /// Duplicate full Section-10 terrain comparison rate when enabled.
            #[arg(long, default_value_t = 10)]
            terrain_hz: u32,
            /// Retain exact terrain-cell infection-bit transitions alongside shots.
            #[arg(long)]
            capture_terrain_infection: bool,
            /// Stop before the next sample when this externally managed marker exists.
            #[arg(long)]
            stop_file: Option<PathBuf>,
            #[arg(short, long)]
            output: PathBuf,
        },

        /// Follow startup, frontend, Intro2, and first-world resources in one run.
        SessionTimeline {
            #[arg(long, default_value_t = 360.0)]
            seconds: f64,
            #[arg(long, default_value_t = 50)]
            hz: u32,
            #[arg(short, long)]
            output: PathBuf,
        },

        /// Correlate actor state/lifecycle with optional effects and terrain infection.
        WorldBehaviorTimeline {
            /// Protocol label only; it never changes interpretation or selection.
            #[arg(long, default_value = "world-lifecycle")]
            scenario: String,
            /// Retain every actor of this type. May be repeated.
            #[arg(long = "type", value_parser = parse_u32)]
            entity_types: Vec<u32>,
            /// Retain every actor whose active model has this global ID. May be repeated.
            #[arg(long = "model", value_parser = parse_u32)]
            model_ids: Vec<u32>,
            /// Retain this exact entity handle. May be repeated.
            #[arg(long, value_parser = parse_u32)]
            handle: Vec<u32>,
            /// Retain type 46 plus this many closest non-player actors.
            #[arg(long, default_value_t = 16)]
            nearest_player: usize,
            #[arg(long, default_value_t = 300.0)]
            seconds: f64,
            /// Particle/audio/input sampling rate.
            #[arg(long, default_value_t = 200)]
            hz: u32,
            /// Stable full-entity and compact actor sampling rate.
            #[arg(long, default_value_t = 50)]
            context_hz: u32,
            /// Bounded behavior/component/type callback evidence rate.
            #[arg(long, default_value_t = 20)]
            evidence_hz: u32,
            /// Duplicate full Section-10 terrain comparison rate when enabled.
            #[arg(long, default_value_t = 5)]
            terrain_hz: u32,
            #[arg(long)]
            capture_effects: bool,
            #[arg(long)]
            capture_terrain_infection: bool,
            #[arg(long)]
            stop_file: Option<PathBuf>,
            #[arg(short, long)]
            output: PathBuf,
        },
    }

    #[derive(Debug, Clone, Copy, ValueEnum)]
    enum SnapshotFormat {
        Text,
        Json,
    }

    #[derive(Serialize)]
    struct CaptureMeta<'a> {
        record_kind: &'static str,
        tool_version: &'static str,
        command: &'a str,
        process_id: u32,
        executable: &'a str,
        build: &'a BuildFingerprint,
    }

    #[derive(Serialize)]
    struct EntityDocument<'a> {
        meta: CaptureMeta<'a>,
        snapshot: &'a crate::entity::EntitySnapshot,
    }

    #[derive(Debug, Serialize)]
    struct CameraEyeSurfaceEvidence {
        render_tick_before: u32,
        render_tick_after: u32,
        render_tick_stable: bool,
        camera_duplicate_stable: bool,
        pair_tick_contiguous: bool,
        derived_from_joint_stable_sample: bool,
        surface: crate::physics::WorldSurfaceProbeEvidence,
    }

    fn camera_eye_surface_pair_is_joint_stable(
        render_tick_stable: bool,
        camera_duplicate_stable: bool,
        render_tick_after: u32,
        surface_tick_before: Option<u32>,
        surface_tick_stable: Option<bool>,
    ) -> bool {
        render_tick_stable
            && camera_duplicate_stable
            && surface_tick_before == Some(render_tick_after)
            && surface_tick_stable == Some(true)
    }

    fn pair_camera_eye_surface_evidence(
        render: &crate::render::RenderSnapshot,
        mut surface: crate::physics::WorldSurfaceProbeEvidence,
    ) -> CameraEyeSurfaceEvidence {
        let pair_tick_contiguous = surface.tick_before == Some(render.tick_after);
        let derived_from_joint_stable_sample = camera_eye_surface_pair_is_joint_stable(
            render.tick_stable,
            render.camera_duplicate_stable,
            render.tick_after,
            surface.tick_before,
            surface.tick_stable,
        );
        if !derived_from_joint_stable_sample {
            surface.height_above_ride_surface_raw = None;
            surface.height_above_ride_surface_cells = None;
        }

        CameraEyeSurfaceEvidence {
            render_tick_before: render.tick_before,
            render_tick_after: render.tick_after,
            render_tick_stable: render.tick_stable,
            camera_duplicate_stable: render.camera_duplicate_stable,
            pair_tick_contiguous,
            derived_from_joint_stable_sample,
            surface,
        }
    }

    struct EntityTrackRequest<'a> {
        command_name: &'static str,
        entity_type: u32,
        handle: Option<u32>,
        seconds: f64,
        hz: u32,
        context_hz: u32,
        capture_player_controls: bool,
        capture_vtol_evidence: bool,
        capture_render_evidence: bool,
        capture_particle_events: bool,
        capture_audio_events: bool,
        capture_sound_resource_catalog: bool,
        stop_file: Option<&'a Path>,
        output: &'a Path,
    }

    #[derive(Clone, Copy)]
    struct InitialVtolRuntimeSample {
        pointer: u32,
        random_sample_at_0x38: i32,
        tick_before: u32,
        tick_after: u32,
    }

    fn read_loaded_sound_resource_catalog(
        process: &Process,
    ) -> Result<crate::sound_resource::SoundResourceCatalog, String> {
        let pools = crate::resources::read(process)?;
        let catalog = crate::resources::read_catalog(process)?;
        crate::sound_resource::read(process, pools.sounds, catalog.section_totals[11])
    }

    fn read_surface_material_response_catalog(
        process: &Process,
    ) -> Result<SurfaceMaterialResponseCatalog, String> {
        Ok(SurfaceMaterialResponseCatalog {
            runtime_material_to_selector_address: TERRAIN_MATERIAL_SELECTOR_MAP as u32,
            runtime_material_to_selector: read_i32_table_8(process, TERRAIN_MATERIAL_SELECTOR_MAP)?,
            selector_to_particle_class_address: SURFACE_PARTICLE_CLASS_TABLE as u32,
            selector_to_particle_class: read_i32_table_8(process, SURFACE_PARTICLE_CLASS_TABLE)?,
        })
    }

    fn read_i32_table_8(process: &Process, address: usize) -> Result<[i32; 8], String> {
        let bytes = process.read_bytes(address, SURFACE_RESPONSE_TABLE_BYTES)?;
        let mut values = [0; 8];
        for (value, chunk) in values.iter_mut().zip(bytes.chunks_exact(4)) {
            *value = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        Ok(values)
    }

    fn cadence_due(sample: u64, primary_hz: u32, context_hz: u32) -> bool {
        sample == 0
            || sample * u64::from(context_hz) / u64::from(primary_hz)
                != (sample - 1) * u64::from(context_hz) / u64::from(primary_hz)
    }

    pub fn run() -> Result<(), String> {
        let cli = Cli::parse();
        if !cli.wait_for_process_seconds.is_finite() || cli.wait_for_process_seconds < 0.0 {
            return Err("--wait-for-process-seconds must be a finite non-negative number".into());
        }
        let process = if cli.wait_for_process_seconds > 0.0 {
            Process::attach_wait(
                cli.pid,
                Duration::from_secs_f64(cli.wait_for_process_seconds),
                Duration::from_millis(cli.stable_process_milliseconds),
            )?
        } else {
            Process::attach(cli.pid)?
        };
        if cli.wait_for_process_seconds > 0.0 {
            eprintln!(
                "attached to stable {} process (PID {})",
                process.exe_name, process.process_id
            );
        }
        match cli.command.unwrap_or(Command::Entities {
            format: SnapshotFormat::Text,
            output: None,
        }) {
            Command::Entities { format, output } => {
                capture_entities(&process, format, output.as_deref())
            }
            Command::TrackEntity {
                entity_type,
                handle,
                seconds,
                hz,
                capture_player_controls,
                capture_vtol_evidence,
                capture_render_evidence,
                stop_file,
                output,
            } => capture_entity_track(
                &process,
                EntityTrackRequest {
                    command_name: "track-entity",
                    entity_type,
                    handle,
                    seconds,
                    hz,
                    context_hz: hz,
                    capture_player_controls,
                    capture_vtol_evidence,
                    capture_render_evidence,
                    capture_particle_events: false,
                    capture_audio_events: false,
                    capture_sound_resource_catalog: false,
                    stop_file: stop_file.as_deref(),
                    output: &output,
                },
            ),
            Command::VtolSurfaceEffectsTimeline {
                seconds,
                hz,
                context_hz,
                stop_file,
                output,
            } => capture_entity_track(
                &process,
                EntityTrackRequest {
                    command_name: "vtol-surface-effects-timeline",
                    entity_type: 46,
                    handle: None,
                    seconds,
                    hz,
                    context_hz,
                    capture_player_controls: true,
                    capture_vtol_evidence: true,
                    capture_render_evidence: false,
                    capture_particle_events: true,
                    capture_audio_events: true,
                    capture_sound_resource_catalog: true,
                    stop_file: stop_file.as_deref(),
                    output: &output,
                },
            ),
            Command::EntityEffectsTimeline {
                entity_type,
                handle,
                seconds,
                hz,
                context_hz,
                capture_player_controls,
                capture_vtol_evidence,
                capture_render_evidence,
                stop_file,
                output,
            } => capture_entity_track(
                &process,
                EntityTrackRequest {
                    command_name: "entity-effects-timeline",
                    entity_type,
                    handle,
                    seconds,
                    hz,
                    context_hz,
                    capture_player_controls,
                    capture_vtol_evidence,
                    capture_render_evidence,
                    capture_particle_events: true,
                    capture_audio_events: true,
                    capture_sound_resource_catalog: true,
                    stop_file: stop_file.as_deref(),
                    output: &output,
                },
            ),
            Command::PlayerWreckTimeline {
                seconds,
                hz,
                context_hz,
                stop_file,
                output,
            } => capture_entity_track(
                &process,
                EntityTrackRequest {
                    command_name: "player-wreck-timeline",
                    entity_type: 46,
                    handle: None,
                    seconds,
                    hz,
                    context_hz,
                    capture_player_controls: true,
                    capture_vtol_evidence: true,
                    capture_render_evidence: false,
                    capture_particle_events: true,
                    capture_audio_events: true,
                    capture_sound_resource_catalog: true,
                    stop_file: stop_file.as_deref(),
                    output: &output,
                },
            ),
            Command::RadarTimeline {
                seconds,
                hz,
                context_hz,
                buffer_hz,
                stop_file,
                output,
            } => crate::radar_timeline::capture(
                &process,
                seconds,
                hz,
                context_hz,
                buffer_hz,
                stop_file.as_deref(),
                &output,
            ),
            Command::RadarResources { output } => {
                crate::radar_timeline::capture_resources(&process, &output)
            }
            Command::CollisionHealthTimeline {
                seconds,
                hz,
                output,
            } => crate::collision_timeline::capture(&process, seconds, hz, &output),
            Command::EntityEvents {
                seconds,
                hz,
                output,
            } => capture_entity_events(&process, seconds, hz, &output),
            Command::MenuTimeline {
                seconds,
                hz,
                output,
            } => capture_menu_timeline(&process, seconds, hz, &output),
            Command::MenuPoseTimeline {
                seconds,
                hz,
                stop_file,
                output,
            } => crate::menu_pose::capture(
                &process,
                crate::menu_pose::MenuPoseTimelineRequest {
                    seconds,
                    hz,
                    stop_file: stop_file.as_deref(),
                    output: &output,
                },
            ),
            Command::RenderTimeline {
                seconds,
                hz,
                output,
            } => capture_render_timeline(&process, seconds, hz, &output),
            Command::AudioEvents {
                seconds,
                hz,
                output,
            } => capture_audio_events(&process, seconds, hz, &output),
            Command::FactoryManufacturingAudio {
                seconds,
                hz,
                stop_file,
                output,
            } => {
                crate::factory_audio::capture(&process, seconds, hz, stop_file.as_deref(), &output)
            }
            Command::FanAudioTimeline {
                seconds,
                hz,
                stop_file,
                output,
            } => capture_fan_audio_timeline(&process, seconds, hz, stop_file.as_deref(), &output),
            Command::ParticleEvents {
                seconds,
                hz,
                output,
            } => capture_particle_events(&process, seconds, hz, &output),
            Command::ParticleRenderGateTimeline {
                scene,
                seconds,
                hz,
                stop_file,
                output,
            } => crate::particle_gate::capture(
                &process,
                &scene,
                seconds,
                hz,
                stop_file.as_deref(),
                &output,
            ),
            Command::FireEvents {
                seconds,
                hz,
                context_hz,
                terrain_hz,
                capture_terrain_infection,
                stop_file,
                output,
            } => capture_fire_events(
                &process,
                seconds,
                hz,
                context_hz,
                terrain_hz,
                capture_terrain_infection,
                stop_file.as_deref(),
                &output,
            ),
            Command::SessionTimeline {
                seconds,
                hz,
                output,
            } => capture_session_timeline(&process, seconds, hz, &output),
            Command::WorldBehaviorTimeline {
                scenario,
                entity_types,
                model_ids,
                handle,
                nearest_player,
                seconds,
                hz,
                context_hz,
                evidence_hz,
                terrain_hz,
                capture_effects,
                capture_terrain_infection,
                stop_file,
                output,
            } => crate::world_timeline::capture(
                &process,
                crate::world_timeline::WorldTimelineRequest {
                    scenario: &scenario,
                    entity_types: &entity_types,
                    model_ids: &model_ids,
                    handles: &handle,
                    nearest_player,
                    seconds,
                    hz,
                    context_hz,
                    evidence_hz,
                    terrain_hz,
                    capture_effects,
                    capture_terrain_infection,
                    stop_file: stop_file.as_deref(),
                    output: &output,
                },
            ),
        }
    }

    fn capture_entities(
        process: &Process,
        format: SnapshotFormat,
        output: Option<&Path>,
    ) -> Result<(), String> {
        let snapshot = crate::entity::read_snapshot(process)?;
        let mut writer: Box<dyn Write> = match output {
            Some(path) => Box::new(create_writer(path)?),
            None => Box::new(std::io::stdout()),
        };
        match format {
            SnapshotFormat::Text => crate::entity::write_text(process, &snapshot, &mut writer)
                .map_err(|error| error.to_string())?,
            SnapshotFormat::Json => {
                let document = EntityDocument {
                    meta: capture_meta(process, "entities"),
                    snapshot: &snapshot,
                };
                serde_json::to_writer_pretty(&mut writer, &document)
                    .map_err(|error| error.to_string())?;
                writeln!(writer).map_err(|error| error.to_string())?;
            }
        }
        writer.flush().map_err(|error| error.to_string())
    }

    fn capture_entity_track(
        process: &Process,
        request: EntityTrackRequest<'_>,
    ) -> Result<(), String> {
        let EntityTrackRequest {
            command_name,
            entity_type,
            handle,
            seconds,
            hz,
            context_hz,
            capture_player_controls,
            capture_vtol_evidence,
            capture_render_evidence,
            capture_particle_events,
            capture_audio_events,
            capture_sound_resource_catalog,
            stop_file,
            output,
        } = request;
        validate_timeline(seconds, hz, 1000)?;
        if !(1..=1000).contains(&context_hz) {
            return Err("--context-hz must be between 1 and 1000".into());
        }
        if capture_player_controls && entity_type != 46 {
            return Err("--capture-player-controls requires --type 46".into());
        }
        if capture_vtol_evidence && entity_type != 46 {
            return Err("--capture-vtol-evidence requires --type 46".into());
        }
        ensure_distinct_capture_paths(stop_file, output)?;
        ensure_stop_file_absent(stop_file)?;
        let sound_resource_catalog =
            capture_sound_resource_catalog.then(|| read_loaded_sound_resource_catalog(process));
        let surface_material_response =
            capture_particle_events.then(|| read_surface_material_response_catalog(process));
        let mut writer = create_writer(output)?;
        write_json_line(
            &mut writer,
            &json!({
                "meta": capture_meta(process, command_name),
                "entity_type": entity_type,
                "handle": handle,
                "seconds": seconds,
                "hz": hz,
                "context_hz": context_hz,
                "capture_player_controls": capture_player_controls,
                "capture_vtol_evidence": capture_vtol_evidence,
                "capture_render_evidence": capture_render_evidence,
                "capture_particle_events": capture_particle_events,
                "capture_audio_events": capture_audio_events,
                "effects_sample_policy": (capture_particle_events || capture_audio_events).then_some(json!({
                    "primary_hz": hz,
                    "entity_surface_context_hz": context_hz,
                    "sample_order": "foreground input, optional entity/VTOL context, duplicate-read particle pool, active-sound list",
                    "particle_motion": "the 200 Hz duplicate-read sampler detects lifecycle changes and retains every distinct observationally stable complete active-pool state, including age, position, velocity, class, source, flags, cached surface, and raw bytes; byte-identical repeats are suppressed without using the pre-update 50 Hz clock as a deduplication key",
                    "terrain_material_response": "each exact Section-10 three-byte corner cell retains its packed material/shade byte; the capture-start catalog records the live eight-entry DAT_004FEC48 material-to-selector map and the verified-build 0x004CD750 selector-to-particle-class table",
                    "audio_identity": "start/update/stop events retain PCM pointer and byte length; the capture-start resource catalog provides candidate global IDs without requiring a successful PCM hash",
                    "water_entry_static_gate": "FUN_004129B0 dispatches the authored surface program on fully-above flag 0x400000 set -> clear; captured velocity measures entry intensity but is not assumed to be a minimum threshold",
                    "separate_buoyancy_gate": "FUN_0040E100 common underwater response uses static-surface center depth < -100; do not conflate this with first wave intersection",
                })),
                "stop_file": stop_file.map(|path| path.display().to_string()),
                "stop_file_policy": stop_file.map(|_| json!({
                    "ownership": "external; the inspector never creates, removes, or modifies the marker",
                    "startup": "refuse capture if the marker already exists",
                    "polling": "check after each sample deadline and stop before taking the next sample when the marker exists",
                    "read_error": "abort capture rather than silently ignore an unreadable marker path",
                })),
                "foreground_key_controls": capture_player_controls.then_some([
                    "space", "right_shift", "up", "down", "left", "right", "s", "x", "tab", "m"
                ]),
                "foreground_protocol_controls": capture_player_controls.then_some([
                    "caps_lock"
                ]),
                "foreground_control_sample_order": capture_player_controls.then_some([
                    "space", "right_shift", "up", "down", "left", "right", "s", "x", "tab", "m", "caps_lock"
                ]),
                "foreground_key_source": capture_player_controls.then_some(
                    "Win32 GetAsyncKeyState high bit while the attached V2000 process owns the foreground window; correlation evidence, not the game's DirectInput buffer"
                ),
                "input_edge_policy": capture_player_controls.then_some(
                    "first focused sample is a baseline; losing focus clears the baseline; returning cannot synthesize release edges"
                ),
                "caps_lock_protocol": capture_player_controls.then_some(json!({
                    "source": "Win32 GetAsyncKeyState high bit while the attached V2000 process owns the foreground window",
                    "role": "guided-capture protocol delimiter; retail also binds Caps Lock to Previous Weapon, so post-retry inventory selection is explicitly contaminated",
                    "edge_meaning": "a physical press edge delimits an invalid attempt that the external runner may stop and retry after release",
                    "sampling_reason": "physical state is process-independent; the Windows toggle bit is deliberately not used because it belongs to the calling thread's keyboard-message queue",
                })),
                "controller_motion_vector": capture_player_controls.then_some(json!({
                    "chain": "*(0x004F72C8) -> session +0x27C -> controller",
                    "entity_handle_at_controller_offset": "0x68",
                    "offset": "0x280",
                    "byte_length": 16,
                    "neutral_raw_fields": ["i16@00", "i16@02", "i16@04", "i16@06", "i32@08", "i32@0C"],
                    "evidence_backed_decodes": ["turn@00", "pitch@02", "vertical@04", "throttle@08", "fire@0C"],
                    "unknown_field_policy": "signed word +0x06 remains offset-named",
                })),
                "mouse_policy": capture_player_controls.then_some(
                    "no OS cursor position or delta is sampled; analogue/mouse effects must be inferred only from the live controller bytes and later code evidence"
                ),
                "vtol_evidence_policy": capture_vtol_evidence.then_some(json!({
                    "authored_chain": "entity+0x58 type -> *(0x004FE650) type table -> type record -> *(type record+0xE8) static 0x68-byte Sub-G",
                    "runtime_chain": "entity+0x4C -> component root; root+0x0C -> component table; table+0x18 -> per-entity 0x44-byte runtime Sub-G",
                    "session_byte": "raw u8 at *(0x004F72C8) +0x296; FUN_00456F10 interprets it as signed i8",
                    "global_delta_us": "u32 at 0x004D04E4, written by FUN_0044FE20",
                    "dispatch_delta_us": "unavailable_read_only: FUN_00446640::param_3 is a transient stack argument with no proven passive address",
                    "surface_replay": "bounded passive replay of center FUN_00445860/FUN_00445920 and FUN_0041B210's FUN_0041DE60 center/XZ +/-0x100 five-probe minimum; no retail function is invoked",
                    "surface_units": "signed engine 8.8 world units; 256 raw units = one terrain cell; negative clearance/depth means penetration",
                    "surface_stability": "surface-only fields require their narrow tick bracket; derivations using pre-inner model radius, entity type flags, runtime Sub-G, or signed session mode additionally require the enclosing VTOL tick bracket",
                    "surface_probe_order": ["center [0,0]", "x-0x100", "z-0x100", "x+0x100", "z+0x100"],
                    "terrain_corner_array_order": ["x0_z0", "x1_z0", "x0_z1", "x1_z1"],
                    "wave_array_order": ["X", "X+Z", "Z-X"],
                    "altitude_fields": ["height_above_ride_surface_raw", "within_vtol_altitude_assist_window [-299,999]", "lift_limit_clearance_raw", "0x800/0xC00 attenuation threshold and divisors"],
                    "static_water_impact_fields": "common FUN_0040E100 surface=max(integer-cell terrain, static sea), plus center depth < -100; actual response additionally requires effective environment bit 2 clear and the nested type/environment byte nonzero",
                    "controller_chain": "FUN_00443AF0 resolves *(0x004F72C8)+0x27C; FUN_00446640 passes that controller as FUN_00445310 param_1, while controller+0x68 resolves the distinct world entity",
                    "ceiling_confounders": ["fuel i32 at controller+0x88", "turbo capability bit controller+0x197&2", "runtime Sub-G manual lift +0x20, altitude assist +0x24, turbo flag +0x3D", "signed session mode byte at session+0x296; mode 4 bypasses height attenuation only"],
                    "callback_policy": "no callbacks are invoked and process memory is never written",
                })),
                "render_evidence_policy": capture_render_evidence.then_some(json!({
                    "cadence": "sampled once per entity context interval",
                    "contents": "duplicate-bracketed retail camera spring/output, tracked body basis, camera target, dynamic ED10 parameters, fog, terrain scan extent, frame delta, the live Active Camera setting, and a separately tick-bracketed terrain/wave sample beneath the output eye",
                    "joint_stability": "camera-eye clearance is retained only when the render tick is stable, the camera block is duplicate-stable, the surface tick is stable, and the surface read begins on the render read's ending tick",
                    "comparison_rule": "compare camera output against the same sample's player pose/body basis and ride-surface evidence; operator controls are correlation evidence, not assumed-identical inputs",
                })),
                "runtime_evidence": [
                    "type record +0x78 model callback pair: exact 8-byte pair plus a bounded 0x40-byte best-effort window at its opaque second dword; callback address is recorded but never invoked",
                    "bounded behavior window at entity +0xB8",
                    "live entity +0xC8 environment flags, their Section-12 +0xC0 source, and behavior masks",
                    "type record +0xD8 underwater-response config and signed enable byte +0x0C",
                    "bounded wind state at 0x004F7194..0x004F71AD",
                    "bounded FUN_0040A800 component chain rooted at entity +0x4C",
                    "FUN_00401120 component callback/state windows, including FUN_00401290 pair-contact callback/context +0x18/+0x1C",
                    "type record +0x7C callback table, including generic hit +0x30, pair dispatcher +0x38, and relation-release dispatcher +0x48",
                    "current behavior style release +0x0C, pair +0x18, and death +0x2C callbacks",
                    "Section-12 type-record +0x88..+0x8E active-pair contact-sound words",
                    "optional entity+0x4C -> root+0x0C -> table+0x30 linked damage/progression state",
                    "intrusive list index/next pointer, exact entity +0x08 pair-decision bits, +0x60/+0x68 recent-relation gate, +0x6C scheduler accumulator, +0x70 subject scan gate, +0xB2 animation offset, and +0xB6 unit-delta override",
                    "entity +0x30 live collision health, +0x44 damage modifier, +0x50 damage buffer, +0x34 supporting last-hit tick, and contact/water/death flag bits",
                    "exact Section-10/Section-9 static-object candidate scan selected by the active model collision radius"
                ],
                "nearby_entity_policy": {
                    "horizontal_radius_raw_8_8": crate::entity::NEARBY_ENTITY_RADIUS_RAW,
                    "horizontal_radius_cells": crate::entity::NEARBY_ENTITY_RADIUS_RAW as f32 / 256.0,
                    "maximum_entities": crate::entity::MAX_NEARBY_ENTITIES,
                    "selection": "nearest non-player entities by toroidal X/Z distance; compact state is emitted every sample so pickup/contact motion and disappearance remain synchronized",
                },
            }),
        )?;
        // The guided wrapper reads this first line to bind its foreground
        // controls to the exact process selected by the inspector. Publish it
        // before the sampler begins instead of relying on BufWriter capacity.
        writer.flush().map_err(|error| error.to_string())?;
        if let Some(catalog) = sound_resource_catalog {
            match catalog {
                Ok(catalog) => write_json_line(
                    &mut writer,
                    &json!({
                        "record_kind": "sound_resource_catalog",
                        "sample": 0,
                        "elapsed_ms": 0.0,
                        "current": catalog,
                    }),
                )?,
                Err(error) => write_json_line(
                    &mut writer,
                    &json!({
                        "record_kind": "sound_resource_catalog_error",
                        "sample": 0,
                        "elapsed_ms": 0.0,
                        "error": error,
                    }),
                )?,
            }
            writer.flush().map_err(|error| error.to_string())?;
        }
        if let Some(catalog) = surface_material_response {
            match catalog {
                Ok(catalog) => write_json_line(
                    &mut writer,
                    &json!({
                        "record_kind": "surface_material_response_catalog",
                        "sample": 0,
                        "elapsed_ms": 0.0,
                        "current": catalog,
                    }),
                )?,
                Err(error) => write_json_line(
                    &mut writer,
                    &json!({
                        "record_kind": "surface_material_response_catalog_error",
                        "sample": 0,
                        "elapsed_ms": 0.0,
                        "error": error,
                    }),
                )?,
            }
            writer.flush().map_err(|error| error.to_string())?;
        }
        let mut previous: Option<EntityRecord> = None;
        let mut previous_input: Option<crate::foreground_input::HoverInputState> = None;
        let mut previous_foreground = None;
        let mut initial_vtol_runtime_sample: Option<InitialVtolRuntimeSample> = None;
        let mut particles = ParticleTimelineState::new(true);
        let mut audio = AudioTimelineState::default();
        let outcome = run_stop_aware(seconds, hz, stop_file, |sample, elapsed_ms| {
            let foreground_input = capture_player_controls
                .then(|| crate::foreground_input::poll_hover_foreground(process.process_id))
                .flatten();
            let foreground_input_edges = match (previous_input, foreground_input) {
                (Some(before), Some(current)) => current.edges_from(before),
                _ => Vec::new(),
            };
            previous_input = foreground_input;
            let v2000_foreground = capture_player_controls.then_some(foreground_input.is_some());
            if capture_particle_events || capture_audio_events {
                write_json_line(
                    &mut writer,
                    &json!({
                        "record_kind": "surface_effects_input_sample",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "v2000_foreground": v2000_foreground,
                        "foreground_input_state": foreground_input,
                        "foreground_input_edges": &foreground_input_edges,
                    }),
                )?;
            }
            if let Some(foreground) = v2000_foreground {
                if previous_foreground != Some(foreground) {
                    write_json_line(
                        &mut writer,
                        &json!({
                            "record_kind": "hover_input_scope",
                            "sample": sample,
                            "elapsed_ms": elapsed_ms,
                            "v2000_foreground": foreground,
                        }),
                    )?;
                    previous_foreground = Some(foreground);
                }
            }
            let context_due = cadence_due(sample, hz, context_hz);
            if context_due {
                match crate::entity::read_snapshot(process) {
                    Ok(snapshot) => {
                        let record = snapshot
                            .records
                            .iter()
                            .find(|record| {
                                record.entity_type == entity_type
                                    && handle.is_none_or(|expected| expected == record.handle)
                            })
                            .cloned();
                        let controller_motion = capture_player_controls.then(|| {
                            crate::controller::read_motion_evidence(
                                process,
                                record.as_ref().map(|record| record.handle),
                            )
                        });
                        let runtime_evidence = record
                            .as_ref()
                            .map(|record| crate::entity::read_runtime_evidence(process, record));
                        let (render_evidence, render_evidence_error) = if capture_render_evidence {
                            match crate::render::read_snapshot(process) {
                                Ok(snapshot) => (Some(snapshot), None),
                                Err(error) => (None, Some(error)),
                            }
                        } else {
                            (None, None)
                        };
                        let camera_eye_surface_evidence = render_evidence.as_ref().map(|render| {
                            let surface = crate::physics::read_world_surface_probe(
                                process,
                                render.camera.output_eye_raw,
                            );
                            pair_camera_eye_surface_evidence(render, surface)
                        });
                        let nearby_entities = record
                            .as_ref()
                            .map(|focus| crate::entity::nearby_entities(focus, &snapshot.records))
                            .unwrap_or_default();
                        let (physics, physics_error) =
                            match record.as_ref().filter(|record| record.entity_type == 46) {
                                Some(player) => {
                                    match crate::physics::read(
                                        process,
                                        player,
                                        &snapshot.records,
                                        capture_vtol_evidence,
                                    ) {
                                        Ok(context) => (Some(context), None),
                                        Err(error) => (None, Some(error)),
                                    }
                                }
                                None => (None, None),
                            };
                        let previous_outer_entity_snapshot_body_basis_raw = capture_vtol_evidence
                            .then(|| {
                                previous
                                    .as_ref()
                                    .zip(record.as_ref())
                                    .filter(|(before, after)| before.pointer == after.pointer)
                                    .map(|(before, _)| before.body_basis_raw)
                            })
                            .flatten();
                        let current_vtol_runtime_sample = physics.as_ref().and_then(|context| {
                            let runtime = context.vtol_runtime.as_ref()?;
                            Some(InitialVtolRuntimeSample {
                                pointer: runtime.runtime_sub_g_pointer?,
                                random_sample_at_0x38: runtime
                                    .runtime_values
                                    .random_sample_at_0x38?,
                                tick_before: runtime.tick_before?,
                                tick_after: runtime.tick_after?,
                            })
                        });
                        if capture_vtol_evidence {
                            match current_vtol_runtime_sample {
                                Some(current) => {
                                    if initial_vtol_runtime_sample
                                        .is_some_and(|initial| initial.pointer != current.pointer)
                                    {
                                        initial_vtol_runtime_sample = None;
                                    }
                                    if initial_vtol_runtime_sample.is_none()
                                        && current.tick_before == current.tick_after
                                    {
                                        initial_vtol_runtime_sample = Some(current);
                                    }
                                }
                                None => initial_vtol_runtime_sample = None,
                            }
                        }
                        let vtol_sample_context = capture_vtol_evidence.then(|| json!({
                        "previous_outer_entity_snapshot_body_basis_raw": previous_outer_entity_snapshot_body_basis_raw,
                        "initial_runtime_sub_g_pointer": initial_vtol_runtime_sample.map(|value| value.pointer),
                        "initial_captured_random_sample_at_runtime_sub_g_0x38": initial_vtol_runtime_sample.map(|value| value.random_sample_at_0x38),
                        "initial_sample_tick_before": initial_vtol_runtime_sample.map(|value| value.tick_before),
                        "initial_sample_tick_after": initial_vtol_runtime_sample.map(|value| value.tick_after),
                        "initial_sample_tick_stable": initial_vtol_runtime_sample.map(|value| value.tick_before == value.tick_after),
                        "current_runtime_sub_g_pointer": current_vtol_runtime_sample.map(|value| value.pointer),
                        "current_random_sample_at_runtime_sub_g_0x38": current_vtol_runtime_sample.map(|value| value.random_sample_at_0x38),
                    }));
                        let changed_offsets = match (&previous, &record) {
                            (Some(before), Some(after)) if before.pointer == after.pointer => {
                                crate::entity::changed_offsets(&before.raw_hex, &after.raw_hex)
                            }
                            _ => Vec::new(),
                        };
                        write_json_line(
                            &mut writer,
                            &json!({
                                "record_kind": "entity_sample",
                                "sample": sample,
                                "elapsed_ms": elapsed_ms,
                                "tick_before": snapshot.tick_before,
                                "tick_after": snapshot.tick_after,
                                "tick_stable": snapshot.tick_stable,
                                "topology_stable": snapshot.topology_stable,
                                "v2000_foreground": v2000_foreground,
                                "foreground_input_state": foreground_input,
                                "foreground_input_edges": &foreground_input_edges,
                                "controller_motion": controller_motion,
                                "changed_offsets": changed_offsets,
                                "entity": record,
                                "nearby_entities": nearby_entities,
                                "runtime_evidence": runtime_evidence,
                                "render_evidence": render_evidence,
                                "render_evidence_error": render_evidence_error,
                                "camera_eye_surface_evidence": camera_eye_surface_evidence,
                                "player_physics": physics,
                                "player_physics_error": physics_error,
                                "vtol_sample_context": vtol_sample_context,
                                "warnings": snapshot.warnings,
                            }),
                        )?;
                        previous = record;
                    }
                    Err(error) => {
                        let controller_motion = capture_player_controls
                            .then(|| crate::controller::read_motion_evidence(process, None));
                        write_json_line(
                            &mut writer,
                            &json!({
                                "record_kind": "entity_sample_error",
                                "sample": sample,
                                "elapsed_ms": elapsed_ms,
                                "v2000_foreground": v2000_foreground,
                                "foreground_input_state": foreground_input,
                                "foreground_input_edges": &foreground_input_edges,
                                "controller_motion": controller_motion,
                                "error": error,
                            }),
                        )?;
                        previous = None;
                    }
                }
            }
            if capture_particle_events {
                particles.sample(process, &mut writer, sample, elapsed_ms)?;
            }
            if capture_audio_events {
                audio.sample(process, &mut writer, sample, elapsed_ms)?;
            }
            if !foreground_input_edges.is_empty() || sample % u64::from(hz) == 0 {
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
        eprintln!("wrote entity timeline to {}", output.display());
        Ok(())
    }

    fn capture_entity_events(
        process: &Process,
        seconds: f64,
        hz: u32,
        output: &Path,
    ) -> Result<(), String> {
        validate_timeline(seconds, hz, 500)?;
        let mut writer = create_writer(output)?;
        write_json_line(
            &mut writer,
            &json!({
                "meta": capture_meta(process, "entity-events"),
                "seconds": seconds,
                "hz": hz,
                "movement_ignored": true,
            }),
        )?;
        let mut previous: HashMap<u32, EntitySignature> = HashMap::new();
        run_timeline(seconds, hz, |sample, elapsed_ms| {
            let snapshot = match crate::entity::read_snapshot(process) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    write_json_line(
                        &mut writer,
                        &json!({
                            "record_kind": "entity_snapshot_error",
                            "sample": sample,
                            "elapsed_ms": elapsed_ms,
                            "error": error,
                        }),
                    )?;
                    return Ok(());
                }
            };
            if !snapshot.topology_stable {
                write_json_line(
                    &mut writer,
                    &json!({
                        "record_kind": "entity_snapshot_unstable",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "tick_before": snapshot.tick_before,
                        "tick_after": snapshot.tick_after,
                        "warnings": snapshot.warnings,
                    }),
                )?;
                return Ok(());
            }

            let current: HashMap<u32, EntitySignature> = snapshot
                .records
                .iter()
                .map(|record| (record.pointer, record.signature()))
                .collect();
            for record in &snapshot.records {
                match previous.get(&record.pointer) {
                    None => write_entity_event(
                        &mut writer,
                        "spawn",
                        sample,
                        elapsed_ms,
                        snapshot.tick_after,
                        record.pointer,
                        None,
                        Some(record),
                    )?,
                    Some(before) if before != &record.signature() => write_entity_event(
                        &mut writer,
                        "update",
                        sample,
                        elapsed_ms,
                        snapshot.tick_after,
                        record.pointer,
                        Some(before),
                        Some(record),
                    )?,
                    _ => {}
                }
            }
            for (pointer, before) in &previous {
                if !current.contains_key(pointer) {
                    write_entity_event(
                        &mut writer,
                        "despawn",
                        sample,
                        elapsed_ms,
                        snapshot.tick_after,
                        *pointer,
                        Some(before),
                        None,
                    )?;
                }
            }
            previous = current;
            Ok(())
        })?;
        writer.flush().map_err(|error| error.to_string())?;
        eprintln!("wrote entity events to {}", output.display());
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn write_entity_event(
        writer: &mut impl Write,
        event: &str,
        sample: u64,
        elapsed_ms: f64,
        tick: u32,
        pointer: u32,
        previous: Option<&EntitySignature>,
        entity: Option<&EntityRecord>,
    ) -> Result<(), String> {
        write_json_line(
            writer,
            &json!({
                "record_kind": "entity_event",
                "event": event,
                "sample": sample,
                "elapsed_ms": elapsed_ms,
                "tick": tick,
                "pointer": pointer,
                "previous": previous,
                "entity": entity,
            }),
        )
    }

    fn capture_menu_timeline(
        process: &Process,
        seconds: f64,
        hz: u32,
        output: &Path,
    ) -> Result<(), String> {
        validate_timeline(seconds, hz, 500)?;
        let mut writer = create_writer(output)?;
        write_json_line(
            &mut writer,
            &json!({
                "meta": capture_meta(process, "menu-timeline"),
                "seconds": seconds,
                "hz": hz,
                "process_loss_auto_stop_seconds": 0.5,
                "coverage": [
                    "continuous_menu_stack_selection_settings_and_animation_clocks",
                    "menu_item_and_background_model_resources",
                    "active_sound_start_update_stop_lifecycle",
                    "retail_process_readability_boundary"
                ],
            }),
        )?;

        let mut seen_models: HashMap<u16, (u32, String)> = HashMap::new();
        let mut failed_models: HashMap<(u16, u32), (u64, String)> = HashMap::new();
        let mut previous_sounds: HashMap<u32, SoundSignature> = HashMap::new();
        let mut previous_audio_warnings: Vec<String> = Vec::new();
        let mut previous_audio_error: Option<String> = None;
        let mut previous_process_readable: Option<bool> = None;
        let mut unreadable_samples = 0u64;
        let process_loss_samples = u64::from(hz.div_ceil(2).max(1));
        let total_samples = (seconds * hz as f64).ceil().max(1.0) as u64;
        let interval = Duration::from_secs_f64(1.0 / hz as f64);
        let start = Instant::now();
        let mut last_flush = start;
        let mut final_sample = 0u64;
        let mut stop_reason = "safety_timeout";

        for sample in 0..total_samples {
            final_sample = sample;
            let target = start + interval.mul_f64(sample as f64);
            let now = Instant::now();
            if target > now {
                sleep(target - now);
            }
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

            match crate::menu::read_snapshot(process) {
                Ok(snapshot) => {
                    for item in &snapshot.model_items {
                        if let Ok(model_id) = u16::try_from(item.resource_id) {
                            observe_model_resource(
                                process,
                                &mut writer,
                                &mut seen_models,
                                &mut failed_models,
                                sample,
                                elapsed_ms,
                                hz,
                                model_id,
                                item.model_resource,
                                json!({
                                    "source": "menu_item",
                                    "screen": snapshot.top_screen,
                                    "screen_name": snapshot.top_screen_name,
                                    "item_address": item.address,
                                }),
                            )?;
                        }
                    }
                    if let Some(model_id) = snapshot.background_model_id {
                        observe_model_resource(
                            process,
                            &mut writer,
                            &mut seen_models,
                            &mut failed_models,
                            sample,
                            elapsed_ms,
                            hz,
                            model_id,
                            snapshot.background_model_resource,
                            json!({
                                "source": "menu_background",
                                "screen": snapshot.top_screen,
                                "screen_name": snapshot.top_screen_name,
                            }),
                        )?;
                    }
                    write_json_line(
                        &mut writer,
                        &json!({
                            "record_kind": "menu_sample",
                            "sample": sample,
                            "elapsed_ms": elapsed_ms,
                            "snapshot": snapshot,
                        }),
                    )?;
                }
                Err(error) => write_json_line(
                    &mut writer,
                    &json!({
                        "record_kind": "menu_sample_error",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "error": error,
                    }),
                )?,
            }

            match crate::audio::read_snapshot(process) {
                Ok(mut snapshot) => {
                    previous_audio_error = None;
                    if !snapshot.topology_stable {
                        if let Ok(retry) = crate::audio::read_snapshot(process) {
                            snapshot = retry;
                        }
                    }
                    if snapshot.warnings != previous_audio_warnings {
                        if !snapshot.warnings.is_empty() {
                            write_json_line(
                                &mut writer,
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
                        previous_audio_warnings = snapshot.warnings.clone();
                    }
                    if snapshot.topology_stable {
                        let current: HashMap<u32, SoundSignature> = snapshot
                            .records
                            .iter()
                            .map(|record| (record.pointer, record.signature()))
                            .collect();
                        for record in &snapshot.records {
                            match previous_sounds.get(&record.pointer) {
                                None => write_sound_event(
                                    &mut writer,
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
                                    &mut writer,
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
                        for (pointer, before) in &previous_sounds {
                            if !current.contains_key(pointer) {
                                write_sound_event(
                                    &mut writer,
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
                        previous_sounds = current;
                    } else {
                        write_json_line(
                            &mut writer,
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
                    }
                }
                Err(error) => {
                    if previous_audio_error.as_ref() != Some(&error) {
                        write_json_line(
                            &mut writer,
                            &json!({
                                "record_kind": "audio_snapshot_error",
                                "sample": sample,
                                "elapsed_ms": elapsed_ms,
                                "error": error,
                            }),
                        )?;
                    }
                    previous_audio_error = Some(error);
                }
            }

            let (process_readable, process_read_error) =
                match process.read_bytes(crate::process::RETAIL_IMAGE_BASE, 2) {
                    Ok(bytes) if bytes == b"MZ" => (true, None),
                    Ok(bytes) => (
                        false,
                        Some(format!("retail image signature changed to {bytes:02X?}")),
                    ),
                    Err(error) => (false, Some(error)),
                };
            if previous_process_readable != Some(process_readable) {
                write_json_line(
                    &mut writer,
                    &json!({
                        "record_kind": "process_readability_boundary",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "readable": process_readable,
                        "error": process_read_error,
                    }),
                )?;
                previous_process_readable = Some(process_readable);
            }
            if process_readable {
                unreadable_samples = 0;
            } else {
                unreadable_samples += 1;
                if unreadable_samples >= process_loss_samples {
                    stop_reason = "process_unreadable";
                }
            }

            if last_flush.elapsed() >= Duration::from_secs(1) {
                writer.flush().map_err(|error| error.to_string())?;
                last_flush = Instant::now();
            }
            if stop_reason != "safety_timeout" {
                break;
            }
        }

        write_json_line(
            &mut writer,
            &json!({
                "record_kind": "menu_capture_end",
                "sample": final_sample,
                "elapsed_ms": start.elapsed().as_secs_f64() * 1000.0,
                "reason": stop_reason,
            }),
        )?;
        writer.flush().map_err(|error| error.to_string())?;
        eprintln!(
            "wrote menu timeline to {} ({stop_reason})",
            output.display()
        );
        Ok(())
    }

    fn capture_render_timeline(
        process: &Process,
        seconds: f64,
        hz: u32,
        output: &Path,
    ) -> Result<(), String> {
        validate_timeline(seconds, hz, 500)?;
        let mut writer = create_writer(output)?;
        write_json_line(
            &mut writer,
            &json!({
                "meta": capture_meta(process, "render-timeline"),
                "seconds": seconds,
                "hz": hz,
                "read_only": true,
            }),
        )?;
        run_timeline(
            seconds,
            hz,
            |sample, elapsed_ms| match crate::render::read_snapshot(process) {
                Ok(snapshot) => write_json_line(
                    &mut writer,
                    &json!({
                        "record_kind": "render_sample",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "snapshot": snapshot,
                    }),
                ),
                Err(error) => write_json_line(
                    &mut writer,
                    &json!({
                        "record_kind": "render_sample_error",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "error": error,
                    }),
                ),
            },
        )?;
        writer.flush().map_err(|error| error.to_string())?;
        eprintln!("wrote render timeline to {}", output.display());
        Ok(())
    }

    fn capture_audio_events(
        process: &Process,
        seconds: f64,
        hz: u32,
        output: &Path,
    ) -> Result<(), String> {
        validate_timeline(seconds, hz, 1000)?;
        let mut writer = create_writer(output)?;
        write_json_line(
            &mut writer,
            &json!({
                "meta": capture_meta(process, "audio-events"),
                "seconds": seconds,
                "hz": hz,
                "passive_alias_warning": "resolved aliases with identical PCM cannot always be distinguished",
            }),
        )?;
        let mut state = AudioTimelineState::default();
        run_timeline(seconds, hz, |sample, elapsed_ms| {
            state.sample(process, &mut writer, sample, elapsed_ms)
        })?;
        writer.flush().map_err(|error| error.to_string())?;
        eprintln!("wrote audio events to {}", output.display());
        Ok(())
    }

    fn capture_fan_audio_timeline(
        process: &Process,
        seconds: f64,
        hz: u32,
        stop_file: Option<&Path>,
        output: &Path,
    ) -> Result<(), String> {
        validate_timeline(seconds, hz, 1000)?;
        ensure_distinct_capture_paths(stop_file, output)?;
        ensure_stop_file_absent(stop_file)?;
        let target_resources = crate::fan_audio::read_target_resources(process)?;
        let mut writer = create_writer(output)?;
        write_json_line(
            &mut writer,
            &json!({
                "meta": capture_meta(process, "fan-audio-timeline"),
                "seconds": seconds,
                "hz": hz,
                "stop_file": stop_file.map(|path| path.display().to_string()),
                "stop_file_policy": stop_file.map(|_| json!({
                    "ownership": "external; the inspector never creates, removes, or modifies the marker",
                    "startup": "refuse capture if the marker already exists",
                    "polling": "check after each sample deadline and stop before taking the next sample when the marker exists",
                    "read_error": "abort capture rather than silently ignore an unreadable marker path",
                })),
                "foreground_key_controls": [
                    "space", "right_shift", "up", "down", "left", "right", "s", "x", "tab"
                ],
                "target_controls": ["space", "right_shift"],
                "foreground_protocol_controls": ["caps_lock"],
                "foreground_key_source": "Win32 GetAsyncKeyState high bit while the attached V2000 process owns the foreground window; correlation evidence, not the game's DirectInput buffer",
                "input_edge_policy": "first focused sample is a baseline; losing focus clears the baseline; returning cannot synthesize release edges",
                "sample_order": "foreground input, complete tick-bracketed controller/fan/DirectSound memory sample (retried once as a unit when unstable), serialized record",
                "controller_throttle": "signed dword at controller +0x288, retained so simultaneous Space/Right Shift cancellation can be verified independently of Win32 key state",
                "fan_state_chain": "controller+0x68 handle -> read-only resource-cache resolution -> entity+0x4C -> root+0x0C -> component table+0x38 -> 0x4C-byte Sub-O state",
                "fan_state_fields": "upper logical voice handle +0x1C, shared visual/audio RPM state +0x20, gain envelope +0x24; lower logical voice handle is entity+0x8C",
                "audio_sample_policy": "every scheduled sample retains every active voice, including unchanged voices; targets are matched by the live catalog's PCM pointer plus byte length, not length alone",
                "target_resources": &target_resources,
                "recorded_voice_fields": [
                    "volume_16_16", "frequency_16_16", "frequency_hz", "pan",
                    "resolved_section11_type", "pcm_pointer", "pcm_bytes",
                    "directsound_play_flags", "auto_gc"
                ],
                "read_only": true,
                "callback_policy": "no retail callback is invoked and process memory is never written",
            }),
        )?;
        // The guided wrapper binds its input cues to the exact process id in
        // this first line. Do not leave it waiting on the BufWriter capacity.
        writer.flush().map_err(|error| error.to_string())?;

        let mut previous_input: Option<crate::foreground_input::HoverInputState> = None;
        let mut previous_foreground = None;
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
                        "record_kind": "fan_audio_input_scope",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "v2000_foreground": foreground,
                    }),
                )?;
                previous_foreground = Some(foreground);
            }

            let memory_sample = crate::fan_audio::read_memory_sample(process, &target_resources);
            write_json_line(
                &mut writer,
                &json!({
                    "record_kind": "fan_audio_sample",
                    "sample": sample,
                    "elapsed_ms": elapsed_ms,
                    "v2000_foreground": foreground,
                    "foreground_input_state": input,
                    "foreground_input_edges": input_edges,
                    "memory_sample": memory_sample,
                }),
            )?;
            if !input_edges.is_empty() || sample % u64::from(hz) == 0 {
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
        eprintln!("wrote fan audio timeline to {}", output.display());
        Ok(())
    }

    fn capture_particle_events(
        process: &Process,
        seconds: f64,
        hz: u32,
        output: &Path,
    ) -> Result<(), String> {
        validate_timeline(seconds, hz, 1000)?;
        let mut writer = create_writer(output)?;
        write_json_line(
            &mut writer,
            &json!({
                "meta": capture_meta(process, "particle-events"),
                "seconds": seconds,
                "hz": hz,
                "read_only": true,
                "pool": {
                    "base": format!("0x{:08X}", crate::particle::PARTICLE_POOL),
                    "record_bytes": crate::particle::PARTICLE_RECORD_BYTES,
                    "capacity": crate::particle::PARTICLE_CAPACITY,
                    "priority_list_base": format!("0x{:08X}", crate::particle::PRIORITY_LISTS),
                    "priority_list_count": crate::particle::PRIORITY_LIST_COUNT,
                    "descriptor_table": format!("0x{:08X}", crate::particle::PARTICLE_DESCRIPTOR_TABLE),
                    "descriptor_bytes": crate::particle::PARTICLE_DESCRIPTOR_BYTES,
                    "descriptor_count": crate::particle::PARTICLE_DESCRIPTOR_COUNT,
                },
                "record_layout": {
                    "next": "u32@00",
                    "previous_link": "u32@04",
                    "position_raw_8_8": ["i16@08", "i16@0A", "i16@0C"],
                    "velocity_raw": ["i16@0E", "i16@10", "i16@12"],
                    "source_entity_handle": "u32@14",
                    "surface_height_raw_8_8": "i16@18",
                    "class_live_marker": "u8@1A; zero is free",
                    "age": "u8@1B",
                    "source_entity_type": "u8@1C",
                    "state_flags": "u8@1D; retained raw",
                    "unknown_padding": "u16@1E",
                },
                "stability_policy": "observational, not atomic: accept only unchanged duplicate reads of the full 0x1900-byte pool with unchanged 7-list roots, an unchanged 50-Hz clock, and complete intrusive-list/descriptor-priority validation; retail exposes no proven post-update epoch",
                "event_policy": "only observationally stable endpoints are compared: birth/death uses class zero transitions; nonzero class change or age decrease is slot recycling; a source change without generation reset is reported separately; events spanning rejected samples carry the exact unstable-sample count",
                "passive_sampling_limit": "an allocate-and-free cycle wholly between accepted samples, or indistinguishable same-class same-age slot reuse, cannot be observed without debugger instrumentation",
            }),
        )?;

        let mut state = ParticleTimelineState::new(false);
        run_timeline(seconds, hz, |sample, elapsed_ms| {
            state.sample(process, &mut writer, sample, elapsed_ms)
        })?;
        writer.flush().map_err(|error| error.to_string())?;
        eprintln!("wrote particle events to {}", output.display());
        Ok(())
    }

    fn capture_fire_events(
        process: &Process,
        seconds: f64,
        hz: u32,
        context_hz: u32,
        terrain_hz: u32,
        capture_terrain_infection: bool,
        stop_file: Option<&Path>,
        output: &Path,
    ) -> Result<(), String> {
        validate_timeline(seconds, hz, 1000)?;
        if context_hz == 0 || context_hz > hz {
            return Err(format!(
                "--context-hz must be between 1 and the primary --hz ({hz})"
            ));
        }
        if capture_terrain_infection && (terrain_hz == 0 || terrain_hz > hz) {
            return Err(format!(
                "--terrain-hz must be between 1 and the primary --hz ({hz})"
            ));
        }
        ensure_distinct_capture_paths(stop_file, output)?;
        ensure_stop_file_absent(stop_file)?;
        let context_stride = u64::from(hz.div_ceil(context_hz).max(1));
        let effective_context_hz = hz as f64 / context_stride as f64;
        let master_weapon_catalog = crate::fire_state::read_master_weapon_catalog(process)?;
        let sound_resource_catalog = read_loaded_sound_resource_catalog(process);
        let mut writer = create_writer(output)?;
        write_json_line(
            &mut writer,
            &json!({
                "meta": capture_meta(process, "fire-events"),
                "seconds": seconds,
                "hz": hz,
                "context_hz_requested": context_hz,
                "context_sample_stride": context_stride,
                "context_hz_effective": effective_context_hz,
                "terrain_hz": capture_terrain_infection.then_some(terrain_hz),
                "capture_terrain_infection": capture_terrain_infection,
                "entity_event_policy": "stable context samples emit changes in identity/models/behavior/power-up payload and collision health/last-damage tick/damage buffer; motion alone is omitted",
                "stop_file": stop_file.map(|path| path.display().to_string()),
                "stop_file_policy": stop_file.map(|_| json!({
                    "ownership": "external; the inspector never creates, removes, or modifies the marker",
                    "startup": "refuse capture if the marker already exists",
                    "polling": "check after each sample deadline and stop before taking the next sample when the marker exists",
                    "read_error": "abort capture rather than silently ignore an unreadable marker path",
                })),
                "controls": [
                    "enter", "right_mouse", "left_control", "right_control",
                    "key_e", "key_b", "key_v", "left_shift",
                    "capture_stop_f12"
                ],
                "retail_selection_bindings": {
                    "key_e": "rotate separate 12-byte target/selection list through FUN_00418980",
                    "key_b": "next weapon (same callback as PageDown, Left Shift, and L)",
                    "key_v": "previous weapon (same callback as PageUp, Caps Lock, and K)",
                    "left_shift": "next weapon; sampled independently because it also suppresses C/D beam bindings",
                },
                "foreground_process_only": true,
                "continuous_transient_shot_list": true,
                "continuous_controller_weapon_state": true,
                "continuous_controller_pickup_state": true,
                "power_up_payload": "type 61 entity +0x88: selector=low byte; amount=arithmetic signed shift right 8",
                "targetter_acquisition": "selector 0x3C sets controller +0x197 bit 0 and initializes the exact +0x22C..+0x27F block",
                "weapon_acquisition": "selectors below 0x32 copy/update a canonical 0x18-byte descriptor; controller +0x5A8 and runtime weapon +0x14 remain independent observations",
                "shot_record_bytes": 44,
                "continuous_entity_lifecycle": true,
                "continuous_audio_lifecycle": true,
                "context_particle_lifecycle": true,
                "complete_snapshots_on_input_edges": true,
                "input_sample_order": if capture_terrain_infection {
                    "foreground input edge, fire-state snapshot, duplicate-read terrain grid, entity snapshot, audio snapshot, particle pool"
                } else {
                    "foreground input edge, fire-state snapshot, entity snapshot, audio snapshot, particle pool"
                },
                "read_only": true,
                "callback_policy": "no retail callback is invoked and process memory is never written",
                "terrain_policy": capture_terrain_infection.then_some(
                    "duplicate full Section-10 reads retain exact bounded three-byte cell diffs, infection-bit transitions, retail counters, and resource-boundary resets"
                ),
            }),
        )?;
        write_json_line(
            &mut writer,
            &json!({
                "record_kind": "master_weapon_catalog",
                "sample": 0,
                "elapsed_ms": 0.0,
                "catalog": master_weapon_catalog,
            }),
        )?;
        match sound_resource_catalog {
            Ok(catalog) => write_json_line(
                &mut writer,
                &json!({
                    "record_kind": "sound_resource_catalog",
                    "sample": 0,
                    "elapsed_ms": 0.0,
                    "current": catalog,
                }),
            )?,
            Err(error) => write_json_line(
                &mut writer,
                &json!({
                    "record_kind": "sound_resource_catalog_error",
                    "sample": 0,
                    "elapsed_ms": 0.0,
                    "error": error,
                }),
            )?,
        }
        // The F12 wrapper reads the first line to bind itself to the exact
        // retail PID selected by this inspector. Publish it before sampling.
        writer.flush().map_err(|error| error.to_string())?;

        let mut previous_input = None;
        let mut previous_foreground = None;
        let mut previous_fire_state = None;
        let mut previous_fire_error: Option<String> = None;
        let mut previous_entities: HashMap<u32, EntitySignature> = HashMap::new();
        let mut previous_sounds: HashMap<u32, SoundSignature> = HashMap::new();
        let mut particles = ParticleTimelineState::new(false);
        let mut terrain = crate::terrain_infection::TerrainInfectionState::new();
        let mut previous_terrain_error: Option<String> = None;
        let outcome = run_stop_aware(seconds, hz, stop_file, |sample, elapsed_ms| {
            // `poll_foreground` checks ownership before querying either key.
            // Dropping the baseline while unfocused prevents synthetic release
            // edges when the user Alt-Tabs away and back.
            let input = crate::foreground_input::poll_fire_foreground(process.process_id);
            let foreground = input.is_some();
            if previous_foreground != Some(foreground) {
                write_json_line(
                    &mut writer,
                    &json!({
                        "record_kind": "fire_input_scope",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "v2000_foreground": foreground,
                    }),
                )?;
                previous_foreground = Some(foreground);
            }
            let input_edges = match (previous_input, input) {
                (Some(before), Some(current)) => current.edges_from(before),
                _ => Vec::new(),
            };
            previous_input = input;

            // The retail primary lives on a short-lived intrusive list rooted
            // in the player record, not in the global entity list. Sample that
            // path at the command's full rate before the heavier context reads.
            let (fire_state_snapshot, fire_state_error) =
                match crate::fire_state::read_snapshot(process) {
                    Ok(snapshot) => {
                        let signature = snapshot.signature();
                        let stable = snapshot.tick_stable
                            && snapshot.entity_topology_stable
                            && snapshot.shots.topology_stable
                            && snapshot.shots.traversal_complete;
                        let event = if !stable {
                            Some("unstable_observation")
                        } else if previous_fire_state.is_none() {
                            Some("baseline")
                        } else if previous_fire_state.as_ref() != Some(&signature) {
                            Some("change")
                        } else if !snapshot.shots.records.is_empty() {
                            Some("shot_observation")
                        } else {
                            None
                        };
                        if let Some(event) = event {
                            write_json_line(
                                &mut writer,
                                &json!({
                                    "record_kind": "fire_state_event",
                                    "event": event,
                                    "sample": sample,
                                    "elapsed_ms": elapsed_ms,
                                    "previous": previous_fire_state.as_ref(),
                                    "snapshot": &snapshot,
                                }),
                            )?;
                        }
                        if stable {
                            previous_fire_state = Some(signature);
                        }
                        previous_fire_error = None;
                        (Some(snapshot), None)
                    }
                    Err(error) => {
                        if previous_fire_error.as_deref() != Some(error.as_str()) {
                            write_json_line(
                                &mut writer,
                                &json!({
                                    "record_kind": "fire_state_error",
                                    "sample": sample,
                                    "elapsed_ms": elapsed_ms,
                                    "error": &error,
                                }),
                            )?;
                        }
                        previous_fire_error = Some(error.clone());
                        (None, Some(error))
                    }
                };

            if capture_terrain_infection && cadence_due(sample, hz, terrain_hz) {
                match terrain.sample(process, &mut writer, sample, elapsed_ms) {
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

            // Full entity/model resolution and DirectSound traversal are much
            // more expensive and do not need 1-ms sampling. Always take them
            // on an input edge; otherwise sample at the requested context rate.
            let context_due = sample % context_stride == 0 || !input_edges.is_empty();
            if !context_due {
                if sample % u64::from(hz) == 0 {
                    writer.flush().map_err(|error| error.to_string())?;
                }
                return Ok(());
            }

            let (entity_snapshot, entity_error) = match crate::entity::read_snapshot(process) {
                Ok(snapshot) => (Some(snapshot), None),
                Err(error) => {
                    write_json_line(
                        &mut writer,
                        &json!({
                            "record_kind": "entity_snapshot_error",
                            "sample": sample,
                            "elapsed_ms": elapsed_ms,
                            "error": &error,
                        }),
                    )?;
                    (None, Some(error))
                }
            };
            if let Some(snapshot) = entity_snapshot.as_ref() {
                if snapshot.topology_stable {
                    let current: HashMap<u32, EntitySignature> = snapshot
                        .records
                        .iter()
                        .map(|record| (record.pointer, record.signature()))
                        .collect();
                    for record in &snapshot.records {
                        match previous_entities.get(&record.pointer) {
                            None => write_entity_event(
                                &mut writer,
                                "spawn",
                                sample,
                                elapsed_ms,
                                snapshot.tick_after,
                                record.pointer,
                                None,
                                Some(record),
                            )?,
                            Some(before) if before != &record.signature() => write_entity_event(
                                &mut writer,
                                "update",
                                sample,
                                elapsed_ms,
                                snapshot.tick_after,
                                record.pointer,
                                Some(before),
                                Some(record),
                            )?,
                            _ => {}
                        }
                    }
                    for (pointer, before) in &previous_entities {
                        if !current.contains_key(pointer) {
                            write_entity_event(
                                &mut writer,
                                "despawn",
                                sample,
                                elapsed_ms,
                                snapshot.tick_after,
                                *pointer,
                                Some(before),
                                None,
                            )?;
                        }
                    }
                    previous_entities = current;
                } else {
                    write_json_line(
                        &mut writer,
                        &json!({
                            "record_kind": "entity_snapshot_unstable",
                            "sample": sample,
                            "elapsed_ms": elapsed_ms,
                            "tick_before": snapshot.tick_before,
                            "tick_after": snapshot.tick_after,
                            "warnings": &snapshot.warnings,
                        }),
                    )?;
                }
            }

            let (audio_snapshot, audio_error) = match crate::audio::read_snapshot(process) {
                Ok(mut snapshot) => {
                    if !snapshot.topology_stable {
                        if let Ok(retry) = crate::audio::read_snapshot(process) {
                            snapshot = retry;
                        }
                    }
                    (Some(snapshot), None)
                }
                Err(error) => {
                    write_json_line(
                        &mut writer,
                        &json!({
                            "record_kind": "audio_snapshot_error",
                            "sample": sample,
                            "elapsed_ms": elapsed_ms,
                            "error": &error,
                        }),
                    )?;
                    (None, Some(error))
                }
            };
            if let Some(snapshot) = audio_snapshot.as_ref() {
                if snapshot.topology_stable {
                    let current: HashMap<u32, SoundSignature> = snapshot
                        .records
                        .iter()
                        .map(|record| (record.pointer, record.signature()))
                        .collect();
                    for record in &snapshot.records {
                        match previous_sounds.get(&record.pointer) {
                            None => write_sound_event(
                                &mut writer,
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
                                &mut writer,
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
                    for (pointer, before) in &previous_sounds {
                        if !current.contains_key(pointer) {
                            write_sound_event(
                                &mut writer,
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
                    previous_sounds = current;
                } else {
                    write_json_line(
                        &mut writer,
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
                }
            }

            // Some authored weapons materialize through the fixed particle
            // pool rather than as ordinary global-list entities. Birth/death
            // and slot-recycle events at the context cadence retain those
            // effects without serializing every unchanged active particle.
            particles.sample(process, &mut writer, sample, elapsed_ms)?;

            if !input_edges.is_empty() {
                for edge in &input_edges {
                    write_json_line(
                        &mut writer,
                        &json!({
                            "record_kind": "fire_input_event",
                            "sample": sample,
                            "elapsed_ms": elapsed_ms,
                            "edge": edge,
                            "input_state": input,
                            "fire_state_snapshot": fire_state_snapshot.as_ref(),
                            "fire_state_error": fire_state_error.as_deref(),
                            "entity_snapshot": entity_snapshot.as_ref(),
                            "entity_snapshot_error": entity_error.as_deref(),
                            "audio_snapshot": audio_snapshot.as_ref(),
                            "audio_snapshot_error": audio_error.as_deref(),
                        }),
                    )?;
                }
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
        eprintln!("wrote fire events to {}", output.display());
        Ok(())
    }

    fn capture_session_timeline(
        process: &Process,
        seconds: f64,
        hz: u32,
        output: &Path,
    ) -> Result<(), String> {
        validate_timeline(seconds, hz, 200)?;
        let mut writer = create_writer(output)?;
        write_json_line(
            &mut writer,
            &json!({
                "record_kind": "capture_meta",
                "meta": capture_meta(process, "session-timeline"),
                "seconds": seconds,
                "hz": hz,
                "level_1_auto_stop_seconds": 5.0,
                "post_intro_klaus_note": "Level-1 auto-stop requires five stable anchor seconds, leaving a conservative margin beyond the proven one-second post-load Klaus leg.",
                "coverage": [
                    "resource_pool_changes",
                    "loaded_overlay_catalog_and_resource_provenance",
                    "loaded_sound_resources_aliases_and_pcm_hashes",
                    "startup_frontend_intro2_level_1_phase_changes",
                    "menu_modes_and_model_items",
                    "entity_lifecycle_and_model_slots",
                    "continuous_klaus_type_0_model_1_pose_and_runtime_evidence",
                    "entity_type_default_models",
                    "resolved_model_names_lengths_and_hashes",
                    "active_sound_lifecycle"
                ],
                "startup_avi_note": "AVI playback is not represented by Section-8 model resources; its mode transitions are still recorded",
            }),
        )?;

        let mut previous_resources: Option<crate::resources::ResourcePools> = None;
        let mut previous_catalog: Option<crate::resources::ResourceCatalog> = None;
        let mut previous_sound_resources: Option<crate::sound_resource::SoundResourceCatalog> =
            None;
        let mut sound_resource_catalog_dirty = true;
        let mut previous_menu: Option<crate::menu::MenuEventState> = None;
        let mut previous_entities: HashMap<u32, EntitySignature> = HashMap::new();
        let mut previous_sounds: HashMap<u32, SoundSignature> = HashMap::new();
        let mut previous_entity_warnings: Vec<String> = Vec::new();
        let mut previous_audio_warnings: Vec<String> = Vec::new();
        let mut seen_models: HashMap<u16, (u32, String)> = HashMap::new();
        let mut failed_models: HashMap<(u16, u32), (u64, String)> = HashMap::new();
        let mut seen_type_defaults: HashMap<u32, (u32, [u16; 4])> = HashMap::new();
        let mut failed_type_defaults: HashMap<u32, (u64, String)> = HashMap::new();
        let mut last_errors: HashMap<&'static str, String> = HashMap::new();
        let mut loaded_levels: Vec<u32> = Vec::new();
        let mut previous_phase: Option<&'static str> = None;
        let mut previous_klaus_pointer: Option<u32> = None;

        let total_samples = (seconds * hz as f64).ceil().max(1.0) as u64;
        let interval = Duration::from_secs_f64(1.0 / hz as f64);
        let start = Instant::now();
        let mut last_flush = start;
        let mut anchor_since: Option<Instant> = None;
        let mut unreadable_samples = 0u64;
        let mut final_sample = 0u64;
        let mut stop_reason = "safety_timeout";

        for sample in 0..total_samples {
            final_sample = sample;
            let target = start + interval.mul_f64(sample as f64);
            let now = Instant::now();
            if target > now {
                sleep(target - now);
            }
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
            let mut readable_domains = 0u32;
            let mut current_game_state = None;
            let mut current_frontend_flag = 0u32;
            let mut current_menu_depth = 0u32;
            let mut entity_count: Option<usize> = None;
            let mut sound_count: Option<usize> = None;
            let mut level_1_anchors = false;

            match crate::resources::read(process) {
                Ok(pools) => {
                    readable_domains += 1;
                    last_errors.remove("resources");
                    let models_changed = previous_resources
                        .as_ref()
                        .is_some_and(|before| before.models != pools.models);
                    let types_changed = previous_resources
                        .as_ref()
                        .is_some_and(|before| before.entity_types != pools.entity_types);
                    let sounds_changed = previous_resources
                        .as_ref()
                        .is_none_or(|before| before.sounds != pools.sounds);
                    if previous_resources.as_ref() != Some(&pools) {
                        write_json_line(
                            &mut writer,
                            &json!({
                                "record_kind": "resource_pools",
                                "sample": sample,
                                "elapsed_ms": elapsed_ms,
                                "previous": previous_resources,
                                "current": pools,
                            }),
                        )?;
                    }
                    if models_changed {
                        seen_models.clear();
                        failed_models.clear();
                    }
                    if types_changed {
                        seen_type_defaults.clear();
                        failed_type_defaults.clear();
                    }
                    sound_resource_catalog_dirty |= sounds_changed;
                    previous_resources = Some(pools);
                }
                Err(error) => write_domain_error_once(
                    &mut writer,
                    &mut last_errors,
                    "resources",
                    sample,
                    elapsed_ms,
                    error,
                )?,
            }

            // The overlay descriptor catalog changes only at load/unload
            // boundaries. Ten samples per second are ample and keep the
            // 53-record scan out of the hot 50 Hz entity path.
            let catalog_interval = u64::from((hz / 10).max(1));
            if sample % catalog_interval == 0 {
                match crate::resources::read_catalog(process) {
                    Ok(catalog) => {
                        last_errors.remove("resource_catalog");
                        loaded_levels = catalog.loaded_level_ids();
                        sound_resource_catalog_dirty |=
                            previous_catalog.as_ref().is_none_or(|before| {
                                sound_resource_sources(before) != sound_resource_sources(&catalog)
                            });
                        if previous_catalog.as_ref() != Some(&catalog) {
                            write_json_line(
                                &mut writer,
                                &json!({
                                    "record_kind": "resource_catalog",
                                    "sample": sample,
                                    "elapsed_ms": elapsed_ms,
                                    "previous": previous_catalog,
                                    "current": catalog,
                                }),
                            )?;
                            previous_catalog = Some(catalog);
                        }
                    }
                    Err(error) => write_domain_error_once(
                        &mut writer,
                        &mut last_errors,
                        "resource_catalog",
                        sample,
                        elapsed_ms,
                        error,
                    )?,
                }

                if sound_resource_catalog_dirty {
                    if let (Some(pools), Some(catalog)) =
                        (previous_resources.as_ref(), previous_catalog.as_ref())
                    {
                        let slot_count = catalog.section_totals[11];
                        match crate::sound_resource::read(process, pools.sounds, slot_count) {
                            Ok(current) => {
                                last_errors.remove("sound_resource_catalog");
                                if previous_sound_resources.as_ref() != Some(&current) {
                                    let previous_loaded_global_ids =
                                        previous_sound_resources.as_ref().map(|before| {
                                            before
                                                .resources
                                                .iter()
                                                .map(|resource| resource.global_id)
                                                .collect::<Vec<_>>()
                                        });
                                    write_json_line(
                                        &mut writer,
                                        &json!({
                                            "record_kind": "sound_resource_catalog",
                                            "sample": sample,
                                            "elapsed_ms": elapsed_ms,
                                            "loaded_levels": loaded_levels,
                                            "previous_loaded_global_ids": previous_loaded_global_ids,
                                            "current": current,
                                        }),
                                    )?;
                                    previous_sound_resources = Some(current);
                                }
                                sound_resource_catalog_dirty = false;
                            }
                            Err(error) => write_domain_error_once(
                                &mut writer,
                                &mut last_errors,
                                "sound_resource_catalog",
                                sample,
                                elapsed_ms,
                                error,
                            )?,
                        }
                    }
                }
            }

            match crate::menu::read_snapshot(process) {
                Ok(snapshot) => {
                    readable_domains += 1;
                    last_errors.remove("menu");
                    current_game_state = Some(snapshot.game_state);
                    current_frontend_flag = snapshot.frontend_flag;
                    current_menu_depth = snapshot.depth;
                    let state = snapshot.event_state();
                    if previous_menu.as_ref() != Some(&state) {
                        write_json_line(
                            &mut writer,
                            &json!({
                                "record_kind": "menu_event",
                                "sample": sample,
                                "elapsed_ms": elapsed_ms,
                                "snapshot": snapshot,
                            }),
                        )?;
                        previous_menu = Some(state);
                    }
                    for item in &snapshot.model_items {
                        if let Ok(model_id) = u16::try_from(item.resource_id) {
                            observe_model_resource(
                                process,
                                &mut writer,
                                &mut seen_models,
                                &mut failed_models,
                                sample,
                                elapsed_ms,
                                hz,
                                model_id,
                                item.model_resource,
                                json!({
                                    "source": "menu_item",
                                    "screen": snapshot.top_screen,
                                    "screen_name": snapshot.top_screen_name,
                                    "item_address": item.address,
                                }),
                            )?;
                        }
                    }
                    if let Some(model_id) = snapshot.background_model_id {
                        observe_model_resource(
                            process,
                            &mut writer,
                            &mut seen_models,
                            &mut failed_models,
                            sample,
                            elapsed_ms,
                            hz,
                            model_id,
                            snapshot.background_model_resource,
                            json!({
                                "source": "menu_background",
                                "screen": snapshot.top_screen,
                                "screen_name": snapshot.top_screen_name,
                            }),
                        )?;
                    }
                }
                Err(error) => write_domain_error_once(
                    &mut writer,
                    &mut last_errors,
                    "menu",
                    sample,
                    elapsed_ms,
                    error,
                )?,
            }

            match crate::entity::read_snapshot(process) {
                Ok(snapshot) => {
                    readable_domains += 1;
                    last_errors.remove("entities");
                    entity_count = Some(snapshot.records.len());
                    if snapshot.warnings != previous_entity_warnings {
                        if !snapshot.warnings.is_empty() {
                            write_json_line(
                                &mut writer,
                                &json!({
                                    "record_kind": "entity_snapshot_warning",
                                    "sample": sample,
                                    "elapsed_ms": elapsed_ms,
                                    "tick_before": snapshot.tick_before,
                                    "tick_after": snapshot.tick_after,
                                    "topology_stable": snapshot.topology_stable,
                                    "warnings": &snapshot.warnings,
                                }),
                            )?;
                        }
                        previous_entity_warnings = snapshot.warnings.clone();
                    }
                    if snapshot.topology_stable {
                        let current: HashMap<u32, EntitySignature> = snapshot
                            .records
                            .iter()
                            .map(|record| (record.pointer, record.signature()))
                            .collect();
                        for record in &snapshot.records {
                            match previous_entities.get(&record.pointer) {
                                None => write_entity_event(
                                    &mut writer,
                                    "spawn",
                                    sample,
                                    elapsed_ms,
                                    snapshot.tick_after,
                                    record.pointer,
                                    None,
                                    Some(record),
                                )?,
                                Some(before) if before != &record.signature() => {
                                    write_entity_event(
                                        &mut writer,
                                        "update",
                                        sample,
                                        elapsed_ms,
                                        snapshot.tick_after,
                                        record.pointer,
                                        Some(before),
                                        Some(record),
                                    )?
                                }
                                _ => {}
                            }
                        }
                        for (pointer, before) in &previous_entities {
                            if !current.contains_key(pointer) {
                                write_entity_event(
                                    &mut writer,
                                    "despawn",
                                    sample,
                                    elapsed_ms,
                                    snapshot.tick_after,
                                    *pointer,
                                    Some(before),
                                    None,
                                )?;
                            }
                        }
                        previous_entities = current;

                        // Type 0 is the proven frontend backdrop entity and its
                        // Section-12 default model is global model 1, `klaus`.
                        // Ordinary entity events intentionally ignore pose-only
                        // changes, so retain this one entity every sample to
                        // cover both New Game handoffs and the jaw/component
                        // state surrounding the Intro2 -> level-1 load.
                        let klaus = snapshot
                            .records
                            .iter()
                            .find(|record| record.entity_type == 0);
                        let klaus_pointer = klaus.map(|record| record.pointer);
                        if previous_klaus_pointer != klaus_pointer {
                            write_json_line(
                                &mut writer,
                                &json!({
                                    "record_kind": "klaus_presence",
                                    "sample": sample,
                                    "elapsed_ms": elapsed_ms,
                                    "previous_pointer": previous_klaus_pointer,
                                    "current_pointer": klaus_pointer,
                                    "identification_basis": "entity_type_0_with_proven_section12_default_global_model_1",
                                    "proven_section12_default_model": 1,
                                    "loaded_levels": &loaded_levels,
                                    "intro2_resident": loaded_levels.contains(&50),
                                    "level_1_resident": loaded_levels.contains(&13),
                                }),
                            )?;
                            previous_klaus_pointer = klaus_pointer;
                        }
                        if let Some(klaus) = klaus {
                            let runtime_evidence =
                                crate::entity::read_runtime_evidence(process, klaus);
                            write_json_line(
                                &mut writer,
                                &json!({
                                    "record_kind": "klaus_sample",
                                    "sample": sample,
                                    "elapsed_ms": elapsed_ms,
                                    "tick": snapshot.tick_after,
                                    "loaded_levels": &loaded_levels,
                                    "intro2_resident": loaded_levels.contains(&50),
                                    "level_1_resident": loaded_levels.contains(&13),
                                    "identification_basis": "entity_type_0_with_proven_section12_default_global_model_1",
                                    "proven_section12_default_model": 1,
                                    "entity": klaus,
                                    "runtime_evidence": runtime_evidence,
                                }),
                            )?;
                        }

                        let mut attempted_types = HashSet::new();
                        for record in &snapshot.records {
                            if attempted_types.insert(record.entity_type) {
                                observe_type_defaults(
                                    process,
                                    &mut writer,
                                    &mut seen_type_defaults,
                                    &mut failed_type_defaults,
                                    &mut seen_models,
                                    &mut failed_models,
                                    sample,
                                    elapsed_ms,
                                    hz,
                                    record.entity_type,
                                    record.pointer,
                                )?;
                            }
                            for (slot, (&model_id, &resource_pointer)) in record
                                .models
                                .iter()
                                .zip(record.model_resources.iter())
                                .enumerate()
                            {
                                observe_model_resource(
                                    process,
                                    &mut writer,
                                    &mut seen_models,
                                    &mut failed_models,
                                    sample,
                                    elapsed_ms,
                                    hz,
                                    model_id,
                                    resource_pointer,
                                    json!({
                                        "source": "entity_slot",
                                        "entity_pointer": record.pointer,
                                        "entity_handle": record.handle,
                                        "entity_type": record.entity_type,
                                        "slot": slot,
                                        "active": slot == record.active_slot,
                                        "parent_handle": record.parent_handle,
                                    }),
                                )?;
                            }
                        }
                        level_1_anchors = has_level_one_anchors(&snapshot.records);
                    } else {
                        write_json_line(
                            &mut writer,
                            &json!({
                                "record_kind": "entity_snapshot_unstable",
                                "sample": sample,
                                "elapsed_ms": elapsed_ms,
                                "tick_before": snapshot.tick_before,
                                "tick_after": snapshot.tick_after,
                                "warnings": snapshot.warnings,
                            }),
                        )?;
                    }
                }
                Err(error) => write_domain_error_once(
                    &mut writer,
                    &mut last_errors,
                    "entities",
                    sample,
                    elapsed_ms,
                    error,
                )?,
            }

            match crate::audio::read_snapshot(process) {
                Ok(mut snapshot) => {
                    readable_domains += 1;
                    last_errors.remove("audio");
                    if !snapshot.topology_stable {
                        if let Ok(retry) = crate::audio::read_snapshot(process) {
                            snapshot = retry;
                        }
                    }
                    sound_count = Some(snapshot.records.len());
                    if snapshot.warnings != previous_audio_warnings {
                        if !snapshot.warnings.is_empty() {
                            write_json_line(
                                &mut writer,
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
                        previous_audio_warnings = snapshot.warnings.clone();
                    }
                    if snapshot.topology_stable {
                        let current: HashMap<u32, SoundSignature> = snapshot
                            .records
                            .iter()
                            .map(|record| (record.pointer, record.signature()))
                            .collect();
                        for record in &snapshot.records {
                            match previous_sounds.get(&record.pointer) {
                                None => write_sound_event(
                                    &mut writer,
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
                                    &mut writer,
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
                        for (pointer, before) in &previous_sounds {
                            if !current.contains_key(pointer) {
                                write_sound_event(
                                    &mut writer,
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
                        previous_sounds = current;
                    } else {
                        write_json_line(
                            &mut writer,
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
                    }
                }
                Err(error) => write_domain_error_once(
                    &mut writer,
                    &mut last_errors,
                    "audio",
                    sample,
                    elapsed_ms,
                    error,
                )?,
            }

            if level_1_anchors {
                let since = anchor_since.get_or_insert_with(Instant::now);
                if since.elapsed() >= Duration::from_secs(5) {
                    stop_reason = "level_1_anchors_stable";
                }
            } else {
                anchor_since = None;
            }

            let phase = derive_session_phase(
                current_game_state,
                current_frontend_flag,
                current_menu_depth,
                &loaded_levels,
                level_1_anchors,
            );
            if previous_phase != Some(phase) {
                write_json_line(
                    &mut writer,
                    &json!({
                        "record_kind": "session_phase",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "phase": phase,
                        "game_state": current_game_state,
                        "frontend_flag": current_frontend_flag,
                        "menu_depth": current_menu_depth,
                        "loaded_levels": loaded_levels,
                        "level_1_anchors": level_1_anchors,
                    }),
                )?;
                previous_phase = Some(phase);
            }

            if readable_domains == 0 {
                unreadable_samples += 1;
                if unreadable_samples >= u64::from(hz) * 2 {
                    stop_reason = "process_unreadable";
                }
            } else {
                unreadable_samples = 0;
            }

            if sample % u64::from(hz) == 0 {
                write_json_line(
                    &mut writer,
                    &json!({
                        "record_kind": "session_heartbeat",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "phase": phase,
                        "game_state": current_game_state,
                        "frontend_flag": current_frontend_flag,
                        "menu_depth": current_menu_depth,
                        "loaded_levels": loaded_levels,
                        "readable_domains": readable_domains,
                        "entity_count": entity_count,
                        "sound_count": sound_count,
                        "level_1_anchors": level_1_anchors,
                    }),
                )?;
            }
            if last_flush.elapsed() >= Duration::from_secs(1) {
                writer.flush().map_err(|error| error.to_string())?;
                last_flush = Instant::now();
            }
            if stop_reason != "safety_timeout" {
                break;
            }
        }

        write_json_line(
            &mut writer,
            &json!({
                "record_kind": "session_end",
                "sample": final_sample,
                "elapsed_ms": start.elapsed().as_secs_f64() * 1000.0,
                "reason": stop_reason,
            }),
        )?;
        writer.flush().map_err(|error| error.to_string())?;
        eprintln!(
            "wrote full-session timeline to {} ({stop_reason})",
            output.display()
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn observe_type_defaults(
        process: &Process,
        writer: &mut impl Write,
        seen_types: &mut HashMap<u32, (u32, [u16; 4])>,
        failed_types: &mut HashMap<u32, (u64, String)>,
        seen_models: &mut HashMap<u16, (u32, String)>,
        failed_models: &mut HashMap<(u16, u32), (u64, String)>,
        sample: u64,
        elapsed_ms: f64,
        hz: u32,
        entity_type: u32,
        entity_pointer: u32,
    ) -> Result<(), String> {
        if seen_types.contains_key(&entity_type) {
            return Ok(());
        }
        if failed_types
            .get(&entity_type)
            .is_some_and(|(last_sample, _)| sample.saturating_sub(*last_sample) < u64::from(hz))
        {
            return Ok(());
        }
        match crate::model::resolve_type_defaults(process, entity_type) {
            Ok(defaults) => {
                write_json_line(
                    writer,
                    &json!({
                        "record_kind": "entity_type_defaults",
                        "sample": sample,
                        "elapsed_ms": elapsed_ms,
                        "first_entity_pointer": entity_pointer,
                        "defaults": defaults,
                    }),
                )?;
                seen_types.insert(entity_type, (defaults.record_pointer, defaults.models));
                failed_types.remove(&entity_type);
                for (slot, model_id) in defaults.models.into_iter().enumerate() {
                    observe_model_resource(
                        process,
                        writer,
                        seen_models,
                        failed_models,
                        sample,
                        elapsed_ms,
                        hz,
                        model_id,
                        0,
                        json!({
                            "source": "entity_type_default",
                            "entity_type": entity_type,
                            "slot": slot,
                        }),
                    )?;
                }
            }
            Err(error) => {
                let changed = failed_types
                    .get(&entity_type)
                    .is_none_or(|(_, before)| before != &error);
                if changed {
                    write_json_line(
                        writer,
                        &json!({
                            "record_kind": "entity_type_defaults_error",
                            "sample": sample,
                            "elapsed_ms": elapsed_ms,
                            "entity_type": entity_type,
                            "error": error,
                        }),
                    )?;
                }
                failed_types.insert(entity_type, (sample, error));
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn observe_model_resource(
        process: &Process,
        writer: &mut impl Write,
        seen: &mut HashMap<u16, (u32, String)>,
        failed: &mut HashMap<(u16, u32), (u64, String)>,
        sample: u64,
        elapsed_ms: f64,
        hz: u32,
        model_id: u16,
        entry_pointer_hint: u32,
        context: serde_json::Value,
    ) -> Result<(), String> {
        // Zero is the absent/fallback marker in entity override slots.
        if model_id == 0 {
            return Ok(());
        }
        let entry_pointer = if entry_pointer_hint != 0 {
            entry_pointer_hint
        } else {
            let table = process.read_u32(crate::entity::MODEL_POOL_PTR).unwrap_or(0);
            if table == 0 {
                0
            } else {
                process
                    .read_u32(table as usize + usize::from(model_id) * 4)
                    .unwrap_or(0)
            }
        };
        if entry_pointer == 0 {
            return Ok(());
        }
        if seen
            .get(&model_id)
            .is_some_and(|(pointer, _)| *pointer == entry_pointer)
        {
            return Ok(());
        }
        let failure_key = (model_id, entry_pointer);
        if failed
            .get(&failure_key)
            .is_some_and(|(last_sample, _)| sample.saturating_sub(*last_sample) < u64::from(hz))
        {
            return Ok(());
        }

        match crate::model::resolve(process, model_id) {
            Ok(resource) => {
                let changed = seen
                    .get(&model_id)
                    .is_none_or(|(before_pointer, before_hash)| {
                        *before_pointer != resource.entry_pointer
                            || before_hash != &resource.fnv1a64
                    });
                if changed {
                    write_json_line(
                        writer,
                        &json!({
                            "record_kind": "model_resource",
                            "sample": sample,
                            "elapsed_ms": elapsed_ms,
                            "context": context,
                            "resource": resource,
                        }),
                    )?;
                }
                seen.insert(model_id, (resource.entry_pointer, resource.fnv1a64.clone()));
                failed.remove(&failure_key);
            }
            Err(error) => {
                let changed = failed
                    .get(&failure_key)
                    .is_none_or(|(_, before)| before != &error);
                if changed {
                    write_json_line(
                        writer,
                        &json!({
                            "record_kind": "model_resource_error",
                            "sample": sample,
                            "elapsed_ms": elapsed_ms,
                            "model_id": model_id,
                            "entry_pointer": entry_pointer,
                            "context": context,
                            "error": error,
                        }),
                    )?;
                }
                failed.insert(failure_key, (sample, error));
            }
        }
        Ok(())
    }

    fn write_domain_error_once(
        writer: &mut impl Write,
        last_errors: &mut HashMap<&'static str, String>,
        domain: &'static str,
        sample: u64,
        elapsed_ms: f64,
        error: String,
    ) -> Result<(), String> {
        if last_errors.get(domain) != Some(&error) {
            write_json_line(
                writer,
                &json!({
                    "record_kind": "domain_read_error",
                    "domain": domain,
                    "sample": sample,
                    "elapsed_ms": elapsed_ms,
                    "error": error,
                }),
            )?;
        }
        last_errors.insert(domain, error);
        Ok(())
    }

    fn has_level_one_anchors(records: &[EntityRecord]) -> bool {
        let has_player = records.iter().any(|record| record.entity_type == 46);
        has_player
            && has_entity_at(records, 6, 80, 60)
            && has_entity_at(records, 68, 75, 60)
            && has_entity_at(records, 66, 87, 58)
    }

    fn sound_resource_sources(
        catalog: &crate::resources::ResourceCatalog,
    ) -> SoundResourceSourceKey {
        let sources = catalog
            .loaded_overlays
            .iter()
            .filter_map(|overlay| {
                let section = overlay.sections.get(11)?;
                (section.count != 0).then_some((
                    overlay.level,
                    overlay.load_state,
                    overlay.descriptor_source_level,
                    section.global_base,
                    section.count,
                ))
            })
            .collect();
        (catalog.active_variant, sources)
    }

    fn derive_session_phase(
        game_state: Option<i32>,
        frontend_flag: u32,
        menu_depth: u32,
        loaded_levels: &[u32],
        level_1_anchors: bool,
    ) -> &'static str {
        if level_1_anchors {
            "level_1"
        } else if loaded_levels.contains(&13) && loaded_levels.contains(&50) {
            "intro2_to_level_1"
        } else if loaded_levels.contains(&13) {
            "level_1_loading"
        } else if loaded_levels.contains(&50) {
            "intro2"
        } else {
            match game_state {
                Some(0) => "startup_avi",
                Some(1) => "frontend_scene",
                Some(2) if frontend_flag != 0 || menu_depth != 0 => "frontend",
                Some(2) => "transition_or_loading",
                None => "unreadable",
                _ => "unlabelled",
            }
        }
    }

    fn has_entity_at(records: &[EntityRecord], entity_type: u32, x: i16, z: i16) -> bool {
        records.iter().any(|record| {
            record.entity_type == entity_type
                && record.position_raw_8_8[0] == x.saturating_mul(256)
                && record.position_raw_8_8[2] == z.saturating_mul(256)
        })
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

    fn parse_u32(value: &str) -> Result<u32, String> {
        if let Some(hex) = value
            .strip_prefix("0x")
            .or_else(|| value.strip_prefix("0X"))
        {
            u32::from_str_radix(hex, 16).map_err(|error| error.to_string())
        } else {
            value.parse::<u32>().map_err(|error| error.to_string())
        }
    }

    #[cfg(test)]
    mod tests {
        use std::path::PathBuf;

        use clap::Parser;

        use super::{
            cadence_due, camera_eye_surface_pair_is_joint_stable, derive_session_phase,
            sound_resource_sources, Cli, Command,
        };
        use crate::resources::{LoadedOverlay, ResourceCatalog, SectionRange};

        fn catalog_with_sections(
            active_variant: u32,
            sound_count: u32,
            model_count: u32,
        ) -> ResourceCatalog {
            let mut sections: Vec<SectionRange> = (0..15)
                .map(|section| SectionRange {
                    section,
                    global_base: 0,
                    count: 0,
                })
                .collect();
            sections[8].count = model_count;
            sections[11].count = sound_count;
            ResourceCatalog {
                section_count: 15,
                level_count: 53,
                active_variant,
                section_totals: [0; 15],
                loaded_overlays: vec![LoadedOverlay {
                    level: 3,
                    descriptor_pointer: 0x1000,
                    load_state: 1,
                    descriptor_source_level: 3,
                    sections,
                }],
            }
        }

        #[test]
        fn camera_eye_surface_pair_requires_one_joint_stable_tick() {
            assert!(camera_eye_surface_pair_is_joint_stable(
                true,
                true,
                100,
                Some(100),
                Some(true)
            ));
            assert!(!camera_eye_surface_pair_is_joint_stable(
                false,
                true,
                100,
                Some(100),
                Some(true)
            ));
            assert!(!camera_eye_surface_pair_is_joint_stable(
                true,
                false,
                100,
                Some(100),
                Some(true)
            ));
            assert!(!camera_eye_surface_pair_is_joint_stable(
                true,
                true,
                100,
                Some(101),
                Some(true)
            ));
            assert!(!camera_eye_surface_pair_is_joint_stable(
                true,
                true,
                100,
                Some(100),
                Some(false)
            ));
        }

        #[test]
        fn loaded_overlay_ids_disambiguate_intro2_and_first_world() {
            let system_only = vec![0, 1, 2, 3, 4, 5, 12];
            assert_eq!(
                derive_session_phase(Some(2), 1, 1, &system_only, false),
                "frontend"
            );
            assert_eq!(
                derive_session_phase(Some(0), 0, 0, &system_only, false),
                "startup_avi"
            );

            let mut intro2 = system_only.clone();
            intro2.push(50);
            assert_eq!(
                derive_session_phase(Some(2), 0, 0, &intro2, false),
                "intro2"
            );

            let mut handoff = intro2;
            handoff.push(13);
            assert_eq!(
                derive_session_phase(Some(2), 0, 0, &handoff, false),
                "intro2_to_level_1"
            );
            assert_eq!(
                derive_session_phase(Some(2), 0, 0, &handoff, true),
                "level_1"
            );
        }

        #[test]
        fn sound_catalog_rescan_key_ignores_unrelated_sections() {
            let before = catalog_with_sections(1, 103, 100);
            let models_changed = catalog_with_sections(1, 103, 200);
            let sounds_changed = catalog_with_sections(1, 104, 100);
            let variant_changed = catalog_with_sections(2, 103, 100);

            assert_eq!(
                sound_resource_sources(&before),
                sound_resource_sources(&models_changed)
            );
            assert_ne!(
                sound_resource_sources(&before),
                sound_resource_sources(&sounds_changed)
            );
            assert_ne!(
                sound_resource_sources(&before),
                sound_resource_sources(&variant_changed)
            );
        }

        #[test]
        fn factory_manufacturing_audio_command_retains_sampling_and_stop_paths() {
            let cli = Cli::try_parse_from([
                "v2k-original-inspect",
                "factory-manufacturing-audio",
                "--seconds",
                "20",
                "--hz",
                "50",
                "--stop-file",
                "factory-audio.stop",
                "--output",
                "factory-audio.jsonl",
            ])
            .expect("factory manufacturing audio command should parse");

            match cli.command {
                Some(Command::FactoryManufacturingAudio {
                    seconds,
                    hz,
                    stop_file,
                    output,
                }) => {
                    assert_eq!(seconds, 20.0);
                    assert_eq!(hz, 50);
                    assert_eq!(stop_file, Some(PathBuf::from("factory-audio.stop")));
                    assert_eq!(output, PathBuf::from("factory-audio.jsonl"));
                }
                _ => panic!("unexpected command variant"),
            }
        }

        #[test]
        fn fan_audio_command_retains_sampling_and_stop_paths() {
            let cli = Cli::try_parse_from([
                "v2k-original-inspect",
                "fan-audio-timeline",
                "--seconds",
                "42.5",
                "--hz",
                "250",
                "--stop-file",
                "capture.stop",
                "--output",
                "fan.jsonl",
            ])
            .expect("fan audio command should parse");

            match cli.command {
                Some(Command::FanAudioTimeline {
                    seconds,
                    hz,
                    stop_file,
                    output,
                }) => {
                    assert_eq!(seconds, 42.5);
                    assert_eq!(hz, 250);
                    assert_eq!(stop_file, Some(PathBuf::from("capture.stop")));
                    assert_eq!(output, PathBuf::from("fan.jsonl"));
                }
                _ => panic!("unexpected command variant"),
            }
        }

        #[test]
        fn menu_pose_command_retains_sampling_and_stop_paths() {
            let cli = Cli::try_parse_from([
                "v2k-original-inspect",
                "menu-pose-timeline",
                "--seconds",
                "180",
                "--hz",
                "100",
                "--stop-file",
                "menu.stop",
                "--output",
                "menu-poses.jsonl",
            ])
            .expect("menu pose command should parse");

            match cli.command {
                Some(Command::MenuPoseTimeline {
                    seconds,
                    hz,
                    stop_file,
                    output,
                }) => {
                    assert_eq!(seconds, 180.0);
                    assert_eq!(hz, 100);
                    assert_eq!(stop_file, Some(PathBuf::from("menu.stop")));
                    assert_eq!(output, PathBuf::from("menu-poses.jsonl"));
                }
                _ => panic!("unexpected command variant"),
            }
        }

        #[test]
        fn fire_command_retains_context_rate_and_stop_path() {
            let cli = Cli::try_parse_from([
                "v2k-original-inspect",
                "fire-events",
                "--seconds",
                "300",
                "--hz",
                "1000",
                "--context-hz",
                "200",
                "--terrain-hz",
                "10",
                "--capture-terrain-infection",
                "--stop-file",
                "capture.stop",
                "--output",
                "powerups.jsonl",
            ])
            .expect("fire command should parse");

            match cli.command {
                Some(Command::FireEvents {
                    seconds,
                    hz,
                    context_hz,
                    terrain_hz,
                    capture_terrain_infection,
                    stop_file,
                    output,
                }) => {
                    assert_eq!(seconds, 300.0);
                    assert_eq!(hz, 1000);
                    assert_eq!(context_hz, 200);
                    assert_eq!(terrain_hz, 10);
                    assert!(capture_terrain_infection);
                    assert_eq!(stop_file, Some(PathBuf::from("capture.stop")));
                    assert_eq!(output, PathBuf::from("powerups.jsonl"));
                }
                _ => panic!("unexpected command variant"),
            }
        }

        #[test]
        fn entity_effects_command_retains_subject_and_player_evidence_flags() {
            let cli = Cli::try_parse_from([
                "v2k-original-inspect",
                "entity-effects-timeline",
                "--type",
                "0x2e",
                "--handle",
                "0x04b70001",
                "--seconds",
                "120",
                "--hz",
                "500",
                "--context-hz",
                "100",
                "--capture-player-controls",
                "--capture-vtol-evidence",
                "--capture-render-evidence",
                "--stop-file",
                "capture.stop",
                "--output",
                "entity.jsonl",
            ])
            .expect("entity effects command should parse");

            match cli.command {
                Some(Command::EntityEffectsTimeline {
                    entity_type,
                    handle,
                    seconds,
                    hz,
                    context_hz,
                    capture_player_controls,
                    capture_vtol_evidence,
                    capture_render_evidence,
                    stop_file,
                    output,
                }) => {
                    assert_eq!(entity_type, 46);
                    assert_eq!(handle, Some(0x04B7_0001));
                    assert_eq!(seconds, 120.0);
                    assert_eq!(hz, 500);
                    assert_eq!(context_hz, 100);
                    assert!(capture_player_controls);
                    assert!(capture_vtol_evidence);
                    assert!(capture_render_evidence);
                    assert_eq!(stop_file, Some(PathBuf::from("capture.stop")));
                    assert_eq!(output, PathBuf::from("entity.jsonl"));
                }
                _ => panic!("unexpected command variant"),
            }
        }

        #[test]
        fn entity_track_command_retains_render_evidence_flag() {
            let cli = Cli::try_parse_from([
                "v2k-original-inspect",
                "track-entity",
                "--type",
                "46",
                "--capture-player-controls",
                "--capture-vtol-evidence",
                "--capture-render-evidence",
                "--seconds",
                "120",
                "--hz",
                "100",
                "--stop-file",
                "capture.stop",
                "--output",
                "camera.jsonl",
            ])
            .expect("entity track command should parse");

            match cli.command {
                Some(Command::TrackEntity {
                    entity_type,
                    capture_player_controls,
                    capture_vtol_evidence,
                    capture_render_evidence,
                    stop_file,
                    output,
                    ..
                }) => {
                    assert_eq!(entity_type, 46);
                    assert!(capture_player_controls);
                    assert!(capture_vtol_evidence);
                    assert!(capture_render_evidence);
                    assert_eq!(stop_file, Some(PathBuf::from("capture.stop")));
                    assert_eq!(output, PathBuf::from("camera.jsonl"));
                }
                _ => panic!("unexpected command variant"),
            }
        }

        #[test]
        fn player_wreck_command_retains_context_and_stop_path() {
            let cli = Cli::try_parse_from([
                "v2k-original-inspect",
                "player-wreck-timeline",
                "--seconds",
                "90",
                "--hz",
                "500",
                "--context-hz",
                "100",
                "--stop-file",
                "wreck.stop",
                "--output",
                "wreck.jsonl",
            ])
            .expect("player wreck command should parse");

            match cli.command {
                Some(Command::PlayerWreckTimeline {
                    seconds,
                    hz,
                    context_hz,
                    stop_file,
                    output,
                }) => {
                    assert_eq!(seconds, 90.0);
                    assert_eq!(hz, 500);
                    assert_eq!(context_hz, 100);
                    assert_eq!(stop_file, Some(PathBuf::from("wreck.stop")));
                    assert_eq!(output, PathBuf::from("wreck.jsonl"));
                }
                _ => panic!("unexpected command variant"),
            }
        }

        #[test]
        fn particle_gate_command_retains_scene_and_stop_path() {
            let cli = Cli::try_parse_from([
                "v2k-original-inspect",
                "particle-render-gate-timeline",
                "--scene",
                "Intro2",
                "--seconds",
                "180",
                "--hz",
                "200",
                "--stop-file",
                "gate.stop",
                "--output",
                "gate.jsonl",
            ])
            .expect("particle gate command should parse");

            match cli.command {
                Some(Command::ParticleRenderGateTimeline {
                    scene,
                    seconds,
                    hz,
                    stop_file,
                    output,
                }) => {
                    assert_eq!(scene, "Intro2");
                    assert_eq!(seconds, 180.0);
                    assert_eq!(hz, 200);
                    assert_eq!(stop_file, Some(PathBuf::from("gate.stop")));
                    assert_eq!(output, PathBuf::from("gate.jsonl"));
                }
                _ => panic!("unexpected command variant"),
            }
        }

        #[test]
        fn world_behavior_command_retains_filters_and_cadences() {
            let cli = Cli::try_parse_from([
                "v2k-original-inspect",
                "world-behavior-timeline",
                "--scenario",
                "hive",
                "--type",
                "67",
                "--type",
                "17",
                "--model",
                "8",
                "--model",
                "47",
                "--handle",
                "0x04b70001",
                "--nearest-player",
                "8",
                "--seconds",
                "240",
                "--hz",
                "200",
                "--context-hz",
                "50",
                "--evidence-hz",
                "10",
                "--terrain-hz",
                "5",
                "--capture-effects",
                "--capture-terrain-infection",
                "--stop-file",
                "world.stop",
                "--output",
                "world.jsonl",
            ])
            .expect("world behavior command should parse");

            match cli.command {
                Some(Command::WorldBehaviorTimeline {
                    scenario,
                    entity_types,
                    model_ids,
                    handle,
                    nearest_player,
                    seconds,
                    hz,
                    context_hz,
                    evidence_hz,
                    terrain_hz,
                    capture_effects,
                    capture_terrain_infection,
                    stop_file,
                    output,
                }) => {
                    assert_eq!(scenario, "hive");
                    assert_eq!(entity_types, vec![67, 17]);
                    assert_eq!(model_ids, vec![8, 47]);
                    assert_eq!(handle, vec![0x04B7_0001]);
                    assert_eq!(nearest_player, 8);
                    assert_eq!(seconds, 240.0);
                    assert_eq!(hz, 200);
                    assert_eq!(context_hz, 50);
                    assert_eq!(evidence_hz, 10);
                    assert_eq!(terrain_hz, 5);
                    assert!(capture_effects);
                    assert!(capture_terrain_infection);
                    assert_eq!(stop_file, Some(PathBuf::from("world.stop")));
                    assert_eq!(output, PathBuf::from("world.jsonl"));
                }
                _ => panic!("unexpected command variant"),
            }
        }

        #[test]
        fn surface_effects_command_retains_primary_and_context_rates() {
            let cli = Cli::try_parse_from([
                "v2k-original-inspect",
                "vtol-surface-effects-timeline",
                "--seconds",
                "240",
                "--hz",
                "200",
                "--context-hz",
                "100",
                "--stop-file",
                "capture.stop",
                "--output",
                "surface.jsonl",
            ])
            .expect("surface effects command should parse");

            match cli.command {
                Some(Command::VtolSurfaceEffectsTimeline {
                    seconds,
                    hz,
                    context_hz,
                    stop_file,
                    output,
                }) => {
                    assert_eq!(seconds, 240.0);
                    assert_eq!(hz, 200);
                    assert_eq!(context_hz, 100);
                    assert_eq!(stop_file, Some(PathBuf::from("capture.stop")));
                    assert_eq!(output, PathBuf::from("surface.jsonl"));
                }
                _ => panic!("unexpected command variant"),
            }
        }

        #[test]
        fn radar_command_retains_all_three_sampling_rates_and_stop_path() {
            let cli = Cli::try_parse_from([
                "v2k-original-inspect",
                "radar-timeline",
                "--seconds",
                "300",
                "--hz",
                "100",
                "--context-hz",
                "20",
                "--buffer-hz",
                "10",
                "--stop-file",
                "capture.stop",
                "--output",
                "radar.jsonl",
            ])
            .expect("radar command should parse");

            match cli.command {
                Some(Command::RadarTimeline {
                    seconds,
                    hz,
                    context_hz,
                    buffer_hz,
                    stop_file,
                    output,
                }) => {
                    assert_eq!(seconds, 300.0);
                    assert_eq!(hz, 100);
                    assert_eq!(context_hz, 20);
                    assert_eq!(buffer_hz, 10);
                    assert_eq!(stop_file, Some(PathBuf::from("capture.stop")));
                    assert_eq!(output, PathBuf::from("radar.jsonl"));
                }
                _ => panic!("unexpected command variant"),
            }
        }

        #[test]
        fn radar_resource_snapshot_command_retains_output_path() {
            let cli = Cli::try_parse_from([
                "v2k-original-inspect",
                "radar-resources",
                "--output",
                "radar-resources.json",
                "--pid",
                "4321",
            ])
            .expect("radar resource command should parse");

            assert_eq!(cli.pid, Some(4321));

            match cli.command {
                Some(Command::RadarResources { output }) => {
                    assert_eq!(output, PathBuf::from("radar-resources.json"));
                }
                _ => panic!("unexpected command variant"),
            }
        }

        #[test]
        fn context_cadence_is_even_for_the_default_two_to_one_rates() {
            let due = (0..8)
                .filter(|sample| cadence_due(*sample, 200, 100))
                .collect::<Vec<_>>();
            assert_eq!(due, vec![0, 2, 4, 6]);
        }
    }
}
