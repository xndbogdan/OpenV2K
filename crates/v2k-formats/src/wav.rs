//! Minimal WAV encoder for raw PCM audio export.
//!
//! Writes a 44-byte RIFF/WAVE header followed by raw PCM payload.
//! No external dependencies — the header is trivial fixed-layout.

/// Audio format parameters for WAV encoding.
#[derive(Debug, Clone, Copy)]
pub struct WavParams {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
}

/// V2000 audio format: 22050 Hz mono 16-bit signed PCM.
/// (From WAVEFORMATEX at V2000.EXE:0x004D6740)
pub const V2K_AUDIO: WavParams = WavParams {
    sample_rate: 22050,
    channels: 1,
    bits_per_sample: 16,
};

/// Encode raw PCM data as a complete WAV file (44-byte header + data).
pub fn encode_wav(pcm_data: &[u8], params: &WavParams) -> Vec<u8> {
    let data_size = pcm_data.len() as u32;
    let block_align = params.channels * (params.bits_per_sample / 8);
    let byte_rate = params.sample_rate * block_align as u32;
    let file_size = 36 + data_size; // RIFF chunk size = total - 8

    let mut buf = Vec::with_capacity(44 + pcm_data.len());

    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // fmt sub-chunk (16 bytes)
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // sub-chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    buf.extend_from_slice(&params.channels.to_le_bytes());
    buf.extend_from_slice(&params.sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&params.bits_per_sample.to_le_bytes());

    // data sub-chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    buf.extend_from_slice(pcm_data);

    buf
}

/// Calculate duration in seconds for a PCM buffer.
pub fn duration_secs(pcm_len: usize, params: &WavParams) -> f64 {
    let bytes_per_sample = params.channels as usize * (params.bits_per_sample as usize / 8);
    if bytes_per_sample == 0 || params.sample_rate == 0 {
        return 0.0;
    }
    let samples = pcm_len / bytes_per_sample;
    samples as f64 / params.sample_rate as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_structure() {
        let pcm = vec![0u8; 100];
        let wav = encode_wav(&pcm, &V2K_AUDIO);

        assert_eq!(wav.len(), 144); // 44 header + 100 data
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");

        // RIFF chunk size = file_size - 8
        let riff_size = u32::from_le_bytes(wav[4..8].try_into().unwrap());
        assert_eq!(riff_size, 136); // 44 - 8 + 100

        // fmt sub-chunk size = 16
        let fmt_size = u32::from_le_bytes(wav[16..20].try_into().unwrap());
        assert_eq!(fmt_size, 16);

        // PCM format = 1
        let audio_fmt = u16::from_le_bytes(wav[20..22].try_into().unwrap());
        assert_eq!(audio_fmt, 1);

        // channels = 1
        let channels = u16::from_le_bytes(wav[22..24].try_into().unwrap());
        assert_eq!(channels, 1);

        // sample rate = 22050
        let sample_rate = u32::from_le_bytes(wav[24..28].try_into().unwrap());
        assert_eq!(sample_rate, 22050);

        // bits per sample = 16
        let bps = u16::from_le_bytes(wav[34..36].try_into().unwrap());
        assert_eq!(bps, 16);

        // data size
        let data_size = u32::from_le_bytes(wav[40..44].try_into().unwrap());
        assert_eq!(data_size, 100);
    }

    #[test]
    fn wav_empty_pcm() {
        let wav = encode_wav(&[], &V2K_AUDIO);
        assert_eq!(wav.len(), 44);
        let data_size = u32::from_le_bytes(wav[40..44].try_into().unwrap());
        assert_eq!(data_size, 0);
    }

    #[test]
    fn duration_calculation() {
        // 22050 samples × 2 bytes/sample = 44100 bytes = 1.0 second
        let dur = duration_secs(44100, &V2K_AUDIO);
        assert!((dur - 1.0).abs() < 0.001);

        // empty
        assert_eq!(duration_secs(0, &V2K_AUDIO), 0.0);

        // half second
        let dur_half = duration_secs(22050, &V2K_AUDIO);
        assert!((dur_half - 0.5).abs() < 0.001);
    }
}
