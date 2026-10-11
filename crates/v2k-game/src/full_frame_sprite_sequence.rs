//! Shared viewport-sized sprite sequence owned by session byte `+0x295`.
//!
//! Retail `FUN_00456750` starts (or restarts) the sequence by writing one.
//! `FUN_00453410` resolves the indexed global sprite, submits a viewport-sized
//! textured command, and advances after the command append succeeds.  Arena
//! exhaustion returns before the state write; the later setup helper itself is
//! unconditional, and the global-sprite pointer is loaded without a null
//! check.  The retail and demo functions are instruction-exact, and both
//! executables carry the same sentinel-terminated table.

use crate::model_color::{sprite_blend, sprite_flat_shade_row};
use crate::resource_cache::ResourceCache;
use v2k_render::WorldSpriteBlend;

/// Active entries at retail table `0x004D11DC`, excluding its unrelated index
/// zero value and trailing null sentinel.
pub const FULL_FRAME_SPRITE_IDS: [u16; 8] =
    [0x227, 0x228, 0x228, 0x228, 0x229, 0x22a, 0x22b, 0x22c];

/// One authenticated view of the current one-based retail table index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FullFrameSpriteFrame {
    pub table_index: u8,
    pub global_sprite_id: u16,
}

/// Decoded selected-tier material needed for the viewport-sized submission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedFullFrameSprite {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub render_flags: u8,
    pub blend: WorldSpriteBlend,
}

/// Exact state owner for the recovered subset of session byte `+0x295`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FullFrameSpriteSequence {
    table_index: u8,
}

impl FullFrameSpriteSequence {
    /// Mirror `FUN_00456750`: every accepted request starts at table index one,
    /// including a request made while an older sequence is still active.
    pub fn request(&mut self) {
        self.table_index = 1;
    }

    pub const fn is_active(self) -> bool {
        self.table_index != 0
    }

    pub fn current_frame(self) -> Option<FullFrameSpriteFrame> {
        let table_offset = self.table_index.checked_sub(1)?;
        let global_sprite_id = *FULL_FRAME_SPRITE_IDS.get(usize::from(table_offset))?;
        Some(FullFrameSpriteFrame {
            table_index: self.table_index,
            global_sprite_id,
        })
    }

    /// Advance only after the exact currently issued frame was submitted.
    /// A stale acknowledgement or failed host-side command preparation retains
    /// the current table index for the next presentation pass, matching the
    /// original arena-capacity failure branch.
    pub fn acknowledge_submitted(&mut self, frame: FullFrameSpriteFrame) -> bool {
        if self.current_frame() != Some(frame) {
            return false;
        }

        if usize::from(frame.table_index) == FULL_FRAME_SPRITE_IDS.len() {
            self.table_index = 0;
        } else {
            self.table_index = frame.table_index + 1;
        }
        true
    }
}

/// Resolve through the cumulative global pool so the selected system-overlay
/// tier supplies the pixels.  The sprite's authored flags select both the flat
/// palette row and the masked/additive framebuffer operation.
pub fn decode_full_frame_sprite(
    cache: &ResourceCache,
    frame: FullFrameSpriteFrame,
) -> Option<DecodedFullFrameSprite> {
    let (atlas, entry) = cache.global_sprite(frame.global_sprite_id)?;
    let render_flags = entry.pal_size as u8;
    let decoded = atlas
        .decode_sprite(entry, usize::from(sprite_flat_shade_row(render_flags)))
        .ok()?;
    Some(DecodedFullFrameSprite {
        rgba: decoded.rgba,
        width: u32::from(decoded.width),
        height: u32::from(decoded.height),
        render_flags,
        blend: sprite_blend(render_flags),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The faststart04 recording's Intro2 card frames (ticks 4003..4026):
    /// each entry fills the 640x480 surface with one RGB565 colour, ending
    /// on black. 0x227 is masked and copies its texels. The rest add over the
    /// card's black clear through the additive rows, which keep the top four
    /// bits of each field (`0xF79E`).
    #[v2k_test_support::retail_test]
    fn intro2_card_frames_match_the_recorded_fade() {
        let data_root = v2k_test_support::retail_dir();
        let mut session = crate::session::GameSession::init(&data_root).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(50, 1).unwrap();
        let recorded: [u16; 8] = [
            0xB596, 0xB596, 0xB596, 0xB596, 0x8410, 0x738E, 0x528A, 0x2104,
        ];
        let mut sequence = FullFrameSpriteSequence::default();
        sequence.request();
        for expected in recorded {
            let frame = sequence.current_frame().unwrap();
            let sprite = decode_full_frame_sprite(&session.cache, frame).unwrap();
            let colours = sprite
                .rgba
                .chunks_exact(4)
                .map(|px| {
                    (u16::from(px[0] >> 3) << 11)
                        | (u16::from(px[1] >> 2) << 5)
                        | u16::from(px[2] >> 3)
                })
                .collect::<Vec<_>>();
            let composite = |colour: u16| match sprite.blend {
                WorldSpriteBlend::Masked => colour,
                _ => colour & 0xF79E,
            };
            assert!(
                colours.iter().all(|&colour| composite(colour) == expected),
                "sprite {:#x}: {colours:04X?}, recorded {expected:04X}",
                frame.global_sprite_id,
            );
            assert!(sequence.acknowledge_submitted(frame));
        }
        assert!(!sequence.is_active());
    }

    #[test]
    fn request_walks_the_exact_eight_frame_table_then_clears() {
        let mut sequence = FullFrameSpriteSequence::default();
        assert!(!sequence.is_active());
        assert_eq!(sequence.current_frame(), None);

        sequence.request();
        let mut observed = Vec::new();
        while let Some(frame) = sequence.current_frame() {
            observed.push(frame.global_sprite_id);
            assert!(sequence.acknowledge_submitted(frame));
        }

        assert_eq!(observed, FULL_FRAME_SPRITE_IDS);
        assert!(!sequence.is_active());
    }

    #[test]
    fn failed_or_stale_submission_retains_the_current_frame() {
        let mut sequence = FullFrameSpriteSequence::default();
        sequence.request();
        let first = sequence.current_frame().unwrap();

        assert!(!sequence.acknowledge_submitted(FullFrameSpriteFrame {
            table_index: 2,
            global_sprite_id: 0x228,
        }));
        assert_eq!(sequence.current_frame(), Some(first));

        assert!(sequence.acknowledge_submitted(first));
        assert_eq!(
            sequence.current_frame(),
            Some(FullFrameSpriteFrame {
                table_index: 2,
                global_sprite_id: 0x228,
            })
        );
    }

    #[test]
    fn a_new_request_restarts_an_in_progress_sequence() {
        let mut sequence = FullFrameSpriteSequence::default();
        sequence.request();
        let first = sequence.current_frame().unwrap();
        assert!(sequence.acknowledge_submitted(first));
        assert_eq!(sequence.current_frame().unwrap().table_index, 2);

        sequence.request();
        assert_eq!(sequence.current_frame(), Some(first));
    }

    #[test]
    fn late_request_arms_next_frame_without_replacing_an_owned_command() {
        let mut sequence = FullFrameSpriteSequence::default();

        // The early consumer has already run, so this frame owns no command.
        let staged_before_simulation = sequence.current_frame();
        assert_eq!(staged_before_simulation, None);

        // A later gameplay producer can only arm the following frame.
        sequence.request();
        let staged_next_frame = sequence.current_frame().unwrap();
        assert_eq!(staged_next_frame.global_sprite_id, 0x227);
        assert!(sequence.acknowledge_submitted(staged_next_frame));

        // A still-later producer restarts persistent state, but cannot mutate
        // the already-owned command which this frame will eventually render.
        sequence.request();
        assert_eq!(staged_next_frame.global_sprite_id, 0x227);
        assert_eq!(
            sequence.current_frame(),
            Some(FullFrameSpriteFrame {
                table_index: 1,
                global_sprite_id: 0x227,
            })
        );
    }
}
