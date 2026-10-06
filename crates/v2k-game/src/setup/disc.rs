use super::{expected_overlay_names, find_child, SetupResult};
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

const LOGICAL_SECTOR: u64 = 2048;
const RAW_SECTOR: u64 = 2352;
const MAX_DIRECTORY_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct DiscSummary {
    pub source: PathBuf,
    pub required_files: usize,
    pub audio_tracks: Vec<u8>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct DataTrack {
    pub file: PathBuf,
    pub start_sector: u64,
    pub end_sector: u64,
    pub sector_bytes: u64,
    pub payload_offset: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct AudioTrack {
    pub number: u8,
    pub file: PathBuf,
    pub start_sector: u64,
    pub end_sector: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct DiscFile {
    pub extent: u32,
    pub bytes: u32,
}

#[derive(Debug, Clone)]
pub struct DiscImage {
    pub(crate) source: PathBuf,
    pub(crate) data: DataTrack,
    pub(crate) files: BTreeMap<String, DiscFile>,
    pub(crate) audio: Vec<AudioTrack>,
    pub(crate) warnings: Vec<String>,
}

impl DiscImage {
    pub fn open(path: &Path) -> SetupResult<Self> {
        let source = fs::canonicalize(path)
            .map_err(|e| format!("Cannot open disc image {}: {e}", path.display()))?;
        let extension = source
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let (data, audio, mut warnings) = match extension.as_str() {
            "cue" => parse_cue(&source)?,
            "bin" => {
                let cue_name = source
                    .with_extension("cue")
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                if let Some(cue) = find_child(source.parent().unwrap(), &cue_name) {
                    let parsed = parse_cue(&cue)?;
                    if parsed.0.file != source {
                        return Err("Companion CUE data track refers to a different BIN; select that CUE directly".into());
                    }
                    parsed
                } else {
                    let length = fs::metadata(&source).map_err(|e| e.to_string())?.len();
                    if length % RAW_SECTOR != 0 {
                        return Err("BIN image is not a complete sequence of 2352-byte sectors; select its CUE sheet".into());
                    }
                    (DataTrack { file: source.clone(), start_sector: 0, end_sector: length / RAW_SECTOR, sector_bytes: RAW_SECTOR, payload_offset: 16 }, Vec::new(), vec!["Music disabled: companion CUE sheet missing. Select the CUE to recover authored audio-track boundaries.".into()])
                }
            }
            "iso" => {
                let length = fs::metadata(&source).map_err(|e| e.to_string())?.len();
                if length % LOGICAL_SECTOR != 0 {
                    return Err("ISO image has a partial 2048-byte sector".into());
                }
                (
                    DataTrack {
                        file: source.clone(),
                        start_sector: 0,
                        end_sector: length / LOGICAL_SECTOR,
                        sector_bytes: LOGICAL_SECTOR,
                        payload_offset: 0,
                    },
                    Vec::new(),
                    vec!["Music disabled: this data-only ISO contains no CD audio tracks.".into()],
                )
            }
            _ => return Err("Select a V2000 BIN, CUE or ISO image".into()),
        };
        let files = read_iso_files(&data)?;
        let mut admitted = BTreeMap::new();
        let prefix = if files.contains_key("V2000/PRELOAD.DAT") {
            "V2000/"
        } else {
            ""
        };
        let mut needed = vec!["PRELOAD.DAT".to_string()];
        needed.extend(expected_overlay_names().map(|name| format!("Overlay/{name}")));
        for relative in needed {
            let key = format!("{prefix}{relative}").to_ascii_uppercase();
            let file = files
                .get(&key)
                .ok_or_else(|| format!("Disc is missing required file {key}"))?;
            admitted.insert(relative, file.clone());
        }
        for name in ["BANNERHI.AVI", "BANNERLO.AVI"] {
            if let Some(file) = files.get(&format!("{prefix}{name}")) {
                admitted.insert(name.to_string(), file.clone());
            } else {
                warnings.push(format!(
                    "Disc is missing {name}; intro video will be unavailable"
                ));
            }
        }
        let missing: Vec<_> = (2..=11)
            .filter(|n| !audio.iter().any(|t| t.number == *n))
            .collect();
        if !audio.is_empty() && !missing.is_empty() {
            warnings.push(format!(
                "Disc soundtrack incomplete; no audio tracks {missing:?}"
            ));
        }
        Ok(Self {
            source,
            data,
            files: admitted,
            audio,
            warnings,
        })
    }

    pub fn summary(&self) -> DiscSummary {
        DiscSummary {
            source: self.source.clone(),
            required_files: self.files.len(),
            audio_tracks: self.audio.iter().map(|t| t.number).collect(),
            warnings: self.warnings.clone(),
        }
    }

    /// Bounded access to the allowlisted retail files discovered at open time.
    pub fn read_file(&self, relative: &str) -> SetupResult<Vec<u8>> {
        let file = self
            .files
            .get(relative)
            .ok_or_else(|| format!("File is outside the installation allowlist: {relative}"))?;
        if file.bytes as u64 > 128 * 1024 * 1024 {
            return Err(format!(
                "Disc file {relative} exceeds the bounded asset size"
            ));
        }
        self.data.read_extent(file.extent, file.bytes)
    }
}

impl DataTrack {
    fn sectors(&self) -> u64 {
        self.end_sector - self.start_sector
    }
    fn sector(&self, file: &mut File, sector: u64) -> SetupResult<[u8; 2048]> {
        if sector >= self.sectors() {
            return Err(format!("ISO sector {sector} is outside its data track"));
        }
        let offset = (self.start_sector + sector)
            .checked_mul(self.sector_bytes)
            .and_then(|n| n.checked_add(self.payload_offset))
            .ok_or("Disc sector offset overflow")?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| e.to_string())?;
        let mut bytes = [0; 2048];
        file.read_exact(&mut bytes)
            .map_err(|e| format!("Truncated data sector {sector}: {e}"))?;
        Ok(bytes)
    }
    fn read_extent(&self, extent: u32, length: u32) -> SetupResult<Vec<u8>> {
        let sectors = (length as u64).div_ceil(LOGICAL_SECTOR);
        if (extent as u64)
            .checked_add(sectors)
            .is_none_or(|end| end > self.sectors())
        {
            return Err("ISO extent leaves the data track".into());
        }
        let mut file = File::open(&self.file).map_err(|e| e.to_string())?;
        let mut bytes = Vec::with_capacity(length as usize);
        for sector in 0..sectors {
            let data = self.sector(&mut file, extent as u64 + sector)?;
            let remaining = length as usize - bytes.len();
            bytes.extend_from_slice(&data[..remaining.min(2048)]);
        }
        Ok(bytes)
    }
}

#[derive(Debug)]
struct CueTrack {
    number: u8,
    mode: String,
    file: PathBuf,
    index0: Option<u64>,
    index1: Option<u64>,
}

fn parse_cue(path: &Path) -> SetupResult<(DataTrack, Vec<AudioTrack>, Vec<String>)> {
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > 1024 * 1024 {
        return Err("CUE sheet is unreasonably large".into());
    }
    let text = fs::read_to_string(path).map_err(|e| format!("Cannot read CUE: {e}"))?;
    if text.len() > 1024 * 1024 {
        return Err("CUE sheet is unreasonably large".into());
    }
    let mut tracks: Vec<CueTrack> = Vec::new();
    let mut current_file = None;
    for (line_index, line) in text.lines().enumerate() {
        let line = line.trim();
        let command = line
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_uppercase();
        match command.as_str() {
            "FILE" => {
                let rest = line[4..].trim();
                let (name, kind) = if let Some(quoted) = rest.strip_prefix('"') {
                    let end = quoted
                        .find('"')
                        .ok_or("CUE FILE has an unterminated filename")?;
                    (&quoted[..end], quoted[end + 1..].trim())
                } else {
                    rest.rsplit_once(char::is_whitespace)
                        .ok_or("CUE FILE requires a type")?
                };
                if !kind.eq_ignore_ascii_case("BINARY") {
                    return Err("Only BINARY CUE files are supported".into());
                }
                let relative = Path::new(name);
                if relative
                    .components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
                    || name.contains(':')
                {
                    return Err("CUE FILE must stay inside the image directory".into());
                }
                let parent = path.parent().ok_or("CUE has no parent directory")?;
                let candidate = parent.join(relative);
                let candidate = if candidate.exists() {
                    candidate
                } else {
                    find_child(parent, name).unwrap_or(candidate)
                };
                let parent = fs::canonicalize(parent).map_err(|e| e.to_string())?;
                let canonical = if candidate.exists() {
                    fs::canonicalize(candidate).map_err(|e| e.to_string())?
                } else {
                    parent.join(relative)
                };
                if !canonical.starts_with(parent) {
                    return Err("CUE FILE resolves outside the image directory".into());
                }
                current_file = Some(canonical);
            }
            "TRACK" => {
                let words: Vec<_> = line.split_whitespace().collect();
                if words.len() != 3 {
                    return Err(format!("Malformed CUE TRACK on line {}", line_index + 1));
                }
                let number: u8 = words[1].parse().map_err(|_| "Invalid CUE track number")?;
                if number == 0 || number > 99 || tracks.last().is_some_and(|t| t.number >= number) {
                    return Err("CUE tracks must have increasing numbers 01–99".into());
                }
                let mode = words[2].to_ascii_uppercase();
                if !matches!(mode.as_str(), "AUDIO" | "MODE1/2352" | "MODE1/2048") {
                    return Err(format!("Unsupported CUE track mode {mode}"));
                }
                tracks.push(CueTrack {
                    number,
                    mode,
                    file: current_file
                        .clone()
                        .ok_or("CUE TRACK appears before FILE")?,
                    index0: None,
                    index1: None,
                });
            }
            "INDEX" => {
                let words: Vec<_> = line.split_whitespace().collect();
                if words.len() != 3 {
                    return Err("Malformed CUE INDEX".into());
                }
                let track = tracks.last_mut().ok_or("CUE INDEX appears before TRACK")?;
                let sector = cue_time(words[2])?;
                match words[1] {
                    "00" => {
                        if track.index0.replace(sector).is_some() {
                            return Err("Duplicate INDEX 00".into());
                        }
                    }
                    "01" => {
                        if track.index1.replace(sector).is_some() {
                            return Err("Duplicate INDEX 01".into());
                        }
                    }
                    _ => {}
                }
            }
            // Metadata and authored gaps do not change FILE sector offsets.
            "" | "REM" | "TITLE" | "PERFORMER" | "SONGWRITER" | "CATALOG" | "ISRC" | "FLAGS"
            | "PREGAP" | "POSTGAP" => {}
            _ => return Err(format!("Unsupported CUE command {command}")),
        }
    }
    let mut data = None;
    let mut audio = Vec::new();
    let mut warnings = Vec::new();
    let mut completed_files = HashSet::new();
    let mut previous_file: Option<&Path> = None;
    for (index, track) in tracks.iter().enumerate() {
        if previous_file != Some(track.file.as_path()) {
            if !completed_files.insert(track.file.clone()) {
                return Err(
                    "CUE reuses a FILE non-contiguously; track ranges would overlap".into(),
                );
            }
            previous_file = Some(&track.file);
        }
        let sector_bytes = if track.mode == "MODE1/2048" {
            LOGICAL_SECTOR
        } else {
            RAW_SECTOR
        };
        if tracks.iter().any(|other| {
            other.file == track.file && (other.mode == "MODE1/2048") != (track.mode == "MODE1/2048")
        }) {
            return Err("Mixed 2048/2352 sector sizes in one CUE FILE are unsupported".into());
        }
        let length = match fs::metadata(&track.file) {
            Ok(metadata) if metadata.is_file() => metadata.len(),
            result => {
                let reason = result
                    .err()
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "not a regular file".into());
                if track.mode == "AUDIO" {
                    warnings.push(format!(
                        "CD track {:02} unavailable: {}: {reason}",
                        track.number,
                        track.file.display()
                    ));
                    continue;
                }
                return Err(format!(
                    "CUE data file {} unavailable: {reason}",
                    track.file.display()
                ));
            }
        };
        let start = track
            .index1
            .ok_or_else(|| format!("CUE track {} has no INDEX 01", track.number))?;
        if track.index0.is_some_and(|zero| zero > start) {
            return Err("INDEX 00 occurs after INDEX 01".into());
        }
        let next_boundary = tracks
            .get(index + 1)
            .filter(|next| next.file == track.file)
            .map(|next| {
                next.index0
                    .or(next.index1)
                    .ok_or("Next CUE track has no index")
            })
            .transpose()?;
        let declared_end = next_boundary.unwrap_or(length / sector_bytes);
        let end = declared_end.min(length / sector_bytes);
        if start >= end
            || (track.mode == "AUDIO"
                && (declared_end > length / sector_bytes
                    || (next_boundary.is_none() && length % sector_bytes != 0)))
        {
            if track.mode == "AUDIO" {
                warnings.push(format!(
                    "CD track {:02} unavailable: its declared sectors are missing or truncated",
                    track.number
                ));
                continue;
            }
            return Err(format!(
                "CUE data track {} has invalid or out-of-bounds indices",
                track.number
            ));
        }
        if length % sector_bytes != 0
            && track.mode != "AUDIO"
            && next_boundary.is_none_or(|boundary| boundary > length / sector_bytes)
        {
            return Err(format!(
                "CUE data file {} has a partial sector",
                track.file.display()
            ));
        }
        if track.mode == "AUDIO" {
            if (2..=11).contains(&track.number) {
                audio.push(AudioTrack {
                    number: track.number,
                    file: track.file.clone(),
                    start_sector: start,
                    end_sector: end,
                });
            }
        } else {
            if data.is_some() {
                return Err(
                    "Disc has multiple data tracks; select a conventional V2000 image".into(),
                );
            }
            data = Some(DataTrack {
                file: track.file.clone(),
                start_sector: start,
                end_sector: end,
                sector_bytes,
                payload_offset: if sector_bytes == RAW_SECTOR { 16 } else { 0 },
            });
        }
    }
    if audio.is_empty() {
        warnings.push("Music disabled: the CUE contains no usable CD audio tracks.".into());
    }
    Ok((
        data.ok_or("CUE has no supported data track")?,
        audio,
        warnings,
    ))
}

fn cue_time(value: &str) -> SetupResult<u64> {
    let parts: Vec<_> = value.split(':').collect();
    if parts.len() != 3 {
        return Err("CUE time must be MM:SS:FF".into());
    }
    let minutes: u64 = parts[0].parse().map_err(|_| "Invalid CUE minutes")?;
    let seconds: u64 = parts[1].parse().map_err(|_| "Invalid CUE seconds")?;
    let frames: u64 = parts[2].parse().map_err(|_| "Invalid CUE frames")?;
    if seconds >= 60 || frames >= 75 {
        return Err("CUE seconds/frames exceed CD limits".into());
    }
    minutes
        .checked_mul(60)
        .and_then(|m| m.checked_add(seconds))
        .and_then(|s| s.checked_mul(75))
        .and_then(|s| s.checked_add(frames))
        .ok_or_else(|| "CUE timestamp overflow".into())
}

#[derive(Debug)]
struct DirectoryEntry {
    name: String,
    extent: u32,
    bytes: u32,
    directory: bool,
}

fn both32(bytes: &[u8], offset: usize) -> SetupResult<u32> {
    let little = u32::from_le_bytes(
        bytes
            .get(offset..offset + 4)
            .ok_or("Truncated ISO field")?
            .try_into()
            .unwrap(),
    );
    let big = u32::from_be_bytes(
        bytes
            .get(offset + 4..offset + 8)
            .ok_or("Truncated ISO field")?
            .try_into()
            .unwrap(),
    );
    if little != big {
        return Err("ISO duplicated integer fields disagree".into());
    }
    Ok(little)
}

fn directory_entry(record: &[u8], volume_sectors: u32) -> SetupResult<Option<DirectoryEntry>> {
    if record.len() < 34 || record[0] as usize != record.len() {
        return Err("Truncated ISO directory record".into());
    }
    let length = record[32] as usize;
    let name = record
        .get(33..33 + length)
        .ok_or("Truncated ISO filename")?;
    if name == [0] || name == [1] {
        return Ok(None);
    }
    if record[1] != 0 || record[26] != 0 || record[27] != 0 || record[25] & (0x80 | 0x04) != 0 {
        return Err("Extended/interleaved/multi-extent ISO records are unsupported".into());
    }
    let extent = both32(record, 2)?;
    let bytes = both32(record, 10)?;
    if (extent as u64) + (bytes as u64).div_ceil(LOGICAL_SECTOR) > volume_sectors as u64 {
        return Err("ISO file extent exceeds the declared volume".into());
    }
    let name = std::str::from_utf8(name).map_err(|_| "ISO filename is not ASCII/UTF-8")?;
    let name = name.split(';').next().unwrap().trim_end_matches('.');
    if name.is_empty()
        || name == "."
        || name == ".."
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'))
    {
        return Err(format!("Unsafe ISO filename {name:?}"));
    }
    Ok(Some(DirectoryEntry {
        name: name.to_ascii_uppercase(),
        extent,
        bytes,
        directory: record[25] & 2 != 0,
    }))
}

fn read_iso_files(track: &DataTrack) -> SetupResult<BTreeMap<String, DiscFile>> {
    let mut file = File::open(&track.file).map_err(|e| e.to_string())?;
    let mut descriptor = None;
    let mut terminated = false;
    let mut descriptor_end = 0;
    for sector in 16..80 {
        let bytes = track.sector(&mut file, sector)?;
        if &bytes[1..6] != b"CD001" || bytes[6] != 1 {
            return Err("Image does not contain a supported ISO9660 volume".into());
        }
        if bytes[0] == 1 {
            descriptor = Some(bytes);
        }
        if bytes[0] == 255 {
            terminated = true;
            descriptor_end = (sector + 1) * LOGICAL_SECTOR;
            break;
        }
    }
    if !terminated {
        return Err("ISO volume descriptor set has no bounded terminator".into());
    }
    let descriptor = descriptor.ok_or("ISO has no primary volume descriptor")?;
    if u16::from_le_bytes(descriptor[128..130].try_into().unwrap()) != 2048
        || u16::from_be_bytes(descriptor[130..132].try_into().unwrap()) != 2048
    {
        return Err("ISO logical block size must be 2048".into());
    }
    let volume = both32(&descriptor, 80)?;
    if volume as u64 > track.sectors() {
        return Err("ISO volume is longer than its data track".into());
    }
    let root = &descriptor[156..190];
    if root[0] != 34 || root[25] & 2 == 0 {
        return Err("Invalid ISO root directory".into());
    }
    let root_extent = both32(root, 2)?;
    let root_size = both32(root, 10)?;
    let mut files = BTreeMap::new();
    let mut pending = vec![(String::new(), root_extent, root_size, 0)];
    let mut visited = HashSet::new();
    let mut entries = 0usize;
    // Exact file aliases may be authored by ISO mastering software. Partial
    // overlaps and any overlap with directories/volume metadata are invalid.
    let mut extents = vec![(
        0u64,
        descriptor_end,
        true,
        "ISO volume metadata".to_string(),
    )];
    while let Some((prefix, extent, length, depth)) = pending.pop() {
        if depth > 8 || length as u64 > MAX_DIRECTORY_BYTES {
            return Err("ISO directory exceeds traversal bounds".into());
        }
        if !visited.insert(extent) {
            return Err("ISO directory tree contains a cycle or reused extent".into());
        }
        if (extent as u64) + (length as u64).div_ceil(LOGICAL_SECTOR) > volume as u64 {
            return Err("ISO directory exceeds the declared volume".into());
        }
        extents.push((
            extent as u64 * LOGICAL_SECTOR,
            extent as u64 * LOGICAL_SECTOR
                + (length as u64).div_ceil(LOGICAL_SECTOR) * LOGICAL_SECTOR,
            true,
            prefix.clone(),
        ));
        let bytes = track.read_extent(extent, length)?;
        let mut position = 0;
        let mut names = HashSet::new();
        while position < bytes.len() {
            let record_length = bytes[position] as usize;
            if record_length == 0 {
                position = ((position / 2048) + 1) * 2048;
                continue;
            }
            if position % 2048 + record_length > 2048 {
                return Err("ISO record crosses a sector boundary".into());
            }
            let record = bytes
                .get(position..position + record_length)
                .ok_or("Truncated ISO directory")?;
            position += record_length;
            let Some(entry) = directory_entry(record, volume)? else {
                continue;
            };
            entries += 1;
            if entries > 16384 || !names.insert(entry.name.clone()) {
                return Err("ISO has too many or duplicate directory entries".into());
            }
            let relative = if prefix.is_empty() {
                entry.name.clone()
            } else {
                format!("{prefix}/{}", entry.name)
            };
            if entry.directory {
                // Only retail data directories are needed. Other disc installers
                // and executable trees are never extracted or traversed.
                if matches!(relative.as_str(), "V2000" | "V2000/OVERLAY" | "OVERLAY") {
                    pending.push((relative, entry.extent, entry.bytes, depth + 1));
                }
            } else {
                if entry.bytes != 0 {
                    extents.push((
                        entry.extent as u64 * LOGICAL_SECTOR,
                        entry.extent as u64 * LOGICAL_SECTOR + entry.bytes as u64,
                        false,
                        relative.clone(),
                    ));
                }
                files.insert(
                    relative,
                    DiscFile {
                        extent: entry.extent,
                        bytes: entry.bytes,
                    },
                );
            }
        }
    }
    extents.sort_by_key(|entry| (entry.0, entry.1));
    for pair in extents.windows(2) {
        let first = &pair[0];
        let second = &pair[1];
        if second.0 < first.1 && (first.0 != second.0 || first.1 != second.1 || first.2 || second.2)
        {
            return Err(format!("ISO extents overlap: {} and {}", first.3, second.3));
        }
    }
    Ok(files)
}
