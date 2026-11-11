//! A game run with its port-only bookkeeping: the session's recording
//! (`--session-log`, and every desktop start's own in `logs/replays`), a
//! recorded session's replay (`--replay-session`), the crash log on a
//! failure and, for a desktop start, the crash reports Options allows.
//! Logs go in `logs` beside the executable, as the saves do.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use v2k_game::crash_log;
use v2k_game::launcher_preferences::{default_state_path, LauncherPreferences};
use v2k_game::session_log::{self, SessionFiles, SessionReplay};

use super::{launcher, launcher_startup, Cli};

/// How many runs `logs/replays` keeps.
const KEPT_REPLAYS: usize = 10;

/// Play: the launcher or a direct run, as `main` describes.
pub(super) fn play(mut cli: Cli) -> ExitCode {
    let raw: Vec<String> = std::env::args().collect();
    let desktop = cli.launcher || (!cli.no_launcher && !cli.direct_run());
    let logs = logs_folder();
    // Any failure from here on leaves a crash log (v2k-crash-log-<time>.txt).
    crash_log::install(crash_context(&logs, &raw, None, None, None));
    crash_log::set_notifier(launcher::show_error);
    let mut replay = None;
    if let Some(path) = cli.replay_session.clone() {
        match replay_cli(cli, &path) {
            Ok((replayed, recording, saves)) => {
                cli = replayed;
                replay = Some((recording, saves));
            }
            Err(error) => {
                return report_failure(
                    desktop,
                    &format!("--replay-session {}: {error}", path.display()),
                )
            }
        }
    }
    if cli.hidden
        && (cli.replay_session.is_none() || cli.resume)
        && (cli.frames.is_none() || cli.settings.is_none())
    {
        return report_failure(
            desktop,
            "--hidden requires --frames and --settings (a --replay-session without --resume needs neither)",
        );
    }
    if desktop {
        // The preference as Options last saved it. The logs are beside the
        // executable, so they go while the launcher is open.
        let preferences = cli
            .launcher_state
            .clone()
            .unwrap_or_else(default_state_path);
        if LauncherPreferences::load(&preferences).send_crash_reports {
            v2k_game::crash_reports::send_in_background(logs.clone());
        }
    }
    let data_dir = match launcher_startup::prepare(&cli) {
        Ok(Some(root)) => root,
        Ok(None) => return ExitCode::SUCCESS,
        Err(error) => {
            return report_failure(
                desktop || cli.launcher_smoke_ms.is_some(),
                &error.to_string(),
            )
        }
    };
    // What a replay of this run plays: a replay's own recording, else this
    // command line with the installation it plays.
    let args = match &replay {
        Some((recording, _)) => recording.args.clone(),
        None => with_data_dir(&raw, &data_dir),
    };
    let mut recording = None;
    let log = match cli.session_log.clone() {
        Some(dir) => {
            let dir = dir.unwrap_or_else(|| logs.clone());
            match start_session_log(&dir, &raw, &data_dir, cli.replay_session.as_deref()) {
                Ok((log, files)) => {
                    recording = Some(files);
                    Some(log)
                }
                Err(error) => return report_failure(desktop, &format!("--session-log: {error}")),
            }
        }
        None => None,
    };
    // A desktop start, or a resumed replay, keeps its own replay among the
    // newest in logs/replays, which its crash log names.
    if log.is_none() && (desktop || cli.resume) {
        match start_rolling_replay(&logs.join("replays")) {
            Ok(files) => recording = Some(files),
            Err(error) => eprintln!("This run keeps no replay: {error}"),
        }
    }
    if let (Some(files), Some(start)) = (&recording, &cli.start) {
        if let Err(error) = keep_start(start, files) {
            return report_failure(desktop, &format!("--start {}: {error}", start.display()));
        }
    }
    let crash_dir = log
        .as_ref()
        .and_then(|log| log.path().parent())
        .map_or_else(|| logs.clone(), Path::to_path_buf);
    crash_log::install(crash_context(
        &crash_dir,
        &raw,
        Some(&data_dir),
        log.as_ref().map(|log| log.path()),
        recording.as_ref().map(|files| files.replay.as_path()),
    ));
    // Without a session log the output still passes through a capture, so
    // a crash log can end with the last lines the game printed.
    let tail = match &log {
        Some(log) => {
            eprintln!("Session log: {}", log.path().display());
            None
        }
        None => session_log::OutputTail::start().ok(),
    };
    let detailed = log.is_some();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut probe = cli.probe_run()?;
        probe.record = recording.map(|files| super::probe::SessionRecord {
            path: files.replay,
            args,
        });
        if let Some((recording, saves)) = replay {
            if cli.hidden {
                // A hidden window opens at the size the display started at.
                probe.size = recording.display.or(probe.size);
            }
            probe.new_game_after_ticks = recording
                .env
                .iter()
                .find(|(name, _)| name == super::NEW_GAME_AFTER_TICKS)
                .and_then(|(_, value)| value.parse().ok());
            probe.replay = Some(recording);
            probe.resume = cli.resume;
            probe.saves = Some(saves);
        }
        super::run_cli_game(&cli, &data_dir, probe)
    }));
    crash_log::idle();
    let failure = match result {
        Ok(Ok(())) => None,
        Ok(Err(error)) => {
            let message = error.to_string();
            eprintln!("Error: {message}");
            let mut failure = vec![format!("V2000 stopped: {message}")];
            let mut source = error.source();
            while let Some(cause) = source {
                failure.push(format!("  caused by: {cause}"));
                source = cause.source();
            }
            // A direct run's error stays on its console; a desktop start's
            // or a logged session's also leaves a crash log.
            Some((message, failure, desktop || detailed))
        }
        Err(_) => {
            // The panic hook has printed and logged it (PANIC above).
            let message = "V2000 stopped: the game panicked.".to_string();
            eprintln!("{message}");
            let failure = crash_log::last_panic().unwrap_or_else(|| vec![message.clone()]);
            Some((message, failure, true))
        }
    };
    // Drain the captured output, so a crash log ends with its last lines.
    drop(tail);
    if let Some(log) = log {
        let path = log.path().to_path_buf();
        drop(log);
        eprintln!("Session log written: {}", path.display());
    }
    let Some((message, failure, logged)) = failure else {
        return ExitCode::SUCCESS;
    };
    let crash = logged.then(|| crash_log::write(&failure)).flatten();
    if let Some(path) = &crash {
        eprintln!("Crash log written: {}", path.display());
    }
    if desktop {
        launcher::show_error(&crash_message(&message, crash.as_deref()));
    }
    ExitCode::FAILURE
}

/// `logs` beside the executable.
fn logs_folder() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_default()
        .join("logs")
}

/// A failure before the game runs: the console, and for a desktop start
/// the crash log and a message box naming it.
fn report_failure(desktop: bool, message: &str) -> ExitCode {
    eprintln!("Error: {message}");
    if desktop {
        let crash = crash_log::write(&[message.to_string()]);
        if let Some(path) = &crash {
            eprintln!("Crash log written: {}", path.display());
        }
        launcher::show_error(&crash_message(message, crash.as_deref()));
    }
    ExitCode::FAILURE
}

/// A failure's message, and where its crash log is.
fn crash_message(message: &str, crash: Option<&Path>) -> String {
    match crash {
        Some(path) => format!("{message}\n\nA crash log was saved to:\n{}", path.display()),
        None => message.to_string(),
    }
}

/// `--replay-session`: the recorded command line, with this run's own
/// presentation and logging flags, over working copies of the recorded
/// settings and saves (the replay may save into them). Also the recording
/// and the saves' copy.
fn replay_cli(current: Cli, path: &Path) -> Result<(Cli, SessionReplay, PathBuf), Box<dyn Error>> {
    use clap::Parser as _;
    let replay = SessionReplay::read(path)?;
    if replay.args.is_empty() {
        return Err("the replay holds no command line".into());
    }
    let mut cli = Cli::try_parse_from(&replay.args)?;
    let files = SessionFiles::from_replay(path);
    let working = |source: &Path, suffix: &str| -> Result<PathBuf, Box<dyn Error>> {
        if !source.is_dir() {
            return Err(format!("{} is missing", source.display()).into());
        }
        let mut name = path.with_extension("").into_os_string();
        name.push(suffix);
        let copy = PathBuf::from(name);
        if copy.exists() {
            std::fs::remove_dir_all(&copy)?;
        }
        copy_folder(source, &copy)?;
        Ok(copy)
    };
    cli.settings = Some(match current.settings {
        Some(settings) => settings,
        None => working(&files.settings, ".replay-settings")?,
    });
    let saves = working(&files.saves, ".replay-saves")?;
    if files.start.is_file() {
        cli.start = Some(files.start.clone());
    }
    if current.data_dir.is_some() {
        cli.data_dir = current.data_dir;
    }
    if current.renderer != "auto" {
        cli.renderer = current.renderer;
    }
    cli.launcher = false;
    cli.launcher_smoke_ms = None;
    cli.verbose |= current.verbose;
    cli.hidden = current.hidden;
    cli.frames = current.frames;
    cli.snapshot = current.snapshot;
    cli.snapshot_at = current.snapshot_at;
    cli.event_log = current.event_log;
    cli.actor_log = current.actor_log;
    cli.vtol_trace = current.vtol_trace;
    cli.session_log = current.session_log;
    // The recording holds the keys a scripted run pressed.
    cli.input_replay = None;
    cli.replay_session = Some(path.to_path_buf());
    cli.resume = current.resume;
    Ok((cli, replay, saves))
}

/// Copy `source`'s files and folders into the new folder `copy`.
fn copy_folder(source: &Path, copy: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(copy)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let target = copy.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_folder(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// `raw` with `--data-dir root` in place of any data folder it named.
fn with_data_dir(raw: &[String], root: &Path) -> Vec<String> {
    let mut result = Vec::new();
    let mut arguments = raw.iter();
    while let Some(argument) = arguments.next() {
        if argument == "--data-dir" {
            arguments.next();
        } else if !argument.starts_with("--data-dir=") {
            result.push(argument.clone());
        }
    }
    result.push("--data-dir".to_string());
    result.push(root.display().to_string());
    result
}

/// Keep this run's replay among the newest [`KEPT_REPLAYS`] in `dir`.
fn start_rolling_replay(dir: &Path) -> std::io::Result<SessionFiles> {
    std::fs::create_dir_all(dir)?;
    prune_replays(dir, KEPT_REPLAYS - 1)?;
    Ok(SessionFiles::new(dir, &session_log::local_stamp()))
}

/// Remove all but the newest `keep` runs from `dir`: each `run-<time>.replay`
/// and every file and folder that shares its stem.
fn prune_replays(dir: &Path, keep: usize) -> std::io::Result<()> {
    let names: Vec<String> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .collect();
    let mut stems: Vec<&str> = names
        .iter()
        .filter_map(|name| name.strip_suffix(".replay"))
        .collect();
    stems.sort_unstable();
    let excess = stems.len().saturating_sub(keep);
    for stem in &stems[..excess] {
        for name in names
            .iter()
            .filter(|name| name.starts_with(&format!("{stem}.")))
        {
            let path = dir.join(name);
            if path.is_dir() {
                std::fs::remove_dir_all(path)?;
            } else {
                std::fs::remove_file(path)?;
            }
        }
    }
    Ok(())
}

/// Start `--session-log` in `dir`: the log, whose header names the files
/// the run keeps beside it.
fn start_session_log(
    dir: &Path,
    raw: &[String],
    data_dir: &Path,
    replaying: Option<&Path>,
) -> Result<(session_log::SessionLog, SessionFiles), Box<dyn Error>> {
    std::fs::create_dir_all(dir)?;
    let stamp = session_log::local_stamp();
    let files = SessionFiles::new(dir, &stamp);
    let header = vec![
        "V2000 port session log".to_string(),
        format!("Started: {stamp} (local)"),
        executable_line(),
        format!("Command line: {}", serde_json::to_string(raw)?),
        format!("Data folder: {}", data_dir.display()),
        format!(
            "Replay: {}{}",
            files.replay.display(),
            replaying
                .map(|source| format!(" (this run replays {})", source.display()))
                .unwrap_or_default()
        ),
        format!(
            "Settings and saves the run starts from: {} and {}",
            files.settings.display(),
            files.saves.display()
        ),
        platform_line(),
    ];
    let log = session_log::SessionLog::start(&files.log, &header)?;
    Ok((log, files))
}

/// Keep a `--start` state, and the save it names, beside the recording.
fn keep_start(start: &Path, files: &SessionFiles) -> Result<(), Box<dyn Error>> {
    let mut state: serde_json::Value = serde_json::from_slice(&std::fs::read(start)?)?;
    if let Some(save) = state.get("save").and_then(|save| save.as_str()) {
        // A relative save path is the state file's neighbour.
        let source = start.parent().unwrap_or(Path::new(".")).join(save);
        std::fs::copy(&source, &files.start_save)?;
        state["save"] = files
            .start_save
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .into();
    }
    std::fs::write(&files.start, serde_json::to_vec_pretty(&state)?)?;
    Ok(())
}

/// The executable with its size and time, and the port's version.
fn executable_line() -> String {
    let exe = std::env::current_exe().ok();
    let build = exe
        .as_ref()
        .and_then(|path| std::fs::metadata(path).ok())
        .map(|meta| {
            let modified = meta
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |time| time.as_secs());
            format!("{} bytes, modified unix {modified}", meta.len())
        })
        .unwrap_or_default();
    format!(
        "Executable: {} ({build}), v2k-game {}",
        exe.map(|path| path.display().to_string())
            .unwrap_or_default(),
        env!("CARGO_PKG_VERSION")
    )
}

fn platform_line() -> String {
    format!(
        "Platform: {} {}, {} logical CPUs",
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::thread::available_parallelism().map_or(0, |count| count.get())
    )
}

/// Where a crash log goes (`dir`) and the run it describes.
fn crash_context(
    dir: &Path,
    raw: &[String],
    data_dir: Option<&Path>,
    session_log: Option<&Path>,
    replay: Option<&Path>,
) -> crash_log::CrashContext {
    let mut run = vec![
        executable_line(),
        format!(
            "Command line: {}",
            serde_json::to_string(raw).unwrap_or_default()
        ),
    ];
    if let Some(data_dir) = data_dir {
        run.push(format!("Data folder: {}", data_dir.display()));
    }
    run.push(platform_line());
    run.push(match session_log {
        Some(path) => format!("Session log: {}", path.display()),
        None => "Session log: none (start with --session-log to keep the whole run)".to_string(),
    });
    if let Some(replay) = replay {
        run.push(format!(
            "Replay: {} (v2k-game --replay-session FILE reruns it; add --resume to play on from where it ends)",
            replay.display()
        ));
    }
    crash_log::CrashContext {
        path: dir.join(crash_log::FILE_NAME),
        run,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A desktop start's replay names the installation it played.
    #[test]
    fn a_recorded_command_line_names_the_data_folder_once() {
        let raw = [
            "v2k.exe",
            "--data-dir",
            "old",
            "--launcher",
            "--data-dir=older",
        ]
        .map(String::from);
        assert_eq!(
            with_data_dir(&raw, Path::new("D:/Games/V2000")),
            ["v2k.exe", "--launcher", "--data-dir", "D:/Games/V2000"]
        );
    }

    /// Only the newest runs stay, each with every file and folder of its stem.
    #[test]
    fn rolling_replays_keep_the_newest_runs_whole() {
        let dir = std::env::temp_dir().join(format!("v2k-replays-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for stamp in ["20261110-100000", "20261110-110000", "20261110-120000"] {
            let files = SessionFiles::new(&dir, stamp);
            std::fs::create_dir_all(&files.saves).unwrap();
            std::fs::write(files.saves.join("Slot01"), b"save").unwrap();
            std::fs::write(&files.replay, b"replay").unwrap();
        }
        let files = start_rolling_replay(&dir).unwrap();
        assert!(files.replay.starts_with(&dir));
        prune_replays(&dir, 1).unwrap();
        let mut left: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        left.sort();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(
            left,
            ["run-20261110-120000.replay", "run-20261110-120000.saves"]
        );
    }
}
