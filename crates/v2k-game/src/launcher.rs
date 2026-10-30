//! The executable's pre-game setup window. All asset work happens before SDL.
//!
//! This is port setup policy, not a reconstructed retail game screen. The native
//! controls deliberately use the classic Windows appearance and keyboard model.

use std::path::PathBuf;
use std::time::Duration;

use crate::launcher_preferences::LauncherPreferences;
use crate::setup::{IssueSeverity, ValidationReport};

#[derive(Debug)]
pub struct LauncherRequest {
    pub initial_root: Option<PathBuf>,
    /// An installation hint is only a chooser default, never runtime admission.
    pub setup_hint: Option<PathBuf>,
    pub preferences: LauncherPreferences,
    pub preferences_path: PathBuf,
    pub executable: PathBuf,
    // Windows setup copies these beside the executable; Linux never copies it.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub runtime_dependencies: Vec<PathBuf>,
    /// An integration-smoke timeout; ordinary launches leave this unset.
    pub smoke_timeout: Option<Duration>,
}

#[derive(Debug)]
pub enum LaunchSelection {
    Play {
        data_dir: PathBuf,
    },
    #[cfg_attr(not(windows), allow(dead_code))]
    Restart {
        executable: PathBuf,
    },
}

mod header_animation;
#[cfg(not(windows))]
mod linux;
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
        linux::run(request)
    }
}

pub(super) fn music_description(report: &ValidationReport) -> String {
    let count = report.music.available_count();
    if report.music.directory_missing {
        "Music directory missing.".to_string()
    } else if count == 0 {
        "Music disabled: no soundtrack tracks are available.".to_string()
    } else if report.music.missing_tracks().is_empty() {
        "Music available: all 10 CD audio tracks.".to_string()
    } else {
        format!("Music incomplete: {count} of 10 tracks available.")
    }
}

pub(super) fn report_description(report: &ValidationReport) -> String {
    let mut lines = vec![format!(
        "Game files: {}. Checked {} overlays.",
        if report.is_ready() {
            "ready"
        } else {
            "missing or inconsistent"
        },
        report.checked_overlays
    )];
    // Missing-directory guidance is already a path-bearing validation warning.
    if !report.music.directory_missing {
        lines.push(music_description(report));
    }
    if !report.music.missing_tracks().is_empty() && report.music.available_count() > 0 {
        lines.push(format!(
            "Missing music tracks: {}.",
            report
                .music
                .missing_tracks()
                .iter()
                .map(|track| format!("{track:02}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if report.embedded_checksums_verified {
        lines.push("Bundled retail checksums verified.".to_string());
    } else if report.manifest_verified {
        lines.push("Installation checksum manifest verified.".to_string());
    }
    for issue in &report.issues {
        // One absent-directory warning also covers optional manifest tracks.
        if report.music.directory_missing
            && issue.severity == IssueSeverity::Warning
            && issue.path != report.music.directory
            && issue.path.starts_with(&report.music.directory)
        {
            continue;
        }
        lines.push(format!("{}\r\n{}", issue.path.display(), issue.message));
    }
    for error in &report.music.scan_errors {
        lines.push(error.clone());
    }
    for track in &report.music.tracks {
        for error in &track.errors {
            lines.push(format!("Music track {:02}: {error}", track.cd_track));
        }
    }
    lines.join("\r\n")
}

/// A completely empty directory has hundreds of diagnostics. Keep the native
/// popup within the screen; the main read-only, scrollable control retains all.
pub(super) fn popup_description(report: &ValidationReport) -> String {
    let description = report_description(report);
    let lines: Vec<_> = description.lines().collect();
    if lines.len() <= 12 {
        description
    } else {
        format!(
            "{}\r\n\r\n{} additional details are available in the main window's scrollable list.",
            lines[..12].join("\r\n"),
            lines.len() - 12
        )
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
    {
        // A desktop start has no terminal; a shell still gets the text.
        eprintln!("V2000: {message}");
        linux::show_error(message);
    }
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
