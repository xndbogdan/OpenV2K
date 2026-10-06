//! Native font baseline/pen, bar and row clipping in one authored UI mapping.

use super::scale_rgba;
use v2k_render::UiMapping;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MenuTextTone {
    Normal,
    Disabled,
}

impl MenuTextTone {
    const fn rgb_scale(self) -> u16 {
        match self {
            Self::Normal => 255,
            // Keep unavailable rows legible while making their state
            // unmistakable against the ordinary green font.
            Self::Disabled => 96,
        }
    }
}

pub(super) fn apply_menu_text_tone(rgba: &mut [u8], tone: MenuTextTone) {
    let scale = tone.rgb_scale();
    if scale == 255 {
        return;
    }
    for pixel in rgba.chunks_exact_mut(4) {
        for channel in &mut pixel[..3] {
            *channel = (u16::from(*channel) * scale / 255) as u8;
        }
    }
}

/// Draw `text` with an original sprite font at a virtual-space position
/// (x = pen start, y = BASELINE), scaled to the window. `max_chars` limits
/// the drawn prefix (typewriter reveal); `centered` centers on x instead.
pub(super) fn draw_menu_text(
    renderer: &mut dyn v2k_render::Renderer,
    font: &v2k_game::menu_text::MenuFont,
    text: &str,
    position: [f32; 2],
    centered: bool,
    max_chars: usize,
    mapping: UiMapping,
) {
    draw_menu_text_toned(
        renderer,
        font,
        text,
        position,
        centered,
        max_chars,
        mapping,
        MenuTextTone::Normal,
    );
}

pub(super) fn draw_menu_text_toned(
    renderer: &mut dyn v2k_render::Renderer,
    font: &v2k_game::menu_text::MenuFont,
    text: &str,
    position: [f32; 2],
    centered: bool,
    max_chars: usize,
    mapping: UiMapping,
    tone: MenuTextTone,
) {
    let [x, baseline] = position;
    let UiMapping {
        scale: s,
        offset_x: ox,
        offset_y: oy,
    } = mapping;
    let mut pen = if centered {
        x - font.measure(text) / 2.0
    } else {
        x
    };
    for (n, c) in text.chars().enumerate() {
        if n >= max_chars {
            break;
        }
        let Some(g) = font.glyph(c) else { continue };
        if g.width > 0 && g.height > 0 {
            // Baseline blit: y = baseline + yoff − sprite_h + 1 (asset px).
            let x0 = ox + ((pen + g.xoff) * s) as i32;
            let x1 = ox + ((pen + g.xoff + g.width as f32) * s) as i32;
            let y0 = oy + ((baseline + g.yoff - g.height as f32 + 1.0) * s) as i32;
            let y1 = oy + ((baseline + g.yoff + 1.0) * s) as i32;
            let w = (x1 - x0).max(1) as u32;
            let h = (y1 - y0).max(1) as u32;
            let mut scaled = scale_rgba(&g.rgba, g.width, g.height, w, h);
            apply_menu_text_tone(&mut scaled, tone);
            renderer.draw_sprite(&scaled, w, h, x0, y0);
        }
        pen += g.advance + g.kern;
    }
}

/// Draw the original spinner-bar glyph run while retaining a one-virtual-pixel
/// cell break. The source 0x1B..0x1E glyphs are five pixels wide and the port's
/// scaled blits otherwise meet edge-to-edge, visually collapsing the bar into
/// one continuous strip instead of the discrete cells seen in the game.
pub(super) fn draw_menu_bar(
    renderer: &mut dyn v2k_render::Renderer,
    font: &v2k_game::menu_text::MenuFont,
    text: &str,
    x: f32,
    baseline: f32,
    max_chars: usize,
    mapping: UiMapping,
    tone: MenuTextTone,
) {
    let UiMapping {
        scale: s,
        offset_x: ox,
        offset_y: oy,
    } = mapping;
    let total = text.chars().count();
    let mut pen = x;
    for (n, c) in text.chars().enumerate() {
        if n >= max_chars {
            break;
        }
        let Some(g) = font.glyph(c) else { continue };
        if g.width > 0 && g.height > 0 {
            let visual_w = if n + 1 < total {
                g.width.saturating_sub(1).max(1)
            } else {
                g.width
            };
            let x0 = ox + ((pen + g.xoff) * s) as i32;
            let x1 = ox + ((pen + g.xoff + visual_w as f32) * s) as i32;
            let y0 = oy + ((baseline + g.yoff - g.height as f32 + 1.0) * s) as i32;
            let y1 = oy + ((baseline + g.yoff + 1.0) * s) as i32;
            let w = (x1 - x0).max(1) as u32;
            let h = (y1 - y0).max(1) as u32;
            let mut scaled = scale_rgba(&g.rgba, g.width, g.height, w, h);
            apply_menu_text_tone(&mut scaled, tone);
            renderer.draw_sprite(&scaled, w, h, x0, y0);
        }
        pen += g.advance + g.kern;
    }
}

/// Typewriter reveal in characters after the caller selects either the
/// staggered DCEC4 elapsed time or the entering-row DCEC8 elapsed time.
pub(super) fn typewriter_chars_from_elapsed(elapsed_ms: f32, len: usize) -> usize {
    let reveal = (elapsed_ms * 100.0).clamp(0.0, 32767.0);
    ((len + 1) as f32 * reveal / 32768.0) as usize
}

pub(super) fn menu_list_clip_y(
    mapping: UiMapping,
    start_y: f32,
    step_y: f32,
    rows: f32,
    ascent: f32,
    descent: f32,
) -> (i32, i32) {
    let UiMapping {
        scale,
        offset_y: y_offset,
        ..
    } = mapping;
    let top = y_offset + ((start_y + 1.0 - ascent) * scale).floor() as i32;
    let last_baseline = start_y + 1.0 + (rows - 1.0).max(0.0) * step_y;
    let bottom = y_offset + ((last_baseline + descent) * scale).ceil() as i32;
    (top, bottom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_game::{menu_text::MenuFonts, session::GameSession};
    use v2k_render::{UiMappingRequest, UiSubmissionPolicy};

    #[v2k_test_support::retail_test]
    fn selected_high_canvases_share_glyph_metrics_and_scale_complete_layouts() {
        let data = v2k_test_support::retail_dir();
        let session = GameSession::init(&data).expect("retail preload required");
        let mut metrics = None;
        for variant in 1..=3 {
            let level = session
                .load_ovl_by_id(2, variant)
                .expect("selected high menu OVL");
            let fonts = MenuFonts::from_level(&level).unwrap();
            let current = ['A', 'N', 'p', ' ', '\u{1e}'].map(|character| {
                let glyph = fonts.normal.glyph(character).unwrap();
                (
                    glyph.width,
                    glyph.height,
                    glyph.advance,
                    glyph.kern,
                    glyph.xoff,
                    glyph.yoff,
                )
            });
            if let Some(expected) = metrics {
                assert_eq!(current, expected, "high tiers retain native metrics");
            } else {
                metrics = Some(current);
            }
            let mapping = UiMapping::new(UiMappingRequest {
                viewport: [1920, 1080],
                authored_canvas: [fonts.virtual_w as u32, fonts.virtual_h as u32],
                policy: UiSubmissionPolicy::NativeCanvas,
            });
            assert_eq!(mapping.scale, [1.8, 1.8, 1.40625][variant as usize - 1]);
            assert_eq!(fonts.normal.line_step, 25.69);
            assert_eq!(
                fonts.layout_point(4),
                Some((0.0, [27.0, 27.0, 43.0][variant as usize - 1]))
            );
            assert_eq!(fonts.layout_point(12).unwrap().0, fonts.virtual_w / 2.0);
            println!("FONT_TIER variant={variant} canvas=({}, {}) line_step={} points={:?} glyphs={current:?}",
                fonts.virtual_w, fonts.virtual_h, fonts.normal.line_step,
                [3,4,6,7,12].map(|point| fonts.layout_point(point).unwrap()));
        }
    }
}
