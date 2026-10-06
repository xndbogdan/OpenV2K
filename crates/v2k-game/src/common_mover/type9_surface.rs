//! Bounded actor-surface behavior recovered from `FUN_0040E370`.
//!
//! First-world types 9 and 17 both prove Section-12 bytes
//! `+0x72=1,+0x73=0`: the entity timer at `+0x48` counts down outside the
//! deep-water band and counts up while submerged. The typed phase classifier is
//! shared by both owners. The random class-42 packet is also profile-neutral;
//! ordinary Type 9 adds its independent sound gate, while authenticated dying
//! Type 17 proves model-selector bit `0x4000` set and returns before that draw.
//!
//! The lifecycle boundary remains explicit.  Retail calls `FUN_00416750` and
//! `FUN_00410C10` synchronously when the timer reaches its authored duration,
//! then continues from the possibly mutated live record.  Until that owner is
//! attached to live Rust entities, this planner returns
//! [`Type9SurfaceFlow::RequiresLifecycleContinuation`] before consuming any
//! later RNG instead of inventing the post-death state.

/// Whole-owner no-op bit tested first by `FUN_0040E370`.
pub const ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT: u32 = 0x2000_0000;
/// Active-model selector bit which also suppresses the underwater sound gate.
pub const TYPE9_SURFACE_SOUND_SUPPRESSED_STATE_BIT: u32 = 0x0000_4000;
/// Bubble class passed to `FUN_00440A60`.
pub const TYPE9_UNDERWATER_BUBBLE_CLASS: u8 = 42;
/// Fixed full-gain, full-rate positional sound emitted underwater.
pub const TYPE9_UNDERWATER_SOUND_ID: u16 = 106;
/// Section-12 `+0x74` in every high-resolution first-world presentation tier.
pub const ORDINARY_TYPE9_AUTHORED_LIFETIME_MS: u32 = 5_000;

/// Runtime dword stored at actor entity `+0x48`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Type9SurfaceRuntime {
    pub lifetime_timer_ms_at_0x48: u32,
}

/// Fully resolved inputs to one supported actor-profile `FUN_0040E370` call.
///
/// `emission_axis_q31` is the live signed-Q31 vector at entity
/// `+0x24/+0x28/+0x2C`. `active_model_extent_raw` is Section-8 model header
/// `+0x08`, not the collision radius at `+0x0A`. `flat_surface_y_raw` comes
/// from the active world header's first dword shifted right by eight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type9SurfaceFrame {
    pub entity_id: u32,
    pub entity_type: u8,
    pub state_flags: u32,
    pub position_raw: [i16; 3],
    pub emission_axis_q31: [i32; 3],
    pub active_model_extent_raw: u16,
    pub flat_surface_y_raw: i16,
    pub elapsed_us: u32,
    pub authored_lifetime_ms: u32,
}

/// Profile-neutral inputs to E370's below-75-percent class-42 RNG gate.
///
/// Callers supply the live post-`FUN_00413F70` forward column and the selected
/// model's Section-8 `+0x08` extent. The helper consumes no sound-gate word;
/// that later branch remains with the owning actor profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorSurfaceBubbleFrame {
    pub entity_id: u32,
    pub entity_type: u8,
    pub state_flags: u32,
    pub position_raw: [i16; 3],
    pub emission_axis_q31: [i32; 3],
    pub active_model_extent_raw: u16,
}

/// Raw request packet supplied to retail particle allocator `FUN_00440A60`.
///
/// Class 42's descriptor adds its own `+0x190` Y velocity after this packet is
/// accepted, so the argument retained here deliberately has zero Y velocity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorSurfaceBubbleRequest {
    pub position_raw: [i16; 3],
    pub velocity_argument_raw: [i16; 3],
    pub owner_entity_id: u32,
    pub owner_entity_type: u8,
    pub suppresses_impact_damage: bool,
}

/// Accepted E370 bubble gate. Owning this non-copyable receipt proves that
/// the one gate word has already been consumed; payload inputs are read only
/// after success, and payload construction cannot repeat the gate.
#[derive(Debug, PartialEq, Eq)]
pub struct ActorSurfaceBubbleGate {
    _private: (),
}

/// State and position used by E370's independent sound branch. The branch
/// does not read the body basis or selected model extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorSurfaceSoundFrame {
    pub state_flags: u32,
    pub position_raw: [i16; 3],
}

impl From<Type9SurfaceFrame> for ActorSurfaceSoundFrame {
    fn from(frame: Type9SurfaceFrame) -> Self {
        Self {
            state_flags: frame.state_flags,
            position_raw: frame.position_raw,
        }
    }
}

/// Fixed positional sound request produced by the owner's independent gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type9SurfaceSoundRequest {
    pub sound_id: u16,
    pub position_raw: [i16; 3],
}

/// Synchronous lifecycle calls which must run before retail continues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type9LifecycleRequest {
    pub entity_id: u32,
}

/// Whether the bounded planner reached a normal return or its live-owner seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type9SurfaceFlow {
    Complete,
    RequiresLifecycleContinuation(Type9LifecycleRequest),
}

/// Complete observable result up to the bounded lifecycle seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type9SurfacePlan {
    pub runtime_after: Type9SurfaceRuntime,
    pub bubble: Option<ActorSurfaceBubbleRequest>,
    pub sound: Option<Type9SurfaceSoundRequest>,
    pub flow: Type9SurfaceFlow,
    /// Every statically recovered `FUN_0040E370` return path returns zero.
    pub callback_result: u32,
}

/// Authored invariants required before state or shared RNG may be touched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorSurfacePlanError {
    ZeroAuthoredLifetime,
}

/// Profile-neutral inputs which select one exact E370 timer phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorSurfaceTimerFrame {
    pub state_flags: u32,
    pub position_y_raw: i16,
    pub active_model_extent_raw: u16,
    pub flat_surface_y_raw: i16,
    pub elapsed_us: u32,
    pub authored_lifetime_ms: u32,
}

/// Exact E370 boundary reached before any shared-RNG draw or lifecycle call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorSurfaceTimerPhase {
    OwnerDisabled,
    NonDeep {
        timer_after_ms: u32,
    },
    DeepBeforeRandomEffects {
        timer_after_ms: u32,
        remaining_percent: u32,
    },
    DeepRandomEffects {
        timer_after_ms: u32,
        remaining_percent: u32,
    },
    DeepLifecycle {
        timer_after_ms: u32,
        /// `FUN_004162B0` computes this after the synchronous lifecycle pair
        /// with wrapping 32-bit multiplication and unsigned division. Exact
        /// expiry is zero; a normal overshoot is a large positive percentage.
        remaining_percent_after_lifecycle: u32,
    },
}

/// Saturating non-deep timer branch shared by `FUN_0040E370` actor profiles.
///
/// The callback truncates each frame independently to milliseconds.  This
/// helper is profile-neutral: the ordinary Type-9 and authenticated Type-17
/// owners select the same branch even though their authored lifetimes differ.
pub fn decay_actor_surface_timer_ms(timer_ms_at_0x48: u32, elapsed_us: u32) -> u32 {
    timer_ms_at_0x48.saturating_sub(elapsed_us / 1_000)
}

/// Classify the authored `+0x72=1,+0x73=0` E370 timer path.
///
/// The result stops at the first boundary which would consume shared RNG or
/// invoke synchronous lifecycle work. This keeps outer owners free to bind
/// those effects independently while sharing the exact threshold arithmetic.
pub fn classify_actor_surface_timer_phase(
    timer_ms_at_0x48: u32,
    frame: ActorSurfaceTimerFrame,
) -> Result<ActorSurfaceTimerPhase, ActorSurfacePlanError> {
    if frame.authored_lifetime_ms == 0 {
        return Err(ActorSurfacePlanError::ZeroAuthoredLifetime);
    }
    if frame.state_flags & ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT != 0 {
        return Ok(ActorSurfaceTimerPhase::OwnerDisabled);
    }

    let deep_threshold =
        i32::from(frame.flat_surface_y_raw) - i32::from(frame.active_model_extent_raw >> 2);
    if i32::from(frame.position_y_raw) >= deep_threshold {
        return Ok(ActorSurfaceTimerPhase::NonDeep {
            timer_after_ms: decay_actor_surface_timer_ms(timer_ms_at_0x48, frame.elapsed_us),
        });
    }

    let timer_after_ms = timer_ms_at_0x48.wrapping_add(frame.elapsed_us / 1_000);
    let remaining_bits = frame.authored_lifetime_ms.wrapping_sub(timer_after_ms);
    let remaining_ms = remaining_bits as i32;
    let remaining_percent = remaining_bits.wrapping_mul(100) / frame.authored_lifetime_ms;
    if remaining_ms <= 0 {
        return Ok(ActorSurfaceTimerPhase::DeepLifecycle {
            timer_after_ms,
            remaining_percent_after_lifecycle: remaining_percent,
        });
    }
    if remaining_percent >= 75 {
        Ok(ActorSurfaceTimerPhase::DeepBeforeRandomEffects {
            timer_after_ms,
            remaining_percent,
        })
    } else {
        Ok(ActorSurfaceTimerPhase::DeepRandomEffects {
            timer_after_ms,
            remaining_percent,
        })
    }
}

/// Plan E370's shared class-42 gate and allocator packet.
///
/// The caller must enter only after the strict deep-water test has produced a
/// remaining percentage below 75. A miss consumes the gate word only; a hit
/// consumes that word followed by X/Y/Z jitter and X/Z velocity words.
pub fn plan_actor_surface_bubble(
    remaining_percent: u32,
    frame: ActorSurfaceBubbleFrame,
    next_random: &mut impl FnMut() -> u32,
) -> Option<ActorSurfaceBubbleRequest> {
    gate_actor_surface_bubble(remaining_percent, next_random)
        .map(|gate| gate.into_request(frame, next_random))
}

/// Consume only E370's first bubble word, before any accepted-payload lookup.
pub fn gate_actor_surface_bubble(
    remaining_percent: u32,
    next_random: &mut impl FnMut() -> u32,
) -> Option<ActorSurfaceBubbleGate> {
    debug_assert!(remaining_percent < 75);
    let bubble_divisor = (remaining_percent / 4).max(2);
    if (next_random() & 0xffff) % bubble_divisor != 0 {
        return None;
    }
    Some(ActorSurfaceBubbleGate { _private: () })
}

impl ActorSurfaceBubbleGate {
    /// Consume the five accepted-payload words after the caller resolves the
    /// live allocation's position/basis and the cached model extent.
    pub fn into_request(
        self,
        frame: ActorSurfaceBubbleFrame,
        next_random: &mut impl FnMut() -> u32,
    ) -> ActorSurfaceBubbleRequest {
        let position_jitter: [i16; 3] = std::array::from_fn(|_| {
            let word = next_random() as u16;
            ((word >> 9) & 0x7f) as i16
        });
        let velocity_x = (((next_random() as u16) >> 7) & 0x1ff) as i16;
        let velocity_z = (((next_random() as u16) >> 7) & 0x1ff) as i16;
        let position_raw = std::array::from_fn(|axis| {
            frame.position_raw[axis]
                .wrapping_add(q31_extent_offset_raw(
                    frame.emission_axis_q31[axis],
                    frame.active_model_extent_raw,
                ))
                .wrapping_add(position_jitter[axis])
        });
        ActorSurfaceBubbleRequest {
            position_raw,
            velocity_argument_raw: [velocity_x, 0, velocity_z],
            owner_entity_id: frame.entity_id,
            owner_entity_type: frame.entity_type,
            suppresses_impact_damage: frame.state_flags >> 31 != 0,
        }
    }
}

/// Plan ordinary Type 9's independent underwater sound gate.
///
/// Retail reaches this draw only after the class-42 bubble branch, including
/// any synchronous allocator call, has finished. Keeping it separate lets a
/// live owner retain that effect boundary without changing the complete
/// stateless planner below.
pub(crate) fn plan_ordinary_type9_surface_sound(
    frame: ActorSurfaceSoundFrame,
    next_random: &mut impl FnMut() -> u32,
) -> Option<Type9SurfaceSoundRequest> {
    if frame.state_flags != 0
        && frame.state_flags & TYPE9_SURFACE_SOUND_SUPPRESSED_STATE_BIT == 0
        && next_random() & 0x0f == 0
    {
        Some(Type9SurfaceSoundRequest {
            sound_id: TYPE9_UNDERWATER_SOUND_ID,
            position_raw: frame.position_raw,
        })
    } else {
        None
    }
}

/// Plan the full first-world ordinary-Type-9 E370 profile.
///
/// The injected RNG must be the process-shared retail stream. Branches consume
/// exactly the words retail consumes: one bubble gate, five payload words
/// after a hit, then one independent sound word when sound is eligible.
pub fn plan_ordinary_type9_surface(
    runtime: Type9SurfaceRuntime,
    frame: Type9SurfaceFrame,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Type9SurfacePlan, ActorSurfacePlanError> {
    let mut plan = Type9SurfacePlan {
        runtime_after: runtime,
        bubble: None,
        sound: None,
        flow: Type9SurfaceFlow::Complete,
        callback_result: 0,
    };
    let remaining_percent = match classify_actor_surface_timer_phase(
        runtime.lifetime_timer_ms_at_0x48,
        ActorSurfaceTimerFrame {
            state_flags: frame.state_flags,
            position_y_raw: frame.position_raw[1],
            active_model_extent_raw: frame.active_model_extent_raw,
            flat_surface_y_raw: frame.flat_surface_y_raw,
            elapsed_us: frame.elapsed_us,
            authored_lifetime_ms: frame.authored_lifetime_ms,
        },
    )? {
        ActorSurfaceTimerPhase::OwnerDisabled => return Ok(plan),
        ActorSurfaceTimerPhase::NonDeep { timer_after_ms }
        | ActorSurfaceTimerPhase::DeepBeforeRandomEffects { timer_after_ms, .. } => {
            plan.runtime_after.lifetime_timer_ms_at_0x48 = timer_after_ms;
            return Ok(plan);
        }
        ActorSurfaceTimerPhase::DeepLifecycle { timer_after_ms, .. } => {
            plan.runtime_after.lifetime_timer_ms_at_0x48 = timer_after_ms;
            plan.flow = Type9SurfaceFlow::RequiresLifecycleContinuation(Type9LifecycleRequest {
                entity_id: frame.entity_id,
            });
            return Ok(plan);
        }
        ActorSurfaceTimerPhase::DeepRandomEffects {
            timer_after_ms,
            remaining_percent,
        } => {
            plan.runtime_after.lifetime_timer_ms_at_0x48 = timer_after_ms;
            remaining_percent
        }
    };

    plan.bubble = plan_actor_surface_bubble(
        remaining_percent,
        ActorSurfaceBubbleFrame {
            entity_id: frame.entity_id,
            entity_type: frame.entity_type,
            state_flags: frame.state_flags,
            position_raw: frame.position_raw,
            emission_axis_q31: frame.emission_axis_q31,
            active_model_extent_raw: frame.active_model_extent_raw,
        },
        next_random,
    );

    plan.sound = plan_ordinary_type9_surface_sound(frame.into(), next_random);

    Ok(plan)
}

fn q31_extent_offset_raw(axis_q31: i32, extent_raw: u16) -> i16 {
    ((i64::from(axis_q31) * i64::from(extent_raw)) >> 31) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENTITY_ID: u32 = 0x04fc_0001;
    const LIFETIME_MS: u32 = 1_000;

    fn frame() -> Type9SurfaceFrame {
        Type9SurfaceFrame {
            entity_id: ENTITY_ID,
            entity_type: 9,
            state_flags: 1,
            position_raw: [1_000, -251, -2_000],
            emission_axis_q31: [i32::MAX, -0x4000_0000, i32::MIN],
            active_model_extent_raw: 1_000,
            flat_surface_y_raw: 0,
            elapsed_us: 0,
            authored_lifetime_ms: LIFETIME_MS,
        }
    }

    fn plan_with_words(
        runtime: Type9SurfaceRuntime,
        frame: Type9SurfaceFrame,
        words: &[u32],
    ) -> (Type9SurfacePlan, usize) {
        let mut consumed = 0;
        let mut next = || {
            let word = words[consumed];
            consumed += 1;
            word
        };
        let plan = plan_ordinary_type9_surface(runtime, frame, &mut next).unwrap();
        (plan, consumed)
    }

    #[test]
    fn disabled_owner_is_a_total_no_op() {
        let runtime = Type9SurfaceRuntime {
            lifetime_timer_ms_at_0x48: 123,
        };
        let mut input = frame();
        input.state_flags |= ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT;
        input.elapsed_us = 20_000;
        let (plan, consumed) = plan_with_words(runtime, input, &[]);
        assert_eq!(plan.runtime_after, runtime);
        assert_eq!(plan.flow, Type9SurfaceFlow::Complete);
        assert_eq!(plan.bubble, None);
        assert_eq!(plan.sound, None);
        assert_eq!(consumed, 0);
    }

    #[test]
    fn deep_threshold_is_strict_and_non_deep_timer_saturates() {
        let runtime = Type9SurfaceRuntime {
            lifetime_timer_ms_at_0x48: 10,
        };
        let mut input = frame();
        input.position_raw[1] = -250;
        input.elapsed_us = 11_999;
        let (at_threshold, consumed) = plan_with_words(runtime, input, &[]);
        assert_eq!(at_threshold.runtime_after.lifetime_timer_ms_at_0x48, 0);
        assert_eq!(consumed, 0);

        input.position_raw[1] = -251;
        let (one_below, consumed) = plan_with_words(runtime, input, &[]);
        assert_eq!(one_below.runtime_after.lifetime_timer_ms_at_0x48, 21);
        assert_eq!(consumed, 0, "98% remaining returns before the RNG gate");
    }

    #[test]
    fn elapsed_time_truncates_to_whole_milliseconds() {
        let runtime = Type9SurfaceRuntime {
            lifetime_timer_ms_at_0x48: 7,
        };
        let mut input = frame();
        input.position_raw[1] = -250;
        input.elapsed_us = 999;
        let (plan, consumed) = plan_with_words(runtime, input, &[]);
        assert_eq!(plan.runtime_after, runtime);
        assert_eq!(consumed, 0);
    }

    #[test]
    fn seventy_five_percent_stops_before_rng_and_seventy_four_uses_divisor_18() {
        let runtime = Type9SurfaceRuntime {
            lifetime_timer_ms_at_0x48: 250,
        };
        let (at_75, consumed) = plan_with_words(runtime, frame(), &[]);
        assert_eq!(at_75.bubble, None);
        assert_eq!(consumed, 0);

        let runtime = Type9SurfaceRuntime {
            lifetime_timer_ms_at_0x48: 260,
        };
        let (at_74, consumed) = plan_with_words(runtime, frame(), &[18, 0, 0, 0, 0, 0, 1]);
        assert!(at_74.bubble.is_some(), "18 modulo 18 must pass");
        assert_eq!(
            consumed, 7,
            "accepted bubble plus eligible sound consumes seven words"
        );
    }

    #[test]
    fn failed_bubble_gate_still_consumes_the_independent_sound_word() {
        let runtime = Type9SurfaceRuntime {
            lifetime_timer_ms_at_0x48: 900,
        };
        let (plan, consumed) = plan_with_words(runtime, frame(), &[1, 0x20]);
        assert_eq!(plan.bubble, None);
        assert_eq!(
            plan.sound,
            Some(Type9SurfaceSoundRequest {
                sound_id: TYPE9_UNDERWATER_SOUND_ID,
                position_raw: frame().position_raw,
            })
        );
        assert_eq!(consumed, 2);
    }

    #[test]
    fn accepted_bubble_preserves_rng_order_q31_offsets_and_wrapping() {
        let runtime = Type9SurfaceRuntime {
            lifetime_timer_ms_at_0x48: 990,
        };
        let mut input = frame();
        input.state_flags = 0x8000_0001;
        input.position_raw = [i16::MAX - 10, -30_000, i16::MIN + 10];
        let words = [
            0,        // divisor-two gate
            127 << 9, // X jitter
            2 << 9,   // Y jitter
            126 << 9, // Z jitter
            511 << 7, // X velocity
            257 << 7, // Z velocity
            1,        // sound miss
        ];
        let (plan, consumed) = plan_with_words(runtime, input, &words);
        let bubble = plan.bubble.unwrap();
        assert_eq!(
            bubble.position_raw,
            [
                input.position_raw[0].wrapping_add(999).wrapping_add(127),
                input.position_raw[1].wrapping_sub(500).wrapping_add(2),
                input.position_raw[2].wrapping_sub(1_000).wrapping_add(126),
            ]
        );
        assert_eq!(bubble.velocity_argument_raw, [511, 0, 257]);
        assert_eq!(bubble.owner_entity_id, ENTITY_ID);
        assert_eq!(bubble.owner_entity_type, 9);
        assert!(bubble.suppresses_impact_damage);
        assert_eq!(plan.sound, None);
        assert_eq!(consumed, 7);
    }

    #[test]
    fn zero_and_model_bit_suppress_sound_without_suppressing_bubble_rng() {
        let runtime = Type9SurfaceRuntime {
            lifetime_timer_ms_at_0x48: 990,
        };
        for state_flags in [0, TYPE9_SURFACE_SOUND_SUPPRESSED_STATE_BIT] {
            let mut input = frame();
            input.state_flags = state_flags;
            let (plan, consumed) = plan_with_words(runtime, input, &[0, 0, 0, 0, 0, 0]);
            assert!(plan.bubble.is_some());
            assert_eq!(plan.sound, None);
            assert_eq!(consumed, 6);
        }
    }

    #[test]
    fn lifecycle_boundary_stops_at_the_explicit_live_owner_seam() {
        let runtime = Type9SurfaceRuntime {
            lifetime_timer_ms_at_0x48: 999,
        };
        let mut input = frame();
        input.elapsed_us = 1_000;
        let (plan, consumed) = plan_with_words(runtime, input, &[]);
        assert_eq!(
            plan.flow,
            Type9SurfaceFlow::RequiresLifecycleContinuation(Type9LifecycleRequest {
                entity_id: ENTITY_ID
            })
        );
        assert_eq!(plan.runtime_after.lifetime_timer_ms_at_0x48, 1_000);
        assert_eq!(plan.bubble, None);
        assert_eq!(plan.sound, None);
        assert_eq!(consumed, 0);
    }

    #[test]
    fn lifecycle_phase_retains_exact_zero_and_wrapped_overshoot_percentages() {
        let input = ActorSurfaceTimerFrame {
            state_flags: 0,
            position_y_raw: -251,
            active_model_extent_raw: 1_000,
            flat_surface_y_raw: 0,
            elapsed_us: 1_000,
            authored_lifetime_ms: 1_000,
        };
        assert_eq!(
            classify_actor_surface_timer_phase(999, input).unwrap(),
            ActorSurfaceTimerPhase::DeepLifecycle {
                timer_after_ms: 1_000,
                remaining_percent_after_lifecycle: 0,
            }
        );
        assert_eq!(
            classify_actor_surface_timer_phase(1_000, input).unwrap(),
            ActorSurfaceTimerPhase::DeepLifecycle {
                timer_after_ms: 1_001,
                remaining_percent_after_lifecycle: u32::MAX.wrapping_mul(100) / 1_000,
            }
        );
    }

    #[test]
    fn zero_authored_lifetime_fails_before_rng() {
        let mut input = frame();
        input.authored_lifetime_ms = 0;
        let mut called = false;
        let result =
            plan_ordinary_type9_surface(Type9SurfaceRuntime::default(), input, &mut || {
                called = true;
                0
            });
        assert_eq!(result, Err(ActorSurfacePlanError::ZeroAuthoredLifetime));
        assert!(!called);
    }
}
