//! Physical CD-track inventory shared by setup validation and game playback.
//!
//! The old player sorted existing OGG filenames densely, then the world mapper
//! clamped to that count. With missing track03, world2 played track04 instead.
//! Slots now always retain tracks02..11; a missing slot remains unavailable.

use std::path::{Path, PathBuf};

pub const FIRST_CD_TRACK: u8 = 2;
pub const LAST_CD_TRACK: u8 = 11;
pub const CD_AUDIO_TRACK_COUNT: usize = (LAST_CD_TRACK - FIRST_CD_TRACK + 1) as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum SoundtrackAvailability {
    Complete,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SoundtrackTrack {
    /// Original disc identity, including when the source is unavailable.
    pub cd_track: u8,
    /// Header-validated OGG or PCM WAVE source, with valid OGG preferred.
    pub path: Option<PathBuf>,
    /// Present candidates rejected because of missing or unsupported headers.
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SoundtrackInventory {
    /// Selected directory shared by setup status and runtime playback.
    pub directory: PathBuf,
    /// Expected directory absence, distinct from an empty or unreadable folder.
    pub directory_missing: bool,
    pub tracks: [SoundtrackTrack; CD_AUDIO_TRACK_COUNT],
    /// Directory/entry errors that prevented a complete inventory.
    pub scan_errors: Vec<String>,
}

impl SoundtrackInventory {
    // A missing optional soundtrack is a setup status, not a filesystem error.
    // Keep unexpected errors actionable, including access-denied directories.
    fn record_directory_error(&mut self, error: std::io::Error) {
        if error.kind() == std::io::ErrorKind::NotFound {
            self.directory_missing = true;
        } else {
            self.scan_errors.push(format!(
                "Cannot inspect music directory {}: {error}",
                self.directory.display()
            ));
        }
    }

    pub fn available_count(&self) -> usize {
        self.tracks
            .iter()
            .filter(|track| track.path.is_some())
            .count()
    }

    pub fn availability(&self) -> SoundtrackAvailability {
        match self.available_count() {
            0 => SoundtrackAvailability::Unavailable,
            CD_AUDIO_TRACK_COUNT => SoundtrackAvailability::Complete,
            _ => SoundtrackAvailability::Partial,
        }
    }

    pub fn track(&self, cd_track: u8) -> Option<&SoundtrackTrack> {
        cd_track
            .checked_sub(FIRST_CD_TRACK)
            .and_then(|index| self.tracks.get(index as usize))
    }

    pub fn missing_tracks(&self) -> Vec<u8> {
        self.tracks
            .iter()
            .filter(|track| track.path.is_none())
            .map(|track| track.cd_track)
            .collect()
    }
}

/// Inspect expected physical tracks without SDL, whole-track decoding, or PCM
/// allocation. Missing music does not invalidate the game-data installation.
pub fn inspect_soundtrack(directory: &Path) -> SoundtrackInventory {
    let mut inventory = SoundtrackInventory {
        directory: std::fs::canonicalize(directory).unwrap_or_else(|_| directory.to_path_buf()),
        directory_missing: false,
        tracks: std::array::from_fn(|index| SoundtrackTrack {
            cd_track: FIRST_CD_TRACK + index as u8,
            path: None,
            errors: Vec::new(),
        }),
        scan_errors: Vec::new(),
    };
    let mut candidates: [Vec<PathBuf>; CD_AUDIO_TRACK_COUNT] = std::array::from_fn(|_| Vec::new());
    let entries = match std::fs::read_dir(&inventory.directory) {
        Ok(entries) => entries,
        Err(error) => {
            inventory.record_directory_error(error);
            return inventory;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                inventory
                    .scan_errors
                    .push(format!("Cannot inspect music file: {error}"));
                continue;
            }
        };
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        let Some(index) = physical_track_slot(&name) else {
            continue;
        };
        let path = entry.path();
        candidates[index].push(path);
    }
    for (track, mut paths) in inventory.tracks.iter_mut().zip(candidates) {
        paths.sort_by_key(|path| {
            let is_ogg = path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("ogg"));
            (!is_ogg, path.clone())
        });
        for path in paths {
            match validate_cd_audio_track(&path) {
                Ok(()) if track.path.is_none() => track.path = Some(path),
                Ok(()) => {}
                Err(error) => track.errors.push(error),
            }
        }
    }
    inventory
}

/// Shared launcher/runtime location policy: a playable root soundtrack wins,
/// including a partial one. Empty or corrupt roots fall through to the data
/// root's parent cdaudio folder, then the historical root/music folder. Tracks from
/// different folders are never combined because that would hide missing slots.
pub fn locate_soundtrack(data_root: &Path) -> SoundtrackInventory {
    let directory = |parent: &Path, name: &str| {
        let expected = parent.join(name);
        if expected.is_dir() {
            return expected;
        }
        parent
            .read_dir()
            .ok()
            .and_then(|entries| {
                entries.filter_map(Result::ok).find_map(|entry| {
                    (entry
                        .file_name()
                        .to_string_lossy()
                        .eq_ignore_ascii_case(name)
                        && entry.path().is_dir())
                    .then(|| entry.path())
                })
            })
            .unwrap_or(expected)
    };
    let root_inventory = inspect_soundtrack(&directory(data_root, "cdaudio"));
    if root_inventory.available_count() > 0 {
        return root_inventory;
    }
    for candidate in [
        directory(&data_root.join(".."), "cdaudio"),
        directory(data_root, "music"),
    ] {
        let inventory = inspect_soundtrack(&candidate);
        if inventory.available_count() > 0 {
            return inventory;
        }
    }
    root_inventory
}

/// Validate OGG identification/comment/setup headers or the full RIFF chunk
/// structure and PCM format. Decoding the stream body remains the worker's job.
pub fn validate_cd_audio_track(path: &Path) -> Result<(), String> {
    super::open_decoder(path).map(|_| ())
}

fn physical_track_slot(name: &str) -> Option<usize> {
    let stem = name
        .strip_suffix(".ogg")
        .or_else(|| name.strip_suffix(".wav"))?;
    let digits = stem.strip_prefix("track")?;
    if digits.len() != 2 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let cd_track: u8 = digits.parse().ok()?;
    if !(FIRST_CD_TRACK..=LAST_CD_TRACK).contains(&cd_track) {
        return None;
    }
    Some((cd_track - FIRST_CD_TRACK) as usize)
}

#[cfg(test)]
mod tests {
    use super::super::wav::tests::pcm_wav;
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn physical_identity_requires_exact_two_digit_track_name() {
        assert_eq!(physical_track_slot("track02.ogg"), Some(0));
        assert_eq!(physical_track_slot("track11.wav"), Some(9));
        for name in [
            "track+2.wav",
            "track2.wav",
            "track002.wav",
            "track01.wav",
            "track12.ogg",
            "track02.ogg.bak",
            "track02.wav.part",
        ] {
            assert_eq!(physical_track_slot(name), None, "{name}");
        }
    }

    fn track_in(directory: &Path, cd_track: u8) {
        std::fs::create_dir_all(directory).unwrap();
        std::fs::write(
            directory.join(format!("track{cd_track:02}.wav")),
            pcm_wav(&[321, -654]),
        )
        .unwrap();
    }

    #[test]
    fn partial_root_soundtrack_precedes_complete_parent_without_merging() {
        let fixture = TestDirectory::new();
        let data_root = fixture.0.join("port");
        let root_audio = data_root.join("cdaudio");
        let parent_audio = fixture.0.join("cdaudio");
        for cd_track in FIRST_CD_TRACK..=LAST_CD_TRACK {
            track_in(&parent_audio, cd_track);
        }
        track_in(&root_audio, 4);
        let inventory = locate_soundtrack(&data_root);
        assert_eq!(
            inventory.directory,
            std::fs::canonicalize(root_audio).unwrap()
        );
        assert_eq!(inventory.available_count(), 1);
        assert!(inventory.track(4).unwrap().path.is_some());
        assert!(inventory.track(2).unwrap().path.is_none());
    }

    #[test]
    fn empty_root_does_not_shadow_valid_case_insensitive_parent_soundtrack() {
        let fixture = TestDirectory::new();
        let data_root = fixture.0.join("port");
        std::fs::create_dir_all(data_root.join("cdaudio")).unwrap();
        let parent_audio = fixture.0.join("CDAUDIO");
        track_in(&parent_audio, 3);
        let inventory = locate_soundtrack(&data_root);
        assert_eq!(
            inventory.directory,
            std::fs::canonicalize(parent_audio).unwrap()
        );
        assert_eq!(inventory.available_count(), 1);
        assert!(inventory.track(3).unwrap().path.is_some());
    }

    #[test]
    fn corrupt_root_and_empty_parent_fall_back_to_legacy_soundtrack() {
        let fixture = TestDirectory::new();
        let data_root = fixture.0.join("port");
        let root_audio = data_root.join("cdaudio");
        std::fs::create_dir_all(&root_audio).unwrap();
        std::fs::write(root_audio.join("track02.wav"), b"RIFF").unwrap();
        std::fs::create_dir_all(fixture.0.join("cdaudio")).unwrap();
        let legacy_audio = data_root.join("MUSIC");
        track_in(&legacy_audio, 11);
        let inventory = locate_soundtrack(&data_root);
        assert_eq!(
            inventory.directory,
            std::fs::canonicalize(legacy_audio).unwrap()
        );
        assert_eq!(inventory.available_count(), 1);
        assert!(inventory.track(11).unwrap().path.is_some());
    }

    #[test]
    fn no_playable_location_returns_root_inventory_and_its_corruption_details() {
        let fixture = TestDirectory::new();
        let data_root = fixture.0.join("port");
        let root_audio = data_root.join("cdaudio");
        let parent_audio = fixture.0.join("cdaudio");
        for directory in [&root_audio, &parent_audio] {
            std::fs::create_dir_all(directory).unwrap();
        }
        std::fs::write(root_audio.join("track02.wav"), b"RIFF").unwrap();
        std::fs::write(parent_audio.join("track03.ogg"), b"OggS").unwrap();
        let inventory = locate_soundtrack(&data_root);
        assert_eq!(
            inventory.directory,
            std::fs::canonicalize(root_audio).unwrap()
        );
        assert_eq!(
            inventory.availability(),
            SoundtrackAvailability::Unavailable
        );
        assert_eq!(inventory.track(2).unwrap().errors.len(), 1);
        assert!(inventory.track(3).unwrap().errors.is_empty());
    }

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let directory = std::env::temp_dir().join(format!(
                "v2k-music-inventory-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir_all(&directory).unwrap();
            Self(directory)
        }

        fn wav(&self, name: &str) {
            std::fs::write(self.0.join(name), pcm_wav(&[321, -654])).unwrap();
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn partial_soundtrack_retains_missing_physical_track_slots() {
        let directory = TestDirectory::new();
        directory.wav("track02.wav");
        directory.wav("track04.wav");
        directory.wav("track11.wav");
        directory.wav("track01.wav");
        directory.wav("track12.wav");
        directory.wav("track3.wav");
        let inventory = inspect_soundtrack(&directory.0);
        assert_eq!(inventory.availability(), SoundtrackAvailability::Partial);
        assert_eq!(inventory.available_count(), 3);
        assert_eq!(inventory.tracks[0].cd_track, 2);
        assert!(inventory.tracks[0].path.is_some());
        assert_eq!(inventory.tracks[1].cd_track, 3);
        assert!(inventory.tracks[1].path.is_none());
        assert_eq!(inventory.tracks[2].cd_track, 4);
        assert!(inventory.tracks[2].path.is_some());
        assert!(inventory.track(11).unwrap().path.is_some());
        assert!(inventory.track(1).is_none());
        assert!(inventory.track(12).is_none());
        assert_eq!(inventory.missing_tracks(), [3, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn complete_unavailable_and_corrupt_sources_have_distinct_inventory() {
        let directory = TestDirectory::new();
        let empty = inspect_soundtrack(&directory.0);
        assert_eq!(empty.availability(), SoundtrackAvailability::Unavailable);
        for cd_track in FIRST_CD_TRACK..=LAST_CD_TRACK {
            directory.wav(&format!("TRACK{cd_track:02}.WAV"));
        }
        let complete = inspect_soundtrack(&directory.0);
        assert_eq!(complete.availability(), SoundtrackAvailability::Complete);
        assert!(complete.missing_tracks().is_empty());
        std::fs::write(directory.0.join("TRACK03.WAV"), b"RIFF").unwrap();
        let corrupt = inspect_soundtrack(&directory.0);
        assert_eq!(corrupt.availability(), SoundtrackAvailability::Partial);
        assert_eq!(corrupt.missing_tracks(), [3]);
        assert_eq!(corrupt.track(3).unwrap().errors.len(), 1);
    }

    #[test]
    fn invalid_ogg_falls_back_to_valid_same_identity_wav() {
        let directory = TestDirectory::new();
        std::fs::write(directory.0.join("track05.ogg"), b"OggS broken").unwrap();
        directory.wav("track05.wav");
        let inventory = inspect_soundtrack(&directory.0);
        let track = inventory.track(5).unwrap();
        assert_eq!(
            track.path.as_ref().unwrap().file_name().unwrap(),
            "track05.wav"
        );
        assert_eq!(track.errors.len(), 1);
    }

    #[test]
    fn nonexistent_directory_is_expected_music_absence_without_os_noise() {
        let directory = TestDirectory::new();
        let inventory = inspect_soundtrack(&directory.0.join("missing"));
        assert_eq!(
            inventory.availability(),
            SoundtrackAvailability::Unavailable
        );
        assert!(inventory.directory_missing);
        assert!(inventory.scan_errors.is_empty());
        assert_eq!(
            inventory.missing_tracks(),
            (FIRST_CD_TRACK..=LAST_CD_TRACK).collect::<Vec<_>>()
        );
        assert!(inventory.tracks.iter().all(|track| track.errors.is_empty()));
    }

    #[test]
    fn empty_and_invalid_soundtracks_are_present_directories() {
        let directory = TestDirectory::new();
        let empty = inspect_soundtrack(&directory.0);
        assert!(!empty.directory_missing);
        assert!(empty.scan_errors.is_empty());
        assert_eq!(empty.availability(), SoundtrackAvailability::Unavailable);
        std::fs::write(directory.0.join("track02.wav"), b"RIFF").unwrap();
        let invalid = inspect_soundtrack(&directory.0);
        assert!(!invalid.directory_missing);
        assert!(invalid.scan_errors.is_empty());
        assert_eq!(invalid.track(2).unwrap().errors.len(), 1);
    }

    #[test]
    fn unreadable_directory_keeps_actionable_error_and_is_not_missing() {
        let directory = TestDirectory::new();
        let mut inventory = inspect_soundtrack(&directory.0);
        // Exercise the production classifier without changing host ACLs; a
        // permission fixture would otherwise depend on the test user's rights.
        inventory.record_directory_error(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "soundtrack directory access denied",
        ));
        assert!(!inventory.directory_missing);
        assert_eq!(inventory.scan_errors.len(), 1);
        assert!(inventory.scan_errors[0].contains(&directory.0.display().to_string()));
        assert!(inventory.scan_errors[0].contains("soundtrack directory access denied"));
    }

    #[test]
    fn non_directory_music_path_retains_unexpected_scan_error() {
        let directory = TestDirectory::new();
        let file = directory.0.join("cdaudio");
        std::fs::write(&file, b"not a directory").unwrap();
        let inventory = inspect_soundtrack(&file);
        assert!(!inventory.directory_missing);
        assert_eq!(
            inventory.availability(),
            SoundtrackAvailability::Unavailable
        );
        assert_eq!(inventory.scan_errors.len(), 1);
        assert!(inventory.scan_errors[0].contains(&file.display().to_string()));
    }
}
