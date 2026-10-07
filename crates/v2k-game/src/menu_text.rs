//! The original menu sprite fonts (OVL Section 4 of system level 2) and the
//! layout-point pool (Section 1) — the ground-truth text system decoded in
//! the 2026-07-05 menu RE pass (MENU_SYSTEM.md §Text rendering).
//!
//! Section 4 holds TWO font records: font 0 = GREEN glyphs (normal rows),
//! font 1 = YELLOW glyphs (selected row; same metrics, sprite ids +173).
//! Selection highlight in the original is purely this font swap. Each
//! record carries 256 glyph metrics {advance, kern, xoff, yoff}: advance and
//! kern use the record's 1/100-pixel pen units, while xoff/yoff are signed
//! whole pixels. It also carries 256 glyph sprite ids into the global
//! Section-3 pool. Glyphs are blitted at the BASELINE:
//! y_blit = baseline + yoff − sprite_h + 1.
//!
//! (The Section-4 parser lives in `v2k_formats::params` under legacy field
//! names from the pre-correction "parameter table" reading: `params[c]` =
//! metrics tuple, `indices[c]` = glyph sprite id, `param_c/param_d` = line
//! advance/gap.)

use crate::level::LevelState;
use crate::resource_cache::ResourceCache;
use crate::system_layout::SystemLayoutSource;

/// Special glyphs used by spinner bars (FUN_0043B690).
pub const GLYPH_BAR_START_CAP: char = '\u{1B}';
pub const GLYPH_BAR_EMPTY: char = '\u{1C}';
pub const GLYPH_BAR_END_CAP: char = '\u{1D}';
pub const GLYPH_BAR_FILLED: char = '\u{1E}';

/// One decoded glyph: RGBA pixels + metrics in (asset-space) pixels.
pub struct Glyph {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Pen advance metric `a`, converted from fixed-point pen units to px.
    pub advance: f32,
    /// Inter-glyph kerning metric `b`, converted from fixed-point pen units.
    pub kern: f32,
    /// The same two metrics in the record's pen units ([`MenuFont::pen_scale`]
    /// per pixel), which retail accumulates before truncating to pixels.
    pub advance_raw: i32,
    pub kern_raw: i32,
    /// Signed whole-pixel horizontal blit offset from the pen.
    pub xoff: f32,
    /// Signed whole-pixel vertical blit offset from the baseline.
    pub yoff: f32,
}

/// One of the two menu fonts (green normal / yellow selected).
pub struct MenuFont {
    /// 256 glyph slots (None = not present; fall back to 0x7F).
    glyphs: Vec<Option<Glyph>>,
    /// Advance-only metrics for missing sprites.
    fallback: usize,
    /// Word-wrap line step ((line_advance + line_gap)/100), px.
    pub line_step: f32,
    /// Record `+0x10`: pen units per pixel (100 in every tier).
    pub pen_scale: i32,
}

impl MenuFont {
    pub fn glyph(&self, c: char) -> Option<&Glyph> {
        let idx = c as usize;
        let g = if idx < self.glyphs.len() {
            self.glyphs[idx].as_ref()
        } else {
            None
        };
        g.or_else(|| self.glyphs.get(self.fallback).and_then(|f| f.as_ref()))
    }

    /// Width of `text` in asset-space pixels.
    pub fn measure(&self, text: &str) -> f32 {
        self.measure_prefix(text, usize::MAX)
    }

    /// Width of the first `max_chars` of `text` in asset-space pixels.
    ///
    /// The value column typewriter draws this prefix right-aligned at
    /// `label_x + pt7.x`. Measuring the full string and then clipping
    /// from that left x grows the option rightward.
    ///
    /// `FUN_00470F80`: every advance plus every kern but the last, in pen
    /// units, truncated to whole pixels.
    pub fn measure_prefix(&self, text: &str, max_chars: usize) -> f32 {
        let mut width = 0;
        let mut remaining = max_chars;
        let mut chars = text.chars().peekable();
        while remaining > 0 {
            let Some(c) = chars.next() else {
                break;
            };
            remaining -= 1;
            if let Some(g) = self.glyph(c) {
                width += g.advance_raw;
                if remaining > 0 && chars.peek().is_some() {
                    width += g.kern_raw;
                }
            }
        }
        (width / self.pen_scale) as f32
    }

    /// Largest distance from a glyph baseline to its top edge. This lets the
    /// scrolling menu clip at the first row's actual visible top instead of
    /// exposing a whole overscan row over the 3D panel frame.
    pub fn max_ascent(&self) -> f32 {
        self.glyphs
            .iter()
            .flatten()
            .filter(|glyph| glyph.height != 0)
            .map(|glyph| glyph.height as f32 - glyph.yoff - 1.0)
            .fold(0.0, f32::max)
    }

    /// Largest distance from the baseline to a glyph quad's lower edge.
    /// Together with [`Self::max_ascent`], this defines a symmetric list
    /// scissor without exposing either overscan row through the panel frame.
    pub fn max_descent(&self) -> f32 {
        self.glyphs
            .iter()
            .flatten()
            .filter(|glyph| glyph.height != 0)
            .map(|glyph| glyph.yoff + 1.0)
            .fold(0.0, f32::max)
    }
}

/// Both menu fonts + the asset-space geometry they were authored for.
pub struct MenuFonts {
    pub normal: MenuFont,
    pub selected: MenuFont,
    /// The asset variant's virtual resolution (320×240 for the PRELOAD
    /// variant-0 system OVL; 640×480, 800×600, or 1024×768 for variants 1-3).
    pub virtual_w: f32,
    pub virtual_h: f32,
    layout_points: Vec<(f32, f32)>,
}

/// Staged display geometry. High tiers share Section-3/4 glyph allocations;
/// replacing their layout must not decode or mutate those intrinsic assets.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedMenuFontLayout {
    virtual_size: (u32, u32),
    layout_points: Vec<(f32, f32)>,
}

impl PreparedMenuFontLayout {
    pub fn from_source(source: &impl SystemLayoutSource) -> Option<Self> {
        let virtual_size = (
            source.system_data_value(2, 2)?,
            source.system_data_value(2, 3)?,
        );
        if virtual_size.0 == 0 || virtual_size.1 == 0 {
            return None;
        }
        let layout_points = (0..13)
            .map(|index| {
                source
                    .system_layout_point(2, index)
                    .map(|(x, y)| (f32::from(x), f32::from(y)))
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            virtual_size,
            layout_points,
        })
    }

    pub const fn virtual_size(&self) -> (u32, u32) {
        self.virtual_size
    }
}

impl MenuFonts {
    /// Publish only geometry from a validated equivalent high-tier stage.
    pub fn refresh_layout(&mut self, prepared: PreparedMenuFontLayout) {
        self.virtual_w = prepared.virtual_size.0 as f32;
        self.virtual_h = prepared.virtual_size.1 as f32;
        self.layout_points = prepared.layout_points;
    }

    /// Decode both fonts from the menu OVL's Section 4 + sprite pool.
    ///
    /// Glyph sprite ids are GLOBAL Section-3 pool ids (resolved through
    /// `ResourceCache::global_sprite`, which matches on each entry's stored
    /// global slot).
    pub fn from_cache(cache: &ResourceCache) -> Option<Self> {
        let menu_ovl = cache.menu_ovl()?;
        Self::from_level(menu_ovl)
    }

    /// Decode the font and layout variant directly from a level-2 OVL.
    /// This is required for high-detail menus: PRELOAD contains variant 0,
    /// while the original loads the selected display-mode variant. Variants
    /// 1-3 share glyph pixels but retain different layout coordinates.
    pub fn from_level(menu_ovl: &LevelState) -> Option<Self> {
        let table = menu_ovl.params.as_ref()?;
        let atlas = menu_ovl.sprites.as_ref()?;
        if table.records.len() < 2 {
            return None;
        }
        // The glyph scale cannot distinguish the three high-resolution modes.
        // FUN_0042B870 reads viewport width/height from system-2 scalars 2/3;
        // retain that same selected display block alongside its Section-1 points.
        let display = menu_ovl.fixup_data.as_ref()?.entries.get(2..4)?;
        let (vw, vh) = (display[0] as f32, display[1] as f32);
        if vw <= 0.0 || vh <= 0.0 {
            return None;
        }

        let build = |rec: &v2k_formats::params::ParamRecord| -> MenuFont {
            let n = rec.params.len().min(rec.indices.len()).min(256);
            let mut glyphs: Vec<Option<Glyph>> = Vec::with_capacity(n);
            let pen_scale = if rec.param_b == 0 {
                100
            } else {
                rec.param_b as i32
            };
            for c in 0..n {
                let t = &rec.params[c];
                let advance_raw = i32::from(t.a as i16);
                let kern_raw = i32::from(t.b as i16);
                let advance = advance_raw as f32 / pen_scale as f32;
                let kern = kern_raw as f32 / pen_scale as f32;
                // FUN_00470B60 divides the running pen by the record's
                // +0x10 scale before adding metrics +4/+6 directly. These
                // offsets are integer pixels, not more fixed-point pen data.
                // Descenders such as p/y use yoff=4 in the high tier.
                let xoff = t.c as i16 as f32;
                let yoff = t.d as i16 as f32;
                let sprite_id = rec.indices[c] as u16;
                let decoded = atlas
                    .entries
                    .iter()
                    .find(|entry| entry.index == sprite_id)
                    .and_then(|entry| {
                        // Glyphs are blitted by `FUN_0047AD90`, which reads
                        // palette row 28 for their flag-0x04 records.
                        let row = crate::model_color::sprite_flat_shade_row(entry.pal_size as u8);
                        atlas.decode_sprite(entry, usize::from(row)).ok()
                    });
                glyphs.push(match decoded {
                    Some(d) if d.width > 0 && d.height > 0 => Some(Glyph {
                        width: d.width as u32,
                        height: d.height as u32,
                        rgba: d.rgba,
                        advance,
                        kern,
                        advance_raw,
                        kern_raw,
                        xoff,
                        yoff,
                    }),
                    // Keep advance-only glyphs (e.g. space has no sprite).
                    _ => Some(Glyph {
                        rgba: Vec::new(),
                        width: 0,
                        height: 0,
                        advance,
                        kern,
                        advance_raw,
                        kern_raw,
                        xoff,
                        yoff,
                    }),
                });
            }
            MenuFont {
                glyphs,
                fallback: (rec.unk_2c as usize).min(n.saturating_sub(1)),
                line_step: (rec.param_c + rec.param_d) as f32 / 100.0,
                pen_scale,
            }
        };

        let layout_points = menu_ovl
            .fixup_code
            .as_ref()
            .map(|table| {
                table
                    .entries
                    .iter()
                    .map(|&raw| {
                        let x = (raw & 0xFFFF) as u16 as i16 as f32;
                        let y = (raw >> 16) as u16 as i16 as f32;
                        (x, y)
                    })
                    .collect()
            })
            .unwrap_or_default();

        Some(Self {
            normal: build(&table.records[0]),
            selected: build(&table.records[1]),
            virtual_w: vw,
            virtual_h: vh,
            layout_points,
        })
    }

    pub fn font(&self, selected: bool) -> &MenuFont {
        if selected {
            &self.selected
        } else {
            &self.normal
        }
    }

    pub fn layout_point(&self, idx: usize) -> Option<(f32, f32)> {
        self.layout_points.get(idx).copied()
    }
}

/// Layout point `idx` from the menu OVL's Section-1 pool (packed s16 x,y),
/// in asset-space pixels. Known points (variant 0): pt0=(160,155) ring
/// anchor, pt3=(67,105) settings-list start, pt4=(0,14) row step,
/// pt5=(160,106) backdrop center, pt6=(160,170) settings prop center,
/// pt7=(185,0) value column, pt12=(160,220) prop-label anchor.
pub fn layout_point(cache: &ResourceCache, idx: usize) -> Option<(f32, f32)> {
    let menu_ovl = cache.menu_ovl()?;
    let table = menu_ovl.fixup_code.as_ref()?;
    let raw = *table.entries.get(idx)?;
    let x = (raw & 0xFFFF) as u16 as i16 as f32;
    let y = (raw >> 16) as u16 as i16 as f32;
    Some((x, y))
}

/// Build the spinner-bar glyph string exactly like `FUN_0043B690`.
///
/// `total_chars` is the high word of the packed row id. The base string is
/// exactly that many glyphs: start cap, empty run, end cap. Then the first
/// `filled` glyphs are overwritten with the filled block.
pub fn bar_string(filled: u32, total_chars: u32) -> String {
    let total = total_chars as usize;
    if total == 0 {
        return String::new();
    }
    let mut chars = vec![GLYPH_BAR_EMPTY; total];
    chars[0] = GLYPH_BAR_START_CAP;
    if let Some(last) = chars.last_mut() {
        *last = GLYPH_BAR_END_CAP;
    }
    for ch in chars.iter_mut().take(filled as usize) {
        *ch = GLYPH_BAR_FILLED;
    }
    chars.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_font(glyphs: Vec<Option<Glyph>>) -> MenuFont {
        MenuFont {
            glyphs,
            fallback: 0,
            line_step: 0.0,
            pen_scale: 100,
        }
    }

    fn glyph(advance: f32, kern: f32) -> Glyph {
        Glyph {
            rgba: Vec::new(),
            width: 0,
            height: 0,
            advance,
            kern,
            advance_raw: (advance * 100.0) as i32,
            kern_raw: (kern * 100.0) as i32,
            xoff: 0.0,
            yoff: 0.0,
        }
    }

    #[test]
    fn measure_matches_original_interglyph_kerning() {
        let mut glyphs = Vec::new();
        glyphs.resize_with(256, || None);
        glyphs[b'A' as usize] = Some(glyph(5.0, 1.0));
        glyphs[b'B' as usize] = Some(glyph(7.0, 3.0));
        let font = test_font(glyphs);

        assert_eq!(font.measure("A"), 5.0);
        assert_eq!(font.measure("AB"), 13.0);
        assert_eq!(font.measure("ABA"), 21.0);
        assert_eq!(font.measure_prefix("ABA", 0), 0.0);
        assert_eq!(font.measure_prefix("ABA", 1), 5.0);
        assert_eq!(font.measure_prefix("ABA", 2), 13.0);
        assert_eq!(font.measure_prefix("ABA", 3), font.measure("ABA"));
        assert_eq!(font.measure_prefix("ABA", 8), font.measure("ABA"));

        let column_right = 252.0;
        let full_x = column_right - font.measure("ABA");
        let prefix_x = column_right - font.measure_prefix("ABA", 1);
        assert!(
            prefix_x > full_x,
            "a shorter prefix must sit further right so the value grows left"
        );
        assert_eq!(
            column_right - font.measure_prefix("ABA", 3),
            full_x,
            "the completed value keeps the same right edge"
        );
    }

    #[test]
    fn measure_truncates_pen_units_like_retail() {
        let mut glyphs = Vec::new();
        glyphs.resize_with(256, || None);
        let mut a = glyph(0.0, 0.0);
        a.advance_raw = 1_237;
        a.kern_raw = 150;
        glyphs[b'A' as usize] = Some(a);
        let font = test_font(glyphs);
        // 1237 + 150 + 1237 pen units: 26.24 px truncates to 26.
        assert_eq!(font.measure("AA"), 26.0);
        assert_eq!(font.measure("A"), 12.0);
    }

    #[test]
    fn max_ascent_uses_baseline_metrics() {
        let mut glyphs = Vec::new();
        glyphs.resize_with(256, || None);
        let mut a = glyph(5.0, 0.0);
        a.height = 10;
        a.yoff = 2.0;
        glyphs[b'A' as usize] = Some(a);
        assert_eq!(test_font(glyphs).max_ascent(), 7.0);
    }

    #[test]
    fn max_descent_uses_baseline_metrics() {
        let mut glyphs = Vec::new();
        glyphs.resize_with(256, || None);
        let mut a = glyph(5.0, 0.0);
        a.height = 10;
        a.yoff = 2.0;
        glyphs[b'A' as usize] = Some(a);
        assert_eq!(test_font(glyphs).max_descent(), 3.0);
    }

    #[test]
    fn bar_string_shapes() {
        assert_eq!(
            bar_string(0, 4).chars().collect::<Vec<_>>(),
            vec![
                GLYPH_BAR_START_CAP,
                GLYPH_BAR_EMPTY,
                GLYPH_BAR_EMPTY,
                GLYPH_BAR_END_CAP
            ]
        );
        assert_eq!(
            bar_string(3, 4).chars().collect::<Vec<_>>(),
            vec![
                GLYPH_BAR_FILLED,
                GLYPH_BAR_FILLED,
                GLYPH_BAR_FILLED,
                GLYPH_BAR_END_CAP
            ]
        );
        assert_eq!(
            bar_string(4, 4).chars().collect::<Vec<_>>(),
            vec![
                GLYPH_BAR_FILLED,
                GLYPH_BAR_FILLED,
                GLYPH_BAR_FILLED,
                GLYPH_BAR_FILLED
            ]
        );
    }
}
