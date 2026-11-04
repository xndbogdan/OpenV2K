//! Type110, the Queen of world 47: class67's `FUN_0040D210` Primary.
//!
//! The Queen lives on the class0 host's owner. Her record (`FUN_00421B40`)
//! hangs off Sub-K `+3C`, so it outlives the Primary: `FUN_00410C10` takes
//! class67 again on a dying body and the same record carries on. Her two guns
//! aim and fire only while her model is drawn: the type-14 selectors 0 and 1
//! of models 1117 and 1119 reach `FUN_00421AE0` from `FUN_0040A9F0`, which
//! stamps each gun's muzzle and sets its two-tick "drawn" count. A gun whose
//! count has run out holds its pose and never fires.

use crate::{
    entity::{Class0ActorError, EntityManager},
    entity_collision_state::{
        RetailRuntimeValue, DYING_STATE_BIT, PAIR_COLLISION_ENABLED_STATE_BIT,
    },
    intro2_gun_turret::task::math::{asin, displacement, length, q31, ratio, rotate},
    world_fx::{DescriptorParticleRequest, ParticleOwnerAtBirth, WorldFx},
};

#[cfg(test)]
mod tests;

pub const ENTITY_TYPE: u32 = 110;
/// `FUN_004225C0` writes this word to the held Type68's `+A8`.
pub const HELD_BODY_MODEL: usize = 286;
/// `FUN_00421AE0`'s source slots: selector 0 stamps the second gun from slot
/// 0x1C, selector 1 the first gun from slot 0x1B.
pub const SECOND_GUN_SLOT: u16 = 0x1c;
pub const FIRST_GUN_SLOT: u16 = 0x1b;
/// The dying Queen calls `FUN_00456F20` once `+00` passes this.
const DEATH_END_US: i32 = 29_999_999;

/// One joint's rate and angle limits (`004C9408`, `004C9420`, `004C9438`):
/// yaw rate, pitch rate, yaw angle, pitch maximum, pitch minimum.
type JointLimits = [i32; 5];
const FIRST_GUN_LIMITS: JointLimits = [0x8000, 0x4000, 0x3000, 0x1000, -0x2000];
const NECK_LIMITS: JointLimits = [0x6000, 0x4000, 0x3000, 0x800, -0x2000];
const SECOND_GUN_LIMITS: JointLimits = [0x8000, 0x4000, 0x7400, 0, -0x4000];

/// A joint's first 0x18 bytes: two Sub-K pointers, the yaw/pitch words, the
/// two rates and the limits pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueenJoint {
    pub yaw: i16,
    pub pitch: i16,
    pub yaw_rate: i32,
    pub pitch_rate: i32,
    limits: JointLimits,
}

impl QueenJoint {
    const fn new(limits: JointLimits) -> Self {
        Self {
            yaw: 0,
            pitch: 0,
            yaw_rate: 0,
            pitch_rate: 0,
            limits,
        }
    }

    /// `FUN_00421C60`: frame-scaled demands on the two rates (doubled against
    /// the current rate), then the rates and the angles clamped.
    fn drive(&mut self, frame_dt: u32, pitch_demand: i32, yaw_demand: i32) {
        let k = (frame_dt as i32) >> 5;
        let pitch_step = i32::from((k.wrapping_mul(pitch_demand) >> 15) as i16);
        let mut yaw_step = i32::from((k.wrapping_mul(yaw_demand) >> 15) as i16);
        if yaw_step.wrapping_mul(self.yaw_rate) < 0 {
            yaw_step = yaw_step.wrapping_mul(2);
        }
        self.yaw_rate = self.yaw_rate.wrapping_add(yaw_step);
        self.pitch_rate = if pitch_step.wrapping_mul(self.pitch_rate) < 0 {
            self.pitch_rate.wrapping_add(pitch_step.wrapping_mul(2))
        } else {
            pitch_step.wrapping_add(self.pitch_rate)
        };
        self.pitch_rate = clamp_symmetric(self.pitch_rate, self.limits[1]);
        self.yaw_rate = clamp_symmetric(self.yaw_rate, self.limits[0]);
        let yaw = i32::from(self.yaw)
            .wrapping_sub(i32::from((k.wrapping_mul(self.yaw_rate) >> 15) as i16));
        self.yaw = clamp_symmetric(yaw, self.limits[2]) as i16;
        let pitch = i32::from(self.pitch)
            .wrapping_sub(i32::from((k.wrapping_mul(self.pitch_rate) >> 15) as i16));
        self.pitch = if pitch > self.limits[3] {
            self.limits[3]
        } else if pitch < self.limits[4] {
            self.limits[4]
        } else {
            pitch
        } as i16;
    }

    /// `FUN_00422B40`: at a limit the rate pushing outward stops and the joint
    /// drives with 0x6000 either way.
    fn keep_within_limits(&mut self, frame_dt: u32) {
        let yaw = i32::from(self.yaw);
        let yaw_demand = if yaw == self.limits[2] {
            if self.yaw_rate < 0 {
                self.yaw_rate = 0;
            }
            0x6000
        } else if yaw == self.limits[2].wrapping_neg() {
            if self.yaw_rate > 0 {
                self.yaw_rate = 0;
            }
            -0x6000
        } else if self.yaw_rate >= 0 {
            0x6000
        } else {
            -0x6000
        };
        let pitch = i32::from(self.pitch);
        let pitch_demand = if pitch == self.limits[3] {
            if self.pitch_rate < 0 {
                self.pitch_rate = 0;
            }
            0x6000
        } else if pitch == self.limits[4] {
            if self.pitch_rate > 0 {
                self.pitch_rate = 0;
            }
            -0x6000
        } else if self.pitch_rate >= 0 {
            0x6000
        } else {
            -0x6000
        };
        self.drive(frame_dt, pitch_demand, yaw_demand);
    }
}

fn clamp_symmetric(value: i32, limit: i32) -> i32 {
    if limit < value {
        limit
    } else if value < limit.wrapping_neg() {
        limit.wrapping_neg()
    } else {
        value
    }
}

/// A gun joint: the joint, then its muzzle (`+18`), the draw count (`+20`),
/// range, fire timer, period, particle class and Q31 frame (`+34`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueenGun {
    pub joint: QueenJoint,
    pub muzzle_raw: [i16; 3],
    /// Set to 2 by each draw of the gun's selector, counted down by the tick.
    pub drawn: i32,
    pub range: i32,
    pub timer_us: u32,
    pub period_us: u32,
    pub particle_class: u8,
    /// Rows lateral, up, forward, as the body basis.
    pub frame_q31: [[i32; 3]; 3],
}

impl QueenGun {
    const fn new(limits: JointLimits, range: i32, period_us: u32, particle_class: u8) -> Self {
        Self {
            joint: QueenJoint::new(limits),
            muzzle_raw: [0; 3],
            drawn: 0,
            range,
            timer_us: 0,
            period_us,
            particle_class,
            frame_q31: [[0; 3]; 3],
        }
    }
}

/// `FUN_00421B40`'s zeroed 0xF8-byte record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueenRecord {
    /// `+00`: microseconds since death.
    pub dying_us: i32,
    /// `+08`: the direction of Sub-K word 8 (its pointer is `+04`).
    pub word8_state: i16,
    /// `+10`: the direction of Sub-K word 7 (its pointer is `+0C`).
    pub word7_state: i16,
    /// `+14`: Sub-K words 6 and 5, class 85 every 300 ms within 5,000.
    pub first_gun: QueenGun,
    /// `+6C`: Sub-K words 2 and 1, class 68 every 600 ms within 3,000.
    pub second_gun: QueenGun,
    /// `+C4`: Sub-K words 4 and 3, steered at half the first gun's demand.
    pub neck: QueenJoint,
    /// `+DC`, which `FUN_00421990` sets and `FUN_00421910` spends.
    pub weak_spot: bool,
    /// `+E0`, `+E8`, `+EC`: the target's position, distance and lateral offset.
    /// `FUN_004226A0` keeps the old position and distance when it finds none.
    pub target_position_raw: [i16; 3],
    pub target_distance: i32,
    pub target_lateral: i32,
    pub target: Option<u32>,
    /// `+F4`: the Type68 she holds.
    pub held: Option<u32>,
}

impl QueenRecord {
    pub const fn new() -> Self {
        Self {
            dying_us: 0,
            word8_state: 0,
            word7_state: 0,
            first_gun: QueenGun::new(FIRST_GUN_LIMITS, 5000, 300_000, 0x55),
            second_gun: QueenGun::new(SECOND_GUN_LIMITS, 3000, 600_000, 0x44),
            neck: QueenJoint::new(NECK_LIMITS),
            weak_spot: false,
            target_position_raw: [0; 3],
            target_distance: 0,
            target_lateral: 0,
            target: None,
            held: None,
        }
    }

    /// Both gun frames from the body basis and last visit's joints. The
    /// second gun hangs reversed, a quarter of its joint per link.
    fn rebuild_gun_frames(&mut self, basis: crate::common_mover::type9_attitude::Type9BodyBasis) {
        let rows = [basis.lateral, basis.up, basis.forward];
        let mut second = rows;
        rotate_frame(&mut second, Axis::Yaw, 0x8000);
        for _ in 0..4 {
            rotate_frame(
                &mut second,
                Axis::Yaw,
                i32::from(self.second_gun.joint.yaw) >> 2,
            );
            rotate_frame(
                &mut second,
                Axis::Pitch,
                i32::from(self.second_gun.joint.pitch) >> 2,
            );
        }
        rotate_frame(&mut second, Axis::Pitch, 0x1000);
        self.second_gun.frame_q31 = second;
        let mut first = rows;
        rotate_frame(&mut first, Axis::Yaw, i32::from(self.neck.yaw));
        rotate_frame(&mut first, Axis::Pitch, i32::from(self.neck.pitch));
        rotate_frame(&mut first, Axis::Yaw, i32::from(self.first_gun.joint.yaw));
        rotate_frame(
            &mut first,
            Axis::Pitch,
            i32::from(self.first_gun.joint.pitch),
        );
        self.first_gun.frame_q31 = first;
    }

    /// The first gun swings to the target's side once it and the neck point
    /// the same way as the target lies.
    fn first_gun_side(&self) -> i32 {
        let side = (i32::from(self.neck.yaw) + i32::from(self.first_gun.joint.yaw))
            .wrapping_mul(self.target_lateral);
        if side < 1 {
            0
        } else {
            self.target_lateral
        }
    }

    /// The second gun swings back across when it points away from the target.
    fn second_gun_side(&self) -> i32 {
        if self
            .target_lateral
            .wrapping_mul(i32::from(self.second_gun.joint.yaw))
            < 0
        {
            self.target_lateral.wrapping_neg()
        } else {
            0
        }
    }

    /// Words 8 and 7 swing between their limits; word 8 waits until the
    /// target comes within 2,500. Then both draw counts run down.
    fn cycle_words(&mut self, words: &mut [i16; 8], dt: u32) {
        if self.word8_state >= 1 {
            words[7] = words[7].wrapping_add((dt >> 4) as i16);
            if words[7] > 0 {
                self.word8_state = 0;
            }
        } else if self.word8_state == 0 {
            if self.target_distance < 2500 {
                self.word8_state = -1;
            }
        } else {
            words[7] = words[7].wrapping_sub((dt >> 5) as i16);
            if words[7] < -0x3000 {
                self.word8_state = 1;
            }
        }
        if self.word7_state < 1 {
            words[6] = words[6].wrapping_sub((dt >> 7) as i16);
            if words[6] < 0 {
                words[6] = 0;
                self.word7_state = 1;
            }
        } else {
            words[6] = words[6].wrapping_add((dt >> 6) as i16);
            if words[6] > 3000 {
                self.word7_state = -1;
            }
        }
        if self.second_gun.drawn != 0 {
            self.second_gun.drawn -= 1;
        }
        if self.first_gun.drawn != 0 {
            self.first_gun.drawn -= 1;
        }
    }

    /// The tick's last writes: the joints into Sub-K words 1..=6.
    fn write_model_words(&self, words: &mut [i16; 8]) {
        words[5] = self.first_gun.joint.yaw.wrapping_neg();
        words[4] = self.first_gun.joint.pitch;
        words[3] = self.neck.yaw.wrapping_neg();
        words[2] = self.neck.pitch;
        words[1] = (i32::from(self.second_gun.joint.yaw).wrapping_neg() >> 2) as i16;
        words[0] = (i32::from(self.second_gun.joint.pitch).wrapping_neg() >> 2) as i16;
    }
}

impl Default for QueenRecord {
    fn default() -> Self {
        Self::new()
    }
}

/// The Queen's Sub-K storage: the eight model words and the record pointer.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QueenSubK {
    /// Selectors 1..=8.
    pub words: [i16; 8],
    pub record: Option<Box<QueenRecord>>,
}

impl QueenSubK {
    pub fn model_word(&self, selector: usize) -> Option<i16> {
        selector
            .checked_sub(1)
            .and_then(|index| self.words.get(index))
            .copied()
    }
}

/// What a visit asks of its scheduler.
#[derive(Debug, Default)]
pub struct QueenTick {
    /// The Type68 `FUN_004225C0` spawned, which this frame's walk reaches.
    pub held_owner: Option<crate::entity::Class0ActorOwner>,
    /// `FUN_00456F20`: thirty seconds after her death, the move to the
    /// closing scene.
    pub closing_scene: Option<ClosingSceneMove>,
}

/// `FUN_00456F20`'s logical world, `0x25`: overlay 49, the closing scene.
pub const CLOSING_SCENE_LOGICAL_WORLD: u32 = 0x25;
/// `FUN_00456F20`'s session phase `+296` for it.
pub const CLOSING_SCENE_PHASE: u8 = 3;
/// The cheats `FUN_00456F20` switches on, in call order: Weapons Menu, then
/// Skip to World.
pub const CLOSING_SCENE_CHEATS: [u32; 2] = [1, 0];
/// `FUN_00450FC0` plays this through `FUN_0042A860` for each cheat it
/// switches on outside a network game.
pub const CHEAT_ENABLE_SOUND_ID: usize = 0x57;

/// `FUN_00456F20`, from the dying visit: session `+290` leaves the world,
/// `FUN_00456750` requests the full-frame flash, cheats 1 then 0 are
/// switched on, and the session moves to logical world `0x25` in phase 3.
/// The Queen lives only in campaign world 47, so `FUN_00450FC0` always takes
/// its single-player branch; its two sounds resolve here, in the visit's
/// RNG order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosingSceneMove {
    pub cheat_sounds: [Result<
        v2k_render::sound::ResolvedSoundPlayback,
        v2k_formats::anim_sound::SoundResolutionError,
    >; 2],
}

/// What a class0 visit asks of the playing world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class0WorldCall {
    /// `FUN_00410C10` on another body, through that body's own death owner.
    Death(u32),
    /// `FUN_004566E0`: the static cells, then the live list.
    Radial {
        origin_raw: [i16; 3],
        template: crate::radial_damage::RadialDamageTemplate,
    },
}

/// The playing world's [`Class0WorldCall`]s.
pub type GenericDeath<'a> = dyn FnMut(
        &mut EntityManager,
        &mut crate::resource_cache::ResourceCache,
        &mut WorldFx,
        Class0WorldCall,
    ) -> Result<(), Class0ActorError>
    + 'a;

pub struct QueenFrame<'a, 'h> {
    pub resources: &'a mut crate::resource_cache::ResourceCache,
    pub world_fx: &'a mut WorldFx,
    /// `DAT_004D04E4`, the frame's microseconds.
    pub frame_dt: u32,
    /// The callback's microseconds since the last.
    pub dt: u32,
    pub retail_tick: u32,
    /// The dying visit's sweep; without it a victim stops her visit.
    pub generic_death: Option<&'a mut GenericDeath<'h>>,
}

fn known_state(manager: &EntityManager, id: u32) -> Result<u32, Class0ActorError> {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Class0ActorError::Allocation)?;
    match entity.collision.state_flags_at_0x08.masked(u32::MAX) {
        RetailRuntimeValue::Known(state) => Ok(state),
        RetailRuntimeValue::Unresolved => Err(Class0ActorError::Runtime("Queen state word")),
    }
}

/// `FUN_004222C0`: the Primary's tick, living or dying.
pub(crate) fn tick(
    manager: &mut EntityManager,
    id: u32,
    frame: &mut QueenFrame<'_, '_>,
) -> Result<QueenTick, Class0ActorError> {
    let entity = manager.entity_mut(id).ok_or(Class0ActorError::Allocation)?;
    entity.set_velocity_raw([0; 3]);
    let Some(mut record) = entity.queen.record.as_deref().copied() else {
        // The first visit allocates the record and returns.
        entity.queen.record = Some(Box::new(QueenRecord::new()));
        return Ok(QueenTick::default());
    };
    let mut words = entity.queen.words;
    let state = known_state(manager, id)?;
    let mut tick = QueenTick::default();
    if state & DYING_STATE_BIT == 0 && state != 0 {
        choose_target(manager, id, &mut record)?;
        let basis = match manager
            .iter_all()
            .find(|entity| entity.id == id)
            .map(|entity| entity.physical_body_basis_q31())
        {
            Some(RetailRuntimeValue::Known(basis)) => basis,
            _ => return Err(Class0ActorError::Runtime("Queen body basis")),
        };
        record.rebuild_gun_frames(basis);
        let side = record.first_gun_side();
        shoot(manager, id, &mut record, Gun::First, side, frame)?;
        let side = record.second_gun_side();
        shoot(manager, id, &mut record, Gun::Second, side, frame)?;
        record.cycle_words(&mut words, frame.dt);
        tick.held_owner = keep_held_body(manager, &mut record, frame)?;
    } else {
        tick.closing_scene = die(manager, id, &mut record, &mut words, frame)?;
    }
    record.write_model_words(&mut words);
    let entity = manager.entity_mut(id).ok_or(Class0ActorError::Allocation)?;
    entity.queen.words = words;
    entity.queen.record = Some(Box::new(record));
    Ok(tick)
}

#[derive(Clone, Copy)]
enum Axis {
    /// `FUN_00457C30`: lateral and forward.
    Yaw,
    /// `FUN_00457A90`: up and forward.
    Pitch,
}

fn rotate_frame(rows: &mut [[i32; 3]; 3], axis: Axis, angle: i32) {
    let [lateral, up, forward] = rows;
    match axis {
        Axis::Yaw => rotate(lateral, forward, angle),
        Axis::Pitch => rotate(up, forward, angle),
    }
}

/// `FUN_004226A0`: of the live bodies with capability bit 0 or 2 and pair
/// state, the best scores 20,000 (the player 30,000) less its distance.
fn choose_target(
    manager: &EntityManager,
    id: u32,
    record: &mut QueenRecord,
) -> Result<(), Class0ActorError> {
    let queen = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Class0ActorError::Allocation)?;
    let origin = queen.position_raw();
    let RetailRuntimeValue::Known(basis) = queen.physical_body_basis_q31() else {
        return Err(Class0ActorError::Runtime("Queen body basis"));
    };
    record.target = None;
    record.target_lateral = 0;
    let mut best = 0;
    for candidate_id in manager.retail_live_order_ids() {
        let candidate = manager
            .iter_all()
            .find(|entity| entity.id == candidate_id)
            .ok_or(Class0ActorError::Runtime("live list"))?;
        if candidate.capability_flags & 5 == 0 {
            continue;
        }
        let RetailRuntimeValue::Known(pair) = candidate
            .collision
            .state_flags_at_0x08
            .masked(PAIR_COLLISION_ENABLED_STATE_BIT)
        else {
            return Err(Class0ActorError::Runtime("target pair state"));
        };
        if pair == 0 {
            continue;
        }
        let (offset, mut distance) = displacement(origin, candidate.position_raw());
        // +F0 holds the best so far, so this never meets a candidate twice.
        if record.target == Some(candidate_id) {
            distance = distance.wrapping_mul(6) >> 2;
        }
        if distance >= 20_000 {
            continue;
        }
        let reach = if candidate.capability_flags & 1 != 0 {
            30_000
        } else {
            20_000
        };
        let score = reach - distance;
        if best < score {
            record.target = Some(candidate_id);
            record.target_distance = distance;
            record.target_position_raw = candidate.position_raw();
            record.target_lateral = (0..3).fold(0_i32, |sum, axis| {
                sum.wrapping_add(q31(i32::from(offset[axis]), basis.lateral[axis]))
            });
            best = score;
        }
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Gun {
    /// `+14`, with the neck at half its demand.
    First,
    /// `+6C`.
    Second,
}

fn gun(record: &mut QueenRecord, which: Gun) -> &mut QueenGun {
    match which {
        Gun::First => &mut record.first_gun,
        Gun::Second => &mut record.second_gun,
    }
}

/// `FUN_00421E00` with its `FUN_00440A60` birth and the timer's fresh draw.
fn shoot(
    manager: &EntityManager,
    id: u32,
    record: &mut QueenRecord,
    which: Gun,
    side: i32,
    frame: &mut QueenFrame<'_, '_>,
) -> Result<(), Class0ActorError> {
    // 421E00 reads the target's velocity afresh; its position is 6A0's copy.
    let target_velocity = record.target.and_then(|target| {
        manager
            .iter_all()
            .find(|entity| entity.id == target)
            .map(|entity| entity.velocity_raw())
    });
    let world_fx = &mut *frame.world_fx;
    let shot = aim(
        record,
        which,
        side,
        target_velocity,
        frame.frame_dt,
        &mut || world_fx.next_shared_retail_random_u16(),
    )?;
    let Some((source_class, position_raw, velocity_raw)) = shot else {
        return Ok(());
    };
    emit(
        frame,
        DescriptorParticleRequest {
            source_class,
            position_raw,
            velocity_raw,
            owner: Some(ParticleOwnerAtBirth {
                entity_id: id,
                entity_type: ENTITY_TYPE as u8,
            }),
            suppresses_impact_damage: false,
        },
    )?;
    let random = i32::from(frame.world_fx.next_shared_retail_random_u16()) << 14;
    let gun_state = gun(record, which);
    gun_state.timer_us = q31(random, gun_state.period_us as i32) as u32;
    Ok(())
}

/// `FUN_00421E00` up to its particle: lead the target by its velocity over
/// the time to reach it, steer the gun (the first with the neck at half),
/// and once its period has passed return the shot (class, muzzle, velocity).
fn aim(
    record: &mut QueenRecord,
    which: Gun,
    side: i32,
    target_velocity: Option<[i16; 3]>,
    frame_dt: u32,
    random: &mut dyn FnMut() -> u16,
) -> Result<Option<(u8, [i16; 3], [i16; 3])>, Class0ActorError> {
    if record.target_distance == 0 {
        return Ok(None);
    }
    let target_position = record.target_position_raw;
    let gun_state = *gun(record, which);
    // Without a target or a drawn gun, retail compares the Queen's handle
    // (its slot index sits in the high word) with 5,000: the gun cannot fire.
    let mut reach = None;
    let (mut pitch, mut yaw) = (
        i32::from(gun_state.joint.pitch),
        i32::from(gun_state.joint.yaw),
    );
    let mut lead = [0_i16; 3];
    if let (Some(velocity), true) = (target_velocity, gun_state.drawn != 0) {
        let (mut offset, distance) = displacement(gun_state.muzzle_raw, target_position);
        let time = distance.wrapping_shl(10) / gun_state.range;
        let fraction = if time < 0x1000 {
            time.wrapping_shl(17)
        } else {
            i32::MAX
        };
        let scaled = |component: i16| q31(i32::from(component), fraction).wrapping_shl(4) as i16;
        offset[0] = offset[0].wrapping_add(scaled(velocity[0]));
        offset[2] = offset[2].wrapping_add(scaled(velocity[2]));
        offset[1] = offset[1]
            .wrapping_add(scaled(velocity[1]))
            .wrapping_add(q31(time.wrapping_mul(time), 0x18_0000) as i16);
        let magnitude = i32::from(length(offset) as i16);
        let dot = |row: [i32; 3]| {
            (0..3).fold(0_i32, |sum, axis| {
                sum.wrapping_add(q31(i32::from(offset[axis]), row[axis]))
            })
        };
        yaw = asin(ratio(dot(gun_state.frame_q31[0]), magnitude));
        if dot(gun_state.frame_q31[2]) < 0 {
            yaw = if yaw < 1 { -0x2000 } else { yaw + 0x2000 };
        }
        pitch = asin(ratio(dot(gun_state.frame_q31[1]), magnitude));
        reach = Some(magnitude);
        lead = offset;
    }
    if side != 0 {
        yaw = if side < 1 { -0x4000 } else { 0x4000 };
    }
    gun(record, which).joint.drive(frame_dt, pitch, yaw);
    if which == Gun::First {
        record.neck.drive(frame_dt, pitch / 2, yaw / 2);
    }
    let gun_state = gun(record, which);
    gun_state.timer_us = gun_state.timer_us.wrapping_add(frame_dt);
    let Some(magnitude) = reach else {
        return Ok(None);
    };
    if gun_state.timer_us <= gun_state.period_us || magnitude >= 5000 {
        return Ok(None);
    }
    // `FUN_00421D40`'s aiming line waits on DAT_004DB0B8, which nothing writes.
    let velocity = if pitch.wrapping_abs() < 0x400 && yaw.wrapping_abs() < 0x400 && 0x40 < magnitude
    {
        lead.map(|component| {
            (i32::from(component).wrapping_mul(gun_state.range) / magnitude) as i16
        })
    } else {
        if frame_dt == 0 {
            return Err(Class0ActorError::Runtime("Queen frame time"));
        }
        let chance = (gun_state.period_us.wrapping_mul(8) / frame_dt).max(1);
        if u32::from(random()) % chance != 0 {
            return Ok(None);
        }
        gun_state.frame_q31[2].map(|component| q31(component, gun_state.range) as i16)
    };
    Ok(Some((
        gun_state.particle_class,
        gun_state.muzzle_raw,
        velocity,
    )))
}

/// `FUN_00440A60` for one of her particles.
fn emit(
    frame: &mut QueenFrame<'_, '_>,
    request: DescriptorParticleRequest,
) -> Result<(), Class0ActorError> {
    let environment = crate::world_fx::ParticleEnvironment::Terrain(
        crate::world_fx::TerrainCollisionContext::from_current_level_cache(&*frame.resources)
            .ok_or(Class0ActorError::Runtime("particle terrain context"))?,
    );
    frame
        .world_fx
        .materialize_descriptor_particle_request(request, environment, frame.retail_tick);
    Ok(())
}

/// `FUN_004225C0`: the Type68 she holds sits on the first gun's muzzle,
/// with state `0x40000000` while that gun is drawn. A missing one is
/// respawned: a zero-record `00438080` Type68 that keeps neither its
/// callback, its motion nor its pair state, with model 286 and capability 8.
fn keep_held_body(
    manager: &mut EntityManager,
    record: &mut QueenRecord,
    frame: &mut QueenFrame<'_, '_>,
) -> Result<Option<crate::entity::Class0ActorOwner>, Class0ActorError> {
    let present = record
        .held
        .is_some_and(|held| manager.iter_all().any(|entity| entity.id == held));
    let mut owner = None;
    if !present {
        let resources = &*frame.resources;
        let extent = |model: usize| resources.global_model(model).map(|model| model.radius);
        let waves_enabled = resources
            .level_desc()
            .and_then(|level| level.raw_u32(0x84))
            .ok_or(Class0ActorError::Runtime("level descriptor"))?
            != 0;
        let spawned = manager.append_native_zero_record_type68(
            crate::entity::NativeType68ZeroRecordConstruction {
                resources: crate::entity::EntityConstructionResources {
                    terrain: resources.level_terrain(),
                    terrain_objects: resources.terrain_objects(),
                    model_extent_raw: Some(&extent),
                },
                retail_tick: frame.retail_tick,
                waves_enabled,
            },
            frame.world_fx,
        )?;
        let held = spawned.entity_id();
        record.held = Some(held);
        manager.set_native_class0_model_slot0(held, HELD_BODY_MODEL)?;
        let entity = manager
            .entity_mut(held)
            .ok_or(Class0ActorError::Allocation)?;
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(!0xfff9_77ff, 0);
        entity.capability_flags = entity.capability_flags & !0x1000 | 8;
        owner = Some(spawned);
    }
    let held = record.held.ok_or(Class0ActorError::Allocation)?;
    let entity = manager
        .entity_mut(held)
        .ok_or(Class0ActorError::Allocation)?;
    entity.set_position_raw(record.first_gun.muzzle_raw);
    let drawn = if record.first_gun.drawn != 0 {
        0x4000_0000
    } else {
        0
    };
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x4000_0000, drawn);
    Ok(owner)
}

/// `FUN_00422840`: thirty seconds of dying, then `FUN_00456F20`. Returns
/// that move when this visit makes it; retail never asks whether the world
/// was lost.
fn die(
    manager: &mut EntityManager,
    id: u32,
    record: &mut QueenRecord,
    words: &mut [i16; 8],
    frame: &mut QueenFrame<'_, '_>,
) -> Result<Option<ClosingSceneMove>, Class0ActorError> {
    record.target_distance = 0;
    record.target_lateral = 0;
    if record.dying_us > DEATH_END_US {
        let pool =
            v2k_formats::anim_sound::SoundPool::from_tables(&frame.resources.global_sound_tables());
        let world_fx = &mut *frame.world_fx;
        return Ok(Some(ClosingSceneMove {
            cheat_sounds: CLOSING_SCENE_CHEATS.map(|_| {
                v2k_render::sound::resolve_sound_playback(
                    &pool,
                    v2k_render::sound::SoundPlaybackRequest::centered(CHEAT_ENABLE_SOUND_ID),
                    &mut || world_fx.next_shared_retail_random_u16(),
                )
            }),
        }));
    }
    // Every visit runs 10C10 on each other living body with capability 8,
    // reading each successor after the death: a tail a death appends is
    // visited too.
    let mut current = manager.retail_live_order_ids().next();
    while let Some(other) = current {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == other)
            .ok_or(Class0ActorError::Runtime("live list"))?;
        let word = entity.collision.state_flags_at_0x08;
        let victim = other != id
            && entity.capability_flags & 8 != 0
            && match (word.masked(DYING_STATE_BIT), word.masked(u32::MAX)) {
                (RetailRuntimeValue::Known(0), RetailRuntimeValue::Known(state)) => state != 0,
                // A partly known word is nonzero through any known set bit.
                (RetailRuntimeValue::Known(0), _) if word.known_value_bits() != 0 => true,
                (RetailRuntimeValue::Known(_), _) => false,
                _ => return Err(Class0ActorError::Runtime("Queen death sweep state")),
            };
        if victim {
            let death = frame
                .generic_death
                .as_deref_mut()
                .ok_or(Class0ActorError::Runtime(
                    "Queen death sweep: no FUN_00410C10 owner outside play",
                ))?;
            death(
                manager,
                &mut *frame.resources,
                &mut *frame.world_fx,
                Class0WorldCall::Death(other),
            )?;
        }
        current = manager
            .retail_live_successor_id(other)
            .map_err(|()| Class0ActorError::Runtime("Queen death sweep: victim removed"))?;
    }
    let dt = frame.dt;
    record.dying_us = record.dying_us.wrapping_add(dt as i32);
    let step = (dt >> 8) as i16;
    let position = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Class0ActorError::Allocation)?
        .position_raw();
    if record.dying_us > 14_999_999 {
        if frame.world_fx.next_shared_retail_random_u16() % 12 == 0 {
            frame
                .world_fx
                .queue_networked_positional_sound_raw(99, position);
        }
        if words[6] != 0 {
            words[6] = words[6].wrapping_sub(step);
            if words[6] < 0 {
                words[6] = 10;
            }
        }
        // The body sinks: its up axis loses height in proportion to time.
        let entity = manager.entity_mut(id).ok_or(Class0ActorError::Allocation)?;
        let RetailRuntimeValue::Known(mut basis) = entity.physical_body_basis_q31() else {
            return Err(Class0ActorError::Runtime("Queen body basis"));
        };
        if basis.up[1] > 0x800_0000 {
            basis.up[1] = basis.up[1].wrapping_sub(q31(basis.up[1], (dt as i32).wrapping_shl(7)));
            entity.physical_body_basis_q31 = RetailRuntimeValue::Known(basis);
        }
    } else {
        if record.dying_us < 10_000_000 {
            record.first_gun.joint.keep_within_limits(frame.frame_dt);
            record.neck.keep_within_limits(frame.frame_dt);
            record.second_gun.joint.keep_within_limits(frame.frame_dt);
            if frame.world_fx.next_shared_retail_random_u16() & 7 == 0 {
                frame
                    .world_fx
                    .queue_networked_positional_sound_raw(0x4b, position);
            }
        } else {
            let first = record.first_gun.joint;
            record.first_gun.joint.drive(
                frame.frame_dt,
                i32::from(first.pitch),
                first.yaw_rate.wrapping_neg(),
            );
            let neck_rate = record.neck.yaw_rate;
            record
                .neck
                .drive(frame.frame_dt, -0x1000, neck_rate.wrapping_neg());
            let second_rate = record.second_gun.joint.yaw_rate;
            record
                .second_gun
                .joint
                .drive(frame.frame_dt, -0x1000, second_rate.wrapping_neg());
            if frame.world_fx.next_shared_retail_random_u16() % 12 == 0 {
                frame
                    .world_fx
                    .queue_networked_positional_sound_raw(99, position);
            }
        }
        if record.word7_state < 1 {
            words[6] = words[6].wrapping_sub(step);
            if words[6] < 1000 {
                words[6] = 1000;
                record.word7_state = 1;
            }
        } else {
            words[6] = words[6].wrapping_add((dt >> 6) as i16);
            if words[6] > 6000 {
                record.word7_state = -1;
            }
        }
    }
    if dt as i32 <= i32::from(frame.world_fx.next_shared_retail_random_u16()) {
        return Ok(None);
    }
    // Four class-5 sparks from the first gun's muzzle, their velocity a
    // random walk from its forward axis.
    let forward = record.first_gun.frame_q31[2];
    let mut velocity = forward.map(|component| i16::from((component >> 24) as i8));
    for _ in 0..4 {
        for component in &mut velocity {
            let random = frame.world_fx.next_shared_retail_random_u16();
            *component = component.wrapping_add(0x80 - ((random >> 8) & 0xff) as i16);
        }
        emit(
            frame,
            DescriptorParticleRequest {
                source_class: 5,
                position_raw: record.first_gun.muzzle_raw,
                velocity_raw: velocity,
                owner: Some(ParticleOwnerAtBirth {
                    entity_id: id,
                    entity_type: ENTITY_TYPE as u8,
                }),
                suppresses_impact_damage: false,
            },
        )?;
    }
    Ok(None)
}

/// `FUN_00421990`, class67's style `+28` hook: the hit stops her, and a hit
/// within 256 of the drawn first gun's muzzle, coming down its up axis
/// within 60 degrees, opens the weak spot.
pub(crate) fn hit(sub_k: &mut QueenSubK, velocity: &mut [i16; 3], hit_position_raw: [i16; 3]) {
    *velocity = [0; 3];
    let Some(record) = sub_k.record.as_deref_mut() else {
        return;
    };
    if record.first_gun.drawn == 0 {
        return;
    }
    let (offset, distance) = displacement(hit_position_raw, record.first_gun.muzzle_raw);
    if !(0 < distance && distance < 0x100) {
        return;
    }
    let magnitude = i32::from(length(offset) as i16);
    let normal = if magnitude == 0 {
        [0x1000, 0, 0]
    } else {
        offset.map(|component| i32::from(((i32::from(component) << 12) / magnitude) as i16))
    };
    let up = record.first_gun.frame_q31[1];
    let dot = (0..3).fold(0_i32, |sum, axis| {
        sum.wrapping_add((up[axis] >> 19).wrapping_mul(normal[axis]))
    });
    if 0x80_0000 < (dot as u32 & 0xffff_f000) as i32 {
        record.weak_spot = true;
    }
}

/// `FUN_00421910`, Type110's instance damage modifier: with a record, only
/// the player's hit through an open weak spot passes, and it closes it.
pub(crate) fn weak_spot_gate(sub_k: &mut QueenSubK, from_player: bool, damage: i32) -> i32 {
    let Some(record) = sub_k.record.as_deref_mut() else {
        return damage;
    };
    if record.weak_spot && from_player {
        record.weak_spot = false;
        return damage;
    }
    0
}

/// `FUN_00421AE0` from `FUN_0040A9F0`'s Queen selectors: a drawn muzzle.
pub fn stamp_drawn_muzzle(sub_k: &mut QueenSubK, selector: u16, position_raw: [i16; 3]) {
    let Some(record) = sub_k.record.as_deref_mut() else {
        return;
    };
    let gun = if selector == 0 {
        &mut record.second_gun
    } else {
        &mut record.first_gun
    };
    gun.muzzle_raw = position_raw;
    gun.drawn = 2;
}

/// The two selectors as a Sub-E pair: `FUN_00421AE0` reads its slots
/// through the same `FUN_00424FA0` as an emitter, 0x1C for selector 0 and
/// 0x1B for selector 1, and nothing else of the descriptor.
pub fn draw_selector_descriptor() -> v2k_formats::collision::ProjectileEmitterDescriptor {
    v2k_formats::collision::ProjectileEmitterDescriptor {
        projectile_method: 0,
        random_interval_us: 0,
        spread_raw: 0,
        aim_threshold_raw: 0,
        speed_override_raw: 0,
        target_axis_tolerance_raw: 0,
        sound_id: 0,
        raw_word_at_0x12: SECOND_GUN_SLOT,
        alternate_emitter_raw: FIRST_GUN_SLOT,
        stochastic_gate_mode: 0,
        auxiliary_command: 0,
        variable_bindings: [0; 4],
    }
}
