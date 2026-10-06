use super::{expected_overlay_names, find_child, SetupResult, MANIFEST_FILENAME};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
};
use v2k_formats::{linkage, ovl::OvlFile, preload::PreloadDat};
use v2k_render::music::{locate_soundtrack, SoundtrackInventory};

const MAX_ASSET_BYTES: u64 = 128 * 1024 * 1024;
const PRELOAD_MATRIX: usize = 15 * 53 * 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum IssueSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssetIssue {
    pub path: PathBuf,
    pub message: String,
    pub severity: IssueSeverity,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    pub root: PathBuf,
    pub issues: Vec<AssetIssue>,
    pub music: SoundtrackInventory,
    pub checked_overlays: usize,
    pub manifest_verified: bool,
    /// All required bytes match the reference compiled into this executable.
    pub embedded_checksums_verified: bool,
    /// Required files verified by the embedded reference or installer hashes.
    pub verified_game_checksums: usize,
}

impl ValidationReport {
    pub fn is_ready(&self) -> bool {
        self.checked_overlays == 212
            && !self
                .issues
                .iter()
                .any(|i| i.severity == IssueSeverity::Error)
    }
    fn issue(&mut self, path: PathBuf, severity: IssueSeverity, message: impl Into<String>) {
        self.issues.push(AssetIssue {
            path,
            severity,
            message: message.into(),
        });
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct InstallManifest {
    pub version: u32,
    pub source: String,
    pub files: Vec<ManifestFile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ManifestFile {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

fn bundled_retail_fingerprints() -> &'static HashMap<String, ManifestFile> {
    static FINGERPRINTS: OnceLock<HashMap<String, ManifestFile>> = OnceLock::new();
    FINGERPRINTS.get_or_init(|| {
        let manifest: InstallManifest = serde_json::from_str(include_str!("retail-checksums.json"))
            .expect("compiled retail fingerprint manifest must be valid");
        manifest
            .files
            .into_iter()
            .map(|entry| (entry.path.to_ascii_lowercase(), entry))
            .collect()
    })
}

fn required_game_paths() -> HashSet<String> {
    std::iter::once("preload.dat".to_string())
        .chain(
            expected_overlay_names().map(|name| format!("overlay/{}", name.to_ascii_lowercase())),
        )
        .collect()
}

fn observe_reference_checksum(
    relative: &str,
    path: &Path,
    bytes: &[u8],
    matched: &mut HashSet<String>,
    unrecognized: &mut Vec<(String, PathBuf)>,
) {
    let relative = relative.to_ascii_lowercase();
    let recognized = bundled_retail_fingerprints()
        .get(&relative)
        .is_some_and(|entry| entry.bytes == bytes.len() as u64 && entry.sha256 == digest(bytes));
    if recognized {
        matched.insert(relative);
    } else {
        unrecognized.push((relative, path.to_path_buf()));
    }
}

pub(crate) fn read_manifest(root: &Path, path: &Path) -> SetupResult<InstallManifest> {
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > 1024 * 1024 {
        return Err("Installation checksum manifest exceeds its size limit".into());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let manifest: InstallManifest = serde_json::from_slice(&bytes)
        .map_err(|e| format!("Invalid installation checksum manifest: {e}"))?;
    if manifest.version != 1 || manifest.files.len() > 1024 {
        return Err("Unsupported installation checksum manifest version or entry count".into());
    }
    let mut seen = std::collections::HashSet::new();
    for entry in &manifest.files {
        if entry.path.is_empty()
            || !seen.insert(entry.path.to_ascii_lowercase())
            || entry.sha256.len() != 64
            || !entry.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("Invalid or duplicate installation manifest entry".into());
        }
        super::install::safe_target(&root, &entry.path)?;
    }
    Ok(manifest)
}

pub(crate) fn read_asset(path: &Path) -> SetupResult<Vec<u8>> {
    let metadata =
        fs::metadata(path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    if !metadata.is_file() || metadata.len() > MAX_ASSET_BYTES {
        return Err(format!(
            "{} is not a bounded game-data file",
            path.display()
        ));
    }
    fs::read(path).map_err(|e| format!("Cannot read {}: {e}", path.display()))
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Canonical parsers deliberately tolerate historical truncated evidence; the
/// launcher adds exact boundaries needed for a playable installation.
pub(crate) fn validate_overlay(
    bytes: &[u8],
    linkage_count: u32,
    global_level: Option<usize>,
) -> SetupResult<()> {
    if !bytes.starts_with(b"abcd") {
        return Err("OVL does not begin with its section marker".into());
    }
    let ovl = OvlFile::parse(bytes).map_err(|e| e.to_string())?;
    for index in [0, 1, 2, 14] {
        let section = &ovl.sections[index];
        if section.header_value as usize != section.data.len() {
            return Err(format!(
                "Section {index} declares {} bytes but contains {}",
                section.header_value,
                section.data.len()
            ));
        }
    }
    let section = &ovl.sections[14];
    if linkage_count == 0 {
        if !section.data.is_empty() {
            return Err("PRELOAD declares no Section-14 records but data is present".into());
        }
        return Ok(());
    }
    let count = linkage_count as usize;
    if count > section.data.len() / 28 {
        return Err("Section-14 descriptor array is truncated".into());
    }
    let table = linkage::parse_linkage(&section.data, count).map_err(|e| e.to_string())?;
    if table.records.len() != count {
        return Err("Section-14 record count disagrees with PRELOAD".into());
    }
    for record in &table.records {
        for pointer in [record.region1_ptr, record.region2_ptr, record.region3_ptr] {
            if pointer != 0
                && ((pointer as usize) < count * 28 || (pointer as usize) > table.data.len())
            {
                return Err(format!(
                    "Section-14 region pointer {pointer} is outside trailing data"
                ));
            }
        }
        if record.record_type == 1 {
            if record.entry_size != 4 || record.layer_count != 1 {
                return Err("Invalid sprite dependency descriptor".into());
            }
            let end = (record.region3_ptr as usize)
                .checked_add(
                    (record.entry_count as usize)
                        .checked_mul(4)
                        .ok_or("Sprite dependency size overflows")?,
                )
                .ok_or("Sprite dependency end overflows")?;
            if end > table.data.len() || table.sprite_dependencies().is_none() {
                return Err("Sprite dependency list is incomplete".into());
            }
        }
        if record.record_type == 2 {
            // Generic grid regions share the canonical radar decoder's shape:
            // two u32 dimensions, layer_count u32 defaults, then count*stride.
            for (pointer, length) in [
                (record.region1_ptr, 8usize),
                (
                    record.region2_ptr,
                    (record.layer_count as usize)
                        .checked_mul(4)
                        .ok_or("Grid default size overflow")?,
                ),
                (
                    record.region3_ptr,
                    (record.entry_count as usize)
                        .checked_mul(record.entry_size as usize)
                        .ok_or("Grid payload size overflow")?,
                ),
            ] {
                if pointer == 0
                    || (pointer as usize)
                        .checked_add(length)
                        .is_none_or(|end| end > table.data.len())
                {
                    return Err("Section-14 grid region is incomplete".into());
                }
            }
            let offset = record.region1_ptr as usize;
            let width =
                v2k_formats::ovl::read_u32(&table.data, offset).map_err(|e| e.to_string())?;
            let height =
                v2k_formats::ovl::read_u32(&table.data, offset + 4).map_err(|e| e.to_string())?;
            if width.checked_mul(height) != Some(record.entry_count) {
                return Err("Section-14 grid dimensions disagree with its entry count".into());
            }
        }
    }
    if global_level == Some(3) {
        table.radar_projection_tables().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(crate) fn validate_preload(bytes: &[u8]) -> SetupResult<PreloadDat> {
    let parsed = PreloadDat::parse(bytes).map_err(|e| e.to_string())?;
    let markers = bytes[PRELOAD_MATRIX..]
        .windows(4)
        .filter(|b| *b == b"abcd")
        .count();
    if parsed.embedded_ovls.len() != 7 || markers != 7 * 15 {
        return Err(format!("PRELOAD must contain exactly seven intact overlays (found {}, {markers} section markers)", parsed.embedded_ovls.len()));
    }
    if parsed.embedded_ovls[0].file_offset != PRELOAD_MATRIX {
        return Err("Unexpected data before PRELOAD's first overlay".into());
    }
    for embedded in &parsed.embedded_ovls {
        let span_end = parsed
            .embedded_ovls
            .get(embedded.index + 1)
            .map(|next| next.file_offset)
            .unwrap_or(bytes.len());
        let last = embedded
            .ovl
            .section(14)
            .ok_or("Embedded overlay has no final section")?;
        let end = embedded
            .file_offset
            .checked_add(last.marker_offset)
            .and_then(|n| n.checked_add(8))
            .and_then(|n| n.checked_add(last.header_value as usize))
            .ok_or("Embedded overlay boundary overflow")?;
        if end > span_end {
            return Err(format!(
                "Embedded overlay {} has a truncated final section",
                embedded.index
            ));
        }
        // 00493EE0 loads the null-terminated startup descriptor list and closes
        // PRELOAD without reading to EOF. Retail PRELOAD retains zero padding
        // after the final empty overlay. Padding is outside the declared final
        // section; it must not become fictitious Section-14 linkage data.
        if end != span_end
            && (embedded.index != 6 || bytes[end..span_end].iter().any(|&byte| byte != 0))
        {
            return Err("Unexpected data outside PRELOAD's seven overlay boundaries".into());
        }
        // Embedded order skips disk-only system level 3. These associations
        // are also retained in session.rs and FORMAT_DOCUMENTATION.md. The
        // final embedded empty descriptor has no external level association.
        let global_level = [0usize, 1, 2, 4, 5, 12].get(embedded.index).copied();
        let count = global_level
            .map(|level| parsed.count(14, level))
            .unwrap_or(0);
        validate_overlay(&bytes[embedded.file_offset..end], count, global_level)
            .map_err(|e| format!("Embedded overlay {}: {e}", embedded.index))?;
    }
    Ok(parsed)
}

pub fn validate_installation(root: &Path) -> ValidationReport {
    let music = locate_soundtrack(root);
    let music_dir = music.directory.clone();
    let mut report = ValidationReport {
        root: root.to_path_buf(),
        issues: Vec::new(),
        music,
        checked_overlays: 0,
        manifest_verified: false,
        embedded_checksums_verified: false,
        verified_game_checksums: 0,
    };
    let mut matched_checksums = HashSet::new();
    let mut unrecognized_checksums = Vec::new();
    let preload_path = find_child(root, "PRELOAD.DAT").unwrap_or_else(|| root.join("PRELOAD.DAT"));
    let preload = match read_asset(&preload_path).and_then(|bytes| {
        let preload = validate_preload(&bytes)?;
        observe_reference_checksum(
            "PRELOAD.DAT",
            &preload_path,
            &bytes,
            &mut matched_checksums,
            &mut unrecognized_checksums,
        );
        Ok(preload)
    }) {
        Ok(preload) => Some(preload),
        Err(error) => {
            report.issue(preload_path, IssueSeverity::Error, error);
            None
        }
    };
    let overlay_dir = find_child(root, "Overlay").unwrap_or_else(|| root.join("Overlay"));
    for name in expected_overlay_names() {
        let path = find_child(&overlay_dir, &name).unwrap_or_else(|| overlay_dir.join(&name));
        let level = name
            .split('X')
            .nth(1)
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap();
        let result = read_asset(&path).and_then(|bytes| {
            if let Some(preload) = &preload {
                validate_overlay(&bytes, preload.count(14, level), Some(level))?;
            } else {
                OvlFile::parse(&bytes)
                    .map(|_| ())
                    .map_err(|e| e.to_string())?;
            }
            observe_reference_checksum(
                &format!("Overlay/{name}"),
                &path,
                &bytes,
                &mut matched_checksums,
                &mut unrecognized_checksums,
            );
            Ok(())
        });
        match result {
            Ok(()) => report.checked_overlays += 1,
            Err(error) => report.issue(path, IssueSeverity::Error, error),
        }
    }
    for movie in ["BANNERHI.AVI", "BANNERLO.AVI"] {
        if find_child(root, movie).is_none() {
            report.issue(
                root.join(movie),
                IssueSeverity::Warning,
                "Intro video unavailable; game can still start",
            );
        }
    }
    if report.music.directory_missing {
        report.issue(
            music_dir,
            IssueSeverity::Warning,
            "Missing cdaudio music directory. See Options.",
        );
    } else if report.music.available_count() == 0 {
        report.issue(
            music_dir,
            IssueSeverity::Warning,
            "Music disabled: no valid CD audio tracks are installed",
        );
    } else if !report.music.missing_tracks().is_empty() {
        report.issue(
            music_dir,
            IssueSeverity::Warning,
            format!(
                "Soundtrack incomplete; missing CD tracks {:?}",
                report.music.missing_tracks()
            ),
        );
    }
    report.embedded_checksums_verified = matched_checksums.len() == required_game_paths().len();
    let mut manifest_failed = false;
    if let Some(path) = find_child(root, MANIFEST_FILENAME) {
        match verify_manifest(root, &path) {
            Ok(verification) => {
                matched_checksums.extend(verification.required_paths);
                report.manifest_verified = verification.warnings.is_empty()
                    && required_game_paths().is_subset(&matched_checksums);
                for (path, message) in verification.warnings {
                    report.issue(path, IssueSeverity::Warning, message);
                }
            }
            Err(error) => {
                manifest_failed = true;
                report.issue(path, IssueSeverity::Error, error);
            }
        }
    }
    report.verified_game_checksums = matched_checksums.len();
    // Differing bytes are not proof of corruption: another release or a
    // custom image may be valid. It must supply installer-recorded source
    // fingerprints before Play can claim the required files are verified.
    if !manifest_failed {
        for (relative, path) in unrecognized_checksums {
            if !matched_checksums.contains(&relative) {
                report.issue(path, IssueSeverity::Error, "Checksum not recognized by the bundled retail reference: unrecognized release or changed required file. Structural checks passed; install or verify from the original disc image before playing");
            }
        }
    }
    report
}

struct ManifestVerification {
    warnings: Vec<(PathBuf, String)>,
    required_paths: HashSet<String>,
}

fn verify_manifest(root: &Path, path: &Path) -> SetupResult<ManifestVerification> {
    let manifest = read_manifest(root, path)?;
    let mut seen = std::collections::HashSet::new();
    let canonical_root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let mut warnings = Vec::new();
    let required_names = required_game_paths();
    let mut required_paths = HashSet::new();
    for entry in manifest.files {
        let relative = Path::new(&entry.path);
        if relative
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
            || entry.path.contains([':', '\\'])
            || !seen.insert(entry.path.to_lowercase())
        {
            return Err("Unsafe or duplicate path in installation checksum manifest".into());
        }
        let target = root.join(relative);
        let result = (|| {
            let canonical = fs::canonicalize(&target)
                .map_err(|e| format!("Checksum verification: {}: {e}", target.display()))?;
            if !canonical.starts_with(&canonical_root) {
                return Err("Checksum target resolves outside the installation".into());
            }
            let mut file = fs::File::open(&canonical).map_err(|e| e.to_string())?;
            let mut hash = Sha256::new();
            let mut length = 0u64;
            let mut buffer = [0; 65536];
            loop {
                let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
                if count == 0 {
                    break;
                }
                length += count as u64;
                hash.update(&buffer[..count]);
            }
            if length != entry.bytes
                || !format!("{:x}", hash.finalize()).eq_ignore_ascii_case(&entry.sha256)
            {
                return Err(format!(
                    "Checksum mismatch: {}; repair from the original disc image",
                    target.display()
                ));
            }
            Ok(())
        })();
        if let Err(error) = result {
            if !required_names.contains(&entry.path.to_ascii_lowercase()) {
                let optional_media = entry.path.to_ascii_lowercase().starts_with("cdaudio/")
                    || entry.path.to_ascii_uppercase().ends_with(".AVI");
                let message = if optional_media {
                    error
                } else {
                    "Port runtime changed or unavailable since installation; game-data checks remain valid.".to_string()
                };
                warnings.push((target, message));
            } else {
                return Err(error);
            }
        } else if required_names.contains(&entry.path.to_ascii_lowercase()) {
            required_paths.insert(entry.path.to_ascii_lowercase());
        }
    }
    Ok(ManifestVerification {
        warnings,
        required_paths,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MusicFixture(PathBuf);

    impl MusicFixture {
        fn new() -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let parent = std::env::temp_dir().join(format!(
                "v2k-validation-music-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(parent.join("port")).unwrap();
            Self(parent)
        }

        fn root(&self) -> PathBuf {
            self.0.join("port")
        }
    }

    impl Drop for MusicFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn music_warning(report: &ValidationReport) -> &str {
        let issues: Vec<_> = report
            .issues
            .iter()
            .filter(|issue| issue.path == report.music.directory)
            .collect();
        assert_eq!(issues.len(), 1, "music status has one optional warning");
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
        &issues[0].message
    }

    #[test]
    fn music_validation_distinguishes_missing_empty_and_invalid_soundtrack() {
        let fixture = MusicFixture::new();
        let root = fixture.root();
        let absent = validate_installation(&root);
        assert!(absent.music.directory_missing);
        assert!(absent.music.scan_errors.is_empty());
        assert_eq!(
            music_warning(&absent),
            "Missing cdaudio music directory. See Options."
        );

        let audio = root.join("cdaudio");
        fs::create_dir_all(&audio).unwrap();
        let empty = validate_installation(&root);
        assert!(!empty.music.directory_missing);
        assert!(empty.music.scan_errors.is_empty());
        assert_eq!(
            music_warning(&empty),
            "Music disabled: no valid CD audio tracks are installed"
        );

        fs::write(audio.join("track02.wav"), b"RIFF").unwrap();
        let invalid = validate_installation(&root);
        assert!(!invalid.music.directory_missing);
        assert_eq!(invalid.music.track(2).unwrap().errors.len(), 1);
        assert_eq!(
            music_warning(&invalid),
            "Music disabled: no valid CD audio tracks are installed"
        );
    }

    #[test]
    fn embedded_retail_reference_covers_exactly_the_required_files() {
        let manifest: InstallManifest =
            serde_json::from_str(include_str!("retail-checksums.json")).unwrap();
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.files.len(), 213);
        assert!(!manifest.source.is_empty());
        let fingerprints = bundled_retail_fingerprints();
        assert_eq!(
            fingerprints.keys().cloned().collect::<HashSet<_>>(),
            required_game_paths()
        );
        for entry in fingerprints.values() {
            assert!(entry.bytes > 0 && entry.bytes <= MAX_ASSET_BYTES);
            assert_eq!(entry.sha256.len(), 64);
            assert!(entry.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()));
        }
    }
}
