//! The Possession Ray's takeover, its release and the MindBall.
//!
//! The lock ([`crate::possession_ray`]) ends in `FUN_004480E0`: the target
//! takes the player's controller and runs Player Control, the craft runs
//! Passive Player, and controller `+70` holds 60000 ms. Player Control on the
//! possessed body counts it down (`FUN_00446640`); at zero `FUN_00446D70`
//! gives the craft its controller back, the target its own behavior, and
//! leaves a Type98 MindBall (`FUN_00409950`) that flies from the target to the
//! craft (`FUN_00409460`) while the session follows it, then hands the view
//! back (`FUN_00409420`). See `docs/re/PLAYER_CRAFT.md#mind-control`.

use v2k_formats::fixed_math::retail_sine_q15;

use crate::hover::q31_mul;

/// `FUN_004480E0` holds controller `+70` at this many milliseconds.
pub const POSSESSION_MS: i32 = 60_000;
/// The MindBall's type, born by `FUN_00409950`.
pub const MIND_BALL_TYPE: u32 = 98;
/// `FUN_004097A0` draws global model 243 (`DAT_004FE640 + 0x3CC`).
pub const MIND_BALL_MODEL_ID: usize = 243;
/// Each of the draw's two rings has this many instances.
const MIND_BALL_RING_INSTANCES: usize = 4;
/// `FUN_00409460`'s phases 0, 1 and 3 last this long; phase 2 adds it to the
/// distance's time.
const MIND_BALL_PHASE_US: i32 = 500_000;
const MIND_BALL_RETURN_BASE_US: i32 = 250_000;

/// The controller words a takeover changes: `+68` the controlled body, `+6C`
/// the craft it left, `+70` the time left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Possession {
    pub body: u32,
    pub craft: u32,
    pub countdown_ms: i32,
}

impl Possession {
    /// `FUN_004480E0`'s successful install.
    pub const fn take_over(craft: u32, body: u32) -> Self {
        Self {
            body,
            craft,
            countdown_ms: POSSESSION_MS,
        }
    }

    /// `FUN_00446640`'s `+70` test on the possessed body's visit: a dying or
    /// dead craft zeroes it, then the frame's milliseconds come off; below
    /// one, `FUN_00446D70` releases. True asks for the release.
    pub fn advance(&mut self, craft_live: bool, elapsed_us: u32) -> bool {
        if self.countdown_ms == 0 {
            return false;
        }
        if !craft_live {
            self.countdown_ms = 0;
        }
        self.countdown_ms = self.countdown_ms.wrapping_sub((elapsed_us / 1000) as i32);
        self.countdown_ms < 1
    }

    /// `FUN_004442B0`, TAB while the craft waits at `+6C`: `+70` becomes 1,
    /// so the next Player Control visit releases the body.
    pub fn end_on_next_visit(&mut self) {
        self.countdown_ms = 1;
    }

    /// `FUN_00448760`: a Hive swallowing the possessed body releases it on
    /// the next Player Control visit.
    pub fn swallowed(&mut self) {
        if self.countdown_ms != 0 {
            self.countdown_ms = 1;
        }
    }
}

/// The possessed body's owner in the port while Player Control runs on it:
/// the controller words and whether the body's own owner was the native one
/// (any world) or the fresh Level 1 one, and its context before the takeover
/// (Player Control's replaces it). The task scheduler holds it, so a
/// shot, blast or pair reaching the body finds its owner there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PossessionCustody {
    pub state: Possession,
    pub native_owner: bool,
    /// The body's own behavior context before the takeover, which the
    /// release's reinstall starts from.
    pub previous_context: crate::entity_behavior::BehaviorContextRuntime,
}

/// Passive Player (`FUN_00446FC0`) smokes a craft below this health.
const PASSIVE_SMOKE_BELOW_HEALTH: i32 = 20_000;

/// `FUN_00446FC0`'s smoke chance for a craft with `health` at its entry:
/// below 20000 one shared word decides, a 1-in-`health >> 13` chance (at
/// least 2); a hit then places the plume as Player Control's
/// (`FUN_00441670`).
pub fn passive_player_smokes(health: i32, mut next_word: impl FnMut() -> u16) -> bool {
    if health >= PASSIVE_SMOKE_BELOW_HEALTH {
        return false;
    }
    let divisor = if ((health as u32) & 0xFFFF_E000) as i32 >= 0x4000 {
        health >> 13
    } else {
        2
    };
    i64::from(next_word()) % i64::from(divisor) == 0
}

/// What one MindBall visit asks of its caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MindBallStep {
    Fly,
    /// `FUN_00410C10` on the ball: its flight is over, or the craft is gone.
    Die,
}

/// The MindBall's record, body `+88` (`FUN_004093B0`'s 0x24 bytes copied
/// from `FUN_00409950`'s birth words).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MindBallFlight {
    /// `+00`: the released body the flight starts at.
    pub target: u32,
    /// `+04`: the craft it returns to; the session follows it at the end.
    pub craft: u32,
    /// `+08..+0D`: the rise's start, then the offset from the craft.
    pub offset_raw: [i16; 3],
    /// `+14`: -1 at birth, then 0..=3.
    pub phase: i32,
    /// `+18`: microseconds into the phase.
    pub elapsed_us: i32,
    /// `+1C`: the phase's length.
    pub duration_us: i32,
    /// `+20`: the draw's fade word.
    pub fade: i32,
}

impl MindBallFlight {
    /// `FUN_00409950`: the birth leaves the phase at -1; the record's other
    /// words are the birth frame's unwritten stack, which the first visit's
    /// phase change overwrites before any read.
    pub const fn born(target: u32, craft: u32) -> Self {
        Self {
            target,
            craft,
            offset_raw: [0; 3],
            phase: -1,
            elapsed_us: 0,
            duration_us: 0,
            fade: 0,
        }
    }

    /// `FUN_00409460` for one visit of `elapsed_us`. `target` and `craft` are
    /// the bodies' current positions (`None` when gone); `ground` is the
    /// current terrain's bilinear height. Writes the ball's position. This
    /// is the MindBall task's (`FUN_00409320`) per-visit work on its record.
    pub fn advance(
        &mut self,
        ball_raw: &mut [i16; 3],
        target: Option<[i16; 3]>,
        craft: Option<[i16; 3]>,
        elapsed_us: i32,
        ground: impl Fn(i16, i16) -> i16,
    ) -> MindBallStep {
        let Some(craft) = craft else {
            return MindBallStep::Die;
        };
        let elapsed = self.elapsed_us.wrapping_add(elapsed_us);
        self.elapsed_us = elapsed;
        let mut t = 0;
        if elapsed < self.duration_us && self.phase > -1 {
            t = ((elapsed as u32).wrapping_mul(0x40) as i32) / (self.duration_us >> 10);
            t = t.min(0xFFFF);
        } else {
            self.elapsed_us = 0;
            self.phase += 1;
            match self.phase {
                0 | 3 => self.duration_us = MIND_BALL_PHASE_US,
                1 => {
                    self.offset_raw = *ball_raw;
                    self.duration_us = MIND_BALL_PHASE_US;
                }
                2 => {
                    let (distance, offset) = octagonal_offset(*ball_raw, craft);
                    self.offset_raw = offset;
                    self.duration_us = distance
                        .wrapping_mul(0x100)
                        .wrapping_add(MIND_BALL_RETURN_BASE_US);
                }
                _ => return MindBallStep::Die,
            }
        }
        let rise = (t >> 5) as i16;
        match self.phase {
            0 => {
                self.fade = t / 2;
                if let Some(target) = target {
                    *ball_raw = target;
                }
            }
            1 => {
                self.fade = (t + 0x10000) / 2;
                ball_raw[1] = ((t >> 4) as i16).wrapping_add(self.offset_raw[1]);
            }
            2 => {
                let left = 0x10000 - t;
                let scaled = |axis: usize| {
                    (i32::from(self.offset_raw[axis]).wrapping_mul(left) >> 16) as i16
                };
                ball_raw[0] = scaled(0).wrapping_add(craft[0]);
                ball_raw[1] = scaled(1).wrapping_add(rise).wrapping_add(craft[1]);
                ball_raw[2] = scaled(2).wrapping_add(craft[2]);
                let floor = ground(ball_raw[0], ball_raw[2]);
                if ball_raw[1] < floor {
                    ball_raw[1] = floor;
                }
            }
            _ => {
                self.fade = 0xFFFF - t;
                *ball_raw = craft;
                ball_raw[1] = ball_raw[1].wrapping_add(0x800i16.wrapping_sub(rise));
            }
        }
        MindBallStep::Fly
    }
}

/// `FUN_004097A0`'s process-global spin, `DAT_004DAEDC` (zero at load).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MindBallSpin(u32);

/// One instance of global model 243 the MindBall's draw submits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MindBallInstance {
    /// Three consecutive Q31 vectors, as a body's `+0x0C`.
    pub basis: [[i32; 3]; 3],
    /// `+0x44` of the draw record, which `FUN_00421570` gives selector 1.
    pub size_word: u16,
}

impl MindBallInstance {
    /// The renderer takes world-from-model rows; the record holds local axes.
    pub fn orientation(&self) -> [[f32; 3]; 3] {
        std::array::from_fn(|row| {
            std::array::from_fn(|column| {
                (f64::from(self.basis[column][row]) / 2_147_483_648.0) as f32
            })
        })
    }
}

impl MindBallSpin {
    /// `FUN_004097A0` for one draw of a ball with body matrix `basis` and
    /// fade word `fade` (record `+20`, at most `0xFFFF`): the spin advances by
    /// the frame's microseconds over 32, then two rings of four instances of
    /// model 243 each turn the matrix once more by the spin about its lateral
    /// axis and by twice it about its forward axis; the first ring's size is
    /// `0xFFFF - fade`, the second's `0xFFFF - (fade >> 3)`.
    pub fn draw(
        &mut self,
        basis: [[i32; 3]; 3],
        fade: i32,
        frame_us: u32,
    ) -> Vec<MindBallInstance> {
        self.0 = self.0.wrapping_add(((frame_us as i32) >> 5) as u32);
        let fade = fade.min(0xFFFF);
        let mut matrix = basis;
        let mut instances = Vec::with_capacity(2 * MIND_BALL_RING_INSTANCES);
        for size in [0xFFFF - fade, 0xFFFF - (fade >> 3)] {
            for _ in 0..MIND_BALL_RING_INSTANCES {
                turn_up_forward(&mut matrix, self.0);
                turn_lateral_up(&mut matrix, self.0.wrapping_mul(2));
                instances.push(MindBallInstance {
                    basis: matrix,
                    size_word: size as u16,
                });
            }
        }
        instances
    }
}

/// The sine table word in both halves (`DAT_004D14D0`), as the matrix turns
/// read it.
fn table_sine_q31(angle: u32) -> i32 {
    let word = retail_sine_q15(angle & 0x7FFF) as u32;
    let value = ((word << 16) | word) as i32;
    if angle & 0x8000 != 0 {
        value.wrapping_neg()
    } else {
        value
    }
}

/// `FUN_00457A90`: the up and forward rows turn by `angle`.
fn turn_up_forward(matrix: &mut [[i32; 3]; 3], angle: u32) {
    let sin = table_sine_q31(angle);
    let cos = table_sine_q31(angle.wrapping_add(0x4000));
    for axis in 0..3 {
        let (up, forward) = (matrix[1][axis], matrix[2][axis]);
        matrix[1][axis] = q31_mul(sin, forward).wrapping_add(q31_mul(cos, up));
        matrix[2][axis] = q31_mul(cos, forward).wrapping_add(q31_mul(sin.wrapping_neg(), up));
    }
}

/// `FUN_00457DD0`: the lateral and up rows turn by `angle`.
fn turn_lateral_up(matrix: &mut [[i32; 3]; 3], angle: u32) {
    let sin = table_sine_q31(angle);
    let cos = table_sine_q31(angle.wrapping_add(0x4000));
    for axis in 0..3 {
        let (lateral, up) = (matrix[0][axis], matrix[1][axis]);
        matrix[0][axis] = q31_mul(sin, up).wrapping_add(q31_mul(cos, lateral));
        matrix[1][axis] = q31_mul(cos, up).wrapping_add(q31_mul(sin.wrapping_neg(), lateral));
    }
}

/// `FUN_00425370`: the wrapped offset `to - from` and its octagonal length.
fn octagonal_offset(to: [i16; 3], from: [i16; 3]) -> (i32, [i16; 3]) {
    let offset: [i16; 3] = std::array::from_fn(|axis| to[axis].wrapping_sub(from[axis]));
    let [x, y, z] = offset.map(|word| i32::from(word).abs());
    let distance = if y < x {
        if x < z {
            ((y + x) >> 1) + z
        } else {
            ((z + y) >> 1) + x
        }
    } else if y < z {
        ((y + x) >> 1) + z
    } else {
        ((z + x) >> 1) + y
    };
    (distance, offset)
}

pub mod live;

#[cfg(test)]
mod tests;
