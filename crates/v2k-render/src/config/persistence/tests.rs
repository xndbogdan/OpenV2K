use super::*;

struct Directory(std::path::PathBuf);
impl Directory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "v2000-native-settings-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn registry_table_retains_exact_names_offsets_and_defaults() {
    assert_eq!(RETAIL_KEY, r"Software\Frontier Developments Ltd\V2000\1.0");
    assert_ne!(RETAIL_KEY, PORT_KEY);
    assert_eq!(
        FIELDS.map(|(_, offset, _)| offset),
        [0, 4, 8, 12, 16, 20, 24, 28, 36, 40, 44, 48, 52, 56, 60]
    );
    assert_eq!(
        FIELDS.map(|(_, _, value)| value),
        [15, 15, 1, 0, 1, 1, 1, 1, 1, 0, 10, 6, 1, 1, 0]
    );
    assert_eq!(FIELDS.len(), 15);
    assert_eq!(NativeSettings::default().get(11), 6);
    assert!(!NativeSettings::default().0.contains_key("Vibration"));
}

#[test]
fn legacy_upgrade_wins_over_retail_and_normalizes_unsupported_display() {
    let dir = Directory::new();
    let legacy = br#"{"renderer":"wgpu","width":1920,"height":1080,"fullscreen":false,"sfx_volume":0.4,"music_volume":0.46666667,"self_righting":false,"scaling":"stretched","classic_framebuffer":true,"difficulty":"hard"}"#;
    fs::write(dir.0.join("config.json"), legacy).unwrap();
    let mut retail = NativeSettings::default();
    retail.set(3, 0x76543210);
    retail.set(4, 0);
    let config = load_from_sources(&dir.0, None, Some(retail));
    assert_eq!((config.width, config.height), (1024, 768));
    assert_eq!(config.renderer, RendererChoice::Wgpu);
    assert_eq!(config.scaling, ScalingMode::Stretched);
    assert!(config.classic_framebuffer_effective());
    assert_eq!(config.difficulty, Difficulty::Hard);
    assert_eq!(config.self_righting, 0);
    assert!(!config.fullscreen);
    let native = native_to_save(&config);
    assert_eq!(native.get(0), 6);
    assert_eq!(native.get(1), 7);
    assert_eq!(native.get(3), 0x76543210);
    assert_eq!(native.get(4), 1);
    assert_eq!(fs::read(dir.0.join("config.json")).unwrap(), legacy);
    assert!(
        !dir.0.join("settings.json").exists(),
        "loading is read-only"
    );
}

#[test]
fn owned_registry_precedes_fallback_and_legacy_without_losing_unknown_dwords() {
    let dir = Directory::new();
    let mut fallback = NativeSettings::default();
    fallback.set(0, 3);
    fallback.0.insert("Unowned DWORD".into(), 0x89abcdef);
    atomic_json(&dir.0.join("settings.json"), &fallback).unwrap();
    let mut owned = NativeSettings::default();
    owned.set(0, 9);
    owned.set(1, 7);
    let config = load_from_sources(&dir.0, Some(owned), Some(NativeSettings::default()));
    let native = native_to_save(&config);
    assert_eq!(native.get(0), 9);
    assert_eq!(native.get(1), 7);
    assert_eq!(native.0["Unowned DWORD"], 0x89abcdef);
}

#[test]
fn native_tier_is_not_modern_resolution_index_and_unrepresented_words_survive() {
    let dir = Directory::new();
    let mut retail = NativeSettings::default();
    retail.set(3, 0x12345678);
    retail.set(4, 2);
    retail.set(8, 0xfffffffe);
    retail.set(14, 0x1234);
    let mut config = load_from_sources(&dir.0, None, Some(retail.clone()));
    assert_eq!((config.width, config.height), (800, 600));
    assert_eq!(config.self_righting, 15);
    assert_eq!(native_to_save(&config), retail);
    config.set_resolution_index(4); // clamped to 1024x768, not retail tier 4
    assert_eq!((config.width, config.height), (1024, 768));
    config.difficulty = Difficulty::Easy;
    config.music_volume = 7.0 / 15.0;
    let native = native_to_save(&config);
    assert_eq!(native.get(4), 2);
    assert_eq!(native.get(3), 0x12345678);
    assert_eq!(native.get(8), 0xfffffffe);
    assert_eq!(native.get(14), 0x1234);
    assert_eq!(native.get(1), 7);
    config.self_righting = 8;
    save_with_registry(&mut config, &dir.0, |_| Ok(())).unwrap();
    config.self_righting = 15;
    assert_eq!(
        native_to_save(&config).get(8),
        15,
        "a subsequent explicit edit does not resurrect the old opaque word"
    );
    save_with_registry(&mut config, &dir.0, |_| Ok(())).unwrap();
    let stored: NativeSettings = read_json(&dir.0.join("settings.json")).unwrap();
    assert_eq!(stored.get(8), 15);
    assert_eq!(stored.get(3), 0x12345678);
    assert_eq!(stored.get(14), 0x1234);
}

#[test]
fn persistence_separates_native_and_port_files_and_preserves_original_evidence() {
    let dir = Directory::new();
    let mut config = GameConfig {
        width: 1920,
        height: 1080,
        scaling: ScalingMode::FourThree,
        classic_framebuffer: true,
        music_volume: 7.0 / 15.0,
        ..GameConfig::default()
    };
    let legacy = serde_json::to_vec(&config).unwrap();
    fs::write(dir.0.join("config.json"), &legacy).unwrap();
    fs::write(dir.0.join("Slot00"), b"original native save evidence").unwrap();
    let mut stored = None;
    save_with_registry(&mut config, &dir.0, |native| {
        stored = Some(native.clone());
        Ok(())
    })
    .unwrap();
    let native: NativeSettings = read_json(&dir.0.join("settings.json")).unwrap();
    assert_eq!(native, stored.unwrap());
    assert_eq!(native.get(1), 7);
    assert_eq!(native.get(4), 1);
    let port: serde_json::Value = read_json(&dir.0.join("port-config.json")).unwrap();
    assert_eq!(port["width"], 1024);
    assert_eq!(port["height"], 768);
    assert!(port.get("music_volume").is_none());
    assert!(port.get("self_righting").is_none());
    assert_eq!(fs::read(dir.0.join("config.json")).unwrap(), legacy);
    assert_eq!(
        fs::read(dir.0.join("Slot00")).unwrap(),
        b"original native save evidence"
    );
    let restored = load_from_sources(&dir.0, None, None);
    assert_eq!((config.width, config.height), (1024, 768));
    assert_eq!((restored.width, restored.height), (1024, 768));
    assert_eq!(restored.scaling, ScalingMode::FourThree);
    assert!(restored.classic_framebuffer_effective());
    assert_eq!(native_to_save(&restored).get(1), 7);
}

#[test]
fn native_write_errors_are_visible_and_do_not_replace_fallback_or_preferences() {
    let dir = Directory::new();
    let mut config = GameConfig::default();
    save_with_registry(&mut config, &dir.0, |_| Ok(())).unwrap();
    let before_native = fs::read(dir.0.join("settings.json")).unwrap();
    let before_port = fs::read(dir.0.join("port-config.json")).unwrap();
    config.music_volume = 0.0;
    let result = save_with_registry(&mut config, &dir.0, |_| {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "test backend denied",
        ))
    });
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(
        fs::read(dir.0.join("settings.json")).unwrap(),
        before_native
    );
    assert_eq!(
        fs::read(dir.0.join("port-config.json")).unwrap(),
        before_port
    );
}

#[test]
fn corrupt_legacy_and_port_fallback_can_still_import_valid_retail_values() {
    let dir = Directory::new();
    fs::write(dir.0.join("config.json"), b"invalid JSON retained").unwrap();
    fs::write(dir.0.join("settings.json"), b"invalid DWORD map retained").unwrap();
    let mut retail = NativeSettings::default();
    retail.set(4, 0);
    retail.set(1, 0);
    let config = load_from_sources(&dir.0, None, Some(retail));
    assert_eq!((config.width, config.height), (640, 480));
    assert_eq!(config.detail, GraphicsDetail::Low);
    assert_eq!(config.system_graphics_variant(), 0);
    assert!(!config.ambient_enabled);
    assert_eq!(
        fs::read(dir.0.join("settings.json")).unwrap(),
        b"invalid DWORD map retained"
    );
}

#[test]
fn old_hd_preferences_normalize_on_load_and_save_without_coercing_native_words() {
    for (input, expected) in [
        ((1280, 720), (800, 600)),
        ((1920, 1080), (1024, 768)),
        ((3840, 2160), (1024, 768)),
    ] {
        for filename in ["config.json", "port-config.json"] {
            let dir = Directory::new();
            let old_config = GameConfig {
                width: input.0,
                height: input.1,
                ..GameConfig::default()
            };
            let imported = serde_json::to_vec(&old_config).unwrap();
            fs::write(dir.0.join(filename), &imported).unwrap();
            let mut native = NativeSettings::default();
            native.set(3, 0x12345678);
            native.set(4, 0x87654321);
            native.0.insert("Unowned DWORD".into(), 0x89abcdef);
            let mut config = load_from_sources(&dir.0, Some(native.clone()), None);
            assert_eq!((config.width, config.height), expected);
            assert_eq!(fs::read(dir.0.join(filename)).unwrap(), imported);
            assert_eq!(native_to_save(&config), native);

            // Save also repairs dimensions edited by an older caller, before
            // writing port preferences or consulting the registry backend.
            config.width = input.0;
            config.height = input.1;
            save_with_registry(&mut config, &dir.0, |stored| {
                assert_eq!(stored, &native);
                Ok(())
            })
            .unwrap();
            assert_eq!((config.width, config.height), expected);
            let stored: PortPreferences = read_json(&dir.0.join("port-config.json")).unwrap();
            assert_eq!((stored.width, stored.height), expected);
            let restored = load_from_sources(&dir.0, None, None);
            assert_eq!((restored.width, restored.height), expected);
            assert_eq!(native_to_save(&restored), native);
            if filename == "config.json" {
                assert_eq!(fs::read(dir.0.join(filename)).unwrap(), imported);
            }
        }
    }
}

#[test]
fn low_detail_uses_minimum_window_while_preserving_native_low_tier_on_save() {
    let dir = Directory::new();
    let mut retail = NativeSettings::default();
    retail.set(4, 0);
    let mut config = load_from_sources(&dir.0, None, Some(retail.clone()));
    assert_eq!((config.width, config.height), (640, 480));
    assert_eq!(config.detail, GraphicsDetail::Low);
    assert_eq!(config.system_graphics_variant(), 0);
    assert_eq!(config.detail.menu_virtual_size(), (320, 240));
    config.width = 320;
    config.height = 240;
    save_with_registry(&mut config, &dir.0, |stored| {
        assert_eq!(stored, &retail);
        Ok(())
    })
    .unwrap();
    let stored: PortPreferences = read_json(&dir.0.join("port-config.json")).unwrap();
    assert_eq!((stored.width, stored.height), (640, 480));
    assert_eq!(stored.detail, GraphicsDetail::Low);
    let restored = load_from_sources(&dir.0, None, None);
    assert_eq!(restored.system_graphics_variant(), 0);
    assert_eq!(native_to_save(&restored), retail);
}

#[test]
fn unsupported_software_preferences_normalize_without_rewriting_native_renderer_evidence() {
    for renderer_word in [0, 0x76543210] {
        let dir = Directory::new();
        let imported = serde_json::to_vec(&GameConfig {
            renderer: RendererChoice::Software,
            ..GameConfig::default()
        })
        .unwrap();
        fs::write(dir.0.join("config.json"), &imported).unwrap();
        let mut native = NativeSettings::default();
        native.set(5, renderer_word);
        let mut config = load_from_sources(&dir.0, Some(native.clone()), None);
        assert_eq!(config.renderer, RendererChoice::OpenGL);
        assert_eq!(native_to_save(&config), native);
        assert_eq!(fs::read(dir.0.join("config.json")).unwrap(), imported);

        // Normalize stale callers as well, before comparing the projected
        // settings to the lossless imported DWORD baseline.
        config.renderer = RendererChoice::Software;
        save_with_registry(&mut config, &dir.0, |stored| {
            assert_eq!(stored, &native);
            Ok(())
        })
        .unwrap();
        assert_eq!(config.renderer, RendererChoice::OpenGL);
        let stored: PortPreferences = read_json(&dir.0.join("port-config.json")).unwrap();
        assert_eq!(stored.renderer, RendererChoice::OpenGL);
        assert_eq!(fs::read(dir.0.join("config.json")).unwrap(), imported);
        let restored = load_from_sources(&dir.0, None, None);
        assert_eq!(restored.renderer, RendererChoice::OpenGL);
        assert_eq!(native_to_save(&restored), native);
    }
}
