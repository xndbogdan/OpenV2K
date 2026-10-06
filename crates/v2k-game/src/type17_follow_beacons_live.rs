//! Authenticated live type-17 binding for class-33 `Follow Beacons`.
//!
//! The detached task programs retain the original allocation, callback, and
//! scheduler contracts. This module binds their proven storage to one live
//! [`Entity`]: variant zero publishes its two tasks and mutates the actor-local
//! common-axis copy before ranked selection; a positive type-52 candidate then
//! performs the synchronous target/style handoff and publishes variant one's
//! primary task. Production now adopts that published variant-zero graph and
//! ticks this live acquire/handoff path. Variant-one `FUN_00403CE0` is bound
//! by the production owner and applies the `V200002.run` first
//! `FUN_0041FCB0` full-reset for spawn-18 seed `0x32`.

use std::convert::Infallible;

use v2k_formats::collision::CommonAxisDescriptor;

use crate::{
    actor_task_dispatcher::{
        prepare_follow_beacons_acquiring_runtime_task,
        prepare_follow_beacons_following_runtime_task, ActorTaskRuntime,
        FollowBeaconsFollowingConstructorEffect, PreparedFollowBeaconsFollowingRuntimeTask,
        PreparedSharedGenericRuntimeTask, SharedGenericConstructorEffect,
    },
    actor_task_owner::{ActorTaskId, ActorTaskSlot},
    common_mover::SubAPropulsionRuntime,
    entity::Entity,
    entity_behavior::{
        audited_behavior_style, behavior_program, initial_behavior_state_policy,
        ActiveBehaviorStyle, BehaviorChoiceListSource, BehaviorContextRuntime,
        BehaviorDescriptorIdentity,
    },
    entity_collision_state::{
        EntityCollisionRuntimeState, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
    follow_beacons::{
        apply_follow_beacons_acquiring_task_setup, apply_follow_beacons_following_task_setup,
        select_follow_beacon, FollowBeaconEntityRef, FollowBeaconOwnerRef, FollowBeaconSelection,
        FollowBeaconSelectionError, FollowBeaconSelectionRequest, FollowBeaconTarget,
        FollowBeaconsFollowingTaskPreparation, FollowBeaconsFollowingTaskSetupError,
        FollowBeaconsTaskPreparation, FollowBeaconsTaskRole, FollowBeaconsTaskSetupError,
        FollowBeaconsVariant, FOLLOW_BEACONS_BEHAVIOR_CLASS_ID,
    },
    sub_h_external_frame::SubHRuntimeState,
    type17_impact_live::{
        is_type17_reselection_axis_descriptor, TYPE17_FOLLOW_BEACONS_LIVE_AXIS_DESCRIPTOR,
        TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR, TYPE17_MODEL256_COMPONENT_TOPOLOGY,
        TYPE17_MODEL256_SUB_A_TARGET_SPEED_BASE_RAW, TYPE17_MODEL256_SUB_H_RECORD_COUNT,
    },
    type17_impact_reselection::TYPE17_IMPACT_ENTITY_TYPE,
    world_fx::WorldFx,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17FollowGenericConstructorEvidence {
    pub random_sample_low16: u16,
    pub sub_a_target_speed_raw: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17FollowFollowingConstructorEvidence {
    pub generic: Type17FollowGenericConstructorEvidence,
    pub final_sub_a_target_speed_raw: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17FollowInitializerFailure {
    pub phase_index: usize,
    pub slot: ActorTaskSlot,
    pub role: FollowBeaconsTaskRole,
}

/// Authenticated lease for a successfully published variant-zero task graph.
///
/// Copies are harmless because every callback revalidates both task IDs and
/// families. A no-positive frame may legitimately reuse the lease; any style
/// handoff or replacement invalidates it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17FollowBeaconsAcquiringOwner {
    entity_id: u32,
    primary_task_id: ActorTaskId,
    secondary_task_id: ActorTaskId,
}

impl Type17FollowBeaconsAcquiringOwner {
    pub const fn entity_id(self) -> u32 {
        self.entity_id
    }

    pub const fn primary_task_id(self) -> ActorTaskId {
        self.primary_task_id
    }

    pub const fn secondary_task_id(self) -> ActorTaskId {
        self.secondary_task_id
    }

    /// Recover the published variant-zero lease from live task/context state.
    pub fn adopt_published(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<Self, Type17FollowBeaconsLiveError> {
        let owner = Self {
            entity_id: entity.id,
            primary_task_id: entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .ok_or(Type17FollowBeaconsLiveError::TaskLeaseChanged)?,
            secondary_task_id: entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Secondary)
                .ok_or(Type17FollowBeaconsLiveError::TaskLeaseChanged)?,
        };
        validate_acquiring_owner(owner, entity, metadata)?;
        Ok(owner)
    }
}

/// Authenticated variant-one owner. Production drives mover-first `FUN_00403CE0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17FollowBeaconsFollowingOwner {
    entity_id: u32,
    primary_task_id: ActorTaskId,
    target_id: u32,
}

impl Type17FollowBeaconsFollowingOwner {
    pub const fn entity_id(self) -> u32 {
        self.entity_id
    }

    pub const fn primary_task_id(self) -> ActorTaskId {
        self.primary_task_id
    }

    pub const fn target_id(self) -> u32 {
        self.target_id
    }

    /// Recover the published variant-one lease without running `FUN_00403CE0`.
    pub fn adopt_published(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<Self, Type17FollowBeaconsLiveError> {
        let context = match entity.current_behavior_context {
            RetailRuntimeValue::Known(Some(context)) => context,
            RetailRuntimeValue::Known(None) => {
                return Err(Type17FollowBeaconsLiveError::BehaviorContextAbsent)
            }
            RetailRuntimeValue::Unresolved => {
                return Err(Type17FollowBeaconsLiveError::BehaviorContextUnresolved)
            }
        };
        let RetailRuntimeValue::Known(Some(target_id)) = context.target_handle_at_0x08() else {
            return Err(Type17FollowBeaconsLiveError::UnexpectedBehaviorContext);
        };
        let owner = Self {
            entity_id: entity.id,
            primary_task_id: entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .ok_or(Type17FollowBeaconsLiveError::TaskLeaseChanged)?,
            target_id,
        };
        owner.validate(entity, metadata)?;
        Ok(owner)
    }

    /// Revalidate the exact live state before the mover-first first frame.
    pub fn validate(
        self,
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<(), Type17FollowBeaconsLiveError> {
        validate_shared_type17_follow_state(entity, metadata)?;
        validate_follow_context(
            entity,
            FollowBeaconsVariant::Following,
            FollowContextTargetExpectation::Exact(self.target_id),
        )?;
        if entity.actor_common_axis_descriptor
            != RetailRuntimeValue::Known(TYPE17_FOLLOW_BEACONS_LIVE_AXIS_DESCRIPTOR)
        {
            return Err(
                Type17FollowBeaconsLiveError::UnexpectedActorCommonAxisDescriptor {
                    actual: entity.actor_common_axis_descriptor,
                },
            );
        }
        if entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) != Some(self.primary_task_id)
            || entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Secondary)
                .is_some()
            || entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Tertiary)
                .is_some()
            || !matches!(
                entity.actor_tasks.task_state(self.primary_task_id),
                Some(ActorTaskRuntime::FollowBeaconsFollowing(state))
                    if state.target_id() == self.target_id
            )
        {
            return Err(Type17FollowBeaconsLiveError::TaskLeaseChanged);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type17FollowBeaconsAcquiringPublication {
    Published {
        owner: Type17FollowBeaconsAcquiringOwner,
        constructors_by_phase: [Type17FollowGenericConstructorEvidence; 2],
    },
    InitializerFallbackPublished {
        failure: Type17FollowInitializerFailure,
        constructors_by_phase: [Option<Type17FollowGenericConstructorEvidence>; 2],
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type17FollowBeaconsAcquisitionOutcome {
    NoPositiveScore {
        output_write: Option<FollowBeaconTarget>,
        retail_tag_address: u32,
    },
    FollowingPublished {
        target: FollowBeaconTarget,
        owner: Type17FollowBeaconsFollowingOwner,
        constructor: Type17FollowFollowingConstructorEvidence,
    },
    InitializerFallbackPublished {
        target: FollowBeaconTarget,
        failure: Type17FollowInitializerFailure,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type17FollowBeaconsLiveError {
    EntityInactive,
    UnexpectedEntityType {
        actual: u32,
    },
    OwnerRuntimeMismatch {
        planned: u32,
        actual: u32,
    },
    MissingInitializer,
    UnexpectedMetadataCommonAxisDescriptor {
        actual: CommonAxisDescriptor,
    },
    ComponentTopologyUnresolved,
    UnexpectedComponentTopology,
    SubADescriptorUnresolved,
    SubADescriptorAbsent,
    UnexpectedSubATargetSpeedBase {
        actual: i16,
    },
    SubHDescriptorUnresolved,
    SubHDescriptorAbsent,
    UnexpectedSubHDescriptorRecordCount {
        actual: usize,
    },
    SubARuntimeUnresolved,
    SubARuntimeAbsent,
    SubHRuntimeUnresolved,
    SubHRuntimeAbsent,
    UnexpectedSubHRuntimeRecordCount {
        actual: usize,
    },
    ActorCommonAxisDescriptorUnresolved,
    UnexpectedActorCommonAxisDescriptor {
        actual: RetailRuntimeValue<CommonAxisDescriptor>,
    },
    BehaviorContextUnresolved,
    BehaviorContextAbsent,
    UnexpectedBehaviorContext,
    TaskLeaseChanged,
    CandidateSelection(FollowBeaconSelectionError),
}

/// Owned entity-list snapshot used by the ranked selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FollowBeaconLiveEntitySnapshot {
    id: u32,
    position_raw: [i16; 3],
    capability_flags: RetailRuntimeValue<u32>,
    score_at_0x88: RetailRuntimeValue<i32>,
    collision: EntityCollisionRuntimeState,
}

impl FollowBeaconLiveEntitySnapshot {
    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            id: entity.id,
            position_raw: entity.position_raw(),
            capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
            score_at_0x88: entity
                .authored_follow_beacon_priority_raw
                .map_or(RetailRuntimeValue::Unresolved, RetailRuntimeValue::Known),
            collision: entity.collision.clone(),
        }
    }

    fn as_selector_ref(&self) -> FollowBeaconEntityRef<'_> {
        FollowBeaconEntityRef {
            id: self.id,
            position_raw: self.position_raw,
            capability_flags: self.capability_flags,
            score_at_0x88: self.score_at_0x88,
            collision: &self.collision,
        }
    }
}

/// Publish variant zero after the impact owner has selected class 33.
///
/// The caller must pass the context derived from the already-authenticated
/// weighted selection. This function is crate-private so that world code
/// cannot forge the weighted-selection entry that authorizes publication.
pub(crate) fn publish_type17_follow_beacons_acquiring(
    entity: &mut Entity,
    selected_context: BehaviorContextRuntime,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
) -> Result<Type17FollowBeaconsAcquiringPublication, Type17FollowBeaconsLiveError> {
    validate_shared_type17_follow_state(entity, metadata)?;
    if !matches!(
        entity.actor_common_axis_descriptor,
        RetailRuntimeValue::Known(descriptor)
            if is_type17_reselection_axis_descriptor(descriptor)
    ) {
        return Err(
            Type17FollowBeaconsLiveError::UnexpectedActorCommonAxisDescriptor {
                actual: entity.actor_common_axis_descriptor,
            },
        );
    }
    validate_named_follow_context(
        selected_context,
        FollowBeaconsVariant::Acquiring,
        FollowContextTargetExpectation::Preserved,
    )?;
    Ok(publish_type17_follow_beacons_acquiring_with_prepare(
        entity,
        selected_context,
        metadata,
        world_fx,
        |preparation, owner_position_raw, metadata| {
            Ok::<_, Infallible>(
                prepare_follow_beacons_acquiring_runtime_task(
                    preparation,
                    owner_position_raw,
                    metadata,
                )
                .expect("authenticated type-17 Follow metadata must prepare variant zero"),
            )
        },
    ))
}

fn publish_type17_follow_beacons_acquiring_with_prepare<E>(
    entity: &mut Entity,
    selected_context: BehaviorContextRuntime,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
    mut prepare: impl FnMut(
        FollowBeaconsTaskPreparation,
        [i16; 3],
        &EntityTypeRuntimeMetadata,
    ) -> Result<PreparedSharedGenericRuntimeTask, E>,
) -> Type17FollowBeaconsAcquiringPublication {
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(selected_context));
    let program = behavior_program(u32::from(FOLLOW_BEACONS_BEHAVIOR_CLASS_ID))
        .expect("Follow Beacons is an authored named program");
    let policy = initial_behavior_state_policy(program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);

    let owner_position_raw = entity.position_raw();
    let mut constructors_by_phase = [None; 2];
    let setup_result: Result<(), FollowBeaconsTaskSetupError<E>> = {
        let Entity {
            actor_tasks,
            sub_a_propulsion_runtime,
            sub_h_external_frame_runtime,
            ..
        } = entity;
        let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
            unreachable!("live preflight retained exact Sub-A storage")
        };
        let RetailRuntimeValue::Known(Some(sub_h)) = sub_h_external_frame_runtime else {
            unreachable!("live preflight retained exact Sub-H storage")
        };

        apply_follow_beacons_acquiring_task_setup(actor_tasks, |preparation| {
            let phase_index = preparation.phase_index;
            let prepared = prepare(preparation, owner_position_raw, metadata)?;
            Ok(prepared.apply_suffix(
                || u32::from(world_fx.next_shared_retail_random_u16()),
                |effect| {
                    apply_generic_constructor_effect(
                        effect,
                        sub_h,
                        sub_a,
                        &mut constructors_by_phase[phase_index],
                    )
                },
            ))
        })
    };

    match setup_result {
        Ok(()) => {
            let primary_task_id = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .expect("successful variant-zero setup published Primary");
            let secondary_task_id = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Secondary)
                .expect("successful variant-zero setup published Secondary");
            Type17FollowBeaconsAcquiringPublication::Published {
                owner: Type17FollowBeaconsAcquiringOwner {
                    entity_id: entity.id,
                    primary_task_id,
                    secondary_task_id,
                },
                constructors_by_phase: constructors_by_phase.map(|receipt| {
                    receipt.expect("successful variant-zero setup applied both suffixes")
                }),
            }
        }
        Err(error) => {
            let failure = Type17FollowInitializerFailure {
                phase_index: error.phase_index,
                slot: error.slot,
                role: error.role,
            };
            entity.publish_behavior_initializer_failure_fallback(selected_context);
            Type17FollowBeaconsAcquiringPublication::InitializerFallbackPublished {
                failure,
                constructors_by_phase,
            }
        }
    }
}

impl Type17FollowBeaconsAcquiringOwner {
    /// Commit variant zero's callback prefix, rank the supplied live snapshots,
    /// and synchronously install variant one for a positive target.
    pub fn acquire_and_handoff(
        self,
        entity: &mut Entity,
        metadata: &EntityTypeRuntimeMetadata,
        candidates_in_intrusive_order: &[FollowBeaconLiveEntitySnapshot],
        world_fx: &mut WorldFx,
    ) -> Result<Type17FollowBeaconsAcquisitionOutcome, Type17FollowBeaconsLiveError> {
        self.acquire_and_handoff_with_prepare(
            entity,
            metadata,
            candidates_in_intrusive_order,
            world_fx,
            |preparation, owner_id, owner_position_raw, metadata| {
                Ok::<_, Infallible>(
                    prepare_follow_beacons_following_runtime_task(
                        preparation,
                        owner_id,
                        owner_position_raw,
                        metadata,
                    )
                    .expect("authenticated type-17 Follow metadata must prepare variant one"),
                )
            },
        )
    }

    fn acquire_and_handoff_with_prepare<E>(
        self,
        entity: &mut Entity,
        metadata: &EntityTypeRuntimeMetadata,
        candidates_in_intrusive_order: &[FollowBeaconLiveEntitySnapshot],
        world_fx: &mut WorldFx,
        prepare: impl FnOnce(
            FollowBeaconsFollowingTaskPreparation,
            u32,
            [i16; 3],
            &EntityTypeRuntimeMetadata,
        ) -> Result<PreparedFollowBeaconsFollowingRuntimeTask, E>,
    ) -> Result<Type17FollowBeaconsAcquisitionOutcome, Type17FollowBeaconsLiveError> {
        validate_acquiring_owner(self, entity, metadata)?;

        let owner_collision = entity.collision.clone();
        let owner_ref = FollowBeaconOwnerRef {
            id: entity.id,
            position_raw: entity.position_raw(),
            collision: &owner_collision,
        };
        let candidate_refs = candidates_in_intrusive_order
            .iter()
            .map(FollowBeaconLiveEntitySnapshot::as_selector_ref)
            .collect::<Vec<_>>();

        // FUN_00402120 commits this actor-local write before any selector read
        // or tie RNG. The detached task's folded copy is kept synchronized.
        entity.actor_common_axis_descriptor =
            RetailRuntimeValue::Known(TYPE17_FOLLOW_BEACONS_LIVE_AXIS_DESCRIPTOR);
        let prefix = match entity.actor_tasks.task_state_mut(self.secondary_task_id) {
            Some(ActorTaskRuntime::FollowBeaconAcquisition(state)) => state.before_callback(),
            _ => return Err(Type17FollowBeaconsLiveError::TaskLeaseChanged),
        };
        debug_assert_eq!(
            prefix.route_range().raw(),
            TYPE17_FOLLOW_BEACONS_LIVE_AXIS_DESCRIPTOR.strict_axis_limit_raw
        );

        let selection = select_follow_beacon(
            FollowBeaconSelectionRequest {
                prefix,
                owner: owner_ref,
                candidates_in_intrusive_order: &candidate_refs,
            },
            || u32::from(world_fx.next_shared_retail_random_u16()),
        )
        .map_err(Type17FollowBeaconsLiveError::CandidateSelection)?;

        match selection {
            FollowBeaconSelection::TaggedNoPositiveScore {
                output_write,
                retail_tag_address,
            } => Ok(Type17FollowBeaconsAcquisitionOutcome::NoPositiveScore {
                output_write,
                retail_tag_address,
            }),
            FollowBeaconSelection::Success { target } => {
                Ok(publish_type17_follow_beacons_following_with_prepare(
                    entity, metadata, target, world_fx, prepare,
                ))
            }
        }
    }
}

fn publish_type17_follow_beacons_following_with_prepare<E>(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    target: FollowBeaconTarget,
    world_fx: &mut WorldFx,
    prepare: impl FnOnce(
        FollowBeaconsFollowingTaskPreparation,
        u32,
        [i16; 3],
        &EntityTypeRuntimeMetadata,
    ) -> Result<PreparedFollowBeaconsFollowingRuntimeTask, E>,
) -> Type17FollowBeaconsAcquisitionOutcome {
    let previous_context = match entity.current_behavior_context {
        RetailRuntimeValue::Known(Some(context)) => context,
        _ => unreachable!("acquiring owner authenticated its live context"),
    };
    let program = behavior_program(u32::from(FOLLOW_BEACONS_BEHAVIOR_CLASS_ID))
        .expect("Follow Beacons is an authored named program");
    let following_style = *audited_behavior_style(u32::from(FOLLOW_BEACONS_BEHAVIOR_CLASS_ID), 1)
        .expect("Follow Beacons variant one is audited");
    let following_context = BehaviorContextRuntime::named_audited(
        program,
        1,
        RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
        RetailRuntimeValue::Known(Some(target.id)),
        previous_context.auxiliary_word_at_0x0c(),
        following_style,
    )
    .expect("canonical Follow Beacons variant-one context");
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(following_context));

    let owner_id = entity.id;
    let owner_position_raw = entity.position_raw();
    let mut generic_receipt = None;
    let mut final_target_speed = None;
    let setup_result: Result<(), FollowBeaconsFollowingTaskSetupError<E>> = {
        let Entity {
            actor_tasks,
            sub_a_propulsion_runtime,
            sub_h_external_frame_runtime,
            ..
        } = entity;
        let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
            unreachable!("live preflight retained exact Sub-A storage")
        };
        let RetailRuntimeValue::Known(Some(sub_h)) = sub_h_external_frame_runtime else {
            unreachable!("live preflight retained exact Sub-H storage")
        };
        apply_follow_beacons_following_task_setup(actor_tasks, target.id, |preparation| {
            let prepared = prepare(preparation, owner_id, owner_position_raw, metadata)?;
            Ok(prepared.apply_suffix(
                || u32::from(world_fx.next_shared_retail_random_u16()),
                |effect| match effect {
                    FollowBeaconsFollowingConstructorEffect::Generic(effect) => {
                        apply_generic_constructor_effect(
                            effect,
                            sub_h,
                            sub_a,
                            &mut generic_receipt,
                        );
                    }
                    FollowBeaconsFollowingConstructorEffect::ApplyFixedSubATargetSpeed {
                        owner_id: effect_owner,
                        suffix,
                    } => {
                        debug_assert_eq!(effect_owner, owner_id);
                        if let Some(target_speed_raw) = suffix.sub_a_target_speed_raw() {
                            sub_a.apply_shared_initializer_target_speed_write(target_speed_raw);
                            final_target_speed = Some(target_speed_raw);
                        }
                    }
                },
            ))
        })
    };

    match setup_result {
        Ok(()) => {
            let primary_task_id = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .expect("successful variant-one setup published Primary");
            Type17FollowBeaconsAcquisitionOutcome::FollowingPublished {
                target,
                owner: Type17FollowBeaconsFollowingOwner {
                    entity_id: entity.id,
                    primary_task_id,
                    target_id: target.id,
                },
                constructor: Type17FollowFollowingConstructorEvidence {
                    generic: generic_receipt
                        .expect("successful variant-one setup applied generic suffix"),
                    final_sub_a_target_speed_raw: final_target_speed
                        .expect("type-17 variant one applies fixed Sub-A speed"),
                },
            }
        }
        Err(error) => {
            let failure = Type17FollowInitializerFailure {
                phase_index: 0,
                slot: error.slot,
                role: error.role,
            };
            entity.publish_behavior_initializer_failure_fallback(following_context);
            Type17FollowBeaconsAcquisitionOutcome::InitializerFallbackPublished { target, failure }
        }
    }
}

fn apply_generic_constructor_effect(
    effect: SharedGenericConstructorEffect,
    sub_h: &mut SubHRuntimeState,
    sub_a: &mut SubAPropulsionRuntime,
    receipt: &mut Option<Type17FollowGenericConstructorEvidence>,
) {
    match effect {
        SharedGenericConstructorEffect::WriteSubHState08 { value } => {
            debug_assert_eq!(value, 1);
            sub_h.set_enabled(value != 0);
        }
        SharedGenericConstructorEffect::WriteSubADirection {
            direction_multiplier,
        } => sub_a.set_direction_multiplier(direction_multiplier),
        SharedGenericConstructorEffect::WriteSubATargetSpeed {
            target_speed_raw,
            random_sample_low16,
        } => {
            sub_a.apply_shared_initializer_target_speed_write(target_speed_raw);
            *receipt = Some(Type17FollowGenericConstructorEvidence {
                random_sample_low16,
                sub_a_target_speed_raw: target_speed_raw,
            });
        }
    }
}

fn validate_acquiring_owner(
    owner: Type17FollowBeaconsAcquiringOwner,
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type17FollowBeaconsLiveError> {
    if owner.entity_id != entity.id {
        return Err(Type17FollowBeaconsLiveError::OwnerRuntimeMismatch {
            planned: owner.entity_id,
            actual: entity.id,
        });
    }
    validate_shared_type17_follow_state(entity, metadata)?;
    validate_follow_context(
        entity,
        FollowBeaconsVariant::Acquiring,
        FollowContextTargetExpectation::Preserved,
    )?;
    if !matches!(
        entity.actor_common_axis_descriptor,
        RetailRuntimeValue::Known(descriptor)
            if is_type17_reselection_axis_descriptor(descriptor)
    ) {
        return Err(
            Type17FollowBeaconsLiveError::UnexpectedActorCommonAxisDescriptor {
                actual: entity.actor_common_axis_descriptor,
            },
        );
    }
    if entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) != Some(owner.primary_task_id)
        || entity.actor_tasks.task_in_slot(ActorTaskSlot::Secondary)
            != Some(owner.secondary_task_id)
        || entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary)
            .is_some()
        || !matches!(
            entity.actor_tasks.task_state(owner.primary_task_id),
            Some(ActorTaskRuntime::SharedRetarget(_))
        )
        || !matches!(
            entity.actor_tasks.task_state(owner.secondary_task_id),
            Some(ActorTaskRuntime::FollowBeaconAcquisition(_))
        )
    {
        return Err(Type17FollowBeaconsLiveError::TaskLeaseChanged);
    }
    Ok(())
}

fn validate_shared_type17_follow_state(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type17FollowBeaconsLiveError> {
    if !entity.active {
        return Err(Type17FollowBeaconsLiveError::EntityInactive);
    }
    if entity.entity_type != TYPE17_IMPACT_ENTITY_TYPE {
        return Err(Type17FollowBeaconsLiveError::UnexpectedEntityType {
            actual: entity.entity_type,
        });
    }
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(Type17FollowBeaconsLiveError::MissingInitializer)?;
    if initializer.common_axis_descriptor != TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR {
        return Err(
            Type17FollowBeaconsLiveError::UnexpectedMetadataCommonAxisDescriptor {
                actual: initializer.common_axis_descriptor,
            },
        );
    }
    match metadata.common_mover_topology {
        RetailRuntimeValue::Known(topology) if topology == TYPE17_MODEL256_COMPONENT_TOPOLOGY => {}
        RetailRuntimeValue::Known(_) => {
            return Err(Type17FollowBeaconsLiveError::UnexpectedComponentTopology)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17FollowBeaconsLiveError::ComponentTopologyUnresolved)
        }
    }
    match metadata.sub_a_propulsion_descriptor {
        RetailRuntimeValue::Known(Some(descriptor))
            if descriptor.target_speed_base_raw == TYPE17_MODEL256_SUB_A_TARGET_SPEED_BASE_RAW => {}
        RetailRuntimeValue::Known(Some(descriptor)) => {
            return Err(
                Type17FollowBeaconsLiveError::UnexpectedSubATargetSpeedBase {
                    actual: descriptor.target_speed_base_raw,
                },
            )
        }
        RetailRuntimeValue::Known(None) => {
            return Err(Type17FollowBeaconsLiveError::SubADescriptorAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17FollowBeaconsLiveError::SubADescriptorUnresolved)
        }
    }
    match &metadata.sub_h_external_frame_descriptor {
        RetailRuntimeValue::Known(Some(descriptor))
            if descriptor.records.len() == TYPE17_MODEL256_SUB_H_RECORD_COUNT => {}
        RetailRuntimeValue::Known(Some(descriptor)) => {
            return Err(
                Type17FollowBeaconsLiveError::UnexpectedSubHDescriptorRecordCount {
                    actual: descriptor.records.len(),
                },
            )
        }
        RetailRuntimeValue::Known(None) => {
            return Err(Type17FollowBeaconsLiveError::SubHDescriptorAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17FollowBeaconsLiveError::SubHDescriptorUnresolved)
        }
    }
    match entity.sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(_)) => {}
        RetailRuntimeValue::Known(None) => {
            return Err(Type17FollowBeaconsLiveError::SubARuntimeAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17FollowBeaconsLiveError::SubARuntimeUnresolved)
        }
    }
    match &entity.sub_h_external_frame_runtime {
        RetailRuntimeValue::Known(Some(runtime))
            if runtime.records().len() == TYPE17_MODEL256_SUB_H_RECORD_COUNT => {}
        RetailRuntimeValue::Known(Some(runtime)) => {
            return Err(
                Type17FollowBeaconsLiveError::UnexpectedSubHRuntimeRecordCount {
                    actual: runtime.records().len(),
                },
            )
        }
        RetailRuntimeValue::Known(None) => {
            return Err(Type17FollowBeaconsLiveError::SubHRuntimeAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17FollowBeaconsLiveError::SubHRuntimeUnresolved)
        }
    }
    if entity.actor_common_axis_descriptor == RetailRuntimeValue::Unresolved {
        return Err(Type17FollowBeaconsLiveError::ActorCommonAxisDescriptorUnresolved);
    }
    Ok(())
}

fn validate_follow_context(
    entity: &Entity,
    variant: FollowBeaconsVariant,
    target: FollowContextTargetExpectation,
) -> Result<(), Type17FollowBeaconsLiveError> {
    let context = match entity.current_behavior_context {
        RetailRuntimeValue::Known(Some(context)) => context,
        RetailRuntimeValue::Known(None) => {
            return Err(Type17FollowBeaconsLiveError::BehaviorContextAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17FollowBeaconsLiveError::BehaviorContextUnresolved)
        }
    };
    validate_named_follow_context(context, variant, target)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FollowContextTargetExpectation {
    /// Descriptor reselection preserves this independent context word, which
    /// may be known, null, or unresolved.
    Preserved,
    Exact(u32),
}

fn validate_named_follow_context(
    context: BehaviorContextRuntime,
    variant: FollowBeaconsVariant,
    target: FollowContextTargetExpectation,
) -> Result<(), Type17FollowBeaconsLiveError> {
    let expected_style =
        audited_behavior_style(u32::from(FOLLOW_BEACONS_BEHAVIOR_CLASS_ID), variant as u8).copied();
    let actual_style = match context.active_style() {
        ActiveBehaviorStyle::Audited(style) => Some(style),
        ActiveBehaviorStyle::InitializerFailureFallback => None,
    };
    let target_matches = match target {
        FollowContextTargetExpectation::Preserved => true,
        FollowContextTargetExpectation::Exact(target_id) => {
            context.target_handle_at_0x08() == RetailRuntimeValue::Known(Some(target_id))
        }
    };
    if !matches!(
        context.descriptor(),
        BehaviorDescriptorIdentity::Named(program)
            if program.class_id == FOLLOW_BEACONS_BEHAVIOR_CLASS_ID
    ) || context.choice_list_source()
        != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        || context.style_table_index_raw_at_0x10() != variant as u32
        || actual_style != expected_style
        || !target_matches
    {
        return Err(Type17FollowBeaconsLiveError::UnexpectedBehaviorContext);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        actor_task_owner::PreparedActorTask,
        entity::EntityKind,
        entity_collision_state::{EntityInitializerSpec, RetailStateWord},
        follow_beacons::FOLLOW_BEACON_REQUIRED_CAPABILITY_MASK,
        shared_retarget_mover::SharedRetargetTaskState,
        type17_impact_live::TYPE17_CAPTURE_PEOPLE_LIVE_AXIS_DESCRIPTOR,
    };
    use v2k_formats::collision::{
        SubAPropulsionDescriptor, SubHExternalFrameDescriptor, SubHExternalFrameRecord,
    };

    const ENTITY_ID: u32 = 0x0497_0001;
    const OLD_TARGET_ID: u32 = 0x047f_0001;
    const AUXILIARY_WORD: u32 = 0x1357_9bdf;
    const BASE_STATE: u32 = 0xa506_8001;

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: 57,
                common_axis_descriptor: TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR,
                behavior_choices: Box::new([]),
                behavior_rule_ref: 0,
                alternate_behavior_class_ref: 0,
            }),
            common_mover_topology: RetailRuntimeValue::Known(TYPE17_MODEL256_COMPONENT_TOPOLOGY),
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                SubAPropulsionDescriptor {
                    acceleration_raw: 100,
                    overspeed_correction_raw: 200,
                    target_speed_base_raw: TYPE17_MODEL256_SUB_A_TARGET_SPEED_BASE_RAW,
                },
            )),
            sub_h_external_frame_descriptor: RetailRuntimeValue::Known(Some(
                SubHExternalFrameDescriptor {
                    completion_sound_id: None,
                    records: vec![
                        SubHExternalFrameRecord {
                            resolver_flags_raw: 0,
                            phase_rate_raw: 0,
                            vertex_refs: [0; 3],
                            axis_mode_raw: 0,
                            dependencies: [0; 4],
                        };
                        TYPE17_MODEL256_SUB_H_RECORD_COUNT
                    ],
                },
            )),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn acquiring_context() -> BehaviorContextRuntime {
        let program = behavior_program(u32::from(FOLLOW_BEACONS_BEHAVIOR_CLASS_ID))
            .expect("Follow Beacons program");
        let style = *audited_behavior_style(
            u32::from(FOLLOW_BEACONS_BEHAVIOR_CLASS_ID),
            FollowBeaconsVariant::Acquiring as u8,
        )
        .expect("Follow Beacons acquiring style");
        BehaviorContextRuntime::named_audited(
            program,
            FollowBeaconsVariant::Acquiring as u32,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(Some(OLD_TARGET_ID)),
            RetailRuntimeValue::Known(AUXILIARY_WORD),
            style,
        )
        .expect("canonical acquiring context")
    }

    fn seeded_task(lifetime_ms: u32) -> PreparedActorTask<ActorTaskRuntime> {
        PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
            SharedRetargetTaskState::new([1, 2, 3], lifetime_ms),
        ))
    }

    fn exact_entity() -> Entity {
        let mut entity = Entity::unresolved_port_entity(
            ENTITY_ID,
            EntityKind::Unknown(TYPE17_IMPACT_ENTITY_TYPE),
            TYPE17_IMPACT_ENTITY_TYPE,
        );
        entity.position = [1.0, 2.0, 3.0];
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(acquiring_context()));
        entity.collision = EntityCollisionRuntimeState::from_constructor(
            None,
            0,
            RetailStateWord::exact(BASE_STATE),
        );
        entity.actor_common_axis_descriptor =
            RetailRuntimeValue::Known(TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR);
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Unresolved, -1, 73),
        ));
        let mut sub_h = SubHRuntimeState::new(TYPE17_MODEL256_SUB_H_RECORD_COUNT).unwrap();
        sub_h.set_enabled(false);
        entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(sub_h));
        for (index, slot) in ActorTaskSlot::IN_RETAIL_TICK_ORDER.into_iter().enumerate() {
            entity
                .actor_tasks
                .replace_prepared(slot, seeded_task(9_000 + index as u32));
        }
        entity
    }

    fn beacon_snapshot(
        id: u32,
        score_raw: Option<i32>,
        capability_flags: u32,
    ) -> FollowBeaconLiveEntitySnapshot {
        let mut entity = Entity::unresolved_port_entity(id, EntityKind::Trigger, 52);
        entity.position = [1.25, 2.0, 3.0];
        entity.authored_follow_beacon_priority_raw = score_raw;
        entity.capability_flags = capability_flags;
        entity.collision =
            EntityCollisionRuntimeState::from_constructor(None, 0, RetailStateWord::exact(1));
        FollowBeaconLiveEntitySnapshot::from_entity(&entity)
    }

    fn publish_acquiring(
        entity: &mut Entity,
        metadata: &EntityTypeRuntimeMetadata,
        world_fx: &mut WorldFx,
    ) -> (
        Type17FollowBeaconsAcquiringOwner,
        [Type17FollowGenericConstructorEvidence; 2],
    ) {
        match publish_type17_follow_beacons_acquiring(
            entity,
            acquiring_context(),
            metadata,
            world_fx,
        )
        .unwrap()
        {
            Type17FollowBeaconsAcquiringPublication::Published {
                owner,
                constructors_by_phase,
            } => (owner, constructors_by_phase),
            publication => panic!("unexpected acquiring publication: {publication:?}"),
        }
    }

    fn task_ids(entity: &Entity) -> [Option<ActorTaskId>; 3] {
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot))
    }

    #[test]
    fn acquiring_publication_commits_two_generic_suffixes_in_phase_order() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let mut world_fx = WorldFx::new();

        let (owner, constructors) = publish_acquiring(&mut entity, &metadata, &mut world_fx);

        assert_eq!(owner.entity_id(), ENTITY_ID);
        assert_eq!(
            constructors,
            [
                Type17FollowGenericConstructorEvidence {
                    random_sample_low16: 0x0026,
                    sub_a_target_speed_raw: 250,
                },
                Type17FollowGenericConstructorEvidence {
                    random_sample_low16: 0x1e27,
                    sub_a_target_speed_raw: 252,
                },
            ]
        );
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            Some(owner.primary_task_id())
        );
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Secondary),
            Some(owner.secondary_task_id())
        );
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary),
            None
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(_))
        ));
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::FollowBeaconAcquisition(_))
        ));
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR)
        );
        validate_follow_context(
            &entity,
            FollowBeaconsVariant::Acquiring,
            FollowContextTargetExpectation::Preserved,
        )
        .unwrap();
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("acquiring context missing")
        };
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(OLD_TARGET_ID))
        );
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("Sub-A missing")
        };
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(252));
        assert_eq!(sub_a.direction_multiplier(), 1);
        let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime else {
            panic!("Sub-H missing")
        };
        assert!(sub_h.is_enabled());
        assert_eq!(world_fx.next_shared_retail_random_u16(), 0xd2f6);
    }

    #[test]
    fn ranked_snapshot_handoff_is_signed_tie_aware_and_publishes_following() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let mut untouched_actor = exact_entity();
        untouched_actor.id = ENTITY_ID + 1;
        let mut world_fx = WorldFx::new();
        let (owner, _) = publish_acquiring(&mut entity, &metadata, &mut world_fx);
        let candidates = [
            beacon_snapshot(0x0520_0001, Some(-1), 0x100),
            beacon_snapshot(0x0520_0002, Some(11), 0x100),
            beacon_snapshot(0x0520_0003, Some(11), 0x100),
        ];

        let outcome = owner
            .acquire_and_handoff(&mut entity, &metadata, &candidates, &mut world_fx)
            .unwrap();
        let Type17FollowBeaconsAcquisitionOutcome::FollowingPublished {
            target,
            owner: following_owner,
            constructor,
        } = outcome
        else {
            panic!("expected following publication: {outcome:?}")
        };

        // The equal-score candidate consumes 0xd2f6; its even low bit leaves
        // the first score-11 candidate selected. The following constructor
        // then consumes the next shared word before its fixed-speed overwrite.
        assert_eq!(
            target,
            FollowBeaconTarget {
                id: 0x0520_0002,
                score_raw: 11
            }
        );
        assert_eq!(
            constructor,
            Type17FollowFollowingConstructorEvidence {
                generic: Type17FollowGenericConstructorEvidence {
                    random_sample_low16: 0x0985,
                    sub_a_target_speed_raw: 250,
                },
                final_sub_a_target_speed_raw: 333,
            }
        );
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(TYPE17_FOLLOW_BEACONS_LIVE_AXIS_DESCRIPTOR)
        );
        assert_eq!(
            untouched_actor.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR)
        );
        assert_eq!(
            metadata
                .initializer
                .as_ref()
                .unwrap()
                .common_axis_descriptor,
            TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR
        );
        validate_follow_context(
            &entity,
            FollowBeaconsVariant::Following,
            FollowContextTargetExpectation::Exact(target.id),
        )
        .unwrap();
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Secondary),
            None
        );
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary),
            None
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::FollowBeaconsFollowing(state))
                if state.target_id() == target.id
        ));
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("Sub-A missing")
        };
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(333));
        following_owner.validate(&entity, &metadata).unwrap();
        assert_eq!(world_fx.next_shared_retail_random_u16(), 0xa297);
    }

    #[test]
    fn no_positive_score_commits_prefix_without_rng_or_one_shot_owner() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        entity.actor_common_axis_descriptor =
            RetailRuntimeValue::Known(TYPE17_CAPTURE_PEOPLE_LIVE_AXIS_DESCRIPTOR);
        let mut world_fx = WorldFx::new();
        let (owner, _) = publish_acquiring(&mut entity, &metadata, &mut world_fx);
        let tasks_before = task_ids(&entity);
        let candidates = [beacon_snapshot(0x0520_0001, Some(-7), 0x100)];

        for _ in 0..2 {
            assert_eq!(
                owner
                    .acquire_and_handoff(&mut entity, &metadata, &candidates, &mut world_fx)
                    .unwrap(),
                Type17FollowBeaconsAcquisitionOutcome::NoPositiveScore {
                    output_write: None,
                    retail_tag_address:
                        crate::follow_beacons::FOLLOW_BEACON_NO_POSITIVE_SCORE_TAG_ADDRESS,
                }
            );
        }

        assert_eq!(task_ids(&entity), tasks_before);
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(TYPE17_FOLLOW_BEACONS_LIVE_AXIS_DESCRIPTOR)
        );
        let Some(ActorTaskRuntime::FollowBeaconAcquisition(state)) =
            entity.actor_task_state(ActorTaskSlot::Secondary)
        else {
            panic!("acquisition task missing")
        };
        assert_eq!(state.filter_raw(), FOLLOW_BEACON_REQUIRED_CAPABILITY_MASK);
        validate_follow_context(
            &entity,
            FollowBeaconsVariant::Acquiring,
            FollowContextTargetExpectation::Preserved,
        )
        .unwrap();
        assert_eq!(world_fx.next_shared_retail_random_u16(), 0xd2f6);
    }

    #[test]
    fn malformed_axis_preflight_fails_before_context_tasks_or_rng() {
        for (axis, expected) in [
            (
                RetailRuntimeValue::Unresolved,
                Type17FollowBeaconsLiveError::ActorCommonAxisDescriptorUnresolved,
            ),
            (
                RetailRuntimeValue::Known(CommonAxisDescriptor {
                    strict_axis_limit_raw: 0x0a00,
                    raw_word_at_0x04: 7,
                }),
                Type17FollowBeaconsLiveError::UnexpectedActorCommonAxisDescriptor {
                    actual: RetailRuntimeValue::Known(CommonAxisDescriptor {
                        strict_axis_limit_raw: 0x0a00,
                        raw_word_at_0x04: 7,
                    }),
                },
            ),
        ] {
            let metadata = exact_metadata();
            let mut entity = exact_entity();
            entity.actor_common_axis_descriptor = axis;
            let context_before = entity.current_behavior_context;
            let state_before = entity.collision.state_flags_at_0x08;
            let tasks_before = task_ids(&entity);
            let mut world_fx = WorldFx::new();

            assert_eq!(
                publish_type17_follow_beacons_acquiring(
                    &mut entity,
                    acquiring_context(),
                    &metadata,
                    &mut world_fx,
                ),
                Err(expected)
            );
            assert_eq!(entity.current_behavior_context, context_before);
            assert_eq!(entity.collision.state_flags_at_0x08, state_before);
            assert_eq!(task_ids(&entity), tasks_before);
            assert_eq!(entity.actor_common_axis_descriptor, axis);
            assert_eq!(world_fx.next_shared_retail_random_u16(), 0x0026);
        }
    }

    #[test]
    fn stale_acquiring_receipt_fails_before_prefix_or_selector_rng() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let mut world_fx = WorldFx::new();
        let (owner, _) = publish_acquiring(&mut entity, &metadata, &mut world_fx);
        entity
            .actor_tasks
            .replace_prepared(ActorTaskSlot::Secondary, seeded_task(0x7777));

        assert_eq!(
            owner.acquire_and_handoff(
                &mut entity,
                &metadata,
                &[beacon_snapshot(0x0520_0001, Some(12), 0x100)],
                &mut world_fx,
            ),
            Err(Type17FollowBeaconsLiveError::TaskLeaseChanged)
        );
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR)
        );
        assert_eq!(world_fx.next_shared_retail_random_u16(), 0xd2f6);
    }

    #[test]
    fn selector_error_retains_prefix_and_any_earlier_tie_draw() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let mut world_fx = WorldFx::new();
        let (owner, _) = publish_acquiring(&mut entity, &metadata, &mut world_fx);
        let tasks_before = task_ids(&entity);
        let unresolved_id = 0x0520_0002;
        let candidates = [
            // Equality with the initial best score consumes 0xd2f6 before
            // the later candidate exposes its unresolved score.
            beacon_snapshot(0x0520_0001, Some(0), 0x100),
            beacon_snapshot(unresolved_id, None, 0x100),
        ];

        assert_eq!(
            owner.acquire_and_handoff(&mut entity, &metadata, &candidates, &mut world_fx),
            Err(Type17FollowBeaconsLiveError::CandidateSelection(
                FollowBeaconSelectionError::CandidateScoreUnresolved { id: unresolved_id }
            ))
        );
        assert_eq!(task_ids(&entity), tasks_before);
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(TYPE17_FOLLOW_BEACONS_LIVE_AXIS_DESCRIPTOR)
        );
        let Some(ActorTaskRuntime::FollowBeaconAcquisition(state)) =
            entity.actor_task_state(ActorTaskSlot::Secondary)
        else {
            panic!("acquisition task missing")
        };
        assert_eq!(state.filter_raw(), FOLLOW_BEACON_REQUIRED_CAPABILITY_MASK);
        assert_eq!(world_fx.next_shared_retail_random_u16(), 0x0985);
    }

    #[test]
    fn following_owner_rejects_changed_context_and_replaced_primary() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let mut world_fx = WorldFx::new();
        let (owner, _) = publish_acquiring(&mut entity, &metadata, &mut world_fx);
        let outcome = owner
            .acquire_and_handoff(
                &mut entity,
                &metadata,
                &[beacon_snapshot(0x0520_0001, Some(12), 0x100)],
                &mut world_fx,
            )
            .unwrap();
        let Type17FollowBeaconsAcquisitionOutcome::FollowingPublished {
            owner: following_owner,
            ..
        } = outcome
        else {
            panic!("expected following publication: {outcome:?}")
        };
        following_owner.validate(&entity, &metadata).unwrap();

        let RetailRuntimeValue::Known(Some(original_context)) = entity.current_behavior_context
        else {
            panic!("following context missing")
        };
        let program = behavior_program(u32::from(FOLLOW_BEACONS_BEHAVIOR_CLASS_ID)).unwrap();
        let style = *audited_behavior_style(
            u32::from(FOLLOW_BEACONS_BEHAVIOR_CLASS_ID),
            FollowBeaconsVariant::Following as u8,
        )
        .unwrap();
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(
            BehaviorContextRuntime::named_audited(
                program,
                FollowBeaconsVariant::Following as u32,
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
                RetailRuntimeValue::Known(Some(0x0520_00ff)),
                original_context.auxiliary_word_at_0x0c(),
                style,
            )
            .unwrap(),
        ));
        assert_eq!(
            following_owner.validate(&entity, &metadata),
            Err(Type17FollowBeaconsLiveError::UnexpectedBehaviorContext)
        );

        entity.current_behavior_context = RetailRuntimeValue::Known(Some(original_context));
        entity
            .actor_tasks
            .replace_prepared(ActorTaskSlot::Primary, seeded_task(0x8888));
        assert_eq!(
            following_owner.validate(&entity, &metadata),
            Err(Type17FollowBeaconsLiveError::TaskLeaseChanged)
        );
    }

    #[test]
    fn acquiring_initializer_failures_publish_fallback_after_prior_effects_only() {
        for failed_phase in [0, 1] {
            let metadata = exact_metadata();
            let mut entity = exact_entity();
            let mut world_fx = WorldFx::new();
            let publication = publish_type17_follow_beacons_acquiring_with_prepare(
                &mut entity,
                acquiring_context(),
                &metadata,
                &mut world_fx,
                |preparation, owner_position_raw, metadata| {
                    if preparation.phase_index == failed_phase {
                        return Err(preparation.phase_index);
                    }
                    prepare_follow_beacons_acquiring_runtime_task(
                        preparation,
                        owner_position_raw,
                        metadata,
                    )
                    .map_err(|_| usize::MAX)
                },
            );
            let Type17FollowBeaconsAcquiringPublication::InitializerFallbackPublished {
                failure,
                constructors_by_phase,
            } = publication
            else {
                panic!("expected acquiring fallback: {publication:?}")
            };

            assert_eq!(failure.phase_index, failed_phase);
            assert_eq!(
                failure.slot,
                if failed_phase == 0 {
                    ActorTaskSlot::Secondary
                } else {
                    ActorTaskSlot::Primary
                }
            );
            assert_eq!(
                failure.role,
                if failed_phase == 0 {
                    FollowBeaconsTaskRole::AcquireBeacon
                } else {
                    FollowBeaconsTaskRole::Retarget
                }
            );
            assert_eq!(
                constructors_by_phase,
                if failed_phase == 0 {
                    [None, None]
                } else {
                    [
                        Some(Type17FollowGenericConstructorEvidence {
                            random_sample_low16: 0x0026,
                            sub_a_target_speed_raw: 250,
                        }),
                        None,
                    ]
                }
            );
            assert_eq!(task_ids(&entity), [None; 3]);
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                panic!("fallback context missing")
            };
            assert_eq!(
                context.descriptor(),
                BehaviorDescriptorIdentity::InitializerFailureFallback
            );
            assert_eq!(
                context.target_handle_at_0x08(),
                RetailRuntimeValue::Known(Some(OLD_TARGET_ID))
            );
            let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
                panic!("Sub-A missing")
            };
            assert_eq!(
                sub_a.target_speed_raw(),
                if failed_phase == 0 {
                    RetailRuntimeValue::Unresolved
                } else {
                    RetailRuntimeValue::Known(250)
                }
            );
            assert_eq!(
                world_fx.next_shared_retail_random_u16(),
                if failed_phase == 0 { 0x0026 } else { 0x1e27 }
            );
        }
    }

    #[test]
    fn following_prepare_failure_keeps_prefix_and_target_but_consumes_no_constructor_rng() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let mut world_fx = WorldFx::new();
        let (owner, _) = publish_acquiring(&mut entity, &metadata, &mut world_fx);
        let target = beacon_snapshot(0x0520_0001, Some(12), 0x100);

        let outcome = owner
            .acquire_and_handoff_with_prepare(
                &mut entity,
                &metadata,
                &[target],
                &mut world_fx,
                |_preparation, _owner_id, _owner_position_raw, _metadata| Err(0x0bad_u32),
            )
            .unwrap();
        let Type17FollowBeaconsAcquisitionOutcome::InitializerFallbackPublished { target, failure } =
            outcome
        else {
            panic!("expected following fallback: {outcome:?}")
        };

        assert_eq!(
            target,
            FollowBeaconTarget {
                id: 0x0520_0001,
                score_raw: 12
            }
        );
        assert_eq!(failure.phase_index, 0);
        assert_eq!(failure.slot, ActorTaskSlot::Primary);
        assert_eq!(failure.role, FollowBeaconsTaskRole::FollowTarget);
        assert_eq!(task_ids(&entity), [None; 3]);
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(TYPE17_FOLLOW_BEACONS_LIVE_AXIS_DESCRIPTOR)
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("fallback context missing")
        };
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::InitializerFailureFallback
        );
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(target.id))
        );
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("Sub-A missing")
        };
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(252));
        assert_eq!(world_fx.next_shared_retail_random_u16(), 0xd2f6);
    }
}
