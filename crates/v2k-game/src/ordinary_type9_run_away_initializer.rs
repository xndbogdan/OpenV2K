//! Bounded already-selected Type-9 class-10 initializer publication.
//!
//! The weighted selector remains detached and caller-RNG-owned. This adapter
//! accepts only its opaque, linear Run Away result, re-authenticates the exact
//! pre-publication entity snapshot and retail metadata, then executes shared
//! `FUN_0040B6C0` ordering. It stops at `FUN_004104B0`'s terminal selected-
//! initializer/fallback boundary. The fresh production composer supplies the
//! shared process RNG/allocation owner, then appends the entity and runs
//! `FUN_00413F70`'s basis/bit-four finalization. Post-wrapper callers remain
//! intentionally ineligible for this initializer seam.

use crate::{
    actor_task_dispatcher::{
        prepare_shared_acquiring_runtime_task, ActorTaskRuntime, SharedGenericConstructorEffect,
        SharedGenericConstructorTopology,
    },
    actor_task_owner::{ActorTaskOwner, ActorTaskSlot},
    common_mover::SubAPropulsionRuntime,
    entity::Entity,
    entity_behavior::behavior_program,
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    ordinary_type9_initial_selection::{
        FreshLevel1Type9InitializerIdentity, FreshLevel1Type9WeightedSelection,
        LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID,
    },
    ordinary_type9_live::OrdinaryType9SelectedRuntimeKind,
    ordinary_type9_selected_initializer::{
        preflight_fresh_level1_type9_selected_initializer, OrdinaryType9SelectedEvidence,
        OrdinaryType9SelectedInitializerPreflight, OrdinaryType9SelectedInitializerPreflightError,
    },
    run_away::{
        apply_run_away_task_setup_with_retirement, RunAwayTaskPreparation, RunAwayTaskRole,
        RunAwayTaskSetupRequest,
    },
};

/// Caller decision at one exact task allocation/private-initialization seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9RunAwayAllocationDecision {
    Prepared,
    Failed,
}

/// Copyable selector diagnostics which carry no initializer authority.
pub type OrdinaryType9RunAwaySelectionEvidence = OrdinaryType9SelectedEvidence;

/// One successful Type-9 Sub-A-only `FUN_00406070` suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9RunAwayConstructorEvidence {
    pub random_sample_low16: u16,
    pub sub_a_target_speed_raw: i32,
}

/// The nested B6C0 phase whose nonzero result was consumed by outer C6B0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9RunAwayInitializerFailure {
    pub phase_index: usize,
    pub slot: ActorTaskSlot,
    pub role: RunAwayTaskRole,
}

/// Exact result of the shared Type-9 `FUN_0040B6C0` task transaction.
///
/// This boundary owns only Tertiary clearing, the two lazy task allocations,
/// their success-only constructor suffixes, and Secondary/Primary
/// publication. Fresh construction and post-callback root reselection retain
/// distinct preflight/context/fallback owners around it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrdinaryType9RunAwayTaskTransactionOutcome {
    Published {
        constructors_by_phase: [OrdinaryType9RunAwayConstructorEvidence; 2],
    },
    AllocationFailed {
        failure: OrdinaryType9RunAwayInitializerFailure,
        constructors_by_phase: [Option<OrdinaryType9RunAwayConstructorEvidence>; 2],
    },
}

/// Terminal result after the selected descriptor has entered outer C6B0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9RunAwayInitializerOutcome {
    Published {
        selected: OrdinaryType9RunAwaySelectionEvidence,
        constructors_by_phase: [OrdinaryType9RunAwayConstructorEvidence; 2],
    },
    InitializerFallbackPublished {
        selected: OrdinaryType9RunAwaySelectionEvidence,
        failure: OrdinaryType9RunAwayInitializerFailure,
        constructors_by_phase: [Option<OrdinaryType9RunAwayConstructorEvidence>; 2],
    },
}

/// A stale or inauthentic receipt/live-entity pairing. No entity state,
/// allocation callback, or constructor-word source has been touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9RunAwayInitializerError {
    SelectedInitializerNotCanonicalRunAway,
    EntityPreflight(OrdinaryType9SelectedInitializerPreflightError),
}

/// Retryable rejection retaining the still-linear selected receipt.
#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9RunAwayInitializerFailureReceipt {
    pub error: OrdinaryType9RunAwayInitializerError,
    receipt: FreshLevel1Type9WeightedSelection,
}

impl OrdinaryType9RunAwayInitializerFailureReceipt {
    pub fn into_receipt(self) -> FreshLevel1Type9WeightedSelection {
        self.receipt
    }

    pub const fn receipt(&self) -> &FreshLevel1Type9WeightedSelection {
        &self.receipt
    }
}

#[derive(Debug)]
struct OrdinaryType9RunAwayPreflight {
    entity: OrdinaryType9SelectedInitializerPreflight,
    selected_evidence: OrdinaryType9RunAwaySelectionEvidence,
}

/// Publish one exact already-selected fresh-Level-1 Type-9 Run Away program.
///
/// `allocate` is called lazily for phase 0 and then phase 1. A failed phase is
/// a terminal successful outer-fallback publication, not a retryable error.
/// `next_constructor_word` is called only after a phase allocates: zero times
/// on phase-0 failure, once on phase-1 failure, and twice on full success.
pub fn apply_selected_ordinary_type9_run_away_initializer(
    entity: &mut Entity,
    receipt: FreshLevel1Type9WeightedSelection,
    metadata: &EntityTypeRuntimeMetadata,
    mut allocate: impl FnMut(RunAwayTaskPreparation) -> OrdinaryType9RunAwayAllocationDecision,
    mut next_constructor_word: impl FnMut() -> u32,
) -> Result<OrdinaryType9RunAwayInitializerOutcome, OrdinaryType9RunAwayInitializerFailureReceipt> {
    let preflight = match preflight_selected_run_away(entity, &receipt, metadata) {
        Ok(preflight) => preflight,
        Err(error) => {
            return Err(OrdinaryType9RunAwayInitializerFailureReceipt { error, receipt });
        }
    };

    // From this point every path is terminal. Publish the common selected
    // prefix and move construction/component custody out of the dormant label.
    let published = preflight.entity.publish(entity);

    // B6C0 copies only the authored common-axis +0x04 word, before Tertiary
    // is cleared. The strict-axis word remains actor-local.
    let RetailRuntimeValue::Known(mut actor_axis) = entity.actor_common_axis_descriptor else {
        unreachable!("preflight retained exact actor common-axis storage")
    };
    actor_axis.raw_word_at_0x04 = metadata
        .initializer
        .as_ref()
        .expect("exact Type-9 metadata includes an initializer")
        .common_axis_descriptor
        .raw_word_at_0x04;
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(actor_axis);

    let owner_position_raw = entity.position_raw();
    let transaction = {
        let Entity {
            actor_tasks,
            sub_a_propulsion_runtime,
            ..
        } = entity;
        let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
            unreachable!("preflight retained exact live Sub-A storage")
        };

        apply_ordinary_type9_run_away_acquiring_task_transaction_parts(
            owner_position_raw,
            actor_tasks,
            sub_a,
            metadata,
            &mut allocate,
            &mut next_constructor_word,
            |_| {},
        )
    };

    match transaction {
        OrdinaryType9RunAwayTaskTransactionOutcome::Published {
            constructors_by_phase,
        } => {
            published.finish_success(
                entity,
                OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished,
            );
            Ok(OrdinaryType9RunAwayInitializerOutcome::Published {
                selected: preflight.selected_evidence,
                constructors_by_phase,
            })
        }
        OrdinaryType9RunAwayTaskTransactionOutcome::AllocationFailed {
            failure,
            constructors_by_phase,
        } => {
            // Outer C6B0 consumes the nested error and terminally publishes
            // unnamed fallback policy plus S -> T -> P clears.
            published.finish_initializer_fallback(entity);
            Ok(
                OrdinaryType9RunAwayInitializerOutcome::InitializerFallbackPublished {
                    selected: preflight.selected_evidence,
                    failure,
                    constructors_by_phase,
                },
            )
        }
    }
}

/// Apply the shared Type-9 class-10 acquiring task transaction to already
/// borrowed live components.
///
/// Retail `FUN_0040B6C0` clears Tertiary before phase zero, publishes the
/// allocated Secondary before attempting phase one, and never consumes a
/// constructor word for a failed allocation. A later allocation failure does
/// not undo the earlier task or Sub-A suffix; the caller owns outer fallback.
pub(crate) fn apply_ordinary_type9_run_away_acquiring_task_transaction_parts(
    owner_position_raw: [i16; 3],
    actor_tasks: &mut ActorTaskOwner<ActorTaskRuntime>,
    sub_a: &mut SubAPropulsionRuntime,
    metadata: &EntityTypeRuntimeMetadata,
    mut allocate: impl FnMut(RunAwayTaskPreparation) -> OrdinaryType9RunAwayAllocationDecision,
    mut next_constructor_word: impl FnMut() -> u32,
    retire: impl FnMut(&ActorTaskRuntime),
) -> OrdinaryType9RunAwayTaskTransactionOutcome {
    let mut constructors_by_phase = [None; 2];
    let setup_result = apply_run_away_task_setup_with_retirement(
        actor_tasks,
        RunAwayTaskSetupRequest::Acquiring,
        |preparation| {
            if allocate(preparation) == OrdinaryType9RunAwayAllocationDecision::Failed {
                return Err(());
            }
            let phase_index = preparation.phase_index;
            let prepared =
                prepare_shared_acquiring_runtime_task(preparation, owner_position_raw, metadata, 0)
                    .expect("exact Type-9 metadata and B6C0 phase must prepare");
            debug_assert_eq!(
                prepared.topology(),
                SharedGenericConstructorTopology::SubAOnly
            );
            Ok(prepared.apply_suffix(&mut next_constructor_word, |effect| {
                apply_type9_constructor_effect(
                    effect,
                    sub_a,
                    &mut constructors_by_phase[phase_index],
                )
            }))
        },
        retire,
    );

    match setup_result {
        Ok(()) => OrdinaryType9RunAwayTaskTransactionOutcome::Published {
            constructors_by_phase: constructors_by_phase
                .map(|evidence| evidence.expect("successful B6C0 setup applies both suffixes")),
        },
        Err(error) => OrdinaryType9RunAwayTaskTransactionOutcome::AllocationFailed {
            failure: OrdinaryType9RunAwayInitializerFailure {
                phase_index: error.phase_index,
                slot: error.slot,
                role: error.role,
            },
            constructors_by_phase,
        },
    }
}

fn apply_type9_constructor_effect(
    effect: SharedGenericConstructorEffect,
    sub_a: &mut SubAPropulsionRuntime,
    evidence: &mut Option<OrdinaryType9RunAwayConstructorEvidence>,
) {
    match effect {
        SharedGenericConstructorEffect::WriteSubHState08 { .. } => {
            unreachable!("exact Type-9 topology has no Sub-H branch")
        }
        SharedGenericConstructorEffect::WriteSubADirection {
            direction_multiplier,
        } => sub_a.set_direction_multiplier(direction_multiplier),
        SharedGenericConstructorEffect::WriteSubATargetSpeed {
            target_speed_raw,
            random_sample_low16,
        } => {
            sub_a.apply_shared_initializer_target_speed_write(target_speed_raw);
            *evidence = Some(OrdinaryType9RunAwayConstructorEvidence {
                random_sample_low16,
                sub_a_target_speed_raw: target_speed_raw,
            });
        }
    }
}

fn preflight_selected_run_away(
    entity: &Entity,
    receipt: &FreshLevel1Type9WeightedSelection,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<OrdinaryType9RunAwayPreflight, OrdinaryType9RunAwayInitializerError> {
    let canonical_program = behavior_program(LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID)
        .expect("class-10 Run Away is statically audited");
    if receipt.initializer_identity() != FreshLevel1Type9InitializerIdentity::RunAwayAcquiring
        || receipt.selection().choice_index != 0
        || receipt.selection().program != canonical_program
    {
        return Err(OrdinaryType9RunAwayInitializerError::SelectedInitializerNotCanonicalRunAway);
    }
    let entity = preflight_fresh_level1_type9_selected_initializer(entity, receipt, metadata)
        .map_err(OrdinaryType9RunAwayInitializerError::EntityPreflight)?;
    Ok(OrdinaryType9RunAwayPreflight {
        entity,
        selected_evidence: OrdinaryType9SelectedEvidence::from_receipt(receipt),
    })
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;
    use crate::{
        actor_animation::ActorAnimationController,
        actor_task_dispatcher::ActorTaskRuntime,
        actor_task_owner::PreparedActorTask,
        common_mover::{
            shared_initializer_target_speed_raw, sub_d::ORDINARY_TYPE9_SUB_D,
            type9_attitude::Type9BodyBasis,
        },
        entity::{Entity, EntityKind},
        entity_behavior::{
            ActiveBehaviorStyle, BehaviorChoiceListSource, BehaviorDescriptorIdentity,
        },
        entity_collision_state::{EntityInitializerSpec, RetailStateWord},
        main_base_type9_abort::{
            MainBaseType9DeathComponentRuntime, LEVEL_ONE_TYPE9_BEHAVIOR_CHOICES,
            LEVEL_ONE_TYPE9_CAPABILITY_FLAGS, LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR,
            LEVEL_ONE_TYPE9_COMPONENT_TOPOLOGY, LEVEL_ONE_TYPE9_DEATH_BEHAVIOR_CLASS,
            LEVEL_ONE_TYPE9_DEATH_SOUND_ID, LEVEL_ONE_TYPE9_ENTITY_TYPE,
            LEVEL_ONE_TYPE9_INITIALIZER_STATE_RAW, LEVEL_ONE_TYPE9_INITIAL_HEALTH_RAW,
            LEVEL_ONE_TYPE9_MASS_RAW, LEVEL_ONE_TYPE9_MODEL_ID, LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR,
            LEVEL_ONE_TYPE9_SUB_B_DESCRIPTOR, LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR,
        },
        ordinary_type9_initial_selection::{
            plan_fresh_level1_type9_weighted_selection, FreshLevel1Type9EntityRef,
            FreshLevel1Type9WeightedSelectionRequest,
            LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK,
        },
        ordinary_type9_live::{
            admit_fresh_level1_ordinary_type9, FreshLevel1OrdinaryType9SpawnFacts,
            OrdinaryType9SelectedComponentRuntime,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_MASK,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE,
        },
        shared_retarget_mover::SharedRetargetTaskState,
    };

    const OWNER_ID: u32 = 0x04A9_0001;
    const SPAWN_INDEX: usize = 9;
    const IMMUTABLE_ANCHOR: [i16; 3] = [111, 22, -333];
    const OWNER_POSITION: [i16; 3] = IMMUTABLE_ANCHOR;

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [u16::try_from(LEVEL_ONE_TYPE9_MODEL_ID).unwrap(); 4],
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
        entity.set_motion_raw(OWNER_POSITION, [0; 3]);
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

    fn run_away_receipt(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> FreshLevel1Type9WeightedSelection {
        let baddie = FreshLevel1Type9EntityRef {
            id: 0x04AB_0001,
            entity_type: 99,
            position_raw: OWNER_POSITION,
            state_flags_raw: RetailStateWord::exact(1),
            capability_flags: RetailRuntimeValue::Known(
                LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK,
            ),
            attached_entity_handle: RetailRuntimeValue::Known(None),
        };
        let result = plan_fresh_level1_type9_weighted_selection(
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
        .expect("baddie-nearby random zero selects Run Away");
        assert_eq!(
            result.initializer_identity(),
            FreshLevel1Type9InitializerIdentity::RunAwayAcquiring
        );
        result
    }

    fn non_run_away_receipt(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> FreshLevel1Type9WeightedSelection {
        plan_fresh_level1_type9_weighted_selection(
            FreshLevel1Type9WeightedSelectionRequest {
                admission: admission(),
                authored_spawn_index: SPAWN_INDEX,
                active_model_id: usize::from(LEVEL_ONE_TYPE9_MODEL_ID),
                metadata,
                owner: owner_snapshot(entity),
                candidates_in_intrusive_order: &[],
            },
            || u32::MAX,
        )
        .expect("always branch remains eligible")
    }

    fn selected_runtime(entity: &Entity) -> OrdinaryType9SelectedComponentRuntime {
        entity
            .ordinary_type9_selected_component_runtime
            .expect("terminal initializer must retain selected component custody")
    }

    fn assert_fallback_context(entity: &Entity) {
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("fallback context must be published")
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
    }

    fn assert_outer_wrapper_still_pending(entity: &Entity, fixed_state_value: u32) {
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_MASK),
            RetailRuntimeValue::Known(fixed_state_value)
        );
        assert_eq!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Unresolved
        );
    }

    #[test]
    fn success_publishes_exact_t_s_p_program_with_two_lazy_suffix_words() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Event {
            Allocate(usize),
            Word(u32),
        }

        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let expected_components = entity
            .ordinary_type9_pending_initial_selection
            .expect("fixture custody");
        let receipt = run_away_receipt(&entity, &metadata);
        let events = RefCell::new(Vec::new());
        let mut words = [0x1234_D2F6, 0x5678_0985].into_iter();

        let outcome = apply_selected_ordinary_type9_run_away_initializer(
            &mut entity,
            receipt,
            &metadata,
            |preparation| {
                events
                    .borrow_mut()
                    .push(Event::Allocate(preparation.phase_index));
                OrdinaryType9RunAwayAllocationDecision::Prepared
            },
            || {
                let word = words.next().expect("exactly two constructor words");
                events.borrow_mut().push(Event::Word(word));
                word
            },
        )
        .unwrap();

        let OrdinaryType9RunAwayInitializerOutcome::Published {
            selected,
            constructors_by_phase,
        } = outcome
        else {
            panic!("expected selected publication: {outcome:?}")
        };
        assert_eq!(selected.owner_id, OWNER_ID);
        assert_eq!(selected.authored_spawn_index, SPAWN_INDEX);
        assert_eq!(selected.selection.choice_index, 0);
        assert_eq!(selected.selection.program.class_id, 10);
        assert_eq!(selected.selector_random_word, 0);
        assert_eq!(
            constructors_by_phase,
            [
                OrdinaryType9RunAwayConstructorEvidence {
                    random_sample_low16: 0xD2F6,
                    sub_a_target_speed_raw: shared_initializer_target_speed_raw(
                        LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw,
                        0xD2F6,
                    ),
                },
                OrdinaryType9RunAwayConstructorEvidence {
                    random_sample_low16: 0x0985,
                    sub_a_target_speed_raw: shared_initializer_target_speed_raw(
                        LEVEL_ONE_TYPE9_SUB_A_DESCRIPTOR.target_speed_base_raw,
                        0x0985,
                    ),
                },
            ]
        );
        assert_eq!(
            events.into_inner(),
            [
                Event::Allocate(0),
                Event::Word(0x1234_D2F6),
                Event::Allocate(1),
                Event::Word(0x5678_0985),
            ]
        );
        assert!(words.next().is_none());

        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::TargetAcquisition(_))
        ));
        let Some(ActorTaskRuntime::SharedRetarget(primary)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("phase one must publish the 500-ms Primary")
        };
        assert_eq!(primary.lifetime_ms(), 500);
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
        assert_eq!(
            entity.initial_behavior,
            RetailRuntimeValue::Known(Some(selected.selection))
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("selected context must be published")
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
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR)
        );
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("Sub-A must survive")
        };
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(constructors_by_phase[1].sub_a_target_speed_raw)
        );
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(sub_a.drive_scale_percent(), 100);
        assert_eq!(entity.ordinary_type9_pending_initial_selection, None);
        assert_eq!(selected_runtime(&entity).components(), expected_components);
        assert_eq!(
            selected_runtime(&entity).kind(),
            OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished
        );
        assert_outer_wrapper_still_pending(
            &entity,
            FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE,
        );
    }

    #[test]
    fn phase_zero_failure_uses_no_word_and_outer_fallback_discharges_custody() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let expected_components = entity.ordinary_type9_pending_initial_selection.unwrap();
        let receipt = run_away_receipt(&entity, &metadata);
        let allocations = Cell::new(0);
        let draws = Cell::new(0);

        let outcome = apply_selected_ordinary_type9_run_away_initializer(
            &mut entity,
            receipt,
            &metadata,
            |preparation| {
                assert_eq!(preparation.phase_index, 0);
                allocations.set(allocations.get() + 1);
                OrdinaryType9RunAwayAllocationDecision::Failed
            },
            || {
                draws.set(draws.get() + 1);
                panic!("failed phase zero must not consume a constructor word")
            },
        )
        .unwrap();

        let OrdinaryType9RunAwayInitializerOutcome::InitializerFallbackPublished {
            failure,
            constructors_by_phase,
            selected,
        } = outcome
        else {
            panic!("expected fallback: {outcome:?}")
        };
        assert_eq!(selected.selection.program.class_id, 10);
        assert_eq!(
            failure,
            OrdinaryType9RunAwayInitializerFailure {
                phase_index: 0,
                slot: ActorTaskSlot::Secondary,
                role: RunAwayTaskRole::AcquireTarget,
            }
        );
        assert_eq!(constructors_by_phase, [None, None]);
        assert_eq!((allocations.get(), draws.get()), (1, 0));
        for slot in [
            ActorTaskSlot::Secondary,
            ActorTaskSlot::Tertiary,
            ActorTaskSlot::Primary,
        ] {
            assert_eq!(entity.actor_task_state(slot), None);
        }
        assert_fallback_context(&entity);
        assert!(matches!(
            entity.initial_behavior,
            RetailRuntimeValue::Known(Some(selection)) if selection.program.class_id == 10
        ));
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!()
        };
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Unresolved);
        assert_eq!(entity.ordinary_type9_pending_initial_selection, None);
        assert_eq!(selected_runtime(&entity).components(), expected_components);
        assert_eq!(
            selected_runtime(&entity).kind(),
            OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
        );
        assert_outer_wrapper_still_pending(&entity, 0x0600_0801);
    }

    #[test]
    fn phase_one_failure_keeps_one_suffix_then_outer_fallback_clears_s_t_p() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let receipt = run_away_receipt(&entity, &metadata);
        let events = RefCell::new(Vec::new());

        let outcome = apply_selected_ordinary_type9_run_away_initializer(
            &mut entity,
            receipt,
            &metadata,
            |preparation| {
                events
                    .borrow_mut()
                    .push(("allocate", preparation.phase_index));
                if preparation.phase_index == 1 {
                    OrdinaryType9RunAwayAllocationDecision::Failed
                } else {
                    OrdinaryType9RunAwayAllocationDecision::Prepared
                }
            },
            || {
                events.borrow_mut().push(("word", 0));
                0xCAFE_FF00
            },
        )
        .unwrap();

        let OrdinaryType9RunAwayInitializerOutcome::InitializerFallbackPublished {
            failure,
            constructors_by_phase,
            ..
        } = outcome
        else {
            panic!("expected fallback: {outcome:?}")
        };
        assert_eq!(failure.phase_index, 1);
        assert_eq!(failure.slot, ActorTaskSlot::Primary);
        assert_eq!(failure.role, RunAwayTaskRole::Wander);
        assert_eq!(
            constructors_by_phase,
            [
                Some(OrdinaryType9RunAwayConstructorEvidence {
                    random_sample_low16: 0xFF00,
                    sub_a_target_speed_raw: 274,
                }),
                None,
            ]
        );
        assert_eq!(
            events.into_inner(),
            [("allocate", 0), ("word", 0), ("allocate", 1)]
        );
        for slot in [
            ActorTaskSlot::Secondary,
            ActorTaskSlot::Tertiary,
            ActorTaskSlot::Primary,
        ] {
            assert_eq!(entity.actor_task_state(slot), None);
        }
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!()
        };
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(274));
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_fallback_context(&entity);
        assert_eq!(entity.ordinary_type9_pending_initial_selection, None);
        assert_eq!(
            selected_runtime(&entity).kind(),
            OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
        );
        assert_outer_wrapper_still_pending(&entity, 0x0600_0801);
    }

    #[test]
    fn full_owner_snapshot_mismatch_is_atomic_for_every_selector_owned_field() {
        for mutation in 0..4 {
            let metadata = exact_metadata();
            let mut entity = exact_entity();
            let receipt = run_away_receipt(&entity, &metadata);
            match mutation {
                0 => entity.set_motion_raw([101, 20, -300], [0; 3]),
                1 => entity.collision.state_flags_at_0x08 = RetailStateWord::exact(3),
                2 => entity.capability_flags ^= 0x20,
                3 => {
                    entity.collision.recent_relation_id_at_0x60 =
                        RetailRuntimeValue::Known(Some(0x04AC_0001))
                }
                _ => unreachable!(),
            }
            let pending = entity.ordinary_type9_pending_initial_selection;
            let calls = Cell::new(0);
            let failure = apply_selected_ordinary_type9_run_away_initializer(
                &mut entity,
                receipt,
                &metadata,
                |_| {
                    calls.set(calls.get() + 1);
                    OrdinaryType9RunAwayAllocationDecision::Prepared
                },
                || {
                    calls.set(calls.get() + 1);
                    0
                },
            )
            .unwrap_err();
            assert!(matches!(
                failure.error,
                OrdinaryType9RunAwayInitializerError::EntityPreflight(
                    OrdinaryType9SelectedInitializerPreflightError::OwnerSnapshotMismatch { .. }
                )
            ));
            assert_eq!(calls.get(), 0);
            assert_eq!(entity.ordinary_type9_pending_initial_selection, pending);
            assert_eq!(entity.ordinary_type9_selected_component_runtime, None);
            assert_eq!(entity.initial_behavior, RetailRuntimeValue::Unresolved);
            assert_eq!(
                entity.current_behavior_context,
                RetailRuntimeValue::Unresolved
            );
        }
    }

    #[test]
    fn retryable_stale_receipt_can_succeed_only_after_exact_entity_is_restored() {
        let metadata = exact_metadata();
        let mut entity = exact_entity();
        let receipt = run_away_receipt(&entity, &metadata);
        entity.set_motion_raw([101, 20, -300], [0; 3]);
        let failure = apply_selected_ordinary_type9_run_away_initializer(
            &mut entity,
            receipt,
            &metadata,
            |_| panic!("stale receipt must not allocate"),
            || panic!("stale receipt must not consume a word"),
        )
        .unwrap_err();
        entity.set_motion_raw(OWNER_POSITION, [0; 3]);

        let outcome = apply_selected_ordinary_type9_run_away_initializer(
            &mut entity,
            failure.into_receipt(),
            &metadata,
            |_| OrdinaryType9RunAwayAllocationDecision::Prepared,
            || 0,
        )
        .unwrap();
        assert!(matches!(
            outcome,
            OrdinaryType9RunAwayInitializerOutcome::Published { .. }
        ));
    }

    #[test]
    fn every_terminal_entity_rejects_an_independently_planned_receipt_without_calls() {
        for fail_phase_zero in [false, true] {
            let metadata = exact_metadata();
            let mut entity = exact_entity();
            let first = run_away_receipt(&entity, &metadata);
            let replay = run_away_receipt(&entity, &metadata);
            apply_selected_ordinary_type9_run_away_initializer(
                &mut entity,
                first,
                &metadata,
                |preparation| {
                    if fail_phase_zero && preparation.phase_index == 0 {
                        OrdinaryType9RunAwayAllocationDecision::Failed
                    } else {
                        OrdinaryType9RunAwayAllocationDecision::Prepared
                    }
                },
                || 0,
            )
            .unwrap();
            let calls = Cell::new(0);
            let failure = apply_selected_ordinary_type9_run_away_initializer(
                &mut entity,
                replay,
                &metadata,
                |_| {
                    calls.set(calls.get() + 1);
                    OrdinaryType9RunAwayAllocationDecision::Prepared
                },
                || {
                    calls.set(calls.get() + 1);
                    0
                },
            )
            .unwrap_err();
            assert_eq!(
                failure.error,
                OrdinaryType9RunAwayInitializerError::EntityPreflight(
                    OrdinaryType9SelectedInitializerPreflightError::SelectedComponentCustodyAlreadyPresent
                )
            );
            assert_eq!(calls.get(), 0);
        }
    }

    #[test]
    fn non_run_away_and_corrupt_basis_axis_task_metadata_or_custody_fail_atomically() {
        fn assert_blocked(
            mut entity: Entity,
            receipt: FreshLevel1Type9WeightedSelection,
            metadata: &EntityTypeRuntimeMetadata,
            expected: OrdinaryType9RunAwayInitializerError,
        ) {
            let pending = entity.ordinary_type9_pending_initial_selection;
            let calls = Cell::new(0);
            let failure = apply_selected_ordinary_type9_run_away_initializer(
                &mut entity,
                receipt,
                metadata,
                |_| {
                    calls.set(calls.get() + 1);
                    OrdinaryType9RunAwayAllocationDecision::Prepared
                },
                || {
                    calls.set(calls.get() + 1);
                    0
                },
            )
            .unwrap_err();
            assert_eq!(failure.error, expected);
            assert_eq!(calls.get(), 0);
            assert_eq!(entity.ordinary_type9_pending_initial_selection, pending);
            assert_eq!(entity.initial_behavior, RetailRuntimeValue::Unresolved);
            assert_eq!(
                entity.current_behavior_context,
                RetailRuntimeValue::Unresolved
            );
        }

        let metadata = exact_metadata();
        let entity = exact_entity();
        let receipt = non_run_away_receipt(&entity, &metadata);
        assert_blocked(
            entity,
            receipt,
            &metadata,
            OrdinaryType9RunAwayInitializerError::SelectedInitializerNotCanonicalRunAway,
        );

        let mut entity = exact_entity();
        let receipt = run_away_receipt(&entity, &metadata);
        entity.physical_body_basis_q31 =
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0x4000, 0, 0));
        assert_blocked(
            entity,
            receipt,
            &metadata,
            OrdinaryType9RunAwayInitializerError::EntityPreflight(
                OrdinaryType9SelectedInitializerPreflightError::UnexpectedPhysicalBodyBasis,
            ),
        );

        let mut entity = exact_entity();
        let receipt = run_away_receipt(&entity, &metadata);
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Unresolved;
        assert_blocked(
            entity,
            receipt,
            &metadata,
            OrdinaryType9RunAwayInitializerError::EntityPreflight(
                OrdinaryType9SelectedInitializerPreflightError::UnexpectedActorCommonAxis,
            ),
        );

        let mut entity = exact_entity();
        let receipt = run_away_receipt(&entity, &metadata);
        entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                SharedRetargetTaskState::new(OWNER_POSITION, 500),
            )),
        );
        assert_blocked(
            entity,
            receipt,
            &metadata,
            OrdinaryType9RunAwayInitializerError::EntityPreflight(
                OrdinaryType9SelectedInitializerPreflightError::BirthTaskTableNotEmpty,
            ),
        );

        let mut entity = exact_entity();
        let receipt = run_away_receipt(&entity, &metadata);
        entity.ordinary_type9_pending_initial_selection = Some(
            admit_fresh_level1_ordinary_type9(FreshLevel1OrdinaryType9SpawnFacts {
                authored_spawn_index: 10,
                ..FreshLevel1OrdinaryType9SpawnFacts {
                    retail_first_world: true,
                    authored_spawn_index: SPAWN_INDEX,
                    entity_type: LEVEL_ONE_TYPE9_ENTITY_TYPE,
                    active_model_slot: RetailRuntimeValue::Known(0),
                    active_model: Some(usize::from(LEVEL_ONE_TYPE9_MODEL_ID)),
                    rotation: [0; 3],
                    immutable_anchor_raw_at_0x90: RetailRuntimeValue::Known(IMMUTABLE_ANCHOR),
                }
            })
            .unwrap()
            .pending_initial_selection(),
        );
        assert_blocked(
            entity,
            receipt,
            &metadata,
            OrdinaryType9RunAwayInitializerError::EntityPreflight(
                OrdinaryType9SelectedInitializerPreflightError::PendingComponentCustodyMismatch,
            ),
        );

        let mut wrong_metadata = metadata.clone();
        wrong_metadata.mass_raw += 1;
        let entity = exact_entity();
        let receipt = run_away_receipt(&entity, &metadata);
        assert_blocked(
            entity,
            receipt,
            &wrong_metadata,
            OrdinaryType9RunAwayInitializerError::EntityPreflight(
                OrdinaryType9SelectedInitializerPreflightError::MetadataNotExact,
            ),
        );
    }

    #[test]
    fn selected_and_death_custody_are_mutually_exclusive_before_mutation() {
        let metadata = exact_metadata();

        let mut entity = exact_entity();
        let receipt = run_away_receipt(&entity, &metadata);
        let components = entity.ordinary_type9_pending_initial_selection.unwrap();
        entity.ordinary_type9_selected_component_runtime =
            Some(OrdinaryType9SelectedComponentRuntime::new(
                components,
                OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished,
            ));
        let failure = apply_selected_ordinary_type9_run_away_initializer(
            &mut entity,
            receipt,
            &metadata,
            |_| panic!(),
            || panic!(),
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9RunAwayInitializerError::EntityPreflight(
                OrdinaryType9SelectedInitializerPreflightError::SelectedComponentCustodyAlreadyPresent
            )
        );

        let mut entity = exact_entity();
        let receipt = run_away_receipt(&entity, &metadata);
        let components = entity.ordinary_type9_pending_initial_selection.unwrap();
        entity.main_base_type9_death_component_runtime =
            Some(MainBaseType9DeathComponentRuntime { components });
        let failure = apply_selected_ordinary_type9_run_away_initializer(
            &mut entity,
            receipt,
            &metadata,
            |_| panic!(),
            || panic!(),
        )
        .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9RunAwayInitializerError::EntityPreflight(
                OrdinaryType9SelectedInitializerPreflightError::DeathComponentCustodyAlreadyPresent
            )
        );
    }
}
