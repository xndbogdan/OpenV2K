//! Retail Targetter controller program (`FUN_0044CEB0` / `FUN_0044D010` /
//! `FUN_0044D0F0`).
//!
//! Selector `0x3C` installs a 0x54-byte program at player-controller `+0x22C`.
//! The program first rebuilds an ordered candidate list from the intrusive
//! live-entity list, then probes the selected weapon trajectory against each
//! candidate's active Section-8 collision program. This module retains that
//! state and arithmetic without coupling it to a renderer or resource cache;
//! the gameplay adapter supplies authored collision and terrain probes.

use crate::entity_collision_state::{RetailRuntimeValue, RetailStateWord};
use v2k_formats::terrain::TerrainGrid;

pub const TARGETTER_SCAN_RADIUS_RAW: i32 = 0x0A00;
pub const TARGETTER_NEAR_RAY_LIMIT_RAW: i32 = 0x0300;
pub const TARGETTER_MODEL_PROBE_RADIUS_RAW: u16 = 0x0078;
pub const TARGETTER_PRIOR_RETENTION_US: u32 = 1_000_000;
pub const TARGETTER_TERRAIN_DISTANCE_MAX_RAW: i32 = 10_000;
pub const TARGETTER_TERRAIN_STEP_MAX_RAW: i32 = 400;
pub const TARGETTER_TERRAIN_STEP_MIN_RAW: i32 = 8;
pub const TARGETTER_TERRAIN_CONVERGED_BELOW_RAW: i32 = 20;
pub const TARGETTER_BALLISTIC_GRAVITY_RAW: i32 = 0x0018_0000;
/// Unclassified constants restored by `FUN_0044CFE0` at controller `+0x268`
/// and `+0x26C`. They remain named evidence until a consumer is recovered.
pub const TARGETTER_INIT_FIELD_3C_RAW: u32 = 0x002D_C6C0;
pub const TARGETTER_INIT_FIELD_40_RAW: u32 = 0x0000_028F;

pub const TARGETTER_RESULT_TERRAIN: u8 = 1;
pub const TARGETTER_RESULT_EXACT_ENTITY: u8 = 2;
pub const TARGETTER_RESULT_NEAR_ENTITY: u8 = 4;

pub const TARGETTER_RING_EXACT_MODEL_ID: usize = 235;
pub const TARGETTER_TERRAIN_CROSSHAIR_MODEL_ID: usize = 236;
pub const TARGETTER_EXACT_CROSSHAIR_MODEL_ID: usize = 237;
pub const TARGETTER_NEAR_CROSSHAIR_MODEL_ID: usize = 238;
pub const TARGETTER_RING_NEAR_MODEL_ID: usize = 239;

const TARGETTER_DYING_STATE_BIT: u32 = 0x0000_4000;
const TARGETTER_REQUIRED_STATE_BITS: u32 = 0x4000_8000;
const TARGETTER_REQUIRED_CAPABILITY_BITS: u32 = 0x0000_0009;
const TARGETTER_EXCLUDED_CAPABILITY_BIT: u32 = 0x0000_0010;
const TARGETTER_EXCLUDED_ENTITY_TYPE: u32 = 0x6E;
const TARGETTER_PARAMETER_SCALE: i32 = 1_000;

/// `FUN_0044EA60`'s two trajectory families. Unknown projectile callback ids
/// remain explicit so a future weapon cannot silently acquire straight-ray
/// targeting merely because its descriptor is otherwise usable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetterTrajectory {
    Straight,
    Ballistic,
    Unsupported(u32),
}

/// `FUN_0044EA60`: weapon-component `+0x10` is the selected descriptor dword
/// copied by `FUN_00444FA0`. Return `0` is straight; any other value is
/// ballistic. The switch has no failure path.
pub const fn fun_0044ea60(selector: u32) -> TargetterTrajectory {
    match selector {
        5 | 0x0c | 0x0d | 0x0e | 0x0f | 0x11 | 0x15 | 0x1f => TargetterTrajectory::Straight,
        _ => TargetterTrajectory::Ballistic,
    }
}

/// Complete raw trajectory returned by the selected runtime weapon component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetterRay {
    pub origin_raw: [i16; 3],
    /// Signed raw velocity components. Retail parameterizes these in thousandths.
    pub velocity_raw: [i32; 3],
    pub trajectory: TargetterTrajectory,
}

/// Candidate fields read by `FUN_0044D010`, in live-list order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetterCandidate {
    pub id: u32,
    pub entity_type: u32,
    pub position_raw: [i16; 3],
    pub state_flags: RetailStateWord,
    pub capability_flags: u32,
}

/// Result of the caller-owned Section-8 radius-0x78 model probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetterModelProbe {
    Hit,
    Miss,
    /// The active model or collision program could not be evaluated exactly.
    Unresolved,
}

#[derive(Debug, Clone, Copy)]
pub struct TargetterUpdateRequest<'a> {
    pub elapsed_micros: u32,
    pub ray: TargetterRay,
    pub candidates: &'a [TargetterCandidate],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetterUpdateStatus {
    Inactive,
    Updated,
    UnsupportedTrajectory(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetterCrosshairKind {
    Terrain,
    ExactEntity,
    NearEntity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetterCrosshair {
    pub model_id: usize,
    pub position_raw: [i16; 3],
    pub kind: TargetterCrosshairKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetterRing {
    pub model_id: usize,
    pub target_id: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TargetterOverlay {
    pub ring: Option<TargetterRing>,
    pub crosshair: Option<TargetterCrosshair>,
}

/// Typed representation of controller `+0x22C..+0x27F`'s active state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetterRuntime {
    owner_id: u32,
    active: bool,
    prior_target_id: Option<u32>,
    current_target_id: Option<u32>,
    since_target_change_us: u32,
    time_on_current_target_us: u32,
    entity_ray_parameter_raw: i32,
    entity_point_raw: [i16; 3],
    terrain_ray_distance_raw: i32,
    terrain_step_raw: i32,
    terrain_point_raw: [i16; 3],
    result_bits: u8,
    ring_spin_raw: u32,
}

impl TargetterRuntime {
    /// Construct the disabled table installed after `FUN_0044CEB0` returns.
    /// The constructor briefly runs active init, then restores the authored
    /// distance/step seed before selecting table `0x004F7170`.
    pub const fn new(owner_id: u32) -> Self {
        Self {
            owner_id,
            active: false,
            prior_target_id: None,
            current_target_id: None,
            since_target_change_us: 0,
            time_on_current_target_us: 0,
            entity_ray_parameter_raw: 0,
            entity_point_raw: [0; 3],
            terrain_ray_distance_raw: 1_000,
            terrain_step_raw: 50,
            terrain_point_raw: [0; 3],
            result_bits: 0,
            ring_spin_raw: 0,
        }
    }

    pub const fn owner_id(&self) -> u32 {
        self.owner_id
    }

    pub const fn is_active(&self) -> bool {
        self.active
    }

    pub const fn current_target_id(&self) -> Option<u32> {
        self.current_target_id
    }

    pub const fn prior_target_id(&self) -> Option<u32> {
        self.prior_target_id
    }

    pub const fn result_bits(&self) -> u8 {
        self.result_bits
    }

    pub const fn terrain_ray_distance_raw(&self) -> i32 {
        self.terrain_ray_distance_raw
    }

    pub const fn terrain_step_raw(&self) -> i32 {
        self.terrain_step_raw
    }

    pub const fn ring_spin_raw(&self) -> u32 {
        self.ring_spin_raw
    }

    /// Install table `0x004D0128`. Its init callback clears +0x04..+0x37;
    /// owner, callback/list storage and immutable constants survive.
    pub fn install(&mut self) {
        self.active = true;
        self.prior_target_id = None;
        self.current_target_id = None;
        self.since_target_change_us = 0;
        self.time_on_current_target_us = 0;
        self.entity_ray_parameter_raw = 0;
        self.entity_point_raw = [0; 3];
        self.terrain_ray_distance_raw = 0;
        self.terrain_step_raw = 0;
        self.terrain_point_raw = [0; 3];
        self.result_bits = 0;
    }

    /// Recreate a world-local controller while retaining the session-owned
    /// acquired program state.
    pub fn rebind_owner(&mut self, owner_id: u32) {
        let active = self.active;
        *self = Self::new(owner_id);
        if active {
            self.install();
        }
    }

    pub fn update<ModelProbe, TerrainHeight>(
        &mut self,
        request: TargetterUpdateRequest<'_>,
        mut model_probe: ModelProbe,
        mut terrain_height_raw: TerrainHeight,
    ) -> TargetterUpdateStatus
    where
        ModelProbe: FnMut(&TargetterCandidate, [i16; 3], u16) -> TargetterModelProbe,
        TerrainHeight: FnMut(i16, i16) -> i16,
    {
        if !self.active {
            return TargetterUpdateStatus::Inactive;
        }
        if let TargetterTrajectory::Unsupported(callback) = request.ray.trajectory {
            self.result_bits = 0;
            self.transition_target(None, request.elapsed_micros);
            return TargetterUpdateStatus::UnsupportedTrajectory(callback);
        }

        self.result_bits = 0;
        let mut nearest_exact_parameter = i32::MAX;
        let mut nearest_score = i32::MAX;
        let mut selected_target = None;

        for candidate in request.candidates.iter().filter(|candidate| {
            candidate_is_eligible(self.owner_id, request.ray.origin_raw, candidate)
        }) {
            let Some(parameter) = candidate_ray_parameter(request.ray, candidate.position_raw)
            else {
                continue;
            };
            if parameter <= 0 || parameter >= nearest_exact_parameter {
                continue;
            }
            let predicted = ray_point_at(request.ray, parameter);
            match model_probe(candidate, predicted, TARGETTER_MODEL_PROBE_RADIUS_RAW) {
                TargetterModelProbe::Hit => {
                    self.result_bits = TARGETTER_RESULT_EXACT_ENTITY;
                    self.entity_ray_parameter_raw = parameter;
                    self.entity_point_raw = predicted;
                    selected_target = Some(candidate.id);
                    nearest_exact_parameter = parameter;
                }
                TargetterModelProbe::Miss => {
                    if self.result_bits & TARGETTER_RESULT_EXACT_ENTITY != 0 {
                        continue;
                    }
                    let distance =
                        approximate_wrapping_distance_raw(candidate.position_raw, predicted);
                    if distance >= TARGETTER_NEAR_RAY_LIMIT_RAW {
                        continue;
                    }
                    let score = if self.current_target_id == Some(candidate.id) {
                        distance.saturating_mul(3)
                    } else {
                        distance.saturating_mul(4)
                    };
                    if score < nearest_score {
                        self.result_bits = TARGETTER_RESULT_NEAR_ENTITY;
                        self.entity_ray_parameter_raw = parameter;
                        self.entity_point_raw = predicted;
                        selected_target = Some(candidate.id);
                        nearest_score = score;
                    }
                }
                // A failed authored-program evaluation is not evidence of a
                // miss; admitting it to the coarse near path would invent a
                // target retail may have rejected as an exact intersection.
                TargetterModelProbe::Unresolved => {}
            }
        }

        self.transition_target(selected_target, request.elapsed_micros);
        self.advance_terrain_convergence(request.ray, &mut terrain_height_raw);
        TargetterUpdateStatus::Updated
    }

    /// `0044DB10` increments its process-global packed spin only when the ring
    /// draw path actually runs. Call this after resolving a live ring target.
    pub fn advance_ring_spin(&mut self, elapsed_micros: u32) {
        self.ring_spin_raw = self.ring_spin_raw.wrapping_add(elapsed_micros >> 5);
    }

    /// Build `FUN_0044DB10`'s draw requests. Retail's config word at
    /// `DAT_004D0148` guards only the entity-ring block at `0044DB93`; the
    /// result-bit crosshair block beginning at `0044DCA2` runs regardless.
    pub fn overlay(&self, show_entity_ring: bool) -> TargetterOverlay {
        if !self.active || self.result_bits == 0 {
            return TargetterOverlay::default();
        }

        let display_target = if self.time_on_current_target_us != 0 {
            self.current_target_id
        } else if self.since_target_change_us < TARGETTER_PRIOR_RETENTION_US {
            self.prior_target_id
        } else {
            None
        };
        let ring = if show_entity_ring {
            display_target.map(|target_id| TargetterRing {
                // Assembly at 0044DBB5 selects pool offset 0x3AC (235) for bit 2,
                // otherwise 0x3BC (239). The names alone suggest the reverse and
                // must not override the executable branch.
                model_id: if self.result_bits & TARGETTER_RESULT_EXACT_ENTITY != 0 {
                    TARGETTER_RING_EXACT_MODEL_ID
                } else {
                    TARGETTER_RING_NEAR_MODEL_ID
                },
                target_id,
            })
        } else {
            None
        };

        let crosshair = if self.result_bits
            & (TARGETTER_RESULT_EXACT_ENTITY | TARGETTER_RESULT_NEAR_ENTITY)
            != 0
        {
            let near = self.result_bits & TARGETTER_RESULT_NEAR_ENTITY != 0;
            Some(TargetterCrosshair {
                model_id: if near {
                    TARGETTER_NEAR_CROSSHAIR_MODEL_ID
                } else {
                    TARGETTER_EXACT_CROSSHAIR_MODEL_ID
                },
                position_raw: self.entity_point_raw,
                kind: if near {
                    TargetterCrosshairKind::NearEntity
                } else {
                    TargetterCrosshairKind::ExactEntity
                },
            })
        } else if self.result_bits & TARGETTER_RESULT_TERRAIN != 0 {
            Some(TargetterCrosshair {
                model_id: TARGETTER_TERRAIN_CROSSHAIR_MODEL_ID,
                position_raw: self.terrain_point_raw,
                kind: TargetterCrosshairKind::Terrain,
            })
        } else {
            None
        };

        TargetterOverlay { ring, crosshair }
    }

    fn transition_target(&mut self, selected: Option<u32>, elapsed_micros: u32) {
        match selected {
            None => {
                if self.current_target_id.is_some() {
                    self.since_target_change_us = 0;
                    self.time_on_current_target_us = 0;
                    self.prior_target_id = self.current_target_id;
                    self.current_target_id = None;
                }
            }
            Some(target) => {
                if self.current_target_id != Some(target) {
                    self.time_on_current_target_us = 0;
                    self.since_target_change_us = 0;
                    self.prior_target_id = self.current_target_id;
                    self.current_target_id = Some(target);
                }
                self.time_on_current_target_us =
                    self.time_on_current_target_us.wrapping_add(elapsed_micros);
            }
        }
        self.since_target_change_us = self.since_target_change_us.wrapping_add(elapsed_micros);
    }

    fn advance_terrain_convergence<TerrainHeight>(
        &mut self,
        ray: TargetterRay,
        terrain_height_raw: &mut TerrainHeight,
    ) where
        TerrainHeight: FnMut(i16, i16) -> i16,
    {
        let mut distance = self.terrain_ray_distance_raw;
        let mut step = self.terrain_step_raw;
        let mut point = ray_point_at(ray, distance);
        let mut above = terrain_height_raw(point[0], point[2]) <= point[1];

        for _ in 0..4 {
            let signed_step = if above { step } else { -step };
            distance = distance.wrapping_add(signed_step);
            if distance < 1 {
                distance = 0;
                step = TARGETTER_TERRAIN_CONVERGED_BELOW_RAW;
                continue;
            }
            if distance > TARGETTER_TERRAIN_DISTANCE_MAX_RAW {
                distance = TARGETTER_TERRAIN_DISTANCE_MAX_RAW;
            }

            point = ray_point_at(ray, distance);
            let next_above = terrain_height_raw(point[0], point[2]) <= point[1];
            if next_above == above {
                step = step.wrapping_mul(2).min(TARGETTER_TERRAIN_STEP_MAX_RAW);
            } else {
                if above {
                    distance = distance.wrapping_sub(step);
                }
                step >>= 1;
                if step < TARGETTER_TERRAIN_STEP_MIN_RAW {
                    step = TARGETTER_TERRAIN_STEP_MIN_RAW;
                    break;
                }
                // Retail keeps the old side after backing out an above→below
                // crossing and adopts the new side for the opposite crossing.
                if !above {
                    above = next_above;
                }
            }
        }

        self.terrain_ray_distance_raw = distance;
        self.terrain_step_raw = step;
        if step < TARGETTER_TERRAIN_CONVERGED_BELOW_RAW {
            self.result_bits |= TARGETTER_RESULT_TERRAIN;
            self.terrain_point_raw = point;
        }
    }
}

pub fn targetter_model_scale_raw(collision_radius_raw: u16) -> u16 {
    u32::from(collision_radius_raw)
        .saturating_add(0x20)
        .saturating_mul(0x80)
        .min(u32::from(u16::MAX)) as u16
}

/// Exact signed-word bilinear terrain sampler used by the retail controller.
///
/// Coordinates wrap as unsigned 16-bit world words. Section-10 heights are
/// signed bytes in raw 1/32-world-unit steps; both interpolation stages use
/// arithmetic right shifts, matching `FUN_0044D010` instead of the renderer's
/// floating-point terrain helper.
pub fn targetter_terrain_height_raw(terrain: &TerrainGrid, x_raw: i16, z_raw: i16) -> i16 {
    let x_word = x_raw as u16;
    let z_word = z_raw as u16;
    let x0 = usize::from(x_word >> 8);
    let z0 = usize::from(z_word >> 8);
    let x1 = (x0 + 1) & 0xFF;
    let z1 = (z0 + 1) & 0xFF;
    let x_fraction = i32::from(x_word & 0xFF);
    let z_fraction = i32::from(z_word & 0xFF);
    let height = |x: usize, z: usize| {
        i32::from(terrain.cell(x, z).expect("complete 256x256 terrain").height as i8) << 5
    };

    let h00 = height(x0, z0);
    let h10 = height(x1, z0);
    let h01 = height(x0, z1);
    let h11 = height(x1, z1);
    let along_x0 = (((h10 - h00) * x_fraction) >> 8) + h00;
    let along_x1 = (((h11 - h01) * x_fraction) >> 8) + h01;
    ((((along_x1 - along_x0) * z_fraction) >> 8) + along_x0) as i16
}

fn candidate_is_eligible(
    owner_id: u32,
    owner_position_raw: [i16; 3],
    candidate: &TargetterCandidate,
) -> bool {
    if candidate.id == owner_id
        || candidate.entity_type == TARGETTER_EXCLUDED_ENTITY_TYPE
        || candidate.capability_flags & TARGETTER_REQUIRED_CAPABILITY_BITS == 0
        || candidate.capability_flags & TARGETTER_EXCLUDED_CAPABILITY_BIT != 0
    {
        return false;
    }
    if candidate.state_flags.masked(TARGETTER_DYING_STATE_BIT) != RetailRuntimeValue::Known(0)
        || candidate.state_flags.known_value_bits() & TARGETTER_REQUIRED_STATE_BITS == 0
    {
        return false;
    }
    within_scan_cube(owner_position_raw, candidate.position_raw)
}

fn within_scan_cube(owner_raw: [i16; 3], candidate_raw: [i16; 3]) -> bool {
    owner_raw
        .into_iter()
        .zip(candidate_raw)
        .all(|(owner, candidate)| {
            i32::from(owner.wrapping_sub(candidate)).abs() < TARGETTER_SCAN_RADIUS_RAW
        })
}

fn candidate_ray_parameter(ray: TargetterRay, candidate_raw: [i16; 3]) -> Option<i32> {
    let dx = i32::from(candidate_raw[0].wrapping_sub(ray.origin_raw[0]));
    let dz = i32::from(candidate_raw[2].wrapping_sub(ray.origin_raw[2]));
    let denominator = if dz.abs() < dx.abs() {
        ray.velocity_raw[0]
    } else {
        ray.velocity_raw[2]
    };
    (denominator != 0).then(|| dx_or_dz(dx, dz) * TARGETTER_PARAMETER_SCALE / denominator)
}

fn dx_or_dz(dx: i32, dz: i32) -> i32 {
    if dz.abs() < dx.abs() {
        dx
    } else {
        dz
    }
}

fn ray_point_at(ray: TargetterRay, parameter: i32) -> [i16; 3] {
    let mut point = std::array::from_fn(|axis| {
        let displacement =
            parameter.wrapping_mul(ray.velocity_raw[axis]) / TARGETTER_PARAMETER_SCALE;
        ray.origin_raw[axis].wrapping_add(displacement as i16)
    });
    if ray.trajectory == TargetterTrajectory::Ballistic {
        let parameter_squared = parameter.wrapping_mul(parameter);
        let drop =
            (i64::from(parameter_squared) * i64::from(TARGETTER_BALLISTIC_GRAVITY_RAW)) >> 31;
        point[1] = point[1].wrapping_sub(drop as i16);
    }
    point
}

fn approximate_wrapping_distance_raw(first: [i16; 3], second: [i16; 3]) -> i32 {
    let mut components: [i32; 3] =
        std::array::from_fn(|axis| i32::from(first[axis].wrapping_sub(second[axis])).abs());
    components.sort_unstable();
    components[2] + ((components[0] + components[1]) >> 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

    fn candidate(id: u32, position_raw: [i16; 3]) -> TargetterCandidate {
        TargetterCandidate {
            id,
            entity_type: 17,
            position_raw,
            state_flags: RetailStateWord::exact(0x8000),
            capability_flags: 1,
        }
    }

    fn straight_ray() -> TargetterRay {
        TargetterRay {
            origin_raw: [0; 3],
            velocity_raw: [0, 0, 1_000],
            trajectory: TargetterTrajectory::Straight,
        }
    }

    #[test]
    fn fun_0044ea60_classifies_selector_not_a_separate_method_id() {
        assert_eq!(fun_0044ea60(1), TargetterTrajectory::Ballistic);
        assert_eq!(fun_0044ea60(2), TargetterTrajectory::Ballistic);
        assert_eq!(fun_0044ea60(3), TargetterTrajectory::Ballistic);
        assert_eq!(fun_0044ea60(0x12), TargetterTrajectory::Ballistic);
        assert_eq!(fun_0044ea60(30), TargetterTrajectory::Ballistic);
        for selector in [5_u32, 0x0c, 0x0d, 0x0e, 0x0f, 0x11, 0x15, 0x1f] {
            assert_eq!(
                fun_0044ea60(selector),
                TargetterTrajectory::Straight,
                "selector {selector:#x}"
            );
        }
    }

    #[test]
    fn constructor_and_install_match_controller_program_boundaries() {
        let mut runtime = TargetterRuntime::new(7);
        assert!(!runtime.is_active());
        assert_eq!(runtime.terrain_ray_distance_raw(), 1_000);
        assert_eq!(runtime.terrain_step_raw(), 50);
        assert_eq!(TARGETTER_INIT_FIELD_3C_RAW, 0x002D_C6C0);
        assert_eq!(TARGETTER_INIT_FIELD_40_RAW, 0x028F);

        runtime.install();
        assert!(runtime.is_active());
        assert_eq!(runtime.owner_id(), 7);
        assert_eq!(runtime.terrain_ray_distance_raw(), 0);
        assert_eq!(runtime.terrain_step_raw(), 0);
        assert_eq!(runtime.result_bits(), 0);
    }

    #[test]
    fn first_active_update_bootstraps_the_zeroed_terrain_step() {
        let mut runtime = TargetterRuntime::new(1);
        runtime.install();
        runtime.update(
            TargetterUpdateRequest {
                elapsed_micros: 20_000,
                ray: TargetterRay {
                    origin_raw: [0, 100, 0],
                    velocity_raw: [0, 0, 1_000],
                    trajectory: TargetterTrajectory::Straight,
                },
                candidates: &[],
            },
            |_, _, _| TargetterModelProbe::Miss,
            |_, _| -1_000,
        );
        // 0x44D0F0 turns the first distance<1 iteration into step 0x14,
        // then spends the other three loop slots growing 20→40→80→160.
        assert_eq!(runtime.terrain_ray_distance_raw(), 140);
        assert_eq!(runtime.terrain_step_raw(), 160);
        assert_eq!(runtime.result_bits() & TARGETTER_RESULT_TERRAIN, 0);
    }

    #[test]
    fn candidate_gate_preserves_strict_cube_wrap_flags_and_order() {
        let owner = [i16::MAX - 5, 0, 0];
        let mut candidates = vec![
            candidate(2, [i16::MIN + 4, 0, 100]),
            candidate(3, [owner[0].wrapping_add(0x09ff), 0, 200]),
            candidate(4, [owner[0].wrapping_add(0x0a00), 0, 300]),
        ];
        assert!(within_scan_cube(owner, candidates[0].position_raw));
        assert!(within_scan_cube(owner, candidates[1].position_raw));
        assert!(!within_scan_cube(owner, candidates[2].position_raw));

        assert!(candidate_is_eligible(1, owner, &candidates[0]));
        candidates[0].state_flags = RetailStateWord::exact(0xC000);
        assert!(!candidate_is_eligible(1, owner, &candidates[0]));
        candidates[0].state_flags = RetailStateWord::from_known_bits(0x8000, 0x8000);
        assert!(!candidate_is_eligible(1, owner, &candidates[0]));
        candidates[0].state_flags = RetailStateWord::exact(0x8000);
        candidates[0].capability_flags = 0x10 | 1;
        assert!(!candidate_is_eligible(1, owner, &candidates[0]));
        candidates[0].capability_flags = 1;
        candidates[0].entity_type = 0x6e;
        assert!(!candidate_is_eligible(1, owner, &candidates[0]));
    }

    #[test]
    fn exact_probe_beats_near_and_nearest_exact_parameter_wins() {
        let candidates = [
            candidate(2, [100, 0, 1_000]),
            candidate(3, [0, 0, 2_000]),
            candidate(4, [0, 0, 1_500]),
        ];
        let mut runtime = TargetterRuntime::new(1);
        runtime.install();
        runtime.update(
            TargetterUpdateRequest {
                elapsed_micros: 20_000,
                ray: straight_ray(),
                candidates: &candidates,
            },
            |candidate, _, _| {
                if candidate.id >= 3 {
                    TargetterModelProbe::Hit
                } else {
                    TargetterModelProbe::Miss
                }
            },
            |_, _| -10_000,
        );
        assert_eq!(runtime.current_target_id(), Some(4));
        assert_eq!(runtime.result_bits() & 6, TARGETTER_RESULT_EXACT_ENTITY);
        assert_eq!(runtime.entity_ray_parameter_raw, 1_500);
    }

    #[test]
    fn near_scoring_gives_current_target_three_to_four_stickiness() {
        let initial = [candidate(2, [100, 0, 1_000])];
        let mut runtime = TargetterRuntime::new(1);
        runtime.install();
        runtime.update(
            TargetterUpdateRequest {
                elapsed_micros: 20_000,
                ray: straight_ray(),
                candidates: &initial,
            },
            |_, _, _| TargetterModelProbe::Miss,
            |_, _| -10_000,
        );
        assert_eq!(runtime.current_target_id(), Some(2));

        // Current distance 120 scores 360; challenger distance 100 scores 400.
        let next = [candidate(3, [100, 0, 1_000]), candidate(2, [120, 0, 1_000])];
        runtime.update(
            TargetterUpdateRequest {
                elapsed_micros: 20_000,
                ray: straight_ray(),
                candidates: &next,
            },
            |_, _, _| TargetterModelProbe::Miss,
            |_, _| -10_000,
        );
        assert_eq!(runtime.current_target_id(), Some(2));
    }

    #[test]
    fn current_prior_and_one_second_overlay_boundary_match_retail() {
        let target = [candidate(2, [0, 0, 1_000])];
        let mut runtime = TargetterRuntime::new(1);
        runtime.install();
        runtime.update(
            TargetterUpdateRequest {
                elapsed_micros: 10,
                ray: straight_ray(),
                candidates: &target,
            },
            |_, _, _| TargetterModelProbe::Hit,
            |_, _| -10_000,
        );
        assert_eq!(runtime.overlay(true).ring.unwrap().target_id, 2);

        runtime.update(
            TargetterUpdateRequest {
                elapsed_micros: TARGETTER_PRIOR_RETENTION_US - 1,
                ray: straight_ray(),
                candidates: &[],
            },
            |_, _, _| TargetterModelProbe::Miss,
            |_, _| -10_000,
        );
        assert_eq!(runtime.current_target_id(), None);
        assert_eq!(runtime.prior_target_id(), Some(2));
        // The draw callback requires any current result bit. In normal play
        // the independently converged terrain probe supplies bit 1 while the
        // one-second prior-target ring is retained.
        runtime.result_bits |= TARGETTER_RESULT_TERRAIN;
        assert_eq!(runtime.overlay(true).ring.unwrap().target_id, 2);

        runtime.update(
            TargetterUpdateRequest {
                elapsed_micros: 1,
                ray: straight_ray(),
                candidates: &[],
            },
            |_, _, _| TargetterModelProbe::Miss,
            |_, _| -10_000,
        );
        runtime.result_bits |= TARGETTER_RESULT_TERRAIN;
        assert!(runtime.overlay(true).ring.is_none());
    }

    #[test]
    fn dying_target_clears_on_the_next_update() {
        let mut live = candidate(2, [0, 0, 1_000]);
        let mut runtime = TargetterRuntime::new(1);
        runtime.install();
        runtime.update(
            TargetterUpdateRequest {
                elapsed_micros: 20_000,
                ray: straight_ray(),
                candidates: &[live],
            },
            |_, _, _| TargetterModelProbe::Hit,
            |_, _| -10_000,
        );
        live.state_flags = RetailStateWord::exact(0xC000);
        runtime.update(
            TargetterUpdateRequest {
                elapsed_micros: 20_000,
                ray: straight_ray(),
                candidates: &[live],
            },
            |_, _, _| TargetterModelProbe::Hit,
            |_, _| -10_000,
        );
        assert_eq!(runtime.current_target_id(), None);
        assert_eq!(runtime.prior_target_id(), Some(2));
    }

    #[test]
    fn terrain_step_converges_and_sets_bit_one_independently() {
        let mut runtime = TargetterRuntime::new(1);
        runtime.install();
        runtime.terrain_ray_distance_raw = 100;
        runtime.terrain_step_raw = 8;
        runtime.update(
            TargetterUpdateRequest {
                elapsed_micros: 20_000,
                ray: TargetterRay {
                    origin_raw: [0, 100, 0],
                    velocity_raw: [0, -1_000, 1_000],
                    trajectory: TargetterTrajectory::Straight,
                },
                candidates: &[],
            },
            |_, _, _| TargetterModelProbe::Miss,
            |_, _| 0,
        );
        assert_ne!(runtime.result_bits() & TARGETTER_RESULT_TERRAIN, 0);
        assert!(runtime.terrain_step_raw() < TARGETTER_TERRAIN_CONVERGED_BELOW_RAW);
    }

    #[test]
    fn result_bits_select_the_executable_model_branches() {
        let mut runtime = TargetterRuntime::new(1);
        runtime.install();
        runtime.current_target_id = Some(2);
        runtime.time_on_current_target_us = 1;
        runtime.result_bits = TARGETTER_RESULT_EXACT_ENTITY;
        runtime.entity_point_raw = [1, 2, 3];
        let exact = runtime.overlay(true);
        assert_eq!(exact.ring.unwrap().model_id, TARGETTER_RING_EXACT_MODEL_ID);
        assert_eq!(
            exact.crosshair.unwrap().model_id,
            TARGETTER_EXACT_CROSSHAIR_MODEL_ID
        );

        runtime.result_bits = TARGETTER_RESULT_NEAR_ENTITY;
        let near = runtime.overlay(true);
        assert_eq!(near.ring.unwrap().model_id, TARGETTER_RING_NEAR_MODEL_ID);
        assert_eq!(
            near.crosshair.unwrap().model_id,
            TARGETTER_NEAR_CROSSHAIR_MODEL_ID
        );

        runtime.current_target_id = None;
        runtime.prior_target_id = None;
        runtime.result_bits = TARGETTER_RESULT_TERRAIN;
        runtime.terrain_point_raw = [4, 5, 6];
        let terrain = runtime.overlay(true);
        assert!(terrain.ring.is_none());
        assert_eq!(
            terrain.crosshair.unwrap().model_id,
            TARGETTER_TERRAIN_CROSSHAIR_MODEL_ID
        );
    }

    #[test]
    fn targetter_config_suppresses_only_the_entity_ring() {
        let mut runtime = TargetterRuntime::new(1);
        runtime.install();
        runtime.current_target_id = Some(2);
        runtime.time_on_current_target_us = 1;
        runtime.result_bits = TARGETTER_RESULT_EXACT_ENTITY;
        runtime.entity_point_raw = [1, 2, 3];

        let hidden_ring = runtime.overlay(false);
        assert!(hidden_ring.ring.is_none());
        assert_eq!(
            hidden_ring.crosshair.unwrap().model_id,
            TARGETTER_EXACT_CROSSHAIR_MODEL_ID
        );
    }

    #[test]
    fn ballistic_prediction_uses_retail_signed_shift() {
        let ray = TargetterRay {
            origin_raw: [0; 3],
            velocity_raw: [0, 2_000, 4_000],
            trajectory: TargetterTrajectory::Ballistic,
        };
        let point = ray_point_at(ray, 500);
        let drop = ((500_i64 * 500 * i64::from(TARGETTER_BALLISTIC_GRAVITY_RAW)) >> 31) as i16;
        assert_eq!(point, [0, 1_000_i16.wrapping_sub(drop), 2_000]);
    }

    #[test]
    fn callback_scale_adds_32_then_shifts_and_caps() {
        assert_eq!(targetter_model_scale_raw(0), 0x1000);
        assert_eq!(targetter_model_scale_raw(100), 0x4200);
        assert_eq!(targetter_model_scale_raw(u16::MAX), u16::MAX);
    }

    #[test]
    fn terrain_probe_uses_signed_heights_bilinear_fractions_and_word_wrap() {
        let mut terrain = TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        };
        let set_height = |terrain: &mut TerrainGrid, x: usize, z: usize, height: i8| {
            terrain.cells[x * GRID_SIZE + z].height = height as u8;
        };
        set_height(&mut terrain, 0, 0, 0);
        set_height(&mut terrain, 1, 0, 10);
        set_height(&mut terrain, 0, 1, -10);
        set_height(&mut terrain, 1, 1, 20);
        assert_eq!(targetter_terrain_height_raw(&terrain, 0x0080, 0x0080), 160);

        set_height(&mut terrain, 255, 0, -8);
        // x=0xffff interpolates 255/256 of the way from wrapped cell 255 to 0.
        assert_eq!(targetter_terrain_height_raw(&terrain, -1, 0), -1);
    }
}
