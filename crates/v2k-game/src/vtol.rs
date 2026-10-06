//! Retail type-46 VTOL thrust and ride-height force projection for the captured
//! normal Sub-G branch (`+0x3F == 0`, `+0x40 == 0`, `+0x42 == 1`,
//! `+0x43 == 0`).
//!
//! `FUN_00445310` computes the manual and terrain-relative lift components,
//! including fuel use and the near-surface trigonometric boost. Its Sub-G
//! runtime fields are then consumed by `FUN_0041B210`, which attenuates manual
//! lift at altitude and projects both components through the previous frame's
//! signed-Q31 body-up basis. Common gravity, water response, drag, and position
//! integration happen later and deliberately do not belong in this module.

use v2k_formats::fixed_math::retail_sine_q15;

use crate::hover::{q31_mul, HoverBasis, HoverFrameForces, RETAIL_FRAME_DELTA_MAX_US};

/// Inputs retained by the player callback for the entity callback's VTOL
/// force phase in the same retail update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VtolControlFrame {
    pub(crate) previous_basis: HoverBasis,
    /// Body pitch consumed by the near-surface correction before A690.
    pub(crate) pitch_before_a690_raw: i16,
    /// Post-Sub-D body roll consumed by the near-surface correction before A690.
    pub(crate) roll_before_a690_raw: i16,
    /// Signed controller throttle in the executable's Q16 input domain.
    pub(crate) throttle_q16: i32,
    pub(crate) has_fuel: bool,
}

/// The mode-specific force packet passed from player control to entity update.
///
/// Hover and VTOL share the later environment tail, but their specialized
/// callbacks are intentionally distinct. Keeping the distinction explicit
/// avoids encoding the mode as an absent packet or another sentinel value.
#[derive(Debug, Clone, Copy)]
pub enum VehicleFrameForces {
    Hover(HoverFrameForces),
    Vtol(VtolControlFrame),
    /// Class 25 installs no specialized +28 force callback. E870 retains the
    /// type-46 common environment/integration policy (style masks both zero).
    DyingCommon,
}

impl VehicleFrameForces {
    /// Retail requests the forced VTOL→Hover return only when a VTOL callback
    /// begins with an empty tank. Fuel reaching zero during this frame must not
    /// make the switch one callback early.
    pub fn vtol_started_without_fuel(self) -> bool {
        matches!(self, Self::Vtol(frame) if !frame.has_fuel)
    }

    /// True only for the powered VTOL branch which later performs the retail
    /// post-burn low-fuel warning check.
    pub fn vtol_started_with_fuel(self) -> bool {
        matches!(self, Self::Vtol(frame) if frame.has_fuel)
    }
}

/// Runtime turbo state used by both the manual-lift multiplier and its higher
/// altitude-attenuation threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VtolBoost {
    Inactive,
    Active,
}

/// `FUN_00456F10() == 4` bypasses only manual height attenuation. It does not
/// bypass upward-velocity attenuation or terrain-relative assist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VtolHeightPolicy {
    RetailAttenuation,
    BypassHeightAttenuation,
}

/// Complete raw input to `FUN_0041A690`'s dormant `Sub-G+0x42 == 0`
/// terrain-derived attitude branch.
///
/// The accepted player traces never enter this branch. This request therefore
/// remains a detached retail-arithmetic contract: no live player adapter may
/// construct it until a writer/caller capture proves the owning transition.
/// The caller must supply the three surface samples selected by retail:
///
/// - `coarse_center_surface_raw` is the current-cell terrain/static-sea sample
///   used to derive the pitch target;
/// - `current_surface_raw` and `projected_surface_raw` are the two
///   `FUN_0041AC40` terrain-or-water samples used for clearance/rate control.
///
/// Keeping those values explicit avoids silently substituting the ordinary
/// VTOL animated ride probes for this distinct controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VtolTerrainDerivedAttitudeRequest {
    pub current_pitch_raw: i16,
    /// Low signed word retained from runtime Sub-G dword `+0x1C`.
    pub saved_target_raw: i16,
    /// Static Sub-G dword `+0x1C`.
    pub target_limit_raw: i32,
    /// Runtime Sub-G signed word `+0x1E`, applied after target smoothing.
    pub manual_pitch_delta_raw: i16,
    /// Runtime Sub-G byte `+0x3C`.
    pub invert_target: bool,
    pub center_y_raw: i16,
    pub active_model_radius_raw: u16,
    pub coarse_center_surface_raw: i16,
    pub current_surface_raw: i16,
    pub projected_surface_raw: i16,
    /// Static Sub-G dwords `+0x10` and `+0x14`.
    pub rate_min_raw: i32,
    pub rate_max_raw: i32,
    /// Runtime Sub-G dword `+0x38`.
    pub target_clearance_raw: i32,
    /// Static Sub-G dword `+0x20`.
    pub max_linear_velocity_raw: i32,
    /// Runtime Sub-G byte `+0x3D` entering `FUN_0041AC40`.
    pub prior_clearance_flag: bool,
    pub velocity_raw: [i16; 3],
}

/// Detached result of the statically complete `Sub-G+0x42 == 0` prefix.
///
/// Animation-channel publication in `FUN_0041AA60` is independent of these
/// values and remains owned by the component that supplies its seven joint
/// bindings. `FUN_0041B210` projection follows this outcome but is already a
/// separate phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VtolTerrainDerivedAttitudeOutcome {
    pub target_pitch_raw: i32,
    pub pitch_raw: i16,
    pub clearance_raw: i32,
    /// Runtime Sub-G dword `+0x34`.
    pub rate_raw: i32,
    /// Runtime Sub-G byte `+0x3D`.
    pub clearance_flag: bool,
    pub velocity_raw: [i16; 3],
}

const TERRAIN_DERIVED_POSITIVE_Y_CAP_RAW: i16 = 0x05dc;
const TERRAIN_DERIVED_DESCENT_FLAG_FLOOR_RAW: i16 = -0x05dd;

/// Velocity-projected X/Z probe consumed by `FUN_0041AC40`.
///
/// Retail advances 0x300 raw units through the entity's previous-frame
/// forward Q31 basis, narrowing each Q31 product into the wrapping signed-word
/// world domain. Y is deliberately unchanged.
pub fn vtol_terrain_projected_probe_raw(
    position_raw: [i16; 3],
    forward_basis_q31: [i32; 3],
) -> [i16; 3] {
    [
        position_raw[0].wrapping_add(q31_mul(forward_basis_q31[0], 0x300) as i16),
        position_raw[1],
        position_raw[2].wrapping_add(q31_mul(forward_basis_q31[2], 0x300) as i16),
    ]
}

/// Plan the exact, detached terrain-derived attitude/rate branch shared by
/// `FUN_0041A690` and `FUN_0041AC40`.
///
/// `None` is a portable fail-closed guard for malformed decoded bounds. Retail
/// assumes valid authored Sub-G data and would otherwise divide/use those
/// fields directly. There is intentionally no call from the normal player
/// update: all accepted player samples have `Sub-G+0x42 == 1`.
pub fn plan_vtol_terrain_derived_attitude_raw(
    request: VtolTerrainDerivedAttitudeRequest,
) -> Option<VtolTerrainDerivedAttitudeOutcome> {
    if request.rate_min_raw > request.rate_max_raw
        || !(1..=i32::from(i16::MAX)).contains(&request.max_linear_velocity_raw)
    {
        return None;
    }

    let underside_over_coarse_surface = i32::from(request.center_y_raw)
        .wrapping_sub(i32::from(request.active_model_radius_raw))
        .wrapping_sub(i32::from(request.coarse_center_surface_raw));
    let terrain_target = i32::from(request.saved_target_raw)
        .wrapping_add(underside_over_coarse_surface.wrapping_mul(2))
        .wrapping_sub(2_000);
    let mut target_pitch_raw = terrain_target.min(request.target_limit_raw);
    if request.invert_target {
        target_pitch_raw = target_pitch_raw.wrapping_neg();
    }

    let mut pitch_raw = request.current_pitch_raw;
    if target_pitch_raw != 0 && i32::from(pitch_raw) != target_pitch_raw {
        let correction = (i32::from(pitch_raw).wrapping_sub(target_pitch_raw)) >> 4;
        pitch_raw = pitch_raw.wrapping_sub(correction as i16);
    }
    pitch_raw = pitch_raw.wrapping_add(request.manual_pitch_delta_raw);

    let controlling_surface_raw = request
        .current_surface_raw
        .max(request.projected_surface_raw);
    let clearance_raw = i32::from(request.center_y_raw)
        .wrapping_sub(i32::from(request.active_model_radius_raw))
        .wrapping_sub(i32::from(controlling_surface_raw));
    let midpoint_raw = request.rate_min_raw.wrapping_add(request.rate_max_raw) / 2;
    let rate_raw = midpoint_raw
        .wrapping_add(request.target_clearance_raw.wrapping_sub(clearance_raw) / 0x28)
        .clamp(request.rate_min_raw, request.rate_max_raw);

    let horizontal_limit = request.max_linear_velocity_raw;
    let mut velocity_raw = request.velocity_raw;
    velocity_raw[0] = i32::from(velocity_raw[0]).clamp(-horizontal_limit, horizontal_limit) as i16;
    // Static Sub-G +0x20 applies only to X/Z. Retail uses a literal 0x5DC
    // positive/upward Y cap and deliberately leaves fast descent unclamped.
    if velocity_raw[1] > TERRAIN_DERIVED_POSITIVE_Y_CAP_RAW {
        velocity_raw[1] = TERRAIN_DERIVED_POSITIVE_Y_CAP_RAW;
    }
    velocity_raw[2] = i32::from(velocity_raw[2]).clamp(-horizontal_limit, horizontal_limit) as i16;

    let descent_within_limit = velocity_raw[1] > TERRAIN_DERIVED_DESCENT_FLAG_FLOOR_RAW;
    let clearance_flag = if descent_within_limit && clearance_raw > 1_200 {
        true
    } else if descent_within_limit && clearance_raw > 1_099 {
        request.prior_clearance_flag
    } else {
        false
    };

    Some(VtolTerrainDerivedAttitudeOutcome {
        target_pitch_raw,
        pitch_raw,
        clearance_raw,
        rate_raw,
        clearance_flag,
        velocity_raw,
    })
}

/// Complete data needed by the pure VTOL specialized-force phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VtolForceRequest {
    pub(crate) frame: VtolControlFrame,
    /// Entity-center Y in signed engine 8.8 world units.
    pub(crate) center_height_raw: i32,
    /// Half of the active model's unsigned radius/height word.
    pub(crate) active_model_half_height_raw: i32,
    /// Animated ride surface sampled at the entity center.
    pub(crate) ride_surface_height_raw: i32,
    /// Minimum of the center and X/Z +/-0x100 ride-surface probes.
    pub(crate) minimum_probe_surface_height_raw: i32,
    /// Self mass plus attached masses, matching `FUN_004185C0`.
    pub(crate) total_mass_raw: u32,
    pub(crate) elapsed_micros: u32,
    pub(crate) turbo: VtolBoost,
    pub(crate) height_policy: VtolHeightPolicy,
}

/// Observable intermediate values from the two retail force callbacks.
///
/// These fields are useful both to update fuel and to compare the port against
/// passive Sub-G runtime captures without duplicating the force arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VtolForceOutcome {
    pub(crate) clearance_raw: i32,
    pub(crate) minimum_probe_clearance_raw: i32,
    pub(crate) base_manual_thrust_raw: i32,
    pub(crate) assist_before_near_correction_raw: i32,
    /// Sub-G runtime `+0x20`, after near correction and turbo.
    pub(crate) manual_lift_before_attenuation_raw: i32,
    /// Sub-G runtime `+0x24`, after near correction.
    pub(crate) assist_lift_raw: i32,
    pub(crate) manual_lift_after_attenuation_raw: i32,
    /// Fuel units consumed by `FUN_00445A40`, computed before the near boost.
    pub(crate) fuel_burn_raw: i32,
}

/// Apply the captured normal branch of `FUN_00445310`'s lift construction,
/// followed by `FUN_0041B210`'s previous-basis projection. The velocity words
/// wrap exactly as retail does. Alternate Sub-G states (`+0x3F != 0`,
/// `+0x40 != 0`, `+0x42 == 0`, or `+0x43 != 0`) remain explicit evidence
/// gates rather than being folded into this branch.
pub(crate) fn apply_vtol_force_raw(
    request: VtolForceRequest,
    velocity: &mut [i16; 3],
) -> VtolForceOutcome {
    let elapsed_micros = request.elapsed_micros.min(RETAIL_FRAME_DELTA_MAX_US);
    let clearance_raw = request
        .center_height_raw
        .wrapping_sub(request.active_model_half_height_raw)
        .wrapping_sub(request.ride_surface_height_raw);
    let minimum_probe_clearance_raw = request
        .center_height_raw
        .wrapping_sub(request.active_model_half_height_raw)
        .wrapping_sub(request.minimum_probe_surface_height_raw);

    if !request.frame.has_fuel {
        return VtolForceOutcome {
            clearance_raw,
            minimum_probe_clearance_raw,
            base_manual_thrust_raw: 0,
            assist_before_near_correction_raw: 0,
            manual_lift_before_attenuation_raw: 0,
            assist_lift_raw: 0,
            manual_lift_after_attenuation_raw: 0,
            fuel_burn_raw: 0,
        };
    }

    let base_manual_thrust_raw = request.frame.throttle_q16.wrapping_mul(0x13) >> 16;
    let assist_before_near_correction_raw = altitude_assist_raw(
        clearance_raw,
        i32::from(velocity[1]),
        request.frame.previous_basis.up[1],
    );
    let pre_near_total = base_manual_thrust_raw.wrapping_add(assist_before_near_correction_raw);
    let fuel_burn_raw = fuel_burn_raw(pre_near_total, elapsed_micros);

    let (mut manual_lift_raw, assist_lift_raw) = near_surface_lift_raw(
        base_manual_thrust_raw,
        assist_before_near_correction_raw,
        clearance_raw,
        request.frame.pitch_before_a690_raw,
        request.frame.roll_before_a690_raw,
        request.frame.previous_basis.up[1],
    );
    if request.turbo == VtolBoost::Active {
        manual_lift_raw = manual_lift_raw.wrapping_mul(3) / 2;
    }

    let manual_lift_after_attenuation_raw = attenuate_manual_lift_raw(
        manual_lift_raw,
        minimum_probe_clearance_raw,
        i32::from(velocity[1]),
        request.turbo,
        request.height_policy,
    );

    // Mass belongs only to FUN_0041B210's projection. The preceding player
    // callback has already constructed lift and burned fuel, so an invalid
    // zero mass suppresses the division without retroactively suppressing the
    // controller-side burn.
    if request.total_mass_raw != 0 {
        project_lift_raw(
            request.frame.previous_basis.up,
            manual_lift_raw,
            manual_lift_after_attenuation_raw,
            assist_lift_raw,
            request.total_mass_raw,
            elapsed_micros,
            velocity,
        );
    }

    VtolForceOutcome {
        clearance_raw,
        minimum_probe_clearance_raw,
        base_manual_thrust_raw,
        assist_before_near_correction_raw,
        manual_lift_before_attenuation_raw: manual_lift_raw,
        assist_lift_raw,
        manual_lift_after_attenuation_raw,
        fuel_burn_raw,
    }
}

/// Strict `[-299, 999]` terrain-relative lift window from `FUN_00445310`.
fn altitude_assist_raw(clearance_raw: i32, velocity_y_raw: i32, body_up_y_q31: i32) -> i32 {
    if body_up_y_q31 <= 0 || !(-299..=999).contains(&clearance_raw) {
        return 0;
    }
    let proportional = (1000 - clearance_raw.max(0)).wrapping_mul(0x18) / 1000;
    let damping = velocity_y_raw.wrapping_mul(5) / 1000;
    proportional.wrapping_sub(damping).max(0)
}

/// `FUN_00445A40`'s positive Q31 product. Its input is the absolute total lift
/// multiplied by 16, and it runs before the near-surface correction.
fn fuel_burn_raw(total_lift_raw: i32, elapsed_micros: u32) -> i32 {
    let lift_q4 = total_lift_raw.wrapping_mul(0x10).wrapping_abs();
    let time_q11 = elapsed_micros.wrapping_shl(11);
    ((i64::from(lift_q4) * i64::from(time_q11)) >> 31) as i32
}

/// Apply the strict `(-300, 800)` trigonometric boost independently to manual
/// and assist lift. Retail constructs its sine as a duplicated signed Q15 word
/// before squaring it in the signed-Q31 domain.
fn near_surface_lift_raw(
    manual_raw: i32,
    assist_raw: i32,
    clearance_raw: i32,
    pitch_raw: i16,
    roll_raw: i16,
    body_up_y_q31: i32,
) -> (i32, i32) {
    if body_up_y_q31 <= 0 || clearance_raw <= -300 || clearance_raw >= 800 {
        return (manual_raw, assist_raw);
    }

    let angle = pitch_raw
        .wrapping_add(roll_raw)
        .wrapping_add(0x4000u16 as i16);
    let sine_q31 = duplicated_sine_q31(angle);
    let sine_squared_q31 = q31_mul(sine_q31, sine_q31);
    let distance_raw = 800i32.wrapping_sub(clearance_raw);

    (
        near_surface_component_raw(manual_raw, sine_squared_q31, distance_raw),
        near_surface_component_raw(assist_raw, sine_squared_q31, distance_raw),
    )
}

fn duplicated_sine_q31(angle_raw: i16) -> i32 {
    // Ghidra renders the positive path as CONCAT22(word, word), but retail
    // negates that already-concatenated magnitude in negative quadrants. This
    // is signed `word * 0x10001`, not bitwise duplication of a negative word.
    retail_sine_q15(angle_raw as u16 as u32).wrapping_mul(0x1_0001)
}

fn near_surface_component_raw(component: i32, sine_squared_q31: i32, distance: i32) -> i32 {
    let correction = q31_mul(component, sine_squared_q31).wrapping_mul(distance) / 800;
    component.wrapping_add(correction)
}

fn attenuate_manual_lift_raw(
    manual_lift_raw: i32,
    clearance_raw: i32,
    velocity_y_raw: i32,
    turbo: VtolBoost,
    height_policy: VtolHeightPolicy,
) -> i32 {
    let mut lift = manual_lift_raw;
    if height_policy == VtolHeightPolicy::RetailAttenuation {
        let threshold = match turbo {
            VtolBoost::Inactive => 0x800,
            VtolBoost::Active => 0xc00,
        };
        if clearance_raw > threshold {
            // The decompiled sign-bias expression is signed division by 128
            // with truncation toward zero. This branch guarantees a positive
            // numerator, but retaining the ordinary division states the rule.
            let height_divisor = ((clearance_raw - threshold) / 0x80).max(1);
            lift /= height_divisor;
        }
    }

    // Only positive/upward velocity increases the divisor; zero and downward
    // motion both select one after the retail signed division by 512.
    let velocity_divisor = (velocity_y_raw / 0x200).max(1);
    lift / velocity_divisor
}

fn project_lift_raw(
    body_up_q31: [i32; 3],
    manual_horizontal_lift_raw: i32,
    manual_vertical_lift_raw: i32,
    assist_lift_raw: i32,
    total_mass_raw: u32,
    elapsed_micros: u32,
    velocity: &mut [i16; 3],
) {
    let time_raw = (elapsed_micros >> 5) as i32;
    let total_mass_raw = total_mass_raw as i32;
    let mass_times_two = total_mass_raw.wrapping_mul(2);
    // FUN_0041B210 applies ceiling/upward-velocity attenuation only to the
    // body-up Y projection. Horizontal pitched-flight thrust continues using
    // runtime Sub-G +0x20 unchanged.
    let manual_horizontal_rate = time_raw.wrapping_mul(manual_horizontal_lift_raw) / mass_times_two;
    let manual_vertical_rate = time_raw.wrapping_mul(manual_vertical_lift_raw) / mass_times_two;
    velocity[0] = velocity[0].wrapping_add(q31_mul(body_up_q31[0], manual_horizontal_rate) as i16);
    velocity[1] = velocity[1].wrapping_add(q31_mul(body_up_q31[1], manual_vertical_rate) as i16);
    velocity[2] = velocity[2].wrapping_add(q31_mul(body_up_q31[2], manual_horizontal_rate) as i16);

    if assist_lift_raw == 0 {
        return;
    }
    let assist_vertical_rate = time_raw.wrapping_mul(assist_lift_raw) / mass_times_two;
    let assist_horizontal_rate =
        time_raw.wrapping_mul(assist_lift_raw) / total_mass_raw.wrapping_mul(0x10);
    velocity[0] = velocity[0].wrapping_add(q31_mul(body_up_q31[0], assist_horizontal_rate) as i16);
    velocity[1] = velocity[1].wrapping_add(q31_mul(body_up_q31[1], assist_vertical_rate) as i16);
    velocity[2] = velocity[2].wrapping_add(q31_mul(body_up_q31[2], assist_horizontal_rate) as i16);
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEVEL_BASIS: HoverBasis = HoverBasis {
        lateral: [0x7ffe_0000, 0, 0],
        up: [0, 0x7ffe_0000, 0],
        forward: [0, 0, 0x7ffe_0000],
    };

    fn request() -> VtolForceRequest {
        VtolForceRequest {
            frame: VtolControlFrame {
                previous_basis: LEVEL_BASIS,
                pitch_before_a690_raw: 0,
                roll_before_a690_raw: 0,
                throttle_q16: 0x1_0000,
                has_fuel: true,
            },
            center_height_raw: 150,
            active_model_half_height_raw: 0,
            ride_surface_height_raw: 0,
            minimum_probe_surface_height_raw: 0,
            total_mass_raw: 100,
            elapsed_micros: 20_000,
            turbo: VtolBoost::Inactive,
            height_policy: VtolHeightPolicy::RetailAttenuation,
        }
    }

    fn terrain_attitude_request() -> VtolTerrainDerivedAttitudeRequest {
        VtolTerrainDerivedAttitudeRequest {
            current_pitch_raw: 1_000,
            saved_target_raw: 0,
            target_limit_raw: 0x4000,
            manual_pitch_delta_raw: 50,
            invert_target: false,
            center_y_raw: 2_000,
            active_model_radius_raw: 100,
            coarse_center_surface_raw: 200,
            current_surface_raw: 250,
            projected_surface_raw: 300,
            rate_min_raw: 600,
            rate_max_raw: 700,
            target_clearance_raw: 120,
            max_linear_velocity_raw: 1_500,
            prior_clearance_flag: false,
            velocity_raw: [2_000, 2_000, -2_000],
        }
    }

    #[test]
    fn terrain_derived_probe_uses_previous_forward_basis_and_wraps_words() {
        let level_forward = [0, 0, 0x7fff_ffff];
        assert_eq!(
            vtol_terrain_projected_probe_raw([100, 200, 300], level_forward),
            [100, 200, 1_067]
        );

        assert_eq!(
            vtol_terrain_projected_probe_raw(
                [i16::MAX - 10, -50, i16::MAX - 20],
                [0x7fff_ffff, 0, 0x7fff_ffff],
            ),
            [-32_012, -50, -32_022]
        );
    }

    #[test]
    fn terrain_derived_target_rate_and_velocity_follow_a690_ac40_order() {
        let outcome = plan_vtol_terrain_derived_attitude_raw(terrain_attitude_request())
            .expect("authored bounds");

        // target = 2 * ((2000 - 100) - 200) - 2000 = 1400.
        // A690 moves 1/16 toward it before adding the external +50.
        assert_eq!(outcome.target_pitch_raw, 1_400);
        assert_eq!(outcome.pitch_raw, 1_075);
        // AC40 chooses the higher projected surface: 2000 - 100 - 300.
        assert_eq!(outcome.clearance_raw, 1_600);
        // 650 + trunc((120 - 1600) / 40) = 613.
        assert_eq!(outcome.rate_raw, 613);
        assert_eq!(outcome.velocity_raw, [1_500, 1_500, -1_500]);
        assert!(outcome.clearance_flag);
    }

    #[test]
    fn terrain_derived_vertical_cap_is_independent_of_authored_xz_limit() {
        let mut request = terrain_attitude_request();
        request.max_linear_velocity_raw = 900;
        request.velocity_raw = [2_000, 2_000, -2_000];
        let outcome = plan_vtol_terrain_derived_attitude_raw(request).expect("authored bounds");

        assert_eq!(outcome.velocity_raw, [900, 1_500, -900]);

        request.velocity_raw[1] = -1_500;
        assert!(
            plan_vtol_terrain_derived_attitude_raw(request)
                .expect("authored bounds")
                .clearance_flag,
            "retail's descent hysteresis floor is the literal -1501, not -Sub-G+0x20"
        );
        request.velocity_raw[1] = -1_501;
        assert!(
            !plan_vtol_terrain_derived_attitude_raw(request)
                .expect("authored bounds")
                .clearance_flag
        );
    }

    #[test]
    fn terrain_derived_target_cap_inversion_and_signed_shift_are_exact() {
        let mut request = terrain_attitude_request();
        request.current_pitch_raw = -1;
        request.saved_target_raw = 10_000;
        request.target_limit_raw = 1_200;
        request.manual_pitch_delta_raw = -10;
        request.invert_target = true;
        let outcome = plan_vtol_terrain_derived_attitude_raw(request).expect("authored bounds");

        assert_eq!(outcome.target_pitch_raw, -1_200);
        // (-1 - -1200) >> 4 = 74, then the manual -10 is applied.
        assert_eq!(outcome.pitch_raw, -85);
    }

    #[test]
    fn terrain_derived_clearance_flag_retains_only_inside_hysteresis_band() {
        let mut request = terrain_attitude_request();
        request.center_y_raw = 1_550;
        request.current_surface_raw = 300;
        request.projected_surface_raw = 300;
        request.velocity_raw[1] = -1_500;
        request.prior_clearance_flag = true;
        let retained = plan_vtol_terrain_derived_attitude_raw(request).expect("authored bounds");
        assert_eq!(retained.clearance_raw, 1_150);
        assert!(retained.clearance_flag);

        request.prior_clearance_flag = false;
        assert!(
            !plan_vtol_terrain_derived_attitude_raw(request)
                .expect("authored bounds")
                .clearance_flag
        );

        request.prior_clearance_flag = true;
        request.velocity_raw[1] = -1_501;
        assert!(
            !plan_vtol_terrain_derived_attitude_raw(request)
                .expect("authored bounds")
                .clearance_flag,
            "retail does not clamp downward Y before testing the flag"
        );
    }

    #[test]
    fn malformed_terrain_derived_bounds_fail_closed() {
        let mut request = terrain_attitude_request();
        request.rate_min_raw = 701;
        assert_eq!(plan_vtol_terrain_derived_attitude_raw(request), None);

        request = terrain_attitude_request();
        request.max_linear_velocity_raw = 0;
        assert_eq!(plan_vtol_terrain_derived_attitude_raw(request), None);
    }

    #[test]
    fn assist_and_near_windows_keep_retail_strict_edges() {
        assert_eq!(altitude_assist_raw(-300, 0, 1), 0);
        assert_eq!(altitude_assist_raw(-299, 0, 1), 24);
        assert_eq!(altitude_assist_raw(999, -1000, 1), 5);
        assert_eq!(altitude_assist_raw(1000, -1000, 1), 0);
        assert_eq!(altitude_assist_raw(0, 5000, 1), 0);
        assert_eq!(altitude_assist_raw(0, 0, 0), 0);

        assert_eq!(near_surface_lift_raw(19, 24, -300, 0, 0, 1), (19, 24));
        assert_ne!(near_surface_lift_raw(19, 24, -299, 0, 0, 1), (19, 24));
        assert_ne!(
            near_surface_lift_raw(1000, 1000, 799, 0, 0, 1),
            (1000, 1000)
        );
        assert_eq!(near_surface_lift_raw(19, 24, 800, 0, 0, 1), (19, 24));
    }

    #[test]
    fn duplicated_sine_negates_the_full_q31_magnitude() {
        let angle = -0x1000i16;
        let sine_q15 = retail_sine_q15(angle as u16 as u32);
        assert!(sine_q15 < 0);
        assert_eq!(duplicated_sine_q31(angle), sine_q15.wrapping_mul(0x1_0001));

        let word = sine_q15 as i16 as u16;
        let bitwise_duplicate = (u32::from(word) | (u32::from(word) << 16)) as i32;
        assert_ne!(duplicated_sine_q31(angle), bitwise_duplicate);
    }

    #[test]
    fn fuel_burn_uses_pre_near_total_and_q31_order() {
        // (19 manual + 20 assist) * 16, multiplied by (20_000 << 11), >>31.
        assert_eq!(fuel_burn_raw(39, 20_000), 11);
        assert_eq!(fuel_burn_raw(-39, 20_000), 11);

        let mut velocity = [0; 3];
        let outcome = apply_vtol_force_raw(request(), &mut velocity);
        assert_eq!(outcome.base_manual_thrust_raw, 19);
        assert_eq!(outcome.assist_before_near_correction_raw, 20);
        assert_eq!(outcome.fuel_burn_raw, 11);
        assert!(outcome.manual_lift_before_attenuation_raw > 19);
        assert!(outcome.assist_lift_raw > 20);
    }

    #[test]
    fn reverse_thrust_uses_the_same_signed_projection_and_absolute_fuel_burn() {
        // Both accepted July-16 acceleration protocols isolate RSHIFT alone,
        // UP+RSHIFT, and DOWN+RSHIFT while the runtime remains in the normal
        // +0x42=1/+0x43=0 branch. A clearance of 1000 keeps this unit test out
        // of both the altitude-assist and near-surface windows, leaving the
        // signed `(throttle * 19) >> 16` contract visible by itself.
        let mut reverse = request();
        reverse.frame.throttle_q16 = -0x1_0000;
        reverse.center_height_raw = 1_000;
        reverse.minimum_probe_surface_height_raw = 0;
        let mut velocity = [0; 3];
        let outcome = apply_vtol_force_raw(reverse, &mut velocity);

        assert_eq!(outcome.base_manual_thrust_raw, -19);
        assert_eq!(outcome.assist_before_near_correction_raw, 0);
        assert_eq!(outcome.manual_lift_before_attenuation_raw, -19);
        assert_eq!(outcome.manual_lift_after_attenuation_raw, -19);
        assert_eq!(outcome.fuel_burn_raw, 5);
        assert_eq!(velocity, [0, -59, 0]);
    }

    #[test]
    fn height_and_upward_velocity_divisors_compose() {
        assert_eq!(
            attenuate_manual_lift_raw(
                80,
                0x900,
                0x400,
                VtolBoost::Inactive,
                VtolHeightPolicy::RetailAttenuation,
            ),
            20
        );
        assert_eq!(
            attenuate_manual_lift_raw(
                80,
                0x900,
                0x400,
                VtolBoost::Inactive,
                VtolHeightPolicy::BypassHeightAttenuation,
            ),
            40
        );
        assert_eq!(
            attenuate_manual_lift_raw(
                80,
                0x900,
                -0x400,
                VtolBoost::Inactive,
                VtolHeightPolicy::RetailAttenuation,
            ),
            40
        );
        // Turbo moves the height threshold from 0x800 to 0xC00.
        assert_eq!(
            attenuate_manual_lift_raw(
                80,
                0x900,
                0,
                VtolBoost::Active,
                VtolHeightPolicy::RetailAttenuation,
            ),
            80
        );
    }

    #[test]
    fn manual_height_attenuation_changes_y_but_not_pitched_horizontal_thrust() {
        let mut velocity = [0; 3];
        project_lift_raw(
            [0x4000_0000, 0x4000_0000, 0],
            80,
            20,
            0,
            100,
            20_000,
            &mut velocity,
        );
        assert_eq!(velocity, [125, 31, 0]);
    }

    #[test]
    fn turbo_multiplies_only_manual_lift_after_near_correction() {
        let mut normal_velocity = [0; 3];
        let normal = apply_vtol_force_raw(request(), &mut normal_velocity);
        let mut turbo_request = request();
        turbo_request.turbo = VtolBoost::Active;
        let mut turbo_velocity = [0; 3];
        let turbo = apply_vtol_force_raw(turbo_request, &mut turbo_velocity);

        assert_eq!(turbo.assist_lift_raw, normal.assist_lift_raw);
        assert_eq!(
            turbo.manual_lift_before_attenuation_raw,
            normal.manual_lift_before_attenuation_raw * 3 / 2
        );
        assert_eq!(turbo.fuel_burn_raw, normal.fuel_burn_raw);
    }

    #[test]
    fn july_16_trace_geometry_pins_previous_basis_projection() {
        // 20260716-220810-vtol-acceleration.jsonl sample 0: position -506,
        // model half-height 140, center wave -768, five-probe minimum -817,
        // mass 100, dt 8 ms, pitch -1428, roll 419, and the exact captured
        // previous-frame Q31 body-up row.
        let trace_request = VtolForceRequest {
            frame: VtolControlFrame {
                previous_basis: HoverBasis {
                    lateral: [-116_916_224, 84_738_048, -2_142_502_912],
                    up: [-287_375_360, 2_125_594_624, 99_745_792],
                    forward: [2_124_677_120, 292_225_024, -104_333_312],
                },
                pitch_before_a690_raw: -1428,
                roll_before_a690_raw: 419,
                throttle_q16: 0,
                has_fuel: true,
            },
            center_height_raw: -506,
            active_model_half_height_raw: 140,
            ride_surface_height_raw: -768,
            minimum_probe_surface_height_raw: -817,
            total_mass_raw: 100,
            elapsed_micros: 8_000,
            turbo: VtolBoost::Inactive,
            height_policy: VtolHeightPolicy::RetailAttenuation,
        };
        let mut velocity = [152, 13, -9];
        let outcome = apply_vtol_force_raw(trace_request, &mut velocity);

        assert_eq!(outcome.clearance_raw, 122);
        assert_eq!(outcome.minimum_probe_clearance_raw, 171);
        assert_eq!(outcome.base_manual_thrust_raw, 0);
        assert_eq!(outcome.assist_before_near_correction_raw, 21);
        assert_eq!(outcome.fuel_burn_raw, 2);
        assert_eq!(outcome.assist_lift_raw, 37);
        assert_eq!(velocity, [151, 58, -9]);
    }

    #[test]
    fn no_fuel_suppresses_lift_while_zero_mass_only_suppresses_projection() {
        let mut no_fuel = request();
        no_fuel.frame.has_fuel = false;
        let mut velocity = [10, 20, 30];
        let outcome = apply_vtol_force_raw(no_fuel, &mut velocity);
        assert_eq!(velocity, [10, 20, 30]);
        assert_eq!(outcome.manual_lift_before_attenuation_raw, 0);

        let mut invalid_mass = request();
        invalid_mass.total_mass_raw = 0;
        let outcome = apply_vtol_force_raw(invalid_mass, &mut velocity);
        assert_eq!(velocity, [10, 20, 30]);
        assert_eq!(outcome.base_manual_thrust_raw, 19);
        assert!(outcome.fuel_burn_raw > 0);
    }

    #[test]
    fn projection_wraps_signed_velocity_words() {
        let mut wrapping_request = request();
        wrapping_request.center_height_raw = 2_000;
        wrapping_request.ride_surface_height_raw = 0;
        wrapping_request.minimum_probe_surface_height_raw = 0;
        wrapping_request.height_policy = VtolHeightPolicy::BypassHeightAttenuation;
        wrapping_request.frame.throttle_q16 = i32::MAX;
        wrapping_request.total_mass_raw = 1;
        let mut velocity = [0, i16::MAX, 0];
        apply_vtol_force_raw(wrapping_request, &mut velocity);
        assert!(velocity[1] < 0);
    }
}
