use super::*;
use v2k_game::frozen_world_presentation::{
    FrozenWorldPresentationSnapshot, FrozenWorldPresentationSource,
};
use v2k_game::gameplay_hud::{GameplayHudRelayout, GameplayHudRelayoutError};
use v2k_game::gameplay_radar::{GameplayRadarLayout, GameplayRadarRelayoutError};

#[cfg(test)]
mod relayout_tests;

/// Keep already-produced HUD/radar/text commands, including their RNG and
/// typewriter decisions. Reprojecting a paused scene must not produce them again.
#[derive(Default)]
pub(super) struct GameplayOverlayFrame {
    pub full_frame_sprite: Option<FullFrameSpriteFrame>,
    pub radar: Option<RadarHudFrame>,
    pub hud: Option<GameplayHudFrame>,
    pub notifications: Vec<GameplayNotificationLine>,
    pub explosion_lights: Vec<TerrainExplosionLight>,
    pub static_explosion_lights: Vec<TerrainExplosionLight>,
    pub presentation: Option<GameplayWorldPresentation>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct GameplayOverlayRelayout {
    pub hud: GameplayHudRelayout,
    pub radar_previous: GameplayRadarLayout,
    pub radar_next: GameplayRadarLayout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GameplayOverlayRelayoutError {
    Hud(GameplayHudRelayoutError),
    Radar(GameplayRadarRelayoutError),
}

/// Only the commands whose authored anchors change are cloned for preflight.
/// Frozen world owners, evaluated effects and notification prefixes stay resident.
pub(super) struct PreparedGameplayOverlayRelayout {
    hud: Option<GameplayHudFrame>,
    radar: Option<RadarHudFrame>,
}

impl GameplayOverlayFrame {
    pub fn prepare_relayout(
        &self,
        request: GameplayOverlayRelayout,
    ) -> Result<PreparedGameplayOverlayRelayout, GameplayOverlayRelayoutError> {
        let mut hud = self.hud.clone();
        if let Some(frame) = &mut hud {
            frame
                .relayout(request.hud)
                .map_err(GameplayOverlayRelayoutError::Hud)?;
        }
        let mut radar = self.radar.clone();
        if let Some(frame) = &mut radar {
            frame
                .relayout(request.radar_previous, request.radar_next)
                .map_err(GameplayOverlayRelayoutError::Radar)?;
        }
        Ok(PreparedGameplayOverlayRelayout { hud, radar })
    }

    /// Publish after the atomic resource/layout replacement has succeeded.
    pub fn publish_relayout(&mut self, prepared: PreparedGameplayOverlayRelayout) {
        self.hud = prepared.hud;
        self.radar = prepared.radar;
    }
}

pub(super) struct PausedGameplayScene {
    world: FrozenWorldPresentationSnapshot,
    targetter: TargetterRuntime,
    retail_tick: u32,
}

impl PausedGameplayScene {
    pub fn capture(
        source: FrozenWorldPresentationSource<'_>,
        targetter: TargetterRuntime,
        retail_tick: u32,
    ) -> Self {
        Self {
            world: FrozenWorldPresentationSnapshot::capture(source),
            targetter,
            retail_tick,
        }
    }

    pub fn redraw(&self, renderer: &mut dyn Renderer, draw: PausedGameplayDraw<'_>) {
        self.world.redraw(|frozen| {
            let mut targetter = self.targetter;
            let mut lights = v2k_render::TerrainLightWindow::default();
            let mut pending_entity_weapon_fire = Vec::new();
            let mut entity_weapon_error_reported = false;
            let _ = draw_gameplay_world(
                renderer,
                GameplayWorldFrame {
                    cache: draw.cache,
                    face_colors: draw.colors,
                    em: frozen.entities,
                    camera: draw.camera,
                    camera_mode: draw.camera_mode,
                    native_viewport: draw.native_viewport,
                    world_projection: Some(draw.projection),
                    sky: draw.sky,
                    terrain_frames: draw.terrain_frames,
                    water_frames: draw.water_frames,
                    terrain_lights: &mut lights,
                    world_fx: frozen.world_fx,
                    explosion_lights: &draw.overlay.explosion_lights,
                    static_explosion_lights: &draw.overlay.static_explosion_lights,
                    abort_frame: draw.abort_frame,
                    specialized_actor_tasks: frozen.specialized_actor_tasks,
                    pending_entity_weapon_fire: &mut pending_entity_weapon_fire,
                    entity_weapon_error_reported: &mut entity_weapon_error_reported,
                    player_craft: draw.craft,
                    player_hull: draw.hull,
                    player_shield: GameplayWorldShield::Frozen(
                        draw.overlay
                            .presentation
                            .as_ref()
                            .and_then(|frame| frame.shield),
                    ),
                    particles: match draw.overlay.presentation.as_ref() {
                        Some(frame) => GameplayWorldParticles::Frozen(&frame.particles),
                        None => GameplayWorldParticles::Live,
                    },
                    player_death: draw.death,
                    player_model_id: draw.player_model_id,
                    retail_tick: self.retail_tick,
                    // Terrain infection animation and presentation clocks stay frozen.
                    elapsed_micros: 0,
                    targetter: GameplayWorldTargetter {
                        runtime: &mut targetter,
                        enabled: draw.targetter_enabled,
                    },
                },
            );
        });
        if let Some(command) = draw
            .overlay
            .full_frame_sprite
            .and_then(|frame| full_frame_sprite_command_for_frame(renderer, draw.cache, frame))
        {
            draw_prepared_full_frame_sprite_command(renderer, &command);
        }
        if let Some(radar) = &draw.overlay.radar {
            draw_gameplay_radar(renderer, radar);
        }
        if let (Some(resources), Some(hud)) = (draw.hud_resources, &draw.overlay.hud) {
            draw_gameplay_hud(
                renderer,
                draw.cache,
                draw.colors,
                resources,
                hud,
                draw.fonts.map(|fonts| &fonts.normal),
            );
        }
        if let Some(fonts) = draw.fonts {
            draw_gameplay_notifications(
                renderer,
                fonts,
                &draw.overlay.notifications,
                GameplayTextContext::WorldOverlay,
            );
        }
        if let Some(shell) = draw.handoff {
            draw_klaus_handoff_overlay(
                renderer,
                KlausHandoffFrame {
                    shell,
                    cache: draw.cache,
                    colors: draw.colors,
                    retail_tick: self.retail_tick,
                },
            );
        }
    }
}

/// Resident resources and immutable gameplay inputs shared with the live draw.
/// The pause snapshot exclusively owns the mutable callback inputs above.
pub(super) struct PausedGameplayDraw<'a> {
    pub cache: &'a v2k_game::resource_cache::ResourceCache,
    pub colors: &'a v2k_game::model_color::ModelMaterialCache,
    pub camera: &'a Camera,
    pub camera_mode: GameplayWorldCameraMode,
    pub native_viewport: Option<v2k_game::native_model_frame::NativeWorldViewport>,
    pub projection: WorldProjection,
    pub sky: Option<&'a v2k_game::sky::SkyBackground>,
    pub terrain_frames: Option<&'a v2k_render::TerrainFrames>,
    pub water_frames: Option<&'a v2k_render::WaterFrames>,
    pub abort_frame: Option<MainBaseAbortFrameRequest>,
    pub craft: &'a PlayerCraft,
    pub hull: &'a PlayerHull,
    pub death: &'a PlayerDeathLifecycle,
    pub player_model_id: Option<usize>,
    pub targetter_enabled: bool,
    pub overlay: &'a GameplayOverlayFrame,
    pub hud_resources: Option<&'a GameplayHudResources>,
    pub fonts: Option<&'a v2k_game::menu_text::MenuFonts>,
    pub handoff: Option<&'a MenuShell>,
}

pub(super) fn setting_requires_paused_redraw(id: SettingId) -> bool {
    matches!(
        id,
        SettingId::ClassicFramebuffer
            | SettingId::Scaling
            | SettingId::Resolution
            | SettingId::FullScreen
            | SettingId::Rendering
    )
}
