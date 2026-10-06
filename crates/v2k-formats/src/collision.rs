//! Section 12 collision volume model decoder.
//!
//! 296-byte (0x128) base records with 14 optional sub-sections at pointer
//! offsets 0x34-0x43. Variable-length Sub-H has a two-word header followed by
//! count × 0x14-byte records. Those records serve both the historical face
//! diagnostics and the common external-frame runtime. A terminated
//! behavior-choice list follows the relocated payloads.
//!
//! Entry-size calculation follows the validated Section-12 record layout
//! documented in `FORMAT_DOCUMENTATION.md`.
//!
//! 9 known topology types across 5 vertex counts (4, 6, 8, 10, 14).
//! 104 models across 2 levels.
//!
//! The executable and runtime observations are authoritative; investigation
//! scripts are supporting evidence only.

use v2k_core::{Result, V2kError};

/// Base entry header size in bytes.
const ENTRY_BASE_SIZE: usize = 0x128; // 296 bytes

/// Sub-section definitions: (field_index, fixed_size, needs_align).
/// Size 0 means variable-length (H and J).
const SUB_DEFS: &[(&str, usize, usize, bool)] = &[
    // (name, field_idx, fixed_size, align)
    ("A", 0x34, 6, false),
    ("B", 0x35, 8, true),
    ("C", 0x36, 16, true),
    ("D", 0x37, 12, true),
    ("E", 0x38, 28, true),
    ("F", 0x39, 14, false),
    ("G", 0x3A, 104, true),
    ("H", 0x3B, 0, true), // variable: 4 + count * 0x14
    ("I", 0x3C, 8, false),
    ("J", 0x3D, 0, false), // variable: 2 + count * 8
    ("K", 0x3E, 2, false),
    ("L", 0x3F, 6, false),
    ("M", 0x41, 18, false),
    ("N", 0x42, 10, false),
    ("O", 0x43, 4, false),
];

/// The historical face-oriented view of one Sub-H record.
///
/// Use [`CollisionEntry::sub_h_external_frame_descriptor`] when consuming the
/// same bytes through the common external-frame runtime.
#[derive(Debug, Clone)]
pub struct Face {
    pub flags1: u32,
    pub flags2: u32,
    pub ref_a: u16,
    pub ref_b: u16,
    pub ref_c: u16,
    /// Vertex indices (4 bytes: v0, v1, v2, v3).
    pub verts: [u8; 4],
    /// True if v2 == v3 (triangle, not quad).
    pub is_triangle: bool,
}

/// Section-12 Sub-H as consumed by the common external-frame component.
///
/// The same authored 0x14-byte records are also retained as [`Face`] values
/// for the collision/render diagnostics that historically exposed them. The
/// retail external-frame constructor and writer give the header word at
/// `+0x02` and several record fields a second, independently proven meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubHExternalFrameDescriptor {
    /// Optional positional cue emitted when a nonzero phase completes.
    pub completion_sound_id: Option<u16>,
    pub records: Vec<SubHExternalFrameRecord>,
}

/// One 0x14-byte Sub-H record in the external-frame interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubHExternalFrameRecord {
    /// Exact dword at `+0x00`; the live constraint resolver owns these bits.
    pub resolver_flags_raw: u32,
    /// Signed phase rate used by `FUN_0041D120`.
    pub phase_rate_raw: i32,
    /// Three authored model-record references used by `FUN_0041D360`.
    pub vertex_refs: [u16; 3],
    /// Authored axis/plane selector used by the live constraint resolver.
    pub axis_mode_raw: u16,
    /// Four Sub-H record indices that must have zero phase before a gated
    /// record may begin its own phase.
    pub dependencies: [u8; 4],
}

/// One authored behavior candidate from the terminated list at field 0x46
/// (runtime type-record offset `+0x118`).
///
/// `FUN_00410090` resolves `weight_rule_id` through the executable's weight-
/// callback table and `behavior_class_id` through the named behavior table.
/// `FUN_00425680` evaluates each rule, multiplies it by
/// `weight_multiplier`, then makes the weighted selection consumed by the
/// common entity initializer. These are not render materials.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BehaviorChoice {
    pub weight_rule_id: u32,
    pub weight_multiplier: u32,
    pub behavior_class_id: u32,
}

/// The unconditional eight-byte descriptor copied by `FUN_00409A80` from
/// Section-12 `+0xC8` into the common entity component at `+0x28`.
///
/// `FUN_00423030` consumes the first signed dword as a strict wrapped-coordinate
/// limit. A zero limit selects its unbounded mode. The second dword is retained
/// exactly until its other consumers are recovered.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CommonAxisDescriptor {
    pub strict_axis_limit_raw: i32,
    pub raw_word_at_0x04: u32,
}

/// Section-12 Sub-A's exact six-byte propulsion descriptor.
///
/// `FUN_00420450` uses [`Self::target_speed_base_raw`] to initialize the
/// common Sub-A runtime's RNG-owned target speed. `FUN_0041E9E0` then selects
/// one of the two authored acceleration limits according to whether the
/// current speed error agrees with the runtime direction multiplier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubAPropulsionDescriptor {
    pub acceleration_raw: i16,
    pub overspeed_correction_raw: i16,
    pub target_speed_base_raw: i16,
}

/// Section-12 Sub-B's exact eight-byte lateral-velocity descriptor.
///
/// `FUN_0041EB50` time-scales both dwords. The first establishes a symmetric
/// projected-velocity threshold; inside that threshold retail removes the
/// complete lateral projection, while outside it removes only the time-scaled
/// correction rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubBLateralDescriptor {
    pub projection_threshold_rate_raw: i32,
    pub correction_rate_raw: i32,
}

/// Section-12 Sub-C's exact sixteen-byte lift/surface descriptor.
///
/// `FUN_0041F1C0` consumes the five signed scalar fields plus byte `+0x0C`
/// and byte `+0x0D`.  Common-Dying `FUN_00404220` independently reads the
/// signed byte at `+0x0C`, so retain its exact value instead of reducing it to
/// a boolean.  The final two bytes are not consumed by the recovered paths but
/// remain part of the fixed-layout evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubCLiftDescriptor {
    pub base_clearance_raw: i16,
    pub lift_range_raw: i16,
    pub strength_raw: i32,
    pub near_boost_range_raw: i16,
    pub damping_range_raw: i16,
    pub surface_mode_raw: i8,
    pub offset_sample_raw: u8,
    pub reserved_at_0x0e: [u8; 2],
}

/// Section-12 Sub-D's exact twelve-byte steering/avoidance descriptor.
///
/// `FUN_0041F660` consumes this through the common mover. The two signed 8.8
/// probe distances feed its terrain/occupancy classifier; the first dword and
/// the two steering flags feed the yaw/pitch integration helpers. Retaining the
/// final byte is intentional even though the audited path does not consume it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubDSteeringDescriptor {
    pub steering_divisor_raw: i32,
    pub couple_yaw_into_roll_raw: i8,
    pub enable_pitch_steering_raw: u8,
    pub forward_probe_raw: i16,
    pub lateral_probe_raw: i16,
    pub classifier_flags: u8,
    pub reserved_at_0x0b: u8,
}

/// Section-12 Sub-F's fourteen-byte swimming descriptor (`424220`/`4236D0`).
///
/// The constructor sign-extends all three words and the six model-variable
/// selectors. Animation mode and the unused final byte retain their raw values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubFSwimmingDescriptor {
    pub clearance_base_raw: i16,
    pub clearance_random_span_raw: i16,
    pub target_speed_base_raw: i16,
    pub animation_mode_raw: u8,
    pub variable_selectors: [i8; 6],
    pub reserved_at_0x0d: u8,
}

/// Section-12 Sub-J's authored attachment-list descriptor.
///
/// Retail `FUN_00418330` and demo `FUN_004182F0` allocate one twelve-byte
/// live record for each authored slot and reject counts above eight.  The
/// live array has its own mutable length and capacity; these bytes describe
/// only the immutable per-slot policy and owner-local offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubJAttachmentDescriptor {
    /// Exact second header byte. No audited consumer assigns it semantics.
    pub reserved_at_0x01: u8,
    pub slots: Box<[SubJAttachmentSlotDescriptor]>,
}

/// One eight-byte authored Sub-J attachment slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubJAttachmentSlotDescriptor {
    /// Exact leading word. Its low byte participates in relation/network
    /// policy; positional application is gated independently by child state.
    pub policy_word_raw: u16,
    /// Signed owner-local X/Y/Z offset consumed by the attachment updater.
    pub local_offset_raw: [i16; 3],
}

/// An authored optional payload attached to a Section 12 entity-type record.
///
/// The loader relocates these blocks through header fields 0x34..0x43.  Most
/// of their gameplay semantics are type-specific, so retain the exact bytes
/// while individual behavior/physics readers are recovered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollisionSubsection {
    /// Original loader label (`A` through `O`, excluding unused field 0x40).
    pub name: &'static str,
    /// Header field index containing this block's authored relative offset.
    pub field_index: usize,
    /// Exact on-disk payload, including the count prefix for variable blocks.
    pub data: Vec<u8>,
}

/// Section-12 Sub-M's exact 18-byte status-component descriptor.
///
/// `FUN_00409A80` uses the six bytes at `+0x02..+0x07` as one-based indices
/// into an entity's u16 variable bank. A zero selector leaves the matching
/// runtime output pointer null. `198E0 ->199B0` consumes word `+0x00` as the
/// current model's marker slot; `19010` and `198E0` consume signed product world
/// offsets at `+0x0C/+0x0E/+0x10`. The intervening tail prefix stays retained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusComponentDescriptor {
    /// Current model marker slot selected by Sub-M external-frame selector0.
    pub raw_word_at_0x00: u16,
    /// One-based u16 entity-variable selectors; zero means unbound.
    pub variable_bindings: [u8; 6],
    /// Exact +08..+11 bytes; indices4..9 are signed product world offsets.
    pub raw_tail: [u8; 10],
}

/// Section-12 Sub-E's exact 28-byte projectile-emitter descriptor.
///
/// `FUN_00424650` consumes this block when a behavior-owned fire task asks the
/// common projectile creator to emit. The names below are limited to fields
/// whose reads are established by that routine. The final four bytes are
/// retained as variable bindings because the Section-12 loader relocates them
/// through the same component-variable machinery as other optional blocks;
/// a zero byte leaves the matching binding absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectileEmitterDescriptor {
    /// Projectile-method/class selector at `+0x00`.
    pub projectile_method: u32,
    /// Stochastic cadence numerator in microseconds at `+0x04`.
    pub random_interval_us: u32,
    /// Symmetric X/Z target-position spread at `+0x08`.
    pub spread_raw: u16,
    /// Signed aim-error half-range at `+0x0A`; values above `0x7FFF` bypass
    /// the normal aim/line-of-sight test.
    pub aim_threshold_raw: u16,
    /// Signed projectile-speed override at `+0x0C`; zero selects the
    /// projectile class table's speed.
    pub speed_override_raw: i16,
    /// Strict wrapped X/Z target-position tolerance at `+0x0E`.
    pub target_axis_tolerance_raw: u16,
    /// Positional sound requested after a successful emission at `+0x10`.
    pub sound_id: u16,
    /// Exact word at `+0x12`; the audited common creator does not read it.
    pub raw_word_at_0x12: u16,
    /// Nonzero makes the common creator toggle its A/B emitter selector after
    /// each successful emission.
    pub alternate_emitter_raw: u16,
    /// Raw `+0x16` stochastic-gate mode.
    pub stochastic_gate_mode: u8,
    /// Nonzero asks the common creator for its auxiliary class-15 command.
    pub auxiliary_command: u8,
    /// Exact final four component-variable binding selectors.
    pub variable_bindings: [u8; 4],
}

/// Section-12 Sub-I's exact 8-byte actor-animation descriptor.
///
/// `FUN_00420760` uses the first two words as mutually exclusive positional
/// cues when a linked target is installed. `FUN_00420870` uses the third word
/// for the attention/forced-stop path. `FUN_00420730` binds the common
/// animation controller's output through the one-based selector at `+0x06`;
/// zero leaves the output unbound. The final byte is the authored directional
/// frame stride consumed by `FUN_00420610`, `FUN_00420630`, and
/// `FUN_00420650`. The controller's phase count is independently hard-coded
/// to four and must not be inferred from that stride.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorAnimationDescriptor {
    /// Cue selected when the linked target has capability bit 3.
    pub capability_bit_3_sound_id: u16,
    /// Cue selected when the linked target has any capability in mask 0x201.
    pub capability_mask_0x201_sound_id: u16,
    /// Cue selected by the attention/forced-stop setup path.
    pub attention_stop_sound_id: u16,
    /// One-based entity-variable selector receiving the animation output.
    pub variable_binding: u8,
    /// Authored selector stride between the eight directional groups.
    pub frames_per_direction: u8,
}

/// A parsed collision volume entry.
#[derive(Debug)]
pub struct CollisionEntry {
    /// Entry index in the section.
    pub index: usize,
    /// Byte offset within section data.
    pub offset: usize,
    /// Total entry size in bytes.
    pub size: usize,
    /// Type tag (field 0).
    pub type_tag: u32,
    /// Scale (field 1).
    pub scale: u32,
    /// ID field (field 2).
    pub id_field: u32,
    /// Four global Section-8 model ids at +0x0C..+0x13. The entity renderer
    /// selects one using the runtime 0x2000/0x4000 state bits. These two
    /// packed dwords were previously mislabeled as collision bounds.
    pub model_ids: [u16; 4],
    /// Packed model-id dwords at fields 3-4, retained for raw diagnostics.
    pub bounds: [u32; 2],
    /// Dimension extents (fields 5-10).
    pub dims: [u32; 6],
    /// Authored initializer-state policy at field 0x30 / runtime type-record
    /// `+0xC0`. `FUN_0040D3C0` copies this to live entity `+0xC8` and
    /// translates its known bits into the initial live state word.
    pub initializer_state_flags_raw: u32,
    /// First dword of the common axis descriptor at field `0x32` / `+0xC8`.
    ///
    /// Kept under its historical public name for manifest/viewer compatibility;
    /// new gameplay code should use [`Self::common_axis_descriptor`].
    pub flags: u32,
    /// Parsed face definitions.
    pub faces: Vec<Face>,
    /// Weighted behavior candidates selected by the common type initializer.
    pub behavior_choices: Vec<BehaviorChoice>,
    /// Names of active sub-sections.
    pub active_subs: Vec<String>,
    /// Exact 0x128-byte on-disk type header, before runtime pointer relocation.
    pub raw_header: [u8; ENTRY_BASE_SIZE],
    /// Exact bytes for every active optional subsection.
    pub subsections: Vec<CollisionSubsection>,
    /// Authored behavior-rule index at header field 0x47. The loader resolves
    /// it through the same executable evaluator table used by
    /// [`BehaviorChoice::weight_rule_id`]; its later alternate-path role is
    /// not yet named more narrowly.
    pub behavior_rule_ref: u32,
    /// Alternate behavior-class index at header field 0x49. The common
    /// selector uses this only for the live-state `0x4000` path; ordinary
    /// fresh entities select from [`Self::behavior_choices`].
    pub alternate_behavior_class_ref: u32,
    /// Sub-A position (3 × i16), if present.
    pub sub_a_pos: Option<(i16, i16, i16)>,
}

impl CollisionEntry {
    /// Decode the exact descriptor copied by `FUN_004235A0` /
    /// `FUN_004235D0` during common entity-component construction.
    pub fn common_axis_descriptor(&self) -> CommonAxisDescriptor {
        CommonAxisDescriptor {
            strict_axis_limit_raw: self.flags as i32,
            raw_word_at_0x04: u32::from_le_bytes(
                self.raw_header[0xCC..0xD0]
                    .try_into()
                    .expect("fixed Section-12 header range"),
            ),
        }
    }

    /// Positional cue used by the ordinary accepted-damage callback.
    ///
    /// Both `FUN_00410D30` and class-1 `FUN_00410EB0` read this unsigned word
    /// from the runtime type record at `+0x80` after accepted damage. The
    /// former applies its strict ten-tick presentation throttle; the latter
    /// has no cadence gate. A zero selector means that the type has no authored
    /// cue for either path.
    pub fn accepted_hit_presentation_sound_id(&self) -> Option<u16> {
        self.optional_sound_id_at(0x80)
    }

    /// Positional cue requested by `FUN_00411250` after it sets entity bit
    /// `0x2000`.
    ///
    /// This unsigned word at type-record `+0x82` is independent from the
    /// accepted-hit selector at `+0x80`. A zero selector skips
    /// `FUN_0044F450`; dying bit `0x4000` already set before the `0x2000`
    /// write also skips it. First-world type 17 authors 92; types 6/9/46/47/66/67
    /// author zero.
    pub fn infected_model_presentation_sound_id(&self) -> Option<u16> {
        self.optional_sound_id_at(0x82)
    }

    /// Positional death cue requested by `FUN_00410C10` before the type death
    /// callback. This selector is independent from both ordinary hit cues.
    pub fn death_sound_id(&self) -> Option<u16> {
        self.optional_sound_id_at(0x90)
    }

    /// Constructor-time retained sound attachment released by generic death.
    ///
    /// `FUN_004104B0` first clears entity `+0x8C`, then creates this optional
    /// resource only when the unsigned Section-12 word at `+0xB4` is nonzero.
    /// `FUN_00410C10` releases and clears the resulting runtime pointer after
    /// requesting the independent `+0x90` death cue.
    pub fn constructor_sound_attachment_id(&self) -> Option<u16> {
        self.optional_sound_id_at(0xB4)
    }

    /// Generic live-entry hit cue requested by `FUN_00414E90`, even when the
    /// pre-health buffer reduces the damage reaching health to zero or the
    /// later health subtraction proves lethal.
    pub fn generic_hit_sound_id(&self) -> Option<u16> {
        self.optional_sound_id_at(0x98)
    }

    /// Optional prelude cue played by class-7 `FUN_0040ADE0` before slot-2.
    ///
    /// A zero selector skips the cue. A nonzero word is `FUN_0044F480` at
    /// entity `+0x96` with full gain and rate. This word is independent from
    /// Aim-and-Fire's later `+0x9C` constructor argument.
    pub fn search_attack_optional_prelude_sound_id(&self) -> Option<u16> {
        self.optional_sound_id_at(0x9A)
    }

    /// Optional Aim-and-Fire cue copied by `FUN_0040ADE0` into `FUN_00402220`.
    pub fn search_attack_aim_sound_id(&self) -> Option<u16> {
        self.optional_sound_id_at(0x9C)
    }

    /// Signed period dword copied by `FUN_0040ADE0` into `FUN_00402220`.
    pub fn search_attack_aim_sound_period_raw(&self) -> i32 {
        i32::from_le_bytes(
            self.raw_header[0xA8..0xAC]
                .try_into()
                .expect("Section-12 headers always include offset +0xA8"),
        )
    }

    /// Optional positional cue copied into class-10 Run Away's fleeing task.
    /// A zero selector disables the cue independently from its period field.
    pub fn run_away_optional_sound_id(&self) -> Option<u16> {
        self.optional_sound_id_at(0x9E)
    }

    /// Raw signed-comparison period copied into class-10 Run Away's fleeing
    /// task. Retail reads the complete dword at type-record `+0xAC`.
    pub fn run_away_sound_period_raw(&self) -> u32 {
        u32::from_le_bytes(
            self.raw_header[0xAC..0xB0]
                .try_into()
                .expect("Section-12 headers always include offset +0xAC"),
        )
    }

    /// Exact unsigned word at runtime type-record `+0xA2`.
    ///
    /// Class-4 `"Defecate Virus"` passes this value to its slot-2 terrain
    /// task as a millisecond lifetime. Keeping the raw authored word on the
    /// canonical type record avoids replacing the captured type-26 value with
    /// a behavior- or entity-type-specific constant.
    pub fn terrain_contact_task_lifetime_ms(&self) -> u16 {
        u16::from_le_bytes(
            self.raw_header[0xA2..0xA4]
                .try_into()
                .expect("Section-12 headers always include offset +0xA2"),
        )
    }

    fn optional_sound_id_at(&self, offset: usize) -> Option<u16> {
        let sound_id = u16::from_le_bytes(
            self.raw_header[offset..offset + 2]
                .try_into()
                .expect("Section-12 headers always include retained sound selectors"),
        );
        (sound_id != 0).then_some(sound_id)
    }

    /// Global Section-3 sprite selected for this entity on the fullscreen map.
    ///
    /// `FUN_0049b140` reads the unsigned word at runtime type-record `+0x70`
    /// and skips the icon when that authored selector is zero. Keeping this on
    /// the type record avoids leaking a magic raw-header offset into gameplay
    /// presentation code.
    pub fn fullscreen_map_icon_sprite_id(&self) -> Option<u16> {
        let sprite_id = u16::from_le_bytes(
            self.raw_header[0x70..0x72]
                .try_into()
                .expect("Section-12 headers always include offset +0x70"),
        );
        (sprite_id != 0).then_some(sprite_id)
    }

    /// Return the exact authored bytes for one optional Section-12 block.
    ///
    /// Block meanings are entity-type specific. In particular, type 46 uses
    /// A/B/C/D for its skimmer drive, lateral correction, lift, and steering
    /// data, so callers must not infer semantics from [`Self::sub_a_pos`].
    pub fn subsection(&self, name: &str) -> Option<&[u8]> {
        self.subsections
            .iter()
            .find(|subsection| subsection.name == name)
            .map(|subsection| subsection.data.as_slice())
    }

    /// Decode the optional common Sub-A propulsion descriptor.
    pub fn sub_a_propulsion_descriptor(&self) -> Option<SubAPropulsionDescriptor> {
        let payload: &[u8; 6] = self.subsection("A")?.try_into().ok()?;
        Some(SubAPropulsionDescriptor {
            acceleration_raw: i16::from_le_bytes(payload[0..2].try_into().unwrap()),
            overspeed_correction_raw: i16::from_le_bytes(payload[2..4].try_into().unwrap()),
            target_speed_base_raw: i16::from_le_bytes(payload[4..6].try_into().unwrap()),
        })
    }

    /// Decode the optional common Sub-B lateral-velocity descriptor.
    pub fn sub_b_lateral_descriptor(&self) -> Option<SubBLateralDescriptor> {
        let payload: &[u8; 8] = self.subsection("B")?.try_into().ok()?;
        Some(SubBLateralDescriptor {
            projection_threshold_rate_raw: i32::from_le_bytes(payload[0..4].try_into().unwrap()),
            correction_rate_raw: i32::from_le_bytes(payload[4..8].try_into().unwrap()),
        })
    }

    /// Decode the optional common Sub-C lift/surface descriptor.
    pub fn sub_c_lift_descriptor(&self) -> Option<SubCLiftDescriptor> {
        let payload: &[u8; 16] = self.subsection("C")?.try_into().ok()?;
        Some(SubCLiftDescriptor {
            base_clearance_raw: i16::from_le_bytes(payload[0..2].try_into().unwrap()),
            lift_range_raw: i16::from_le_bytes(payload[2..4].try_into().unwrap()),
            strength_raw: i32::from_le_bytes(payload[4..8].try_into().unwrap()),
            near_boost_range_raw: i16::from_le_bytes(payload[8..10].try_into().unwrap()),
            damping_range_raw: i16::from_le_bytes(payload[10..12].try_into().unwrap()),
            surface_mode_raw: payload[12] as i8,
            offset_sample_raw: payload[13],
            reserved_at_0x0e: payload[14..16].try_into().unwrap(),
        })
    }

    /// Exact 104-byte Sub-G payload, or `None` when the block is absent or
    /// the authored length is not 104.
    pub fn sub_g_payload(&self) -> Option<[u8; 104]> {
        self.subsection("G")?.try_into().ok()
    }

    /// Exact two-byte Sub-K payload, or `None` when the block is absent or
    /// the authored length is not 2.
    pub fn sub_k_payload(&self) -> Option<[u8; 2]> {
        self.subsection("K")?.try_into().ok()
    }

    /// Exact six-byte Sub-L payload, or `None` when the block is absent or
    /// the authored length is not 6.
    pub fn sub_l_payload(&self) -> Option<[u8; 6]> {
        self.subsection("L")?.try_into().ok()
    }

    /// Decode the optional common Sub-D steering/avoidance descriptor.
    pub fn sub_d_steering_descriptor(&self) -> Option<SubDSteeringDescriptor> {
        let payload: &[u8; 12] = self.subsection("D")?.try_into().ok()?;
        Some(SubDSteeringDescriptor {
            steering_divisor_raw: i32::from_le_bytes(payload[0..4].try_into().unwrap()),
            couple_yaw_into_roll_raw: payload[4] as i8,
            enable_pitch_steering_raw: payload[5],
            forward_probe_raw: i16::from_le_bytes(payload[6..8].try_into().unwrap()),
            lateral_probe_raw: i16::from_le_bytes(payload[8..10].try_into().unwrap()),
            classifier_flags: payload[10],
            reserved_at_0x0b: payload[11],
        })
    }

    /// Decode the exact Sub-F layout, rejecting truncated/concatenated payloads.
    pub fn sub_f_swimming_descriptor(&self) -> Option<SubFSwimmingDescriptor> {
        let payload: &[u8; 14] = self.subsection("F")?.try_into().ok()?;
        Some(SubFSwimmingDescriptor {
            clearance_base_raw: i16::from_le_bytes(payload[0..2].try_into().unwrap()),
            clearance_random_span_raw: i16::from_le_bytes(payload[2..4].try_into().unwrap()),
            target_speed_base_raw: i16::from_le_bytes(payload[4..6].try_into().unwrap()),
            animation_mode_raw: payload[6],
            variable_selectors: std::array::from_fn(|index| payload[7 + index] as i8),
            reserved_at_0x0d: payload[13],
        })
    }

    /// Decode the optional status component attached through Sub-M.
    ///
    /// The retail gate reached by `FUN_00419750` and `FUN_00419B50` is the
    /// relocated pointer at type-record `+0x104` (field `0x41`, Sub-M). It is
    /// not the Sub-G pointer at `+0xE8`. Requiring the exact authored length
    /// keeps malformed or manually assembled records from being partially
    /// interpreted.
    pub fn status_component_descriptor(&self) -> Option<StatusComponentDescriptor> {
        let payload: &[u8; 18] = self.subsection("M")?.try_into().ok()?;
        Some(StatusComponentDescriptor {
            raw_word_at_0x00: u16::from_le_bytes(payload[0..2].try_into().unwrap()),
            variable_bindings: payload[2..8].try_into().unwrap(),
            raw_tail: payload[8..18].try_into().unwrap(),
        })
    }

    /// Decode the optional common projectile-emitter block attached as Sub-E.
    ///
    /// Requiring the exact authored payload length prevents a truncated
    /// diagnostic record from silently becoming a usable weapon descriptor.
    pub fn projectile_emitter_descriptor(&self) -> Option<ProjectileEmitterDescriptor> {
        let payload: &[u8; 28] = self.subsection("E")?.try_into().ok()?;
        Some(ProjectileEmitterDescriptor {
            projectile_method: u32::from_le_bytes(payload[0x00..0x04].try_into().unwrap()),
            random_interval_us: u32::from_le_bytes(payload[0x04..0x08].try_into().unwrap()),
            spread_raw: u16::from_le_bytes(payload[0x08..0x0a].try_into().unwrap()),
            aim_threshold_raw: u16::from_le_bytes(payload[0x0a..0x0c].try_into().unwrap()),
            speed_override_raw: i16::from_le_bytes(payload[0x0c..0x0e].try_into().unwrap()),
            target_axis_tolerance_raw: u16::from_le_bytes(payload[0x0e..0x10].try_into().unwrap()),
            sound_id: u16::from_le_bytes(payload[0x10..0x12].try_into().unwrap()),
            raw_word_at_0x12: u16::from_le_bytes(payload[0x12..0x14].try_into().unwrap()),
            alternate_emitter_raw: u16::from_le_bytes(payload[0x14..0x16].try_into().unwrap()),
            stochastic_gate_mode: payload[0x16],
            auxiliary_command: payload[0x17],
            variable_bindings: payload[0x18..0x1c].try_into().unwrap(),
        })
    }

    /// Decode the optional common actor-animation block attached as Sub-I.
    ///
    /// Requiring the exact eight-byte payload keeps compatibility metadata and
    /// truncated diagnostics from silently enabling a live animation writer.
    pub fn actor_animation_descriptor(&self) -> Option<ActorAnimationDescriptor> {
        let payload: &[u8; 8] = self.subsection("I")?.try_into().ok()?;
        Some(ActorAnimationDescriptor {
            capability_bit_3_sound_id: u16::from_le_bytes(payload[0x00..0x02].try_into().unwrap()),
            capability_mask_0x201_sound_id: u16::from_le_bytes(
                payload[0x02..0x04].try_into().unwrap(),
            ),
            attention_stop_sound_id: u16::from_le_bytes(payload[0x04..0x06].try_into().unwrap()),
            variable_binding: payload[0x06],
            frames_per_direction: payload[0x07],
        })
    }

    /// Decode Sub-H using the retail external-frame descriptor layout.
    ///
    /// `FUN_0041D2A0` constructs at most sixteen corresponding live records.
    /// Rejecting a different byte length prevents a truncated or concatenated
    /// diagnostic payload from silently becoming executable component data.
    pub fn sub_h_external_frame_descriptor(&self) -> Option<SubHExternalFrameDescriptor> {
        let payload = self.subsection("H")?;
        let header: &[u8; 4] = payload.get(..4)?.try_into().ok()?;
        let count = usize::from(u16::from_le_bytes(header[0..2].try_into().unwrap()));
        if count > 16 || payload.len() != 4 + count * 0x14 {
            return None;
        }

        let completion_sound_id = u16::from_le_bytes(header[2..4].try_into().unwrap());
        let mut records = Vec::with_capacity(count);
        for record in payload[4..].chunks_exact(0x14) {
            records.push(SubHExternalFrameRecord {
                resolver_flags_raw: u32::from_le_bytes(record[0x00..0x04].try_into().unwrap()),
                phase_rate_raw: i32::from_le_bytes(record[0x04..0x08].try_into().unwrap()),
                vertex_refs: [
                    u16::from_le_bytes(record[0x08..0x0a].try_into().unwrap()),
                    u16::from_le_bytes(record[0x0a..0x0c].try_into().unwrap()),
                    u16::from_le_bytes(record[0x0c..0x0e].try_into().unwrap()),
                ],
                axis_mode_raw: u16::from_le_bytes(record[0x0e..0x10].try_into().unwrap()),
                dependencies: record[0x10..0x14].try_into().unwrap(),
            });
        }

        Some(SubHExternalFrameDescriptor {
            completion_sound_id: (completion_sound_id != 0).then_some(completion_sound_id),
            records,
        })
    }

    /// Decode Sub-J using the retail/demo attachment descriptor layout.
    ///
    /// Both constructors reject more than eight slots. Requiring the exact
    /// count-derived byte length prevents a truncated or concatenated capture
    /// payload from becoming executable attachment state.
    pub fn sub_j_attachment_descriptor(&self) -> Option<SubJAttachmentDescriptor> {
        let payload = self.subsection("J")?;
        let (&count, tail) = payload.split_first()?;
        let count = usize::from(count);
        if count > 8 || payload.len() != 2 + count * 8 {
            return None;
        }

        let reserved_at_0x01 = *tail.first()?;
        let mut slots = Vec::with_capacity(count);
        for slot in payload[2..].chunks_exact(8) {
            slots.push(SubJAttachmentSlotDescriptor {
                policy_word_raw: u16::from_le_bytes(slot[0..2].try_into().unwrap()),
                local_offset_raw: [
                    i16::from_le_bytes(slot[2..4].try_into().unwrap()),
                    i16::from_le_bytes(slot[4..6].try_into().unwrap()),
                    i16::from_le_bytes(slot[6..8].try_into().unwrap()),
                ],
            });
        }
        Some(SubJAttachmentDescriptor {
            reserved_at_0x01,
            slots: slots.into_boxed_slice(),
        })
    }
}

fn subsection_size(name: &str, data: &[u8], offset: usize) -> usize {
    match name {
        "H" if offset + 2 <= data.len() => {
            4 + u16::from_le_bytes([data[offset], data[offset + 1]]) as usize * 0x14
        }
        "J" if offset < data.len() => 2 + data[offset] as usize * 8,
        _ => SUB_DEFS
            .iter()
            .find(|(candidate, _, _, _)| *candidate == name)
            .map_or(0, |(_, _, size, _)| *size),
    }
}

/// Result of parsing Section 12.
#[derive(Debug)]
pub struct CollisionModel {
    pub entries: Vec<CollisionEntry>,
}

/// Calculate total entry size using the handler's exact algorithm.
/// `data` is the full file data, `base` is the absolute entry start offset,
/// `buf_base` is the section data start offset.
fn calc_entry_size(data: &[u8], base: usize, buf_base: usize) -> (usize, Vec<(String, usize)>) {
    if base + ENTRY_BASE_SIZE > data.len() {
        return (ENTRY_BASE_SIZE, Vec::new());
    }

    let entry_data = &data[base..base + ENTRY_BASE_SIZE];
    let mut size = ENTRY_BASE_SIZE;
    let mut active = Vec::new();

    for &(name, field_idx, fixed_size, needs_align) in SUB_DEFS {
        let field_off = field_idx * 4;
        if field_off + 4 > entry_data.len() {
            continue;
        }
        let field_val = u32::from_le_bytes([
            entry_data[field_off],
            entry_data[field_off + 1],
            entry_data[field_off + 2],
            entry_data[field_off + 3],
        ]);
        if field_val == 0 {
            continue;
        }

        let sub_offset = buf_base + field_val as usize;
        active.push((name.to_string(), sub_offset));

        match name {
            "H" => {
                // Sub-H: align, then 4 + count * 0x14
                size = (size + 3) & !3;
                let sub_size = if sub_offset + 2 <= data.len() {
                    let count =
                        u16::from_le_bytes([data[sub_offset], data[sub_offset + 1]]) as usize;
                    4 + count * 0x14
                } else {
                    4
                };
                size += sub_size;
            }
            "J" => {
                // Sub-J: count byte + count * 8
                let sub_size = if sub_offset < data.len() {
                    let count = data[sub_offset] as usize;
                    2 + count * 8
                } else {
                    2
                };
                size += sub_size;
            }
            "A" => {
                size += 6;
            }
            "F" => {
                size += 14;
            }
            _ => {
                if needs_align {
                    size = (size + 3) & !3;
                }
                size += fixed_size;
            }
        }
    }

    (size, active)
}

/// Parse Sub-H face definitions.
fn parse_faces(data: &[u8], offset: usize) -> Vec<Face> {
    if offset + 2 > data.len() {
        return Vec::new();
    }
    let count = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
    let mut faces = Vec::with_capacity(count);

    for i in 0..count {
        let foff = offset + 4 + i * 0x14; // count u16 + external-frame sound u16
        if foff + 0x14 > data.len() {
            break;
        }
        let flags1 =
            u32::from_le_bytes([data[foff], data[foff + 1], data[foff + 2], data[foff + 3]]);
        let flags2 = u32::from_le_bytes([
            data[foff + 4],
            data[foff + 5],
            data[foff + 6],
            data[foff + 7],
        ]);
        let ref_a = u16::from_le_bytes([data[foff + 8], data[foff + 9]]);
        let ref_b = u16::from_le_bytes([data[foff + 10], data[foff + 11]]);
        let ref_c = u16::from_le_bytes([data[foff + 12], data[foff + 13]]);
        let v0 = data[foff + 0x10];
        let v1 = data[foff + 0x11];
        let v2 = data[foff + 0x12];
        let v3 = data[foff + 0x13];
        let is_triangle = v2 == v3;

        faces.push(Face {
            flags1,
            flags2,
            ref_a,
            ref_b,
            ref_c,
            verts: [v0, v1, v2, v3],
            is_triangle,
        });
    }

    faces
}

/// Parse the behavior-choice list. Each record is three dwords and a zero
/// `weight_rule_id` terminates the list.
fn parse_behavior_choices(data: &[u8], offset: usize) -> Vec<BehaviorChoice> {
    let mut choices = Vec::new();
    let mut pos = offset;
    for _ in 0..16 {
        // safety limit
        if pos + 12 > data.len() {
            break;
        }
        let weight_rule_id =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        if weight_rule_id == 0 {
            break;
        }
        let weight_multiplier =
            u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]);
        let behavior_class_id =
            u32::from_le_bytes([data[pos + 8], data[pos + 9], data[pos + 10], data[pos + 11]]);
        choices.push(BehaviorChoice {
            weight_rule_id,
            weight_multiplier,
            behavior_class_id,
        });
        pos += 12;
    }
    choices
}

/// Parse all Section 12 collision volume entries.
pub fn parse_collision(data: &[u8], sec_start: usize, sec_size: usize) -> Result<CollisionModel> {
    if sec_size < ENTRY_BASE_SIZE {
        return Err(V2kError::section(
            12,
            "section 12 data too short for even one entry",
        ));
    }

    let buf_base = sec_start;
    let mut entries = Vec::new();
    let mut pos = 0usize; // relative to buf_base

    while pos + ENTRY_BASE_SIZE <= sec_size {
        let abs_offset = buf_base + pos;
        if abs_offset + ENTRY_BASE_SIZE > data.len() {
            break;
        }

        let (mut entry_size, active_subs) = calc_entry_size(data, abs_offset, buf_base);

        // Read header fields
        let hdr = &data[abs_offset..abs_offset + ENTRY_BASE_SIZE];
        let field = |idx: usize| -> u32 {
            let off = idx * 4;
            u32::from_le_bytes([hdr[off], hdr[off + 1], hdr[off + 2], hdr[off + 3]])
        };

        // Parse the behavior-choice list at field 0x46.
        let choices_field_value = field(0x46);
        let behavior_choices = if choices_field_value != 0 {
            parse_behavior_choices(data, buf_base + choices_field_value as usize)
        } else {
            Vec::new()
        };

        // The choices and their terminator contribute to the record size.
        entry_size = (entry_size + 3) & !3;
        entry_size += (behavior_choices.len() + 1) * 0xC;

        // Parse faces if Sub-H present
        let mut faces = Vec::new();
        for (name, sub_off) in &active_subs {
            if name == "H" {
                faces = parse_faces(data, *sub_off);
            }
        }

        // Parse Sub-A position
        let mut sub_a_pos = None;
        for (name, sub_off) in &active_subs {
            if name == "A" && *sub_off + 6 <= data.len() {
                let x = i16::from_le_bytes([data[*sub_off], data[*sub_off + 1]]);
                let y = i16::from_le_bytes([data[*sub_off + 2], data[*sub_off + 3]]);
                let z = i16::from_le_bytes([data[*sub_off + 4], data[*sub_off + 5]]);
                sub_a_pos = Some((x, y, z));
            }
        }

        let subsections = active_subs
            .iter()
            .filter_map(|(name, sub_off)| {
                let len = subsection_size(name, data, *sub_off);
                let end = sub_off.checked_add(len)?;
                let payload = data.get(*sub_off..end)?;
                let field_index = SUB_DEFS
                    .iter()
                    .find(|(candidate, _, _, _)| candidate == name)
                    .map(|(_, field_index, _, _)| *field_index)?;
                Some(CollisionSubsection {
                    name: SUB_DEFS
                        .iter()
                        .find(|(candidate, _, _, _)| candidate == name)
                        .map(|(canonical, _, _, _)| *canonical)?,
                    field_index,
                    data: payload.to_vec(),
                })
            })
            .collect();
        let active_names: Vec<String> = active_subs.into_iter().map(|(n, _)| n).collect();
        let raw_header = hdr
            .try_into()
            .expect("header slice has fixed Section 12 size");

        let bounds = [field(3), field(4)];
        entries.push(CollisionEntry {
            index: entries.len(),
            offset: pos,
            size: entry_size,
            type_tag: field(0),
            scale: field(1),
            id_field: field(2),
            model_ids: [
                bounds[0] as u16,
                (bounds[0] >> 16) as u16,
                bounds[1] as u16,
                (bounds[1] >> 16) as u16,
            ],
            bounds,
            dims: [field(5), field(6), field(7), field(8), field(9), field(10)],
            initializer_state_flags_raw: field(0x30),
            flags: field(0x32),
            faces,
            behavior_choices,
            active_subs: active_names,
            raw_header,
            subsections,
            behavior_rule_ref: field(0x47),
            alternate_behavior_class_ref: field(0x49),
            sub_a_pos,
        });

        pos += entry_size;
    }

    Ok(CollisionModel { entries })
}

/// Convenience: parse Section 12 from raw section data (no absolute offset needed).
/// Use this when you have just the section bytes (from OvlSection.data).
pub fn parse_collision_section(section_data: &[u8]) -> Result<CollisionModel> {
    parse_collision(section_data, 0, section_data.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_collision_entry() -> CollisionEntry {
        CollisionEntry {
            index: 0,
            offset: 0,
            size: ENTRY_BASE_SIZE,
            type_tag: 0,
            scale: 0,
            id_field: 0,
            model_ids: [0; 4],
            bounds: [0; 2],
            dims: [0; 6],
            initializer_state_flags_raw: 0,
            flags: 0,
            faces: Vec::new(),
            behavior_choices: Vec::new(),
            active_subs: Vec::new(),
            raw_header: [0; ENTRY_BASE_SIZE],
            subsections: Vec::new(),
            behavior_rule_ref: 0,
            alternate_behavior_class_ref: 0,
            sub_a_pos: None,
        }
    }

    #[test]
    fn sub_f_descriptor_preserves_signed_words_selectors_and_reserved_byte() {
        let mut entry = empty_collision_entry();
        entry.subsections.push(CollisionSubsection {
            name: "F",
            field_index: 0x39,
            data: vec![
                0x00, 0xff, 0x00, 0x02, 0x80, 0x01, 2, 1, 0, 0xff, 0x80, 63, 2, 0xa5,
            ],
        });
        assert_eq!(
            entry.sub_f_swimming_descriptor(),
            Some(SubFSwimmingDescriptor {
                clearance_base_raw: -256,
                clearance_random_span_raw: 512,
                target_speed_base_raw: 384,
                animation_mode_raw: 2,
                variable_selectors: [1, 0, -1, -128, 63, 2],
                reserved_at_0x0d: 0xa5,
            })
        );
        entry.subsections[0].data.push(0);
        assert!(entry.sub_f_swimming_descriptor().is_none());
        entry.subsections[0].data.truncate(13);
        assert!(entry.sub_f_swimming_descriptor().is_none());
    }

    #[test]
    fn subsection_returns_exact_payload_or_none() {
        let mut entry = empty_collision_entry();
        entry.subsections.push(CollisionSubsection {
            name: "D",
            field_index: 0x37,
            data: vec![0x1c, 0, 0, 0],
        });

        assert_eq!(entry.subsection("D"), Some(&[0x1c, 0, 0, 0][..]));
        assert_eq!(entry.subsection("A"), None);
    }

    #[test]
    fn sub_a_propulsion_descriptor_preserves_all_signed_words() {
        let mut entry = empty_collision_entry();
        entry.subsections.push(CollisionSubsection {
            name: "A",
            field_index: 0x34,
            data: [1_500_i16, -3_000_i16, 250_i16]
                .into_iter()
                .flat_map(i16::to_le_bytes)
                .collect(),
        });

        assert_eq!(
            entry.sub_a_propulsion_descriptor(),
            Some(SubAPropulsionDescriptor {
                acceleration_raw: 1_500,
                overspeed_correction_raw: -3_000,
                target_speed_base_raw: 250,
            })
        );
    }

    #[test]
    fn sub_b_lateral_descriptor_preserves_both_signed_dwords() {
        let mut entry = empty_collision_entry();
        entry.subsections.push(CollisionSubsection {
            name: "B",
            field_index: 0x35,
            data: [10_000_i32, -1_000_i32]
                .into_iter()
                .flat_map(i32::to_le_bytes)
                .collect(),
        });

        assert_eq!(
            entry.sub_b_lateral_descriptor(),
            Some(SubBLateralDescriptor {
                projection_threshold_rate_raw: 10_000,
                correction_rate_raw: -1_000,
            })
        );
    }

    #[test]
    fn sub_c_lift_descriptor_preserves_signed_surface_mode_and_tail() {
        let mut entry = empty_collision_entry();
        let mut data = Vec::new();
        data.extend_from_slice(&150_i16.to_le_bytes());
        data.extend_from_slice(&125_i16.to_le_bytes());
        data.extend_from_slice(&0x0090_0000_i32.to_le_bytes());
        data.extend_from_slice(&100_i16.to_le_bytes());
        data.extend_from_slice(&200_i16.to_le_bytes());
        data.extend_from_slice(&[0xfe, 1, 0x34, 0x12]);
        entry.subsections.push(CollisionSubsection {
            name: "C",
            field_index: 0x36,
            data,
        });

        assert_eq!(
            entry.sub_c_lift_descriptor(),
            Some(SubCLiftDescriptor {
                base_clearance_raw: 150,
                lift_range_raw: 125,
                strength_raw: 0x0090_0000,
                near_boost_range_raw: 100,
                damping_range_raw: 200,
                surface_mode_raw: -2,
                offset_sample_raw: 1,
                reserved_at_0x0e: [0x34, 0x12],
            })
        );
    }

    #[test]
    fn sub_d_steering_descriptor_preserves_exact_type9_layout() {
        let mut entry = empty_collision_entry();
        entry.subsections.push(CollisionSubsection {
            name: "D",
            field_index: 0x37,
            data: vec![
                0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x40, 0x00, 0x17, 0x00,
            ],
        });

        assert_eq!(
            entry.sub_d_steering_descriptor(),
            Some(SubDSteeringDescriptor {
                steering_divisor_raw: 20,
                couple_yaw_into_roll_raw: 0,
                enable_pitch_steering_raw: 0,
                forward_probe_raw: 128,
                lateral_probe_raw: 64,
                classifier_flags: 0x17,
                reserved_at_0x0b: 0,
            })
        );
    }

    #[test]
    fn common_axis_descriptor_preserves_both_words_and_signed_limit() {
        let mut entry = empty_collision_entry();
        entry.flags = (-3840_i32) as u32;
        entry.raw_header[0xCC..0xD0].copy_from_slice(&0x0000_0084_u32.to_le_bytes());

        assert_eq!(
            entry.common_axis_descriptor(),
            CommonAxisDescriptor {
                strict_axis_limit_raw: -3840,
                raw_word_at_0x04: 0x84,
            }
        );
    }

    #[test]
    fn status_component_descriptor_decodes_only_exact_sub_m_payloads() {
        let mut entry = empty_collision_entry();
        entry.subsections.push(CollisionSubsection {
            name: "M",
            field_index: 0x41,
            data: vec![
                0x08, 0x00, 0x04, 0x03, 0x00, 0x02, 0x01, 0x00, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
                0x11, 0x22, 0x33, 0x44,
            ],
        });

        assert_eq!(
            entry.status_component_descriptor(),
            Some(StatusComponentDescriptor {
                raw_word_at_0x00: 8,
                variable_bindings: [4, 3, 0, 2, 1, 0],
                raw_tail: [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x11, 0x22, 0x33, 0x44],
            })
        );

        entry.subsections[0].data.pop();
        assert_eq!(entry.status_component_descriptor(), None);
        entry.subsections[0].data.extend_from_slice(&[0x44, 0x55]);
        assert_eq!(entry.status_component_descriptor(), None);
    }

    #[test]
    fn status_component_descriptor_never_decodes_sub_g() {
        let mut entry = empty_collision_entry();
        entry.subsections.push(CollisionSubsection {
            name: "G",
            field_index: 0x3a,
            data: vec![0; 104],
        });

        assert_eq!(entry.status_component_descriptor(), None);
    }

    #[test]
    fn projectile_emitter_descriptor_decodes_only_exact_sub_e_payloads() {
        let mut entry = empty_collision_entry();
        entry.subsections.push(CollisionSubsection {
            name: "E",
            field_index: 0x38,
            data: vec![
                0x1e, 0, 0, 0, 0xb0, 0x71, 0x0b, 0, 0x64, 0, 0x80, 0x3e, 0, 0, 0, 6, 0x46, 0, 0x96,
                0, 0, 0, 0, 0, 1, 2, 3, 4,
            ],
        });

        assert_eq!(
            entry.projectile_emitter_descriptor(),
            Some(ProjectileEmitterDescriptor {
                projectile_method: 30,
                random_interval_us: 750_000,
                spread_raw: 100,
                aim_threshold_raw: 16_000,
                speed_override_raw: 0,
                target_axis_tolerance_raw: 0x0600,
                sound_id: 70,
                raw_word_at_0x12: 150,
                alternate_emitter_raw: 0,
                stochastic_gate_mode: 0,
                auxiliary_command: 0,
                variable_bindings: [1, 2, 3, 4],
            })
        );

        entry.subsections[0].data.pop();
        assert_eq!(entry.projectile_emitter_descriptor(), None);
        entry.subsections[0].data.extend_from_slice(&[0, 0]);
        assert_eq!(entry.projectile_emitter_descriptor(), None);
    }

    #[test]
    fn projectile_emitter_descriptor_never_decodes_a_different_subsection() {
        let mut entry = empty_collision_entry();
        entry.subsections.push(CollisionSubsection {
            name: "F",
            field_index: 0x39,
            data: vec![0; 28],
        });

        assert_eq!(entry.projectile_emitter_descriptor(), None);
    }

    #[test]
    fn actor_animation_descriptor_decodes_only_exact_sub_i_payloads() {
        let mut entry = empty_collision_entry();
        entry.subsections.push(CollisionSubsection {
            name: "I",
            field_index: 0x3c,
            data: vec![0x48, 0, 0, 0, 0x48, 0, 1, 4],
        });

        assert_eq!(
            entry.actor_animation_descriptor(),
            Some(ActorAnimationDescriptor {
                capability_bit_3_sound_id: 72,
                capability_mask_0x201_sound_id: 0,
                attention_stop_sound_id: 72,
                variable_binding: 1,
                frames_per_direction: 4,
            })
        );

        entry.subsections[0].data.pop();
        assert_eq!(entry.actor_animation_descriptor(), None);
        entry.subsections[0].data.extend_from_slice(&[4, 0]);
        assert_eq!(entry.actor_animation_descriptor(), None);
    }

    #[test]
    fn actor_animation_descriptor_never_decodes_a_different_subsection() {
        let mut entry = empty_collision_entry();
        entry.subsections.push(CollisionSubsection {
            name: "B",
            field_index: 0x35,
            data: vec![0x48, 0, 0, 0, 0x48, 0, 1, 4],
        });

        assert_eq!(entry.actor_animation_descriptor(), None);
    }

    #[test]
    fn sub_h_external_frame_descriptor_preserves_header_and_record_fields() {
        let mut entry = empty_collision_entry();
        let mut payload = Vec::from([1, 0, 0x48, 0]);
        payload.extend_from_slice(&[
            0x78, 0x56, 0x34, 0x12, // resolver flags
            0x00, 0x00, 0xff, 0xff, // signed phase rate
            3, 0, 5, 0, 7, 0, // model-record references
            4, 0, // axis/plane mode
            1, 2, 3, 4, // dependency records
        ]);
        entry.subsections.push(CollisionSubsection {
            name: "H",
            field_index: 0x3b,
            data: payload,
        });

        assert_eq!(
            entry.sub_h_external_frame_descriptor(),
            Some(SubHExternalFrameDescriptor {
                completion_sound_id: Some(72),
                records: vec![SubHExternalFrameRecord {
                    resolver_flags_raw: 0x1234_5678,
                    phase_rate_raw: -65_536,
                    vertex_refs: [3, 5, 7],
                    axis_mode_raw: 4,
                    dependencies: [1, 2, 3, 4],
                }],
            })
        );
    }

    #[test]
    fn sub_h_external_frame_descriptor_rejects_bad_lengths_and_large_counts() {
        let mut entry = empty_collision_entry();
        entry.subsections.push(CollisionSubsection {
            name: "H",
            field_index: 0x3b,
            data: vec![1, 0, 0, 0],
        });
        assert_eq!(entry.sub_h_external_frame_descriptor(), None);

        entry.subsections[0].data = vec![17, 0, 0, 0];
        entry.subsections[0].data.resize(4 + 17 * 0x14, 0);
        assert_eq!(entry.sub_h_external_frame_descriptor(), None);
    }

    #[test]
    fn sub_j_attachment_descriptor_decodes_exact_type17_payload() {
        let mut entry = empty_collision_entry();
        entry.subsections.push(CollisionSubsection {
            name: "J",
            field_index: 0x3d,
            data: vec![1, 0, 1, 0, 0, 0, 10, 0, 110, 0],
        });

        assert_eq!(
            entry.sub_j_attachment_descriptor(),
            Some(SubJAttachmentDescriptor {
                reserved_at_0x01: 0,
                slots: vec![SubJAttachmentSlotDescriptor {
                    policy_word_raw: 1,
                    local_offset_raw: [0, 10, 110],
                }]
                .into_boxed_slice(),
            })
        );
    }

    #[test]
    fn sub_j_attachment_descriptor_rejects_bad_lengths_and_large_counts() {
        let mut entry = empty_collision_entry();
        entry.subsections.push(CollisionSubsection {
            name: "J",
            field_index: 0x3d,
            data: vec![1, 0, 1, 0, 0, 0, 10, 0, 110],
        });
        assert_eq!(entry.sub_j_attachment_descriptor(), None);

        entry.subsections[0].data.push(0);
        entry.subsections[0].data.push(0xff);
        assert_eq!(entry.sub_j_attachment_descriptor(), None);

        entry.subsections[0].data = vec![9, 0];
        entry.subsections[0].data.resize(2 + 9 * 8, 0);
        assert_eq!(entry.sub_j_attachment_descriptor(), None);
    }

    #[test]
    fn sub_j_attachment_descriptor_preserves_header_policy_and_signed_offsets() {
        let mut entry = empty_collision_entry();
        let mut data = vec![1, 0x7e];
        data.extend_from_slice(&0xabcd_u16.to_le_bytes());
        data.extend_from_slice(&(-1_i16).to_le_bytes());
        data.extend_from_slice(&i16::MIN.to_le_bytes());
        data.extend_from_slice(&i16::MAX.to_le_bytes());
        entry.subsections.push(CollisionSubsection {
            name: "J",
            field_index: 0x3d,
            data,
        });

        assert_eq!(
            entry.sub_j_attachment_descriptor(),
            Some(SubJAttachmentDescriptor {
                reserved_at_0x01: 0x7e,
                slots: vec![SubJAttachmentSlotDescriptor {
                    policy_word_raw: 0xabcd,
                    local_offset_raw: [-1, i16::MIN, i16::MAX],
                }]
                .into_boxed_slice(),
            })
        );
    }

    #[test]
    fn fullscreen_map_icon_uses_authored_header_selector() {
        let mut entry = empty_collision_entry();
        assert_eq!(entry.fullscreen_map_icon_sprite_id(), None);

        entry.raw_header[0x70..0x72].copy_from_slice(&0x01dd_u16.to_le_bytes());
        assert_eq!(entry.fullscreen_map_icon_sprite_id(), Some(0x01dd));
    }

    #[test]
    fn damage_lifecycle_sounds_keep_their_distinct_authored_selectors() {
        let mut entry = empty_collision_entry();
        assert_eq!(entry.accepted_hit_presentation_sound_id(), None);
        assert_eq!(entry.infected_model_presentation_sound_id(), None);
        assert_eq!(entry.death_sound_id(), None);
        assert_eq!(entry.generic_hit_sound_id(), None);
        assert_eq!(entry.constructor_sound_attachment_id(), None);

        entry.raw_header[0x80..0x82].copy_from_slice(&7_u16.to_le_bytes());
        entry.raw_header[0x82..0x84].copy_from_slice(&92_u16.to_le_bytes());
        entry.raw_header[0x90..0x92].copy_from_slice(&8_u16.to_le_bytes());
        entry.raw_header[0x98..0x9a].copy_from_slice(&9_u16.to_le_bytes());
        entry.raw_header[0xB4..0xB6].copy_from_slice(&10_u16.to_le_bytes());

        assert_eq!(entry.accepted_hit_presentation_sound_id(), Some(7));
        assert_eq!(entry.infected_model_presentation_sound_id(), Some(92));
        assert_eq!(entry.death_sound_id(), Some(8));
        assert_eq!(entry.generic_hit_sound_id(), Some(9));
        assert_eq!(entry.constructor_sound_attachment_id(), Some(10));
    }

    #[test]
    fn run_away_audio_keeps_its_independent_selector_and_raw_period() {
        let mut entry = empty_collision_entry();
        assert_eq!(entry.run_away_optional_sound_id(), None);
        assert_eq!(entry.run_away_sound_period_raw(), 0);

        entry.raw_header[0x9E..0xA0].copy_from_slice(&85_u16.to_le_bytes());
        entry.raw_header[0xAC..0xB0].copy_from_slice(&0xFEDC_BA98_u32.to_le_bytes());

        assert_eq!(entry.run_away_optional_sound_id(), Some(85));
        assert_eq!(entry.run_away_sound_period_raw(), 0xFEDC_BA98);
    }

    #[test]
    fn search_attack_ade0_audio_keeps_prelude_aim_selector_and_signed_period() {
        let mut entry = empty_collision_entry();
        assert_eq!(entry.search_attack_optional_prelude_sound_id(), None);
        assert_eq!(entry.search_attack_aim_sound_id(), None);
        assert_eq!(entry.search_attack_aim_sound_period_raw(), 0);

        entry.raw_header[0x9A..0x9C].copy_from_slice(&11_u16.to_le_bytes());
        entry.raw_header[0x9C..0x9E].copy_from_slice(&70_u16.to_le_bytes());
        entry.raw_header[0xA8..0xAC].copy_from_slice(&(-0x0400_i32).to_le_bytes());

        assert_eq!(entry.search_attack_optional_prelude_sound_id(), Some(11));
        assert_eq!(entry.search_attack_aim_sound_id(), Some(70));
        assert_eq!(entry.search_attack_aim_sound_period_raw(), -0x0400);
    }

    #[test]
    fn terrain_contact_lifetime_keeps_the_unsigned_word_at_0xa2() {
        let mut entry = empty_collision_entry();
        entry.raw_header[0xA2..0xA4].copy_from_slice(&0xFEDC_u16.to_le_bytes());
        assert_eq!(entry.terrain_contact_task_lifetime_ms(), 0xFEDC);
    }

    #[test]
    fn too_small() {
        assert!(parse_collision_section(&[0u8; 100]).is_err());
    }

    #[test]
    fn empty_entry() {
        // A minimal entry with no sub-sections and no behavior choices:
        // All zeros for 0x128 bytes. Field 0x46 = 0, followed by one terminator.
        // Size = 0x128 (aligned) + (0+1)*0xC = 0x128 + 0xC = 308.
        let data = vec![0u8; ENTRY_BASE_SIZE + 0x0C];
        let model = parse_collision_section(&data).unwrap();
        assert_eq!(model.entries.len(), 1);
        assert!(model.entries[0].faces.is_empty());
        assert!(model.entries[0].behavior_choices.is_empty());
    }

    #[test]
    fn behavior_choices_preserve_all_three_authored_fields() {
        let mut data = vec![0u8; ENTRY_BASE_SIZE + 0x18];
        data[0x46 * 4..0x46 * 4 + 4].copy_from_slice(&(ENTRY_BASE_SIZE as u32).to_le_bytes());
        data[ENTRY_BASE_SIZE..ENTRY_BASE_SIZE + 12]
            .copy_from_slice(&[7, 0, 0, 0, 10, 0, 0, 0, 41, 0, 0, 0]);

        let model = parse_collision_section(&data).unwrap();
        assert_eq!(
            model.entries[0].behavior_choices,
            vec![BehaviorChoice {
                weight_rule_id: 7,
                weight_multiplier: 10,
                behavior_class_id: 41,
            }]
        );
    }
}
