//! Exact detached primary-hit reaction from retail `FUN_00411030`.
//!
//! This is the common impulse/jolt phase reached after an optional
//! behavior-style impact callback and before checked damage.  It deliberately
//! remains detached from live damage dispatch: the caller still owns entity
//! lookup, callback ordering, the process-global RNG stream, and the optional
//! network request.

/// Retail entry point retained as provenance for the detached operation.
pub const IMPACT_REACTION_ADDRESS: u32 = 0x0041_1030;
/// State bit required before retail applies either linear or angular reaction.
pub const IMPACT_REACTION_ENABLED_STATE_BIT: u32 = 0x0400_0000;
/// State bit which suppresses the reaction even when the enable bit is set.
pub const IMPACT_REACTION_SUPPRESSED_STATE_BIT: u32 = 0x0800_0000;
/// The original state word requests `FUN_00469200` after a successful reaction.
pub const IMPACT_REACTION_NETWORKED_STATE_BIT: u32 = 0x8000_0000;
/// Optional network request target called after retail commits all six words.
pub const IMPACT_REACTION_NETWORK_REQUEST_ADDRESS: u32 = 0x0046_9200;
/// First literal argument passed to `FUN_00469200`.
pub const IMPACT_REACTION_NETWORK_REQUEST_CLASS: u32 = 1;
/// Second literal argument passed to `FUN_00469200`.
pub const IMPACT_REACTION_NETWORK_REQUEST_KIND: u32 = 10;

const IMPACT_SCALE_NUMERATOR: u32 = 0x0040_0000;
const RANDOM_SAMPLE_SHIFT: u32 = 5;
const Q15_SHIFT: u32 = 15;
const EULER_JOLT_BIAS_RAW: i16 = 0x0400;

/// Mutable entity words touched by retail `FUN_00411030`.
///
/// The angular array follows the entity's contiguous storage: heading/yaw,
/// pitch, roll. Retail consumes random samples in heading, roll, pitch order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImpactReactionBody {
    /// Exact entity state word at `+0x08`, captured before any reaction.
    pub state_flags_at_0x08: u32,
    /// Unsigned entity mass word at `+0xB0`.
    pub mass_raw_at_0xb0: u16,
    /// Signed 8.8 velocity words at `+0x9C/+0x9E/+0xA0`.
    pub linear_velocity_xyz_raw: [i16; 3],
    /// Wrapping heading/yaw, pitch, and roll words at `+0xA2/+0xA4/+0xA6`.
    pub angular_heading_pitch_roll_raw: [i16; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpactReactionSuppression {
    EnableBitClear,
    SuppressionBitSet,
}

/// A deferred, typed representation of retail
/// `FUN_00469200(1, 10, &entity_handle)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImpactReactionNetworkRequest {
    pub request_class: u32,
    pub request_kind: u32,
    pub entity_handle: u32,
}

impl ImpactReactionNetworkRequest {
    const fn for_entity(entity_handle: u32) -> Self {
        Self {
            request_class: IMPACT_REACTION_NETWORK_REQUEST_CLASS,
            request_kind: IMPACT_REACTION_NETWORK_REQUEST_KIND,
            entity_handle,
        }
    }
}

/// Exact committed deltas and the optional post-commit network request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedImpactReaction {
    pub scale_raw: i32,
    pub linear_delta_xyz_raw: [i16; 3],
    /// Low words of the three process-global RNG returns, in draw order:
    /// heading (`+0xA2`), roll (`+0xA6`), pitch (`+0xA4`).
    pub random_samples_heading_roll_pitch_low16: [u16; 3],
    /// Derived angular deltas in contiguous entity storage order:
    /// heading (`+0xA2`), pitch (`+0xA4`), roll (`+0xA6`).
    pub angular_delta_heading_pitch_roll_raw: [i16; 3],
    pub network_request: Option<ImpactReactionNetworkRequest>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpactReactionOutcome {
    Suppressed(ImpactReactionSuppression),
    Applied(AppliedImpactReaction),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpactReactionError {
    /// Retail would fault while dividing `0x00400000` by the mass word.
    ///
    /// The detached port reports the malformed state before consuming RNG or
    /// mutating any entity word, keeping the error atomic.
    ZeroMass,
}

/// Apply retail `FUN_00411030` to one already-resolved entity.
///
/// `signed_impact_raw` is the signed impact value supplied by the caller and
/// `direction_q15` is the three-word reaction direction.  The RNG callback
/// must be the caller's shared retail stream.  An eligible, nonzero-mass call
/// consumes exactly three samples, including when `signed_impact_raw == 0`.
pub fn apply_impact_reaction(
    body: &mut ImpactReactionBody,
    entity_handle: u32,
    signed_impact_raw: i32,
    direction_q15: [i16; 3],
    mut next_shared_random: impl FnMut() -> u32,
) -> Result<ImpactReactionOutcome, ImpactReactionError> {
    let original_state = body.state_flags_at_0x08;
    if original_state & IMPACT_REACTION_ENABLED_STATE_BIT == 0 {
        return Ok(ImpactReactionOutcome::Suppressed(
            ImpactReactionSuppression::EnableBitClear,
        ));
    }
    if original_state & IMPACT_REACTION_SUPPRESSED_STATE_BIT != 0 {
        return Ok(ImpactReactionOutcome::Suppressed(
            ImpactReactionSuppression::SuppressionBitSet,
        ));
    }
    if body.mass_raw_at_0xb0 == 0 {
        return Err(ImpactReactionError::ZeroMass);
    }

    // The quotient is unsigned-positive in retail, then narrowed to the x86
    // signed multiply used for the caller's signed impact.
    let mass_scale_raw = IMPACT_SCALE_NUMERATOR / u32::from(body.mass_raw_at_0xb0);
    let scale_raw = (mass_scale_raw as i32).wrapping_mul(signed_impact_raw) >> 16;

    let linear_delta_xyz_raw = direction_q15
        .map(|axis_q15| (i32::from(axis_q15).wrapping_mul(scale_raw) >> Q15_SHIFT) as i16);

    // These calls are deliberately not expressed as an array map.  Their
    // order is part of the process-global RNG contract: heading, roll, pitch.
    let random_heading_low16 = next_shared_random() as u16;
    let random_roll_low16 = next_shared_random() as u16;
    let random_pitch_low16 = next_shared_random() as u16;
    let random_samples_heading_roll_pitch_low16 =
        [random_heading_low16, random_roll_low16, random_pitch_low16];
    let angular_heading_raw = random_angular_delta_raw(random_heading_low16, scale_raw);
    let angular_roll_raw = random_angular_delta_raw(random_roll_low16, scale_raw);
    let angular_pitch_raw = random_angular_delta_raw(random_pitch_low16, scale_raw);
    let angular_delta_heading_pitch_roll_raw =
        [angular_heading_raw, angular_pitch_raw, angular_roll_raw];

    let next_linear_velocity = std::array::from_fn(|axis| {
        body.linear_velocity_xyz_raw[axis].wrapping_add(linear_delta_xyz_raw[axis])
    });
    let next_angular = std::array::from_fn(|axis| {
        body.angular_heading_pitch_roll_raw[axis]
            .wrapping_add(angular_delta_heading_pitch_roll_raw[axis])
    });

    // Every random sample and derived linear/angular word is complete before
    // either array is committed. This makes the explicit zero-mass error and
    // all pre-commit derivation atomic with respect to the body.
    body.linear_velocity_xyz_raw = next_linear_velocity;
    body.angular_heading_pitch_roll_raw = next_angular;

    let network_request = (original_state & IMPACT_REACTION_NETWORKED_STATE_BIT != 0)
        .then(|| ImpactReactionNetworkRequest::for_entity(entity_handle));
    Ok(ImpactReactionOutcome::Applied(AppliedImpactReaction {
        scale_raw,
        linear_delta_xyz_raw,
        random_samples_heading_roll_pitch_low16,
        angular_delta_heading_pitch_roll_raw,
        network_request,
    }))
}

fn random_angular_delta_raw(sample: u16, scale_raw: i32) -> i16 {
    let sample_11_bit = i32::from(sample >> RANDOM_SAMPLE_SHIFT);
    let scaled = (sample_11_bit.wrapping_mul(scale_raw) >> Q15_SHIFT) as i16;
    scaled.wrapping_sub(EULER_JOLT_BIAS_RAW)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(state_flags_at_0x08: u32) -> ImpactReactionBody {
        ImpactReactionBody {
            state_flags_at_0x08,
            mass_raw_at_0xb0: 0x20,
            linear_velocity_xyz_raw: [100, -200, 300],
            angular_heading_pitch_roll_raw: [1_000, 2_000, 3_000],
        }
    }

    #[test]
    fn state_gates_suppress_without_mass_validation_or_rng() {
        let mut disabled = body(0);
        disabled.mass_raw_at_0xb0 = 0;
        let original_disabled = disabled;
        let mut disabled_draws = 0;
        assert_eq!(
            apply_impact_reaction(&mut disabled, 7, 123, [1, 2, 3], || {
                disabled_draws += 1;
                0
            }),
            Ok(ImpactReactionOutcome::Suppressed(
                ImpactReactionSuppression::EnableBitClear
            ))
        );
        assert_eq!(disabled, original_disabled);
        assert_eq!(disabled_draws, 0);

        let mut suppressed =
            body(IMPACT_REACTION_ENABLED_STATE_BIT | IMPACT_REACTION_SUPPRESSED_STATE_BIT);
        let original_suppressed = suppressed;
        let mut suppressed_draws = 0;
        assert_eq!(
            apply_impact_reaction(&mut suppressed, 7, 123, [1, 2, 3], || {
                suppressed_draws += 1;
                0
            }),
            Ok(ImpactReactionOutcome::Suppressed(
                ImpactReactionSuppression::SuppressionBitSet
            ))
        );
        assert_eq!(suppressed, original_suppressed);
        assert_eq!(suppressed_draws, 0);
    }

    #[test]
    fn zero_mass_is_an_atomic_explicit_error() {
        let mut entity = body(IMPACT_REACTION_ENABLED_STATE_BIT);
        entity.mass_raw_at_0xb0 = 0;
        let original = entity;
        let mut draws = 0;

        assert_eq!(
            apply_impact_reaction(&mut entity, 0x04fc_0001, 50, [1, 2, 3], || {
                draws += 1;
                0xffff
            }),
            Err(ImpactReactionError::ZeroMass)
        );
        assert_eq!(entity, original);
        assert_eq!(draws, 0);
    }

    #[test]
    fn applies_q15_linear_delta_and_heading_roll_pitch_random_order() {
        let mut entity = body(IMPACT_REACTION_ENABLED_STATE_BIT);
        entity.mass_raw_at_0xb0 = 0x40;
        let mut samples = [0xabcd_2000_u32, 0xbcde_4000, 0xcdef_6000].into_iter();

        let outcome = apply_impact_reaction(
            &mut entity,
            0x04fc_0001,
            0x6000,
            [0x4000, -0x4000, 0x2000],
            || samples.next().expect("exactly three retail draws"),
        )
        .unwrap();

        assert_eq!(
            outcome,
            ImpactReactionOutcome::Applied(AppliedImpactReaction {
                scale_raw: 0x6000,
                linear_delta_xyz_raw: [0x3000, -0x3000, 0x1800],
                random_samples_heading_roll_pitch_low16: [0x2000, 0x4000, 0x6000],
                // sample>>5 gives 256, 512, 768. Retail draws heading, roll,
                // pitch, then stores the angular words heading, pitch, roll.
                angular_delta_heading_pitch_roll_raw: [-832, -448, -640],
                network_request: None,
            })
        );
        assert_eq!(entity.linear_velocity_xyz_raw, [12_388, -12_488, 6_444]);
        assert_eq!(entity.angular_heading_pitch_roll_raw, [168, 1_552, 2_360]);
        assert_eq!(samples.next(), None);
    }

    #[test]
    fn zero_impact_still_draws_three_samples_and_applies_bias() {
        let mut entity = body(IMPACT_REACTION_ENABLED_STATE_BIT);
        let mut draws = 0;

        let outcome = apply_impact_reaction(&mut entity, 9, 0, [0x7fff, -0x8000, 1], || {
            draws += 1;
            0xffff
        })
        .unwrap();

        assert_eq!(draws, 3);
        assert_eq!(
            outcome,
            ImpactReactionOutcome::Applied(AppliedImpactReaction {
                scale_raw: 0,
                linear_delta_xyz_raw: [0, 0, 0],
                random_samples_heading_roll_pitch_low16: [0xffff; 3],
                angular_delta_heading_pitch_roll_raw: [-0x400; 3],
                network_request: None,
            })
        );
        assert_eq!(entity.linear_velocity_xyz_raw, [100, -200, 300]);
        assert_eq!(entity.angular_heading_pitch_roll_raw, [-24, 976, 1_976]);
    }

    #[test]
    fn network_request_uses_original_high_bit_and_exact_call_arguments() {
        let mut entity =
            body(IMPACT_REACTION_ENABLED_STATE_BIT | IMPACT_REACTION_NETWORKED_STATE_BIT);

        let outcome = apply_impact_reaction(&mut entity, 0x04fc_0001, 0, [0; 3], || 0).unwrap();

        let ImpactReactionOutcome::Applied(applied) = outcome else {
            panic!("eligible entity must apply the reaction");
        };
        assert_eq!(
            applied.network_request,
            Some(ImpactReactionNetworkRequest {
                request_class: 1,
                request_kind: 10,
                entity_handle: 0x04fc_0001,
            })
        );
        assert_eq!(
            entity.state_flags_at_0x08,
            IMPACT_REACTION_ENABLED_STATE_BIT | IMPACT_REACTION_NETWORKED_STATE_BIT
        );
    }

    #[test]
    fn signed_scale_and_all_commits_follow_x86_wrapping_order() {
        let mut entity = ImpactReactionBody {
            state_flags_at_0x08: IMPACT_REACTION_ENABLED_STATE_BIT,
            mass_raw_at_0xb0: 1,
            linear_velocity_xyz_raw: [i16::MAX, i16::MIN, -1],
            angular_heading_pitch_roll_raw: [i16::MAX, i16::MIN, -1],
        };
        let mut samples = [0xffff, 0x8000, 0x0020].into_iter();

        let outcome =
            apply_impact_reaction(&mut entity, 1, -0x0101, [i16::MAX, i16::MIN, -1], || {
                samples.next().unwrap()
            })
            .unwrap();

        assert_eq!(
            outcome,
            ImpactReactionOutcome::Applied(AppliedImpactReaction {
                scale_raw: -16_448,
                linear_delta_xyz_raw: [-16_448, 16_448, 0],
                random_samples_heading_roll_pitch_low16: [0xffff, 0x8000, 0x0020],
                angular_delta_heading_pitch_roll_raw: [-2_052, -1_025, -1_538],
                network_request: None,
            })
        );
        assert_eq!(entity.linear_velocity_xyz_raw, [16_319, -16_320, -1]);
        assert_eq!(
            entity.angular_heading_pitch_roll_raw,
            [30_715, 31_743, -1_539]
        );
    }
}
