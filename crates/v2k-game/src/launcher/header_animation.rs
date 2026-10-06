//! Playback of the fully prerendered V2K logo and atmospheric background flipbook.
//!
//! Authoring retains the recovered menu pulse at 50 Hz; runtime only selects
//! baked frames. The atlas stays indexed in memory, with one opaque BGRA frame
//! expanded for GDI. Repaints neither allocate nor advance the animation.

use std::io::Cursor;

use serde::Deserialize;

#[derive(Deserialize)]
struct AnimationManifest {
    frame_width: u32,
    frame_height: u32,
    frame_count: u32,
    columns: u32,
    frame_micros: u64,
}

pub(super) struct HeaderFrame {
    pub width: i32,
    pub height: i32,
    /// Top-down, opaque BGRA rows for GDI's SRCCOPY submission.
    pub bgra: Vec<u8>,
}

pub(super) struct HeaderAnimation {
    manifest: AnimationManifest,
    indices: Vec<u8>,
    palette: [[u8; 4]; 256],
    atlas_width: usize,
    loop_micros: u64,
    phase_micros: u64,
    selected_frame: usize,
    cached: HeaderFrame,
}

impl HeaderAnimation {
    pub fn load() -> Result<Self, String> {
        Self::decode(
            include_bytes!("../../data/branding/header-animation.json"),
            include_bytes!("../../data/branding/openv2k-header-animation.png"),
        )
    }

    fn decode(manifest_bytes: &[u8], png_bytes: &[u8]) -> Result<Self, String> {
        let manifest: AnimationManifest = serde_json::from_slice(manifest_bytes)
            .map_err(|error| format!("Cannot read launcher animation manifest: {error}"))?;
        let (atlas_width, atlas_height, loop_micros) = manifest.layout()?;
        let mut decoder = png::Decoder::new(Cursor::new(png_bytes));
        // Preserve palette indices; expanding all 192 frames to RGBA would
        // quadruple the animation's resident pixel memory.
        decoder.set_transformations(png::Transformations::IDENTITY);
        let mut reader = decoder
            .read_info()
            .map_err(|error| format!("Cannot read launcher animation atlas: {error}"))?;
        let info = reader.info();
        if info.width != atlas_width || info.height != atlas_height {
            return Err("Launcher animation atlas dimensions do not match its manifest".into());
        }
        if info.color_type != png::ColorType::Indexed || info.bit_depth != png::BitDepth::Eight {
            return Err("Launcher animation atlas must use 8-bit indexed color".into());
        }
        if info.trns.is_some() {
            return Err("Launcher animation atlas must be opaque, without transparency".into());
        }
        let colors = info
            .palette
            .as_deref()
            .ok_or_else(|| "Launcher animation atlas has no palette".to_string())?;
        if colors.is_empty() || colors.len() > 768 || colors.len() % 3 != 0 {
            return Err("Launcher animation atlas has an invalid RGB palette".into());
        }
        let color_count = colors.len() / 3;
        let mut palette = [[0, 0, 0, 255]; 256];
        for (entry, color) in palette.iter_mut().zip(colors.chunks_exact(3)) {
            *entry = [color[2], color[1], color[0], 255];
        }
        let mut indices = vec![0; reader.output_buffer_size()];
        let output = reader
            .next_frame(&mut indices)
            .map_err(|error| format!("Cannot decode launcher animation atlas: {error}"))?;
        indices.truncate(output.buffer_size());
        let expected_pixels = usize::try_from(u64::from(atlas_width) * u64::from(atlas_height))
            .map_err(|_| "Launcher animation atlas is too large".to_string())?;
        if indices.len() != expected_pixels
            || indices
                .iter()
                .any(|index| usize::from(*index) >= color_count)
        {
            return Err("Launcher animation atlas contains invalid palette indices".into());
        }
        let frame_bytes =
            usize::try_from(u64::from(manifest.frame_width) * u64::from(manifest.frame_height) * 4)
                .map_err(|_| "Launcher animation frame is too large".to_string())?;
        let cached = HeaderFrame {
            width: manifest.frame_width as i32,
            height: manifest.frame_height as i32,
            bgra: vec![0; frame_bytes],
        };
        let mut animation = Self {
            manifest,
            indices,
            palette,
            atlas_width: atlas_width as usize,
            loop_micros,
            phase_micros: 0,
            selected_frame: 0,
            cached,
        };
        animation.expand_selected_frame();
        Ok(animation)
    }

    pub fn frame(&self) -> &HeaderFrame {
        &self.cached
    }

    /// Select directly from elapsed time, including after a delayed timer.
    /// Returns whether a different baked frame needs its banner repainted.
    pub fn advance(&mut self, elapsed_micros: u64) -> bool {
        self.phase_micros = ((u128::from(self.phase_micros) + u128::from(elapsed_micros))
            % u128::from(self.loop_micros)) as u64;
        let selected = (self.phase_micros / self.manifest.frame_micros) as usize;
        if selected == self.selected_frame {
            return false;
        }
        self.selected_frame = selected;
        self.expand_selected_frame();
        true
    }

    fn expand_selected_frame(&mut self) {
        let width = self.manifest.frame_width as usize;
        let height = self.manifest.frame_height as usize;
        let columns = self.manifest.columns as usize;
        let left = self.selected_frame % columns * width;
        let top = self.selected_frame / columns * height;
        for y in 0..height {
            let source = (top + y) * self.atlas_width + left;
            let destination = y * width * 4;
            let row = &mut self.cached.bgra[destination..destination + width * 4];
            for (pixel, index) in row
                .chunks_exact_mut(4)
                .zip(&self.indices[source..source + width])
            {
                pixel.copy_from_slice(&self.palette[usize::from(*index)]);
            }
        }
    }
}

impl AnimationManifest {
    fn layout(&self) -> Result<(u32, u32, u64), String> {
        if self.frame_width == 0
            || self.frame_height == 0
            || self.frame_width > i32::MAX as u32
            || self.frame_height > i32::MAX as u32
            || self.frame_count == 0
            || self.columns == 0
            || self.columns > self.frame_count
            || self.frame_micros == 0
        {
            return Err(
                "Launcher animation needs positive frame dimensions, count and timing".into(),
            );
        }
        let rows =
            self.frame_count / self.columns + u32::from(self.frame_count % self.columns != 0);
        let width = self.frame_width.checked_mul(self.columns);
        let height = self.frame_height.checked_mul(rows);
        let duration = self.frame_micros.checked_mul(u64::from(self.frame_count));
        match (width, height, duration) {
            (Some(width), Some(height), Some(duration)) => Ok((width, height, duration)),
            _ => Err("Launcher animation dimensions or duration overflow their format".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &[u8] = br#"{
        "frame_width":2,"frame_height":1,"frame_count":4,
        "columns":2,"frame_micros":20000
    }"#;

    fn atlas(transparent: bool, indices: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 4, 2);
            encoder.set_color(png::ColorType::Indexed);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_palette(vec![10, 20, 30, 80, 90, 100]);
            if transparent {
                encoder.set_trns(vec![0, 255]);
            }
            encoder
                .write_header()
                .unwrap()
                .write_image_data(indices)
                .unwrap();
        }
        bytes
    }

    fn fixture() -> HeaderAnimation {
        HeaderAnimation::decode(MANIFEST, &atlas(false, &[0, 1, 1, 0, 1, 1, 0, 0])).unwrap()
    }

    #[test]
    fn indexed_atlas_expands_selected_tiles_to_top_down_opaque_bgra() {
        let mut animation = fixture();
        assert_eq!((animation.frame().width, animation.frame().height), (2, 1));
        assert_eq!(animation.frame().bgra, [30, 20, 10, 255, 100, 90, 80, 255]);
        assert!(animation.advance(20_000));
        assert_eq!(animation.frame().bgra, [100, 90, 80, 255, 30, 20, 10, 255]);
        assert!(animation.advance(20_000));
        assert_eq!(animation.frame().bgra, [100, 90, 80, 255, 100, 90, 80, 255]);
        assert!(animation.advance(20_000));
        assert_eq!(animation.frame().bgra, [30, 20, 10, 255, 30, 20, 10, 255]);
    }

    #[test]
    fn exact_frame_boundaries_and_wrap_select_only_changed_frames() {
        let mut animation = fixture();
        assert!(!animation.advance(0));
        assert!(!animation.advance(19_999));
        assert_eq!(animation.selected_frame, 0);
        assert!(animation.advance(1));
        assert_eq!(animation.selected_frame, 1);
        assert!(animation.advance(59_999));
        assert_eq!(animation.selected_frame, 3);
        assert!(animation.advance(1));
        assert_eq!((animation.selected_frame, animation.phase_micros), (0, 0));
        assert!(!animation.advance(80_000));
    }

    #[test]
    fn delayed_timer_jumps_directly_to_the_same_phase_as_small_ticks() {
        let mut delayed = fixture();
        let mut steady = fixture();
        assert!(delayed.advance(1_020_000));
        for _ in 0..51 {
            steady.advance(20_000);
        }
        assert_eq!(delayed.phase_micros, 60_000);
        assert_eq!(delayed.frame().bgra, steady.frame().bgra);
        delayed.advance(u64::MAX);
        assert_eq!(
            delayed.phase_micros,
            ((60_000_u128 + u128::from(u64::MAX)) % 80_000) as u64
        );
    }

    #[test]
    fn paints_only_read_the_same_cached_frame_and_do_not_move_the_clock() {
        let animation = fixture();
        let address = animation.frame().bgra.as_ptr();
        for _ in 0..10 {
            assert_eq!(animation.frame().bgra.as_ptr(), address);
            assert_eq!(animation.phase_micros, 0);
            assert_eq!(animation.selected_frame, 0);
        }
    }

    #[test]
    fn malformed_manifest_atlas_layout_and_palette_fail_before_window_creation() {
        let bytes = atlas(false, &[0, 1, 1, 0, 1, 1, 0, 0]);
        assert!(HeaderAnimation::decode(br#"{}"#, &bytes).is_err());
        for (field, value) in [
            ("frame_width", 0),
            ("frame_height", 0),
            ("frame_count", 0),
            ("columns", 0),
            ("frame_micros", 0),
            ("columns", 3),
            ("frame_width", 3),
        ] {
            let mut manifest: serde_json::Value = serde_json::from_slice(MANIFEST).unwrap();
            manifest[field] = value.into();
            assert!(
                HeaderAnimation::decode(&serde_json::to_vec(&manifest).unwrap(), &bytes).is_err(),
                "invalid {field}={value}"
            );
        }
        assert!(
            HeaderAnimation::decode(MANIFEST, &atlas(true, &[0, 1, 1, 0, 1, 1, 0, 0])).is_err()
        );
        assert!(
            HeaderAnimation::decode(MANIFEST, &atlas(false, &[2, 1, 1, 0, 1, 1, 0, 0])).is_err()
        );
    }

    #[test]
    fn embedded_flipbook_retains_every_fifty_hz_sample_and_compact_storage() {
        let mut animation = HeaderAnimation::load().unwrap();
        assert_eq!(
            (animation.frame().width, animation.frame().height),
            (624, 98)
        );
        assert_eq!(animation.manifest.frame_count, 192);
        assert_eq!(animation.manifest.frame_micros, 20_000);
        assert_eq!(animation.loop_micros, 3_840_000);
        assert_eq!(animation.indices.len(), 624 * 98 * 192);
        assert_eq!(animation.frame().bgra.len(), 624 * 98 * 4);
        let first = animation.frame().bgra.clone();
        for _ in 0..192 {
            assert!(animation.advance(20_000));
            assert!(animation
                .frame()
                .bgra
                .chunks_exact(4)
                .all(|pixel| pixel[3] == 255));
        }
        assert_eq!(animation.phase_micros, 0);
        assert_eq!(animation.frame().bgra, first);
    }
}
