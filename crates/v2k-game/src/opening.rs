//! New-game opening sequence timing.
//!
//! Retail New Game starts campaign slot `0x26`; the level loader adds the
//! twelve system slots and therefore opens `0X50XX.OVL` (`Intro2`).  Intro2's
//! Section-2 strings contain the timed story captions below.  The mode exits
//! at tick `0x10CC` of the 50 Hz clock (about 86 seconds), not after 4.3 s as
//! an older RE note claimed.

use crate::chase_camera::{
    terrain_height_raw, ChaseBodyBasis, ChaseCameraPose, ChaseCameraState, ChaseCameraTarget,
    ChaseTerrainContext,
};
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::{uses_audited_clock_model_callback, Entity},
    hover::integrate_position_word,
    intro2_type26_defecate_virus::INTRO2_DEFECATE_VIRUS_ENTITY_TYPE,
    intro2_type47_live::INTRO2_TYPE47_ENTITY_TYPE,
};
use v2k_formats::models::AnimVars;

/// Story-intro overlay level selected by a new game (`0x26 + 0x0c`).
pub const INTRO2_LEVEL_ID: u32 = 50;
/// First playable campaign world after Intro2.
pub const FIRST_WORLD_LEVEL_ID: u32 = 13;
/// Intro2 phase cutoff: `0x10CC` ticks at 50 Hz.
pub const INTRO2_DURATION_SECS: f32 = 0x10CC as f32 / 50.0;
/// 503C0 switches the backdrop only when the 50-Hz clock is strictly >4000.
/// The caption record itself may already be active at tick 4000.
pub const INTRO2_BLACK_CARD_START_TICK: u32 = 4_001;

/// Authored background presentation for the current Intro2 clock tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Backdrop {
    World,
    BlackCard,
}

/// Select the retained world montage or its final black title card.
///
/// The accepted full-frame trace resumes black `COLORFILL` for the final `#`
/// page. Once selected, black remains active for any transition-overrun frame;
/// the surrounding Intro2 state owns the subsequent exit-latch handoff.
pub const fn intro2_backdrop(retail_tick: u32) -> Intro2Backdrop {
    if retail_tick >= INTRO2_BLACK_CARD_START_TICK {
        Intro2Backdrop::BlackCard
    } else {
        Intro2Backdrop::World
    }
}

/// 503C0 requests exit only when `0x10CC < g_default_param`. 44FFA0
/// consumes that request on its next visit, before ticking world actors.
pub const fn intro2_finished(retail_tick: u32) -> bool {
    retail_tick > 0x10CC
}
/// Command 5's frontend-background zoom reaches zero after about 524 ms.
///
/// This is not the load boundary: the 2026-07-17 retail session trace keeps
/// the ordinary 0x7000 menu fly clock alive for its complete 626.4-ms leg.
pub const MENU_BACKGROUND_ZOOM_SECS: f32 = 0.524;
/// Physical Section-11 level-3 blob 43 is global pool slot 7 + 43.
pub const POST_INTRO_SOUND_ID: usize = 50;

/// Exact callback inputs currently proven for ordinary Intro2 actors.
///
/// `FUN_0040D320` returns the low 16 bits of the process 50-Hz clock for
/// selector zero. Nonzero selectors address per-entity joint words; this
/// stateless baseline leaves them zero. Live actors publish their recovered
/// component outputs through `Entity::presentation_anim_vars`; the renderer
/// consumes that same bank without advancing a separate presentation clock.
pub fn intro_actor_anim_vars(entity_type: u32, retail_tick: u32) -> AnimVars {
    let mut vars = AnimVars::default();
    if uses_audited_clock_model_callback(entity_type) {
        vars.dynamic[0] = i32::from(retail_tick as u16);
    }
    vars
}

/// Authored class-1 camera spawn before Intro2's first follow callback.
///
/// The 2026-07-19 passive trace observes this exact raw pose at tick 6. The
/// following callback snaps it to target 32, so treating zero as the initial
/// pose loses the complete opening composition even though later shots settle.
const INTRO_CAMERA_INITIAL_POSITION_RAW: [i16; 3] = [0, 0x200, -0x1100];
// FUN_004503C0 follows through an absolute raw error of 0x3200 and enters the
// snap branch only at 0x3201.
const INTRO_CAMERA_SNAP_DISTANCE_RAW: i32 = 0x3200;
const INTRO_CAMERA_MAX_DESIRED_VELOCITY_RAW: i32 = 10_000;
const INTRO_CAMERA_TARGET_TRAIL_RAW: i16 = 0x200;
/// Intro2's Section-2 layout records author a 30-ms interval per character.
/// `FUN_00452790` divides its integer millisecond clock by this value.
pub const INTRO_CAPTION_CHAR_MILLIS: u32 = 30;

/// Stateful proxy for Intro2's hidden class-1 camera entity.
///
/// `FUN_004503C0` does not orbit the scripted subject. Each retail mode update
/// drives the camera entity's wrapping signed-word X/Z velocity toward the
/// active target, trails it by 0x200 raw units on Z, and eases by one sixth.
/// The generic entity integrator advances that velocity before the next
/// `FUN_0040ED10` call. Large discontinuities snap the proxy and reset both
/// final eye/focus springs; ordinary target changes do neither.
#[derive(Debug, Clone)]
pub struct IntroCameraController {
    position_raw: [i16; 3],
    velocity_raw: [i16; 3],
    chase: ChaseCameraState,
}

impl Default for IntroCameraController {
    fn default() -> Self {
        Self {
            position_raw: INTRO_CAMERA_INITIAL_POSITION_RAW,
            velocity_raw: [0; 3],
            chase: ChaseCameraState::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntroCameraSubject {
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
}

#[derive(Clone, Copy)]
pub struct IntroCameraEyeUpdate<'a> {
    pub elapsed_micros: u32,
    pub active_camera: u8,
    pub terrain: Option<ChaseTerrainContext<'a>>,
}

impl IntroCameraController {
    pub fn focus_position_raw(&self) -> [i16; 3] {
        self.chase.focus_position_raw()
    }

    pub fn native_viewport(&self) -> Option<crate::native_model_frame::NativeWorldViewport> {
        self.chase.native_viewport()
    }

    /// Run ED10 before physical particles and contacts. The active world has
    /// already advanced the proxy actor; 50C00's closing transition leaves
    /// that actor fixed while continuing these eye/focus springs.
    pub fn advance_eye(&mut self, request: IntroCameraEyeUpdate<'_>) -> ChaseCameraPose {
        self.chase.update(
            ChaseCameraTarget {
                position_raw: self.position_raw,
                body_basis: ChaseBodyBasis::RETAIL_IDENTITY,
                active_camera: request.active_camera,
            },
            request.terrain,
            request.elapsed_micros,
        )
    }

    /// 503C0 reads the surviving subject after the contact pass. A missing
    /// handle suppresses follow control, without undoing the earlier eye step.
    /// This changes next-frame proxy motion; it does not recompute this eye.
    pub fn follow_subject(
        &mut self,
        subject: Option<IntroCameraSubject>,
        terrain: Option<ChaseTerrainContext<'_>>,
    ) {
        if let Some(subject) = subject {
            self.follow_target(subject.position_raw, subject.velocity_raw, terrain);
        }
    }

    /// Advance the persistent class-1 actor during the active world actor pass.
    /// Captured 8-ms calls do not define a fixed camera substep.
    pub fn advance_proxy(&mut self, elapsed_micros: u32) {
        for axis in 0..3 {
            self.position_raw[axis] = integrate_position_word(
                self.position_raw[axis],
                self.velocity_raw[axis],
                elapsed_micros,
            );
        }
    }

    fn follow_target(
        &mut self,
        target_position_raw: [i16; 3],
        target_velocity_raw: [i16; 3],
        terrain: Option<ChaseTerrainContext<'_>>,
    ) {
        let error_x = target_position_raw[0]
            .wrapping_add(target_velocity_raw[0])
            .wrapping_sub(self.position_raw[0]);
        if Self::follow_error(error_x, &mut self.velocity_raw[0]) {
            self.snap_to_target(target_position_raw);
        }

        let error_z = target_position_raw[2]
            .wrapping_add(target_velocity_raw[2])
            .wrapping_sub(self.position_raw[2])
            .wrapping_sub(INTRO_CAMERA_TARGET_TRAIL_RAW);
        if Self::follow_error(error_z, &mut self.velocity_raw[2]) {
            self.snap_to_target(target_position_raw);
        }

        // Intro2 is not mode 6, so FUN_004503C0 applies its terrain-relative
        // vertical correction. This writes proxy Y directly; target Y velocity
        // and the proxy's stored Y velocity are deliberately absent.
        let mut y_error = target_position_raw[1].wrapping_sub(self.position_raw[1]);
        if let Some(context) = terrain {
            let ground_raw =
                terrain_height_raw(context.terrain, self.position_raw[0], self.position_raw[2]);
            let terrain_bias =
                (i32::from(self.position_raw[1]) - i32::from(ground_raw) + 0x500) / 4;
            y_error = y_error.wrapping_sub(terrain_bias as i16);
            self.position_raw[1] = self.position_raw[1].wrapping_add(y_error / 6);
            let floor_raw = i32::from(ground_raw) + 0x100;
            if i32::from(self.position_raw[1]) < floor_raw {
                self.position_raw[1] = floor_raw as i16;
            }
        } else {
            self.position_raw[1] = self.position_raw[1].wrapping_add(y_error / 6);
        }
    }

    /// Returns true when the caller must take FUN_004503C0's snap branch.
    fn follow_error(error: i16, velocity: &mut i16) -> bool {
        if i32::from(error).abs() > INTRO_CAMERA_SNAP_DISTANCE_RAW {
            return true;
        }
        let desired = i32::from(error).clamp(
            -INTRO_CAMERA_MAX_DESIRED_VELOCITY_RAW,
            INTRO_CAMERA_MAX_DESIRED_VELOCITY_RAW,
        );
        let correction = (desired - i32::from(*velocity)) / 6;
        *velocity = velocity.wrapping_add(correction as i16);
        false
    }

    fn snap_to_target(&mut self, target_position_raw: [i16; 3]) {
        self.position_raw = [
            target_position_raw[0],
            target_position_raw[1],
            target_position_raw[2].wrapping_sub(INTRO_CAMERA_TARGET_TRAIL_RAW),
        ];
        self.velocity_raw = [0; 3];
        self.chase.reset();
    }
}

const INTRO2_CAMERA_ANCHORS: &[usize] = &[14, 22, 23, 27, 28, 29, 32, 37, 39, 47, 48, 57];

/// Whether an Intro2 actor should be submitted at this point in the timeline.
/// Type-52 `flag` objects are invisible behavior/camera anchors. Every other
/// authored spawn carries retail render bit `0x800` from the first stable
/// Intro2 sample; operation 2 enables behavior/integration, not visibility.
pub fn intro_actor_visible(entity_index: usize) -> bool {
    !INTRO2_CAMERA_ANCHORS.contains(&entity_index)
}

fn intro_actor_model_slot(entity_index: usize) -> usize {
    match entity_index {
        // The full-session trace records the type-115 infected plant born and
        // retained on slot 2: model 332 `virusedsunflower`, not slot-0 model
        // 331 `sunflwr`. This is initial state, independent of timeline age.
        61 => 2,
        _ => 0,
    }
}

fn intro_actor_motion_position(entity_index: usize, base: [f32; 3], elapsed_secs: f32) -> [f32; 3] {
    // Creature groups begin moving on the Section-2 command clusters that
    // introduce them. Ground attackers advance in deterministic formation;
    // flying attackers add the authored-looking altitude oscillation that
    // their generic AI callback would otherwise supply. Default-operation
    // camera cuts to the red turret actors (52..=54) do not activate motion:
    // retail keeps their authored position and parked basis until combat
    // destroys them.
    let attack_start = match entity_index {
        4 | 5 | 20 | 38 => Some(5.5),
        21 | 46 => Some(6.5),
        7 | 8 | 30 | 41 | 42 | 43 | 44 => Some(15.0),
        // Entity 61 is the infected sunflower selected by the same command
        // cluster. The full-session retail trace keeps its position, velocity,
        // and rotation exactly authored throughout; operation 2 changes its
        // activation flags, not its world transform.
        // Type77 spawn 45 has capability 8 and no enable record; bounce or a
        // synthetic flying group is not a substitute for that dormant gate.
        0 | 6 | 10 | 25 => Some(22.0),
        _ => None,
    };
    if let Some(start) = attack_start {
        let t = (elapsed_secs - start).max(0.0);
        let distance = (t * 1.7).min(18.0);
        let angle = entity_index as f32 * 2.399_963_1;
        let flying = matches!(entity_index, 0 | 44 | 46);
        return [
            base[0] + angle.sin() * distance,
            base[1]
                + if flying {
                    2.5 + (t * 2.2 + entity_index as f32).sin() * 1.2
                } else {
                    0.0
                },
            base[2] + angle.cos() * distance,
        ];
    }

    // Villagers scatter once the attack reaches the settlement at 22 s.
    if matches!(
        entity_index,
        2 | 3 | 9 | 11 | 16 | 17 | 18 | 19 | 49 | 50 | 58 | 59 | 60
    ) {
        let t = (elapsed_secs - 22.0).clamp(0.0, 8.0);
        let angle = entity_index as f32 * 1.618_034;
        return [
            base[0] + angle.sin() * t * 1.25,
            base[1],
            base[2] + angle.cos() * t * 1.25,
        ];
    }

    base
}

/// Whether Intro2 should submit this actor from its committed runtime pose.
///
/// Recovered component ownership survives task replacement. Presentation uses
/// the same position, body basis, active model and selector bank as the live
/// scheduler and camera; a scripted trajectory must not overwrite those writes.
pub fn intro2_uses_live_actor_pose(entity: &Entity) -> bool {
    match entity.entity_type {
        8 => crate::intro2_type8::intro2_type8_allocation_authenticates(entity),
        9 => crate::intro2_type9::intro2_type9_allocation_authenticates(entity),
        16 => crate::intro2_type16::intro2_type16_allocation_authenticates(entity),
        58 => crate::intro2_type58::intro2_type58_allocation_authenticates(entity),
        66 => crate::intro2_type66::intro2_type66_allocation_authenticates(entity),
        10 => crate::intro2_type10::intro2_type10_allocation_authenticates(entity),
        92 | 102 | 115 => {
            crate::intro2_gun_turret::intro2_gun_turret_allocation_authenticates(entity)
        }
        94 => crate::intro2_type94::intro2_type94_allocation_authenticates(entity),
        17 => crate::intro2_type17::intro2_type17_allocation_authenticates(entity),
        crate::intro2_type13_live::TYPE13_ENTITY_TYPE => {
            crate::intro2_type13_live::authenticate_intro2_type13(entity).is_ok()
                && entity.intro2_type13_common_mover_runtime.is_some()
        }
        crate::intro2_meteors::INTRO2_METEOR_TYPE => {
            crate::intro2_meteors::Intro2MeteorOwner::adopt_published(entity).is_ok()
        }
        INTRO2_DEFECATE_VIRUS_ENTITY_TYPE => entity.intro2_type26_sub_d_frame_owner.is_some(),
        INTRO2_TYPE47_ENTITY_TYPE => entity.intro2_type47_sub_d_frame_owner.is_some(),
        53 => crate::intro2_type53::intro2_type53_allocation_authenticates(entity),
        122 => crate::native_type122::type122_allocation_authenticates(entity),
        crate::intro2_flyers_live::INTRO2_TYPE15_ENTITY_TYPE
        | crate::intro2_flyers_live::INTRO2_TYPE87_ENTITY_TYPE => {
            entity.intro2_flyer_frame_owner.is_some()
        }
        57 => crate::intro2_type57::intro2_type57_allocation_authenticates(entity),
        77 => intro2_type77_dormant_allocation_authenticates(entity),
        52 => intro2_type52_class0_allocation_authenticates(entity),
        67 => intro2_type67_hive_allocation_authenticates(entity),
        _ => false,
    }
}

/// Intro2's twelve Type52 flags publish the shared class-0 timer. They are
/// invisible camera/beacon anchors, so live pose is the grounded entity
/// transform rather than `intro_actor_pose`.
pub fn intro2_type52_class0_allocation_authenticates(entity: &Entity) -> bool {
    entity.active
        && entity.entity_type == 52
        && entity.capability_flags == 0x100
        && entity.model_slots == [Some(145); 4]
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::Class0Timer(_))
        )
}

/// Intro2 spawn 24 retains the authored hive radial emitter. Presentation must
/// follow the live model/pose so a later wreck slot is not stuck on slot 0.
pub fn intro2_type67_hive_allocation_authenticates(entity: &Entity) -> bool {
    entity.active && entity.entity_type == 67 && entity.authored_radial_emitter.is_some()
}

/// Type77 spawn 45 is constructed and visible, but capability 8 never receives
/// an operation-2 enable. Presentation must keep the committed authored pose
/// rather than the old synthetic flying oscillator.
pub fn intro2_type77_dormant_allocation_authenticates(entity: &Entity) -> bool {
    entity.active
        && entity.entity_type == 77
        && entity.authored_spawn_index == Some(45)
        && entity.model_slots == [Some(271); 4]
        && entity.model_index == Some(271)
        && entity.capability_flags & 0x08 != 0
}

/// Presentation writes the next mover's detail mode from the final live pose
/// and camera, including world passes that Klaus currently covers.
pub fn publish_intro2_actor_view_detail(
    manager: &mut crate::entity::EntityManager,
    context: crate::entity_view_detail::RetailViewDetailContext,
) {
    let ids: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.active && intro2_uses_live_actor_pose(entity))
        .map(|entity| entity.id)
        .collect();
    for id in ids {
        let entity = manager
            .entity_mut(id)
            .expect("presentation does not change topology");
        context.publish(
            entity.position_raw(),
            &mut entity.collision.state_flags_at_0x08,
        );
    }
}

/// Remaining presentation fallback for actors without native task ownership.
/// Native actors, including the hut/factory damage and crater lifecycle, use
/// their committed entity model, position and body basis through
/// [`intro2_uses_live_actor_pose`]. No clock selects a destroyed model here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntroActorPose {
    pub model_slot: usize,
    pub position: [f32; 3],
}

pub fn intro_actor_pose(entity_index: usize, base: [f32; 3], elapsed_secs: f32) -> IntroActorPose {
    IntroActorPose {
        model_slot: intro_actor_model_slot(entity_index),
        position: intro_actor_motion_position(entity_index, base, elapsed_secs),
    }
}

/// Heading supplied by the corresponding Intro2 behavior approximation.
pub fn intro_actor_heading(entity_index: usize) -> f32 {
    if matches!(
        entity_index,
        0 | 4 | 5 | 6 | 7 | 8 | 10 | 19 | 20 | 21 | 25 | 30 | 38 | 41 | 42 | 43 | 44 | 46
    ) {
        return entity_index as f32 * 2.399_963_1;
    }
    0.0
}

/// Where `FUN_00452790` lays out an Intro2 narrative record.
///
/// Every narrative record's header selects placement style 2 (style 3 on the
/// final page), which places the text at 5% of the display mode's width,
/// its baseline at 16% of the height, and wraps it at 80% of the width,
/// each as an integer percentage. 1024x768 gives `(51, 122)`; 640x480 gives
/// `(32, 76)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoryCaptionPlacement {
    pub x: i32,
    pub baseline: i32,
    pub wrap_width: i32,
}

impl StoryCaptionPlacement {
    pub const fn for_display(width: i32, height: i32) -> Self {
        Self {
            x: 5 * width / 100,
            baseline: height * 16 / 100,
            wrap_width: 80 * width / 100,
        }
    }
}

/// A caption active during retail's inclusive `[start, end]` interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StoryCaption {
    /// Absolute Intro2 clock bounds after composing the two final shot-local
    /// records with their Section-2 command windows.
    pub start_millis: u32,
    pub end_millis: u32,
    pub text: &'static str,
}

impl StoryCaption {
    /// Visible ASCII prefix and whether retail's trailing `_` cursor is still
    /// present. Intro2's authored English strings are single-byte text.
    pub fn revealed_text(self, retail_tick: u32) -> (&'static str, bool) {
        let elapsed_millis =
            intro_clock_millis(retail_tick).saturating_sub(u64::from(self.start_millis));
        let visible = (elapsed_millis / u64::from(INTRO_CAPTION_CHAR_MILLIS))
            .min(self.text.len() as u64) as usize;
        (&self.text[..visible], visible < self.text.len())
    }
}

fn intro_clock_millis(retail_tick: u32) -> u64 {
    // The retail caller passes `(DAT_004FED60 * 1000) / 50` to
    // FUN_00452790. Widening first preserves that integer expression without
    // imposing the executable's practical Intro2 tick bound on this helper.
    u64::from(retail_tick) * 1000 / 50
}

/// Narrative captions from `0X50XX.OVL` Section 2.
pub const INTRO2_CAPTIONS: &[StoryCaption] = &[
    StoryCaption {
        start_millis: 2_000,
        end_millis: 6_000,
        text: "The worlds were quiet and peaceful until the meteors came ...",
    },
    StoryCaption {
        start_millis: 7_000,
        end_millis: 15_000,
        text: "The alien creatures followed the meteors in their thousands",
    },
    StoryCaption {
        start_millis: 17_000,
        end_millis: 22_000,
        text: "Travelling between the worlds through their hives",
    },
    StoryCaption {
        start_millis: 22_000,
        end_millis: 25_000,
        text: "Spreading disease and destruction",
    },
    StoryCaption {
        start_millis: 26_000,
        end_millis: 30_000,
        text: "They attacked our villages",
    },
    StoryCaption {
        start_millis: 30_000,
        end_millis: 35_000,
        text: "And slaughtered our people",
    },
    StoryCaption {
        start_millis: 35_000,
        end_millis: 41_000,
        text: "All the time spreading their lethal virus",
    },
    StoryCaption {
        start_millis: 48_500,
        end_millis: 52_000,
        text: "Even our best defences were no match for the advancing monsters",
    },
    StoryCaption {
        start_millis: 69_000,
        end_millis: 75_000,
        text: "We have been completely overwhelmed",
    },
    StoryCaption {
        // The `#` page begins at 80 seconds. This record's wildcard start and
        // 3000-ms end are local to that final page.
        start_millis: 80_000,
        end_millis: 83_000,
        text: "You are our last chance",
    },
    StoryCaption {
        // Raw final-page bounds are 3500..6000 ms. Tick 4300 (86 seconds)
        // remains active; the Intro2 handoff begins after tick 4300.
        start_millis: 83_500,
        end_millis: 86_000,
        text: "End transmission",
    },
];

/// All authored captions active at this tick, in Section-2 table order.
///
/// Retail's bounds are inclusive. Adjacent records therefore both run on a
/// shared boundary tick; processing only the first one would omit the new
/// line's draw and type-on cue.
pub fn active_captions(retail_tick: u32) -> impl Iterator<Item = &'static StoryCaption> + Clone {
    let elapsed_millis = intro_clock_millis(retail_tick);
    INTRO2_CAPTIONS.iter().filter(move |caption| {
        elapsed_millis >= u64::from(caption.start_millis)
            && elapsed_millis <= u64::from(caption.end_millis)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test driver for consecutive source callbacks without intervening world
    // contacts. Production resolves the subject between these two phases.
    struct IntroCameraUpdate<'a> {
        elapsed_micros: u32,
        subject: Option<IntroCameraSubject>,
        active_camera: u8,
        terrain: Option<ChaseTerrainContext<'a>>,
    }

    impl IntroCameraController {
        fn update(&mut self, request: IntroCameraUpdate<'_>) -> ChaseCameraPose {
            self.advance_proxy(request.elapsed_micros);
            let pose = self.advance_eye(IntroCameraEyeUpdate {
                elapsed_micros: request.elapsed_micros,
                active_camera: request.active_camera,
                terrain: request.terrain,
            });
            self.follow_subject(request.subject, request.terrain);
            pose
        }
    }
    use crate::gameplay_notifications::TextTypewriterCadence;

    const CAPTURED_CAMERA_FRAME_MICROS: u32 = 8_000;

    #[test]
    fn intro_duration_uses_the_50_hz_clock() {
        assert!((INTRO2_DURATION_SECS - 86.0).abs() < f32::EPSILON);
        assert!(!intro2_finished(0x10CC));
        assert!(intro2_finished(0x10CC + 1));
    }

    #[test]
    fn final_page_uses_black_through_the_transition_frame() {
        assert_eq!(intro2_backdrop(3_999), Intro2Backdrop::World);
        assert_eq!(intro2_backdrop(4_000), Intro2Backdrop::World);
        assert_eq!(intro2_backdrop(4_001), Intro2Backdrop::BlackCard);
        assert_eq!(intro2_backdrop(4_300), Intro2Backdrop::BlackCard);
        assert_eq!(intro2_backdrop(4_301), Intro2Backdrop::BlackCard);
    }

    #[test]
    fn narrative_captions_use_style_two_screen_percentages() {
        // DirectDraw trace frame 345026 draws the caption's left edge at 51
        // and its glyph bottom at 122 on the 1024x768 mode.
        assert_eq!(
            StoryCaptionPlacement::for_display(1024, 768),
            StoryCaptionPlacement {
                x: 51,
                baseline: 122,
                wrap_width: 819,
            }
        );
        assert_eq!(
            StoryCaptionPlacement::for_display(640, 480),
            StoryCaptionPlacement {
                x: 32,
                baseline: 76,
                wrap_width: 512,
            }
        );
    }

    #[test]
    fn intro_hands_off_to_retail_first_world() {
        assert_eq!(FIRST_WORLD_LEVEL_ID, 13);
    }

    #[test]
    fn intro_actor_model_callback_exposes_wrapping_50_hz_clock_only_when_proven() {
        for entity_type in [9, 13, 17, 26, 47] {
            let vars = intro_actor_anim_vars(entity_type, 0x1_2345);
            assert_eq!(vars.dynamic[0], 0x2345);
            assert!(vars.dynamic[1..].iter().all(|value| *value == 0));
            assert!(vars.registers.iter().all(|value| *value == 0));
        }

        assert_eq!(intro_actor_anim_vars(34, 0x1_2345).dynamic, [0; 64]);
    }

    #[test]
    fn caption_intervals_include_the_authored_end_tick() {
        assert_eq!(active_captions(99).count(), 0);
        assert_eq!(
            active_captions(100).next().unwrap().text,
            INTRO2_CAPTIONS[0].text
        );
        assert_eq!(
            active_captions(300).next().unwrap().text,
            INTRO2_CAPTIONS[0].text
        );
        assert_eq!(active_captions(301).count(), 0);
    }

    #[test]
    fn captions_are_ordered() {
        for pair in INTRO2_CAPTIONS.windows(2) {
            assert!(pair[0].start_millis < pair[0].end_millis);
            assert!(pair[0].end_millis <= pair[1].start_millis);
        }
    }

    #[test]
    fn inclusive_shared_ticks_process_both_records_in_table_order() {
        for (tick, earlier, later) in [(1_100, 2, 3), (1_500, 4, 5), (1_750, 5, 6)] {
            let active = active_captions(tick).collect::<Vec<_>>();
            assert_eq!(
                active,
                vec![&INTRO2_CAPTIONS[earlier], &INTRO2_CAPTIONS[later]]
            );
        }
    }

    #[test]
    fn shared_boundary_clears_then_cues_in_one_table_order_pass() {
        let mut cadence = TextTypewriterCadence::default();
        assert!(cadence.observe_active_line(true, 1_099));

        let mut play_sound = false;
        for caption in active_captions(1_100) {
            let (_, revealing) = caption.revealed_text(1_100);
            if cadence.observe_active_line(revealing, 1_100) {
                play_sound = true;
            }
        }
        // The older 17..22-second line is complete and clears the field. The
        // new 22..25-second line then cues immediately at the same tick, as in
        // the retail trace. Selecting only the first match would return false.
        assert!(play_sound);
    }

    #[test]
    fn final_page_uses_its_traced_eighty_second_origin() {
        assert_eq!(active_captions(3_999).count(), 0);
        assert_eq!(
            active_captions(4_000).next().unwrap().text,
            "You are our last chance"
        );
        assert_eq!(active_captions(4_174).count(), 0);
        assert_eq!(
            active_captions(4_175).next().unwrap().text,
            "End transmission"
        );
        assert_eq!(
            active_captions(4_300).next().unwrap().text,
            "End transmission"
        );
        assert_eq!(active_captions(4_301).count(), 0);
    }

    #[test]
    fn captions_type_on_at_the_authored_thirty_millisecond_rate() {
        let caption = INTRO2_CAPTIONS[0];
        assert_eq!(caption.revealed_text(100), ("", true));
        assert_eq!(caption.revealed_text(101), ("", true));
        assert_eq!(caption.revealed_text(102), (&caption.text[..1], true));
        assert_eq!(caption.revealed_text(103), (&caption.text[..2], true));
        assert_eq!(caption.revealed_text(110), (&caption.text[..6], true));
        assert_eq!(caption.revealed_text(299), (caption.text, false));
    }

    #[test]
    fn render_eligibility_is_independent_of_operation_two_activation() {
        for visible in [2, 31, 33, 34, 35, 44] {
            assert!(intro_actor_visible(visible));
        }
        for anchor in INTRO2_CAMERA_ANCHORS {
            assert!(!intro_actor_visible(*anchor));
        }
    }

    #[test]
    fn intro_red_turret_camera_subjects_keep_authored_pose() {
        // Default-operation records only select these actors as camera
        // subjects. Their later deaths are combat-driven and deliberately not
        // encoded as deterministic Intro2 clock events.
        let post_cut_samples = [
            (52, [88.0, 0.125, -28.0], 54.0),
            (53, [100.0, 0.75, -11.0], 52.0),
            (54, [104.0, 1.125, -21.0], 57.0),
        ];

        for (entity_index, authored_position, elapsed_secs) in post_cut_samples {
            let pose = intro_actor_pose(entity_index, authored_position, elapsed_secs);
            assert_eq!(pose.position, authored_position);
            assert_eq!(intro_actor_heading(entity_index), 0.0);
        }
    }

    #[test]
    fn intro_type77_spawn45_fallback_cannot_lift_or_translate() {
        let base = [40.0, 1.0, -12.0];
        for elapsed_secs in [0.0, 1.0, 5.5, 6.0, 22.0, INTRO2_DURATION_SECS] {
            assert_eq!(
                intro_actor_pose(45, base, elapsed_secs),
                IntroActorPose {
                    model_slot: 0,
                    position: base
                }
            );
            assert_eq!(intro_actor_heading(45), 0.0);
        }
        // Neighbors in the old synthetic flying group still have a clock path
        // if they lack a native owner; spawn 45 must not share it.
        assert_ne!(intro_actor_pose(21, base, 7.0).position, base);
        assert_ne!(intro_actor_pose(46, base, 7.0).position, base);
    }

    #[test]
    fn intro_fallback_cannot_destroy_structures_at_sampled_retail_times() {
        for (spawn, base) in [(36, [-112.0, -1.0, -128.0]), (51, [82.0, -1.25, -23.0])] {
            for elapsed in [
                0.0,
                308.0 / 50.0,
                309.0 / 50.0,
                3479.0 / 50.0,
                70.0,
                INTRO2_DURATION_SECS,
            ] {
                assert_eq!(
                    intro_actor_pose(spawn, base, elapsed),
                    IntroActorPose {
                        model_slot: 0,
                        position: base
                    }
                );
            }
        }
        assert_eq!(
            intro_actor_pose(35, [0.0; 3], INTRO2_DURATION_SECS).model_slot,
            0
        );
        assert_eq!(intro_actor_pose(61, [0.0; 3], 0.0).model_slot, 2);
        assert_eq!(
            intro_actor_pose(61, [0.0; 3], INTRO2_DURATION_SECS).model_slot,
            2
        );
    }

    #[test]
    fn infected_sunflower_keeps_its_captured_authored_world_pose() {
        let base = [-64.0, 0.5, 11.0];
        for elapsed_secs in [
            0.12, 22.0799, 23.10025, 27.75982, 33.01986, 41.51992, 48.10006,
        ] {
            let pose = intro_actor_pose(61, base, elapsed_secs);
            assert_eq!(pose.model_slot, 2);
            assert_eq!(pose.position, base);
        }
        assert_eq!(intro_actor_heading(61), 0.0);

        // The correction is specific to the stationary plant and must not
        // erase the existing proxy motion of its neighboring attacker group.
        assert_ne!(intro_actor_pose(25, base, 27.7598).position, base);
    }

    fn camera_request(
        target_position_raw: [i16; 3],
        target_velocity_raw: [i16; 3],
    ) -> IntroCameraUpdate<'static> {
        IntroCameraUpdate {
            elapsed_micros: CAPTURED_CAMERA_FRAME_MICROS,
            subject: Some(IntroCameraSubject {
                position_raw: target_position_raw,
                velocity_raw: target_velocity_raw,
            }),
            // The capture retained the retail/default Display setting 6.
            // This is independent of FUN_0040ED10's zero caller offset.
            active_camera: 6,
            terrain: None,
        }
    }

    fn camera_at_origin() -> IntroCameraController {
        let mut camera = IntroCameraController::default();
        camera.position_raw = [0; 3];
        camera
    }

    #[test]
    fn intro_camera_starts_from_captured_proxy_and_uses_active_camera_setting() {
        let mut camera = IntroCameraController::default();
        let pose = camera.update(camera_request([-29184, 8, -32256], [0; 3]));

        // 20260719-012555-intro2-camera-spring.jsonl, tick 6: ED10 sees
        // [0,0x200,-0x1100] with Active Camera 6 before FUN_004503C0 performs
        // the first snap. Retail inherits a prior menu spring for this hidden
        // call; the isolated controller initializes to the same raw target.
        assert_eq!(INTRO_CAMERA_INITIAL_POSITION_RAW, [0, 0x200, -0x1100]);
        assert_eq!(pose.eye, [0.0, 2.0, 230.996_1]);
        assert_eq!(pose.distance, 2049.0 / 256.0);
        assert_eq!(pose.focus, [0.0, 2.0, 61433.0 / 256.0]);
        assert_eq!(camera.position_raw, [-29184, 8, -32768]);
        assert_eq!(camera.velocity_raw, [0; 3]);
    }

    #[test]
    fn intro_camera_snaps_only_after_the_retail_boundary() {
        let mut camera = camera_at_origin();
        camera.update(camera_request([0x3200, 0, 0x200], [0; 3]));
        assert_eq!(camera.position_raw, [0; 3]);
        assert_eq!(camera.velocity_raw[0], 10_000 / 6);

        let mut beyond = camera_at_origin();
        beyond.update(camera_request([0x3201, 0, 0x200], [0; 3]));
        assert_eq!(beyond.position_raw, [0x3201, 0, 0]);
        assert_eq!(beyond.velocity_raw, [0; 3]);
    }

    #[test]
    fn intro_camera_integrates_previous_velocity_before_follow_control() {
        let mut camera = camera_at_origin();
        camera.update(camera_request([0x3200, 0, 0x200], [0; 3]));
        let velocity_before = camera.velocity_raw[0];
        camera.update(camera_request([0x3200, 0, 0x200], [0; 3]));
        assert_eq!(
            camera.position_raw[0],
            integrate_position_word(0, velocity_before, CAPTURED_CAMERA_FRAME_MICROS)
        );
    }

    #[test]
    fn intro2_closing_updates_camera_springs_without_advancing_the_proxy() {
        let mut camera = camera_at_origin();
        let request = IntroCameraEyeUpdate {
            elapsed_micros: 40_000,
            active_camera: 6,
            terrain: None,
        };
        let initial = camera.advance_eye(request);
        // 503C0's final active visit can leave a new target and velocity for
        // the next actor pass. 50C00 runs ED10 without performing that pass.
        camera.position_raw[0] = 1_000;
        camera.velocity_raw[0] = 2_000;
        let closing = camera.advance_eye(request);
        assert_eq!(camera.position_raw, [1_000, 0, 0]);
        assert_eq!(camera.velocity_raw, [2_000, 0, 0]);
        assert_ne!(closing.eye, initial.eye);
        assert_ne!(closing.focus, initial.focus);
    }

    #[test]
    fn intro_camera_vertical_follow_writes_position_and_ignores_both_velocities() {
        let mut camera = camera_at_origin();
        camera.velocity_raw[1] = 12_000;
        camera.update(camera_request([0, 60, 0x200], [0, 30_000, 0]));

        // Generic integration runs first, then direct Y correction applies.
        let integrated = integrate_position_word(0, 12_000, CAPTURED_CAMERA_FRAME_MICROS);
        assert_eq!(camera.position_raw[1], integrated + (60 - integrated) / 6);
        assert_eq!(camera.velocity_raw[1], 12_000);
    }

    #[test]
    fn intro_camera_15_second_cut_follows_without_reset() {
        let mut camera = IntroCameraController::default();
        camera.position_raw = [-29184, -386, -32768];
        camera.velocity_raw = [0; 3];

        // Fresh trace samples 3412..3415: target 8 changes first; the proxy
        // retains its prior state and receives these exact next velocities.
        camera.follow_target([-16896, -590, 31744], [0; 3], None);
        assert_eq!(camera.position_raw, [-29184, -420, -32768]);
        assert_eq!(camera.velocity_raw, [1_666, 0, -256]);
        camera.advance_proxy(CAPTURED_CAMERA_FRAME_MICROS);
        assert_eq!(camera.position_raw, [-29172, -420, 32766]);
    }

    #[test]
    fn intro_camera_23_second_cut_snaps_on_wrapped_z_error() {
        let mut camera = IntroCameraController::default();
        camera.position_raw = [-17628, -507, 30737];
        camera.velocity_raw = [0; 3];

        // Fresh trace samples 5012..5015. X remains below the 0x3201 reset
        // threshold; wrapped Z exceeds it and owns the reset.
        camera.follow_target([-15881, 183, 3791], [-29, -55, 266], None);
        assert_eq!(camera.position_raw, [-15881, 183, 3279]);
        assert_eq!(camera.velocity_raw, [0; 3]);
    }

    #[test]
    fn intro_camera_27_5_second_cut_remains_inside_both_snap_limits() {
        let mut camera = IntroCameraController::default();
        camera.position_raw = [-16069, 346, 3497];
        camera.velocity_raw = [211, 0, -67];

        // Fresh trace samples 5913..5918: errors -9871 and +3181 follow
        // smoothly. A non-wrapping absolute-distance check gets this wrong.
        camera.follow_target([-24537, 858, 6788], [-1403, -790, 402], None);
        assert_eq!(camera.position_raw[0], -16069);
        assert_eq!(camera.position_raw[2], 3497);
        assert_ne!(camera.velocity_raw, [0; 3]);
    }

    #[test]
    fn camera_uses_one_follow_callback_per_world_frame_at_the_supplied_duration() {
        let mut camera = camera_at_origin();
        camera.velocity_raw = [1_000, 0, 0];
        camera.update(IntroCameraUpdate {
            elapsed_micros: 40_000,
            subject: Some(IntroCameraSubject {
                position_raw: [1_000, 0, 512],
                velocity_raw: [0; 3],
            }),
            active_camera: 0,
            terrain: None,
        });
        assert_eq!(camera.position_raw, [38, 0, 0]);
        assert_eq!(camera.velocity_raw, [994, 0, 0]);

        let mut short_frame = camera_at_origin();
        short_frame.update(IntroCameraUpdate {
            elapsed_micros: 4_000,
            subject: Some(IntroCameraSubject {
                position_raw: [600, 0, 512],
                velocity_raw: [0; 3],
            }),
            active_camera: 0,
            terrain: None,
        });
        assert_eq!(
            short_frame.velocity_raw,
            [100, 0, 0],
            "a short frame still calls 503C0"
        );
    }

    #[test]
    fn unresolved_subject_skips_only_follow_control() {
        let mut camera = camera_at_origin();
        camera.velocity_raw = [1_000, 0, -1_000];
        camera.update(IntroCameraUpdate {
            elapsed_micros: CAPTURED_CAMERA_FRAME_MICROS,
            subject: None,
            active_camera: 0,
            terrain: None,
        });
        assert_eq!(camera.position_raw, [7, 0, -8]);
        assert_eq!(camera.velocity_raw, [1_000, 0, -1_000]);
    }
}
