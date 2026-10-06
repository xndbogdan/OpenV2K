//! Atomic publication of the resident display tier and dependent snapshots.
//!
//! This port extension preserves the current world, its allocated terrain
//! radar, evaluated HUD choices and frozen effects. The next world uses the
//! newly selected authored system data at its normal construction boundary.

use super::*;
use v2k_game::gameplay_hud::GameplayHudRelayout;
use v2k_game::gameplay_radar::GameplayRadarLayout;
use v2k_game::menu_text::{MenuFonts, PreparedMenuFontLayout};
use v2k_game::session::GameSession;
use v2k_game::system_layout::{HighSystemLayoutTier, SystemLayoutRefreshError};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub(super) enum DisplayRefreshError {
    Resource(SystemLayoutRefreshError),
    Unavailable(&'static str),
    Radar(v2k_game::gameplay_radar::GameplayRadarRelayoutError),
    Overlay(paused_gameplay::GameplayOverlayRelayoutError),
}

impl std::fmt::Display for DisplayRefreshError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resource(error) => write!(formatter, "{error}"),
            Self::Unavailable(owner) => write!(formatter, "{owner} is unavailable"),
            Self::Radar(error) => write!(formatter, "radar layout: {error:?}"),
            Self::Overlay(error) => write!(formatter, "cached overlay layout: {error:?}"),
        }
    }
}

/// Mutually dependent presentation owners. No simulation owner is borrowed.
pub(super) struct ResidentDisplay<'a> {
    pub session: &'a mut GameSession,
    pub fonts: &'a mut Option<MenuFonts>,
    pub variant: &'a mut Option<u32>,
    pub world_variant: &'a mut u32,
    pub reference_size: &'a mut (u32, u32),
    pub hud_layout: &'a mut Option<GameplayHudLayout>,
    pub radar: &'a mut Option<GameplayRadar>,
    pub map_status: &'a mut Option<FullscreenMapStatusResources>,
    pub overlay: &'a mut GameplayOverlayFrame,
    pub projection: &'a mut WorldProjection,
}

impl ResidentDisplay<'_> {
    pub fn refresh(&mut self, tier: HighSystemLayoutTier) -> Result<bool, DisplayRefreshError> {
        let Some(previous_tier) = self.variant.and_then(HighSystemLayoutTier::from_variant) else {
            return Err(DisplayRefreshError::Unavailable("resident high artwork"));
        };
        if previous_tier == tier {
            return Ok(false);
        }
        if self.fonts.is_none() {
            return Err(DisplayRefreshError::Unavailable("resident sprite fonts"));
        }
        let stage = self
            .session
            .prepare_high_system_layout_refresh(tier)
            .map_err(DisplayRefreshError::Resource)?;
        let fonts = PreparedMenuFontLayout::from_source(&stage)
            .ok_or(DisplayRefreshError::Unavailable("staged menu layout"))?;
        let projection = WorldProjection::from_cache(&stage)
            .ok_or(DisplayRefreshError::Unavailable("staged world projection"))?;
        let previous_hud = self
            .hud_layout
            .ok_or(DisplayRefreshError::Unavailable("resident HUD layout"))?;
        let next_hud = GameplayHudLayout::from_cache(&stage, tier.variant())
            .ok_or(DisplayRefreshError::Unavailable("staged HUD layout"))?;
        let previous_radar =
            GameplayRadarLayout::from_cache(&self.session.cache, previous_tier.variant())
                .ok_or(DisplayRefreshError::Unavailable("resident radar layout"))?;
        let next_radar = GameplayRadarLayout::from_cache(&stage, tier.variant())
            .ok_or(DisplayRefreshError::Unavailable("staged radar layout"))?;
        let radar = self
            .radar
            .as_ref()
            .map(|radar| radar.prepare_layout_refresh(next_radar))
            .transpose()
            .map_err(DisplayRefreshError::Radar)?;
        let overlay = self
            .overlay
            .prepare_relayout(paused_gameplay::GameplayOverlayRelayout {
                hud: GameplayHudRelayout {
                    previous: previous_hud,
                    next: next_hud,
                },
                radar_previous: previous_radar,
                radar_next: next_radar,
            })
            .map_err(DisplayRefreshError::Overlay)?;
        let mut map_status = self.map_status.clone();
        if let Some(status) = &mut map_status {
            status
                .refresh_layout(&stage)
                .ok_or(DisplayRefreshError::Unavailable("staged map status layout"))?;
        }

        // Every fallible dependent snapshot is prepared before either table
        // changes. Commit verifies the resident receipts again; the remaining
        // publication consists only of infallible swaps in this thread.
        self.session
            .cache
            .commit_high_system_layout_refresh(stage)
            .map_err(DisplayRefreshError::Resource)?;
        *self.reference_size = fonts.virtual_size();
        self.fonts
            .as_mut()
            .expect("preflight retained fonts")
            .refresh_layout(fonts);
        *self.hud_layout = Some(next_hud);
        if let (Some(radar), Some(prepared)) = (self.radar.as_mut(), radar) {
            radar.refresh_layout(prepared);
        }
        *self.map_status = map_status;
        self.overlay.publish_relayout(overlay);
        *self.projection = projection;
        *self.variant = Some(tier.variant());
        *self.world_variant = tier.variant();
        Ok(true)
    }
}

/// Follow actual resident artwork, including the labelled low fallback.
pub(super) fn ui_policy(
    config: &GameConfig,
    resident_variant: Option<u32>,
) -> v2k_render::UiSubmissionPolicy {
    if config.scaling == v2k_render::ScalingMode::Native
        && resident_variant
            .and_then(HighSystemLayoutTier::from_variant)
            .is_some()
    {
        v2k_render::UiSubmissionPolicy::NativeCanvas
    } else {
        v2k_render::UiSubmissionPolicy::FitAuthoredCanvas
    }
}
