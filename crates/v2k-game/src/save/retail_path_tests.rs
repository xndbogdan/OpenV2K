use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture {
    root: PathBuf,
    data: PathBuf,
    original: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "v2k-retail-save-path-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let data = root.join("port").join("data");
        let original = root.join("original");
        fs::create_dir_all(&data).unwrap();
        fs::create_dir_all(&original).unwrap();
        Self {
            root,
            data,
            original,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn checkpoint(name: &str, logical: u32) -> NativeCompatibilityPreview {
    let mut state_payload = [0; STATE_PAYLOAD_SIZE];
    state_payload[..name.len()].copy_from_slice(name.as_bytes());
    for (offset, value) in [
        (0x20, logical),
        (0x3c, 40_000),
        (0x40, 1),
        (0x140, 46),
        (0x1b0, 0x89123456),
    ] {
        state_payload[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    state_payload[0x145] = 0xab;
    NativeCompatibilityPreview {
        logical_level_id: logical,
        state_payload,
        saved_hint_mask: None,
    }
}

fn write_original(
    fixture: &Fixture,
    slot: usize,
    native: &NativeCompatibilityPreview,
) -> [u8; v2k_formats::saves::NATIVE_SLOT_SIZE] {
    let bytes = encode_native_save_slot(&native.state_payload, 5, 0x76543210);
    fs::write(fixture.original.join(format!("Slot{slot:02}")), &bytes).unwrap();
    bytes
}

#[test]
fn registry_selected_native_directory_is_import_only_and_opaque_bytes_survive() {
    let fixture = Fixture::new();
    let native = checkpoint("Original Mediaeval", 2);
    let original = write_original(&fixture, 0, &native);
    assert!(!SaveManager::load_all(&fixture.data, None).is_occupied(0));
    let mut manager = SaveManager::load_all(&fixture.data, Some(&fixture.original));
    let loaded = manager.slot(0).unwrap();
    assert_eq!(loaded.level_id, 14);
    assert_eq!(
        loaded.native.as_ref().unwrap().state_payload,
        native.state_payload
    );
    assert_eq!(
        manager
            .write_checkpoint(0, &native, SaveWriteDisposition::EmptySlotOnly, 0)
            .unwrap_err(),
        OCCUPIED_SLOT_ERROR
    );
    let replacement = checkpoint("Port Castle", 3);
    manager
        .write_checkpoint(
            0,
            &replacement,
            SaveWriteDisposition::ConfirmedOverwrite,
            0x12345678,
        )
        .unwrap();
    assert_eq!(fs::read(fixture.original.join("Slot00")).unwrap(), original);
    let reloaded = SaveManager::load_all(&fixture.data, Some(&fixture.original));
    assert_eq!(reloaded.slot(0).unwrap().level_name, "Port Castle");
    assert_eq!(
        RetailSaveSlot::parse(&fs::read(fixture.data.join("Slot00")).unwrap())
            .unwrap()
            .tail_value,
        0x12345678
    );
    assert!(!fixture.data.join("saves").exists());
    assert!(!fixture.data.parent().unwrap().join("Slot00").exists());
}

#[test]
fn save_path_is_last_fallback_and_late_files_are_rechecked_before_writing() {
    let fixture = Fixture::new();
    let native = checkpoint("Registry", 2);
    write_original(&fixture, 0, &native);
    let direct = checkpoint("Data", 3);
    fs::write(
        fixture.data.join("Slot00"),
        encode_native_save_slot(&direct.state_payload, 5, 0),
    )
    .unwrap();
    let mut manager = SaveManager::load_all(&fixture.data, Some(&fixture.original));
    assert_eq!(manager.slot(0).unwrap().level_name, "Data");
    assert!(!manager.is_occupied(1));
    let late = write_original(&fixture, 1, &native);
    assert_eq!(
        manager
            .write_checkpoint(1, &direct, SaveWriteDisposition::EmptySlotOnly, 0)
            .unwrap_err(),
        OCCUPIED_SLOT_ERROR
    );
    assert_eq!(manager.slot(1).unwrap().level_name, "Registry");
    assert!(!fixture.data.join("Slot01").exists());
    assert!(!fixture.data.parent().unwrap().join("Slot01").exists());
    assert!(!fixture.data.join("saves/Slot01").exists());
    assert_eq!(fs::read(fixture.original.join("Slot01")).unwrap(), late);
}

#[test]
fn corrupt_port_checkpoint_cannot_fall_through_to_registry_selected_save() {
    let fixture = Fixture::new();
    write_original(&fixture, 0, &checkpoint("Registry", 2));
    fs::create_dir_all(fixture.data.join("saves")).unwrap();
    fs::write(
        fixture.data.join("saves/Slot00"),
        b"retained damaged checkpoint",
    )
    .unwrap();
    let manager = SaveManager::load_all(&fixture.data, Some(&fixture.original));
    assert_eq!(manager.slot_status(0), SaveSlotStatus::Corrupt);
    assert!(!manager.is_loadable(0));
}

#[test]
fn empty_registry_save_path_does_not_redirect_a_new_local_slot() {
    let fixture = Fixture::new();
    let snapshot = checkpoint("Local Castle", 3);
    let mut manager = SaveManager::load_all(&fixture.data, Some(&fixture.original));
    manager
        .write_checkpoint(4, &snapshot, SaveWriteDisposition::EmptySlotOnly, 42)
        .unwrap();
    assert!(fixture.data.join("Slot04").is_file());
    assert!(!fixture.data.parent().unwrap().join("Slot04").exists());
    assert!(!fixture.original.join("Slot04").exists());
    assert!(!fixture.data.join("saves").exists());
    let restarted = SaveManager::load_all(&fixture.data, Some(&fixture.original));
    assert_eq!(restarted.slot(4).unwrap().level_name, "Local Castle");
}
