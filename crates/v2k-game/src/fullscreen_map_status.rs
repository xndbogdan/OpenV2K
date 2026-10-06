//! Authored text panel layered beside the modal fullscreen terrain map.
//!
//! `FUN_00453FE0` reads this presentation geometry from the selected
//! `?X3XX.OVL` rather than scaling one hard-coded layout.  Keep the recovered
//! Craft/Trophies subset separate from the terrain-raster implementation: the
//! later Building/new-ship lines depend on controller state that the port does
//! not yet own.

use crate::resource_cache::ResourceCache;
use crate::system_layout::SystemLayoutSource;

const SYSTEM_LEVEL: u32 = 3;
const PANEL_ORIGIN_LOCAL_INDEX: usize = 29;
const GROUP_ADVANCE_LOCAL_INDEX: usize = 11;
const VALUE_ADVANCE_LOCAL_INDEX: usize = 12;
const CRAFT_STRING_GLOBAL_ID: usize = 149;
const TROPHIES_STRING_GLOBAL_ID: usize = 150;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullscreenMapStatusLine {
    pub text: String,
    /// Left-aligned sprite-font baseline in the selected system tier's
    /// virtual framebuffer.
    pub baseline: [i32; 2],
}

/// Static resources used by the recovered portion of `FUN_00453FE0`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullscreenMapStatusResources {
    origin: [i32; 2],
    value_advance: i32,
    group_advance: i32,
    craft_label: String,
    trophies_label: String,
}

impl FullscreenMapStatusResources {
    /// Resolve only authored data. Missing strings or tier-local layout data
    /// disable this presentation layer instead of substituting guessed text or
    /// coordinates.
    pub fn from_cache(cache: &ResourceCache) -> Option<Self> {
        let mut result = Self {
            origin: [0; 2],
            value_advance: 0,
            group_advance: 0,
            craft_label: cache.global_string(CRAFT_STRING_GLOBAL_ID)?.to_owned(),
            trophies_label: cache.global_string(TROPHIES_STRING_GLOBAL_ID)?.to_owned(),
        };
        result.refresh_layout(cache)?;
        Some(result)
    }

    /// Refresh geometry from a prepared tier while retaining the intrinsic
    /// labels. Call on a cloned snapshot before publishing the cache stage.
    pub fn refresh_layout(&mut self, source: &impl SystemLayoutSource) -> Option<()> {
        let (x, y) = source.system_layout_point(SYSTEM_LEVEL, PANEL_ORIGIN_LOCAL_INDEX)?;
        let value_advance =
            i32::try_from(source.system_data_value(SYSTEM_LEVEL, VALUE_ADVANCE_LOCAL_INDEX)?)
                .ok()?;
        let group_advance =
            i32::try_from(source.system_data_value(SYSTEM_LEVEL, GROUP_ADVANCE_LOCAL_INDEX)?)
                .ok()?;
        self.origin = [i32::from(x), i32::from(y)];
        self.value_advance = value_advance;
        self.group_advance = group_advance;
        Some(())
    }

    /// Build the four currently supported lines. Retail formats the lives byte
    /// as an unsigned value after integer promotion and adds one, so `0xFF`
    /// is presented as 256 rather than wrapping to zero.
    pub fn lines(&self, extra_lives: u8, trophy_count: u8) -> [FullscreenMapStatusLine; 4] {
        let [x, y] = self.origin;
        let craft_value_y = y + self.value_advance;
        let trophies_label_y = craft_value_y + self.group_advance;
        let trophies_value_y = trophies_label_y + self.value_advance;

        [
            FullscreenMapStatusLine {
                text: self.craft_label.clone(),
                baseline: [x, y],
            },
            FullscreenMapStatusLine {
                text: (u16::from(extra_lives) + 1).to_string(),
                baseline: [x, craft_value_y],
            },
            FullscreenMapStatusLine {
                text: self.trophies_label.clone(),
                baseline: [x, trophies_label_y],
            },
            FullscreenMapStatusLine {
                text: trophy_count.to_string(),
                baseline: [x, trophies_value_y],
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::FullscreenMapStatusResources;

    #[test]
    fn panel_lines_follow_retail_baseline_sequence_and_integer_promotion() {
        let resources = FullscreenMapStatusResources {
            origin: [469, 20],
            value_advance: 20,
            group_advance: 30,
            craft_label: "Craft".to_owned(),
            trophies_label: "Trophies".to_owned(),
        };

        let lines = resources.lines(u8::MAX, 7);
        assert_eq!(
            lines.map(|line| (line.text, line.baseline)),
            [
                ("Craft".to_owned(), [469, 20]),
                ("256".to_owned(), [469, 40]),
                ("Trophies".to_owned(), [469, 70]),
                ("7".to_owned(), [469, 90]),
            ]
        );
    }
}
