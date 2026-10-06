//! Retail player-hull collision damage and health state.
//!
//! `FUN_00411760` first resolves a solid model contact, then measures the
//! change between the entity's pre- and post-response signed 8.8 velocity
//! words. The resulting impact packet is filtered through the entity type's
//! Section-12 threshold/multiplier table by `FUN_004255E0`; generic damage
//! application (`FUN_00414E90`) consumes the `entity+0x50` buffer before
//! subtracting the remainder from hull health at `entity+0x30`.
//!
//! The same channel-1 path also follows authored solid-terrain response. Water
//! classification itself does not damage the hull; the accepted surface trace
//! merely shows that its ordinary terrain/seabed contacts remain below the
//! damage threshold. Callers must submit only a velocity pair produced by an
//! authored solid model or terrain response.

use crate::damage::{
    velocity_delta_impact_raw, DamagePacket, DamageProfile, TYPE_46_DAMAGE_PROFILE,
};
use v2k_formats::collision::CollisionEntry;

/// Authored `"Dying bounce:"` control-object duration (`0xBB8`).
pub const PLAYER_DYING_DURATION_MS: u32 = 3_000;
/// `FUN_00446640` submits the hull warning only strictly below this value.
pub const HULL_LOW_WARNING_BELOW_RAW: i32 = 0x1389;
/// `DAT_004DE858` must differ from the shared retail tick by strictly more
/// than `0x3C` before the low-hull warning can replace its presentation slot.
pub const HULL_LOW_WARNING_TICK_GAP: i32 = 0x3c;

/// Authored type-46 values from cumulative Section-12 entry 44. The values are
/// byte-identical across the `?X3XX.OVL` display tiers; `0X3XX` was used only
/// as the compact evidence fixture, not as a visual-art default.
pub const PLAYER_TYPE_46_HULL_PROFILE: HullDamageProfile = HullDamageProfile {
    max_health_raw: 40_000,
    mass_raw: 100,
    damage: TYPE_46_DAMAGE_PROFILE,
};

/// Section-12 values used by the solid-contact damage path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HullDamageProfile {
    /// Initial hull health, header field 5 (`+0x14`).
    pub max_health_raw: i32,
    /// Unsigned entity mass copied from header field 1 (`+0x04`) to `+0xB0`.
    pub mass_raw: u16,
    /// Complete seven-channel packet filter at header `+0x18..+0x4F`.
    pub damage: DamageProfile,
}

impl HullDamageProfile {
    /// Decode the fields read by the retail solid-contact damage path.
    ///
    /// The Section-12 parser retains header words as `u32`; threshold and
    /// multiplier reads are signed in `FUN_004255E0`, so preserve their bit
    /// patterns when converting them here. Entity construction truncates the
    /// authored mass word to the runtime `u16` field.
    pub fn from_type_record(record: &CollisionEntry) -> Self {
        Self {
            max_health_raw: record.dims[0] as i32,
            mass_raw: record.scale as u16,
            damage: DamageProfile::from_section12_header(&record.raw_header)
                .expect("fixed Section-12 header range"),
        }
    }

    /// Compute `FUN_00411760`'s impact value from the velocity immediately
    /// before and after its contact response.
    ///
    /// The executable performs three signed-word subtractions followed by
    /// wrapping 32-bit multiplies/adds and arithmetic shifts. Explicit
    /// wrapping keeps debug and release builds faithful to those x86
    /// operations, including the signed-word seam at `i16::{MIN,MAX}`.
    pub fn impact_raw(self, velocity_before_raw: [i16; 3], velocity_after_raw: [i16; 3]) -> i32 {
        velocity_delta_impact_raw(velocity_before_raw, velocity_after_raw, self.mass_raw)
    }

    /// Apply packet-channel 1's strict threshold and Q8 multiplier.
    pub fn damage_from_impact_raw(self, impact_raw: i32) -> i32 {
        DamagePacket::collision(impact_raw).filtered_raw(Some(&self.damage))
    }

    /// Compute damage from one completed authored solid-contact response.
    pub fn collision_damage_raw(
        self,
        velocity_before_raw: [i16; 3],
        velocity_after_raw: [i16; 3],
    ) -> i32 {
        self.damage_from_impact_raw(self.impact_raw(velocity_before_raw, velocity_after_raw))
    }
}

/// Player hull values stored on the retail entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerHull {
    profile: HullDamageProfile,
    /// `entity+0x30`.
    pub health_raw: i32,
    /// Shield/pre-health damage buffer at `entity+0x50`.
    pub pre_health_damage_buffer_raw: i32,
    /// Retail flag `0x00004000`; death presentation/lifetime is owned by the
    /// entity lifecycle rather than this arithmetic state.
    pub dying: bool,
}

/// Process-global cadence state behind `DAT_004DE858`'s low-hull warning.
///
/// Retail retains this word across player spawns and level changes. The caller
/// must poll with the hull value captured at `FUN_00446640` entry: solid
/// contact occurs later in the world update, so newly inflicted damage becomes
/// eligible on the following controlled-player callback. The executable has no
/// damage-magnitude test; a large hit that leaves entry health `>= 0x1389`
/// never cues.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HullWarningCadence {
    last_tick: i32,
}

impl HullWarningCadence {
    /// Consume the exact callback-entry hull warning condition.
    pub fn poll(&mut self, health_at_callback_entry_raw: i32, tick: i32) -> bool {
        if health_at_callback_entry_raw >= HULL_LOW_WARNING_BELOW_RAW {
            return false;
        }

        let delta = self.last_tick.wrapping_sub(tick);
        let sign = delta >> 31;
        let absolute_delta = (delta ^ sign).wrapping_sub(sign);
        if absolute_delta <= HULL_LOW_WARNING_TICK_GAP {
            return false;
        }

        self.last_tick = tick;
        true
    }

    #[cfg(test)]
    const fn last_tick(self) -> i32 {
        self.last_tick
    }
}

impl Default for PlayerHull {
    fn default() -> Self {
        Self::new(PLAYER_TYPE_46_HULL_PROFILE)
    }
}

impl PlayerHull {
    pub const fn new(profile: HullDamageProfile) -> Self {
        Self {
            profile,
            health_raw: profile.max_health_raw,
            pre_health_damage_buffer_raw: 0,
            dying: false,
        }
    }

    /// Apply the `42E270 ->448070 ->443440` campaign arrival override.
    ///
    /// With the old body gone, controller `+0x290` flag1 replaces health with
    /// 40000. Shield stays in controller `+1BC` and `443560` restores it to
    /// the new entity `+50`; it is not part of the arrival override.
    pub const fn campaign_world_replacement(self, profile: HullDamageProfile) -> Self {
        Self {
            profile,
            health_raw: 40_000,
            pre_health_damage_buffer_raw: self.pre_health_damage_buffer_raw,
            dying: false,
        }
    }

    /// Read back entity-owned damage after synchronous constructor callbacks.
    /// Unknown fields leave their restored controller values intact.
    pub fn readback_entity_damage_state(
        &mut self,
        collision: &crate::entity_collision_state::EntityCollisionRuntimeState,
    ) {
        use crate::entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT};
        if let RetailRuntimeValue::Known(health) = collision.health_raw {
            self.health_raw = health;
        }
        if let RetailRuntimeValue::Known(shield) = collision.pre_health_damage_buffer_raw {
            self.pre_health_damage_buffer_raw = shield;
        }
        if let RetailRuntimeValue::Known(dying) =
            collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
        {
            self.dying = dying != 0;
        }
    }

    /// Section-12 profile attached to the live type-46 entity.
    pub const fn profile(&self) -> HullDamageProfile {
        self.profile
    }

    /// Compute packet-channel 1 damage for one completed solid response.
    pub fn collision_damage_raw(
        &self,
        velocity_before_raw: [i16; 3],
        velocity_after_raw: [i16; 3],
    ) -> i32 {
        self.profile
            .collision_damage_raw(velocity_before_raw, velocity_after_raw)
    }

    /// Apply generic retail damage after packet filtering.
    ///
    /// Non-positive values are ignored because the collision profile never
    /// produces healing. Buffer consumption precedes the DYING check exactly
    /// as it does in `FUN_00414E90`; lethal hull damage is then clamped to zero
    /// by `FUN_00410C10` and raises the dying state once.
    pub fn apply_damage_raw(&mut self, damage_raw: i32) -> HullDamageOutcome {
        if damage_raw <= 0 {
            return HullDamageOutcome::default();
        }

        let absorbed_by_buffer_raw = if self.pre_health_damage_buffer_raw > 0 {
            let absorbed = self.pre_health_damage_buffer_raw.min(damage_raw);
            self.pre_health_damage_buffer_raw -= absorbed;
            absorbed
        } else {
            0
        };
        let unbuffered_damage_raw = damage_raw - absorbed_by_buffer_raw;

        if self.dying || unbuffered_damage_raw == 0 {
            return HullDamageOutcome {
                requested_damage_raw: damage_raw,
                absorbed_by_buffer_raw,
                unbuffered_damage_raw,
                health_lost_raw: 0,
                destroyed_now: false,
            };
        }

        let health_before = self.health_raw;
        let remaining = health_before.wrapping_sub(unbuffered_damage_raw);
        let destroyed_now = remaining < 1;
        if destroyed_now {
            self.health_raw = 0;
            self.dying = true;
        } else {
            self.health_raw = remaining;
        }

        HullDamageOutcome {
            requested_damage_raw: damage_raw,
            absorbed_by_buffer_raw,
            unbuffered_damage_raw,
            health_lost_raw: health_before - self.health_raw,
            destroyed_now,
        }
    }
}

/// Observable arithmetic result of one generic-damage application.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HullDamageOutcome {
    pub requested_damage_raw: i32,
    pub absorbed_by_buffer_raw: i32,
    pub unbuffered_damage_raw: i32,
    pub health_lost_raw: i32,
    pub destroyed_now: bool,
}

/// Type-46's authored `"Dying bounce:"` timed-control state.
///
/// This remains separate from [`PlayerHull`]: damage owns the idempotent HP
/// transition, while the behavior system owns how long the wreck survives and
/// when the player allocation is removed. Control output 7 is the authored
/// `play4ded` model callback value: its type-8 vertices spread the linked wreck
/// pieces while the first child uses the same value as a packed rotation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerDeathLifecycle {
    entered: bool,
    active: bool,
    elapsed_ms: u32,
    control_output_7_raw: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerDeathTick {
    Inactive,
    Dying {
        elapsed_ms: u32,
        control_output_7_raw: u16,
    },
    ExpiredNow {
        elapsed_ms: u32,
        control_output_7_raw: u16,
    },
}

impl PlayerDeathLifecycle {
    /// Attach the authored timed object once. Returns true only on entry.
    pub fn begin(&mut self) -> bool {
        if self.entered {
            return false;
        }
        self.entered = true;
        self.active = true;
        self.elapsed_ms = 0;
        self.control_output_7_raw = 0;
        true
    }

    pub const fn is_active(self) -> bool {
        self.active
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.elapsed_ms
    }

    /// Current authored callback slot 7 consumed by the wreck model.
    pub const fn control_output_7_raw(self) -> u16 {
        self.control_output_7_raw
    }

    /// Advance one retail behavior tick.
    ///
    /// `FUN_00401120` accumulates `dt_us / 1000` and expires only when the
    /// authored duration is strictly less than elapsed. The per-tick callback
    /// independently adds `floor(dt_us / 40)` to control output 7 with a
    /// `0xFFFF` ceiling.
    pub fn advance_micros(&mut self, elapsed_micros: u32) -> PlayerDeathTick {
        if !self.active {
            return PlayerDeathTick::Inactive;
        }

        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1_000);
        self.control_output_7_raw = u32::from(self.control_output_7_raw)
            .saturating_add(elapsed_micros / 40)
            .min(u32::from(u16::MAX)) as u16;

        if PLAYER_DYING_DURATION_MS < self.elapsed_ms {
            self.active = false;
            PlayerDeathTick::ExpiredNow {
                elapsed_ms: self.elapsed_ms,
                control_output_7_raw: self.control_output_7_raw,
            }
        } else {
            PlayerDeathTick::Dying {
                elapsed_ms: self.elapsed_ms,
                control_output_7_raw: self.control_output_7_raw,
            }
        }
    }
}

/// One retail hull-bar smoothing tick from `menu_system.c`.
///
/// Damage lowers the visible value by at least 800 per tick; this can
/// intentionally undershoot the true health. The next tick snaps upward to
/// the true value. Healing also snaps immediately.
pub fn smooth_visible_hull_health_raw(visible_raw: i32, health_raw: i32) -> i32 {
    if health_raw < visible_raw {
        let decrement = (visible_raw.wrapping_sub(health_raw) / 6).max(800);
        visible_raw.wrapping_sub(decrement)
    } else if health_raw > visible_raw {
        health_raw
    } else {
        visible_raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_46_profile_uses_strict_threshold_and_q8_identity_multiplier() {
        let profile = PLAYER_TYPE_46_HULL_PROFILE;
        assert_eq!(profile.damage_from_impact_raw(5_999), 0);
        assert_eq!(profile.damage_from_impact_raw(6_000), 0);
        assert_eq!(profile.damage_from_impact_raw(6_001), 1);
        assert_eq!(profile.damage_from_impact_raw(12_800), 6_800);
    }

    #[test]
    fn impact_preserves_retail_integer_order_and_signed_word_delta() {
        let profile = PLAYER_TYPE_46_HULL_PROFILE;
        assert_eq!(profile.impact_raw([4_096, 0, 0], [0, 0, 0]), 12_800);
        assert_eq!(
            profile.collision_damage_raw([4_096, 0, 0], [0, 0, 0]),
            6_800
        );

        // x86 `sub ax, word` wraps before sign-extension: MAX - MIN is -1,
        // not 65535. Its tiny squared delta rounds below one impact unit.
        assert_eq!(profile.impact_raw([i16::MAX, 0, 0], [i16::MIN, 0, 0]), 0);
    }

    #[test]
    fn generic_damage_consumes_buffer_before_hull() {
        let mut hull = PlayerHull::default();
        hull.pre_health_damage_buffer_raw = 2_000;
        let outcome = hull.apply_damage_raw(3_228);
        assert_eq!(
            outcome,
            HullDamageOutcome {
                requested_damage_raw: 3_228,
                absorbed_by_buffer_raw: 2_000,
                unbuffered_damage_raw: 1_228,
                health_lost_raw: 1_228,
                destroyed_now: false,
            }
        );
        assert_eq!(hull.health_raw, 38_772);
        assert_eq!(hull.pre_health_damage_buffer_raw, 0);
        assert!(!hull.dying);
    }

    #[test]
    fn campaign_replacement_refills_health_and_preserves_controller_shield() {
        let mut source = PlayerHull::default();
        source.health_raw = 12_345;
        source.pre_health_damage_buffer_raw = 80_833;
        source.dying = true;

        let replacement = source.campaign_world_replacement(PLAYER_TYPE_46_HULL_PROFILE);

        assert_eq!(replacement.health_raw, 40_000);
        assert_eq!(replacement.pre_health_damage_buffer_raw, 80_833);
        assert!(!replacement.dying);
        assert_eq!(replacement.profile(), PLAYER_TYPE_46_HULL_PROFILE);
    }

    #[test]
    fn lethal_damage_clamps_health_and_enters_dying_once() {
        let mut hull = PlayerHull::default();
        hull.health_raw = 4_104;
        let outcome = hull.apply_damage_raw(8_667);
        assert_eq!(hull.health_raw, 0);
        assert!(hull.dying);
        assert_eq!(outcome.health_lost_raw, 4_104);
        assert!(outcome.destroyed_now);

        let repeated = hull.apply_damage_raw(1_000);
        assert_eq!(hull.health_raw, 0);
        assert!(!repeated.destroyed_now);
    }

    #[test]
    fn visible_health_preserves_retail_undershoot_then_snap() {
        assert_eq!(smooth_visible_hull_health_raw(1_000, 900), 200);
        assert_eq!(smooth_visible_hull_health_raw(200, 900), 900);
        assert_eq!(smooth_visible_hull_health_raw(40_000, 36_772), 39_200);
    }

    #[test]
    fn low_hull_warning_uses_strict_health_and_sixty_tick_gaps() {
        let mut cadence = HullWarningCadence::default();

        assert!(!cadence.poll(HULL_LOW_WARNING_BELOW_RAW, 1_000));
        assert_eq!(cadence.last_tick(), 0);
        assert!(!cadence.poll(HULL_LOW_WARNING_BELOW_RAW - 1, 60));
        assert!(cadence.poll(HULL_LOW_WARNING_BELOW_RAW - 1, 61));
        assert_eq!(cadence.last_tick(), 61);
        assert!(!cadence.poll(0, 121));
        assert!(cadence.poll(0, 122));
        assert_eq!(cadence.last_tick(), 122);

        // Recovering above the threshold does not consume or reset the
        // process-global cadence word. A still-healthy hull after a large
        // hit likewise never cues: FUN_00446640 has no damage-delta test.
        assert!(!cadence.poll(HULL_LOW_WARNING_BELOW_RAW, 10_000));
        assert_eq!(cadence.last_tick(), 122);
        assert!(!cadence.poll(HULL_LOW_WARNING_BELOW_RAW + 4_144, 10_061));
        assert_eq!(cadence.last_tick(), 122);
    }

    #[test]
    fn death_lifecycle_expires_only_after_three_thousand_integer_milliseconds() {
        let mut death = PlayerDeathLifecycle::default();
        assert!(death.begin());
        assert!(!death.begin(), "death entry is idempotent");
        for _ in 0..150 {
            assert!(matches!(
                death.advance_micros(20_000),
                PlayerDeathTick::Dying { .. }
            ));
        }
        assert_eq!(death.elapsed_ms(), 3_000);
        assert_eq!(
            death.advance_micros(20_000),
            PlayerDeathTick::ExpiredNow {
                elapsed_ms: 3_020,
                control_output_7_raw: u16::MAX,
            }
        );
        assert_eq!(death.advance_micros(20_000), PlayerDeathTick::Inactive);
        assert!(!death.begin(), "expired death cannot re-enter");
    }

    #[test]
    fn death_lifecycle_truncates_each_tick_before_accumulating() {
        let mut death = PlayerDeathLifecycle::default();
        death.begin();
        for _ in 0..3_000 {
            death.advance_micros(999);
        }
        assert_eq!(death.elapsed_ms(), 0);
        assert!(death.is_active());
    }

    #[test]
    fn death_lifecycle_keeps_two_second_crossing_as_an_ordinary_animation_tick() {
        let mut death = PlayerDeathLifecycle::default();
        death.begin();

        assert_eq!(
            death.advance_micros(1_994_000),
            PlayerDeathTick::Dying {
                elapsed_ms: 1_994,
                control_output_7_raw: 49_850,
            }
        );
        assert_eq!(
            death.advance_micros(8_000),
            PlayerDeathTick::Dying {
                elapsed_ms: 2_002,
                control_output_7_raw: 50_050,
            }
        );
        assert!(matches!(
            death.advance_micros(8_000),
            PlayerDeathTick::Dying {
                elapsed_ms: 2_010,
                ..
            }
        ));

        let mut exact_threshold = PlayerDeathLifecycle::default();
        exact_threshold.begin();
        assert!(matches!(
            exact_threshold.advance_micros(2_000_000),
            PlayerDeathTick::Dying {
                elapsed_ms: 2_000,
                ..
            }
        ));
        assert!(matches!(
            exact_threshold.advance_micros(1_000),
            PlayerDeathTick::Dying {
                elapsed_ms: 2_001,
                ..
            }
        ));
    }
}
