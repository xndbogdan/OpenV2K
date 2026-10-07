#![cfg_attr(windows, windows_subsystem = "windows")]

use clap::{Parser, Subcommand, ValueEnum};
use std::collections::HashSet;
use std::fs::File;
use std::io::{LineWriter, Write};
use std::num::NonZeroU64;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

mod diagnostic_console;
mod launcher;
mod launcher_startup;
mod live_display;
use v2k_game::{launcher_preferences, setup};
mod paused_gameplay;
use paused_gameplay::{
    setting_requires_paused_redraw, GameplayOverlayFrame, PausedGameplayDraw, PausedGameplayScene,
};
#[cfg(test)]
mod native_weapon_visual_smoke;
#[cfg(test)]
mod plasma_presentation_tests;
mod type9_task_diagnostics;

/// Global verbose flag — when false, suppresses diagnostic output.
static VERBOSE: AtomicBool = AtomicBool::new(false);

/// Print to stderr only when verbose mode is enabled.
macro_rules! log {
    ($($arg:tt)*) => {
        if VERBOSE.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!($($arg)*);
        }
    };
}

use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;

use v2k_formats::anim_sound::SoundPool;
use v2k_formats::avi::AviPlayer;
use v2k_formats::fixed_math::retail_sine_q15;
use v2k_formats::models::{AnimVars, StagedEffectRequest};
use v2k_game::actor_standard_death_live::Type17CommonDyingPublicationOutcome;
use v2k_game::campaign_failure::{
    CampaignCasualtyFrame, CampaignSelectorCursor, CampaignSelectorEntry, CampaignSelectorStep,
};
use v2k_game::campaign_transition::{
    CampaignArrivalCatalog, CampaignTransition, CampaignWarpRuntime, FailedWorldRetry,
};
use v2k_game::chase_camera::{
    ChaseBodyBasis, ChaseCameraState, ChaseCameraTarget, ChaseTerrainContext,
};
use v2k_game::common_mover::type9_attitude::Type9BodyBasis;
use v2k_game::damage::EntityHitEntry;
use v2k_game::debug_panel::DebugPanel;
use v2k_game::entity::{
    world_position_raw, BaseFactoryProgressionEvent, BeamCommand, BeamOutcome, CargoDropContext,
    CargoProxyEvent, CheckedProjectileDamageOutcome, CheckedProjectileDamageRequest,
    CheckedProjectileDamageUnresolved, CheckedProjectileHiveDeathStarted, Entity, EntityManager,
    Fun00411250ImpactReactionOutcome, Fun00411250Type9DamageOutcome, PlayerCargoFrame,
    PlayerCheckedDamageFrame, PlayerCheckedDamageOutcome, PlayerCheckedDamageRequest,
    PlayerUpdateRequest, PlayerVtolFrameDiagnostics, PlayerWaterEntryResponse,
};
#[cfg(test)]
use v2k_game::entity::{PlayerSurfaceContactDispatch, PlayerSurfaceContactPhase};
use v2k_game::entity_collision_state::{
    terrain_collision_subject_eligible, EntityTypeRuntimeMetadata, RetailRuntimeValue,
};
use v2k_game::entity_positional_audio::{EntityPositionalAudio, EntityPositionalAudioFrame};
use v2k_game::entity_view_detail::{RetailViewDetail, RetailViewDetailContext};
use v2k_game::factory_pair_live::{
    resolve_factory_pair_arrivals, FactoryPairPassResult, FactoryPairRequest,
};
use v2k_game::full_frame_sprite_sequence::{
    decode_full_frame_sprite, FullFrameSpriteFrame, FullFrameSpriteSequence,
};
use v2k_game::fullscreen_map_status::FullscreenMapStatusResources;
use v2k_game::game_state::{
    ambient_setting_value, apply_setting_to_config, FrontendDepthFadeMode, GameState,
    Intro2PresentationStage, KlausBackdropAnimState, LoadingPurpose, MenuCtx, MenuLayout,
    MenuShell, PostIntroStage, ShellEvent,
};
use v2k_game::gameplay_hud::{
    advance_retail_bar_clock, retail_weapon_sprite_rect, GameplayHud, GameplayHudClip,
    GameplayHudFrame, GameplayHudLayout, GameplayHudResources, GameplayHudWeaponRoster,
};
use v2k_game::gameplay_notifications::{
    GameplayNotificationLine, GameplayNotifications, TextTypewriterCadence,
    GAMEPLAY_TEXT_TYPE_SOUND_ID,
};
use v2k_game::gameplay_radar::{GameplayRadar, RadarHudFrame, TerrainRadarView};
use v2k_game::hive_controller::{objective_hostile_present, HIVE_ENTITY_TYPE};
use v2k_game::hive_death::hive_death_radial_template;
use v2k_game::hover::{HoverPhysicsConfig, RETAIL_FRAME_DELTA_MAX_US};
use v2k_game::infection_evolution::infected_cell_count;
use v2k_game::main_base_abort::{
    MainBaseAbortActorLease, MainBaseAbortControllerStorage, MainBaseAbortFrameRequest,
    MainBaseAbortTransactionId, MainBaseAbortWorldControlLease, MainBaseTerminalAbortOrigin,
};
use v2k_game::main_base_abort_production::execute_campaign_abort;
use v2k_game::main_base_conversion_live::{
    resolve_first_world_main_base_conversions, FirstWorldMainBaseConversionPass,
    MainBaseConversionFrame,
};
use v2k_game::main_base_type54_abort::MainBaseType54SeaLevelProductionOutcome;
use v2k_game::main_base_type66_production::MainBaseType66ProductionOutcome;
use v2k_game::main_base_type9_production::MainBaseType9ExplodingProductionOutcome;
use v2k_game::menu::{compute_carousel_layout, scale_rgba, tint_rgba};
use v2k_game::menu_billboard::{MenuBillboardLayout, MenuBillboardPose, MenuBillboardRect};
use v2k_game::menu_data::SettingId;
use v2k_game::menu_engine::MenuInput;
use v2k_game::model_tree::{
    linked_model_named_tips, model_is_camera_facing_actor, ModelSceneLight as MenuSceneLight,
    ModelTreeChildTransform, ModelTreeNamedTipRequest, ModelTreePainterComposition,
    ModelTreeRenderer, ModelTreeRootLinkPolicy, ModelTreeView,
};
use v2k_game::native_entity_weapons::{
    launch::{
        entity_weapon_direction_q31, entity_weapon_drain_position_raw,
        entity_weapon_launch_velocity_raw,
    },
    EntityWeaponConstructionRequest, EntityWeaponKind,
};
use v2k_game::opening::{
    active_captions, intro2_backdrop, intro2_finished, intro2_uses_live_actor_pose,
    intro_actor_heading, intro_actor_pose, intro_actor_visible, Intro2Backdrop,
    IntroCameraController, IntroCameraEyeUpdate, FIRST_WORLD_LEVEL_ID, INTRO2_DURATION_SECS,
    INTRO2_LEVEL_ID, POST_INTRO_SOUND_ID,
};
use v2k_game::overlay_51_backdrop::Overlay51Backdrop;
use v2k_game::particle_descriptors::particle_descriptor;
use v2k_game::player::{
    FuelWarningCadence, MotionChannels, PlayerCraft, PlayerHoverAttitudeRequest,
    VehicleModeToggleOutcome,
};
use v2k_game::player_active_contact::{entity_pair_to_world, PlayerActivePairPass};

use v2k_game::player_contact_style::PlayerContactStyleRequest;
use v2k_game::player_fan_audio::PlayerFanAudio;
use v2k_game::player_hull::{
    HullDamageProfile, HullWarningCadence, PlayerDeathLifecycle, PlayerDeathTick, PlayerHull,
    HULL_LOW_WARNING_BELOW_RAW, PLAYER_TYPE_46_HULL_PROFILE,
};
use v2k_game::player_surface_contact::{PlayerSurfaceContactFrame, PlayerSurfaceContactRequest};
use v2k_game::power_up_contact::{
    AcceptedPowerUpContact, CampaignControlSlotError, NonSpawningPowerUpAcquisition,
    PlayerCampaignProgress, PlayerPowerUpContactPass, PlayerPowerUpRecipient,
    PowerUpContactOutcome, PowerUpContactRejection, TrophyAcquisition, RETAIL_CONTROL_SLOT_COUNT,
    TARGETTER_DUPLICATE_RATE_Q16, TROPHY_EXTRA_LIFE_RATE_Q16, TROPHY_PICKUP_SOUND_ID,
    TURBO_DUPLICATE_RATE_Q16, WEAPON_PICKUP_REJECTED_SOUND_ID,
};
#[cfg(test)]
use v2k_game::primary_weapon::UPGRADED_PRIMARY_PROFILE;
use v2k_game::primary_weapon::{
    fire_profile_from_descriptor, PrimaryAuthoredGunMounts, PrimaryFireGeometry, PrimaryGunChannel,
    PrimaryGunMounts, PrimaryLaunchBasis, PrimaryShotBudget, PrimaryTriggerInput, PrimaryWeapon,
};
use v2k_game::projectile_emitter::projectile_class_row;
use v2k_game::radial_damage::radial_distance_raw;
use v2k_game::retail_clock::RetailTickClock;
use v2k_game::save::{SaveManager, SavePathContext, SaveSource, SavedPlayerState};
use v2k_game::specialized_actor_task_production::{
    SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
    SpecializedActorTaskScheduler,
};
use v2k_game::static_contact::{
    resolve_player_static_contact, PlayerStaticStyleCallbackFrame,
    PlayerStaticStyleCallbackOutcome, StaticPickupOutcome,
};
use v2k_game::static_damage::{
    scan_static_radial, StaticDamageAction, StaticDamageOutcome, StaticDamageScheduler,
};
use v2k_game::static_damage_live::resolve_current_static_damage_target;
use v2k_game::static_objects::{
    collect_retail_static_terrain_objects, collect_static_terrain_objects,
    free_camera_entity_view_contains_position,
};
use v2k_game::targetter::{
    fun_0044ea60, targetter_model_scale_raw, targetter_terrain_height_raw, TargetterCandidate,
    TargetterCrosshairKind, TargetterModelProbe, TargetterRay, TargetterRuntime,
    TargetterUpdateRequest,
};
use v2k_game::terrain_contact::DEFAULT_PLAYER_TERRAIN_HAPTIC_SCALE_RAW;
use v2k_game::time_trophy::{TimeTrophyLoadOutcome, TimeTrophySound};
use v2k_game::type17_common_dying_production::Type17CommonDyingProductionOutcome;
use v2k_game::type17_primary_hit_live::{
    apply_level_one_type17_projectile_primary_hit, LevelOneType17PrimaryHitOutcome,
    LevelOneType17ProjectilePrimaryHitRequest,
};
use v2k_game::type47_checked_damage::{
    apply_type47_checked_damage_after_c690, Type47CheckedDamageOutcome, Type47CheckedDamageRequest,
};
use v2k_game::type47_common_dying_production::Type47CommonDyingProductionOutcome;
use v2k_game::type47_impact_live::{
    apply_type47_impact_c690_live, Type47ImpactLiveOutcome, Type47ImpactLiveRequest,
};
use v2k_game::type47_impact_reaction::{
    apply_type47_impact_reaction_after_c690, Type47ImpactReactionOutcome,
};
use v2k_game::type47_scheduler_production::OrdinaryType47SchedulerProductionOutcome;
use v2k_game::type60_exploding_ring_production::Type60ExplodingRingProductionOutcome;
use v2k_game::vtol::VehicleFrameForces;
use v2k_game::vtol::{VtolBoost, VtolHeightPolicy};
use v2k_game::weapon_inventory::{
    AmmoCommit, Ammunition, PlayerCapabilities, WeaponAcquisition, WeaponCycleDirection,
    WeaponInventory,
};
use v2k_game::world_complete_results::{
    fun_0042dd10_selector3_rescued, landscape_virus_percent, session_music_should_play,
    WorldCompleteResultsRuntime, WorldCompleteTally,
};
use v2k_game::world_fx::{
    particle_uses_fun_0043f780_entity_hit, EntityCollisionModel, ParticleCollisionCacheRefresh,
    ParticleEnvironment, ParticleOwnerMotion, ParticlePresentationFrame, PrimaryImpact,
    TerrainCollisionContext, TerrainExplosionLight, WorldFx, FUEL_EXHAUSTED_SOUND_ID,
    FUEL_LOW_SOUND_ID, HULL_LOW_SOUND_GAIN, HULL_LOW_SOUND_ID, PLAYER_PICKUP_SOUND_ID,
};
use v2k_game::world_projection::WorldProjection;
use v2k_render::config::GraphicsDetail;
use v2k_render::sound::{
    resolve_positional_sound, resolve_sound_playback, PositionalSoundListener,
    PositionalSoundRequest, SoundPlaybackRequest,
};
use v2k_render::{
    mat3_mul, orientation_from_ypr, project_particle_center, AudioPlayer, Camera, CapturedFrame,
    ExternalFrameMode, FrameCaptureSource, GameConfig, GameEvent, GameWindow, ModelDepthFade,
    ModelNearClip, ModelOverlayKind, MusicPlayer, ProjectionEffect, RenderScene, Renderer,
    RendererChoice, SoundListener, SoundManager, SpriteFog, ViewPinMode, WorldSprite,
    WorldSpriteBlend, HALF_ADDITIVE_ALPHA,
};

/// Sea-plane tint for the translucent water pass. A flat sea blue stands in
/// for the engine's animated Section 9 water sprite frames (not yet wired);
/// the ~50% alpha is applied in the renderer to match the engine's blend.
const WATER_COLOR: [f32; 3] = [0.12, 0.32, 0.52];

/// Stored render-context far words consumed by `FUN_0043D410`, as captured in
/// the stable retail Intro2 and Level-1 world contexts. These are deliberately
/// separate from the transient menu/iris values seen before Intro2 becomes
/// visible.
const INTRO2_PARTICLE_FAR_DEPTH_RAW: i32 = 0x1800;
const GAMEPLAY_PARTICLE_FAR_DEPTH_RAW: i32 = 0x1800;

/// Retail session control slots resolve their world OVL as `slot + 12`.
/// New Game enters slot 1, which therefore loads Level-1 overlay 13.
const WORLD_OVERLAY_CONTROL_SLOT_OFFSET: u32 = FIRST_WORLD_LEVEL_ID - 1;

fn world_control_slot(level_id: u32) -> Option<usize> {
    level_id
        .checked_sub(WORLD_OVERLAY_CONTROL_SLOT_OFFSET)
        .and_then(|slot| usize::try_from(slot).ok())
        .filter(|slot| *slot < RETAIL_CONTROL_SLOT_COUNT)
}

/// Apply the OpenGL classic-framebuffer preference. The software backend
/// already presents its authored-resolution surface in the 4:3 modes, so the
/// preference is kept for a later OpenGL launch rather than cleared.
fn apply_classic_framebuffer_presentation(
    renderer: &mut dyn Renderer,
    config: &mut GameConfig,
    backend: v2k_render::RenderBackend,
) {
    let requested = config.classic_framebuffer_effective();
    let active = renderer.set_classic_framebuffer(requested);
    if requested && !active && backend == v2k_render::RenderBackend::OpenGL {
        log!("Classic framebuffer is unavailable on the active renderer; disabling the option");
        config.classic_framebuffer = false;
        renderer.set_classic_framebuffer(false);
    }
}

/// Display-menu Rendering change (`FUN_0043CC70` -> `FUN_0043CC40` ->
/// `FUN_0044E0E0`). Retail rebuilds the display at once and, when the new
/// one cannot start, keeps running on the previous one; the menu value stays
/// as chosen. The new renderer starts without textures, so the caller
/// re-uploads what the previous one held.
fn replace_renderer(
    game_window: &GameWindow,
    renderer: &mut Box<dyn Renderer>,
    config: &GameConfig,
) -> Result<v2k_render::RenderBackend, String> {
    let (replacement, backend) = v2k_render::create_renderer(
        game_window,
        "V2K",
        config.width,
        config.height,
        config.resolve_backend(None),
        false,
    )?;
    let world_model_fog = renderer.retained_world_model_fog();
    drop(std::mem::replace(renderer, replacement));
    renderer.set_world_model_fog(world_model_fog);
    Ok(backend)
}

/// Ordinary worlds run the conversion/intake walkers on authenticated owners.
fn ordinary_world_pair_pass_required(current_level_id: Option<u32>) -> bool {
    current_level_id.is_some_and(|id| (13..=49).contains(&id))
}

/// Whether the session should run the bounded player-solid adapter.
///
/// Intro2 keeps the previous skip so its cinematic Power-Up path is not
/// gated on a solid pass. Campaign worlds run the adapter and only visit
/// candidates whose constructor/census pair identity is closed.
fn player_active_solid_pass_required(current_level_id: Option<u32>) -> bool {
    match current_level_id {
        Some(id) if id == INTRO2_LEVEL_ID => false,
        Some(_) => true,
        None => false,
    }
}

fn queue_premature_hive_hit_hint(
    em: &EntityManager,
    outcome: &CheckedProjectileDamageOutcome,
    retail_tick: i32,
    notifications: &mut GameplayNotifications,
) {
    let target_id = match *outcome {
        CheckedProjectileDamageOutcome::Applied(applied) => applied.target_id,
        CheckedProjectileDamageOutcome::Unresolved {
            target_id,
            reason: CheckedProjectileDamageUnresolved::HiveObjectiveHostilesRemain,
        } => target_id,
        _ => return,
    };
    if !em
        .iter()
        .any(|entity| entity.id == target_id && entity.entity_type == HIVE_ENTITY_TYPE)
    {
        return;
    }
    if objective_hostile_present(em.iter()) != Ok(true) {
        return;
    }
    notifications.queue_hive_locked_hint(retail_tick);
}

fn campaign_saved_world_count(progress: &PlayerCampaignProgress) -> i32 {
    (0..RETAIL_CONTROL_SLOT_COUNT)
        .filter(|&slot| progress.control_slot_world_saved(slot) == Some(true))
        .count() as i32
}

/// Hive lethal records completion and event 3 / `0xE4` in the active HUD.
/// The progress map is owned by the later dead-hive exit transition.
/// `0xC2` snapshots `FUN_004366F0` from the current level's infection census.
fn record_world_complete_after_hive_death(
    notifications: &mut GameplayNotifications,
    results: &mut WorldCompleteResultsRuntime,
    abort: &mut MainBaseAbortBridgeState,
    progress: &mut PlayerCampaignProgress,
    em: &EntityManager,
    tally: &mut WorldCompleteTally,
    terrain: Option<&v2k_formats::terrain::TerrainGrid>,
    retail_tick: i32,
    world_fx: &mut WorldFx,
) {
    notifications.queue_hive_destroyed_hint(retail_tick);
    // 4567B0 gates both the time stamp and2EE70 campaign write on session28F
    // and the retained timestamp. A failed Hive kill only opens its retry.
    if tally.stamp_fun_004567b0(abort.abort_flag_0x28f(), retail_tick) {
        match abort.complete_current_world(progress) {
            Ok(outcome) => {
                if let Some((reward, player)) = outcome.reward.zip(em.player()) {
                    queue_trophy_acquisition_feedback(
                        reward,
                        player.position_raw(),
                        notifications,
                        world_fx,
                        retail_tick as u32,
                    );
                }
            }
            Err(error) => {
                eprintln!("Ordinary world-complete campaign transaction failed: {error:?}")
            }
        }
    }
    tally.recense_fun_0042dd10(em.fun_0042e210_capability_census());
    let slot = progress.current_control_slot();
    results.record_hive_completion(
        tally.snapshot_stats(
            landscape_virus_percent(terrain.map(infected_cell_count).unwrap_or(0)),
            slot.and_then(|slot| progress.control_slot_time_trophy_claimed(slot))
                .unwrap_or(false),
            slot.and_then(|slot| progress.control_slot_claimed(slot))
                .unwrap_or(false),
            campaign_saved_world_count(progress),
        ),
    );
}

/// Shared `FUN_00440950` burst then `FUN_004566E0` static-then-dynamic radial.
/// Deferred until after the particle traversal so the cache borrow matches the
/// player-wreck helper. Dying-slot-0 `FUN_004260F0` keeps shared `FUN_0041BEB0`
/// and ramps the Sub-K bound u16 on the wreck.
struct OrdinaryHiveDeathFrame<'a> {
    entities: &'a mut EntityManager,
    cache: &'a mut v2k_game::resource_cache::ResourceCache,
    world_fx: &'a mut WorldFx,
    static_damage: &'a mut StaticDamageScheduler,
    player_hull: &'a mut PlayerHull,
    extra_lives: RetailRuntimeValue<u8>,
    actor_tasks: &'a mut SpecializedActorTaskScheduler,
    notifications: &'a mut GameplayNotifications,
    retail_tick: u32,
}

fn apply_ordinary_hive_death_burst_and_radial(
    frame: OrdinaryHiveDeathFrame<'_>,
    started: CheckedProjectileHiveDeathStarted,
) {
    let OrdinaryHiveDeathFrame {
        entities: em,
        cache,
        world_fx,
        static_damage,
        player_hull,
        extra_lives,
        actor_tasks,
        notifications,
        retail_tick,
    } = frame;
    let source_extent_raw = cache
        .global_model(started.model_after)
        .map(|model| model.radius)
        .unwrap_or(0);
    let sea_level_raw = cache
        .terrain()
        .and_then(|terrain| terrain.water_enabled().then(|| terrain.sea_level_raw()));
    world_fx.emit_scatter_surface_burst_raw(
        started.target_position_raw,
        source_extent_raw,
        sea_level_raw,
        started.target_id,
    );

    let template = hive_death_radial_template(started.target_id);
    let static_hits = cache.terrain().map_or_else(Vec::new, |terrain| {
        scan_static_radial(terrain, started.target_position_raw, template, |cell| {
            resolve_current_static_damage_target(cache, cell)
                .ok()
                .flatten()
                .map(|target| target.state)
        })
    });
    for hit in static_hits {
        let outcome = static_damage.submit_hit(hit.target, hit.packet, &mut || {
            world_fx.next_shared_retail_random_u16()
        });
        apply_immediate_static_damage_outcome(outcome, cache, world_fx);
        log!(
            "Hive death static radial: cell ({}, {}) distance {} {outcome:?}",
            hit.target.cell[0],
            hit.target.cell[1],
            hit.distance_raw
        );
    }

    let result = actor_tasks.apply_playing_radial_damage(
        v2k_game::specialized_actor_task_production::PlayingRadialFrame {
            resources: cache,
            static_damage,
            active_terminal_calls: Vec::new(),
            entities: em,
            player_hull,
            extra_lives,
            world_fx,
            notifications,
            retail_tick,
            origin_raw: started.target_position_raw,
            template,
        },
    );
    log!("Hive death dynamic radial: {result:?}");
}

///424650 retains the direction and source while11400/14870 consume the
/// transient after simulation/contact. A body gets its first13500 visit on
/// the following frame, with current source velocity and Euler words.
#[derive(Debug, Clone, Copy)]
struct PendingEntityWeaponFire {
    kind: EntityWeaponKind,
    source_actor_id: u32,
    direction_q31: [i32; 3],
    emitter_index: u16,
    time_offset_us: u32,
}

#[derive(Default)]
struct EntityWeaponDrawOrigins {
    points: [Option<[i16; 3]>; 2],
    admitted: bool,
    blocked: bool,
}

struct EntityWeaponPresentationFrame<'a> {
    entities: &'a mut EntityManager,
    cache: &'a v2k_game::resource_cache::ResourceCache,
    world_fx: &'a mut WorldFx,
    actor_tasks: &'a mut SpecializedActorTaskScheduler,
    retail_tick: u32,
    error_reported: &'a mut bool,
}

fn publish_entity_weapon_bodies(
    frame: EntityWeaponPresentationFrame<'_>,
    pending: &mut Vec<PendingEntityWeaponFire>,
    origins: &EntityWeaponDrawOrigins,
) {
    for shot in std::mem::take(pending) {
        if !origins.admitted || origins.blocked {
            if !*frame.error_reported {
                eprintln!(
                    "Native {:?} birth suppressed: unowned player draw/emitter origin",
                    shot.kind
                );
                *frame.error_reported = true;
            }
            continue;
        }
        let request = frame
            .entities
            .iter_all()
            .find(|actor| actor.id == shot.source_actor_id)
            .map(|source| EntityWeaponConstructionRequest {
                kind: shot.kind,
                source_actor_id: shot.source_actor_id,
                position_raw: entity_weapon_drain_position_raw(
                    shot.kind,
                    origins.points[usize::from(shot.emitter_index != 0)]
                        .unwrap_or_else(|| source.position_raw()),
                    shot.direction_q31,
                    source.velocity_raw(),
                    shot.time_offset_us,
                ),
                velocity_raw: entity_weapon_launch_velocity_raw(
                    shot.kind,
                    shot.direction_q31,
                    source.velocity_raw(),
                ),
                rotation_raw: source.rotation_heading_pitch_roll_raw(),
            });
        let result = request
            .ok_or(v2k_game::native_entity_weapons::EntityWeaponBlock::SourceAllocation)
            .and_then(|request| {
                frame.entities.construct_entity_weapon(
                    request,
                    frame.cache,
                    frame.world_fx,
                    frame.retail_tick,
                )
            });
        match result {
            Ok(owner) => {
                if let Err(reason) = frame
                    .actor_tasks
                    .register_native_weapon(frame.entities, owner)
                {
                    eprintln!(
                        "Native {:?} body {} task publication blocked: {reason:?}",
                        shot.kind,
                        owner.entity_id()
                    );
                }
            }
            Err(reason) if !*frame.error_reported => {
                eprintln!("Native {:?} birth suppressed: {reason:?}", shot.kind);
                *frame.error_reported = true;
            }
            Err(_) => {}
        }
    }
}

/// Host-side one-shot guard for the currently recovered subset of retail
/// session byte `+0x28F`.
///
/// The Main Base terminal path may be observed through more than one host-side
/// presentation edge, but `FUN_00456960` owns one world-abort transaction. The
/// recovered player state and full-frame request live in a distinct controller
/// owner so this bridge never aliases them with campaign progress. Retail's
/// preceding `FUN_0044C940`
/// only garbage-collects disposable one-shot logical requests; auto-collected
/// physical playback and owner-held persistent loops survive, so it is not a
/// blanket voice stop requiring a host-side `SoundManager::stop_all`.
#[derive(Debug, Default, PartialEq, Eq)]
struct MainBaseAbortBridgeState {
    /// Controller +0x1E8 warning latch, reset with the loaded world.
    casualty: v2k_game::campaign_failure::CampaignCasualtyState,
    /// Retail session byte `+0x28F`.
    active: bool,
    /// Retail session dword `+0x2BC`, reset before positional sound/dispatch.
    session_dword_0x2bc: Option<u32>,
    /// Retail session byte `+0x295`, requested after the dispatcher returns
    /// even when a null world-control slot suppressed its complete body.
    outer_frame_sequence: FullFrameSpriteSequence,
    /// Linear terminal-callback authority retained across outer admission and
    /// consumed by the synchronous systemic body.
    terminal_origin: Option<MainBaseTerminalAbortOrigin>,
    /// Non-retryable witness for a transaction that rolled back cleanly and
    /// selected the former bounded compatibility body instead.
    compatibility_fallback_actor: Option<MainBaseAbortActorLease>,
    controller: Option<MainBaseAbortControllerStorage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MainBaseAbortAdmission {
    Duplicate,
    BodySuppressed,
    BodyAdmitted(MainBaseAbortWorldControlLease),
}

/// Global positional sound submitted at the player before
/// `FUN_0042F1A0` begins the Main Base's systemic death dispatch.
const MAIN_BASE_ABORT_SOUND_ID: u16 = 0x3e;

impl MainBaseAbortBridgeState {
    fn bind_loaded_world(
        &mut self,
        load_generation: NonZeroU64,
        level: &v2k_formats::levels::LevelDescriptor,
        progress: &mut PlayerCampaignProgress,
    ) -> Result<TimeTrophyLoadOutcome, CampaignControlSlotError> {
        let (controller, time_trophy_load) = MainBaseAbortControllerStorage::from_loaded_level(
            load_generation,
            level,
            self.active,
            progress,
        )?;
        *self = Self {
            controller: Some(controller),
            ..Self::default()
        };
        Ok(time_trophy_load)
    }

    const fn abort_flag_0x28f(&self) -> bool {
        self.active
    }

    fn begin_header(&mut self, world_fx: &mut WorldFx) -> bool {
        if self.active {
            return false;
        }
        // FUN_00456960 calls FUN_0044C940 before either session write. In the
        // port, WorldFx owns exactly the queued/unsubmitted disposable subset.
        world_fx.garbage_collect_disposable_positional_sounds();
        self.active = true;
        self.session_dword_0x2bc = Some(0);
        true
    }

    #[cfg(test)]
    fn begin_fixture_without_terminal_origin(&mut self, world_fx: &mut WorldFx) -> bool {
        self.begin_header(world_fx)
    }

    fn finish_outer_dispatch(&mut self) {
        debug_assert!(self.active);
        self.outer_frame_sequence.request();
    }

    fn world_control_lease(&self) -> Option<MainBaseAbortWorldControlLease> {
        self.controller
            .as_ref()
            .map(MainBaseAbortControllerStorage::world_control_lease)
    }

    fn submitted_frame_request(&self) -> Option<MainBaseAbortFrameRequest> {
        self.controller
            .as_ref()
            .map(MainBaseAbortControllerStorage::submitted_frame_request)
    }

    fn advance_time_trophy(&mut self, elapsed_micros: u32) -> Option<TimeTrophySound> {
        self.controller
            .as_mut()
            .and_then(|controller| controller.advance_time_trophy(elapsed_micros))
    }

    fn stop_time_trophy_countdown_for_hidden_pickup(&mut self) {
        if let Some(controller) = self.controller.as_mut() {
            controller.stop_time_trophy_countdown_for_hidden_pickup();
        }
    }

    fn complete_current_world(
        &mut self,
        progress: &mut PlayerCampaignProgress,
    ) -> Result<v2k_game::time_trophy::TimeTrophyWorldCompletionOutcome, CampaignControlSlotError>
    {
        self.controller.as_mut().map_or(
            Err(CampaignControlSlotError::ControlSlotUnresolved),
            |controller| controller.complete_current_world(progress),
        )
    }
}

/// Begin `FUN_00456960` in retail order: player-positioned resource `0x3E`,
/// then admission of the world-control-gated systemic body.
///
/// Missing player state suppresses only the positional sound; retail still
/// calls `FUN_0042F1A0` after that resolver failure. A null world control slot
/// suppresses its entire broad death/darkness/player-handoff body.
fn begin_main_base_abort_transaction(
    state: &mut MainBaseAbortBridgeState,
    origin: Option<MainBaseTerminalAbortOrigin>,
    entities: &mut EntityManager,
    world_fx: &mut WorldFx,
) -> MainBaseAbortAdmission {
    // FUN_00456960 sets session +0x28F before resolving controller +0x68.
    if !state.begin_header(world_fx) {
        return MainBaseAbortAdmission::Duplicate;
    }

    begin_main_base_abort_transaction_body(state, origin, entities, world_fx)
}

fn begin_main_base_abort_transaction_body(
    state: &mut MainBaseAbortBridgeState,
    terminal_origin: Option<MainBaseTerminalAbortOrigin>,
    entities: &mut EntityManager,
    world_fx: &mut WorldFx,
) -> MainBaseAbortAdmission {
    if let Some(position_raw) = entities.player().map(|player| player.position_raw()) {
        world_fx.queue_fixed_positional_sound_raw(MAIN_BASE_ABORT_SOUND_ID, position_raw);
    } else {
        log!("Main Base terminal: player position unavailable for resource 0x3E");
    }

    let Some(world_control_lease) = state.world_control_lease() else {
        log!("Main Base terminal: world control slot unavailable; systemic abort body skipped");
        state.finish_outer_dispatch();
        return MainBaseAbortAdmission::BodySuppressed;
    };

    // The null world-control branch above returns before FUN_004170A0's actor
    // sweep. Retain its linear origin only when that complete systemic body is
    // actually admitted.
    state.terminal_origin = terminal_origin;

    MainBaseAbortAdmission::BodyAdmitted(world_control_lease)
}

#[cfg(test)]
fn begin_main_base_abort_transaction_fixture(
    state: &mut MainBaseAbortBridgeState,
    entities: &mut EntityManager,
    world_fx: &mut WorldFx,
) -> MainBaseAbortAdmission {
    if !state.begin_fixture_without_terminal_origin(world_fx) {
        return MainBaseAbortAdmission::Duplicate;
    }
    begin_main_base_abort_transaction_body(state, None, entities, world_fx)
}

/// Select the projector callbacks installed by retail `FUN_00433FA0`.
/// Equality remains dry: the alternate callbacks are installed only when the
/// camera/context Y is strictly below the Section-10 sea word.
fn world_projection_effect(
    camera_y: f32,
    sea_level_raw: Option<i16>,
    retail_tick: u32,
) -> ProjectionEffect {
    let camera_y_raw = (camera_y * 256.0).round() as i32;
    match sea_level_raw {
        Some(sea_level_raw) if camera_y_raw < i32::from(sea_level_raw) => {
            ProjectionEffect::RetailUnderwater {
                tick: retail_tick as i32,
            }
        }
        _ => ProjectionEffect::None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedFullFrameSpriteCommand {
    frame: FullFrameSpriteFrame,
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    blend: WorldSpriteBlend,
}

/// Append the current `FUN_00453410` frame against the selected global sprite
/// pool. Retail advances session `+0x295` when this presentation command is
/// built, after the mode pump's simulation visit. Its global-pool lookup is
/// unchecked; the port defensively retains the frame when
/// selected-tier data cannot be bound instead of manufacturing an invalid
/// render command.
fn prepare_full_frame_sprite_command(
    renderer: &dyn Renderer,
    cache: &v2k_game::resource_cache::ResourceCache,
    sequence: &mut FullFrameSpriteSequence,
) -> Option<PreparedFullFrameSpriteCommand> {
    let Some(frame) = sequence.current_frame() else {
        return None;
    };
    let command = full_frame_sprite_command_for_frame(renderer, cache, frame)?;
    let acknowledged = sequence.acknowledge_submitted(frame);
    debug_assert!(
        acknowledged,
        "the single-threaded render owner cannot stale"
    );
    Some(command)
}

/// Decode/scale an already-selected frame without advancing its sequence.
fn full_frame_sprite_command_for_frame(
    renderer: &dyn Renderer,
    cache: &v2k_game::resource_cache::ResourceCache,
    frame: FullFrameSpriteFrame,
) -> Option<PreparedFullFrameSpriteCommand> {
    let Some(sprite) = decode_full_frame_sprite(cache, frame) else {
        log!(
            "Full-frame sprite sequence: global sprite {} is unavailable; retaining table index {}",
            frame.global_sprite_id,
            frame.table_index,
        );
        return None;
    };
    let (width, height) = renderer.viewport_size();
    if width == 0 || height == 0 || sprite.width == 0 || sprite.height == 0 {
        return None;
    }

    let rgba = scale_rgba(&sprite.rgba, sprite.width, sprite.height, width, height);
    Some(PreparedFullFrameSpriteCommand {
        frame,
        rgba,
        width,
        height,
        blend: sprite.blend,
    })
}

fn draw_prepared_full_frame_sprite_command(
    renderer: &mut dyn Renderer,
    command: &PreparedFullFrameSpriteCommand,
) {
    // Retail stretches the authored two-by-two material over the complete
    // current viewport. Use the material path rather than draw_fullscreen:
    // id 0x227 is masked, while 0x228..=0x22C are additive.
    debug_assert_eq!(
        command.blend,
        if command.frame.global_sprite_id == 0x227 {
            WorldSpriteBlend::Masked
        } else {
            WorldSpriteBlend::Additive
        }
    );
    renderer.draw_material_sprite(
        &command.rgba,
        command.width,
        command.height,
        0,
        0,
        command.blend,
    );
}

/// One-shot gameplay edges recovered from the retail key reader.
///
/// C/D are active only while Left Shift is up. Because Shift can complete the
/// transition on release, both key-down and key-up events pass through here.
/// The held-key state remains available to ordinary gameplay/free-camera users.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct GameplayInputEdges {
    collect: bool,
    drop: bool,
    weapon_cycle: Option<WeaponCycleDirection>,
}

fn gameplay_key_transition(
    keys_down: &mut HashSet<Keycode>,
    key: Keycode,
    pressed: bool,
) -> GameplayInputEdges {
    let collect_was_down = !keys_down.contains(&Keycode::LShift) && keys_down.contains(&Keycode::C);
    let drop_was_down = !keys_down.contains(&Keycode::LShift) && keys_down.contains(&Keycode::D);
    let weapon_forward_was_down = weapon_cycle_forward_held(keys_down);
    let weapon_backward_was_down = weapon_cycle_backward_held(keys_down);

    if pressed {
        keys_down.insert(key);
    } else {
        keys_down.remove(&key);
    }

    let collect_is_down = !keys_down.contains(&Keycode::LShift) && keys_down.contains(&Keycode::C);
    let drop_is_down = !keys_down.contains(&Keycode::LShift) && keys_down.contains(&Keycode::D);
    let weapon_forward_is_down = weapon_cycle_forward_held(keys_down);
    let weapon_backward_is_down = weapon_cycle_backward_held(keys_down);
    GameplayInputEdges {
        collect: collect_is_down && !collect_was_down,
        drop: drop_is_down && !drop_was_down,
        weapon_cycle: if weapon_forward_is_down && !weapon_forward_was_down {
            Some(WeaponCycleDirection::Forward)
        } else if weapon_backward_is_down && !weapon_backward_was_down {
            Some(WeaponCycleDirection::Backward)
        } else {
            None
        },
    }
}

fn weapon_cycle_forward_held(keys_down: &HashSet<Keycode>) -> bool {
    [Keycode::B, Keycode::PageDown, Keycode::LShift, Keycode::L]
        .iter()
        .any(|key| keys_down.contains(key))
}

fn weapon_cycle_backward_held(keys_down: &HashSet<Keycode>) -> bool {
    [Keycode::V, Keycode::PageUp, Keycode::CapsLock, Keycode::K]
        .iter()
        .any(|key| keys_down.contains(key))
}

fn apply_gameplay_input_edges(
    edges: GameplayInputEdges,
    has_player: bool,
    player_dying: bool,
    entities: &mut EntityManager,
    inventory: &mut WeaponInventory,
    craft: &mut PlayerCraft,
) {
    if !has_player || player_dying {
        return;
    }
    if edges.collect {
        entities.queue_beam(BeamCommand::Collect);
    }
    if edges.drop {
        entities.queue_beam(BeamCommand::Drop);
    }
    if let Some(direction) = edges.weapon_cycle {
        inventory.cycle(direction);
        craft.select_weapon_callback(inventory.selected_descriptor().callback_selector());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameplayInterruption {
    QuitApplication,
    ReturnToFrontend,
    Pause,
}

/// Decide the only three transitions that may pre-empt a live gameplay tick.
/// Escape is deliberately a pause request; only the completed death lifecycle
/// or the pause menu's confirmed Quit reaches `ReturningToFrontend`.
fn gameplay_interruption(
    return_to_frontend_pending: bool,
    events: &[GameEvent],
) -> Option<GameplayInterruption> {
    if events.iter().any(|event| matches!(event, GameEvent::Quit)) {
        Some(GameplayInterruption::QuitApplication)
    } else if return_to_frontend_pending {
        Some(GameplayInterruption::ReturnToFrontend)
    } else if events
        .iter()
        .any(|event| matches!(event, GameEvent::KeyDown(Keycode::Escape)))
    {
        Some(GameplayInterruption::Pause)
    } else {
        None
    }
}

#[derive(Parser)]
#[command(name = "v2k-game", about = "V2000 game runtime and tools")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    // ── Default run-game options (used when no subcommand is given) ──
    /// Asset root for direct diagnostic runs. Normal launcher Play requires
    /// PRELOAD.DAT and Overlay beside this executable. Saves stay beside the
    /// executable even when a diagnostic run uses another asset root.
    #[arg(long)]
    data_dir: Option<PathBuf>,

    /// Open the pre-game launcher even when automatic launch is enabled.
    #[arg(long, conflicts_with = "no_launcher")]
    launcher: bool,

    /// Start directly with validated game data (also used by debug runs).
    #[arg(long)]
    no_launcher: bool,

    /// Override the launcher preference file for a portable/test session.
    #[arg(long, value_name = "PATH")]
    launcher_state: Option<PathBuf>,

    #[arg(long, hide = true)]
    launcher_smoke_ms: Option<u64>,

    /// Global gameplay overlay. When passed, skip the AVI and menu and load
    /// this overlay immediately (debug). First world (Level 1) is 13. Later
    /// IDs are other campaign worlds, not a 1-based mission index; they do
    /// not inject the persistent player. Omit this flag for the normal
    /// intro/menu path; New Game still enters overlay 13.
    #[arg(long)]
    level: Option<u32>,

    /// Renderer backend: auto, opengl, software
    #[arg(long, default_value = "auto")]
    renderer: String,

    /// Skip the intro video and go straight to the menu
    #[arg(long)]
    skip_intro: bool,

    /// Enable verbose logging to stderr
    #[arg(long, short)]
    verbose: bool,

    /// Write ordinary VTOL controller/force/environment stages as JSONL.
    /// This is a matched-run RE diagnostic; it never changes simulation.
    #[arg(long, value_name = "PATH")]
    vtol_trace: Option<PathBuf>,

    /// Controlled depth policy for ground-surface overlays (RENDER_PIPELINE.md).
    /// Presentation-only. Omitted uses retail painter analogue (`terrain-recede`).
    #[arg(long, value_enum)]
    overlay_depth_policy: Option<OverlayDepthPolicyArg>,
}

/// CLI spelling of [`v2k_render::OverlayDepthPolicy`].
#[derive(Copy, Clone, Debug, ValueEnum)]
enum OverlayDepthPolicyArg {
    /// Previous port workaround: overlay-side LEQUAL plus a small polygon offset.
    DecalOffset,
    /// Recede the opaque terrain fill so later overlays win ties (retail painter).
    TerrainRecede,
    /// Diagnosis only: terrain-conformant overlay faces skip the depth test.
    OverlayAlways,
}

impl OverlayDepthPolicyArg {
    const fn to_policy(self) -> v2k_render::OverlayDepthPolicy {
        match self {
            Self::DecalOffset => v2k_render::OverlayDepthPolicy::DecalOffset,
            Self::TerrainRecede => v2k_render::OverlayDepthPolicy::TerrainRecede,
            Self::OverlayAlways => v2k_render::OverlayDepthPolicy::OverlayAlways,
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// Verify all retail assets and any installation checksum manifest
    Verify {
        #[arg(default_value = ".")]
        data_dir: PathBuf,
    },
    /// Install the port and retail assets from a BIN/CUE or ISO image
    Install {
        #[arg(long)]
        image: PathBuf,
        #[arg(long)]
        destination: PathBuf,
        /// Copy data without extracting CD music
        #[arg(long)]
        no_music: bool,
        /// Install data only, without copying this executable
        #[arg(long)]
        assets_only: bool,
    },
    /// Extract numbered lossless CD music into an installation's cdaudio folder
    ExtractMusic {
        #[arg(long)]
        image: PathBuf,
        #[arg(long)]
        data_dir: PathBuf,
    },
    /// Load and inspect a single OVL file
    Ovl {
        /// Path to a V2000 OVL file
        path: PathBuf,
    },
    /// Initialize full game session from a V2000 data directory
    Init {
        /// Path to the V2000 data directory (containing PRELOAD.DAT and Overlay/)
        #[arg(default_value = ".")]
        data_dir: PathBuf,

        /// Optionally load a specific level by ID
        #[arg(long)]
        level: Option<u32>,

        /// Common system resource/display tier: 0=320x240 low, 1=640x480
        /// high, 2=800x600, 3=1024x768
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(0..=3))]
        variant: u32,
    },
}

/// One port-side counterpart to the retail VTOL runtime timeline. The nested
/// callback stages come directly from `EntityManager::update`; the front end
/// adds only controls, configured settings, post-control attitude, and the
/// final player words needed to align adjacent frames.
#[derive(serde::Serialize)]
struct PortVtolTraceRecord<'a> {
    record_type: &'static str,
    schema_version: u8,
    sequence: u64,
    level_id: Option<u32>,
    sensitivity: u8,
    self_righting: u8,
    input_up: bool,
    input_down: bool,
    input_space: bool,
    input_right_shift: bool,
    input_s: bool,
    input_x: bool,
    pitch_channel_raw: i16,
    throttle_q16: i32,
    positive_thrust_binding_active: bool,
    fine_pitch_active: bool,
    body_pitch_after_control_raw: i16,
    body_roll_after_control_raw: i16,
    final_position_raw: [i16; 3],
    final_velocity_raw: [i16; 3],
    terrain_contact_normal_q12: Option<[i16; 3]>,
    terrain_contact_penetration_raw: Option<i32>,
    terrain_collision_impact_raw: Option<i32>,
    terrain_collision_damage_raw: Option<i32>,
    terrain_health_lost_raw: Option<i32>,
    hull_health_after_contact_raw: i32,
    hull_dying_after_contact: bool,
    #[serde(flatten)]
    callback: &'a PlayerVtolFrameDiagnostics,
}

struct PortVtolTraceContext<'a> {
    level_id: Option<u32>,
    sensitivity: u8,
    self_righting: u8,
    keys: &'a HashSet<Keycode>,
    channels: MotionChannels,
    craft: &'a PlayerCraft,
    player: &'a v2k_game::entity::Entity,
    terrain_contact: Option<&'a v2k_game::terrain_contact::PlayerTerrainContactOutcome>,
    hull: &'a PlayerHull,
    callback: &'a PlayerVtolFrameDiagnostics,
}

fn append_port_vtol_trace(
    writer: &mut Option<LineWriter<File>>,
    sequence: &mut u64,
    context: PortVtolTraceContext<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(writer) = writer else {
        return Ok(());
    };
    let [body_pitch_after_control_raw, body_roll_after_control_raw] =
        context.craft.body_angle_words();
    let record = PortVtolTraceRecord {
        record_type: "port_vtol_frame",
        schema_version: 2,
        sequence: *sequence,
        level_id: context.level_id,
        sensitivity: context.sensitivity,
        self_righting: context.self_righting,
        input_up: context.keys.contains(&Keycode::Up),
        input_down: context.keys.contains(&Keycode::Down),
        input_space: context.keys.contains(&Keycode::Space),
        input_right_shift: context.keys.contains(&Keycode::RShift),
        input_s: context.keys.contains(&Keycode::S),
        input_x: context.keys.contains(&Keycode::X),
        pitch_channel_raw: context.channels.pitch as i16,
        throttle_q16: context.channels.throttle_q16,
        positive_thrust_binding_active: context.channels.positive_thrust_binding_active,
        fine_pitch_active: context.channels.fine_pitch_active,
        body_pitch_after_control_raw,
        body_roll_after_control_raw,
        final_position_raw: context.player.position_raw(),
        final_velocity_raw: context.player.velocity_raw(),
        terrain_contact_normal_q12: context
            .terrain_contact
            .map(|outcome| outcome.contact.normal_q12),
        terrain_contact_penetration_raw: context
            .terrain_contact
            .map(|outcome| outcome.contact.penetration_raw),
        terrain_collision_impact_raw: context
            .terrain_contact
            .map(|outcome| outcome.collision_impact_raw),
        terrain_collision_damage_raw: context
            .terrain_contact
            .map(|outcome| outcome.collision_damage_raw),
        terrain_health_lost_raw: context
            .terrain_contact
            .map(|outcome| outcome.hull_damage.health_lost_raw),
        hull_health_after_contact_raw: context.hull.health_raw,
        hull_dying_after_contact: context.hull.dying,
        callback: context.callback,
    };
    serde_json::to_writer(&mut *writer, &record)?;
    writer.write_all(b"\n")?;
    *sequence = sequence.wrapping_add(1);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    launcher::prepare_console();
    let cli = Cli::parse();

    // Set global verbose flag for the log! macro
    VERBOSE.store(cli.verbose, std::sync::atomic::Ordering::Relaxed);

    let data_dir = if cli.command.is_none() {
        let selection = launcher_startup::prepare(&cli).map_err(|error| {
            if cli.launcher
                || cli.launcher_smoke_ms.is_some()
                || (!cli.no_launcher && cli.level.is_none() && cli.vtol_trace.is_none())
            {
                launcher::show_error(&error.to_string());
            }
            error
        })?;
        let Some(root) = selection else {
            return Ok(());
        };
        root
    } else {
        cli.data_dir.clone().unwrap_or_else(|| PathBuf::from("."))
    };

    match cli.command {
        Some(Command::Verify { data_dir }) => {
            let report = setup::validate_installation(&data_dir);
            println!("{}", serde_json::to_string_pretty(&report)?);
            if !report.is_ready() {
                return Err("V2000 installation verification failed".into());
            }
        }
        Some(Command::Install {
            image,
            destination,
            no_music,
            assets_only,
        }) => {
            let disc = setup::DiscImage::open(&image).map_err(std::io::Error::other)?;
            let result = disc
                .install(
                    &setup::InstallRequest {
                        destination,
                        executable: if assets_only {
                            None
                        } else {
                            Some(std::env::current_exe()?)
                        },
                        runtime_dependencies: Vec::new(),
                        extract_music: !no_music,
                    },
                    |progress| {
                        eprintln!(
                            "{} ({}/{})",
                            progress.message, progress.completed, progress.total
                        )
                    },
                )
                .map_err(std::io::Error::other)?;
            println!("Installed V2000 to {}", result.root.display());
            for warning in result.warnings {
                eprintln!("{warning}");
            }
            println!("{}", serde_json::to_string_pretty(&result.report)?);
        }
        Some(Command::ExtractMusic { image, data_dir }) => {
            let disc = setup::DiscImage::open(&image).map_err(std::io::Error::other)?;
            let result = disc
                .extract_music(&data_dir, |progress| {
                    eprintln!(
                        "{} ({}/{})",
                        progress.message, progress.completed, progress.total
                    )
                })
                .map_err(std::io::Error::other)?;
            for warning in result.warnings {
                eprintln!("{warning}");
            }
            println!("{}", serde_json::to_string_pretty(&result.report)?);
        }
        Some(Command::Ovl { path }) => {
            let start = Instant::now();
            let state = v2k_game::loader::load_level(&path)?;
            let elapsed = start.elapsed();
            print!("{}", state.summary());
            println!("\n  Loaded in {:.1}ms", elapsed.as_secs_f64() * 1000.0);
        }
        Some(Command::Init {
            data_dir,
            level,
            variant,
        }) => {
            let start = Instant::now();
            let mut session = v2k_game::session::GameSession::init(&data_dir)?;
            // Match the runtime bootstrap: common level-3 pools live on disk,
            // and visual consumers must use the selected presentation tier.
            session.load_auxiliary_ovl(3, variant)?;
            let init_elapsed = start.elapsed();

            println!("=== Game Session ===");
            println!("  Data dir:      {}", data_dir.display());
            println!("  Resource tier: {variant}");
            println!("  Preload OVLs:  {}", session.cache.preload().len());

            for (i, pre) in session.cache.preload().iter().enumerate() {
                println!("    [{}] {}/15 sections", i, pre.populated_count());
            }

            let grid_total: u32 = session.preload_grid.iter().flat_map(|row| row.iter()).sum();
            let grid_nonzero: usize = session
                .preload_grid
                .iter()
                .flat_map(|row| row.iter())
                .filter(|&&v| v > 0)
                .count();
            println!(
                "  Preload grid:  {} total ({} non-zero entries)",
                grid_total, grid_nonzero
            );
            println!(
                "  Init time:     {:.1}ms",
                init_elapsed.as_secs_f64() * 1000.0
            );

            if let Some(level_id) = level {
                println!();
                let level_start = Instant::now();
                // The diagnostic session is bound to the same explicitly
                // selected display-resource tier as its common pool.
                session.load_level_by_id(level_id, variant)?;
                let level_elapsed = level_start.elapsed();

                if let Some(lv) = session.cache.level() {
                    print!("{}", lv.summary());
                    println!(
                        "\n  Level loaded in {:.1}ms",
                        level_elapsed.as_secs_f64() * 1000.0
                    );

                    if let Some(desc) = &lv.level {
                        let type_models = session.cache.global_entity_model_table();
                        println!("\n  Section-13 entities:");
                        for spawn in &desc.entities {
                            let override_id = spawn.model_overrides[0];
                            let model_id = (override_id != 0)
                                .then_some(override_id as usize)
                                .or_else(|| {
                                    type_models
                                        .get(spawn.entity_type as usize)
                                        .map(|models| models[0] as usize)
                                        .filter(|&id| id != 0)
                                });
                            let model_name = model_id
                                .and_then(|id| session.cache.global_model(id))
                                .and_then(|model| model.name.as_deref())
                                .unwrap_or("<none>");
                            println!(
                                "    {:>2}: type {:>3}, model {:>3?} {}",
                                spawn.index, spawn.entity_type, model_id, model_name
                            );
                        }
                    }
                }
            }

            println!();
            print!("{}", session.cache.status());
            println!();
            print!("{}", session.system_coverage_report());
        }
        // No subcommand → run the game
        None => {
            let enter_world_immediately = cli.level.is_some();
            let outcome = run_game(
                &data_dir,
                cli.level.unwrap_or(FIRST_WORLD_LEVEL_ID),
                &cli.renderer,
                cli.skip_intro || enter_world_immediately,
                enter_world_immediately,
                cli.vtol_trace.as_deref(),
                cli.overlay_depth_policy
                    .unwrap_or(OverlayDepthPolicyArg::TerrainRecede)
                    .to_policy(),
            );
            if let Err(error) = outcome {
                if cli.launcher
                    || (!cli.no_launcher && cli.level.is_none() && cli.vtol_trace.is_none())
                {
                    launcher::show_error(&error.to_string());
                }
                return Err(error);
            }
        }
    }

    Ok(())
}

struct LoadedMenuAssets {
    resources: v2k_game::menu::MenuResources,
    fonts: Option<v2k_game::menu_text::MenuFonts>,
    reference_size: (u32, u32),
}

/// Load the mutually dependent menu atlas, sprite fonts, and authored virtual
/// size as one operation. Keeping variant selection here prevents the main
/// runtime loop from accumulating asset-tier fallback policy.
fn load_menu_assets(
    session: &mut v2k_game::session::GameSession,
    detail: GraphicsDetail,
    variant: u32,
    fallback_size: (u32, u32),
) -> LoadedMenuAssets {
    use v2k_game::menu::MenuResources;

    // PRELOAD embeds the authored 320x240 variants of system levels 2 and 5.
    // Retail overlays both with the active display-mode digit. Keep the
    // selected packs in the shared cache: menu prop models resolve their face
    // sprites through the global pools, so merely decoding high-resolution
    // logos from a temporary LevelState leaves props such as `slopt` bound to
    // PRELOAD's 32x32 artwork instead of the authored 64x64 sprites.
    let high_fonts_loaded = variant >= 1 && session.load_auxiliary_ovl(2, variant).is_ok();
    let high_graphics_loaded = variant >= 1 && session.load_auxiliary_ovl(5, variant).is_ok();

    let graphics = session.cache.menu_graphics_ovl();
    let logo_scale = if high_graphics_loaded { 1 } else { 3 };
    let resources = if let Some(graphics) = graphics {
        let mut resources = MenuResources::from_menu_ovl(graphics);
        resources.logo_scale = logo_scale;
        log!(
            "Menu resources (detail={}, {}): {} trophy icons, copyright={}, logos={}/{}",
            detail.label(),
            if logo_scale == 1 {
                "on-disk high tier"
            } else {
                "embedded 0X5XX"
            },
            resources.trophy_icons.len(),
            resources.copyright_banner.is_some(),
            resources.frontier_logo.is_some(),
            resources.publisher_logo.is_some()
        );
        resources
    } else {
        log!("Warning: No menu graphics OVL found");
        MenuResources::empty()
    };

    // The active level-2 layer owns both sprite fonts and their authored layout
    // coordinates. If its high tier is absent, PRELOAD remains the complete
    // low-resolution fallback.
    let fonts = v2k_game::menu_text::MenuFonts::from_cache(&session.cache);
    let reference_size = fonts
        .as_ref()
        .map(|fonts| (fonts.virtual_w as u32, fonts.virtual_h as u32))
        .unwrap_or(fallback_size);
    match &fonts {
        Some(fonts) => log!(
            "Menu fonts: green/yellow sprite fonts loaded ({}x{} asset space, {})",
            fonts.virtual_w,
            fonts.virtual_h,
            if high_fonts_loaded {
                "on-disk high tier"
            } else {
                "PRELOAD fallback"
            }
        ),
        None => log!("Menu fonts: Section 4 unavailable — falling back to built-in rasterizer"),
    }

    LoadedMenuAssets {
        resources,
        fonts,
        reference_size,
    }
}

fn run_game(
    data_dir: &Path,
    level_id: u32,
    renderer_choice: &str,
    skip_intro: bool,
    enter_world_immediately: bool,
    vtol_trace_path: Option<&Path>,
    overlay_depth_policy: v2k_render::OverlayDepthPolicy,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load/create config
    let mut diagnostic_console = diagnostic_console::DiagnosticConsole::new();
    // `V2K_NEW_GAME_AFTER_TICKS`: headless diagnostics confirm the frontend
    // ring's default New Game after this many frontend ticks, so a capture
    // takes the production Begin-Intro path rather than the generic debug
    // load of `--level 50`.
    let mut auto_new_game_tick = std::env::var("V2K_NEW_GAME_AFTER_TICKS")
        .ok()
        .and_then(|value| value.parse::<u32>().ok());
    let mut config = GameConfig::load(data_dir);
    log!(
        "Controls: {} => Sensitivity {}/15, Self Righting {}",
        data_dir.join("config.json").display(),
        (config.sensitivity * 15.0).round().clamp(0.0, 15.0) as u8,
        config.self_righting
    );

    // Determine backend
    let cli_override = match renderer_choice {
        "auto" => None,
        other => Some(other),
    };
    let preferred = config.resolve_backend(cli_override);

    // Initialize SDL2 and renderer
    let mut game_window = GameWindow::new()?;
    let mut debug_panel = DebugPanel::new();
    let mut vtol_trace = match vtol_trace_path {
        Some(path) => {
            if let Some(parent) = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
            {
                std::fs::create_dir_all(parent)?;
            }
            Some(LineWriter::new(File::create(path)?))
        }
        None => None,
    };
    let mut vtol_trace_sequence = 0u64;

    let (mut renderer, mut actual_backend) = v2k_render::create_renderer(
        &game_window,
        "V2K",
        config.width,
        config.height,
        preferred,
        // Retail's startup search (`FUN_0044E2E0` -> `FUN_0042D340`) tries
        // each resolution and window mode, then the other renderer, and keeps
        // the first combination that starts. Only the backend can fail to
        // start here, so without a command-line choice OpenGL falls back to
        // the software renderer.
        cli_override.is_none(),
    )?;

    let initial_menu_size = config.detail.menu_virtual_size();
    renderer.set_scaling_mode(config.scaling, initial_menu_size.0, initial_menu_size.1);
    renderer.set_overlay_depth_policy(overlay_depth_policy);
    if overlay_depth_policy == v2k_render::OverlayDepthPolicy::OverlayAlways {
        log!("Overlay depth diagnosis policy active: {overlay_depth_policy:?} (presentation only)");
    }
    apply_classic_framebuffer_presentation(renderer.as_mut(), &mut config, actual_backend);
    if config.fullscreen {
        renderer.set_fullscreen(true);
    }

    log!("Renderer: {}", renderer.backend_name());

    // Keep the backend that started, as the retail search leaves the
    // working combination in the settings.
    if config.renderer == RendererChoice::Auto || actual_backend != preferred {
        config.set_detected_backend(actual_backend);
        if let Err(error) = config.try_save(data_dir) {
            eprintln!("Could not save settings: {error}");
        }
    }

    // --- Persistent game session (lives for the entire run) ---
    let mut session = v2k_game::session::GameSession::init(data_dir)?;

    // System level 3 is not embedded in PRELOAD.DAT. Retail reloads it with
    // the active display-mode digit, so its common pools and HUD artwork must
    // come from the same low/high tier as the frontend assets. High tiers keep
    // intrinsic pools resident and can replace their layout tables live.
    // Keep the labelled variant-0 fallback for incomplete installations.
    let requested_common_variant = config.system_graphics_variant();
    let mut gameplay_hud_variant = match session.load_auxiliary_ovl(3, requested_common_variant) {
        Ok(()) => {
            log!(
                "Global pools: {} strings from {}X3XX.OVL (system levels 2-5)",
                session.cache.global_strings().len(),
                requested_common_variant
            );
            Some(requested_common_variant)
        }
        Err(high_error) if requested_common_variant != 0 => {
            log!(
                "Warning: {}X3XX.OVL not loaded ({high_error}); trying low-resolution 0X3XX.OVL",
                requested_common_variant
            );
            match session.load_auxiliary_ovl(3, 0) {
                Ok(()) => Some(0),
                Err(low_error) => {
                    log!(
                        "Warning: 0X3XX.OVL not loaded ({low_error}) — menu falls back to embedded labels"
                    );
                    None
                }
            }
        }
        Err(error) => {
            log!("Warning: 0X3XX.OVL not loaded ({error}) — menu falls back to embedded labels");
            None
        }
    };

    // Bind world and world-style resources to the tier which actually won the
    // startup common-pool load. A missing high tier therefore keeps the
    // existing complete fallback to variant 0. Live high-tier layout changes
    // also update the source selected for subsequent world loads.
    let mut startup_world_variant = gameplay_hud_variant.unwrap_or(0);

    // FUN_004292B0's persistent left status compositor uses the selected
    // level-3 variant's Section-1 points and global sprites 499..506. Keep
    // decoded assets resident across world loads just like the common OVL.
    let gameplay_hud_resources =
        gameplay_hud_variant.and_then(|_| GameplayHudResources::from_cache(&session.cache));
    let mut gameplay_hud_layout = gameplay_hud_variant
        .and_then(|variant| GameplayHudLayout::from_cache(&session.cache, variant));
    if let Some(variant) = gameplay_hud_variant {
        if let Err(error) = session.load_auxiliary_ovl(51, variant) {
            log!("Warning: {}X51XX.OVL not loaded ({error})", variant);
        }
    }
    let overlay_51_backdrop =
        gameplay_hud_variant.and_then(|_| Overlay51Backdrop::from_cache(&session.cache));
    // FUN_00453FE0's map-side labels use this same selected level-3 tier's
    // Section-0/1 layout values and cumulative Section-2 strings.
    let mut fullscreen_map_status =
        gameplay_hud_variant.and_then(|_| FullscreenMapStatusResources::from_cache(&session.cache));

    let menu_assets = load_menu_assets(
        &mut session,
        config.detail,
        startup_world_variant,
        initial_menu_size,
    );
    let menu_resources = menu_assets.resources;
    let mut menu_fonts = menu_assets.fonts;
    let mut menu_reference_size = menu_assets.reference_size;
    let mut world_projection = WorldProjection::from_cache(&session.cache)
        .ok_or("System level 2 has no valid authored world projection")?;

    // Extract strings only after binding the selected system level-2 tier.
    let system_strings: Vec<String> = session
        .cache
        .menu_ovl()
        .map(|ovl| ovl.strings.clone())
        .unwrap_or_default();
    if !system_strings.is_empty() {
        log!(
            "System strings: {} entries from menu OVL",
            system_strings.len()
        );
    }
    renderer.set_scaling_mode(config.scaling, menu_reference_size.0, menu_reference_size.1);
    renderer.set_ui_submission_policy(live_display::ui_policy(&config, gameplay_hud_variant));
    apply_classic_framebuffer_presentation(renderer.as_mut(), &mut config, actual_backend);

    // --- Background flame billboard animation (table 0x4CA938) ---
    // .data initial state (frame 8, accumulator 2,000,000 µs) wraps on the
    // first frame → the rumble fires once immediately at menu entry.
    let mut billboard = BillboardAnim::new();

    // --- Per-face model colours (mat operand → Section-7 palette / sprite
    // colour), memoised per model id and reused every frame. ---
    let face_colors = v2k_game::model_color::ModelMaterialCache::new();

    // --- Save manager ---
    let retail_save_directory = v2k_render::config::retail_save_directory();
    let executable = std::env::current_exe()?;
    let save_paths = SavePathContext::beside_executable(
        &executable,
        data_dir,
        retail_save_directory.as_deref(),
    )?;
    log!("Save directory: {}", save_paths.save_directory().display());
    let mut save_manager = SaveManager::load_paths(save_paths);
    // 448E30 owns a retained controller checkpoint, not a live-world dump.
    let mut save_baseline = [0u8; v2k_formats::saves::STATE_PAYLOAD_SIZE];
    let mut map_save_checkpoint: Option<v2k_game::save::NativeCompatibilityPreview> = None;

    // --- Intro state setup ---
    let mut avi_player: Option<AviPlayer> = None;
    let mut audio_player: Option<AudioPlayer> = None;
    let mut intro_start_time: Option<Instant> = None;

    macro_rules! make_menu {
        () => {{
            let ctx = MenuCtx {
                cache: &session.cache,
                config: &config,
                saves: Some(&save_manager),
            };
            // Cold entry (after the AVI) runs the original's 3.0 s
            // scene-only intro before the ring flies in.
            MenuShell::new_frontend(&ctx, true)
        }};
        // Returning from gameplay skips the intro (fly-in only).
        ($intro:expr) => {{
            let ctx = MenuCtx {
                cache: &session.cache,
                config: &config,
                saves: Some(&save_manager),
            };
            MenuShell::new_frontend(&ctx, $intro)
        }};
    }

    let mut state = if enter_world_immediately {
        eprintln!("Debug boot: loading overlay {level_id}, skipping intro and menu");
        GameState::Loading {
            level_id,
            purpose: LoadingPurpose::Normal,
        }
    } else if skip_intro {
        GameState::Menu(make_menu!())
    } else {
        // BANNERHI/BANNERLO are the complete Grolier -> Frontier -> V2000
        // presentation. Development data keeps them one directory above
        // port/, while installed builds may place them beside PRELOAD.DAT.
        let avi_path = find_banner_avi(data_dir, config.detail);
        if avi_path.exists() {
            match std::fs::read(&avi_path) {
                Ok(avi_data) => match AviPlayer::open(&avi_data) {
                    Ok(player) => {
                        log!(
                            "Intro: {}x{}, {:.1}fps, {} frames, audio: {}Hz {}ch {}bit",
                            player.info.width,
                            player.info.height,
                            player.info.fps,
                            player.frame_count(),
                            player.info.audio_rate,
                            player.info.audio_channels,
                            player.info.audio_bits
                        );

                        // Open the device paused. Playback starts with the first
                        // presented video frame, after the remaining startup work.
                        if !player.audio_pcm().is_empty() {
                            match AudioPlayer::new(
                                &game_window.sdl,
                                player.info.audio_rate,
                                player.info.audio_channels as u8,
                                player.info.audio_bits,
                            ) {
                                Ok(ap) => {
                                    audio_player = Some(ap);
                                }
                                Err(e) => eprintln!("Audio init failed: {}", e),
                            }
                        }

                        avi_player = Some(player);
                        GameState::Intro
                    }
                    Err(e) => {
                        log!("Failed to parse AVI: {} — skipping intro", e);
                        GameState::Menu(make_menu!())
                    }
                },
                Err(e) => {
                    log!(
                        "Failed to read {}: {} — skipping intro",
                        avi_path.display(),
                        e
                    );
                    GameState::Menu(make_menu!())
                }
            }
        } else {
            log!("No BANNERHI.AVI found — skipping intro");
            GameState::Menu(make_menu!())
        }
    };

    // --- Sound manager: GLOBAL Section-11 pool (system L2 ids 0-6 + L3 ids
    // 7-109, mirroring DAT_004FE64C). One pool serves menu and gameplay; the
    // menu ids 0-3, prop whoosh 0x57 and billboard rumble 0x39 all resolve
    // here, with type-5 alias rate/volume/variance honored at play time.
    let mut sound_manager: Option<SoundManager> = None;
    {
        let tables = session.cache.global_sound_tables();
        if tables.is_empty() {
            eprintln!("Sound init: no Section 11 tables in system OVLs");
        } else {
            match SoundManager::from_tables(&game_window.sdl, &tables, configured_sfx_gain(&config))
            {
                Ok(sm) => {
                    if sm.sound_count() < 110 {
                        log!(
                            "Sound: global pool only {} slots (selected ?X3XX.OVL missing?) — whoosh/rumble unavailable",
                            sm.sound_count()
                        );
                    } else {
                        log!("Sound: global pool loaded ({} slots)", sm.sound_count());
                    }
                    sound_manager = Some(sm);
                }
                Err(e) => eprintln!("Sound init failed: {}", e),
            }
        }
    }

    // --- Music player for CD audio tracks ---
    let mut music_player: Option<MusicPlayer> = None;
    if let Some(music_dir) = find_cd_audio_dir(data_dir) {
        match MusicPlayer::new(&game_window.sdl, &music_dir) {
            Ok(mp) => {
                log!(
                    "Music: {} OGG tracks found in {}",
                    mp.track_count(),
                    music_dir.display()
                );
                music_player = Some(mp);
            }
            Err(e) => eprintln!("Music init failed: {}", e),
        }
    }

    // --- Playing state data (initialized lazily on Loading->Playing transition) ---
    let mut entity_manager: Option<v2k_game::entity::EntityManager> = None;
    let mut specialized_actor_tasks = SpecializedActorTaskScheduler::new();
    let mut specialized_actor_task_error_reported = false;
    let mut particle_collision_error_reported = HashSet::<u32>::new();
    let mut type9_task_diagnostics = type9_task_diagnostics::Type9TaskDiagnostics::default();
    let mut current_level_id: Option<u32> = None;
    // Session+38 retains entry XYZ across movement.456D10 reuses these words
    // for failed-world return and forces fresh velocity/attitude/heading.
    let mut current_world_entry_position_raw: Option<[i16; 3]> = None;
    let mut pending_failed_world_interior_request = false;
    let mut main_base_level_abort = MainBaseAbortBridgeState::default();
    let mut next_world_control_load_generation = NonZeroU64::new(1).unwrap();
    // Per-level shoreline frame set (GPU texture); rebuilt on level load.
    let mut water_frames: Option<v2k_render::WaterFrames> = None;
    // Per-level opaque terrain transition frames (120 canonical sprites).
    let mut terrain_frames: Option<v2k_render::TerrainFrames> = None;
    // Original scrolling 32×32 signed terrain-light field.
    let mut terrain_lights = v2k_render::TerrainLightWindow::default();
    // Section 13 +0x54 master-palette background fill.
    let mut sky_background: Option<v2k_game::sky::SkyBackground> = None;
    let (render_w, render_h) = renderer.viewport_size();
    let mut camera = Camera::new(render_w as f32 / render_h.max(1) as f32);
    world_projection.apply_to(&mut camera, (render_w, render_h));
    let mut chase_camera = ChaseCameraState::default();
    let mut intro_camera = IntroCameraController::default();
    let mut post_intro_world_cover: Option<MenuShell> = None;
    let mut intro_commands = v2k_game::intro2_commands::Intro2Commands::default();
    // DAT_004F72E8 belongs to the common authored-text renderer. Intro2 and
    // gameplay notifications share this one process-lifetime owner, including
    // across level-clock resets and an early cinematic skip.
    let mut text_typewriter_cadence = TextTypewriterCadence::default();
    // Shared gameplay/cinematic effects. Intro2's meteors are the first
    // end-to-end users; later projectile and destruction callbacks enqueue
    // through the same backend-neutral system.
    let mut world_fx = WorldFx::new();
    let mut gameplay_hud = GameplayHud::default();
    // FUN_0044AB20's one per-level north-up terrain raster backs both the
    // right HUD globe and the modal M map.
    let mut gameplay_radar: Option<GameplayRadar> = None;
    // DAT_004DB1B8 advances only through active FUN_0044FFA0 gameplay
    // updates, using the same 125-ms-capped delta as vehicle simulation.
    let mut gameplay_hud_elapsed_micros = 0u32;
    let mut gameplay_notifications = GameplayNotifications::new();
    let mut world_complete_results = WorldCompleteResultsRuntime::default();
    let mut world_complete_tally = WorldCompleteTally::default();
    let mut keys_down: HashSet<Keycode> = HashSet::new();
    let mut mouse_buttons_down: HashSet<MouseButton> = HashSet::new();
    let mut chase_mode = false;
    let mut has_player = false;
    // Player vehicle control state (V2000 scheme, see v2k_game::player). TAB
    // toggles the movement style; VTOL burns fuel and an empty tank forces
    // Hover. F11 toggles the dev free-fly camera; Backquote owns the console.
    let mut player_craft = PlayerCraft::new();
    let mut player_fan_audio = PlayerFanAudio::default();
    // FUN_004104B0's entity +0x8C logical sound custody. Type 61 owns the
    // fixed sound-44 loop; types 15/44/87/108 mix sound 11 from the
    // FUN_0044C920 pre-integration snapshot.
    let mut entity_positional_audio = EntityPositionalAudio::default();
    // Kinds 0x16..=0x1A materialize generic trigger entities in retail, but
    // campaign routing remains a separate world-controller concern. This
    // owner consumes authored routes from player static-contact stamps;
    // the native Type111 helpers retain their own sound-100 loop custody.
    let mut campaign_warp = CampaignWarpRuntime::default();
    let mut campaign_arrivals: Option<CampaignArrivalCatalog> = None;
    // DAT_004DE854 is process-global rather than part of a player spawn, so
    // this warning cadence deliberately survives level reloads.
    let mut fuel_warning_cadence = FuelWarningCadence::default();
    // DAT_004DE858 has the same process-global lifetime. In particular, do not
    // reset it when replacing the per-spawn PlayerHull below.
    let mut hull_warning_cadence = HullWarningCadence::default();
    let mut player_hull = PlayerHull::default();
    let mut player_shield = v2k_game::player_shield::PlayerShieldPresentation::default();
    let mut player_death = PlayerDeathLifecycle::default();
    let mut static_damage = StaticDamageScheduler::new();
    let mut return_to_frontend_pending = false;
    let mut static_contact_error_reported = false;
    let mut active_pair_contact_error_reported = false;
    let mut main_base_conversion_error_reported = false;
    let mut factory_pair_error_reported = false;
    let mut power_up_contact_error_reported = false;
    let mut primary_weapon = PrimaryWeapon::new();
    let mut pending_entity_weapon_fire = Vec::new();
    let mut native_entity_weapon_error_reported = false;
    let mut targetter = TargetterRuntime::new(0);
    let mut weapon_inventory = WeaponInventory::new();
    let mut player_capabilities = PlayerCapabilities::default();
    let mut player_campaign_progress = PlayerCampaignProgress::default();
    // Resolved player4 hull id for the persistent type-46 player entity.
    let mut player_model_id: Option<usize> = None;

    let mouse_sensitivity = 0.003_f32;
    let fly_speed = 50.0_f32;

    let target_frame_time = Duration::from_millis(16);
    let mut last_frame = Instant::now();
    // Exact process-global 50-Hz clock (`DAT_004FED60`). Active engine states
    // advance it once per frame; loading, the AVI and gameplay pause freeze it.
    let mut retail_clock = RetailTickClock::new();
    // Reuse the frozen raster on ordinary pause frames. Display changes redraw
    // isolated world owners and retained overlays, then replace that raster.
    let mut paused_world_frame: Option<CapturedFrame> = None;
    let mut paused_gameplay_scene: Option<PausedGameplayScene> = None;
    let mut paused_world_redraw_pending = false;
    let mut gameplay_overlay_frame = GameplayOverlayFrame::default();

    'main_loop: loop {
        let now = Instant::now();
        let frame_delta = now.duration_since(last_frame);
        let uncapped_elapsed_micros = frame_delta.as_micros().min(u128::from(u32::MAX)) as u32;
        let elapsed_micros = uncapped_elapsed_micros.min(RETAIL_FRAME_DELTA_MAX_US);
        let dt = elapsed_micros as f32 / 1_000_000.0;
        let mut retail_tick = retail_clock.current();
        last_frame = now;

        // Process events — behavior depends on game state
        let mut events = Vec::new();
        for event in game_window.poll_events() {
            match event {
                // Backquote/tilde owns the text console in every state and
                // must not skip the intro, activate a menu or move the camera.
                GameEvent::KeyDown(Keycode::Backquote) => match diagnostic_console.toggle() {
                    Ok(true) => game_window.release_mouse_capture(),
                    Ok(false) => {}
                    Err(error) => eprintln!("Console visibility error: {error}"),
                },
                GameEvent::KeyUp(Keycode::Backquote) => {}
                // F12 belongs to the diagnostics layer in every game state;
                // consuming it here prevents it from skipping the intro or
                // reaching menu/game controls.
                GameEvent::KeyDown(Keycode::F12) => {
                    if let Err(error) = debug_panel.toggle(&game_window.video) {
                        eprintln!("Debug panel error: {error}");
                    }
                    if debug_panel.is_visible() {
                        game_window.release_mouse_capture();
                    }
                }
                GameEvent::AuxiliaryWindowClick { window_id, x, y }
                    if debug_panel.owns_window(window_id) =>
                {
                    if let Err(error) = debug_panel.handle_click(x, y, &game_window.video) {
                        eprintln!("Debug clipboard error: {error}");
                    }
                }
                GameEvent::WindowClosed(window_id) if debug_panel.owns_window(window_id) => {
                    debug_panel.hide();
                }
                GameEvent::WindowClosed(_) => events.push(GameEvent::Quit),
                other => events.push(other),
            }
        }

        let menu_is_paused = matches!(&state, GameState::Paused { .. });
        match &mut state {
            GameState::Intro => {
                // Any key/click skips the intro
                let mut skip = false;
                for event in &events {
                    match event {
                        GameEvent::Quit => break 'main_loop,
                        GameEvent::Resize(w, h) => renderer.resize(*w, *h),
                        // The Shift edge can arrive one frame before tilde.
                        // Leave modifier-only input inert so opening the
                        // diagnostic console cannot skip the video first.
                        GameEvent::KeyDown(Keycode::LShift | Keycode::RShift) => {}
                        GameEvent::KeyDown(_) | GameEvent::MouseButtonDown(_) => skip = true,
                        _ => {}
                    }
                }

                if let Some(ref mut player) = avi_player {
                    let elapsed = intro_start_time
                        .map(|start| start.elapsed().as_secs_f64())
                        .unwrap_or(0.0);
                    let frame_duration = 1.0 / f64::from(player.info.fps);
                    let frame_idx = (elapsed / frame_duration) as usize;

                    if frame_idx >= player.frame_count() || skip {
                        // Intro finished or skipped
                        if let Some(ref ap) = audio_player {
                            ap.stop();
                        }
                        avi_player = None;
                        audio_player = None;
                        state = GameState::Menu(make_menu!());
                    } else {
                        // Decode every dependency through the desired frame;
                        // a slow presentation must not skip delta-coded data.
                        let w = player.info.width;
                        let h = player.info.height;
                        let rgba = player.decode_frame(frame_idx);

                        renderer.clear(0.0, 0.0, 0.0);
                        renderer.draw_fullscreen(rgba, w, h);
                        renderer.present();
                        if intro_start_time.is_none() {
                            if let Some(ref ap) = audio_player {
                                ap.play(player.audio_pcm());
                            }
                            intro_start_time = Some(Instant::now());
                        }
                    }
                } else {
                    // No AVI loaded, skip to menu
                    state = GameState::Menu(make_menu!());
                }
            }

            GameState::Menu(shell) | GameState::Paused { shell } => {
                let mut redraw_paused_world =
                    std::mem::take(&mut paused_world_redraw_pending) && menu_is_paused;
                if !menu_is_paused {
                    retail_tick = retail_clock.advance(u64::from(elapsed_micros));
                }
                // The original frontend menu is SILENT — CD music exists only
                // in-game. A pause is still part of the live gameplay session,
                // so it must not stop or replace that session's audio state.
                if !menu_is_paused {
                    if let Some(ref mut mp) = music_player {
                        if mp.current_cd_track().is_some() {
                            mp.stop();
                        }
                    }
                }

                // Translate key events into engine inputs; collect side effects.
                let mut shell_events: Vec<ShellEvent> = Vec::new();
                for event in &events {
                    match event {
                        GameEvent::Quit => break 'main_loop,
                        GameEvent::Resize(w, h) => {
                            renderer.resize(*w, *h);
                            world_projection.apply_to(&mut camera, renderer.viewport_size());
                            redraw_paused_world |= menu_is_paused;
                        }
                        GameEvent::KeyDown(kc) => {
                            keys_down.insert(*kc);
                            let shift = keys_down.contains(&Keycode::LShift)
                                || keys_down.contains(&Keycode::RShift);
                            let alt = keys_down.contains(&Keycode::LAlt)
                                || keys_down.contains(&Keycode::RAlt);

                            // Mode-level key combos (original key slots at
                            // 0x4C05A0 fire regardless of the screen-engine tick):
                            match *kc {
                                // Shift+Esc = instant quit to desktop (FUN_0042BAA0).
                                Keycode::Escape if shift => break 'main_loop,
                                // Alt+Enter = toggle fullscreen (FUN_0042D5D0).
                                Keycode::Return if alt => {
                                    config.fullscreen = !config.fullscreen;
                                    renderer.set_fullscreen(config.fullscreen);
                                    world_projection
                                        .apply_to(&mut camera, renderer.viewport_size());
                                    redraw_paused_world |= menu_is_paused;
                                    if let Err(error) = config.try_save(data_dir) {
                                        eprintln!("Could not save settings: {error}");
                                    }
                                    continue;
                                }
                                _ => {}
                            }

                            // Input is ignored while a transition or the intro
                            // is running (the original's screen-engine tick
                            // early-outs). This also prevents a double-tapped
                            // Esc during the ring fly-in — when depth is
                            // already 1 but the ring isn't interactive yet —
                            // from quitting the whole game.
                            if shell.is_transitioning() {
                                continue;
                            }

                            if !menu_is_paused {
                                // Direct save-slot quick-load (keys 1-3 → slots
                                // 0-2; the original's keys 1-6 arm DAT_004DB228 for
                                // a direct-slot game start). Empty slot = no-op blip.
                                let quick_slot = match *kc {
                                    Keycode::Num1 | Keycode::Kp1 => Some(0),
                                    Keycode::Num2 | Keycode::Kp2 => Some(1),
                                    Keycode::Num3 | Keycode::Kp3 => Some(2),
                                    _ => None,
                                };
                                if let Some(s) = quick_slot {
                                    if save_manager.slot(s).is_some() {
                                        shell_events.push(ShellEvent::Sound(3));
                                        shell_events.push(ShellEvent::LoadSlot(s));
                                    } else {
                                        shell_events.push(ShellEvent::Sound(0));
                                    }
                                    continue;
                                }
                            }

                            let input = match *kc {
                                // FUN_0042BAD0 cannot pop the depth-1 root.
                                // Retail therefore leaves plain Esc inert here;
                                // Shift+Esc and the Exit prop remain the two
                                // desktop-quit paths.
                                Keycode::Escape if shell.back_is_inert() => None,
                                Keycode::Escape => {
                                    // Leaving a settings screen — persist config.
                                    if let Err(error) = config.try_save(data_dir) {
                                        eprintln!("Could not save settings: {error}");
                                    }
                                    Some(MenuInput::Back)
                                }
                                Keycode::Up => Some(MenuInput::Up),
                                Keycode::Down => Some(MenuInput::Down),
                                Keycode::Left => Some(MenuInput::Left),
                                Keycode::Right => Some(MenuInput::Right),
                                Keycode::Return | Keycode::Space => Some(MenuInput::Select),
                                _ => None,
                            };
                            if let Some(input) = input {
                                let ctx = MenuCtx {
                                    cache: &session.cache,
                                    config: &config,
                                    saves: Some(&save_manager),
                                };
                                shell_events.extend(shell.input(input, &ctx));
                            }
                        }
                        GameEvent::KeyUp(kc) => {
                            keys_down.remove(kc);
                        }
                        _ => {}
                    }
                }

                if let Some(tick) = auto_new_game_tick {
                    if !menu_is_paused && retail_tick >= tick && !shell.is_transitioning() {
                        auto_new_game_tick = None;
                        let ctx = MenuCtx {
                            cache: &session.cache,
                            config: &config,
                            saves: Some(&save_manager),
                        };
                        shell_events.extend(shell.input(MenuInput::Select, &ctx));
                    }
                }

                // Apply side effects from the menu engine.
                let mut next_state: Option<GameState> = None;
                for ev in shell_events {
                    match ev {
                        ShellEvent::Sound(id) => {
                            GlobalSoundRuntime::new(
                                &session.cache,
                                &mut world_fx,
                                sound_manager.as_mut(),
                            )
                            .play_global_sound(id as usize);
                        }
                        ShellEvent::SettingChanged(id, v) => {
                            redraw_paused_world |=
                                menu_is_paused && setting_requires_paused_redraw(id);
                            apply_setting_to_config(&mut config, id, v);
                            apply_live_audio_setting(
                                id,
                                &config,
                                &mut sound_manager,
                                &mut music_player,
                                world_complete_results.is_progress_map_active(),
                            );
                            match id {
                                v2k_game::menu_data::SettingId::FullScreen => {
                                    renderer.set_fullscreen(config.fullscreen);
                                }
                                v2k_game::menu_data::SettingId::Resolution => {
                                    renderer.set_window_size(config.width, config.height);
                                }
                                v2k_game::menu_data::SettingId::Rendering
                                    if config.resolve_backend(None) != actual_backend =>
                                {
                                    match replace_renderer(&game_window, &mut renderer, &config) {
                                        Ok(backend) => {
                                            actual_backend = backend;
                                            renderer.set_overlay_depth_policy(overlay_depth_policy);
                                            if config.fullscreen {
                                                renderer.set_fullscreen(true);
                                            }
                                            game_window.release_mouse_capture();
                                            // Texture ids belonged to the old renderer.
                                            face_colors.forget_textures();
                                            if terrain_frames.is_some() {
                                                terrain_frames =
                                                    v2k_game::terrain_render::build_terrain_frames(
                                                        &session.cache,
                                                        renderer.as_mut(),
                                                    );
                                            }
                                            if water_frames.is_some() {
                                                water_frames = v2k_game::water::build_water_frames(
                                                    &session.cache,
                                                    renderer.as_mut(),
                                                );
                                            }
                                            log!("Renderer: {}", renderer.backend_name());
                                        }
                                        Err(error) => eprintln!(
                                            "Renderer change failed ({error}); keeping {}",
                                            renderer.backend_name()
                                        ),
                                    }
                                }
                                _ => {}
                            }
                            if matches!(
                                id,
                                SettingId::Resolution
                                    | SettingId::Scaling
                                    | SettingId::ClassicFramebuffer
                                    | SettingId::Rendering
                            ) {
                                if let Some(tier) =
                                    v2k_game::system_layout::HighSystemLayoutTier::from_variant(
                                        config.system_graphics_variant(),
                                    )
                                {
                                    match (live_display::ResidentDisplay {
                                        session: &mut session,
                                        fonts: &mut menu_fonts,
                                        variant: &mut gameplay_hud_variant,
                                        world_variant: &mut startup_world_variant,
                                        reference_size: &mut menu_reference_size,
                                        hud_layout: &mut gameplay_hud_layout,
                                        radar: &mut gameplay_radar,
                                        map_status: &mut fullscreen_map_status,
                                        overlay: &mut gameplay_overlay_frame,
                                        projection: &mut world_projection,
                                    }).refresh(tier) {
                                        Ok(true) => log!("Display layout refreshed to {}x{}", tier.size().0, tier.size().1),
                                        Ok(false) => {}
                                        Err(error) => eprintln!("Display layout change rejected; retained resident presentation: {error}"),
                                    }
                                }
                                renderer.set_scaling_mode(
                                    config.scaling,
                                    menu_reference_size.0,
                                    menu_reference_size.1,
                                );
                                renderer.set_ui_submission_policy(live_display::ui_policy(
                                    &config,
                                    gameplay_hud_variant,
                                ));
                                apply_classic_framebuffer_presentation(
                                    renderer.as_mut(),
                                    &mut config,
                                    actual_backend,
                                );
                                shell.engine.settings.set(
                                    SettingId::ClassicFramebuffer,
                                    config.classic_framebuffer_effective() as u32,
                                );
                            }
                            world_projection.apply_to(&mut camera, renderer.viewport_size());
                            if let Err(error) = config.try_save(data_dir) {
                                eprintln!("Could not save settings: {error}");
                            }
                            let ctx = MenuCtx {
                                cache: &session.cache,
                                config: &config,
                                saves: Some(&save_manager),
                            };
                            shell.refresh(&ctx);
                        }
                        // MenuShell consumes StartGame and runs the verified
                        // Klaus-mouth handoff before arming the transition.
                        ShellEvent::StartGame => {}
                        ShellEvent::QuitToDesktop => break 'main_loop,
                        ShellEvent::LoadSlot(slot) => {
                            if let Some(save) = save_manager.slot(slot) {
                                let Some(player) = save.player else {
                                    eprintln!(
                                        "Load blocked: slot {} has no supported player snapshot",
                                        slot
                                    );
                                    continue;
                                };
                                let purpose = match save.source {
                                    SaveSource::NativeCompatibilityPreview => {
                                        let Some(native) = save.native.as_ref() else {
                                            eprintln!("Load blocked: native slot {slot} lost its controller payload");
                                            continue;
                                        };
                                        let restore =
                                            match v2k_game::save::NativeSaveRestore::decode(native)
                                            {
                                                Ok(restore) => restore,
                                                Err(error) => {
                                                    eprintln!("Load blocked: native slot {slot}: {error:?}");
                                                    continue;
                                                }
                                            };
                                        save_baseline = restore.state_payload;
                                        log!(
                                            "Opening native slot {}: global level {}, controller/player/cargo restoration; retained session flags 0x{:08X}, controller +195 {}",
                                            slot,
                                            save.level_id,
                                            restore.session_flags_raw,
                                            restore.controller_195_raw,
                                        );
                                        LoadingPurpose::NativeSave {
                                            restore: Box::new(restore),
                                        }
                                    }
                                    SaveSource::Portable => {
                                        log!(
                                            "Opening portable slot {} as a compatibility preview: level {} with player motion/attitude/health (craft mode, fuel, inventory, cargo, and campaign state remain unrestored)",
                                            slot,
                                            save.level_id
                                        );
                                        LoadingPurpose::PortableCompatibilityPreview { player }
                                    }
                                };
                                next_state = Some(GameState::Loading {
                                    level_id: save.level_id,
                                    purpose,
                                });
                            }
                        }
                        ShellEvent::SaveSlot(slot) => {
                            let result = map_save_checkpoint
                                .as_ref()
                                .ok_or_else(|| "No campaign-map checkpoint is active".to_owned())
                                .and_then(|checkpoint| {
                                    let disposition = if save_manager.is_occupied(slot) {
                                        v2k_game::save::SaveWriteDisposition::ConfirmedOverwrite
                                    } else {
                                        v2k_game::save::SaveWriteDisposition::EmptySlotOnly
                                    };
                                    save_manager.write_checkpoint(
                                        slot,
                                        checkpoint,
                                        disposition,
                                        gameplay_notifications.save_tail_seen_mask(),
                                    )
                                });
                            let succeeded = match result {
                                Ok(()) => {
                                    log!("Saved campaign checkpoint to slot {}", slot);
                                    true
                                }
                                Err(error) => {
                                    eprintln!("Save error: {error}");
                                    GlobalSoundRuntime::new(
                                        &session.cache,
                                        &mut world_fx,
                                        sound_manager.as_mut(),
                                    )
                                    .play_global_sound(2);
                                    false
                                }
                            };
                            let ctx = MenuCtx {
                                cache: &session.cache,
                                config: &config,
                                saves: Some(&save_manager),
                            };
                            shell.complete_save(succeeded, &ctx);
                        }
                        ShellEvent::ResumeGame if menu_is_paused => {
                            // The shell normally defers this until PAUSE_FULL's
                            // fly-out commits. Keep this handler centralized
                            // for that boundary event and any equivalent
                            // evidence-backed pause flow.
                            keys_down.clear();
                            mouse_buttons_down.clear();
                            next_state =
                                Some(match world_complete_results.continue_progress_map() {
                                    Some(route) => GameState::Loading {
                                        level_id: route.destination_level_id(),
                                        purpose: LoadingPurpose::CampaignWarp {
                                            source_level_id: route.source_level_id(),
                                            arrival_position_raw: route.arrival().position_raw,
                                            arrival_heading_raw: route.arrival().heading_raw,
                                        },
                                    },
                                    None => GameState::Playing,
                                });
                            map_save_checkpoint = None;
                        }
                        ShellEvent::QuitToFrontend if menu_is_paused => {
                            keys_down.clear();
                            mouse_buttons_down.clear();
                            next_state = Some(GameState::ReturningToFrontend);
                        }
                        // Pause-only flows are inert in the frontend context.
                        ShellEvent::ResumeGame | ShellEvent::QuitToFrontend => {}
                        ShellEvent::CheatGranted(code) => {
                            log!("Cheat granted: {:#x} (gameplay wiring pending)", code);
                        }
                    }
                }

                // Animate and render. In-game pause owns only the authored
                // screen stack; Klaus, the flame and frontend branding do not
                // replace the frozen gameplay presentation beneath it.
                let frontend_fade_before_actor = shell.frontend_depth_fade_mode();
                shell.update(
                    elapsed_micros,
                    &mut GlobalSoundRuntime::new(
                        &session.cache,
                        &mut world_fx,
                        sound_manager.as_mut(),
                    ),
                );
                if menu_is_paused {
                    if let Some(ShellEvent::ResumeGame) = shell.take_switch_away_event() {
                        // The 0x7000 fly-out has reached PAUSE_FULL's
                        // switch-away boundary. Schedule the live world for
                        // the next loop, after this final frozen-underlay frame
                        // has been presented.
                        keys_down.clear();
                        mouse_buttons_down.clear();
                        next_state = Some(match world_complete_results.continue_progress_map() {
                            Some(route) => GameState::Loading {
                                level_id: route.destination_level_id(),
                                purpose: LoadingPurpose::CampaignWarp {
                                    source_level_id: route.source_level_id(),
                                    arrival_position_raw: route.arrival().position_raw,
                                    arrival_heading_raw: route.arrival().heading_raw,
                                },
                            },
                            None => GameState::Playing,
                        });
                        map_save_checkpoint = None;
                    }
                }
                let begin_intro = shell.take_transition_ready();
                if menu_is_paused {
                    if redraw_paused_world {
                        if world_complete_results.is_progress_map_active() {
                            renderer.begin_scene(RenderScene::Menu);
                            renderer.clear(0.0, 0.0, 0.0);
                            if let (Some(backdrop), Some(fonts)) =
                                (overlay_51_backdrop.as_ref(), menu_fonts.as_ref())
                            {
                                draw_overlay_51_backdrop(
                                    renderer.as_mut(),
                                    fonts,
                                    backdrop,
                                    &player_campaign_progress,
                                    world_complete_results.age_ms(),
                                );
                                if let Some(prompt) = world_complete_results
                                    .continue_prompt_line(|id| session.cache.global_string(id))
                                {
                                    draw_gameplay_notifications(
                                        renderer.as_mut(),
                                        fonts,
                                        &[prompt],
                                        GameplayTextContext::AuthoredCanvas,
                                    );
                                }
                            }
                            paused_world_frame =
                                renderer.capture_frame(FrameCaptureSource::CurrentScene);
                        } else if let Some(paused) = &paused_gameplay_scene {
                            paused.redraw(
                                renderer.as_mut(),
                                PausedGameplayDraw {
                                    cache: &session.cache,
                                    colors: &face_colors,
                                    camera: &camera,
                                    camera_mode: if chase_mode {
                                        GameplayWorldCameraMode::RetailChase
                                    } else {
                                        GameplayWorldCameraMode::Free
                                    },
                                    native_viewport: if chase_mode {
                                        chase_camera.native_viewport()
                                    } else {
                                        None
                                    },
                                    projection: world_projection,
                                    sky: sky_background.as_ref(),
                                    terrain_frames: terrain_frames.as_ref(),
                                    water_frames: water_frames.as_ref(),
                                    abort_frame: main_base_level_abort.submitted_frame_request(),
                                    craft: &player_craft,
                                    hull: &player_hull,
                                    death: &player_death,
                                    player_model_id,
                                    targetter_enabled: config.targetter,
                                    overlay: &gameplay_overlay_frame,
                                    hud_resources: gameplay_hud_resources.as_ref(),
                                    fonts: menu_fonts.as_ref(),
                                    handoff: post_intro_world_cover.as_ref(),
                                },
                            );
                            paused_world_frame =
                                renderer.capture_frame(FrameCaptureSource::CurrentScene);
                        }
                    }
                    render_pause_menu(
                        &mut *renderer,
                        shell,
                        &system_strings,
                        &menu_resources,
                        menu_fonts.as_ref(),
                        &session.cache,
                        &face_colors,
                        retail_tick,
                        paused_world_frame.as_ref(),
                    );
                } else {
                    let (billboard_frame, wrapped) = billboard.prepare_frontend_render_frame(
                        elapsed_micros,
                        frontend_fade_before_actor,
                        shell.frontend_depth_fade_mode(),
                    );
                    render_menu(
                        &mut *renderer,
                        shell,
                        &system_strings,
                        &menu_resources,
                        menu_fonts.as_ref(),
                        &session.cache,
                        &face_colors,
                        &billboard_frame,
                        retail_tick,
                    );
                    if wrapped {
                        GlobalSoundRuntime::new(
                            &session.cache,
                            &mut world_fx,
                            sound_manager.as_mut(),
                        )
                        .play_global_sound(v2k_game::menu_engine::SOUND_BG_RUMBLE as usize);
                    }
                }

                if begin_intro && !menu_is_paused {
                    // Klaus is one persistent retail entity across this load.
                    // Carry the live shell instead of restarting its pose,
                    // scene clock, and twitch RNG after Intro2 is resident.
                    let handoff = std::mem::replace(shell, make_menu!(false));
                    next_state = Some(GameState::Loading {
                        level_id: INTRO2_LEVEL_ID,
                        purpose: LoadingPurpose::BeginIntro { shell: handoff },
                    });
                }

                if let Some(ns) = next_state {
                    if menu_is_paused {
                        paused_world_frame = None;
                        paused_gameplay_scene = None;
                    }
                    state = ns;
                }
            }

            GameState::ReturningToFrontend => {
                post_intro_world_cover = None;
                log!("Returning to menu...");
                retail_clock.reset();
                if let Some(ref mut sm) = sound_manager {
                    sm.stop_all();
                }
                player_fan_audio.reset();
                entity_positional_audio.reset_after_audio_stop();
                campaign_warp.reset_after_audio_stop();
                entity_manager = None;
                gameplay_overlay_frame = GameplayOverlayFrame::default();
                specialized_actor_tasks.clear_after_manager_reset();
                specialized_actor_task_error_reported = false;
                particle_collision_error_reported.clear();
                type9_task_diagnostics.reset_level();
                current_level_id = None;
                return_to_frontend_pending = false;
                gameplay_radar = None;
                world_fx.clear();
                pending_entity_weapon_fire.clear();
                native_entity_weapon_error_reported = false;
                static_damage = StaticDamageScheduler::new();
                keys_down.clear();
                mouse_buttons_down.clear();
                primary_weapon = PrimaryWeapon::new();
                targetter = TargetterRuntime::new(0);
                weapon_inventory = WeaponInventory::new();
                player_capabilities = PlayerCapabilities::default();
                player_campaign_progress = PlayerCampaignProgress::default();
                save_baseline.fill(0);
                map_save_checkpoint = None;
                session.cache.unload_level();
                // The cumulative system Section-11 pool is immutable across
                // worlds. Stop its voices above, but retain the decoded pool
                // and SDL device rather than rebuilding both at every handoff.
                // Original FUN_0044FEB0: returning to the frontend STOPS the
                // CD and rewinds — the frontend itself is silent.
                if let Some(ref mut mp) = music_player {
                    mp.stop();
                }
                let shell = make_menu!(false);
                state = GameState::Menu(shell);
                // Teardown is synchronous and must not become the first
                // frontend clock advance.
                reset_frame_clock_after_blocking_load(&mut last_frame);
                continue;
            }

            GameState::Loading { level_id, purpose } => {
                post_intro_world_cover = None;
                let lid = *level_id;
                let purpose = std::mem::replace(purpose, LoadingPurpose::Normal);
                // 42E270 stages flags0xF before the old body is released.
                // Both a map checkpoint and a direct handoff restore this
                // controller snapshot before any destination actor is born.
                let campaign_restore = if let LoadingPurpose::CampaignWarp {
                    arrival_position_raw,
                    arrival_heading_raw,
                    ..
                } = &purpose
                {
                    let restored = (|| -> Result<_, String> {
                        let manager = entity_manager.as_ref().ok_or("source manager absent")?;
                        let player = manager.player().ok_or("source player absent")?;
                        let RetailRuntimeValue::Known(cargo) =
                            manager.campaign_cargo_controller_state()
                        else {
                            return Err("player Sub-J state unresolved".into());
                        };
                        let logical_level_id = lid
                            .checked_sub(WORLD_OVERLAY_CONTROL_SLOT_OFFSET)
                            .ok_or("invalid campaign destination")?;
                        let name = session
                            .cache
                            .global_string(152 + logical_level_id as usize)
                            .or_else(|| session.cache.global_string(128))
                            .ok_or("campaign destination name unavailable")?;
                        let checkpoint = v2k_game::save::NativeSaveSnapshot {
                            logical_level_id,
                            player: SavedPlayerState {
                                position_raw: player.position_raw(),
                                velocity_raw: player.velocity_raw(),
                                heading_raw: player.heading_raw(),
                                pitch_raw: 0,
                                roll_raw: 0,
                                health_raw: player_hull.health_raw,
                            },
                            craft: &player_craft,
                            hull: &player_hull,
                            inventory: &weapon_inventory,
                            capabilities: &player_capabilities,
                            cargo: &cargo,
                            campaign: &player_campaign_progress,
                        }
                        .encode_campaign_arrival(
                            &save_baseline,
                            v2k_game::campaign_transition::WarpArrival {
                                position_raw: *arrival_position_raw,
                                heading_raw: *arrival_heading_raw,
                            },
                            name,
                        )
                        .map_err(|error| format!("controller checkpoint: {error:?}"))?;
                        v2k_game::save::NativeSaveRestore::decode(&checkpoint)
                            .map_err(|error| format!("controller restore: {error:?}"))
                    })();
                    match restored {
                        Ok(restore) => Some(restore),
                        Err(error) => {
                            eprintln!("Campaign warp cancelled: {error}");
                            state = GameState::Playing;
                            continue;
                        }
                    }
                } else {
                    None
                };
                let native_restore = match &purpose {
                    LoadingPurpose::NativeSave { restore } => Some(restore.as_ref()),
                    _ => campaign_restore.as_ref(),
                };
                let saved_player = match &purpose {
                    LoadingPurpose::NativeSave { restore } => Some(restore.player),
                    LoadingPurpose::PortableCompatibilityPreview { player } => Some(*player),
                    _ => None,
                };
                let campaign_cargo_controller_state =
                    native_restore.map(|restore| restore.cargo.clone());
                // Every retained specialized receipt authenticates the old
                // manager allocation. Invalidate that allocation before
                // releasing scheduler custody; campaign-warp cancellation
                // above deliberately returns before either operation.
                entity_manager = None;
                specialized_actor_tasks.clear_after_manager_reset();
                specialized_actor_task_error_reported = false;
                particle_collision_error_reported.clear();
                type9_task_diagnostics.reset_level();
                main_base_level_abort = MainBaseAbortBridgeState::default();
                current_world_entry_position_raw = None;
                pending_failed_world_interior_request = false;
                gameplay_radar = None;
                world_fx.clear();
                pending_entity_weapon_fire.clear();
                native_entity_weapon_error_reported = false;
                // 44F7E1 resets hint history for frontend/session selection.
                // A456D10 campaign warp retains that history across worlds.
                if !matches!(&purpose, LoadingPurpose::CampaignWarp { .. }) {
                    gameplay_notifications.reset_session();
                }
                world_complete_results.reset();
                world_complete_tally = WorldCompleteTally::default();
                gameplay_hud.reset();
                gameplay_overlay_frame = GameplayOverlayFrame::default();
                static_damage = StaticDamageScheduler::new();
                // Loading a world terminates menu/previous-world voices while
                // retaining the immutable global Section-11 pool and device.
                if let Some(ref mut sm) = sound_manager {
                    sm.stop_all();
                }
                player_fan_audio.reset();
                entity_positional_audio.reset_after_audio_stop();
                campaign_warp.reset_after_audio_stop();
                log!("Loading level {}...", lid);
                render_loading_card(
                    renderer.as_mut(),
                    &system_strings,
                    &menu_resources,
                    menu_fonts.as_ref(),
                    &session.cache,
                    retail_tick,
                );

                let start = Instant::now();
                if let Err(error) = session.load_level_by_id(lid, startup_world_variant) {
                    eprintln!("Level {} load blocked: {}", lid, error);
                    current_level_id = None;
                    session.cache.unload_level();
                    state = GameState::ReturningToFrontend;
                    continue 'main_loop;
                }
                if session.cache.level_desc().is_none() {
                    eprintln!(
                        "Level {} load blocked: overlay has no gameplay descriptor (Section 13)",
                        lid
                    );
                    current_level_id = None;
                    session.cache.unload_level();
                    state = GameState::ReturningToFrontend;
                    continue 'main_loop;
                }
                // Direct --level has no source route. Use a deterministic incoming
                // authored arrival; New Game, saves and real warps own their poses.
                let direct_world_arrival = if matches!(
                    &purpose,
                    LoadingPurpose::Normal | LoadingPurpose::ResumePostIntro { .. }
                ) && lid != FIRST_WORLD_LEVEL_ID
                    && (13..=48).contains(&lid)
                {
                    if campaign_arrivals.is_none() {
                        match CampaignArrivalCatalog::load(&session, startup_world_variant) {
                            Ok(catalog) => campaign_arrivals = Some(catalog),
                            Err(error) => {
                                eprintln!("Level {lid} entry blocked: {error}");
                                current_level_id = None;
                                session.cache.unload_level();
                                state = GameState::ReturningToFrontend;
                                continue 'main_loop;
                            }
                        }
                    }
                    campaign_arrivals.as_ref().and_then(|catalog| {
                        catalog.direct_world_entry(lid).map(|entry| {
                            log!("Direct world {lid} entry: {entry:?}");
                            let arrival = entry.arrival();
                            v2k_game::entity::AuthoredPlayerArrival {
                                position_raw: arrival.position_raw,
                                heading_raw: arrival.heading_raw,
                            }
                        })
                    })
                } else {
                    None
                };
                current_level_id = Some(lid);
                if let Some(restore) = native_restore {
                    // Mode6 restores these controller owners before42E570's
                    // world initialization and before any pickup/death event.
                    weapon_inventory = restore.inventory.clone();
                    player_capabilities = restore.capabilities;
                    player_campaign_progress = restore.campaign;
                }
                // The session campaign slot must be installed before
                // FUN_0042E570 initializes this world's +0x1F0/+0x1EC time-
                // trophy controller. Inventory and campaign bits persist
                // across an in-memory world replacement; the timer does not.
                player_campaign_progress.set_current_control_slot(world_control_slot(lid));
                let mut time_trophy_load_reward = None;
                if world_control_slot(lid).is_some() {
                    let descriptor = session
                        .cache
                        .level_desc()
                        .expect("validated Section 13 remains loaded");
                    let time_trophy_load = main_base_level_abort
                        .bind_loaded_world(
                            next_world_control_load_generation,
                            descriptor,
                            &mut player_campaign_progress,
                        )
                        .expect("validated world control slot initializes campaign state");
                    time_trophy_load_reward = time_trophy_load.reward;
                    log!(
                        "Time trophy: deadline {}s, state {:?}{}",
                        descriptor.time_trophy_deadline_seconds(),
                        time_trophy_load.state,
                        if time_trophy_load.auto_claimed_now {
                            ", newly claimed campaign bit 0x8"
                        } else {
                            ""
                        }
                    );
                    next_world_control_load_generation = NonZeroU64::new(
                        next_world_control_load_generation
                            .get()
                            .checked_add(1)
                            .expect("world-control load generation exhausted"),
                    )
                    .unwrap();
                }

                let arrived_by_campaign_warp =
                    matches!(&purpose, LoadingPurpose::CampaignWarp { .. });
                match (
                    session.cache.level_desc(),
                    session.cache.terrain(),
                    session.cache.terrain_objects(),
                ) {
                    (Some(level), Some(terrain), Some(terrain_objects)) => {
                        if let Err(error) =
                            campaign_warp.rebuild(lid, level, terrain, terrain_objects)
                        {
                            eprintln!("Campaign warp markers disabled: {error:?}");
                            campaign_warp.reset_after_audio_stop();
                        }
                    }
                    _ => campaign_warp.reset_after_audio_stop(),
                }

                // In-game CD music, matching Music_SelectTrack (0x456AF0):
                // track = world + 1 where world = Section13 +0x48 (1-6 →
                // CD tracks 2-7; track02.ogg = index 0 → index = world-1).
                // Selection occurs even at Ambient=0; that setting only
                // pauses the selected descriptor at its preserved cursor.
                // Intro2 owns no CD track, and unproven retail mode overrides
                // remain intentionally absent.
                if let Some(ref mut mp) = music_player {
                    if lid == INTRO2_LEVEL_ID {
                        // Intro2 is an in-engine story level, not an ordinary
                        // world session, and owns no CD-audio descriptor.
                        mp.stop();
                    } else {
                        let world_style = session
                            .cache
                            .level_desc()
                            .map(|d| d.world_style)
                            .unwrap_or(1);
                        if let Some(track) = normal_world_music_track(lid, world_style) {
                            if let Err(e) = mp.select_cd_track(track) {
                                eprintln!("Music track switch error: {}", e);
                                mp.stop();
                            } else {
                                apply_session_music_gate(&config, mp, false);
                            }
                        }
                    }
                }
                // Drop the previous level's cached terrain mesh so this level
                // rebuilds its own (the cache is not keyed on the grid).
                renderer.invalidate_terrain_cache();
                terrain_lights.clear();
                if let Some(old) = terrain_frames.take() {
                    for texture in old.textures() {
                        renderer.destroy_texture(texture);
                    }
                }
                terrain_frames = v2k_game::terrain_render::build_terrain_frames(
                    &session.cache,
                    renderer.as_mut(),
                );
                match &terrain_frames {
                    Some(_) => log!("Terrain: 120 canonical + 5 infection shapes loaded"),
                    None => log!("Terrain: transition sprites unavailable — diagnostic fallback"),
                }
                sky_background = main_base_level_abort
                    .submitted_frame_request()
                    .and_then(|request| {
                        v2k_game::sky::build_sky_background_for_index(
                            &session.cache,
                            request.word_0xb0,
                        )
                    })
                    .or_else(|| v2k_game::sky::build_sky_background(&session.cache));
                match (&sky_background, session.cache.level_desc()) {
                    (Some(_), Some(desc)) => log!(
                        "Sky: master palette colour {} loaded (model {})",
                        desc.sky_color_index,
                        desc.sky_model
                    ),
                    _ => log!("Sky: authored background colour unavailable"),
                }
                // Rebuild the shoreline frame set for this level (5 shape
                // sprites at terrain_sprite_base+125, see v2k_game::water).
                if let Some(old) = water_frames.take() {
                    for texture in old.textures() {
                        renderer.destroy_texture(texture);
                    }
                }
                water_frames =
                    v2k_game::water::build_water_frames(&session.cache, renderer.as_mut());
                match &water_frames {
                    Some(_) => log!("Water: textured shoreline frames loaded"),
                    None => {
                        if session.cache.terrain().is_some_and(|t| t.water_enabled()) {
                            log!("Water: shoreline sprites unavailable — flat-color fallback");
                        }
                    }
                }
                let load_time = start.elapsed();
                log!(
                    "Level {} loaded in {:.1}ms",
                    lid,
                    load_time.as_secs_f64() * 1000.0
                );

                let hover_physics = session
                    .cache
                    .global_entity_type(46)
                    .and_then(HoverPhysicsConfig::from_record)
                    .unwrap_or_default();

                // Create entity manager
                let Some(level_desc) = session.cache.level_desc() else {
                    eprintln!(
                        "Level {} initialization blocked: Section 13 disappeared after validation",
                        lid
                    );
                    current_level_id = None;
                    session.cache.unload_level();
                    state = GameState::ReturningToFrontend;
                    continue 'main_loop;
                };
                let mut native_terrain =
                    native_restore.and_then(|_| session.cache.level_terrain().cloned());
                let mut em = {
                    let type_models = session.cache.global_entity_model_table();
                    let type_metadata: Vec<_> = type_models
                        .iter()
                        .copied()
                        .enumerate()
                        .map(|(entity_type, model_slots)| {
                            session
                                .cache
                                .global_entity_type(entity_type)
                                .map(EntityTypeRuntimeMetadata::from_section12)
                                .unwrap_or(EntityTypeRuntimeMetadata {
                                    model_slots,
                                    ..EntityTypeRuntimeMetadata::default()
                                })
                        })
                        .collect();
                    log!(
                        "Entity models: {} exact Section-12 type records loaded",
                        type_models.len()
                    );
                    let model_extent_raw =
                        |id| session.cache.global_model(id).map(|model| model.radius);
                    let resources = v2k_game::entity::EntityConstructionResources {
                        terrain: session.cache.terrain(),
                        terrain_objects: session.cache.terrain_objects(),
                        model_extent_raw: Some(&model_extent_raw),
                    };
                    let manager: Result<_, String> = if matches!(
                        &purpose,
                        LoadingPurpose::BeginIntro { .. }
                    ) {
                        v2k_game::entity::EntityManager::from_native_intro2_frontend(
                            level_desc,
                            &type_metadata,
                            resources,
                            retail_tick,
                            &mut world_fx,
                        )
                        .map_err(|error| format!("captured Intro2 publication: {error:?}"))
                    } else if purpose.uses_authored_world_publication(lid) {
                        let player_arrival = match &purpose {
                            LoadingPurpose::CampaignWarp {
                                arrival_position_raw,
                                arrival_heading_raw,
                                ..
                            } => Some(v2k_game::entity::AuthoredPlayerArrival {
                                position_raw: *arrival_position_raw,
                                heading_raw: *arrival_heading_raw,
                            }),
                            _ if lid == 49 => {
                                // Intro1 is outside the campaign graph. Keep its
                                // existing direct-debug controller publication.
                                let arrival = v2k_game::campaign_transition::RETAIL_CONTROLLER_DEFAULT_ARRIVAL;
                                Some(v2k_game::entity::AuthoredPlayerArrival {
                                    position_raw: arrival.position_raw,
                                    heading_raw: arrival.heading_raw,
                                })
                            }
                            _ => direct_world_arrival,
                        };
                        let construction = v2k_game::entity::AuthoredWorldConstruction {
                            logical_world_index: (lid - WORLD_OVERLAY_CONTROL_SLOT_OFFSET) as i32,
                            level: level_desc,
                            type_metadata: &type_metadata,
                            resources,
                            player_arrival,
                            retail_tick,
                        };
                        let construction = match native_restore {
                            Some(restore) => construction.with_native_save(
                                v2k_game::entity::NativeSaveWorldState {
                                    restore,
                                    terrain: native_terrain.as_mut().expect("native world terrain"),
                                    resources: &session.cache,
                                    static_damage: &mut static_damage,
                                },
                            ),
                            None => construction.with_exit_marker_bits(
                                world_control_slot(lid)
                                    .and_then(|slot| {
                                        player_campaign_progress.control_slot_bits(slot)
                                    })
                                    .unwrap_or(0),
                            ),
                        };
                        v2k_game::entity::EntityManager::from_authored_world(
                            construction,
                            &mut world_fx,
                        )
                        .map_err(|error| format!("authored world construction: {error:?}"))
                    } else {
                        Ok(
                            v2k_game::entity::EntityManager::from_level_with_type_metadata(
                                level_desc,
                                &type_metadata,
                                resources,
                            ),
                        )
                    };
                    let mut manager = match manager {
                        Ok(manager) => manager,
                        Err(error) => {
                            eprintln!("Level {} initialization blocked: {error}", lid);
                            current_level_id = None;
                            session.cache.unload_level();
                            state = GameState::ReturningToFrontend;
                            continue 'main_loop;
                        }
                    };
                    if let LoadingPurpose::PortableCompatibilityPreview { player } = &purpose {
                        manager.restore_persistent_player_from_save(
                            type_metadata.get(46),
                            player.position_raw,
                            player.velocity_raw,
                            player.heading_raw,
                            session.cache.terrain(),
                        );
                        if let Err(error) = manager.construct_player_propulsion(&mut world_fx) {
                            eprintln!("Portable player construction blocked: {error}");
                            current_level_id = None;
                            session.cache.unload_level();
                            state = GameState::ReturningToFrontend;
                            continue 'main_loop;
                        }
                    }
                    manager
                };
                if let Some((reward, player)) = time_trophy_load_reward.zip(em.player()) {
                    queue_trophy_acquisition_feedback(
                        reward,
                        player.position_raw(),
                        &mut gameplay_notifications,
                        &mut world_fx,
                        retail_tick,
                    );
                }
                if let Some(terrain) = native_terrain {
                    *session
                        .cache
                        .level_terrain_mut()
                        .expect("native world terrain") = terrain;
                    renderer.invalidate_terrain_cache();
                }
                // 42E570 finishes authored construction and completed-world terrain
                // writes before 44AFB0 builds the persistent radar. 451710 restores
                // cargo only afterwards: both phases consume the process RNG.
                if let Err(error) = session.cache.initialize_level_terrain_radar(&mut || {
                    world_fx.next_shared_retail_random_u16()
                }) {
                    eprintln!("Level {lid} radar initialization blocked: {error:?}");
                    current_level_id = None;
                    session.cache.unload_level();
                    state = GameState::ReturningToFrontend;
                    continue 'main_loop;
                }
                if purpose.uses_authored_world_publication(lid) {
                    let level_desc = session
                        .cache
                        .level_desc()
                        .expect("validated world descriptor");
                    let model_extent_raw =
                        |id| session.cache.global_model(id).map(|model| model.radius);
                    let resources = v2k_game::entity::EntityConstructionResources {
                        terrain: session.cache.terrain(),
                        terrain_objects: session.cache.terrain_objects(),
                        model_extent_raw: Some(&model_extent_raw),
                    };
                    // 51710 runs451C00 after every ordinary native player
                    // load, including an empty fresh profile. Its surface
                    // and world-style6 ballast phases are not warp-only.
                    let cargo_state = campaign_cargo_controller_state.unwrap_or_else(|| {
                        v2k_game::entity::CampaignCargoControllerState::from_packed(
                            em.player_cargo_unlock_raw(),
                            [],
                        )
                    });
                    match em.restore_campaign_cargo_controller_state(
                        cargo_state,
                        v2k_game::entity::CampaignCargoRestoreContext {
                            level: level_desc,
                            resources,
                            retail_tick,
                            scheduler: &mut specialized_actor_tasks,
                            world_fx: &mut world_fx,
                            notifications: &mut gameplay_notifications,
                        },
                    ) {
                        Ok(()) => {
                            if let RetailRuntimeValue::Known(restored) =
                                em.campaign_cargo_controller_state()
                            {
                                let occupied = restored.occupied_slots();
                                log!(
                                    "Campaign cargo: restored {} occupied slot{}",
                                    occupied,
                                    if occupied == 1 { "" } else { "s" }
                                );
                            }
                        }
                        Err(error) => {
                            eprintln!("Level {lid} cargo initialization blocked: {error:?}");
                            current_level_id = None;
                            session.cache.unload_level();
                            state = GameState::ReturningToFrontend;
                            continue 'main_loop;
                        }
                    }
                }
                // Intro2 has no gameplay HUD. Ordinary presentation borrows the
                // live raster; decoding these resources consumes no RNG.
                if lid != INTRO2_LEVEL_ID {
                    gameplay_radar = gameplay_hud_variant
                        .and_then(|variant| GameplayRadar::from_cache(&session.cache, variant));
                    log!(
                        "Radar: {}",
                        if gameplay_radar.is_some() {
                            "terrain globe and fullscreen map resources loaded"
                        } else {
                            "authored resources unavailable"
                        }
                    );
                }

                if purpose.uses_authored_world_publication(lid) {
                    world_fx.process_deferred_cargo_transfers(
                        v2k_game::world_fx::ParticleBirthContext {
                            environment: ParticleEnvironment::Terrain(
                                TerrainCollisionContext::from_current_level_cache(&session.cache)
                                    .expect("native world terrain and descriptor"),
                            ),
                            retail_tick,
                        },
                    );
                }
                em.set_hover_physics(hover_physics);
                if lid == INTRO2_LEVEL_ID {
                    // Intro2 operation 2 releases authored behavior
                    // components over the story timeline. Their models remain
                    // independently eligible for rendering before release.
                    em.disable_authored_behavior_components();
                }

                has_player = em.player().is_some();
                if has_player {
                    log!("Player entity found — chase camera active (F11 toggles free-fly)");
                } else {
                    log!("No player entity (type 46) — free-fly camera only");
                }
                chase_mode = has_player;

                // Set up this world's fog configuration from scratch. The
                // successful retail world load reads Section13 +88/+8C as
                // far distance/fog width, independent of the terrain scan cap.
                // It uses the authored sky colour as its terminal colour.
                // Section 6 is the surface shade-row lookup, not a fog RGB
                // source. Menu scene entry suppresses this world-only state.
                renderer.set_world_model_fog(None);
                let authored_fog = session
                    .cache
                    .level_desc()
                    .map(|descriptor| descriptor.fog_planes_raw());
                let fog_color = sky_background
                    .as_ref()
                    .map(|sky| sky.color)
                    .unwrap_or([0.5, 0.6, 0.7]);
                if let Some(planes) = authored_fog {
                    renderer.set_world_model_fog(Some(v2k_render::WorldModelFog {
                        planes,
                        color: fog_color,
                    }));
                } else {
                    eprintln!("World fog disabled: loaded world missing authored Section13 planes");
                }

                // Session+38 retains the incoming tuple, before D4A0 terrain
                // placement. A constructed player's adjustedY cannot replace it.
                current_world_entry_position_raw = native_restore
                    .map(|restore| restore.player.position_raw)
                    .or_else(|| saved_player.map(|player| player.position_raw))
                    .or_else(|| direct_world_arrival.map(|arrival| arrival.position_raw))
                    .or_else(|| {
                        (lid == FIRST_WORLD_LEVEL_ID || lid == 49).then_some(
                            v2k_game::campaign_transition::RETAIL_CONTROLLER_DEFAULT_ARRIVAL
                                .position_raw,
                        )
                    });

                // Every player allocation constructs fresh type-46 component
                // state and draws the authored drive-target jitter. Campaign
                // warps retain the same controller pointer, mode, and exact
                // fuel value while replacing only the entity/component. This
                // must precede chase-camera initialization: captured arrivals
                // have an identity body basis, not the source craft's attitude.
                let hover_target = if let Some(player) = em.player() {
                    let RetailRuntimeValue::Known(Some(sub_a)) = player.sub_a_propulsion_runtime
                    else {
                        return Err("loaded player has no constructed Sub-A".into());
                    };
                    let RetailRuntimeValue::Known(target) = sub_a.target_speed_raw() else {
                        return Err("loaded player has no constructed drive target".into());
                    };
                    target
                } else {
                    // Cinematics without a player do not allocate Sub-A or
                    // consume its random word. This craft state remains idle.
                    i32::from(hover_physics.target_speed_base_raw)
                };
                player_craft = if arrived_by_campaign_warp {
                    player_craft.campaign_world_replacement(hover_physics, hover_target)
                } else {
                    PlayerCraft::with_hover_target(hover_physics, hover_target)
                };
                if let Some(restore) = native_restore {
                    restore.restore_craft(&mut player_craft);
                } else if let Some(player) = saved_player {
                    player_craft.restore_body_angle_words([
                        player.pitch_raw as i16,
                        player.roll_raw as i16,
                    ]);
                }

                // Position camera
                let (render_w, render_h) = renderer.viewport_size();
                camera = Camera::new(render_w as f32 / render_h.max(1) as f32);
                world_projection.apply_to(&mut camera, (render_w, render_h));
                chase_camera.reset();
                if let Some(player) = em.player() {
                    let pose = chase_camera.update(
                        ChaseCameraTarget {
                            position_raw: player.position_raw(),
                            body_basis: ChaseBodyBasis::from_vectors(
                                player_craft.retail_body_basis_q31(player.heading),
                            ),
                            active_camera: config.active_camera,
                        },
                        gameplay_chase_terrain(&session.cache),
                        0,
                    );
                    // FUN_0040F3A0 initializes both springs at their targets,
                    // so level entry starts on the exact pose without a swoop.
                    pose.apply_to(&mut camera);
                }

                sync_player_weapon_callback(&mut player_craft, &weapon_inventory);
                player_fan_audio.reset();
                let loaded_hull_profile = session
                    .cache
                    .global_entity_type(46)
                    .map(HullDamageProfile::from_type_record)
                    .unwrap_or(PLAYER_TYPE_46_HULL_PROFILE);
                player_hull = if arrived_by_campaign_warp {
                    // 443440's flag1 refills hull health;443560 also restores
                    // the retained controller+1BC shield into entity+50.
                    player_hull.campaign_world_replacement(loaded_hull_profile)
                } else {
                    PlayerHull::new(loaded_hull_profile)
                };
                if let Some(restore) = native_restore {
                    restore.restore_hull(&mut player_hull);
                    if let Some(player) = em.player() {
                        player_hull.readback_entity_damage_state(&player.collision);
                    }
                    em.sync_player_hull_collision_state(&player_hull);
                } else if let Some(player) = saved_player {
                    player_hull.health_raw = player.health_raw;
                    em.sync_player_hull_collision_state(&player_hull);
                } else if arrived_by_campaign_warp {
                    em.sync_player_hull_collision_state(&player_hull);
                }
                player_death = PlayerDeathLifecycle::default();
                player_shield = v2k_game::player_shield::PlayerShieldPresentation::default();
                if let Some(player) = em.player() {
                    targetter.rebind_owner(player.id);
                    if player_capabilities.has_targetter() && !targetter.is_active() {
                        targetter.install();
                    }
                }
                return_to_frontend_pending = false;
                static_contact_error_reported = false;
                active_pair_contact_error_reported = false;
                main_base_conversion_error_reported = false;
                factory_pair_error_reported = false;
                power_up_contact_error_reported = false;
                log!(
                    "Hover: Section-12 A/B/C/D loaded, drive target {} raw",
                    player_craft.hover_target_speed_raw()
                );
                player_model_id = session
                    .cache
                    .global_model_by_name("player4")
                    .map(|(id, _)| id);
                match player_model_id {
                    Some(id) => log!("Player model: player4 = global pool id {}", id),
                    None => log!("Player model: player4 not found in global pool"),
                }
                // The old manager was cleared before construction.451C00 may
                // already own authored class0 cargo or a native ballast birth;
                // preserve those leases while adopting the remaining families.
                match specialized_actor_tasks.adopt_fresh_level1_type9_selected(&mut em) {
                    Ok(0) => {}
                    Ok(adopted) => log!(
                        "Ordinary Type-9: adopted {adopted} selected Run Away/Go-To-Job scheduler owner{}",
                        if adopted == 1 { "" } else { "s" }
                    ),
                    Err(error) => {
                        eprintln!(
                            "Level {lid} initialization blocked while adopting selected Type-9 production: {error:?}"
                        );
                        specialized_actor_tasks.clear_after_manager_reset();
                        current_level_id = None;
                        session.cache.unload_level();
                        state = GameState::ReturningToFrontend;
                        continue 'main_loop;
                    }
                }
                match specialized_actor_tasks.adopt_fresh_level1_type17_follow_beacons(&em) {
                    0 => {}
                    adopted => log!(
                        "Ordinary Type-17: adopted {adopted} Follow Beacons scheduler owner{}",
                        if adopted == 1 { "" } else { "s" }
                    ),
                }
                match specialized_actor_tasks.adopt_level_one_factory_arrival(&em) {
                    0 => {}
                    adopted => {
                        log!("Level-1 factory: adopted {adopted} explicit-arrival scheduler owner")
                    }
                }
                match specialized_actor_tasks.adopt_fresh_level1_type47_scheduler(&mut em) {
                    Ok(0) => {}
                    Ok(adopted) => log!(
                        "Ordinary Type-47: adopted {adopted} Guard/Wander scheduler owner{}",
                        if adopted == 1 { "" } else { "s" }
                    ),
                    Err(error) => {
                        eprintln!(
                            "Level {lid} initialization blocked while adopting Type-47 production: {error:?}"
                        );
                        specialized_actor_tasks.clear_after_manager_reset();
                        current_level_id = None;
                        session.cache.unload_level();
                        state = GameState::ReturningToFrontend;
                        continue 'main_loop;
                    }
                }
                match specialized_actor_tasks.adopt_intro2_type13_search_attack(&em) {
                    0 => {}
                    adopted => {
                        log!("Intro2 type-13: adopted {adopted} B6C0 Search And Attack owner")
                    }
                }
                match specialized_actor_tasks.adopt_intro2_type26(&em) {
                    0 => {}
                    adopted => {
                        log!("Intro2 type-26: adopted {adopted} native task owner")
                    }
                }
                match specialized_actor_tasks.adopt_intro2_type47_guards(&em) {
                    0 => {}
                    adopted => log!(
                        "Shared Type-47: adopted {adopted} Guard/Wander owner{}",
                        if adopted == 1 { "" } else { "s" }
                    ),
                }
                match specialized_actor_tasks.adopt_intro2_flyers(&em) {
                    0 => {}
                    adopted => log!(
                        "Intro2 flyers: adopted {adopted} live flyer mover owner{}",
                        if adopted == 1 { "" } else { "s" }
                    ),
                }
                specialized_actor_tasks.adopt_intro2_type53(&em);
                specialized_actor_tasks.adopt_type122(&em);
                specialized_actor_tasks.adopt_type30(&em);
                specialized_actor_tasks.adopt_type40(&em);
                specialized_actor_tasks.adopt_type56(&em);
                specialized_actor_tasks.adopt_shared_fish(&em);
                specialized_actor_tasks.adopt_cleansing_vehicle(&em);
                specialized_actor_tasks.adopt_intro2_type16(&em);
                specialized_actor_tasks.adopt_intro2_type58(&em);
                specialized_actor_tasks.adopt_intro2_type94(&em);
                specialized_actor_tasks.adopt_intro2_type66(&em);
                specialized_actor_tasks.adopt_class0_actors(&em);
                specialized_actor_tasks.adopt_main_base(&em);
                specialized_actor_tasks.adopt_intro2_type10(&em);
                specialized_actor_tasks.adopt_intro2_type57(&em);
                specialized_actor_tasks.adopt_intro2_gun_turret(&em);
                specialized_actor_tasks.adopt_intro2_type17(&em);
                specialized_actor_tasks.adopt_intro2_type8(&mut em);
                specialized_actor_tasks.adopt_intro2_type9(&mut em);
                specialized_actor_tasks.adopt_native_type123(&mut em);
                specialized_actor_tasks.adopt_native_type86(&mut em);
                let meteors = specialized_actor_tasks.adopt_intro2_meteors(&em);
                if meteors > 0 {
                    log!("Intro2: adopted {meteors} Boulder/Trailing Fire owners");
                }
                // FUN_004568B0 admits the callback only in gameplay phase 5.
                // Ordinary native Section-13 rebuilds drain here.451710 then
                // clears the visible load slots while preserving their hint
                // masks, before428AD0 resets the world clock below.
                if purpose.uses_authored_world_publication(lid) {
                    match gameplay_notifications
                        .drain_fresh_level1_type9_attract_attention_receipts(&mut em)
                    {
                        Ok(0) => {}
                        Ok(drained) => log!(
                            "Ordinary Type-9 Attract Attention: drained {drained} canonical resource-text receipt{}",
                            if drained == 1 { "" } else { "s" }
                        ),
                        Err(error) => {
                            eprintln!(
                                "Level {lid} initialization blocked while draining Type-9 Attract Attention notifications: {error:?}"
                            );
                            specialized_actor_tasks.clear_after_manager_reset();
                            current_level_id = None;
                            session.cache.unload_level();
                            state = GameState::ReturningToFrontend;
                            continue 'main_loop;
                        }
                    }
                    gameplay_notifications.clear_native_load_slots();
                } else if !em
                    .pending_fresh_level1_type9_resource_text_receipts()
                    .is_empty()
                {
                    // A receipt outside an overlay-13 Section-13 rebuild would
                    // indicate an unauthenticated construction route.
                    eprintln!(
                        "Level {lid} initialization blocked: Type-9 Attract Attention notification escaped its fresh first-world admission"
                    );
                    specialized_actor_tasks.clear_after_manager_reset();
                    current_level_id = None;
                    session.cache.unload_level();
                    state = GameState::ReturningToFrontend;
                    continue 'main_loop;
                }
                world_complete_tally
                    .snapshot_world_load_census(em.fun_0042e210_capability_census());
                entity_manager = Some(em);

                keys_down.clear();
                mouse_buttons_down.clear();
                primary_weapon = PrimaryWeapon::new();
                // FUN_00428AD0 resets the shared tick only after a successful
                // synchronous load; the next active frame starts at zero.
                retail_clock.reset();
                retail_tick = retail_clock.current();
                state = match purpose {
                    LoadingPurpose::BeginIntro { mut shell } => {
                        intro_camera = IntroCameraController::default();
                        intro_commands =
                            v2k_game::intro2_commands::Intro2Commands::from_cache(&session.cache)
                                .expect("loaded Intro2 Section-2 commands");
                        shell.begin_intro_reveal();
                        // 4519D7 follows construction/radar and Klaus priming:
                        // Intro2's Type66 actors start with current health 1.
                        let manager = entity_manager.as_mut().expect("loaded Intro2 actors");
                        let structures: Vec<_> = manager
                            .iter_all()
                            .filter(|entity| entity.entity_type == 66)
                            .map(|entity| entity.id)
                            .collect();
                        for id in structures {
                            if let Err(error) =
                                v2k_game::intro2_type66::apply_intro2_type66_post_load_health(
                                    manager.entity_mut(id).expect("loaded Intro2 structure"),
                                )
                            {
                                eprintln!("Intro2 structure setup blocked: {error:?}");
                                entity_manager = None;
                                specialized_actor_tasks.clear_after_manager_reset();
                                current_level_id = None;
                                session.cache.unload_level();
                                state = GameState::ReturningToFrontend;
                                continue 'main_loop;
                            }
                        }
                        log!("Intro2: advancing behind Klaus's opening morph");
                        GameState::OpeningCinematic {
                            exit_pending: false,
                            elapsed: 0.0,
                            stage: Intro2PresentationStage::Covered { shell },
                        }
                    }
                    LoadingPurpose::ResumePostIntro { mut shell } => {
                        // Retail starts global id 50 only once the first-world
                        // load/HUD setup has completed. Global 50 is physical
                        // sec11_3XX_043, verified as a 62,700-byte PCM blob.
                        GlobalSoundRuntime::new(
                            &session.cache,
                            &mut world_fx,
                            sound_manager.as_mut(),
                        )
                        .play_global_sound(POST_INTRO_SOUND_ID);
                        shell.begin_post_intro_leg(PostIntroStage::AfterWorldLoad);
                        // 4515E0 restores 4D0918: normal world callbacks run
                        // throughout command 2's opening morph. Retain only
                        // its presentation owner over the live Playing pass.
                        post_intro_world_cover = Some(shell);
                        GameState::Playing
                    }
                    LoadingPurpose::CampaignWarp {
                        source_level_id, ..
                    } => {
                        log!("Campaign warp complete: {} -> {}", source_level_id, lid);
                        GameState::Playing
                    }
                    LoadingPurpose::Normal if lid == INTRO2_LEVEL_ID => {
                        intro_camera = IntroCameraController::default();
                        intro_commands =
                            v2k_game::intro2_commands::Intro2Commands::from_cache(&session.cache)
                                .expect("loaded Intro2 Section-2 commands");
                        log!(
                            "Intro2: entering {:.1}s story cinematic",
                            INTRO2_DURATION_SECS
                        );
                        // Direct --level 50 remains a development entry and
                        // has no frontend presentation to carry across load.
                        GameState::OpeningCinematic {
                            exit_pending: false,
                            elapsed: 0.0,
                            stage: Intro2PresentationStage::Visible { shell: None },
                        }
                    }
                    LoadingPurpose::NativeSave { .. }
                    | LoadingPurpose::PortableCompatibilityPreview { .. } => GameState::Playing,
                    LoadingPurpose::Normal => GameState::Playing,
                };
                // Loading is synchronous. Its wall time is neither a retail
                // simulation tick nor part of Klaus's opening callback, so
                // the next frame must begin from this boundary.
                reset_frame_clock_after_blocking_load(&mut last_frame);
            }

            GameState::OpeningCinematic {
                elapsed,
                stage,
                exit_pending,
            } => {
                let mut static_damage_explosion_lights = Vec::new();
                retail_tick = retail_clock.advance(u64::from(elapsed_micros));
                // FUN_0044FFA0 belongs to the active world handler, not the
                // frontend/AVI/loading paths. It sees the raw wall duration
                // before the simulation clock's independent 125 ms clamp.
                world_fx.advance_frame_pacing(uncapped_elapsed_micros);
                let mut skip = false;
                for event in &events {
                    match event {
                        GameEvent::Quit => break 'main_loop,
                        GameEvent::Resize(w, h) => {
                            renderer.resize(*w, *h);
                            world_projection.apply_to(&mut camera, renderer.viewport_size());
                        }
                        GameEvent::KeyDown(Keycode::Escape)
                        | GameEvent::KeyDown(Keycode::Return)
                        | GameEvent::KeyDown(Keycode::Space)
                        | GameEvent::MouseButtonDown(_) => skip = true,
                        _ => {}
                    }
                }

                if *exit_pending {
                    if let Some(ref mut sm) = sound_manager {
                        sm.stop_all();
                    }
                    // Retail keeps Intro2 resident throughout the first
                    // closing morph. The typed Loading state replaces the
                    // level and destroys its cached render resources only
                    // after the Klaus closing morph completes.
                    let ctx = MenuCtx {
                        cache: &session.cache,
                        config: &config,
                        saves: Some(&save_manager),
                    };
                    let shell = match stage.take_shell() {
                        Some(mut shell) => {
                            shell.begin_post_intro_leg(PostIntroStage::BeforeWorldLoad);
                            shell
                        }
                        None => MenuShell::new_post_intro(&ctx),
                    };
                    // 453BA0 selects world[4]; its 44FE20 callback consumes
                    // world+274 and resets the 50 Hz clock through 428AD0
                    // before the first closing actor callback.
                    retail_clock.reset();
                    state = GameState::PostIntro { shell };
                    continue;
                }

                stage.advance(
                    elapsed,
                    elapsed_micros,
                    &mut GlobalSoundRuntime::new(
                        &session.cache,
                        &mut world_fx,
                        sound_manager.as_mut(),
                    ),
                );
                intro_camera.advance_proxy(elapsed_micros);
                let terrain_scan_dimensions = terrain_frames
                    .as_ref()
                    .map(|frames| (frames.scan_columns, frames.scan_rows))
                    .or_else(|| {
                        session.cache.level_desc().map(|desc| {
                            v2k_render::terrain_tiles::scan_dimensions(desc.terrain_draw_depth)
                        })
                    })
                    .unwrap_or((52, 30));
                let camera_true_up_z = camera.view_matrix()[9];
                let actor_animation_claims = specialized_actor_tasks
                    .actor_animation_claims()
                    .collect::<Vec<_>>();
                {
                    let em = entity_manager.as_mut().expect("Intro2 entity manager");
                    em.advance_environment_frame(&mut world_fx, elapsed_micros);
                    em.advance_unclaimed_actor_animations(elapsed_micros, &actor_animation_claims);
                }
                // FUN_0044FFA0 runs FUN_00413500 before FUN_00440120. The
                // captured type-26 owner therefore emits class-5 carriers in
                // this same cinematic pass so their FUN_0043E180 ordinary-
                // surface tail can write terrain bit 0x10 below.
                {
                    let em = entity_manager
                        .as_mut()
                        .expect("Intro2 loaded without an entity manager");
                    let specialized_pass = specialized_actor_tasks.tick(
                        em,
                        SpecializedActorTaskProductionFrame {
                            world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                            hive_components: Some(v2k_game::specialized_actor_task_production::HiveComponentProductionContext {
                                world_complete_tally: &mut world_complete_tally,
                                view_detail: RetailViewDetailContext::from_world(camera.position,
                                    camera_true_up_z, terrain_scan_dimensions),
                            }),
                            resources: &mut session.cache,
                            world_fx: &mut world_fx,
                            static_damage: &mut static_damage,
                            elapsed_micros,
                            global_elapsed_micros: elapsed_micros,
                            retail_tick,
                            notification_phase: v2k_game::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                            main_base_abort_active: main_base_level_abort.active,
                        },
                        &mut gameplay_notifications,
                    );
                    if let Some(block) = specialized_pass.block {
                        if !specialized_actor_task_error_reported {
                            eprintln!(
                                "Intro2 specialized actor task scheduler stopped at unresolved frame evidence: {block:?}"
                            );
                            specialized_actor_task_error_reported = true;
                        }
                    }
                    if specialized_pass.terrain_changed {
                        renderer.invalidate_terrain_cache();
                    }
                    if specialized_pass.progressive_death_presentation_requested {
                        main_base_level_abort.outer_frame_sequence.request();
                    }
                    static_damage_explosion_lights.extend(specialized_pass.explosion_lights);
                    for owner_outcome in specialized_pass.outcomes {
                        if let Some(message) = type9_task_diagnostics.observe(
                            &owner_outcome,
                            retail_tick,
                            |entity_id| {
                                em.iter_all()
                                    .find(|entity| entity.id == entity_id)
                                    .map(|entity| type9_task_diagnostics::Type9DiagnosticActor {
                                        active: entity.active,
                                        authored_spawn_index: entity.authored_spawn_index,
                                    })
                            },
                        ) {
                            eprintln!("{message}");
                        }
                        match &owner_outcome {
                            SpecializedActorTaskProductionOutcome::OrdinaryType9Wander(_)
                            | SpecializedActorTaskProductionOutcome::OrdinaryType9GoToJob(_)
                            | SpecializedActorTaskProductionOutcome::OrdinaryType9RunAway(_)
                            | SpecializedActorTaskProductionOutcome::OrdinaryType9AttractAttention(_) => {}
                            SpecializedActorTaskProductionOutcome::Intro2Meteor {
                                outcome,
                                death,
                            } => {
                                if let Some(death) = death {
                                    log!("Intro2 meteor death: {death:?}");
                                }
                                if matches!(
                                    outcome,
                                    v2k_game::intro2_meteors::Intro2MeteorOutcome::Blocked { .. }
                                ) && !specialized_actor_task_error_reported
                                {
                                    eprintln!("Intro2 meteor stopped: {outcome:?}");
                                    specialized_actor_task_error_reported = true;
                                }
                            }
                            SpecializedActorTaskProductionOutcome::Intro2Type53(
                                v2k_game::intro2_type53::Intro2Type53Outcome::Blocked { .. },
                            )
                            | SpecializedActorTaskProductionOutcome::NativeType122(
                                v2k_game::native_type122::Type122Outcome::Blocked { .. },
                            )
                            | SpecializedActorTaskProductionOutcome::NativeType30(
                                v2k_game::native_type30::Type30Outcome::Blocked { .. },
                            )
                            | SpecializedActorTaskProductionOutcome::NativeType40(
                                v2k_game::native_type40::Type40Outcome::Blocked { .. },
                            )
                            | SpecializedActorTaskProductionOutcome::NativeType56(
                                v2k_game::native_type56::Type56Outcome::Blocked { .. },
                            )
                            | SpecializedActorTaskProductionOutcome::SharedFish(
                                v2k_game::shared_fish::SharedFishOutcome::Blocked { .. },
                            )
                            | SpecializedActorTaskProductionOutcome::CleansingVehicle(
                                v2k_game::cleansing_vehicle::CleansingVehicleOutcome::Blocked { .. },
                            )
                            | SpecializedActorTaskProductionOutcome::Intro2Type16(
                                v2k_game::intro2_type16::Intro2Type16Outcome::Blocked { .. },
                            )
                            | SpecializedActorTaskProductionOutcome::Intro2Type58(
                                v2k_game::intro2_type58::Intro2Type58Outcome::Blocked { .. },
                            )
                            | SpecializedActorTaskProductionOutcome::Intro2Type94(
                                v2k_game::intro2_type94::Intro2Type94Outcome::Blocked { .. },
                            )
                            | SpecializedActorTaskProductionOutcome::Intro2Type66(
                                v2k_game::intro2_type66::Intro2Type66Outcome::Blocked { .. },
                            )
                            | SpecializedActorTaskProductionOutcome::Intro2Type10(
                                v2k_game::intro2_type10::Intro2Type10Outcome::Blocked { .. },
                            )
                            | SpecializedActorTaskProductionOutcome::Intro2Type8(
                                v2k_game::intro2_type8::Intro2Type8Outcome::Blocked { .. },
                            )
                            | SpecializedActorTaskProductionOutcome::Intro2GunTurret(
                                v2k_game::intro2_gun_turret::Intro2GunTurretOutcome::Blocked { .. },
                            )
                            | SpecializedActorTaskProductionOutcome::Intro2Type10Tumble(
                                v2k_game::intro2_type10::Intro2Type10TumbleOutcome::Blocked { .. },
                            )
                            | SpecializedActorTaskProductionOutcome::Intro2Type17(
                                v2k_game::intro2_type17::Intro2Type17Outcome::Blocked { .. },
                            )
                            | SpecializedActorTaskProductionOutcome::Intro2CommonDying(
                                v2k_game::intro2_common_dying::Intro2CommonDyingOutcome::Blocked {
                                    ..
                                },
                            )
                            | SpecializedActorTaskProductionOutcome::Intro2Type9Class14(
                                v2k_game::intro2_type9_class14::Intro2Type9Class14Outcome::Blocked {
                                    ..
                                },
                            )
                            | SpecializedActorTaskProductionOutcome::Intro2Type13SearchAttack(
                                v2k_game::intro2_type13_live::Intro2Type13WorldOutcome::Blocked { .. }
                                | v2k_game::intro2_type13_live::Intro2Type13WorldOutcome::Task {
                                    completed: false, ..
                                },
                            )
                            | SpecializedActorTaskProductionOutcome::Intro2Type47Scheduler(
                                v2k_game::intro2_type47_live::world::Intro2Type47WorldOutcome::Blocked { .. }
                                | v2k_game::intro2_type47_live::world::Intro2Type47WorldOutcome::Task {
                                    completed: false, ..
                                },
                            )
                            | SpecializedActorTaskProductionOutcome::Intro2FlyerScheduler(
                                v2k_game::intro2_flyers_live::Intro2FlyerSchedulerProductionOutcome::Blocked { .. },
                            ) if !specialized_actor_task_error_reported => {
                                eprintln!("Intro2 actor stopped at unresolved runtime evidence: {owner_outcome:?}");
                                specialized_actor_task_error_reported = true;
                            }
                            _ => {}
                        }
                        if let SpecializedActorTaskProductionOutcome::Intro2Type26(
                            v2k_game::intro2_type26_defecate_virus::Intro2Type26WorldOutcome::Blocked { .. },
                        ) = &owner_outcome {
                            if !specialized_actor_task_error_reported {
                                eprintln!("Intro2 Type26 stopped: {owner_outcome:?}");
                                specialized_actor_task_error_reported = true;
                            }
                        }
                    }
                }
                // 13500 closes its actor walk with 14990. Deaths queued by
                // the later particle/contact passes remain until this point
                // on the next frame, rather than disappearing before draw.
                entity_manager
                    .as_mut()
                    .expect("Intro2 entity manager")
                    .cleanup_pending_actor_deferred_destroys();
                // 44FFA0 runs the static program scheduler (374B0/281A0)
                // after actor callbacks and before eye/particle updates.
                static_damage.begin_advance(elapsed_micros);
                while let Some(batch) = static_damage.next_node_batch(|cell| {
                    resolve_current_static_damage_target(&session.cache, cell)
                        .ok()
                        .flatten()
                        .map(|target| target.state)
                }) {
                    let (token, actions) = batch.into_parts();
                    let radial_reports = apply_static_damage_actions(
                        &mut static_damage,
                        actions,
                        &mut session.cache,
                        entity_manager.as_mut().expect("Intro2 entity manager"),
                        StaticDamageWorldContext::Intro2 {
                            actor_tasks: &mut specialized_actor_tasks,
                            notifications: &mut gameplay_notifications,
                            retail_tick,
                        },
                        &mut world_fx,
                        &mut static_damage_explosion_lights,
                        intro_camera.focus_position_raw(),
                        &mut main_base_level_abort.outer_frame_sequence,
                    );
                    for report in radial_reports {
                        log!("Intro2 static radial: {report:?}");
                        let completed = match &report {
                            v2k_game::intro2_radial::Intro2RadialReport::Applied {
                                dynamic,
                                ..
                            } => dynamic.completed(),
                            v2k_game::intro2_radial::Intro2RadialReport::StaticLookupBlocked(_) => {
                                false
                            }
                        };
                        if !completed && !specialized_actor_task_error_reported {
                            eprintln!("Intro2 static radial stopped: {report:?}");
                            specialized_actor_task_error_reported = true;
                        }
                    }
                    let completed = static_damage.complete_node_batch(token);
                    debug_assert!(completed, "applied static node remains registered");
                }
                if session.cache.take_level_terrain_presentation_dirty() {
                    renderer.invalidate_terrain_cache();
                }
                // ED10 precedes physical particles and active contacts. Its
                // current eye is retained even if a later contact moves or
                // destroys the Section-2 camera subject.
                advance_opening_camera_eye(
                    &session.cache,
                    &mut camera,
                    &mut intro_camera,
                    elapsed_micros,
                    config.active_camera,
                );
                let effects = v2k_game::intro2_effects::update_intro2_effects(
                    v2k_game::intro2_effects::Intro2EffectsFrame {
                        cache: &mut session.cache,
                        entities: entity_manager.as_mut().expect("Intro2 entity manager"),
                        world_fx: &mut world_fx,
                        static_damage: &mut static_damage,
                        scheduler: &mut specialized_actor_tasks,
                        notifications: &mut gameplay_notifications,
                        elapsed_micros,
                        retail_tick,
                    },
                );
                for &entity_id in &effects.unresolved_collision_entity_ids {
                    report_unresolved_particle_collision(
                        entity_id,
                        &mut particle_collision_error_reported,
                    );
                }
                for delivery in &effects.entity_deliveries {
                    log!("Intro2 impact: {delivery:?}");
                }
                let outcome = effects.particles;
                debug_assert!(outcome.unhandled_ground_programs.is_empty());
                let em = entity_manager
                    .as_mut()
                    .expect("Intro2 loaded without an entity manager");
                for report in v2k_game::intro2_contacts::resolve_intro2_contacts(
                    v2k_game::intro2_contacts::Intro2ContactFrame {
                        entities: em,
                        resources: &mut session.cache,
                        world_fx: &mut world_fx,
                        static_damage: &mut static_damage,
                        notifications: &mut gameplay_notifications,
                        retail_tick,
                        actor_tasks: &mut specialized_actor_tasks,
                    },
                ) {
                    use v2k_game::intro2_contacts::Intro2ContactReport;
                    use v2k_game::intro2_type58::contact::Intro2Type58ContactOutcome;
                    match report {
                        Intro2ContactReport::NativeFlyingSurface {
                            result: v2k_game::native_flying_surface_contact::NativeFlyingSurfaceContactOutcome::Ineligible
                                | v2k_game::native_flying_surface_contact::NativeFlyingSurfaceContactOutcome::Applied {
                                    solid_contact: false, water_entry: false, ..
                                }, ..
                        } => {}
                        Intro2ContactReport::NativeSurface {
                            result: v2k_game::native_actor_surface_contact::NativeActorSurfaceContactOutcome::Ineligible
                                | v2k_game::native_actor_surface_contact::NativeActorSurfaceContactOutcome::Applied {
                                    solid_contact: false, water_entry: false, ..
                                }, ..
                        } => {}
                        Intro2ContactReport::Type17Pair {
                            result: v2k_game::intro2_type17::pair::Type17PairOutcome::Ineligible, ..
                        } => {}
                        Intro2ContactReport::Type17Pair {
                            result: v2k_game::intro2_type17::pair::Type17PairOutcome::Resolved { ref visits }, ..
                        } if visits.is_empty() => {}
                        Intro2ContactReport::Type10 {
                            result:
                                v2k_game::intro2_type10::contact::Intro2Type10ContactOutcome::Ineligible
                                | v2k_game::intro2_type10::contact::Intro2Type10ContactOutcome::Applied(
                                    v2k_game::intro2_type10::contact::Intro2Type10ContactReport {
                                        terrain_contact: false,
                                        water_entry: false,
                                        static_contact: None,
                                        terminal: None,
                                        ..
                                    },
                                ),
                            ..
                        } => {}
                        Intro2ContactReport::Flyer {
                            result:
                                v2k_game::intro2_flyer_contacts::Intro2FlyerContactOutcome::Ineligible
                                | v2k_game::intro2_flyer_contacts::Intro2FlyerContactOutcome::Applied {
                                    solid_contact: false,
                                    water_entry: false,
                                    ..
                                },
                            ..
                        } => {}
                        Intro2ContactReport::Meteor {
                            result: Ok(None), ..
                        }
                        | Intro2ContactReport::Type58 {
                            result:
                                Intro2Type58ContactOutcome::Ineligible
                                | Intro2Type58ContactOutcome::Miss,
                            ..
                        }
                        | Intro2ContactReport::InsectStatic {
                            result: v2k_game::native_ground_actor::contact::NativeGroundContactOutcome::Ineligible
                                | v2k_game::native_ground_actor::contact::NativeGroundContactOutcome::Miss,
                            ..
                        }
                        | Intro2ContactReport::Type122 {
                            result:
                                v2k_game::native_type122::contact::Type122ContactOutcome::Ineligible
                                | v2k_game::native_type122::contact::Type122ContactOutcome::Miss,
                            ..
                        } => {}
                        other => log!("Intro2 contact: {other:?}"),
                    }
                }
                if session.cache.take_level_terrain_presentation_dirty() {
                    renderer.invalidate_terrain_cache();
                }
                world_fx.process_pending();

                // Retail's Intro2 clock, actors, effects, and hidden camera
                // all run while Klaus's authored geometry covers the world.
                // Keep advancing that state throughout the opening morph.
                intro_camera.follow_subject(
                    intro_commands.camera_subject(em),
                    gameplay_chase_terrain(&session.cache),
                );
                // 493FA0 simulation precedes 493F50 presentation. The latter
                // reaches 53410 before 53760's actor detail/FIFO traversal,
                // so a terminal 56750 request is visible in this same frame.
                let prepared_full_frame_sprite = prepare_full_frame_sprite_command(
                    &*renderer,
                    &session.cache,
                    &mut main_base_level_abort.outer_frame_sequence,
                );
                // FUN_0044FFA0 advances ED10 before the presentation pass.
                // Type-13's next 12DA0 mode must use this frame's camera,
                // including the hidden Intro2 passes behind Klaus.
                if intro2_backdrop(retail_tick) == Intro2Backdrop::World {
                    let _ = v2k_game::opening::publish_intro2_actor_view_detail(
                        em,
                        RetailViewDetailContext::from_world(
                            camera.position,
                            camera.view_matrix()[9],
                            terrain_scan_dimensions,
                        ),
                    );
                }
                let mut play_caption_type_sound = false;
                if stage.is_visible() {
                    // FUN_00452790 visits every active Section-2 record in
                    // table order. Inclusive adjacent windows overlap for one
                    // tick; a complete older record can clear the shared
                    // cadence before the new record cues in the same pass.
                    for caption in active_captions(retail_tick) {
                        let (_, revealing) = caption.revealed_text(retail_tick);
                        if text_typewriter_cadence
                            .observe_active_line(revealing, retail_tick as i32)
                        {
                            play_caption_type_sound = true;
                        }
                    }
                }
                let wrapped = match stage.covered_shell() {
                    Some(shell) => {
                        let (billboard_sprite_id, wrapped) =
                            billboard.prepare_cinematic_frame(elapsed_micros);
                        render_opening_cinematic(
                            &mut *renderer,
                            &session.cache,
                            &face_colors,
                            em,
                            &camera,
                            *elapsed,
                            menu_fonts.as_ref(),
                            sky_background.as_ref(),
                            terrain_frames.as_ref(),
                            water_frames.as_ref(),
                            OpeningWorldEffects {
                                terrain_lights: &mut terrain_lights,
                                world_fx: &mut world_fx,
                                explosion_lights: &static_damage_explosion_lights,
                                full_frame_sprite: prepared_full_frame_sprite.as_ref(),
                                native_viewport: intro_camera.native_viewport(),
                                world_projection: Some(world_projection),
                            },
                            &menu_resources,
                            billboard_sprite_id,
                            retail_tick,
                            elapsed_micros,
                            OpeningCinematicPresentation::KlausCover(KlausHandoffFrame {
                                shell,
                                cache: &session.cache,
                                colors: &face_colors,
                                retail_tick,
                            }),
                        );
                        wrapped
                    }
                    None => {
                        let (billboard_sprite_id, wrapped) =
                            billboard.prepare_cinematic_frame(elapsed_micros);
                        render_opening_cinematic(
                            &mut *renderer,
                            &session.cache,
                            &face_colors,
                            em,
                            &camera,
                            *elapsed,
                            menu_fonts.as_ref(),
                            sky_background.as_ref(),
                            terrain_frames.as_ref(),
                            water_frames.as_ref(),
                            OpeningWorldEffects {
                                terrain_lights: &mut terrain_lights,
                                world_fx: &mut world_fx,
                                explosion_lights: &static_damage_explosion_lights,
                                full_frame_sprite: prepared_full_frame_sprite.as_ref(),
                                native_viewport: intro_camera.native_viewport(),
                                world_projection: Some(world_projection),
                            },
                            &menu_resources,
                            billboard_sprite_id,
                            retail_tick,
                            elapsed_micros,
                            OpeningCinematicPresentation::Story,
                        );
                        wrapped
                    }
                };
                // 537F0's emblem precedes 453AA0 audio, then 52CB0 commands.
                if wrapped {
                    GlobalSoundRuntime::new(&session.cache, &mut world_fx, sound_manager.as_mut())
                        .play_global_sound(v2k_game::menu_engine::SOUND_BG_RUMBLE as usize);
                }
                play_world_audio(
                    WorldAudioFrame {
                        cache: &session.cache,
                        entities: em,
                        camera: &camera,
                        elapsed_micros,
                    },
                    &mut world_fx,
                    &mut sound_manager,
                    &mut entity_positional_audio,
                );
                // 52CB0 consumes Section-2 commands after model and effect presentation.
                intro_commands.present(em, retail_tick);
                if play_caption_type_sound {
                    if let Some(sound_manager) = sound_manager.as_mut() {
                        sound_manager
                            .play_centered_sound_with_gain(GAMEPLAY_TEXT_TYPE_SOUND_ID, 0.5);
                    }
                }

                // 503C0 writes world+290 after this frame's simulation.
                // 44FFA0 consumes that latch on the next visit, before actors.
                *exit_pending = skip || intro2_finished(retail_tick);
            }

            GameState::PostIntro { shell } => {
                retail_tick = retail_clock.advance(u64::from(elapsed_micros));
                for event in &events {
                    match event {
                        GameEvent::Quit => break 'main_loop,
                        GameEvent::Resize(w, h) => {
                            renderer.resize(*w, *h);
                            world_projection.apply_to(&mut camera, renderer.viewport_size());
                        }
                        _ => {}
                    }
                }
                shell.update(
                    elapsed_micros,
                    &mut GlobalSoundRuntime::new(
                        &session.cache,
                        &mut world_fx,
                        sound_manager.as_mut(),
                    ),
                );
                // 4D08C8 -> 4D0710 runs 50C00: only Klaus, ED10 and
                // physical particles continue while the world remains loaded.
                // The class-1 proxy, ordinary actors, static programs and
                // active-pair contacts do not advance in this transition.
                advance_opening_camera_eye(
                    &session.cache,
                    &mut camera,
                    &mut intro_camera,
                    elapsed_micros,
                    config.active_camera,
                );
                let effects = v2k_game::intro2_effects::update_intro2_effects(
                    v2k_game::intro2_effects::Intro2EffectsFrame {
                        cache: &mut session.cache,
                        entities: entity_manager
                            .as_mut()
                            .expect("retained Intro2 entity manager"),
                        world_fx: &mut world_fx,
                        static_damage: &mut static_damage,
                        scheduler: &mut specialized_actor_tasks,
                        notifications: &mut gameplay_notifications,
                        elapsed_micros,
                        retail_tick,
                    },
                );
                for &entity_id in &effects.unresolved_collision_entity_ids {
                    report_unresolved_particle_collision(
                        entity_id,
                        &mut particle_collision_error_reported,
                    );
                }
                for delivery in &effects.entity_deliveries {
                    log!("Intro2 closing impact: {delivery:?}");
                }
                if session.cache.take_level_terrain_presentation_dirty() {
                    renderer.invalidate_terrain_cache();
                }
                world_fx.process_pending();
                // Before load, the completed Intro2 black card stays resident.
                renderer.begin_scene(RenderScene::World);
                renderer.clear(0.0, 0.0, 0.0);
                draw_klaus_handoff_overlay(
                    &mut *renderer,
                    KlausHandoffFrame {
                        shell,
                        cache: &session.cache,
                        colors: &face_colors,
                        retail_tick,
                    },
                );
                play_world_audio(
                    WorldAudioFrame {
                        cache: &session.cache,
                        entities: entity_manager
                            .as_mut()
                            .expect("retained Intro2 entity manager"),
                        camera: &camera,
                        elapsed_micros,
                    },
                    &mut world_fx,
                    &mut sound_manager,
                    &mut entity_positional_audio,
                );
                renderer.present();
                if shell.take_transition_ready() {
                    // Keep --level useful as a direct world override
                    // for development; its default is retail level 13.
                    // An explicit 50 still advances to 13 instead of
                    // recursively replaying Intro2.
                    let first_world = if level_id == INTRO2_LEVEL_ID {
                        FIRST_WORLD_LEVEL_ID
                    } else {
                        level_id
                    };
                    let ctx = MenuCtx {
                        cache: &session.cache,
                        config: &config,
                        saves: Some(&save_manager),
                    };
                    let carried_shell = std::mem::replace(shell, MenuShell::new_post_intro(&ctx));
                    state = GameState::Loading {
                        level_id: first_world,
                        purpose: LoadingPurpose::ResumePostIntro {
                            shell: carried_shell,
                        },
                    };
                }
            }

            GameState::Playing => {
                // Craft feel constants now live in v2k_game::player (the two
                // integrators), keyed to the RE'd reader/integrator anchors.

                // Apply window geometry before a same-batch Escape can suspend
                // gameplay. Invalidate its underlay as well: an FBO resize can
                // discard the old image before pause captures it.
                let mut resized_before_pause = false;
                for event in &events {
                    if let GameEvent::Resize(w, h) = event {
                        renderer.resize(*w, *h);
                        world_projection.apply_to(&mut camera, renderer.viewport_size());
                        resized_before_pause = true;
                    }
                }

                // Descriptor 004D0AA0 owns a modal progress map after the
                // exit. Gameplay no longer runs while its input table waits.
                if world_complete_results.is_progress_map_active() {
                    for event in &events {
                        match event {
                            GameEvent::Quit => break 'main_loop,
                            GameEvent::KeyDown(Keycode::S) => {
                                let checkpoint = (|| -> Result<_, String> {
                                    let route = world_complete_results
                                        .progress_map_route()
                                        .ok_or("campaign map route unavailable")?;
                                    let manager = entity_manager
                                        .as_ref()
                                        .ok_or("player manager unavailable")?;
                                    let player = manager.player().ok_or("player unavailable")?;
                                    let v2k_game::entity_collision_state::RetailRuntimeValue::Known(
                                        cargo,
                                    ) = manager.campaign_cargo_controller_state()
                                    else {
                                        return Err("cargo snapshot unavailable".into());
                                    };
                                    // 4D0568's low words are153..188 for logical1..36;
                                    // 451FB0 uses only that half of the name/briefing pair.
                                    let name = session
                                        .cache
                                        .global_string(
                                            152 + route.destination_logical_level() as usize,
                                        )
                                        .or_else(|| session.cache.global_string(128))
                                        .ok_or("checkpoint name unavailable")?;
                                    let snapshot = v2k_game::save::NativeSaveSnapshot {
                                        logical_level_id: route.destination_logical_level(),
                                        player: SavedPlayerState {
                                            position_raw: player.position_raw(),
                                            velocity_raw: player.velocity_raw(),
                                            heading_raw: player.heading_raw(),
                                            pitch_raw: 0,
                                            roll_raw: 0,
                                            health_raw: player_hull.health_raw,
                                        },
                                        craft: &player_craft,
                                        hull: &player_hull,
                                        inventory: &weapon_inventory,
                                        capabilities: &player_capabilities,
                                        cargo: &cargo,
                                        campaign: &player_campaign_progress,
                                    };
                                    snapshot
                                        .encode_campaign_arrival(
                                            &save_baseline,
                                            route.arrival(),
                                            name,
                                        )
                                        .map_err(|error| format!("checkpoint snapshot: {error:?}"))
                                })();
                                match checkpoint {
                                    Ok(checkpoint) => {
                                        save_baseline = checkpoint.state_payload;
                                        map_save_checkpoint = Some(checkpoint);
                                        keys_down.clear();
                                        mouse_buttons_down.clear();
                                        paused_world_frame =
                                            renderer.capture_frame(FrameCaptureSource::Presented);
                                        paused_world_redraw_pending = resized_before_pause;
                                        let ctx = MenuCtx {
                                            cache: &session.cache,
                                            config: &config,
                                            saves: Some(&save_manager),
                                        };
                                        state = GameState::Paused { shell: MenuShell::new_single_player_menu(
                                            &ctx, v2k_game::game_state::SinglePlayerMenuKind::CampaignSave,
                                        ) };
                                        GlobalSoundRuntime::new(
                                            &session.cache,
                                            &mut world_fx,
                                            sound_manager.as_mut(),
                                        )
                                        .play_global_sound(3);
                                        continue 'main_loop;
                                    }
                                    Err(error) => eprintln!("Campaign save blocked: {error}"),
                                }
                            }
                            GameEvent::KeyDown(Keycode::Space) => {
                                if let Some(route) = world_complete_results.continue_progress_map()
                                {
                                    GlobalSoundRuntime::new(
                                        &session.cache,
                                        &mut world_fx,
                                        sound_manager.as_mut(),
                                    )
                                    .play_global_sound(v2k_game::world_complete_results::FUN_00455CC0_CONTINUE_SOUND_ID);
                                    keys_down.clear();
                                    mouse_buttons_down.clear();
                                    state = GameState::Loading {
                                        level_id: route.destination_level_id(),
                                        purpose: LoadingPurpose::CampaignWarp {
                                            source_level_id: route.source_level_id(),
                                            arrival_position_raw: route.arrival().position_raw,
                                            arrival_heading_raw: route.arrival().heading_raw,
                                        },
                                    };
                                    continue 'main_loop;
                                }
                            }
                            GameEvent::FocusLost => {
                                keys_down.clear();
                                mouse_buttons_down.clear();
                            }
                            _ => {}
                        }
                    }
                    world_complete_results.advance(elapsed_micros);
                    renderer.begin_scene(RenderScene::Menu);
                    renderer.clear(0.0, 0.0, 0.0);
                    if let (Some(backdrop), Some(fonts)) =
                        (overlay_51_backdrop.as_ref(), menu_fonts.as_ref())
                    {
                        draw_overlay_51_backdrop(
                            &mut *renderer,
                            fonts,
                            backdrop,
                            &player_campaign_progress,
                            world_complete_results.age_ms(),
                        );
                        if let Some(prompt) = world_complete_results
                            .continue_prompt_line(|id| session.cache.global_string(id))
                        {
                            draw_gameplay_notifications(
                                &mut *renderer,
                                fonts,
                                &[prompt],
                                GameplayTextContext::AuthoredCanvas,
                            );
                        }
                    }
                    renderer.present();
                    continue 'main_loop;
                }

                // The M screen is a modal table layered over state 5 in
                // retail, not a separate game-state value. Its private clock
                // advances while world simulation, the process tick, and held
                // vehicle controls remain frozen.
                if gameplay_radar
                    .as_ref()
                    .is_some_and(GameplayRadar::is_fullscreen_open)
                {
                    let mut close_map = false;
                    for event in &events {
                        match event {
                            GameEvent::Quit => break 'main_loop,
                            GameEvent::KeyDown(Keycode::M) => {
                                if keys_down.insert(Keycode::M) {
                                    close_map = true;
                                }
                            }
                            GameEvent::KeyUp(Keycode::M) => {
                                keys_down.remove(&Keycode::M);
                            }
                            GameEvent::FocusLost => {
                                keys_down.clear();
                                mouse_buttons_down.clear();
                            }
                            _ => {}
                        }
                    }
                    let radar = gameplay_radar.as_mut().expect("checked above");
                    if close_map {
                        radar.leave_fullscreen();
                        mouse_buttons_down.clear();
                        continue 'main_loop;
                    }
                    radar.advance_fullscreen(elapsed_micros);
                    if let (Some(entities), Some(terrain)) =
                        (entity_manager.as_ref(), session.cache.level_terrain_radar())
                    {
                        draw_fullscreen_map(
                            &mut *renderer,
                            radar,
                            FullscreenMapDrawRequest {
                                terrain,
                                entities,
                                fonts: menu_fonts.as_ref(),
                                status: fullscreen_map_status.as_ref(),
                                progress: &player_campaign_progress,
                            },
                            &mut || world_fx.next_shared_retail_random_u16(),
                        );
                    } else {
                        renderer.begin_scene(RenderScene::Menu);
                        renderer.clear(0.0, 0.0, 0.0);
                    }
                    renderer.present();
                    continue 'main_loop;
                }

                let open_map = events
                    .iter()
                    .any(|event| matches!(event, GameEvent::KeyDown(Keycode::M)))
                    && !keys_down.contains(&Keycode::M);
                if open_map && !events.iter().any(|event| matches!(event, GameEvent::Quit)) {
                    if let (Some(radar), Some(entities), Some(terrain)) = (
                        gameplay_radar.as_mut(),
                        entity_manager.as_ref(),
                        session.cache.level_terrain_radar(),
                    ) {
                        keys_down.clear();
                        keys_down.insert(Keycode::M);
                        mouse_buttons_down.clear();
                        radar.enter_fullscreen();
                        radar.advance_fullscreen(elapsed_micros);
                        draw_fullscreen_map(
                            &mut *renderer,
                            radar,
                            FullscreenMapDrawRequest {
                                terrain,
                                entities,
                                fonts: menu_fonts.as_ref(),
                                status: fullscreen_map_status.as_ref(),
                                progress: &player_campaign_progress,
                            },
                            &mut || world_fx.next_shared_retail_random_u16(),
                        );
                        renderer.present();
                        continue 'main_loop;
                    }
                }

                // Resolve mode changes before borrowing or ticking any live
                // world state. Escape owns only a pause shell; the entity
                // manager, current level cache, effects and audio remain live.
                match gameplay_interruption(
                    std::mem::take(&mut return_to_frontend_pending),
                    &events,
                ) {
                    Some(GameplayInterruption::QuitApplication) => break 'main_loop,
                    Some(GameplayInterruption::ReturnToFrontend) => {
                        state = GameState::ReturningToFrontend;
                        continue;
                    }
                    Some(GameplayInterruption::Pause) => {
                        suspend_world_audio(
                            &mut world_fx,
                            &mut sound_manager,
                            &mut entity_positional_audio,
                            &mut player_fan_audio,
                        );
                        let ctx = MenuCtx {
                            cache: &session.cache,
                            config: &config,
                            saves: Some(&save_manager),
                        };
                        keys_down.clear();
                        mouse_buttons_down.clear();
                        paused_world_frame = renderer.capture_frame(FrameCaptureSource::Presented);
                        paused_world_redraw_pending = resized_before_pause;
                        paused_gameplay_scene = entity_manager.as_ref().map(|entities| {
                            PausedGameplayScene::capture(
                                v2k_game::frozen_world_presentation::FrozenWorldPresentationSource {
                                    entities, world_fx: &world_fx,
                                    specialized_actor_tasks: &specialized_actor_tasks,
                                },
                                targetter, retail_tick,
                            )
                        });
                        state = GameState::Paused {
                            shell: MenuShell::new_single_player_menu(
                                &ctx,
                                v2k_game::game_state::SinglePlayerMenuKind::Pause,
                            ),
                        };
                        continue;
                    }
                    None => {}
                }

                // This legacy Playing adapter still stages 53410 before
                // simulation. Retail's 493FA0 -> 493F50 order is the reverse;
                // Intro2 uses that proven boundary. Moving all gameplay
                // producers to the same-frame consumer remains separate work.
                let prepared_full_frame_sprite = prepare_full_frame_sprite_command(
                    &*renderer,
                    &session.cache,
                    &mut main_base_level_abort.outer_frame_sequence,
                );

                retail_tick = retail_clock.advance(u64::from(elapsed_micros));
                world_complete_results.advance(elapsed_micros);
                if let Some(shell) = post_intro_world_cover.as_mut() {
                    // The persistent Klaus task precedes loaded world actors.
                    shell.update(
                        elapsed_micros,
                        &mut GlobalSoundRuntime::new(
                            &session.cache,
                            &mut world_fx,
                            sound_manager.as_mut(),
                        ),
                    );
                }
                // FUN_0042D9B0 is owned by the same world controller as the
                // Main Base/results state at +0x1F0. This point is after every
                // pause/map early exit, so only active gameplay spends the
                // signed millisecond countdown.
                if let Some(sound) = main_base_level_abort.advance_time_trophy(elapsed_micros) {
                    if let Some(ref mut sm) = sound_manager {
                        sm.play_centered_sound_with_rate_q16(
                            usize::from(sound.sound_id),
                            sound.rate_q16,
                        );
                    }
                }
                world_fx.advance_frame_pacing(uncapped_elapsed_micros);
                if entity_manager
                    .as_ref()
                    .and_then(EntityManager::player)
                    .is_some()
                {
                    player_shield.advance_player_callback(elapsed_micros);
                }

                // Keep the exact entry claims through every later legacy
                // pass. A specialized owner which terminates or drops during
                // FUN_00413500 must not receive a second callback through a
                // manager-wide compatibility path in the same retail frame.
                let mut actor_animation_claims = specialized_actor_tasks
                    .actor_animation_claims()
                    .collect::<Vec<_>>();
                let type66_progression_claims = specialized_actor_tasks
                    .main_base_type66_actor_claims()
                    .collect::<Vec<_>>();
                let em = entity_manager.as_mut().unwrap();
                em.advance_environment_frame(&mut world_fx, elapsed_micros);
                let mut pending_hive_death_effects = None;
                let mut static_damage_explosion_lights = Vec::new();

                for event in &events {
                    match event {
                        GameEvent::Quit => break 'main_loop,
                        GameEvent::Resize(_, _) => {}
                        GameEvent::KeyDown(kc) => {
                            // TAB = hovercraft <-> VTOL mode toggle (V2000).
                            // SDL repeat events are filtered by GameWindow, so
                            // this is one request per physical key-down edge.
                            // Retail briefly installs VTOL even with an empty
                            // tank, then FUN_00445310 immediately queues event
                            // 0x0B/sound 0x31 and requests Hover again. The port
                            // collapses that short-lived mode round trip into the
                            // explicit RefusedNoFuel outcome below.
                            if *kc == Keycode::Tab && has_player && !player_hull.dying {
                                match player_craft.toggle_mode() {
                                    VehicleModeToggleOutcome::Changed(mode) => {
                                        log!("Vehicle mode: {}", mode.label());
                                    }
                                    VehicleModeToggleOutcome::RefusedNoFuel => {
                                        gameplay_notifications.queue_fuel_empty(retail_tick as i32);
                                        if let Some(position_raw) =
                                            em.player().map(|player| player.position_raw())
                                        {
                                            // FUN_00445310 presents global
                                            // sound 49 from the craft origin.
                                            world_fx.queue_fixed_positional_sound_raw(
                                                FUEL_EXHAUSTED_SOUND_ID,
                                                position_raw,
                                            );
                                        }
                                        log!("Vehicle mode: Hover (no fuel)");
                                    }
                                }
                            }
                            // F11 = dev free-fly camera toggle.
                            if *kc == Keycode::F11 && has_player && !player_hull.dying {
                                chase_mode = !chase_mode;
                                log!("Camera: {}", if chase_mode { "chase" } else { "free-fly" });
                            }
                            let input_edges = gameplay_key_transition(&mut keys_down, *kc, true);
                            apply_gameplay_input_edges(
                                input_edges,
                                has_player,
                                player_hull.dying,
                                em,
                                &mut weapon_inventory,
                                &mut player_craft,
                            );
                        }
                        GameEvent::KeyUp(kc) => {
                            let input_edges = gameplay_key_transition(&mut keys_down, *kc, false);
                            apply_gameplay_input_edges(
                                input_edges,
                                has_player,
                                player_hull.dying,
                                em,
                                &mut weapon_inventory,
                                &mut player_craft,
                            );
                        }
                        GameEvent::MouseButtonDown(button) => {
                            mouse_buttons_down.insert(*button);
                        }
                        GameEvent::MouseButtonUp(button) => {
                            mouse_buttons_down.remove(button);
                        }
                        GameEvent::FocusLost => {
                            keys_down.clear();
                            mouse_buttons_down.clear();
                        }
                        GameEvent::MouseMotion { xrel, yrel } => {
                            if !chase_mode {
                                // Live-world free-fly keeps gameplay's left-
                                // handed view, so mouse-right must increase
                                // screen-right rather than world +X.
                                let yaw_delta = *xrel as f32 * mouse_sensitivity;
                                camera.yaw += if camera.left_handed {
                                    -yaw_delta
                                } else {
                                    yaw_delta
                                };
                                camera.pitch -= *yrel as f32 * mouse_sensitivity;
                                camera.pitch = camera.pitch.clamp(-1.4, 1.4);
                            }
                        }
                        _ => {}
                    }
                }

                gameplay_hud_elapsed_micros =
                    advance_retail_bar_clock(gameplay_hud_elapsed_micros, elapsed_micros);
                gameplay_hud.advance_spins(elapsed_micros);
                let drop_context = em.player().and_then(|player| {
                    session.cache.terrain().map(|terrain| CargoDropContext {
                        carrier_orientation: player_model_orientation(&player_craft, player),
                        terrain,
                    })
                });
                let mut cargo_full_fx = None;
                let cargo_frame = em.update_player_cargo(PlayerCargoFrame {
                    elapsed_micros,
                    drop_context,
                    retail_tick,
                    scheduler: &mut specialized_actor_tasks,
                    world_fx: &mut world_fx,
                    notifications: &mut gameplay_notifications,
                });
                for (entity_id, reason) in &cargo_frame.blocked {
                    log!("Cargo behavior {entity_id} blocked: {reason}");
                }
                if let Some(outcome) = cargo_frame.beam {
                    log!("Cargo beam: {outcome:?}");
                    match outcome {
                        BeamOutcome::CargoFull => {
                            cargo_full_fx = em.player().map(|player| player.position);
                            gameplay_notifications.queue_cargo_full(retail_tick as i32);
                        }
                        _ => {}
                    }
                }

                // FUN_0044FFA0 runs the complete FUN_00413500 actor-task pass
                // before its later FUN_00411A80 entity/contact walker. This
                // ordering is material for Type 54's live sea word and for
                // the shared RNG consumed by a later hard-water Type-60
                // constructor. Owners published by that water response or by
                // the still-later Main Base sweep are therefore newborn until
                // the following frame.
                let type8_adopted = specialized_actor_tasks.adopt_intro2_type8(em)
                    + specialized_actor_tasks.adopt_live_type8_go_to_job(em);
                if type8_adopted != 0 {
                    log!(
                        "Type-8: adopted {type8_adopted} new worker scheduler owner{}",
                        if type8_adopted == 1 { "" } else { "s" }
                    );
                }
                // Dynamic four-choice workers publish after the preceding
                // actor pass. Adopt their exact birth/carrying graph only at
                // this next-frame boundary, before animation claims and ticks.
                let _ = specialized_actor_tasks.adopt_native_type86(em);
                let _ = specialized_actor_tasks.adopt_level_one_factory_arrival(em);
                // Retain entry claims and include both cargo transfers and
                // workers first adopted this frame before any task can run.
                // The later neutral pass must not advance Sub-I after either
                // a detailed callback or a coarse callback's deliberate skip.
                for claim in specialized_actor_tasks.actor_animation_claims() {
                    if !actor_animation_claims.contains(&claim) {
                        actor_animation_claims.push(claim);
                    }
                }
                let specialized_pass = specialized_actor_tasks.tick(
                    em,
                    SpecializedActorTaskProductionFrame {
                        world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Playing {
                            player_hull: &mut player_hull,
                            extra_lives: RetailRuntimeValue::Known(player_campaign_progress.extra_lives()),
                        },
                        hive_components: Some(v2k_game::specialized_actor_task_production::HiveComponentProductionContext {
                            world_complete_tally: &mut world_complete_tally,
                            view_detail: RetailViewDetailContext::from_world(camera.position,
                                camera.view_matrix()[9], terrain_frames.as_ref()
                                    .map(|frames| (frames.scan_columns, frames.scan_rows))
                                    .or_else(|| session.cache.level_desc().map(|level|
                                        v2k_render::terrain_tiles::scan_dimensions(level.terrain_draw_depth)))
                                    .unwrap_or((52, 30))),
                        }),
                        resources: &mut session.cache,
                        world_fx: &mut world_fx,
                        static_damage: &mut static_damage,
                        elapsed_micros,
                        global_elapsed_micros: elapsed_micros,
                        retail_tick,
                        notification_phase:
                            v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
                        main_base_abort_active: main_base_level_abort.active,
                    },
                    &mut gameplay_notifications,
                );

                // Live world uses V2000's +Z-forward, +X-screen-right camera.
                // Free-fly only detaches the chase spring; it keeps the same
                // view basis so F11 does not mirror left/right. Menus
                // never reach this write.
                camera.left_handed = true;

                // E870 visits the already-published timed object before A800
                // moves the body. A death created later in this frame begins
                // at the final entry boundary and receives its first dt here
                // on the following controlled visit.
                if player_death.is_active() {
                    match player_death.advance_micros(elapsed_micros) {
                        PlayerDeathTick::ExpiredNow { .. } => {
                            em.remove_player_and_attached_entities();
                            has_player = false;
                            chase_mode = false;
                            return_to_frontend_pending = true;
                            continue 'main_loop;
                        }
                        PlayerDeathTick::Inactive | PlayerDeathTick::Dying { .. } => {}
                    }
                }

                if chase_mode {
                    // FUN_00446640 snapshots entity +0x30 at callback entry;
                    // the later solid-contact pass cannot affect its warning
                    // decision until the following controlled-player tick.
                    let hull_health_at_controller_entry_raw = player_hull.health_raw;
                    // V2000 vehicle controls: one reader collapses the held keys
                    // into the motion channels; the per-mode integrator (hover
                    // gun-aim + throttle propulsion / fly tilt-to-fly + ascent)
                    // turns them into velocity/heading. See v2k_game::player.
                    let sensitivity = (config.sensitivity * 15.0).round().clamp(0.0, 15.0) as u8;
                    let channels = if player_hull.dying {
                        MotionChannels::default()
                    } else {
                        MotionChannels::from_keys_with_sensitivity(
                            &keys_down,
                            elapsed_micros,
                            sensitivity,
                        )
                    };
                    let frame_forces = em.player_mut().map(|player| {
                        if player_hull.dying {
                            return VehicleFrameForces::DyingCommon;
                        }
                        player_craft.integrate_configured_micros_with_sensitivity(
                            elapsed_micros,
                            &channels,
                            player,
                            config.self_righting,
                            sensitivity,
                        )
                    });
                    // FUN_00420920 creates and retunes both persistent fan
                    // layers inside the controlled-player callback, before
                    // entity force/contact integration moves the emitter.
                    if let (Some(sound_manager), Some(player)) = (
                        sound_manager.as_mut().filter(|_| !player_hull.dying),
                        em.player(),
                    ) {
                        player_fan_audio.update(
                            sound_manager,
                            player_craft.fan_sound_frame(),
                            player.position,
                            SoundListener::new(camera.position, camera.right()),
                        );
                    }
                    // The empty-fuel cue is sourced at callback entry, before
                    // common gravity/contact can move the craft. Retain that
                    // signed-word position until the callback's delayed mode
                    // request is consumed below.
                    let fuel_exhaustion_position_raw = if frame_forces
                        .as_ref()
                        .is_some_and(|frame| frame.vtol_started_without_fuel())
                    {
                        em.player().map(|player| player.position_raw())
                    } else {
                        None
                    };

                    // 11AD0 retains the entry model and eligibility through all
                    // terrain/water/static suffixes, even if a callback changes
                    // the live allocation to its wreck model and Dying style.
                    let player_contact_model_id = em
                        .player()
                        .and_then(|player| player.model_index.or(player_model_id));
                    let mut player_contact_anim_vars = player_craft.anim_vars();
                    if player_hull.dying {
                        player_contact_anim_vars.dynamic[7] =
                            i32::from(player_death.control_output_7_raw());
                    }
                    let player_static_entry_eligible = em.player().is_some_and(|player| {
                        let radius = player_contact_model_id
                            .and_then(|id| session.cache.global_model(id))
                            .map_or(0, |model| model.collision_radius_raw);
                        v2k_game::static_contact::static_contact_subject_eligible(
                            &player.collision,
                            radius,
                        ) == RetailRuntimeValue::Known(true)
                    });
                    if let (Some(terrain), Some(frame)) = (session.cache.terrain(), frame_forces) {
                        let level_desc = session
                            .cache
                            .level_desc()
                            .expect("active gameplay has a Section-13 level descriptor");
                        let water_response_selectors = level_desc.water_response_selectors();
                        let ground_response_selectors = level_desc.ground_response_selectors();
                        let controlled_entity_handle = em.player().map(|player| player.id);
                        // The active model's ordinary +0x08 radius—not its
                        // distinct +0x0A collision radius—defines the VTOL
                        // underside used by both assist and ceiling policy.
                        let active_model = em
                            .player()
                            // Use the same resolved hull as presentation and
                            // collision. Retail traces keep type 46 on active
                            // slot zero/model 41 across both movement styles;
                            // the fallback preserves data-driven level loads.
                            .and_then(|player| player.model_index.or(player_model_id))
                            .and_then(|model_id| session.cache.global_model(model_id));
                        let active_model_extent_raw = active_model.map_or(0, |model| model.radius);
                        let active_model_half_radius_raw = active_model_extent_raw >> 1;
                        let active_model_collision_radius_raw =
                            active_model.map_or(0, |model| model.collision_radius_raw);
                        let terrain_contact_enabled = em.player().is_some_and(|player| {
                            terrain_collision_subject_eligible(
                                &player.collision,
                                active_model_collision_radius_raw,
                            ) == RetailRuntimeValue::Known(true)
                        });
                        let terrain_contact_model =
                            active_model.filter(|_| terrain_contact_enabled);
                        let sea = terrain.water_enabled().then(|| terrain.sea_level_world_y());
                        let RetailRuntimeValue::Known(attached_cargo_mass) =
                            em.attached_cargo_mass_state()
                        else {
                            // Attachment mass is a physics input, so an
                            // unauthenticated player Sub-J runtime cannot be
                            // approximated as an empty cargo list.
                            continue;
                        };
                        if let Some(player) = em.player().filter(|_| !player_hull.dying) {
                            player_craft.apply_hover_common_terrain_attitude(
                                PlayerHoverAttitudeRequest {
                                    terrain,
                                    player,
                                    active_model_extent_raw,
                                    attached_cargo_mass,
                                    water_enabled: level_desc.raw_u32(0x84).unwrap_or(0) != 0,
                                    retail_tick,
                                    elapsed_micros,
                                },
                            );
                        }
                        let active_anim_vars = player_contact_anim_vars.clone();
                        let (update, terrain_contact_result) = {
                            let mut contacts = PlayerSurfaceContactFrame {
                                request: PlayerSurfaceContactRequest {
                                    resources: &session.cache,
                                    terrain,
                                    entry_model: terrain_contact_model,
                                    entry_anim_vars: &active_anim_vars,
                                    vehicle_mode: player_craft.mode,
                                    controlled_entity_handle,
                                    ground_response_selectors,
                                    haptic_scale_raw: DEFAULT_PLAYER_TERRAIN_HAPTIC_SCALE_RAW,
                                    retail_tick,
                                    extra_lives: player_campaign_progress.extra_lives(),
                                },
                                hull: &mut player_hull,
                                world_fx: &mut world_fx,
                                notifications: &mut gameplay_notifications,
                                dying_runtime: em.player_dying_contact_runtime(),
                                water_style_outcome: None,
                            };
                            let result = em.update_with_player_surface_contacts(
                                PlayerUpdateRequest {
                                    body_pitch_roll_raw: player_craft.body_angle_words(),
                                    elapsed_micros,
                                    terrain,
                                    sea_level: sea,
                                    retail_tick,
                                    water_response_selectors,
                                    attached_cargo_mass,
                                    active_model_half_radius_raw,
                                    active_model_collision_radius_raw,
                                    frame,
                                    // Selector 0x3E's campaign-owned capability
                                    // drives both retail Turbo consumers: 1.5x
                                    // manual lift and the 0xC00 height threshold.
                                    vtol_boost: player_vtol_boost(&player_capabilities),
                                    vtol_height_policy: VtolHeightPolicy::RetailAttenuation,
                                },
                                terrain_contact_enabled,
                                |manager, phase| contacts.dispatch(manager, phase),
                            );
                            if !contacts.commit_runtime(em) {
                                eprintln!("Player dying contact runtime lost its controlled allocation before commit");
                            }
                            result
                        };
                        if let Some(angles) = update.body_angle_words_after_environment {
                            player_craft.restore_body_angle_words(angles);
                        }
                        if let Some(commit) = update.hard_water_entry_commit {
                            world_fx.note_hard_entry();
                            match em.commit_player_hard_water_entry(commit, terrain, &mut world_fx)
                            {
                                Ok(construction) => {
                                    if let Some(task_lease) = construction.primary_task_lease() {
                                        let entity_id = task_lease.actor().entity_id;
                                        match specialized_actor_tasks
                                            .register_type60_exploding_ring(task_lease)
                                        {
                                            Ok(None) => {}
                                            Ok(Some(_displaced)) => {
                                                eprintln!(
                                                    "Hard-water Type-60 task for entity {entity_id} unexpectedly replaced existing scheduler custody"
                                                );
                                            }
                                            Err(failure) => {
                                                eprintln!(
                                                    "Hard-water Type-60 task for entity {entity_id} was rejected by the specialized scheduler: {:?}",
                                                    failure.conflict
                                                );
                                            }
                                        }
                                    } else {
                                        world_fx.note_ring_rejection();
                                    }
                                }
                                Err(block) => {
                                    eprintln!(
                                        "Hard-water Type-60 player commit stopped before constructor order could complete: {block:?}"
                                    );
                                }
                            }
                        }
                        let traced_terrain_contact = terrain_contact_result
                            .as_ref()
                            .and_then(Option::as_ref)
                            .and_then(|result| result.as_ref().ok())
                            .and_then(Option::as_ref);
                        if let (Some(callback), Some(player)) =
                            (update.vtol_diagnostics.as_ref(), em.player())
                        {
                            append_port_vtol_trace(
                                &mut vtol_trace,
                                &mut vtol_trace_sequence,
                                PortVtolTraceContext {
                                    level_id: current_level_id,
                                    sensitivity,
                                    self_righting: config.self_righting,
                                    keys: &keys_down,
                                    channels,
                                    craft: &player_craft,
                                    player,
                                    terrain_contact: traced_terrain_contact,
                                    hull: &player_hull,
                                    callback,
                                },
                            )?;
                        }
                        if let Some(result) = terrain_contact_result.flatten() {
                            match result {
                                Ok(Some(outcome)) => {
                                    let haptic=match outcome.style_callback {
                                        v2k_game::terrain_contact::PlayerTerrainStyleCallbackOutcome::PlayerControl(style)=>style.unsupported_haptic,
                                        v2k_game::terrain_contact::PlayerTerrainStyleCallbackOutcome::DyingBounce(style)=>style.unsupported_haptic,
                                        _=>None,
                                    };
                                    if let Some(haptic) = haptic {
                                        eprintln!(
                                            "Player terrain haptic backend is unowned: {haptic:?}"
                                        );
                                    }
                                    if outcome.hull_damage.health_lost_raw > 0 {
                                        log!(
                                            "Hull: terrain impact {} damage {} health {}",
                                            outcome.hull_damage.requested_damage_raw,
                                            outcome.hull_damage.health_lost_raw,
                                            player_hull.health_raw
                                        );
                                    }
                                }
                                Ok(None) => {}
                                Err(error) => log!("Terrain contact unresolved: {error:?}"),
                            }
                        }
                        // Dispatch the returned whole-body SurfaceBurst before
                        // the returned class-19 presentation because both can
                        // consume the shared direction cursor. Hard entries
                        // are constructor-owned and were committed above. The
                        // class-19 payload itself remains the callback-entry
                        // pose captured before forces and contact.
                        if let Some(water_entry) = update.water_entry {
                            if let PlayerWaterEntryResponse::SurfaceBurst { response_selector } =
                                water_entry.response
                            {
                                world_fx.emit_whole_body_water_entry_burst_raw(
                                    water_entry.position_raw,
                                    water_entry.surface_y_raw,
                                    response_selector,
                                    water_entry.owner_entity_id,
                                );
                            }
                        }
                        if let Some(surface_effect) = update.surface_effect {
                            // FUN_00440E80 runs synchronously inside the
                            // type-46 callback from the pre-force craft pose.
                            // The emitted class-19 probe then participates in
                            // this frame's ordinary particle traversal below.
                            world_fx.emit_player_surface_effect_raw(
                                surface_effect,
                                terrain.sea_level_raw(),
                            );
                        }
                        player_craft.consume_fuel_raw(update.fuel_burn_raw);
                        // g_default_param is a wrapping signed 50-Hz word.
                        if fuel_warning_cadence.poll(
                            frame,
                            player_craft.fuel_raw,
                            retail_tick as i32,
                        ) {
                            gameplay_notifications.queue_fuel_low(retail_tick as i32);
                            GlobalSoundRuntime::new(
                                &session.cache,
                                &mut world_fx,
                                sound_manager.as_mut(),
                            )
                            .play_global_sound(FUEL_LOW_SOUND_ID);
                            log!("Fuel low: {}", player_craft.fuel_raw);
                        }
                    }

                    // Retail additionally requires the resolved controlled
                    // entity's type flags at +0x64 to contain bit 0. The port
                    // enters this branch only for its authored type-46 player
                    // (flags = 5), so a resolved frame is the corresponding
                    // controller/entity-presence gate. It deliberately stays
                    // true during the three-second dying dwell: retail does
                    // not test the dying bit here and the controller remains
                    // bound until the player allocation is removed.
                    let controlled_player_callback_active = frame_forces.is_some();
                    if controlled_player_callback_active
                        && hull_warning_cadence
                            .poll(hull_health_at_controller_entry_raw, retail_tick as i32)
                    {
                        gameplay_notifications.queue_hull_low(retail_tick as i32);
                        if let Some(sound_manager) = sound_manager.as_mut() {
                            sound_manager.play_centered_sound_with_gain(
                                HULL_LOW_SOUND_ID,
                                HULL_LOW_SOUND_GAIN,
                            );
                        }
                        log!("Hull low: {hull_health_at_controller_entry_raw}");
                    }

                    // The recurring plume is adjacent to the warning in
                    // FUN_00446640 but has no warning throttle or clock latch.
                    // Every successful controlled-player callback whose
                    // entry-snapshotted hull is strictly below 0x1389 consumes
                    // placement RNG and attempts a synchronous allocation.
                    // Retail then ORs entity +0x84 bit 3 after the attempt; its
                    // separate generic bit-31 consumer is not active for the
                    // bound type-46 controller, so that persistent internal
                    // latch does not create another port-side emission path.
                    if controlled_player_callback_active
                        && hull_health_at_controller_entry_raw < HULL_LOW_WARNING_BELOW_RAW
                    {
                        let plume_source = em.player().map(|player| {
                            (
                                player.position_raw(),
                                player_body_basis(&player_craft, player).forward,
                                player.id,
                            )
                        });
                        if let Some((position_raw, body_forward_q31, source_id)) = plume_source {
                            // FUN_00441670 compares the loaded Section-10 sea
                            // word unconditionally. `water_enabled` controls
                            // presentation/other water behavior, not this
                            // particle classifier; only absent terrain lacks a
                            // comparison plane.
                            let sea_level_raw = session
                                .cache
                                .terrain()
                                .map(|terrain| terrain.sea_level_raw());
                            world_fx.emit_player_low_hull_smoke_raw(
                                position_raw,
                                body_forward_q31,
                                source_id,
                                sea_level_raw,
                            );
                        }
                    }

                    // Section-10 static objects run second. Type 46's detailed
                    // player4 spheres query the complete static-model collision
                    // program after any bare-terrain separation above.
                    let contact_result = {
                        let terrain = session.cache.terrain();
                        let terrain_objects = session.cache.terrain_objects();
                        let active_model = player_contact_model_id
                            .and_then(|model_id| session.cache.global_model(model_id))
                            .filter(|_| player_static_entry_eligible);
                        match (terrain, terrain_objects, active_model, em.player_mut()) {
                            (
                                Some(terrain),
                                Some(terrain_objects),
                                Some(active_model),
                                Some(player),
                            ) => {
                                let mut position_raw = player.position_raw();
                                let mut velocity_raw = player.velocity_raw();
                                let active_model_to_world_basis =
                                    player_body_basis(&player_craft, player)
                                        .orientation_world_from_model()
                                        .map(|row| row.map(f64::from));
                                let active_anim_vars = player_contact_anim_vars.clone();
                                let result = resolve_player_static_contact(
                                    terrain,
                                    terrain_objects,
                                    &session.cache,
                                    retail_tick,
                                    active_model,
                                    active_model_to_world_basis,
                                    &active_anim_vars,
                                    PlayerStaticStyleCallbackFrame {
                                        request: PlayerContactStyleRequest {
                                            behavior_context: player.current_behavior_context,
                                            entity_handle: player.id,
                                            controlled_entity_handle: Some(player.id),
                                            haptic_scale_raw:
                                                DEFAULT_PLAYER_TERRAIN_HAPTIC_SCALE_RAW,
                                        },
                                        static_damage: &mut static_damage,
                                        world_fx: &mut world_fx,
                                        contact_sound_id: session
                                            .cache
                                            .global_entity_type(player.entity_type as usize)
                                            .map_or(RetailRuntimeValue::Unresolved, |record| {
                                                RetailRuntimeValue::Known(u16::from_le_bytes([
                                                    record.raw_header[0x8a],
                                                    record.raw_header[0x8b],
                                                ]))
                                            }),
                                    },
                                    &mut position_raw,
                                    &mut velocity_raw,
                                    &mut player_hull,
                                    &mut player_craft,
                                );
                                if result.as_ref().is_ok_and(Option::is_some) {
                                    player.set_motion_raw(position_raw, velocity_raw);
                                }
                                Some(result)
                            }
                            _ => None,
                        }
                    };

                    if player_hull.dying {
                        em.sync_player_hull_collision_state(&player_hull);
                        if let Err(block) = em.begin_player_dying(PlayerCheckedDamageFrame {
                            hull: &mut player_hull,
                            resources: &session.cache,
                            world_fx: &mut world_fx,
                            retail_tick,
                            notifications: &mut gameplay_notifications,
                            extra_lives: player_campaign_progress.extra_lives(),
                        }) {
                            eprintln!(
                                "Player static physical death initialization blocked: {block:?}"
                            );
                        }
                    }

                    let mut consumed_static_cell = None;
                    let mut static_damage_contact = None;
                    if let Some(result) = contact_result {
                        match result {
                            Ok(Some(outcome)) => {
                                if let PlayerStaticStyleCallbackOutcome::Completed {
                                    unsupported_haptic: Some(haptic),
                                    ..
                                } = outcome.style_callback
                                {
                                    eprintln!(
                                        "Player static haptic backend is unowned: {haptic:?}"
                                    );
                                }
                                campaign_warp
                                    .observe_player_static_contact(outcome.contact, retail_tick);
                                if outcome.hull_damage.health_lost_raw > 0 {
                                    log!(
                                        "Hull: static kind {} impact {} damage {} health {}",
                                        outcome.contact.kind_index,
                                        outcome.hull_damage.requested_damage_raw,
                                        outcome.hull_damage.health_lost_raw,
                                        player_hull.health_raw
                                    );
                                }
                                static_damage_contact = resolve_current_static_damage_target(
                                    &session.cache,
                                    outcome.contact.cell,
                                )
                                .ok()
                                .flatten()
                                .map(|target| (target, outcome.collision_packet));
                                match outcome.pickup {
                                    StaticPickupOutcome::FuelAccepted {
                                        fuel_before_raw,
                                        fuel_after_raw,
                                        clear_attribute_at,
                                    } => {
                                        consumed_static_cell = Some(clear_attribute_at);
                                        gameplay_notifications
                                            .queue_fuel_collected(retail_tick as i32);
                                        world_fx.queue_fixed_positional_sound_raw(
                                            PLAYER_PICKUP_SOUND_ID,
                                            outcome.position_before_raw,
                                        );
                                        log!(
                                            "Fuel: {} -> {} at cell ({}, {})",
                                            fuel_before_raw,
                                            fuel_after_raw,
                                            clear_attribute_at[0],
                                            clear_attribute_at[1]
                                        );
                                    }
                                    StaticPickupOutcome::FuelRejectedAtCapacity { fuel_raw } => {
                                        gameplay_notifications.queue_fuel_full(retail_tick as i32);
                                        log!("Fuel full: {fuel_raw}");
                                    }
                                    StaticPickupOutcome::HullRepairAccepted {
                                        health_before_raw,
                                        health_after_raw,
                                        clear_attribute_at,
                                    } => {
                                        consumed_static_cell = Some(clear_attribute_at);
                                        gameplay_notifications
                                            .queue_hull_repaired(retail_tick as i32);
                                        world_fx.queue_fixed_positional_sound_raw(
                                            PLAYER_PICKUP_SOUND_ID,
                                            outcome.position_before_raw,
                                        );
                                        log!(
                                            "Hull repair: {} -> {} at cell ({}, {})",
                                            health_before_raw,
                                            health_after_raw,
                                            clear_attribute_at[0],
                                            clear_attribute_at[1]
                                        );
                                    }
                                    StaticPickupOutcome::HullRepairRejectedAtCapacity {
                                        health_raw,
                                    } => {
                                        gameplay_notifications
                                            .queue_hull_repair_full(retail_tick as i32);
                                        log!("Hull repair rejected: {health_raw}");
                                    }
                                    StaticPickupOutcome::ShieldAccepted {
                                        buffer_before_raw,
                                        buffer_after_raw,
                                        clear_attribute_at,
                                    } => {
                                        consumed_static_cell = Some(clear_attribute_at);
                                        gameplay_notifications
                                            .queue_shield_collected(retail_tick as i32);
                                        log!(
                                            "Shield buffer: {} -> {} at cell ({}, {})",
                                            buffer_before_raw,
                                            buffer_after_raw,
                                            clear_attribute_at[0],
                                            clear_attribute_at[1]
                                        );
                                    }
                                    StaticPickupOutcome::None
                                    | StaticPickupOutcome::InactiveState { .. } => {}
                                }
                            }
                            Ok(None) => {}
                            Err(error) if !static_contact_error_reported => {
                                eprintln!("Static contact disabled by unresolved data: {error:?}");
                                static_contact_error_reported = true;
                            }
                            Err(_) => {}
                        }
                    }
                    if let Some(cell) = consumed_static_cell {
                        // Retail clears the live decompressed Section-10
                        // attribute, so every renderer/collision consumer sees
                        // the same removal without a parallel pickup mask.
                        session.cache.clear_level_terrain_object_attribute(cell);
                    }
                    em.sync_player_hull_collision_state(&player_hull);

                    if let Some((target, packet)) = static_damage_contact {
                        let outcome = static_damage.submit_hit(target, packet, &mut || {
                            world_fx.next_shared_retail_random_u16()
                        });
                        apply_immediate_static_damage_outcome(
                            outcome,
                            &mut session.cache,
                            &mut world_fx,
                        );
                        log!(
                            "Static damage: cell ({}, {}) {outcome:?}",
                            target.cell[0],
                            target.cell[1]
                        );
                    }

                    // Retail advances the cell-keyed damage-program FIFO after
                    // the entity/contact pass and executes each node before
                    // sampling the next saved node. Apply each returned batch
                    // synchronously so terrain changes are visible to the next
                    // lookup. Radial children are absent from begin_advance's
                    // captured ID queue and therefore wait until the next pass.
                    // Opcode 6 samples the first chase spring at this boundary;
                    // FUN_0044FFA0 updates that focus only later in the frame.
                    let static_damage_camera_focus_raw = chase_camera.focus_position_raw();
                    static_damage.begin_advance(elapsed_micros);
                    while let Some(batch) = static_damage.next_node_batch(|cell| {
                        resolve_current_static_damage_target(&session.cache, cell)
                            .ok()
                            .flatten()
                            .map(|target| target.state)
                    }) {
                        let (token, static_damage_actions) = batch.into_parts();
                        apply_static_damage_actions(
                            &mut static_damage,
                            static_damage_actions,
                            &mut session.cache,
                            em,
                            StaticDamageWorldContext::Playing {
                                extra_lives: RetailRuntimeValue::Known(
                                    player_campaign_progress.extra_lives(),
                                ),
                                player_hull: &mut player_hull,
                                actor_tasks: &mut specialized_actor_tasks,
                                notifications: &mut gameplay_notifications,
                                retail_tick,
                            },
                            &mut world_fx,
                            &mut static_damage_explosion_lights,
                            static_damage_camera_focus_raw,
                            &mut main_base_level_abort.outer_frame_sequence,
                        );
                        let completed = static_damage.complete_node_batch(token);
                        debug_assert!(completed, "applied node batch token must remain current");
                    }
                    if session.cache.take_level_terrain_presentation_dirty() {
                        renderer.invalidate_terrain_cache();
                    }

                    // FUN_00445310 tests fuel at callback entry. A positive
                    // tank that reaches zero from this frame's exact burn does
                    // not request Hover until the following VTOL callback.
                    if frame_forces.is_some_and(|frame| player_craft.finish_fuel_frame(frame)) {
                        gameplay_notifications.queue_fuel_empty(retail_tick as i32);
                        if let Some(position_raw) = fuel_exhaustion_position_raw {
                            world_fx.queue_fixed_positional_sound_raw(
                                FUEL_EXHAUSTED_SOUND_ID,
                                position_raw,
                            );
                        }
                        log!("Out of fuel — forced back to Hover");
                    }

                    if let Some(player) = em.player() {
                        // The eye keeps retail's fixed -Z compass bearing while
                        // FUN_0044FFA0's terrain clearance probe adjusts its Y/Z
                        // target. Independent signed-word eye/focus springs
                        // reproduce FUN_0040F3A0, including shortest-path torus
                        // motion.
                        let basis = player_body_basis(&player_craft, player);
                        chase_camera
                            .update(
                                ChaseCameraTarget {
                                    position_raw: player.position_raw(),
                                    body_basis: ChaseBodyBasis::from_vectors([
                                        basis.lateral,
                                        basis.up,
                                        basis.forward,
                                    ]),
                                    active_camera: config.active_camera,
                                },
                                gameplay_chase_terrain(&session.cache),
                                elapsed_micros,
                            )
                            .apply_to(&mut camera);
                    }
                } else {
                    let speed = fly_speed * dt;
                    let fwd = camera.forward();
                    let right = camera.right();

                    if keys_down.contains(&Keycode::W) {
                        camera.position[0] += fwd[0] * speed;
                        camera.position[1] += fwd[1] * speed;
                        camera.position[2] += fwd[2] * speed;
                    }
                    if keys_down.contains(&Keycode::S) {
                        camera.position[0] -= fwd[0] * speed;
                        camera.position[1] -= fwd[1] * speed;
                        camera.position[2] -= fwd[2] * speed;
                    }
                    if keys_down.contains(&Keycode::A) {
                        camera.position[0] -= right[0] * speed;
                        camera.position[2] -= right[2] * speed;
                    }
                    if keys_down.contains(&Keycode::D) {
                        camera.position[0] += right[0] * speed;
                        camera.position[2] += right[2] * speed;
                    }
                    if keys_down.contains(&Keycode::Space) {
                        camera.position[1] += speed;
                    }
                    if keys_down.contains(&Keycode::LShift) {
                        camera.position[1] -= speed;
                    }
                    // Keep the free-fly camera on the torus too.
                    camera.position[0] = v2k_core::world::wrap(camera.position[0]);
                    camera.position[2] = v2k_core::world::wrap(camera.position[2]);
                }

                let sea_level = session.cache.terrain().and_then(|terrain| {
                    terrain.water_enabled().then(|| terrain.sea_level_world_y())
                });
                let primary_trigger = PrimaryTriggerInput {
                    source_a: !player_hull.dying && keys_down.contains(&Keycode::Return),
                    source_b: !player_hull.dying
                        && mouse_buttons_down.contains(&MouseButton::Right),
                };
                if let Some(player) = em.player() {
                    let origin = player.position;
                    let gun_mounts = if primary_trigger.held() {
                        player_model_id
                            .map(|model_id| {
                                resolve_player_primary_gun_mounts(
                                    &session.cache,
                                    model_id,
                                    &player_craft,
                                    origin,
                                    player.heading,
                                )
                            })
                            .unwrap_or_default()
                    } else {
                        PrimaryGunMounts::Unresolved
                    };
                    let geometry = PrimaryFireGeometry {
                        launch_basis: PrimaryLaunchBasis {
                            origin_world: origin,
                            direction_unit: player_primary_fire_direction(&player_craft, player),
                        },
                        gun_mounts,
                        shooter_origin_world: origin,
                        shooter_id: player.id,
                        shooter_entity_type_at_birth: u8::try_from(player.entity_type).ok(),
                        shooter_velocity_world: player.velocity,
                        sound_origin_world: origin,
                    };
                    let simulation_delta = Duration::from_micros(u64::from(elapsed_micros));
                    let descriptor = *weapon_inventory.selected_descriptor();
                    if let Some(profile) = fire_profile_from_descriptor(&descriptor) {
                        let shot_budget = match descriptor.ammunition() {
                            Ammunition::Infinite => PrimaryShotBudget::Unlimited,
                            Ammunition::Finite(rounds) => PrimaryShotBudget::Limited(rounds),
                        };
                        let fire_events = primary_weapon.update_with_profile(
                            simulation_delta,
                            primary_trigger,
                            geometry,
                            profile,
                            shot_budget,
                        );
                        // Retain the selected descriptor and launch words before
                        // a final round can automatically select another weapon.
                        if !fire_events.is_empty() {
                            if let Some(kind) =
                                EntityWeaponKind::from_selector(descriptor.selector())
                            {
                                let body_basis = player_body_basis(&player_craft, player);
                                let direction_q31 = entity_weapon_direction_q31(
                                    kind,
                                    body_basis,
                                    player_craft.gun_barrel as i16,
                                );
                                pending_entity_weapon_fire.extend(fire_events.iter().map(
                                    |event| {
                                        PendingEntityWeaponFire {
                                            kind,
                                            source_actor_id: event.projectile.owner_id,
                                            direction_q31,
                                            emitter_index: match event.gun_channel {
                                                PrimaryGunChannel::A => 1,
                                                PrimaryGunChannel::B => 0,
                                            },
                                            time_offset_us: primary_weapon
                                                .elapsed()
                                                .saturating_sub(event.fired_at)
                                                .as_micros()
                                                .min(u128::from(u32::MAX))
                                                as u32,
                                        }
                                    },
                                ));
                            }
                        }
                        for _ in &fire_events {
                            let commit = weapon_inventory.commit_selected_round();
                            debug_assert!(matches!(commit, AmmoCommit::Fired { .. }));
                            sync_player_weapon_after_ammo_commit(
                                commit,
                                &mut player_craft,
                                &weapon_inventory,
                            );
                        }
                        world_fx.queue_primary_fire_batch(&fire_events, sea_level);
                    } else {
                        // Unrecovered selectors still visit FUN_00444FA0.
                        // Keep A/B phase instead of resetting the scheduler.
                        primary_weapon.idle(
                            simulation_delta,
                            descriptor.joint_decay_rate(),
                            primary_trigger.held(),
                        );
                    }
                } else {
                    primary_weapon = PrimaryWeapon::new();
                }
                player_craft.set_primary_joint_pulses(primary_weapon.joint_pulse_words());

                update_targetter_runtime(
                    &mut targetter,
                    &session.cache,
                    em,
                    &player_craft,
                    &weapon_inventory,
                    elapsed_micros,
                    retail_tick,
                );

                // Fire events are produced before the retail particle pass.
                // Materialize them now so the selected class-1/2/3
                // primary receives one integration step before its descriptor
                // collision callback examines it.
                world_fx.process_pending();
                // Controlled-player movement precedes the Hive's no-RNG wreck
                // contact phase; its component clock already ran in13500.
                pending_failed_world_interior_request |= em
                    .apply_authored_hive_wreck_player_contacts(
                        v2k_game::hive_controller::HiveWreckPlayerContactFrame {
                            elapsed_us: elapsed_micros,
                            session_aborted: main_base_level_abort.abort_flag_0x28f(),
                        },
                    );
                let mut base_factory_progression = advance_base_factory_progression(
                    &session.cache,
                    em,
                    &mut world_fx,
                    elapsed_micros,
                    retail_tick,
                    &type66_progression_claims,
                );
                // Stage 32 raises the campaign-owned abort edge during the
                // task pass. FUN_0044FFA0 consumes it only after the remaining
                // actor tasks, particles, active pairs, and Power-Up contacts;
                // the later systemic sweep publishes newborn owners for the
                // next frame rather than revisiting FUN_00413500 now.
                let pending_main_base_abort_origin = specialized_actor_tasks
                    .take_main_base_terminal_abort_origins()
                    .into_iter()
                    .next()
                    .or_else(|| base_factory_progression.terminal_abort_origin.take());
                if base_factory_progression.full_frame_sequence_requested {
                    // FUN_00419B50 stage 32, param_3 == 0: FUN_00456750.
                    main_base_level_abort.outer_frame_sequence.request();
                }

                // The specialized pass already ran before the player/contact
                // walker. Consume its owned diagnostics here, before the
                // remaining neutral animation and particle phases.
                if let Some(block) = specialized_pass.block {
                    if !specialized_actor_task_error_reported {
                        eprintln!(
                            "Specialized actor task scheduler stopped at unresolved frame evidence: {block:?}"
                        );
                        specialized_actor_task_error_reported = true;
                    }
                }
                if specialized_pass.sea_level_changed {
                    // Type 54 mutates only the live Section-10 sea word.
                    // Water geometry and projection sample that header every
                    // frame; the cached terrain mesh deliberately excludes it.
                    log!("Main Base Type-54 task updated the live sea level");
                }
                for owner_outcome in specialized_pass.outcomes {
                    if let Some(message) =
                        type9_task_diagnostics.observe(&owner_outcome, retail_tick, |entity_id| {
                            em.iter_all()
                                .find(|entity| entity.id == entity_id)
                                .map(|entity| type9_task_diagnostics::Type9DiagnosticActor {
                                    active: entity.active,
                                    authored_spawn_index: entity.authored_spawn_index,
                                })
                        })
                    {
                        eprintln!("{message}");
                    }
                    match owner_outcome {
                        SpecializedActorTaskProductionOutcome::SharedFish(
                            v2k_game::shared_fish::SharedFishOutcome::Blocked {
                                entity_id, reason, ..
                            },
                        ) if !specialized_actor_task_error_reported => {
                            eprintln!("Fish entity {entity_id} stopped: {reason:?}");
                            specialized_actor_task_error_reported = true;
                        }
                        SpecializedActorTaskProductionOutcome::SharedFish(
                            v2k_game::shared_fish::SharedFishOutcome::Dropped { entity_id },
                        ) if em.iter_all().any(|entity| entity.id == entity_id && entity.active)
                            && !specialized_actor_task_error_reported =>
                        {
                            eprintln!("Fish entity {entity_id} lost its native task owner");
                            specialized_actor_task_error_reported = true;
                        }
                        SpecializedActorTaskProductionOutcome::CleansingVehicle(
                            v2k_game::cleansing_vehicle::CleansingVehicleOutcome::Blocked {
                                entity_id, reason, ..
                            },
                        ) if !specialized_actor_task_error_reported => {
                            eprintln!("Cleansing vehicle entity {entity_id} stopped: {reason:?}");
                            specialized_actor_task_error_reported = true;
                        }
                        SpecializedActorTaskProductionOutcome::CleansingVehicle(
                            v2k_game::cleansing_vehicle::CleansingVehicleOutcome::Dropped { entity_id },
                        ) if em.iter_all().any(|entity| entity.id == entity_id && entity.active)
                            && !specialized_actor_task_error_reported =>
                        {
                            eprintln!("Cleansing vehicle entity {entity_id} lost its native task owner");
                            specialized_actor_task_error_reported = true;
                        }
                        SpecializedActorTaskProductionOutcome::Type17CommonDying(
                            Type17CommonDyingProductionOutcome::Advanced {
                                entity_id,
                                live_outcome,
                            },
                        ) if matches!(
                            live_outcome,
                            v2k_game::common_dying_live::Type17CommonDyingLiveFrameOutcome::DeferredDestroyStaged { .. }
                        ) => {
                            log!("Type-17 Common-Dying entity {entity_id}: {live_outcome:?}");
                        }
                        SpecializedActorTaskProductionOutcome::Type17CommonDying(
                            Type17CommonDyingProductionOutcome::Blocked { entity_id, reason },
                        ) if !specialized_actor_task_error_reported => {
                            eprintln!(
                                "Type-17 Common-Dying entity {entity_id} stopped at unresolved production evidence: {reason:?}"
                            );
                            specialized_actor_task_error_reported = true;
                        }
                        SpecializedActorTaskProductionOutcome::Type17CommonDying(
                            Type17CommonDyingProductionOutcome::LiveError {
                                entity_id,
                                error,
                                retained,
                            },
                        ) if !specialized_actor_task_error_reported => {
                            eprintln!(
                                "Type-17 Common-Dying entity {entity_id} stopped after its live prefix (retained={retained}): {error:?}"
                            );
                            specialized_actor_task_error_reported = true;
                        }
                        SpecializedActorTaskProductionOutcome::Type17CommonDying(
                            Type17CommonDyingProductionOutcome::Dropped { entity_id, reason },
                        ) => {
                            log!(
                                "Type-17 Common-Dying entity {entity_id}: dropped stale receipt {reason:?}"
                            );
                        }
                        SpecializedActorTaskProductionOutcome::MainBaseType54SeaLevel(
                            MainBaseType54SeaLevelProductionOutcome::Blocked { entity_id, reason },
                        )
                        | SpecializedActorTaskProductionOutcome::MainBaseType54SeaLevel(
                            MainBaseType54SeaLevelProductionOutcome::Dropped { entity_id, reason },
                        ) if !specialized_actor_task_error_reported => {
                            eprintln!(
                                "Main Base Type-54 task entity {entity_id} stopped: {reason:?}"
                            );
                            specialized_actor_task_error_reported = true;
                        }
                        SpecializedActorTaskProductionOutcome::MainBaseType9Exploding(
                            MainBaseType9ExplodingProductionOutcome::Blocked { entity_id, reason },
                        ) if !specialized_actor_task_error_reported => {
                            eprintln!(
                                "Main Base Type-9 task entity {entity_id} stopped: {reason:?}"
                            );
                            specialized_actor_task_error_reported = true;
                        }
                        SpecializedActorTaskProductionOutcome::MainBaseType9Exploding(
                            MainBaseType9ExplodingProductionOutcome::Dropped { entity_id, reason },
                        ) => {
                            log!("Main Base Type-9 task entity {entity_id}: dropped {reason:?}");
                        }
                        SpecializedActorTaskProductionOutcome::OrdinaryType9Class14(
                            MainBaseType9ExplodingProductionOutcome::SchedulerWaiting { .. }
                            | MainBaseType9ExplodingProductionOutcome::Continuing { .. }
                            | MainBaseType9ExplodingProductionOutcome::TransitionSuppressed {
                                ..
                            }
                            | MainBaseType9ExplodingProductionOutcome::Terminal(_),
                        ) => {}
                        SpecializedActorTaskProductionOutcome::OrdinaryType9Class14(
                            MainBaseType9ExplodingProductionOutcome::Blocked { entity_id, reason },
                        ) if !specialized_actor_task_error_reported => {
                            eprintln!(
                                "Ordinary Type-9 class-14 entity {entity_id} stopped: {reason:?}"
                            );
                            specialized_actor_task_error_reported = true;
                        }
                        SpecializedActorTaskProductionOutcome::OrdinaryType9Class14(
                            MainBaseType9ExplodingProductionOutcome::Dropped { entity_id, reason },
                        ) => {
                            log!("Ordinary Type-9 class-14 entity {entity_id}: dropped {reason:?}");
                        }
                        SpecializedActorTaskProductionOutcome::MainBaseType66Production(
                            MainBaseType66ProductionOutcome::Blocked { entity_id, reason },
                        ) if !specialized_actor_task_error_reported => {
                            eprintln!(
                                "Main Base Type-66 task entity {entity_id} stopped: {reason:?}"
                            );
                            specialized_actor_task_error_reported = true;
                        }
                        SpecializedActorTaskProductionOutcome::MainBaseType66Production(
                            MainBaseType66ProductionOutcome::Dropped { entity_id, reason },
                        ) => {
                            log!("Main Base Type-66 task entity {entity_id}: dropped {reason:?}");
                        }
                        // Selected Type9 failures are reported per actor above.
                        // Despite the historical name, PostBasisTailPending is
                        // emitted only after the retained outer tail completes.
                        SpecializedActorTaskProductionOutcome::OrdinaryType9RunAway(_)
                        | SpecializedActorTaskProductionOutcome::OrdinaryType9AttractAttention(_)
                        | SpecializedActorTaskProductionOutcome::OrdinaryType9GoToJob(_)
                        | SpecializedActorTaskProductionOutcome::OrdinaryType9Wander(_) => {}
                        SpecializedActorTaskProductionOutcome::Type60ExplodingRing(
                            Type60ExplodingRingProductionOutcome::Blocked { entity_id, reason },
                        ) if !specialized_actor_task_error_reported => {
                            eprintln!(
                                "Type-60 Exploding-Ring entity {entity_id} stopped at unresolved production evidence: {reason:?}"
                            );
                            specialized_actor_task_error_reported = true;
                        }
                        SpecializedActorTaskProductionOutcome::Type60ExplodingRing(
                            Type60ExplodingRingProductionOutcome::Dropped { entity_id, reason },
                        ) => {
                            log!("Type-60 Exploding-Ring entity {entity_id}: dropped {reason:?}");
                        }
                        SpecializedActorTaskProductionOutcome::Type47CommonDying(
                            Type47CommonDyingProductionOutcome::Blocked { entity_id, reason },
                        ) if !specialized_actor_task_error_reported => {
                            eprintln!(
                                "Type-47 Common-Dying entity {entity_id} stopped: {reason:?}"
                            );
                            specialized_actor_task_error_reported = true;
                        }
                        SpecializedActorTaskProductionOutcome::Type47CommonDying(
                            Type47CommonDyingProductionOutcome::Dropped { entity_id, reason },
                        ) => {
                            log!("Type-47 Common-Dying entity {entity_id}: dropped {reason:?}");
                        }
                        SpecializedActorTaskProductionOutcome::OrdinaryType47Scheduler(
                            OrdinaryType47SchedulerProductionOutcome::GraphRetained { .. }
                            | OrdinaryType47SchedulerProductionOutcome::WanderNear { .. }
                            | OrdinaryType47SchedulerProductionOutcome::AimAndFire { .. }
                            | OrdinaryType47SchedulerProductionOutcome::Acquisition { .. }
                            | OrdinaryType47SchedulerProductionOutcome::Pursuing { .. },
                        ) => {}
                        SpecializedActorTaskProductionOutcome::OrdinaryType47Scheduler(
                            OrdinaryType47SchedulerProductionOutcome::Blocked { entity_id, reason },
                        ) if !specialized_actor_task_error_reported => {
                            eprintln!(
                                "Ordinary Type-47 entity {entity_id} stopped: {reason:?}"
                            );
                            specialized_actor_task_error_reported = true;
                        }
                        SpecializedActorTaskProductionOutcome::OrdinaryType47Scheduler(
                            OrdinaryType47SchedulerProductionOutcome::Dropped { entity_id, reason },
                        ) => {
                            log!("Ordinary Type-47 entity {entity_id}: dropped {reason:?}");
                        }
                        SpecializedActorTaskProductionOutcome::Class0Actor(
                            v2k_game::entity::Class0ActorOutcome::Blocked { entity_id, reason, .. },
                        ) if !specialized_actor_task_error_reported => {
                            eprintln!("Class0 entity {entity_id} stopped: {reason:?}");
                            specialized_actor_task_error_reported = true;
                        }
                        SpecializedActorTaskProductionOutcome::Class0Actor(
                            v2k_game::entity::Class0ActorOutcome::Dropped { entity_id },
                        ) => {
                            log!("Class0 entity {entity_id}: owner dropped");
                        }
                        SpecializedActorTaskProductionOutcome::Class0Actor(_) => {}
                        SpecializedActorTaskProductionOutcome::Intro2Type26(outcome) => {
                            log!("Intro2 Type26: {outcome:?}");
                        }
                        other => log!(
                            "Specialized actor task {:?} entity {}: {other:?}",
                            other.family(),
                            other.entity_id(),
                        ),
                    }
                }
                base_factory_progression
                    .explosion_lights
                    .extend(specialized_pass.explosion_lights);

                // Snapshot claims were taken before the task pass, so a
                // terminal Type-9 or worker owner cannot fall through to this neutral
                // manager-wide Sub-I callback in the same frame.
                em.advance_unclaimed_actor_animations(elapsed_micros, &actor_animation_claims);
                // Player drops and Type17 missing-owner callbacks append
                // Type93 after the child. Run its callback and Sub-J pose
                // publication here, preserving the child's earlier carrying
                // visit. Factory materialisers retain their existing late phase.
                if let Some(terrain) = session.cache.terrain() {
                    for (entity_id, reason) in em.update_late_tail_materialisers(
                        v2k_game::entity::LateTailMaterialiserFrame {
                            elapsed_micros,
                            terrain,
                            world_fx: &mut world_fx,
                            scheduler: &mut specialized_actor_tasks,
                            notifications: &mut gameplay_notifications,
                            retail_tick,
                        },
                    ) {
                        log!("Materialiser child {entity_id}: release blocked: {reason}");
                    }
                }
                for entity_id in em.cleanup_pending_actor_deferred_destroys() {
                    log!("Actor entity {entity_id}: deferred sweep complete");
                }
                let particle_snapshot = build_particle_callback_snapshot(
                    &session.cache,
                    em,
                    player_model_id,
                    &player_craft,
                    &mut particle_collision_error_reported,
                );
                let mut particle_host = v2k_game::playing_particle_host::PlayingParticleHost {
                    cache: &mut session.cache,
                    static_damage: &mut static_damage,
                    owner_motions: particle_snapshot.owner_motions,
                    collision_models: particle_snapshot.collision_models,
                    attachment_owners: em.iter_all().map(v2k_game::world_fx::ParticleAttachmentOwner::from_entity).collect(),
                    on_actor_event: |cache: &mut v2k_game::resource_cache::ResourceCache, static_damage: &mut StaticDamageScheduler, world_fx: &mut WorldFx, event: v2k_game::playing_particle_host::PlayingParticleActorEvent| {
                        use v2k_game::playing_particle_host::{PlayingParticleActorEvent, PlayingParticleActorResponse};
                        let entity_impact = match event {
                            PlayingParticleActorEvent::Impact(impact) => impact,
                            PlayingParticleActorEvent::AttachedUpdate(request) => {
                                return PlayingParticleActorResponse::AttachedUpdate(
                                    v2k_game::world_fx::begin_attached_particle_owner_update(em, request),
                                );
                            }
                            PlayingParticleActorEvent::AttachedCascadeOwner(request) => {
                                return PlayingParticleActorResponse::AttachedCascadeOwner(
                                    v2k_game::world_fx::sample_attached_particle_cached_owner(em, request),
                                );
                            }
                            PlayingParticleActorEvent::AttachedDamage(request) => {
                                let result = v2k_game::attached_particle_damage::apply_attached_particle_damage(
                                    v2k_game::attached_particle_damage::AttachedParticleDamageFrame {
                                        resources: cache, entities: em, world_fx, static_damage,
                                        scheduler: &mut specialized_actor_tasks,
                                        notifications: &mut gameplay_notifications, retail_tick,
                                        world: v2k_game::attached_particle_damage::AttachedParticleDamageWorld::Playing { player_hull: &mut player_hull, extra_lives: RetailRuntimeValue::Known(player_campaign_progress.extra_lives()) },
                                    }, request,
                                );
                                return PlayingParticleActorResponse::AttachedDamage {
                                    result,
                                    refresh: v2k_game::playing_particle_host::PlayingParticleRefresh {
                                        collision: ParticleCollisionCacheRefresh::Replace(build_particle_collision_models(
                                            cache, em, player_model_id, &player_craft,
                                            &mut particle_collision_error_reported,
                                        )),
                                        attachment_owners: em.iter_all().map(v2k_game::world_fx::ParticleAttachmentOwner::from_entity).collect(),
                                        owner_motions: em.iter_all().map(|entity| ParticleOwnerMotion {
                                            owner_id: entity.id, velocity: entity.velocity,
                                        }).collect(),
                                    },
                                };
                            }
                        };
                        let collision = (|| {
                        let Some(damage) = entity_impact.damage else {
                            log!(
                                "Particle class {} entity impact: target {} damage suppressed by particle +0x1D bit0",
                                entity_impact.source_particle_class,
                                entity_impact.target_entity_id,
                            );
                            return ParticleCollisionCacheRefresh::Unchanged;
                        };
                        let cleansing = v2k_game::cleansing_vehicle::impact::apply_cleansing_vehicle_particle_hit(
                            v2k_game::cleansing_vehicle::impact::CleansingVehicleImpactFrame {
                                extra_lives: RetailRuntimeValue::Known(player_campaign_progress.extra_lives()),
                                entities: em, resources: cache, static_damage,
                                notifications: &mut gameplay_notifications, world_fx,
                                scheduler: &mut specialized_actor_tasks, retail_tick,
                                player_hull: &mut player_hull,
                            }, entity_impact,
                        );
                        if !matches!(cleansing, v2k_game::cleansing_vehicle::impact::CleansingVehicleImpactOutcome::NotApplicable) {
                            log!("Cleansing vehicle {} particle hit: {cleansing:?}", entity_impact.target_entity_id);
                            return ParticleCollisionCacheRefresh::Replace(build_particle_collision_models(
                                cache, em, player_model_id, &player_craft,
                                &mut particle_collision_error_reported,
                            ));
                        }
                        let player_switch = em.apply_player_model_switch_particle_hit(
                            entity_impact, PlayerCheckedDamageFrame { hull: &mut player_hull, resources: cache, world_fx, retail_tick, notifications: &mut gameplay_notifications, extra_lives: player_campaign_progress.extra_lives() },
                        );
                        if !matches!(player_switch, v2k_game::entity::PlayerModelSwitchHitOutcome::NotApplicable) {
                            log!("Player {} model-switch hit: {player_switch:?}", entity_impact.target_entity_id);
                            if matches!(player_switch, v2k_game::entity::PlayerModelSwitchHitOutcome::Applied { .. }) {
                                player_model_id = em.player().and_then(|player| player.model_index);
                            }
                            return ParticleCollisionCacheRefresh::Replace(build_particle_collision_models(
                                cache, em, player_model_id, &player_craft,
                                &mut particle_collision_error_reported,
                            ));
                        }
                        if let Some(outcome) = v2k_game::shared_actor_impact::apply_playing_actor_particle_hit(
                            v2k_game::shared_actor_impact::PlayingActorImpactFrame {
                                extra_lives: RetailRuntimeValue::Known(player_campaign_progress.extra_lives()),
                                resources: cache,
                                entities: em, world_fx, scheduler: &mut specialized_actor_tasks,
                                notifications: &mut gameplay_notifications, retail_tick,
                                static_damage, player_hull: &mut player_hull,
                            }, entity_impact,
                        ) {
                            log!("Shared actor {} particle hit: {outcome:?}", entity_impact.target_entity_id);
                            return ParticleCollisionCacheRefresh::Replace(build_particle_collision_models(
                                cache, em, player_model_id, &player_craft,
                                &mut particle_collision_error_reported,
                            ));
                        }
                        let f780_hit = particle_uses_fun_0043f780_entity_hit(
                            entity_impact.source_particle_class,
                        );
                        if f780_hit {
                            // FUN_00411250 sets 0x2000, then type +0x82 through
                            // FUN_0044F450 at entity +0x96 with scale 0x10000.
                            // Retained player/native actor owners above dispatch
                            // their exact styles. Do not take F590's10EB0/C690.
                            if let Some(applied) = em.apply_fun_00411250_infected_model_bit(
                                entity_impact.target_entity_id,
                            ) {
                                if let Some(sound_id) = applied.sound_id {
                                    world_fx.queue_fixed_positional_sound_raw(
                                        sound_id,
                                        applied.position_raw,
                                    );
                                }
                            }
                            match em.apply_fun_00411250_type9_checked_damage(
                                entity_impact.target_entity_id,
                                damage.packet,
                                world_fx,
                                retail_tick as i32,
                                Some(&mut gameplay_notifications),
                            ) {
                                Fun00411250Type9DamageOutcome::NotType9 => {}
                                Fun00411250Type9DamageOutcome::Survived {
                                    generic_hit_sound_id,
                                    position_raw,
                                } => {
                                    if let Some(sound_id) = generic_hit_sound_id {
                                        world_fx.queue_fixed_positional_sound_raw(
                                            sound_id,
                                            position_raw,
                                        );
                                    }
                                    return ParticleCollisionCacheRefresh::Replace(
                                        build_particle_collision_models(
                                            cache,
                                            em,
                                            player_model_id,
                                            &player_craft,
                                            &mut particle_collision_error_reported,
                                        ),
                                    );
                                }
                                Fun00411250Type9DamageOutcome::Lethal { task_lease, .. } => {
                                    if let Some(task_lease) = task_lease {
                                        if let Err(failure) = specialized_actor_tasks
                                            .adopt_ordinary_type9_class14_after_checked_death(
                                                task_lease,
                                            )
                                        {
                                            eprintln!(
                                                "Type-9 F780 class-14 adopt rejected: {:?}",
                                                failure.conflict
                                            );
                                        }
                                    }
                                    return ParticleCollisionCacheRefresh::Replace(
                                        build_particle_collision_models(
                                            cache,
                                            em,
                                            player_model_id,
                                            &player_craft,
                                            &mut particle_collision_error_reported,
                                        ),
                                    );
                                }
                                Fun00411250Type9DamageOutcome::AlreadyDying
                                | Fun00411250Type9DamageOutcome::Ineligible
                                | Fun00411250Type9DamageOutcome::FilteredOut
                                | Fun00411250Type9DamageOutcome::Unresolved
                                | Fun00411250Type9DamageOutcome::StandardDeathBlocked => {
                                    return ParticleCollisionCacheRefresh::Replace(
                                        build_particle_collision_models(
                                            cache,
                                            em,
                                            player_model_id,
                                            &player_craft,
                                            &mut particle_collision_error_reported,
                                        ),
                                    );
                                }
                            }
                        } else if let Some(delivery) = damage.delivery_record() {
                            // Run each bounded owner at F590's physical scan point
                            // so actor mutations, RNG, audio, notifications, and
                            // particles remain between F610 and the parent free.
                            // Type 17 requires complete birth provenance; every
                            // non-applicable target continues to Base/Factory.
                            let type17_outcome = apply_level_one_type17_projectile_primary_hit(
                                em,
                                cache,
                                world_fx,
                                &mut gameplay_notifications,
                                LevelOneType17ProjectilePrimaryHitRequest {
                                    target_entity_id: entity_impact.target_entity_id,
                                    delivery,
                                    source_provenance: entity_impact
                                        .impact_position_argument_va,
                                    impact_direction_q15: entity_impact.velocity_raw,
                                    retail_tick,
                                },
                            );
                            if let LevelOneType17PrimaryHitOutcome::Complete(completion) =
                                &type17_outcome
                            {
                                if let Some(Type17CommonDyingPublicationOutcome::Published {
                                    owner,
                                    ..
                                }) = completion.common_dying
                                {
                                    if let Err(failure) = specialized_actor_tasks
                                        .register_type17_common_dying(owner)
                                    {
                                        eprintln!(
                                            "Type-17 Common-Dying publication rejected by specialized scheduler: {:?}",
                                            failure.conflict
                                        );
                                    }
                                }
                            }
                            if type17_outcome != LevelOneType17PrimaryHitOutcome::NotApplicable {
                                log!(
                                    "Particle class {} type-17 impact: owner {:?} source_type_at_birth {:?} target {} position_arg {:#010x} velocity {:?} {type17_outcome:?}",
                                    entity_impact.source_particle_class,
                                    damage.source_owner_id,
                                    damage.source_entity_type_at_birth,
                                    entity_impact.target_entity_id,
                                    entity_impact.impact_position_argument_va,
                                    entity_impact.velocity_raw,
                                );
                                return ParticleCollisionCacheRefresh::Replace(
                                    build_particle_collision_models(
                                        cache,
                                        em,
                                        player_model_id,
                                        &player_craft,
                                        &mut particle_collision_error_reported,
                                    ),
                                );
                            }
                        }
                        if entity_impact.damage_delivery_record().is_some() {
                            match apply_type47_impact_c690_live(
                                em,
                                entity_impact.target_entity_id,
                                world_fx,
                                Type47ImpactLiveRequest {
                                    entry: if particle_uses_fun_0043f780_entity_hit(entity_impact.source_particle_class) {
                                        EntityHitEntry::Infected
                                    } else { EntityHitEntry::PrimaryProjectile },
                                    retail_tick,
                                },
                            ) {
                                Ok(Type47ImpactLiveOutcome::NotApplicable) => {}
                                Ok(Type47ImpactLiveOutcome::Class12Published { publication, .. }) => {
                                    log!(
                                        "Particle class {} type-47 hit callback class-12: target {}",
                                        entity_impact.source_particle_class,
                                        entity_impact.target_entity_id,
                                    );
                                    if let Err(failure) = specialized_actor_tasks
                                        .register_type47_common_dying(publication.owner)
                                    {
                                        eprintln!(
                                            "Type-47 C690 class-12 publication rejected by specialized scheduler: {:?}",
                                            failure.conflict
                                        );
                                    }
                                }
                                Ok(outcome) => {
                                    log!(
                                        "Particle class {} type-47 hit callback: target {} {outcome:?}",
                                        entity_impact.source_particle_class,
                                        entity_impact.target_entity_id,
                                    );
                                }
                                Err(error) => {
                                    log!(
                                        "Particle class {} type-47 hit callback blocked: target {} {error:?}",
                                        entity_impact.source_particle_class,
                                        entity_impact.target_entity_id,
                                    );
                                    return ParticleCollisionCacheRefresh::Replace(
                                        build_particle_collision_models(
                                            cache,
                                            em,
                                            player_model_id,
                                            &player_craft,
                                            &mut particle_collision_error_reported,
                                        ),
                                    );
                                }
                            }
                        }
                        match apply_type47_impact_reaction_after_c690(
                            em,
                            world_fx,
                            entity_impact.target_entity_id,
                            damage.packet.impact_sum_raw(),
                            entity_impact.velocity_raw,
                        ) {
                            Type47ImpactReactionOutcome::NotApplicable => {}
                            Type47ImpactReactionOutcome::Applied(outcome) => {
                                log!(
                                    "Particle class {} type-47 11030: target {} {outcome:?}",
                                    entity_impact.source_particle_class,
                                    entity_impact.target_entity_id,
                                );
                            }
                            Type47ImpactReactionOutcome::Blocked { entity_id, reason } => {
                                log!(
                                    "Particle class {} type-47 11030 blocked: target {entity_id} {reason:?}",
                                    entity_impact.source_particle_class,
                                );
                            }
                        }
                        let Some(delivery) = entity_impact.damage_delivery_record() else {
                            log!("Particle class {} lacks its damage-delivery provenance", entity_impact.source_particle_class);
                            return ParticleCollisionCacheRefresh::Replace(
                                build_particle_collision_models(
                                    cache,
                                    em,
                                    player_model_id,
                                    &player_craft,
                                    &mut particle_collision_error_reported,
                                ),
                            );
                        };
                        let type47_damage = apply_type47_checked_damage_after_c690(
                            em,
                            world_fx,
                            entity_impact.target_entity_id,
                            Type47CheckedDamageRequest {
                                delivery,
                                entry: if particle_uses_fun_0043f780_entity_hit(entity_impact.source_particle_class) {
                                    EntityHitEntry::Infected
                                } else {
                                    EntityHitEntry::PrimaryProjectile
                                },
                                retail_tick,
                            },
                        );
                        let damage_outcome = match type47_damage {
                            Type47CheckedDamageOutcome::Lethal {
                                publication: Some(publication),
                                ..
                            } => {
                                if let Err(failure) = specialized_actor_tasks
                                    .register_type47_common_dying(publication.owner)
                                {
                                    eprintln!(
                                        "Type-47 10C10 class-12 publication rejected by specialized scheduler: {:?}",
                                        failure.conflict
                                    );
                                }
                                format!("Lethal({:?})", publication.owner.entity_id())
                            }
                            Type47CheckedDamageOutcome::NotApplicable => {
                                match em.apply_fun_00411250_player_impact_reaction(
                                    entity_impact.target_entity_id,
                                    damage.packet,
                                    entity_impact.velocity_raw,
                                    || u32::from(world_fx.next_shared_retail_random_u16()),
                                ) {
                                    Fun00411250ImpactReactionOutcome::NotPlayer => {}
                                    Fun00411250ImpactReactionOutcome::Applied(outcome) => {
                                        log!(
                                            "Particle class {} player 11030: target {} {outcome:?}",
                                            entity_impact.source_particle_class,
                                            entity_impact.target_entity_id,
                                        );
                                    }
                                    Fun00411250ImpactReactionOutcome::Blocked(reason) => {
                                        log!(
                                            "Particle class {} player 11030 blocked: target {} {reason:?}",
                                            entity_impact.source_particle_class,
                                            entity_impact.target_entity_id,
                                        );
                                    }
                                }
                                match em.apply_player_checked_damage(
                                    PlayerCheckedDamageRequest {
                                        entry: v2k_game::entity::PlayerDamageEntry::Checked,
                                        target_id: entity_impact.target_entity_id,
                                        delivery,
                                        ratio_numerator: 0,
                                        ratio_denominator: 0,
                                    },
                                    PlayerCheckedDamageFrame { hull: &mut player_hull, resources: cache, world_fx, retail_tick, notifications: &mut gameplay_notifications, extra_lives: player_campaign_progress.extra_lives() },
                                ) {
                                    PlayerCheckedDamageOutcome::NotPlayer => {}
                                    PlayerCheckedDamageOutcome::Applied { .. } => {
                                        return ParticleCollisionCacheRefresh::Replace(
                                            build_particle_collision_models(
                                                cache,
                                                em,
                                                player_model_id,
                                                &player_craft,
                                                &mut particle_collision_error_reported,
                                            ),
                                        );
                                    }
                                    PlayerCheckedDamageOutcome::Blocked(reason) => {
                                        log!("Player checked damage blocked: {reason:?}");
                                        return ParticleCollisionCacheRefresh::Replace(build_particle_collision_models(
                                            cache, em, player_model_id, &player_craft,
                                            &mut particle_collision_error_reported,
                                        ));
                                    }
                                    PlayerCheckedDamageOutcome::Ineligible
                                    | PlayerCheckedDamageOutcome::FilteredOut
                                    => {
                                        return ParticleCollisionCacheRefresh::Replace(
                                            build_particle_collision_models(
                                                cache,
                                                em,
                                                player_model_id,
                                                &player_craft,
                                                &mut particle_collision_error_reported,
                                            ),
                                        );
                                    }
                                }
                                match em.apply_fun_00411250_type9_checked_damage(
                                    entity_impact.target_entity_id,
                                    damage.packet,
                                    world_fx,
                                    retail_tick as i32,
                                    Some(&mut gameplay_notifications),
                                ) {
                                    Fun00411250Type9DamageOutcome::NotType9 => {}
                                    Fun00411250Type9DamageOutcome::Survived {
                                        generic_hit_sound_id,
                                        position_raw,
                                    } => {
                                        if let Some(sound_id) = generic_hit_sound_id {
                                            world_fx.queue_fixed_positional_sound_raw(
                                                sound_id,
                                                position_raw,
                                            );
                                        }
                                        return ParticleCollisionCacheRefresh::Replace(
                                            build_particle_collision_models(
                                                cache,
                                                em,
                                                player_model_id,
                                                &player_craft,
                                                &mut particle_collision_error_reported,
                                            ),
                                        );
                                    }
                                    Fun00411250Type9DamageOutcome::Lethal { task_lease, .. } => {
                                        if let Some(task_lease) = task_lease {
                                            if let Err(failure) = specialized_actor_tasks
                                                .adopt_ordinary_type9_class14_after_checked_death(
                                                    task_lease,
                                                )
                                            {
                                                eprintln!(
                                                    "Type-9 class-14 adopt rejected: {:?}",
                                                    failure.conflict
                                                );
                                            }
                                        }
                                        return ParticleCollisionCacheRefresh::Replace(
                                            build_particle_collision_models(
                                                cache,
                                                em,
                                                player_model_id,
                                                &player_craft,
                                                &mut particle_collision_error_reported,
                                            ),
                                        );
                                    }
                                    Fun00411250Type9DamageOutcome::AlreadyDying
                                    | Fun00411250Type9DamageOutcome::Ineligible
                                    | Fun00411250Type9DamageOutcome::FilteredOut
                                    | Fun00411250Type9DamageOutcome::Unresolved
                                    | Fun00411250Type9DamageOutcome::StandardDeathBlocked => {
                                        return ParticleCollisionCacheRefresh::Replace(
                                            build_particle_collision_models(
                                                cache,
                                                em,
                                                player_model_id,
                                                &player_craft,
                                                &mut particle_collision_error_reported,
                                            ),
                                        );
                                    }
                                }
                                let outcome = em.apply_audited_base_factory_projectile_damage(
                                    entity_impact.target_entity_id,
                                    CheckedProjectileDamageRequest {
                                        delivery,
                                        entry: if particle_uses_fun_0043f780_entity_hit(entity_impact.source_particle_class) {
                                            EntityHitEntry::Infected
                                        } else { EntityHitEntry::PrimaryProjectile },
                                        retail_tick,
                                    },
                                );
                                queue_checked_projectile_damage_audio(world_fx, &outcome);
                                queue_premature_hive_hit_hint(
                                    em,
                                    &outcome,
                                    retail_tick as i32,
                                    &mut gameplay_notifications,
                                );
                                if let CheckedProjectileDamageOutcome::HiveDeathStarted(started) =
                                    outcome
                                {
                                    record_world_complete_after_hive_death(
                                        &mut gameplay_notifications,
                                        &mut world_complete_results,
                                        &mut main_base_level_abort,
                                        &mut player_campaign_progress,
                                        em,
                                        &mut world_complete_tally,
                                        cache.level_terrain(),
                                        retail_tick as i32,
                                        world_fx,
                                    );
                                    pending_hive_death_effects = Some(started);
                                }
                                format!("{outcome:?}")
                            }
                            type47_outcome => {
                                log!(
                                    "Particle class {} type-47 15040: target {} {type47_outcome:?}",
                                    entity_impact.source_particle_class,
                                    entity_impact.target_entity_id,
                                );
                                format!("{type47_outcome:?}")
                            }
                        };
                        log!(
                            "Particle class {} entity impact: owner {:?} source_type_at_birth {:?} target {} position_arg {:#010x} velocity {:?} {damage_outcome}",
                            entity_impact.source_particle_class,
                            damage.source_owner_id,
                            damage.source_entity_type_at_birth,
                            entity_impact.target_entity_id,
                            entity_impact.impact_position_argument_va,
                            entity_impact.velocity_raw,
                        );
                        ParticleCollisionCacheRefresh::Replace(
                            build_particle_collision_models(
                                cache,
                                em,
                                player_model_id,
                                &player_craft,
                                &mut particle_collision_error_reported,
                            ),
                        )
                        })();
                        PlayingParticleActorResponse::Impact(v2k_game::playing_particle_host::PlayingParticleRefresh {
                            collision,
                            attachment_owners: em.iter_all().map(v2k_game::world_fx::ParticleAttachmentOwner::from_entity).collect(),
                            owner_motions: em.iter_all().map(|entity| ParticleOwnerMotion {
                                owner_id: entity.id, velocity: entity.velocity,
                            }).collect(),
                        })
                    },
                    on_terrain_event: |static_damage: &mut StaticDamageScheduler, world_fx: &mut WorldFx, event| {
                        use v2k_game::world_fx::ParticleTerrainResponse;
                        let static_impact = match event {
                            v2k_game::world_fx::ParticleTerrainEvent::GroundProgram(request) => {
                                let cell = [
                                    ((request.position_raw[0] as u16) >> 8) as u8,
                                    ((request.position_raw[2] as u16) >> 8) as u8,
                                ];
                                static_damage.submit_fireball_ground_program(cell);
                                return ParticleTerrainResponse::GroundProgramAccepted;
                            }
                            v2k_game::world_fx::ParticleTerrainEvent::StaticImpact(impact) => impact,
                        };
                        let Some(damage) = static_impact.damage else {
                            log!(
                                "Particle class {} static impact: cell ({}, {}) damage suppressed by particle +0x1D bit0",
                                static_impact.source_particle_class,
                                static_impact.cell[0],
                                static_impact.cell[1],
                            );
                            return ParticleTerrainResponse::Unhandled;
                        };
                        let Some(current) = static_impact.current else {
                            log!(
                                "Particle class {} static impact: cell ({}, {}) current target unresolved",
                                static_impact.source_particle_class,
                                static_impact.cell[0],
                                static_impact.cell[1],
                            );
                            return ParticleTerrainResponse::Unhandled;
                        };
                        let damage_outcome =
                            static_damage.submit_hit(current.target, damage.packet, &mut || {
                                world_fx.next_shared_retail_random_u16()
                            });
                        debug_assert!(
                            !matches!(
                                damage_outcome,
                                StaticDamageOutcome::BurnedKind10Transition { .. }
                            ),
                            "the exact F800 packets cannot reach kind-10's immediate mutation"
                        );
                        log!(
                            "Particle class {} static impact: collision kind {} current kind {} cell ({}, {}) {damage_outcome:?}",
                            static_impact.source_particle_class,
                            static_impact.kind_index,
                            current.target.state.kind_index,
                            current.target.cell[0],
                            current.target.cell[1],
                        );
                        ParticleTerrainResponse::StaticDamage(damage_outcome)
                    },
                };
                let outcome = world_fx.update_with_traversal_host(
                    v2k_game::world_fx::ParticleTraversalTiming {
                        elapsed_micros,
                        retail_tick,
                    },
                    &mut particle_host,
                );
                if let Some(started) = pending_hive_death_effects {
                    apply_ordinary_hive_death_burst_and_radial(
                        OrdinaryHiveDeathFrame {
                            extra_lives: RetailRuntimeValue::Known(
                                player_campaign_progress.extra_lives(),
                            ),
                            entities: em,
                            cache: &mut session.cache,
                            world_fx: &mut world_fx,
                            static_damage: &mut static_damage,
                            player_hull: &mut player_hull,
                            actor_tasks: &mut specialized_actor_tasks,
                            notifications: &mut gameplay_notifications,
                            retail_tick,
                        },
                        started,
                    );
                }
                for impact in outcome.primary_impacts {
                    if !matches!(impact, PrimaryImpact::StaticTile(_)) {
                        log!("Primary impact: {impact:?}");
                    }
                }
                // `ballistic_static_impacts` and primary StaticTile entries
                // are retrospective F800 telemetry only. Their exact packets
                // were already delivered by the inline handler above.
                for static_impact in outcome.class_87_static_impacts {
                    // The descriptor's FUN_0042E8E0 static callback is
                    // intentionally empty. WorldFx already consumed the
                    // parent without a visual or damage delivery.
                    log!("Class-87 static impact: {static_impact:?}");
                }

                // Native surface and static phases precede
                // the admitted active pairs. A spider may be the second body.
                let contact_ids: Vec<_> = em.retail_live_order_ids().collect();
                let type9_metadata = session
                    .cache
                    .global_entity_type(9)
                    .map(EntityTypeRuntimeMetadata::from_section12);
                for id in contact_ids {
                    use v2k_game::intro2_type17::contact::{
                        resolve_type17_static_contact, Type17ContactOutcome,
                    };
                    if let Some(owner) = specialized_actor_tasks.native_weapon_owner(em, id) {
                        let outcome = v2k_game::native_entity_weapons::contact::resolve_entity_weapon_contacts(
                            v2k_game::native_entity_weapons::contact::EntityWeaponContactFrame {
                                common: v2k_game::intro2_contacts::Intro2ContactFrame {
                                    entities: em, resources: &mut session.cache, world_fx: &mut world_fx,
                                    static_damage: &mut static_damage, notifications: &mut gameplay_notifications,
                                    retail_tick, actor_tasks: &mut specialized_actor_tasks,
                                },
                                world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Playing {
                                    player_hull: &mut player_hull,
                                    extra_lives: RetailRuntimeValue::Known(player_campaign_progress.extra_lives()),
                                },
                            }, owner,
                        );
                        match outcome {
                            Err(reason) => {
                                eprintln!("Native weapon {id} contact stopped: {reason:?}")
                            }
                            Ok(outcome) => log!("Native weapon {id} contact: {outcome:?}"),
                        }
                        continue;
                    }
                    let mut contact_frame = v2k_game::intro2_contacts::Intro2ContactFrame {
                        entities: em,
                        resources: &mut session.cache,
                        world_fx: &mut world_fx,
                        static_damage: &mut static_damage,
                        notifications: &mut gameplay_notifications,
                        retail_tick,
                        actor_tasks: &mut specialized_actor_tasks,
                    };
                    let native_flyer = contact_frame
                        .entities
                        .iter_all()
                        .find(|entity| entity.id == id)
                        .is_some_and(|entity| matches!(entity.entity_type, 15 | 87));
                    if native_flyer {
                        use v2k_game::{
                            intro2_flyer_contacts::{
                                resolve_native_flyer_contacts, Intro2FlyerContactOutcome,
                            },
                            native_ground_actor::contact::NativeGroundContactOutcome,
                        };
                        // The same11AD0 prefix owns Intro2 and native Hive children:
                        // solid/water, then static using the retained entry model.
                        let outcome = resolve_native_flyer_contacts(&mut contact_frame, id);
                        if matches!(
                            &outcome.surface,
                            Intro2FlyerContactOutcome::Blocked { .. }
                                | Intro2FlyerContactOutcome::Applied {
                                    solid_contact: true,
                                    ..
                                }
                                | Intro2FlyerContactOutcome::Applied {
                                    water_entry: true,
                                    ..
                                }
                        ) {
                            log!("Native flyer {id} surface contact: {:?}", outcome.surface);
                        }
                        if let Some(result) = &outcome.static_contact {
                            if !matches!(
                                result,
                                NativeGroundContactOutcome::Ineligible
                                    | NativeGroundContactOutcome::Miss
                            ) {
                                log!("Native flyer {id} static contact: {result:?}");
                            }
                        }
                        if outcome.blocks_later_contacts() {
                            continue;
                        }
                    } else {
                        use v2k_game::native_actor_surface_contact::{
                            resolve_native_actor_surface_contact, NativeActorSurfaceContactOutcome,
                        };
                        let surface = resolve_native_actor_surface_contact(&mut contact_frame, id);
                        if matches!(
                            &surface,
                            NativeActorSurfaceContactOutcome::Blocked { .. }
                                | NativeActorSurfaceContactOutcome::Applied {
                                    solid_contact: true,
                                    ..
                                }
                                | NativeActorSurfaceContactOutcome::Applied {
                                    water_entry: true,
                                    ..
                                }
                        ) {
                            log!("Native actor {id} surface contact: {surface:?}");
                        }
                        if matches!(surface, NativeActorSurfaceContactOutcome::Blocked { .. }) {
                            continue;
                        }
                        {
                            use v2k_game::native_ground_actor::contact::{
                                resolve_insect_static_contact, NativeGroundContactOutcome,
                            };
                            let insect_static =
                                resolve_insect_static_contact(&mut contact_frame, id);
                            if !matches!(
                                insect_static,
                                NativeGroundContactOutcome::Ineligible
                                    | NativeGroundContactOutcome::Miss
                            ) {
                                log!("Native insect {id} static contact: {insect_static:?}");
                            }
                            if matches!(insect_static, NativeGroundContactOutcome::Blocked { .. }) {
                                continue;
                            }
                        }
                    }
                    let result = resolve_type17_static_contact(&mut contact_frame, id);
                    if !matches!(
                        result,
                        Type17ContactOutcome::Ineligible | Type17ContactOutcome::Miss
                    ) {
                        log!("Shared spider {id} static contact: {result:?}");
                    }
                    // 11AD0 static response precedes this subject's active pairs.
                    {
                        use v2k_game::intro2_type58::contact::{
                            resolve_intro2_type58_static_contact, Intro2Type58ContactOutcome,
                        };
                        let result = resolve_intro2_type58_static_contact(&mut contact_frame, id);
                        if !matches!(
                            result,
                            Intro2Type58ContactOutcome::Ineligible
                                | Intro2Type58ContactOutcome::Miss
                        ) {
                            log!("Ordinary Type58 {id} static contact: {result:?}");
                        }
                        if matches!(result, Intro2Type58ContactOutcome::Blocked { .. }) {
                            continue;
                        }
                    }
                    // D920 static callbacks precede this subject's pair walk.
                    {
                        use v2k_game::intro2_type53::contact::{
                            resolve_type53_static_contact, Type53ContactOutcome,
                        };
                        let result = resolve_type53_static_contact(&mut contact_frame, id);
                        if !matches!(
                            result,
                            Type53ContactOutcome::Ineligible | Type53ContactOutcome::Miss
                        ) {
                            log!("Ordinary Type53 {id} static contact: {result:?}");
                        }
                        if matches!(result, Type53ContactOutcome::Blocked { .. }) {
                            continue;
                        }
                    }
                    // D920 static callbacks precede this subject's pair walk.
                    {
                        use v2k_game::native_type122::contact::{
                            resolve_type122_static_contact, Type122ContactOutcome,
                        };
                        let result = resolve_type122_static_contact(&mut contact_frame, id);
                        if !matches!(
                            result,
                            Type122ContactOutcome::Ineligible | Type122ContactOutcome::Miss
                        ) {
                            log!("Ordinary Type122 {id} static contact: {result:?}");
                        }
                        if matches!(result, Type122ContactOutcome::Blocked { .. }) {
                            continue;
                        }
                    }
                    // D920 static callbacks precede this subject's pair walk.
                    {
                        use v2k_game::native_type30::contact::{
                            resolve_type30_static_contact, Type30ContactOutcome,
                        };
                        let result = resolve_type30_static_contact(&mut contact_frame, id);
                        if !matches!(
                            result,
                            Type30ContactOutcome::Ineligible | Type30ContactOutcome::Miss
                        ) {
                            log!("Ordinary Type30 {id} static contact: {result:?}");
                        }
                        if matches!(result, Type30ContactOutcome::Blocked { .. }) {
                            continue;
                        }
                    }
                    // D920 static callbacks precede this subject's pair walk.
                    {
                        use v2k_game::native_type40::contact::{
                            resolve_type40_static_contact, Type40ContactOutcome,
                        };
                        let result = resolve_type40_static_contact(&mut contact_frame, id);
                        if !matches!(
                            result,
                            Type40ContactOutcome::Ineligible | Type40ContactOutcome::Miss
                        ) {
                            log!("Ordinary Type40 {id} static contact: {result:?}");
                        }
                        if matches!(result, Type40ContactOutcome::Blocked { .. }) {
                            continue;
                        }
                    }
                    // D920 static callbacks precede this subject's pair walk.
                    {
                        use v2k_game::native_type56::contact::{
                            resolve_type56_static_contact, Type56ContactOutcome,
                        };
                        let result = resolve_type56_static_contact(&mut contact_frame, id);
                        if !matches!(
                            result,
                            Type56ContactOutcome::Ineligible | Type56ContactOutcome::Miss
                        ) {
                            log!("Ordinary Type56 {id} static contact: {result:?}");
                        }
                        if matches!(result, Type56ContactOutcome::Blocked { .. }) {
                            continue;
                        }
                    }
                    //11AD0 retains the peasant's entry admission, runs its
                    // fence response, then visits successor actor pairs.
                    if contact_frame
                        .entities
                        .iter_all()
                        .any(|entity| entity.id == id && entity.entity_type == 9)
                    {
                        if let Some(metadata) = type9_metadata.as_ref() {
                            use v2k_game::ordinary_type9_static_contact::{
                                resolve_ordinary_type9_late_contact, OrdinaryType9PairSuffix,
                                OrdinaryType9StaticContactOutcome,
                            };
                            let late = resolve_ordinary_type9_late_contact(
                                &mut contact_frame,
                                id,
                                metadata,
                                Some(v2k_game::native_actor_capture::pair::PlayingPlayerContact {
                                    hull: &mut player_hull,
                                    extra_lives: RetailRuntimeValue::Known(
                                        player_campaign_progress.extra_lives(),
                                    ),
                                }),
                            );
                            if !matches!(
                                late.static_contact,
                                OrdinaryType9StaticContactOutcome::Ineligible
                                    | OrdinaryType9StaticContactOutcome::Miss
                            ) {
                                log!(
                                    "Ordinary peasant {id} static contact: {:?}",
                                    late.static_contact
                                );
                            }
                            if let OrdinaryType9PairSuffix::Visited(pairs) = late.pair_suffix {
                                use v2k_game::intro2_type17::pair::Type17PairOutcome;
                                if matches!(&pairs, Type17PairOutcome::Blocked { .. })
                                    || matches!(&pairs, Type17PairOutcome::Resolved { visits } if !visits.is_empty())
                                {
                                    log!("Shared actor contacts at peasant {id}: {pairs:?}");
                                }
                            }
                        } else {
                            log!("Ordinary peasant {id} contact blocked: missing Type9 metadata");
                        }
                        // A blocked static prefix skips only this subject's
                        // dynamic suffix; subsequent intrusive actors proceed.
                        continue;
                    }
                    if !matches!(result, Type17ContactOutcome::Blocked { .. }) {
                        use v2k_game::intro2_type17::pair::{
                            resolve_type17_active_contacts_with_playing, CaptureFeedbackPolicy,
                            Type17PairOutcome,
                        };
                        let pairs = resolve_type17_active_contacts_with_playing(
                            &mut contact_frame,
                            id,
                            CaptureFeedbackPolicy::Gameplay,
                            Some(v2k_game::native_actor_capture::pair::PlayingPlayerContact {
                                hull: &mut player_hull,
                                extra_lives: RetailRuntimeValue::Known(
                                    player_campaign_progress.extra_lives(),
                                ),
                            }),
                        );
                        if matches!(&pairs, Type17PairOutcome::Blocked { .. })
                            || matches!(&pairs, Type17PairOutcome::Resolved { visits } if !visits.is_empty())
                        {
                            log!("Shared spider contacts at subject {id}: {pairs:?}");
                        }
                    }
                    // Ordinary Type47 newants collide with static objects as
                    // well. The 02CA0 task-hook retarget stays open.
                    if let Some(metadata) = session
                        .cache
                        .global_entity_type(47)
                        .map(EntityTypeRuntimeMetadata::from_section12)
                    {
                        use v2k_game::type47_static_contact::{
                            resolve_ordinary_type47_static_contact, Type47StaticContactFrame,
                            Type47StaticContactOutcome,
                        };
                        let type47_frame = Type47StaticContactFrame {
                            resources: &mut session.cache,
                            static_damage: &mut static_damage,
                            world_fx: &mut world_fx,
                            scheduler: &mut specialized_actor_tasks,
                            retail_tick,
                        };
                        let result =
                            resolve_ordinary_type47_static_contact(em, id, &metadata, type47_frame);
                        if !matches!(
                            result,
                            Type47StaticContactOutcome::Ineligible
                                | Type47StaticContactOutcome::Miss
                        ) {
                            log!("Ordinary newant {id} static contact: {result:?}");
                        }
                    }
                }

                if session.cache.take_level_terrain_presentation_dirty() {
                    renderer.invalidate_terrain_cache();
                }

                // Retail walks every active entity pair after the particle
                // pass. The bounded Main Base adapter owns the proven
                // native-person conversion transaction before the player-subject
                // subset below: saved-next traversal, event 1, deferred
                // destruction, selected-worker tail append, target components,
                // and the no-damage physical suffix.
                if ordinary_world_pair_pass_required(current_level_id) {
                    match resolve_first_world_main_base_conversions(MainBaseConversionFrame {
                        entities: em,
                        actor_tasks: &mut specialized_actor_tasks,
                        model_pool: &session.cache,
                        terrain: session.cache.terrain(),
                        world_fx: &mut world_fx,
                        notifications: &mut gameplay_notifications,
                        retail_tick,
                        world_style: session
                            .cache
                            .level_desc()
                            .map_or(RetailRuntimeValue::Unresolved, |level| {
                                RetailRuntimeValue::Known(level.world_style)
                            }),
                    }) {
                        Ok(FirstWorldMainBaseConversionPass::Resolved(pass)) => {
                            for visit in pass
                                .visits
                                .iter()
                                .filter(|visit| !visit.action_outcomes.is_empty())
                            {
                                log!(
                                    "Main Base conversion entity {}: actions {:?}, suffix {:?}",
                                    visit.entity_id,
                                    visit.action_outcomes,
                                    visit.suffix_outcome
                                );
                            }
                        }
                        Ok(FirstWorldMainBaseConversionPass::MainBaseAbsent) => {}
                        Err(error) if !main_base_conversion_error_reported => {
                            eprintln!("Main Base conversion stopped at unresolved data: {error:?}");
                            main_base_conversion_error_reported = true;
                        }
                        Err(_) => {}
                    }
                    let factory_ids: Vec<_> = em
                        .iter()
                        .filter(|entity| entity.entity_type == 66)
                        .map(|entity| entity.id)
                        .collect();
                    for factory_id in factory_ids {
                        match resolve_factory_pair_arrivals(
                            em,
                            &session.cache,
                            FactoryPairRequest {
                                factory_id,
                                retail_tick,
                            },
                        ) {
                            Ok(FactoryPairPassResult::FactoryAbsent) => {}
                            Ok(FactoryPairPassResult::Resolved(pass)) => {
                                for contact in pass.contacts {
                                    if let Some(frame) = specialized_actor_tasks
                                        .apply_factory_pair_arrival(
                                            em,
                                            &mut world_fx,
                                            &mut gameplay_notifications,
                                            pass.factory_id,
                                            contact.scientist_id,
                                            retail_tick,
                                        )
                                    {
                                        log!(
                                            "Factory pair arrival scientist {}: {frame:?}",
                                            contact.scientist_id
                                        );
                                    }
                                }
                            }
                            Err(error) if !factory_pair_error_reported => {
                                eprintln!(
                                    "Factory pair walker stopped at unresolved data: {error:?}"
                                );
                                factory_pair_error_reported = true;
                            }
                            Err(_) => {}
                        }
                    }
                }

                // The currently closed general active-pair subset uses the
                // persistent player entity as subject.
                // The production adapter snapshots the complete local live
                // order and commits the active-solid response/damage pass
                // only after every reached callback and model program
                // resolves. Stateful or remote paths remain a fail-closed
                // boundary for that solid subsystem.
                let active_pair_pass_succeeded = if !player_active_solid_pass_required(
                    current_level_id,
                ) {
                    // Intro2 / no-world: keep the Power-Up path available
                    // without a recovered solid census.
                    true
                } else if !player_hull.dying {
                    match em.resolve_player_active_contacts(
                        &session.cache,
                        v2k_game::player_active_contact::PlayerActivePairFrame {
                            resources: &session.cache,
                            retail_tick: retail_tick,
                            player_craft: &player_craft,
                            player_hull: &mut player_hull,
                            scheduler: &mut specialized_actor_tasks,
                            world_fx: &mut world_fx,
                            notifications: &mut gameplay_notifications,
                            extra_lives: player_campaign_progress.extra_lives(),
                        },
                    ) {
                        Ok(PlayerActivePairPass {
                            core,
                            unsupported_presentation,
                            sounds,
                            descriptor_contacts,
                        }) => {
                            for sound in sounds {
                                world_fx.queue_fixed_positional_sound_raw(
                                    sound.sound_id,
                                    sound.position_raw,
                                );
                            }
                            if !descriptor_contacts.is_empty() {
                                log!("Player descriptor contacts: {descriptor_contacts:?}");
                            }
                            if core.dispositions.iter().any(|disposition| {
                                matches!(
                                    disposition,
                                    v2k_game::active_pair::PairCandidateDisposition::Resolved { .. }
                                )
                            }) {
                                log!("Player active contacts: {:?}", core.dispositions);
                            }
                            if !unsupported_presentation.is_empty() {
                                log!(
                                    "Unsupported player contact presentation: {unsupported_presentation:?}"
                                );
                            }
                            true
                        }
                        Err(error) if !active_pair_contact_error_reported => {
                            eprintln!(
                                "Player active contact stopped at unresolved data: {error:?}"
                            );
                            active_pair_contact_error_reported = true;
                            false
                        }
                        Err(_) => false,
                    }
                } else {
                    false
                };

                // 10C10 can run in the early physical or late particle/pair
                // phase. Its constructor is synchronous; this idempotent
                // publication attaches the BB8 clock without charging the
                // current frame's already-completed controlled visit.
                if player_hull.dying {
                    match em.begin_player_dying(PlayerCheckedDamageFrame {
                        hull: &mut player_hull,
                        resources: &session.cache,
                        world_fx: &mut world_fx,
                        retail_tick,
                        notifications: &mut gameplay_notifications,
                        extra_lives: player_campaign_progress.extra_lives(),
                    }) {
                        Ok(_) => {
                            if player_death.begin() {
                                player_model_id = em.player().and_then(|player| player.model_index);
                                // Local adapter cleanup for the replaced slot0;
                                // 447280 itself does not call the gun controller.
                                primary_weapon = PrimaryWeapon::new();
                                log!("Player destroyed: native wreck constructor, 3000 ms dwell");
                            }
                        }
                        Err(reason) => log!("Player dying constructor blocked: {reason:?}"),
                    }
                }

                // Type-61's proven callback delegates its inventory suffix to
                // the existing bounded transaction. It uses the same oriented
                // models, mutates cloned controller state, and queues accepted
                // allocations for the following update's live-list splice.
                // This suffix is atomic within its own subsystem; a malformed
                // pickup cannot roll back a solid pass already committed, so
                // it is attempted only after that earlier pass succeeds.
                if active_pair_pass_succeeded {
                    match em.resolve_player_power_up_contacts(
                        &session.cache,
                        retail_tick,
                        PlayerPowerUpRecipient {
                            craft: &mut player_craft,
                            weapon_inventory: &mut weapon_inventory,
                            capabilities: &mut player_capabilities,
                            hull: &mut player_hull,
                            campaign_progress: &mut player_campaign_progress,
                        },
                    ) {
                        Ok(pass) => {
                            let feedback = apply_power_up_contact_feedback(
                                &pass,
                                &weapon_inventory,
                                &mut player_craft,
                                &mut gameplay_notifications,
                                &mut world_fx,
                                retail_tick,
                            );
                            if feedback.hull_state_changed {
                                em.sync_player_hull_collision_state(&player_hull);
                            }
                            if pass.accepted().any(|contact| {
                                matches!(contact,
                                    AcceptedPowerUpContact::Trophy { acquisition, .. }
                                        if acquisition.restored_hull)
                            }) {
                                // Explicit requested clock policy. The native
                                // hidden trophy bit remains separate from bit8.
                                main_base_level_abort
                                    .stop_time_trophy_countdown_for_hidden_pickup();
                            }
                            if feedback.targetter_acquired {
                                targetter.install();
                            }
                            if !pass.contacts.is_empty() {
                                log!("Power Up contacts: {:?}", pass.contacts);
                            }
                        }
                        Err(error) if !power_up_contact_error_reported => {
                            eprintln!("Power Up contact disabled by unresolved data: {error:?}");
                            power_up_contact_error_reported = true;
                        }
                        Err(_) => {}
                    }
                }

                // 42DD10 snapshots saved/abort state once, then visits authored
                // records in order. An earlier route returns before later casualty
                // text; a casualty abort resumes at the following record.
                let world_saved = player_campaign_progress
                    .current_control_slot()
                    .and_then(|slot| player_campaign_progress.control_slot_world_saved(slot))
                    .unwrap_or(false);
                let mut selector = CampaignSelectorCursor::new(
                    CampaignCasualtyFrame {
                        census: em.fun_0042e210_capability_census(),
                        world_saved,
                        abort_active: main_base_level_abort.abort_flag_0x28f(),
                    },
                    CampaignSelectorEntry {
                        main_base_request: pending_main_base_abort_origin.is_some(),
                        failed_world_interior_request: pending_failed_world_interior_request,
                    },
                );
                let mut pending_main_base_abort_origin = pending_main_base_abort_origin;
                let mut selected_campaign_route = None;
                loop {
                    let records = session
                        .cache
                        .level_desc()
                        .map(|level| level.campaign_records.as_slice())
                        .unwrap_or(&[]);
                    let step =
                        match selector.next_step(&mut main_base_level_abort.casualty, records) {
                            Ok(step) => step,
                            Err(error) => {
                                eprintln!("Campaign casualty predicate unresolved: {error:?}");
                                break;
                            }
                        };
                    let abort_origin = match step {
                        CampaignSelectorStep::DirectText(text) => {
                            gameplay_notifications
                                .queue_campaign_casualty_text(text, retail_tick as i32);
                            continue;
                        }
                        CampaignSelectorStep::RetryCurrentWorld => {
                            selected_campaign_route = current_level_id
                                .zip(current_world_entry_position_raw)
                                .and_then(|(world, position)| {
                                    FailedWorldRetry::new(world, position)
                                })
                                .map(CampaignTransition::FailedWorldRetry);
                            if selected_campaign_route.is_some() {
                                pending_failed_world_interior_request = false;
                            } else {
                                eprintln!("Failed-world Hive retry blocked: retained entry position unavailable");
                            }
                            break;
                        }
                        CampaignSelectorStep::CheckRoute { record_index } => {
                            // 42E300 checks the resolved player's capability1, not
                            // the hull's dying bit or the session abort flag.
                            if em.player().is_some_and(|player| {
                                player.active && player.capability_flags & 1 != 0
                            }) && !world_complete_results.is_progress_map_active()
                            {
                                selected_campaign_route = campaign_warp
                                    .poll_route(
                                        &mut player_campaign_progress,
                                        retail_tick,
                                        v2k_game::campaign_transition::CampaignRouteVisit::Record(
                                            record_index,
                                        ),
                                    )
                                    .map(CampaignTransition::AuthoredMarker);
                                if selected_campaign_route.is_some() {
                                    break;
                                }
                            }
                            continue;
                        }
                        CampaignSelectorStep::AbortMainBase => {
                            pending_main_base_abort_origin.take()
                        }
                        CampaignSelectorStep::AbortCasualties { record_index } => {
                            log!("Campaign casualty loss: record {record_index}");
                            None
                        }
                        CampaignSelectorStep::Finished => break,
                    };
                    let main_base_abort_admission = begin_main_base_abort_transaction(
                        &mut main_base_level_abort,
                        abort_origin,
                        em,
                        &mut world_fx,
                    );
                    if let MainBaseAbortAdmission::BodyAdmitted(world_control_lease) =
                        main_base_abort_admission
                    {
                        let transaction_id = MainBaseAbortTransactionId::new(
                            world_control_lease.allocation_identity,
                        )
                        .expect("loaded-world generation is nonzero");
                        let terminal_origin = main_base_level_abort.terminal_origin.take();
                        let controller = main_base_level_abort
                            .controller
                            .as_mut()
                            .expect("admitted systemic body retains its controller");
                        let production = execute_campaign_abort(
                            transaction_id,
                            terminal_origin,
                            controller,
                            em,
                            &mut session.cache,
                            &mut static_damage,
                            &mut player_hull,
                            &mut world_fx,
                            &mut specialized_actor_tasks,
                            v2k_game::main_base_abort_production::MainBaseAbortGameplayContext {
                                extra_lives: RetailRuntimeValue::Known(
                                    player_campaign_progress.extra_lives(),
                                ),
                                notifications: &mut gameplay_notifications,
                                retail_tick,
                            },
                        );

                        match production {
                            Ok(report) => {
                                let world_effects = report.world_effects.summary();
                                if report.world_effects.terrain_dirty() || report.terrain.is_some()
                                {
                                    renderer.invalidate_terrain_cache();
                                }
                                if let Some(outcome) = report.terrain {
                                    log!(
                                        "Main Base terminal: darkened {} terrain light cells, activated {} object cells ({} calls, {} kind-10 RNG draws)",
                                        outcome.darkened_light_cells,
                                        outcome.newly_activated_object_cells,
                                        outcome.object_activation_calls,
                                        outcome.kind_10_rng_draws,
                                    );
                                } else {
                                    // Retail ignores FUN_00433E30's zero return
                                    // and still performs the controller handoff.
                                    log!("Main Base terminal: terrain darkness unavailable or already applied");
                                }

                                let request = report.frame_request;
                                sky_background = v2k_game::sky::build_sky_background_for_index(
                                    &session.cache,
                                    request.word_0xb0,
                                );
                                let fog_color = sky_background
                                    .as_ref()
                                    .map(|sky| sky.color)
                                    .unwrap_or([0.5, 0.6, 0.7]);
                                let planes =
                                    v2k_formats::levels::LevelFogPlanes::from_authored_cells(
                                        request.dword_0xb8,
                                        request.dword_0xbc,
                                    );
                                renderer.set_world_model_fog(Some(v2k_render::WorldModelFog {
                                    planes,
                                    color: fog_color,
                                }));
                                log!(
                                    "Main Base terminal: processed {} actors, adopted {} specialized owners, published {}/{} terrain/static effects, stored controller state 5, frame colour {}, model {} submitted",
                                    report.processed_actors.len(),
                                    report.publications.specialized_total(),
                                    world_effects.terrain_attribute_publication_count,
                                    world_effects.static_radial_outcome_count,
                                    request.word_0xb0,
                                    request.word_0xb2,
                                );
                            }
                            Err(failure) => {
                                let fallback = failure.apply_bounded_world_fallback(
                                    v2k_game::main_base_abort_production::MainBaseAbortFallbackRequest {
                                        world_control_lease,
                                        controller,
                                        entities: em,
                                        resources: &mut session.cache,
                                        world_fx: &mut world_fx,
                                    },
                                );
                                main_base_level_abort.compatibility_fallback_actor =
                                    fallback.terminal_actor;
                                for started in &fallback.factories {
                                    log!(
                                        "Main Base terminal fallback: armed Working Factory entity {}",
                                        started.target_id
                                    );
                                }

                                if let Some(outcome) = fallback.terrain {
                                    renderer.invalidate_terrain_cache();
                                    log!(
                                        "Main Base terminal fallback: darkened {} terrain light cells, activated {} object cells ({} calls, {} kind-10 RNG draws)",
                                        outcome.darkened_light_cells,
                                        outcome.newly_activated_object_cells,
                                        outcome.object_activation_calls,
                                        outcome.kind_10_rng_draws,
                                    );
                                } else {
                                    log!(
                                        "Main Base terminal fallback: terrain darkness unavailable or already applied"
                                    );
                                }

                                match fallback.frame_request {
                                    Ok(request) => {
                                        sky_background =
                                            v2k_game::sky::build_sky_background_for_index(
                                                &session.cache,
                                                request.word_0xb0,
                                            );
                                        let fog_color = sky_background
                                            .as_ref()
                                            .map(|sky| sky.color)
                                            .unwrap_or([0.5, 0.6, 0.7]);
                                        let planes = v2k_formats::levels::LevelFogPlanes::from_authored_cells(request.dword_0xb8, request.dword_0xbc);
                                        renderer.set_world_model_fog(Some(v2k_render::WorldModelFog { planes, color: fog_color }));
                                    }
                                    Err(error) => eprintln!(
                                        "Main Base terminal fallback could not submit the controller request: {:?}",
                                        error,
                                    ),
                                }
                                eprintln!(
                                    "Main Base systemic abort used the bounded compatibility body after atomic rejection: {:?}",
                                    fallback.diagnostic,
                                );
                            }
                        }
                        main_base_level_abort.finish_outer_dispatch();
                    }
                }

                // 450344 consumes the first successful authored route.
                if let Some(route) = selected_campaign_route {
                    match route {
                        CampaignTransition::AuthoredMarker(marker) => log!(
                            "Campaign warp contact: {} -> {} at cell ({}, {})",
                            marker.source_level_id,
                            marker.destination_level_id,
                            marker.marker_cell[0],
                            marker.marker_cell[1],
                        ),
                        CampaignTransition::FailedWorldRetry(_) => log!(
                            "Failed-world Hive retry: {} -> {}",
                            route.source_level_id(),
                            route.destination_level_id(),
                        ),
                    }
                    keys_down.clear();
                    mouse_buttons_down.clear();
                    world_complete_results.open_progress_map(route);
                    suspend_world_audio(
                        &mut world_fx,
                        &mut sound_manager,
                        &mut entity_positional_audio,
                        &mut player_fan_audio,
                    );
                    // 450344 selects the destination before 454FF0
                    // builds the map's current-world blink node. Secret
                    // pads use the same phase-5 map as ordinary exits.
                    player_campaign_progress
                        .set_current_control_slot(world_control_slot(route.destination_level_id()));
                    if let Some(ref mp) = music_player {
                        apply_session_music_gate(&config, mp, true);
                    }
                    continue 'main_loop;
                }

                // Cargo callbacks run in the entity pass. Queue their visual
                // and audio work after the particle integration above so a
                // newly emitted class-0x33 sprite is first presented at age
                // zero, as it is in retail's deferred particle allocator.
                let cargo_birth_environment =
                    TerrainCollisionContext::from_current_level_cache(&session.cache)
                        .map_or(ParticleEnvironment::Dry, ParticleEnvironment::Terrain);
                world_fx.process_deferred_cargo_transfers(
                    v2k_game::world_fx::ParticleBirthContext {
                        environment: cargo_birth_environment,
                        retail_tick,
                    },
                );
                if let Some(position) = cargo_full_fx {
                    world_fx.queue_cargo_full_sound(position);
                }
                for event in em.take_cargo_proxy_events() {
                    match event {
                        CargoProxyEvent::Spawned {
                            target, sound_id, ..
                        } => {
                            debug_assert_eq!(sound_id, v2k_game::world_fx::CARGO_TRANSFER_SOUND_ID);
                            world_fx.queue_cargo_transfer_sound(target);
                        }
                        CargoProxyEvent::PositionChanged {
                            proxy_id,
                            position,
                            particle_class,
                            ..
                        } => {
                            debug_assert_eq!(
                                particle_class,
                                v2k_game::world_fx::CARGO_TRANSFER_PARTICLE_CLASS
                            );
                            world_fx.queue_cargo_transfer_particle(position, Some(proxy_id));
                        }
                        CargoProxyEvent::Released { .. } => {}
                    }
                }
                // Cargo entity callbacks happen after the retail particle
                // traversal. Materialize their deferred records now so they
                // present at age zero this frame without entering that pass.
                world_fx.process_pending();
                gameplay_overlay_frame = GameplayOverlayFrame {
                    full_frame_sprite: prepared_full_frame_sprite
                        .as_ref()
                        .map(|command| command.frame),
                    explosion_lights: base_factory_progression.explosion_lights.clone(),
                    static_explosion_lights: static_damage_explosion_lights.clone(),
                    ..Default::default()
                };
                gameplay_overlay_frame.presentation = Some(draw_gameplay_world(
                    &mut *renderer,
                    GameplayWorldFrame {
                        cache: &session.cache,
                        face_colors: &face_colors,
                        em,
                        camera: &camera,
                        camera_mode: if chase_mode {
                            GameplayWorldCameraMode::RetailChase
                        } else {
                            GameplayWorldCameraMode::Free
                        },
                        native_viewport: if chase_mode {
                            chase_camera.native_viewport()
                        } else {
                            None
                        },
                        world_projection: Some(world_projection),
                        sky: sky_background.as_ref(),
                        terrain_frames: terrain_frames.as_ref(),
                        water_frames: water_frames.as_ref(),
                        terrain_lights: &mut terrain_lights,
                        world_fx: &mut world_fx,
                        explosion_lights: &base_factory_progression.explosion_lights,
                        static_explosion_lights: &static_damage_explosion_lights,
                        abort_frame: main_base_level_abort.submitted_frame_request(),
                        specialized_actor_tasks: &mut specialized_actor_tasks,
                        pending_entity_weapon_fire: &mut pending_entity_weapon_fire,
                        entity_weapon_error_reported: &mut native_entity_weapon_error_reported,
                        player_craft: &player_craft,
                        player_hull: &player_hull,
                        player_shield: GameplayWorldShield::Live(&mut player_shield),
                        particles: GameplayWorldParticles::Live,
                        player_death: &player_death,
                        player_model_id,
                        retail_tick,
                        elapsed_micros,
                        targetter: GameplayWorldTargetter {
                            runtime: &mut targetter,
                            enabled: config.targetter,
                        },
                    },
                ));

                // The retail queue sorts positive world depth ahead of the
                // sequence's key zero, then submits radar/HUD later. Preserve
                // that over-world, under-HUD composition explicitly.
                if let Some(command) = prepared_full_frame_sprite.as_ref() {
                    draw_prepared_full_frame_sprite_command(&mut *renderer, command);
                }

                // Overlay 51's completion descriptor replaces the in-game HUD
                // chain. World, water, and particles still draw underneath.
                play_world_audio(
                    WorldAudioFrame {
                        cache: &session.cache,
                        entities: em,
                        camera: &camera,
                        elapsed_micros,
                    },
                    &mut world_fx,
                    &mut sound_manager,
                    &mut entity_positional_audio,
                );

                let overlay_51_results = world_complete_results.replaces_ingame_hud();

                if config.hud && player_hull.health_raw > 0 && !overlay_51_results {
                    if let (Some(radar), Some(terrain)) =
                        (gameplay_radar.as_ref(), session.cache.level_terrain_radar())
                    {
                        if let Some(frame) =
                            radar.hud_frame(terrain, &*em, retail_tick, &mut || {
                                world_fx.next_shared_retail_random_u16()
                            })
                        {
                            draw_gameplay_radar(&mut *renderer, &frame);
                            gameplay_overlay_frame.radar = Some(frame);
                        }
                    }
                }

                // FUN_00452270 case 2 reads DAT_004fecdc selector 3 at draw
                // time, after class-14 has already set the dying bit.
                let census = em.fun_0042e210_capability_census();
                gameplay_notifications.set_people_left(fun_0042dd10_selector3_rescued(
                    census.cap_0x400,
                    census.cap_0x800,
                ));

                if config.hud {
                    if player_hull.health_raw > 0 && !overlay_51_results {
                        if let (Some(resources), Some(layout)) =
                            (gameplay_hud_resources.as_ref(), gameplay_hud_layout)
                        {
                            let cargo_models = entity_manager
                                .as_ref()
                                .map(|manager| manager.player_cargo_hud_models())
                                .unwrap_or_default();
                            let weapon_roster =
                                GameplayHudWeaponRoster::from_inventory(&weapon_inventory);
                            let frame = gameplay_hud.frame(
                                resources,
                                layout,
                                player_craft.fuel_raw,
                                player_hull.health_raw,
                                retail_tick,
                                gameplay_hud_elapsed_micros,
                                &weapon_roster,
                                &cargo_models,
                                main_base_level_abort
                                    .controller
                                    .map(|controller| controller.time_trophy_runtime())
                                    .unwrap_or_default(),
                            );
                            draw_gameplay_hud(
                                &mut *renderer,
                                &session.cache,
                                &face_colors,
                                resources,
                                &frame,
                                menu_fonts.as_ref().map(|fonts| &fonts.normal),
                            );
                            gameplay_overlay_frame.hud = Some(frame);
                        }
                    }
                    {
                        // FUN_00452CB0 first draws the world's arrival records
                        // (its strings up to `#`) unless its control slot is
                        // already completed.
                        let world_records: Vec<&str> = current_level_id
                            .and_then(world_control_slot)
                            .filter(|&slot| {
                                player_campaign_progress
                                    .control_slot_bits(slot)
                                    .is_none_or(|bits| bits & 1 == 0)
                            })
                            .and_then(|_| session.cache.level())
                            .map(|level| {
                                level
                                    .strings
                                    .iter()
                                    .map(String::as_str)
                                    .take_while(|record| !record.starts_with('#'))
                                    .collect()
                            })
                            .unwrap_or_default();
                        let notification_presentation = gameplay_notifications
                            .presentation_with_world_text(
                                retail_tick as i32,
                                &mut text_typewriter_cadence,
                                &world_records,
                                |id| session.cache.global_string(id),
                            );
                        if notification_presentation.play_typewriter_sound && menu_fonts.is_some() {
                            if let Some(sound_manager) = sound_manager.as_mut() {
                                sound_manager.play_centered_sound_with_gain(
                                    GAMEPLAY_TEXT_TYPE_SOUND_ID,
                                    0.5,
                                );
                            }
                        }
                        if let Some(fonts) = menu_fonts.as_ref() {
                            draw_gameplay_notifications(
                                &mut *renderer,
                                fonts,
                                &notification_presentation.lines,
                                GameplayTextContext::WorldOverlay,
                            );
                            gameplay_overlay_frame
                                .notifications
                                .extend(notification_presentation.lines);
                        }
                    }
                    if world_complete_results.has_hive_statistics() {
                        let results_presentation = world_complete_results
                            .presentation_with_typewriter(
                                &mut text_typewriter_cadence,
                                retail_tick as i32,
                                |id| session.cache.global_string(id),
                            );
                        if results_presentation.play_typewriter_sound && menu_fonts.is_some() {
                            if let Some(sound_manager) = sound_manager.as_mut() {
                                sound_manager.play_centered_sound_with_gain(
                                    GAMEPLAY_TEXT_TYPE_SOUND_ID,
                                    0.5,
                                );
                            }
                        }
                        if let Some(fonts) = menu_fonts.as_ref() {
                            draw_gameplay_notifications(
                                &mut *renderer,
                                fonts,
                                &results_presentation.lines,
                                GameplayTextContext::WorldOverlay,
                            );
                            gameplay_overlay_frame
                                .notifications
                                .extend(results_presentation.lines);
                        }
                    }
                }

                if let Some(shell) = post_intro_world_cover.as_mut() {
                    draw_klaus_handoff_overlay(
                        &mut *renderer,
                        KlausHandoffFrame {
                            shell,
                            cache: &session.cache,
                            colors: &face_colors,
                            retail_tick,
                        },
                    );
                    if shell.take_transition_ready() {
                        post_intro_world_cover = None;
                    }
                }
                renderer.present();
            }
        }

        if debug_panel.is_visible() {
            let lines = build_debug_panel_lines(
                &state,
                current_level_id,
                &session,
                entity_manager.as_ref(),
                &mut world_fx,
                &camera,
                &player_craft,
                player_model_id,
                chase_mode,
                dt,
                retail_tick,
            );
            debug_panel.render(&lines);
        }

        // Frame rate limiter
        let frame_time = now.elapsed();
        if frame_time < target_frame_time {
            std::thread::sleep(target_frame_time - frame_time);
        }
    }

    log!("Exiting.");
    Ok(())
}

/// Construct the complete active-gameplay context expected by
/// `FUN_0044FFA0` -> `FUN_0040ED10`. If a level has not installed both Section
/// 10 terrain and its Section 9 static-object table, retain the explicit
/// null-context target rather than running a partial clearance approximation.
fn gameplay_chase_terrain(
    cache: &v2k_game::resource_cache::ResourceCache,
) -> Option<ChaseTerrainContext<'_>> {
    Some(ChaseTerrainContext {
        terrain: cache.terrain()?,
        terrain_objects: cache.terrain_objects()?,
        model_pool: cache,
    })
}

fn find_banner_avi(data_dir: &Path, detail: GraphicsDetail) -> PathBuf {
    let names = match detail {
        GraphicsDetail::High => ["BANNERHI.AVI", "BANNERLO.AVI"],
        GraphicsDetail::Low => ["BANNERLO.AVI", "BANNERHI.AVI"],
    };
    for name in names {
        for root in [data_dir.to_path_buf(), data_dir.join("..")] {
            let candidate = root.join(name);
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    data_dir.join(names[0])
}

/// Locate the separately stored original-disc audio without making it part of
/// the port's runtime-data directory. A source checkout may keep it in a
/// sibling `cdaudio/`, one directory above the data root.
/// The old `port/music/` location remains a compatibility fallback.
fn find_cd_audio_dir(data_dir: &Path) -> Option<PathBuf> {
    let soundtrack = v2k_render::music::locate_soundtrack(data_dir);
    (soundtrack.available_count() != 0).then_some(soundtrack.directory)
}

fn debug_ride_uses_wave_surface(water_enabled: bool, attached_cargo_mass: u32) -> bool {
    water_enabled && attached_cargo_mass <= 99
}

#[allow(clippy::too_many_arguments)]
fn build_debug_panel_lines(
    state: &GameState,
    level_id: Option<u32>,
    session: &v2k_game::session::GameSession,
    entities: Option<&v2k_game::entity::EntityManager>,
    world_fx: &mut v2k_game::world_fx::WorldFx,
    camera: &Camera,
    craft: &PlayerCraft,
    player_model_id: Option<usize>,
    chase_mode: bool,
    dt: f32,
    retail_tick: u32,
) -> Vec<String> {
    let state_name = match state {
        GameState::Intro => "INTRO",
        GameState::Menu(_) => "MENU",
        GameState::Loading { .. } => "LOADING",
        GameState::OpeningCinematic { .. } => "OPENING CINEMATIC",
        GameState::PostIntro { .. } => "POST INTRO / BEFORE LOAD",
        GameState::Paused { .. } => "PAUSED",
        GameState::ReturningToFrontend => "RETURNING TO FRONTEND",
        GameState::Playing => "PLAYING",
    };
    let level = session.cache.level_desc();
    let level_name = match (level_id, level.map(|descriptor| descriptor.name.as_str())) {
        (Some(13), Some("")) => "Peasant",
        (_, Some(name)) => name,
        _ => "NONE",
    };
    let fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };
    let attached_cargo_mass =
        entities.and_then(|manager| match manager.attached_cargo_mass_state() {
            RetailRuntimeValue::Known(mass) => Some(mass),
            RetailRuntimeValue::Unresolved => None,
        });
    let beam_timer_ms = entities.map(|manager| manager.beam_timer_ms()).unwrap_or(0);
    let active_entities = entities
        .map(|manager| manager.iter().filter(|entity| entity.active).count())
        .unwrap_or(0);
    let total_entities = entities
        .map(|manager| manager.iter_all().count())
        .unwrap_or(0);

    let mut lines = vec![
        "[V2000 LIVE RE DIAGNOSTICS]".to_string(),
        "F12 HIDE  CLOSE BUTTON HIDES PANEL".to_string(),
        format!("STATE: {state_name}"),
        format!(
            "LEVEL: {}  {level_name}",
            level_id.map_or(-1, |id| id as i32)
        ),
        format!("FRAME: {:6.2} MS  FPS {:6.1}", dt * 1000.0, fps),
        format!(
            "CLOCK: {:10.3} S  TICK50 {:10}",
            retail_tick as f64 / 50.0,
            retail_tick
        ),
        "[CRAFT]".to_string(),
    ];

    if let Some(player) = entities.and_then(|manager| manager.player()) {
        let self_mass = u32::from(player.mass_raw);
        let total_mass = attached_cargo_mass.map(|mass| self_mass.wrapping_add(mass));
        let horizontal_speed = (player.velocity[0] * player.velocity[0]
            + player.velocity[2] * player.velocity[2])
            .sqrt();
        let raw = [
            (player.position[0] * 256.0).round() as i32,
            (player.position[1] * 256.0).round() as i32,
            (player.position[2] * 256.0).round() as i32,
        ];
        let cell_x = player.position[0].floor().rem_euclid(256.0) as usize;
        let cell_z = player.position[2].floor().rem_euclid(256.0) as usize;
        lines.extend([
            format!(
                "MODE: {}  MODEL: {}",
                craft.mode.label(),
                player_model_id.map_or(-1, |id| id as i32)
            ),
            format!(
                "POS:  {:8.3} {:8.3} {:8.3}",
                player.position[0], player.position[1], player.position[2]
            ),
            format!("RAW8.8: {:+7} {:+7} {:+7}", raw[0], raw[1], raw[2]),
            format!(
                "CELL: {:3} {:3}  ADDRESS {:5}",
                cell_x,
                cell_z,
                cell_x * 256 + cell_z
            ),
            format!(
                "VEL:  {:+8.3} {:+8.3} {:+8.3}",
                player.velocity[0], player.velocity[1], player.velocity[2]
            ),
            format!(
                "SPEED H/V: {:7.3} {:+7.3}",
                horizontal_speed, player.velocity[1]
            ),
            format!(
                "HEADING: {:+8.4} RAD {:+7.2} DEG",
                player.heading,
                player.heading.to_degrees()
            ),
            format!(
                "PITCH/ROLL: {:+7.3} {:+7.3}",
                craft.body_pitch, craft.body_roll
            ),
            format!(
                "GUN: {:+7.3} RAD  FAN: {:7.3}",
                craft.gun_barrel_rad(),
                craft.spin_angle
            ),
            format!("FUEL: {:9} / 200000", craft.fuel_raw),
            format!(
                "MASS SELF/CARGO/TOTAL: {self_mass}/{}/{}",
                attached_cargo_mass.map_or_else(|| "?".to_string(), |mass| mass.to_string()),
                total_mass.map_or_else(|| "?".to_string(), |mass| mass.to_string())
            ),
            format!("BEAM TIMER: {beam_timer_ms:+4} MS"),
            "[SURFACE UNDER CRAFT]".to_string(),
        ]);

        if let Some(terrain) = session.cache.terrain() {
            let ground = terrain.height_at(player.position[0], player.position[2]);
            let sea = terrain.sea_level_world_y();
            let uses_wave_surface = attached_cargo_mass
                .is_some_and(|mass| debug_ride_uses_wave_surface(terrain.water_enabled(), mass));
            let ride = if uses_wave_surface {
                f32::from(v2k_formats::terrain::wave_surface_raw(
                    (player.position[0] * 256.0).round() as i32 as i16,
                    (player.position[2] * 256.0).round() as i32 as i16,
                    retail_tick as i32,
                    (sea * 256.0).round() as i32 as i16,
                    (ground * 256.0).round() as i32 as i16,
                )) / 256.0
            } else {
                ground
            };
            let cell = terrain.cell(cell_x, cell_z);
            lines.extend([
                format!("TERRAIN Y: {:+9.4}", ground),
                format!("SEA Y: {:+9.4} RAW {:+11}", sea, terrain.header[0]),
                format!(
                    "RIDE Y: {:+9.4}  SOURCE {}",
                    ride,
                    if uses_wave_surface { "WAVE" } else { "TERRAIN" }
                ),
                format!("CRAFT - RIDE: {:+9.4}", player.position[1] - ride),
                format!(
                    "TILE HEIGHT/ATTR/TYPE: {}/{}/{}",
                    cell.map_or(0, |value| value.height as i8 as i32),
                    cell.map_or(0, |value| value.attribute as i32),
                    cell.map_or(0, |value| value.terrain_type as i32),
                ),
            ]);
        } else {
            lines.push("NO TERRAIN LOADED".to_string());
        }

        let splash = world_fx.take_splash_path_counters();
        lines.extend([
            "[SPLASH]".to_string(),
            format!(
                "POOL {}/{}  PROBES {}  SPRAY {}  VIS {}  SOUNDS {}",
                splash.pool_live,
                splash.pool_capacity,
                splash.probes_spawned,
                splash.spray_particles_spawned,
                splash.spray_presented,
                splash.splash_sounds
            ),
            format!(
                "ENTRIES {}  HARD {}  RINGS {}  REJ {}",
                splash.water_entries,
                splash.hard_entries,
                splash.rings_live,
                splash.ring_rejections
            ),
        ]);
    } else {
        lines.push("NO PLAYER ENTITY".to_string());
    }

    let camera_distance = entities
        .and_then(|manager| manager.player())
        .map(|player| {
            let dx = v2k_core::world::delta(camera.position[0], player.position[0]);
            let dz = v2k_core::world::delta(camera.position[2], player.position[2]);
            (dx * dx + dz * dz).sqrt()
        })
        .unwrap_or(0.0);

    lines.extend([
        "[CAMERA]".to_string(),
        format!(
            "POS:  {:8.3} {:8.3} {:8.3}",
            camera.position[0], camera.position[1], camera.position[2]
        ),
        format!("YAW/PITCH: {:+8.4} {:+8.4}", camera.yaw, camera.pitch),
        format!("DIST XZ: {:8.3}", camera_distance),
        format!(
            "CHASE: {}  ASPECT: {:.4}",
            if chase_mode { "YES" } else { "NO" },
            camera.aspect
        ),
        "[WORLD]".to_string(),
        format!("ENTITIES ACTIVE/TOTAL: {active_entities}/{total_entities}"),
    ]);
    if let Some(descriptor) = level {
        lines.extend([
            format!(
                "DRAW DEPTH: {}  WORLD STYLE: {}",
                descriptor.terrain_draw_depth, descriptor.world_style
            ),
            format!("TERRAIN SPRITE BASE: {}", descriptor.terrain_sprite_base),
            format!(
                "SKY COLOR/MODEL: {}/{}",
                descriptor.sky_color_index, descriptor.sky_model
            ),
        ]);
    }
    lines
}

struct GameplayWorldTargetter<'a> {
    runtime: &'a mut v2k_game::targetter::TargetterRuntime,
    enabled: bool,
}

/// Retail's world-axis scan follows its fixed compass-bearing chase camera.
/// The port's developer free camera retains its camera-relative scan window.
#[derive(Clone, Copy)]
enum GameplayWorldCameraMode {
    RetailChase,
    Free,
}

enum GameplayWorldShield<'a> {
    Live(&'a mut v2k_game::player_shield::PlayerShieldPresentation),
    Frozen(Option<v2k_game::player_shield::PlayerShieldFrame>),
}

enum GameplayWorldParticles<'a> {
    Live,
    Frozen(&'a ParticlePresentationFrame),
}

struct GameplayWorldPresentation {
    shield: Option<v2k_game::player_shield::PlayerShieldFrame>,
    particles: ParticlePresentationFrame,
}

struct GameplayWorldFrame<'a> {
    cache: &'a v2k_game::resource_cache::ResourceCache,
    face_colors: &'a v2k_game::model_color::ModelMaterialCache,
    em: &'a mut v2k_game::entity::EntityManager,
    camera: &'a Camera,
    camera_mode: GameplayWorldCameraMode,
    native_viewport: Option<v2k_game::native_model_frame::NativeWorldViewport>,
    world_projection: Option<WorldProjection>,
    sky: Option<&'a v2k_game::sky::SkyBackground>,
    terrain_frames: Option<&'a v2k_render::TerrainFrames>,
    water_frames: Option<&'a v2k_render::WaterFrames>,
    terrain_lights: &'a mut v2k_render::TerrainLightWindow,
    world_fx: &'a mut WorldFx,
    explosion_lights: &'a [TerrainExplosionLight],
    static_explosion_lights: &'a [TerrainExplosionLight],
    abort_frame: Option<MainBaseAbortFrameRequest>,
    specialized_actor_tasks: &'a mut SpecializedActorTaskScheduler,
    pending_entity_weapon_fire: &'a mut Vec<PendingEntityWeaponFire>,
    entity_weapon_error_reported: &'a mut bool,
    player_craft: &'a PlayerCraft,
    player_hull: &'a PlayerHull,
    player_shield: GameplayWorldShield<'a>,
    particles: GameplayWorldParticles<'a>,
    player_death: &'a PlayerDeathLifecycle,
    player_model_id: Option<usize>,
    retail_tick: u32,
    elapsed_micros: u32,
    targetter: GameplayWorldTargetter<'a>,
}

/// 11720 visits attached allocations too: 11400 tests body bit800, not
/// relation bit1000. 18440 clears800 for a hidden Sub-J slot, whereas the
/// spider's authored policy1 slot retains it. Keep the existing unparented
/// admission until the complete body/effect presentation gate is owned.
fn world_body_submission_eligible(entity: &Entity) -> bool {
    entity.attached_to.is_none()
        || entity.collision.state_flags_at_0x08.masked(0x800) == RetailRuntimeValue::Known(0x800)
}

#[cfg(test)]
mod world_body_submission_tests {
    use super::*;

    #[test]
    fn attached_body_admission_uses_proven_render_bit_without_changing_unparented_policy() {
        for entity_type in [8, 9, 68] {
            for parent in [None, Some(42)] {
                for render_bit in [None, Some(0), Some(0x800)] {
                    let mut entity = Entity::unresolved_port_entity(
                        7,
                        v2k_game::entity::EntityKind::Enemy,
                        entity_type,
                    );
                    entity.attached_to = parent;
                    entity
                        .collision
                        .state_flags_at_0x08
                        .overwrite(0x1000, if parent.is_some() { 0x1000 } else { 0 });
                    if let Some(bit) = render_bit {
                        entity.collision.state_flags_at_0x08.overwrite(0x800, bit);
                    }
                    assert_eq!(
                        world_body_submission_eligible(&entity),
                        parent.is_none() || render_bit == Some(0x800),
                        "type{entity_type} parent{parent:?} render{render_bit:?}",
                    );
                }
            }
        }
    }
}

/// A missing native owner suppresses that selector only. Announce each
/// model/boundary once so repeated visits do not flood the runtime log.
fn report_actor_emitter_boundary(
    entity_id: u32,
    model_id: usize,
    boundary: v2k_game::actor_emitter_external_frame::ActorEmitterExternalFrameBoundary,
) {
    static REPORTED: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashSet<(usize, String)>>,
    > = std::sync::OnceLock::new();
    let key = (model_id, format!("{boundary:?}"));
    let mut reported = REPORTED
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if reported.insert(key) {
        eprintln!("Entity {entity_id} model{model_id} Sub-E selector suppressed: {boundary:?}");
    }
}

fn draw_gameplay_world(
    renderer: &mut dyn v2k_render::Renderer,
    frame: GameplayWorldFrame<'_>,
) -> GameplayWorldPresentation {
    let GameplayWorldFrame {
        cache,
        face_colors,
        em,
        camera,
        camera_mode,
        native_viewport,
        world_projection,
        sky,
        terrain_frames,
        water_frames,
        terrain_lights,
        world_fx,
        explosion_lights,
        static_explosion_lights,
        abort_frame,
        specialized_actor_tasks,
        pending_entity_weapon_fire,
        entity_weapon_error_reported,
        player_craft,
        player_hull,
        player_shield,
        particles,
        player_death,
        player_model_id,
        retail_tick,
        elapsed_micros,
        targetter,
    } = frame;

    let mut presented_shield = None;
    // Render
    let fog_color = cache
        .fog_gradient()
        .and_then(|f| f.last())
        .map(|e| [e.r as f32 / 255.0, e.g as f32 / 255.0, e.b as f32 / 255.0])
        .unwrap_or([0.5, 0.6, 0.7]);

    renderer.begin_scene(RenderScene::World);
    let projection_effect = world_projection_effect(
        camera.position[1],
        cache.terrain().map(|terrain| terrain.sea_level_raw()),
        retail_tick,
    );
    renderer.set_projection_effect(projection_effect);
    let clear_color = sky.map(|sky| sky.color).unwrap_or(fog_color);
    renderer.clear(clear_color[0], clear_color[1], clear_color[2]);
    renderer.set_camera(camera);
    renderer.set_scene_projection_authority(
        v2k_game::world_projection::scene_projection_authority(
            world_projection,
            native_viewport.is_some(),
            matches!(camera_mode, GameplayWorldCameraMode::RetailChase),
            renderer.viewport_size(),
            projection_effect,
        ),
    );
    renderer.set_native_world_viewport(native_viewport.map(|viewport| viewport.words()));
    draw_authored_sky_model(
        renderer,
        cache,
        abort_frame,
        face_colors,
        camera,
        retail_tick,
    );

    // 530D0/53570 and pre-actor light writers precede 53760. Actor
    // commands capture this field; 53A60 adds particle light before terrain.
    begin_particle_terrain_lights(terrain_lights, camera);
    for light in explosion_lights {
        terrain_lights.add_explosion(
            i32::from(light.x_raw),
            i32::from(light.z_raw),
            i32::from(light.radius_raw),
        );
    }
    for light in static_explosion_lights {
        terrain_lights.add_explosion(
            i32::from(light.x_raw),
            i32::from(light.z_raw),
            i32::from(light.radius_raw),
        );
    }

    // Non-player entities: exact Section-12 type → global-model
    // binding, with the retained physical matrix when proven and
    // the existing yaw policy otherwise. The
    // player craft is drawn separately below (per-frame materialized
    // hull + oriented body + spinning fan sub-models).
    let player_entity_id = em.player().map(|p| p.id);
    let terrain_scan_dimensions = terrain_frames
        .map(|frames| (frames.scan_columns, frames.scan_rows))
        .or_else(|| {
            cache
                .level_desc()
                .map(|desc| v2k_render::terrain_tiles::scan_dimensions(desc.terrain_draw_depth))
        })
        .unwrap_or((52, 30));
    // Retail's presentation traversal publishes FUN_00411400's
    // detail bits after the current FUN_00413500 simulation pass.
    // Registered actor owners therefore consume this
    // camera result on their next scheduler visit. In particular a nearby
    // peasant needs the detailed callback to advance walking/help frames.
    let entity_view = RetailViewDetailContext::from_world(
        camera.position,
        camera.view_matrix()[9],
        terrain_scan_dimensions,
    );
    specialized_actor_tasks.publish_presented_view_detail(em, entity_view);
    // FUN_0041D360 writes PRIMARY_CACHE_VALID / WAIT_FOR_DEPS on the live
    // Sub-H record. The first-world body loop used to resolve a copy through
    // with_external_frame and never commit, so Type17 gait stayed at rest
    // while 12DA0 translated the thorax. Intro2 already uses the draw-owned
    // writeback; share that recovered path here.
    let mut actor_cursor = em.retail_live_order_ids().next();
    let mut player_shield = Some(player_shield);
    let mut model_submissions = v2k_game::model_tree::ModelTreeSubmissionBuffer::default();
    let mut wreck_ids = Vec::new();
    let player_shade_shift = em.player().map(|player| {
        world_model_shade_shift(player.position, cache.terrain(), Some(terrain_lights))
    });
    while let Some(entity_id) = actor_cursor {
        if Some(entity_id) == player_entity_id {
            // The live craft uses its morph/fan child policy. The authored
            // dying flag switches type 46 to model slot 1 (play4ded), whose
            // own linked hierarchy must be drawn without grafting live
            // player4 fan/gun transforms onto the wreck pieces.
            {
                if let Some(player) = em.player() {
                    let draw_pos = camera_relative(camera, player.position);
                    let player_orientation = player_model_orientation(player_craft, player);
                    let resolved = player
                        .model_index
                        .or(player_model_id)
                        .and_then(|id| cache.global_model(id).map(|_| id));
                    if let Some(id) = resolved {
                        let animation = if player_hull.dying {
                            PlayerCraftDrawAnimation::Dying {
                                control_output_7_raw: i32::from(
                                    player_death.control_output_7_raw(),
                                ),
                            }
                        } else {
                            PlayerCraftDrawAnimation::Live(player_craft)
                        };
                        let origins = draw_player_craft(
                            renderer,
                            PlayerCraftDrawFrame {
                                cache,
                                colors: face_colors,
                                camera,
                                model_id: id,
                                animation,
                                position: draw_pos,
                                body: player_orientation,
                                shade_shift: player_shade_shift.unwrap_or(0),
                                retail_tick,
                                native_viewport,
                                actor_origin_raw: player.position_raw(),
                                body_basis: player.physical_body_basis_q31(),
                                submissions: &mut model_submissions,
                            },
                        );
                        publish_entity_weapon_bodies(
                            EntityWeaponPresentationFrame {
                                entities: em,
                                cache,
                                world_fx,
                                actor_tasks: specialized_actor_tasks,
                                retail_tick,
                                error_reported: entity_weapon_error_reported,
                            },
                            pending_entity_weapon_fire,
                            &origins,
                        );
                    }
                    if resolved.is_some() {
                        presented_shield = match player_shield
                            .take()
                            .expect("one player draw per intrusive pass")
                        {
                            GameplayWorldShield::Live(runtime) => runtime.prepare_frame(
                                player_hull.pre_health_damage_buffer_raw,
                                retail_tick,
                                &mut || world_fx.next_shared_retail_random_u16(),
                            ),
                            GameplayWorldShield::Frozen(frame) => frame,
                        };
                        if let Some(shield) = presented_shield {
                            use v2k_game::player_shield::{
                                PLAYER_SHIELD_FIRST_SPRITE_ID, PLAYER_SHIELD_MODEL_ID,
                            };
                            let sprite_id = PLAYER_SHIELD_FIRST_SPRITE_ID
                                + u16::from(shield.damage_flash_level);
                            if cache.global_model(PLAYER_SHIELD_MODEL_ID).is_some()
                                && cache.global_sprite(sprite_id).is_some()
                            {
                                ModelTreeRenderer::new_world(
                                    renderer,
                                    cache,
                                    face_colors,
                                    GAMEPLAY_MODEL_SCALE,
                                    Some(MenuSceneLight::NEUTRAL),
                                    retail_tick as i32,
                                )
                                .with_submission_buffer(&mut model_submissions)
                                .with_view(camera.into())
                                .with_shade_shift(player_shade_shift.unwrap_or(0))
                                .draw_linked(
                                    PLAYER_SHIELD_MODEL_ID,
                                    shield.orientation(player_orientation),
                                    draw_pos,
                                    0,
                                    None,
                                    &shield.animation_vars(),
                                );
                            } else {
                                static REPORTED: std::sync::Once = std::sync::Once::new();
                                REPORTED.call_once(|| {
                                eprintln!(
                                    "Player shield aura suppressed: authored model245/sprite missing"
                                )
                            });
                            }
                        }
                    }
                }
            }
        }
        let relation_origin_raw = em
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .map(|entity| entity.collision.recent_relation_id_at_0x60)
            .map_or(RetailRuntimeValue::Unresolved, |relation| match relation {
                RetailRuntimeValue::Known(id) => RetailRuntimeValue::Known(id.and_then(|id| {
                    em.iter_all()
                        .find(|entity| entity.id == id)
                        .map(|entity| entity.position_raw())
                })),
                RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
            });
        'draw_actor: {
            let Some(entity) = em.entity_mut(entity_id) else {
                break 'draw_actor;
            };
            if !world_body_submission_eligible(entity)
                || !entity.active
                || entity.kind == v2k_game::entity::EntityKind::Trigger
                || Some(entity.id) == player_entity_id
                || world_fx
                    .exploding_rings()
                    .iter()
                    .any(|ring| ring.associated_entity_id() == entity.id)
            {
                break 'draw_actor;
            }

            let Some(model_id) = entity.model_index else {
                break 'draw_actor;
            };
            // Draw each entity at its nearest toroidal image relative
            // to the camera, matching the engine's camera-relative
            // signed-16-bit unwrap.
            let draw_pos = camera_relative(camera, entity.position);
            // FUN_00411400's main-model window uses wrapped world X/Z, with a
            // height-dependent rear plane. Rotating that window with camera yaw
            // clips diagonal buildings before their authored scan boundary.
            // This shares only the geometric classification: the existing actor
            // eligibility and receipt-owned state publication remain above.
            let within_scan = match camera_mode {
                GameplayWorldCameraMode::RetailChase => {
                    entity_view.classify(entity.position_raw()) == RetailViewDetail::Full
                }
                GameplayWorldCameraMode::Free => free_camera_entity_view_contains_position(
                    camera.position,
                    camera.forward(),
                    terrain_scan_dimensions,
                    draw_pos,
                ),
            };
            if !within_scan {
                break 'draw_actor;
            }
            let orientation = cache
                .global_model(model_id)
                .filter(|model| model_is_camera_facing_actor(model))
                .map(|_| {
                    camera_facing_entity_orientation(camera.position, draw_pos, camera.left_handed)
                })
                .unwrap_or_else(|| {
                    non_player_entity_orientation(entity.heading, entity.physical_body_basis_q31())
                });
            let entity_anim_vars = entity.presentation_anim_vars(retail_tick);
            let descriptor = cache
                .global_entity_type(entity.entity_type as usize)
                .and_then(|record| record.sub_h_external_frame_descriptor());
            let body_axes_q31 = match entity.physical_body_basis_q31() {
                RetailRuntimeValue::Known(basis) => Some([basis.lateral, basis.up, basis.forward]),
                RetailRuntimeValue::Unresolved => None,
            };
            let origin_raw = world_position_raw(entity.position);
            let shade_shift =
                world_model_shade_shift(entity.position, cache.terrain(), Some(terrain_lights));
            let mut sub_m_product_write = None;
            let sub_m_presentation = if descriptor.is_none() {
                v2k_game::sub_m_external_frame::SubMPresentation::for_entity(
                    entity,
                    draw_pos,
                    &mut sub_m_product_write,
                )
            } else {
                None
            };
            let type_record = cache.global_entity_type(entity.entity_type as usize);
            let emitter_descriptor =
                type_record.and_then(|record| record.projectile_emitter_descriptor());
            let native_context = native_viewport.zip(body_axes_q31).map(|(viewport, axes)| {
                (
                    v2k_game::native_model_frame::NativeModelFrame::from_actor(
                        viewport, origin_raw, axes,
                    ),
                    viewport,
                )
            });
            let mut emitter_boundary = None;
            let mut stamp_error = None;
            let mut stamp_origin = |point| {
                if let Some(runtime) = entity.ordinary_type47_aim_and_fire_runtime.as_mut() {
                    if let Err(reason) = runtime.stamp_draw_emitter_origin(entity_id, point) {
                        stamp_error = Some(reason);
                    }
                }
            };
            let emitter = native_context.as_ref().zip(emitter_descriptor).and_then(|((frame, viewport), descriptor)| {
                Some(v2k_game::actor_emitter_external_frame::ActorEmitterExternalFramePresentation {
                    descriptor, root_model: cache.global_model(model_id)?, root_vars: &entity_anim_vars,
                    frame: frame.clone(), viewport: *viewport, stamp_origin: &mut stamp_origin,
                    last_boundary: &mut emitter_boundary, current_node_owned: true, current_source_points_view: None,
                })
            });
            let mut random_u16 = || world_fx.next_shared_retail_random_u16();
            // Only absent joint/M/G components authenticate the terminal domain.
            // An authored G descriptor alone cannot prove its live callback owner.
            let fallback = type_record.filter(|record| record.subsection("I").is_none()
                && record.subsection("M").is_none() && record.subsection("G").is_none()
                && (record.subsection("E").is_none() || emitter_descriptor.is_some()))
                .and_then(|_| descriptor.as_ref()).map(|h| {
                    v2k_game::actor_external_frame_fallback::ActorExternalFrameFallback {
                        reservations: v2k_game::actor_external_frame_fallback::ActorExternalFrameReservations {
                            sub_h_records: h.records.len() as u16,
                            sub_e_joint_slots: emitter_descriptor.map_or(0, v2k_game::actor_emitter_external_frame::emitter_external_selector_count),
                            sub_m_present: false, live_sub_g_present: false,
                        },
                        own_origin_raw: origin_raw, relation_origin_raw, random_u16: &mut random_u16,
                        last_boundary: None,
                    }
                });
            let mut tree = ModelTreeRenderer::new_world(
                renderer,
                cache,
                face_colors,
                GAMEPLAY_MODEL_SCALE,
                Some(MenuSceneLight::NEUTRAL),
                retail_tick as i32,
            )
            .with_submission_buffer(&mut model_submissions)
            .with_view(camera.into())
            .with_shade_shift(shade_shift);
            if let (Some(descriptor), Some(model), RetailRuntimeValue::Known(Some(runtime))) = (
                descriptor.as_ref(),
                cache.global_model(model_id),
                &mut entity.sub_h_external_frame_runtime,
            ) {
                tree = tree.with_sub_h_presentation(
                    v2k_game::sub_h_external_frame::SubHPresentation {
                        runtime,
                        retail_tick,
                        descriptor,
                        model_records: &model.records,
                        origin_raw,
                        origin_world: draw_pos,
                        body_axes_q31,
                        native_context,
                        fallback,
                        emitter,
                    },
                );
            }
            // These families have no Sub-H/I selectors before Sub-M selector0.
            // The callback returns geometry even when +88 has no live product.
            if let Some(presentation) = sub_m_presentation {
                let presentation = match native_viewport {
                    Some(viewport) => presentation.with_native_viewport(viewport),
                    None => presentation,
                };
                tree = tree.with_sub_m_presentation(presentation);
            }
            tree.draw_linked(model_id, orientation, draw_pos, 8, None, &entity_anim_vars);
            for boundary in tree.native_sub_m_boundaries() {
                eprintln!(
                    "Entity {} model{} Sub-M marker suppressed: {boundary:?}",
                    entity.id, model_id,
                );
            }
            drop(tree);
            drop(stamp_origin);
            drop(random_u16);
            if let Some(boundary) = emitter_boundary {
                report_actor_emitter_boundary(entity_id, model_id, boundary);
            }
            if let Some(reason) = stamp_error {
                eprintln!("Entity {entity_id} draw muzzle stamp stopped: {reason:?}");
            }
            wreck_ids.push(entity_id);
            if let Some(write) = sub_m_product_write {
                em.apply_sub_m_product_marker_write(write);
            }
        }
        let environment = TerrainCollisionContext::from_current_level_cache(cache)
            .map_or(ParticleEnvironment::Dry, ParticleEnvironment::Terrain);
        if let Some(drain) = v2k_game::intro2_projectiles::drain_intro2_projectile_source(
            em,
            world_fx,
            cache,
            environment,
            retail_tick,
            entity_id,
        ) {
            match drain {
                Ok(consumed) if consumed > 0 => {
                    log!("Playing entity {entity_id}: drained {consumed} shot(s)")
                }
                Err(reason) => {
                    eprintln!("Playing entity {entity_id} shot drain stopped: {reason:?}")
                }
                _ => {}
            }
        }
        //11720 reloads the current body's successor after11400, allowing
        // bodies appended by the player's transient drain to draw at age zero.
        actor_cursor = em
            .retail_live_order_ids()
            .skip_while(|id| *id != entity_id)
            .nth(1);
    }
    if !pending_entity_weapon_fire.is_empty() {
        publish_entity_weapon_bodies(
            EntityWeaponPresentationFrame {
                entities: em,
                cache,
                world_fx,
                actor_tasks: specialized_actor_tasks,
                retail_tick,
                error_reported: entity_weapon_error_reported,
            },
            pending_entity_weapon_fire,
            &EntityWeaponDrawOrigins::default(),
        );
    }

    // 53760 -> 53A60 -> 536A0: callbacks and each FIFO precede the
    // single destructive particle draw gate, including this frame's births.
    let particle_presentation = match particles {
        GameplayWorldParticles::Live => prepare_particle_presentation(
            renderer,
            camera,
            world_fx,
            GAMEPLAY_PARTICLE_FAR_DEPTH_RAW,
        ),
        GameplayWorldParticles::Frozen(frame) => {
            let (width, height) = renderer.viewport_size();
            frame.reprojected([width, height], |particle| {
                project_particle_center(
                    camera,
                    [width, height],
                    camera_relative(camera, particle.position),
                )
            })
        }
    };
    if let Some(terrain) = cache.terrain() {
        accumulate_particle_terrain_lights(terrain_lights, terrain, &particle_presentation);
    }
    if let Some(terrain) = cache.terrain() {
        let colors = cache.color_palettes().map(|c| c.as_slice()).unwrap_or(&[]);
        renderer.draw_terrain(
            terrain,
            colors,
            terrain_frames,
            Some(terrain_lights),
            elapsed_micros,
        );
    }

    draw_static_terrain_objects(
        renderer,
        cache,
        face_colors,
        camera,
        camera_mode,
        terrain_frames,
        Some(terrain_lights),
        retail_tick,
        world_fx,
    );
    model_submissions.flush(
        renderer,
        cache
            .terrain()
            .map(|terrain| v2k_render::WorldSurfaceProjection::new(terrain, retail_tick as i32)),
    );
    for entity_id in wreck_ids {
        if let Some(entity) = em.iter_all().find(|entity| entity.id == entity_id) {
            draw_hive_wreck_rings(renderer, cache, face_colors, camera, entity, retail_tick);
        }
    }

    draw_exploding_rings(
        renderer,
        cache,
        face_colors,
        camera,
        Some(terrain_lights),
        world_fx,
        retail_tick,
    );

    draw_targetter_overlay(
        renderer,
        cache,
        face_colors,
        camera,
        em,
        player_craft,
        targetter.runtime,
        targetter.enabled,
        elapsed_micros,
        retail_tick,
    );

    // Sea: wave-displaced, per-level height from the Section 10
    // header, marching-squares shoreline tiles blended over the
    // scene (engine pass 2, sort key +0x180 — after the opaque
    // terrain and models so submerged geometry blends underneath).
    // Only the levels whose sea level clears the water gate draw it.
    if let Some(terrain) = cache.terrain() {
        if terrain.water_enabled() {
            renderer.draw_water(
                terrain,
                terrain.sea_level_world_y(),
                WATER_COLOR,
                retail_tick as i32,
                water_frames,
            );
        }
    }

    let Some(descriptor) = cache.level_desc() else {
        eprintln!("Gameplay particle draw missing authored Section13 fog planes");
        world_fx.note_spray_presented(0);
        return GameplayWorldPresentation {
            shield: presented_shield,
            particles: particle_presentation,
        };
    };
    let particle_fog = ParticleSpriteFogPlanes::gameplay(
        descriptor.terrain_draw_depth,
        descriptor.fog_width_cells(),
    );
    let spray_presented = draw_world_fx(
        renderer,
        cache,
        face_colors,
        camera,
        &particle_presentation,
        particle_fog,
    );
    world_fx.note_spray_presented(spray_presented);
    GameplayWorldPresentation {
        shield: presented_shield,
        particles: particle_presentation,
    }
}

/// Render the authored Intro2 world without exposing gameplay controls.
///
/// Section 2 supplies actor activation, camera subjects, and captions. Live
/// actor owners submit their committed pose, model, body basis, and animation
/// bank. Remaining unimplemented families retain explicit presentation
/// fallbacks; they must not overwrite state belonging to a live owner.
#[derive(Clone, Copy)]
enum OpeningCinematicPresentation<'a> {
    Story,
    KlausCover(KlausHandoffFrame<'a>),
}

struct OpeningWorldEffects<'a> {
    terrain_lights: &'a mut v2k_render::TerrainLightWindow,
    world_fx: &'a mut WorldFx,
    explosion_lights: &'a [TerrainExplosionLight],
    full_frame_sprite: Option<&'a PreparedFullFrameSpriteCommand>,
    native_viewport: Option<v2k_game::native_model_frame::NativeWorldViewport>,
    world_projection: Option<WorldProjection>,
}

#[allow(clippy::too_many_arguments)]
fn render_opening_cinematic(
    renderer: &mut dyn v2k_render::Renderer,
    cache: &v2k_game::resource_cache::ResourceCache,
    face_colors: &v2k_game::model_color::ModelMaterialCache,
    entities: &mut v2k_game::entity::EntityManager,
    camera: &Camera,
    elapsed: f32,
    fonts: Option<&v2k_game::menu_text::MenuFonts>,
    sky: Option<&v2k_game::sky::SkyBackground>,
    terrain_frames: Option<&v2k_render::TerrainFrames>,
    water_frames: Option<&v2k_render::WaterFrames>,
    effects: OpeningWorldEffects<'_>,
    menu_resources: &v2k_game::menu::MenuResources,
    billboard_sprite_id: u16,
    retail_tick: u32,
    elapsed_micros: u32,
    presentation: OpeningCinematicPresentation<'_>,
) {
    let OpeningWorldEffects {
        terrain_lights,
        world_fx,
        explosion_lights,
        full_frame_sprite,
        native_viewport,
        world_projection,
    } = effects;
    if matches!(intro2_backdrop(retail_tick), Intro2Backdrop::BlackCard) {
        renderer.begin_scene(RenderScene::World);
        renderer.clear(0.0, 0.0, 0.0);
        finish_opening_cinematic_frame(
            renderer,
            fonts,
            menu_resources,
            billboard_sprite_id,
            retail_tick,
            presentation,
            full_frame_sprite,
        );
        return;
    }

    renderer.begin_scene(RenderScene::World);
    let projection_effect = world_projection_effect(
        camera.position[1],
        cache.terrain().map(|terrain| terrain.sea_level_raw()),
        retail_tick,
    );
    renderer.set_projection_effect(projection_effect);
    let clear_color = sky.map(|sky| sky.color).unwrap_or([0.06, 0.08, 0.12]);
    renderer.clear(clear_color[0], clear_color[1], clear_color[2]);
    renderer.set_camera(camera);
    renderer.set_scene_projection_authority(
        v2k_game::world_projection::scene_projection_authority(
            world_projection,
            native_viewport.is_some(),
            true,
            renderer.viewport_size(),
            projection_effect,
        ),
    );
    renderer.set_native_world_viewport(native_viewport.map(|viewport| viewport.words()));
    draw_authored_sky_model(renderer, cache, None, face_colors, camera, retail_tick);

    // 530D0/53570 and pre-actor light writers precede 53760. Actor
    // commands capture this field; 53A60 adds particle light before terrain.
    begin_particle_terrain_lights(terrain_lights, camera);
    for light in explosion_lights {
        terrain_lights.add_explosion(
            i32::from(light.x_raw),
            i32::from(light.z_raw),
            i32::from(light.radius_raw),
        );
    }

    let actor_ids: Vec<_> = entities.iter_all().map(|entity| entity.id).collect();
    let mut model_submissions = v2k_game::model_tree::ModelTreeSubmissionBuffer::default();
    let mut wreck_ids = Vec::new();
    for entity_id in actor_ids {
        let relation_origin_raw = entities
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .map(|entity| entity.collision.recent_relation_id_at_0x60)
            .map_or(RetailRuntimeValue::Unresolved, |relation| match relation {
                RetailRuntimeValue::Known(id) => RetailRuntimeValue::Known(id.and_then(|id| {
                    entities
                        .iter_all()
                        .find(|entity| entity.id == id)
                        .map(|entity| entity.position_raw())
                })),
                RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
            });
        'draw_actor: {
            let Some(entity) = entities.entity_mut(entity_id) else {
                break 'draw_actor;
            };
            if !world_body_submission_eligible(entity)
                || !entity.active
                || entity.kind == v2k_game::entity::EntityKind::Trigger
            {
                break 'draw_actor;
            }

            let Some(entity_index) = entity.authored_spawn_index else {
                break 'draw_actor;
            };
            if !intro_actor_visible(entity_index) {
                break 'draw_actor;
            }
            let intro_anim_vars = entity.presentation_anim_vars(retail_tick);
            let live_actor_pose = intro2_uses_live_actor_pose(entity);
            let (model_id, position, body_orientation) = if live_actor_pose {
                let Some(model_id) = entity.model_index else {
                    break 'draw_actor;
                };
                (
                    model_id,
                    entity.position,
                    non_player_entity_orientation(entity.heading, entity.physical_body_basis_q31()),
                )
            } else {
                let pose = intro_actor_pose(entity_index, entity.position, elapsed);
                let Some(model_id) = entity.model_in_slot(pose.model_slot) else {
                    break 'draw_actor;
                };
                (
                    model_id,
                    pose.position,
                    non_player_model_orientation(
                        entity.heading + intro_actor_heading(entity_index),
                    ),
                )
            };
            let draw_position = camera_relative(camera, position);
            let orientation = cache
                .global_model(model_id)
                .filter(|model| model_is_camera_facing_actor(model))
                .map(|_| {
                    camera_facing_entity_orientation(
                        camera.position,
                        draw_position,
                        camera.left_handed,
                    )
                })
                .unwrap_or(body_orientation);
            let descriptor = cache
                .global_entity_type(entity.entity_type as usize)
                .and_then(|record| record.sub_h_external_frame_descriptor());
            let body_axes_q31 = match entity.physical_body_basis_q31() {
                RetailRuntimeValue::Known(basis) => Some([basis.lateral, basis.up, basis.forward]),
                RetailRuntimeValue::Unresolved => None,
            };
            let mut sub_m_product_write = None;
            let sub_m_presentation = if live_actor_pose && descriptor.is_none() {
                v2k_game::sub_m_external_frame::SubMPresentation::for_entity(
                    entity,
                    draw_position,
                    &mut sub_m_product_write,
                )
            } else {
                None
            };
            let type_record = cache.global_entity_type(entity.entity_type as usize);
            let emitter_descriptor =
                type_record.and_then(|record| record.projectile_emitter_descriptor());
            let native_context = native_viewport.zip(body_axes_q31).map(|(viewport, axes)| {
                (
                    v2k_game::native_model_frame::NativeModelFrame::from_actor(
                        viewport,
                        world_position_raw(position),
                        axes,
                    ),
                    viewport,
                )
            });
            let mut emitter_boundary = None;
            let mut stamp_error = None;
            let mut stamp_origin = |point| {
                if let Some(runtime) = entity.ordinary_type47_aim_and_fire_runtime.as_mut() {
                    if let Err(reason) = runtime.stamp_draw_emitter_origin(entity_id, point) {
                        stamp_error = Some(reason);
                    }
                }
            };
            let emitter = native_context.as_ref().zip(emitter_descriptor).and_then(|((frame, viewport), descriptor)| {
                Some(v2k_game::actor_emitter_external_frame::ActorEmitterExternalFramePresentation {
                    descriptor, root_model: cache.global_model(model_id)?, root_vars: &intro_anim_vars,
                    frame: frame.clone(), viewport: *viewport, stamp_origin: &mut stamp_origin,
                    last_boundary: &mut emitter_boundary, current_node_owned: true, current_source_points_view: None,
                })
            });
            let mut random_u16 = || world_fx.next_shared_retail_random_u16();
            // Only absent joint/M/G components authenticate the terminal domain.
            // An authored G descriptor alone cannot prove its live callback owner.
            let fallback = type_record.filter(|record| record.subsection("I").is_none()
                && record.subsection("M").is_none() && record.subsection("G").is_none()
                && (record.subsection("E").is_none() || emitter_descriptor.is_some()))
                .and_then(|_| descriptor.as_ref()).map(|h| {
                    v2k_game::actor_external_frame_fallback::ActorExternalFrameFallback {
                        reservations: v2k_game::actor_external_frame_fallback::ActorExternalFrameReservations {
                            sub_h_records: h.records.len() as u16,
                            sub_e_joint_slots: emitter_descriptor.map_or(0, v2k_game::actor_emitter_external_frame::emitter_external_selector_count),
                            sub_m_present: false, live_sub_g_present: false,
                        },
                        own_origin_raw: world_position_raw(position), relation_origin_raw, random_u16: &mut random_u16,
                        last_boundary: None,
                    }
                });
            let mut tree = ModelTreeRenderer::new_world(
                renderer,
                cache,
                face_colors,
                GAMEPLAY_MODEL_SCALE,
                Some(MenuSceneLight::NEUTRAL),
                retail_tick as i32,
            )
            .with_submission_buffer(&mut model_submissions)
            .with_view(camera.into())
            .with_shade_shift(world_model_shade_shift(
                position,
                cache.terrain(),
                Some(terrain_lights),
            ));
            // D360 writes persistent caches in the physical actor's coordinate
            // system. A remaining cinematic pose proxy cannot own those writes.
            if let (true, Some(descriptor), Some(model), RetailRuntimeValue::Known(Some(runtime))) = (
                live_actor_pose,
                descriptor.as_ref(),
                cache.global_model(model_id),
                &mut entity.sub_h_external_frame_runtime,
            ) {
                tree = tree.with_sub_h_presentation(
                    v2k_game::sub_h_external_frame::SubHPresentation {
                        runtime,
                        retail_tick,
                        descriptor,
                        model_records: &model.records,
                        origin_raw: world_position_raw(position),
                        origin_world: draw_position,
                        body_axes_q31,
                        native_context,
                        fallback,
                        emitter,
                    },
                );
            }
            if let Some(presentation) = sub_m_presentation {
                let presentation = match native_viewport {
                    Some(viewport) => presentation.with_native_viewport(viewport),
                    None => presentation,
                };
                tree = tree.with_sub_m_presentation(presentation);
            }
            tree.draw_linked(
                model_id,
                orientation,
                draw_position,
                8,
                None,
                &intro_anim_vars,
            );
            for boundary in tree.native_sub_m_boundaries() {
                eprintln!(
                    "Entity {} model{} Sub-M marker suppressed: {boundary:?}",
                    entity.id, model_id,
                );
            }
            drop(tree);
            drop(stamp_origin);
            drop(random_u16);
            if let Some(boundary) = emitter_boundary {
                report_actor_emitter_boundary(entity_id, model_id, boundary);
            }
            if let Some(reason) = stamp_error {
                eprintln!("Entity {entity_id} draw muzzle stamp stopped: {reason:?}");
            }
            wreck_ids.push(entity_id);
            if let Some(write) = sub_m_product_write {
                entities.apply_sub_m_product_marker_write(write);
            }
        }
        let environment = TerrainCollisionContext::from_current_level_cache(cache)
            .map_or(ParticleEnvironment::Dry, ParticleEnvironment::Terrain);
        if let Some(drain) = v2k_game::intro2_projectiles::drain_intro2_projectile_source(
            entities,
            world_fx,
            cache,
            environment,
            retail_tick,
            entity_id,
        ) {
            match drain {
                Ok(consumed) if consumed > 0 => {
                    log!("Intro2 entity {entity_id}: drained {consumed} shot(s)")
                }
                Err(reason) => {
                    eprintln!("Intro2 entity {entity_id} shot drain stopped: {reason:?}")
                }
                _ => {}
            }
        }
    }

    // 53760 -> 53A60 -> 536A0: callbacks and each FIFO precede the
    // single destructive particle draw gate, including this frame's births.
    let particle_presentation =
        prepare_particle_presentation(renderer, camera, world_fx, INTRO2_PARTICLE_FAR_DEPTH_RAW);
    if let Some(terrain) = cache.terrain() {
        accumulate_particle_terrain_lights(terrain_lights, terrain, &particle_presentation);
    }
    if let Some(terrain) = cache.terrain() {
        let colors = cache
            .color_palettes()
            .map(|palettes| palettes.as_slice())
            .unwrap_or(&[]);
        renderer.draw_terrain(
            terrain,
            colors,
            terrain_frames,
            Some(terrain_lights),
            elapsed_micros,
        );
    }

    draw_static_terrain_objects(
        renderer,
        cache,
        face_colors,
        camera,
        GameplayWorldCameraMode::RetailChase,
        terrain_frames,
        Some(terrain_lights),
        retail_tick,
        world_fx,
    );
    model_submissions.flush(
        renderer,
        cache
            .terrain()
            .map(|terrain| v2k_render::WorldSurfaceProjection::new(terrain, retail_tick as i32)),
    );
    for entity_id in wreck_ids {
        if let Some(entity) = entities.iter_all().find(|entity| entity.id == entity_id) {
            draw_hive_wreck_rings(renderer, cache, face_colors, camera, entity, retail_tick);
        }
    }

    draw_exploding_rings(
        renderer,
        cache,
        face_colors,
        camera,
        Some(terrain_lights),
        world_fx,
        retail_tick,
    );

    if let Some(terrain) = cache.terrain() {
        if terrain.water_enabled() {
            renderer.draw_water(
                terrain,
                terrain.sea_level_world_y(),
                WATER_COLOR,
                retail_tick as i32,
                water_frames,
            );
        }
    }

    let _ = draw_world_fx(
        renderer,
        cache,
        face_colors,
        camera,
        &particle_presentation,
        ParticleSpriteFogPlanes::INTRO2,
    );
    finish_opening_cinematic_frame(
        renderer,
        fonts,
        menu_resources,
        billboard_sprite_id,
        retail_tick,
        presentation,
        full_frame_sprite,
    );
}

/// Complete an Intro2 frame with the overlays shared by world shots and the
/// final black card.
fn finish_opening_cinematic_frame(
    renderer: &mut dyn v2k_render::Renderer,
    fonts: Option<&v2k_game::menu_text::MenuFonts>,
    menu_resources: &v2k_game::menu::MenuResources,
    billboard_sprite_id: u16,
    retail_tick: u32,
    presentation: OpeningCinematicPresentation<'_>,
    full_frame_sprite: Option<&PreparedFullFrameSpriteCommand>,
) {
    // Klaus remains geometry in the first sorted queue: his positive outer
    // key drains before 53410's key zero. Captions/emblem are later overlays.
    if let OpeningCinematicPresentation::KlausCover(frame) = presentation {
        draw_klaus_handoff_overlay(renderer, frame);
    }
    if let Some(command) = full_frame_sprite {
        draw_prepared_full_frame_sprite_command(renderer, command);
    }
    if let Some(fonts) = fonts {
        for caption in active_captions(retail_tick) {
            let (visible, revealing) = caption.revealed_text(retail_tick);
            let mut text = visible.to_owned();
            if revealing {
                text.push('_');
            }
            draw_story_caption(renderer, fonts, &text);
        }
    }
    draw_intro_billboard(renderer, menu_resources, billboard_sprite_id);
    renderer.present();
}

/// Advance Intro2's hidden class-1 camera independently of its world draw.
/// The world clock runs throughout the Klaus wipe, so coupling this
/// controller to [`render_opening_cinematic`] would restart/snap the first
/// visible shot after the cover disappears.
fn advance_opening_camera_eye(
    cache: &v2k_game::resource_cache::ResourceCache,
    camera: &mut Camera,
    intro_camera: &mut IntroCameraController,
    elapsed_micros: u32,
    active_camera_setting: u8,
) {
    configure_opening_world_camera(camera);
    intro_camera
        .advance_eye(IntroCameraEyeUpdate {
            elapsed_micros,
            // The complete Intro2 entry trace proves caller parameter 4 is
            // zero on every call. FUN_0040ED10 separately reads the persisted
            // Display -> Active Camera setting inside its distance formula.
            active_camera: active_camera_setting,
            terrain: gameplay_chase_terrain(cache),
        })
        .apply_to(camera);
}

/// Intro2 is rendered through the same left-handed world presentation as
/// live gameplay, including free-fly. Menu cameras stay right-handed.
fn configure_opening_world_camera(camera: &mut Camera) {
    camera.left_handed = true;
}

/// Run the destructive retail particle gate once for the presented world
/// frame. The resulting owned list is shared by terrain lighting and the later
/// sprite pass so neither phase can observe a different particle lifetime.
fn prepare_particle_presentation(
    renderer: &dyn v2k_render::Renderer,
    camera: &Camera,
    world_fx: &mut WorldFx,
    far_depth_raw: i32,
) -> ParticlePresentationFrame {
    let (width, height) = renderer.viewport_size();
    let viewport = [width, height];
    world_fx.prepare_presentation(viewport, far_depth_raw, |particle| {
        project_particle_center(camera, viewport, camera_relative(camera, particle.position))
    })
}

/// Clear/position 530D0/53570 before actor model callbacks and light reads.
fn begin_particle_terrain_lights(
    terrain_lights: &mut v2k_render::TerrainLightWindow,
    camera: &Camera,
) {
    terrain_lights.begin_world_frame(
        (camera.position[0] * 256.0).round() as i32,
        (camera.position[2] * 256.0).round() as i32,
    );
}

/// 53A60/D410 adds to the same window after actors and before terrain/static.
fn accumulate_particle_terrain_lights(
    terrain_lights: &mut v2k_render::TerrainLightWindow,
    terrain: &v2k_formats::terrain::TerrainGrid,
    presentation: &ParticlePresentationFrame,
) {
    for presented in presentation.particles() {
        let Some(light) = presented.particle.terrain_light_emission(terrain) else {
            continue;
        };
        terrain_lights.add_radial(
            i32::from(light.x_raw),
            i32::from(light.z_raw),
            light.radius_raw,
        );
    }
}

#[cfg(test)]
fn rebuild_particle_terrain_lights(
    terrain_lights: &mut v2k_render::TerrainLightWindow,
    camera: &Camera,
    terrain: &v2k_formats::terrain::TerrainGrid,
    presentation: &ParticlePresentationFrame,
) {
    begin_particle_terrain_lights(terrain_lights, camera);
    accumulate_particle_terrain_lights(terrain_lights, terrain, presentation);
}

/// Submit the acquired Targetter's ring and crosshair Section-8 models.
/// Configuration suppresses only the selected-entity ring; crosshairs and the
/// controller continue regardless, matching `FUN_0044DB10`'s internal branch.
#[allow(clippy::too_many_arguments)]
fn draw_targetter_overlay(
    renderer: &mut dyn v2k_render::Renderer,
    cache: &v2k_game::resource_cache::ResourceCache,
    colors: &v2k_game::model_color::ModelMaterialCache,
    camera: &Camera,
    entities: &EntityManager,
    player_craft: &PlayerCraft,
    targetter: &mut TargetterRuntime,
    show_entity_ring: bool,
    elapsed_micros: u32,
    retail_tick: u32,
) {
    if !renderer.supports_models() {
        return;
    }
    let overlay = targetter.overlay(show_entity_ring);

    if let Some(ring) = overlay.ring {
        let target = entities.iter().find(|entity| entity.id == ring.target_id);
        if let Some(target) = target.filter(|entity| {
            entity.active
                && entity.collision.state_flags_at_0x08.masked(0x4000)
                    == RetailRuntimeValue::Known(0)
        }) {
            let active_model = match target.collision.active_model_slot() {
                RetailRuntimeValue::Known(slot) => target
                    .model_in_slot(slot)
                    .and_then(|model_id| cache.global_model(model_id)),
                RetailRuntimeValue::Unresolved => None,
            };
            if let (Some(active_model), RetailRuntimeValue::Known(base_orientation)) =
                (active_model, entity_pair_to_world(target))
            {
                targetter.advance_ring_spin(elapsed_micros);
                let spin =
                    f32::from(targetter.ring_spin_raw() as u16) * std::f32::consts::TAU / 65_536.0;
                let orientation = mat3_mul(
                    base_orientation,
                    orientation_from_ypr(spin, 0.0, spin * 0.5),
                );
                let mut vars = AnimVars::default();
                vars.dynamic[0] = (retail_tick & 0xffff) as i32;
                let scale_raw = targetter_model_scale_raw(active_model.collision_radius_raw);
                vars.dynamic[1] = i32::from(scale_raw);
                vars.dynamic[2] = i32::from(scale_raw);
                ModelTreeRenderer::new_world(
                    renderer,
                    cache,
                    colors,
                    GAMEPLAY_MODEL_SCALE,
                    Some(MenuSceneLight::NEUTRAL),
                    retail_tick as i32,
                )
                .with_view(camera.into())
                .draw_linked(
                    ring.model_id,
                    orientation,
                    camera_relative(camera, target.position),
                    8,
                    None,
                    &vars,
                );
            }
        }
    }

    let Some(crosshair) = overlay.crosshair else {
        return;
    };
    let Some(player) = entities.player() else {
        return;
    };
    let (orientation, overlay_kind) = match crosshair.kind {
        TargetterCrosshairKind::Terrain => {
            let Some(terrain) = cache.terrain() else {
                return;
            };
            (
                targetter_terrain_orientation(
                    terrain,
                    crosshair.position_raw,
                    player_model_orientation(player_craft, player),
                ),
                ModelOverlayKind::TerrainSurface,
            )
        }
        TargetterCrosshairKind::ExactEntity | TargetterCrosshairKind::NearEntity => (
            player_model_orientation(player_craft, player),
            ModelOverlayKind::None,
        ),
    };
    let mut vars = AnimVars::default();
    vars.dynamic[0] = (retail_tick & 0xffff) as i32;
    let position = crosshair
        .position_raw
        .map(|component| f32::from(component) / 256.0);
    ModelTreeRenderer::new_world(
        renderer,
        cache,
        colors,
        GAMEPLAY_MODEL_SCALE,
        Some(MenuSceneLight::NEUTRAL),
        retail_tick as i32,
    )
    .with_view(camera.into())
    .with_overlay(overlay_kind)
    .draw_linked(
        crosshair.model_id,
        orientation,
        camera_relative(camera, position),
        8,
        None,
        &vars,
    );
}

/// Port of `FUN_0044DF00`'s terrain-facing basis policy. The helper takes the
/// owner's forward column, samples one signed Section-10 height cell forward
/// and one to its right, normalizes both tangents, then derives the third axis.
fn targetter_terrain_orientation(
    terrain: &v2k_formats::terrain::TerrainGrid,
    point_raw: [i16; 3],
    owner_orientation: [[f32; 3]; 3],
) -> [[f32; 3]; 3] {
    let forward_x = owner_orientation[0][2];
    let forward_z = owner_orientation[2][2];
    let length = (forward_x * forward_x + forward_z * forward_z).sqrt();
    if length <= f32::EPSILON {
        return owner_orientation;
    }
    let forward_x_raw = (forward_x * 256.0 / length).round() as i16;
    let forward_z_raw = (forward_z * 256.0 / length).round() as i16;
    let base_height = targetter_cell_height_raw(terrain, point_raw[0], point_raw[2]);
    let forward_height = targetter_cell_height_raw(
        terrain,
        point_raw[0].wrapping_add(forward_x_raw),
        point_raw[2].wrapping_add(forward_z_raw),
    );
    let right_height = targetter_cell_height_raw(
        terrain,
        point_raw[0].wrapping_add(forward_z_raw),
        point_raw[2].wrapping_sub(forward_x_raw),
    );
    let right = normalize3([
        f32::from(forward_z_raw),
        f32::from(right_height.wrapping_sub(base_height)),
        -f32::from(forward_x_raw),
    ]);
    let forward = normalize3([
        f32::from(forward_x_raw),
        f32::from(forward_height.wrapping_sub(base_height)),
        f32::from(forward_z_raw),
    ]);
    let vertical = [
        forward[2] * right[1] - forward[1] * right[2],
        forward[0] * right[2] - forward[2] * right[0],
        forward[1] * right[0] - forward[0] * right[1],
    ];
    [
        [right[0], forward[0], vertical[0]],
        [right[1], forward[1], vertical[1]],
        [right[2], forward[2], vertical[2]],
    ]
}

fn targetter_cell_height_raw(
    terrain: &v2k_formats::terrain::TerrainGrid,
    x_raw: i16,
    z_raw: i16,
) -> i16 {
    terrain
        .cell(
            usize::from((x_raw as u16) >> 8),
            usize::from((z_raw as u16) >> 8),
        )
        .map(|cell| i16::from(cell.height as i8) << 5)
        .unwrap_or(0)
}

fn normalize3(vector: [f32; 3]) -> [f32; 3] {
    let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if length <= f32::EPSILON {
        [0.0; 3]
    } else {
        vector.map(|component| component / length)
    }
}

fn particle_sprite_sort_key_raw(depth_raw: i32, source_class: u8) -> i32 {
    depth_raw.wrapping_add(
        particle_descriptor(source_class)
            .map(|descriptor| i32::from(descriptor.sort_bias_raw()))
            .unwrap_or(0),
    )
}

// Report each copied-record pointer-phase boundary once per process.
static PARTICLE_STACK_PHASE_REPORTED: [std::sync::atomic::AtomicU64; 4] =
    [const { std::sync::atomic::AtomicU64::new(0) }; 4];

fn prepared_particle_draw_scale_raw(presented: &v2k_game::world_fx::PreparedParticle) -> i32 {
    match presented.native_effective_draw_scale_raw() {
        Ok(scale) => scale,
        Err(boundary) => {
            let class = usize::from(boundary.source_class);
            let word = &PARTICLE_STACK_PHASE_REPORTED[class / 64];
            let bit = 1_u64 << (class % 64);
            if word.fetch_or(bit, std::sync::atomic::Ordering::Relaxed) & bit == 0 {
                eprintln!(
                    "Particle class{} draw0x{:08X}: copied-record size jitter needs an owned retail stack address; rendering the unadjusted source scale (divisor{}). Retail trail width remains unresolved.",
                    boundary.source_class, boundary.callback_va, boundary.divisor_raw,
                );
            }
            boundary.sample_scale_raw
        }
    }
}

/// Convert and submit the shared particle pool through the renderer's
/// camera-facing world-sprite path. Section-3 entry flags select the same
/// fixed shade and masked/half-additive/additive material families as retail.
fn draw_world_fx(
    renderer: &mut dyn v2k_render::Renderer,
    cache: &v2k_game::resource_cache::ResourceCache,
    materials: &v2k_game::model_color::ModelMaterialCache,
    camera: &Camera,
    presentation: &ParticlePresentationFrame,
    fog_planes: ParticleSpriteFogPlanes,
) -> u32 {
    let mut sprites = Vec::with_capacity(presentation.particles().len());
    let mut spray_presented = 0u32;
    let (viewport_width, viewport_height) = renderer.viewport_size();
    let viewport = [viewport_width, viewport_height];
    for presented in presentation.particles() {
        let particle = &presented.particle;
        let frame = particle.current_frame();
        let Some((width, height)) = materials.sprite_dimensions(cache, frame.sprite_id) else {
            continue;
        };
        let draw_scale_raw = prepared_particle_draw_scale_raw(presented);
        let size = [
            particle_sprite_world_dimension(width, draw_scale_raw, frame.scale_raw),
            particle_sprite_world_dimension(height, draw_scale_raw, frame.scale_raw),
        ];
        let descriptor = particle_descriptor(particle.source_class);
        let clamp_half_extent = descriptor.is_some_and(|descriptor| descriptor.flags() & 0x08 != 0);
        let half_extent = particle_sprite_screen_half_extent(
            camera,
            viewport,
            size,
            presented.projection.depth_raw,
            clamp_half_extent,
        );
        if !presentation.sprite_visible(presented, half_extent) {
            continue;
        }
        let Some(fog) = fog_planes.at_depth_raw(presented.projection.depth_raw) else {
            continue;
        };
        if particle.source_class == v2k_game::world_fx::PLAYER_WATER_DOWNWASH_PARTICLE_CLASS {
            spray_presented += 1;
        }
        let Some(texture) = materials.sprite_texture(cache, renderer, frame.sprite_id) else {
            continue;
        };
        let flags = materials
            .sprite_render_flags(cache, frame.sprite_id)
            .unwrap_or(0);
        let (blend, color) = particle_sprite_material(flags);
        let position = camera_relative(camera, particle.presentation_position());
        let native = descriptor.map(|descriptor| {
            let raw = |value: f32| (value * 256.0).round() as i32 as i16;
            let world = particle.presentation_position();
            let position_raw = [raw(world[0]), raw(world[1]), raw(world[2])];
            v2k_render::NativeParticle {
                position_raw,
                scale_raw: draw_scale_raw.wrapping_mul(i32::from(frame.scale_raw as i16)),
                frame_size_raw: frame.scale_raw,
                flags: descriptor.flags(),
                sort_bias_raw: descriptor.sort_bias_raw(),
                fog_near_raw: fog_planes.near_raw,
                fog_far_raw: fog_planes.far_raw,
                shadow: (descriptor.shadow_size_raw() != 0).then(|| {
                    v2k_render::NativeParticleShadow {
                        size: descriptor.shadow_size_raw(),
                        ground_raw: cache.terrain().map_or(0, |terrain| {
                            particle_ground_raw(terrain, position_raw[0], position_raw[2])
                        }),
                        colour: particle_shadow_colour(cache),
                    }
                }),
            }
        });
        sprites.push(WorldSprite {
            texture,
            position,
            size,
            rotation: 0.0,
            color,
            sort_key_raw: particle_sprite_sort_key_raw(
                presented.projection.depth_raw,
                particle.source_class,
            ),
            blend,
            flat_shade_row: v2k_game::model_color::sprite_flat_shade_row(flags),
            fog,
            native,
        });
    }
    renderer.draw_world_sprites(&sprites);
    spray_presented
}

/// `FUN_0043DB60` for a drawn particle: the terrain height under it,
/// bilinear between the four surrounding cells.
fn particle_ground_raw(terrain: &v2k_formats::terrain::TerrainGrid, x_raw: i16, z_raw: i16) -> i16 {
    let (x, z) = (x_raw as u16, z_raw as u16);
    let (x_cell, z_cell) = (usize::from(x >> 8), usize::from(z >> 8));
    let height = |x: usize, z: usize| {
        terrain
            .cell(x & 0xFF, z & 0xFF)
            .map_or(0, |cell| i32::from(cell.height as i8) * 0x20)
    };
    let (fx, fz) = (i32::from(x & 0xFF), i32::from(z & 0xFF));
    let near = height(x_cell, z_cell);
    let near = (((height(x_cell + 1, z_cell) - near) * fx) >> 8) + near;
    let far = height(x_cell, z_cell + 1);
    let far = (((height(x_cell + 1, z_cell + 1) - far) * fx) >> 8) + far;
    (near + (((far - near) * fz) >> 8)) as i16
}

/// System-2 palette entry 32, the particle shadow colour, as a display word.
fn particle_shadow_colour(cache: &v2k_game::resource_cache::ResourceCache) -> u32 {
    cache
        .master_color_palette()
        .and_then(|palette| palette.get(32))
        .map_or(0, |entry| {
            u32::from(((entry.rgb555 & 0x7FE0) << 1) | (entry.rgb555 & 0x1F))
        })
}

/// `LAB_0041CB10` continues after the admitted hive's ordinary body draw.
/// Its ring model uses the global palette and its own callback parameters;
/// the body's Sub-K words, terrain shade shift and attachment do not carry over.
fn draw_hive_wreck_rings(
    renderer: &mut dyn v2k_render::Renderer,
    cache: &v2k_game::resource_cache::ResourceCache,
    colors: &v2k_game::model_color::ModelMaterialCache,
    camera: &Camera,
    entity: &v2k_game::entity::Entity,
    retail_tick: u32,
) {
    use v2k_game::hive_wreck_presentation::{
        HiveWreckPresentationRequest, HIVE_WRECK_RING_MODEL_ID,
    };
    let (Some(request), Some(terrain)) = (
        HiveWreckPresentationRequest::from_entity(entity),
        cache.terrain(),
    ) else {
        return;
    };
    for ring in request.ring_submissions(terrain) {
        let camera_raw = camera.position.map(|world| (world * 256.0) as i32);
        let draw_position = ring
            .unwrapped_position_raw(camera_raw)
            .map(|raw| raw as f32 / 256.0);
        ModelTreeRenderer::new_world(
            renderer,
            cache,
            colors,
            GAMEPLAY_MODEL_SCALE,
            Some(MenuSceneLight::NEUTRAL),
            retail_tick as i32,
        )
        .with_view(camera.into())
        .draw_linked(
            HIVE_WRECK_RING_MODEL_ID,
            ring.orientation(),
            draw_position,
            8,
            None,
            &ring.anim_vars(retail_tick),
        );
    }
}

/// Draw live type-60 hard-entry rings in the ordinary dynamic-model pass,
/// before the translucent water surface. Their authored segment geometry reads
/// component control output one through `AnimVars::dynamic[1]`.
fn draw_exploding_rings(
    renderer: &mut dyn v2k_render::Renderer,
    cache: &v2k_game::resource_cache::ResourceCache,
    colors: &v2k_game::model_color::ModelMaterialCache,
    camera: &Camera,
    terrain_lights: Option<&v2k_render::TerrainLightWindow>,
    world_fx: &WorldFx,
    retail_tick: u32,
) {
    let orientation = non_player_model_orientation(0.0);
    for ring in world_fx.exploding_rings() {
        if cache.global_model(ring.model_id).is_none() {
            continue;
        }
        let draw_position = camera_relative(camera, ring.position);
        ModelTreeRenderer::new_world(
            renderer,
            cache,
            colors,
            GAMEPLAY_MODEL_SCALE,
            Some(MenuSceneLight::NEUTRAL),
            retail_tick as i32,
        )
        .with_view(camera.into())
        .with_shade_shift(world_model_shade_shift(
            ring.position,
            cache.terrain(),
            terrain_lights,
        ))
        .draw_linked(
            ring.model_id,
            orientation,
            draw_position,
            8,
            None,
            &ring.anim_vars(),
        );
    }
}

/// Retail Section-3 fixed-billboard material flags. The meteor scatter family
/// is masked (`0x05`), its plume is additive (`0x15`), and its trails use
/// source-plus-destination-half blending (`0x0d`).
fn particle_sprite_material(flags: u8) -> (WorldSpriteBlend, [f32; 4]) {
    let blend = v2k_game::model_color::sprite_blend(flags);
    let opacity = if blend == WorldSpriteBlend::HalfAdditive {
        HALF_ADDITIVE_ALPHA
    } else {
        1.0
    };
    (blend, [1.0, 1.0, 1.0, opacity])
}

/// Fixed-sprite draw planes in retail 8.8 view-depth units. These are distinct
/// from the destructive presentation gate: Intro2's recorded active context
/// installs 12..24 cells, while gameplay uses the loaded Section13 far/width.
#[derive(Clone, Copy)]
struct ParticleSpriteFogPlanes {
    near_raw: i32,
    far_raw: i32,
}

impl ParticleSpriteFogPlanes {
    const INTRO2: Self = Self {
        near_raw: 12 * 256,
        far_raw: 24 * 256,
    };

    fn gameplay(far_cells: u32, width_cells: u32) -> Self {
        let planes =
            v2k_formats::levels::LevelFogPlanes::from_authored_cells(far_cells, width_cells);
        Self {
            near_raw: planes.near_raw,
            far_raw: planes.far_raw,
        }
    }

    /// FUN_0043D410 selects the near constructor strictly before the near
    /// plane. The far constructor consumes the center byte produced by
    /// FUN_00470A10's truncated reciprocal, including at boundary equality.
    fn at_depth_raw(self, depth_raw: i32) -> Option<SpriteFog> {
        // This draw-only far test follows terrain-light publication. It must
        // neither delete the record nor suppress its earlier light emission.
        if depth_raw >= self.far_raw {
            return None;
        }
        if depth_raw < self.near_raw {
            return Some(SpriteFog::Near);
        }
        let reciprocal = 0x0100_0000_i64 / (i64::from(self.far_raw) - i64::from(self.near_raw));
        let fade_byte = (((i64::from(depth_raw) - i64::from(self.near_raw)) * reciprocal) >> 16)
            .clamp(0, i64::from(u8::MAX)) as u8;
        Some(SpriteFog::Far { fade_byte })
    }
}

/// Exact `FUN_0043D410` projected-size conversion after cancelling its camera
/// scale and 8.8 view depth against the port's ordinary perspective matrix.
fn particle_sprite_world_dimension(
    sprite_dimension: u16,
    draw_scale_raw: i32,
    frame_scale_raw: u16,
) -> f32 {
    let scaled =
        (i64::from(sprite_dimension) * i64::from(draw_scale_raw) * i64::from(frame_scale_raw)) >> 8;
    scaled as f32 / 65_536.0
}

/// Convert the submitted world-space billboard size back to the integer
/// half-extents consumed by `FUN_0043D410`'s expanded screen-overlap test.
/// The raw 8.8 center depth deliberately remains the denominator: retail
/// performs this calculation after its integer center projector, not from the
/// later GL quad corners.
fn particle_sprite_screen_half_extent(
    camera: &Camera,
    viewport: [u32; 2],
    size: [f32; 2],
    depth_raw: i32,
    clamp_min_one: bool,
) -> [i32; 2] {
    let depth_raw = depth_raw.max(1) as f32;
    let focal_y = viewport[1] as f32 * 0.5 / (camera.fov * 0.5).tan();
    let focal_x = viewport[0] as f32 * 0.5 / (camera.fov * 0.5).tan() / camera.aspect;
    let mut half_extent = [
        (size[0] * focal_x * 128.0 / depth_raw).trunc() as i32,
        (size[1] * focal_y * 128.0 / depth_raw).trunc() as i32,
    ];
    if clamp_min_one {
        half_extent[0] = half_extent[0].max(1);
        half_extent[1] = half_extent[1].max(1);
    }
    half_extent
}

/// Inputs at 453AA0/4C970 after world presentation. Camera still uses its
/// explicit world-to-GL adapter; audio restores forward-positive view Z.
struct WorldAudioFrame<'a> {
    cache: &'a v2k_game::resource_cache::ResourceCache,
    entities: &'a mut EntityManager,
    camera: &'a Camera,
    elapsed_micros: u32,
}

/// Map454EE0 and pause4513E0 enter44F3E0's mask8. Stop retained physical
/// loops while preserving their logical owners; disposable one-shots already
/// playing and independent UI voices are not handles owned by44CE70.
fn suspend_world_audio(
    world_fx: &mut WorldFx,
    sound_manager: &mut Option<SoundManager>,
    entity_audio: &mut EntityPositionalAudio,
    player_audio: &mut PlayerFanAudio,
) {
    entity_audio.suspend_physical_voices(sound_manager.as_mut());
    player_audio.suspend_physical_voices(sound_manager.as_mut());
    world_fx.garbage_collect_disposable_positional_sounds();
    // Modal branches skip world mixing/RNG. The next gameplay update admits
    // fresh physical voices, matching4556A0/4512E0 clearing the mask.
}

fn world_sound_listener(camera: &Camera) -> PositionalSoundListener {
    let view = camera.view_matrix();
    let q31 = |value: f32| {
        (f64::from(value) * 2_147_483_648.0).clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
    };
    PositionalSoundListener {
        position_raw: world_position_raw(camera.position),
        world_to_view_q31: [
            [view[0], view[4], view[8]].map(q31),
            [view[1], view[5], view[9]].map(q31),
            [-view[2], -view[6], -view[10]].map(q31),
        ],
    }
}

fn play_world_audio(
    frame: WorldAudioFrame<'_>,
    world_fx: &mut WorldFx,
    sound_manager: &mut Option<SoundManager>,
    entity_audio: &mut EntityPositionalAudio,
) {
    let pool = SoundPool::from_tables(&frame.cache.global_sound_tables());
    let listener = world_sound_listener(frame.camera);
    frame
        .entities
        .publish_remaining_constructor_sound_follow_positions();
    // Intro2's attached loops are created during construction, before these
    // later one-shots. Device absence must not suppress audible warble/alias
    // draws. General interleaved world births need one logical creation list.
    if let Err(error) = entity_audio.update(
        frame.entities,
        EntityPositionalAudioFrame {
            sound_manager: sound_manager.as_mut(),
            sound_pool: &pool,
            listener,
            elapsed_micros: frame.elapsed_micros,
        },
        &mut || world_fx.next_shared_retail_random_u16(),
    ) {
        eprintln!("Entity audio: {error:?}");
    }
    for sound in world_fx.take_positional_sounds() {
        let request = PositionalSoundRequest {
            position_raw: world_position_raw(sound.position),
            playback: SoundPlaybackRequest {
                frequency_q16: sound.frequency_q16,
                ..SoundPlaybackRequest::centered(sound.sound_id)
            },
        };
        match resolve_positional_sound(&pool, listener, request, &mut || {
            world_fx.next_shared_retail_random_u16()
        }) {
            Ok(Some(playback)) => {
                if let Some(manager) = sound_manager.as_mut() {
                    manager.play_resolved(playback);
                }
            }
            Ok(None) => {}
            Err(error) => eprintln!("Positional sound {}: {error:?}", sound.sound_id),
        }
    }
}

/// FUN_004537F0 queues the global nine-frame V2000 emblem in every cinematic
/// phase through FUN_0042D030. Unlike the large centered menu billboard, this
/// HUD instance is one quarter of the framebuffer width tall and bottom-left.
fn draw_intro_billboard(
    renderer: &mut dyn v2k_render::Renderer,
    menu_resources: &v2k_game::menu::MenuResources,
    billboard_sprite_id: u16,
) {
    let gid = billboard_sprite_id;
    let idx = (gid as usize).saturating_sub(1294);
    let Some(frame) = menu_resources.flame_frames.get(idx) else {
        return;
    };
    if frame.width == 0 || frame.height == 0 {
        return;
    }
    let (vw, vh) = renderer.viewport_size();
    let h = (vw / 4).max(1);
    let w = (frame.width as f32 * h as f32 / frame.height as f32)
        .round()
        .max(1.0) as u32;
    let scaled = scale_rgba(&frame.rgba, frame.width, frame.height, w, h);
    renderer.draw_additive_sprite_at_depth(
        &scaled,
        w,
        h,
        0,
        vh.saturating_sub(h) as i32,
        0.1,
        0.1,
        1_000.0,
    );
}

fn draw_story_caption(
    renderer: &mut dyn v2k_render::Renderer,
    fonts: &v2k_game::menu_text::MenuFonts,
    text: &str,
) {
    let (vw, vh) = renderer.viewport_size();
    let mapping = UiMapping::new(UiMappingRequest {
        viewport: [vw, vh],
        authored_canvas: [fonts.virtual_w as u32, fonts.virtual_h as u32],
        policy: renderer.ui_submission_policy(),
    });
    let font = &fonts.selected;
    let placement = v2k_game::opening::StoryCaptionPlacement::for_display(
        fonts.virtual_w as i32,
        fonts.virtual_h as i32,
    );
    let max_width = placement.wrap_width as f32;
    let mut lines = Vec::<String>::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && font.measure(&candidate) > max_width {
            lines.push(std::mem::take(&mut line));
            line.push_str(word);
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }

    // The display mode's percentages, independent of the bottom-left V2000
    // emblem.
    let base = placement.baseline as f32;
    for (index, line) in lines.iter().enumerate() {
        draw_menu_text(
            renderer,
            font,
            line,
            [placement.x as f32, base + index as f32 * font.line_step],
            false,
            usize::MAX,
            mapping,
        );
    }
}

fn draw_overlay_51_backdrop(
    renderer: &mut dyn v2k_render::Renderer,
    fonts: &v2k_game::menu_text::MenuFonts,
    backdrop: &Overlay51Backdrop,
    progress: &v2k_game::power_up_contact::PlayerCampaignProgress,
    age_ms: i32,
) {
    let (viewport_width, viewport_height) = renderer.viewport_size();
    let mapping = UiMapping::new(UiMappingRequest {
        viewport: [viewport_width, viewport_height],
        authored_canvas: [fonts.virtual_w as u32, fonts.virtual_h as u32],
        policy: renderer.ui_submission_policy(),
    });
    renderer.begin_scene(RenderScene::Menu);
    renderer.set_sprite_clip(None);
    let marker = backdrop.marker();
    let (_, _, marker_width, marker_height) =
        map_gameplay_hud_rect(0, 0, marker.width, marker.height, mapping);
    let scaled_marker = scale_rgba(
        &marker.rgba,
        marker.width,
        marker.height,
        marker_width,
        marker_height,
    );
    let scaled_box = backdrop.flag_box().map(|box_sprite| {
        (
            box_sprite,
            scale_rgba(
                &box_sprite.rgba,
                box_sprite.width,
                box_sprite.height,
                marker_width,
                marker_height,
            ),
        )
    });
    let extra = backdrop.flag_extra();
    let scaled_extra = extra.map(|extra| {
        let (_, _, extra_width, extra_height) =
            map_gameplay_hud_rect(0, 0, extra.width, extra.height, mapping);
        (
            extra,
            extra_width,
            extra_height,
            scale_rgba(
                &extra.rgba,
                extra.width,
                extra.height,
                extra_width,
                extra_height,
            ),
        )
    });
    for object in backdrop.visible_objects(progress, age_ms) {
        let tile = object.tile;
        let (x, y, width, height) = map_gameplay_hud_rect(
            tile.origin.0,
            tile.origin.1,
            tile.width,
            tile.height,
            mapping,
        );
        let scaled = scale_rgba(&tile.rgba, tile.width, tile.height, width, height);
        renderer.draw_material_sprite(&scaled, width, height, x, y, tile.blend);
        // FUN_00454c70's second FUN_0047a7a0 reuses the tile percent origin.
        let (marker_x, marker_y, _, _) = map_gameplay_hud_rect(
            tile.origin.0,
            tile.origin.1,
            marker.width,
            marker.height,
            mapping,
        );
        renderer.draw_material_sprite(
            &scaled_marker,
            marker_width,
            marker_height,
            marker_x,
            marker_y,
            marker.blend,
        );
        if v2k_game::overlay_51_backdrop::overlay_51_flag_box_visible(object.flags) {
            if let Some((box_sprite, scaled_box)) = scaled_box.as_ref() {
                renderer.draw_material_sprite(
                    scaled_box,
                    marker_width,
                    marker_height,
                    marker_x,
                    marker_y,
                    box_sprite.blend,
                );
            }
        }
        if let Some((extra, extra_width, extra_height, scaled_extra)) = scaled_extra.as_ref() {
            let offsets = [
                (object.flags & 8 != 0).then_some(extra.hidden_offset),
                (object.flags & 0x20 != 0).then_some(extra.time_offset),
            ];
            for offset in offsets.into_iter().flatten() {
                let origin = v2k_game::overlay_51_backdrop::overlay_51_flag_extra_origin(
                    tile.origin,
                    (tile.width, tile.height),
                    offset,
                    (extra.width, extra.height),
                );
                let (extra_x, extra_y, _, _) =
                    map_gameplay_hud_rect(origin.0, origin.1, extra.width, extra.height, mapping);
                renderer.draw_material_sprite(
                    scaled_extra,
                    *extra_width,
                    *extra_height,
                    extra_x,
                    extra_y,
                    extra.blend,
                );
            }
        }
    }
    for link in backdrop.visible_pair_links(progress) {
        let map_corners = |corners: [(i32, i32); 4]| {
            corners.map(|(x, y)| {
                let (mx, my, _, _) = map_gameplay_hud_rect(x, y, 1, 1, mapping);
                (mx, my)
            })
        };
        renderer.draw_material_sprite_quad(
            &link.ribbon.rgba,
            link.ribbon.width,
            link.ribbon.height,
            map_corners(link.geometry.ribbon),
            link.ribbon.blend,
        );
        if let Some(cap) = link.cap {
            for corners in [link.geometry.cap_left, link.geometry.cap_right]
                .into_iter()
                .flatten()
            {
                renderer.draw_material_sprite_quad(
                    &cap.rgba,
                    cap.width,
                    cap.height,
                    map_corners(corners),
                    cap.blend,
                );
            }
        }
    }
}

/// Draw authored messages with an explicit world-overlay or canvas context.
fn draw_gameplay_notifications(
    renderer: &mut dyn v2k_render::Renderer,
    fonts: &v2k_game::menu_text::MenuFonts,
    lines: &[GameplayNotificationLine],
    context: GameplayTextContext,
) {
    gameplay_text_submission::draw_notifications(renderer, fonts, lines, context);
}

/// Draw the right-hand terrain globe in system-level-3's authored pixel
/// space. The RGBA edge coverage deliberately preserves the world already in
/// the framebuffer; unlike the left status orb, retail supplies no black
/// backing plate here.
fn draw_gameplay_radar(renderer: &mut dyn v2k_render::Renderer, frame: &RadarHudFrame) {
    let (viewport_width, viewport_height) = renderer.viewport_size();
    let mapping = UiMapping::new(UiMappingRequest {
        viewport: [viewport_width, viewport_height],
        authored_canvas: [frame.virtual_size[0], frame.virtual_size[1]],
        policy: renderer
            .ui_submission_policy()
            .for_gameplay_hud(UiAnchor::BottomRight),
    });
    let (x, y, width, height) = map_gameplay_hud_rect(
        frame.origin[0],
        frame.origin[1],
        frame.image.width,
        frame.image.height,
        mapping,
    );
    let scaled = scale_rgba(
        &frame.image.rgba,
        frame.image.width,
        frame.image.height,
        width,
        height,
    );
    renderer.begin_scene(RenderScene::Menu);
    renderer.set_sprite_clip(None);
    renderer.draw_sprite(&scaled, width, height, x, y);
}

/// Present the retail M modal without advancing the gameplay clock. The map
/// raster and entity icons share the selected level-3 variant's virtual
/// framebuffer, so the existing 4:3 mapping also preserves the original
/// authored margins at arbitrary output sizes.
struct FullscreenMapDrawRequest<'a> {
    terrain: TerrainRadarView<'a>,
    entities: &'a EntityManager,
    fonts: Option<&'a v2k_game::menu_text::MenuFonts>,
    status: Option<&'a FullscreenMapStatusResources>,
    progress: &'a PlayerCampaignProgress,
}

fn draw_fullscreen_map(
    renderer: &mut dyn v2k_render::Renderer,
    radar: &mut GameplayRadar,
    request: FullscreenMapDrawRequest<'_>,
    next_random: &mut impl FnMut() -> u16,
) {
    let placements =
        radar.fullscreen_icon_placements(request.terrain, request.entities, next_random);
    let rect = radar.map_rect();
    let [virtual_width, virtual_height] = radar.virtual_size();
    let (viewport_width, viewport_height) = renderer.viewport_size();
    let mapping = UiMapping::new(UiMappingRequest {
        viewport: [viewport_width, viewport_height],
        authored_canvas: [virtual_width, virtual_height],
        // The modal, icons and status panel fit together as one centred canvas.
        policy: UiSubmissionPolicy::FitAuthoredCanvas,
    });

    renderer.begin_scene(RenderScene::Menu);
    renderer.clear(0.0, 0.0, 0.0);
    renderer.set_sprite_clip(None);

    if let Some(image) = radar.fullscreen_image(request.terrain) {
        let (x, y, width, height) =
            map_gameplay_hud_rect(rect.x, rect.y, image.width, image.height, mapping);
        let scaled = scale_rgba(&image.rgba, image.width, image.height, width, height);
        renderer.draw_sprite(&scaled, width, height, x, y);
    }

    // FUN_0044C430 clips each native type icon to the authored map rectangle,
    // preventing markers at the wrapped world edge from bleeding into the
    // black status-panel margin.
    renderer.set_sprite_clip(Some(map_gameplay_hud_rect(
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        mapping,
    )));
    for placement in placements {
        let Some(icon) = radar.fullscreen_icon(placement.entity_type) else {
            continue;
        };
        let icon_x = placement.center_x - icon.width as i32 / 2;
        let icon_y = placement.center_y - icon.height as i32 / 2;
        let (x, y, width, height) =
            map_gameplay_hud_rect(icon_x, icon_y, icon.width, icon.height, mapping);
        let scaled = scale_rgba(&icon.rgba, icon.width, icon.height, width, height);
        renderer.draw_sprite(&scaled, width, height, x, y);
    }
    renderer.set_sprite_clip(None);

    // FUN_00453FE0 draws this normal-font status panel after the terrain map.
    // Fail closed if the independently loaded menu font tier does not match
    // the map's level-3 virtual framebuffer; mixing 320- and 640-pixel assets
    // would be less faithful than omitting the panel.
    if let (Some(fonts), Some(status)) = (request.fonts, request.status) {
        let font_virtual_size = [fonts.virtual_w as u32, fonts.virtual_h as u32];
        if font_virtual_size == [virtual_width, virtual_height] {
            for line in status.lines(
                request.progress.extra_lives(),
                request.progress.trophy_count(),
            ) {
                draw_menu_text(
                    renderer,
                    &fonts.normal,
                    &line.text,
                    [line.baseline[0] as f32, line.baseline[1] as f32],
                    false,
                    usize::MAX,
                    mapping,
                );
            }
        }
    }
}

/// Submit the exact currently recovered part of `FUN_004292B0`'s persistent
/// status orb in the selected level-3 variant's authored composition space.
/// The selected weapon and its switching neighbor are drawn through their
/// descriptor-selected model/sprite paths before the later orb sprites mask
/// them into the aperture. The right-side Targetter uses its separate
/// world-model callback rather than this HUD composition pass.
fn draw_gameplay_hud(
    renderer: &mut dyn v2k_render::Renderer,
    cache: &v2k_game::resource_cache::ResourceCache,
    colors: &v2k_game::model_color::ModelMaterialCache,
    resources: &GameplayHudResources,
    frame: &GameplayHudFrame,
    timer_font: Option<&v2k_game::menu_text::MenuFont>,
) {
    let (viewport_width, viewport_height) = renderer.viewport_size();
    let ui_request = UiMappingRequest {
        viewport: [viewport_width, viewport_height],
        authored_canvas: [frame.virtual_width, frame.virtual_height],
        policy: renderer
            .ui_submission_policy()
            .for_gameplay_hud(UiAnchor::BottomLeft),
    };
    let mapping = UiMapping::new(ui_request);

    // Retail constructs a dedicated HUD render context. Reusing the menu
    // baseline suppresses world fog/underwater projection without clearing
    // color, then a depth-only clear isolates the foreground model from world
    // geometry. The following sprites intentionally retain its color pixels.
    renderer.begin_scene(RenderScene::Menu);
    renderer.clear_depth();
    renderer.set_sprite_clip(None);

    // 429509 admits this model and font-0 clock only while the controller
    // state is 1. Their anchors are absolute, independent of the status orb.
    if let Some(trophy) = &frame.time_trophy {
        let trophy_request = UiMappingRequest {
            policy: renderer
                .ui_submission_policy()
                .for_gameplay_hud(UiAnchor::TopLeft),
            ..ui_request
        };
        let animation_vars = trophy.animation_vars();
        draw_gameplay_hud_model(
            renderer,
            cache,
            colors,
            trophy.global_model_id,
            trophy.model_position,
            trophy.orientation(),
            trophy.depth_raw(),
            GameplayHudModelMaterialization::Linked(&animation_vars),
            GameplayHudModelView::time_trophy(trophy_request),
        );
        if let Some(font) = timer_font {
            draw_menu_text(
                renderer,
                font,
                &trophy.text,
                [trophy.text_position.x as f32, trophy.text_position.y as f32],
                false,
                usize::MAX,
                UiMapping::new(trophy_request),
            );
        }
    }

    // FUN_004292B0 draws one sprite-533 marker per currently unlocked cargo
    // slot, then overlays the type-primary model of every occupied attachment.
    // This row precedes both the weapon carousel and the status-orb sprites.
    for cargo in &frame.cargo {
        let marker = resources.sprite(cargo.marker);
        let (x, y, width, height) = map_gameplay_hud_rect(
            cargo.marker_position.x,
            cargo.marker_position.y,
            marker.width,
            marker.height,
            mapping,
        );
        let scaled = scale_rgba(&marker.rgba, marker.width, marker.height, width, height);
        renderer.draw_material_sprite(&scaled, width, height, x, y, marker.blend);

        if let Some(global_model_id) = cargo.global_model_id {
            let Some(model) = cache.global_model(global_model_id) else {
                continue;
            };
            let animation_vars = cargo.animation_vars();
            draw_gameplay_hud_model(
                renderer,
                cache,
                colors,
                global_model_id,
                cargo.model_position,
                cargo.orientation_for_model(model),
                cargo.depth_raw(),
                GameplayHudModelMaterialization::Linked(&animation_vars),
                GameplayHudModelView::selected_canvas(ui_request),
            );
        }
    }

    for weapon in &frame.weapons {
        let model_drawn = weapon.global_model_id.is_some_and(|global_model_id| {
            draw_gameplay_hud_model(
                renderer,
                cache,
                colors,
                global_model_id,
                weapon.model_position,
                weapon.orientation(),
                // FUN_0042A6F0 reads the selected internal display width from
                // g_sprite_meta, not the final desktop viewport. Feeding a native
                // 1920-wide window here pushed the HUD model to depth 20480 and
                // made the weapon almost disappear; variant 1's authored width is
                // 640 and therefore retains the normal depth-3000 branch.
                weapon.depth_raw(frame.virtual_width),
                GameplayHudModelMaterialization::Static,
                GameplayHudModelView::selected_canvas(ui_request),
            )
        });
        if !model_drawn {
            if let Some(fallback_sprite) = weapon.fallback_sprite.and_then(|global_id| {
                v2k_game::gameplay_hud::decode_gameplay_hud_sprite(cache, global_id)
            }) {
                let sprite = fallback_sprite;
                let (position, authored_width, authored_height) = retail_weapon_sprite_rect(
                    weapon.fallback_position,
                    sprite.width,
                    sprite.height,
                    weapon.sprite_size_percent_raw,
                );
                let (x, y, width, height) = map_gameplay_hud_rect(
                    position.x,
                    position.y,
                    authored_width,
                    authored_height,
                    mapping,
                );
                let scaled = scale_rgba(&sprite.rgba, sprite.width, sprite.height, width, height);
                renderer.draw_material_sprite(&scaled, width, height, x, y, sprite.blend);
            }
        }
    }

    for layer in &frame.layers {
        let sprite = resources.sprite(layer.sprite);
        let (x, y, width, height) = map_gameplay_hud_rect(
            layer.position.x,
            layer.position.y,
            sprite.width,
            sprite.height,
            mapping,
        );
        renderer.set_sprite_clip(layer.clip.map(|clip| map_gameplay_hud_clip(clip, mapping)));
        let scaled = scale_rgba(&sprite.rgba, sprite.width, sprite.height, width, height);
        renderer.draw_material_sprite(&scaled, width, height, x, y, sprite.blend);
    }
    renderer.set_sprite_clip(None);
}

/// Section-8 materialization policy for the HUD model callers.
///
/// Retail installs callback `0x42A520` for cargo and the timer trophy. The selected weapon
/// retains its independently recovered static submission until its own
/// callback contract is established.
enum GameplayHudModelMaterialization<'a> {
    Static,
    Linked(&'a AnimVars),
}

/// Model pixel metrics are separate from the selected layout's edge anchors.
enum GameplayHudModelLens {
    SelectedCanvas,
    High640Art,
}

struct GameplayHudModelView {
    ui: UiMappingRequest,
    lens: GameplayHudModelLens,
}

impl GameplayHudModelView {
    fn selected_canvas(ui: UiMappingRequest) -> Self {
        Self {
            ui,
            lens: GameplayHudModelLens::SelectedCanvas,
        }
    }

    fn time_trophy(ui: UiMappingRequest) -> Self {
        let effective_height = (ui.viewport[1] as f32).min(ui.viewport[0] as f32 * 0.75);
        let lens = match ui.policy {
            UiSubmissionPolicy::NativeHud { .. } if effective_height > 768.0 => {
                // The high tiers share trophy geometry and font pixels, but
                // tier 3 returns to the 640-tier (60,30) anchor while its lens
                // grows with 768 canvas rows. Modern readability must enlarge
                // normal high-art pixels, not that already oversized lens.
                GameplayHudModelLens::High640Art
            }
            _ => GameplayHudModelLens::SelectedCanvas,
        };
        Self { ui, lens }
    }

    fn camera(self, point: [f32; 2]) -> v2k_render::Camera {
        let mut camera = menu_camera_for_authored_point(point, self.ui);
        if let GameplayHudModelLens::High640Art = self.lens {
            let (_, high_art_height) =
                v2k_game::gameplay_hud::gameplay_hud_virtual_size(1).unwrap();
            let scale = UiMapping::new(self.ui).scale;
            let mapped_art_height = (high_art_height as f32 * scale).max(1.0);
            let base_fov = menu_camera(1.0).fov;
            camera.fov = 2.0
                * (self.ui.viewport[1].max(1) as f32 / mapped_art_height * (base_fov * 0.5).tan())
                    .atan();
        }
        camera
    }
}

/// Submit one Section-8 object into retail's dedicated HUD model context.
/// Cargo and the selected weapon share this geometry path but retain their
/// independently recovered transforms, depths, and materialization policies
/// at the call site.
fn draw_gameplay_hud_model(
    renderer: &mut dyn v2k_render::Renderer,
    cache: &v2k_game::resource_cache::ResourceCache,
    colors: &v2k_game::model_color::ModelMaterialCache,
    global_model_id: usize,
    model_position: v2k_game::gameplay_hud::GameplayHudPoint,
    orientation: [[f32; 3]; 3],
    depth_raw: i32,
    materialization: GameplayHudModelMaterialization<'_>,
    view: GameplayHudModelView,
) -> bool {
    if !renderer.supports_models() || cache.global_model(global_model_id).is_none() {
        return false;
    }

    let model_depth = depth_raw as f32 / 100.0;
    let mut hud_camera = view.camera([model_position.x as f32, model_position.y as f32]);
    // Retail's high-resolution weapon branch can exceed the menu ring's
    // ordinary far plane. Widen only this foreground camera; cargo's fixed
    // 4000-raw-unit depth already fits but follows the same safe contract.
    hud_camera.far = hud_camera.far.max(model_depth + 100.0);
    renderer.set_camera(&hud_camera);
    let mut tree = ModelTreeRenderer::new(
        renderer,
        cache,
        colors,
        1.0,
        Some(MenuSceneLight::NEUTRAL),
        ViewPinMode::Disabled,
    )
    .with_near_clip(ModelNearClip::RetailFrontend)
    .with_view((&hud_camera).into());
    match materialization {
        GameplayHudModelMaterialization::Static => tree.draw_static(
            global_model_id,
            orientation,
            [0.0, 0.0, -model_depth],
            0,
            None,
        ),
        GameplayHudModelMaterialization::Linked(vars) => tree.draw_linked(
            global_model_id,
            orientation,
            [0.0, 0.0, -model_depth],
            // 42A520 materializes the complete linked hierarchy. Model138
            // is a face-free trophy root whose visible body is child139.
            8,
            None,
            vars,
        ),
    }
    true
}

#[cfg(test)]
mod gameplay_hud_submission_tests;

fn map_gameplay_hud_rect(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    UiMapping {
        scale,
        offset_x,
        offset_y,
    }: UiMapping,
) -> (i32, i32, u32, u32) {
    let x0 = offset_x + (x as f32 * scale) as i32;
    let y0 = offset_y + (y as f32 * scale) as i32;
    let x1 = offset_x + ((x + width as i32) as f32 * scale) as i32;
    let y1 = offset_y + ((y + height as i32) as f32 * scale) as i32;
    (x0, y0, (x1 - x0).max(1) as u32, (y1 - y0).max(1) as u32)
}

fn map_gameplay_hud_clip(clip: GameplayHudClip, mapping: UiMapping) -> (i32, i32, u32, u32) {
    map_gameplay_hud_rect(clip.x, clip.y, clip.width, clip.height, mapping)
}

/// Resolve the persisted Sound 0..15 setting to its effective linear gain.
fn configured_sfx_gain(config: &GameConfig) -> f32 {
    if config.sound_enabled {
        config.sfx_volume
    } else {
        0.0
    }
}

/// Which live audio subsystem, if any, owns a menu setting change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiveAudioSetting {
    SoundGain,
    AmbientGate,
    Unrelated,
}

fn classify_live_audio_setting(setting: SettingId) -> LiveAudioSetting {
    match setting {
        SettingId::SoundVolume => LiveAudioSetting::SoundGain,
        SettingId::AmbientVolume => LiveAudioSetting::AmbientGate,
        _ => LiveAudioSetting::Unrelated,
    }
}

/// Apply the binary Ambient gate and overlay-51 occupancy without changing
/// track or cursor.
fn apply_session_music_gate(config: &GameConfig, music_player: &MusicPlayer, results_active: bool) {
    if session_music_should_play(ambient_setting_value(config), results_active) {
        music_player.resume();
    } else {
        music_player.pause();
    }
}

/// Apply only the subsystem owned by the changed setting.
///
/// Overlay-51 occupancy stays in the music gate so a live Ambient write cannot
/// resume a track the results card has paused.
fn apply_live_audio_setting(
    setting: SettingId,
    config: &GameConfig,
    sound_manager: &mut Option<SoundManager>,
    music_player: &mut Option<MusicPlayer>,
    results_active: bool,
) {
    match classify_live_audio_setting(setting) {
        LiveAudioSetting::SoundGain => {
            if let Some(ref mut sm) = sound_manager {
                sm.set_master_volume(configured_sfx_gain(config));
            }
        }
        LiveAudioSetting::AmbientGate => {
            if let Some(ref mp) = music_player {
                apply_session_music_gate(config, mp, results_active);
            }
        }
        LiveAudioSetting::Unrelated => {}
    }
}

/// Apply the observable single-player suffix of one committed type-61 pass.
///
/// The resolver retains live-list order and mutates all gameplay owners
/// atomically. This adapter keeps presentation ordering in that same stream,
/// updates the selected callback only for an actual auto-selection, and names
/// any cross-subsystem follow-up that the caller must perform.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct PowerUpFeedbackResult {
    hull_state_changed: bool,
    targetter_acquired: bool,
}

fn player_vtol_boost(capabilities: &PlayerCapabilities) -> VtolBoost {
    if capabilities.has_turbo() {
        VtolBoost::Active
    } else {
        VtolBoost::Inactive
    }
}

/// Both physical selector-0x3F pickups and the time-award operation 0x13F
/// enter 445A90. Only the zero-amount hidden pickup claims the hidden text;
/// both share the counter-dependent sound and extra-life notification.
fn queue_trophy_acquisition_feedback(
    acquisition: TrophyAcquisition,
    position_raw: [i16; 3],
    notifications: &mut GameplayNotifications,
    world_fx: &mut WorldFx,
    retail_tick: u32,
) {
    if acquisition.first_claim {
        notifications.queue_trophy_level_claimed(retail_tick as i32);
    }
    if acquisition.awarded_extra_life {
        notifications.queue_trophy_reward(retail_tick as i32);
        world_fx.queue_fixed_positional_sound_raw_at_rate(
            TROPHY_PICKUP_SOUND_ID,
            position_raw,
            TROPHY_EXTRA_LIFE_RATE_Q16,
        );
    } else {
        world_fx.queue_fixed_positional_sound_raw(TROPHY_PICKUP_SOUND_ID, position_raw);
    }
}

fn apply_power_up_contact_feedback(
    pass: &PlayerPowerUpContactPass,
    weapon_inventory: &WeaponInventory,
    player_craft: &mut PlayerCraft,
    notifications: &mut GameplayNotifications,
    world_fx: &mut WorldFx,
    retail_tick: u32,
) -> PowerUpFeedbackResult {
    let mut weapon_callback_changed = false;
    let mut hull_restored = false;
    let mut targetter_acquired = false;

    for contact in pass.contacts.iter().copied() {
        match contact {
            PowerUpContactOutcome::Accepted(AcceptedPowerUpContact::Weapon {
                position_raw,
                acquisition:
                    WeaponAcquisition::Accepted {
                        slot,
                        auto_selected,
                        ..
                    },
                ..
            }) => {
                weapon_callback_changed |= auto_selected;
                if let Some(descriptor) = weapon_inventory.slot(slot) {
                    notifications.queue_weapon_pickup_hint(
                        descriptor.pickup_resource_event_id(),
                        retail_tick as i32,
                    );
                    world_fx.queue_fixed_positional_sound_raw(PLAYER_PICKUP_SOUND_ID, position_raw);
                    notifications.queue_weapon_collected_text(
                        descriptor.pickup_text_argument_id(),
                        retail_tick as i32,
                    );
                }
            }
            PowerUpContactOutcome::Accepted(AcceptedPowerUpContact::Weapon {
                acquisition: WeaponAcquisition::Rejected(_),
                ..
            }) => unreachable!("accepted contact contains rejected weapon"),
            PowerUpContactOutcome::Accepted(AcceptedPowerUpContact::Targetter {
                position_raw,
                ..
            }) => {
                targetter_acquired = true;
                notifications.queue_targetter_collected(retail_tick as i32);
                world_fx.queue_fixed_positional_sound_raw(PLAYER_PICKUP_SOUND_ID, position_raw);
            }
            PowerUpContactOutcome::Accepted(AcceptedPowerUpContact::Turbo {
                position_raw, ..
            }) => {
                notifications.queue_turbo_collected(retail_tick as i32);
                world_fx.queue_fixed_positional_sound_raw(PLAYER_PICKUP_SOUND_ID, position_raw);
            }
            PowerUpContactOutcome::Accepted(AcceptedPowerUpContact::NonSpawning {
                position_raw,
                acquisition,
                ..
            }) => match acquisition {
                NonSpawningPowerUpAcquisition::Fuel { .. } => {
                    notifications.queue_fuel_collected(retail_tick as i32);
                    world_fx.queue_fixed_positional_sound_raw(PLAYER_PICKUP_SOUND_ID, position_raw);
                }
                NonSpawningPowerUpAcquisition::Shield { .. } => {
                    hull_restored = true;
                    notifications.queue_shield_collected(retail_tick as i32);
                }
                NonSpawningPowerUpAcquisition::ExtraLife { .. } => {
                    notifications.queue_extra_life_collected(retail_tick as i32);
                    world_fx.queue_fixed_positional_sound_raw(PLAYER_PICKUP_SOUND_ID, position_raw);
                }
                NonSpawningPowerUpAcquisition::HullRepair { .. } => {
                    hull_restored = true;
                    notifications.queue_hull_repaired(retail_tick as i32);
                    world_fx.queue_fixed_positional_sound_raw(PLAYER_PICKUP_SOUND_ID, position_raw);
                }
                NonSpawningPowerUpAcquisition::CargoCapacity {
                    cargo_list_available,
                    ..
                } => {
                    if cargo_list_available {
                        notifications.queue_cargo_capacity_collected(retail_tick as i32);
                    }
                }
            },
            PowerUpContactOutcome::Accepted(AcceptedPowerUpContact::Trophy {
                position_raw,
                acquisition,
                ..
            }) => {
                hull_restored |= acquisition.restored_hull;
                queue_trophy_acquisition_feedback(
                    acquisition,
                    position_raw,
                    notifications,
                    world_fx,
                    retail_tick,
                );
            }
            PowerUpContactOutcome::Rejected(rejected) => match rejected.reason {
                PowerUpContactRejection::WeaponInventory(_) => {
                    world_fx.queue_fixed_positional_sound_raw(
                        WEAPON_PICKUP_REJECTED_SOUND_ID,
                        rejected.position_raw,
                    );
                }
                PowerUpContactRejection::TargetterAlreadyAcquired => {
                    world_fx.queue_fixed_positional_sound_raw_at_rate(
                        PLAYER_PICKUP_SOUND_ID,
                        rejected.position_raw,
                        TARGETTER_DUPLICATE_RATE_Q16,
                    );
                }
                PowerUpContactRejection::TurboAlreadyAcquired => {
                    world_fx.queue_fixed_positional_sound_raw_at_rate(
                        PLAYER_PICKUP_SOUND_ID,
                        rejected.position_raw,
                        TURBO_DUPLICATE_RATE_Q16,
                    );
                }
                PowerUpContactRejection::FuelAtCapacity { .. } => {
                    notifications.queue_fuel_full(retail_tick as i32);
                }
                PowerUpContactRejection::HullRepairAtCapacity { .. } => {
                    notifications.queue_hull_repair_full(retail_tick as i32);
                }
                PowerUpContactRejection::UnsupportedSelector { .. }
                | PowerUpContactRejection::Trophy(_) => {}
            },
        }
    }

    if weapon_callback_changed {
        sync_player_weapon_callback(player_craft, weapon_inventory);
    }
    PowerUpFeedbackResult {
        hull_state_changed: hull_restored,
        targetter_acquired,
    }
}

/// Rebind the selected persistent inventory descriptor to a freshly created
/// player component or to a descriptor auto-selection.
fn sync_player_weapon_callback(player_craft: &mut PlayerCraft, weapon_inventory: &WeaponInventory) {
    player_craft.select_weapon_callback(weapon_inventory.selected_descriptor().callback_selector());
}

/// Apply the component-side half of retail's post-shot automatic selection.
///
/// The final projectile and sound are already submitted before this phase.
/// Only a successful finite-ammunition transition changes callback word 4;
/// ordinary shots and attempts to fire an already-empty descriptor leave the
/// currently presented model unchanged.
fn sync_player_weapon_after_ammo_commit(
    commit: AmmoCommit,
    player_craft: &mut PlayerCraft,
    weapon_inventory: &WeaponInventory,
) {
    if matches!(
        commit,
        AmmoCommit::Fired {
            automatic_successor_slot: Some(_),
            ..
        }
    ) {
        sync_player_weapon_callback(player_craft, weapon_inventory);
    }
}

/// Retail ordinary worlds select the physical CD track world-style + 1.
fn normal_world_music_track(level_id: u32, world_style: u32) -> Option<u8> {
    if level_id == INTRO2_LEVEL_ID {
        return None;
    }
    Some((world_style.clamp(1, 6) + 1) as u8)
}

/// Dispatch menu rendering based on layout type.
/// Background flame billboard animation state (FUN_0042D030, table
/// 0x4CA938): 9 frames of {global sprite id, duration µs}. The cycle lasts
/// ≈940 ms; the wrap 8→0 plays the low rumble.
struct BillboardAnim {
    frame: usize,
    accum_micros: u32,
    /// Live menu-world depth-fade planes (`ctx+0x74/+0x78`).
    depth_fade_near_raw: i32,
    depth_fade_far_raw: i32,
}

/// One retail frontend submission spans both sides of `FUN_0042D030`.
/// Klaus and the current emblem use the evolved pre-advance state; later
/// flag-`0x02` prop rows see any frame-0/frame-4 near-plane pin.
#[derive(Debug, Clone, Copy, PartialEq)]
struct BillboardRenderFrame {
    sprite_id: u16,
    scene_depth_fade_near_raw: f32,
    scene_depth_fade_far_raw: f32,
    row_depth_fade_near_raw: f32,
}

/// (global sprite id, duration µs). Frame 0 lasts a single video frame.
const BILLBOARD_FRAMES: [(u16, u32); 9] = [
    (1295, 0),
    (1299, 120_000),
    (1298, 80_000),
    (1297, 80_000),
    (1296, 80_000),
    (1295, 80_000),
    (1294, 180_000),
    (1295, 140_000),
    (1294, 180_000),
];

impl BillboardAnim {
    /// Matches the .data initial state (frame 8, accumulator 2,000,000 µs):
    /// the very first update wraps and fires the rumble once at menu entry.
    fn new() -> Self {
        Self {
            frame: 8,
            accum_micros: 2_000_000,
            depth_fade_near_raw: 0x1000,
            depth_fade_far_raw: 0x1400,
        }
    }

    /// Prepare the two timing snapshots consumed by one frontend draw.
    ///
    /// `FUN_0042B5D0` first evolves the live planes. `FUN_0042B870` then
    /// queues Klaus and the current emblem, calls `FUN_0042D030`, and applies
    /// the returned frame-0/frame-4 pin before `FUN_0042BA30` visits prop rows.
    fn prepare_frontend_render_frame(
        &mut self,
        elapsed_micros: u32,
        before_actor: FrontendDepthFadeMode,
        after_actor: FrontendDepthFadeMode,
    ) -> (BillboardRenderFrame, bool) {
        self.evolve_depth_fade(elapsed_micros, before_actor);
        let sprite_id = self.sprite_id();
        let scene_depth_fade_near_raw = self.depth_fade_near_raw as f32;
        let scene_depth_fade_far_raw = self.depth_fade_far_raw as f32;
        let wrapped = self.advance(elapsed_micros);
        self.apply_frontend_pin(after_actor);
        (
            BillboardRenderFrame {
                sprite_id,
                scene_depth_fade_near_raw,
                scene_depth_fade_far_raw,
                row_depth_fade_near_raw: self.depth_fade_near_raw as f32,
            },
            wrapped,
        )
    }

    /// Intro2's `FUN_004537F0` calls `FUN_0042D030` directly: queue the old
    /// sprite and advance its clock without evolving or pinning menu planes.
    fn prepare_cinematic_frame(&mut self, elapsed_micros: u32) -> (u16, bool) {
        let sprite_id = self.sprite_id();
        let wrapped = self.advance(elapsed_micros);
        (sprite_id, wrapped)
    }

    fn evolve_depth_fade(&mut self, elapsed_micros: u32, mode: FrontendDepthFadeMode) {
        // FUN_0042B5D0 compares the complete 42D210 return to 1. The
        // decompiler's bool signature hides Selected=4 and FlyOff=5.
        let near_step = (elapsed_micros / 200) as i32;
        match mode {
            FrontendDepthFadeMode::RingPulse => {
                let far_step = (elapsed_micros / 1000) as i32;
                self.depth_fade_far_raw = (self.depth_fade_far_raw - far_step).max(0x1000);
                self.depth_fade_near_raw = (self.depth_fade_near_raw + near_step).min(0x0C00);
            }
            FrontendDepthFadeMode::Receded => {
                self.depth_fade_far_raw = (self.depth_fade_far_raw + near_step).min(0x1900);
                self.depth_fade_near_raw = (self.depth_fade_near_raw + near_step).min(0x1500);
            }
        }
    }

    /// `FUN_0042D030` advances at most one entry per render and resets its
    /// accumulator on wrap.
    fn advance(&mut self, elapsed_micros: u32) -> bool {
        self.accum_micros += elapsed_micros;
        let mut wrapped = false;
        let dur = BILLBOARD_FRAMES[self.frame].1;
        if dur < self.accum_micros {
            self.accum_micros -= dur;
            self.frame += 1;
            if self.frame >= BILLBOARD_FRAMES.len() {
                self.frame = 0;
                self.accum_micros = 0;
                wrapped = true;
            }
        }
        wrapped
    }

    /// `FUN_0042B870` applies these after queuing the old/current billboard.
    fn apply_frontend_pin(&mut self, mode: FrontendDepthFadeMode) {
        self.depth_fade_near_raw = match (mode, self.frame) {
            (FrontendDepthFadeMode::RingPulse, 0) => 0x500,
            (FrontendDepthFadeMode::RingPulse, 4) => 0x800,
            (FrontendDepthFadeMode::Receded, 0) => self.depth_fade_near_raw.min(0xF00),
            (FrontendDepthFadeMode::Receded, 4) => self.depth_fade_near_raw.min(0x1200),
            _ => self.depth_fade_near_raw,
        };
    }

    /// Current frame's global sprite id.
    fn sprite_id(&self) -> u16 {
        BILLBOARD_FRAMES[self.frame].0
    }
}

use v2k_render::{UiAnchor, UiMapping, UiMappingRequest, UiSubmissionPolicy};

#[path = "gameplay_text_submission.rs"]
mod gameplay_text_submission;
use gameplay_text_submission::GameplayTextContext;

#[path = "menu_text_submission.rs"]
mod menu_text_submission;
#[cfg(test)]
use menu_text_submission::apply_menu_text_tone;
use menu_text_submission::{
    draw_menu_bar, draw_menu_text, draw_menu_text_toned, menu_list_clip_y,
    typewriter_chars_from_elapsed, MenuTextTone,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuPresentation {
    Frontend,
    GameplayPause,
}

/// FUN_0042A860 resolves aliases through the process RNG before attempting
/// an audio-device voice. Klaus uses this same phase synchronously inside
/// C090, so the whoosh's alias draw precedes its twitch draw even without sound.
struct GlobalSoundRuntime<'a> {
    pool: SoundPool<'a>,
    world_fx: &'a mut WorldFx,
    sound_manager: Option<&'a mut SoundManager>,
}

impl<'a> GlobalSoundRuntime<'a> {
    fn new(
        cache: &'a v2k_game::resource_cache::ResourceCache,
        world_fx: &'a mut WorldFx,
        sound_manager: Option<&'a mut SoundManager>,
    ) -> Self {
        Self {
            pool: SoundPool::from_tables(&cache.global_sound_tables()),
            world_fx,
            sound_manager,
        }
    }

    fn play_global_sound(&mut self, global_sound_id: usize) {
        let resolved = resolve_sound_playback(
            &self.pool,
            SoundPlaybackRequest::centered(global_sound_id),
            &mut || self.world_fx.next_shared_retail_random_u16(),
        );
        match resolved {
            Ok(sound) => {
                if let Some(manager) = self.sound_manager.as_mut() {
                    manager.play_resolved(sound);
                }
            }
            Err(error) => eprintln!("Global sound {global_sound_id}: {error:?}"),
        }
    }
}

impl v2k_game::game_state::KlausEffects for GlobalSoundRuntime<'_> {
    fn next_random_u16(&mut self) -> u16 {
        self.world_fx.next_shared_retail_random_u16()
    }

    fn play_sound(&mut self, global_sound_id: u32) {
        self.play_global_sound(global_sound_id as usize);
    }
}

#[derive(Clone, Copy)]
struct KlausHandoffFrame<'a> {
    shell: &'a MenuShell,
    cache: &'a v2k_game::resource_cache::ResourceCache,
    colors: &'a v2k_game::model_color::ModelMaterialCache,
    retail_tick: u32,
}

/// Retail submits the persistent frontend actor over the resident world.
/// Its model's authored polygons form the iris; only depth is reset between
/// the independent world and frontend camera spaces.
fn draw_klaus_handoff_overlay(
    renderer: &mut dyn v2k_render::Renderer,
    frame: KlausHandoffFrame<'_>,
) {
    if !frame.shell.menu_backdrop_anim_state().visible || !renderer.supports_models() {
        return;
    }
    renderer.begin_scene(RenderScene::Menu);
    renderer.clear_depth();
    let (width, height) = renderer.viewport_size();
    renderer.set_camera(&menu_camera(width as f32 / height.max(1) as f32));
    draw_menu_backdrop(
        renderer,
        frame.cache,
        frame.colors,
        MenuBackdropFrame {
            presentation: MenuBackdropPresentation::Handoff,
            retail_tick: frame.retail_tick,
            anim_state: frame.shell.menu_backdrop_anim_state(),
            view_depth_raw: frame
                .shell
                .menu_backdrop_depth_raw()
                .max(MENU_BACKDROP_VIEW_DEPTH_RAW),
            projection_y_offset: frame.shell.menu_billboard_y_offset(),
            depth_fade: klaus_handoff_depth_fade(frame.cache),
        },
        MenuSceneLight::NEUTRAL,
    );
}

impl MenuPresentation {
    fn uses_frontend_hangar(self) -> bool {
        self == Self::Frontend
    }

    fn uses_frozen_world(self) -> bool {
        self == Self::GameplayPause
    }
}

#[allow(clippy::too_many_arguments)]
fn render_menu(
    renderer: &mut dyn v2k_render::Renderer,
    shell: &MenuShell,
    system_strings: &[String],
    menu_resources: &v2k_game::menu::MenuResources,
    fonts: Option<&v2k_game::menu_text::MenuFonts>,
    cache: &v2k_game::resource_cache::ResourceCache,
    colors: &v2k_game::model_color::ModelMaterialCache,
    billboard: &BillboardRenderFrame,
    retail_tick: u32,
) {
    render_menu_scene(
        renderer,
        shell,
        system_strings,
        menu_resources,
        fonts,
        cache,
        colors,
        Some(billboard),
        retail_tick,
        MenuPresentation::Frontend,
        None,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_pause_menu(
    renderer: &mut dyn v2k_render::Renderer,
    shell: &MenuShell,
    system_strings: &[String],
    menu_resources: &v2k_game::menu::MenuResources,
    fonts: Option<&v2k_game::menu_text::MenuFonts>,
    cache: &v2k_game::resource_cache::ResourceCache,
    colors: &v2k_game::model_color::ModelMaterialCache,
    retail_tick: u32,
    frozen_world: Option<&CapturedFrame>,
) {
    render_menu_scene(
        renderer,
        shell,
        system_strings,
        menu_resources,
        fonts,
        cache,
        colors,
        None,
        retail_tick,
        MenuPresentation::GameplayPause,
        frozen_world,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_menu_scene(
    renderer: &mut dyn v2k_render::Renderer,
    shell: &MenuShell,
    system_strings: &[String],
    menu_resources: &v2k_game::menu::MenuResources,
    fonts: Option<&v2k_game::menu_text::MenuFonts>,
    cache: &v2k_game::resource_cache::ResourceCache,
    colors: &v2k_game::model_color::ModelMaterialCache,
    billboard: Option<&BillboardRenderFrame>,
    retail_tick: u32,
    presentation: MenuPresentation,
    frozen_world: Option<&CapturedFrame>,
) {
    renderer.begin_scene(RenderScene::Menu);
    debug_assert_eq!(
        billboard.is_some(),
        presentation.uses_frontend_hangar(),
        "only the frontend presentation owns an emblem/depth-fade snapshot"
    );

    if presentation.uses_frozen_world() {
        // A display/scaling change can expose a different set of bars around
        // the active viewport. Clear the complete output first, then restore
        // the captured world into the current viewport so those bars cannot
        // inherit stale pixels from an older back buffer.
        renderer.clear(0.0, 0.0, 0.0);
        if let Some(frame) = frozen_world {
            renderer.draw_fullscreen(&frame.rgba, frame.width, frame.height);
        }
        renderer.clear_depth();
    }

    let Some(menu) = shell.render_view() else {
        if presentation.uses_frozen_world() {
            renderer.present();
            return;
        }
        // Scene-only presentation: backdrop + emblem, no screen UI. The cold
        // boot intro retains the frontend branding; both Klaus mouth bridges
        // suppress it, matching the retail New Game capture.
        let billboard = billboard.expect("frontend scene requires its render snapshot");
        let (vw, vh) = renderer.viewport_size();
        renderer.clear(0.01, 0.01, 0.03);
        renderer.set_camera(&menu_camera(vw as f32 / vh as f32));
        if renderer.supports_models() {
            let backdrop_depth = shell
                .menu_backdrop_depth_raw()
                .max(MENU_BACKDROP_VIEW_DEPTH_RAW);
            draw_menu_backdrop(
                renderer,
                cache,
                colors,
                MenuBackdropFrame {
                    presentation: MenuBackdropPresentation::Frontend,
                    retail_tick,
                    anim_state: shell.menu_backdrop_anim_state(),
                    view_depth_raw: backdrop_depth,
                    projection_y_offset: shell.menu_billboard_y_offset(),
                    depth_fade: menu_depth_fade_from_raw(
                        billboard.scene_depth_fade_near_raw,
                        billboard.scene_depth_fade_far_raw,
                        menu_depth_fade_color(cache),
                    ),
                },
                MenuSceneLight::NEUTRAL,
            );
        }
        draw_billboard(
            renderer,
            cache,
            menu_resources,
            billboard,
            shell.menu_billboard_pose(),
        );
        if shell.frontend_branding_visible() {
            draw_menu_branding(renderer, system_strings, menu_resources, vw, vh);
        }
        renderer.present();
        return;
    };

    // The ring needs every prop model resolvable; otherwise fall back to
    // the 2D sprite carousel (also used by the software backend).
    let ring_3d = renderer.supports_models()
        && menu.layout == MenuLayout::Carousel
        && !menu.items.is_empty()
        && menu.items.iter().all(|i| {
            i.model_id
                .is_some_and(|id| cache.global_model(id as usize).is_some())
        });

    debug_assert!(
        presentation.uses_frontend_hangar() || menu.layout == MenuLayout::VerticalList,
        "the proven PAUSE_FULL path is a vertical list"
    );

    match menu.layout {
        MenuLayout::Carousel if ring_3d => render_ring_3d(
            renderer,
            shell,
            menu,
            system_strings,
            menu_resources,
            fonts,
            cache,
            colors,
            billboard.expect("frontend ring requires its render snapshot"),
            retail_tick,
        ),
        MenuLayout::Carousel => {
            render_carousel(renderer, shell, menu, system_strings, menu_resources)
        }
        MenuLayout::VerticalList => render_list(
            renderer,
            shell,
            menu,
            system_strings,
            fonts,
            cache,
            colors,
            menu_resources,
            billboard,
            retail_tick,
            presentation,
        ),
    }

    renderer.present();
}

/// Camera for the 3D menu scene: at the origin looking down −Z toward the
/// prop ring (original ring depths 2300..4700 raw units → z −23..−47).
/// FOV matches the engine's fixed-point projection (focal ≈ 256 px for a
/// 320×240 frame → vertical FOV = 2·atan(120/256) ≈ 50.4°).
fn menu_camera(aspect: f32) -> v2k_render::Camera {
    let mut cam = v2k_render::Camera::new(aspect);
    cam.position = [0.0, 0.0, 0.0];
    cam.yaw = 0.0;
    cam.pitch = 0.0;
    cam.fov = 50.4_f32.to_radians();
    cam.near = MENU_CAMERA_NEAR;
    cam.far = MENU_CAMERA_FAR;
    cam
}

/// V2000 changes the renderer context's projection centre for each prop row
/// (`ctx+0x5c/+0x60`). Using a true off-centre projection keeps the authored
/// model at its fixed 3-D position while it flies through depth; translating
/// the model by a depth-dependent Y offset subtly bends articulated geometry.
fn menu_camera_for_projection_y(
    projection_y: f32,
    request: UiMappingRequest,
) -> v2k_render::Camera {
    if request.policy == UiSubmissionPolicy::NativeCanvas {
        // The foreground frame and its text share the authored canvas. Native
        // fullscreen must scale both their anchor and their pixel focal length.
        return menu_camera_for_authored_point(
            [request.authored_canvas[0] as f32 * 0.5, projection_y],
            request,
        );
    }
    // Low Native and the fitted classic modes retain their existing lens,
    // including the full-height projection-centre policy on a narrow drawable.
    let mut cam = menu_camera(request.viewport[0] as f32 / request.viewport[1].max(1) as f32);
    cam.projection_offset[1] = 2.0 * projection_y / request.authored_canvas[1].max(1) as f32 - 1.0;
    cam
}

/// Off-centre menu camera whose origin projects to one authored 2-D point.
/// The authored point and the model's focal length use the same explicit UI
/// submission policy as its sprites and text. Native high art grows together;
/// Fit retains the existing menu lens. OpenGL projection X has the opposite
/// stored sign from top-origin screen X.
fn menu_camera_for_authored_point(
    point: [f32; 2],
    request: UiMappingRequest,
) -> v2k_render::Camera {
    let [viewport_width, viewport_height] = request.viewport;
    let UiMapping {
        scale,
        offset_x,
        offset_y,
    } = UiMapping::new(request);
    let screen_x = offset_x as f32 + point[0] * scale;
    let screen_y = offset_y as f32 + point[1] * scale;
    let mut cam = menu_camera(viewport_width as f32 / viewport_height.max(1) as f32);
    if matches!(
        request.policy,
        UiSubmissionPolicy::NativeCanvas | UiSubmissionPolicy::NativeHud { .. }
    ) {
        // A full-viewport perspective would enlarge the model independently of
        // its sprites. Apply the authored canvas's scale to the pixel focal
        // length in this dedicated foreground camera, including down-fitting.
        let mapped_canvas_height = (request.authored_canvas[1].max(1) as f32 * scale).max(1.0);
        let viewport_to_canvas = viewport_height.max(1) as f32 / mapped_canvas_height;
        cam.fov = 2.0 * (viewport_to_canvas * (cam.fov * 0.5).tan()).atan();
    }
    cam.projection_offset = [
        1.0 - 2.0 * screen_x / viewport_width.max(1) as f32,
        2.0 * screen_y / viewport_height.max(1) as f32 - 1.0,
    ];
    cam
}

/// Convert the scene entity's vertical displacement into world space. Menu
/// prop rows use [`menu_camera_for_projection_y`] instead; this helper remains
/// for Klaus, whose tracked entity genuinely moves vertically in FUN_0042C090.
fn menu_world_y_for_projection_center(
    local_y_raw: f32,
    view_depth_raw: f32,
    projection_y: f32,
    virtual_height: f32,
) -> f32 {
    let depth = view_depth_raw / 100.0;
    let center_y = virtual_height * 0.5;
    let focal_y = center_y / (50.4_f32.to_radians() * 0.5).tan();
    local_y_raw / 100.0 - (projection_y - center_y) * depth / focal_y
}

/// Nearest toroidal image of a world position relative to the camera, so
/// objects near the world seam draw on the correct side (engine camera-relative
/// unwrap). Y passes through unchanged (the world does not wrap vertically).
fn camera_relative(camera: &v2k_render::Camera, pos: [f32; 3]) -> [f32; 3] {
    [
        camera.position[0] + v2k_core::world::delta(pos[0], camera.position[0]),
        pos[1],
        camera.position[2] + v2k_core::world::delta(pos[2], camera.position[2]),
    ]
}

fn queue_checked_projectile_damage_audio(
    world_fx: &mut WorldFx,
    outcome: &CheckedProjectileDamageOutcome,
) {
    match outcome {
        CheckedProjectileDamageOutcome::Applied(applied) => {
            // FUN_00414E90's generic-hit submission precedes the
            // target-specific accepted-hit presentation.
            if let Some(sound_id) = applied.transition.generic_hit_sound_id {
                world_fx.queue_fixed_positional_sound_raw(sound_id, applied.target_position_raw);
            }
            if let Some(sound_id) = applied
                .accepted_hit_presentation
                .and_then(|presentation| presentation.sound_id)
            {
                world_fx.queue_fixed_positional_sound_raw(sound_id, applied.target_position_raw);
            }
        }
        CheckedProjectileDamageOutcome::ProgressiveDeathStarted(started) => {
            // The first lethal hit submits generic-hit, generic-death, then
            // accepted-hit audio before FUN_00419750 exposes the revived
            // structure.
            for sound_id in [
                started.generic_hit_sound_id,
                started.death_sound_id,
                started
                    .accepted_hit_presentation
                    .and_then(|presentation| presentation.sound_id),
            ]
            .into_iter()
            .flatten()
            {
                world_fx.queue_fixed_positional_sound_raw(sound_id, started.target_position_raw);
            }
        }
        CheckedProjectileDamageOutcome::HiveDeathStarted(started) => {
            if let Some(sound_id) = started.generic_hit_sound_id {
                world_fx.queue_fixed_positional_sound_raw(sound_id, started.target_position_raw);
            }
            if let Some(sound_id) = started
                .accepted_hit_presentation
                .and_then(|presentation| presentation.sound_id)
            {
                world_fx.queue_fixed_positional_sound_raw(sound_id, started.target_position_raw);
            }
        }
        _ => {}
    }
}

/// Apply immediate mutation branches returned by `FUN_00427950`.
///
/// A null static program burns its cell immediately through `FUN_004337E0`.
/// Kind 10 does not enqueue a program when a positive accepted hit finds its
/// terrain bit already set. Retail clears bit `0x08` through `FUN_004337E0`
/// and then increments the live Section-10 attribute byte through
/// `FUN_004338C0`.
fn apply_immediate_static_damage_outcome(
    outcome: StaticDamageOutcome,
    cache: &mut v2k_game::resource_cache::ResourceCache,
    world_fx: &mut WorldFx,
) {
    match outcome {
        StaticDamageOutcome::ImmediateBurn { cell, .. } => {
            if let Err(error) =
                v2k_game::static_terrain_burn::apply_immediate_static_burn(cell, cache, world_fx)
            {
                log!("Static burn callback blocked: {error:?}");
            }
        }
        StaticDamageOutcome::BurnedKind10Transition { cell, .. } => {
            cache.apply_burned_kind_10_transition(cell);
        }
        _ => {}
    }
}

/// Apply static-program opcode 6 against retail's current focus spring.
///
/// `FUN_0040F9A0` forwards through the shared wrapped-word distance helper and
/// `FUN_004281A0` invokes `FUN_00456750` on the inclusive `<=` branch. The
/// request deliberately restarts an active sequence at table index one.
fn request_full_frame_sequence_within(
    sequence: &mut FullFrameSpriteSequence,
    position_raw: [i16; 3],
    focus_position_raw: [i16; 3],
    max_distance_raw: i32,
) -> bool {
    if radial_distance_raw(position_raw, focus_position_raw) > max_distance_raw {
        return false;
    }
    sequence.request();
    true
}

/// Static blasts retain the callback custody of the world running their FIFO.
enum StaticDamageWorldContext<'a> {
    Intro2 {
        actor_tasks: &'a mut SpecializedActorTaskScheduler,
        notifications: &'a mut GameplayNotifications,
        retail_tick: u32,
    },
    Playing {
        player_hull: &'a mut PlayerHull,
        extra_lives: RetailRuntimeValue<u8>,
        actor_tasks: &'a mut SpecializedActorTaskScheduler,
        notifications: &'a mut GameplayNotifications,
        retail_tick: u32,
    },
}

/// Apply one FIFO node's scheduler batch in program order. The caller keeps
/// the node registered in `StaticDamageScheduler` throughout this function,
/// matching the retail node that opcode 0 removes only after opcode 12 returns.
fn apply_static_damage_actions(
    scheduler: &mut StaticDamageScheduler,
    actions: Vec<StaticDamageAction>,
    cache: &mut v2k_game::resource_cache::ResourceCache,
    entities: &mut EntityManager,
    mut world: StaticDamageWorldContext<'_>,
    world_fx: &mut WorldFx,
    explosion_lights: &mut Vec<TerrainExplosionLight>,
    camera_focus_raw: [i16; 3],
    outer_frame_sequence: &mut FullFrameSpriteSequence,
) -> Vec<v2k_game::intro2_radial::Intro2RadialReport> {
    let mut radial_reports = Vec::new();
    for action in actions {
        match action {
            StaticDamageAction::Effect18 { position_raw } => {
                let sea_level_raw = cache.terrain().map(|terrain| terrain.sea_level_raw());
                world_fx.emit_static_effect18_raw(position_raw, sea_level_raw);
            }
            StaticDamageAction::CommonExplosion30 {
                position_raw,
                source_extent_raw,
            } => {
                explosion_lights.push(
                    world_fx.emit_common_explosion_bundle_raw(position_raw, source_extent_raw),
                );
            }
            StaticDamageAction::BallisticScatter79 { position_raw } => {
                world_fx.emit_static_kind9_scatter_raw(position_raw);
            }
            StaticDamageAction::OrdinaryEffect79 { position_raw } => {
                world_fx.emit_static_effect79_raw(position_raw);
            }
            StaticDamageAction::FixedSound {
                sound_id,
                position_raw,
                gain_q16,
                rate_q16,
            } => {
                debug_assert_eq!(gain_q16, 0x1_0000);
                debug_assert_eq!(rate_q16, 0x1_0000);
                world_fx.emit_fixed_positional_sound_raw(sound_id, position_raw);
            }
            StaticDamageAction::RequestFullFrameSequenceWithin {
                position_raw,
                max_distance_raw,
            } => {
                request_full_frame_sequence_within(
                    outer_frame_sequence,
                    position_raw,
                    camera_focus_raw,
                    max_distance_raw,
                );
            }
            StaticDamageAction::SetBurned { cell } => {
                if let Err(error) = v2k_game::static_terrain_burn::apply_static_terrain_burn(
                    cell, cache, entities, world_fx, scheduler,
                ) {
                    eprintln!("static opcode10 cell {cell:?}: {error:?}; committed callback is not replayed");
                    break;
                }
            }
            StaticDamageAction::LowerTerrainLight { cell, amount } => {
                if let Err(error) = cache.lower_level_terrain_light(cell, amount, &mut || {
                    world_fx.next_shared_retail_random_u16()
                }) {
                    eprintln!(
                        "static opcode11 cell {cell:?}: {error}; consumed program is not replayed"
                    );
                }
            }
            StaticDamageAction::Radial {
                source_cell: _,
                origin_raw,
                template,
            } => {
                let (player_hull, extra_lives, actor_tasks, notifications, retail_tick) =
                    match &mut world {
                        StaticDamageWorldContext::Intro2 {
                            actor_tasks,
                            notifications,
                            retail_tick,
                        } => {
                            let report = v2k_game::intro2_radial::apply_intro2_radial_damage(
                                &mut v2k_game::intro2_radial::Intro2RadialFrame {
                                    active_terminal_calls: Vec::new(),
                                    entities,
                                    resources: cache,
                                    world_fx,
                                    static_damage: scheduler,
                                    notifications,
                                    retail_tick: *retail_tick,
                                    actor_tasks: &mut **actor_tasks,
                                },
                                origin_raw,
                                template,
                            );
                            // Opcode12 returns to this node even when the port
                            // cannot execute a later dynamic target. Never replay
                            // its committed prefix; the caller consumes the batch.
                            radial_reports.push(report);
                            continue;
                        }
                        StaticDamageWorldContext::Playing {
                            player_hull,
                            extra_lives,
                            actor_tasks,
                            notifications,
                            retail_tick,
                        } => (
                            &mut **player_hull,
                            *extra_lives,
                            &mut **actor_tasks,
                            &mut **notifications,
                            *retail_tick,
                        ),
                    };
                let static_hits = cache.terrain().map_or_else(Vec::new, |terrain| {
                    scan_static_radial(terrain, origin_raw, template, |cell| {
                        resolve_current_static_damage_target(cache, cell)
                            .ok()
                            .flatten()
                            .map(|target| target.state)
                    })
                });
                // FUN_004566E0 always completes this X-outer/Z-inner static
                // pass before entering its separate dynamic-entity pass.
                for hit in static_hits {
                    let outcome = scheduler.submit_hit(hit.target, hit.packet, &mut || {
                        world_fx.next_shared_retail_random_u16()
                    });
                    apply_immediate_static_damage_outcome(outcome, cache, world_fx);
                    log!(
                        "Static radial: cell ({}, {}) distance {} {outcome:?}",
                        hit.target.cell[0],
                        hit.target.cell[1],
                        hit.distance_raw
                    );
                }

                // Each14AE0 callback and its RNG complete before sampling
                // the next live target; an unsupported later target retains
                // the already-completed prefix.
                let result = actor_tasks.apply_playing_radial_damage(
                    v2k_game::specialized_actor_task_production::PlayingRadialFrame {
                        resources: cache,
                        static_damage: scheduler,
                        active_terminal_calls: Vec::new(),
                        entities,
                        player_hull,
                        extra_lives,
                        origin_raw,
                        template,
                        world_fx,
                        notifications,
                        retail_tick,
                    },
                );
                log!("Dynamic radial: {result:?}");
            }
        }
    }
    radial_reports
}

/// Per-world-model light-table shift from `FUN_004136C0` and
/// `FUN_004138F0`: signed terrain light at the entity cell minus
/// `FUN_004336E0`'s 0..8 underwater darkness step. World coordinates remain
/// exact signed 8.8 values in the entity paths, so the float-to-word boundary
/// only restores the representation already used by the simulation.
fn world_model_shade_shift(
    position: [f32; 3],
    terrain: Option<&v2k_formats::terrain::TerrainGrid>,
    terrain_lights: Option<&v2k_render::TerrainLightWindow>,
) -> i32 {
    let cell_x = position[0].floor() as i32;
    let cell_z = position[2].floor() as i32;
    let light_delta = terrain_lights
        .map(|lights| lights.sample(cell_x, cell_z))
        .unwrap_or(0);
    let Some(terrain) = terrain else {
        return i32::from(light_delta);
    };
    let height_raw = (position[1] * 256.0) as i32 as i16;
    v2k_game::model_color::contextual_model_shade_shift(
        light_delta,
        height_raw,
        terrain.darkness_start_world_y(),
        terrain.darkness_range(),
    )
}

/// Submit the authored Section-10 static-object layer over the same wrapped
/// footprint as opaque terrain. `FUN_0042F650` uses an identity root basis,
/// raw cell centre `+0x80`, and exposes only the global 50 Hz tick on dynamic
/// animation channel zero.
#[allow(clippy::too_many_arguments)]
fn draw_static_terrain_objects(
    renderer: &mut dyn v2k_render::Renderer,
    cache: &v2k_game::resource_cache::ResourceCache,
    colors: &v2k_game::model_color::ModelMaterialCache,
    camera: &Camera,
    camera_mode: GameplayWorldCameraMode,
    terrain_frames: Option<&v2k_render::TerrainFrames>,
    terrain_lights: Option<&v2k_render::TerrainLightWindow>,
    retail_tick: u32,
    world_fx: &mut WorldFx,
) {
    let (Some(terrain), Some(objects)) = (cache.terrain(), cache.terrain_objects()) else {
        return;
    };
    let scan_dimensions = terrain_frames
        .map(|frames| (frames.scan_columns, frames.scan_rows))
        .unwrap_or((52, 30));
    let mut vars = AnimVars::default();
    vars.dynamic[0] = (retail_tick & 0xFFFF) as i32;
    let orientation = orientation_from_ypr(0.0, 0.0, 0.0);

    let visible_objects = match camera_mode {
        // FUN_0042F530 walks whole cell words on the world axes.
        GameplayWorldCameraMode::RetailChase => collect_retail_static_terrain_objects(
            terrain,
            objects,
            camera.position.map(|value| (value * 256.0).round() as i32),
            v2k_core::render_scan::terrain_row_lead_raw(camera.forward()[1]),
            scan_dimensions,
        ),
        GameplayWorldCameraMode::Free => collect_static_terrain_objects(
            terrain,
            objects,
            camera.position,
            camera.forward(),
            scan_dimensions,
        ),
    };
    let sea_level = terrain.water_enabled().then(|| terrain.sea_level_world_y());
    world_fx.emit_static_terrain_object_particles(&visible_objects, sea_level);
    // Retail allocates these records directly from the static-object draw
    // traversal, so they are visible at age zero in the same frame.

    for object in visible_objects {
        // Retail draws static terrain objects with the identity root
        // orientation (FORMAT_DOCUMENTATION.md §9); tree trunks stay authored
        // and their bases are grounded by the world tf-12 callback family in
        // the renderer, not by yawing geometry toward the camera.
        ModelTreeRenderer::new_world(
            renderer,
            cache,
            colors,
            GAMEPLAY_MODEL_SCALE,
            Some(MenuSceneLight::NEUTRAL),
            retail_tick as i32,
        )
        .with_view(camera.into())
        .with_external_frame(ExternalFrameMode::WorldPoint(
            object.external_frame_world_point(),
        ))
        .with_shade_shift(world_model_shade_shift(
            object.position,
            cache.terrain(),
            terrain_lights,
        ))
        .draw_linked(
            usize::from(object.model_id),
            orientation,
            object.position,
            8,
            None,
            &vars,
        );
    }
}

struct ParticleCallbackSnapshot {
    owner_motions: Vec<ParticleOwnerMotion>,
    collision_models: Vec<EntityCollisionModel>,
}

/// Advance the shared Main Base / Working Factory destruction callback and
/// materialize its authored Section-8 effect anchors into the common retail
/// explosion bundle.
///
/// Animation inputs are snapshotted before the callback advances because the
/// terminal stage switches the entity to its destroyed model and clears the
/// factory counters. The stage-31 walk still belongs to the preceding normal
/// model frame when both boundaries are crossed in one host update.
struct BaseFactoryProgressionFrame {
    explosion_lights: Vec<TerrainExplosionLight>,
    terminal_abort_origin: Option<MainBaseTerminalAbortOrigin>,
    full_frame_sequence_requested: bool,
}

fn advance_base_factory_progression(
    cache: &v2k_game::resource_cache::ResourceCache,
    entities: &mut EntityManager,
    world_fx: &mut WorldFx,
    elapsed_micros: u32,
    retail_tick: u32,
    specialized_claims: &[v2k_game::main_base_abort::MainBaseAbortActorLease],
) -> BaseFactoryProgressionFrame {
    let anim_snapshots = entities
        .iter_all()
        .map(|entity| (entity.id, entity.presentation_anim_vars(retail_tick)))
        .collect::<Vec<_>>();
    let events = entities
        .advance_unclaimed_base_factory_progressive_deaths(elapsed_micros, specialized_claims);
    let mut lights = Vec::new();
    let mut terminal_abort_origin = None;
    let mut full_frame_sequence_requested = false;

    for event in events {
        match event {
            BaseFactoryProgressionEvent::ModelEffectStage {
                target_id,
                model_index,
                model_origin_raw,
                heading,
                effect,
            } => {
                let Some(model) = cache.global_model(model_index) else {
                    log!("Base/factory effect model {model_index} missing for entity {target_id}");
                    continue;
                };
                let Some((_, anim_vars)) = anim_snapshots
                    .iter()
                    .find(|(entity_id, _)| *entity_id == target_id)
                else {
                    log!("Base/factory effect entity {target_id} missing from frame snapshot");
                    continue;
                };
                let request = base_factory_staged_effect_request(model_origin_raw, heading);
                let points = match model.staged_effect_points_raw(request, anim_vars, cache) {
                    Ok(points) => points,
                    Err(error) => {
                        log!(
                            "Base/factory effect walk unresolved for entity {target_id}, model {model_index}: {error:?}"
                        );
                        continue;
                    }
                };

                for point in points {
                    let accepted = effect.threshold >= 0x1_0000
                        || i32::from(world_fx.next_shared_retail_random_u16() & 0xff)
                            < effect.threshold;
                    if !accepted {
                        continue;
                    }
                    let position_raw = point
                        .center_raw
                        .map(|component| (component.round() as i32) as i16);
                    lights.push(
                        world_fx.emit_common_explosion_bundle_raw(
                            position_raw,
                            point.scatter_radius_raw,
                        ),
                    );
                }
            }
            BaseFactoryProgressionEvent::TerminalDeath {
                target_id: _,
                target_position_raw,
                death_sound_id,
                full_frame_sequence_requested: request_full_frame,
            } => {
                if let Some(sound_id) = death_sound_id {
                    world_fx.queue_fixed_positional_sound_raw(sound_id, target_position_raw);
                }
                full_frame_sequence_requested |= request_full_frame;
            }
            BaseFactoryProgressionEvent::MainBaseLevelAbort { origin } => {
                let target_id = origin.target_id();
                if terminal_abort_origin.is_none() {
                    terminal_abort_origin = Some(origin);
                } else {
                    log!(
                        "Main Base entity {target_id} produced a duplicate terminal origin in one progression frame"
                    );
                }
                log!("Main Base entity {target_id} entered terminal level-abort state");
            }
            BaseFactoryProgressionEvent::Unresolved { target_id, reason } => {
                log!("Base/factory progression unresolved for entity {target_id}: {reason:?}");
            }
        }
    }

    BaseFactoryProgressionFrame {
        explosion_lights: lights,
        terminal_abort_origin,
        full_frame_sequence_requested,
    }
}

fn base_factory_staged_effect_request(
    model_origin_raw: [i16; 3],
    heading: f32,
) -> StagedEffectRequest {
    let orientation = non_player_model_orientation(heading);
    StagedEffectRequest {
        model_to_output_basis: orientation.map(|row| row.map(f64::from)),
        model_origin_raw: model_origin_raw.map(f64::from),
    }
}

/// Build the frame-start entity state read by particle descriptor callbacks.
/// Owner motion remains a frame snapshot. The collision list is only the seed:
/// every damaging F590 handler rebuilds it from live manager order before the
/// next physical slot, matching retail's repeated active-entity scans.
fn build_particle_callback_snapshot(
    cache: &v2k_game::resource_cache::ResourceCache,
    entities: &EntityManager,
    player_model_id: Option<usize>,
    player_craft: &PlayerCraft,
    particle_collision_error_reported: &mut HashSet<u32>,
) -> ParticleCallbackSnapshot {
    let owner_motions = entities
        .iter_all()
        .map(|entity| ParticleOwnerMotion {
            owner_id: entity.id,
            velocity: entity.velocity,
        })
        .collect();
    let collision_models = build_particle_collision_models(
        cache,
        entities,
        player_model_id,
        player_craft,
        particle_collision_error_reported,
    );
    ParticleCallbackSnapshot {
        owner_motions,
        collision_models,
    }
}

/// Rebuild the complete active-list-ordered collision view after an inline
/// entity-impact mutation. Both the frame-start snapshot and each same-pass
/// refresh use this path so attachment/proxy eligibility and model selection
/// cannot diverge.
fn build_particle_collision_models(
    cache: &v2k_game::resource_cache::ResourceCache,
    entities: &EntityManager,
    player_model_id: Option<usize>,
    player_craft: &PlayerCraft,
    particle_collision_error_reported: &mut HashSet<u32>,
) -> Vec<EntityCollisionModel> {
    let player_id = entities.player().map(|player| player.id);
    entities
        .iter_collidable()
        .filter_map(|entity| {
            match build_particle_collision_model(
                cache,
                entity,
                player_id,
                player_model_id,
                player_craft,
            ) {
                Ok(model) => model,
                Err(entity_id) => {
                    report_unresolved_particle_collision(
                        entity_id,
                        particle_collision_error_reported,
                    );
                    None
                }
            }
        })
        .collect()
}

fn report_unresolved_particle_collision(entity_id: u32, reported: &mut HashSet<u32>) {
    if reported.insert(entity_id) {
        eprintln!(
            "Particle collision excluded entity {entity_id}: actor eligibility is unresolved; traversal continues for other candidates"
        );
    }
}

fn build_particle_collision_model(
    cache: &v2k_game::resource_cache::ResourceCache,
    entity: &Entity,
    player_id: Option<u32>,
    player_model_id: Option<usize>,
    player_craft: &PlayerCraft,
) -> Result<Option<EntityCollisionModel>, u32> {
    if !entity.active || entity.kind == v2k_game::entity::EntityKind::Trigger {
        return Ok(None);
    }
    match v2k_game::entity_collision_state::particle_model_collision_eligible(&entity.collision) {
        RetailRuntimeValue::Known(true) => {}
        RetailRuntimeValue::Known(false) => return Ok(None),
        RetailRuntimeValue::Unresolved => return Err(entity.id),
    }
    let is_player = Some(entity.id) == player_id;
    let model = if is_player {
        entity.model_index.or(player_model_id)
    } else {
        entity.model_index
    };
    let Some((model_id, radius_raw)) = model.and_then(|id| {
        cache
            .global_model(id)
            .map(|model| (id, model.collision_radius_raw))
    }) else {
        return Ok(None);
    };
    Ok((radius_raw != 0).then_some(EntityCollisionModel {
        entity_id: entity.id,
        center_world: entity.position,
        radius_raw,
        model_id,
        orientation_world_from_model: if is_player {
            player_model_orientation(player_craft, entity)
        } else {
            non_player_entity_orientation(entity.heading, entity.physical_body_basis_q31())
        },
        anim_vars: if is_player {
            player_craft.anim_vars()
        } else {
            AnimVars::default()
        },
        state_flags_at_0x08: match entity.collision.state_flags_at_0x08.masked(0xFFFF_FFFF) {
            RetailRuntimeValue::Known(bits) => Some(bits),
            _ => None,
        },
        capability_flags_at_0x64: entity.capability_flags,
    }))
}

#[cfg(test)]
mod particle_collision_projection_tests {
    use super::*;
    use v2k_formats::models::{ModelCollection, ModelEntry, StreamStats};
    use v2k_game::entity::EntityKind;
    use v2k_game::level::LevelState;
    use v2k_game::resource_cache::ResourceCache;

    fn collision_model(collision_radius_raw: u16) -> ModelEntry {
        ModelEntry {
            index: 0,
            cmd_word_count: 0,
            extra_count: 0,
            flags: 0x40,
            slot_count: 0,
            face_val: 2,
            radius: 0,
            collision_radius_raw,
            collision_program: Vec::new(),
            records: Vec::new(),
            normal_pool: Vec::new(),
            cmd_words: Vec::new(),
            has_view_commands: false,
            vertices: Vec::new(),
            vertex_type_flags: Vec::new(),
            vertex_projection: Vec::new(),
            vertex_clip: Vec::new(),
            vertex_surface_origin: Vec::new(),
            triangles: Vec::new(),
            face_vertices: Vec::new(),
            normals: Vec::new(),
            face_cull: Vec::new(),
            face_materials: Vec::new(),
            face_uvs: Vec::new(),
            face_corner_normals: Vec::new(),
            face_shading: Vec::new(),
            shadow_triangles: Vec::new(),
            edges: Vec::new(),
            billboards: Vec::new(),
            instances: Vec::new(),
            painter_program: Vec::new(),
            name: None,
        }
    }

    fn collision_cache(radius_raw: u16) -> ResourceCache {
        ResourceCache::new(vec![LevelState {
            source_path: "particle-collision-projection-test".to_owned(),
            system_level: Some(2),
            fixup_data: None,
            fixup_code: None,
            strings: Vec::new(),
            sprites: None,
            params: None,
            display_modes: None,
            fog_gradient: None,
            color_palettes: None,
            models: Some(ModelCollection {
                sub_blocks: Vec::new(),
                all_entries: vec![collision_model(radius_raw)],
                stats: StreamStats::default(),
            }),
            anim_frames: None,
            terrain: None,
            anim_sound: None,
            collision: None,
            level: None,
            linkage: None,
        }])
    }

    #[test]
    fn non_player_collision_projection_rebuild_publishes_live_yaw() {
        let cache = collision_cache(315);
        let craft = PlayerCraft::new();
        let mut entity = Entity::unresolved_port_entity(17, EntityKind::Enemy, 17);
        entity.position = [12.0, -3.0, 9.5];
        entity.model_index = Some(0);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x9800, 0x8000);

        let before = build_particle_collision_model(&cache, &entity, None, None, &craft)
            .unwrap()
            .expect("nonzero-radius model must be collidable");
        entity.heading = std::f32::consts::FRAC_PI_2;
        let refreshed = build_particle_collision_model(&cache, &entity, None, None, &craft)
            .unwrap()
            .expect("live entity must remain collidable after its yaw changes");

        assert_eq!(refreshed.entity_id, entity.id);
        assert_eq!(refreshed.center_world, entity.position);
        assert_eq!(refreshed.model_id, 0);
        assert_eq!(refreshed.radius_raw, 315);
        assert_eq!(refreshed.anim_vars.registers, [0; 64]);
        assert_eq!(refreshed.anim_vars.dynamic, [0; 64]);
        assert_eq!(
            refreshed.orientation_world_from_model,
            non_player_model_orientation(entity.heading)
        );
        assert_ne!(
            refreshed.orientation_world_from_model, before.orientation_world_from_model,
            "the production projection builder must not retain frame-start yaw"
        );
    }

    #[test]
    fn particle_projection_reports_unknown_and_preserves_f980_alternate_admission() {
        let cache = collision_cache(315);
        let craft = PlayerCraft::new();
        let mut entity = Entity::unresolved_port_entity(77, EntityKind::Enemy, 77);
        entity.model_index = Some(0);
        assert!(matches!(
            build_particle_collision_model(&cache, &entity, None, None, &craft),
            Err(77)
        ));
        // The disabled gate needs no evidence for 800/1000 or surface state.
        entity.collision.state_flags_at_0x08.overwrite(0x8000, 0);
        assert!(matches!(
            build_particle_collision_model(&cache, &entity, None, None, &craft),
            Ok(None)
        ));
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x9800, 0x9000);
        assert!(matches!(
            build_particle_collision_model(&cache, &entity, None, None, &craft),
            Ok(None)
        ));
        entity.collision.state_flags_at_0x08.overwrite(0x800, 0x800);
        assert!(
            matches!(build_particle_collision_model(&cache, &entity, None, None, &craft), Ok(Some(model)) if model.entity_id == 77)
        );
    }

    #[test]
    fn non_player_collision_orientation_uses_each_live_retained_body_basis() {
        let first = Type9BodyBasis::from_angle_words(0x1234, -0x0800, 0x0400);
        let second = Type9BodyBasis::from_angle_words(0x1234, 0x1000, -0x2000);

        let first_orientation =
            non_player_entity_orientation(0.0, RetailRuntimeValue::Known(first));
        let refreshed_orientation =
            non_player_entity_orientation(0.0, RetailRuntimeValue::Known(second));

        assert_eq!(first_orientation, first.orientation_world_from_model());
        assert_eq!(refreshed_orientation, second.orientation_world_from_model());
        assert_ne!(refreshed_orientation, first_orientation);
        assert_ne!(
            refreshed_orientation,
            non_player_model_orientation(0.0),
            "a proven physical matrix must supersede the yaw compatibility basis"
        );
    }
}

/// Advance the acquired Targetter controller from the same selected weapon
/// descriptor and live-list order used by gameplay. Authored Section-8 model
/// programs remain the exact/near boundary: an unavailable program is passed
/// through as unresolved and cannot silently become a coarse target.
fn update_targetter_runtime(
    targetter: &mut TargetterRuntime,
    cache: &v2k_game::resource_cache::ResourceCache,
    entities: &EntityManager,
    player_craft: &PlayerCraft,
    weapon_inventory: &WeaponInventory,
    elapsed_micros: u32,
    retail_tick: u32,
) {
    if !targetter.is_active() {
        return;
    }
    let Some(player) = entities.player() else {
        return;
    };
    let selector = weapon_inventory.selected_descriptor().selector();
    let trajectory = fun_0044ea60(selector);
    let projectile_speed_raw = projectile_class_row(selector)
        .map(|row| row.speed_raw)
        .unwrap_or(0);
    let direction = player_primary_fire_direction(&player_craft, player);
    let velocity_raw =
        direction.map(|component| (component * projectile_speed_raw as f32).round() as i32);
    let candidates = entities
        .iter()
        .filter(|entity| entity.active)
        .map(|entity| TargetterCandidate {
            id: entity.id,
            entity_type: entity.entity_type,
            position_raw: entity.position_raw(),
            state_flags: entity.collision.state_flags_at_0x08,
            capability_flags: entity.capability_flags,
        })
        .collect::<Vec<_>>();
    let Some(terrain) = cache.terrain() else {
        return;
    };

    let _ = targetter.update(
        TargetterUpdateRequest {
            elapsed_micros,
            ray: TargetterRay {
                origin_raw: player.position_raw(),
                velocity_raw,
                trajectory,
            },
            candidates: &candidates,
        },
        |candidate, point_raw, radius_raw| {
            let Some(entity) = entities.iter().find(|entity| entity.id == candidate.id) else {
                return TargetterModelProbe::Unresolved;
            };
            let RetailRuntimeValue::Known(active_slot) = entity.collision.active_model_slot()
            else {
                return TargetterModelProbe::Unresolved;
            };
            let Some(model_id) = entity.model_in_slot(active_slot) else {
                return TargetterModelProbe::Unresolved;
            };
            let Some(model) = cache.global_model(model_id) else {
                return TargetterModelProbe::Unresolved;
            };
            let entity_raw = entity.position_raw();
            let delta_raw = std::array::from_fn(|axis| {
                f64::from(point_raw[axis].wrapping_sub(entity_raw[axis]))
            });
            let RetailRuntimeValue::Known(orientation) = entity_pair_to_world(entity) else {
                return TargetterModelProbe::Unresolved;
            };
            let orientation = orientation.map(|row| row.map(f64::from));
            match model.collide_sphere_raw_oriented(
                delta_raw,
                radius_raw,
                orientation,
                &entity.presentation_anim_vars(retail_tick),
                cache,
            ) {
                Ok(Some(_)) => TargetterModelProbe::Hit,
                Ok(None) => TargetterModelProbe::Miss,
                Err(_) => TargetterModelProbe::Unresolved,
            }
        },
        |x_raw, z_raw| targetter_terrain_height_raw(terrain, x_raw, z_raw),
    );
}

/// Player F70 publishes before EC60 changes the angle words. Every later
/// matrix consumer reads that retained basis; only legacy allocations with
/// no authenticated writer use the previous angle-derived fallback.
fn player_body_basis(craft: &PlayerCraft, player: &Entity) -> Type9BodyBasis {
    match player.physical_body_basis_q31() {
        RetailRuntimeValue::Known(basis) => basis,
        RetailRuntimeValue::Unresolved => {
            let [lateral, up, forward] = craft.retail_body_basis_q31(player.heading);
            Type9BodyBasis {
                lateral,
                up,
                forward,
            }
        }
    }
}

/// 424650 combines the retained entity up/forward columns with the barrel
/// joint. EC60's later Euler-word changes do not turn this frame's muzzle.
fn player_primary_fire_direction(craft: &PlayerCraft, player: &Entity) -> [f32; 3] {
    let body = player_model_orientation(craft, player);
    let (up, forward) = craft.gun_barrel_rad().sin_cos();
    std::array::from_fn(|row| body[row][1] * up + body[row][2] * forward)
}

fn player_model_orientation(craft: &PlayerCraft, player: &Entity) -> [[f32; 3]; 3] {
    match player.physical_body_basis_q31() {
        RetailRuntimeValue::Known(basis) => basis.orientation_world_from_model(),
        RetailRuntimeValue::Unresolved => craft.model_orientation(player.heading),
    }
}

#[cfg(test)]
mod player_environment_presentation_tests {
    use super::*;

    #[v2k_test_support::retail_test]
    fn retained_environment_basis_is_shared_by_presentation_collision_and_muzzle() {
        let root = v2k_test_support::retail_dir();
        assert!(root.join("PRELOAD.DAT").exists(), "retail corpus required");
        let mut session = v2k_game::session::GameSession::init(&root).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(46, 1).unwrap();
        let arrival = v2k_game::campaign_transition::CampaignArrivalCatalog::load(&session, 1)
            .unwrap()
            .direct_world_entry(46)
            .unwrap()
            .arrival();
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(id, &model_slots)| {
                session
                    .cache
                    .global_entity_type(id)
                    .map(EntityTypeRuntimeMetadata::from_section12)
                    .unwrap_or(EntityTypeRuntimeMetadata {
                        model_slots,
                        ..Default::default()
                    })
            })
            .collect();
        let extent = |id| session.cache.global_model(id).map(|model| model.radius);
        let mut fx = WorldFx::new();
        let mut manager = EntityManager::from_authored_world(
            v2k_game::entity::AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index: 34,
                type_metadata: &metadata,
                resources: v2k_game::entity::EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&extent),
                },
                player_arrival: Some(v2k_game::entity::AuthoredPlayerArrival {
                    position_raw: arrival.position_raw,
                    heading_raw: arrival.heading_raw,
                }),
                retail_tick: 4793,
            },
            &mut fx,
        )
        .unwrap();
        let mut craft = PlayerCraft::new();
        let force = craft.integrate_configured_micros_with_sensitivity(
            0,
            &MotionChannels::default(),
            manager.player_mut().unwrap(),
            0,
            5,
        );
        // Complete the ordinary player's first F70 boundary before modeling
        // the later EC60 angle-only writes.
        manager.update_with_player_surface_contacts(
            PlayerUpdateRequest {
                elapsed_micros: 0,
                terrain: session.cache.terrain().unwrap(),
                sea_level: None,
                retail_tick: 4793,
                water_response_selectors: [6; 8],
                attached_cargo_mass: 0,
                active_model_half_radius_raw: 0,
                active_model_collision_radius_raw: 0,
                frame: force,
                body_pitch_roll_raw: [0; 2],
                vtol_boost: VtolBoost::Inactive,
                vtol_height_policy: VtolHeightPolicy::RetailAttenuation,
            },
            false,
            |_, phase| match phase {
                PlayerSurfaceContactPhase::Terrain => PlayerSurfaceContactDispatch::Terrain(()),
                PlayerSurfaceContactPhase::Water { .. } => {
                    PlayerSurfaceContactDispatch::WaterComplete
                }
            },
        );
        let player = manager.player_mut().unwrap();
        let RetailRuntimeValue::Known(retained) = player.physical_body_basis_q31() else {
            panic!("native player has a completed F70 writer");
        };
        // Represent EC60's post-F70 Euler writes without replacing its matrix.
        craft.restore_body_angle_words([0x2345, -0x1234]);
        player.set_rotation_heading_pitch_roll_raw([player.heading_raw() as i16, 0x2345, -0x1234]);
        let expected = retained.orientation_world_from_model();
        assert_ne!(expected, craft.model_orientation(player.heading));
        assert_eq!(player_model_orientation(&craft, player), expected);
        assert_eq!(player_body_basis(&craft, player), retained);
        let model = build_particle_collision_model(
            &session.cache,
            player,
            Some(player.id),
            player.model_index,
            &craft,
        )
        .unwrap()
        .expect("native active player hull is collidable");
        assert_eq!(model.orientation_world_from_model, expected);
        assert_eq!(
            player_primary_fire_direction(&craft, player),
            expected.map(|row| row[2])
        );
    }
}

/// `FUN_00413D40` stores its basis as columns. Transposed into this port's
/// row-major `world = M * model` convention, pitch=roll=0 reduces to
/// `Ry(PI/2 - heading)`. Section-13 stores `heading` itself; both retail model
/// rendering and authored model collision consume this derived live matrix.
fn non_player_model_orientation(heading: f32) -> [[f32; 3]; 3] {
    orientation_from_ypr(std::f32::consts::FRAC_PI_2 - heading, 0.0, 0.0)
}

/// Consume a retained retail body matrix when an authenticated callback has
/// published one. Allocations without a proven matrix writer preserve the
/// existing yaw-derived presentation policy.
fn non_player_entity_orientation(
    heading: f32,
    body_basis_q31: RetailRuntimeValue<Type9BodyBasis>,
) -> [[f32; 3]; 3] {
    match body_basis_q31 {
        RetailRuntimeValue::Known(basis) => basis.orientation_world_from_model(),
        RetailRuntimeValue::Unresolved => non_player_model_orientation(heading),
    }
}

/// Cylindrical billboard basis for a flat actor whose authored face normal is
/// local +Z. Pitch is intentionally ignored so the villager remains upright.
///
/// Live-world cameras reflect view X so authored +X stays screen-right.
/// `Ry(atan2)` alone then puts implicit sprite U (local +X) on screen-left,
/// which moonwalks 8-direction cycles. Reflect local X at this submission
/// when that view is active. Do not flip parsed UVs.
fn camera_facing_entity_orientation(
    camera_position: [f32; 3],
    entity_position: [f32; 3],
    left_handed: bool,
) -> [[f32; 3]; 3] {
    let to_camera_x = camera_position[0] - entity_position[0];
    let to_camera_z = camera_position[2] - entity_position[2];
    let yaw = if to_camera_x.abs() + to_camera_z.abs() > 1.0e-6 {
        to_camera_x.atan2(to_camera_z)
    } else {
        0.0
    };
    let facing = orientation_from_ypr(yaw, 0.0, 0.0);
    if left_handed {
        mat3_mul(facing, [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
    } else {
        facing
    }
}

/// Rotate `v` by the row-major 3×3 `m` (`world = M · v`).
#[cfg(test)]
fn mat3_apply(m: [[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// Rotation matrix (row-major) about a unit-ish axis by `angle` (Rodrigues).
fn rotation_about_axis(axis: [f32; 3], angle: f32) -> [[f32; 3]; 3] {
    let len = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    if len < 1e-6 {
        return orientation_from_ypr(0.0, 0.0, 0.0);
    }
    let (x, y, z) = (axis[0] / len, axis[1] / len, axis[2] / len);
    let (s, c) = angle.sin_cos();
    let t = 1.0 - c;
    [
        [t * x * x + c, t * x * y - s * z, t * x * z + s * y],
        [t * x * y + s * z, t * y * y + c, t * y * z - s * x],
        [t * x * z - s * y, t * y * z + s * x, t * z * z + c],
    ]
}

/// Player-only child transform propagated through the shared linked hierarchy.
/// It elevates both side guns and spins matching blades. PLAYER4 callback word
/// 6 already rotates the complete fan assembly during a mode change.
struct PlayerCraftChildTransform<'a> {
    craft: &'a PlayerCraft,
}

/// Fan-blade spin axis in the blade's own local model space. `pl4engine`'s
/// materialized bbox is perfectly flat in Y (extent X≈112, Y=0, Z≈109 — the
/// disc lies in the local XZ plane), so its normal — the spin axis — is
/// model-local +Y. Verified via `cargo run -p v2k-formats --example fan_bbox`.
const FAN_SPIN_AXIS: [f32; 3] = [0.0, 1.0, 0.0];

impl ModelTreeChildTransform for PlayerCraftChildTransform<'_> {
    fn transform(
        &self,
        _child_id: usize,
        child_name: Option<&str>,
        authored_local: [[f32; 3]; 3],
    ) -> [[f32; 3]; 3] {
        let mut transformed = authored_local;

        // Blade spin is composed in the blade's own local frame so nested
        // `pl4engine` children rotate without a second surround spin.
        if PlayerCraft::spins_fan(child_name) {
            transformed = mat3_mul(
                transformed,
                rotation_about_axis(FAN_SPIN_AXIS, self.craft.spin_angle),
            );
        }

        transformed
    }

    fn native_frame_policy(
        &self,
        _child_id: usize,
        child_name: Option<&str>,
    ) -> v2k_game::model_tree::ModelTreeNativeChildPolicy {
        if PlayerCraft::spins_fan(child_name) {
            v2k_game::model_tree::ModelTreeNativeChildPolicy::Unowned
        } else {
            v2k_game::model_tree::ModelTreeNativeChildPolicy::Authored
        }
    }
}

/// Section 13 +0x56 supplies an optional global model after the background
/// colour. `FUN_0042F270` renders it twice with an identity basis: X tracks
/// half the camera X (distant parallax), Z tracks the camera exactly, and the
/// second copy is one half-world (`0x8000` in 8.8 coordinates) to the left.
fn draw_authored_sky_model(
    renderer: &mut dyn v2k_render::Renderer,
    cache: &v2k_game::resource_cache::ResourceCache,
    frame_request: Option<MainBaseAbortFrameRequest>,
    colors: &v2k_game::model_color::ModelMaterialCache,
    camera: &Camera,
    retail_tick: u32,
) {
    let Some(model_id) = v2k_game::sky::submitted_sky_model_id(cache, frame_request) else {
        return;
    };
    let x = camera.position[0] * 0.5;
    for draw_x in [x, x - 128.0] {
        ModelTreeRenderer::new_world(
            renderer,
            cache,
            colors,
            GAMEPLAY_MODEL_SCALE,
            None,
            retail_tick as i32,
        )
        .with_view(camera.into())
        .draw_static(
            model_id,
            orientation_from_ypr(0.0, 0.0, 0.0),
            [draw_x, 0.0, camera.position[2]],
            8,
            None,
        );
    }
}

/// Draw a prop model plus its inline instances (op 0x0E — e.g. the
/// `screenop` wrapper instances `screeno2` by global pool id). The menu prop
/// callback exposes the global 50 Hz tick on channel 0 and the fly clock on
/// channels 1/2; materializing the hierarchy here preserves authored wrapper
/// rotations instead of freezing them at callback value zero.
#[allow(clippy::too_many_arguments)]
fn draw_prop(
    renderer: &mut dyn v2k_render::Renderer,
    cache: &v2k_game::resource_cache::ResourceCache,
    colors: &v2k_game::model_color::ModelMaterialCache,
    camera: &Camera,
    model_id: usize,
    position: [f32; 3],
    rotation_y: f32,
    scale: f32,
    depth: u8,
    light: MenuSceneLight,
    depth_fade: ModelDepthFade,
    retail_tick: u32,
    fly_anim_raw: u16,
) {
    let orientation = menu_model_orientation(rotation_y);
    let mut vars = AnimVars::default();
    vars.dynamic[0] = (retail_tick & 0xFFFF) as i32;
    vars.dynamic[1] = fly_anim_raw as i32;
    vars.dynamic[2] = fly_anim_raw as i32;
    ModelTreeRenderer::new(
        renderer,
        cache,
        colors,
        scale,
        Some(light),
        ViewPinMode::Disabled,
    )
    .with_view(camera.into())
    .with_depth_fade(depth_fade)
    .with_near_clip(ModelNearClip::RetailFrontend)
    .draw_linked(model_id, orientation, position, depth, None, &vars);
}

/// Recursion depth for the player craft sub-model hierarchy
/// (player4 → enginesurround → tubegun …).
const PLAYER_DRAW_DEPTH: u8 = 3;
/// The shared renderer predates the gameplay RE and converts raw model vertices
/// with `/100` for the separately reconstructed menu scene. The original
/// gameplay vertex handlers do not contain that conversion: signed model
/// coordinates are multiplied by the entity's 1.31 matrix and added directly
/// to its 8.8 translation. Compensate at gameplay call sites so raw vertices
/// and instance attachment offsets become `/256` world units without changing
/// the menu projection contract.
const GAMEPLAY_MODEL_SCALE: f32 = 100.0 / 256.0;

/// World-load fog planes from Section13 +88 far distance and +8C fog width.
/// `42E920` shifts both dwords left eight; `451710` stores far-width and far.
/// The terrain's `330D0` 30-row cap does not participate in this publication.
#[cfg(test)]
fn gameplay_fog_planes(far_cells: u32, width_cells: u32) -> (f32, f32) {
    let planes = v2k_formats::levels::LevelFogPlanes::from_authored_cells(far_cells, width_cells);
    (
        planes.near_raw as f32 / 256.0,
        planes.far_raw as f32 / 256.0,
    )
}

/// Uniform draw scale for the player craft in the gameplay coordinate domain.
const PLAYER_DRAW_SCALE: f32 = GAMEPLAY_MODEL_SCALE;

/// The two selector-1 child instances used by `player4` for its side-mounted
/// chain guns.
const DEFAULT_PRIMARY_GUN_MODEL_NAME: &str = "pl4gatgun";
/// PLAYER4 callback word 4 value 4 replaces the chain-gun pair with these two
/// authored selector-2 tube/flare-gun children.
const UPGRADED_PRIMARY_GUN_MODEL_NAME: &str = "pl4tubegun";
/// Factory selector 0x12 chooses callback word 4 value 15 and the larger pair.
const RAPID_PRIMARY_GUN_MODEL_NAME: &str = "pl4biggatgun";
/// Inventory selectors 0x0E/0x0D/0x0C write callback word 4 values 6/7/8.
const PLASMA_RED_GUN_MODEL_NAME: &str = "pl4plasmared";
const PLASMA_GREEN_GUN_MODEL_NAME: &str = "pl4plasmagreen";
const PLASMA_BLUE_GUN_MODEL_NAME: &str = "pl4plasmablue";

/// Resolve PLAYER4 loadouts whose descriptors and A/B firing phase are proven.
/// `FUN_00424650` only dual-emits selectors 10/22/26; plasma and the recovered
/// machine-gun family all toggle on a nonzero descriptor `+0x14`, so they share
/// the alternating mount walk. Other callback-word-4 branches stay fail-closed.
fn primary_gun_model_name(callback_selector: i32) -> Option<&'static str> {
    match callback_selector {
        0 => Some(DEFAULT_PRIMARY_GUN_MODEL_NAME),
        4 => Some(UPGRADED_PRIMARY_GUN_MODEL_NAME),
        6 => Some(PLASMA_RED_GUN_MODEL_NAME),
        7 => Some(PLASMA_GREEN_GUN_MODEL_NAME),
        8 => Some(PLASMA_BLUE_GUN_MODEL_NAME),
        15 => Some(RAPID_PRIMARY_GUN_MODEL_NAME),
        _ => None,
    }
}

/// The proven primary mount meshes point along model-local +Z and elevate
/// about body-local +X.
const GUN_TIP_AXIS: [f32; 3] = [0.0, 0.0, 1.0];

/// Resolve the two visible launch bases from the live `player4` hierarchy.
///
/// Channel A/B follows authored instance order. If the expected pair cannot be
/// proven from the loaded data, retain an explicit unresolved state; the
/// weapon scheduler then presents at the already-proven center rather than
/// manufacturing a symmetric hull offset.
fn resolve_player_primary_gun_mounts(
    cache: &v2k_game::resource_cache::ResourceCache,
    model_id: usize,
    craft: &PlayerCraft,
    position: [f32; 3],
    heading: f32,
) -> PrimaryGunMounts {
    let Some(gun_model_name) = primary_gun_model_name(craft.weapon_selector()) else {
        return PrimaryGunMounts::Unresolved;
    };
    let vars = craft.anim_vars();
    let body = craft.model_orientation(heading);
    let child_transform = PlayerCraftChildTransform { craft };
    let tips = linked_model_named_tips(
        cache,
        ModelTreeNamedTipRequest {
            model_id,
            orientation: body,
            position,
            scale: PLAYER_DRAW_SCALE,
            depth: PLAYER_DRAW_DEPTH,
            linked: None,
            vars: &vars,
            child_transform: Some(&child_transform),
            model_name: gun_model_name,
            local_tip_axis: GUN_TIP_AXIS,
        },
    );
    let [gun_a, gun_b] = tips.as_slice() else {
        return PrimaryGunMounts::Unresolved;
    };
    let separation_squared = gun_a
        .origin_world
        .iter()
        .zip(gun_b.origin_world)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>();
    let direction_delta_squared = gun_a
        .direction_unit
        .iter()
        .zip(gun_b.direction_unit)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>();
    if separation_squared <= 1.0e-8 || direction_delta_squared > 1.0e-8 {
        return PrimaryGunMounts::Unresolved;
    }

    PrimaryGunMounts::Authored(PrimaryAuthoredGunMounts {
        gun_a: PrimaryLaunchBasis {
            origin_world: gun_a.origin_world,
            direction_unit: gun_a.direction_unit,
        },
        gun_b: PrimaryLaunchBasis {
            origin_world: gun_b.origin_world,
            direction_unit: gun_b.direction_unit,
        },
    })
}

/// Draw the player craft through the shared live linked hierarchy. The hull and
/// every op-0x0E descendant retain authored materials, UVs, billboards, linked
/// slots, and per-frame animation while the player-only fan transform is
/// layered over the relevant child mounts.
enum PlayerCraftDrawAnimation<'a> {
    Live(&'a PlayerCraft),
    Dying { control_output_7_raw: i32 },
}

struct PlayerCraftDrawFrame<'a> {
    cache: &'a v2k_game::resource_cache::ResourceCache,
    colors: &'a v2k_game::model_color::ModelMaterialCache,
    camera: &'a Camera,
    model_id: usize,
    animation: PlayerCraftDrawAnimation<'a>,
    position: [f32; 3],
    body: [[f32; 3]; 3],
    shade_shift: i32,
    retail_tick: u32,
    native_viewport: Option<v2k_game::native_model_frame::NativeWorldViewport>,
    actor_origin_raw: [i16; 3],
    body_basis: RetailRuntimeValue<Type9BodyBasis>,
    submissions: &'a mut v2k_game::model_tree::ModelTreeSubmissionBuffer,
}

fn draw_player_craft(
    renderer: &mut dyn v2k_render::Renderer,
    draw: PlayerCraftDrawFrame<'_>,
) -> EntityWeaponDrawOrigins {
    let PlayerCraftDrawFrame {
        cache,
        colors,
        camera,
        model_id,
        animation,
        position,
        body,
        shade_shift,
        retail_tick,
        native_viewport,
        actor_origin_raw,
        body_basis,
        submissions,
    } = draw;
    let (mut vars, child_transform) = match animation {
        PlayerCraftDrawAnimation::Live(craft) => {
            (craft.anim_vars(), Some(PlayerCraftChildTransform { craft }))
        }
        PlayerCraftDrawAnimation::Dying {
            control_output_7_raw,
        } => {
            let mut vars = AnimVars::default();
            vars.dynamic[7] = control_output_7_raw;
            (vars, None)
        }
    };
    // 40D320 selector zero returns the global 50-Hz word, independently of
    // component joints. Plasma child models shift it by ten for their glow.
    vars.dynamic[0] = i32::from(retail_tick as u16);

    // One shared retail body basis drives the visible hull, propulsion, muzzle,
    // cargo, and collision. In particular, spawn heading PI/2 is identity and
    // the yaw derivative is negative after transposing FUN_00413F70's legacy
    // lateral/up/forward layout; that is what makes the exact raw Left/Right
    // steering signs appear on their named screen sides.
    let native = native_viewport.and_then(|viewport| match body_basis {
        RetailRuntimeValue::Known(basis) => Some((
            v2k_game::native_model_frame::NativeModelFrame::from_actor(
                viewport,
                actor_origin_raw,
                [basis.lateral, basis.up, basis.forward],
            ),
            viewport,
        )),
        RetailRuntimeValue::Unresolved => None,
    });
    let descriptor = cache
        .global_entity_type(46)
        .filter(|record| record.sub_h_external_frame_descriptor().is_none())
        .and_then(|record| record.projectile_emitter_descriptor());
    let mut origins = EntityWeaponDrawOrigins {
        admitted: native.is_some() && descriptor.is_some(),
        ..Default::default()
    };
    let mut boundary = None;
    let mut stamp =
        |point: v2k_game::actor_emitter_external_frame::ActorEmitterExternalFramePoint| {
            if let Some(origin) = origins.points.get_mut(usize::from(point.emitter_index)) {
                *origin = Some(point.position_words);
            }
        };
    let mut tree = ModelTreeRenderer::new_world(
        renderer,
        cache,
        colors,
        PLAYER_DRAW_SCALE,
        None,
        retail_tick as i32,
    )
    .with_submission_buffer(submissions)
    .with_view(camera.into())
    .with_shade_shift(shade_shift);
    if let (Some((frame, viewport)), Some(descriptor), Some(root_model)) =
        (native, descriptor, cache.global_model(model_id))
    {
        tree = tree.with_actor_emitter_presentation(
            v2k_game::actor_emitter_external_frame::ActorEmitterModelPresentation {
                emitter:
                    v2k_game::actor_emitter_external_frame::ActorEmitterExternalFramePresentation {
                        descriptor,
                        root_model,
                        root_vars: &vars,
                        frame,
                        viewport,
                        stamp_origin: &mut stamp,
                        last_boundary: &mut boundary,
                        current_node_owned: true,
                        current_source_points_view: None,
                    },
                actor_origin_raw,
                actor_draw_origin: position,
            },
        );
    }
    tree.draw_linked_with_transform(
        model_id,
        body,
        position,
        PLAYER_DRAW_DEPTH,
        None,
        &vars,
        child_transform
            .as_ref()
            .map(|hook| hook as &dyn ModelTreeChildTransform),
    );
    drop(tree);
    drop(stamp);
    if let Some(boundary) = boundary {
        origins.blocked = true;
        report_actor_emitter_boundary(0, model_id, boundary);
    }
    origins
}

/// Ring math constants (MENU_SYSTEM.md §3D prop ring, FUN_0043B410):
/// position = (800·sin θ, −320·cos θ, y_in − 1200·cos θ) in raw units
/// (÷100 → port world units), y_in = 0xDAC = 3500, angular step between
/// items = 9200/65536 of a turn (≈50.5° — seven props span ≈354°, NOT a
/// uniform TAU/n), uniform authored scale, all props spin together.
const RING_STEP_RAD: f32 = 9200.0 / 65536.0 * std::f32::consts::TAU;
const RING_Y_IN: f32 = 3500.0;
const MENU_CAMERA_NEAR: f32 = 0.05;
const MENU_CAMERA_FAR: f32 = 200.0;
/// `FUN_0040ED10` starts the menu chase camera `0x800` raw units behind the
/// co-located class-1 anchor. Its separate `local_20 = 0xC00` is only the
/// direction-vector basis before normalization (and also happens to match the
/// flame billboard's sort depth); using it as Klaus's view depth made the
/// creature exactly 2/3 of its original projected size.
const MENU_BACKDROP_VIEW_DEPTH_RAW: f32 = 0x800 as f32;
const MENU_BILLBOARD_VIEW_DEPTH_RAW: f32 = 0xC00 as f32;
const MENU_BACKDROP_MODEL_ID: usize = 1;
const MENU_BACKDROP_DEPTH: u8 = 8;
/// Persistent frontend Klaus/world context at `DAT_004CA838 +0x50..+0x58`.
/// Ring props use the ordinary `(73,73,-73)` model-light vector instead.
const MENU_BACKDROP_LIGHT_DIRECTION_RAW: [i32; 3] = [-100, 50, -50];
const MENU_PROP_ROW_LIVE_NEAR_OFFSET_RAW: f32 = 0x900 as f32;
const MENU_PROP_ROW_FADE_CAP_RAW: f32 = 0x200 as f32;
const MENU_PROP_ROW_NEUTRAL_NEAR_RAW: f32 = 0xC00 as f32;
const MENU_SCREEN_FADE_RAW: f32 = 0x1E00 as f32;
/// `FUN_0042B230` installs cumulative Section-7 entry 55 at world-context
/// `+0x7C`. Both retail menu tiers author RGB555 `0x7C00` there.
const MENU_DEPTH_FADE_COLOR_INDEX: usize = 55;
const MENU_DEPTH_FADE_COLOR_FALLBACK: [f32; 3] = [248.0 / 255.0, 0.0, 0.0];
/// Pause overlays do not run the frontend initializer that installs S7[55].
/// Retain their existing neutral terminal color until their live world-context
/// color is recovered independently.
const MENU_PAUSE_DEPTH_FADE_COLOR: [f32; 3] = [0.0; 3];
/// Retail's frontend model renderer presents local +X opposite the port's
/// canonical model basis. Keep that conversion at the two menu hierarchy
/// roots: translations remain in traced ring/camera space, while geometry,
/// child anchors, and directional artwork are reflected together.
const MENU_MODEL_LOCAL_X_BASIS: [[f32; 3]; 3] =
    [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
/// `optionsh`'s two mirrored `anim` children author their decorated front on
/// the opposite local-Z side from the ordinary ring props. This half-turn is
/// the port-basis equivalent of the retail menu renderer's angle-zero basis.
const MENU_OPTIONS_PANEL_ROTATION_Y: f32 = std::f32::consts::PI;

fn menu_model_orientation(rotation_y: f32) -> [[f32; 3]; 3] {
    mat3_mul(
        orientation_from_ypr(rotation_y, 0.0, 0.0),
        MENU_MODEL_LOCAL_X_BASIS,
    )
}

fn menu_depth_fade_color(cache: &v2k_game::resource_cache::ResourceCache) -> [f32; 3] {
    cache
        .global_palette_entry(MENU_DEPTH_FADE_COLOR_INDEX)
        .map(|entry| {
            [
                f32::from(entry.r) / 255.0,
                f32::from(entry.g) / 255.0,
                f32::from(entry.b) / 255.0,
            ]
        })
        .unwrap_or(MENU_DEPTH_FADE_COLOR_FALLBACK)
}

/// Convert retail's 8.8-style frontend depth words into the model renderer's
/// `/100` view-space convention. This is deliberately submission-local rather
/// than scene fog; the terminal colour comes from world context `+0x7C`.
fn menu_depth_fade_from_raw(near_raw: f32, far_raw: f32, color: [f32; 3]) -> ModelDepthFade {
    ModelDepthFade::FrontendFixedPointLinear {
        near_raw: near_raw.round() as i32,
        far_raw: far_raw.round() as i32,
        color,
    }
}

/// The handoff has already switched from the frontend to game context
/// 4D04E8. 4515E0 installs these planes; 453C20's closing minima (F00/1400)
/// preserve them. 453570 gets its terminal colour through 2E8D0, whose
/// runtime level+B0 is initialized from Section 13+54 by 2E570.
fn klaus_handoff_depth_fade(cache: &v2k_game::resource_cache::ResourceCache) -> ModelDepthFade {
    let color = cache
        .level_desc()
        .and_then(|level| cache.global_palette_entry(usize::from(level.sky_color_index)))
        .map(|entry| {
            [
                f32::from(entry.r) / 255.0,
                f32::from(entry.g) / 255.0,
                f32::from(entry.b) / 255.0,
            ]
        })
        .unwrap_or([0.0; 3]);
    menu_depth_fade_from_raw(0x1E00 as f32, 0x2800 as f32, color)
}

/// Flag-`0x02` row policy in `FUN_0043AA40`: reset the local band from the
/// post-pin live world near plane before every item, then shift by that item's
/// root Z in `FUN_0043B410`.
fn menu_prop_row_depth_fade(
    live_near_raw: f32,
    root_depth_raw: f32,
    color: [f32; 3],
) -> ModelDepthFade {
    let near_raw = (live_near_raw - MENU_PROP_ROW_LIVE_NEAR_OFFSET_RAW)
        .min(MENU_PROP_ROW_FADE_CAP_RAW)
        + root_depth_raw;
    let far_raw = MENU_PROP_ROW_FADE_CAP_RAW + root_depth_raw;
    menu_depth_fade_from_raw(near_raw, far_raw, color)
}

/// Add a signed packed-angle displacement with the original u16 wraparound.
fn add_packed_angle(base: i32, delta: i32) -> i32 {
    (base + delta).rem_euclid(0x1_0000)
}

/// Original packed 50-Hz sine phase (`g_default_param * rate + phase`).
fn menu_idle_wave(retail_tick: u32, rate: i32, phase: i32, amplitude: i32) -> i32 {
    let angle = retail_tick
        .wrapping_mul(rate as u32)
        .wrapping_add(phase as u32)
        & 0xFFFF;
    packed_sine(angle as i32, amplitude)
}

fn q31_mul(a: i32, b: i32) -> i32 {
    ((a as i64 * b as i64) >> 31) as i32
}

fn packed_sine(angle: i32, amplitude: i32) -> i32 {
    let sine_q31 = retail_sine_q15(angle as u32).wrapping_mul(0x1_0001);
    q31_mul(sine_q31, amplitude)
}

fn cubic_pose_envelope(phase: i32, scale: i32, strength: i32) -> i32 {
    let base = 0x7FFF - phase / 2;
    let a = ((base * scale) >> 15) as i16 as i32;
    let b = ((a * base) >> 15) as i16 as i32;
    let c = ((b * base) >> 15) as i16 as i32;
    c * strength / 100
}

fn folded_pose_sine(phase: i32, offset: i32, amplitude: i32) -> i32 {
    let angle = phase * 3 / 2 + offset;
    // This is the signed clamp/fold emitted by FUN_0042C710 before its table
    // lookup: negative attack phases use the quarter-turn endpoint.
    let folded = if angle < 0 { 0x4000 } else { angle + 0x4000 };
    packed_sine(folded, amplitude)
}

fn coupled_pose_slope(envelope: i32, phase: i32, repeated_magic: u16) -> i32 {
    let magic = ((repeated_magic as u32) << 16 | repeated_magic as u32) as i32;
    let remainder = envelope - q31_mul(magic.wrapping_neg(), envelope);
    (((remainder as i64 * (phase / 2) as i64) >> 15) as i32) as i16 as i32
}

/// FUN_0042C710's primary click-driven pose. Retail still evaluates phase
/// zero through its integer sine/envelope path, producing several deliberate
/// one-LSB corrections; command 4 then drives one packed-angle revolution.
fn apply_primary_backdrop_pose(vars: &mut AnimVars, phase: u16) {
    let phase = phase as i32;

    let env_5 = cubic_pose_envelope(phase, 0x5000, 100);
    let wave_5 = packed_sine(phase * 3 / 2 + 0x4000, env_5);
    let slope_5 = coupled_pose_slope(env_5, phase, 0x7FFF);
    vars.dynamic[3] = add_packed_angle(vars.dynamic[3], -wave_5 - slope_5 + env_5);
    let shared = (env_5 - wave_5 - slope_5) / 2;
    vars.dynamic[10] = add_packed_angle(vars.dynamic[10], -shared);
    vars.dynamic[8] = add_packed_angle(vars.dynamic[8], -shared);

    let env_6 = cubic_pose_envelope(phase, 0x6000, 100);
    let wave_6 = folded_pose_sine(phase, -0x1800, env_6);
    let slope_6 = coupled_pose_slope(env_6, phase, 0x7A79);
    vars.dynamic[4] = add_packed_angle(vars.dynamic[4], -wave_6 - slope_6 + env_6);

    let env_7 = cubic_pose_envelope(phase, 0x7000, 100);
    let wave_7 = folded_pose_sine(phase, -0x3000, env_7);
    let slope_7 = coupled_pose_slope(env_7, phase, 0x6A66);
    vars.dynamic[5] = add_packed_angle(vars.dynamic[5], -wave_7 - slope_7 + env_7);
    vars.dynamic[9] = add_packed_angle(vars.dynamic[9], packed_sine(phase.min(0x7FFF), 0x1000));

    let wave_11 = folded_pose_sine(phase, -0x4800, env_7);
    let slope_11 = coupled_pose_slope(env_7, phase, 0x512A);
    vars.dynamic[11] = add_packed_angle(vars.dynamic[11], -wave_11 - slope_11 + env_7);
}

/// Secondary FUN_0042C710 call used by the occasional random twitch. It adds
/// the same signed displacement to animation channels 6 and 7.
fn backdrop_twitch_delta(phase: u16, strength: i32) -> i32 {
    if phase == 0 {
        return 0;
    }
    let phase = phase as i32;
    let envelope = cubic_pose_envelope(phase, 0x7000, strength);
    let wave = folded_pose_sine(phase, -0x3000, envelope);
    let slope = coupled_pose_slope(envelope, phase, 0x6A66);
    (envelope - wave - slope) / 4
}

/// Klaus's authored menu pose (`FUN_0042C660` followed by the steady-idle
/// branch of `FUN_0042C710`). The callback reads these as packed u16 angles.
/// The staggered 2.02–5.24 s waves are what keep the nested wing/bone chain
/// alive while the ring is idle; a reset-only pose looks unnaturally frozen.
fn menu_backdrop_anim_vars(retail_tick: u32, state: KlausBackdropAnimState) -> AnimVars {
    let mut vars = AnimVars::default();
    // C710 publishes private +0x10 to selector 1. Klaus's authored model
    // stream distributes this morph across the body, head and jaw mounts.
    vars.dynamic[1] = i32::from(state.morph_progress);
    // C090 copies DAT_004DB218 before invoking C610/C710. It is one in the
    // interactive frontend and zero after game start commits. The separate
    // spin jaw hinge separately consumes dynamic channel 9.
    vars.dynamic[2] = i32::from(state.model_state);
    vars.dynamic[3] = add_packed_angle(0xF000, menu_idle_wave(retail_tick, 500, 0, 0x800));
    vars.dynamic[4] = add_packed_angle(0xF800, menu_idle_wave(retail_tick, 500, -0x4800, 0xC00));
    vars.dynamic[5] = add_packed_angle(0xE800, menu_idle_wave(retail_tick, 500, -0x7800, 0xC00));
    vars.dynamic[6] = add_packed_angle(
        0x0000,
        menu_idle_wave(
            retail_tick,
            0xFA,
            0x4000,
            i32::from(state.sway_amplitude_raw),
        ),
    );
    vars.dynamic[7] = 0x0800;
    vars.dynamic[8] = add_packed_angle(0x0000, menu_idle_wave(retail_tick, 0x28A, 0, 0x800));
    vars.dynamic[9] = 0x0000;
    vars.dynamic[10] = add_packed_angle(0xF800, menu_idle_wave(retail_tick, 0x1C2, 0, 0x400));
    vars.dynamic[11] = add_packed_angle(0xF000, menu_idle_wave(retail_tick, 500, -0xA800, 0x1000));
    apply_primary_backdrop_pose(&mut vars, state.primary_phase);
    let twitch = backdrop_twitch_delta(state.twitch_phase, state.twitch_strength);
    vars.dynamic[6] = add_packed_angle(vars.dynamic[6], twitch);
    vars.dynamic[7] = add_packed_angle(vars.dynamic[7], twitch);
    vars
}

#[derive(Clone, Copy)]
enum MenuBackdropPresentation {
    Frontend,
    Handoff,
}

#[derive(Clone, Copy)]
struct MenuBackdropFrame {
    presentation: MenuBackdropPresentation,
    retail_tick: u32,
    anim_state: KlausBackdropAnimState,
    view_depth_raw: f32,
    projection_y_offset: f32,
    depth_fade: ModelDepthFade,
}

fn draw_menu_backdrop(
    renderer: &mut dyn v2k_render::Renderer,
    cache: &v2k_game::resource_cache::ResourceCache,
    colors: &v2k_game::model_color::ModelMaterialCache,
    frame: MenuBackdropFrame,
    light: MenuSceneLight,
) {
    if !frame.anim_state.visible || cache.global_model(MENU_BACKDROP_MODEL_ID).is_none() {
        return;
    }
    let vars = menu_backdrop_anim_vars(frame.retail_tick, frame.anim_state);
    // The Intro Sequence moves persistent Klaus vertically as it recedes. The
    // original chase-camera composition therefore shifts the model
    // by the same projected Y delta as the flame billboard. The port already
    // applied that delta to the billboard; omitting it here left Klaus centered
    // behind the submenu panel with only its wing tips visible.
    let world_y = menu_world_y_for_projection_center(
        0.0,
        frame.view_depth_raw,
        120.0 + frame.projection_y_offset,
        240.0,
    );
    let viewport = renderer.viewport_size();
    let mut tree = ModelTreeRenderer::new(
        renderer,
        cache,
        colors,
        1.0,
        Some(light),
        ViewPinMode::Disabled,
    )
    .with_root_link_policy(ModelTreeRootLinkPolicy::KlausMatte { viewport })
    .with_near_clip(ModelNearClip::RetailFrontend)
    .with_light_direction_raw(MENU_BACKDROP_LIGHT_DIRECTION_RAW)
    .with_depth_fade(frame.depth_fade)
    .with_painter_view(
        ModelTreeView {
            position: [0.0; 3],
            forward: [0.0, 0.0, -1.0],
        },
        match frame.presentation {
            // Model 1's outer 0x15 group encloses ALL Klaus, including his
            // head. Its idle key is private view Z + slot 12's 0xA00, behind
            // the emblem's 0xC00. Descendant geometric Z cannot mask it.
            MenuBackdropPresentation::Frontend => ModelTreePainterComposition::OpaqueSceneGroup,
            MenuBackdropPresentation::Handoff => ModelTreePainterComposition::Isolated,
        },
    );
    tree.draw_linked(
        MENU_BACKDROP_MODEL_ID,
        // The entity and chase-camera headings do not cancel to identity in
        // the original view: Klaus faces the camera (head forward, tail
        // behind). The model stream already assembles its local hierarchy;
        // this half-turn supplies the missing camera-relative root heading.
        menu_model_orientation(std::f32::consts::PI),
        [0.0, world_y, -frame.view_depth_raw / 100.0],
        MENU_BACKDROP_DEPTH,
        None,
        &vars,
    );
}

/// Render the main menu as the original 3D prop ring: the seven props
/// (player4 vehicle, slopt, screenop, sfxopt, psjoypad, optexit, multipc)
/// on the ring; the selected one sits at θ=0 — closest and lowest —
/// distinguished only by position and its label (yellow font at pt12).
#[allow(clippy::too_many_arguments)]
fn render_ring_3d(
    renderer: &mut dyn v2k_render::Renderer,
    shell: &MenuShell,
    menu: &v2k_game::game_state::MenuState,
    system_strings: &[String],
    menu_resources: &v2k_game::menu::MenuResources,
    fonts: Option<&v2k_game::menu_text::MenuFonts>,
    cache: &v2k_game::resource_cache::ResourceCache,
    colors: &v2k_game::model_color::ModelMaterialCache,
    billboard: &BillboardRenderFrame,
    retail_tick: u32,
) {
    renderer.clear(0.01, 0.01, 0.03);
    let (vw, vh) = renderer.viewport_size();
    let ui_request = UiMappingRequest {
        viewport: [vw, vh],
        authored_canvas: fonts
            .map(|f| [f.virtual_w as u32, f.virtual_h as u32])
            .unwrap_or([320, 240]),
        policy: renderer.ui_submission_policy(),
    };

    renderer.set_camera(&menu_camera(vw as f32 / vh as f32));

    // Backdrop creature (`klaus`, the class-0 menu entity), assembled in the
    // original head-toward-camera orientation by `draw_menu_backdrop`.
    let backdrop_depth = shell
        .menu_backdrop_depth_raw()
        .max(MENU_BACKDROP_VIEW_DEPTH_RAW);
    draw_menu_backdrop(
        renderer,
        cache,
        colors,
        MenuBackdropFrame {
            presentation: MenuBackdropPresentation::Frontend,
            retail_tick,
            anim_state: shell.menu_backdrop_anim_state(),
            view_depth_raw: backdrop_depth,
            projection_y_offset: shell.menu_billboard_y_offset(),
            depth_fade: menu_depth_fade_from_raw(
                billboard.scene_depth_fade_near_raw,
                billboard.scene_depth_fade_far_raw,
                menu_depth_fade_color(cache),
            ),
        },
        MenuSceneLight::NEUTRAL,
    );

    // Fly-transition depth offsets (raw units on the ring's y_in axis).
    let (sel_off, other_off) = v2k_game::game_state::fly_offsets(shell.phase);
    let virtual_h = fonts.map(|f| f.virtual_h).unwrap_or(240.0);
    let projection_y = fonts
        .and_then(|f| f.layout_point(0))
        .or_else(|| v2k_game::menu_text::layout_point(cache, 0))
        .map(|(_, y)| y)
        .unwrap_or(virtual_h * 155.0 / 240.0);
    let prop_camera = menu_camera_for_projection_y(projection_y, ui_request);
    renderer.set_camera(&prop_camera);

    let sel = menu.selected as f32;
    let depth_fade_color = menu_depth_fade_color(cache);
    for (i, item) in menu.items.iter().enumerate() {
        let Some(model_id) = item.model_id else {
            continue;
        };
        // θ = (idx − sel + scroll/200) · step; selected lands at θ=0 (front).
        let theta = (i as f32 - sel + menu.scroll / 200.0) * RING_STEP_RAD;
        let selected = i == menu.selected;
        let y_in = RING_Y_IN + if selected { sel_off } else { other_off };
        // Raw units → port world units (÷100); camera looks down −Z.
        let local_y_raw = -320.0 * theta.cos();
        let depth_raw = y_in - 1200.0 * theta.cos();
        let pos = [
            800.0 * theta.sin() / 100.0,
            local_y_raw / 100.0,
            -depth_raw / 100.0,
        ];
        // ALL props spin about one axis at 3.436 s/rev (DAT_004DCEB0);
        // authored sizes are intentional — no per-prop normalization, no
        // selected-prop scale boost.
        draw_prop(
            renderer,
            cache,
            colors,
            &prop_camera,
            model_id as usize,
            pos,
            menu.spin,
            1.0,
            2,
            MenuSceneLight::NEUTRAL,
            menu_prop_row_depth_fade(
                billboard.row_depth_fade_near_raw,
                depth_raw,
                depth_fade_color,
            ),
            retail_tick,
            v2k_game::game_state::fly_model_anim_raw(shell.phase),
        );
    }

    // The authored 0x10 software material is additive (source + destination);
    // the alternate callback maps it to D3D ONE/ONE. Submit after opaque models
    // and depth-test at sort key 0xC00: front props mask the emblem while
    // its glow adds over Klaus's entire outer group and rear ring props.
    draw_billboard(
        renderer,
        cache,
        menu_resources,
        billboard,
        shell.menu_billboard_pose(),
    );

    // Selected prop's label, centered at layout pt12 (160,220), yellow
    // (selected) font — only when no transition is running.
    if shell.phase == v2k_game::game_state::MenuPhase::Idle {
        let label = &menu.items[menu.selected].label;
        if let Some(f) = fonts {
            let (px, py) = f
                .layout_point(12)
                .or_else(|| v2k_game::menu_text::layout_point(cache, 12))
                .unwrap_or((f.virtual_w * 0.5, f.virtual_h - 60.0));
            let mapping = UiMapping::new(ui_request);
            draw_menu_text(
                renderer,
                f.font(true),
                label,
                [px, py],
                true,
                usize::MAX,
                mapping,
            );
        } else {
            let (label_rgba, label_w, label_h) = rasterize_text_colored(label, 255, 255, 0);
            let label_x = (vw as i32 - label_w as i32) / 2;
            renderer.draw_sprite(&label_rgba, label_w, label_h, label_x, vh as i32 * 11 / 12);
        }
    }

    if shell.frontend_branding_visible() {
        draw_menu_branding(renderer, system_strings, menu_resources, vw, vh);
    }
}

/// Map completed retail pixel geometry into the port's menu viewport. Integer
/// C090/D030 operations precede this uniform presentation scale and margins.
fn menu_billboard_viewport_rect(
    mapping: UiMapping,
    layout: &MenuBillboardLayout,
    pose: MenuBillboardPose,
    frame_size: [u16; 2],
) -> Option<MenuBillboardRect> {
    let rect = layout.rect(pose, frame_size)?;
    let UiMapping {
        scale: s,
        offset_x: ox,
        offset_y: oy,
    } = mapping;
    let width = (rect.width as f32 * s).round() as u32;
    let height = (rect.height as f32 * s).round() as u32;
    if width == 0 || height == 0 {
        return None;
    }
    Some(MenuBillboardRect {
        x: ox + (rect.x as f32 * s).round() as i32,
        y: oy + (rect.y as f32 * s).round() as i32,
        width,
        height,
    })
}

/// Draw using C090's reference-sprite origin and D030's selected-sprite size,
/// retaining the existing additive queue depth.
fn draw_billboard(
    renderer: &mut dyn v2k_render::Renderer,
    cache: &v2k_game::resource_cache::ResourceCache,
    menu_resources: &v2k_game::menu::MenuResources,
    billboard: &BillboardRenderFrame,
    pose: MenuBillboardPose,
) {
    let gid = billboard.sprite_id;
    let idx = (gid as usize).saturating_sub(1294);
    let Some(frame) = menu_resources.flame_frames.get(idx) else {
        return;
    };
    if frame.width == 0 || frame.height == 0 {
        return;
    }
    let (vw, vh) = renderer.viewport_size();
    let Some(layout) = MenuBillboardLayout::from_cache(cache) else {
        return;
    };
    let Some((_, entry)) = cache.global_sprite(gid) else {
        return;
    };
    let frame_size = [entry.flags as u16, (entry.flags >> 16) as u16];
    let Some(rect) = menu_billboard_viewport_rect(
        UiMapping::new(UiMappingRequest {
            viewport: [vw, vh],
            authored_canvas: layout.framebuffer.map(|dimension| dimension as u32),
            policy: renderer.ui_submission_policy(),
        }),
        &layout,
        pose,
        frame_size,
    ) else {
        return;
    };
    let scaled = scale_rgba(
        &frame.rgba,
        frame.width,
        frame.height,
        rect.width,
        rect.height,
    );
    renderer.draw_additive_sprite_at_depth(
        &scaled,
        rect.width,
        rect.height,
        rect.x,
        rect.y,
        MENU_BILLBOARD_VIEW_DEPTH_RAW / 100.0,
        MENU_CAMERA_NEAR,
        MENU_CAMERA_FAR,
    );
}

/// Copyright banner + studio logos shared by the menu screens.
fn draw_menu_branding(
    renderer: &mut dyn v2k_render::Renderer,
    system_strings: &[String],
    menu_resources: &v2k_game::menu::MenuResources,
    vw: u32,
    vh: u32,
) {
    use v2k_game::game_state::menu_strings;

    // FUN_0042B040 uses sprite dimensions in the selected framebuffer. The
    // renderer supplies the additional uniform scale only for Native mode on
    // a larger desktop; classic modes scale the whole logical framebuffer.
    let authored_scale = renderer.authored_pixel_scale().max(0.01);

    // Preserve retail's left-logo, centred-copyright, right-logo submission order.
    if let Some(ref logo) = menu_resources.frontier_logo {
        // logo_scale (from the detail tier): 1 = blit the high-res atlas native
        // (crisp); 3 = enlarge the low-res variant-0 sprite to keep the footprint
        // (the old blurry path, used only when High falls back to the embedded set).
        let s = menu_resources.logo_scale as f32 * authored_scale;
        let (w, h) = (
            (logo.width as f32 * s).round().max(1.0) as u32,
            (logo.height as f32 * s).round().max(1.0) as u32,
        );
        let x = 0;
        let y = vh as i32 - h as i32;
        if w == logo.width && h == logo.height {
            renderer.draw_sprite(&logo.rgba, w, h, x, y);
        } else {
            let scaled = scale_rgba(&logo.rgba, logo.width, logo.height, w, h);
            renderer.draw_sprite(&scaled, w, h, x, y);
        }
    }

    if let Some(ref banner) = menu_resources.copyright_banner {
        let copy_w = (banner.width as f32 * authored_scale).round().max(1.0) as u32;
        let copy_h = (banner.height as f32 * authored_scale).round().max(1.0) as u32;
        let copy_x = (vw as i32 - copy_w as i32) / 2;
        let copy_y = vh as i32 - copy_h as i32;
        if copy_w == banner.width && copy_h == banner.height {
            renderer.draw_sprite(&banner.rgba, copy_w, copy_h, copy_x, copy_y);
        } else {
            let scaled = scale_rgba(&banner.rgba, banner.width, banner.height, copy_w, copy_h);
            renderer.draw_sprite(&scaled, copy_w, copy_h, copy_x, copy_y);
        }
    } else {
        let copyright = system_strings
            .get(menu_strings::COPYRIGHT)
            .map(|s| s.as_str())
            .unwrap_or("V2000 Frontier Developments 1998");
        let (copy_rgba, source_w, source_h) = rasterize_text(copyright);
        let copy_w = (source_w as f32 * authored_scale).round().max(1.0) as u32;
        let copy_h = (source_h as f32 * authored_scale).round().max(1.0) as u32;
        let copy_x = (vw as i32 - copy_w as i32) / 2;
        let copy_y = vh as i32 - copy_h as i32;
        if copy_w == source_w && copy_h == source_h {
            renderer.draw_sprite(&copy_rgba, copy_w, copy_h, copy_x, copy_y);
        } else {
            let scaled = scale_rgba(&copy_rgba, source_w, source_h, copy_w, copy_h);
            renderer.draw_sprite(&scaled, copy_w, copy_h, copy_x, copy_y);
        }
    }

    if let Some(ref logo) = menu_resources.publisher_logo {
        let s = menu_resources.logo_scale as f32 * authored_scale;
        let (w, h) = (
            (logo.width as f32 * s).round().max(1.0) as u32,
            (logo.height as f32 * s).round().max(1.0) as u32,
        );
        let x = vw as i32 - w as i32;
        let y = vh as i32 - h as i32;
        if w == logo.width && h == logo.height {
            renderer.draw_sprite(&logo.rgba, w, h, x, y);
        } else {
            let scaled = scale_rgba(&logo.rgba, logo.width, logo.height, w, h);
            renderer.draw_sprite(&scaled, w, h, x, y);
        }
    }
}

/// Draw the authored synchronous-load card before entering the blocking OVL
/// reader.
///
/// Retail `FUN_0042B040` draws the three bottom branding sprites first, then
/// uses the normal (green) system font at level-2 layout point 12. Its caller
/// gates the text on the even half of `(DAT_004FED60 / 20) & 1`; the card
/// itself remains visible while the loader blocks. The port has no proven
/// retry loop, so this path intentionally requests system string 34
/// (`Loading`) only.
fn render_loading_card(
    renderer: &mut dyn v2k_render::Renderer,
    system_strings: &[String],
    menu_resources: &v2k_game::menu::MenuResources,
    fonts: Option<&v2k_game::menu_text::MenuFonts>,
    cache: &v2k_game::resource_cache::ResourceCache,
    retail_tick: u32,
) {
    use v2k_game::game_state::menu_strings;

    renderer.begin_scene(RenderScene::Menu);
    renderer.clear(0.0, 0.0, 0.0);

    let (vw, vh) = renderer.viewport_size();
    draw_menu_branding(renderer, system_strings, menu_resources, vw, vh);

    if loading_text_visible(retail_tick) {
        let text = system_strings
            .get(menu_strings::LOADING)
            .map(String::as_str)
            .unwrap_or("Loading");
        if let Some(fonts) = fonts {
            let (x, y) = fonts
                .layout_point(12)
                .or_else(|| v2k_game::menu_text::layout_point(cache, 12))
                .unwrap_or((fonts.virtual_w * 0.5, fonts.virtual_h * 11.0 / 12.0));
            let mapping = UiMapping::new(UiMappingRequest {
                viewport: [vw, vh],
                authored_canvas: [fonts.virtual_w as u32, fonts.virtual_h as u32],
                policy: renderer.ui_submission_policy(),
            });
            draw_menu_text(
                renderer,
                fonts.font(false),
                text,
                [x, y],
                true,
                usize::MAX,
                mapping,
            );
        } else {
            // Missing menu OVLs are already an explicitly degraded install.
            // Keep the status legible without changing the proven path above.
            let (rgba, width, height) = rasterize_text_colored(text, 60, 200, 60);
            renderer.draw_sprite(
                &rgba,
                width,
                height,
                (vw as i32 - width as i32) / 2,
                vh as i32 * 11 / 12 - height as i32,
            );
        }
    }

    renderer.present();
}

/// The loading label is submitted during the even 20-tick half-period.
/// `FUN_0042AF00` preserves the retail signed-division/sign-fold sequence so
/// the cadence also remains defined after the wrapping clock crosses bit 31.
fn loading_text_visible(retail_tick: u32) -> bool {
    ((retail_tick as i32) / 20).unsigned_abs() & 1 == 0
}

/// 2D sprite-carousel fallback for the main menu (software backend or
/// missing prop models).
fn render_carousel(
    renderer: &mut dyn v2k_render::Renderer,
    shell: &MenuShell,
    menu: &v2k_game::game_state::MenuState,
    system_strings: &[String],
    menu_resources: &v2k_game::menu::MenuResources,
) {
    renderer.clear(0.02, 0.02, 0.08);

    let (vw, vh) = renderer.viewport_size();
    let cx = vw as f32 / 2.0;
    let cy = vh as f32 * 0.45;
    let rx = vw as f32 * 0.3;
    let ry = vh as f32 * 0.06;

    let handoff = matches!(
        shell.phase,
        v2k_game::game_state::MenuPhase::GameStart { .. }
    );
    if !handoff {
        // Draw title — yellow matching original's palette index 57 (255,255,0)
        let (title_rgba, title_w, title_h) = rasterize_text_colored("V2000", 255, 255, 0);
        let title_x = (vw as i32 - title_w as i32) / 2;
        renderer.draw_sprite(&title_rgba, title_w, title_h, title_x, vh as i32 / 8);
    }

    // Compute carousel layout (fallback path): rotation from the original
    // scroll interpolator; selected item lands at the front.
    let n = menu.items.len().max(1) as f32;
    let angle = (menu.scroll / 200.0 - menu.selected as f32) * (std::f32::consts::TAU / n);
    let slots = compute_carousel_layout(menu.items.len(), angle, cx, cy, rx, ry);
    let (selected_offset, other_offset) = v2k_game::game_state::fly_offsets(shell.phase);

    // Draw icons back-to-front using trophy sprites from OVL
    for slot in &slots {
        // Use trophy sprite from MenuResources if available, else fall back to item icon
        let (icon_rgba, icon_w, icon_h) =
            if let Some(trophy) = menu_resources.trophy_icons.get(slot.item_index) {
                (&trophy.rgba[..], trophy.width, trophy.height)
            } else {
                let item = &menu.items[slot.item_index];
                if item.icon_width == 0 || item.icon_height == 0 {
                    continue;
                }
                (&item.icon_rgba[..], item.icon_width, item.icon_height)
            };

        // Match the 3D path's authored depth fly in screen space. Scaling the
        // projected displacement as well as the icon makes the selected prop
        // approach the lens while all other props recede, without inventing a
        // separate software-only transition.
        let theta =
            (slot.item_index as f32 - menu.selected as f32 + menu.scroll / 200.0) * RING_STEP_RAD;
        let base_depth = RING_Y_IN - 1200.0 * theta.cos();
        let offset = if slot.item_index == menu.selected {
            selected_offset
        } else {
            other_offset
        };
        let fly_scale = menu_depth_scale(base_depth, offset);
        let scaled_w = (icon_w as f32 * slot.scale * fly_scale) as u32;
        let scaled_h = (icon_h as f32 * slot.scale * fly_scale) as u32;

        if scaled_w == 0 || scaled_h == 0 {
            continue;
        }

        let scaled = scale_rgba(icon_rgba, icon_w, icon_h, scaled_w, scaled_h);
        let tinted = tint_rgba(&scaled, slot.alpha, slot.alpha);

        let projected_x = cx + (slot.x - cx) * fly_scale;
        let projected_y = cy + (slot.y - cy) * fly_scale;
        let draw_x = projected_x as i32 - scaled_w as i32 / 2;
        let draw_y = projected_y as i32 - scaled_h as i32 / 2;
        renderer.draw_sprite(&tinted, scaled_w, scaled_h, draw_x, draw_y);
    }

    if !handoff {
        // Draw selected item label — white (palette index 60)
        let selected_label = &menu.items[menu.selected].label;
        let (label_rgba, label_w, label_h) = rasterize_text_colored(selected_label, 255, 255, 255);
        let label_x = (vw as i32 - label_w as i32) / 2;
        let label_y = cy as i32 + ry as i32 + 50;
        renderer.draw_sprite(&label_rgba, label_w, label_h, label_x, label_y);

        // Navigation hint — cyan (palette index 62, 0,255,255)
        let hint = "Left/Right: Select  Enter: Confirm";
        let (hint_rgba, hint_w, hint_h) = rasterize_text_colored(hint, 0, 200, 200);
        let hint_x = (vw as i32 - hint_w as i32) / 2;
        let hint_y = vh as i32 - 60;
        renderer.draw_sprite(&hint_rgba, hint_w, hint_h, hint_x, hint_y);
    }

    if shell.frontend_branding_visible() {
        draw_menu_branding(renderer, system_strings, menu_resources, vw, vh);
    }
}

fn menu_depth_scale(base_depth_raw: f32, offset_raw: f32) -> f32 {
    base_depth_raw / (base_depth_raw + offset_raw).max(1.0)
}

fn reset_frame_clock_after_blocking_load(last_frame: &mut Instant) {
    *last_frame = Instant::now();
}

/// Render a vertical list menu (submenus), with the screen's decorative
/// 3D prop (from the screen record) turning behind the text.
#[allow(clippy::too_many_arguments)]
fn render_list(
    renderer: &mut dyn v2k_render::Renderer,
    shell: &MenuShell,
    menu: &v2k_game::game_state::MenuState,
    system_strings: &[String],
    fonts: Option<&v2k_game::menu_text::MenuFonts>,
    cache: &v2k_game::resource_cache::ResourceCache,
    colors: &v2k_game::model_color::ModelMaterialCache,
    menu_resources: &v2k_game::menu::MenuResources,
    billboard: Option<&BillboardRenderFrame>,
    retail_tick: u32,
    presentation: MenuPresentation,
) {
    use v2k_game::menu_text;

    let frontend = presentation.uses_frontend_hangar();
    let frontend_frame = if frontend {
        Some(billboard.expect("frontend list requires its render snapshot"))
    } else {
        None
    };
    if frontend {
        renderer.clear(0.01, 0.01, 0.03);
    }
    let (vw, vh) = renderer.viewport_size();
    let ui_request = UiMappingRequest {
        viewport: [vw, vh],
        authored_canvas: fonts
            .map(|f| [f.virtual_w as u32, f.virtual_h as u32])
            .unwrap_or([320, 240]),
        policy: renderer.ui_submission_policy(),
    };

    let (_sel_off, other_off) = v2k_game::game_state::fly_offsets(shell.phase);

    if renderer.supports_models() {
        let depth_fade_color = if frontend {
            menu_depth_fade_color(cache)
        } else {
            MENU_PAUSE_DEPTH_FADE_COLOR
        };
        renderer.set_camera(&menu_camera(vw as f32 / vh as f32));
        if frontend {
            let frame = frontend_frame.expect("frontend list snapshot");
            let backdrop_depth = shell
                .menu_backdrop_depth_raw()
                .max(MENU_BACKDROP_VIEW_DEPTH_RAW);
            draw_menu_backdrop(
                renderer,
                cache,
                colors,
                MenuBackdropFrame {
                    presentation: MenuBackdropPresentation::Frontend,
                    retail_tick,
                    anim_state: shell.menu_backdrop_anim_state(),
                    view_depth_raw: backdrop_depth,
                    projection_y_offset: shell.menu_billboard_y_offset(),
                    depth_fade: menu_depth_fade_from_raw(
                        frame.scene_depth_fade_near_raw,
                        frame.scene_depth_fade_far_raw,
                        depth_fade_color,
                    ),
                },
                MenuSceneLight::NEUTRAL,
            );
        }

        // Screen flag 0x10: full-screen `optionsh` backdrop model (global
        // id 73) at fixed angle 0. FUN_0043B130 passes ring input y=0x1004
        // into FUN_0043B410; angle 0 applies the ring offsets y=-320 raw and
        // z=0x1004-0x4B0 = 2900 raw.
        if menu.has_backdrop_panel {
            // B130 overwrites AA40's local pair before invoking B410 for the
            // panel. The following flag-0x02 decoration resets independently.
            let panel_fade_raw = MENU_SCREEN_FADE_RAW + 2900.0;
            let panel_depth_fade =
                menu_depth_fade_from_raw(panel_fade_raw, panel_fade_raw, depth_fade_color);
            if cache.global_model(73).is_some() {
                let virtual_h = fonts.map(|f| f.virtual_h).unwrap_or(240.0);
                let (_, projection_y) = fonts
                    .and_then(|f| f.layout_point(5))
                    .or_else(|| menu_text::layout_point(cache, 5))
                    .unwrap_or((virtual_h * 2.0 / 3.0, virtual_h * 106.0 / 240.0));
                let prop_camera = menu_camera_for_projection_y(projection_y, ui_request);
                renderer.set_camera(&prop_camera);
                draw_prop(
                    renderer,
                    cache,
                    colors,
                    &prop_camera,
                    73,
                    [0.0, -0x140 as f32 / 100.0, -29.0],
                    // `optionsh` is a wrapper around two mirrored `anim`
                    // halves. Its authored front points down local -Z, opposite
                    // the ordinary ring props. Retail's angle-zero menu basis
                    // presents that side to the camera; the port's identity
                    // model basis presented the back instead. That left only the
                    // rear pipe/light geometry visible and culled every front
                    // face carrying sprites 390 (the two emblems), 628 (the dark
                    // green fill), 1180 (hazard tape), and 1183 (the inner
                    // frame). Supply the equivalent half-turn before
                    // authored-plane culling.
                    MENU_OPTIONS_PANEL_ROTATION_Y,
                    1.0,
                    2,
                    // The emblem attack illuminates Klaus behind the submenu,
                    // not the opaque optionsh frame/screen in the foreground.
                    MenuSceneLight::NEUTRAL,
                    panel_depth_fade,
                    retail_tick,
                    v2k_game::game_state::fly_model_anim_raw(shell.phase),
                );
            }
        }

        // The screen's own decoration prop (screenop/sfxopt2/…) at layout
        // pt6 (160,170 → below center), spinning at the global 3.436 s/rev.
        if let Some(model_id) = menu.backdrop_model {
            if cache.global_model(model_id as usize).is_some() {
                let virtual_h = fonts.map(|f| f.virtual_h).unwrap_or(240.0);
                let (_, projection_y) = fonts
                    .and_then(|f| f.layout_point(6))
                    .or_else(|| menu_text::layout_point(cache, 6))
                    .unwrap_or((virtual_h * 2.0 / 3.0, virtual_h * 170.0 / 240.0));
                let depth_raw = 2500.0 + other_off;
                let prop_camera = menu_camera_for_projection_y(projection_y, ui_request);
                renderer.set_camera(&prop_camera);
                let row_near_raw = frontend_frame
                    .map(|frame| frame.row_depth_fade_near_raw)
                    .unwrap_or(MENU_PROP_ROW_NEUTRAL_NEAR_RAW);
                draw_prop(
                    renderer,
                    cache,
                    colors,
                    &prop_camera,
                    model_id as usize,
                    [0.0, -0x140 as f32 / 100.0, -depth_raw / 100.0],
                    menu.spin,
                    1.0,
                    2,
                    // Screen-specific foreground decoration (the monitor on
                    // Display, speaker on Sound, etc.) keeps authored colour.
                    MenuSceneLight::NEUTRAL,
                    // This one-prop row has flags 0x03, so AA40 rebuilds its
                    // band before B410. Frontend uses the post-pin world near;
                    // pause retains a capped neutral band without S7[55] red.
                    menu_prop_row_depth_fade(row_near_raw, depth_raw, depth_fade_color),
                    retail_tick,
                    v2k_game::game_state::fly_model_anim_raw(shell.phase),
                );
            }
        }
    }

    if frontend {
        let frame = frontend_frame.expect("frontend list snapshot");
        // The billboard is an additive sorted sprite, not an opaque alpha quad.
        // Drawing it after the model pass lets the depth buffer reproduce the
        // original queue ordering while retaining Klaus through the glow.
        draw_billboard(
            renderer,
            cache,
            menu_resources,
            frame,
            shell.menu_billboard_pose(),
        );
    }

    if shell.phase != v2k_game::game_state::MenuPhase::Idle {
        if frontend {
            draw_menu_branding(renderer, system_strings, menu_resources, vw, vh);
        }
        return;
    }

    let Some(f) = fonts else {
        render_list_fallback(renderer, menu, vw, vh);
        if frontend {
            draw_menu_branding(renderer, system_strings, menu_resources, vw, vh);
        }
        return;
    };
    let mapping = UiMapping::new(ui_request);

    // Synthetic overlay screens (save/load slot picker) keep a title row;
    // real screens have none in the original.
    if !menu.title.is_empty() {
        draw_menu_text(
            renderer,
            f.font(true),
            &menu.title,
            [160.0, 40.0],
            true,
            usize::MAX,
            mapping,
        );
    }

    // Rows: layout pt3 = list start (67,105), pt4 = row step (0,14), values
    // right-aligned at label_x + pt7.x (185). A scrolling window of
    // Authored row-group window: retail keeps one row of look-ahead in the
    // navigation direction (highlight = yellow font, snaps), and glides only
    // when that committed window moves. `shell.list_scroll()` is the smooth
    // top-of-window origin in row units.
    let (start_x, start_y) = f
        .layout_point(3)
        .or_else(|| menu_text::layout_point(cache, 3))
        .unwrap_or((67.0, 105.0));
    let (_, step_y) = f
        .layout_point(4)
        .or_else(|| menu_text::layout_point(cache, 4))
        .unwrap_or((0.0, 14.0));
    let (value_dx, _) = f
        .layout_point(7)
        .or_else(|| menu_text::layout_point(cache, 7))
        .unwrap_or((185.0, 0.0));

    let win = shell.list_window_rows() as f32;
    let top = shell.list_scroll();

    // Keep a one-row overscan in the iterator so scrolling remains smooth, but
    // clip its pixels at the first row's real glyph top. The previous
    // `start_y-step_y` scissor exposed that whole overscan row over optionsh's
    // upper frame (the port-only "Rendering" text above the rectangle). The
    // lower edge likewise ends at the last visible glyph, not the next row.
    let ascent = f.normal.max_ascent().max(f.selected.max_ascent());
    let descent = f.normal.max_descent().max(f.selected.max_descent());
    let (clip_top, clip_bottom) = menu_list_clip_y(mapping, start_y, step_y, win, ascent, descent);
    renderer.set_sprite_clip(Some((
        0,
        clip_top,
        vw,
        (clip_bottom - clip_top).max(0) as u32,
    )));

    for (i, item) in menu.items.iter().enumerate() {
        // Row position relative to the window origin. Allow a one-row overscan
        // each edge so a row scrolling in/out slides smoothly rather than
        // popping.
        let offset = i as f32 - top;
        if offset <= -1.0 || offset >= win {
            continue;
        }
        // Disabled port-extension rows stay visible but use a deliberately
        // darkened green font. In particular, Classic Framebuffer is visibly
        // unavailable while Scaling is Native.
        let selected = i == menu.selected && item.is_interactive();
        let font = f.font(selected);
        let tone = if item.enabled {
            MenuTextTone::Normal
        } else {
            MenuTextTone::Disabled
        };
        // One authored pixel of inset avoids the scaled glyph tops riding the
        // upper frame edge (roughly 3 px at the common 1024x768 viewport).
        let baseline = start_y + 1.0 + offset * step_y;

        // Row-group flag 8 uses the staggered screen clock. The row entering
        // during a window glide uses the independently reset DCEC8 clock
        // unless flag 0x10 suppresses that retail behavior.
        let reveal_ms = shell.list_row_typewriter_elapsed_ms(i);
        let label_chars = typewriter_chars_from_elapsed(reveal_ms, item.label.chars().count());
        if label_chars == 0 {
            continue;
        }
        draw_menu_text_toned(
            renderer,
            font,
            &item.label,
            [start_x, baseline],
            false,
            label_chars,
            mapping,
            tone,
        );

        // Value column: original segmented glyph bar where requested, text
        // otherwise. Retail right-aligns the visible typewriter prefix at
        // label_x + pt7.x so the option appears from the right. Measuring
        // the full string first parks a left x that then grows rightward.
        let value_right = start_x + value_dx;
        if let Some((v, max)) = item.bar {
            let vt = menu_text::bar_string(v, max);
            let value_chars = typewriter_chars_from_elapsed(reveal_ms, vt.chars().count());
            if value_chars > 0 {
                let vx = value_right - font.measure_prefix(&vt, value_chars);
                draw_menu_bar(
                    renderer,
                    font,
                    &vt,
                    vx,
                    baseline,
                    value_chars,
                    mapping,
                    tone,
                );
            }
        } else if let Some(vt) = item.value_label.as_ref() {
            let value_chars = typewriter_chars_from_elapsed(reveal_ms, vt.chars().count());
            if value_chars > 0 {
                let vx = value_right - font.measure_prefix(vt, value_chars);
                draw_menu_text_toned(
                    renderer,
                    font,
                    vt,
                    [vx, baseline],
                    false,
                    value_chars,
                    mapping,
                    tone,
                );
            }
        }
    }

    renderer.set_sprite_clip(None);

    // FUN_0042B040 draws the Frontier/copyright/Grolier bottom sprites every
    // frontend frame, including settings screens and fly transitions.
    if frontend {
        draw_menu_branding(renderer, system_strings, menu_resources, vw, vh);
    }
}

/// Built-in-rasterizer list fallback for when the OVL fonts are missing.
fn render_list_fallback(
    renderer: &mut dyn v2k_render::Renderer,
    menu: &v2k_game::game_state::MenuState,
    vw: u32,
    vh: u32,
) {
    if !menu.title.is_empty() {
        let (title_rgba, title_w, title_h) = rasterize_text_colored(&menu.title, 255, 255, 0);
        let title_x = (vw as i32 - title_w as i32) / 2;
        renderer.draw_sprite(&title_rgba, title_w, title_h, title_x, vh as i32 / 6);
    }
    let row_height = 30i32;
    let total_height = menu.items.len() as i32 * row_height;
    let start_y = (vh as i32 - total_height) / 2;
    for (i, item) in menu.items.iter().enumerate() {
        let y = start_y + i as i32 * row_height;
        let text = if let Some(ref val) = item.value_label {
            format!("{}: {}", item.label, val)
        } else {
            item.label.clone()
        };
        let (r, g, b) = if !item.enabled {
            (24, 80, 24)
        } else if i == menu.selected && item.is_interactive() {
            (255, 255, 0)
        } else {
            (60, 200, 60)
        };
        let (text_rgba, text_w, text_h) = rasterize_text_colored(&text, r, g, b);
        let x = (vw as i32 - text_w as i32) / 2;
        renderer.draw_sprite(&text_rgba, text_w, text_h, x, y);
    }
}

/// Simple bitmap font rasterizer — renders text as RGBA pixels.
fn rasterize_text(text: &str) -> (Vec<u8>, u32, u32) {
    rasterize_text_colored(text, 200, 200, 220)
}

/// Rasterize text with a specific color (matching Level 2 Section 7 palette).
fn rasterize_text_colored(text: &str, r: u8, g: u8, b: u8) -> (Vec<u8>, u32, u32) {
    let char_w = 8u32;
    let char_h = 12u32;
    let width = text.len() as u32 * char_w;
    let height = char_h;

    if width == 0 {
        return (vec![], 0, 0);
    }

    let mut rgba = vec![0u8; (width * height * 4) as usize];

    for (ci, ch) in text.chars().enumerate() {
        let glyph = get_glyph(ch);
        let x_off = ci as u32 * char_w;
        for (row, &bits) in glyph.iter().enumerate() {
            for col in 0..char_w {
                if (bits >> (7 - col)) & 1 != 0 {
                    let px = x_off + col;
                    let py = row as u32;
                    let offset = ((py * width + px) * 4) as usize;
                    if offset + 3 < rgba.len() {
                        rgba[offset] = r;
                        rgba[offset + 1] = g;
                        rgba[offset + 2] = b;
                        rgba[offset + 3] = 255;
                    }
                }
            }
        }
    }

    (rgba, width, height)
}

/// Get a simple 8-pixel-wide bitmap glyph for a character (12 rows).
fn get_glyph(ch: char) -> [u8; 12] {
    match ch {
        'A' => [
            0x00, 0x18, 0x3C, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00,
        ],
        'B' => [
            0x00, 0x7C, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x66, 0x7C, 0x00, 0x00, 0x00,
        ],
        'C' => [
            0x00, 0x3C, 0x66, 0x60, 0x60, 0x60, 0x60, 0x66, 0x3C, 0x00, 0x00, 0x00,
        ],
        'D' => [
            0x00, 0x78, 0x6C, 0x66, 0x66, 0x66, 0x66, 0x6C, 0x78, 0x00, 0x00, 0x00,
        ],
        'E' => [
            0x00, 0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x60, 0x7E, 0x00, 0x00, 0x00,
        ],
        'F' => [
            0x00, 0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x60, 0x60, 0x00, 0x00, 0x00,
        ],
        'G' => [
            0x00, 0x3C, 0x66, 0x60, 0x60, 0x6E, 0x66, 0x66, 0x3E, 0x00, 0x00, 0x00,
        ],
        'H' => [
            0x00, 0x66, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00,
        ],
        'I' => [
            0x00, 0x3C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00, 0x00,
        ],
        'J' => [
            0x00, 0x1E, 0x0C, 0x0C, 0x0C, 0x0C, 0x6C, 0x6C, 0x38, 0x00, 0x00, 0x00,
        ],
        'K' => [
            0x00, 0x66, 0x6C, 0x78, 0x70, 0x78, 0x6C, 0x66, 0x66, 0x00, 0x00, 0x00,
        ],
        'L' => [
            0x00, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7E, 0x00, 0x00, 0x00,
        ],
        'M' => [
            0x00, 0x63, 0x77, 0x7F, 0x6B, 0x63, 0x63, 0x63, 0x63, 0x00, 0x00, 0x00,
        ],
        'N' => [
            0x00, 0x66, 0x76, 0x7E, 0x7E, 0x6E, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00,
        ],
        'O' => [
            0x00, 0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00,
        ],
        'P' => [
            0x00, 0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x60, 0x00, 0x00, 0x00,
        ],
        'Q' => [
            0x00, 0x3C, 0x66, 0x66, 0x66, 0x66, 0x6E, 0x3C, 0x0E, 0x00, 0x00, 0x00,
        ],
        'R' => [
            0x00, 0x7C, 0x66, 0x66, 0x7C, 0x78, 0x6C, 0x66, 0x66, 0x00, 0x00, 0x00,
        ],
        'S' => [
            0x00, 0x3C, 0x66, 0x60, 0x3C, 0x06, 0x06, 0x66, 0x3C, 0x00, 0x00, 0x00,
        ],
        'T' => [
            0x00, 0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00, 0x00,
        ],
        'U' => [
            0x00, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00,
        ],
        'V' => [
            0x00, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x3C, 0x18, 0x00, 0x00, 0x00,
        ],
        'W' => [
            0x00, 0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x63, 0x00, 0x00, 0x00,
        ],
        'X' => [
            0x00, 0x66, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00,
        ],
        'Y' => [
            0x00, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00, 0x00,
        ],
        'Z' => [
            0x00, 0x7E, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x60, 0x7E, 0x00, 0x00, 0x00,
        ],
        'a' => [
            0x00, 0x00, 0x00, 0x3C, 0x06, 0x3E, 0x66, 0x66, 0x3E, 0x00, 0x00, 0x00,
        ],
        'b' => [
            0x00, 0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x7C, 0x00, 0x00, 0x00,
        ],
        'c' => [
            0x00, 0x00, 0x00, 0x3C, 0x66, 0x60, 0x60, 0x66, 0x3C, 0x00, 0x00, 0x00,
        ],
        'd' => [
            0x00, 0x06, 0x06, 0x3E, 0x66, 0x66, 0x66, 0x66, 0x3E, 0x00, 0x00, 0x00,
        ],
        'e' => [
            0x00, 0x00, 0x00, 0x3C, 0x66, 0x7E, 0x60, 0x66, 0x3C, 0x00, 0x00, 0x00,
        ],
        'f' => [
            0x00, 0x1C, 0x36, 0x30, 0x7C, 0x30, 0x30, 0x30, 0x30, 0x00, 0x00, 0x00,
        ],
        'g' => [
            0x00, 0x00, 0x00, 0x3E, 0x66, 0x66, 0x3E, 0x06, 0x66, 0x3C, 0x00, 0x00,
        ],
        'h' => [
            0x00, 0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00,
        ],
        'i' => [
            0x00, 0x18, 0x00, 0x38, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00, 0x00,
        ],
        'j' => [
            0x00, 0x0C, 0x00, 0x1C, 0x0C, 0x0C, 0x0C, 0x6C, 0x6C, 0x38, 0x00, 0x00,
        ],
        'k' => [
            0x00, 0x60, 0x60, 0x66, 0x6C, 0x78, 0x6C, 0x66, 0x66, 0x00, 0x00, 0x00,
        ],
        'l' => [
            0x00, 0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00, 0x00,
        ],
        'm' => [
            0x00, 0x00, 0x00, 0x76, 0x7F, 0x6B, 0x6B, 0x63, 0x63, 0x00, 0x00, 0x00,
        ],
        'n' => [
            0x00, 0x00, 0x00, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00,
        ],
        'o' => [
            0x00, 0x00, 0x00, 0x3C, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00,
        ],
        'p' => [
            0x00, 0x00, 0x00, 0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x00, 0x00,
        ],
        'q' => [
            0x00, 0x00, 0x00, 0x3E, 0x66, 0x66, 0x3E, 0x06, 0x06, 0x06, 0x00, 0x00,
        ],
        'r' => [
            0x00, 0x00, 0x00, 0x7C, 0x66, 0x60, 0x60, 0x60, 0x60, 0x00, 0x00, 0x00,
        ],
        's' => [
            0x00, 0x00, 0x00, 0x3E, 0x60, 0x3C, 0x06, 0x06, 0x7C, 0x00, 0x00, 0x00,
        ],
        't' => [
            0x00, 0x30, 0x30, 0x7C, 0x30, 0x30, 0x30, 0x36, 0x1C, 0x00, 0x00, 0x00,
        ],
        'u' => [
            0x00, 0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3E, 0x00, 0x00, 0x00,
        ],
        'v' => [
            0x00, 0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00, 0x00, 0x00,
        ],
        'w' => [
            0x00, 0x00, 0x00, 0x63, 0x63, 0x6B, 0x6B, 0x7F, 0x36, 0x00, 0x00, 0x00,
        ],
        'x' => [
            0x00, 0x00, 0x00, 0x66, 0x66, 0x3C, 0x3C, 0x66, 0x66, 0x00, 0x00, 0x00,
        ],
        'y' => [
            0x00, 0x00, 0x00, 0x66, 0x66, 0x66, 0x3E, 0x06, 0x66, 0x3C, 0x00, 0x00,
        ],
        'z' => [
            0x00, 0x00, 0x00, 0x7E, 0x0C, 0x18, 0x30, 0x60, 0x7E, 0x00, 0x00, 0x00,
        ],
        '0' => [
            0x00, 0x3C, 0x66, 0x6E, 0x76, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00,
        ],
        '1' => [
            0x00, 0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00, 0x00, 0x00,
        ],
        '2' => [
            0x00, 0x3C, 0x66, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x7E, 0x00, 0x00, 0x00,
        ],
        '3' => [
            0x00, 0x3C, 0x66, 0x06, 0x1C, 0x06, 0x06, 0x66, 0x3C, 0x00, 0x00, 0x00,
        ],
        '4' => [
            0x00, 0x0C, 0x1C, 0x3C, 0x6C, 0x7E, 0x0C, 0x0C, 0x0C, 0x00, 0x00, 0x00,
        ],
        '5' => [
            0x00, 0x7E, 0x60, 0x7C, 0x06, 0x06, 0x06, 0x66, 0x3C, 0x00, 0x00, 0x00,
        ],
        '6' => [
            0x00, 0x3C, 0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00,
        ],
        '7' => [
            0x00, 0x7E, 0x06, 0x0C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00, 0x00,
        ],
        '8' => [
            0x00, 0x3C, 0x66, 0x66, 0x3C, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00,
        ],
        '9' => [
            0x00, 0x3C, 0x66, 0x66, 0x3E, 0x06, 0x06, 0x06, 0x3C, 0x00, 0x00, 0x00,
        ],
        ' ' => [0x00; 12],
        ':' => [
            0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00,
        ],
        '/' => [
            0x00, 0x02, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x40, 0x00, 0x00, 0x00, 0x00,
        ],
        '.' => [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00,
        ],
        ',' => [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x30, 0x00, 0x00,
        ],
        '-' => [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
        '!' => [
            0x00, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00,
        ],
        '?' => [
            0x00, 0x3C, 0x66, 0x06, 0x0C, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00,
        ],
        '%' => [
            0x00, 0x62, 0x66, 0x0C, 0x18, 0x30, 0x66, 0x46, 0x00, 0x00, 0x00, 0x00,
        ],
        '(' => [
            0x00, 0x0C, 0x18, 0x30, 0x30, 0x30, 0x30, 0x18, 0x0C, 0x00, 0x00, 0x00,
        ],
        ')' => [
            0x00, 0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x0C, 0x18, 0x30, 0x00, 0x00, 0x00,
        ],
        '\'' => [
            0x00, 0x18, 0x18, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
        '"' => [
            0x00, 0x66, 0x66, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
        '#' => [
            0x00, 0x24, 0x24, 0x7E, 0x24, 0x7E, 0x24, 0x24, 0x00, 0x00, 0x00, 0x00,
        ],
        '<' => [
            0x00, 0x06, 0x0C, 0x18, 0x30, 0x18, 0x0C, 0x06, 0x00, 0x00, 0x00, 0x00,
        ],
        '>' => [
            0x00, 0x60, 0x30, 0x18, 0x0C, 0x18, 0x30, 0x60, 0x00, 0x00, 0x00, 0x00,
        ],
        '+' => [
            0x00, 0x00, 0x00, 0x18, 0x18, 0x7E, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00,
        ],
        '=' => [
            0x00, 0x00, 0x00, 0x7E, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
        '_' => [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00,
        ],
        '@' => [
            0x00, 0x3C, 0x66, 0x6E, 0x6E, 0x60, 0x60, 0x66, 0x3C, 0x00, 0x00, 0x00,
        ],
        _ => [
            0x00, 0x7E, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x7E, 0x00, 0x00, 0x00,
        ],
    }
}

#[cfg(test)]
mod active_pair_scope_tests {
    use super::*;
    use v2k_formats::levels::{EntitySpawn, LevelDescriptor};

    fn main_base_abort_test_entities(player_position_raw: Option<[i16; 3]>) -> EntityManager {
        let entities = player_position_raw
            .map(|position_raw| EntitySpawn {
                index: 0,
                entity_type: 46,
                pos_data_1: [
                    position_raw[0].to_le_bytes()[0],
                    position_raw[0].to_le_bytes()[1],
                    position_raw[1].to_le_bytes()[0],
                    position_raw[1].to_le_bytes()[1],
                ],
                pos_data_2: [
                    position_raw[2].to_le_bytes()[0],
                    position_raw[2].to_le_bytes()[1],
                    0,
                    0,
                ],
                param: 0,
                rotation: [0; 3],
                extra: [0; 40],
                initial_damage_buffer_raw: 0,
                model_overrides: [0; 4],
                has_animation: false,
                anim_frames: 0,
                animation: None,
                has_config: false,
                config: None,
            })
            .into_iter()
            .collect::<Vec<_>>();
        let level = LevelDescriptor {
            raw_header: [0; 0xd0],
            name: "Main Base abort transaction test".into(),
            world_style: 0,
            terrain_sprite_base: 0,
            sky_color_index: 0,
            sky_model: 0,
            main_base_abort_sky_color_index: 0,
            main_base_abort_sky_model: 0,
            terrain_draw_depth: 30,
            sub_count: entities.len() as u32,
            campaign_record_count: 0,
            entities,
            campaign_records: Vec::new(),
        };
        EntityManager::from_level(&level, &vec![[0; 4]; 47], None)
    }

    fn bind_main_base_abort_test_controller(state: &mut MainBaseAbortBridgeState, generation: u64) {
        let mut data = vec![0u8; 0xD0];
        data[0x4C..0x50].copy_from_slice(&0x650u32.to_le_bytes());
        data[0x54..0x56].copy_from_slice(&27u16.to_le_bytes());
        data[0x56..0x58].copy_from_slice(&305u16.to_le_bytes());
        data[0x58..0x5A].copy_from_slice(&32u16.to_le_bytes());
        data[0x88..0x8C].copy_from_slice(&21u32.to_le_bytes());
        data[0x8C..0x90].copy_from_slice(&8u32.to_le_bytes());
        let level = v2k_formats::levels::parse_level(&data).unwrap();
        let mut progress = PlayerCampaignProgress::new();
        progress.set_current_control_slot(Some(1));
        state
            .bind_loaded_world(NonZeroU64::new(generation).unwrap(), &level, &mut progress)
            .unwrap();
    }

    #[test]
    fn castle_casualty_warning_resets_on_load_and_abort_header_suppresses_reentry() {
        use v2k_game::campaign_failure::{
            CampaignCasualtyAction, CampaignCasualtyFrame, CampaignCasualtyText,
        };
        let mut raw = [0; 32];
        raw[0x0c..0x10].copy_from_slice(&0x84u32.to_le_bytes());
        raw[0x19] = 5; // Castle's authored record2, confirmed by corpus test.
        let records = [v2k_formats::levels::CampaignRecord { data: raw }];
        let mut frame = CampaignCasualtyFrame {
            census: v2k_game::world_complete_results::Fun0042e210CapabilityCensus {
                cap_0x400: 0,
                cap_0x800: 6,
                cap_0x08: 0,
            },
            world_saved: false,
            abort_active: false,
        };
        let warning = [CampaignCasualtyAction::DirectText(
            CampaignCasualtyText::OneMoreLoss,
        )];
        let mut state = MainBaseAbortBridgeState::default();
        bind_main_base_abort_test_controller(&mut state, 30);
        assert_eq!(state.casualty.evaluate(&records, frame).unwrap(), warning);
        assert!(state.casualty.evaluate(&records, frame).unwrap().is_empty());
        frame.census.cap_0x800 = 5;
        assert_eq!(
            state.casualty.evaluate(&records, frame).unwrap(),
            [
                CampaignCasualtyAction::DirectText(CampaignCasualtyText::WorldLost),
                CampaignCasualtyAction::AbortWorld { record_index: 0 },
            ]
        );
        let mut entities = main_base_abort_test_entities(Some([12, 34, 56]));
        let mut fx = WorldFx::new();
        assert!(matches!(
            begin_main_base_abort_transaction(&mut state, None, &mut entities, &mut fx),
            MainBaseAbortAdmission::BodyAdmitted(_)
        ));
        assert!(state.terminal_origin.is_none());
        frame.abort_active = state.abort_flag_0x28f();
        assert!(state.casualty.evaluate(&records, frame).unwrap().is_empty());
        bind_main_base_abort_test_controller(&mut state, 31);
        frame.abort_active = state.abort_flag_0x28f();
        frame.census.cap_0x800 = 6;
        assert!(!frame.abort_active);
        assert_eq!(state.casualty.evaluate(&records, frame).unwrap(), warning);
        frame.world_saved = true;
        frame.census.cap_0x800 = 0;
        assert!(state.casualty.evaluate(&records, frame).unwrap().is_empty());
    }

    #[test]
    fn shared_pair_walkers_run_in_ordinary_worlds() {
        assert!(ordinary_world_pair_pass_required(Some(
            FIRST_WORLD_LEVEL_ID
        )));
        assert!(ordinary_world_pair_pass_required(Some(
            FIRST_WORLD_LEVEL_ID + 1
        )));
        assert!(!ordinary_world_pair_pass_required(Some(INTRO2_LEVEL_ID)));
        assert!(!ordinary_world_pair_pass_required(None));
    }

    #[test]
    fn active_solid_adapter_runs_on_campaign_worlds_but_not_intro2() {
        assert!(player_active_solid_pass_required(Some(
            FIRST_WORLD_LEVEL_ID
        )));
        assert!(player_active_solid_pass_required(Some(
            FIRST_WORLD_LEVEL_ID + 1
        )));
        assert!(!player_active_solid_pass_required(Some(INTRO2_LEVEL_ID)));
        assert!(!player_active_solid_pass_required(None));
    }

    #[test]
    fn main_base_level_abort_state_is_one_shot() {
        let mut state = MainBaseAbortBridgeState::default();
        let mut world_fx = WorldFx::new();
        assert!(state.begin_fixture_without_terminal_origin(&mut world_fx));
        assert!(state.active);
        assert_eq!(state.session_dword_0x2bc, Some(0));
        assert!(!state.outer_frame_sequence.is_active());
        assert!(!state.begin_fixture_without_terminal_origin(&mut world_fx));
        assert!(state.active);
    }

    #[test]
    fn main_base_abort_queues_resource_3e_once_at_the_player() {
        let player_position_raw = [0x123, -0x234, 0x345];
        let mut entities = main_base_abort_test_entities(Some(player_position_raw));
        let mut state = MainBaseAbortBridgeState::default();
        bind_main_base_abort_test_controller(&mut state, 1);
        let mut world_fx = WorldFx::new();
        world_fx.queue_fixed_positional_sound_raw(1, [0, 0, 0]);
        world_fx.process_pending();
        world_fx.queue_fixed_positional_sound_raw(2, [0, 0, 0]);

        assert!(matches!(
            begin_main_base_abort_transaction_fixture(&mut state, &mut entities, &mut world_fx),
            MainBaseAbortAdmission::BodyAdmitted(_)
        ));
        world_fx.queue_fixed_positional_sound_raw(3, [0, 0, 0]);
        assert_eq!(
            begin_main_base_abort_transaction_fixture(&mut state, &mut entities, &mut world_fx),
            MainBaseAbortAdmission::Duplicate
        );
        world_fx.process_pending();

        assert_eq!(
            world_fx.take_positional_sounds(),
            vec![
                v2k_game::world_fx::PositionalSoundEvent::fixed(
                    usize::from(MAIN_BASE_ABORT_SOUND_ID),
                    player_position_raw.map(|word| f32::from(word) / 256.0),
                ),
                v2k_game::world_fx::PositionalSoundEvent::fixed(3, [0.0; 3]),
            ],
            "the first admission drops old requests, while a duplicate cannot sweep later ones"
        );
    }

    #[test]
    fn missing_player_suppresses_only_the_abort_sound() {
        let mut entities = main_base_abort_test_entities(None);
        let mut state = MainBaseAbortBridgeState::default();
        bind_main_base_abort_test_controller(&mut state, 2);
        let mut world_fx = WorldFx::new();

        assert!(matches!(
            begin_main_base_abort_transaction_fixture(&mut state, &mut entities, &mut world_fx),
            MainBaseAbortAdmission::BodyAdmitted(_)
        ));
        assert!(state.active);
        world_fx.process_pending();
        assert!(world_fx.take_positional_sounds().is_empty());
    }

    #[test]
    fn missing_control_slot_suppresses_the_systemic_abort_body() {
        let player_position_raw = [0x123, -0x234, 0x345];
        let mut entities = main_base_abort_test_entities(Some(player_position_raw));
        let mut state = MainBaseAbortBridgeState::default();
        let mut world_fx = WorldFx::new();
        world_fx.queue_fixed_positional_sound_raw(1, [0, 0, 0]);
        world_fx.process_pending();
        world_fx.queue_fixed_positional_sound_raw(2, [0, 0, 0]);

        assert_eq!(
            begin_main_base_abort_transaction_fixture(&mut state, &mut entities, &mut world_fx),
            MainBaseAbortAdmission::BodySuppressed
        );
        assert!(state.active, "FUN_00456960 has already armed session abort");
        assert_eq!(state.session_dword_0x2bc, Some(0));
        assert_eq!(
            state.outer_frame_sequence.current_frame(),
            Some(v2k_game::full_frame_sprite_sequence::FullFrameSpriteFrame {
                table_index: 1,
                global_sprite_id: 0x227,
            })
        );
        assert!(
            state.terminal_origin.is_none(),
            "a suppressed FUN_0042F1A0 body must not retain actor-sweep authority"
        );
        world_fx.process_pending();
        assert_eq!(
            world_fx.take_positional_sounds(),
            vec![v2k_game::world_fx::PositionalSoundEvent::fixed(
                usize::from(MAIN_BASE_ABORT_SOUND_ID),
                player_position_raw.map(|word| f32::from(word) / 256.0),
            )],
            "the positional 0x3E submission precedes FUN_0042F1A0's slot gate"
        );
    }

    #[test]
    fn bounded_post_terrain_suffix_submits_once_and_rebind_rejects_old_lease() {
        let mut entities = main_base_abort_test_entities(Some([0, 0, 0]));
        let mut state = MainBaseAbortBridgeState::default();
        bind_main_base_abort_test_controller(&mut state, 10);
        let normal = state.submitted_frame_request().unwrap();
        assert_eq!((normal.word_0xb0, normal.word_0xb2), (27, 305));
        assert_eq!(
            state.controller.unwrap().player_state(),
            RetailRuntimeValue::Known(v2k_game::time_trophy::TimeTrophyState::Secured as u32)
        );

        let mut world_fx = WorldFx::new();
        let MainBaseAbortAdmission::BodyAdmitted(old_lease) =
            begin_main_base_abort_transaction_fixture(&mut state, &mut entities, &mut world_fx)
        else {
            panic!("expected admitted Main Base abort body");
        };
        let abort = state
            .controller
            .as_mut()
            .unwrap()
            .commit_post_terrain_abort(old_lease)
            .unwrap();
        state.finish_outer_dispatch();
        assert_eq!(
            state.controller.unwrap().player_state(),
            RetailRuntimeValue::Known(v2k_game::main_base_abort::MAIN_BASE_ABORT_PLAYER_STATE)
        );
        assert_eq!(
            (
                abort.callback_address,
                abort.retained_dword_0xac,
                abort.word_0xb0,
                abort.word_0xb2,
                abort.dword_0xb4,
                abort.dword_0xb8,
                abort.dword_0xbc,
            ),
            (0x0042_E860, 0, 32, 0, 0x650, 21, 8),
        );
        let first_outer_frame = state.outer_frame_sequence.current_frame().unwrap();
        assert_eq!(first_outer_frame.global_sprite_id, 0x227);
        assert!(state
            .outer_frame_sequence
            .acknowledge_submitted(first_outer_frame));
        let second_outer_frame = state.outer_frame_sequence.current_frame().unwrap();
        assert_eq!(
            (
                second_outer_frame.table_index,
                second_outer_frame.global_sprite_id
            ),
            (2, 0x228)
        );
        assert_eq!(
            begin_main_base_abort_transaction_fixture(&mut state, &mut entities, &mut world_fx),
            MainBaseAbortAdmission::Duplicate
        );
        assert_eq!(
            state.outer_frame_sequence.current_frame(),
            Some(second_outer_frame),
            "a duplicate outer admission cannot restart FUN_00456750"
        );

        bind_main_base_abort_test_controller(&mut state, 11);
        assert!(!state.active);
        assert_eq!(state.session_dword_0x2bc, None);
        assert!(!state.outer_frame_sequence.is_active());
        assert_eq!(
            state
                .submitted_frame_request()
                .map(|request| (request.word_0xb0, request.word_0xb2,)),
            Some((27, 305)),
        );
        assert!(state
            .controller
            .as_mut()
            .unwrap()
            .commit_post_terrain_abort(old_lease)
            .is_err());
        assert!(matches!(
            begin_main_base_abort_transaction_fixture(&mut state, &mut entities, &mut world_fx),
            MainBaseAbortAdmission::BodyAdmitted(_)
        ));
    }
}

#[cfg(test)]
mod static_damage_presentation_tests {
    use super::*;

    #[test]
    fn opcode_6_uses_an_inclusive_wrapped_focus_distance_and_restarts_the_cursor() {
        let mut sequence = FullFrameSpriteSequence::default();
        let prepared_before_simulation = sequence.current_frame();

        assert!(!request_full_frame_sequence_within(
            &mut sequence,
            [0; 3],
            [0x1401, 0, 0],
            0x1400,
        ));
        assert_eq!(sequence.current_frame(), None);

        assert!(request_full_frame_sequence_within(
            &mut sequence,
            [0; 3],
            [0x1400, 0, 0],
            0x1400,
        ));
        assert_eq!(prepared_before_simulation, None);
        let first = sequence
            .current_frame()
            .expect("late request arms next frame");
        assert_eq!((first.table_index, first.global_sprite_id), (1, 0x227));
        assert!(sequence.acknowledge_submitted(first));
        let second = sequence.current_frame().expect("sequence advanced");
        assert_eq!(second.table_index, 2);

        assert!(!request_full_frame_sequence_within(
            &mut sequence,
            [0; 3],
            [0x1401, 0, 0],
            0x1400,
        ));
        assert_eq!(sequence.current_frame(), Some(second));
        let prepared_for_current_frame = second;
        assert!(sequence.acknowledge_submitted(prepared_for_current_frame));
        assert_eq!(sequence.current_frame().unwrap().table_index, 3);

        assert!(request_full_frame_sequence_within(
            &mut sequence,
            [i16::MAX, 0, 0],
            [i16::MIN, 0, 0],
            0x1400,
        ));
        assert_eq!(prepared_for_current_frame, second);
        assert_eq!(sequence.current_frame(), Some(first));
    }
}

#[cfg(test)]
mod menu_visual_tests {
    use super::*;

    #[test]
    fn loading_label_uses_the_even_twenty_tick_half_period() {
        assert!(loading_text_visible(0));
        assert!(loading_text_visible(19));
        assert!(!loading_text_visible(20));
        assert!(!loading_text_visible(39));
        assert!(loading_text_visible(40));
        assert!(loading_text_visible(59));
    }

    #[test]
    fn loading_label_blink_is_stable_across_the_wrapping_tick_boundary() {
        assert!(loading_text_visible((-1_i32) as u32));
        assert!(loading_text_visible((-17_i32) as u32));
        assert!(!loading_text_visible((-20_i32) as u32));
        assert!(!loading_text_visible((-39_i32) as u32));
        assert!(loading_text_visible((-40_i32) as u32));
        assert!(loading_text_visible(0));
    }

    #[test]
    fn underwater_projection_gate_is_strict_and_forwards_the_wrapping_tick() {
        assert_eq!(
            world_projection_effect(4.0, Some(1024), 0x8000_0001),
            ProjectionEffect::None
        );
        assert_eq!(
            world_projection_effect(1023.0 / 256.0, Some(1024), 0x8000_0001),
            ProjectionEffect::RetailUnderwater {
                tick: 0x8000_0001_u32 as i32
            }
        );
        assert_eq!(
            world_projection_effect(-100.0, None, 17),
            ProjectionEffect::None
        );
    }

    #[test]
    fn configured_sfx_gain_uses_the_persisted_slider_and_off_gate() {
        let mut config = GameConfig {
            sfx_volume: 4.0 / 15.0,
            ..GameConfig::default()
        };
        assert!((configured_sfx_gain(&config) - 4.0 / 15.0).abs() < f32::EPSILON);

        config.sound_enabled = false;
        assert_eq!(configured_sfx_gain(&config), 0.0);
    }

    #[test]
    fn live_audio_settings_are_scoped_to_their_own_subsystems() {
        assert_eq!(
            classify_live_audio_setting(SettingId::SoundVolume),
            LiveAudioSetting::SoundGain
        );
        assert_eq!(
            classify_live_audio_setting(SettingId::AmbientVolume),
            LiveAudioSetting::AmbientGate
        );
        assert_eq!(
            classify_live_audio_setting(SettingId::Bilinear),
            LiveAudioSetting::Unrelated
        );
        assert_eq!(
            classify_live_audio_setting(SettingId::Sensitivity),
            LiveAudioSetting::Unrelated
        );
    }

    #[test]
    fn ordinary_world_music_is_selected_even_when_the_gate_may_be_off() {
        for world_style in 1..=6 {
            assert_eq!(
                normal_world_music_track(FIRST_WORLD_LEVEL_ID, world_style),
                Some((world_style + 1) as u8)
            );
        }

        assert_eq!(normal_world_music_track(INTRO2_LEVEL_ID, 1), None);
        assert_eq!(normal_world_music_track(FIRST_WORLD_LEVEL_ID, 6), Some(7));
    }

    #[test]
    fn world_overlay_ids_map_back_to_retail_control_slots() {
        assert_eq!(world_control_slot(FIRST_WORLD_LEVEL_ID), Some(1));
        assert_eq!(world_control_slot(FIRST_WORLD_LEVEL_ID + 1), Some(2));
        assert_eq!(
            world_control_slot(WORLD_OVERLAY_CONTROL_SLOT_OFFSET),
            Some(0)
        );
        assert_eq!(
            world_control_slot(WORLD_OVERLAY_CONTROL_SLOT_OFFSET - 1),
            None
        );
        assert_eq!(
            world_control_slot(
                WORLD_OVERLAY_CONTROL_SLOT_OFFSET + RETAIL_CONTROL_SLOT_COUNT as u32
            ),
            None
        );
    }

    #[test]
    fn persistent_loadout_rebinds_to_a_fresh_player_component() {
        let mut inventory = WeaponInventory::new();
        assert!(matches!(
            inventory.acquire_weapon(v2k_game::weapon_inventory::PowerUpPayload {
                selector: 2,
                amount: 200,
            }),
            WeaponAcquisition::Accepted {
                auto_selected: true,
                ..
            }
        ));
        let mut craft = PlayerCraft::new();

        sync_player_weapon_callback(&mut craft, &inventory);

        assert_eq!(craft.weapon_selector(), 4);
    }

    #[test]
    fn final_upgraded_round_queues_before_rebinding_the_default_weapon() {
        let mut inventory = WeaponInventory::new();
        assert!(matches!(
            inventory.acquire_weapon(v2k_game::weapon_inventory::PowerUpPayload {
                selector: 2,
                amount: 1,
            }),
            WeaponAcquisition::Accepted {
                auto_selected: true,
                ..
            }
        ));
        let mut craft = PlayerCraft::new();
        sync_player_weapon_callback(&mut craft, &inventory);
        assert_eq!(craft.weapon_selector(), 4);

        let geometry = PrimaryFireGeometry {
            launch_basis: PrimaryLaunchBasis {
                origin_world: [0.0, 1.0, 0.0],
                direction_unit: [0.0, 0.0, 1.0],
            },
            gun_mounts: PrimaryGunMounts::Authored(PrimaryAuthoredGunMounts {
                gun_a: PrimaryLaunchBasis {
                    origin_world: [-1.0, 1.0, 0.0],
                    direction_unit: [0.0, 0.0, 1.0],
                },
                gun_b: PrimaryLaunchBasis {
                    origin_world: [1.0, 1.0, 0.0],
                    direction_unit: [0.0, 0.0, 1.0],
                },
            }),
            shooter_origin_world: [0.0, 1.0, 0.0],
            shooter_id: 46,
            shooter_entity_type_at_birth: Some(46),
            shooter_velocity_world: [0.0; 3],
            sound_origin_world: [0.0, 1.0, 0.0],
        };
        let event = PrimaryWeapon::new()
            .update_with_profile(
                Duration::ZERO,
                PrimaryTriggerInput {
                    source_a: true,
                    source_b: false,
                },
                geometry,
                UPGRADED_PRIMARY_PROFILE,
                PrimaryShotBudget::Limited(1),
            )
            .into_iter()
            .next()
            .expect("the final round still emits");

        let commit = inventory.commit_selected_round();
        let mut world_fx = WorldFx::new();
        world_fx.queue_primary_fire_batch(&[event], Some(0.0));
        sync_player_weapon_after_ammo_commit(commit, &mut craft, &inventory);
        world_fx.process_pending();

        assert!(matches!(
            commit,
            AmmoCommit::Fired {
                slot: 1,
                selector: 2,
                remaining: Ammunition::Finite(0),
                automatic_successor_slot: Some(0),
            }
        ));
        assert_eq!(inventory.selected_slot(), 0);
        assert_eq!(craft.weapon_selector(), 0);
        assert_eq!(world_fx.particle_count(), 2);
        assert_eq!(world_fx.take_positional_sounds().len(), 1);
    }

    #[test]
    fn committed_power_up_feedback_keeps_live_order_rates_and_weapon_callback() {
        let mut inventory = WeaponInventory::new();
        let payload = v2k_game::weapon_inventory::PowerUpPayload {
            selector: 2,
            amount: 200,
        };
        let acquisition = inventory.acquire_weapon(payload);
        let pass = PlayerPowerUpContactPass {
            contacts: vec![
                PowerUpContactOutcome::Accepted(AcceptedPowerUpContact::Weapon {
                    entity_id: 1,
                    position_raw: [256, 512, 768],
                    payload,
                    acquisition,
                }),
                PowerUpContactOutcome::Rejected(
                    v2k_game::power_up_contact::RejectedPowerUpContact {
                        entity_id: 2,
                        position_raw: [512, 768, 1024],
                        payload: v2k_game::weapon_inventory::PowerUpPayload {
                            selector: 0x3c,
                            amount: 0,
                        },
                        reason: PowerUpContactRejection::TargetterAlreadyAcquired,
                    },
                ),
                PowerUpContactOutcome::Accepted(AcceptedPowerUpContact::Turbo {
                    entity_id: 3,
                    position_raw: [768, 1024, 1280],
                    payload: v2k_game::weapon_inventory::PowerUpPayload {
                        selector: 0x3e,
                        amount: 0,
                    },
                }),
                PowerUpContactOutcome::Rejected(
                    v2k_game::power_up_contact::RejectedPowerUpContact {
                        entity_id: 4,
                        position_raw: [1024, 1280, 1536],
                        payload: v2k_game::weapon_inventory::PowerUpPayload {
                            selector: 0x3e,
                            amount: 0,
                        },
                        reason: PowerUpContactRejection::TurboAlreadyAcquired,
                    },
                ),
                PowerUpContactOutcome::Accepted(AcceptedPowerUpContact::Trophy {
                    entity_id: 5,
                    position_raw: [1280, 1536, 1792],
                    payload: v2k_game::weapon_inventory::PowerUpPayload {
                        selector: 0x3f,
                        amount: 0,
                    },
                    acquisition: v2k_game::power_up_contact::TrophyAcquisition {
                        restored_hull: true,
                        first_claim: true,
                        counted_trophy: true,
                        awarded_extra_life: true,
                        trophy_count: 5,
                        extra_lives: 1,
                    },
                }),
            ],
        };
        let mut craft = PlayerCraft::new();
        let mut notifications = GameplayNotifications::new();
        let mut fx = WorldFx::new();

        assert!(
            apply_power_up_contact_feedback(
                &pass,
                &inventory,
                &mut craft,
                &mut notifications,
                &mut fx,
                50,
            )
            .hull_state_changed
        );
        assert_eq!(craft.weapon_selector(), 4);
        fx.process_pending();
        let sounds = fx.take_positional_sounds();
        assert_eq!(
            sounds
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>(),
            vec![5, 5, 5, 5, 0x32]
        );
        assert_eq!(
            sounds
                .iter()
                .map(|sound| sound.frequency_q16)
                .collect::<Vec<_>>(),
            vec![0x1_0000, 0xAAAA, 0x1_0000, 0xAAAA, 0x2_0000,]
        );
        assert_eq!(
            sounds
                .iter()
                .map(|sound| sound.position)
                .collect::<Vec<_>>(),
            vec![
                [1.0, 2.0, 3.0],
                [2.0, 3.0, 4.0],
                [3.0, 4.0, 5.0],
                [4.0, 5.0, 6.0],
                [5.0, 6.0, 7.0],
            ]
        );
    }

    #[test]
    fn turbo_capability_maps_directly_to_the_vtol_boost_policy() {
        let mut capabilities = PlayerCapabilities::default();
        assert_eq!(player_vtol_boost(&capabilities), VtolBoost::Inactive);

        assert_eq!(
            capabilities.acquire_turbo(v2k_game::weapon_inventory::PowerUpPayload {
                selector: v2k_game::weapon_inventory::TURBO_SELECTOR,
                amount: 0,
            }),
            v2k_game::weapon_inventory::TurboAcquisition::Acquired
        );
        assert_eq!(player_vtol_boost(&capabilities), VtolBoost::Active);
    }

    #[test]
    fn non_spawning_power_up_feedback_preserves_each_branch_sound_policy() {
        use v2k_game::weapon_inventory::PowerUpPayload;

        let accepted = |entity_id, selector, acquisition| {
            PowerUpContactOutcome::Accepted(AcceptedPowerUpContact::NonSpawning {
                entity_id,
                position_raw: [entity_id as i16 * 256, 256, 512],
                payload: PowerUpPayload {
                    selector,
                    amount: 1,
                },
                acquisition,
            })
        };
        let pass = PlayerPowerUpContactPass {
            contacts: vec![
                accepted(
                    1,
                    0x33,
                    NonSpawningPowerUpAcquisition::Fuel {
                        fuel_before_raw: 0,
                        fuel_after_raw: 1,
                    },
                ),
                accepted(
                    2,
                    0x35,
                    NonSpawningPowerUpAcquisition::Shield {
                        buffer_before_raw: 0,
                        buffer_after_raw: 1,
                    },
                ),
                accepted(
                    3,
                    0x36,
                    NonSpawningPowerUpAcquisition::ExtraLife {
                        extra_lives_before: 0,
                        extra_lives_after: 1,
                    },
                ),
                accepted(
                    4,
                    0x37,
                    NonSpawningPowerUpAcquisition::HullRepair {
                        health_before_raw: 1,
                        health_after_raw: 2,
                    },
                ),
                accepted(
                    5,
                    0x3a,
                    NonSpawningPowerUpAcquisition::CargoCapacity {
                        capacity_before: 1,
                        capacity_after: 2,
                        unlock_raw_before: 1,
                        unlock_raw_after: 2,
                        cargo_list_available: true,
                    },
                ),
            ],
        };
        let inventory = WeaponInventory::new();
        let mut craft = PlayerCraft::new();
        let mut notifications = GameplayNotifications::new();
        let mut fx = WorldFx::new();

        assert!(
            apply_power_up_contact_feedback(
                &pass,
                &inventory,
                &mut craft,
                &mut notifications,
                &mut fx,
                50,
            )
            .hull_state_changed
        );
        fx.process_pending();
        let sounds = fx.take_positional_sounds();
        assert_eq!(
            sounds
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>(),
            vec![
                usize::from(PLAYER_PICKUP_SOUND_ID),
                usize::from(PLAYER_PICKUP_SOUND_ID),
                usize::from(PLAYER_PICKUP_SOUND_ID)
            ]
        );
        assert_eq!(
            sounds
                .iter()
                .map(|sound| sound.position)
                .collect::<Vec<_>>(),
            vec![[1.0, 1.0, 2.0], [3.0, 1.0, 2.0], [4.0, 1.0, 2.0]]
        );
    }

    #[test]
    fn gameplay_hud_uses_the_shared_letterboxed_virtual_mapping() {
        let mapping = UiMapping::new(UiMappingRequest {
            viewport: [1280, 720],
            authored_canvas: [(320.0) as u32, (240.0) as u32],
            policy: UiSubmissionPolicy::FitAuthoredCanvas,
        });
        assert_eq!(
            mapping,
            UiMapping {
                scale: 3.0,
                offset_x: 160,
                offset_y: 0
            }
        );
        assert_eq!(
            map_gameplay_hud_rect(7, 165, 62, 61, mapping),
            (181, 495, 186, 183)
        );
        assert_eq!(
            map_gameplay_hud_clip(
                GameplayHudClip {
                    x: 0,
                    y: 171,
                    width: 320,
                    height: 69,
                },
                mapping,
            ),
            (160, 513, 960, 207)
        );
    }

    #[test]
    fn gameplay_hud_weapon_projection_tracks_authored_point_and_native_pillarbox() {
        let point = [44.0, 198.0];
        let four_three = menu_camera_for_authored_point(
            point,
            UiMappingRequest {
                viewport: [320, 240],
                authored_canvas: [320, 240],
                policy: UiSubmissionPolicy::FitAuthoredCanvas,
            },
        );
        assert!((four_three.projection_offset[0] - 0.725).abs() < 1.0e-6);
        assert!((four_three.projection_offset[1] - 0.65).abs() < 1.0e-6);

        let native_widescreen = menu_camera_for_authored_point(
            point,
            UiMappingRequest {
                viewport: [1920, 1080],
                authored_canvas: [320, 240],
                policy: UiSubmissionPolicy::FitAuthoredCanvas,
            },
        );
        assert!((native_widescreen.projection_offset[0] - 0.54375).abs() < 1.0e-6);
        assert!((native_widescreen.projection_offset[1] - 0.65).abs() < 1.0e-6);
        assert_eq!(four_three.fov, menu_camera(4.0 / 3.0).fov);
        assert_eq!(native_widescreen.fov, menu_camera(1920.0 / 1080.0).fov);
    }

    #[test]
    fn high_native_hud_models_share_sprite_anchor_and_authored_pixel_focal_length() {
        for authored_canvas in [[640, 480], [800, 600], [1024, 768]] {
            for viewport in [authored_canvas, [1920, 1080], [320, 240], [3840, 2160]] {
                let request = UiMappingRequest {
                    viewport,
                    authored_canvas,
                    policy: UiSubmissionPolicy::NativeCanvas.for_gameplay_hud(UiAnchor::BottomLeft),
                };
                let point = v2k_game::gameplay_hud::GameplayHudPoint {
                    x: authored_canvas[0] as i32 / 4,
                    y: authored_canvas[1] as i32 * 3 / 4,
                };
                let mapping = UiMapping::new(request);
                let camera =
                    menu_camera_for_authored_point([point.x as f32, point.y as f32], request);
                let projection = camera.projection_matrix();
                let focal_y_pixels = projection[5] * viewport[1] as f32 * 0.5;
                let authored_focal_y_pixels = menu_camera(4.0 / 3.0).projection_matrix()[5]
                    * authored_canvas[1] as f32
                    * mapping.scale
                    * 0.5;
                assert!((focal_y_pixels - authored_focal_y_pixels).abs() < 0.001);
                // Viewport aspect also keeps the horizontal pixel focal equal
                // to the vertical one, so arbitrary articulated models retain
                // the same native footprint rather than only a fitted anchor.
                assert!((projection[0] * viewport[0] as f32 * 0.5 - focal_y_pixels).abs() < 0.001);
                let projected_origin = [
                    (1.0 - camera.projection_offset[0]) * viewport[0] as f32 * 0.5,
                    (1.0 + camera.projection_offset[1]) * viewport[1] as f32 * 0.5,
                ];
                let mapped_origin = [
                    mapping.offset_x as f32 + point.x as f32 * mapping.scale,
                    mapping.offset_y as f32 + point.y as f32 * mapping.scale,
                ];
                assert!((projected_origin[0] - mapped_origin[0]).abs() < 0.001);
                assert!((projected_origin[1] - mapped_origin[1]).abs() < 0.001);
            }
        }
    }

    #[v2k_test_support::retail_test]
    fn native_hud_radar_and_weapon_keep_their_authored_relationship_on_modern_outputs() {
        use v2k_game::gameplay_radar::GameplayRadarLayout;

        let data = v2k_test_support::retail_dir();
        let mut session =
            v2k_game::session::GameSession::init(&data).expect("canonical retail preload required");
        for variant in 1..=3 {
            session.load_auxiliary_ovl(3, variant).unwrap();
            let hud = GameplayHudLayout::from_cache(&session.cache, variant).unwrap();
            let radar = GameplayRadarLayout::from_cache(&session.cache, variant).unwrap();
            let authored_canvas = [hud.virtual_width, hud.virtual_height];
            assert_eq!(radar.virtual_size, authored_canvas);
            assert_eq!(radar.globe_size, [96, 96]);
            let point = [
                (hud.base.x + hud.weapon_model.x) as f32,
                (hud.base.y + hud.weapon_model.y) as f32,
            ];
            for (viewport, expected_scale) in [
                (authored_canvas, 1.0),
                ([1920, 1080], 1.8),
                ([2560, 1440], 2.4),
                ([3840, 2160], 3.6),
                (authored_canvas, 1.0),
            ] {
                let request = UiMappingRequest {
                    viewport,
                    authored_canvas,
                    policy: UiSubmissionPolicy::NativeCanvas.for_gameplay_hud(UiAnchor::BottomLeft),
                };
                let mapping = UiMapping::new(request);
                assert_eq!(mapping.scale, expected_scale);
                let radar_mapping = UiMapping::new(UiMappingRequest {
                    policy: UiSubmissionPolicy::NativeCanvas
                        .for_gameplay_hud(UiAnchor::BottomRight),
                    ..request
                });
                let globe = map_gameplay_hud_rect(
                    radar.hud_origin[0],
                    radar.hud_origin[1],
                    96,
                    96,
                    radar_mapping,
                );
                for pixels in [globe.2, globe.3] {
                    assert!((pixels as f32 - 96.0 * expected_scale).abs() < 1.0);
                }
                let expected_right_inset =
                    (authored_canvas[0] as i32 - radar.hud_origin[0] - 96) as f32 * expected_scale;
                let expected_bottom_inset =
                    (authored_canvas[1] as i32 - radar.hud_origin[1] - 96) as f32 * expected_scale;
                assert!(
                    (viewport[0] as f32 - (globe.0 + globe.2 as i32) as f32 - expected_right_inset)
                        .abs()
                        < 1.0
                );
                assert!(
                    (viewport[1] as f32
                        - (globe.1 + globe.3 as i32) as f32
                        - expected_bottom_inset)
                        .abs()
                        < 1.0
                );
                let camera = menu_camera_for_authored_point(point, request);
                let projection = camera.projection_matrix();
                let projected_weapon_origin = [
                    (1.0 - camera.projection_offset[0]) * viewport[0] as f32 * 0.5,
                    (1.0 + camera.projection_offset[1]) * viewport[1] as f32 * 0.5,
                ];
                let sprite_weapon_origin = [
                    mapping.offset_x as f32 + point[0] * expected_scale,
                    mapping.offset_y as f32 + point[1] * expected_scale,
                ];
                for (actual, expected) in projected_weapon_origin
                    .into_iter()
                    .zip(sprite_weapon_origin)
                {
                    assert!((actual - expected).abs() < 0.001);
                }
                let expected_focal = authored_canvas[1] as f32 * expected_scale * 0.5
                    / (50.4_f32.to_radians() * 0.5).tan();
                assert!((projection[5] * viewport[1] as f32 * 0.5 - expected_focal).abs() < 0.001);
                // The modal keeps a separate centred fit. Its entire map,
                // icon clips and status panel must remain on the drawable even
                // when the corner HUD outgrows its unused authored canvas.
                let modal_map = map_gameplay_hud_rect(
                    radar.map_rect.x,
                    radar.map_rect.y,
                    radar.map_rect.width,
                    radar.map_rect.height,
                    UiMapping::new(UiMappingRequest {
                        policy: UiSubmissionPolicy::FitAuthoredCanvas,
                        ..request
                    }),
                );
                assert!(modal_map.0 >= 0 && modal_map.1 >= 0);
                assert!(modal_map.0 as u32 + modal_map.2 <= viewport[0]);
                assert!(modal_map.1 as u32 + modal_map.3 <= viewport[1]);
            }
        }
    }

    #[test]
    fn blocking_load_time_is_not_charged_to_the_next_simulation_frame() {
        let mut last_frame = Instant::now() - Duration::from_secs(5);
        reset_frame_clock_after_blocking_load(&mut last_frame);
        assert!(last_frame.elapsed() < Duration::from_millis(50));
    }

    #[test]
    fn pause_presentation_uses_frozen_world_without_frontend_hangar_layers() {
        assert!(MenuPresentation::GameplayPause.uses_frozen_world());
        assert!(!MenuPresentation::GameplayPause.uses_frontend_hangar());
        assert!(MenuPresentation::Frontend.uses_frontend_hangar());
        assert!(!MenuPresentation::Frontend.uses_frozen_world());
    }

    #[test]
    fn options_panel_turns_its_authored_front_toward_the_menu_camera() {
        let orientation = menu_model_orientation(MENU_OPTIONS_PANEL_ROTATION_Y);
        let authored_front = mat3_apply(orientation, [0.0, 0.0, -1.0]);
        assert!(authored_front[0].abs() < 1.0e-6);
        assert!(authored_front[1].abs() < 1.0e-6);
        assert!(
            authored_front[2] > 0.999,
            "optionsh's decorated -Z side must face the camera at the origin"
        );
    }

    #[test]
    fn menu_model_basis_reflects_local_x() {
        let rotation = std::f32::consts::FRAC_PI_4;
        let orientation = menu_model_orientation(rotation);
        let expected = mat3_apply(orientation_from_ypr(rotation, 0.0, 0.0), [-1.0, 0.0, 0.0]);
        let actual = mat3_apply(orientation, [1.0, 0.0, 0.0]);
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() < 1.0e-6);
        }
    }

    #[test]
    fn gameplay_escape_requests_pause_not_world_teardown() {
        let events = [GameEvent::KeyDown(Keycode::Escape)];

        assert_eq!(
            gameplay_interruption(false, &events),
            Some(GameplayInterruption::Pause)
        );
        assert_eq!(
            gameplay_interruption(true, &events),
            Some(GameplayInterruption::ReturnToFrontend),
            "a completed death lifecycle must retain priority over pause"
        );
        assert_eq!(gameplay_interruption(false, &[]), None);
        assert_eq!(
            gameplay_interruption(false, &[GameEvent::KeyDown(Keycode::P)]),
            None,
            "P belongs to the separate retail results-screen table"
        );
    }

    #[test]
    fn software_ring_fly_uses_the_same_depth_offsets_as_the_model_path() {
        let selected = menu_depth_scale(2300.0, -896.0);
        let other = menu_depth_scale(3500.0, 7168.0);
        assert!(selected > 1.0, "selected prop must approach the lens");
        assert!(other < 1.0, "other props must recede");
        assert_eq!(menu_depth_scale(2300.0, 0.0), 1.0);
    }

    #[test]
    fn cd_audio_prefers_the_repository_root_and_ignores_empty_shadow_directories() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before Unix epoch")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("v2k-cdaudio-lookup-{}-{nonce}", std::process::id()));
        let port = root.join("port");
        let root_audio = root.join("cdaudio");
        std::fs::create_dir_all(port.join("cdaudio")).expect("create empty shadow directory");
        std::fs::create_dir_all(&root_audio).expect("create root CD-audio directory");
        let wave = b"RIFF\x28\0\0\0WAVEfmt \x10\0\0\0\x01\0\x02\0\x44\xAC\0\0\x10\xB1\x02\0\x04\0\x10\0data\x04\0\0\0\x01\0\x02\0";
        std::fs::write(root_audio.join("track02.wav"), wave).expect("create valid PCM test track");

        let found = find_cd_audio_dir(&port).expect("root CD audio should be found");
        assert_eq!(found, std::fs::canonicalize(&root_audio).unwrap());

        std::fs::remove_dir_all(root).expect("remove CD-audio test tree");
    }

    #[test]
    fn cd_audio_lookup_rejects_directories_without_valid_cd_audio() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before Unix epoch")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("v2k-cdaudio-empty-{}-{nonce}", std::process::id()));
        let port = root.join("port");
        std::fs::create_dir_all(port.join("music")).expect("create legacy music directory");
        std::fs::write(port.join("music").join("README.txt"), b"not a track")
            .expect("create non-track file");
        std::fs::write(port.join("music/track02.ogg"), []).expect("create invalid track");

        assert_eq!(find_cd_audio_dir(&port), None);

        std::fs::remove_dir_all(root).expect("remove empty CD-audio test tree");
    }

    #[test]
    fn cargo_keys_are_edge_triggered_and_only_left_shift_suppresses_them() {
        let mut keys = HashSet::new();
        let cargo = |collect, drop| GameplayInputEdges {
            collect,
            drop,
            weapon_cycle: None,
        };

        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::C, true),
            cargo(true, false)
        );
        assert!(keys.contains(&Keycode::C));
        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::C, true),
            GameplayInputEdges::default(),
            "a held cargo key must not queue again"
        );
        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::C, false),
            GameplayInputEdges::default()
        );

        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::LShift, true),
            GameplayInputEdges {
                weapon_cycle: Some(WeaponCycleDirection::Forward),
                ..GameplayInputEdges::default()
            }
        );
        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::C, true),
            GameplayInputEdges::default(),
            "Left Shift suppresses C while it is held"
        );
        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::LShift, false),
            cargo(true, false),
            "releasing Left Shift while C is held activates collect"
        );
        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::LShift, true),
            GameplayInputEdges {
                weapon_cycle: Some(WeaponCycleDirection::Forward),
                ..GameplayInputEdges::default()
            },
            "pressing Left Shift deactivates collect without queuing"
        );
        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::LShift, false),
            cargo(true, false),
            "a second Left Shift release is another false-to-true edge"
        );
        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::C, false),
            GameplayInputEdges::default()
        );

        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::D, true),
            cargo(false, true)
        );
        assert!(keys.contains(&Keycode::D));
        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::D, false),
            GameplayInputEdges::default()
        );

        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::RShift, true),
            GameplayInputEdges::default()
        );
        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::D, true),
            cargo(false, true),
            "Right Shift is irrelevant to the retail cargo bindings"
        );
    }

    #[test]
    fn weapon_cycle_aliases_share_one_logical_edge_in_each_direction() {
        let mut keys = HashSet::new();
        let cycle = |direction| GameplayInputEdges {
            weapon_cycle: Some(direction),
            ..GameplayInputEdges::default()
        };

        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::B, true),
            cycle(WeaponCycleDirection::Forward)
        );
        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::PageDown, true),
            GameplayInputEdges::default(),
            "a second forward alias must not cycle while the logical action is held"
        );
        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::B, false),
            GameplayInputEdges::default(),
            "releasing only one of two held aliases keeps the action held"
        );
        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::PageDown, false),
            GameplayInputEdges::default()
        );

        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::V, true),
            cycle(WeaponCycleDirection::Backward)
        );
        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::V, true),
            GameplayInputEdges::default(),
            "key repeat must not create another cycle"
        );
        assert_eq!(
            gameplay_key_transition(&mut keys, Keycode::V, false),
            GameplayInputEdges::default()
        );
    }

    #[test]
    fn debug_ride_surface_matches_the_cargo_mass_branch() {
        assert!(debug_ride_uses_wave_surface(true, 0));
        assert!(debug_ride_uses_wave_surface(true, 99));
        assert!(!debug_ride_uses_wave_surface(true, 100));
        assert!(!debug_ride_uses_wave_surface(false, 0));
    }

    #[test]
    fn particle_draw_scale_matches_retail_projection_algebra() {
        assert_eq!(particle_sprite_world_dimension(16, 0x0500, 0x0100), 0.3125);
        assert_eq!(particle_sprite_world_dimension(32, 0x0f00, 0x0100), 1.875);
    }

    #[test]
    fn particle_painter_key_adds_the_signed_descriptor_bias() {
        assert_eq!(particle_sprite_sort_key_raw(1_000, 5), 900);
        assert_eq!(particle_sprite_sort_key_raw(1_000, 19), 1_256);
        assert_eq!(particle_sprite_sort_key_raw(1_000, 47), 700);
        assert_eq!(particle_sprite_sort_key_raw(1_000, 16), 1_000);
    }

    #[test]
    fn intro2_world_camera_keeps_authored_positive_x_on_screen_right() {
        let mut camera = Camera::new(4.0 / 3.0);
        camera.position = [0.0, 0.0, -8.0];
        camera.yaw = std::f32::consts::PI;
        camera.pitch = 0.0;

        configure_opening_world_camera(&mut camera);

        let view = camera.view_matrix();
        let authored_positive_x = [1.0_f32, 0.0, 0.0];
        let view_x = view[0] * authored_positive_x[0]
            + view[4] * authored_positive_x[1]
            + view[8] * authored_positive_x[2]
            + view[12];
        assert!(camera.left_handed);
        assert!(view_x > 0.0, "Intro2 +X must project to screen-right");
    }

    #[test]
    fn gameplay_fog_uses_authored_far_width_without_scan_cap() {
        assert_eq!(gameplay_fog_planes(21, 8), (13.0, 21.0));
        assert_eq!(gameplay_fog_planes(22, 6), (16.0, 22.0)); // Medaeval
        assert_eq!(gameplay_fog_planes(22, 10), (12.0, 22.0)); // world15
        assert_eq!(gameplay_fog_planes(29, 5), (24.0, 29.0)); // world31
        assert_eq!(gameplay_fog_planes(30, 29), (1.0, 30.0)); // Arena4
        assert_eq!(gameplay_fog_planes(40, 12), (28.0, 40.0)); // controlled uncapped input
    }

    #[test]
    fn player_child_policy_preserves_authored_mode_mount_and_spins_blade_locally() {
        let identity = orientation_from_ypr(0.0, 0.0, 0.0);
        let mut craft = PlayerCraft::new();
        craft.spin_angle = std::f32::consts::FRAC_PI_2;
        let policy = PlayerCraftChildTransform { craft: &craft };

        // Callback word 6 has already produced this mount orientation during
        // materialization; the player-only policy must not rotate it again.
        let authored_mode_mount =
            rotation_about_axis([1.0, 0.0, 0.0], -std::f32::consts::FRAC_PI_2);
        let assembly = policy.transform(1, Some("pl4enginesurround"), authored_mode_mount);
        assert_eq!(assembly, authored_mode_mount);

        let blade = policy.transform(2, Some("pl4engine"), identity);
        let blade_forward = mat3_apply(blade, [0.0, 0.0, 1.0]);
        assert!((blade_forward[0] - 1.0).abs() < 1.0e-6);
        assert!(blade_forward[1].abs() < 1.0e-6);
        assert!(blade_forward[2].abs() < 1.0e-6);
    }

    #[v2k_test_support::retail_test]
    fn authored_player_mounts_use_the_raw_barrel_without_a_second_child_rotation() {
        let root = v2k_test_support::retail_dir();
        let mut session =
            v2k_game::session::GameSession::init(&root).expect("retail corpus required");
        session.load_auxiliary_ovl(3, 1).unwrap();
        let model_id = session.cache.global_model_by_name("player4").unwrap().0;
        let mut craft = PlayerCraft::new();
        craft.gun_barrel = 0x3000 as f32;
        for callback_selector in [0, 4, 6, 7, 8, 15] {
            craft.select_weapon_callback(callback_selector as u8);
            let mounts = resolve_player_primary_gun_mounts(
                &session.cache,
                model_id,
                &craft,
                [0.0; 3],
                std::f32::consts::FRAC_PI_2,
            );
            let PrimaryGunMounts::Authored(mounts) = mounts else {
                panic!("missing authored pair {callback_selector}")
            };
            let expected = craft.primary_fire_direction(std::f32::consts::FRAC_PI_2);
            for mount in [mounts.gun_a, mounts.gun_b] {
                for axis in 0..3 {
                    assert!(
                        (mount.direction_unit[axis] - expected[axis]).abs() < 0.0001,
                        "selector{callback_selector} axis{axis}"
                    );
                }
            }
        }
    }

    #[test]
    fn level_one_primary_callbacks_select_their_authored_mount_pairs() {
        assert_eq!(
            primary_gun_model_name(0),
            Some(DEFAULT_PRIMARY_GUN_MODEL_NAME)
        );
        assert_eq!(
            primary_gun_model_name(4),
            Some(UPGRADED_PRIMARY_GUN_MODEL_NAME)
        );
        assert_eq!(primary_gun_model_name(6), Some(PLASMA_RED_GUN_MODEL_NAME));
        assert_eq!(primary_gun_model_name(7), Some(PLASMA_GREEN_GUN_MODEL_NAME));
        assert_eq!(primary_gun_model_name(8), Some(PLASMA_BLUE_GUN_MODEL_NAME));
        assert_eq!(primary_gun_model_name(1), None);
        assert_eq!(
            primary_gun_model_name(15),
            Some(RAPID_PRIMARY_GUN_MODEL_NAME)
        );
        assert_eq!(primary_gun_model_name(12), None);
    }

    #[test]
    fn meteor_sprite_flags_preserve_three_retail_material_families() {
        let (blend, color) = particle_sprite_material(0x05);
        assert_eq!(blend, WorldSpriteBlend::Masked);
        assert_eq!(color, [1.0; 4]);

        let (blend, color) = particle_sprite_material(0x15);
        assert_eq!(blend, WorldSpriteBlend::Additive);
        assert_eq!(color, [1.0; 4]);

        let (blend, color) = particle_sprite_material(0x0d);
        assert_eq!(blend, WorldSpriteBlend::HalfAdditive);
        assert_eq!(color, [1.0, 1.0, 1.0, HALF_ADDITIVE_ALPHA]);
    }

    #[test]
    fn particle_sprite_fog_retains_boundaries_and_context_reciprocals() {
        let gameplay = ParticleSpriteFogPlanes::gameplay(21, 8);
        assert_eq!(gameplay.at_depth_raw(13 * 256 - 1), Some(SpriteFog::Near));
        assert_eq!(
            gameplay.at_depth_raw(13 * 256),
            Some(SpriteFog::Far { fade_byte: 0 })
        );
        assert_eq!(
            gameplay.at_depth_raw(17 * 256),
            Some(SpriteFog::Far { fade_byte: 128 })
        );
        assert_eq!(
            gameplay.at_depth_raw(21 * 256 - 1),
            Some(SpriteFog::Far { fade_byte: 255 })
        );
        assert_eq!(gameplay.at_depth_raw(21 * 256), None);
        assert_eq!(gameplay.at_depth_raw(22 * 256), None);

        let intro = ParticleSpriteFogPlanes::INTRO2;
        assert_eq!(intro.at_depth_raw(12 * 256 - 1), Some(SpriteFog::Near));
        assert_eq!(
            intro.at_depth_raw(12 * 256),
            Some(SpriteFog::Far { fade_byte: 0 })
        );
        assert_eq!(
            intro.at_depth_raw(18 * 256),
            Some(SpriteFog::Far { fade_byte: 127 })
        );
        assert_eq!(
            intro.at_depth_raw(24 * 256 - 1),
            Some(SpriteFog::Far { fade_byte: 255 })
        );
        assert_eq!(intro.at_depth_raw(24 * 256), None);
        assert_eq!(
            gameplay.at_depth_raw(18 * 256),
            Some(SpriteFog::Far { fade_byte: 160 })
        );
        assert_eq!(
            ParticleSpriteFogPlanes::gameplay(40, 8).at_depth_raw(32 * 256 - 1),
            Some(SpriteFog::Near)
        );
    }

    #[test]
    fn particle_sprite_fog_uses_medaeval_authored_width() {
        let planes = ParticleSpriteFogPlanes::gameplay(22, 6);
        assert_eq!((planes.near_raw, planes.far_raw), (4096, 5632));
        assert_eq!(planes.at_depth_raw(14 * 256), Some(SpriteFog::Near));
        assert_eq!(
            planes.at_depth_raw(16 * 256),
            Some(SpriteFog::Far { fade_byte: 0 })
        );
        assert_eq!(
            planes.at_depth_raw(19 * 256),
            Some(SpriteFog::Far { fade_byte: 127 })
        );
        assert_eq!(planes.at_depth_raw(22 * 256), None);
        assert_eq!(
            (
                ParticleSpriteFogPlanes::INTRO2.near_raw,
                ParticleSpriteFogPlanes::INTRO2.far_raw
            ),
            (3072, 6144)
        );
    }

    #[test]
    fn particle_sprite_fog_uses_projected_center_depth_off_axis() {
        let mut camera = Camera::new(4.0 / 3.0);
        camera.position = [0.0; 3];
        camera.pitch = 0.0;
        for left_handed in [false, true] {
            camera.left_handed = left_handed;
            let center = project_particle_center(&camera, [640, 480], [0.0, 0.0, -18.0]);
            let off_axis = project_particle_center(&camera, [640, 480], [9.0, 3.0, -18.0]);
            assert_ne!(center.screen, off_axis.screen);
            assert_eq!(center.depth_raw, off_axis.depth_raw);
            for planes in [
                ParticleSpriteFogPlanes::INTRO2,
                ParticleSpriteFogPlanes::gameplay(21, 8),
            ] {
                assert_eq!(
                    planes.at_depth_raw(center.depth_raw),
                    planes.at_depth_raw(off_axis.depth_raw)
                );
            }
        }
    }

    #[test]
    fn non_player_model_basis_transposes_retail_column_vectors() {
        let at_zero = non_player_model_orientation(0.0);
        let local_forward = mat3_apply(at_zero, [0.0, 0.0, 1.0]);
        assert!((local_forward[0] - 1.0).abs() < 1.0e-6);
        assert!(local_forward[1].abs() < 1.0e-6);
        assert!(local_forward[2].abs() < 1.0e-6);

        let at_quarter_turn = non_player_model_orientation(std::f32::consts::FRAC_PI_2);
        let local_forward = mat3_apply(at_quarter_turn, [0.0, 0.0, 1.0]);
        assert!(local_forward[0].abs() < 1.0e-6);
        assert!(local_forward[1].abs() < 1.0e-6);
        assert!((local_forward[2] - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn unresolved_non_player_body_basis_preserves_yaw_policy() {
        let heading = 0.37;
        assert_eq!(
            non_player_entity_orientation(heading, RetailRuntimeValue::Unresolved),
            non_player_model_orientation(heading)
        );
    }

    #[test]
    fn staged_effect_request_uses_the_same_wrapped_entity_basis() {
        let request = base_factory_staged_effect_request([123, -456, 789], 0.0);
        assert_eq!(request.model_origin_raw, [123.0, -456.0, 789.0]);
        let local_forward = [
            request.model_to_output_basis[0][2],
            request.model_to_output_basis[1][2],
            request.model_to_output_basis[2][2],
        ];
        assert!((local_forward[0] - 1.0).abs() < 1.0e-6);
        assert!(local_forward[1].abs() < 1.0e-6);
        assert!(local_forward[2].abs() < 1.0e-6);

        let quarter =
            base_factory_staged_effect_request([-32_000, 7, 32_000], std::f32::consts::FRAC_PI_2);
        assert_eq!(quarter.model_origin_raw, [-32_000.0, 7.0, 32_000.0]);
        assert!((quarter.model_to_output_basis[0][0] - 1.0).abs() < 1.0e-6);
        assert!((quarter.model_to_output_basis[1][1] - 1.0).abs() < 1.0e-6);
        assert!((quarter.model_to_output_basis[2][2] - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn flat_entity_billboard_faces_camera_and_stays_upright() {
        let camera = [8.0, 9.0, -2.0];
        let entity = [2.0, 1.0, 3.0];
        let orientation = camera_facing_entity_orientation(camera, entity, false);
        let normal = mat3_apply(orientation, [0.0, 0.0, 1.0]);
        let expected = [camera[0] - entity[0], 0.0, camera[2] - entity[2]];
        let length = (expected[0] * expected[0] + expected[2] * expected[2]).sqrt();

        assert!((normal[0] - expected[0] / length).abs() < 1.0e-6);
        assert!(normal[1].abs() < 1.0e-6);
        assert!((normal[2] - expected[2] / length).abs() < 1.0e-6);
        assert_eq!(mat3_apply(orientation, [0.0, 1.0, 0.0]), [0.0, 1.0, 0.0]);
    }

    #[test]
    fn left_handed_flat_actor_puts_sprite_u_on_screen_right() {
        let camera = [0.0, 0.0, -8.0];
        let entity = [0.0, 0.0, 0.0];
        let orientation = camera_facing_entity_orientation(camera, entity, true);
        let local_x = mat3_apply(orientation, [1.0, 0.0, 0.0]);
        assert!(
            local_x[0] > 0.999,
            "left-handed world +X is screen-right; sprite U is local +X"
        );
        assert!(local_x[1].abs() < 1.0e-6);
        assert!(local_x[2].abs() < 1.0e-6);

        let uncorrected = camera_facing_entity_orientation(camera, entity, false);
        let flipped = mat3_apply(uncorrected, [1.0, 0.0, 0.0]);
        assert!(
            flipped[0] < -0.999,
            "Ry(atan2) alone puts sprite U on screen-left under this view"
        );
    }

    #[test]
    fn menu_row_depth_fade_resets_from_the_live_plane_for_each_prop() {
        for (live_near_raw, expected_near_raw) in [
            (0x500 as f32, 1276),
            (0x800 as f32, 2044),
            (0xC00 as f32, 2812),
        ] {
            let ModelDepthFade::FrontendFixedPointLinear {
                near_raw,
                far_raw,
                color,
            } = menu_prop_row_depth_fade(live_near_raw, 2300.0, MENU_DEPTH_FADE_COLOR_FALLBACK)
            else {
                panic!("menu prop must retain the retail fixed-point fade domain");
            };
            assert_eq!(near_raw, expected_near_raw);
            assert_eq!(far_raw, 2812);
            assert_eq!(color, MENU_DEPTH_FADE_COLOR_FALLBACK);
        }

        let ModelDepthFade::FrontendFixedPointLinear {
            near_raw, far_raw, ..
        } = menu_prop_row_depth_fade(0x500 as f32, 2900.0, MENU_DEPTH_FADE_COLOR_FALLBACK)
        else {
            panic!("second menu prop must install an independent band");
        };
        assert_eq!(near_raw, 1876);
        assert_eq!(far_raw, 3412);

        let panel_raw = MENU_SCREEN_FADE_RAW + 2900.0;
        let ModelDepthFade::FrontendFixedPointLinear {
            near_raw, far_raw, ..
        } = menu_depth_fade_from_raw(panel_raw, panel_raw, MENU_DEPTH_FADE_COLOR_FALLBACK)
        else {
            panic!("optionsh must install its authored equal-plane step");
        };
        assert_eq!(near_raw, 10580);
        assert_eq!(near_raw, far_raw);

        let ModelDepthFade::FrontendFixedPointLinear {
            near_raw, far_raw, ..
        } = menu_prop_row_depth_fade(0x800 as f32, 2500.0, MENU_DEPTH_FADE_COLOR_FALLBACK)
        else {
            panic!("submenu decoration must reset after optionsh");
        };
        assert_eq!(near_raw, 2244);
        assert_eq!(far_raw, 3012);

        let ModelDepthFade::FrontendFixedPointLinear {
            near_raw,
            far_raw,
            color,
            ..
        } = menu_prop_row_depth_fade(
            MENU_PROP_ROW_NEUTRAL_NEAR_RAW,
            2500.0,
            MENU_PAUSE_DEPTH_FADE_COLOR,
        )
        else {
            panic!("pause decoration must retain a neutral row band");
        };
        assert_eq!(near_raw, 3012);
        assert_eq!(near_raw, far_raw);
        assert_eq!(color, MENU_PAUSE_DEPTH_FADE_COLOR);
    }

    #[test]
    fn frontend_frame_keeps_pre_advance_scene_and_post_pin_rows() {
        let mut anim = BillboardAnim::new();

        let (first, wrapped) = anim.prepare_frontend_render_frame(
            16_000,
            FrontendDepthFadeMode::RingPulse,
            FrontendDepthFadeMode::RingPulse,
        );
        assert!(wrapped);
        assert_eq!(first.sprite_id, BILLBOARD_FRAMES[8].0);
        assert_eq!(first.scene_depth_fade_near_raw, 0xC00 as f32);
        assert_eq!(first.scene_depth_fade_far_raw, 0x13F0 as f32);
        assert_eq!(first.row_depth_fade_near_raw, 0x500 as f32);
        assert_eq!(anim.frame, 0);
        assert_eq!(anim.accum_micros, 0);

        let (second, wrapped) = anim.prepare_frontend_render_frame(
            16_000,
            FrontendDepthFadeMode::RingPulse,
            FrontendDepthFadeMode::RingPulse,
        );
        assert!(!wrapped);
        assert_eq!(second.sprite_id, BILLBOARD_FRAMES[0].0);
        assert_eq!(second.scene_depth_fade_near_raw, 0x550 as f32);
        assert_eq!(second.row_depth_fade_near_raw, 0x550 as f32);
        assert_eq!(anim.frame, 1, "zero-duration flash advances after drawing");

        anim.frame = 3;
        anim.accum_micros = BILLBOARD_FRAMES[3].1;
        anim.depth_fade_near_raw = 0xC00;
        anim.depth_fade_far_raw = 0x1400;
        let (attack, wrapped) = anim.prepare_frontend_render_frame(
            1_000,
            FrontendDepthFadeMode::RingPulse,
            FrontendDepthFadeMode::RingPulse,
        );
        assert!(!wrapped);
        assert_eq!(attack.sprite_id, BILLBOARD_FRAMES[3].0);
        assert_eq!(attack.scene_depth_fade_near_raw, 0xC00 as f32);
        assert_eq!(attack.scene_depth_fade_far_raw, 0x13FF as f32);
        assert_eq!(attack.row_depth_fade_near_raw, 0x800 as f32);
        assert_eq!(anim.frame, 4);
        assert_eq!(anim.accum_micros, 1_000);

        let (held_attack, wrapped) = anim.prepare_frontend_render_frame(
            16_000,
            FrontendDepthFadeMode::RingPulse,
            FrontendDepthFadeMode::RingPulse,
        );
        assert!(!wrapped);
        assert_eq!(held_attack.sprite_id, BILLBOARD_FRAMES[4].0);
        assert_eq!(held_attack.scene_depth_fade_near_raw, 0x850 as f32);
        assert_eq!(held_attack.row_depth_fade_near_raw, 0x800 as f32);
    }

    #[test]
    fn frontend_depth_fade_truncates_each_microsecond_step_like_retail() {
        let mut anim = BillboardAnim::new();
        anim.frame = 1;
        anim.accum_micros = 0;
        anim.depth_fade_near_raw = 0x500;
        anim.depth_fade_far_raw = 0x1400;

        let (first, wrapped) = anim.prepare_frontend_render_frame(
            16_667,
            FrontendDepthFadeMode::RingPulse,
            FrontendDepthFadeMode::RingPulse,
        );
        assert!(!wrapped);
        assert_eq!(first.scene_depth_fade_near_raw, (0x500 + 83) as f32);
        assert_eq!(first.scene_depth_fade_far_raw, (0x1400 - 16) as f32);

        let (second, wrapped) = anim.prepare_frontend_render_frame(
            16_667,
            FrontendDepthFadeMode::RingPulse,
            FrontendDepthFadeMode::RingPulse,
        );
        assert!(!wrapped);
        assert_eq!(second.scene_depth_fade_near_raw, (0x500 + 166) as f32);
        assert_eq!(second.scene_depth_fade_far_raw, (0x1400 - 32) as f32);
    }

    #[test]
    fn submenu_depth_fade_recedes_and_keeps_both_billboard_attacks_off_the_prop() {
        let mut anim = BillboardAnim::new();
        anim.depth_fade_near_raw = 0x500;
        let mut saw_attack = [false; 2];
        for tick in 0..150 {
            let (frame, _) = anim.prepare_frontend_render_frame(
                20_000,
                FrontendDepthFadeMode::Receded,
                FrontendDepthFadeMode::Receded,
            );
            if tick >= 32 {
                assert_eq!(frame.scene_depth_fade_far_raw, 0x1900 as f32);
                let ModelDepthFade::FrontendFixedPointLinear {
                    near_raw, far_raw, ..
                } = menu_prop_row_depth_fade(
                    frame.row_depth_fade_near_raw,
                    2500.0,
                    MENU_DEPTH_FADE_COLOR_FALLBACK,
                )
                else {
                    panic!("submenu keeps the shared row fade policy")
                };
                assert_eq!(near_raw, 3012);
                assert_eq!(near_raw, far_raw);
                match anim.frame {
                    0 => {
                        saw_attack[0] = true;
                        assert_eq!(frame.row_depth_fade_near_raw, 0xF00 as f32);
                    }
                    4 => {
                        saw_attack[1] = true;
                        assert_eq!(frame.row_depth_fade_near_raw, 0x1200 as f32);
                    }
                    _ => {}
                }
            }
        }
        assert_eq!(saw_attack, [true, true]);
    }

    #[test]
    fn frontend_fade_samples_command_transition_on_each_side_of_actor_update() {
        // Entering a submenu evolves the old ring state, then clamps using
        // Selected=4 after C090 consumes command 4. Returning does the reverse.
        for (before, after, expected_far, expected_row_near) in [
            (
                FrontendDepthFadeMode::RingPulse,
                FrontendDepthFadeMode::Receded,
                0x13F0,
                0xC00,
            ),
            (
                FrontendDepthFadeMode::Receded,
                FrontendDepthFadeMode::RingPulse,
                0x1450,
                0x500,
            ),
        ] {
            let mut anim = BillboardAnim::new();
            anim.depth_fade_near_raw = 0xC00;
            let (frame, wrapped) = anim.prepare_frontend_render_frame(16_000, before, after);
            assert!(wrapped);
            assert_eq!(frame.scene_depth_fade_far_raw, expected_far as f32);
            assert_eq!(frame.row_depth_fade_near_raw, expected_row_near as f32);
        }
    }

    #[test]
    fn billboard_advance_keeps_retail_strict_duration_boundary() {
        let mut anim = BillboardAnim::new();
        anim.frame = 1;
        anim.accum_micros = 0;

        assert!(!anim.advance(BILLBOARD_FRAMES[1].1));
        assert_eq!(anim.frame, 1);
        assert_eq!(anim.accum_micros, BILLBOARD_FRAMES[1].1);

        assert!(!anim.advance(1));
        assert_eq!(anim.frame, 2);
        assert_eq!(anim.accum_micros, 1);
    }

    #[test]
    fn cinematic_billboard_advance_does_not_touch_frontend_planes() {
        let mut anim = BillboardAnim::new();
        anim.frame = 3;
        anim.accum_micros = BILLBOARD_FRAMES[3].1;
        anim.depth_fade_near_raw = 0xC00;
        anim.depth_fade_far_raw = 0x1400;

        let (sprite_id, wrapped) = anim.prepare_cinematic_frame(1_000);
        assert!(!wrapped);
        assert_eq!(sprite_id, BILLBOARD_FRAMES[3].0);
        assert_eq!(anim.frame, 4);
        assert_eq!(anim.depth_fade_near_raw, 0xC00);
        assert_eq!(anim.depth_fade_far_raw, 0x1400);
    }

    #[test]
    fn menu_billboard_viewport_maps_completed_authored_rect_without_recentering() {
        let pose = MenuBillboardPose {
            zoom_raw: 0xffff,
            height_raw: 0,
        };
        let layout = MenuBillboardLayout {
            framebuffer: [640, 480],
            focal_y: 512,
            layout_y: 110,
            reference_size: [102, 129],
        };
        assert_eq!(
            menu_billboard_viewport_rect(
                UiMapping::new(UiMappingRequest {
                    viewport: [640, 480],
                    authored_canvas: layout.framebuffer.map(|dimension| dimension as u32),
                    policy: UiSubmissionPolicy::FitAuthoredCanvas,
                }),
                &layout,
                pose,
                [203, 254]
            ),
            Some(MenuBillboardRect {
                x: 218,
                y: 110,
                width: 206,
                height: 258
            })
        );
        // Explicit fitted enlargement preserves the authored one-pixel centre bias.
        assert_eq!(
            menu_billboard_viewport_rect(
                UiMapping::new(UiMappingRequest {
                    viewport: [1280, 720],
                    authored_canvas: layout.framebuffer.map(|dimension| dimension as u32),
                    policy: UiSubmissionPolicy::FitAuthoredCanvas,
                }),
                &layout,
                pose,
                [203, 254]
            ),
            Some(MenuBillboardRect {
                x: 487,
                y: 165,
                width: 309,
                height: 387
            })
        );
        assert_eq!(
            menu_billboard_viewport_rect(
                UiMapping::new(UiMappingRequest {
                    viewport: [0, 0],
                    authored_canvas: layout.framebuffer.map(|dimension| dimension as u32),
                    policy: UiSubmissionPolicy::FitAuthoredCanvas,
                }),
                &layout,
                pose,
                [203, 254]
            ),
            None
        );
    }

    #[test]
    fn high_native_billboard_scales_the_completed_tier_geometry_with_its_canvas() {
        let layout = MenuBillboardLayout {
            framebuffer: [640, 480],
            focal_y: 512,
            layout_y: 110,
            reference_size: [102, 129],
        };
        let mapping = UiMapping::new(UiMappingRequest {
            viewport: [1920, 1080],
            authored_canvas: [640, 480],
            policy: UiSubmissionPolicy::NativeCanvas,
        });
        assert_eq!(
            menu_billboard_viewport_rect(
                mapping,
                &layout,
                MenuBillboardPose {
                    zoom_raw: 0xffff,
                    height_raw: 0
                },
                [203, 254]
            ),
            Some(MenuBillboardRect {
                x: 776,
                y: 306,
                width: 371,
                height: 464
            })
        );
    }

    #[test]
    fn menu_billboard_viewport_retains_low_tier_integer_and_asset_differences() {
        let pose = MenuBillboardPose {
            zoom_raw: 0xffff,
            height_raw: 0,
        };
        let low = MenuBillboardLayout {
            framebuffer: [320, 240],
            focal_y: 256,
            layout_y: 55,
            reference_size: [51, 64],
        };
        assert_eq!(
            menu_billboard_viewport_rect(
                UiMapping::new(UiMappingRequest {
                    viewport: [640, 480],
                    authored_canvas: low.framebuffer.map(|dimension| dimension as u32),
                    policy: UiSubmissionPolicy::FitAuthoredCanvas,
                }),
                &low,
                pose,
                [102, 128]
            ),
            Some(MenuBillboardRect {
                x: 218,
                y: 110,
                width: 204,
                height: 258
            })
        );
    }

    #[test]
    fn submenu_projection_centers_keep_prop_below_panel() {
        let focal_y = 120.0 / (50.4_f32.to_radians() * 0.5).tan();
        let request = UiMappingRequest {
            viewport: [320, 240],
            authored_canvas: [320, 240],
            policy: UiSubmissionPolicy::FitAuthoredCanvas,
        };
        let panel = menu_camera_for_projection_y(106.0, request);
        let prop = menu_camera_for_projection_y(170.0, request);
        let panel_screen_y = 120.0
            * (1.0 - (panel.projection_matrix()[5] * -3.2 / 29.0 - panel.projection_matrix()[9]));
        let prop_screen_y = 120.0
            * (1.0 - (prop.projection_matrix()[5] * -3.2 / 25.0 - prop.projection_matrix()[9]));

        assert!((panel_screen_y - (106.0 + focal_y * 3.2 / 29.0)).abs() < 0.01);
        assert!((prop_screen_y - (170.0 + focal_y * 3.2 / 25.0)).abs() < 0.01);
        assert!(prop_screen_y > panel_screen_y + 60.0);
    }

    #[test]
    fn high_native_frontend_cameras_keep_text_anchors_through_fullscreen_and_back() {
        for (authored_canvas, hd_scale, uhd_scale) in [
            ([640, 480], 1.8, 3.6),
            ([800, 600], 1.8, 3.6),
            ([1024, 768], 1.40625, 2.8125),
        ] {
            let authored_height = authored_canvas[1] as f32;
            let authored_focal = authored_height * 0.5 / (50.4_f32.to_radians() * 0.5).tan();
            for (viewport, expected_scale) in [
                (authored_canvas, 1.0),
                ([1920, 1080], hd_scale),
                ([3840, 2160], uhd_scale),
                (authored_canvas, 1.0),
            ] {
                let request = UiMappingRequest {
                    viewport,
                    authored_canvas,
                    policy: UiSubmissionPolicy::NativeCanvas,
                };
                let text_mapping = UiMapping::new(request);
                // Text and the foreground model grow together, with the whole
                // authored composition bounded by the drawable dimensions.
                assert_eq!(text_mapping.scale, expected_scale);
                let margins = [
                    ((viewport[0] as f32 - authored_canvas[0] as f32 * expected_scale) * 0.5) as i32
                        as f32,
                    ((viewport[1] as f32 - authored_canvas[1] as f32 * expected_scale) * 0.5) as i32
                        as f32,
                ];
                // Ring, optionsh panel and screen decoration each own a Y
                // projection centre, while all intentionally share centre X.
                for authored_y in [155.0, 106.0, 170.0].map(|y| y * authored_height / 240.0) {
                    let camera = menu_camera_for_projection_y(authored_y, request);
                    let projection = camera.projection_matrix();
                    let origin = [
                        (1.0 - camera.projection_offset[0]) * viewport[0] as f32 * 0.5,
                        (1.0 + camera.projection_offset[1]) * viewport[1] as f32 * 0.5,
                    ];
                    let expected_origin = [
                        margins[0] + authored_canvas[0] as f32 * 0.5 * expected_scale,
                        margins[1] + authored_y * expected_scale,
                    ];
                    for (actual, expected) in origin.into_iter().zip(expected_origin) {
                        assert!((actual - expected).abs() < 0.001);
                    }
                    let text_origin = [
                        text_mapping.offset_x as f32
                            + authored_canvas[0] as f32 * 0.5 * text_mapping.scale,
                        text_mapping.offset_y as f32 + authored_y * text_mapping.scale,
                    ];
                    for (actual, expected) in origin.into_iter().zip(text_origin) {
                        assert!((actual - expected).abs() < 0.001);
                    }
                    let focal = [
                        projection[0] * viewport[0] as f32 * 0.5,
                        projection[5] * viewport[1] as f32 * 0.5,
                    ];
                    for actual in focal {
                        assert!((actual - authored_focal * expected_scale).abs() < 0.001);
                    }
                    // A panel point below its origin grows by the same factor
                    // as the distance from its corresponding glyph baseline.
                    let panel_y = origin[1] + focal[1] * 3.2 / 29.0;
                    let expected_panel_y =
                        margins[1] + (authored_y + authored_focal * 3.2 / 29.0) * expected_scale;
                    assert!((panel_y - expected_panel_y).abs() < 0.001);
                }
            }
        }
    }

    #[test]
    fn fitted_frontend_cameras_keep_the_existing_low_and_scaled_lens() {
        for authored_canvas in [[320, 240], [640, 480], [800, 600], [1024, 768]] {
            for viewport in [authored_canvas, [1920, 1080], [480, 800]] {
                let request = UiMappingRequest {
                    viewport,
                    authored_canvas,
                    policy: UiSubmissionPolicy::FitAuthoredCanvas,
                };
                let authored_y = authored_canvas[1] as f32 * 106.0 / 240.0;
                let camera = menu_camera_for_projection_y(authored_y, request);
                let mut expected = menu_camera(viewport[0] as f32 / viewport[1] as f32);
                expected.projection_offset[1] = 2.0 * authored_y / authored_canvas[1] as f32 - 1.0;
                assert_eq!(camera.projection_matrix(), expected.projection_matrix());
            }
        }
    }

    #[test]
    fn selected_ring_prop_uses_authored_high_detail_projection_center() {
        let virtual_h = 480.0;
        let center_y = virtual_h * 0.5;
        let focal_y = center_y / (50.4_f32.to_radians() * 0.5).tan();
        let depth_raw = 2300.0;
        let camera = menu_camera_for_projection_y(
            310.0,
            UiMappingRequest {
                viewport: [640, 480],
                authored_canvas: [640, 480],
                policy: UiSubmissionPolicy::FitAuthoredCanvas,
            },
        );
        let projection = camera.projection_matrix();
        let screen_y =
            center_y * (1.0 - (projection[5] * -3.2 / (depth_raw / 100.0) - projection[9]));

        assert!((screen_y - (310.0 + focal_y * 3.2 / 23.0)).abs() < 0.01);
        assert!(screen_y > center_y + 100.0);
    }

    #[test]
    fn world_model_shade_samples_cell_light_and_underwater_depth() {
        let terrain = v2k_formats::terrain::TerrainGrid {
            header: [0, 0, 0, 0, (512_i32 << 16) | i32::from((-1024_i16) as u16)],
            cells: Vec::new(),
        };
        let mut lights = v2k_render::TerrainLightWindow::default();
        lights.add_point(10, 20, 3);

        // Raw Y -1280 is halfway through the -1024..-1536 darkness ramp:
        // local light +3 minus darkness 4 gives a signed table shift of -1.
        assert_eq!(
            world_model_shade_shift([10.25, -5.0, 20.75], Some(&terrain), Some(&lights)),
            -1
        );
    }

    #[test]
    fn world_render_light_stage_rebuilds_current_particle_contributions() {
        let terrain = v2k_formats::terrain::TerrainGrid {
            header: [0; 5],
            cells: vec![
                v2k_formats::terrain::TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0,
                };
                v2k_formats::terrain::GRID_SIZE * v2k_formats::terrain::GRID_SIZE
            ],
        };
        let mut world_fx = WorldFx::new();
        world_fx.queue_meteor_impact([16.0, 0.0, 16.0], 0, None);
        world_fx.process_pending();

        let mut camera = Camera::new(4.0 / 3.0);
        // The retail +10-cell Z lead places the light window around Z=16.
        camera.position = [16.0, 4.0, 6.0];
        camera.yaw = std::f32::consts::PI;
        camera.pitch = 0.0;
        let presentation = world_fx.prepare_presentation([320, 240], 0x1800, |particle| {
            project_particle_center(
                &camera,
                [320, 240],
                camera_relative(&camera, particle.position),
            )
        });
        let mut lights = v2k_render::TerrainLightWindow::default();
        rebuild_particle_terrain_lights(&mut lights, &camera, &terrain, &presentation);
        let frame_value = lights.sample(16, 16);
        assert!(frame_value > 0);

        // 53570 precedes actors; pre-actor light writers survive 53A60.
        // A buffered actor keeps its captured shade while terrain/static
        // consumers see the additional current-frame particle contribution.
        begin_particle_terrain_lights(&mut lights, &camera);
        lights.add_point(16, 16, 1);
        let actor_shade = world_model_shade_shift([16.0, 0.0, 16.0], Some(&terrain), Some(&lights));
        assert_eq!(actor_shade, 1);
        accumulate_particle_terrain_lights(&mut lights, &terrain, &presentation);
        assert_eq!(lights.sample(16, 16), frame_value + 1);
        assert_eq!(actor_shade, 1);
        assert_ne!(
            world_model_shade_shift([16.0, 0.0, 16.0], Some(&terrain), Some(&lights)),
            actor_shade
        );

        // A second light rebuild of unchanged presentation starts from a
        // cleared table; it must reproduce the frame value instead of
        // accumulating history.
        lights.add_point(16, 16, 100);
        rebuild_particle_terrain_lights(&mut lights, &camera, &terrain, &presentation);
        assert_eq!(lights.sample(16, 16), frame_value);
    }

    #[test]
    fn klaus_pose_channels_match_retail_fixed_point_goldens() {
        let cases = [
            (
                0,
                0,
                0,
                0,
                [
                    0x0000, 0x0000, 0x0001, 0xF001, 0xEC3C, 0xE5A9, 0x051D, 0x0800, 0x0000, 0x0000,
                    0xF800, 0xFD4D,
                ],
            ),
            (
                37,
                0,
                0,
                0,
                [
                    0x0000, 0x0000, 0x0001, 0xF7D6, 0xF814, 0xDCF3, 0x033B, 0x0800, 0x05EF, 0x0000,
                    0xFBFF, 0xE49D,
                ],
            ),
            (
                37,
                0x3800,
                0,
                0,
                [
                    0x0000, 0x0000, 0x0001, 0x1F44, 0x0DC9, 0xDB19, 0x033B, 0x0800, 0xF238, 0x0FB1,
                    0xE848, 0xD3D5,
                ],
            ),
            (
                37,
                0x6000,
                0,
                0,
                [
                    0x0000, 0x0000, 0x0001, 0x0EC0, 0x154B, 0xF8D7, 0x033B, 0x0800, 0xFA7A, 0x0B4F,
                    0xF08A, 0xF488,
                ],
            ),
            (
                37,
                0,
                0x3800,
                32,
                [
                    0x0000, 0x0000, 0x0001, 0xF7D6, 0xF814, 0xDCF3, 0x0316, 0x07DB, 0x05EF, 0x0000,
                    0xFBFF, 0xE49D,
                ],
            ),
        ];
        for (retail_tick, primary_phase, twitch_phase, twitch_strength, expected) in cases {
            let vars = menu_backdrop_anim_vars(
                retail_tick,
                KlausBackdropAnimState {
                    morph_progress: 0,
                    sway_amplitude_raw: 0xFFFF / 0x32,
                    primary_phase,
                    twitch_phase,
                    twitch_strength,
                    model_state: 1,
                    visible: true,
                },
            );
            assert_eq!(&vars.dynamic[..12], expected.as_slice());
        }

        for phase in (0..=u16::MAX).step_by(257) {
            let vars = menu_backdrop_anim_vars(
                50,
                KlausBackdropAnimState {
                    morph_progress: 0,
                    sway_amplitude_raw: 0xFFFF / 0x32,
                    primary_phase: phase,
                    twitch_phase: phase,
                    twitch_strength: 32,
                    model_state: 1,
                    visible: true,
                },
            );
            assert!(vars.dynamic.iter().all(|&v| (0..=0xFFFF).contains(&v)));
        }

        // The iris morph and the spin jaw hinge are independent retail
        // inputs. Opening must neither replace nor add a guessed angle to 9.
        for morph_progress in [0, 0x4000, 0x8000, 0xFFFF] {
            let opened = menu_backdrop_anim_vars(
                37,
                KlausBackdropAnimState {
                    morph_progress,
                    sway_amplitude_raw: (0xFFFF - morph_progress) / 0x32,
                    primary_phase: 0x3800,
                    twitch_phase: 0,
                    twitch_strength: 0,
                    model_state: 0,
                    visible: true,
                },
            );
            assert_eq!(opened.dynamic[1], i32::from(morph_progress));
            assert_eq!(opened.dynamic[2], 0);
            assert_eq!(opened.dynamic[9], 0x0FB1);
        }
    }

    #[test]
    fn submenu_clip_starts_at_first_glyph_not_overscan_row() {
        let mapping = UiMapping {
            scale: 2.0,
            offset_x: 0,
            offset_y: 0,
        };
        let (top, bottom) = menu_list_clip_y(mapping, 105.0, 14.0, 5.0, 9.0, 1.0);
        assert_eq!(top, 194); // (baseline 106 - ascent 9) * 2
        assert_eq!(bottom, 326); // (last baseline 162 + descent 1) * 2
        assert!(top > ((105.0 - 14.0) * 2.0) as i32);
        assert!(bottom < ((105.0 + 5.0 * 14.0) * 2.0) as i32);
    }

    #[test]
    fn disabled_menu_tone_dims_rgb_without_changing_alpha() {
        let original = [255, 200, 100, 128, 0, 40, 0, 17];
        let mut normal = original;
        apply_menu_text_tone(&mut normal, MenuTextTone::Normal);
        assert_eq!(normal, original);

        let mut disabled = original;
        apply_menu_text_tone(&mut disabled, MenuTextTone::Disabled);
        assert_eq!(disabled, [96, 75, 37, 128, 0, 15, 0, 17]);
    }

    #[test]
    fn submenu_klaus_tracks_billboard_vertical_shift() {
        let focal_y = 120.0 / (50.4_f32.to_radians() * 0.5).tan();
        let depth_raw = 0x1200 as f32;
        let billboard_offset = -50.0;
        let world_y =
            menu_world_y_for_projection_center(0.0, depth_raw, 120.0 + billboard_offset, 240.0);
        let screen_y = 120.0 - focal_y * world_y / (depth_raw / 100.0);

        assert!((screen_y - (120.0 + billboard_offset)).abs() < 0.01);
        assert!(world_y > 0.0, "negative screen offset must lift Klaus");
    }
}
