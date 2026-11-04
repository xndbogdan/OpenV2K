//! The camera glide that opens every world with a player (`FUN_004530D0`).
//!
//! `FUN_00451710` stands the class-1 camera proxy on the player's body, up
//! 0x200 and along Z by `0x700 - far` ([`camera_arrival_raw`]), and starts
//! the eye on it. Each frame `FUN_004530D0` then measures the player's Y and
//! Z words against the proxy's (X is equal from the copy). Once both
//! distances are under `0x12D` it starts play; until then it adds
//! `(distance * (dt << 14)) >> 31` to each word, about 15% a 20-ms frame.
//! The visit in the load's own frame already takes the first step, and the
//! frame that finds the proxy arrived presents nothing: play's first frame
//! is the next one. No phase below 5 but the attract demo's builds a player
//! (`FUN_00450390`), so those worlds, and network play (`DAT_004F741C`),
//! start play at the first visit, with no glide.
//!
//! [`camera_arrival_raw`]: crate::closing_scene::camera_arrival_raw

use crate::chase_camera::ChaseCameraState;
use crate::opening::IntroCameraController;

/// `FUN_004530D0` starts play once both distances are at most `0x12C`.
pub const ARRIVAL_DISTANCE_RAW: i32 = 0x12D;

/// One visit's step for one word: `IMUL` of the distance by `dt << 14`, the
/// product's bits 31..62, added as a word.
pub fn glide_step_raw(distance: i32, elapsed_micros: u32) -> i16 {
    let scale = (elapsed_micros as i32).wrapping_shl(14);
    ((i64::from(distance) * i64::from(scale)) >> 31) as i32 as i16
}

/// What one `FUN_004530D0` visit did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlideVisit {
    /// The proxy moved; the world stays under the glide.
    Stepped,
    /// Both distances are under `0x12D`: play starts with the next frame,
    /// and this one presents nothing.
    Arrived,
}

/// The class-1 proxy and the eye on it while the world opens.
pub struct WorldStartGlide {
    pub camera: IntroCameraController,
}

impl WorldStartGlide {
    /// `FUN_00451710`'s proxy and `FUN_0040F390`'s fresh springs; the caller
    /// runs the eye's first update with no elapsed time.
    pub fn new(player_raw: [i16; 3], far_cells: u32, world_style: u32) -> Self {
        Self {
            camera: IntroCameraController::at_rest(
                crate::closing_scene::camera_arrival_raw(player_raw, far_cells),
                world_style,
                ChaseCameraState::default(),
            ),
        }
    }

    /// One `FUN_004530D0` visit against the player's position words.
    pub fn visit(&mut self, player_raw: [i16; 3], elapsed_micros: u32) -> GlideVisit {
        let proxy = self.camera.position_raw();
        let dy = i32::from(player_raw[1]) - i32::from(proxy[1]);
        let dz = i32::from(player_raw[2]) - i32::from(proxy[2]);
        if dy.abs() < ARRIVAL_DISTANCE_RAW && dz.abs() < ARRIVAL_DISTANCE_RAW {
            return GlideVisit::Arrived;
        }
        self.camera.set_position_raw([
            proxy[0],
            proxy[1].wrapping_add(glide_step_raw(dy, elapsed_micros)),
            proxy[2].wrapping_add(glide_step_raw(dz, elapsed_micros)),
        ]);
        GlideVisit::Stepped
    }

    /// The eye's springs carry into play: `FUN_004532E0` only retargets
    /// session `+2A8` to the player's body.
    pub fn into_chase(self) -> ChaseCameraState {
        self.camera.into_chase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The retail lab's Replay this World on Level 1 (player at Y -256, Z
    /// 0x3A00, far 0x15 cells): the proxy's Y and Z words after each visit,
    /// sixteen steps from the load's frame, then arrival.
    #[test]
    fn replay_of_level_one_steps_as_the_lab_measured() {
        let player = [0x4D00, -256, 0x3A00];
        let mut glide = WorldStartGlide::new(player, 0x15, 0);
        let lab: [(i16, i16); 16] = [
            (0x00B1, 0x2E22),
            (0x006E, 0x2FF1),
            (0x0036, 0x3179),
            (0x0006, 0x32C6),
            (-34, 0x33E0),
            (-68, 0x34CF),
            (-97, 0x3599),
            (-122, 0x3644),
            (-143, 0x36D5),
            (-161, 0x3750),
            (-176, 0x37B8),
            (-189, 0x3811),
            (-200, 0x385C),
            (-209, 0x389C),
            (-217, 0x38D2),
            (-223, 0x3900),
        ];
        for (y, z) in lab {
            assert_eq!(glide.visit(player, 20_000), GlideVisit::Stepped);
            assert_eq!(glide.camera.position_raw(), [0x4D00, y, z]);
        }
        assert_eq!(glide.visit(player, 20_000), GlideVisit::Arrived);
        assert_eq!(glide.camera.position_raw(), [0x4D00, -223, 0x3900]);
    }

    /// The lab's New Game and Replay this World on Level 1 (Active Camera 6):
    /// the springs after the load's pass, from the loading card's eye level
    /// with the proxy, then after each glide frame's ED10 with terrain, to
    /// the arriving frame.
    #[v2k_test_support::retail_test]
    fn level_one_glide_springs_match_the_lab() {
        let mut session =
            crate::session::GameSession::init(&v2k_test_support::retail_dir()).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(13, 1).unwrap();
        let level = session.cache.level_desc().unwrap();
        let player = [0x4D00, -256, 0x3A00];
        let mut glide = WorldStartGlide::new(player, level.terrain_draw_depth, level.world_style);
        let lab: [[[i16; 3]; 4]; 17] = [
            [
                [19712, 256, 11513],
                [0, 0, 0],
                [19712, 256, 9216],
                [0, 0, 0],
            ],
            [
                [19712, 255, 11803],
                [0, -32, 529],
                [19712, 256, 9261],
                [0, 0, 164],
            ],
            [
                [19712, 252, 12266],
                [0, -133, 1212],
                [19712, 255, 9722],
                [0, -1, 453],
            ],
            [
                [19712, 246, 12658],
                [0, -285, 1780],
                [19712, 253, 10111],
                [0, -81, 707],
            ],
            [
                [19712, 236, 12991],
                [0, -473, 2253],
                [19712, 248, 10443],
                [0, -218, 934],
            ],
            [
                [19712, 222, 13273],
                [0, -682, 2645],
                [19712, 240, 10723],
                [0, -392, 1136],
            ],
            [
                [19712, 188, 13512],
                [0, -900, 2968],
                [19712, 228, 10962],
                [0, -585, 1318],
            ],
            [
                [19712, 159, 13714],
                [0, -1101, 3233],
                [19712, 213, 11163],
                [0, -781, 1481],
            ],
            [
                [19712, 134, 13885],
                [0, -1285, 3451],
                [19712, 194, 11333],
                [0, -972, 1628],
            ],
            [
                [19712, 106, 14030],
                [0, -1455, 3628],
                [19712, 172, 11478],
                [0, -1146, 1762],
            ],
            [
                [19712, 75, 14153],
                [0, -1604, 3772],
                [19712, 147, 11601],
                [0, -1298, 1884],
            ],
            [
                [19712, 42, 14257],
                [0, -1728, 3888],
                [19712, 119, 11704],
                [0, -1422, 1994],
            ],
            [
                [19712, 7, 14346],
                [0, -1824, 3981],
                [19712, 90, 11793],
                [0, -1512, 2096],
            ],
            [
                [19712, -30, 14423],
                [0, -1889, 4054],
                [19712, 60, 11868],
                [0, -1571, 2190],
            ],
            [
                [19712, -67, 14501],
                [0, -1920, 4109],
                [19712, 29, 11932],
                [0, -1597, 2277],
            ],
            [
                [19712, -104, 14579],
                [0, -1918, 4136],
                [19712, -2, 11986],
                [0, -1592, 2357],
            ],
            [
                [19712, -140, 14657],
                [0, -1883, 4127],
                [19712, -32, 12032],
                [0, -1555, 2432],
            ],
        ];
        glide.camera.start_eye_after_load(6);
        assert_eq!(glide.visit(player, 20_000), GlideVisit::Stepped);
        assert_eq!(glide.camera.clone().into_chase().springs_raw(), lab[0]);
        for (frame, springs) in lab.iter().enumerate().skip(1) {
            glide
                .camera
                .advance_eye(crate::opening::IntroCameraEyeUpdate {
                    elapsed_micros: 20_000,
                    active_camera: 6,
                    terrain: Some(crate::chase_camera::ChaseTerrainContext {
                        terrain: session.cache.terrain().unwrap(),
                        terrain_objects: session.cache.terrain_objects().unwrap(),
                        model_pool: &session.cache,
                    }),
                });
            assert_eq!(
                glide.camera.clone().into_chase().springs_raw(),
                *springs,
                "frame {frame}"
            );
            let arrived = glide.visit(player, 20_000) == GlideVisit::Arrived;
            assert_eq!(arrived, frame == 16);
        }
    }

    #[test]
    fn step_floors_negative_distances() {
        assert_eq!(glide_step_raw(-512, 20_000), -79);
        assert_eq!(glide_step_raw(302, 20_000), 46);
        assert_eq!(glide_step_raw(0, 20_000), 0);
    }
}
