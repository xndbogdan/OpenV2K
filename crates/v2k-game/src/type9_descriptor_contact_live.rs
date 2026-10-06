//! Authenticated live adapter for one ordinary type-9 descriptor contact.
//!
//! The six admitted fresh-Level-1 type-9 allocations publish component
//! callback `FUN_00402DA0`.  This adapter binds that callback to an
//! already-published Primary `OrdinaryType9Wander` task without activating a
//! task or consuming scheduler RNG. Both the legacy pending fixture boundary
//! and fresh production's exact finalized selected-Wander custody are admitted;
//! other selected behavior branches remain ineligible.
//!
//! Source ownership is authenticated and copied before the active-pair pass.
//! The contacted body is supplied later because an earlier pair response can
//! move the player before this callback is reached.  Planning preserves
//! retail's lazy gate: a negative forward projection succeeds without
//! consulting the task-private record or Sub-A allocation.  A nonnegative
//! projection admits only the proven Sub-I branch, calls the shared detached
//! descriptor planner, and retains every live owner needed for an atomic
//! stale-state check at commit.

use crate::actor_animation::ActorAnimationController;
use crate::actor_task_dispatcher::{ActorTaskRuntime, ActorTaskRuntimeFamily};
use crate::actor_task_owner::{ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags};
use crate::common_mover::target_prelude::{
    CommonMoverPreludeSubA, CommonMoverPreludeSubD, CommonMoverTargetPreludeTopology,
};
use crate::common_mover::SubAPropulsionRuntime;
use crate::descriptor_contact::{
    plan_descriptor_contact, DescriptorContactBlock, DescriptorContactMiss,
    DescriptorContactOutcome, DescriptorContactPlan, DescriptorContactRequest,
    DescriptorContactSourceSnapshot, DescriptorContactTargetSnapshot,
};
use crate::entity::Entity;
use crate::entity_collision_state::{
    EntityTypeRuntimeMetadata, PairComponentContactPolicy, PairOrientationPolicy,
    RetailRuntimeValue,
};
use crate::hover::HoverBasis;
use crate::ordinary_type9_live::{
    OrdinaryType9LiveOwner, OrdinaryType9LiveOwnerError, OrdinaryType9PendingInitialSelection,
    OrdinaryType9SelectedComponentRuntime,
};
use crate::ordinary_type9_wander_owner::OrdinaryType9WanderTaskState;
use v2k_formats::collision::SubAPropulsionDescriptor;

const TYPE9_DESCRIPTOR_COMPONENTS: [PairComponentContactPolicy; 3] = [
    PairComponentContactPolicy::DescriptorContact,
    PairComponentContactPolicy::None,
    PairComponentContactPolicy::None,
];

/// Why a live entity cannot own this exact descriptor-contact callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type9DescriptorContactAuthenticateError {
    LiveOwner(OrdinaryType9LiveOwnerError),
    ComponentContactUnresolved,
    ComponentContactMismatch {
        actual: [PairComponentContactPolicy; 3],
    },
    OrientationUnresolved,
    RotationPolicyMismatch {
        expected_pitch_roll_raw: [u16; 2],
        actual_pitch_roll_raw: [u16; 2],
    },
}

/// Hit-only state which may remain unavailable while a rear-half-space miss
/// is still completely resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type9DescriptorContactHitStateError {
    PrimaryTaskUnavailable,
    PrimaryTaskWrapperUnavailable,
    PrimaryTaskInCallback,
    PrimaryTaskFamilyMismatch { actual: ActorTaskRuntimeFamily },
    SubARuntimeUnresolved,
    SubARuntimeAbsent,
    ActorAnimationRuntimeUnresolved,
    ActorAnimationRuntimeAbsent,
}

/// Why a dynamically supplied contact could not be planned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type9DescriptorContactPlanError {
    HitState(Type9DescriptorContactHitStateError),
    Descriptor(DescriptorContactBlock),
}

/// Result of the retail half-space gate and, on a hit, descriptor effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type9DescriptorContactPreparation {
    Miss(DescriptorContactMiss),
    Apply(Type9DescriptorContactPlan),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Type9DescriptorContactProvenanceLease {
    active: bool,
    entity_type: u32,
    authored_spawn_index: Option<usize>,
    active_model_slot: RetailRuntimeValue<usize>,
    model_slot_zero: Option<usize>,
    active_model: Option<usize>,
    pending_initial_selection: Option<OrdinaryType9PendingInitialSelection>,
    selected_component_runtime: Option<OrdinaryType9SelectedComponentRuntime>,
    has_death_component_runtime: bool,
    component_contact: RetailRuntimeValue<[PairComponentContactPolicy; 3]>,
    orientation_policy: RetailRuntimeValue<PairOrientationPolicy>,
}

impl Type9DescriptorContactProvenanceLease {
    fn snapshot(entity: &Entity) -> Self {
        Self {
            active: entity.active,
            entity_type: entity.entity_type,
            authored_spawn_index: entity.authored_spawn_index,
            active_model_slot: entity.collision.active_model_slot(),
            model_slot_zero: entity.model_slots[0],
            active_model: entity.model_index,
            pending_initial_selection: entity.ordinary_type9_pending_initial_selection,
            selected_component_runtime: entity.ordinary_type9_selected_component_runtime,
            has_death_component_runtime: entity.main_base_type9_death_component_runtime.is_some(),
            component_contact: entity.collision.pair_callbacks.component_contact,
            orientation_policy: entity.collision.pair_callbacks.orientation_policy,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Type9DescriptorContactTopologyTemplate {
    sub_a_descriptor: SubAPropulsionDescriptor,
    sub_d: CommonMoverPreludeSubD,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Type9DescriptorContactRuntimeLease {
    visit: ActorTaskVisit,
    wrapper_flags: ActorTaskWrapperFlags,
    task_state: OrdinaryType9WanderTaskState,
    sub_a_runtime: SubAPropulsionRuntime,
    actor_animation_runtime: ActorAnimationController,
}

/// Copyable source owner prepared before active-pair traversal.
///
/// Authentication proves the exact fresh-Level-1 allocation, callback slot,
/// Section-12 D/I/A/B topology, and retained collision basis.  Hit-only task
/// and Sub-A state is copied as a deferred result so [`Self::plan`] can honor
/// retail's early miss without converting absent descriptor state into an
/// evidence block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type9DescriptorContactOwner {
    source: DescriptorContactSourceSnapshot,
    rotation_raw: [i16; 3],
    provenance: Type9DescriptorContactProvenanceLease,
    topology: Type9DescriptorContactTopologyTemplate,
    hit_runtime: Result<Type9DescriptorContactRuntimeLease, Type9DescriptorContactHitStateError>,
}

impl Type9DescriptorContactOwner {
    pub fn authenticate(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<Self, Type9DescriptorContactAuthenticateError> {
        OrdinaryType9LiveOwner::from_entity_metadata_for_descriptor_contact(entity, metadata)
            .map_err(Type9DescriptorContactAuthenticateError::LiveOwner)?;

        match entity.collision.pair_callbacks.component_contact {
            RetailRuntimeValue::Known(actual) if actual == TYPE9_DESCRIPTOR_COMPONENTS => {}
            RetailRuntimeValue::Known(actual) => {
                return Err(
                    Type9DescriptorContactAuthenticateError::ComponentContactMismatch { actual },
                );
            }
            RetailRuntimeValue::Unresolved => {
                return Err(Type9DescriptorContactAuthenticateError::ComponentContactUnresolved);
            }
        }

        let PairOrientationPolicy::LiveHeadingWithAuthoredPitchRoll {
            pitch_raw,
            roll_raw,
        } = match entity.collision.pair_callbacks.orientation_policy {
            RetailRuntimeValue::Known(policy) => policy,
            RetailRuntimeValue::Unresolved => {
                return Err(Type9DescriptorContactAuthenticateError::OrientationUnresolved);
            }
        };
        let rotation_raw = entity.rotation_heading_pitch_roll_raw();
        let actual_pitch_roll_raw = [rotation_raw[1] as u16, rotation_raw[2] as u16];
        let expected_pitch_roll_raw = [pitch_raw, roll_raw];
        if actual_pitch_roll_raw != expected_pitch_roll_raw {
            return Err(
                Type9DescriptorContactAuthenticateError::RotationPolicyMismatch {
                    expected_pitch_roll_raw,
                    actual_pitch_roll_raw,
                },
            );
        }

        let sub_a_descriptor = match metadata.sub_a_propulsion_descriptor {
            RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
            _ => unreachable!("the authenticated ordinary type-9 topology proves Sub-A"),
        };
        let sub_d_descriptor = match metadata.sub_d_steering_descriptor {
            RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
            _ => unreachable!("the authenticated ordinary type-9 topology proves Sub-D"),
        };
        let forward_q31 =
            HoverBasis::from_angle_words(rotation_raw[0], rotation_raw[1], rotation_raw[2]).forward;

        Ok(Self {
            source: DescriptorContactSourceSnapshot {
                entity_id: entity.id,
                position_raw: entity.position_raw(),
                forward_q31,
                heading_raw: rotation_raw[0] as u16,
                roll_raw: rotation_raw[2] as u16,
            },
            rotation_raw,
            provenance: Type9DescriptorContactProvenanceLease::snapshot(entity),
            topology: Type9DescriptorContactTopologyTemplate {
                sub_a_descriptor,
                sub_d: CommonMoverPreludeSubD {
                    steering_divisor_raw: sub_d_descriptor.steering_divisor_raw,
                    couple_yaw_into_roll: sub_d_descriptor.couple_yaw_into_roll_raw != 0,
                },
            },
            hit_runtime: snapshot_hit_runtime(entity),
        })
    }

    pub const fn entity_id(self) -> u32 {
        self.source.entity_id
    }

    pub const fn source_snapshot(self) -> DescriptorContactSourceSnapshot {
        self.source
    }

    /// Plan against the contacted body's state at the exact callback point.
    ///
    /// The first shared-planner call deliberately supplies unresolved private
    /// and topology owners.  It can therefore return only a proven miss or the
    /// expected hit-path `UnresolvedPrivateState` boundary.  Only the latter
    /// consults the deferred live task/Sub-A snapshot and invokes the complete
    /// zero-RNG Sub-I plan.
    pub fn plan(
        self,
        target: DescriptorContactTargetSnapshot,
    ) -> Result<Type9DescriptorContactPreparation, Type9DescriptorContactPlanError> {
        let probe = plan_descriptor_contact(
            DescriptorContactRequest {
                source: RetailRuntimeValue::Known(self.source),
                target: RetailRuntimeValue::Known(target),
                private_state: RetailRuntimeValue::Unresolved,
                topology: RetailRuntimeValue::Unresolved,
            },
            || unreachable!("the descriptor half-space gate consumes no RNG"),
        );
        match probe {
            Ok(DescriptorContactOutcome::Miss(miss)) => {
                return Ok(Type9DescriptorContactPreparation::Miss(miss));
            }
            Err(DescriptorContactBlock::UnresolvedPrivateState) => {}
            Ok(DescriptorContactOutcome::Apply(_))
            | Err(DescriptorContactBlock::UnresolvedSource)
            | Err(DescriptorContactBlock::UnresolvedTarget)
            | Err(DescriptorContactBlock::UnresolvedTopology)
            | Err(DescriptorContactBlock::InvalidSubDReversalDivisor { .. }) => {
                unreachable!("known source/target probe has one exact hit boundary")
            }
        }

        let runtime = self
            .hit_runtime
            .map_err(Type9DescriptorContactPlanError::HitState)?;
        let topology = CommonMoverTargetPreludeTopology {
            sub_a: Some(CommonMoverPreludeSubA {
                descriptor: Some(self.topology.sub_a_descriptor),
                runtime: runtime.sub_a_runtime,
            }),
            sub_d: Some(self.topology.sub_d),
            sub_f: false,
            sub_g: false,
            sub_i: true,
            sub_l: false,
        };
        let complete = plan_descriptor_contact(
            DescriptorContactRequest {
                source: RetailRuntimeValue::Known(self.source),
                target: RetailRuntimeValue::Known(target),
                private_state: RetailRuntimeValue::Known(runtime.task_state.private_state()),
                topology: RetailRuntimeValue::Known(topology),
            },
            || unreachable!("the authenticated type-9 Sub-I effect consumes no RNG"),
        )
        .map_err(Type9DescriptorContactPlanError::Descriptor)?;
        let DescriptorContactOutcome::Apply(descriptor) = complete else {
            unreachable!("the complete plan repeats the identical accepted half-space gate")
        };

        debug_assert_eq!(descriptor.effect.rng_draw_count, 0);
        debug_assert_eq!(
            descriptor.effect.target_state,
            runtime.task_state.private_state()
        );
        debug_assert_eq!(descriptor.effect.roll_raw, self.source.roll_raw);
        debug_assert!(descriptor.effect.sub_d_reversal_write.is_none());
        debug_assert_eq!(descriptor.effect.sub_f_reverse_write, None);
        debug_assert_eq!(descriptor.effect.sub_g_reverse_write, None);

        Ok(Type9DescriptorContactPreparation::Apply(
            Type9DescriptorContactPlan {
                owner: self,
                runtime,
                descriptor,
            },
        ))
    }
}

fn snapshot_hit_runtime(
    entity: &Entity,
) -> Result<Type9DescriptorContactRuntimeLease, Type9DescriptorContactHitStateError> {
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) else {
        return Err(Type9DescriptorContactHitStateError::PrimaryTaskUnavailable);
    };
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let Some(wrapper_flags) = entity.actor_tasks.wrapper_flags(task_id) else {
        return Err(Type9DescriptorContactHitStateError::PrimaryTaskWrapperUnavailable);
    };
    if wrapper_flags
        != (ActorTaskWrapperFlags {
            alive: true,
            in_callback: false,
        })
    {
        return Err(Type9DescriptorContactHitStateError::PrimaryTaskInCallback);
    }
    let task_state = match entity.actor_tasks.task_state(task_id) {
        Some(ActorTaskRuntime::OrdinaryType9Wander(state)) => *state,
        Some(actual) => {
            return Err(
                Type9DescriptorContactHitStateError::PrimaryTaskFamilyMismatch {
                    actual: actual.family(),
                },
            );
        }
        None => return Err(Type9DescriptorContactHitStateError::PrimaryTaskUnavailable),
    };
    let sub_a_runtime = match entity.sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(runtime)) => runtime,
        RetailRuntimeValue::Known(None) => {
            return Err(Type9DescriptorContactHitStateError::SubARuntimeAbsent);
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type9DescriptorContactHitStateError::SubARuntimeUnresolved);
        }
    };
    let actor_animation_runtime = match entity.actor_animation_runtime {
        RetailRuntimeValue::Known(Some(runtime)) => runtime,
        RetailRuntimeValue::Known(None) => {
            return Err(Type9DescriptorContactHitStateError::ActorAnimationRuntimeAbsent);
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type9DescriptorContactHitStateError::ActorAnimationRuntimeUnresolved);
        }
    };
    Ok(Type9DescriptorContactRuntimeLease {
        visit,
        wrapper_flags,
        task_state,
        sub_a_runtime,
        actor_animation_runtime,
    })
}

/// Positive contact plan whose fields cannot be fabricated outside this
/// module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type9DescriptorContactPlan {
    owner: Type9DescriptorContactOwner,
    runtime: Type9DescriptorContactRuntimeLease,
    descriptor: DescriptorContactPlan,
}

impl Type9DescriptorContactPlan {
    pub const fn entity_id(self) -> u32 {
        self.owner.source.entity_id
    }

    pub const fn target_entity_id(self) -> u32 {
        self.descriptor.target_entity_id
    }

    pub const fn task_visit(self) -> ActorTaskVisit {
        self.runtime.visit
    }

    pub const fn expected_heading_raw(self) -> u16 {
        self.descriptor.expected_source.heading_raw
    }

    pub const fn resolved_heading_raw(self) -> u16 {
        self.descriptor.effect.heading_raw
    }
}

/// A stale plan is rejected before either heading or Sub-A becomes visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type9DescriptorContactCommitError {
    EntityIdentityChanged {
        expected: u32,
        actual: u32,
    },
    ProvenanceChanged,
    PositionChanged {
        expected: [i16; 3],
        actual: [i16; 3],
    },
    RotationChanged {
        expected: [i16; 3],
        actual: [i16; 3],
    },
    TaskLeaseChanged,
    TaskWrapperChanged,
    TaskStateChanged,
    SubARuntimeChanged,
    ActorAnimationRuntimeChanged,
}

/// Auditable result of one exact Sub-I descriptor effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type9DescriptorContactCommitOutcome {
    pub entity_id: u32,
    pub target_entity_id: u32,
    pub task_visit: ActorTaskVisit,
    pub heading_raw_before: u16,
    pub heading_raw_after: u16,
    pub direction_multiplier_before: i32,
    pub direction_multiplier_after: i32,
}

/// Validate every source owner retained by a positive descriptor plan.
///
/// The contacted body has no destination in this transaction.  Its dynamic
/// body lease remains with the active-pair coordinator which supplied the
/// target snapshot.
pub fn validate_type9_descriptor_contact(
    entity: &Entity,
    plan: Type9DescriptorContactPlan,
) -> Result<(), Type9DescriptorContactCommitError> {
    if entity.id != plan.owner.source.entity_id {
        return Err(Type9DescriptorContactCommitError::EntityIdentityChanged {
            expected: plan.owner.source.entity_id,
            actual: entity.id,
        });
    }
    if Type9DescriptorContactProvenanceLease::snapshot(entity) != plan.owner.provenance {
        return Err(Type9DescriptorContactCommitError::ProvenanceChanged);
    }
    let actual_position = entity.position_raw();
    if actual_position != plan.owner.source.position_raw {
        return Err(Type9DescriptorContactCommitError::PositionChanged {
            expected: plan.owner.source.position_raw,
            actual: actual_position,
        });
    }
    let actual_rotation = entity.rotation_heading_pitch_roll_raw();
    if actual_rotation != plan.owner.rotation_raw {
        return Err(Type9DescriptorContactCommitError::RotationChanged {
            expected: plan.owner.rotation_raw,
            actual: actual_rotation,
        });
    }
    if entity.actor_tasks.task_in_slot(plan.runtime.visit.slot) != Some(plan.runtime.visit.task_id)
    {
        return Err(Type9DescriptorContactCommitError::TaskLeaseChanged);
    }
    if entity.actor_tasks.wrapper_flags(plan.runtime.visit.task_id)
        != Some(plan.runtime.wrapper_flags)
    {
        return Err(Type9DescriptorContactCommitError::TaskWrapperChanged);
    }
    match entity.actor_tasks.task_state(plan.runtime.visit.task_id) {
        Some(ActorTaskRuntime::OrdinaryType9Wander(state)) if *state == plan.runtime.task_state => {
        }
        _ => return Err(Type9DescriptorContactCommitError::TaskStateChanged),
    }
    if entity.sub_a_propulsion_runtime
        != RetailRuntimeValue::Known(Some(plan.runtime.sub_a_runtime))
    {
        return Err(Type9DescriptorContactCommitError::SubARuntimeChanged);
    }
    if entity.actor_animation_runtime
        != RetailRuntimeValue::Known(Some(plan.runtime.actor_animation_runtime))
    {
        return Err(Type9DescriptorContactCommitError::ActorAnimationRuntimeChanged);
    }
    Ok(())
}

/// Atomically commit the exact heading and Sub-A writes from one positive
/// type-9 descriptor contact.
pub fn commit_type9_descriptor_contact(
    entity: &mut Entity,
    plan: Type9DescriptorContactPlan,
) -> Result<Type9DescriptorContactCommitOutcome, Type9DescriptorContactCommitError> {
    validate_type9_descriptor_contact(entity, plan)?;

    let resolved_sub_a = plan
        .descriptor
        .effect
        .sub_a_runtime
        .expect("authenticated ordinary type-9 topology proves Sub-A");
    entity.set_heading_raw(plan.descriptor.effect.heading_raw);
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(resolved_sub_a));

    Ok(Type9DescriptorContactCommitOutcome {
        entity_id: entity.id,
        target_entity_id: plan.descriptor.target_entity_id,
        task_visit: plan.runtime.visit,
        heading_raw_before: plan.descriptor.expected_source.heading_raw,
        heading_raw_after: plan.descriptor.effect.heading_raw,
        direction_multiplier_before: plan.runtime.sub_a_runtime.direction_multiplier(),
        direction_multiplier_after: resolved_sub_a.direction_multiplier(),
    })
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::*;
    use crate::actor_task_dispatcher::ActorTaskRuntime;
    use crate::actor_task_owner::ActorTaskSlot;
    use crate::common_mover::sub_d::ORDINARY_TYPE9_SUB_D;
    use crate::common_mover::type9_attitude::Type9BodyBasis;
    use crate::entity::{Entity, EntityKind};
    use crate::entity_behavior::{behavior_program, BehaviorContextRuntime, BehaviorSelection};
    use crate::entity_collision_state::{
        EntityInitializerSpec, EntityPairCallbackRuntimeState, RetailStateWord,
    };
    use crate::ordinary_type9_live::{
        admit_fresh_level1_ordinary_type9, FreshLevel1OrdinaryType9SpawnFacts,
        OrdinaryType9SelectedComponentRuntime, OrdinaryType9SelectedRuntimeKind,
        FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID,
    };
    use crate::ordinary_type9_wander_owner::plan_ordinary_type9_wander_setup;
    use v2k_formats::collision::ActorAnimationDescriptor;

    const TYPE9_ENTITY_TYPE: u32 = 9;
    const SUB_A: SubAPropulsionDescriptor = SubAPropulsionDescriptor {
        acceleration_raw: 1_500,
        overspeed_correction_raw: -3_000,
        target_speed_base_raw: 250,
    };
    const SUB_I: ActorAnimationDescriptor = ActorAnimationDescriptor {
        capability_bit_3_sound_id: 72,
        capability_mask_0x201_sound_id: 0,
        attention_stop_sound_id: 72,
        variable_binding: 1,
        frames_per_direction: 4,
    };

    fn metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID as u16; 4],
            mass_raw: crate::main_base_type9_abort::LEVEL_ONE_TYPE9_MASS_RAW,
            capability_flags: crate::main_base_type9_abort::LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
            initial_health_raw: Some(
                crate::main_base_type9_abort::LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW,
            ),
            accepted_hit_presentation_sound_id: RetailRuntimeValue::Known(Some(95)),
            death_sound_id: RetailRuntimeValue::Known(Some(
                crate::main_base_type9_abort::LEVEL_ONE_TYPE9_DEATH_SOUND_ID,
            )),
            constructor_sound_attachment_id: RetailRuntimeValue::Known(None),
            generic_hit_sound_id: RetailRuntimeValue::Known(None),
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                crate::main_base_type9_abort::LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR,
            )),
            sub_b_lateral_descriptor: RetailRuntimeValue::Known(Some(
                crate::main_base_type9_abort::LEVEL_ONE_TYPE9_SUB_B_DESCRIPTOR,
            )),
            sub_d_steering_descriptor: RetailRuntimeValue::Known(Some(ORDINARY_TYPE9_SUB_D)),
            actor_animation_descriptor: RetailRuntimeValue::Known(Some(
                crate::main_base_type9_abort::LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR,
            )),
            common_mover_topology: RetailRuntimeValue::Known(
                crate::main_base_type9_abort::LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY,
            ),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw:
                    crate::main_base_type9_abort::LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW,
                common_axis_descriptor:
                    crate::main_base_type9_abort::LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR,
                behavior_choices: crate::main_base_type9_abort::LEVEL_ONE_TYPE9_BEHAVIOR_CHOICES
                    .into(),
                behavior_rule_ref: 1,
                alternate_behavior_class_ref: u32::from(
                    crate::main_base_type9_abort::LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS,
                ),
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn admitted_entity() -> Entity {
        let mut entity = Entity::unresolved_port_entity(
            0x04ac_0001,
            EntityKind::Unknown(TYPE9_ENTITY_TYPE),
            TYPE9_ENTITY_TYPE,
        );
        entity.authored_spawn_index = Some(9);
        entity.model_slots = [Some(FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID); 4];
        entity.model_index = Some(FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID);
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(
            crate::main_base_type9_abort::LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR,
        );
        entity.ordinary_type9_pending_initial_selection = Some(
            admit_fresh_level1_ordinary_type9(FreshLevel1OrdinaryType9SpawnFacts {
                retail_first_world: true,
                authored_spawn_index: 9,
                entity_type: TYPE9_ENTITY_TYPE,
                active_model_slot: RetailRuntimeValue::Known(0),
                active_model: Some(FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID),
                rotation: [0; 3],
                immutable_anchor_raw_at_0x90: RetailRuntimeValue::Known([0, 100, 0]),
            })
            .unwrap()
            .pending_initial_selection(),
        );
        entity.collision.pair_callbacks = EntityPairCallbackRuntimeState::audited_local(
            Some(crate::descriptor_contact::DESCRIPTOR_CONTACT_CALLBACK_ADDRESS),
            RetailRuntimeValue::Known(PairOrientationPolicy::LiveHeadingWithAuthoredPitchRoll {
                pitch_raw: 0,
                roll_raw: 0,
            }),
        );
        entity.set_motion_raw([0, 100, 0], [0; 3]);
        entity.set_heading_raw(0);
        entity.actor_animation_runtime = RetailRuntimeValue::Known(Some(
            ActorAnimationController::from_descriptor(SUB_I).expect("valid Sub-I descriptor"),
        ));
        entity
    }

    fn entity_with_published_wander() -> (Entity, ActorTaskVisit) {
        let mut entity = admitted_entity();
        let mut sub_a =
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(333), 1, 73);
        plan_ordinary_type9_wander_setup([0, 100, 0], SUB_A.target_speed_base_raw)
            .apply(&mut entity.actor_tasks, &mut sub_a, |specification| {
                Ok::<_, Infallible>(
                    specification
                        .prepare_after_allocation(|| 0x1234_5678)
                        .map_task(ActorTaskRuntime::OrdinaryType9Wander),
                )
            })
            .unwrap();
        sub_a.set_direction_multiplier(-1);
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(sub_a));
        let visit = ActorTaskVisit {
            slot: ActorTaskSlot::Primary,
            task_id: entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap(),
        };
        (entity, visit)
    }

    fn entity_with_selected_published_wander() -> (Entity, ActorTaskVisit) {
        let (mut entity, visit) = entity_with_published_wander();
        let components = entity
            .ordinary_type9_pending_initial_selection
            .take()
            .expect("test fixture starts with pending component custody");
        entity.ordinary_type9_selected_component_runtime =
            Some(OrdinaryType9SelectedComponentRuntime::new(
                components,
                OrdinaryType9SelectedRuntimeKind::WanderNearPublished,
            ));
        let program = behavior_program(
            crate::ordinary_type9_initial_selection::LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID,
        )
        .expect("class-6 Wander Near is audited");
        let selection = BehaviorSelection {
            choice_index: 3,
            program,
        };
        entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(
            BehaviorContextRuntime::from_fresh_weighted_selection(selection)
                .expect("selected Wander has an audited initial context"),
        ));
        entity.physical_body_basis_q31 =
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
                entity.heading_raw() as i16,
                entity.rotation_heading_pitch_roll_raw()[1],
                entity.rotation_heading_pitch_roll_raw()[2],
            ));
        entity.collision.state_flags_at_0x08 =
            RetailStateWord::exact(crate::entity_collision_state::BODY_BASIS_REBUILT_STATE_BIT);
        (entity, visit)
    }

    fn target(position_raw: [i16; 3]) -> DescriptorContactTargetSnapshot {
        DescriptorContactTargetSnapshot {
            entity_id: 0x04b5_0001,
            position_raw,
        }
    }

    fn applied(preparation: Type9DescriptorContactPreparation) -> Type9DescriptorContactPlan {
        let Type9DescriptorContactPreparation::Apply(plan) = preparation else {
            panic!("expected positive descriptor contact");
        };
        plan
    }

    #[test]
    fn rear_half_space_miss_is_lazy_without_task_or_sub_a() {
        let entity = admitted_entity();
        let owner = Type9DescriptorContactOwner::authenticate(&entity, &metadata()).unwrap();

        assert!(matches!(
            owner.plan(target([-100, 100, 0])).unwrap(),
            Type9DescriptorContactPreparation::Miss(DescriptorContactMiss {
                source_entity_id: 0x04ac_0001,
                target_entity_id: 0x04b5_0001,
                forward_projection_raw: ..=-1,
            })
        ));
        assert_eq!(
            owner.plan(target([100, 100, 0])),
            Err(Type9DescriptorContactPlanError::HitState(
                Type9DescriptorContactHitStateError::PrimaryTaskUnavailable
            ))
        );
    }

    #[test]
    fn positive_contact_commits_only_proven_zero_rng_sub_i_writes() {
        let (mut entity, visit) = entity_with_published_wander();
        entity.set_heading_raw(0xd7b1);
        let task_before = *entity.actor_tasks.task_state(visit.task_id).unwrap();
        let sub_a_before = match entity.sub_a_propulsion_runtime {
            RetailRuntimeValue::Known(Some(runtime)) => runtime,
            _ => unreachable!(),
        };
        let owner = Type9DescriptorContactOwner::authenticate(&entity, &metadata()).unwrap();
        let plan = applied(owner.plan(target([100, 100, 0])).unwrap());

        assert_eq!(plan.expected_heading_raw(), 0xd7b1);
        assert_eq!(plan.resolved_heading_raw(), 0xf7b1);
        let outcome = commit_type9_descriptor_contact(&mut entity, plan).unwrap();
        assert_eq!(
            outcome,
            Type9DescriptorContactCommitOutcome {
                entity_id: 0x04ac_0001,
                target_entity_id: 0x04b5_0001,
                task_visit: visit,
                heading_raw_before: 0xd7b1,
                heading_raw_after: 0xf7b1,
                direction_multiplier_before: -1,
                direction_multiplier_after: 1,
            }
        );
        assert_eq!(entity.heading_raw(), 0xf7b1);
        assert_eq!(
            entity.actor_tasks.task_state(visit.task_id),
            Some(&task_before),
            "the Sub-I descriptor effect does not rewrite task-private state"
        );
        let RetailRuntimeValue::Known(Some(sub_a_after)) = entity.sub_a_propulsion_runtime else {
            panic!("Sub-A must survive the component effect");
        };
        assert_eq!(sub_a_after.direction_multiplier(), 1);
        assert_eq!(
            sub_a_after.target_speed_raw(),
            sub_a_before.target_speed_raw()
        );
        assert_eq!(
            sub_a_after.drive_scale_percent(),
            sub_a_before.drive_scale_percent()
        );
    }

    #[test]
    fn selected_wander_custody_authenticates_and_survives_contact() {
        let (mut entity, visit) = entity_with_selected_published_wander();
        let selected_before = entity.ordinary_type9_selected_component_runtime;
        let task_before = *entity.actor_tasks.task_state(visit.task_id).unwrap();
        let owner = Type9DescriptorContactOwner::authenticate(&entity, &metadata()).unwrap();
        let plan = applied(owner.plan(target([100, 100, 0])).unwrap());

        let outcome = commit_type9_descriptor_contact(&mut entity, plan).unwrap();
        assert_eq!(outcome.task_visit, visit);
        assert_eq!(
            entity.ordinary_type9_selected_component_runtime,
            selected_before
        );
        assert_eq!(
            entity.actor_tasks.task_state(visit.task_id),
            Some(&task_before)
        );
        assert!(entity.ordinary_type9_pending_initial_selection.is_none());
    }

    #[test]
    fn selected_wander_custody_change_rejects_contact_atomically() {
        let (mut entity, _) = entity_with_selected_published_wander();
        let owner = Type9DescriptorContactOwner::authenticate(&entity, &metadata()).unwrap();
        let plan = applied(owner.plan(target([100, 100, 0])).unwrap());
        let selected = entity
            .ordinary_type9_selected_component_runtime
            .expect("selected fixture");
        entity.ordinary_type9_selected_component_runtime =
            Some(OrdinaryType9SelectedComponentRuntime::new(
                selected.components(),
                OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished,
            ));
        let heading_before = entity.heading_raw();
        let sub_a_before = entity.sub_a_propulsion_runtime;

        assert_eq!(
            commit_type9_descriptor_contact(&mut entity, plan),
            Err(Type9DescriptorContactCommitError::ProvenanceChanged)
        );
        assert_eq!(entity.heading_raw(), heading_before);
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before);
    }

    #[test]
    fn stale_task_state_rejects_before_heading_or_sub_a_write() {
        let (mut entity, visit) = entity_with_published_wander();
        let owner = Type9DescriptorContactOwner::authenticate(&entity, &metadata()).unwrap();
        let plan = applied(owner.plan(target([100, 100, 0])).unwrap());
        let Some(ActorTaskRuntime::OrdinaryType9Wander(task)) =
            entity.actor_tasks.task_state_mut(visit.task_id)
        else {
            unreachable!()
        };
        let mut changed_private = task.private_state();
        changed_private.reversal_timer_ms = 77;
        task.replace_private_state(changed_private);
        let heading_before = entity.heading_raw();
        let sub_a_before = entity.sub_a_propulsion_runtime;

        assert_eq!(
            commit_type9_descriptor_contact(&mut entity, plan),
            Err(Type9DescriptorContactCommitError::TaskStateChanged)
        );
        assert_eq!(entity.heading_raw(), heading_before);
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before);
    }

    #[test]
    fn stale_sub_a_rotation_and_provenance_each_fail_atomically() {
        let (entity, _) = entity_with_published_wander();
        let owner = Type9DescriptorContactOwner::authenticate(&entity, &metadata()).unwrap();
        let plan = applied(owner.plan(target([100, 100, 0])).unwrap());

        let mut stale_sub_a = entity_with_published_wander().0;
        let RetailRuntimeValue::Known(Some(runtime)) = &mut stale_sub_a.sub_a_propulsion_runtime
        else {
            unreachable!()
        };
        runtime.set_direction_multiplier(7);
        let sub_a_before = stale_sub_a.sub_a_propulsion_runtime;
        assert_eq!(
            commit_type9_descriptor_contact(&mut stale_sub_a, plan),
            Err(Type9DescriptorContactCommitError::SubARuntimeChanged)
        );
        assert_eq!(stale_sub_a.heading_raw(), 0);
        assert_eq!(stale_sub_a.sub_a_propulsion_runtime, sub_a_before);

        let mut stale_rotation = entity;
        stale_rotation.set_heading_raw(1);
        let sub_a_before = stale_rotation.sub_a_propulsion_runtime;
        assert!(matches!(
            commit_type9_descriptor_contact(&mut stale_rotation, plan),
            Err(Type9DescriptorContactCommitError::RotationChanged { .. })
        ));
        assert_eq!(stale_rotation.heading_raw(), 1);
        assert_eq!(stale_rotation.sub_a_propulsion_runtime, sub_a_before);

        let (mut stale_animation, _) = entity_with_published_wander();
        let RetailRuntimeValue::Known(Some(animation)) =
            &mut stale_animation.actor_animation_runtime
        else {
            unreachable!()
        };
        animation.advance_neutral(20_000, 0);
        let heading_before = stale_animation.heading_raw();
        let sub_a_before = stale_animation.sub_a_propulsion_runtime;
        assert_eq!(
            commit_type9_descriptor_contact(&mut stale_animation, plan),
            Err(Type9DescriptorContactCommitError::ActorAnimationRuntimeChanged)
        );
        assert_eq!(stale_animation.heading_raw(), heading_before);
        assert_eq!(stale_animation.sub_a_propulsion_runtime, sub_a_before);

        let (mut stale_provenance, _) = entity_with_published_wander();
        stale_provenance.ordinary_type9_pending_initial_selection = None;
        let heading_before = stale_provenance.heading_raw();
        let sub_a_before = stale_provenance.sub_a_propulsion_runtime;
        assert_eq!(
            commit_type9_descriptor_contact(&mut stale_provenance, plan),
            Err(Type9DescriptorContactCommitError::ProvenanceChanged)
        );
        assert_eq!(stale_provenance.heading_raw(), heading_before);
        assert_eq!(stale_provenance.sub_a_propulsion_runtime, sub_a_before);
    }
}
