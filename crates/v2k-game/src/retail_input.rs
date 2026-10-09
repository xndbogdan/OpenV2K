//! Retail mouse and joystick terms for the craft input reader.
//!
//! `V2000.EXE` reads the mouse through a DirectInput device (`c_dfDIMouse`,
//! relative axes, FGDK device type 1) and every attached joystick through
//! WinMM `joyGetPosEx` (device type 2, polled every 20 ms). The FGDK binding
//! layer picks each device's table by its type from the craft binding set
//! `0x004C4120`, the only one the PC build selects (`FUN_004292A0` returns 0):
//!
//! - slot 1, the mouse: X, Y and wheel feed analog consumers through
//!   `0x004C2DE0`; button 1 (left) drives the Space bindings and button 2
//!   (right) the primary fire;
//! - slot 2, the joystick: X and Y feed analog consumers through
//!   `0x004C2E90`; button 1 fires and button 2 drives the Space bindings.
//!
//! The reader `FUN_004445E0` consumes those values. This module holds its
//! arithmetic; `docs/re/CONTROLS.md` records the evidence. No menu binding set
//! has a mouse or joystick slot, so these terms apply to the craft only.

use v2k_formats::fixed_math::{retail_arcsine_q31, retail_integer_sqrt};
use v2k_render::{ControllerSample, PadSnapshot};

use crate::weapon_inventory::WeaponCycleDirection;

/// Turn or pitch words per mickey. The slot-1 binding scales each mouse axis
/// by `-400` and Q31 one half; the reader adds `previous - current` of X to the
/// turn channel and `current - previous` of Y to the pitch channel.
pub const MOUSE_AXIS_GAIN: i32 = 200;

/// Turn-channel term for one reader call's mouse X motion in mickeys. The
/// reader works on 16-bit words, so very fast motion wraps instead of
/// saturating.
pub fn mouse_turn_raw(dx_mickeys: i32) -> i16 {
    dx_mickeys.wrapping_mul(MOUSE_AXIS_GAIN) as i16
}

/// Pitch-channel term for one reader call's mouse Y motion in mickeys
/// (positive towards the player). Pulling the mouse back elevates the Hover
/// gun and lifts the VTOL nose; pushing it forward does the opposite.
pub fn mouse_pitch_raw(dy_mickeys: i32) -> i16 {
    dy_mickeys.wrapping_mul(-MOUSE_AXIS_GAIN) as i16
}

/// The reader's single weapon step for the wheel motion since its last call.
///
/// The wheel consumer stores the negated wheel total, and the reader steps
/// once when it changed: forward (`0x004441D0`) when the value rose, backward
/// (`0x00444260`) when it fell. Rotating the wheel towards the player
/// (negative DirectInput `lZ`, negative SDL wheel `y`) therefore selects the
/// next weapon. Several notches in one frame still step only once.
pub fn wheel_weapon_step(wheel_away_from_player: i32) -> Option<WeaponCycleDirection> {
    match wheel_away_from_player.signum() {
        -1 => Some(WeaponCycleDirection::Forward),
        1 => Some(WeaponCycleDirection::Backward),
        _ => None,
    }
}

/// WinMM axis value at rest; the type-2 adapter bias (`0x004CA448` row 2)
/// subtracts it from every axis.
pub const JOYSTICK_AXIS_CENTRE: u16 = 0x8000;

/// One WinMM joystick sample: the six `JOYINFOEX` positions (X, Y, Z, R, U, V
/// in `0..=0xFFFF`) and the button mask, bit `n - 1` for button `n`. Retail
/// requests the POV hat but never forwards it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JoystickSample {
    pub axes: [u16; 6],
    pub buttons: u32,
}

impl JoystickSample {
    pub const AT_REST: Self = Self {
        axes: [JOYSTICK_AXIS_CENTRE; 6],
        buttons: 0,
    };

    /// Whether 1-based button `number` is held.
    pub fn button(self, number: u32) -> bool {
        (1..=32).contains(&number) && self.buttons & (1 << (number - 1)) != 0
    }

    /// Button 1 shares the primary-fire descriptor with Enter.
    pub fn fire(self) -> bool {
        self.button(1)
    }

    /// Button 2 shares both Space descriptors (throttle and the positive-thrust
    /// gate).
    pub fn thrust(self) -> bool {
        self.button(2)
    }
}

/// SDL's signed axis as the WinMM position: `-32768..=32767` becomes
/// `0..=0xFFFF`, so left and forward stay low.
fn winmm_position(value: i16) -> u16 {
    (i32::from(value) + 0x8000) as u16
}

impl JoystickSample {
    /// A standard pad in the layout Windows' legacy joystick API gives an
    /// XInput pad: left stick on X/Y, both triggers sharing Z, the right
    /// stick's Y on R and X on U, and buttons A, B, X, Y, left and right
    /// shoulder, Back, Start, left stick and right stick as 1 to 10. Retail
    /// binds only X, Y and buttons 1 and 2, so a pad steers and aims with the
    /// left stick, fires with the south button and thrusts with the east one.
    pub fn from_pad(pad: PadSnapshot) -> Self {
        let triggers = (0x8000 + i32::from(pad.left_trigger) - i32::from(pad.right_trigger))
            .clamp(0, 0xFFFF) as u16;
        let buttons = pad
            .buttons
            .iter()
            .enumerate()
            .filter(|(_, held)| **held)
            .fold(0, |mask, (index, _)| mask | 1 << index);
        Self {
            axes: [
                winmm_position(pad.left_x),
                winmm_position(pad.left_y),
                triggers,
                winmm_position(pad.right_y),
                winmm_position(pad.right_x),
                JOYSTICK_AXIS_CENTRE,
            ],
            buttons,
        }
    }

    /// Any controller sample. Other joysticks keep their own axis and button
    /// order, which is how WinMM numbers them too.
    pub fn from_controller(sample: ControllerSample) -> Self {
        match sample {
            ControllerSample::Pad(pad) => Self::from_pad(pad),
            ControllerSample::Joystick { axes, buttons } => Self {
                axes: axes.map(winmm_position),
                buttons,
            },
        }
    }
}

/// Reader dead zone around the joystick consumers (`0x0044463F..0x00444663`).
pub fn joystick_dead_zone(value: i32) -> i32 {
    if value < -2000 {
        value + 2000
    } else if value < 2000 {
        0
    } else {
        value - 2000
    }
}

/// Joystick X after the adapter bias, the slot-2 binding (Q31 one eighth)
/// and the reader dead zone. Full deflection gives about +/-2096 turn words,
/// close to a held arrow's 2303 at the default sensitivity.
pub fn joystick_turn_raw(sample: JoystickSample) -> i32 {
    joystick_dead_zone((i32::from(sample.axes[0]) - i32::from(JOYSTICK_AXIS_CENTRE)) >> 3)
}

/// Joystick Y, negated by its binding: pushing forward is positive, which
/// depresses the Hover gun and drops the VTOL nose.
pub fn joystick_pitch_raw(sample: JoystickSample) -> i32 {
    joystick_dead_zone((i32::from(JOYSTICK_AXIS_CENTRE) - i32::from(sample.axes[1])) >> 3)
}

/// Settings word `+0x08`. Relative adds the keyboard and joystick terms to
/// the channels; Absolute steers towards the bearing they form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoystickMode {
    Absolute,
    Relative,
}

impl JoystickMode {
    pub fn from_setting(value: u8) -> Self {
        if value == 1 {
            Self::Relative
        } else {
            Self::Absolute
        }
    }
}

/// Inputs `FUN_004445E0` passes to `FUN_00444B90` in Absolute mode: the
/// quantized keyboard terms plus the joystick terms, and settings `+0x28`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbsoluteSteeringRequest {
    pub pitch_sum: i32,
    pub turn_sum: i32,
    /// "Full" Absolute Mode; only flying styles consult it.
    pub full: bool,
}

/// Movement-style classes `FUN_00444B90` dispatches through `0x00444CDC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SteeringStyle {
    /// Styles 0 and 4, such as Hover.
    Ground,
    /// Styles 1 and 3, such as VTOL.
    Flying,
    /// Style 2 ignores the request.
    Special,
}

/// Channel words `FUN_00444B90` produces.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AbsoluteSteering {
    /// Subtracted from the turn channel.
    pub turn_subtrahend: i16,
    /// The pitch channel after the call. Only the Full flying path writes
    /// it; otherwise the channel stays at the reader's zero.
    pub pitch: i16,
}

/// `FUN_00444B90`: steer towards the bearing of `(turn_sum, pitch_sum)`.
///
/// The bearing is `asin(pitch / magnitude)`, mirrored for leftward input.
/// It is a heading word, so zero faces world +X and `0x4000` world +Z: right
/// and ahead in the first world's opening view, whichever way the craft
/// faces now. The turn word is the heading error scaled by twice the
/// magnitude, so the craft swings onto the bearing and holds it.
///
/// Retail saturates the ratio as `(pitch ^ magnitude) | 0x7FFFFFFF`, which is
/// -1 rather than the most negative ratio when the input points straight
/// back. Such input therefore aims at bearing 0, or `-0x8000` with any
/// leftward component, instead of `-0x4000`.
///
/// With Full set, a flying craft whose bearing lies more than a quarter turn
/// off its heading turns more gently (not at all when the bearing is dead
/// astern) and negates the magnitude. Full flying also writes the pitch word:
/// nose down by the magnitude towards a bearing ahead, nose up for one
/// astern, relative to `body_pitch_raw`.
pub fn absolute_steering(
    request: AbsoluteSteeringRequest,
    heading_raw: i16,
    body_pitch_raw: i16,
    style: SteeringStyle,
) -> AbsoluteSteering {
    let pitch_sum = request.pitch_sum;
    let turn_sum = request.turn_sum;
    let squared = pitch_sum
        .wrapping_mul(pitch_sum)
        .wrapping_add(turn_sum.wrapping_mul(turn_sum));
    let mut magnitude = retail_integer_sqrt(squared) as i32;

    let ratio = if pitch_sum.wrapping_abs() >= magnitude {
        (pitch_sum ^ magnitude) | 0x7FFF_FFFF
    } else {
        ((i64::from(pitch_sum) << 31) / i64::from(magnitude)) as i32
    };
    let mut bearing = retail_arcsine_q31(ratio);
    if turn_sum < 0 {
        bearing = (-0x8000_i32).wrapping_sub(bearing);
    }
    let offset = (bearing as i16).wrapping_sub(heading_raw);

    let mut output = AbsoluteSteering::default();
    match style {
        SteeringStyle::Special => {}
        SteeringStyle::Ground => {
            output.turn_subtrahend =
                (i32::from(offset).wrapping_mul(magnitude.wrapping_mul(2)) >> 15) as i16;
        }
        SteeringStyle::Flying => {
            let behind = request.full && !(-0x4000..=0x4000).contains(&offset);
            let turn = if behind {
                let limit: i32 = if offset > 0 { 0x7FFF } else { -0x7FFF };
                let away = limit.wrapping_sub(i32::from(offset)) as i16;
                let turn = i32::from(away).wrapping_mul(magnitude.wrapping_mul(2)) >> 15;
                magnitude = magnitude.wrapping_neg();
                turn
            } else {
                i32::from(offset).wrapping_mul(magnitude.wrapping_mul(2)) >> 15
            } as i16;
            output.turn_subtrahend = turn;
            if request.full {
                let lean = 0x2000 - i32::from(turn).abs();
                let target = magnitude
                    .wrapping_mul(2)
                    .wrapping_sub(i32::from(body_pitch_raw));
                let pitch = (target.wrapping_mul(lean) >> 15) as i16;
                output.pitch = pitch.wrapping_add((i32::from(body_pitch_raw) / 4) as i16);
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_terms_follow_the_reader_signs_and_wrap_as_words() {
        assert_eq!(mouse_turn_raw(1), 200);
        assert_eq!(mouse_turn_raw(-3), -600);
        assert_eq!(mouse_pitch_raw(1), -200);
        assert_eq!(mouse_pitch_raw(-2), 400);
        // 164 mickeys in one reader call exceed a signed word: retail wraps.
        assert_eq!(mouse_turn_raw(164), (164 * 200 - 65_536) as i16);
        assert_eq!(mouse_turn_raw(i32::MAX), i32::MAX.wrapping_mul(200) as i16);
    }

    #[test]
    fn wheel_towards_the_player_selects_the_next_weapon_once() {
        assert_eq!(wheel_weapon_step(-3), Some(WeaponCycleDirection::Forward));
        assert_eq!(wheel_weapon_step(1), Some(WeaponCycleDirection::Backward));
        assert_eq!(wheel_weapon_step(0), None);
    }

    #[test]
    fn joystick_axes_pass_the_adapter_binding_and_dead_zone() {
        let at = |x: u16, y: u16| JoystickSample {
            axes: [x, y, 0x8000, 0x8000, 0x8000, 0x8000],
            buttons: 0,
        };
        assert_eq!(joystick_turn_raw(JoystickSample::AT_REST), 0);
        assert_eq!(joystick_pitch_raw(JoystickSample::AT_REST), 0);
        // Full right/left: (raw - 0x8000) >> 3 = 4095 / -4096, less 2000.
        assert_eq!(joystick_turn_raw(at(0xFFFF, 0x8000)), 2095);
        assert_eq!(joystick_turn_raw(at(0, 0x8000)), -2096);
        // Forward (WinMM Y towards zero) is positive pitch.
        assert_eq!(joystick_pitch_raw(at(0x8000, 0)), 2096);
        assert_eq!(joystick_pitch_raw(at(0x8000, 0xFFFF)), -2096);
        // Exactly +/-2000 after scaling lands inside the dead zone.
        assert_eq!(joystick_dead_zone(2000), 0);
        assert_eq!(joystick_dead_zone(-2000), 0);
        assert_eq!(joystick_dead_zone(-2001), -1);
        assert_eq!(joystick_dead_zone(2001), 1);
        // Arithmetic shifts floor negative values.
        assert_eq!((0x8000 - 0x8001_i32) >> 3, -1);
    }

    #[test]
    fn pads_report_the_windows_legacy_numbering() {
        let mut pad = PadSnapshot {
            left_x: i16::MAX,
            left_y: i16::MIN,
            left_trigger: i16::MAX,
            ..PadSnapshot::default()
        };
        pad.buttons[0] = true;
        pad.buttons[1] = true;
        let sample = JoystickSample::from_pad(pad);
        assert_eq!(sample.axes[0], 0xFFFF);
        // SDL's stick-up is WinMM's forward, which the binding makes positive.
        assert_eq!(sample.axes[1], 0);
        assert_eq!(joystick_pitch_raw(sample), 2096);
        assert_eq!(sample.axes[2], 0xFFFF);
        assert!(sample.fire());
        assert!(sample.thrust());
        assert!(!sample.button(3));

        let idle = JoystickSample::from_pad(PadSnapshot::default());
        assert_eq!(idle, JoystickSample::AT_REST);
        assert_eq!(joystick_turn_raw(idle), 0);

        let mut shoulders = PadSnapshot::default();
        shoulders.buttons[4] = true;
        shoulders.buttons[9] = true;
        let shoulders = JoystickSample::from_pad(shoulders);
        assert!(shoulders.button(5) && shoulders.button(10));
        assert!(!shoulders.fire() && !shoulders.thrust());
    }

    #[test]
    fn other_joysticks_keep_their_own_numbering() {
        let sample = JoystickSample::from_controller(ControllerSample::Joystick {
            axes: [i16::MIN, 0, 0, 0, 0, i16::MAX],
            buttons: 0b10,
        });
        assert_eq!(sample.axes, [0, 0x8000, 0x8000, 0x8000, 0x8000, 0xFFFF]);
        assert!(!sample.fire());
        assert!(sample.thrust());
        assert_eq!(joystick_turn_raw(sample), -2096);
    }

    #[test]
    fn joystick_mode_follows_the_settings_word() {
        assert_eq!(JoystickMode::from_setting(1), JoystickMode::Relative);
        assert_eq!(JoystickMode::from_setting(0), JoystickMode::Absolute);
    }

    #[test]
    fn absolute_ground_steering_turns_towards_the_bearing() {
        let request = AbsoluteSteeringRequest {
            pitch_sum: 0,
            turn_sum: 2303,
            full: false,
        };
        // Facing forward (0x4000) with input to the right (bearing 0): the
        // offset is -0x4000, so the turn channel gains the magnitude.
        let steering = absolute_steering(request, 0x4000, 0, SteeringStyle::Ground);
        assert_eq!(steering.turn_subtrahend, -2303);
        assert_eq!(steering.pitch, 0);
        // Already facing the bearing: no turn.
        let aligned = absolute_steering(request, 0, 0, SteeringStyle::Ground);
        assert_eq!(aligned.turn_subtrahend, 0);
        // Leftward input mirrors the bearing to the other half-turn.
        let left = absolute_steering(
            AbsoluteSteeringRequest {
                turn_sum: -2303,
                ..request
            },
            -0x8000,
            0,
            SteeringStyle::Ground,
        );
        assert_eq!(left.turn_subtrahend, 0);
        // Style 2 ignores the request.
        assert_eq!(
            absolute_steering(request, 0x4000, 0, SteeringStyle::Special),
            AbsoluteSteering::default()
        );
    }

    #[test]
    fn absolute_straight_back_keeps_the_retail_saturation_quirk() {
        let back = AbsoluteSteeringRequest {
            pitch_sum: -2303,
            turn_sum: 0,
            full: false,
        };
        // The saturated ratio is -1, so the bearing is 0 rather than -0x4000:
        // a craft facing backwards turns towards +X.
        let facing_back = absolute_steering(back, -0x4000, 0, SteeringStyle::Ground);
        assert_eq!(facing_back.turn_subtrahend, 2303);
        assert_eq!(
            absolute_steering(back, 0, 0, SteeringStyle::Ground).turn_subtrahend,
            0
        );
        // A small leftward component mirrors that bearing to -0x8000.
        let back_left = AbsoluteSteeringRequest {
            turn_sum: -10,
            ..back
        };
        assert_eq!(
            absolute_steering(back_left, -0x8000, 0, SteeringStyle::Ground).turn_subtrahend,
            0
        );
        // Off the saturated band the ratio divides normally: about -0x4000.
        let mostly_back = AbsoluteSteeringRequest {
            turn_sum: 300,
            ..back
        };
        let aimed = absolute_steering(mostly_back, -0x4000, 0, SteeringStyle::Ground);
        assert!(aimed.turn_subtrahend.abs() < 400, "{aimed:?}");
    }

    #[test]
    fn absolute_flying_full_reverses_for_targets_behind() {
        // Pure forward input: bearing asin(saturated) = table[1023] = 15923.
        let request = AbsoluteSteeringRequest {
            pitch_sum: 2303,
            turn_sum: 0,
            full: true,
        };
        // Facing backward (-0x4000): the bearing is astern, so the turn eases
        // off towards dead astern and the negated magnitude lifts the nose.
        let behind = absolute_steering(request, -0x4000, 0, SteeringStyle::Flying);
        let offset = (15_923_i16).wrapping_sub(-0x4000);
        let away = (0x7FFF - i32::from(offset)) as i16;
        let turn = ((i32::from(away) * (2 * 2303)) >> 15) as i16;
        assert_eq!(behind.turn_subtrahend, turn);
        let lean = 0x2000 - i32::from(turn).abs();
        assert_eq!(behind.pitch, (((-2 * 2303) * lean) >> 15) as i16);
        // Half mode never takes the reverse branch and leaves pitch at zero.
        let half = absolute_steering(
            AbsoluteSteeringRequest {
                full: false,
                ..request
            },
            -0x4000,
            0,
            SteeringStyle::Flying,
        );
        assert_eq!(half.pitch, 0);
        assert_eq!(
            half.turn_subtrahend,
            ((i32::from(offset) * (2 * 2303)) >> 15) as i16
        );
    }

    #[test]
    fn absolute_flying_full_adds_a_quarter_of_body_pitch() {
        let request = AbsoluteSteeringRequest {
            pitch_sum: 0,
            turn_sum: 1000,
            full: true,
        };
        let steering = absolute_steering(request, 0, -7, SteeringStyle::Flying);
        assert_eq!(steering.turn_subtrahend, 0);
        // (2 * 1000 + 7) * 0x2000 >> 15, plus trunc(-7 / 4) = -1.
        assert_eq!(steering.pitch, (((2 * 1000 + 7) * 0x2000) >> 15) as i16 - 1);
    }
}
