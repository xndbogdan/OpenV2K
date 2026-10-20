//! Unattended test runs: a fixed clock step, scripted keys, a frame limit,
//! snapshots, a start state and an event log, so a run plays the same on
//! every machine and every time. Frames count from 1, as `--frames` does:
//! frame N's scripted keys arrive with its own events, and a snapshot "at N"
//! is the image frame N presented.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use sdl2::keyboard::Keycode;
use serde::{Deserialize, Serialize};
use v2k_render::GameEvent;

/// One scripted key change of `--input-replay`.
#[derive(Debug, Deserialize)]
struct ScriptedKey {
    frame: u64,
    /// SDL key name: `Up`, `Space`, `Return`, `Tab`, `Right Shift`, `A`...
    key: String,
    pressed: bool,
}

/// `--input-replay`: key changes by frame, in file order within a frame.
#[derive(Debug, Default)]
pub struct InputReplay {
    keys: Vec<(u64, Keycode, bool)>,
    next: usize,
}

impl InputReplay {
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let keys: Vec<ScriptedKey> = serde_json::from_slice(&text)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        Self::from_keys(keys)
    }

    fn from_keys(keys: Vec<ScriptedKey>) -> Result<Self, String> {
        let mut previous = 1;
        let keys = keys
            .into_iter()
            .map(|key| {
                if key.frame < previous {
                    return Err(format!(
                        "input replay: frame {} is not a positive frame in order",
                        key.frame
                    ));
                }
                previous = key.frame;
                let code = Keycode::from_name(&key.key)
                    .ok_or_else(|| format!("input replay: {:?} is not an SDL key name", key.key))?;
                Ok((key.frame, code, key.pressed))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { keys, next: 0 })
    }

    /// The scripted events of `frame`, once.
    pub fn events(&mut self, frame: u64) -> Vec<GameEvent> {
        let mut events = Vec::new();
        while let Some(&(at, key, pressed)) = self.keys.get(self.next) {
            if at > frame {
                break;
            }
            self.next += 1;
            events.push(if pressed {
                GameEvent::KeyDown(key)
            } else {
                GameEvent::KeyUp(key)
            });
        }
        events
    }
}

/// `--snapshot-at FRAME:PATH`.
pub fn parse_snapshot_at(text: &str) -> Result<(u64, PathBuf), String> {
    let (frame, path) = text
        .split_once(':')
        .ok_or("expected FRAME:PATH, for example 120:end.png")?;
    let frame = frame
        .parse::<u64>()
        .ok()
        .filter(|&frame| frame > 0)
        .ok_or("FRAME must be a positive frame number")?;
    if path.is_empty() {
        return Err("PATH is empty".into());
    }
    Ok((frame, PathBuf::from(path)))
}

/// `--fixed-step-micros`: whole milliseconds, as retail's `timeGetTime`
/// pump produces, of at least its 8-ms minimum frame period.
pub fn parse_fixed_step(text: &str) -> Result<u32, String> {
    let micros = text.parse::<u32>().map_err(|error| error.to_string())?;
    if micros < 8_000 || micros % 1_000 != 0 {
        return Err("a whole number of milliseconds of at least 8000 microseconds".into());
    }
    Ok(micros)
}

/// `--start STATE.json`: where gameplay begins, skipping the intro and menus.
/// Either `save` (a native save file such as a copy of `Slot02`) or `level`
/// (a global overlay id: 13 is the first world), optionally with the
/// player's raw world `position` `[x, y, z]` and `heading` (0..65535), and
/// with that position its raw `velocity` `[x, y, z]` and body `attitude`
/// `[pitch, roll]`: retail keeps all of them across Skip to World.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartState {
    pub save: Option<PathBuf>,
    pub level: Option<u32>,
    pub position: Option<[i16; 3]>,
    pub heading: Option<u16>,
    pub velocity: Option<[i16; 3]>,
    pub attitude: Option<[i16; 2]>,
    /// A network game instead of a save or level.
    pub network: Option<NetworkStart>,
}

/// `{"network": {"host": true, "arena": 1}}` hosts a session on this machine
/// and enters arena 1..=6 (the frontend's keys 1-6; without one, as retail,
/// one of the first five at random); `{"network": {"join": "HOST"}}` searches
/// HOST (blank: the local network) and joins the first session found, then
/// waits for the host's world. `port` overrides the session port; a raw
/// `position` [x, y, z] and `heading` replace the random entry point of every
/// entry, the first and each one after a death.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkStart {
    #[serde(default)]
    pub host: bool,
    pub arena: Option<u8>,
    pub join: Option<String>,
    pub port: Option<u16>,
    pub position: Option<[i16; 3]>,
    pub heading: Option<u16>,
}

impl StartState {
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let state: Self = serde_json::from_slice(&text)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let state = Self {
            // A relative save path is the state file's neighbour.
            save: state
                .save
                .map(|save| path.parent().unwrap_or(Path::new(".")).join(save)),
            ..state
        };
        state.validate()?;
        Ok(state)
    }

    fn validate(&self) -> Result<(), String> {
        if let Some(network) = &self.network {
            if self.save.is_some() || self.level.is_some() || self.position.is_some() {
                return Err("start state: a network game takes no save, level or pose".into());
            }
            if network.host == network.join.is_some() {
                return Err("start state: a network game either hosts or joins".into());
            }
            return Ok(());
        }
        match (&self.save, self.level) {
            (Some(_), Some(_)) => Err("start state: give either save or level".into()),
            (None, None) => Err("start state: give save or level".into()),
            (Some(_), None)
                if self.position.is_some()
                    || self.heading.is_some()
                    || self.velocity.is_some()
                    || self.attitude.is_some() =>
            {
                Err("start state: a save holds its own pose".into())
            }
            (None, Some(_))
                if self.position.is_none()
                    && (self.heading.is_some()
                        || self.velocity.is_some()
                        || self.attitude.is_some()) =>
            {
                Err("start state: a heading, velocity or attitude needs a position".into())
            }
            _ => Ok(()),
        }
    }
}

/// Everything a test run asked for; the default is an ordinary session.
#[derive(Default)]
pub struct ProbeRun {
    /// `--settings`: the portable settings folder.
    pub settings: Option<PathBuf>,
    /// `--width`/`--height`.
    pub size: Option<(u32, u32)>,
    pub hidden: bool,
    pub frames: Option<u64>,
    pub fixed_step_micros: Option<u32>,
    pub snapshot: Option<PathBuf>,
    pub snapshot_at: Vec<(u64, PathBuf)>,
    pub input: InputReplay,
    pub start: Option<StartState>,
    pub event_log: Option<EventLog>,
    /// `--actor-log`: every live entity each frame, in the event log's format.
    pub actor_log: Option<EventLog>,
    pub entities: EntityCensus,
}

/// The entities alive at the end of the previous frame, so `--event-log` can
/// name those that appeared and went since.
#[derive(Debug, Default)]
pub struct EntityCensus {
    live: std::collections::BTreeMap<u32, (u32, [i16; 3])>,
}

impl EntityCensus {
    /// Replace the census with `entities` (id, type, position) and return
    /// the changes: spawns first, by id, then disposals, by id.
    pub fn update(
        &mut self,
        entities: impl Iterator<Item = (u32, u32, [i16; 3])>,
    ) -> Vec<(&'static str, u32, u32, [i16; 3])> {
        let now: std::collections::BTreeMap<_, _> = entities
            .map(|(id, entity_type, position)| (id, (entity_type, position)))
            .collect();
        let spawned = now
            .iter()
            .filter(|(id, _)| !self.live.contains_key(id))
            .map(|(&id, &(entity_type, position))| ("spawn", id, entity_type, position));
        let disposed = self
            .live
            .iter()
            .filter(|(id, _)| !now.contains_key(id))
            .map(|(&id, &(entity_type, position))| ("dispose", id, entity_type, position));
        let changes = spawned.chain(disposed).collect();
        self.live = now;
        changes
    }

    pub fn clear(&mut self) {
        self.live.clear();
    }
}

/// `--event-log`: one JSON line per event, after a header line.
pub struct EventLog {
    out: BufWriter<File>,
}

/// The player's state at the end of a gameplay frame.
#[derive(Serialize)]
pub struct PlayerLine {
    pub position: [i16; 3],
    pub velocity: [i16; 3],
    pub heading: u16,
    /// Body pitch and roll words.
    pub attitude: [i16; 2],
    pub mode: &'static str,
    pub health: i32,
}

impl EventLog {
    pub fn create(path: &Path) -> std::io::Result<Self> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        let mut log = Self {
            out: BufWriter::new(File::create(path)?),
        };
        log.line(&serde_json::json!({
            "openv2k": env!("CARGO_PKG_VERSION"),
            "schema": 1,
            "args": std::env::args().skip(1).collect::<Vec<_>>(),
        }))?;
        Ok(log)
    }

    fn line(&mut self, value: &serde_json::Value) -> std::io::Result<()> {
        serde_json::to_writer(&mut self.out, value)?;
        self.out.write_all(b"\n")
    }

    /// A frame's clock: its delta and the 50-Hz world tick after it, with
    /// the game state and the loaded global level.
    pub fn frame(
        &mut self,
        frame: u64,
        state: &str,
        level: Option<u32>,
        delta_us: u32,
        tick: u32,
    ) -> std::io::Result<()> {
        self.line(&serde_json::json!({
            "frame": frame, "ev": "frame", "state": state, "level": level,
            "delta_us": delta_us, "tick": tick,
        }))
    }

    pub fn player(&mut self, frame: u64, tick: u32, player: &PlayerLine) -> std::io::Result<()> {
        let mut value = serde_json::to_value(player)?;
        value["frame"] = frame.into();
        value["ev"] = "player".into();
        value["tick"] = tick.into();
        self.line(&value)
    }

    /// An entity that appeared (`spawn`) or went (`dispose`) during a frame.
    pub fn entity(
        &mut self,
        frame: u64,
        tick: u32,
        ev: &str,
        id: u32,
        entity_type: u32,
        position: [i16; 3],
    ) -> std::io::Result<()> {
        self.line(&serde_json::json!({
            "frame": frame, "ev": ev, "tick": tick, "id": id, "type": entity_type,
            "position": position,
        }))
    }

    /// Every live entity at the end of a frame: `[id, type, x, y, z, health,
    /// detailed]`, health and the detailed state bit null while unresolved.
    pub fn actors(
        &mut self,
        frame: u64,
        tick: u32,
        actors: impl Iterator<Item = (u32, u32, [i16; 3], Option<i32>, Option<bool>)>,
    ) -> std::io::Result<()> {
        let rows: Vec<_> = actors
            .map(|(id, entity_type, [x, y, z], health, detailed)| {
                serde_json::json!([id, entity_type, x, y, z, health, detailed])
            })
            .collect();
        self.line(&serde_json::json!({
            "frame": frame, "ev": "actors", "tick": tick, "actors": rows,
        }))
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.out.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(frame: u64, key: &str, pressed: bool) -> ScriptedKey {
        ScriptedKey {
            frame,
            key: key.into(),
            pressed,
        }
    }

    #[test]
    fn scripted_keys_arrive_once_on_their_frames_in_order() {
        let mut replay = InputReplay::from_keys(vec![
            key(2, "Up", true),
            key(2, "Space", true),
            key(5, "Up", false),
        ])
        .unwrap();
        assert!(replay.events(1).is_empty());
        let frame2 = replay.events(2);
        assert!(matches!(
            frame2.as_slice(),
            [
                GameEvent::KeyDown(Keycode::Up),
                GameEvent::KeyDown(Keycode::Space)
            ]
        ));
        assert!(replay.events(2).is_empty());
        assert!(replay.events(4).is_empty());
        assert!(matches!(
            replay.events(5).as_slice(),
            [GameEvent::KeyUp(Keycode::Up)]
        ));
    }

    #[test]
    fn scripted_keys_refuse_unknown_names_and_frames_out_of_order() {
        assert!(InputReplay::from_keys(vec![key(1, "NoSuchKey", true)]).is_err());
        assert!(InputReplay::from_keys(vec![key(0, "Up", true)]).is_err());
        assert!(InputReplay::from_keys(vec![key(3, "Up", true), key(2, "Up", false)]).is_err());
        assert!(InputReplay::from_keys(vec![key(1, "Right Shift", true)]).is_ok());
    }

    #[test]
    fn snapshot_frames_and_fixed_steps_are_checked() {
        assert_eq!(
            parse_snapshot_at("120:out/end.png").unwrap(),
            (120, PathBuf::from("out/end.png"))
        );
        // A drive letter stays part of the path.
        assert_eq!(
            parse_snapshot_at("5:D:/scratch/a.png").unwrap(),
            (5, PathBuf::from("D:/scratch/a.png"))
        );
        assert!(parse_snapshot_at("0:a.png").is_err());
        assert!(parse_snapshot_at("a.png").is_err());
        assert_eq!(parse_fixed_step("16000"), Ok(16_000));
        assert!(parse_fixed_step("7000").is_err());
        assert!(parse_fixed_step("16500").is_err());
    }

    #[test]
    fn the_census_names_entities_that_appeared_and_went() {
        let mut census = EntityCensus::default();
        let first = census.update([(4, 9, [1, 2, 3]), (2, 46, [0, 0, 0])].into_iter());
        assert_eq!(
            first,
            [("spawn", 2, 46, [0, 0, 0]), ("spawn", 4, 9, [1, 2, 3])]
        );
        let second = census.update([(2, 46, [5, 5, 5]), (7, 17, [9, 9, 9])].into_iter());
        assert_eq!(
            second,
            [("spawn", 7, 17, [9, 9, 9]), ("dispose", 4, 9, [1, 2, 3])]
        );
        assert!(census
            .update([(2, 46, [6, 6, 6]), (7, 17, [9, 9, 9])].into_iter())
            .is_empty());
    }

    #[test]
    fn actor_lines_hold_every_entity_with_unresolved_fields_null() {
        let path = std::env::temp_dir().join(format!("v2k-actor-log-{}.jsonl", std::process::id()));
        let mut log = EventLog::create(&path).unwrap();
        log.actors(
            3,
            2,
            [
                (1, 46, [1, -2, 3], Some(40_000), Some(true)),
                (7, 77, [4, 5, 6], None, None),
            ]
            .into_iter(),
        )
        .unwrap();
        log.flush().unwrap();
        drop(log);
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        let line: serde_json::Value = serde_json::from_str(text.lines().nth(1).unwrap()).unwrap();
        assert_eq!(
            line,
            serde_json::json!({
                "frame": 3, "ev": "actors", "tick": 2,
                "actors": [[1, 46, 1, -2, 3, 40000, true], [7, 77, 4, 5, 6, null, null]],
            })
        );
    }

    #[test]
    fn start_states_name_a_save_or_a_level() {
        let level = StartState {
            level: Some(13),
            position: Some([1, 2, 3]),
            heading: Some(0x4000),
            ..StartState::default()
        };
        assert!(level.validate().is_ok());
        let save = StartState {
            save: Some("Slot02".into()),
            ..StartState::default()
        };
        assert!(save.validate().is_ok());
        assert!(StartState::default().validate().is_err());
        assert!(StartState {
            heading: Some(1),
            ..save
        }
        .validate()
        .is_err());
        assert!(StartState {
            level: Some(13),
            heading: Some(1),
            ..StartState::default()
        }
        .validate()
        .is_err());
        assert!(StartState {
            velocity: Some([2, 10, -129]),
            attitude: Some([251, -12]),
            ..level
        }
        .validate()
        .is_ok());
        for motion in [
            StartState {
                velocity: Some([0, 0, 1]),
                ..StartState::default()
            },
            StartState {
                attitude: Some([1, 0]),
                ..StartState::default()
            },
        ] {
            assert!(StartState {
                level: Some(13),
                ..motion
            }
            .validate()
            .is_err());
        }
        assert!(StartState {
            save: Some("Slot02".into()),
            attitude: Some([1, 0]),
            ..StartState::default()
        }
        .validate()
        .is_err());
    }
}
