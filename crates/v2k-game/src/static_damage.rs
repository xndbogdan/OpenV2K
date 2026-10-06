//! Recovered static-object damage programs and radial static scan.
//!
//! `FUN_00427950` filters a two-slot damage packet, applies the already-burned
//! severity rule, consumes the shared retail RNG when the result is below the
//! authored chance gate, and only then checks the active-program cell key.
//! `FUN_004281A0` advances accepted programs in FIFO order. This module admits
//! only the ten kinds whose complete action closure is now owned by the
//! port: kinds 0/28's event-18-plus-radial program, kinds 1/8's short
//! event-18 program, kinds 3/10/11's event-30 destruction programs, and kind
//! 9's exact opcode-13 class-79 scatter, plus kind 29's ordinary class-79
//! event followed by the shared layered destruction tail. Kind 27 adds the
//! exact camera-focus-gated full-frame request before its authored burn/radial
//! tail. It preserves that decision and timing layer without coupling it to
//! particles, audio, model lookup, camera state, or dynamic entities.
//! Kinds2/4/22 additionally admit27950's null-program branch: an accepted hit
//! requests immediate4337E0 ignition without allocating a timed-program node.
//! Class38 also submits its standalone 4C98F8 terrain-light program through
//! the same 28720 cell registry; this program requires no static object.
//!
//! The caller owns the one shared retail random stream. Direct collision hits
//! are submitted before that frame's scheduler pass, as in `FUN_0044FFA0`, so
//! their initial records execute during that pass and inherit its delta. Hits
//! produced synchronously by a radial action are appended while that source
//! node is in flight, but the pass traverses a saved FIFO membership. Their
//! initial records therefore wait for the next update and never receive
//! retroactive elapsed time.

use std::collections::VecDeque;

use v2k_formats::terrain::{TerrainGrid, GRID_SIZE};

use crate::damage::{DamagePacket, DamageProfile};
use crate::radial_damage::{scale_radial_damage, RadialDamageTemplate};
use crate::static_kind_catalog::{
    static_damage_profile, static_kind_descriptor, static_program, static_radial_template,
    StaticProgramRecord, FIREBALL_GROUND_PROGRAM_VA, STATIC_KIND_27_AUXILIARY_BLOB,
    STATIC_OPCODE_13_BLOB,
};

/// Static-object kinds whose recovered destruction programs are implemented.
pub const KIND_0_STATIC_OBJECT: u32 = 0;
pub const KIND_1_STATIC_OBJECT: u32 = 1;
pub const KIND_2_STATIC_OBJECT: u32 = 2;
pub const KIND_3_STATIC_OBJECT: u32 = 3;
pub const KIND_4_STATIC_OBJECT: u32 = 4;
pub const KIND_22_STATIC_OBJECT: u32 = 22;
pub const KIND_8_STATIC_OBJECT: u32 = 8;
pub const KIND_9_STATIC_OBJECT: u32 = 9;
pub const KIND_10_STATIC_OBJECT: u32 = 10;
pub const KIND_11_STATIC_OBJECT: u32 = 11;
pub const KIND_27_STATIC_OBJECT: u32 = 27;
pub const KIND_28_STATIC_OBJECT: u32 = 28;
pub const KIND_29_STATIC_OBJECT: u32 = 29;
/// Kind 10 bypasses the generic already-burned severity halving rule.
pub const BURNED_SEVERITY_EXCEPTION_KIND: u32 = KIND_10_STATIC_OBJECT;
/// Section-10 terrain-state bit selected by the program's opcode 10.
pub const BURNED_TERRAIN_TYPE_BIT: u8 = 0x08;
/// Kind 0's authored probabilistic gate in filtered damage units.
pub const KIND_0_CHANCE_GATE_RAW: i32 = 1_000;
/// Time of the second effect/sound pair.
pub const KIND_0_SECOND_EFFECT_US: u32 = 500_000;
/// Additional delay from the second effect to ignition and radial delivery.
pub const KIND_0_IGNITION_DELAY_US: u32 = 100_000;
const KIND_0_DAMAGE_PROFILE_VA: u32 = 0x004C_9BE8;
const KIND_0_TIMED_PROGRAM_VA: u32 = 0x004C_9918;
const KIND_0_RADIAL_TEMPLATE_VA: u32 = 0x004C_9898;
const KIND_1_DAMAGE_PROFILE_VA: u32 = 0x004C_9C20;
const KIND_2_DAMAGE_PROFILE_VA: u32 = 0x004C_9C58;
const KIND_3_DAMAGE_PROFILE_VA: u32 = 0x004C_9CC8;
const KIND_8_DAMAGE_PROFILE_VA: u32 = 0x004C_9D38;
const KIND_9_DAMAGE_PROFILE_VA: u32 = 0x004C_9C90;
const KIND_9_TIMED_PROGRAM_VA: u32 = 0x004C_9B70;
const KIND_9_OPCODE_13_PAYLOAD_VA: u32 = 0x004C_9B58;
const KIND_9_OPCODE_13_RAW_WORDS: [u32; 5] = [0x0000_0002, 0, 0x4F4F_0000, 0, 0];
const KIND_10_DAMAGE_PROFILE_VA: u32 = 0x004C_9D70;
const KIND_11_DAMAGE_PROFILE_VA: u32 = 0x004C_9DA8;
const KIND_27_DAMAGE_PROFILE_VA: u32 = 0x004C_9DE0;
const KIND_27_TIMED_PROGRAM_VA: u32 = 0x004C_99A8;
const KIND_27_RADIAL_TEMPLATE_VA: u32 = 0x004C_98B8;
const KIND_27_AUXILIARY_VA: u32 = 0x004C_9E28;
const KIND_27_AUXILIARY_RAW_WORDS: [u32; 4] = [0x800, 20, 0x400, 10];
const KIND_29_DAMAGE_PROFILE_VA: u32 = 0x004C_9D00;
const KIND_29_TIMED_PROGRAM_VA: u32 = 0x004C_9B00;
const SIMPLE_DESTRUCTION_PROGRAM_VA: u32 = 0x004C_9978;
const LAYERED_DESTRUCTION_PROGRAM_VA: u32 = 0x004C_9A60;
const LAYERED_DESTRUCTION_NO_RADIAL_PROGRAM_VA: u32 = 0x004C_9AB8;
const LAYERED_DESTRUCTION_RADIAL_TEMPLATE_VA: u32 = 0x004C_9878;

/// Full seven-channel profile at executable address `0x004C9BE8`.
pub const KIND_0_DAMAGE_PROFILE: DamageProfile = DamageProfile {
    thresholds_raw: [0, 4_000, 2_000, 0, 6_000, 0, 0],
    multipliers_q8: [0, 256, 64, 512, 256, 0, 0],
};

/// Raw radial template referenced by kind 0's opcode-12 record (`0x004C9898`).
pub const KIND_0_RADIAL_TEMPLATE: RadialDamageTemplate = RadialDamageTemplate {
    inner_radius_raw: 0x100,
    outer_radius_raw: 0x200,
    impulse_raw: 2_000,
    packet: DamagePacket {
        channels: [1, 3],
        amounts_raw: [1_000, 1_000],
    },
    trailing_raw: [-1, 0],
};

/// Inclusive camera-focus distance used by kind 27's two opcode-6 records.
pub const KIND_27_FULL_FRAME_MAX_DISTANCE_RAW: i32 = 0x1400;

/// Raw radial template referenced by kind 27's opcode-12 record (`0x004C98B8`).
pub const KIND_27_RADIAL_TEMPLATE: RadialDamageTemplate = RadialDamageTemplate {
    inner_radius_raw: 0x800,
    outer_radius_raw: 0xDAC,
    impulse_raw: 2_000,
    packet: DamagePacket {
        channels: [2, 3],
        amounts_raw: [6_000, 6_000],
    },
    trailing_raw: [-1, 0],
};

/// Current data needed from a static object at one Section-10 cell.
///
/// `terrain_height_byte` is the single signed cell height used by the damage
/// program. It is deliberately not the four-corner render height or the
/// collision path's `0x7f`-centred height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticDamageTargetState {
    pub kind_index: u32,
    pub terrain_type: u8,
    pub terrain_height_byte: u8,
    /// Unsigned model-header word at `+0x0A`, selected from the current slot.
    pub collision_radius_raw: u16,
    /// Unsigned model-header word at `+0x08`, selected from the current slot.
    ///
    /// `FUN_004281A0` passes this independent extent to `FUN_00441200` for
    /// event 30/58 layered destruction. It is not interchangeable with the
    /// collision radius at `+0x0A`.
    pub effect_extent_raw: u16,
}

/// A current static target and its wrapped Section-10 cell key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticDamageTarget {
    pub cell: [u8; 2],
    pub state: StaticDamageTargetState,
}

/// One sampled chance decision. Deterministic severities have no sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticDamageChanceSample {
    pub cutoff: u16,
    pub roll: u16,
}

/// Result of attempting to start one admitted static program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticDamageOutcome {
    UnsupportedKind {
        kind_index: u32,
    },
    NoDamage {
        severity_raw: i32,
    },
    ///27950's accepted, unburned null-program branch calls4337E0(...,1)
    /// synchronously, before the caller continues with another recipient.
    ImmediateBurn {
        cell: [u8; 2],
        severity_raw: i32,
        sample: Option<StaticDamageChanceSample>,
    },
    ChanceRejected {
        severity_raw: i32,
        sample: StaticDamageChanceSample,
    },
    /// Retail still filters the packet and consumes any probabilistic chance
    /// draw before reaching this branch, but an already-burned non-kind-10
    /// tile neither starts a second program nor enters active-cell dedup.
    BurnedIgnored {
        severity_raw: i32,
        sample: Option<StaticDamageChanceSample>,
    },
    /// Kind 10 is the sole retail exception: a positive accepted hit on an
    /// already-burned cell clears bit `0x08` and increments the live
    /// Section-10 attribute byte instead of starting a timed program.
    BurnedKind10Transition {
        cell: [u8; 2],
        severity_raw: i32,
        sample: Option<StaticDamageChanceSample>,
    },
    Duplicate {
        severity_raw: i32,
        sample: Option<StaticDamageChanceSample>,
    },
    Started {
        severity_raw: i32,
        sample: Option<StaticDamageChanceSample>,
    },
}

/// Direct `27B20 -> 28720` terrain-deformation admission. Unlike a hit,
/// this entry has no damage filter, chance draw or already-burned gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StaticCraterOutcome {
    BurnCell,
    Started,
    Duplicate,
    UnsupportedKind { kind_index: u32 },
}

/// Direct 441B70 -> 28720 admission of the cell-only 4C98F8 program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticGroundProgramOutcome {
    Started,
    Duplicate,
}

/// Typed side effects produced by the recovered program.
///
/// The scheduler never performs these effects itself. Applying this sequence
/// in order keeps mutable terrain, audio, particle, and radial-delivery borrows
/// outside the timing state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticDamageAction {
    Effect18 {
        position_raw: [i16; 3],
    },
    CommonExplosion30 {
        position_raw: [i16; 3],
        source_extent_raw: u16,
    },
    /// Kind 9's exact opcode-13 payload: two pacing-scaled class-79 attempts
    /// through `FUN_004407D0`, with minimum extent 16 and velocity scale one.
    BallisticScatter79 {
        position_raw: [i16; 3],
    },
    /// Kind 29's opcode-3 argument `0x4F`: one ordinary class-79 event through
    /// `FUN_004410B0`, with zero input velocity and no water substitution.
    OrdinaryEffect79 {
        position_raw: [i16; 3],
    },
    FixedSound {
        sound_id: u16,
        position_raw: [i16; 3],
        gain_q16: u32,
        rate_q16: u32,
    },
    /// Opcode 6 compares the static cell base with the current chase-camera
    /// focus spring. The runtime presentation owner applies this request.
    RequestFullFrameSequenceWithin {
        position_raw: [i16; 3],
        max_distance_raw: i32,
    },
    /// Opcode10 invokes 4337E0 and its synchronous burn-change callbacks.
    SetBurned {
        cell: [u8; 2],
    },
    /// Opcode11 calls 37100, preserving the low five terrain-type bits.
    LowerTerrainLight {
        cell: [u8; 2],
        amount: i32,
    },
    Radial {
        /// Cell whose opcode-12 program remains registered until this radial
        /// delivery returns and the following opcode 0 removes its list node.
        source_cell: [u8; 2],
        origin_raw: [i16; 3],
        template: RadialDamageTemplate,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectPlacement {
    QuarterHeight,
    HalfHeight,
    ThreeQuarterHeight,
    SubmittedPoint,
}

impl EffectPlacement {
    fn from_opcode(opcode: u32) -> Option<Self> {
        match opcode {
            2 => Some(Self::QuarterHeight),
            3 => Some(Self::HalfHeight),
            4 => Some(Self::ThreeQuarterHeight),
            5 => Some(Self::SubmittedPoint),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdmittedStaticOpcode {
    Terminate,
    RequireStatic { raw: u32 },
    Effect18 { placement: EffectPlacement },
    CommonExplosion30 { placement: EffectPlacement },
    BallisticScatter79,
    OrdinaryEffect79 { placement: EffectPlacement },
    FixedSound { sound_id: u16 },
    RequestFullFrameSequenceWithin { max_distance_raw: i32 },
    SetBurned,
    LowerTerrainLight { amount: i32 },
    Radial { template: RadialDamageTemplate },
}

/// Decode only the closure proven for the ten admitted kinds. The catalog
/// contains other opcodes and programs, but recording them is not runtime
/// admission.
fn decode_admitted_static_record(record: StaticProgramRecord) -> Option<AdmittedStaticOpcode> {
    match (record.opcode_raw(), record.argument_raw()) {
        (0, _) => Some(AdmittedStaticOpcode::Terminate),
        (1, raw) => Some(AdmittedStaticOpcode::RequireStatic { raw }),
        (opcode @ (3 | 4), 0x12) => Some(AdmittedStaticOpcode::Effect18 {
            placement: EffectPlacement::from_opcode(opcode)?,
        }),
        (3, 0x4F) => Some(AdmittedStaticOpcode::OrdinaryEffect79 {
            placement: EffectPlacement::HalfHeight,
        }),
        (opcode @ (2 | 3 | 4 | 5), 0x1E) => Some(AdmittedStaticOpcode::CommonExplosion30 {
            placement: EffectPlacement::from_opcode(opcode)?,
        }),
        (13, payload_va)
            if payload_va == KIND_9_OPCODE_13_PAYLOAD_VA
                && STATIC_OPCODE_13_BLOB.source_va() == KIND_9_OPCODE_13_PAYLOAD_VA
                && *STATIC_OPCODE_13_BLOB.raw_words() == KIND_9_OPCODE_13_RAW_WORDS =>
        {
            Some(AdmittedStaticOpcode::BallisticScatter79)
        }
        (8, sound_id) => Some(AdmittedStaticOpcode::FixedSound {
            sound_id: u16::try_from(sound_id).ok()?,
        }),
        (6, max_distance_raw) if max_distance_raw == KIND_27_FULL_FRAME_MAX_DISTANCE_RAW as u32 => {
            Some(AdmittedStaticOpcode::RequestFullFrameSequenceWithin {
                max_distance_raw: max_distance_raw as i32,
            })
        }
        (10, _) => Some(AdmittedStaticOpcode::SetBurned),
        (11, 1) => Some(AdmittedStaticOpcode::LowerTerrainLight { amount: 1 }),
        (
            12,
            template_va @ (KIND_0_RADIAL_TEMPLATE_VA
            | KIND_27_RADIAL_TEMPLATE_VA
            | LAYERED_DESTRUCTION_RADIAL_TEMPLATE_VA),
        ) => Some(AdmittedStaticOpcode::Radial {
            template: static_radial_template(template_va)?.decoded(),
        }),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
struct AdmittedStaticCatalog {
    profile: DamageProfile,
    chance_gate_raw: i32,
    program_va: u32,
}

fn admitted_static_catalog(kind_index: u32) -> Option<AdmittedStaticCatalog> {
    let descriptor = static_kind_descriptor(kind_index)?;
    let expected = match kind_index {
        KIND_0_STATIC_OBJECT => (KIND_0_DAMAGE_PROFILE_VA, KIND_0_TIMED_PROGRAM_VA),
        KIND_1_STATIC_OBJECT => (KIND_1_DAMAGE_PROFILE_VA, SIMPLE_DESTRUCTION_PROGRAM_VA),
        KIND_2_STATIC_OBJECT => (KIND_2_DAMAGE_PROFILE_VA, 0),
        KIND_3_STATIC_OBJECT => (KIND_3_DAMAGE_PROFILE_VA, LAYERED_DESTRUCTION_PROGRAM_VA),
        KIND_4_STATIC_OBJECT => (KIND_27_DAMAGE_PROFILE_VA, 0),
        KIND_8_STATIC_OBJECT => (KIND_8_DAMAGE_PROFILE_VA, SIMPLE_DESTRUCTION_PROGRAM_VA),
        KIND_9_STATIC_OBJECT => (KIND_9_DAMAGE_PROFILE_VA, KIND_9_TIMED_PROGRAM_VA),
        KIND_10_STATIC_OBJECT => (
            KIND_10_DAMAGE_PROFILE_VA,
            LAYERED_DESTRUCTION_NO_RADIAL_PROGRAM_VA,
        ),
        KIND_11_STATIC_OBJECT => (KIND_11_DAMAGE_PROFILE_VA, LAYERED_DESTRUCTION_PROGRAM_VA),
        KIND_22_STATIC_OBJECT => (0x004C_9BB0, 0),
        KIND_27_STATIC_OBJECT => (KIND_27_DAMAGE_PROFILE_VA, KIND_27_TIMED_PROGRAM_VA),
        KIND_28_STATIC_OBJECT => (KIND_11_DAMAGE_PROFILE_VA, KIND_0_TIMED_PROGRAM_VA),
        KIND_29_STATIC_OBJECT => (KIND_29_DAMAGE_PROFILE_VA, KIND_29_TIMED_PROGRAM_VA),
        _ => return None,
    };
    if (
        descriptor.damage_profile_va(),
        descriptor.timed_program_va(),
    ) != expected
    {
        return None;
    }
    if kind_index == KIND_27_STATIC_OBJECT
        && (descriptor.field_0c_raw() != KIND_27_AUXILIARY_VA
            || STATIC_KIND_27_AUXILIARY_BLOB.source_va() != KIND_27_AUXILIARY_VA
            || *STATIC_KIND_27_AUXILIARY_BLOB.raw_words() != KIND_27_AUXILIARY_RAW_WORDS)
    {
        return None;
    }
    let profile = static_damage_profile(descriptor.damage_profile_va())?.decoded();
    // The null pointer is an authored program choice, not missing evidence:
    //427AC4 branches to427AF8/4337E0. Kinds2/4/22 have authored null programs;
    //22 participates in the completed-world Hive initializer's static radial.
    let immediate_gate = match kind_index {
        // Kind2's zero gate follows the nonpositive-severity return, so no
        // accepted damage can enter the chance division or consume RNG.
        KIND_2_STATIC_OBJECT => Some(0),
        KIND_4_STATIC_OBJECT => Some(500),
        KIND_22_STATIC_OBJECT => Some(8_000),
        _ => None,
    };
    if let Some(gate) = immediate_gate {
        return (descriptor.chance_gate_raw() == gate).then_some(AdmittedStaticCatalog {
            profile,
            chance_gate_raw: descriptor.chance_gate_raw(),
            program_va: 0,
        });
    }
    let program = static_program(descriptor.timed_program_va())?;
    let records = program.records();
    if records.is_empty()
        || !records.last()?.is_terminator()
        || records[..records.len() - 1]
            .iter()
            .any(StaticProgramRecord::is_terminator)
        || records
            .iter()
            .copied()
            .any(|record| decode_admitted_static_record(record).is_none())
    {
        return None;
    }
    Some(AdmittedStaticCatalog {
        profile,
        chance_gate_raw: descriptor.chance_gate_raw(),
        program_va: program.source_va(),
    })
}

/// Whether the recovered static-damage runtime admits this object kind.
///
/// This queries the same validated descriptor/program closure used by
/// [`StaticDamageScheduler::submit_hit`]; catalog presence alone is not
/// sufficient runtime admission.
pub(crate) fn static_damage_kind_is_admitted(kind_index: u32) -> bool {
    admitted_static_catalog(kind_index).is_some()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActiveStaticProgram {
    id: u64,
    cell: [u8; 2],
    /// Exact submitted point would be required by opcode 5. None of the ten
    /// admitted programs reaches that opcode, so widening admission must first
    /// populate it.
    submitted_position_raw: Option<[i16; 3]>,
    elapsed_us: u32,
    record_index: usize,
    require_static_raw: u32,
    program_va: u32,
}

#[derive(Debug, Clone)]
struct PendingAdvance {
    delta_us: u32,
    original_program_ids: VecDeque<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InFlightCompletion {
    KeepActive,
    Retire,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticDamageNodeToken(u64);

/// One retail FIFO node's complete ordered side-effect sequence.
///
/// The caller must apply these actions and complete the token before asking
/// for the next node. That makes this node's terrain mutations visible to the
/// next node's current-static lookup, as in `FUN_004281A0`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticDamageNodeBatch {
    token: StaticDamageNodeToken,
    source_cell: [u8; 2],
    actions: Vec<StaticDamageAction>,
}

impl StaticDamageNodeBatch {
    pub fn source_cell(&self) -> [u8; 2] {
        self.source_cell
    }

    pub fn actions(&self) -> &[StaticDamageAction] {
        &self.actions
    }

    pub fn into_parts(self) -> (StaticDamageNodeToken, Vec<StaticDamageAction>) {
        (self.token, self.actions)
    }
}

#[derive(Debug, Clone, Copy)]
struct InFlightProgram {
    token: StaticDamageNodeToken,
    original_index: usize,
    program: ActiveStaticProgram,
    completion: InFlightCompletion,
}

#[derive(Debug, Default)]
pub struct StaticDamageScheduler {
    active: VecDeque<ActiveStaticProgram>,
    next_program_id: u64,
    pending_advance: Option<PendingAdvance>,
    in_flight: Option<InFlightProgram>,
}

impl StaticDamageScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Deep-copy the FIFO owner only for the isolated Main Base abort
    /// transaction.  Static hits from a failed speculative sweep must not
    /// advance ids, retain programs, or disturb an in-flight node.
    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            active: self.active.clone(),
            next_program_id: self.next_program_id,
            pending_advance: self.pending_advance.clone(),
            in_flight: self.in_flight,
        }
    }

    pub fn active_program_count(&self) -> usize {
        self.active.len() + usize::from(self.in_flight.is_some())
    }

    pub fn contains_cell(&self, cell: [u8; 2]) -> bool {
        self.active.iter().any(|program| program.cell == cell)
            || self
                .in_flight
                .is_some_and(|in_flight| in_flight.program.cell == cell)
    }

    /// Filter and submit one hit to an admitted authored program.
    ///
    /// `next_shared_random_u16` must draw from the caller's shared retail RNG;
    /// this subsystem intentionally owns no private seed. For severities in
    /// `1..chance_gate`, the draw happens before the burned-state and
    /// active-cell checks, exactly as in `FUN_00427950`.
    pub fn submit_hit(
        &mut self,
        target: StaticDamageTarget,
        packet: DamagePacket,
        next_shared_random_u16: &mut impl FnMut() -> u16,
    ) -> StaticDamageOutcome {
        let Some(catalog) = admitted_static_catalog(target.state.kind_index) else {
            return StaticDamageOutcome::UnsupportedKind {
                kind_index: target.state.kind_index,
            };
        };
        let severity_raw = filtered_static_severity_raw(
            packet,
            &catalog.profile,
            target.state.kind_index,
            target.state.terrain_type,
        );
        if severity_raw <= 0 {
            return StaticDamageOutcome::NoDamage { severity_raw };
        }

        let sample = if severity_raw < catalog.chance_gate_raw {
            let cutoff = ((severity_raw * i32::from(u16::MAX)) / catalog.chance_gate_raw) as u16;
            let sample = StaticDamageChanceSample {
                cutoff,
                roll: next_shared_random_u16(),
            };
            if sample.cutoff < sample.roll {
                return StaticDamageOutcome::ChanceRejected {
                    severity_raw,
                    sample,
                };
            }
            Some(sample)
        } else {
            None
        };

        if target.state.terrain_type & BURNED_TERRAIN_TYPE_BIT != 0 {
            if target.state.kind_index == KIND_10_STATIC_OBJECT {
                return StaticDamageOutcome::BurnedKind10Transition {
                    cell: target.cell,
                    severity_raw,
                    sample,
                };
            }
            return StaticDamageOutcome::BurnedIgnored {
                severity_raw,
                sample,
            };
        }

        if catalog.program_va == 0 {
            return StaticDamageOutcome::ImmediateBurn {
                cell: target.cell,
                severity_raw,
                sample,
            };
        }

        if self.contains_cell(target.cell) {
            return StaticDamageOutcome::Duplicate {
                severity_raw,
                sample,
            };
        }

        let id = self.next_program_id;
        self.next_program_id = self.next_program_id.wrapping_add(1);
        self.active.push_back(ActiveStaticProgram {
            id,
            cell: target.cell,
            submitted_position_raw: None,
            elapsed_us: 0,
            record_index: 0,
            require_static_raw: 0,
            program_va: catalog.program_va,
        });
        StaticDamageOutcome::Started {
            severity_raw,
            sample,
        }
    }

    /// `FUN_00427B20` uses the kind's +08 program directly. A null program
    /// instead requests `337E0(...,1)` on the caller's live terrain cell.
    pub(crate) fn submit_crater_destruction(
        &mut self,
        cell: [u8; 2],
        kind_index: u32,
    ) -> StaticCraterOutcome {
        let Some(descriptor) = static_kind_descriptor(kind_index) else {
            return StaticCraterOutcome::UnsupportedKind { kind_index };
        };
        if descriptor.timed_program_va() == 0 {
            return StaticCraterOutcome::BurnCell;
        }
        let Some(catalog) = admitted_static_catalog(kind_index) else {
            return StaticCraterOutcome::UnsupportedKind { kind_index };
        };
        if self.contains_cell(cell) {
            return StaticCraterOutcome::Duplicate;
        }
        let id = self.next_program_id;
        self.next_program_id = self.next_program_id.wrapping_add(1);
        self.active.push_back(ActiveStaticProgram {
            id,
            cell,
            // The admitted programs contain no opcode5. Their position
            // consumers read the current cell exactly as the hit entry does.
            submitted_position_raw: None,
            elapsed_us: 0,
            record_index: 0,
            require_static_raw: 0,
            program_va: catalog.program_va,
        });
        StaticCraterOutcome::Started
    }

    /// Queue class38's exact 4C98F8 program in the shared 28720 cell registry.
    ///
    /// No hit filter, static/model lookup, height or RNG is consumed. Particle
    /// traversal follows the static pass, so its newly appended node executes
    /// at the next pass, including when that pass has a zero delta.
    pub fn submit_fireball_ground_program(&mut self, cell: [u8; 2]) -> StaticGroundProgramOutcome {
        if self.contains_cell(cell) {
            return StaticGroundProgramOutcome::Duplicate;
        }
        let id = self.next_program_id;
        self.next_program_id = self.next_program_id.wrapping_add(1);
        self.active.push_back(ActiveStaticProgram {
            id,
            cell,
            // Both records are cell-only; the 28720 sampled height is unused.
            submitted_position_raw: None,
            elapsed_us: 0,
            record_index: 0,
            require_static_raw: 0,
            program_va: FIREBALL_GROUND_PROGRAM_VA,
        });
        StaticGroundProgramOutcome::Started
    }

    /// Capture the current FIFO membership and begin one retail scheduler pass.
    ///
    /// Programs submitted while applying a returned radial batch are appended
    /// to `active` but are absent from this ID queue, so they cannot inherit
    /// the current pass's delta.
    pub fn begin_advance(&mut self, delta_us: u32) {
        assert!(
            self.pending_advance.is_none() && self.in_flight.is_none(),
            "complete the previous static-damage pass before beginning another"
        );
        self.pending_advance = Some(PendingAdvance {
            delta_us,
            original_program_ids: self.active.iter().map(|program| program.id).collect(),
        });
    }

    /// Advance the next original FIFO node which produces external actions.
    ///
    /// Nodes with no ready actions are completed internally. After a batch is
    /// returned, the source remains registered for active-cell dedup until
    /// [`Self::complete_node_batch`] is called.
    pub fn next_node_batch(
        &mut self,
        mut lookup_static: impl FnMut([u8; 2]) -> Option<StaticDamageTargetState>,
    ) -> Option<StaticDamageNodeBatch> {
        assert!(
            self.in_flight.is_none(),
            "apply and complete the current static-damage node before sampling the next"
        );

        loop {
            let (delta_us, program_id) = {
                let pending = self
                    .pending_advance
                    .as_mut()
                    .expect("begin_advance must precede next_node_batch");
                let Some(program_id) = pending.original_program_ids.pop_front() else {
                    self.pending_advance = None;
                    return None;
                };
                (pending.delta_us, program_id)
            };
            let Some(original_index) = self
                .active
                .iter()
                .position(|program| program.id == program_id)
            else {
                continue;
            };
            let mut program = self
                .active
                .remove(original_index)
                .expect("position came from active FIFO");
            let (actions, completion) =
                advance_admitted_static_program(&mut program, delta_us, &mut lookup_static);

            if actions.is_empty() {
                if completion == InFlightCompletion::KeepActive {
                    self.active
                        .insert(original_index.min(self.active.len()), program);
                }
                continue;
            }

            let token = StaticDamageNodeToken(program.id);
            let source_cell = program.cell;
            self.in_flight = Some(InFlightProgram {
                token,
                original_index,
                program,
                completion,
            });
            return Some(StaticDamageNodeBatch {
                token,
                source_cell,
                actions,
            });
        }
    }

    /// Commit one applied node batch and release its in-flight dedup key.
    pub fn complete_node_batch(&mut self, token: StaticDamageNodeToken) -> bool {
        if self.in_flight.as_ref().map(|batch| batch.token) != Some(token) {
            return false;
        }
        let in_flight = self.in_flight.take().expect("token matched in-flight node");
        if in_flight.completion == InFlightCompletion::KeepActive {
            self.active.insert(
                in_flight.original_index.min(self.active.len()),
                in_flight.program,
            );
        }
        true
    }

    /// Collect actions without applying their external effects.
    ///
    /// This is useful for isolated scheduler tests. Runtime code must use the
    /// node-at-a-time API so each batch is applied before the next lookup.
    pub fn advance(
        &mut self,
        delta_us: u32,
        mut lookup_static: impl FnMut([u8; 2]) -> Option<StaticDamageTargetState>,
    ) -> Vec<StaticDamageAction> {
        self.begin_advance(delta_us);
        let mut actions = Vec::new();
        while let Some(batch) = self.next_node_batch(&mut lookup_static) {
            let (token, batch_actions) = batch.into_parts();
            actions.extend(batch_actions);
            assert!(self.complete_node_batch(token));
        }
        actions
    }
}

fn advance_admitted_static_program(
    program: &mut ActiveStaticProgram,
    delta_us: u32,
    mut lookup_static: impl FnMut([u8; 2]) -> Option<StaticDamageTargetState>,
) -> (Vec<StaticDamageAction>, InFlightCompletion) {
    let Some(catalog_program) = static_program(program.program_va) else {
        return (Vec::new(), InFlightCompletion::Retire);
    };

    program.elapsed_us = program.elapsed_us.wrapping_add(delta_us);
    let Some(next_record) = catalog_program.records().get(program.record_index) else {
        return (Vec::new(), InFlightCompletion::Retire);
    };
    if program.elapsed_us < next_record.delay_us() {
        // FUN_004281A0 does not reacquire the current static until the next
        // record is due. A removed require-static target therefore retains its
        // FIFO/dedup node through the remaining delay.
        return (Vec::new(), InFlightCompletion::KeepActive);
    }
    let state = lookup_static(program.cell);
    if state.is_none() && program.require_static_raw != 0 {
        return (Vec::new(), InFlightCompletion::Retire);
    }

    let mut actions = Vec::new();
    loop {
        let Some(record) = catalog_program.records().get(program.record_index).copied() else {
            // A missing terminator is unsupported. Discard the not-yet-applied
            // batch rather than partially executing a malformed program.
            return (Vec::new(), InFlightCompletion::Retire);
        };
        if program.elapsed_us < record.delay_us() {
            return (actions, InFlightCompletion::KeepActive);
        }
        program.elapsed_us -= record.delay_us();

        let Some(opcode) = decode_admitted_static_record(record) else {
            return (Vec::new(), InFlightCompletion::Retire);
        };
        match opcode {
            AdmittedStaticOpcode::Terminate => {
                return (actions, InFlightCompletion::Retire);
            }
            AdmittedStaticOpcode::RequireStatic { raw } => {
                program.require_static_raw = raw;
                program.record_index += 1;
                if state.is_none() && raw != 0 {
                    return (Vec::new(), InFlightCompletion::Retire);
                }
            }
            AdmittedStaticOpcode::Effect18 { placement } => {
                let Some(state) = state else {
                    return (Vec::new(), InFlightCompletion::Retire);
                };
                let Some(position_raw) = effect_position_raw(program, state, placement) else {
                    return (Vec::new(), InFlightCompletion::Retire);
                };
                actions.push(StaticDamageAction::Effect18 { position_raw });
                program.record_index += 1;
            }
            AdmittedStaticOpcode::CommonExplosion30 { placement } => {
                let Some(state) = state else {
                    return (Vec::new(), InFlightCompletion::Retire);
                };
                let Some(position_raw) = effect_position_raw(program, state, placement) else {
                    return (Vec::new(), InFlightCompletion::Retire);
                };
                actions.push(StaticDamageAction::CommonExplosion30 {
                    position_raw,
                    source_extent_raw: state.effect_extent_raw,
                });
                program.record_index += 1;
            }
            AdmittedStaticOpcode::BallisticScatter79 => {
                let Some(state) = state else {
                    return (Vec::new(), InFlightCompletion::Retire);
                };
                let Some(position_raw) =
                    effect_position_raw(program, state, EffectPlacement::HalfHeight)
                else {
                    return (Vec::new(), InFlightCompletion::Retire);
                };
                actions.push(StaticDamageAction::BallisticScatter79 { position_raw });
                program.record_index += 1;
            }
            AdmittedStaticOpcode::OrdinaryEffect79 { placement } => {
                let Some(state) = state else {
                    return (Vec::new(), InFlightCompletion::Retire);
                };
                let Some(position_raw) = effect_position_raw(program, state, placement) else {
                    return (Vec::new(), InFlightCompletion::Retire);
                };
                actions.push(StaticDamageAction::OrdinaryEffect79 { position_raw });
                program.record_index += 1;
            }
            AdmittedStaticOpcode::FixedSound { sound_id } => {
                let Some(state) = state else {
                    return (Vec::new(), InFlightCompletion::Retire);
                };
                actions.push(fixed_sound(
                    sound_id,
                    program_base_raw(program.cell, state.terrain_height_byte),
                ));
                program.record_index += 1;
            }
            AdmittedStaticOpcode::RequestFullFrameSequenceWithin { max_distance_raw } => {
                let Some(state) = state else {
                    return (Vec::new(), InFlightCompletion::Retire);
                };
                actions.push(StaticDamageAction::RequestFullFrameSequenceWithin {
                    position_raw: program_base_raw(program.cell, state.terrain_height_byte),
                    max_distance_raw,
                });
                program.record_index += 1;
            }
            AdmittedStaticOpcode::LowerTerrainLight { amount } => {
                actions.push(StaticDamageAction::LowerTerrainLight {
                    cell: program.cell,
                    amount,
                });
                program.record_index += 1;
            }
            AdmittedStaticOpcode::SetBurned => {
                if state.is_none() {
                    return (Vec::new(), InFlightCompletion::Retire);
                }
                actions.push(StaticDamageAction::SetBurned { cell: program.cell });
                program.record_index += 1;
            }
            AdmittedStaticOpcode::Radial { template } => {
                let Some(state) = state else {
                    return (Vec::new(), InFlightCompletion::Retire);
                };
                actions.push(StaticDamageAction::Radial {
                    source_cell: program.cell,
                    origin_raw: program_base_raw(program.cell, state.terrain_height_byte),
                    template,
                });
                program.record_index += 1;
            }
        }
    }
}

fn effect_position_raw(
    program: &ActiveStaticProgram,
    state: StaticDamageTargetState,
    placement: EffectPlacement,
) -> Option<[i16; 3]> {
    if placement == EffectPlacement::SubmittedPoint {
        return program.submitted_position_raw;
    }

    let base = program_base_raw(program.cell, state.terrain_height_byte);
    let radius = state.collision_radius_raw;
    let y_offset = match placement {
        EffectPlacement::QuarterHeight => quarter(radius),
        EffectPlacement::HalfHeight => half(radius),
        EffectPlacement::ThreeQuarterHeight => three_quarters(radius),
        EffectPlacement::SubmittedPoint => unreachable!("handled above"),
    };
    Some([
        base[0],
        base[1].wrapping_add(y_offset),
        base[2].wrapping_sub(quarter(radius)),
    ])
}

/// Apply the common post-filter already-burned rule used before chance gating.
pub fn filtered_static_severity_raw(
    packet: DamagePacket,
    profile: &DamageProfile,
    kind_index: u32,
    terrain_type: u8,
) -> i32 {
    let filtered = profile.filter(packet);
    if terrain_type & BURNED_TERRAIN_TYPE_BIT != 0 && kind_index != BURNED_SEVERITY_EXCEPTION_KIND {
        filtered / 2
    } else {
        filtered
    }
}

#[cfg(test)]
fn time_zero_actions(target: StaticDamageTarget) -> Vec<StaticDamageAction> {
    let base = program_base_raw(target.cell, target.state.terrain_height_byte);
    let radius = target.state.collision_radius_raw;
    vec![
        StaticDamageAction::Effect18 {
            position_raw: [
                base[0],
                base[1].wrapping_add(three_quarters(radius)),
                base[2].wrapping_sub(quarter(radius)),
            ],
        },
        fixed_sound(68, base),
    ]
}

#[cfg(test)]
fn second_effect_actions(cell: [u8; 2], state: StaticDamageTargetState) -> [StaticDamageAction; 2] {
    let base = program_base_raw(cell, state.terrain_height_byte);
    [
        StaticDamageAction::Effect18 {
            position_raw: [
                base[0],
                base[1].wrapping_add(half(state.collision_radius_raw)),
                base[2].wrapping_sub(quarter(state.collision_radius_raw)),
            ],
        },
        fixed_sound(90, base),
    ]
}

fn fixed_sound(sound_id: u16, position_raw: [i16; 3]) -> StaticDamageAction {
    StaticDamageAction::FixedSound {
        sound_id,
        position_raw,
        gain_q16: 0x1_0000,
        rate_q16: 0x1_0000,
    }
}

fn program_base_raw(cell: [u8; 2], terrain_height_byte: u8) -> [i16; 3] {
    [
        ((u16::from(cell[0]) << 8).wrapping_add(0x80)) as i16,
        i16::from(terrain_height_byte as i8).wrapping_mul(0x20),
        (u16::from(cell[1]) << 8) as i16,
    ]
}

fn quarter(radius_raw: u16) -> i16 {
    (radius_raw / 4) as i16
}

fn half(radius_raw: u16) -> i16 {
    (radius_raw / 2) as i16
}

fn three_quarters(radius_raw: u16) -> i16 {
    (u32::from(radius_raw).wrapping_mul(3) / 4) as i16
}

/// OR terrain-state bits without disturbing material or other model-slot bits.
/// Returns whether the byte changed; repeated application is idempotent.
pub fn apply_terrain_type_bits(terrain: &mut TerrainGrid, cell: [u8; 2], bits: u8) -> bool {
    let index = usize::from(cell[0]) * GRID_SIZE + usize::from(cell[1]);
    let terrain_type = &mut terrain
        .cells
        .get_mut(index)
        .expect("complete 256x256 Section-10 terrain")
        .terrain_type;
    let previous = *terrain_type;
    *terrain_type |= bits;
    *terrain_type != previous
}

/// One static-object hit found by the radial scan.
///
/// Retail does not apply the template's impulse to static objects, so only the
/// scaled damage packet and preserved trailing words are exposed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticRadialHit {
    pub target: StaticDamageTarget,
    pub position_raw: [i16; 3],
    pub distance_raw: i32,
    pub packet: DamagePacket,
    pub trailing_raw: [i32; 2],
}

/// Scan static cells in retail X-outer/Z-inner order around a radial origin.
///
/// The result contains only the static phase of `FUN_004566E0`; callers must
/// deliver it before a separately implemented dynamic-entity phase. No
/// dynamic traversal or presentation side effect is inferred here.
pub fn scan_static_radial(
    terrain: &TerrainGrid,
    origin_raw: [i16; 3],
    template: RadialDamageTemplate,
    mut lookup_static: impl FnMut([u8; 2]) -> Option<StaticDamageTargetState>,
) -> Vec<StaticRadialHit> {
    let mut hits = Vec::new();
    let outer_radius_raw = i32::from(template.outer_radius_raw);
    if outer_radius_raw < 0 {
        return hits;
    }

    // FUN_00427F20 starts both signed-short axes at -outer and advances by one
    // cell while offset <= outer. Kind 27's 0x0DAC radius consequently visits
    // 28x28 candidates rather than a radius-independent five-cell square.
    for offset_x in (-outer_radius_raw..=outer_radius_raw).step_by(0x100) {
        for offset_z in (-outer_radius_raw..=outer_radius_raw).step_by(0x100) {
            let offset_x = offset_x as i16;
            let offset_z = offset_z as i16;
            let x_raw = origin_raw[0].wrapping_add(offset_x);
            let z_raw = origin_raw[2].wrapping_add(offset_z);
            let cell = [(x_raw as u16 >> 8) as u8, (z_raw as u16 >> 8) as u8];
            let Some(state) = lookup_static(cell) else {
                continue;
            };

            let terrain_y_raw = four_corner_terrain_y_raw(terrain, cell);
            let position_raw = [
                x_raw,
                static_radial_vertical_position_raw(
                    terrain_y_raw,
                    origin_raw[1],
                    state.collision_radius_raw,
                ),
                z_raw,
            ];
            let Some(scaled) = scale_radial_damage(template, origin_raw, position_raw, false)
            else {
                continue;
            };
            hits.push(StaticRadialHit {
                target: StaticDamageTarget { cell, state },
                position_raw,
                distance_raw: scaled.distance_raw,
                packet: scaled.packet,
                trailing_raw: scaled.trailing_raw,
            });
        }
    }

    hits
}

/// Exact signed/wrapping vertical position used for a static radial candidate.
pub fn static_radial_vertical_position_raw(
    terrain_y_raw: i16,
    blast_y_raw: i16,
    collision_radius_raw: u16,
) -> i16 {
    let mut position_y_raw = terrain_y_raw;
    if terrain_y_raw < blast_y_raw {
        let top_raw = terrain_y_raw.wrapping_add(collision_radius_raw as i16);
        position_y_raw = top_raw;
        if blast_y_raw < top_raw {
            position_y_raw = blast_y_raw;
        }
    }
    position_y_raw
}

fn four_corner_terrain_y_raw(terrain: &TerrainGrid, cell: [u8; 2]) -> i16 {
    let x = usize::from(cell[0]);
    let z = usize::from(cell[1]);
    let next_x = (x + 1) & 0xff;
    let next_z = (z + 1) & 0xff;
    let sum = [(x, z), (next_x, z), (x, next_z), (next_x, next_z)]
        .into_iter()
        .map(|(corner_x, corner_z)| {
            let height = terrain
                .cell(corner_x, corner_z)
                .expect("wrapped Section-10 corner")
                .height as i8;
            i32::from(height) << 5
        })
        .sum::<i32>();
    (sum / 4) as i16
}

#[cfg(test)]
mod tests {
    mod ground_program;
    mod immediate_burn;

    use std::cell::Cell;

    use super::*;
    use v2k_formats::terrain::TerrainCell;

    fn target(
        cell: [u8; 2],
        kind_index: u32,
        terrain_type: u8,
        terrain_height: i8,
        collision_radius_raw: u16,
    ) -> StaticDamageTarget {
        StaticDamageTarget {
            cell,
            state: StaticDamageTargetState {
                kind_index,
                terrain_type,
                terrain_height_byte: terrain_height as u8,
                collision_radius_raw,
                effect_extent_raw: collision_radius_raw,
            },
        }
    }

    fn flat_terrain(height: i8, terrain_type: u8) -> TerrainGrid {
        TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: height as u8,
                    attribute: 0,
                    terrain_type,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn cell_mut(terrain: &mut TerrainGrid, cell: [u8; 2]) -> &mut TerrainCell {
        &mut terrain.cells[usize::from(cell[0]) * GRID_SIZE + usize::from(cell[1])]
    }

    #[test]
    fn live_kind0_catalog_path_is_exact_and_closed() {
        let descriptor = static_kind_descriptor(KIND_0_STATIC_OBJECT).expect("kind 0 descriptor");
        assert_eq!(descriptor.damage_profile_va(), KIND_0_DAMAGE_PROFILE_VA);
        assert_eq!(descriptor.chance_gate_raw(), KIND_0_CHANCE_GATE_RAW);
        assert_eq!(descriptor.timed_program_va(), KIND_0_TIMED_PROGRAM_VA);

        let program = static_program(descriptor.timed_program_va()).expect("kind 0 program");
        assert_eq!(
            program
                .records()
                .iter()
                .map(|record| {
                    (
                        record.delay_us(),
                        record.opcode_raw(),
                        record.argument_raw(),
                    )
                })
                .collect::<Vec<_>>(),
            [
                (0, 1, 1),
                (0, 4, 0x12),
                (0, 8, 0x44),
                (500_000, 3, 0x12),
                (0, 8, 0x5A),
                (100_000, 10, 0),
                (0, 12, KIND_0_RADIAL_TEMPLATE_VA),
                (0, 0, 0),
            ]
        );

        let catalog =
            admitted_static_catalog(KIND_0_STATIC_OBJECT).expect("bounded kind 0 closure");
        assert_eq!(catalog.profile, KIND_0_DAMAGE_PROFILE);
        assert_eq!(catalog.chance_gate_raw, KIND_0_CHANCE_GATE_RAW);
        assert_eq!(catalog.program_va, KIND_0_TIMED_PROGRAM_VA);
        assert_eq!(
            static_radial_template(KIND_0_RADIAL_TEMPLATE_VA)
                .expect("kind 0 radial")
                .decoded(),
            KIND_0_RADIAL_TEMPLATE
        );

        let mut scheduler = StaticDamageScheduler::new();
        let mut rng = || panic!("unsupported kinds must not draw RNG");
        assert_eq!(
            scheduler.submit_hit(
                target([1, 1], 5, 0, 0, 100),
                DamagePacket::collision(5_000),
                &mut rng,
            ),
            StaticDamageOutcome::UnsupportedKind { kind_index: 5 }
        );
        assert_eq!(scheduler.active_program_count(), 0);
    }

    #[test]
    fn simple_destruction_catalog_paths_are_exact_and_closed() {
        for (kind, profile_va, chance_gate_raw) in [
            (KIND_1_STATIC_OBJECT, KIND_1_DAMAGE_PROFILE_VA, 1_000),
            (KIND_8_STATIC_OBJECT, KIND_8_DAMAGE_PROFILE_VA, 2_000),
        ] {
            let descriptor = static_kind_descriptor(kind).expect("admitted static descriptor");
            assert_eq!(descriptor.damage_profile_va(), profile_va);
            assert_eq!(descriptor.chance_gate_raw(), chance_gate_raw);
            assert_eq!(descriptor.timed_program_va(), SIMPLE_DESTRUCTION_PROGRAM_VA);

            let catalog = admitted_static_catalog(kind).expect("bounded destruction closure");
            assert_eq!(
                catalog.profile,
                static_damage_profile(profile_va)
                    .expect("catalogued static profile")
                    .decoded()
            );
            assert_eq!(catalog.chance_gate_raw, chance_gate_raw);
            assert_eq!(catalog.program_va, SIMPLE_DESTRUCTION_PROGRAM_VA);
        }

        assert_eq!(
            static_program(SIMPLE_DESTRUCTION_PROGRAM_VA)
                .expect("simple destruction program")
                .records()
                .iter()
                .map(|record| {
                    (
                        record.delay_us(),
                        record.opcode_raw(),
                        record.argument_raw(),
                    )
                })
                .collect::<Vec<_>>(),
            [(0, 4, 0x12), (0, 8, 90), (500_000, 10, 0), (0, 0, 0),]
        );
    }

    #[test]
    fn kinds_1_and_8_run_the_exact_simple_destruction_timeline() {
        for (kind, impact_raw) in [(KIND_1_STATIC_OBJECT, 5_000), (KIND_8_STATIC_OBJECT, 6_000)] {
            let static_target = target([12, 34], kind, 0, 5, 1_200);
            let base =
                program_base_raw(static_target.cell, static_target.state.terrain_height_byte);
            let three_quarter_height = [
                base[0],
                base[1].wrapping_add(900),
                base[2].wrapping_sub(300),
            ];
            let mut scheduler = StaticDamageScheduler::new();
            let mut rng = || panic!("gate-equal severity must not draw RNG");

            assert!(matches!(
                scheduler.submit_hit(static_target, DamagePacket::collision(impact_raw), &mut rng),
                StaticDamageOutcome::Started { sample: None, .. }
            ));
            assert_eq!(
                scheduler.advance(0, |_| Some(static_target.state)),
                vec![
                    StaticDamageAction::Effect18 {
                        position_raw: three_quarter_height,
                    },
                    fixed_sound(90, base),
                ]
            );
            assert!(scheduler
                .advance(499_999, |_| Some(static_target.state))
                .is_empty());
            assert_eq!(
                scheduler.advance(1, |_| Some(static_target.state)),
                vec![StaticDamageAction::SetBurned {
                    cell: static_target.cell,
                }]
            );
            assert_eq!(scheduler.active_program_count(), 0);
        }
    }

    #[test]
    fn kind_28_reuses_kind_0_program_with_its_own_profile_and_gate() {
        let descriptor = static_kind_descriptor(KIND_28_STATIC_OBJECT).expect("kind 28 descriptor");
        assert_eq!(descriptor.damage_profile_va(), KIND_11_DAMAGE_PROFILE_VA);
        assert_eq!(descriptor.chance_gate_raw(), 100_000);
        assert_eq!(descriptor.timed_program_va(), KIND_0_TIMED_PROGRAM_VA);
        let catalog = admitted_static_catalog(KIND_28_STATIC_OBJECT)
            .expect("kind 28 closes over the admitted kind-0 program");
        assert_eq!(catalog.chance_gate_raw, 100_000);
        assert_eq!(catalog.program_va, KIND_0_TIMED_PROGRAM_VA);

        let static_target = target([12, 34], KIND_28_STATIC_OBJECT, 0, 5, 1_200);
        let mut scheduler = StaticDamageScheduler::new();
        let mut rng = || panic!("gate-equal severity must not draw RNG");
        assert!(matches!(
            scheduler.submit_hit(static_target, DamagePacket::collision(120_000), &mut rng),
            StaticDamageOutcome::Started {
                severity_raw: 100_000,
                sample: None,
            }
        ));
        assert_eq!(
            scheduler.advance(0, |_| Some(static_target.state)),
            time_zero_actions(static_target)
        );
        assert_eq!(
            scheduler.advance(500_000, |_| Some(static_target.state)),
            second_effect_actions(static_target.cell, static_target.state)
        );
        assert_eq!(
            scheduler.advance(100_000, |_| Some(static_target.state)),
            vec![
                StaticDamageAction::SetBurned {
                    cell: static_target.cell,
                },
                StaticDamageAction::Radial {
                    source_cell: static_target.cell,
                    origin_raw: program_base_raw(
                        static_target.cell,
                        static_target.state.terrain_height_byte,
                    ),
                    template: KIND_0_RADIAL_TEMPLATE,
                },
            ]
        );
        assert_eq!(scheduler.active_program_count(), 0);
    }

    #[test]
    fn kind_9_runs_its_exact_sound_scatter_and_burn_program() {
        let descriptor = static_kind_descriptor(KIND_9_STATIC_OBJECT).expect("kind 9 descriptor");
        assert_eq!(descriptor.damage_profile_va(), KIND_9_DAMAGE_PROFILE_VA);
        assert_eq!(descriptor.chance_gate_raw(), 1_000);
        assert_eq!(descriptor.timed_program_va(), KIND_9_TIMED_PROGRAM_VA);
        assert_eq!(
            static_damage_profile(KIND_9_DAMAGE_PROFILE_VA)
                .expect("kind 9 profile")
                .decoded(),
            DamageProfile {
                thresholds_raw: [0, 4_000, 1_000, 500, 2_000, 0, 0],
                multipliers_q8: [0, 512, 512, 512, 128, 0, 0],
            }
        );
        assert_eq!(
            STATIC_OPCODE_13_BLOB.source_va(),
            KIND_9_OPCODE_13_PAYLOAD_VA
        );
        assert_eq!(
            *STATIC_OPCODE_13_BLOB.raw_words(),
            KIND_9_OPCODE_13_RAW_WORDS
        );
        assert_eq!(
            static_program(KIND_9_TIMED_PROGRAM_VA)
                .expect("kind 9 program")
                .records()
                .iter()
                .map(|record| {
                    (
                        record.delay_us(),
                        record.opcode_raw(),
                        record.argument_raw(),
                    )
                })
                .collect::<Vec<_>>(),
            [
                (0, 1, 1),
                (0, 8, 59),
                (0, 13, KIND_9_OPCODE_13_PAYLOAD_VA),
                (0, 10, 0),
                (0, 0, 0),
            ]
        );
        assert!(
            decode_admitted_static_record(StaticProgramRecord::from_raw_words([
                0,
                13,
                KIND_9_OPCODE_13_PAYLOAD_VA.wrapping_add(4),
            ]))
            .is_none()
        );

        let static_target = target([12, 34], KIND_9_STATIC_OBJECT, 0, 5, 1_200);
        let base = program_base_raw(static_target.cell, static_target.state.terrain_height_byte);
        let mut scheduler = StaticDamageScheduler::new();
        let mut rng = || panic!("gate-exceeding severity must not draw RNG");
        assert_eq!(
            scheduler.submit_hit(static_target, DamagePacket::collision(5_000), &mut rng),
            StaticDamageOutcome::Started {
                severity_raw: 2_000,
                sample: None,
            }
        );
        assert_eq!(
            scheduler.advance(0, |_| Some(static_target.state)),
            vec![
                fixed_sound(59, base),
                StaticDamageAction::BallisticScatter79 {
                    position_raw: [
                        base[0],
                        base[1].wrapping_add(600),
                        base[2].wrapping_sub(300),
                    ],
                },
                StaticDamageAction::SetBurned {
                    cell: static_target.cell,
                },
            ]
        );
        assert_eq!(scheduler.active_program_count(), 0);
    }

    #[test]
    fn kind_9_preserves_chance_burned_and_require_static_gates() {
        let static_target = target([12, 34], KIND_9_STATIC_OBJECT, 0, 5, 1_200);
        let mut scheduler = StaticDamageScheduler::new();
        let mut draws = 0;
        assert_eq!(
            scheduler.submit_hit(static_target, DamagePacket::collision(4_250), &mut || {
                draws += 1;
                0
            },),
            StaticDamageOutcome::Started {
                severity_raw: 500,
                sample: Some(StaticDamageChanceSample {
                    cutoff: 32_767,
                    roll: 0,
                }),
            }
        );
        assert_eq!(draws, 1);
        assert!(
            scheduler.advance(0, |_| None).is_empty(),
            "opcode 1 must cancel the complete time-zero batch when the static object disappeared"
        );
        assert_eq!(scheduler.active_program_count(), 0);

        let burned_target = StaticDamageTarget {
            state: StaticDamageTargetState {
                terrain_type: BURNED_TERRAIN_TYPE_BIT,
                ..static_target.state
            },
            ..static_target
        };
        let mut no_rng = || panic!("gate-equal burned severity must not draw RNG");
        assert_eq!(
            scheduler.submit_hit(burned_target, DamagePacket::collision(5_000), &mut no_rng,),
            StaticDamageOutcome::BurnedIgnored {
                severity_raw: 1_000,
                sample: None,
            }
        );
    }

    #[test]
    fn kind_29_runs_ordinary_class79_then_shared_layered_destruction() {
        let descriptor = static_kind_descriptor(KIND_29_STATIC_OBJECT).expect("kind 29 descriptor");
        assert_eq!(descriptor.damage_profile_va(), KIND_29_DAMAGE_PROFILE_VA);
        assert_eq!(descriptor.chance_gate_raw(), 2_000);
        assert_eq!(descriptor.timed_program_va(), KIND_29_TIMED_PROGRAM_VA);
        assert_eq!(
            static_damage_profile(KIND_29_DAMAGE_PROFILE_VA)
                .expect("kind 29 profile")
                .decoded(),
            DamageProfile {
                thresholds_raw: [0, 3_000, 3_000, 500, 1_000, 0, 0],
                multipliers_q8: [0, 256, 256, 512, 128, 0, 0],
            }
        );
        assert_eq!(
            static_program(KIND_29_TIMED_PROGRAM_VA)
                .expect("kind 29 program")
                .records()
                .iter()
                .map(|record| {
                    (
                        record.delay_us(),
                        record.opcode_raw(),
                        record.argument_raw(),
                    )
                })
                .collect::<Vec<_>>(),
            [
                (0, 1, 1),
                (0, 3, 79),
                (0, 8, 90),
                (500_000, 3, 18),
                (0, 10, 0),
                (0, 12, LAYERED_DESTRUCTION_RADIAL_TEMPLATE_VA),
                (0, 0, 0),
            ]
        );
        assert!(
            decode_admitted_static_record(StaticProgramRecord::from_raw_words([0, 4, 79]))
                .is_none(),
            "only kind 29's recovered half-height class-79 form is admitted"
        );

        let catalog =
            admitted_static_catalog(KIND_29_STATIC_OBJECT).expect("bounded kind 29 closure");
        assert_eq!(catalog.chance_gate_raw, 2_000);
        assert_eq!(catalog.program_va, KIND_29_TIMED_PROGRAM_VA);

        let static_target = target([12, 34], KIND_29_STATIC_OBJECT, 0, 5, 1_200);
        let base = program_base_raw(static_target.cell, static_target.state.terrain_height_byte);
        let half_height = [
            base[0],
            base[1].wrapping_add(600),
            base[2].wrapping_sub(300),
        ];
        let mut scheduler = StaticDamageScheduler::new();
        let mut rng = || panic!("gate-equal severity must not draw RNG");
        assert_eq!(
            scheduler.submit_hit(static_target, DamagePacket::collision(5_000), &mut rng),
            StaticDamageOutcome::Started {
                severity_raw: 2_000,
                sample: None,
            }
        );
        assert_eq!(
            scheduler.advance(0, |_| Some(static_target.state)),
            vec![
                StaticDamageAction::OrdinaryEffect79 {
                    position_raw: half_height,
                },
                fixed_sound(90, base),
            ]
        );
        assert_eq!(
            scheduler.advance(500_000, |_| Some(static_target.state)),
            vec![
                StaticDamageAction::Effect18 {
                    position_raw: half_height,
                },
                StaticDamageAction::SetBurned {
                    cell: static_target.cell,
                },
                StaticDamageAction::Radial {
                    source_cell: static_target.cell,
                    origin_raw: base,
                    template: static_radial_template(LAYERED_DESTRUCTION_RADIAL_TEMPLATE_VA)
                        .expect("kind 29 radial")
                        .decoded(),
                },
            ]
        );
        assert_eq!(scheduler.active_program_count(), 0);
    }

    #[test]
    fn kind_27_runs_the_exact_repeated_effect_focus_request_and_radial_program() {
        let descriptor = static_kind_descriptor(KIND_27_STATIC_OBJECT).expect("kind 27 descriptor");
        assert_eq!(descriptor.damage_profile_va(), KIND_27_DAMAGE_PROFILE_VA);
        assert_eq!(descriptor.chance_gate_raw(), 500);
        assert_eq!(descriptor.timed_program_va(), KIND_27_TIMED_PROGRAM_VA);
        assert_eq!(descriptor.field_0c_raw(), KIND_27_AUXILIARY_VA);
        assert_eq!(
            static_damage_profile(KIND_27_DAMAGE_PROFILE_VA)
                .expect("kind 27 profile")
                .decoded(),
            DamageProfile {
                thresholds_raw: [0, 4_000, 500, 0, 0, 0, 0],
                multipliers_q8: [0, 256, 512, 1_024, 0, 0, 0],
            }
        );
        assert_eq!(
            static_program(KIND_27_TIMED_PROGRAM_VA)
                .expect("kind 27 program")
                .records()
                .iter()
                .map(|record| {
                    (
                        record.delay_us(),
                        record.opcode_raw(),
                        record.argument_raw(),
                    )
                })
                .collect::<Vec<_>>(),
            [
                (0, 1, 1),
                (0, 4, 18),
                (0, 8, 90),
                (500_000, 4, 18),
                (0, 8, 90),
                (500_000, 4, 18),
                (0, 8, 90),
                (500_000, 4, 18),
                (0, 8, 90),
                (500_000, 6, KIND_27_FULL_FRAME_MAX_DISTANCE_RAW as u32),
                (0, 8, 62),
                (100_000, 6, KIND_27_FULL_FRAME_MAX_DISTANCE_RAW as u32),
                (0, 10, 0),
                (0, 12, KIND_27_RADIAL_TEMPLATE_VA),
                (0, 0, 0),
            ]
        );
        assert_eq!(
            static_radial_template(KIND_27_RADIAL_TEMPLATE_VA)
                .expect("kind 27 radial")
                .decoded(),
            KIND_27_RADIAL_TEMPLATE
        );
        assert!(
            decode_admitted_static_record(StaticProgramRecord::from_raw_words([
                0,
                6,
                KIND_27_FULL_FRAME_MAX_DISTANCE_RAW as u32,
            ]))
            .is_some()
        );
        assert!(
            decode_admitted_static_record(StaticProgramRecord::from_raw_words([0, 6, 0x1401,]))
                .is_none()
        );

        let catalog =
            admitted_static_catalog(KIND_27_STATIC_OBJECT).expect("bounded kind 27 closure");
        assert_eq!(catalog.chance_gate_raw, 500);
        assert_eq!(catalog.program_va, KIND_27_TIMED_PROGRAM_VA);

        let static_target = target([12, 34], KIND_27_STATIC_OBJECT, 0, 5, 1_200);
        let base = program_base_raw(static_target.cell, static_target.state.terrain_height_byte);
        let three_quarter_height = [
            base[0],
            base[1].wrapping_add(900),
            base[2].wrapping_sub(300),
        ];
        let mut scheduler = StaticDamageScheduler::new();
        let mut rng = || panic!("gate-equal severity must not draw RNG");
        assert_eq!(
            scheduler.submit_hit(static_target, DamagePacket::collision(4_500), &mut rng,),
            StaticDamageOutcome::Started {
                severity_raw: 500,
                sample: None,
            }
        );
        let repeated_effect = vec![
            StaticDamageAction::Effect18 {
                position_raw: three_quarter_height,
            },
            fixed_sound(90, base),
        ];
        assert_eq!(
            scheduler.advance(0, |_| Some(static_target.state)),
            repeated_effect
        );
        for _ in 0..3 {
            assert_eq!(
                scheduler.advance(500_000, |_| Some(static_target.state)),
                repeated_effect
            );
        }
        assert_eq!(
            scheduler.advance(500_000, |_| Some(static_target.state)),
            vec![
                StaticDamageAction::RequestFullFrameSequenceWithin {
                    position_raw: base,
                    max_distance_raw: KIND_27_FULL_FRAME_MAX_DISTANCE_RAW,
                },
                fixed_sound(62, base),
            ]
        );
        assert_eq!(
            scheduler.advance(100_000, |_| Some(static_target.state)),
            vec![
                StaticDamageAction::RequestFullFrameSequenceWithin {
                    position_raw: base,
                    max_distance_raw: KIND_27_FULL_FRAME_MAX_DISTANCE_RAW,
                },
                StaticDamageAction::SetBurned {
                    cell: static_target.cell,
                },
                StaticDamageAction::Radial {
                    source_cell: static_target.cell,
                    origin_raw: base,
                    template: KIND_27_RADIAL_TEMPLATE,
                },
            ]
        );
        assert_eq!(scheduler.active_program_count(), 0);
    }

    #[test]
    fn kind_admission_query_uses_the_validated_runtime_closure() {
        for kind in [
            KIND_0_STATIC_OBJECT,
            KIND_1_STATIC_OBJECT,
            KIND_2_STATIC_OBJECT,
            KIND_3_STATIC_OBJECT,
            KIND_8_STATIC_OBJECT,
            KIND_9_STATIC_OBJECT,
            KIND_10_STATIC_OBJECT,
            KIND_11_STATIC_OBJECT,
            KIND_27_STATIC_OBJECT,
            KIND_28_STATIC_OBJECT,
            KIND_29_STATIC_OBJECT,
        ] {
            assert!(static_damage_kind_is_admitted(kind), "kind {kind}");
        }
        for kind in [5, u32::MAX] {
            assert!(!static_damage_kind_is_admitted(kind), "kind {kind}");
        }
    }

    #[test]
    fn layered_destruction_catalog_paths_are_exact_and_closed() {
        for (kind, profile_va, chance_gate_raw, program_va) in [
            (
                KIND_3_STATIC_OBJECT,
                KIND_3_DAMAGE_PROFILE_VA,
                4_000,
                LAYERED_DESTRUCTION_PROGRAM_VA,
            ),
            (
                KIND_10_STATIC_OBJECT,
                KIND_10_DAMAGE_PROFILE_VA,
                4_000,
                LAYERED_DESTRUCTION_NO_RADIAL_PROGRAM_VA,
            ),
            (
                KIND_11_STATIC_OBJECT,
                KIND_11_DAMAGE_PROFILE_VA,
                100_000,
                LAYERED_DESTRUCTION_PROGRAM_VA,
            ),
        ] {
            let descriptor = static_kind_descriptor(kind).expect("admitted static descriptor");
            assert_eq!(descriptor.damage_profile_va(), profile_va);
            assert_eq!(descriptor.chance_gate_raw(), chance_gate_raw);
            assert_eq!(descriptor.timed_program_va(), program_va);

            let catalog = admitted_static_catalog(kind).expect("bounded destruction closure");
            assert_eq!(
                catalog.profile,
                static_damage_profile(profile_va)
                    .expect("catalogued static profile")
                    .decoded()
            );
            assert_eq!(catalog.chance_gate_raw, chance_gate_raw);
            assert_eq!(catalog.program_va, program_va);
        }

        assert_eq!(
            static_program(LAYERED_DESTRUCTION_PROGRAM_VA)
                .expect("layered destruction program")
                .records()
                .iter()
                .map(|record| {
                    (
                        record.delay_us(),
                        record.opcode_raw(),
                        record.argument_raw(),
                    )
                })
                .collect::<Vec<_>>(),
            [
                (0, 1, 1),
                (0, 3, 0x1E),
                (0, 8, 90),
                (500_000, 3, 0x12),
                (0, 10, 0),
                (0, 12, LAYERED_DESTRUCTION_RADIAL_TEMPLATE_VA),
                (0, 0, 0),
            ]
        );
        assert_eq!(
            static_program(LAYERED_DESTRUCTION_NO_RADIAL_PROGRAM_VA)
                .expect("kind-10 destruction program")
                .records()
                .iter()
                .map(|record| {
                    (
                        record.delay_us(),
                        record.opcode_raw(),
                        record.argument_raw(),
                    )
                })
                .collect::<Vec<_>>(),
            [
                (0, 1, 1),
                (0, 3, 0x1E),
                (0, 8, 90),
                (500_000, 3, 0x12),
                (0, 10, 0),
                (0, 0, 0),
            ]
        );
        assert_eq!(
            static_radial_template(LAYERED_DESTRUCTION_RADIAL_TEMPLATE_VA)
                .expect("layered destruction radial")
                .decoded(),
            RadialDamageTemplate {
                inner_radius_raw: 512,
                outer_radius_raw: 1_024,
                impulse_raw: 2_000,
                packet: DamagePacket {
                    channels: [1, 3],
                    amounts_raw: [2_000, 1_000],
                },
                trailing_raw: [-1, 0],
            }
        );
    }

    #[test]
    fn kinds_3_and_11_run_the_exact_layered_destruction_timeline() {
        let radial = static_radial_template(LAYERED_DESTRUCTION_RADIAL_TEMPLATE_VA)
            .expect("layered destruction radial")
            .decoded();
        for (kind, impact_raw) in [
            (KIND_3_STATIC_OBJECT, 10_000),
            (KIND_11_STATIC_OBJECT, 420_000),
        ] {
            let mut static_target = target([12, 34], kind, 0, 5, 1_200);
            static_target.state.effect_extent_raw = 777;
            let base =
                program_base_raw(static_target.cell, static_target.state.terrain_height_byte);
            let half_height = [
                base[0],
                base[1].wrapping_add(600),
                base[2].wrapping_sub(300),
            ];
            let mut scheduler = StaticDamageScheduler::new();
            let mut rng = || panic!("gate-equal severity must not draw RNG");

            assert!(matches!(
                scheduler.submit_hit(static_target, DamagePacket::collision(impact_raw), &mut rng),
                StaticDamageOutcome::Started { sample: None, .. }
            ));
            assert_eq!(
                scheduler.advance(0, |_| Some(static_target.state)),
                vec![
                    StaticDamageAction::CommonExplosion30 {
                        position_raw: half_height,
                        source_extent_raw: 777,
                    },
                    fixed_sound(90, base),
                ]
            );
            assert!(scheduler
                .advance(499_999, |_| Some(static_target.state))
                .is_empty());
            assert_eq!(
                scheduler.advance(1, |_| Some(static_target.state)),
                vec![
                    StaticDamageAction::Effect18 {
                        position_raw: half_height,
                    },
                    StaticDamageAction::SetBurned {
                        cell: static_target.cell,
                    },
                    StaticDamageAction::Radial {
                        source_cell: static_target.cell,
                        origin_raw: base,
                        template: radial,
                    },
                ]
            );
            assert_eq!(scheduler.active_program_count(), 0);
        }
    }

    #[test]
    fn kind_10_runs_the_non_radial_timeline_and_preserves_extent_provenance() {
        let mut static_target = target([12, 34], KIND_10_STATIC_OBJECT, 0, 5, 1_200);
        static_target.state.effect_extent_raw = 777;
        let base = program_base_raw(static_target.cell, static_target.state.terrain_height_byte);
        let half_height = [
            base[0],
            base[1].wrapping_add(600),
            base[2].wrapping_sub(300),
        ];
        let mut scheduler = StaticDamageScheduler::new();
        let mut rng = || panic!("gate-equal severity must not draw RNG");

        assert!(matches!(
            scheduler.submit_hit(static_target, DamagePacket::collision(10_000), &mut rng),
            StaticDamageOutcome::Started { sample: None, .. }
        ));
        assert_eq!(
            scheduler.advance(0, |_| Some(static_target.state)),
            vec![
                StaticDamageAction::CommonExplosion30 {
                    position_raw: half_height,
                    source_extent_raw: 777,
                },
                fixed_sound(90, base),
            ]
        );
        assert_eq!(
            scheduler.advance(500_000, |_| Some(static_target.state)),
            vec![
                StaticDamageAction::Effect18 {
                    position_raw: half_height,
                },
                StaticDamageAction::SetBurned {
                    cell: static_target.cell,
                },
            ]
        );
        assert_eq!(scheduler.active_program_count(), 0);
    }

    #[test]
    fn burned_kind_10_accepts_after_chance_then_requests_the_immediate_transition() {
        let static_target = target(
            [7, 9],
            KIND_10_STATIC_OBJECT,
            BURNED_TERRAIN_TYPE_BIT,
            0,
            100,
        );
        let mut scheduler = StaticDamageScheduler::new();
        let calls = Cell::new(0);
        let mut rng = || {
            calls.set(calls.get() + 1);
            0
        };

        assert_eq!(
            scheduler.submit_hit(static_target, DamagePacket::collision(9_999), &mut rng),
            StaticDamageOutcome::BurnedKind10Transition {
                cell: static_target.cell,
                severity_raw: 3_999,
                sample: Some(StaticDamageChanceSample {
                    cutoff: 65_518,
                    roll: 0,
                }),
            }
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(scheduler.active_program_count(), 0);
    }

    #[test]
    fn kind0_profile_has_strict_collision_threshold_and_full_radial_channel3() {
        for (impact, severity) in [(4_000, 0), (4_001, 1), (4_999, 999), (5_000, 1_000)] {
            assert_eq!(
                KIND_0_DAMAGE_PROFILE.filter(DamagePacket::collision(impact)),
                severity
            );
        }
        assert_eq!(
            KIND_0_DAMAGE_PROFILE.filter(KIND_0_RADIAL_TEMPLATE.packet),
            2_000
        );
    }

    #[test]
    fn primary_projectile_lands_on_kind0_strict_threshold_without_rng_or_program() {
        use crate::damage::PRIMARY_PROJECTILE_DAMAGE_PACKET;

        assert_eq!(
            KIND_0_DAMAGE_PROFILE.filter(PRIMARY_PROJECTILE_DAMAGE_PACKET),
            0
        );

        let mut scheduler = StaticDamageScheduler::new();
        let calls = Cell::new(0);
        let mut rng = || {
            calls.set(calls.get() + 1);
            panic!("zero severity must not consume the shared retail RNG")
        };
        assert_eq!(
            scheduler.submit_hit(
                target([17, 23], 0, 0, 0, 1_224),
                PRIMARY_PROJECTILE_DAMAGE_PACKET,
                &mut rng,
            ),
            StaticDamageOutcome::NoDamage { severity_raw: 0 }
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(scheduler.active_program_count(), 0);
    }

    #[test]
    fn ballistic_particle_stays_below_kind0_strict_threshold_without_rng_or_program() {
        use crate::damage::BALLISTIC_PARTICLE_DAMAGE_PACKET;

        assert_eq!(
            KIND_0_DAMAGE_PROFILE.filter(BALLISTIC_PARTICLE_DAMAGE_PACKET),
            0
        );

        let mut scheduler = StaticDamageScheduler::new();
        let calls = Cell::new(0);
        let mut rng = || {
            calls.set(calls.get() + 1);
            panic!("zero severity must not consume the shared retail RNG")
        };
        assert_eq!(
            scheduler.submit_hit(
                target([17, 23], 0, 0, 0, 1_224),
                BALLISTIC_PARTICLE_DAMAGE_PACKET,
                &mut rng,
            ),
            StaticDamageOutcome::NoDamage { severity_raw: 0 }
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(scheduler.active_program_count(), 0);
    }

    #[test]
    fn f800_packets_have_exact_admitted_kind_matrix_without_rng() {
        use crate::damage::{BALLISTIC_PARTICLE_DAMAGE_PACKET, PRIMARY_PROJECTILE_DAMAGE_PACKET};

        for (kind_index, expected) in [
            (
                KIND_0_STATIC_OBJECT,
                StaticDamageOutcome::NoDamage { severity_raw: 0 },
            ),
            (
                KIND_1_STATIC_OBJECT,
                StaticDamageOutcome::NoDamage { severity_raw: 0 },
            ),
            (
                KIND_3_STATIC_OBJECT,
                StaticDamageOutcome::NoDamage { severity_raw: 0 },
            ),
            (
                KIND_8_STATIC_OBJECT,
                StaticDamageOutcome::NoDamage { severity_raw: 0 },
            ),
            (
                KIND_9_STATIC_OBJECT,
                StaticDamageOutcome::Started {
                    severity_raw: 2_000,
                    sample: None,
                },
            ),
            (
                KIND_10_STATIC_OBJECT,
                StaticDamageOutcome::NoDamage { severity_raw: 0 },
            ),
            (
                KIND_11_STATIC_OBJECT,
                StaticDamageOutcome::NoDamage { severity_raw: 0 },
            ),
            (
                KIND_27_STATIC_OBJECT,
                StaticDamageOutcome::Started {
                    severity_raw: 3_000,
                    sample: None,
                },
            ),
            (
                KIND_28_STATIC_OBJECT,
                StaticDamageOutcome::NoDamage { severity_raw: 0 },
            ),
            (
                KIND_29_STATIC_OBJECT,
                StaticDamageOutcome::NoDamage { severity_raw: 0 },
            ),
        ] {
            let calls = Cell::new(0);
            let mut rng = || {
                calls.set(calls.get() + 1);
                panic!("exact primary F800 packets never sample admitted static kinds")
            };
            let mut scheduler = StaticDamageScheduler::new();
            assert_eq!(
                scheduler.submit_hit(
                    target([kind_index as u8, 1], kind_index, 0, 0, 1_224),
                    PRIMARY_PROJECTILE_DAMAGE_PACKET,
                    &mut rng,
                ),
                expected,
                "primary kind {kind_index}"
            );
            assert_eq!(calls.get(), 0, "primary kind {kind_index}");
        }

        for kind_index in [
            KIND_0_STATIC_OBJECT,
            KIND_1_STATIC_OBJECT,
            KIND_3_STATIC_OBJECT,
            KIND_8_STATIC_OBJECT,
            KIND_9_STATIC_OBJECT,
            KIND_10_STATIC_OBJECT,
            KIND_11_STATIC_OBJECT,
            KIND_27_STATIC_OBJECT,
            KIND_28_STATIC_OBJECT,
            KIND_29_STATIC_OBJECT,
        ] {
            let calls = Cell::new(0);
            let mut rng = || {
                calls.set(calls.get() + 1);
                panic!("exact ballistic F800 packets never sample admitted static kinds")
            };
            let mut scheduler = StaticDamageScheduler::new();
            assert_eq!(
                scheduler.submit_hit(
                    target([kind_index as u8, 2], kind_index, 0, 0, 1_224),
                    BALLISTIC_PARTICLE_DAMAGE_PACKET,
                    &mut rng,
                ),
                StaticDamageOutcome::NoDamage { severity_raw: 0 },
                "ballistic kind {kind_index}"
            );
            assert_eq!(calls.get(), 0, "ballistic kind {kind_index}");
            assert_eq!(scheduler.active_program_count(), 0);
        }
    }

    #[test]
    fn burned_rule_halves_non_kind10_after_filter_with_signed_truncation() {
        let packet = DamagePacket::collision(5_001);
        assert_eq!(
            filtered_static_severity_raw(packet, &KIND_0_DAMAGE_PROFILE, 0, 0x08),
            500
        );
        assert_eq!(
            filtered_static_severity_raw(packet, &KIND_0_DAMAGE_PROFILE, 10, 0x08),
            1_001
        );
        assert_eq!(
            filtered_static_severity_raw(packet, &KIND_0_DAMAGE_PROFILE, 0, 0),
            1_001
        );
    }

    #[test]
    fn chance_gate_consumes_only_for_positive_subthreshold_severity() {
        let mut scheduler = StaticDamageScheduler::new();
        let calls = Cell::new(0);
        let mut rng = || {
            calls.set(calls.get() + 1);
            65
        };
        let accepted = scheduler.submit_hit(
            target([1, 2], 0, 0, 0, 100),
            DamagePacket::collision(4_001),
            &mut rng,
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(
            accepted,
            StaticDamageOutcome::Started {
                severity_raw: 1,
                sample: Some(StaticDamageChanceSample {
                    cutoff: 65,
                    roll: 65,
                }),
            }
        );

        let no_damage = scheduler.submit_hit(
            target([2, 2], 0, 0, 0, 100),
            DamagePacket::collision(4_000),
            &mut rng,
        );
        let deterministic = scheduler.submit_hit(
            target([3, 2], 0, 0, 0, 100),
            DamagePacket::collision(5_000),
            &mut rng,
        );
        assert_eq!(calls.get(), 1);
        assert!(matches!(
            no_damage,
            StaticDamageOutcome::NoDamage { severity_raw: 0 }
        ));
        assert!(matches!(
            deterministic,
            StaticDamageOutcome::Started {
                severity_raw: 1_000,
                sample: None
            }
        ));
    }

    #[test]
    fn chance_rejection_precedes_dedup_and_accepted_duplicate_still_draws_rng() {
        let mut scheduler = StaticDamageScheduler::new();
        let static_target = target([7, 9], 0, 0, 0, 100);
        let packet = DamagePacket::collision(4_999);
        let mut rolls = [0, u16::MAX, 1].into_iter();
        let mut rng = || rolls.next().expect("three chance draws");

        assert!(matches!(
            scheduler.submit_hit(static_target, packet, &mut rng),
            StaticDamageOutcome::Started { .. }
        ));
        assert!(matches!(
            scheduler.submit_hit(static_target, packet, &mut rng),
            StaticDamageOutcome::ChanceRejected { .. }
        ));
        assert!(matches!(
            scheduler.submit_hit(static_target, packet, &mut rng),
            StaticDamageOutcome::Duplicate { .. }
        ));
        assert!(rolls.next().is_none());
    }

    #[test]
    fn burned_kind0_filters_and_rolls_but_never_starts_or_deduplicates() {
        let static_target = target([7, 9], 0, BURNED_TERRAIN_TYPE_BIT, 0, 100);
        let mut scheduler = StaticDamageScheduler::new();
        let calls = Cell::new(0);
        let mut rng = || {
            calls.set(calls.get() + 1);
            0
        };

        // Filtering gives 1,998, then the already-burned rule halves it to
        // 999. Retail consumes that chance draw before its burned-state branch.
        assert_eq!(
            scheduler.submit_hit(static_target, DamagePacket::collision(5_998), &mut rng,),
            StaticDamageOutcome::BurnedIgnored {
                severity_raw: 999,
                sample: Some(StaticDamageChanceSample {
                    cutoff: 65_469,
                    roll: 0,
                }),
            }
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(scheduler.active_program_count(), 0);

        // A deterministic burned hit performs the same no-op without drawing.
        assert_eq!(
            scheduler.submit_hit(static_target, DamagePacket::collision(6_000), &mut rng,),
            StaticDamageOutcome::BurnedIgnored {
                severity_raw: 1_000,
                sample: None,
            }
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(scheduler.active_program_count(), 0);
    }

    #[test]
    fn time_zero_positions_and_word_wrap_match_program_arithmetic() {
        let mut scheduler = StaticDamageScheduler::new();
        let mut rng = || panic!("deterministic severity must not draw RNG");
        let submitted = scheduler.submit_hit(
            target([255, 128], 0, 0, -128, 0xfffe),
            DamagePacket::collision(5_000),
            &mut rng,
        );
        assert!(matches!(submitted, StaticDamageOutcome::Started { .. }));
        let base = [-128, -4_096, i16::MIN];
        let actions = scheduler.advance(0, |_| Some(target([255, 128], 0, 0, -128, 0xfffe).state));
        assert_eq!(
            actions,
            vec![
                StaticDamageAction::Effect18 {
                    position_raw: [
                        base[0],
                        base[1].wrapping_add(three_quarters(0xfffe)),
                        base[2].wrapping_sub(quarter(0xfffe)),
                    ],
                },
                fixed_sound(68, base),
            ]
        );
    }

    #[test]
    fn fifo_schedule_uses_equality_and_burn_precedes_radial() {
        let first = target([1, 2], 0, 0, 3, 1_224);
        let second = target([4, 5], 0, 0, -2, 720);
        let mut scheduler = StaticDamageScheduler::new();
        let mut rng = || panic!("deterministic severity must not draw RNG");
        scheduler.submit_hit(first, DamagePacket::collision(5_000), &mut rng);
        scheduler.submit_hit(second, DamagePacket::collision(5_000), &mut rng);

        let lookup = |cell| {
            [first, second]
                .into_iter()
                .find(|target| target.cell == cell)
                .map(|target| target.state)
        };
        assert_eq!(
            scheduler.advance(0, lookup),
            vec![
                time_zero_actions(first)[0],
                time_zero_actions(first)[1],
                time_zero_actions(second)[0],
                time_zero_actions(second)[1],
            ]
        );
        assert!(scheduler.advance(499_999, lookup).is_empty());
        let effects = scheduler.advance(1, lookup);
        assert_eq!(
            effects,
            vec![
                second_effect_actions(first.cell, first.state)[0],
                second_effect_actions(first.cell, first.state)[1],
                second_effect_actions(second.cell, second.state)[0],
                second_effect_actions(second.cell, second.state)[1],
            ]
        );
        assert!(scheduler.advance(99_999, lookup).is_empty());
        let ignition = scheduler.advance(1, lookup);
        assert_eq!(
            ignition,
            vec![
                StaticDamageAction::SetBurned { cell: first.cell },
                StaticDamageAction::Radial {
                    source_cell: first.cell,
                    origin_raw: program_base_raw(first.cell, first.state.terrain_height_byte),
                    template: KIND_0_RADIAL_TEMPLATE,
                },
                StaticDamageAction::SetBurned { cell: second.cell },
                StaticDamageAction::Radial {
                    source_cell: second.cell,
                    origin_raw: program_base_raw(second.cell, second.state.terrain_height_byte),
                    template: KIND_0_RADIAL_TEMPLATE,
                },
            ]
        );
        assert_eq!(scheduler.active_program_count(), 0);
    }

    #[test]
    fn one_large_update_uses_one_current_radius_snapshot_for_both_stages() {
        let static_target = target([9, 11], 0, 0, 4, 1_224);
        let current = StaticDamageTargetState {
            collision_radius_raw: 360,
            ..static_target.state
        };
        let mut scheduler = StaticDamageScheduler::new();
        let mut rng = || panic!("deterministic severity must not draw RNG");
        scheduler.submit_hit(static_target, DamagePacket::collision(5_000), &mut rng);
        let mut lookups = 0;
        let actions = scheduler.advance(600_000, |_| {
            lookups += 1;
            Some(current)
        });
        assert_eq!(lookups, 1);
        assert_eq!(
            actions,
            vec![
                StaticDamageAction::Effect18 {
                    position_raw: [2_432, 398, 2_726],
                },
                StaticDamageAction::FixedSound {
                    sound_id: 68,
                    position_raw: [2_432, 128, 2_816],
                    gain_q16: 0x1_0000,
                    rate_q16: 0x1_0000,
                },
                StaticDamageAction::Effect18 {
                    position_raw: [2_432, 308, 2_726],
                },
                StaticDamageAction::FixedSound {
                    sound_id: 90,
                    position_raw: [2_432, 128, 2_816],
                    gain_q16: 0x1_0000,
                    rate_q16: 0x1_0000,
                },
                StaticDamageAction::SetBurned {
                    cell: static_target.cell,
                },
                StaticDamageAction::Radial {
                    source_cell: static_target.cell,
                    origin_raw: [2_432, 128, 2_816],
                    template: KIND_0_RADIAL_TEMPLATE,
                },
            ]
        );
        assert_eq!(scheduler.active_program_count(), 0);
    }

    #[test]
    fn same_frame_parents_retire_in_fifo_radial_return_order() {
        let first = target([20, 21], 0, 0, 0, 100);
        let second = target([22, 23], 0, 0, 0, 100);
        let mut scheduler = StaticDamageScheduler::new();
        let mut rng = || panic!("deterministic severity must not draw RNG");
        scheduler.submit_hit(first, DamagePacket::collision(5_000), &mut rng);
        scheduler.submit_hit(second, DamagePacket::collision(5_000), &mut rng);
        scheduler.begin_advance(600_000);

        let first_batch = scheduler
            .next_node_batch(|cell| {
                [first, second]
                    .into_iter()
                    .find(|target| target.cell == cell)
                    .map(|target| target.state)
            })
            .expect("first FIFO parent");
        assert_eq!(first_batch.source_cell(), first.cell);
        assert!(matches!(
            first_batch.actions().last(),
            Some(StaticDamageAction::Radial { .. })
        ));
        assert!(matches!(
            scheduler.submit_hit(first, DamagePacket::collision(5_000), &mut rng),
            StaticDamageOutcome::Duplicate { .. }
        ));
        assert!(matches!(
            scheduler.submit_hit(second, DamagePacket::collision(5_000), &mut rng),
            StaticDamageOutcome::Duplicate { .. }
        ));

        let (first_token, _) = first_batch.into_parts();
        assert!(scheduler.complete_node_batch(first_token));
        assert!(matches!(
            scheduler.submit_hit(first, DamagePacket::collision(5_000), &mut rng),
            StaticDamageOutcome::Started { .. }
        ));
        assert!(matches!(
            scheduler.submit_hit(second, DamagePacket::collision(5_000), &mut rng),
            StaticDamageOutcome::Duplicate { .. }
        ));

        let second_batch = scheduler
            .next_node_batch(|cell| {
                [first, second]
                    .into_iter()
                    .find(|target| target.cell == cell)
                    .map(|target| target.state)
            })
            .expect("second FIFO parent");
        assert_eq!(second_batch.source_cell(), second.cell);
        let (second_token, _) = second_batch.into_parts();
        assert!(scheduler.complete_node_batch(second_token));
        assert!(scheduler
            .next_node_batch(|cell| {
                [first, second]
                    .into_iter()
                    .find(|target| target.cell == cell)
                    .map(|target| target.state)
            })
            .is_none());
        assert!(matches!(
            scheduler.submit_hit(second, DamagePacket::collision(5_000), &mut rng),
            StaticDamageOutcome::Started { .. }
        ));
    }

    #[test]
    fn each_node_applies_before_the_next_current_static_lookup() {
        let first = target([30, 31], 0, 0, 0, 100);
        let second = target([32, 33], 0, 0, 0, 100);
        let mut scheduler = StaticDamageScheduler::new();
        let mut rng = || panic!("deterministic severity must not draw RNG");
        scheduler.submit_hit(first, DamagePacket::collision(5_000), &mut rng);
        scheduler.submit_hit(second, DamagePacket::collision(5_000), &mut rng);
        scheduler.begin_advance(600_000);

        let mut lookup_order = Vec::new();
        let first_batch = scheduler
            .next_node_batch(|cell| {
                lookup_order.push(cell);
                (cell == first.cell).then_some(first.state)
            })
            .expect("first node must produce its complete batch");
        assert_eq!(lookup_order, [first.cell]);
        assert_eq!(first_batch.source_cell(), first.cell);

        let first_burn_applied = first_batch.actions().iter().any(|action| {
            matches!(
                action,
                StaticDamageAction::SetBurned {
                    cell,
                } if *cell == first.cell
            )
        });
        assert!(first_burn_applied);
        let (first_token, _) = first_batch.into_parts();
        assert!(scheduler.complete_node_batch(first_token));

        let second_batch = scheduler
            .next_node_batch(|cell| {
                lookup_order.push(cell);
                (first_burn_applied && cell == second.cell).then_some(second.state)
            })
            .expect("second lookup must observe the first node's applied mutation");
        assert_eq!(lookup_order, [first.cell, second.cell]);
        assert_eq!(second_batch.source_cell(), second.cell);
        let (second_token, _) = second_batch.into_parts();
        assert!(scheduler.complete_node_batch(second_token));
        assert!(scheduler.next_node_batch(|_| unreachable!()).is_none());
    }

    #[test]
    fn original_fifo_membership_excludes_radial_children_from_the_current_pass() {
        let parent = target([1, 1], 0, 0, 0, 100);
        let child = target([2, 1], 0, 0, 0, 100);
        let mut scheduler = StaticDamageScheduler::new();
        let mut rng = || panic!("deterministic severity must not draw RNG");
        scheduler.submit_hit(parent, DamagePacket::collision(5_000), &mut rng);
        scheduler.begin_advance(600_000);
        let parent_batch = scheduler
            .next_node_batch(|cell| (cell == parent.cell).then_some(parent.state))
            .expect("parent radial batch");
        assert!(matches!(
            parent_batch.actions().last(),
            Some(StaticDamageAction::Radial { .. })
        ));

        // This submission models the radial callback while the source node is
        // still registered. The child appends to the live list but was not in
        // begin_advance's saved FIFO membership.
        assert!(matches!(
            scheduler.submit_hit(parent, DamagePacket::collision(5_000), &mut rng),
            StaticDamageOutcome::Duplicate { .. }
        ));
        assert!(matches!(
            scheduler.submit_hit(child, DamagePacket::collision(5_000), &mut rng),
            StaticDamageOutcome::Started { .. }
        ));
        let (parent_token, _) = parent_batch.into_parts();
        assert!(scheduler.complete_node_batch(parent_token));
        assert!(scheduler.next_node_batch(|_| unreachable!()).is_none());

        assert_eq!(
            scheduler.advance(499_999, |cell| {
                (cell == child.cell).then_some(child.state)
            }),
            time_zero_actions(child)
        );
        assert_eq!(
            scheduler.advance(1, |cell| (cell == child.cell).then_some(child.state)),
            second_effect_actions(child.cell, child.state)
        );
    }

    #[test]
    fn require_static_cancels_and_releases_the_cell_key() {
        let static_target = target([9, 9], 0, 0, 0, 100);
        let mut scheduler = StaticDamageScheduler::new();
        let mut rng = || panic!("deterministic severity must not draw RNG");
        scheduler.submit_hit(static_target, DamagePacket::collision(5_000), &mut rng);
        assert_eq!(
            scheduler.advance(0, |_| Some(static_target.state)),
            time_zero_actions(static_target)
        );

        let mut lookups = 0;
        assert!(scheduler
            .advance(499_999, |_| {
                lookups += 1;
                None
            })
            .is_empty());
        assert_eq!(lookups, 0, "retail does not reacquire before the deadline");
        assert!(scheduler.contains_cell(static_target.cell));

        assert!(scheduler
            .advance(1, |_| {
                lookups += 1;
                None
            })
            .is_empty());
        assert_eq!(lookups, 1);
        assert!(!scheduler.contains_cell(static_target.cell));
    }

    #[test]
    fn terrain_burn_or_preserves_bits_and_is_idempotent() {
        let mut terrain = flat_terrain(7, 0b1010_0101);
        let cell = [33, 44];
        cell_mut(&mut terrain, cell).attribute = 19;
        assert!(apply_terrain_type_bits(
            &mut terrain,
            cell,
            BURNED_TERRAIN_TYPE_BIT
        ));
        assert_eq!(cell_mut(&mut terrain, cell).terrain_type, 0b1010_1101);
        assert_eq!(cell_mut(&mut terrain, cell).attribute, 19);
        assert_eq!(cell_mut(&mut terrain, cell).height, 7);
        assert!(!apply_terrain_type_bits(
            &mut terrain,
            cell,
            BURNED_TERRAIN_TYPE_BIT
        ));
        assert_eq!(cell_mut(&mut terrain, cell).terrain_type, 0b1010_1101);
    }

    #[test]
    fn radial_distance_wraps_words_sorts_components_and_obeys_boundaries() {
        assert_eq!(
            crate::radial_damage::radial_distance_raw([i16::MAX, 0, 0], [i16::MIN, 0, 0],),
            1
        );
        assert_eq!(
            crate::radial_damage::radial_distance_raw([0; 3], [100, 300, 200]),
            450
        );
        assert_eq!(
            scale_radial_damage(KIND_0_RADIAL_TEMPLATE, [0; 3], [256, 0, 0], false)
                .unwrap()
                .packet,
            KIND_0_RADIAL_TEMPLATE.packet
        );
        assert_eq!(
            scale_radial_damage(KIND_0_RADIAL_TEMPLATE, [0; 3], [384, 0, 0], false)
                .unwrap()
                .packet
                .amounts_raw,
            [500, 500]
        );
        assert!(scale_radial_damage(KIND_0_RADIAL_TEMPLATE, [0; 3], [512, 0, 0], false).is_none());
    }

    #[test]
    fn static_scan_is_x_outer_z_inner_toroidal_and_uses_signed_corner_average() {
        let mut terrain = flat_terrain(0, 0);
        let origin = [-128, 64, -128];
        let expected_cells = [
            [253, 253],
            [253, 254],
            [253, 255],
            [253, 0],
            [253, 1],
            [254, 253],
        ];
        let seam = [255, 255];
        cell_mut(&mut terrain, seam).height = (-8_i8) as u8;
        cell_mut(&mut terrain, [0, 255]).height = 4;
        cell_mut(&mut terrain, [255, 0]).height = 8;
        cell_mut(&mut terrain, [0, 0]).height = (-20_i8) as u8;

        let mut visited = Vec::new();
        let hits = scan_static_radial(&terrain, origin, KIND_0_RADIAL_TEMPLATE, |cell| {
            visited.push(cell);
            Some(StaticDamageTargetState {
                kind_index: 0,
                terrain_type: 0,
                terrain_height_byte: terrain
                    .cell(usize::from(cell[0]), usize::from(cell[1]))
                    .unwrap()
                    .height,
                collision_radius_raw: if cell == seam { 100 } else { 0 },
                effect_extent_raw: 0,
            })
        });
        assert_eq!(&visited[..expected_cells.len()], &expected_cells);

        let seam_hit = hits
            .iter()
            .find(|hit| hit.target.cell == seam)
            .expect("seam cell inside radial range");
        // (-8 + 4 + 8 - 20) * 32 / 4 = -128, then top=-28.
        assert_eq!(seam_hit.position_raw[1], -28);
        assert_eq!(seam_hit.trailing_raw, [-1, 0]);
    }

    #[test]
    fn kind_27_static_scan_uses_its_authored_outer_radius_and_axis_cadence() {
        let terrain = flat_terrain(0, 0);
        let mut visited = Vec::new();
        let hits = scan_static_radial(&terrain, [0; 3], KIND_27_RADIAL_TEMPLATE, |cell| {
            visited.push(cell);
            None
        });

        assert!(hits.is_empty());
        assert_eq!(visited.len(), 28 * 28);
        assert_eq!(visited[0], [242, 242]);
        assert_eq!(visited[1], [242, 243]);
        assert_eq!(visited[28], [243, 242]);
        assert_eq!(visited.last().copied(), Some([13, 13]));
    }

    #[test]
    fn static_vertical_branch_preserves_wrapped_top_comparison() {
        assert_eq!(static_radial_vertical_position_raw(-100, 50, 120), 20);
        assert_eq!(static_radial_vertical_position_raw(-100, 0, 200), 0);
        assert_eq!(static_radial_vertical_position_raw(100, 0, 200), 100);
        assert_eq!(
            static_radial_vertical_position_raw(32_000, 32_100, 1_000),
            32_000_i16.wrapping_add(1_000)
        );
    }
}
