use super::*;

fn directory(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("v2k_checkpoint_{name}"));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

pub(super) fn native() -> NativeCompatibilityPreview {
    let mut state_payload = [0; STATE_PAYLOAD_SIZE];
    state_payload[..10].copy_from_slice(b"Checkpoint");
    for (offset, value) in [
        (0x20, 3_u32),
        (0x38, 199_123),
        (0x3c, 40_000),
        (0x40, 1),
        (0x48, 2),
        (0x4c, 123),
        (0x140, 46),
        (0x14c, 0x0401_0044),
        (0x150, 0x8000_0000),
        (0x154, 0x0402_0044),
        (0x16c, 80_833),
        (0x170, 49),
        (0x1b0, 0x29),
        (0x1bc, 0x208),
    ] {
        state_payload[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    for (offset, value) in [(0x24, -32_000_i16), (0x2a, i16::MIN), (0x32, -1234)] {
        state_payload[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }
    state_payload[0x36] = 1;
    state_payload[0x144] = 7;
    state_payload[0x145] = 0x56;
    state_payload[0x146] = 1;
    state_payload[0x147] = 3;
    state_payload[0x149] = 5;
    state_payload[0x14a] = 19;
    NativeCompatibilityPreview {
        logical_level_id: 3,
        state_payload,
    }
}

pub(super) fn portable() -> SaveSlot {
    SaveSlot {
        level_id: 14,
        level_name: "Portable".into(),
        timestamp: "test".into(),
        player: Some(NativeSaveRestore::decode(&native()).unwrap().player),
        source: SaveSource::Portable,
        native: None,
    }
}

#[test]
fn checkpoint_roundtrip_replaces_direct_native_and_preserves_parent_copy() {
    let root = directory("roundtrip");
    let data = root.join("port");
    fs::create_dir_all(&data).unwrap();
    let snapshot = native();
    let retail_bytes = encode_native_save_slot(&snapshot.state_payload, 5, 0x1111);
    fs::write(root.join("Slot00"), retail_bytes).unwrap();
    fs::write(data.join("Slot00"), retail_bytes).unwrap();
    let mut manager = SaveManager::load_all(&data, None);
    manager.overwrite_slot(0, portable()).unwrap();
    let json_path = data.join("saves/slot_0.json");
    assert!(json_path.exists());
    manager
        .write_checkpoint(
            0,
            &snapshot,
            SaveWriteDisposition::ConfirmedOverwrite,
            0x0020_1000,
        )
        .unwrap();
    assert_eq!(
        manager.slot(0).unwrap().source,
        SaveSource::NativeCompatibilityPreview
    );
    assert_eq!(manager.slot_label(0), "Slot 1: Checkpoint");
    let encoded = fs::read(data.join("Slot00")).unwrap();
    let parsed = RetailSaveSlot::parse(&encoded).unwrap();
    assert!(parsed.full_load_is_valid());
    assert_eq!(parsed.magic, 5);
    assert_eq!(parsed.tail_value, 0x0020_1000);
    assert_eq!(parsed.state_payload, snapshot.state_payload);
    assert_eq!(fs::read(root.join("Slot00")).unwrap(), retail_bytes);
    assert!(!data.join("saves/Slot00").exists());
    assert!(!json_path.exists());

    let reloaded = SaveManager::load_all(&data, None);
    let saved = reloaded.slot(0).unwrap();
    assert_eq!(saved.source, SaveSource::NativeCompatibilityPreview);
    let restore = NativeSaveRestore::decode(saved.native.as_ref().unwrap()).unwrap();
    assert_eq!(restore.state_payload, snapshot.state_payload);
    assert_eq!(restore.pre_health_damage_buffer_raw, 80_833);
    assert_eq!(restore.player.position_raw[0], -32_000);
    assert_eq!(restore.player.velocity_raw[0], i16::MIN);
    assert_eq!(restore.inventory.selected_slot(), 1);
    assert_eq!(
        restore
            .inventory
            .selected_descriptor()
            .stored_resource_count(),
        123
    );
    assert_eq!(restore.cargo.unlock_raw, 5);
    assert_eq!(restore.cargo.carried_identities()[0].entity_type(), 68);
    assert_eq!(restore.cargo.carried_identities()[1].entity_type(), 0);
    assert_eq!(restore.cargo.auxiliary_owned_types()[0], 49);
    assert_eq!(restore.campaign.control_slot_bits(2), Some(0x208));
    assert_eq!(restore.controller_195_raw, 0x56);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn checkpoint_write_guards_validate_before_creating_or_replacing_files() {
    let root = directory("guards");
    let mut manager = SaveManager::load_all(&root, None);
    let snapshot = native();
    assert_eq!(
        manager
            .write_checkpoint(0, &snapshot, SaveWriteDisposition::ConfirmedOverwrite, 0)
            .unwrap_err(),
        EMPTY_OVERWRITE_ERROR
    );
    assert_eq!(
        manager
            .write_checkpoint(
                SETTINGS_SLOT,
                &snapshot,
                SaveWriteDisposition::EmptySlotOnly,
                0
            )
            .unwrap_err(),
        "Used for game settings"
    );
    assert!(manager
        .write_checkpoint(NUM_SLOTS, &snapshot, SaveWriteDisposition::EmptySlotOnly, 0)
        .is_err());
    let mut invalid = snapshot.clone();
    invalid.state_payload[0x140..0x144].fill(0);
    assert!(manager
        .write_checkpoint(0, &invalid, SaveWriteDisposition::EmptySlotOnly, 0)
        .is_err());
    invalid = snapshot.clone();
    invalid.logical_level_id = 4;
    assert!(manager
        .write_checkpoint(0, &invalid, SaveWriteDisposition::EmptySlotOnly, 0)
        .is_err());
    assert!(!root.join("saves").exists());
    assert!(!root.join("Slot00").exists());

    manager
        .write_checkpoint(0, &snapshot, SaveWriteDisposition::EmptySlotOnly, 7)
        .unwrap();
    let path = root.join("Slot00");
    let before = fs::read(&path).unwrap();
    assert_eq!(
        manager
            .write_checkpoint(0, &snapshot, SaveWriteDisposition::EmptySlotOnly, 8)
            .unwrap_err(),
        OCCUPIED_SLOT_ERROR
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    manager
        .write_checkpoint(0, &snapshot, SaveWriteDisposition::ConfirmedOverwrite, 9)
        .unwrap();
    assert_eq!(
        RetailSaveSlot::parse(&fs::read(path).unwrap())
            .unwrap()
            .tail_value,
        9
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn corrupt_owned_checkpoint_is_not_hidden_by_older_portable_or_retail_copy() {
    let root = directory("corrupt_precedence");
    let mut manager = SaveManager::load_all(&root, None);
    manager.save_to_slot(0, portable()).unwrap();
    fs::write(
        root.join("Slot00"),
        encode_native_save_slot(&native().state_payload, 5, 0),
    )
    .unwrap();
    fs::write(root.join("saves/Slot00"), b"partial checkpoint").unwrap();
    let mut reloaded = SaveManager::load_all(&root, None);
    assert_eq!(reloaded.slot_status(0), SaveSlotStatus::Corrupt);
    assert!(!reloaded.is_loadable(0));
    assert_eq!(
        reloaded
            .write_checkpoint(0, &native(), SaveWriteDisposition::EmptySlotOnly, 0)
            .unwrap_err(),
        OCCUPIED_SLOT_ERROR
    );
    reloaded
        .write_checkpoint(0, &native(), SaveWriteDisposition::ConfirmedOverwrite, 0)
        .unwrap();
    assert!(!root.join("saves/Slot00").exists());
    assert!(!root.join("saves/slot_0.json").exists());
    assert!(SaveManager::load_all(&root, None).is_loadable(0));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn late_native_or_portable_file_is_not_shadowed_by_empty_slot_creation() {
    for (name, relative_path) in [
        ("late_native", "saves/Slot00"),
        ("late_json", "saves/slot_0.json"),
        ("late_direct", "Slot00"),
        ("late_parent", "../Slot00"),
    ] {
        let root = directory(name);
        let data = root.join("port");
        fs::create_dir_all(&data).unwrap();
        let mut manager = SaveManager::load_all(&data, None);
        let path = data.join(relative_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let bytes = b"appeared after menu scan";
        fs::write(&path, bytes).unwrap();
        assert_eq!(
            manager
                .write_checkpoint(0, &native(), SaveWriteDisposition::EmptySlotOnly, 0)
                .unwrap_err(),
            OCCUPIED_SLOT_ERROR
        );
        assert_eq!(fs::read(path).unwrap(), bytes);
        assert_eq!(manager.slot_status(0), SaveSlotStatus::Corrupt);
        if relative_path != "Slot00" {
            assert!(!data.join("Slot00").exists());
        }
        if relative_path != "../Slot00" {
            assert!(!root.join("Slot00").exists());
        }
        let _ = fs::remove_dir_all(root);
    }
}

#[test]
fn portable_confirmed_overwrite_keeps_native_file_and_load_precedence() {
    let root = directory("portable_replacement");
    let mut manager = SaveManager::load_all(&root, None);
    manager
        .write_checkpoint(0, &native(), SaveWriteDisposition::EmptySlotOnly, 0)
        .unwrap();
    let native_before = fs::read(root.join("Slot00")).unwrap();
    manager.overwrite_slot(0, portable()).unwrap();
    assert!(!root.join("saves/Slot00").exists());
    assert_eq!(fs::read(root.join("Slot00")).unwrap(), native_before);
    let reloaded = SaveManager::load_all(&root, None);
    assert_eq!(reloaded.slot(0).unwrap().source, SaveSource::Portable);
    assert_eq!(reloaded.slot_label(0), "Slot 1: Portable");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn parent_native_is_overwritten_in_place_and_new_slots_stay_with_parent_set() {
    let root = directory("parent_native");
    let data = root.join("port");
    fs::create_dir_all(&data).unwrap();
    let snapshot = native();
    fs::write(
        root.join("Slot08"),
        encode_native_save_slot(&snapshot.state_payload, 5, 8),
    )
    .unwrap();
    let mut manager = SaveManager::load_all(&data, None);
    manager
        .write_checkpoint(8, &snapshot, SaveWriteDisposition::ConfirmedOverwrite, 80)
        .unwrap();
    manager
        .write_checkpoint(0, &snapshot, SaveWriteDisposition::EmptySlotOnly, 10)
        .unwrap();
    for (slot, tail) in [(8, 80), (0, 10)] {
        let filename = format!("Slot{slot:02}");
        let bytes = fs::read(root.join(&filename)).unwrap();
        let saved = RetailSaveSlot::parse(&bytes).unwrap();
        assert_eq!(saved.tail_value, tail);
        assert_eq!(saved.state_payload, snapshot.state_payload);
        assert!(!data.join(filename).exists());
    }
    assert!(!data.join("saves").exists());
    let restarted = SaveManager::load_all(&data, None);
    assert!(restarted.is_loadable(8));
    assert!(restarted.is_loadable(0));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn mixed_local_native_locations_keep_each_existing_slot_and_prefer_data_for_new_slots() {
    let root = directory("mixed_native");
    let data = root.join("port");
    fs::create_dir_all(&data).unwrap();
    let snapshot = native();
    let initial = encode_native_save_slot(&snapshot.state_payload, 5, 0);
    fs::write(root.join("Slot00"), initial).unwrap();
    fs::write(data.join("Slot01"), initial).unwrap();
    let mut manager = SaveManager::load_all(&data, None);
    for (slot, disposition) in [
        (0, SaveWriteDisposition::ConfirmedOverwrite),
        (1, SaveWriteDisposition::ConfirmedOverwrite),
        (2, SaveWriteDisposition::EmptySlotOnly),
    ] {
        manager
            .write_checkpoint(slot, &snapshot, disposition, 10 + slot as u32)
            .unwrap();
    }
    for (slot, expected_dir, other_dir) in [(0, &root, &data), (1, &data, &root), (2, &data, &root)]
    {
        let filename = format!("Slot{slot:02}");
        assert_eq!(
            RetailSaveSlot::parse(&fs::read(expected_dir.join(&filename)).unwrap())
                .unwrap()
                .tail_value,
            10 + slot
        );
        assert!(!other_dir.join(filename).exists());
    }
    assert!(!data.join("saves").exists());
    let restarted = SaveManager::load_all(&data, None);
    for slot in 0..3 {
        assert!(restarted.is_loadable(slot));
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn explicit_native_save_migrates_only_its_legacy_shadows_and_reloads_local_progress() {
    let root = directory("legacy_migration");
    let data = root.join("port");
    let legacy = data.join("saves");
    fs::create_dir_all(&legacy).unwrap();
    let snapshot = native();
    let mut older = snapshot.clone();
    older.state_payload[..0x20].fill(0);
    older.state_payload[..5].copy_from_slice(b"Older");
    fs::write(
        root.join("Slot00"),
        encode_native_save_slot(&older.state_payload, 5, 1),
    )
    .unwrap();
    let legacy_native = encode_native_save_slot(&snapshot.state_payload, 5, 2);
    let legacy_json = serde_json::to_vec(&portable()).unwrap();
    for slot in [0, 1] {
        fs::write(legacy.join(format!("Slot{slot:02}")), legacy_native).unwrap();
        fs::write(legacy.join(format!("slot_{slot}.json")), &legacy_json).unwrap();
    }
    let mut manager = SaveManager::load_all(&data, None);
    assert_eq!(manager.slot(0).unwrap().level_name, "Checkpoint");
    let mut replacement = snapshot.clone();
    replacement.state_payload[..0x20].fill(0);
    replacement.state_payload[..7].copy_from_slice(b"Updated");
    manager
        .write_checkpoint(0, &replacement, SaveWriteDisposition::ConfirmedOverwrite, 3)
        .unwrap();
    assert!(!legacy.join("Slot00").exists());
    assert!(!legacy.join("slot_0.json").exists());
    assert_eq!(fs::read(legacy.join("Slot01")).unwrap(), legacy_native);
    assert_eq!(fs::read(legacy.join("slot_1.json")).unwrap(), legacy_json);
    assert!(!data.join("Slot00").exists());
    let saved = fs::read(root.join("Slot00")).unwrap();
    assert_eq!(RetailSaveSlot::parse(&saved).unwrap().tail_value, 3);
    let restarted = SaveManager::load_all(&data, None);
    assert_eq!(restarted.slot(0).unwrap().level_name, "Updated");
    assert_eq!(restarted.slot(1).unwrap().level_name, "Checkpoint");
    assert_eq!(
        restarted
            .slot(0)
            .unwrap()
            .native
            .as_ref()
            .unwrap()
            .state_payload,
        replacement.state_payload
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn legacy_only_native_or_json_slot_moves_to_local_native_on_explicit_save() {
    for native_legacy in [true, false] {
        let root = directory(if native_legacy {
            "legacy_only_native"
        } else {
            "legacy_only_json"
        });
        let data = root.join("port");
        let legacy = data.join("saves");
        fs::create_dir_all(&legacy).unwrap();
        let snapshot = native();
        let legacy_path = legacy.join(if native_legacy {
            "Slot00"
        } else {
            "slot_0.json"
        });
        let legacy_bytes = if native_legacy {
            encode_native_save_slot(&snapshot.state_payload, 5, 11).to_vec()
        } else {
            serde_json::to_vec(&portable()).unwrap()
        };
        fs::write(&legacy_path, &legacy_bytes).unwrap();
        let mut manager = SaveManager::load_all(&data, None);
        assert!(manager.is_loadable(0));
        assert_eq!(fs::read(&legacy_path).unwrap(), legacy_bytes);
        assert!(!data.join("Slot00").exists());
        manager
            .write_checkpoint(0, &snapshot, SaveWriteDisposition::ConfirmedOverwrite, 22)
            .unwrap();
        assert!(!legacy_path.exists());
        assert!(!root.join("Slot00").exists());
        assert_eq!(
            RetailSaveSlot::parse(&fs::read(data.join("Slot00")).unwrap())
                .unwrap()
                .tail_value,
            22
        );
        let restarted = SaveManager::load_all(&data, None);
        let saved = restarted.slot(0).unwrap();
        assert_eq!(saved.source, SaveSource::NativeCompatibilityPreview);
        assert_eq!(
            saved.native.as_ref().unwrap().state_payload,
            snapshot.state_payload
        );
        let _ = fs::remove_dir_all(root);
    }
}

#[test]
fn unreadable_direct_native_destination_does_not_fall_back_to_parent_or_remove_legacy() {
    let root = directory("blocked_direct");
    let data = root.join("port");
    let legacy = data.join("saves");
    fs::create_dir_all(&legacy).unwrap();
    fs::create_dir_all(data.join("Slot00")).unwrap();
    let snapshot = native();
    let parent_bytes = encode_native_save_slot(&snapshot.state_payload, 5, 11);
    let legacy_bytes = encode_native_save_slot(&snapshot.state_payload, 5, 22);
    fs::write(root.join("Slot00"), parent_bytes).unwrap();
    fs::write(legacy.join("Slot00"), legacy_bytes).unwrap();
    let mut manager = SaveManager::load_all(&data, None);
    assert!(manager.is_loadable(0));
    assert!(manager
        .write_checkpoint(0, &snapshot, SaveWriteDisposition::ConfirmedOverwrite, 33)
        .is_err());
    assert!(data.join("Slot00").is_dir());
    assert_eq!(fs::read(root.join("Slot00")).unwrap(), parent_bytes);
    assert_eq!(fs::read(legacy.join("Slot00")).unwrap(), legacy_bytes);
    assert!(SaveManager::load_all(&data, None).is_loadable(0));
    let _ = fs::remove_dir_all(root);
}

#[test]
#[cfg(windows)]
fn readonly_local_native_publish_failure_preserves_original_and_legacy_copies() {
    let root = directory("readonly_native");
    let data = root.join("port");
    let legacy = data.join("saves");
    fs::create_dir_all(&legacy).unwrap();
    let snapshot = native();
    let local_path = data.join("Slot00");
    let original = encode_native_save_slot(&snapshot.state_payload, 5, 11);
    let legacy_native = encode_native_save_slot(&snapshot.state_payload, 5, 22);
    let legacy_json = serde_json::to_vec(&portable()).unwrap();
    fs::write(&local_path, original).unwrap();
    fs::write(legacy.join("Slot00"), legacy_native).unwrap();
    fs::write(legacy.join("slot_0.json"), &legacy_json).unwrap();
    let mut manager = SaveManager::load_all(&data, None);
    let original_permissions = fs::metadata(&local_path).unwrap().permissions();
    let mut readonly = original_permissions.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&local_path, readonly).unwrap();
    let result =
        manager.write_checkpoint(0, &snapshot, SaveWriteDisposition::ConfirmedOverwrite, 33);
    fs::set_permissions(&local_path, original_permissions).unwrap();
    assert!(result.is_err());
    assert_eq!(fs::read(&local_path).unwrap(), original);
    assert_eq!(fs::read(legacy.join("Slot00")).unwrap(), legacy_native);
    assert_eq!(fs::read(legacy.join("slot_0.json")).unwrap(), legacy_json);
    let restarted = SaveManager::load_all(&data, None);
    assert_eq!(manager.slot_status(0), restarted.slot_status(0));
    assert_eq!(manager.slot_label(0), restarted.slot_label(0));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn failed_legacy_cleanup_reports_error_and_matches_restarted_slot_state() {
    for blocked_native in [true, false] {
        let root = directory(if blocked_native {
            "blocked_native_cleanup"
        } else {
            "blocked_json_cleanup"
        });
        let data = root.join("port");
        let legacy = data.join("saves");
        fs::create_dir_all(&legacy).unwrap();
        let snapshot = native();
        let bytes = encode_native_save_slot(&snapshot.state_payload, 5, 11);
        fs::write(data.join("Slot00"), bytes).unwrap();
        let blocked_path = legacy.join(if blocked_native {
            "Slot00"
        } else {
            "slot_0.json"
        });
        fs::create_dir_all(&blocked_path).unwrap();
        if !blocked_native {
            fs::write(legacy.join("Slot00"), bytes).unwrap();
        }
        let mut manager = SaveManager::load_all(&data, None);
        let mut replacement = snapshot.clone();
        replacement.state_payload[..0x20].fill(0);
        replacement.state_payload[..7].copy_from_slice(b"Updated");
        assert!(manager
            .write_checkpoint(
                0,
                &replacement,
                SaveWriteDisposition::ConfirmedOverwrite,
                22
            )
            .is_err());
        assert!(blocked_path.is_dir());
        assert_eq!(
            RetailSaveSlot::parse(&fs::read(data.join("Slot00")).unwrap())
                .unwrap()
                .tail_value,
            22
        );
        let restarted = SaveManager::load_all(&data, None);
        assert_eq!(manager.slot_status(0), restarted.slot_status(0));
        assert_eq!(manager.slot_label(0), restarted.slot_label(0));
        assert_eq!(manager.is_loadable(0), restarted.is_loadable(0));
        if blocked_native {
            assert_eq!(manager.slot_status(0), SaveSlotStatus::Corrupt);
        } else {
            assert_eq!(manager.slot(0).unwrap().level_name, "Updated");
        }
        let _ = fs::remove_dir_all(root);
    }
}
