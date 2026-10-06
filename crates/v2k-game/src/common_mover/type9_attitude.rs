//! Detached shared `FUN_0040E640` terrain/surface attitude correction.
//!
//! The retail normal callback runs this phase before environment drag and
//! ground snap whenever effective flag `0x10` is set. It then rebuilds the
//! body basis with `FUN_00413F70` before entering those later phases. The two
//! phases remain explicit here: [`plan_type9_terrain_attitude_raw`] performs
//! only `FUN_0040E640`/`FUN_0041EC70`, while
//! [`Type9TerrainAttitudePlan::rebuild_body_basis`] performs the subsequent
//! Euler-to-Q31 conversion.
//!
//! The API retains its historical Type-9 name because that was its first
//! recovered owner. The routine is not Type-9-specific: the player Hover path
//! now consumes this same planner after its specialized force phase. Do not
//! fork a player-only scalar easing or transition clamp from this shared code.
//!
//! Runtime-owned inputs which are not intrinsic to this calculation remain
//! arguments. In particular, the signed surface-mode byte, active-model
//! extent, water gate, 50-Hz tick, and carry-adjusted elapsed step must come
//! from the live owner; this module does not infer them from nearby data.

use crate::hover::HoverBasis;
use v2k_formats::terrain::{wave_surface_raw, TerrainGrid};

use super::type9_tail::bilinear_terrain_height_raw;

/// Effective normal-callback flag which admits `FUN_0040E640`.
pub const TERRAIN_ATTITUDE_EFFECTIVE_FLAG: u32 = 0x0000_0010;

/// Entity state bit selecting bilinear rather than coarse probe samples.
pub const TERRAIN_ATTITUDE_BILINEAR_STATE_BIT: u32 = 0x0200_0000;

/// Effective normal-callback flag which clamps pitch and roll after correction.
pub const TERRAIN_ATTITUDE_CLAMP_EFFECTIVE_FLAG: u32 = 0x0001_0000;

/// Near-surface `FUN_0041EC70` gain.
pub const TERRAIN_ATTITUDE_NEAR_GAIN_RAW: i32 = 0x100;

/// Far-from-surface `FUN_0041EC70` gain.
pub const TERRAIN_ATTITUDE_FAR_GAIN_RAW: i32 = 0x40;

const ATTITUDE_LIMIT_RAW: i16 = 0x1800;
const NEAR_CLEARANCE_MAX_RAW: i32 = 0x100;
const PROBE_OFFSET_RAW: i16 = 0x80;

/// Complete explicit input to the detached terrain-attitude phase.
#[derive(Debug, Clone, Copy)]
pub struct Type9TerrainAttitudeInput<'a> {
    pub terrain: &'a TerrainGrid,
    pub position_raw: [i16; 3],
    pub pitch_raw: i16,
    pub roll_raw: i16,
    pub lateral_basis_q31: [i32; 3],
    pub forward_basis_q31: [i32; 3],
    pub state_flags: u32,
    pub effective_flags: u32,
    /// Exact selected model-header `+0x08` extent, before retail halves it.
    pub active_model_extent_raw: u16,
    /// Signed descriptor byte resolved by the callback owner. Zero selects
    /// terrain-only probes; every nonzero value admits the water path.
    pub resolved_surface_mode_raw: i8,
    /// Runtime `DAT_004FECE4`, kept distinct from authored terrain metadata.
    pub water_enabled: bool,
    /// Runtime wave clock (`g_default_param`) in 50-Hz ticks.
    pub wave_tick_50hz: i32,
    /// Carry-adjusted owner step. This function deliberately does not cap it.
    pub effective_elapsed_micros: u32,
}

/// Exact detached result of `FUN_0040E640`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type9TerrainAttitudePlan {
    pub pitch_raw: i16,
    pub roll_raw: i16,
    pub gain_raw: i32,
    pub center_surface_y_raw: i16,
    /// Probe order is A `(x-128,z-128)`, B `(x-128,z)`,
    /// C `(x,z-128)`.
    pub probe_surface_y_raw: [i16; 3],
}

/// Direct `FUN_0041EC70(..., 0x100, selector, 1, dt)` input used by
/// Common-Dying.
///
/// Unlike [`Type9TerrainAttitudeInput`], this call does not run the outer
/// near/far gain selector or the effective-flag clamp.  Mode 1 reverses the
/// roll slope expression and, while body-up Y is positive, forces the roll
/// drive away from zero with magnitude at least `0x28` (zero becomes
/// `-0x28`).
#[derive(Debug, Clone, Copy)]
pub struct CommonDyingTerrainAttitudeInput<'a> {
    pub terrain: &'a TerrainGrid,
    pub position_raw: [i16; 3],
    pub pitch_raw: i16,
    pub roll_raw: i16,
    pub lateral_basis_q31: [i32; 3],
    pub forward_basis_q31: [i32; 3],
    pub body_up_y_q31: i32,
    pub state_flags: u32,
    pub resolved_surface_mode_raw: i8,
    pub water_enabled: bool,
    pub wave_tick_50hz: i32,
    pub effective_elapsed_micros: u32,
}

/// Exact angle-word result of Common-Dying's direct mode-1 attitude call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonDyingTerrainAttitudePlan {
    pub pitch_raw: i16,
    pub roll_raw: i16,
    pub pitch_drive_raw: i32,
    pub roll_drive_raw: i32,
    pub probe_surface_y_raw: [i16; 3],
}

/// Q31 body basis produced by the subsequent `FUN_00413F70` phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type9BodyBasis {
    pub lateral: [i32; 3],
    pub up: [i32; 3],
    pub forward: [i32; 3],
}

impl Type9BodyBasis {
    /// Rebuild the complete retail entity matrix from its three angle words.
    ///
    /// `FUN_00413F70` is the sole Euler-to-matrix writer recovered for this
    /// path. Callers must use this only at a proven `FUN_00413F70` boundary;
    /// callback consumers read the retained entity matrix instead of invoking
    /// this conversion on demand.
    pub fn from_angle_words(heading_raw: i16, pitch_raw: i16, roll_raw: i16) -> Self {
        let basis = HoverBasis::from_angle_words(heading_raw, pitch_raw, roll_raw);
        Self {
            lateral: basis.lateral,
            up: basis.up,
            forward: basis.forward,
        }
    }

    /// Convert the retained Q31 body axes into the renderer/collision matrix.
    ///
    /// Retail stores the three model-space axes as world-space columns. The
    /// port consumes row-major `world = M * model`, so this is deliberately a
    /// transpose rather than a fresh Euler reconstruction.
    pub fn orientation_world_from_model(self) -> [[f32; 3]; 3] {
        const Q31_SCALE: f32 = 2_147_483_648.0;
        let columns = [self.lateral, self.up, self.forward];
        std::array::from_fn(|world_axis| {
            std::array::from_fn(|model_axis| columns[model_axis][world_axis] as f32 / Q31_SCALE)
        })
    }
}

impl Type9TerrainAttitudePlan {
    /// Run the exact *subsequent* Euler-to-Q31 basis rebuild.
    ///
    /// Keeping this method separate preserves the retail callback boundary:
    /// `FUN_0040E640` changes angle words, then `FUN_00413F70` rebuilds all
    /// three axes before drag, snap, surface lifecycle, and master motion.
    pub fn rebuild_body_basis(self, heading_raw: i16) -> Type9BodyBasis {
        Type9BodyBasis::from_angle_words(heading_raw, self.pitch_raw, self.roll_raw)
    }
}

/// Plan exact terrain/surface attitude correction for one admitted callback.
///
/// The caller owns the effective-flag `0x10` admission gate. This function
/// preserves `FUN_0040E640`'s center-surface gain boundary, the four sampling
/// branches in `FUN_0041EC70`, Q31-to-Q12 axis narrowing, wrapping 32-bit
/// products, signed-word angle writes, and optional `0x10000` clamp.
pub fn plan_type9_terrain_attitude_raw(
    input: Type9TerrainAttitudeInput<'_>,
) -> Type9TerrainAttitudePlan {
    let x_raw = input.position_raw[0];
    let z_raw = input.position_raw[2];
    let terrain_y_raw = bilinear_terrain_height_raw(input.terrain, x_raw, z_raw);
    let center_surface_y_raw = terrain_or_wave_raw(
        input.terrain,
        x_raw,
        z_raw,
        terrain_y_raw,
        input.water_enabled,
        input.wave_tick_50hz,
    );
    let clearance_raw = i32::from(input.position_raw[1])
        .wrapping_sub(i32::from(input.active_model_extent_raw >> 1))
        .wrapping_sub(i32::from(center_surface_y_raw));
    let gain_raw = if clearance_raw <= NEAR_CLEARANCE_MAX_RAW {
        TERRAIN_ATTITUDE_NEAR_GAIN_RAW
    } else {
        TERRAIN_ATTITUDE_FAR_GAIN_RAW
    };

    let probes = sample_probe_surfaces_raw(SurfaceProbeInput {
        terrain: input.terrain,
        position_raw: input.position_raw,
        state_flags: input.state_flags,
        resolved_surface_mode_raw: input.resolved_surface_mode_raw,
        water_enabled: input.water_enabled,
        wave_tick_50hz: input.wave_tick_50hz,
    });
    let lateral_q12 = basis_axis_q12(input.lateral_basis_q31);
    let forward_q12 = basis_axis_q12(input.forward_basis_q31);
    let (pitch_drive_raw, roll_drive_raw) = slope_drives_raw(probes, lateral_q12, forward_q12);
    let elapsed_step = (input.effective_elapsed_micros >> 5) as i32;
    let pitch_delta = scale_attitude_drive_raw(pitch_drive_raw, elapsed_step, gain_raw);
    let roll_delta = scale_attitude_drive_raw(roll_drive_raw, elapsed_step, gain_raw);
    let mut pitch_raw = input.pitch_raw.wrapping_sub(pitch_delta);
    let mut roll_raw = input.roll_raw.wrapping_add(roll_delta);

    if input.effective_flags & TERRAIN_ATTITUDE_CLAMP_EFFECTIVE_FLAG != 0 {
        pitch_raw = pitch_raw.clamp(-ATTITUDE_LIMIT_RAW, ATTITUDE_LIMIT_RAW);
        roll_raw = roll_raw.clamp(-ATTITUDE_LIMIT_RAW, ATTITUDE_LIMIT_RAW);
    }

    Type9TerrainAttitudePlan {
        pitch_raw,
        roll_raw,
        gain_raw,
        center_surface_y_raw,
        probe_surface_y_raw: probes,
    }
}

/// Plan the exact fixed-gain, mode-1 terrain-attitude call made by
/// Common-Dying before its shared common mover.
pub fn plan_common_dying_terrain_attitude_raw(
    input: CommonDyingTerrainAttitudeInput<'_>,
) -> CommonDyingTerrainAttitudePlan {
    let probes = sample_probe_surfaces_raw(SurfaceProbeInput {
        terrain: input.terrain,
        position_raw: input.position_raw,
        state_flags: input.state_flags,
        resolved_surface_mode_raw: input.resolved_surface_mode_raw,
        water_enabled: input.water_enabled,
        wave_tick_50hz: input.wave_tick_50hz,
    });
    let lateral_q12 = basis_axis_q12(input.lateral_basis_q31);
    let forward_q12 = basis_axis_q12(input.forward_basis_q31);
    let (pitch_drive_raw, _) = slope_drives_raw(probes, lateral_q12, forward_q12);
    let mut roll_drive_raw = reverse_roll_drive_raw(probes, lateral_q12);
    if input.body_up_y_q31 > 0 {
        if roll_drive_raw < 1 {
            if -0x28 < roll_drive_raw {
                roll_drive_raw = -0x28;
            }
        } else if roll_drive_raw < 0x28 {
            roll_drive_raw = 0x28;
        }
    }

    let elapsed_step = (input.effective_elapsed_micros >> 5) as i32;
    let pitch_delta = scale_attitude_drive_raw(
        pitch_drive_raw,
        elapsed_step,
        TERRAIN_ATTITUDE_NEAR_GAIN_RAW,
    );
    let roll_delta =
        scale_attitude_drive_raw(roll_drive_raw, elapsed_step, TERRAIN_ATTITUDE_NEAR_GAIN_RAW);

    CommonDyingTerrainAttitudePlan {
        pitch_raw: input.pitch_raw.wrapping_sub(pitch_delta),
        roll_raw: input.roll_raw.wrapping_add(roll_delta),
        pitch_drive_raw,
        roll_drive_raw,
        probe_surface_y_raw: probes,
    }
}

#[derive(Clone, Copy)]
struct SurfaceProbeInput<'a> {
    terrain: &'a TerrainGrid,
    position_raw: [i16; 3],
    state_flags: u32,
    resolved_surface_mode_raw: i8,
    water_enabled: bool,
    wave_tick_50hz: i32,
}

fn sample_probe_surfaces_raw(input: SurfaceProbeInput<'_>) -> [i16; 3] {
    let x_minus_raw = input.position_raw[0].wrapping_sub(PROBE_OFFSET_RAW);
    let z_minus_raw = input.position_raw[2].wrapping_sub(PROBE_OFFSET_RAW);
    [
        sample_probe_surface_raw(input, x_minus_raw, z_minus_raw),
        sample_probe_surface_raw(input, x_minus_raw, input.position_raw[2]),
        sample_probe_surface_raw(input, input.position_raw[0], z_minus_raw),
    ]
}

fn sample_probe_surface_raw(input: SurfaceProbeInput<'_>, x_raw: i16, z_raw: i16) -> i16 {
    let use_bilinear = input.state_flags & TERRAIN_ATTITUDE_BILINEAR_STATE_BIT != 0;
    if input.resolved_surface_mode_raw == 0 {
        if use_bilinear {
            bilinear_terrain_height_raw(input.terrain, x_raw, z_raw)
        } else {
            coarse_terrain_height_raw(input.terrain, x_raw, z_raw)
        }
    } else if use_bilinear {
        let terrain_y_raw = bilinear_terrain_height_raw(input.terrain, x_raw, z_raw);
        terrain_or_wave_raw(
            input.terrain,
            x_raw,
            z_raw,
            terrain_y_raw,
            input.water_enabled,
            input.wave_tick_50hz,
        )
    } else {
        // Retail's coarse water branch compares directly with the flat sea
        // word; unlike FUN_00445920, this branch does not read the water gate.
        coarse_terrain_height_raw(input.terrain, x_raw, z_raw).max(input.terrain.sea_level_raw())
    }
}

fn terrain_or_wave_raw(
    terrain: &TerrainGrid,
    x_raw: i16,
    z_raw: i16,
    terrain_y_raw: i16,
    water_enabled: bool,
    wave_tick_50hz: i32,
) -> i16 {
    if water_enabled {
        wave_surface_raw(
            x_raw,
            z_raw,
            wave_tick_50hz,
            terrain.sea_level_raw(),
            terrain_y_raw,
        )
    } else {
        // 45920 returns the static sea word when DAT_FECE4 is zero; the
        // caller still takes the greater of that and its terrain sample.
        terrain_y_raw.max(terrain.sea_level_raw())
    }
}

fn coarse_terrain_height_raw(terrain: &TerrainGrid, x_raw: i16, z_raw: i16) -> i16 {
    let x = usize::from((x_raw as u16) >> 8);
    let z = usize::from((z_raw as u16) >> 8);
    i16::from(terrain.cell(x, z).expect("complete 256x256 terrain").height as i8) << 5
}

fn basis_axis_q12(axis_q31: [i32; 3]) -> [i16; 3] {
    axis_q31.map(|component| (component >> 19) as i16)
}

fn slope_drives_raw(
    [a_raw, b_raw, c_raw]: [i16; 3],
    lateral_q12: [i16; 3],
    forward_q12: [i16; 3],
) -> (i32, i32) {
    let b_minus_a = i32::from(b_raw).wrapping_sub(i32::from(a_raw));
    let c_minus_b = i32::from(c_raw).wrapping_sub(i32::from(b_raw));
    let pitch_drive_raw = c_minus_b
        .wrapping_mul(i32::from(forward_q12[0]))
        .wrapping_add(b_minus_a.wrapping_mul(i32::from(forward_q12[2])))
        .wrapping_sub(i32::from(forward_q12[1]).wrapping_mul(0x100))
        >> 12;
    let roll_drive_raw = b_minus_a
        .wrapping_mul(i32::from(lateral_q12[2]))
        .wrapping_add(c_minus_b.wrapping_mul(i32::from(lateral_q12[0])))
        .wrapping_sub(i32::from(lateral_q12[1]).wrapping_mul(0x100))
        >> 12;
    (pitch_drive_raw, roll_drive_raw)
}

fn reverse_roll_drive_raw([a_raw, b_raw, c_raw]: [i16; 3], lateral_q12: [i16; 3]) -> i32 {
    let a_minus_b = i32::from(a_raw).wrapping_sub(i32::from(b_raw));
    let b_minus_c = i32::from(b_raw).wrapping_sub(i32::from(c_raw));
    a_minus_b
        .wrapping_mul(i32::from(lateral_q12[2]))
        .wrapping_add(b_minus_c.wrapping_mul(i32::from(lateral_q12[0])))
        .wrapping_add(i32::from(lateral_q12[1]).wrapping_mul(0x100))
        >> 12
}

fn scale_attitude_drive_raw(drive_raw: i32, elapsed_step: i32, gain_raw: i32) -> i16 {
    (drive_raw.wrapping_mul(elapsed_step).wrapping_mul(gain_raw) >> 15) as i16
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

    fn flat_terrain(height: i8, sea_y_raw: i16) -> TerrainGrid {
        let mut header = [0; 5];
        header[0] = i32::from(sea_y_raw) << 8;
        TerrainGrid {
            header,
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

    fn set_height(terrain: &mut TerrainGrid, x: usize, z: usize, height: i8) {
        terrain.cells[x * GRID_SIZE + z].height = height as u8;
    }

    fn base_input(terrain: &TerrainGrid) -> Type9TerrainAttitudeInput<'_> {
        let basis = HoverBasis::from_angle_words(0, 0, 0);
        Type9TerrainAttitudeInput {
            terrain,
            position_raw: [0, 0, 0],
            pitch_raw: 0,
            roll_raw: 0,
            lateral_basis_q31: basis.lateral,
            forward_basis_q31: basis.forward,
            state_flags: 0,
            effective_flags: TERRAIN_ATTITUDE_EFFECTIVE_FLAG,
            active_model_extent_raw: 0,
            resolved_surface_mode_raw: 0,
            water_enabled: false,
            wave_tick_50hz: 0,
            effective_elapsed_micros: 0,
        }
    }

    #[test]
    fn retained_body_basis_transposes_q31_columns_for_model_consumers() {
        let basis = Type9BodyBasis {
            lateral: [0x4000_0000, -0x2000_0000, 0x1000_0000],
            up: [-0x0800_0000, 0x0400_0000, -0x0200_0000],
            forward: [0x0100_0000, -0x0080_0000, 0x0040_0000],
        };

        assert_eq!(
            basis.orientation_world_from_model(),
            [
                [0.5, -0.0625, 0.0078125],
                [-0.25, 0.03125, -0.00390625],
                [0.125, -0.015625, 0.001953125],
            ]
        );
    }

    fn common_dying_input(terrain: &TerrainGrid) -> CommonDyingTerrainAttitudeInput<'_> {
        let basis = HoverBasis::from_angle_words(0, 0, 0);
        CommonDyingTerrainAttitudeInput {
            terrain,
            position_raw: [0, 0, 0],
            pitch_raw: 0,
            roll_raw: 0,
            lateral_basis_q31: basis.lateral,
            forward_basis_q31: basis.forward,
            body_up_y_q31: basis.up[1],
            state_flags: 0,
            resolved_surface_mode_raw: 0,
            water_enabled: false,
            wave_tick_50hz: 0,
            effective_elapsed_micros: 20_000,
        }
    }

    fn sloped_terrain() -> TerrainGrid {
        let mut terrain = flat_terrain(0, -4096);
        set_height(&mut terrain, 1, 1, 0);
        set_height(&mut terrain, 1, 2, 10);
        set_height(&mut terrain, 2, 1, 20);
        set_height(&mut terrain, 2, 2, 30);
        terrain
    }

    #[test]
    fn gain_boundary_treats_clearance_100_as_near_and_101_as_far() {
        let terrain = flat_terrain(0, -4096);
        let mut input = base_input(&terrain);
        input.active_model_extent_raw = 0x200;
        input.position_raw[1] = 0x200;
        assert_eq!(
            plan_type9_terrain_attitude_raw(input).gain_raw,
            TERRAIN_ATTITUDE_NEAR_GAIN_RAW
        );

        input.position_raw[1] = 0x201;
        assert_eq!(
            plan_type9_terrain_attitude_raw(input).gain_raw,
            TERRAIN_ATTITUDE_FAR_GAIN_RAW
        );
    }

    #[test]
    fn probe_modes_preserve_coarse_bilinear_flat_sea_and_wave_branches() {
        let terrain = sloped_terrain();
        let mut input = base_input(&terrain);
        input.position_raw = [0x0180, 0, 0x0180];
        assert_eq!(
            plan_type9_terrain_attitude_raw(input).probe_surface_y_raw,
            [0, 0, 0]
        );

        input.state_flags = TERRAIN_ATTITUDE_BILINEAR_STATE_BIT;
        assert_eq!(
            plan_type9_terrain_attitude_raw(input).probe_surface_y_raw,
            [0, 160, 320]
        );

        let mut water_terrain = sloped_terrain();
        water_terrain.header[0] = 1000 << 8;
        let mut water_input = base_input(&water_terrain);
        water_input.position_raw = [0x0180, 0, 0x0180];
        water_input.state_flags = 0;
        water_input.resolved_surface_mode_raw = -1;
        assert_eq!(
            plan_type9_terrain_attitude_raw(water_input).probe_surface_y_raw,
            [1000, 1000, 1000]
        );

        water_input.state_flags = TERRAIN_ATTITUDE_BILINEAR_STATE_BIT;
        water_input.water_enabled = false;
        assert_eq!(
            plan_type9_terrain_attitude_raw(water_input).probe_surface_y_raw,
            [1000, 1000, 1000]
        );

        water_input.water_enabled = true;
        water_input.wave_tick_50hz = 17;
        let actual = plan_type9_terrain_attitude_raw(water_input).probe_surface_y_raw;
        let expected = [
            wave_surface_raw(0x0100, 0x0100, 17, 1000, 0),
            wave_surface_raw(0x0100, 0x0180, 17, 1000, 160),
            wave_surface_raw(0x0180, 0x0100, 17, 1000, 320),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn slope_drive_uses_retail_probe_order_axis_narrowing_and_gain() {
        let terrain = sloped_terrain();
        let mut input = base_input(&terrain);
        input.position_raw = [0x0180, 0, 0x0180];
        input.state_flags = TERRAIN_ATTITUDE_BILINEAR_STATE_BIT;
        input.lateral_basis_q31 = [0, 0, i32::MIN];
        input.forward_basis_q31 = [i32::MAX, 0, 0];
        input.effective_elapsed_micros = 32_768;
        let plan = plan_type9_terrain_attitude_raw(input);

        assert_eq!(plan.gain_raw, TERRAIN_ATTITUDE_NEAR_GAIN_RAW);
        assert_eq!(plan.pitch_raw, -1272);
        assert_eq!(plan.roll_raw, -1280);
    }

    #[test]
    fn clamp_is_owned_only_by_effective_flag_10000() {
        let terrain = flat_terrain(0, -4096);
        let mut input = base_input(&terrain);
        input.pitch_raw = 10_000;
        input.roll_raw = -10_000;
        let unclamped = plan_type9_terrain_attitude_raw(input);
        assert_eq!(unclamped.pitch_raw, 10_000);
        assert_eq!(unclamped.roll_raw, -10_000);

        input.effective_flags |= TERRAIN_ATTITUDE_CLAMP_EFFECTIVE_FLAG;
        let clamped = plan_type9_terrain_attitude_raw(input);
        assert_eq!(clamped.pitch_raw, ATTITUDE_LIMIT_RAW);
        assert_eq!(clamped.roll_raw, -ATTITUDE_LIMIT_RAW);
    }

    #[test]
    fn common_dying_mode_one_forces_flat_upright_roll_away_from_zero() {
        let terrain = flat_terrain(0, -4096);
        let input = common_dying_input(&terrain);
        let plan = plan_common_dying_terrain_attitude_raw(input);

        assert_eq!(plan.pitch_drive_raw, 0);
        assert_eq!(plan.roll_drive_raw, -0x28);
        assert_eq!(plan.pitch_raw, 0);
        assert_eq!(plan.roll_raw, -196);

        let mut upside_down = input;
        upside_down.body_up_y_q31 = -1;
        let plan = plan_common_dying_terrain_attitude_raw(upside_down);
        assert_eq!(plan.roll_drive_raw, 0);
        assert_eq!(plan.roll_raw, 0);
    }

    #[test]
    fn common_dying_reverses_only_the_roll_slope_expression() {
        let terrain = sloped_terrain();
        let mut ordinary = base_input(&terrain);
        ordinary.position_raw = [0x0180, 0, 0x0180];
        ordinary.state_flags = TERRAIN_ATTITUDE_BILINEAR_STATE_BIT;
        ordinary.lateral_basis_q31 = [0, 0, i32::MIN];
        ordinary.forward_basis_q31 = [i32::MAX, 0, 0];
        ordinary.effective_elapsed_micros = 32_768;
        let ordinary_plan = plan_type9_terrain_attitude_raw(ordinary);

        let common = CommonDyingTerrainAttitudeInput {
            terrain: ordinary.terrain,
            position_raw: ordinary.position_raw,
            pitch_raw: ordinary.pitch_raw,
            roll_raw: ordinary.roll_raw,
            lateral_basis_q31: ordinary.lateral_basis_q31,
            forward_basis_q31: ordinary.forward_basis_q31,
            body_up_y_q31: 0,
            state_flags: ordinary.state_flags,
            resolved_surface_mode_raw: ordinary.resolved_surface_mode_raw,
            water_enabled: ordinary.water_enabled,
            wave_tick_50hz: ordinary.wave_tick_50hz,
            effective_elapsed_micros: ordinary.effective_elapsed_micros,
        };
        let common_plan = plan_common_dying_terrain_attitude_raw(common);

        assert_eq!(common_plan.pitch_raw, ordinary_plan.pitch_raw);
        assert_eq!(common_plan.roll_raw, ordinary_plan.roll_raw.wrapping_neg());
    }

    #[test]
    fn basis_rebuild_uses_corrected_angles_in_the_following_phase() {
        let terrain = sloped_terrain();
        let mut input = base_input(&terrain);
        input.position_raw = [0x0180, 0, 0x0180];
        input.state_flags = TERRAIN_ATTITUDE_BILINEAR_STATE_BIT;
        input.lateral_basis_q31 = [0, 0, i32::MIN];
        input.forward_basis_q31 = [i32::MAX, 0, 0];
        input.effective_elapsed_micros = 32_768;
        let heading_raw = 0x1234;

        let plan = plan_type9_terrain_attitude_raw(input);
        let rebuilt = plan.rebuild_body_basis(heading_raw);
        let expected = HoverBasis::from_angle_words(heading_raw, plan.pitch_raw, plan.roll_raw);
        let stale = HoverBasis::from_angle_words(heading_raw, input.pitch_raw, input.roll_raw);

        assert_eq!(rebuilt.lateral, expected.lateral);
        assert_eq!(rebuilt.up, expected.up);
        assert_eq!(rebuilt.forward, expected.forward);
        assert_ne!(rebuilt.forward, stale.forward);
    }
}
