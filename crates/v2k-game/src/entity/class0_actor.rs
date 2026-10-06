//! Shared native class0 objects: common birth, C490 timer and living reselection.
//!
//! These instances use current Section12/13 records in ordinary worlds and Intro2.
//! Type52/68 quiet death, Type54 Change-Sea-Level abort and Type111 gates authenticate the
//! same constructor receipt; captured Level1 coordinates, model numbers and
//! task omissions do not.

use super::*;
use crate::{
    class0_timer::Class0TimerTaskState,
    entity_behavior::{select_initial_behavior, BehaviorWeightRule},
    entity_initializer::resolve_entity_initializer_with_selected_behavior,
};
use v2k_formats::levels::EntitySpawn;

pub(super) mod cargo;
#[cfg(test)]
mod cargo_tests;
mod live;
mod zero_record;
pub(crate) use zero_record::NativeType68ZeroRecordConstruction;
#[cfg(test)]
mod tests;
pub use live::{
    tick_class0_actor_owner, Class0ActorFrame, Class0ActorOutcome, Class0ActorOwner,
    Class0ActorTick,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class0ActorError {
    Allocation,
    Metadata,
    Graph,
    Runtime(&'static str),
}

#[derive(Debug, Clone, Copy)]
pub(super) struct NativeClass0Runtime {
    allocation: MainBaseAbortActorLease,
    entity_type: u32,
    model_slots: [Option<usize>; 4],
    /// D4A0's post-grounding +90 copy, distinct from later current position.
    anchor_raw: [i16; 3],
    pending_prefix: bool,
    /// 416FF0's +80=self word is distinct from a SubJ/cargo attachment.
    pub(super) gate_self_relation: Option<u32>,
}

pub(super) fn authenticate_metadata(
    entity_type: u32,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Class0ActorError> {
    match entity_type {
        52 | 68 => {
            let profile = fresh_level_one_quiet_death_profile(entity_type)
                .ok_or(Class0ActorError::Metadata)?;
            if !quiet_death_metadata_matches(metadata, profile)
                || metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
            {
                return Err(Class0ActorError::Metadata);
            }
            Ok(())
        }
        54 => {
            if crate::main_base_type54_abort::exact_level_one_type54_metadata(metadata) {
                Ok(())
            } else {
                Err(Class0ActorError::Metadata)
            }
        }
        111 if gate_metadata_matches(metadata) => Ok(()),
        _ => Err(Class0ActorError::Metadata),
    }
}

/// 416FF0 uses the common104B0/C490 constructor, with only SubK allocated.
/// Model slots remain authored data and are retained by the allocation receipt.
fn gate_metadata_matches(metadata: &EntityTypeRuntimeMetadata) -> bool {
    use crate::entity_collision_state::{CommonMoverGklPayloads, CommonWorldEffectProfile};

    let Some(initializer) = &metadata.initializer else {
        return false;
    };
    let [choice] = initializer.behavior_choices.as_ref() else {
        return false;
    };
    metadata.mass_raw == 1
        && metadata.capability_flags == 0
        && metadata.initial_health_raw == Some(1)
        && metadata.damage_profile
            == Some(crate::damage::DamageProfile {
                thresholds_raw: [0; 7],
                multipliers_q8: [0; 7],
            })
        && metadata.common_world_effects
            == RetailRuntimeValue::Known(CommonWorldEffectProfile::default())
        && metadata.terrain_contact_task_lifetime_ms == RetailRuntimeValue::Known(0)
        && metadata.constructor_sound_attachment_id == RetailRuntimeValue::Known(Some(100))
        && metadata.accepted_hit_presentation_sound_id == RetailRuntimeValue::Known(None)
        && metadata.infected_model_presentation_sound_id == RetailRuntimeValue::Known(None)
        && metadata.death_sound_id == RetailRuntimeValue::Known(None)
        && metadata.target_warning_sound_id == RetailRuntimeValue::Known(None)
        && metadata.generic_hit_sound_id == RetailRuntimeValue::Known(None)
        && metadata.search_attack_optional_prelude_sound_id == RetailRuntimeValue::Known(None)
        && metadata.search_attack_aim_sound_id == RetailRuntimeValue::Known(None)
        && metadata.search_attack_aim_sound_period_raw == RetailRuntimeValue::Known(0)
        && metadata.run_away_optional_sound_id == RetailRuntimeValue::Known(None)
        && metadata.run_away_sound_period_raw == RetailRuntimeValue::Known(0)
        && metadata.sub_a_propulsion_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_b_lateral_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_c_lift_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_d_steering_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_f_swimming_descriptor == RetailRuntimeValue::Known(None)
        && metadata.projectile_emitter_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_n_payload.is_none()
        && metadata.status_component_descriptor == RetailRuntimeValue::Known(None)
        && metadata.actor_animation_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_h_external_frame_descriptor == RetailRuntimeValue::Known(None)
        && metadata.sub_j_attachment_descriptor == RetailRuntimeValue::Known(None)
        && metadata.common_mover_topology
            == RetailRuntimeValue::Known(CommonMoverComponentTopology {
                sub_k: true,
                ..CommonMoverComponentTopology::default()
            })
        && metadata.common_mover_gkl_payloads
            == RetailRuntimeValue::Known(CommonMoverGklPayloads {
                sub_g: None,
                sub_k: Some([1, 0]),
                sub_l: None,
            })
        && metadata.model_variable_count_raw == RetailRuntimeValue::Known(1)
        && initializer.initializer_state_flags_raw == 0x27285
        && initializer.common_axis_descriptor == CommonAxisDescriptor::default()
        && initializer.behavior_rule_ref == 1
        && initializer.alternate_behavior_class_ref == 2
        && choice.weight_rule_id == 1
        && choice.weight_multiplier == 1
        && choice.behavior_class_id == 0
}

fn publish_class0_graph(entity: &mut Entity, context: BehaviorContextRuntime) {
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(behavior_program(0).expect("class0 program"));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    // C490 clears S then T before replacing P through02800/05F80. There are
    // no06070 component branches for these profiles, hence no extra RNG draw.
    entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Primary,
        PreparedActorTask::new(ActorTaskRuntime::Class0Timer(Class0TimerTaskState::new())),
    );
}

pub(super) fn reselect_class0_actor(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut dyn FnMut() -> u32,
) -> Result<(), Class0ActorError> {
    authenticate_metadata(entity.entity_type, metadata)?;
    if entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT) != RetailRuntimeValue::Known(0)
    {
        return Err(Class0ActorError::Runtime("living selector"));
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Class0ActorError::Graph);
    };
    let program = behavior_program(0).ok_or(Class0ActorError::Metadata)?;
    let context = context
        .reselect_named_type_default(program, 0, program.initial_style)
        .ok_or(Class0ActorError::Graph)?;
    // AC60's living path draws even for the singleton Always choice. The
    // already-dying factory/MainBase class0 policy deliberately differs.
    let choices = &metadata.initializer.as_ref().unwrap().behavior_choices;
    let selected = select_initial_behavior(
        choices,
        |rule| i32::from(rule == BehaviorWeightRule::Always),
        next_random,
    )
    .map_err(|_| Class0ActorError::Metadata)?;
    debug_assert_eq!(
        selected.map(|selection| selection.program.class_id),
        Some(0)
    );
    publish_class0_graph(entity, context);
    Ok(())
}

/// Inputs to the shared104B0/D4A0/C490 birth before its caller publishes
/// the allocation. Surface bits come from that caller's proven world clock or
/// dry-terrain comparison; task and grounding semantics do not depend on scene.
#[derive(Clone, Copy)]
pub(super) enum Class0SpawnInput<'a> {
    Authored(&'a EntitySpawn),
    ///451C00 zeros the instance record before setting its requested type.
    ZeroRecord {
        entity_type: u32,
    },
    /// Controller op `0x38` copies the player's current XYZ, velocity, and Euler.
    AtPose {
        entity_type: u32,
        position_raw: [i16; 3],
        rotation_raw: [i16; 3],
        velocity_raw: [i16; 3],
    },
}

impl Class0SpawnInput<'_> {
    fn authorizes(self, entity: &Entity) -> bool {
        match self {
            Self::Authored(spawn) => {
                entity.authored_spawn_index == Some(spawn.index)
                    && entity.entity_type == spawn.entity_type
                    && !spawn.has_animation
                    && spawn.animation.is_none()
                    && !spawn.has_config
                    && spawn.config.is_none()
            }
            Self::ZeroRecord { entity_type } => {
                entity.authored_spawn_index.is_none()
                    && entity.entity_type == entity_type
                    && entity.position_raw() == [0; 3]
                    && entity.velocity_raw() == [0; 3]
                    && entity.rotation_heading_pitch_roll_raw() == [0; 3]
            }
            Self::AtPose {
                entity_type,
                position_raw,
                rotation_raw,
                velocity_raw,
            } => {
                entity.authored_spawn_index.is_none()
                    && entity.entity_type == entity_type
                    && entity.position_raw() == position_raw
                    && entity.rotation_heading_pitch_roll_raw() == rotation_raw
                    && entity.velocity_raw() == velocity_raw
            }
        }
    }

    fn position_raw(self) -> [i16; 3] {
        match self {
            Self::Authored(spawn) => spawn.position_raw(),
            Self::ZeroRecord { .. } => [0; 3],
            Self::AtPose { position_raw, .. } => position_raw,
        }
    }

    fn param(self) -> u32 {
        match self {
            Self::Authored(spawn) => spawn.param,
            Self::ZeroRecord { .. } | Self::AtPose { .. } => 0,
        }
    }
}

pub(super) struct Class0ActorConstruction<'a> {
    pub allocation: MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub spawn: Class0SpawnInput<'a>,
    pub resources: EntityConstructionResources<'a>,
    pub constructor_surface_bits: u32,
}

pub(super) fn construct_class0_actor(
    entity: &mut Entity,
    request: Class0ActorConstruction<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<NativeClass0Runtime, Class0ActorError> {
    let Class0ActorConstruction {
        allocation,
        metadata,
        spawn,
        resources,
        constructor_surface_bits,
    } = request;
    authenticate_metadata(entity.entity_type, metadata)?;
    let terrain = resources
        .terrain
        .ok_or(Class0ActorError::Runtime("constructor terrain"))?;
    if !spawn.authorizes(entity)
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .any(|slot| entity.actor_task_state(slot).is_some())
        || allocation.entity_id != entity.id
    {
        return Err(Class0ActorError::Graph);
    }
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(Class0ActorError::Metadata)?;
    let initial_selection = match entity.initial_behavior {
        RetailRuntimeValue::Known(Some(selection)) if selection.program.class_id == 0 => selection,
        _ => return Err(Class0ActorError::Metadata),
    };
    let resolution = resolve_entity_initializer_with_selected_behavior(
        EntityInitializerRequest {
            metadata: Some(metadata),
            spawn_param: spawn.param(),
            authored_position_raw: spawn.position_raw(),
            terrain: Some(terrain),
            resource_domain: ResourceDomainRelation::Current,
        },
        Some(initial_selection),
    );
    let mut state = resolution.state_flags;
    state.overwrite(
        crate::entity_initializer::CONSTRUCTOR_SURFACE_STATE_MASK,
        constructor_surface_bits,
    );
    let mut grounded = spawn.position_raw();
    // D4A0's40D513 bit20 gates the entire terrain/radius/+90 block.
    // Type111 lacks it and must retain the caller's marker height.
    if initializer.initializer_state_flags_raw & 0x20 != 0 {
        let model_height = if initializer.initializer_state_flags_raw & 0x40 != 0 {
            let RetailRuntimeValue::Known(active_slot) = entity.collision.active_model_slot()
            else {
                return Err(Class0ActorError::Runtime("constructor model slot"));
            };
            let active_model = entity
                .model_slots
                .get(active_slot)
                .copied()
                .flatten()
                .ok_or(Class0ActorError::Runtime("constructor model"))?;
            resources
                .model_extent_raw
                .and_then(|lookup| lookup(active_model))
                .ok_or(Class0ActorError::Runtime("constructor model header08"))? as i16
        } else {
            0
        };
        grounded[1] = terrain
            .bilinear_height_raw(grounded[0], grounded[2])
            .wrapping_add(model_height);
    }
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(initial_selection)
        .ok_or(Class0ActorError::Metadata)?;
    // Every fallible source/shape lookup precedes the singleton selector.
    let selected = select_initial_behavior(
        &initializer.behavior_choices,
        |rule| i32::from(rule == BehaviorWeightRule::Always),
        next_random,
    )
    .map_err(|_| Class0ActorError::Metadata)?;
    debug_assert_eq!(selected, Some(initial_selection));
    entity.set_position_raw(grounded);
    entity.collision.state_flags_at_0x08 = state;
    // Explicit native policy for104B0's unwritten transient mass word.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    // D720/13F70 retains the spawn Euler-derived basis, including 0x38's
    // player +0xA2/+0xA4/+0xA6 copy.
    entity.apply_d720_euler_body_basis();
    publish_class0_graph(entity, context);
    if entity.entity_type == 54 {
        if let Class0SpawnInput::Authored(spawn) = spawn {
            let word = i32::from_le_bytes(
                spawn.extra[8..12]
                    .try_into()
                    .expect("fixed Section-13 payload slice"),
            );
            entity.main_base_type54_sea_delta_source = RetailRuntimeValue::Known(if word == 0 {
                crate::main_base_type54_abort::MainBaseType54SeaDeltaSource::DeriveFromActorModelAndSea
            } else {
                crate::main_base_type54_abort::MainBaseType54SeaDeltaSource::ExplicitPreShiftWords(
                    word,
                )
            });
        }
    }
    Ok(NativeClass0Runtime {
        allocation,
        entity_type: entity.entity_type,
        model_slots: entity.model_slots,
        anchor_raw: grounded,
        pending_prefix: false,
        gate_self_relation: None,
    })
}

impl EntityManager {
    pub(crate) fn native_gate_self_relation(&self, id: u32) -> Option<u32> {
        self.native_class0_actors.get(&id)?.gate_self_relation
    }

    pub(crate) fn native_class0_construction_present(&self, id: u32) -> bool {
        self.native_class0_actors.contains_key(&id)
    }

    pub(crate) fn native_class0_allocation_authenticates(&self, id: u32) -> bool {
        let Some(runtime) = self.native_class0_actors.get(&id) else {
            return false;
        };
        self.entities
            .iter()
            .find(|entity| entity.id == id)
            .is_some_and(|entity| {
                observe_main_base_abort_actor(entity, self.allocation_generation).lease
                    == runtime.allocation
                    && entity.entity_type == runtime.entity_type
                    && entity.model_slots == runtime.model_slots
            })
    }

    pub(crate) fn native_class0_has_pending_prefix(&self, id: u32) -> bool {
        self.native_class0_actors
            .get(&id)
            .is_some_and(|runtime| runtime.pending_prefix)
    }

    pub(super) fn set_native_class0_pending_prefix(&mut self, id: u32, pending: bool) -> bool {
        let Some(runtime) = self.native_class0_actors.get_mut(&id) else {
            return false;
        };
        runtime.pending_prefix = pending;
        true
    }
}
