//! Shared subject-first oriented Section-8 model collision probe.
//!
//! The retail active-pair pass and bounded callback executors must ask the
//! same geometric question.  Keeping the transform/interpreter bridge here
//! prevents a callback-specific executor from drifting into a sphere or
//! proximity approximation while preserving callback/lifecycle policy in the
//! owning adapter.

use crate::active_pair::{
    ActivePairBody, ActivePairContact, ActivePairModel, PairNarrowPhaseOutcome, PairRuntimeField,
};
use crate::entity_collision_state::RetailRuntimeValue;
use v2k_formats::models::{AnimVars, CollisionModelPool, ModelCollisionError};

/// Complete data request for one oriented model-pair probe.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OrientedModelPairProbe<'a> {
    pub subject: &'a ActivePairBody,
    pub subject_model: ActivePairModel,
    pub subject_to_world: RetailRuntimeValue<[[f32; 3]; 3]>,
    pub subject_anim_vars: &'a AnimVars,
    pub candidate: &'a ActivePairBody,
    pub candidate_model: ActivePairModel,
    pub candidate_to_world: RetailRuntimeValue<[[f32; 3]; 3]>,
    pub candidate_anim_vars: &'a AnimVars,
}

/// Run retail's oriented subject-model versus candidate-model narrow phase.
///
/// Positions are wrapping signed 8.8 words and both bases are model-to-world.
/// Missing resources and interpreter gaps remain explicit; an unavailable
/// basis is never replaced with an identity transform.
pub(crate) fn oriented_model_pair_narrow_phase<P: CollisionModelPool + ?Sized>(
    model_pool: &P,
    request: OrientedModelPairProbe<'_>,
) -> PairNarrowPhaseOutcome {
    let OrientedModelPairProbe {
        subject,
        subject_model,
        subject_to_world,
        subject_anim_vars,
        candidate,
        candidate_model,
        candidate_to_world,
        candidate_anim_vars,
    } = request;
    let Some(subject_model_resource) = model_pool.collision_model(subject_model.global_id) else {
        return PairNarrowPhaseOutcome::Unresolved(ModelCollisionError::MissingChild {
            global_id: subject_model.global_id,
        });
    };
    let Some(candidate_model_resource) = model_pool.collision_model(candidate_model.global_id)
    else {
        return PairNarrowPhaseOutcome::Unresolved(ModelCollisionError::MissingChild {
            global_id: candidate_model.global_id,
        });
    };

    let RetailRuntimeValue::Known(candidate_to_world) = candidate_to_world else {
        return PairNarrowPhaseOutcome::RuntimeUnresolved(PairRuntimeField::Orientation);
    };
    let RetailRuntimeValue::Known(subject_to_world) = subject_to_world else {
        return PairNarrowPhaseOutcome::RuntimeUnresolved(PairRuntimeField::Orientation);
    };
    let candidate_to_world = candidate_to_world.map(|row| row.map(f64::from));
    let world_to_candidate = transpose(candidate_to_world);
    let subject_to_world = subject_to_world.map(|row| row.map(f64::from));
    let query_to_candidate = multiply_basis(world_to_candidate, subject_to_world);
    let world_delta_raw = std::array::from_fn(|axis| {
        f64::from(subject.position_raw[axis].wrapping_sub(candidate.position_raw[axis]))
    });
    let query_origin_in_candidate = transform_vector(world_to_candidate, world_delta_raw);

    match subject_model_resource.collide_model_raw_oriented(
        candidate_model_resource,
        query_origin_in_candidate,
        query_to_candidate,
        subject_anim_vars,
        candidate_anim_vars,
        model_pool,
    ) {
        Ok(None) => PairNarrowPhaseOutcome::Miss,
        Ok(Some(hit)) => {
            let normal_world = transform_vector(candidate_to_world, hit.normal);
            PairNarrowPhaseOutcome::Contact(ActivePairContact {
                normal_q12: normal_world.map(|component| (component * 4096.0).round() as i16),
                penetration_raw: hit.penetration_raw as i32,
            })
        }
        Err(source) => PairNarrowPhaseOutcome::Unresolved(source),
    }
}

fn transpose(matrix: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|row| std::array::from_fn(|column| matrix[column][row]))
}

fn transform_vector(matrix: [[f64; 3]; 3], vector: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|row| {
        matrix[row][0] * vector[0] + matrix[row][1] * vector[1] + matrix[row][2] * vector[2]
    })
}

fn multiply_basis(left: [[f64; 3]; 3], right: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            (0..3)
                .map(|axis| left[row][axis] * right[axis][column])
                .sum()
        })
    })
}
