use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use v2k_extract::{
    export_global_sounds, export_models, export_sprites, source_key, ExportReport, ExportResult,
    V2000SoundSources,
};
use v2k_formats::ovl::OvlFile;
use v2k_formats::palette::{BRIGHTEST_SHADE, SHADE_LEVELS};
use v2k_formats::preload::PreloadDat;

#[derive(Parser)]
#[command(
    name = "v2k-extract",
    about = "Extract V2000 assets with the same Rust decoders used by the port"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// V2000 directory containing PRELOAD.DAT and Overlay/.
    #[arg(long, short = 'd', default_value = ".", global = true)]
    data_dir: PathBuf,

    /// Destination for generated assets and manifests.
    #[arg(long, short, default_value = "extracted", global = true)]
    output: PathBuf,

    /// Scan only these OVL files for sprites/models. Relative paths are
    /// resolved from data-dir.
    #[arg(long = "ovl", global = true)]
    ovls: Vec<PathBuf>,

    /// Do not scan embedded PRELOAD.DAT OVLs for sprites/models. The canonical
    /// sound pool still reads its level-2 table from PRELOAD.DAT.
    #[arg(long, global = true)]
    skip_preload: bool,

    /// Sprite shade row, from 0 (darkest) through the brightest authored row.
    #[arg(long, default_value_t = BRIGHTEST_SHADE, value_parser = parse_shade, global = true)]
    shade: usize,

    /// Preserve successful sources and record section warnings instead of
    /// failing the complete staged extraction.
    #[arg(long, global = true)]
    keep_going: bool,
}

#[derive(Clone, Copy, Subcommand)]
enum Command {
    /// Export sprites, models, and the canonical global sound pool.
    All,
    /// Export Section-3 sprites as RGBA PNGs.
    Sprites,
    /// Export Section-8 models as diagnostic OBJ files.
    Models,
    /// Export the 110-slot global sound pool as 52 WAVs plus alias metadata.
    Sounds,
}

impl Command {
    fn sprites(self) -> bool {
        matches!(self, Self::All | Self::Sprites)
    }

    fn models(self) -> bool {
        matches!(self, Self::All | Self::Models)
    }

    fn sounds(self) -> bool {
        matches!(self, Self::All | Self::Sounds)
    }

    fn file_assets(self) -> bool {
        self.sprites() || self.models()
    }
}

#[derive(Default, Deserialize, Serialize)]
struct BatchManifest {
    schema_version: u32,
    sources: Vec<SourceReport>,
    global_sounds: Option<GlobalSoundReport>,
    totals: AssetCounts,
}

#[derive(Deserialize, Serialize)]
struct GlobalSoundReport {
    level2_source: String,
    level3_source: String,
    slots: usize,
    pcm_wavs: usize,
}

#[derive(Deserialize, Serialize)]
struct SourceReport {
    source: String,
    key: String,
    assets: AssetCounts,
    warnings: Vec<String>,
}

#[derive(Clone, Copy, Default, Deserialize, Serialize)]
struct AssetCounts {
    sprites: usize,
    models: usize,
    global_sound_slots: usize,
    pcm_wavs: usize,
}

impl std::ops::AddAssign for AssetCounts {
    fn add_assign(&mut self, rhs: Self) {
        self.sprites += rhs.sprites;
        self.models += rhs.models;
        self.global_sound_slots += rhs.global_sound_slots;
        self.pcm_wavs += rhs.pcm_wavs;
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cli = Cli::parse();
    let final_output = cli.output.clone();
    let staged_output = StagedOutput::new(&final_output)?;
    cli.output = staged_output.path().to_path_buf();
    let mut manifest = BatchManifest {
        schema_version: v2k_extract::MANIFEST_SCHEMA_VERSION,
        ..BatchManifest::default()
    };

    let needs_preload = cli.command.sounds()
        || (cli.command.file_assets() && !cli.skip_preload && cli.ovls.is_empty());
    let preload = if needs_preload {
        let preload_path = cli.data_dir.join("PRELOAD.DAT");
        Some(PreloadDat::parse(&std::fs::read(&preload_path)?)?)
    } else {
        None
    };

    if cli.command.file_assets() && !cli.skip_preload && cli.ovls.is_empty() {
        for embedded in &preload.as_ref().expect("preload required").embedded_ovls {
            let source = format!("PRELOAD.DAT[{}]", embedded.index);
            extract_ovl(&cli, &source, &embedded.ovl, &mut manifest)?;
        }
    }

    if cli.command.file_assets() {
        for path in ovl_paths(&cli)? {
            let data = std::fs::read(&path)?;
            let ovl = OvlFile::parse(&data)?;
            let source = logical_source(&cli.data_dir, &path);
            extract_ovl(&cli, &source, &ovl, &mut manifest)?;
        }
    }

    if cli.command.sounds() {
        export_canonical_sound_pool(
            &cli,
            preload.as_ref().expect("preload required"),
            &mut manifest,
        )?;
    }

    std::fs::create_dir_all(&cli.output).map_err(|error| {
        format!(
            "failed to create staged output {}: {error}",
            cli.output.display()
        )
    })?;
    let manifest_path = cli.output.join("manifest.json");
    let file = std::fs::File::create(&manifest_path).map_err(|error| {
        format!(
            "failed to create batch manifest {}: {error}",
            manifest_path.display()
        )
    })?;
    let mut writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, &manifest)?;
    writer
        .flush()
        .map_err(|error| format!("failed to flush {}: {error}", manifest_path.display()))?;
    drop(writer);
    staged_output.commit()?;
    println!(
        "Extracted {} sprites, {} models, and {} global sound slots ({} PCM WAVs) from {} asset sources into {}",
        manifest.totals.sprites,
        manifest.totals.models,
        manifest.totals.global_sound_slots,
        manifest.totals.pcm_wavs,
        manifest.sources.len(),
        final_output.display(),
    );
    Ok(())
}

fn logical_source(data_dir: &Path, path: &Path) -> String {
    path.strip_prefix(data_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn parse_shade(value: &str) -> Result<usize, String> {
    let shade = value
        .parse::<usize>()
        .map_err(|_| format!("invalid shade level {value:?}"))?;
    if shade < SHADE_LEVELS {
        Ok(shade)
    } else {
        Err(format!(
            "shade level must be between 0 and {BRIGHTEST_SHADE}"
        ))
    }
}

fn export_canonical_sound_pool(
    cli: &Cli,
    preload: &PreloadDat,
    batch: &mut BatchManifest,
) -> Result<(), Box<dyn std::error::Error>> {
    const LEVEL2_SOURCE: &str = "PRELOAD.DAT[2]";
    // Section 11 is byte-identical across all four display tiers. Keep one
    // canonical copy without implying that variant 0 is a visual default.
    const LEVEL3_SOURCE: &str = "Overlay/0X3XX.OVL";

    let level2_ovl = preload
        .embedded_ovls
        .iter()
        .find(|embedded| embedded.index == 2)
        .ok_or("PRELOAD.DAT does not contain the system-level-2 OVL at embedded index 2")?;
    let level2 = v2k_formats::sections::parse_anim_sound(&level2_ovl.ovl)?;

    let level3_path = cli.data_dir.join("Overlay").join("0X3XX.OVL");
    let level3_ovl = OvlFile::parse(&std::fs::read(&level3_path)?)?;
    let level3 = v2k_formats::sections::parse_anim_sound(&level3_ovl)?;

    let report = export_global_sounds(
        &cli.output.join("sounds"),
        V2000SoundSources {
            level2_source: LEVEL2_SOURCE,
            level2: &level2,
            level3_source: LEVEL3_SOURCE,
            level3: &level3,
        },
    )?;
    batch.totals.global_sound_slots = report.slots;
    batch.totals.pcm_wavs = report.pcm_files;
    batch.global_sounds = Some(GlobalSoundReport {
        level2_source: LEVEL2_SOURCE.to_string(),
        level3_source: LEVEL3_SOURCE.to_string(),
        slots: report.slots,
        pcm_wavs: report.pcm_files,
    });
    Ok(())
}

fn ovl_paths(cli: &Cli) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut paths: Vec<PathBuf> = if cli.ovls.is_empty() {
        let overlay = cli.data_dir.join("Overlay");
        std::fs::read_dir(&overlay)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("ovl"))
            })
            .collect()
    } else {
        cli.ovls
            .iter()
            .map(|path| {
                if path.is_absolute() {
                    path.clone()
                } else {
                    cli.data_dir.join(path)
                }
            })
            .collect()
    };
    paths.sort_by_key(|path| path.to_string_lossy().to_ascii_lowercase());
    Ok(paths)
}

fn extract_ovl(
    cli: &Cli,
    source: &str,
    ovl: &OvlFile,
    batch: &mut BatchManifest,
) -> Result<(), Box<dyn std::error::Error>> {
    let key = source_key(source);
    if let Some(previous) = batch.sources.iter().find(|report| report.key == key) {
        return Err(format!(
            "asset source-key collision: {source:?} and {:?} both map to {key:?}",
            previous.source,
        )
        .into());
    }
    let mut assets = AssetCounts::default();
    let mut warnings = Vec::new();

    if cli.command.sprites() && has_sprite_payload(ovl) {
        let output_dir = cli.output.join("sprites").join(&key);
        let result = (|| -> ExportResult<ExportReport> {
            let atlas = v2k_formats::sections::parse_sprites(ovl)?;
            let sprites = atlas
                .decode_all_strict(cli.shade)
                .map_err(|error| format!("failed to decode every sprite in {source}: {error}"))?;
            export_sprites(source, &output_dir, &sprites, cli.shade)
        })();
        match retain_complete_asset_family("sprites", &output_dir, result, &mut warnings)? {
            Some(report) => assets.sprites = report.items,
            None => assets.sprites = 0,
        }
    }

    if cli.command.models() && ovl.has_data(v2k_formats::sections::idx::SPRITE_METADATA) {
        let output_dir = cli.output.join("models").join(&key);
        let result = (|| -> ExportResult<ExportReport> {
            let models = v2k_formats::sections::parse_models(ovl)?;
            export_models(source, &output_dir, &models)
        })();
        match retain_complete_asset_family("models", &output_dir, result, &mut warnings)? {
            Some(report) => assets.models = report.items,
            None => assets.models = 0,
        }
    }

    for warning in &warnings {
        eprintln!("warning: {source}: {warning}");
    }
    if !warnings.is_empty() && !cli.keep_going {
        return Err(format!(
            "{source} could not be extracted strictly: {} (use --keep-going to record the warning and retain other complete sources)",
            warnings.join("; "),
        )
        .into());
    }
    batch.totals += assets;
    batch.sources.push(SourceReport {
        source: source.to_string(),
        key,
        assets,
        warnings,
    });
    Ok(())
}

/// Record a source-local parser/decoder/export error without ever retaining a
/// partially serialized family. Strict mode rejects the accumulated warnings
/// below; `--keep-going` commits only complete families from other sources.
fn retain_complete_asset_family(
    family: &str,
    output_dir: &Path,
    result: ExportResult<ExportReport>,
    warnings: &mut Vec<String>,
) -> Result<Option<ExportReport>, Box<dyn std::error::Error>> {
    match result {
        Ok(report) => Ok(Some(report)),
        Err(error) => {
            if output_dir.exists() {
                fs::remove_dir_all(output_dir).map_err(|cleanup_error| {
                    format!(
                        "failed to clean partial {family} output {} after {error}: {cleanup_error}",
                        output_dir.display(),
                    )
                })?;
            }
            warnings.push(format!("{family}: {error}"));
            Ok(None)
        }
    }
}

fn has_sprite_payload(ovl: &OvlFile) -> bool {
    ovl.section(v2k_formats::sections::idx::SPRITE_ATLAS)
        .and_then(|section| section.data.get(..4))
        .is_some_and(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()) != 0)
}

const MANAGED_OUTPUT_MARKER: &str = ".v2k-extract-managed";

/// Build into a sibling directory and replace the prior generated tree only
/// after every parser and serializer succeeds. This prevents a failed strict
/// run from leaving a half-new tree and makes narrower commands deterministic:
/// files from an earlier `all` run cannot survive a later `sprites` run.
struct StagedOutput {
    final_path: PathBuf,
    staging_path: PathBuf,
    committed: bool,
}

impl StagedOutput {
    fn new(final_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let name = final_path
            .file_name()
            .filter(|name| !name.is_empty())
            .ok_or("output must name a generated directory, not a filesystem root")?;
        let parent = final_path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create output parent {}: {error}",
                parent.display()
            )
        })?;
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let staging_path = parent.join(format!(
            ".{}.v2k-extract-staging-{}-{nonce}",
            name.to_string_lossy(),
            std::process::id(),
        ));
        fs::create_dir(&staging_path).map_err(|error| {
            format!(
                "failed to create staging directory {}: {error}",
                staging_path.display()
            )
        })?;
        Ok(Self {
            final_path: final_path.to_path_buf(),
            staging_path,
            committed: false,
        })
    }

    fn path(&self) -> &Path {
        &self.staging_path
    }

    fn commit(mut self) -> Result<(), Box<dyn std::error::Error>> {
        let marker_path = self.staging_path.join(MANAGED_OUTPUT_MARKER);
        fs::write(
            &marker_path,
            "Generated by v2k-extract. Safe to replace as one complete tree.\n",
        )
        .map_err(|error| format!("failed to write {}: {error}", marker_path.display()))?;

        let mut backup = None;
        if self.final_path.exists() {
            ensure_managed_output(&self.final_path)?;
            let name = self.final_path.file_name().unwrap().to_string_lossy();
            let backup_path = self
                .final_path
                .with_file_name(format!(".{name}.v2k-extract-backup-{}", std::process::id(),));
            if backup_path.exists() {
                return Err(format!(
                    "refusing to replace {} while stale backup {} exists",
                    self.final_path.display(),
                    backup_path.display(),
                )
                .into());
            }
            fs::rename(&self.final_path, &backup_path).map_err(|error| {
                format!(
                    "failed to move managed output {} to backup {}: {error}",
                    self.final_path.display(),
                    backup_path.display()
                )
            })?;
            backup = Some(backup_path);
        }

        if let Err(error) = fs::rename(&self.staging_path, &self.final_path) {
            if let Some(backup_path) = &backup {
                let _ = fs::rename(backup_path, &self.final_path);
            }
            return Err(format!(
                "failed to publish staging directory {} as {}: {error}",
                self.staging_path.display(),
                self.final_path.display()
            )
            .into());
        }
        self.committed = true;

        if let Some(backup_path) = backup {
            fs::remove_dir_all(&backup_path).map_err(|error| {
                format!(
                    "published new output but failed to remove backup {}: {error}",
                    backup_path.display()
                )
            })?;
        }
        Ok(())
    }
}

impl Drop for StagedOutput {
    fn drop(&mut self) {
        if !self.committed && self.staging_path.exists() {
            let _ = fs::remove_dir_all(&self.staging_path);
        }
    }
}

fn ensure_managed_output(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !path.is_dir() {
        return Err(format!("output {} exists but is not a directory", path.display()).into());
    }
    if path.join(MANAGED_OUTPUT_MARKER).is_file() {
        return Ok(());
    }
    if fs::read_dir(path)?.next().is_none() {
        return Ok(());
    }

    // Accept the pre-marker exporter layout once, then install the marker on
    // replacement. Deserialize the complete root schema: merely mentioning
    // field names in an unrelated manifest must never authorize deletion.
    let legacy_manifest = path.join("manifest.json");
    if let Ok(json) = fs::read_to_string(legacy_manifest) {
        if serde_json::from_str::<BatchManifest>(&json)
            .is_ok_and(|manifest| manifest.schema_version == v2k_extract::MANIFEST_SCHEMA_VERSION)
        {
            return Ok(());
        }
    }
    Err(format!(
        "refusing to replace unmanaged non-empty output directory {}; choose an empty directory",
        path.display(),
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shade_parser_accepts_the_full_authored_range() {
        assert_eq!(parse_shade("0"), Ok(0));
        assert_eq!(
            parse_shade(&BRIGHTEST_SHADE.to_string()),
            Ok(BRIGHTEST_SHADE)
        );
        assert!(parse_shade(&SHADE_LEVELS.to_string()).is_err());
    }

    fn test_output(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("v2k-extract-stage-{label}-{}", std::process::id(),))
    }

    fn empty_ovl() -> OvlFile {
        let mut bytes = Vec::new();
        for _ in 0..v2k_formats::ovl::NUM_SECTIONS {
            bytes.extend_from_slice(b"abcd");
            bytes.extend_from_slice(&0u32.to_le_bytes());
        }
        OvlFile::parse(&bytes).unwrap()
    }

    fn undecodable_sprite_ovl() -> OvlFile {
        let mut bytes = Vec::new();
        for section in 0..v2k_formats::ovl::NUM_SECTIONS {
            bytes.extend_from_slice(b"abcd");
            bytes.extend_from_slice(
                &(if section == v2k_formats::sections::idx::SPRITE_ATLAS {
                    v2k_formats::sprites::ATLAS_STRIDE as u32
                } else {
                    0
                })
                .to_le_bytes(),
            );
            if section == v2k_formats::sections::idx::SPRITE_ATLAS {
                bytes.extend_from_slice(&28u32.to_le_bytes());
                bytes.extend_from_slice(&[0u8; 28]);
                bytes.extend_from_slice(&0xffffu16.to_le_bytes());
            }
        }
        OvlFile::parse(&bytes).unwrap()
    }

    fn test_cli(output: PathBuf) -> Cli {
        Cli {
            command: Command::Sprites,
            data_dir: PathBuf::from("."),
            output,
            ovls: Vec::new(),
            skip_preload: true,
            shade: BRIGHTEST_SHADE,
            keep_going: false,
        }
    }

    #[test]
    fn assetless_sources_remain_visible_and_key_collisions_fail() {
        let output = test_output("source-report");
        let _ = fs::remove_dir_all(&output);
        fs::create_dir_all(&output).unwrap();
        let cli = test_cli(output.clone());
        let ovl = empty_ovl();
        let mut batch = BatchManifest::default();

        extract_ovl(&cli, "Overlay/0X14XX.OVL", &ovl, &mut batch).unwrap();
        assert_eq!(batch.sources.len(), 1);
        assert_eq!(batch.sources[0].assets.sprites, 0);

        let error = extract_ovl(&cli, "alternate/0X14XX.OVL", &ovl, &mut batch)
            .unwrap_err()
            .to_string();
        assert!(error.contains("source-key collision"));
        let _ = fs::remove_dir_all(output);
    }

    #[test]
    fn keep_going_records_decode_failures_without_partial_source_output() {
        let output = test_output("decode-warning");
        let _ = fs::remove_dir_all(&output);
        fs::create_dir_all(&output).unwrap();
        let mut cli = test_cli(output.clone());
        cli.keep_going = true;
        let mut batch = BatchManifest::default();

        extract_ovl(
            &cli,
            "Overlay/broken.OVL",
            &undecodable_sprite_ovl(),
            &mut batch,
        )
        .unwrap();

        assert_eq!(batch.sources.len(), 1);
        assert_eq!(batch.sources[0].warnings.len(), 1);
        assert!(batch.sources[0].warnings[0].contains("failed to decode every sprite"));
        assert!(!output.join("sprites").join("broken").exists());
        let _ = fs::remove_dir_all(output);
    }

    #[test]
    fn staged_output_replaces_a_managed_tree_without_stale_files() {
        let output = test_output("replace");
        let _ = fs::remove_dir_all(&output);
        fs::create_dir_all(&output).unwrap();
        fs::write(output.join(MANAGED_OUTPUT_MARKER), "managed").unwrap();
        fs::write(output.join("stale.obj"), "old").unwrap();

        let staged = StagedOutput::new(&output).unwrap();
        fs::write(staged.path().join("manifest.json"), "{\"sources\":[]}").unwrap();
        staged.commit().unwrap();

        assert!(output.join("manifest.json").is_file());
        assert!(!output.join("stale.obj").exists());
        assert!(output.join(MANAGED_OUTPUT_MARKER).is_file());
        let _ = fs::remove_dir_all(output);
    }

    #[test]
    fn staged_output_refuses_to_replace_unmanaged_files() {
        let output = test_output("unmanaged");
        let _ = fs::remove_dir_all(&output);
        fs::create_dir_all(&output).unwrap();
        fs::write(output.join("keep.txt"), "user data").unwrap();

        let staged = StagedOutput::new(&output).unwrap();
        fs::write(staged.path().join("manifest.json"), "{\"sources\":[]}").unwrap();
        let error = staged.commit().unwrap_err().to_string();

        assert!(error.contains("unmanaged"));
        assert_eq!(
            fs::read_to_string(output.join("keep.txt")).unwrap(),
            "user data"
        );
        let _ = fs::remove_dir_all(output);
    }

    #[test]
    fn staged_output_rejects_manifest_field_name_lookalikes() {
        let output = test_output("manifest-lookalike");
        let _ = fs::remove_dir_all(&output);
        fs::create_dir_all(&output).unwrap();
        fs::write(
            output.join("manifest.json"),
            r#"{"schema_version":"not a version","sources":"not an array"}"#,
        )
        .unwrap();

        let staged = StagedOutput::new(&output).unwrap();
        fs::write(staged.path().join("manifest.json"), "{\"sources\":[]}").unwrap();
        let error = staged.commit().unwrap_err().to_string();

        assert!(error.contains("unmanaged"));
        assert!(output.join("manifest.json").is_file());
        let _ = fs::remove_dir_all(output);
    }

    #[test]
    fn staged_output_accepts_the_complete_pre_marker_manifest_schema() {
        let output = test_output("legacy-manifest");
        let _ = fs::remove_dir_all(&output);
        fs::create_dir_all(&output).unwrap();
        let legacy = BatchManifest {
            schema_version: v2k_extract::MANIFEST_SCHEMA_VERSION,
            ..BatchManifest::default()
        };
        fs::write(
            output.join("manifest.json"),
            serde_json::to_vec(&legacy).unwrap(),
        )
        .unwrap();
        fs::write(output.join("stale.obj"), "old").unwrap();

        let staged = StagedOutput::new(&output).unwrap();
        fs::write(staged.path().join("manifest.json"), "{\"sources\":[]}").unwrap();
        staged.commit().unwrap();

        assert!(!output.join("stale.obj").exists());
        assert!(output.join(MANAGED_OUTPUT_MARKER).is_file());
        let _ = fs::remove_dir_all(output);
    }
}
