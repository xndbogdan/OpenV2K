//! Synchronous, receipt-journaled Level-1 type-17 projectile hit owner.
//!
//! Retail `FUN_00410EB0` stamps the target, invokes its current impact style,
//! applies `FUN_00411030`, drives checked damage through `FUN_00415040`, and
//! only then emits its cached accepted-hit suffix.  The full-game/demo C is
//! instruction-equivalent across that chain.  This adapter composes the
//! existing detached machines for only the four captured fresh post-Intro
//! Level-1 type-17/model-256 allocations.
//!
//! The adapter is deliberately synchronous.  Every issued receipt is entered
//! in a small action journal before its external effect and retired only after
//! the exact machine accepts the completion.  A later failure preserves all
//! prior entity, task, RNG, audio, notification, and particle mutations and is
//! returned as a committed-prefix block; the transaction is never replayed.
//!
//! Fresh post-Intro construction now authenticates the exact Level-1 birth
//! surface, evaluates the prefix-only type-default choices, and publishes the
//! selected initializer/context/task graph before linkage.  This owner still
//! rejects direct starts and any missing or stale context rather than
//! manufacturing Follow Beacons or Capture People state at impact time.

use v2k_formats::models::CollisionModelPool;

use crate::{
    actor_standard_death_live::{
        publish_type17_common_dying_from_checked_death, Type17CommonDyingPublicationError,
        Type17CommonDyingPublicationOutcome, Type17CommonDyingPublicationRequest,
        TYPE17_STANDARD_DEATH_CALLBACK_ADDRESS,
    },
    checked_damage::{
        CachedTargetWriteObservation, CheckedDamageAction, CheckedDamageAdmissionTarget,
        CheckedDamageBlock, CheckedDamageExternalBlock, CheckedDamageFilterBinding,
        CheckedDamagePhase, CheckedDamageResume, DeathAttachmentObservation,
        DeathCallbackObservation, DeathEntryObservation, DeathEntryTarget,
        DeathNetworkTailObservation, DeathSoundObservation, DeathTargetAfterCallbackObservation,
        GenericDamageEntryObservation, GenericDamageEntryTarget, GenericHealthObservation,
        GenericHitCallbackObservation, GenericHitSoundObservation, GenericTargetStateSample,
        OriginalTargetAfterDeathObservation, SurvivorNetworkTailObservation,
        PLAYER_KILL_FEEDBACK_SELECTOR,
    },
    common_dying_live::{
        FRESH_LEVEL1_TYPE17_COMMON_DYING_SPAWN_INDICES, TYPE17_COMMON_DYING_ENTITY_TYPE,
        TYPE17_COMMON_DYING_MODEL_ID,
    },
    damage::DamageDeliveryRecord,
    entity::{Entity, EntityManager},
    entity_collision_state::{
        active_model_slot_from_state_flags, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
    gameplay_notifications::GameplayNotifications,
    impact_reaction::{
        apply_impact_reaction, ImpactReactionBody, ImpactReactionOutcome,
        IMPACT_REACTION_NETWORKED_STATE_BIT,
    },
    primary_hit::{
        IssuedPrimaryHitAction, PrimaryHitAction, PrimaryHitBlock, PrimaryHitCachedEntry,
        PrimaryHitCachedTargetSnapshot, PrimaryHitCompletion, PrimaryHitImpactReactionResult,
        PrimaryHitMachine, PrimaryHitModelBinding, PrimaryHitPhase, PrimaryHitPoll,
        PrimaryHitRequest, PrimaryHitResume, PrimaryHitTransactionId,
        PRIMARY_HIT_TYPE_CALLBACK_ADDRESS,
    },
    primary_hit_checked_damage::{
        PrimaryHitCheckedDamageCoordinator, PrimaryHitCheckedDamageEntry,
        PrimaryHitCheckedDamagePoll,
    },
    type17_impact_live::{
        apply_type17_impact_reselection_live, Type17ImpactLiveError, Type17ImpactLiveOutcome,
        Type17ImpactLiveRequest,
    },
    type17_impact_reselection::{Type17ImpactEntityRef, Type17ImpactReselectionBoundary},
    world_fx::WorldFx,
};

const ALLOCATION_IDENTITY_DOMAIN: u64 = 0x1700_0000_0000_0000;
const TYPE_RECORD_IDENTITY_DOMAIN: u64 = 0x1710_0000_0000_0000;
const PARENT_TRANSACTION_DOMAIN: u64 = 0x1720_0000_0000_0000;
const CHILD_TRANSACTION_DOMAIN: u64 = 0x1730_0000_0000_0000;

/// Complete immutable arguments retained from one F590 entity-impact event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelOneType17ProjectilePrimaryHitRequest {
    pub target_entity_id: u32,
    pub delivery: DamageDeliveryRecord,
    /// Exact opaque `particle + 8` token supplied to `FUN_00410EB0`.
    pub source_provenance: u32,
    /// Particle velocity words passed as the reaction direction.
    pub impact_direction_q15: [i16; 3],
    pub retail_tick: u32,
}

/// Evidence which must be complete before the unconditional hit-tick write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LevelOneType17PrimaryHitPreflightError {
    NotFreshNewGameFirstWorld,
    EntityInactive,
    UnauthenticatedSpawn { actual: Option<usize> },
    UnexpectedModelSlots { actual: [Option<usize>; 4] },
    CurrentBehaviorContextUnresolved,
    CurrentBehaviorContextAbsent,
    StateWordNotExact { known_mask: u32 },
    ImpactReactionWouldRequestNetwork,
    ZeroMass,
    RuntimeFieldUnresolved(&'static str),
    RuntimeCallbackMismatch(&'static str),
    MissingTypeMetadata,
    MissingPlayerType,
    PlayerTypeOutOfRange { actual: u32 },
    MissingModel { global_model_id: usize },
    ModelIdOutOfRange { global_model_id: usize },
}

/// Exact point at which a synchronous transaction can no longer continue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelOneType17PrimaryHitPhase {
    Primary(PrimaryHitPhase),
    CheckedDamage(CheckedDamagePhase),
}

/// Runtime/adaptor reason for preserving a committed prefix and stopping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LevelOneType17PrimaryHitBlockReason {
    PrimaryMachine(PrimaryHitBlock),
    CheckedDamageMachine(CheckedDamageBlock),
    TargetUnavailable,
    TargetIdentityChanged,
    StateWordBecameUnresolved,
    RuntimeValueChanged(&'static str),
    TypeImpact(Type17ImpactLiveError),
    CapturePeopleCleanup(Type17ImpactReselectionBoundary),
    CommonDying(Type17CommonDyingPublicationError),
    UnsupportedCheckedAction(&'static str),
}

/// Successful same-frame composition evidence returned to its live caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelOneType17PrimaryHitCompletion {
    pub primary: PrimaryHitCompletion,
    pub impact_reselection: Type17ImpactLiveOutcome,
    pub impact_reaction: ImpactReactionOutcome,
    pub common_dying: Option<Type17CommonDyingPublicationOutcome>,
    pub player_kill_feedback_queued: bool,
}

/// Terminal result of one bounded live attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LevelOneType17PrimaryHitOutcome {
    /// Missing targets and every non-type-17 target remain available to the
    /// independently audited Base/Factory bridge.
    NotApplicable,
    /// No retail-visible mutation occurred.
    PreflightRejected(LevelOneType17PrimaryHitPreflightError),
    Complete(LevelOneType17PrimaryHitCompletion),
    /// At least the hit-tick stamp was committed.  The caller must not retry
    /// this projectile event as a fresh transaction.
    CommittedPrefixBlocked {
        phase: LevelOneType17PrimaryHitPhase,
        reason: LevelOneType17PrimaryHitBlockReason,
    },
}

#[derive(Debug, Clone)]
struct Type17PrimaryHitPreflight {
    metadata: EntityTypeRuntimeMetadata,
    target_allocation_identity: u64,
    type_record_identity: u64,
    local_player_type: u8,
    /// Exact fresh type-17 death-helper resource slot at entity `+0x8C`.
    /// This is distinct from the cargo/parent relation in `Entity::attached_to`.
    death_attachment_resource_identity: RetailRuntimeValue<Option<u64>>,
    model_by_state_slot: [RetailRuntimeValue<Option<PrimaryHitModelBinding>>; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct JournalKey {
    transaction_id: u64,
    action_sequence: u64,
}

#[derive(Debug, Default)]
struct SynchronousActionJournal {
    /// Nested action stack. Checked-damage receipts sit above the retained
    /// parent delivery receipt until the coordinator resumes or blocks it.
    active: Vec<JournalKey>,
    completed: Vec<JournalKey>,
}

impl SynchronousActionJournal {
    fn begin(&mut self, key: JournalKey) {
        assert!(
            !self.completed.contains(&key) && !self.active.contains(&key),
            "a receipt-bound action was about to be replayed"
        );
        self.active.push(key);
    }

    fn complete(&mut self, key: JournalKey) {
        assert_eq!(self.active.pop(), Some(key));
        self.completed.push(key);
    }
}

/// Compose the exact local type-17 primary-hit transaction through class-12
/// publication and the cached class-5 suffix.
pub fn apply_level_one_type17_projectile_primary_hit(
    manager: &mut EntityManager,
    models: &impl CollisionModelPool,
    world_fx: &mut WorldFx,
    notifications: &mut GameplayNotifications,
    request: LevelOneType17ProjectilePrimaryHitRequest,
) -> LevelOneType17PrimaryHitOutcome {
    let preflight = match preflight_type17_primary_hit(manager, models, request.target_entity_id) {
        Ok(Some(preflight)) => preflight,
        Ok(None) => return LevelOneType17PrimaryHitOutcome::NotApplicable,
        Err(error) => return LevelOneType17PrimaryHitOutcome::PreflightRejected(error),
    };

    // This owner holds both mutable world stores for the complete call, so no
    // second primary-hit machine can overlap it.  Domain-separated per-target
    // IDs remain unique for the lifetime of these synchronous machines.
    let parent_transaction_id = PrimaryHitTransactionId::new(
        PARENT_TRANSACTION_DOMAIN | (u64::from(request.target_entity_id) + 1),
    )
    .expect("domain-tagged parent transaction id is nonzero");
    let child_transaction_id = crate::checked_damage::CheckedDamageTransactionId::new(
        CHILD_TRANSACTION_DOMAIN | (u64::from(request.target_entity_id) + 1),
    )
    .expect("domain-tagged child transaction id is nonzero");
    let mut parent = Some(PrimaryHitMachine::start(
        parent_transaction_id,
        PrimaryHitRequest {
            target_handle: request.target_entity_id,
            delivery: request.delivery,
            source_provenance: request.source_provenance,
            direction_q15: request.impact_direction_q15,
            retail_tick: request.retail_tick,
            cached_entry: RetailRuntimeValue::Known(Some(PrimaryHitCachedEntry {
                target_allocation_identity: preflight.target_allocation_identity,
                type_record_identity: preflight.type_record_identity,
                type_callback_address: RetailRuntimeValue::Known(Some(
                    PRIMARY_HIT_TYPE_CALLBACK_ADDRESS,
                )),
            })),
        },
    ));
    let mut journal = SynchronousActionJournal::default();
    let mut impact_reselection = None;
    let mut impact_reaction = None;
    let mut last_primary_phase = PrimaryHitPhase::CommitTickStamp;

    let (parent_machine, checked_delivery) = loop {
        let machine = parent
            .as_mut()
            .expect("parent is retained before child start");
        match machine.poll() {
            PrimaryHitPoll::Action(issued) => {
                let phase = issued.action.phase();
                last_primary_phase = phase;
                let key = primary_journal_key(&issued);
                journal.begin(key);
                match issued.action {
                    PrimaryHitAction::CommitTickStamp {
                        target_handle,
                        target_allocation_identity,
                        retail_tick,
                        ..
                    } => {
                        let result = commit_hit_tick(
                            manager,
                            target_handle,
                            target_allocation_identity,
                            retail_tick,
                        );
                        if let Err(reason) = result {
                            block_parent(machine, issued, &mut journal, key);
                            return committed_primary_block(phase, reason);
                        }
                        acknowledge_parent(machine, issued, &mut journal, key);
                    }
                    PrimaryHitAction::InvokeTypeImpactCallback {
                        callback_address,
                        target_handle,
                        ..
                    } => {
                        debug_assert_eq!(callback_address, PRIMARY_HIT_TYPE_CALLBACK_ADDRESS);
                        let candidates = type17_candidate_snapshot(manager);
                        let result = manager
                            .type17_primary_hit_entity_mut(target_handle)
                            .ok_or(LevelOneType17PrimaryHitBlockReason::TargetUnavailable)
                            .and_then(|entity| {
                                apply_type17_impact_reselection_live(
                                    entity,
                                    Type17ImpactLiveRequest {
                                        metadata: &preflight.metadata,
                                        candidates_in_intrusive_order: &candidates,
                                        current_tick: request.retail_tick,
                                    },
                                    world_fx,
                                )
                                .map_err(LevelOneType17PrimaryHitBlockReason::TypeImpact)
                            });
                        match result {
                            Ok(Type17ImpactLiveOutcome::Boundary(
                                boundary @ Type17ImpactReselectionBoundary::CapturePeopleCleanup {
                                    ..
                                },
                            )) => {
                                block_parent(machine, issued, &mut journal, key);
                                return committed_primary_block(
                                    phase,
                                    LevelOneType17PrimaryHitBlockReason::CapturePeopleCleanup(
                                        boundary,
                                    ),
                                );
                            }
                            Ok(outcome) => {
                                impact_reselection = Some(outcome);
                                acknowledge_parent(machine, issued, &mut journal, key);
                            }
                            Err(reason) => {
                                block_parent(machine, issued, &mut journal, key);
                                return committed_primary_block(phase, reason);
                            }
                        }
                    }
                    PrimaryHitAction::ApplyImpactReaction {
                        target_handle,
                        impact_sum_raw,
                        direction_q15,
                        ..
                    } => {
                        let result = apply_live_impact_reaction(
                            manager,
                            world_fx,
                            target_handle,
                            preflight.target_allocation_identity,
                            impact_sum_raw,
                            direction_q15,
                        );
                        match result {
                            Ok(outcome) => {
                                impact_reaction = Some(outcome);
                                resume_parent(
                                    machine,
                                    issued,
                                    PrimaryHitResume::ImpactReactionResolved {
                                        phase,
                                        result: PrimaryHitImpactReactionResult::Committed(outcome),
                                    },
                                    &mut journal,
                                    key,
                                );
                            }
                            Err(reason) => {
                                block_parent(machine, issued, &mut journal, key);
                                return committed_primary_block(phase, reason);
                            }
                        }
                    }
                    PrimaryHitAction::SubmitImpactReactionNetwork { .. } => {
                        block_parent(machine, issued, &mut journal, key);
                        return committed_primary_block(
                            phase,
                            LevelOneType17PrimaryHitBlockReason::RuntimeValueChanged(
                                "impact-reaction network predicate",
                            ),
                        );
                    }
                    PrimaryHitAction::DeliverCheckedDamage { .. } => {
                        break (
                            parent
                                .take()
                                .expect("parent moves into checked coordinator"),
                            issued,
                        );
                    }
                    _ => unreachable!("accepted suffix cannot precede checked damage"),
                }
            }
            PrimaryHitPoll::Blocked(block) => {
                return committed_primary_block(
                    last_primary_phase,
                    LevelOneType17PrimaryHitBlockReason::PrimaryMachine(block),
                );
            }
            PrimaryHitPoll::Awaiting(phase) => {
                panic!("synchronous parent left an unretired receipt at {phase:?}")
            }
            PrimaryHitPoll::Complete(completion) => {
                panic!("primary hit completed before checked delivery: {completion:?}")
            }
        }
    };

    let checked_entry =
        match sample_checked_damage_entry(manager, &preflight, request.target_entity_id) {
            Ok(entry) => entry,
            Err(reason) => {
                let mut parent_machine = parent_machine;
                // The issued delivery receipt was journaled but not yet accepted.
                let key = primary_journal_key(&checked_delivery);
                block_parent(&mut parent_machine, checked_delivery, &mut journal, key);
                return committed_primary_block(PrimaryHitPhase::CheckedDamageDelivery, reason);
            }
        };
    let checked_delivery_key = primary_journal_key(&checked_delivery);
    let mut coordinator = PrimaryHitCheckedDamageCoordinator::start(
        parent_machine,
        checked_delivery,
        child_transaction_id,
        checked_entry,
    )
    .expect("the live owner passes the exact issued checked-delivery receipt");
    let mut common_dying = None;
    let mut player_kill_feedback_queued = false;
    let mut last_checked_phase = CheckedDamagePhase::ResolveGenericDamageEntry;

    loop {
        match coordinator.poll() {
            PrimaryHitCheckedDamagePoll::Action(issued) => {
                let phase = issued.action.phase();
                last_checked_phase = phase;
                let key = checked_journal_key(&issued);
                journal.begin(key);
                let action_result = drive_checked_action(
                    &mut coordinator,
                    issued,
                    &preflight,
                    manager,
                    world_fx,
                    notifications,
                    request.retail_tick,
                    &mut common_dying,
                    &mut player_kill_feedback_queued,
                    &mut journal,
                    key,
                );
                if let Err(reason) = action_result {
                    match coordinator.poll() {
                        PrimaryHitCheckedDamagePoll::Blocked(_) => {}
                        other => panic!(
                            "blocked child did not retire its retained parent receipt: {other:?}"
                        ),
                    }
                    journal.complete(checked_delivery_key);
                    return LevelOneType17PrimaryHitOutcome::CommittedPrefixBlocked {
                        phase: LevelOneType17PrimaryHitPhase::CheckedDamage(phase),
                        reason,
                    };
                }
            }
            PrimaryHitCheckedDamagePoll::ParentResumed => {
                journal.complete(checked_delivery_key);
                break;
            }
            PrimaryHitCheckedDamagePoll::Blocked(block) => {
                journal.complete(checked_delivery_key);
                return LevelOneType17PrimaryHitOutcome::CommittedPrefixBlocked {
                    phase: LevelOneType17PrimaryHitPhase::CheckedDamage(last_checked_phase),
                    reason: LevelOneType17PrimaryHitBlockReason::CheckedDamageMachine(block),
                };
            }
            PrimaryHitCheckedDamagePoll::Awaiting(phase) => {
                panic!("synchronous checked owner left an unretired receipt at {phase:?}")
            }
        }
    }

    let mut parent = coordinator
        .into_parent()
        .expect("completed child returns its parent machine");
    let primary_completion = loop {
        match parent.poll() {
            PrimaryHitPoll::Action(issued) => {
                let phase = issued.action.phase();
                last_primary_phase = phase;
                let key = primary_journal_key(&issued);
                journal.begin(key);
                match issued.action {
                    PrimaryHitAction::SampleCachedOuterTarget { .. } => {
                        let sample = sample_cached_outer_target(
                            manager,
                            &preflight,
                            request.target_entity_id,
                        );
                        resume_parent(
                            &mut parent,
                            issued,
                            PrimaryHitResume::CachedOuterTargetSampled {
                                phase,
                                cached_outer_target: sample,
                            },
                            &mut journal,
                            key,
                        );
                    }
                    PrimaryHitAction::PlayAcceptedHitSound {
                        sound_id,
                        position_raw,
                        ..
                    } => {
                        world_fx.queue_fixed_positional_sound_raw(sound_id, position_raw);
                        acknowledge_parent(&mut parent, issued, &mut journal, key);
                    }
                    PrimaryHitAction::EmitCapabilityFollowUp { emission, .. } => {
                        world_fx.emit_primary_hit_capability_follow_up_raw(emission);
                        acknowledge_parent(&mut parent, issued, &mut journal, key);
                    }
                    _ => unreachable!("checked child already consumed the prefix"),
                }
            }
            PrimaryHitPoll::Complete(completion) => break completion,
            PrimaryHitPoll::Blocked(block) => {
                return committed_primary_block(
                    last_primary_phase,
                    LevelOneType17PrimaryHitBlockReason::PrimaryMachine(block),
                );
            }
            PrimaryHitPoll::Awaiting(phase) => {
                panic!("synchronous suffix left an unretired receipt at {phase:?}")
            }
        }
    };

    LevelOneType17PrimaryHitOutcome::Complete(LevelOneType17PrimaryHitCompletion {
        primary: primary_completion,
        impact_reselection: impact_reselection
            .expect("authenticated type-17 callback always yields one supported outcome"),
        impact_reaction: impact_reaction
            .expect("checked delivery follows one completed reaction helper"),
        common_dying,
        player_kill_feedback_queued,
    })
}

fn preflight_type17_primary_hit(
    manager: &EntityManager,
    models: &impl CollisionModelPool,
    target_entity_id: u32,
) -> Result<Option<Type17PrimaryHitPreflight>, LevelOneType17PrimaryHitPreflightError> {
    let Some(entity) = manager
        .iter_all()
        .find(|entity| entity.id == target_entity_id)
    else {
        return Ok(None);
    };
    if entity.entity_type != TYPE17_COMMON_DYING_ENTITY_TYPE {
        return Ok(None);
    }
    if !manager.is_fresh_new_game_first_world() {
        return Err(LevelOneType17PrimaryHitPreflightError::NotFreshNewGameFirstWorld);
    }
    if !entity.active {
        return Err(LevelOneType17PrimaryHitPreflightError::EntityInactive);
    }
    if !entity
        .authored_spawn_index
        .is_some_and(|spawn| FRESH_LEVEL1_TYPE17_COMMON_DYING_SPAWN_INDICES.contains(&spawn))
    {
        return Err(
            LevelOneType17PrimaryHitPreflightError::UnauthenticatedSpawn {
                actual: entity.authored_spawn_index,
            },
        );
    }
    let expected_slots = [Some(TYPE17_COMMON_DYING_MODEL_ID); 4];
    if entity.model_slots != expected_slots {
        return Err(
            LevelOneType17PrimaryHitPreflightError::UnexpectedModelSlots {
                actual: entity.model_slots,
            },
        );
    }
    match entity.current_behavior_context {
        RetailRuntimeValue::Unresolved => {
            return Err(LevelOneType17PrimaryHitPreflightError::CurrentBehaviorContextUnresolved)
        }
        RetailRuntimeValue::Known(None) => {
            return Err(LevelOneType17PrimaryHitPreflightError::CurrentBehaviorContextAbsent)
        }
        RetailRuntimeValue::Known(Some(_)) => {}
    }
    let state = exact_state(entity).map_err(|known_mask| {
        LevelOneType17PrimaryHitPreflightError::StateWordNotExact { known_mask }
    })?;
    if state & IMPACT_REACTION_NETWORKED_STATE_BIT != 0 {
        return Err(LevelOneType17PrimaryHitPreflightError::ImpactReactionWouldRequestNetwork);
    }
    if entity.mass_raw == 0 {
        return Err(LevelOneType17PrimaryHitPreflightError::ZeroMass);
    }
    require_known(entity.collision.health_raw, "health")?;
    require_known(
        entity.collision.pre_health_damage_buffer_raw,
        "pre-health buffer",
    )?;
    require_known(entity.collision.damage_profile, "damage profile")?;
    require_known(
        entity.collision.last_hit_presentation_tick_at_0x34,
        "last-hit tick",
    )?;
    require_known(
        entity.collision.accepted_hit_presentation_sound_id,
        "accepted-hit sound",
    )?;
    require_known(entity.collision.death_sound_id, "death sound")?;
    require_known(entity.collision.generic_hit_sound_id, "generic-hit sound")?;
    if entity.collision.pair_callbacks.damage_modifier_address != RetailRuntimeValue::Known(None) {
        return Err(
            LevelOneType17PrimaryHitPreflightError::RuntimeCallbackMismatch("damage modifier"),
        );
    }
    if entity.collision.pair_callbacks.type_hit_callback_address != RetailRuntimeValue::Known(None)
    {
        return Err(
            LevelOneType17PrimaryHitPreflightError::RuntimeCallbackMismatch("generic hit callback"),
        );
    }

    let metadata = manager
        .type_runtime_metadata(TYPE17_COMMON_DYING_ENTITY_TYPE)
        .cloned()
        .ok_or(LevelOneType17PrimaryHitPreflightError::MissingTypeMetadata)?;
    let player_type = manager
        .player()
        .ok_or(LevelOneType17PrimaryHitPreflightError::MissingPlayerType)?
        .entity_type;
    let local_player_type = u8::try_from(player_type).map_err(|_| {
        LevelOneType17PrimaryHitPreflightError::PlayerTypeOutOfRange {
            actual: player_type,
        }
    })?;
    let mut model_by_state_slot = [RetailRuntimeValue::Unresolved; 4];
    for (slot, model_id) in entity.model_slots.into_iter().enumerate() {
        let global_model_id = model_id.expect("exact model slots were authenticated above");
        let global_model_id_u16 = u16::try_from(global_model_id).map_err(|_| {
            LevelOneType17PrimaryHitPreflightError::ModelIdOutOfRange { global_model_id }
        })?;
        let model = models
            .collision_model(global_model_id)
            .ok_or(LevelOneType17PrimaryHitPreflightError::MissingModel { global_model_id })?;
        model_by_state_slot[slot] = RetailRuntimeValue::Known(Some(PrimaryHitModelBinding {
            global_model_id: global_model_id_u16,
            base_z_raw: model.radius as i16,
        }));
    }

    Ok(Some(Type17PrimaryHitPreflight {
        metadata,
        target_allocation_identity: allocation_identity(entity.id),
        type_record_identity: type_record_identity(entity.entity_type),
        local_player_type,
        // Fresh constructor/static C plus the accepted Level-1 type-17 death
        // oracle establish this separate +0x8C slot as null for the bounded
        // allocation family. Never substitute the cargo parent relation.
        death_attachment_resource_identity: RetailRuntimeValue::Known(None),
        model_by_state_slot,
    }))
}

fn require_known<T: Copy>(
    value: RetailRuntimeValue<T>,
    field: &'static str,
) -> Result<T, LevelOneType17PrimaryHitPreflightError> {
    match value {
        RetailRuntimeValue::Known(value) => Ok(value),
        RetailRuntimeValue::Unresolved => {
            Err(LevelOneType17PrimaryHitPreflightError::RuntimeFieldUnresolved(field))
        }
    }
}

fn exact_state(entity: &Entity) -> Result<u32, u32> {
    let state = entity.collision.state_flags_at_0x08;
    if state.known_mask() == u32::MAX {
        Ok(state.known_value_bits())
    } else {
        Err(state.known_mask())
    }
}

const fn allocation_identity(entity_id: u32) -> u64 {
    ALLOCATION_IDENTITY_DOMAIN | (entity_id as u64 + 1)
}

const fn type_record_identity(entity_type: u32) -> u64 {
    TYPE_RECORD_IDENTITY_DOMAIN | (entity_type as u64 + 1)
}

fn commit_hit_tick(
    manager: &mut EntityManager,
    entity_id: u32,
    expected_identity: u64,
    retail_tick: u32,
) -> Result<(), LevelOneType17PrimaryHitBlockReason> {
    let entity = manager
        .type17_primary_hit_entity_mut(entity_id)
        .ok_or(LevelOneType17PrimaryHitBlockReason::TargetUnavailable)?;
    if allocation_identity(entity.id) != expected_identity {
        return Err(LevelOneType17PrimaryHitBlockReason::TargetIdentityChanged);
    }
    entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(retail_tick);
    Ok(())
}

fn type17_candidate_snapshot(manager: &EntityManager) -> Vec<Type17ImpactEntityRef> {
    manager
        .iter()
        .map(|entity| Type17ImpactEntityRef {
            id: entity.id,
            entity_type: entity.entity_type,
            position_raw: entity.position_raw(),
            state_flags_raw: entity.collision.state_flags_at_0x08,
            capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
            // Candidate attachment values are not read by the evaluator.
            attached_entity_handle: RetailRuntimeValue::Known(None),
        })
        .collect()
}

fn apply_live_impact_reaction(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    expected_identity: u64,
    impact_sum_raw: i32,
    direction_q15: [i16; 3],
) -> Result<ImpactReactionOutcome, LevelOneType17PrimaryHitBlockReason> {
    let entity = manager
        .type17_primary_hit_entity_mut(entity_id)
        .ok_or(LevelOneType17PrimaryHitBlockReason::TargetUnavailable)?;
    if allocation_identity(entity.id) != expected_identity {
        return Err(LevelOneType17PrimaryHitBlockReason::TargetIdentityChanged);
    }
    let mut body = ImpactReactionBody {
        state_flags_at_0x08: exact_state(entity)
            .map_err(|_| LevelOneType17PrimaryHitBlockReason::StateWordBecameUnresolved)?,
        mass_raw_at_0xb0: entity.mass_raw,
        linear_velocity_xyz_raw: entity.velocity_raw(),
        angular_heading_pitch_roll_raw: entity.rotation_heading_pitch_roll_raw(),
    };
    let outcome =
        apply_impact_reaction(&mut body, entity_id, impact_sum_raw, direction_q15, || {
            u32::from(world_fx.next_shared_retail_random_u16())
        })
        .map_err(|_| LevelOneType17PrimaryHitBlockReason::RuntimeValueChanged("entity mass"))?;
    entity.set_velocity_raw(body.linear_velocity_xyz_raw);
    entity.set_rotation_heading_pitch_roll_raw(body.angular_heading_pitch_roll_raw);
    Ok(outcome)
}

fn sample_checked_damage_entry(
    manager: &EntityManager,
    preflight: &Type17PrimaryHitPreflight,
    entity_id: u32,
) -> Result<PrimaryHitCheckedDamageEntry, LevelOneType17PrimaryHitBlockReason> {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .ok_or(LevelOneType17PrimaryHitBlockReason::TargetUnavailable)?;
    let target_allocation_identity = allocation_identity(entity.id);
    if target_allocation_identity != preflight.target_allocation_identity {
        return Err(LevelOneType17PrimaryHitBlockReason::TargetIdentityChanged);
    }
    let current_type_record_identity = type_record_identity(entity.entity_type);
    if current_type_record_identity != preflight.type_record_identity {
        return Err(LevelOneType17PrimaryHitBlockReason::RuntimeValueChanged(
            "target type record identity",
        ));
    }
    let state = exact_state(entity)
        .map_err(|_| LevelOneType17PrimaryHitBlockReason::StateWordBecameUnresolved)?;
    Ok(PrimaryHitCheckedDamageEntry {
        admission_target: RetailRuntimeValue::Known(Some(CheckedDamageAdmissionTarget {
            target_allocation_identity,
            state_flags_at_0x08: state,
            capability_flags_at_0x64: entity.capability_flags as u8,
            modifier_address: entity.collision.pair_callbacks.damage_modifier_address,
        })),
        filter_binding: RetailRuntimeValue::Known(Some(CheckedDamageFilterBinding {
            target_allocation_identity,
            type_record_identity: current_type_record_identity,
            profile: entity.collision.damage_profile,
        })),
    })
}

#[allow(clippy::too_many_arguments)]
fn drive_checked_action(
    coordinator: &mut PrimaryHitCheckedDamageCoordinator,
    issued: crate::checked_damage::IssuedCheckedDamageAction,
    preflight: &Type17PrimaryHitPreflight,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    notifications: &mut GameplayNotifications,
    retail_tick: u32,
    common_dying: &mut Option<Type17CommonDyingPublicationOutcome>,
    player_kill_feedback_queued: &mut bool,
    journal: &mut SynchronousActionJournal,
    key: JournalKey,
) -> Result<(), LevelOneType17PrimaryHitBlockReason> {
    let phase = issued.action.phase();
    let entity_id = match &issued.action {
        CheckedDamageAction::ResolveGenericDamageEntry { target_handle, .. }
        | CheckedDamageAction::SampleCachedGenericHitSound { target_handle, .. }
        | CheckedDamageAction::SampleCachedGenericHitCallback { target_handle, .. }
        | CheckedDamageAction::SampleCachedGenericHealth { target_handle, .. }
        | CheckedDamageAction::SampleSurvivorNetworkTail { target_handle, .. }
        | CheckedDamageAction::ResolveDeathEntry { target_handle, .. }
        | CheckedDamageAction::SampleCachedDeathSound { target_handle, .. }
        | CheckedDamageAction::SampleCachedDeathAttachment { target_handle, .. }
        | CheckedDamageAction::SampleCachedDeathCallback { target_handle, .. }
        | CheckedDamageAction::SampleCachedDeathTargetAfterCallback { target_handle, .. }
        | CheckedDamageAction::SampleOriginalTargetAfterDeath { target_handle, .. }
        | CheckedDamageAction::SampleDeathNetworkTail { target_handle, .. } => *target_handle,
        CheckedDamageAction::CommitPreHealthBuffer { commit, .. } => commit.target_handle,
        CheckedDamageAction::CommitGenericHealth { commit, .. } => commit.target_handle,
        CheckedDamageAction::CommitDeathState { target_handle, .. } => *target_handle,
        CheckedDamageAction::InvokeDeathCallback { target_handle, .. } => *target_handle,
        _ => 0,
    };

    let completion = match &issued.action {
        CheckedDamageAction::ResolveGenericDamageEntry { .. } => {
            let observation = manager
                .iter_all()
                .find(|entity| entity.id == entity_id)
                .map_or(
                    GenericDamageEntryObservation {
                        target: RetailRuntimeValue::Known(None),
                        type_record_identity: RetailRuntimeValue::Known(None),
                        local_player_type_at_call: RetailRuntimeValue::Known(
                            preflight.local_player_type,
                        ),
                    },
                    |entity| GenericDamageEntryObservation {
                        target: exact_state(entity).map_or(
                            RetailRuntimeValue::Unresolved,
                            |state_flags_at_0x08| {
                                entity.collision.pre_health_damage_buffer_raw.map(
                                    |pre_health_buffer_raw| {
                                        Some(GenericDamageEntryTarget {
                                            target_allocation_identity: allocation_identity(
                                                entity.id,
                                            ),
                                            state_flags_at_0x08,
                                            pre_health_buffer_raw,
                                        })
                                    },
                                )
                            },
                        ),
                        type_record_identity: RetailRuntimeValue::Known(Some(
                            type_record_identity(entity.entity_type),
                        )),
                        local_player_type_at_call: RetailRuntimeValue::Known(
                            preflight.local_player_type,
                        ),
                    },
                );
            CheckedDamageResume::GenericDamageEntryResolved { phase, observation }
        }
        CheckedDamageAction::CommitPreHealthBuffer { commit, .. } => {
            let target = manager
                .type17_primary_hit_entity_mut(commit.target_handle)
                .and_then(|entity| {
                    (allocation_identity(entity.id) == commit.target_allocation_identity
                        && entity.collision.pre_health_damage_buffer_raw
                            == RetailRuntimeValue::Known(commit.before_raw))
                    .then_some(entity)
                })
                .and_then(|entity| {
                    entity.collision.pre_health_damage_buffer_raw =
                        RetailRuntimeValue::Known(commit.after_raw);
                    exact_state(entity)
                        .ok()
                        .map(|state_flags_at_0x08| GenericTargetStateSample {
                            target_allocation_identity: allocation_identity(entity.id),
                            state_flags_at_0x08,
                        })
                });
            CheckedDamageResume::PreHealthBufferCommitted {
                phase,
                target: RetailRuntimeValue::Known(target),
            }
        }
        CheckedDamageAction::SampleCachedGenericHitSound { .. } => {
            CheckedDamageResume::GenericHitSoundSampled {
                phase,
                observation: sample_entity(manager, entity_id, |entity| {
                    GenericHitSoundObservation {
                        target_allocation_identity: allocation_identity(entity.id),
                        type_record_identity: type_record_identity(entity.entity_type),
                        position_raw: entity.position_raw(),
                        sound_id: entity.collision.generic_hit_sound_id,
                    }
                }),
            }
        }
        CheckedDamageAction::PlayPositionalSound {
            sound_id,
            position_raw,
            ..
        } => {
            world_fx.queue_fixed_positional_sound_raw(*sound_id, *position_raw);
            CheckedDamageResume::Acknowledged { phase }
        }
        CheckedDamageAction::SampleCachedGenericHitCallback { .. } => {
            CheckedDamageResume::GenericHitCallbackSampled {
                phase,
                observation: sample_entity(manager, entity_id, |entity| {
                    GenericHitCallbackObservation {
                        type_record_identity: type_record_identity(entity.entity_type),
                        callback_address: entity.collision.pair_callbacks.type_hit_callback_address,
                    }
                }),
            }
        }
        CheckedDamageAction::SampleCachedGenericHealth { .. } => {
            CheckedDamageResume::GenericHealthSampled {
                phase,
                observation: sample_entity_runtime(manager, entity_id, |entity| {
                    entity.collision.health_raw.map(|health_raw| {
                        Some(GenericHealthObservation {
                            target_allocation_identity: allocation_identity(entity.id),
                            health_raw,
                        })
                    })
                }),
            }
        }
        CheckedDamageAction::CommitGenericHealth { commit, .. } => {
            let observation = manager
                .type17_primary_hit_entity_mut(commit.target_handle)
                .filter(|entity| {
                    allocation_identity(entity.id) == commit.target_allocation_identity
                        && entity.collision.health_raw
                            == RetailRuntimeValue::Known(commit.health_before_raw)
                })
                .map(|entity| {
                    entity.collision.health_raw =
                        RetailRuntimeValue::Known(commit.health_after_raw);
                    CachedTargetWriteObservation {
                        target_allocation_identity: allocation_identity(entity.id),
                    }
                });
            CheckedDamageResume::GenericHealthCommitted {
                phase,
                observation: RetailRuntimeValue::Known(observation),
            }
        }
        CheckedDamageAction::SampleSurvivorNetworkTail { .. } => {
            CheckedDamageResume::SurvivorNetworkTailSampled {
                phase,
                observation: sample_entity(manager, entity_id, |entity| {
                    SurvivorNetworkTailObservation {
                        target_allocation_identity: allocation_identity(entity.id),
                        capability_flags_at_0x64: entity.capability_flags as u8,
                        network_session_active: false,
                        current_local_player_type: RetailRuntimeValue::Unresolved,
                    }
                }),
            }
        }
        CheckedDamageAction::ResolveDeathEntry { .. } => {
            let observation = manager
                .iter_all()
                .find(|entity| entity.id == entity_id)
                .map_or(
                    DeathEntryObservation {
                        target: RetailRuntimeValue::Known(None),
                        type_record_identity: RetailRuntimeValue::Known(None),
                    },
                    |entity| DeathEntryObservation {
                        target: exact_state(entity).map_or(
                            RetailRuntimeValue::Unresolved,
                            |state_flags_at_0x08| {
                                RetailRuntimeValue::Known(Some(DeathEntryTarget {
                                    target_allocation_identity: allocation_identity(entity.id),
                                    state_flags_at_0x08,
                                }))
                            },
                        ),
                        type_record_identity: RetailRuntimeValue::Known(Some(
                            type_record_identity(entity.entity_type),
                        )),
                    },
                );
            CheckedDamageResume::DeathEntryResolved { phase, observation }
        }
        CheckedDamageAction::CommitDeathState {
            target_handle,
            target_allocation_identity,
            state_flags_before,
            state_flags_after,
            health_after_raw,
            ..
        } => {
            let observation = manager
                .type17_primary_hit_entity_mut(*target_handle)
                .filter(|entity| {
                    allocation_identity(entity.id) == *target_allocation_identity
                        && exact_state(entity) == Ok(*state_flags_before)
                })
                .and_then(|entity| {
                    entity.collision.health_raw = RetailRuntimeValue::Known(*health_after_raw);
                    let slot = active_model_slot_from_state_flags(*state_flags_after);
                    entity.select_active_model_slot(slot)?;
                    (exact_state(entity) == Ok(*state_flags_after)).then_some(
                        CachedTargetWriteObservation {
                            target_allocation_identity: allocation_identity(entity.id),
                        },
                    )
                });
            CheckedDamageResume::DeathStateCommitted {
                phase,
                observation: RetailRuntimeValue::Known(observation),
            }
        }
        CheckedDamageAction::SampleCachedDeathSound { .. } => {
            CheckedDamageResume::DeathSoundSampled {
                phase,
                observation: sample_entity(manager, entity_id, |entity| DeathSoundObservation {
                    target_allocation_identity: allocation_identity(entity.id),
                    type_record_identity: type_record_identity(entity.entity_type),
                    position_raw: entity.position_raw(),
                    sound_id: entity.collision.death_sound_id,
                }),
            }
        }
        CheckedDamageAction::SampleCachedDeathAttachment { .. } => {
            CheckedDamageResume::DeathAttachmentSampled {
                phase,
                observation: sample_entity(manager, entity_id, |entity| {
                    DeathAttachmentObservation {
                        target_allocation_identity: allocation_identity(entity.id),
                        attached_resource_identity: preflight.death_attachment_resource_identity,
                    }
                }),
            }
        }
        CheckedDamageAction::SampleCachedDeathCallback { .. } => {
            CheckedDamageResume::DeathCallbackSampled {
                phase,
                observation: sample_entity(manager, entity_id, |entity| DeathCallbackObservation {
                    type_record_identity: type_record_identity(entity.entity_type),
                    callback_address: RetailRuntimeValue::Known(Some(
                        TYPE17_STANDARD_DEATH_CALLBACK_ADDRESS,
                    )),
                }),
            }
        }
        CheckedDamageAction::InvokeDeathCallback {
            callback_address, ..
        } if *callback_address == TYPE17_STANDARD_DEATH_CALLBACK_ADDRESS => {
            let result = publish_type17_common_dying_from_checked_death(
                coordinator,
                issued,
                manager,
                Type17CommonDyingPublicationRequest {
                    retail_first_world: manager.is_fresh_new_game_first_world(),
                    metadata: &preflight.metadata,
                },
                world_fx,
            );
            match result {
                Ok(outcome) => {
                    *common_dying = Some(outcome);
                    journal.complete(key);
                    return Ok(());
                }
                Err(failure) => {
                    let error = failure.error;
                    let receipt = failure.issued_action.receipt;
                    coordinator
                        .resume_child(
                            receipt,
                            CheckedDamageResume::Blocked {
                                phase,
                                reason: CheckedDamageExternalBlock::CallbackUnavailable,
                            },
                        )
                        .expect("publisher returned the exact outstanding death receipt");
                    journal.complete(key);
                    return Err(LevelOneType17PrimaryHitBlockReason::CommonDying(error));
                }
            }
        }
        CheckedDamageAction::SampleCachedDeathTargetAfterCallback { .. } => {
            CheckedDamageResume::DeathTargetAfterCallbackSampled {
                phase,
                observation: sample_entity_runtime(manager, entity_id, |entity| {
                    exact_state(entity).map_or(RetailRuntimeValue::Unresolved, |state| {
                        RetailRuntimeValue::Known(Some(DeathTargetAfterCallbackObservation {
                            target_allocation_identity: allocation_identity(entity.id),
                            state_flags_at_0x08: state,
                            capability_flags_at_0x64: entity.capability_flags as u8,
                        }))
                    })
                }),
            }
        }
        CheckedDamageAction::SampleOriginalTargetAfterDeath { .. } => {
            CheckedDamageResume::OriginalTargetAfterDeathSampled {
                phase,
                observation: sample_entity_runtime(manager, entity_id, |entity| {
                    exact_state(entity).map_or(RetailRuntimeValue::Unresolved, |state| {
                        RetailRuntimeValue::Known(Some(OriginalTargetAfterDeathObservation {
                            target_allocation_identity: allocation_identity(entity.id),
                            state_flags_at_0x08: state,
                        }))
                    })
                }),
            }
        }
        CheckedDamageAction::SelectFeedback { selector, .. }
            if phase == CheckedDamagePhase::SelectPlayerKillFeedback
                && *selector == PLAYER_KILL_FEEDBACK_SELECTOR =>
        {
            notifications.queue_player_kill(retail_tick as i32);
            *player_kill_feedback_queued = true;
            CheckedDamageResume::Acknowledged { phase }
        }
        CheckedDamageAction::SampleDeathNetworkTail { .. } => {
            CheckedDamageResume::DeathNetworkTailSampled {
                phase,
                observation: sample_entity(manager, entity_id, |entity| {
                    DeathNetworkTailObservation {
                        target_allocation_identity: allocation_identity(entity.id),
                        capability_flags_at_0x64: entity.capability_flags as u8,
                        network_session_active: false,
                        current_local_player_type: RetailRuntimeValue::Unresolved,
                    }
                }),
            }
        }
        _ => {
            let label = checked_action_label(&issued.action);
            let receipt = issued.receipt;
            coordinator
                .resume_child(
                    receipt,
                    CheckedDamageResume::Blocked {
                        phase,
                        reason: CheckedDamageExternalBlock::AdapterRejected(1),
                    },
                )
                .expect("the adapter blocks the exact outstanding checked receipt");
            journal.complete(key);
            return Err(LevelOneType17PrimaryHitBlockReason::UnsupportedCheckedAction(label));
        }
    };

    let receipt = issued.receipt;
    coordinator
        .resume_child(receipt, completion)
        .expect("the synchronous owner resumes the exact checked action");
    journal.complete(key);
    Ok(())
}

fn sample_entity<T>(
    manager: &EntityManager,
    entity_id: u32,
    map: impl FnOnce(&Entity) -> T,
) -> RetailRuntimeValue<Option<T>> {
    RetailRuntimeValue::Known(
        manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .map(map),
    )
}

fn sample_entity_runtime<T>(
    manager: &EntityManager,
    entity_id: u32,
    map: impl FnOnce(&Entity) -> RetailRuntimeValue<Option<T>>,
) -> RetailRuntimeValue<Option<T>> {
    match manager.iter_all().find(|entity| entity.id == entity_id) {
        Some(entity) => map(entity),
        None => RetailRuntimeValue::Known(None),
    }
}

fn sample_cached_outer_target(
    manager: &EntityManager,
    preflight: &Type17PrimaryHitPreflight,
    entity_id: u32,
) -> RetailRuntimeValue<Option<PrimaryHitCachedTargetSnapshot>> {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return RetailRuntimeValue::Known(None);
    };
    let Ok(state_flags_at_0x08) = exact_state(entity) else {
        return RetailRuntimeValue::Unresolved;
    };
    RetailRuntimeValue::Known(Some(PrimaryHitCachedTargetSnapshot {
        target_allocation_identity: allocation_identity(entity.id),
        cached_type_record_identity: type_record_identity(entity.entity_type),
        state_flags_at_0x08,
        capability_flags_at_0x64: entity.capability_flags as u8,
        position_raw: entity.position_raw(),
        model_by_state_slot: preflight.model_by_state_slot,
        accepted_hit_sound_id: entity.collision.accepted_hit_presentation_sound_id,
    }))
}

fn checked_action_label(action: &CheckedDamageAction) -> &'static str {
    match action {
        CheckedDamageAction::SelectFeedback { .. } => "feedback selector",
        CheckedDamageAction::PlayNonPositionalSound { .. } => "non-positional sound",
        CheckedDamageAction::InvokeDamageModifier { .. } => "damage modifier",
        CheckedDamageAction::ForwardRemoteDamage { .. } => "remote damage",
        CheckedDamageAction::InvokeGenericHitCallback { .. } => "generic hit callback",
        CheckedDamageAction::SubmitSurvivorNetwork { .. } => "survivor network tail",
        CheckedDamageAction::ReleaseDeathAttachment { .. } => "death attachment release",
        CheckedDamageAction::CommitDeathAttachmentClear { .. } => "death attachment clear",
        CheckedDamageAction::InvokeDeathCallback { .. } => "unsupported death callback",
        CheckedDamageAction::FinalizeManagedDeath { .. } => "managed death finalizer",
        CheckedDamageAction::SubmitDeathNetwork { .. } => "death network tail",
        _ => "unexpected checked action",
    }
}

fn primary_journal_key(issued: &IssuedPrimaryHitAction) -> JournalKey {
    JournalKey {
        transaction_id: issued.receipt.transaction_id().get(),
        action_sequence: issued.receipt.action_sequence(),
    }
}

fn checked_journal_key(issued: &crate::checked_damage::IssuedCheckedDamageAction) -> JournalKey {
    JournalKey {
        transaction_id: issued.receipt.transaction_id().get(),
        action_sequence: issued.receipt.action_sequence(),
    }
}

fn acknowledge_parent(
    machine: &mut PrimaryHitMachine,
    issued: IssuedPrimaryHitAction,
    journal: &mut SynchronousActionJournal,
    key: JournalKey,
) {
    let phase = issued.action.phase();
    resume_parent(
        machine,
        issued,
        PrimaryHitResume::Acknowledged { phase },
        journal,
        key,
    );
}

fn block_parent(
    machine: &mut PrimaryHitMachine,
    issued: IssuedPrimaryHitAction,
    journal: &mut SynchronousActionJournal,
    key: JournalKey,
) {
    let phase = issued.action.phase();
    resume_parent(
        machine,
        issued,
        PrimaryHitResume::Blocked {
            phase,
            reason: primary_external_block(phase),
        },
        journal,
        key,
    );
}

const fn primary_external_block(
    phase: PrimaryHitPhase,
) -> crate::primary_hit::PrimaryHitExternalBlock {
    use crate::primary_hit::PrimaryHitExternalBlock;

    match phase {
        PrimaryHitPhase::CommitTickStamp => PrimaryHitExternalBlock::TickStampUnavailable,
        PrimaryHitPhase::TypeImpactCallback => {
            PrimaryHitExternalBlock::TypeImpactCallbackUnavailable
        }
        PrimaryHitPhase::ImpactReaction => PrimaryHitExternalBlock::ImpactReactionUnavailable,
        PrimaryHitPhase::ImpactReactionNetwork => {
            PrimaryHitExternalBlock::ImpactReactionNetworkUnavailable
        }
        PrimaryHitPhase::CheckedDamageDelivery => PrimaryHitExternalBlock::CheckedDamageUnavailable,
        PrimaryHitPhase::SampleCachedTargetAfterCheckedDamage
        | PrimaryHitPhase::SampleCachedTargetAfterAcceptedHitSound => {
            PrimaryHitExternalBlock::CachedOuterTargetSampleUnavailable
        }
        PrimaryHitPhase::AcceptedHitSound => PrimaryHitExternalBlock::AcceptedHitSoundUnavailable,
        PrimaryHitPhase::CapabilityFollowUp => {
            PrimaryHitExternalBlock::CapabilityFollowUpUnavailable
        }
    }
}

fn resume_parent(
    machine: &mut PrimaryHitMachine,
    issued: IssuedPrimaryHitAction,
    completion: PrimaryHitResume,
    journal: &mut SynchronousActionJournal,
    key: JournalKey,
) {
    machine
        .resume(issued.receipt, completion)
        .expect("the synchronous owner resumes the exact primary-hit action");
    journal.complete(key);
}

fn committed_primary_block(
    phase: PrimaryHitPhase,
    reason: LevelOneType17PrimaryHitBlockReason,
) -> LevelOneType17PrimaryHitOutcome {
    LevelOneType17PrimaryHitOutcome::CommittedPrefixBlocked {
        phase: LevelOneType17PrimaryHitPhase::Primary(phase),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        actor_standard_death_live::Type17CommonDyingPublicationOutcome,
        common_dying_live::COMMON_DYING_BEHAVIOR_CLASS_ID,
        common_mover::type9_attitude::TERRAIN_ATTITUDE_BILINEAR_STATE_BIT,
        damage::{DamagePacket, DamageProfile, DAMAGE_CHANNEL_COUNT},
        entity::CargoProxyEvent,
        entity_behavior::{
            audited_behavior_style, behavior_program, BehaviorChoiceListSource,
            BehaviorContextRuntime, BehaviorDescriptorIdentity,
        },
        entity_collision_state::{
            CommonMoverComponentTopology, EntityInitializerSpec, RetailStateWord,
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT,
            DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT, SURFACE_STATE_MASK,
        },
        retail_rng::retail_random_u16,
        sub_j_attachment::SubJAttachmentRuntime,
        type17_common_dying_production::{
            Type17CommonDyingProductionBlock, Type17CommonDyingProductionFrame,
            Type17CommonDyingProductionOutcome, Type17CommonDyingScheduler,
        },
        type17_impact_live::{
            TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR, TYPE17_MODEL256_COMPONENT_TOPOLOGY,
            TYPE17_MODEL256_SUB_A_TARGET_SPEED_BASE_RAW, TYPE17_MODEL256_SUB_H_RECORD_COUNT,
        },
        type17_impact_reselection::{TYPE17_BEHAVIOR_RULE_REF, TYPE17_IMPACT_BEHAVIOR_CHOICES},
    };
    use v2k_formats::{
        collision::{
            BehaviorChoice, SubAPropulsionDescriptor, SubBLateralDescriptor, SubCLiftDescriptor,
            SubHExternalFrameDescriptor, SubHExternalFrameRecord, SubJAttachmentDescriptor,
            SubJAttachmentSlotDescriptor,
        },
        levels::{EntitySpawn, LevelDescriptor},
        models::ModelEntry,
        terrain::{TerrainCell, TerrainGrid, GRID_SIZE},
    };

    const TARGET_SPAWN_INDEX: usize = FRESH_LEVEL1_TYPE17_COMMON_DYING_SPAWN_INDICES[0];
    const CAPTURED_LIVE_STATE: u32 = 0x0746_8825;
    const NORMAL_CALLBACK_ENABLED_STATE_BIT: u32 = 0x0002_0000;
    const SOURCE_OWNER: u32 = 0x04bd_0001;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct CoarseMutationSnapshot {
        state_flags: RetailStateWord,
        mass_raw: u16,
        position_raw: [i16; 3],
        velocity_raw: [i16; 3],
        rotation_raw: [i16; 3],
        surface_timer: RetailRuntimeValue<u32>,
        scheduler_recent_elapsed: RetailRuntimeValue<u32>,
        scheduler_callback_accumulator: RetailRuntimeValue<u32>,
        scheduler_subject_gate: RetailRuntimeValue<u32>,
        scheduler_unit_delta_flag: RetailRuntimeValue<u8>,
        animation_offset: RetailRuntimeValue<u16>,
        behavior_context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
        tasks: [Option<crate::actor_task_dispatcher::ActorTaskRuntime>; 3],
    }

    fn coarse_mutation_snapshot(entity: &Entity) -> CoarseMutationSnapshot {
        CoarseMutationSnapshot {
            state_flags: entity.collision.state_flags_at_0x08,
            mass_raw: entity.mass_raw,
            position_raw: entity.position_raw(),
            velocity_raw: entity.velocity_raw(),
            rotation_raw: entity.rotation_heading_pitch_roll_raw(),
            surface_timer: entity.surface_lifetime_timer_ms_at_0x48,
            scheduler_recent_elapsed: entity.collision.recent_relation_elapsed_us_at_0x68,
            scheduler_callback_accumulator: entity
                .collision
                .callback_scheduler_accumulator_us_at_0x6c,
            scheduler_subject_gate: entity.collision.subject_scan_gate_at_0x70,
            scheduler_unit_delta_flag: entity.collision.scheduler_unit_delta_flag_at_0xb6,
            animation_offset: entity.collision.animation_offset_at_0xb2,
            behavior_context: entity.current_behavior_context,
            tasks: crate::actor_task_owner::ActorTaskSlot::IN_RETAIL_TICK_ORDER
                .map(|slot| entity.actor_task_state(slot).copied()),
        }
    }

    struct TestModels {
        type17: ModelEntry,
    }

    impl CollisionModelPool for TestModels {
        fn collision_model(&self, global_id: usize) -> Option<&ModelEntry> {
            (global_id == TYPE17_COMMON_DYING_MODEL_ID).then_some(&self.type17)
        }
    }

    fn test_models() -> TestModels {
        TestModels {
            type17: ModelEntry {
                index: TYPE17_COMMON_DYING_MODEL_ID,
                cmd_word_count: 0,
                extra_count: 0,
                flags: 0,
                slot_count: 0,
                face_val: 0,
                radius: 73,
                collision_radius_raw: 100,
                collision_program: Vec::new(),
                records: Vec::new(),
                normal_pool: Vec::new(),
                cmd_words: Vec::new(),
                has_view_commands: false,
                vertices: Vec::new(),
                vertex_type_flags: Vec::new(),
                vertex_projection: Vec::new(),
                vertex_clip: Vec::new(),
                vertex_surface_origin: Vec::new(),
                triangles: Vec::new(),
                face_vertices: Vec::new(),
                normals: Vec::new(),
                face_cull: Vec::new(),
                face_materials: Vec::new(),
                face_uvs: Vec::new(),
                face_corner_normals: Vec::new(),
                face_shading: Vec::new(),
                shadow_triangles: Vec::new(),
                edges: Vec::new(),
                billboards: Vec::new(),
                instances: Vec::new(),
                name: Some("spider".into()),
            },
        }
    }

    fn identity_damage_profile() -> DamageProfile {
        DamageProfile {
            thresholds_raw: [0; DAMAGE_CHANNEL_COUNT],
            multipliers_q8: [256; DAMAGE_CHANNEL_COUNT],
        }
    }

    fn exact_type17_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [TYPE17_COMMON_DYING_MODEL_ID as u16; 4],
            mass_raw: 100,
            capability_flags: 8,
            initial_health_raw: Some(5_000),
            damage_profile: Some(identity_damage_profile()),
            accepted_hit_presentation_sound_id: RetailRuntimeValue::Known(Some(84)),
            infected_model_presentation_sound_id: RetailRuntimeValue::Known(Some(92)),
            death_sound_id: RetailRuntimeValue::Known(Some(94)),
            generic_hit_sound_id: RetailRuntimeValue::Known(None),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: 57,
                common_axis_descriptor: TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR,
                behavior_choices: TYPE17_IMPACT_BEHAVIOR_CHOICES.to_vec().into_boxed_slice(),
                behavior_rule_ref: TYPE17_BEHAVIOR_RULE_REF,
                alternate_behavior_class_ref: COMMON_DYING_BEHAVIOR_CLASS_ID,
            }),
            common_mover_topology: RetailRuntimeValue::Known(TYPE17_MODEL256_COMPONENT_TOPOLOGY),
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                SubAPropulsionDescriptor {
                    acceleration_raw: 100,
                    overspeed_correction_raw: 200,
                    target_speed_base_raw: TYPE17_MODEL256_SUB_A_TARGET_SPEED_BASE_RAW,
                },
            )),
            sub_b_lateral_descriptor: RetailRuntimeValue::Known(Some(SubBLateralDescriptor {
                projection_threshold_rate_raw: 10_000,
                correction_rate_raw: 1_000,
            })),
            sub_c_lift_descriptor: RetailRuntimeValue::Known(Some(SubCLiftDescriptor {
                base_clearance_raw: 150,
                lift_range_raw: 125,
                strength_raw: 0x0090_0000,
                near_boost_range_raw: 100,
                damping_range_raw: 200,
                surface_mode_raw: 0,
                offset_sample_raw: 0,
                reserved_at_0x0e: [0; 2],
            })),
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
            sub_j_attachment_descriptor: RetailRuntimeValue::Known(Some(
                SubJAttachmentDescriptor {
                    reserved_at_0x01: 0,
                    slots: vec![SubJAttachmentSlotDescriptor {
                        policy_word_raw: 1,
                        local_offset_raw: [0, 10, 110],
                    }]
                    .into_boxed_slice(),
                },
            )),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn exact_type93_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            cured_model_presentation_sound_id: RetailRuntimeValue::Unresolved,
            common_world_effects: RetailRuntimeValue::Unresolved,
            detailed_sound_policy: RetailRuntimeValue::Unresolved,
            model_slots: [0; 4],
            mass_raw: 1,
            capability_flags: 0,
            initial_health_raw: Some(1_000),
            damage_profile: Some(DamageProfile {
                thresholds_raw: [0; DAMAGE_CHANNEL_COUNT],
                multipliers_q8: [0; DAMAGE_CHANNEL_COUNT],
            }),
            accepted_hit_presentation_sound_id: RetailRuntimeValue::Known(None),
            infected_model_presentation_sound_id: RetailRuntimeValue::Known(None),
            death_sound_id: RetailRuntimeValue::Known(None),
            target_warning_sound_id: RetailRuntimeValue::Unresolved,
            constructor_sound_attachment_id: RetailRuntimeValue::Known(None),
            generic_hit_sound_id: RetailRuntimeValue::Known(None),
            search_attack_optional_prelude_sound_id: RetailRuntimeValue::Unresolved,
            search_attack_aim_sound_id: RetailRuntimeValue::Unresolved,
            search_attack_aim_sound_period_raw: RetailRuntimeValue::Unresolved,
            run_away_optional_sound_id: RetailRuntimeValue::Unresolved,
            run_away_sound_period_raw: RetailRuntimeValue::Unresolved,
            terrain_contact_task_lifetime_ms: RetailRuntimeValue::Unresolved,
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(None),
            sub_b_lateral_descriptor: RetailRuntimeValue::Known(None),
            sub_c_lift_descriptor: RetailRuntimeValue::Known(None),
            sub_d_steering_descriptor: RetailRuntimeValue::Known(None),
            sub_f_swimming_descriptor: RetailRuntimeValue::Known(None),
            model_variable_count_raw: RetailRuntimeValue::Unresolved,
            projectile_emitter_descriptor: RetailRuntimeValue::Known(None),
            sub_n_payload: None,
            status_component_descriptor: RetailRuntimeValue::Known(None),
            actor_animation_descriptor: RetailRuntimeValue::Known(None),
            sub_h_external_frame_descriptor: RetailRuntimeValue::Known(None),
            sub_j_attachment_descriptor: RetailRuntimeValue::Known(Some(
                SubJAttachmentDescriptor {
                    reserved_at_0x01: 0,
                    slots: vec![SubJAttachmentSlotDescriptor {
                        policy_word_raw: 0,
                        local_offset_raw: [0; 3],
                    }]
                    .into_boxed_slice(),
                },
            )),
            common_mover_topology: RetailRuntimeValue::Known(CommonMoverComponentTopology {
                sub_j: true,
                ..CommonMoverComponentTopology::default()
            }),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: 0x4023,
                common_axis_descriptor: Default::default(),
                behavior_choices: vec![BehaviorChoice {
                    weight_rule_id: 1,
                    weight_multiplier: 1,
                    behavior_class_id: 30,
                }]
                .into_boxed_slice(),
                behavior_rule_ref: 1,
                alternate_behavior_class_ref: 2,
            }),
            common_mover_gkl_payloads: RetailRuntimeValue::Unresolved,
        }
    }

    fn known_non_type17_constructor_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: 0,
                common_axis_descriptor: Default::default(),
                behavior_choices: Box::new([]),
                behavior_rule_ref: 0,
                alternate_behavior_class_ref: 0,
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn spawn(index: usize, entity_type: u32) -> EntitySpawn {
        EntitySpawn {
            index,
            entity_type,
            pos_data_1: [0; 4],
            pos_data_2: [0; 4],
            param: 0,
            rotation: [0; 3],
            extra: [0; 40],
            initial_damage_buffer_raw: 0,
            model_overrides: [0; 4],
            has_animation: false,
            anim_frames: 0,
            animation: None,
            has_config: false,
            config: None,
        }
    }

    fn first_world_level() -> LevelDescriptor {
        let mut entities = (0..35).map(|index| spawn(index, 0)).collect::<Vec<_>>();
        entities[6] = spawn(6, 6);
        entities[6].pos_data_1 = [0x00, 0x50, 0x00, 0x00];
        entities[6].pos_data_2[..2].copy_from_slice(&[0x00, 0x3c]);
        entities[TARGET_SPAWN_INDEX] = spawn(TARGET_SPAWN_INDEX, 17);
        entities[TARGET_SPAWN_INDEX].param = 1;
        LevelDescriptor {
            raw_header: [0; 0xd0],
            name: String::new(),
            world_style: 0,
            terrain_sprite_base: 0,
            sky_color_index: 0,
            sky_model: 0,
            main_base_abort_sky_color_index: 0,
            main_base_abort_sky_model: 0,
            terrain_draw_depth: 0,
            sub_count: entities.len() as u32,
            campaign_record_count: 0,
            entities,
            campaign_records: Vec::new(),
        }
    }

    fn exact_constructor_terrain() -> TerrainGrid {
        TerrainGrid {
            // Static ground and the authored Y=0 are both above sea level,
            // proving the exact Level-1 constructor surface bit.
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

    fn named_context(class_id: u32, variant: u8) -> BehaviorContextRuntime {
        let program = behavior_program(class_id).expect("audited behavior program");
        let style = *audited_behavior_style(class_id, variant).expect("audited behavior style");
        BehaviorContextRuntime::named_audited(
            program,
            u32::from(variant),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(Some(0x047f_0001)),
            RetailRuntimeValue::Known(0x1357_9bdf),
            style,
        )
        .expect("coherent named behavior context")
    }

    fn exact_manager(health_raw: i32) -> (EntityManager, u32) {
        let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 94];
        for entity_type in [0, 6, 46] {
            metadata[entity_type] = known_non_type17_constructor_metadata();
        }
        metadata[17] = exact_type17_metadata();
        metadata[93] = exact_type93_metadata();
        let terrain = exact_constructor_terrain();
        let mut world_fx = WorldFx::new();
        let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
            &first_world_level(),
            &metadata,
            Some(&terrain),
            0,
            &mut world_fx,
        )
        .expect("fresh type-17 birth publication");
        let all_ids = manager
            .iter_all()
            .map(|entity| entity.id)
            .collect::<Vec<_>>();
        for id in all_ids {
            manager
                .entity_mut_for_test(id)
                .expect("fixture allocation")
                .collision
                .state_flags_at_0x08 = RetailStateWord::exact(0);
        }
        let target_id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(TARGET_SPAWN_INDEX))
            .expect("authored type-17 target")
            .id;
        let target = manager
            .entity_mut_for_test(target_id)
            .expect("type-17 fixture target");
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(CAPTURED_LIVE_STATE);
        target.collision.health_raw = RetailRuntimeValue::Known(health_raw);
        target.current_behavior_context = RetailRuntimeValue::Known(Some(named_context(33, 1)));
        target.set_velocity_raw([11, -22, 33]);
        (manager, target_id)
    }

    fn request(target_entity_id: u32) -> LevelOneType17ProjectilePrimaryHitRequest {
        LevelOneType17ProjectilePrimaryHitRequest {
            target_entity_id,
            delivery: DamageDeliveryRecord {
                packet: DamagePacket {
                    channels: [1, 0],
                    amounts_raw: [1_800, 0],
                },
                source_entity_type_raw: 46,
                owner_handle: SOURCE_OWNER,
            },
            source_provenance: 0x1234_5678,
            impact_direction_q15: [i16::MAX, 0, i16::MIN],
            retail_tick: 300,
        }
    }

    #[test]
    fn unresolved_fresh_context_rejects_before_the_hit_tick_moves() {
        let (mut manager, target_id) = exact_manager(5_000);
        let target = manager.entity_mut_for_test(target_id).unwrap();
        target.current_behavior_context = RetailRuntimeValue::Unresolved;
        target.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();

        let outcome = apply_level_one_type17_projectile_primary_hit(
            &mut manager,
            &test_models(),
            &mut world_fx,
            &mut notifications,
            request(target_id),
        );

        assert_eq!(
            outcome,
            LevelOneType17PrimaryHitOutcome::PreflightRejected(
                LevelOneType17PrimaryHitPreflightError::CurrentBehaviorContextUnresolved
            )
        );
        assert_eq!(
            manager
                .iter_all()
                .find(|entity| entity.id == target_id)
                .unwrap()
                .collision
                .last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(17)
        );
        assert_eq!(world_fx.particle_count(), 0);
    }

    #[test]
    fn impact_candidate_snapshot_excludes_attached_otherwise_eligible_cargo() {
        let (mut manager, target_id) = exact_manager(5_000);
        let candidate_id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(0))
            .expect("fixture candidate")
            .id;
        let candidate = manager
            .entity_mut_for_test(candidate_id)
            .expect("fixture candidate remains live");
        candidate.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        candidate.capability_flags = u32::MAX;

        assert!(type17_candidate_snapshot(&manager)
            .iter()
            .any(|candidate| candidate.id == candidate_id));

        manager
            .entity_mut_for_test(candidate_id)
            .expect("fixture candidate remains live")
            .attached_to = Some(target_id);

        assert!(manager
            .iter_all()
            .any(|candidate| candidate.id == candidate_id));
        assert!(!type17_candidate_snapshot(&manager)
            .iter()
            .any(|candidate| candidate.id == candidate_id));
    }

    #[test]
    fn impact_candidate_snapshot_preserves_partial_state_evidence() {
        let (mut manager, _) = exact_manager(5_000);
        let candidate_id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(0))
            .expect("fixture candidate")
            .id;
        let masked_state = RetailStateWord::from_known_bits(1, !SURFACE_STATE_MASK);
        manager
            .entity_mut_for_test(candidate_id)
            .expect("fixture candidate remains live")
            .collision
            .state_flags_at_0x08 = masked_state;

        let candidate = type17_candidate_snapshot(&manager)
            .into_iter()
            .find(|candidate| candidate.id == candidate_id)
            .expect("ordinary candidate remains in the live snapshot");
        assert_eq!(candidate.state_flags_raw, masked_state);
    }

    #[test]
    fn checked_entry_reauthenticates_the_fresh_type_record_identity() {
        let (mut manager, target_id) = exact_manager(5_000);
        let preflight = preflight_type17_primary_hit(&manager, &test_models(), target_id)
            .expect("preflight evidence is complete")
            .expect("fixture is the bounded type-17 target");
        manager
            .entity_mut_for_test(target_id)
            .expect("fixture target remains live")
            .entity_type = 47;

        assert_eq!(
            sample_checked_damage_entry(&manager, &preflight, target_id),
            Err(LevelOneType17PrimaryHitBlockReason::RuntimeValueChanged(
                "target type record identity"
            ))
        );
    }

    #[test]
    fn authenticated_nonlethal_hit_composes_reselection_reaction_damage_and_suffix() {
        let (mut manager, target_id) = exact_manager(5_000);
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();

        let outcome = apply_level_one_type17_projectile_primary_hit(
            &mut manager,
            &test_models(),
            &mut world_fx,
            &mut notifications,
            request(target_id),
        );
        let LevelOneType17PrimaryHitOutcome::Complete(completion) = outcome else {
            panic!("expected completed nonlethal transaction: {outcome:?}")
        };

        assert_eq!(
            completion.primary,
            PrimaryHitCompletion::Accepted(std::num::NonZeroI32::new(1_800).unwrap())
        );
        assert!(completion.common_dying.is_none());
        assert!(!completion.player_kill_feedback_queued);
        let target = manager
            .iter_all()
            .find(|entity| entity.id == target_id)
            .unwrap();
        assert_eq!(
            target.collision.health_raw,
            RetailRuntimeValue::Known(3_200)
        );
        assert_eq!(
            target.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(300)
        );
        assert_ne!(target.velocity_raw(), [11, -22, 33]);
        world_fx.process_pending();
        assert_eq!(
            world_fx.take_positional_sounds(),
            vec![crate::world_fx::PositionalSoundEvent::fixed(
                84,
                target.position
            )]
        );
        assert_eq!(world_fx.particle_count(), 1);
    }

    #[test]
    fn capture_people_variant_zero_publication_continues_the_primary_hit() {
        let (mut manager, target_id) = exact_manager(5_000);
        let candidate_id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(0))
            .expect("fixture Capture candidate")
            .id;
        let candidate = manager
            .entity_mut_for_test(candidate_id)
            .expect("fixture Capture candidate remains live");
        candidate.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        candidate.capability_flags = 0x0400;
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();

        let outcome = apply_level_one_type17_projectile_primary_hit(
            &mut manager,
            &test_models(),
            &mut world_fx,
            &mut notifications,
            request(target_id),
        );
        let LevelOneType17PrimaryHitOutcome::Complete(completion) = outcome else {
            panic!("Capture People publication blocked the parent hit: {outcome:?}")
        };
        let Type17ImpactLiveOutcome::SharedAcquiringPublished(published) =
            completion.impact_reselection
        else {
            panic!(
                "expected Capture People shared acquisition: {:?}",
                completion.impact_reselection
            )
        };
        assert_eq!(published.weighted.selection.program.class_id, 9);
        assert_eq!(published.weighted.selection.choice_index, 0);
        assert_eq!(published.weighted.random_word, 0x0026);
        assert_eq!(
            published.constructors_by_phase[0].random_sample_low16,
            0x1e27
        );
        assert_eq!(
            published.constructors_by_phase[1].random_sample_low16,
            0xd2f6
        );

        let target = manager
            .iter_all()
            .find(|entity| entity.id == target_id)
            .expect("type-17 target remains live");
        assert_eq!(
            target.collision.health_raw,
            RetailRuntimeValue::Known(3_200)
        );
        let RetailRuntimeValue::Known(Some(context)) = target.current_behavior_context else {
            panic!("Capture People context was not published")
        };
        assert!(matches!(
            context.descriptor(),
            BehaviorDescriptorIdentity::Named(program) if program.class_id == 9
        ));
    }

    #[test]
    fn authenticated_lethal_hit_publishes_class12_feedback_death_sound_and_class5() {
        let (mut manager, target_id) = exact_manager(1_400);
        let terrain = exact_constructor_terrain();
        let mut scheduler = Type17CommonDyingScheduler::new();
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();

        // Main's actor-manager phase precedes the physical-particle callback
        // that publishes this receipt.  Nothing can age during that phase;
        // the newly registered owner first becomes eligible on the next
        // scheduler invocation.
        let pre_publication_pass = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || 0,
        );
        assert!(pre_publication_pass.outcomes.is_empty());
        let (position_before, model_before, active_before, attached_before, heading_before_raw) = {
            let target = manager
                .iter_all()
                .find(|entity| entity.id == target_id)
                .expect("authenticated type-17 target");
            (
                target.position,
                target.model_index,
                target.active,
                target.attached_to,
                target.rotation_heading_pitch_roll_raw()[0],
            )
        };

        let outcome = apply_level_one_type17_projectile_primary_hit(
            &mut manager,
            &test_models(),
            &mut world_fx,
            &mut notifications,
            request(target_id),
        );
        let LevelOneType17PrimaryHitOutcome::Complete(completion) = outcome else {
            panic!("expected completed lethal transaction: {outcome:?}")
        };

        let Some(Type17CommonDyingPublicationOutcome::Published { owner, .. }) =
            completion.common_dying
        else {
            panic!("lethal transaction did not return its Common-Dying receipt")
        };
        assert!(completion.player_kill_feedback_queued);
        let target = manager
            .iter_all()
            .find(|entity| entity.id == target_id)
            .unwrap();
        assert_eq!(target.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            active_model_slot_from_state_flags(exact_state(target).unwrap()),
            1
        );
        assert_eq!(target.model_index, Some(TYPE17_COMMON_DYING_MODEL_ID));
        assert_eq!(target.position, position_before);
        assert_eq!(target.model_index, model_before);
        assert_eq!(target.active, active_before);
        assert_eq!(target.attached_to, attached_before);
        let ImpactReactionOutcome::Applied(applied_reaction) = completion.impact_reaction else {
            panic!("authenticated lethal hit must apply its impact reaction")
        };
        assert_eq!(
            target.rotation_heading_pitch_roll_raw()[0],
            heading_before_raw
                .wrapping_add(applied_reaction.angular_delta_heading_pitch_roll_raw[0]),
            "the lethal transaction must commit its post-impact heading"
        );
        assert_ne!(
            target.rotation_heading_pitch_roll_raw()[0],
            heading_before_raw,
            "the exact lethal fixture must exercise an orientation mutation"
        );
        assert_eq!(target.velocity_raw()[1], 500);
        let RetailRuntimeValue::Known(Some(context)) = target.current_behavior_context else {
            panic!("class-12 context was not published")
        };
        assert!(matches!(
            context.descriptor(),
            BehaviorDescriptorIdentity::Named(program)
                if u32::from(program.class_id) == COMMON_DYING_BEHAVIOR_CLASS_ID
        ));
        assert_eq!(
            target
                .collision
                .state_flags_at_0x08
                .masked(crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT),
            RetailRuntimeValue::Known(
                crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            )
        );
        assert_eq!(
            target.collision.callback_scheduler_accumulator_us_at_0x6c,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            target.collision.subject_scan_gate_at_0x70,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            target.collision.scheduler_unit_delta_flag_at_0xb6,
            RetailRuntimeValue::Known(0)
        );
        let mut scheduler_probe = target.collision.clone();
        let mut scheduler_random_draws = 0;
        assert_eq!(
            scheduler_probe.advance_common_scheduler_prefix(20_000, &mut || {
                scheduler_random_draws += 1;
                0
            }),
            RetailRuntimeValue::Known(
                crate::entity_scheduler::CommonSchedulerPrefixFlow::Continue {
                    callback_elapsed_us: 20_000,
                }
            )
        );
        assert_eq!(scheduler_random_draws, 0);
        world_fx.process_pending();
        assert_eq!(
            world_fx.take_positional_sounds(),
            vec![crate::world_fx::PositionalSoundEvent::fixed(
                94,
                target.position
            )]
        );
        assert_eq!(world_fx.particle_count(), 1);

        scheduler.register(owner);
        let target = manager
            .entity_mut_for_test(target_id)
            .expect("published Common-Dying target remains live");
        target.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Unresolved;
        let unresolved_suffix_before = coarse_mutation_snapshot(target);
        let blocked = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || 0,
        );
        assert_eq!(
            blocked.outcomes,
            [Type17CommonDyingProductionOutcome::Blocked {
                entity_id: target_id,
                reason: Type17CommonDyingProductionBlock::UnresolvedSurfaceLifetimeTimer,
            }]
        );
        let target = manager
            .iter_all()
            .find(|entity| entity.id == target_id)
            .expect("preflight-blocked detailed target remains live");
        assert_eq!(coarse_mutation_snapshot(target), unresolved_suffix_before);
        assert_eq!(scheduler.registered_len(), 1);
        manager
            .entity_mut_for_test(target_id)
            .expect("preflight-blocked detailed target remains live")
            .surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(0);
        let Some(crate::actor_task_dispatcher::ActorTaskRuntime::CommonDying(task)) = manager
            .iter_all()
            .find(|entity| entity.id == target_id)
            .and_then(|entity| {
                entity.actor_task_state(crate::actor_task_owner::ActorTaskSlot::Primary)
            })
        else {
            panic!("published Common-Dying task was not live")
        };
        assert_eq!(task.elapsed_ms(), 0);
        let master_motion_mask =
            crate::common_mover::type9_tail::COMMON_MASTER_MOTION_REQUIRED_STATE_MASK;
        let target = manager
            .entity_mut_for_test(target_id)
            .expect("published Common-Dying target remains live");
        let master_motion_bits = exact_state(target).unwrap() & master_motion_mask;
        target
            .collision
            .state_flags_at_0x08
            .invalidate(master_motion_mask);
        let unresolved_master_before = coarse_mutation_snapshot(target);
        let blocked = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || 0,
        );
        assert_eq!(
            blocked.outcomes,
            [Type17CommonDyingProductionOutcome::Blocked {
                entity_id: target_id,
                reason: Type17CommonDyingProductionBlock::UnresolvedMasterMotionState,
            }]
        );
        let target = manager
            .iter_all()
            .find(|entity| entity.id == target_id)
            .expect("master-motion-preflight-blocked target remains live");
        assert_eq!(coarse_mutation_snapshot(target), unresolved_master_before);
        assert_eq!(scheduler.registered_len(), 1);
        let target = manager
            .entity_mut_for_test(target_id)
            .expect("master-motion-preflight-blocked target remains live");
        target
            .collision
            .state_flags_at_0x08
            .overwrite(master_motion_mask, master_motion_bits);
        target.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(77);
        let child_id = manager
            .iter_all()
            .find(|entity| entity.id != target_id && entity.attached_to.is_none())
            .expect("fresh Level 1 retains a direct-attachment child fixture")
            .id;
        let child = manager.entity_mut_for_test(child_id).unwrap();
        child.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        child.set_motion_raw([777, -888, 999], [7, 8, 9]);
        let target = manager.entity_mut_for_test(target_id).unwrap();
        let RetailRuntimeValue::Known(Some(runtime)) = &mut target.sub_j_attachment_runtime else {
            panic!("published Type-17 target retains its exact Sub-J runtime")
        };
        runtime.append(child_id).unwrap();
        let callback_mass_raw = crate::entity::LEVEL_ONE_TYPE17_SELF_MASS_RAW + 77;
        let pre_frame_position_raw = target.position_raw();
        let expected_child_position_raw =
            [pre_frame_position_raw[0], 150, pre_frame_position_raw[2]];
        let pass = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || 0,
        );
        assert!(matches!(
            pass.outcomes.as_slice(),
            [Type17CommonDyingProductionOutcome::Advanced {
                entity_id,
                live_outcome:
                    crate::common_dying_live::Type17CommonDyingLiveFrameOutcome::Continue { .. },
            }] if *entity_id == target_id
        ));
        assert!(manager.cleanup_pending_actor_deferred_destroys().is_empty());
        let [Type17CommonDyingProductionOutcome::Advanced {
            live_outcome:
                crate::common_dying_live::Type17CommonDyingLiveFrameOutcome::Continue {
                    linear_velocity_raw,
                    ..
                },
            ..
        }] = pass.outcomes.as_slice()
        else {
            unreachable!("the detailed success shape was asserted above")
        };
        let mut expected_suffix_velocity = *linear_velocity_raw;
        crate::entity::apply_first_world_type17_environment_raw(
            &mut expected_suffix_velocity,
            20_000,
            callback_mass_raw,
        );
        let mut expected_master_position = pre_frame_position_raw;
        let mut expected_master_velocity = expected_suffix_velocity;
        let mut expected_master_state = master_motion_bits;
        crate::common_mover::type9_tail::apply_common_master_motion_raw(
            &mut expected_master_position,
            &mut expected_master_velocity,
            &mut expected_master_state,
            20_000,
            0,
        );
        let target = manager
            .iter_all()
            .find(|entity| entity.id == target_id)
            .unwrap();
        assert_eq!(
            target.surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            target.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(20_000)
        );
        assert_eq!(
            target.collision.callback_scheduler_accumulator_us_at_0x6c,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            target.collision.subject_scan_gate_at_0x70,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            target.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(target.mass_raw, callback_mass_raw);
        assert_eq!(target.velocity_raw(), expected_master_velocity);
        assert_eq!(target.position_raw()[0], expected_master_position[0]);
        assert_eq!(target.position_raw()[2], expected_master_position[2]);
        assert_eq!(
            target.collision.state_flags_at_0x08.masked(0x20),
            RetailRuntimeValue::Known(0x20)
        );
        assert_eq!(
            target
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT),
            RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
        );
        let target_position_after_master_raw = target.position_raw();
        let RetailRuntimeValue::Known(Some(runtime)) = &target.sub_j_attachment_runtime else {
            panic!("direct-attachment runtime remains live")
        };
        assert_eq!(runtime.ordered_entity_ids(), [child_id]);
        let child = manager
            .iter_all()
            .find(|entity| entity.id == child_id)
            .unwrap();
        assert_eq!(
            child.position_raw(),
            expected_child_position_raw,
            "the child consumes the post-callback owner pose before outer master motion"
        );
        assert_eq!(child.velocity_raw(), [0; 3]);
        assert_ne!(
            target_position_after_master_raw, expected_child_position_raw,
            "the active outer master step must visibly separate owner and child poses"
        );
        assert_eq!(scheduler.registered_len(), 1);
    }

    #[test]
    fn scheduler_owner_and_authoritative_attachment_gates_block_before_shared_rng() {
        let (mut manager, target_id) = exact_manager(1_400);
        let terrain = exact_constructor_terrain();
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let outcome = apply_level_one_type17_projectile_primary_hit(
            &mut manager,
            &test_models(),
            &mut world_fx,
            &mut notifications,
            request(target_id),
        );
        let LevelOneType17PrimaryHitOutcome::Complete(completion) = outcome else {
            panic!("expected completed lethal transaction: {outcome:?}")
        };
        let Some(Type17CommonDyingPublicationOutcome::Published { owner, .. }) =
            completion.common_dying
        else {
            panic!("lethal transaction did not publish Common-Dying")
        };
        let mut scheduler = Type17CommonDyingScheduler::new();
        scheduler.register(owner);

        let target = manager.entity_mut_for_test(target_id).unwrap();
        target
            .collision
            .state_flags_at_0x08
            .overwrite(REMOTE_OWNED_STATE_BIT, REMOTE_OWNED_STATE_BIT);
        let before = coarse_mutation_snapshot(target);
        let pass = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || panic!("a remote owner must block before shared RNG"),
        );
        assert_eq!(
            pass.outcomes,
            [Type17CommonDyingProductionOutcome::Blocked {
                entity_id: target_id,
                reason: Type17CommonDyingProductionBlock::RemoteSchedulerOwnerUnimplemented,
            }]
        );
        assert_eq!(
            coarse_mutation_snapshot(manager.entity_mut_for_test(target_id).unwrap()),
            before
        );

        let target = manager.entity_mut_for_test(target_id).unwrap();
        target.collision.state_flags_at_0x08.overwrite(
            REMOTE_OWNED_STATE_BIT | NORMAL_CALLBACK_ENABLED_STATE_BIT,
            0,
        );
        let before = coarse_mutation_snapshot(target);
        let pass = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || panic!("a callback-disabled owner must block before shared RNG"),
        );
        assert_eq!(
            pass.outcomes,
            [Type17CommonDyingProductionOutcome::Blocked {
                entity_id: target_id,
                reason: Type17CommonDyingProductionBlock::SchedulerCallbackDisabled,
            }]
        );
        assert_eq!(
            coarse_mutation_snapshot(manager.entity_mut_for_test(target_id).unwrap()),
            before
        );

        let retained_sub_j_runtime = {
            let target = manager.entity_mut_for_test(target_id).unwrap();
            target.collision.state_flags_at_0x08.overwrite(
                REMOTE_OWNED_STATE_BIT
                    | NORMAL_CALLBACK_ENABLED_STATE_BIT
                    | TERRAIN_ATTITUDE_BILINEAR_STATE_BIT,
                NORMAL_CALLBACK_ENABLED_STATE_BIT,
            );
            match std::mem::replace(
                &mut target.sub_j_attachment_runtime,
                RetailRuntimeValue::Unresolved,
            ) {
                RetailRuntimeValue::Known(Some(runtime)) => runtime,
                unexpected => panic!("fixture must retain exact Sub-J runtime: {unexpected:?}"),
            }
        };
        let before = coarse_mutation_snapshot(manager.entity_mut_for_test(target_id).unwrap());
        let pass = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || panic!("unresolved Sub-J must block a coarse frame before shared RNG"),
        );
        assert_eq!(
            pass.outcomes,
            [Type17CommonDyingProductionOutcome::Blocked {
                entity_id: target_id,
                reason: Type17CommonDyingProductionBlock::UnresolvedSubJAttachmentRuntime,
            }]
        );
        assert_eq!(
            coarse_mutation_snapshot(manager.entity_mut_for_test(target_id).unwrap()),
            before
        );

        manager
            .entity_mut_for_test(target_id)
            .unwrap()
            .sub_j_attachment_runtime = RetailRuntimeValue::Known(Some(retained_sub_j_runtime));
        let child_id = manager
            .iter_all()
            .find(|entity| entity.id != target_id && entity.attached_to.is_none())
            .expect("fixture must retain one unattached peer")
            .id;
        manager.entity_mut_for_test(child_id).unwrap().attached_to = Some(target_id);
        manager
            .entity_mut_for_test(target_id)
            .unwrap()
            .collision
            .animation_offset_at_0xb2 = RetailRuntimeValue::Unresolved;
        let pass = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || panic!("an unrelated backlink must not reach shared RNG"),
        );
        assert_eq!(
            pass.outcomes,
            [Type17CommonDyingProductionOutcome::Blocked {
                entity_id: target_id,
                reason: Type17CommonDyingProductionBlock::UnresolvedAnimationOffset,
            }],
            "inverse backlinks are not authoritative Sub-J rows"
        );

        manager.entity_mut_for_test(child_id).unwrap().attached_to = None;
        let target = manager.entity_mut_for_test(target_id).unwrap();
        target.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        let RetailRuntimeValue::Known(Some(runtime)) = &mut target.sub_j_attachment_runtime else {
            panic!("fixture Sub-J runtime was restored")
        };
        runtime.append(child_id).unwrap();
        assert_eq!(
            manager
                .iter_all()
                .find(|entity| entity.id == child_id)
                .unwrap()
                .attached_to,
            None,
            "the mismatch fixture deliberately omits the inverse backlink"
        );
        let pass = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || 0,
        );
        assert!(matches!(
            pass.outcomes.as_slice(),
            [Type17CommonDyingProductionOutcome::Advanced {
                entity_id,
                live_outcome:
                    crate::common_dying_live::Type17CommonDyingLiveFrameOutcome::DeferredDestroyStaged { .. },
            }] if *entity_id == target_id
        ));
        let target = manager.entity_mut_for_test(target_id).unwrap();
        let RetailRuntimeValue::Known(Some(runtime)) = &target.sub_j_attachment_runtime else {
            panic!("fixture Sub-J runtime remains authoritative")
        };
        assert!(runtime.is_empty(), "the exact-zero child row is compacted");
        assert_eq!(scheduler.registered_len(), 0);
    }

    #[test]
    fn detailed_scheduler_returns_the_live_e370_class42_request() {
        let (mut manager, target_id) = exact_manager(1_400);
        let mut terrain = exact_constructor_terrain();
        terrain.header[0] = (0x7fff_i32) << 8;
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let outcome = apply_level_one_type17_projectile_primary_hit(
            &mut manager,
            &test_models(),
            &mut world_fx,
            &mut notifications,
            request(target_id),
        );
        let LevelOneType17PrimaryHitOutcome::Complete(completion) = outcome else {
            panic!("expected completed lethal transaction: {outcome:?}")
        };
        let Some(Type17CommonDyingPublicationOutcome::Published { owner, .. }) =
            completion.common_dying
        else {
            panic!("lethal transaction did not publish Common-Dying")
        };
        let target = manager.entity_mut_for_test(target_id).unwrap();
        target.position = crate::entity::raw_position_world([0; 3]);
        target.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(500);

        let mut scheduler = Type17CommonDyingScheduler::new();
        scheduler.register(owner);
        let mut samples = [0, 0, 0, 0, 0, 0].into_iter();
        let mut draws = 0;
        let pass = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 125_000,
            },
            &mut || {
                draws += 1;
                samples.next().unwrap()
            },
        );

        assert_eq!(draws, 6, "detailed visits enter E370 without scheduler RNG");
        assert!(matches!(
            pass.outcomes.as_slice(),
            [Type17CommonDyingProductionOutcome::Advanced {
                entity_id,
                live_outcome:
                    crate::common_dying_live::Type17CommonDyingLiveFrameOutcome::Continue { .. },
            }] if *entity_id == target_id
        ));
        let [request] = pass.surface_bubbles.as_slice() else {
            panic!("a zero gate must return one ordered class-42 request")
        };
        assert_eq!(request.owner_entity_id, target_id);
        assert_eq!(
            request.owner_entity_type,
            TYPE17_COMMON_DYING_ENTITY_TYPE as u8
        );
        assert!(!request.suppresses_impact_damage);
        assert_eq!(request.velocity_argument_raw, [0; 3]);
        assert_eq!(
            manager
                .entity_mut_for_test(target_id)
                .unwrap()
                .surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(625)
        );
    }

    #[test]
    fn detailed_exact_surface_expiry_commits_release_then_divisor_two_gate() {
        let (mut manager, target_id) = exact_manager(1_400);
        let mut terrain = exact_constructor_terrain();
        terrain.header[0] = (0x7fff_i32) << 8;
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let outcome = apply_level_one_type17_projectile_primary_hit(
            &mut manager,
            &test_models(),
            &mut world_fx,
            &mut notifications,
            request(target_id),
        );
        let LevelOneType17PrimaryHitOutcome::Complete(completion) = outcome else {
            panic!("expected completed lethal transaction: {outcome:?}")
        };
        let Some(Type17CommonDyingPublicationOutcome::Published { owner, .. }) =
            completion.common_dying
        else {
            panic!("lethal transaction did not publish Common-Dying")
        };
        let target = manager.entity_mut_for_test(target_id).unwrap();
        target.position = crate::entity::raw_position_world([0; 3]);
        target.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(1_875);
        target.collision.default_state_flags_at_0xc8 = RetailRuntimeValue::Known(0xDEAD_BEEF);
        target
            .collision
            .state_flags_at_0x08
            .overwrite(0x0001_0000, 0x0001_0000);
        target.attached_to = Some(0x04AA_00FF);
        let behavior_before = target.current_behavior_context;

        let mut scheduler = Type17CommonDyingScheduler::new();
        scheduler.register(owner);
        let mut draws = 0;
        let pass = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 125_000,
            },
            &mut || {
                draws += 1;
                1
            },
        );

        assert_eq!(draws, 1, "exact expiry reaches the divisor-two miss gate");
        assert!(pass.surface_bubbles.is_empty());
        assert!(matches!(
            pass.outcomes.as_slice(),
            [Type17CommonDyingProductionOutcome::Advanced {
                entity_id,
                live_outcome:
                    crate::common_dying_live::Type17CommonDyingLiveFrameOutcome::Continue { .. },
            }] if *entity_id == target_id
        ));
        let target = manager
            .iter_all()
            .find(|entity| entity.id == target_id)
            .expect("the null release hook and suppressed death path preserve the allocation");
        assert_eq!(
            target.surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(2_000)
        );
        assert_eq!(
            target.collision.default_state_flags_at_0xc8,
            RetailRuntimeValue::Known(
                crate::type17_common_dying_production::LEVEL_ONE_TYPE17_DEFAULT_STATE_FLAGS_RAW
            )
        );
        assert_eq!(target.attached_to, None);
        assert_eq!(
            target.collision.state_flags_at_0x08.masked(0x0001_5800),
            RetailRuntimeValue::Known(0x0000_4800),
            "release clears 0x10000/0x1000, sets 0x800, and preserves selected-model 0x4000"
        );
        assert_eq!(target.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(target.current_behavior_context, behavior_before);
        assert!(target
            .actor_task_state(crate::actor_task_owner::ActorTaskSlot::Primary)
            .is_some());
        assert_eq!(scheduler.registered_len(), 1);
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
    }

    #[test]
    fn missing_relation_owners_materialise_then_retire_dying_type93_row() {
        for expected_relation_owner_id in [None, Some(0x04aa_00ff)] {
            let (mut manager, target_id) = exact_manager(1_400);
            let mut terrain = exact_constructor_terrain();
            for (x, z, height) in [(5, 3, -1), (6, 3, 3), (5, 4, -5), (6, 4, 1)] {
                terrain.cells[x * GRID_SIZE + z].height = height as u8;
            }
            let mut world_fx = WorldFx::new();
            let mut notifications = GameplayNotifications::new();
            let outcome = apply_level_one_type17_projectile_primary_hit(
                &mut manager,
                &test_models(),
                &mut world_fx,
                &mut notifications,
                request(target_id),
            );
            let LevelOneType17PrimaryHitOutcome::Complete(completion) = outcome else {
                panic!("expected completed lethal transaction: {outcome:?}")
            };
            let Some(Type17CommonDyingPublicationOutcome::Published { owner, .. }) =
                completion.common_dying
            else {
                panic!("lethal transaction did not publish Common-Dying")
            };

            let entry_position_raw = [5 * 256 + 128, 700, 3 * 256 + 64];
            let target = manager.entity_mut_for_test(target_id).unwrap();
            target.set_position_raw(entry_position_raw);
            target.attached_to = expected_relation_owner_id;
            target.collision.state_flags_at_0x08.overwrite(
                REMOTE_OWNED_STATE_BIT
                    | ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT
                    | 0x0004_0000
                    | crate::common_mover::type9_surface::ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
                ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT
                    | 0x0004_0000
                    | crate::common_mover::type9_surface::ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
            );
            let active_model = target.model_slots[0];

            let mut scheduler = Type17CommonDyingScheduler::new();
            scheduler.register(owner);
            let mut constructor_draws = 0;
            let pass = scheduler.tick(
                &mut manager,
                Type17CommonDyingProductionFrame {
                    terrain: &terrain,
                    elapsed_micros: 20_000,
                },
                &mut || {
                    constructor_draws += 1;
                    0x1234_5678
                },
            );
            assert_eq!(
                constructor_draws, 1,
                "Type 93's singleton Always choice still consumes one RNG word"
            );
            assert!(matches!(
                pass.outcomes.as_slice(),
                [Type17CommonDyingProductionOutcome::Advanced {
                    entity_id,
                    live_outcome:
                        crate::common_dying_live::Type17CommonDyingLiveFrameOutcome::Continue { .. },
                }] if *entity_id == target_id
            ));

            let proxy = manager
                .iter_all()
                .find(|entity| entity.entity_type == 93)
                .expect("missing-owner recovery tail-appends Type 93");
            let proxy_id = proxy.id;
            assert_eq!(
                proxy.position_raw(),
                [entry_position_raw[0], 9, entry_position_raw[2]]
            );
            assert_eq!(proxy.model_slots, [active_model, None, active_model, None]);
            assert_eq!(proxy.mass_raw, 1);
            assert_eq!(
                proxy.initial_behavior,
                RetailRuntimeValue::Known(Some(crate::entity_behavior::BehaviorSelection {
                    choice_index: 0,
                    program: behavior_program(30).unwrap(),
                }))
            );
            let RetailRuntimeValue::Known(Some(proxy_runtime)) = &proxy.sub_j_attachment_runtime
            else {
                panic!("Type 93 must retain its authored owner-side Sub-J runtime")
            };
            assert_eq!(proxy_runtime.ordered_entity_ids(), [target_id]);
            let target = manager
                .iter_all()
                .find(|entity| entity.id == target_id)
                .unwrap();
            assert_eq!(target.attached_to, Some(proxy_id));
            assert_eq!(
                target.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
                RetailRuntimeValue::Known(DYING_STATE_BIT),
                "the lethal publication remains dying through materialiser attachment"
            );
            assert_eq!(
                target.collision.state_flags_at_0x08.masked(0x2005_1800),
                RetailRuntimeValue::Known(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT),
                "release+reattach clears 0x20000000/0x10000/0x800/0x40000 and retains 0x1000"
            );
            assert!(manager.take_cargo_proxy_events().is_empty());

            // The tail-appended proxy receives its first callback later in this
            // same live-list pass. It emits movement for the +1Y link raise, then
            // clamps to the unraised terrain target. Its following 18640 visit
            // retires the dying child row without a 409030 release callback.
            let mut materialiser_tasks =
                crate::specialized_actor_task_production::SpecializedActorTaskScheduler::new();
            let mut materialiser_notifications =
                crate::gameplay_notifications::GameplayNotifications::new();
            assert!(manager
                .update_late_tail_materialisers(crate::entity::LateTailMaterialiserFrame {
                    elapsed_micros: 20_000,
                    terrain: &terrain,
                    world_fx: &mut world_fx,
                    scheduler: &mut materialiser_tasks,
                    notifications: &mut materialiser_notifications,
                    retail_tick: 0,
                })
                .is_empty());
            assert_eq!(
                manager.take_cargo_proxy_events(),
                vec![CargoProxyEvent::PositionChanged {
                    proxy_id,
                    cargo_id: target_id,
                    position: crate::entity::raw_position_world([
                        entry_position_raw[0],
                        9,
                        entry_position_raw[2],
                    ]),
                    particle_class: 0x33,
                }]
            );
            let proxy = manager
                .iter_all()
                .find(|entity| entity.id == proxy_id)
                .unwrap();
            assert!(matches!(
                &proxy.sub_j_attachment_runtime,
                RetailRuntimeValue::Known(Some(rows)) if rows.is_empty()
            ));
            assert_eq!(
                manager
                    .iter_all()
                    .find(|entity| entity.id == target_id)
                    .unwrap()
                    .attached_to,
                Some(proxy_id),
                "callback-free row retirement leaves child relation custody for its next visit"
            );

            // The next child 12DA0 visit resolves the existing parent but finds
            // no 18A00 membership. Its own 16750 reconciliation releases the
            // relation without 409030's master-motion restoration. No second
            // constructor draw or proxy allocation occurs.
            let mut second_frame_draws = 0;
            let pass = scheduler.tick(
                &mut manager,
                Type17CommonDyingProductionFrame {
                    terrain: &terrain,
                    elapsed_micros: 20_000,
                },
                &mut || {
                    second_frame_draws += 1;
                    0
                },
            );
            assert_eq!(second_frame_draws, 0);
            assert!(matches!(
                pass.outcomes.as_slice(),
                [Type17CommonDyingProductionOutcome::Advanced { entity_id, .. }]
                    if *entity_id == target_id
            ));
            assert_eq!(
                manager
                    .iter_all()
                    .filter(|entity| entity.entity_type == 93)
                    .count(),
                1
            );
            let target = manager
                .iter_all()
                .find(|entity| entity.id == target_id)
                .unwrap();
            assert_eq!(target.attached_to, None);
            assert_eq!(
                target.collision.state_flags_at_0x08.masked(0x2005_1800),
                RetailRuntimeValue::Known(0x0000_0800),
                "empty-row reconciliation releases relation without enabling master motion"
            );
            assert!(manager.take_cargo_proxy_events().is_empty());

            assert!(manager
                .update_late_tail_materialisers(crate::entity::LateTailMaterialiserFrame {
                    elapsed_micros: 20_000,
                    terrain: &terrain,
                    world_fx: &mut world_fx,
                    scheduler: &mut materialiser_tasks,
                    notifications: &mut materialiser_notifications,
                    retail_tick: 1,
                })
                .is_empty());
            for _ in 0..24 {
                assert!(manager
                    .update_late_tail_materialisers(crate::entity::LateTailMaterialiserFrame {
                        elapsed_micros: 20_000,
                        terrain: &terrain,
                        world_fx: &mut world_fx,
                        scheduler: &mut materialiser_tasks,
                        notifications: &mut materialiser_notifications,
                        retail_tick: 2,
                    })
                    .is_empty());
            }
            assert!(manager.iter_all().any(|entity| entity.id == proxy_id));
            assert!(manager
                .update_late_tail_materialisers(crate::entity::LateTailMaterialiserFrame {
                    elapsed_micros: 20_000,
                    terrain: &terrain,
                    world_fx: &mut world_fx,
                    scheduler: &mut materialiser_tasks,
                    notifications: &mut materialiser_notifications,
                    retail_tick: 3,
                })
                .is_empty());
            assert!(!manager.iter_all().any(|entity| entity.id == proxy_id));
            let target = manager
                .iter_all()
                .find(|entity| entity.id == target_id)
                .unwrap();
            assert_eq!(target.attached_to, None);
            assert_eq!(
                target.collision.default_state_flags_at_0xc8,
                RetailRuntimeValue::Known(0x39)
            );
            assert_eq!(
                target.collision.state_flags_at_0x08.masked(0x2005_1800),
                RetailRuntimeValue::Known(0x0000_0800),
                "empty proxy expiry does not invoke FUN_00409030 on the retired child"
            );
            assert!(manager.take_cargo_proxy_events().is_empty());

            let mut post_release_draws = 0;
            let _ = scheduler.tick(
                &mut manager,
                Type17CommonDyingProductionFrame {
                    terrain: &terrain,
                    elapsed_micros: 20_000,
                },
                &mut || {
                    post_release_draws += 1;
                    0
                },
            );
            assert_eq!(post_release_draws, 0);
            assert!(!manager.iter_all().any(|entity| entity.entity_type == 93));
        }
    }

    #[test]
    fn coarse_scheduler_wait_precedes_missing_relation_materialiser() {
        let (mut manager, target_id) = exact_manager(1_400);
        let terrain = exact_constructor_terrain();
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let outcome = apply_level_one_type17_projectile_primary_hit(
            &mut manager,
            &test_models(),
            &mut world_fx,
            &mut notifications,
            request(target_id),
        );
        let LevelOneType17PrimaryHitOutcome::Complete(completion) = outcome else {
            panic!("expected completed lethal transaction: {outcome:?}")
        };
        let Some(Type17CommonDyingPublicationOutcome::Published { owner, .. }) =
            completion.common_dying
        else {
            panic!("lethal transaction did not publish Common-Dying")
        };
        let missing_owner_id = 0x04aa_00ff;
        let target = manager.entity_mut_for_test(target_id).unwrap();
        target.attached_to = Some(missing_owner_id);
        target
            .collision
            .state_flags_at_0x08
            .overwrite(TERRAIN_ATTITUDE_BILINEAR_STATE_BIT, 0);
        target.collision.state_flags_at_0x08.overwrite(
            REMOTE_OWNED_STATE_BIT | ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
        );
        target.collision.state_flags_at_0x08.overwrite(
            crate::common_mover::type9_surface::ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
            0,
        );
        target.position = crate::entity::raw_position_world([
            target.position_raw()[0],
            terrain.sea_level_raw(),
            target.position_raw()[2],
        ]);
        target.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(125);
        target.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(77);

        let mut scheduler = Type17CommonDyingScheduler::new();
        scheduler.register(owner);
        let mut samples = [0x0026, 0x1e27].into_iter();
        let mut draws = 0;
        let pass = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 1_000,
            },
            &mut || {
                draws += 1;
                samples
                    .next()
                    .expect("the wait consumes only scheduler draws")
            },
        );
        assert_eq!(draws, 2);
        assert_eq!(
            pass.outcomes,
            [Type17CommonDyingProductionOutcome::SchedulerWaiting {
                entity_id: target_id,
            }]
        );
        let target = manager
            .iter_all()
            .find(|entity| entity.id == target_id)
            .unwrap();
        assert_eq!(target.attached_to, Some(missing_owner_id));
        assert_eq!(
            target
                .collision
                .state_flags_at_0x08
                .masked(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT),
            RetailRuntimeValue::Known(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT)
        );
        assert!(!manager.iter_all().any(|entity| entity.entity_type == 93));
        assert!(manager.take_cargo_proxy_events().is_empty());
    }

    #[test]
    fn detailed_timeout_runs_dca0_suffix_but_skips_terminal_master_motion() {
        let (mut manager, target_id) = exact_manager(1_400);
        let terrain = exact_constructor_terrain();
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let outcome = apply_level_one_type17_projectile_primary_hit(
            &mut manager,
            &test_models(),
            &mut world_fx,
            &mut notifications,
            request(target_id),
        );
        let LevelOneType17PrimaryHitOutcome::Complete(completion) = outcome else {
            panic!("expected completed lethal transaction: {outcome:?}")
        };
        let Some(Type17CommonDyingPublicationOutcome::Published { owner, .. }) =
            completion.common_dying
        else {
            panic!("lethal transaction did not publish Common-Dying")
        };
        let mut scheduler = Type17CommonDyingScheduler::new();
        scheduler.register(owner);
        manager
            .entity_mut_for_test(target_id)
            .expect("published Common-Dying target remains live")
            .collision
            .state_flags_at_0x08
            .overwrite(
                crate::common_mover::type9_surface::ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
                crate::common_mover::type9_surface::ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
            );

        for _ in 0..72 {
            let pass = scheduler.tick(
                &mut manager,
                Type17CommonDyingProductionFrame {
                    terrain: &terrain,
                    elapsed_micros: 125_000,
                },
                &mut || 0,
            );
            assert!(matches!(
                pass.outcomes.as_slice(),
                [Type17CommonDyingProductionOutcome::Advanced {
                    live_outcome:
                        crate::common_dying_live::Type17CommonDyingLiveFrameOutcome::Continue { .. },
                    ..
                }]
            ));
        }

        let target = manager
            .entity_mut_for_test(target_id)
            .expect("exact-timeout Common-Dying target remains live");
        let Some(crate::actor_task_dispatcher::ActorTaskRuntime::CommonDying(task)) =
            target.actor_task_state(crate::actor_task_owner::ActorTaskSlot::Primary)
        else {
            panic!("Common-Dying task was not live at exact timeout")
        };
        assert_eq!(task.elapsed_ms(), 9_000);
        let terminal_position_raw = [0x1200, terrain.sea_level_raw(), -0x2300];
        target.position = crate::entity::raw_position_world(terminal_position_raw);
        target.set_velocity_raw([12_000, 1_000, -8_000]);
        target.collision.state_flags_at_0x08.overwrite(
            crate::common_mover::type9_surface::ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
            0,
        );
        target.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(333);
        target.collision.state_flags_at_0x08.overwrite(0x20, 0);
        target
            .collision
            .state_flags_at_0x08
            .invalidate(crate::common_mover::type9_tail::COMMON_MASTER_MOTION_REQUIRED_STATE_MASK);

        let pass = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 1_000,
            },
            &mut || 0,
        );
        assert!(matches!(
            pass.outcomes.as_slice(),
            [Type17CommonDyingProductionOutcome::Advanced {
                entity_id,
                live_outcome:
                    crate::common_dying_live::Type17CommonDyingLiveFrameOutcome::DeferredDestroyStaged {
                        callback_evidence:
                            crate::common_dying_live::Type17CommonDyingCallbackEvidence::Detailed {
                                selected_effect_raw: 0,
                            },
                        ..
                    },
            }] if *entity_id == target_id
        ));
        let [Type17CommonDyingProductionOutcome::Advanced {
            live_outcome:
                crate::common_dying_live::Type17CommonDyingLiveFrameOutcome::DeferredDestroyStaged {
                    linear_velocity_raw,
                    ..
                },
            ..
        }] = pass.outcomes.as_slice()
        else {
            unreachable!("the detailed terminal shape was asserted above")
        };
        let mut expected_suffix_velocity = *linear_velocity_raw;
        crate::entity::apply_first_world_type17_environment_raw(
            &mut expected_suffix_velocity,
            1_000,
            crate::entity::LEVEL_ONE_TYPE17_SELF_MASS_RAW,
        );
        let staged = manager
            .iter_all()
            .find(|entity| entity.id == target_id)
            .expect("detailed terminal suffix commits before deferred sweep");
        assert_eq!(staged.position_raw()[0], terminal_position_raw[0]);
        assert_eq!(staged.position_raw()[2], terminal_position_raw[2]);
        assert_eq!(staged.velocity_raw(), expected_suffix_velocity);
        assert_eq!(
            staged.collision.state_flags_at_0x08.masked(0x0004_0000),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            staged.collision.state_flags_at_0x08.masked(0x20),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            staged
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT),
            RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
        );
        assert_eq!(
            staged.surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(332)
        );
        assert_eq!(
            staged.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            staged.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(9_001_000)
        );
        assert!(staged
            .actor_task_state(crate::actor_task_owner::ActorTaskSlot::Primary)
            .is_none());
        assert_eq!(scheduler.registered_len(), 0);
        assert_eq!(
            manager.cleanup_pending_actor_deferred_destroys(),
            [target_id]
        );
    }

    #[test]
    fn clear_detailed_update_bit_runs_mode_one_suffix_and_same_tick_sweep() {
        let (mut manager, target_id) = exact_manager(1_400);
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let outcome = apply_level_one_type17_projectile_primary_hit(
            &mut manager,
            &test_models(),
            &mut world_fx,
            &mut notifications,
            request(target_id),
        );
        let LevelOneType17PrimaryHitOutcome::Complete(completion) = outcome else {
            panic!("expected completed lethal transaction: {outcome:?}")
        };
        let Some(Type17CommonDyingPublicationOutcome::Published { owner, .. }) =
            completion.common_dying
        else {
            panic!("lethal transaction did not return its Common-Dying receipt")
        };

        let terrain = exact_constructor_terrain();
        let mut scheduler = Type17CommonDyingScheduler::new();
        scheduler.register(owner);
        let mut detailed_random_draws = 0;
        for _ in 0..56 {
            let pass = scheduler.tick(
                &mut manager,
                Type17CommonDyingProductionFrame {
                    terrain: &terrain,
                    elapsed_micros: 125_000,
                },
                &mut || {
                    detailed_random_draws += 1;
                    0
                },
            );
            assert!(matches!(
                pass.outcomes.as_slice(),
                [Type17CommonDyingProductionOutcome::Advanced {
                    entity_id,
                    live_outcome:
                        crate::common_dying_live::Type17CommonDyingLiveFrameOutcome::Continue {
                            ..
                        },
                }] if *entity_id == target_id
            ));
        }
        assert_eq!(
            detailed_random_draws, 0,
            "the broader-detail state must suppress both common-scheduler draws"
        );

        let relation_parent_id = manager
            .iter_all()
            .find(|entity| entity.id != target_id)
            .expect("fresh Level 1 retains a local relation-parent fixture")
            .id;
        let relation_descriptor = SubJAttachmentDescriptor {
            reserved_at_0x01: 0,
            slots: vec![SubJAttachmentSlotDescriptor {
                policy_word_raw: 1,
                local_offset_raw: [0, 10, 110],
            }]
            .into_boxed_slice(),
        };
        let mut relation_runtime =
            SubJAttachmentRuntime::from_descriptor(&relation_descriptor).unwrap();
        relation_runtime.append(target_id).unwrap();
        manager
            .entity_mut_for_test(relation_parent_id)
            .unwrap()
            .sub_j_attachment_runtime = RetailRuntimeValue::Known(Some(relation_runtime));

        let target = manager
            .entity_mut_for_test(target_id)
            .expect("Common-Dying target remains live");
        let Some(crate::actor_task_dispatcher::ActorTaskRuntime::CommonDying(task)) =
            target.actor_task_state(crate::actor_task_owner::ActorTaskSlot::Primary)
        else {
            panic!("published Common-Dying task was not live")
        };
        assert_eq!(task.elapsed_ms(), 7_000);
        target
            .collision
            .state_flags_at_0x08
            .overwrite(TERRAIN_ATTITUDE_BILINEAR_STATE_BIT, 0);
        target
            .collision
            .state_flags_at_0x08
            .overwrite(BODY_BASIS_REBUILT_STATE_BIT, 0);
        target.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(125);
        target.collision.state_flags_at_0x08.overwrite(
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
        );
        target.attached_to = Some(relation_parent_id);
        let suppressed_before = coarse_mutation_snapshot(target);
        let pass = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 125_000,
            },
            &mut || 0,
        );
        assert!(matches!(
            pass.outcomes.as_slice(),
            [Type17CommonDyingProductionOutcome::Advanced {
                entity_id,
                live_outcome:
                    crate::common_dying_live::Type17CommonDyingLiveFrameOutcome::TransitionSuppressed {
                        callback_evidence:
                            crate::common_dying_live::Type17CommonDyingCallbackEvidence::Coarse {
                                ..
                            },
                        ..
                    },
            }] if *entity_id == target_id
        ));
        let retained = manager
            .iter_all()
            .find(|entity| entity.id == target_id)
            .expect("suppressed coarse transition remains live");
        let Some(crate::actor_task_dispatcher::ActorTaskRuntime::CommonDying(task)) =
            retained.actor_task_state(crate::actor_task_owner::ActorTaskSlot::Primary)
        else {
            panic!("suppressed coarse transition lost the Common-Dying task")
        };
        assert_eq!(task.elapsed_ms(), 7_125);
        assert_ne!(
            coarse_mutation_snapshot(retained),
            suppressed_before,
            "E870 and outer master motion still commit after transition suppression"
        );
        assert_eq!(retained.attached_to, Some(relation_parent_id));
        assert_eq!(
            retained
                .collision
                .state_flags_at_0x08
                .masked(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT),
            RetailRuntimeValue::Known(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT)
        );
        assert_eq!(scheduler.registered_len(), 1);
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());

        let target = manager
            .entity_mut_for_test(target_id)
            .expect("blocked Common-Dying target remains live");
        target
            .collision
            .state_flags_at_0x08
            .invalidate(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT);
        let unresolved_gate_before = coarse_mutation_snapshot(target);
        let pass = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 125_000,
            },
            &mut || panic!("an unresolved transition gate must block before shared RNG"),
        );
        assert_eq!(
            pass.outcomes,
            [Type17CommonDyingProductionOutcome::Blocked {
                entity_id: target_id,
                reason: Type17CommonDyingProductionBlock::UnresolvedNormalSchedulerOwnerState,
            }]
        );
        let retained = manager
            .iter_all()
            .find(|entity| entity.id == target_id)
            .expect("unresolved transition gate remains live");
        assert_eq!(coarse_mutation_snapshot(retained), unresolved_gate_before);
        assert_eq!(scheduler.registered_len(), 1);
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());

        let target = manager
            .entity_mut_for_test(target_id)
            .expect("blocked Common-Dying target remains live");
        target
            .collision
            .state_flags_at_0x08
            .overwrite(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT, 0);
        target.attached_to = None;
        target.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Unresolved;
        let parent = manager.entity_mut_for_test(relation_parent_id).unwrap();
        let RetailRuntimeValue::Known(Some(runtime)) = &mut parent.sub_j_attachment_runtime else {
            panic!("local relation-parent runtime remains live")
        };
        runtime.clear();
        let unresolved_timer_before =
            coarse_mutation_snapshot(manager.entity_mut_for_test(target_id).unwrap());
        let pass = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 125_000,
            },
            &mut || panic!("an unresolved surface timer must block before shared RNG"),
        );
        assert_eq!(
            pass.outcomes,
            [Type17CommonDyingProductionOutcome::Blocked {
                entity_id: target_id,
                reason: Type17CommonDyingProductionBlock::UnresolvedSurfaceLifetimeTimer,
            }]
        );
        let retained = manager
            .iter_all()
            .find(|entity| entity.id == target_id)
            .expect("unresolved timer block remains live");
        assert_eq!(coarse_mutation_snapshot(retained), unresolved_timer_before);
        assert_eq!(scheduler.registered_len(), 1);
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());

        let target = manager
            .entity_mut_for_test(target_id)
            .expect("blocked Common-Dying target remains live");
        target.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(125);
        target
            .collision
            .state_flags_at_0x08
            .invalidate(crate::common_mover::type9_surface::ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT);
        let unresolved_owner_before = coarse_mutation_snapshot(target);
        let pass = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 125_000,
            },
            &mut || panic!("an unresolved surface owner must block before shared RNG"),
        );
        assert_eq!(
            pass.outcomes,
            [Type17CommonDyingProductionOutcome::Blocked {
                entity_id: target_id,
                reason: Type17CommonDyingProductionBlock::UnresolvedSurfaceOwnerState,
            }]
        );
        let retained = manager
            .iter_all()
            .find(|entity| entity.id == target_id)
            .expect("unresolved owner block remains live");
        assert_eq!(coarse_mutation_snapshot(retained), unresolved_owner_before);
        assert_eq!(scheduler.registered_len(), 1);
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());

        let target = manager
            .entity_mut_for_test(target_id)
            .expect("blocked Common-Dying target remains live");
        target.collision.state_flags_at_0x08.overwrite(
            crate::common_mover::type9_surface::ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
            0,
        );
        let original_position_raw = target.position_raw();
        let coarse_position_raw = [
            original_position_raw[0],
            terrain.sea_level_raw(),
            original_position_raw[2],
        ];
        target.position = crate::entity::raw_position_world(coarse_position_raw);
        target.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(125);
        target.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(77);
        let RetailRuntimeValue::Known(recent_elapsed_before_wait) =
            target.collision.recent_relation_elapsed_us_at_0x68
        else {
            panic!("authenticated scheduler relation age must be exact")
        };
        let Some(crate::actor_task_dispatcher::ActorTaskRuntime::CommonDying(task)) =
            target.actor_task_state(crate::actor_task_owner::ActorTaskSlot::Primary)
        else {
            panic!("coarse scheduler fixture lost its Common-Dying task")
        };
        let task_elapsed_before_wait = task.elapsed_ms();
        let mut expected_velocity = target.velocity_raw();
        crate::entity::apply_first_world_type17_environment_raw(
            &mut expected_velocity,
            125_000,
            crate::entity::LEVEL_ONE_TYPE17_SELF_MASS_RAW,
        );

        let mut scheduler_fx = WorldFx::new();
        let mut consumed_scheduler_samples = Vec::new();
        let waiting = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 1_000,
            },
            &mut || {
                let sample = scheduler_fx.next_shared_retail_random_u16();
                consumed_scheduler_samples.push(sample);
                u32::from(sample)
            },
        );
        assert_eq!(
            waiting.outcomes,
            [Type17CommonDyingProductionOutcome::SchedulerWaiting {
                entity_id: target_id,
            }]
        );
        let waiting_entity = manager
            .iter_all()
            .find(|entity| entity.id == target_id)
            .expect("a callback-gate wait must retain the entity");
        let Some(crate::actor_task_dispatcher::ActorTaskRuntime::CommonDying(waiting_task)) =
            waiting_entity.actor_task_state(crate::actor_task_owner::ActorTaskSlot::Primary)
        else {
            panic!("a callback-gate wait must retain the task")
        };
        assert_eq!(waiting_task.elapsed_ms(), task_elapsed_before_wait);
        assert_eq!(
            waiting_entity
                .collision
                .callback_scheduler_accumulator_us_at_0x6c,
            RetailRuntimeValue::Known(1_000)
        );
        assert_eq!(
            waiting_entity.collision.subject_scan_gate_at_0x70,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            waiting_entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(recent_elapsed_before_wait)
        );
        assert_eq!(
            waiting_entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(scheduler.registered_len(), 1);
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());

        let pass = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 124_001,
            },
            &mut || {
                let sample = scheduler_fx.next_shared_retail_random_u16();
                consumed_scheduler_samples.push(sample);
                u32::from(sample)
            },
        );
        assert_eq!(consumed_scheduler_samples, [0x0026, 0x1e27, 0xd2f6]);
        let mut scheduler_rng_oracle = 0;
        for sample in consumed_scheduler_samples {
            assert_eq!(sample, retail_random_u16(&mut scheduler_rng_oracle));
        }
        assert_eq!(
            scheduler_fx.next_shared_retail_random_u16(),
            retail_random_u16(&mut scheduler_rng_oracle),
            "the wait plus continuation consume exactly three shared words"
        );
        assert!(matches!(
            pass.outcomes.as_slice(),
            [Type17CommonDyingProductionOutcome::Advanced {
                entity_id,
                live_outcome:
                    crate::common_dying_live::Type17CommonDyingLiveFrameOutcome::DeferredDestroyStaged {
                        callback_evidence:
                            crate::common_dying_live::Type17CommonDyingCallbackEvidence::Coarse {
                                tagged_result,
                            },
                        ..
                    },
            }] if *entity_id == target_id
                && tagged_result.singleton_address == 0x004B_E170
                && tagged_result.tag == 0x9C01
        ));
        let staged = manager
            .iter_all()
            .find(|entity| entity.id == target_id)
            .expect("E870 suffix commits before the deferred sweep");
        assert_eq!(staged.position_raw(), coarse_position_raw);
        assert_eq!(staged.velocity_raw(), expected_velocity);
        assert_eq!(
            staged.collision.callback_scheduler_accumulator_us_at_0x6c,
            RetailRuntimeValue::Known(1)
        );
        assert_eq!(
            staged.collision.subject_scan_gate_at_0x70,
            RetailRuntimeValue::Known(124_001)
        );
        assert_eq!(
            staged.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(recent_elapsed_before_wait.wrapping_add(125_000))
        );
        assert_eq!(
            staged
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT),
            RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
        );
        assert_eq!(
            staged.surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(0)
        );
        assert!(staged
            .actor_task_state(crate::actor_task_owner::ActorTaskSlot::Primary)
            .is_none());
        assert_eq!(scheduler.registered_len(), 0);
        assert_eq!(
            manager.cleanup_pending_actor_deferred_destroys(),
            [target_id]
        );
        assert!(manager.iter_all().all(|entity| entity.id != target_id));
    }

    #[test]
    fn capture_people_cleanup_blocks_after_the_tick_without_replaying_damage() {
        let (mut manager, target_id) = exact_manager(5_000);
        manager
            .entity_mut_for_test(target_id)
            .unwrap()
            .current_behavior_context = RetailRuntimeValue::Known(Some(named_context(9, 2)));
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();

        let outcome = apply_level_one_type17_projectile_primary_hit(
            &mut manager,
            &test_models(),
            &mut world_fx,
            &mut notifications,
            request(target_id),
        );

        assert!(matches!(
            outcome,
            LevelOneType17PrimaryHitOutcome::CommittedPrefixBlocked {
                phase: LevelOneType17PrimaryHitPhase::Primary(PrimaryHitPhase::TypeImpactCallback),
                reason: LevelOneType17PrimaryHitBlockReason::CapturePeopleCleanup(_),
            }
        ));
        let target = manager
            .iter_all()
            .find(|entity| entity.id == target_id)
            .unwrap();
        assert_eq!(
            target.collision.health_raw,
            RetailRuntimeValue::Known(5_000)
        );
        assert_eq!(
            target.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(300)
        );
        assert_eq!(world_fx.particle_count(), 0);
        assert!(world_fx.take_positional_sounds().is_empty());
    }

    #[test]
    fn checked_child_block_retires_the_nested_parent_receipt_after_committed_reaction() {
        let (mut manager, target_id) = exact_manager(5_000);
        let initial_rotation = manager
            .iter_all()
            .find(|entity| entity.id == target_id)
            .unwrap()
            .rotation_heading_pitch_roll_raw();
        let mut zero_damage = request(target_id);
        zero_damage.delivery.packet = DamagePacket {
            channels: [2, 0],
            amounts_raw: [0, 0],
        };
        let mut world_fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();

        let outcome = apply_level_one_type17_projectile_primary_hit(
            &mut manager,
            &test_models(),
            &mut world_fx,
            &mut notifications,
            zero_damage,
        );

        assert_eq!(
            outcome,
            LevelOneType17PrimaryHitOutcome::CommittedPrefixBlocked {
                phase: LevelOneType17PrimaryHitPhase::CheckedDamage(
                    CheckedDamagePhase::SelectZeroDamageFeedback,
                ),
                reason: LevelOneType17PrimaryHitBlockReason::UnsupportedCheckedAction(
                    "feedback selector",
                ),
            }
        );
        let target = manager
            .iter_all()
            .find(|entity| entity.id == target_id)
            .unwrap();
        assert_eq!(
            target.collision.health_raw,
            RetailRuntimeValue::Known(5_000)
        );
        assert_ne!(target.rotation_heading_pitch_roll_raw(), initial_rotation);
        assert_eq!(
            target.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(300)
        );
        assert_eq!(world_fx.particle_count(), 0);
    }
}
