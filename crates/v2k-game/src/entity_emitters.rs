//! Retained authored entity components which emit descriptor particles.
//!
//! Alien-Hive behavior initializer `FUN_00425760` clears task slots 2 and 1,
//! then `FUN_00425E10` `FUN_00401020`-publishes slot 0 with tick
//! `FUN_00425EA0` onto the already loader-owned component table. That tick
//! writes the Sub-K bound u16 through `FUN_0040a950` and forwards to shared
//! program `FUN_0041BEB0`, caching the authored animation and attachment for
//! radial/infection updates.
//! The controller survives later behavior-style changes, but it does not own
//! the Sub-N allocation or its alternate-cleanup state.

use v2k_formats::{fixed_math::retail_sine_q15, levels::EntityAnimation, models::AnimVars};

pub use crate::component_update::ComponentUpdateMode;
use crate::entity_behavior::BehaviorProgram;
use crate::entity_view_detail::RetailViewDetailContext;
use crate::hive_birth::{HiveBirthDecodeError, HiveBirthRuntime};
use crate::hive_controller::{
    advance_live_health, notify_health_transition, AuthoredHiveComponentFrame, HiveComponentHealth,
};
use crate::infection_evolution::{
    HiveInfectionEvolution, InfectionEvolutionEffects, InfectionTerrainSnapshot,
};
use crate::radial_damage::radial_distance_raw;

/// Initializer which installs the recovered Alien-Hive radial emitter.
pub const ALIEN_HIVE_EMITTER_INITIALIZER_VA: u32 = 0x0042_5760;
/// Particle class written into the component's shared spawn record.
pub const ALIEN_HIVE_EMITTER_PARTICLE_CLASS: u8 = 5;
/// Live `FUN_00425EA0` adds `elapsed_us >> 5` into Sub-K `+0x08`.
pub const HIVE_SUB_K_LIVE_PHASE_SHIFT: u32 = 5;
/// Dying `FUN_004260F0` adds `elapsed_us >> 8` into the bound Sub-K u16.
pub const HIVE_SUB_K_DYING_PHASE_SHIFT: u32 = 8;
/// `FUN_004260F0` unsigned cap after the wrapping add.
pub const HIVE_SUB_K_DYING_OUTPUT_CAP: u16 = 0xD000;
/// Bias applied after the duplicated-sine `SAR 19` in `FUN_00425EA0`.
pub const HIVE_SUB_K_SINE_BIAS: i32 = 0x1000;
/// `FUN_00425E10` / live `FUN_0041BEB0` controller word 0 while hostiles remain.
pub const HIVE_CONTROLLER_LOCKED: u32 = 1;
/// `FUN_0041BEB0` unlock: authored health restored, class-5 spit stops.
pub const HIVE_CONTROLLER_VULNERABLE: u32 = 2;
/// Lethal continuation: `*param_4 = 0`, spit stays off, wreck suction may arm.
pub const HIVE_CONTROLLER_DEAD: u32 = 0;
/// `FUN_0041BEB0` death write to controller word `0x12` when Sub-N `+0x4C` is set.
pub const HIVE_DEATH_SUCTION_DELAY_US: i32 = -6_000_000;
/// Dead-state wrap for that same word after suction has been live.
pub const HIVE_DEAD_SUCTION_WRAP_US: i32 = 600_000_000;
/// `FUN_00425370` outer gate before the wreck contact walk.
const HIVE_WRECK_CONTACT_APPROX_LIMIT: i32 = 8_000;
/// Player XZ² and `dy` envelope for wreck suction.
const HIVE_WRECK_SUCTION_XZ_SQ: i32 = 0x19_0000;
const HIVE_WRECK_SUCTION_MAX_DY: i32 = 3_000;
const HIVE_WRECK_INNER_XZ_SQ: i32 = 0x1_0000;
const HIVE_WRECK_NORMALIZE_XZ_SQ: i32 = 0x4_0000;
const HIVE_WRECK_Y_TARGET_RAW: i16 = -1_000;

/// Exact `FUN_00425EA0` Sub-K output from the live phase accumulator.
///
/// Retail zero-extends the quarter-sine table word, duplicates it into both
/// 16-bit halves, arithmetic-shifts 19, then adds `0x1000`.
pub fn fun_00425ea0_sub_k_word(phase: u32) -> u16 {
    let table_word = retail_sine_q15(phase) as u16;
    let duplicated = u32::from(table_word) | (u32::from(table_word) << 16);
    (duplicated as i32 >> 19).wrapping_add(HIVE_SUB_K_SINE_BIAS) as u16
}

/// Exact `FUN_004260F0` below-cap gate, wrapping add and unsigned `0xD000` cap.
pub fn fun_004260f0_sub_k_word(current: u16, elapsed_us: u32) -> u16 {
    if current >= HIVE_SUB_K_DYING_OUTPUT_CAP {
        return current;
    }
    let added = current.wrapping_add((elapsed_us >> HIVE_SUB_K_DYING_PHASE_SHIFT) as u16);
    if added > HIVE_SUB_K_DYING_OUTPUT_CAP {
        HIVE_SUB_K_DYING_OUTPUT_CAP
    } else {
        added
    }
}

/// Reproduce `FUN_00411400`'s outer detailed-update gate.
///
/// This is deliberately separate from render culling. Retail compares the
/// entity's wrapping signed X/Z words against a world-axis-aligned box around
/// the camera eye; it does not rotate the delta into camera space. Behind the
/// eye, the final plane uses camera-basis `U.z` from context `+0x28`.
pub fn retail_component_update_mode(
    entity_position_raw: [i16; 3],
    camera_eye: [f32; 3],
    camera_true_up_z: f32,
    scan_dimensions: (u32, u32),
) -> ComponentUpdateMode {
    if RetailViewDetailContext::from_world(camera_eye, camera_true_up_z, scan_dimensions)
        .classify(entity_position_raw)
        .uses_detailed_update()
    {
        ComponentUpdateMode::Detailed
    } else {
        ComponentUpdateMode::Coarse
    }
}

/// One synchronous class-5 allocation requested by the retained component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthoredRadialEmission {
    pub position_raw: [i16; 3],
    pub source_id: u32,
    pub particle_class: u8,
}

/// One ordered side-effect sink for the complete retained hive component.
///
/// Infection consumes shared RNG and can enqueue sound before the radial
/// emitter consumes its three particle words. A single adapter makes that
/// retail call order structural rather than dependent on caller discipline.
pub trait AuthoredHiveComponentEffects: InfectionEvolutionEffects {
    fn emit_authored_radial(&mut self, emission: AuthoredRadialEmission);
    /// DCA0's post-task cue uses the entity center, after Sub-N/K callbacks.
    fn queue_hive_detailed_sound(&mut self, sound_id: u16, position_raw: [i16; 3]);
}

pub(crate) struct HiveComponentVisit<'a> {
    pub objective_hostile_present: bool,
    pub mode: ComponentUpdateMode,
    pub source_position_raw: [i16; 3],
    pub source_id: u32,
    pub health: HiveComponentHealth<'a>,
}

/// Behavior-installed Alien-Hive radial/infection controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredRadialEmitterController {
    /// Exact six dwords copied from the Section-13 animation header to
    /// component words 7..12. Word zero is the emission interval.
    animation_words: [u32; 6],
    /// Sub-N words 2..4, added to the entity's wrapping signed position words.
    attachment_raw: [i16; 3],
    accumulator_us: u32,
    /// `FUN_0041BBD0` initializes the shared component controller to state 1.
    controller_state: u32,
    /// Intro2 operation 2 controls behavior/integration independently from
    /// render eligibility. Ordinary gameplay components start enabled.
    behavior_enabled: bool,
    /// Controller words 2..5 and their FIFO tail list, advanced before the
    /// radial-emission branch by the same `FUN_0041BEB0` callback.
    infection_evolution: HiveInfectionEvolution,
    /// Authored Sub-K is present. Type 67 retains `[1, 0]`.
    sub_k_bound: bool,
    /// Authored Sub-K byte 0. `FUN_00424450` stores `FUN_0040a950(entity, this)`
    /// at Sub-K `+0` when nonzero; byte 1 is a second bind the hive ticks do
    /// not write. Selector 1 maps to `AnimVars.dynamic[1]`.
    sub_k_word_selector: u8,
    /// Sub-K runtime `+0x08` phase word used by live `FUN_00425EA0`.
    sub_k_phase: u32,
    /// u16 written through `*[Sub-K+0]`.
    sub_k_output: u16,
    /// Slot-0 tick is `FUN_004260F0` rather than live `FUN_00425EA0`.
    dying_slot0: bool,
    /// Sub-N `+0x4C`: nearby campaign-marker kinds 22..=26 from `FUN_0041BC20`.
    wreck_contact_enabled: bool,
    /// Controller word `0x12`. Death arms `-6_000_000`; suction/warp require `> 0`.
    suction_timer_us: i32,
    /// Loader `1BC20` rows and two `1C830/1CA90` slots. The manager temporarily
    /// detaches this owner while native child callbacks mutate the live list.
    birth_runtime: Option<HiveBirthRuntime>,
    /// Invalid authored rows block their own lane, preserving radial/infection.
    birth_decode_error: Option<HiveBirthDecodeError>,
}

impl AuthoredRadialEmitterController {
    /// Build only the component proven to be installed by the resolved
    /// behavior initializer. Unknown/weighted behavior identity remains absent
    /// instead of becoming a type-number fallback.
    pub fn from_authored_spawn(
        program: &BehaviorProgram,
        sub_n_payload: Option<[u8; 10]>,
        animation: Option<&EntityAnimation>,
        sub_k_payload: Option<[u8; 2]>,
    ) -> Option<Self> {
        if program.initializer_callback_address != ALIEN_HIVE_EMITTER_INITIALIZER_VA {
            return None;
        }
        let sub_n_payload = sub_n_payload?;
        let animation = animation?;
        let attachment_raw = [
            i16::from_le_bytes(sub_n_payload[4..6].try_into().ok()?),
            i16::from_le_bytes(sub_n_payload[6..8].try_into().ok()?),
            i16::from_le_bytes(sub_n_payload[8..10].try_into().ok()?),
        ];
        let animation_words = std::array::from_fn(|index| {
            let start = index * 4;
            u32::from_le_bytes(
                animation.header[start..start + 4]
                    .try_into()
                    .expect("fixed animation header dword"),
            )
        });
        let (sub_k_bound, sub_k_word_selector) = match sub_k_payload {
            Some(payload) => (true, payload[0]),
            None => (false, 0),
        };
        let (birth_runtime, birth_decode_error) = match HiveBirthRuntime::from_animation(animation)
        {
            Ok(runtime) => (Some(runtime), None),
            Err(block) => (None, Some(block)),
        };
        Some(Self {
            animation_words,
            attachment_raw,
            accumulator_us: 0,
            controller_state: HIVE_CONTROLLER_LOCKED,
            behavior_enabled: true,
            infection_evolution: HiveInfectionEvolution::default(),
            sub_k_bound,
            sub_k_word_selector,
            sub_k_phase: 0,
            sub_k_output: 0,
            dying_slot0: false,
            wreck_contact_enabled: false,
            suction_timer_us: 0,
            birth_runtime,
            birth_decode_error,
        })
    }

    pub fn set_behavior_enabled(&mut self, enabled: bool) {
        self.behavior_enabled = enabled;
        if !enabled {
            self.accumulator_us = 0;
        }
    }

    pub const fn behavior_enabled(&self) -> bool {
        self.behavior_enabled
    }

    pub const fn interval_us(&self) -> u32 {
        self.animation_words[0]
    }

    pub const fn attachment_raw(&self) -> [i16; 3] {
        self.attachment_raw
    }

    pub const fn accumulator_us(&self) -> u32 {
        self.accumulator_us
    }

    pub const fn infection_accumulator_us(&self) -> i32 {
        self.infection_evolution.accumulator_us()
    }

    pub fn pending_infection_tail_count(&self) -> usize {
        self.infection_evolution.pending_tail_count()
    }

    pub const fn sub_k_output(&self) -> u16 {
        self.sub_k_output
    }

    pub const fn sub_k_word_selector(&self) -> u8 {
        self.sub_k_word_selector
    }

    pub const fn dying_slot0(&self) -> bool {
        self.dying_slot0
    }

    pub const fn controller_state(&self) -> u32 {
        self.controller_state
    }

    pub const fn wreck_contact_enabled(&self) -> bool {
        self.wreck_contact_enabled
    }

    pub const fn suction_timer_us(&self) -> i32 {
        self.suction_timer_us
    }

    pub fn birth_runtime(&self) -> Option<&HiveBirthRuntime> {
        self.birth_runtime.as_ref()
    }

    pub const fn birth_decode_error(&self) -> Option<HiveBirthDecodeError> {
        self.birth_decode_error
    }

    /// Detach after the radial prefix; restore after rows, contact and final
    /// ejection aging. Native birth may reallocate the manager's live vector.
    pub fn take_birth_runtime(&mut self) -> Option<HiveBirthRuntime> {
        self.birth_runtime.take()
    }

    pub fn restore_birth_runtime(&mut self, runtime: HiveBirthRuntime) {
        assert!(
            self.birth_runtime.is_none() && self.birth_decode_error.is_none(),
            "Hive birth restore requires the same detached loader-owned lane"
        );
        self.birth_runtime = Some(runtime);
    }

    /// Suction after the `-6_000_000` delay on a marker-backed wreck. Normal
    /// campaign routing separately consumes the player's static-contact stamp.
    pub const fn wreck_suction_ready(&self) -> bool {
        self.behavior_enabled
            && self.dying_slot0
            && self.wreck_contact_enabled
            && self.suction_timer_us > 0
    }

    /// Explicit fixture setup; production enters state2 in the shared callback.
    #[cfg(test)]
    pub fn enter_vulnerable(&mut self) {
        if self.dying_slot0 || self.controller_state != HIVE_CONTROLLER_LOCKED {
            return;
        }
        self.controller_state = HIVE_CONTROLLER_VULNERABLE;
        self.accumulator_us = 0;
    }

    /// Publish the Sub-K LFO through the one-based `FUN_0040a950` selector.
    ///
    /// `FUN_00424450` writes `FUN_0040a950(entity, authored[0])` at Sub-K `+0`
    /// when that byte is nonzero; selector 0 is a null bind. Live
    /// `FUN_00425EA0` and dying `FUN_004260F0` store the u16 through that
    /// pointer. The port maps selector N onto `AnimVars.dynamic[N]`, matching
    /// Sub-M's word-bank publishers. Do not invent a hive1xa morph; this write
    /// is the recovered consumer even if the mesh does not read selector 1.
    pub fn publish_sub_k_word(&self, vars: &mut AnimVars) {
        if !self.sub_k_bound {
            return;
        }
        let index = usize::from(self.sub_k_word_selector);
        if index != 0 && index < vars.dynamic.len() {
            vars.dynamic[index] = i32::from(self.sub_k_output);
        }
    }

    /// `FUN_00425790` replaces slot 0 with tick `FUN_004260F0` and sets state 0.
    pub fn enter_dying_slot0(&mut self) {
        self.dying_slot0 = true;
        self.controller_state = HIVE_CONTROLLER_DEAD;
        self.accumulator_us = 0;
    }

    /// Sub-N `+0x4C` was set by `FUN_0041BC20`; death writes word `0x12 = -6_000_000`.
    pub fn arm_wreck_suction(&mut self) {
        if !self.dying_slot0 {
            return;
        }
        self.wreck_contact_enabled = true;
        self.suction_timer_us = HIVE_DEATH_SUCTION_DELAY_US;
    }

    /// `FUN_0041BEB0` advances the dead-state word before the entity contact
    /// walk, including frames with no eligible player near the wreck.
    pub fn advance_wreck_timer(&mut self, elapsed_us: u32) {
        if !self.behavior_enabled || !self.dying_slot0 || !self.wreck_contact_enabled {
            return;
        }
        self.suction_timer_us = self.suction_timer_us.wrapping_add(elapsed_us as i32);
        if self.suction_timer_us > HIVE_DEAD_SUCTION_WRAP_US {
            self.suction_timer_us -= HIVE_DEAD_SUCTION_WRAP_US;
        }
    }

    /// Pull an eligible player toward the Sub-N contact origin using the
    /// already-advanced timer. The host owns entity eligibility and warp.
    pub fn apply_wreck_suction(
        &self,
        elapsed_us: u32,
        origin_raw: [i16; 3],
        player_position_raw: [i16; 3],
        player_velocity_raw: [i16; 3],
    ) -> Option<[i16; 3]> {
        if !self.wreck_suction_ready() {
            return None;
        }
        apply_fun_0041beb0_wreck_suction(
            elapsed_us,
            origin_raw,
            player_position_raw,
            player_velocity_raw,
        )
    }

    fn advance_live_sub_k(&mut self, elapsed_us: u32) {
        if !self.sub_k_bound {
            return;
        }
        self.sub_k_phase = self
            .sub_k_phase
            .wrapping_add(elapsed_us >> HIVE_SUB_K_LIVE_PHASE_SHIFT);
        self.sub_k_output = fun_00425ea0_sub_k_word(self.sub_k_phase);
    }

    fn advance_dying_sub_k(&mut self, elapsed_us: u32, mode: ComponentUpdateMode) {
        if !self.sub_k_bound || mode.retail_value() != 0 {
            return;
        }
        self.sub_k_output = fun_004260f0_sub_k_word(self.sub_k_output, elapsed_us);
    }

    /// Live `FUN_00425EA0` writes Sub-K before `FUN_0041BEB0`; dying
    /// `FUN_004260F0` writes it after, and only on detailed ticks.
    pub fn advance_sub_k(&mut self, elapsed_us: u32, mode: ComponentUpdateMode) {
        if self.dying_slot0 {
            self.advance_dying_sub_k(elapsed_us, mode);
        } else {
            self.advance_live_sub_k(elapsed_us);
        }
    }

    /// Advance the infection/health/radial prefix of retained `FUN_0041BEB0`.
    ///
    /// The infection interval is independent from the detailed/coarse radial
    /// gate, but both operations require the component callback itself to be
    /// enabled. The current port is single-player, so it takes retail's
    /// authoritative `FUN_00436960` branch. The manager follows with the
    /// detached creature rows, contact walk and final `1CA90` slots.
    pub(crate) fn advance_component(
        &mut self,
        visit: HiveComponentVisit<'_>,
        frame: &mut AuthoredHiveComponentFrame<'_>,
        terrain: &mut InfectionTerrainSnapshot,
        effects: &mut impl AuthoredHiveComponentEffects,
    ) {
        let elapsed_us = frame.elapsed_us;
        let HiveComponentVisit {
            objective_hostile_present,
            mode,
            source_position_raw,
            source_id,
            health,
        } = visit;
        if !self.dying_slot0 {
            self.advance_sub_k(elapsed_us, mode);
        }
        if self.behavior_enabled {
            self.infection_evolution.advance(
                elapsed_us as i32,
                objective_hostile_present,
                true,
                self.controller_state,
                self.animation_words,
                terrain,
                effects,
            );
            // 1BEB0 advances infection first, then the live lock/unlock, then
            // decides whether state1 still permits this callback's class5.
            if let HiveComponentHealth::Live(live) = health {
                if let Some(transition) = advance_live_health(
                    &mut self.controller_state,
                    live,
                    objective_hostile_present,
                    elapsed_us,
                ) {
                    notify_health_transition(transition, mode, frame);
                }
            }
        }
        self.advance_wreck_timer(elapsed_us);
        self.advance_radial(
            elapsed_us,
            mode,
            source_position_raw,
            source_id,
            |emission| effects.emit_authored_radial(emission),
        );
    }

    /// Advance the exact strict-interval gate and synchronously visit every
    /// crossed allocation in catch-up order.
    ///
    /// Coarse updates, disabled behavior, and non-state-1 controllers reset the
    /// accumulator exactly like `FUN_0041BEB0`. Equality does not emit.
    pub fn advance_radial(
        &mut self,
        elapsed_us: u32,
        mode: ComponentUpdateMode,
        source_position_raw: [i16; 3],
        source_id: u32,
        mut emit: impl FnMut(AuthoredRadialEmission),
    ) {
        if !self.behavior_enabled
            || mode.retail_value() != 0
            || self.controller_state != 1
            || self.interval_us() == 0
        {
            self.accumulator_us = 0;
            return;
        }

        self.accumulator_us = self.accumulator_us.wrapping_add(elapsed_us);
        let interval_us = self.interval_us();
        while interval_us < self.accumulator_us {
            let position_raw = std::array::from_fn(|axis| {
                source_position_raw[axis].wrapping_add(self.attachment_raw[axis])
            });
            emit(AuthoredRadialEmission {
                position_raw,
                source_id,
                particle_class: ALIEN_HIVE_EMITTER_PARTICLE_CLASS,
            });
            self.accumulator_us = self.accumulator_us.wrapping_sub(interval_us);
        }
    }
}

/// `FUN_0041BEB0` Q31 extract: shift the signed dword before widening for
/// the signed multiply, then retain the low result word.
pub fn fun_0041beb0_q31_delta(elapsed_us: u32, shift: u32, operand: i32) -> i16 {
    let shifted_elapsed = (elapsed_us as i32).wrapping_shl(shift);
    let product = i64::from(shifted_elapsed) * i64::from(operand);
    (product >> 31) as i16
}

/// Dead-hive player pull from `FUN_0041BEB0` after the suction timer is live.
pub fn apply_fun_0041beb0_wreck_suction(
    elapsed_us: u32,
    origin_raw: [i16; 3],
    player_position_raw: [i16; 3],
    player_velocity_raw: [i16; 3],
) -> Option<[i16; 3]> {
    if radial_distance_raw(origin_raw, player_position_raw) >= HIVE_WRECK_CONTACT_APPROX_LIMIT {
        return None;
    }
    let dx = i32::from(player_position_raw[0].wrapping_sub(origin_raw[0]));
    let dy = i32::from(player_position_raw[1].wrapping_sub(origin_raw[1]));
    let dz = i32::from(player_position_raw[2].wrapping_sub(origin_raw[2]));
    let xz_sq = dx.wrapping_mul(dx).wrapping_add(dz.wrapping_mul(dz));
    if xz_sq >= HIVE_WRECK_SUCTION_XZ_SQ || dy >= HIVE_WRECK_SUCTION_MAX_DY {
        return None;
    }

    let mut velocity = player_velocity_raw;
    if xz_sq < HIVE_WRECK_INNER_XZ_SQ {
        let step = fun_0041beb0_q31_delta(elapsed_us, 11, 0x5dc);
        if velocity[1] < -999 {
            let next = velocity[1].wrapping_add(step);
            velocity[1] = if next > HIVE_WRECK_Y_TARGET_RAW {
                HIVE_WRECK_Y_TARGET_RAW
            } else {
                next
            };
        } else {
            let next = velocity[1].wrapping_sub(step);
            velocity[1] = if next < HIVE_WRECK_Y_TARGET_RAW {
                HIVE_WRECK_Y_TARGET_RAW
            } else {
                next
            };
        }
    }

    let (pull_x, pull_z) = if xz_sq > HIVE_WRECK_NORMALIZE_XZ_SQ {
        (
            ((i64::from(dx) << 18) / i64::from(xz_sq)) as i32,
            ((i64::from(dz) << 18) / i64::from(xz_sq)) as i32,
        )
    } else {
        (dx, dz)
    };
    // Retail brakes outward velocity four times harder than it accelerates
    // a resting or already inward-moving player in FUN_0041BEB0.
    let x_shift = if i32::from(velocity[0]).wrapping_mul(pull_x) < 1 {
        12
    } else {
        14
    };
    velocity[0] = velocity[0].wrapping_sub(fun_0041beb0_q31_delta(elapsed_us, x_shift, pull_x));
    let z_shift = if i32::from(velocity[2]).wrapping_mul(pull_z) < 1 {
        12
    } else {
        14
    };
    velocity[2] = velocity[2].wrapping_sub(fun_0041beb0_q31_delta(elapsed_us, z_shift, pull_z));
    Some(velocity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_behavior::behavior_program;
    use crate::infection_evolution::{
        InfectionEvolutionEffects, InfectionTailSound, InfectionTerrainSnapshot,
        INFECTION_TERRAIN_TYPE_BIT,
    };
    use std::collections::VecDeque;
    use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};

    fn animation(words: [u32; 6]) -> EntityAnimation {
        let mut header = [0; 0x18];
        for (index, word) in words.into_iter().enumerate() {
            header[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        EntityAnimation {
            header,
            frames: Vec::new(),
        }
    }

    fn controller() -> AuthoredRadialEmitterController {
        AuthoredRadialEmitterController::from_authored_spawn(
            behavior_program(46).expect("Alien Hive behavior"),
            Some([0, 0, 0, 0, 0x80, 0x01, 0x90, 0x01, 0x80, 0x01]),
            Some(&animation([50_000, 5_000, 5, 5, 0, 0x1234_5678])),
            Some([1, 0]),
        )
        .expect("authored emitter component")
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum OrderedEvent {
        TailSound(InfectionTailSound),
        Radial(AuthoredRadialEmission),
    }

    struct OrderedEffects {
        random_words: VecDeque<u16>,
        events: Vec<OrderedEvent>,
    }

    impl InfectionEvolutionEffects for OrderedEffects {
        fn next_shared_random_u16(&mut self) -> u16 {
            self.random_words
                .pop_front()
                .expect("scripted shared RNG exhausted")
        }

        fn queue_infection_tail_sound(&mut self, sound: InfectionTailSound) {
            self.events.push(OrderedEvent::TailSound(sound));
        }
    }

    impl AuthoredHiveComponentEffects for OrderedEffects {
        fn queue_hive_detailed_sound(&mut self, _sound_id: u16, _position_raw: [i16; 3]) {
            panic!("the isolated component does not execute DCA0's sound suffix");
        }
        fn emit_authored_radial(&mut self, emission: AuthoredRadialEmission) {
            self.events.push(OrderedEvent::Radial(emission));
        }
    }

    fn infection_terrain() -> TerrainGrid {
        let mut terrain = TerrainGrid {
            header: [i32::from(-0x1000_i16) << 8, 0, 0, 0, 0],
            cells: vec![
                TerrainCell {
                    height: 2,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        };
        for x in 9_u8..=12 {
            for z in 9_u8..=11 {
                terrain.cells[usize::from(x) * GRID_SIZE + usize::from(z)].terrain_type |=
                    INFECTION_TERRAIN_TYPE_BIT;
            }
        }
        terrain
    }

    #[test]
    fn constructor_requires_the_proven_initializer_and_retains_authored_words() {
        let controller = controller();
        assert_eq!(controller.interval_us(), 50_000);
        assert_eq!(controller.attachment_raw(), [384, 400, 384]);
        assert!(controller.behavior_enabled());
        assert_eq!(controller.sub_k_word_selector(), 1);
        assert_eq!(controller.sub_k_output(), 0);
        assert!(AuthoredRadialEmitterController::from_authored_spawn(
            behavior_program(39).expect("Working Factory behavior"),
            Some([0; 10]),
            Some(&animation([50_000, 0, 0, 0, 0, 0])),
            Some([1, 0]),
        )
        .is_none());
    }

    #[test]
    fn component_retains_birth_records_and_detached_custody_across_style_changes() {
        let mut authored = animation([50_000, 5_000, 5, 5, 1, 0x1234_5678]);
        let mut record = [0; 0x1C];
        for (index, word) in [15_i32, -123, 100, -7, 3, 2, 3].into_iter().enumerate() {
            record[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        authored.frames.push(record);
        let mut controller = AuthoredRadialEmitterController::from_authored_spawn(
            behavior_program(46).unwrap(),
            Some([0; 10]),
            Some(&authored),
            None,
        )
        .unwrap();
        assert_eq!(controller.birth_decode_error(), None);
        let runtime = controller.take_birth_runtime().unwrap();
        assert!(controller.birth_runtime().is_none());
        assert_eq!(runtime.rows()[0].timer_us(), -123);
        assert_eq!(runtime.rows()[0].record().jitter_operand, -7);
        let retained = runtime.clone();
        controller.enter_vulnerable();
        controller.restore_birth_runtime(runtime);
        assert_eq!(controller.birth_runtime(), Some(&retained));
        controller.enter_dying_slot0();
        assert_eq!(controller.birth_runtime(), Some(&retained));
        assert_eq!(controller.controller_state(), 0);
    }

    #[test]
    fn malformed_birth_count_blocks_only_rows_and_radial_prefix_still_runs() {
        let authored = animation([50_000, 5_000, 5, 5, 1, 0x1234_5678]);
        let mut controller = AuthoredRadialEmitterController::from_authored_spawn(
            behavior_program(46).unwrap(),
            Some([0; 10]),
            Some(&authored),
            None,
        )
        .unwrap();
        assert_eq!(
            controller.birth_decode_error(),
            Some(HiveBirthDecodeError {
                authored_count: 1,
                decoded_count: 0,
            })
        );
        assert!(controller.take_birth_runtime().is_none());
        let mut emissions = Vec::new();
        controller.advance_radial(
            50_001,
            ComponentUpdateMode::Detailed,
            [0; 3],
            67,
            |emission| emissions.push(emission),
        );
        assert_eq!(emissions.len(), 1);
    }

    #[test]
    fn strict_interval_and_catch_up_match_component_callback() {
        let mut controller = controller();
        let mut emissions = Vec::new();
        controller.advance_radial(
            50_000,
            ComponentUpdateMode::Detailed,
            [i16::MAX, -640, 32_512],
            24,
            |emission| emissions.push(emission),
        );
        assert!(emissions.is_empty(), "interval equality must not emit");
        assert_eq!(controller.accumulator_us(), 50_000);

        controller.advance_radial(
            100_001,
            ComponentUpdateMode::Detailed,
            [i16::MAX, -640, 32_512],
            24,
            |emission| emissions.push(emission),
        );
        assert_eq!(emissions.len(), 3);
        assert!(emissions.iter().all(|emission| {
            emission.position_raw == [-32_385, -240, -32_640]
                && emission.source_id == 24
                && emission.particle_class == 5
        }));
        assert_eq!(controller.accumulator_us(), 1);
    }

    #[test]
    fn coarse_or_disabled_updates_reset_accumulated_time() {
        let mut controller = controller();
        controller.advance_radial(
            40_000,
            ComponentUpdateMode::Detailed,
            [0; 3],
            1,
            |_| unreachable!(),
        );
        controller.advance_radial(
            20_000,
            ComponentUpdateMode::Coarse,
            [0; 3],
            1,
            |_| unreachable!(),
        );
        assert_eq!(controller.accumulator_us(), 0);

        controller.advance_radial(
            40_000,
            ComponentUpdateMode::Detailed,
            [0; 3],
            1,
            |_| unreachable!(),
        );
        controller.set_behavior_enabled(false);
        assert_eq!(controller.accumulator_us(), 0);
        controller.advance_radial(
            60_000,
            ComponentUpdateMode::Detailed,
            [0; 3],
            1,
            |_| unreachable!(),
        );
        assert_eq!(controller.accumulator_us(), 0);
    }

    #[test]
    fn infection_tail_sound_precedes_same_callback_radial_emission() {
        let mut health = 2_000;
        let mut grace = 0;
        let mut controller = AuthoredRadialEmitterController::from_authored_spawn(
            behavior_program(46).expect("Alien Hive behavior"),
            Some([0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            Some(&animation([300_000, 5_000, 5, 5, 0, 0])),
            Some([1, 0]),
        )
        .expect("authored emitter component");
        let terrain = infection_terrain();
        let mut terrain = InfectionTerrainSnapshot::from_terrain(&terrain);
        let mut effects = OrderedEffects {
            random_words: [10, 10, 0, 1, 1, 1].into(),
            events: Vec::new(),
        };

        controller.advance_component(
            HiveComponentVisit {
                objective_hostile_present: true,
                mode: ComponentUpdateMode::Detailed,
                source_position_raw: [0; 3],
                source_id: 67,
                health: HiveComponentHealth::Live(crate::hive_controller::HiveLiveHealth {
                    health_raw: &mut health,
                    authored_health_raw: 2_000,
                    grace_timer_us: &mut grace,
                }),
            },
            &mut AuthoredHiveComponentFrame {
                elapsed_us: 5_001,
                terrain: None,
                retail_tick: 0,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                world_complete_tally:
                    &mut crate::world_complete_results::WorldCompleteTally::default(),
            },
            &mut terrain,
            &mut effects,
        );
        assert!(effects.events.is_empty());
        assert_eq!(controller.pending_infection_tail_count(), 1);

        effects
            .random_words
            .extend(std::iter::repeat(0).take(69 * 2));
        effects.random_words.push_back(1);
        controller.advance_component(
            HiveComponentVisit {
                objective_hostile_present: true,
                mode: ComponentUpdateMode::Detailed,
                source_position_raw: [0; 3],
                source_id: 67,
                health: HiveComponentHealth::Live(crate::hive_controller::HiveLiveHealth {
                    health_raw: &mut health,
                    authored_health_raw: 2_000,
                    grace_timer_us: &mut grace,
                }),
            },
            &mut AuthoredHiveComponentFrame {
                elapsed_us: 345_000,
                terrain: None,
                retail_tick: 0,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                world_complete_tally:
                    &mut crate::world_complete_results::WorldCompleteTally::default(),
            },
            &mut terrain,
            &mut effects,
        );

        assert!(matches!(
            effects.events.as_slice(),
            [
                OrderedEvent::TailSound(InfectionTailSound {
                    global_sound_id: 0x40,
                    ..
                }),
                OrderedEvent::Radial(AuthoredRadialEmission { source_id: 67, .. })
            ]
        ));
    }

    #[test]
    fn live_sub_k_word_matches_fun_00425ea0() {
        assert_eq!(fun_00425ea0_sub_k_word(0), 0x1000);
        let peak = fun_00425ea0_sub_k_word(0x4000);
        assert_eq!(peak, (0x1000 + (0x7fff >> 3)) as u16);
        let mut controller = controller();
        controller.advance_sub_k(32, ComponentUpdateMode::Detailed);
        assert_eq!(controller.sub_k_output(), fun_00425ea0_sub_k_word(1));
        controller.advance_sub_k(32, ComponentUpdateMode::Coarse);
        assert_eq!(controller.sub_k_output(), fun_00425ea0_sub_k_word(2));
    }

    #[test]
    fn dying_sub_k_word_ramps_only_on_detailed_ticks() {
        assert_eq!(fun_004260f0_sub_k_word(0x1000, 0x100), 0x1001);
        assert_eq!(
            fun_004260f0_sub_k_word(HIVE_SUB_K_DYING_OUTPUT_CAP - 1, 0x200),
            HIVE_SUB_K_DYING_OUTPUT_CAP
        );
        let mut controller = controller();
        controller.advance_sub_k(0x20, ComponentUpdateMode::Detailed);
        let live = controller.sub_k_output();
        controller.enter_dying_slot0();
        controller.advance_sub_k(0x100, ComponentUpdateMode::Coarse);
        assert_eq!(controller.sub_k_output(), live);
        controller.advance_sub_k(0x100, ComponentUpdateMode::Detailed);
        assert_eq!(
            controller.sub_k_output(),
            fun_004260f0_sub_k_word(live, 0x100)
        );
        let mut vars = AnimVars::default();
        controller.publish_sub_k_word(&mut vars);
        assert_eq!(vars.dynamic[1], i32::from(controller.sub_k_output()));
        assert!(vars.dynamic[0] == 0 && vars.dynamic[2..].iter().all(|&value| value == 0));
    }

    #[test]
    fn sub_k_selector_one_publishes_the_lfo_into_anim_vars() {
        let mut controller = controller();
        let mut vars = AnimVars::default();
        controller.publish_sub_k_word(&mut vars);
        assert_eq!(vars.dynamic[1], 0);
        controller.advance_sub_k(32, ComponentUpdateMode::Detailed);
        controller.publish_sub_k_word(&mut vars);
        assert_eq!(vars.dynamic[1], i32::from(fun_00425ea0_sub_k_word(1)));
        assert!(vars.dynamic[0] == 0 && vars.dynamic[2..].iter().all(|&value| value == 0));
    }

    #[test]
    fn sub_k_selector_zero_is_a_null_fun_0040a950_bind() {
        let mut controller = AuthoredRadialEmitterController::from_authored_spawn(
            behavior_program(46).expect("Alien Hive behavior"),
            Some([0, 0, 0, 0, 0x80, 0x01, 0x90, 0x01, 0x80, 0x01]),
            Some(&animation([50_000, 5_000, 5, 5, 0, 0x1234_5678])),
            Some([0, 0]),
        )
        .expect("authored emitter component");
        controller.advance_sub_k(32, ComponentUpdateMode::Detailed);
        assert_ne!(controller.sub_k_output(), 0);
        let mut vars = AnimVars::default();
        controller.publish_sub_k_word(&mut vars);
        assert!(vars.dynamic.iter().all(|&value| value == 0));
    }

    #[test]
    fn unlock_and_death_stop_class_5_spit() {
        let mut controller = controller();
        let mut emissions = Vec::new();
        controller.advance_radial(
            50_001,
            ComponentUpdateMode::Detailed,
            [0; 3],
            1,
            |emission| emissions.push(emission),
        );
        assert_eq!(emissions.len(), 1);
        assert_eq!(controller.controller_state(), HIVE_CONTROLLER_LOCKED);

        controller.enter_vulnerable();
        assert_eq!(controller.controller_state(), HIVE_CONTROLLER_VULNERABLE);
        assert_eq!(controller.accumulator_us(), 0);
        emissions.clear();
        controller.advance_radial(
            50_001,
            ComponentUpdateMode::Detailed,
            [0; 3],
            1,
            |emission| emissions.push(emission),
        );
        assert!(emissions.is_empty());

        controller.enter_dying_slot0();
        assert_eq!(controller.controller_state(), HIVE_CONTROLLER_DEAD);
        controller.advance_radial(
            50_001,
            ComponentUpdateMode::Detailed,
            [0; 3],
            1,
            |emission| emissions.push(emission),
        );
        assert!(emissions.is_empty());
    }

    #[test]
    fn wreck_suction_waits_six_seconds_then_pulls_toward_the_anchor() {
        assert_eq!(fun_0041beb0_q31_delta(20_000, 11, 0x5dc), 28);
        let mut controller = controller();
        controller.enter_dying_slot0();
        controller.arm_wreck_suction();
        assert_eq!(controller.suction_timer_us(), HIVE_DEATH_SUCTION_DELAY_US);
        assert!(!controller.wreck_suction_ready());
        let origin = [-17_536, 0, -32_384];
        controller.advance_wreck_timer(6_000_000);
        assert_eq!(
            controller.apply_wreck_suction(20_000, origin, [origin[0] + 100, 0, origin[2]], [0; 3]),
            None,
            "equality on the -6_000_000 delay must not pull"
        );
        controller.advance_wreck_timer(20_000);
        assert_eq!(
            controller.apply_wreck_suction(20_000, origin, [origin[0] + 100, 0, origin[2]], [0; 3]),
            Some([-3, -28, 0])
        );
        assert!(controller.wreck_suction_ready());
        assert!(apply_fun_0041beb0_wreck_suction(
            20_000,
            origin,
            [origin[0] + 2_000, 0, origin[2]],
            [0; 3]
        )
        .is_none());
    }

    #[test]
    fn wreck_suction_brakes_outward_motion_harder_on_each_axis() {
        // Source branches use SHL 12 for velocity * displacement < 1 and
        // SHL 14 otherwise. Signed Q31 rounding deliberately differs for
        // opposite directions; the two sides are not exact negations.
        for (position, initial, expected) in [
            ([100, 0, 0], [0, 0, 0], [-3, -28, 0]),
            ([100, 0, 0], [-100, 0, 0], [-103, -28, 0]),
            ([100, 0, 0], [100, 0, 0], [85, -28, 0]),
            ([-100, 0, 0], [0, 0, 0], [4, -28, 0]),
            ([-100, 0, 0], [100, 0, 0], [104, -28, 0]),
            ([-100, 0, 0], [-100, 0, 0], [-84, -28, 0]),
            ([0, 0, 100], [0, 0, 100], [0, -28, 85]),
            ([0, 0, -100], [0, 0, 0], [0, -28, 4]),
            ([0, 0, -100], [0, 0, -100], [0, -28, -84]),
        ] {
            assert_eq!(
                apply_fun_0041beb0_wreck_suction(20_000, [0; 3], position, initial),
                Some(expected),
                "position {position:?}, initial velocity {initial:?}"
            );
        }
    }

    #[test]
    fn wreck_suction_preserves_contact_envelope_and_vertical_target() {
        for (position, initial, expected) in [
            ([255, 0, 0], [0, -990, 0], Some([-9, -1000, 0])),
            ([256, 0, 0], [0, -990, 0], Some([-9, -990, 0])),
            ([0, 0, 0], [0, -1005, 0], Some([0, -1000, 0])),
            ([0, 0, 0], [0, -1200, 0], Some([0, -1172, 0])),
            ([640, 0, 0], [0; 3], Some([-15, 0, 0])),
            ([1279, 0, 0], [0; 3], Some([-7, 0, 0])),
            ([1280, 0, 0], [0; 3], None),
            ([0, 2999, 0], [0; 3], Some([0, -28, 0])),
            ([0, 3000, 0], [0; 3], None),
            ([0, -7999, 0], [0; 3], Some([0, -28, 0])),
            ([0, -8000, 0], [0; 3], None),
        ] {
            assert_eq!(
                apply_fun_0041beb0_wreck_suction(20_000, [0; 3], position, initial),
                expected,
                "position {position:?}, initial velocity {initial:?}"
            );
        }
        assert_eq!(
            apply_fun_0041beb0_wreck_suction(
                20_000,
                [32760, 0, 32760],
                [-32676, 0, -32676],
                [0; 3],
            ),
            Some([-3, -28, -3]),
            "the contact delta wraps through signed position words"
        );
    }

    #[test]
    fn wreck_q31_shifts_the_signed_dword_before_multiplication() {
        assert_eq!(fun_0041beb0_q31_delta(125_000, 14, 100), 95);
        assert_eq!(fun_0041beb0_q31_delta(131_071, 14, 100), 99);
        assert_eq!(fun_0041beb0_q31_delta(131_072, 14, 100), -100);
        assert_eq!(fun_0041beb0_q31_delta(131_072, 14, -100), 100);
    }

    #[test]
    fn wreck_timer_is_independent_of_contacts_and_respects_callback_admission() {
        let mut controller = controller();
        controller.enter_dying_slot0();
        assert!(
            !controller.wreck_suction_ready(),
            "an unmarked wreck is not an exit"
        );
        controller.advance_wreck_timer(6_000_001);
        assert_eq!(controller.suction_timer_us(), 0);
        controller.arm_wreck_suction();
        controller.set_behavior_enabled(false);
        assert!(!controller.wreck_suction_ready());
        controller.advance_wreck_timer(20_000);
        assert_eq!(controller.suction_timer_us(), HIVE_DEATH_SUCTION_DELAY_US);
        controller.set_behavior_enabled(true);
        controller.advance_wreck_timer(606_000_000);
        assert_eq!(controller.suction_timer_us(), HIVE_DEAD_SUCTION_WRAP_US);
        controller.advance_wreck_timer(1);
        assert_eq!(controller.suction_timer_us(), 1);
        assert!(controller.wreck_suction_ready());
        controller.set_behavior_enabled(false);
        assert_eq!(
            controller.apply_wreck_suction(20_000, [0; 3], [100, 0, 0], [0; 3]),
            None,
        );
    }

    #[test]
    fn coarse_wreck_callback_advances_timer_without_advancing_sub_k() {
        let mut controller = controller();
        controller.enter_dying_slot0();
        controller.arm_wreck_suction();
        let terrain = infection_terrain();
        let mut terrain = InfectionTerrainSnapshot::from_terrain(&terrain);
        let mut effects = OrderedEffects {
            random_words: [].into(),
            events: Vec::new(),
        };
        controller.advance_component(
            HiveComponentVisit {
                objective_hostile_present: false,
                mode: ComponentUpdateMode::Coarse,
                source_position_raw: [0; 3],
                source_id: 67,
                health: HiveComponentHealth::Dying,
            },
            &mut AuthoredHiveComponentFrame {
                elapsed_us: 6_000_001,
                terrain: None,
                retail_tick: 0,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                world_complete_tally:
                    &mut crate::world_complete_results::WorldCompleteTally::default(),
            },
            &mut terrain,
            &mut effects,
        );
        assert_eq!(controller.suction_timer_us(), 1);
        assert_eq!(controller.sub_k_output(), 0);
        assert!(controller.wreck_suction_ready());
        assert!(effects.events.is_empty());
    }
}
