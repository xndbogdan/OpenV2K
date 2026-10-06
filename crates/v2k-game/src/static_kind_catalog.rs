//! Lossless executable-owned static-terrain-object kind data.
//!
//! The supported retail executable
//! (`E9BE7A833612FBA3A5A5AB92A974ECE1A689E4B7E72409D9EE8331380573B4BA`)
//! stores 32 records at `0x004C9E38`, each exactly `0x2C` bytes.  The records
//! point into bounded damage-profile and timed-program corpora immediately
//! preceding the table.  Every dword is retained here even when its semantics
//! are not yet established.
//!
//! This module is data only.  In particular, cataloguing an opcode does not
//! admit its runtime effect.  The scheduler must continue to fail closed for
//! actions whose callees or state ownership have not been recovered.

use crate::damage::{DamagePacket, DamageProfile};
use crate::radial_damage::RadialDamageTemplate;

pub const STATIC_KIND_TABLE_VA: u32 = 0x004C_9E38;
pub const STATIC_KIND_RECORD_STRIDE: usize = 0x2C;
pub const STATIC_KIND_COUNT: usize = 32;
pub const STATIC_KIND_TABLE_END_VA: u32 =
    STATIC_KIND_TABLE_VA + (STATIC_KIND_COUNT * STATIC_KIND_RECORD_STRIDE) as u32;

pub const STATIC_DAMAGE_PROFILE_WORD_COUNT: usize = 14;
pub const STATIC_DAMAGE_PROFILE_BYTE_COUNT: usize = STATIC_DAMAGE_PROFILE_WORD_COUNT * 4;
pub const STATIC_PROGRAM_RECORD_STRIDE: usize = 0x0C;
pub const STATIC_RADIAL_TEMPLATE_WORD_COUNT: usize = 8;

/// One exact executable `0x2C` static-kind record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticKindDescriptor {
    raw_words: [u32; STATIC_KIND_RECORD_STRIDE / 4],
}

impl StaticKindDescriptor {
    pub const fn from_raw_words(raw_words: [u32; STATIC_KIND_RECORD_STRIDE / 4]) -> Self {
        Self { raw_words }
    }

    pub const fn raw_words(&self) -> &[u32; STATIC_KIND_RECORD_STRIDE / 4] {
        &self.raw_words
    }

    pub const fn raw_u32(&self, offset: usize) -> u32 {
        self.raw_words[offset / 4]
    }

    /// Seven-channel damage-profile pointer consumed by `FUN_00427950`.
    pub const fn damage_profile_va(&self) -> u32 {
        self.raw_words[0]
    }

    /// Positive severity/chance gate consumed by `FUN_00427950`.
    pub const fn chance_gate_raw(&self) -> i32 {
        self.raw_words[1] as i32
    }

    /// Optional timed-program pointer consumed by `FUN_00427950`.
    pub const fn timed_program_va(&self) -> u32 {
        self.raw_words[2]
    }

    /// Optional radius/depth pair consumed by the burn-change callback `427760`.
    pub const fn field_0c_raw(&self) -> u32 {
        self.raw_words[3]
    }

    /// `427894` forwards the first two signed dwords to `436E00`.
    /// The following pair at 4C9E30 has no established consumer here.
    pub fn burn_crater(&self) -> Option<StaticBurnCrater> {
        (self.field_0c_raw() == STATIC_KIND_27_AUXILIARY_BLOB.source_va()).then(|| {
            let words = STATIC_KIND_27_AUXILIARY_BLOB.raw_words();
            StaticBurnCrater {
                radius_raw: words[0] as i32,
                depth_height_units: words[1] as i32,
            }
        })
    }

    /// `4278B4..4278E2`: nonzero +14 selects both scatter classes; the
    /// signed +10 / 100 quotient is stored as an unsigned packet word.
    pub fn burn_burst(&self) -> Option<StaticBurnBurst> {
        (self.raw_words[5] != 0).then_some(StaticBurnBurst {
            scatter_count: ((self.raw_words[4] as i32) / 100) as u16,
            particle_class: self.raw_words[5] as u8,
        })
    }

    /// Contact-action mode consumed by `FUN_00427E20`.
    pub const fn contact_mode_raw(&self) -> u32 {
        self.raw_words[8]
    }

    /// Packed contact-action argument consumed by `FUN_00427E20`.
    pub const fn contact_argument_raw(&self) -> u32 {
        self.raw_words[9]
    }

    /// Signed 8.8 response word consumed by static collision.
    pub const fn contact_response_raw(&self) -> i16 {
        self.raw_words[10] as u16 as i16
    }

    /// Uninterpreted final halfword at `+0x2A`.
    pub const fn trailing_halfword_raw(&self) -> i16 {
        (self.raw_words[10] >> 16) as u16 as i16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticBurnCrater {
    pub radius_raw: i32,
    pub depth_height_units: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticBurnBurst {
    pub scatter_count: u16,
    pub particle_class: u8,
}

/// One exact seven-threshold/seven-multiplier profile and its source VA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticDamageProfileBlob {
    source_va: u32,
    raw_words: [u32; STATIC_DAMAGE_PROFILE_WORD_COUNT],
}

impl StaticDamageProfileBlob {
    pub const fn from_raw_words(
        source_va: u32,
        raw_words: [u32; STATIC_DAMAGE_PROFILE_WORD_COUNT],
    ) -> Self {
        Self {
            source_va,
            raw_words,
        }
    }

    pub const fn source_va(&self) -> u32 {
        self.source_va
    }

    pub const fn raw_words(&self) -> &[u32; STATIC_DAMAGE_PROFILE_WORD_COUNT] {
        &self.raw_words
    }

    pub const fn decoded(&self) -> DamageProfile {
        DamageProfile {
            thresholds_raw: [
                self.raw_words[0] as i32,
                self.raw_words[1] as i32,
                self.raw_words[2] as i32,
                self.raw_words[3] as i32,
                self.raw_words[4] as i32,
                self.raw_words[5] as i32,
                self.raw_words[6] as i32,
            ],
            multipliers_q8: [
                self.raw_words[7] as i32,
                self.raw_words[8] as i32,
                self.raw_words[9] as i32,
                self.raw_words[10] as i32,
                self.raw_words[11] as i32,
                self.raw_words[12] as i32,
                self.raw_words[13] as i32,
            ],
        }
    }
}

/// One exact 12-byte record consumed by `FUN_004281A0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticProgramRecord {
    raw_words: [u32; STATIC_PROGRAM_RECORD_STRIDE / 4],
}

impl StaticProgramRecord {
    pub const fn from_raw_words(raw_words: [u32; STATIC_PROGRAM_RECORD_STRIDE / 4]) -> Self {
        Self { raw_words }
    }

    pub const fn raw_words(&self) -> &[u32; STATIC_PROGRAM_RECORD_STRIDE / 4] {
        &self.raw_words
    }

    pub const fn delay_us(&self) -> u32 {
        self.raw_words[0]
    }

    pub const fn opcode_raw(&self) -> u32 {
        self.raw_words[1]
    }

    pub const fn argument_raw(&self) -> u32 {
        self.raw_words[2]
    }

    pub const fn is_terminator(&self) -> bool {
        self.opcode_raw() == 0
    }

    /// Event argument for positional-effect opcodes 2 through 5.
    pub const fn effect_event_raw(&self) -> Option<u32> {
        match self.opcode_raw() {
            2..=5 => Some(self.argument_raw()),
            _ => None,
        }
    }

    /// Exact branch selecting `FUN_00441200` rather than `FUN_004410B0`.
    pub const fn uses_layered_effect_writer(&self) -> bool {
        matches!(self.effect_event_raw(), Some(0x1E | 0x3A))
    }
}

/// One opcode-0-terminated timed program and its source VA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticProgram {
    source_va: u32,
    records: &'static [StaticProgramRecord],
}

impl StaticProgram {
    pub const fn new(source_va: u32, records: &'static [StaticProgramRecord]) -> Self {
        Self { source_va, records }
    }

    pub const fn source_va(&self) -> u32 {
        self.source_va
    }

    pub const fn records(&self) -> &'static [StaticProgramRecord] {
        self.records
    }

    pub const fn record_va(&self, index: usize) -> Option<u32> {
        if index < self.records.len() {
            Some(self.source_va + (index * STATIC_PROGRAM_RECORD_STRIDE) as u32)
        } else {
            None
        }
    }
}

/// One exact 32-byte radial payload referenced by program opcode 12.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticRadialTemplateBlob {
    source_va: u32,
    raw_words: [u32; STATIC_RADIAL_TEMPLATE_WORD_COUNT],
}

impl StaticRadialTemplateBlob {
    pub const fn from_raw_words(
        source_va: u32,
        raw_words: [u32; STATIC_RADIAL_TEMPLATE_WORD_COUNT],
    ) -> Self {
        Self {
            source_va,
            raw_words,
        }
    }

    pub const fn source_va(&self) -> u32 {
        self.source_va
    }

    pub const fn raw_words(&self) -> &[u32; STATIC_RADIAL_TEMPLATE_WORD_COUNT] {
        &self.raw_words
    }

    pub const fn decoded(&self) -> RadialDamageTemplate {
        RadialDamageTemplate {
            inner_radius_raw: self.raw_words[0] as u16 as i16,
            outer_radius_raw: (self.raw_words[0] >> 16) as u16 as i16,
            impulse_raw: self.raw_words[1] as i32,
            packet: DamagePacket {
                channels: [self.raw_words[2] as i32, self.raw_words[3] as i32],
                amounts_raw: [self.raw_words[4] as i32, self.raw_words[5] as i32],
            },
            trailing_raw: [self.raw_words[6] as i32, self.raw_words[7] as i32],
        }
    }
}

/// An executable-owned blob whose consumers are retained but not interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticOpaqueBlob<const WORDS: usize> {
    source_va: u32,
    raw_words: [u32; WORDS],
}

impl<const WORDS: usize> StaticOpaqueBlob<WORDS> {
    pub const fn from_raw_words(source_va: u32, raw_words: [u32; WORDS]) -> Self {
        Self {
            source_va,
            raw_words,
        }
    }

    pub const fn source_va(&self) -> u32 {
        self.source_va
    }

    pub const fn raw_words(&self) -> &[u32; WORDS] {
        &self.raw_words
    }

    pub const fn end_va(&self) -> u32 {
        self.source_va + (WORDS * 4) as u32
    }

    pub const fn contains_va(&self, va: u32) -> bool {
        va >= self.source_va && va < self.end_va() && (va - self.source_va) % 4 == 0
    }
}

pub const STATIC_DAMAGE_PROFILES: [StaticDamageProfileBlob; 11] = [
    StaticDamageProfileBlob::from_raw_words(
        0x004C_9BB0,
        [
            0x00000000, 0x00000FA0, 0x00000FA0, 0x00000FA0, 0x00000FA0, 0x00000000, 0x00000000,
            0x00000000, 0x00000100, 0x00000100, 0x00000100, 0x00000100, 0x00000000, 0x00000000,
        ],
    ),
    StaticDamageProfileBlob::from_raw_words(
        0x004C_9BE8,
        [
            0x00000000, 0x00000FA0, 0x000007D0, 0x00000000, 0x00001770, 0x00000000, 0x00000000,
            0x00000000, 0x00000100, 0x00000040, 0x00000200, 0x00000100, 0x00000000, 0x00000000,
        ],
    ),
    StaticDamageProfileBlob::from_raw_words(
        0x004C_9C20,
        [
            0x00000000, 0x00000FA0, 0x000007D0, 0x00000000, 0x00000BB8, 0x00000000, 0x00000000,
            0x00000000, 0x00000100, 0x00000040, 0x00000200, 0x00000100, 0x00000000, 0x00000000,
        ],
    ),
    StaticDamageProfileBlob::from_raw_words(
        0x004C_9C58,
        [
            0x00000000, 0x00000FA0, 0x00000FA0, 0x00000FA0, 0x00000FA0, 0x00000000, 0x00000000,
            0x00000000, 0x00000100, 0x00000100, 0x00000100, 0x00000100, 0x00000000, 0x00000000,
        ],
    ),
    StaticDamageProfileBlob::from_raw_words(
        0x004C_9C90,
        [
            0x00000000, 0x00000FA0, 0x000003E8, 0x000001F4, 0x000007D0, 0x00000000, 0x00000000,
            0x00000000, 0x00000200, 0x00000200, 0x00000200, 0x00000080, 0x00000000, 0x00000000,
        ],
    ),
    StaticDamageProfileBlob::from_raw_words(
        0x004C_9CC8,
        [
            0x00000000, 0x00001770, 0x00000BB8, 0x00001388, 0x000003E8, 0x00000000, 0x00000000,
            0x00000000, 0x00000100, 0x00000100, 0x00000040, 0x00000080, 0x00000000, 0x00000000,
        ],
    ),
    StaticDamageProfileBlob::from_raw_words(
        0x004C_9D00,
        [
            0x00000000, 0x00000BB8, 0x00000BB8, 0x000001F4, 0x000003E8, 0x00000000, 0x00000000,
            0x00000000, 0x00000100, 0x00000100, 0x00000200, 0x00000080, 0x00000000, 0x00000000,
        ],
    ),
    StaticDamageProfileBlob::from_raw_words(
        0x004C_9D38,
        [
            0x00000000, 0x00000FA0, 0x000007D0, 0x000007D0, 0x000003E8, 0x00000000, 0x00000000,
            0x00000000, 0x00000100, 0x00000100, 0x00000100, 0x00000080, 0x00000000, 0x00000000,
        ],
    ),
    StaticDamageProfileBlob::from_raw_words(
        0x004C_9D70,
        [
            0x00000000, 0x00001770, 0x000009C4, 0x00001388, 0x00000BB8, 0x00000000, 0x00000000,
            0x00000000, 0x00000100, 0x00000100, 0x00000040, 0x00000080, 0x00000000, 0x00000000,
        ],
    ),
    StaticDamageProfileBlob::from_raw_words(
        0x004C_9DA8,
        [
            0x00000000, 0x00004E20, 0x00004E20, 0x00002710, 0x00004E20, 0x00000000, 0x00000000,
            0x00000000, 0x00000100, 0x00000040, 0x00000200, 0x00000080, 0x00000000, 0x00000000,
        ],
    ),
    StaticDamageProfileBlob::from_raw_words(
        0x004C_9DE0,
        [
            0x00000000, 0x00000FA0, 0x000001F4, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
            0x00000000, 0x00000100, 0x00000200, 0x00000400, 0x00000000, 0x00000000, 0x00000000,
        ],
    ),
];

/// Class38's 441B70 ground entry references this program through 4C9910.
/// The pointer at 4C9910 follows the terminator and is not a third record.
pub const FIREBALL_GROUND_PROGRAM_VA: u32 = 0x004C_98F8;
const PROGRAM_4C98F8: [StaticProgramRecord; 2] = [
    StaticProgramRecord::from_raw_words([0, 11, 1]),
    StaticProgramRecord::from_raw_words([0, 0, 0]),
];

const PROGRAM_4C9918: [StaticProgramRecord; 8] = [
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000001, 0x00000001]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000004, 0x00000012]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000008, 0x00000044]),
    StaticProgramRecord::from_raw_words([0x0007A120, 0x00000003, 0x00000012]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000008, 0x0000005A]),
    StaticProgramRecord::from_raw_words([0x000186A0, 0x0000000A, 0x00000000]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x0000000C, 0x004C9898]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000000, 0x00000000]),
];

const PROGRAM_4C9978: [StaticProgramRecord; 4] = [
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000004, 0x00000012]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000008, 0x0000005A]),
    StaticProgramRecord::from_raw_words([0x0007A120, 0x0000000A, 0x00000000]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000000, 0x00000000]),
];

const PROGRAM_4C99A8: [StaticProgramRecord; 15] = [
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000001, 0x00000001]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000004, 0x00000012]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000008, 0x0000005A]),
    StaticProgramRecord::from_raw_words([0x0007A120, 0x00000004, 0x00000012]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000008, 0x0000005A]),
    StaticProgramRecord::from_raw_words([0x0007A120, 0x00000004, 0x00000012]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000008, 0x0000005A]),
    StaticProgramRecord::from_raw_words([0x0007A120, 0x00000004, 0x00000012]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000008, 0x0000005A]),
    StaticProgramRecord::from_raw_words([0x0007A120, 0x00000006, 0x00001400]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000008, 0x0000003E]),
    StaticProgramRecord::from_raw_words([0x000186A0, 0x00000006, 0x00001400]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x0000000A, 0x00000000]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x0000000C, 0x004C98B8]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000000, 0x00000000]),
];

const PROGRAM_4C9A60: [StaticProgramRecord; 7] = [
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000001, 0x00000001]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000003, 0x0000001E]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000008, 0x0000005A]),
    StaticProgramRecord::from_raw_words([0x0007A120, 0x00000003, 0x00000012]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x0000000A, 0x00000000]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x0000000C, 0x004C9878]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000000, 0x00000000]),
];

const PROGRAM_4C9AB8: [StaticProgramRecord; 6] = [
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000001, 0x00000001]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000003, 0x0000001E]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000008, 0x0000005A]),
    StaticProgramRecord::from_raw_words([0x0007A120, 0x00000003, 0x00000012]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x0000000A, 0x00000000]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000000, 0x00000000]),
];

const PROGRAM_4C9B00: [StaticProgramRecord; 7] = [
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000001, 0x00000001]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000003, 0x0000004F]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000008, 0x0000005A]),
    StaticProgramRecord::from_raw_words([0x0007A120, 0x00000003, 0x00000012]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x0000000A, 0x00000000]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x0000000C, 0x004C9878]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000000, 0x00000000]),
];

const PROGRAM_4C9B70: [StaticProgramRecord; 5] = [
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000001, 0x00000001]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000008, 0x0000003B]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x0000000D, 0x004C9B58]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x0000000A, 0x00000000]),
    StaticProgramRecord::from_raw_words([0x00000000, 0x00000000, 0x00000000]),
];

pub const STATIC_PROGRAMS: [StaticProgram; 8] = [
    StaticProgram::new(FIREBALL_GROUND_PROGRAM_VA, &PROGRAM_4C98F8),
    StaticProgram::new(0x004C_9918, &PROGRAM_4C9918),
    StaticProgram::new(0x004C_9978, &PROGRAM_4C9978),
    StaticProgram::new(0x004C_99A8, &PROGRAM_4C99A8),
    StaticProgram::new(0x004C_9A60, &PROGRAM_4C9A60),
    StaticProgram::new(0x004C_9AB8, &PROGRAM_4C9AB8),
    StaticProgram::new(0x004C_9B00, &PROGRAM_4C9B00),
    StaticProgram::new(0x004C_9B70, &PROGRAM_4C9B70),
];

pub const STATIC_RADIAL_TEMPLATES: [StaticRadialTemplateBlob; 3] = [
    StaticRadialTemplateBlob::from_raw_words(
        0x004C_9878,
        [
            0x04000200, 0x000007D0, 0x00000001, 0x00000003, 0x000007D0, 0x000003E8, 0xFFFFFFFF,
            0x00000000,
        ],
    ),
    StaticRadialTemplateBlob::from_raw_words(
        0x004C_9898,
        [
            0x02000100, 0x000007D0, 0x00000001, 0x00000003, 0x000003E8, 0x000003E8, 0xFFFFFFFF,
            0x00000000,
        ],
    ),
    StaticRadialTemplateBlob::from_raw_words(
        0x004C_98B8,
        [
            0x0DAC0800, 0x000007D0, 0x00000002, 0x00000003, 0x00001770, 0x00001770, 0xFFFFFFFF,
            0x00000000,
        ],
    ),
];

/// The sole 20-byte payload referenced by authored opcode 13.
pub const STATIC_OPCODE_13_BLOB: StaticOpaqueBlob<5> = StaticOpaqueBlob::from_raw_words(
    0x004C_9B58,
    [0x00000002, 0x00000000, 0x4F4F0000, 0x00000000, 0x00000000],
);

/// Kind27's burn crater radius/depth, followed by an unowned adjacent pair.
pub const STATIC_KIND_27_AUXILIARY_BLOB: StaticOpaqueBlob<4> = StaticOpaqueBlob::from_raw_words(
    0x004C_9E28,
    [0x00000800, 0x00000014, 0x00000400, 0x0000000A],
);

pub const STATIC_KIND_DESCRIPTORS: [StaticKindDescriptor; STATIC_KIND_COUNT] = [
    StaticKindDescriptor::from_raw_words([
        0x004C9BE8, 0x000003E8, 0x004C9918, 0x00000000, 0x00001388, 0x00000000, 0x00000005,
        0x00000006, 0x00000000, 0x00000000, 0x0000FC00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9C20, 0x000003E8, 0x004C9978, 0x00000000, 0x00000BB8, 0x00000000, 0x00000005,
        0x00000006, 0x00000000, 0x00000000, 0x00000700,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9C58, 0x00000000, 0x00000000, 0x00000000, 0x00002710, 0x00000000, 0x00000000,
        0x00000000, 0x00000001, 0x004E2037, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9CC8, 0x00000FA0, 0x004C9A60, 0x00000000, 0x00002328, 0x00000000, 0x00000005,
        0x00000006, 0x00000000, 0x00000000, 0x0000FC00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9DE0, 0x000001F4, 0x00000000, 0x00000000, 0x00001F40, 0x0000005D, 0x00000000,
        0x00000000, 0x00000001, 0x0186A033, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9C58, 0x00000000, 0x00000000, 0x00000000, 0x00002710, 0x00000000, 0x00000000,
        0x00000000, 0x00000001, 0x00000136, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9C58, 0x00000000, 0x00000000, 0x00000000, 0x00002710, 0x00000000, 0x00000000,
        0x00000000, 0x00000000, 0x00000000, 0x00000000,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9C58, 0x00000000, 0x00000000, 0x00000000, 0x00002710, 0x00000010, 0x00000000,
        0x00000000, 0x00000001, 0x00006435, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9D38, 0x000007D0, 0x004C9978, 0x00000000, 0x00000BB8, 0x00000000, 0x00000005,
        0x00000006, 0x00000000, 0x00000000, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9C90, 0x000003E8, 0x004C9B70, 0x00000000, 0x00000BB8, 0x00000000, 0x00000005,
        0x00000006, 0x00000000, 0x00000000, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9D70, 0x00000FA0, 0x004C9AB8, 0x00000000, 0x00002328, 0x00000000, 0x00000005,
        0x00000006, 0x00000000, 0x00000000, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9DA8, 0x000186A0, 0x004C9A60, 0x00000000, 0x00001388, 0x00000000, 0x00000005,
        0x00000006, 0x00000000, 0x00000000, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9C58, 0x00000000, 0x00000000, 0x00000000, 0x00002710, 0x00000010, 0x00000000,
        0x00000000, 0x00000000, 0x00000000, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9C58, 0x00000000, 0x00000000, 0x00000000, 0x00002710, 0x00000010, 0x00000000,
        0x00000000, 0x00000000, 0x00000000, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9C58, 0x00000000, 0x00000000, 0x00000000, 0x00002710, 0x00000010, 0x00000000,
        0x00000000, 0x00000000, 0x00000000, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9C58, 0x00000000, 0x00000000, 0x00000000, 0x00002710, 0x00000010, 0x00000000,
        0x00000000, 0x00000000, 0x00000000, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9C58, 0x00000000, 0x00000000, 0x00000000, 0x00002710, 0x00000010, 0x00000000,
        0x00000000, 0x00000000, 0x00000000, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9C58, 0x00000000, 0x00000000, 0x00000000, 0x00002710, 0x00000010, 0x00000000,
        0x00000000, 0x00000000, 0x00000000, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9C58, 0x00000000, 0x00000000, 0x00000000, 0x00002710, 0x00000010, 0x00000000,
        0x00000000, 0x00000000, 0x00000000, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9C58, 0x00000000, 0x00000000, 0x00000000, 0x00002710, 0x00000010, 0x00000000,
        0x00000000, 0x00000000, 0x00000000, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9C58, 0x00000000, 0x00000000, 0x00000000, 0x00002710, 0x00000010, 0x00000000,
        0x00000000, 0x00000000, 0x00000000, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9DE0, 0x000001F4, 0x00000000, 0x00000000, 0x00002710, 0x00000010, 0x00000000,
        0x00000000, 0x00000001, 0x0003E834, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9BB0, 0x00001F40, 0x00000000, 0x00000000, 0x00002710, 0x00000010, 0x00000000,
        0x00000000, 0x00000002, 0x00000001, 0x00000600,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9BB0, 0x00001F40, 0x00000000, 0x00000000, 0x00002710, 0x00000010, 0x00000000,
        0x00000000, 0x00000002, 0x00000002, 0x00000600,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9BB0, 0x00001F40, 0x00000000, 0x00000000, 0x00002710, 0x00000010, 0x00000000,
        0x00000000, 0x00000002, 0x00000003, 0x00000600,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9BB0, 0x00001F40, 0x00000000, 0x00000000, 0x00002710, 0x00000010, 0x00000000,
        0x00000000, 0x00000002, 0x00000004, 0x00000600,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9BB0, 0x00001F40, 0x00000000, 0x00000000, 0x00002710, 0x00000010, 0x00000000,
        0x00000000, 0x00000002, 0x00000005, 0x00000600,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9DE0, 0x000001F4, 0x004C99A8, 0x004C9E28, 0x00003E80, 0x0000005D, 0x00000000,
        0x00000000, 0x00000000, 0x00000000, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9DA8, 0x000186A0, 0x004C9918, 0x00000000, 0x00001388, 0x00000000, 0x00000005,
        0x00000006, 0x00000000, 0x00000000, 0x0000FF00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9D00, 0x000007D0, 0x004C9B00, 0x00000000, 0x00002328, 0x00000000, 0x00000005,
        0x00000006, 0x00000000, 0x00000000, 0x0000FC00,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9BB0, 0x00000000, 0x00000000, 0x00000000, 0x00002710, 0x00000000, 0x00000000,
        0x00000000, 0x00000000, 0x00000000, 0x00000000,
    ]),
    StaticKindDescriptor::from_raw_words([
        0x004C9C58, 0x00000000, 0x00000000, 0x00000000, 0x00002710, 0x00000010, 0x00000000,
        0x00000000, 0x00000001, 0x00000001, 0x0000FF00,
    ]),
];

pub fn static_kind_descriptor(kind_index: u32) -> Option<&'static StaticKindDescriptor> {
    STATIC_KIND_DESCRIPTORS.get(kind_index as usize)
}

pub fn static_damage_profile(source_va: u32) -> Option<&'static StaticDamageProfileBlob> {
    STATIC_DAMAGE_PROFILES
        .iter()
        .find(|profile| profile.source_va() == source_va)
}

pub fn static_program(source_va: u32) -> Option<&'static StaticProgram> {
    STATIC_PROGRAMS
        .iter()
        .find(|program| program.source_va() == source_va)
}

pub fn static_radial_template(source_va: u32) -> Option<&'static StaticRadialTemplateBlob> {
    STATIC_RADIAL_TEMPLATES
        .iter()
        .find(|template| template.source_va() == source_va)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fnv1a64<'a>(bytes: impl IntoIterator<Item = &'a u8>) -> u64 {
        bytes.into_iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }

    #[test]
    fn descriptor_table_has_exact_retail_bounds_and_bytes() {
        assert_eq!(STATIC_KIND_RECORD_STRIDE, 0x2C);
        assert_eq!(STATIC_KIND_DESCRIPTORS.len(), 32);
        assert_eq!(STATIC_KIND_TABLE_END_VA, 0x004C_A3B8);
        assert_eq!(
            STATIC_KIND_TABLE_VA
                + (STATIC_KIND_COUNT as u32 - 1) * STATIC_KIND_RECORD_STRIDE as u32,
            0x004C_A38C
        );

        let bytes: Vec<_> = STATIC_KIND_DESCRIPTORS
            .iter()
            .flat_map(StaticKindDescriptor::raw_words)
            .flat_map(|word| word.to_le_bytes())
            .collect();
        assert_eq!(bytes.len(), STATIC_KIND_COUNT * STATIC_KIND_RECORD_STRIDE);
        assert_eq!(fnv1a64(&bytes), 0xCA20_DB0D_2714_1B00);
    }

    #[test]
    fn referenced_blob_catalogs_match_the_exact_retail_bytes() {
        let profile_bytes: Vec<_> = STATIC_DAMAGE_PROFILES
            .iter()
            .flat_map(StaticDamageProfileBlob::raw_words)
            .flat_map(|word| word.to_le_bytes())
            .collect();
        assert_eq!(
            profile_bytes.len(),
            STATIC_DAMAGE_PROFILES.len() * STATIC_DAMAGE_PROFILE_BYTE_COUNT
        );
        assert_eq!(fnv1a64(&profile_bytes), 0x5650_82FA_B58C_351D);

        let program_bytes: Vec<_> = STATIC_PROGRAMS
            .iter()
            .flat_map(StaticProgram::records)
            .flat_map(StaticProgramRecord::raw_words)
            .flat_map(|word| word.to_le_bytes())
            .collect();
        assert_eq!(fnv1a64(&program_bytes), 0x627A_9052_D7A1_1836);

        let radial_bytes: Vec<_> = STATIC_RADIAL_TEMPLATES
            .iter()
            .flat_map(StaticRadialTemplateBlob::raw_words)
            .flat_map(|word| word.to_le_bytes())
            .collect();
        assert_eq!(fnv1a64(&radial_bytes), 0x6405_2E11_AB1C_268D);

        let opcode_13_bytes: Vec<_> = STATIC_OPCODE_13_BLOB
            .raw_words()
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect();
        assert_eq!(fnv1a64(&opcode_13_bytes), 0x3320_6D84_3667_712D);

        let auxiliary_bytes: Vec<_> = STATIC_KIND_27_AUXILIARY_BLOB
            .raw_words()
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect();
        assert_eq!(fnv1a64(&auxiliary_bytes), 0xA4C4_AC45_7097_BDCF);
    }

    #[test]
    fn every_descriptor_pointer_closes_over_the_bounded_catalog() {
        for (kind, descriptor) in STATIC_KIND_DESCRIPTORS.iter().enumerate() {
            assert!(
                static_damage_profile(descriptor.damage_profile_va()).is_some(),
                "kind {kind} profile {:08X}",
                descriptor.damage_profile_va()
            );
            if descriptor.timed_program_va() != 0 {
                assert!(
                    static_program(descriptor.timed_program_va()).is_some(),
                    "kind {kind} program {:08X}",
                    descriptor.timed_program_va()
                );
            }
            if descriptor.field_0c_raw() != 0 {
                assert!(
                    STATIC_KIND_27_AUXILIARY_BLOB.contains_va(descriptor.field_0c_raw()),
                    "kind {kind} +0x0C {:08X}",
                    descriptor.field_0c_raw()
                );
            }
            assert_eq!(descriptor.trailing_halfword_raw(), 0, "kind {kind}");
        }

        for program in STATIC_PROGRAMS {
            assert!(
                program
                    .records()
                    .last()
                    .is_some_and(StaticProgramRecord::is_terminator),
                "program {:08X}",
                program.source_va()
            );
            assert!(
                program.records()[..program.records().len() - 1]
                    .iter()
                    .all(|record| !record.is_terminator()),
                "program {:08X}",
                program.source_va()
            );
            for record in program.records() {
                match record.opcode_raw() {
                    12 => assert!(
                        static_radial_template(record.argument_raw()).is_some(),
                        "program {:08X} radial {:08X}",
                        program.source_va(),
                        record.argument_raw()
                    ),
                    13 => assert_eq!(record.argument_raw(), STATIC_OPCODE_13_BLOB.source_va()),
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn descriptor_profiles_and_programs_use_the_exact_unique_pointer_sets() {
        let mut profiles: Vec<_> = STATIC_KIND_DESCRIPTORS
            .iter()
            .map(StaticKindDescriptor::damage_profile_va)
            .collect();
        profiles.sort_unstable();
        profiles.dedup();
        assert_eq!(
            profiles,
            STATIC_DAMAGE_PROFILES
                .iter()
                .map(StaticDamageProfileBlob::source_va)
                .collect::<Vec<_>>()
        );

        let mut programs: Vec<_> = STATIC_KIND_DESCRIPTORS
            .iter()
            .map(StaticKindDescriptor::timed_program_va)
            .filter(|address| *address != 0)
            .collect();
        // 441B70 supplies this standalone program rather than a kind descriptor.
        programs.push(FIREBALL_GROUND_PROGRAM_VA);
        programs.sort_unstable();
        programs.dedup();
        assert_eq!(
            programs,
            STATIC_PROGRAMS
                .iter()
                .map(StaticProgram::source_va)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn response_words_match_the_independently_recovered_collision_table() {
        let responses: Vec<_> = STATIC_KIND_DESCRIPTORS
            .iter()
            .map(StaticKindDescriptor::contact_response_raw)
            .collect();
        assert_eq!(
            responses,
            [
                -1024, 1792, -256, -1024, -256, -256, 0, -256, -256, -256, -256, -256, -256, -256,
                -256, -256, -256, -256, -256, -256, -256, -256, 1536, 1536, 1536, 1536, 1536, -256,
                -256, -1024, 0, -256,
            ]
        );
    }

    #[test]
    fn profile_and_radial_accessors_decode_representative_exact_payloads() {
        let kind_zero = static_damage_profile(0x004C_9BE8).unwrap().decoded();
        assert_eq!(
            kind_zero,
            DamageProfile {
                thresholds_raw: [0, 4000, 2000, 0, 6000, 0, 0],
                multipliers_q8: [0, 256, 64, 512, 256, 0, 0],
            }
        );

        let radial = static_radial_template(0x004C_9898).unwrap().decoded();
        assert_eq!(radial.inner_radius_raw, 0x100);
        assert_eq!(radial.outer_radius_raw, 0x200);
        assert_eq!(radial.impulse_raw, 2000);
        assert_eq!(radial.packet.channels, [1, 3]);
        assert_eq!(radial.packet.amounts_raw, [1000, 1000]);
        assert_eq!(radial.trailing_raw, [-1, 0]);
    }

    #[test]
    fn event_bytes_remain_exact_and_the_layered_branch_keeps_both_classes() {
        let authored_events: Vec<_> = STATIC_PROGRAMS
            .iter()
            .flat_map(StaticProgram::records)
            .filter_map(StaticProgramRecord::effect_event_raw)
            .collect();
        assert!(authored_events.contains(&0x1E));
        assert!(!authored_events.contains(&0x3A));

        let class_1e = StaticProgramRecord::from_raw_words([0, 3, 0x1E]);
        let class_3a = StaticProgramRecord::from_raw_words([0, 3, 0x3A]);
        let ordinary = StaticProgramRecord::from_raw_words([0, 3, 18]);
        assert!(class_1e.uses_layered_effect_writer());
        assert!(class_3a.uses_layered_effect_writer());
        assert!(!ordinary.uses_layered_effect_writer());
        assert_ne!(class_1e.argument_raw(), class_3a.argument_raw());
    }
}
