//! Fill slots that write the surface without the scan converter.
//!
//! Lines, the unscaled sprite blit, word-image rows, whole-row fills and the
//! clip setter. Each mirrors its retail handler, including two quirks that
//! make parts of them inert: the faded line `FUN_0047B6C0` compares 16.16
//! positions against the pixel clip rectangle (so it only plots points
//! within 1/64 pixel of the origin, at a 16.16-scaled address), and the row
//! fill `FUN_0047B9F0` ignores x extents and rounds its byte count down to
//! 16-byte blocks taken from the end of the range.

use super::material::{flags, MaterialSource};
use super::slots::{channels, Packet};
use super::span::SpanTarget;
use super::state::ClipRect;
use super::{SoftwareRaster, WordImageSource};

impl SoftwareRaster {
    /// `FUN_0047B640`: one horizontal run clipped to the rectangle.
    fn clipped_run(&self, target: &mut SpanTarget<'_>, x: i32, y: i32, length: i32, colour: u16) {
        let clip = self.state.clip;
        let mut length = length;
        let mut x = x;
        if length < 0 || y < i32::from(clip.y0) || y >= i32::from(clip.y1) {
            return;
        }
        if x < i32::from(clip.x0) {
            length = length.wrapping_add(x - i32::from(clip.x0));
            if length < 0 {
                return;
            }
            x = i32::from(clip.x0);
        }
        if length.wrapping_add(x) >= i32::from(clip.x1) {
            length = i32::from(clip.x1) - x;
            if length < 0 {
                return;
            }
        }
        if length > 0 {
            let start = row_word_offset(target.pitch, y).wrapping_add(x as u32) as usize;
            target.pixels[start..start + length as usize].fill(colour);
        }
    }

    /// `FUN_0047B360`: a flat line drawn as one clipped run per scanline.
    pub(super) fn line(&mut self, packet: &Packet<'_>, target: &mut SpanTarget<'_>) {
        let colour = packet.u32(8) as u16;
        let (x0, y0) = (packet.i16(0), packet.i16(2));
        let (x1, y1) = (packet.i16(4), packet.i16(6));
        if y0 == y1 {
            let (left, right) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
            let length = i32::from(right) - i32::from(left) + 1;
            self.clipped_run(target, i32::from(left), i32::from(y0), length, colour);
            return;
        }
        let ((top_x, top_y), (bottom_x, bottom_y)) = if y1 < y0 {
            ((x1, y1), (x0, y0))
        } else {
            ((x0, y0), (x1, y1))
        };
        let (top_x, top_y) = (i32::from(top_x), i32::from(top_y));
        let (bottom_x, bottom_y) = (i32::from(bottom_x), i32::from(bottom_y));
        let slope = (bottom_x.wrapping_sub(top_x) << 16) / (bottom_y - top_y);
        let mut x = top_x << 16;
        if slope >= 0 {
            if slope <= 0x10000 {
                let mut y = top_y;
                while y < bottom_y {
                    self.clipped_run(target, x >> 16, y, 1, colour);
                    x = x.wrapping_add(slope);
                    y += 1;
                }
                self.clipped_run(target, bottom_x, y, 1, colour);
            } else {
                let mut y = top_y;
                let mut next = x.wrapping_add(slope);
                while y < bottom_y {
                    self.clipped_run(target, x >> 16, y, (next >> 16) - (x >> 16), colour);
                    next = next.wrapping_add(slope);
                    x = x.wrapping_add(slope);
                    y += 1;
                }
                self.clipped_run(target, x >> 16, y, bottom_x - (x >> 16) + 1, colour);
            }
        } else if slope >= -0x10000 {
            let mut y = top_y;
            while y < bottom_y {
                self.clipped_run(target, x >> 16, y, 1, colour);
                x = x.wrapping_add(slope);
                y += 1;
            }
            self.clipped_run(target, bottom_x, y, 1, colour);
        } else {
            self.clipped_run(target, top_x, top_y, 1, colour);
            let mut y = top_y + 1;
            let mut next = x.wrapping_add(slope);
            while y <= bottom_y {
                self.clipped_run(target, next >> 16, y, (x >> 16) - (next >> 16), colour);
                next = next.wrapping_add(slope);
                x = x.wrapping_add(slope);
                y += 1;
            }
        }
    }

    /// `FUN_0047B6C0`: a line whose two ends fade colour A toward colour B
    /// by the bytes at `+0x14`/`+0x15`.
    pub(super) fn faded_line(&mut self, packet: &Packet<'_>, target: &mut SpanTarget<'_>) {
        let format = self.format;
        let (x0, y0) = (i32::from(packet.i16(0)), i32::from(packet.i16(2)));
        let dx = i32::from(packet.i16(4)) - x0;
        let dy = i32::from(packet.i16(6)) - y0;
        let steps = dx.wrapping_abs().max(dy.wrapping_abs());
        let a = channels(packet.u32(8), format);
        let b = channels(packet.u32(0x10), format);
        // Blue extraction shifts only the low byte (`shl dl, cl`).
        let a_blue = i32::from(
            (packet
                .u8(8)
                .wrapping_shl(u32::from(format.blue_shift).wrapping_sub(1) & 31))
                & 0xF8,
        );
        let b_blue = i32::from(
            (packet
                .u8(0x10)
                .wrapping_shl(u32::from(format.blue_shift).wrapping_sub(1) & 31))
                & 0xF8,
        );
        let a = [a[0], a[1], a_blue];
        let b = [b[0], b[1], b_blue];
        let mix = |weight: i32| -> [i32; 3] {
            std::array::from_fn(|c| {
                (b[c]
                    .wrapping_sub(a[c])
                    .wrapping_mul(weight)
                    .wrapping_add(a[c] << 8))
                    << 8
            })
        };
        let start = mix(i32::from(packet.u8(0x14)) + (i32::from(packet.u8(0x14)) >> 7));
        let mut x = x0 << 16;
        let mut y = y0 << 16;
        if steps == 0 {
            self.faded_point(target, x, y, start);
            return;
        }
        let end = mix(i32::from(packet.u8(0x15)) + (i32::from(packet.u8(0x15)) >> 7));
        let x_step = (dx << 16) / steps;
        let y_step = (dy << 16) / steps;
        let colour_step: [i32; 3] = std::array::from_fn(|c| end[c].wrapping_sub(start[c]) / steps);
        let mut colour = start;
        for _ in 0..=steps {
            self.faded_point(target, x, y, colour);
            x = x.wrapping_add(x_step);
            y = y.wrapping_add(y_step);
            for c in 0..3 {
                colour[c] = colour[c].wrapping_add(colour_step[c]);
            }
        }
    }

    /// One faded-line point: the 16.16 position is tested against the
    /// pixel clip rectangle and used as the pixel address.
    fn faded_point(&self, target: &mut SpanTarget<'_>, x: i32, y: i32, colour: [i32; 3]) {
        let clip = self.state.clip;
        if x < i32::from(clip.x0)
            || x >= i32::from(clip.x1)
            || y < i32::from(clip.y0)
            || y >= i32::from(clip.y1)
        {
            return;
        }
        let format = self.format;
        let pixel = ((((colour[2] >> 16) & 0xF8)
            >> (u32::from(format.blue_shift).wrapping_sub(1) & 31))
            | (((colour[0] >> 16) & 0xF8) << ((u32::from(format.red_shift) + 1) & 31))
            | (((colour[1] >> 16) & 0xF8) << ((u32::from(format.green_shift) + 1) & 31)))
            as u16;
        let index = row_word_offset(target.pitch, y).wrapping_add(x as u32) as usize;
        // Retail writes wherever the scaled address lands; outside the
        // surface that would corrupt process memory instead.
        if let Some(word) = target.pixels.get_mut(index) {
            *word = pixel;
        }
    }

    /// `FUN_0047AB20`: clamp a requested rectangle to the surface.
    pub(super) fn set_clip_packet(&mut self, packet: &Packet<'_>, target: &mut SpanTarget<'_>) {
        let (x0, y0, x1, y1) = (packet.i16(0), packet.i16(2), packet.i16(4), packet.i16(6));
        if x1 < x0 || y1 < y0 {
            return;
        }
        let width = target.width as i32;
        let height = target.height as i32;
        self.state.clip = ClipRect {
            x0: x0.max(0),
            y0: y0.max(0),
            x1: if width < i32::from(x1) {
                width as i16
            } else {
                x1
            },
            y1: if height < i32::from(y1) {
                height as i16
            } else {
                y1
            },
        };
    }

    /// `FUN_0047B9F0`: fill `height` whole rows from `(x0 / 4)` dwords into
    /// row `y0`, in 16-byte blocks counted back from the end of the range.
    pub(super) fn fill_rows(&mut self, packet: &Packet<'_>, target: &mut SpanTarget<'_>) {
        let pitch = target.pitch as u32;
        let x0 = i32::from(packet.i16(0));
        let start = (i32::from(packet.i16(2)).wrapping_mul(pitch as i32) as u32 >> 2)
            .wrapping_add((x0 / 4) as u32);
        let end =
            (i32::from(packet.i16(8)).wrapping_mul(pitch as i32) as u32 >> 2).wrapping_add(start);
        let colour = packet.u16(4);
        let blocks = end.wrapping_sub(start).wrapping_mul(4) >> 4;
        let words = target.pixels.len();
        for block in 0..blocks {
            let block_end = (end as usize * 2).wrapping_sub(block as usize * 8);
            let block_start = block_end.wrapping_sub(8);
            if block_end <= words {
                target.pixels[block_start..block_end].fill(colour);
            }
        }
    }

    /// `FUN_0047AC30`: copy a `width × height` array of dwords (low words
    /// written; zero dwords skipped when keyed). Draws nothing unless the
    /// whole image lies inside the clip rectangle.
    pub(super) fn word_image(
        &mut self,
        packet: &Packet<'_>,
        target: &mut SpanTarget<'_>,
        images: &dyn WordImageSource,
    ) {
        let clip = self.state.clip;
        let x = i32::from(packet.i16(0));
        let y = i32::from(packet.i16(2));
        let width = u32::from(packet.u16(8));
        let height = u32::from(packet.u16(0xA));
        if !(i32::from(clip.x0) <= x
            && i32::from(clip.y0) <= y
            && width as i32 + x <= i32::from(clip.x1)
            && height as i32 + y <= i32::from(clip.y1))
        {
            return;
        }
        let image = images.word_image(packet.u32(4));
        let keyed = packet.u8(0xC) & 1 != 0;
        for row in 0..height {
            let base = row_word_offset(target.pitch, row as i32 + y);
            for column in 0..width {
                let value = image[(row * width + column) as usize];
                if keyed && value == 0 {
                    continue;
                }
                let index = base.wrapping_add(column).wrapping_add(x as u32) as usize;
                target.pixels[index] = value as u16;
            }
        }
    }

    /// `FUN_0047AD90`: unscaled sprite at `(+4, +6)`, clipped per pixel;
    /// flag 0x08 adds the texel to half the destination.
    pub(super) fn sprite(
        &mut self,
        packet: &Packet<'_>,
        target: &mut SpanTarget<'_>,
        materials: &dyn MaterialSource,
    ) {
        let material = materials.material(packet.u32(0));
        let clip = self.state.clip;
        let x = i32::from(packet.i16(4));
        let y = i32::from(packet.i16(6));
        let right = i32::from(material.width as i16) + x;
        let bottom = i32::from(material.height as i16) + y;
        if !(i32::from(clip.x0) < right
            && i32::from(clip.y0) < bottom
            && x < i32::from(clip.x1)
            && y < i32::from(clip.y1))
        {
            return;
        }
        let start_x = x.max(i32::from(clip.x0));
        let (skip_rows, start_y) = if y < i32::from(clip.y0) {
            (i32::from(clip.y0) - y, i32::from(clip.y0))
        } else {
            (0, y)
        };
        let skip_columns = if x < i32::from(clip.x0) {
            i32::from(clip.x0) - x
        } else {
            0
        };
        let rows = if i32::from(clip.y1) <= bottom {
            i32::from(clip.y1) - y
        } else {
            i32::from(material.height)
        };
        let columns = if i32::from(clip.x1) <= right {
            i32::from(clip.x1) - x
        } else {
            i32::from(material.width)
        };
        if skip_rows >= rows {
            return;
        }
        let half = material.flags & flags::HALF_ADDITIVE != 0;
        let keyed = material.flags & flags::KEYED != 0;
        let raw = material.flags & flags::RAW != 0;
        let palette = if material.flags & flags::ROW_28 != 0 {
            0x380 / 2
        } else {
            0
        };
        let mask = self.format.halved_pixel_mask();
        let row_stride = target.pitch & !1;
        let mut row_start =
            row_word_offset(target.pitch, start_y).wrapping_add(start_x as u32) as usize;
        for row in skip_rows..rows {
            let mut dst = row_start;
            for column in skip_columns..columns {
                let source = if raw {
                    let word = material.texel_word((row * 0x400 + column) as u32);
                    if keyed && word == 0 {
                        dst += 1;
                        continue;
                    }
                    word
                } else {
                    let texel = material.texel_byte((row * 0x800 + column) as u32);
                    if keyed && texel == 0 {
                        dst += 1;
                        continue;
                    }
                    material.palette_word(palette, u32::from(texel))
                };
                let pixel = &mut target.pixels[dst];
                *pixel = if half {
                    source.wrapping_add((*pixel >> 1) & mask)
                } else {
                    source
                };
                dst += 1;
            }
            row_start += row_stride / 2;
        }
    }
}

/// `((pitch * y) >> 1)` with an unsigned shift, in surface words.
fn row_word_offset(pitch: usize, y: i32) -> u32 {
    ((pitch as u32).wrapping_mul(y as u32)) >> 1
}
