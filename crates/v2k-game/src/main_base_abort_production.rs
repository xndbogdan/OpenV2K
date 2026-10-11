//! Synchronous production owner for the shared campaign abort.
//!
//! Retail runs the nested actor walk in live intrusive-list order, including
//! Type-60 actors appended by Type-61 callbacks, before transforming terrain
//! and submitting the replacement full-frame request. This adapter keeps that
//! ordering inside one call. Newly published actor-task owners are adopted by
//! [`SpecializedActorTaskScheduler`] but are deliberately not ticked here;
//! retail's task pass has already completed for the aborting frame.

mod fallback;
mod native;
pub use fallback::{MainBaseAbortFallbackReport, MainBaseAbortFallbackRequest};
pub use native::NativeMainBaseAbortDeathBlock;

use crate::common_dying_live::LevelOneType17CommonDyingOwner;
use crate::entity::{
    Class0ActorOwner, EntityManager, MainBaseAbortAlternateCleanupAdvance,
    MainBaseAbortAlternateCleanupBlock, CARGO_DROP_PROXY_ENTITY_TYPE,
};
use crate::entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT};
use crate::main_base_abort::{
    MainBaseAbortAction, MainBaseAbortActorObservation, MainBaseAbortActorRoute,
    MainBaseAbortBlock, MainBaseAbortCompletion, MainBaseAbortControllerLeaseMismatch,
    MainBaseAbortControllerStorage, MainBaseAbortExternalBlock, MainBaseAbortFrameRequest,
    MainBaseAbortMachine, MainBaseAbortPhase, MainBaseAbortPlanningBlock, MainBaseAbortPoll,
    MainBaseAbortProtocolError, MainBaseAbortQuietDeathAdvance, MainBaseAbortQuietDeathBlock,
    MainBaseAbortResume, MainBaseAbortTerminalSelfAdvance, MainBaseAbortTerminalSelfBlock,
    MainBaseAbortTransactionId, MainBaseTerminalAbortOrigin, MainBaseType93DeathAdvance,
    MainBaseType93DeathBlock,
};
use crate::main_base_abort_world_effects::{
    snapshot_main_base_abort_terrain_geometry, MainBaseAbortWorldEffects,
    MainBaseAbortWorldEffectsConstructorBlock, MainBaseAbortWorldEffectsReport,
};
use crate::main_base_type17_abort::{
    MainBaseType17DeathAdvance, MainBaseType17DeathBlock, MainBaseType17DeathOutcome,
};
use crate::main_base_type47_abort::{
    MainBaseType47DeathAdvance, MainBaseType47DeathBlock, MainBaseType47DeathOutcome,
};
use crate::main_base_type54_abort::{
    MainBaseType54DeathAdvance, MainBaseType54DeathBlock, MainBaseType54DeathOutcome,
    MainBaseType54NetworkSession, MainBaseType54SeaLevelTickReceipt, LEVEL_ONE_TYPE54_ENTITY_TYPE,
    LEVEL_ONE_TYPE54_MODEL_EXTENT_RAW,
};
use crate::main_base_type61_abort::{
    MainBaseType60RingDeathAdvance, MainBaseType60RingDeathBlock, MainBaseType60RingDeathOutcome,
    MainBaseType61CapturedSpawn, MainBaseType61DeathAdvance, MainBaseType61DeathBlock,
    MainBaseType61DeathOutcome, MainBaseType61DeathRequest, MainBaseType61LogicalOwner,
    MainBaseType61NetworkSession, LEVEL_ONE_TYPE61_DEFAULT_MODEL_EXTENT_RAW,
    LEVEL_ONE_TYPE61_ENTITY_TYPE,
};
use crate::main_base_type66_abort::{
    MainBaseType66DeathAdvance, MainBaseType66DeathBlock, MainBaseType66DeathOutcome,
    LEVEL_ONE_TYPE66_ENTITY_TYPE,
};
use crate::main_base_type66_production::MainBaseType66ProductionOwner;
use crate::main_base_type9_abort::{
    MainBaseType9DeathAdvance, MainBaseType9DeathBlock, MainBaseType9DeathOutcome,
    MainBaseType9ResultScreenState, LEVEL_ONE_TYPE9_ENTITY_TYPE,
};
use crate::main_base_type9_actor_production::MainBaseType9ActorProductionOwner;
use crate::ordinary_type47_death_live::{
    FreshLevelOneType47CommonDyingOwner, TYPE47_COMMON_DYING_ENTITY_TYPE,
};
use crate::ordinary_type9_initial_production::FreshLevel1Type9InitialProductionOwner;
use crate::player_hull::PlayerHull;
use crate::resource_cache::{MainBaseAbortTerrainOutcome, ResourceCache};
use crate::specialized_actor_task_production::{
    OrdinaryType9SelectedCustodyTakeBlock, OrdinaryType9SelectedProductionOwner,
    OrdinaryType9SelectedProductionResume, SpecializedActorTaskRegistrationConflict,
    SpecializedActorTaskScheduler, Type60ExplodingRingCustodyTakeBlock,
};
use crate::static_damage::StaticDamageScheduler;
use crate::type17_impact_reselection::TYPE17_IMPACT_ENTITY_TYPE;
use crate::type60_exploding_ring::{Type60ConstructionOutcome, TYPE60_RING_ENTITY_TYPE};
use crate::type60_exploding_ring_production::Type60ExplodingRingProductionOwner;
use crate::world_fx::WorldFx;

const MAIN_BASE_ENTITY_TYPE: u32 = 6;
const QUIET_DEATH_ENTITY_TYPES: [u32; 3] = [52, 62, 68];

/// One actor whose exact callback (or callback-free route) completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseAbortProcessedActor {
    pub observation: MainBaseAbortActorObservation,
    pub route: MainBaseAbortActorRoute,
    pub disposition: MainBaseAbortActorDisposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseAbortActorDisposition {
    NoCallback,
    AlternateCleanup,
    TerminalMainBaseNoOp,
    MainBaseDeath,
    QuietDeath,
    Type93Death,
    Type8Death,
    Type123Death,
    Type86Death,
    Type9Death,
    Type17Death,
    Type26Death,
    Type47Death,
    Type53Death,
    Type58Death,
    Type122Death,
    Type18Death,
    Type28Death,
    Type76Death,
    /// Ordinary Type10-family rows: alternate class11 publishes Tumble.
    Type10Death,
    Class49Death,
    Type54Death,
    Type60RingDeath,
    Type61Death,
    Type66Death,
}

/// New owners published synchronously by actor callbacks and adopted for the
/// next frame's heterogeneous task pass.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MainBaseAbortPublicationCounts {
    pub type8_exploding: usize,
    pub type123_exploding: usize,
    pub type86_exploding: usize,
    pub type9_exploding: usize,
    pub type17_common_dying: usize,
    pub type26_common_dying: usize,
    pub type47_common_dying: usize,
    pub type53_common_dying: usize,
    pub type58_common_dying: usize,
    pub type122_common_dying: usize,
    pub type18_common_dying: usize,
    pub type28_common_dying: usize,
    pub type76_common_dying: usize,
    pub type10_tumble: usize,
    pub type54_sea_level: usize,
    pub type66_production: usize,
    pub main_base_production: usize,
    pub appended_type60_actors: usize,
    /// Class63 Type61 drops appended behind the live-list cursor.
    pub appended_type61_actors: usize,
    /// Deliberate same-family replacement of a stale pre-callback owner. In
    /// particular, the normal spawn-23 Type-66 owner is replaced when its
    /// Primary task is republished by the abort callback.
    pub same_family_replacements: usize,
}

impl MainBaseAbortPublicationCounts {
    pub const fn specialized_total(self) -> usize {
        self.type8_exploding
            + self.type123_exploding
            + self.type86_exploding
            + self.type9_exploding
            + self.type17_common_dying
            + self.type26_common_dying
            + self.type47_common_dying
            + self.type53_common_dying
            + self.type58_common_dying
            + self.type122_common_dying
            + self.type18_common_dying
            + self.type28_common_dying
            + self.type76_common_dying
            + self.type10_tumble
            + self.type54_sea_level
            + self.type66_production
            + self.main_base_production
    }
}

/// Complete owned result after actor, terrain, player, and frame-request
/// phases have all been acknowledged by [`MainBaseAbortMachine`].
#[derive(Debug)]
pub struct MainBaseAbortProductionReport {
    pub completion: MainBaseAbortCompletion,
    /// `None` is retail's accepted zero/null terrain-transform return. It does
    /// not suppress the controller handoff.
    pub terrain: Option<MainBaseAbortTerrainOutcome>,
    pub world_effects: MainBaseAbortWorldEffectsReport,
    pub frame_request: MainBaseAbortFrameRequest,
    pub processed_actors: Vec<MainBaseAbortProcessedActor>,
    pub publications: MainBaseAbortPublicationCounts,
}

/// Linear authority returned when a speculative manager/scheduler transfer or
/// task adoption cannot complete without losing or crossing custody.
#[derive(Debug)]
pub enum MainBaseAbortUnscheduledOwner {
    Type9(MainBaseType9ActorProductionOwner),
    OrdinaryType9Class14(crate::main_base_type9_production::MainBaseType9ExplodingProductionOwner),
    OrdinaryType9Initial(FreshLevel1Type9InitialProductionOwner),
    OrdinaryType9Selected(OrdinaryType9SelectedProductionOwner),
    Type17(LevelOneType17CommonDyingOwner),
    Type47(FreshLevelOneType47CommonDyingOwner),
    Type54(MainBaseType54SeaLevelTickReceipt),
    Type60(Type60ExplodingRingProductionOwner),
    Type66(MainBaseType66ProductionOwner),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MainBaseAbortActorCallbackBlock {
    Native(NativeMainBaseAbortDeathBlock),
    NoCallback(crate::main_base_abort::MainBaseAbortNoCallbackBlock),
    AlternateCleanup(MainBaseAbortAlternateCleanupBlock),
    TerminalSelf(MainBaseAbortTerminalSelfBlock),
    QuietDeath(MainBaseAbortQuietDeathBlock),
    Type93(MainBaseType93DeathBlock),
    OrdinaryType9SelectedCustody(OrdinaryType9SelectedCustodyTakeBlock),
    Type9(MainBaseType9DeathBlock),
    Type17(MainBaseType17DeathBlock),
    Type47(MainBaseType47DeathBlock),
    Type54(MainBaseType54DeathBlock),
    Type60Custody(Type60ExplodingRingCustodyTakeBlock),
    Type60(MainBaseType60RingDeathBlock),
    Type61(MainBaseType61DeathBlock),
    Type61Request(MainBaseType61RequestBlock),
    Type66(MainBaseType66DeathBlock),
    UnsupportedOrdinaryType { entity_type: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType61RequestBlock {
    EntityMissing,
    UnauthenticatedSpawn { actual: Option<usize> },
    LogicalOwnerUnresolved,
    LogicalOwnerAmbiguous { relation_id: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MainBaseAbortProductionDiagnostic {
    WorldEffects(MainBaseAbortWorldEffectsConstructorBlock),
    PlanningBlocked(MainBaseAbortPlanningBlock),
    MachineBlocked(MainBaseAbortBlock),
    MachineProtocol(MainBaseAbortProtocolError),
    UnexpectedMachinePhase {
        expected: MainBaseAbortPhase,
        actual: MainBaseAbortPhase,
    },
    UnexpectedCompletion(MainBaseAbortCompletion),
    TerminalOriginNotConsumed,
    ActorCallbackBlocked {
        actor: MainBaseAbortActorObservation,
        error: MainBaseAbortActorCallbackBlock,
    },
    ActorSuccessorUnavailableAfterCommit {
        actor: MainBaseAbortActorObservation,
    },
    ActorVisitLimitExceeded {
        limit: usize,
        actor: MainBaseAbortActorObservation,
    },
    SpecializedOwnerConflict(SpecializedActorTaskRegistrationConflict),
    OrdinaryType9SelectedSidecarRestoreConflict {
        entity_id: u32,
    },
    OrdinaryType9SelectedOwnerMissingAfterNoOp {
        entity_id: u32,
    },
    Class0OwnerMissingAfterType54NoOp {
        entity_id: u32,
    },
    OrdinaryType9SelectedOwnerReconstitutionFailed {
        entity_id: u32,
    },
    OrdinaryType9SelectedOwnerRestoreReplacedExisting {
        entity_id: u32,
    },
    Type60OwnerRestoreReplacedExisting {
        entity_id: u32,
    },
    ControllerHandoff(MainBaseAbortControllerLeaseMismatch),
    SubmittedFrameMismatch {
        expected: MainBaseAbortFrameRequest,
        actual: MainBaseAbortFrameRequest,
    },
}

#[derive(Debug, Default)]
pub struct MainBaseAbortProductionProgress {
    pub processed_actors: Vec<MainBaseAbortProcessedActor>,
    pub publications: MainBaseAbortPublicationCounts,
    pub world_effects: Option<MainBaseAbortWorldEffectsReport>,
    pub terrain: Option<MainBaseAbortTerrainOutcome>,
    pub frame_request: Option<MainBaseAbortFrameRequest>,
}

/// A fail-closed result from the atomic production boundary.
///
/// Every returned failure restores the admitted world exactly, returns the
/// original optional terminal origin, and reports only committed-state-neutral
/// progress. `unscheduled_owner` therefore remains `None` for this public
/// entry point; the field is retained for the private speculative body's
/// custody diagnostics.
#[derive(Debug)]
pub struct MainBaseAbortProductionFailure {
    pub terminal_origin: Option<MainBaseTerminalAbortOrigin>,
    pub diagnostic: MainBaseAbortProductionDiagnostic,
    pub progress: MainBaseAbortProductionProgress,
    pub unscheduled_owner: Option<MainBaseAbortUnscheduledOwner>,
}

struct ActorPhaseFailure {
    origin: Option<MainBaseTerminalAbortOrigin>,
    diagnostic: MainBaseAbortProductionDiagnostic,
    progress: MainBaseAbortProductionProgress,
    unscheduled_owner: Option<MainBaseAbortUnscheduledOwner>,
}

struct ActorPhaseSuccess {
    machine: MainBaseAbortMachine,
    terrain_receipt: crate::main_base_abort::MainBaseAbortReceipt,
    origin: Option<MainBaseTerminalAbortOrigin>,
    progress: MainBaseAbortProductionProgress,
}

struct DispatchSuccess {
    disposition: MainBaseAbortActorDisposition,
    successor: Option<MainBaseAbortActorObservation>,
    successor_available: bool,
}

struct DispatchFailure {
    callback_error: Option<MainBaseAbortActorCallbackBlock>,
    diagnostic: Option<MainBaseAbortProductionDiagnostic>,
    unscheduled_owner: Option<MainBaseAbortUnscheduledOwner>,
}

/// Current controller context used by synchronous child-release callbacks.
/// The abort's speculative transaction owns notification rollback too.
pub struct MainBaseAbortGameplayContext<'a> {
    pub notifications: &'a mut crate::gameplay_notifications::GameplayNotifications,
    pub retail_tick: u32,
    pub extra_lives: RetailRuntimeValue<u8>,
}

/// Execute the complete admitted native or retained-Level-1 abort body synchronously.
/// Main Base destruction supplies its terminal origin; an authored campaign
/// loss has no terminal actor and passes `None`. Both enter `00456960` and
/// perform the same `0042F1A0` actor/terrain/controller transaction.
///
/// This call performs no deferred-destroy cleanup and no specialized task
/// tick. Every callback sees the successor sampled from the mutated live list,
/// so the Type-60 ring tails appended by Type-61 remain part of this sweep.
#[allow(clippy::too_many_arguments)]
pub fn execute_campaign_abort(
    transaction_id: MainBaseAbortTransactionId,
    origin: Option<MainBaseTerminalAbortOrigin>,
    controller: &mut MainBaseAbortControllerStorage,
    entities: &mut EntityManager,
    resources: &mut ResourceCache,
    static_damage: &mut StaticDamageScheduler,
    player_hull: &mut PlayerHull,
    world_fx: &mut WorldFx,
    specialized_tasks: &mut SpecializedActorTaskScheduler,
    gameplay: MainBaseAbortGameplayContext<'_>,
) -> Result<MainBaseAbortProductionReport, MainBaseAbortProductionFailure> {
    // These explicitly named forks are not general clones.  The live owners
    // remain parked and inaccessible until the complete speculative body has
    // either succeeded or been discarded.
    let speculative_origin = origin
        .as_ref()
        .map(MainBaseTerminalAbortOrigin::fork_for_main_base_abort_transaction);
    let mut speculative_controller = *controller;
    let mut speculative_entities = entities.fork_for_main_base_abort_transaction();
    let mut speculative_static_damage = static_damage.fork_for_main_base_abort_transaction();
    let mut speculative_player_hull = *player_hull;
    let mut speculative_world_fx = world_fx.fork_for_main_base_abort_transaction();
    let mut speculative_specialized_tasks =
        specialized_tasks.fork_for_main_base_abort_transaction();
    let mut speculative_notifications = gameplay.notifications.clone();
    let mut resource_transaction = resources.begin_main_base_abort_resource_transaction();

    let attempt = execute_campaign_abort_body(
        transaction_id,
        speculative_origin,
        &mut speculative_controller,
        &mut speculative_entities,
        &mut resource_transaction,
        &mut speculative_static_damage,
        &mut speculative_player_hull,
        &mut speculative_world_fx,
        &mut speculative_specialized_tasks,
        &mut MainBaseAbortGameplayContext {
            notifications: &mut speculative_notifications,
            retail_tick: gameplay.retail_tick,
            extra_lives: gameplay.extra_lives,
        },
    );

    match attempt {
        Ok(report) => {
            // Swaps cannot fail and keep each manager/scheduler identity pair
            // coherent.  Commit the resource guard only after every other
            // live owner contains its accepted speculative state.
            std::mem::swap(controller, &mut speculative_controller);
            std::mem::swap(entities, &mut speculative_entities);
            std::mem::swap(static_damage, &mut speculative_static_damage);
            std::mem::swap(player_hull, &mut speculative_player_hull);
            std::mem::swap(world_fx, &mut speculative_world_fx);
            std::mem::swap(specialized_tasks, &mut speculative_specialized_tasks);
            std::mem::swap(gameplay.notifications, &mut speculative_notifications);
            resource_transaction.commit();
            drop(origin);
            Ok(report)
        }
        Err(speculative_failure) => {
            let diagnostic = speculative_failure.diagnostic.clone();
            // Drop every speculative receipt/proof before returning the
            // canonical terminal origin.  Dropping the resource guard restores
            // the parked terrain and one-shot latch, including during unwind.
            drop(speculative_failure);
            drop(resource_transaction);
            Err(MainBaseAbortProductionFailure {
                terminal_origin: origin,
                diagnostic,
                progress: MainBaseAbortProductionProgress::default(),
                unscheduled_owner: None,
            })
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_campaign_abort_body(
    transaction_id: MainBaseAbortTransactionId,
    origin: Option<MainBaseTerminalAbortOrigin>,
    controller: &mut MainBaseAbortControllerStorage,
    entities: &mut EntityManager,
    resources: &mut ResourceCache,
    static_damage: &mut StaticDamageScheduler,
    player_hull: &mut PlayerHull,
    world_fx: &mut WorldFx,
    specialized_tasks: &mut SpecializedActorTaskScheduler,
    gameplay: &mut MainBaseAbortGameplayContext<'_>,
) -> Result<MainBaseAbortProductionReport, MainBaseAbortProductionFailure> {
    let first_actor = entities.first_main_base_abort_actor_observation();
    let machine = MainBaseAbortMachine::start(
        transaction_id,
        RetailRuntimeValue::Known(Some(controller.world_control_lease())),
        RetailRuntimeValue::Known(first_actor),
    );
    let geometry = match snapshot_main_base_abort_terrain_geometry(resources) {
        Ok(geometry) => geometry,
        Err(error) => {
            return Err(MainBaseAbortProductionFailure {
                terminal_origin: origin,
                diagnostic: MainBaseAbortProductionDiagnostic::WorldEffects(error),
                progress: MainBaseAbortProductionProgress::default(),
                unscheduled_owner: None,
            })
        }
    };
    let mut effects = match MainBaseAbortWorldEffects::new(&geometry, resources, static_damage) {
        Ok(effects) => effects,
        Err(error) => {
            return Err(MainBaseAbortProductionFailure {
                terminal_origin: origin,
                diagnostic: MainBaseAbortProductionDiagnostic::WorldEffects(error),
                progress: MainBaseAbortProductionProgress::default(),
                unscheduled_owner: None,
            })
        }
    };

    let actor_phase = run_actor_phase(
        machine,
        origin,
        entities,
        geometry.terrain(),
        player_hull,
        world_fx,
        &mut effects,
        specialized_tasks,
        gameplay,
    );
    let world_effects = effects.finish();
    let ActorPhaseSuccess {
        mut machine,
        terrain_receipt,
        origin,
        mut progress,
    } = match actor_phase {
        Ok(success) => success,
        Err(mut failure) => {
            failure.progress.world_effects = Some(world_effects);
            return Err(MainBaseAbortProductionFailure {
                terminal_origin: failure.origin,
                diagnostic: failure.diagnostic,
                progress: failure.progress,
                unscheduled_owner: failure.unscheduled_owner,
            });
        }
    };
    debug_assert!(origin.is_none());
    progress.world_effects = Some(world_effects);

    // FUN_00433E30's null/zero return is explicitly nonblocking: retail
    // continues to the player handoff in both cases.
    let terrain = resources
        .apply_main_base_abort_terrain_transform(|| world_fx.next_shared_retail_random_u16());
    progress.terrain = terrain;
    if let Err(failure) = machine.resume(
        terrain_receipt,
        MainBaseAbortResume::TerrainTransformed {
            phase: MainBaseAbortPhase::TerrainTransform,
        },
    ) {
        return Err(production_failure(
            None,
            MainBaseAbortProductionDiagnostic::MachineProtocol(failure.error),
            progress,
            None,
        ));
    }

    let issued = match poll_action(&mut machine, MainBaseAbortPhase::PlayerHandoff) {
        Ok(issued) => issued,
        Err(diagnostic) => return Err(production_failure(None, diagnostic, progress, None)),
    };
    let MainBaseAbortAction::CommitPlayerHandoff {
        player_state,
        world_control,
        ..
    } = issued.action
    else {
        unreachable!("phase-authenticated player action")
    };
    let request = match controller.commit_post_terrain_abort(world_control) {
        Ok(request) => request,
        Err(error) => {
            let diagnostic = MainBaseAbortProductionDiagnostic::ControllerHandoff(error);
            if let Err(failure) = machine.resume(
                issued.receipt,
                MainBaseAbortResume::Blocked {
                    phase: MainBaseAbortPhase::PlayerHandoff,
                    reason: MainBaseAbortExternalBlock::PlayerHandoffUnavailable,
                },
            ) {
                return Err(production_failure(
                    None,
                    MainBaseAbortProductionDiagnostic::MachineProtocol(failure.error),
                    progress,
                    None,
                ));
            }
            return Err(production_failure(None, diagnostic, progress, None));
        }
    };
    progress.frame_request = Some(request);
    if let Err(failure) = machine.resume(
        issued.receipt,
        MainBaseAbortResume::PlayerHandoffCommitted {
            phase: MainBaseAbortPhase::PlayerHandoff,
            player_state,
            world_control: controller.world_control(),
            request,
        },
    ) {
        return Err(production_failure(
            None,
            MainBaseAbortProductionDiagnostic::MachineProtocol(failure.error),
            progress,
            None,
        ));
    }

    let issued = match poll_action(&mut machine, MainBaseAbortPhase::FullFrameRequest) {
        Ok(issued) => issued,
        Err(diagnostic) => return Err(production_failure(None, diagnostic, progress, None)),
    };
    let MainBaseAbortAction::SubmitFullFrame {
        request: expected, ..
    } = issued.action
    else {
        unreachable!("phase-authenticated full-frame action")
    };
    let actual = controller.submitted_frame_request();
    if actual != expected {
        let diagnostic =
            MainBaseAbortProductionDiagnostic::SubmittedFrameMismatch { expected, actual };
        if let Err(failure) = machine.resume(
            issued.receipt,
            MainBaseAbortResume::Blocked {
                phase: MainBaseAbortPhase::FullFrameRequest,
                reason: MainBaseAbortExternalBlock::FullFrameSubmissionUnavailable,
            },
        ) {
            return Err(production_failure(
                None,
                MainBaseAbortProductionDiagnostic::MachineProtocol(failure.error),
                progress,
                None,
            ));
        }
        return Err(production_failure(None, diagnostic, progress, None));
    }
    if let Err(failure) = machine.resume(
        issued.receipt,
        MainBaseAbortResume::FullFrameSubmitted {
            phase: MainBaseAbortPhase::FullFrameRequest,
            request: actual,
        },
    ) {
        return Err(production_failure(
            None,
            MainBaseAbortProductionDiagnostic::MachineProtocol(failure.error),
            progress,
            None,
        ));
    }
    let completion = match machine.poll() {
        MainBaseAbortPoll::Complete(completion) => completion,
        MainBaseAbortPoll::PlanningBlocked(error) => {
            return Err(production_failure(
                None,
                MainBaseAbortProductionDiagnostic::PlanningBlocked(error),
                progress,
                None,
            ))
        }
        MainBaseAbortPoll::Blocked(error) => {
            return Err(production_failure(
                None,
                MainBaseAbortProductionDiagnostic::MachineBlocked(error),
                progress,
                None,
            ))
        }
        MainBaseAbortPoll::Action(issued) => {
            return Err(production_failure(
                None,
                MainBaseAbortProductionDiagnostic::UnexpectedMachinePhase {
                    expected: MainBaseAbortPhase::FullFrameRequest,
                    actual: issued.action.phase(),
                },
                progress,
                None,
            ))
        }
        MainBaseAbortPoll::Awaiting(actual) => {
            return Err(production_failure(
                None,
                MainBaseAbortProductionDiagnostic::UnexpectedMachinePhase {
                    expected: MainBaseAbortPhase::FullFrameRequest,
                    actual,
                },
                progress,
                None,
            ))
        }
    };
    let world_effects = progress
        .world_effects
        .take()
        .expect("completed actor phase owns its world-effects report");
    Ok(MainBaseAbortProductionReport {
        completion,
        terrain: progress.terrain,
        world_effects,
        frame_request: request,
        processed_actors: progress.processed_actors,
        publications: progress.publications,
    })
}

#[allow(clippy::too_many_arguments)]
fn run_actor_phase(
    mut machine: MainBaseAbortMachine,
    mut origin: Option<MainBaseTerminalAbortOrigin>,
    entities: &mut EntityManager,
    terrain: &v2k_formats::terrain::TerrainGrid,
    player_hull: &mut PlayerHull,
    world_fx: &mut WorldFx,
    effects: &mut MainBaseAbortWorldEffects<'_>,
    specialized_tasks: &mut SpecializedActorTaskScheduler,
    gameplay: &mut MainBaseAbortGameplayContext<'_>,
) -> Result<ActorPhaseSuccess, ActorPhaseFailure> {
    // The admitted walk may visit every actor present at entry
    // plus at most one appended Type-60 tail per authenticated class49 source
    // (native turrets, cleansing vehicles and Power Ups, or captured/factory
    // Type61). Class1 flowers share the blast prefix but append no ring.
    // A repeated/cyclic successor must
    // fail instead of spinning in a speculative world whose eventual rollback
    // would hide the topology bug.
    let actor_visit_limit = entities.iter_all().count().saturating_add(
        entities
            .iter_all()
            .filter(|entity| {
                entities.is_main_base_type61_constructor_authenticated(entity.id)
                    || (crate::class49_death::source_profile(entity).is_some_and(|profile| {
                        profile.policy() == crate::class49_death::NativeExplosionPolicy::Class49
                    }) && crate::class49_death::allocation_authenticates(entities, entity.id))
            })
            .count(),
    );
    let mut progress = MainBaseAbortProductionProgress::default();
    loop {
        let issued = match machine.poll() {
            MainBaseAbortPoll::Action(issued) => issued,
            MainBaseAbortPoll::PlanningBlocked(error) => {
                return Err(actor_phase_failure(
                    origin,
                    MainBaseAbortProductionDiagnostic::PlanningBlocked(error),
                    progress,
                    None,
                ))
            }
            MainBaseAbortPoll::Blocked(error) => {
                return Err(actor_phase_failure(
                    origin,
                    MainBaseAbortProductionDiagnostic::MachineBlocked(error),
                    progress,
                    None,
                ))
            }
            MainBaseAbortPoll::Complete(completion) => {
                return Err(actor_phase_failure(
                    origin,
                    MainBaseAbortProductionDiagnostic::UnexpectedCompletion(completion),
                    progress,
                    None,
                ))
            }
            MainBaseAbortPoll::Awaiting(actual) => {
                return Err(actor_phase_failure(
                    origin,
                    MainBaseAbortProductionDiagnostic::UnexpectedMachinePhase {
                        expected: MainBaseAbortPhase::ActorSweep,
                        actual,
                    },
                    progress,
                    None,
                ))
            }
        };
        match issued.action {
            MainBaseAbortAction::TransformTerrain { .. } => {
                if origin.is_some() {
                    return Err(actor_phase_failure(
                        origin,
                        MainBaseAbortProductionDiagnostic::TerminalOriginNotConsumed,
                        progress,
                        None,
                    ));
                }
                return Ok(ActorPhaseSuccess {
                    machine,
                    terrain_receipt: issued.receipt,
                    origin,
                    progress,
                });
            }
            MainBaseAbortAction::ProcessActor { actor, route, .. } => {
                if let Err(diagnostic) =
                    admit_actor_visit(actor_visit_limit, progress.processed_actors.len(), actor)
                {
                    return Err(block_actor_action(
                        machine,
                        issued.receipt,
                        MainBaseAbortExternalBlock::ActorSuccessorUnavailable,
                        origin,
                        diagnostic,
                        progress,
                        None,
                    ));
                }
                let dispatched = dispatch_actor(
                    actor,
                    route,
                    &mut origin,
                    entities,
                    terrain,
                    player_hull,
                    world_fx,
                    effects,
                    specialized_tasks,
                    &mut progress.publications,
                    gameplay,
                );
                let dispatched = match dispatched {
                    Ok(dispatched) => dispatched,
                    Err(error) => {
                        let diagnostic = error.diagnostic.unwrap_or_else(|| {
                            MainBaseAbortProductionDiagnostic::ActorCallbackBlocked {
                                actor,
                                error: error.callback_error.expect(
                                    "dispatch failures identify callback or scheduler custody",
                                ),
                            }
                        });
                        return Err(block_actor_action(
                            machine,
                            issued.receipt,
                            MainBaseAbortExternalBlock::ActorProcessingUnavailable,
                            origin,
                            diagnostic,
                            progress,
                            error.unscheduled_owner,
                        ));
                    }
                };
                progress.processed_actors.push(MainBaseAbortProcessedActor {
                    observation: actor,
                    route,
                    disposition: dispatched.disposition,
                });
                if !dispatched.successor_available {
                    return Err(block_actor_action(
                        machine,
                        issued.receipt,
                        MainBaseAbortExternalBlock::ActorSuccessorUnavailable,
                        origin,
                        MainBaseAbortProductionDiagnostic::ActorSuccessorUnavailableAfterCommit {
                            actor,
                        },
                        progress,
                        None,
                    ));
                }
                if let Err(failure) = machine.resume(
                    issued.receipt,
                    MainBaseAbortResume::ActorProcessed {
                        phase: MainBaseAbortPhase::ActorSweep,
                        actor: actor.lease,
                        successor: RetailRuntimeValue::Known(dispatched.successor),
                    },
                ) {
                    return Err(actor_phase_failure(
                        origin,
                        MainBaseAbortProductionDiagnostic::MachineProtocol(failure.error),
                        progress,
                        None,
                    ));
                }
            }
            action => {
                return Err(actor_phase_failure(
                    origin,
                    MainBaseAbortProductionDiagnostic::UnexpectedMachinePhase {
                        expected: MainBaseAbortPhase::ActorSweep,
                        actual: action.phase(),
                    },
                    progress,
                    None,
                ))
            }
        }
    }
}

/// Production Type-54 abort: retire any class0 owner, run class 38, then
/// register Change-Sea-Level. Captured fixtures without a class0 owner still
/// abort. A generic-death no-op restores the class0 owner.
pub fn apply_type54_abort_with_scheduler(
    lease: crate::main_base_abort::MainBaseAbortActorLease,
    entities: &mut EntityManager,
    terrain: &v2k_formats::terrain::TerrainGrid,
    world_fx: &mut WorldFx,
    specialized_tasks: &mut SpecializedActorTaskScheduler,
    publications: &mut MainBaseAbortPublicationCounts,
) -> Result<(), MainBaseAbortProductionDiagnostic> {
    let actor = entities
        .main_base_abort_actor_observation(lease.entity_id)
        .ok_or(MainBaseAbortProductionDiagnostic::ActorCallbackBlocked {
            actor: MainBaseAbortActorObservation {
                lease,
                entity_type: LEVEL_ONE_TYPE54_ENTITY_TYPE,
                capability_flags: 0,
                state_flags: RetailRuntimeValue::Unresolved,
            },
            error: MainBaseAbortActorCallbackBlock::Type54(MainBaseType54DeathBlock::EntityMissing),
        })?;
    dispatch_type54_death(
        actor,
        entities,
        terrain,
        world_fx,
        specialized_tasks,
        publications,
    )
    .map(|_| ())
    .map_err(|error| {
        error.diagnostic.unwrap_or_else(|| {
            MainBaseAbortProductionDiagnostic::ActorCallbackBlocked {
                actor,
                error: error
                    .callback_error
                    .expect("dispatch failures identify callback or scheduler custody"),
            }
        })
    })
}

fn dispatch_type54_death(
    actor: MainBaseAbortActorObservation,
    entities: &mut EntityManager,
    terrain: &v2k_formats::terrain::TerrainGrid,
    world_fx: &mut WorldFx,
    specialized_tasks: &mut SpecializedActorTaskScheduler,
    publications: &mut MainBaseAbortPublicationCounts,
) -> Result<DispatchSuccess, DispatchFailure> {
    let id = actor.lease.entity_id;
    let had_class0 = specialized_tasks.take_class0_actor_external_mutation(entities, id);
    let advance = entities
        .apply_main_base_abort_type54_death(
            actor.lease,
            terrain,
            LEVEL_ONE_TYPE54_MODEL_EXTENT_RAW,
            MainBaseType54NetworkSession::SoloNetworkingDisabled,
            world_fx,
        )
        .map_err(|error| callback_error(MainBaseAbortActorCallbackBlock::Type54(error)))?;
    let (outcome, successor, successor_available) = match advance {
        MainBaseType54DeathAdvance::Advanced {
            outcome,
            next_actor,
        } => (outcome, next_actor, true),
        MainBaseType54DeathAdvance::SuccessorUnavailableAfterCommit { outcome } => {
            (outcome, None, false)
        }
    };
    match outcome {
        MainBaseType54DeathOutcome::SeaLevelTaskPublished { tick_receipt, .. } => {
            match specialized_tasks.register_main_base_type54_sea_level(tick_receipt) {
                Ok(None) => publications.type54_sea_level += 1,
                Ok(Some(_displaced)) => {
                    publications.type54_sea_level += 1;
                    publications.same_family_replacements += 1;
                }
                Err(failure) => {
                    return Err(conflicting_owner(
                        failure.conflict,
                        MainBaseAbortUnscheduledOwner::Type54(failure.rejected),
                    ))
                }
            }
        }
        MainBaseType54DeathOutcome::RemoteOwnedNoOp { .. }
        | MainBaseType54DeathOutcome::AlreadyDyingNoOp { .. } => {
            if had_class0 {
                let owner = Class0ActorOwner::adopt(entities, id).map_err(|_| DispatchFailure {
                    callback_error: None,
                    diagnostic: Some(
                        MainBaseAbortProductionDiagnostic::Class0OwnerMissingAfterType54NoOp {
                            entity_id: id,
                        },
                    ),
                    unscheduled_owner: None,
                })?;
                specialized_tasks.register_class0_actor(owner);
            }
        }
        MainBaseType54DeathOutcome::InitializerFallbackAfterTaskAllocationFailure { .. } => {}
    }
    Ok(DispatchSuccess {
        disposition: MainBaseAbortActorDisposition::Type54Death,
        successor,
        successor_available,
    })
}

fn admit_actor_visit(
    limit: usize,
    already_processed: usize,
    actor: MainBaseAbortActorObservation,
) -> Result<(), MainBaseAbortProductionDiagnostic> {
    if already_processed < limit {
        Ok(())
    } else {
        Err(MainBaseAbortProductionDiagnostic::ActorVisitLimitExceeded { limit, actor })
    }
}

#[allow(clippy::too_many_arguments)]
fn dispatch_actor(
    actor: MainBaseAbortActorObservation,
    route: MainBaseAbortActorRoute,
    origin: &mut Option<MainBaseTerminalAbortOrigin>,
    entities: &mut EntityManager,
    terrain: &v2k_formats::terrain::TerrainGrid,
    player_hull: &mut PlayerHull,
    world_fx: &mut WorldFx,
    effects: &mut MainBaseAbortWorldEffects<'_>,
    specialized_tasks: &mut SpecializedActorTaskScheduler,
    publications: &mut MainBaseAbortPublicationCounts,
    gameplay: &mut MainBaseAbortGameplayContext<'_>,
) -> Result<DispatchSuccess, DispatchFailure> {
    match route {
        MainBaseAbortActorRoute::ExcludedType { .. }
        | MainBaseAbortActorRoute::SuppressedByState { .. } => {
            let advance = entities
                .advance_main_base_abort_no_callback(actor.lease)
                .map_err(|error| {
                    callback_error(MainBaseAbortActorCallbackBlock::NoCallback(error))
                })?;
            Ok(DispatchSuccess {
                disposition: MainBaseAbortActorDisposition::NoCallback,
                successor: advance.next_actor,
                successor_available: true,
            })
        }
        MainBaseAbortActorRoute::AlternateCleanup { .. } => {
            let advance = entities
                .apply_main_base_abort_alternate_cleanup(actor.lease, effects)
                .map_err(|error| {
                    callback_error(MainBaseAbortActorCallbackBlock::AlternateCleanup(error))
                })?;
            let (successor, successor_available) = match advance {
                MainBaseAbortAlternateCleanupAdvance::Advanced { next_actor, .. } => {
                    (next_actor, true)
                }
                MainBaseAbortAlternateCleanupAdvance::SuccessorUnavailableAfterCommit {
                    ..
                } => (None, false),
            };
            Ok(DispatchSuccess {
                disposition: MainBaseAbortActorDisposition::AlternateCleanup,
                successor,
                successor_available,
            })
        }
        MainBaseAbortActorRoute::UnresolvedState { .. } => Err(callback_error(
            MainBaseAbortActorCallbackBlock::UnsupportedOrdinaryType {
                entity_type: actor.entity_type,
            },
        )),
        MainBaseAbortActorRoute::OrdinaryDeath { .. } => dispatch_ordinary_actor(
            actor,
            origin,
            entities,
            terrain,
            player_hull,
            world_fx,
            effects,
            specialized_tasks,
            publications,
            gameplay,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn dispatch_ordinary_actor(
    actor: MainBaseAbortActorObservation,
    origin: &mut Option<MainBaseTerminalAbortOrigin>,
    entities: &mut EntityManager,
    terrain: &v2k_formats::terrain::TerrainGrid,
    player_hull: &mut PlayerHull,
    world_fx: &mut WorldFx,
    effects: &mut MainBaseAbortWorldEffects<'_>,
    specialized_tasks: &mut SpecializedActorTaskScheduler,
    publications: &mut MainBaseAbortPublicationCounts,
    gameplay: &mut MainBaseAbortGameplayContext<'_>,
) -> Result<DispatchSuccess, DispatchFailure> {
    if let Some(result) = native::dispatch_native_class49(
        actor,
        effects,
        crate::main_base_abort_world_effects::MainBaseAbortClass49Frame {
            entities,
            world_fx,
            notifications: gameplay.notifications,
            retail_tick: gameplay.retail_tick,
            scheduler: specialized_tasks,
            player_hull,
            extra_lives: gameplay.extra_lives,
        },
        publications,
    ) {
        return result;
    }
    // Only the actual Main Base which supplied the terminal origin needs
    // that linear already-dying receipt. A live Main Base reached by an
    // authored casualty loss enters the same native standard-death publisher
    // as a lethal hit, beginning 19750's staged destruction.
    let terminal_self = origin
        .as_ref()
        .is_some_and(|origin| origin.actor_lease() == actor.lease);
    if !terminal_self {
        if let Some(result) = native::dispatch_native_actor(
            actor,
            entities,
            world_fx,
            specialized_tasks,
            publications,
            gameplay,
        ) {
            return result;
        }
    }
    match actor.entity_type {
        MAIN_BASE_ENTITY_TYPE => {
            let terminal_origin = origin.take().ok_or_else(|| {
                callback_error(MainBaseAbortActorCallbackBlock::UnsupportedOrdinaryType {
                    entity_type: actor.entity_type,
                })
            })?;
            let advance = match entities
                .apply_main_base_abort_terminal_self_noop(terminal_origin, actor.lease)
            {
                Ok(advance) => advance,
                Err(failure) => {
                    *origin = Some(failure.origin);
                    return Err(callback_error(
                        MainBaseAbortActorCallbackBlock::TerminalSelf(failure.error),
                    ));
                }
            };
            let (successor, successor_available) = match advance {
                MainBaseAbortTerminalSelfAdvance::Advanced { next_actor, .. } => (next_actor, true),
                MainBaseAbortTerminalSelfAdvance::SuccessorUnavailableAfterCommit { .. } => {
                    (None, false)
                }
            };
            Ok(DispatchSuccess {
                disposition: MainBaseAbortActorDisposition::TerminalMainBaseNoOp,
                successor,
                successor_available,
            })
        }
        entity_type if QUIET_DEATH_ENTITY_TYPES.contains(&entity_type) => {
            let advance = entities
                .apply_main_base_abort_quiet_death(actor.lease, world_fx)
                .map_err(|error| {
                    callback_error(MainBaseAbortActorCallbackBlock::QuietDeath(error))
                })?;
            let (successor, successor_available) = match advance {
                MainBaseAbortQuietDeathAdvance::Advanced { next_actor, .. } => (next_actor, true),
                MainBaseAbortQuietDeathAdvance::SuccessorUnavailableAfterCommit { .. } => {
                    (None, false)
                }
            };
            Ok(DispatchSuccess {
                disposition: MainBaseAbortActorDisposition::QuietDeath,
                successor,
                successor_available,
            })
        }
        CARGO_DROP_PROXY_ENTITY_TYPE => {
            let advance = entities
                .apply_main_base_abort_type93_death(actor.lease)
                .map_err(|error| callback_error(MainBaseAbortActorCallbackBlock::Type93(error)))?;
            let (successor, successor_available) = match advance {
                MainBaseType93DeathAdvance::Advanced { next_actor, .. } => (next_actor, true),
                MainBaseType93DeathAdvance::SuccessorUnavailableAfterCommit { .. } => (None, false),
            };
            Ok(DispatchSuccess {
                disposition: MainBaseAbortActorDisposition::Type93Death,
                successor,
                successor_available,
            })
        }
        LEVEL_ONE_TYPE9_ENTITY_TYPE => {
            let scheduler_owner = specialized_tasks
                .take_ordinary_type9_selected_for_main_base_abort(actor.lease)
                .map_err(|error| {
                    callback_error(
                        MainBaseAbortActorCallbackBlock::OrdinaryType9SelectedCustody(error),
                    )
                })?;
            let selected_resume = if let Some(scheduler_owner) = scheduler_owner {
                let (original_index, initial_owner, resume_state) =
                    scheduler_owner.decompose_for_main_base_abort();
                if let Err(initial_owner) = entities
                    .restore_fresh_level1_type9_initial_production(original_index, initial_owner)
                {
                    let scheduler_owner = reconstitute_ordinary_type9_selected_owner(
                        actor.lease.entity_id,
                        original_index,
                        initial_owner,
                        resume_state,
                    )?;
                    restore_ordinary_type9_selected_owner(
                        actor.lease.entity_id,
                        scheduler_owner,
                        specialized_tasks,
                    )?;
                    return Err(DispatchFailure {
                        callback_error: None,
                        diagnostic: Some(
                            MainBaseAbortProductionDiagnostic::
                                OrdinaryType9SelectedSidecarRestoreConflict {
                                    entity_id: actor.lease.entity_id,
                                },
                        ),
                        unscheduled_owner: None,
                    });
                }
                Some((original_index, resume_state))
            } else {
                None
            };
            let advance = match entities.apply_main_base_abort_type9_death(
                actor.lease,
                MainBaseType9ResultScreenState::AlreadyShownByMainBaseAbort,
                world_fx,
            ) {
                Ok(advance) => advance,
                Err(error) => {
                    if let Some((original_index, resume_state)) = selected_resume {
                        restore_ordinary_type9_selected_after_nonconsuming_death(
                            entities,
                            specialized_tasks,
                            actor.lease.entity_id,
                            original_index,
                            resume_state,
                        )?;
                    }
                    return Err(callback_error(MainBaseAbortActorCallbackBlock::Type9(
                        error,
                    )));
                }
            };
            let (outcome, successor, successor_available) = match advance {
                MainBaseType9DeathAdvance::Advanced {
                    outcome,
                    next_actor,
                } => (outcome, next_actor, true),
                MainBaseType9DeathAdvance::SuccessorUnavailableAfterCommit { outcome } => {
                    (outcome, None, false)
                }
            };
            if matches!(
                &outcome,
                MainBaseType9DeathOutcome::RemoteOwnedNoOp { .. }
                    | MainBaseType9DeathOutcome::AlreadyDyingNoOp { .. }
            ) {
                if let Some((original_index, resume_state)) = selected_resume {
                    restore_ordinary_type9_selected_after_nonconsuming_death(
                        entities,
                        specialized_tasks,
                        actor.lease.entity_id,
                        original_index,
                        resume_state,
                    )?;
                }
            }
            if let MainBaseType9DeathOutcome::ExplodingTaskPublished { task_lease, .. } = outcome {
                match specialized_tasks.register_main_base_type9_exploding(task_lease) {
                    Ok(None) => publications.type9_exploding += 1,
                    Ok(Some(_displaced)) => {
                        publications.type9_exploding += 1;
                        publications.same_family_replacements += 1;
                    }
                    Err(failure) => {
                        return Err(conflicting_owner(
                            failure.conflict,
                            MainBaseAbortUnscheduledOwner::Type9(failure.rejected),
                        ))
                    }
                }
            }
            Ok(DispatchSuccess {
                disposition: MainBaseAbortActorDisposition::Type9Death,
                successor,
                successor_available,
            })
        }
        TYPE17_IMPACT_ENTITY_TYPE => {
            let advance = entities
                .apply_main_base_abort_type17_death(actor.lease, world_fx)
                .map_err(|error| callback_error(MainBaseAbortActorCallbackBlock::Type17(error)))?;
            let (outcome, successor, successor_available) = match advance {
                MainBaseType17DeathAdvance::Advanced {
                    outcome,
                    next_actor,
                } => (outcome, next_actor, true),
                MainBaseType17DeathAdvance::SuccessorUnavailableAfterCommit { outcome } => {
                    (outcome, None, false)
                }
            };
            if let MainBaseType17DeathOutcome::CommonDyingPublished(publication) = outcome {
                if let Err(failure) =
                    specialized_tasks.register_type17_common_dying(publication.owner)
                {
                    return Err(conflicting_owner(
                        failure.conflict,
                        MainBaseAbortUnscheduledOwner::Type17(failure.rejected),
                    ));
                }
                publications.type17_common_dying += 1;
            }
            Ok(DispatchSuccess {
                disposition: MainBaseAbortActorDisposition::Type17Death,
                successor,
                successor_available,
            })
        }
        TYPE47_COMMON_DYING_ENTITY_TYPE => {
            let advance = entities
                .apply_main_base_abort_type47_death(actor.lease, world_fx)
                .map_err(|error| callback_error(MainBaseAbortActorCallbackBlock::Type47(error)))?;
            let (outcome, successor, successor_available) = match advance {
                MainBaseType47DeathAdvance::Advanced {
                    outcome,
                    next_actor,
                } => (outcome, next_actor, true),
                MainBaseType47DeathAdvance::SuccessorUnavailableAfterCommit { outcome } => {
                    (outcome, None, false)
                }
            };
            if let MainBaseType47DeathOutcome::CommonDyingPublished(publication) = outcome {
                if let Err(failure) =
                    specialized_tasks.register_type47_common_dying(publication.owner)
                {
                    return Err(conflicting_owner(
                        failure.conflict,
                        MainBaseAbortUnscheduledOwner::Type47(failure.rejected),
                    ));
                }
                publications.type47_common_dying += 1;
            }
            Ok(DispatchSuccess {
                disposition: MainBaseAbortActorDisposition::Type47Death,
                successor,
                successor_available,
            })
        }
        LEVEL_ONE_TYPE54_ENTITY_TYPE => dispatch_type54_death(
            actor,
            entities,
            terrain,
            world_fx,
            specialized_tasks,
            publications,
        ),
        TYPE60_RING_ENTITY_TYPE => {
            let scheduler_owner = specialized_tasks
                .take_type60_exploding_ring_for_main_base_abort(actor.lease)
                .map_err(|error| {
                    callback_error(MainBaseAbortActorCallbackBlock::Type60Custody(error))
                })?;
            let advance = entities
                .apply_main_base_abort_type60_ring_death(
                    actor.lease,
                    scheduler_owner.as_ref(),
                    world_fx,
                )
                .map_err(|error| callback_error(MainBaseAbortActorCallbackBlock::Type60(error)))?;
            let (outcome, successor, successor_available) = match advance {
                MainBaseType60RingDeathAdvance::Advanced {
                    outcome,
                    next_actor,
                } => (outcome, next_actor, true),
                MainBaseType60RingDeathAdvance::SuccessorUnavailableAfterCommit { outcome } => {
                    (outcome, None, false)
                }
            };
            if matches!(
                outcome,
                MainBaseType60RingDeathOutcome::RemoteOwnedNoOp { .. }
                    | MainBaseType60RingDeathOutcome::AlreadyDyingNoOp { .. }
            ) {
                if let Some(scheduler_owner) = scheduler_owner {
                    match specialized_tasks
                        .restore_type60_exploding_ring_after_main_base_abort_noop(scheduler_owner)
                    {
                        Ok(None) => {}
                        Ok(Some(displaced)) => {
                            return Err(DispatchFailure {
                                callback_error: None,
                                diagnostic: Some(
                                    MainBaseAbortProductionDiagnostic::
                                        Type60OwnerRestoreReplacedExisting {
                                            entity_id: actor.lease.entity_id,
                                        },
                                ),
                                unscheduled_owner: Some(MainBaseAbortUnscheduledOwner::Type60(
                                    displaced,
                                )),
                            });
                        }
                        Err(failure) => {
                            return Err(conflicting_owner(
                                failure.conflict,
                                MainBaseAbortUnscheduledOwner::Type60(failure.rejected),
                            ));
                        }
                    }
                }
            }
            Ok(DispatchSuccess {
                disposition: MainBaseAbortActorDisposition::Type60RingDeath,
                successor,
                successor_available,
            })
        }
        LEVEL_ONE_TYPE61_ENTITY_TYPE => {
            let request = type61_request(entities, actor, terrain).map_err(|error| {
                callback_error(MainBaseAbortActorCallbackBlock::Type61Request(error))
            })?;
            let advance = entities
                .apply_main_base_abort_type61_death(
                    actor.lease,
                    request,
                    terrain,
                    player_hull,
                    world_fx,
                    effects,
                )
                .map_err(|error| callback_error(MainBaseAbortActorCallbackBlock::Type61(error)))?;
            let (outcome, successor, successor_available) = match advance {
                MainBaseType61DeathAdvance::Advanced {
                    outcome,
                    next_actor,
                } => (outcome, next_actor, true),
                MainBaseType61DeathAdvance::SuccessorUnavailableAfterCommit { outcome } => {
                    (outcome, None, false)
                }
            };
            if matches!(
                outcome,
                MainBaseType61DeathOutcome::ExplodeWithRingCommitted {
                    ring: Type60ConstructionOutcome::ActorLinked(_),
                    ..
                }
            ) {
                publications.appended_type60_actors += 1;
            }
            Ok(DispatchSuccess {
                disposition: MainBaseAbortActorDisposition::Type61Death,
                successor,
                successor_available,
            })
        }
        LEVEL_ONE_TYPE66_ENTITY_TYPE => {
            let advance = entities
                .apply_main_base_abort_type66_death(actor.lease, world_fx)
                .map_err(|error| callback_error(MainBaseAbortActorCallbackBlock::Type66(error)))?;
            let (outcome, successor, successor_available) = match advance {
                MainBaseType66DeathAdvance::Advanced {
                    outcome,
                    next_actor,
                } => (outcome, next_actor, true),
                MainBaseType66DeathAdvance::SuccessorUnavailableAfterCommit { outcome } => {
                    (outcome, None, false)
                }
            };
            if let MainBaseType66DeathOutcome::WorkingFactoryTaskPublished { task_lease, .. } =
                outcome
            {
                let _ = specialized_tasks.release_level_one_factory_arrival(actor.lease.entity_id);
                match specialized_tasks.register_main_base_type66_production(task_lease) {
                    Ok(None) => publications.type66_production += 1,
                    Ok(Some(_displaced)) => {
                        publications.type66_production += 1;
                        publications.same_family_replacements += 1;
                    }
                    Err(failure) => {
                        return Err(conflicting_owner(
                            failure.conflict,
                            MainBaseAbortUnscheduledOwner::Type66(failure.rejected),
                        ))
                    }
                }
            }
            Ok(DispatchSuccess {
                disposition: MainBaseAbortActorDisposition::Type66Death,
                successor,
                successor_available,
            })
        }
        entity_type => Err(callback_error(
            MainBaseAbortActorCallbackBlock::UnsupportedOrdinaryType { entity_type },
        )),
    }
}

fn type61_request(
    entities: &EntityManager,
    actor: MainBaseAbortActorObservation,
    terrain: &v2k_formats::terrain::TerrainGrid,
) -> Result<MainBaseType61DeathRequest, MainBaseType61RequestBlock> {
    let entity = entities
        .iter_all()
        .find(|entity| entity.id == actor.lease.entity_id)
        .ok_or(MainBaseType61RequestBlock::EntityMissing)?;
    let generic_no_op = match entity
        .collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT)
    {
        RetailRuntimeValue::Known(value) if value != 0 => true,
        RetailRuntimeValue::Known(0) => matches!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(value) if value != 0
        ),
        RetailRuntimeValue::Known(_) | RetailRuntimeValue::Unresolved => false,
    };
    let active_model_extent_raw = if generic_no_op {
        // Generic death returns before consulting the class-49 request.
        0
    } else {
        match (
            entity
                .authored_spawn_index
                .and_then(MainBaseType61CapturedSpawn::for_spawn),
            entity.factory_type61_birth_provenance(),
        ) {
            (Some(captured), None) => captured.active_model_extent_raw,
            (None, Some(_)) => LEVEL_ONE_TYPE61_DEFAULT_MODEL_EXTENT_RAW,
            (None, None) | (Some(_), Some(_)) => {
                return Err(MainBaseType61RequestBlock::UnauthenticatedSpawn {
                    actual: entity.authored_spawn_index,
                })
            }
        }
    };
    let source_owner = MainBaseType61LogicalOwner {
        entity_id: entity.id,
        entity_type: entity.entity_type,
    };
    let logical_owner = if generic_no_op {
        // The generic-death prefix returns before consulting this request.
        // Source ownership is the exact null/dangling relation fallback and
        // avoids requiring irrelevant relation evidence on that no-op path.
        source_owner
    } else {
        match entity.collision.recent_relation_id_at_0x60 {
            RetailRuntimeValue::Unresolved => {
                return Err(MainBaseType61RequestBlock::LogicalOwnerUnresolved)
            }
            RetailRuntimeValue::Known(None) => source_owner,
            RetailRuntimeValue::Known(Some(relation_id)) => {
                let mut owners = entities
                    .iter_all()
                    .filter(|candidate| candidate.id == relation_id);
                match (owners.next(), owners.next()) {
                    (Some(owner), None) => MainBaseType61LogicalOwner {
                        entity_id: owner.id,
                        entity_type: owner.entity_type,
                    },
                    (None, None) => source_owner,
                    _ => {
                        return Err(MainBaseType61RequestBlock::LogicalOwnerAmbiguous {
                            relation_id,
                        })
                    }
                }
            }
        }
    };
    Ok(MainBaseType61DeathRequest {
        active_model_extent_raw,
        sea_level_raw: Some((terrain.header[0] >> 8) as i16),
        logical_owner,
        network_session: MainBaseType61NetworkSession::SoloNetworkingDisabled,
    })
}

fn poll_action(
    machine: &mut MainBaseAbortMachine,
    expected: MainBaseAbortPhase,
) -> Result<crate::main_base_abort::IssuedMainBaseAbortAction, MainBaseAbortProductionDiagnostic> {
    match machine.poll() {
        MainBaseAbortPoll::Action(issued) if issued.action.phase() == expected => Ok(issued),
        MainBaseAbortPoll::Action(issued) => {
            Err(MainBaseAbortProductionDiagnostic::UnexpectedMachinePhase {
                expected,
                actual: issued.action.phase(),
            })
        }
        MainBaseAbortPoll::Awaiting(actual) => {
            Err(MainBaseAbortProductionDiagnostic::UnexpectedMachinePhase { expected, actual })
        }
        MainBaseAbortPoll::PlanningBlocked(error) => {
            Err(MainBaseAbortProductionDiagnostic::PlanningBlocked(error))
        }
        MainBaseAbortPoll::Blocked(error) => {
            Err(MainBaseAbortProductionDiagnostic::MachineBlocked(error))
        }
        MainBaseAbortPoll::Complete(completion) => Err(
            MainBaseAbortProductionDiagnostic::UnexpectedCompletion(completion),
        ),
    }
}

fn callback_error(error: MainBaseAbortActorCallbackBlock) -> DispatchFailure {
    DispatchFailure {
        callback_error: Some(error),
        diagnostic: None,
        unscheduled_owner: None,
    }
}

fn conflicting_owner(
    conflict: SpecializedActorTaskRegistrationConflict,
    owner: MainBaseAbortUnscheduledOwner,
) -> DispatchFailure {
    DispatchFailure {
        callback_error: None,
        diagnostic: Some(MainBaseAbortProductionDiagnostic::SpecializedOwnerConflict(
            conflict,
        )),
        unscheduled_owner: Some(owner),
    }
}

fn restore_ordinary_type9_selected_after_nonconsuming_death(
    entities: &mut EntityManager,
    specialized_tasks: &mut SpecializedActorTaskScheduler,
    entity_id: u32,
    original_index: usize,
    resume: OrdinaryType9SelectedProductionResume,
) -> Result<(), DispatchFailure> {
    let Some((_temporary_index, initial_owner)) =
        entities.take_fresh_level1_type9_initial_production_for_entity(entity_id)
    else {
        return Err(DispatchFailure {
            callback_error: None,
            diagnostic: Some(
                MainBaseAbortProductionDiagnostic::OrdinaryType9SelectedOwnerMissingAfterNoOp {
                    entity_id,
                },
            ),
            unscheduled_owner: None,
        });
    };
    let scheduler_owner = reconstitute_ordinary_type9_selected_owner(
        entity_id,
        original_index,
        initial_owner,
        resume,
    )?;
    restore_ordinary_type9_selected_owner(entity_id, scheduler_owner, specialized_tasks)
}

fn restore_ordinary_type9_selected_owner(
    entity_id: u32,
    owner: OrdinaryType9SelectedProductionOwner,
    specialized_tasks: &mut SpecializedActorTaskScheduler,
) -> Result<(), DispatchFailure> {
    match specialized_tasks.restore_ordinary_type9_selected_after_main_base_abort_noop(owner) {
        Ok(None) => Ok(()),
        Ok(Some(displaced)) => Err(DispatchFailure {
            callback_error: None,
            diagnostic: Some(
                MainBaseAbortProductionDiagnostic::
                    OrdinaryType9SelectedOwnerRestoreReplacedExisting { entity_id },
            ),
            unscheduled_owner: Some(MainBaseAbortUnscheduledOwner::OrdinaryType9Selected(
                displaced,
            )),
        }),
        Err(failure) => Err(conflicting_owner(
            failure.conflict,
            MainBaseAbortUnscheduledOwner::OrdinaryType9Selected(failure.rejected),
        )),
    }
}

fn reconstitute_ordinary_type9_selected_owner(
    entity_id: u32,
    original_index: usize,
    initial_owner: FreshLevel1Type9InitialProductionOwner,
    resume: OrdinaryType9SelectedProductionResume,
) -> Result<OrdinaryType9SelectedProductionOwner, DispatchFailure> {
    resume
        .resume_after_main_base_abort_noop(original_index, initial_owner)
        .map_err(|initial_owner| DispatchFailure {
            callback_error: None,
            diagnostic: Some(
                MainBaseAbortProductionDiagnostic::OrdinaryType9SelectedOwnerReconstitutionFailed {
                    entity_id,
                },
            ),
            unscheduled_owner: Some(MainBaseAbortUnscheduledOwner::OrdinaryType9Initial(
                initial_owner,
            )),
        })
}

fn block_actor_action(
    mut machine: MainBaseAbortMachine,
    receipt: crate::main_base_abort::MainBaseAbortReceipt,
    reason: MainBaseAbortExternalBlock,
    origin: Option<MainBaseTerminalAbortOrigin>,
    mut diagnostic: MainBaseAbortProductionDiagnostic,
    progress: MainBaseAbortProductionProgress,
    unscheduled_owner: Option<MainBaseAbortUnscheduledOwner>,
) -> ActorPhaseFailure {
    if let Err(failure) = machine.resume(
        receipt,
        MainBaseAbortResume::Blocked {
            phase: MainBaseAbortPhase::ActorSweep,
            reason,
        },
    ) {
        diagnostic = MainBaseAbortProductionDiagnostic::MachineProtocol(failure.error);
    }
    actor_phase_failure(origin, diagnostic, progress, unscheduled_owner)
}

fn actor_phase_failure(
    origin: Option<MainBaseTerminalAbortOrigin>,
    diagnostic: MainBaseAbortProductionDiagnostic,
    progress: MainBaseAbortProductionProgress,
    unscheduled_owner: Option<MainBaseAbortUnscheduledOwner>,
) -> ActorPhaseFailure {
    ActorPhaseFailure {
        origin,
        diagnostic,
        progress,
        unscheduled_owner,
    }
}

fn production_failure(
    origin: Option<MainBaseTerminalAbortOrigin>,
    diagnostic: MainBaseAbortProductionDiagnostic,
    progress: MainBaseAbortProductionProgress,
    unscheduled_owner: Option<MainBaseAbortUnscheduledOwner>,
) -> MainBaseAbortProductionFailure {
    MainBaseAbortProductionFailure {
        terminal_origin: origin,
        diagnostic,
        progress,
        unscheduled_owner,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use crate::actor_task_dispatcher::ActorTaskRuntime;
    use crate::actor_task_owner::ActorTaskSlot;
    use crate::entity::{
        exact_level_one_type9_attract_attention_manager, exact_level_one_type9_go_to_job_manager,
        exact_level_one_type9_run_away_manager, exact_level_one_type9_wander_manager,
        main_base_abort_world_cache, EntityManager,
    };
    use crate::entity_collision_state::{
        RetailRuntimeValue, DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
    };
    use crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT;
    use crate::main_base_abort::MainBaseAbortActorLease;
    use crate::ordinary_type9_initial_production::FreshLevel1Type9InitialProductionOwner;
    use crate::ordinary_type9_live::OrdinaryType9SelectedRuntimeKind;
    use crate::ordinary_type9_run_away_production::{
        OrdinaryType9RunAwayActivePhase, OrdinaryType9RunAwayProductionBlock,
        OrdinaryType9RunAwayProductionOutcome,
    };
    use crate::specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
    };
    use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};

    fn flat_terrain() -> TerrainGrid {
        TerrainGrid {
            header: [0; 5],
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

    #[test]
    fn casualty_abort_needs_no_terminal_actor_and_preserves_atomic_rejection() {
        use crate::power_up_contact::PlayerCampaignProgress;
        use std::num::NonZeroU64;

        for terrain_available in [true, false] {
            let mut resources = main_base_abort_world_cache(flat_terrain());
            let level = resources.level_desc().unwrap();
            let mut entities = EntityManager::from_level(level, &[], resources.terrain());
            let mut progress = PlayerCampaignProgress::new();
            progress.set_current_control_slot(Some(3));
            let (mut controller, _) = MainBaseAbortControllerStorage::from_loaded_level(
                NonZeroU64::new(17).unwrap(),
                level,
                false,
                &mut progress,
            )
            .unwrap();
            let before = controller;
            if !terrain_available {
                resources = ResourceCache::new(Vec::new());
            }
            let result = execute_campaign_abort(
                MainBaseAbortTransactionId::new(17).unwrap(),
                None,
                &mut controller,
                &mut entities,
                &mut resources,
                &mut StaticDamageScheduler::new(),
                &mut PlayerHull::default(),
                &mut WorldFx::new(),
                &mut SpecializedActorTaskScheduler::new(),
                MainBaseAbortGameplayContext {
                    extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                    notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                    retail_tick: 0x1dcb,
                },
            );
            if terrain_available {
                let report =
                    result.expect("a campaign goal does not need Main Base origin custody");
                assert!(report.completion.full_frame_submitted);
                assert!(controller.abort_frame_submitted());
                assert_eq!(controller.player_state(), RetailRuntimeValue::Known(5));
                assert!(report.processed_actors.is_empty());
            } else {
                let failure = result.expect_err("missing geometry keeps the shared body atomic");
                assert!(failure.terminal_origin.is_none());
                assert_eq!(controller, before);
                assert!(failure.progress.processed_actors.is_empty());
                assert!(failure.progress.frame_request.is_none());
            }
        }
    }

    fn isolated_fresh_run_away_owner() -> (
        EntityManager,
        SpecializedActorTaskScheduler,
        MainBaseAbortActorLease,
    ) {
        let mut manager = exact_level_one_type9_run_away_manager();
        let run_away_ids = manager
            .fresh_level1_type9_initial_productions()
            .iter()
            .filter(|owner| owner.successful_run_away_selection().is_some())
            .map(FreshLevel1Type9InitialProductionOwner::entity_id)
            .collect::<Vec<_>>();
        let entity_id = *run_away_ids
            .first()
            .expect("the exact deterministic cohort selects Run Away");
        assert_eq!(
            manager
                .ordinary_type9_run_away_entity_mut(entity_id)
                .expect("selected Run Away actor remains live")
                .collision
                .animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0),
            "authenticated native Type-9 construction initializes the transient +0xB2 contribution"
        );

        let mut adopted = SpecializedActorTaskScheduler::new();
        assert_eq!(
            adopted.adopt_fresh_level1_type9_run_away(&mut manager),
            Ok(run_away_ids.len())
        );
        let actor = manager
            .main_base_abort_actor_observation(entity_id)
            .expect("selected Run Away actor remains observable")
            .lease;
        let owner = adopted
            .take_ordinary_type9_run_away_for_main_base_abort(actor)
            .expect("the exact actor lease owns Run Away custody")
            .expect("load adoption publishes the selected Run Away owner");
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler
            .register_ordinary_type9_run_away(owner)
            .expect("isolated scheduler has no cross-family conflict")
            .is_none());
        (manager, scheduler, actor)
    }

    fn isolated_fresh_go_to_job_owner() -> (
        EntityManager,
        SpecializedActorTaskScheduler,
        MainBaseAbortActorLease,
    ) {
        let mut manager = exact_level_one_type9_go_to_job_manager();
        let entity_id = manager
            .fresh_level1_type9_initial_productions()
            .iter()
            .find(|owner| owner.successful_go_to_job_selection().is_some())
            .map(FreshLevel1Type9InitialProductionOwner::entity_id)
            .expect("the scripted fixture publishes Go-To-Job");
        let entity = manager
            .ordinary_type9_selected_entity_mut(entity_id)
            .expect("selected Go-To-Job actor remains live");
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        entity.collision.death_sound_id = RetailRuntimeValue::Known(Some(
            crate::main_base_type9_abort::LEVEL_ONE_TYPE9_DEATH_SOUND_ID,
        ));

        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(
            scheduler.adopt_fresh_level1_type9_selected(&mut manager),
            Ok(1)
        );
        let actor = manager
            .main_base_abort_actor_observation(entity_id)
            .expect("selected Go-To-Job actor remains observable")
            .lease;
        (manager, scheduler, actor)
    }

    fn isolated_fresh_wander_owner() -> (
        EntityManager,
        SpecializedActorTaskScheduler,
        MainBaseAbortActorLease,
    ) {
        let mut manager = exact_level_one_type9_wander_manager();
        let entity_id = manager
            .fresh_level1_type9_initial_productions()
            .iter()
            .find(|owner| owner.successful_wander_selection().is_some())
            .map(FreshLevel1Type9InitialProductionOwner::entity_id)
            .expect("the scripted fixture publishes Wander");
        let entity = manager
            .ordinary_type9_selected_entity_mut(entity_id)
            .expect("selected Wander actor remains live");
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        entity.collision.death_sound_id = RetailRuntimeValue::Known(Some(
            crate::main_base_type9_abort::LEVEL_ONE_TYPE9_DEATH_SOUND_ID,
        ));

        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(
            scheduler.adopt_fresh_level1_type9_selected(&mut manager),
            Ok(1)
        );
        let actor = manager
            .main_base_abort_actor_observation(entity_id)
            .expect("selected Wander actor remains observable")
            .lease;
        (manager, scheduler, actor)
    }

    fn isolated_fresh_attract_attention_owner(
        odd_parity: bool,
    ) -> (
        EntityManager,
        SpecializedActorTaskScheduler,
        MainBaseAbortActorLease,
    ) {
        let mut manager = exact_level_one_type9_attract_attention_manager(odd_parity);
        let entity_id = manager
            .fresh_level1_type9_initial_productions()
            .iter()
            .find(|owner| owner.successful_attract_attention_selection().is_some())
            .map(FreshLevel1Type9InitialProductionOwner::entity_id)
            .expect("the scripted fixture publishes Attract Attention");
        let entity = manager
            .ordinary_type9_selected_entity_mut(entity_id)
            .expect("selected Attract Attention actor remains live");
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        entity.collision.death_sound_id = RetailRuntimeValue::Known(Some(
            crate::main_base_type9_abort::LEVEL_ONE_TYPE9_DEATH_SOUND_ID,
        ));

        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(
            scheduler.adopt_fresh_level1_type9_selected(&mut manager),
            Ok(1)
        );
        let actor = manager
            .main_base_abort_actor_observation(entity_id)
            .expect("selected Attract Attention actor remains observable")
            .lease;
        (manager, scheduler, actor)
    }

    fn fork_selected_owner_through_resume(
        owner: OrdinaryType9SelectedProductionOwner,
    ) -> (
        OrdinaryType9SelectedProductionOwner,
        OrdinaryType9SelectedProductionOwner,
    ) {
        let expected_owner = owner.fork_for_main_base_abort_transaction();
        let (expected_index, expected_initial_owner, expected_resume) =
            expected_owner.decompose_for_main_base_abort();
        let expected = expected_resume
            .resume_after_main_base_abort_noop(expected_index, expected_initial_owner)
            .expect("forked selected sidecar reconstitutes the same owner");
        let (original_index, initial_owner, resume) = owner.decompose_for_main_base_abort();
        let installed = resume
            .resume_after_main_base_abort_noop(original_index, initial_owner)
            .expect("original selected sidecar reconstitutes the same owner");
        (expected, installed)
    }

    fn dispatch_type9_for_test(
        manager: &mut EntityManager,
        scheduler: &mut SpecializedActorTaskScheduler,
        actor: MainBaseAbortActorLease,
    ) -> (
        Result<DispatchSuccess, DispatchFailure>,
        MainBaseAbortPublicationCounts,
    ) {
        let observation = manager
            .main_base_abort_actor_observation(actor.entity_id)
            .expect("selected Type-9 actor remains observable");
        let terrain = flat_terrain();
        let mut cache = main_base_abort_world_cache(flat_terrain());
        let geometry = snapshot_main_base_abort_terrain_geometry(&cache).unwrap();
        let mut static_damage = StaticDamageScheduler::new();
        let mut effects =
            MainBaseAbortWorldEffects::new(&geometry, &mut cache, &mut static_damage).unwrap();
        let mut origin = None;
        let mut player_hull = PlayerHull::default();
        let mut world_fx = WorldFx::new();
        let mut publications = MainBaseAbortPublicationCounts::default();
        let result = dispatch_ordinary_actor(
            observation,
            &mut origin,
            manager,
            &terrain,
            &mut player_hull,
            &mut world_fx,
            &mut effects,
            scheduler,
            &mut publications,
            &mut crate::main_base_abort_production::MainBaseAbortGameplayContext {
                extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                retail_tick: 123,
            },
        );
        (result, publications)
    }

    fn configure_run_away_scheduler_entry(
        manager: &mut EntityManager,
        actor: MainBaseAbortActorLease,
        animation_offset_raw: u16,
        random_waits_disabled: bool,
    ) {
        let entity = manager
            .ordinary_type9_run_away_entity_mut(actor.entity_id)
            .expect("selected Run Away actor remains live");
        // This helper supplies a later scheduler entry, with an explicit
        // accumulated offset and callback mode after the startup publication.
        entity.collision.fresh_level1_type9_first_scheduler_pending = false;
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(animation_offset_raw);
        entity.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(7);
        entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        entity.collision.scheduler_unit_delta_flag_at_0xb6 = RetailRuntimeValue::Known(0);
        entity.collision.state_flags_at_0x08.overwrite(
            SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            if random_waits_disabled {
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            } else {
                0
            },
        );
    }

    fn progressed_run_away_owner() -> (
        EntityManager,
        SpecializedActorTaskScheduler,
        MainBaseAbortActorLease,
        ResourceCache,
        WorldFx,
    ) {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let (mut manager, mut scheduler, actor) = isolated_fresh_run_away_owner();
        configure_run_away_scheduler_entry(&mut manager, actor, 0, true);
        assert_eq!(
            scheduler.actor_animation_claims().collect::<Vec<_>>(),
            [actor],
            "mode-zero Run Away owns the live Sub-I update"
        );
        let mut resources = main_base_abort_world_cache(flat_terrain());
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([0x1111_u32, 0x2222, 0xD2F6]);
        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut resources,
                world_fx: &mut world_fx,
                static_damage: &mut static_damage,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
                main_base_abort_active: false,
            },
            &mut || {
                words
                    .pop_front()
                    .expect("the acquiring mover consumes three words")
            },
        );
        assert_eq!(
            pass.outcomes,
            [SpecializedActorTaskProductionOutcome::OrdinaryType9RunAway(
                OrdinaryType9RunAwayProductionOutcome::PostBasisTailPending {
                    entity_id: actor.entity_id,
                    completed_phase: OrdinaryType9RunAwayActivePhase::Fleeing,
                    callback_elapsed_micros: 20_000,
                },
            )]
        );
        assert!(words.is_empty());
        (manager, scheduler, actor, resources, world_fx)
    }

    #[test]
    fn finite_actor_visit_guard_blocks_the_first_successor_beyond_the_bound() {
        let actor = MainBaseAbortActorObservation {
            lease: MainBaseAbortActorLease {
                entity_id: 99,
                allocation_identity: 123,
            },
            entity_type: 61,
            capability_flags: 0x40,
            state_flags: RetailRuntimeValue::Known(0),
        };
        assert!(admit_actor_visit(39, 38, actor).is_ok());
        assert!(matches!(
            admit_actor_visit(39, 39, actor),
            Err(MainBaseAbortProductionDiagnostic::ActorVisitLimitExceeded {
                limit: 39,
                actor: repeated,
            }) if repeated == actor
        ));
    }

    #[test]
    fn adopted_run_away_ticks_prefix_handoff_and_waits_after_completed_tail_without_task_replay() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        const ELAPSED_MICROS: u32 = 20_000;

        let (mut manager, mut scheduler, actor) = isolated_fresh_run_away_owner();
        // The shared cohort fixture supplies a captured pre-abort state. This
        // test separately exercises the native first-visit publication before
        // its later callback/prefix handoff checks.
        manager
            .ordinary_type9_run_away_entity_mut(actor.entity_id)
            .unwrap()
            .collision
            .fresh_level1_type9_first_scheduler_pending = true;
        let mut resources = main_base_abort_world_cache(flat_terrain());
        let mut world_fx = WorldFx::new();
        let snapshot = |manager: &EntityManager| {
            let entity = manager
                .iter_all()
                .find(|entity| entity.id == actor.entity_id)
                .expect("selected Run Away actor remains live");
            (
                entity.collision.clone(),
                entity.mass_raw,
                entity.position,
                entity.velocity,
                entity.heading.to_bits(),
                entity.rotation_heading_pitch_roll_raw(),
                entity.physical_body_basis_q31,
                entity.current_behavior_context,
                entity.sub_a_propulsion_runtime,
                entity.actor_animation_runtime,
                entity.ordinary_type9_selected_component_runtime,
                ActorTaskSlot::IN_RETAIL_TICK_ORDER
                    .map(|slot| entity.actor_task_state(slot).copied()),
            )
        };
        let mut expected_wait = snapshot(&manager);
        expected_wait.0.state_flags_at_0x08 = crate::entity_collision_state::RetailStateWord::exact(
            crate::ordinary_type9_live::FRESH_LEVEL1_ORDINARY_TYPE9_FIRST_SCHEDULER_STATE_VALUE,
        );
        expected_wait.0.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        expected_wait.0.fresh_level1_type9_first_scheduler_pending = false;
        expected_wait.0.callback_scheduler_accumulator_us_at_0x6c =
            RetailRuntimeValue::Known(ELAPSED_MICROS);
        expected_wait.0.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(ELAPSED_MICROS);
        let mut wait_words = [u32::MAX, u32::MAX].into_iter();
        let waiting = scheduler.tick_with_scripted_random(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut resources,
                world_fx: &mut world_fx,
                static_damage: &mut static_damage,
                elapsed_micros: ELAPSED_MICROS,
                global_elapsed_micros: ELAPSED_MICROS,
                retail_tick: 1,
                main_base_abort_active: false,
            },
            &mut || {
                wait_words
                    .next()
                    .expect("the wait prefix consumes exactly two words")
            },
        );
        assert_eq!(
            waiting.outcomes,
            [SpecializedActorTaskProductionOutcome::OrdinaryType9RunAway(
                OrdinaryType9RunAwayProductionOutcome::SchedulerWaiting {
                    entity_id: actor.entity_id,
                },
            )]
        );
        assert_eq!(wait_words.next(), None);
        assert_eq!(snapshot(&manager), expected_wait);
        assert_eq!(scheduler.registered_len(), 1);

        configure_run_away_scheduler_entry(&mut manager, actor, 7, false);
        let expected_angles = [0x2345, -0x0678, 0x0123];
        let stale_basis = crate::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(
            -0x1111, 0x2222, -0x3333,
        );
        {
            let entity = manager
                .ordinary_type9_run_away_entity_mut(actor.entity_id)
                .expect("selected Run Away actor remains live");
            entity.set_rotation_heading_pitch_roll_raw(expected_angles);
            entity.physical_body_basis_q31 = RetailRuntimeValue::Known(stale_basis);
        }
        let type9_animation_claims = scheduler.actor_animation_claims().collect::<Vec<_>>();
        assert_eq!(
            type9_animation_claims,
            [actor],
            "mode-one Run Away owns the decision to preserve Sub-I, including exclusion from the neutral pass"
        );
        let mode_one_animation_before = manager
            .iter_all()
            .find(|entity| entity.id == actor.entity_id)
            .expect("selected Run Away actor remains live")
            .actor_animation_runtime;
        let mut scripted = VecDeque::from([0_u32, 0, 0x1111, 0x2222, 0xD2F6]);
        let mut consumed = Vec::new();
        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut resources,
                world_fx: &mut world_fx,
                static_damage: &mut static_damage,
                elapsed_micros: ELAPSED_MICROS,
                global_elapsed_micros: ELAPSED_MICROS,
                retail_tick: 2,
                main_base_abort_active: false,
            },
            &mut || {
                let word = scripted
                    .pop_front()
                    .expect("scheduler prefix and acquiring mover consume exactly five words");
                consumed.push(word);
                word
            },
        );
        assert_eq!(
            pass.outcomes,
            [SpecializedActorTaskProductionOutcome::OrdinaryType9RunAway(
                OrdinaryType9RunAwayProductionOutcome::PostBasisTailPending {
                    entity_id: actor.entity_id,
                    completed_phase: OrdinaryType9RunAwayActivePhase::Fleeing,
                    callback_elapsed_micros: ELAPSED_MICROS,
                },
            )]
        );
        assert_eq!(consumed, [0, 0, 0x1111, 0x2222, 0xD2F6]);
        assert!(scripted.is_empty());
        assert_eq!(
            manager
                .iter_all()
                .find(|entity| entity.id == actor.entity_id)
                .expect("selected Run Away actor remains live")
                .actor_animation_runtime,
            mode_one_animation_before,
            "mode-one task dispatch must leave the live Sub-I controller untouched"
        );
        manager.advance_unclaimed_actor_animations(ELAPSED_MICROS, &type9_animation_claims);
        assert_eq!(
            manager
                .iter_all()
                .find(|entity| entity.id == actor.entity_id)
                .expect("selected Run Away actor remains live")
                .actor_animation_runtime,
            mode_one_animation_before,
            "the later neutral pass must preserve the coarse callback's Sub-I decision"
        );
        {
            let entity = manager
                .iter_all()
                .find(|entity| entity.id == actor.entity_id)
                .expect("selected Run Away actor remains live");
            assert_eq!(
                entity.collision.recent_relation_elapsed_us_at_0x68,
                RetailRuntimeValue::Known(ELAPSED_MICROS + 7)
            );
            assert_eq!(
                entity.mass_raw,
                crate::main_base_type9_abort::LEVEL_ONE_TYPE9_MASS_RAW.wrapping_add(7)
            );
            let post_task_angles = entity.rotation_heading_pitch_roll_raw();
            assert_eq!(
                [post_task_angles[1], post_task_angles[2]],
                [expected_angles[1], expected_angles[2]],
                "ordinary 0x2F skips E640 while the mover may update heading"
            );
            assert_eq!(
                entity.physical_body_basis_q31,
                RetailRuntimeValue::Known(
                    crate::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(
                        post_task_angles[0],
                        post_task_angles[1],
                        post_task_angles[2],
                    )
                ),
                "the first callback publishes F70 from its post-task angles"
            );
            assert_ne!(
                entity.physical_body_basis_q31,
                RetailRuntimeValue::Known(stale_basis)
            );
            assert_eq!(
                entity
                    .ordinary_type9_selected_component_runtime
                    .expect("selected component custody remains published")
                    .kind(),
                OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished
            );
            let Some(ActorTaskRuntime::RunAway(fleeing)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("the synchronous handoff must publish Run Away Primary")
            };
            assert_eq!(
                fleeing.elapsed_ms(),
                0,
                "fresh slot reread skips replacement Primary"
            );
            assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
            assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        }

        let mut expected_wait = snapshot(&manager);
        expected_wait.0.callback_scheduler_accumulator_us_at_0x6c =
            RetailRuntimeValue::Known(ELAPSED_MICROS);
        expected_wait.0.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(ELAPSED_MICROS);
        let mut wait_words = [u32::MAX, u32::MAX].into_iter();
        let replay = scheduler.tick_with_scripted_random(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut resources,
                world_fx: &mut world_fx,
                static_damage: &mut static_damage,
                elapsed_micros: ELAPSED_MICROS,
                global_elapsed_micros: ELAPSED_MICROS * 2,
                retail_tick: 3,
                main_base_abort_active: false,
            },
            &mut || {
                wait_words
                    .next()
                    .expect("only the next frame's wait prefix consumes RNG")
            },
        );
        assert_eq!(
            replay.outcomes,
            [SpecializedActorTaskProductionOutcome::OrdinaryType9RunAway(
                OrdinaryType9RunAwayProductionOutcome::SchedulerWaiting {
                    entity_id: actor.entity_id,
                },
            )]
        );
        assert_eq!(wait_words.next(), None);
        assert_eq!(snapshot(&manager), expected_wait);
        assert_eq!(scheduler.registered_len(), 1);
    }

    #[test]
    fn adopted_run_away_preserves_committed_scheduler_prefix_on_wrong_audio_metadata() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        for (actual_sound_id, actual_period_raw) in [(Some(84), 0), (Some(85), 0x400)] {
            let (mut manager, mut scheduler, actor) = isolated_fresh_run_away_owner();
            configure_run_away_scheduler_entry(&mut manager, actor, 0, true);
            let metadata = manager
                .type_runtime_metadata_mut_for_test(9)
                .expect("the exact Type-9 metadata remains loaded");
            metadata.run_away_optional_sound_id = RetailRuntimeValue::Known(actual_sound_id);
            metadata.run_away_sound_period_raw = RetailRuntimeValue::Known(actual_period_raw);
            let mut expected = {
                let entity = manager
                    .iter_all()
                    .find(|entity| entity.id == actor.entity_id)
                    .unwrap();
                (
                    entity.collision.clone(),
                    entity.mass_raw,
                    entity.position,
                    entity.velocity,
                    entity.heading.to_bits(),
                    entity.current_behavior_context,
                    entity.sub_a_propulsion_runtime,
                    entity.actor_animation_runtime,
                    entity.ordinary_type9_selected_component_runtime,
                    ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .map(|slot| entity.actor_task_state(slot).copied()),
                )
            };
            expected.0.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(20_007);
            let mut resources = main_base_abort_world_cache(flat_terrain());
            let mut world_fx = WorldFx::new();

            let pass = scheduler.tick_with_scripted_random(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase:
                        crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                    resources: &mut resources,
                    world_fx: &mut world_fx,
                    static_damage: &mut static_damage,
                    elapsed_micros: 20_000,
                    global_elapsed_micros: 20_000,
                    retail_tick: 1,
                    main_base_abort_active: false,
                },
                &mut || panic!("wrong authored audio must reject before scheduler RNG"),
            );
            assert_eq!(
                pass.outcomes,
                [SpecializedActorTaskProductionOutcome::OrdinaryType9RunAway(
                    OrdinaryType9RunAwayProductionOutcome::Blocked {
                        entity_id: actor.entity_id,
                        reason: OrdinaryType9RunAwayProductionBlock::AuthoredRunAwayAudioMismatch {
                            actual_sound_id,
                            actual_period_raw,
                        },
                    },
                )]
            );
            let entity = manager
                .iter_all()
                .find(|entity| entity.id == actor.entity_id)
                .unwrap();
            assert_eq!(
                (
                    entity.collision.clone(),
                    entity.mass_raw,
                    entity.position,
                    entity.velocity,
                    entity.heading.to_bits(),
                    entity.current_behavior_context,
                    entity.sub_a_propulsion_runtime,
                    entity.actor_animation_runtime,
                    entity.ordinary_type9_selected_component_runtime,
                    ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .map(|slot| entity.actor_task_state(slot).copied()),
                ),
                expected
            );
            assert_eq!(scheduler.registered_len(), 1);
        }
    }

    #[test]
    fn main_base_run_away_noop_resume_keeps_completed_tail_authentication() {
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        let (mut manager, mut scheduler, actor, mut resources, mut world_fx) =
            progressed_run_away_owner();
        let owner = scheduler
            .take_ordinary_type9_run_away_for_main_base_abort(actor)
            .unwrap()
            .expect("completed Run Away frame may transfer to Main Base");
        let expected = owner.fork_for_main_base_abort_transaction();
        let (original_index, initial_owner, resume) = owner.decompose_for_main_base_abort();
        let restored = crate::ordinary_type9_run_away_production::OrdinaryType9RunAwayProductionOwner::resume_after_main_base_abort_noop(
            original_index,
            initial_owner,
            resume,
        ).expect("matching birth custody restores the completed frame");
        assert_eq!(restored, expected);
        assert!(scheduler
            .register_ordinary_type9_run_away(restored)
            .unwrap()
            .is_none());

        let entity = manager
            .ordinary_type9_run_away_entity_mut(actor.entity_id)
            .unwrap();
        let mut tampered_velocity = entity.velocity_raw();
        tampered_velocity[0] = tampered_velocity[0].wrapping_add(1);
        entity.set_velocity_raw(tampered_velocity);
        let pass = scheduler.tick_with_scripted_random(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut resources,
                world_fx: &mut world_fx,
                static_damage: &mut static_damage,
                elapsed_micros: 20_000,
                global_elapsed_micros: 40_000,
                retail_tick: 2,
                main_base_abort_active: false,
            },
            &mut || panic!("stale completed-tail evidence must reject before RNG"),
        );
        assert_eq!(pass.outcomes, [SpecializedActorTaskProductionOutcome::OrdinaryType9RunAway(
            OrdinaryType9RunAwayProductionOutcome::Dropped {
                entity_id: actor.entity_id,
                reason: crate::ordinary_type9_run_away_production::OrdinaryType9RunAwayProductionDrop::OuterTailStateMismatch,
            }
        )]);
        assert_eq!(scheduler.registered_len(), 0);
        assert_eq!(
            manager
                .ordinary_type9_run_away_entity_mut(actor.entity_id)
                .unwrap()
                .velocity_raw(),
            tampered_velocity
        );
    }

    #[test]
    fn main_base_type9_death_consumes_progressed_run_away_into_exploding() {
        let (mut manager, mut scheduler, actor, mut cache, mut world_fx) =
            progressed_run_away_owner();
        let observation = manager
            .main_base_abort_actor_observation(actor.entity_id)
            .expect("progressed Run Away actor remains observable");
        let terrain = flat_terrain();
        let geometry = snapshot_main_base_abort_terrain_geometry(&cache).unwrap();
        let mut static_damage = StaticDamageScheduler::new();
        let mut effects =
            MainBaseAbortWorldEffects::new(&geometry, &mut cache, &mut static_damage).unwrap();
        let mut origin = None;
        let mut player_hull = PlayerHull::default();
        let mut publications = MainBaseAbortPublicationCounts::default();

        let Ok(result) = dispatch_ordinary_actor(
            observation,
            &mut origin,
            &mut manager,
            &terrain,
            &mut player_hull,
            &mut world_fx,
            &mut effects,
            &mut scheduler,
            &mut publications,
            &mut crate::main_base_abort_production::MainBaseAbortGameplayContext {
                extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                retail_tick: 123,
            },
        ) else {
            panic!("the progressed selected owner must transfer into class 14")
        };
        assert_eq!(
            result.disposition,
            MainBaseAbortActorDisposition::Type9Death
        );
        assert_eq!(publications.type9_exploding, 1);
        assert_eq!(scheduler.registered_len(), 1);
        assert_eq!(
            scheduler.take_ordinary_type9_run_away_for_main_base_abort(actor),
            Ok(None),
            "successful death consumes Run Away custody before Exploding registration"
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == actor.entity_id)
            .expect("deferred dying actor remains linked");
        assert!(entity.ordinary_type9_selected_component_runtime.is_none());
        assert!(entity.main_base_type9_death_component_runtime.is_some());
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(task))
                if task.lifetime_ms()
                    == crate::main_base_type9_abort::LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS
        ));
    }

    #[test]
    fn main_base_type9_remote_and_dying_noops_restore_identical_progressed_run_away() {
        for state_bit in [REMOTE_OWNED_STATE_BIT, DYING_STATE_BIT] {
            let (mut manager, mut scheduler, actor, mut cache, mut world_fx) =
                progressed_run_away_owner();
            manager
                .ordinary_type9_run_away_entity_mut(actor.entity_id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(state_bit, state_bit);
            let expected = scheduler
                .take_ordinary_type9_run_away_for_main_base_abort(actor)
                .unwrap()
                .expect("progressed Run Away owner remains scheduled");
            let expected_snapshot = expected.fork_for_main_base_abort_transaction();
            assert!(scheduler
                .restore_ordinary_type9_run_away_after_main_base_abort_noop(expected)
                .unwrap()
                .is_none());
            let observation = manager
                .main_base_abort_actor_observation(actor.entity_id)
                .expect("progressed Run Away actor remains observable");
            let entity_before = {
                let entity = manager
                    .iter_all()
                    .find(|entity| entity.id == actor.entity_id)
                    .unwrap();
                (
                    entity.collision.clone(),
                    entity.mass_raw,
                    entity.current_behavior_context,
                    entity.sub_a_propulsion_runtime,
                    entity.actor_animation_runtime,
                    entity.ordinary_type9_selected_component_runtime,
                    ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .map(|slot| entity.actor_task_state(slot).copied()),
                )
            };
            let terrain = flat_terrain();
            let geometry = snapshot_main_base_abort_terrain_geometry(&cache).unwrap();
            let mut static_damage = StaticDamageScheduler::new();
            let mut effects =
                MainBaseAbortWorldEffects::new(&geometry, &mut cache, &mut static_damage).unwrap();
            let mut origin = None;
            let mut player_hull = PlayerHull::default();
            let mut publications = MainBaseAbortPublicationCounts::default();

            let Ok(result) = dispatch_ordinary_actor(
                observation,
                &mut origin,
                &mut manager,
                &terrain,
                &mut player_hull,
                &mut world_fx,
                &mut effects,
                &mut scheduler,
                &mut publications,
                &mut crate::main_base_abort_production::MainBaseAbortGameplayContext {
                    extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                    notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                    retail_tick: 123,
                },
            ) else {
                panic!("generic-death no-op must restore Run Away custody")
            };
            assert_eq!(
                result.disposition,
                MainBaseAbortActorDisposition::Type9Death
            );
            assert_eq!(publications.type9_exploding, 0);
            let restored = scheduler
                .take_ordinary_type9_run_away_for_main_base_abort(actor)
                .unwrap()
                .expect("no-op must restore the exact progressed owner");
            assert_eq!(restored, expected_snapshot, "state bit {state_bit:#x}");
            assert!(scheduler
                .restore_ordinary_type9_run_away_after_main_base_abort_noop(restored)
                .unwrap()
                .is_none());
            let entity = manager
                .iter_all()
                .find(|entity| entity.id == actor.entity_id)
                .unwrap();
            assert_eq!(
                (
                    entity.collision.clone(),
                    entity.mass_raw,
                    entity.current_behavior_context,
                    entity.sub_a_propulsion_runtime,
                    entity.actor_animation_runtime,
                    entity.ordinary_type9_selected_component_runtime,
                    ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .map(|slot| entity.actor_task_state(slot).copied()),
                ),
                entity_before,
                "generic-death no-op must not mutate the progressed actor"
            );
        }
    }

    #[test]
    fn main_base_type9_preflight_rejection_restores_progressed_run_away_custody() {
        let (mut manager, mut scheduler, actor, mut cache, mut world_fx) =
            progressed_run_away_owner();
        let expected = scheduler
            .take_ordinary_type9_run_away_for_main_base_abort(actor)
            .unwrap()
            .expect("progressed Run Away owner remains scheduled");
        let expected_snapshot = expected.fork_for_main_base_abort_transaction();
        assert!(scheduler
            .restore_ordinary_type9_run_away_after_main_base_abort_noop(expected)
            .unwrap()
            .is_none());
        manager
            .ordinary_type9_run_away_entity_mut(actor.entity_id)
            .unwrap()
            .mass_raw = crate::main_base_type9_abort::LEVEL_ONE_TYPE9_MASS_RAW.wrapping_add(1);
        let observation = manager
            .main_base_abort_actor_observation(actor.entity_id)
            .expect("progressed Run Away actor remains observable");
        let entity_before = {
            let entity = manager
                .iter_all()
                .find(|entity| entity.id == actor.entity_id)
                .unwrap();
            (
                entity.collision.clone(),
                entity.mass_raw,
                entity.current_behavior_context,
                entity.sub_a_propulsion_runtime,
                entity.actor_animation_runtime,
                entity.ordinary_type9_selected_component_runtime,
                ActorTaskSlot::IN_RETAIL_TICK_ORDER
                    .map(|slot| entity.actor_task_state(slot).copied()),
            )
        };
        let terrain = flat_terrain();
        let geometry = snapshot_main_base_abort_terrain_geometry(&cache).unwrap();
        let mut static_damage = StaticDamageScheduler::new();
        let mut effects =
            MainBaseAbortWorldEffects::new(&geometry, &mut cache, &mut static_damage).unwrap();
        let mut origin = None;
        let mut player_hull = PlayerHull::default();
        let mut publications = MainBaseAbortPublicationCounts::default();

        let failure = match dispatch_ordinary_actor(
            observation,
            &mut origin,
            &mut manager,
            &terrain,
            &mut player_hull,
            &mut world_fx,
            &mut effects,
            &mut scheduler,
            &mut publications,
            &mut crate::main_base_abort_production::MainBaseAbortGameplayContext {
                extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
                retail_tick: 123,
            },
        ) {
            Ok(_) => panic!("the mismatched mass must reject before generic-death mutation"),
            Err(failure) => failure,
        };
        assert_eq!(
            failure.callback_error,
            Some(MainBaseAbortActorCallbackBlock::Type9(
                MainBaseType9DeathBlock::TypeMetadataMismatch,
            ))
        );
        assert!(failure.diagnostic.is_none());
        assert!(failure.unscheduled_owner.is_none());
        assert_eq!(publications.type9_exploding, 0);
        assert!(manager
            .fresh_level1_type9_initial_productions()
            .iter()
            .all(|owner| owner.entity_id() != actor.entity_id));
        let restored = scheduler
            .take_ordinary_type9_run_away_for_main_base_abort(actor)
            .unwrap()
            .expect("preflight rejection must restore the exact progressed owner");
        assert_eq!(restored, expected_snapshot);

        let entity = manager
            .iter_all()
            .find(|entity| entity.id == actor.entity_id)
            .unwrap();
        assert_eq!(
            (
                entity.collision.clone(),
                entity.mass_raw,
                entity.current_behavior_context,
                entity.sub_a_propulsion_runtime,
                entity.actor_animation_runtime,
                entity.ordinary_type9_selected_component_runtime,
                ActorTaskSlot::IN_RETAIL_TICK_ORDER
                    .map(|slot| entity.actor_task_state(slot).copied()),
            ),
            entity_before,
            "death preflight rejection must leave the progressed actor untouched"
        );
    }

    #[test]
    fn main_base_type9_death_consumes_selected_go_to_job_into_exploding() {
        let (mut manager, mut scheduler, actor) = isolated_fresh_go_to_job_owner();
        let (result, publications) = dispatch_type9_for_test(&mut manager, &mut scheduler, actor);
        let result = match result {
            Ok(result) => result,
            Err(failure) => panic!(
                "selected Go-To-Job custody must transfer into class 14: callback={:?}, diagnostic={:?}, unscheduled={}",
                failure.callback_error,
                failure.diagnostic,
                failure.unscheduled_owner.is_some(),
            ),
        };
        assert_eq!(
            result.disposition,
            MainBaseAbortActorDisposition::Type9Death
        );
        assert_eq!(publications.type9_exploding, 1);
        assert_eq!(scheduler.registered_len(), 1);
        assert_eq!(
            scheduler.take_ordinary_type9_selected_for_main_base_abort(actor),
            Ok(None),
            "successful death consumes Go-To-Job before Exploding registration"
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == actor.entity_id)
            .expect("deferred dying actor remains linked");
        assert!(entity.ordinary_type9_selected_component_runtime.is_none());
        assert!(entity.main_base_type9_death_component_runtime.is_some());
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(task))
                if task.lifetime_ms()
                    == crate::main_base_type9_abort::LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS
        ));
    }

    #[test]
    fn main_base_type9_death_consumes_selected_wander_into_exploding() {
        let (mut manager, mut scheduler, actor) = isolated_fresh_wander_owner();
        let (result, publications) = dispatch_type9_for_test(&mut manager, &mut scheduler, actor);
        let result = match result {
            Ok(result) => result,
            Err(failure) => panic!(
                "selected Wander custody must transfer into class 14: callback={:?}, diagnostic={:?}, unscheduled={}",
                failure.callback_error,
                failure.diagnostic,
                failure.unscheduled_owner.is_some(),
            ),
        };
        assert_eq!(
            result.disposition,
            MainBaseAbortActorDisposition::Type9Death
        );
        assert_eq!(publications.type9_exploding, 1);
        assert_eq!(scheduler.registered_len(), 1);
        assert_eq!(
            scheduler.take_ordinary_type9_selected_for_main_base_abort(actor),
            Ok(None),
            "successful death consumes Wander before Exploding registration"
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == actor.entity_id)
            .expect("deferred dying actor remains linked");
        assert!(entity.ordinary_type9_selected_component_runtime.is_none());
        assert!(entity.main_base_type9_death_component_runtime.is_some());
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(task))
                if task.lifetime_ms()
                    == crate::main_base_type9_abort::LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS
        ));
    }

    #[test]
    fn main_base_type9_death_consumes_selected_attract_attention_into_exploding() {
        for odd_parity in [false, true] {
            let (mut manager, mut scheduler, actor) =
                isolated_fresh_attract_attention_owner(odd_parity);
            let (result, publications) =
                dispatch_type9_for_test(&mut manager, &mut scheduler, actor);
            let Ok(result) = result else {
                panic!("selected Attract Attention custody must transfer into class 14")
            };
            assert_eq!(
                result.disposition,
                MainBaseAbortActorDisposition::Type9Death
            );
            assert_eq!(publications.type9_exploding, 1);
            assert_eq!(scheduler.registered_len(), 1);
            assert_eq!(
                scheduler.take_ordinary_type9_selected_for_main_base_abort(actor),
                Ok(None),
                "successful death consumes both Attract slots before Exploding registration"
            );
            let entity = manager
                .iter_all()
                .find(|entity| entity.id == actor.entity_id)
                .expect("deferred dying actor remains linked");
            assert!(entity.ordinary_type9_selected_component_runtime.is_none());
            assert!(entity.main_base_type9_death_component_runtime.is_some());
            assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::SharedRetarget(task))
                    if task.lifetime_ms()
                        == crate::main_base_type9_abort::LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS
            ));
            assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
            assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        }
    }

    #[test]
    fn main_base_type9_death_rejects_attract_sub_a_tamper_atomically() {
        for odd_parity in [false, true] {
            let (mut manager, mut scheduler, actor) =
                isolated_fresh_attract_attention_owner(odd_parity);
            let entity = manager
                .ordinary_type9_selected_entity_mut(actor.entity_id)
                .unwrap();
            let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
                panic!("exact Attract fixture has Sub-A propulsion")
            };
            entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
                crate::common_mover::SubAPropulsionRuntime::from_retail_words(
                    sub_a.target_speed_raw(),
                    sub_a.direction_multiplier(),
                    sub_a.drive_scale_percent() + 1,
                ),
            ));
            let entity_before = {
                let entity = manager
                    .iter_all()
                    .find(|entity| entity.id == actor.entity_id)
                    .unwrap();
                (
                    entity.collision.clone(),
                    entity.mass_raw,
                    entity.current_behavior_context,
                    entity.sub_a_propulsion_runtime,
                    entity.actor_animation_runtime,
                    entity.ordinary_type9_selected_component_runtime,
                    entity.main_base_type9_death_component_runtime,
                    ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .map(|slot| entity.actor_task_state(slot).copied()),
                )
            };

            let (result, publications) =
                dispatch_type9_for_test(&mut manager, &mut scheduler, actor);

            assert!(matches!(
                result,
                Err(DispatchFailure {
                    callback_error: Some(MainBaseAbortActorCallbackBlock::Type9(
                        MainBaseType9DeathBlock::InitialBehaviorProvenanceMismatch
                    )),
                    diagnostic: None,
                    unscheduled_owner: None,
                })
            ));
            assert_eq!(publications.type9_exploding, 0);
            assert_eq!(scheduler.registered_len(), 1);
            assert!(manager
                .fresh_level1_type9_initial_productions()
                .iter()
                .all(|owner| owner.entity_id() != actor.entity_id));
            let entity = manager
                .iter_all()
                .find(|entity| entity.id == actor.entity_id)
                .unwrap();
            assert_eq!(
                (
                    entity.collision.clone(),
                    entity.mass_raw,
                    entity.current_behavior_context,
                    entity.sub_a_propulsion_runtime,
                    entity.actor_animation_runtime,
                    entity.ordinary_type9_selected_component_runtime,
                    entity.main_base_type9_death_component_runtime,
                    ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .map(|slot| entity.actor_task_state(slot).copied()),
                ),
                entity_before
            );
            let restored = scheduler
                .take_ordinary_type9_selected_for_main_base_abort(actor)
                .unwrap()
                .expect("strict preflight restores Attract scheduler custody");
            assert!(matches!(
                restored,
                OrdinaryType9SelectedProductionOwner::AttractAttention(_)
            ));
        }
    }

    #[test]
    fn main_base_type9_remote_and_dying_noops_restore_identical_wander() {
        for state_bit in [REMOTE_OWNED_STATE_BIT, DYING_STATE_BIT] {
            let (mut manager, mut scheduler, actor) = isolated_fresh_wander_owner();
            manager
                .ordinary_type9_selected_entity_mut(actor.entity_id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(state_bit, state_bit);
            let owner = scheduler
                .take_ordinary_type9_selected_for_main_base_abort(actor)
                .unwrap()
                .expect("selected Wander owner remains scheduled");
            assert!(matches!(
                owner,
                OrdinaryType9SelectedProductionOwner::Wander(_)
            ));
            let (expected, installed) = fork_selected_owner_through_resume(owner);
            assert!(scheduler
                .restore_ordinary_type9_selected_after_main_base_abort_noop(installed)
                .unwrap()
                .is_none());

            let (result, publications) =
                dispatch_type9_for_test(&mut manager, &mut scheduler, actor);
            let Ok(result) = result else {
                panic!("generic-death no-op must restore Wander")
            };
            assert_eq!(
                result.disposition,
                MainBaseAbortActorDisposition::Type9Death
            );
            assert_eq!(publications.type9_exploding, 0);
            let restored = scheduler
                .take_ordinary_type9_selected_for_main_base_abort(actor)
                .unwrap()
                .expect("no-op restores exact selected Wander owner");
            assert_eq!(restored, expected, "state bit {state_bit:#x}");
            assert!(scheduler
                .restore_ordinary_type9_selected_after_main_base_abort_noop(restored)
                .unwrap()
                .is_none());
        }
    }

    #[test]
    fn main_base_type9_remote_and_dying_noops_restore_identical_attract_attention() {
        for odd_parity in [false, true] {
            for state_bit in [REMOTE_OWNED_STATE_BIT, DYING_STATE_BIT] {
                for tamper_sub_a in [false, true] {
                    let (mut manager, mut scheduler, actor) =
                        isolated_fresh_attract_attention_owner(odd_parity);
                    let entity = manager
                        .ordinary_type9_selected_entity_mut(actor.entity_id)
                        .unwrap();
                    entity
                        .collision
                        .state_flags_at_0x08
                        .overwrite(state_bit, state_bit);
                    if tamper_sub_a {
                        let RetailRuntimeValue::Known(Some(sub_a)) =
                            entity.sub_a_propulsion_runtime
                        else {
                            panic!("exact Attract fixture has Sub-A propulsion")
                        };
                        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
                            crate::common_mover::SubAPropulsionRuntime::from_retail_words(
                                sub_a.target_speed_raw(),
                                sub_a.direction_multiplier(),
                                sub_a.drive_scale_percent() + 1,
                            ),
                        ));
                    }
                    let owner = scheduler
                        .take_ordinary_type9_selected_for_main_base_abort(actor)
                        .unwrap()
                        .expect("initial Attract graph remains transferable");
                    assert!(matches!(
                        owner,
                        OrdinaryType9SelectedProductionOwner::AttractAttention(_)
                    ));
                    let (expected, installed) = fork_selected_owner_through_resume(owner);
                    assert!(scheduler
                        .restore_ordinary_type9_selected_after_main_base_abort_noop(installed)
                        .unwrap()
                        .is_none());

                    let entity_before = {
                        let entity = manager
                            .iter_all()
                            .find(|entity| entity.id == actor.entity_id)
                            .unwrap();
                        (
                            entity.collision.clone(),
                            entity.mass_raw,
                            entity.current_behavior_context,
                            entity.sub_a_propulsion_runtime,
                            entity.actor_animation_runtime,
                            entity.ordinary_type9_selected_component_runtime,
                            ActorTaskSlot::IN_RETAIL_TICK_ORDER
                                .map(|slot| entity.actor_task_state(slot).copied()),
                        )
                    };
                    let (result, publications) =
                        dispatch_type9_for_test(&mut manager, &mut scheduler, actor);
                    let Ok(result) = result else {
                        panic!("generic-death no-op must restore Attract Attention")
                    };
                    assert_eq!(
                        result.disposition,
                        MainBaseAbortActorDisposition::Type9Death
                    );
                    assert_eq!(publications.type9_exploding, 0);
                    assert!(manager
                        .fresh_level1_type9_initial_productions()
                        .iter()
                        .all(|owner| owner.entity_id() != actor.entity_id));
                    let restored = scheduler
                        .take_ordinary_type9_selected_for_main_base_abort(actor)
                        .unwrap()
                        .expect("no-op restores exact Attract graph custody");
                    assert_eq!(
                        restored, expected,
                        "parity {odd_parity}, state bit {state_bit:#x}, tamper {tamper_sub_a}"
                    );
                    assert!(scheduler
                        .restore_ordinary_type9_selected_after_main_base_abort_noop(restored)
                        .unwrap()
                        .is_none());
                    let entity = manager
                        .iter_all()
                        .find(|entity| entity.id == actor.entity_id)
                        .unwrap();
                    assert_eq!(
                (
                    entity.collision.clone(),
                    entity.mass_raw,
                    entity.current_behavior_context,
                    entity.sub_a_propulsion_runtime,
                    entity.actor_animation_runtime,
                    entity.ordinary_type9_selected_component_runtime,
                    ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .map(|slot| entity.actor_task_state(slot).copied()),
                ),
                entity_before,
                "generic-death no-op must not mutate Attract Attention (parity {odd_parity}, state bit {state_bit:#x}, tamper {tamper_sub_a})"
            );
                }
            }
        }
    }

    #[test]
    fn main_base_type9_remote_and_dying_noops_restore_identical_go_to_job() {
        for state_bit in [REMOTE_OWNED_STATE_BIT, DYING_STATE_BIT] {
            let (mut manager, mut scheduler, actor) = isolated_fresh_go_to_job_owner();
            manager
                .ordinary_type9_selected_entity_mut(actor.entity_id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(state_bit, state_bit);
            let owner = scheduler
                .take_ordinary_type9_selected_for_main_base_abort(actor)
                .unwrap()
                .expect("selected Go-To-Job owner remains scheduled");
            assert!(matches!(
                owner,
                OrdinaryType9SelectedProductionOwner::GoToJob(_)
            ));
            let (expected, installed) = fork_selected_owner_through_resume(owner);
            assert!(scheduler
                .restore_ordinary_type9_selected_after_main_base_abort_noop(installed)
                .unwrap()
                .is_none());

            let entity_before = {
                let entity = manager
                    .iter_all()
                    .find(|entity| entity.id == actor.entity_id)
                    .unwrap();
                (
                    entity.collision.clone(),
                    entity.mass_raw,
                    entity.current_behavior_context,
                    entity.sub_a_propulsion_runtime,
                    entity.actor_animation_runtime,
                    entity.ordinary_type9_selected_component_runtime,
                    ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .map(|slot| entity.actor_task_state(slot).copied()),
                )
            };
            let (result, publications) =
                dispatch_type9_for_test(&mut manager, &mut scheduler, actor);
            let Ok(result) = result else {
                panic!("generic-death no-op must restore Go-To-Job")
            };
            assert_eq!(
                result.disposition,
                MainBaseAbortActorDisposition::Type9Death
            );
            assert_eq!(publications.type9_exploding, 0);
            let restored = scheduler
                .take_ordinary_type9_selected_for_main_base_abort(actor)
                .unwrap()
                .expect("no-op restores exact selected Go-To-Job owner");
            assert_eq!(restored, expected, "state bit {state_bit:#x}");
            assert!(scheduler
                .restore_ordinary_type9_selected_after_main_base_abort_noop(restored)
                .unwrap()
                .is_none());
            let entity = manager
                .iter_all()
                .find(|entity| entity.id == actor.entity_id)
                .unwrap();
            assert_eq!(
                (
                    entity.collision.clone(),
                    entity.mass_raw,
                    entity.current_behavior_context,
                    entity.sub_a_propulsion_runtime,
                    entity.actor_animation_runtime,
                    entity.ordinary_type9_selected_component_runtime,
                    ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .map(|slot| entity.actor_task_state(slot).copied()),
                ),
                entity_before,
                "generic-death no-op must not mutate Go-To-Job"
            );
        }
    }

    #[test]
    fn main_base_type9_preflight_rejection_restores_go_to_job_custody() {
        let (mut manager, mut scheduler, actor) = isolated_fresh_go_to_job_owner();
        let owner = scheduler
            .take_ordinary_type9_selected_for_main_base_abort(actor)
            .unwrap()
            .expect("selected Go-To-Job owner remains scheduled");
        let (expected, installed) = fork_selected_owner_through_resume(owner);
        assert!(scheduler
            .restore_ordinary_type9_selected_after_main_base_abort_noop(installed)
            .unwrap()
            .is_none());
        manager
            .ordinary_type9_selected_entity_mut(actor.entity_id)
            .unwrap()
            .mass_raw = crate::main_base_type9_abort::LEVEL_ONE_TYPE9_MASS_RAW.wrapping_add(1);

        let (result, publications) = dispatch_type9_for_test(&mut manager, &mut scheduler, actor);
        let Err(failure) = result else {
            panic!("mismatched mass must reject before generic-death mutation")
        };
        assert_eq!(
            failure.callback_error,
            Some(MainBaseAbortActorCallbackBlock::Type9(
                MainBaseType9DeathBlock::TypeMetadataMismatch,
            ))
        );
        assert!(failure.diagnostic.is_none());
        assert!(failure.unscheduled_owner.is_none());
        assert_eq!(publications.type9_exploding, 0);
        assert!(manager
            .fresh_level1_type9_initial_productions()
            .iter()
            .all(|owner| owner.entity_id() != actor.entity_id));
        let restored = scheduler
            .take_ordinary_type9_selected_for_main_base_abort(actor)
            .unwrap()
            .expect("preflight rejection restores exact selected Go-To-Job owner");
        assert_eq!(restored, expected);
    }
}
