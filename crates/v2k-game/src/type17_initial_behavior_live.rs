//! Fresh-Level-1 type-17 behavior selection and publication.
//!
//! Retail `FUN_004104B0` invokes the common actor initializer before it
//! appends the new allocation to the intrusive live list.  The exact chain is
//! `FUN_004381F0 -> FUN_0040AC60 -> FUN_00425680 -> FUN_00438340 ->
//! FUN_0040ABE0/FUN_0040ABB0 -> FUN_0040C6B0`.  Consequently each spider
//! evaluates only the already-published birth prefix, consumes one selector
//! word, runs its selected two-phase initializer, and only then becomes
//! visible to the following spawn. Successful publication also mints the
//! `V200002.run` pending first-query Sub-D owner for spawn indices 17--20.

use crate::{
    actor_task_owner::ActorTaskSlot,
    entity::Entity,
    entity_behavior::{BehaviorContextRuntime, BehaviorSelection},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord},
    type17_follow_beacons_live::{
        publish_type17_follow_beacons_acquiring, Type17FollowBeaconsAcquiringPublication,
        Type17FollowBeaconsLiveError,
    },
    type17_impact_live::{
        publish_type17_shared_acquiring, Type17SharedAcquiringPublication,
        TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR, TYPE17_MODEL256_COMPONENT_TOPOLOGY,
        TYPE17_MODEL256_SUB_A_TARGET_SPEED_BASE_RAW, TYPE17_MODEL256_SUB_H_RECORD_COUNT,
    },
    type17_impact_reselection::{
        plan_type17_weighted_selection, Type17ImpactEntityRef, Type17ImpactInstallDecision,
        Type17ImpactReselectionError, Type17ImpactWeightedSelection,
        Type17WeightedSelectionRequest, TYPE17_IMPACT_ENTITY_TYPE, TYPE17_IMPACT_MODEL_ID,
    },
    world_fx::WorldFx,
};

pub const FRESH_LEVEL1_TYPE17_INITIAL_STATE_RAW: u32 = 0x0746_8805;
pub const FRESH_LEVEL1_TYPE17_SPAWN_INDICES: [usize; 4] = [17, 18, 19, 20];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshType17InitializerPublication {
    SharedAcquiring(Type17SharedAcquiringPublication),
    FollowBeacons(Type17FollowBeaconsAcquiringPublication),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshType17InitialBehaviorPublication {
    pub entity_id: u32,
    pub authored_spawn_index: usize,
    pub weighted: Type17ImpactWeightedSelection,
    pub initializer: FreshType17InitializerPublication,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreshType17InitialBehaviorError {
    EntityInactive,
    MissingAuthoredSpawnIndex,
    UnsupportedAuthoredSpawnIndex { actual: usize },
    UnexpectedEntityType { actual: u32 },
    UnexpectedModelSlots { actual: [Option<usize>; 4] },
    UnexpectedActiveModel { actual: Option<usize> },
    UnexpectedInitialState { actual: RetailStateWord },
    InitialBehaviorAlreadyResolved,
    CurrentBehaviorContextAlreadyResolved,
    LastHitTickUnresolved,
    UnexpectedLastHitTick { actual: u32 },
    AttachedEntityUnresolved,
    ActorCommonAxisDescriptorUnresolved,
    UnexpectedActorCommonAxisDescriptor,
    ComponentTopologyUnresolved,
    UnexpectedComponentTopology,
    MissingInitializer,
    UnexpectedMetadataCommonAxisDescriptor,
    SubADescriptorUnresolved,
    SubADescriptorAbsent,
    UnexpectedSubATargetSpeedBase { actual: i16 },
    SubARuntimeUnresolved,
    SubARuntimeAbsent,
    SubHDescriptorUnresolved,
    SubHDescriptorAbsent,
    UnexpectedSubHDescriptorRecordCount { actual: usize },
    SubHRuntimeUnresolved,
    SubHRuntimeAbsent,
    UnexpectedSubHRuntimeRecordCount { actual: usize },
    BirthTaskTableNotEmpty,
    SelectedInitializerResolutionMismatch,
    CanonicalFreshContextUnavailable,
    Plan(Type17ImpactReselectionError),
    FollowBeacons(Type17FollowBeaconsLiveError),
}

/// Build one candidate view for the current retail list prefix.
///
/// Candidate attachment is not consumed by `FUN_00422C10`; keeping it as an
/// explicit null avoids conflating the retail null handle with port entity id
/// zero (the persistent Level-1 player).
pub(crate) fn fresh_type17_candidate_ref(entity: &Entity) -> Type17ImpactEntityRef {
    Type17ImpactEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle: RetailRuntimeValue::Known(None),
    }
}

/// Validate the complete fresh model-256 publication surface, evaluate the
/// current live prefix, and consume exactly the selector's one shared word.
/// No entity state changes before this returns successfully.
pub(crate) fn plan_fresh_type17_initial_behavior(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    candidates_in_intrusive_order: &[Type17ImpactEntityRef],
    current_tick: u32,
    world_fx: &mut WorldFx,
) -> Result<Type17ImpactWeightedSelection, FreshType17InitialBehaviorError> {
    validate_fresh_publication_surface(entity, metadata)?;

    let last_hit_tick = match entity.collision.last_hit_presentation_tick_at_0x34 {
        RetailRuntimeValue::Known(value) => value,
        RetailRuntimeValue::Unresolved => {
            return Err(FreshType17InitialBehaviorError::LastHitTickUnresolved)
        }
    };
    if last_hit_tick != 0 {
        return Err(FreshType17InitialBehaviorError::UnexpectedLastHitTick {
            actual: last_hit_tick,
        });
    }
    let attached_entity_handle = match entity.collision.recent_relation_id_at_0x60 {
        RetailRuntimeValue::Known(value) => RetailRuntimeValue::Known(value),
        RetailRuntimeValue::Unresolved => {
            return Err(FreshType17InitialBehaviorError::AttachedEntityUnresolved)
        }
    };
    let owner = Type17ImpactEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        // The selector skips the separate owner identity before it can read
        // this candidate word, but retain the real constructor evidence.
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle,
    };

    plan_type17_weighted_selection(
        Type17WeightedSelectionRequest {
            active_model_id: TYPE17_IMPACT_MODEL_ID,
            current_tick,
            last_hit_tick,
            metadata,
            owner,
            candidates_in_intrusive_order,
        },
        || u32::from(world_fx.next_shared_retail_random_u16()),
    )
    .map_err(FreshType17InitialBehaviorError::Plan)
}

/// Publish the selected variant-zero context and its fallible initializer.
///
/// The caller must first apply the selected `EntityInitializerResolution` to
/// the local, not-yet-linked allocation.  Initializer failure is a successful
/// publication of the existing unnamed fallback, not an error.
///
/// Retail may fail the heap allocation in `FUN_0040ABE0` after selection. The
/// port stores this bounded context inline, so production context construction
/// is infallible once the canonical selection has been authenticated; only the
/// selected task initializer retains its original fallible publication.
pub(crate) fn publish_fresh_type17_initial_behavior(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    weighted: Type17ImpactWeightedSelection,
    world_fx: &mut WorldFx,
) -> Result<FreshType17InitialBehaviorPublication, FreshType17InitialBehaviorError> {
    validate_selected_publication_surface(entity, metadata, weighted.selection)?;
    let selected_context =
        BehaviorContextRuntime::from_fresh_weighted_selection(weighted.selection)
            .ok_or(FreshType17InitialBehaviorError::CanonicalFreshContextUnavailable)?;

    let initializer = match weighted.install {
        Type17ImpactInstallDecision::SharedAcquiring => {
            FreshType17InitializerPublication::SharedAcquiring(publish_type17_shared_acquiring(
                entity,
                weighted.selection.program,
                selected_context,
                metadata,
                world_fx,
            ))
        }
        Type17ImpactInstallDecision::FollowBeaconsAcquiring => {
            FreshType17InitializerPublication::FollowBeacons(
                publish_type17_follow_beacons_acquiring(
                    entity,
                    selected_context,
                    metadata,
                    world_fx,
                )
                .map_err(FreshType17InitialBehaviorError::FollowBeacons)?,
            )
        }
    };

    if let Some(seed) = crate::common_mover::sub_d::type17_seed_for_fresh_level1_spawn(
        entity
            .authored_spawn_index
            .expect("fresh publication preflight retained the authored spawn"),
    ) {
        entity.type17_sub_d_frame_owner =
            crate::common_mover::sub_d::type17_first_query_owner_for_seed(seed);
        entity.type17_sub_d_runtime =
            Some(crate::common_mover::sub_d::Type9SubDRuntime::from_constructor());
    }

    Ok(FreshType17InitialBehaviorPublication {
        entity_id: entity.id,
        authored_spawn_index: entity
            .authored_spawn_index
            .expect("fresh publication preflight retained the authored spawn"),
        weighted,
        initializer,
    })
}

fn validate_fresh_publication_surface(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), FreshType17InitialBehaviorError> {
    validate_common_publication_surface(entity, metadata)?;
    if entity.initial_behavior != RetailRuntimeValue::Unresolved {
        return Err(FreshType17InitialBehaviorError::InitialBehaviorAlreadyResolved);
    }
    if entity.current_behavior_context != RetailRuntimeValue::Unresolved {
        return Err(FreshType17InitialBehaviorError::CurrentBehaviorContextAlreadyResolved);
    }
    Ok(())
}

fn validate_selected_publication_surface(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    expected_selection: BehaviorSelection,
) -> Result<(), FreshType17InitialBehaviorError> {
    validate_common_publication_surface(entity, metadata)?;
    if entity.initial_behavior != RetailRuntimeValue::Known(Some(expected_selection))
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
    {
        return Err(FreshType17InitialBehaviorError::SelectedInitializerResolutionMismatch);
    }
    Ok(())
}

fn validate_common_publication_surface(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), FreshType17InitialBehaviorError> {
    if !entity.active {
        return Err(FreshType17InitialBehaviorError::EntityInactive);
    }
    let spawn_index = entity
        .authored_spawn_index
        .ok_or(FreshType17InitialBehaviorError::MissingAuthoredSpawnIndex)?;
    if !FRESH_LEVEL1_TYPE17_SPAWN_INDICES.contains(&spawn_index) {
        return Err(
            FreshType17InitialBehaviorError::UnsupportedAuthoredSpawnIndex {
                actual: spawn_index,
            },
        );
    }
    if entity.entity_type != TYPE17_IMPACT_ENTITY_TYPE {
        return Err(FreshType17InitialBehaviorError::UnexpectedEntityType {
            actual: entity.entity_type,
        });
    }
    let expected_model = Some(usize::from(TYPE17_IMPACT_MODEL_ID));
    if entity.model_slots != [expected_model; 4] {
        return Err(FreshType17InitialBehaviorError::UnexpectedModelSlots {
            actual: entity.model_slots,
        });
    }
    if entity.model_index != expected_model {
        return Err(FreshType17InitialBehaviorError::UnexpectedActiveModel {
            actual: entity.model_index,
        });
    }
    if entity.collision.state_flags_at_0x08
        != RetailStateWord::exact(FRESH_LEVEL1_TYPE17_INITIAL_STATE_RAW)
    {
        return Err(FreshType17InitialBehaviorError::UnexpectedInitialState {
            actual: entity.collision.state_flags_at_0x08,
        });
    }
    if entity.actor_tasks_nonempty_for_initial_publication() {
        return Err(FreshType17InitialBehaviorError::BirthTaskTableNotEmpty);
    }

    match entity.actor_common_axis_descriptor {
        RetailRuntimeValue::Known(actual) if actual == TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR => {}
        RetailRuntimeValue::Known(_) => {
            return Err(FreshType17InitialBehaviorError::UnexpectedActorCommonAxisDescriptor)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(FreshType17InitialBehaviorError::ActorCommonAxisDescriptorUnresolved)
        }
    }
    match metadata.common_mover_topology {
        RetailRuntimeValue::Known(actual) if actual == TYPE17_MODEL256_COMPONENT_TOPOLOGY => {}
        RetailRuntimeValue::Known(_) => {
            return Err(FreshType17InitialBehaviorError::UnexpectedComponentTopology)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(FreshType17InitialBehaviorError::ComponentTopologyUnresolved)
        }
    }
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(FreshType17InitialBehaviorError::MissingInitializer)?;
    if initializer.common_axis_descriptor != TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR {
        return Err(FreshType17InitialBehaviorError::UnexpectedMetadataCommonAxisDescriptor);
    }
    let sub_a_descriptor = match metadata.sub_a_propulsion_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
        RetailRuntimeValue::Known(None) => {
            return Err(FreshType17InitialBehaviorError::SubADescriptorAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(FreshType17InitialBehaviorError::SubADescriptorUnresolved)
        }
    };
    if sub_a_descriptor.target_speed_base_raw != TYPE17_MODEL256_SUB_A_TARGET_SPEED_BASE_RAW {
        return Err(
            FreshType17InitialBehaviorError::UnexpectedSubATargetSpeedBase {
                actual: sub_a_descriptor.target_speed_base_raw,
            },
        );
    }
    match entity.sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(_)) => {}
        RetailRuntimeValue::Known(None) => {
            return Err(FreshType17InitialBehaviorError::SubARuntimeAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(FreshType17InitialBehaviorError::SubARuntimeUnresolved)
        }
    }
    let sub_h_descriptor = match &metadata.sub_h_external_frame_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
        RetailRuntimeValue::Known(None) => {
            return Err(FreshType17InitialBehaviorError::SubHDescriptorAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(FreshType17InitialBehaviorError::SubHDescriptorUnresolved)
        }
    };
    if sub_h_descriptor.records.len() != TYPE17_MODEL256_SUB_H_RECORD_COUNT {
        return Err(
            FreshType17InitialBehaviorError::UnexpectedSubHDescriptorRecordCount {
                actual: sub_h_descriptor.records.len(),
            },
        );
    }
    let sub_h_runtime = match &entity.sub_h_external_frame_runtime {
        RetailRuntimeValue::Known(Some(runtime)) => runtime,
        RetailRuntimeValue::Known(None) => {
            return Err(FreshType17InitialBehaviorError::SubHRuntimeAbsent)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(FreshType17InitialBehaviorError::SubHRuntimeUnresolved)
        }
    };
    if sub_h_runtime.records().len() != TYPE17_MODEL256_SUB_H_RECORD_COUNT {
        return Err(
            FreshType17InitialBehaviorError::UnexpectedSubHRuntimeRecordCount {
                actual: sub_h_runtime.records().len(),
            },
        );
    }
    Ok(())
}

trait FreshType17BirthTaskState {
    fn actor_tasks_nonempty_for_initial_publication(&self) -> bool;
}

impl FreshType17BirthTaskState for Entity {
    fn actor_tasks_nonempty_for_initial_publication(&self) -> bool {
        [
            ActorTaskSlot::Primary,
            ActorTaskSlot::Secondary,
            ActorTaskSlot::Tertiary,
        ]
        .into_iter()
        .any(|slot| self.actor_task_state(slot).is_some())
    }
}
