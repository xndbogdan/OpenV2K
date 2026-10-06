use super::checkpoint_tests::{native, portable};
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture {
    root: PathBuf,
    executable_directory: PathBuf,
    data: PathBuf,
    retail: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "v2k-executable-saves-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let executable_directory = root.join("executable");
        let data = root.join("imports").join("data");
        let retail = root.join("retail");
        for directory in [&executable_directory, &data, &retail] {
            fs::create_dir_all(directory).unwrap();
        }
        Self {
            root,
            executable_directory,
            data,
            retail,
        }
    }

    fn paths(&self) -> SavePathContext {
        SavePathContext::beside_executable(
            &self.executable_directory.join("v2k-game.exe"),
            &self.data,
            Some(&self.retail),
        )
        .unwrap()
    }

    fn write_native(&self, directory: &Path, slot: usize) -> Vec<u8> {
        fs::create_dir_all(directory).unwrap();
        let bytes = encode_native_save_slot(&native().state_payload, 5, 0x1234).to_vec();
        fs::write(directory.join(format!("Slot{slot:02}")), &bytes).unwrap();
        bytes
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn native_and_portable_writes_use_executable_directory_despite_data_and_registry_sets() {
    let fixture = Fixture::new();
    fixture.write_native(&fixture.data, 7);
    fixture.write_native(fixture.data.parent().unwrap(), 8);
    fixture.write_native(&fixture.retail, 9);
    let paths = fixture.paths();
    assert_eq!(paths.save_directory(), fixture.executable_directory);
    assert_ne!(paths.save_directory(), std::env::current_dir().unwrap());
    let mut manager = SaveManager::load_paths(paths);
    for slot in [7, 8, 9] {
        assert!(manager.is_loadable(slot));
    }
    assert!(!fixture.executable_directory.join("Slot00").exists());
    manager.save_to_slot(0, portable()).unwrap();
    manager
        .write_checkpoint(1, &native(), SaveWriteDisposition::EmptySlotOnly, 1)
        .unwrap();
    assert!(fixture.executable_directory.join("slot_0.json").is_file());
    assert!(fixture.executable_directory.join("Slot01").is_file());
    assert!(!fixture.executable_directory.join("saves").exists());
    assert!(!fixture.data.join("saves").exists());
    for directory in [
        &fixture.data,
        fixture.data.parent().unwrap(),
        &fixture.retail,
    ] {
        assert!(!directory.join("Slot01").exists());
        assert!(!directory.join("slot_0.json").exists());
    }
    let restarted = SaveManager::load_paths(fixture.paths());
    assert_eq!(restarted.slot(0).unwrap().source, SaveSource::Portable);
    assert_eq!(
        restarted.slot(1).unwrap().source,
        SaveSource::NativeCompatibilityPreview
    );
}

#[test]
fn imported_sources_remain_unchanged_when_explicit_save_creates_executable_checkpoint() {
    for source in ["legacy_native", "legacy_json", "data", "parent", "registry"] {
        let fixture = Fixture::new();
        let directory = match source {
            "legacy_native" | "legacy_json" => fixture.data.join("saves"),
            "data" => fixture.data.clone(),
            "parent" => fixture.data.parent().unwrap().to_path_buf(),
            "registry" => fixture.retail.clone(),
            _ => unreachable!(),
        };
        fs::create_dir_all(&directory).unwrap();
        let filename = if source == "legacy_json" {
            "slot_0.json"
        } else {
            "Slot00"
        };
        let imported = if source == "legacy_json" {
            serde_json::to_vec(&portable()).unwrap()
        } else {
            fixture.write_native(&directory, 0)
        };
        fs::write(directory.join(filename), &imported).unwrap();
        let mut manager = SaveManager::load_paths(fixture.paths());
        assert!(manager.is_loadable(0), "{source}");
        assert!(!fixture.executable_directory.join("Slot00").exists());
        let mut replacement = native();
        replacement.state_payload[..32].fill(0);
        replacement.state_payload[..7].copy_from_slice(b"Updated");
        manager
            .write_checkpoint(
                0,
                &replacement,
                SaveWriteDisposition::ConfirmedOverwrite,
                0x7654_3210,
            )
            .unwrap();
        assert_eq!(
            fs::read(directory.join(filename)).unwrap(),
            imported,
            "{source}"
        );
        let checkpoint = fs::read(fixture.executable_directory.join("Slot00")).unwrap();
        let parsed = RetailSaveSlot::parse(&checkpoint).unwrap();
        assert!(parsed.full_load_is_valid());
        assert_eq!(parsed.state_payload, replacement.state_payload);
        assert_eq!(parsed.tail_value, 0x7654_3210);
        let restarted = SaveManager::load_paths(fixture.paths());
        assert_eq!(restarted.slot(0).unwrap().level_name, "Updated", "{source}");
    }
}

#[test]
fn corrupt_or_unsupported_executable_slot_cannot_fall_through_to_older_import() {
    let mut unsupported = portable();
    unsupported.player = None;
    for (filename, bytes, expected_status) in [
        (
            "Slot00",
            b"incomplete save".to_vec(),
            SaveSlotStatus::Corrupt,
        ),
        (
            "slot_0.json",
            b"incomplete save".to_vec(),
            SaveSlotStatus::Corrupt,
        ),
        (
            "slot_0.json",
            serde_json::to_vec(&unsupported).unwrap(),
            SaveSlotStatus::Unsupported,
        ),
    ] {
        let fixture = Fixture::new();
        fixture.write_native(&fixture.data.join("saves"), 0);
        fs::write(fixture.executable_directory.join(filename), &bytes).unwrap();
        let mut manager = SaveManager::load_paths(fixture.paths());
        assert!(manager.is_occupied(0));
        assert!(!manager.is_loadable(0));
        assert_eq!(manager.slot_status(0), expected_status);
        assert_eq!(
            manager
                .write_checkpoint(0, &native(), SaveWriteDisposition::EmptySlotOnly, 0,)
                .unwrap_err(),
            OCCUPIED_SLOT_ERROR
        );
        assert_eq!(
            fs::read(fixture.executable_directory.join(filename)).unwrap(),
            bytes
        );
    }
}

#[test]
fn late_registry_import_is_rechecked_before_creating_an_empty_executable_slot() {
    let fixture = Fixture::new();
    let mut manager = SaveManager::load_paths(fixture.paths());
    assert!(!manager.is_occupied(3));
    let original = fixture.write_native(&fixture.retail, 3);
    assert_eq!(
        manager.save_to_slot(3, portable()).unwrap_err(),
        OCCUPIED_SLOT_ERROR
    );
    assert!(manager.is_loadable(3));
    assert!(!fixture.executable_directory.join("slot_3.json").exists());
    assert!(!fixture.executable_directory.join("Slot03").exists());
    assert_eq!(fs::read(fixture.retail.join("Slot03")).unwrap(), original);
}

#[test]
fn explicit_format_replacement_retires_only_executable_owned_shadows() {
    let fixture = Fixture::new();
    let mut manager = SaveManager::load_paths(fixture.paths());
    manager
        .write_checkpoint(0, &native(), SaveWriteDisposition::EmptySlotOnly, 1)
        .unwrap();
    let legacy_directory = fixture.data.join("saves");
    let original = fixture.write_native(&legacy_directory, 0);
    let json = serde_json::to_vec(&portable()).unwrap();
    fs::write(legacy_directory.join("slot_0.json"), &json).unwrap();
    manager.overwrite_slot(0, portable()).unwrap();
    assert!(!fixture.executable_directory.join("Slot00").exists());
    assert_eq!(
        SaveManager::load_paths(fixture.paths())
            .slot(0)
            .unwrap()
            .source,
        SaveSource::Portable
    );
    manager
        .write_checkpoint(0, &native(), SaveWriteDisposition::ConfirmedOverwrite, 2)
        .unwrap();
    assert!(!fixture.executable_directory.join("slot_0.json").exists());
    assert_eq!(fs::read(legacy_directory.join("Slot00")).unwrap(), original);
    assert_eq!(
        fs::read(legacy_directory.join("slot_0.json")).unwrap(),
        json
    );
    assert_eq!(
        SaveManager::load_paths(fixture.paths())
            .slot(0)
            .unwrap()
            .source,
        SaveSource::NativeCompatibilityPreview
    );
}

#[test]
fn blocked_executable_destination_never_redirects_writes_to_imports() {
    let fixture = Fixture::new();
    let original = fixture.write_native(&fixture.retail, 0);
    fs::create_dir(fixture.executable_directory.join("Slot00")).unwrap();
    let mut manager = SaveManager::load_paths(fixture.paths());
    assert_eq!(manager.slot_status(0), SaveSlotStatus::Corrupt);
    assert!(manager
        .write_checkpoint(0, &native(), SaveWriteDisposition::ConfirmedOverwrite, 2)
        .is_err());
    assert!(fixture.executable_directory.join("Slot00").is_dir());
    assert!(!fixture.data.join("Slot00").exists());
    assert_eq!(fs::read(fixture.retail.join("Slot00")).unwrap(), original);
    let restarted = SaveManager::load_paths(fixture.paths());
    assert_eq!(manager.slot_status(0), restarted.slot_status(0));
}

#[test]
fn relative_executable_is_rejected_instead_of_using_the_working_directory() {
    let fixture = Fixture::new();
    for executable in [
        Path::new("v2k-game.exe"),
        Path::new("relative/v2k-game.exe"),
    ] {
        assert!(SavePathContext::beside_executable(
            executable,
            &fixture.data,
            Some(&fixture.retail)
        )
        .is_err());
    }
}
