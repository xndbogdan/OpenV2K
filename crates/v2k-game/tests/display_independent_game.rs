//! The game must not depend on the display: the model root test
//! (`FUN_00465870`) and the particle gate (`FUN_0043D410`) write game state,
//! and both are the authored frame's whatever the frame shown. A New Game
//! into Intro2, at one display tier (800x600), plays the same process random
//! stream in the software renderer's 4:3 frame and in OpenGL's widescreen.

use std::path::Path;
use std::process::Command;

fn random_words(name: &str, renderer: &str, scaling: &str) -> Vec<String> {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("display_independent_game")
        .join(name);
    let settings = dir.join("settings");
    std::fs::create_dir_all(&settings).unwrap();
    std::fs::write(
        settings.join("port-config.json"),
        format!(
            r#"{{"renderer": "{renderer}", "width": 1280, "height": 720, "scaling": "{scaling}"}}"#
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("keys.json"),
        r#"[{"frame": 400, "key": "Return", "pressed": true},
            {"frame": 403, "key": "Return", "pressed": false}]"#,
    )
    .unwrap();
    let events = dir.join("events.jsonl");
    let _ = std::fs::remove_file(&events);
    let output = Command::new(env!("CARGO_BIN_EXE_v2k-game"))
        .arg("--data-dir")
        .arg(v2k_test_support::retail_dir())
        .arg("--settings")
        .arg(&settings)
        .arg("--skip-intro")
        .arg("--input-replay")
        .arg(dir.join("keys.json"))
        .args(["--hidden", "--renderer", renderer])
        .args(["--fixed-step-micros", "20000", "--frames", "560"])
        .arg("--event-log")
        .arg(&events)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let lines: Vec<serde_json::Value> = std::fs::read_to_string(&events)
        .unwrap()
        .lines()
        .skip(1)
        .map(|line| serde_json::from_str(line).unwrap())
        .filter(|line: &serde_json::Value| line["ev"] == "frame")
        .collect();
    assert!(
        lines
            .iter()
            .any(|line| line["state"] == "opening_cinematic"),
        "{name} never reached Intro2"
    );
    lines
        .iter()
        .map(|line| format!("{} {}", line["frame"], line["rng"]))
        .collect()
}

// Hidden OpenGL windows are what the Windows check machine provides.
#[cfg(windows)]
#[v2k_test_support::retail_test]
fn intro2_plays_one_random_stream_in_software_four_three_and_opengl_widescreen() {
    let software = random_words("software", "software", "four_three");
    let widescreen = random_words("widescreen", "opengl", "native");
    let parting = software
        .iter()
        .zip(&widescreen)
        .find(|(software, widescreen)| software != widescreen);
    assert_eq!(parting, None);
    assert_eq!(software.len(), widescreen.len());
}
