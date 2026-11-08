//! The Possession Ray's mind-control lock and its reticle.
//!
//! Selector `0x1B`'s launch (`FUN_0044E770`) fires nothing; it starts the
//! lock (`FUN_00421800`) on the shooter's lock record, its component `+0x38`
//! (mode `+34`, mode time `+38`, target `+3C`, locked time `+40`, reticle
//! `+44`, reticle angle `+48`). Player Control advances it every frame outside
//! network play (`FUN_00420A50` -> `FUN_00420C60`): a search of up to 1.5 s
//! (`FUN_00420E00`), then a lock held while the target stays ahead
//! (`FUN_00421070`), and after 1 s locked the takeover (`FUN_004480E0`). The
//! craft's draw callback (`FUN_00421590`) draws the reticle (`FUN_00421230`)
//! while a lock runs. See `docs/re/PLAYER_CRAFT.md#mind-control`.

use v2k_formats::{fixed_math::retail_integer_sqrt, models::AnimVars};

use crate::{
    common_mover::{target_correction::normalize_retail_q31, type9_attitude::Type9BodyBasis},
    entity::Entity,
    entity_collision_state::{
        active_model_slot_from_state_flags, recent_relation_suppresses_pair, RetailRuntimeValue,
        ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
    },
    hover::q31_mul,
    resource_cache::ResourceCache,
};

/// `FUN_00421800` restarts a running lock only after this much mode time.
pub const LOCK_RESTART_AFTER_US: u32 = 1_000_000;
/// `FUN_00420C60` searches only below this mode time.
pub const LOCK_SEARCH_LIMIT_US: u32 = 1_500_000;
/// `FUN_00420C60` takes control once the locked time exceeds this.
pub const LOCK_TAKEOVER_AFTER_US: i32 = 1_000_000;
/// The reticle word's ceiling.
pub const RETICLE_MAX: u32 = 0x600;
/// `FUN_00421230` draws global model 76 (`DAT_004FE640 + 0x130`).
pub const RETICLE_MODEL_ID: usize = 76;

/// The takeover's stop: `FUN_004480E0` makes the target's own body run
/// Player Control, which the port runs only on the craft.
pub const POSSESSION_TAKEOVER_HELD: &str = "Possession Ray takeover held: FUN_004480E0 runs Player Control on the target's own body, which the port runs only on a Type17 with an empty hold or a native captor's captured body (docs/re/PLAYER_CRAFT.md#mind-control)";

/// Capability bit 3 (`+0x64`): a body the search may lock.
const LOCKABLE_CAPABILITY: u32 = 0x8;
const DYING_STATE: u32 = 0x4000;
const ADMITTED_STATE: u32 = 0x8000;
const NON_SOLID_STATE: u32 = 0x0800_0000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LockMode {
    #[default]
    Idle,
    Searching,
    Locked,
}

impl LockMode {
    /// The record's mode word `+34`.
    pub const fn word(self) -> u32 {
        match self {
            Self::Idle => 0,
            Self::Searching => 1,
            Self::Locked => 2,
        }
    }
}

/// The lock record, component `+0x38` of the shooter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MindControlLock {
    pub mode: LockMode,
    pub mode_time_us: u32,
    /// `+3C`: the last body the search chose; it outlives the lock.
    pub target: Option<u32>,
    pub locked_time_us: i32,
    /// `+44`: the advance's growing reticle, or the draw's distance to the
    /// target while locked.
    pub reticle: i32,
    /// `+48`: the reticle's phase, within `0..=0x80`.
    pub angle: i32,
}

/// What one advance asks of its caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockAdvance {
    Continue,
    /// `FUN_004480E0(shooter, target)`: the lock has ended in a takeover.
    TakeOver {
        target: u32,
    },
}

/// A value the lock needs that the port does not hold for a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockBlock {
    StateFlags {
        entity_id: u32,
    },
    RecentRelation {
        entity_id: u32,
    },
    /// The locked target's active model, whose radius the reticle reads.
    TargetModel {
        entity_id: u32,
    },
}

/// The craft's lock record. It is a component of the craft's body, so a new
/// body starts with a zeroed one.
#[derive(Debug, Default, Clone)]
pub struct PlayerMindControlLock {
    body: Option<u32>,
    lock: MindControlLock,
}

impl PlayerMindControlLock {
    pub fn for_body(&mut self, body: u32) -> &mut MindControlLock {
        if self.body != Some(body) {
            *self = Self {
                body: Some(body),
                lock: MindControlLock::default(),
            };
        }
        &mut self.lock
    }

    pub fn of_body(&self, body: u32) -> Option<&MindControlLock> {
        (self.body == Some(body)).then_some(&self.lock)
    }
}

/// The radius word (`+08`) of the model a body's state word selects
/// (`+A8..+AE` by bits `0x2000`/`0x4000`).
pub fn target_model_radius(cache: &ResourceCache, target: &Entity) -> Result<u16, LockBlock> {
    let block = LockBlock::TargetModel {
        entity_id: target.id,
    };
    let RetailRuntimeValue::Known(selector) = target
        .collision
        .state_flags_at_0x08
        .masked(ACTIVE_MODEL_SLOT_LOW_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT)
    else {
        return Err(block);
    };
    target.model_slots[active_model_slot_from_state_flags(selector)]
        .and_then(|id| cache.global_model(id))
        .map(|model| model.radius)
        .ok_or(block)
}

/// The shooter as the lock reads it: its body and its matrix `+0x0C`.
#[derive(Clone, Copy)]
pub struct LockShooter<'a> {
    pub body: &'a Entity,
    pub basis: Type9BodyBasis,
}

impl MindControlLock {
    /// A typed stop ends the lock as an expired search does.
    pub fn stop(&mut self) {
        self.mode = LockMode::Idle;
        self.mode_time_us = 0;
    }

    /// `FUN_00421800`: start a search when idle or more than 1 s into a mode.
    pub fn start(&mut self) {
        if self.mode == LockMode::Idle || self.mode_time_us > LOCK_RESTART_AFTER_US {
            self.mode = LockMode::Searching;
            self.mode_time_us = 0;
            self.angle = 0;
        }
    }

    /// `FUN_00420C60` for one Player Control frame of `elapsed_us`.
    /// `bodies` is the live list in retail order; the shooter is among them.
    pub fn advance<'a>(
        &mut self,
        shooter: LockShooter<'_>,
        bodies: impl Iterator<Item = &'a Entity> + Clone,
        elapsed_us: i32,
    ) -> Result<LockAdvance, LockBlock> {
        if self.mode == LockMode::Idle {
            return Ok(LockAdvance::Continue);
        }
        self.mode_time_us = self.mode_time_us.wrapping_add(elapsed_us as u32);
        self.reticle =
            ((self.mode_time_us >> 10).wrapping_mul(0x600) / 0x5B8).min(RETICLE_MAX) as i32;
        let mut outcome = LockAdvance::Continue;
        let next = match body_liveness(shooter.body)? {
            Liveness::Live => match self.mode {
                LockMode::Searching => {
                    self.angle += ((elapsed_us >> 6) * 0x600) / 0x5B8D;
                    if self.mode_time_us < LOCK_SEARCH_LIMIT_US {
                        if let Some(target) = self.search(shooter, bodies)? {
                            self.target = Some(target);
                            self.locked_time_us = 0;
                            LockMode::Locked
                        } else {
                            LockMode::Searching
                        }
                    } else {
                        LockMode::Idle
                    }
                }
                LockMode::Locked => {
                    self.angle += ((elapsed_us >> 6) * -0x600) / 0xB71B;
                    let target = self
                        .target
                        .and_then(|id| bodies.clone().find(|body| body.id == id));
                    if !self.holds(shooter, target)? {
                        LockMode::Searching
                    } else {
                        self.locked_time_us = self.locked_time_us.wrapping_add(elapsed_us);
                        if self.locked_time_us > LOCK_TAKEOVER_AFTER_US {
                            if let Some(target) = self.target {
                                outcome = LockAdvance::TakeOver { target };
                            }
                            LockMode::Idle
                        } else {
                            LockMode::Locked
                        }
                    }
                }
                LockMode::Idle => LockMode::Idle,
            },
            Liveness::Dying | Liveness::Dead => LockMode::Idle,
        };
        while self.angle < 0 {
            self.angle += 0x80;
        }
        while self.angle > 0x80 {
            self.angle -= 0x80;
        }
        if next != self.mode {
            self.mode = next;
            if next == LockMode::Idle {
                self.mode_time_us = 0;
            }
        }
        Ok(outcome)
    }

    /// `FUN_00420E00`: the closest lockable body inside the forward cone,
    /// within 4/3 of the reticle, scored by forward distance plus twice the
    /// lateral offsets. The first of equal scores in list order wins.
    fn search<'a>(
        &self,
        shooter: LockShooter<'_>,
        bodies: impl Iterator<Item = &'a Entity>,
    ) -> Result<Option<u32>, LockBlock> {
        let mut best = i32::MAX;
        let mut chosen = None;
        let range = (self.reticle << 2) / 3;
        for body in bodies {
            if body.capability_flags & LOCKABLE_CAPABILITY == 0 || body.id == shooter.body.id {
                continue;
            }
            if body_liveness(body)? != Liveness::Live {
                continue;
            }
            match recent_relation_suppresses_pair(
                body.id,
                &body.collision,
                shooter.body.id,
                &shooter.body.collision,
            ) {
                RetailRuntimeValue::Known(false) => {}
                RetailRuntimeValue::Known(true) => continue,
                RetailRuntimeValue::Unresolved => {
                    return Err(LockBlock::RecentRelation { entity_id: body.id })
                }
            }
            let flags = &body.collision.state_flags_at_0x08;
            let (RetailRuntimeValue::Known(admitted), RetailRuntimeValue::Known(non_solid)) =
                (flags.masked(ADMITTED_STATE), flags.masked(NON_SOLID_STATE))
            else {
                return Err(LockBlock::StateFlags { entity_id: body.id });
            };
            if admitted == 0 || non_solid != 0 {
                continue;
            }
            let (distance, offset) =
                octagonal_offset(body.position_raw(), shooter.body.position_raw());
            if distance >= range {
                continue;
            }
            let [lateral, up, forward] = project(offset, shooter.basis);
            if forward <= 0x100 {
                continue;
            }
            let (lateral, up) = (retail_abs(lateral), retail_abs(up));
            if lateral < forward / 2 && up < forward / 2 {
                let score = forward.wrapping_add(lateral.wrapping_add(up).wrapping_mul(2));
                if score < best {
                    best = score;
                    chosen = Some(body.id);
                }
            }
        }
        Ok(chosen)
    }

    /// `FUN_00421070`: the target lives, lies within four reticles and stays
    /// ahead with neither lateral offset beyond the forward distance.
    fn holds(&self, shooter: LockShooter<'_>, target: Option<&Entity>) -> Result<bool, LockBlock> {
        let Some(target) = target else {
            return Ok(false);
        };
        if body_liveness(target)? != Liveness::Live {
            return Ok(false);
        }
        let (distance, offset) =
            octagonal_offset(target.position_raw(), shooter.body.position_raw());
        if distance > self.reticle.wrapping_mul(4) {
            return Ok(false);
        }
        let [lateral, up, forward] = project(offset, shooter.basis);
        Ok(forward > 0x80 && retail_abs(lateral) <= forward && retail_abs(up) <= forward)
    }

    /// `FUN_00421230` for one draw: model 76 instances along the line from
    /// the shooter, its forward axis while searching, to the target and
    /// around it while locked. A locked draw stores the distance in `+44`.
    pub fn reticle_submissions(
        &mut self,
        shooter: LockShooter<'_>,
        target: Option<(&Entity, u16)>,
    ) -> Result<Vec<ReticleSubmission>, LockBlock> {
        if self.mode == LockMode::Idle || body_liveness(shooter.body)? != Liveness::Live {
            return Ok(Vec::new());
        }
        let origin = shooter.body.position_raw();
        let Type9BodyBasis {
            lateral,
            up,
            forward,
        } = shooter.basis;
        let mut basis = [lateral, up, forward];
        let mut radius = 0i32;
        if self.mode == LockMode::Locked {
            if let Some((target, model_radius)) = target {
                radius = i32::from(model_radius);
                let to = target.position_raw();
                let delta: [i32; 3] =
                    std::array::from_fn(|axis| i32::from(to[axis]) - i32::from(origin[axis]));
                let squared = delta
                    .iter()
                    .fold(0i32, |sum, word| sum.wrapping_add(word.wrapping_mul(*word)));
                self.reticle = i32::from(retail_integer_sqrt(squared) as i16);
                let dir = normalize_retail_q31(delta);
                let side = normalize_retail_q31([dir[2], 0, dir[0].wrapping_neg()]);
                let up = [
                    q31_mul(dir[1], side[2]).wrapping_sub(q31_mul(dir[2], side[1])),
                    q31_mul(dir[2], side[0]).wrapping_sub(q31_mul(dir[0], side[2])),
                    q31_mul(dir[0], side[1]).wrapping_sub(q31_mul(dir[1], side[0])),
                ];
                basis = [side, up, dir];
            }
        }
        let dir = basis[2];
        let length = radius.wrapping_add(self.reticle);
        let mut submissions = Vec::new();
        let mut step = self.angle;
        while step < length {
            let position_raw = std::array::from_fn(|axis| {
                (q31_mul(dir[axis], step) as i16).wrapping_add(origin[axis])
            });
            step = step.wrapping_add(0x80);
            let size = if step < self.reticle.wrapping_sub(radius) {
                0x14
            } else {
                let past = step.wrapping_sub(self.reticle);
                let half_chord = retail_integer_sqrt(
                    radius
                        .wrapping_mul(radius)
                        .wrapping_sub(past.wrapping_mul(past)),
                ) as i16;
                i32::from(half_chord).wrapping_mul(0xFFFF) / 1024
            };
            submissions.push(ReticleSubmission {
                position_raw,
                basis,
                size_word: size as u16,
            });
        }
        Ok(submissions)
    }

    /// The record's six words, as the event log writes them.
    pub fn words(&self) -> [i64; 6] {
        [
            i64::from(self.mode.word()),
            i64::from(self.mode_time_us),
            self.target.map_or(-1, i64::from),
            i64::from(self.locked_time_us),
            i64::from(self.reticle),
            i64::from(self.angle),
        ]
    }
}

/// One model-76 instance of the reticle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReticleSubmission {
    pub position_raw: [i16; 3],
    /// Three consecutive Q31 vectors, as a body's `+0x0C`.
    pub basis: [[i32; 3]; 3],
    /// `+0x44` of the draw record: `0x14` along the line, the chord around
    /// the target.
    pub size_word: u16,
}

impl ReticleSubmission {
    /// `FUN_00421570`: selector 0 is the world clock, 1 the size word.
    pub fn anim_vars(&self, retail_tick: u32) -> AnimVars {
        let mut vars = AnimVars::default();
        vars.dynamic[0] = i32::from(retail_tick as u16);
        vars.dynamic[1] = i32::from(self.size_word);
        vars
    }

    /// The renderer takes world-from-model rows; the record holds local axes.
    pub fn orientation(&self) -> [[f32; 3]; 3] {
        std::array::from_fn(|row| {
            std::array::from_fn(|column| {
                (f64::from(self.basis[column][row]) / 2_147_483_648.0) as f32
            })
        })
    }

    /// Each signed word unwrapped against the camera's full dword.
    pub fn unwrapped_position_raw(&self, camera_position_raw: [i32; 3]) -> [i32; 3] {
        std::array::from_fn(|axis| {
            camera_position_raw[axis].wrapping_add(i32::from(
                self.position_raw[axis].wrapping_sub(camera_position_raw[axis] as i16),
            ))
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Liveness {
    Live,
    Dying,
    Dead,
}

/// A resolved body's `+08` test: `0x4000` dying, zero dead.
fn body_liveness(body: &Entity) -> Result<Liveness, LockBlock> {
    let flags = &body.collision.state_flags_at_0x08;
    match flags.masked(DYING_STATE) {
        RetailRuntimeValue::Known(0) => {}
        RetailRuntimeValue::Known(_) => return Ok(Liveness::Dying),
        RetailRuntimeValue::Unresolved => return Err(LockBlock::StateFlags { entity_id: body.id }),
    }
    if flags.known_value_bits() != 0 {
        Ok(Liveness::Live)
    } else if flags.known_mask() == u32::MAX {
        Ok(Liveness::Dead)
    } else {
        Err(LockBlock::StateFlags { entity_id: body.id })
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

/// The offset on the shooter's three matrix rows: lateral, up, forward.
fn project(offset: [i16; 3], basis: Type9BodyBasis) -> [i32; 3] {
    [basis.lateral, basis.up, basis.forward].map(|row| {
        (0..3).fold(0i32, |sum, axis| {
            sum.wrapping_add(q31_mul(i32::from(offset[axis]), row[axis]))
        })
    })
}

fn retail_abs(value: i32) -> i32 {
    let sign = value >> 31;
    (value ^ sign).wrapping_sub(sign)
}

#[cfg(test)]
mod tests;
