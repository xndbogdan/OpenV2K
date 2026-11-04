//! Class 55, Megablast: the Type65 seedpod (`FUN_00407800`, `FUN_004078C0`).
//!
//! A pickup of operation `0x39` builds one seedpod in mode 0 at the player;
//! the deploy `FUN_00444400` builds three in mode 1. Each visit puts the pod
//! on its owner (`+60`) with zero velocity and spins it. Mode 0 charges Sub-K
//! variable 1 and dies once it would pass `0xFFFF`. Mode 1 drains it, asking
//! for the one-frame terrain flash on a random draw, and blasts when it would
//! go below zero: the owner is taken out of checked damage, the radial runs at
//! the pod over the owner's type and handle, the screen flashes and the
//! owner's admission returns. Both ends return tag `0x9C00`, whose style slot
//! `+04` (`FUN_0040CEF0`) is the pod's generic death.

use crate::damage::DamagePacket;
use crate::radial_damage::RadialDamageTemplate;

pub const ENTITY_TYPE: u32 = 65;
pub const BEHAVIOR_CLASS: u32 = 55;
/// `FUN_00445A90` and `FUN_00444400`: controller `+195` stays at most 4.
pub const MAX_COUNT: u8 = 4;
/// `FUN_00444400`'s cue, at a random rate up to an eighth higher.
pub const DEPLOY_SOUND_ID: u16 = 0x4F;
/// `FUN_00456900(0xC9, 0)` after a pickup's or a deploy's births.
pub const TEXT_ID: usize = 0xC9;
/// The pickup's `FUN_0044F450` cue at the player (`local_18 = 5`).
pub const PICKUP_SOUND_ID: u16 = 5;

/// `FUN_004078C0`'s radial: radii `0x800`/`0x1000`, impulse 2000 and packet
/// `[4, 3]` / `[25000, 25000]` over the owner's type and handle.
pub fn radial_template(owner_type: u32, owner_id: u32) -> RadialDamageTemplate {
    RadialDamageTemplate {
        inner_radius_raw: 0x800,
        outer_radius_raw: 0x1000,
        impulse_raw: 2000,
        packet: DamagePacket {
            channels: [4, 3],
            amounts_raw: [25_000, 25_000],
        },
        trailing_raw: [owner_type as i32, owner_id as i32],
    }
}

/// `FUN_00407800`'s Primary: Sub-K variable 1 (the charge) and variable 2
/// (the birth record's `+88` word, the mode).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MegablastTaskState {
    pub charge: u16,
    pub mode: i16,
}

impl MegablastTaskState {
    /// Variable 2 takes the word; variable 1 starts full for any nonzero word.
    pub fn new(word_0x88: i32) -> Self {
        Self {
            charge: if word_0x88 != 0 { 0xFFFF } else { 0 },
            mode: word_0x88 as i16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MegablastStep {
    /// Returned 0.
    Continue,
    /// Mode 1's draw fell under the threshold: `FUN_00456740` (session
    /// `+293`), then 0.
    TerrainFlash,
    /// Mode 1 emptied: variable 1 is 0, the blast runs, then tag `0x9C00`.
    Detonate,
    /// Any other mode filled: variable 1 is `0xFFFF`, then tag `0x9C00`.
    Charged,
}

impl MegablastTaskState {
    /// The charge arithmetic of `FUN_004078C0`; `next_random` is
    /// `FUN_00457930`, drawn only on mode 1's surviving drain.
    pub fn step(&mut self, dt: u32, next_random: &mut dyn FnMut() -> u32) -> MegablastStep {
        let charge = u32::from(self.charge);
        if self.mode != 1 {
            let step = dt >> 3;
            if charge + step > 0xFFFF {
                self.charge = 0xFFFF;
                return MegablastStep::Charged;
            }
            self.charge = (charge + step) as u16;
            return MegablastStep::Continue;
        }
        let rate = charge.clamp(0x5000, 0xD000);
        let step = ((dt >> 12).wrapping_mul(rate)) >> 7;
        if (charge.wrapping_sub(step) as i32) < 0 {
            self.charge = 0;
            return MegablastStep::Detonate;
        }
        self.charge = charge.wrapping_sub(step) as u16;
        let headroom = 0xFFFF - u32::from(self.charge);
        if next_random() & 0xFFFF < headroom >> 2 {
            MegablastStep::TerrainFlash
        } else {
            MegablastStep::Continue
        }
    }
}

/// Controller `+195` after operation `0x39`: a byte add, then at most 4.
pub fn add_count(count: u8, amount: i32) -> u8 {
    count.wrapping_add(amount as u8).min(MAX_COUNT)
}

/// `FUN_00444400`'s three births: the player's attitude, then pitch
/// `+0x4000`, then roll `+0x4000` as well.
pub fn deploy_attitudes(rotation_raw: [i16; 3]) -> [[i16; 3]; 3] {
    let [heading, pitch, roll] = rotation_raw;
    let pitched = pitch.wrapping_add(0x4000);
    [
        rotation_raw,
        [heading, pitched, roll],
        [heading, pitched, roll.wrapping_add(0x4000)],
    ]
}

/// `FUN_00444400`'s cue rate: `((random & 0xFFFF) >> 3) + 0x10000`.
pub fn deploy_sound_rate(random: u32) -> u32 {
    ((random & 0xFFFF) >> 3) + 0x10000
}

/// The world a pickup's or a deploy's births enter.
pub struct MegablastWorld<'a> {
    pub entities: &'a mut crate::entity::EntityManager,
    pub scheduler: &'a mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
    pub terrain: &'a v2k_formats::terrain::TerrainGrid,
    /// Descriptor `+84`, the constructor's wave clock.
    pub waves_enabled: bool,
    pub world_fx: &'a mut crate::world_fx::WorldFx,
    pub notifications: &'a mut crate::gameplay_notifications::GameplayNotifications,
    pub retail_tick: u32,
    /// The player's `+A2/+A4/+A6`: heading and the craft's body angles.
    pub player_rotation_raw: [i16; 3],
}

impl MegablastWorld<'_> {
    fn birth(&mut self, rotation_raw: [i16; 3], mode: i32) -> Result<u32, String> {
        let player = self.entities.player().ok_or("no player")?;
        let birth = crate::entity::MegablastBirth {
            owner_id: player.id,
            position_raw: player.position_raw(),
            rotation_raw,
            velocity_raw: player.velocity_raw(),
            mode,
        };
        let owner = self
            .entities
            .append_megablast(
                birth,
                crate::entity::EntityConstructionResources::new(Some(self.terrain), None),
                self.retail_tick,
                self.waves_enabled,
                self.world_fx,
            )
            .map_err(|error| format!("Type65 construction: {error:?}"))?;
        self.scheduler.register_class0_actor(owner);
        Ok(owner.entity_id())
    }

    /// The rest of `FUN_00445A90`'s operation `0x39` once the count is
    /// raised: one Type65 in mode 0, then sound 5 at the player and text
    /// `0xC9`. A refused birth refuses the pickup; the count stays raised.
    pub fn pickup(&mut self) -> Result<u32, String> {
        let id = self.birth(self.player_rotation_raw, 0)?;
        if let Some(position) = self.entities.player().map(|player| player.position_raw()) {
            self.world_fx
                .queue_fixed_positional_sound_raw(PICKUP_SOUND_ID, position);
        }
        self.notifications.queue_megablast(self.retail_tick as i32);
        Ok(id)
    }

    /// `FUN_00444400` on a press: with a count, one is taken, sound `0x4F`
    /// plays (and is sent) at the player at a random rate, three Type65 are
    /// born in mode 1 until one is refused, and text `0xC9` follows.
    pub fn deploy(&mut self, count: &mut u8) -> Result<Vec<u32>, String> {
        if *count == 0 {
            return Ok(Vec::new());
        }
        *count -= 1;
        let position = self
            .entities
            .player()
            .map(|player| player.position_raw())
            .ok_or("no player")?;
        let rate = deploy_sound_rate(u32::from(self.world_fx.next_shared_retail_random_u16()));
        self.world_fx
            .queue_networked_positional_sound_raw_at_rate(DEPLOY_SOUND_ID, position, rate);
        let mut ids = Vec::new();
        let mut refused = None;
        for rotation in deploy_attitudes(self.player_rotation_raw) {
            match self.birth(rotation, 1) {
                Ok(id) => ids.push(id),
                Err(error) => {
                    refused = Some(error);
                    break;
                }
            }
        }
        self.notifications.queue_megablast(self.retail_tick as i32);
        match refused {
            Some(error) => Err(error),
            None => Ok(ids),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_zero_charges_by_an_eighth_and_dies_full() {
        let mut task = MegablastTaskState::new(0);
        assert_eq!(task, MegablastTaskState { charge: 0, mode: 0 });
        let mut never = || -> u32 { panic!("mode 0 draws nothing") };
        for tick in 1..=26 {
            assert_eq!(task.step(20_000, &mut never), MegablastStep::Continue);
            assert_eq!(u32::from(task.charge), tick * 2500);
        }
        assert_eq!(task.step(20_000, &mut never), MegablastStep::Charged);
        assert_eq!(task.charge, 0xFFFF);
    }

    #[test]
    fn mode_one_drains_by_the_clamped_charge_and_flashes_on_the_draw() {
        let mut task = MegablastTaskState::new(1);
        assert_eq!(task.charge, 0xFFFF);
        // (20000 >> 12) * 0xD000 >> 7 = 1664: the top of the clamp.
        let mut draws = vec![0xFFFF_u32, 0];
        let mut next = || draws.remove(0);
        assert_eq!(task.step(20_000, &mut next), MegablastStep::Continue);
        assert_eq!(task.charge, 0xFFFF - 1664);
        // A quarter of 1664 is 416: a zero draw is under it.
        assert_eq!(task.step(20_000, &mut next), MegablastStep::TerrainFlash);
        assert_eq!(task.charge, 0xFFFF - 2 * 1664);
    }

    #[test]
    fn mode_one_detonates_at_zero_after_the_floor_rate() {
        let mut task = MegablastTaskState {
            charge: 0x200,
            mode: 1,
        };
        // Below 0x5000 the rate is 0x5000: 4 * 0x5000 >> 7 = 640 > 0x200.
        let mut never = || -> u32 { panic!("the blast draws nothing") };
        assert_eq!(task.step(20_000, &mut never), MegablastStep::Detonate);
        assert_eq!(task.charge, 0);
    }

    #[test]
    fn retail_lab_blast_comes_seventy_ticks_after_the_deploy() {
        // Level 1 lab: births at tick 1010, radials at 1080 (20-ms frames).
        let mut task = MegablastTaskState::new(1);
        let mut ticks = 0;
        let mut quiet = || 0xFFFF;
        loop {
            ticks += 1;
            if task.step(20_000, &mut quiet) == MegablastStep::Detonate {
                break;
            }
        }
        assert_eq!(ticks, 70);
    }

    #[test]
    fn count_is_a_byte_add_held_at_four() {
        assert_eq!(add_count(0, 5), 4);
        assert_eq!(add_count(1, 2), 3);
        assert_eq!(add_count(3, -1), 2);
        assert_eq!(add_count(0, 0x104), 4);
    }

    #[test]
    fn deploy_turns_the_second_and_third_pods() {
        assert_eq!(
            deploy_attitudes([0x100, 0x7000, -0x7000]),
            [
                [0x100, 0x7000, -0x7000],
                [0x100, -0x5000, -0x7000],
                [0x100, -0x5000, -0x3000],
            ]
        );
        assert_eq!(deploy_sound_rate(0xFFFF_FFFF), 0x10000 + 0x1FFF);
    }
}
