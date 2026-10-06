use super::{
    validation::{digest, validate_overlay, validate_preload, InstallManifest, ManifestFile},
    DiscImage, SetupResult, ValidationReport, MANIFEST_FILENAME,
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone)]
pub struct InstallRequest {
    pub destination: PathBuf,
    pub executable: Option<PathBuf>,
    pub runtime_dependencies: Vec<PathBuf>,
    pub extract_music: bool,
}

#[derive(Debug, Clone)]
pub struct SetupProgress {
    pub phase: String,
    pub message: String,
    pub completed: usize,
    pub total: usize,
}

#[derive(Debug, Clone)]
pub struct InstallResult {
    pub root: PathBuf,
    pub report: ValidationReport,
    pub files_written: usize,
    pub backup_directory: Option<PathBuf>,
    pub warnings: Vec<String>,
}

struct Stage {
    path: PathBuf,
}
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn hash_file(path: &Path) -> SetupResult<(u64, String)> {
    let mut file = File::open(path).map_err(|e| format!("Cannot hash {}: {e}", path.display()))?;
    let mut hash = Sha256::new();
    let mut length = 0;
    let mut bytes = [0; 65536];
    loop {
        let count = file.read(&mut bytes).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(&bytes[..count]);
        length += count as u64;
    }
    Ok((length, format!("{:x}", hash.finalize())))
}

fn progress(
    callback: &mut impl FnMut(SetupProgress),
    phase: &str,
    message: impl Into<String>,
    completed: usize,
    total: usize,
) {
    callback(SetupProgress {
        phase: phase.into(),
        message: message.into(),
        completed,
        total,
    });
}

fn stage_at(destination: &Path) -> SetupResult<(PathBuf, Stage, String)> {
    fs::create_dir_all(destination).map_err(|e| {
        format!(
            "Cannot create installation directory {}: {e}",
            destination.display()
        )
    })?;
    let root = fs::canonicalize(destination).map_err(|e| e.to_string())?;
    let nonce = format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos()
    );
    let stage = Stage {
        path: root.join(format!(".v2000-stage-{nonce}")),
    };
    fs::create_dir(&stage.path).map_err(|e| e.to_string())?;
    Ok((root, stage, nonce))
}

fn write_stage(stage: &Stage, relative: &str, bytes: &[u8]) -> SetupResult<ManifestFile> {
    let path = stage.path.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, bytes).map_err(|e| format!("Cannot stage {relative}: {e}"))?;
    let source_hash = digest(bytes);
    let (length, written_hash) = hash_file(&path)?;
    if length != bytes.len() as u64 || written_hash != source_hash {
        return Err(format!(
            "Staged source/destination checksum mismatch: {relative}"
        ));
    }
    Ok(ManifestFile {
        path: relative.replace('\\', "/"),
        bytes: length,
        sha256: source_hash,
    })
}

/// Place this portable launcher into a validated existing retail installation.
/// Required data, settings and saves are never copied or rewritten by this path.
pub fn install_launcher(
    destination: &Path,
    executable: &Path,
    runtime_dependencies: &[PathBuf],
    mut callback: impl FnMut(SetupProgress),
) -> SetupResult<InstallResult> {
    let root = fs::canonicalize(destination)
        .map_err(|e| format!("Cannot select installation {}: {e}", destination.display()))?;
    let report = super::validate_installation(&root);
    if !report.is_ready() {
        return Err(format!(
            "Existing installation needs repair before placing the launcher: {}",
            report
                .issues
                .iter()
                .filter(|issue| issue.severity == super::IssueSeverity::Error)
                .map(|issue| issue.message.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    let (root, stage, nonce) = stage_at(&root)?;
    let mut entries = Vec::new();
    let mut names = std::collections::HashSet::new();
    let total = runtime_dependencies.len() + 1;
    for (index, source) in std::iter::once(executable)
        .chain(runtime_dependencies.iter().map(PathBuf::as_path))
        .enumerate()
    {
        let filename = source
            .file_name()
            .ok_or("Runtime file has no filename")?
            .to_string_lossy()
            .into_owned();
        if !names.insert(filename.to_ascii_lowercase()) {
            return Err("Duplicate filename in launcher runtime files".into());
        }
        let target = safe_target(&root, &filename)?;
        progress(
            &mut callback,
            "Installing launcher",
            &filename,
            index,
            total,
        );
        let source_path = fs::canonicalize(source)
            .map_err(|e| format!("Cannot read launcher runtime {}: {e}", source.display()))?;
        if fs::canonicalize(&target).is_ok_and(|target| target == source_path) {
            continue;
        }
        let bytes = super::validation::read_asset(&source_path)?;
        entries.push(write_stage(&stage, &filename, &bytes)?);
    }
    if entries.is_empty() {
        progress(
            &mut callback,
            "Complete",
            "Launcher is already in this installation",
            total,
            total,
        );
        return Ok(InstallResult {
            root,
            report,
            files_written: 0,
            backup_directory: None,
            warnings: Vec::new(),
        });
    }
    let manifest_path = root.join(MANIFEST_FILENAME);
    let mut manifest = if manifest_path.exists() {
        super::validation::read_manifest(&root, &manifest_path)?
    } else {
        InstallManifest {
            version: 1,
            source: format!("Existing installation: {}", root.display()),
            files: Vec::new(),
        }
    };
    manifest.files.retain(|entry| {
        !entries
            .iter()
            .any(|new| new.path.eq_ignore_ascii_case(&entry.path))
    });
    manifest.files.extend(entries);
    let (written, backup, report) = commit_stage(&root, &stage, &nonce, manifest, &mut callback)?;
    progress(
        &mut callback,
        "Complete",
        "Launcher installed; starting the installed copy",
        total,
        total,
    );
    Ok(InstallResult {
        root,
        report,
        files_written: written,
        backup_directory: backup,
        warnings: Vec::new(),
    })
}

impl DiscImage {
    pub fn install(
        &self,
        request: &InstallRequest,
        mut callback: impl FnMut(SetupProgress),
    ) -> SetupResult<InstallResult> {
        let (root, stage, nonce) = stage_at(&request.destination)?;
        let mut warnings = self.warnings.clone();
        let total = self.files.len()
            + if request.extract_music {
                self.audio.len()
            } else {
                0
            }
            + request.runtime_dependencies.len()
            + usize::from(request.executable.is_some());
        let mut completed = 0;
        let mut entries = Vec::new();
        progress(
            &mut callback,
            "Checking disc",
            "Validating PRELOAD and every overlay before replacing files",
            completed,
            total,
        );
        let preload_bytes = self.read_file("PRELOAD.DAT")?;
        let preload = validate_preload(&preload_bytes)?;
        entries.push(write_stage(&stage, "PRELOAD.DAT", &preload_bytes)?);
        completed += 1;
        for (relative, _) in self
            .files
            .iter()
            .filter(|(name, _)| name.as_str() != "PRELOAD.DAT")
        {
            progress(
                &mut callback,
                "Installing game data",
                relative.clone(),
                completed,
                total,
            );
            let bytes = self.read_file(relative)?;
            if relative.starts_with("Overlay/") {
                let level = relative
                    .split('X')
                    .nth(1)
                    .and_then(|s| s.parse::<usize>().ok())
                    .ok_or("Invalid allowlisted overlay name")?;
                validate_overlay(&bytes, preload.count(14, level), Some(level))
                    .map_err(|e| format!("Disc {relative}: {e}"))?;
            }
            entries.push(write_stage(&stage, relative, &bytes)?);
            completed += 1;
        }
        if request.extract_music {
            warnings.extend(self.stage_music(
                &stage,
                &mut entries,
                &mut completed,
                total,
                &mut callback,
            ));
        }
        for source in request
            .executable
            .iter()
            .chain(&request.runtime_dependencies)
        {
            let filename = source.file_name().ok_or("Runtime file has no filename")?;
            let destination = root.join(filename);
            if matches!((fs::canonicalize(source), fs::canonicalize(&destination)), (Ok(source), Ok(destination)) if source == destination)
            {
                // A running executable already in the destination is retained.
                completed += 1;
                continue;
            }
            progress(
                &mut callback,
                "Installing executable",
                filename.to_string_lossy(),
                completed,
                total,
            );
            let bytes = fs::read(source)
                .map_err(|e| format!("Cannot copy runtime {}: {e}", source.display()))?;
            entries.push(write_stage(&stage, &filename.to_string_lossy(), &bytes)?);
            completed += 1;
        }
        let manifest = InstallManifest {
            version: 1,
            source: self.source.to_string_lossy().into_owned(),
            files: entries,
        };
        let (written, backup, report) =
            commit_stage(&root, &stage, &nonce, manifest, &mut callback)?;
        progress(
            &mut callback,
            "Complete",
            "Installation verified",
            total,
            total,
        );
        Ok(InstallResult {
            root,
            report,
            files_written: written,
            backup_directory: backup,
            warnings,
        })
    }

    pub fn extract_music(
        &self,
        destination: &Path,
        mut callback: impl FnMut(SetupProgress),
    ) -> SetupResult<InstallResult> {
        let (root, stage, nonce) = stage_at(destination)?;
        let mut entries = Vec::new();
        let mut completed = 0;
        let mut warnings = self.warnings.clone();
        if self.audio.is_empty() {
            warnings
                .push("Music disabled: no usable audio tracks are available in this image".into());
            return Ok(InstallResult {
                report: super::validate_installation(&root),
                root,
                files_written: 0,
                backup_directory: None,
                warnings,
            });
        }
        warnings.extend(self.stage_music(
            &stage,
            &mut entries,
            &mut completed,
            self.audio.len(),
            &mut callback,
        ));
        if entries.is_empty() {
            return Ok(InstallResult {
                report: super::validate_installation(&root),
                root,
                files_written: 0,
                backup_directory: None,
                warnings,
            });
        }
        let manifest_path = root.join(MANIFEST_FILENAME);
        let mut manifest = if manifest_path.exists() {
            super::validation::read_manifest(&root, &manifest_path)?
        } else {
            InstallManifest {
                version: 1,
                source: self.source.to_string_lossy().into_owned(),
                files: Vec::new(),
            }
        };
        manifest.files.retain(|entry| {
            !entries
                .iter()
                .any(|new| new.path.eq_ignore_ascii_case(&entry.path))
        });
        manifest.files.extend(entries);
        manifest.source = self.source.to_string_lossy().into_owned();
        let (written, backup, report) =
            commit_stage(&root, &stage, &nonce, manifest, &mut callback)?;
        progress(
            &mut callback,
            "Complete",
            "CD audio extracted and verified",
            completed,
            completed,
        );
        Ok(InstallResult {
            root,
            report,
            files_written: written,
            backup_directory: backup,
            warnings,
        })
    }

    fn stage_music(
        &self,
        stage: &Stage,
        entries: &mut Vec<ManifestFile>,
        completed: &mut usize,
        total: usize,
        callback: &mut impl FnMut(SetupProgress),
    ) -> Vec<String> {
        let mut warnings = Vec::new();
        let initial_entries = entries.len();
        if let Err(error) = fs::create_dir_all(stage.path.join("cdaudio")) {
            return vec![format!(
                "Music disabled: cannot prepare audio directory: {error}"
            )];
        }
        for track in &self.audio {
            let relative = format!("cdaudio/track{:02}.wav", track.number);
            let path = stage.path.join(&relative);
            progress(
                callback,
                "Extracting CD audio",
                format!("CD track {:02}", track.number),
                *completed,
                total,
            );
            let outcome: SetupResult<ManifestFile> = (|| {
                let data_length = (track.end_sector - track.start_sector)
                    .checked_mul(2352)
                    .ok_or("CD audio size overflow")?;
                if data_length > u32::MAX as u64 - 36 {
                    return Err("CD track exceeds WAV's 32-bit length limit".into());
                }
                let mut source = File::open(&track.file).map_err(|e| e.to_string())?;
                source
                    .seek(SeekFrom::Start(track.start_sector * 2352))
                    .map_err(|e| e.to_string())?;
                let mut destination = File::create(&path).map_err(|e| e.to_string())?;
                let mut header = Vec::with_capacity(44);
                header.extend_from_slice(b"RIFF");
                header.extend_from_slice(&(data_length as u32 + 36).to_le_bytes());
                header.extend_from_slice(b"WAVEfmt ");
                header.extend_from_slice(&16u32.to_le_bytes());
                header.extend_from_slice(&1u16.to_le_bytes());
                header.extend_from_slice(&2u16.to_le_bytes());
                header.extend_from_slice(&44100u32.to_le_bytes());
                header.extend_from_slice(&176400u32.to_le_bytes());
                header.extend_from_slice(&4u16.to_le_bytes());
                header.extend_from_slice(&16u16.to_le_bytes());
                header.extend_from_slice(b"data");
                header.extend_from_slice(&(data_length as u32).to_le_bytes());
                destination.write_all(&header).map_err(|e| e.to_string())?;
                let mut source_hash = Sha256::new();
                source_hash.update(&header);
                let mut buffer = [0; 2352 * 32];
                let mut remaining = data_length;
                while remaining != 0 {
                    let count = remaining.min(buffer.len() as u64) as usize;
                    source
                        .read_exact(&mut buffer[..count])
                        .map_err(|e| format!("Truncated CD track {}: {e}", track.number))?;
                    // BINARY BIN audio is interleaved little-endian signed PCM;
                    // preserve each source sample verbatim in the PCM WAV payload.
                    destination
                        .write_all(&buffer[..count])
                        .map_err(|e| e.to_string())?;
                    source_hash.update(&buffer[..count]);
                    remaining -= count as u64;
                }
                destination.sync_all().map_err(|e| e.to_string())?;
                drop(destination);
                let (length, written_hash) = hash_file(&path)?;
                let source_hash = format!("{:x}", source_hash.finalize());
                if length != data_length + 44 || source_hash != written_hash {
                    return Err(format!(
                        "CD track {} source/destination checksum mismatch",
                        track.number
                    ));
                }
                Ok(ManifestFile {
                    path: relative,
                    bytes: length,
                    sha256: written_hash,
                })
            })();
            match outcome {
                Ok(entry) => entries.push(entry),
                Err(error) => {
                    let _ = fs::remove_file(&path);
                    warnings.push(format!("CD track {:02} unavailable: {error}", track.number));
                }
            }
            *completed += 1;
        }
        if entries.len() == initial_entries {
            warnings.push("Music disabled: no CD audio tracks could be extracted".into());
        }
        warnings
    }
}

pub(crate) fn safe_target(root: &Path, relative: &str) -> SetupResult<PathBuf> {
    let relative_path = Path::new(relative);
    if relative_path
        .components()
        .any(|c| !matches!(c, std::path::Component::Normal(_)))
        || relative.contains([':', '\\'])
    {
        return Err("Unsafe installation target path".into());
    }
    let target = root.join(relative_path);
    let mut parent = target.parent();
    while let Some(path) = parent {
        if path.exists()
            && !fs::canonicalize(path)
                .map_err(|e| e.to_string())?
                .starts_with(root)
        {
            return Err(format!(
                "Installation path escapes its destination: {}",
                path.display()
            ));
        }
        if path == root {
            break;
        }
        parent = path.parent();
    }
    if target.exists()
        && (!target.is_file()
            || !fs::canonicalize(&target)
                .map_err(|e| e.to_string())?
                .starts_with(root))
    {
        return Err(format!(
            "Unsafe existing installation target {}",
            target.display()
        ));
    }
    Ok(target)
}

fn commit_stage(
    root: &Path,
    stage: &Stage,
    nonce: &str,
    manifest: InstallManifest,
    callback: &mut impl FnMut(SetupProgress),
) -> SetupResult<(usize, Option<PathBuf>, ValidationReport)> {
    let backup_root = root.join("v2000-backups").join(nonce);
    let mut replacements = Vec::new();
    for entry in &manifest.files {
        let staged = stage.path.join(&entry.path);
        if !staged.exists() {
            continue;
        } // Retained entries during music-only update.
        let target = safe_target(root, &entry.path)?;
        if target.exists() && hash_file(&target)? == (entry.bytes, entry.sha256.clone()) {
            continue;
        }
        replacements.push((entry.path.clone(), staged, target));
    }
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    write_stage(stage, MANIFEST_FILENAME, &manifest_bytes)?;
    replacements.push((
        MANIFEST_FILENAME.into(),
        stage.path.join(MANIFEST_FILENAME),
        safe_target(root, MANIFEST_FILENAME)?,
    ));
    // Preflight every target before the first existing file is moved.
    let mut applied: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();
    let mut report = None;
    let result: SetupResult<()> = (|| {
        for (index, (relative, staged, target)) in replacements.iter().enumerate() {
            progress(
                callback,
                "Verifying installation",
                relative,
                index,
                replacements.len(),
            );
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let backup = if target.exists() {
                let backup = safe_target(root, &format!("v2000-backups/{nonce}/{relative}"))?;
                fs::create_dir_all(backup.parent().unwrap()).map_err(|e| e.to_string())?;
                fs::rename(target, &backup)
                    .map_err(|e| format!("Cannot preserve {}: {e}", target.display()))?;
                Some(backup)
            } else {
                None
            };
            applied.push((target.clone(), backup));
            fs::rename(staged, target)
                .map_err(|e| format!("Cannot install {}: {e}", target.display()))?;
            if relative != MANIFEST_FILENAME {
                let expected = manifest
                    .files
                    .iter()
                    .find(|entry| entry.path == *relative)
                    .unwrap();
                if hash_file(target)? != (expected.bytes, expected.sha256.clone()) {
                    return Err(format!("Installed checksum mismatch: {relative}"));
                }
            }
        }
        let validation = super::validate_installation(root);
        if !validation.is_ready() {
            return Err(format!(
                "Installation validation failed: {}",
                validation
                    .issues
                    .iter()
                    .filter(|issue| issue.severity == super::IssueSeverity::Error)
                    .map(|issue| issue.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }
        report = Some(validation);
        Ok(())
    })();
    if let Err(error) = result {
        let mut rollback_errors = Vec::new();
        for (target, backup) in applied.into_iter().rev() {
            if target.exists() {
                if let Err(error) = fs::remove_file(&target) {
                    rollback_errors.push(error.to_string());
                }
            }
            if let Some(backup) = backup {
                if let Err(error) = fs::rename(backup, target) {
                    rollback_errors.push(error.to_string());
                }
            }
        }
        if rollback_errors.is_empty() {
            return Err(format!("{error}; previous files restored"));
        }
        return Err(format!(
            "{error}; rollback needs attention: {}",
            rollback_errors.join("; ")
        ));
    }
    Ok((
        replacements.len(),
        backup_root.exists().then_some(backup_root),
        report.unwrap(),
    ))
}
