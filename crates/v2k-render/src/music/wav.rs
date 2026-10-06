//! Bounded streaming of lossless CD-DA PCM extracted from a mixed-mode image.
//!
//! RIFF lengths are validated before PCM reaches the worker. Unknown chunks
//! are skipped by seeking; neither metadata nor the audio body is loaded whole.

use std::io::{Read, Seek, SeekFrom};

const PACKET_FRAMES: usize = 4096;
const CD_FRAME_BYTES: usize = 4;

pub(super) struct WavReader<R> {
    source: R,
    remaining_bytes: u64,
}

impl<R: Read + Seek> WavReader<R> {
    pub(super) fn new(mut source: R) -> Result<Self, String> {
        let file_len = source.seek(SeekFrom::End(0)).map_err(io_error)?;
        source.seek(SeekFrom::Start(0)).map_err(io_error)?;
        let mut header = [0u8; 12];
        source.read_exact(&mut header).map_err(io_error)?;
        if &header[..4] != b"RIFF" || &header[8..] != b"WAVE" {
            return Err("Expected a RIFF WAVE CD-audio file".to_owned());
        }
        let riff_len = u32::from_le_bytes(header[4..8].try_into().unwrap()) as u64;
        let riff_end = 8 + riff_len;
        if riff_len < 4 || riff_end > file_len {
            return Err("Truncated RIFF WAVE container".to_owned());
        }

        let mut position = 12u64;
        let mut found_format = false;
        let mut audio = None;
        while position < riff_end {
            if riff_end - position < 8 {
                return Err("Truncated WAVE chunk header".to_owned());
            }
            let mut chunk = [0u8; 8];
            source.read_exact(&mut chunk).map_err(io_error)?;
            let chunk_len = u32::from_le_bytes(chunk[4..].try_into().unwrap()) as u64;
            let payload_start = position + 8;
            let payload_end = payload_start + chunk_len;
            let next_chunk = payload_end + (chunk_len & 1);
            if next_chunk > riff_end {
                return Err("Truncated WAVE chunk payload or padding".to_owned());
            }
            match &chunk[..4] {
                b"fmt " => {
                    if found_format || chunk_len < 16 {
                        return Err("Short or duplicate PCM WAVE format fields".to_owned());
                    }
                    let mut format = [0u8; 16];
                    source.read_exact(&mut format).map_err(io_error)?;
                    let encoding = u16::from_le_bytes(format[0..2].try_into().unwrap());
                    let channels = u16::from_le_bytes(format[2..4].try_into().unwrap());
                    let sample_rate = u32::from_le_bytes(format[4..8].try_into().unwrap());
                    let byte_rate = u32::from_le_bytes(format[8..12].try_into().unwrap());
                    let block_align = u16::from_le_bytes(format[12..14].try_into().unwrap());
                    let bits = u16::from_le_bytes(format[14..16].try_into().unwrap());
                    if encoding != 1
                        || channels != 2
                        || sample_rate != 44_100
                        || bits != 16
                        || byte_rate != 176_400
                        || block_align != CD_FRAME_BYTES as u16
                    {
                        return Err(format!(
                            "Unsupported WAVE format: encoding {encoding}, {sample_rate} Hz,                              {channels} channels, {bits} bits (expected PCM 44100 Hz stereo 16-bit)"
                        ));
                    }
                    found_format = true;
                }
                b"data" => {
                    if audio.is_some() || chunk_len == 0 || chunk_len % CD_FRAME_BYTES as u64 != 0 {
                        return Err("Empty, duplicate, or incomplete stereo WAVE data".to_owned());
                    }
                    audio = Some((payload_start, chunk_len));
                }
                _ => {}
            }
            source.seek(SeekFrom::Start(next_chunk)).map_err(io_error)?;
            position = next_chunk;
        }
        if !found_format {
            return Err("WAVE file has no PCM format chunk".to_owned());
        }
        let (audio_start, remaining_bytes) =
            audio.ok_or_else(|| "WAVE file has no audio data chunk".to_owned())?;
        source
            .seek(SeekFrom::Start(audio_start))
            .map_err(io_error)?;
        Ok(Self {
            source,
            remaining_bytes,
        })
    }

    pub(super) fn read_packet(&mut self) -> Result<Option<Vec<i16>>, String> {
        if self.remaining_bytes == 0 {
            return Ok(None);
        }
        let byte_count = self
            .remaining_bytes
            .min((PACKET_FRAMES * CD_FRAME_BYTES) as u64) as usize;
        let mut bytes = vec![0u8; byte_count];
        self.source.read_exact(&mut bytes).map_err(io_error)?;
        self.remaining_bytes -= byte_count as u64;
        Ok(Some(
            bytes
                .chunks_exact(2)
                .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
                .collect(),
        ))
    }
}

fn io_error(error: std::io::Error) -> String {
    format!("Cannot read PCM WAVE stream: {error}")
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use std::io::Cursor;

    pub(crate) fn pcm_wav(samples: &[i16]) -> Vec<u8> {
        let mut bytes = b"RIFF".to_vec();
        bytes.extend_from_slice(&(36 + samples.len() as u32 * 2).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&44_100u32.to_le_bytes());
        bytes.extend_from_slice(&176_400u32.to_le_bytes());
        bytes.extend_from_slice(&4u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(samples.len() as u32 * 2).to_le_bytes());
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn pcm_stream_is_little_endian_packet_bounded_and_reaches_exact_eof() {
        let samples: Vec<i16> = (0..PACKET_FRAMES * 2 + 6)
            .map(|i| (i as i16).wrapping_mul(-13))
            .collect();
        let mut reader = WavReader::new(Cursor::new(pcm_wav(&samples))).unwrap();
        assert_eq!(
            reader.read_packet().unwrap().unwrap(),
            samples[..PACKET_FRAMES * 2]
        );
        assert_eq!(
            reader.read_packet().unwrap().unwrap(),
            samples[PACKET_FRAMES * 2..]
        );
        assert!(reader.read_packet().unwrap().is_none());
    }

    #[test]
    fn wav_rejects_short_containers_chunk_payloads_and_partial_stereo_frames() {
        let valid = pcm_wav(&[0x1234, -0x2345]);
        for length in 0..valid.len() {
            assert!(WavReader::new(Cursor::new(&valid[..length])).is_err());
        }
        let mut broken = valid.clone();
        broken[40..44].copy_from_slice(&8u32.to_le_bytes());
        assert!(WavReader::new(Cursor::new(broken)).is_err());
        assert!(WavReader::new(Cursor::new(pcm_wav(&[1]))).is_err());
        assert!(WavReader::new(Cursor::new(pcm_wav(&[]))).is_err());
    }

    #[test]
    fn wav_skips_padded_metadata_and_accepts_data_before_format() {
        let valid = pcm_wav(&[123, -456]);
        let mut bytes = valid[..12].to_vec();
        bytes.extend_from_slice(b"JUNK");
        bytes.extend_from_slice(&3u32.to_le_bytes());
        bytes.extend_from_slice(b"abc\0");
        bytes.extend_from_slice(&valid[36..]);
        bytes.extend_from_slice(&valid[12..36]);
        let len = bytes.len() as u32 - 8;
        bytes[4..8].copy_from_slice(&len.to_le_bytes());
        let mut reader = WavReader::new(Cursor::new(bytes)).unwrap();
        assert_eq!(reader.read_packet().unwrap().unwrap(), [123, -456]);
    }

    #[test]
    fn wav_rejects_wrong_pcm_rate_channels_bits_and_alignment() {
        for (offset, replacement) in [
            (20, vec![3, 0]),
            (22, vec![1, 0]),
            (24, 48_000u32.to_le_bytes().to_vec()),
            (28, 1u32.to_le_bytes().to_vec()),
            (32, vec![2, 0]),
            (34, vec![8, 0]),
        ] {
            let mut broken = pcm_wav(&[1, 2]);
            broken[offset..offset + replacement.len()].copy_from_slice(&replacement);
            assert!(WavReader::new(Cursor::new(broken)).is_err());
        }
    }

    #[test]
    fn wav_rejects_missing_format_data_and_truncated_unknown_chunks() {
        for chunk_range in [12..36, 36..48] {
            let mut bytes = pcm_wav(&[1, 2]);
            bytes.drain(chunk_range);
            let len = bytes.len() as u32 - 8;
            bytes[4..8].copy_from_slice(&len.to_le_bytes());
            assert!(WavReader::new(Cursor::new(bytes)).is_err());
        }
        let mut bytes = pcm_wav(&[1, 2]);
        bytes.extend_from_slice(b"JUNK");
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        let len = bytes.len() as u32 - 8;
        bytes[4..8].copy_from_slice(&len.to_le_bytes());
        assert!(WavReader::new(Cursor::new(bytes)).is_err());
    }
}
