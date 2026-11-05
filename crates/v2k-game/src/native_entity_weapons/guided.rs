//! Type45 Guided Missile: class28's style `4C81E8`.
//!
//! 44E770's row `0x11` is rocket-shaped (`{1, 1, 0, 45}`): speed 1 plus the
//! shooter's forward closing speed. Class28's initializer `40D160` clears
//! Secondary, publishes the rocket's class31 trail in Tertiary
//! (`4069E0(entity, 2, 0x1F)`) and the Primary `FUN_00407F20`: record `408610`
//! (target, flight time, speed: the integer length of the birth velocity,
//! after `413F70` rebuilt the basis), tick `407FB0`, destructor `4086A0`.
//! Neither publication draws a random word. The style's `+34` sets `0x4000`,
//! so the common update never rebuilds the matrix from the Euler words: the
//! tick steers the matrix itself. Its `+10`, `+18` and `+1C` are `40CEF0`;
//! `+00`, `+04`, `+28` and `+2C` are null; `410C10` takes the alternate
//! class1 (`40BAC0`).

use super::EntityWeaponBlock;
use crate::{
    common_mover::type9_attitude::Type9BodyBasis,
    entity_collision_state::{
        CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
    intro2_gun_turret::task::math::{angle, displacement, q31, rotate},
};
use v2k_formats::collision::SubAPropulsionDescriptor;

pub const GUIDED_MISSILE_ENTITY_TYPE: u32 = 45;
pub const GUIDED_MISSILE_STYLE_ADDRESS: u32 = 0x004c_81e8;
/// Class28's `+34`: no Euler rebuild after the tasks.
pub const GUIDED_MISSILE_STYLE_ENABLE_BITS: u32 = 0x4000;
/// `407FB0` runs `410C10` once the shooter relation is older than this.
pub const GUIDED_MISSILE_LIFETIME_US: u32 = 15_000_000;
/// `4083F0`'s reach, by `425370`'s distance.
pub const GUIDED_MISSILE_SEARCH_RANGE_RAW: i32 = 0x1194;

pub(crate) fn authenticate_guided_missile_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), EntityWeaponBlock> {
    let Some(initializer) = metadata.initializer.as_ref() else {
        return Err(EntityWeaponBlock::Metadata);
    };
    let topology = CommonMoverComponentTopology {
        sub_a: true,
        sub_b: true,
        sub_d: true,
        ..CommonMoverComponentTopology::default()
    };
    if metadata.model_slots != [240; 4]
        || metadata.mass_raw != 10
        || metadata.capability_flags != 0x40
        || metadata.initial_health_raw != Some(1)
        || metadata.common_mover_topology != RetailRuntimeValue::Known(topology)
        || !matches!(
            metadata.sub_a_propulsion_descriptor,
            RetailRuntimeValue::Known(Some(_))
        )
        || initializer.initializer_state_flags_raw != 12
        || initializer.behavior_choices.len() != 1
        || initializer.behavior_choices[0].weight_rule_id != 1
        || initializer.behavior_choices[0].weight_multiplier != 1
        || initializer.behavior_choices[0].behavior_class_id != 28
        || initializer.alternate_behavior_class_ref != 1
    {
        return Err(EntityWeaponBlock::Metadata);
    }
    Ok(())
}

/// `408610`'s 12-byte record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuidedMissileTaskState {
    pub target: Option<u32>,
    /// Microseconds since the current target was taken; zero without one.
    pub flight_us: u32,
    pub speed_raw: i32,
}

impl GuidedMissileTaskState {
    /// `FUN_00408610`: the zeroed record, its speed the `457730` length of the
    /// birth velocity, narrowed to a signed word.
    pub fn new(velocity_raw: [i16; 3]) -> Self {
        let squared = velocity_raw.into_iter().fold(0_u32, |sum, word| {
            sum.wrapping_add((i32::from(word) * i32::from(word)) as u32)
        });
        Self {
            target: None,
            flight_us: 0,
            speed_raw: i32::from(
                v2k_formats::fixed_math::retail_integer_sqrt(squared as i32) as i16
            ),
        }
    }
}

/// One body `4083F0` may take, in the live list's order.
#[derive(Debug, Clone, Copy)]
pub struct GuidedMissileCandidate {
    pub id: u32,
    pub position_raw: [i16; 3],
    /// Live capability word `+64`.
    pub capability_flags: u32,
    /// State `+08` masked by `0xC000`.
    pub living_state_raw: u32,
    /// `411A20(candidate, missile)`: a recent relation either way.
    pub relation_suppressed: bool,
}

fn local_coordinates(vector: [i16; 3], basis: &Type9BodyBasis) -> [i32; 3] {
    [basis.lateral, basis.up, basis.forward].map(|row| {
        (0..3).fold(0_i32, |sum, axis| {
            sum.wrapping_add(q31(i32::from(vector[axis]), row[axis]))
        })
    })
}

/// `FUN_004083F0`: the closest admitted body ahead, scored by forward distance plus
/// twice the lateral and vertical offsets, inside a 45-degree cone.
pub fn search_guided_missile_target(
    own_id: u32,
    position_raw: [i16; 3],
    basis: &Type9BodyBasis,
    candidates: impl IntoIterator<Item = GuidedMissileCandidate>,
) -> Option<u32> {
    let mut best_score = i32::MAX;
    let mut best = None;
    for candidate in candidates {
        if candidate.capability_flags & 9 == 0
            || candidate.living_state_raw != 0x8000
            || candidate.id == own_id
        {
            continue;
        }
        let (vector, distance) = displacement(position_raw, candidate.position_raw);
        if distance >= GUIDED_MISSILE_SEARCH_RANGE_RAW || candidate.relation_suppressed {
            continue;
        }
        let [lateral, up, forward] = local_coordinates(vector, basis);
        if forward <= 0x200 {
            continue;
        }
        let (lateral, up) = (lateral.wrapping_abs(), up.wrapping_abs());
        if lateral < forward && up < forward {
            let score = forward.wrapping_add(lateral.wrapping_add(up).wrapping_mul(2));
            if score < best_score {
                best_score = score;
                best = Some(candidate.id);
            }
        }
    }
    best
}

fn clamp_turn(value: i32, limit: i32) -> i32 {
    if limit < value {
        limit
    } else if value < limit.wrapping_neg() {
        limit.wrapping_neg()
    } else {
        value
    }
}

/// `FUN_00407FB0` after its lifetime test: the roll, the turn toward `target`
/// (its position, or `None` when the record holds no live body), the speed
/// and the new velocity. The caller runs `4083F0` first when the target is
/// gone and passes `None` here.
pub fn steer_guided_missile(
    state: &mut GuidedMissileTaskState,
    basis: &mut Type9BodyBasis,
    position_raw: [i16; 3],
    target_position_raw: Option<[i16; 3]>,
    sub_a: SubAPropulsionDescriptor,
    elapsed_micros: u32,
) -> [i16; 3] {
    let mut distance = 0;
    if let Some(target) = target_position_raw {
        state.flight_us = state.flight_us.wrapping_add(elapsed_micros);
        let (mut vector, length) = displacement(position_raw, target);
        distance = length;
        // Far from it, aim 500 above.
        if state.speed_raw < distance {
            vector[1] = vector[1].wrapping_add(500);
        }
        let limit = (elapsed_micros >> 5) as i32;
        let [lateral, up, forward] = local_coordinates(vector, basis);
        let behind = forward < 0;
        let yaw = clamp_turn(angle(lateral, distance, behind), limit);
        let pitch = clamp_turn(angle(up, distance, behind), limit);
        // 457A90 turns up/forward, 457C30 lateral/forward; the larger turn
        // is whole and the other halved.
        if yaw.wrapping_abs() < pitch.wrapping_abs() {
            rotate(&mut basis.up, &mut basis.forward, pitch.wrapping_neg());
            rotate(
                &mut basis.lateral,
                &mut basis.forward,
                (yaw / 2).wrapping_neg(),
            );
        } else {
            rotate(&mut basis.lateral, &mut basis.forward, yaw.wrapping_neg());
            rotate(
                &mut basis.up,
                &mut basis.forward,
                (pitch / 2).wrapping_neg(),
            );
        }
    }
    let cap = i32::from(sub_a.target_speed_base_raw) / 4;
    if state.speed_raw < cap {
        state.speed_raw = state.speed_raw.wrapping_add(q31(
            i32::from(sub_a.acceleration_raw),
            (elapsed_micros as i32).wrapping_shl(11),
        ));
        if cap < state.speed_raw {
            state.speed_raw = cap;
        }
    }
    let mut velocity = basis.forward.map(|axis| q31(state.speed_raw, axis) as i16);
    // Untargeted, or farther than half its speed: climb by an eighth.
    if state.flight_us == 0 || state.speed_raw / 2 < distance {
        let climb = state.speed_raw / 8;
        for (word, axis) in velocity.iter_mut().zip(basis.up) {
            *word = word.wrapping_add(q31(climb, axis) as i16);
        }
    }
    velocity
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY: Type9BodyBasis = Type9BodyBasis {
        lateral: [i32::MAX, 0, 0],
        up: [0, i32::MAX, 0],
        forward: [0, 0, i32::MAX],
    };
    const SUB_A: SubAPropulsionDescriptor = SubAPropulsionDescriptor {
        acceleration_raw: 4000,
        overspeed_correction_raw: -4000,
        target_speed_base_raw: 10_000,
    };

    fn candidate(id: u32, position_raw: [i16; 3]) -> GuidedMissileCandidate {
        GuidedMissileCandidate {
            id,
            position_raw,
            capability_flags: 8,
            living_state_raw: 0x8000,
            relation_suppressed: false,
        }
    }

    #[test]
    fn search_takes_the_best_scored_body_in_the_forward_cone_and_reach() {
        let found = search_guided_missile_target(
            1,
            [0, 0, 0],
            &IDENTITY,
            [
                candidate(2, [0, 0, 0x200]),    // not past 0x200 ahead
                candidate(3, [3000, 0, 2000]),  // outside the cone
                candidate(4, [0, 0, 4600]),     // beyond 4500
                candidate(5, [400, 400, 3000]), // 2999 + 2 * 798
                candidate(6, [0, 0, 4000]),     // 3999
                candidate(7, [0, 0, -1000]),    // behind
            ],
        );
        assert_eq!(found, Some(6));
        let mut suppressed = candidate(8, [0, 0, 1000]);
        suppressed.relation_suppressed = true;
        let mut cargo = candidate(9, [0, 0, 1000]);
        cargo.capability_flags = 0x40;
        assert_eq!(
            search_guided_missile_target(1, [0; 3], &IDENTITY, [suppressed, cargo]),
            None
        );
    }

    #[test]
    fn untargeted_flight_accelerates_to_a_quarter_of_the_base_and_climbs() {
        let mut state = GuidedMissileTaskState::new([0, 0, 1000]);
        assert_eq!(state.speed_raw, 1000);
        let mut basis = IDENTITY;
        let velocity = steer_guided_missile(&mut state, &mut basis, [0; 3], None, SUB_A, 20_000);
        // q31(4000, 20000 << 11) = 76.
        assert_eq!(state.speed_raw, 1076);
        assert_eq!(velocity, [0, 133, 1075]);
        state.speed_raw = 2490;
        steer_guided_missile(&mut state, &mut basis, [0; 3], None, SUB_A, 20_000);
        assert_eq!(state.speed_raw, 2500);
        assert_eq!(basis, IDENTITY);
    }

    #[test]
    fn a_target_turns_the_larger_axis_whole_capped_by_the_elapsed_time() {
        let mut state = GuidedMissileTaskState::new([0, 0, 2000]);
        state.target = Some(5);
        let mut basis = IDENTITY;
        // Off to the side and near: the yaw is capped at 20000 >> 5 = 625.
        steer_guided_missile(
            &mut state,
            &mut basis,
            [0; 3],
            Some([1000, 0, 1000]),
            SUB_A,
            20_000,
        );
        assert_eq!(state.flight_us, 20_000);
        let mut expected = IDENTITY;
        crate::intro2_gun_turret::task::math::rotate(
            &mut expected.lateral,
            &mut expected.forward,
            -625,
        );
        crate::intro2_gun_turret::task::math::rotate(&mut expected.up, &mut expected.forward, 0);
        assert_eq!(basis, expected);
    }
}
