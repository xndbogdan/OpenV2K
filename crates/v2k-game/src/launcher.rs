//! The executable's pre-game setup window. All asset work happens before SDL.
//!
//! This is port setup policy, not a reconstructed retail game screen. The native
//! controls deliberately use the classic Windows appearance and keyboard model.

use std::path::PathBuf;
use std::time::Duration;

use crate::launcher_preferences::LauncherPreferences;

#[derive(Debug)]
pub struct LauncherRequest {
    pub initial_root: Option<PathBuf>,
    /// An installation hint is only a chooser default, never runtime admission.
    pub setup_hint: Option<PathBuf>,
    pub preferences: LauncherPreferences,
    pub preferences_path: PathBuf,
    pub executable: PathBuf,
    pub runtime_dependencies: Vec<PathBuf>,
    /// An integration-smoke timeout; ordinary launches leave this unset.
    pub smoke_timeout: Option<Duration>,
}

#[derive(Debug)]
pub enum LaunchSelection {
    Play { data_dir: PathBuf },
    Restart { executable: PathBuf },
}

#[cfg(windows)]
mod native;

/// A selected installation admits this process only when its data is beside
/// the executable. Remembered locations and current directories are setup hints.
#[cfg(any(windows, test))]
fn is_executable_installation(root: &std::path::Path, executable: &std::path::Path) -> bool {
    executable.parent().is_some_and(|executable_root| {
        match (
            std::fs::canonicalize(root),
            std::fs::canonicalize(executable_root),
        ) {
            (Ok(root), Ok(executable_root)) => root == executable_root,
            _ => false,
        }
    })
}

/// Return after Play, a restart into the installed copy, or Quit.
pub fn run(request: LauncherRequest) -> Result<Option<LaunchSelection>, String> {
    #[cfg(windows)]
    {
        native::run(request)
    }
    #[cfg(not(windows))]
    {
        let root = request
            .initial_root
            .or_else(|| {
                request
                    .executable
                    .parent()
                    .map(std::path::Path::to_path_buf)
            })
            .ok_or_else(|| "Cannot determine the game-data directory".to_string())?;
        let report = crate::setup::validate_installation(&root);
        if report.is_ready() {
            Ok(Some(LaunchSelection::Play {
                data_dir: report.root,
            }))
        } else {
            Err(format!(
                "V2000 data at {} is incomplete. Use --data-dir with a valid installation or the setup commands to install a disc image.",
                root.display()
            ))
        }
    }
}

/// Use an existing parent console for CLI output without creating a console.
/// Redirected standard streams stay connected to the caller's pipes or files.
pub fn prepare_console() {
    #[cfg(windows)]
    native::attach_parent_console();
}

pub fn show_error(message: &str) {
    #[cfg(windows)]
    native::show_error(message);
    #[cfg(not(windows))]
    eprintln!("V2000: {message}");
}

#[cfg(test)]
mod tests {
    use super::is_executable_installation;

    #[test]
    fn placement_requires_the_executable_directory_and_resolves_aliases() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!(
            "v2k-launcher-placement-{}-{unique}",
            std::process::id()
        ));
        let installation = base.join("installed");
        let release = base.join("release");
        std::fs::create_dir_all(installation.join("child")).unwrap();
        std::fs::create_dir_all(&release).unwrap();
        assert!(is_executable_installation(
            &installation,
            &installation.join("v2k-game.exe"),
        ));
        assert!(is_executable_installation(
            &installation.join("child").join(".."),
            &installation.join("v2k-game.exe"),
        ));
        assert!(!is_executable_installation(
            &installation,
            &release.join("v2k-game.exe"),
        ));
        assert!(!is_executable_installation(
            &base.join("missing"),
            &installation.join("v2k-game.exe"),
        ));
        std::fs::remove_dir_all(base).unwrap();
    }
}
