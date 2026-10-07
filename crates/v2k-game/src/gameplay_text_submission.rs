//! Authored percentage messages and the High Native world-text adaptation.
//!
//! Intrinsic Section-4 metrics, typewriter state and message grammar remain
//! independent of output size. The world overlay has actual drawable anchors;
//! Overlay51 keeps the same complete-canvas mapping as its backdrop.

use super::{draw_menu_text, GameplayNotificationLine};
use v2k_game::menu_text::{MenuFont, MenuFonts};
use v2k_render::{Renderer, UiAnchor, UiMapping, UiMappingRequest, UiSubmissionPolicy};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GameplayTextContext {
    WorldOverlay,
    AuthoredCanvas,
}

pub(super) fn draw_notifications(
    renderer: &mut dyn Renderer,
    fonts: &MenuFonts,
    notifications: &[GameplayNotificationLine],
    context: GameplayTextContext,
) {
    let (width, height) = renderer.viewport_size();
    let request = UiMappingRequest {
        viewport: [width, height],
        authored_canvas: [fonts.virtual_w as u32, fonts.virtual_h as u32],
        policy: renderer.ui_submission_policy(),
    };
    let native_world = context == GameplayTextContext::WorldOverlay
        && matches!(
            request.policy,
            UiSubmissionPolicy::NativeCanvas | UiSubmissionPolicy::NativeHud { .. }
        );
    // Preserve each original matching display exactly, including centered
    // single-line messages and their authored wrap widths. Low/Fit always
    // retain this same retail submission grammar.
    if !native_world || request.viewport == request.authored_canvas {
        draw_authored_notifications(renderer, fonts, notifications, UiMapping::new(request));
        return;
    }
    if width == 0 || height == 0 {
        return;
    }

    let glyph_mapping = UiMapping::new(UiMappingRequest {
        policy: request.policy.for_gameplay_hud(UiAnchor::TopLeft),
        ..request
    });
    renderer.set_sprite_clip(Some((0, 0, width, height)));
    for notification in notifications {
        draw_world_notification(
            renderer,
            &fonts.selected,
            notification,
            request.viewport,
            glyph_mapping,
        );
    }
    renderer.set_sprite_clip(None);
}

/// FUN_00452790/FUN_00470D00's selected-font percentage and baseline path.
fn draw_authored_notifications(
    renderer: &mut dyn Renderer,
    fonts: &MenuFonts,
    notifications: &[GameplayNotificationLine],
    mapping: UiMapping,
) {
    let font = &fonts.selected;
    for notification in notifications {
        let baseline = fonts.virtual_h * notification.baseline_percent as f32 / 100.0;
        if notification.center_x {
            draw_menu_text(
                renderer,
                font,
                &notification.text,
                [fonts.virtual_w * 0.5, baseline],
                true,
                usize::MAX,
                mapping,
            );
            continue;
        }
        let pen_x = fonts.virtual_w * notification.x_percent as f32 / 100.0;
        let wrap_width = fonts.virtual_w * notification.width_percent as f32 / 100.0;
        for (index, line) in wrap_notification(font, &notification.text, wrap_width)
            .iter()
            .enumerate()
        {
            draw_menu_text(
                renderer,
                font,
                line,
                [pen_x, baseline + index as f32 * font.line_step],
                false,
                usize::MAX,
                mapping,
            );
        }
    }
}

/// Port-owned High Native readability policy. Percentages name drawable
/// positions while glyph metrics remain font pixels, converted explicitly by
/// the shared HUD density. Wrapped blocks move only enough to remain visible.
fn draw_world_notification(
    renderer: &mut dyn Renderer,
    font: &MenuFont,
    notification: &GameplayNotificationLine,
    viewport: [u32; 2],
    mapping: UiMapping,
) {
    let [width, height] = viewport.map(|value| value as f32);
    let pen_pixels = if notification.center_x {
        width * 0.5
    } else {
        (width * notification.x_percent as f32 / 100.0).clamp(0.0, width - 1.0)
    };
    let available_width = if notification.center_x {
        width
    } else {
        width - pen_pixels
    };
    let wrap_pixels =
        (width * notification.width_percent as f32 / 100.0).clamp(1.0, available_width.max(1.0));
    let lines = wrap_notification(font, &notification.text, wrap_pixels / mapping.scale);
    let mut top = f32::INFINITY;
    let mut bottom = f32::NEG_INFINITY;
    for (index, line) in lines.iter().enumerate() {
        if let Some((line_top, line_bottom)) = line_bounds(font, line) {
            let advance = index as f32 * font.line_step;
            top = top.min(advance + line_top);
            bottom = bottom.max(advance + line_bottom);
        }
    }
    if !top.is_finite() {
        return;
    }
    let desired_baseline = height * notification.baseline_percent as f32 / 100.0;
    let first_baseline = -top * mapping.scale;
    let last_baseline = height - bottom * mapping.scale;
    let baseline_pixels = if last_baseline >= first_baseline {
        desired_baseline.clamp(first_baseline, last_baseline)
    } else {
        // An oversized message begins at the top; the drawable clip bounds
        // its remaining lines without changing the producer's text or clock.
        first_baseline
    };
    let pen_font_pixels = (pen_pixels - mapping.offset_x as f32) / mapping.scale;
    let baseline_font_pixels = (baseline_pixels - mapping.offset_y as f32) / mapping.scale;
    for (index, line) in lines.iter().enumerate() {
        let baseline = baseline_font_pixels + index as f32 * font.line_step;
        if let Some((line_top, line_bottom)) = line_bounds(font, line) {
            let visible_top = mapping.offset_y as f32 + (baseline + line_top) * mapping.scale;
            let visible_bottom = mapping.offset_y as f32 + (baseline + line_bottom) * mapping.scale;
            if visible_bottom <= 0.0 || visible_top >= height {
                continue;
            }
        }
        // FUN_00452790 centres presets above 9 at (width - text width) / 2,
        // unlike the menu routines' `x - width / 2`.
        let pen = if notification.center_x {
            pen_font_pixels - font.measure(line) / 2.0
        } else {
            pen_font_pixels
        };
        draw_menu_text(
            renderer,
            font,
            line,
            [pen, baseline],
            false,
            usize::MAX,
            mapping,
        );
    }
}

fn line_bounds(font: &MenuFont, text: &str) -> Option<(f32, f32)> {
    text.chars()
        .filter_map(|character| font.glyph(character))
        .filter(|glyph| glyph.width != 0 && glyph.height != 0)
        .map(|glyph| (glyph.yoff - glyph.height as f32 + 1.0, glyph.yoff + 1.0))
        .reduce(|(top, bottom), (next_top, next_bottom)| {
            (top.min(next_top), bottom.max(next_bottom))
        })
}

/// FUN_00470D00 wraps only at spaces and moves the complete overflowing word
/// to the next baseline. The authored cargo strings use ordinary spaces, so
/// this word form retains the recovered byte scanner's decisions.
fn wrap_notification(font: &MenuFont, text: &str, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && font.measure(&candidate) > max_width {
            lines.push(std::mem::take(&mut line));
            line.push_str(word);
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::{system::PaletteEntry, terrain::TerrainGrid};
    use v2k_render::{Camera, ModelDraw};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Sprite {
        rect: (i32, i32, u32, u32),
        clip: Option<(i32, i32, u32, u32)>,
    }

    struct TextRecorder {
        viewport: [u32; 2],
        policy: UiSubmissionPolicy,
        sprites: Vec<Sprite>,
        clip: Option<(i32, i32, u32, u32)>,
    }

    impl TextRecorder {
        fn new(viewport: [u32; 2], policy: UiSubmissionPolicy) -> Self {
            Self {
                viewport,
                policy,
                sprites: Vec::new(),
                clip: None,
            }
        }

        fn submit(
            &mut self,
            fonts: &MenuFonts,
            lines: &[GameplayNotificationLine],
            context: GameplayTextContext,
        ) {
            self.sprites.clear();
            crate::draw_gameplay_notifications(self, fonts, lines, context);
            assert_eq!(self.clip, None, "world text restores unclipped output");
        }
    }

    impl Renderer for TextRecorder {
        fn backend_name(&self) -> &str {
            "text submission recorder"
        }
        fn clear(&mut self, _: f32, _: f32, _: f32) {}
        fn present(&mut self) {}
        fn resize(&mut self, width: u32, height: u32) {
            self.viewport = [width, height];
        }
        fn set_camera(&mut self, _: &Camera) {}
        fn set_fog(&mut self, _: bool, _: f32, _: f32, _: [f32; 3]) {}
        fn draw_terrain(
            &mut self,
            _: &TerrainGrid,
            _: &[PaletteEntry],
            _: Option<&v2k_render::terrain_tiles::TerrainFrames>,
            _: Option<&v2k_render::terrain_light::TerrainLightWindow>,
            _: u32,
        ) {
        }
        fn draw_model_body(&mut self, _: ModelDraw<'_>) {}
        fn set_sprite_clip(&mut self, clip: Option<(i32, i32, u32, u32)>) {
            self.clip = clip;
        }
        fn draw_sprite(&mut self, _: &[u8], width: u32, height: u32, x: i32, y: i32) {
            self.sprites.push(Sprite {
                rect: (x, y, width, height),
                clip: self.clip,
            });
        }
        fn draw_material_sprite(
            &mut self,
            rgba: &[u8],
            width: u32,
            height: u32,
            x: i32,
            y: i32,
            _: v2k_render::WorldSpriteBlend,
        ) {
            self.draw_sprite(rgba, width, height, x, y);
        }
        fn draw_fullscreen(&mut self, _: &[u8], _: u32, _: u32) {}
        fn draw_color_overlay(&mut self, _: f32, _: f32, _: f32, _: f32) {}
        fn viewport_size(&self) -> (u32, u32) {
            (self.viewport[0], self.viewport[1])
        }
        fn ui_submission_policy(&self) -> UiSubmissionPolicy {
            self.policy
        }
    }

    fn fonts(variant: u32) -> MenuFonts {
        let root = v2k_test_support::retail_dir();
        let session = v2k_game::session::GameSession::init(&root).expect("retail PRELOAD required");
        let level = session
            .load_ovl_by_id(2, variant)
            .expect("retail high font tier required");
        MenuFonts::from_level(&level).unwrap()
    }

    fn line(
        text: &str,
        x_percent: i32,
        baseline_percent: i32,
        width_percent: i32,
        center_x: bool,
    ) -> GameplayNotificationLine {
        GameplayNotificationLine {
            string_id: 0,
            text: text.to_owned(),
            x_percent,
            baseline_percent,
            width_percent,
            center_x,
        }
    }

    fn assert_rect(actual: (i32, i32, u32, u32), expected: (i32, i32, u32, u32)) {
        for (actual, expected) in [actual.0, actual.1, actual.2 as i32, actual.3 as i32]
            .into_iter()
            .zip([expected.0, expected.1, expected.2 as i32, expected.3 as i32])
        {
            assert!(
                (actual - expected).abs() <= 1,
                "{actual} differs from {expected}"
            );
        }
    }

    #[v2k_test_support::retail_test]
    fn native_world_text_reaches_drawable_percent_anchors_and_returns_to_each_original() {
        let messages = [
            line("A", 60, 7, 45, true),
            line("A", 25, 87, 50, false),
            line("A", 50, 95, 100, true),
        ];
        for (variant, authored) in [
            (
                // FUN_00452790 centres at (640 - 10) / 2 with the truncated
                // whole-pixel width.
                1,
                [(315, 20, 11, 14), (160, 404, 11, 14), (315, 443, 11, 14)],
            ),
            (
                2,
                [(395, 29, 11, 14), (200, 509, 11, 14), (395, 557, 11, 14)],
            ),
            (
                3,
                [(507, 40, 11, 14), (256, 655, 11, 14), (507, 716, 11, 14)],
            ),
        ] {
            let fonts = fonts(variant);
            let canvas = [fonts.virtual_w as u32, fonts.virtual_h as u32];
            let mut renderer = TextRecorder::new(canvas, UiSubmissionPolicy::NativeCanvas);
            renderer.submit(&fonts, &messages, GameplayTextContext::WorldOverlay);
            assert_eq!(
                renderer
                    .sprites
                    .iter()
                    .map(|sprite| sprite.rect)
                    .collect::<Vec<_>>(),
                authored
            );
            let original = renderer.sprites.clone();
            for (viewport, expected) in [
                (
                    [1920, 1080],
                    [(950, 52, 19, 25), (480, 916, 19, 25), (950, 1002, 19, 25)],
                ),
                (
                    [3840, 2160],
                    [
                        (1900, 104, 39, 50),
                        (960, 1832, 39, 50),
                        (1900, 2005, 39, 50),
                    ],
                ),
            ] {
                renderer.resize(viewport[0], viewport[1]);
                renderer.submit(&fonts, &messages, GameplayTextContext::WorldOverlay);
                assert_eq!(renderer.sprites.len(), 3);
                for (sprite, expected) in renderer.sprites.iter().zip(expected) {
                    assert_rect(sprite.rect, expected);
                    assert_eq!(sprite.clip, Some((0, 0, viewport[0], viewport[1])));
                }
            }
            renderer.resize(canvas[0], canvas[1]);
            renderer.submit(&fonts, &messages, GameplayTextContext::WorldOverlay);
            assert_eq!(
                renderer.sprites, original,
                "return restores the original authored display"
            );
        }
    }

    #[v2k_test_support::retail_test]
    fn native_world_multiline_block_and_oversized_tail_are_bounded_to_the_drawable() {
        let fonts = fonts(3);
        let mut renderer = TextRecorder::new([3840, 2160], UiSubmissionPolicy::NativeCanvas);
        renderer.submit(
            &fonts,
            &[line("A A A A A", 25, 87, 2, false)],
            GameplayTextContext::WorldOverlay,
        );
        assert_eq!(
            renderer.sprites.len(),
            5,
            "each whole word wraps at the scaled physical width"
        );
        assert!(
            renderer.sprites[0].rect.1 < 1832,
            "bottom overflow shifts the whole block up"
        );
        for sprite in &renderer.sprites {
            let (x, y, width, height) = sprite.rect;
            assert_eq!(x, 960);
            assert!(width <= 77 && y >= 0 && y + height as i32 <= 2160);
            assert_eq!(sprite.clip, Some((0, 0, 3840, 2160)));
        }
        let last = renderer.sprites.last().unwrap().rect;
        assert!((last.1 + last.3 as i32 - 2160).abs() <= 1);
        renderer.submit(
            &fonts,
            &[line("A 0 A 0 A", 60, 87, 2, true)],
            GameplayTextContext::WorldOverlay,
        );
        assert_eq!(
            renderer.sprites.len(),
            5,
            "centered messages wrap each short word within the physical width"
        );
        for sprite in &renderer.sprites {
            let (x, y, width, height) = sprite.rect;
            // Different glyph widths must each center on the drawable, rather
            // than sharing the first line's left edge or the authored center.
            assert!((x * 2 + width as i32 - 3840).abs() <= 2);
            assert!(width <= 77 && y >= 0 && y + height as i32 <= 2160);
            assert_eq!(sprite.clip, Some((0, 0, 3840, 2160)));
        }
        let oversized = std::iter::repeat_n("A", 40).collect::<Vec<_>>().join(" ");
        renderer.submit(
            &fonts,
            &[line(&oversized, 25, 87, 2, false)],
            GameplayTextContext::WorldOverlay,
        );
        assert!(
            renderer.sprites.len() < 40,
            "fully clipped tail lines are not submitted"
        );
        assert_eq!(renderer.sprites.first().unwrap().rect.1, 0);
        assert!(renderer
            .sprites
            .iter()
            .all(|sprite| sprite.clip == Some((0, 0, 3840, 2160))));
    }

    #[v2k_test_support::retail_test]
    fn overlay51_canvas_prompt_and_fitted_world_messages_keep_the_paired_canvas_policy() {
        let fonts = fonts(3);
        let prompt = [line("A", 50, 95, 100, true)];
        let mut renderer = TextRecorder::new([3840, 2160], UiSubmissionPolicy::NativeCanvas);
        renderer.submit(&fonts, &prompt, GameplayTextContext::AuthoredCanvas);
        assert_eq!(renderer.sprites.len(), 1);
        assert_rect(renderer.sprites[0].rect, (1904, 2015, 31, 39));
        assert_eq!(renderer.sprites[0].clip, None);
        renderer.policy = UiSubmissionPolicy::FitAuthoredCanvas;
        renderer.submit(&fonts, &prompt, GameplayTextContext::WorldOverlay);
        let fitted = renderer.sprites.clone();
        renderer.submit(&fonts, &prompt, GameplayTextContext::AuthoredCanvas);
        assert_eq!(
            renderer.sprites, fitted,
            "Low/Fit retains the original submission grammar"
        );
    }
}
