//! Resolve launcher policy before creating SDL or writable game settings.
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::time::Duration;
use v2k_game::installation_discovery::registry_candidates;
use v2k_game::launcher_preferences::{default_state_path, LauncherPreferences};
use v2k_game::setup::{
    discover_installation, discover_setup_hint, validate_installation, DiscoveryRequest,
};

fn open_launcher(force: bool, bypass: bool, skip: bool, valid: bool, direct: bool) -> bool {
    force || (!bypass && (!valid || (!skip && !direct)))
}

pub(super) fn prepare(cli: &super::Cli) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let executable = std::env::current_exe()?;
    let preferences_path = cli
        .launcher_state
        .clone()
        .unwrap_or_else(default_state_path);
    let preferences = LauncherPreferences::load(&preferences_path);
    let direct = cli.level.is_some() || cli.vtol_trace.is_some();
    let explicit = cli
        .data_dir
        .clone()
        .or_else(|| direct.then(|| PathBuf::from(".")));
    let mut hints = registry_candidates();
    if let Ok(current) = std::env::current_dir() {
        hints.push(current);
    }
    let request = DiscoveryRequest {
        executable: executable.clone(),
        explicit,
        remembered: preferences.data_dir.clone(),
        registry_candidates: hints,
    };
    // GUI Play always belongs to the executable's own installation. Explicit
    // asset roots remain available to direct command-line diagnostic runs.
    let local_request = DiscoveryRequest {
        explicit: None,
        ..request.clone()
    };
    let local = discover_installation(&local_request);
    let discovered = if direct || cli.no_launcher || !cfg!(windows) {
        discover_installation(&request)
    } else {
        local.clone()
    };
    let valid = discovered.as_ref().is_some_and(|report| report.is_ready());
    let force = cli.launcher || cli.launcher_smoke_ms.is_some();
    let show_launcher = open_launcher(
        force,
        cli.no_launcher,
        preferences.skip_launcher,
        valid,
        direct,
    );
    let root = if show_launcher && cfg!(windows) {
        let selection = super::launcher::run(super::launcher::LauncherRequest {
            initial_root: local.as_ref().map(|report| report.root.clone()),
            setup_hint: discover_setup_hint(&request),
            preferences,
            preferences_path,
            executable: executable.clone(),
            // Windows SDL is statically linked, so the executable is portable.
            runtime_dependencies: Vec::new(),
            smoke_timeout: cli.launcher_smoke_ms.map(Duration::from_millis),
        })
        .map_err(std::io::Error::other)?;
        let Some(selection) = selection else {
            return Ok(None);
        };
        match selection {
            super::launcher::LaunchSelection::Play { data_dir } => data_dir,
            super::launcher::LaunchSelection::Restart { executable } => {
                restart_installed_launcher(&executable)?;
                return Ok(None);
            }
        }
    } else {
        discovered.map(|report| report.root).ok_or("Cannot locate the executable directory. Use --data-dir <folder> for a direct diagnostic run.")?
    };
    let report = validate_installation(&root);
    if !report.is_ready() {
        let details = report
            .issues
            .iter()
            .map(|issue| format!("{}: {}", issue.path.display(), issue.message))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!("V2000 installation is incomplete or damaged:\n{details}").into());
    }
    Ok(Some(std::fs::canonicalize(&root)?))
}

fn restart_arguments(
    arguments: impl IntoIterator<Item = OsString>,
    working_directory: &Path,
) -> Vec<OsString> {
    let mut result = Vec::new();
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        if argument == OsStr::new("--data-dir") {
            arguments.next();
        } else if argument == OsStr::new("--launcher-state")
            || argument == OsStr::new("--vtol-trace")
        {
            result.push(argument);
            if let Some(value) = arguments.next() {
                let path = PathBuf::from(value);
                result.push(if path.is_absolute() {
                    path.into_os_string()
                } else {
                    working_directory.join(path).into_os_string()
                });
            }
        } else if argument != OsStr::new("--no-launcher")
            && !argument.to_string_lossy().starts_with("--data-dir=")
        {
            let value = argument.to_string_lossy();
            if let Some((name, path)) =
                ["--launcher-state=", "--vtol-trace="]
                    .iter()
                    .find_map(|name| {
                        value
                            .strip_prefix(*name)
                            .map(|path| (*name, PathBuf::from(path)))
                    })
            {
                let path = if path.is_absolute() {
                    path
                } else {
                    working_directory.join(path)
                };
                let mut forwarded = OsString::from(name);
                forwarded.push(path);
                result.push(forwarded);
            } else {
                result.push(argument);
            }
        }
    }
    if !result
        .iter()
        .any(|argument| argument == OsStr::new("--launcher"))
    {
        result.push(OsString::from("--launcher"));
    }
    result
}

fn restart_installed_launcher(executable: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let root = executable
        .parent()
        .ok_or("Installed launcher has no parent directory")?;
    let report = validate_installation(root);
    if !report.is_ready() {
        return Err("The installed launcher directory no longer has complete game data. Reopen setup to repair it.".into());
    }
    std::process::Command::new(executable)
        .args(restart_arguments(
            std::env::args_os().skip(1),
            &std::env::current_dir()?,
        ))
        .current_dir(root)
        .spawn()
        .map_err(|error| {
            format!(
                "Could not open the installed launcher at {}: {error}",
                executable.display()
            )
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn setup_recovers_invalid_data_even_when_launcher_was_skipped() {
        assert!(open_launcher(false, false, true, false, false));
        assert!(!open_launcher(false, false, true, true, false));
        assert!(open_launcher(true, false, true, true, false));
    }
    #[test]
    fn direct_diagnostics_preserve_bypass_but_force_launcher_is_explicit() {
        assert!(!open_launcher(false, false, false, true, true));
        assert!(!open_launcher(false, true, false, false, true));
        assert!(open_launcher(true, false, false, true, true));
    }

    #[test]
    fn installed_restart_keeps_options_but_drops_previous_asset_root() {
        let working_directory = std::env::temp_dir();
        let state = working_directory.join("state.json").into_os_string();
        let arguments = [
            "--data-dir",
            "C:/old root",
            "--skip-intro",
            "--renderer",
            "software",
            "--launcher-state",
        ];
        let forwarded = restart_arguments(
            arguments
                .into_iter()
                .map(OsString::from)
                .chain([state.clone()]),
            &working_directory,
        );
        let expected = ["--skip-intro", "--renderer", "software", "--launcher-state"];
        assert_eq!(
            forwarded,
            expected
                .into_iter()
                .map(OsString::from)
                .chain([state, OsString::from("--launcher")])
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn installed_restart_removes_equal_asset_root_and_direct_bypass() {
        let arguments = ["--data-dir=C:/old root", "--no-launcher", "--launcher"];
        assert_eq!(
            restart_arguments(arguments.into_iter().map(OsString::from), Path::new(".")),
            vec![OsString::from("--launcher")]
        );
    }

    #[test]
    fn installed_restart_preserves_relative_output_locations() {
        let working_directory = std::env::temp_dir();
        let arguments = ["--launcher-state", "state.json", "--vtol-trace=trace.jsonl"];
        let result = restart_arguments(
            arguments.into_iter().map(OsString::from),
            &working_directory,
        );
        let mut trace = OsString::from("--vtol-trace=");
        trace.push(working_directory.join("trace.jsonl"));
        assert_eq!(
            result,
            vec![
                OsString::from("--launcher-state"),
                working_directory.join("state.json").into_os_string(),
                trace,
                OsString::from("--launcher")
            ]
        );
    }
}
