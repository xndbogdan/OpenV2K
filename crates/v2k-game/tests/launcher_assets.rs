use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};
use v2k_formats::ovl::OvlFile;
use v2k_game::setup::{
    discover_installation, discover_setup_hint, expected_overlay_names, install_launcher,
    validate_installation, DiscImage, DiscoveryRequest, InstallRequest, MANIFEST_FILENAME,
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("launcher-tests")
            .join(format!(
                "{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::SeqCst)
            ));
        fs::create_dir_all(&root).unwrap();
        Self {
            root: fs::canonicalize(root).unwrap(),
        }
    }
    fn image(&self) -> PathBuf {
        let path = self.root.join("V2000.iso");
        fs::write(&path, synthetic_iso()).unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert!(self.root.starts_with(
            fs::canonicalize(PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("launcher-tests"))
                .unwrap()
        ));
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn empty_overlay() -> Vec<u8> {
    (0..15)
        .flat_map(|_| [b'a', b'b', b'c', b'd', 0, 0, 0, 0])
        .collect()
}
fn preload() -> Vec<u8> {
    let mut bytes = vec![0; 3180];
    for _ in 0..7 {
        bytes.extend_from_slice(&empty_overlay());
    }
    bytes
}
fn both32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    bytes[offset + 4..offset + 8].copy_from_slice(&value.to_be_bytes());
}
fn record(name: &[u8], extent: u32, length: u32, directory: bool) -> Vec<u8> {
    let count = 33 + name.len() + usize::from(name.len() % 2 == 0);
    let mut record = vec![0; count];
    record[0] = count as u8;
    both32(&mut record, 2, extent);
    both32(&mut record, 10, length);
    record[25] = if directory { 2 } else { 0 };
    record[28] = 1;
    record[31] = 1;
    record[32] = name.len() as u8;
    record[33..33 + name.len()].copy_from_slice(name);
    record
}
fn directory(bytes: &mut [u8], start: usize, records: Vec<Vec<u8>>) {
    let mut position = start;
    for record in records {
        if position % 2048 + record.len() > 2048 {
            position = (position / 2048 + 1) * 2048;
        }
        bytes[position..position + record.len()].copy_from_slice(&record);
        position += record.len();
    }
}
fn synthetic_iso() -> Vec<u8> {
    let volume = 242u32;
    let mut bytes = vec![0; volume as usize * 2048];
    let pvd = &mut bytes[16 * 2048..17 * 2048];
    pvd[0] = 1;
    pvd[1..6].copy_from_slice(b"CD001");
    pvd[6] = 1;
    both32(pvd, 80, volume);
    pvd[128..130].copy_from_slice(&2048u16.to_le_bytes());
    pvd[130..132].copy_from_slice(&2048u16.to_be_bytes());
    pvd[156..190].copy_from_slice(&record(&[0], 18, 2048, true));
    let terminator = &mut bytes[17 * 2048..18 * 2048];
    terminator[0] = 255;
    terminator[1..6].copy_from_slice(b"CD001");
    terminator[6] = 1;
    directory(
        &mut bytes,
        18 * 2048,
        vec![
            record(&[0], 18, 2048, true),
            record(&[1], 18, 2048, true),
            record(b"V2000", 19, 2048, true),
        ],
    );
    directory(
        &mut bytes,
        19 * 2048,
        vec![
            record(&[0], 19, 2048, true),
            record(&[1], 18, 2048, true),
            record(b"OVERLAY", 20, 6 * 2048, true),
            record(b"PRELOAD.DAT;1", 26, preload().len() as u32, false),
            record(b"BANNERHI.AVI;1", 28, 12, false),
            record(b"BANNERLO.AVI;1", 28, 12, false),
        ],
    );
    let mut records = vec![
        record(&[0], 20, 6 * 2048, true),
        record(&[1], 19, 2048, true),
    ];
    for (index, name) in expected_overlay_names().enumerate() {
        let extent = 29 + index as u32;
        records.push(record(format!("{name};1").as_bytes(), extent, 120, false));
        bytes[extent as usize * 2048..extent as usize * 2048 + 120]
            .copy_from_slice(&empty_overlay());
    }
    directory(&mut bytes, 20 * 2048, records);
    let preload = preload();
    bytes[26 * 2048..26 * 2048 + preload.len()].copy_from_slice(&preload);
    bytes[28 * 2048..28 * 2048 + 12].copy_from_slice(b"RIFF\0\0\0\0AVI ");
    bytes
}
fn request(destination: &Path) -> InstallRequest {
    InstallRequest {
        destination: destination.into(),
        executable: None,
        runtime_dependencies: Vec::new(),
        extract_music: false,
    }
}

#[test]
fn installs_complete_iso_corpus_and_verifies_manifest() {
    let fixture = Fixture::new();
    let image = DiscImage::open(&fixture.image()).unwrap();
    assert_eq!(image.summary().required_files, 215);
    assert!(image.summary().audio_tracks.is_empty());
    let destination = fixture.root.join("installed");
    let result = image.install(&request(&destination), |_| {}).unwrap();
    assert!(result.report.is_ready(), "{:?}", result.report.issues);
    assert!(result.report.manifest_verified);
    assert_eq!(result.report.checked_overlays, 212);
    assert_eq!(result.report.music.available_count(), 0);
    let exe = destination.join("v2k-game.exe");
    assert!(discover_installation(&DiscoveryRequest {
        executable: exe,
        ..Default::default()
    })
    .unwrap()
    .is_ready());
    fs::write(
        destination.join("Overlay/1X17XX.OVL"),
        empty_overlay().split_at(119).0,
    )
    .unwrap();
    let bad = validate_installation(&destination);
    assert!(!bad.is_ready());
    assert!(bad
        .issues
        .iter()
        .any(|issue| issue.path.ends_with("1X17XX.OVL")));
    let repaired = image.install(&request(&destination), |_| {}).unwrap();
    assert!(repaired.report.is_ready());
    assert!(repaired
        .backup_directory
        .unwrap()
        .join("Overlay/1X17XX.OVL")
        .exists());
}

#[test]
fn rejects_missing_embedded_overlay_and_incomplete_dependency_list() {
    let fixture = Fixture::new();
    let image = DiscImage::open(&fixture.image()).unwrap();
    let destination = fixture.root.join("installed");
    image.install(&request(&destination), |_| {}).unwrap();
    fs::remove_file(destination.join(MANIFEST_FILENAME)).unwrap();
    let path = destination.join("PRELOAD.DAT");
    let mut bytes = preload();
    bytes.truncate(bytes.len() - 120);
    fs::write(&path, bytes).unwrap();
    assert!(!validate_installation(&destination).is_ready());
    let mut bytes = preload();
    bytes[(14 * 53 + 17) * 4..(14 * 53 + 17) * 4 + 4].copy_from_slice(&1u32.to_le_bytes());
    fs::write(&path, bytes).unwrap();
    let mut ovl = empty_overlay();
    ovl.truncate(14 * 8);
    ovl.extend_from_slice(b"abcd");
    ovl.extend_from_slice(&28u32.to_le_bytes());
    let mut linkage = vec![0; 28];
    linkage[0] = 1;
    linkage[4] = 1;
    linkage[8] = 2;
    linkage[12] = 4;
    linkage[24] = 28;
    ovl.extend(linkage);
    fs::write(destination.join("Overlay/1X17XX.OVL"), ovl).unwrap();
    let report = validate_installation(&destination);
    assert!(!report.is_ready());
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.message.contains("dependency list is incomplete")));
}

#[test]
fn rejects_iso_extent_escape_unsafe_names_and_directory_cycle() {
    let fixture = Fixture::new();
    let first_overlay = 20 * 2048 + 68;
    let mut bytes = synthetic_iso();
    both32(&mut bytes, first_overlay + 2, 99999);
    let path = fixture.root.join("bounds.iso");
    fs::write(&path, bytes).unwrap();
    assert!(DiscImage::open(&path).unwrap_err().contains("exceeds"));
    let mut bytes = synthetic_iso();
    bytes[first_overlay + 33] = b'/';
    fs::write(&path, bytes).unwrap();
    assert!(DiscImage::open(&path)
        .unwrap_err()
        .contains("Unsafe ISO filename"));
    let mut bytes = synthetic_iso();
    both32(&mut bytes, 18 * 2048 + 68 + 2, 18);
    fs::write(&path, bytes).unwrap();
    assert!(DiscImage::open(&path).unwrap_err().contains("cycle"));
}

#[test]
fn cue_resolves_bin_preserves_track_numbers_and_wav_source_bytes() {
    let fixture = Fixture::new();
    let iso = synthetic_iso();
    let data_sectors = iso.len() / 2048;
    let mut raw = vec![0; (data_sectors + 4) * 2352];
    for (index, sector) in iso.chunks_exact(2048).enumerate() {
        raw[index * 2352 + 16..index * 2352 + 16 + 2048].copy_from_slice(sector);
    }
    for (index, byte) in raw[data_sectors * 2352..].iter_mut().enumerate() {
        *byte = (index % 251) as u8;
    }
    let bin = fixture.root.join("V2000.bin");
    fs::write(&bin, &raw).unwrap();
    let stamp = |sector: usize| {
        format!(
            "{:02}:{:02}:{:02}",
            sector / 4500,
            (sector / 75) % 60,
            sector % 75
        )
    };
    let cue = fixture.root.join("V2000.cue");
    fs::write(&cue,format!("FILE \"V2000.bin\" BINARY\n TRACK 01 MODE1/2352\n INDEX 01 00:00:00\n TRACK 02 AUDIO\n INDEX 01 {}\n TRACK 04 AUDIO\n INDEX 01 {}\n",stamp(data_sectors),stamp(data_sectors+2))).unwrap();
    let image = DiscImage::open(&bin).unwrap();
    assert_eq!(image.summary().audio_tracks, vec![2, 4]);
    let destination = fixture.root.join("installed");
    let mut install = request(&destination);
    install.extract_music = true;
    let result = image.install(&install, |_| {}).unwrap();
    assert!(result.report.is_ready());
    assert_eq!(result.report.music.available_count(), 2);
    let wav = fs::read(destination.join("cdaudio/track02.wav")).unwrap();
    assert_eq!(&wav[..4], b"RIFF");
    assert_eq!(
        &wav[44..],
        &raw[data_sectors * 2352..(data_sectors + 2) * 2352]
    );
    assert!(!destination.join("cdaudio/track03.wav").exists());
    fs::remove_file(destination.join("cdaudio/track02.wav")).unwrap();
    assert!(
        validate_installation(&destination).is_ready(),
        "optional missing music must not block Play"
    );
    fs::write(
        &cue,
        "FILE \"../outside.bin\" BINARY\nTRACK 01 MODE1/2352\nINDEX 01 00:00:00\n",
    )
    .unwrap();
    assert!(DiscImage::open(&cue)
        .unwrap_err()
        .contains("inside the image directory"));
}

#[test]
fn executable_folder_precedes_hints_but_explicit_invalid_folder_is_authoritative() {
    let fixture = Fixture::new();
    let image = DiscImage::open(&fixture.image()).unwrap();
    let installed = fixture.root.join("installed");
    image.install(&request(&installed), |_| {}).unwrap();
    let bad = fixture.root.join("empty");
    fs::create_dir(&bad).unwrap();
    let mut request = DiscoveryRequest {
        executable: installed.join("v2k-game.exe"),
        remembered: Some(bad.clone()),
        ..Default::default()
    };
    assert_eq!(discover_installation(&request).unwrap().root, installed);
    request.explicit = Some(bad.clone());
    let report = discover_installation(&request).unwrap();
    assert_eq!(report.root, bad);
    assert!(!report.is_ready());
}

#[v2k_test_support::retail_test]
fn supplied_retail_installation_matches_bundled_checksums() {
    let root = v2k_test_support::retail_dir();
    let report = validate_installation(&root);
    assert!(report.is_ready(), "{:?}", report.issues);
    assert!(report.embedded_checksums_verified);
    assert_eq!(report.verified_game_checksums, 213);
}

fn raw_image(iso: &[u8], audio_sectors: usize) -> Vec<u8> {
    let mut raw = vec![0; (iso.len() / 2048 + audio_sectors) * 2352];
    for (index, sector) in iso.chunks_exact(2048).enumerate() {
        raw[index * 2352 + 16..index * 2352 + 16 + 2048].copy_from_slice(sector);
    }
    raw
}
fn stamp(sector: usize) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        sector / 4500,
        (sector / 75) % 60,
        sector % 75
    )
}
fn audio_image(fixture: &Fixture) -> DiscImage {
    let iso = synthetic_iso();
    let raw = raw_image(&iso, 2);
    let sectors = iso.len() / 2048;
    fs::write(fixture.root.join("V2000.bin"), raw).unwrap();
    let cue = fixture.root.join("V2000.cue");
    fs::write(&cue,format!("FILE \"V2000.bin\" BINARY\nTRACK 01 MODE1/2352\nINDEX 01 00:00:00\nTRACK 02 AUDIO\nINDEX 01 {}\n",stamp(sectors))).unwrap();
    DiscImage::open(&cue).unwrap()
}

#[test]
fn valid_data_installs_when_declared_audio_file_or_tail_is_missing() {
    let fixture = Fixture::new();
    let iso = fixture.image();
    let cue = fixture.root.join("missing.cue");
    fs::write(&cue,"FILE \"V2000.iso\" BINARY\nTRACK 01 MODE1/2048\nINDEX 01 00:00:00\nFILE \"missing-audio.bin\" BINARY\nTRACK 02 AUDIO\nINDEX 01 00:00:00\n").unwrap();
    let image = DiscImage::open(&cue).unwrap();
    assert!(image.summary().audio_tracks.is_empty());
    let mut install = request(&fixture.root.join("missing-audio-install"));
    install.extract_music = true;
    let result = image.install(&install, |_| {}).unwrap();
    assert!(result.report.is_ready());
    assert!(result
        .warnings
        .iter()
        .any(|message| message.contains("Music disabled")));
    let bytes = fs::read(iso).unwrap();
    let sectors = bytes.len() / 2048;
    let bin = fixture.root.join("truncated.bin");
    fs::write(&bin, raw_image(&bytes, 0)).unwrap();
    fs::write(&cue,format!("FILE \"truncated.bin\" BINARY\nTRACK 01 MODE1/2352\nINDEX 01 00:00:00\nTRACK 02 AUDIO\nINDEX 01 {}\n",stamp(sectors))).unwrap();
    let image = DiscImage::open(&cue).unwrap();
    assert!(image.summary().audio_tracks.is_empty());
    install.destination = fixture.root.join("truncated-audio-install");
    assert!(image.install(&install, |_| {}).unwrap().report.is_ready());
}

#[test]
fn rejects_partial_iso_extent_overlap_and_noncontiguous_cue_file_reuse() {
    let fixture = Fixture::new();
    let mut bytes = synthetic_iso();
    both32(&mut bytes, 20 * 2048 + 68 + 10, 3000);
    let iso = fixture.root.join("overlap.iso");
    fs::write(&iso, bytes).unwrap();
    assert!(DiscImage::open(&iso).unwrap_err().contains("overlap"));
    let iso = synthetic_iso();
    let sectors = iso.len() / 2048;
    fs::write(fixture.root.join("first.bin"), raw_image(&iso, 4)).unwrap();
    fs::write(fixture.root.join("second.bin"), vec![0; 2352 * 2]).unwrap();
    let cue = fixture.root.join("reuse.cue");
    fs::write(&cue,format!("FILE \"first.bin\" BINARY\nTRACK 01 MODE1/2352\nINDEX 01 00:00:00\nFILE \"second.bin\" BINARY\nTRACK 02 AUDIO\nINDEX 01 00:00:00\nFILE \"first.bin\" BINARY\nTRACK 03 AUDIO\nINDEX 01 {}\n",stamp(sectors+2))).unwrap();
    assert!(DiscImage::open(&cue)
        .unwrap_err()
        .contains("non-contiguously"));
}

#[test]
fn tampered_or_oversize_manifest_is_rejected_before_music_commit() {
    let fixture = Fixture::new();
    let image = audio_image(&fixture);
    let destination = fixture.root.join("installed");
    image.install(&request(&destination), |_| {}).unwrap();
    let manifest = destination.join(MANIFEST_FILENAME);
    let original = fs::read(&manifest).unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
    json["files"][0]["path"] = "../escape.bin".into();
    let tampered = serde_json::to_vec(&json).unwrap();
    fs::write(&manifest, &tampered).unwrap();
    assert!(image
        .extract_music(&destination, |_| {})
        .unwrap_err()
        .contains("Unsafe"));
    assert_eq!(fs::read(&manifest).unwrap(), tampered);
    assert!(!destination.join("cdaudio/track02.wav").exists());
    fs::write(&manifest, vec![b' '; 1024 * 1024 + 1]).unwrap();
    assert!(image
        .extract_music(&destination, |_| {})
        .unwrap_err()
        .contains("size limit"));
    assert!(!destination.join("cdaudio/track02.wav").exists());
}

#[test]
fn failed_final_validation_rolls_back_music_and_manifest() {
    let fixture = Fixture::new();
    let image = audio_image(&fixture);
    let destination = fixture.root.join("installed");
    image.install(&request(&destination), |_| {}).unwrap();
    let original_manifest = fs::read(destination.join(MANIFEST_FILENAME)).unwrap();
    let mut changed = false;
    let error = image
        .extract_music(&destination, |progress| {
            if !changed && progress.phase == "Verifying installation" {
                changed = true;
                fs::write(
                    destination.join("Overlay/1X17XX.OVL"),
                    b"damaged during commit",
                )
                .unwrap();
            }
        })
        .unwrap_err();
    assert!(error.contains("previous files restored"));
    assert!(!destination.join("cdaudio/track02.wav").exists());
    assert_eq!(
        fs::read(destination.join(MANIFEST_FILENAME)).unwrap(),
        original_manifest
    );
}

#[test]
fn damaged_executable_installation_is_selected_for_repair_before_healthy_hint() {
    let fixture = Fixture::new();
    let image = DiscImage::open(&fixture.image()).unwrap();
    let healthy = fixture.root.join("healthy");
    image.install(&request(&healthy), |_| {}).unwrap();
    let damaged = fixture.root.join("damaged");
    fs::create_dir(&damaged).unwrap();
    fs::write(damaged.join("PRELOAD.DAT"), b"broken").unwrap();
    let discovery = DiscoveryRequest {
        executable: damaged.join("v2k-game.exe"),
        remembered: Some(healthy),
        ..Default::default()
    };
    let selected = discover_installation(&discovery).unwrap();
    assert_eq!(selected.root, damaged);
    assert!(!selected.is_ready());
    fs::remove_file(damaged.join("PRELOAD.DAT")).unwrap();
    fs::create_dir(damaged.join("Overlay")).unwrap();
    assert_eq!(discover_installation(&discovery).unwrap().root, damaged);
    fs::remove_dir(damaged.join("Overlay")).unwrap();
    fs::write(
        damaged.join(MANIFEST_FILENAME),
        r#"{"version":1,"source":"fixture","files":[]}"#,
    )
    .unwrap();
    assert_eq!(discover_installation(&discovery).unwrap().root, damaged);
}

#[test]
fn audio_source_disappearing_after_open_does_not_block_valid_game_data_install() {
    let fixture = Fixture::new();
    fixture.image();
    let audio = fixture.root.join("audio.bin");
    fs::write(&audio, vec![0; 2352 * 2]).unwrap();
    let cue = fixture.root.join("separate.cue");
    fs::write(&cue,"FILE \"V2000.iso\" BINARY\nTRACK 01 MODE1/2048\nINDEX 01 00:00:00\nFILE \"audio.bin\" BINARY\nTRACK 02 AUDIO\nINDEX 01 00:00:00\n").unwrap();
    let image = DiscImage::open(&cue).unwrap();
    assert_eq!(image.summary().audio_tracks, vec![2]);
    fs::remove_file(audio).unwrap();
    let mut install = request(&fixture.root.join("installed"));
    install.extract_music = true;
    let result = image.install(&install, |_| {}).unwrap();
    assert!(result.report.is_ready());
    assert_eq!(result.report.music.available_count(), 0);
    assert!(result
        .warnings
        .iter()
        .any(|message| message.contains("Music disabled")));
    assert!(!install.destination.join("cdaudio/track02.wav").exists());
}

#[test]
fn updated_port_executable_preserves_data_readiness_and_music_only_repair() {
    let fixture = Fixture::new();
    let image = audio_image(&fixture);
    let source_exe = fixture.root.join("v2k-game.exe");
    fs::write(&source_exe, b"old port executable").unwrap();
    let destination = fixture.root.join("installed");
    let mut install = request(&destination);
    install.executable = Some(source_exe);
    assert!(image.install(&install, |_| {}).unwrap().report.is_ready());
    let installed_exe = destination.join("v2k-game.exe");
    fs::write(&installed_exe, b"updated port executable").unwrap();
    let report = validate_installation(&destination);
    assert!(report.is_ready());
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.path.ends_with("v2k-game.exe")
            && issue.severity == v2k_game::setup::IssueSeverity::Warning));
    assert!(image
        .extract_music(&destination, |_| {})
        .unwrap()
        .report
        .is_ready());
    assert_eq!(
        fs::read(&installed_exe).unwrap(),
        b"updated port executable"
    );
    fs::remove_file(&installed_exe).unwrap();
    assert!(validate_installation(&destination).is_ready());
}

#[test]
fn healthy_setup_hints_cannot_mask_an_empty_executable_directory() {
    let fixture = Fixture::new();
    let image = DiscImage::open(&fixture.image()).unwrap();
    let healthy = fixture.root.join("healthy");
    image.install(&request(&healthy), |_| {}).unwrap();
    let empty = fixture.root.join("release");
    fs::create_dir(&empty).unwrap();
    // Startup may collect the current working directory among these hints. It
    // has no authority to redirect the executable's local installation check.
    for (remembered, registry_candidates) in [
        (Some(healthy.clone()), Vec::new()),
        (None, vec![healthy.clone()]),
        (
            Some(healthy.clone()),
            vec![healthy.clone(), healthy.clone()],
        ),
    ] {
        let selected = discover_installation(&DiscoveryRequest {
            executable: empty.join("v2k-game.exe"),
            explicit: None,
            remembered,
            registry_candidates,
        })
        .unwrap();
        assert_eq!(selected.root, empty);
        assert!(!selected.is_ready());
        assert_eq!(selected.checked_overlays, 0);
    }
    let explicit = discover_installation(&DiscoveryRequest {
        executable: empty.join("v2k-game.exe"),
        explicit: Some(healthy.clone()),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(explicit.root, healthy);
    assert!(explicit.is_ready());
    let hints = DiscoveryRequest {
        executable: empty.join("v2k-game.exe"),
        remembered: Some(empty.clone()),
        registry_candidates: vec![empty.clone(), healthy.clone(), healthy.clone()],
        ..Default::default()
    };
    assert_eq!(
        discover_setup_hint(&hints),
        Some(fs::canonicalize(&healthy).unwrap())
    );
    assert_eq!(discover_installation(&hints).unwrap().root, empty);
    assert_eq!(
        discover_setup_hint(&DiscoveryRequest {
            executable: empty.join("v2k-game.exe"),
            remembered: Some(empty),
            ..Default::default()
        }),
        None
    );
}

#[test]
fn launcher_placement_preserves_retail_assets_settings_saves_and_manifest() {
    let fixture = Fixture::new();
    let image = DiscImage::open(&fixture.image()).unwrap();
    let installed = fixture.root.join("retail");
    image.install(&request(&installed), |_| {}).unwrap();
    let previous_manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(installed.join(MANIFEST_FILENAME)).unwrap()).unwrap();
    let source_directory = fixture.root.join("release");
    fs::create_dir(&source_directory).unwrap();
    let executable = source_directory.join("v2k-game.exe");
    let dependency = source_directory.join("SDL2.dll");
    fs::write(&executable, b"new portable launcher").unwrap();
    fs::write(&dependency, b"new SDL runtime").unwrap();
    fs::write(installed.join("v2k-game.exe"), b"old portable launcher").unwrap();
    fs::write(installed.join("Slot00"), b"user save bytes").unwrap();
    fs::write(installed.join("settings.json"), b"user settings bytes").unwrap();
    let preload_before = fs::read(installed.join("PRELOAD.DAT")).unwrap();
    let overlay_before = fs::read(installed.join("Overlay/1X17XX.OVL")).unwrap();
    let placed = install_launcher(&installed, &executable, &[dependency], |_| {}).unwrap();
    assert!(placed.report.is_ready());
    assert_eq!(placed.root, fs::canonicalize(&installed).unwrap());
    assert_eq!(placed.report.root, placed.root);
    assert_eq!(
        fs::read(installed.join("v2k-game.exe")).unwrap(),
        b"new portable launcher"
    );
    assert_eq!(
        fs::read(installed.join("SDL2.dll")).unwrap(),
        b"new SDL runtime"
    );
    assert_eq!(
        fs::read(placed.backup_directory.unwrap().join("v2k-game.exe")).unwrap(),
        b"old portable launcher"
    );
    assert_eq!(
        fs::read(installed.join("PRELOAD.DAT")).unwrap(),
        preload_before
    );
    assert_eq!(
        fs::read(installed.join("Overlay/1X17XX.OVL")).unwrap(),
        overlay_before
    );
    assert_eq!(
        fs::read(installed.join("Slot00")).unwrap(),
        b"user save bytes"
    );
    assert_eq!(
        fs::read(installed.join("settings.json")).unwrap(),
        b"user settings bytes"
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(installed.join(MANIFEST_FILENAME)).unwrap()).unwrap();
    assert_eq!(manifest["source"], previous_manifest["source"]);
    for entry in previous_manifest["files"].as_array().unwrap() {
        assert!(manifest["files"].as_array().unwrap().contains(entry));
    }
    let installed_exe = installed.join("v2k-game.exe");
    let installed_dependency = installed.join("SDL2.dll");
    let in_place =
        install_launcher(&installed, &installed_exe, &[installed_dependency], |_| {}).unwrap();
    assert_eq!(in_place.files_written, 0);
    assert!(in_place.backup_directory.is_none());
    // This synthetic corpus is not in the bundled retail reference. Removing
    // its source fingerprints makes it unverified, rather than a bare retail
    // installation whose known fingerprints are already compiled into the port.
    fs::remove_file(installed.join(MANIFEST_FILENAME)).unwrap();
    fs::remove_file(&installed_exe).unwrap();
    fs::remove_file(installed.join("SDL2.dll")).unwrap();
    assert!(!validate_installation(&installed).is_ready());
    let bare = install_launcher(
        &installed,
        &executable,
        &[source_directory.join("SDL2.dll")],
        |_| {},
    )
    .unwrap_err();
    assert!(bare.contains("needs repair"));
    assert!(!installed_exe.exists());
    assert_eq!(
        fs::read(installed.join("PRELOAD.DAT")).unwrap(),
        preload_before
    );
    for name in expected_overlay_names() {
        assert_eq!(
            fs::read(installed.join("Overlay").join(&name)).unwrap(),
            image.read_file(&format!("Overlay/{name}")).unwrap()
        );
    }
    assert_eq!(
        fs::read(installed.join("Slot00")).unwrap(),
        b"user save bytes"
    );
    assert_eq!(
        fs::read(installed.join("settings.json")).unwrap(),
        b"user settings bytes"
    );
    let invalid = fixture.root.join("invalid");
    fs::create_dir(&invalid).unwrap();
    assert!(install_launcher(&invalid, &executable, &[], |_| {})
        .unwrap_err()
        .contains("needs repair"));
    assert!(!invalid.join("v2k-game.exe").exists());
}

#[v2k_test_support::retail_test]
fn bundled_checksums_cover_bare_retail_and_enforce_recorded_custom_file_consistency() {
    let fixture = Fixture::new();
    let retail = v2k_test_support::retail_dir();
    let installed = fixture.root.join("bundled-retail");
    fs::create_dir_all(installed.join("Overlay")).unwrap();
    fs::copy(retail.join("PRELOAD.DAT"), installed.join("PRELOAD.DAT")).unwrap();
    for name in expected_overlay_names() {
        fs::copy(
            retail.join("Overlay").join(&name),
            installed.join("Overlay").join(name),
        )
        .unwrap();
    }
    let bare = validate_installation(&installed);
    assert!(
        bare.is_ready(),
        "known retail bytes allow Play without a sidecar: {:?}",
        bare.issues
    );
    assert!(bare.embedded_checksums_verified);
    assert_eq!(bare.verified_game_checksums, 213);
    assert!(!bare.manifest_verified);

    let executable = fixture.root.join("v2k-game.exe");
    fs::write(&executable, b"portable launcher fixture").unwrap();
    let placed = install_launcher(&installed, &executable, &[], |_| {}).unwrap();
    assert!(placed.report.is_ready());
    assert!(placed.report.embedded_checksums_verified);
    assert_eq!(placed.report.verified_game_checksums, 213);
    assert!(placed.report.manifest_verified);
    assert!(!placed
        .report
        .issues
        .iter()
        .any(|issue| issue.message.contains("Checksums are unavailable")));
    let manifest_path = installed.join(MANIFEST_FILENAME);
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    assert_eq!(
        manifest["files"].as_array().unwrap().len(),
        1,
        "launcher placement need not record known retail files"
    );

    // Mutate one palette payload byte in the disposable copy. Its fixed layout
    // and all structural/dependency boundaries remain valid; a checksum still
    // has to detect the changed bytes before Play can be admitted.
    let relative = "Overlay/1X2XX.OVL";
    let path = installed.join(relative);
    let original = fs::read(&path).unwrap();
    let parsed = OvlFile::parse(&original).unwrap();
    let palette = parsed.section(7).unwrap();
    assert!(!palette.data.is_empty());
    let mut changed = original.clone();
    changed[palette.marker_offset + 8] ^= 1;
    fs::write(&path, &changed).unwrap();
    let unrecognized = validate_installation(&installed);
    assert_eq!(
        unrecognized.checked_overlays, 212,
        "the byte mutation still passes structural checks"
    );
    assert!(
        !unrecognized.is_ready(),
        "unverified changed required bytes must block Play"
    );
    assert!(!unrecognized.embedded_checksums_verified);
    assert_eq!(unrecognized.verified_game_checksums, 212);
    assert!(unrecognized.issues.iter().any(|issue| issue.path == path
        && issue.severity == v2k_game::setup::IssueSeverity::Error
        && issue
            .message
            .contains("unrecognized release or changed required file")));

    manifest["files"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "path": relative,
            "bytes": original.len(),
            "sha256": format!("{:x}", Sha256::digest(&original)),
        }));
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let recorded_mismatch = validate_installation(&installed);
    assert!(
        !recorded_mismatch.is_ready(),
        "an installer-recorded mismatch must block Play"
    );
    assert!(recorded_mismatch.issues.iter().any(|issue| issue.severity
        == v2k_game::setup::IssueSeverity::Error
        && issue.message.contains("Checksum mismatch")
        && issue.message.contains("1X2XX.OVL")));

    // A custom disc's matching recorded source fingerprint is supported even
    // when the bytes are not one of the bundled retail reference's entries.
    manifest["files"][1]["sha256"] =
        serde_json::Value::String(format!("{:X}", Sha256::digest(&changed)));
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let custom = validate_installation(&installed);
    assert!(
        custom.is_ready(),
        "recorded custom source bytes allow Play: {:?}",
        custom.issues
    );
    assert!(!custom.embedded_checksums_verified);
    assert!(custom.manifest_verified);
    assert_eq!(custom.verified_game_checksums, 213);

    fs::remove_file(manifest_path).unwrap();
    assert!(
        !validate_installation(&installed).is_ready(),
        "a bare unlisted release needs source verification"
    );
    fs::write(path, original).unwrap();
    assert!(
        validate_installation(&installed).is_ready(),
        "restored bundled bytes require no sidecar"
    );
}
