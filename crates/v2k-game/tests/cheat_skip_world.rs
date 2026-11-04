//! A menu replay through retail's cheats: the master code typed in play
//! (Left Shift held over C H E A T S E Q), then Pause -> Cheats -> Skip to
//! World, Right once for Mediaeval, and Space on the campaign map once Klaus
//! has closed over the world (about 53 frames) and the map is up.

use std::path::Path;
use std::process::Command;

fn press(keys: &mut Vec<String>, frame: u32, key: &str) {
    for (at, pressed) in [(frame, true), (frame + 2, false)] {
        keys.push(format!(
            r#"{{"frame": {at}, "key": "{key}", "pressed": {pressed}}}"#
        ));
    }
}

fn hold(keys: &mut Vec<String>, frame: u32, key: &str, pressed: bool) {
    keys.push(format!(
        r#"{{"frame": {frame}, "key": "{key}", "pressed": {pressed}}}"#
    ));
}

fn replay() -> String {
    let mut keys = Vec::new();
    press(&mut keys, 30, "Space"); // skips Level 1's opening
    hold(&mut keys, 60, "Left Shift", true);
    let mut frame = 65;
    for key in ["C", "H", "E", "A", "T", "S", "E", "Q"] {
        press(&mut keys, frame, key);
        frame += 6;
    }
    hold(&mut keys, frame + 5, "Left Shift", false);
    frame += 40;
    press(&mut keys, frame, "Escape");
    frame += 70;
    for _ in 0..5 {
        press(&mut keys, frame, "Down"); // Continue .. Replay, then Cheats
        frame += 15;
    }
    press(&mut keys, frame + 20, "Return");
    frame += 100;
    press(&mut keys, frame, "Right"); // world 1 -> 2
    press(&mut keys, frame + 30, "Return");
    press(&mut keys, frame + 120, "Space"); // the campaign map
    format!("[{}]", keys.join(",\n"))
}

#[derive(Debug, PartialEq)]
struct Pose {
    position: serde_json::Value,
    velocity: serde_json::Value,
    heading: serde_json::Value,
    attitude: serde_json::Value,
    health: serde_json::Value,
}

#[v2k_test_support::retail_test]
fn skip_to_world_keeps_the_players_pose_into_the_chosen_world() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cheat_skip_world");
    let settings = dir.join("settings");
    std::fs::create_dir_all(&settings).unwrap();
    std::fs::write(dir.join("start.json"), r#"{"level": 13}"#).unwrap();
    std::fs::write(dir.join("keys.json"), replay()).unwrap();
    let events = dir.join("events.jsonl");
    let _ = std::fs::remove_file(&events);
    let output = Command::new(env!("CARGO_BIN_EXE_v2k-game"))
        .arg("--data-dir")
        .arg(v2k_test_support::retail_dir())
        .arg("--settings")
        .arg(&settings)
        .arg("--start")
        .arg(dir.join("start.json"))
        .arg("--input-replay")
        .arg(dir.join("keys.json"))
        .args(["--hidden", "--renderer", "software"])
        .args(["--fixed-step-micros", "20000", "--frames", "620"])
        .arg("--event-log")
        .arg(&events)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    for stop in [
        "blocked",
        "stopped at unresolved data",
        "is unowned",
        "held",
    ] {
        assert!(!stderr.contains(stop), "{stop}: {stderr}");
    }

    let lines: Vec<serde_json::Value> = std::fs::read_to_string(&events)
        .unwrap()
        .lines()
        .skip(1)
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let states: Vec<(String, u64)> = lines
        .iter()
        .filter(|line| line["ev"] == "frame")
        .map(|line| {
            (
                line["state"].as_str().unwrap().to_owned(),
                line["level"].as_u64().unwrap(),
            )
        })
        .collect();
    let mut changes = states.clone();
    changes.dedup();
    let names: Vec<(&str, u64)> = changes.iter().map(|(s, l)| (s.as_str(), *l)).collect();
    assert_eq!(
        names,
        [
            ("playing", 13),
            ("paused", 13),
            ("playing", 13),
            ("closing_scene_leg", 13),
            ("playing", 13),
            ("loading", 13),
            ("playing", 14),
        ]
    );

    let players: Vec<&serde_json::Value> =
        lines.iter().filter(|line| line["ev"] == "player").collect();
    let pose = |line: &serde_json::Value| Pose {
        position: line["position"].clone(),
        velocity: line["velocity"].clone(),
        heading: line["heading"].clone(),
        attitude: line["attitude"].clone(),
        health: line["health"].clone(),
    };
    let arrival = states
        .iter()
        .position(|(state, level)| state == "playing" && *level == 14)
        .unwrap();
    // The skip leaves after one more tick; the exit leg and the map hold the
    // world, so the destination's first frame has the pose the source left
    // with.
    assert_eq!(pose(players[arrival - 1]), pose(players[arrival]));
    // One tick ran between the pause menu closing and the map.
    let resumed = states
        .iter()
        .rposition(|(state, level)| state == "paused" && *level == 13)
        .unwrap()
        + 1;
    let ticks: Vec<u64> = lines
        .iter()
        .filter(|line| line["ev"] == "frame")
        .map(|line| line["tick"].as_u64().unwrap())
        .collect();
    // One frame of play runs the skip's tick, whose +28C starts the exit leg
    // at once; the leg's 453BA0 then restarts the clock (44FE20).
    let leg = states
        .iter()
        .position(|(state, _)| state == "closing_scene_leg")
        .unwrap();
    assert_eq!(leg, resumed + 1);
    assert_eq!(ticks[leg], 0);
}
