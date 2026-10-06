//! Shared Sub-G allocation and the exact normal terrain-derived flight branch.
//!
//! Retail keeps a 0x44-byte Sub-G allocation. `FUN_0041B8C0` fills it, copies
//! five descriptor bytes at `+0x30` stride 12 into `+0x28..+0x2C`, consumes
//! one word for dword `+0x38`, then writes `+0x1C = 0x5000`, `+0x1E = 0`, and
//! `+0x41 = 1`. Shared `FUN_00406070` later overwrites `+0x3F`, `+0x38`,
//! `+0x40`, `+0x24`, `+0x20`, and `+0x3C`.
//!
//! The normal `FUN_0041A690` path (`+0x3F/+0x40/+0x42/+0x43 == 0`) is also
//! shared by Type-13. It publishes the seven bound animation words, advances
//! `FUN_0041AC40`'s terrain-derived attitude/rate controller, applies the
//! Type-13 self-righting words, and finishes with `FUN_0041B210`'s retained-
//! basis force projection. The tumble mode with +3F nonzero and +40 zero
//! clears the two forces and preserves animation/attitude state. Other modes
//! remain explicit gates.

use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::entity_collision_state::RetailRuntimeValue;
use crate::hover::q31_mul;
use crate::vtol::{
    plan_vtol_terrain_derived_attitude_raw, vtol_terrain_projected_probe_raw,
    VtolTerrainDerivedAttitudeRequest,
};
use v2k_formats::fixed_math::retail_sine_q15;
use v2k_formats::terrain::{wave_surface_raw, TerrainGrid};

pub const SUB_G_1B8C0_ALLOCATION_SIZE: usize = 0x44;
pub const SUB_G_1B8C0_WORD_AT_0X1C: u16 = 0x5000;
pub const SUB_G_1B8C0_ANGLE_RAW_AT_0X1E: i16 = 0;
pub const SUB_G_1B8C0_BYTE_AT_0X41: u8 = 1;

/// Live Sub-G words written by the birth constructor, shared setup suffix,
/// and the authenticated normal `FUN_0041A690` frame path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubG06070RuntimeState {
    word_at_0x1c: RetailRuntimeValue<u16>,
    angle_raw_at_0x1e: RetailRuntimeValue<i16>,
    source_raw_at_0x20: RetailRuntimeValue<u32>,
    accumulator_raw_at_0x24: RetailRuntimeValue<u32>,
    table_bytes_at_0x28: RetailRuntimeValue<[u8; 5]>,
    decaying_output_raw_at_0x30: RetailRuntimeValue<i32>,
    rate_raw_at_0x34: RetailRuntimeValue<i32>,
    randomized_target_raw_at_0x38: RetailRuntimeValue<i32>,
    invert_target_at_0x3c: RetailRuntimeValue<u8>,
    clearance_flag_at_0x3d: RetailRuntimeValue<u8>,
    audio_gate_at_0x3e: RetailRuntimeValue<u8>,
    mode_at_0x3f: RetailRuntimeValue<u8>,
    mode_at_0x40: RetailRuntimeValue<u8>,
    byte_at_0x41: RetailRuntimeValue<u8>,
    terrain_attitude_mode_at_0x42: RetailRuntimeValue<u8>,
    alternate_basis_mode_at_0x43: RetailRuntimeValue<u8>,
    /// Values last written through the seven constructor-resolved pointers.
    /// These words live in the entity variable bank, not inside the 0x44-byte
    /// allocation, but retaining them here keeps their Sub-G writer atomic.
    animation_outputs_raw: RetailRuntimeValue<[i16; 7]>,
}

impl SubG06070RuntimeState {
    /// 014E0's direction propagation precedes Sub-D and the G callback.
    pub(crate) fn apply_direction_reverse_write(&mut self, reverse: bool) {
        self.invert_target_at_0x3c = RetailRuntimeValue::Known(u8::from(reverse));
    }
    /// `404360 -> 1B970(..., 1)`: disable the normal flight controller for
    /// the Tumble To Death task. Only byte +3F changes; 06070's prior suffix
    /// and the allocated animation/rate state retain their own chronology.
    pub fn enter_tumble(&mut self) {
        self.mode_at_0x3f = RetailRuntimeValue::Known(1);
    }

    /// Direct018A0's G phase while tumbling: 41A9CB -> 41AA36 writes only
    /// the zero force words. Its B210 call needs no terrain/basis/mass data
    /// because both force predicates now fail. Leave other runtime evidence
    /// untouched, including fields the normal-flight branch would require.
    pub fn advance_tumble_mode(&mut self) -> Result<(), Type13SubGFrameBlock> {
        let (RetailRuntimeValue::Known(mode), RetailRuntimeValue::Known(override_mode)) =
            (self.mode_at_0x3f, self.mode_at_0x40)
        else {
            return Err(Type13SubGFrameBlock::RuntimeUnavailable);
        };
        if mode == 0 || override_mode != 0 {
            return Err(Type13SubGFrameBlock::UnsupportedMode);
        }
        self.source_raw_at_0x20 = RetailRuntimeValue::Known(0);
        self.accumulator_raw_at_0x24 = RetailRuntimeValue::Known(0);
        Ok(())
    }

    /// Existing allocation whose constructor words have not been written.
    pub const fn pending() -> Self {
        Self {
            word_at_0x1c: RetailRuntimeValue::Unresolved,
            angle_raw_at_0x1e: RetailRuntimeValue::Unresolved,
            source_raw_at_0x20: RetailRuntimeValue::Unresolved,
            accumulator_raw_at_0x24: RetailRuntimeValue::Unresolved,
            table_bytes_at_0x28: RetailRuntimeValue::Unresolved,
            decaying_output_raw_at_0x30: RetailRuntimeValue::Unresolved,
            rate_raw_at_0x34: RetailRuntimeValue::Unresolved,
            randomized_target_raw_at_0x38: RetailRuntimeValue::Unresolved,
            invert_target_at_0x3c: RetailRuntimeValue::Unresolved,
            clearance_flag_at_0x3d: RetailRuntimeValue::Unresolved,
            audio_gate_at_0x3e: RetailRuntimeValue::Unresolved,
            mode_at_0x3f: RetailRuntimeValue::Unresolved,
            mode_at_0x40: RetailRuntimeValue::Unresolved,
            byte_at_0x41: RetailRuntimeValue::Unresolved,
            terrain_attitude_mode_at_0x42: RetailRuntimeValue::Unresolved,
            alternate_basis_mode_at_0x43: RetailRuntimeValue::Unresolved,
            animation_outputs_raw: RetailRuntimeValue::Unresolved,
        }
    }

    /// Successful `FUN_0041B8C0` after the 0x44-byte allocation.
    ///
    /// `Mem_Fill` zeroes the block first. The five table bytes come from
    /// descriptor `+0x30` at stride 12. One shared word seeds `+0x38` from
    /// signed descriptor `+0x0C`.
    pub fn from_1b8c0_constructor(descriptor: &[u8; 104], random_sample_low16: u16) -> Self {
        let base_raw = i16::from_le_bytes([descriptor[0x0C], descriptor[0x0D]]);
        let randomized_target_raw_at_0x38 =
            i32::from(base_raw).wrapping_add(i32::from(random_sample_low16 >> 8));
        Self {
            word_at_0x1c: RetailRuntimeValue::Known(SUB_G_1B8C0_WORD_AT_0X1C),
            angle_raw_at_0x1e: RetailRuntimeValue::Known(SUB_G_1B8C0_ANGLE_RAW_AT_0X1E),
            source_raw_at_0x20: RetailRuntimeValue::Known(0),
            accumulator_raw_at_0x24: RetailRuntimeValue::Known(0),
            table_bytes_at_0x28: RetailRuntimeValue::Known([
                descriptor[0x30],
                descriptor[0x3C],
                descriptor[0x48],
                descriptor[0x54],
                descriptor[0x60],
            ]),
            decaying_output_raw_at_0x30: RetailRuntimeValue::Known(0),
            rate_raw_at_0x34: RetailRuntimeValue::Known(0),
            randomized_target_raw_at_0x38: RetailRuntimeValue::Known(randomized_target_raw_at_0x38),
            invert_target_at_0x3c: RetailRuntimeValue::Known(0),
            clearance_flag_at_0x3d: RetailRuntimeValue::Known(0),
            audio_gate_at_0x3e: RetailRuntimeValue::Known(0),
            mode_at_0x3f: RetailRuntimeValue::Known(0),
            mode_at_0x40: RetailRuntimeValue::Known(0),
            byte_at_0x41: RetailRuntimeValue::Known(SUB_G_1B8C0_BYTE_AT_0X41),
            terrain_attitude_mode_at_0x42: RetailRuntimeValue::Known(0),
            alternate_basis_mode_at_0x43: RetailRuntimeValue::Known(0),
            animation_outputs_raw: RetailRuntimeValue::Known([0; 7]),
        }
    }

    /// Apply the type-13 B6C0 / class-12 shared Sub-G initializer branch.
    ///
    /// Order is `FUN_0041B970(..., 0)`, `FUN_0041B940(..., 0)` already folded
    /// into `randomized_target_raw`, `FUN_0041B980(..., 0)`, then
    /// `FUN_00424380(..., 0)`. Birth constructor fields are preserved.
    pub fn apply_shared_06070_sub_g_branch(
        &mut self,
        randomized_target_raw_at_0x38: i32,
        source_raw_at_0x00: u32,
    ) {
        self.mode_at_0x3f = RetailRuntimeValue::Known(0);
        self.randomized_target_raw_at_0x38 =
            RetailRuntimeValue::Known(randomized_target_raw_at_0x38);
        self.mode_at_0x40 = RetailRuntimeValue::Known(0);
        self.accumulator_raw_at_0x24 = RetailRuntimeValue::Known(0);
        self.source_raw_at_0x20 = RetailRuntimeValue::Known(source_raw_at_0x00);
        self.invert_target_at_0x3c = RetailRuntimeValue::Known(0);
    }

    pub const fn word_at_0x1c(self) -> RetailRuntimeValue<u16> {
        self.word_at_0x1c
    }

    pub const fn angle_raw_at_0x1e(self) -> RetailRuntimeValue<i16> {
        self.angle_raw_at_0x1e
    }

    pub const fn source_raw_at_0x20(self) -> RetailRuntimeValue<u32> {
        self.source_raw_at_0x20
    }

    pub const fn accumulator_raw_at_0x24(self) -> RetailRuntimeValue<u32> {
        self.accumulator_raw_at_0x24
    }

    pub const fn table_bytes_at_0x28(self) -> RetailRuntimeValue<[u8; 5]> {
        self.table_bytes_at_0x28
    }

    pub const fn randomized_target_raw_at_0x38(self) -> RetailRuntimeValue<i32> {
        self.randomized_target_raw_at_0x38
    }

    pub const fn rate_raw_at_0x34(self) -> RetailRuntimeValue<i32> {
        self.rate_raw_at_0x34
    }

    pub const fn decaying_output_raw_at_0x30(self) -> RetailRuntimeValue<i32> {
        self.decaying_output_raw_at_0x30
    }

    pub const fn invert_target_at_0x3c(self) -> RetailRuntimeValue<u8> {
        self.invert_target_at_0x3c
    }

    pub const fn mode_at_0x3f(self) -> RetailRuntimeValue<u8> {
        self.mode_at_0x3f
    }

    pub const fn mode_at_0x40(self) -> RetailRuntimeValue<u8> {
        self.mode_at_0x40
    }

    pub const fn byte_at_0x41(self) -> RetailRuntimeValue<u8> {
        self.byte_at_0x41
    }

    pub const fn clearance_flag_at_0x3d(self) -> RetailRuntimeValue<u8> {
        self.clearance_flag_at_0x3d
    }

    pub const fn audio_gate_at_0x3e(self) -> RetailRuntimeValue<u8> {
        self.audio_gate_at_0x3e
    }

    pub const fn animation_outputs_raw(self) -> RetailRuntimeValue<[i16; 7]> {
        self.animation_outputs_raw
    }

    /// Collision/event helper `FUN_0041B9D0` writes this pulse independently
    /// of the per-frame Sub-G callback. Publication still happens before decay
    /// on the next `FUN_0041AA60` visit.
    pub fn apply_collision_pulse_1b9d0(&mut self) {
        self.decaying_output_raw_at_0x30 = RetailRuntimeValue::Known(0x3000);
    }
}

/// Exact Sub-G branch currently authenticated for the Intro2 Type-13 owner.
///
/// The selected model extent is Section-8 header `+0x08` without the half
/// applied later by `FUN_0041B210`. `retained_body_basis` is the matrix read
/// before Sub-D changes the entity angle words.
#[derive(Debug, Clone, Copy)]
pub struct Type13SubGFrameRequest<'a> {
    pub descriptor: &'a [u8; 104],
    pub runtime: SubG06070RuntimeState,
    pub reverse_write: Option<bool>,
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub pitch_raw: i16,
    pub roll_raw: u16,
    pub retained_body_basis: Type9BodyBasis,
    pub active_model_extent_raw: u16,
    pub self_mass_raw: u16,
    pub attached_cargo_mass: u32,
    pub capability_flags: u32,
    pub terrain: &'a TerrainGrid,
    pub retail_tick: u32,
    pub elapsed_micros: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type13SubGSound {
    pub sound_id: u16,
    pub position_raw: [i16; 3],
    pub rate_q16: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type13SubGFrameOutcome {
    pub runtime: SubG06070RuntimeState,
    pub velocity_raw: [i16; 3],
    pub pitch_raw: i16,
    pub roll_raw: u16,
    pub sound: Option<Type13SubGSound>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type13SubGFrameBlock {
    RuntimeUnavailable,
    UnsupportedMode,
    PlayerProjectionUnsupported,
    MalformedDescriptor,
    ZeroTotalMass,
}

/// Apply shared `FUN_0041A690`: the normal AA60/AC40/self-righting/B210
/// flight path or the tumble path's two force clears and zero-force B210.
///
/// All state is copied into the request and returned as one outcome, so a
/// caller can keep task/component/entity publication atomic when any evidence
/// gate fails. No RNG is consumed by this per-frame path.
pub fn plan_type13_sub_g_frame(
    request: Type13SubGFrameRequest<'_>,
) -> Result<Type13SubGFrameOutcome, Type13SubGFrameBlock> {
    plan_sub_g_frame_with_prefix(request).map_err(|failure| failure.reason)
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SubGFramePrefix {
    pub runtime: Option<SubG06070RuntimeState>,
    pub velocity_raw: Option<[i16; 3]>,
    pub pitch_raw: Option<i16>,
    pub roll_raw: Option<u16>,
    pub sound: Option<Type13SubGSound>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SubGFrameFailure {
    pub reason: Type13SubGFrameBlock,
    pub prefix: SubGFramePrefix,
}

/// Live users retain 1AA60's animation/audio and AC40's attitude writes even
/// if the later B210 mass/divisor boundary rejects its force projection.
pub(crate) fn plan_sub_g_frame_with_prefix(
    request: Type13SubGFrameRequest<'_>,
) -> Result<Type13SubGFrameOutcome, SubGFrameFailure> {
    let mut prefix = SubGFramePrefix::default();
    let result = (|| {
        let Type13SubGFrameRequest {
            descriptor,
            mut runtime,
            reverse_write,
            position_raw,
            mut velocity_raw,
            pitch_raw,
            roll_raw,
            retained_body_basis,
            active_model_extent_raw,
            self_mass_raw,
            attached_cargo_mass,
            capability_flags,
            terrain,
            retail_tick,
            elapsed_micros,
        } = request;

        // 41A6AA tests +3F before reading any normal-controller state. Its
        // nonzero branch tests +40 at41A9CB, then41AA36 clears dwords+20/+24.
        // B210's force predicates now both fail: no terrain, mass division,
        // retained basis, animation, sound, velocity or Euler write occurs.
        let (RetailRuntimeValue::Known(mode), RetailRuntimeValue::Known(override_mode)) =
            (runtime.mode_at_0x3f, runtime.mode_at_0x40)
        else {
            return Err(Type13SubGFrameBlock::RuntimeUnavailable);
        };
        if mode != 0 && override_mode == 0 {
            if let Some(reverse) = reverse_write {
                // This write belongs to the preceding common-mover prelude, not
                // 1A690. Direct018A0 callers supply None and retain this byte.
                runtime.invert_target_at_0x3c = RetailRuntimeValue::Known(u8::from(reverse));
            }
            runtime.advance_tumble_mode()?;
            return Ok(Type13SubGFrameOutcome {
                runtime,
                velocity_raw,
                pitch_raw,
                roll_raw,
                sound: None,
            });
        }

        let (
            RetailRuntimeValue::Known(mut phases),
            RetailRuntimeValue::Known(mut decaying_output_raw),
            RetailRuntimeValue::Known(prior_rate_raw),
            RetailRuntimeValue::Known(target_clearance_raw),
            RetailRuntimeValue::Known(mut invert_target_raw),
            RetailRuntimeValue::Known(prior_clearance_flag_raw),
            RetailRuntimeValue::Known(mut audio_gate_raw),
            RetailRuntimeValue::Known(mode_at_0x3f),
            RetailRuntimeValue::Known(mode_at_0x40),
            RetailRuntimeValue::Known(self_righting_mode),
            RetailRuntimeValue::Known(terrain_attitude_mode),
            RetailRuntimeValue::Known(alternate_basis_mode),
            RetailRuntimeValue::Known(saved_target_word),
            RetailRuntimeValue::Known(manual_pitch_delta_raw),
            RetailRuntimeValue::Known(accumulator_raw),
        ) = (
            runtime.table_bytes_at_0x28,
            runtime.decaying_output_raw_at_0x30,
            runtime.rate_raw_at_0x34,
            runtime.randomized_target_raw_at_0x38,
            runtime.invert_target_at_0x3c,
            runtime.clearance_flag_at_0x3d,
            runtime.audio_gate_at_0x3e,
            runtime.mode_at_0x3f,
            runtime.mode_at_0x40,
            runtime.byte_at_0x41,
            runtime.terrain_attitude_mode_at_0x42,
            runtime.alternate_basis_mode_at_0x43,
            runtime.word_at_0x1c,
            runtime.angle_raw_at_0x1e,
            runtime.accumulator_raw_at_0x24,
        )
        else {
            return Err(Type13SubGFrameBlock::RuntimeUnavailable);
        };
        if mode_at_0x3f != 0
            || mode_at_0x40 != 0
            || terrain_attitude_mode != 0
            || alternate_basis_mode != 0
            || self_righting_mode != 1
        {
            return Err(Type13SubGFrameBlock::UnsupportedMode);
        }
        // Type-13 is an ordinary non-player entity. The capability-bit-one branch
        // uses the separate five-probe VTOL policy and is already owned by vtol.rs.
        if capability_flags & 1 != 0 {
            return Err(Type13SubGFrameBlock::PlayerProjectionUnsupported);
        }
        if let Some(reverse) = reverse_write {
            invert_target_raw = u8::from(reverse);
        }

        let bindings: [u8; 7] = descriptor[0x24..=0x2a]
            .try_into()
            .expect("fixed Sub-G descriptor binding range");
        let mut animation_outputs_raw = [0i16; 7];
        if bindings[5] != 0 {
            animation_outputs_raw[5] = 0;
        }
        if bindings[6] != 0 {
            animation_outputs_raw[6] = decaying_output_raw as i16;
            if decaying_output_raw > 0x100 {
                let decay = ((elapsed_micros >> 4) as i32).wrapping_mul(decaying_output_raw) >> 15;
                decaying_output_raw = decaying_output_raw.wrapping_sub(decay as i16 as i32);
            }
        }
        for channel in 0..5 {
            if bindings[channel] == 0 {
                continue;
            }
            let record_offset = 0x2c + channel * 0x0c;
            if read_i32(descriptor, record_offset + 8) == 1 {
                let phase_step = prior_rate_raw.wrapping_mul((elapsed_micros >> 12) as i32) >> 6;
                phases[channel] = phases[channel].wrapping_add(phase_step as u8);
                let base_raw = read_i16(descriptor, record_offset);
                let amplitude_raw = read_u16(descriptor, record_offset + 2);
                let sine_q31 =
                    retail_sine_q15(u32::from(phases[channel]) << 8).wrapping_mul(0x1_0001);
                let oscillation_raw =
                    ((i64::from(amplitude_raw) * i64::from(sine_q31)) >> 31) as i32;
                animation_outputs_raw[channel] = base_raw.wrapping_add(oscillation_raw as i16);
            }
        }

        let sound_id = read_u16(descriptor, 0x0e);
        let sound = if (0x61..=0x9f).contains(&phases[0]) {
            if audio_gate_raw == 0 {
                audio_gate_raw = 1;
                (sound_id != 0).then_some(Type13SubGSound {
                    sound_id,
                    position_raw,
                    rate_q16: 0x0000_aaaa,
                })
            } else {
                None
            }
        } else {
            audio_gate_raw = 0;
            None
        };
        runtime.table_bytes_at_0x28 = RetailRuntimeValue::Known(phases);
        runtime.decaying_output_raw_at_0x30 = RetailRuntimeValue::Known(decaying_output_raw);
        runtime.invert_target_at_0x3c = RetailRuntimeValue::Known(invert_target_raw);
        runtime.audio_gate_at_0x3e = RetailRuntimeValue::Known(audio_gate_raw);
        runtime.animation_outputs_raw = RetailRuntimeValue::Known(animation_outputs_raw);
        prefix.runtime = Some(runtime);
        prefix.sound = sound;

        let surface_mode = descriptor[0x2b];
        let coarse_center_surface_raw =
            coarse_surface_raw(terrain, position_raw[0], position_raw[2], surface_mode == 0);
        let projected_raw =
            vtol_terrain_projected_probe_raw(position_raw, retained_body_basis.forward);
        let current_surface_raw = sub_g_surface_raw(
            terrain,
            position_raw[0],
            position_raw[2],
            surface_mode,
            retail_tick,
        );
        let projected_surface_raw = sub_g_surface_raw(
            terrain,
            projected_raw[0],
            projected_raw[2],
            surface_mode,
            retail_tick,
        );
        let attitude = plan_vtol_terrain_derived_attitude_raw(VtolTerrainDerivedAttitudeRequest {
            current_pitch_raw: pitch_raw,
            saved_target_raw: saved_target_word as i16,
            target_limit_raw: read_i32(descriptor, 0x1c),
            manual_pitch_delta_raw,
            invert_target: invert_target_raw != 0,
            center_y_raw: position_raw[1],
            active_model_radius_raw: active_model_extent_raw,
            coarse_center_surface_raw,
            current_surface_raw,
            projected_surface_raw,
            rate_min_raw: read_i32(descriptor, 0x10),
            rate_max_raw: read_i32(descriptor, 0x14),
            target_clearance_raw,
            max_linear_velocity_raw: read_i32(descriptor, 0x20),
            prior_clearance_flag: prior_clearance_flag_raw != 0,
            velocity_raw,
        })
        .ok_or(Type13SubGFrameBlock::MalformedDescriptor)?;
        velocity_raw = attitude.velocity_raw;
        runtime.rate_raw_at_0x34 = RetailRuntimeValue::Known(attitude.rate_raw);
        runtime.clearance_flag_at_0x3d =
            RetailRuntimeValue::Known(u8::from(attitude.clearance_flag));
        prefix.runtime = Some(runtime);
        prefix.velocity_raw = Some(velocity_raw);
        prefix.pitch_raw = Some(attitude.pitch_raw);

        let rate_midpoint_raw =
            read_i32(descriptor, 0x10).wrapping_add(read_i32(descriptor, 0x14)) / 2;
        let rate_denominator = rate_midpoint_raw.wrapping_mul(rate_midpoint_raw);
        if rate_denominator == 0 {
            return Err(Type13SubGFrameBlock::MalformedDescriptor);
        }
        let derived_force_raw = read_i32(descriptor, 0x04)
            .wrapping_mul(attitude.rate_raw)
            .wrapping_mul(attitude.rate_raw)
            / rate_denominator;
        let manual_force_raw = if bindings[0] == 0 {
            // The null-pointer branch uses one quarter of the derived force.
            // Type-13 binds selector one and therefore never takes this branch.
            derived_force_raw / 4
        } else if audio_gate_raw != 0 {
            derived_force_raw
        } else {
            read_i32(descriptor, 0x00)
        };

        let mut damped_pitch_raw = attitude.pitch_raw;
        let pitch_correction =
            ((elapsed_micros >> 10) as i32).wrapping_mul(i32::from(damped_pitch_raw)) >> 9;
        damped_pitch_raw = damped_pitch_raw.wrapping_sub(pitch_correction as i16);
        let mut damped_roll_raw = roll_raw as i16;
        let roll_correction =
            ((elapsed_micros >> 10) as i32).wrapping_mul(i32::from(damped_roll_raw)) >> 8;
        damped_roll_raw = damped_roll_raw.wrapping_sub(roll_correction as i16);
        runtime.source_raw_at_0x20 = RetailRuntimeValue::Known(manual_force_raw as u32);
        prefix.runtime = Some(runtime);
        prefix.pitch_raw = Some(damped_pitch_raw);
        prefix.roll_raw = Some(damped_roll_raw as u16);

        apply_type13_b210_projection(Type13B210ProjectionRequest {
            position_raw,
            velocity_raw: &mut velocity_raw,
            body_up_q31: retained_body_basis.up,
            active_model_extent_raw,
            self_mass_raw,
            attached_cargo_mass,
            manual_force_raw,
            assist_force_raw: accumulator_raw as i32,
            rate_raw: attitude.rate_raw,
            clearance_flag: attitude.clearance_flag,
            surface_mode,
            terrain,
            retail_tick,
            elapsed_micros,
        })?;

        runtime.table_bytes_at_0x28 = RetailRuntimeValue::Known(phases);
        runtime.decaying_output_raw_at_0x30 = RetailRuntimeValue::Known(decaying_output_raw);
        runtime.rate_raw_at_0x34 = RetailRuntimeValue::Known(attitude.rate_raw);
        runtime.invert_target_at_0x3c = RetailRuntimeValue::Known(invert_target_raw);
        runtime.clearance_flag_at_0x3d =
            RetailRuntimeValue::Known(u8::from(attitude.clearance_flag));
        runtime.audio_gate_at_0x3e = RetailRuntimeValue::Known(audio_gate_raw);
        runtime.source_raw_at_0x20 = RetailRuntimeValue::Known(manual_force_raw as u32);
        runtime.animation_outputs_raw = RetailRuntimeValue::Known(animation_outputs_raw);

        Ok(Type13SubGFrameOutcome {
            runtime,
            velocity_raw,
            pitch_raw: damped_pitch_raw,
            roll_raw: damped_roll_raw as u16,
            sound,
        })
    })();
    result.map_err(|reason| SubGFrameFailure { reason, prefix })
}

struct Type13B210ProjectionRequest<'a> {
    position_raw: [i16; 3],
    velocity_raw: &'a mut [i16; 3],
    body_up_q31: [i32; 3],
    active_model_extent_raw: u16,
    self_mass_raw: u16,
    attached_cargo_mass: u32,
    manual_force_raw: i32,
    assist_force_raw: i32,
    rate_raw: i32,
    clearance_flag: bool,
    surface_mode: u8,
    terrain: &'a TerrainGrid,
    retail_tick: u32,
    elapsed_micros: u32,
}

fn apply_type13_b210_projection(
    request: Type13B210ProjectionRequest<'_>,
) -> Result<(), Type13SubGFrameBlock> {
    let mut vertical_force_raw = 0;
    if request.manual_force_raw != 0 && (!request.clearance_flag || request.rate_raw < 200) {
        let surface_raw = sub_g_surface_raw(
            request.terrain,
            request.position_raw[0],
            request.position_raw[2],
            request.surface_mode,
            request.retail_tick,
        );
        let underside_raw = i32::from(request.position_raw[1])
            .wrapping_sub(i32::from(request.active_model_extent_raw >> 1));
        if underside_raw.wrapping_sub(i32::from(surface_raw)) < 0x800 {
            vertical_force_raw = request.manual_force_raw;
        }
    }

    if request.manual_force_raw == 0 && request.assist_force_raw == 0 {
        return Ok(());
    }
    let total_mass_raw =
        u32::from(request.self_mass_raw).wrapping_add(request.attached_cargo_mass) as i32;
    if total_mass_raw == 0 {
        return Err(Type13SubGFrameBlock::ZeroTotalMass);
    }
    let elapsed_step_raw = (request.elapsed_micros >> 5) as i32;
    let mass_twice_raw = total_mass_raw.wrapping_mul(2);
    let horizontal_rate_raw =
        elapsed_step_raw.wrapping_mul(request.manual_force_raw) / mass_twice_raw;
    let vertical_rate_raw = elapsed_step_raw.wrapping_mul(vertical_force_raw) / mass_twice_raw;
    request.velocity_raw[0] = request.velocity_raw[0]
        .wrapping_add(q31_mul(request.body_up_q31[0], horizontal_rate_raw) as i16);
    request.velocity_raw[1] = request.velocity_raw[1]
        .wrapping_add(q31_mul(request.body_up_q31[1], vertical_rate_raw) as i16);
    request.velocity_raw[2] = request.velocity_raw[2]
        .wrapping_add(q31_mul(request.body_up_q31[2], horizontal_rate_raw) as i16);

    if request.assist_force_raw != 0 {
        let assist_vertical_rate_raw =
            elapsed_step_raw.wrapping_mul(request.assist_force_raw) / mass_twice_raw;
        let assist_horizontal_rate_raw = elapsed_step_raw.wrapping_mul(request.assist_force_raw)
            / total_mass_raw.wrapping_mul(0x10);
        request.velocity_raw[0] = request.velocity_raw[0]
            .wrapping_add(q31_mul(request.body_up_q31[0], assist_horizontal_rate_raw) as i16);
        request.velocity_raw[1] = request.velocity_raw[1]
            .wrapping_add(q31_mul(request.body_up_q31[1], assist_vertical_rate_raw) as i16);
        request.velocity_raw[2] = request.velocity_raw[2]
            .wrapping_add(q31_mul(request.body_up_q31[2], assist_horizontal_rate_raw) as i16);
    }
    Ok(())
}

fn coarse_surface_raw(
    terrain: &TerrainGrid,
    x_raw: i16,
    z_raw: i16,
    include_static_sea: bool,
) -> i16 {
    let x = usize::from((x_raw as u16) >> 8);
    let z = usize::from((z_raw as u16) >> 8);
    let terrain_raw = i16::from(
        terrain
            .cell(x, z)
            .expect("retail terrain is a complete 256x256 torus")
            .height as i8,
    ) << 5;
    if include_static_sea {
        terrain_raw.max(terrain.sea_level_raw())
    } else {
        terrain_raw
    }
}

fn sub_g_surface_raw(
    terrain: &TerrainGrid,
    x_raw: i16,
    z_raw: i16,
    surface_mode: u8,
    retail_tick: u32,
) -> i16 {
    let terrain_raw = terrain.bilinear_height_raw(x_raw, z_raw);
    if surface_mode == 0 && terrain.water_enabled() {
        wave_surface_raw(
            x_raw,
            z_raw,
            retail_tick as i32,
            terrain.sea_level_raw(),
            terrain_raw,
        )
    } else {
        terrain_raw
    }
}

fn read_u16(bytes: &[u8; 104], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_i16(bytes: &[u8; 104], offset: usize) -> i16 {
    i16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_i32(bytes: &[u8; 104], offset: usize) -> i32 {
    i32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search_attack_live::{
        TYPE13_SEARCH_ATTACK_SUB_G, TYPE13_SEARCH_ATTACK_SUB_G_1B8C0_TABLE_BYTES,
        TYPE13_SEARCH_ATTACK_SUB_G_RANDOMIZED_TARGET_BASE_RAW_AT_0X0C,
    };

    #[test]
    fn type13_1b8c0_writes_table_angle_and_randomized_target() {
        let runtime =
            SubG06070RuntimeState::from_1b8c0_constructor(&TYPE13_SEARCH_ATTACK_SUB_G, 0x1100);
        assert_eq!(
            TYPE13_SEARCH_ATTACK_SUB_G_1B8C0_TABLE_BYTES,
            [0, 0, 0, 230, 210]
        );
        assert_eq!(
            runtime.table_bytes_at_0x28(),
            RetailRuntimeValue::Known(TYPE13_SEARCH_ATTACK_SUB_G_1B8C0_TABLE_BYTES)
        );
        assert_eq!(
            runtime.randomized_target_raw_at_0x38(),
            RetailRuntimeValue::Known(
                i32::from(TYPE13_SEARCH_ATTACK_SUB_G_RANDOMIZED_TARGET_BASE_RAW_AT_0X0C) + 0x11
            )
        );
        assert_eq!(
            runtime.word_at_0x1c(),
            RetailRuntimeValue::Known(SUB_G_1B8C0_WORD_AT_0X1C)
        );
        assert_eq!(
            runtime.angle_raw_at_0x1e(),
            RetailRuntimeValue::Known(SUB_G_1B8C0_ANGLE_RAW_AT_0X1E)
        );
        assert_eq!(
            runtime.byte_at_0x41(),
            RetailRuntimeValue::Known(SUB_G_1B8C0_BYTE_AT_0X41)
        );
        assert_eq!(runtime.source_raw_at_0x20(), RetailRuntimeValue::Known(0));
        assert_eq!(
            runtime.invert_target_at_0x3c(),
            RetailRuntimeValue::Known(0)
        );
    }

    #[test]
    fn shared_06070_branch_preserves_1b8c0_table_and_angle() {
        let mut runtime =
            SubG06070RuntimeState::from_1b8c0_constructor(&TYPE13_SEARCH_ATTACK_SUB_G, 0x1100);
        runtime.apply_shared_06070_sub_g_branch(217, 0);
        assert_eq!(runtime.mode_at_0x3f(), RetailRuntimeValue::Known(0));
        assert_eq!(
            runtime.randomized_target_raw_at_0x38(),
            RetailRuntimeValue::Known(217)
        );
        assert_eq!(
            runtime.table_bytes_at_0x28(),
            RetailRuntimeValue::Known(TYPE13_SEARCH_ATTACK_SUB_G_1B8C0_TABLE_BYTES)
        );
        assert_eq!(runtime.word_at_0x1c(), RetailRuntimeValue::Known(0x5000));
        assert_eq!(runtime.angle_raw_at_0x1e(), RetailRuntimeValue::Known(0));
        assert_eq!(runtime.byte_at_0x41(), RetailRuntimeValue::Known(1));
    }

    #[test]
    fn tumble_setter_and_frame_keep_exact_source_write_sets() {
        let mut runtime = SubG06070RuntimeState::pending();
        runtime.mode_at_0x40 = RetailRuntimeValue::Known(0);
        runtime.source_raw_at_0x20 = RetailRuntimeValue::Known(-456i32 as u32);
        runtime.accumulator_raw_at_0x24 = RetailRuntimeValue::Known(123);
        runtime.animation_outputs_raw = RetailRuntimeValue::Known([12, -13, 14, -15, 16, -17, 18]);
        runtime.audio_gate_at_0x3e = RetailRuntimeValue::Known(1);
        runtime.table_bytes_at_0x28 = RetailRuntimeValue::Known([120, 0, 255, 3, 4]);
        let mut expected = runtime;
        expected.mode_at_0x3f = RetailRuntimeValue::Known(1);
        runtime.enter_tumble();
        assert_eq!(runtime, expected, "1B970 stores only byte3F");
        expected.source_raw_at_0x20 = RetailRuntimeValue::Known(0);
        expected.accumulator_raw_at_0x24 = RetailRuntimeValue::Known(0);
        runtime.advance_tumble_mode().unwrap();
        assert_eq!(
            runtime, expected,
            "AA36 must not clear retained animation, sound gate, or unknown controller words"
        );
        runtime.advance_tumble_mode().unwrap();
        assert_eq!(
            runtime, expected,
            "later callbacks do not run oscillation or attitude control"
        );
    }

    #[test]
    fn tumble_branch_skips_terrain_descriptor_mass_and_attitude_reads() {
        let terrain = TerrainGrid {
            header: [0; 5],
            cells: Vec::new(),
        };
        let mut runtime = SubG06070RuntimeState::pending();
        runtime.enter_tumble();
        runtime.mode_at_0x40 = RetailRuntimeValue::Known(0);
        let mut expected = runtime;
        expected.advance_tumble_mode().unwrap();
        let frame = plan_type13_sub_g_frame(Type13SubGFrameRequest {
            descriptor: &[0; 104],
            runtime,
            reverse_write: None,
            position_raw: [123, -456, 789],
            velocity_raw: [32000, -1234, -32000],
            pitch_raw: -3210,
            roll_raw: 0xffed,
            retained_body_basis: Type9BodyBasis {
                lateral: [0; 3],
                up: [0; 3],
                forward: [0; 3],
            },
            active_model_extent_raw: 0,
            self_mass_raw: 0,
            attached_cargo_mass: 0,
            capability_flags: 1,
            terrain: &terrain,
            retail_tick: u32::MAX,
            elapsed_micros: u32::MAX,
        })
        .unwrap();
        assert_eq!(frame.runtime, expected);
        assert_eq!(frame.velocity_raw, [32000, -1234, -32000]);
        assert_eq!(frame.pitch_raw, -3210);
        assert_eq!(frame.roll_raw, 0xffed);
        assert_eq!(frame.sound, None);
    }

    #[test]
    fn tumble_mode_gate_rejects_unknown_or_other_branch_without_writes() {
        for (mode, override_mode, block) in [
            (
                RetailRuntimeValue::Unresolved,
                RetailRuntimeValue::Known(0),
                Type13SubGFrameBlock::RuntimeUnavailable,
            ),
            (
                RetailRuntimeValue::Known(1),
                RetailRuntimeValue::Unresolved,
                Type13SubGFrameBlock::RuntimeUnavailable,
            ),
            (
                RetailRuntimeValue::Known(0),
                RetailRuntimeValue::Known(0),
                Type13SubGFrameBlock::UnsupportedMode,
            ),
            (
                RetailRuntimeValue::Known(1),
                RetailRuntimeValue::Known(1),
                Type13SubGFrameBlock::UnsupportedMode,
            ),
        ] {
            let mut runtime = SubG06070RuntimeState::pending();
            runtime.mode_at_0x3f = mode;
            runtime.mode_at_0x40 = override_mode;
            runtime.source_raw_at_0x20 = RetailRuntimeValue::Known(777);
            runtime.accumulator_raw_at_0x24 = RetailRuntimeValue::Known(888);
            let before = runtime;
            assert_eq!(runtime.advance_tumble_mode(), Err(block));
            assert_eq!(runtime, before);
        }
    }
}
