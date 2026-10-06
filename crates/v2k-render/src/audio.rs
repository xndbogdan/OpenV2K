//! Minimal SDL2 audio queue wrapper for PCM playback.

use sdl2::audio::{AudioQueue, AudioSpecDesired};
use sdl2::Sdl;

/// Simple audio player using SDL2's audio queue.
/// Supports both 8-bit unsigned and 16-bit signed PCM.
pub struct AudioPlayer {
    inner: AudioInner,
}

enum AudioInner {
    U8(AudioQueue<u8>),
    I16(AudioQueue<i16>),
}

impl AudioPlayer {
    /// Create a new audio player for the given PCM format.
    pub fn new(sdl: &Sdl, sample_rate: u32, channels: u8, bits: u16) -> Result<Self, String> {
        let audio = sdl.audio()?;

        let desired = AudioSpecDesired {
            freq: Some(sample_rate as i32),
            channels: Some(channels),
            samples: Some(4096),
        };

        let inner = if bits == 16 {
            let queue = audio.open_queue::<i16, _>(None, &desired)?;
            AudioInner::I16(queue)
        } else {
            let queue = audio.open_queue::<u8, _>(None, &desired)?;
            AudioInner::U8(queue)
        };

        Ok(Self { inner })
    }

    /// Queue raw PCM data for playback and start playing.
    pub fn play(&self, pcm_data: &[u8]) {
        let chunk_size = 65536;
        match &self.inner {
            AudioInner::U8(queue) => {
                queue.clear();
                for chunk in pcm_data.chunks(chunk_size) {
                    queue.queue_audio(chunk).ok();
                }
                queue.resume();
            }
            AudioInner::I16(queue) => {
                queue.clear();
                // Convert raw bytes to i16 samples (little-endian)
                let samples: Vec<i16> = pcm_data
                    .chunks_exact(2)
                    .map(|b| i16::from_le_bytes([b[0], b[1]]))
                    .collect();
                for chunk in samples.chunks(chunk_size / 2) {
                    queue.queue_audio(chunk).ok();
                }
                queue.resume();
            }
        }
    }

    /// Stop playback.
    pub fn stop(&self) {
        match &self.inner {
            AudioInner::U8(queue) => {
                queue.pause();
                queue.clear();
            }
            AudioInner::I16(queue) => {
                queue.pause();
                queue.clear();
            }
        }
    }

    /// Check if audio is still playing.
    pub fn is_playing(&self) -> bool {
        match &self.inner {
            AudioInner::U8(queue) => queue.size() > 0,
            AudioInner::I16(queue) => queue.size() > 0,
        }
    }
}
