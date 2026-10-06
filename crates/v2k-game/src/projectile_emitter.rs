//! Isolated type-47 projectile presentation policy.
//!
//! This module is the narrow pure policy for the first-world type-47 shot after
//! the common emitter has resolved a launch. It is not the generic
//! `FUN_00424650` transaction; that ordering-sensitive mutable runtime lives in
//! [`crate::generic_projectile_emitter`]. This module deliberately does not own
//! target acquisition, behavior scheduling, movement, particle-pool
//! allocation, audio playback, or damage dispatch. Those systems must supply
//! the proven pre-gates and consume a successful [`ProjectileEmissionRequest`]
//! in their existing shared order.

use v2k_formats::collision::ProjectileEmitterDescriptor;

use crate::{
    damage::{DamagePacket, TYPE_47_PROJECTILE_DAMAGE_PACKET},
    particle_descriptors::{
        particle_descriptor, particle_frame, ParticleDescriptor, ParticleFrame,
    },
};

/// Cumulative Section-12 type using the recovered first-world shooter block.
pub const FIRST_WORLD_SHOOTER_TYPE: u32 = 47;
/// Projectile method selected by type 47's authored Sub-E descriptor.
pub const FIRST_WORLD_SHOOTER_PROJECTILE_METHOD: u32 = 30;
/// Retail address of projectile-method 30's four-dword table row.
pub const PROJECTILE_METHOD_30_TABLE_VA: u32 = 0x004D_04A0;
/// Projectile particle selected by method 30.
pub const FIRST_WORLD_SHOOTER_PARTICLE_CLASS: u8 = 87;
/// Default raw speed selected by method 30 when Sub-E has no override.
pub const FIRST_WORLD_SHOOTER_PROJECTILE_SPEED_RAW: i32 = 3_000;
/// Positional Section-11 cue authored in type 47's Sub-E block.
pub const FIRST_WORLD_SHOOTER_SOUND_ID: u16 = 70;
/// Projectile method selected by type 13's authored Sub-E descriptor.
pub const TYPE13_PROJECTILE_METHOD: u32 = 10;
/// Projectile particle selected by method 10 (`DAT_004D02C8`).
pub const TYPE13_PROJECTILE_PARTICLE_CLASS: u8 = 38;
/// `FUN_004410B0` underwater substitute for class 38 (`0x2E`).
pub const TYPE13_PROJECTILE_UNDERWATER_PARTICLE_CLASS: u8 = 0x2E;
/// Default raw speed selected by method 10 when Sub-E has no override.
pub const TYPE13_PROJECTILE_SPEED_RAW: i32 = 2_400;

/// Class-87's update callback (`descriptor + 0x14`).
pub const CLASS_87_UPDATE_CALLBACK_VA: u32 = 0x0043_F260;
/// Shared class-52/53/68/69/87 surface callback (`descriptor + 0x18`).
pub const PROJECTILE_SURFACE_CALLBACK_VA: u32 = 0x0044_2420;
/// Class-87's entity-hit callback (`descriptor + 0x1C`).
///
/// `FUN_0043F980` is the common mode-3 sweep which invokes this descriptor
/// slot. `FUN_0043F590` itself emits the `FUN_0043F610` contact visual and
/// conditionally delivers the descriptor's damage packet.
pub const CLASS_87_ENTITY_HIT_CALLBACK_VA: u32 = 0x0043_F590;
/// Address of class 87's two-slot damage packet (`descriptor + 0x20`).
pub const CLASS_87_DAMAGE_PACKET_VA: u32 = 0x004C_C0F0;
/// Class-87's solid-static callback (`descriptor + 0x24`).
///
/// This is the retail no-op callback: a solid-static hit consumes the parent
/// without emitting `FUN_0043F610` or delivering the packet.
pub const CLASS_87_STATIC_HIT_CALLBACK_VA: u32 = 0x0042_E8E0;

/// One immutable row from the retail projectile-method table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectileMethodSpec {
    pub selector: u32,
    pub table_va: u32,
    pub leading_raw: u32,
    pub speed_raw: i32,
    pub particle_class: u8,
    pub trailing_raw: u32,
}

/// Exact method-30 table row `{0, 3000, 87, 0}`.
pub const PROJECTILE_METHOD_30: ProjectileMethodSpec = ProjectileMethodSpec {
    selector: FIRST_WORLD_SHOOTER_PROJECTILE_METHOD,
    table_va: PROJECTILE_METHOD_30_TABLE_VA,
    leading_raw: 0,
    speed_raw: FIRST_WORLD_SHOOTER_PROJECTILE_SPEED_RAW,
    particle_class: FIRST_WORLD_SHOOTER_PARTICLE_CLASS,
    trailing_raw: 0,
};

/// Retail virtual address of projectile-class row 0 (`DAT_004D02C0`).
pub const PROJECTILE_CLASS_TABLE_VA: u32 = 0x004D_02C0;
/// Exact size of one `DAT_004D02C0` row.
pub const PROJECTILE_CLASS_STRIDE: usize = 0x10;
/// Rows `0..=31`. Method 30 is `PROJECTILE_CLASS_TABLE_VA + 30 * 0x10`.
pub const PROJECTILE_CLASS_COUNT: usize = 32;

/// One `DAT_004D02C0` row. `FUN_00444FA0` copies the selected weapon
/// descriptor dword into component `+0x10`, and that dword indexes this table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectileClassRow {
    pub leading_raw: u32,
    pub speed_raw: i32,
    pub particle_class: u8,
    pub trailing_raw: u32,
}

const fn projectile_class_row_from_raw(
    leading_raw: u32,
    speed_raw: i32,
    particle_class: u8,
    trailing_raw: u32,
) -> ProjectileClassRow {
    ProjectileClassRow {
        leading_raw,
        speed_raw,
        particle_class,
        trailing_raw,
    }
}

/// Exact executable rows `0..=31`.
pub const PROJECTILE_CLASS_TABLE: [ProjectileClassRow; PROJECTILE_CLASS_COUNT] = [
    projectile_class_row_from_raw(0, 0, 0, 0),
    projectile_class_row_from_raw(0, 4_000, 1, 0),
    projectile_class_row_from_raw(0, 2_000, 3, 0),
    projectile_class_row_from_raw(0, 2_000, 0, 59),
    projectile_class_row_from_raw(0, 1_000, 0, 59),
    projectile_class_row_from_raw(1, 1, 0, 42),
    projectile_class_row_from_raw(0, 2_000, 4, 0),
    projectile_class_row_from_raw(0, 2_000, 6, 0),
    projectile_class_row_from_raw(0, 2_000, 5, 0),
    projectile_class_row_from_raw(0, 1_000, 0, 48),
    projectile_class_row_from_raw(1, 2_400, 38, 0),
    projectile_class_row_from_raw(0, 2_400, 0, 34),
    projectile_class_row_from_raw(1, 6_000, 49, 0),
    projectile_class_row_from_raw(1, 5_500, 56, 0),
    projectile_class_row_from_raw(1, 5_000, 55, 0),
    projectile_class_row_from_raw(0, 1, 32, 0),
    projectile_class_row_from_raw(0, 2_000, 50, 0),
    projectile_class_row_from_raw(1, 1, 0, 45),
    projectile_class_row_from_raw(0, 4_000, 2, 0),
    projectile_class_row_from_raw(0, 2_000, 54, 0),
    projectile_class_row_from_raw(0, 3_000, 52, 0),
    projectile_class_row_from_raw(0, 5_000, 49, 0),
    projectile_class_row_from_raw(1, 1_500, 39, 0),
    projectile_class_row_from_raw(0, 1, 0, 120),
    projectile_class_row_from_raw(0, 2_000, 68, 0),
    projectile_class_row_from_raw(0, 1, 0, 69),
    projectile_class_row_from_raw(0, 1_000, 57, 0),
    projectile_class_row_from_raw(0, 1_000, 0, 0),
    projectile_class_row_from_raw(0, 2_000, 77, 0),
    projectile_class_row_from_raw(0, 2_000, 78, 0),
    projectile_class_row_from_raw(0, 3_000, 87, 0),
    projectile_class_row_from_raw(0, 3_000, 89, 0),
];

/// Index `DAT_004D02C0` by the selected weapon descriptor dword.
pub const fn projectile_class_row(selector: u32) -> Option<ProjectileClassRow> {
    if (selector as usize) < PROJECTILE_CLASS_COUNT {
        Some(PROJECTILE_CLASS_TABLE[selector as usize])
    } else {
        None
    }
}

/// Resolve only projectile-method rows whose executable data has been audited.
pub const fn projectile_method_spec(selector: u32) -> Option<ProjectileMethodSpec> {
    match selector {
        FIRST_WORLD_SHOOTER_PROJECTILE_METHOD => Some(PROJECTILE_METHOD_30),
        _ => None,
    }
}

/// Class-87 data retained by the exact executable particle tables.
///
/// `FUN_00442950` belongs to a different particle descriptor and is
/// intentionally absent here. Class 87 directly stores `FUN_0043F590` in its
/// entity-hit callback slot.
#[derive(Debug, Clone, Copy)]
pub struct Class87ParticleSpec {
    pub descriptor: &'static ParticleDescriptor,
    pub frames: [ParticleFrame; 3],
    pub update_callback_va: u32,
    pub surface_callback_va: u32,
    pub entity_hit_callback_va: u32,
    pub damage_packet_va: u32,
    pub static_hit_callback_va: u32,
    pub damage_packet: DamagePacket,
}

pub fn class_87_particle_spec() -> Class87ParticleSpec {
    Class87ParticleSpec {
        descriptor: particle_descriptor(FIRST_WORLD_SHOOTER_PARTICLE_CLASS)
            .expect("class 87 is inside the exact retail descriptor table"),
        frames: std::array::from_fn(|index| {
            particle_frame(FIRST_WORLD_SHOOTER_PARTICLE_CLASS, index)
                .expect("class 87 has exactly three retained retail frames")
        }),
        update_callback_va: CLASS_87_UPDATE_CALLBACK_VA,
        surface_callback_va: PROJECTILE_SURFACE_CALLBACK_VA,
        entity_hit_callback_va: CLASS_87_ENTITY_HIT_CALLBACK_VA,
        damage_packet_va: CLASS_87_DAMAGE_PACKET_VA,
        static_hit_callback_va: CLASS_87_STATIC_HIT_CALLBACK_VA,
        damage_packet: TYPE_47_PROJECTILE_DAMAGE_PACKET,
    }
}

/// An explicitly supported common-emitter profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectileEmitterProfile {
    pub descriptor: ProjectileEmitterDescriptor,
    pub method: ProjectileMethodSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileEmitterProfileError {
    UnknownProjectileMethod(u32),
    UnsupportedStochasticGateMode(u8),
    UnsupportedAlternateEmitter(u16),
    UnsupportedAuxiliaryCommand(u8),
}

impl ProjectileEmitterProfile {
    /// Resolve the static data needed by this isolated one-shot policy.
    ///
    /// Alternating emitters and auxiliary commands are valid retail features,
    /// but they require additional request records. Rejecting them here keeps
    /// this type-47 foundation exact instead of silently dropping work.
    pub fn from_descriptor(
        descriptor: ProjectileEmitterDescriptor,
    ) -> Result<Self, ProjectileEmitterProfileError> {
        let method = projectile_method_spec(descriptor.projectile_method).ok_or(
            ProjectileEmitterProfileError::UnknownProjectileMethod(descriptor.projectile_method),
        )?;
        if descriptor.stochastic_gate_mode != 0 {
            return Err(
                ProjectileEmitterProfileError::UnsupportedStochasticGateMode(
                    descriptor.stochastic_gate_mode,
                ),
            );
        }
        if descriptor.alternate_emitter_raw != 0 {
            return Err(ProjectileEmitterProfileError::UnsupportedAlternateEmitter(
                descriptor.alternate_emitter_raw,
            ));
        }
        if descriptor.auxiliary_command != 0 {
            return Err(ProjectileEmitterProfileError::UnsupportedAuxiliaryCommand(
                descriptor.auxiliary_command,
            ));
        }
        Ok(Self { descriptor, method })
    }

    pub const fn speed_raw(self) -> i32 {
        if self.descriptor.speed_override_raw == 0 {
            self.method.speed_raw
        } else {
            self.descriptor.speed_override_raw as i32
        }
    }
}

/// Caller-resolved target information inspected only after the cadence draw
/// succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectileTargetCandidate {
    pub target_id: u32,
    pub position_raw: [i16; 3],
    pub aim_error_raw: i16,
    /// Whether `FUN_0041E930` places the target on or ahead of the source's
    /// forward plane.
    pub in_forward_half_space: bool,
}

/// Inputs owned by behavior, scheduler, and target-acquisition systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectileEmissionInput {
    /// Behavior/scheduler gate evaluated before entering the common creator.
    pub scheduler_allows: bool,
    /// Current behavior mode's independent firing gate.
    pub mode_allows: bool,
    /// Nonzero simulation delta supplied to `FUN_00424650`.
    pub elapsed_us: u32,
    pub source_id: u32,
    pub source_position_raw: [i16; 3],
    pub target: Option<ProjectileTargetCandidate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileTargetRejection {
    Missing,
    OutsideAxisTolerance,
    AimError,
    OutsideForwardHalfSpace,
}

/// The exact shared-RNG samples consumed by one successful type-47 emission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectileEmissionRandomSamples {
    pub cadence: u16,
    pub spread_x: u16,
    pub spread_z: u16,
}

/// Backend-neutral request produced after every retail acceptance gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectileEmissionRequest {
    pub source_id: u32,
    pub target_id: u32,
    pub target_position_raw: [i16; 3],
    pub projectile_method: u32,
    pub particle_class: u8,
    pub speed_raw: i32,
    pub sound_id: u16,
    pub damage_packet: DamagePacket,
    pub random_samples: ProjectileEmissionRandomSamples,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileEmissionDecision {
    /// No random value is consumed before either external pre-gate passes.
    PreGateRejected,
    /// Invalid deltas are rejected without touching the shared random stream.
    InvalidElapsed,
    CadenceRejected {
        cadence_sample: u16,
        cadence_divisor: u32,
    },
    TargetRejected {
        cadence_sample: u16,
        cadence_divisor: u32,
        reason: ProjectileTargetRejection,
    },
    Emit(ProjectileEmissionRequest),
}

/// Evaluate `FUN_00424650`'s isolated type-47 one-shot path.
///
/// `next_shared_random` must draw from the same world RNG stream as the other
/// retail systems. The ordering is intentional: no draw for a failed pre-gate,
/// one cadence draw for every entered attempt, and two spread draws only after
/// target range, aim, and forward-half-space gates all pass.
pub fn plan_projectile_emission(
    profile: ProjectileEmitterProfile,
    input: ProjectileEmissionInput,
    next_shared_random: &mut impl FnMut() -> u16,
) -> ProjectileEmissionDecision {
    if !input.scheduler_allows || !input.mode_allows {
        return ProjectileEmissionDecision::PreGateRejected;
    }
    if input.elapsed_us == 0 {
        return ProjectileEmissionDecision::InvalidElapsed;
    }

    let cadence_divisor = (profile.descriptor.random_interval_us / input.elapsed_us).max(1);
    let cadence_sample = next_shared_random();
    if u32::from(cadence_sample) % cadence_divisor != 0 {
        return ProjectileEmissionDecision::CadenceRejected {
            cadence_sample,
            cadence_divisor,
        };
    }

    let Some(target) = input.target else {
        return ProjectileEmissionDecision::TargetRejected {
            cadence_sample,
            cadence_divisor,
            reason: ProjectileTargetRejection::Missing,
        };
    };
    if !within_strict_wrapped_axis_tolerance(
        input.source_position_raw[0],
        target.position_raw[0],
        profile.descriptor.target_axis_tolerance_raw,
    ) || !within_strict_wrapped_axis_tolerance(
        input.source_position_raw[2],
        target.position_raw[2],
        profile.descriptor.target_axis_tolerance_raw,
    ) {
        return ProjectileEmissionDecision::TargetRejected {
            cadence_sample,
            cadence_divisor,
            reason: ProjectileTargetRejection::OutsideAxisTolerance,
        };
    }

    let threshold = profile.descriptor.aim_threshold_raw;
    if threshold <= i16::MAX as u16 {
        let aim_error = i32::from(target.aim_error_raw);
        let threshold = i32::from(threshold);
        if aim_error < -threshold || aim_error >= threshold {
            return ProjectileEmissionDecision::TargetRejected {
                cadence_sample,
                cadence_divisor,
                reason: ProjectileTargetRejection::AimError,
            };
        }
        if !target.in_forward_half_space {
            return ProjectileEmissionDecision::TargetRejected {
                cadence_sample,
                cadence_divisor,
                reason: ProjectileTargetRejection::OutsideForwardHalfSpace,
            };
        }
    }

    let spread_x = next_shared_random();
    let spread_z = next_shared_random();
    let spread_raw = profile.descriptor.spread_raw;
    let mut target_position_raw = target.position_raw;
    target_position_raw[0] =
        target_position_raw[0].wrapping_add(spread_offset(spread_x, spread_raw));
    target_position_raw[2] =
        target_position_raw[2].wrapping_add(spread_offset(spread_z, spread_raw));

    ProjectileEmissionDecision::Emit(ProjectileEmissionRequest {
        source_id: input.source_id,
        target_id: target.target_id,
        target_position_raw,
        projectile_method: profile.method.selector,
        particle_class: profile.method.particle_class,
        speed_raw: profile.speed_raw(),
        sound_id: profile.descriptor.sound_id,
        damage_packet: TYPE_47_PROJECTILE_DAMAGE_PACKET,
        random_samples: ProjectileEmissionRandomSamples {
            cadence: cadence_sample,
            spread_x,
            spread_z,
        },
    })
}

fn within_strict_wrapped_axis_tolerance(source: i16, target: i16, tolerance: u16) -> bool {
    i32::from(source.wrapping_sub(target)).abs() < i32::from(tolerance)
}

fn spread_offset(sample: u16, spread_raw: u16) -> i16 {
    let scaled = (u32::from(sample) * u32::from(spread_raw)) >> 15;
    (scaled as i32 - i32::from(spread_raw)) as i16
}

/// Exact source-class and material response selected by `FUN_00442420`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileSurfaceResponse {
    ConvertToParticleClass(u8),
    /// The callback itself leaves the record unchanged and returns one. The
    /// enclosing mode-3 dispatcher then frees the parent.
    Delete,
}

pub const fn projectile_surface_response(
    source_class: u8,
    selector: u8,
) -> ProjectileSurfaceResponse {
    // Retail's source switch precedes its material switch, including the
    // selector-six water branch. Unknown source classes must still delete.
    let replacement = match source_class {
        52 | 53 => 74,
        68 | 69 => 76,
        87 => 88,
        _ => return ProjectileSurfaceResponse::Delete,
    };
    match selector {
        6 => ProjectileSurfaceResponse::ConvertToParticleClass(73),
        0..=5 | 8..=12 => ProjectileSurfaceResponse::ConvertToParticleClass(replacement),
        7 | 13..=u8::MAX => ProjectileSurfaceResponse::Delete,
    }
}

/// Exact ground-tail response selected by `FUN_0043E3D0` after a projectile has
/// already been replaced in place by `FUN_00442420`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileReplacementGroundResponse {
    ConvertToParticleClass73,
    /// `FUN_00441770` attempts one class-43/75 child, then the enclosing
    /// dispatcher deletes the parent regardless of allocation success.
    EmitSurfaceChildAndDelete,
    Delete,
}

pub const PROJECTILE_REPLACEMENT_GROUND_CALLBACK_VA: u32 = 0x0043_E3D0;

pub const fn projectile_replacement_ground_response(
    selector: u8,
) -> ProjectileReplacementGroundResponse {
    match selector {
        0..=6 | 8 | 10 => ProjectileReplacementGroundResponse::ConvertToParticleClass73,
        7 | 9 => ProjectileReplacementGroundResponse::EmitSurfaceChildAndDelete,
        11..=u8::MAX => ProjectileReplacementGroundResponse::Delete,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::damage::TYPE_46_DAMAGE_PROFILE;

    fn type_47_descriptor() -> ProjectileEmitterDescriptor {
        ProjectileEmitterDescriptor {
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
            variable_bindings: [0; 4],
        }
    }

    fn profile() -> ProjectileEmitterProfile {
        ProjectileEmitterProfile::from_descriptor(type_47_descriptor())
            .expect("exact type-47 profile")
    }

    fn accepted_input() -> ProjectileEmissionInput {
        ProjectileEmissionInput {
            scheduler_allows: true,
            mode_allows: true,
            elapsed_us: 20_000,
            source_id: 47,
            source_position_raw: [0, 200, 0],
            target: Some(ProjectileTargetCandidate {
                target_id: 46,
                position_raw: [1_000, 300, -1_000],
                aim_error_raw: 0,
                in_forward_half_space: true,
            }),
        }
    }

    #[test]
    fn type_47_static_profile_resolves_exact_method_particle_and_damage_data() {
        let profile = profile();
        assert_eq!(profile.method, PROJECTILE_METHOD_30);
        assert_eq!(profile.speed_raw(), 3_000);
        assert_eq!(profile.descriptor.sound_id, 70);

        let particle = class_87_particle_spec();
        assert_eq!(
            particle.frames.map(|frame| frame.sprite_id),
            [840, 841, 842]
        );
        assert_eq!(particle.descriptor.frame_count(), 3);
        assert_eq!(particle.descriptor.lifetime_ticks(), 200);
        assert_eq!(
            particle.descriptor.raw_u32(0x14),
            particle.update_callback_va
        );
        assert_eq!(
            particle.descriptor.raw_u32(0x18),
            particle.surface_callback_va
        );
        assert_eq!(
            particle.descriptor.raw_u32(0x1c),
            particle.entity_hit_callback_va
        );
        assert_eq!(particle.descriptor.raw_u32(0x20), particle.damage_packet_va);
        assert_eq!(
            particle.descriptor.raw_u32(0x24),
            particle.static_hit_callback_va
        );
        assert_ne!(particle.entity_hit_callback_va, 0x0044_2950);
        assert_eq!(particle.damage_packet, TYPE_47_PROJECTILE_DAMAGE_PACKET);
        assert_eq!(
            particle
                .damage_packet
                .filtered_raw(Some(&TYPE_46_DAMAGE_PROFILE)),
            (1_000 - TYPE_46_DAMAGE_PROFILE.thresholds_raw[2])
                * TYPE_46_DAMAGE_PROFILE.multipliers_q8[2]
                / 256
                + (1_000 - TYPE_46_DAMAGE_PROFILE.thresholds_raw[6])
                    * TYPE_46_DAMAGE_PROFILE.multipliers_q8[6]
                    / 256
        );
    }

    #[test]
    fn unsupported_request_shapes_are_explicit() {
        let mut descriptor = type_47_descriptor();
        descriptor.projectile_method = 31;
        assert_eq!(
            ProjectileEmitterProfile::from_descriptor(descriptor),
            Err(ProjectileEmitterProfileError::UnknownProjectileMethod(31))
        );

        descriptor = type_47_descriptor();
        descriptor.stochastic_gate_mode = 1;
        assert_eq!(
            ProjectileEmitterProfile::from_descriptor(descriptor),
            Err(ProjectileEmitterProfileError::UnsupportedStochasticGateMode(1))
        );
    }

    #[test]
    fn pre_gates_and_invalid_delta_consume_no_shared_rng() {
        for input in [
            ProjectileEmissionInput {
                scheduler_allows: false,
                ..accepted_input()
            },
            ProjectileEmissionInput {
                mode_allows: false,
                ..accepted_input()
            },
            ProjectileEmissionInput {
                elapsed_us: 0,
                ..accepted_input()
            },
        ] {
            let mut calls = 0;
            let decision = plan_projectile_emission(profile(), input, &mut || {
                calls += 1;
                0
            });
            assert!(matches!(
                decision,
                ProjectileEmissionDecision::PreGateRejected
                    | ProjectileEmissionDecision::InvalidElapsed
            ));
            assert_eq!(calls, 0);
        }
    }

    #[test]
    fn cadence_rejection_consumes_exactly_one_shared_rng_value() {
        let mut samples = [1, 2, 3].into_iter();
        let mut calls = 0;
        let decision = plan_projectile_emission(profile(), accepted_input(), &mut || {
            calls += 1;
            samples.next().unwrap()
        });
        assert_eq!(
            decision,
            ProjectileEmissionDecision::CadenceRejected {
                cadence_sample: 1,
                cadence_divisor: 37,
            }
        );
        assert_eq!(calls, 1);
    }

    #[test]
    fn post_cadence_target_rejection_does_not_consume_spread_rng() {
        let mut input = accepted_input();
        input.target.as_mut().unwrap().position_raw[0] = 0x0600;
        let mut samples = [0, 2, 3].into_iter();
        let mut calls = 0;
        let decision = plan_projectile_emission(profile(), input, &mut || {
            calls += 1;
            samples.next().unwrap()
        });
        assert_eq!(
            decision,
            ProjectileEmissionDecision::TargetRejected {
                cadence_sample: 0,
                cadence_divisor: 37,
                reason: ProjectileTargetRejection::OutsideAxisTolerance,
            }
        );
        assert_eq!(calls, 1, "strict ±0x600 gate precedes spread draws");
    }

    #[test]
    fn successful_emission_consumes_cadence_then_x_and_z_spread() {
        let mut samples = [0, 0, 0x8000].into_iter();
        let mut calls = 0;
        let decision = plan_projectile_emission(profile(), accepted_input(), &mut || {
            calls += 1;
            samples.next().unwrap()
        });
        let ProjectileEmissionDecision::Emit(request) = decision else {
            panic!("accepted request should emit");
        };

        assert_eq!(calls, 3);
        assert_eq!(request.source_id, 47);
        assert_eq!(request.target_id, 46);
        assert_eq!(request.target_position_raw, [900, 300, -1_000]);
        assert_eq!(request.projectile_method, 30);
        assert_eq!(request.particle_class, 87);
        assert_eq!(request.speed_raw, 3_000);
        assert_eq!(request.sound_id, 70);
        assert_eq!(request.damage_packet, TYPE_47_PROJECTILE_DAMAGE_PACKET);
        assert_eq!(
            request.random_samples,
            ProjectileEmissionRandomSamples {
                cadence: 0,
                spread_x: 0,
                spread_z: 0x8000,
            }
        );
    }

    #[test]
    fn aim_and_forward_half_space_are_post_cadence_and_high_threshold_bypasses_both() {
        let mut input = accepted_input();
        input.target.as_mut().unwrap().aim_error_raw = 16_000;
        input.target.as_mut().unwrap().in_forward_half_space = false;
        let mut calls = 0;
        assert_eq!(
            plan_projectile_emission(profile(), input, &mut || {
                calls += 1;
                0
            }),
            ProjectileEmissionDecision::TargetRejected {
                cadence_sample: 0,
                cadence_divisor: 37,
                reason: ProjectileTargetRejection::AimError,
            }
        );
        assert_eq!(calls, 1);

        input.target.as_mut().unwrap().aim_error_raw = 0;
        assert_eq!(
            plan_projectile_emission(profile(), input, &mut || {
                calls += 1;
                0
            }),
            ProjectileEmissionDecision::TargetRejected {
                cadence_sample: 0,
                cadence_divisor: 37,
                reason: ProjectileTargetRejection::OutsideForwardHalfSpace,
            }
        );
        assert_eq!(calls, 2);

        input.target.as_mut().unwrap().aim_error_raw = 16_000;
        let mut bypass_descriptor = type_47_descriptor();
        bypass_descriptor.aim_threshold_raw = 0x8000;
        let bypass_profile = ProjectileEmitterProfile::from_descriptor(bypass_descriptor).unwrap();
        let decision = plan_projectile_emission(bypass_profile, input, &mut || {
            calls += 1;
            0
        });
        assert!(matches!(decision, ProjectileEmissionDecision::Emit(_)));
        assert_eq!(calls, 5);
    }

    #[test]
    fn class_87_surface_selector_policy_is_exact_and_bounded() {
        for selector in [0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 12] {
            assert_eq!(
                projectile_surface_response(87, selector),
                ProjectileSurfaceResponse::ConvertToParticleClass(88)
            );
        }
        assert_eq!(
            projectile_surface_response(87, 6),
            ProjectileSurfaceResponse::ConvertToParticleClass(73)
        );
        assert_eq!(
            projectile_surface_response(87, 7),
            ProjectileSurfaceResponse::Delete
        );
        assert_eq!(
            projectile_surface_response(87, 13),
            ProjectileSurfaceResponse::Delete
        );
        assert_eq!(
            projectile_surface_response(87, u8::MAX),
            ProjectileSurfaceResponse::Delete
        );
    }

    #[test]
    fn class_87_replacement_ground_selector_policy_is_exact_and_bounded() {
        for selector in [0, 1, 2, 3, 4, 5, 6, 8, 10] {
            assert_eq!(
                projectile_replacement_ground_response(selector),
                ProjectileReplacementGroundResponse::ConvertToParticleClass73
            );
        }
        for selector in [7, 9] {
            assert_eq!(
                projectile_replacement_ground_response(selector),
                ProjectileReplacementGroundResponse::EmitSurfaceChildAndDelete
            );
        }
        for selector in [11, 12, 13, u8::MAX] {
            assert_eq!(
                projectile_replacement_ground_response(selector),
                ProjectileReplacementGroundResponse::Delete
            );
        }
    }
}
