//! Retail primary-weapon firing cadence and event generation.
//!
//! The recovered weapon descriptors identify weapon id 1 as an
//! infinite-ammunition class-1 projectile and weapon id 2 as a finite class-3
//! projectile. Both set the auxiliary-command flag. `FUN_00444FA0` converts
//! their cadence bytes to 20,000-us game ticks. `FUN_00424650` creates the
//! selected lightweight projectile record, then a distinct class-15 record
//! with zero direction/time fields when flags bit 2 is set, and plays the
//! descriptor-selected sound once after the callback's catch-up loop. The
//! caller submits that complete event batch to `WorldFx`. `DAT_004D02C0` rows
//! whose entity-hit slot is `FUN_0043F590`, `FUN_0043F6E0`, or `FUN_0043F780`
//! use the same emitter;
//! selector `0x12` selects the same ballistic gun family as selector 1 at a
//! 40-ms cadence, with particle class 2. Antidote's `FUN_0043F7C0` selects
//! the separate `FUN_00411320` cure entry. The particle families are not ordinary
//! world entities and have no Section-8 projectile model. Selectors3/4/5 instead
//! launch Section-12 Type59/59/42 bodies through the explicitly named local
//! entity-weapon approximation, retaining this same cadence/ammunition owner.
//!
//! The caller supplies the live body/barrel launch basis and shooter state.
//! Static tracing proves that both transient commands start at the shooter
//! center, the class-1 bullet inherits shooter velocity, and class 15 resolves
//! to the above-water class-32 muzzle flash/smoke effect. Collision callbacks
//! remain the only part of the class-1 lifecycle outside this scheduler.

use std::time::Duration;

use crate::particle_descriptors::{particle_descriptor, particle_frame};
use crate::projectile_emitter::projectile_class_row;
use crate::targetter::{fun_0044ea60, TargetterTrajectory};
use crate::weapon_inventory::WeaponDescriptor;

/// `FUN_0043F590` entity-hit slot shared by recovered player projectile classes.
const FUN_0043F590_HIT_VA: u32 = 0x0043_F590;
/// `FUN_0043F6E0` entity-hit slot: F610 visual, then always `FUN_00410EB0`,
/// then positional sound 90. Firing is admitted; the hit suffix stays fail-closed.
const FUN_0043F6E0_HIT_VA: u32 = 0x0043_F6E0;
/// `FUN_0043F780` entity-hit slot for class 5 (selector 8). Firing is admitted;
/// F780 entity `+0x82` sound and F920/`FUN_00427DE0` static are live.
const FUN_0043F780_HIT_VA: u32 = 0x0043_F780;
/// Class6 Antidote: 11320 cure entry and F950 terrain infection clear.
const FUN_0043F7C0_HIT_VA: u32 = 0x0043_F7C0;

/// Retail simulation tick length (`1 / 50 s`).
pub const RETAIL_TICK_DURATION: Duration = Duration::from_micros(20_000);
/// Descriptor byte `+0x0a` for the default primary weapon.
pub const PRIMARY_REPEAT_TICKS: u64 = 8;
/// Exact interval between held-trigger firing events.
pub const PRIMARY_REPEAT_INTERVAL: Duration = Duration::from_micros(20_000 * PRIMARY_REPEAT_TICKS);
/// Descriptor byte `+0x0a` for the Level-1 weapon pickup (selector 2).
pub const UPGRADED_PRIMARY_REPEAT_TICKS: u64 = 15;
/// Exact selector-2 interval between held-trigger firing events.
pub const UPGRADED_PRIMARY_REPEAT_INTERVAL: Duration =
    Duration::from_micros(20_000 * UPGRADED_PRIMARY_REPEAT_TICKS);
/// Projectile class selected by weapon/resource id 1.
pub const PRIMARY_PROJECTILE_CLASS: u8 = 1;
/// Global Section-3 sprite used by particle class 1.
pub const PRIMARY_PROJECTILE_SPRITE_ID: u16 = 705;
/// Default speed from projectile-class record `0x004D02D0`.
pub const PRIMARY_PROJECTILE_SPEED_RAW: i32 = 4_000;
/// Particle-class-1 lifetime in original 50-Hz ticks.
pub const PRIMARY_PROJECTILE_LIFETIME_TICKS: u16 = 200;
/// Projectile class selected by weapon/resource id 2.
pub const UPGRADED_PRIMARY_PROJECTILE_CLASS: u8 = 3;
/// Global Section-3 sprite used by particle class 3.
pub const UPGRADED_PRIMARY_PROJECTILE_SPRITE_ID: u16 = 704;
/// Default speed selected by projectile resource 2.
pub const UPGRADED_PRIMARY_PROJECTILE_SPEED_RAW: i32 = 2_000;
/// Particle-class-3 lifetime in original 50-Hz ticks.
pub const UPGRADED_PRIMARY_PROJECTILE_LIFETIME_TICKS: u16 = 255;
/// Auxiliary projectile/effect selector created by descriptor byte `+0x17`.
pub const PRIMARY_AUXILIARY_CLASS: u8 = 15;
/// Projectile-table speed stored for auxiliary class 15.
pub const PRIMARY_AUXILIARY_SPEED_RAW: i32 = 1;
/// Projectile-table particle class stored for auxiliary class 15.
pub const PRIMARY_AUXILIARY_PARTICLE_CLASS: u8 = 32;
/// Global Section-11 alias played once per callback which fires any rounds.
pub const PRIMARY_SOUND_ID: usize = 88;
/// Global Section-11 alias played once per selector-2 callback which fires.
pub const UPGRADED_PRIMARY_SOUND_ID: usize = 60;
/// Descriptor byte `+0x0B` for both proven primary weapons.
pub const PRIMARY_JOINT_DECAY_RATE: u8 = 5;

/// Convert one component from the retail particle velocity domain to port
/// world units per second.
///
/// `FUN_00440120` integrates raw velocity as
/// `(dt_us >> 5) * velocity >> 15` into signed 8.8 positions. The smooth port
/// equivalent is therefore `raw * (1_000_000 / 2^20) / 256` per second.
pub const fn raw_velocity_to_world_per_second(raw: i32) -> f32 {
    raw as f32 * (1_000_000.0 / 1_048_576.0) / 256.0
}

/// Proven immutable metadata for one default-primary projectile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrimaryProjectileSpec {
    pub projectile_class: u8,
    pub sprite_id: u16,
    pub speed_raw: i32,
    pub speed_world_per_second: f32,
    pub lifetime_ticks: u16,
}

pub const PRIMARY_PROJECTILE_SPEC: PrimaryProjectileSpec = PrimaryProjectileSpec {
    projectile_class: PRIMARY_PROJECTILE_CLASS,
    sprite_id: PRIMARY_PROJECTILE_SPRITE_ID,
    speed_raw: PRIMARY_PROJECTILE_SPEED_RAW,
    speed_world_per_second: raw_velocity_to_world_per_second(PRIMARY_PROJECTILE_SPEED_RAW),
    lifetime_ticks: PRIMARY_PROJECTILE_LIFETIME_TICKS,
};

pub const UPGRADED_PRIMARY_PROJECTILE_SPEC: PrimaryProjectileSpec = PrimaryProjectileSpec {
    projectile_class: UPGRADED_PRIMARY_PROJECTILE_CLASS,
    sprite_id: UPGRADED_PRIMARY_PROJECTILE_SPRITE_ID,
    speed_raw: UPGRADED_PRIMARY_PROJECTILE_SPEED_RAW,
    speed_world_per_second: raw_velocity_to_world_per_second(UPGRADED_PRIMARY_PROJECTILE_SPEED_RAW),
    lifetime_ticks: UPGRADED_PRIMARY_PROJECTILE_LIFETIME_TICKS,
};

/// Exact immutable firing fields selected by one live inventory descriptor.
///
/// Ammunition ownership deliberately stays outside this type. The caller owns
/// the inventory slots and supplies an explicit [`PrimaryShotBudget`] to each
/// update, while this profile owns only the recovered firing policy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrimaryWeaponProfile {
    pub selector: u8,
    pub delivery: PlayerProjectileDelivery,
    pub repeat_interval: Duration,
    pub projectile: PrimaryProjectileSpec,
    /// Trajectory family returned by retail `FUN_0044EA60` for this
    /// descriptor's projectile-method id. Targetter prediction consumes the
    /// same family as firing instead of guessing from projectile visuals.
    pub targetter_trajectory: TargetterTrajectory,
    pub auxiliary: PrimaryAuxiliarySpec,
    /// `FUN_00444FA0` emitter `+0x17` from descriptor flags bit 2.
    pub emits_auxiliary: bool,
    pub sound_id: usize,
    /// Multiplier applied to `(elapsed_us >> 4)` while both retained gun-joint
    /// pulse words decay toward zero.
    pub joint_decay_rate: u8,
}

/// Lightweight particles and Section-12 bodies have separate lifecycle owners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerProjectileDelivery {
    Particle,
    NativeEntityWeapon { entity_type: u16 },
}

pub const DEFAULT_PRIMARY_PROFILE: PrimaryWeaponProfile = PrimaryWeaponProfile {
    selector: 1,
    delivery: PlayerProjectileDelivery::Particle,
    repeat_interval: PRIMARY_REPEAT_INTERVAL,
    projectile: PRIMARY_PROJECTILE_SPEC,
    targetter_trajectory: fun_0044ea60(1),
    auxiliary: PRIMARY_AUXILIARY_SPEC,
    emits_auxiliary: true,
    sound_id: PRIMARY_SOUND_ID,
    joint_decay_rate: PRIMARY_JOINT_DECAY_RATE,
};

pub const UPGRADED_PRIMARY_PROFILE: PrimaryWeaponProfile = PrimaryWeaponProfile {
    selector: 2,
    delivery: PlayerProjectileDelivery::Particle,
    repeat_interval: UPGRADED_PRIMARY_REPEAT_INTERVAL,
    projectile: UPGRADED_PRIMARY_PROJECTILE_SPEC,
    targetter_trajectory: fun_0044ea60(2),
    auxiliary: PRIMARY_AUXILIARY_SPEC,
    emits_auxiliary: true,
    sound_id: UPGRADED_PRIMARY_SOUND_ID,
    joint_decay_rate: PRIMARY_JOINT_DECAY_RATE,
};

/// Resolve the two primary descriptors proven by the Level-1 runtime trace.
pub const fn primary_weapon_profile(selector: u8) -> Option<PrimaryWeaponProfile> {
    match selector {
        1 => Some(DEFAULT_PRIMARY_PROFILE),
        2 => Some(UPGRADED_PRIMARY_PROFILE),
        _ => None,
    }
}

/// `FUN_00444FA0` / `DAT_004D02C0` firing profile for one live descriptor.
///
/// Selector `0x12` uses table row `0x004D03E0`: speed 4000, particle class 2.
/// Its descriptor `0x004CDDE8` supplies cadence 2, sound 88 and auxiliary bit
/// 2. It has no extra-volley branch in `FUN_00424650` (only selectors
/// 10/22/26 do), so each event retains the ordinary alternating gun phase.
/// Selectors3/4/5 explicitly deliver modeled Type59/59/42 bodies to the native
/// entity-weapon constructor. Other rows whose particle class is zero, or
/// whose entity-hit slot is none of
/// `FUN_0043F590` / `FUN_0043F6E0` / `FUN_0043F780` / `FUN_0043F7C0`,
/// remain unsupported. Antidote retains its separate cure/static-clear owners.
pub fn fire_profile_from_descriptor(descriptor: &WeaponDescriptor) -> Option<PrimaryWeaponProfile> {
    let selector = descriptor.selector();
    if selector == 0 {
        return None;
    }
    let selector_u8 = u8::try_from(selector).ok()?;
    if let Some(profile) = primary_weapon_profile(selector_u8) {
        return Some(profile);
    }
    let row = projectile_class_row(selector)?;
    if matches!(selector, 3..=5) {
        return Some(PrimaryWeaponProfile {
            selector: selector_u8,
            delivery: PlayerProjectileDelivery::NativeEntityWeapon {
                entity_type: row.trailing_raw as u16,
            },
            repeat_interval: Duration::from_micros(descriptor.cadence_micros()),
            projectile: PrimaryProjectileSpec {
                projectile_class: 0,
                sprite_id: 0,
                speed_raw: row.speed_raw,
                speed_world_per_second: raw_velocity_to_world_per_second(row.speed_raw),
                lifetime_ticks: 0,
            },
            targetter_trajectory: fun_0044ea60(selector),
            auxiliary: PRIMARY_AUXILIARY_SPEC,
            emits_auxiliary: descriptor.emits_auxiliary_command(),
            sound_id: usize::from(descriptor.firing_sound_id()),
            joint_decay_rate: descriptor.joint_decay_rate(),
        });
    }
    if row.particle_class == 0 {
        return None;
    }
    let particle = particle_descriptor(row.particle_class)?;
    if !matches!(
        particle.raw_u32(0x1c),
        FUN_0043F590_HIT_VA | FUN_0043F6E0_HIT_VA | FUN_0043F780_HIT_VA | FUN_0043F7C0_HIT_VA
    ) {
        return None;
    }
    let frame = particle_frame(row.particle_class, 0)?;
    Some(PrimaryWeaponProfile {
        selector: selector_u8,
        delivery: PlayerProjectileDelivery::Particle,
        repeat_interval: Duration::from_micros(descriptor.cadence_micros()),
        projectile: PrimaryProjectileSpec {
            projectile_class: row.particle_class,
            sprite_id: frame.sprite_id,
            speed_raw: row.speed_raw,
            speed_world_per_second: raw_velocity_to_world_per_second(row.speed_raw),
            lifetime_ticks: particle.lifetime_ticks(),
        },
        targetter_trajectory: fun_0044ea60(selector),
        auxiliary: PRIMARY_AUXILIARY_SPEC,
        emits_auxiliary: descriptor.emits_auxiliary_command(),
        sound_id: usize::from(descriptor.firing_sound_id()),
        joint_decay_rate: descriptor.joint_decay_rate(),
    })
}

/// Maximum number of successful events this update may consume.
///
/// This is an explicit policy instead of encoding infinite ammunition as a
/// magic integer. Due cadence boundaries still advance after a finite budget
/// reaches zero; only event/audio emission and A/B muzzle alternation are
/// suppressed, matching the recovered empty-weapon guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryShotBudget {
    Unlimited,
    Limited(u32),
}

impl PrimaryShotBudget {
    fn consume_one(&mut self) -> bool {
        match self {
            Self::Unlimited => true,
            Self::Limited(remaining) if *remaining > 0 => {
                *remaining -= 1;
                true
            }
            Self::Limited(_) => false,
        }
    }
}

/// The two physical controls which feed the one retail held-trigger state.
///
/// A handoff from one source to the other is not a release/re-press edge.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PrimaryTriggerInput {
    pub source_a: bool,
    pub source_b: bool,
}

impl PrimaryTriggerInput {
    pub const fn held(self) -> bool {
        self.source_a || self.source_b
    }
}

/// Caller-resolved launch basis for the class-1 projectile.
///
/// `direction_unit` is the recovered body-up/body-forward barrel combination
/// and is expected to be a unit world-space vector. It is not normalized here
/// because retail consumes the fixed-point basis directly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrimaryLaunchBasis {
    pub origin_world: [f32; 3],
    pub direction_unit: [f32; 3],
}

/// Alternating runtime selector shared by the two proven primary loadouts'
/// authored side guns. The live capture identifies selector variants 0/1 but
/// does not prove a player-relative left/right label, so this API deliberately
/// retains the neutral A/B names.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PrimaryGunChannel {
    #[default]
    A,
    B,
}

impl PrimaryGunChannel {
    const fn next(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }
}

/// The two presentation launch bases recovered from the currently selected
/// authored `player4` loadout. Their origins are mesh/mount results, not
/// guessed hull offsets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrimaryAuthoredGunMounts {
    pub gun_a: PrimaryLaunchBasis,
    pub gun_b: PrimaryLaunchBasis,
}

/// Resolution state for the visible side-gun mounts.
///
/// Simulation never depends on this result: retail's class-1/class-15 dispatch
/// refreshes both transient records from the entity center. When model data is
/// unavailable, the typed unresolved state therefore falls back to that proven
/// center without inventing a symmetric pair.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum PrimaryGunMounts {
    Authored(PrimaryAuthoredGunMounts),
    #[default]
    Unresolved,
}

impl PrimaryGunMounts {
    pub const fn basis(self, channel: PrimaryGunChannel) -> Option<PrimaryLaunchBasis> {
        match (self, channel) {
            (Self::Authored(mounts), PrimaryGunChannel::A) => Some(mounts.gun_a),
            (Self::Authored(mounts), PrimaryGunChannel::B) => Some(mounts.gun_b),
            (Self::Unresolved, _) => None,
        }
    }
}

/// Geometry sampled for a firing event.
///
/// Retail starts the bullet, auxiliary command, and sound at the shooter center;
/// the fields remain explicit so the scheduler does not depend on world storage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrimaryFireGeometry {
    /// Proven simulation launch basis. Its origin remains the shooter center.
    pub launch_basis: PrimaryLaunchBasis,
    /// Authored presentation-only bases for the visible alternating streams.
    pub gun_mounts: PrimaryGunMounts,
    /// Retail refreshes the class-15 record's origin from the shooter before
    /// dispatch, so it is stationary at this point rather than a second muzzle.
    pub shooter_origin_world: [f32; 3],
    /// Runtime entity handle/id used by the smoke callback to follow the
    /// shooter's current velocity for its short lifetime.
    pub shooter_id: u32,
    /// Immutable retail entity type copied into projectile byte `+0x1C` at
    /// birth. Callers without a proven entity-backed shooter retain `None`.
    pub shooter_entity_type_at_birth: Option<u8>,
    /// Retail adds the complete shooter velocity to the bullet and initializes
    /// the class-32 effect with it as well.
    pub shooter_velocity_world: [f32; 3],
    pub sound_origin_world: [f32; 3],
}

/// One lightweight projectile-record creation request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrimaryProjectileSpawn {
    pub owner_id: u32,
    /// Simulation/collision origin refreshed from the shooter center by retail.
    pub origin_world: [f32; 3],
    pub direction_unit: [f32; 3],
    /// Visible origin/direction resolved from the selected authored gun model.
    /// World collision continues to use `origin_world` and `direction_unit`.
    pub presentation_basis: PrimaryLaunchBasis,
    pub velocity_world_per_second: [f32; 3],
    pub spec: PrimaryProjectileSpec,
}

/// Proven immutable metadata for the distinct class-15 auxiliary command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimaryAuxiliarySpec {
    pub projectile_class: u8,
    pub speed_raw: i32,
    pub particle_class: u8,
    pub direction_raw: [i32; 3],
    pub travel_or_time_scalar: i32,
}

pub const PRIMARY_AUXILIARY_SPEC: PrimaryAuxiliarySpec = PrimaryAuxiliarySpec {
    projectile_class: PRIMARY_AUXILIARY_CLASS,
    speed_raw: PRIMARY_AUXILIARY_SPEED_RAW,
    particle_class: PRIMARY_AUXILIARY_PARTICLE_CLASS,
    direction_raw: [0; 3],
    travel_or_time_scalar: 0,
};

/// One class-15 auxiliary-effect request emitted after the normal bullet.
///
/// Retail zeroes its direction/time fields on creation and refreshes its origin
/// from the shooter before dispatch. The dispatcher resolves it to the
/// above-water class-32 flash/smoke particle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrimaryAuxiliaryCommand {
    pub owner_id: u32,
    /// Center used by retail dispatch and the above-water classification.
    pub origin_world: [f32; 3],
    /// Authored muzzle used only to place the visible class-32 flash/smoke.
    pub presentation_origin_world: [f32; 3],
    pub inherited_velocity_world_per_second: [f32; 3],
    pub spec: PrimaryAuxiliarySpec,
}

/// One positional sound request. Alias 88 contains its recovered 1.6x rate,
/// so this layer must not duplicate the alias multiplier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrimarySoundEvent {
    pub sound_id: usize,
    pub origin_world: [f32; 3],
}

/// Atomic output of one retail firing event: one selected projectile, its
/// distinct class-15 auxiliary command, and one sound request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrimaryFireEvent {
    pub delivery: PlayerProjectileDelivery,
    pub fired_at: Duration,
    pub gun_channel: PrimaryGunChannel,
    pub source_entity_type_at_birth: Option<u8>,
    pub projectile: PrimaryProjectileSpawn,
    pub auxiliary: PrimaryAuxiliaryCommand,
    /// `FUN_00444FA0` only emits the class-15 companion when flags bit 2 is set.
    pub emits_auxiliary: bool,
    pub sound: PrimarySoundEvent,
}

/// Deterministic held-trigger scheduler shared by the proven primaries.
#[derive(Debug, Clone, Default)]
pub struct PrimaryWeapon {
    elapsed: Duration,
    trigger_held: bool,
    next_repeat: Option<Duration>,
    next_gun: PrimaryGunChannel,
    /// Weapon-component joint pointers `+0x00/+0x04`, which resolve to player
    /// callback words 2 and 3 respectively.
    joint_pulses: [u16; 2],
}

impl PrimaryWeapon {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    pub fn trigger_held(&self) -> bool {
        self.trigger_held
    }

    pub fn next_gun(&self) -> PrimaryGunChannel {
        self.next_gun
    }

    /// Current callback words 2 and 3 in player-model order.
    pub fn joint_pulse_words(&self) -> [u16; 2] {
        self.joint_pulses
    }

    /// Advance the weapon clock and return every firing event crossed by this
    /// update.
    ///
    /// The supplied input is the state observed at the end of the interval. A
    /// rising combined trigger fires at that boundary; while it remains held,
    /// repeats occur at exact descriptor cadence boundaries regardless of how
    /// render frames partition time. Anchoring a new edge at `end` prevents a
    /// long frame before the press from creating phantom catch-up shots.
    /// Callers using a large `delta` should supply geometry representative of
    /// that whole interval, or advance in their normal simulation steps.
    pub fn update(
        &mut self,
        delta: Duration,
        input: PrimaryTriggerInput,
        geometry: PrimaryFireGeometry,
    ) -> Vec<PrimaryFireEvent> {
        self.update_with_profile(
            delta,
            input,
            geometry,
            DEFAULT_PRIMARY_PROFILE,
            PrimaryShotBudget::Unlimited,
        )
    }

    /// Advance time and decay joints without emitting a shot or resetting A/B.
    ///
    /// `FUN_00444FA0` still visits an unfireable selected slot. Retail keeps
    /// the A/B phase across those visits and across later fireable reselection.
    pub fn idle(&mut self, delta: Duration, joint_decay_rate: u8, trigger_held: bool) {
        self.decay_joint_pulses(delta, joint_decay_rate);
        self.elapsed = self.elapsed.saturating_add(delta);
        if !trigger_held {
            self.next_repeat = None;
        }
        self.trigger_held = trigger_held;
    }

    /// Advance one selected primary using an explicit caller-owned shot budget.
    ///
    /// The one scheduler retains its A/B phase when the caller changes
    /// `profile`. A rejected empty-ammunition attempt advances the cadence but
    /// does not emit an event, play audio, or consume the alternating phase.
    pub fn update_with_profile(
        &mut self,
        delta: Duration,
        input: PrimaryTriggerInput,
        geometry: PrimaryFireGeometry,
        profile: PrimaryWeaponProfile,
        mut shot_budget: PrimaryShotBudget,
    ) -> Vec<PrimaryFireEvent> {
        let held = input.held();
        let end = self.elapsed.saturating_add(delta);
        let mut events = Vec::new();
        let mut pulse_cursor = self.elapsed;

        if held && !self.trigger_held {
            self.decay_joint_pulses(delta, profile.joint_decay_rate);
            pulse_cursor = end;
            if shot_budget.consume_one() {
                events.push(self.fire_event_and_pulse(end, geometry, profile));
            }
            self.next_repeat = end.checked_add(profile.repeat_interval);
        } else if !held {
            self.decay_joint_pulses(delta, profile.joint_decay_rate);
            pulse_cursor = end;
            self.next_repeat = None;
        }

        if held {
            while let Some(next_repeat) = self.next_repeat {
                if next_repeat > end {
                    break;
                }
                self.decay_joint_pulses(
                    next_repeat.saturating_sub(pulse_cursor),
                    profile.joint_decay_rate,
                );
                pulse_cursor = next_repeat;
                if shot_budget.consume_one() {
                    events.push(self.fire_event_and_pulse(next_repeat, geometry, profile));
                }
                self.next_repeat = next_repeat.checked_add(profile.repeat_interval);
            }
            self.decay_joint_pulses(end.saturating_sub(pulse_cursor), profile.joint_decay_rate);
        }

        self.elapsed = end;
        self.trigger_held = held;
        events
    }

    fn fire_event(
        &mut self,
        fired_at: Duration,
        geometry: PrimaryFireGeometry,
        profile: PrimaryWeaponProfile,
    ) -> PrimaryFireEvent {
        let gun_channel = self.next_gun;
        self.next_gun = self.next_gun.next();
        primary_fire_event(fired_at, gun_channel, geometry, profile)
    }

    fn fire_event_and_pulse(
        &mut self,
        fired_at: Duration,
        geometry: PrimaryFireGeometry,
        profile: PrimaryWeaponProfile,
    ) -> PrimaryFireEvent {
        let event = self.fire_event(fired_at, geometry, profile);
        // FUN_00424EE0 writes the alternate `+0x04` binding for selector zero
        // and the `+0x00` binding otherwise. The captured A/B shot variants
        // therefore map A -> callback word 3 and B -> callback word 2.
        let pulse_index = match event.gun_channel {
            PrimaryGunChannel::A => 1,
            PrimaryGunChannel::B => 0,
        };
        self.joint_pulses[pulse_index] = u16::MAX;
        event
    }

    fn decay_joint_pulses(&mut self, delta: Duration, rate: u8) {
        let elapsed_us = delta.as_micros().min(u128::from(u32::MAX)) as u32;
        let decrement = u32::from(rate).saturating_mul(elapsed_us >> 4);
        for pulse in &mut self.joint_pulses {
            *pulse = pulse.saturating_sub(decrement.min(u32::from(u16::MAX)) as u16);
        }
    }
}

fn primary_fire_event(
    fired_at: Duration,
    gun_channel: PrimaryGunChannel,
    geometry: PrimaryFireGeometry,
    profile: PrimaryWeaponProfile,
) -> PrimaryFireEvent {
    let basis = geometry.launch_basis;
    let presentation_basis = geometry.gun_mounts.basis(gun_channel).unwrap_or(basis);
    let speed = profile.projectile.speed_world_per_second;
    let projectile_velocity = std::array::from_fn(|axis| {
        basis.direction_unit[axis] * speed + geometry.shooter_velocity_world[axis]
    });
    let projectile = PrimaryProjectileSpawn {
        owner_id: geometry.shooter_id,
        origin_world: basis.origin_world,
        direction_unit: basis.direction_unit,
        presentation_basis,
        velocity_world_per_second: projectile_velocity,
        spec: profile.projectile,
    };

    PrimaryFireEvent {
        delivery: profile.delivery,
        fired_at,
        gun_channel,
        source_entity_type_at_birth: geometry.shooter_entity_type_at_birth,
        projectile,
        auxiliary: PrimaryAuxiliaryCommand {
            owner_id: geometry.shooter_id,
            origin_world: geometry.shooter_origin_world,
            presentation_origin_world: presentation_basis.origin_world,
            inherited_velocity_world_per_second: geometry.shooter_velocity_world,
            spec: profile.auxiliary,
        },
        emits_auxiliary: profile.emits_auxiliary,
        sound: PrimarySoundEvent {
            sound_id: profile.sound_id,
            origin_world: geometry.sound_origin_world,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_inventory::{Ammunition, PowerUpPayload, WeaponInventory};

    fn geometry() -> PrimaryFireGeometry {
        PrimaryFireGeometry {
            launch_basis: PrimaryLaunchBasis {
                origin_world: [1.0, 2.0, 3.0],
                direction_unit: [1.0, 0.0, 0.0],
            },
            gun_mounts: PrimaryGunMounts::Authored(PrimaryAuthoredGunMounts {
                gun_a: PrimaryLaunchBasis {
                    origin_world: [10.0, 20.0, 30.0],
                    direction_unit: [1.0, 0.0, 0.0],
                },
                gun_b: PrimaryLaunchBasis {
                    origin_world: [40.0, 50.0, 60.0],
                    direction_unit: [1.0, 0.0, 0.0],
                },
            }),
            shooter_origin_world: [4.0, 5.0, 6.0],
            shooter_id: 46,
            shooter_entity_type_at_birth: Some(46),
            shooter_velocity_world: [0.25, -0.5, 0.75],
            sound_origin_world: [7.0, 8.0, 9.0],
        }
    }

    const HELD_A: PrimaryTriggerInput = PrimaryTriggerInput {
        source_a: true,
        source_b: false,
    };
    const RELEASED: PrimaryTriggerInput = PrimaryTriggerInput {
        source_a: false,
        source_b: false,
    };

    #[test]
    fn recovered_primary_profiles_use_the_ballistic_targetter_family() {
        assert_eq!(
            primary_weapon_profile(1).unwrap().targetter_trajectory,
            TargetterTrajectory::Ballistic
        );
        assert_eq!(
            primary_weapon_profile(2).unwrap().targetter_trajectory,
            TargetterTrajectory::Ballistic
        );
        assert!(primary_weapon_profile(3).is_none());
    }

    #[test]
    fn tap_fires_immediately_and_only_once() {
        let mut weapon = PrimaryWeapon::new();
        let events = weapon.update(Duration::ZERO, HELD_A, geometry());
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].fired_at, Duration::ZERO);

        assert!(weapon
            .update(Duration::from_millis(20), RELEASED, geometry())
            .is_empty());
        assert!(weapon
            .update(Duration::from_secs(1), RELEASED, geometry())
            .is_empty());
    }

    #[test]
    fn hold_repeats_after_exactly_eight_retail_ticks() {
        let mut weapon = PrimaryWeapon::new();
        assert_eq!(weapon.update(Duration::ZERO, HELD_A, geometry()).len(), 1);

        for _ in 0..7 {
            assert!(weapon
                .update(RETAIL_TICK_DURATION, HELD_A, geometry())
                .is_empty());
        }
        let repeat = weapon.update(RETAIL_TICK_DURATION, HELD_A, geometry());
        assert_eq!(repeat.len(), 1);
        assert_eq!(repeat[0].fired_at, PRIMARY_REPEAT_INTERVAL);
    }

    #[test]
    fn every_firing_event_contains_bullet_auxiliary_and_one_sound() {
        let mut weapon = PrimaryWeapon::new();
        let event = weapon.update(Duration::ZERO, HELD_A, geometry())[0];

        assert_eq!(event.gun_channel, PrimaryGunChannel::A);
        assert_eq!(event.source_entity_type_at_birth, Some(46));
        assert_eq!(event.projectile.origin_world, [1.0, 2.0, 3.0]);
        assert_eq!(
            event.projectile.presentation_basis,
            PrimaryLaunchBasis {
                origin_world: [10.0, 20.0, 30.0],
                direction_unit: [1.0, 0.0, 0.0],
            }
        );
        assert_eq!(event.projectile.spec, PRIMARY_PROJECTILE_SPEC);
        assert_eq!(event.projectile.spec.projectile_class, 1);
        assert_eq!(event.projectile.spec.sprite_id, 705);
        assert_eq!(event.projectile.spec.speed_raw, 4_000);
        assert_eq!(event.projectile.spec.lifetime_ticks, 200);
        assert_eq!(
            event.projectile.velocity_world_per_second,
            [
                PRIMARY_PROJECTILE_SPEC.speed_world_per_second + 0.25,
                -0.5,
                0.75,
            ]
        );
        assert_eq!(event.projectile.owner_id, 46);
        assert_eq!(event.auxiliary.origin_world, [4.0, 5.0, 6.0]);
        assert_eq!(
            event.auxiliary.presentation_origin_world,
            [10.0, 20.0, 30.0]
        );
        assert_eq!(event.auxiliary.owner_id, 46);
        assert_eq!(
            event.auxiliary.inherited_velocity_world_per_second,
            [0.25, -0.5, 0.75]
        );
        assert_eq!(event.auxiliary.spec, PRIMARY_AUXILIARY_SPEC);
        assert_eq!(event.auxiliary.spec.projectile_class, 15);
        assert_eq!(event.auxiliary.spec.speed_raw, 1);
        assert_eq!(event.auxiliary.spec.particle_class, 32);
        assert_eq!(event.auxiliary.spec.direction_raw, [0; 3]);
        assert_eq!(event.auxiliary.spec.travel_or_time_scalar, 0);
        assert_eq!(event.sound.sound_id, 88);
        assert_eq!(event.sound.origin_world, [7.0, 8.0, 9.0]);
    }

    #[test]
    fn unknown_shooter_type_stays_explicitly_unresolved() {
        let mut geometry = geometry();
        geometry.shooter_entity_type_at_birth = None;

        let mut weapon = PrimaryWeapon::new();
        let event = weapon.update(Duration::ZERO, HELD_A, geometry)[0];

        assert_eq!(event.source_entity_type_at_birth, None);
    }

    #[test]
    fn render_frame_partition_does_not_change_output() {
        fn run(frame_count: u64) -> Vec<PrimaryFireEvent> {
            let mut weapon = PrimaryWeapon::new();
            let total_nanos = 1_000_000_000_u64;
            let base = total_nanos / frame_count;
            let remainder = total_nanos % frame_count;
            // Establish the already-held state at the beginning of the measured
            // second; subsequent frame partitioning must not move its cadence.
            let mut events = weapon.update(Duration::ZERO, HELD_A, geometry());
            for frame in 0..frame_count {
                let nanos = base + u64::from(frame < remainder);
                events.extend(weapon.update(Duration::from_nanos(nanos), HELD_A, geometry()));
            }
            assert_eq!(weapon.elapsed(), Duration::from_secs(1));
            events
        }

        let sixty_hz = run(60);
        let one_forty_four_hz = run(144);
        assert_eq!(sixty_hz, one_forty_four_hz);
        assert_eq!(sixty_hz.len(), 7);
        assert_eq!(
            sixty_hz
                .iter()
                .map(|event| event.fired_at.as_millis())
                .collect::<Vec<_>>(),
            vec![0, 160, 320, 480, 640, 800, 960]
        );
    }

    #[test]
    fn a_new_press_after_a_long_frame_does_not_catch_up_repeats() {
        let mut weapon = PrimaryWeapon::new();
        assert!(weapon
            .update(Duration::from_millis(500), RELEASED, geometry())
            .is_empty());

        let press = weapon.update(Duration::from_millis(250), HELD_A, geometry());
        assert_eq!(press.len(), 1);
        assert_eq!(press[0].fired_at, Duration::from_millis(750));
        assert!(weapon
            .update(Duration::from_millis(159), HELD_A, geometry())
            .is_empty());
        let repeat = weapon.update(Duration::from_millis(1), HELD_A, geometry());
        assert_eq!(repeat.len(), 1);
        assert_eq!(repeat[0].fired_at, Duration::from_millis(910));
    }

    #[test]
    fn release_and_repress_fire_immediately_and_restart_cadence() {
        let mut weapon = PrimaryWeapon::new();
        let first = weapon.update(Duration::ZERO, HELD_A, geometry());
        assert_eq!(first[0].gun_channel, PrimaryGunChannel::A);
        assert!(weapon
            .update(Duration::from_millis(80), HELD_A, geometry())
            .is_empty());
        assert!(weapon
            .update(Duration::from_millis(40), RELEASED, geometry())
            .is_empty());

        let repress = weapon.update(Duration::ZERO, HELD_A, geometry());
        assert_eq!(repress.len(), 1);
        assert_eq!(repress[0].fired_at, Duration::from_millis(120));
        assert_eq!(repress[0].gun_channel, PrimaryGunChannel::B);
        assert_eq!(
            repress[0].projectile.presentation_basis.origin_world,
            [40.0, 50.0, 60.0]
        );
        assert!(weapon
            .update(Duration::from_millis(159), HELD_A, geometry())
            .is_empty());
        let repeat = weapon.update(Duration::from_millis(1), HELD_A, geometry());
        assert_eq!(repeat.len(), 1);
        assert_eq!(repeat[0].fired_at, Duration::from_millis(280));
        assert_eq!(repeat[0].gun_channel, PrimaryGunChannel::A);
    }

    #[test]
    fn switching_trigger_sources_does_not_create_a_second_edge() {
        let mut weapon = PrimaryWeapon::new();
        assert_eq!(weapon.update(Duration::ZERO, HELD_A, geometry()).len(), 1);
        assert!(weapon
            .update(
                Duration::from_millis(80),
                PrimaryTriggerInput {
                    source_a: false,
                    source_b: true,
                },
                geometry(),
            )
            .is_empty());
        let repeat = weapon.update(
            Duration::from_millis(80),
            PrimaryTriggerInput {
                source_a: false,
                source_b: true,
            },
            geometry(),
        );
        assert_eq!(repeat.len(), 1);
        assert_eq!(repeat[0].fired_at, Duration::from_millis(160));
        assert_eq!(repeat[0].gun_channel, PrimaryGunChannel::B);
        assert_eq!(
            repeat[0].projectile.presentation_basis.origin_world,
            [40.0, 50.0, 60.0]
        );
    }

    #[test]
    fn unresolved_mounts_preserve_proven_center_presentation() {
        let mut unresolved = geometry();
        unresolved.gun_mounts = PrimaryGunMounts::Unresolved;
        let mut weapon = PrimaryWeapon::new();

        let event = weapon.update(Duration::ZERO, HELD_A, unresolved)[0];

        assert_eq!(event.projectile.origin_world, [1.0, 2.0, 3.0]);
        assert_eq!(event.projectile.presentation_basis, unresolved.launch_basis);
        assert_eq!(
            event.auxiliary.presentation_origin_world,
            unresolved.launch_basis.origin_world
        );
    }

    #[test]
    fn catch_up_shots_alternate_once_per_actual_event() {
        let mut weapon = PrimaryWeapon::new();
        let mut events = weapon.update(Duration::ZERO, HELD_A, geometry());
        events.extend(weapon.update(Duration::from_millis(480), HELD_A, geometry()));

        assert_eq!(
            events
                .iter()
                .map(|event| event.gun_channel)
                .collect::<Vec<_>>(),
            vec![
                PrimaryGunChannel::A,
                PrimaryGunChannel::B,
                PrimaryGunChannel::A,
                PrimaryGunChannel::B,
            ]
        );
        assert_eq!(weapon.next_gun(), PrimaryGunChannel::A);
    }

    #[test]
    fn retained_joint_pulses_follow_captured_a_b_mapping_and_decay() {
        let mut weapon = PrimaryWeapon::new();
        let first = weapon.update(Duration::ZERO, HELD_A, geometry());
        assert_eq!(first[0].gun_channel, PrimaryGunChannel::A);
        assert_eq!(weapon.joint_pulse_words(), [0, u16::MAX]);

        let repeat = weapon.update(PRIMARY_REPEAT_INTERVAL, HELD_A, geometry());
        assert_eq!(repeat[0].gun_channel, PrimaryGunChannel::B);
        // 5 * (160_000 >> 4) = 50_000 before the next side is pulsed.
        assert_eq!(weapon.joint_pulse_words(), [u16::MAX, 15_535]);

        let released = weapon.update(Duration::from_micros(8_000), RELEASED, geometry());
        assert!(released.is_empty());
        assert_eq!(weapon.joint_pulse_words(), [63_035, 13_035]);
    }

    #[test]
    fn selector_two_uses_its_recovered_projectile_cadence_and_sound() {
        let mut weapon = PrimaryWeapon::new();
        let first = weapon.update_with_profile(
            Duration::ZERO,
            HELD_A,
            geometry(),
            UPGRADED_PRIMARY_PROFILE,
            PrimaryShotBudget::Limited(200),
        );
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].projectile.spec, UPGRADED_PRIMARY_PROJECTILE_SPEC);
        assert_eq!(first[0].projectile.spec.projectile_class, 3);
        assert_eq!(first[0].projectile.spec.sprite_id, 704);
        assert_eq!(first[0].projectile.spec.speed_raw, 2_000);
        assert_eq!(first[0].projectile.spec.lifetime_ticks, 255);
        assert_eq!(first[0].sound.sound_id, UPGRADED_PRIMARY_SOUND_ID);
        assert_eq!(first[0].auxiliary.spec, PRIMARY_AUXILIARY_SPEC);

        assert!(weapon
            .update_with_profile(
                Duration::from_millis(299),
                HELD_A,
                geometry(),
                UPGRADED_PRIMARY_PROFILE,
                PrimaryShotBudget::Limited(199),
            )
            .is_empty());
        let repeat = weapon.update_with_profile(
            Duration::from_millis(1),
            HELD_A,
            geometry(),
            UPGRADED_PRIMARY_PROFILE,
            PrimaryShotBudget::Limited(199),
        );
        assert_eq!(repeat.len(), 1);
        assert_eq!(repeat[0].fired_at, UPGRADED_PRIMARY_REPEAT_INTERVAL);
    }

    #[test]
    fn factory_machine_gun_uses_its_own_descriptor_and_supported_particle_family() {
        let mut inventory = WeaponInventory::new();
        inventory.acquire_weapon(PowerUpPayload::from_runtime_word(0x0001_f412));
        let descriptor = inventory.selected_descriptor();
        let profile = fire_profile_from_descriptor(descriptor).expect("factory machine gun");
        assert_eq!(profile.selector, 0x12);
        assert_eq!(profile.repeat_interval, Duration::from_millis(40));
        assert_eq!(profile.projectile.projectile_class, 2);
        assert_eq!(profile.projectile.sprite_id, 705);
        assert_eq!(profile.projectile.speed_raw, 4_000);
        assert_eq!(profile.projectile.lifetime_ticks, 200);
        assert_eq!(profile.targetter_trajectory, TargetterTrajectory::Ballistic);
        assert!(profile.emits_auxiliary);
        assert_eq!(profile.auxiliary, PRIMARY_AUXILIARY_SPEC);
        assert_eq!(profile.sound_id, 88);
        assert_eq!(profile.joint_decay_rate, 5);
        assert_eq!(descriptor.ammunition(), Ammunition::Infinite);

        // F7C0 belongs to Antidote/selector 7, not the machine-gun upgrade.
        inventory.acquire_weapon(PowerUpPayload {
            selector: 7,
            amount: 999,
        });
        let antidote = fire_profile_from_descriptor(inventory.selected_descriptor()).unwrap();
        assert_eq!(antidote.selector, 7);
        assert_eq!(antidote.projectile.projectile_class, 6);
        assert_eq!(antidote.projectile.sprite_id, 917);
        assert_eq!(antidote.projectile.speed_raw, 2000);
        assert_eq!(antidote.repeat_interval, Duration::from_millis(80));
        assert_eq!(antidote.sound_id, 64);
        assert!(!antidote.emits_auxiliary);
    }

    #[test]
    fn factory_machine_gun_fires_one_alternating_shot_every_two_ticks() {
        let mut inventory = WeaponInventory::new();
        inventory.acquire_weapon(PowerUpPayload::from_runtime_word(0x0001_f412));
        let profile = fire_profile_from_descriptor(inventory.selected_descriptor()).unwrap();
        let mut weapon = PrimaryWeapon::new();
        let mut events = weapon.update_with_profile(
            Duration::ZERO,
            HELD_A,
            geometry(),
            profile,
            PrimaryShotBudget::Unlimited,
        );
        for tick in 1..=8 {
            let next = weapon.update_with_profile(
                RETAIL_TICK_DURATION,
                HELD_A,
                geometry(),
                profile,
                PrimaryShotBudget::Unlimited,
            );
            assert_eq!(next.len(), usize::from(tick % 2 == 0), "tick {tick}");
            events.extend(next);
        }
        assert_eq!(events.len(), 5);
        for (index, event) in events.iter().enumerate() {
            assert_eq!(event.fired_at, Duration::from_millis(index as u64 * 40));
            assert_eq!(
                event.gun_channel,
                if index % 2 == 0 {
                    PrimaryGunChannel::A
                } else {
                    PrimaryGunChannel::B
                }
            );
            // The larger visible bullet retains the normal center-origin
            // trajectory and shooter velocity, without invented spread.
            assert_eq!(
                event.projectile.origin_world,
                geometry().launch_basis.origin_world
            );
            assert_eq!(
                event.projectile.direction_unit,
                geometry().launch_basis.direction_unit
            );
            assert_eq!(
                event.projectile.velocity_world_per_second,
                [
                    PRIMARY_PROJECTILE_SPEC.speed_world_per_second + 0.25,
                    -0.5,
                    0.75
                ]
            );
            assert!(event.emits_auxiliary);
            assert_eq!(event.auxiliary.spec.direction_raw, [0; 3]);
            assert_eq!(event.sound.sound_id, 88);
            inventory.commit_selected_round();
        }
        assert_eq!(inventory.selected_descriptor().selector(), 0x12);
        assert_eq!(inventory.selected_ammunition(), Ammunition::Infinite);
        // At t=160ms A has just pulsed; B decayed for 40ms at rate 5.
        assert_eq!(weapon.joint_pulse_words(), [53_035, u16::MAX]);
        weapon.update_with_profile(
            Duration::ZERO,
            RELEASED,
            geometry(),
            profile,
            PrimaryShotBudget::Unlimited,
        );
        let default = weapon.update(Duration::ZERO, HELD_A, geometry());
        assert_eq!(
            default[0].gun_channel,
            PrimaryGunChannel::B,
            "phase survives weapon changes"
        );
    }

    #[test]
    fn finite_budget_suppresses_only_unfunded_events_and_phase_changes() {
        let mut weapon = PrimaryWeapon::new();
        let events = weapon.update_with_profile(
            Duration::ZERO,
            HELD_A,
            geometry(),
            UPGRADED_PRIMARY_PROFILE,
            PrimaryShotBudget::Limited(0),
        );
        assert!(events.is_empty());
        assert_eq!(weapon.next_gun(), PrimaryGunChannel::A);
        assert_eq!(weapon.joint_pulse_words(), [0, 0]);

        // The empty attempt established the normal held-trigger cadence. A
        // newly supplied round before that boundary does not create an
        // invented rising edge or catch-up event.
        assert!(weapon
            .update_with_profile(
                Duration::from_millis(299),
                HELD_A,
                geometry(),
                UPGRADED_PRIMARY_PROFILE,
                PrimaryShotBudget::Limited(1),
            )
            .is_empty());
        let funded = weapon.update_with_profile(
            Duration::from_millis(1),
            HELD_A,
            geometry(),
            UPGRADED_PRIMARY_PROFILE,
            PrimaryShotBudget::Limited(1),
        );
        assert_eq!(funded.len(), 1);
        assert_eq!(funded[0].gun_channel, PrimaryGunChannel::A);
        assert_eq!(weapon.next_gun(), PrimaryGunChannel::B);
    }

    #[test]
    fn weapon_switch_preserves_one_shared_alternating_muzzle_phase() {
        let mut weapon = PrimaryWeapon::new();
        let first = weapon.update(Duration::ZERO, HELD_A, geometry());
        assert_eq!(first[0].gun_channel, PrimaryGunChannel::A);
        weapon.update(Duration::ZERO, RELEASED, geometry());

        let upgraded = weapon.update_with_profile(
            Duration::ZERO,
            HELD_A,
            geometry(),
            UPGRADED_PRIMARY_PROFILE,
            PrimaryShotBudget::Limited(1),
        );
        assert_eq!(upgraded[0].gun_channel, PrimaryGunChannel::B);
        assert_eq!(
            upgraded[0].projectile.presentation_basis.origin_world,
            [40.0, 50.0, 60.0]
        );
    }
}
