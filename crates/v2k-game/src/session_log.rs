//! Port-only session logging and replay. `--session-log` copies everything
//! the game prints (stdout and stderr), line by line with the time since
//! launch, into `run-<local time>.log` while the console still shows it.
//! Beside it the run keeps `run-<local time>.replay`, every input the game
//! read from the machine in the order it read them ([`Tape`]), and copies of
//! the settings and saves it started from, so `--replay-session` plays the
//! same session again.
//!
//! The copy works at the operating-system level: the process's standard
//! handles are swapped for pipes that a thread drains. Native code that
//! wrote through the C runtime's own descriptors (SDL's few messages) is not
//! captured.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, BufRead, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use v2k_render::{ControllerSample, GameEvent, PadButton, PadSnapshot};

/// The files of one recorded session, sharing the `run-<stamp>` stem.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionFiles {
    pub log: PathBuf,
    pub replay: PathBuf,
    /// A portable settings folder (`settings.json`, `port-config.json`).
    pub settings: PathBuf,
    /// The save files the run could load ([`crate::save::SaveManager::keep_loadable`]).
    pub saves: PathBuf,
    /// A `--start` state and the save it names.
    pub start: PathBuf,
    pub start_save: PathBuf,
}

impl SessionFiles {
    /// `dir/run-<stamp>.*`.
    pub fn new(dir: &Path, stamp: &str) -> Self {
        Self::from_replay(&dir.join(format!("run-{stamp}.replay")))
    }

    /// The sibling files of a `.replay` path.
    pub fn from_replay(replay: &Path) -> Self {
        let stem = replay.with_extension("");
        let file = |suffix: &str| {
            let mut name = stem.clone().into_os_string();
            name.push(suffix);
            PathBuf::from(name)
        };
        Self {
            log: file(".log"),
            replay: replay.to_path_buf(),
            settings: file(".settings"),
            saves: file(".saves"),
            start: file(".start.json"),
            start_save: file(".start-save"),
        }
    }
}

/// Local wall time as `YYYYMMDD-HHMMSS` (UTC where the platform has no
/// local clock here).
pub fn local_stamp() -> String {
    let [year, month, day, hour, minute, second] = os::local_time();
    format!("{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}")
}

/// A running capture. Dropping it restores the standard handles and waits
/// until every captured byte is in the log.
pub struct SessionLog {
    path: PathBuf,
    streams: Vec<(os::Redirect, JoinHandle<()>)>,
}

impl SessionLog {
    /// Create the log, write `header` and start copying stdout and stderr.
    pub fn start(path: &Path, header: &[String]) -> io::Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut file = File::create(path)?;
        for line in header {
            writeln!(file, "{line}")?;
        }
        writeln!(file)?;
        let file = Arc::new(Mutex::new(file));
        let streams = capture("session-log", Some(&file))?;
        Ok(Self {
            path: path.to_path_buf(),
            streams,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SessionLog {
    fn drop(&mut self) {
        release(&mut self.streams);
    }
}

/// Without a session log the output still passes through the same pipes,
/// echoed to the console, so a crash log can end with the last lines the
/// game printed ([`crate::crash_log`]). Dropping it restores the standard
/// handles once every captured line is remembered.
pub struct OutputTail {
    streams: Vec<(os::Redirect, JoinHandle<()>)>,
}

impl OutputTail {
    pub fn start() -> io::Result<Self> {
        Ok(Self {
            streams: capture("output-tail", None)?,
        })
    }
}

impl Drop for OutputTail {
    fn drop(&mut self) {
        release(&mut self.streams);
    }
}

/// Swap stdout and stderr for pipes that a thread each drains into
/// [`copy_lines`].
fn capture(
    name: &str,
    file: Option<&Arc<Mutex<File>>>,
) -> io::Result<Vec<(os::Redirect, JoinHandle<()>)>> {
    let epoch = Instant::now();
    let mut streams = Vec::new();
    for stream in [os::Stream::Stdout, os::Stream::Stderr] {
        // Rust's stdout buffers whole lines; write them out before the
        // handle changes underneath it.
        io::stdout().flush()?;
        let (redirect, reader, echo) = os::redirect(stream)?;
        let file = file.map(Arc::clone);
        let thread = std::thread::Builder::new()
            .name(format!("{name}-{stream:?}"))
            .spawn(move || copy_lines(reader, echo, file.as_deref(), epoch))?;
        streams.push((redirect, thread));
    }
    Ok(streams)
}

fn release(streams: &mut Vec<(os::Redirect, JoinHandle<()>)>) {
    let _ = io::stdout().flush();
    let _ = io::stderr().flush();
    for (redirect, thread) in streams.drain(..) {
        // Restoring closes the pipe's last writer; the thread then sees
        // the end of the stream.
        drop(redirect);
        let _ = thread.join();
    }
}

/// Echo the raw bytes to the console and each complete line, stamped with
/// the time since launch, to the log (when there is one) and the crash
/// log's last lines. A partial last line is written too.
fn copy_lines(reader: File, mut echo: Option<File>, file: Option<&Mutex<File>>, epoch: Instant) {
    let mut reader = io::BufReader::new(reader);
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        if let Some(echo) = echo.as_mut() {
            let _ = echo.write_all(&line);
            let _ = echo.flush();
        }
        let elapsed = epoch.elapsed();
        let text = String::from_utf8_lossy(&line);
        let stamped = format!(
            "[{:>5}:{:02}.{:03}] {}",
            elapsed.as_secs() / 60,
            elapsed.as_secs() % 60,
            elapsed.subsec_millis(),
            text.trim_end_matches(['\r', '\n'])
        );
        if let Some(Ok(mut file)) = file.map(Mutex::lock) {
            let _ = writeln!(file, "{stamped}");
            let _ = file.flush();
        }
        crate::crash_log::remember(stamped);
    }
}

/// One input of a recorded session, in the order the game read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplayEntry {
    /// A frame's clock: its delta in microseconds, which begins the frame.
    Frame(u64),
    /// A key, with the microseconds into its frame it arrived (the key
    /// reader's on-time).
    Key {
        key: Keycode,
        pressed: bool,
        offset_us: u32,
    },
    MouseMotion {
        x: i32,
        y: i32,
    },
    MouseButton {
        button: MouseButton,
        pressed: bool,
    },
    MouseWheel(i32),
    PadButton {
        button: PadButton,
        pressed: bool,
    },
    /// The window gained (true) or lost focus.
    Focus(bool),
    Resize(u32, u32),
    Quit,
    /// The active controller's reading from this frame on.
    Controller(Option<ControllerSample>),
    /// The intro movie's frame, from the movie's own clock.
    IntroFrame(usize),
    /// Whether the loading card's timer had fallen due when the load
    /// checked it.
    CardDue(bool),
    /// A loading card callback's delta in microseconds.
    CardDelta(u32),
    /// The recording stops here, for this reason.
    End(String),
}

impl ReplayEntry {
    /// A frame's input: what the event poll and the controller give.
    fn is_input(&self) -> bool {
        !matches!(
            self,
            Self::Frame(_)
                | Self::IntroFrame(_)
                | Self::CardDue(_)
                | Self::CardDelta(_)
                | Self::End(_)
        )
    }
}

const REPLAY_MAGIC: &str = "v2k-game session replay 1";

/// Every positional pad control, in [`pad_button_name`]'s spelling.
const PAD_BUTTONS: [PadButton; 20] = [
    PadButton::South,
    PadButton::East,
    PadButton::West,
    PadButton::North,
    PadButton::Back,
    PadButton::Start,
    PadButton::LeftStick,
    PadButton::RightStick,
    PadButton::LeftShoulder,
    PadButton::RightShoulder,
    PadButton::LeftTrigger,
    PadButton::RightTrigger,
    PadButton::DPadUp,
    PadButton::DPadDown,
    PadButton::DPadLeft,
    PadButton::DPadRight,
    PadButton::LeftStickUp,
    PadButton::LeftStickDown,
    PadButton::LeftStickLeft,
    PadButton::LeftStickRight,
];

/// Exhaustive, so a new control fails to build until [`PAD_BUTTONS`] has it.
fn pad_button_name(button: PadButton) -> &'static str {
    match button {
        PadButton::South => "South",
        PadButton::East => "East",
        PadButton::West => "West",
        PadButton::North => "North",
        PadButton::Back => "Back",
        PadButton::Start => "Start",
        PadButton::LeftStick => "LeftStick",
        PadButton::RightStick => "RightStick",
        PadButton::LeftShoulder => "LeftShoulder",
        PadButton::RightShoulder => "RightShoulder",
        PadButton::LeftTrigger => "LeftTrigger",
        PadButton::RightTrigger => "RightTrigger",
        PadButton::DPadUp => "DPadUp",
        PadButton::DPadDown => "DPadDown",
        PadButton::DPadLeft => "DPadLeft",
        PadButton::DPadRight => "DPadRight",
        PadButton::LeftStickUp => "LeftStickUp",
        PadButton::LeftStickDown => "LeftStickDown",
        PadButton::LeftStickLeft => "LeftStickLeft",
        PadButton::LeftStickRight => "LeftStickRight",
    }
}

/// Writes a `.replay` file. The format is text, one entry per line, in the
/// order the session read them:
///
/// ```text
/// v2k-game session replay 1
/// args <the run's command line, as JSON>
/// env <NAME> <value>             a diagnostic variable the run read
/// display <W>x<H>                the resolution the display started at
/// s <microseconds> [repeats]     a frame's clock delta, which begins it
/// k <0|1> <offset> <code> <name> a key up or down, its offset into the
///                                frame in microseconds and SDL keycode
/// m <x> <y>                      relative mouse motion
/// b <0|1> <button>               a mouse button (SDL's number)
/// w <notches>                    the mouse wheel
/// p <0|1> <control>              a game controller's positional control
/// f <0|1>                        the window lost or gained focus
/// r <W> <H>                      the window was resized
/// q                              the window was closed
/// j -                            no controller, from this frame on
/// j pad <6 axes> <buttons>       a pad's reading (buttons as hex bits)
/// j joy <6 axes> <buttons>       a joystick's reading
/// i <frame> [repeats]            the intro movie's frame, by its clock
/// c <0|1>                        whether the loading card's timer was due
/// t <microseconds>               a loading card callback's delta
/// x <reason>                     the recording stops here
/// ```
///
/// A frame's events and controller follow its clock; the controller is
/// written only when its reading changes.
pub struct SessionRecorder {
    out: BufWriter<File>,
    /// A run of equal `s` or `i` values not written yet.
    pending: Option<(char, u64, u64)>,
    controller: Option<Option<ControllerSample>>,
}

impl SessionRecorder {
    pub fn create(path: &Path, args: &[String], env: &[(String, String)]) -> io::Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut out = BufWriter::new(File::create(path)?);
        writeln!(out, "{REPLAY_MAGIC}")?;
        writeln!(
            out,
            "args {}",
            serde_json::to_string(args).map_err(io::Error::other)?
        )?;
        for (name, value) in env {
            writeln!(out, "env {name} {value}")?;
        }
        Ok(Self {
            out,
            pending: None,
            controller: None,
        })
    }

    pub fn display(&mut self, (width, height): (u32, u32)) {
        self.line(format_args!("display {width}x{height}"));
    }

    pub fn entry(&mut self, entry: &ReplayEntry) {
        match entry {
            ReplayEntry::Frame(micros) => return self.repeated('s', *micros),
            ReplayEntry::IntroFrame(frame) => return self.repeated('i', *frame as u64),
            ReplayEntry::Controller(sample) => {
                if self.controller == Some(*sample) {
                    return;
                }
                self.controller = Some(*sample);
            }
            _ => {}
        }
        let line = match entry {
            ReplayEntry::Key {
                key,
                pressed,
                offset_us,
            } => format!(
                "k {} {offset_us} {} {}",
                u8::from(*pressed),
                key.into_i32(),
                key.name()
            ),
            ReplayEntry::MouseMotion { x, y } => format!("m {x} {y}"),
            ReplayEntry::MouseButton { button, pressed } => {
                format!("b {} {}", u8::from(*pressed), *button as u8)
            }
            ReplayEntry::MouseWheel(y) => format!("w {y}"),
            ReplayEntry::PadButton { button, pressed } => {
                format!("p {} {}", u8::from(*pressed), pad_button_name(*button))
            }
            ReplayEntry::Focus(gained) => format!("f {}", u8::from(*gained)),
            ReplayEntry::Resize(width, height) => format!("r {width} {height}"),
            ReplayEntry::Quit => "q".to_string(),
            ReplayEntry::Controller(None) => "j -".to_string(),
            ReplayEntry::Controller(Some(ControllerSample::Pad(pad))) => {
                let buttons = pad
                    .buttons
                    .iter()
                    .enumerate()
                    .fold(0u32, |bits, (index, &down)| bits | u32::from(down) << index);
                format!(
                    "j pad {} {} {} {} {} {} {buttons:x}",
                    pad.left_x,
                    pad.left_y,
                    pad.right_x,
                    pad.right_y,
                    pad.left_trigger,
                    pad.right_trigger
                )
            }
            ReplayEntry::Controller(Some(ControllerSample::Joystick {
                axes: [a, b, c, d, e, f],
                buttons,
            })) => format!("j joy {a} {b} {c} {d} {e} {f} {buttons:x}"),
            ReplayEntry::CardDue(due) => format!("c {}", u8::from(*due)),
            ReplayEntry::CardDelta(micros) => format!("t {micros}"),
            ReplayEntry::End(reason) => format!("x {reason}"),
            ReplayEntry::Frame(_) | ReplayEntry::IntroFrame(_) => unreachable!(),
        };
        self.line(format_args!("{line}"));
    }

    /// Push everything written so far to the file, so a crash loses
    /// nothing before it.
    pub fn flush(&mut self) {
        self.flush_pending();
        let _ = self.out.flush();
    }

    fn repeated(&mut self, tag: char, value: u64) {
        match &mut self.pending {
            Some((pending, pending_value, count)) if *pending == tag && *pending_value == value => {
                *count += 1
            }
            _ => {
                self.flush_pending();
                self.pending = Some((tag, value, 1));
            }
        }
    }

    fn line(&mut self, line: std::fmt::Arguments<'_>) {
        self.flush_pending();
        let _ = writeln!(self.out, "{line}");
    }

    fn flush_pending(&mut self) {
        if let Some((tag, value, count)) = self.pending.take() {
            let _ = if count == 1 {
                writeln!(self.out, "{tag} {value}")
            } else {
                writeln!(self.out, "{tag} {value} {count}")
            };
        }
    }
}

impl Drop for SessionRecorder {
    fn drop(&mut self) {
        self.flush();
    }
}

/// A parsed `.replay` file.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionReplay {
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub display: Option<(u32, u32)>,
    pub entries: Vec<ReplayEntry>,
}

impl SessionReplay {
    pub fn read(path: &Path) -> io::Result<Self> {
        let mut text = String::new();
        File::open(path)?.read_to_string(&mut text)?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> io::Result<Self> {
        let mut lines = text.lines().enumerate();
        match lines.next() {
            Some((_, magic)) if magic.trim_end() == REPLAY_MAGIC => {}
            _ => return Err(invalid(0, "not a v2k-game session replay")),
        }
        let mut replay = Self {
            args: Vec::new(),
            env: Vec::new(),
            display: None,
            entries: Vec::new(),
        };
        for (index, line) in lines {
            let line = line.trim_end_matches('\r');
            let (tag, rest) = line.split_once(' ').unwrap_or((line, ""));
            let bad = |why: &str| invalid(index, why);
            match tag {
                "" => {}
                "args" => {
                    replay.args =
                        serde_json::from_str(rest).map_err(|_| bad("bad argument list"))?;
                }
                "env" => {
                    let (name, value) = rest.split_once(' ').ok_or_else(|| bad("bad variable"))?;
                    replay.env.push((name.to_string(), value.to_string()));
                }
                "display" => {
                    replay.display = Some(
                        rest.split_once('x')
                            .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
                            .ok_or_else(|| bad("bad display"))?,
                    );
                }
                "s" | "i" => {
                    let mut fields = rest.split(' ');
                    let value: u64 = number(fields.next()).ok_or_else(|| bad("bad value"))?;
                    let count: u64 = match fields.next() {
                        Some(count) => count.parse().map_err(|_| bad("bad repeat count"))?,
                        None => 1,
                    };
                    let entry = if tag == "s" {
                        ReplayEntry::Frame(value)
                    } else {
                        ReplayEntry::IntroFrame(value as usize)
                    };
                    replay.entries.extend((0..count).map(|_| entry.clone()));
                }
                "x" => replay.entries.push(ReplayEntry::End(rest.to_string())),
                _ => replay
                    .entries
                    .push(parse_entry(tag, rest).ok_or_else(|| bad("bad or unknown entry"))?),
            }
        }
        Ok(replay)
    }
}

fn invalid(line: usize, why: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("replay line {}: {why}", line + 1),
    )
}

fn number<T: std::str::FromStr>(field: Option<&str>) -> Option<T> {
    field?.parse().ok()
}

fn flag(field: Option<&str>) -> Option<bool> {
    match field? {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

fn parse_entry(tag: &str, rest: &str) -> Option<ReplayEntry> {
    let mut fields = rest.split_whitespace();
    let entry = match tag {
        "k" => ReplayEntry::Key {
            pressed: flag(fields.next())?,
            offset_us: number(fields.next())?,
            key: Keycode::from_i32(number(fields.next())?)?,
        },
        "m" => ReplayEntry::MouseMotion {
            x: number(fields.next())?,
            y: number(fields.next())?,
        },
        "b" => ReplayEntry::MouseButton {
            pressed: flag(fields.next())?,
            button: MouseButton::from_ll(number(fields.next())?),
        },
        "w" => ReplayEntry::MouseWheel(number(fields.next())?),
        "p" => {
            let pressed = flag(fields.next())?;
            let name = fields.next()?;
            ReplayEntry::PadButton {
                button: *PAD_BUTTONS
                    .iter()
                    .find(|button| pad_button_name(**button) == name)?,
                pressed,
            }
        }
        "f" => ReplayEntry::Focus(flag(fields.next())?),
        "r" => ReplayEntry::Resize(number(fields.next())?, number(fields.next())?),
        "q" => ReplayEntry::Quit,
        "c" => ReplayEntry::CardDue(flag(fields.next())?),
        "t" => ReplayEntry::CardDelta(number(fields.next())?),
        "j" => {
            let kind = fields.next()?;
            if kind == "-" {
                return Some(ReplayEntry::Controller(None));
            }
            let mut axes = [0i16; 6];
            for axis in &mut axes {
                *axis = number(fields.next())?;
            }
            let buttons = u32::from_str_radix(fields.next()?, 16).ok()?;
            ReplayEntry::Controller(Some(match kind {
                "pad" => {
                    let [left_x, left_y, right_x, right_y, left_trigger, right_trigger] = axes;
                    ControllerSample::Pad(PadSnapshot {
                        left_x,
                        left_y,
                        right_x,
                        right_y,
                        left_trigger,
                        right_trigger,
                        buttons: std::array::from_fn(|index| buttons >> index & 1 != 0),
                    })
                }
                "joy" => ControllerSample::Joystick { axes, buttons },
                _ => return None,
            }))
        }
        _ => return None,
    };
    // A key's name follows its code and is only for the reader.
    (tag == "k" || fields.next().is_none()).then_some(entry)
}

/// A frame's clock from [`Tape::frame`].
pub enum TapeFrame {
    Delta(u64),
    /// `--resume`: the replay has ended and the host takes over; restart
    /// the frame pump and ask again.
    Resumed,
    /// The replay has ended.
    Ended,
}

/// The game's inputs from the machine: its frame clock, events, controller,
/// the intro movie's clock and the loading card's timer. Live, it passes the
/// host's through and, recording, writes them to the `.replay`; replaying,
/// it hands back the recording's instead, in the same order.
#[derive(Default)]
pub struct Tape {
    recorder: Option<SessionRecorder>,
    replay: Option<ReplayCursor>,
    /// `--resume`: the host takes over when the replay runs out.
    resume: bool,
}

struct ReplayCursor {
    entries: VecDeque<ReplayEntry>,
    /// Entries read, for the messages.
    read: usize,
    frames: u64,
    controller: Option<ControllerSample>,
    /// Why the replay ended in the middle of a frame; the next frame
    /// reports it.
    ended: Option<String>,
}

impl Tape {
    pub fn new(
        recorder: Option<SessionRecorder>,
        replay: Option<SessionReplay>,
        resume: bool,
    ) -> Self {
        Self {
            recorder,
            replay: replay.map(|replay| ReplayCursor {
                entries: replay.entries.into(),
                read: 0,
                frames: 0,
                controller: None,
                ended: None,
            }),
            resume,
        }
    }

    pub fn is_replaying(&self) -> bool {
        self.replay.is_some()
    }

    pub fn display(&mut self, size: (u32, u32)) {
        if let Some(recorder) = self.recorder.as_mut() {
            recorder.display(size);
        }
    }

    /// End the recording here: a session this cannot replay (a network
    /// game) goes on unrecorded.
    pub fn stop_recording(&mut self, reason: &str) {
        if let Some(mut recorder) = self.recorder.take() {
            eprintln!("Session recording stops: {reason}");
            recorder.entry(&ReplayEntry::End(reason.to_string()));
        }
    }

    /// The next frame's clock: the recording's, else `live`'s.
    pub fn frame(&mut self, live: impl FnOnce() -> u64) -> TapeFrame {
        let delta = match self.replay.as_mut() {
            Some(replay) => {
                let why = match replay.ended.take() {
                    Some(why) => why,
                    None => match replay.entries.pop_front() {
                        Some(ReplayEntry::Frame(delta)) => {
                            replay.read += 1;
                            replay.frames += 1;
                            self.record(&ReplayEntry::Frame(delta));
                            return TapeFrame::Delta(delta);
                        }
                        Some(ReplayEntry::End(reason)) => {
                            format!("the recording stopped there: {reason}")
                        }
                        Some(other) => replay.parted(&format!("{other:?}"), "the next frame"),
                        None => "the recording ends there".to_string(),
                    },
                };
                eprintln!(
                    "Session replay: {why}, after {} frames{}",
                    replay.frames,
                    if self.resume {
                        "; the clock, keys and controllers are live from here (--resume)"
                    } else {
                        ""
                    }
                );
                self.replay = None;
                return if self.resume {
                    TapeFrame::Resumed
                } else {
                    TapeFrame::Ended
                };
            }
            None => live(),
        };
        self.record(&ReplayEntry::Frame(delta));
        TapeFrame::Delta(delta)
    }

    /// The frame's events and their keys' offsets into the frame, in the
    /// order the game reads them. Replaying, the recording's replace the
    /// host's, of which only closing the window is kept. `auxiliary`
    /// names the windows besides the game's (the debug panel), whose clicks
    /// and closing never reach the game and are not recorded.
    pub fn frame_input(
        &mut self,
        live: Vec<GameEvent>,
        live_keys: Vec<(Keycode, bool, u32)>,
        auxiliary: impl Fn(u32) -> bool,
    ) -> (Vec<GameEvent>, Vec<(Keycode, bool, u32)>) {
        let (events, keys) = match self.replay.as_mut() {
            None => (live, live_keys),
            Some(replay) => {
                let mut events = Vec::new();
                let mut keys = Vec::new();
                while replay.entries.front().is_some_and(ReplayEntry::is_input) {
                    replay.read += 1;
                    match replay.entries.pop_front().expect("checked front") {
                        ReplayEntry::Key {
                            key,
                            pressed,
                            offset_us,
                        } => {
                            events.push(if pressed {
                                GameEvent::KeyDown(key)
                            } else {
                                GameEvent::KeyUp(key)
                            });
                            keys.push((key, pressed, offset_us));
                        }
                        ReplayEntry::MouseMotion { x, y } => {
                            events.push(GameEvent::MouseMotion { xrel: x, yrel: y })
                        }
                        ReplayEntry::MouseButton { button, pressed } => events.push(if pressed {
                            GameEvent::MouseButtonDown(button)
                        } else {
                            GameEvent::MouseButtonUp(button)
                        }),
                        ReplayEntry::MouseWheel(y) => events.push(GameEvent::MouseWheel { y }),
                        ReplayEntry::PadButton { button, pressed } => {
                            events.push(GameEvent::PadButton { button, pressed })
                        }
                        ReplayEntry::Focus(true) => events.push(GameEvent::FocusGained),
                        ReplayEntry::Focus(false) => events.push(GameEvent::FocusLost),
                        ReplayEntry::Resize(width, height) => {
                            events.push(GameEvent::Resize(width, height))
                        }
                        ReplayEntry::Quit => events.push(GameEvent::Quit),
                        ReplayEntry::Controller(sample) => replay.controller = sample,
                        _ => unreachable!("not an input"),
                    }
                }
                if live.iter().any(|event| matches!(event, GameEvent::Quit)) {
                    events.push(GameEvent::Quit);
                }
                (events, keys)
            }
        };
        if let Some(recorder) = self.recorder.as_mut() {
            let mut offsets = keys.iter();
            for event in &events {
                let entry = match *event {
                    GameEvent::KeyDown(_) | GameEvent::KeyUp(_) => {
                        let &(key, pressed, offset_us) =
                            offsets.next().expect("one offset for each key event");
                        ReplayEntry::Key {
                            key,
                            pressed,
                            offset_us,
                        }
                    }
                    GameEvent::MouseMotion { xrel, yrel } => {
                        ReplayEntry::MouseMotion { x: xrel, y: yrel }
                    }
                    GameEvent::MouseButtonDown(button) => ReplayEntry::MouseButton {
                        button,
                        pressed: true,
                    },
                    GameEvent::MouseButtonUp(button) => ReplayEntry::MouseButton {
                        button,
                        pressed: false,
                    },
                    GameEvent::MouseWheel { y } => ReplayEntry::MouseWheel(y),
                    GameEvent::PadButton { button, pressed } => {
                        ReplayEntry::PadButton { button, pressed }
                    }
                    GameEvent::FocusGained => ReplayEntry::Focus(true),
                    GameEvent::FocusLost => ReplayEntry::Focus(false),
                    GameEvent::Resize(width, height) => ReplayEntry::Resize(width, height),
                    GameEvent::Quit => ReplayEntry::Quit,
                    // The game's own window closing quits it.
                    GameEvent::WindowClosed(id) if !auxiliary(id) => ReplayEntry::Quit,
                    GameEvent::WindowClosed(_) | GameEvent::AuxiliaryWindowClick { .. } => continue,
                };
                recorder.entry(&entry);
            }
            recorder.flush();
        }
        (events, keys)
    }

    /// The active controller's reading this frame.
    pub fn controller(&mut self, live: Option<ControllerSample>) -> Option<ControllerSample> {
        let sample = match &self.replay {
            Some(replay) => replay.controller,
            None => live,
        };
        self.record(&ReplayEntry::Controller(sample));
        sample
    }

    /// The intro movie's frame by its clock.
    pub fn intro_frame(&mut self, live: impl FnOnce() -> usize) -> usize {
        let frame = self
            .replayed("the intro movie's frame", |entry| match entry {
                ReplayEntry::IntroFrame(frame) => Some(*frame),
                _ => None,
            })
            .unwrap_or_else(live);
        self.record(&ReplayEntry::IntroFrame(frame));
        frame
    }

    /// Whether the loading card's timer has fallen due.
    pub fn card_due(&mut self, live: impl FnOnce() -> bool) -> bool {
        let due = self
            .replayed("the loading card's timer", |entry| match entry {
                ReplayEntry::CardDue(due) => Some(*due),
                _ => None,
            })
            .unwrap_or_else(live);
        self.record(&ReplayEntry::CardDue(due));
        due
    }

    /// A loading card callback's delta.
    pub fn card_delta(&mut self, live: impl FnOnce() -> u32) -> u32 {
        let delta = self
            .replayed("a loading card callback", |entry| match entry {
                ReplayEntry::CardDelta(delta) => Some(*delta),
                _ => None,
            })
            .unwrap_or_else(live);
        self.record(&ReplayEntry::CardDelta(delta));
        delta
    }

    /// The recording's next entry, when it is `what`. Anything else ends the
    /// replay at the next frame, and this read takes the host's value.
    fn replayed<T>(&mut self, what: &str, take: impl Fn(&ReplayEntry) -> Option<T>) -> Option<T> {
        let replay = self.replay.as_mut()?;
        if replay.ended.is_some() {
            return None;
        }
        match replay.entries.front().map(&take) {
            Some(Some(value)) => {
                replay.entries.pop_front();
                replay.read += 1;
                Some(value)
            }
            Some(None) => {
                let found = format!("{:?}", replay.entries.front().expect("checked front"));
                replay.ended = Some(replay.parted(&found, what));
                None
            }
            None => {
                replay.ended = Some(format!("the recording ends before {what}"));
                None
            }
        }
    }

    fn record(&mut self, entry: &ReplayEntry) {
        if let Some(recorder) = self.recorder.as_mut() {
            recorder.entry(entry);
        }
    }
}

impl ReplayCursor {
    fn parted(&self, found: &str, expected: &str) -> String {
        format!(
            "the game went another way than the recording at its entry {}: it read {expected} where the recording holds {found}",
            self.read + 1
        )
    }
}

#[cfg(windows)]
mod os {
    use std::ffi::c_void;
    use std::fs::File;
    use std::io;
    use std::os::windows::io::FromRawHandle;

    type Handle = *mut c_void;
    const INVALID_HANDLE: Handle = -1isize as Handle;

    #[repr(C)]
    #[derive(Default)]
    struct SystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        milliseconds: u16,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(which: u32) -> Handle;
        fn SetStdHandle(which: u32, handle: Handle) -> i32;
        fn CreatePipe(
            read: *mut Handle,
            write: *mut Handle,
            attributes: *mut c_void,
            size: u32,
        ) -> i32;
        fn GetLocalTime(time: *mut SystemTime);
    }

    #[derive(Clone, Copy, Debug)]
    pub enum Stream {
        Stdout,
        Stderr,
    }

    impl Stream {
        fn id(self) -> u32 {
            match self {
                Self::Stdout => -11i32 as u32,
                Self::Stderr => -12i32 as u32,
            }
        }
    }

    /// Restores the original handle on drop, then closes the pipe's writer.
    pub struct Redirect {
        stream: Stream,
        original: Handle,
        writer: Option<File>,
    }

    // The handles are process-wide values, not thread-bound.
    unsafe impl Send for Redirect {}

    impl Drop for Redirect {
        fn drop(&mut self) {
            unsafe {
                SetStdHandle(self.stream.id(), self.original);
            }
            self.writer.take();
        }
    }

    pub fn redirect(stream: Stream) -> io::Result<(Redirect, File, Option<File>)> {
        unsafe {
            let original = GetStdHandle(stream.id());
            let (mut read, mut write) = (std::ptr::null_mut(), std::ptr::null_mut());
            if CreatePipe(&mut read, &mut write, std::ptr::null_mut(), 0) == 0 {
                return Err(io::Error::last_os_error());
            }
            if SetStdHandle(stream.id(), write) == 0 {
                let error = io::Error::last_os_error();
                drop(File::from_raw_handle(read));
                drop(File::from_raw_handle(write));
                return Err(error);
            }
            // The console (if any) keeps receiving the output. The echo is
            // a second owner of a handle the process keeps; duplicating it
            // keeps the original open when the echo closes.
            let echo = if original.is_null() || original == INVALID_HANDLE {
                None
            } else {
                duplicate(original)
            };
            Ok((
                Redirect {
                    stream,
                    original,
                    writer: Some(File::from_raw_handle(write)),
                },
                File::from_raw_handle(read),
                echo,
            ))
        }
    }

    fn duplicate(handle: Handle) -> Option<File> {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetCurrentProcess() -> Handle;
            fn DuplicateHandle(
                source_process: Handle,
                source: Handle,
                target_process: Handle,
                target: *mut Handle,
                access: u32,
                inherit: i32,
                options: u32,
            ) -> i32;
        }
        const DUPLICATE_SAME_ACCESS: u32 = 2;
        unsafe {
            let process = GetCurrentProcess();
            let mut copy = std::ptr::null_mut();
            if DuplicateHandle(
                process,
                handle,
                process,
                &mut copy,
                0,
                0,
                DUPLICATE_SAME_ACCESS,
            ) == 0
            {
                return None;
            }
            Some(File::from_raw_handle(copy))
        }
    }

    pub fn local_time() -> [u16; 6] {
        let mut time = SystemTime::default();
        unsafe { GetLocalTime(&mut time) };
        [
            time.year,
            time.month,
            time.day,
            time.hour,
            time.minute,
            time.second,
        ]
    }
}

#[cfg(unix)]
mod os {
    use std::fs::File;
    use std::io;
    use std::os::unix::io::FromRawFd;

    extern "C" {
        fn pipe(fds: *mut i32) -> i32;
        fn dup(fd: i32) -> i32;
        fn dup2(from: i32, to: i32) -> i32;
        fn close(fd: i32) -> i32;
    }

    #[derive(Clone, Copy, Debug)]
    pub enum Stream {
        Stdout,
        Stderr,
    }

    impl Stream {
        fn fd(self) -> i32 {
            match self {
                Self::Stdout => 1,
                Self::Stderr => 2,
            }
        }
    }

    /// Points the descriptor back at the original on drop, which closes the
    /// pipe's last writer.
    pub struct Redirect {
        stream: Stream,
        original: i32,
    }

    impl Drop for Redirect {
        fn drop(&mut self) {
            unsafe {
                dup2(self.original, self.stream.fd());
                close(self.original);
            }
        }
    }

    pub fn redirect(stream: Stream) -> io::Result<(Redirect, File, Option<File>)> {
        unsafe {
            let original = dup(stream.fd());
            if original < 0 {
                return Err(io::Error::last_os_error());
            }
            let mut fds = [0; 2];
            if pipe(fds.as_mut_ptr()) != 0 {
                let error = io::Error::last_os_error();
                close(original);
                return Err(error);
            }
            if dup2(fds[1], stream.fd()) < 0 {
                let error = io::Error::last_os_error();
                close(fds[0]);
                close(fds[1]);
                close(original);
                return Err(error);
            }
            close(fds[1]);
            let echo = match dup(original) {
                fd if fd >= 0 => Some(File::from_raw_fd(fd)),
                _ => None,
            };
            Ok((
                Redirect { stream, original },
                File::from_raw_fd(fds[0]),
                echo,
            ))
        }
    }

    /// UTC: the standard library has no local time zone.
    pub fn local_time() -> [u16; 6] {
        let seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs() as i64);
        let days = seconds.div_euclid(86_400);
        let rest = seconds.rem_euclid(86_400);
        // Howard Hinnant's civil_from_days.
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = yoe + era * 400 + i64::from(month <= 2);
        [
            year as u16,
            month as u16,
            day as u16,
            (rest / 3600) as u16,
            (rest / 60 % 60) as u16,
            (rest % 60) as u16,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad() -> ControllerSample {
        ControllerSample::Pad(PadSnapshot {
            left_x: -32768,
            left_y: 12,
            right_x: 0,
            right_y: 32767,
            left_trigger: 0,
            right_trigger: 4000,
            buttons: [
                true, false, false, true, false, false, false, false, false, true,
            ],
        })
    }

    /// Every entry kind is written and read back in session order; runs of
    /// equal clocks and an unchanged controller take one line.
    #[test]
    fn a_recording_reads_back_in_session_order() {
        let dir = std::env::temp_dir().join(format!("v2k-session-log-{}", std::process::id()));
        let path = dir.join("run.replay");
        let entries = vec![
            ReplayEntry::Frame(8_000),
            ReplayEntry::Controller(None),
            ReplayEntry::IntroFrame(0),
            ReplayEntry::IntroFrame(0),
            ReplayEntry::Frame(16_000),
            ReplayEntry::Frame(16_000),
            ReplayEntry::Key {
                key: Keycode::RShift,
                pressed: true,
                offset_us: 4_321,
            },
            ReplayEntry::MouseMotion { x: -3, y: 7 },
            ReplayEntry::MouseButton {
                button: MouseButton::Right,
                pressed: true,
            },
            ReplayEntry::MouseWheel(-1),
            ReplayEntry::PadButton {
                button: PadButton::LeftStickRight,
                pressed: false,
            },
            ReplayEntry::Focus(false),
            ReplayEntry::Resize(1024, 768),
            ReplayEntry::Controller(Some(pad())),
            ReplayEntry::CardDue(true),
            ReplayEntry::CardDelta(50_000),
            ReplayEntry::Frame(9_000),
            ReplayEntry::Controller(Some(pad())),
            ReplayEntry::Controller(Some(ControllerSample::Joystick {
                axes: [1, -2, 3, -4, 5, -6],
                buttons: 0x8000_0001,
            })),
            ReplayEntry::Quit,
            ReplayEntry::End("network play".into()),
        ];
        {
            let mut recorder = SessionRecorder::create(
                &path,
                &["v2k-game".into(), "--hidden".into()],
                &[("V2K_NEW_GAME_AFTER_TICKS".into(), "40".into())],
            )
            .unwrap();
            recorder.display((1280, 960));
            for entry in &entries {
                recorder.entry(entry);
            }
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let replay = SessionReplay::read(&path).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(text.contains("\ns 16000 2\n"), "{text}");
        assert!(text.contains("\ni 0 2\n"), "{text}");
        assert!(text.contains("\nk 1 4321 "), "{text}");
        assert_eq!(replay.args, ["v2k-game", "--hidden"]);
        assert_eq!(
            replay.env,
            [("V2K_NEW_GAME_AFTER_TICKS".into(), "40".into())]
        );
        assert_eq!(replay.display, Some((1280, 960)));
        let mut expected = entries;
        // The repeated pad reading is not written again.
        expected.remove(17);
        assert_eq!(replay.entries, expected);
    }

    #[test]
    fn a_replay_rejects_other_files_and_bad_entries() {
        assert!(SessionReplay::parse("{}").is_err());
        for bad in [
            "k 2 0 97 A",
            "f",
            "j pad 1 2 3",
            "p 1 Nowhere",
            "s x",
            "z 1",
            "q 1",
        ] {
            assert!(
                SessionReplay::parse(&format!("{REPLAY_MAGIC}\n{bad}")).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn replay_siblings_share_the_run_stem() {
        let files = SessionFiles::from_replay(Path::new("logs/run-20261110-201500.replay"));
        assert_eq!(files.log, Path::new("logs/run-20261110-201500.log"));
        assert_eq!(files.saves, Path::new("logs/run-20261110-201500.saves"));
        assert_eq!(
            files,
            SessionFiles::new(Path::new("logs"), "20261110-201500")
        );
    }

    /// Replaying hands back the recording in order and ends where it does;
    /// an entry of another kind than the game asks for ends it too.
    #[test]
    fn a_replay_feeds_the_recording_back_and_stops_where_it_parts() {
        let replay = |entries: Vec<ReplayEntry>| SessionReplay {
            args: Vec::new(),
            env: Vec::new(),
            display: None,
            entries,
        };
        let mut tape = Tape::new(
            None,
            Some(replay(vec![
                ReplayEntry::Frame(8_000),
                ReplayEntry::Key {
                    key: Keycode::Space,
                    pressed: true,
                    offset_us: 7,
                },
                ReplayEntry::Controller(Some(pad())),
                ReplayEntry::CardDue(false),
                ReplayEntry::Frame(12_000),
            ])),
            false,
        );
        assert!(matches!(tape.frame(|| 1), TapeFrame::Delta(8_000)));
        let (events, keys) = tape.frame_input(
            vec![GameEvent::KeyDown(Keycode::A)],
            vec![(Keycode::A, true, 0)],
            |_| false,
        );
        assert!(matches!(
            events.as_slice(),
            [GameEvent::KeyDown(Keycode::Space)]
        ));
        assert_eq!(keys, [(Keycode::Space, true, 7)]);
        assert_eq!(tape.controller(None), Some(pad()));
        assert!(!tape.card_due(|| true));
        assert!(matches!(tape.frame(|| 1), TapeFrame::Delta(12_000)));
        assert!(matches!(tape.frame(|| 1), TapeFrame::Ended));
        assert!(!tape.is_replaying());
        assert!(matches!(tape.frame(|| 16_000), TapeFrame::Delta(16_000)));

        let mut tape = Tape::new(None, Some(replay(vec![ReplayEntry::CardDelta(9)])), true);
        // The card's delta stands where the intro's frame was asked for.
        assert_eq!(tape.intro_frame(|| 3), 3);
        assert!(matches!(tape.frame(|| 1), TapeFrame::Resumed));
        assert!(matches!(tape.frame(|| 16_000), TapeFrame::Delta(16_000)));
    }
}
