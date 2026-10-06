use serde::Serialize;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use v2k_formats::anim_sound::{AnimSoundTable, EntryType, SoundPool};

use crate::ExportResult;

pub const V2000_SOUND_SLOT_COUNT: usize = 110;
pub const V2000_PCM_COUNT: usize = 52;
pub const V2000_ALIAS_COUNT: usize = 58;
pub const V2000_LEVEL2_SOUND_COUNT: usize = 7;
pub const V2000_LEVEL3_SOUND_COUNT: usize = 103;

/// The two Section-11 tables that retail appends to `DAT_004FE64C`.
///
/// Level 2 comes from the third PRELOAD.DAT OVL and level 3 comes from the
/// variant-invariant Section-11 copy in `Overlay/0X3XX.OVL`. Variant 0 is used
/// only as the canonical sound fixture; it remains the low-resolution visual
/// tier and is not a default for sprite/model presentation.
pub struct V2000SoundSources<'a> {
    pub level2_source: &'a str,
    pub level2: &'a AnimSoundTable,
    pub level3_source: &'a str,
    pub level3: &'a AnimSoundTable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobalSoundExportReport {
    pub slots: usize,
    pub pcm_files: usize,
    /// WAV files plus manifest.json.
    pub files: usize,
}

#[derive(Serialize)]
struct GlobalSoundManifest<'a> {
    schema_version: u32,
    kind: &'static str,
    slot_count: usize,
    pcm_count: usize,
    alias_count: usize,
    audio_format: AudioFormat,
    sources: [SoundSourceRecord<'a>; 2],
    sounds: Vec<GlobalSoundRecord<'a>>,
}

#[derive(Serialize)]
struct AudioFormat {
    sample_rate_hz: u32,
    channels: u16,
    bits_per_sample: u16,
    encoding: &'static str,
}

#[derive(Serialize)]
struct SoundSourceRecord<'a> {
    system_level: u32,
    source: &'a str,
    global_base: usize,
    slot_count: usize,
    pcm_count: usize,
    alias_count: usize,
}

#[derive(Serialize)]
struct GlobalSoundRecord<'a> {
    global_id: usize,
    source_level: u32,
    source_local_id: usize,
    source: &'a str,
    entry_type: &'static str,
    alias_target_global_id: Option<usize>,
    pcm_bytes: Option<usize>,
    resolved_pcm_global_id: usize,
    resolved_pcm_file: String,
    resolution_chain: Vec<usize>,
    resolved_alias_hops: Vec<AliasHopRecord>,
    frequency_variance_16_16: Option<u32>,
    frequency_variance: Option<f64>,
    frequency_multiplier_16_16: Option<u32>,
    frequency_multiplier: Option<f64>,
    volume_multiplier_16_16: Option<u32>,
    volume_multiplier: Option<f64>,
    resolved_frequency_multiplier: f64,
    resolved_volume_multiplier: f64,
}

#[derive(Serialize)]
struct AliasHopRecord {
    global_id: usize,
    target_global_id: usize,
    frequency_variance_16_16: u32,
    frequency_multiplier_16_16: u32,
    volume_multiplier_16_16: u32,
}

/// Export the one canonical 110-slot V2000 global sound pool and its 52 PCM
/// samples. Display variants are deliberately excluded: they are alternate
/// copies of the same system resources, not additional global sound slots.
pub fn export_global_sounds(
    output_dir: &Path,
    sources: V2000SoundSources<'_>,
) -> ExportResult<GlobalSoundExportReport> {
    validate_retail_shape(&sources)?;
    std::fs::create_dir_all(output_dir)?;

    let tables = [sources.level2, sources.level3];
    let pool = SoundPool::from_tables(&tables);
    let source_records = [
        source_record(2, sources.level2_source, 0, sources.level2),
        source_record(
            3,
            sources.level3_source,
            V2000_LEVEL2_SOUND_COUNT,
            sources.level3,
        ),
    ];

    let pcm_files: Vec<Option<String>> = pool
        .slots()
        .iter()
        .map(|slot| {
            if slot.entry.entry_type != EntryType::DataBlob {
                return Ok(None);
            }
            let filename = format!("sound_{:03}.wav", slot.global_id);
            let wav = slot.table.blob_as_wav(slot.entry).ok_or_else(|| {
                format!(
                    "global sound {} has an invalid PCM offset or size",
                    slot.global_id
                )
            })?;
            std::fs::write(output_dir.join(&filename), wav)?;
            Ok(Some(filename))
        })
        .collect::<ExportResult<_>>()?;

    let mut records = Vec::with_capacity(pool.len());
    for slot in pool.slots() {
        let resolved = pool
            .resolve(slot.global_id)
            .map_err(|error| format!("global sound {} cannot resolve: {error}", slot.global_id))?;
        let (source_level, source) = match slot.table_index {
            0 => (2, sources.level2_source),
            1 => (3, sources.level3_source),
            other => return Err(format!("unexpected sound source table {other}").into()),
        };
        let pcm_file = pcm_files[resolved.pcm_global_id]
            .as_ref()
            .expect("resolved PCM slot must have a WAV filename")
            .clone();
        let is_alias = slot.entry.entry_type == EntryType::Alias;
        let resolved_alias_hops = resolved
            .alias_hops
            .iter()
            .map(|hop| AliasHopRecord {
                global_id: hop.global_id,
                target_global_id: hop.target_global_id,
                frequency_variance_16_16: hop.frequency_variance_16_16,
                frequency_multiplier_16_16: hop.frequency_multiplier_16_16,
                volume_multiplier_16_16: hop.volume_multiplier_16_16,
            })
            .collect();
        records.push(GlobalSoundRecord {
            global_id: slot.global_id,
            source_level,
            source_local_id: slot.local_id,
            source,
            entry_type: if is_alias { "alias" } else { "pcm" },
            alias_target_global_id: is_alias.then_some(slot.entry.offset_or_index as usize),
            pcm_bytes: (!is_alias).then_some(slot.entry.size_or_scale as usize),
            resolved_pcm_global_id: resolved.pcm_global_id,
            resolved_pcm_file: pcm_file,
            resolution_chain: resolved.resolution_chain,
            resolved_alias_hops,
            frequency_variance_16_16: is_alias.then_some(slot.entry.size_or_scale),
            frequency_variance: is_alias.then(|| slot.entry.frequency_variance_f64()),
            frequency_multiplier_16_16: is_alias.then_some(slot.entry.param1),
            frequency_multiplier: is_alias.then(|| slot.entry.frequency_multiplier_f64()),
            volume_multiplier_16_16: is_alias.then_some(slot.entry.param2),
            volume_multiplier: is_alias.then(|| slot.entry.volume_multiplier_f64()),
            resolved_frequency_multiplier: resolved.frequency_multiplier,
            resolved_volume_multiplier: resolved.volume_multiplier,
        });
    }

    let manifest = GlobalSoundManifest {
        schema_version: crate::MANIFEST_SCHEMA_VERSION,
        kind: "v2000_global_sounds",
        slot_count: pool.len(),
        pcm_count: V2000_PCM_COUNT,
        alias_count: V2000_ALIAS_COUNT,
        audio_format: AudioFormat {
            sample_rate_hz: 22_050,
            channels: 1,
            bits_per_sample: 16,
            encoding: "signed_pcm_le",
        },
        sources: source_records,
        sounds: records,
    };
    let file = File::create(output_dir.join("manifest.json"))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, &manifest)?;
    writer.flush()?;

    Ok(GlobalSoundExportReport {
        slots: pool.len(),
        pcm_files: V2000_PCM_COUNT,
        files: V2000_PCM_COUNT + 1,
    })
}

fn source_record<'a>(
    system_level: u32,
    source: &'a str,
    global_base: usize,
    table: &AnimSoundTable,
) -> SoundSourceRecord<'a> {
    SoundSourceRecord {
        system_level,
        source,
        global_base,
        slot_count: table.entries.len(),
        pcm_count: table.type1_count(),
        alias_count: table.type5_count(),
    }
}

fn validate_retail_shape(sources: &V2000SoundSources<'_>) -> ExportResult<()> {
    let level2 = sources.level2;
    let level3 = sources.level3;
    if level2.entries.len() != V2000_LEVEL2_SOUND_COUNT {
        return Err(format!(
            "system level 2 has {} Section-11 slots; retail requires {V2000_LEVEL2_SOUND_COUNT}",
            level2.entries.len()
        )
        .into());
    }
    if level3.entries.len() != V2000_LEVEL3_SOUND_COUNT {
        return Err(format!(
            "system level 3 has {} Section-11 slots; retail requires {V2000_LEVEL3_SOUND_COUNT}",
            level3.entries.len()
        )
        .into());
    }
    let pcm_count = level2.type1_count() + level3.type1_count();
    let alias_count = level2.type5_count() + level3.type5_count();
    if pcm_count != V2000_PCM_COUNT || alias_count != V2000_ALIAS_COUNT {
        return Err(format!(
            "global Section-11 pool has {pcm_count} PCM and {alias_count} alias slots; retail requires {V2000_PCM_COUNT} PCM and {V2000_ALIAS_COUNT} aliases"
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::anim_sound;

    fn sound_table(pcm_count: usize, alias_count: usize) -> AnimSoundTable {
        let entry_count = pcm_count + alias_count;
        let table_bytes = (entry_count + 1) * 0x14;
        let mut bytes = vec![0u8; table_bytes + pcm_count * 2];
        for index in 0..pcm_count {
            let entry = index * 0x14;
            bytes[entry..entry + 4].copy_from_slice(&1u32.to_le_bytes());
            bytes[entry + 4..entry + 8]
                .copy_from_slice(&(table_bytes as u32 + index as u32 * 2).to_le_bytes());
            bytes[entry + 8..entry + 12].copy_from_slice(&2u32.to_le_bytes());
        }
        for index in pcm_count..entry_count {
            let entry = index * 0x14;
            bytes[entry..entry + 4].copy_from_slice(&5u32.to_le_bytes());
            bytes[entry + 4..entry + 8].copy_from_slice(&0u32.to_le_bytes());
            bytes[entry + 8..entry + 12].copy_from_slice(&0x2000u32.to_le_bytes());
            bytes[entry + 12..entry + 16].copy_from_slice(&0x10000u32.to_le_bytes());
            bytes[entry + 16..entry + 20].copy_from_slice(&0x8000u32.to_le_bytes());
        }
        let end = entry_count * 0x14;
        bytes[end..end + 4].copy_from_slice(&99u32.to_le_bytes());
        anim_sound::parse_anim_sound(&bytes).unwrap()
    }

    #[test]
    fn canonical_export_writes_one_pool_and_52_wavs() {
        let level2 = sound_table(3, 4);
        let level3 = sound_table(49, 54);
        let output = std::env::temp_dir().join(format!("v2k-global-sounds-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&output);
        let report = export_global_sounds(
            &output,
            V2000SoundSources {
                level2_source: "PRELOAD.DAT[2]",
                level2: &level2,
                level3_source: "Overlay/0X3XX.OVL",
                level3: &level3,
            },
        )
        .unwrap();

        assert_eq!(
            report,
            GlobalSoundExportReport {
                slots: 110,
                pcm_files: 52,
                files: 53,
            }
        );
        assert_eq!(
            std::fs::read_dir(&output)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "wav"))
                .count(),
            52
        );
        let manifest = std::fs::read_to_string(output.join("manifest.json")).unwrap();
        assert!(manifest.contains("\"global_id\": 7"));
        assert!(manifest.contains("\"source_local_id\": 0"));
        assert!(manifest.contains("\"volume_multiplier_16_16\": 32768"));
        assert!(manifest.contains("\"resolution_chain\""));
        assert!(manifest.contains("\"resolved_alias_hops\""));
        let _ = std::fs::remove_dir_all(output);
    }
}
