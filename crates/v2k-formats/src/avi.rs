//! RIFF/AVI container parser and Microsoft Video 1 (CRAM/MSVC) decoder.
//!
//! Decodes AVI files using the Microsoft Video 1 codec (FOURCC: CRAM/MSVC/WHAM).
//! This codec uses 4x4 block-based compression with 16-bit BGR555 color.
//! Audio is expected to be uncompressed PCM.
//!
//! Decoder closely follows ffmpeg's `msvideo1_decode_16bit()`.

use std::fmt;

/// Parsed AVI file metadata.
#[derive(Debug, Clone)]
pub struct AviFile {
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub total_frames: u32,
    pub audio_rate: u32,
    pub audio_channels: u16,
    pub audio_bits: u16,
}

/// AVI player that decodes video frames and provides audio data.
pub struct AviPlayer {
    pub info: AviFile,
    frame_chunks: Vec<Vec<u8>>,
    audio_data: Vec<u8>,
    framebuffer: Vec<u8>, // RGBA, top-down
    decoded_frame: Option<usize>,
}

/// Errors during AVI parsing/decoding.
#[derive(Debug)]
pub enum AviError {
    NotRiff,
    NotAvi,
    NoVideoStream,
    UnsupportedCodec(String),
}

impl fmt::Display for AviError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AviError::NotRiff => write!(f, "Not a RIFF file"),
            AviError::NotAvi => write!(f, "Not an AVI file"),
            AviError::NoVideoStream => write!(f, "No video stream found"),
            AviError::UnsupportedCodec(c) => write!(f, "Unsupported codec: {}", c),
        }
    }
}

impl std::error::Error for AviError {}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_tag(data: &[u8], offset: usize) -> [u8; 4] {
    [
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]
}

fn tag_str(tag: &[u8; 4]) -> &str {
    std::str::from_utf8(tag).unwrap_or("????")
}

impl AviPlayer {
    /// Parse an AVI file from raw bytes.
    pub fn open(data: &[u8]) -> Result<Self, AviError> {
        if data.len() < 12 {
            return Err(AviError::NotRiff);
        }
        if &data[0..4] != b"RIFF" {
            return Err(AviError::NotRiff);
        }
        if &data[8..12] != b"AVI " {
            return Err(AviError::NotAvi);
        }

        let mut width = 0u32;
        let mut height = 0u32;
        let mut fps = 15.0f32;
        let mut total_frames = 0u32;
        let mut audio_rate = 22050u32;
        let mut audio_channels = 1u16;
        let mut audio_bits = 8u16;
        let mut codec_fourcc = [0u8; 4];
        let mut found_video = false;
        let mut frame_chunks: Vec<Vec<u8>> = Vec::new();
        let mut audio_data: Vec<u8> = Vec::new();

        let mut current_stream_type: Option<[u8; 4]> = None;
        parse_chunks(
            data,
            12,
            data.len(),
            &mut width,
            &mut height,
            &mut fps,
            &mut total_frames,
            &mut audio_rate,
            &mut audio_channels,
            &mut audio_bits,
            &mut codec_fourcc,
            &mut found_video,
            &mut frame_chunks,
            &mut audio_data,
            &mut current_stream_type,
        );

        if !found_video {
            return Err(AviError::NoVideoStream);
        }

        // Validate codec
        let cc_upper = [
            codec_fourcc[0].to_ascii_uppercase(),
            codec_fourcc[1].to_ascii_uppercase(),
            codec_fourcc[2].to_ascii_uppercase(),
            codec_fourcc[3].to_ascii_uppercase(),
        ];
        if &cc_upper != b"CRAM"
            && &cc_upper != b"MSVC"
            && &cc_upper != b"WHAM"
            && codec_fourcc != [0, 0, 0, 0]
        {
            return Err(AviError::UnsupportedCodec(
                tag_str(&codec_fourcc).to_string(),
            ));
        }

        if total_frames == 0 {
            total_frames = frame_chunks.len() as u32;
        }

        let framebuffer = vec![0u8; (width * height * 4) as usize];

        Ok(Self {
            info: AviFile {
                width,
                height,
                fps,
                total_frames,
                audio_rate,
                audio_channels,
                audio_bits,
            },
            frame_chunks,
            audio_data,
            framebuffer,
            decoded_frame: None,
        })
    }

    /// Total number of video frames available.
    pub fn frame_count(&self) -> usize {
        self.frame_chunks.len()
    }

    /// Decode a specific frame into the internal framebuffer.
    /// Frames are delta-coded: decode any missing predecessors before presenting
    /// the requested frame. Repeated requests reuse the framebuffer, and seeking
    /// backward resets and replays from the beginning. An out-of-range request
    /// leaves the current framebuffer and decoder position unchanged.
    ///
    /// Returns the RGBA framebuffer (top-down row order).
    pub fn decode_frame(&mut self, index: usize) -> &[u8] {
        if index >= self.frame_chunks.len() || self.decoded_frame == Some(index) {
            return &self.framebuffer;
        }
        if self.decoded_frame.is_some_and(|decoded| index < decoded) {
            self.framebuffer.fill(0);
            self.decoded_frame = None;
        }
        let first = self.decoded_frame.map_or(0, |decoded| decoded + 1);
        for chunk in &self.frame_chunks[first..=index] {
            if !chunk.is_empty() {
                decode_msvc1_16bit(
                    chunk,
                    &mut self.framebuffer,
                    self.info.width as usize,
                    self.info.height as usize,
                );
            }
        }
        self.decoded_frame = Some(index);
        &self.framebuffer
    }

    /// Get the concatenated PCM audio data.
    pub fn audio_pcm(&self) -> &[u8] {
        &self.audio_data
    }
}

#[allow(clippy::too_many_arguments)]
fn parse_chunks(
    data: &[u8],
    start: usize,
    end: usize,
    width: &mut u32,
    height: &mut u32,
    fps: &mut f32,
    total_frames: &mut u32,
    audio_rate: &mut u32,
    audio_channels: &mut u16,
    audio_bits: &mut u16,
    codec_fourcc: &mut [u8; 4],
    found_video: &mut bool,
    frame_chunks: &mut Vec<Vec<u8>>,
    audio_data: &mut Vec<u8>,
    current_stream_type: &mut Option<[u8; 4]>,
) {
    let mut pos = start;

    while pos + 8 <= end {
        let tag = read_tag(data, pos);
        let size = read_u32(data, pos + 4) as usize;
        let chunk_end = pos + 8 + size;

        match &tag {
            b"LIST" => {
                if pos + 12 > end {
                    break;
                }
                let list_type = read_tag(data, pos + 8);
                let list_end = chunk_end.min(end);
                match &list_type {
                    b"hdrl" | b"strl" | b"movi" => {
                        parse_chunks(
                            data,
                            pos + 12,
                            list_end,
                            width,
                            height,
                            fps,
                            total_frames,
                            audio_rate,
                            audio_channels,
                            audio_bits,
                            codec_fourcc,
                            found_video,
                            frame_chunks,
                            audio_data,
                            current_stream_type,
                        );
                    }
                    _ => {}
                }
            }
            b"avih" => {
                if chunk_end <= end && size >= 40 {
                    let chunk = &data[pos + 8..chunk_end];
                    let usec = read_u32(chunk, 0);
                    if usec > 0 {
                        *fps = 1_000_000.0 / usec as f32;
                    }
                    *total_frames = read_u32(chunk, 16);
                    *width = read_u32(chunk, 32);
                    *height = read_u32(chunk, 36);
                }
            }
            b"strh" => {
                if chunk_end <= end && size >= 28 {
                    let chunk = &data[pos + 8..chunk_end];
                    let stream_type = read_tag(chunk, 0);
                    let handler = read_tag(chunk, 4);
                    *current_stream_type = Some(stream_type);
                    if &stream_type == b"vids" {
                        *found_video = true;
                        *codec_fourcc = handler;
                        let scale = read_u32(chunk, 20);
                        let rate = read_u32(chunk, 24);
                        if scale > 0 {
                            *fps = rate as f32 / scale as f32;
                        }
                    }
                }
            }
            b"strf" => {
                if chunk_end <= end && size >= 16 {
                    let chunk = &data[pos + 8..chunk_end];
                    match current_stream_type.as_ref() {
                        Some(b"vids") => {
                            if size >= 40 {
                                // BITMAPINFOHEADER
                                *width = read_u32(chunk, 4);
                                let h = read_u32(chunk, 8) as i32;
                                *height = h.unsigned_abs();
                                let compression = read_tag(chunk, 16);
                                if compression != [0, 0, 0, 0] {
                                    *codec_fourcc = compression;
                                }
                            }
                        }
                        Some(b"auds") => {
                            // WAVEFORMATEX
                            *audio_channels = read_u16(chunk, 2);
                            *audio_rate = read_u32(chunk, 4);
                            if size >= 16 {
                                *audio_bits = read_u16(chunk, 14);
                            }
                            eprintln!(
                                "AVI audio strf: {}Hz {}ch {}bit",
                                *audio_rate, *audio_channels, *audio_bits
                            );
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                if chunk_end <= end {
                    if tag[2] == b'd' && tag[3] == b'c' {
                        frame_chunks.push(data[pos + 8..chunk_end].to_vec());
                    } else if tag[2] == b'w' && tag[3] == b'b' {
                        audio_data.extend_from_slice(&data[pos + 8..chunk_end]);
                    }
                }
            }
        }

        // Advance past chunk (word-aligned)
        pos = chunk_end;
        if pos % 2 != 0 {
            pos += 1;
        }
    }
}

/// Decode a Microsoft Video 1 (MSVC/CRAM) 16-bit frame.
///
/// Closely follows ffmpeg's `msvideo1_decode_16bit()` algorithm.
/// Block traversal is left-to-right, bottom-to-top (BMP convention).
/// Output framebuffer is top-down RGBA.
fn decode_msvc1_16bit(chunk: &[u8], fb: &mut [u8], width: usize, height: usize) {
    let blocks_w = width / 4;
    let blocks_h = height / 4;
    if blocks_w == 0 || blocks_h == 0 {
        return;
    }

    let mut total_blocks = blocks_w * blocks_h;
    let mut pos = 0;
    let mut block_x = 0usize;
    let mut block_y = blocks_h - 1; // start at bottom row

    while pos + 2 <= chunk.len() && total_blocks > 0 {
        let byte_a = chunk[pos] as u16;
        let byte_b = chunk[pos + 1] as u16;
        pos += 2;

        // Zero is also a valid multi-color flag word. The end marker is only
        // meaningful after all blocks, when this loop has already finished.
        // Treating it as an unconditional terminator freezes the rest of many
        // BANNERHI frames (the first occurrence is frame 2).

        // Skip blocks: (byte_b & 0xFC) == 0x84
        if (byte_b & 0xFC) == 0x84 {
            let skip = (((byte_b - 0x84) << 8) + byte_a) as usize;
            // Current block counts as first skipped, so advance skip-1 additional
            let advance = if skip > 0 { skip - 1 } else { 0 };
            for _ in 0..advance {
                block_x += 1;
                if block_x >= blocks_w {
                    block_x = 0;
                    if block_y == 0 {
                        return;
                    }
                    block_y -= 1;
                }
                total_blocks = total_blocks.saturating_sub(1);
            }
        } else if byte_b >= 0x80 {
            // 1-color fill: the raw 16-bit word IS the color (keep high bit)
            let color = bgr555_to_rgba(byte_b << 8 | byte_a);
            fill_block(fb, width, height, block_x * 4, block_y * 4, &color);
        } else {
            // Multi-color block: byte_b < 0x80
            let mut flags = (byte_b << 8) | byte_a;

            if pos + 4 > chunk.len() {
                return;
            }
            let color_a = read_u16(chunk, pos);
            let color_b = read_u16(chunk, pos + 2);
            pos += 4;

            if color_a & 0x8000 != 0 {
                // 8-color block: colors[0..1] read, read 6 more (8 total)
                if pos + 12 > chunk.len() {
                    return;
                }
                let mut colors = [0u16; 8];
                colors[0] = color_a;
                colors[1] = color_b;
                for color in &mut colors[2..] {
                    *color = read_u16(chunk, pos);
                    pos += 2;
                }

                let bx = block_x * 4;
                let by = block_y * 4;
                for py in 0..4u16 {
                    for px in 0..4u16 {
                        let base = ((py & 2) << 1) + (px & 2);
                        let idx = (base + ((flags & 1) ^ 1)) as usize;
                        let c = bgr555_to_rgba(colors[idx] & 0x7FFF);
                        // py=0 is bottom of block in BMP; map to top-down
                        let screen_y = by + (3 - py as usize);
                        set_pixel(fb, width, height, bx + px as usize, screen_y, &c);
                        flags >>= 1;
                    }
                }
            } else {
                // 2-color block
                let colors = [bgr555_to_rgba(color_a), bgr555_to_rgba(color_b)];

                let bx = block_x * 4;
                let by = block_y * 4;
                for py in 0..4u16 {
                    for px in 0..4u16 {
                        let idx = ((flags & 1) ^ 1) as usize;
                        let screen_y = by + (3 - py as usize);
                        set_pixel(fb, width, height, bx + px as usize, screen_y, &colors[idx]);
                        flags >>= 1;
                    }
                }
            }
        }

        // Advance to next block
        block_x += 1;
        if block_x >= blocks_w {
            block_x = 0;
            if block_y == 0 && total_blocks <= 1 {
                break;
            }
            block_y = block_y.saturating_sub(1);
        }
        total_blocks = total_blocks.saturating_sub(1);
    }
}

/// Convert RGB555 (15-bit, 5 bits each R:14-10 G:9-5 B:4-0) to RGBA32.
fn bgr555_to_rgba(bgr: u16) -> [u8; 4] {
    let b5 = (bgr & 0x1F) as u8;
    let g5 = ((bgr >> 5) & 0x1F) as u8;
    let r5 = ((bgr >> 10) & 0x1F) as u8;
    [
        (r5 << 3) | (r5 >> 2),
        (g5 << 3) | (g5 >> 2),
        (b5 << 3) | (b5 >> 2),
        255,
    ]
}

/// Set a single pixel in the top-down RGBA framebuffer.
fn set_pixel(fb: &mut [u8], width: usize, height: usize, x: usize, y: usize, color: &[u8; 4]) {
    if x < width && y < height {
        let offset = (y * width + x) * 4;
        if offset + 3 < fb.len() {
            fb[offset..offset + 4].copy_from_slice(color);
        }
    }
}

/// Fill a 4x4 block with a single color.
fn fill_block(fb: &mut [u8], width: usize, height: usize, bx: usize, by: usize, color: &[u8; 4]) {
    for dy in 0..4 {
        for dx in 0..4 {
            set_pixel(fb, width, height, bx + dx, by + dy, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bgr555_to_rgba() {
        // Black
        assert_eq!(bgr555_to_rgba(0x0000), [0, 0, 0, 255]);
        // White
        assert_eq!(bgr555_to_rgba(0x7FFF), [255, 255, 255, 255]);
        // Pure red (R in bits 10-14)
        let red = bgr555_to_rgba(0x7C00);
        assert_eq!(red[0], 255); // R
        assert_eq!(red[1], 0); // G
        assert_eq!(red[2], 0); // B
                               // Pure blue (B in bits 0-4)
        let blue = bgr555_to_rgba(0x001F);
        assert_eq!(blue[0], 0);
        assert_eq!(blue[1], 0);
        assert_eq!(blue[2], 255);
    }

    #[test]
    fn test_fill_block() {
        let mut fb = vec![0u8; 8 * 8 * 4];
        fill_block(&mut fb, 8, 8, 0, 0, &[255, 0, 0, 255]);
        // Check corner pixels
        assert_eq!(&fb[0..4], &[255, 0, 0, 255]);
        let offset = (3 * 8 + 3) * 4;
        assert_eq!(&fb[offset..offset + 4], &[255, 0, 0, 255]);
        // Pixel outside block should be black
        let (row, col) = (0usize, 4usize);
        let outside = (row * 8 + col) * 4;
        assert_eq!(&fb[outside..outside + 4], &[0, 0, 0, 0]);
    }

    #[test]
    fn test_not_riff() {
        assert!(AviPlayer::open(b"NOT_RIFF_DATA").is_err());
    }

    #[test]
    fn test_not_avi() {
        let mut data = vec![0u8; 12];
        data[0..4].copy_from_slice(b"RIFF");
        data[4..8].copy_from_slice(&100u32.to_le_bytes());
        data[8..12].copy_from_slice(b"WAVE");
        assert!(AviPlayer::open(&data).is_err());
    }

    #[test]
    fn test_minimal_avi_parses() {
        let data = build_test_avi(8, 8, &[]);
        let player = AviPlayer::open(&data).unwrap();
        assert_eq!(player.info.width, 8);
        assert_eq!(player.info.height, 8);
        assert_eq!(player.info.fps, 15.0);
        assert_eq!(player.frame_count(), 0);
    }

    #[test]
    fn test_single_frame_decode() {
        // Build a frame with a single 1-color block (8x8 = 4 blocks, fill all with red)
        // 1-color: byte_b >= 0x80 (not skip range), raw 16-bit word is the color
        let red_bgr555: u16 = 0x7C00 | 0x8000; // R=31 in bits 10-14, high bit set
        let mut frame_data = Vec::new();
        for _ in 0..4 {
            frame_data.extend_from_slice(&red_bgr555.to_le_bytes());
        }
        // End of frame marker
        frame_data.extend_from_slice(&0u16.to_le_bytes());

        let avi_data = build_test_avi(8, 8, &frame_data);
        let mut player = AviPlayer::open(&avi_data).unwrap();
        assert_eq!(player.frame_count(), 1);

        let fb = player.decode_frame(0);
        // All pixels should be red
        assert_eq!(fb[0], 255); // R
        assert_eq!(fb[1], 0); // G
        assert_eq!(fb[2], 0); // B
        assert_eq!(fb[3], 255); // A
    }

    #[test]
    fn test_2color_block() {
        // 4x4 image = 1 block. 2-color block:
        //   flags (2 bytes) + color_a (2 bytes) + color_b (2 bytes)
        // flags byte_b must be < 0x80 for multi-color path.
        // flags = 0x7FFF: bit 15 is 0 (byte_b=0x7F), bits 0-14 are 1.
        // Per ffmpeg: idx = (bit ^ 1). bit=1 -> idx=0 (color_a), bit=0 -> idx=1 (color_b).
        // Pixels processed bottom-to-top: py=0..3, px=0..3. flags >>= 1 each pixel.
        // Bit 0 (first pixel, py=0 px=0 = screen bottom-left) = 1 -> colors[0] = red.
        // Screen pixel (0,0) = py=3,px=0 = bit 12. flags bit 12 = 1 -> colors[0] = red.
        let mut frame_data = Vec::new();
        let flags: u16 = 0x7FFF; // byte_b=0x7F < 0x80, 15 bits set
        let color_a: u16 = 0x7C00; // red (R=31 in bits 10-14, no high bit)
        let color_b: u16 = 0x03E0; // green (G=31, no high bit)
        frame_data.extend_from_slice(&flags.to_le_bytes());
        frame_data.extend_from_slice(&color_a.to_le_bytes());
        frame_data.extend_from_slice(&color_b.to_le_bytes());
        frame_data.extend_from_slice(&0u16.to_le_bytes()); // end

        let avi_data = build_test_avi(4, 4, &frame_data);
        let mut player = AviPlayer::open(&avi_data).unwrap();
        let fb = player.decode_frame(0);
        // Screen (0,0) corresponds to py=3,px=0 = bit 12. Bit 12 of 0x7FFF is 1.
        // idx = (1^1) = 0 -> color_a (red)
        assert_eq!(fb[0], 255); // R
        assert_eq!(fb[1], 0); // G
        assert_eq!(fb[2], 0); // B

        // Screen (0,3) corresponds to py=0,px=3 = bit 3. Bit 3 of 0x7FFF is 1.
        // idx = (1^1) = 0 -> color_a (red)
        let (row, col) = (3usize, 0usize);
        let bottom_left = (row * 4 + col) * 4; // pixel (0, 3)
        assert_eq!(fb[bottom_left], 255); // R

        // Bit 15 is 0 -> idx = (0^1) = 1 -> color_b (green)
        // Bit 15 = py=3,px=3. Screen y = 0 + (3-3) = 0, x = 3. So pixel (3, 0).
        let (row, col) = (0usize, 3usize);
        let top_right = (row * 4 + col) * 4; // pixel (3, 0)
        assert_eq!(fb[top_right], 0); // R
        assert_eq!(fb[top_right + 1], 255); // G
    }

    #[test]
    fn test_skip_count() {
        // 8x4 image = 2 blocks. Skip 1 (which skips the current block),
        // then fill second block with blue.
        let mut frame_data = Vec::new();
        // Skip block: byte_b & 0xFC == 0x84, count = ((0x84-0x84)<<8)+1 = 1
        // skip=1 means skip current block only (advance 0 additional)
        frame_data.push(1); // byte_a = 1
        frame_data.push(0x84); // byte_b = 0x84
                               // Blue 1-color fill for second block
        let blue: u16 = 0x001F | 0x8000;
        frame_data.extend_from_slice(&blue.to_le_bytes());
        frame_data.extend_from_slice(&0u16.to_le_bytes()); // end

        let avi_data = build_test_avi(8, 4, &frame_data);
        let mut player = AviPlayer::open(&avi_data).unwrap();
        let fb = player.decode_frame(0);
        // First block (cols 0-3) should be untouched (black)
        assert_eq!(&fb[0..4], &[0, 0, 0, 0]);
        // Second block (cols 4-7, row 0): blue
        let off = 4 * 4; // pixel (4, 0)
        assert_eq!(fb[off], 0); // R
        assert_eq!(fb[off + 1], 0); // G
        assert_eq!(fb[off + 2], 255); // B
    }

    #[test]
    fn zero_flags_decode_two_color_block_and_continue() {
        // A zero flag word selects color B for all pixels; it is not an end
        // marker while either of these two blocks still needs to be decoded.
        let frame = [0, 0, 0, 0x7c, 0x1f, 0, 0xe0, 0x83, 0, 0];
        let mut player = AviPlayer::open(&build_test_avi(8, 4, &frame)).unwrap();
        let pixels = player.decode_frame(0);
        assert_eq!(&pixels[0..4], &[0, 0, 255, 255]);
        assert_eq!(&pixels[16..20], &[0, 255, 0, 255]);
    }

    #[test]
    fn zero_flags_decode_eight_color_quadrants_and_continue() {
        let mut frame = vec![0, 0];
        // The first color's high bit selects eight-color mode. Zero flags
        // select the odd color in each bottom-left/right, top-left/right pair.
        for color in [0x8000u16, 0x7c00, 0, 0x03e0, 0, 0x001f, 0, 0x7fff] {
            frame.extend_from_slice(&color.to_le_bytes());
        }
        frame.extend_from_slice(&0x801fu16.to_le_bytes());
        frame.extend_from_slice(&[0, 0]);
        let mut player = AviPlayer::open(&build_test_avi(8, 4, &frame)).unwrap();
        let pixels = player.decode_frame(0);
        for (x, y, expected) in [
            (0, 3, [255, 0, 0, 255]),
            (2, 3, [0, 255, 0, 255]),
            (0, 0, [0, 0, 255, 255]),
            (2, 0, [255, 255, 255, 255]),
            (4, 0, [0, 0, 255, 255]),
        ] {
            let offset = (y * 8 + x) * 4;
            assert_eq!(&pixels[offset..offset + 4], &expected);
        }
    }

    #[test]
    fn delayed_first_request_and_skipped_presentations_decode_dependencies() {
        let red = [0, 0xfc, 0, 0xfc];
        let green_left = [0xe0, 0x83, 1, 0x84];
        let blue_right = [1, 0x84, 0x1f, 0x80];
        let data = build_test_avi_frames(8, 4, &[&red, &green_left, &blue_right]);
        for start_at_zero in [false, true] {
            let mut player = AviPlayer::open(&data).unwrap();
            if start_at_zero {
                player.decode_frame(0);
            }
            let pixels = player.decode_frame(2);
            assert_eq!(&pixels[0..4], &[0, 255, 0, 255]);
            assert_eq!(&pixels[16..20], &[0, 0, 255, 255]);
        }
    }

    #[test]
    fn repeated_and_empty_frames_hold_the_previous_image() {
        let red = [0, 0xfc, 0, 0xfc];
        let green_left = [0xe0, 0x83, 1, 0x84];
        let data = build_test_avi_frames(8, 4, &[&red, &[], &green_left, &[]]);
        let mut player = AviPlayer::open(&data).unwrap();
        let first = player.decode_frame(0).to_vec();
        assert_eq!(player.decode_frame(0), first);
        assert_eq!(player.decode_frame(1), first);
        let last = player.decode_frame(3).to_vec();
        assert_eq!(&last[0..4], &[0, 255, 0, 255]);
        assert_eq!(&last[16..20], &[255, 0, 0, 255]);
        assert_eq!(player.decode_frame(3), last);
        assert_eq!(player.decode_frame(usize::MAX), last);
        assert_eq!(player.decode_frame(1), first);
    }

    #[test]
    fn backward_seek_does_not_retain_future_delta_pixels() {
        let red_right = [1, 0x84, 0, 0xfc];
        let blue_left = [0x1f, 0x80, 1, 0x84];
        let data = build_test_avi_frames(8, 4, &[&red_right, &blue_left]);
        let mut player = AviPlayer::open(&data).unwrap();
        assert_eq!(&player.decode_frame(1)[0..4], &[0, 0, 255, 255]);
        let first = player.decode_frame(0);
        assert_eq!(&first[0..4], &[0, 0, 0, 0]);
        assert_eq!(&first[16..20], &[255, 0, 0, 255]);
    }

    /// Build a minimal valid AVI file with the given dimensions and a single video frame.
    fn build_test_avi(w: u32, h: u32, frame_data: &[u8]) -> Vec<u8> {
        if frame_data.is_empty() {
            build_test_avi_frames(w, h, &[])
        } else {
            build_test_avi_frames(w, h, &[frame_data])
        }
    }

    fn build_test_avi_frames(w: u32, h: u32, frames: &[&[u8]]) -> Vec<u8> {
        let mut data = Vec::new();

        // RIFF header
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&0u32.to_le_bytes()); // placeholder
        data.extend_from_slice(b"AVI ");

        // hdrl LIST
        let hdrl_start = data.len();
        data.extend_from_slice(b"LIST");
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(b"hdrl");

        // avih
        data.extend_from_slice(b"avih");
        data.extend_from_slice(&56u32.to_le_bytes());
        let mut avih = vec![0u8; 56];
        avih[0..4].copy_from_slice(&66666u32.to_le_bytes()); // ~15fps
        avih[16..20].copy_from_slice(&(frames.len() as u32).to_le_bytes());
        avih[32..36].copy_from_slice(&w.to_le_bytes());
        avih[36..40].copy_from_slice(&h.to_le_bytes());
        data.extend_from_slice(&avih);

        // strl LIST
        let strl_start = data.len();
        data.extend_from_slice(b"LIST");
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(b"strl");

        // strh
        data.extend_from_slice(b"strh");
        data.extend_from_slice(&48u32.to_le_bytes());
        let mut strh = vec![0u8; 48];
        strh[0..4].copy_from_slice(b"vids");
        strh[4..8].copy_from_slice(b"CRAM");
        strh[20..24].copy_from_slice(&1u32.to_le_bytes());
        strh[24..28].copy_from_slice(&15u32.to_le_bytes());
        data.extend_from_slice(&strh);

        // Fix strl size
        let strl_size = data.len() - strl_start - 8;
        data[strl_start + 4..strl_start + 8].copy_from_slice(&(strl_size as u32).to_le_bytes());

        // Fix hdrl size
        let hdrl_size = data.len() - hdrl_start - 8;
        data[hdrl_start + 4..hdrl_start + 8].copy_from_slice(&(hdrl_size as u32).to_le_bytes());

        // movi LIST
        let movi_start = data.len();
        data.extend_from_slice(b"LIST");
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(b"movi");

        for frame_data in frames {
            data.extend_from_slice(b"00dc");
            data.extend_from_slice(&(frame_data.len() as u32).to_le_bytes());
            data.extend_from_slice(frame_data);
            if data.len() % 2 != 0 {
                data.push(0);
            }
        }

        let movi_size = data.len() - movi_start - 8;
        data[movi_start + 4..movi_start + 8].copy_from_slice(&(movi_size as u32).to_le_bytes());

        // Fix RIFF size
        let riff_size = data.len() - 8;
        data[4..8].copy_from_slice(&(riff_size as u32).to_le_bytes());

        data
    }
}
