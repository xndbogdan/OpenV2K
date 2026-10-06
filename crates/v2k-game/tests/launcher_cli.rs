//! Invalid data must fail before creating game settings or opening SDL.
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct Scratch(PathBuf);
impl Scratch {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "v2k-launcher-cli-{name}-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn help_output_survives_gui_subsystem_startup() {
    let result = Command::new(env!("CARGO_BIN_EXE_v2k-game"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(result.status.success());
    let output = String::from_utf8(result.stdout).unwrap();
    assert!(output.contains("--launcher"), "{output}");
    assert!(output.contains("extract-music"), "{output}");
}

#[test]
fn direct_invalid_root_fails_without_settings_or_launcher_state_writes() {
    let scratch = Scratch::new("invalid");
    let state = scratch.0.join("launcher.json");
    let result = Command::new(env!("CARGO_BIN_EXE_v2k-game"))
        .current_dir(&scratch.0)
        .arg("--no-launcher")
        .arg("--data-dir")
        .arg(&scratch.0)
        .arg("--launcher-state")
        .arg(&state)
        .output()
        .unwrap();
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("PRELOAD.DAT"), "{stderr}");
    assert!(!state.exists());
    for name in ["settings.json", "port-config.json", "config.json"] {
        assert!(
            !scratch.0.join(name).exists(),
            "invalid root acquired {name}"
        );
    }
}

#[test]
fn verify_is_a_noninteractive_structured_diagnostic() {
    let scratch = Scratch::new("verify");
    let result = Command::new(env!("CARGO_BIN_EXE_v2k-game"))
        .arg("verify")
        .arg(&scratch.0)
        .output()
        .unwrap();
    assert!(!result.status.success());
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["checked_overlays"], 0);
    assert!(report["issues"]
        .as_array()
        .unwrap()
        .iter()
        .any(|issue| issue["path"].as_str().unwrap().ends_with("PRELOAD.DAT")));
    assert_eq!(std::fs::read_dir(&scratch.0).unwrap().count(), 0);
}
