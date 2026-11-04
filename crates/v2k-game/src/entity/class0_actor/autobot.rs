//! Type64, the Autobot (Defender Drone), on the class0 constructor in class53.
//!
//! Operation `0x38` births it at the player and puts it in the controller's
//! `+220` list. `FUN_00409A80` builds its two components: Sub-D
//! (`FUN_004203D0`, one process stagger seed) and Sub-E (`FUN_00424E30`,
//! zeroed with method 1 and sound 88; it draws nothing). Class53's
//! initializer `FUN_0040B200` clears Tertiary then Secondary and
//! `FUN_004071F0` builds the Primary (tick `FUN_00407280`, no timeout); none
//! of it draws a random word.
//!
//! Each visit, while the owner (`+60`) resolves, zeroes the velocity and
//! searches (`FUN_00422CD0`, the component table's `+28` copy of the type's
//! range and capability mask). With a target its goal is the owner's position
//! less 300 along the unit vector from the target to the Autobot; its pitch
//! eases a quarter toward level, Sub-D (`FUN_0041F660`) steers heading, roll
//! and pitch on its own matrix, and Sub-E fires (`FUN_00424650`) with the
//! owner in each shot's command. With no target it eases each Euler word a
//! quarter of the way to the owner's and aims 350 along the owner's lateral
//! row and 150 along its up row. Either way it then closes on the goal by
//! q31(elapsed << 13, goal - position) per axis. Every visit damps the roll.

use super::*;
use crate::common_mover::sub_d::{
    integrate_yaw_step, target_body_projection_raw, target_pitch_rate, target_steering_rate,
    NativeSubDConstruction,
};
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::generic_projectile_emitter::{GenericEmitterAppendRequest, GenericEmitterRuntime};
use crate::hover::q31_mul;
use crate::ordinary_type47_shot_math::normalize_retail_vector_q31;
use crate::search_attack::{
    select_search_attack_target, SearchAttackAcquisitionRequest, SearchAttackCandidateFilter,
    SearchAttackRadius, SearchAttackTargetSelection,
};
use v2k_formats::collision::{ProjectileEmitterDescriptor, SubDSteeringDescriptor};

mod emitter;
#[cfg(test)]
mod tests;

pub(crate) const ENTITY_TYPE: u32 = 64;
pub(crate) const BEHAVIOR_CLASS: u32 = 53;
const MODEL: u16 = 248;

const SUB_D: SubDSteeringDescriptor = SubDSteeringDescriptor {
    steering_divisor_raw: 16,
    couple_yaw_into_roll_raw: 1,
    enable_pitch_steering_raw: 1,
    forward_probe_raw: 0,
    lateral_probe_raw: 0,
    classifier_flags: 0,
    reserved_at_0x0b: 0,
};

const EMITTER: ProjectileEmitterDescriptor = ProjectileEmitterDescriptor {
    projectile_method: 1,
    random_interval_us: 200_000,
    spread_raw: 1,
    aim_threshold_raw: 3000,
    speed_override_raw: 0,
    target_axis_tolerance_raw: 1536,
    sound_id: 88,
    raw_word_at_0x12: 0,
    alternate_emitter_raw: 0,
    stochastic_gate_mode: 0,
    auxiliary_command: 1,
    variable_bindings: [0; 4],
};

/// Sub-D's steering words, which `FUN_004203D0` clears: the yaw and pitch
/// rates (`+04`, `+08`) and the steps `FUN_00420360` last applied (`+10`,
/// `+14`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct AutobotSteering {
    yaw_rate: i32,
    pitch_rate: i32,
    yaw_step: i32,
    pitch_step: i32,
}

/// The Autobot's component allocations kept beside its class0 runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AutobotComponents {
    pub(crate) sub_d: crate::common_mover::sub_d::Type9SubDFrameOwner,
    pub(crate) sub_d_seed: u8,
    steering: AutobotSteering,
    /// Sub-E's cadence and selector words.
    sub_e: GenericEmitterRuntime,
    /// The body's `+40` transient list, which `FUN_00411400` drains.
    shots: Vec<GenericEmitterAppendRequest>,
    next_transaction: u64,
}

/// `FUN_00424E30`: the 0x3C bytes zeroed, then the method and sound words.
const SUB_E_AT_BIRTH: GenericEmitterRuntime = GenericEmitterRuntime {
    joint_bindings: [None; 2],
    projectile_method: EMITTER.projectile_method,
    emitter_selector: 0,
    sound_id: EMITTER.sound_id as u32,
    direct_mode: 0,
    remaining_time_raw: 0,
    manual_step_raw: 0,
    remaining_bursts_raw: 0,
    cadence_raw: 0,
    basis_adjustment_identity: None,
};

/// Type64's Section-12 row in every world: model 248, mass 10, capability
/// `0x40`, health 3000, state `0x84`, range 3072 with mask `0x18`, class53
/// with alternate class11, death cue 62, Sub-D and Sub-E alone.
pub(super) fn authenticate_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Class0ActorError> {
    use crate::entity_collision_state::{CommonMoverGklPayloads, CommonWorldEffectProfile};
    let Some(initializer) = &metadata.initializer else {
        return Err(Class0ActorError::Metadata);
    };
    let ok = metadata.model_slots == [MODEL; 4]
        && metadata.mass_raw == 10
        && metadata.capability_flags == 0x40
        && metadata.initial_health_raw == Some(3000)
        && metadata.damage_profile
            == Some(crate::damage::DamageProfile {
                thresholds_raw: [0, 6000, 1000, 0, 200, 0, 0],
                multipliers_q8: [0, 256, 256, 512, 128, 0, 0],
            })
        && metadata.common_world_effects
            == RetailRuntimeValue::Known(CommonWorldEffectProfile::default())
        && metadata.terrain_contact_task_lifetime_ms == RetailRuntimeValue::Known(0)
        && metadata.accepted_hit_presentation_sound_id == RetailRuntimeValue::Known(None)
        && metadata.infected_model_presentation_sound_id == RetailRuntimeValue::Known(None)
        && metadata.cured_model_presentation_sound_id == RetailRuntimeValue::Known(None)
        && metadata.death_sound_id == RetailRuntimeValue::Known(Some(62))
        && metadata.target_warning_sound_id == RetailRuntimeValue::Known(None)
        && metadata.constructor_sound_attachment_id == RetailRuntimeValue::Known(None)
        && metadata.generic_hit_sound_id == RetailRuntimeValue::Known(None)
        && metadata.sub_a_propulsion_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_b_lateral_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_c_lift_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_d_steering_descriptor == RetailRuntimeValue::Known(Some(SUB_D))
        && metadata.sub_f_swimming_descriptor == RetailRuntimeValue::Known(None)
        && metadata.model_variable_count_raw == RetailRuntimeValue::Known(0)
        && metadata.projectile_emitter_descriptor == RetailRuntimeValue::Known(Some(EMITTER))
        && metadata.sub_n_payload.is_none()
        && metadata.status_component_descriptor == RetailRuntimeValue::Known(None)
        && metadata.actor_animation_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_h_external_frame_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_j_attachment_descriptor == RetailRuntimeValue::Known(None)
        && metadata.common_mover_topology
            == RetailRuntimeValue::Known(CommonMoverComponentTopology {
                sub_d: true,
                sub_e: true,
                ..CommonMoverComponentTopology::default()
            })
        && metadata.common_mover_gkl_payloads
            == RetailRuntimeValue::Known(CommonMoverGklPayloads {
                sub_g: None,
                sub_k: None,
                sub_l: None,
            })
        && initializer.initializer_state_flags_raw == 0x84
        && initializer.common_axis_descriptor
            == v2k_formats::collision::CommonAxisDescriptor {
                strict_axis_limit_raw: 3072,
                raw_word_at_0x04: 0x18,
            }
        && initializer.behavior_choices.as_ref()
            == [v2k_formats::collision::BehaviorChoice {
                weight_rule_id: 1,
                weight_multiplier: 1,
                behavior_class_id: BEHAVIOR_CLASS,
            }]
        && initializer.behavior_rule_ref == 1
        && initializer.alternate_behavior_class_ref == 11;
    if ok {
        Ok(())
    } else {
        Err(Class0ActorError::Metadata)
    }
}

/// The no-target goal: the owner's position plus 350 along its lateral row
/// and 150 along its up row, each product truncated to a word.
fn formation_goal(owner_position: [i16; 3], owner_basis: Type9BodyBasis) -> [i16; 3] {
    std::array::from_fn(|axis| {
        owner_position[axis]
            .wrapping_add(q31_mul(owner_basis.up[axis], 150) as i16)
            .wrapping_add(q31_mul(owner_basis.lateral[axis], 350) as i16)
    })
}

/// `(3 * own + owner) / 4`, truncating as retail's signed shift does.
fn ease_quarter(own: i16, owner: i16) -> i16 {
    ((i32::from(own) * 3 + i32::from(owner)) / 4) as i16
}

/// One no-target visit of `FUN_00407280` on the body's words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AutobotPose {
    position: [i16; 3],
    rotation: [i16; 3],
}

#[cfg(test)]
fn formation_step(
    pose: AutobotPose,
    owner_position: [i16; 3],
    owner_rotation: [i16; 3],
    owner_basis: Type9BodyBasis,
    dt: u32,
) -> AutobotPose {
    let rotation = std::array::from_fn(|i| ease_quarter(pose.rotation[i], owner_rotation[i]));
    let goal = formation_goal(owner_position, owner_basis);
    AutobotPose {
        position: closed_on(pose.position, goal, dt),
        rotation,
    }
}

/// Each axis closes by q31(elapsed << 13, goal - position) of the word gap.
fn closed_on(position: [i16; 3], goal: [i16; 3], dt: u32) -> [i16; 3] {
    let rate = dt.wrapping_shl(13) as i32;
    std::array::from_fn(|axis| {
        let gap = goal[axis].wrapping_sub(position[axis]);
        position[axis].wrapping_add(q31_mul(rate, i32::from(gap)) as i16)
    })
}

/// The target goal: the owner's position less 300 times the unit vector
/// (`FUN_00457960`) from the target to the Autobot, each product a word.
fn target_goal(position: [i16; 3], owner_position: [i16; 3], target: [i16; 3]) -> [i16; 3] {
    let unit = normalize_retail_vector_q31(std::array::from_fn(|axis| {
        i32::from(position[axis]) - i32::from(target[axis])
    }));
    std::array::from_fn(|axis| owner_position[axis].wrapping_sub(q31_mul(unit[axis], 300) as i16))
}

/// The target branch's turn: the pitch eased a quarter toward level, then
/// `FUN_0041F660` with no terrain classes: `FUN_0041E700` on the Autobot's
/// own matrix, the yaw and pitch rates (`FUN_00420260`, `FUN_004202E0`, by
/// the visit's elapsed time) and their steps (`FUN_00420360`, by the frame's),
/// the yaw step taken from the heading and, coupled, from the roll.
fn target_turn(
    steering: &mut AutobotSteering,
    pose: AutobotPose,
    target: [i16; 3],
    basis: Type9BodyBasis,
    dt: u32,
    frame_dt: u32,
) -> [i16; 3] {
    let [heading, pitch, roll] = pose.rotation;
    let pitch = ease_quarter(pitch, 0);
    let projection = target_body_projection_raw(pose.position, target, basis);
    let divisor = SUB_D.steering_divisor_raw;
    steering.yaw_rate =
        target_steering_rate(steering.yaw_rate, projection.lateral_raw, dt, divisor);
    steering.pitch_rate =
        target_pitch_rate(steering.pitch_rate, projection.vertical_raw, dt, divisor);
    let yaw_step = integrate_yaw_step(steering.yaw_rate, frame_dt);
    let pitch_step = integrate_yaw_step(steering.pitch_rate, frame_dt);
    steering.yaw_step = i32::from(yaw_step);
    steering.pitch_step = i32::from(pitch_step);
    [
        heading.wrapping_sub(yaw_step),
        pitch.wrapping_sub(pitch_step),
        roll.wrapping_sub(yaw_step),
    ]
}

/// The visit's closing roll damping: `roll -= ((dt >> 10) * roll) >> 8`.
fn damped_roll(roll: i16, dt: u32) -> i16 {
    roll.wrapping_sub((((dt >> 10) as i32).wrapping_mul(i32::from(roll)) >> 8) as i16)
}

impl EntityManager {
    /// `FUN_00409A80`'s Sub-D slot for a new Autobot: one stagger seed.
    pub(crate) fn construct_autobot_components(
        &mut self,
        id: u32,
        metadata: &EntityTypeRuntimeMetadata,
        world_fx: &mut WorldFx,
    ) {
        let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_d_steering_descriptor else {
            unreachable!("authenticated Autobot Sub-D");
        };
        let NativeSubDConstruction {
            seed, frame_owner, ..
        } = world_fx.construct_entity_sub_d(descriptor);
        if let Some(runtime) = self.native_class0_actors.get_mut(&id) {
            runtime.autobot = Some(AutobotComponents {
                sub_d: frame_owner,
                sub_d_seed: seed,
                steering: AutobotSteering::default(),
                sub_e: SUB_E_AT_BIRTH,
                shots: Vec::new(),
                next_transaction: 1,
            });
        }
    }

    /// The owner `+60` resolves to a live allocation.
    fn autobot_owner(&self, id: u32) -> Result<Option<&Entity>, Class0ActorError> {
        let entity = self
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(Class0ActorError::Allocation)?;
        let RetailRuntimeValue::Known(owner) = entity.collision.recent_relation_id_at_0x60 else {
            return Err(Class0ActorError::Runtime("Autobot owner +60"));
        };
        Ok(owner.and_then(|owner| self.iter_all().find(|entity| entity.id == owner)))
    }

    /// `FUN_00422CD0` over the live list with the body's range and mask.
    fn autobot_search(&self, id: u32) -> Result<Option<u32>, Class0ActorError> {
        let entity = self
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(Class0ActorError::Allocation)?;
        let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
            return Err(Class0ActorError::Runtime("Autobot search range"));
        };
        let snapshot = crate::search_attack_live::search_attack_live_list_snapshot(self);
        let owner = *snapshot
            .iter()
            .find(|candidate| candidate.id == id)
            .ok_or(Class0ActorError::Allocation)?;
        let selection = select_search_attack_target(SearchAttackAcquisitionRequest {
            owner,
            candidates_in_intrusive_order: &snapshot,
            radius: SearchAttackRadius::from_raw(axis.strict_axis_limit_raw),
            filter: SearchAttackCandidateFilter::from_raw(axis.raw_word_at_0x04),
        })
        .map_err(|_| Class0ActorError::Runtime("Autobot search"))?;
        Ok(match selection {
            // A zero range never tags; the Autobot's is 3072.
            SearchAttackTargetSelection::Success { output_write } => Some(
                output_write
                    .ok_or(Class0ActorError::Runtime("Autobot unbounded search"))?
                    .id,
            ),
            SearchAttackTargetSelection::TaggedNoBoundedTarget { .. } => None,
        })
    }

    fn autobot_components_mut(
        &mut self,
        id: u32,
    ) -> Result<&mut AutobotComponents, Class0ActorError> {
        self.native_class0_actors
            .get_mut(&id)
            .and_then(|runtime| runtime.autobot.as_mut())
            .ok_or(Class0ActorError::Runtime("Autobot components"))
    }

    /// `FUN_00407280`. `frame_dt` is the frame's elapsed time, which
    /// `FUN_00420360` reads in place of the visit's.
    pub(super) fn autobot_visit(
        &mut self,
        id: u32,
        dt: u32,
        frame_dt: u32,
        world_fx: &mut WorldFx,
    ) -> Result<(), Class0ActorError> {
        let owner = self.autobot_owner(id)?.map(|owner| {
            (
                owner.id,
                owner.position_raw(),
                owner.rotation_heading_pitch_roll_raw(),
                owner.physical_body_basis_q31(),
            )
        });
        if let Some((owner_id, owner_position, owner_rotation, owner_basis)) = owner {
            self.entity_mut(id)
                .ok_or(Class0ActorError::Allocation)?
                .set_velocity_raw([0; 3]);
            let goal = match self.autobot_search(id)? {
                Some(target) => {
                    let target_position = self
                        .iter_all()
                        .find(|entity| entity.id == target)
                        .ok_or(Class0ActorError::Allocation)?
                        .position_raw();
                    let entity = self.entity_mut(id).ok_or(Class0ActorError::Allocation)?;
                    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
                        return Err(Class0ActorError::Runtime("Autobot matrix"));
                    };
                    let pose = AutobotPose {
                        position: entity.position_raw(),
                        rotation: entity.rotation_heading_pitch_roll_raw(),
                    };
                    let goal = target_goal(pose.position, owner_position, target_position);
                    let components = self.autobot_components_mut(id)?;
                    components.sub_d.evidence_for_classifier_free_frame(
                        SUB_D,
                        pose.position,
                        target_position,
                        basis.lateral,
                        basis.forward,
                    );
                    let rotation = target_turn(
                        &mut components.steering,
                        pose,
                        target_position,
                        basis,
                        dt,
                        frame_dt,
                    );
                    self.entity_mut(id)
                        .ok_or(Class0ActorError::Allocation)?
                        .set_rotation_heading_pitch_roll_raw(rotation);
                    self.autobot_fire(id, owner_id, target, dt, world_fx)?;
                    goal
                }
                None => {
                    let RetailRuntimeValue::Known(basis) = owner_basis else {
                        return Err(Class0ActorError::Runtime("Autobot owner matrix"));
                    };
                    let entity = self.entity_mut(id).ok_or(Class0ActorError::Allocation)?;
                    let rotation: [i16; 3] = std::array::from_fn(|i| {
                        ease_quarter(
                            entity.rotation_heading_pitch_roll_raw()[i],
                            owner_rotation[i],
                        )
                    });
                    entity.set_rotation_heading_pitch_roll_raw(rotation);
                    formation_goal(owner_position, basis)
                }
            };
            let entity = self.entity_mut(id).ok_or(Class0ActorError::Allocation)?;
            entity.set_position_raw(closed_on(entity.position_raw(), goal, dt));
        }
        let entity = self.entity_mut(id).ok_or(Class0ActorError::Allocation)?;
        let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
        entity.set_rotation_heading_pitch_roll_raw([heading, pitch, damped_roll(roll, dt)]);
        Ok(())
    }
}
