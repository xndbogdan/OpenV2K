//! The type-67 wreck's descending rings, `LAB_0041CB10`.
//!
//! `FUN_00438080` replaces type 67's presentation slot `+0x20` with this
//! callback after construction. It draws `FUN_004138F0`'s ordinary body first,
//! then ten independently phased instances of global model 243. This is a
//! presentation suffix, not a particle allocator or a new type-111 gate.

use v2k_formats::{fixed_math::retail_sine_q15, models::AnimVars, terrain::TerrainGrid};

use crate::{
    chase_camera::terrain_height_raw, entity::Entity, entity_collision_state::RetailRuntimeValue,
    entity_emitters::HIVE_CONTROLLER_DEAD, hover::q31_mul,
};

pub const HIVE_WRECK_RING_MODEL_ID: usize = 243;

/// The retained Sub-N inputs read by `LAB_0041CB10`, after body admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveWreckPresentationRequest {
    pub controller_state: u32,
    pub marker_present: bool,
    pub suction_timer_us: i32,
    pub anchor_raw: [i16; 3],
}

impl HiveWreckPresentationRequest {
    /// Constructor-owned type identity and component custody are required;
    /// an arbitrary dying model does not acquire the hive callback.
    pub fn from_entity(entity: &Entity) -> Option<Self> {
        // 11400's render flag gates the installed callback, including its
        // suffix. Do not inherit the general body adapter's unresolved gate.
        if entity.entity_type != 67
            || entity.collision.state_flags_at_0x08.masked(0x800)
                != RetailRuntimeValue::Known(0x800)
        {
            return None;
        }
        let controller = entity.authored_radial_emitter.as_ref()?;
        let RetailRuntimeValue::Known(Some(sub_n)) = entity.sub_n_runtime else {
            return None;
        };
        let RetailRuntimeValue::Known(marker_present) = sub_n.terrain_patch_present() else {
            return None;
        };
        let RetailRuntimeValue::Known(anchor_raw) = sub_n.anchor_raw() else {
            return None;
        };
        Some(Self {
            controller_state: controller.controller_state(),
            marker_present,
            suction_timer_us: controller.suction_timer_us(),
            anchor_raw,
        })
    }

    /// `41CB5D..41CE06`: the timer phases begin drawing during the six-second
    /// delay. Requiring a positive timer here would lose that authored build-up.
    pub fn ring_submissions(self, terrain: &TerrainGrid) -> Vec<HiveWreckRingSubmission> {
        if self.controller_state != HIVE_CONTROLLER_DEAD || !self.marker_present {
            return Vec::new();
        }
        let [x, _, z] = self.anchor_raw;
        // `41CBB2..41CC79` samples signed terrain, not sea or the Sub-N Y word.
        let base_y = i32::from(terrain_height_raw(terrain, x, z)) + 200;
        (0u8..10)
            .filter_map(|sample_index| {
                let time_us = self
                    .suction_timer_us
                    .wrapping_add(i32::from(sample_index) * 600_000);
                if time_us <= 0 {
                    return None;
                }
                let phase_ms = (time_us / 1_000) % 6_000;
                let (height_raw, scale_raw) = if phase_ms < 3_000 {
                    let third = phase_ms / 3;
                    (
                        3_000 - third * third / 1_000,
                        ((3_000 - phase_ms) * 65_535 / 3_000).max(64),
                    )
                } else {
                    (2_000 - (phase_ms - 3_000) / 2, 64)
                };
                let square = phase_ms * phase_ms;
                // `41CD11/41CD21`: even instances negate before SAR 8.
                let rotation = if sample_index & 1 != 0 {
                    square >> 8
                } else {
                    (-square) >> 8
                };
                Some(HiveWreckRingSubmission {
                    sample_index,
                    phase_ms: phase_ms as u16,
                    position_raw: [x, (base_y + height_raw) as i16, z],
                    yaw_raw: rotation as u16,
                    // `421570` returns callback param one as a word for
                    // selector 1. Model 243 performs the actual morph.
                    shrink_output_raw: (65_535 - scale_raw) as u16,
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiveWreckRingSubmission {
    pub sample_index: u8,
    pub phase_ms: u16,
    pub position_raw: [i16; 3],
    pub yaw_raw: u16,
    pub shrink_output_raw: u16,
}

impl HiveWreckRingSubmission {
    /// `41CC53/41CC89/41CDBF`: each signed word is unwrapped against the
    /// camera's full dword. This includes Y; it is a presentation conversion,
    /// not a claim that the simulated world wraps vertically.
    pub fn unwrapped_position_raw(self, camera_position_raw: [i32; 3]) -> [i32; 3] {
        std::array::from_fn(|axis| {
            camera_position_raw[axis].wrapping_add(i32::from(
                self.position_raw[axis].wrapping_sub(camera_position_raw[axis] as i16),
            ))
        })
    }

    /// `FUN_00421570`: selector 0 is global time, 1 is this ring's morph;
    /// every other selector returns zero.
    pub fn anim_vars(self, retail_tick: u32) -> AnimVars {
        let mut vars = AnimVars::default();
        vars.dynamic[0] = i32::from(retail_tick as u16);
        vars.dynamic[1] = i32::from(self.shrink_output_raw);
        vars
    }

    /// `FUN_00457C30` rotates a Q31 identity around Y. Keep its literal sine
    /// words and product rounding, then convert once at the GL submission.
    pub fn orientation(self) -> [[f32; 3]; 3] {
        let sine = |angle| {
            let signed = retail_sine_q15(angle);
            let magnitude = signed.unsigned_abs();
            let duplicated = ((magnitude << 16) | magnitude) as i32;
            if signed < 0 {
                -duplicated
            } else {
                duplicated
            }
        };
        let angle = u32::from(self.yaw_raw);
        let s = sine(angle);
        let c = sine(angle + 0x4000);
        let basis = [
            [q31_mul(c, i32::MAX), 0, q31_mul(s, i32::MAX)],
            [0, i32::MAX, 0],
            [q31_mul(-s, i32::MAX), 0, q31_mul(c, i32::MAX)],
        ];
        // Native consecutive vectors are local axes; the renderer consumes
        // world-from-model rows. Transpose after the signed products so the
        // two sine terms retain their independent Q31 rounding.
        std::array::from_fn(|row| {
            std::array::from_fn(|column| (f64::from(basis[column][row]) / 2_147_483_648.0) as f32)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

    fn terrain() -> TerrainGrid {
        TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn request(timer: i32) -> HiveWreckPresentationRequest {
        HiveWreckPresentationRequest {
            controller_state: HIVE_CONTROLLER_DEAD,
            marker_present: true,
            suction_timer_us: timer,
            anchor_raw: [-17_536, 7_777, -32_384],
        }
    }

    #[test]
    fn rings_build_up_before_suction_with_strict_phase_boundaries() {
        let terrain = terrain();
        for timer in [-6_000_000, -5_400_000] {
            assert!(request(timer).ring_submissions(&terrain).is_empty());
        }
        let first = request(-5_399_999).ring_submissions(&terrain);
        assert_eq!(first.len(), 1);
        assert_eq!((first[0].sample_index, first[0].phase_ms), (9, 0));
        assert_eq!(first[0].position_raw, [-17_536, 3_200, -32_384]);
        assert_eq!(first[0].shrink_output_raw, 0);
        assert_eq!(request(-3_000_000).ring_submissions(&terrain).len(), 4);
        assert_eq!(request(-2_999_999).ring_submissions(&terrain).len(), 5);
        assert_eq!(request(0).ring_submissions(&terrain).len(), 9);
        assert_eq!(request(1).ring_submissions(&terrain).len(), 10);
        let repeated = request(6_000_001).ring_submissions(&terrain);
        assert_eq!(repeated, request(1).ring_submissions(&terrain));
        let mut hidden = request(1);
        hidden.controller_state = 2;
        assert!(hidden.ring_submissions(&terrain).is_empty());
        hidden.controller_state = 0;
        hidden.marker_present = false;
        assert!(hidden.ring_submissions(&terrain).is_empty());
    }

    #[test]
    fn ring_height_morph_and_opposed_rotation_follow_both_native_halves() {
        let terrain = terrain();
        let at_phase = |phase_ms: i32| request(phase_ms * 1_000).ring_submissions(&terrain)[0];
        let middle = at_phase(1_500);
        assert_eq!(middle.position_raw[1], 2_950);
        assert_eq!(middle.shrink_output_raw, 32_768);
        assert_eq!(middle.yaw_raw, ((-2_250_000i32) >> 8) as u16);
        let second_half = at_phase(3_000);
        assert_eq!(second_half.position_raw[1], 2_200);
        assert_eq!(second_half.shrink_output_raw, 65_471);
        assert_eq!(at_phase(5_999).position_raw[1], 701);
        assert_eq!(at_phase(6_000).position_raw[1], 3_200);
        let odd = request(400_000).ring_submissions(&terrain)[1];
        assert_eq!(odd.phase_ms, 1_000);
        assert_eq!(odd.yaw_raw, (1_000_000i32 >> 8) as u16);
        let vars = middle.anim_vars(0x12345);
        assert_eq!(vars.dynamic[0], 0x2345);
        assert_eq!(vars.dynamic[1], 32_768);
        assert!(vars.dynamic[2..].iter().all(|word| *word == 0));
    }

    #[test]
    fn ring_base_samples_wrapped_signed_bilinear_terrain_not_anchor_y() {
        let mut terrain = terrain();
        for (x, z, height) in [(255, 255, -8i8), (0, 255, -4), (255, 0, 2), (0, 0, 6)] {
            terrain.cells[x * GRID_SIZE + z].height = height as u8;
        }
        let mut input = request(6_000_000);
        input.anchor_raw = [-128, 32_000, -128];
        let first = input.ring_submissions(&terrain)[0];
        assert_eq!(first.position_raw, [-128, 3_168, -128]);
    }

    #[test]
    fn native_quarter_turn_maps_local_x_toward_world_z() {
        let ring = HiveWreckRingSubmission {
            sample_index: 0,
            phase_ms: 0,
            position_raw: [0; 3],
            yaw_raw: 0x4000,
            shrink_output_raw: 0,
        };
        let orientation = ring.orientation();
        assert!(orientation[2][0] > 0.999);
        assert!(orientation[0][2] < -0.999);
        assert!(orientation[0][0].abs() < 0.001);
        assert!(orientation[2][2].abs() < 0.001);
    }

    #[test]
    fn ring_submission_unwraps_each_axis_against_full_camera_dwords() {
        let ring = HiveWreckRingSubmission {
            sample_index: 0,
            phase_ms: 0,
            position_raw: [-32_700, -32_700, 32_700],
            yaw_raw: 0,
            shrink_output_raw: 0,
        };
        assert_eq!(
            ring.unwrapped_position_raw([32_700, 98_236, -32_700]),
            [32_836, 98_372, -32_836],
        );
    }
}
