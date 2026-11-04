//! Type64, the Autobot (Defender Drone), on the class0 constructor in class53.
//!
//! Operation `0x38` births it at the player and puts it in the controller's
//! `+220` list. `FUN_00409A80` builds its two components: Sub-D
//! (`FUN_004203D0`, one process stagger seed) and Sub-E (`FUN_00424E30`).
//! Class53's initializer `FUN_0040B200` clears Tertiary then Secondary and
//! `FUN_004071F0` builds the Primary (tick `FUN_00407280`, no timeout); none
//! of it draws a random word.
//!
//! Each visit, while the owner (`+60`) resolves, zeroes the velocity and
//! searches (`FUN_00422CD0`, the component table's `+28` copy of the type's
//! range and capability mask). With no target it eases each Euler word a
//! quarter of the way to the owner's and aims 350 along the owner's lateral
//! row and 150 along its up row; it then closes on that goal by
//! q31(elapsed << 13, goal - position) per axis. Every visit damps the roll.
//! The target branch (Sub-D steering with pitch, the emitter) is held: a
//! visit that finds a target stops before it begins.

use super::*;
use crate::common_mover::sub_d::NativeSubDConstruction;
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::hover::q31_mul;
use crate::search_attack::{
    select_search_attack_target, SearchAttackAcquisitionRequest, SearchAttackCandidateFilter,
    SearchAttackRadius, SearchAttackTargetSelection,
};
use v2k_formats::collision::{ProjectileEmitterDescriptor, SubDSteeringDescriptor};

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

/// The Autobot's component allocations kept beside its class0 runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AutobotComponents {
    pub(crate) sub_d: crate::common_mover::sub_d::Type9SubDFrameOwner,
    pub(crate) sub_d_seed: u8,
}

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

    /// Whether this visit would take the held target branch. The search reads
    /// no random word and writes nothing, so it runs ahead of the visit.
    pub(super) fn autobot_target_held(&self, id: u32) -> Result<(), Class0ActorError> {
        if self.autobot_owner(id)?.is_none() {
            return Ok(());
        }
        match self.autobot_search(id)? {
            None => Ok(()),
            Some(_) => Err(Class0ActorError::Runtime(
                "Autobot target branch held (Sub-D pitch steering and the emitter)",
            )),
        }
    }

    /// `FUN_00407280` without a target, the only branch the port runs.
    pub(super) fn autobot_visit(&mut self, id: u32, dt: u32) -> Result<(), Class0ActorError> {
        let owner = self.autobot_owner(id)?.map(|owner| {
            (
                owner.position_raw(),
                owner.rotation_heading_pitch_roll_raw(),
                owner.physical_body_basis_q31(),
            )
        });
        if owner.is_some() && self.autobot_search(id)?.is_some() {
            return Err(Class0ActorError::Runtime("Autobot target branch held"));
        }
        let entity = self.entity_mut(id).ok_or(Class0ActorError::Allocation)?;
        if let Some((position, rotation, basis)) = owner {
            let RetailRuntimeValue::Known(basis) = basis else {
                return Err(Class0ActorError::Runtime("Autobot owner matrix"));
            };
            entity.set_velocity_raw([0; 3]);
            let pose = formation_step(
                AutobotPose {
                    position: entity.position_raw(),
                    rotation: entity.rotation_heading_pitch_roll_raw(),
                },
                position,
                rotation,
                basis,
                dt,
            );
            entity.set_position_raw(pose.position);
            entity.set_rotation_heading_pitch_roll_raw(pose.rotation);
        }
        let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
        entity.set_rotation_heading_pitch_roll_raw([heading, pitch, damped_roll(roll, dt)]);
        Ok(())
    }
}
