//! Port-only: crash reports to the port's Sentry project, while the
//! launcher's Options allow them (off until the player ticks the box). A
//! desktop start sends, on a thread of its own, every crash log in the log
//! folder that no report carried yet, each with its run's recording,
//! settings and saves ([`crate::crash_log`]'s `Replay:` line), the recording
//! and saves gzipped. Only repeats wait: a failure (its lines, digits
//! dropped) goes once from one machine for each version of the game, and a
//! log repeating one that went is passed over. `crash-reports-sent.txt`
//! beside the logs lists what went or was passed over, and each report's id
//! comes from its log, so Sentry drops a report sent twice. The upload is
//! `curl`'s, which Windows 10 and Linux ship; without it nothing goes, and
//! the next start tries again.
//!
//! A report's attachments rebuild the run's files: `run-<time>.replay.gz`
//! unzipped is the recording, `settings.json` and `port-config.json` go in
//! its `run-<time>.settings` folder, and each `Slot<NN>.gz` (`retail-` ones
//! under `retail`) in its `run-<time>.saves` folder.
//!
//! Where reports go is the build's: `OPENV2K_SENTRY_DSN` when it is compiled
//! (`scripts/release.ps1` passes the project's). A build without one sends
//! nothing, and the launcher shows the row unavailable.

use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The list of logs already handled, in the log folder: each log's name,
/// its game's version and its failure's signature.
const LEDGER: &str = "crash-reports-sent.txt";
/// A crash log's name begins with this ([`crate::crash_log::FILE_NAME`]).
const LOG_PREFIX: &str = "v2k-crash-log-";

/// Whether this build has somewhere to send crash reports.
pub fn available() -> bool {
    endpoint().is_some()
}

/// The build's Sentry envelope endpoint.
fn endpoint() -> Option<String> {
    endpoint_of(option_env!("OPENV2K_SENTRY_DSN")?)
}

/// The envelope endpoint of Sentry DSN `SCHEME://KEY@HOST[/PATH]/PROJECT`.
fn endpoint_of(dsn: &str) -> Option<String> {
    let (scheme, rest) = dsn.trim().split_once("://")?;
    let (key, rest) = rest.split_once('@')?;
    // An old DSN's key carries its secret after a colon.
    let key = key.split(':').next()?;
    let (host, project) = rest.trim_end_matches('/').rsplit_once('/')?;
    (!key.is_empty() && !host.is_empty() && !project.is_empty()).then(|| {
        format!("{scheme}://{host}/api/{project}/envelope/?sentry_key={key}&sentry_version=7")
    })
}

/// Send the unsent crash logs in `logs` in the background.
pub fn send_in_background(logs: PathBuf) {
    let Some(endpoint) = endpoint() else {
        return;
    };
    let _ = std::thread::Builder::new()
        .name("crash-reports".into())
        .spawn(move || send_pending(&logs, |envelope| post(&endpoint, envelope)));
}

/// Send each crash log in `logs` that the ledger does not list, oldest
/// first, through `post` (which answers the HTTP status).
fn send_pending(logs: &Path, mut post: impl FnMut(&[u8]) -> io::Result<u16>) -> io::Result<()> {
    let ledger = logs.join(LEDGER);
    let mut handled: Vec<[String; 3]> = fs::read_to_string(&ledger)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t').map(String::from);
            Some([fields.next()?, fields.next()?, fields.next()?])
        })
        .collect();
    let mut pending: Vec<(u64, PathBuf)> = fs::read_dir(logs)?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let ours = name.starts_with(LOG_PREFIX) && name.ends_with(".txt");
            let written = unix(entry.metadata().ok()?.modified().ok()?);
            (ours && !handled.iter().any(|[sent, ..]| *sent == name))
                .then(|| (written, entry.path()))
        })
        .collect();
    pending.sort_unstable();
    for (written, path) in pending {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let text = scrub_home(&fs::read_to_string(&path)?);
        let (release, signature) = (release(&text).to_string(), signature(&text));
        let repeat = handled
            .iter()
            .any(|[_, version, sent]| *version == release && *sent == signature);
        if !repeat {
            let status = post(&envelope(&name, &text, written, &signature))?;
            // A refusal other than a busy server's would refuse again.
            if status != 200 && !(400..500).contains(&status) || status == 429 {
                return Ok(());
            }
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&ledger)?;
        writeln!(file, "{name}\t{release}\t{signature}")?;
        handled.push([name, release, signature]);
    }
    Ok(())
}

fn unix(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

/// The log with the home folder (which names the player's account) as `~`,
/// in plain paths and in the command line's escaped ones.
fn scrub_home(text: &str) -> String {
    let home =
        std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default();
    if home.len() < 4 {
        return text.to_string();
    }
    text.replace(&home.replace('\\', "\\\\"), "~")
        .replace(&home, "~")
}

/// The log's "What failed" lines.
fn failure(text: &str) -> Vec<&str> {
    text.lines()
        .skip_while(|line| !line.starts_with("What failed"))
        .skip(1)
        .take_while(|line| !line.trim().is_empty())
        .map(str::trim)
        .collect()
}

/// The failure with its digits dropped (addresses, frames, times), which
/// groups one bug's reports.
fn signature(text: &str) -> String {
    failure(text)
        .join(" ")
        .chars()
        .filter(|c| !c.is_ascii_digit() && *c != '\t')
        .take(400)
        .collect()
}

/// The value after `label` on the log's line that starts with it.
fn field<'a>(text: &'a str, label: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|line| line.trim_start().strip_prefix(label))
}

/// The version of the game that wrote the log.
fn release(text: &str) -> &str {
    field(text, "Executable: ")
        .and_then(|line| line.rsplit("v2k-game ").next())
        .filter(|version| version.chars().all(|c| c.is_ascii_digit() || c == '.'))
        .unwrap_or(env!("CARGO_PKG_VERSION"))
}

fn envelope(name: &str, text: &str, written: u64, signature: &str) -> Vec<u8> {
    let release = release(text);
    let mut id = [0u64; 2];
    for (seed, half) in id.iter_mut().enumerate() {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (seed, name, text).hash(&mut hasher);
        *half = hasher.finish();
    }
    let event_id = format!("{:016x}{:016x}", id[0], id[1]);
    let level = field(text, "State ")
        .and_then(|line| line.split("level ").nth(1))
        .and_then(|rest| rest.split(['.', ' ', ',']).next())
        .unwrap_or("none");
    let event = serde_json::json!({
        "event_id": event_id,
        "timestamp": written,
        "platform": "native",
        "level": "fatal",
        "release": format!("openv2k@{release}"),
        "message": { "formatted": failure(text).join("\n") },
        "fingerprint": [signature],
        "tags": { "os": std::env::consts::OS, "world": level },
    });
    let mut items: Vec<(serde_json::Value, Vec<u8>)> = vec![
        (
            serde_json::json!({ "type": "event" }),
            event.to_string().into_bytes(),
        ),
        (attachment(name, "text/plain"), text.as_bytes().to_vec()),
    ];
    // The run's recording, settings and saves beside it: what replays it.
    let recording = field(text, "Replay: ")
        .and_then(|line| line.split(" (").next())
        .map(PathBuf::from);
    if let Some(recording) = recording {
        let files = crate::session_log::SessionFiles::from_replay(&recording);
        let mut run_files = vec![(files.replay.clone(), String::new(), true)];
        for (dir, prefix, zipped) in [
            (files.settings, "", false),
            (files.saves.clone(), "", true),
            (files.saves.join("retail"), "retail-", true),
        ] {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            let mut names: Vec<PathBuf> = entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_file())
                .collect();
            names.sort();
            run_files.extend(
                names
                    .into_iter()
                    .map(|path| (path, prefix.to_string(), zipped)),
            );
        }
        run_files.push((files.start, String::new(), false));
        run_files.push((files.start_save, String::new(), true));
        for (path, prefix, zipped) in run_files {
            let (Ok(bytes), Some(file)) = (fs::read(&path), path.file_name()) else {
                continue;
            };
            let file = format!("{prefix}{}", file.to_string_lossy());
            items.push(if zipped {
                (
                    attachment(&format!("{file}.gz"), "application/gzip"),
                    gzip(&bytes),
                )
            } else {
                (attachment(&file, "application/json"), bytes)
            });
        }
    }
    let mut out = serde_json::json!({ "event_id": event_id })
        .to_string()
        .into_bytes();
    for (mut header, body) in items {
        header["length"] = body.len().into();
        out.push(b'\n');
        out.extend(header.to_string().into_bytes());
        out.push(b'\n');
        out.extend(body);
    }
    out.push(b'\n');
    out
}

fn attachment(filename: &str, content_type: &str) -> serde_json::Value {
    serde_json::json!({ "type": "attachment", "filename": filename, "content_type": content_type })
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    let _ = encoder.write_all(bytes);
    encoder.finish().unwrap_or_default()
}

/// POST `envelope` to `endpoint` with curl; its HTTP status.
fn post(endpoint: &str, envelope: &[u8]) -> io::Result<u16> {
    let mut command = Command::new("curl");
    command
        .args([
            "-sS",
            "--max-time",
            "300",
            "-o",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        ])
        .args([
            "-w",
            "%{http_code}",
            "-H",
            "Content-Type: application/x-sentry-envelope",
        ])
        .args(["--data-binary", "@-", endpoint])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        // No console window flashes up beside the game.
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn()?;
    child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("curl's input"))?
        .write_all(envelope)?;
    let output = child.wait_with_output()?;
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .map_err(|_| io::Error::other("curl gave no status"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log(failure: &str, version: &str) -> String {
        format!(
            "V2000 port crash log\nWritten: 20261110-075000 (local)\n\nWhat failed:\n  {failure}\n\nThe run:\n  Executable: X:\\OpenV2K.exe (17958912 bytes), v2k-game {version}\n  Replay: X:\\nowhere\\run-1.replay (v2k-game --replay-session FILE reruns it)\n\nThe game, as last noted:\n  State playing, level 33.\n  No player.\n"
        )
    }

    #[test]
    fn a_dsn_names_its_envelope_endpoint() {
        assert_eq!(
            endpoint_of("https://abc123@o1.ingest.example.io/42").as_deref(),
            Some(
                "https://o1.ingest.example.io/api/42/envelope/?sentry_key=abc123&sentry_version=7"
            )
        );
        assert_eq!(
            endpoint_of("https://abc:secret@example.com/sentry/7/").as_deref(),
            Some("https://example.com/sentry/api/7/envelope/?sentry_key=abc&sentry_version=7")
        );
        for broken in [
            "",
            "https://example.com/42",
            "https://abc@example.com",
            "abc@x/1",
        ] {
            assert_eq!(endpoint_of(broken), None, "{broken}");
        }
    }

    #[test]
    fn a_report_groups_by_its_failure_and_frames_its_items() {
        let text = log("Entity 412 failed at frame 9001", "0.8.0");
        assert_eq!(signature(&text), "Entity  failed at frame ");
        let envelope = envelope("v2k-crash-log-1.txt", &text, 1_793_770_200, "sig");
        let lines: Vec<&[u8]> = envelope.split(|&byte| byte == b'\n').collect();
        let json = |line: &[u8]| serde_json::from_slice::<serde_json::Value>(line).unwrap();
        let (header, item, event) = (json(lines[0]), json(lines[1]), json(lines[2]));
        assert_eq!(header["event_id"], event["event_id"]);
        assert_eq!(item["length"], lines[2].len());
        assert_eq!(event["release"], "openv2k@0.8.0");
        assert_eq!(event["tags"]["world"], "33");
        assert_eq!(event["fingerprint"][0], "sig");
        // The log follows; its missing recording adds nothing.
        assert_eq!(json(lines[3])["length"], text.len());
        assert_eq!(
            envelope.len(),
            lines[..4].iter().map(|l| l.len() + 1).sum::<usize>() + text.len() + 1
        );
    }

    /// One failure goes once a version: a log repeating it (its digits
    /// aside) is passed over, another failure or version goes, and a busy
    /// server leaves the rest for the next start.
    #[test]
    fn only_new_failures_are_sent_once_a_version() {
        let dir = std::env::temp_dir().join(format!("v2k-crash-reports-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let write = |index: u32, failure: &str, version: &str| {
            fs::write(
                dir.join(format!("{LOG_PREFIX}2026111{index}-000000.txt")),
                log(failure, version),
            )
            .unwrap();
        };
        write(1, "Entity 12 failed at 0048D891", "0.8.0");
        write(2, "Entity 77 failed at 0048D895", "0.8.0");
        write(3, "The world loader stopped", "0.8.0");
        let mut sent = Vec::new();
        let mut status = 429;
        let send = |sent: &mut Vec<String>, status: u16| {
            send_pending(&dir, |envelope| {
                let text = String::from_utf8_lossy(envelope);
                let first = text.lines().nth(2).unwrap();
                let event: serde_json::Value = serde_json::from_str(first).unwrap();
                sent.push(event["message"]["formatted"].as_str().unwrap().to_string());
                Ok(status)
            })
            .unwrap();
        };
        // Busy: nothing is recorded, so everything waits.
        send(&mut sent, status);
        assert_eq!(sent.len(), 1);
        assert!(!dir.join(LEDGER).exists());
        sent.clear();
        status = 200;
        send(&mut sent, status);
        assert_eq!(
            sent,
            ["Entity 12 failed at 0048D891", "The world loader stopped"]
        );
        let ledger = fs::read_to_string(dir.join(LEDGER)).unwrap();
        assert_eq!(ledger.lines().count(), 3, "{ledger}");
        // Nothing new: nothing goes.
        sent.clear();
        send(&mut sent, status);
        assert!(sent.is_empty());
        // The same failure in the next version goes once more.
        write(4, "Entity 5 failed at 0048D891", "0.9.0");
        write(5, "Entity 6 failed at 0048D891", "0.9.0");
        send(&mut sent, status);
        assert_eq!(sent, ["Entity 5 failed at 0048D891"]);
        fs::remove_dir_all(&dir).unwrap();
    }
}
