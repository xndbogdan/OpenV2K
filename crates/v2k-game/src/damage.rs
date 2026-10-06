//! Retail two-slot damage packets and Section-12 damage filters.
//!
//! `FUN_004255E0` consumes two `(channel, amount)` slots. Each channel selects
//! one of seven signed thresholds and signed Q8 multipliers from the entity
//! type's Section-12 header. An amount contributes only when it is strictly
//! greater than its threshold; both the per-slot arithmetic and the final sum
//! use wrapping signed 32-bit operations.
//!
//! The delivery structure used by surrounding damage code appends two source
//! provenance words at `+0x10` and `+0x14`. They never enter `FUN_004255E0`,
//! so this arithmetic foundation models its exact four-word filter view only.
//! The function's optional post-filter ratio is exposed separately. Retail
//! observes only the numerator's low word and the ABI's unsigned denominator,
//! with wrapping signed multiplication before truncating signed division.

use std::num::NonZeroI32;

/// Number of authored damage-filter channels in every Section-12 type record.
pub const DAMAGE_CHANNEL_COUNT: usize = 7;

/// Source entry point around checked entity damage. These wrappers differ in
/// callback slot, timestamp writes, and their accepted-hit presentation suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityHitEntry {
    /// 10EB0 stamps +34, dispatches DAC0/style+28, then runs its sound/class5 suffix.
    PrimaryProjectile,
    /// 11250 dispatches DA00/style+20 without the primary stamp or suffix.
    Infected,
    /// 11320 clears model bit 0x2000, emits type+84 even while dying, then
    /// dispatches DA60/style+24 without the primary stamp or suffix.
    Cured,
}

/// Authored filter tables for cumulative Section-12 entity type 46.
pub const TYPE_46_DAMAGE_PROFILE: DamageProfile = DamageProfile {
    thresholds_raw: [0, 6_000, 200, 0, 200, 0, 0],
    multipliers_q8: [0, 256, 256, 512, 128, 0, 512],
};

/// Authored filter tables for cumulative Section-12 entity type 9.
///
/// Channel 6 matches type 46 (`512`), so class-5 `FUN_0043F780` packets
/// filter to 4000 against 1500 authored health.
pub const TYPE_9_DAMAGE_PROFILE: DamageProfile = DamageProfile {
    thresholds_raw: [0, 2_000, 400, 0, 200, 0, 0],
    multipliers_q8: [0, 256, 256, 512, 128, 0, 512],
};

/// Raw Section-12 byte offset of channel zero's signed threshold.
const THRESHOLDS_OFFSET: usize = 0x18;
/// Raw Section-12 byte offset of channel zero's signed Q8 multiplier.
const MULTIPLIERS_OFFSET: usize = 0x34;

/// The exact two-slot packet consumed by the retail damage filter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DamagePacket {
    pub channels: [i32; 2],
    pub amounts_raw: [i32; 2],
}

/// Exact six-dword record passed through the retail damage-delivery pipeline.
///
/// [`DamagePacket`] deliberately remains the four-word arithmetic view used by
/// `FUN_004255E0`. The trailing words do not enter that filter, but nested hit,
/// modifier, UI, and network paths can inspect them and therefore cannot be
/// dropped by a complete delivery transaction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DamageDeliveryRecord {
    pub packet: DamagePacket,
    /// Zero-extended source entity type captured by the projectile at birth.
    pub source_entity_type_raw: u32,
    pub owner_handle: u32,
}

impl DamageDeliveryRecord {
    /// FUN_00415040's feedback gate after the damage filter returned zero.
    /// EXE 4150EA/4150EF reads the two channel dwords at +0/+4; neither
    /// amount participates in this exemption. Only player-originated packets
    /// against capability 8 can request the feedback callbacks.
    pub const fn filtered_zero_feedback_required(self, capability_flags: u32) -> bool {
        capability_flags & 8 != 0
            && self.source_entity_type_raw == 0x2e
            && (self.packet.channels[0] != 1 || self.packet.channels[1] != 0)
    }

    /// Preserve the exact signed bit patterns of all six retail dwords.
    pub const fn raw_dwords(self) -> [u32; 6] {
        [
            self.packet.channels[0] as u32,
            self.packet.channels[1] as u32,
            self.packet.amounts_raw[0] as u32,
            self.packet.amounts_raw[1] as u32,
            self.source_entity_type_raw,
            self.owner_handle,
        ]
    }
}

/// Class-1's exact global damage template at retail address `0x004CBF70`.
///
/// `FUN_0043F980` and `FUN_0043FF10` overwrite the two following provenance
/// words before dispatch, but [`DamageProfile`] consumes only these four
/// channel/amount words. Keeping the complete filter view here prevents the
/// first slot's channel number from being mistaken for a generic damage type.
/// Upgraded-primary class 3 selects the adjacent `0x004CBF88` record; its four
/// filter dwords are byte-identical, so both descriptors share this value.
pub const PRIMARY_PROJECTILE_DAMAGE_PACKET: DamagePacket = DamagePacket {
    channels: [2, 0],
    amounts_raw: [2_000, 0],
};

/// Class-4/54 packet at `0x004CBFA0`, consumed by `FUN_0043F6E0`.
///
/// Classes 77/78 share that entity-hit callback with a null `+0x20` word;
/// those rows fail-closed at `FUN_00410EB0` instead of inventing a packet.
pub const FUN_0043F6E0_DAMAGE_PACKET: DamagePacket = DamagePacket {
    channels: [2, 0],
    amounts_raw: [10_000, 0],
};

/// Class-5 packet at `0x004CBFD0`, consumed by `FUN_0043F780` / `FUN_00411250`.
pub const FUN_0043F780_DAMAGE_PACKET: DamagePacket = DamagePacket {
    channels: [6, 0],
    amounts_raw: [2_000, 0],
};

/// F780 forwards the unchanged static record at 004CBFD0 to 411250. Unlike
/// F590/F6E0, it does not write the particle's source byte or owner into the
/// two trailing dwords; both are zero in the retail executable.
pub const FUN_0043F780_DAMAGE_DELIVERY: DamageDeliveryRecord = DamageDeliveryRecord {
    packet: FUN_0043F780_DAMAGE_PACKET,
    source_entity_type_raw: 0,
    owner_handle: 0,
};

/// Class-6 Antidote packet at 004CBFE8. F7C0 forwards the unchanged static
/// six-dword record to 11320; the particle's birth provenance is not copied.
pub const FUN_0043F7C0_DAMAGE_PACKET: DamagePacket = DamagePacket {
    channels: [2, 0],
    amounts_raw: [1_000, 0],
};

pub const FUN_0043F7C0_DAMAGE_DELIVERY: DamageDeliveryRecord = DamageDeliveryRecord {
    packet: FUN_0043F7C0_DAMAGE_PACKET,
    source_entity_type_raw: 0,
    owner_handle: 0,
};

/// Exact packet at retail address `0x004CC000`, shared by every particle
/// descriptor whose mode-3 collision callbacks are `FUN_0043F590` and
/// `FUN_0043F800`.
///
/// The two following dwords in the six-dword delivery record are filled with
/// the particle's immutable source-type byte and owner handle at impact time;
/// they are provenance and do not enter [`DamageProfile::filter`].
pub const BALLISTIC_PARTICLE_DAMAGE_PACKET: DamagePacket = DamagePacket {
    channels: [1, 0],
    amounts_raw: [2_500, 0],
};

/// Class38's F590/F800 packet at retail 0x004CC048.
pub const DRAGON_FIREBALL_DAMAGE_PACKET: DamagePacket = DamagePacket {
    channels: [2, 3],
    amounts_raw: [500, 6000],
};

/// Class49/80's F590/F800 packet at retail0x004CC078. Method12 uses
///this bolt row; it is distinct from method14's class55/81 packet.
pub const CLASS49_TURRET_BOLT_DAMAGE_PACKET: DamagePacket = DamagePacket {
    channels: [2, 3],
    amounts_raw: [6000, 2000],
};

/// Class55/81's F590/F800 packet at retail 0x004CC0A8.
pub const TURRET_BOLT_DAMAGE_PACKET: DamagePacket = DamagePacket {
    channels: [2, 3],
    amounts_raw: [3000, 1000],
};

/// Green plasma class56/82's F590/F800 packet at retail `0x004CC090`.
/// Method13 retains this distinct row through both sides of its water pair.
pub const CLASS56_TURRET_BOLT_DAMAGE_PACKET: DamagePacket = DamagePacket {
    channels: [2, 3],
    amounts_raw: [4000, 1500],
};

/// Type-47's class-87 projectile damage template at retail address
/// `0x004CC0F0`.
///
/// Class 87's descriptor directly selects `FUN_0043F590`; after its common
/// contact visual, that callback forwards this complete two-slot packet to the
/// ordinary entity damage path unless particle byte `+0x1D` bit zero is set.
/// `FUN_00442950` belongs to another particle descriptor. In particular, the
/// player hull's resulting damage must continue to come from
/// [`TYPE_46_DAMAGE_PROFILE`] instead of being cached as one magic post-filter
/// amount.
pub const TYPE_47_PROJECTILE_DAMAGE_PACKET: DamagePacket = DamagePacket {
    channels: [2, 6],
    amounts_raw: [1_000, 1_000],
};

/// Class68's `FUN_00442950` packet at retail `0x004CC108`.
///
/// Recovered by a read-only PE section read of the retail executable,
/// cross-validated against the seven documented packet rows at `0x004CBF70`,
/// `0x004CBFA0`, `0x004CBFD0`, `0x004CC000`, `0x004CC048`, `0x004CC0A8` and
/// `0x004CC0F0` (all reproduce exactly, including zero provenance words).
/// `FUN_00441180` sums channels 1/2 via `FUN_00425590` (2,000 here) and
/// strikes `FUN_00411030` with eight times that amount; the Section-12 filter
/// consumes the full packet unchanged.
pub const CLASS68_STATIC_ROUTE_DAMAGE_PACKET: DamagePacket = DamagePacket {
    channels: [2, 6],
    amounts_raw: [2_000, 2_000],
};

impl DamagePacket {
    /// Solid-contact packet built by `FUN_00411760`.
    pub const fn collision(impact_raw: i32) -> Self {
        Self {
            channels: [1, 0],
            amounts_raw: [impact_raw, 0],
        }
    }

    /// Retail's no-profile branch returns the signed wrapping sum of the two
    /// amounts without consulting their channels.
    pub const fn sum_raw(self) -> i32 {
        self.amounts_raw[0].wrapping_add(self.amounts_raw[1])
    }

    /// Exact impact magnitude selected by retail `FUN_00425590`.
    ///
    /// Only packet channels 1 and 2 contribute to the pre-damage impact
    /// callback and `FUN_00411030` reaction. The two signed amounts are added
    /// in packet order with 32-bit wrapping; every other channel is ignored.
    /// This is deliberately separate from [`Self::sum_raw`] and the
    /// Section-12 filter because all three consumers have different channel
    /// semantics.
    pub const fn impact_sum_raw(self) -> i32 {
        let mut sum = 0_i32;
        if self.channels[0] == 1 || self.channels[0] == 2 {
            sum = sum.wrapping_add(self.amounts_raw[0]);
        }
        if self.channels[1] == 1 || self.channels[1] == 2 {
            sum = sum.wrapping_add(self.amounts_raw[1]);
        }
        sum
    }

    /// Complete `FUN_004255E0` profile-pointer branch.
    pub fn filtered_raw(self, profile: Option<&DamageProfile>) -> i32 {
        profile.map_or_else(|| self.sum_raw(), |profile| profile.filter(self))
    }

    /// Complete `FUN_004255E0` result including its optional ratio suffix.
    ///
    /// A zero denominator skips the ratio entirely, including any numerator.
    /// Otherwise retail wraps the signed 32-bit product of the filtered value
    /// and `ratio_numerator & 0xFFFF`, then divides by the positive
    /// zero-extended 16-bit denominator with truncation toward zero.
    pub fn filtered_raw_with_ratio(
        self,
        profile: Option<&DamageProfile>,
        ratio_numerator: u32,
        ratio_denominator: u16,
    ) -> i32 {
        let filtered_raw = self.filtered_raw(profile);
        if ratio_denominator == 0 {
            return filtered_raw;
        }
        filtered_raw.wrapping_mul((ratio_numerator & 0xFFFF) as i32) / i32::from(ratio_denominator)
    }
}

/// One entity type's complete seven-channel Section-12 damage filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageProfile {
    pub thresholds_raw: [i32; DAMAGE_CHANNEL_COUNT],
    pub multipliers_q8: [i32; DAMAGE_CHANNEL_COUNT],
}

/// One target in `FUN_00414D30`'s ordered pair-damage cap.
///
/// The cap consults live health but deliberately does not consult the
/// pre-health buffer consumed later by `FUN_00414E90`.
#[derive(Debug, Clone, Copy)]
pub struct PairDamageCapTarget<'a> {
    pub health_raw: i32,
    pub profile: &'a DamageProfile,
}

/// Pure arithmetic state entering retail's local generic-damage path.
///
/// This is the state after packet filtering and any entity `+0x44` modifier.
/// Remote-owned entities (`state & 0x8000_0000`) do not enter this path and
/// must be forwarded by their caller instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericEntityDamageState {
    pub health_raw: i32,
    pub pre_health_buffer_raw: i32,
    pub already_dying: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericEntityDamageStage {
    AlreadyDying,
    Survived,
    DeathDispatchRequired,
}

/// Exact arithmetic result of the local portion of `FUN_00414E90`.
///
/// This does not play the authored hit sound, invoke the type hit callback, or
/// dispatch death. [`Self::hit_effect_damage_raw`] tells the lifecycle owner
/// what value retail passes to those effects; [`GenericEntityDamageStage::DeathDispatchRequired`]
/// is a hard handoff boundary rather than permission to remove the entity
/// generically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericEntityDamageTransition {
    pub requested_damage_raw: i32,
    pub health_before_raw: i32,
    pub health_after_subtraction_raw: i32,
    pub pre_health_buffer_before_raw: i32,
    pub pre_health_buffer_after_raw: i32,
    pub damage_after_buffer_raw: i32,
    pub stage: GenericEntityDamageStage,
}

/// Pure `FUN_00410D30` presentation result after checked damage and all
/// target-owned death callbacks have completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceptedHitPresentationTransition {
    pub last_tick_before: u32,
    pub last_tick_after: u32,
    /// Positional cue to stage. The throttle tick can advance without a cue
    /// when the selector is zero or the final callback state remains dying.
    pub sound_id: Option<u16>,
}

/// Pure `FUN_00410EB0` projectile-hit presentation result after its checked
/// damage call and target-owned death callbacks have completed.
///
/// Unlike [`AcceptedHitPresentationTransition`], this wrapper has no cadence
/// gate: retail writes entity `+0x34` before impact callbacks and checked
/// damage, then requests the authored `+0x80` cue on every accepted nonzero hit
/// whose final state is not dying.
///
/// The historical Rust type name predates recovery of class 87's use of this
/// same downstream checked-delivery wrapper; it is not an allocator/request
/// boundary specific to the player's primary gun.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimaryProjectileHitPresentationTransition {
    pub last_tick_after: u32,
    pub sound_id: Option<u16>,
}

/// Fully known state entering the checked local-damage survivor transaction.
///
/// The caller must first prove that `FUN_00415040` accepted a nonzero filtered
/// value and that the target's optional `+0x44` modifier is
/// an audited null pointer. Keeping that callback policy outside this value
/// makes it impossible to mistake an unresolved callback for identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckedLocalDamageState {
    pub generic: GenericEntityDamageState,
    pub generic_hit_sound_id: Option<u16>,
}

/// Atomic survivor result of the checked local-damage chain.
///
/// This owns only `FUN_00414E90`'s generic-hit output. Outer hit wrappers own
/// their independent accepted-hit presentation and cadence policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckedLocalDamageApplied {
    pub accepted_damage_raw: i32,
    pub generic: GenericEntityDamageTransition,
    pub generic_hit_sound_id: Option<u16>,
}

/// Pure result after an audited-null checked-damage modifier.
///
/// Only [`Self::Survived`] is a complete state transaction. Dying and lethal
/// targets require lifecycle callbacks that are deliberately outside this
/// bounded core, so callers must not apply either transition piecemeal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckedLocalDamageTransition {
    Survived(CheckedLocalDamageApplied),
    AlreadyDying {
        generic: GenericEntityDamageTransition,
    },
    DeathDispatchRequired {
        generic: GenericEntityDamageTransition,
    },
}

/// Apply the accepted-hit wrapper's exact nonzero/cadence/final-state gates.
///
/// The executable compares unsigned words as `last_tick < tick - 10`; using a
/// wrapping subtraction preserves that behavior through the 32-bit clock seam.
/// Once accepted, `+0x34` is updated before the final dying-state/sound check.
pub fn accepted_hit_presentation_transition(
    last_tick: u32,
    retail_tick: u32,
    accepted_damage_raw: i32,
    final_dying: bool,
    authored_sound_id: Option<u16>,
) -> AcceptedHitPresentationTransition {
    let cadence_accepts = last_tick < retail_tick.wrapping_sub(10);
    if accepted_damage_raw == 0 || !cadence_accepts {
        return AcceptedHitPresentationTransition {
            last_tick_before: last_tick,
            last_tick_after: last_tick,
            sound_id: None,
        };
    }
    AcceptedHitPresentationTransition {
        last_tick_before: last_tick,
        last_tick_after: retail_tick,
        sound_id: if final_dying { None } else { authored_sound_id },
    }
}

/// Apply `FUN_00410EB0`'s downstream unconditional tick stamp and final sound
/// gate.
///
/// The caller invokes this only after `FUN_00415040` returned nonzero. The
/// tick is nevertheless modeled independently because retail writes it before
/// checked delivery; callers that stop at an unresolved death lifecycle must
/// not apply this result piecemeal.
pub const fn primary_projectile_hit_presentation_transition(
    retail_tick: u32,
    final_dying: bool,
    authored_sound_id: Option<u16>,
) -> PrimaryProjectileHitPresentationTransition {
    PrimaryProjectileHitPresentationTransition {
        last_tick_after: retail_tick,
        sound_id: if final_dying { None } else { authored_sound_id },
    }
}

/// Continue `FUN_00415040` after its nonzero filter and an explicitly
/// audited null `+0x44` modifier.
///
/// `accepted_damage_raw` is the filtered damage that the absent modifier
/// leaves unchanged, not the post-buffer health loss. Outer wrappers therefore
/// still accept a hit whose pre-health buffer absorbs the complete packet. The
/// target type's ordinary hit callback and accepted-hit presentation are
/// independent prerequisites owned by the caller; this arithmetic function
/// performs no callback dispatch.
///
/// A present modifier is outside this bounded helper. In particular, retail
/// still runs the generic damage lifecycle when that callback returns zero;
/// zero suppresses only the outer accepted-hit presentation after the generic
/// helper has unwound.
pub fn checked_local_damage_transition_after_null_modifier(
    state: CheckedLocalDamageState,
    accepted_damage_raw: NonZeroI32,
) -> CheckedLocalDamageTransition {
    let accepted_damage_raw = accepted_damage_raw.get();
    let generic = generic_entity_damage_transition(state.generic, accepted_damage_raw);
    match generic.stage {
        GenericEntityDamageStage::AlreadyDying => {
            CheckedLocalDamageTransition::AlreadyDying { generic }
        }
        GenericEntityDamageStage::DeathDispatchRequired => {
            CheckedLocalDamageTransition::DeathDispatchRequired { generic }
        }
        GenericEntityDamageStage::Survived => {
            CheckedLocalDamageTransition::Survived(CheckedLocalDamageApplied {
                accepted_damage_raw,
                generic,
                generic_hit_sound_id: state.generic_hit_sound_id,
            })
        }
    }
}

impl GenericEntityDamageTransition {
    /// Retail requests the nonzero type `+0x98` hit sound and invokes a
    /// non-null type hit callback on every non-dying path, even when this value
    /// is zero.
    pub const fn hit_effect_damage_raw(self) -> Option<i32> {
        match self.stage {
            GenericEntityDamageStage::AlreadyDying => None,
            GenericEntityDamageStage::Survived
            | GenericEntityDamageStage::DeathDispatchRequired => Some(self.damage_after_buffer_raw),
        }
    }
}

impl DamageProfile {
    /// Decode the signed threshold and multiplier tables from an exact raw
    /// Section-12 header. Returns `None` only for a truncated diagnostic
    /// buffer; parsed [`v2k_formats::collision::CollisionEntry`] headers are
    /// always large enough.
    pub fn from_section12_header(raw_header: &[u8]) -> Option<Self> {
        Some(Self {
            thresholds_raw: read_i32_table(raw_header, THRESHOLDS_OFFSET)?,
            multipliers_q8: read_i32_table(raw_header, MULTIPLIERS_OFFSET)?,
        })
    }

    /// Apply `FUN_004255E0`'s strict signed threshold and Q8 filter.
    ///
    /// Authored packet channels are in `0..7`; direct indexing deliberately
    /// keeps an invalid reconstructed packet visible instead of silently
    /// treating it as a different retail channel.
    pub fn filter(self, packet: DamagePacket) -> i32 {
        packet.channels.into_iter().zip(packet.amounts_raw).fold(
            0_i32,
            |sum, (channel, amount_raw)| {
                let channel = usize::try_from(channel)
                    .expect("retail damage-packet channels must be nonnegative");
                let threshold_raw = self.thresholds_raw[channel];
                if amount_raw <= threshold_raw {
                    return sum;
                }
                let contribution = self.multipliers_q8[channel]
                    .wrapping_mul(amount_raw.wrapping_sub(threshold_raw))
                    >> 8;
                sum.wrapping_add(contribution)
            },
        )
    }
}

/// Apply `FUN_00414D30`'s ordered overkill cap to one shared raw amount.
///
/// Each target filters the amount independently. If that filtered value is
/// more than `health + 1`, retail converts only the excess back through the
/// selected channel's Q8 multiplier and subtracts it from the shared raw
/// amount before consulting the next target. The final signed amount is
/// clamped to zero. Arithmetic deliberately wraps in the same order as the
/// 32-bit executable.
pub fn cap_pair_damage_raw(
    channel: i32,
    amount_raw: i32,
    ordered_targets: [PairDamageCapTarget<'_>; 2],
) -> i32 {
    let mut amount_raw = amount_raw;
    for (target_index, target) in ordered_targets.into_iter().enumerate() {
        // Retail always evaluates the subject. Only the candidate is gated by
        // the remaining raw amount being positive.
        if target_index != 0 && amount_raw <= 0 {
            break;
        }
        let packet = DamagePacket {
            channels: [channel, 0],
            amounts_raw: [amount_raw, 0],
        };
        let filtered_raw = target.profile.filter(packet);
        let excess_raw = filtered_raw.wrapping_sub(target.health_raw).wrapping_sub(1);
        if excess_raw > 0 {
            amount_raw = amount_raw.wrapping_sub(inverse_channel_damage_raw(
                excess_raw,
                channel,
                target.profile,
            ));
        }
    }
    amount_raw.max(0)
}

fn inverse_channel_damage_raw(
    filtered_excess_raw: i32,
    channel: i32,
    profile: &DamageProfile,
) -> i32 {
    let channel =
        usize::try_from(channel).expect("retail damage-packet channels must be nonnegative");
    let multiplier_q8 = profile.multipliers_q8[channel];
    if multiplier_q8 == 0 {
        filtered_excess_raw
    } else {
        filtered_excess_raw
            .wrapping_shl(8)
            .wrapping_div(multiplier_q8)
    }
}

/// Compute the local buffer/health transition in `FUN_00414E90`.
///
/// Positive buffer is consumed before the dying-state check. A dying entity
/// therefore still loses buffer, but receives no hit callback and no health
/// subtraction. A live entity invokes its hit callback even when the buffer
/// absorbs the complete packet. Lethal health is left at its wrapping
/// post-subtraction value because `FUN_00410C10` owns the subsequent zeroing,
/// dying flag, sound, and behavior transition.
pub fn generic_entity_damage_transition(
    state: GenericEntityDamageState,
    damage_raw: i32,
) -> GenericEntityDamageTransition {
    let (pre_health_buffer_raw, unbuffered_damage_raw) =
        absorb_pre_health_damage_buffer(state.pre_health_buffer_raw, damage_raw);

    if state.already_dying {
        return GenericEntityDamageTransition {
            requested_damage_raw: damage_raw,
            health_before_raw: state.health_raw,
            health_after_subtraction_raw: state.health_raw,
            pre_health_buffer_before_raw: state.pre_health_buffer_raw,
            pre_health_buffer_after_raw: pre_health_buffer_raw,
            damage_after_buffer_raw: unbuffered_damage_raw,
            stage: GenericEntityDamageStage::AlreadyDying,
        };
    }

    let health_raw = state.health_raw.wrapping_sub(unbuffered_damage_raw);
    GenericEntityDamageTransition {
        requested_damage_raw: damage_raw,
        health_before_raw: state.health_raw,
        health_after_subtraction_raw: health_raw,
        pre_health_buffer_before_raw: state.pre_health_buffer_raw,
        pre_health_buffer_after_raw: pre_health_buffer_raw,
        damage_after_buffer_raw: unbuffered_damage_raw,
        stage: if health_raw < 1 {
            GenericEntityDamageStage::DeathDispatchRequired
        } else {
            GenericEntityDamageStage::Survived
        },
    }
}

/// `FUN_00414E90` commits this buffer phase before reading the dying bit,
/// invoking the generic hit callback, or sampling health.
pub(crate) fn absorb_pre_health_damage_buffer(buffer_raw: i32, damage_raw: i32) -> (i32, i32) {
    if buffer_raw > 0 {
        let remaining_buffer = buffer_raw.wrapping_sub(damage_raw);
        if remaining_buffer < 0 {
            (0, remaining_buffer.wrapping_neg())
        } else {
            (remaining_buffer, 0)
        }
    } else {
        (buffer_raw, damage_raw)
    }
}

fn read_i32_table(raw_header: &[u8], offset: usize) -> Option<[i32; DAMAGE_CHANNEL_COUNT]> {
    let mut values = [0_i32; DAMAGE_CHANNEL_COUNT];
    for (index, value) in values.iter_mut().enumerate() {
        let start = offset.checked_add(index.checked_mul(4)?)?;
        *value = i32::from_le_bytes(raw_header.get(start..start + 4)?.try_into().ok()?);
    }
    Some(values)
}

/// `FUN_00411760`'s common impact magnitude from signed 8.8 velocity words.
///
/// Each subtraction wraps as an `i16` before sign extension. Squaring,
/// summing, mass multiplication, and shifts then follow the executable's
/// signed wrapping 32-bit order exactly.
pub fn velocity_delta_impact_raw(
    velocity_before_raw: [i16; 3],
    velocity_after_raw: [i16; 3],
    mass_raw: u16,
) -> i32 {
    let delta_x = i32::from(velocity_before_raw[0].wrapping_sub(velocity_after_raw[0]));
    let delta_y = i32::from(velocity_before_raw[1].wrapping_sub(velocity_after_raw[1]));
    let delta_z = i32::from(velocity_before_raw[2].wrapping_sub(velocity_after_raw[2]));
    let squared_delta = delta_x
        .wrapping_mul(delta_x)
        .wrapping_add(delta_y.wrapping_mul(delta_y))
        .wrapping_add(delta_z.wrapping_mul(delta_z));
    (squared_delta >> 7).wrapping_mul(i32::from(mass_raw)) >> 10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fun_0043f780_packet_channel_6_filters_type_46_and_has_zero_impact_sum() {
        assert_eq!(FUN_0043F780_DAMAGE_PACKET.channels, [6, 0]);
        assert_eq!(FUN_0043F780_DAMAGE_PACKET.amounts_raw, [2_000, 0]);
        assert_eq!(FUN_0043F780_DAMAGE_PACKET.impact_sum_raw(), 0);
        assert_eq!(
            TYPE_46_DAMAGE_PROFILE.filter(FUN_0043F780_DAMAGE_PACKET),
            4_000
        );
        assert_eq!(
            TYPE_9_DAMAGE_PROFILE.filter(FUN_0043F780_DAMAGE_PACKET),
            4_000
        );
    }

    #[test]
    fn type_46_radial_packet_filters_both_slots_and_uses_wrapping_arithmetic() {
        assert_eq!(
            TYPE_46_DAMAGE_PROFILE.filter(DamagePacket {
                channels: [1, 3],
                amounts_raw: [1_000, 1_000],
            }),
            2_000
        );

        let wrapping = DamageProfile {
            thresholds_raw: [0; DAMAGE_CHANNEL_COUNT],
            multipliers_q8: [256; DAMAGE_CHANNEL_COUNT],
        };
        // The first Q8 product wraps to -256 before shifting (-1); the
        // second contributes +1, so the signed wrapping sum returns zero.
        assert_eq!(
            wrapping.filter(DamagePacket {
                channels: [1, 3],
                amounts_raw: [i32::MAX, 1],
            }),
            0
        );
    }

    #[test]
    fn checked_filter_ratio_uses_low_word_wrapping_product_and_signed_division() {
        let packet = DamagePacket {
            channels: [0, 0],
            amounts_raw: [0x4000_0001, 0],
        };
        assert_eq!(
            packet.filtered_raw_with_ratio(None, 0xABCD_0003, 2),
            -536_870_910
        );

        let negative = DamagePacket {
            channels: [0, 0],
            amounts_raw: [-101, 0],
        };
        assert_eq!(negative.filtered_raw_with_ratio(None, 0xFFFF_0002, 3), -67);
    }

    #[test]
    fn zero_checked_filter_denominator_skips_the_complete_ratio_suffix() {
        let packet = DamagePacket {
            channels: [0, 0],
            amounts_raw: [-321, 0],
        };
        assert_eq!(packet.filtered_raw_with_ratio(None, 0, 0), -321);
        assert_eq!(packet.filtered_raw_with_ratio(None, u32::MAX, 0), -321);
        assert_eq!(packet.filtered_raw_with_ratio(None, 0, u16::MAX), 0);
    }

    #[test]
    fn type_46_collision_threshold_is_strict() {
        assert_eq!(
            TYPE_46_DAMAGE_PROFILE.filter(DamagePacket::collision(6_000)),
            0
        );
        assert_eq!(
            TYPE_46_DAMAGE_PROFILE.filter(DamagePacket::collision(6_001)),
            1
        );
    }

    #[test]
    fn primary_projectile_packet_preserves_both_retail_slots() {
        assert_eq!(
            PRIMARY_PROJECTILE_DAMAGE_PACKET,
            DamagePacket {
                channels: [2, 0],
                amounts_raw: [2_000, 0],
            }
        );
    }

    #[test]
    fn delivery_record_preserves_all_six_retail_dword_bit_patterns() {
        let delivery = DamageDeliveryRecord {
            packet: DamagePacket {
                channels: [0x89ab_cdef_u32 as i32, -1],
                amounts_raw: [i32::MIN, i32::MAX],
            },
            source_entity_type_raw: 0x0000_002f,
            owner_handle: 0x04aa_0001,
        };

        assert_eq!(
            delivery.raw_dwords(),
            [
                0x89ab_cdef,
                0xffff_ffff,
                0x8000_0000,
                0x7fff_ffff,
                0x0000_002f,
                0x04aa_0001,
            ]
        );
    }

    #[test]
    fn impact_sum_selects_only_channels_one_and_two_with_signed_wrapping() {
        assert_eq!(
            DamagePacket {
                channels: [1, 2],
                amounts_raw: [i32::MAX, 1],
            }
            .impact_sum_raw(),
            i32::MIN
        );
        assert_eq!(
            DamagePacket {
                channels: [6, 2],
                amounts_raw: [12_345, -77],
            }
            .impact_sum_raw(),
            -77
        );
        assert_eq!(
            DamagePacket {
                channels: [0, 7],
                amounts_raw: [i32::MAX, i32::MIN],
            }
            .impact_sum_raw(),
            0
        );
    }

    #[test]
    fn type_47_projectile_packet_preserves_both_slots_and_filters_per_target() {
        assert_eq!(
            TYPE_47_PROJECTILE_DAMAGE_PACKET,
            DamagePacket {
                channels: [2, 6],
                amounts_raw: [1_000, 1_000],
            }
        );
        assert_eq!(
            TYPE_47_PROJECTILE_DAMAGE_PACKET.filtered_raw(Some(&TYPE_46_DAMAGE_PROFILE)),
            (1_000 - TYPE_46_DAMAGE_PROFILE.thresholds_raw[2])
                * TYPE_46_DAMAGE_PROFILE.multipliers_q8[2]
                / 256
                + (1_000 - TYPE_46_DAMAGE_PROFILE.thresholds_raw[6])
                    * TYPE_46_DAMAGE_PROFILE.multipliers_q8[6]
                    / 256
        );
    }

    #[test]
    fn accepted_hit_presentation_uses_strict_wrapping_tick_gate() {
        let at_ten = accepted_hit_presentation_transition(90, 100, 1, false, Some(7));
        assert_eq!(
            at_ten,
            AcceptedHitPresentationTransition {
                last_tick_before: 90,
                last_tick_after: 90,
                sound_id: None,
            }
        );
        assert_eq!(
            accepted_hit_presentation_transition(89, 100, 1, false, Some(7)),
            AcceptedHitPresentationTransition {
                last_tick_before: 89,
                last_tick_after: 100,
                sound_id: Some(7),
            }
        );

        // tick 5 is eleven wrapping ticks after u32::MAX - 5.
        assert_eq!(
            accepted_hit_presentation_transition(u32::MAX - 5, 5, 1, false, Some(7)).sound_id,
            Some(7)
        );
        assert_eq!(
            accepted_hit_presentation_transition(u32::MAX - 4, 5, 1, false, Some(7)).sound_id,
            None,
            "ten wrapping ticks is still throttled"
        );
    }

    #[test]
    fn accepted_hit_updates_tick_before_final_sound_gate() {
        assert_eq!(
            accepted_hit_presentation_transition(0, 11, 1, true, Some(7)),
            AcceptedHitPresentationTransition {
                last_tick_before: 0,
                last_tick_after: 11,
                sound_id: None,
            },
            "a persistent dying state suppresses sound but not the accepted tick"
        );
        assert_eq!(
            accepted_hit_presentation_transition(0, 11, 1, false, None),
            AcceptedHitPresentationTransition {
                last_tick_before: 0,
                last_tick_after: 11,
                sound_id: None,
            },
            "an authored zero selector still advances the throttle"
        );
        assert_eq!(
            accepted_hit_presentation_transition(0, 11, 0, false, Some(7)),
            AcceptedHitPresentationTransition {
                last_tick_before: 0,
                last_tick_after: 0,
                sound_id: None,
            },
            "a modifier result of zero suppresses the complete wrapper"
        );
    }

    #[test]
    fn primary_projectile_presentation_has_no_cadence_gate() {
        assert_eq!(
            primary_projectile_hit_presentation_transition(100, false, Some(7)),
            PrimaryProjectileHitPresentationTransition {
                last_tick_after: 100,
                sound_id: Some(7),
            }
        );
        assert_eq!(
            primary_projectile_hit_presentation_transition(101, false, Some(7)),
            PrimaryProjectileHitPresentationTransition {
                last_tick_after: 101,
                sound_id: Some(7),
            },
            "consecutive accepted bullets are not throttled by FUN_00410D30's separate cadence"
        );
        assert_eq!(
            primary_projectile_hit_presentation_transition(102, true, Some(7)),
            PrimaryProjectileHitPresentationTransition {
                last_tick_after: 102,
                sound_id: None,
            }
        );
    }

    #[test]
    fn checked_local_survivor_keeps_generic_output_outside_hit_wrappers() {
        let state = CheckedLocalDamageState {
            generic: GenericEntityDamageState {
                health_raw: 5_000,
                pre_health_buffer_raw: 1_000,
                already_dying: false,
            },
            generic_hit_sound_id: Some(9),
        };
        let CheckedLocalDamageTransition::Survived(applied) =
            checked_local_damage_transition_after_null_modifier(
                state,
                NonZeroI32::new(1_800).unwrap(),
            )
        else {
            panic!("nonlethal checked damage must produce a survivor transaction");
        };
        assert_eq!(applied.accepted_damage_raw, 1_800);
        assert_eq!(applied.generic.pre_health_buffer_after_raw, 0);
        assert_eq!(applied.generic.damage_after_buffer_raw, 800);
        assert_eq!(applied.generic.health_after_subtraction_raw, 4_200);
        assert_eq!(applied.generic_hit_sound_id, Some(9));
    }

    #[test]
    fn checked_local_lethal_transition_stops_before_accepted_hit_presentation() {
        let transition = checked_local_damage_transition_after_null_modifier(
            CheckedLocalDamageState {
                generic: GenericEntityDamageState {
                    health_raw: 1_800,
                    pre_health_buffer_raw: 0,
                    already_dying: false,
                },
                generic_hit_sound_id: None,
            },
            NonZeroI32::new(1_800).unwrap(),
        );
        assert!(matches!(
            transition,
            CheckedLocalDamageTransition::DeathDispatchRequired { generic }
                if generic.health_after_subtraction_raw == 0
        ));
    }

    #[test]
    fn absent_profile_uses_the_packets_signed_wrapping_sum() {
        let packet = DamagePacket {
            channels: [6, 1],
            amounts_raw: [i32::MAX, 1],
        };
        assert_eq!(packet.sum_raw(), i32::MIN);
        assert_eq!(packet.filtered_raw(None), i32::MIN);
    }

    #[test]
    fn velocity_delta_wraps_at_the_signed_word_seam() {
        assert_eq!(
            velocity_delta_impact_raw([4_096, 0, 0], [0, 0, 0], 100),
            12_800
        );
        assert_eq!(
            velocity_delta_impact_raw([i16::MAX, 0, 0], [i16::MIN, 0, 0], 100),
            0
        );
    }

    #[test]
    fn section12_header_decodes_all_seven_signed_channels() {
        let thresholds: [i32; DAMAGE_CHANNEL_COUNT] = [-1, 6_000, 200, 0, 200, i32::MIN, i32::MAX];
        let multipliers: [i32; DAMAGE_CHANNEL_COUNT] = [0, 256, 256, 512, 128, -256, 512];
        let mut raw_header = [0_u8; 0x128];
        for (index, value) in thresholds.into_iter().enumerate() {
            let start = THRESHOLDS_OFFSET + index * 4;
            raw_header[start..start + 4].copy_from_slice(&value.to_le_bytes());
        }
        for (index, value) in multipliers.into_iter().enumerate() {
            let start = MULTIPLIERS_OFFSET + index * 4;
            raw_header[start..start + 4].copy_from_slice(&value.to_le_bytes());
        }

        assert_eq!(
            DamageProfile::from_section12_header(&raw_header),
            Some(DamageProfile {
                thresholds_raw: thresholds,
                multipliers_q8: multipliers,
            })
        );
        assert_eq!(
            DamageProfile::from_section12_header(&raw_header[..0x4f]),
            None
        );
    }

    fn single_channel_profile(
        channel: usize,
        threshold_raw: i32,
        multiplier_q8: i32,
    ) -> DamageProfile {
        let mut profile = DamageProfile {
            thresholds_raw: [0; DAMAGE_CHANNEL_COUNT],
            multipliers_q8: [0; DAMAGE_CHANNEL_COUNT],
        };
        profile.thresholds_raw[channel] = threshold_raw;
        profile.multipliers_q8[channel] = multiplier_q8;
        profile
    }

    #[test]
    fn pair_cap_preserves_the_intentional_health_plus_one_boundary() {
        let identity = single_channel_profile(1, 0, 256);
        let targets = [
            PairDamageCapTarget {
                health_raw: 100,
                profile: &identity,
            },
            PairDamageCapTarget {
                health_raw: i32::MAX,
                profile: &identity,
            },
        ];
        assert_eq!(cap_pair_damage_raw(1, 101, targets), 101);
        assert_eq!(cap_pair_damage_raw(1, 102, targets), 101);

        let harmless_second = PairDamageCapTarget {
            health_raw: i32::MAX,
            profile: &TYPE_46_DAMAGE_PROFILE,
        };
        let capped = cap_pair_damage_raw(
            1,
            50_000,
            [
                PairDamageCapTarget {
                    health_raw: 40_000,
                    profile: &TYPE_46_DAMAGE_PROFILE,
                },
                harmless_second,
            ],
        );
        assert_eq!(capped, 46_001);
        assert_eq!(
            DamagePacket::collision(capped).filtered_raw(Some(&TYPE_46_DAMAGE_PROFILE)),
            40_001
        );
    }

    #[test]
    fn pair_cap_retains_target_order_truncation_and_zero_multiplier_guard() {
        let half = single_channel_profile(1, 0, 128);
        let three_quarters = single_channel_profile(1, 1, 192);
        let a = PairDamageCapTarget {
            health_raw: 0,
            profile: &half,
        };
        let b = PairDamageCapTarget {
            health_raw: 0,
            profile: &three_quarters,
        };
        assert_eq!(cap_pair_damage_raw(1, 4, [a, b]), 2);
        assert_eq!(cap_pair_damage_raw(1, 4, [b, a]), 3);

        let zero_multiplier = single_channel_profile(1, 0, 0);
        let target = PairDamageCapTarget {
            health_raw: -5,
            profile: &zero_multiplier,
        };
        assert_eq!(cap_pair_damage_raw(1, 10, [target, target]), 2);

        let amplified = single_channel_profile(1, 0, 1);
        let target = PairDamageCapTarget {
            health_raw: -1_000,
            profile: &amplified,
        };
        assert_eq!(cap_pair_damage_raw(1, 1, [target, target]), 0);

        let mut unconditional_first = single_channel_profile(1, 0, -1);
        unconditional_first.thresholds_raw[0] = -1;
        unconditional_first.multipliers_q8[0] = 256;
        let first = PairDamageCapTarget {
            health_raw: -1,
            profile: &unconditional_first,
        };
        let second = PairDamageCapTarget {
            health_raw: i32::MAX,
            profile: &unconditional_first,
        };
        assert_eq!(cap_pair_damage_raw(1, 0, [first, second]), 256);

        let harmless_first = PairDamageCapTarget {
            health_raw: i32::MAX,
            profile: &half,
        };
        let observable_candidate = PairDamageCapTarget {
            health_raw: -1,
            profile: &unconditional_first,
        };
        assert_eq!(
            cap_pair_damage_raw(1, 0, [harmless_first, observable_candidate]),
            0,
            "a nonpositive remainder skips the candidate even though the subject always runs"
        );
    }

    #[test]
    fn generic_damage_drains_buffer_then_requests_live_hit_effects() {
        let absorbed = generic_entity_damage_transition(
            GenericEntityDamageState {
                health_raw: 100,
                pre_health_buffer_raw: 50,
                already_dying: false,
            },
            30,
        );
        assert_eq!(absorbed.pre_health_buffer_after_raw, 20);
        assert_eq!(absorbed.damage_after_buffer_raw, 0);
        assert_eq!(absorbed.health_after_subtraction_raw, 100);
        assert_eq!(absorbed.stage, GenericEntityDamageStage::Survived);
        assert_eq!(absorbed.hit_effect_damage_raw(), Some(0));

        let overflow = generic_entity_damage_transition(
            GenericEntityDamageState {
                health_raw: 100,
                pre_health_buffer_raw: 30,
                already_dying: false,
            },
            50,
        );
        assert_eq!(overflow.pre_health_buffer_after_raw, 0);
        assert_eq!(overflow.damage_after_buffer_raw, 20);
        assert_eq!(overflow.health_after_subtraction_raw, 80);
        assert_eq!(overflow.stage, GenericEntityDamageStage::Survived);
        assert_eq!(overflow.hit_effect_damage_raw(), Some(20));
    }

    #[test]
    fn generic_damage_keeps_buffer_mutation_before_the_dying_gate() {
        let transition = generic_entity_damage_transition(
            GenericEntityDamageState {
                health_raw: 10,
                pre_health_buffer_raw: 30,
                already_dying: true,
            },
            50,
        );
        assert_eq!(transition.pre_health_buffer_after_raw, 0);
        assert_eq!(transition.damage_after_buffer_raw, 20);
        assert_eq!(transition.health_after_subtraction_raw, 10);
        assert_eq!(transition.stage, GenericEntityDamageStage::AlreadyDying);
        assert_eq!(transition.hit_effect_damage_raw(), None);
    }

    #[test]
    fn generic_damage_reports_death_without_clamping_or_dropping_signed_inputs() {
        let transition = |health_raw, damage_raw| {
            generic_entity_damage_transition(
                GenericEntityDamageState {
                    health_raw,
                    pre_health_buffer_raw: 0,
                    already_dying: false,
                },
                damage_raw,
            )
        };
        assert_eq!(transition(10, 9).stage, GenericEntityDamageStage::Survived);
        assert_eq!(
            transition(10, 10).stage,
            GenericEntityDamageStage::DeathDispatchRequired
        );
        let overkill = transition(10, 11);
        assert_eq!(overkill.health_after_subtraction_raw, -1);
        assert_eq!(
            overkill.stage,
            GenericEntityDamageStage::DeathDispatchRequired
        );
        assert_eq!(transition(100, -10).health_after_subtraction_raw, 110);
        assert_eq!(
            transition(i32::MIN, 1).health_after_subtraction_raw,
            i32::MAX
        );

        let buffer_heal = generic_entity_damage_transition(
            GenericEntityDamageState {
                health_raw: 100,
                pre_health_buffer_raw: 20,
                already_dying: false,
            },
            -5,
        );
        assert_eq!(buffer_heal.pre_health_buffer_after_raw, 25);
        assert_eq!(buffer_heal.damage_after_buffer_raw, 0);
        assert_eq!(buffer_heal.hit_effect_damage_raw(), Some(0));
    }
}
