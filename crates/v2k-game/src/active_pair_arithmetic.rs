//! Exact fixed-point arithmetic used by active-entity pair contact.
//!
//! This is kept separate from the ordered pass policy so the signed-word
//! response can be tested and reused without acquiring callback or lifecycle
//! assumptions.

/// Q12 normal and signed penetration returned by `FUN_0046AE40`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivePairContact {
    pub normal_q12: [i16; 3],
    pub penetration_raw: i32,
}

/// Exact signed-word AABB and squared-sphere broad gate from `FUN_00412530`.
pub fn pair_broad_phase_overlaps_raw(
    subject_position_raw: [i16; 3],
    subject_radius_raw: u16,
    candidate_position_raw: [i16; 3],
    candidate_radius_raw: u16,
) -> bool {
    let radius_raw = u32::from(subject_radius_raw).wrapping_add(u32::from(candidate_radius_raw));
    let diameter_raw = radius_raw.wrapping_mul(2);
    let delta_raw = std::array::from_fn::<_, 3, _>(|axis| {
        i32::from(candidate_position_raw[axis].wrapping_sub(subject_position_raw[axis]))
    });
    if delta_raw
        .iter()
        .any(|delta| (delta.wrapping_add(radius_raw as i32) as u32) > diameter_raw)
    {
        return false;
    }
    let squared_raw = delta_raw.iter().fold(0_i32, |sum, delta| {
        sum.wrapping_add(delta.wrapping_mul(*delta))
    });
    (squared_raw as u32) <= radius_raw.wrapping_mul(radius_raw)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairResponseError {
    ZeroCombinedMovableMass,
}

/// Apply the exact `FUN_00411AD0` velocity branches and `FUN_00412760`
/// mass-weighted separation to two raw bodies.
#[allow(clippy::too_many_arguments)]
pub fn apply_pair_response_raw(
    subject_position_raw: &mut [i16; 3],
    subject_velocity_raw: &mut [i16; 3],
    subject_mass_raw: u16,
    subject_fixed: bool,
    candidate_position_raw: &mut [i16; 3],
    candidate_velocity_raw: &mut [i16; 3],
    candidate_mass_raw: u16,
    candidate_fixed: bool,
    contact: ActivePairContact,
) -> Result<(), PairResponseError> {
    if !subject_fixed && !candidate_fixed {
        let combined_mass_raw = u32::from(subject_mass_raw) + u32::from(candidate_mass_raw);
        if combined_mass_raw == 0 {
            return Err(PairResponseError::ZeroCombinedMovableMass);
        }
        for axis in 0..3 {
            let center_raw = i32::from(subject_mass_raw)
                .wrapping_mul(i32::from(subject_velocity_raw[axis]))
                .wrapping_add(
                    i32::from(candidate_mass_raw)
                        .wrapping_mul(i32::from(candidate_velocity_raw[axis])),
                )
                / combined_mass_raw as i32;
            let center_raw = center_raw as i16;
            let subject_half_raw = ((i32::from(subject_velocity_raw[axis]) << 14) >> 15) as i16;
            let candidate_half_raw = ((i32::from(candidate_velocity_raw[axis]) << 14) >> 15) as i16;
            subject_velocity_raw[axis] = center_raw.wrapping_sub(subject_half_raw);
            candidate_velocity_raw[axis] = center_raw.wrapping_sub(candidate_half_raw);
        }
        let subject_separation_raw = i32::from(candidate_mass_raw)
            .wrapping_mul(contact.penetration_raw)
            / combined_mass_raw as i32;
        let candidate_separation_raw = i32::from(subject_mass_raw)
            .wrapping_mul(contact.penetration_raw)
            / combined_mass_raw as i32;
        separate_pair_raw(
            subject_position_raw,
            candidate_position_raw,
            contact.normal_q12,
            subject_separation_raw,
            candidate_separation_raw,
        );
    } else if !subject_fixed {
        project_fixed_contact_velocity(subject_velocity_raw, contact.normal_q12);
        separate_pair_raw(
            subject_position_raw,
            candidate_position_raw,
            contact.normal_q12,
            contact.penetration_raw,
            0,
        );
    } else {
        // Retail deliberately falls through here when both bodies are fixed.
        project_fixed_contact_velocity(candidate_velocity_raw, contact.normal_q12);
        separate_pair_raw(
            subject_position_raw,
            candidate_position_raw,
            contact.normal_q12,
            0,
            contact.penetration_raw,
        );
    }
    Ok(())
}

fn project_fixed_contact_velocity(velocity_raw: &mut [i16; 3], normal_q12: [i16; 3]) {
    let inward_raw =
        normal_q12
            .iter()
            .zip(velocity_raw.iter())
            .fold(0_i32, |sum, (&normal, &velocity)| {
                sum.wrapping_add(i32::from(normal).wrapping_mul(i32::from(velocity)))
            })
            >> 12;
    for (velocity, &normal) in velocity_raw.iter_mut().zip(&normal_q12) {
        let projection_raw = (i32::from(normal).wrapping_mul(inward_raw) >> 12) as i16;
        *velocity = velocity.wrapping_sub(projection_raw);
    }
    let retained_eighth_raw = (inward_raw.wrapping_shl(12) >> 15) as i16;
    for (velocity, &normal) in velocity_raw.iter_mut().zip(&normal_q12) {
        let retained_axis_raw =
            (i32::from(normal).wrapping_mul(i32::from(retained_eighth_raw)) >> 12) as i16;
        *velocity = velocity.wrapping_add(retained_axis_raw);
    }
}

fn separate_pair_raw(
    subject_position_raw: &mut [i16; 3],
    candidate_position_raw: &mut [i16; 3],
    normal_q12: [i16; 3],
    subject_separation_raw: i32,
    candidate_separation_raw: i32,
) {
    for axis in 0..3 {
        if subject_separation_raw != 0 {
            let correction_raw =
                (i32::from(normal_q12[axis]).wrapping_mul(subject_separation_raw) >> 12) as i16;
            subject_position_raw[axis] = subject_position_raw[axis].wrapping_add(correction_raw);
        }
        if candidate_separation_raw != 0 {
            let correction_raw =
                (i32::from(normal_q12[axis]).wrapping_mul(candidate_separation_raw) >> 12) as i16;
            candidate_position_raw[axis] =
                candidate_position_raw[axis].wrapping_sub(correction_raw);
        }
    }
}

/// Active-pair impact term from `FUN_00411AD0`.
///
/// Unlike one-body `FUN_00411760`, each old/new `i16` velocity is widened
/// before subtraction. The squares, sum, shifts, and mass product still wrap
/// in signed 32-bit order.
pub fn pair_velocity_change_impact_raw(
    velocity_before_raw: [i16; 3],
    velocity_after_raw: [i16; 3],
    mass_raw: u16,
) -> i32 {
    let squared_delta_raw = (0..3).fold(0_i32, |sum, axis| {
        let delta_raw =
            i32::from(velocity_after_raw[axis]).wrapping_sub(i32::from(velocity_before_raw[axis]));
        sum.wrapping_add(delta_raw.wrapping_mul(delta_raw))
    });
    (squared_delta_raw >> 7).wrapping_mul(i32::from(mass_raw)) >> 10
}
