//! Class65 Geyser: the Primary `FUN_0040C500 -> FUN_00407BC0` publishes when
//! a Type118 dies.
//!
//! The task has no timeout. Its private word is the sea delta: the body's
//! `+88` word, or its height above the sea when that word is zero, shifted
//! into raw units. Each `FUN_00407CA0` visit sprays class-13 particles when
//! the visit is detailed, lifts every live body hovering over the geyser,
//! then moves the sea through `FUN_004013A0` until the delta is spent.

use crate::behavior::SeaLevelTransition;
use v2k_formats::terrain::TerrainGrid;

/// `FUN_00407CA0`'s spray particle class.
pub const GEYSER_SPRAY_PARTICLE_CLASS: u8 = 13;
/// One spray particle per started 30 ms of callback time.
const SPRAY_SPAN_US: i32 = 30_000;
/// The spray's initial upward velocity word.
const SPRAY_VELOCITY_Y_RAW: i16 = 0x5DC;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeyserTaskState {
    elapsed_ms: u32,
    sea: SeaLevelTransition,
}

impl GeyserTaskState {
    pub fn new(sea_delta_raw: i32) -> Self {
        Self {
            elapsed_ms: 0,
            sea: SeaLevelTransition::new(sea_delta_raw),
        }
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.elapsed_ms
    }

    pub fn remaining_sea_delta_raw(self) -> i32 {
        self.sea.remaining()
    }

    /// `FUN_00401120`'s age prefix; a zero lifetime never times out.
    pub fn advance_prefix(&mut self, elapsed_micros: u32) {
        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1000);
    }

    /// The visit's last step: a nonzero delta moves the sea. A spent geyser
    /// keeps spraying and lifting; it never removes its body.
    pub fn step_sea(&mut self, terrain: &mut TerrainGrid, elapsed_micros: u32) {
        self.sea.tick(terrain, elapsed_micros as i32);
    }
}

/// `FUN_00407BC0`'s payload: a nonzero `+88` word, else the body's height
/// over the sea, both shifted into raw units.
pub fn geyser_sea_delta_raw(
    word_0x88: i32,
    position_y_raw: i16,
    terrain_header_sea_raw: i32,
) -> i32 {
    if word_0x88 != 0 {
        word_0x88.wrapping_shl(8)
    } else {
        crate::main_base_type54_abort::change_sea_level_delta_raw(
            position_y_raw,
            0,
            terrain_header_sea_raw,
        )
    }
}

/// One spray particle: its birth position and velocity words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeyserSprayParticle {
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
}

/// A detailed visit's spray: one particle per started 30 ms (at least one).
/// Each draws X, Y and Z velocity steps and a Y position step, in that
/// order; every step accumulates over the visit.
pub fn geyser_spray(
    position_raw: [i16; 3],
    elapsed_micros: u32,
    next_random: &mut impl FnMut() -> u16,
) -> Vec<GeyserSprayParticle> {
    let mut particles = Vec::new();
    let mut position = position_raw;
    let mut velocity = [0, SPRAY_VELOCITY_Y_RAW, 0];
    let mut remaining = elapsed_micros as i32;
    loop {
        velocity[0] = velocity[0].wrapping_add(((next_random() >> 8) & 0xff) as i16 - 0x80);
        velocity[1] = velocity[1].wrapping_add(((next_random() >> 9) & 0x7f) as i16 - 0x40);
        velocity[2] = velocity[2].wrapping_add(((next_random() >> 8) & 0xff) as i16 - 0x80);
        position[1] = position[1].wrapping_add(((next_random() >> 9) & 0x7f) as i16 - 0x40);
        particles.push(GeyserSprayParticle {
            position_raw: position,
            velocity_raw: velocity,
        });
        remaining = remaining.wrapping_sub(SPRAY_SPAN_US);
        if remaining <= 0 {
            return particles;
        }
    }
}

/// `FUN_00407DF0`'s lift of one other body: bodies above the geyser within
/// a cone (`dy/2 + 50` around its axis, under 1200 units up) gain
/// `(2000 - dy) * 4` scaled by the callback time on their Y velocity word.
pub fn geyser_lift_raw(
    geyser_position_raw: [i16; 3],
    body_position_raw: [i16; 3],
    elapsed_micros: u32,
) -> Option<i16> {
    let delta =
        |axis: usize| i32::from(body_position_raw[axis]) - i32::from(geyser_position_raw[axis]);
    let dy = delta(1);
    if dy <= 0 {
        return None;
    }
    let (dx, dz) = (delta(0), delta(2));
    let reach = dy / 2 + 0x32;
    if dx.abs() > reach
        || dz.abs() > reach
        || dz.wrapping_mul(dz).wrapping_add(dx.wrapping_mul(dx)) >= reach * reach
        || dy >= 0x4B0
    {
        return None;
    }
    let lift = ((i64::from((2000 - dy) * 4) * i64::from((elapsed_micros as i32).wrapping_shl(13)))
        >> 31) as i32;
    // A negative product leaves the word unchanged.
    (lift >= 0).then_some(lift as i16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_takes_the_word_or_the_height_over_the_sea() {
        assert_eq!(geyser_sea_delta_raw(1000, -3296, -3614 << 8), 1000 << 8);
        // A zero word: the body's height over the sea, no model extent.
        assert_eq!(geyser_sea_delta_raw(0, -3296, -3614 << 8), 318 << 8);
    }

    #[test]
    fn a_20ms_visit_sprays_once_and_each_started_30ms_once_more() {
        let mut words = [0x8000u16, 0x8000, 0x8000, 0x8000].into_iter().cycle();
        let one = geyser_spray([10, 20, 30], 20_000, &mut || words.next().unwrap());
        // 0x8000 is exactly the midpoint of each draw.
        assert_eq!(
            one,
            [GeyserSprayParticle {
                position_raw: [10, 20, 30],
                velocity_raw: [0, 0x5DC, 0],
            }]
        );
        let mut words = [0xFFFFu16; 8].into_iter();
        let two = geyser_spray([10, 20, 30], 30_001, &mut || words.next().unwrap());
        assert_eq!(two.len(), 2);
        assert_eq!(two[1].velocity_raw, [0x7F * 2, 0x5DC + 0x3F * 2, 0x7F * 2]);
        assert_eq!(two[1].position_raw, [10, 20 + 0x3F * 2, 30]);
        // A zero callback time still sprays once (do-while).
        let mut words = std::iter::repeat(0u16);
        assert_eq!(
            geyser_spray([0; 3], 0, &mut || words.next().unwrap()).len(),
            1
        );
    }

    #[test]
    fn the_lift_reaches_a_cone_over_the_geyser() {
        let geyser = [0, 100, 0];
        // 300 up: reach 200, lift (2000-300)*4*(20000<<13) >> 31 = 518.
        assert_eq!(geyser_lift_raw(geyser, [100, 400, 100], 20_000), Some(518));
        // Outside the circle, though inside the square.
        assert_eq!(geyser_lift_raw(geyser, [150, 400, 150], 20_000), None);
        // At or below the geyser, and at 1200 up, nothing.
        assert_eq!(geyser_lift_raw(geyser, [0, 100, 0], 20_000), None);
        assert_eq!(geyser_lift_raw(geyser, [0, 1300, 0], 20_000), None);
    }
}
