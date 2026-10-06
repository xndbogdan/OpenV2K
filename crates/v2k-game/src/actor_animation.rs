//! Exact common Section-12 Sub-I actor-animation controller.
//!
//! `FUN_00420730` allocates and zeroes a 20-byte controller, installs an
//! 80-millisecond reload, and binds its output through Sub-I's one-based
//! entity-variable selector. `FUN_00420520` advances at most one four-frame
//! phase per call: it subtracts `floor(delta_us / 1000)`, reloads to exactly
//! 80 only when the signed countdown becomes negative, and discards overshoot.
//! The authored descriptor byte at `+0x07` is a selector stride, not the phase
//! count.
//!
//! This module retains linked/special/forced-stop state and exposes its
//! precedence as a pure calculation. Fresh Attract Attention construction
//! owns the forced-stop write, and its selected production scheduler owns the
//! resulting velocity zero through an explicit Sub-I policy. Ordinary Type-9
//! cargo owns the exact relation attach/release writes and linked-target reads.

use v2k_formats::{collision::ActorAnimationDescriptor, models::AnimVars};

/// Reload installed at controller `+0x08`.
pub const ACTOR_ANIMATION_RELOAD_MILLIS: i32 = 80;
/// The phase byte at controller `+0x0C` always spans four frames.
pub const ACTOR_ANIMATION_PHASE_COUNT: u8 = 4;

/// Pure inputs whose linked-target ownership remains outside the selected
/// Attract Attention forced-stop path.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActorAnimationSelectionInput {
    /// Exact entity heading word read at entity `+0xA2`.
    pub yaw_raw: u16,
    /// Whether controller `+0x10` resolves to a currently live allocation.
    pub linked_target_valid: bool,
}

/// Result of the exact selector precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorAnimationSelection {
    pub selector: u16,
    /// The forced-stop branch zeroes entity velocity before selecting its
    /// linked-group frame. The selected Attract Attention production owner
    /// now applies it through its explicit Sub-I policy; neutral callers do
    /// not infer that authority.
    pub zero_velocity: bool,
}

/// Convert the entity's unsigned 16-bit heading into retail's eight bins.
pub const fn actor_direction_bin(yaw_raw: u16) -> u8 {
    match yaw_raw {
        0x0000..=0x0fff | 0xf000..=0xffff => 6,
        0x1000..=0x2fff => 5,
        0x3000..=0x4fff => 4,
        0x5000..=0x6fff => 3,
        0x7000..=0x8fff => 2,
        0x9000..=0xafff => 1,
        0xb000..=0xcfff => 0,
        0xd000..=0xefff => 7,
    }
}

/// Resolve `FUN_00420520`'s special → linked → forced-stop → directional
/// selector precedence without mutating an entity.
pub const fn actor_animation_selection(
    descriptor: ActorAnimationDescriptor,
    phase: u8,
    special_mode: bool,
    forced_stop: bool,
    input: ActorAnimationSelectionInput,
) -> ActorAnimationSelection {
    let stride = descriptor.frames_per_direction as u16;
    let phase = if phase < ACTOR_ANIMATION_PHASE_COUNT {
        phase
    } else {
        ACTOR_ANIMATION_PHASE_COUNT - 1
    } as u16;
    if special_mode {
        return ActorAnimationSelection {
            selector: stride * 9 + 2 + phase,
            zero_velocity: false,
        };
    }
    if input.linked_target_valid {
        return ActorAnimationSelection {
            selector: stride * 8 + phase,
            zero_velocity: false,
        };
    }
    if forced_stop {
        return ActorAnimationSelection {
            selector: stride * 8 + phase,
            zero_velocity: true,
        };
    }
    ActorAnimationSelection {
        selector: stride * actor_direction_bin(input.yaw_raw) as u16 + phase,
        zero_velocity: false,
    }
}

/// Per-entity state matching the fields consumed by `FUN_00420520`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorAnimationController {
    descriptor: ActorAnimationDescriptor,
    /// Controller `+0x04`.
    countdown_millis: i32,
    /// Controller `+0x08`.
    reload_millis: i32,
    /// Controller `+0x0C`.
    phase: u8,
    /// Controller `+0x0D`.
    cached_direction: u8,
    /// Controller `+0x0E`; nonzero enters the clamped special group.
    special_mode: bool,
    /// Controller `+0x0F`.
    forced_stop: bool,
    /// Controller `+0x10`; populated by the relation-attach callback.
    linked_handle: Option<u32>,
    /// Current word published through the descriptor's one-based binding.
    output: u16,
}

impl ActorAnimationController {
    /// Construct only when the descriptor can address the retained AnimVars
    /// bank. Unknown compatibility metadata and malformed bindings remain a
    /// fail-closed runtime boundary.
    pub fn from_descriptor(descriptor: ActorAnimationDescriptor) -> Option<Self> {
        let binding = usize::from(descriptor.variable_binding);
        if binding == 0 || binding >= AnimVars::default().dynamic.len() {
            return None;
        }
        Some(Self {
            descriptor,
            countdown_millis: 0,
            reload_millis: ACTOR_ANIMATION_RELOAD_MILLIS,
            phase: 0,
            cached_direction: 0,
            special_mode: false,
            forced_stop: false,
            linked_handle: None,
            output: 0,
        })
    }

    pub const fn descriptor(self) -> ActorAnimationDescriptor {
        self.descriptor
    }

    pub const fn output(self) -> u16 {
        self.output
    }

    pub const fn phase(self) -> u8 {
        self.phase
    }

    pub const fn countdown_millis(self) -> i32 {
        self.countdown_millis
    }

    pub const fn cached_direction(self) -> u8 {
        self.cached_direction
    }

    pub const fn linked_handle(self) -> Option<u32> {
        self.linked_handle
    }

    #[cfg(test)]
    pub(crate) fn set_linked_handle_for_test(&mut self, linked_handle: Option<u32>) {
        self.linked_handle = linked_handle;
    }

    pub const fn special_mode(self) -> bool {
        self.special_mode
    }

    pub const fn forced_stop(self) -> bool {
        self.forced_stop
    }

    /// Optional positional sound selected by `FUN_00420760` from the
    /// relation owner's capability word. The caller emits it at that
    /// owner's position before publishing the controller writes.
    pub(crate) const fn relation_attach_sound_id(self, owner_capabilities: u32) -> Option<u16> {
        let sound_id = if owner_capabilities & 0x201 != 0 {
            self.descriptor.capability_mask_0x201_sound_id
        } else if owner_capabilities & 8 != 0 {
            self.descriptor.capability_bit_3_sound_id
        } else {
            0
        };
        if sound_id == 0 {
            None
        } else {
            Some(sound_id)
        }
    }

    /// `FUN_00420760` writes only controller `+0x10` and phase `+0x0C`,
    /// after its optional sound. Later task retirement can independently call
    /// `420830` again, so the caller must preserve that callback order.
    pub(crate) fn apply_relation_attach(&mut self, owner_id: u32) {
        self.linked_handle = Some(owner_id);
        self.phase = 0;
    }

    /// `FUN_00420830` clears the linked handle, phase and forced-stop byte.
    /// The animation clock, cached direction, special mode and output survive.
    pub(crate) fn apply_relation_release(&mut self) {
        self.linked_handle = None;
        self.phase = 0;
        self.forced_stop = false;
    }

    /// `FUN_00402AC0`, the destructor installed by the attention-cue
    /// constructor at `0x00402A4F`, calls `FUN_00420830` too. This is an
    /// inner-task retirement effect, including retirement during a callback.
    /// It preserves the countdown, cached direction, special mode and output.
    pub(crate) fn apply_attention_cue_retirement(&mut self) {
        self.apply_relation_release();
    }

    /// Apply Exploding Person's exact `FUN_00420850` Sub-I reset.
    ///
    /// Retail writes the zero handle sentinel, special-mode byte one, and
    /// phase byte zero. Countdown, reload, cached direction, forced-stop, and
    /// the already-published output word deliberately survive unchanged.
    pub(crate) fn apply_exploding_person_reset(&mut self) {
        self.linked_handle = None;
        self.special_mode = true;
        self.phase = 0;
    }

    /// Apply Attract Attention's exact `FUN_00420870` forced-stop write.
    ///
    /// The caller owns the preceding optional positional sound. Retail emits
    /// that sound first, then writes only controller byte `+0x0F`; every other
    /// animation field survives unchanged.
    pub(crate) fn apply_attract_attention_forced_stop(&mut self) {
        self.forced_stop = true;
    }

    /// Whether linked, special, and forced-stop modes are all inactive, making
    /// [`Self::advance_neutral`] an exact runtime operation. The bounded
    /// selected Attract owner handles forced stop through a separate policy.
    ///
    /// Callers which cannot prove this state must fail closed instead of
    /// relying on the debug-only assertions in the narrow production bridge.
    pub const fn is_neutral_runtime(self) -> bool {
        !self.special_mode && !self.forced_stop && self.linked_handle.is_none()
    }

    /// Advance one common-controller call. `linked_target_valid` is separate
    /// from the retained handle because retail validates it through the live
    /// entity database on every update.
    pub fn advance(
        &mut self,
        elapsed_micros: u32,
        yaw_raw: u16,
        linked_target_valid: bool,
    ) -> ActorAnimationSelection {
        self.countdown_millis = self
            .countdown_millis
            .wrapping_sub((elapsed_micros / 1_000) as i32);
        if self.countdown_millis < 0 {
            self.countdown_millis = self.reload_millis;
            if self.special_mode {
                self.phase = self
                    .phase
                    .saturating_add(1)
                    .min(ACTOR_ANIMATION_PHASE_COUNT - 1);
            } else {
                self.phase = self.phase.wrapping_add(1);
                if self.phase >= ACTOR_ANIMATION_PHASE_COUNT {
                    self.phase = 0;
                }
            }
        }

        let selection = actor_animation_selection(
            self.descriptor,
            self.phase,
            self.special_mode,
            self.forced_stop,
            ActorAnimationSelectionInput {
                yaw_raw,
                linked_target_valid,
            },
        );
        if !self.special_mode && !linked_target_valid && !self.forced_stop {
            self.cached_direction = actor_direction_bin(yaw_raw);
        }
        self.output = selection.selector;
        selection
    }

    /// Production bridge for the currently proven neutral state.
    pub fn advance_neutral(&mut self, elapsed_micros: u32, yaw_raw: u16) {
        debug_assert!(!self.special_mode);
        debug_assert!(!self.forced_stop);
        debug_assert!(self.linked_handle.is_none());
        let selection = self.advance(elapsed_micros, yaw_raw, false);
        debug_assert!(!selection.zero_velocity);
    }

    /// Publish through the exact one-based selector installed by the loader.
    pub fn publish(self, vars: &mut AnimVars) {
        let binding = usize::from(self.descriptor.variable_binding);
        if binding != 0 && binding < vars.dynamic.len() {
            vars.dynamic[binding] = i32::from(self.output);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAN2: ActorAnimationDescriptor = ActorAnimationDescriptor {
        capability_bit_3_sound_id: 72,
        capability_mask_0x201_sound_id: 0,
        attention_stop_sound_id: 72,
        variable_binding: 1,
        frames_per_direction: 4,
    };

    #[test]
    fn exact_yaw_bin_boundaries_match_the_retail_chain() {
        for (yaw, expected) in [
            (0x0000, 6),
            (0x0fff, 6),
            (0x1000, 5),
            (0x2fff, 5),
            (0x3000, 4),
            (0x4fff, 4),
            (0x5000, 3),
            (0x6fff, 3),
            (0x7000, 2),
            (0x8fff, 2),
            (0x9000, 1),
            (0xafff, 1),
            (0xb000, 0),
            (0xcfff, 0),
            (0xd000, 7),
            (0xefff, 7),
            (0xf000, 6),
            (0xffff, 6),
        ] {
            assert_eq!(actor_direction_bin(yaw), expected, "yaw {yaw:04x}");
        }
    }

    #[test]
    fn fresh_controller_is_an_explicitly_neutral_runtime() {
        let mut controller = ActorAnimationController::from_descriptor(MAN2).unwrap();
        assert!(controller.is_neutral_runtime());

        controller.special_mode = true;
        assert!(!controller.is_neutral_runtime());
        controller.special_mode = false;
        controller.forced_stop = true;
        assert!(!controller.is_neutral_runtime());
        controller.forced_stop = false;
        controller.linked_handle = Some(0x04ac_0001);
        assert!(!controller.is_neutral_runtime());
    }

    #[test]
    fn exploding_person_reset_changes_only_the_three_authored_fields() {
        let mut controller = ActorAnimationController::from_descriptor(MAN2).unwrap();
        controller.countdown_millis = -17;
        controller.reload_millis = 91;
        controller.phase = 4;
        controller.cached_direction = 7;
        controller.special_mode = false;
        controller.forced_stop = true;
        controller.linked_handle = Some(0x04ac_0001);
        controller.output = 23;

        controller.apply_exploding_person_reset();

        assert_eq!(controller.countdown_millis, -17);
        assert_eq!(controller.reload_millis, 91);
        assert_eq!(controller.phase(), 0);
        assert_eq!(controller.cached_direction, 7);
        assert!(controller.special_mode());
        assert!(controller.forced_stop());
        assert_eq!(controller.linked_handle(), None);
        assert_eq!(controller.output, 23);
    }

    #[test]
    fn attract_attention_forced_stop_changes_only_controller_byte_0x0f() {
        let mut controller = ActorAnimationController::from_descriptor(MAN2).unwrap();
        controller.countdown_millis = -17;
        controller.reload_millis = 91;
        controller.phase = 3;
        controller.cached_direction = 7;
        controller.special_mode = true;
        controller.linked_handle = Some(0x04ac_0001);
        controller.output = 23;
        let before = controller;

        controller.apply_attract_attention_forced_stop();

        assert!(controller.forced_stop());
        assert_eq!(controller.descriptor(), before.descriptor());
        assert_eq!(controller.countdown_millis(), before.countdown_millis());
        assert_eq!(controller.reload_millis, before.reload_millis);
        assert_eq!(controller.phase(), before.phase());
        assert_eq!(controller.cached_direction(), before.cached_direction());
        assert_eq!(controller.special_mode(), before.special_mode());
        assert_eq!(controller.linked_handle(), before.linked_handle());
        assert_eq!(controller.output(), before.output());
    }

    #[test]
    fn relation_callbacks_preserve_clock_and_output_and_clear_stop_only_on_release() {
        let mut controller = ActorAnimationController::from_descriptor(MAN2).unwrap();
        controller.countdown_millis = -17;
        controller.reload_millis = 91;
        controller.phase = 3;
        controller.cached_direction = 7;
        controller.special_mode = true;
        controller.forced_stop = true;
        controller.output = 23;
        let before = controller;

        controller.apply_relation_attach(0x04ac_0001);
        assert_eq!(controller.linked_handle(), Some(0x04ac_0001));
        assert_eq!(controller.phase(), 0);
        assert!(controller.forced_stop());

        controller.phase = 2;
        controller.apply_relation_release();
        assert_eq!(controller.linked_handle(), None);
        assert_eq!(controller.phase(), 0);
        assert!(!controller.forced_stop());
        assert_eq!(controller.descriptor(), before.descriptor());
        assert_eq!(controller.countdown_millis(), before.countdown_millis());
        assert_eq!(controller.reload_millis, before.reload_millis);
        assert_eq!(controller.cached_direction(), before.cached_direction());
        assert_eq!(controller.special_mode(), before.special_mode());
        assert_eq!(controller.output(), before.output());
    }

    #[test]
    fn relation_sound_uses_owner_capabilities_with_mask_201_precedence() {
        let controller = ActorAnimationController::from_descriptor(MAN2).unwrap();
        assert_eq!(controller.relation_attach_sound_id(0), None);
        assert_eq!(controller.relation_attach_sound_id(8), Some(72));
        assert_eq!(controller.relation_attach_sound_id(0x209), None);
        let controller = ActorAnimationController::from_descriptor(ActorAnimationDescriptor {
            capability_mask_0x201_sound_id: 73,
            ..MAN2
        })
        .unwrap();
        assert_eq!(controller.relation_attach_sound_id(0x209), Some(73));
        assert_eq!(controller.relation_attach_sound_id(1), Some(73));
        assert_eq!(controller.relation_attach_sound_id(0x200), Some(73));
    }

    #[test]
    fn twenty_millisecond_updates_advance_immediately_then_every_fifth_call() {
        let mut controller = ActorAnimationController::from_descriptor(MAN2).unwrap();
        let mut outputs = Vec::new();
        for _ in 0..11 {
            controller.advance_neutral(20_000, 0xb000);
            outputs.push(controller.output());
        }
        assert_eq!(outputs, [1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3]);
        assert_eq!(controller.phase(), 3);
        assert_eq!(controller.countdown_millis(), 80);
    }

    #[test]
    fn timer_uses_a_strict_negative_edge_and_discards_large_delta_overshoot() {
        let mut controller = ActorAnimationController::from_descriptor(MAN2).unwrap();
        controller.advance_neutral(1_000, 0xb000);
        assert_eq!((controller.phase(), controller.countdown_millis()), (1, 80));

        controller.advance_neutral(80_000, 0xb000);
        assert_eq!(
            (controller.phase(), controller.countdown_millis()),
            (1, 0),
            "reaching zero does not advance"
        );
        controller.advance_neutral(1_000, 0xb000);
        assert_eq!((controller.phase(), controller.countdown_millis()), (2, 80));

        let mut large_delta = ActorAnimationController::from_descriptor(MAN2).unwrap();
        large_delta.advance_neutral(200_000, 0xb000);
        assert_eq!(
            (large_delta.phase(), large_delta.countdown_millis()),
            (1, 80),
            "retail advances once and discards overshoot"
        );
    }

    #[test]
    fn precedence_and_offsets_are_exact_and_forced_stop_is_only_a_request() {
        let input = ActorAnimationSelectionInput {
            yaw_raw: 0xb000,
            linked_target_valid: true,
        };
        assert_eq!(
            actor_animation_selection(MAN2, 2, true, true, input),
            ActorAnimationSelection {
                selector: 40,
                zero_velocity: false,
            },
            "special mode wins"
        );
        assert_eq!(
            actor_animation_selection(MAN2, 2, false, true, input),
            ActorAnimationSelection {
                selector: 34,
                zero_velocity: false,
            },
            "a valid link wins before forced stop"
        );
        assert_eq!(
            actor_animation_selection(
                MAN2,
                2,
                false,
                true,
                ActorAnimationSelectionInput {
                    linked_target_valid: false,
                    ..input
                },
            ),
            ActorAnimationSelection {
                selector: 34,
                zero_velocity: true,
            }
        );
    }

    #[test]
    fn type_9_publishes_selector_one_and_bad_bindings_fail_closed() {
        let mut controller = ActorAnimationController::from_descriptor(MAN2).unwrap();
        controller.advance_neutral(20_000, 0xb000);
        let mut vars = AnimVars::default();
        controller.publish(&mut vars);
        assert_eq!(vars.dynamic[1], 1);
        assert_eq!(vars.dynamic[0], 0);

        assert!(
            ActorAnimationController::from_descriptor(ActorAnimationDescriptor {
                variable_binding: 0,
                ..MAN2
            })
            .is_none()
        );
        assert!(
            ActorAnimationController::from_descriptor(ActorAnimationDescriptor {
                variable_binding: 64,
                ..MAN2
            })
            .is_none()
        );
    }
}
