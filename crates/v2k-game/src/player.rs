//! Player craft control state + hover/fly integrators (V2000 scheme).
//!
//! Ground truth: `docs/re/PLAYER_CRAFT.md` §2/§3/§5 and `CONTROLS.md`.
//! The craft has one shared input reader (`FUN_004445E0`) that collapses the
//! held keys into a five-channel motion vector (`S+0x280`); the two integrators
//! (`FUN_00444CF0` hover, `FUN_00445310` fly) interpret those channels
//! differently. Bindings are identical in both modes — only the integrator
//! differs — so TAB is a pure movement-style swap (hover style 0 ↔ fly style 1
//! for the player = body 0), not an animated transition.
//!
//! Angles are the engine's 16-bit domain (full circle = 0x10000). Hover yaw,
//! velocity, and position now pass through the recovered wrapping fixed-point
//! formulas using the native microsecond delta. The gun barrel joint remains in
//! angle units. VTOL attitude is still stored in renderer radians, but its
//! barrel decay, manual pitch, normal-player Self Righting recurrence, and
//! type-46 yaw/bank step round trip through the executable's native signed-angle
//! domain every frame.

use std::collections::HashSet;

use sdl2::keyboard::Keycode;
use v2k_formats::models::AnimVars;
use v2k_formats::terrain::TerrainGrid;
use v2k_render::orientation_from_ypr;

use crate::common_mover::type9_attitude::{
    plan_type9_terrain_attitude_raw, Type9TerrainAttitudeInput,
    TERRAIN_ATTITUDE_CLAMP_EFFECTIVE_FLAG, TERRAIN_ATTITUDE_EFFECTIVE_FLAG,
};
use crate::entity::Entity;
use crate::hover::{
    keyboard_pitch_raw, keyboard_turn_raw, steer_heading_raw, steering_step_raw, HoverBasis,
    HoverFrameForces, HoverPhysicsConfig, RETAIL_DEFAULT_SENSITIVITY, RETAIL_FRAME_DELTA_MAX_US,
};
use crate::retail_input::{
    absolute_steering, joystick_pitch_raw, joystick_turn_raw, mouse_pitch_raw, mouse_turn_raw,
    AbsoluteSteeringRequest, JoystickMode, JoystickSample, SteeringStyle,
};
use crate::vtol::{VehicleFrameForces, VtolControlFrame};

// ── Tick / angle conversion ─────────────────────────────────────────────────

/// Tick rate of `DAT_004FED60`. The engine advances this clock once per
/// 20,000 microseconds, so authored per-tick control deltas are 50 Hz, not the
/// port's former assumed 60 Hz.
#[cfg(test)]
const TICK_HZ: f32 = 50.0;
/// Full 16-bit angle circle.
const ANGLE_FULL: f32 = 0x10000 as f32;

/// Convert a 16-bit-domain angle to radians.
fn angle_to_rad(a: f32) -> f32 {
    a / ANGLE_FULL * std::f32::consts::TAU
}

// ── Reader channel scales (PLAYER_CRAFT.md §2 / CONTROLS.md keymap) ──────────

/// UP/DOWN pitch-channel scale (0xD80 = 3456), the Absolute-Mode default path.
#[cfg(test)]
const PITCH_SCALE_ARROW: f32 = 0xD80 as f32;
/// S/X finer pitch-channel scale (0x900 = 2304); the S/X term also clamps ±0x900.
const PITCH_SCALE_SX: f32 = 0x900 as f32;
/// S/X per-frame clamp on the finer term.
const PITCH_SX_CLAMP: f32 = 0x900 as f32;

// ── Gun barrel joint (hover) — gunpitch_decomp.c:481-489 ─────────────────────

/// Barrel elevation clamp low end (−0x800 = −11.25°).
const GUN_BARREL_MIN: i16 = -0x800;
/// Barrel elevation clamp high end (+0x3000 = +67.5°).
const GUN_BARREL_MAX: i16 = 0x3000;

// ── VTOL propulsion feel (Hover uses authored fixed-point Section-12 data) ───

// ── Player fuel domain ─────────────────────────────────────────────────────────

/// Full tank in controller +0x88's integer runtime domain.
pub const FUEL_FULL_RAW: i32 = 200_000;
/// Retail accepts another fuel pickup only while the tank is below this value.
pub const FUEL_PICKUP_ACCEPT_BELOW_RAW: i32 = 190_001;
/// The powered VTOL callback warns only strictly below this raw level.
pub const FUEL_LOW_WARNING_BELOW_RAW: i32 = 10_000;
/// `FUN_00445310` requires an absolute tick difference strictly greater than
/// 0x46 before replacing the global low-fuel warning slot.
pub const FUEL_LOW_WARNING_TICK_GAP: i32 = 0x46;

/// Controller-side result of applying a type-0x33 fuel pickup.
///
/// The Section-10 static-contact system owns world contact and shared terrain
/// removal. HUD text and sound remain separate presentation concerns; this
/// controller operation only applies the recovered fuel-domain policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuelPickupOutcome {
    Collected { added_raw: i32, fuel_raw: i32 },
    TankFull { fuel_raw: i32 },
}

/// Process-global cadence state behind `DAT_004DE854`'s low-fuel warning.
///
/// Retail does not reset this word at level entry. Keeping it outside the
/// per-spawn [`PlayerCraft`] state preserves that lifetime while leaving the
/// actual HUD and audio presentation with the caller.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FuelWarningCadence {
    last_tick: i32,
}

impl FuelWarningCadence {
    /// Consume the exact post-burn VTOL warning condition.
    pub fn poll(&mut self, frame: VehicleFrameForces, fuel_raw: i32, tick: i32) -> bool {
        if !frame.vtol_started_with_fuel() || fuel_raw >= FUEL_LOW_WARNING_BELOW_RAW {
            return false;
        }

        let delta = self.last_tick.wrapping_sub(tick);
        let sign = delta >> 31;
        let absolute_delta = (delta ^ sign).wrapping_sub(sign);
        if absolute_delta <= FUEL_LOW_WARNING_TICK_GAP {
            return false;
        }

        self.last_tick = tick;
        true
    }

    #[cfg(test)]
    fn last_tick(self) -> i32 {
        self.last_tick
    }
}

// ── Shared fan drive (§4 — FUN_00420A50 / FUN_00420920) ────────────────────

/// Persistent upper fan layer (`sound_047.wav`) in the global Section-11 pool.
pub const PLAYER_FAN_UPPER_SOUND_ID: usize = 47;
/// Persistent lower fan layer (`sound_031.wav`) in the global Section-11 pool.
pub const PLAYER_FAN_LOWER_SOUND_ID: usize = 31;

const FAN_RPM_IDLE_Q16: i32 = 0x1_0000;
const FAN_RPM_EXCESS_MAX_Q16: i32 = 0x8000;
const FAN_GAIN_IDLE_Q16: i32 = 0x2000;
const FAN_GAIN_POWERED_Q16: i32 = 0x1_0000;
const FAN_GAIN_RELEASE_THRESHOLD_Q16: i32 = 0x4000;
const FAN_VISUAL_IDLE_RATE_Q16: i32 = 0x6000;

/// Per-frame parameters submitted to the two persistent positional fan loops.
/// Values remain in the executable's signed 16.16 domain until the audio edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerFanSoundFrame {
    pub upper_rate_q16: i32,
    /// Creation path uses the full envelope before steady updates switch to
    /// the divided gain below.
    pub upper_start_gain_q16: i32,
    pub upper_update_gain_q16: i32,
    pub lower_rate_q16: i32,
    pub lower_gain_q16: i32,
}

/// Live owner inputs for the player's shared `FUN_0040E640` terrain/water
/// attitude phase.
///
/// This phase is deliberately separate from [`PlayerCraft::integrate_hover`]:
/// retail first applies Hover forces through the retained body basis, rebuilds
/// the current basis, and only then corrects pitch/roll against the surface.
#[derive(Clone, Copy)]
pub struct PlayerHoverAttitudeRequest<'a> {
    pub terrain: &'a TerrainGrid,
    pub player: &'a Entity,
    /// Selected model-header `+0x08` extent, before retail halves it.
    pub active_model_extent_raw: u16,
    pub attached_cargo_mass: u32,
    pub water_enabled: bool,
    pub retail_tick: u32,
    pub elapsed_micros: u32,
}

// ── Authored PLAYER4 mode joints (Sub-O / FUN_00420A50) ─────────────────────

/// Hover writes one and VTOL writes zero to the two Sub-O mode targets. The
/// target bit is expanded to a full u16 before the retail fixed-point easing
/// step. These are animation callback indices 5 and 6, not model-slot selectors.
const MODE_JOINT_DEPLOYED_TARGET: u16 = u16::MAX;

/// V2000 movement style. Player = body 0: hover style 0 ↔ fly style 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleMode {
    Hover,
    Vtol,
}

/// Result of one edge-triggered TAB request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleModeToggleOutcome {
    Changed(VehicleMode),
    /// Hover -> VTOL was refused because controller fuel `+0x88` was empty.
    RefusedNoFuel,
}

impl VehicleMode {
    pub fn label(self) -> &'static str {
        match self {
            VehicleMode::Hover => "Hover",
            VehicleMode::Vtol => "VTOL",
        }
    }
}

/// Per-frame input channels collapsed from the held keys, mouse and joystick,
/// mirroring the reader `FUN_004445E0` motion vector `S+0x280`. `pitch` is in
/// the engine's 16-bit angle domain (arrow scale 0xD80 + S/X scale 0x900);
/// `turn` remains a normalized held magnitude while throttle retains its
/// native signed-Q16 channel value.
#[derive(Debug, Clone, Copy, Default)]
pub struct MotionChannels {
    /// `+0x00` turn: LEFT(−1) / RIGHT(+1).
    pub turn: f32,
    /// `+0x02` pitch: UP(+0xD80)/DOWN(−0xD80) + S(+0x900)/X(−0x900), plus the
    /// mouse and Relative joystick terms, wrapped to one signed word. One
    /// shared channel — the integrator decides whether it depresses/elevates
    /// the barrel or tilts the body. Retail's hover convention is
    /// intentionally inverted from a menu cursor: positive (UP/S) depresses
    /// the gun.
    pub pitch: f32,
    /// `+0x04` vertical. Only console pads bind it, so it stays 0 on the PC.
    pub vertical: f32,
    /// `+0x08` throttle: SPACE(+0x10000) / RSHIFT(−0x10000), retained in the
    /// executable's signed-Q16 domain so transition values need not round-trip
    /// through normalized floats.
    pub throttle_q16: i32,
    /// `+0x0C` fire: Enter or joystick button 1 (mode-independent). Right
    /// mouse joins this held channel at the window/gameplay layer.
    pub fire: bool,
    /// Whether the separately clamped S/X fine-pitch term participated in this
    /// input sample. VTOL's powered attitude coupling is suppressed by either
    /// fine-pitch key, while hover continues to consume the combined pitch.
    pub fine_pitch_active: bool,
    /// Whether the dedicated positive-thrust binding is held. Retail uses this
    /// binding—not the collapsed signed throttle—to gate powered pitch
    /// correction, so SPACE+RSHIFT can have zero net lift while retaining the
    /// SPACE-side correction.
    pub positive_thrust_binding_active: bool,
    /// Turn words added after the keyboard term: mouse X and, in Relative
    /// mode, joystick X.
    pub analog_turn_raw: i16,
    /// Joystick Absolute mode: the keyboard and joystick sums that
    /// `FUN_00444B90` steers by. The keyboard turn, arrow pitch and joystick
    /// terms are then absent from the other channels.
    pub absolute: Option<AbsoluteSteeringRequest>,
}

/// Mouse and joystick state for one reader call.
#[derive(Debug, Clone, Copy, Default)]
pub struct AnalogInput {
    /// Mouse motion since the previous reader call, in mickeys (positive
    /// right and towards the player).
    pub mouse_dx: i32,
    pub mouse_dy: i32,
    /// Mouse button 1 (left), which shares Space's bindings.
    pub mouse_thrust: bool,
    /// The first attached controller, if any.
    pub joystick: Option<JoystickSample>,
}

/// The control settings the reader consults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReaderSettings {
    /// Settings `+0x2C`, 0..=15.
    pub sensitivity: u8,
    /// Settings `+0x08`.
    pub joystick_mode: JoystickMode,
    /// Settings `+0x28` ("Absolute Mode" enabled, called Full here).
    pub full_absolute: bool,
}

impl ReaderSettings {
    /// The executable's static defaults: Relative, Half, sensitivity 10.
    pub const DEFAULT: Self = Self::relative(RETAIL_DEFAULT_SENSITIVITY);

    pub const fn relative(sensitivity: u8) -> Self {
        Self {
            sensitivity,
            joystick_mode: JoystickMode::Relative,
            full_absolute: false,
        }
    }
}

/// Turn and pitch words as the reader leaves them for one integrator call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReaderWords {
    turn: i16,
    pitch: i16,
}

impl MotionChannels {
    /// Build the channel vector using the retail default sensitivity and one
    /// nominal 20 ms input interval.
    pub fn from_keys(keys: &HashSet<Keycode>) -> Self {
        Self::from_keys_with_sensitivity(keys, 20_000, RETAIL_DEFAULT_SENSITIVITY)
    }

    /// Build the channel vector from the current key-down set using the live
    /// Relative-mode sensitivity and input interval.
    pub fn from_keys_with_sensitivity(
        keys: &HashSet<Keycode>,
        elapsed_micros: u32,
        sensitivity: u8,
    ) -> Self {
        Self::from_input(
            keys,
            AnalogInput::default(),
            elapsed_micros,
            ReaderSettings::relative(sensitivity),
        )
    }

    /// Build the channel vector from the keys, mouse and joystick, in the
    /// reader's order: joystick and keyboard terms (directly in Relative
    /// mode, through `FUN_00444B90` in Absolute mode), then mouse Y, S/X and
    /// mouse X. Every addition wraps as a signed word, as in retail.
    pub fn from_input(
        keys: &HashSet<Keycode>,
        analog: AnalogInput,
        elapsed_micros: u32,
        settings: ReaderSettings,
    ) -> Self {
        let held = |k: Keycode| keys.contains(&k);
        let sensitivity = settings.sensitivity;

        // Pitch: sensitivity/duty-cycle-scaled arrow term (authored full scale
        // 0xD80) plus the separately clamped fine S/X term (±0x900).
        let mut arrow_pitch = 0.0;
        if held(Keycode::Up) {
            arrow_pitch += 1.0;
        }
        if held(Keycode::Down) {
            arrow_pitch -= 1.0;
        }
        let arrow_pitch_raw =
            i32::from(keyboard_pitch_raw(arrow_pitch, elapsed_micros, sensitivity));
        let mut sx = 0.0;
        if held(Keycode::S) {
            sx += PITCH_SCALE_SX;
        }
        if held(Keycode::X) {
            sx -= PITCH_SCALE_SX;
        }
        let sx_raw = sx.clamp(-PITCH_SX_CLAMP, PITCH_SX_CLAMP) as i32;

        let mut turn = 0.0;
        if held(Keycode::Left) || held(Keycode::Comma) {
            turn -= 1.0;
        }
        if held(Keycode::Right) || held(Keycode::Period) {
            turn += 1.0;
        }

        let joystick = analog.joystick;
        let joystick_turn = joystick.map_or(0, joystick_turn_raw);
        let joystick_pitch = joystick.map_or(0, joystick_pitch_raw);
        let mouse_turn = i32::from(mouse_turn_raw(analog.mouse_dx));
        let mouse_pitch = i32::from(mouse_pitch_raw(analog.mouse_dy));

        let (pitch_raw, analog_turn_raw, absolute) = match settings.joystick_mode {
            JoystickMode::Relative => (
                arrow_pitch_raw + joystick_pitch + mouse_pitch + sx_raw,
                joystick_turn + mouse_turn,
                None,
            ),
            JoystickMode::Absolute => (
                mouse_pitch + sx_raw,
                mouse_turn,
                Some(AbsoluteSteeringRequest {
                    pitch_sum: arrow_pitch_raw + joystick_pitch,
                    turn_sum: i32::from(keyboard_turn_raw(turn, elapsed_micros, sensitivity))
                        + joystick_turn,
                    full: settings.full_absolute,
                }),
            ),
        };

        // Space, mouse button 1 and joystick button 2 share both thrust
        // descriptors; any of them holds the channel.
        let thrust = held(Keycode::Space)
            || analog.mouse_thrust
            || joystick.is_some_and(JoystickSample::thrust);
        let mut throttle_q16 = 0i32;
        if thrust {
            throttle_q16 += 0x1_0000;
        }
        if held(Keycode::RShift) {
            throttle_q16 -= 0x1_0000;
        }

        let fire = held(Keycode::Return) || joystick.is_some_and(JoystickSample::fire);

        Self {
            turn,
            pitch: f32::from(pitch_raw as i16),
            vertical: 0.0,
            throttle_q16,
            fire,
            fine_pitch_active: held(Keycode::S) || held(Keycode::X),
            positive_thrust_binding_active: thrust,
            analog_turn_raw: analog_turn_raw as i16,
            absolute,
        }
    }

    /// The reader's turn and pitch words for a craft with this heading and
    /// body pitch. Relative mode adds the keyboard turn term; Absolute mode
    /// replaces it with `FUN_00444B90`, which also sets pitch for a Full
    /// flying craft.
    fn reader_words(
        &self,
        elapsed_micros: u32,
        sensitivity: u8,
        heading_raw: i16,
        body_pitch_raw: i16,
        style: SteeringStyle,
    ) -> ReaderWords {
        let pitch = self.pitch as i16;
        match self.absolute {
            None => ReaderWords {
                turn: keyboard_turn_raw(self.turn, elapsed_micros, sensitivity)
                    .wrapping_add(self.analog_turn_raw),
                pitch,
            },
            Some(request) => {
                let steering = absolute_steering(request, heading_raw, body_pitch_raw, style);
                ReaderWords {
                    turn: self.analog_turn_raw.wrapping_sub(steering.turn_subtrahend),
                    pitch: steering.pitch.wrapping_add(pitch),
                }
            }
        }
    }

    fn throttle_normalized(self) -> f32 {
        self.throttle_q16 as f32 / 65_536.0
    }
}

/// Full player craft control state (the port's stand-in for the controller
/// struct reached via `g_rng_state+0x27c`).
#[derive(Debug, Clone)]
pub struct PlayerCraft {
    /// Movement style (TAB swaps hover ↔ fly).
    pub mode: VehicleMode,
    /// Gun barrel joint elevation (16-bit angle domain; clamp per §3 hover).
    /// Both integrators keep it a whole signed word; it is stored as f32 for
    /// the renderer and diagnostics.
    pub gun_barrel: f32,
    /// Body pitch (radians) — nose-down positive; drives forward flight in fly.
    pub body_pitch: f32,
    /// Body roll (radians) — bank-to-turn lean in fly.
    pub body_roll: f32,
    /// Fan spin accumulator (radians, wrapped).
    pub spin_angle: f32,
    /// Sub-O +0x20: shared visual/global-47 RPM state.
    fan_rpm_state_q16: i32,
    /// Sub-O +0x24: shared gain envelope for globals 47 and 31.
    fan_gain_envelope_q16: i32,
    /// Type-46 Sub-O animation callback words 5 and 6. Hover eases both toward
    /// 0xFFFF (deployed gear / horizontal fan); VTOL eases both toward zero.
    mode_joint_5: u16,
    mode_joint_6: u16,
    /// Fuel in controller +0x88's exact integer 0..200000 domain. A newly
    /// created player starts empty; type-0x33 pickup integration fills it.
    pub fuel_raw: i32,
    /// Callback word 4 is the authored weapon/loadout selector. Vehicle mode
    /// never writes it; zero retains the default pair of `pl4gatgun` children.
    weapon_selector: i32,
    /// Weapon-component callback words 2 and 3. A successful shot pulses only
    /// the selected side to `0xFFFF`; the weapon scheduler owns the decay.
    primary_joint_pulses: [u16; 2],
    /// Decoded type-46 Section-12 A/B/C/D values.
    hover_physics: HoverPhysicsConfig,
    /// Persistent target created by `FUN_00420450` once per craft spawn.
    hover_target_speed_raw: i32,
}

impl Default for PlayerCraft {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerCraft {
    pub fn new() -> Self {
        let physics = HoverPhysicsConfig::default();
        Self::with_hover_target(physics, i32::from(physics.target_speed_base_raw))
    }

    /// Bind the drive target already produced by the entity's 20450 Sub-A
    /// constructor. Loading must not draw a second, independent random word.
    pub fn with_hover_target(
        hover_physics: HoverPhysicsConfig,
        hover_target_speed_raw: i32,
    ) -> Self {
        Self {
            mode: VehicleMode::Hover,
            gun_barrel: 0.0,
            body_pitch: 0.0,
            body_roll: 0.0,
            spin_angle: 0.0,
            fan_rpm_state_q16: 0,
            fan_gain_envelope_q16: 0,
            mode_joint_5: 0,
            mode_joint_6: 0,
            fuel_raw: 0,
            weapon_selector: 0,
            primary_joint_pulses: [0; 2],
            hover_physics,
            hover_target_speed_raw,
        }
    }

    /// Rebuild the world-local type-46 component while retaining the fields
    /// owned by the persistent campaign controller.
    ///
    /// Secret-world captures retain the same controller pointer, movement
    /// style, and exact fuel value across each handoff, but allocate a new
    /// player entity at a zero-velocity authored pose. Consequently animation,
    /// body attitude, fan envelopes, and the randomized drive target belong to
    /// the new component; only controller mode/fuel cross this boundary here.
    /// Weapon selection is rebound separately from the session inventory by the
    /// main load transaction.
    pub fn campaign_world_replacement(
        &self,
        hover_physics: HoverPhysicsConfig,
        hover_target_speed_raw: i32,
    ) -> Self {
        let mut replacement = Self::with_hover_target(hover_physics, hover_target_speed_raw);
        replacement.mode = self.mode;
        replacement.fuel_raw = self.fuel_raw;
        replacement
    }

    pub fn hover_physics(&self) -> HoverPhysicsConfig {
        self.hover_physics
    }

    pub fn hover_target_speed_raw(&self) -> i32 {
        self.hover_target_speed_raw
    }

    /// Child model names that carry the spinning fan.
    ///
    /// **Blades only** (`pl4engine`). User ground truth (2026-07-03): only the
    /// fan BLADES rotate, not the ring/shroud (`pl4enginesurround`). The blades
    /// are nested *inside* the surround's own instance list, so the renderer
    /// propagates this spin down the sub-model recursion (see
    /// `draw_model_tree`'s spin descriptor in `main.rs`).
    pub fn fan_spin_names() -> &'static [&'static str] {
        &["pl4engine"]
    }

    /// Whether this child is an authored fan blade (`pl4engine`).
    pub fn spins_fan(child_name: Option<&str>) -> bool {
        child_name.is_some_and(|name| Self::fan_spin_names().contains(&name))
    }

    /// Request a mode toggle (TAB).
    ///
    /// **Retail-observed result:** Hover→VTOL with an empty fuel bar leaves the
    /// craft in Hover. Static disassembly closes the mechanism: TAB itself is
    /// unconditional, but the first `FUN_00445310` callback sees fuel `< 1`,
    /// submits event `0x0B` and sound `0x31`, and requests Hover again. Because
    /// that round trip performs no motion, the port represents its net state as
    /// `RefusedNoFuel`; the runtime call site owns the exact feedback. Vtol→Hover
    /// remains unconditional.
    ///
    /// A successful swap changes only the behavior style. The live model joints
    /// retain their current words and ease toward the new mode's targets.
    pub fn toggle_mode(&mut self) -> VehicleModeToggleOutcome {
        let new_mode = match self.mode {
            // Collapse retail's empty-tank VTOL callback and immediate
            // mode-change request into its identical stable Hover state.
            VehicleMode::Hover => {
                if self.fuel_raw > 0 {
                    VehicleMode::Vtol
                } else {
                    return VehicleModeToggleOutcome::RefusedNoFuel;
                }
            }
            // Vtol→Hover always allowed.
            VehicleMode::Vtol => VehicleMode::Hover,
        };
        self.mode = new_mode;
        VehicleModeToggleOutcome::Changed(self.mode)
    }

    /// Advance control state one frame and write attitude/heading into `player`.
    /// Position integration and mode-specific forces stay in
    /// [`crate::entity::EntityManager::update`].
    pub fn integrate(&mut self, dt: f32, ch: &MotionChannels, player: &mut Entity) {
        self.integrate_configured(dt, ch, player, 1);
    }

    /// Integrate using the menu's 0-15 Self Righting mode. Zero disables the
    /// flight correction, one damps toward level, and 2..=15 seek progressively
    /// steeper nose-down attitudes using the retail integer recurrence.
    pub fn integrate_configured(
        &mut self,
        dt: f32,
        ch: &MotionChannels,
        player: &mut Entity,
        self_righting: u8,
    ) {
        let elapsed_micros = (dt.max(0.0) * 1_000_000.0).round() as u32;
        self.integrate_configured_micros(elapsed_micros, ch, player, self_righting);
    }

    /// Same integration using the executable's native microsecond delta. The
    /// main loop uses this entry point so signed shifts and integer divisions
    /// do not inherit a float round trip.
    pub fn integrate_configured_micros(
        &mut self,
        elapsed_micros: u32,
        ch: &MotionChannels,
        player: &mut Entity,
        self_righting: u8,
    ) {
        let _ = self.integrate_configured_micros_with_sensitivity(
            elapsed_micros,
            ch,
            player,
            self_righting,
            RETAIL_DEFAULT_SENSITIVITY,
        );
    }

    /// Retail-microsecond integration with the live 0..15 menu sensitivity.
    /// Returns the retained previous-frame basis and drive inputs so the entity
    /// callback can apply the mode-specific force phase in executable order.
    pub fn integrate_configured_micros_with_sensitivity(
        &mut self,
        elapsed_micros: u32,
        ch: &MotionChannels,
        player: &mut Entity,
        self_righting: u8,
        sensitivity: u8,
    ) -> VehicleFrameForces {
        let elapsed_micros = elapsed_micros.min(RETAIL_FRAME_DELTA_MAX_US);
        let frame_forces = match self.mode {
            VehicleMode::Hover => VehicleFrameForces::Hover(self.integrate_hover(
                elapsed_micros,
                ch,
                player,
                sensitivity,
            )),
            VehicleMode::Vtol => VehicleFrameForces::Vtol(self.integrate_fly(
                elapsed_micros,
                ch,
                player,
                self_righting.min(15),
                sensitivity,
            )),
        };

        // FUN_00420A50 snapshots the prior RPM excess for the visual joint,
        // then FUN_00420920 advances the one state shared by animation and the
        // two persistent sound layers. It tests signed throttle only for zero:
        // Space and Right Shift are identical, while holding both cancels.
        let prior_rpm_excess = (self.fan_rpm_state_q16 - FAN_RPM_IDLE_Q16).max(0);
        self.advance_fan_drive(elapsed_micros, ch.throttle_q16 != 0);
        let spin_step = fan_visual_step_raw(prior_rpm_excess, elapsed_micros);
        let spin_raw = radians_to_angle_word(self.spin_angle).wrapping_add(spin_step);
        self.spin_angle = angle_word_to_rad(spin_raw);

        // FUN_00444CF0 writes targets {1,1} in Hover; FUN_00445310 writes
        // {0,0} in VTOL. FUN_00444F60 then calls FUN_00420A50, whose signed
        // Q31 recurrence updates callback words 5 and 6 independently.
        let deployed = self.mode == VehicleMode::Hover;
        self.mode_joint_5 = step_mode_joint(self.mode_joint_5, deployed, elapsed_micros);
        self.mode_joint_6 = step_mode_joint(self.mode_joint_6, deployed, elapsed_micros);
        frame_forces
    }

    /// Current fixed-point requests for the two retail fan loops.
    pub fn fan_sound_frame(&self) -> PlayerFanSoundFrame {
        PlayerFanSoundFrame {
            upper_rate_q16: self.fan_rpm_state_q16 * 3 / 2,
            upper_start_gain_q16: self.fan_gain_envelope_q16,
            upper_update_gain_q16: self.fan_gain_envelope_q16 / 6,
            lower_rate_q16: 0x8000,
            lower_gain_q16: self.fan_gain_envelope_q16 / 2 + 0x2000,
        }
    }

    fn advance_fan_drive(&mut self, elapsed_micros: u32, powered: bool) {
        // FUN_00420920 first divides the native delta by 16. Its subsequent
        // signed shifts reduce to the exact integer steps below for the
        // positive, frame-capped gameplay delta.
        let quantum = (elapsed_micros >> 4) as i32;
        let mut rpm_excess = (self.fan_rpm_state_q16 - FAN_RPM_IDLE_Q16).max(0);
        if powered {
            rpm_excess = (rpm_excess + (quantum << 2)).min(FAN_RPM_EXCESS_MAX_Q16);
            self.fan_gain_envelope_q16 = FAN_GAIN_POWERED_Q16;
        } else {
            let gain_step = if self.fan_gain_envelope_q16 <= FAN_GAIN_RELEASE_THRESHOLD_Q16 {
                quantum >> 2
            } else {
                quantum << 1
            };
            self.fan_gain_envelope_q16 =
                (self.fan_gain_envelope_q16 - gain_step).max(FAN_GAIN_IDLE_Q16);
            rpm_excess = (rpm_excess - (quantum << 1)).max(0);
        }
        self.fan_rpm_state_q16 = FAN_RPM_IDLE_Q16 + rpm_excess;
    }

    /// EC60 may change the retained angles after the last F70 writer. Force
    /// callbacks read that earlier matrix, including across a mode switch.
    fn previous_force_basis(&self, player: &Entity) -> HoverBasis {
        match player.physical_body_basis_q31() {
            crate::entity_collision_state::RetailRuntimeValue::Known(basis) => HoverBasis {
                lateral: basis.lateral,
                up: basis.up,
                forward: basis.forward,
            },
            crate::entity_collision_state::RetailRuntimeValue::Unresolved => {
                HoverBasis::from_angle_words(
                    radians_to_angle_word(player.heading),
                    radians_to_angle_word(self.body_pitch),
                    radians_to_angle_word(self.body_roll),
                )
            }
        }
    }

    /// HOVER (`FUN_00444CF0`): pitch channel aims the gun barrel joint (the body
    /// does not pitch); throttle propels forward along the heading; direct yaw.
    fn integrate_hover(
        &mut self,
        elapsed_micros: u32,
        ch: &MotionChannels,
        player: &mut Entity,
        sensitivity: u8,
    ) -> HoverFrameForces {
        // Retail rebuilds the entity matrix only after FUN_00444CF0 returns.
        // Preserve that one-tick lag by capturing the force basis before yaw.
        let previous_basis = self.previous_force_basis(player);
        let heading_raw = radians_to_angle_word(player.heading);
        let words = ch.reader_words(
            elapsed_micros,
            sensitivity,
            heading_raw,
            radians_to_angle_word(self.body_pitch),
            SteeringStyle::Ground,
        );

        // pitch channel → gun barrel, once per callback and not scaled by the
        // frame delta: `*barrel += -(channel / 2)` as a signed word, clamped.
        // Mouse motion therefore aims by the same angle at any frame rate.
        let barrel_step = -(i32::from(words.pitch) / 2) as i16;
        self.gun_barrel = f32::from(
            (self.gun_barrel as i16)
                .wrapping_add(barrel_step)
                .clamp(GUN_BARREL_MIN, GUN_BARREL_MAX),
        );

        // Steering: reader turn word -> Sub-D rate -> wrapping 16-bit heading.
        // Positive/right turns subtract.
        player.heading = angle_word_to_rad(steer_heading_raw(
            heading_raw,
            words.turn,
            elapsed_micros,
            self.hover_physics.steering_divisor_raw,
        ));

        // Vertical inertia belongs to the Sub-C hover controller. Do not clear
        // it here: retained upward velocity is what lets the retail craft glide
        // briefly above a falling wave crest.
        HoverFrameForces::new(
            previous_basis,
            ch.throttle_normalized(),
            self.hover_target_speed_raw,
        )
    }

    /// Apply the player's shared post-force `FUN_0040E640` surface-attitude
    /// phase and retain its corrected angles for basis rebuild.
    ///
    /// Accepted submerged-transition captures prove why this is not part of
    /// [`Self::toggle_mode`]: TAB preserves an extreme VTOL pose, the first
    /// Hover force pass uses that pose, and only the following common phase
    /// clamps pitch/roll to ±`0x1800`. Later frames follow the terrain/wave
    /// gradient through `FUN_0041EC70` instead of scalar easing toward zero.
    pub fn apply_hover_common_terrain_attitude(&mut self, request: PlayerHoverAttitudeRequest<'_>) {
        if self.mode != VehicleMode::Hover {
            return;
        }

        let heading_raw = radians_to_angle_word(request.player.heading);
        let pitch_raw = radians_to_angle_word(self.body_pitch);
        let roll_raw = radians_to_angle_word(self.body_roll);
        let basis = HoverBasis::from_angle_words(heading_raw, pitch_raw, roll_raw);

        // FUN_0040E640 reads the signed Sub-C surface-mode byte but disables
        // it only when FUN_004185C0 reports attached mass above 0x96. This
        // threshold is intentionally distinct from Sub-C lift's 99-unit gate.
        let resolved_surface_mode_raw = i8::from(
            self.hover_physics.lift.use_wave_surface && request.attached_cargo_mass <= 0x96,
        );
        let effective_flags = self.hover_physics.environment_flags_at_type_record_0xc0
            | TERRAIN_ATTITUDE_EFFECTIVE_FLAG
            | TERRAIN_ATTITUDE_CLAMP_EFFECTIVE_FLAG;
        let plan = plan_type9_terrain_attitude_raw(Type9TerrainAttitudeInput {
            terrain: request.terrain,
            position_raw: request.player.position_raw(),
            pitch_raw,
            roll_raw,
            lateral_basis_q31: basis.lateral,
            forward_basis_q31: basis.forward,
            state_flags: request
                .player
                .collision
                .state_flags_at_0x08
                .known_value_bits(),
            effective_flags,
            active_model_extent_raw: request.active_model_extent_raw,
            resolved_surface_mode_raw,
            water_enabled: request.water_enabled,
            wave_tick_50hz: request.retail_tick as i32,
            effective_elapsed_micros: request.elapsed_micros.min(RETAIL_FRAME_DELTA_MAX_US),
        });
        self.body_pitch = angle_word_to_rad(plan.pitch_raw);
        self.body_roll = angle_word_to_rad(plan.roll_raw);
    }

    /// FLY (`FUN_00445310`): barrel locked (decays to 0); pitch channel tilts the
    /// body and bank-to-turn updates attitude. Retained thrust inputs are
    /// returned for the exact entity-side force phase.
    fn integrate_fly(
        &mut self,
        elapsed_micros: u32,
        ch: &MotionChannels,
        player: &mut Entity,
        self_righting: u8,
        sensitivity: u8,
    ) -> VtolControlFrame {
        // Retail rebuilds the body matrix only after FUN_00445310 returns.
        // Preserve the force basis and pitch word that entered the callback.
        // Sub-D changes roll before the near-surface force phase, while Sub-G
        // still projects lift through this previous-frame body basis.
        let previous_basis = self.previous_force_basis(player);
        let pitch_before_raw = radians_to_angle_word(self.body_pitch);
        let roll_before_raw = radians_to_angle_word(self.body_roll);
        let words = ch.reader_words(
            elapsed_micros,
            sensitivity,
            radians_to_angle_word(player.heading),
            pitch_before_raw,
            SteeringStyle::Flying,
        );

        // Type-46 Sub-D byte 4 constructs the shared yaw/roll steering channel.
        // FUN_00445310 enables its roll output for VTOL, so FUN_00420360
        // subtracts the same wrapping step from heading and roll before the
        // near-surface force correction. FUN_0041A690 later applies one
        // signed-word roll damping pass.
        let steering_step = steering_step_raw(
            words.turn,
            elapsed_micros,
            self.hover_physics.steering_divisor_raw,
        );
        player.heading =
            angle_word_to_rad(radians_to_angle_word(player.heading).wrapping_sub(steering_step));
        let roll_before_a690_raw = roll_before_raw.wrapping_sub(steering_step);

        // Sub-O clears precede the locked-barrel phase in retail. The port has
        // no Sub-O runtime yet, but retains the callback order: subtract the
        // truncated native delta and floor the signed joint at zero. A negative
        // hover aim therefore snaps to zero on the first fly frame.
        let barrel_step = (elapsed_micros >> 6) as i16;
        let barrel_raw = (self.gun_barrel as i16).wrapping_sub(barrel_step);
        self.gun_barrel = f32::from(barrel_raw.max(0));

        let has_fuel = self.fuel_raw > 0;

        // An empty callback branches back to hover before FUN_0041A690. Sub-D
        // steering and barrel decay have already happened, but pitch/manual
        // Self Righting and the one roll-damping pass must be skipped.
        if has_fuel {
            self.body_roll = angle_word_to_rad(vtol_roll_after_a690_raw(
                roll_before_a690_raw,
                elapsed_micros,
            ));
        } else {
            self.body_roll = angle_word_to_rad(roll_before_a690_raw);
        }

        // FUN_00445310 writes this signed i16 into Sub-G runtime +0x1E;
        // FUN_0041A690 adds it to body pitch before applying the controller's
        // live Self Righting mode. The normal player callback clears Sub-G
        // +0x1C first, so its separate terrain-derived target phase is inactive
        // in the captured +0x42=1/+0x43=0 path.
        // The input reader supplies integral i16-domain channel values, despite
        // MotionChannels retaining f32 for the still-provisional integrators.
        let throttle_q16 = ch.throttle_q16;
        if has_fuel {
            let effective_pitch_raw = vtol_effective_pitch_raw(
                words.pitch,
                pitch_before_raw,
                ch.positive_thrust_binding_active,
                ch.fine_pitch_active,
                self_righting,
            );
            let pitch_delta = fly_manual_pitch_delta_raw(
                elapsed_micros,
                effective_pitch_raw,
                self.hover_physics.steering_divisor_raw,
            );
            // Body pitch is a wrapping signed angle word. The former ±0x1800
            // clamp was a port invention contradicted by the clean July 16
            // trace, whose UP-only and UP+RSHIFT windows sustain values above
            // +0x4B00. SPACE is a distinct, shallower powered-attitude regime.
            let pitch_after_manual_raw = pitch_before_raw.wrapping_add(pitch_delta);
            self.body_pitch = angle_word_to_rad(vtol_self_right_pitch_raw(
                pitch_after_manual_raw,
                elapsed_micros,
                self_righting,
            ));
        }

        VtolControlFrame {
            previous_basis,
            pitch_before_a690_raw: pitch_before_raw,
            roll_before_a690_raw,
            throttle_q16,
            has_fuel,
        }
    }

    /// Apply the integer-domain fuel drain produced by the VTOL force phase.
    pub fn consume_fuel_raw(&mut self, amount: i32) {
        self.fuel_raw = self.fuel_raw.saturating_sub(amount.max(0)).max(0);
    }

    /// Recovered controller-side stub for a type-0x33 fuel pickup.
    ///
    /// Retail accepts the pickup only below 190001 raw units, adds its authored
    /// value, and caps the tank at 200000. The world pickup/collision callback
    /// can call this once its separate lifecycle is ported.
    pub fn collect_fuel_pickup_raw(&mut self, pickup_value_raw: i32) -> FuelPickupOutcome {
        if self.fuel_raw >= FUEL_PICKUP_ACCEPT_BELOW_RAW {
            return FuelPickupOutcome::TankFull {
                fuel_raw: self.fuel_raw,
            };
        }
        let before = self.fuel_raw.max(0);
        self.fuel_raw = before
            .saturating_add(pickup_value_raw.max(0))
            .min(FUEL_FULL_RAW);
        FuelPickupOutcome::Collected {
            added_raw: self.fuel_raw - before,
            fuel_raw: self.fuel_raw,
        }
    }

    /// Finish the callback-entry fuel policy after entity-side integration.
    /// Retail sets `+0x20D=1` only when this VTOL callback *started* empty; a
    /// tank depleted by the current burn remains VTOL until the next callback.
    pub fn finish_fuel_frame(&mut self, frame: VehicleFrameForces) -> bool {
        if frame.vtol_started_without_fuel() && self.mode == VehicleMode::Vtol {
            self.mode = VehicleMode::Hover;
            return true;
        }
        false
    }

    /// Body-attitude render orientation (yaw from the entity heading, plus the
    /// craft's body pitch/roll). Gun barrel elevation is a separate joint and is
    /// not folded in here.
    pub fn body_angles(&self, heading: f32) -> (f32, f32, f32) {
        (heading, self.body_pitch, self.body_roll)
    }

    /// Current pitch/roll in the executable's wrapping signed angle-word
    /// domain. Optional matched-run diagnostics use these words directly so
    /// their output can be compared with retail controller samples without a
    /// second float-to-fixed conversion policy in the front end.
    pub fn body_angle_words(&self) -> [i16; 2] {
        [
            radians_to_angle_word(self.body_pitch),
            radians_to_angle_word(self.body_roll),
        ]
    }

    /// Restore native443560 or portable-save attitude words without a lossy
    /// intermediate policy. The native caller also rebuilds the entity basis
    /// through413F70 before publishing authored actors.
    pub fn restore_body_angle_words(&mut self, [pitch_raw, roll_raw]: [i16; 2]) {
        self.body_pitch = angle_word_to_rad(pitch_raw);
        self.body_roll = angle_word_to_rad(roll_raw);
    }

    /// Authored player-model local-to-world basis used consistently by draw,
    /// propulsion, muzzle direction, cargo placement, and Section-8 collision.
    /// `FUN_00413F70` stores contiguous lateral/up/forward vectors; transposing
    /// that legacy layout into the renderer's row-major `world = M * local`
    /// convention gives `Ry(PI/2 - heading)`. The captured first-world heading
    /// PI/2 therefore produces the retail identity body basis.
    pub fn model_orientation(&self, heading: f32) -> [[f32; 3]; 3] {
        orientation_from_ypr(
            std::f32::consts::FRAC_PI_2 - heading,
            self.body_pitch,
            self.body_roll,
        )
    }

    /// Current lateral/up/forward body vectors in the executable's signed-Q31
    /// domain.
    ///
    /// Presentation can use [`Self::model_orientation`], but simulation paths
    /// such as `FUN_00446640` consume the matrix words built by
    /// `FUN_00413F70`. That builder narrows every intermediate Q15 product, so
    /// converting the renderer's float matrix back to Q31 is not bit-exact.
    pub fn retail_body_basis_q31(&self, heading: f32) -> [[i32; 3]; 3] {
        let basis = HoverBasis::from_angle_words(
            radians_to_angle_word(heading),
            radians_to_angle_word(self.body_pitch),
            radians_to_angle_word(self.body_roll),
        );
        [basis.lateral, basis.up, basis.forward]
    }

    /// Bit-faithful local-to-world orientation for authored collision models.
    ///
    /// Retail stores lateral/up/forward as Q31 columns. Collision geometry uses
    /// those narrowed words directly, so this transposes them into the row-major
    /// `world = M * local` convention without rebuilding the attitude in float.
    pub fn collision_model_orientation(&self, heading: f32) -> [[f64; 3]; 3] {
        const Q31_SCALE: f64 = 2_147_483_648.0;
        let columns = self.retail_body_basis_q31(heading);
        std::array::from_fn(|world_axis| {
            std::array::from_fn(|local_axis| f64::from(columns[local_axis][world_axis]) / Q31_SCALE)
        })
    }

    /// Current body-forward vector retained as a convenience for projectile
    /// and diagnostics consumers that do not need the other two axes.
    pub fn retail_body_forward_q31(&self, heading: f32) -> [i32; 3] {
        self.retail_body_basis_q31(heading)[2]
    }

    /// Signed gun-barrel angle in radians (positive elevates, negative depresses).
    pub fn gun_barrel_rad(&self) -> f32 {
        angle_to_rad(self.gun_barrel)
    }

    /// Retail default-primary direction in the port's world axes.
    ///
    /// `FUN_00424650` combines the entity body's up/forward basis columns as
    /// `up*sin(barrel) + forward*cos(barrel)`. Applying the shared retail body
    /// matrix to local `[0, sin(barrel), cos(barrel)]` keeps the projectile
    /// aligned with the visible craft in Hover and VTOL. Launch position is
    /// owned separately by411400's draw-stamped transient or centre fallback.
    pub fn primary_fire_direction(&self, heading: f32) -> [f32; 3] {
        let body = self.model_orientation(heading);
        let barrel = self.gun_barrel_rad();
        let local_up = barrel.sin();
        let local_forward = barrel.cos();
        std::array::from_fn(|row| body[row][1] * local_up + body[row][2] * local_forward)
    }

    /// Build the live component [`AnimVars`]. Callback word 4 selects the
    /// equipped weapon; words 2 and 3 retain the alternating side-gun pulse;
    /// Sub-O words 5 and 6 carry the retained mode transition into PLAYER4's
    /// authored lerps, conditional geometry, and mount bases. Presentation
    /// supplies the independent global clock in callback word 0 (40D320).
    pub fn anim_vars(&self) -> AnimVars {
        let mut v = AnimVars::default();
        //40A950 selector1 aliases Sub-E's barrel joint. PLAYER4's authored
        //op5C consumes this WORD before each side-gun instance.
        v.dynamic[1] = i32::from(self.gun_barrel as i16 as u16);
        v.dynamic[2] = i32::from(self.primary_joint_pulses[0]);
        v.dynamic[3] = i32::from(self.primary_joint_pulses[1]);
        v.dynamic[4] = self.weapon_selector;
        v.dynamic[5] = i32::from(self.mode_joint_5);
        v.dynamic[6] = i32::from(self.mode_joint_6);
        v
    }

    /// Current authored mode-joint words, exposed for fidelity diagnostics.
    pub fn mode_joint_words(&self) -> [u16; 2] {
        [self.mode_joint_5, self.mode_joint_6]
    }

    /// Current callback-word-4 weapon/loadout selector.
    pub fn weapon_selector(&self) -> i32 {
        self.weapon_selector
    }

    /// Publish the weapon component's retained `+0x00/+0x04` joint words into
    /// the player hierarchy's callback words 2 and 3.
    pub fn set_primary_joint_pulses(&mut self, pulses: [u16; 2]) {
        self.primary_joint_pulses = pulses;
    }

    /// Install descriptor +0x08's callback selector for the equipped weapon.
    /// This is independent of the inventory's public weapon id: the captured
    /// selector-2 descriptor drives PLAYER4 callback word 4 with value 4.
    pub fn select_weapon_callback(&mut self, selector: u8) {
        self.weapon_selector = i32::from(selector);
    }
}

/// One visual fan-joint step from `FUN_00420A50`.
///
/// The callback uses the RPM excess captured before the current sound update,
/// adds the authored idle rate, multiplies by the native microsecond delta in
/// Q13, then takes the signed product's Q31 result.
fn fan_visual_step_raw(prior_rpm_excess_q16: i32, elapsed_micros: u32) -> i16 {
    let rate_q16 = prior_rpm_excess_q16 + FAN_VISUAL_IDLE_RATE_Q16;
    let product = i64::from(rate_q16) * (i64::from(elapsed_micros) << 13);
    (product >> 31) as i16
}

/// One `FUN_00420A50` Sub-O mode-joint update in the executable's integer
/// domain. `FUN_00444CF0` supplies a one-bit Hover target and `FUN_00445310` a
/// zero VTOL target; the callback expands that bit to 0xFFFF and applies the
/// signed Q31 product below. The final u16 add intentionally wraps, matching the
/// halfword write through the model callback pointer.
fn step_mode_joint(current: u16, deployed: bool, elapsed_micros: u32) -> u16 {
    let target = if deployed {
        i32::from(MODE_JOINT_DEPLOYED_TARGET)
    } else {
        0
    };
    let delta = target - i32::from(current);
    let step = (i64::from(delta) * (i64::from(elapsed_micros) << 12)) >> 31;
    current.wrapping_add(step as u16)
}

/// `FUN_00445310` adds a powered pitch/self-right term unless the S/X fine
/// channel is active. SPACE gates it; reverse thrust deliberately does not.
fn vtol_effective_pitch_raw(
    pitch_raw: i16,
    pitch_before_raw: i16,
    positive_thrust_binding_active: bool,
    fine_pitch_active: bool,
    self_righting: u8,
) -> i16 {
    let powered_coupling =
        if positive_thrust_binding_active && !fine_pitch_active && self_righting > 1 {
            (-i32::from(pitch_before_raw) / 6).clamp(-0x600, 0x600)
        } else {
            0
        };
    (i32::from(pitch_raw) + powered_coupling) as i16
}

/// Normal-player pitch branch of `FUN_0041A690` in the executable's signed
/// angle-word domain. `FUN_00448450` copies the menu's 0..15 Self Righting
/// value to Sub-G runtime byte +0x41. Mode zero leaves pitch alone, mode one
/// damps toward zero, and higher modes seek `(mode - 1) * 0x2800 / 15` with
/// deliberately asymmetric divisors. The step runs after manual pitch even
/// while Up or Down is held.
fn vtol_self_right_pitch_raw(pitch_raw: i16, elapsed_micros: u32, self_righting: u8) -> i16 {
    let mode = i32::from(self_righting.min(15));
    let tick = (elapsed_micros >> 10) as i32;
    match mode {
        0 => pitch_raw,
        1 => {
            let correction = (tick * i32::from(pitch_raw)) >> 9;
            pitch_raw.wrapping_sub(correction as i16)
        }
        _ => {
            let target = ((mode - 1) * 0x2800) / 15;
            let error = target - i32::from(pitch_raw);
            let correction = if error < 1 {
                let divisor = (((1 - mode) * 0x100) / 15) + 0x200;
                (tick * error) / divisor
            } else {
                let divisor = (((mode - 1) * 0x200) / 15) + 0x200;
                let mut correction = (error * tick) / divisor;
                if error > 0x4000 {
                    correction += ((error - 0x4000) * tick) / divisor;
                }
                correction
            };
            pitch_raw.wrapping_add(correction as i16)
        }
    }
}

/// The normal-player roll tail of `FUN_0041A690`. Sub-D has already applied the
/// shared steering step; A690 damps that signed word exactly once.
fn vtol_roll_after_a690_raw(roll_raw: i16, elapsed_micros: u32) -> i16 {
    let correction = (((elapsed_micros >> 10) as i32 * i32::from(roll_raw)) >> 8) as i16;
    roll_raw.wrapping_sub(correction)
}

fn radians_to_angle_word(radians: f32) -> i16 {
    ((radians / std::f32::consts::TAU * ANGLE_FULL).round() as i32) as i16
}

fn angle_word_to_rad(angle: i16) -> f32 {
    f32::from(angle) / ANGLE_FULL * std::f32::consts::TAU
}

/// `FUN_00445310`'s independently recovered manual body-pitch contribution.
/// The authored type-46 Sub-D dword is 28 in every difficulty overlay.
fn fly_manual_pitch_delta_raw(elapsed_micros: u32, pitch_raw: i16, divisor_raw: i32) -> i16 {
    debug_assert!(divisor_raw > 0);
    if divisor_raw <= 0 {
        return 0;
    }

    let elapsed_rate = elapsed_micros.wrapping_mul(0x0d) / divisor_raw as u32;
    ((elapsed_rate as i32).wrapping_mul(i32::from(pitch_raw)) >> 15) as i16
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

    fn keys(list: &[Keycode]) -> HashSet<Keycode> {
        list.iter().copied().collect()
    }

    fn fueled_craft() -> PlayerCraft {
        let mut craft = PlayerCraft::new();
        craft.fuel_raw = FUEL_FULL_RAW;
        craft
    }

    #[test]
    fn portable_body_attitude_round_trips_exact_angle_words() {
        let mut craft = PlayerCraft::new();
        craft.restore_body_angle_words([-0x1234, 0x2345]);
        assert_eq!(craft.body_angle_words(), [-0x1234, 0x2345]);
    }

    #[test]
    fn both_force_callbacks_retain_f70_basis_after_environment_angle_changes() {
        let basis = crate::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(
            0x3210, -0x1234, 0x2345,
        );
        for mode in [VehicleMode::Hover, VehicleMode::Vtol] {
            let mut craft = fueled_craft();
            craft.mode = mode;
            craft.restore_body_angle_words([0x2000, -0x1000]);
            let mut player = dummy_entity();
            player.physical_body_basis_q31 =
                crate::entity_collision_state::RetailRuntimeValue::Known(basis);
            let frame = craft.integrate_configured_micros_with_sensitivity(
                20_000,
                &MotionChannels::default(),
                &mut player,
                0,
                RETAIL_DEFAULT_SENSITIVITY,
            );
            let previous = match frame {
                VehicleFrameForces::Hover(frame) => frame.basis,
                VehicleFrameForces::Vtol(frame) => frame.previous_basis,
                VehicleFrameForces::DyingCommon => {
                    unreachable!("control integrator cannot return Dying")
                }
            };
            assert_eq!(
                [previous.lateral, previous.up, previous.forward],
                [basis.lateral, basis.up, basis.forward]
            );
            assert_ne!(
                craft.retail_body_basis_q31(player.heading),
                [basis.lateral, basis.up, basis.forward]
            );
        }
    }

    fn flat_terrain(height: i8, sea_y_raw: i16) -> TerrainGrid {
        TerrainGrid {
            header: [i32::from(sea_y_raw) << 8, 0, 0, 0, 0],
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

    #[test]
    fn pitch_channel_sums_arrow_and_sx() {
        // Default sensitivity 10 scales UP to 0x8FF; S remains the separate
        // 0x900 fine term.
        let ch = MotionChannels::from_keys(&keys(&[Keycode::Up, Keycode::S]));
        assert_eq!(ch.pitch, 0x08ff as f32 + PITCH_SCALE_SX);
        assert!(ch.fine_pitch_active);
        // DOWN + X → negative.
        let ch = MotionChannels::from_keys(&keys(&[Keycode::Down, Keycode::X]));
        assert_eq!(ch.pitch, -(0x08ff as f32 + PITCH_SCALE_SX));
        assert!(ch.fine_pitch_active);
    }

    #[test]
    fn live_pitch_sensitivity_matches_keyed_capture() {
        let up = MotionChannels::from_keys_with_sensitivity(
            &keys(&[Keycode::Up]),
            20_000,
            RETAIL_DEFAULT_SENSITIVITY,
        );
        let down = MotionChannels::from_keys_with_sensitivity(
            &keys(&[Keycode::Down]),
            20_000,
            RETAIL_DEFAULT_SENSITIVITY,
        );
        assert_eq!(up.pitch, 2303.0);
        assert_eq!(down.pitch, -2303.0);
        assert!(!up.fine_pitch_active);
        assert!(!down.fine_pitch_active);
    }

    #[test]
    fn turn_and_throttle_keep_normalized_and_q16_domains_explicit() {
        let ch = MotionChannels::from_keys(&keys(&[Keycode::Right, Keycode::Space]));
        assert_eq!(ch.turn, 1.0);
        assert_eq!(ch.throttle_q16, 65_536);
        assert_eq!(ch.throttle_normalized(), 1.0);
        let ch = MotionChannels::from_keys(&keys(&[Keycode::Left, Keycode::RShift]));
        assert_eq!(ch.turn, -1.0);
        assert_eq!(ch.throttle_q16, -65_536);
        assert_eq!(ch.throttle_normalized(), -1.0);

        let alternate = MotionChannels::from_keys(&keys(&[Keycode::Comma]));
        assert_eq!(alternate.turn, -1.0);
        let alternate = MotionChannels::from_keys(&keys(&[Keycode::Period]));
        assert_eq!(alternate.turn, 1.0);
    }

    #[test]
    fn reverse_thrust_spins_the_fan_forward_at_the_forward_thrust_rate() {
        let step = |mode: VehicleMode, pressed: &[Keycode]| {
            let mut craft = fueled_craft();
            if mode == VehicleMode::Vtol {
                assert_eq!(
                    craft.toggle_mode(),
                    VehicleModeToggleOutcome::Changed(VehicleMode::Vtol)
                );
            }
            let mut player = dummy_entity();
            let channels = MotionChannels::from_keys(&keys(pressed));
            // The visible joint consumes the prior RPM state, so the first
            // powered frame still advances at idle and the second exposes the
            // recovered Space/Shift envelope.
            for _ in 0..2 {
                craft.integrate_configured_micros(20_000, &channels, &mut player, 1);
            }
            craft.spin_angle
        };

        for mode in [VehicleMode::Hover, VehicleMode::Vtol] {
            let idle = step(mode, &[]);
            let forward = step(mode, &[Keycode::Space]);
            let reverse = step(mode, &[Keycode::RShift]);
            let cancelled = step(mode, &[Keycode::Space, Keycode::RShift]);

            assert_eq!(reverse, forward);
            assert!(reverse > idle);
            assert_eq!(cancelled, idle);
        }
    }

    #[test]
    fn fan_drive_replays_retail_fixed_point_attack_release_and_mix() {
        let mut craft = fueled_craft();
        let mut player = dummy_entity();
        let idle = MotionChannels::default();
        craft.integrate_configured_micros(20_000, &idle, &mut player, 1);
        assert_eq!(
            craft.fan_sound_frame(),
            PlayerFanSoundFrame {
                upper_rate_q16: 0x1_8000,
                upper_start_gain_q16: 0x2000,
                upper_update_gain_q16: 0x2000 / 6,
                lower_rate_q16: 0x8000,
                lower_gain_q16: 0x3000,
            }
        );

        let powered = MotionChannels::from_keys(&keys(&[Keycode::Space]));
        craft.integrate_configured_micros(20_000, &powered, &mut player, 1);
        assert_eq!(craft.fan_rpm_state_q16, 0x1_0000 + 5_000);
        assert_eq!(craft.fan_gain_envelope_q16, 0x1_0000);
        assert_eq!(craft.fan_sound_frame().upper_update_gain_q16, 0x1_0000 / 6);
        assert_eq!(craft.fan_sound_frame().lower_gain_q16, 0xA000);

        for _ in 0..6 {
            craft.integrate_configured_micros(20_000, &powered, &mut player, 1);
        }
        assert_eq!(craft.fan_rpm_state_q16, 0x1_8000);
        assert_eq!(craft.fan_sound_frame().upper_rate_q16, 0x2_4000);

        craft.integrate_configured_micros(20_000, &idle, &mut player, 1);
        assert_eq!(craft.fan_rpm_state_q16, 0x1_8000 - 2_500);
        assert_eq!(craft.fan_gain_envelope_q16, 0x1_0000 - 2_500);
    }

    #[test]
    fn fan_gain_release_preserves_the_retail_threshold_and_slow_tail() {
        let mut craft = fueled_craft();
        craft.fan_gain_envelope_q16 = 0x4001;
        craft.advance_fan_drive(8_000, false);
        assert_eq!(craft.fan_gain_envelope_q16, 0x4001 - 1_000);

        craft.fan_gain_envelope_q16 = 0x4000;
        craft.advance_fan_drive(8_000, false);
        assert_eq!(craft.fan_gain_envelope_q16, 0x4000 - 125);
    }

    #[test]
    fn reverse_fan_audio_matches_forward_and_simultaneous_inputs_cancel() {
        let frame_after = |pressed: &[Keycode]| {
            let mut craft = fueled_craft();
            let mut player = dummy_entity();
            craft.integrate_configured_micros(
                8_000,
                &MotionChannels::from_keys(&keys(pressed)),
                &mut player,
                1,
            );
            craft.fan_sound_frame()
        };

        assert_eq!(
            frame_after(&[Keycode::Space]),
            frame_after(&[Keycode::RShift])
        );
        assert_eq!(
            frame_after(&[Keycode::Space, Keycode::RShift]),
            frame_after(&[])
        );
    }

    #[test]
    fn visual_fan_step_uses_prior_rpm_and_authored_idle_offset() {
        assert_eq!(fan_visual_step_raw(0, 20_000), 1_875);
        assert_eq!(fan_visual_step_raw(0x8000, 20_000), 4_375);
    }

    #[test]
    fn retail_fire_key_is_enter_not_the_old_ctrl_placeholder() {
        assert!(MotionChannels::from_keys(&keys(&[Keycode::Return])).fire);
        assert!(!MotionChannels::from_keys(&keys(&[Keycode::LCtrl])).fire);
    }

    #[test]
    fn primary_direction_matches_drawn_nose_and_barrel_elevation() {
        let mut craft = PlayerCraft::new();
        let level = craft.primary_fire_direction(std::f32::consts::FRAC_PI_2);
        assert!(level[0].abs() < 1.0e-6);
        assert!(level[1].abs() < 1.0e-6);
        assert!((level[2] - 1.0).abs() < 1.0e-6);

        let right_turn = craft.primary_fire_direction(0.0);
        assert!((right_turn[0] - 1.0).abs() < 1.0e-6);
        assert!(right_turn[1].abs() < 1.0e-6);
        assert!(right_turn[2].abs() < 1.0e-6);

        craft.gun_barrel = 0x2000 as f32;
        let raised = craft.primary_fire_direction(std::f32::consts::FRAC_PI_2);
        assert!(raised[1] > 0.0);
        assert!((raised.iter().map(|value| value * value).sum::<f32>() - 1.0).abs() < 1.0e-5);
    }

    #[test]
    fn exposed_body_forward_retains_exact_retail_q31_narrowing() {
        let mut craft = PlayerCraft::new();
        craft.body_pitch = angle_word_to_rad(68);
        craft.body_roll = angle_word_to_rad(702);
        let heading = angle_word_to_rad(37_125u16 as i16);

        // July 16's keyed skimmer capture pins these exact matrix words. The
        // public accessor exists for simulation consumers, not as a float
        // round-trip through the renderer orientation.
        assert_eq!(
            craft.retail_body_forward_q31(heading),
            [-1_962_541_056, -13_959_168, -870_973_440]
        );
        assert_eq!(
            PlayerCraft::new().retail_body_forward_q31(std::f32::consts::FRAC_PI_2),
            [0, 0, 0x7ffe_0000]
        );
    }

    #[test]
    fn collision_orientation_transposes_the_exact_retail_q31_body_columns() {
        let mut craft = PlayerCraft::new();
        craft.body_pitch = angle_word_to_rad(68);
        craft.body_roll = angle_word_to_rad(702);
        let heading = angle_word_to_rad(37_125u16 as i16);
        let columns = craft.retail_body_basis_q31(heading);
        let orientation = craft.collision_model_orientation(heading);

        for local_axis in 0..3 {
            for world_axis in 0..3 {
                assert_eq!(
                    (orientation[world_axis][local_axis] * 2_147_483_648.0).round() as i32,
                    columns[local_axis][world_axis]
                );
            }
        }
    }

    #[test]
    fn retail_heading_basis_makes_named_keys_turn_toward_screen_side() {
        let spawn_heading = std::f32::consts::FRAC_PI_2;

        let mut left_craft = PlayerCraft::new();
        let mut left_player = dummy_entity();
        left_player.heading = spawn_heading;
        let left = MotionChannels::from_keys(&keys(&[Keycode::Left]));
        left_craft.integrate_configured_micros(20_000, &left, &mut left_player, 1);
        let left_nose = left_craft.primary_fire_direction(left_player.heading);
        assert!(
            left_nose[0] < 0.0,
            "Left must turn the nose toward screen-left"
        );

        let mut right_craft = PlayerCraft::new();
        let mut right_player = dummy_entity();
        right_player.heading = spawn_heading;
        let right = MotionChannels::from_keys(&keys(&[Keycode::Right]));
        right_craft.integrate_configured_micros(20_000, &right, &mut right_player, 1);
        let right_nose = right_craft.primary_fire_direction(right_player.heading);
        assert!(
            right_nose[0] > 0.0,
            "Right must turn the nose toward screen-right"
        );
    }

    #[test]
    fn retail_hover_aim_keys_drive_shot_vertical_direction() {
        let mut up_craft = PlayerCraft::new();
        let mut up_player = dummy_entity();
        let up = MotionChannels::from_keys(&keys(&[Keycode::Up]));
        up_craft.integrate(1.0 / TICK_HZ, &up, &mut up_player);
        assert_eq!(up_craft.gun_barrel, -1151.0);
        assert!(up_craft.gun_barrel < 0.0);
        assert!(up_craft.primary_fire_direction(0.0)[1] < 0.0);

        let mut down_craft = PlayerCraft::new();
        let mut down_player = dummy_entity();
        let down = MotionChannels::from_keys(&keys(&[Keycode::Down]));
        down_craft.integrate(1.0 / TICK_HZ, &down, &mut down_player);
        assert!(down_craft.gun_barrel > 0.0);
        assert!(down_craft.primary_fire_direction(0.0)[1] > 0.0);
    }

    #[test]
    fn hover_pitch_aims_barrel_not_body() {
        let mut craft = PlayerCraft::new();
        let mut player = dummy_entity();
        let ch = MotionChannels {
            pitch: PITCH_SCALE_ARROW,
            ..Default::default()
        };
        // Enough frames to move off zero.
        for _ in 0..8 {
            craft.integrate(1.0 / 60.0, &ch, &mut player);
        }
        // Barrel moved (per `-= channel/2`), body pitch stayed level.
        assert!(craft.gun_barrel < 0.0);
        assert_eq!(craft.gun_barrel, f32::from(GUN_BARREL_MIN));
        assert!(craft.body_pitch.abs() < 1e-4);
    }

    #[test]
    fn barrel_clamps_to_range() {
        let mut craft = PlayerCraft::new();
        craft.gun_barrel = f32::from(GUN_BARREL_MAX) + 5000.0;
        let mut player = dummy_entity();
        let ch = MotionChannels::default();
        craft.integrate(1.0 / 60.0, &ch, &mut player);
        assert_eq!(craft.gun_barrel, f32::from(GUN_BARREL_MAX));
    }

    #[test]
    fn hover_barrel_moves_half_the_pitch_word_per_callback_at_any_delta() {
        let up = MotionChannels::from_keys(&keys(&[Keycode::Up]));
        for elapsed_micros in [4_000, 8_000, 20_000] {
            let mut craft = PlayerCraft::new();
            let mut player = dummy_entity();
            craft.integrate_configured_micros(elapsed_micros, &up, &mut player, 1);
            // 2303 / 2 truncates to 1151 whatever the frame length.
            assert_eq!(craft.gun_barrel, -1151.0);
        }

        // Truncation is toward zero for negative words too.
        let mut craft = PlayerCraft::new();
        let mut player = dummy_entity();
        let down = MotionChannels::from_keys(&keys(&[Keycode::Down]));
        craft.integrate_configured_micros(20_000, &down, &mut player, 1);
        assert_eq!(craft.gun_barrel, 1151.0);
    }

    fn analog(mouse_dx: i32, mouse_dy: i32) -> AnalogInput {
        AnalogInput {
            mouse_dx,
            mouse_dy,
            ..AnalogInput::default()
        }
    }

    #[test]
    fn mouse_y_aims_the_hover_gun_and_mouse_x_steers() {
        let none = HashSet::new();
        // Pulling the mouse 10 mickeys back gives pitch -2000: +1000 barrel.
        let back =
            MotionChannels::from_input(&none, analog(0, 10), 20_000, ReaderSettings::DEFAULT);
        assert_eq!(back.pitch, -2000.0);
        let mut craft = PlayerCraft::new();
        let mut player = dummy_entity();
        player.heading = std::f32::consts::FRAC_PI_2;
        craft.integrate_configured_micros(20_000, &back, &mut player, 1);
        assert_eq!(craft.gun_barrel, 1000.0);
        assert_eq!(radians_to_angle_word(player.heading), 0x4000);

        // Five mickeys right is turn word 1000, steered like any other word.
        let right =
            MotionChannels::from_input(&none, analog(5, 0), 20_000, ReaderSettings::DEFAULT);
        assert_eq!(right.analog_turn_raw, 1000);
        let mut craft = PlayerCraft::new();
        let mut player = dummy_entity();
        player.heading = std::f32::consts::FRAC_PI_2;
        craft.integrate_configured_micros(20_000, &right, &mut player, 1);
        let expected = steer_heading_raw(
            0x4000,
            1000,
            20_000,
            craft.hover_physics().steering_divisor_raw,
        );
        assert!(expected < 0x4000);
        assert_eq!(radians_to_angle_word(player.heading), expected);
    }

    #[test]
    fn mouse_and_keys_share_one_wrapping_pitch_word() {
        let up = keys(&[Keycode::Up, Keycode::S]);
        // 2303 + 2304 - 200 * -150 = 34607 wraps to -30929, as retail's word
        // additions do.
        let wrapped =
            MotionChannels::from_input(&up, analog(0, -150), 20_000, ReaderSettings::DEFAULT);
        assert_eq!(wrapped.pitch, f32::from((2303 + 2304 + 30_000) as i16));
        assert!(wrapped.pitch < 0.0);
        assert!(wrapped.fine_pitch_active);
    }

    #[test]
    fn mouse_pitch_drives_vtol_body_pitch_like_the_keys() {
        let none = HashSet::new();
        let mut from_mouse = fueled_craft();
        from_mouse.toggle_mode();
        let mut from_keys = from_mouse.clone();
        let mut mouse_player = dummy_entity();
        let mut key_player = dummy_entity();

        // Eleven mickeys forward is pitch word +2200, which VTOL consumes
        // exactly as it would the same word from the keys.
        let pushed =
            MotionChannels::from_input(&none, analog(0, -11), 20_000, ReaderSettings::DEFAULT);
        assert_eq!(pushed.pitch, 2200.0);
        from_mouse.integrate_configured_micros(20_000, &pushed, &mut mouse_player, 1);
        let keyed = MotionChannels {
            pitch: 2200.0,
            ..Default::default()
        };
        from_keys.integrate_configured_micros(20_000, &keyed, &mut key_player, 1);
        assert_eq!(from_mouse.body_angle_words(), from_keys.body_angle_words());
        assert!(
            from_mouse.body_angle_words()[0] > 0,
            "pushing forward drops the nose"
        );
    }

    #[test]
    fn joystick_terms_join_the_relative_channels() {
        let none = HashSet::new();
        let stick = JoystickSample {
            axes: [0xFFFF, 0, 0x8000, 0x8000, 0x8000, 0x8000],
            buttons: 0b11,
        };
        let input = AnalogInput {
            joystick: Some(stick),
            ..AnalogInput::default()
        };
        let channels = MotionChannels::from_input(&none, input, 20_000, ReaderSettings::DEFAULT);
        assert_eq!(channels.analog_turn_raw, 2095);
        assert_eq!(channels.pitch, 2096.0);
        assert!(channels.fire);
        assert_eq!(channels.throttle_q16, 0x1_0000);
        assert!(channels.positive_thrust_binding_active);
        assert!(channels.absolute.is_none());
    }

    #[test]
    fn mouse_button_one_and_space_hold_one_thrust_channel() {
        let space = keys(&[Keycode::Space]);
        let held = AnalogInput {
            mouse_thrust: true,
            ..AnalogInput::default()
        };
        let both = MotionChannels::from_input(&space, held, 20_000, ReaderSettings::DEFAULT);
        assert_eq!(both.throttle_q16, 0x1_0000);
        let mouse_only =
            MotionChannels::from_input(&HashSet::new(), held, 20_000, ReaderSettings::DEFAULT);
        assert_eq!(mouse_only.throttle_q16, 0x1_0000);
        assert!(mouse_only.positive_thrust_binding_active);
        let braking = MotionChannels::from_input(
            &keys(&[Keycode::RShift]),
            held,
            20_000,
            ReaderSettings::DEFAULT,
        );
        assert_eq!(braking.throttle_q16, 0);
    }

    #[test]
    fn absolute_mode_routes_arrows_through_the_bearing_helper() {
        let settings = ReaderSettings {
            joystick_mode: JoystickMode::Absolute,
            ..ReaderSettings::DEFAULT
        };
        let right = MotionChannels::from_input(
            &keys(&[Keycode::Right]),
            AnalogInput::default(),
            20_000,
            settings,
        );
        assert_eq!(
            right.absolute,
            Some(AbsoluteSteeringRequest {
                pitch_sum: 0,
                turn_sum: 2303,
                full: false,
            })
        );
        // The arrows no longer aim the gun directly.
        let up = MotionChannels::from_input(
            &keys(&[Keycode::Up]),
            AnalogInput::default(),
            20_000,
            settings,
        );
        assert_eq!(up.pitch, 0.0);

        // Facing forward (0x4000), Right steers towards bearing 0: the turn
        // word is the full magnitude and the craft turns right.
        let mut craft = PlayerCraft::new();
        let mut player = dummy_entity();
        player.heading = std::f32::consts::FRAC_PI_2;
        craft.integrate_configured_micros(20_000, &right, &mut player, 1);
        let expected = steer_heading_raw(
            0x4000,
            2303,
            20_000,
            craft.hover_physics().steering_divisor_raw,
        );
        assert_eq!(radians_to_angle_word(player.heading), expected);

        // Once the heading matches the bearing, the same input holds course.
        let mut aligned = dummy_entity();
        aligned.heading = 0.0;
        craft.integrate_configured_micros(20_000, &right, &mut aligned, 1);
        assert_eq!(radians_to_angle_word(aligned.heading), 0);
    }

    #[test]
    fn fly_locks_barrel_and_tilts_body() {
        let mut craft = fueled_craft();
        craft.toggle_mode();
        assert_eq!(craft.mode, VehicleMode::Vtol);
        craft.gun_barrel = 0x2000 as f32;
        let mut player = dummy_entity();
        let ch = MotionChannels {
            pitch: PITCH_SCALE_ARROW,
            ..Default::default()
        };
        for _ in 0..30 {
            craft.integrate(1.0 / 60.0, &ch, &mut player);
        }
        // Barrel decayed toward 0; body pitched.
        assert!(craft.gun_barrel < 0x2000 as f32);
        assert!(craft.body_pitch > 0.0);
    }

    #[test]
    fn fly_barrel_uses_native_microsecond_decay_and_zero_floor() {
        let mut craft = fueled_craft();
        craft.toggle_mode();
        let mut player = dummy_entity();

        craft.gun_barrel = 0x2000 as f32;
        craft.integrate_configured_micros(20_000, &MotionChannels::default(), &mut player, 0);
        assert_eq!(craft.gun_barrel, (0x2000 - 312) as f32);

        // The runtime joint is an i16 even though the provisional port state is
        // stored as f32; narrow before applying the retail word subtraction.
        craft.gun_barrel = 1000.75;
        craft.integrate_configured_micros(0, &MotionChannels::default(), &mut player, 0);
        assert_eq!(craft.gun_barrel, 1000.0);

        craft.gun_barrel = 100.0;
        craft.integrate_configured_micros(20_000, &MotionChannels::default(), &mut player, 0);
        assert_eq!(craft.gun_barrel, 0.0);

        // Retail subtracts then floors; it does not ease a negative hover aim.
        craft.gun_barrel = -1.0;
        craft.integrate_configured_micros(0, &MotionChannels::default(), &mut player, 0);
        assert_eq!(craft.gun_barrel, 0.0);
    }

    #[test]
    fn fly_manual_pitch_matches_retail_signed_shift() {
        assert_eq!(fly_manual_pitch_delta_raw(20_000, 2_303, 28), 652);
        assert_eq!(fly_manual_pitch_delta_raw(20_000, -2_303, 28), -653);

        let mut craft = fueled_craft();
        craft.toggle_mode();
        let mut player = dummy_entity();
        craft.integrate_configured_micros(
            20_000,
            &MotionChannels {
                pitch: 2_303.0,
                ..Default::default()
            },
            &mut player,
            0,
        );
        assert!((craft.body_pitch - angle_to_rad(652.0)).abs() < 1.0e-7);
    }

    #[test]
    fn vtol_powered_pitch_coupling_uses_space_without_replacing_arrow_input() {
        // A positive pre-frame pitch contributes its opposing sixth while
        // SPACE is held, both alone and on top of the UP arrow channel.
        assert_eq!(vtol_effective_pitch_raw(0, 0x600, true, false, 15), -0x100);
        assert_eq!(
            vtol_effective_pitch_raw(2_303, 0x600, true, false, 15),
            2_303 - 0x100
        );

        // Without the dedicated SPACE binding the term is absent. Either S/X
        // fine-pitch key also suppresses it even though its pitch contribution
        // remains in the combined channel supplied as `pitch_raw`.
        assert_eq!(
            vtol_effective_pitch_raw(2_303, 0x600, false, false, 15),
            2_303
        );
        assert_eq!(
            vtol_effective_pitch_raw(2_303, 0x600, true, true, 15),
            2_303
        );
    }

    #[test]
    fn vtol_powered_pitch_requires_self_righting_above_one_and_not_net_throttle() {
        assert_eq!(vtol_effective_pitch_raw(0, 0x600, true, false, 0), 0);
        assert_eq!(vtol_effective_pitch_raw(0, 0x600, true, false, 1), 0);
        assert_eq!(vtol_effective_pitch_raw(0, 0x600, true, false, 2), -0x100);

        let both = MotionChannels::from_keys(&keys(&[Keycode::Space, Keycode::RShift]));
        assert_eq!(both.throttle_q16, 0);
        assert!(both.positive_thrust_binding_active);
        assert_eq!(
            vtol_effective_pitch_raw(
                both.pitch as i16,
                0x600,
                both.positive_thrust_binding_active,
                both.fine_pitch_active,
                15,
            ),
            -0x100
        );
    }

    #[test]
    fn vtol_body_pitch_is_not_clamped_at_the_disproved_0x1800_boundary() {
        let mut craft = fueled_craft();
        craft.toggle_mode();
        craft.body_pitch = angle_word_to_rad(0x4800);
        let mut player = dummy_entity();
        craft.integrate_configured_micros(
            20_000,
            &MotionChannels {
                pitch: 2_303.0,
                ..Default::default()
            },
            &mut player,
            0,
        );
        assert!(radians_to_angle_word(craft.body_pitch) > 0x4800);
    }

    #[test]
    fn submerged_mode_launch_uses_extreme_basis_once_then_common_attitude_clamps() {
        let terrain = flat_terrain(0, -0x34f);

        for (extreme_pitch_raw, clamped_pitch_raw) in [(19_220, 0x1800), (-17_166, -0x1800)] {
            let mut craft = fueled_craft();
            assert_eq!(
                craft.toggle_mode(),
                VehicleModeToggleOutcome::Changed(VehicleMode::Vtol)
            );
            craft.body_pitch = angle_word_to_rad(extreme_pitch_raw);
            craft.body_roll = 0.0;

            let mut player = dummy_entity();
            player.position = [77.0, -4.0, 58.0];
            player.heading = angle_word_to_rad(0x2000);
            player.velocity = [3.0, -2.0, 1.0];
            let velocity_before = player.velocity;
            let extreme_basis = HoverBasis::from_angle_words(0x2000, extreme_pitch_raw, 0);

            assert_eq!(
                craft.toggle_mode(),
                VehicleModeToggleOutcome::Changed(VehicleMode::Hover)
            );
            assert_eq!(
                radians_to_angle_word(craft.body_pitch),
                extreme_pitch_raw,
                "TAB must not clamp the retained VTOL pose"
            );

            let first_frame = craft.integrate_configured_micros_with_sensitivity(
                8_000,
                &MotionChannels::default(),
                &mut player,
                10,
                RETAIL_DEFAULT_SENSITIVITY,
            );
            let VehicleFrameForces::Hover(first_hover) = first_frame else {
                panic!("the first post-TAB frame must use Hover forces");
            };
            assert_eq!(
                first_hover.basis, extreme_basis,
                "the launch force must use the retained extreme basis once"
            );

            craft.apply_hover_common_terrain_attitude(PlayerHoverAttitudeRequest {
                terrain: &terrain,
                player: &player,
                active_model_extent_raw: 280,
                attached_cargo_mass: 0,
                water_enabled: true,
                retail_tick: 2_796,
                elapsed_micros: 8_000,
            });
            assert_eq!(radians_to_angle_word(craft.body_pitch), clamped_pitch_raw);
            assert_eq!(radians_to_angle_word(craft.body_roll), 0);
            assert_eq!(
                player.velocity, velocity_before,
                "the attitude phase must not inject a transition impulse"
            );

            let second_frame = craft.integrate_configured_micros_with_sensitivity(
                8_000,
                &MotionChannels::default(),
                &mut player,
                10,
                RETAIL_DEFAULT_SENSITIVITY,
            );
            let VehicleFrameForces::Hover(second_hover) = second_frame else {
                panic!("the second post-TAB frame must remain Hover");
            };
            assert_eq!(
                second_hover.basis,
                HoverBasis::from_angle_words(0x2000, clamped_pitch_raw, 0),
                "only the following force pass may consume the corrected pose"
            );
        }
    }

    #[test]
    fn self_righting_modes_follow_fun_0041a690_integer_branches() {
        assert_eq!(vtol_self_right_pitch_raw(0x2000, 8_000, 0), 0x2000);
        assert_eq!(vtol_self_right_pitch_raw(0x2000, 8_000, 1), 0x1f90);

        // At the captured 8 ms delta, mode 10's +0x1800 target has a small
        // truncation dead band. Retail repeatedly rests at 6028/6029 in it.
        assert_eq!(vtol_self_right_pitch_raw(6_027, 8_000, 10), 6_028);
        assert_eq!(vtol_self_right_pitch_raw(6_028, 8_000, 10), 6_028);
        assert_eq!(vtol_self_right_pitch_raw(6_029, 8_000, 10), 6_029);

        // Positive errors above 0x4000 receive the executable's second step.
        assert_eq!(vtol_self_right_pitch_raw(-0x4000, 8_000, 10), -16_140);
    }

    #[test]
    fn captured_mode10_powered_pitch_fixed_points_replay_exactly() {
        fn step(pitch: i16, input: i16) -> i16 {
            let effective = vtol_effective_pitch_raw(input, pitch, true, false, 10);
            let manual = fly_manual_pitch_delta_raw(8_000, effective, 28);
            vtol_self_right_pitch_raw(pitch.wrapping_add(manual), 8_000, 10)
        }

        assert_eq!(step(1_854, 0), 1_854, "Space-only equilibrium");
        assert_eq!(step(10_469, 2_303), 10_469, "Space+Up equilibrium");
        assert_eq!(step(-7_572, -2_303), -7_572, "Space+Down equilibrium");
    }

    #[test]
    fn sustained_up_keeps_retail_space_and_reverse_thrust_attitude_regimes_separate() {
        fn sustained_pitch(pressed: &[Keycode], self_righting: u8) -> i16 {
            let mut craft = fueled_craft();
            assert_eq!(
                craft.toggle_mode(),
                VehicleModeToggleOutcome::Changed(VehicleMode::Vtol)
            );
            // The keyed acceleration trace begins each isolated input window
            // from mode-10's neutral dead band.
            craft.body_pitch = angle_word_to_rad(6_029);
            let mut player = dummy_entity();
            let channels = MotionChannels::from_keys_with_sensitivity(
                &keys(pressed),
                8_000,
                RETAIL_DEFAULT_SENSITIVITY,
            );
            for _ in 0..375 {
                craft.integrate_configured_micros_with_sensitivity(
                    8_000,
                    &channels,
                    &mut player,
                    self_righting,
                    RETAIL_DEFAULT_SENSITIVITY,
                );
            }
            radians_to_angle_word(craft.body_pitch)
        }

        // 20260716-220810-vtol-acceleration.jsonl (Self Righting 10) isolates
        // these three sustained windows. SPACE applies the bounded -pitch/6
        // input-reader term and therefore settles much shallower; RSHIFT is
        // signed reverse lift only and deliberately retains the unpowered UP
        // attitude.
        assert_eq!(sustained_pitch(&[Keycode::Up], 10), 19_269);
        assert_eq!(sustained_pitch(&[Keycode::Up, Keycode::Space], 10), 10_469);
        assert_eq!(sustained_pitch(&[Keycode::Up, Keycode::RShift], 10), 19_269);
        assert!(
            HoverBasis::from_angle_words(0, sustained_pitch(&[Keycode::Up, Keycode::Space], 10), 0)
                .up[1]
                > 0,
            "the captured mode-10 powered regime keeps lift's vertical projection upward"
        );

        // The pristine frontend default is materially different. Mode one
        // omits SPACE's opposing-sixth term, so fully held UP+SPACE can pass
        // 90 degrees and project the otherwise-authentic powered lift down.
        // Pin that distinction rather than repairing it with a global pitch
        // clamp: matched comparisons must use the same Self Righting value.
        let mode_one_powered_pitch = sustained_pitch(&[Keycode::Up, Keycode::Space], 1);
        assert!(mode_one_powered_pitch > 0x4000);
        assert!(HoverBasis::from_angle_words(0, mode_one_powered_pitch, 0).up[1] < 0);
        assert!(
            HoverBasis::from_angle_words(0, sustained_pitch(&[Keycode::Up, Keycode::Space], 0), 0,)
                .up[1]
                < 0,
            "the retail-verified mode-zero regime also projects sustained powered lift downward"
        );

        // The later 20260730 submerged-launch captures were made with Self
        // Righting 9. Their UP-only equilibrium is 19_220, not evidence that
        // should overwrite the mode-10 powered trace. Pin the corresponding
        // powered and reverse regimes so both capture families stay coherent.
        assert_eq!(sustained_pitch(&[Keycode::Up], 9), 19_220);
        assert_eq!(sustained_pitch(&[Keycode::Up, Keycode::Space], 9), 9_995);
        assert_eq!(sustained_pitch(&[Keycode::Up, Keycode::RShift], 9), 19_220);
    }

    #[test]
    fn powered_pitch_reaches_the_force_basis_on_the_following_retail_frame() {
        fn two_frames(keys_down: &[Keycode]) -> (i16, HoverBasis) {
            let mut craft = fueled_craft();
            assert_eq!(
                craft.toggle_mode(),
                VehicleModeToggleOutcome::Changed(VehicleMode::Vtol)
            );
            craft.body_pitch = angle_word_to_rad(6_029);
            let mut player = dummy_entity();
            let channels = MotionChannels::from_keys_with_sensitivity(
                &keys(keys_down),
                8_000,
                RETAIL_DEFAULT_SENSITIVITY,
            );

            let first = craft.integrate_configured_micros_with_sensitivity(
                8_000,
                &channels,
                &mut player,
                10,
                RETAIL_DEFAULT_SENSITIVITY,
            );
            let VehicleFrameForces::Vtol(first) = first else {
                panic!("VTOL must emit a VTOL force frame");
            };
            assert_eq!(
                first.previous_basis,
                HoverBasis::from_angle_words(0, 6_029, 0),
                "retail projects force through the callback-entry basis"
            );

            let pitch_after_first = radians_to_angle_word(craft.body_pitch);
            let second = craft.integrate_configured_micros_with_sensitivity(
                8_000,
                &channels,
                &mut player,
                10,
                RETAIL_DEFAULT_SENSITIVITY,
            );
            let VehicleFrameForces::Vtol(second) = second else {
                panic!("VTOL must emit a VTOL force frame");
            };
            (pitch_after_first, second.previous_basis)
        }

        let (up_pitch, up_basis) = two_frames(&[Keycode::Up]);
        let (space_pitch, space_basis) = two_frames(&[Keycode::Up, Keycode::Space]);
        assert!(
            space_pitch < up_pitch,
            "SPACE's opposing sixth must reduce nose-down attitude immediately"
        );
        assert_eq!(
            up_basis,
            HoverBasis::from_angle_words(0, up_pitch, 0),
            "the next force phase must observe the UP-only attitude"
        );
        assert_eq!(
            space_basis,
            HoverBasis::from_angle_words(0, space_pitch, 0),
            "the next force phase must observe the SPACE-reduced attitude"
        );
        assert!(
            space_basis.up[1] > up_basis.up[1],
            "the shallower powered pose must retain more vertical lift"
        );
    }

    #[test]
    fn captured_vtol_steering_replays_sub_d_then_one_a690_roll_damp() {
        let mut craft = fueled_craft();
        craft.toggle_mode();
        craft.body_roll = angle_word_to_rad(603);
        let mut player = dummy_entity();
        player.heading = angle_word_to_rad(4_250);

        let frame = craft.integrate_configured_micros_with_sensitivity(
            8_000,
            &MotionChannels {
                turn: -1.0,
                ..Default::default()
            },
            &mut player,
            10,
            RETAIL_DEFAULT_SENSITIVITY,
        );
        let VehicleFrameForces::Vtol(frame) = frame else {
            panic!("VTOL mode must return a VTOL force frame");
        };

        // Retail trace samples 4062 -> 4063: -2303 produces a -262 shared
        // steering step, then A690 damps post-steer roll 865 to 842.
        assert_eq!(
            frame.previous_basis,
            HoverBasis::from_angle_words(4_250, 0, 603)
        );
        assert_eq!(radians_to_angle_word(player.heading), 4_512);
        assert_eq!(frame.roll_before_a690_raw, 865);
        assert_eq!(radians_to_angle_word(craft.body_roll), 842);

        // The opposite sustained window fixes the signs independently.
        craft.body_roll = angle_word_to_rad(-285);
        player.heading = angle_word_to_rad(5_889);
        craft.integrate_configured_micros_with_sensitivity(
            8_000,
            &MotionChannels {
                turn: 1.0,
                ..Default::default()
            },
            &mut player,
            10,
            RETAIL_DEFAULT_SENSITIVITY,
        );
        assert_eq!(radians_to_angle_word(player.heading), 5_628);
        assert_eq!(radians_to_angle_word(craft.body_roll), -531);

        // With no steering, only the one A690 damping copy remains.
        craft.body_roll = angle_word_to_rad(1_983);
        craft.integrate_configured_micros_with_sensitivity(
            8_000,
            &MotionChannels::default(),
            &mut player,
            10,
            RETAIL_DEFAULT_SENSITIVITY,
        );
        assert_eq!(radians_to_angle_word(craft.body_roll), 1_929);
    }

    #[test]
    fn empty_vtol_callback_keeps_sub_d_steering_but_skips_a690_attitude() {
        let mut craft = PlayerCraft::new();
        craft.mode = VehicleMode::Vtol;
        craft.body_pitch = angle_word_to_rad(0x1000);
        craft.body_roll = angle_word_to_rad(603);
        craft.gun_barrel = 1_000.0;
        let mut player = dummy_entity();
        player.heading = angle_word_to_rad(4_250);

        let frame = craft.integrate_configured_micros_with_sensitivity(
            8_000,
            &MotionChannels {
                pitch: 2_303.0,
                turn: -1.0,
                positive_thrust_binding_active: true,
                ..Default::default()
            },
            &mut player,
            10,
            RETAIL_DEFAULT_SENSITIVITY,
        );
        let VehicleFrameForces::Vtol(frame) = frame else {
            panic!("VTOL mode must return a VTOL force frame");
        };

        assert!(!frame.has_fuel);
        assert_eq!(radians_to_angle_word(player.heading), 4_512);
        assert_eq!(frame.roll_before_a690_raw, 865);
        assert_eq!(radians_to_angle_word(craft.body_roll), 865);
        assert_eq!(radians_to_angle_word(craft.body_pitch), 0x1000);
        assert_eq!(craft.gun_barrel, 875.0);
    }

    #[test]
    fn fuel_exhaustion_returns_on_the_next_empty_vtol_callback() {
        let mut craft = PlayerCraft::new();
        craft.fuel_raw = 1;
        craft.toggle_mode();
        assert_eq!(craft.mode, VehicleMode::Vtol);
        craft.body_pitch = angle_word_to_rad(0x1000);
        craft.body_roll = angle_word_to_rad(1_983);

        let mut player = dummy_entity();
        let fueled_frame = craft.integrate_configured_micros_with_sensitivity(
            20_000,
            &MotionChannels::default(),
            &mut player,
            10,
            RETAIL_DEFAULT_SENSITIVITY,
        );
        assert!(!fueled_frame.vtol_started_without_fuel());
        assert_eq!(
            radians_to_angle_word(craft.body_pitch),
            vtol_self_right_pitch_raw(0x1000, 20_000, 10)
        );
        assert_eq!(
            radians_to_angle_word(craft.body_roll),
            vtol_roll_after_a690_raw(1_983, 20_000)
        );

        // The specialized force phase burns the last unit after the control
        // callback has already run A690. The mode return therefore belongs to
        // the following callback, which begins empty and skips A690.
        craft.consume_fuel_raw(1);
        assert_eq!(craft.fuel_raw, 0);
        assert_eq!(craft.mode, VehicleMode::Vtol);
        let pitch_after_last_fueled_callback = radians_to_angle_word(craft.body_pitch);
        let roll_after_last_fueled_callback = radians_to_angle_word(craft.body_roll);

        let empty_frame = craft.integrate_configured_micros_with_sensitivity(
            20_000,
            &MotionChannels::default(),
            &mut player,
            10,
            RETAIL_DEFAULT_SENSITIVITY,
        );
        assert!(empty_frame.vtol_started_without_fuel());
        assert_eq!(
            radians_to_angle_word(craft.body_pitch),
            pitch_after_last_fueled_callback
        );
        assert_eq!(
            radians_to_angle_word(craft.body_roll),
            roll_after_last_fueled_callback
        );
        assert!(craft.finish_fuel_frame(empty_frame));
        assert_eq!(craft.mode, VehicleMode::Hover);

        // The empty callback has completed the return and VTOL stays locked
        // until another pickup supplies a positive controller value.
        assert_eq!(craft.toggle_mode(), VehicleModeToggleOutcome::RefusedNoFuel);
        assert!(matches!(
            craft.collect_fuel_pickup_raw(10_000),
            FuelPickupOutcome::Collected { .. }
        ));
        assert_eq!(
            craft.toggle_mode(),
            VehicleModeToggleOutcome::Changed(VehicleMode::Vtol)
        );
    }

    #[test]
    fn low_fuel_warning_uses_powered_branch_and_strict_seventy_tick_gap() {
        let mut craft = PlayerCraft::new();
        craft.fuel_raw = FUEL_LOW_WARNING_BELOW_RAW - 1;
        craft.toggle_mode();
        let mut player = dummy_entity();
        let powered_frame = craft.integrate_configured_micros_with_sensitivity(
            20_000,
            &MotionChannels::default(),
            &mut player,
            10,
            RETAIL_DEFAULT_SENSITIVITY,
        );
        assert!(powered_frame.vtol_started_with_fuel());

        let mut cadence = FuelWarningCadence::default();
        assert!(!cadence.poll(powered_frame, craft.fuel_raw, 70));
        assert_eq!(cadence.last_tick(), 0);
        assert!(cadence.poll(powered_frame, craft.fuel_raw, 71));
        assert_eq!(cadence.last_tick(), 71);
        assert!(!cadence.poll(powered_frame, craft.fuel_raw, 141));
        assert!(cadence.poll(powered_frame, craft.fuel_raw, 142));

        // The comparison is strict in both dimensions, and the empty callback
        // takes the separate automatic-return branch instead of warning.
        assert!(!cadence.poll(powered_frame, FUEL_LOW_WARNING_BELOW_RAW, 1_000));
        craft.fuel_raw = 0;
        let empty_frame = craft.integrate_configured_micros_with_sensitivity(
            20_000,
            &MotionChannels::default(),
            &mut player,
            10,
            RETAIL_DEFAULT_SENSITIVITY,
        );
        assert!(empty_frame.vtol_started_without_fuel());
        assert!(!cadence.poll(empty_frame, 0, 1_000));
    }

    #[test]
    fn empty_callback_latches_return_even_if_fuel_is_collected_before_finish() {
        let mut craft = PlayerCraft::new();
        // Construct the callback-entry state directly: an already-airborne
        // craft can begin a later VTOL callback with its tank empty.
        craft.mode = VehicleMode::Vtol;
        let mut player = dummy_entity();
        let empty_frame = craft.integrate_configured_micros_with_sensitivity(
            20_000,
            &MotionChannels::default(),
            &mut player,
            0,
            RETAIL_DEFAULT_SENSITIVITY,
        );
        craft.collect_fuel_pickup_raw(10_000);

        assert!(craft.finish_fuel_frame(empty_frame));
        assert_eq!(craft.mode, VehicleMode::Hover);
        assert_eq!(craft.fuel_raw, 10_000);
    }

    #[test]
    fn raw_fuel_consumption_is_exact_nonnegative_and_saturating() {
        let mut craft = PlayerCraft::new();
        craft.fuel_raw = 100;

        craft.consume_fuel_raw(-7);
        assert_eq!(craft.fuel_raw, 100);
        craft.consume_fuel_raw(37);
        assert_eq!(craft.fuel_raw, 63);
        craft.consume_fuel_raw(100);
        assert_eq!(craft.fuel_raw, 0);
    }

    #[test]
    fn player_starts_empty_and_pickup_stub_preserves_retail_threshold_and_cap() {
        let mut craft = PlayerCraft::new();
        assert_eq!(craft.fuel_raw, 0);
        assert_eq!(craft.toggle_mode(), VehicleModeToggleOutcome::RefusedNoFuel);

        assert_eq!(
            craft.collect_fuel_pickup_raw(10_000),
            FuelPickupOutcome::Collected {
                added_raw: 10_000,
                fuel_raw: 10_000,
            }
        );
        assert_eq!(
            craft.toggle_mode(),
            VehicleModeToggleOutcome::Changed(VehicleMode::Vtol)
        );
        assert_eq!(
            craft.toggle_mode(),
            VehicleModeToggleOutcome::Changed(VehicleMode::Hover)
        );

        craft.fuel_raw = 190_000;
        assert_eq!(
            craft.collect_fuel_pickup_raw(25_000),
            FuelPickupOutcome::Collected {
                added_raw: 10_000,
                fuel_raw: FUEL_FULL_RAW,
            }
        );
        craft.fuel_raw = FUEL_PICKUP_ACCEPT_BELOW_RAW;
        assert_eq!(
            craft.collect_fuel_pickup_raw(10_000),
            FuelPickupOutcome::TankFull {
                fuel_raw: FUEL_PICKUP_ACCEPT_BELOW_RAW,
            }
        );
    }

    #[test]
    fn campaign_replacement_keeps_controller_fields_but_resets_world_component() {
        let mut craft = PlayerCraft::new();
        craft.mode = VehicleMode::Vtol;
        craft.fuel_raw = 99_119;
        craft.gun_barrel = 0.5;
        craft.body_pitch = -0.4;
        craft.body_roll = 0.25;
        craft.spin_angle = 1.5;
        craft.fan_rpm_state_q16 = 12_345;
        craft.fan_gain_envelope_q16 = 23_456;
        craft.mode_joint_5 = 30_000;
        craft.mode_joint_6 = 40_000;
        craft.weapon_selector = 2;

        let physics = HoverPhysicsConfig::default();
        let constructed_target = physics.randomized_target_speed(u16::MAX);
        let replacement = craft.campaign_world_replacement(physics, constructed_target);

        assert_eq!(replacement.mode, VehicleMode::Vtol);
        assert_eq!(replacement.fuel_raw, 99_119);
        assert_eq!(replacement.gun_barrel, 0.0);
        assert_eq!(replacement.body_pitch, 0.0);
        assert_eq!(replacement.body_roll, 0.0);
        assert_eq!(replacement.spin_angle, 0.0);
        assert_eq!(replacement.fan_rpm_state_q16, 0);
        assert_eq!(replacement.fan_gain_envelope_q16, 0);
        assert_eq!(replacement.mode_joint_words(), [0, 0]);
        assert_eq!(replacement.weapon_selector(), 0);
        assert_eq!(
            replacement.hover_target_speed_raw(),
            HoverPhysicsConfig::default().randomized_target_speed(u16::MAX)
        );
    }

    #[test]
    fn player4_mode_joints_follow_retail_q31_recurrence() {
        assert_eq!(step_mode_joint(0, true, 20_000), 2_499);
        assert_eq!(step_mode_joint(2_499, true, 20_000), 4_903);

        let mut joint = 0;
        for _ in 0..500 {
            joint = step_mode_joint(joint, true, 20_000);
        }
        // Positive truncation stalls 26 units below the full 0xFFFF target at
        // the retail 50-Hz step; that small asymmetry is observable behavior.
        assert_eq!(joint, 65_509);
        assert_eq!(step_mode_joint(joint, false, 20_000), 63_010);
        for _ in 0..300 {
            joint = step_mode_joint(joint, false, 20_000);
        }
        assert_eq!(joint, 0);
    }

    #[test]
    fn hover_deploys_player4_joints_and_vtol_retracts_without_changing_loadout() {
        let mut craft = fueled_craft();
        let mut player = dummy_entity();
        let ch = MotionChannels::default();

        assert_eq!(craft.mode_joint_words(), [0, 0]);
        for _ in 0..500 {
            craft.integrate_configured_micros(20_000, &ch, &mut player, 1);
        }
        assert_eq!(craft.mode_joint_words(), [65_509, 65_509]);
        assert_eq!(craft.anim_vars().dynamic[4], 0);
        assert_eq!(craft.anim_vars().dynamic[5], 65_509);
        assert_eq!(craft.anim_vars().dynamic[6], 65_509);

        assert_eq!(
            craft.toggle_mode(),
            VehicleModeToggleOutcome::Changed(VehicleMode::Vtol)
        );
        assert_eq!(craft.mode_joint_words(), [65_509, 65_509]);
        craft.integrate_configured_micros(20_000, &ch, &mut player, 1);
        assert_eq!(craft.mode_joint_words(), [63_010, 63_010]);
        assert_eq!(craft.anim_vars().dynamic[4], 0);
        for _ in 0..300 {
            craft.integrate_configured_micros(20_000, &ch, &mut player, 1);
        }
        assert_eq!(craft.mode_joint_words(), [0, 0]);
    }

    #[test]
    fn primary_joint_pulses_publish_to_callback_words_two_and_three() {
        let mut craft = PlayerCraft::new();
        craft.set_primary_joint_pulses([63_035, 15_535]);
        let vars = craft.anim_vars();
        assert_eq!(vars.dynamic[2], 63_035);
        assert_eq!(vars.dynamic[3], 15_535);
    }

    #[test]
    fn toggle_needs_fuel() {
        // User ground truth (2026-07-03): Hover→VTOL is refused on an empty
        // tank; the craft stays in Hover. With fuel it enters VTOL, and
        // VTOL→Hover is always allowed regardless of fuel.
        let mut craft = PlayerCraft::new();
        craft.fuel_raw = 0;
        // Empty tank: TAB refused, stays Hover.
        assert_eq!(craft.toggle_mode(), VehicleModeToggleOutcome::RefusedNoFuel);
        assert_eq!(craft.mode, VehicleMode::Hover);
        // With fuel: TAB enters VTOL.
        craft.fuel_raw = FUEL_FULL_RAW;
        assert_eq!(
            craft.toggle_mode(),
            VehicleModeToggleOutcome::Changed(VehicleMode::Vtol)
        );
        // Empty in VTOL: TAB back to Hover is still allowed.
        craft.fuel_raw = 0;
        assert_eq!(
            craft.toggle_mode(),
            VehicleModeToggleOutcome::Changed(VehicleMode::Hover)
        );
    }

    #[test]
    fn vtol_control_frame_preserves_pre_update_force_state_without_translation() {
        let mut craft = fueled_craft();
        assert_eq!(
            craft.toggle_mode(),
            VehicleModeToggleOutcome::Changed(VehicleMode::Vtol)
        );
        craft.body_pitch = angle_word_to_rad(0x600);
        craft.body_roll = angle_word_to_rad(-0x300);
        let mut player = dummy_entity();
        player.heading = angle_word_to_rad(0x1000);
        player.velocity = [8.0, 3.0, -7.0];
        let velocity_before = player.velocity;
        let fuel_before = craft.fuel_raw;
        let expected_basis = HoverBasis::from_angle_words(0x1000, 0x600, -0x300);

        let up = MotionChannels {
            throttle_q16: 65_536,
            pitch: 2_303.0,
            ..Default::default()
        };
        let frame = craft.integrate_configured_micros_with_sensitivity(
            20_000,
            &up,
            &mut player,
            0,
            RETAIL_DEFAULT_SENSITIVITY,
        );
        let VehicleFrameForces::Vtol(frame) = frame else {
            panic!("VTOL mode must return a VTOL force frame");
        };

        assert_eq!(frame.previous_basis, expected_basis);
        assert_eq!(frame.pitch_before_a690_raw, 0x600);
        assert_eq!(frame.roll_before_a690_raw, -0x300);
        assert_eq!(frame.throttle_q16, 65_536);
        assert!(frame.has_fuel);
        assert_eq!(player.velocity, velocity_before);
        assert_eq!(craft.fuel_raw, fuel_before);
    }

    #[test]
    fn vtol_control_frame_preserves_partial_q16_throttle_without_float_round_trip() {
        let mut craft = fueled_craft();
        assert_eq!(
            craft.toggle_mode(),
            VehicleModeToggleOutcome::Changed(VehicleMode::Vtol)
        );
        let mut player = dummy_entity();
        let frame = craft.integrate_configured_micros_with_sensitivity(
            8_000,
            &MotionChannels {
                throttle_q16: 0x4000,
                ..Default::default()
            },
            &mut player,
            10,
            RETAIL_DEFAULT_SENSITIVITY,
        );
        let VehicleFrameForces::Vtol(frame) = frame else {
            panic!("VTOL mode must return a VTOL force frame");
        };
        assert_eq!(frame.throttle_q16, 0x4000);
    }

    fn dummy_entity() -> Entity {
        Entity {
            construction_stamp_at_0xb4:
                crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            id: 0,
            authored_spawn_index: None,
            kind: crate::entity::EntityKind::Player,
            entity_type: 6,
            authored_follow_beacon_priority_raw: None,
            power_up_payload_packed: None,
            auto_pilot_payload_packed: None,
            factory_type61_birth_provenance: None,
            type60_construction_provenance: None,
            main_base_type54_sea_delta_source:
                crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            position: [0.0; 3],
            heading: 0.0,
            pitch_roll_raw: [0; 2],
            physical_body_basis_q31: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            velocity: [0.0; 3],
            surface_lifetime_timer_ms_at_0x48:
                crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            mass_raw: 0,
            capability_flags: 0,
            attached_to: None,
            model_slots: [Some(0); 4],
            model_index: Some(0),
            collision:
                crate::entity_collision_state::EntityCollisionRuntimeState::unresolved_port_entity(
                    0,
                ),
            initial_behavior: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            current_behavior_context: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            authored_radial_emitter: None,
            sub_n_runtime: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            base_factory_runtime: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            actor_animation_runtime: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            sub_a_propulsion_runtime: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            sub_g_06070_runtime: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            intro2_type13_common_mover_runtime: None,
            native_type13_allocation: None,
            intro2_type13_aim_runtime: None,
            intro2_type16_aim_runtime: None,
            intro2_type58_aim_runtime: None,
            intro2_type94_aim_runtime: None,
            intro2_flyer_aim_runtime: None,
            sub_h_external_frame_runtime:
                crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            sub_j_attachment_runtime: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            actor_common_axis_descriptor:
                crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            actor_tasks: Default::default(),
            ordinary_type9_pending_initial_selection: None,
            ordinary_type9_selected_component_runtime: None,
            main_base_type9_death_component_runtime: None,
            ordinary_type47_aim_and_fire_runtime: None,
            native_type61_allocation: None,
            native_type26_allocation: None,
            intro2_type26_sub_d_frame_owner: None,
            intro2_type26_sub_d_runtime: None,
            intro2_type47_sub_d_frame_owner: None,
            intro2_type47_sub_d_runtime: None,
            native_type47_construction: None,
            intro2_flyer_frame_owner: None,
            intro2_type53_runtime: None,
            native_type122_runtime: None,
            native_type30_runtime: None,
            native_type30_aim_runtime: None,
            native_type40_runtime: None,
            native_type40_aim_runtime: None,
            native_type43_runtime: None,
            native_type43_aim_runtime: None,
            native_type56_runtime: None,
            native_type56_aim_runtime: None,
            native_type122_aim_runtime: None,
            shared_fish_runtime: None,
            cleansing_vehicle_runtime: None,
            intro2_type16_runtime: None,
            intro2_type58_runtime: None,
            intro2_type66_runtime: None,
            intro2_gun_turret_runtime: None,
            intro2_gun_turret_aim_runtime: None,
            class49_death_runtime: None,
            native_entity_weapon_runtime: None,
            intro2_type10_runtime: None,
            intro2_type10_aim_runtime: None,
            intro2_type57_runtime: None,
            intro2_type57_aim_runtime: None,
            intro2_type94_runtime: None,
            intro2_type17_runtime: None,
            native_capture_relation: None,
            intro2_type8_runtime: None,
            native_type123_runtime: None,
            native_type123_anchor_raw_at_0x90:
                crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            native_type86_runtime: None,
            native_type86_anchor_raw_at_0x90:
                crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            intro2_type9_runtime: None,
            ordinary_type9_native_receipt: None,
            main_base_runtime: None,
            type17_sub_d_frame_owner: None,
            type17_sub_d_runtime: None,
            type8_sub_d_frame_owner: None,
            type8_wander_anchor_raw_at_0x90:
                crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            type8_sub_d_runtime: None,
            type47_immutable_anchor_raw_at_0x90:
                crate::entity_collision_state::RetailRuntimeValue::Unresolved,
            active: true,
        }
    }
}
