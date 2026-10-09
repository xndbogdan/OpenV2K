use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::renderer::RenderBackend;

mod display;
mod persistence;

pub use display::{
    holds_an_original, DisplayModes, DisplayRequest, WindowMode, ORIGINAL_RESOLUTIONS,
};

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
    /// The selected resolution: a window's client size or a full-screen
    /// display mode, in physical pixels. Borderless covers the desktop and
    /// keeps this for the next window or mode.
    pub width: u32,
    pub height: u32,
    /// Display row. Older port files stored a `fullscreen` boolean, which
    /// loads as the exclusive Full Screen it now names.
    #[serde(
        default,
        alias = "fullscreen",
        deserialize_with = "deserialize_display"
    )]
    pub display: WindowMode,
    /// The desktop size of the window's display, which Borderless covers.
    /// The display code sets it whenever it reads the monitor; never saved.
    #[serde(skip)]
    pub desktop: Option<(u32, u32)>,
    /// Modern/native, preserved 4:3, or legacy stretched presentation.
    #[serde(default)]
    pub scaling: ScalingMode,
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

fn deserialize_display<'de, D>(deserializer: D) -> Result<WindowMode, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Compat {
        FullScreen(bool),
        Mode(WindowMode),
    }

    Ok(match Compat::deserialize(deserializer)? {
        Compat::FullScreen(true) => WindowMode::FullScreen,
        Compat::FullScreen(false) => WindowMode::Window,
        Compat::Mode(mode) => mode,
    })
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            settings_snapshot: None,
            renderer: RendererChoice::Auto,
            width: 800,
            height: 600,
            display: WindowMode::Window,
            desktop: None,
            scaling: ScalingMode::Native,
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
    /// Select the retail display tier for the configured resolution.
    /// High detail takes the largest original size that fits it (and so its
    /// 4:3 area), through retail's 1024x768 ceiling; in Borderless that
    /// resolution is the desktop. Low detail remains the explicit 320x240
    /// tier. The same authored layouts serve every scaling mode; high-tier
    /// layout changes can be applied live.
    pub const fn system_graphics_variant(&self) -> u32 {
        let (width, height) = self.resolution();
        if matches!(self.detail, GraphicsDetail::Low) {
            0
        } else if width >= 1024 && height >= 768 {
            3
        } else if width >= 800 && height >= 600 {
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
        // Any size that holds an original tier is kept; whether the monitor
        // offers it is the startup search's question. Smaller sizes, including
        // Low detail's 320x240 tier word, take the 640x480 minimum window.
        if !holds_an_original((self.width, self.height)) {
            (self.width, self.height) = ORIGINAL_RESOLUTIONS[0];
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

    /// The exact selected size.
    pub const fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// The Resolution row's value: the size, or in Borderless the desktop
    /// it covers (the size until the desktop is known).
    pub const fn resolution(&self) -> (u32, u32) {
        match (self.display, self.desktop) {
            (WindowMode::Borderless, Some(desktop)) => desktop,
            _ => self.size(),
        }
    }

    /// Take `mode` with the row entry `size`. Borderless's only entry is the
    /// desktop, so it keeps the saved size, and leaving it restores that.
    pub fn adopt(&mut self, mode: WindowMode, size: (u32, u32)) {
        self.display = mode;
        if mode != WindowMode::Borderless {
            (self.width, self.height) = size;
        }
    }

    /// Change the Display row; the size becomes the entry it selects in
    /// the new row. An empty row (no reported modes) keeps the size.
    pub fn select_window_mode(&mut self, mode: WindowMode, modes: &DisplayModes) {
        match modes.resolve(mode, self.size()) {
            Some(size) => self.adopt(mode, size),
            None => self.display = mode,
        }
    }

    /// Choose the Resolution row's `index`th entry for the current display.
    pub fn select_resolution(&mut self, index: usize, modes: &DisplayModes) {
        if let Some(&size) = modes.resolutions(self.display).get(index) {
            self.adopt(self.display, size);
        }
    }

    /// What the game window should be for this configuration, as
    /// `FUN_0042D2A0` describes the display from the settings words. The
    /// renderer is created separately and Bilinear never reaches the display.
    pub const fn display_request(&self) -> DisplayRequest {
        DisplayRequest {
            mode: self.display,
            size: (self.width, self.height),
        }
    }

    /// Get a human-readable label for the current resolution.
    pub fn resolution_label(&self) -> String {
        let (width, height) = self.resolution();
        format!("{width}x{height}")
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
        assert_eq!(cfg.display, WindowMode::Window);
        assert_eq!(cfg.scaling, ScalingMode::Native);
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
        assert_eq!(cfg.display, cfg2.display);
        assert_eq!(cfg.scaling, cfg2.scaling);
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
        // Old config with only display fields — new fields should get defaults.
        // Its desktop-mode `fullscreen` loads as the exclusive Full Screen.
        let json = r#"{"renderer":"opengl","width":1024,"height":768,"fullscreen":true}"#;
        let cfg: GameConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.width, 1024);
        assert_eq!(cfg.display, WindowMode::FullScreen);
        assert_eq!(cfg.scaling, ScalingMode::Native);
        assert!(cfg.sound_enabled);
        assert!((cfg.music_volume - 1.0).abs() < 0.01);
        assert_eq!(cfg.difficulty, Difficulty::Medium);
    }

    #[test]
    fn output_resolution_selects_authored_tiers_through_1024_in_every_presentation_mode() {
        let mut config = GameConfig {
            scaling: ScalingMode::FourThree,
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
                config.scaling = scaling;
                assert_eq!(config.system_graphics_variant(), variant);
            }
        }
        config.detail = GraphicsDetail::Low;
        assert_eq!(config.system_graphics_variant(), 0);
        config.detail = GraphicsDetail::High;
        config.scaling = ScalingMode::Native;
        assert_eq!(config.system_graphics_variant(), 2);
        config.scaling = ScalingMode::FourThree;
        assert_eq!(config.system_graphics_variant(), 2);
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
    fn display_field_reads_old_booleans_and_new_values() {
        for (json, display) in [
            (
                r#"{"renderer":"opengl","width":800,"height":600,"fullscreen":false}"#,
                WindowMode::Window,
            ),
            (
                r#"{"renderer":"opengl","width":800,"height":600,"fullscreen":true}"#,
                WindowMode::FullScreen,
            ),
            (
                r#"{"renderer":"opengl","width":800,"height":600,"display":"borderless"}"#,
                WindowMode::Borderless,
            ),
            (
                r#"{"renderer":"opengl","width":800,"height":600}"#,
                WindowMode::Window,
            ),
        ] {
            let cfg: GameConfig = serde_json::from_str(json).unwrap();
            assert_eq!(cfg.display, display, "{json}");
        }
    }

    #[test]
    fn exact_sizes_survive_normalization_and_small_ones_take_the_minimum() {
        for (input, expected) in [
            ((0, 0), (640, 480)),
            ((320, 240), (640, 480)),
            ((639, 1024), (640, 480)),
            ((640, 480), (640, 480)),
            ((799, 599), (799, 599)),
            ((768, 1024), (768, 1024)),
            ((1280, 720), (1280, 720)),
            ((3840, 2160), (3840, 2160)),
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
    fn selecting_display_and_resolution_follows_the_rows() {
        let modes = DisplayModes::new(
            Some((1920, 1080)),
            [(640, 480), (1024, 768), (1280, 720), (1920, 1080)],
        );
        let mut cfg = GameConfig::default(); // In a Window at 800x600
        assert_eq!(cfg.resolution(), (800, 600));
        assert_eq!(modes.selection(cfg.display, cfg.size()), Some(1));

        cfg.select_resolution(4, &modes);
        assert_eq!(cfg.size(), (1920, 1080));
        assert_eq!(cfg.system_graphics_variant(), 3);

        // Borderless shows the desktop, draws with its tier, and keeps the
        // saved size, so a window comes back at 800x600.
        cfg.select_resolution(1, &modes);
        assert_eq!(cfg.size(), (800, 600));
        cfg.desktop = modes.desktop;
        cfg.select_window_mode(WindowMode::Borderless, &modes);
        assert_eq!(cfg.size(), (800, 600));
        assert_eq!(cfg.resolution(), (1920, 1080));
        assert_eq!(cfg.resolution_label(), "1920x1080");
        assert_eq!(cfg.system_graphics_variant(), 3);
        // Its row has one entry; nothing else can be chosen.
        assert_eq!(modes.resolutions(cfg.display), [(1920, 1080)]);
        cfg.select_resolution(0, &modes);
        cfg.select_resolution(1, &modes);
        assert_eq!(cfg.size(), (800, 600));
        cfg.select_window_mode(WindowMode::Window, &modes);
        assert_eq!(cfg.size(), (800, 600));
        assert_eq!(cfg.system_graphics_variant(), 2);

        // Full Screen offers reported modes only: 800x600 is not one.
        cfg.select_window_mode(WindowMode::FullScreen, &modes);
        assert_eq!(cfg.size(), (1024, 768));
        assert_eq!(
            cfg.display_request(),
            DisplayRequest {
                mode: WindowMode::FullScreen,
                size: (1024, 768)
            }
        );
        // An index past the row leaves the choice alone.
        cfg.select_resolution(9, &modes);
        assert_eq!(cfg.size(), (1024, 768));
        // A monitor reporting nothing keeps the size and takes the mode.
        cfg.select_window_mode(WindowMode::Window, &modes);
        cfg.select_window_mode(WindowMode::FullScreen, &DisplayModes::default());
        assert_eq!(cfg.display, WindowMode::FullScreen);
        assert_eq!(cfg.size(), (1024, 768));
    }

    #[test]
    fn original_resolutions_are_the_high_display_records() {
        assert_eq!(ORIGINAL_RESOLUTIONS, [(640, 480), (800, 600), (1024, 768)]);
        let mut cfg = GameConfig::default();
        for (variant, size) in (1..).zip(ORIGINAL_RESOLUTIONS) {
            (cfg.width, cfg.height) = size;
            assert_eq!(cfg.system_graphics_variant(), variant);
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
