//! Timing loop shared by read-only retail capture commands.
//!
//! A stop marker is an opt-in external protocol: this module only checks for
//! its presence. It never creates, removes, or modifies the marker.

use std::path::{absolute, Path, PathBuf};
use std::thread::sleep;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StopReason {
    MaximumSamples,
    StopFile,
}

impl StopReason {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::MaximumSamples => "maximum_samples",
            Self::StopFile => "stop_file",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Outcome {
    pub(crate) reason: StopReason,
    pub(crate) samples_completed: u64,
    pub(crate) samples_requested: u64,
    pub(crate) elapsed_ms: f64,
}

pub(crate) fn ensure_stop_file_absent(stop_file: Option<&Path>) -> Result<(), String> {
    if stop_requested(stop_file)? {
        let path = stop_file.expect("a requested stop has a marker path");
        return Err(format!(
            "--stop-file {} already exists; remove the stale marker before capture",
            path.display()
        ));
    }
    Ok(())
}

fn stop_requested(stop_file: Option<&Path>) -> Result<bool, String> {
    let Some(path) = stop_file else {
        return Ok(false);
    };
    path.try_exists()
        .map_err(|error| format!("could not inspect stop file {}: {error}", path.display()))
}

pub(crate) fn ensure_distinct_capture_paths(
    stop_file: Option<&Path>,
    output: &Path,
) -> Result<(), String> {
    let Some(stop_file) = stop_file else {
        return Ok(());
    };
    let stop_file_key = comparison_path(stop_file)?;
    let output_key = comparison_path(output)?;
    if stop_file_key
        .to_string_lossy()
        .eq_ignore_ascii_case(&output_key.to_string_lossy())
    {
        return Err(format!(
            "--stop-file {} must not resolve to the output path {}",
            stop_file.display(),
            output.display()
        ));
    }
    Ok(())
}

fn comparison_path(path: &Path) -> Result<PathBuf, String> {
    let absolute = absolute(path)
        .map_err(|error| format!("could not resolve path {}: {error}", path.display()))?;
    if let Ok(canonical) = absolute.canonicalize() {
        return Ok(canonical);
    }
    let Some(file_name) = absolute.file_name() else {
        return Ok(absolute);
    };
    let Some(parent) = absolute.parent() else {
        return Ok(absolute);
    };
    Ok(parent
        .canonicalize()
        .unwrap_or_else(|_| parent.to_path_buf())
        .join(file_name))
}

pub(crate) fn run(
    seconds: f64,
    hz: u32,
    sample_fn: impl FnMut(u64, f64) -> Result<(), String>,
) -> Result<(), String> {
    run_stop_aware(seconds, hz, None, sample_fn).map(|_| ())
}

pub(crate) fn run_stop_aware(
    seconds: f64,
    hz: u32,
    stop_file: Option<&Path>,
    mut sample_fn: impl FnMut(u64, f64) -> Result<(), String>,
) -> Result<Outcome, String> {
    let total_samples = (seconds * hz as f64).ceil().max(1.0) as u64;
    let interval = Duration::from_secs_f64(1.0 / hz as f64);
    let start = Instant::now();
    for sample in 0..total_samples {
        let target = start + interval.mul_f64(sample as f64);
        let now = Instant::now();
        if target > now {
            sleep(target - now);
        }
        if stop_requested(stop_file)? {
            return Ok(Outcome {
                reason: StopReason::StopFile,
                samples_completed: sample,
                samples_requested: total_samples,
                elapsed_ms: start.elapsed().as_secs_f64() * 1000.0,
            });
        }
        sample_fn(sample, start.elapsed().as_secs_f64() * 1000.0)?;
    }
    // A marker may arrive while the final callback is running. Recheck before
    // reporting natural exhaustion so the external shutdown handshake cannot
    // lose that boundary race.
    if stop_requested(stop_file)? {
        return Ok(Outcome {
            reason: StopReason::StopFile,
            samples_completed: total_samples,
            samples_requested: total_samples,
            elapsed_ms: start.elapsed().as_secs_f64() * 1000.0,
        });
    }
    Ok(Outcome {
        reason: StopReason::MaximumSamples,
        samples_completed: total_samples,
        samples_requested: total_samples,
        elapsed_ms: start.elapsed().as_secs_f64() * 1000.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{remove_file, File};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_STOP_FILE: AtomicU64 = AtomicU64::new(0);

    fn unused_stop_file() -> PathBuf {
        let sequence = NEXT_STOP_FILE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "v2k-inspect-stop-{}-{sequence}.marker",
            std::process::id()
        ))
    }

    #[test]
    fn stale_stop_file_is_rejected_before_capture() {
        let path = unused_stop_file();
        File::create(&path).expect("create stale stop marker");

        let error = ensure_stop_file_absent(Some(&path)).expect_err("reject stale marker");
        assert!(error.contains("already exists"));

        remove_file(path).expect("remove stale stop marker");
    }

    #[test]
    fn stop_file_must_not_alias_the_output_path() {
        let path = unused_stop_file();
        let dotted_alias = path
            .parent()
            .expect("temporary parent")
            .join(".")
            .join(path.file_name().expect("temporary file name"));

        let error = ensure_distinct_capture_paths(Some(&dotted_alias), &path)
            .expect_err("reject aliased capture paths");
        assert!(error.contains("must not resolve"));
    }

    #[test]
    fn stop_file_ends_timeline_before_the_next_sample() {
        let path = unused_stop_file();
        ensure_stop_file_absent(Some(&path)).expect("marker starts absent");
        let mut sampled = 0;

        let outcome = run_stop_aware(0.05, 1000, Some(&path), |_, _| {
            sampled += 1;
            File::create(&path).map_err(|error| error.to_string())?;
            Ok(())
        })
        .expect("stop-aware timeline");

        assert_eq!(outcome.reason, StopReason::StopFile);
        assert_eq!(outcome.samples_completed, 1);
        assert_eq!(outcome.samples_requested, 50);
        assert!(outcome.elapsed_ms >= 0.0);
        assert_eq!(sampled, 1);
        remove_file(path).expect("remove stop marker");
    }

    #[test]
    fn stop_file_present_before_sample_zero_takes_no_samples() {
        let path = unused_stop_file();
        ensure_stop_file_absent(Some(&path)).expect("marker starts absent");
        File::create(&path).expect("request immediate stop");
        let mut sampled = 0;

        let outcome = run_stop_aware(0.05, 1000, Some(&path), |_, _| {
            sampled += 1;
            Ok(())
        })
        .expect("immediately stopped timeline");

        assert_eq!(outcome.reason, StopReason::StopFile);
        assert_eq!(outcome.samples_completed, 0);
        assert_eq!(outcome.samples_requested, 50);
        assert_eq!(sampled, 0);
        remove_file(path).expect("remove stop marker");
    }

    #[test]
    fn absent_stop_file_allows_the_maximum_sample_count() {
        let path = unused_stop_file();
        let mut sampled = 0;

        let outcome = run_stop_aware(0.05, 1, Some(&path), |_, _| {
            sampled += 1;
            Ok(())
        })
        .expect("complete timeline");

        assert_eq!(outcome.reason, StopReason::MaximumSamples);
        assert_eq!(outcome.samples_completed, 1);
        assert_eq!(outcome.samples_requested, 1);
        assert_eq!(sampled, 1);
    }

    #[test]
    fn stop_file_created_by_the_final_sample_wins_the_boundary_race() {
        let path = unused_stop_file();
        ensure_stop_file_absent(Some(&path)).expect("marker starts absent");

        let outcome = run_stop_aware(0.05, 1, Some(&path), |_, _| {
            File::create(&path).map_err(|error| error.to_string())?;
            Ok(())
        })
        .expect("final-sample stop-aware timeline");

        assert_eq!(outcome.reason, StopReason::StopFile);
        assert_eq!(outcome.samples_completed, 1);
        assert_eq!(outcome.samples_requested, 1);
        remove_file(path).expect("remove stop marker");
    }
}
