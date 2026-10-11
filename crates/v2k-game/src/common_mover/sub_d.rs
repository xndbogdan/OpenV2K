//! Bounded generic Sub-D construction/cache ownership and ordinary type-9
//! steering/terrain-avoidance.
//!
//! Retail `FUN_004203D0` constructs the exact 0x3c-byte allocation represented
//! by [`GenericSubDRuntime`]. [`construct_generic_sub_d`] retains allocator
//! residue instead of inventing values for bytes the constructor never writes,
//! and commits descriptor defaults plus the process-wide stagger counter only
//! after a successful allocation.
//!
//! Retail `FUN_0041F660` owns a mutable 8×8 terrain-classifier cache inside
//! that allocation. [`GenericSubDClassifierCache`] preserves the asymmetric
//! window shifts in `FUN_0041FCB0` and exposes them through an atomic
//! copy/commit transaction. [`Type9SubDFrameOwner`] retains the ordinary
//! type-9 specialization. Keeping it separate from
//! [`Type9SubDRuntime`] makes the remaining entity/allocation seam explicit:
//! every caller must supply the exact per-construction global stagger seed.
//! Evidence replay retains allocator-owned uninitialized origins; native
//! construction explicitly starts an empty, unpositioned classifier cache.
//! Zero-flags descriptors bypass queries but still advance the stagger cadence.

use crate::entity_collision_state::RetailRuntimeValue;
use crate::hover::q31_mul;
use std::convert::Infallible;
use v2k_formats::collision::SubDSteeringDescriptor;
use v2k_formats::terrain::{TerrainGrid, GRID_SIZE};

pub const GENERIC_SUB_D_RUNTIME_BYTES: usize = 0x3c;
pub const GENERIC_SUB_D_DEFAULT_FORWARD_PROBE_RAW: i16 = 0x0200;
pub const GENERIC_SUB_D_DEFAULT_LATERAL_PROBE_RAW: i16 = 0x0100;

/// Bytes retained from the allocator because `FUN_004203D0` never writes them.
///
/// The accepted constructor transcripts prove that `+0x38/+0x39` vary between
/// matched fresh processes. They are allocator residue, not authored cache
/// coordinates. `+0x00` and `+0x3B` are equally outside the constructor's
/// write set and must not be silently cleared by a detached implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericSubDAllocatorResidue {
    pub retained_at_0x00: u32,
    pub cache_origin: [u8; 2],
    pub retained_at_0x3b: u8,
}

/// Exact detached image of retail's 0x3c-byte Sub-D allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericSubDRuntime {
    retained_at_0x00: u32,
    yaw_rate_raw: i32,
    pitch_rate_raw: i32,
    couple_yaw_into_roll_raw: i32,
    last_yaw_step_raw: i32,
    last_pitch_step_raw: i32,
    classifier_cache: GenericSubDClassifierCache,
    retained_at_0x3b: u8,
}

impl GenericSubDRuntime {
    pub const fn retained_at_0x00(self) -> u32 {
        self.retained_at_0x00
    }

    pub const fn yaw_rate_raw(self) -> i32 {
        self.yaw_rate_raw
    }

    pub const fn pitch_rate_raw(self) -> i32 {
        self.pitch_rate_raw
    }

    pub const fn couple_yaw_into_roll_raw(self) -> i32 {
        self.couple_yaw_into_roll_raw
    }

    pub const fn last_yaw_step_raw(self) -> i32 {
        self.last_yaw_step_raw
    }

    pub const fn last_pitch_step_raw(self) -> i32 {
        self.last_pitch_step_raw
    }

    pub const fn classifier_cache(&self) -> &GenericSubDClassifierCache {
        &self.classifier_cache
    }

    pub fn classifier_cache_mut(&mut self) -> &mut GenericSubDClassifierCache {
        &mut self.classifier_cache
    }

    pub const fn retained_at_0x3b(self) -> u8 {
        self.retained_at_0x3b
    }

    /// Serialize the allocation into its literal retail byte layout.
    ///
    /// This is an evidence/test boundary, not a live allocator adapter.
    pub fn to_retail_bytes(self) -> [u8; GENERIC_SUB_D_RUNTIME_BYTES] {
        let mut bytes = [0u8; GENERIC_SUB_D_RUNTIME_BYTES];
        bytes[0x00..0x04].copy_from_slice(&self.retained_at_0x00.to_le_bytes());
        bytes[0x04..0x08].copy_from_slice(&self.yaw_rate_raw.to_le_bytes());
        bytes[0x08..0x0c].copy_from_slice(&self.pitch_rate_raw.to_le_bytes());
        bytes[0x0c..0x10].copy_from_slice(&self.couple_yaw_into_roll_raw.to_le_bytes());
        bytes[0x10..0x14].copy_from_slice(&self.last_yaw_step_raw.to_le_bytes());
        bytes[0x14..0x18].copy_from_slice(&self.last_pitch_step_raw.to_le_bytes());
        for (index, row) in self.classifier_cache.rows().into_iter().enumerate() {
            let offset = 0x18 + index * 4;
            bytes[offset..offset + 4].copy_from_slice(&row.to_le_bytes());
        }
        let RetailRuntimeValue::Known(origin) = self.classifier_cache.origin() else {
            unreachable!("a constructed generic allocation always retains concrete residue");
        };
        bytes[0x38..0x3a].copy_from_slice(&origin);
        bytes[0x3a] = self.classifier_cache.stagger_counter();
        bytes[0x3b] = self.retained_at_0x3b;
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericSubDAllocationAttempt {
    Failed,
    Succeeded(GenericSubDAllocatorResidue),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericSubDConstructorResult {
    AllocationFailed,
    Constructed,
}

/// Exact first-world type-9 Section-12 Sub-D descriptor.
pub const ORDINARY_TYPE9_SUB_D: SubDSteeringDescriptor = SubDSteeringDescriptor {
    steering_divisor_raw: 20,
    couple_yaw_into_roll_raw: 0,
    enable_pitch_steering_raw: 0,
    forward_probe_raw: 128,
    lateral_probe_raw: 64,
    classifier_flags: 0x17,
    reserved_at_0x0b: 0,
};

/// Exact ordinary Type90 Section-12 Sub-D descriptor.
///
/// Same probes/flags as the worker descriptor; only the steering divisor
/// differs (32 vs 20). Retained as its own constant so a divisor-20 check
/// cannot silently admit a Type90 allocation and vice versa.
pub const ORDINARY_TYPE90_SUB_D: SubDSteeringDescriptor = SubDSteeringDescriptor {
    steering_divisor_raw: 32,
    couple_yaw_into_roll_raw: 0,
    enable_pitch_steering_raw: 0,
    forward_probe_raw: 128,
    lateral_probe_raw: 64,
    classifier_flags: 0x17,
    reserved_at_0x0b: 0,
};

/// Exact Type7 diver Section12 Sub-D descriptor. `41F660` still advances the
/// allocation's stagger byte and invalidates rows, then flags0 selects the
/// classifier-free `41FC90` arm. `20260` uses this actor's own divisor32;
/// neither the person/worker terrain classifier nor flyer roll coupling runs.
pub const ORDINARY_TYPE7_SUB_D: SubDSteeringDescriptor = SubDSteeringDescriptor {
    steering_divisor_raw: 32,
    couple_yaw_into_roll_raw: 0,
    enable_pitch_steering_raw: 0,
    forward_probe_raw: 512,
    lateral_probe_raw: 256,
    classifier_flags: 0,
    reserved_at_0x0b: 0,
};

/// Exact first-world type-26 Section-12 Sub-D descriptor.
///
/// Flags `0x12` are `0x10|0x02` (steepness then water). They are not Type-9
/// `0x17`, Type-47 `0x13`, or type-13 `0`.
pub const INTRO2_TYPE26_SUB_D: SubDSteeringDescriptor = SubDSteeringDescriptor {
    steering_divisor_raw: 128,
    couple_yaw_into_roll_raw: 0,
    enable_pitch_steering_raw: 0,
    forward_probe_raw: 0x0200,
    lateral_probe_raw: 200,
    classifier_flags: 0x12,
    reserved_at_0x0b: 0,
};

/// Exact type-47 Section-12 Sub-D descriptor.
///
/// Flags `0x13` are `0x10|0x02|0x01` (steepness, water, then object).
/// They are not Type-9 `0x17` or type-26 `0x12`. The material bit `0x04`
/// stays off.
pub const TYPE47_SUB_D: SubDSteeringDescriptor = SubDSteeringDescriptor {
    steering_divisor_raw: 64,
    couple_yaw_into_roll_raw: 0,
    enable_pitch_steering_raw: 0,
    forward_probe_raw: 0x0200,
    lateral_probe_raw: 0x0100,
    classifier_flags: 0x13,
    reserved_at_0x0b: 0,
};

/// Exact native Intro2 Type16 Section-12 descriptor. 1F660's same no-pitch
/// steering path uses its authored divisor128, forward600 and lateral256;
/// classifier13 selects the already recovered steepness/water/object branches.
pub const INTRO2_TYPE16_SUB_D: SubDSteeringDescriptor = SubDSteeringDescriptor {
    steering_divisor_raw: 128,
    couple_yaw_into_roll_raw: 0,
    enable_pitch_steering_raw: 0,
    forward_probe_raw: 600,
    lateral_probe_raw: 256,
    classifier_flags: 0x13,
    reserved_at_0x0b: 0,
};

/// Exact native Intro2 Type58 descriptor. Its no-pitch 1F660 path uses
/// divisor96, forward512, lateral200 and the recovered object/water/slope13.
pub const INTRO2_TYPE58_SUB_D: SubDSteeringDescriptor = SubDSteeringDescriptor {
    steering_divisor_raw: 96,
    couple_yaw_into_roll_raw: 0,
    enable_pitch_steering_raw: 0,
    forward_probe_raw: 512,
    lateral_probe_raw: 200,
    classifier_flags: 0x13,
    reserved_at_0x0b: 0,
};

/// Canonical Alpine Type30 Section12 Sub-D descriptor (OVL17/19/32/38).
/// 41F660/20260/20360 use the shared no-pitch/no-roll-coupling arithmetic,
/// authored divisor96 and probes700/400. Flags12 select the already owned
/// 41FEB0 steepness-then-water branches; object/material arms are skipped.
/// This admits a component record, not a Type30 actor allocation/task owner.
pub const NATIVE_TYPE30_SUB_D: SubDSteeringDescriptor = SubDSteeringDescriptor {
    steering_divisor_raw: 96,
    couple_yaw_into_roll_raw: 0,
    enable_pitch_steering_raw: 0,
    forward_probe_raw: 700,
    lateral_probe_raw: 400,
    classifier_flags: 0x12,
    reserved_at_0x0b: 0,
};

/// Ordinary Type38/Type129 (worlds 41/42/43): the shared no-pitch 1F660 path
/// with divisor100, probes512/256 and the recovered object/water/slope13.
pub const NATIVE_TYPE38_SUB_D: SubDSteeringDescriptor = SubDSteeringDescriptor {
    steering_divisor_raw: 100,
    couple_yaw_into_roll_raw: 0,
    enable_pitch_steering_raw: 0,
    forward_probe_raw: 512,
    lateral_probe_raw: 256,
    classifier_flags: 0x13,
    reserved_at_0x0b: 0,
};

/// Type94's exact authored descriptor equals Type58's steering/probe values.
/// Its independent constructor/first-query receipt below is not interchangeable.
pub const INTRO2_TYPE94_SUB_D: SubDSteeringDescriptor = INTRO2_TYPE58_SUB_D;

/// Exact flyer Section-12 Sub-D descriptor (types 13, 15, 87).
///
/// Flags `0` skip the terrain/obstacle classifier entirely (`LAB_0041fc90`).
pub const FLYER_SUB_D: SubDSteeringDescriptor = SubDSteeringDescriptor {
    steering_divisor_raw: 64,
    couple_yaw_into_roll_raw: 1,
    enable_pitch_steering_raw: 0,
    forward_probe_raw: 0x0200,
    lateral_probe_raw: 0x0100,
    classifier_flags: 0,
    reserved_at_0x0b: 0,
};

/// Type10's own Section12 steering divisor. `41F660 -> 20260` divides by
/// this signed word; flags0 take41FC90 without querying the cache origin.
pub const INTRO2_TYPE10_SUB_D: SubDSteeringDescriptor = SubDSteeringDescriptor {
    steering_divisor_raw: 48,
    ..FLYER_SUB_D
};

/// Types22/23/24/124 use the same classifier-free `41FC90` steering arm as
/// flyers. Their descriptor disables the separate `204C0` yaw/roll coupling.
pub const SHARED_FISH_SUB_D: SubDSteeringDescriptor = SubDSteeringDescriptor {
    couple_yaw_into_roll_raw: 0,
    ..FLYER_SUB_D
};

/// Type62's zebrafish samples `41FEB0`'s center-cell shallow-water gate.
/// Its authored probes and steering divisor differ from the aquatic fish.
pub const TYPE62_SUB_D: SubDSteeringDescriptor = SubDSteeringDescriptor {
    steering_divisor_raw: 32,
    couple_yaw_into_roll_raw: 0,
    enable_pitch_steering_raw: 0,
    forward_probe_raw: 768,
    lateral_probe_raw: 256,
    classifier_flags: 0x20,
    reserved_at_0x0b: 0,
};

/// Type49's water-capable rover uses steepness/object checks, deliberately
/// skipping water and infected-material rejection (classifier flags0x11).
pub const CLEANSING_VEHICLE_SUB_D: SubDSteeringDescriptor = SubDSteeringDescriptor {
    steering_divisor_raw: 64,
    couple_yaw_into_roll_raw: 0,
    enable_pitch_steering_raw: 0,
    forward_probe_raw: 256,
    lateral_probe_raw: 128,
    classifier_flags: 0x11,
    reserved_at_0x0b: 0,
};

const fn admits_shared_fun_0041f660_steering(descriptor: SubDSteeringDescriptor) -> bool {
    matches!(
        descriptor,
        ORDINARY_TYPE9_SUB_D
            | ORDINARY_TYPE7_SUB_D
            | ORDINARY_TYPE90_SUB_D
            | INTRO2_TYPE26_SUB_D
            | TYPE47_SUB_D
            | INTRO2_TYPE16_SUB_D
            | INTRO2_TYPE58_SUB_D
            | NATIVE_TYPE30_SUB_D
            | NATIVE_TYPE38_SUB_D
            | INTRO2_TYPE10_SUB_D
            | SHARED_FISH_SUB_D
            | TYPE62_SUB_D
            | CLEANSING_VEHICLE_SUB_D
            | FLYER_SUB_D
    )
}

/// X/Z probes passed to retail's cached terrain classifier.
///
/// Names follow call order in `FUN_0041F660`. `half_forward` is the sample
/// whose result becomes the central avoidance class; `rear` is consulted only
/// when that class is non-zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type9SubDProbePlan {
    pub left_raw: [i16; 2],
    pub half_forward_raw: [i16; 2],
    pub right_raw: [i16; 2],
    pub rear_raw: [i16; 2],
}

/// Height samples needed by retail's class-4/class-5 tie breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type9SubDTerrainHeights {
    pub left_raw: i16,
    pub current_raw: i16,
    pub right_raw: i16,
}

/// Exact mutable classifier cache at Sub-D runtime `+0x18..+0x3B`.
///
/// Each row stores eight four-bit values. Zero means "not classified"; retail
/// stores `class + 1`, allowing classes `0..=6` to remain distinguishable.
/// The two cache-origin bytes are left uninitialized by `FUN_004203D0`.
/// Cleared rows make the first returned class independent from those bytes,
/// but not the resulting window state: allocator garbage can place that first
/// cell at a different cache edge and change later fills/invalidations.
/// Therefore an unresolved replay origin remains an explicit runtime boundary.
/// Native construction names its different initial-cache policy explicitly in
/// [`construct_native_sub_d`] rather than fabricating allocator bytes.
///
/// The accepted fresh-New-Game Level-1 trace
/// `20260730-064237-sub-d-first-consumer.txt` closes one narrower route. Its
/// six ordinary type-9 allocations retained varied origin residue, but every
/// first classifier call came from `0x0041F7A8` and took retail's full-reset
/// branch to the queried cell. [`FreshLevel1Type9ClassifierAdmission`] models
/// only that route; it is not generic allocator emulation.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GenericSubDClassifierOrigin {
    Unresolved,
    /// Native empty-cache policy, independent of captured allocation histories.
    NativeFirstQueryReset,
    FreshLevel1FirstQueryReset,
    Intro2Type9FirstQueryReset,
    Intro2Type16FirstQueryReset,
    Intro2Type58FirstQueryReset,
    Intro2Type94FirstQueryReset,
    Intro2Type53FirstQueryReset,
    Intro2Type26FirstQueryReset,
    Intro2Type47FirstQueryReset,
    Level1Type47FirstQueryReset,
    Type17FirstQueryReset,
    Type8FirstQueryReset,
    Known([u8; 2]),
}

impl GenericSubDClassifierOrigin {
    const fn runtime_value(self) -> RetailRuntimeValue<[u8; 2]> {
        match self {
            Self::Known(origin) => RetailRuntimeValue::Known(origin),
            Self::Unresolved
            | Self::NativeFirstQueryReset
            | Self::FreshLevel1FirstQueryReset
            | Self::Intro2Type9FirstQueryReset
            | Self::Intro2Type16FirstQueryReset
            | Self::Intro2Type58FirstQueryReset
            | Self::Intro2Type94FirstQueryReset
            | Self::Intro2Type53FirstQueryReset
            | Self::Intro2Type26FirstQueryReset
            | Self::Intro2Type47FirstQueryReset
            | Self::Level1Type47FirstQueryReset
            | Self::Type17FirstQueryReset
            | Self::Type8FirstQueryReset => RetailRuntimeValue::Unresolved,
        }
    }

    const fn can_classify(self) -> bool {
        !matches!(self, Self::Unresolved)
    }
}

/// Route receipt for the six ordinary type-9 actors constructed by a fresh
/// New Game handoff into Level 1.
///
/// The private field prevents callers from manufacturing an admission. They
/// must name [`Self::ACCEPTED_FIRST_CONSUMER_TRACE`] at the fresh-Level-1
/// construction boundary rather than treating any unresolved origin as this
/// route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshLevel1Type9ClassifierAdmission {
    _private: (),
}

impl FreshLevel1Type9ClassifierAdmission {
    pub const ACCEPTED_FIRST_CONSUMER_TRACE: Self = Self { _private: () };
}

/// The thirteen native Intro2 Type9 allocations joined to their own first
/// classifier use in `V200001.run`, accepted 2026-09-08. Each observed
/// `00401602 -> 0041F660 -> 0041F7A8 -> 0041FCB0` takes full-reset.
/// The birth publisher authenticates the authored position and metadata;
/// this receipt additionally binds each spawn to its own constructor seed.
/// Other allocation histories and actor types retain their separate policy.
pub(crate) const fn intro2_type9_first_query_owner_for_birth(
    spawn_index: usize,
    seed: u8,
) -> Option<Type9SubDFrameOwner> {
    match (spawn_index, seed) {
        (2, 0x02)
        | (3, 0x03)
        | (9, 0x09)
        | (11, 0x0B)
        | (16, 0x0F)
        | (17, 0x10)
        | (18, 0x11)
        | (19, 0x12)
        | (49, 0x20)
        | (50, 0x21)
        | (58, 0x24)
        | (59, 0x25)
        | (60, 0x26) => Some(Type9SubDFrameOwner {
            cache: Type9SubDClassifierCache {
                rows: [0; 8],
                origin: GenericSubDClassifierOrigin::Intro2Type9FirstQueryReset,
                stagger_counter: seed,
            },
        }),
        _ => None,
    }
}

/// The two native Type16 allocations joined to their own first classifier
/// use in the 2026-09-08 `V200001.run` address-read query. Both observed
/// `00401602 -> 0041F660 -> 0041F7A8 -> 0041FCB0` take full-reset, with their
/// own constructor seeds 05/1B. This receipt does not initialize arbitrary
/// allocator origins or admit a different birth/seed combination.
pub(crate) const fn intro2_type16_first_query_owner_for_birth(
    spawn_index: usize,
    seed: u8,
) -> Option<Type9SubDFrameOwner> {
    match (spawn_index, seed) {
        (5, 0x05) | (42, 0x1B) => Some(Type9SubDFrameOwner {
            cache: Type9SubDClassifierCache {
                rows: [0; 8],
                origin: GenericSubDClassifierOrigin::Intro2Type16FirstQueryReset,
                stagger_counter: seed,
            },
        }),
        _ => None,
    }
}

/// Spawn40's own203D0 allocation/first41FCC4 read in V200001, retained as
/// intro2-type58-{constructors,origins}.txt (2026-09-08). Seed19's first
/// 00401602 -> 0041F7A8 query BDFF/0DC8 takes full-reset, class0, row0=1.
/// This receipt neither assigns generic origin bytes nor admits other births.
pub(crate) const fn intro2_type58_first_query_owner_for_birth(
    spawn_index: usize,
    seed: u8,
) -> Option<Type9SubDFrameOwner> {
    match (spawn_index, seed) {
        (40, 0x19) => Some(Type9SubDFrameOwner {
            cache: Type9SubDClassifierCache {
                rows: [0; 8],
                origin: GenericSubDClassifierOrigin::Intro2Type58FirstQueryReset,
                stagger_counter: seed,
            },
        }),
        _ => None,
    }
}

/// Spawn43's own203D0 allocation/first41FCC4 read in V200001, retained as
/// intro2-type94-{constructors,origins}.txt (2026-09-09). Seed1C's first
/// 00401602 -> 0041F7A8 query C1FF/82C8 takes full-reset, class0, row0=1.
/// Equal descriptor bytes do not authorize borrowing Type58's birth receipt.
pub(crate) const fn intro2_type94_first_query_owner_for_birth(
    spawn_index: usize,
    seed: u8,
) -> Option<Type9SubDFrameOwner> {
    match (spawn_index, seed) {
        (43, 0x1C) => Some(Type9SubDFrameOwner {
            cache: Type9SubDClassifierCache {
                rows: [0; 8],
                origin: GenericSubDClassifierOrigin::Intro2Type94FirstQueryReset,
                stagger_counter: seed,
            },
        }),
        _ => None,
    }
}

/// The four Type53 allocations joined to their own first classifier use in
/// the 2026-09-08 `V200001.run` walks. The later address-read query separately
/// closes spawns26/41: 00401602 -> 0041F7A8 -> full-reset, with their own
/// constructor seeds16/1A. Other spawn/seed combinations remain unresolved.
pub(crate) const fn intro2_type53_first_query_owner_for_birth(
    spawn_index: usize,
    seed: u8,
) -> Option<Type9SubDFrameOwner> {
    match (spawn_index, seed) {
        (20, 0x13) | (26, 0x16) | (38, 0x18) | (41, 0x1A) => Some(Type9SubDFrameOwner {
            cache: Type9SubDClassifierCache {
                rows: [0; 8],
                origin: GenericSubDClassifierOrigin::Intro2Type53FirstQueryReset,
                stagger_counter: seed,
            },
        }),
        _ => None,
    }
}

/// Route receipt for the two accepted Intro2 type-26 identities in
/// `V200001.run` (`BE00/0F00` seed `0x0A`, `C200/0E00` seed `0x15`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type26ClassifierAdmission {
    _private: (),
}

impl Intro2Type26ClassifierAdmission {
    pub const ACCEPTED_TTD_V200001: Self = Self { _private: () };
}

/// Process seeds joined by `V200001.run` to the two Intro2 type-26 identities.
pub const INTRO2_TYPE26_FIRST_QUERY_SEEDS: [u8; 2] = [0x0A, 0x15];

/// Mint the pending first-query owner only for the two accepted Intro2
/// type-26 seeds. Other classifier-bearing Sub-D owners stay fail-closed;
/// Type 13's zero-flags descriptor uses its separate query-free frame path.
pub const fn intro2_type26_first_query_owner_for_seed(seed: u8) -> Option<Type9SubDFrameOwner> {
    match seed {
        0x0A | 0x15 => Some(
            Type9SubDFrameOwner::pending_intro2_type26_first_query_reset(
                seed,
                Intro2Type26ClassifierAdmission::ACCEPTED_TTD_V200001,
            ),
        ),
        _ => None,
    }
}

/// Route receipt for the three Intro2 Type-47 constructions in
/// `V200001.run` (seeds `0x06/0x07/0x08`).
///
/// This is not the Level-1 trio `0x2B/0x2C/0x2D`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type47ClassifierAdmission {
    _private: (),
}

impl Intro2Type47ClassifierAdmission {
    pub const ACCEPTED_TTD_V200001: Self = Self { _private: () };
}

/// Intro2 Type-47 constructor seeds from `V200001.run`. This is not the
/// Level-1 trio `0x2B/0x2C/0x2D`.
pub const INTRO2_TYPE47_FIRST_QUERY_SEEDS: [u8; 3] = [0x06, 0x07, 0x08];

/// Mint the pending first-query owner only for the three Intro2 Type-47
/// seeds. Level-1 Type-47 `0x2B/0x2C/0x2D` use
/// [`level1_type47_first_query_owner_for_seed`].
pub const fn intro2_type47_first_query_owner_for_seed(seed: u8) -> Option<Type9SubDFrameOwner> {
    match seed {
        0x06 | 0x07 | 0x08 => Some(
            Type9SubDFrameOwner::pending_intro2_type47_first_query_reset(
                seed,
                Intro2Type47ClassifierAdmission::ACCEPTED_TTD_V200001,
            ),
        ),
        _ => None,
    }
}

/// Route receipt for the three fresh-Level-1 Type-47 constructions in
/// `20260831-215158-ttd-level1-first-consumer` (`V200003.run`).
///
/// Spawns 11/12/13 at `B700/7F00`, `BE00/8500`, `BE00/7D00` with constructor
/// seeds `0x2B/0x2C/0x2D`. Replay-level seeds `0x3C/0x3D/0x3E` are not this
/// admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level1Type47ClassifierAdmission {
    _private: (),
}

impl Level1Type47ClassifierAdmission {
    pub const ACCEPTED_TTD_V200003: Self = Self { _private: () };
}

/// Fresh-Level-1 Type-47 constructor seeds from `V200003.run`.
pub const LEVEL1_TYPE47_FIRST_QUERY_SEEDS: [u8; 3] = [0x2B, 0x2C, 0x2D];

/// Mint the pending first-query owner only for the three fresh-Level-1
/// Type-47 seeds. Intro2 `0x06/0x07/0x08`, replay-level `0x3C/0x3D/0x3E`,
/// and type 13 stay `None`.
pub const fn level1_type47_first_query_owner_for_seed(seed: u8) -> Option<Type9SubDFrameOwner> {
    match seed {
        0x2B | 0x2C | 0x2D => Some(
            Type9SubDFrameOwner::pending_level1_type47_first_query_reset(
                seed,
                Level1Type47ClassifierAdmission::ACCEPTED_TTD_V200003,
            ),
        ),
        _ => None,
    }
}

/// Route receipt for type-17 first queries in `V200002.run`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17ClassifierAdmission {
    _private: (),
}

impl Type17ClassifierAdmission {
    pub const ACCEPTED_TTD_V200002: Self = Self { _private: () };
}

/// Intro2 type-17 seeds `0x04/0x17` and fresh Level-1 spawns 17–20
/// `0x31/0x32/0x33/0x34` from `V200002.run`.
pub const TYPE17_FIRST_QUERY_SEEDS: [u8; 6] = [0x04, 0x17, 0x31, 0x32, 0x33, 0x34];
/// Fresh Level-1 spawn order 17/18/19/20.
pub const FRESH_LEVEL1_TYPE17_SUB_D_SEEDS: [u8; 4] = [0x31, 0x32, 0x33, 0x34];

pub const fn type17_first_query_owner_for_seed(seed: u8) -> Option<Type9SubDFrameOwner> {
    match seed {
        0x04 | 0x17 | 0x31 | 0x32 | 0x33 | 0x34 => {
            Some(Type9SubDFrameOwner::pending_type17_first_query_reset(
                seed,
                Type17ClassifierAdmission::ACCEPTED_TTD_V200002,
            ))
        }
        _ => None,
    }
}

pub const fn type17_seed_for_fresh_level1_spawn(spawn_index: usize) -> Option<u8> {
    let mut index = 0;
    const SPAWNS: [usize; 4] = [17, 18, 19, 20];
    while index < SPAWNS.len() {
        if SPAWNS[index] == spawn_index {
            return Some(FRESH_LEVEL1_TYPE17_SUB_D_SEEDS[index]);
        }
        index += 1;
    }
    None
}

/// Route receipt for type-8 first queries in `V200002.run`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type8ClassifierAdmission {
    _private: (),
}

impl Type8ClassifierAdmission {
    pub const ACCEPTED_TTD_V200002: Self = Self { _private: () };
}

/// Intro2 authored type-8 seeds `0x0C/0x0D/0x0E` and the cargo-converted
/// Main Base scientist `0x38` from `V200002.run`.
pub const TYPE8_FIRST_QUERY_SEEDS: [u8; 4] = [0x0C, 0x0D, 0x0E, 0x38];
pub const TYPE8_MAIN_BASE_CONVERSION_SUB_D_SEED: u8 = 0x38;

pub const fn type8_first_query_owner_for_seed(seed: u8) -> Option<Type9SubDFrameOwner> {
    match seed {
        0x0C | 0x0D | 0x0E | 0x38 => Some(Type9SubDFrameOwner::pending_type8_first_query_reset(
            seed,
            Type8ClassifierAdmission::ACCEPTED_TTD_V200002,
        )),
        _ => None,
    }
}

/// Process-lifetime owner of retail `DAT_004DB0B0`.
///
/// `FUN_004203D0` copies this byte to allocation `+0x3A` and increments it
/// only after a successful 0x3c-byte allocation. It is shared by every Sub-D
/// descriptor and does not reset when the world tick resets at a level handoff.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SubDAllocationCounter {
    next_seed: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubDAllocationOutcome {
    Failed,
    Succeeded,
}

impl SubDAllocationCounter {
    pub const fn from_next_seed(next_seed: u8) -> Self {
        Self { next_seed }
    }

    pub const fn next_seed(self) -> u8 {
        self.next_seed
    }

    /// Finish one allocator attempt in retail order.
    ///
    /// A failed attempt returns no seed and leaves the process counter
    /// untouched. A successful attempt returns the pre-increment byte.
    pub fn finish_attempt(&mut self, outcome: SubDAllocationOutcome) -> Option<u8> {
        match outcome {
            SubDAllocationOutcome::Failed => None,
            SubDAllocationOutcome::Succeeded => {
                let seed = self.next_seed;
                self.next_seed = self.next_seed.wrapping_add(1);
                Some(seed)
            }
        }
    }
}

/// Successful native Sub-D construction, before any behavior initializer.
///
/// The descriptor retains its authored policy and receives only `203D0`'s two
/// zero-probe defaults. The runtime is the steering subset used by live movers;
/// this value is not a serialized image of uninitialized allocator bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeSubDConstruction {
    pub(crate) descriptor: SubDSteeringDescriptor,
    pub(crate) seed: u8,
    pub(crate) frame_owner: Type9SubDFrameOwner,
    pub(crate) runtime: Type9SubDRuntime,
}

/// Finish an actual successful native `FUN_004203D0` allocation.
///
/// Callers must reach this boundary in component-construction order, including
/// descriptors whose classifier flags are zero or whose behavior is not yet
/// implemented. Failure before or inside the Sub-D allocation consumes no seed;
/// failure in a later component or behavior must not rewind this counter.
///
/// Native code starts the constructor-cleared cache without a positioned
/// window. Its first actual classifier query clears/anchors that window at the
/// queried cell, then follows the retail cache algorithm. Every zero-row retail
/// allocation returns the same first classification regardless of origin
/// residue. Subsequent cache placement and staggered invalidation can differ
/// when retail's first query took a near-window shift instead of full reset.
/// This is an explicit deterministic native policy, not evidence that every
/// retail allocation takes full reset. Known-residue and captured replay
/// constructors retain their distinct policies unchanged.
pub(crate) fn construct_native_sub_d(
    counter: &mut SubDAllocationCounter,
    descriptor: SubDSteeringDescriptor,
) -> NativeSubDConstruction {
    let descriptor = normalized_sub_d_probe_words(descriptor);
    let seed = counter
        .finish_attempt(SubDAllocationOutcome::Succeeded)
        .expect("a successful Sub-D allocation consumes one process seed");
    NativeSubDConstruction {
        descriptor,
        seed,
        frame_owner: Type9SubDFrameOwner {
            cache: GenericSubDClassifierCache {
                rows: [0; 8],
                origin: GenericSubDClassifierOrigin::NativeFirstQueryReset,
                stagger_counter: seed,
            },
        },
        runtime: Type9SubDRuntime::from_constructor(),
    }
}

fn normalized_sub_d_probe_words(mut descriptor: SubDSteeringDescriptor) -> SubDSteeringDescriptor {
    if descriptor.forward_probe_raw == 0 {
        descriptor.forward_probe_raw = GENERIC_SUB_D_DEFAULT_FORWARD_PROBE_RAW;
    }
    if descriptor.lateral_probe_raw == 0 {
        descriptor.lateral_probe_raw = GENERIC_SUB_D_DEFAULT_LATERAL_PROBE_RAW;
    }
    descriptor
}

/// Apply the exact successful-write set of `FUN_004203D0`.
///
/// Allocation itself remains an adapter responsibility. A failed attempt
/// performs no descriptor, runtime-slot, or counter write in this detached
/// transaction. On success, the returned allocation retains all allocator
/// residue, normalizes the two zero probe words in the mutable descriptor,
/// copies the process byte to `+0x3A`, then wrapping-increments the owner.
pub fn construct_generic_sub_d(
    counter: &mut SubDAllocationCounter,
    descriptor: &mut SubDSteeringDescriptor,
    runtime_slot: &mut Option<GenericSubDRuntime>,
    allocation: GenericSubDAllocationAttempt,
) -> GenericSubDConstructorResult {
    let GenericSubDAllocationAttempt::Succeeded(residue) = allocation else {
        return GenericSubDConstructorResult::AllocationFailed;
    };

    let next_descriptor = normalized_sub_d_probe_words(*descriptor);

    let seed = counter.next_seed();
    let next_runtime = GenericSubDRuntime {
        retained_at_0x00: residue.retained_at_0x00,
        yaw_rate_raw: 0,
        pitch_rate_raw: 0,
        couple_yaw_into_roll_raw: i32::from(next_descriptor.couple_yaw_into_roll_raw),
        last_yaw_step_raw: 0,
        last_pitch_step_raw: 0,
        classifier_cache: GenericSubDClassifierCache::from_retail_state(
            [0; 8],
            residue.cache_origin,
            seed,
        ),
        retained_at_0x3b: residue.retained_at_0x3b,
    };

    *descriptor = next_descriptor;
    *runtime_slot = Some(next_runtime);
    let copied_seed = counter.finish_attempt(SubDAllocationOutcome::Succeeded);
    debug_assert_eq!(copied_seed, Some(seed));
    GenericSubDConstructorResult::Constructed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericSubDClassifierCache {
    rows: [u32; 8],
    origin: GenericSubDClassifierOrigin,
    stagger_counter: u8,
}

/// Backward-compatible name for the ordinary type-9 specialization.
pub type Type9SubDClassifierCache = GenericSubDClassifierCache;

impl GenericSubDClassifierCache {
    /// Retain the deterministic constructor fields while leaving retail's two
    /// uninitialized cache-origin bytes unresolved.
    pub const fn pending_constructor_origin(stagger_seed: u8) -> Self {
        Self {
            rows: [0; 8],
            origin: GenericSubDClassifierOrigin::Unresolved,
            stagger_counter: stagger_seed,
        }
    }

    /// Retain constructor state for the captured fresh-Level-1 ordinary-type-9
    /// route until its first classifier query performs retail's observed full
    /// reset.
    const fn pending_fresh_level1_first_query_reset(
        stagger_seed: u8,
        _admission: FreshLevel1Type9ClassifierAdmission,
    ) -> Self {
        Self {
            rows: [0; 8],
            origin: GenericSubDClassifierOrigin::FreshLevel1FirstQueryReset,
            stagger_counter: stagger_seed,
        }
    }

    const fn pending_intro2_type26_first_query_reset(
        stagger_seed: u8,
        _admission: Intro2Type26ClassifierAdmission,
    ) -> Self {
        Self {
            rows: [0; 8],
            origin: GenericSubDClassifierOrigin::Intro2Type26FirstQueryReset,
            stagger_counter: stagger_seed,
        }
    }

    const fn pending_intro2_type47_first_query_reset(
        stagger_seed: u8,
        _admission: Intro2Type47ClassifierAdmission,
    ) -> Self {
        Self {
            rows: [0; 8],
            origin: GenericSubDClassifierOrigin::Intro2Type47FirstQueryReset,
            stagger_counter: stagger_seed,
        }
    }

    const fn pending_level1_type47_first_query_reset(
        stagger_seed: u8,
        _admission: Level1Type47ClassifierAdmission,
    ) -> Self {
        Self {
            rows: [0; 8],
            origin: GenericSubDClassifierOrigin::Level1Type47FirstQueryReset,
            stagger_counter: stagger_seed,
        }
    }

    const fn pending_type17_first_query_reset(
        stagger_seed: u8,
        _admission: Type17ClassifierAdmission,
    ) -> Self {
        Self {
            rows: [0; 8],
            origin: GenericSubDClassifierOrigin::Type17FirstQueryReset,
            stagger_counter: stagger_seed,
        }
    }

    const fn pending_type8_first_query_reset(
        stagger_seed: u8,
        _admission: Type8ClassifierAdmission,
    ) -> Self {
        Self {
            rows: [0; 8],
            origin: GenericSubDClassifierOrigin::Type8FirstQueryReset,
            stagger_counter: stagger_seed,
        }
    }

    /// Exact capture/fixture constructor for a complete live cache state.
    pub const fn from_retail_state(rows: [u32; 8], origin: [u8; 2], stagger_counter: u8) -> Self {
        Self {
            rows,
            origin: GenericSubDClassifierOrigin::Known(origin),
            stagger_counter,
        }
    }

    pub const fn stagger_counter(&self) -> u8 {
        self.stagger_counter
    }

    pub const fn origin(&self) -> RetailRuntimeValue<[u8; 2]> {
        self.origin.runtime_value()
    }

    pub const fn can_classify(&self) -> bool {
        self.origin.can_classify()
    }

    pub const fn rows(&self) -> [u32; 8] {
        self.rows
    }

    /// Opening phase of `FUN_0041F660`: advance the allocation's stagger byte
    /// and clear one cache row on every eighth call.
    fn begin_frame(&mut self) {
        self.stagger_counter = self.stagger_counter.wrapping_add(1);
        if self.stagger_counter & 7 == 0 {
            self.rows[usize::from((self.stagger_counter & 0x38) >> 3)] = 0;
        }
    }

    /// Run one complete cache phase against a copy and publish it only when
    /// every query succeeds.
    ///
    /// The stagger increment and its possible row clear are part of the same
    /// transaction. An unresolved origin or external classifier failure leaves
    /// the original cache byte-for-byte unchanged.
    pub fn transact<T, E>(
        &mut self,
        operation: impl FnOnce(
            &mut GenericSubDCacheTransaction,
        ) -> Result<T, GenericSubDCacheTransactionBlock<E>>,
    ) -> Result<T, GenericSubDCacheTransactionBlock<E>> {
        if !self.origin.can_classify() {
            return Err(GenericSubDCacheTransactionBlock::UnresolvedOrigin);
        }
        let mut transaction = GenericSubDCacheTransaction { next: *self };
        transaction.next.begin_frame();
        let result = operation(&mut transaction)?;
        *self = transaction.next;
        Ok(result)
    }

    /// Exact type-9 (`classifier_flags == 0x17`) single-query compatibility
    /// helper. This preserves the historical explicit [`Self::begin_frame`]
    /// ownership used by focused fixtures; new frame paths should use
    /// [`Self::transact`] so the stagger write and all queries are atomic.
    pub fn classify(
        &mut self,
        terrain: &TerrainGrid,
        point_raw: [i16; 2],
    ) -> RetailRuntimeValue<u16> {
        match self.query_with(point_raw, |_| {
            Ok::<u16, Infallible>(classify_type9_terrain(terrain, point_raw))
        }) {
            Ok(class) => RetailRuntimeValue::Known(class),
            Err(GenericSubDCacheTransactionBlock::UnresolvedOrigin) => {
                RetailRuntimeValue::Unresolved
            }
            Err(GenericSubDCacheTransactionBlock::Classifier(error)) => match error {},
        }
    }

    fn query_with<E>(
        &mut self,
        point_raw: [i16; 2],
        classify: impl FnOnce([i16; 2]) -> Result<u16, E>,
    ) -> Result<u16, GenericSubDCacheTransactionBlock<E>> {
        let cell = [
            (point_raw[0] as u16 >> 8) as u8,
            (point_raw[1] as u16 >> 8) as u8,
        ];
        let RetailRuntimeValue::Known([dx, dz]) = self.move_window_to_include(cell) else {
            return Err(GenericSubDCacheTransactionBlock::UnresolvedOrigin);
        };
        let shift = u32::from(dx) * 4;
        let row = &mut self.rows[usize::from(dz)];
        let cached = (*row >> shift) & 0x0f;
        if cached != 0 {
            return Ok((cached - 1) as u16);
        }

        let class = classify(point_raw).map_err(GenericSubDCacheTransactionBlock::Classifier)?;
        *row |= u32::from(class + 1) << shift;
        Ok(class)
    }

    fn move_window_to_include(&mut self, cell: [u8; 2]) -> RetailRuntimeValue<[u8; 2]> {
        let [mut origin_x, mut origin_z] = match self.origin {
            GenericSubDClassifierOrigin::Unresolved => {
                return RetailRuntimeValue::Unresolved;
            }
            GenericSubDClassifierOrigin::NativeFirstQueryReset
            | GenericSubDClassifierOrigin::FreshLevel1FirstQueryReset
            | GenericSubDClassifierOrigin::Intro2Type9FirstQueryReset
            | GenericSubDClassifierOrigin::Intro2Type16FirstQueryReset
            | GenericSubDClassifierOrigin::Intro2Type58FirstQueryReset
            | GenericSubDClassifierOrigin::Intro2Type94FirstQueryReset
            | GenericSubDClassifierOrigin::Intro2Type53FirstQueryReset
            | GenericSubDClassifierOrigin::Intro2Type26FirstQueryReset
            | GenericSubDClassifierOrigin::Intro2Type47FirstQueryReset
            | GenericSubDClassifierOrigin::Level1Type47FirstQueryReset
            | GenericSubDClassifierOrigin::Type17FirstQueryReset
            | GenericSubDClassifierOrigin::Type8FirstQueryReset => {
                self.rows = [0; 8];
                self.origin = GenericSubDClassifierOrigin::Known(cell);
                return RetailRuntimeValue::Known([0, 0]);
            }
            GenericSubDClassifierOrigin::Known(origin) => origin,
        };

        let mut dx = cell[0].wrapping_sub(origin_x);
        let mut dz = cell[1].wrapping_sub(origin_z);
        if dx > 7 || dz > 7 {
            let x_near = dx < 0x0f || dx > 0xf8;
            let z_near = dz < 0x0f || dz > 0xf8;
            if !x_near || !z_near {
                self.rows = [0; 8];
                origin_x = cell[0];
                origin_z = cell[1];
                dx = 0;
                dz = 0;
            } else {
                if dx > 7 {
                    if dx < 0x0f {
                        let amount = dx - 7;
                        origin_x = origin_x.wrapping_add(amount);
                        dx = 7;
                        let shift = u32::from(amount) * 4;
                        for row in &mut self.rows {
                            *row <<= shift;
                        }
                    } else {
                        let amount = 0u8.wrapping_sub(dx);
                        origin_x = origin_x.wrapping_sub(amount);
                        dx = 0;
                        let shift = u32::from(amount) * 4;
                        for row in &mut self.rows {
                            *row >>= shift;
                        }
                    }
                }

                if dz > 7 {
                    if dz < 0x0f {
                        let amount = dz - 7;
                        origin_z = origin_z.wrapping_add(amount);
                        dz = 7;
                        // This apparent asymmetry is literal retail behavior:
                        // the positive-Z path moves the origin without copying
                        // rows. Do not "repair" it into a symmetric shift.
                    } else {
                        let amount = 0u8.wrapping_sub(dz);
                        origin_z = origin_z.wrapping_sub(amount);
                        dz = 0;
                        for destination in (usize::from(amount)..8).rev() {
                            self.rows[destination] = self.rows[destination - usize::from(amount)];
                        }
                        self.rows[..usize::from(amount)].fill(0);
                    }
                }
            }
        }

        self.origin = GenericSubDClassifierOrigin::Known([origin_x, origin_z]);
        RetailRuntimeValue::Known([dx, dz])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericSubDCacheTransactionBlock<E> {
    UnresolvedOrigin,
    Classifier(E),
}

/// Restricted query surface used by [`GenericSubDClassifierCache::transact`].
pub struct GenericSubDCacheTransaction {
    next: GenericSubDClassifierCache,
}

impl GenericSubDCacheTransaction {
    pub fn query<E>(
        &mut self,
        point_raw: [i16; 2],
        classify: impl FnOnce([i16; 2]) -> Result<u16, E>,
    ) -> Result<u16, GenericSubDCacheTransactionBlock<E>> {
        self.next.query_with(point_raw, classify)
    }
}

/// Mutable per-allocation owner that turns concrete terrain/target state into
/// one complete [`Type9SubDFrameEvidence`] value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type9SubDFrameOwner {
    cache: Type9SubDClassifierCache,
}

impl Type9SubDFrameOwner {
    pub const fn flyer() -> Self {
        Self {
            cache: Type9SubDClassifierCache::pending_constructor_origin(0),
        }
    }

    pub const fn pending_constructor_origin(stagger_seed: u8) -> Self {
        Self {
            cache: Type9SubDClassifierCache::pending_constructor_origin(stagger_seed),
        }
    }

    pub const fn pending_fresh_level1_first_query_reset(
        stagger_seed: u8,
        admission: FreshLevel1Type9ClassifierAdmission,
    ) -> Self {
        Self {
            cache: Type9SubDClassifierCache::pending_fresh_level1_first_query_reset(
                stagger_seed,
                admission,
            ),
        }
    }

    pub const fn pending_intro2_type26_first_query_reset(
        stagger_seed: u8,
        admission: Intro2Type26ClassifierAdmission,
    ) -> Self {
        Self {
            cache: Type9SubDClassifierCache::pending_intro2_type26_first_query_reset(
                stagger_seed,
                admission,
            ),
        }
    }

    pub const fn pending_intro2_type47_first_query_reset(
        stagger_seed: u8,
        admission: Intro2Type47ClassifierAdmission,
    ) -> Self {
        Self {
            cache: Type9SubDClassifierCache::pending_intro2_type47_first_query_reset(
                stagger_seed,
                admission,
            ),
        }
    }

    pub const fn pending_level1_type47_first_query_reset(
        stagger_seed: u8,
        admission: Level1Type47ClassifierAdmission,
    ) -> Self {
        Self {
            cache: Type9SubDClassifierCache::pending_level1_type47_first_query_reset(
                stagger_seed,
                admission,
            ),
        }
    }

    pub const fn pending_type17_first_query_reset(
        stagger_seed: u8,
        admission: Type17ClassifierAdmission,
    ) -> Self {
        Self {
            cache: Type9SubDClassifierCache::pending_type17_first_query_reset(
                stagger_seed,
                admission,
            ),
        }
    }

    pub const fn pending_type8_first_query_reset(
        stagger_seed: u8,
        admission: Type8ClassifierAdmission,
    ) -> Self {
        Self {
            cache: Type9SubDClassifierCache::pending_type8_first_query_reset(
                stagger_seed,
                admission,
            ),
        }
    }

    pub const fn from_retail_state(rows: [u32; 8], origin: [u8; 2], stagger_counter: u8) -> Self {
        Self {
            cache: Type9SubDClassifierCache::from_retail_state(rows, origin, stagger_counter),
        }
    }

    pub const fn classifier_cache(&self) -> &Type9SubDClassifierCache {
        &self.cache
    }

    /// Advance the allocation-owned frame cadence and build the query-free
    /// evidence selected by a zero classifier-flags byte.
    ///
    /// `FUN_0041F660` increments `+0x3A` and performs its possible cache-row
    /// clear before testing descriptor `+0x0A`. The origin bytes are never
    /// consumed on this route, but the cadence write still belongs to the
    /// transaction.
    pub fn evidence_for_classifier_free_frame(
        &mut self,
        descriptor: SubDSteeringDescriptor,
        position_raw: [i16; 3],
        target_raw: [i16; 3],
        right_q31: [i32; 3],
        forward_q31: [i32; 3],
    ) -> Option<Type9SubDFrameEvidence> {
        let evidence = classifier_free_sub_d_frame_evidence(
            descriptor,
            position_raw,
            target_raw,
            right_q31,
            forward_q31,
        )?;
        self.cache.begin_frame();
        Some(evidence)
    }

    /// Execute one `FUN_0041FCB0` lookup. A pending first-query route full-
    /// resets onto the supplied cell; unresolved constructor residue stays
    /// unresolved and does not mutate.
    pub fn apply_first_query(
        &mut self,
        terrain: &TerrainGrid,
        point_raw: [i16; 2],
    ) -> RetailRuntimeValue<u16> {
        self.cache.classify(terrain, point_raw)
    }

    /// Execute the cache/projection portion of one ordinary type-9 Sub-D call.
    ///
    /// Query order is retail's left, half-forward, right, then conditional
    /// rear sequence. The rear cell is not touched when the combined central
    /// class is clear, preserving both cache fill order and stale-entry timing.
    pub fn evidence_for_frame(
        &mut self,
        terrain: &TerrainGrid,
        position_raw: [i16; 3],
        target_raw: [i16; 3],
        right_q31: [i32; 3],
        forward_q31: [i32; 3],
        direction_multiplier: i32,
    ) -> Type9SubDFrameEvidence {
        self.evidence_for_frame_with_descriptor(
            ORDINARY_TYPE9_SUB_D,
            terrain,
            position_raw,
            target_raw,
            right_q31,
            forward_q31,
            direction_multiplier,
        )
    }

    /// Same left / half-forward / right / conditional-rear sequence as
    /// [`Self::evidence_for_frame`], classified with the descriptor's flags
    /// and probe distances.
    pub fn evidence_for_frame_with_descriptor(
        &mut self,
        descriptor: SubDSteeringDescriptor,
        terrain: &TerrainGrid,
        position_raw: [i16; 3],
        target_raw: [i16; 3],
        right_q31: [i32; 3],
        forward_q31: [i32; 3],
        direction_multiplier: i32,
    ) -> Type9SubDFrameEvidence {
        if descriptor.classifier_flags == 0 {
            return self
                .evidence_for_classifier_free_frame(
                    descriptor,
                    position_raw,
                    target_raw,
                    right_q31,
                    forward_q31,
                )
                .expect("zero classifier flags admit the query-free frame route");
        }
        let target_lateral_raw = RetailRuntimeValue::Known(target_lateral_projection_raw(
            position_raw,
            target_raw,
            right_q31,
            forward_q31,
        ));
        if !self.cache.can_classify() {
            return Type9SubDFrameEvidence {
                target_lateral_raw,
                classifier_evidence: RetailRuntimeValue::Unresolved,
            };
        }

        let probes = type9_probe_plan(
            position_raw,
            right_q31,
            forward_q31,
            direction_multiplier,
            descriptor,
        );
        let flags = descriptor.classifier_flags;
        let samples = self.cache.transact(|transaction| {
            let query = |transaction: &mut GenericSubDCacheTransaction, point_raw| {
                transaction.query(point_raw, |_| {
                    Ok::<u16, Infallible>(classify_terrain(terrain, point_raw, flags))
                })
            };
            let left = query(transaction, probes.left_raw)?;
            let half_forward = query(transaction, probes.half_forward_raw)?;
            let right = query(transaction, probes.right_raw)?;
            let mut combined = half_forward;
            if left != 0 && left == right {
                combined |= left;
            }
            let rear = if combined != 0 {
                query(transaction, probes.rear_raw)?
            } else {
                0
            };
            Ok(Type9SubDClassifierSamples {
                left,
                half_forward,
                right,
                rear,
                terrain_heights: RetailRuntimeValue::Known(Type9SubDTerrainHeights {
                    left_raw: terrain_height_raw(terrain, probes.left_raw[0], probes.left_raw[1]),
                    current_raw: terrain_height_raw(terrain, position_raw[0], position_raw[2]),
                    right_raw: terrain_height_raw(
                        terrain,
                        probes.right_raw[0],
                        probes.right_raw[1],
                    ),
                }),
            })
        });
        let samples = match samples {
            Ok(samples) => samples,
            Err(GenericSubDCacheTransactionBlock::UnresolvedOrigin) => {
                return Type9SubDFrameEvidence {
                    target_lateral_raw,
                    classifier_evidence: RetailRuntimeValue::Unresolved,
                };
            }
            Err(GenericSubDCacheTransactionBlock::Classifier(error)) => match error {},
        };

        Type9SubDFrameEvidence {
            target_lateral_raw,
            classifier_evidence: RetailRuntimeValue::Known(Type9SubDClassifierEvidence::Samples(
                samples,
            )),
        }
    }
}

/// Results produced by the still-separate `FUN_0041FCB0` cache owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type9SubDClassifierSamples {
    pub left: u16,
    pub half_forward: u16,
    pub right: u16,
    pub rear: u16,
    pub terrain_heights: RetailRuntimeValue<Type9SubDTerrainHeights>,
}

/// Explicit result of retail's descriptor `+0x0A` classifier gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type9SubDClassifierEvidence {
    /// Zero flags branched around every `FUN_0041FCB0` call.
    Bypassed,
    /// A nonzero flags byte issued the ordered classifier probes.
    Samples(Type9SubDClassifierSamples),
}

/// Per-frame inputs whose live owners are not part of this bounded phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type9SubDFrameEvidence {
    /// Lateral target projection returned through `FUN_0041E700`.
    pub target_lateral_raw: RetailRuntimeValue<i16>,
    /// Authored bypass or four mutable-cache results in retail call order.
    pub classifier_evidence: RetailRuntimeValue<Type9SubDClassifierEvidence>,
}

/// Build the query-free steering evidence for descriptors which bypass
/// `FUN_0041FCB0` entirely.
///
/// Retail checks descriptor byte `+0x0A` before issuing any classifier call.
/// A zero byte jumps directly to the shared steering tail, so no terrain or
/// mutable cache lookup participates in this route. The allocation's stagger
/// byte and periodic row clear still run before this branch; live owners must
/// use [`Type9SubDFrameOwner::evidence_for_classifier_free_frame`] to retain
/// those writes. The descriptor guard keeps callers from using zero samples as
/// a stand-in for an unresolved nonzero classifier policy.
fn classifier_free_sub_d_frame_evidence(
    descriptor: SubDSteeringDescriptor,
    position_raw: [i16; 3],
    target_raw: [i16; 3],
    right_q31: [i32; 3],
    forward_q31: [i32; 3],
) -> Option<Type9SubDFrameEvidence> {
    if descriptor.classifier_flags != 0 {
        return None;
    }
    Some(Type9SubDFrameEvidence {
        target_lateral_raw: RetailRuntimeValue::Known(target_lateral_projection_raw(
            position_raw,
            target_raw,
            right_q31,
            forward_q31,
        )),
        classifier_evidence: RetailRuntimeValue::Known(Type9SubDClassifierEvidence::Bypassed),
    })
}

/// Deterministic type-9 subset of retail's 0x3c-byte Sub-D runtime.
///
/// Cache bytes and the stagger counter belong to [`Type9SubDFrameOwner`];
/// native construction supplies both owners through [`construct_native_sub_d`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Type9SubDRuntime {
    pub yaw_rate_raw: i32,
    pub last_yaw_step_raw: i16,
}

impl Type9SubDRuntime {
    /// Exact deterministic words written by retail `FUN_004203D0`.
    ///
    /// The constructor clears the yaw-rate dword at `+0x04` and the published
    /// yaw-step dword at `+0x10` represented here. It also clears the
    /// pitch-rate and published pitch-step dwords at `+0x08/+0x14`; `+0x0c`
    /// receives the descriptor's yaw/roll-coupling byte. Cache rows, origin,
    /// and the process-owned stagger byte remain in [`Type9SubDFrameOwner`]
    /// because their provenance is intentionally different.
    pub const fn from_constructor() -> Self {
        Self {
            yaw_rate_raw: 0,
            last_yaw_step_raw: 0,
        }
    }
}

/// Result of one bounded type-9 Sub-D attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type9SubDStep {
    Applied { yaw_step_raw: i16 },
    UnsupportedDescriptor,
    UnresolvedTargetProjection,
    UnresolvedClassifierCache,
    ClassifierEvidenceMismatch,
    UnresolvedTerrainHeights,
}

/// Construct the four X/Z probes from `FUN_0041F660`.
///
/// Retail subtracts the rounded Q31 lateral product at `0041F740/0041F78D`
/// and the rounded half-forward product at `0041F908/0041F936`. Negating
/// either scale before multiplication changes nonintegral products by one
/// raw unit and can select an adjacent classifier cell.
pub fn type9_probe_plan(
    position_raw: [i16; 3],
    right_q31: [i32; 3],
    forward_q31: [i32; 3],
    direction_multiplier: i32,
    descriptor: SubDSteeringDescriptor,
) -> Type9SubDProbePlan {
    let forward_scale = direction_multiplier.wrapping_mul(i32::from(descriptor.forward_probe_raw));
    let lateral_scale = direction_multiplier.wrapping_mul(i32::from(descriptor.lateral_probe_raw));
    let half_forward_scale = forward_scale / 2;

    let point = |offset: [i32; 2]| {
        [
            position_raw[0].wrapping_add(offset[0] as i16),
            position_raw[2].wrapping_add(offset[1] as i16),
        ]
    };
    let forward = [0, 2].map(|axis| q31_mul(forward_q31[axis], forward_scale));
    let lateral = [0, 2].map(|axis| q31_mul(right_q31[axis], lateral_scale));
    let half_forward = [0, 2].map(|axis| q31_mul(forward_q31[axis], half_forward_scale));

    Type9SubDProbePlan {
        left_raw: point([0, 1].map(|axis| forward[axis].wrapping_sub(lateral[axis]))),
        half_forward_raw: point(half_forward),
        right_raw: point([0, 1].map(|axis| forward[axis].wrapping_add(lateral[axis]))),
        rear_raw: point(half_forward.map(i32::wrapping_neg)),
    }
}

/// Exact lateral half-word returned through the first output of
/// `FUN_0041E700`.
///
/// Position subtraction wraps in signed 16-bit world space before the vector
/// is normalized. The unusual negative-axis saturation is retained from
/// retail: a component whose magnitude exactly equals the vector length
/// becomes Q31 `-1`, not `i32::MIN`. Retail also computes the forward
/// projection and replaces the lateral result with signed `0x7fff` whenever
/// the target is behind the actor.
pub fn target_lateral_projection_raw(
    position_raw: [i16; 3],
    target_raw: [i16; 3],
    right_q31: [i32; 3],
    forward_q31: [i32; 3],
) -> i16 {
    target_body_projection_raw(
        position_raw,
        target_raw,
        super::type9_attitude::Type9BodyBasis {
            lateral: right_q31,
            up: [0; 3],
            forward: forward_q31,
        },
    )
    .lateral_raw
}

/// The two signed output words of `FUN_0041E700`. The behind-target override
/// changes only the lateral output; Sub-L also consumes the vertical output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetBodyProjection {
    pub lateral_raw: i16,
    pub vertical_raw: i16,
}

/// Project a wrapped world-space target through the retained body matrix.
/// This shares Sub-D's exact normalization, including negative-axis saturation.
pub fn target_body_projection_raw(
    position_raw: [i16; 3],
    target_raw: [i16; 3],
    retained_body_basis: super::type9_attitude::Type9BodyBasis,
) -> TargetBodyProjection {
    let mut displacement: [i32; 3] =
        std::array::from_fn(|axis| i32::from(target_raw[axis].wrapping_sub(position_raw[axis])));
    let mut magnitude_or = displacement
        .into_iter()
        .fold(0i32, |acc, value| acc | value.wrapping_abs());
    while magnitude_or >= 0x6883 {
        magnitude_or >>= 1;
        for component in &mut displacement {
            *component >>= 1;
        }
    }

    let length_squared = displacement.into_iter().fold(0u32, |sum, component| {
        sum.wrapping_add((component.wrapping_mul(component)) as u32)
    });
    let length = integer_sqrt(length_squared) as i32;
    let normalized = displacement.map(|component| normalized_component_q31(component, length));
    let lateral_projection = retained_body_basis
        .lateral
        .into_iter()
        .zip(normalized)
        .fold(0i32, |sum, (basis, component)| {
            sum.wrapping_add(q31_mul(component, basis))
        });
    let lateral_raw = (lateral_projection >> 16) as i16;
    let vertical_projection = retained_body_basis
        .up
        .into_iter()
        .zip(normalized)
        .fold(0i32, |sum, (basis, component)| {
            sum.wrapping_add(q31_mul(component, basis))
        });
    let forward_projection = retained_body_basis
        .forward
        .into_iter()
        .zip(normalized)
        .fold(0i32, |sum, (basis, component)| {
            sum.wrapping_add(q31_mul(component, basis))
        });
    let lateral_raw = if forward_projection < 0 {
        // FUN_0041E700 forces a full turn when the normalized target lies
        // behind the actor. Its `< 1` test gives an exactly centered target
        // the same negative-turn bias as a target on the left.
        if lateral_raw < 1 {
            -0x7fff
        } else {
            0x7fff
        }
    } else {
        lateral_raw
    };
    TargetBodyProjection {
        lateral_raw,
        vertical_raw: (vertical_projection >> 16) as i16,
    }
}

fn classify_type9_terrain(terrain: &TerrainGrid, point_raw: [i16; 2]) -> u16 {
    classify_terrain(terrain, point_raw, ORDINARY_TYPE9_SUB_D.classifier_flags)
}

fn classify_terrain(terrain: &TerrainGrid, point_raw: [i16; 2], flags: u8) -> u16 {
    let [x_raw, z_raw] = point_raw.map(|word| word as u16);

    // 41FEB0 tests bit0x20 before every other classifier arm. It reads one
    // signed cell height, not the later water arm's four-corner footprint.
    // Exactly256 raw units of water remains traversable; shallower water
    // and land return6. Both operands widen AFTER the retail signed words.
    if flags & 0x20 != 0
        && i32::from(terrain.sea_level_raw()) - i32::from(cell_height_raw(terrain, x_raw, z_raw))
            < 0x100
    {
        return 6;
    }

    // classifier_flags bit 0x10: reject a 257×257-raw footprint whose four
    // authored cell heights differ by more than 0x15E.
    if flags & 0x10 != 0 {
        let footprint_81 = [
            cell_height_raw(terrain, x_raw.wrapping_sub(0x81), z_raw.wrapping_sub(0x81)),
            cell_height_raw(terrain, x_raw.wrapping_add(0x81), z_raw.wrapping_sub(0x81)),
            cell_height_raw(terrain, x_raw.wrapping_add(0x81), z_raw.wrapping_add(0x81)),
            cell_height_raw(terrain, x_raw.wrapping_sub(0x81), z_raw.wrapping_add(0x81)),
        ];
        let minimum = footprint_81.into_iter().min().unwrap();
        let maximum = footprint_81.into_iter().max().unwrap();
        if i32::from(maximum) - i32::from(minimum) > 0x15e {
            return 4;
        }
    }

    // classifier_flags bit 0x02: any corner below the authored sea plane.
    if flags & 0x02 != 0 {
        let footprint_80 = [
            cell_height_raw(terrain, x_raw.wrapping_sub(0x80), z_raw.wrapping_sub(0x80)),
            cell_height_raw(terrain, x_raw.wrapping_add(0x80), z_raw.wrapping_sub(0x80)),
            cell_height_raw(terrain, x_raw.wrapping_sub(0x80), z_raw.wrapping_add(0x80)),
            cell_height_raw(terrain, x_raw.wrapping_add(0x80), z_raw.wrapping_add(0x80)),
        ];
        if footprint_80
            .into_iter()
            .any(|height| height < terrain.sea_level_raw())
        {
            return 5;
        }
    }

    let x = usize::from((x_raw >> 8) as u8);
    let z = usize::from((z_raw >> 8) as u8);
    let cell = terrain
        .cell(x, z)
        .expect("complete 256x256 terrain classifier grid");
    let packed_kind = u32::from(cell.attribute) + (u32::from(cell.terrain_type & 0xf8) << 8);

    // classifier_flags bit 0x01. Type 9 does not set bit 0x08, so no static
    // object-kind lookup participates in this branch. Type-26 `0x12` skips it;
    // Type-47 `0x13` takes it.
    if flags & 0x01 != 0 && cell.attribute != 0 && packed_kind & 0x0800 == 0 {
        return 2;
    }
    // classifier_flags bit 0x04. Type-26 `0x12` skips it.
    if flags & 0x04 != 0 && packed_kind & 0x1000 != 0 {
        return 1;
    }
    0
}

fn cell_height_raw(terrain: &TerrainGrid, x_raw: u16, z_raw: u16) -> i16 {
    let x = usize::from((x_raw >> 8) as u8);
    let z = usize::from((z_raw >> 8) as u8);
    i16::from(
        terrain
            .cell(x, z)
            .expect("complete 256x256 terrain classifier grid")
            .height as i8,
    ) << 5
}

fn terrain_height_raw(terrain: &TerrainGrid, x_raw: i16, z_raw: i16) -> i16 {
    let x_word = x_raw as u16;
    let z_word = z_raw as u16;
    let x0 = usize::from(x_word >> 8);
    let z0 = usize::from(z_word >> 8);
    let x1 = (x0 + 1) & (GRID_SIZE - 1);
    let z1 = (z0 + 1) & (GRID_SIZE - 1);
    let x_fraction = i32::from(x_word & 0xff);
    let z_fraction = i32::from(z_word & 0xff);
    let height = |x, z| {
        i32::from(
            terrain
                .cell(x, z)
                .expect("complete 256x256 terrain classifier grid")
                .height as i8,
        ) << 5
    };

    let h00 = height(x0, z0);
    let h10 = height(x1, z0);
    let h01 = height(x0, z1);
    let h11 = height(x1, z1);
    let along_x0 = (((h10 - h00) * x_fraction) >> 8) + h00;
    let along_x1 = (((h11 - h01) * x_fraction) >> 8) + h01;
    ((((along_x1 - along_x0) * z_fraction) >> 8) + along_x0) as i16
}

fn integer_sqrt(mut value: u32) -> u32 {
    let mut root = 0u32;
    let mut bit = 1u32 << 30;
    while bit > value {
        bit >>= 2;
    }
    while bit != 0 {
        if value >= root + bit {
            value -= root + bit;
            root = (root >> 1) + bit;
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    root
}

fn normalized_component_q31(component: i32, length: i32) -> i32 {
    if component.wrapping_abs() < length.wrapping_abs() {
        ((i64::from(component) << 31) / i64::from(length)) as i32
    } else {
        (component ^ length) | i32::MAX
    }
}

/// Apply the exact deterministic type-9 subset of `FUN_0041F660`.
///
/// `elapsed_micros` is the mover argument used by `FUN_00420260`;
/// `global_elapsed_micros` is retail `DAT_004D04E4`, used independently by
/// `FUN_00420360`. The runtime is committed only after every input required by
/// the selected branch is known.
pub fn apply_type9_sub_d(
    descriptor: SubDSteeringDescriptor,
    runtime: &mut Type9SubDRuntime,
    evidence: Type9SubDFrameEvidence,
    elapsed_micros: u32,
    global_elapsed_micros: u32,
) -> Type9SubDStep {
    if !admits_shared_fun_0041f660_steering(descriptor) {
        return Type9SubDStep::UnsupportedDescriptor;
    }
    let target_lateral_raw = match evidence.target_lateral_raw {
        RetailRuntimeValue::Known(value) => value,
        RetailRuntimeValue::Unresolved => return Type9SubDStep::UnresolvedTargetProjection,
    };
    let classifier_evidence = match evidence.classifier_evidence {
        RetailRuntimeValue::Known(value) => value,
        RetailRuntimeValue::Unresolved => return Type9SubDStep::UnresolvedClassifierCache,
    };

    let mut next_yaw_rate = target_steering_rate(
        runtime.yaw_rate_raw,
        target_lateral_raw,
        elapsed_micros,
        descriptor.steering_divisor_raw,
    );
    match classifier_evidence {
        Type9SubDClassifierEvidence::Bypassed if descriptor.classifier_flags != 0 => {
            return Type9SubDStep::ClassifierEvidenceMismatch;
        }
        Type9SubDClassifierEvidence::Bypassed => {}
        Type9SubDClassifierEvidence::Samples(_) if descriptor.classifier_flags == 0 => {
            return Type9SubDStep::ClassifierEvidenceMismatch;
        }
        Type9SubDClassifierEvidence::Samples(samples) => {
            let mut center = samples.half_forward;
            if samples.left != 0 && samples.left == samples.right {
                center |= samples.left;
            }

            if center == 0 {
                if samples.left < samples.right {
                    next_yaw_rate = forced_yaw_rate(-0x1_0000, descriptor.steering_divisor_raw);
                } else if samples.left > samples.right {
                    next_yaw_rate = forced_yaw_rate(0x1_0000, descriptor.steering_divisor_raw);
                }
            } else if center >= samples.rear {
                next_yaw_rate = match center {
                    4 | 5 => {
                        let heights = match samples.terrain_heights {
                            RetailRuntimeValue::Known(value) => value,
                            RetailRuntimeValue::Unresolved => {
                                return Type9SubDStep::UnresolvedTerrainHeights;
                            }
                        };
                        let turn_positive = if center == 5 {
                            heights.right_raw > heights.left_raw
                        } else {
                            let average =
                                (i32::from(heights.right_raw) + i32::from(heights.left_raw)) / 2;
                            if average < i32::from(heights.current_raw) {
                                heights.left_raw < heights.right_raw
                            } else {
                                heights.right_raw < heights.left_raw
                            }
                        };
                        forced_yaw_rate(
                            if turn_positive { 0x2_0000 } else { -0x2_0000 },
                            descriptor.steering_divisor_raw,
                        )
                    }
                    _ => forced_yaw_rate(0x1_8000, descriptor.steering_divisor_raw),
                };
            }
        }
    }

    let yaw_step_raw = integrate_yaw_step(next_yaw_rate, global_elapsed_micros);
    *runtime = Type9SubDRuntime {
        yaw_rate_raw: next_yaw_rate,
        last_yaw_step_raw: yaw_step_raw,
    };
    Type9SubDStep::Applied { yaw_step_raw }
}

pub(crate) fn target_steering_rate(
    current: i32,
    target_lateral_raw: i16,
    elapsed_micros: u32,
    steering_divisor_raw: i32,
) -> i32 {
    let damping_product = (elapsed_micros >> 5).wrapping_mul(current as u32) as i32;
    let damping = (damping_product >> 15) as i16;
    let mut rate = current.wrapping_sub(i32::from(damping));
    if target_lateral_raw == 0 {
        return rate;
    }

    let target = i32::from(target_lateral_raw);
    let desired = target.wrapping_shl(10) / steering_divisor_raw;
    let shift = if target.wrapping_mul(rate) < 0 { 7 } else { 9 };
    rate = rate.wrapping_add(desired >> shift);
    let limit = 0x2_0000 / steering_divisor_raw;
    if desired < 1 {
        rate.max(-limit)
    } else {
        rate.min(limit)
    }
}

fn forced_yaw_rate(numerator: i32, divisor: i32) -> i32 {
    (numerator / divisor).clamp(-0x4000, 0x4000)
}

pub(crate) fn integrate_yaw_step(yaw_rate_raw: i32, global_elapsed_micros: u32) -> i16 {
    let product = yaw_rate_raw.wrapping_mul((global_elapsed_micros >> 2) as i32);
    (product >> 15) as i16
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::terrain::TerrainCell;

    #[test]
    fn generic_constructor_reproduces_the_exact_0x3c_write_set() {
        let mut counter = SubDAllocationCounter::from_next_seed(0xfe);
        let mut descriptor = SubDSteeringDescriptor {
            steering_divisor_raw: 17,
            couple_yaw_into_roll_raw: -7,
            enable_pitch_steering_raw: 1,
            forward_probe_raw: 0,
            lateral_probe_raw: 0,
            classifier_flags: 0x53,
            reserved_at_0x0b: 0xa4,
        };
        let mut slot = None;
        assert_eq!(
            construct_generic_sub_d(
                &mut counter,
                &mut descriptor,
                &mut slot,
                GenericSubDAllocationAttempt::Succeeded(GenericSubDAllocatorResidue {
                    retained_at_0x00: 0x1122_3344,
                    cache_origin: [0xab, 0xcd],
                    retained_at_0x3b: 0x77,
                }),
            ),
            GenericSubDConstructorResult::Constructed
        );

        assert_eq!(
            (descriptor.forward_probe_raw, descriptor.lateral_probe_raw),
            (
                GENERIC_SUB_D_DEFAULT_FORWARD_PROBE_RAW,
                GENERIC_SUB_D_DEFAULT_LATERAL_PROBE_RAW
            )
        );
        assert_eq!(counter.next_seed(), 0xff);
        let runtime = slot.expect("successful allocation publishes one runtime");
        assert_eq!(runtime.retained_at_0x00(), 0x1122_3344);
        assert_eq!(runtime.couple_yaw_into_roll_raw(), -7);
        assert_eq!(runtime.classifier_cache().rows(), [0; 8]);
        assert_eq!(
            runtime.classifier_cache().origin(),
            RetailRuntimeValue::Known([0xab, 0xcd])
        );
        assert_eq!(runtime.classifier_cache().stagger_counter(), 0xfe);
        assert_eq!(runtime.retained_at_0x3b(), 0x77);

        let bytes = runtime.to_retail_bytes();
        assert_eq!(&bytes[0x00..0x04], &0x1122_3344u32.to_le_bytes());
        assert_eq!(&bytes[0x04..0x0c], &[0; 8]);
        assert_eq!(&bytes[0x0c..0x10], &(-7i32).to_le_bytes());
        assert_eq!(&bytes[0x10..0x38], &[0; 40]);
        assert_eq!(&bytes[0x38..0x3c], &[0xab, 0xcd, 0xfe, 0x77]);
    }

    #[test]
    fn generic_constructor_preserves_nonzero_probe_words() {
        let mut counter = SubDAllocationCounter::default();
        let mut descriptor = ORDINARY_TYPE9_SUB_D;
        let expected = descriptor;
        let mut slot = None;
        assert_eq!(
            construct_generic_sub_d(
                &mut counter,
                &mut descriptor,
                &mut slot,
                GenericSubDAllocationAttempt::Succeeded(GenericSubDAllocatorResidue {
                    retained_at_0x00: 0,
                    cache_origin: [0, 0],
                    retained_at_0x3b: 0,
                }),
            ),
            GenericSubDConstructorResult::Constructed
        );
        assert_eq!(descriptor, expected);
    }

    #[test]
    fn failed_generic_allocation_is_atomic() {
        let mut seed_counter = SubDAllocationCounter::from_next_seed(7);
        let mut seed_descriptor = ORDINARY_TYPE9_SUB_D;
        let mut slot = None;
        construct_generic_sub_d(
            &mut seed_counter,
            &mut seed_descriptor,
            &mut slot,
            GenericSubDAllocationAttempt::Succeeded(GenericSubDAllocatorResidue {
                retained_at_0x00: 0xdead_beef,
                cache_origin: [2, 3],
                retained_at_0x3b: 4,
            }),
        );

        let original_slot = slot;
        let mut counter = SubDAllocationCounter::from_next_seed(0x42);
        let mut descriptor = SubDSteeringDescriptor {
            forward_probe_raw: 0,
            lateral_probe_raw: 0,
            ..ORDINARY_TYPE9_SUB_D
        };
        let original_descriptor = descriptor;
        assert_eq!(
            construct_generic_sub_d(
                &mut counter,
                &mut descriptor,
                &mut slot,
                GenericSubDAllocationAttempt::Failed,
            ),
            GenericSubDConstructorResult::AllocationFailed
        );
        assert_eq!(counter.next_seed(), 0x42);
        assert_eq!(descriptor, original_descriptor);
        assert_eq!(slot, original_slot);
    }

    #[test]
    fn native_constructor_uses_process_order_across_profiles_and_preserves_defaults() {
        let mut counter = SubDAllocationCounter::from_next_seed(0xfe);
        let authored = SubDSteeringDescriptor {
            steering_divisor_raw: 17,
            couple_yaw_into_roll_raw: -7,
            enable_pitch_steering_raw: 1,
            forward_probe_raw: 0,
            lateral_probe_raw: 0,
            classifier_flags: 0x53,
            reserved_at_0x0b: 0xa4,
        };
        let first = construct_native_sub_d(&mut counter, authored);
        assert_eq!(first.seed, 0xfe);
        assert_eq!(
            first.descriptor,
            SubDSteeringDescriptor {
                forward_probe_raw: 0x0200,
                lateral_probe_raw: 0x0100,
                ..authored
            }
        );
        assert_eq!(first.runtime, Type9SubDRuntime::from_constructor());
        assert_eq!(first.frame_owner.classifier_cache().rows(), [0; 8]);
        assert_eq!(first.frame_owner.classifier_cache().stagger_counter(), 0xfe);
        assert_eq!(
            first.frame_owner.classifier_cache().origin(),
            RetailRuntimeValue::Unresolved
        );
        assert!(first.frame_owner.classifier_cache().can_classify());

        assert_eq!(counter.finish_attempt(SubDAllocationOutcome::Failed), None);
        assert_eq!(counter.next_seed(), 0xff);
        // Even a descriptor that never queries terrain owns its allocation.
        let flyer = construct_native_sub_d(&mut counter, FLYER_SUB_D);
        assert_eq!(flyer.seed, 0xff);
        assert_eq!(flyer.descriptor, FLYER_SUB_D);
        assert_eq!(counter.next_seed(), 0);
        let peasant = construct_native_sub_d(&mut counter, ORDINARY_TYPE9_SUB_D);
        assert_eq!(peasant.seed, 0);
        assert_eq!(peasant.descriptor, ORDINARY_TYPE9_SUB_D);
        assert_eq!(counter.next_seed(), 1);
    }

    #[test]
    fn native_first_classification_matches_all_empty_retail_origins_without_claiming_same_window() {
        let mut counter = SubDAllocationCounter::default();
        let mut native = construct_native_sub_d(&mut counter, ORDINARY_TYPE9_SUB_D)
            .frame_owner
            .cache;
        let query = [0x0b80, 0x0c40];
        assert_eq!(native.query_with(query, |_| Ok::<_, Infallible>(3)), Ok(3));
        assert_eq!(native.origin(), RetailRuntimeValue::Known([11, 12]));
        assert_eq!(native.rows(), [4, 0, 0, 0, 0, 0, 0, 0]);
        for x in 0..=u8::MAX {
            for z in 0..=u8::MAX {
                let mut retail = GenericSubDClassifierCache::from_retail_state([0; 8], [x, z], 0);
                assert_eq!(
                    retail.query_with(query, |_| Ok::<_, Infallible>(3)),
                    Ok(3),
                    "empty retail origin {x},{z} must classify the first query"
                );
            }
        }
        let mut nearby_retail = GenericSubDClassifierCache::from_retail_state([0; 8], [10, 10], 0);
        assert_eq!(
            nearby_retail.query_with(query, |_| Ok::<_, Infallible>(3)),
            Ok(3)
        );
        assert_eq!(nearby_retail.origin(), RetailRuntimeValue::Known([10, 10]));
        assert_ne!(nearby_retail.rows(), native.rows());
    }

    #[test]
    fn native_cache_retains_first_query_state_and_rolls_back_failed_query_transactions() {
        let mut counter = SubDAllocationCounter::from_next_seed(0xff);
        let mut cache = construct_native_sub_d(&mut counter, ORDINARY_TYPE9_SUB_D)
            .frame_owner
            .cache;
        let before = cache;
        let first = [0x0b00, 0x0c00];
        let second = [0x0c00, 0x0c00];
        assert_eq!(
            cache.transact(|transaction| {
                transaction.query(first, |_| Ok::<_, ()>(3))?;
                transaction.query(second, |_| Err(()))
            }),
            Err(GenericSubDCacheTransactionBlock::Classifier(()))
        );
        assert_eq!(cache, before);
        // The failed consumer is unrelated to successful construction custody.
        assert_eq!(counter.next_seed(), 0);
        assert_eq!(
            cache.transact(|transaction| {
                transaction.query(first, |_| Ok::<_, Infallible>(3))?;
                transaction.query(second, |_| Ok::<_, Infallible>(5))
            }),
            Ok(5)
        );
        assert_eq!(cache.stagger_counter(), 0);
        assert_eq!(cache.origin(), RetailRuntimeValue::Known([11, 12]));
        assert_eq!(cache.rows(), [0x64, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            cache.transact(|transaction| {
                transaction.query(second, |_| -> Result<u16, Infallible> {
                    panic!("a later frame must reuse the retained cache entry")
                })
            }),
            Ok(5)
        );
        assert_eq!(cache.stagger_counter(), 1);
        assert_eq!(cache.origin(), RetailRuntimeValue::Known([11, 12]));
        assert_eq!(cache.rows()[0], 0x64);
    }

    #[test]
    fn native_classifier_free_frames_advance_stagger_without_choosing_an_origin() {
        for descriptor in [FLYER_SUB_D, ORDINARY_TYPE7_SUB_D] {
            let mut counter = SubDAllocationCounter::from_next_seed(0xfe);
            let mut construction = construct_native_sub_d(&mut counter, descriptor);
            for expected_stagger in [0xff, 0] {
                assert!(construction
                    .frame_owner
                    .evidence_for_classifier_free_frame(
                        construction.descriptor,
                        [0; 3],
                        [0, 0, 256],
                        [i32::MAX, 0, 0],
                        [0, 0, i32::MAX],
                    )
                    .is_some());
                let cache = construction.frame_owner.classifier_cache();
                assert_eq!(cache.stagger_counter(), expected_stagger);
                assert_eq!(cache.origin(), RetailRuntimeValue::Unresolved);
                assert_eq!(cache.rows(), [0; 8]);
            }
            assert_eq!(counter.next_seed(), 0xff);
        }
    }

    #[test]
    fn ordinary_type9_constructor_starts_both_retained_steering_words_at_zero() {
        assert_eq!(
            Type9SubDRuntime::from_constructor(),
            Type9SubDRuntime {
                yaw_rate_raw: 0,
                last_yaw_step_raw: 0,
            }
        );
    }

    fn terrain(height: i8, sea_level_raw: i16) -> TerrainGrid {
        TerrainGrid {
            header: [i32::from(sea_level_raw) << 8, 0, 0, 0, 0],
            cells: vec![
                TerrainCell {
                    height: height as u8,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn cell_mut(terrain: &mut TerrainGrid, x: usize, z: usize) -> &mut TerrainCell {
        &mut terrain.cells[x * GRID_SIZE + z]
    }

    fn empty_cache(stagger_seed: u8, origin: [u8; 2]) -> Type9SubDClassifierCache {
        Type9SubDClassifierCache::from_retail_state([0; 8], origin, stagger_seed)
    }

    fn classify(
        cache: &mut Type9SubDClassifierCache,
        terrain: &TerrainGrid,
        point_raw: [i16; 2],
    ) -> u16 {
        let RetailRuntimeValue::Known(class) = cache.classify(terrain, point_raw) else {
            panic!("test fixture supplies exact cache-origin bytes");
        };
        class
    }

    #[test]
    fn process_allocation_counter_copies_before_increment_and_skips_failures() {
        let mut counter = SubDAllocationCounter::default();
        assert_eq!(counter.finish_attempt(SubDAllocationOutcome::Failed), None);
        assert_eq!(counter.next_seed(), 0);
        assert_eq!(
            counter.finish_attempt(SubDAllocationOutcome::Succeeded),
            Some(0)
        );
        assert_eq!(
            counter.finish_attempt(SubDAllocationOutcome::Succeeded),
            Some(1)
        );
        assert_eq!(counter.next_seed(), 2);

        let mut wrapping = SubDAllocationCounter::from_next_seed(0xff);
        assert_eq!(
            wrapping.finish_attempt(SubDAllocationOutcome::Succeeded),
            Some(0xff)
        );
        assert_eq!(wrapping.next_seed(), 0);
    }

    #[test]
    fn generic_cache_transaction_rolls_back_stagger_and_partial_window_writes() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct Blocked;

        let mut cache = GenericSubDClassifierCache::from_retail_state(
            [0x1111_1111, 0x2222_2222, 0, 0, 0, 0, 0, 0],
            [10, 10],
            7,
        );
        let before = cache;
        let result = cache.transact(|transaction| {
            transaction.query([0x1200, 0x0a00], |_| Ok::<_, Blocked>(0))?;
            transaction.query([0x1200, 0x1200], |_| Err(Blocked))?;
            Ok(())
        });
        assert_eq!(
            result,
            Err(GenericSubDCacheTransactionBlock::Classifier(Blocked))
        );
        assert_eq!(
            cache, before,
            "the row-1 stagger clear, positive-X shift, and first fill all roll back"
        );
    }

    #[test]
    fn generic_cache_transaction_commits_one_stagger_step_and_all_queries() {
        let mut cache = GenericSubDClassifierCache::from_retail_state([0; 8], [10, 10], 7);
        let result = cache
            .transact(|transaction| {
                let first = transaction.query([0x1200, 0x0a00], |_| Ok::<_, Infallible>(2))?;
                let second = transaction.query([0x1200, 0x1200], |_| Ok::<_, Infallible>(4))?;
                Ok((first, second))
            })
            .expect("known origin and infallible classifier");
        assert_eq!(result, (2, 4));
        assert_eq!(cache.stagger_counter(), 8);
        assert_eq!(cache.origin(), RetailRuntimeValue::Known([11, 11]));
        assert_eq!(
            cache.rows()[0] & 0xf000_0000,
            0x3000_0000,
            "positive Z changes only the origin, preserving the positive-X fill in row zero"
        );
        assert_eq!(cache.rows()[7] & 0xf000_0000, 0x5000_0000);
    }

    fn known(samples: Type9SubDClassifierSamples, lateral: i16) -> Type9SubDFrameEvidence {
        Type9SubDFrameEvidence {
            target_lateral_raw: RetailRuntimeValue::Known(lateral),
            classifier_evidence: RetailRuntimeValue::Known(Type9SubDClassifierEvidence::Samples(
                samples,
            )),
        }
    }

    fn samples(left: u16, center: u16, right: u16, rear: u16) -> Type9SubDClassifierSamples {
        Type9SubDClassifierSamples {
            left,
            half_forward: center,
            right,
            rear,
            terrain_heights: RetailRuntimeValue::Unresolved,
        }
    }

    #[test]
    fn exact_type9_probe_plan_uses_authored_forward_and_lateral_distances() {
        let plan = type9_probe_plan(
            [1_000, 25, 2_000],
            [i32::MAX, 0, 0],
            [0, 0, i32::MAX],
            1,
            ORDINARY_TYPE9_SUB_D,
        );
        assert_eq!(plan.left_raw, [937, 2_127]);
        assert_eq!(plan.half_forward_raw, [1_000, 2_063]);
        assert_eq!(plan.right_raw, [1_063, 2_127]);
        assert_eq!(plan.rear_raw, [1_000, 1_937]);

        let reverse = type9_probe_plan(
            [1_000, 25, 2_000],
            [i32::MAX, 0, 0],
            [0, 0, i32::MAX],
            -1,
            ORDINARY_TYPE9_SUB_D,
        );
        let type26 = type9_probe_plan(
            [0xC200u16 as i16, 0, 0x0E00],
            [i32::MAX, 0, 0],
            [0, 0, i32::MAX],
            1,
            INTRO2_TYPE26_SUB_D,
        );
        assert_eq!(
            type26.left_raw,
            [0xC139u16 as i16, 0x0FFF],
            "retail subtracts the rounded positive lateral product, 199"
        );
        assert_eq!(reverse.left_raw, [1_064, 1_872]);
        assert_eq!(reverse.right_raw, [936, 1_872]);
    }

    #[test]
    fn native_type30_uses_own_probe_words_and_shared_divisor96_steering() {
        let probes = type9_probe_plan(
            [1_000, 25, 2_000],
            [i32::MAX, 0, 0],
            [0, 0, i32::MAX],
            1,
            NATIVE_TYPE30_SUB_D,
        );
        assert_eq!(probes.left_raw, [601, 2_699]);
        assert_eq!(probes.half_forward_raw, [1_000, 2_349]);
        assert_eq!(probes.right_raw, [1_399, 2_699]);
        assert_eq!(probes.rear_raw, [1_000, 1_651]);

        // The clear-class branch reads the same divisor96 and coupling words
        // as Type58; Type30's distinct classifier/probes remain in its owner.
        let initial = Type9SubDRuntime {
            yaw_rate_raw: 8192,
            last_yaw_step_raw: -3,
        };
        let mut type30 = initial;
        let mut type58 = initial;
        let evidence = known(samples(0, 0, 0, 0), 2048);
        let result30 =
            apply_type9_sub_d(NATIVE_TYPE30_SUB_D, &mut type30, evidence, 20_000, 40_000);
        let result58 =
            apply_type9_sub_d(INTRO2_TYPE58_SUB_D, &mut type58, evidence, 20_000, 40_000);
        assert!(matches!(result30, Type9SubDStep::Applied { .. }));
        assert_eq!(result30, result58);
        assert_eq!(type30, type58);
    }

    #[test]
    fn probes_subtract_after_signed_q31_rounding_and_wrap_world_words() {
        // Independent instruction-level vectors for 41F660: IMUL then
        // SHL/RCL retains product bits 31..62; SUB follows that rounding.
        // Odd forward distance also distinguishes truncating signed /2 from
        // arithmetic shifting the negative distance before the Q31 product.
        let descriptor = SubDSteeringDescriptor {
            forward_probe_raw: 601,
            lateral_probe_raw: 255,
            ..INTRO2_TYPE16_SUB_D
        };
        let cases = [
            (
                [32_700, 25, -32_700],
                [0x5fff_ffff, 0, -0x3000_0001],
                [-0x4000_0001, 0, 0x6fff_ffff],
                1,
                Type9SubDProbePlan {
                    left_raw: [32_208, -32_079],
                    half_forward_raw: [32_549, -32_438],
                    right_raw: [32_590, -32_271],
                    rear_raw: [-32_685, 32_574],
                },
            ),
            (
                [32_700, 25, -32_700],
                [0x5fff_ffff, 0, -0x3000_0001],
                [-0x4000_0001, 0, 0x6fff_ffff],
                -1,
                Type9SubDProbePlan {
                    left_raw: [-32_344, 32_215],
                    half_forward_raw: [-32_686, 32_573],
                    right_raw: [-32_728, 32_405],
                    rear_raw: [32_550, -32_437],
                },
            ),
            (
                [-32_760, 0, 32_760],
                [i32::MIN, 0, 0],
                [0, 0, i32::MIN],
                1,
                Type9SubDProbePlan {
                    left_raw: [-32_505, 32_159],
                    half_forward_raw: [-32_760, 32_460],
                    right_raw: [32_521, 32_159],
                    rear_raw: [-32_760, -32_476],
                },
            ),
            (
                [-32_760, 0, 32_760],
                [i32::MIN, 0, 0],
                [0, 0, i32::MIN],
                -1,
                Type9SubDProbePlan {
                    left_raw: [32_521, -32_175],
                    half_forward_raw: [-32_760, -32_476],
                    right_raw: [-32_505, -32_175],
                    rear_raw: [-32_760, 32_460],
                },
            ),
        ];
        for (position, right, forward, direction, expected) in cases {
            assert_eq!(
                type9_probe_plan(position, right, forward, direction, descriptor),
                expected,
                "position={position:?}, direction={direction}"
            );
        }

        let seam = type9_probe_plan(
            [63, 0, 63],
            [i32::MAX, 0, 0],
            [0, 0, i32::MAX],
            1,
            ORDINARY_TYPE9_SUB_D,
        );
        assert_eq!(seam.left_raw, [0, 190]);
        assert_eq!(seam.rear_raw, [63, 0]);
        assert_eq!(seam.left_raw[0] as u16 >> 8, 0);
        assert_eq!(seam.rear_raw[1] as u16 >> 8, 0);
    }

    #[test]
    fn target_projection_preserves_retail_normalization_and_word_wrap() {
        let right = [i32::MAX, 0, 0];
        let forward = [0, 0, i32::MAX];
        assert_eq!(
            target_lateral_projection_raw([0, 0, 0], [100, 0, 0], right, forward),
            0x7fff
        );
        assert_eq!(
            target_lateral_projection_raw([0, 0, 0], [-100, 0, 0], right, forward),
            -1,
            "retail's exact negative-axis saturation is Q31 -1"
        );
        assert_eq!(
            target_lateral_projection_raw([32_760, 0, 0], [-32_760, 0, 0], right, forward,),
            0x7fff,
            "target subtraction wraps before normalization"
        );
        assert!(target_lateral_projection_raw([0, 0, 0], [-100, 0, 100], right, forward) < -23_000);
    }

    #[test]
    fn target_projection_saturates_the_retail_turn_for_targets_behind() {
        let right = [i32::MAX, 0, 0];
        let forward = [0, 0, i32::MAX];
        assert_eq!(
            target_lateral_projection_raw([0, 0, 0], [100, 0, -100], right, forward),
            0x7fff
        );
        assert_eq!(
            target_lateral_projection_raw([0, 0, 0], [-100, 0, -100], right, forward),
            -0x7fff
        );
        assert_eq!(
            target_lateral_projection_raw([0, 0, 0], [0, 0, -100], right, forward),
            -0x7fff,
            "retail gives a centered behind target its negative-turn bias"
        );
    }

    #[test]
    fn target_body_projection_preserves_vertical_when_behind_override_turns_lateral() {
        let basis = super::super::type9_attitude::Type9BodyBasis {
            lateral: [i32::MAX, 0, 0],
            up: [0, i32::MAX, 0],
            forward: [0, 0, i32::MAX],
        };
        let ahead = target_body_projection_raw([0; 3], [100, 100, 100], basis);
        let behind = target_body_projection_raw([0; 3], [100, 100, -100], basis);
        assert!((18_000..20_000).contains(&ahead.vertical_raw));
        assert_eq!(behind.vertical_raw, ahead.vertical_raw);
        assert_eq!(behind.lateral_raw, 0x7fff);
        assert!(ahead.lateral_raw < behind.lateral_raw);

        let inverted_up = super::super::type9_attitude::Type9BodyBasis {
            up: [0, -i32::MAX, 0],
            ..basis
        };
        let inverted = target_body_projection_raw([0; 3], [100, 100, 100], inverted_up);
        assert_eq!(inverted.lateral_raw, ahead.lateral_raw);
        assert!((-20_000..-18_000).contains(&inverted.vertical_raw));
        assert_eq!(
            target_body_projection_raw([0; 3], [0, -100, 0], basis).vertical_raw,
            -1,
            "the second output retains retail's exact negative-axis saturation"
        );
    }

    #[test]
    fn classifier_free_frame_advances_stagger_without_querying_the_cache() {
        let right = [0, 0, -0x7ffe_0000];
        let forward = [0x7ffe_0000, 0, 0];
        assert!(classifier_free_sub_d_frame_evidence(
            ORDINARY_TYPE9_SUB_D,
            [0; 3],
            [0, 0, 1_000],
            right,
            forward,
        )
        .is_none());

        let mut owner =
            Type9SubDFrameOwner::from_retail_state([1, 2, 3, 4, 5, 6, 7, 8], [9, 10], 7);
        let evidence = owner
            .evidence_for_classifier_free_frame(FLYER_SUB_D, [0; 3], [0, 0, 1_000], right, forward)
            .expect("zero flags select the classifier-free route");
        assert_eq!(
            evidence.target_lateral_raw,
            RetailRuntimeValue::Known(target_lateral_projection_raw(
                [0; 3],
                [0, 0, 1_000],
                right,
                forward,
            ))
        );
        assert_eq!(
            evidence.classifier_evidence,
            RetailRuntimeValue::Known(Type9SubDClassifierEvidence::Bypassed)
        );
        assert_eq!(owner.classifier_cache().stagger_counter(), 8);
        assert_eq!(
            owner.classifier_cache().origin(),
            RetailRuntimeValue::Known([9, 10])
        );
        assert_eq!(owner.classifier_cache().rows(), [1, 0, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn exact_type9_classifier_preserves_branch_priority_and_authored_bits() {
        let point = [0x0100, 0x0100];
        let mut grid = terrain(0, -100);
        let mut cache = empty_cache(0, [1, 1]);
        assert_eq!(classify(&mut cache, &grid, point), 0);

        cell_mut(&mut grid, 1, 1).attribute = 1;
        let mut object_cache = empty_cache(0, [1, 1]);
        assert_eq!(classify(&mut object_cache, &grid, point), 2);

        cell_mut(&mut grid, 1, 1).attribute = 0;
        cell_mut(&mut grid, 1, 1).terrain_type = 0x10;
        let mut material_cache = empty_cache(0, [1, 1]);
        assert_eq!(classify(&mut material_cache, &grid, point), 1);

        let water = terrain(0, 1);
        let mut water_cache = empty_cache(0, [1, 1]);
        assert_eq!(classify(&mut water_cache, &water, point), 5);

        let mut steep = terrain(10, -100);
        cell_mut(&mut steep, 0, 0).height = (-10i8) as u8;
        let mut steep_cache = empty_cache(0, [1, 1]);
        assert_eq!(
            classify(&mut steep_cache, &steep, point),
            4,
            "steepness precedes water, objects, and material classification"
        );
    }

    #[test]
    fn cache_reuses_stale_class_until_its_staggered_row_is_cleared() {
        let point = [0x0a00, 0x1400];
        let mut grid = terrain(0, -100);
        let mut cache = empty_cache(0xff, [10, 20]);
        assert_eq!(classify(&mut cache, &grid, point), 0);
        assert_eq!(cache.rows()[0] & 0x0f, 1);

        cell_mut(&mut grid, 10, 20).attribute = 1;
        assert_eq!(
            classify(&mut cache, &grid, point),
            0,
            "a nonzero nibble is returned without reclassification"
        );
        cache.begin_frame();
        assert_eq!(cache.stagger_counter(), 0);
        assert_eq!(cache.rows()[0], 0);
        assert_eq!(classify(&mut cache, &grid, point), 2);
    }

    #[test]
    fn cache_window_matches_retail_x_shifts_and_asymmetric_z_paths() {
        let grid = terrain(0, -100);
        let mut cache = empty_cache(0, [10, 10]);
        assert_eq!(classify(&mut cache, &grid, [0x0a00, 0x0a00]), 0);
        assert_eq!(cache.origin(), RetailRuntimeValue::Known([10, 10]));
        assert_eq!(cache.rows()[0], 1);

        assert_eq!(classify(&mut cache, &grid, [0x1200, 0x0a00]), 0);
        assert_eq!(cache.origin(), RetailRuntimeValue::Known([11, 10]));
        assert_eq!(cache.rows()[0], 0x1000_0010);

        assert_eq!(classify(&mut cache, &grid, [0x0a00, 0x0a00]), 0);
        assert_eq!(cache.origin(), RetailRuntimeValue::Known([10, 10]));
        assert_eq!(cache.rows()[0], 0x0100_0001);

        assert_eq!(classify(&mut cache, &grid, [0x0a00, 0x1200]), 0);
        assert_eq!(cache.origin(), RetailRuntimeValue::Known([10, 11]));
        assert_eq!(
            cache.rows()[0],
            0x0100_0001,
            "positive Z moves only the origin; retail does not copy rows"
        );
        assert_ne!(cache.rows()[7], 0);

        assert_eq!(classify(&mut cache, &grid, [0x0a00, 0x0a00]), 0);
        assert_eq!(cache.origin(), RetailRuntimeValue::Known([10, 10]));
        assert_eq!(cache.rows()[0] & 0x0f, 1);
        assert_eq!(cache.rows()[1], 0x0100_0001);
    }

    #[test]
    fn unresolved_constructor_origin_never_anchors_or_mutates_the_cache() {
        let grid = terrain(0, -100);
        let mut owner = Type9SubDFrameOwner::pending_constructor_origin(7);
        let before = owner;

        let evidence = owner.evidence_for_frame(
            &grid,
            [0x0100, 0, 0x0100],
            [0x0200, 0, 0x0100],
            [i32::MAX, 0, 0],
            [0, 0, i32::MAX],
            1,
        );

        assert_eq!(evidence.classifier_evidence, RetailRuntimeValue::Unresolved);
        assert_eq!(owner, before);
        assert_eq!(
            owner.classifier_cache().origin(),
            RetailRuntimeValue::Unresolved
        );
    }

    #[test]
    fn accepted_fresh_level1_route_resets_the_first_query_to_its_cell() {
        let grid = terrain(0, -100);
        let mut cache = Type9SubDClassifierCache::pending_fresh_level1_first_query_reset(
            0x2a,
            FreshLevel1Type9ClassifierAdmission::ACCEPTED_FIRST_CONSUMER_TRACE,
        );

        assert_eq!(cache.origin(), RetailRuntimeValue::Unresolved);
        assert!(
            cache.can_classify(),
            "the accepted route is pending a proven first-query reset, not an arbitrary origin"
        );
        assert_eq!(classify(&mut cache, &grid, [-0x7000, 0x7e00]), 0);
        assert_eq!(cache.origin(), RetailRuntimeValue::Known([0x90, 0x7e]));
        assert_eq!(
            cache.rows(),
            [1, 0, 0, 0, 0, 0, 0, 0],
            "the first query must reproduce retail's full reset and row-zero fill"
        );

        assert_eq!(classify(&mut cache, &grid, [-0x6f00, 0x7e00]), 0);
        assert_eq!(
            cache.rows()[0],
            0x11,
            "after the admitted reset, ordinary cache-window behavior resumes"
        );
    }

    #[test]
    fn intro2_type16_own_first_consumers_reset_once_and_keep_later_cache_fills() {
        for (spawn, seed, point) in [
            (5, 0x05, [0x9500u16 as i16, 0x8ba8u16 as i16]),
            (42, 0x1b, [0xbd57u16 as i16, 0x7a00]),
        ] {
            let mut cache = *intro2_type16_first_query_owner_for_birth(spawn, seed)
                .unwrap()
                .classifier_cache();
            assert_eq!(cache.origin(), RetailRuntimeValue::Unresolved);
            assert!(cache.can_classify());
            assert_eq!(
                cache.transact(|tx| tx.query(point, |_| Ok::<_, ()>(0))),
                Ok(0)
            );
            assert_eq!(cache.stagger_counter(), seed + 1);
            assert_eq!(
                cache.origin(),
                RetailRuntimeValue::Known(point.map(|raw| (raw as u16 >> 8) as u8))
            );
            assert_eq!(cache.rows(), [1, 0, 0, 0, 0, 0, 0, 0]);
            let first_origin = cache.origin();
            let adjacent = [point[0].wrapping_add(0x100), point[1]];
            assert_eq!(
                cache.transact(|tx| tx.query(adjacent, |_| Ok::<_, ()>(2))),
                Ok(2)
            );
            assert_eq!(cache.stagger_counter(), seed + 2);
            assert_eq!(cache.origin(), first_origin);
            assert_eq!(cache.rows()[0], 0x31, "the receipt is consumed only once");
        }
        for (spawn, seed) in [(5, 0x1b), (42, 0x05), (4, 0x04), (41, 0x1a)] {
            assert!(intro2_type16_first_query_owner_for_birth(spawn, seed).is_none());
        }
        assert!(!Type9SubDFrameOwner::pending_constructor_origin(0x05)
            .classifier_cache()
            .can_classify());
    }

    #[test]
    fn intro2_type53_receipts_cover_only_the_four_own_first_consumers() {
        for (spawn, seed, point) in [
            (20, 0x13, [0x9300u16 as i16, 0x8d00u16 as i16]),
            (26, 0x16, [0xc200u16 as i16, 0x1501]),
            (38, 0x18, [0x9100u16 as i16, 0x8d00u16 as i16]),
            (41, 0x1a, [0x9500u16 as i16, 0x6901]),
        ] {
            let mut cache = *intro2_type53_first_query_owner_for_birth(spawn, seed)
                .unwrap()
                .classifier_cache();
            assert_eq!(cache.origin(), RetailRuntimeValue::Unresolved);
            assert_eq!(
                cache.transact(|tx| tx.query(point, |_| Ok::<_, ()>(0))),
                Ok(0)
            );
            assert_eq!(cache.stagger_counter(), seed + 1);
            assert_eq!(
                cache.origin(),
                RetailRuntimeValue::Known(point.map(|raw| (raw as u16 >> 8) as u8))
            );
            assert_eq!(cache.rows(), [1, 0, 0, 0, 0, 0, 0, 0]);
            let first_origin = cache.origin();
            let adjacent = [point[0].wrapping_add(0x100), point[1]];
            assert_eq!(
                cache.transact(|tx| tx.query(adjacent, |_| Ok::<_, ()>(2))),
                Ok(2)
            );
            assert_eq!(cache.stagger_counter(), seed + 2);
            assert_eq!(cache.origin(), first_origin);
            assert_eq!(cache.rows()[0], 0x31, "each birth receipt resets only once");
        }
        for (spawn, seed) in [(26, 0x1a), (41, 0x16), (20, 0x18), (38, 0x13), (2, 0x02)] {
            assert!(intro2_type53_first_query_owner_for_birth(spawn, seed).is_none());
        }
    }

    #[test]
    fn intro2_type58_own_first_consumer_resets_once_and_preserves_later_cache() {
        let mut cache = *intro2_type58_first_query_owner_for_birth(40, 0x19)
            .unwrap()
            .classifier_cache();
        let point = [0xbdffu16 as i16, 0x0dc8];
        assert_eq!(cache.origin(), RetailRuntimeValue::Unresolved);
        assert!(cache.can_classify());
        assert_eq!(
            cache.transact(|tx| tx.query(point, |_| Ok::<_, ()>(0))),
            Ok(0)
        );
        assert_eq!(cache.stagger_counter(), 0x1a);
        assert_eq!(cache.origin(), RetailRuntimeValue::Known([0xbd, 0x0d]));
        assert_eq!(cache.rows(), [1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            cache.transact(
                |tx| tx.query([point[0].wrapping_add(0x100), point[1]], |_| Ok::<_, ()>(2))
            ),
            Ok(2)
        );
        assert_eq!(cache.stagger_counter(), 0x1b);
        assert_eq!(cache.origin(), RetailRuntimeValue::Known([0xbd, 0x0d]));
        assert_eq!(cache.rows()[0], 0x31);
        for (spawn, seed) in [(40, 0x18), (40, 0x1a), (38, 0x19), (41, 0x19)] {
            assert!(intro2_type58_first_query_owner_for_birth(spawn, seed).is_none());
        }
        assert!(!Type9SubDFrameOwner::pending_constructor_origin(0x19)
            .classifier_cache()
            .can_classify());
    }

    #[test]
    fn intro2_type94_own_first_consumer_resets_once_and_preserves_later_cache() {
        let mut cache = *intro2_type94_first_query_owner_for_birth(43, 0x1c)
            .unwrap()
            .classifier_cache();
        let point = [0xc1ffu16 as i16, 0x82c8u16 as i16];
        assert_eq!(cache.origin(), RetailRuntimeValue::Unresolved);
        assert!(cache.can_classify());
        assert_eq!(
            cache.transact(|tx| tx.query(point, |_| Ok::<_, ()>(0))),
            Ok(0)
        );
        assert_eq!(cache.stagger_counter(), 0x1d);
        assert_eq!(cache.origin(), RetailRuntimeValue::Known([0xc1, 0x82]));
        assert_eq!(cache.rows(), [1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            cache.transact(
                |tx| tx.query([point[0].wrapping_add(0x100), point[1]], |_| Ok::<_, ()>(2))
            ),
            Ok(2)
        );
        assert_eq!(cache.stagger_counter(), 0x1e);
        assert_eq!(cache.origin(), RetailRuntimeValue::Known([0xc1, 0x82]));
        assert_eq!(cache.rows()[0], 0x31, "own receipt is consumed only once");
        for (spawn, seed) in [(43, 0x1b), (43, 0x1d), (40, 0x19), (40, 0x1c), (42, 0x1c)] {
            assert!(intro2_type94_first_query_owner_for_birth(spawn, seed).is_none());
        }
        assert!(!Type9SubDFrameOwner::pending_constructor_origin(0x1c)
            .classifier_cache()
            .can_classify());
    }

    #[test]
    fn native_type94_sub_d_reproduces_own_first_probe_and_96_steering() {
        // Actual zero-angle Q31 columns used by 01430: lateral -Z, forward+X.
        // 1F660 rounds each scaled product before subtracting the left offset.
        let plan = type9_probe_plan(
            [0xc000u16 as i16, 0, 0x8200u16 as i16],
            [0, 0, -i32::MAX],
            [i32::MAX, 0, 0],
            1,
            INTRO2_TYPE94_SUB_D,
        );
        assert_eq!(plan.left_raw, [0xc1ffu16 as i16, 0x82c8u16 as i16]);
        assert_eq!(plan.half_forward_raw, [0xc0ffu16 as i16, 0x8200u16 as i16]);
        assert_eq!(plan.right_raw, [0xc1ffu16 as i16, 0x8138u16 as i16]);
        assert_eq!(plan.rear_raw, [0xbf01u16 as i16, 0x8200u16 as i16]);
        let mut runtime = Type9SubDRuntime::from_constructor();
        assert_eq!(
            apply_type9_sub_d(
                INTRO2_TYPE94_SUB_D,
                &mut runtime,
                known(samples(0, 0, 2, 0), 0),
                20_000,
                20_000
            ),
            Type9SubDStep::Applied { yaw_step_raw: -105 }
        );
        assert_eq!(runtime.yaw_rate_raw, -682);
        assert_eq!(runtime.last_yaw_step_raw, -105);
    }

    #[test]
    fn native_type58_sub_d_uses_512_200_probes_and_96_steering() {
        let origin = [1000, 25, 2000];
        assert_eq!(
            type9_probe_plan(
                origin,
                [i32::MAX, 0, 0],
                [0, 0, i32::MAX],
                1,
                INTRO2_TYPE58_SUB_D
            ),
            Type9SubDProbePlan {
                left_raw: [801, 2511],
                half_forward_raw: [1000, 2255],
                right_raw: [1199, 2511],
                rear_raw: [1000, 1745],
            }
        );
        assert_eq!(
            type9_probe_plan(
                origin,
                [i32::MAX, 0, 0],
                [0, 0, i32::MAX],
                -1,
                INTRO2_TYPE58_SUB_D
            ),
            Type9SubDProbePlan {
                left_raw: [1200, 1488],
                half_forward_raw: [1000, 1744],
                right_raw: [800, 1488],
                rear_raw: [1000, 2256],
            }
        );
        let mut runtime = Type9SubDRuntime::from_constructor();
        assert_eq!(
            apply_type9_sub_d(
                INTRO2_TYPE58_SUB_D,
                &mut runtime,
                known(samples(0, 0, 2, 0), 0),
                20_000,
                20_000
            ),
            Type9SubDStep::Applied { yaw_step_raw: -105 }
        );
        assert_eq!(runtime.yaw_rate_raw, -682);
        assert_eq!(runtime.last_yaw_step_raw, -105);
        let before = runtime;
        assert_eq!(
            apply_type9_sub_d(
                SubDSteeringDescriptor {
                    steering_divisor_raw: 95,
                    ..INTRO2_TYPE58_SUB_D
                },
                &mut runtime,
                known(samples(0, 0, 2, 0), 0),
                20_000,
                20_000
            ),
            Type9SubDStep::UnsupportedDescriptor
        );
        assert_eq!(runtime, before);
    }

    #[test]
    fn intro2_type9_own_birth_witnesses_reset_once_and_keep_frame_cadence() {
        let witnesses = [
            (2, 0x02, [0x8c7fu16, 0x7e40]),
            (3, 0x03, [0x8e7f, 0x7e40]),
            (9, 0x09, [0x8e7f, 0x8040]),
            (11, 0x0b, [0x8d7f, 0x7c40]),
            (16, 0x0f, [0xb47f, 0x1140]),
            (17, 0x10, [0xae7f, 0x2540]),
            (18, 0x11, [0xb57f, 0x1340]),
            (19, 0x12, [0xb97f, 0x1640]),
            (49, 0x20, [0xbc7f, 0x1640]),
            (50, 0x21, [0xbf7f, 0x1540]),
            (58, 0x24, [0xbf7f, 0x7d40]),
            (59, 0x25, [0x9f7f, 0x8040]),
            (60, 0x26, [0xa27f, 0x7f40]),
        ];
        for (spawn, seed, point) in witnesses {
            let owner = intro2_type9_first_query_owner_for_birth(spawn, seed).unwrap();
            let mut cache = *owner.classifier_cache();
            assert_eq!(cache.origin(), RetailRuntimeValue::Unresolved);
            assert!(cache.can_classify());
            assert_eq!(cache.stagger_counter(), seed);
            let point = point.map(|raw| raw as i16);
            let first = cache.transact(|tx| tx.query(point, |_| Ok::<_, ()>(0)));
            assert_eq!(first, Ok(0));
            assert_eq!(cache.stagger_counter(), seed + 1);
            assert_eq!(cache.rows(), [1, 0, 0, 0, 0, 0, 0, 0]);
            let origin = point.map(|raw| (raw as u16 >> 8) as u8);
            assert_eq!(cache.origin(), RetailRuntimeValue::Known(origin));
            let adjacent = [point[0].wrapping_add(0x100), point[1]];
            assert_eq!(
                cache.transact(|tx| tx.query(adjacent, |_| Ok::<_, ()>(2))),
                Ok(2)
            );
            assert_eq!(cache.stagger_counter(), seed + 2);
            assert_eq!(cache.origin(), RetailRuntimeValue::Known(origin));
            assert_eq!(cache.rows()[0], 0x31, "the receipt cannot reset twice");
            assert!(intro2_type9_first_query_owner_for_birth(spawn, seed + 1).is_none());
        }
        for (spawn, seed) in [(20, 0x13), (26, 0x16), (38, 0x18), (41, 0x1a), (2, 0x27)] {
            assert!(intro2_type9_first_query_owner_for_birth(spawn, seed).is_none());
        }
        assert!(!Type9SubDFrameOwner::pending_constructor_origin(2)
            .classifier_cache()
            .can_classify());
    }

    #[test]
    fn intro2_type26_ttd_route_resets_the_first_query_to_its_cell() {
        let grid = terrain(0, -100);
        let mut cache = Type9SubDClassifierCache::pending_intro2_type26_first_query_reset(
            0x0a,
            Intro2Type26ClassifierAdmission::ACCEPTED_TTD_V200001,
        );
        assert_eq!(cache.origin(), RetailRuntimeValue::Unresolved);
        assert!(cache.can_classify());
        assert_eq!(
            classify(
                &mut cache,
                &grid,
                [(0xbcu16 << 8) as i16, (0x0eu16 << 8) as i16]
            ),
            0
        );
        assert_eq!(cache.origin(), RetailRuntimeValue::Known([0xbc, 0x0e]));
        assert_eq!(cache.rows(), [1, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn intro2_type26_seed_15_ttd_route_resets_the_first_query_to_its_cell() {
        let grid = terrain(0, -100);
        let mut owner = intro2_type26_first_query_owner_for_seed(0x15).expect("spawn-25 seed");
        assert_eq!(
            owner.apply_first_query(&grid, [(0xc1u16 << 8) as i16, (0x0fu16 << 8) as i16]),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            owner.classifier_cache().origin(),
            RetailRuntimeValue::Known([0xc1, 0x0f])
        );
        assert_eq!(owner.classifier_cache().rows(), [1, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn type13_and_level1_type47_seeds_cannot_mint_intro2_first_query_owners() {
        assert!(intro2_type26_first_query_owner_for_seed(0).is_none());
        assert!(intro2_type47_first_query_owner_for_seed(0x2B).is_none());
        assert!(intro2_type47_first_query_owner_for_seed(0x2C).is_none());
        assert!(intro2_type47_first_query_owner_for_seed(0x2D).is_none());
        assert!(level1_type47_first_query_owner_for_seed(0x06).is_none());
        assert!(level1_type47_first_query_owner_for_seed(0x3C).is_none());
        assert_eq!(LEVEL1_TYPE47_FIRST_QUERY_SEEDS, [0x2B, 0x2C, 0x2D]);
        assert!(level1_type47_first_query_owner_for_seed(0x2B).is_some());
        assert!(level1_type47_first_query_owner_for_seed(0x2C).is_some());
        assert!(level1_type47_first_query_owner_for_seed(0x2D).is_some());
        assert!(intro2_type26_first_query_owner_for_seed(0x2B).is_none());
        assert_eq!(INTRO2_TYPE26_FIRST_QUERY_SEEDS, [0x0A, 0x15]);
        assert_eq!(INTRO2_TYPE47_FIRST_QUERY_SEEDS, [0x06, 0x07, 0x08]);
        assert!(type17_first_query_owner_for_seed(0x2B).is_none());
        assert!(type8_first_query_owner_for_seed(0x2B).is_none());
        assert!(type17_first_query_owner_for_seed(0x32).is_some());
        assert!(type8_first_query_owner_for_seed(0x38).is_some());
        assert_eq!(type17_seed_for_fresh_level1_spawn(18), Some(0x32));
        assert_eq!(type17_seed_for_fresh_level1_spawn(11), None);
    }

    #[test]
    fn type17_and_type8_ttd_routes_reset_the_first_query_to_their_cell() {
        let grid = terrain(0, -100);
        for mut cache in [
            Type9SubDClassifierCache::pending_type17_first_query_reset(
                0x32,
                Type17ClassifierAdmission::ACCEPTED_TTD_V200002,
            ),
            Type9SubDClassifierCache::pending_type8_first_query_reset(
                0x38,
                Type8ClassifierAdmission::ACCEPTED_TTD_V200002,
            ),
        ] {
            assert_eq!(cache.origin(), RetailRuntimeValue::Unresolved);
            assert!(cache.can_classify());
            assert_eq!(
                classify(
                    &mut cache,
                    &grid,
                    [(0x50u16 << 8) as i16, (0x3au16 << 8) as i16]
                ),
                0
            );
            assert_eq!(cache.origin(), RetailRuntimeValue::Known([0x50, 0x3a]));
            assert_eq!(cache.rows(), [1, 0, 0, 0, 0, 0, 0, 0]);
        }
    }

    #[test]
    fn intro2_type47_ttd_route_resets_the_first_query_to_its_cell() {
        let grid = terrain(0, -100);
        let mut cache = Type9SubDClassifierCache::pending_intro2_type47_first_query_reset(
            0x07,
            Intro2Type47ClassifierAdmission::ACCEPTED_TTD_V200001,
        );
        assert_eq!(cache.origin(), RetailRuntimeValue::Unresolved);
        assert!(cache.can_classify());
        assert_eq!(
            classify(
                &mut cache,
                &grid,
                [(0xbcu16 << 8) as i16, (0x7cu16 << 8) as i16]
            ),
            0
        );
        assert_eq!(cache.origin(), RetailRuntimeValue::Known([0xbc, 0x7c]));
        assert_eq!(cache.rows(), [1, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn level1_type47_ttd_route_resets_the_first_query_to_its_cell() {
        let grid = terrain(0, -100);
        let mut cache = Type9SubDClassifierCache::pending_level1_type47_first_query_reset(
            0x2D,
            Level1Type47ClassifierAdmission::ACCEPTED_TTD_V200003,
        );
        assert_eq!(cache.origin(), RetailRuntimeValue::Unresolved);
        assert!(cache.can_classify());
        assert_eq!(
            classify(
                &mut cache,
                &grid,
                [(0xbfu16 << 8) as i16, (0x7eu16 << 8) as i16]
            ),
            0
        );
        assert_eq!(cache.origin(), RetailRuntimeValue::Known([0xbf, 0x7e]));
        assert_eq!(cache.rows(), [1, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn allocator_origin_can_change_window_state_even_with_cleared_rows() {
        let grid = terrain(0, -100);
        let point = [(0x90u16 << 8) as i16, (0x7eu16 << 8) as i16];
        let mut near_band = Type9SubDClassifierCache::from_retail_state([0; 8], [0x8a, 0x7e], 0x29);
        let mut far_band = Type9SubDClassifierCache::from_retail_state([0; 8], [0xec, 0xff], 0x29);

        assert_eq!(
            near_band.classify(&grid, point),
            far_band.classify(&grid, point)
        );
        assert_eq!(near_band.origin(), RetailRuntimeValue::Known([0x8a, 0x7e]));
        assert_eq!(far_band.origin(), RetailRuntimeValue::Known([0x90, 0x7e]));
        assert_ne!(
            near_band.rows(),
            far_band.rows(),
            "a near-band residue places the nibble at another cache edge"
        );

        let unresolved = Type9SubDClassifierCache::pending_constructor_origin(0x29);
        assert_eq!(unresolved.origin(), RetailRuntimeValue::Unresolved);
    }

    #[test]
    fn frame_owner_skips_rear_cache_fill_when_the_central_class_is_clear() {
        let grid = terrain(0, -100);
        let mut owner = Type9SubDFrameOwner::from_retail_state([0; 8], [0, 1], 0);
        let evidence = owner.evidence_for_frame(
            &grid,
            [0x0100, 0, 0x0100],
            [0x0200, 0, 0x0100],
            [i32::MAX, 0, 0],
            [0, 0, i32::MAX],
            1,
        );
        let RetailRuntimeValue::Known(Type9SubDClassifierEvidence::Samples(samples)) =
            evidence.classifier_evidence
        else {
            panic!("concrete terrain must produce classifier samples");
        };
        assert_eq!(
            (samples.left, samples.half_forward, samples.right),
            (0, 0, 0)
        );
        assert_eq!(samples.rear, 0);
        assert_eq!(
            owner.classifier_cache().origin(),
            RetailRuntimeValue::Known([0, 1])
        );
        assert_eq!(owner.classifier_cache().rows()[0], 0x11);
        assert!(
            owner.classifier_cache().rows()[1..]
                .iter()
                .all(|row| *row == 0),
            "rear cell [1, 0] must not be queried on a clear central class"
        );
    }

    #[test]
    fn unresolved_resource_or_classifier_ownership_is_a_strict_no_op() {
        let initial = Type9SubDRuntime {
            yaw_rate_raw: 77,
            last_yaw_step_raw: -3,
        };
        let mut runtime = initial;
        let result = apply_type9_sub_d(
            ORDINARY_TYPE9_SUB_D,
            &mut runtime,
            Type9SubDFrameEvidence {
                target_lateral_raw: RetailRuntimeValue::Unresolved,
                classifier_evidence: RetailRuntimeValue::Unresolved,
            },
            20_000,
            20_000,
        );
        assert_eq!(result, Type9SubDStep::UnresolvedTargetProjection);
        assert_eq!(runtime, initial);

        let result = apply_type9_sub_d(
            ORDINARY_TYPE9_SUB_D,
            &mut runtime,
            Type9SubDFrameEvidence {
                target_lateral_raw: RetailRuntimeValue::Known(0),
                classifier_evidence: RetailRuntimeValue::Unresolved,
            },
            20_000,
            20_000,
        );
        assert_eq!(result, Type9SubDStep::UnresolvedClassifierCache);
        assert_eq!(runtime, initial);
    }

    #[test]
    fn classifier_gate_evidence_must_match_the_descriptor() {
        let initial = Type9SubDRuntime {
            yaw_rate_raw: 77,
            last_yaw_step_raw: -3,
        };
        let mut runtime = initial;
        assert_eq!(
            apply_type9_sub_d(
                ORDINARY_TYPE9_SUB_D,
                &mut runtime,
                Type9SubDFrameEvidence {
                    target_lateral_raw: RetailRuntimeValue::Known(100),
                    classifier_evidence: RetailRuntimeValue::Known(
                        Type9SubDClassifierEvidence::Bypassed,
                    ),
                },
                20_000,
                20_000,
            ),
            Type9SubDStep::ClassifierEvidenceMismatch
        );
        assert_eq!(runtime, initial);

        assert_eq!(
            apply_type9_sub_d(
                FLYER_SUB_D,
                &mut runtime,
                known(samples(0, 0, 0, 0), 100),
                20_000,
                20_000,
            ),
            Type9SubDStep::ClassifierEvidenceMismatch
        );
        assert_eq!(runtime, initial);
    }

    #[test]
    fn native_type10_query_free_steering_uses_signed_divisor48_and_global_step() {
        // 20260: (32767<<10)/48=699029; SAR9=1365 (negative SAR=-1366).
        // 20360 independently uses global_dt/4=5000, then SAR15.
        for (projection, expected_rate, expected_step) in
            [(32767, 1365, 208), (-32767, -1366, -209)]
        {
            let mut runtime = Type9SubDRuntime::from_constructor();
            let evidence = Type9SubDFrameEvidence {
                target_lateral_raw: RetailRuntimeValue::Known(projection),
                classifier_evidence: RetailRuntimeValue::Known(
                    Type9SubDClassifierEvidence::Bypassed,
                ),
            };
            assert_eq!(
                apply_type9_sub_d(INTRO2_TYPE10_SUB_D, &mut runtime, evidence, 125_000, 20_000),
                Type9SubDStep::Applied {
                    yaw_step_raw: expected_step
                }
            );
            assert_eq!(runtime.yaw_rate_raw, expected_rate);
            assert_eq!(runtime.last_yaw_step_raw, expected_step);
            let before = runtime;
            assert_eq!(
                apply_type9_sub_d(
                    INTRO2_TYPE10_SUB_D,
                    &mut runtime,
                    known(samples(0, 0, 0, 0), projection),
                    125_000,
                    20_000
                ),
                Type9SubDStep::ClassifierEvidenceMismatch
            );
            assert_eq!(runtime, before);
        }
    }

    #[test]
    fn native_fish_query_free_steering_admits_uncoupled_divisor64() {
        // 20260: (32767<<10)/64=524272; SAR9=1023 (negative SAR=-1024).
        // 20360 independently integrates that rate with global_dt/4.
        for (projection, rate, step) in [(32767, 1023, 156), (-32767, -1024, -157)] {
            let mut runtime = Type9SubDRuntime::from_constructor();
            assert_eq!(
                apply_type9_sub_d(
                    SHARED_FISH_SUB_D,
                    &mut runtime,
                    Type9SubDFrameEvidence {
                        target_lateral_raw: RetailRuntimeValue::Known(projection),
                        classifier_evidence: RetailRuntimeValue::Known(
                            Type9SubDClassifierEvidence::Bypassed,
                        ),
                    },
                    125_000,
                    20_000,
                ),
                Type9SubDStep::Applied { yaw_step_raw: step }
            );
            assert_eq!(runtime.yaw_rate_raw, rate);
            assert_eq!(runtime.last_yaw_step_raw, step);
        }
    }

    #[test]
    fn type7_query_free_steering_uses_signed_divisor32_and_global_step() {
        // 20260: (32767<<10)/32=1048544; SAR9=2047 (negative SAR=-2048).
        // 20360 independently uses global_dt/4=5000, then SAR15.
        for (projection, rate, step) in [(32767, 2047, 312), (-32767, -2048, -313)] {
            let mut runtime = Type9SubDRuntime::from_constructor();
            assert_eq!(
                apply_type9_sub_d(
                    ORDINARY_TYPE7_SUB_D,
                    &mut runtime,
                    Type9SubDFrameEvidence {
                        target_lateral_raw: RetailRuntimeValue::Known(projection),
                        classifier_evidence: RetailRuntimeValue::Known(
                            Type9SubDClassifierEvidence::Bypassed,
                        ),
                    },
                    125_000,
                    20_000,
                ),
                Type9SubDStep::Applied { yaw_step_raw: step }
            );
            assert_eq!(runtime.yaw_rate_raw, rate);
            assert_eq!(runtime.last_yaw_step_raw, step);
            let before = runtime;
            assert_eq!(
                apply_type9_sub_d(
                    ORDINARY_TYPE7_SUB_D,
                    &mut runtime,
                    known(samples(0, 0, 0, 0), projection),
                    125_000,
                    20_000,
                ),
                Type9SubDStep::ClassifierEvidenceMismatch
            );
            assert_eq!(
                runtime, before,
                "terrain evidence cannot mutate a flags0 actor"
            );
        }
    }

    #[test]
    fn clear_center_uses_left_right_class_ordering() {
        let mut runtime = Type9SubDRuntime::default();
        let result = apply_type9_sub_d(
            ORDINARY_TYPE9_SUB_D,
            &mut runtime,
            known(samples(0, 0, 2, 0), 0),
            20_000,
            20_000,
        );
        assert_eq!(runtime.yaw_rate_raw, -0x1_0000 / 20);
        assert_eq!(result, Type9SubDStep::Applied { yaw_step_raw: -500 });

        apply_type9_sub_d(
            ORDINARY_TYPE9_SUB_D,
            &mut runtime,
            known(samples(2, 0, 0, 0), 0),
            20_000,
            20_000,
        );
        assert_eq!(runtime.yaw_rate_raw, 0x1_0000 / 20);
    }

    #[test]
    fn blocking_class_uses_rear_gate_and_non_height_fallback() {
        let mut runtime = Type9SubDRuntime {
            yaw_rate_raw: 1_000,
            last_yaw_step_raw: 0,
        };
        apply_type9_sub_d(
            ORDINARY_TYPE9_SUB_D,
            &mut runtime,
            known(samples(0, 2, 0, 3), 0),
            20_000,
            20_000,
        );
        assert_ne!(runtime.yaw_rate_raw, 0x1_8000 / 20);

        apply_type9_sub_d(
            ORDINARY_TYPE9_SUB_D,
            &mut runtime,
            known(samples(0, 2, 0, 1), 0),
            20_000,
            20_000,
        );
        assert_eq!(runtime.yaw_rate_raw, 0x1_8000 / 20);
    }

    #[test]
    fn height_tie_breaker_fails_closed_then_matches_class_four_and_five() {
        let initial = Type9SubDRuntime {
            yaw_rate_raw: 123,
            last_yaw_step_raw: 4,
        };
        let mut runtime = initial;
        assert_eq!(
            apply_type9_sub_d(
                ORDINARY_TYPE9_SUB_D,
                &mut runtime,
                known(samples(0, 4, 0, 0), 0),
                20_000,
                20_000,
            ),
            Type9SubDStep::UnresolvedTerrainHeights
        );
        assert_eq!(runtime, initial);

        let with_heights = |center, left, current, right| Type9SubDClassifierSamples {
            terrain_heights: RetailRuntimeValue::Known(Type9SubDTerrainHeights {
                left_raw: left,
                current_raw: current,
                right_raw: right,
            }),
            ..samples(0, center, 0, 0)
        };
        apply_type9_sub_d(
            ORDINARY_TYPE9_SUB_D,
            &mut runtime,
            known(with_heights(5, 10, 0, 20), 0),
            20_000,
            20_000,
        );
        assert_eq!(runtime.yaw_rate_raw, 0x2_0000 / 20);

        apply_type9_sub_d(
            ORDINARY_TYPE9_SUB_D,
            &mut runtime,
            known(with_heights(4, 20, 30, 10), 0),
            20_000,
            20_000,
        );
        assert_eq!(runtime.yaw_rate_raw, -0x2_0000 / 20);
    }

    #[test]
    fn unsupported_descriptor_never_mutates_type9_runtime() {
        let initial = Type9SubDRuntime {
            yaw_rate_raw: 5,
            last_yaw_step_raw: 6,
        };
        let mut runtime = initial;
        let descriptor = SubDSteeringDescriptor {
            classifier_flags: 0x16,
            ..ORDINARY_TYPE9_SUB_D
        };
        assert_eq!(
            apply_type9_sub_d(
                descriptor,
                &mut runtime,
                known(samples(0, 0, 0, 0), 0),
                20_000,
                20_000,
            ),
            Type9SubDStep::UnsupportedDescriptor
        );
        assert_eq!(runtime, initial);
    }

    #[test]
    fn type26_flags_skip_object_and_material_classes() {
        let mut grid = terrain(0, -100);
        cell_mut(&mut grid, 1, 1).attribute = 1;
        let point = [0x0180i16, 0x0180];
        assert_eq!(classify_terrain(&grid, point, 0x17), 2);
        assert_eq!(classify_terrain(&grid, point, 0x12), 0);

        cell_mut(&mut grid, 1, 1).attribute = 0;
        cell_mut(&mut grid, 1, 1).terrain_type = 0x10;
        assert_eq!(classify_terrain(&grid, point, 0x17), 1);
        assert_eq!(classify_terrain(&grid, point, 0x12), 0);
    }

    #[test]
    fn type26_steering_uses_divisor_128() {
        let mut runtime = Type9SubDRuntime::default();
        let result = apply_type9_sub_d(
            INTRO2_TYPE26_SUB_D,
            &mut runtime,
            known(samples(0, 0, 2, 0), 0),
            20_000,
            20_000,
        );
        assert_eq!(runtime.yaw_rate_raw, -0x1_0000 / 128);
        // Retail SAR keeps the negative fractional step: (-512 * 5000) >> 15.
        assert_eq!(result, Type9SubDStep::Applied { yaw_step_raw: -79 });
    }

    #[test]
    fn native_type16_sub_d_uses_authored_600_probe_and_128_steering() {
        let probe = type9_probe_plan(
            [1000, 25, 2000],
            [i32::MAX, 0, 0],
            [0, 0, i32::MAX],
            1,
            INTRO2_TYPE16_SUB_D,
        );
        assert_eq!(
            probe,
            Type9SubDProbePlan {
                left_raw: [745, 2599],
                half_forward_raw: [1000, 2299],
                right_raw: [1255, 2599],
                rear_raw: [1000, 1701],
            }
        );
        // Same object avoidance branch as flags13/Type47, with Type16's own
        // divisor. Source 20360 independently consumes global_dt/4=5000.
        let mut runtime = Type9SubDRuntime::from_constructor();
        assert_eq!(
            apply_type9_sub_d(
                INTRO2_TYPE16_SUB_D,
                &mut runtime,
                known(samples(0, 0, 2, 0), 0),
                20_000,
                20_000
            ),
            Type9SubDStep::Applied { yaw_step_raw: -79 }
        );
        assert_eq!(runtime.yaw_rate_raw, -512);
        assert_eq!(runtime.last_yaw_step_raw, -79);
        let before = runtime;
        assert_eq!(
            apply_type9_sub_d(
                SubDSteeringDescriptor {
                    enable_pitch_steering_raw: 1,
                    ..INTRO2_TYPE16_SUB_D
                },
                &mut runtime,
                known(samples(0, 0, 0, 0), 0),
                20_000,
                20_000
            ),
            Type9SubDStep::UnsupportedDescriptor
        );
        assert_eq!(
            runtime, before,
            "this admission does not invent a pitch runtime"
        );
    }

    #[test]
    fn type47_flags_take_object_class_and_skip_material() {
        let mut grid = terrain(0, -100);
        cell_mut(&mut grid, 1, 1).attribute = 1;
        let point = [0x0180i16, 0x0180];
        assert_eq!(classify_terrain(&grid, point, 0x13), 2);
        assert_eq!(classify_terrain(&grid, point, 0x12), 0);

        cell_mut(&mut grid, 1, 1).attribute = 0;
        cell_mut(&mut grid, 1, 1).terrain_type = 0x10;
        assert_eq!(classify_terrain(&grid, point, 0x17), 1);
        assert_eq!(classify_terrain(&grid, point, 0x13), 0);
    }

    #[test]
    fn type47_steering_uses_divisor_64() {
        let mut runtime = Type9SubDRuntime::default();
        let result = apply_type9_sub_d(
            TYPE47_SUB_D,
            &mut runtime,
            known(samples(0, 0, 2, 0), 0),
            20_000,
            20_000,
        );
        assert_eq!(runtime.yaw_rate_raw, -0x1_0000 / 64);
        assert!(matches!(result, Type9SubDStep::Applied { .. }));
    }

    #[test]
    fn cleansing_vehicle_admits_water_and_infection_but_rejects_objects() {
        let mut grid = terrain(-10, 0);
        let point = [0x0180i16, 0x0180];
        cell_mut(&mut grid, 1, 1).terrain_type = 0x10;
        assert_eq!(classify_terrain(&grid, point, 0x13), 5);
        assert_eq!(
            classify_terrain(&grid, point, CLEANSING_VEHICLE_SUB_D.classifier_flags),
            0
        );
        cell_mut(&mut grid, 1, 1).attribute = 1;
        assert_eq!(
            classify_terrain(&grid, point, CLEANSING_VEHICLE_SUB_D.classifier_flags),
            2
        );
        let mut runtime = Type9SubDRuntime::default();
        assert!(matches!(
            apply_type9_sub_d(
                CLEANSING_VEHICLE_SUB_D,
                &mut runtime,
                known(samples(0, 0, 2, 0), 0),
                20_000,
                20_000
            ),
            Type9SubDStep::Applied { .. }
        ));
        assert_eq!(runtime.yaw_rate_raw, -0x1_0000 / 64);
    }

    #[test]
    fn zebrafish_shallow_water_gate_uses_signed_center_depth_and_strict_boundary() {
        let point = [-128_i16, -128_i16];
        for (ground, sea, expected) in [
            (-32, -769, 6), // depth255
            (-32, -768, 0), // depth256
            (-32, -767, 0), // depth257
            (-32, -1024, 6),
            (-32, -1056, 6), // land above the sea
            (127, -4096, 6),
            (-128, 4096, 0),
        ] {
            let grid = terrain(ground, sea);
            assert_eq!(
                classify_terrain(&grid, point, TYPE62_SUB_D.classifier_flags),
                expected,
                "ground{ground} sea{sea}"
            );
        }

        let mut grid = terrain(-16, 0);
        // The wrapped center is cell255,255; a dry adjacent corner does not
        // replace its512-unit depth. Conversely, that center becoming dry
        // must win before the steep footprint, object and material arms.
        cell_mut(&mut grid, 0, 0).height = 0;
        assert_eq!(classify_terrain(&grid, point, 0x20), 0);
        cell_mut(&mut grid, 255, 255).height = 0;
        cell_mut(&mut grid, 255, 255).attribute = 1;
        cell_mut(&mut grid, 255, 255).terrain_type = 0x10;
        assert_eq!(classify_terrain(&grid, point, 0x37), 6);
        assert_eq!(classify_terrain(&grid, point, 0x17), 4);
    }

    #[test]
    fn zebrafish_class_six_uses_the_existing_generic_avoidance_turn() {
        let mut runtime = Type9SubDRuntime::default();
        assert!(matches!(
            apply_type9_sub_d(
                TYPE62_SUB_D,
                &mut runtime,
                known(samples(0, 6, 0, 0), 0),
                20_000,
                20_000,
            ),
            Type9SubDStep::Applied { .. }
        ));
        // 41F660's non4/non5 central classification, with divisor32.
        assert_eq!(runtime.yaw_rate_raw, 0x1_8000 / 32);
    }
}
