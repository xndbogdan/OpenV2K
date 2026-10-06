//! Shared pre-link context-allocation owner for fresh-Level-1 Type-9 actors.
//!
//! The weighted selector has already consumed its process-RNG word when this
//! owner receives the provisional entity and its linear selection receipt.
//! Native context allocation then occurs before any selected style, branch
//! initializer, unnamed fallback, task, body-basis finalization, or live-list
//! publication. Native failure destroys the complete provisional entity and
//! unregisters its handle. The Rust entity has not entered `EntityManager`'s
//! live vector, so consuming and dropping it models that terminal teardown; it
//! is not a retryable selected-initializer error.
//!
//! Rust stores behavior contexts inline, so allocation itself remains an
//! injected decision. The prepared continuation is crate-private and opaque:
//! only the outer production composer forwards its still-linear entity/receipt
//! pair to one of the four selected initializer adapters.

use crate::{
    entity::Entity,
    entity_collision_state::EntityTypeRuntimeMetadata,
    ordinary_type9_initial_selection::FreshLevel1Type9WeightedSelection,
    ordinary_type9_selected_initializer::{
        preflight_fresh_level1_type9_selected_initializer, OrdinaryType9SelectedEvidence,
        OrdinaryType9SelectedInitializerPreflightError,
    },
};

/// Size passed to native `Mem_Alloc` by retail `FUN_0040ABE0` and demo
/// `FUN_0040ABF0` for one fresh behavior context.
pub const FRESH_LEVEL1_TYPE9_CONTEXT_ALLOCATION_SIZE_BYTES: usize = 0x1c;

/// Injected result at the native fresh-context allocation boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshLevel1Type9ContextAllocationDecision {
    Prepared,
    Failed,
}

/// Read-only allocation request. It contains diagnostics, never selector or
/// provisional-entity authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshLevel1Type9ContextAllocationRequest {
    selected: OrdinaryType9SelectedEvidence,
}

impl FreshLevel1Type9ContextAllocationRequest {
    pub const fn selected(self) -> OrdinaryType9SelectedEvidence {
        self.selected
    }

    pub const fn size_bytes(self) -> usize {
        FRESH_LEVEL1_TYPE9_CONTEXT_ALLOCATION_SIZE_BYTES
    }
}

/// Why the shared outer owner destroyed a provisional allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreshLevel1Type9InitialConstructionFailureKind {
    /// Port evidence changed or was inauthentic before the allocator seam.
    /// The allocator was not called.
    Preflight(OrdinaryType9SelectedInitializerPreflightError),
    /// Native fresh-context allocation returned its nonzero allocation error.
    ContextAllocationFailed,
}

/// Diagnostics retained after terminal provisional-entity destruction.
///
/// Deliberately contains neither `Entity`, pending component custody, nor the
/// weighted-selection receipt. The id is diagnostic only: its allocation was
/// never admitted to the live list and cannot be used as a surviving handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreshLevel1Type9InitialConstructionFailure {
    invalidated_entity_id: u32,
    selected: OrdinaryType9SelectedEvidence,
    kind: FreshLevel1Type9InitialConstructionFailureKind,
}

impl FreshLevel1Type9InitialConstructionFailure {
    pub const fn invalidated_entity_id(&self) -> u32 {
        self.invalidated_entity_id
    }

    pub const fn selected(&self) -> OrdinaryType9SelectedEvidence {
        self.selected
    }

    pub const fn kind(&self) -> &FreshLevel1Type9InitialConstructionFailureKind {
        &self.kind
    }
}

/// Linear custody of one selected but still-unlinked Type-9 allocation.
///
/// Construction is crate-private so detached callers cannot relabel a live
/// entity as provisional. The manager composer creates this only while its
/// `Entity` remains local, before `append_live_entity`.
pub(crate) struct FreshLevel1Type9ProvisionalConstruction {
    entity: Entity,
    selection: FreshLevel1Type9WeightedSelection,
}

impl FreshLevel1Type9ProvisionalConstruction {
    pub(crate) fn new(entity: Entity, selection: FreshLevel1Type9WeightedSelection) -> Self {
        Self { entity, selection }
    }
}

/// Opaque successful fresh-context continuation.
///
/// It still owns both linear values. Only the outer composer may unpack them
/// to dispatch the selected initializer, then link/finalize the entity.
pub(crate) struct FreshLevel1Type9ContextPrepared {
    entity: Entity,
    selection: FreshLevel1Type9WeightedSelection,
}

impl FreshLevel1Type9ContextPrepared {
    pub(crate) fn into_parts(self) -> (Entity, FreshLevel1Type9WeightedSelection) {
        (self.entity, self.selection)
    }
}

/// Authenticate and attempt native fresh-context allocation before any
/// selected branch can publish.
///
/// Both failure paths consume and destroy the provisional `Entity` and its
/// selection receipt. `ContextAllocationFailed` therefore cannot be retried
/// and cannot enter the selected initializer's unnamed fallback. Success
/// returns the only continuation capable of forwarding the pair.
pub(crate) fn prepare_fresh_level1_type9_context(
    provisional: FreshLevel1Type9ProvisionalConstruction,
    metadata: &EntityTypeRuntimeMetadata,
    allocate: impl FnOnce(
        FreshLevel1Type9ContextAllocationRequest,
    ) -> FreshLevel1Type9ContextAllocationDecision,
) -> Result<FreshLevel1Type9ContextPrepared, FreshLevel1Type9InitialConstructionFailure> {
    let selected = OrdinaryType9SelectedEvidence::from_receipt(&provisional.selection);
    if let Err(error) = preflight_fresh_level1_type9_selected_initializer(
        &provisional.entity,
        &provisional.selection,
        metadata,
    ) {
        return Err(destroy_provisional(
            provisional,
            selected,
            FreshLevel1Type9InitialConstructionFailureKind::Preflight(error),
        ));
    }

    let request = FreshLevel1Type9ContextAllocationRequest { selected };
    match allocate(request) {
        FreshLevel1Type9ContextAllocationDecision::Prepared => {
            Ok(FreshLevel1Type9ContextPrepared {
                entity: provisional.entity,
                selection: provisional.selection,
            })
        }
        FreshLevel1Type9ContextAllocationDecision::Failed => Err(destroy_provisional(
            provisional,
            selected,
            FreshLevel1Type9InitialConstructionFailureKind::ContextAllocationFailed,
        )),
    }
}

fn destroy_provisional(
    provisional: FreshLevel1Type9ProvisionalConstruction,
    selected: OrdinaryType9SelectedEvidence,
    kind: FreshLevel1Type9InitialConstructionFailureKind,
) -> FreshLevel1Type9InitialConstructionFailure {
    let invalidated_entity_id = provisional.entity.id;
    let FreshLevel1Type9ProvisionalConstruction { entity, selection } = provisional;
    drop(selection);
    drop(entity);
    FreshLevel1Type9InitialConstructionFailure {
        invalidated_entity_id,
        selected,
        kind,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::{
        actor_animation::ActorAnimationController,
        actor_task_dispatcher::ActorTaskRuntime,
        actor_task_owner::ActorTaskSlot,
        common_mover::{sub_d::ORDINARY_TYPE9_SUB_D, SubAPropulsionRuntime},
        entity::{Entity, EntityKind},
        entity_collision_state::{EntityInitializerSpec, RetailRuntimeValue, RetailStateWord},
        main_base_type9_abort::{
            LEVEL_ONE_TYPE9_BEHAVIOR_CHOICES, LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
            LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR, LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY,
            LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS, LEVEL_ONE_TYPE9_DEATH_SOUND_ID,
            LEVEL_ONE_TYPE9_ENTITY_TYPE, LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW,
            LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW, LEVEL_ONE_TYPE9_MASS_RAW, LEVEL_ONE_TYPE9_MODEL_ID,
            LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR, LEVEL_ONE_TYPE9_SUB_B_DESCRIPTOR,
            LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR,
        },
        ordinary_type9_initial_selection::{
            plan_fresh_level1_type9_weighted_selection, FreshLevel1Type9EntityRef,
            FreshLevel1Type9InitializerIdentity, FreshLevel1Type9WeightedSelectionRequest,
            LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID,
            LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK, LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID,
            LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK, LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID,
            LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID,
        },
        ordinary_type9_live::{
            admit_fresh_level1_ordinary_type9, FreshLevel1OrdinaryType9SpawnFacts,
            OrdinaryType9SelectedRuntimeKind,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_MASK,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE,
        },
        ordinary_type9_wander_initializer::{
            apply_selected_ordinary_type9_wander_initializer,
            OrdinaryType9WanderAllocationDecision, OrdinaryType9WanderInitializerOutcome,
        },
    };

    const OWNER_ID: u32 = 0x04A9_0001;
    const SPAWN_INDEX: usize = 9;
    const IMMUTABLE_ANCHOR: [i16; 3] = [111, 22, -333];

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [LEVEL_ONE_TYPE9_MODEL_ID as u16; 4],
            mass_raw: LEVEL_ONE_TYPE9_MASS_RAW,
            capability_flags: LEVEL_ONE_TYPE9_CAPABILITY_FLAGS,
            initial_health_raw: Some(LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW),
            accepted_hit_presentation_sound_id: RetailRuntimeValue::Known(Some(95)),
            death_sound_id: RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE9_DEATH_SOUND_ID)),
            constructor_sound_attachment_id: RetailRuntimeValue::Known(None),
            generic_hit_sound_id: RetailRuntimeValue::Known(None),
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR,
            )),
            sub_b_lateral_descriptor: RetailRuntimeValue::Known(Some(
                LEVEL_ONE_TYPE9_SUB_B_DESCRIPTOR,
            )),
            sub_d_steering_descriptor: RetailRuntimeValue::Known(Some(ORDINARY_TYPE9_SUB_D)),
            actor_animation_descriptor: RetailRuntimeValue::Known(Some(
                LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR,
            )),
            common_mover_topology: RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW,
                common_axis_descriptor: LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR,
                behavior_choices: LEVEL_ONE_TYPE9_BEHAVIOR_CHOICES.to_vec().into_boxed_slice(),
                behavior_rule_ref: 1,
                alternate_behavior_class_ref: u32::from(LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS),
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn admission() -> crate::ordinary_type9_live::FreshLevel1OrdinaryType9Admission {
        admit_fresh_level1_ordinary_type9(FreshLevel1OrdinaryType9SpawnFacts {
            retail_first_world: true,
            authored_spawn_index: SPAWN_INDEX,
            entity_type: LEVEL_ONE_TYPE9_ENTITY_TYPE,
            active_model_slot: RetailRuntimeValue::Known(0),
            active_model: Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)),
            rotation: [0; 3],
            immutable_anchor_raw_at_0x90: RetailRuntimeValue::Known(IMMUTABLE_ANCHOR),
        })
        .expect("exact fresh Type-9 admission")
    }

    fn exact_entity() -> Entity {
        let mut entity = Entity::unresolved_port_entity(
            OWNER_ID,
            EntityKind::Unknown(LEVEL_ONE_TYPE9_ENTITY_TYPE),
            LEVEL_ONE_TYPE9_ENTITY_TYPE,
        );
        entity.authored_spawn_index = Some(SPAWN_INDEX);
        entity.model_slots = [Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)); 4];
        entity.model_index = Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID));
        entity.mass_raw = LEVEL_ONE_TYPE9_MASS_RAW;
        entity.capability_flags = LEVEL_ONE_TYPE9_CAPABILITY_FLAGS;
        entity.collision.state_flags_at_0x08 = RetailStateWord::from_known_bits(
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_MASK,
        );
        entity.collision.default_state_flags_at_0xc8 =
            RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW);
        entity.collision.health_raw = RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW);
        entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        entity.initial_behavior = RetailRuntimeValue::Unresolved;
        entity.current_behavior_context = RetailRuntimeValue::Unresolved;
        entity.actor_common_axis_descriptor =
            RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR);
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            SubAPropulsionRuntime::pending_constructor_rng(LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR),
        ));
        entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(None);
        entity.actor_animation_runtime = RetailRuntimeValue::Known(Some(
            ActorAnimationController::from_descriptor(LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR).unwrap(),
        ));
        entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
        entity.ordinary_type9_pending_initial_selection =
            Some(admission().pending_initial_selection());
        entity.set_motion_raw(IMMUTABLE_ANCHOR, [0; 3]);
        entity
    }

    fn owner_snapshot(entity: &Entity) -> FreshLevel1Type9EntityRef {
        FreshLevel1Type9EntityRef {
            id: entity.id,
            entity_type: entity.entity_type,
            position_raw: entity.position_raw(),
            state_flags_raw: entity.collision.state_flags_at_0x08,
            capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
            attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
        }
    }

    fn selected_receipt(
        identity: FreshLevel1Type9InitializerIdentity,
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> FreshLevel1Type9WeightedSelection {
        let candidate_mask = match identity {
            FreshLevel1Type9InitializerIdentity::RunAwayAcquiring => {
                Some(LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK)
            }
            FreshLevel1Type9InitializerIdentity::AttractAttentionInitial => {
                Some(LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK)
            }
            FreshLevel1Type9InitializerIdentity::GoToJob => {
                Some(crate::ordinary_type9_initial_selection::LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK)
            }
            FreshLevel1Type9InitializerIdentity::WanderNearLocation => None,
        };
        let candidates = candidate_mask
            .map(|capability_flags| FreshLevel1Type9EntityRef {
                id: OWNER_ID + 1,
                entity_type: 99,
                position_raw: IMMUTABLE_ANCHOR,
                state_flags_raw: RetailStateWord::exact(1),
                capability_flags: RetailRuntimeValue::Known(capability_flags),
                attached_entity_handle: RetailRuntimeValue::Known(None),
            })
            .into_iter()
            .collect::<Vec<_>>();
        let receipt = plan_fresh_level1_type9_weighted_selection(
            FreshLevel1Type9WeightedSelectionRequest {
                admission: admission(),
                authored_spawn_index: SPAWN_INDEX,
                active_model_id: usize::from(LEVEL_ONE_TYPE9_MODEL_ID),
                metadata,
                owner: owner_snapshot(entity),
                candidates_in_intrusive_order: &candidates,
            },
            || 0,
        )
        .expect("exact selector identity");
        assert_eq!(receipt.initializer_identity(), identity);
        receipt
    }

    fn class_id(identity: FreshLevel1Type9InitializerIdentity) -> u8 {
        match identity {
            FreshLevel1Type9InitializerIdentity::RunAwayAcquiring => {
                LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID as u8
            }
            FreshLevel1Type9InitializerIdentity::AttractAttentionInitial => {
                LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID as u8
            }
            FreshLevel1Type9InitializerIdentity::GoToJob => {
                LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID as u8
            }
            FreshLevel1Type9InitializerIdentity::WanderNearLocation => {
                LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID as u8
            }
        }
    }

    #[test]
    fn all_four_context_allocation_failures_consume_provisional_custody() {
        let metadata = exact_metadata();
        for identity in [
            FreshLevel1Type9InitializerIdentity::RunAwayAcquiring,
            FreshLevel1Type9InitializerIdentity::AttractAttentionInitial,
            FreshLevel1Type9InitializerIdentity::GoToJob,
            FreshLevel1Type9InitializerIdentity::WanderNearLocation,
        ] {
            let entity = exact_entity();
            let receipt = selected_receipt(identity, &entity, &metadata);
            let allocator_calls = Cell::new(0);
            let result = prepare_fresh_level1_type9_context(
                FreshLevel1Type9ProvisionalConstruction::new(entity, receipt),
                &metadata,
                |request| {
                    allocator_calls.set(allocator_calls.get() + 1);
                    assert_eq!(request.size_bytes(), 0x1c);
                    assert_eq!(
                        request.selected().selection.program.class_id,
                        class_id(identity)
                    );
                    FreshLevel1Type9ContextAllocationDecision::Failed
                },
            );
            let failure = match result {
                Err(failure) => failure,
                Ok(_) => panic!("failed context allocation returned a continuation"),
            };
            assert_eq!(allocator_calls.get(), 1);
            assert_eq!(failure.invalidated_entity_id(), OWNER_ID);
            assert_eq!(failure.selected().owner_id, OWNER_ID);
            assert_eq!(
                failure.selected().selection.program.class_id,
                class_id(identity)
            );
            assert_eq!(
                failure.kind(),
                &FreshLevel1Type9InitialConstructionFailureKind::ContextAllocationFailed
            );
        }
    }

    #[test]
    fn stale_selection_is_destroyed_before_allocator_entry() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let receipt = selected_receipt(
            FreshLevel1Type9InitializerIdentity::WanderNearLocation,
            &entity,
            &metadata,
        );
        entity.set_motion_raw([IMMUTABLE_ANCHOR[0] + 1, 22, -333], [0; 3]);
        let allocator_calls = Cell::new(0);
        let result = prepare_fresh_level1_type9_context(
            FreshLevel1Type9ProvisionalConstruction::new(entity, receipt),
            &metadata,
            |_| {
                allocator_calls.set(allocator_calls.get() + 1);
                FreshLevel1Type9ContextAllocationDecision::Prepared
            },
        );
        let failure = match result {
            Err(failure) => failure,
            Ok(_) => panic!("stale selection reached context allocation"),
        };
        assert_eq!(allocator_calls.get(), 0);
        assert!(matches!(
            failure.kind(),
            FreshLevel1Type9InitialConstructionFailureKind::Preflight(
                OrdinaryType9SelectedInitializerPreflightError::OwnerSnapshotMismatch { .. }
            )
        ));
        assert_eq!(failure.invalidated_entity_id(), OWNER_ID);
        assert_eq!(failure.selected().owner_id, OWNER_ID);
    }

    #[test]
    fn prepared_context_feeds_existing_wander_adapter_without_publication() {
        let metadata = exact_metadata();
        let entity = exact_entity();
        let receipt = selected_receipt(
            FreshLevel1Type9InitializerIdentity::WanderNearLocation,
            &entity,
            &metadata,
        );
        let allocator_calls = Cell::new(0);
        let prepared = match prepare_fresh_level1_type9_context(
            FreshLevel1Type9ProvisionalConstruction::new(entity, receipt),
            &metadata,
            |request| {
                allocator_calls.set(allocator_calls.get() + 1);
                assert_eq!(
                    request.selected().selection.program.class_id,
                    LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID as u8
                );
                FreshLevel1Type9ContextAllocationDecision::Prepared
            },
        ) {
            Ok(prepared) => prepared,
            Err(failure) => panic!("exact context preparation failed: {failure:?}"),
        };
        assert_eq!(allocator_calls.get(), 1);

        let (mut entity, receipt) = prepared.into_parts();
        assert_eq!(entity.initial_behavior, RetailRuntimeValue::Unresolved);
        assert_eq!(
            entity.current_behavior_context,
            RetailRuntimeValue::Unresolved
        );
        assert!(entity.ordinary_type9_pending_initial_selection.is_some());
        assert!(entity.ordinary_type9_selected_component_runtime.is_none());

        let outcome = apply_selected_ordinary_type9_wander_initializer(
            &mut entity,
            receipt,
            &metadata,
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            || 0x1234_5678,
        )
        .expect("prepared context must remain acceptable to the selected adapter");
        assert!(matches!(
            outcome,
            OrdinaryType9WanderInitializerOutcome::Published { .. }
        ));
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::OrdinaryType9Wander(_))
        ));
        assert_eq!(entity.ordinary_type9_pending_initial_selection, None);
        assert_eq!(
            entity
                .ordinary_type9_selected_component_runtime
                .expect("selected branch takes pending component custody")
                .kind(),
            OrdinaryType9SelectedRuntimeKind::WanderNearPublished
        );
    }
}
