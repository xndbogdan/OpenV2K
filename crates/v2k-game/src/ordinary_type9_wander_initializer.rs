//! Bounded already-selected Type-9 class-6 initializer publication.
//!
//! This adapter consumes only an exact fresh-Level-1 Wander Near receipt. It
//! publishes the common selected descriptor/context/state prefix, executes
//! `FUN_0040AD10`'s Tertiary -> Secondary -> Primary transaction, and moves
//! pending component custody to a terminal selected or initializer-fallback
//! owner. It deliberately stops before entity linking and `FUN_00413F70`
//! wrapper finalization and does not own the process-global selector RNG.

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskOwner, ActorTaskSlot},
    common_mover::{shared_initializer_target_speed_raw, SubAPropulsionRuntime},
    entity::Entity,
    entity_behavior::behavior_program,
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    ordinary_type9_initial_selection::{
        FreshLevel1Type9InitializerIdentity, FreshLevel1Type9WeightedSelection,
        LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID,
    },
    ordinary_type9_live::{
        issue_selected_wander_live_owner, OrdinaryType9SelectedRuntimeKind,
        OrdinaryType9SelectedWanderLiveOwner,
    },
    ordinary_type9_selected_initializer::{
        preflight_fresh_level1_type9_selected_initializer, OrdinaryType9SelectedEvidence,
        OrdinaryType9SelectedInitializerPreflight, OrdinaryType9SelectedInitializerPreflightError,
    },
    ordinary_type9_wander_owner::{plan_ordinary_type9_wander_setup, OrdinaryType9WanderTaskSpec},
};

/// Caller decision at the exact Primary allocation/private-initialization seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9WanderAllocationDecision {
    Prepared,
    Failed,
}

pub type OrdinaryType9WanderSelectionEvidence = OrdinaryType9SelectedEvidence;

/// Successful Type-9 Sub-A-only constructor suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9WanderConstructorEvidence {
    pub random_sample_low16: u16,
    pub sub_a_target_speed_raw: i32,
}

/// The failed class-6 task phase consumed by outer C6B0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9WanderInitializerFailure {
    pub action_index: usize,
    pub slot: ActorTaskSlot,
}

/// Result of the shared class-6 Tertiary -> Secondary -> Primary task
/// transaction after its behavior context has already been published.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrdinaryType9WanderTaskTransactionOutcome {
    Published(OrdinaryType9WanderConstructorEvidence),
    AllocationFailed(OrdinaryType9WanderInitializerFailure),
}

/// Terminal result after the class-6 descriptor has entered outer C6B0.
#[derive(Debug, PartialEq, Eq)]
pub enum OrdinaryType9WanderInitializerOutcome {
    Published {
        selected: OrdinaryType9WanderSelectionEvidence,
        constructor: OrdinaryType9WanderConstructorEvidence,
        live_owner: OrdinaryType9SelectedWanderLiveOwner,
    },
    InitializerFallbackPublished {
        selected: OrdinaryType9WanderSelectionEvidence,
        failure: OrdinaryType9WanderInitializerFailure,
    },
}

/// Rejection before selected-prefix publication, allocation, or constructor
/// RNG. The failure receipt retains the still-linear selector authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9WanderInitializerError {
    SelectedInitializerNotCanonicalWander,
    EntityPreflight(OrdinaryType9SelectedInitializerPreflightError),
}

#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9WanderInitializerFailureReceipt {
    pub error: OrdinaryType9WanderInitializerError,
    receipt: FreshLevel1Type9WeightedSelection,
}

impl OrdinaryType9WanderInitializerFailureReceipt {
    pub fn into_receipt(self) -> FreshLevel1Type9WeightedSelection {
        self.receipt
    }

    pub const fn receipt(&self) -> &FreshLevel1Type9WeightedSelection {
        &self.receipt
    }
}

#[derive(Debug)]
struct OrdinaryType9WanderPreflight {
    entity: OrdinaryType9SelectedInitializerPreflight,
    selected_evidence: OrdinaryType9WanderSelectionEvidence,
    sub_a_target_speed_base_raw: i16,
}

/// Publish one exact already-selected fresh-Level-1 Type-9 Wander Near task.
///
/// `allocate` is called once after Tertiary and Secondary clear. A failed
/// allocation is terminal outer fallback and consumes no constructor word.
/// Success consumes exactly one constructor word after allocation, applies the
/// Sub-A reset, and only then publishes Primary.
pub fn apply_selected_ordinary_type9_wander_initializer(
    entity: &mut Entity,
    receipt: FreshLevel1Type9WeightedSelection,
    metadata: &EntityTypeRuntimeMetadata,
    mut allocate: impl FnMut(OrdinaryType9WanderTaskSpec) -> OrdinaryType9WanderAllocationDecision,
    mut next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9WanderInitializerOutcome, OrdinaryType9WanderInitializerFailureReceipt> {
    let preflight = match preflight_selected_wander(entity, &receipt, metadata) {
        Ok(preflight) => preflight,
        Err(error) => {
            return Err(OrdinaryType9WanderInitializerFailureReceipt { error, receipt });
        }
    };

    // The branch is terminal after this common publication. Class 6 performs
    // no class-10 common-axis copy; its exact actor axis remains untouched.
    let published = preflight.entity.publish(entity);
    let selected = preflight.selected_evidence;
    match apply_ordinary_type9_wander_task_transaction(
        entity,
        preflight.sub_a_target_speed_base_raw,
        &mut allocate,
        &mut next_constructor_word,
    ) {
        OrdinaryType9WanderTaskTransactionOutcome::Published(constructor) => {
            published.finish_success(
                entity,
                OrdinaryType9SelectedRuntimeKind::WanderNearPublished,
            );
            let live_owner = issue_selected_wander_live_owner(entity, metadata);
            Ok(OrdinaryType9WanderInitializerOutcome::Published {
                selected,
                constructor,
                live_owner,
            })
        }
        OrdinaryType9WanderTaskTransactionOutcome::AllocationFailed(failure) => {
            published.finish_initializer_fallback(entity);
            Ok(
                OrdinaryType9WanderInitializerOutcome::InitializerFallbackPublished {
                    selected,
                    failure,
                },
            )
        }
    }
}

/// Apply the exact class-6 task transaction after the enclosing behavior
/// owner has published its selected context and state policy.
///
/// Allocation is attempted only after Tertiary and Secondary have been
/// cleared. A failed allocation consumes no constructor word and leaves the
/// old Primary installed for the enclosing C6B0 fallback to clear. Success
/// consumes one word, applies Sub-A, and publishes the prepared Primary.
pub(crate) fn apply_ordinary_type9_wander_task_transaction(
    entity: &mut Entity,
    sub_a_target_speed_base_raw: i16,
    allocate: impl FnMut(OrdinaryType9WanderTaskSpec) -> OrdinaryType9WanderAllocationDecision,
    next_constructor_word: impl FnMut() -> u32,
) -> OrdinaryType9WanderTaskTransactionOutcome {
    let owner_position_raw = entity.position_raw();
    let Entity {
        actor_tasks,
        sub_a_propulsion_runtime,
        ..
    } = entity;
    let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
        unreachable!("authenticated class-6 transaction retains live Sub-A storage")
    };

    apply_ordinary_type9_wander_task_transaction_parts(
        owner_position_raw,
        actor_tasks,
        sub_a,
        sub_a_target_speed_base_raw,
        allocate,
        next_constructor_word,
        |_| {},
    )
}

/// Borrow-split form of the exact class-6 task transaction used by retained
/// production owners that already hold the surrounding entity components.
pub(crate) fn apply_ordinary_type9_wander_task_transaction_parts(
    owner_position_raw: [i16; 3],
    actor_tasks: &mut ActorTaskOwner<ActorTaskRuntime>,
    sub_a: &mut SubAPropulsionRuntime,
    sub_a_target_speed_base_raw: i16,
    mut allocate: impl FnMut(OrdinaryType9WanderTaskSpec) -> OrdinaryType9WanderAllocationDecision,
    mut next_constructor_word: impl FnMut() -> u32,
    retire: impl FnMut(&ActorTaskRuntime),
) -> OrdinaryType9WanderTaskTransactionOutcome {
    let mut constructor = None;
    let setup_result =
        plan_ordinary_type9_wander_setup(owner_position_raw, sub_a_target_speed_base_raw)
            .apply_with_retirement(
                actor_tasks,
                sub_a,
                |specification| {
                    if allocate(specification) == OrdinaryType9WanderAllocationDecision::Failed {
                        return Err(());
                    }
                    let random_word = next_constructor_word();
                    let random_sample_low16 = random_word as u16;
                    constructor = Some(OrdinaryType9WanderConstructorEvidence {
                        random_sample_low16,
                        sub_a_target_speed_raw: shared_initializer_target_speed_raw(
                            sub_a_target_speed_base_raw,
                            random_sample_low16,
                        ),
                    });
                    Ok(specification
                        .prepare_after_allocation(|| random_word)
                        .map_task(ActorTaskRuntime::OrdinaryType9Wander))
                },
                retire,
            );

    match setup_result {
        Ok(()) => OrdinaryType9WanderTaskTransactionOutcome::Published(
            constructor.expect("successful class-6 setup applies exactly one suffix"),
        ),
        Err(error) => {
            debug_assert!(constructor.is_none());
            OrdinaryType9WanderTaskTransactionOutcome::AllocationFailed(
                OrdinaryType9WanderInitializerFailure {
                    action_index: error.action_index,
                    slot: error.slot,
                },
            )
        }
    }
}

fn preflight_selected_wander(
    entity: &Entity,
    receipt: &FreshLevel1Type9WeightedSelection,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<OrdinaryType9WanderPreflight, OrdinaryType9WanderInitializerError> {
    let canonical_program = behavior_program(LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID)
        .expect("class-6 Wander Near is statically audited");
    if receipt.initializer_identity() != FreshLevel1Type9InitializerIdentity::WanderNearLocation
        || receipt.selection().choice_index != 3
        || receipt.selection().program != canonical_program
    {
        return Err(OrdinaryType9WanderInitializerError::SelectedInitializerNotCanonicalWander);
    }
    let entity = preflight_fresh_level1_type9_selected_initializer(entity, receipt, metadata)
        .map_err(OrdinaryType9WanderInitializerError::EntityPreflight)?;
    let RetailRuntimeValue::Known(Some(sub_a_descriptor)) = metadata.sub_a_propulsion_descriptor
    else {
        unreachable!("common exact-metadata preflight retained Type-9 Sub-A descriptor")
    };
    Ok(OrdinaryType9WanderPreflight {
        entity,
        selected_evidence: OrdinaryType9SelectedEvidence::from_receipt(receipt),
        sub_a_target_speed_base_raw: sub_a_descriptor.target_speed_base_raw,
    })
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::num::NonZeroU64;

    use super::*;
    use crate::{
        actor_animation::ActorAnimationController,
        actor_task_owner::{ActorTaskSlot, PreparedActorTask},
        common_mover::{
            sub_d::ORDINARY_TYPE9_SUB_D, type9_attitude::Type9BodyBasis,
            type9_transaction::OrdinaryType9TransactionId, SubAPropulsionRuntime,
        },
        entity::{Entity, EntityKind},
        entity_behavior::{
            ActiveBehaviorStyle, BehaviorChoiceListSource, BehaviorDescriptorIdentity,
        },
        entity_collision_state::{
            EntityInitializerSpec, RetailStateWord, ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
            BODY_BASIS_REBUILT_STATE_BIT,
        },
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
            FreshLevel1Type9WeightedSelectionRequest,
            LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK,
        },
        ordinary_type9_live::{
            admit_fresh_level1_ordinary_type9, FreshLevel1OrdinaryType9SpawnFacts,
            OrdinaryType9LiveFrameError, OrdinaryType9SelectedComponentRuntime,
            OrdinaryType9SelectedWanderLiveFrameError, OrdinaryType9SelectedWanderLiveFrameOutcome,
            OrdinaryType9SelectedWanderLiveFrameRequest, OrdinaryType9SelectedWanderLiveOwner,
            OrdinaryType9SelectedWanderLivePreflightError,
            OrdinaryType9SelectedWanderTransitionSelection,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_MASK,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE,
        },
        shared_retarget_mover::SharedRetargetTaskState,
        wander_near_location::{
            WanderNearCommonMoverReturn, WanderNearLifetimeStatus, WanderNearPrivateState,
        },
    };
    use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};

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

    fn wander_receipt(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> FreshLevel1Type9WeightedSelection {
        let selection = plan_fresh_level1_type9_weighted_selection(
            FreshLevel1Type9WeightedSelectionRequest {
                admission: admission(),
                authored_spawn_index: SPAWN_INDEX,
                active_model_id: usize::from(LEVEL_ONE_TYPE9_MODEL_ID),
                metadata,
                owner: owner_snapshot(entity),
                candidates_in_intrusive_order: &[],
            },
            || 0xFACE_FFFF,
        )
        .expect("only Always remains weighted");
        assert_eq!(
            selection.initializer_identity(),
            FreshLevel1Type9InitializerIdentity::WanderNearLocation
        );
        selection
    }

    fn run_away_receipt(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> FreshLevel1Type9WeightedSelection {
        let baddie = FreshLevel1Type9EntityRef {
            id: 0x04AB_0001,
            entity_type: 99,
            position_raw: IMMUTABLE_ANCHOR,
            state_flags_raw: RetailStateWord::exact(1),
            capability_flags: RetailRuntimeValue::Known(
                LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK,
            ),
            attached_entity_handle: RetailRuntimeValue::Known(None),
        };
        plan_fresh_level1_type9_weighted_selection(
            FreshLevel1Type9WeightedSelectionRequest {
                admission: admission(),
                authored_spawn_index: SPAWN_INDEX,
                active_model_id: usize::from(LEVEL_ONE_TYPE9_MODEL_ID),
                metadata,
                owner: owner_snapshot(entity),
                candidates_in_intrusive_order: &[baddie],
            },
            || 0,
        )
        .expect("baddie-nearby zero selects Run Away")
    }

    fn selected_runtime(entity: &Entity) -> OrdinaryType9SelectedComponentRuntime {
        entity
            .ordinary_type9_selected_component_runtime
            .expect("terminal initializer must retain selected component custody")
    }

    fn assert_wrapper_pending(entity: &Entity, fixed_state: u32) {
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_MASK),
            RetailRuntimeValue::Known(fixed_state)
        );
        assert_eq!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Unresolved
        );
    }

    fn published_wander_owner() -> (
        Entity,
        EntityTypeRuntimeMetadata,
        OrdinaryType9SelectedWanderLiveOwner,
    ) {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let receipt = wander_receipt(&entity, &metadata);
        let outcome = apply_selected_ordinary_type9_wander_initializer(
            &mut entity,
            receipt,
            &metadata,
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            || 0x1234_5678,
        )
        .expect("exact selected Wander publication");
        let OrdinaryType9WanderInitializerOutcome::Published { live_owner, .. } = outcome else {
            unreachable!("prepared allocation publishes class 6")
        };
        (entity, metadata, live_owner)
    }

    fn finalize_selected_wander(entity: &mut Entity) {
        let [heading_raw, pitch_raw, roll_raw] = entity.rotation_heading_pitch_roll_raw();
        entity.physical_body_basis_q31 = RetailRuntimeValue::Known(
            Type9BodyBasis::from_angle_words(heading_raw, pitch_raw, roll_raw),
        );
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    }

    fn flat_terrain() -> TerrainGrid {
        TerrainGrid {
            header: [-1 << 8, 0, 0, 0, 0],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn selected_frame_request(
        terrain: &TerrainGrid,
        transaction_id: u64,
    ) -> OrdinaryType9SelectedWanderLiveFrameRequest<'_> {
        OrdinaryType9SelectedWanderLiveFrameRequest {
            transaction_id: OrdinaryType9TransactionId::new(
                NonZeroU64::new(transaction_id).expect("test transaction ids are nonzero"),
            ),
            tracked_target: RetailRuntimeValue::Unresolved,
            terrain,
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            scheduler_mode: 0,
        }
    }

    fn set_selected_wander_target(
        entity: &mut Entity,
        owner: &OrdinaryType9SelectedWanderLiveOwner,
        target: [i16; 3],
    ) {
        let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) =
            entity.actor_tasks.task_state_mut(owner.visit().task_id)
        else {
            panic!("selected fixture retains its Wander Primary")
        };
        let mut private = state.private_state();
        private.target_position_raw = target;
        state.replace_private_state(private);
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct SelectedLiveSnapshot {
        selected: Option<OrdinaryType9SelectedComponentRuntime>,
        sub_a: RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
        animation: RetailRuntimeValue<Option<ActorAnimationController>>,
        task: ActorTaskRuntime,
        wrapper: crate::actor_task_owner::ActorTaskWrapperFlags,
        state: RetailStateWord,
        heading_raw: u16,
        velocity_raw: [i16; 3],
        basis: RetailRuntimeValue<Type9BodyBasis>,
    }

    fn selected_live_snapshot(
        entity: &Entity,
        owner: &OrdinaryType9SelectedWanderLiveOwner,
    ) -> SelectedLiveSnapshot {
        SelectedLiveSnapshot {
            selected: entity.ordinary_type9_selected_component_runtime,
            sub_a: entity.sub_a_propulsion_runtime,
            animation: entity.actor_animation_runtime,
            task: *entity
                .actor_tasks
                .task_state(owner.visit().task_id)
                .expect("selected fixture retains its captured task"),
            wrapper: entity
                .actor_tasks
                .wrapper_flags(owner.visit().task_id)
                .expect("selected fixture retains its captured wrapper"),
            state: entity.collision.state_flags_at_0x08,
            heading_raw: entity.heading_raw(),
            velocity_raw: entity.velocity_raw(),
            basis: entity.physical_body_basis_q31(),
        }
    }

    fn expect_selected_preflight_failure(
        owner: OrdinaryType9SelectedWanderLiveOwner,
        entity: &mut Entity,
        metadata: &EntityTypeRuntimeMetadata,
        terrain: &TerrainGrid,
        expected: OrdinaryType9SelectedWanderLivePreflightError,
    ) -> OrdinaryType9SelectedWanderLiveOwner {
        let before = selected_live_snapshot(entity, &owner);
        let draws = Cell::new(0);
        let failure = owner
            .tick(entity, metadata, selected_frame_request(terrain, 1), || {
                draws.set(draws.get() + 1);
                1
            })
            .expect_err("fixture mutation must fail selected-live preflight");
        assert_eq!(
            failure.error,
            OrdinaryType9SelectedWanderLiveFrameError::Preflight(expected)
        );
        let owner = failure
            .into_owner()
            .expect("mutation-free preflight failure returns linear owner");
        assert_eq!(draws.get(), 0);
        assert_eq!(selected_live_snapshot(entity, &owner), before);
        owner
    }

    #[test]
    fn success_publishes_one_primary_after_one_lazy_constructor_word() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Event {
            Allocate,
            Word(u32),
        }

        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let original_axis = entity.actor_common_axis_descriptor;
        let expected_components = entity.ordinary_type9_pending_initial_selection.unwrap();
        let receipt = wander_receipt(&entity, &metadata);
        let events = RefCell::new(Vec::new());
        let word = 0x1234_D2F6;
        let outcome = apply_selected_ordinary_type9_wander_initializer(
            &mut entity,
            receipt,
            &metadata,
            |specification| {
                assert_eq!(specification.current_position_raw(), IMMUTABLE_ANCHOR);
                assert_eq!(
                    specification.sub_a_target_speed_base_raw(),
                    LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw
                );
                events.borrow_mut().push(Event::Allocate);
                OrdinaryType9WanderAllocationDecision::Prepared
            },
            || {
                events.borrow_mut().push(Event::Word(word));
                word
            },
        )
        .unwrap();

        let OrdinaryType9WanderInitializerOutcome::Published {
            selected,
            constructor,
            live_owner,
        } = outcome
        else {
            panic!("expected selected Wander publication: {outcome:?}")
        };
        assert_eq!(events.into_inner(), [Event::Allocate, Event::Word(word)]);
        assert_eq!(selected.selection.choice_index, 3);
        assert_eq!(selected.selection.program.class_id, 6);
        assert_eq!(selected.selector_random_word, 0xFACE_FFFF);
        assert_eq!(live_owner.entity_id(), OWNER_ID);
        assert_eq!(live_owner.visit().slot, ActorTaskSlot::Primary);
        assert_eq!(
            constructor,
            OrdinaryType9WanderConstructorEvidence {
                random_sample_low16: 0xD2F6,
                sub_a_target_speed_raw: shared_initializer_target_speed_raw(
                    LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw,
                    0xD2F6,
                ),
            }
        );
        let Some(ActorTaskRuntime::OrdinaryType9Wander(primary)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("class 6 must publish one Wander Near Primary")
        };
        assert_eq!(
            primary.private_state(),
            WanderNearPrivateState::ordinary_type9(IMMUTABLE_ANCHOR)
        );
        assert_eq!(primary.elapsed_ms(), 0);
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Secondary), None);
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!()
        };
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(constructor.sub_a_target_speed_raw)
        );
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(sub_a.drive_scale_percent(), 100);
        assert_eq!(entity.actor_common_axis_descriptor, original_axis);
        assert_eq!(
            entity.initial_behavior,
            RetailRuntimeValue::Known(Some(selected.selection))
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::Named(selected.selection.program)
        );
        assert_eq!(context.active_style().audited().unwrap().variant, 0);
        assert_eq!(
            context.choice_list_source(),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        );
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(entity.ordinary_type9_pending_initial_selection, None);
        assert_eq!(selected_runtime(&entity).components(), expected_components);
        assert_eq!(
            selected_runtime(&entity).kind(),
            OrdinaryType9SelectedRuntimeKind::WanderNearPublished
        );
        assert_wrapper_pending(
            &entity,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE,
        );
    }

    #[test]
    fn allocation_failure_consumes_no_word_and_terminally_publishes_fallback() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let expected_components = entity.ordinary_type9_pending_initial_selection.unwrap();
        let initial_sub_a = entity.sub_a_propulsion_runtime;
        let receipt = wander_receipt(&entity, &metadata);
        let allocations = Cell::new(0);
        let draws = Cell::new(0);
        let outcome = apply_selected_ordinary_type9_wander_initializer(
            &mut entity,
            receipt,
            &metadata,
            |_| {
                allocations.set(allocations.get() + 1);
                OrdinaryType9WanderAllocationDecision::Failed
            },
            || {
                draws.set(draws.get() + 1);
                panic!("failed allocation must not consume constructor RNG")
            },
        )
        .unwrap();
        let OrdinaryType9WanderInitializerOutcome::InitializerFallbackPublished {
            selected,
            failure,
        } = outcome
        else {
            panic!("expected terminal fallback: {outcome:?}")
        };
        assert_eq!(selected.selection.program.class_id, 6);
        assert_eq!(
            failure,
            OrdinaryType9WanderInitializerFailure {
                action_index: 2,
                slot: ActorTaskSlot::Primary,
            }
        );
        assert_eq!((allocations.get(), draws.get()), (1, 0));
        assert_eq!(entity.sub_a_propulsion_runtime, initial_sub_a);
        for slot in [
            ActorTaskSlot::Secondary,
            ActorTaskSlot::Tertiary,
            ActorTaskSlot::Primary,
        ] {
            assert_eq!(entity.actor_task_state(slot), None);
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::InitializerFailureFallback
        );
        assert_eq!(
            context.active_style(),
            ActiveBehaviorStyle::InitializerFailureFallback
        );
        assert_eq!(selected_runtime(&entity).components(), expected_components);
        assert_eq!(
            selected_runtime(&entity).kind(),
            OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
        );
        assert_wrapper_pending(&entity, 0x0600_0801);
    }

    #[test]
    fn wrong_branch_and_stale_entity_return_receipt_without_callbacks_or_mutation() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let pending_before = entity.ordinary_type9_pending_initial_selection;
        let initial_behavior_before = entity.initial_behavior;
        let wrong_receipt = run_away_receipt(&entity, &metadata);
        let calls = Cell::new(0);
        let failure = apply_selected_ordinary_type9_wander_initializer(
            &mut entity,
            wrong_receipt,
            &metadata,
            |_| {
                calls.set(calls.get() + 1);
                OrdinaryType9WanderAllocationDecision::Prepared
            },
            || {
                calls.set(calls.get() + 1);
                0
            },
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9WanderInitializerError::SelectedInitializerNotCanonicalWander
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(
            entity.ordinary_type9_pending_initial_selection,
            pending_before
        );
        assert_eq!(entity.initial_behavior, initial_behavior_before);

        let receipt = wander_receipt(&entity, &metadata);
        entity.set_motion_raw([112, 22, -333], [0; 3]);
        let failure = apply_selected_ordinary_type9_wander_initializer(
            &mut entity,
            receipt,
            &metadata,
            |_| panic!(),
            || panic!(),
        )
        .unwrap_err();
        assert!(matches!(
            failure.error,
            OrdinaryType9WanderInitializerError::EntityPreflight(
                OrdinaryType9SelectedInitializerPreflightError::OwnerSnapshotMismatch { .. }
            )
        ));
        let receipt = failure.into_receipt();
        entity.set_motion_raw(IMMUTABLE_ANCHOR, [0; 3]);
        assert!(apply_selected_ordinary_type9_wander_initializer(
            &mut entity,
            receipt,
            &metadata,
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            || 0,
        )
        .is_ok());
    }

    #[test]
    fn success_and_fallback_both_reject_replay_before_callbacks() {
        for fail_allocation in [false, true] {
            let metadata = exact_metadata();
            let mut entity = exact_entity();
            let receipt = wander_receipt(&entity, &metadata);
            let outcome = apply_selected_ordinary_type9_wander_initializer(
                &mut entity,
                receipt,
                &metadata,
                |_| {
                    if fail_allocation {
                        OrdinaryType9WanderAllocationDecision::Failed
                    } else {
                        OrdinaryType9WanderAllocationDecision::Prepared
                    }
                },
                || 0,
            )
            .unwrap();
            assert_eq!(
                matches!(
                    outcome,
                    OrdinaryType9WanderInitializerOutcome::InitializerFallbackPublished { .. }
                ),
                fail_allocation
            );

            let replay_receipt = wander_receipt_from_terminal_snapshot(&entity, &metadata);
            let failure = apply_selected_ordinary_type9_wander_initializer(
                &mut entity,
                replay_receipt,
                &metadata,
                |_| panic!("replay must stop before allocation"),
                || panic!("replay must stop before RNG"),
            )
            .unwrap_err();
            assert_eq!(
                failure.error,
                OrdinaryType9WanderInitializerError::EntityPreflight(
                    OrdinaryType9SelectedInitializerPreflightError::SelectedComponentCustodyAlreadyPresent,
                )
            );
        }
    }

    fn wander_receipt_from_terminal_snapshot(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> FreshLevel1Type9WeightedSelection {
        let mut snapshot = exact_entity();
        snapshot.id = entity.id;
        wander_receipt(&snapshot, metadata)
    }

    #[test]
    fn preflight_rejects_existing_task_and_surface_bits_remain_exempt() {
        let metadata = exact_metadata();
        let mut blocked = exact_entity();
        let receipt = wander_receipt(&blocked, &metadata);
        blocked.actor_tasks.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                SharedRetargetTaskState::new(IMMUTABLE_ANCHOR, 500),
            )),
        );
        let failure = apply_selected_ordinary_type9_wander_initializer(
            &mut blocked,
            receipt,
            &metadata,
            |_| panic!(),
            || panic!(),
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9WanderInitializerError::EntityPreflight(
                OrdinaryType9SelectedInitializerPreflightError::BirthTaskTableNotEmpty,
            )
        );

        for surface_bits in [
            0,
            crate::entity_collision_state::FULLY_ABOVE_SURFACE_STATE_BIT,
            crate::entity_collision_state::FULLY_BELOW_SURFACE_STATE_BIT,
            crate::entity_collision_state::FULLY_ABOVE_SURFACE_STATE_BIT
                | crate::entity_collision_state::FULLY_BELOW_SURFACE_STATE_BIT,
        ] {
            let mut entity = exact_entity();
            entity.collision.state_flags_at_0x08 = RetailStateWord::exact(
                FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE | surface_bits,
            );
            let receipt = wander_receipt(&entity, &metadata);
            assert!(apply_selected_ordinary_type9_wander_initializer(
                &mut entity,
                receipt,
                &metadata,
                |_| OrdinaryType9WanderAllocationDecision::Prepared,
                || 0,
            )
            .is_ok());
            assert_eq!(
                entity.collision.state_flags_at_0x08.masked(0x0060_0000),
                RetailRuntimeValue::Known(surface_bits)
            );
        }
    }

    #[test]
    fn selected_live_owner_waits_for_finalization_and_rejects_stale_graphs_atomically() {
        let terrain = flat_terrain();
        let (mut entity, metadata, mut owner) = published_wander_owner();

        owner = expect_selected_preflight_failure(
            owner,
            &mut entity,
            &metadata,
            &terrain,
            OrdinaryType9SelectedWanderLivePreflightError::PhysicalBodyBasisUnavailable,
        );
        finalize_selected_wander(&mut entity);

        entity.id = entity.id.wrapping_add(1);
        owner = expect_selected_preflight_failure(
            owner,
            &mut entity,
            &metadata,
            &terrain,
            OrdinaryType9SelectedWanderLivePreflightError::OwnerEntityMismatch {
                expected: OWNER_ID,
                actual: OWNER_ID.wrapping_add(1),
            },
        );
        entity.id = OWNER_ID;

        let selected = selected_runtime(&entity);
        entity.ordinary_type9_selected_component_runtime =
            Some(OrdinaryType9SelectedComponentRuntime::new(
                selected.components(),
                OrdinaryType9SelectedRuntimeKind::GoToJobPublished,
            ));
        owner = expect_selected_preflight_failure(
            owner,
            &mut entity,
            &metadata,
            &terrain,
            OrdinaryType9SelectedWanderLivePreflightError::UnexpectedSelectedRuntimeKind {
                actual: OrdinaryType9SelectedRuntimeKind::GoToJobPublished,
            },
        );
        entity.ordinary_type9_selected_component_runtime = Some(selected);

        let context = entity.current_behavior_context;
        entity.current_behavior_context = RetailRuntimeValue::Unresolved;
        owner = expect_selected_preflight_failure(
            owner,
            &mut entity,
            &metadata,
            &terrain,
            OrdinaryType9SelectedWanderLivePreflightError::BehaviorContextMismatch,
        );
        entity.current_behavior_context = context;

        let duplicate = *entity
            .actor_tasks
            .task_state(owner.visit().task_id)
            .expect("captured Primary");
        entity
            .actor_tasks
            .replace_prepared(ActorTaskSlot::Secondary, PreparedActorTask::new(duplicate));
        owner = expect_selected_preflight_failure(
            owner,
            &mut entity,
            &metadata,
            &terrain,
            OrdinaryType9SelectedWanderLivePreflightError::AdditionalPublishedTask {
                slot: ActorTaskSlot::Secondary,
            },
        );
        entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);

        entity
            .collision
            .state_flags_at_0x08
            .overwrite(BODY_BASIS_REBUILT_STATE_BIT, 0);
        owner = expect_selected_preflight_failure(
            owner,
            &mut entity,
            &metadata,
            &terrain,
            OrdinaryType9SelectedWanderLivePreflightError::BodyBasisRebuiltStateBitClear,
        );
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);

        let basis = entity.physical_body_basis_q31;
        entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
        owner = expect_selected_preflight_failure(
            owner,
            &mut entity,
            &metadata,
            &terrain,
            OrdinaryType9SelectedWanderLivePreflightError::PhysicalBodyBasisUnavailable,
        );
        entity.physical_body_basis_q31 = basis;

        let state = entity.collision.state_flags_at_0x08;
        entity.collision.state_flags_at_0x08 = RetailStateWord::from_known_bits(
            state.known_value_bits(),
            state.known_mask() & !BODY_BASIS_REBUILT_STATE_BIT,
        );
        owner = expect_selected_preflight_failure(
            owner,
            &mut entity,
            &metadata,
            &terrain,
            OrdinaryType9SelectedWanderLivePreflightError::BodyBasisRebuiltStateBitUnresolved,
        );
        entity.collision.state_flags_at_0x08 = state;

        let stale_visit = owner.visit();
        let replacement = *entity
            .actor_tasks
            .task_state(stale_visit.task_id)
            .expect("captured Primary");
        entity
            .actor_tasks
            .replace_prepared(ActorTaskSlot::Primary, PreparedActorTask::new(replacement));
        let draws = Cell::new(0);
        let failure = owner
            .tick(
                &mut entity,
                &metadata,
                selected_frame_request(&terrain, 2),
                || {
                    draws.set(draws.get() + 1);
                    1
                },
            )
            .expect_err("replaced Primary identity must be stale");
        assert_eq!(
            failure.error,
            OrdinaryType9SelectedWanderLiveFrameError::Preflight(
                OrdinaryType9SelectedWanderLivePreflightError::TaskLeaseChanged,
            )
        );
        assert!(failure.owner().is_some());
        assert_eq!(draws.get(), 0);
        let Some(ActorTaskRuntime::OrdinaryType9Wander(replacement)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!()
        };
        assert_eq!(replacement.elapsed_ms(), 0);
    }

    #[test]
    fn selected_custody_commits_exact_mode_zero_and_nonzero_mover_paths() {
        let terrain = flat_terrain();
        for scheduler_mode in [0, 1] {
            let (mut entity, metadata, owner) = published_wander_owner();
            finalize_selected_wander(&mut entity);
            set_selected_wander_target(&mut entity, &owner, [3_000, 22, -333]);
            let before_components = selected_runtime(&entity).components();
            let before_animation = entity.actor_animation_runtime;
            let initial_stagger = before_components
                .sub_d_frame_owner
                .classifier_cache()
                .stagger_counter();
            let mut request = selected_frame_request(&terrain, 10 + scheduler_mode as u64);
            request.scheduler_mode = scheduler_mode;
            let draws = Cell::new(0);
            let outcome = owner
                .tick(&mut entity, &metadata, request, || {
                    draws.set(draws.get() + 1);
                    1
                })
                .expect("selected-custody mover frame");
            let OrdinaryType9SelectedWanderLiveFrameOutcome::Continue {
                owner,
                committed_prefix,
                mover_return,
            } = outcome
            else {
                panic!("active nonzero mover must continue")
            };
            assert_eq!(owner.entity_id(), OWNER_ID);
            assert_eq!(
                committed_prefix.lifetime_status,
                WanderNearLifetimeStatus::Active
            );
            assert_eq!(mover_return, WanderNearCommonMoverReturn::NonZero);
            assert_eq!(draws.get(), 1);
            assert_eq!(entity.ordinary_type9_pending_initial_selection, None);
            assert_eq!(
                selected_runtime(&entity).kind(),
                OrdinaryType9SelectedRuntimeKind::WanderNearPublished
            );
            let after_components = selected_runtime(&entity).components();
            assert_ne!(after_components, before_components);
            assert_eq!(
                after_components
                    .sub_d_frame_owner
                    .classifier_cache()
                    .stagger_counter(),
                initial_stagger.wrapping_add(1),
                "the selected path must perform the retained first query"
            );
            if scheduler_mode == 0 {
                assert_ne!(entity.actor_animation_runtime, before_animation);
            } else {
                assert_eq!(entity.actor_animation_runtime, before_animation);
            }
        }
    }

    #[test]
    fn selected_post_entry_mover_failure_consumes_owner_after_prefix_commit() {
        let terrain = flat_terrain();
        let (mut entity, metadata, owner) = published_wander_owner();
        finalize_selected_wander(&mut entity);
        let visit = owner.visit();
        let Some(ActorTaskRuntime::OrdinaryType9Wander(wander)) =
            entity.actor_tasks.task_state_mut(visit.task_id)
        else {
            panic!()
        };
        let mut private = wander.private_state();
        private.tracked_entity_handle = 0x04AB_0001;
        wander.replace_private_state(private);
        let draws = Cell::new(0);
        let failure = owner
            .tick(
                &mut entity,
                &metadata,
                selected_frame_request(&terrain, 20),
                || {
                    draws.set(draws.get() + 1);
                    1
                },
            )
            .expect_err("unresolved tracked target blocks after callback entry");
        assert!(matches!(
            failure.error,
            OrdinaryType9SelectedWanderLiveFrameError::Frame(
                OrdinaryType9LiveFrameError::FrameBlocked {
                    committed_prefix:
                        crate::ordinary_type9_wander_owner::OrdinaryType9WanderCallbackPrefix {
                            lifetime_status: WanderNearLifetimeStatus::Active,
                            ..
                        },
                    ..
                }
            )
        ));
        assert!(failure.into_owner().is_none());
        assert_eq!(draws.get(), 1);
        let Some(ActorTaskRuntime::OrdinaryType9Wander(wander)) =
            entity.actor_tasks.task_state(visit.task_id)
        else {
            panic!()
        };
        assert_eq!(wander.elapsed_ms(), 20);
    }

    #[test]
    fn selected_timeout_applies_post_unwind_gate_and_types_root_transition() {
        let terrain = flat_terrain();
        let (mut entity, metadata, owner) = published_wander_owner();
        finalize_selected_wander(&mut entity);
        set_selected_wander_target(&mut entity, &owner, [3_000, 22, -333]);
        let Some(ActorTaskRuntime::OrdinaryType9Wander(wander)) =
            entity.actor_tasks.task_state_mut(owner.visit().task_id)
        else {
            panic!()
        };
        assert_eq!(
            wander.before_callback(5_000_000),
            WanderNearLifetimeStatus::Active
        );
        entity.collision.state_flags_at_0x08.overwrite(
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
        );
        let mut request = selected_frame_request(&terrain, 30);
        request.elapsed_micros = 1_000;
        request.global_elapsed_micros = 1_000;
        let OrdinaryType9SelectedWanderLiveFrameOutcome::TransitionSuppressed {
            owner,
            transition,
            mover_return,
        } = owner.tick(&mut entity, &metadata, request, || 1).unwrap()
        else {
            panic!("known-set 0x1000 must preserve the live owner")
        };
        assert_eq!(mover_return, WanderNearCommonMoverReturn::NonZero);
        assert_eq!(transition.entity_id, OWNER_ID);
        assert_eq!(
            transition.selection,
            OrdinaryType9SelectedWanderTransitionSelection::TypeDefaultRootFirst
        );
        assert_eq!(
            transition.owner_request.committed_prefix.lifetime_status,
            WanderNearLifetimeStatus::OwnerTransitionDue
        );

        entity
            .collision
            .state_flags_at_0x08
            .overwrite(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT, 0);
        let mut request = selected_frame_request(&terrain, 31);
        request.elapsed_micros = 0;
        request.global_elapsed_micros = 0;
        let OrdinaryType9SelectedWanderLiveFrameOutcome::TransitionPending {
            transition,
            mover_return,
        } = owner.tick(&mut entity, &metadata, request, || 1).unwrap()
        else {
            panic!("known-clear 0x1000 must consume into root transition")
        };
        assert_eq!(mover_return, WanderNearCommonMoverReturn::NonZero);
        assert_eq!(transition.entity_id, OWNER_ID);
        assert_eq!(
            transition.selection,
            OrdinaryType9SelectedWanderTransitionSelection::TypeDefaultRootFirst
        );

        let (mut unresolved, metadata, owner) = published_wander_owner();
        finalize_selected_wander(&mut unresolved);
        set_selected_wander_target(&mut unresolved, &owner, [3_000, 22, -333]);
        let Some(ActorTaskRuntime::OrdinaryType9Wander(wander)) =
            unresolved.actor_tasks.task_state_mut(owner.visit().task_id)
        else {
            panic!()
        };
        wander.before_callback(5_000_000);
        let state = unresolved.collision.state_flags_at_0x08;
        unresolved.collision.state_flags_at_0x08 = RetailStateWord::from_known_bits(
            state.known_value_bits(),
            state.known_mask() & !ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
        );
        let mut request = selected_frame_request(&terrain, 32);
        request.elapsed_micros = 1_000;
        request.global_elapsed_micros = 1_000;
        let failure = owner
            .tick(&mut unresolved, &metadata, request, || 1)
            .expect_err("unresolved post-unwind gate fails closed");
        assert!(matches!(
            failure.error,
            OrdinaryType9SelectedWanderLiveFrameError::TransitionGateUnresolved {
                transition:
                    crate::ordinary_type9_live::OrdinaryType9SelectedWanderTransitionRequest {
                        selection:
                            OrdinaryType9SelectedWanderTransitionSelection::TypeDefaultRootFirst,
                        ..
                    },
                ..
            }
        ));
        assert!(failure.into_owner().is_none());
    }
}
