//! Class69 Shark Attack: `FUN_004056D0` and its record, tick and pair hook.
//!
//! Type25's hunting program. `FUN_0040D2D0` clears Secondary and Tertiary,
//! then `56D0` builds the Primary through `01020` directly: no `06070` Sub-F
//! reset and no lifetime. `FUN_00405D50`'s record holds the prey handle, the
//! phase, a microsecond timer, an `012E0` target record and three speeds. The
//! tick (`FUN_00405760`) hunts (`FUN_00405950`), circles (`FUN_004059E0`) or
//! attacks (`FUN_00405C00`) over the incoming body basis; its result tag
//! `9C00` reaches the style's null `+04`, so the hunt never ends by itself.
//! The pair hook is `FUN_00405E10`. Distances are `FUN_00425370`'s and the
//! ahead test is `FUN_0041E930`'s.

use crate::wander_near_location::WanderNearPrivateState;

/// Retail `0x578`: the circling radius and the attack's close range.
const RING_RAW: i32 = 0x578;
/// Circling blends its speeds inside this distance (`0x15E0`).
const BLEND_RAW: i32 = 0x15e0;
/// A prey this far away (`0x3139`) ends the hunt.
const LOST_RAW: i32 = 0x3139;
/// `5E10` holds the attack timer at 100 ms after a bite.
pub const BITE_TIMER_MICROS: u32 = 100_000;
/// `5E10`'s checked packet: channel1, 8,000, scaled by the global frame
/// delta over `25000 << 2`.
pub const BITE_DAMAGE_RAW: i32 = 8000;
pub const BITE_RATIO_DENOMINATOR: u16 = 25_000;
/// Class45 bubbles at the prey when it is below the sea.
pub const BITE_PARTICLE_CLASS: u8 = 0x2d;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharkAttackTaskState {
    prey: u32,
    phase: u32,
    timer_micros: u32,
    target: WanderNearPrivateState,
    speeds: [i32; 2],
    speed: i32,
}

/// The body the tick and hook read: position, velocity and the forward
/// column (`+24..+2C`) of the incoming basis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharkBody {
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub forward_q31: [i32; 3],
}

/// Sub-F words `5760` writes before `01430`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharkSwimmerWrite {
    /// `+3D`: set only while attacking.
    pub follow_target: bool,
    /// `+34`, written outside the attack once the body is more than 16 units
    /// from the goal height.
    pub clearance_raw: Option<i32>,
    /// `+38`.
    pub target_speed_raw: i32,
}

impl SharkAttackTaskState {
    /// `5D50` after its allocation: `012E0` with a null handle at the body's
    /// position, then speeds `[0C]` and `[0D]`, one word each, between
    /// 128/256 and 191/256 of the swimmer's speed word `+4`.
    pub fn new(
        position_raw: [i16; 3],
        swimmer_speed_raw: i16,
        next_random: &mut impl FnMut() -> u32,
    ) -> Self {
        let mut draw =
            || (((next_random() & 0x3f) as i32 + 0x80) * i32::from(swimmer_speed_raw)) >> 8;
        let first = draw();
        let second = draw();
        Self {
            prey: 0,
            phase: 0,
            timer_micros: 0,
            target: WanderNearPrivateState::ordinary_type9(position_raw),
            speeds: [first, second],
            speed: 0,
        }
    }

    pub const fn prey(&self) -> u32 {
        self.prey
    }
    pub const fn phase(&self) -> u32 {
        self.phase
    }
    pub const fn timer_micros(&self) -> u32 {
        self.timer_micros
    }
    pub const fn speeds(&self) -> [i32; 2] {
        self.speeds
    }
    pub const fn speed(&self) -> i32 {
        self.speed
    }
    pub const fn target(&self) -> &WanderNearPrivateState {
        &self.target
    }
    pub fn target_mut(&mut self) -> &mut WanderNearPrivateState {
        &mut self.target
    }

    /// `5760` before `01430`: hunt, count the timer down, steer, then fill
    /// the swimmer's words. `candidates` is the live list as (handle,
    /// capability, position); `resolve` returns a live handle's position.
    pub fn before_mover(
        &mut self,
        body: SharkBody,
        elapsed_micros: u32,
        sea_level_raw: i16,
        cell_height_raw: i8,
        candidates: &[(u32, u32, [i16; 3])],
        resolve: impl Fn(u32) -> Option<[i16; 3]>,
        next_random: &mut impl FnMut() -> u32,
    ) -> SharkSwimmerWrite {
        if self.phase == 0 {
            self.find_prey(body.position_raw, candidates, next_random);
        }
        self.timer_micros = self.timer_micros.saturating_sub(elapsed_micros);
        match self.phase as i32 {
            1 | 2 => self.circle(body, resolve(self.prey), next_random),
            3 => self.attack(body, resolve(self.prey), next_random),
            _ => {}
        }
        let cap = i32::from(sea_level_raw) - 0x80;
        if cap < i32::from(self.target.target_position_raw[1]) {
            self.target.target_position_raw[1] = cap as i16;
        }
        let goal_y = i32::from(self.target.target_position_raw[1]);
        let follow_target = self.phase == 3;
        let clearance_raw = (!follow_target
            && (i32::from(body.position_raw[1]) - goal_y).abs() > 0x10)
            .then(|| goal_y - i32::from((i16::from(cell_height_raw)) << 5));
        SharkSwimmerWrite {
            follow_target,
            clearance_raw,
            target_speed_raw: self.speed,
        }
    }

    /// `5760`'s result: null while hunting, tag `9C00` once idle.
    pub const fn hunting(&self) -> bool {
        self.phase != 0
    }

    /// `5950`: the nearest capability-bit0 body by `25370` distance; each
    /// strictly nearer one draws a fresh timer.
    fn find_prey(
        &mut self,
        position_raw: [i16; 3],
        candidates: &[(u32, u32, [i16; 3])],
        next_random: &mut impl FnMut() -> u32,
    ) {
        let mut best = 0xffff;
        for &(id, capability, prey) in candidates {
            if capability & 1 == 0 {
                continue;
            }
            let distance = distance(prey, position_raw).1;
            if distance < best {
                self.prey = id;
                self.phase = 1;
                self.timer_micros = (next_random() & 0xffff) << 7;
                best = distance;
            }
        }
    }

    /// `59E0`: phase1 rings the prey at 1,400 units; phase2 swims 1,400
    /// units onward from the body and attacks once far enough away.
    fn circle(
        &mut self,
        body: SharkBody,
        prey: Option<[i16; 3]>,
        next_random: &mut impl FnMut() -> u32,
    ) {
        let Some(prey) = prey else {
            self.phase = 0;
            return;
        };
        let ahead = [
            body.velocity_raw[0].wrapping_add(body.position_raw[0]),
            prey[1],
            body.velocity_raw[2].wrapping_add(body.position_raw[2]),
        ];
        let (delta, distance) = distance(ahead, prey);
        let mut goal = ahead;
        goal[1] = ((distance << 5) / RING_RAW) as i16;
        goal[1] = goal[1]
            .wrapping_sub(body.position_raw[1])
            .wrapping_add(prey[1].wrapping_mul(2))
            .wrapping_sub(0x20);
        if self.phase == 1 {
            if distance != 0 {
                goal[0] = ((i32::from(delta[0]) * RING_RAW) / distance) as i16;
                goal[0] = goal[0].wrapping_add(prey[0]);
                goal[2] = ((i32::from(delta[2]) * RING_RAW) / distance) as i16;
                goal[2] = goal[2].wrapping_add(prey[2]);
            }
            self.speed = self.speeds[1] / 2 + self.speeds[0];
        } else {
            if distance != 0 {
                goal[0] = ((i32::from(delta[0]) * RING_RAW) / distance) as i16;
                goal[0] = goal[0].wrapping_add(body.position_raw[0]);
                goal[2] = ((i32::from(delta[2]) * RING_RAW) / distance) as i16;
                goal[2] = goal[2].wrapping_add(body.position_raw[2]);
            }
            if distance <= BLEND_RAW {
                self.speed =
                    self.speeds[0] - distance * self.speeds[1] / BLEND_RAW + self.speeds[1];
            } else {
                self.phase = 3;
                self.timer_micros = ((next_random() & 0xffff) + 0x2625a) * 0x40;
            }
        }
        self.target.target_position_raw = goal;
        if self.timer_micros == 0 {
            self.phase = 2;
            self.timer_micros = ((next_random() & 0xffff) + 0xf424) * 0x40;
        }
        if distance >= LOST_RAW {
            self.phase = 0;
        }
    }

    /// `5C00`: steer at the prey, faster the more squarely it lies ahead.
    fn attack(
        &mut self,
        body: SharkBody,
        prey: Option<[i16; 3]>,
        next_random: &mut impl FnMut() -> u32,
    ) {
        let Some(prey) = prey else {
            self.phase = 0;
            return;
        };
        let (delta, distance) = distance(prey, body.position_raw);
        let lift = ((distance << 5) / RING_RAW) as i16;
        self.target.target_position_raw = [
            prey[0],
            lift.wrapping_add(prey[1])
                .wrapping_sub(0x20)
                .wrapping_add(delta[1]),
            prey[2],
        ];
        let dot = projection(delta, body.forward_q31);
        if distance < RING_RAW {
            if dot < 1 {
                self.phase = 1;
            }
        } else if self.timer_micros == 0 {
            if dot < 1 {
                self.phase = 1;
            }
            self.timer_micros = ((next_random() & 0xffff) + 0x3d09) * 0x80;
        }
        self.speed = self.speeds[0];
        if dot > 0 && distance != 0 {
            self.speed = self.speeds[1] * dot / distance + self.speeds[0];
        }
    }

    /// `5E10` for one touched body. A body behind ends an attack with one
    /// word; touching the prey ahead returns the bite for the caller.
    pub fn pair_hook(
        &mut self,
        body: SharkBody,
        touched: u32,
        touched_position_raw: [i16; 3],
        next_random: &mut impl FnMut() -> u32,
    ) -> SharkPairHook {
        if !ahead(body, touched_position_raw) {
            if self.phase == 3 {
                self.phase = 2;
                self.timer_micros = ((next_random() & 0xffff) + 0xf424) * 0x40;
                return SharkPairHook::AttackBroken;
            }
            return SharkPairHook::Behind;
        }
        if self.prey != touched {
            return SharkPairHook::Ahead;
        }
        self.timer_micros = BITE_TIMER_MICROS;
        SharkPairHook::Bite
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharkPairHook {
    Behind,
    AttackBroken,
    Ahead,
    Bite,
}

/// `25370`: `a - b` in signed words and the largest component plus half the
/// other two.
fn distance(a: [i16; 3], b: [i16; 3]) -> ([i16; 3], i32) {
    let delta: [i16; 3] = std::array::from_fn(|axis| a[axis].wrapping_sub(b[axis]));
    let absolute = delta.map(|value| i32::from(value).abs());
    let largest = *absolute.iter().max().unwrap();
    (
        delta,
        largest + ((absolute.iter().sum::<i32>() - largest) >> 1),
    )
}

/// Signed Q31 projection on the forward column, as `1E930` and `5C00` sum it.
fn projection(delta: [i16; 3], forward_q31: [i32; 3]) -> i32 {
    (0..3).fold(0_i32, |sum, axis| {
        sum.wrapping_add(((i64::from(delta[axis]) * i64::from(forward_q31[axis])) >> 31) as i32)
    })
}

/// `1E930`: the point is ahead when its projection is not negative.
fn ahead(body: SharkBody, point_raw: [i16; 3]) -> bool {
    let delta = std::array::from_fn(|axis| point_raw[axis].wrapping_sub(body.position_raw[axis]));
    projection(delta, body.forward_q31) >= 0
}

/// The bite's particle count: `(delta * 50) >> 20`, run at least once.
pub const fn bite_particle_count(global_elapsed_micros: u32) -> u32 {
    let count = (global_elapsed_micros.wrapping_mul(0x32) as i32) >> 0x14;
    if count < 1 {
        1
    } else {
        count as u32
    }
}

#[cfg(test)]
mod tests;
