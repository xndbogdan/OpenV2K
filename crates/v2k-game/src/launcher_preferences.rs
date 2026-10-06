//! Launcher preferences are separate from native gameplay settings and saves.
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct LauncherPreferences {
    pub data_dir: Option<PathBuf>,
    pub last_image: Option<PathBuf>,
    pub skip_launcher: bool,
}

impl LauncherPreferences {
    pub fn load(path: &Path) -> Self {
        fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    /// Publish a complete document; retain the previous state if replacement fails.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        let backup = path.with_extension(format!("{}.bak", std::process::id()));
        let bytes = serde_json::to_vec_pretty(self)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        let result = (|| {
            file.write_all(&bytes)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            drop(file);
            let had_previous = path.exists();
            if had_previous {
                fs::rename(path, &backup)?;
            }
            if let Err(error) = fs::rename(&temporary, path) {
                if had_previous {
                    let _ = fs::rename(&backup, path);
                }
                return Err(error);
            }
            if had_previous {
                let _ = fs::remove_file(&backup);
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temporary);
        }
        result
    }
}

pub fn default_state_path() -> PathBuf {
    #[cfg(windows)]
    if let Some(root) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(root).join("V2000 Port").join("launcher.json");
    }
    if let Some(root) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(root).join("v2000-port").join("launcher.json");
    }
    if let Some(root) = std::env::var_os("HOME") {
        return PathBuf::from(root).join(".config/v2000-port/launcher.json");
    }
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("launcher.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_round_trip_without_gameplay_settings() {
        let directory =
            std::env::temp_dir().join(format!("v2k-launcher-prefs-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("launcher.json");
        let state = LauncherPreferences {
            data_dir: Some(PathBuf::from("Game Files/Ω")),
            last_image: Some(PathBuf::from("Disc/V2000.cue")),
            skip_launcher: true,
        };
        state.save(&path).unwrap();
        assert_eq!(LauncherPreferences::load(&path), state);
        let changed = LauncherPreferences {
            skip_launcher: false,
            ..state
        };
        changed.save(&path).unwrap();
        assert_eq!(LauncherPreferences::load(&path), changed);
        assert!(!directory.join("settings.json").exists());
        fs::remove_file(&path).unwrap();
        fs::remove_dir(directory).unwrap();
    }
    #[test]
    fn missing_or_malformed_state_uses_setup_defaults() {
        let path =
            std::env::temp_dir().join(format!("v2k-launcher-corrupt-{}.json", std::process::id()));
        fs::write(&path, b"{ incomplete").unwrap();
        assert_eq!(
            LauncherPreferences::load(&path),
            LauncherPreferences::default()
        );
        fs::remove_file(&path).unwrap();
        assert_eq!(
            LauncherPreferences::load(&path),
            LauncherPreferences::default()
        );
    }
}
