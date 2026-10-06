//! Live selected/fallback publisher for the bounded type-17 impact path.
//!
//! The detached planner owns `FUN_0040C690 -> FUN_0040AC60` evaluation and
//! the first process-RNG draw. This module authenticates the corresponding
//! live Entity storage and the wrapper's same-tick `+0x34` stamp before
//! allowing that draw, then composes selected class-9 Capture People,
//! class-10 Run Away, and class-33 Follow Beacons variant-zero publications
//! with the existing task owner, component state, and shared-constructor
//! suffixes.
//!
//! `FUN_0040B6C0` first restores the actor-local type-authored search filter,
//! then passes initial style `+0x44` into the phase-zero TargetAcquisition
//! constructor. Capture People supplies override `0x0C00`; Run Away supplies
//! zero. Both the persistent actor write and task-private override retain their
//! retail order across the enclosing initializer-failure fallback.

use v2k_formats::collision::{CommonAxisDescriptor, SubDSteeringDescriptor};

use crate::{
    actor_task_dispatcher::{
        prepare_shared_acquiring_runtime_task, PreparedSharedGenericRuntimeTask,
        SharedGenericConstructorEffect,
    },
    actor_task_owner::ActorTaskSlot,
    common_mover::SubAPropulsionRuntime,
    entity::Entity,
    entity_behavior::{
        initial_behavior_state_policy, ActiveBehaviorStyle, BehaviorChoiceListSource,
        BehaviorContextRuntime, BehaviorDescriptorIdentity,
    },
    entity_collision_state::{
        CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
    follow_beacons::FOLLOW_BEACON_REQUIRED_CAPABILITY_MASK,
    run_away::{
        apply_run_away_task_setup, RunAwayTaskPreparation, RunAwayTaskRole, RunAwayTaskSetupError,
        RunAwayTaskSetupRequest,
    },
    sub_h_external_frame::SubHRuntimeState,
    type17_follow_beacons_live::{
        publish_type17_follow_beacons_acquiring, Type17FollowBeaconsAcquiringPublication,
        Type17FollowBeaconsLiveError,
    },
    type17_impact_reselection::{
        plan_type17_impact_reselection, Type17ImpactEntityRef, Type17ImpactInstallDecision,
        Type17ImpactReselectionBoundary, Type17ImpactReselectionError,
        Type17ImpactReselectionOutcome, Type17ImpactReselectionRequest,
        Type17ImpactWeightedSelection, TYPE17_DYING_STATE_BIT, TYPE17_IMPACT_ENTITY_TYPE,
        TYPE17_IMPACT_MODEL_ID,
    },
    world_fx::WorldFx,
};

pub const TYPE17_MODEL256_SUB_A_TARGET_SPEED_BASE_RAW: i16 = 250;
pub const TYPE17_MODEL256_SUB_H_RECORD_COUNT: usize = 8;
pub const TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 2_560,
    raw_word_at_0x04: 3,
};
pub const TYPE17_CAPTURE_PEOPLE_LIVE_AXIS_DESCRIPTOR: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 2_560,
    raw_word_at_0x04: 0x0C00,
};
pub const TYPE17_FOLLOW_BEACONS_LIVE_AXIS_DESCRIPTOR: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 2_560,
    raw_word_at_0x04: FOLLOW_BEACON_REQUIRED_CAPABILITY_MASK,
};

pub fn is_type17_reselection_axis_descriptor(actual: CommonAxisDescriptor) -> bool {
    actual == TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR
        || actual == TYPE17_CAPTURE_PEOPLE_LIVE_AXIS_DESCRIPTOR
        || actual == TYPE17_FOLLOW_BEACONS_LIVE_AXIS_DESCRIPTOR
}

pub const TYPE17_MODEL256_COMPONENT_TOPOLOGY: CommonMoverComponentTopology =
    CommonMoverComponentTopology {
        sub_a: true,
        sub_b: true,
        sub_c: true,
        sub_d: true,
        sub_e: false,
        sub_f: false,
        sub_g: false,
        sub_h: true,
        sub_i: false,
        sub_j: true,
        sub_k: false,
        sub_l: false,
        sub_m: false,
        sub_n: false,
        sub_o: false,
    };

/// Exact first-world type-17 / model-256 Section-12 Sub-D descriptor.
///
/// The authored bytes match [`TYPE47_SUB_D`](crate::common_mover::sub_d::TYPE47_SUB_D)
/// including divisor 64 and probes `0x200/0x100`. Shared `FUN_0041F660`
/// therefore admits this record by value. The bytes do not authorize
/// Type-9's first-query reset (`0x17`). Type-17 first query uses the
/// `V200002.run` owner, not this descriptor alone.
pub const TYPE17_MODEL256_SUB_D: SubDSteeringDescriptor = SubDSteeringDescriptor {
    steering_divisor_raw: 64,
    couple_yaw_into_roll_raw: 0,
    enable_pitch_steering_raw: 0,
    forward_probe_raw: 0x0200,
    lateral_probe_raw: 0x0100,
    classifier_flags: 0x13,
    reserved_at_0x0b: 0,
};

/// Inputs outside the live Entity and process-shared RNG owner.
#[derive(Debug, Clone, Copy)]
pub struct Type17ImpactLiveRequest<'a> {
    pub metadata: &'a EntityTypeRuntimeMetadata,
    /// Must retain retail intrusive-list order.
    pub candidates_in_intrusive_order: &'a [Type17ImpactEntityRef],
    /// `DAT_004FED60` after the primary-hit wrapper stamped the same tick at
    /// entity `+0x34`.
    pub current_tick: u32,
}

/// Evidence unavailable before the selector is allowed to move shared RNG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type17ImpactLiveError {
    EntityInactive,
    UnexpectedEntityType {
        actual: u32,
    },
    UnexpectedEntityModelSlots {
        actual: [Option<usize>; 4],
    },
    CurrentBehaviorContextUnresolved,
    CurrentBehaviorContextAbsent,
    CurrentBehaviorContextIsInitializerFallback,
    CurrentBehaviorClassNotAuthoredForType17 {
        actual: u8,
    },
    CurrentBehaviorChoiceSourceUnresolved,
    CurrentBehaviorStyleIsInitializerFallback,
    CurrentBehaviorDescriptorStyleMismatch {
        descriptor_class: u8,
        style_class: u8,
    },
    DyingStateBitUnresolved,
    LastHitTickUnresolved,
    LastHitTickMismatch {
        expected: u32,
        actual: u32,
    },
    OwnerAttachedEntityUnresolved,
    ComponentTopologyUnresolved,
    UnexpectedComponentTopology {
        actual: CommonMoverComponentTopology,
    },
    MissingInitializer,
    UnexpectedCommonAxisDescriptor {
        actual: CommonAxisDescriptor,
    },
    ActorCommonAxisDescriptorUnresolved,
    UnexpectedActorCommonAxisDescriptor {
        actual: CommonAxisDescriptor,
    },
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
    Plan(Type17ImpactReselectionError),
    FollowBeacons(Type17FollowBeaconsLiveError),
}

/// One successful `FUN_00406070` Sub-H/Sub-A suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17SharedAcquiringConstructorEvidence {
    pub random_sample_low16: u16,
    pub sub_a_target_speed_raw: i32,
}

/// The initializer phase whose nonzero result was consumed by outer C6B0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17SharedAcquiringInitializerFailure {
    pub phase_index: usize,
    pub slot: ActorTaskSlot,
    pub role: RunAwayTaskRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17SharedAcquiringPublished {
    pub weighted: Type17ImpactWeightedSelection,
    /// Successful constructor suffixes indexed by acquiring phase.
    pub constructors_by_phase: [Type17SharedAcquiringConstructorEvidence; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17SharedAcquiringFallbackPublished {
    pub weighted: Type17ImpactWeightedSelection,
    pub failure: Type17SharedAcquiringInitializerFailure,
    /// Constructor suffixes committed before the failed allocation.
    pub constructors_by_phase: [Option<Type17SharedAcquiringConstructorEvidence>; 2],
}

/// Result of the class-9/class-10 `FUN_0040B6C0` initializer independent of
/// whether the selected descriptor came from construction or impact
/// reselection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type17SharedAcquiringPublication {
    Published {
        constructors_by_phase: [Type17SharedAcquiringConstructorEvidence; 2],
    },
    InitializerFallbackPublished {
        failure: Type17SharedAcquiringInitializerFailure,
        constructors_by_phase: [Option<Type17SharedAcquiringConstructorEvidence>; 2],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17FollowBeaconsPublished {
    pub weighted: Type17ImpactWeightedSelection,
    pub publication: Type17FollowBeaconsAcquiringPublication,
}

/// Final state of one synchronous impact-reselection publication attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type17ImpactLiveOutcome {
    /// Null/Capture-cleanup/dying paths remain outside this publisher and do
    /// not consume selector RNG.
    Boundary(Type17ImpactReselectionBoundary),
    FollowBeaconsPublished(Type17FollowBeaconsPublished),
    /// Class 9 Capture People and class 10 Run Away share `FUN_0040B6C0`.
    /// The selected class remains explicit in `weighted`.
    SharedAcquiringPublished(Type17SharedAcquiringPublished),
    /// Outer C6B0 consumed a nonzero initializer result and returned zero
    /// after publishing its special fallback. This is not an error/rollback.
    SharedAcquiringInitializerFallbackPublished(Type17SharedAcquiringFallbackPublished),
}

#[derive(Debug, Clone, Copy)]
struct Type17ImpactLivePreflight {
    current_context: BehaviorContextRuntime,
    current_style: crate::entity_behavior::BehaviorStyle,
    state_flags_for_planner: RetailRuntimeValue<u32>,
    owner: Type17ImpactEntityRef,
}

/// Re-plan and synchronously publish one bounded type-17 impact reselection.
///
/// All live storage needed by either shared acquiring initializer, including
/// the wrapper's exact same-tick hit stamp, is checked before the planner can
/// consume selector RNG. Capture People, Follow Beacons, and Run Away each
/// consume two additional constructor words on full variant-zero publication.
pub fn apply_type17_impact_reselection_live(
    entity: &mut Entity,
    request: Type17ImpactLiveRequest<'_>,
    world_fx: &mut WorldFx,
) -> Result<Type17ImpactLiveOutcome, Type17ImpactLiveError> {
    apply_type17_impact_reselection_live_with_prepare(
        entity,
        request,
        world_fx,
        |preparation, owner_position_raw, metadata, constructor_filter_override_raw| {
            prepare_shared_acquiring_runtime_task(
                preparation,
                owner_position_raw,
                metadata,
                constructor_filter_override_raw,
            )
        },
    )
}

fn apply_type17_impact_reselection_live_with_prepare<E>(
    entity: &mut Entity,
    request: Type17ImpactLiveRequest<'_>,
    world_fx: &mut WorldFx,
    mut prepare: impl FnMut(
        RunAwayTaskPreparation,
        [i16; 3],
        &EntityTypeRuntimeMetadata,
        u32,
    ) -> Result<PreparedSharedGenericRuntimeTask, E>,
) -> Result<Type17ImpactLiveOutcome, Type17ImpactLiveError> {
    let preflight = preflight_type17_impact_live(entity, request.metadata, request.current_tick)?;
    let planned = plan_type17_impact_reselection(
        Type17ImpactReselectionRequest {
            active_model_id: TYPE17_IMPACT_MODEL_ID,
            current_style: RetailRuntimeValue::Known(Some(preflight.current_style)),
            state_flags_raw: preflight.state_flags_for_planner,
            current_tick: request.current_tick,
            metadata: request.metadata,
            owner: preflight.owner,
            candidates_in_intrusive_order: request.candidates_in_intrusive_order,
        },
        || u32::from(world_fx.next_shared_retail_random_u16()),
    )
    .map_err(Type17ImpactLiveError::Plan)?;

    let weighted = match planned {
        Type17ImpactReselectionOutcome::Boundary(boundary) => {
            return Ok(Type17ImpactLiveOutcome::Boundary(boundary));
        }
        Type17ImpactReselectionOutcome::Weighted(weighted) => weighted,
    };
    match weighted.install {
        Type17ImpactInstallDecision::FollowBeaconsAcquiring => {
            let selected_program = weighted.selection.program;
            debug_assert_eq!(selected_program.class_id, 33);
            let selected_context = preflight
                .current_context
                .reselect_named_type_default(
                    selected_program,
                    selected_program.initial_style_table_index_raw,
                    selected_program.initial_style,
                )
                .expect("the planner returned the statically audited class-33 program");
            let publication = publish_type17_follow_beacons_acquiring(
                entity,
                selected_context,
                request.metadata,
                world_fx,
            )
            .map_err(Type17ImpactLiveError::FollowBeacons)?;
            return Ok(Type17ImpactLiveOutcome::FollowBeaconsPublished(
                Type17FollowBeaconsPublished {
                    weighted,
                    publication,
                },
            ));
        }
        Type17ImpactInstallDecision::SharedAcquiring => {}
    }

    let selected_program = weighted.selection.program;
    debug_assert!(matches!(selected_program.class_id, 9 | 10));
    let selected_context = preflight
        .current_context
        .reselect_named_type_default(
            selected_program,
            selected_program.initial_style_table_index_raw,
            selected_program.initial_style,
        )
        .expect("the planner returned a statically audited shared acquiring program");

    match publish_type17_shared_acquiring_with_prepare(
        entity,
        selected_program,
        selected_context,
        request.metadata,
        world_fx,
        &mut prepare,
    ) {
        Type17SharedAcquiringPublication::Published {
            constructors_by_phase,
        } => Ok(Type17ImpactLiveOutcome::SharedAcquiringPublished(
            Type17SharedAcquiringPublished {
                weighted,
                constructors_by_phase,
            },
        )),
        Type17SharedAcquiringPublication::InitializerFallbackPublished {
            failure,
            constructors_by_phase,
        } => Ok(
            Type17ImpactLiveOutcome::SharedAcquiringInitializerFallbackPublished(
                Type17SharedAcquiringFallbackPublished {
                    weighted,
                    failure,
                    constructors_by_phase,
                },
            ),
        ),
    }
}

/// Publish one already-authenticated class-9/class-10 selection through the
/// shared B6C0 initializer.  Construction and impact reselection both reach
/// this exact routine; only the context words they seed before entry differ.
pub(crate) fn publish_type17_shared_acquiring(
    entity: &mut Entity,
    selected_program: &'static crate::entity_behavior::BehaviorProgram,
    selected_context: BehaviorContextRuntime,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
) -> Type17SharedAcquiringPublication {
    publish_type17_shared_acquiring_with_prepare(
        entity,
        selected_program,
        selected_context,
        metadata,
        world_fx,
        &mut |preparation, owner_position_raw, metadata, constructor_filter_override_raw| {
            Ok::<_, std::convert::Infallible>(
                prepare_shared_acquiring_runtime_task(
                    preparation,
                    owner_position_raw,
                    metadata,
                    constructor_filter_override_raw,
                )
                .expect("authenticated type-17 B6C0 metadata must prepare both task phases"),
            )
        },
    )
}

fn publish_type17_shared_acquiring_with_prepare<E>(
    entity: &mut Entity,
    selected_program: &'static crate::entity_behavior::BehaviorProgram,
    selected_context: BehaviorContextRuntime,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
    prepare: &mut impl FnMut(
        RunAwayTaskPreparation,
        [i16; 3],
        &EntityTypeRuntimeMetadata,
        u32,
    ) -> Result<PreparedSharedGenericRuntimeTask, E>,
) -> Type17SharedAcquiringPublication {
    debug_assert!(matches!(selected_program.class_id, 9 | 10));
    let RetailRuntimeValue::Known(constructor_filter_override_raw) =
        selected_program.initializer_argument_raw
    else {
        unreachable!("both statically audited B6C0 programs retain initial style +0x44")
    };

    // FUN_0040ABB0/FUN_0040C6B0 publish the selected descriptor/style before
    // entering its fallible initializer. Both B6C0 programs have a no-op
    // initial-state policy, but use the shared translation so this order
    // remains explicit.
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(selected_context));
    let selected_policy = initial_behavior_state_policy(selected_program);
    entity.collision.state_flags_at_0x08.overwrite(
        selected_policy.set_bits | selected_policy.clear_bits,
        selected_policy.set_bits,
    );

    // B6C0 copies only the type-authored common-axis `+0x04` word before it
    // clears Tertiary. Preserve the live strict-axis word authenticated by
    // the caller and commit this reset independently of later task allocation.
    let RetailRuntimeValue::Known(mut actor_common_axis_descriptor) =
        entity.actor_common_axis_descriptor
    else {
        unreachable!("preflight retained exact actor-local common-axis storage")
    };
    actor_common_axis_descriptor.raw_word_at_0x04 = metadata
        .initializer
        .as_ref()
        .expect("preflight authenticated the type initializer")
        .common_axis_descriptor
        .raw_word_at_0x04;
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(actor_common_axis_descriptor);

    let owner_position_raw = entity.position_raw();
    let mut constructors_by_phase = [None; 2];
    let setup_result: Result<(), RunAwayTaskSetupError<E>> = {
        let Entity {
            actor_tasks,
            sub_a_propulsion_runtime,
            sub_h_external_frame_runtime,
            ..
        } = entity;
        let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
            unreachable!("preflight retained exact live Sub-A storage")
        };
        let RetailRuntimeValue::Known(Some(sub_h)) = sub_h_external_frame_runtime else {
            unreachable!("preflight retained exact live Sub-H storage")
        };

        apply_run_away_task_setup(
            actor_tasks,
            RunAwayTaskSetupRequest::Acquiring,
            |preparation| {
                let phase_index = preparation.phase_index;
                let prepared = prepare(
                    preparation,
                    owner_position_raw,
                    metadata,
                    constructor_filter_override_raw,
                )?;
                Ok(prepared.apply_suffix(
                    || u32::from(world_fx.next_shared_retail_random_u16()),
                    |effect| {
                        apply_shared_acquiring_constructor_effect(
                            effect,
                            sub_h,
                            sub_a,
                            &mut constructors_by_phase[phase_index],
                        )
                    },
                ))
            },
        )
    };

    match setup_result {
        Ok(()) => Type17SharedAcquiringPublication::Published {
            constructors_by_phase: constructors_by_phase.map(|evidence| {
                evidence.expect("successful acquiring setup applied both constructor suffixes")
            }),
        },
        Err(error) => {
            let failure = Type17SharedAcquiringInitializerFailure {
                phase_index: error.phase_index,
                slot: error.slot,
                role: error.role,
            };
            entity.publish_behavior_initializer_failure_fallback(selected_context);
            Type17SharedAcquiringPublication::InitializerFallbackPublished {
                failure,
                constructors_by_phase,
            }
        }
    }
}

fn apply_shared_acquiring_constructor_effect(
    effect: SharedGenericConstructorEffect,
    sub_h: &mut SubHRuntimeState,
    sub_a: &mut SubAPropulsionRuntime,
    constructor: &mut Option<Type17SharedAcquiringConstructorEvidence>,
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
            *constructor = Some(Type17SharedAcquiringConstructorEvidence {
                random_sample_low16,
                sub_a_target_speed_raw: target_speed_raw,
            });
        }
    }
}

fn preflight_type17_impact_live(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    current_tick: u32,
) -> Result<Type17ImpactLivePreflight, Type17ImpactLiveError> {
    if !entity.active {
        return Err(Type17ImpactLiveError::EntityInactive);
    }
    if entity.entity_type != TYPE17_IMPACT_ENTITY_TYPE {
        return Err(Type17ImpactLiveError::UnexpectedEntityType {
            actual: entity.entity_type,
        });
    }
    let expected_model = Some(usize::from(TYPE17_IMPACT_MODEL_ID));
    if entity.model_slots != [expected_model; 4] {
        return Err(Type17ImpactLiveError::UnexpectedEntityModelSlots {
            actual: entity.model_slots,
        });
    }

    let current_context = match entity.current_behavior_context {
        RetailRuntimeValue::Known(Some(context)) => context,
        RetailRuntimeValue::Known(None) => {
            return Err(Type17ImpactLiveError::CurrentBehaviorContextAbsent);
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17ImpactLiveError::CurrentBehaviorContextUnresolved);
        }
    };
    let descriptor_class = match current_context.descriptor() {
        BehaviorDescriptorIdentity::Named(program) => program.class_id,
        BehaviorDescriptorIdentity::InitializerFailureFallback => {
            return Err(Type17ImpactLiveError::CurrentBehaviorContextIsInitializerFallback);
        }
    };
    if !matches!(descriptor_class, 9 | 10 | 33) {
        return Err(
            Type17ImpactLiveError::CurrentBehaviorClassNotAuthoredForType17 {
                actual: descriptor_class,
            },
        );
    }
    if current_context.choice_list_source()
        != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
    {
        return Err(Type17ImpactLiveError::CurrentBehaviorChoiceSourceUnresolved);
    }
    let current_style = match current_context.active_style() {
        ActiveBehaviorStyle::Audited(style) => style,
        ActiveBehaviorStyle::InitializerFailureFallback => {
            return Err(Type17ImpactLiveError::CurrentBehaviorStyleIsInitializerFallback);
        }
    };
    if current_style.class_id != descriptor_class {
        return Err(
            Type17ImpactLiveError::CurrentBehaviorDescriptorStyleMismatch {
                descriptor_class,
                style_class: current_style.class_id,
            },
        );
    }

    let state_flags_for_planner = entity
        .collision
        .state_flags_at_0x08
        .masked(TYPE17_DYING_STATE_BIT);
    if state_flags_for_planner == RetailRuntimeValue::Unresolved {
        return Err(Type17ImpactLiveError::DyingStateBitUnresolved);
    }
    match entity.collision.last_hit_presentation_tick_at_0x34 {
        RetailRuntimeValue::Known(actual) if actual == current_tick => {}
        RetailRuntimeValue::Known(actual) => {
            return Err(Type17ImpactLiveError::LastHitTickMismatch {
                expected: current_tick,
                actual,
            });
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17ImpactLiveError::LastHitTickUnresolved);
        }
    }
    let attached_entity_handle = match entity.collision.recent_relation_id_at_0x60 {
        RetailRuntimeValue::Known(handle) => RetailRuntimeValue::Known(handle),
        RetailRuntimeValue::Unresolved => {
            return Err(Type17ImpactLiveError::OwnerAttachedEntityUnresolved);
        }
    };

    let topology = match metadata.common_mover_topology {
        RetailRuntimeValue::Known(topology) => topology,
        RetailRuntimeValue::Unresolved => {
            return Err(Type17ImpactLiveError::ComponentTopologyUnresolved);
        }
    };
    if topology != TYPE17_MODEL256_COMPONENT_TOPOLOGY {
        return Err(Type17ImpactLiveError::UnexpectedComponentTopology { actual: topology });
    }
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(Type17ImpactLiveError::MissingInitializer)?;
    if initializer.common_axis_descriptor != TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR {
        return Err(Type17ImpactLiveError::UnexpectedCommonAxisDescriptor {
            actual: initializer.common_axis_descriptor,
        });
    }
    match entity.actor_common_axis_descriptor {
        RetailRuntimeValue::Known(actual) if is_type17_reselection_axis_descriptor(actual) => {}
        RetailRuntimeValue::Known(actual) => {
            return Err(Type17ImpactLiveError::UnexpectedActorCommonAxisDescriptor { actual });
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17ImpactLiveError::ActorCommonAxisDescriptorUnresolved);
        }
    }
    let sub_a_descriptor = match metadata.sub_a_propulsion_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
        RetailRuntimeValue::Known(None) => {
            return Err(Type17ImpactLiveError::SubADescriptorAbsent);
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17ImpactLiveError::SubADescriptorUnresolved);
        }
    };
    if sub_a_descriptor.target_speed_base_raw != TYPE17_MODEL256_SUB_A_TARGET_SPEED_BASE_RAW {
        return Err(Type17ImpactLiveError::UnexpectedSubATargetSpeedBase {
            actual: sub_a_descriptor.target_speed_base_raw,
        });
    }
    let sub_h_descriptor = match &metadata.sub_h_external_frame_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
        RetailRuntimeValue::Known(None) => {
            return Err(Type17ImpactLiveError::SubHDescriptorAbsent);
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17ImpactLiveError::SubHDescriptorUnresolved);
        }
    };
    if sub_h_descriptor.records.len() != TYPE17_MODEL256_SUB_H_RECORD_COUNT {
        return Err(Type17ImpactLiveError::UnexpectedSubHDescriptorRecordCount {
            actual: sub_h_descriptor.records.len(),
        });
    }

    match entity.sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(_)) => {}
        RetailRuntimeValue::Known(None) => {
            return Err(Type17ImpactLiveError::SubARuntimeAbsent);
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17ImpactLiveError::SubARuntimeUnresolved);
        }
    }
    let sub_h_runtime = match &entity.sub_h_external_frame_runtime {
        RetailRuntimeValue::Known(Some(runtime)) => runtime,
        RetailRuntimeValue::Known(None) => {
            return Err(Type17ImpactLiveError::SubHRuntimeAbsent);
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17ImpactLiveError::SubHRuntimeUnresolved);
        }
    };
    if sub_h_runtime.records().len() != TYPE17_MODEL256_SUB_H_RECORD_COUNT {
        return Err(Type17ImpactLiveError::UnexpectedSubHRuntimeRecordCount {
            actual: sub_h_runtime.records().len(),
        });
    }

    Ok(Type17ImpactLivePreflight {
        current_context,
        current_style,
        state_flags_for_planner,
        owner: Type17ImpactEntityRef {
            id: entity.id,
            entity_type: entity.entity_type,
            position_raw: entity.position_raw(),
            // Candidate selection skips the owner identity before reading this
            // word. Retain its real partial evidence instead of manufacturing
            // a complete candidate dword.
            state_flags_raw: entity.collision.state_flags_at_0x08,
            capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
            attached_entity_handle,
        },
    })
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use super::*;
    use crate::{
        actor_task_dispatcher::{ActorTaskRuntime, ActorTaskRuntimeFamily},
        actor_task_owner::{ActorTaskId, ActorTaskOwner, PreparedActorTask},
        entity::EntityKind,
        entity_behavior::{audited_behavior_style, behavior_program},
        entity_collision_state::{EntityInitializerSpec, RetailStateWord},
        shared_retarget_mover::SharedRetargetTaskState,
        type17_impact_reselection::{
            TYPE17_ALTERNATE_COMMON_DYING_CLASS_ID, TYPE17_BEHAVIOR_RULE_REF,
            TYPE17_IMPACT_BEHAVIOR_CHOICES,
        },
    };
    use v2k_formats::collision::{
        SubAPropulsionDescriptor, SubHExternalFrameDescriptor, SubHExternalFrameRecord,
    };

    const ENTITY_ID: u32 = 0x0497_0001;
    const TARGET_HANDLE: u32 = 0x047f_0001;
    const AUXILIARY_WORD: u32 = 0x1357_9bdf;
    const BASE_STATE: u32 = 0xa506_8001;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct EntityMutationSnapshot {
        context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
        state: RetailStateWord,
        last_hit_tick: RetailRuntimeValue<u32>,
        actor_common_axis_descriptor: RetailRuntimeValue<CommonAxisDescriptor>,
        sub_a: RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
        sub_h: RetailRuntimeValue<Option<SubHRuntimeState>>,
        task_ids: [Option<ActorTaskId>; 3],
    }

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [TYPE17_IMPACT_MODEL_ID; 4],
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: 57,
                common_axis_descriptor: TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR,
                behavior_choices: TYPE17_IMPACT_BEHAVIOR_CHOICES.to_vec().into_boxed_slice(),
                behavior_rule_ref: TYPE17_BEHAVIOR_RULE_REF,
                alternate_behavior_class_ref: TYPE17_ALTERNATE_COMMON_DYING_CLASS_ID,
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

    fn follow_beacons_impact_context() -> BehaviorContextRuntime {
        let program = behavior_program(33).expect("Follow Beacons program");
        let style = *audited_behavior_style(33, 1).expect("Follow Beacons impact style");
        BehaviorContextRuntime::named_audited(
            program,
            1,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(Some(TARGET_HANDLE)),
            RetailRuntimeValue::Known(AUXILIARY_WORD),
            style,
        )
        .expect("canonical Follow Beacons live context")
    }

    fn seeded_task(lifetime_ms: u32) -> PreparedActorTask<ActorTaskRuntime> {
        PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
            SharedRetargetTaskState::new([1, 2, 3], lifetime_ms),
        ))
    }

    fn exact_entity(current_tick: u32) -> Entity {
        let mut entity = Entity::unresolved_port_entity(ENTITY_ID, EntityKind::Unknown(17), 17);
        entity.model_slots = [Some(usize::from(TYPE17_IMPACT_MODEL_ID)); 4];
        entity.model_index = Some(usize::from(TYPE17_IMPACT_MODEL_ID));
        entity.current_behavior_context =
            RetailRuntimeValue::Known(Some(follow_beacons_impact_context()));
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(BASE_STATE);
        entity.collision.last_hit_presentation_tick_at_0x34 =
            RetailRuntimeValue::Known(current_tick);
        entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
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

    fn request<'a>(
        metadata: &'a EntityTypeRuntimeMetadata,
        current_tick: u32,
    ) -> Type17ImpactLiveRequest<'a> {
        request_with_candidates(metadata, current_tick, &[])
    }

    fn request_with_candidates<'a>(
        metadata: &'a EntityTypeRuntimeMetadata,
        current_tick: u32,
        candidates_in_intrusive_order: &'a [Type17ImpactEntityRef],
    ) -> Type17ImpactLiveRequest<'a> {
        Type17ImpactLiveRequest {
            metadata,
            candidates_in_intrusive_order,
            current_tick,
        }
    }

    fn task_ids(entity: &Entity) -> [Option<ActorTaskId>; 3] {
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot))
    }

    fn snapshot(entity: &Entity) -> EntityMutationSnapshot {
        EntityMutationSnapshot {
            context: entity.current_behavior_context,
            state: entity.collision.state_flags_at_0x08,
            last_hit_tick: entity.collision.last_hit_presentation_tick_at_0x34,
            actor_common_axis_descriptor: entity.actor_common_axis_descriptor,
            sub_a: entity.sub_a_propulsion_runtime,
            sub_h: entity.sub_h_external_frame_runtime.clone(),
            task_ids: task_ids(entity),
        }
    }

    #[test]
    fn selected_follow_beacons_publishes_variant_zero_and_both_suffixes() {
        let metadata = exact_metadata();
        let mut entity = exact_entity(0);
        let mut world_fx = WorldFx::new();

        let outcome =
            apply_type17_impact_reselection_live(&mut entity, request(&metadata, 0), &mut world_fx)
                .unwrap();
        let Type17ImpactLiveOutcome::FollowBeaconsPublished(published) = outcome else {
            panic!("expected Follow Beacons publication: {outcome:?}")
        };
        assert_eq!(published.weighted.selection.program.class_id, 33);
        assert_eq!(published.weighted.random_word, 0x0026);
        let Type17FollowBeaconsAcquiringPublication::Published {
            constructors_by_phase,
            ..
        } = published.publication
        else {
            panic!("variant-zero setup unexpectedly failed")
        };
        assert_eq!(constructors_by_phase[0].random_sample_low16, 0x1e27);
        assert_eq!(constructors_by_phase[1].random_sample_low16, 0xd2f6);
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(_))
        ));
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::FollowBeaconAcquisition(_))
        ));
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
        assert_eq!(world_fx.next_shared_retail_random_u16(), 0x0985);
    }

    #[test]
    fn post_capture_axis_can_reselect_follow_without_a_post_rng_rejection() {
        let metadata = exact_metadata();
        let mut entity = exact_entity(0);
        entity.actor_common_axis_descriptor =
            RetailRuntimeValue::Known(TYPE17_CAPTURE_PEOPLE_LIVE_AXIS_DESCRIPTOR);
        let mut world_fx = WorldFx::new();

        let outcome =
            apply_type17_impact_reselection_live(&mut entity, request(&metadata, 0), &mut world_fx)
                .unwrap();
        let Type17ImpactLiveOutcome::FollowBeaconsPublished(published) = outcome else {
            panic!("expected Capture-to-Follow publication: {outcome:?}")
        };
        assert_eq!(published.weighted.selection.program.class_id, 33);
        assert_eq!(published.weighted.random_word, 0x0026);
        assert!(matches!(
            published.publication,
            Type17FollowBeaconsAcquiringPublication::Published { .. }
        ));
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(TYPE17_CAPTURE_PEOPLE_LIVE_AXIS_DESCRIPTOR)
        );
        assert_eq!(world_fx.next_shared_retail_random_u16(), 0x0985);
    }

    #[test]
    fn dying_boundary_consumes_no_rng_and_preserves_entity() {
        let metadata = exact_metadata();
        let mut entity = exact_entity(250);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(TYPE17_DYING_STATE_BIT, TYPE17_DYING_STATE_BIT);
        let before = snapshot(&entity);
        let mut world_fx = WorldFx::new();

        let outcome = apply_type17_impact_reselection_live(
            &mut entity,
            request(&metadata, 250),
            &mut world_fx,
        )
        .unwrap();
        assert!(matches!(
            outcome,
            Type17ImpactLiveOutcome::Boundary(
                Type17ImpactReselectionBoundary::AlternateCommonDying { .. }
            )
        ));
        assert_eq!(snapshot(&entity), before);
        assert_eq!(world_fx.next_shared_retail_random_u16(), 0x0026);
    }

    #[test]
    fn unresolved_or_mismatched_live_hit_tick_fails_before_rng_or_mutation() {
        for (live_tick, expected) in [
            (
                RetailRuntimeValue::Unresolved,
                Type17ImpactLiveError::LastHitTickUnresolved,
            ),
            (
                RetailRuntimeValue::Known(249),
                Type17ImpactLiveError::LastHitTickMismatch {
                    expected: 250,
                    actual: 249,
                },
            ),
        ] {
            let metadata = exact_metadata();
            let mut entity = exact_entity(250);
            entity.collision.last_hit_presentation_tick_at_0x34 = live_tick;
            let before = snapshot(&entity);
            let mut world_fx = WorldFx::new();

            assert_eq!(
                apply_type17_impact_reselection_live(
                    &mut entity,
                    request(&metadata, 250),
                    &mut world_fx,
                ),
                Err(expected)
            );
            assert_eq!(snapshot(&entity), before);
            assert_eq!(world_fx.next_shared_retail_random_u16(), 0x0026);
        }
    }

    #[test]
    fn selected_run_away_publishes_context_tasks_and_both_shared_suffixes() {
        let metadata = exact_metadata();
        let mut entity = exact_entity(250);
        // A prior Follow Beacons callback committed its actor-local capability
        // policy; B6C0 must restore the type-authored word before task clears.
        entity.actor_common_axis_descriptor =
            RetailRuntimeValue::Known(TYPE17_FOLLOW_BEACONS_LIVE_AXIS_DESCRIPTOR);
        let mut world_fx = WorldFx::new();
        assert_eq!(world_fx.next_shared_retail_random_u16(), 0x0026);

        let outcome = apply_type17_impact_reselection_live(
            &mut entity,
            request(&metadata, 250),
            &mut world_fx,
        )
        .unwrap();
        let Type17ImpactLiveOutcome::SharedAcquiringPublished(published) = outcome else {
            panic!("expected selected Run Away publication: {outcome:?}")
        };
        assert_eq!(published.weighted.selection.program.class_id, 10);
        assert_eq!(published.weighted.random_word, 0x1e27);
        assert_eq!(
            published.constructors_by_phase,
            [
                Type17SharedAcquiringConstructorEvidence {
                    random_sample_low16: 0xd2f6,
                    sub_a_target_speed_raw: 270,
                },
                Type17SharedAcquiringConstructorEvidence {
                    random_sample_low16: 0x0985,
                    sub_a_target_speed_raw: 250,
                },
            ]
        );

        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("selected context was not published")
        };
        assert!(matches!(
            context.descriptor(),
            BehaviorDescriptorIdentity::Named(program) if program.class_id == 10
        ));
        assert_eq!(context.style_table_index_raw_at_0x10(), 0);
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(TARGET_HANDLE))
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(AUXILIARY_WORD)
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.known_value_bits(),
            BASE_STATE
        );
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR)
        );

        let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime else {
            panic!("live Sub-H disappeared")
        };
        assert!(sub_h.is_enabled());
        assert_eq!(sub_h.cursor(), 0);
        assert_eq!(sub_h.records().len(), TYPE17_MODEL256_SUB_H_RECORD_COUNT);
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("live Sub-A disappeared")
        };
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(250));
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(sub_a.drive_scale_percent(), 73);

        assert!(entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary)
            .is_none());
        assert_eq!(
            entity
                .actor_task_state(ActorTaskSlot::Secondary)
                .map(ActorTaskRuntime::family),
            Some(ActorTaskRuntimeFamily::TargetAcquisition)
        );
        assert_eq!(
            entity
                .actor_task_state(ActorTaskSlot::Primary)
                .map(ActorTaskRuntime::family),
            Some(ActorTaskRuntimeFamily::SharedRetarget)
        );
        let Some(ActorTaskRuntime::SharedRetarget(primary)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            unreachable!()
        };
        assert_eq!(primary.lifetime_ms(), 500);
        assert_eq!(world_fx.next_shared_retail_random_u16(), 0xa297);
    }

    #[test]
    fn selected_capture_people_publishes_the_shared_b6c0_program() {
        let metadata = exact_metadata();
        let mut entity = exact_entity(249);
        entity.actor_common_axis_descriptor =
            RetailRuntimeValue::Known(TYPE17_CAPTURE_PEOPLE_LIVE_AXIS_DESCRIPTOR);
        let candidates = [Type17ImpactEntityRef {
            id: 0x0400_0002,
            entity_type: 99,
            position_raw: entity.position_raw(),
            state_flags_raw: RetailStateWord::exact(1),
            capability_flags: RetailRuntimeValue::Known(0x0400),
            attached_entity_handle: RetailRuntimeValue::Known(None),
        }];
        let mut world_fx = WorldFx::new();

        let outcome = apply_type17_impact_reselection_live(
            &mut entity,
            request_with_candidates(&metadata, 249, &candidates),
            &mut world_fx,
        )
        .unwrap();
        let Type17ImpactLiveOutcome::SharedAcquiringPublished(published) = outcome else {
            panic!("expected selected Capture People publication: {outcome:?}")
        };
        assert_eq!(published.weighted.selection.program.class_id, 9);
        assert_eq!(published.weighted.selection.choice_index, 0);
        assert_eq!(published.weighted.random_word, 0x0026);
        assert_eq!(
            published.constructors_by_phase,
            [
                Type17SharedAcquiringConstructorEvidence {
                    random_sample_low16: 0x1e27,
                    sub_a_target_speed_raw: 252,
                },
                Type17SharedAcquiringConstructorEvidence {
                    random_sample_low16: 0xd2f6,
                    sub_a_target_speed_raw: 270,
                },
            ]
        );

        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("selected context was not published")
        };
        assert!(matches!(
            context.descriptor(),
            BehaviorDescriptorIdentity::Named(program) if program.class_id == 9
        ));
        assert_eq!(context.style_table_index_raw_at_0x10(), 0);
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(TARGET_HANDLE))
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(AUXILIARY_WORD)
        );
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR)
        );
        let Some(ActorTaskRuntime::TargetAcquisition(acquisition)) =
            entity.actor_task_state(ActorTaskSlot::Secondary)
        else {
            panic!("Capture People phase zero changed task family")
        };
        assert_eq!(acquisition.radius().raw(), 2_560);
        assert_eq!(acquisition.filter().raw(), 3);
        assert_eq!(acquisition.constructor_filter_override_raw(), 0x0C00);
        assert_eq!(
            entity
                .actor_task_state(ActorTaskSlot::Primary)
                .map(ActorTaskRuntime::family),
            Some(ActorTaskRuntimeFamily::SharedRetarget)
        );
        assert!(entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary)
            .is_none());
        assert_eq!(world_fx.next_shared_retail_random_u16(), 0x0985);
    }

    #[test]
    fn missing_or_malformed_live_components_fail_before_rng_or_mutation() {
        #[derive(Debug, Clone, Copy)]
        enum BrokenRuntime {
            ActorAxisUnresolved,
            ActorAxisWrong,
            SubAUnresolved,
            SubAAbsent,
            SubHUnresolved,
            SubHAbsent,
            SubHWrongCount,
        }

        for (broken, expected) in [
            (
                BrokenRuntime::ActorAxisUnresolved,
                Type17ImpactLiveError::ActorCommonAxisDescriptorUnresolved,
            ),
            (
                BrokenRuntime::ActorAxisWrong,
                Type17ImpactLiveError::UnexpectedActorCommonAxisDescriptor {
                    actual: CommonAxisDescriptor {
                        strict_axis_limit_raw: 0x0a00,
                        raw_word_at_0x04: 7,
                    },
                },
            ),
            (
                BrokenRuntime::SubAUnresolved,
                Type17ImpactLiveError::SubARuntimeUnresolved,
            ),
            (
                BrokenRuntime::SubAAbsent,
                Type17ImpactLiveError::SubARuntimeAbsent,
            ),
            (
                BrokenRuntime::SubHUnresolved,
                Type17ImpactLiveError::SubHRuntimeUnresolved,
            ),
            (
                BrokenRuntime::SubHAbsent,
                Type17ImpactLiveError::SubHRuntimeAbsent,
            ),
            (
                BrokenRuntime::SubHWrongCount,
                Type17ImpactLiveError::UnexpectedSubHRuntimeRecordCount { actual: 7 },
            ),
        ] {
            let metadata = exact_metadata();
            let mut entity = exact_entity(250);
            match broken {
                BrokenRuntime::ActorAxisUnresolved => {
                    entity.actor_common_axis_descriptor = RetailRuntimeValue::Unresolved;
                }
                BrokenRuntime::ActorAxisWrong => {
                    entity.actor_common_axis_descriptor =
                        RetailRuntimeValue::Known(CommonAxisDescriptor {
                            strict_axis_limit_raw: 0x0a00,
                            raw_word_at_0x04: 7,
                        });
                }
                BrokenRuntime::SubAUnresolved => {
                    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Unresolved;
                }
                BrokenRuntime::SubAAbsent => {
                    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(None);
                }
                BrokenRuntime::SubHUnresolved => {
                    entity.sub_h_external_frame_runtime = RetailRuntimeValue::Unresolved;
                }
                BrokenRuntime::SubHAbsent => {
                    entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(None);
                }
                BrokenRuntime::SubHWrongCount => {
                    entity.sub_h_external_frame_runtime =
                        RetailRuntimeValue::Known(Some(SubHRuntimeState::new(7).unwrap()));
                }
            }
            let before = snapshot(&entity);
            let mut world_fx = WorldFx::new();
            assert_eq!(
                apply_type17_impact_reselection_live(
                    &mut entity,
                    request(&metadata, 250),
                    &mut world_fx,
                ),
                Err(expected),
                "broken runtime {broken:?}"
            );
            assert_eq!(snapshot(&entity), before, "broken runtime {broken:?}");
            assert_eq!(
                world_fx.next_shared_retail_random_u16(),
                0x0026,
                "broken runtime {broken:?}"
            );
        }
    }

    #[test]
    fn either_allocation_failure_publishes_fallback_without_rolling_back_prior_suffixes() {
        for failed_phase in [0, 1] {
            let metadata = exact_metadata();
            let mut entity = exact_entity(250);
            let mut world_fx = WorldFx::new();
            assert_eq!(world_fx.next_shared_retail_random_u16(), 0x0026);

            let outcome = apply_type17_impact_reselection_live_with_prepare(
                &mut entity,
                request(&metadata, 250),
                &mut world_fx,
                |preparation, owner_position_raw, metadata, constructor_filter_override_raw| {
                    if preparation.phase_index == failed_phase {
                        return Err(preparation.phase_index);
                    }
                    prepare_shared_acquiring_runtime_task(
                        preparation,
                        owner_position_raw,
                        metadata,
                        constructor_filter_override_raw,
                    )
                    .map_err(|_| usize::MAX)
                },
            )
            .unwrap();
            let Type17ImpactLiveOutcome::SharedAcquiringInitializerFallbackPublished(published) =
                outcome
            else {
                panic!("expected terminal initializer fallback: {outcome:?}")
            };
            assert_eq!(published.weighted.selection.program.class_id, 10);
            assert_eq!(published.weighted.random_word, 0x1e27);
            assert_eq!(published.failure.phase_index, failed_phase);
            assert_eq!(
                published.constructors_by_phase,
                if failed_phase == 0 {
                    [None, None]
                } else {
                    [
                        Some(Type17SharedAcquiringConstructorEvidence {
                            random_sample_low16: 0xd2f6,
                            sub_a_target_speed_raw: 270,
                        }),
                        None,
                    ]
                }
            );
            assert_eq!(
                published.failure.slot,
                if failed_phase == 0 {
                    ActorTaskSlot::Secondary
                } else {
                    ActorTaskSlot::Primary
                }
            );
            assert_eq!(
                published.failure.role,
                if failed_phase == 0 {
                    RunAwayTaskRole::AcquireTarget
                } else {
                    RunAwayTaskRole::Wander
                }
            );

            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                panic!("fallback context was not published")
            };
            assert_eq!(
                context.descriptor(),
                BehaviorDescriptorIdentity::InitializerFailureFallback
            );
            assert_eq!(
                context.active_style(),
                ActiveBehaviorStyle::InitializerFailureFallback
            );
            assert_eq!(
                context.target_handle_at_0x08(),
                RetailRuntimeValue::Known(Some(TARGET_HANDLE))
            );
            assert_eq!(
                context.auxiliary_word_at_0x0c(),
                RetailRuntimeValue::Known(AUXILIARY_WORD)
            );
            assert_eq!(
                entity.collision.state_flags_at_0x08.known_value_bits(),
                BASE_STATE & !0x0006_8000
            );
            assert_eq!(task_ids(&entity), [None; 3]);

            let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime
            else {
                panic!("fallback lost Sub-H storage")
            };
            assert_eq!(sub_h.is_enabled(), failed_phase == 1);
            let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
                panic!("fallback lost Sub-A storage")
            };
            assert_eq!(
                sub_a.target_speed_raw(),
                if failed_phase == 0 {
                    RetailRuntimeValue::Unresolved
                } else {
                    RetailRuntimeValue::Known(270)
                }
            );
            assert_eq!(
                sub_a.direction_multiplier(),
                if failed_phase == 0 { -1 } else { 1 }
            );
            assert_eq!(sub_a.drive_scale_percent(), 73);
            assert_eq!(
                world_fx.next_shared_retail_random_u16(),
                if failed_phase == 0 { 0xd2f6 } else { 0x0985 }
            );
        }
    }

    #[test]
    fn capture_people_failures_keep_the_preclear_filter_reset_and_exact_rng_prefix() {
        for failed_phase in [0, 1] {
            let metadata = exact_metadata();
            let mut entity = exact_entity(249);
            entity.actor_common_axis_descriptor =
                RetailRuntimeValue::Known(TYPE17_FOLLOW_BEACONS_LIVE_AXIS_DESCRIPTOR);
            let candidates = [Type17ImpactEntityRef {
                id: 0x0400_0002,
                entity_type: 99,
                position_raw: entity.position_raw(),
                state_flags_raw: RetailStateWord::exact(1),
                capability_flags: RetailRuntimeValue::Known(0x0400),
                attached_entity_handle: RetailRuntimeValue::Known(None),
            }];
            let mut world_fx = WorldFx::new();
            let mut observed_overrides = Vec::new();

            let outcome = apply_type17_impact_reselection_live_with_prepare(
                &mut entity,
                request_with_candidates(&metadata, 249, &candidates),
                &mut world_fx,
                |preparation, owner_position_raw, metadata, constructor_filter_override_raw| {
                    observed_overrides.push(constructor_filter_override_raw);
                    if preparation.phase_index == failed_phase {
                        return Err(preparation.phase_index);
                    }
                    prepare_shared_acquiring_runtime_task(
                        preparation,
                        owner_position_raw,
                        metadata,
                        constructor_filter_override_raw,
                    )
                    .map_err(|_| usize::MAX)
                },
            )
            .unwrap();
            let Type17ImpactLiveOutcome::SharedAcquiringInitializerFallbackPublished(published) =
                outcome
            else {
                panic!("expected Capture People initializer fallback: {outcome:?}")
            };

            assert_eq!(published.weighted.selection.program.class_id, 9);
            assert_eq!(published.weighted.random_word, 0x0026);
            assert_eq!(published.failure.phase_index, failed_phase);
            assert_eq!(observed_overrides, vec![0x0C00; failed_phase + 1]);
            assert_eq!(
                published.constructors_by_phase,
                if failed_phase == 0 {
                    [None, None]
                } else {
                    [
                        Some(Type17SharedAcquiringConstructorEvidence {
                            random_sample_low16: 0x1e27,
                            sub_a_target_speed_raw: 252,
                        }),
                        None,
                    ]
                }
            );
            assert_eq!(
                entity.actor_common_axis_descriptor,
                RetailRuntimeValue::Known(TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR)
            );
            assert_eq!(task_ids(&entity), [None; 3]);
            assert_eq!(
                entity.collision.state_flags_at_0x08.known_value_bits(),
                BASE_STATE & !0x0006_8000
            );
            assert_eq!(
                world_fx.next_shared_retail_random_u16(),
                if failed_phase == 0 { 0x1e27 } else { 0xd2f6 }
            );
        }
    }

    #[test]
    fn initializer_fallback_retires_tasks_in_retail_order() {
        #[derive(Debug)]
        struct DropLoggedTask {
            slot: ActorTaskSlot,
            drops: Rc<RefCell<Vec<ActorTaskSlot>>>,
        }

        impl Drop for DropLoggedTask {
            fn drop(&mut self) {
                self.drops.borrow_mut().push(self.slot);
            }
        }

        let drops = Rc::new(RefCell::new(Vec::new()));
        let mut owner = ActorTaskOwner::new();
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            owner.replace_prepared(
                slot,
                PreparedActorTask::new(DropLoggedTask {
                    slot,
                    drops: Rc::clone(&drops),
                }),
            );
        }

        owner.clear_behavior_initializer_failure_slots();

        assert_eq!(
            drops.borrow().as_slice(),
            [
                ActorTaskSlot::Secondary,
                ActorTaskSlot::Tertiary,
                ActorTaskSlot::Primary,
            ]
        );
    }
}
