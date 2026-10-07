use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::renderer::RenderBackend;

mod persistence;

/// Read retail's optional REG_SZ Save Path without changing the installation.
/// Callers explicitly choose whether to import this directory; tests and
/// portable tools need not consult the host registry.
pub fn retail_save_directory() -> Option<std::path::PathBuf> {
    persistence::retail_save_directory()
}

/// Renderer preference in config file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RendererChoice {
    Auto,
    OpenGL,
    Software,
    Wgpu,
}

/// How the port maps the authored 4:3 image onto the output display.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScalingMode {
    /// Modern presentation: native-aspect 3-D with undistorted 2-D elements.
    #[default]
    Native,
    /// Preserve the complete 4:3 frame and add bars where required.
    FourThree,
    /// Reproduce legacy monitor scaling by stretching a 4:3 frame to fill.
    Stretched,
}

impl ScalingMode {
    pub const fn index(self) -> u32 {
        match self {
            Self::Native => 0,
            Self::FourThree => 1,
            Self::Stretched => 2,
        }
    }

    pub const fn from_index(index: u32) -> Self {
        match index {
            1 => Self::FourThree,
            2 => Self::Stretched,
            _ => Self::Native,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Native => "Native",
            Self::FourThree => "4:3",
            Self::Stretched => "Stretched",
        }
    }
}

/// Difficulty level.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Easy,
    #[default]
    Medium,
    Hard,
}

impl Difficulty {
    pub fn cycle(self, dir: i32) -> Self {
        let values = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard];
        let idx = values.iter().position(|&d| d == self).unwrap_or(1);
        let new_idx = (idx as i32 + dir).rem_euclid(values.len() as i32) as usize;
        values[new_idx]
    }

    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Medium => "Medium",
            Difficulty::Hard => "Hard",
        }
    }
}

/// Menu/sprite detail tier. Low uses the 320×240 variant-0 art embedded in
/// PRELOAD.DAT; High uses the shared high-resolution art with a 640×480,
/// 800×600 or 1024×768 layout selected by `GameConfig::system_graphics_variant`.
/// Defaults to High.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GraphicsDetail {
    Low,
    #[default]
    High,
}

impl GraphicsDetail {
    pub fn toggle(self) -> Self {
        match self {
            GraphicsDetail::Low => GraphicsDetail::High,
            GraphicsDetail::High => GraphicsDetail::Low,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            GraphicsDetail::Low => "Low",
            GraphicsDetail::High => "High",
        }
    }

    /// Initial fallback size before the selected level-2 layout is loaded.
    pub const fn menu_virtual_size(self) -> (u32, u32) {
        match self {
            GraphicsDetail::Low => (320, 240),
            GraphicsDetail::High => (640, 480),
        }
    }
}

/// Game configuration, loaded from/saved to config.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    /// Lossless native DWORD baseline; not part of the legacy JSON schema.
    #[serde(skip)]
    #[doc(hidden)]
    pub settings_snapshot: Option<persistence::SettingsSnapshot>,
    // Display
    pub renderer: RendererChoice,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    /// Modern/native, preserved 4:3, or legacy stretched presentation.
    #[serde(default)]
    pub scaling: ScalingMode,
    /// Port-only presentation option: render the selected tier's authored
    /// frame offscreen before scaling it to the output. Native scaling
    /// deliberately bypasses this path.
    #[serde(default)]
    pub classic_framebuffer: bool,
    /// Persisted value of retail's inert Bilinear Filtering menu option.
    /// The software renderer never consumed it; world textures stay point sampled.
    #[serde(default = "default_true")]
    pub bilinear_filtering: bool,

    // Sound
    #[serde(default = "default_true")]
    pub sound_enabled: bool,
    /// Compatibility gate for the original Ambient setting. The authoritative
    /// raw value is `music_volume * 15`; valid writes keep both fields aligned.
    #[serde(default = "default_true")]
    pub ambient_enabled: bool,
    /// Normalized persistence of the original integer Ambient value (0..15).
    /// Its positive magnitude is not a PCM gain: CD music always plays at
    /// unity and only zero/nonzero controls pause/resume.
    #[serde(default = "default_volume")]
    pub music_volume: f32,
    #[serde(default = "default_volume")]
    pub sfx_volume: f32,

    // Controls
    #[serde(default = "default_sensitivity")]
    pub sensitivity: f32,
    /// Original 0-15 self-righting strength. Older port configs stored this
    /// as a boolean; the compatibility deserializer maps false/true to 0/1.
    #[serde(
        default = "default_self_righting",
        deserialize_with = "deserialize_self_righting"
    )]
    pub self_righting: u8,
    #[serde(default = "default_joystick_mode")]
    pub joystick_mode: u8,
    #[serde(default)]
    pub absolute_mode: bool,
    #[serde(default = "default_active_camera")]
    pub active_camera: u8,
    #[serde(default = "default_true")]
    pub targetter: bool,
    #[serde(default = "default_true")]
    pub hud: bool,
    #[serde(default)]
    pub language: u8,

    // Game
    #[serde(default)]
    pub difficulty: Difficulty,

    // Graphics
    /// Menu/sprite detail tier (Low = variant-0 assets, High = variant-1).
    /// In the original this was the Display→"Resolution" spinner (settings
    /// 0x4CB3E8) used directly as the OVL variant digit. The menu updates this
    /// alongside the window mode; resource-tier changes apply next launch.
    #[serde(default)]
    pub detail: GraphicsDetail,
}

fn default_true() -> bool {
    true
}
fn default_volume() -> f32 {
    1.0
}
fn default_sensitivity() -> f32 {
    10.0 / 15.0
}
fn default_self_righting() -> u8 {
    1
}
fn default_joystick_mode() -> u8 {
    1
}
fn default_active_camera() -> u8 {
    6
}

fn deserialize_self_righting<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Compat {
        Bool(bool),
        Number(u8),
    }

    Ok(match Compat::deserialize(deserializer)? {
        Compat::Bool(on) => u8::from(on),
        Compat::Number(value) => value.min(15),
    })
}

/// Authored game resolution presets; fullscreen drawable size is independent.
pub const RESOLUTIONS: &[(u32, u32)] = &[(640, 480), (800, 600), (1024, 768)];

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            settings_snapshot: None,
            renderer: RendererChoice::Auto,
            width: 800,
            height: 600,
            fullscreen: false,
            scaling: ScalingMode::Native,
            classic_framebuffer: false,
            bilinear_filtering: true,
            sound_enabled: true,
            ambient_enabled: true,
            music_volume: 1.0,
            sfx_volume: 1.0,
            sensitivity: 10.0 / 15.0,
            self_righting: 1,
            joystick_mode: 1,
            absolute_mode: false,
            active_camera: 6,
            targetter: true,
            hud: true,
            language: 0,
            difficulty: Difficulty::Medium,
            detail: GraphicsDetail::High,
        }
    }
}

impl GameConfig {
    /// Whether the classic authored-tier presentation path is effective for
    /// the current output mode. Native presentation always renders directly
    /// at the drawable resolution.
    pub const fn classic_framebuffer_effective(&self) -> bool {
        self.classic_framebuffer && !matches!(self.scaling, ScalingMode::Native)
    }

    /// Select the retail display tier for the configured output.
    /// High-detail modes retain the largest authored 4:3 size that fits the
    /// configured output, through retail's 1024x768 ceiling. Low detail remains
    /// the explicit 320x240 tier. The same authored layouts serve Native and
    /// Classic presentation; high-tier layout changes can be applied live.
    pub const fn system_graphics_variant(&self) -> u32 {
        if matches!(self.detail, GraphicsDetail::Low) {
            0
        } else if self.width >= 1024 && self.height >= 768 {
            3
        } else if self.width >= 800 && self.height >= 600 {
            2
        } else {
            1
        }
    }

    /// Load native settings and the separate port presentation preferences.
    pub fn load(dir: &Path) -> Self {
        persistence::load(dir)
    }

    fn normalize_presentation(&mut self) {
        // Older port preferences allowed custom/HD output sizes. Retain the
        // largest authored tier that fits both dimensions, with a 640x480
        // minimum window even when Low detail selects 320x240 game art.
        let (width, height) = RESOLUTIONS
            .iter()
            .rev()
            .copied()
            .find(|&(width, height)| width <= self.width && height <= self.height)
            .unwrap_or(RESOLUTIONS[0]);
        self.width = width;
        self.height = height;
        if self.scaling == ScalingMode::Native {
            self.classic_framebuffer = false;
        }
    }

    /// Persist to the port-owned native namespace and presentation file.
    /// Imported retail settings and the legacy config.json are never written.
    pub fn try_save(&mut self, dir: &Path) -> std::io::Result<()> {
        persistence::save(self, dir)
    }

    /// Resolve the actual backend to use, given the config preference
    /// and an optional CLI override.
    pub fn resolve_backend(&self, cli_override: Option<&str>) -> RenderBackend {
        match cli_override {
            Some("opengl") => RenderBackend::OpenGL,
            Some("software") => RenderBackend::Software,
            _ => match &self.renderer {
                RendererChoice::OpenGL => RenderBackend::OpenGL,
                RendererChoice::Software => RenderBackend::Software,
                _ => RenderBackend::OpenGL,
            },
        }
    }

    /// Update config with the successfully-initialized backend.
    pub fn set_detected_backend(&mut self, backend: RenderBackend) {
        self.renderer = match backend {
            RenderBackend::OpenGL => RendererChoice::OpenGL,
            RenderBackend::Software => RendererChoice::Software,
        };
    }

    /// Set resolution by preset index (clamped).
    pub fn set_resolution_index(&mut self, idx: usize) {
        let idx = idx.min(RESOLUTIONS.len() - 1);
        self.width = RESOLUTIONS[idx].0;
        self.height = RESOLUTIONS[idx].1;
    }

    /// Index of the current resolution in the preset list, if it matches one.
    pub fn resolution_index(&self) -> Option<usize> {
        RESOLUTIONS
            .iter()
            .position(|&(w, h)| w == self.width && h == self.height)
    }

    /// Cycle resolution to the next preset.
    pub fn cycle_resolution(&mut self, dir: i32) {
        let current = RESOLUTIONS
            .iter()
            .position(|&(w, h)| w == self.width && h == self.height);
        let idx = current.unwrap_or(1); // default to 800x600 position
        let new_idx = (idx as i32 + dir).rem_euclid(RESOLUTIONS.len() as i32) as usize;
        self.width = RESOLUTIONS[new_idx].0;
        self.height = RESOLUTIONS[new_idx].1;
    }

    /// Get a human-readable label for the current resolution.
    pub fn resolution_label(&self) -> String {
        format!("{}x{}", self.width, self.height)
    }

    /// Get a label for the fullscreen setting.
    pub fn fullscreen_label(&self) -> &str {
        if self.fullscreen {
            "Full Screen"
        } else {
            "In a Window"
        }
    }

    /// Get a label for the renderer.
    pub fn renderer_label(&self) -> &str {
        match &self.renderer {
            RendererChoice::OpenGL => "OpenGL",
            RendererChoice::Software => "Software",
            RendererChoice::Auto => "Auto",
            RendererChoice::Wgpu => "Wgpu",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let cfg = GameConfig::default();
        assert_eq!(cfg.renderer, RendererChoice::Auto);
        assert_eq!(cfg.width, 800);
        assert_eq!(cfg.height, 600);
        assert!(!cfg.fullscreen);
        assert_eq!(cfg.scaling, ScalingMode::Native);
        assert!(!cfg.classic_framebuffer);
        assert!(!cfg.classic_framebuffer_effective());
        assert!(cfg.bilinear_filtering);
        assert!(cfg.sound_enabled);
        assert!(cfg.ambient_enabled);
        assert!((cfg.music_volume - 1.0).abs() < 0.01);
        assert_eq!(cfg.self_righting, 1);
        assert_eq!(cfg.joystick_mode, 1);
        assert_eq!(cfg.active_camera, 6);
        assert!(cfg.targetter);
        assert_eq!(cfg.difficulty, Difficulty::Medium);
    }

    #[test]
    fn serialize_roundtrip() {
        let cfg = GameConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let cfg2: GameConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg.renderer, cfg2.renderer);
        assert_eq!(cfg.width, cfg2.width);
        assert_eq!(cfg.scaling, cfg2.scaling);
        assert_eq!(cfg.classic_framebuffer, cfg2.classic_framebuffer);
        assert_eq!(cfg.difficulty, cfg2.difficulty);
    }

    #[test]
    fn nondefault_ambient_setting_survives_json_roundtrip() {
        let cfg = GameConfig {
            ambient_enabled: true,
            // Normalized persistence for the original integer Ambient=7.
            // Playback still treats every positive value as unity/on.
            music_volume: 7.0 / 15.0,
            ..GameConfig::default()
        };

        let json = serde_json::to_string(&cfg).unwrap();
        let restored: GameConfig = serde_json::from_str(&json).unwrap();

        assert!(restored.ambient_enabled);
        assert!((restored.music_volume * 15.0 - 7.0).abs() < f32::EPSILON);
    }

    #[test]
    fn resolve_cli_override() {
        let cfg = GameConfig::default();
        assert_eq!(
            cfg.resolve_backend(Some("software")),
            RenderBackend::Software
        );
        assert_eq!(cfg.resolve_backend(Some("opengl")), RenderBackend::OpenGL);
        assert_eq!(cfg.resolve_backend(None), RenderBackend::OpenGL);
    }

    #[test]
    fn normalized_preferences_keep_a_software_choice() {
        let mut cfg = GameConfig {
            renderer: RendererChoice::Software,
            ..GameConfig::default()
        };
        cfg.normalize_presentation();
        assert_eq!(cfg.renderer, RendererChoice::Software);
        assert_eq!(cfg.resolve_backend(None), RenderBackend::Software);
        assert_eq!(cfg.resolve_backend(Some("opengl")), RenderBackend::OpenGL);
    }

    #[test]
    fn backward_compat_old_config() {
        // Old config with only display fields — new fields should get defaults
        let json = r#"{"renderer":"opengl","width":1024,"height":768,"fullscreen":true}"#;
        let cfg: GameConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.width, 1024);
        assert!(cfg.fullscreen);
        assert_eq!(cfg.scaling, ScalingMode::Native);
        assert!(!cfg.classic_framebuffer);
        assert!(cfg.sound_enabled);
        assert!((cfg.music_volume - 1.0).abs() < 0.01);
        assert_eq!(cfg.difficulty, Difficulty::Medium);
    }

    #[test]
    fn classic_framebuffer_is_effective_only_for_scaled_output() {
        let mut cfg = GameConfig {
            classic_framebuffer: true,
            ..GameConfig::default()
        };
        assert!(!cfg.classic_framebuffer_effective());

        cfg.scaling = ScalingMode::FourThree;
        assert!(cfg.classic_framebuffer_effective());

        cfg.scaling = ScalingMode::Stretched;
        assert!(cfg.classic_framebuffer_effective());
    }

    #[test]
    fn output_resolution_selects_authored_tiers_through_1024_in_every_presentation_mode() {
        let mut config = GameConfig {
            scaling: ScalingMode::FourThree,
            classic_framebuffer: true,
            ..GameConfig::default()
        };
        for (width, height, variant) in [
            (640, 480, 1),
            (800, 600, 2),
            (1024, 768, 3),
            (1920, 1080, 3),
            (1280, 720, 2),
        ] {
            config.width = width;
            config.height = height;
            for scaling in [
                ScalingMode::Native,
                ScalingMode::FourThree,
                ScalingMode::Stretched,
            ] {
                for classic_framebuffer in [false, true] {
                    config.scaling = scaling;
                    config.classic_framebuffer = classic_framebuffer;
                    assert_eq!(config.system_graphics_variant(), variant);
                }
            }
        }
        config.detail = GraphicsDetail::Low;
        assert_eq!(config.system_graphics_variant(), 0);
        config.detail = GraphicsDetail::High;
        config.scaling = ScalingMode::Native;
        assert_eq!(config.system_graphics_variant(), 2);
        config.scaling = ScalingMode::FourThree;
        config.classic_framebuffer = false;
        assert_eq!(config.system_graphics_variant(), 2);
    }

    #[test]
    fn loaded_native_config_cannot_retain_a_hidden_classic_request() {
        let mut cfg = GameConfig {
            scaling: ScalingMode::Native,
            classic_framebuffer: true,
            ..GameConfig::default()
        };
        cfg.normalize_presentation();
        assert!(!cfg.classic_framebuffer);
    }

    #[test]
    fn old_boolean_self_righting_is_accepted() {
        let off: GameConfig = serde_json::from_str(
            r#"{"renderer":"opengl","width":800,"height":600,"fullscreen":false,"self_righting":false}"#,
        ).unwrap();
        let on: GameConfig = serde_json::from_str(
            r#"{"renderer":"opengl","width":800,"height":600,"fullscreen":false,"self_righting":true}"#,
        ).unwrap();
        assert_eq!(off.self_righting, 0);
        assert_eq!(on.self_righting, 1);
    }

    #[test]
    fn cycle_resolution() {
        let mut cfg = GameConfig::default(); // 800x600
        cfg.cycle_resolution(1); // → 1024x768
        assert_eq!((cfg.width, cfg.height), (1024, 768));
        cfg.cycle_resolution(-1); // → 800x600
        assert_eq!((cfg.width, cfg.height), (800, 600));
        cfg.cycle_resolution(-1); // → 640x480
        assert_eq!((cfg.width, cfg.height), (640, 480));
        cfg.cycle_resolution(-1); // → wraps to 1024x768
        assert_eq!((cfg.width, cfg.height), (1024, 768));
        cfg.cycle_resolution(1); // → wraps to 640x480
        assert_eq!((cfg.width, cfg.height), (640, 480));
    }

    #[test]
    fn resolution_presets_and_clamped_selection_stay_within_authored_modes() {
        assert_eq!(RESOLUTIONS, &[(640, 480), (800, 600), (1024, 768)]);
        let mut cfg = GameConfig::default();
        for (index, &(width, height)) in RESOLUTIONS.iter().enumerate() {
            cfg.set_resolution_index(index);
            assert_eq!((cfg.width, cfg.height), (width, height));
            assert_eq!(cfg.resolution_index(), Some(index));
        }
        cfg.set_resolution_index(usize::MAX);
        assert_eq!((cfg.width, cfg.height), (1024, 768));
    }

    #[test]
    fn unsupported_dimensions_normalize_to_the_largest_fitting_authored_mode() {
        for (input, expected) in [
            ((0, 0), (640, 480)),
            ((320, 240), (640, 480)),
            ((799, 599), (640, 480)),
            ((640, 480), (640, 480)),
            ((800, 600), (800, 600)),
            ((1000, 1000), (800, 600)),
            ((768, 1024), (640, 480)),
            ((1024, 768), (1024, 768)),
            ((1280, 720), (800, 600)),
            ((1920, 1080), (1024, 768)),
            ((3840, 2160), (1024, 768)),
        ] {
            for detail in [GraphicsDetail::Low, GraphicsDetail::High] {
                let mut cfg = GameConfig {
                    width: input.0,
                    height: input.1,
                    detail,
                    ..GameConfig::default()
                };
                let original_variant = cfg.system_graphics_variant();
                cfg.normalize_presentation();
                assert_eq!((cfg.width, cfg.height), expected, "input={input:?}");
                assert_eq!(cfg.detail, detail);
                assert_eq!(cfg.system_graphics_variant(), original_variant);
            }
        }
    }

    #[test]
    fn difficulty_cycle() {
        assert_eq!(Difficulty::Easy.cycle(1), Difficulty::Medium);
        assert_eq!(Difficulty::Medium.cycle(1), Difficulty::Hard);
        assert_eq!(Difficulty::Hard.cycle(1), Difficulty::Easy);
        assert_eq!(Difficulty::Easy.cycle(-1), Difficulty::Hard);
    }
}
