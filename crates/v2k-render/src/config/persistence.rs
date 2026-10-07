//! 449140/4491D0 persist 15 DWORDs from 4CB3D8, using table 4C2160.
//! The original registry key is import-only. The port owns its key and files.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use super::{Difficulty, GameConfig, GraphicsDetail, RendererChoice, ScalingMode};

#[cfg(windows)]
mod registry;

#[cfg(any(windows, test))]
pub(super) const RETAIL_KEY: &str = r"Software\Frontier Developments Ltd\V2000\1.0";
#[cfg(any(windows, test))]
pub(super) const PORT_KEY: &str = r"Software\V2000 Port\1.0";

// (registry name, offset in the retail settings struct, initial DWORD).
// +20 and +40 are not in the registry table. In particular +40 Vibration
// is not a sixteenth persisted value, and Game type is not port Difficulty.
const FIELDS: [(&str, u32, u32); 15] = [
    ("Sound On", 0x00, 15),
    ("Ambient On", 0x04, 15),
    ("Joystick Mode", 0x08, 1),
    ("Game type", 0x0c, 0),
    ("Resolution", 0x10, 1),
    ("Renderer", 0x14, 1),
    ("Bilinear Filtering", 0x18, 1),
    ("Full Screen", 0x1c, 1),
    ("Self Righting", 0x24, 1),
    ("Full Analogue", 0x28, 0),
    ("Sensitivity", 0x2c, 10),
    ("Camera", 0x30, 6),
    ("Targetter", 0x34, 1),
    ("HUD", 0x38, 1),
    ("Language", 0x3c, 0),
];

/// Named native DWORDs. Unknown values in the portable fallback survive edits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub(super) struct NativeSettings(BTreeMap<String, u32>);

impl Default for NativeSettings {
    fn default() -> Self {
        Self(FIELDS.map(|(name, _, value)| (name.into(), value)).into())
    }
}

impl NativeSettings {
    fn get(&self, index: usize) -> u32 {
        self.0
            .get(FIELDS[index].0)
            .copied()
            .unwrap_or(FIELDS[index].2)
    }

    fn set(&mut self, index: usize, value: u32) {
        self.0.insert(FIELDS[index].0.into(), value);
    }

    fn apply(&self, config: &mut GameConfig) {
        config.sound_enabled = self.get(0) != 0;
        config.sfx_volume = self.get(0).min(15) as f32 / 15.0;
        config.ambient_enabled = self.get(1) != 0;
        config.music_volume = self.get(1).min(15) as f32 / 15.0;
        config.joystick_mode = self.get(2).min(1) as u8;
        // Native Resolution selects authored tiers, not the modern preset list.
        let tier = self.get(4);
        if let Some(&(width, height)) =
            [(320, 240), (640, 480), (800, 600), (1024, 768)].get(tier as usize)
        {
            config.width = width;
            config.height = height;
        }
        config.detail = if tier == 0 {
            GraphicsDetail::Low
        } else {
            GraphicsDetail::High
        };
        config.renderer = if self.get(5) == 0 {
            RendererChoice::Software
        } else {
            RendererChoice::OpenGL
        };
        config.bilinear_filtering = self.get(6) != 0;
        config.fullscreen = self.get(7) != 0;
        config.self_righting = self.get(8).min(15) as u8;
        config.absolute_mode = self.get(9) != 0;
        config.sensitivity = self.get(10).min(15) as f32 / 15.0;
        config.active_camera = self.get(11).min(10) as u8;
        config.targetter = self.get(12) != 0;
        config.hud = self.get(13) != 0;
        config.language = self.get(14).min(u8::MAX as u32) as u8;
    }

    fn from_config(config: &GameConfig) -> Self {
        let mut result = Self::default();
        let volume = |enabled, value: f32| {
            if enabled {
                (value.clamp(0.0, 1.0) * 15.0).round() as u32
            } else {
                0
            }
        };
        let values = [
            volume(config.sound_enabled, config.sfx_volume),
            volume(config.ambient_enabled, config.music_volume),
            config.joystick_mode.min(1) as u32,
            0, // Game type is retained, never inferred from Difficulty.
            u32::from(config.detail == GraphicsDetail::High),
            u32::from(config.renderer != RendererChoice::Software),
            config.bilinear_filtering as u32,
            config.fullscreen as u32,
            config.self_righting.min(15) as u32,
            config.absolute_mode as u32,
            volume(true, config.sensitivity),
            config.active_camera.min(10) as u32,
            config.targetter as u32,
            config.hud as u32,
            config.language as u32,
        ];
        for (index, value) in values.into_iter().enumerate() {
            result.set(index, value);
        }
        result
    }
}

/// Retains DWORDs the runtime cannot represent, without coercing them on save.
#[derive(Debug, Clone)]
pub struct SettingsSnapshot {
    native: NativeSettings,
    projected: NativeSettings,
}

fn native_to_save(config: &GameConfig) -> NativeSettings {
    let current = NativeSettings::from_config(config);
    let Some(snapshot) = &config.settings_snapshot else {
        return current;
    };
    let mut result = snapshot.native.clone();
    for index in 0..FIELDS.len() {
        if index != 3 && current.get(index) != snapshot.projected.get(index) {
            result.set(index, current.get(index));
        }
    }
    result
}

/// Genuine port policies do not become invented values in the retail table.
#[derive(Debug, Serialize, Deserialize)]
struct PortPreferences {
    renderer: RendererChoice,
    width: u32,
    height: u32,
    #[serde(default)]
    scaling: ScalingMode,
    #[serde(default)]
    detail: GraphicsDetail,
    #[serde(default)]
    difficulty: Difficulty,
}

impl PortPreferences {
    fn from_config(config: &GameConfig) -> Self {
        Self {
            renderer: config.renderer.clone(),
            width: config.width,
            height: config.height,
            scaling: config.scaling,
            detail: config.detail,
            difficulty: config.difficulty,
        }
    }
    fn apply(self, config: &mut GameConfig) {
        config.renderer = self.renderer;
        config.width = self.width;
        config.height = self.height;
        config.scaling = self.scaling;
        config.detail = self.detail;
        config.difficulty = self.difficulty;
    }
}

pub(super) fn load(dir: &Path) -> GameConfig {
    #[cfg(windows)]
    let sources = (registry::read(PORT_KEY), registry::read(RETAIL_KEY));
    #[cfg(not(windows))]
    let sources = (None, None);
    load_from_sources(dir, sources.0, sources.1)
}

pub(super) fn retail_save_directory() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    {
        registry::read_save_directory()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn load_from_sources(
    dir: &Path,
    port_registry: Option<NativeSettings>,
    retail: Option<NativeSettings>,
) -> GameConfig {
    // A prior port config takes precedence over the original installation.
    // Merely loading settings performs no writes or migration renames.
    let legacy: Option<GameConfig> = read_json(&dir.join("config.json"));
    let fallback: Option<NativeSettings> = read_json(&dir.join("settings.json"));
    let owned = port_registry
        .map(|registry| {
            let mut merged = fallback.clone().unwrap_or_default();
            merged.0.extend(registry.0);
            merged
        })
        .or(fallback);
    let mut config = legacy.clone().unwrap_or_default();
    let native = if let Some(native) = owned {
        native.apply(&mut config);
        native
    } else if legacy.is_some() {
        // Preserve even the unowned Game type when importing a legacy config.
        let mut native = NativeSettings::from_config(&config);
        if let Some(retail) = retail {
            native.set(3, retail.get(3));
        }
        native
    } else if let Some(native) = retail {
        native.apply(&mut config);
        native
    } else {
        NativeSettings::from_config(&config)
    };
    // Legacy presentation preferences take precedence over native tier values;
    // unsupported dimensions are normalized after all sources are applied.
    if let Some(legacy) = legacy {
        PortPreferences::from_config(&legacy).apply(&mut config);
    }
    if let Some(port) = read_json::<PortPreferences>(&dir.join("port-config.json")) {
        port.apply(&mut config);
    }
    config.normalize_presentation();
    config.settings_snapshot = Some(SettingsSnapshot {
        native,
        projected: NativeSettings::from_config(&config),
    });
    config
}

pub(super) fn save(config: &mut GameConfig, dir: &Path) -> io::Result<()> {
    save_with_registry(config, dir, |native| {
        #[cfg(windows)]
        {
            registry::write(PORT_KEY, native)
        }
        #[cfg(not(windows))]
        {
            let _ = native;
            Ok(())
        }
    })
}

fn save_with_registry(
    config: &mut GameConfig,
    dir: &Path,
    write_registry: impl FnOnce(&NativeSettings) -> io::Result<()>,
) -> io::Result<()> {
    config.normalize_presentation();
    let native = native_to_save(config);
    fs::create_dir_all(dir)?;
    // Do not claim a successful fallback after a failed Windows registry
    // write: the old registry record would take precedence on the next load.
    write_registry(&native)?;
    atomic_json(&dir.join("settings.json"), &native)?;
    atomic_json(
        &dir.join("port-config.json"),
        &PortPreferences::from_config(config),
    )?;
    config.settings_snapshot = Some(SettingsSnapshot {
        native,
        projected: NativeSettings::from_config(config),
    });
    Ok(())
}

fn atomic_json(path: &Path, value: &impl Serialize) -> io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    let temporary = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = file.write_all(&bytes).and_then(|()| file.sync_all());
    drop(file);
    let result = result.and_then(|()| fs::rename(&temporary, path));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests;
