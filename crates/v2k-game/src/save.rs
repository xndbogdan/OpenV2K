//! Save/load system: native checkpoint persistence, retail import and portable JSON.

mod paths;
pub use paths::SavePathContext;
use paths::SaveStoragePolicy;

mod native_restore;
pub use native_restore::{NativeSaveRestore, NativeSaveRestoreError};
mod native_snapshot;
pub use native_snapshot::{NativeSaveSnapshot, NativeSaveSnapshotError};

use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use v2k_formats::saves::{
    encode_native_save_slot, NativePlayerSnapshot, SaveSlot as RetailSaveSlot, SAVE_SLOT_COUNT,
    SETTINGS_SLOT_INDEX, SLOT_ROW_COUNT, STATE_PAYLOAD_SIZE,
};

const FIRST_GAMEPLAY_OVERLAY_ID: u32 = 13;
const LAST_GAMEPLAY_OVERLAY_ID: u32 = 48;
const OCCUPIED_SLOT_ERROR: &str = "Slot is occupied; overwrite confirmation required";
const EMPTY_OVERWRITE_ERROR: &str = "Slot is empty; confirmed overwrite is not applicable";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveWriteDisposition {
    EmptySlotOnly,
    ConfirmedOverwrite,
}

/// Number of rows in the retail `0x4C1D28` list.
///
/// Rows 0..13 are playable save slots. Row 14 is drawn by
/// `FUN_0043BB20` as the informational settings row and deliberately has no
/// selection callback. Keep this single constant shared by persistence and
/// menu navigation so the portable save layer cannot silently truncate or
/// make the reserved row writable again.
pub const NUM_SLOTS: usize = SLOT_ROW_COUNT;
/// Number of rows that can contain an ordinary game save.
pub const NUM_SAVE_SLOTS: usize = SAVE_SLOT_COUNT;
/// Informational final row used by retail for game settings.
pub const SETTINGS_SLOT: usize = SETTINGS_SLOT_INDEX;

/// Data stored in a save slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveSlot {
    pub level_id: u32,
    pub level_name: String,
    pub timestamp: String,
    /// Proven player state required to enter a saved world without fabricating
    /// a Section-13 player spawn. Old level-only JSON files deserialize with
    /// `None` and are deliberately not loadable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<SavedPlayerState>,
    /// Persistence provenance. Native dispatch carries the complete snapshot
    /// through NativeSaveRestore; portable JSON retains its player subset.
    #[serde(default)]
    pub source: SaveSource,
    /// Complete native snapshot retained for restoration and further RE.
    /// Portable JSON never serializes or impersonates this retail snapshot.
    #[serde(skip)]
    pub native: Option<NativeCompatibilityPreview>,
}

/// Player motion/hull fields shared by portable and native restoration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedPlayerState {
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub heading_raw: u16,
    /// Exact wrapping body attitude words restored by native 443560/413F70
    /// and the portable snapshot path.
    pub pitch_raw: u16,
    pub roll_raw: u16,
    pub health_raw: i32,
}

impl From<NativePlayerSnapshot> for SavedPlayerState {
    fn from(player: NativePlayerSnapshot) -> Self {
        Self {
            position_raw: player.position_raw,
            velocity_raw: player.velocity_raw,
            heading_raw: player.heading_raw,
            pitch_raw: player.pitch_raw,
            roll_raw: player.roll_raw,
            health_raw: player.health_raw,
        }
    }
}

/// Native-only provenance retained by the slot browser. Loading decodes this
/// into NativeSaveRestore rather than discarding everything but player pose.
#[derive(Debug, Clone)]
pub struct NativeCompatibilityPreview {
    pub logical_level_id: u32,
    pub state_payload: [u8; STATE_PAYLOAD_SIZE],
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaveSource {
    #[default]
    Portable,
    NativeCompatibilityPreview,
}

/// Load-screen classification. Retail distinguishes missing and corrupt rows;
/// `Unsupported` is the port's fail-closed extension for structurally valid
/// saves whose proven state cannot yet be restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveSlotStatus {
    Missing,
    Corrupt,
    Unsupported,
    Valid,
}

#[derive(Debug, Clone)]
enum SaveSlotEntry {
    Missing,
    Corrupt,
    Unsupported(SaveSlot),
    ProbeValidLoadInvalid(SaveSlot),
    Valid(SaveSlot),
}

impl SaveSlotEntry {
    fn status(&self) -> SaveSlotStatus {
        match self {
            Self::Missing => SaveSlotStatus::Missing,
            Self::Corrupt => SaveSlotStatus::Corrupt,
            Self::Unsupported(_) => SaveSlotStatus::Unsupported,
            Self::ProbeValidLoadInvalid(_) | Self::Valid(_) => SaveSlotStatus::Valid,
        }
    }

    fn save(&self) -> Option<&SaveSlot> {
        match self {
            Self::Valid(save) => Some(save),
            _ => None,
        }
    }

    fn display_save(&self) -> Option<&SaveSlot> {
        match self {
            Self::ProbeValidLoadInvalid(save) | Self::Valid(save) => Some(save),
            _ => None,
        }
    }
}

/// Manages save/load operations across all retail slots.
pub struct SaveManager {
    slots: [SaveSlotEntry; NUM_SLOTS],
    paths: SavePathContext,
}

impl SaveManager {
    /// Load the historical data-directory layout used by retained fixtures.
    /// Interactive runtime uses `load_paths` with an executable-bound context.
    pub fn load_all(data_dir: &Path, retail_save_dir: Option<&Path>) -> Self {
        Self::load_paths(SavePathContext::legacy_layout(data_dir, retail_save_dir))
    }

    /// Load writable slots plus read-only legacy/retail imports. Reading never
    /// migrates, deletes, or creates files; explicit saves use the context's
    /// destination regardless of the source from which a row was imported.
    pub fn load_paths(paths: SavePathContext) -> Self {
        let mut slots: [SaveSlotEntry; NUM_SLOTS] = std::array::from_fn(|_| SaveSlotEntry::Missing);
        for (index, entry) in slots.iter_mut().enumerate().take(NUM_SAVE_SLOTS) {
            *entry = load_slot(&paths, index);
        }
        Self { slots, paths }
    }

    /// Save to a specific slot.
    pub fn save_to_slot(&mut self, slot: usize, data: SaveSlot) -> Result<(), String> {
        self.write_portable_slot(slot, data, SaveWriteDisposition::EmptySlotOnly)
    }

    /// Replace an occupied slot after the menu admits retail's write path.
    /// Probe-valid rows require the explicit overwrite dialog; corrupt rows
    /// advance directly to Saving. Imported retail `SlotNN` files are never modified;
    /// the portable JSON copy replaces any port-owned checkpoint and becomes
    /// the preferred load source instead.
    pub fn overwrite_slot(&mut self, slot: usize, data: SaveSlot) -> Result<(), String> {
        self.write_portable_slot(slot, data, SaveWriteDisposition::ConfirmedOverwrite)
    }

    fn write_portable_slot(
        &mut self,
        slot: usize,
        data: SaveSlot,
        disposition: SaveWriteDisposition,
    ) -> Result<(), String> {
        self.validate_write_disposition(slot, disposition)?;
        if !portable_slot_is_supported(&data) {
            return Err("Portable save lacks a supported player snapshot or level".into());
        }

        fs::create_dir_all(&self.paths.save_directory)
            .map_err(|e| format!("Failed to create saves dir: {}", e))?;

        let path = self
            .paths
            .save_directory
            .join(format!("slot_{}.json", slot));
        let json =
            serde_json::to_string_pretty(&data).map_err(|e| format!("Serialize error: {}", e))?;
        let mut options = OpenOptions::new();
        options.write(true);
        match disposition {
            SaveWriteDisposition::EmptySlotOnly => {
                options.create_new(true);
            }
            SaveWriteDisposition::ConfirmedOverwrite => {
                options.create(true).truncate(true);
            }
        }
        let mut file = match options.open(&path) {
            Ok(file) => file,
            Err(error)
                if disposition == SaveWriteDisposition::EmptySlotOnly
                    && error.kind() == ErrorKind::AlreadyExists =>
            {
                self.slots[slot] = SaveSlotEntry::Corrupt;
                return Err(OCCUPIED_SLOT_ERROR.into());
            }
            Err(error) => return Err(format!("Write error: {}", error)),
        };
        if let Err(error) = file.write_all(json.as_bytes()) {
            // Exclusive creation may have left a partial file. Preserve it as
            // evidence and immediately make the row occupied/non-loadable.
            self.slots[slot] = SaveSlotEntry::Corrupt;
            return Err(format!("Write error: {}", error));
        }

        // A confirmed portable replacement supersedes only our own native
        // checkpoint at the writable location. Read-only imports remain intact.
        if disposition == SaveWriteDisposition::ConfirmedOverwrite {
            let native_path = self.paths.save_directory.join(format!("Slot{slot:02}"));
            match fs::remove_file(&native_path) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => {
                    self.slots[slot] = load_native_slot(&native_path, "checkpoint");
                    return Err(format!("Failed to replace native checkpoint: {error}"));
                }
            }
        }
        self.slots[slot] = SaveSlotEntry::Valid(data);
        Ok(())
    }

    fn validate_write_disposition(
        &mut self,
        slot: usize,
        disposition: SaveWriteDisposition,
    ) -> Result<(), String> {
        if slot >= NUM_SLOTS {
            return Err(format!("Invalid slot index: {slot}"));
        }
        if slot == SETTINGS_SLOT {
            return Err("Used for game settings".into());
        }
        match disposition {
            SaveWriteDisposition::EmptySlotOnly => {
                if !matches!(self.slots[slot], SaveSlotEntry::Missing) {
                    return Err(OCCUPIED_SLOT_ERROR.into());
                }
                // A file may have appeared after the menu scan, including a
                // different format for the same row. Do not shadow that save.
                let current = load_slot(&self.paths, slot);
                if !matches!(current, SaveSlotEntry::Missing) {
                    self.slots[slot] = current;
                    return Err(OCCUPIED_SLOT_ERROR.into());
                }
            }
            SaveWriteDisposition::ConfirmedOverwrite
                if matches!(self.slots[slot], SaveSlotEntry::Missing) =>
            {
                return Err(EMPTY_OVERWRITE_ERROR.into());
            }
            _ => {}
        }
        Ok(())
    }

    /// Persist the complete checkpoint prepared by the native save owner.
    /// Runtime writes SlotNN beside the executable, including when replacing
    /// an imported row. Only a portable JSON shadow at that writable location
    /// is retired after success; read-only imports remain untouched. Retained
    /// legacy-layout fixtures preserve their original data/parent placement.
    /// The menu supplies the overwrite decision; 448CF0 owns magic5 and CRCs.
    pub fn write_checkpoint(
        &mut self,
        slot: usize,
        native: &NativeCompatibilityPreview,
        disposition: SaveWriteDisposition,
        tail_seen_mask: u32,
    ) -> Result<(), String> {
        let restore = NativeSaveRestore::decode(native)
            .map_err(|error| format!("Unsupported native checkpoint: {error:?}"))?;
        if native.logical_level_id != restore.logical_level_id {
            return Err("Native checkpoint logical world disagrees with its payload".into());
        }
        self.validate_write_disposition(slot, disposition)?;
        let encoded = encode_native_save_slot(&native.state_payload, 5, tail_seen_mask);
        let row = parse_native_slot(&encoded, "checkpoint");
        if !matches!(row, SaveSlotEntry::Valid(_)) {
            return Err("Native checkpoint did not encode a loadable slot".into());
        }
        let path = self.paths.native_slot_destination(slot);
        fs::create_dir_all(path.parent().expect("native save parent"))
            .map_err(|error| format!("Failed to create save directory: {error}"))?;
        let written = match disposition {
            SaveWriteDisposition::EmptySlotOnly => OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .and_then(|mut file| file.write_all(&encoded).and_then(|()| file.sync_all())),
            SaveWriteDisposition::ConfirmedOverwrite => replace_native_checkpoint(&path, &encoded),
        };
        match written {
            Ok(()) => {}
            Err(error)
                if disposition == SaveWriteDisposition::EmptySlotOnly
                    && error.kind() == ErrorKind::AlreadyExists =>
            {
                self.slots[slot] = load_slot(&self.paths, slot);
                return Err(OCCUPIED_SLOT_ERROR.into());
            }
            Err(error) => {
                self.slots[slot] = load_slot(&self.paths, slot);
                return Err(format!("Native checkpoint write error: {error}"));
            }
        }
        // Retire only shadows owned by the selected writable layout after an
        // explicit successful save. Imported files are never cleanup targets.
        for legacy_path in self.paths.checkpoint_shadow_paths(slot) {
            match fs::remove_file(&legacy_path) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => {
                    self.slots[slot] = load_slot(&self.paths, slot);
                    return Err(format!(
                        "Native checkpoint written, but could not retire {}: {error}",
                        legacy_path.display()
                    ));
                }
            }
        }
        self.slots[slot] = row;
        Ok(())
    }

    /// Write a native 2,048-byte `SlotNN` file using the canonical redundant
    /// CRC record layout matching `FUN_00448CF0`.
    pub fn write_native_slot(
        &mut self,
        target_dir: &Path,
        slot: usize,
        state_payload: &[u8; STATE_PAYLOAD_SIZE],
        magic: u32,
        tail_value: u32,
    ) -> Result<(), String> {
        if slot >= NUM_SAVE_SLOTS {
            return Err(format!("Invalid native save slot index: {}", slot));
        }
        let encoded = encode_native_save_slot(state_payload, magic, tail_value);
        let path = target_dir.join(format!("Slot{:02}", slot));
        fs::create_dir_all(target_dir)
            .map_err(|e| format!("Failed to create save directory: {}", e))?;
        fs::write(&path, &encoded).map_err(|e| format!("Failed to write native save: {}", e))?;
        Ok(())
    }

    /// Get info for a slot.
    pub fn slot(&self, index: usize) -> Option<&SaveSlot> {
        self.slots.get(index).and_then(SaveSlotEntry::save)
    }

    /// Retail-compatible row status plus the port's explicit unsupported
    /// state for snapshots that cannot be restored without approximation.
    pub fn slot_status(&self, index: usize) -> SaveSlotStatus {
        self.slots
            .get(index)
            .map(SaveSlotEntry::status)
            .unwrap_or(SaveSlotStatus::Missing)
    }

    pub fn is_loadable(&self, index: usize) -> bool {
        self.slot(index).is_some()
    }

    /// Get a display label for a slot.
    pub fn slot_label(&self, index: usize) -> String {
        if index == SETTINGS_SLOT {
            return "Used for game settings".into();
        }
        match self.slots.get(index).and_then(SaveSlotEntry::display_save) {
            Some(save) => format!("Slot {}: {}", index + 1, save.level_name),
            None => match self.slots.get(index) {
                Some(SaveSlotEntry::Corrupt) => "Corrupt".into(),
                Some(SaveSlotEntry::Unsupported(save)) => {
                    format!("Unsupported: {}", save.level_name)
                }
                _ => String::new(),
            },
        }
    }

    /// Whether a slot has data.
    pub fn is_occupied(&self, index: usize) -> bool {
        if index == SETTINGS_SLOT {
            return false;
        }
        self.slots
            .get(index)
            .is_some_and(|entry| !matches!(entry, SaveSlotEntry::Missing))
    }

    /// Whether retail state 4's probe-valid row requires "Overwrite game?".
    /// Corrupt state-3 rows are occupied but proceed directly to Saving.
    pub fn requires_overwrite_confirmation(&self, index: usize) -> bool {
        self.slots.get(index).is_some_and(|entry| {
            matches!(
                entry,
                SaveSlotEntry::Unsupported(_)
                    | SaveSlotEntry::ProbeValidLoadInvalid(_)
                    | SaveSlotEntry::Valid(_)
            )
        })
    }
}

fn portable_slot_is_supported(slot: &SaveSlot) -> bool {
    slot.source == SaveSource::Portable
        && (FIRST_GAMEPLAY_OVERLAY_ID..=LAST_GAMEPLAY_OVERLAY_ID).contains(&slot.level_id)
        && slot.player.is_some_and(|player| player.health_raw > 0)
}

fn replace_native_checkpoint(path: &Path, encoded: &[u8]) -> std::io::Result<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
    let (temporary, mut file) = loop {
        let temporary = path.with_extension(format!(
            "{}.{}.tmp",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => break (temporary, file),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };
    let prepared = file.write_all(encoded).and_then(|()| file.sync_all());
    drop(file);
    let result = prepared.and_then(|()| fs::rename(&temporary, path));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn load_slot(paths: &SavePathContext, index: usize) -> SaveSlotEntry {
    if paths.storage_policy == SaveStoragePolicy::BesideExecutable {
        let native = load_native_slot(
            &paths.save_directory.join(format!("Slot{index:02}")),
            "checkpoint",
        );
        if !matches!(native, SaveSlotEntry::Missing) {
            return native;
        }
        let portable = load_portable_slot(&paths.save_directory, index);
        if !matches!(portable, SaveSlotEntry::Missing) {
            return portable;
        }
    }

    // Preserve legacy import precedence and framing, including an occupied
    // corrupt native checkpoint that cannot fall through to an older save.
    let legacy_directory = paths.data_directory.join("saves");
    let checkpoint = load_native_slot(
        &legacy_directory.join(format!("Slot{index:02}")),
        "checkpoint",
    );
    if !matches!(checkpoint, SaveSlotEntry::Missing) {
        return checkpoint;
    }
    let portable = load_portable_slot(&legacy_directory, index);
    if matches!(portable, SaveSlotEntry::Valid(_)) {
        return portable;
    }
    match load_retail_slot(
        &paths.data_directory,
        paths.retail_save_directory.as_deref(),
        index,
    ) {
        SaveSlotEntry::Missing => portable,
        retail => retail,
    }
}

fn load_portable_slot(directory: &Path, index: usize) -> SaveSlotEntry {
    match fs::read(directory.join(format!("slot_{index}.json"))) {
        Ok(contents) => match serde_json::from_slice::<SaveSlot>(&contents) {
            Ok(slot) if portable_slot_is_supported(&slot) => SaveSlotEntry::Valid(slot),
            Ok(slot) => SaveSlotEntry::Unsupported(slot),
            Err(_) => SaveSlotEntry::Corrupt,
        },
        Err(error) if error.kind() == ErrorKind::NotFound => SaveSlotEntry::Missing,
        Err(_) => SaveSlotEntry::Corrupt,
    }
}

fn load_retail_slot(
    data_dir: &Path,
    retail_save_dir: Option<&Path>,
    index: usize,
) -> SaveSlotEntry {
    // A source checkout may run beside the install, with user-owned retail
    // saves one directory above it. A packaged data directory may carry them
    // directly.
    for root in [data_dir.to_path_buf(), data_dir.join("..")]
        .into_iter()
        .chain(retail_save_dir.map(Path::to_path_buf))
    {
        let entry = load_native_slot(&root.join(format!("Slot{index:02}")), "retail");
        if !matches!(entry, SaveSlotEntry::Missing) {
            return entry;
        }
    }
    SaveSlotEntry::Missing
}

fn load_native_slot(path: &Path, timestamp: &str) -> SaveSlotEntry {
    match fs::read(path) {
        Ok(bytes) => parse_native_slot(&bytes, timestamp),
        Err(error) if error.kind() == ErrorKind::NotFound => SaveSlotEntry::Missing,
        Err(_) => SaveSlotEntry::Corrupt,
    }
}

fn parse_native_slot(bytes: &[u8], timestamp: &str) -> SaveSlotEntry {
    let retail = match RetailSaveSlot::parse(bytes) {
        Ok(retail) => retail,
        Err(_) => return SaveSlotEntry::Corrupt,
    };
    if !retail.menu_probe_is_valid() {
        return SaveSlotEntry::Corrupt;
    }
    let full_load_is_valid = retail.full_load_is_valid();
    let level_id = retail.game_state.saveable_global_level_id();
    let save = SaveSlot {
        level_id: level_id.unwrap_or(retail.game_state.logical_level_id),
        level_name: retail.game_state.display_name,
        timestamp: timestamp.into(),
        player: Some(retail.game_state.player.into()),
        source: SaveSource::NativeCompatibilityPreview,
        native: Some(NativeCompatibilityPreview {
            logical_level_id: retail.game_state.logical_level_id,
            state_payload: retail.state_payload,
        }),
    };
    if level_id.is_none() {
        return SaveSlotEntry::Unsupported(save);
    }
    if !full_load_is_valid {
        return SaveSlotEntry::ProbeValidLoadInvalid(save);
    }
    if NativeSaveRestore::decode(save.native.as_ref().expect("native slot payload")).is_err() {
        return SaveSlotEntry::Unsupported(save);
    }
    SaveSlotEntry::Valid(save)
}

#[cfg(test)]
mod checkpoint_tests;

#[cfg(test)]
mod retail_path_tests;

#[cfg(test)]
mod portable_path_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn empty_slots() {
        let dir = env::temp_dir().join("v2k_save_test_empty");
        let _ = fs::remove_dir_all(&dir);
        let mgr = SaveManager::load_all(&dir, None);
        assert!(mgr.slot(0).is_none());
        assert!(mgr.slot(1).is_none());
        assert!(mgr.slot(2).is_none());
        assert!(mgr.slot(NUM_SLOTS - 1).is_none());
        assert_eq!(mgr.slot_label(0), "");
        assert_eq!(mgr.slot_label(NUM_SLOTS - 1), "Used for game settings");
    }

    #[test]
    fn save_and_load() {
        let dir = env::temp_dir().join("v2k_save_test_roundtrip");
        let _ = fs::remove_dir_all(&dir);
        let mut portable_player = test_player();
        portable_player.pitch_raw = (-0x1234_i16) as u16;
        portable_player.roll_raw = 0x2345;

        let mut mgr = SaveManager::load_all(&dir, None);
        mgr.save_to_slot(
            1,
            SaveSlot {
                level_id: 14,
                level_name: "Mediaeval".into(),
                timestamp: "2026-03-15T12:00:00".into(),
                player: Some(portable_player),
                source: SaveSource::Portable,
                native: None,
            },
        )
        .unwrap();

        assert_eq!(mgr.slot_label(0), "");
        assert_eq!(mgr.slot_label(1), "Slot 2: Mediaeval");
        assert!(mgr.is_occupied(1));
        assert!(!mgr.is_occupied(0));

        // Reload from disk
        let mgr2 = SaveManager::load_all(&dir, None);
        assert_eq!(mgr2.slot(1).unwrap().level_id, 14);
        assert_eq!(mgr2.slot(1).unwrap().player, Some(portable_player));
        assert_eq!(mgr2.slot_label(1), "Slot 2: Mediaeval");

        let overwrite_error = mgr
            .save_to_slot(
                1,
                SaveSlot {
                    level_id: 15,
                    level_name: "Castle".into(),
                    timestamp: "later".into(),
                    player: Some(test_player()),
                    source: SaveSource::Portable,
                    native: None,
                },
            )
            .unwrap_err();
        assert_eq!(
            overwrite_error,
            "Slot is occupied; overwrite confirmation required"
        );

        mgr.overwrite_slot(
            1,
            SaveSlot {
                level_id: 15,
                level_name: "Castle".into(),
                timestamp: "confirmed".into(),
                player: Some(test_player()),
                source: SaveSource::Portable,
                native: None,
            },
        )
        .unwrap();
        assert_eq!(mgr.slot_label(1), "Slot 2: Castle");
        let overwritten = SaveManager::load_all(&dir, None);
        assert_eq!(overwritten.slot(1).unwrap().level_id, 15);
        assert_eq!(overwritten.slot(1).unwrap().level_name, "Castle");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn confirmed_overwrite_preserves_native_slot_evidence() {
        let dir = env::temp_dir().join("v2k_save_test_native_overwrite_preservation");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let native = make_retail_slot("Retail Amazon", 18);
        let native_path = dir.join("Slot00");
        fs::write(&native_path, &native).unwrap();

        let mut mgr = SaveManager::load_all(&dir, None);
        assert!(mgr.is_occupied(0));
        mgr.overwrite_slot(0, portable_slot("Portable Castle", 15))
            .unwrap();

        assert_eq!(fs::read(&native_path).unwrap(), native);
        let reloaded = SaveManager::load_all(&dir, None);
        let replacement = reloaded.slot(0).unwrap();
        assert_eq!(replacement.level_name, "Portable Castle");
        assert_eq!(replacement.source, SaveSource::Portable);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn highest_playable_retail_slot_round_trips_without_renaming_existing_slots() {
        let dir = env::temp_dir().join("v2k_save_test_slot_15");
        let _ = fs::remove_dir_all(&dir);

        let mut mgr = SaveManager::load_all(&dir, None);
        mgr.save_to_slot(
            NUM_SAVE_SLOTS - 1,
            SaveSlot {
                level_id: 20,
                level_name: "Last Slot".into(),
                timestamp: "2026-07-17T04:00:00".into(),
                player: Some(test_player()),
                source: SaveSource::Portable,
                native: None,
            },
        )
        .unwrap();

        // The additive expansion retains the established zero-based filename
        // scheme: old slot_0..slot_2 saves remain exactly where they were.
        assert!(dir
            .join("saves")
            .join(format!("slot_{}.json", NUM_SAVE_SLOTS - 1))
            .is_file());
        let reloaded = SaveManager::load_all(&dir, None);
        assert_eq!(
            reloaded.slot(NUM_SAVE_SLOTS - 1).unwrap().level_name,
            "Last Slot"
        );

        let error = mgr
            .save_to_slot(
                SETTINGS_SLOT,
                SaveSlot {
                    level_id: 1,
                    level_name: "Reserved".into(),
                    timestamp: "never".into(),
                    player: None,
                    source: SaveSource::Portable,
                    native: None,
                },
            )
            .unwrap_err();
        assert_eq!(error, "Used for game settings");

        let _ = fs::remove_dir_all(&dir);
    }

    fn make_retail_slot(name: &str, level_id: u32) -> Vec<u8> {
        const SIZE: usize = 0x800;
        const STATE_A: usize = 0x200;
        const STATE_B: usize = 0x480;
        const STATE_SIZE: usize = 0x248;

        fn write_record(bytes: &mut [u8], offset: usize, payload: &[u8]) {
            bytes[offset..offset + payload.len()].copy_from_slice(payload);
            let crc = v2k_formats::saves::retail_crc32(payload);
            bytes[offset + payload.len()..offset + payload.len() + 4]
                .copy_from_slice(&crc.to_le_bytes());
        }

        let mut bytes = vec![0; SIZE];
        let header = 5_u32.to_le_bytes();
        for copy in 0..4 {
            write_record(&mut bytes, copy * 0x80, &header);
        }
        let mut state = vec![0_u8; STATE_SIZE];
        let name = name.as_bytes();
        let len = name.len().min(31);
        state[..len].copy_from_slice(&name[..len]);
        state[0x20..0x24].copy_from_slice(&level_id.to_le_bytes());
        for (offset, value) in [(0x24, 7424_i16), (0x26, 5120), (0x28, 9728)] {
            state[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
        }
        state[0x30..0x32].copy_from_slice(&0x4000_u16.to_le_bytes());
        state[0x3c..0x40].copy_from_slice(&40_000_i32.to_le_bytes());
        state[0x40..0x44].copy_from_slice(&1_u32.to_le_bytes());
        state[0x140..0x144].copy_from_slice(&46_u32.to_le_bytes());
        write_record(&mut bytes, STATE_A, &state);
        write_record(&mut bytes, STATE_B, &state);
        let tail = 0x0020_1000_u32.to_le_bytes();
        write_record(&mut bytes, 0x700, &tail);
        write_record(&mut bytes, 0x780, &tail);
        bytes
    }

    #[test]
    fn imports_retail_slot_from_parent_data_directory() {
        let root = env::temp_dir().join("v2k_save_test_retail_import");
        let data_dir = root.join("port");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&data_dir).unwrap();
        fs::write(root.join("Slot03"), make_retail_slot("Cistern", 18)).unwrap();

        let mgr = SaveManager::load_all(&data_dir, None);
        let slot = mgr.slot(3).expect("retail slot should be imported");
        assert_eq!(slot.level_name, "Cistern");
        assert_eq!(slot.level_id, 30);
        assert_eq!(slot.timestamp, "retail");
        assert_eq!(slot.source, SaveSource::NativeCompatibilityPreview);
        assert_eq!(slot.player, Some(test_player()));
        let native = slot.native.as_ref().expect("native evidence retained");
        assert_eq!(native.logical_level_id, 18);
        assert_eq!(native.state_payload.len(), STATE_PAYLOAD_SIZE);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn legacy_level_only_json_cannot_shadow_a_supported_native_slot() {
        let root = env::temp_dir().join("v2k_save_test_native_over_legacy_json");
        let data_dir = root.join("port");
        let save_dir = data_dir.join("saves");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&save_dir).unwrap();
        fs::write(root.join("Slot03"), make_retail_slot("Cistern", 18)).unwrap();
        fs::write(
            save_dir.join("slot_3.json"),
            r#"{
  "level_id": 18,
  "level_name": "Level 18",
  "timestamp": "legacy",
  "source": "portable"
}"#,
        )
        .unwrap();

        let mgr = SaveManager::load_all(&data_dir, None);
        let slot = mgr.slot(3).expect("native slot should remain loadable");
        assert_eq!(slot.source, SaveSource::NativeCompatibilityPreview);
        assert_eq!(slot.level_name, "Cistern");
        assert_eq!(slot.level_id, 30);
        assert_eq!(mgr.slot_status(3), SaveSlotStatus::Valid);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn legacy_level_only_json_without_native_evidence_is_unsupported() {
        let root = env::temp_dir().join("v2k_save_test_legacy_json_unsupported");
        let save_dir = root.join("saves");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&save_dir).unwrap();
        fs::write(
            save_dir.join("slot_0.json"),
            r#"{"level_id":18,"level_name":"Level 18","timestamp":"legacy"}"#,
        )
        .unwrap();

        let mgr = SaveManager::load_all(&root, None);
        assert_eq!(mgr.slot_status(0), SaveSlotStatus::Unsupported);
        assert!(!mgr.is_loadable(0));
        assert!(mgr.is_occupied(0));
        assert_eq!(mgr.slot_label(0), "Unsupported: Level 18");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn portable_player_snapshot_without_attitude_fails_closed() {
        let root = env::temp_dir().join("v2k_save_test_portable_missing_attitude");
        let save_dir = root.join("saves");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&save_dir).unwrap();
        fs::write(
            save_dir.join("slot_0.json"),
            r#"{
  "level_id": 18,
  "level_name": "Incomplete",
  "timestamp": "legacy",
  "player": {
    "position_raw": [1, 2, 3],
    "velocity_raw": [0, 0, 0],
    "heading_raw": 16384,
    "health_raw": 40000
  },
  "source": "portable"
}"#,
        )
        .unwrap();

        let mgr = SaveManager::load_all(&root, None);
        assert_eq!(mgr.slot_status(0), SaveSlotStatus::Corrupt);
        assert!(!mgr.is_loadable(0));
        assert!(mgr.is_occupied(0));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn crc_invalid_native_file_is_corrupt_and_never_loadable() {
        let root = env::temp_dir().join("v2k_save_test_corrupt_native");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let mut bytes = make_retail_slot("Cistern", 18);
        bytes[0x448] ^= 0xff;
        bytes[0x6c8] ^= 0xff;
        fs::write(root.join("Slot00"), bytes).unwrap();

        let mgr = SaveManager::load_all(&root, None);
        assert_eq!(mgr.slot_status(0), SaveSlotStatus::Corrupt);
        assert!(!mgr.is_loadable(0));
        assert_eq!(mgr.slot_label(0), "Corrupt");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn tail_invalid_native_row_keeps_probe_label_but_cannot_load() {
        let root = env::temp_dir().join("v2k_save_test_tail_invalid_native");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let mut bytes = make_retail_slot("Cistern", 18);
        bytes[0x704] ^= 0xff;
        bytes[0x784] ^= 0xff;
        fs::write(root.join("Slot00"), bytes).unwrap();

        let mut mgr = SaveManager::load_all(&root, None);
        assert_eq!(mgr.slot_status(0), SaveSlotStatus::Valid);
        assert_eq!(mgr.slot_label(0), "Slot 1: Cistern");
        assert!(mgr.is_occupied(0));
        assert!(!mgr.is_loadable(0));
        assert!(mgr.slot(0).is_none());
        assert_eq!(
            mgr.save_to_slot(0, portable_slot("Replacement", 18))
                .unwrap_err(),
            OCCUPIED_SLOT_ERROR
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn invalid_utf8_portable_file_is_corrupt_occupied_and_preserved() {
        let root = env::temp_dir().join("v2k_save_test_invalid_utf8_portable");
        let save_dir = root.join("saves");
        let path = save_dir.join("slot_0.json");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&save_dir).unwrap();
        let invalid_utf8 = [0xff, 0xfe, 0xfd];
        fs::write(&path, invalid_utf8).unwrap();

        let mut mgr = SaveManager::load_all(&root, None);
        assert_eq!(mgr.slot_status(0), SaveSlotStatus::Corrupt);
        assert_eq!(mgr.slot_label(0), "Corrupt");
        assert!(mgr.is_occupied(0));
        assert!(!mgr.is_loadable(0));
        assert_eq!(
            mgr.save_to_slot(0, portable_slot("Replacement", 18))
                .unwrap_err(),
            OCCUPIED_SLOT_ERROR
        );
        assert_eq!(fs::read(&path).unwrap(), invalid_utf8);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn non_not_found_portable_and_native_read_errors_are_corrupt() {
        let root = env::temp_dir().join("v2k_save_test_read_error_classification");
        let data_dir = root.join("port");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(data_dir.join("saves").join("slot_0.json")).unwrap();
        fs::create_dir_all(data_dir.join("Slot01")).unwrap();

        let mgr = SaveManager::load_all(&data_dir, None);
        for index in [0, 1] {
            assert_eq!(mgr.slot_status(index), SaveSlotStatus::Corrupt);
            assert_eq!(mgr.slot_label(index), "Corrupt");
            assert!(mgr.is_occupied(index));
            assert!(!mgr.is_loadable(index));
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn corrupt_direct_native_slot_is_not_bypassed_by_valid_parent_copy() {
        let root = env::temp_dir().join("v2k_save_test_direct_native_precedence");
        let data_dir = root.join("port");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&data_dir).unwrap();
        let mut direct = make_retail_slot("Broken direct", 18);
        direct[0x448] ^= 0xff;
        direct[0x6c8] ^= 0xff;
        fs::write(data_dir.join("Slot00"), direct).unwrap();
        fs::write(root.join("Slot00"), make_retail_slot("Parent", 18)).unwrap();

        let mgr = SaveManager::load_all(&data_dir, None);
        assert_eq!(mgr.slot_status(0), SaveSlotStatus::Corrupt);
        assert_eq!(mgr.slot_label(0), "Corrupt");
        assert!(!mgr.is_loadable(0));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn exclusive_portable_create_preserves_file_that_appeared_after_scan() {
        let root = env::temp_dir().join("v2k_save_test_exclusive_create");
        let path = root.join("saves").join("slot_0.json");
        let _ = fs::remove_dir_all(&root);
        let mut mgr = SaveManager::load_all(&root, None);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let existing = b"appeared after load_all";
        fs::write(&path, existing).unwrap();

        assert_eq!(
            mgr.save_to_slot(0, portable_slot("Replacement", 18))
                .unwrap_err(),
            OCCUPIED_SLOT_ERROR
        );
        assert!(mgr.is_occupied(0));
        assert_eq!(mgr.slot_status(0), SaveSlotStatus::Corrupt);
        assert_eq!(fs::read(&path).unwrap(), existing);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn cinematic_overlays_are_not_loadable_as_saved_gameplay() {
        let root = env::temp_dir().join("v2k_save_test_cinematic_rejection");
        let save_dir = root.join("saves");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&save_dir).unwrap();
        fs::write(root.join("Slot00"), make_retail_slot("Intro1", 37)).unwrap();
        fs::write(
            save_dir.join("slot_1.json"),
            serde_json::to_string(&SaveSlot {
                level_id: 50,
                level_name: "Intro2".into(),
                timestamp: "test".into(),
                player: Some(test_player()),
                source: SaveSource::Portable,
                native: None,
            })
            .unwrap(),
        )
        .unwrap();

        let mgr = SaveManager::load_all(&root, None);
        assert_eq!(mgr.slot_status(0), SaveSlotStatus::Unsupported);
        assert_eq!(mgr.slot_status(1), SaveSlotStatus::Unsupported);
        assert!(!mgr.is_loadable(0));
        assert!(!mgr.is_loadable(1));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn write_native_slot_creates_parseable_slot_file() {
        let root = env::temp_dir().join("v2k_save_test_write_native");
        let _ = fs::remove_dir_all(&root);
        let mut mgr = SaveManager::load_all(&root, None);

        let mut state_payload = [0_u8; STATE_PAYLOAD_SIZE];
        let name = b"Peasant Level";
        // A runnable native controller owns its player type and at least the
        // selected weapon; the importer does not invent these from zero data.
        state_payload[0x40..0x44].copy_from_slice(&1_u32.to_le_bytes());
        state_payload[0x140..0x144].copy_from_slice(&46_u32.to_le_bytes());
        state_payload[..name.len()].copy_from_slice(name);
        state_payload[0x20..0x24].copy_from_slice(&1_u32.to_le_bytes()); // logical level 1 -> global 13
        state_payload[0x3c..0x40].copy_from_slice(&40_000_i32.to_le_bytes()); // health 40000

        mgr.write_native_slot(&root, 0, &state_payload, 5, 0x0020_1000)
            .unwrap();

        let slot_path = root.join("Slot00");
        assert!(slot_path.exists());
        let slot_bytes = fs::read(&slot_path).unwrap();
        assert_eq!(slot_bytes.len(), 2048);

        let parsed = RetailSaveSlot::parse(&slot_bytes).unwrap();
        assert_eq!(parsed.magic, 5);
        assert_eq!(parsed.game_state.display_name, "Peasant Level");
        assert_eq!(parsed.game_state.logical_level_id, 1);
        assert_eq!(parsed.game_state.global_level_id(), Some(13));
        assert_eq!(parsed.game_state.player.health_raw, 40_000);
        assert!(parsed.is_valid());

        // Reload manager from root
        let loaded_mgr = SaveManager::load_all(&root, None);
        assert!(loaded_mgr.is_occupied(0));
        assert!(loaded_mgr.is_loadable(0));
        assert_eq!(loaded_mgr.slot(0).unwrap().level_id, 13);
        assert_eq!(loaded_mgr.slot_label(0), "Slot 1: Peasant Level");

        let _ = fs::remove_dir_all(&root);
    }

    fn test_player() -> SavedPlayerState {
        SavedPlayerState {
            position_raw: [7424, 5120, 9728],
            velocity_raw: [0; 3],
            heading_raw: 0x4000,
            pitch_raw: 0,
            roll_raw: 0,
            health_raw: 40_000,
        }
    }

    fn portable_slot(level_name: &str, level_id: u32) -> SaveSlot {
        SaveSlot {
            level_id,
            level_name: level_name.into(),
            timestamp: "test".into(),
            player: Some(test_player()),
            source: SaveSource::Portable,
            native: None,
        }
    }
}
