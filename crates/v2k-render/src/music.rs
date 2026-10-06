//! CD audio music player — bounded OGG Vorbis and PCM WAV streaming + SDL2.
//!
//! Music uses a dedicated SDL2 audio device (separate from `SoundManager`) so
//! music and effects mix at the hardware level. A worker reads short PCM or Vorbis
//! packets into a bounded queue; the real-time SDL callback only consumes that
//! queue and emits silence on contention or underflow. Track selection parses
//! only the stream headers and therefore never materializes a whole CD track as
//! PCM during a level load.
//!
//! Retail CD audio is binary: decoded music is played at unity, while the
//! Ambient setting only pauses or resumes the selected track. Pausing keeps
//! the queued playback position; stopping rewinds and releases the selection.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, TryLockError};
use std::thread::{self, JoinHandle};

use lewton::inside_ogg::OggStreamReader;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use sdl2::Sdl;

mod inventory;
mod wav;

pub use inventory::{
    inspect_soundtrack, locate_soundtrack, validate_cd_audio_track, SoundtrackAvailability,
    SoundtrackInventory, SoundtrackTrack, CD_AUDIO_TRACK_COUNT, FIRST_CD_TRACK, LAST_CD_TRACK,
};

const OUTPUT_SAMPLE_RATE: u32 = 44_100;
const OUTPUT_CHANNELS: u16 = 2;
const STREAM_BUFFER_SECONDS: usize = 4;
const STREAM_BUFFER_SAMPLES: usize =
    OUTPUT_SAMPLE_RATE as usize * OUTPUT_CHANNELS as usize * STREAM_BUFFER_SECONDS;

type VorbisReader = OggStreamReader<BufReader<File>>;

/// State shared by the main thread, decoder worker, and SDL callback.
struct MusicState {
    /// Bounded stereo i16 queue consumed by the SDL callback.
    samples: VecDeque<i16>,
    /// The decoder worker's desired source, or `None` after stop.
    selected_path: Option<PathBuf>,
    /// Invalidates decoder work whenever selection changes or playback stops.
    generation: u64,
    /// Prevents a corrupt selection from being reopened in a busy loop.
    failed_generation: Option<u64>,
    /// Ambient's binary playback gate. Selection itself starts paused.
    playing: bool,
    /// Requests clean worker shutdown from `MusicPlayer::drop`.
    shutdown: bool,
}

impl Default for MusicState {
    fn default() -> Self {
        Self {
            samples: VecDeque::with_capacity(STREAM_BUFFER_SAMPLES),
            selected_path: None,
            generation: 0,
            failed_generation: None,
            playing: false,
            shutdown: false,
        }
    }
}

impl MusicState {
    /// Select and rewind a source without starting playback.
    fn select(&mut self, path: PathBuf) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.samples.clear();
        self.selected_path = Some(path);
        self.failed_generation = None;
        self.playing = false;
        self.generation
    }

    /// Resume a valid selection. An empty queue means buffering, not stopped.
    fn resume(&mut self) {
        self.playing =
            self.selected_path.is_some() && self.failed_generation != Some(self.generation);
    }

    /// Pause without changing the selected track or buffered playback point.
    fn pause(&mut self) {
        self.playing = false;
    }

    /// Stop, rewind, and invalidate any packet currently being decoded.
    fn stop(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.samples.clear();
        self.selected_path = None;
        self.failed_generation = None;
        self.playing = false;
    }

    /// Append current-generation PCM without exceeding the fixed queue bound.
    fn enqueue(&mut self, generation: u64, samples: &[i16]) -> usize {
        if generation != self.generation
            || self.selected_path.is_none()
            || self.failed_generation == Some(generation)
        {
            return 0;
        }

        let count = samples
            .len()
            .min(STREAM_BUFFER_SAMPLES.saturating_sub(self.samples.len()));
        self.samples.extend(samples[..count].iter().copied());
        count
    }

    /// Copy queued PCM to a pre-zeroed callback buffer.
    fn write_output(&mut self, out: &mut [i16]) -> bool {
        if !self.playing {
            return false;
        }

        let mut consumed = false;
        for sample in out {
            let Some(decoded) = self.samples.pop_front() else {
                break;
            };
            *sample = decoded;
            consumed = true;
        }
        consumed
    }
}

struct SharedMusic {
    state: Mutex<MusicState>,
    decoder_wake: Condvar,
}

impl Default for SharedMusic {
    fn default() -> Self {
        Self {
            state: Mutex::new(MusicState::default()),
            decoder_wake: Condvar::new(),
        }
    }
}

impl SharedMusic {
    fn lock(&self) -> MutexGuard<'_, MusicState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Validate outside the callback lock, then commit a selection or a stop.
    /// Every failed physical-track request releases the previous source.
    fn select_cd_track(&self, tracks: &SoundtrackInventory, cd_track: u8) -> Result<(), String> {
        let selected = tracks
            .track(cd_track)
            .ok_or_else(|| format!("CD track {cd_track} out of range (expected 2..11)"))
            .and_then(|track| {
                track
                    .path
                    .clone()
                    .ok_or_else(|| format!("CD track {cd_track:02} is unavailable"))
            })
            .and_then(|path| open_decoder(&path).map(|_| path));
        let result = match selected {
            Ok(path) => {
                self.lock().select(path);
                Ok(())
            }
            Err(error) => {
                self.lock().stop();
                Err(error)
            }
        };
        self.decoder_wake.notify_one();
        result
    }
    fn wait<'a>(&self, state: MutexGuard<'a, MusicState>) -> MutexGuard<'a, MusicState> {
        self.decoder_wake
            .wait(state)
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// SDL2 callback: no file I/O, decoding, allocation, or blocking locks.
struct MusicCallback {
    shared: Arc<SharedMusic>,
}

impl AudioCallback for MusicCallback {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        out.fill(0);
        let mut state = match self.shared.state.try_lock() {
            Ok(state) => state,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => return,
        };

        if state.write_output(out) {
            self.shared.decoder_wake.notify_one();
        }
    }
}

/// Music player for the bundled 44.1-kHz stereo CD-audio tracks.
pub struct MusicPlayer {
    _device: AudioDevice<MusicCallback>,
    shared: Arc<SharedMusic>,
    decoder_worker: Option<JoinHandle<()>>,
    /// Fixed physical CD-track slots, preserving missing-track identities.
    tracks: SoundtrackInventory,
    /// Original physical CD identity, including while Ambient has paused it.
    current_cd_track: Option<u8>,
}

impl MusicPlayer {
    /// Create a player and one persistent, bounded decoder worker.
    pub fn new(sdl: &Sdl, music_dir: &Path) -> Result<Self, String> {
        let audio = sdl.audio()?;
        let shared = Arc::new(SharedMusic::default());

        let desired = AudioSpecDesired {
            freq: Some(OUTPUT_SAMPLE_RATE as i32),
            channels: Some(OUTPUT_CHANNELS as u8),
            samples: Some(4096),
        };

        let callback_shared = Arc::clone(&shared);
        let device = audio.open_playback(None, &desired, |_spec| MusicCallback {
            shared: callback_shared,
        })?;

        let tracks = inspect_soundtrack(music_dir);

        let worker_shared = Arc::clone(&shared);
        let decoder_worker = thread::Builder::new()
            .name("v2k-music-decoder".to_owned())
            .spawn(move || decoder_worker(worker_shared))
            .map_err(|error| format!("Failed to start music decoder: {error}"))?;

        // It emits silence until Ambient resumes a selected track.
        device.resume();

        Ok(Self {
            _device: device,
            shared,
            decoder_worker: Some(decoder_worker),
            tracks,
            current_cd_track: None,
        })
    }

    /// Number of available tracks.
    pub fn track_count(&self) -> usize {
        self.tracks.available_count()
    }

    /// Select an original physical CD track, rewound and initially paused.
    ///
    /// Missing, corrupt, or unsupported sources stop the previous track; physical
    /// identities never shift. Only headers are parsed on the loading thread.
    /// PCM reading, Vorbis decoding and looping remain bounded worker tasks.
    pub fn select_cd_track(&mut self, cd_track: u8) -> Result<(), String> {
        self.current_cd_track = None;
        self.shared.select_cd_track(&self.tracks, cd_track)?;
        self.current_cd_track = Some(cd_track);
        Ok(())
    }

    /// Selected original disc identity, including while Ambient has paused it.
    pub fn current_cd_track(&self) -> Option<u8> {
        self.current_cd_track
    }

    /// Resume the selected track from its preserved buffered position.
    pub fn resume(&self) {
        self.shared.lock().resume();
        self.shared.decoder_wake.notify_one();
    }

    /// Pause the selected track without rewinding it.
    pub fn pause(&self) {
        self.shared.lock().pause();
    }

    /// Stop playback, rewind, and release the selected track.
    pub fn stop(&mut self) {
        self.shared.lock().stop();
        self.shared.decoder_wake.notify_one();
        self.current_cd_track = None;
    }
}

impl Drop for MusicPlayer {
    fn drop(&mut self) {
        self._device.pause();
        {
            let mut state = self.shared.lock();
            state.shutdown = true;
            state.stop();
        }
        self.shared.decoder_wake.notify_all();
        if let Some(worker) = self.decoder_worker.take() {
            let _ = worker.join();
        }
    }
}

/// Both supported formats feed the same bounded PCM queue. WAV packets contain
/// at most 4096 stereo frames; Vorbis packets retain the decoder's packet bound.
enum StreamDecoder {
    Vorbis(Box<VorbisReader>),
    PcmWav(wav::WavReader<BufReader<File>>),
}

impl StreamDecoder {
    fn read_packet(&mut self) -> Result<Option<Vec<i16>>, String> {
        match self {
            Self::Vorbis(reader) => reader
                .read_dec_packet_itl()
                .map_err(|error| error.to_string()),
            Self::PcmWav(reader) => reader.read_packet(),
        }
    }
}

/// Parse stream headers without materializing the audio body. WAV validation
/// checks every chunk boundary through seeking, and both sources must match the
/// dedicated SDL device's original 44.1-kHz stereo i16 format exactly.
fn open_decoder(path: &Path) -> Result<StreamDecoder, String> {
    let mut file =
        File::open(path).map_err(|error| format!("Failed to open {}: {error}", path.display()))?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)
        .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("Failed to seek {}: {error}", path.display()))?;
    if magic == *b"RIFF" {
        return wav::WavReader::new(BufReader::new(file))
            .map(StreamDecoder::PcmWav)
            .map_err(|error| format!("Failed to parse {}: {error}", path.display()));
    }
    if magic != *b"OggS" {
        return Err(format!(
            "Unrecognized CD-audio header in {}",
            path.display()
        ));
    }
    let reader = OggStreamReader::new(BufReader::new(file))
        .map_err(|error| format!("Failed to parse {}: {error}", path.display()))?;
    let channels = reader.ident_hdr.audio_channels as u16;
    let sample_rate = reader.ident_hdr.audio_sample_rate;
    if channels != OUTPUT_CHANNELS || sample_rate != OUTPUT_SAMPLE_RATE {
        return Err(format!(
            "Unsupported CD-audio format in {}: {} Hz, {} channels (expected {} Hz stereo)",
            path.display(),
            sample_rate,
            channels,
            OUTPUT_SAMPLE_RATE
        ));
    }
    Ok(StreamDecoder::Vorbis(Box::new(reader)))
}

/// Decode packets into a fixed-size queue. Generation checks discard stale
/// packets after a rapid track switch or stop, and EOF reopens the source to
/// preserve the retail looping behavior.
fn decoder_worker(shared: Arc<SharedMusic>) {
    let mut active_generation = u64::MAX;
    let mut decoder: Option<StreamDecoder> = None;
    let mut decoded_pcm_this_pass = false;
    let mut pending = Vec::<i16>::new();
    let mut pending_cursor = 0usize;

    loop {
        let (generation, path) = {
            let mut state = shared.lock();
            loop {
                if state.shutdown {
                    return;
                }

                if state.generation != active_generation {
                    active_generation = state.generation;
                    decoder = None;
                    decoded_pcm_this_pass = false;
                    pending.clear();
                    pending_cursor = 0;
                }

                let Some(path) = state.selected_path.clone() else {
                    state = shared.wait(state);
                    continue;
                };

                if state.failed_generation == Some(active_generation) {
                    state = shared.wait(state);
                    continue;
                }

                if pending_cursor < pending.len() {
                    let added = state.enqueue(active_generation, &pending[pending_cursor..]);
                    pending_cursor += added;
                    if pending_cursor == pending.len() {
                        pending.clear();
                        pending_cursor = 0;
                    }
                    if added != 0 {
                        continue;
                    }
                }

                if state.samples.len() >= STREAM_BUFFER_SAMPLES {
                    state = shared.wait(state);
                    continue;
                }

                break (active_generation, path);
            }
        };

        if decoder.is_none() {
            match open_decoder(&path) {
                Ok(opened) => {
                    decoder = Some(opened);
                    decoded_pcm_this_pass = false;
                }
                Err(error) => {
                    fail_generation(&shared, generation, error);
                    continue;
                }
            }
        }

        let packet = decoder
            .as_mut()
            .expect("decoder was opened above")
            .read_packet();
        match packet {
            Ok(Some(samples)) => {
                decoded_pcm_this_pass |= !samples.is_empty();
                pending = samples;
                pending_cursor = 0;
            }
            Ok(None) => {
                if !decoded_pcm_this_pass {
                    fail_generation(
                        &shared,
                        generation,
                        format!("Empty CD-audio stream in {}", path.display()),
                    );
                    continue;
                }
                // Loop by reopening from the beginning on the next iteration.
                decoder = None;
            }
            Err(error) => {
                fail_generation(
                    &shared,
                    generation,
                    format!("CD-audio decode error in {}: {error}", path.display()),
                );
            }
        }
    }
}

fn fail_generation(shared: &SharedMusic, generation: u64, error: String) {
    let mut state = shared.lock();
    if state.generation == generation {
        state.failed_generation = Some(generation);
        state.playing = false;
        state.samples.clear();
        eprintln!("Music stream error: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct WavWorkerFixture {
        path: PathBuf,
        shared: Arc<SharedMusic>,
        worker: Option<JoinHandle<()>>,
    }

    impl WavWorkerFixture {
        fn start(samples: &[i16]) -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "v2k-music-worker-{}-{nonce}.wav",
                std::process::id()
            ));
            std::fs::write(&path, wav::tests::pcm_wav(samples)).unwrap();
            let shared = Arc::new(SharedMusic::default());
            shared.lock().select(path.clone());
            let worker_shared = Arc::clone(&shared);
            let worker = thread::spawn(move || decoder_worker(worker_shared));
            Self {
                path,
                shared,
                worker: Some(worker),
            }
        }

        fn wait_for_queue(&self, expected: usize) {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            while self.shared.lock().samples.len() < expected {
                assert!(
                    std::time::Instant::now() < deadline,
                    "PCM worker did not buffer in time"
                );
                thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    }

    impl Drop for WavWorkerFixture {
        fn drop(&mut self) {
            {
                let mut state = self.shared.lock();
                state.shutdown = true;
                state.stop();
            }
            self.shared.decoder_wake.notify_all();
            if let Some(worker) = self.worker.take() {
                worker.join().unwrap();
            }
            let _ = std::fs::remove_file(&self.path);
        }
    }

    #[test]
    fn wav_worker_loops_exact_pcm_through_bounded_callback_queue() {
        let fixture = WavWorkerFixture::start(&[101, -202, 303, -404]);
        fixture.wait_for_queue(12);
        fixture.shared.lock().resume();
        let mut callback = callback_for(&fixture.shared);
        let mut out = [0i16; 12];
        callback.callback(&mut out);
        assert_eq!(
            out,
            [101, -202, 303, -404, 101, -202, 303, -404, 101, -202, 303, -404]
        );
        assert!(fixture.shared.lock().samples.len() <= STREAM_BUFFER_SAMPLES);
    }

    #[test]
    fn wav_worker_rejects_packets_after_stop() {
        let fixture = WavWorkerFixture::start(&[101, -202, 303, -404]);
        fixture.wait_for_queue(8);
        fixture.shared.lock().stop();
        fixture.shared.decoder_wake.notify_all();
        fixture.shared.lock().resume();
        let mut callback = callback_for(&fixture.shared);
        let mut out = [7i16; 8];
        callback.callback(&mut out);
        assert_eq!(out, [0; 8]);
        assert!(fixture.shared.lock().samples.is_empty());
    }

    #[test]
    fn unavailable_physical_track_request_clears_previous_selection_and_pcm() {
        let inventory = SoundtrackInventory {
            directory: PathBuf::from("cdaudio"),
            directory_missing: false,
            tracks: std::array::from_fn(|index| SoundtrackTrack {
                cd_track: FIRST_CD_TRACK + index as u8,
                path: None,
                errors: Vec::new(),
            }),
            scan_errors: Vec::new(),
        };
        for cd_track in [3, 0, 12] {
            let shared = SharedMusic::default();
            {
                let mut state = shared.lock();
                let generation = state.select(PathBuf::from("track02.ogg"));
                state.enqueue(generation, &[123, -456]);
                state.resume();
            }
            assert!(shared.select_cd_track(&inventory, cd_track).is_err());
            let state = shared.lock();
            assert!(state.selected_path.is_none());
            assert!(state.samples.is_empty());
            assert!(!state.playing);
        }
    }

    fn callback_for(shared: &Arc<SharedMusic>) -> MusicCallback {
        MusicCallback {
            shared: Arc::clone(shared),
        }
    }

    #[test]
    fn selected_music_starts_paused_and_plays_pcm_at_unity() {
        let shared = Arc::new(SharedMusic::default());
        let generation = {
            let mut state = shared.lock();
            let generation = state.select(PathBuf::from("track02.ogg"));
            assert_eq!(state.enqueue(generation, &[12_345, -23_456]), 2);
            generation
        };
        let mut callback = callback_for(&shared);
        let mut out = [1i16; 2];

        callback.callback(&mut out);
        assert_eq!(out, [0, 0]);
        assert_eq!(shared.lock().samples.len(), 2);

        let mut state = shared.lock();
        assert_eq!(state.generation, generation);
        state.resume();
        drop(state);
        callback.callback(&mut out);
        assert_eq!(out, [12_345, -23_456]);
    }

    #[test]
    fn pause_and_resume_preserve_buffered_playback_position() {
        let shared = Arc::new(SharedMusic::default());
        {
            let mut state = shared.lock();
            let generation = state.select(PathBuf::from("track02.ogg"));
            state.enqueue(generation, &[10, 20, 30, 40]);
            state.resume();
        }
        let mut callback = callback_for(&shared);
        let mut first = [0i16; 2];
        callback.callback(&mut first);
        assert_eq!(first, [10, 20]);

        shared.lock().pause();
        let mut paused = [1i16; 2];
        callback.callback(&mut paused);
        assert_eq!(paused, [0, 0]);
        assert_eq!(
            shared.lock().samples.iter().copied().collect::<Vec<_>>(),
            [30, 40]
        );

        shared.lock().resume();
        let mut resumed = [0i16; 2];
        callback.callback(&mut resumed);
        assert_eq!(resumed, [30, 40]);
    }

    #[test]
    fn resume_before_the_first_packet_preserves_play_intent() {
        let shared = Arc::new(SharedMusic::default());
        let generation = shared.lock().select(PathBuf::from("track02.ogg"));
        shared.lock().resume();
        let mut callback = callback_for(&shared);

        let mut buffering = [1i16; 2];
        callback.callback(&mut buffering);
        assert_eq!(buffering, [0, 0]);
        assert!(shared.lock().playing);

        shared.lock().enqueue(generation, &[101, -202]);
        let mut ready = [0i16; 2];
        callback.callback(&mut ready);
        assert_eq!(ready, [101, -202]);
    }

    #[test]
    fn callback_underflow_zero_fills_then_continues_without_skipping() {
        let shared = Arc::new(SharedMusic::default());
        let generation = {
            let mut state = shared.lock();
            let generation = state.select(PathBuf::from("track02.ogg"));
            state.enqueue(generation, &[11, 22]);
            state.resume();
            generation
        };
        let mut callback = callback_for(&shared);

        let mut underflow = [9i16; 4];
        callback.callback(&mut underflow);
        assert_eq!(underflow, [11, 22, 0, 0]);
        assert!(shared.lock().playing);

        shared.lock().enqueue(generation, &[33, 44]);
        let mut continued = [0i16; 2];
        callback.callback(&mut continued);
        assert_eq!(continued, [33, 44]);
    }

    #[test]
    fn callback_emits_silence_instead_of_waiting_for_the_decoder_lock() {
        let shared = Arc::new(SharedMusic::default());
        let mut callback = callback_for(&shared);
        let _decoder_guard = shared.lock();
        let mut out = [7i16; 4];

        callback.callback(&mut out);
        assert_eq!(out, [0, 0, 0, 0]);
    }

    #[test]
    fn switching_tracks_rejects_stale_decoder_packets() {
        let mut state = MusicState::default();
        let old_generation = state.select(PathBuf::from("track02.ogg"));
        state.enqueue(old_generation, &[1, 2, 3]);

        let new_generation = state.select(PathBuf::from("track03.ogg"));
        assert_eq!(state.enqueue(old_generation, &[4, 5]), 0);
        assert_eq!(state.enqueue(new_generation, &[6, 7]), 2);
        assert_eq!(state.samples.iter().copied().collect::<Vec<_>>(), [6, 7]);
        assert!(!state.playing);
    }

    #[test]
    fn reselecting_the_same_track_still_invalidates_old_packets() {
        let mut state = MusicState::default();
        let old_generation = state.select(PathBuf::from("track02.ogg"));
        let new_generation = state.select(PathBuf::from("track02.ogg"));

        assert_ne!(old_generation, new_generation);
        assert_eq!(state.enqueue(old_generation, &[1, 2]), 0);
        assert_eq!(state.enqueue(new_generation, &[3, 4]), 2);
        assert_eq!(state.samples.iter().copied().collect::<Vec<_>>(), [3, 4]);
    }

    #[test]
    fn decoder_queue_is_strictly_bounded() {
        let mut state = MusicState::default();
        let generation = state.select(PathBuf::from("track02.ogg"));
        let oversized = vec![9; STREAM_BUFFER_SAMPLES + 32];

        assert_eq!(state.enqueue(generation, &oversized), STREAM_BUFFER_SAMPLES);
        assert_eq!(state.samples.len(), STREAM_BUFFER_SAMPLES);
        assert_eq!(state.enqueue(generation, &[1, 2]), 0);
    }

    #[test]
    fn stop_releases_selection_and_invalidates_in_flight_decode() {
        let mut state = MusicState::default();
        let generation = state.select(PathBuf::from("track02.ogg"));
        state.enqueue(generation, &[1, 2, 3]);
        state.resume();

        state.stop();
        assert!(state.selected_path.is_none());
        assert!(state.samples.is_empty());
        assert!(!state.playing);
        assert_eq!(state.enqueue(generation, &[4, 5]), 0);
    }
}
