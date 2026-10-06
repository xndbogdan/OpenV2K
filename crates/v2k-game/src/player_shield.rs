//! Entity shield presentation from `FUN_004138F0 -> FUN_004136C0`.
//!
//! The hull owns the persistent damage buffer (`entity+50`). This transient
//! owner retains only the draw-time smoothing word (`+54`) and the admitted
//! player-callback elapsed clock used by the rotating `shipaura` model.

use v2k_formats::models::AnimVars;
use v2k_render::{mat3_mul, orientation_from_ypr};

pub const PLAYER_SHIELD_MODEL_ID: usize = 245;
pub const PLAYER_SHIELD_FIRST_SPRITE_ID: u16 = 620;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerShieldPresentation {
    display_buffer_raw: i32,
    player_callback_elapsed_micros_raw: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerShieldFrame {
    pub spin_raw: u16,
    pub retail_tick: u32,
    pub damage_flash_level: u8,
}

impl PlayerShieldPresentation {
    /// The current player adapter runs one capped callback per active frame.
    /// Keep its elapsed clock separate from the process tick and render count.
    pub fn advance_player_callback(&mut self, elapsed_micros: u32) {
        self.player_callback_elapsed_micros_raw = self
            .player_callback_elapsed_micros_raw
            .wrapping_add(elapsed_micros.min(crate::hover::RETAIL_FRAME_DELTA_MAX_US));
    }

    /// One ordinary visible-body draw. Retail moves +54 toward the live
    /// buffer only while drawing, then consumes one RNG word below 40000.
    /// Neither elapsed time nor this visual decay spends the actual shield.
    pub fn prepare_frame(
        &mut self,
        buffer_raw: i32,
        retail_tick: u32,
        random_u16: &mut impl FnMut() -> u16,
    ) -> Option<PlayerShieldFrame> {
        if self.display_buffer_raw == 0 && buffer_raw == 0 {
            return None;
        }
        let difference = self.display_buffer_raw.wrapping_sub(buffer_raw);
        self.display_buffer_raw = if difference > 4 {
            self.display_buffer_raw.wrapping_sub(difference / 4).max(0)
        } else {
            buffer_raw
        };
        if self.display_buffer_raw < 40_000 {
            let divisor = (self.display_buffer_raw >> 13).max(1) as u16;
            if random_u16() % divisor == 0 {
                return None;
            }
        }
        Some(PlayerShieldFrame {
            spin_raw: (self.player_callback_elapsed_micros_raw >> 8) as u16,
            retail_tick,
            // Callback413670: selector0 returns process tick; every nonzero
            // selector clamps signed param3/1000 to 0..7.
            damage_flash_level: (difference / 1_000).clamp(0, 7) as u8,
        })
    }
}

impl PlayerShieldFrame {
    pub fn animation_vars(self) -> AnimVars {
        let mut vars = AnimVars::default();
        vars.dynamic[0] = (self.retail_tick & 0xffff) as i32;
        vars.dynamic[1..].fill(i32::from(self.damage_flash_level));
        vars
    }

    /// 457C30 mixes lateral/forward, then457DD0 mixes lateral/up. In the
    /// renderer's column basis these are local Ry(-angle) then Rz(angle).
    pub fn orientation(self, body: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
        let angle = f32::from(self.spin_raw) * std::f32::consts::TAU / 65_536.0;
        mat3_mul(body, orientation_from_ypr(-angle, 0.0, angle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_shield_has_no_time_decay_and_damage_drives_the_authored_flash() {
        let mut presentation = PlayerShieldPresentation::default();
        let mut rng = || panic!("full shield does not consume RNG");
        let grant = presentation.prepare_frame(100_000, 1, &mut rng).unwrap();
        assert_eq!(grant.damage_flash_level, 0);
        for tick in 2..200 {
            presentation.advance_player_callback(100_000);
            let steady = presentation.prepare_frame(100_000, tick, &mut rng).unwrap();
            assert_eq!(steady.damage_flash_level, 0);
            assert_eq!(presentation.display_buffer_raw, 100_000);
        }
        let hit = presentation.prepare_frame(80_000, 200, &mut rng).unwrap();
        assert_eq!(hit.damage_flash_level, 7);
        assert_eq!(presentation.display_buffer_raw, 95_000);
        assert_eq!(hit.animation_vars().dynamic[1], 7);
        assert_eq!(hit.animation_vars().dynamic[0], 200);
    }

    #[test]
    fn depleted_shield_flickers_using_one_shared_word_per_draw_then_disappears() {
        let mut presentation = PlayerShieldPresentation::default();
        let mut count = 0;
        assert!(presentation
            .prepare_frame(32_000, 1, &mut || {
                count += 1;
                0
            })
            .is_none());
        assert_eq!(count, 1);
        assert!(presentation.prepare_frame(32_000, 2, &mut || 1).is_some());
        for tick in 3..200 {
            presentation.prepare_frame(0, tick, &mut || 1);
        }
        assert_eq!(presentation.display_buffer_raw, 0);
        assert!(presentation
            .prepare_frame(0, 201, &mut || panic!("zero shield needs no RNG"))
            .is_none());
    }

    #[test]
    fn shield_clock_uses_wrapping_admitted_microseconds_and_both_local_axes() {
        let mut presentation = PlayerShieldPresentation::default();
        presentation.advance_player_callback(125_000);
        let frame = presentation
            .prepare_frame(100_000, 0x12345, &mut || unreachable!())
            .unwrap();
        assert_eq!(frame.spin_raw, (125_000 >> 8) as u16);
        assert_eq!(frame.animation_vars().dynamic[0], 0x2345);
        let frame = PlayerShieldFrame {
            spin_raw: 0x4000,
            ..frame
        };
        let basis = frame.orientation(orientation_from_ypr(0.0, 0.0, 0.0));
        let expected = [[0.0, 0.0, -1.0], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0]];
        for row in 0..3 {
            for column in 0..3 {
                assert!((basis[row][column] - expected[row][column]).abs() < 1e-6);
            }
        }
    }
}
