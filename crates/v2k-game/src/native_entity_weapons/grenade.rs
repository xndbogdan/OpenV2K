//! Type59's class35 task and common contact arithmetic.
//!
//! Grenade and Depth Charge inventory selectors share this entity. Their
//! direction, speed and launch cue belong to 44E770's launch request; class35
//! does not retain a weapon-selection mode or consume RNG. These kernels do
//! not allocate/publish actors or execute the 410C10/class49 death transaction.

use crate::actor_task_owner::ActorTaskSlot;
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::damage::DamageProfile;
use crate::entity_collision_state::{
    CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
};
use crate::intro2_meteors::BoulderRollingTaskState;
use crate::terrain_contact::{
    apply_common_solid_terrain_motion_raw, CommonSolidTerrainMotionOutcome, TerrainModelContact,
};
use crate::whole_body_surface::{
    plan_fallback_whole_body_surface_response, WholeBodySurfaceEntryContact,
    WholeBodySurfaceFallbackResponseRequest, WholeBodySurfaceResponse,
};
use v2k_formats::collision::CommonAxisDescriptor;

pub const GRENADE_ENTITY_TYPE: u32 = 59;
pub const GRENADE_MODEL_ID: u16 = 128;
pub const GRENADE_MASS_RAW: u16 = 10;
pub const GRENADE_CAPABILITY_FLAGS: u32 = 0x40;
pub const GRENADE_INITIALIZER_FLAGS: u32 = 8;
pub const GRENADE_INITIAL_HEALTH_RAW: i32 = 1;
pub const GRENADE_COMMON_AXIS_DESCRIPTOR: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 512,
    raw_word_at_0x04: 8,
};
pub const GRENADE_DAMAGE_PROFILE: DamageProfile = DamageProfile {
    thresholds_raw: [0, 2_000, 200, 0, 200, 0, 0],
    multipliers_q8: [0, 256, 256, 512, 128, 0, 0],
};
pub const GRENADE_BEHAVIOR_CLASS: u32 = 35;
pub const GRENADE_ALTERNATE_BEHAVIOR_CLASS: u32 = 49;
pub const GRENADE_STYLE_ADDRESS: u32 = 0x004c_8350;
pub const GRENADE_PRIMARY_LIFETIME_MS: u32 = 2_000;
pub const GRENADE_STANDARD_DEATH_CALLBACK: u32 = 0x0040_cef0;
/// 40B250 commits these clears before 404580 attempts Primary construction.
pub const GRENADE_CLASS35_CLEAR_ORDER: [ActorTaskSlot; 2] =
    [ActorTaskSlot::Tertiary, ActorTaskSlot::Secondary];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrenadeKernelBlock {
    Metadata,
    /// Detailed displaced rolling divides by model header +08, not +0A.
    ZeroModelExtent,
}

/// Authenticate the Type59 fields needed by this componentless class35 owner.
/// This does not prove allocation, callback publication or retail acceptance.
pub fn authenticate_grenade_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), GrenadeKernelBlock> {
    let Some(initializer) = &metadata.initializer else {
        return Err(GrenadeKernelBlock::Metadata);
    };
    if metadata.model_slots != [GRENADE_MODEL_ID; 4]
        || metadata.mass_raw != GRENADE_MASS_RAW
        || metadata.capability_flags != GRENADE_CAPABILITY_FLAGS
        || metadata.initial_health_raw != Some(GRENADE_INITIAL_HEALTH_RAW)
        || metadata.damage_profile != Some(GRENADE_DAMAGE_PROFILE)
        || metadata.death_sound_id != RetailRuntimeValue::Known(Some(62))
        || metadata.common_mover_topology
            != RetailRuntimeValue::Known(CommonMoverComponentTopology::default())
        || initializer.initializer_state_flags_raw != GRENADE_INITIALIZER_FLAGS
        || initializer.common_axis_descriptor != GRENADE_COMMON_AXIS_DESCRIPTOR
        || initializer.behavior_rule_ref != 1
        || initializer.behavior_choices.len() != 1
        || initializer.behavior_choices[0].weight_rule_id != 1
        || initializer.behavior_choices[0].weight_multiplier != 1
        || initializer.behavior_choices[0].behavior_class_id != GRENADE_BEHAVIOR_CLASS
        || initializer.alternate_behavior_class_ref != GRENADE_ALTERNATE_BEHAVIOR_CLASS
    {
        return Err(GrenadeKernelBlock::Metadata);
    }
    Ok(())
}

/// Callback-entry values; invoke before common gravity/wind/master motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrenadePrimaryStepRequest {
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub body_basis: Type9BodyBasis,
    /// Selected model header +08, distinct from contact radius at +0A.
    pub model_extent_raw: u16,
    /// Already selected by the native common-update frame; no extra clamp.
    pub elapsed_micros: u32,
    /// Native 404690 parameter4 == 0.
    pub detailed: bool,
    /// Result of the wrapper's 416410 owner-transition gate.
    pub owner_transition_suppressed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrenadePrimaryOwnerTransition {
    /// Tag 9C00 selects style +04, then 401120 returns its result directly.
    StationaryTagged,
    /// Strict elapsed >2000 selects style +00 after an untagged callback.
    StrictLifetimeExpired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrenadePrimaryStep {
    pub velocity_raw: [i16; 3],
    pub body_basis: Type9BodyBasis,
    pub stationary_tagged: bool,
    /// Arithmetic comparison only; a tag transition returns before this test.
    pub strict_lifetime_expired: bool,
    /// Both style slots select 40CEF0 -> 410C10 for class35.
    pub owner_transition: Option<GrenadePrimaryOwnerTransition>,
}

/// 401120 -> 404690 with class35's 2000ms wrapper lifetime.
///
/// The shared rolling state increments truncated whole milliseconds before
/// comparing prior XYZ, applies detailed basis rotation only on displacement,
/// and skips friction when the strict >10 unchanged-visit branch returns its
/// tag. When stillness and timeout coincide, +04 owns one death call; +00 is
/// not also invoked. A suppressed owner invokes neither transition.
pub fn step_grenade_primary(
    task: &mut BoulderRollingTaskState,
    request: GrenadePrimaryStepRequest,
) -> Result<GrenadePrimaryStep, GrenadeKernelBlock> {
    if request.detailed
        && task.previous_position_raw != request.position_raw
        && request.model_extent_raw == 0
    {
        // Validate a missing model consumer before committing callback words.
        return Err(GrenadeKernelBlock::ZeroModelExtent);
    }
    let mut velocity_raw = request.velocity_raw;
    let mut body_basis = request.body_basis;
    let stationary_tagged = task.step(
        request.position_raw,
        &mut velocity_raw,
        &mut body_basis,
        request.model_extent_raw,
        request.elapsed_micros,
        request.detailed,
    );
    let strict_lifetime_expired = task.elapsed_ms > GRENADE_PRIMARY_LIFETIME_MS;
    let owner_transition = if request.owner_transition_suppressed {
        None
    } else if stationary_tagged {
        Some(GrenadePrimaryOwnerTransition::StationaryTagged)
    } else if strict_lifetime_expired {
        Some(GrenadePrimaryOwnerTransition::StrictLifetimeExpired)
    } else {
        None
    };
    Ok(GrenadePrimaryStep {
        velocity_raw,
        body_basis,
        stationary_tagged,
        strict_lifetime_expired,
        owner_transition,
    })
}

/// Resolved pair callbacks, not a replacement for native pair classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrenadePairContact {
    ActiveEntity,
    StaticGeometry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrenadePairContactPlan {
    pub standard_death_callback: u32,
    /// D920 re-resolves the actor and then runs 11760's common static tail.
    pub common_static_tail_after_callback: bool,
}

/// Class35 style +18/+1C both call 40CEF0; +10/+14 are null.
pub const fn plan_grenade_pair_contact(contact: GrenadePairContact) -> GrenadePairContactPlan {
    GrenadePairContactPlan {
        standard_death_callback: GRENADE_STANDARD_DEATH_CALLBACK,
        common_static_tail_after_callback: matches!(contact, GrenadePairContact::StaticGeometry),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrenadeTerrainContactRequest {
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    /// Contact produced by the selected oriented model's collision spheres.
    pub contact: TerrainModelContact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrenadeTerrainContactPlan {
    pub position_after_raw: [i16; 3],
    pub velocity_after_raw: [i16; 3],
    pub motion: CommonSolidTerrainMotionOutcome,
}

/// D7F0's null style callback continues into 141D0(...,MAX,2,1).
/// The host commits grounded state, separation, inward impact cue, material
/// response, velocity correction and checked channel-1 damage in that order.
/// A bare-terrain contact does not directly select 40CEF0.
pub fn plan_grenade_terrain_contact(
    request: GrenadeTerrainContactRequest,
) -> GrenadeTerrainContactPlan {
    let mut position_after_raw = request.position_raw;
    let mut velocity_after_raw = request.velocity_raw;
    let motion = apply_common_solid_terrain_motion_raw(
        &mut position_after_raw,
        &mut velocity_after_raw,
        request.contact,
    );
    GrenadeTerrainContactPlan {
        position_after_raw,
        velocity_after_raw,
        motion,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrenadeWaterContactRequest {
    /// The above-to-intersecting/below edge is authenticated by 4129B0.
    pub position_raw: [i16; 3],
    pub surface_y_raw: i16,
    /// Rounded nearest-cell material, independently from the wet/dry gate.
    pub material_code: u8,
    pub vertical_velocity_raw: i16,
    pub water_response_selectors: [u8; 8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrenadeWaterContactPlan {
    pub response: Option<WholeBodySurfaceResponse>,
    pub response_origin_raw: [i16; 3],
    /// Sound17 uses current entity XYZ, independently of the water-Y origin.
    pub sound_origin_raw: [i16; 3],
    /// Commit only after the response: hard entry constructs Type60, disposes
    /// its nonzero return, and plays sound17 before this signed Y damping.
    pub vertical_velocity_after_raw: i16,
}

impl GrenadeWaterContactPlan {
    /// Zeroed 0x4C Type60 construction, with this sole model override.
    pub const fn water_ring_model_id(self) -> Option<u16> {
        match self.response {
            Some(WholeBodySurfaceResponse::HardImpact { severe: true }) => Some(130),
            Some(WholeBodySurfaceResponse::HardImpact { severe: false }) => Some(132),
            _ => None,
        }
    }
}

/// D860's null style callback continues into 141D0(...,0,5,0).
/// This is the common water branch, independently of solid normal cancellation.
pub fn plan_grenade_water_contact(request: GrenadeWaterContactRequest) -> GrenadeWaterContactPlan {
    let plan = plan_fallback_whole_body_surface_response(WholeBodySurfaceFallbackResponseRequest {
        contact: WholeBodySurfaceEntryContact {
            position_raw: request.position_raw,
            surface_y_raw: request.surface_y_raw,
            material_code: request.material_code,
        },
        vertical_velocity_raw: request.vertical_velocity_raw,
        water_response_selectors: request.water_response_selectors,
    });
    GrenadeWaterContactPlan {
        response: plan.response,
        response_origin_raw: [
            request.position_raw[0],
            request.surface_y_raw,
            request.position_raw[2],
        ],
        sound_origin_raw: request.position_raw,
        vertical_velocity_after_raw: plan.vertical_velocity_after_raw,
    }
}

#[cfg(test)]
mod tests;
