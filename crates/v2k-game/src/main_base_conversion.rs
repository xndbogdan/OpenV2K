//! Bounded live Main Base peasant-conversion pair executor.
//!
//! Retail `FUN_00411AD0` walks the intrusive entity list with a saved `next`
//! pointer, while the Main Base behavior callback `FUN_004258A0` can
//! tail-append a replacement.  The accepted
//! `20260727-235038-base-conversion-pair-tail.txt` trace proves that this is a
//! sequential, non-transactional pass: notification and deferred destruction
//! survive spawn failure; the target component and physical suffix still run;
//! and an append behind an old tail which has not yet saved `next` is visited
//! in the same pass.
//!
//! This module owns that ordering contract but not EntityManager storage,
//! model resources, or initializer RNG. A host must prove the exact oriented
//! model contact and close the complete post-callback suffix before the first
//! action is applied. Unknown live state therefore stops before mutating the
//! current visit instead of becoming a proximity-based conversion shortcut.

use crate::entity_collision_state::RetailRuntimeValue;
use crate::entity_pair_callbacks::{
    plan_main_base_conversion, EntityPairCallbackAction, EntityPairCallbackUnresolved,
    MainBaseConversionInput, PairCallbackEntitySnapshot, MAIN_BASE_CONVERSION_RESOURCE_EVENT_ID,
};
use crate::live_list_pass::{
    try_run_saved_next_live_pass, FallibleLivePassError, FallibleLiveVisitPlan,
};

/// Runtime inputs sampled independently of the changing intrusive list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseConversionRequest {
    pub main_base_entity_id: u32,
    /// Current Section-13 world-style field `+0x48`. This selects the replacement type;
    /// it is not an RNG result.
    pub world_style: RetailRuntimeValue<u32>,
}

/// Dependent tail operation represented by
/// [`EntityPairCallbackAction::SpawnMainBaseReplacement`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseReplacementSpawn {
    /// Source entity which the preceding callback action queued for deferred
    /// destruction. The dependent replacement transaction must validate this
    /// exact source before publishing the append.
    pub source_entity_id: u32,
    pub main_base_entity_id: u32,
    pub replacement_type: u32,
    pub position_raw: [i16; 3],
}

/// Phase whose live prerequisites could not be closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseConversionPhase {
    ExactPairContact,
    TargetSnapshot,
    ReplacementPreflight,
    ContactSuffix,
}

/// Explicit cutover boundary for one sequential visit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MainBaseConversionUnresolved<HostError> {
    Host {
        candidate_id: u32,
        phase: MainBaseConversionPhase,
        source: HostError,
    },
    Callback {
        candidate_id: u32,
        source: EntityPairCallbackUnresolved,
    },
    /// A future callback-plan change must be integrated deliberately rather
    /// than being silently ignored by this bounded executor.
    CallbackJournal { candidate_id: u32 },
}

/// Observable result of applying one callback action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseConversionActionOutcome {
    Applied,
    ReplacementSpawned {
        replacement_entity_id: u32,
    },
    /// Retail spawn failure is not a pass error. Prior notification/destroy
    /// actions stay committed and the current contact suffix still runs.
    ReplacementSpawnFailed,
}

/// One saved-`next` visit, including filtered/missed candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainBaseConversionVisit<SuffixOutcome> {
    pub entity_id: u32,
    pub action_outcomes: Vec<MainBaseConversionActionOutcome>,
    /// Present only after an exact model contact whose complete suffix was
    /// preflighted and applied.
    pub suffix_outcome: Option<SuffixOutcome>,
}

/// Complete successful live-list scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainBaseConversionPass<SuffixOutcome> {
    pub visits: Vec<MainBaseConversionVisit<SuffixOutcome>>,
}

/// Sequential pass failure after zero or more earlier visits committed.
///
/// `completed` must be drained by the runtime bridge even on `Err`; it can
/// contain event-1 or successful replacement/operation-`0x33` presentation
/// outcomes which retail has already made durable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainBaseConversionFailure<HostError, SuffixOutcome> {
    pub unresolved: MainBaseConversionUnresolved<HostError>,
    pub completed: MainBaseConversionPass<SuffixOutcome>,
}

pub type MainBaseConversionResult<HostError, SuffixOutcome> = Result<
    MainBaseConversionPass<SuffixOutcome>,
    MainBaseConversionFailure<HostError, SuffixOutcome>,
>;

/// Runtime bridge required by the bounded executor.
///
/// `classify_exact_contact` must include retail active-pair eligibility,
/// domain/recent-relation gates, exact raw broad phase, and the shared oriented
/// Section-8 narrow phase. It must return `Err`, not `false`, when any
/// prerequisite is unknown.
///
/// `plan_contact_suffix` closes both directional component chains and the
/// physical response/damage suffix without mutating state or consuming shared
/// RNG. The returned plan is committed after every callback action outcome.
/// For the accepted type-9 delivery this includes its descriptor component and
/// the physical suffix observed after destruction had already been queued.
pub trait MainBaseConversionHost {
    type Contact;
    type ReplacementPlan;
    type ContactSuffixPlan;
    type ContactSuffixOutcome;
    type Unresolved;

    fn first_live_entity_id(&self) -> Option<u32>;
    fn next_live_entity_id(&self, entity_id: u32) -> Option<u32>;

    fn classify_exact_contact(
        &self,
        main_base_entity_id: u32,
        candidate_id: u32,
    ) -> Result<Option<Self::Contact>, Self::Unresolved>;

    fn target_snapshot(
        &self,
        candidate_id: u32,
    ) -> Result<PairCallbackEntitySnapshot, Self::Unresolved>;

    /// Close every non-allocation prerequisite of the dependent replacement
    /// before notification or deferred destruction can become visible. The
    /// returned plan may retain immutable world decisions, but must not consume
    /// initializer/shared RNG.
    fn plan_main_base_replacement(
        &self,
        request: MainBaseReplacementSpawn,
    ) -> Result<Self::ReplacementPlan, Self::Unresolved>;

    /// Close the suffix before notification/destruction/spawn can mutate the
    /// current visit. This phase must not consume initializer/component RNG.
    fn plan_contact_suffix(
        &self,
        main_base_entity_id: u32,
        candidate_id: u32,
        contact: &Self::Contact,
    ) -> Result<Self::ContactSuffixPlan, Self::Unresolved>;

    fn queue_resource_notification(&mut self, event_id: u8);
    fn queue_deferred_destroy(&mut self, entity_id: u32);

    /// Attempt one dependent allocation transaction. On success the host must
    /// tail-append it, set replacement `+0x60` to the Main Base, and emit
    /// operation `0x33` from the replacement. On failure it must do none of
    /// those dependent steps and return `None`. Initializer/shared RNG belongs
    /// inside this apply phase, never preflight.
    fn spawn_main_base_replacement(&mut self, plan: Self::ReplacementPlan) -> Option<u32>;

    /// Commit a suffix which was completely closed before the first action.
    /// It is intentionally infallible: an unresolved suffix belongs in
    /// `plan_contact_suffix`, before any non-rollback action is visible.
    fn apply_contact_suffix(
        &mut self,
        candidate_id: u32,
        plan: Self::ContactSuffixPlan,
    ) -> Self::ContactSuffixOutcome;
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PlannedMainBaseConversionAction<ReplacementPlan> {
    QueueResourceNotification { event_id: u8 },
    QueueDeferredDestroy { entity_id: u32 },
    SpawnMainBaseReplacement(ReplacementPlan),
}

/// Run the bounded Main Base conversion scan in retail live-list order.
pub fn run_main_base_conversion_pass<H: MainBaseConversionHost>(
    host: &mut H,
    request: MainBaseConversionRequest,
) -> MainBaseConversionResult<H::Unresolved, H::ContactSuffixOutcome> {
    let first_entity_id = host.first_live_entity_id();
    let executions = try_run_saved_next_live_pass(
        host,
        first_entity_id,
        H::next_live_entity_id,
        |host, candidate_id| plan_visit(host, request, candidate_id),
        apply_action::<H>,
        H::apply_contact_suffix,
    )
    .map_err(
        |FallibleLivePassError {
             source,
             completed_executions,
             ..
         }| MainBaseConversionFailure {
            unresolved: source,
            completed: pass_from_executions(completed_executions),
        },
    )?;

    Ok(pass_from_executions(executions))
}

fn pass_from_executions<SuffixOutcome>(
    executions: Vec<
        crate::live_list_pass::FallibleLiveVisitExecution<
            u32,
            MainBaseConversionActionOutcome,
            SuffixOutcome,
        >,
    >,
) -> MainBaseConversionPass<SuffixOutcome> {
    MainBaseConversionPass {
        visits: executions
            .into_iter()
            .map(|execution| MainBaseConversionVisit {
                entity_id: execution.entity_id,
                action_outcomes: execution.action_outcomes,
                suffix_outcome: execution.suffix_outcome,
            })
            .collect(),
    }
}

fn plan_visit<H: MainBaseConversionHost>(
    host: &H,
    request: MainBaseConversionRequest,
    candidate_id: u32,
) -> Result<
    FallibleLiveVisitPlan<
        PlannedMainBaseConversionAction<H::ReplacementPlan>,
        H::ContactSuffixPlan,
    >,
    MainBaseConversionUnresolved<H::Unresolved>,
> {
    if candidate_id == request.main_base_entity_id {
        return Ok(FallibleLiveVisitPlan {
            actions: Vec::new(),
            suffix: None,
        });
    }

    let contact = host
        .classify_exact_contact(request.main_base_entity_id, candidate_id)
        .map_err(|source| MainBaseConversionUnresolved::Host {
            candidate_id,
            phase: MainBaseConversionPhase::ExactPairContact,
            source,
        })?;
    let Some(contact) = contact else {
        return Ok(FallibleLiveVisitPlan {
            actions: Vec::new(),
            suffix: None,
        });
    };
    let target = host.target_snapshot(candidate_id).map_err(|source| {
        MainBaseConversionUnresolved::Host {
            candidate_id,
            phase: MainBaseConversionPhase::TargetSnapshot,
            source,
        }
    })?;
    let callback = plan_main_base_conversion(MainBaseConversionInput {
        main_base_entity_id: request.main_base_entity_id,
        target: RetailRuntimeValue::Known(Some(target)),
        world_style: request.world_style,
    })
    .map_err(|source| MainBaseConversionUnresolved::Callback {
        candidate_id,
        source,
    })?;
    validate_bounded_journal(request.main_base_entity_id, candidate_id, &callback.actions)
        .map_err(|()| MainBaseConversionUnresolved::CallbackJournal { candidate_id })?;
    // A geometrically contacting entity which lacks Main Base's target
    // capability still reaches the behavior callback, but that callback
    // returns null without converting it. Its ordinary component/physical
    // handling remains owned by the general active-pair pass; this bounded
    // transaction must not replay that suffix.
    if callback.actions.is_empty() {
        return Ok(FallibleLiveVisitPlan {
            actions: Vec::new(),
            suffix: None,
        });
    }
    let mut planned_actions = Vec::with_capacity(callback.actions.len());
    for action in callback.actions {
        let planned = match action {
            EntityPairCallbackAction::QueueResourceNotification { event_id } => {
                PlannedMainBaseConversionAction::QueueResourceNotification { event_id }
            }
            EntityPairCallbackAction::QueueDeferredDestroy { entity_id } => {
                PlannedMainBaseConversionAction::QueueDeferredDestroy { entity_id }
            }
            EntityPairCallbackAction::SpawnMainBaseReplacement {
                main_base_entity_id,
                replacement_type,
                position_raw,
            } => {
                let replacement = host
                    .plan_main_base_replacement(MainBaseReplacementSpawn {
                        source_entity_id: candidate_id,
                        main_base_entity_id,
                        replacement_type,
                        position_raw,
                    })
                    .map_err(|source| MainBaseConversionUnresolved::Host {
                        candidate_id,
                        phase: MainBaseConversionPhase::ReplacementPreflight,
                        source,
                    })?;
                PlannedMainBaseConversionAction::SpawnMainBaseReplacement(replacement)
            }
            _ => unreachable!("bounded Main Base journal was validated above"),
        };
        planned_actions.push(planned);
    }
    let suffix = host
        .plan_contact_suffix(request.main_base_entity_id, candidate_id, &contact)
        .map_err(|source| MainBaseConversionUnresolved::Host {
            candidate_id,
            phase: MainBaseConversionPhase::ContactSuffix,
            source,
        })?;

    Ok(FallibleLiveVisitPlan {
        actions: planned_actions,
        suffix: Some(suffix),
    })
}

fn validate_bounded_journal(
    main_base_entity_id: u32,
    candidate_id: u32,
    actions: &[EntityPairCallbackAction],
) -> Result<(), ()> {
    match actions {
        [] => Ok(()),
        [EntityPairCallbackAction::QueueResourceNotification {
            event_id: MAIN_BASE_CONVERSION_RESOURCE_EVENT_ID,
        }, EntityPairCallbackAction::QueueDeferredDestroy { entity_id }, EntityPairCallbackAction::SpawnMainBaseReplacement {
            main_base_entity_id: action_main_base_id,
            ..
        }] if *entity_id == candidate_id && *action_main_base_id == main_base_entity_id => Ok(()),
        _ => Err(()),
    }
}

fn apply_action<H: MainBaseConversionHost>(
    host: &mut H,
    _candidate_id: u32,
    action: PlannedMainBaseConversionAction<H::ReplacementPlan>,
) -> MainBaseConversionActionOutcome {
    match action {
        PlannedMainBaseConversionAction::QueueResourceNotification { event_id } => {
            host.queue_resource_notification(event_id);
            MainBaseConversionActionOutcome::Applied
        }
        PlannedMainBaseConversionAction::QueueDeferredDestroy { entity_id } => {
            host.queue_deferred_destroy(entity_id);
            MainBaseConversionActionOutcome::Applied
        }
        PlannedMainBaseConversionAction::SpawnMainBaseReplacement(plan) => {
            match host.spawn_main_base_replacement(plan) {
                Some(replacement_entity_id) => {
                    MainBaseConversionActionOutcome::ReplacementSpawned {
                        replacement_entity_id,
                    }
                }
                None => MainBaseConversionActionOutcome::ReplacementSpawnFailed,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    const MAIN_BASE_ID: u32 = 0x04B5_0001;
    const PEASANT_ID: u32 = 0x04C2_0001;
    const REPLACEMENT_ID: u32 = 0x0497_0001;
    const PEASANT_POSITION_RAW: [i16; 3] = [0x50BA, -0x02FE, 0x39EC];

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum HostError {
        Contact,
        Snapshot,
        Replacement,
        Suffix,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum HostEvent {
        Resource(u8),
        DeferredDestroy(u32),
        Spawn(MainBaseReplacementSpawn),
        Suffix(u32),
    }

    #[derive(Debug)]
    struct Host {
        next: BTreeMap<u32, Option<u32>>,
        snapshots: BTreeMap<u32, PairCallbackEntitySnapshot>,
        contacts: BTreeMap<u32, Result<bool, HostError>>,
        replacement_errors: BTreeMap<u32, HostError>,
        suffix_errors: BTreeMap<u32, HostError>,
        spawn_succeeds: bool,
        events: Vec<HostEvent>,
        rng_draws: usize,
    }

    impl Host {
        fn chain(ids: &[u32]) -> Self {
            Self {
                next: ids
                    .iter()
                    .enumerate()
                    .map(|(index, &id)| (id, ids.get(index + 1).copied()))
                    .collect(),
                snapshots: BTreeMap::new(),
                contacts: BTreeMap::new(),
                replacement_errors: BTreeMap::new(),
                suffix_errors: BTreeMap::new(),
                spawn_succeeds: true,
                events: Vec::new(),
                rng_draws: 0,
            }
        }

        fn add_peasant(&mut self) {
            self.snapshots.insert(
                PEASANT_ID,
                PairCallbackEntitySnapshot {
                    id: PEASANT_ID,
                    position_raw: PEASANT_POSITION_RAW,
                    state_flags_raw: 0x06C6_8825,
                    capability_flags_raw: 0x0800,
                },
            );
            self.contacts.insert(PEASANT_ID, Ok(true));
        }

        fn append(&mut self, entity_id: u32) {
            let tail = self
                .next
                .iter()
                .find_map(|(&id, &next)| next.is_none().then_some(id))
                .expect("test chain must have a tail");
            self.next.insert(entity_id, None);
            self.next.insert(tail, Some(entity_id));
            self.contacts.insert(entity_id, Ok(false));
        }
    }

    impl MainBaseConversionHost for Host {
        type Contact = ();
        type ReplacementPlan = MainBaseReplacementSpawn;
        type ContactSuffixPlan = ();
        type ContactSuffixOutcome = ();
        type Unresolved = HostError;

        fn first_live_entity_id(&self) -> Option<u32> {
            self.next.keys().find_map(|&candidate| {
                (!self
                    .next
                    .values()
                    .any(|successor| *successor == Some(candidate)))
                .then_some(candidate)
            })
        }

        fn next_live_entity_id(&self, entity_id: u32) -> Option<u32> {
            self.next.get(&entity_id).copied().flatten()
        }

        fn classify_exact_contact(
            &self,
            _main_base_entity_id: u32,
            candidate_id: u32,
        ) -> Result<Option<Self::Contact>, Self::Unresolved> {
            match self
                .contacts
                .get(&candidate_id)
                .copied()
                .unwrap_or(Ok(false))?
            {
                true => Ok(Some(())),
                false => Ok(None),
            }
        }

        fn target_snapshot(
            &self,
            candidate_id: u32,
        ) -> Result<PairCallbackEntitySnapshot, Self::Unresolved> {
            self.snapshots
                .get(&candidate_id)
                .copied()
                .ok_or(HostError::Snapshot)
        }

        fn plan_main_base_replacement(
            &self,
            request: MainBaseReplacementSpawn,
        ) -> Result<Self::ReplacementPlan, Self::Unresolved> {
            match self
                .replacement_errors
                .get(&request.source_entity_id)
                .copied()
            {
                Some(error) => Err(error),
                None => Ok(request),
            }
        }

        fn plan_contact_suffix(
            &self,
            _main_base_entity_id: u32,
            candidate_id: u32,
            _contact: &Self::Contact,
        ) -> Result<Self::ContactSuffixPlan, Self::Unresolved> {
            match self.suffix_errors.get(&candidate_id).copied() {
                Some(error) => Err(error),
                None => Ok(()),
            }
        }

        fn queue_resource_notification(&mut self, event_id: u8) {
            self.events.push(HostEvent::Resource(event_id));
        }

        fn queue_deferred_destroy(&mut self, entity_id: u32) {
            self.events.push(HostEvent::DeferredDestroy(entity_id));
        }

        fn spawn_main_base_replacement(&mut self, request: Self::ReplacementPlan) -> Option<u32> {
            self.events.push(HostEvent::Spawn(request));
            if !self.spawn_succeeds {
                return None;
            }
            // The accepted type-8 JobNearby initializer consumes shared RNG
            // only once the spawn action is actually attempted.
            self.rng_draws += 1;
            self.append(REPLACEMENT_ID);
            Some(REPLACEMENT_ID)
        }

        fn apply_contact_suffix(
            &mut self,
            candidate_id: u32,
            (): Self::ContactSuffixPlan,
        ) -> Self::ContactSuffixOutcome {
            self.events.push(HostEvent::Suffix(candidate_id));
        }
    }

    fn request() -> MainBaseConversionRequest {
        MainBaseConversionRequest {
            main_base_entity_id: MAIN_BASE_ID,
            world_style: RetailRuntimeValue::Known(1),
        }
    }

    #[test]
    fn accepted_capture_order_keeps_suffix_after_spawn_and_visits_tail_append() {
        let old_tail = 0x04D0_0001;
        let mut host = Host::chain(&[MAIN_BASE_ID, PEASANT_ID, old_tail]);
        host.add_peasant();

        let pass = run_main_base_conversion_pass(&mut host, request()).unwrap();

        assert_eq!(
            pass.visits
                .iter()
                .map(|visit| visit.entity_id)
                .collect::<Vec<_>>(),
            [MAIN_BASE_ID, PEASANT_ID, old_tail, REPLACEMENT_ID]
        );
        assert_eq!(
            host.events,
            [
                HostEvent::Resource(MAIN_BASE_CONVERSION_RESOURCE_EVENT_ID),
                HostEvent::DeferredDestroy(PEASANT_ID),
                HostEvent::Spawn(MainBaseReplacementSpawn {
                    source_entity_id: PEASANT_ID,
                    main_base_entity_id: MAIN_BASE_ID,
                    replacement_type: 8,
                    position_raw: PEASANT_POSITION_RAW,
                }),
                HostEvent::Suffix(PEASANT_ID),
            ]
        );
        assert_eq!(host.rng_draws, 1);
        assert_eq!(
            pass.visits[1].action_outcomes,
            [
                MainBaseConversionActionOutcome::Applied,
                MainBaseConversionActionOutcome::Applied,
                MainBaseConversionActionOutcome::ReplacementSpawned {
                    replacement_entity_id: REPLACEMENT_ID,
                },
            ]
        );
    }

    #[test]
    fn append_from_a_source_which_saved_tail_next_waits_until_next_pass() {
        let mut host = Host::chain(&[MAIN_BASE_ID, PEASANT_ID]);
        host.add_peasant();

        let pass = run_main_base_conversion_pass(&mut host, request()).unwrap();

        assert_eq!(
            pass.visits
                .iter()
                .map(|visit| visit.entity_id)
                .collect::<Vec<_>>(),
            [MAIN_BASE_ID, PEASANT_ID]
        );
        assert_eq!(host.next_live_entity_id(PEASANT_ID), Some(REPLACEMENT_ID));
    }

    #[test]
    fn spawn_failure_preserves_prior_actions_and_still_runs_suffix() {
        let mut host = Host::chain(&[MAIN_BASE_ID, PEASANT_ID]);
        host.add_peasant();
        host.spawn_succeeds = false;

        let pass = run_main_base_conversion_pass(&mut host, request()).unwrap();

        assert_eq!(
            host.events,
            [
                HostEvent::Resource(MAIN_BASE_CONVERSION_RESOURCE_EVENT_ID),
                HostEvent::DeferredDestroy(PEASANT_ID),
                HostEvent::Spawn(MainBaseReplacementSpawn {
                    source_entity_id: PEASANT_ID,
                    main_base_entity_id: MAIN_BASE_ID,
                    replacement_type: 8,
                    position_raw: PEASANT_POSITION_RAW,
                }),
                HostEvent::Suffix(PEASANT_ID),
            ]
        );
        assert_eq!(host.rng_draws, 0);
        assert_eq!(
            pass.visits[1].action_outcomes[2],
            MainBaseConversionActionOutcome::ReplacementSpawnFailed
        );
        assert_eq!(pass.visits[1].suffix_outcome, Some(()));
    }

    #[test]
    fn unresolved_contact_replacement_or_suffix_applies_no_current_visit_action_or_rng() {
        for failure in [
            (
                MainBaseConversionPhase::ExactPairContact,
                HostError::Contact,
            ),
            (
                MainBaseConversionPhase::ReplacementPreflight,
                HostError::Replacement,
            ),
            (MainBaseConversionPhase::ContactSuffix, HostError::Suffix),
        ] {
            let mut host = Host::chain(&[MAIN_BASE_ID, PEASANT_ID]);
            host.add_peasant();
            match failure.0 {
                MainBaseConversionPhase::ExactPairContact => {
                    host.contacts.insert(PEASANT_ID, Err(failure.1));
                }
                MainBaseConversionPhase::ReplacementPreflight => {
                    host.replacement_errors.insert(PEASANT_ID, failure.1);
                }
                MainBaseConversionPhase::ContactSuffix => {
                    host.suffix_errors.insert(PEASANT_ID, failure.1);
                }
                MainBaseConversionPhase::TargetSnapshot => unreachable!(),
            }

            let error = run_main_base_conversion_pass(&mut host, request()).unwrap_err();

            assert!(matches!(
                error.unresolved,
                MainBaseConversionUnresolved::Host {
                    candidate_id: PEASANT_ID,
                    phase,
                    source,
                } if phase == failure.0 && source == failure.1
            ));
            assert_eq!(
                error
                    .completed
                    .visits
                    .iter()
                    .map(|visit| visit.entity_id)
                    .collect::<Vec<_>>(),
                [MAIN_BASE_ID]
            );
            assert!(host.events.is_empty());
            assert_eq!(host.rng_draws, 0);
        }
    }

    #[test]
    fn unresolved_world_style_is_closed_before_notification_or_destroy() {
        let mut host = Host::chain(&[MAIN_BASE_ID, PEASANT_ID]);
        host.add_peasant();
        let error = run_main_base_conversion_pass(
            &mut host,
            MainBaseConversionRequest {
                main_base_entity_id: MAIN_BASE_ID,
                world_style: RetailRuntimeValue::Unresolved,
            },
        )
        .unwrap_err();

        assert_eq!(
            error.unresolved,
            MainBaseConversionUnresolved::Callback {
                candidate_id: PEASANT_ID,
                source: EntityPairCallbackUnresolved::MainBaseWorldStyle,
            }
        );
        assert!(host.events.is_empty());
        assert_eq!(host.rng_draws, 0);
    }

    #[test]
    fn exact_contact_without_target_capability_stays_out_of_bounded_suffix() {
        let mut host = Host::chain(&[MAIN_BASE_ID, PEASANT_ID]);
        host.snapshots.insert(
            PEASANT_ID,
            PairCallbackEntitySnapshot {
                id: PEASANT_ID,
                position_raw: PEASANT_POSITION_RAW,
                state_flags_raw: 0,
                capability_flags_raw: 0,
            },
        );
        host.contacts.insert(PEASANT_ID, Ok(true));

        let pass = run_main_base_conversion_pass(&mut host, request()).unwrap();

        assert!(host.events.is_empty());
        assert!(pass.visits[1].action_outcomes.is_empty());
        assert_eq!(pass.visits[1].suffix_outcome, None);
    }

    #[test]
    fn later_unresolved_visit_retains_already_committed_presentation_outcomes() {
        let unresolved_tail = 0x04D0_0001;
        let mut host = Host::chain(&[MAIN_BASE_ID, PEASANT_ID, unresolved_tail]);
        host.add_peasant();
        host.contacts
            .insert(unresolved_tail, Err(HostError::Contact));

        let error = run_main_base_conversion_pass(&mut host, request()).unwrap_err();

        assert!(matches!(
            error.unresolved,
            MainBaseConversionUnresolved::Host {
                candidate_id,
                phase: MainBaseConversionPhase::ExactPairContact,
                source: HostError::Contact,
            } if candidate_id == unresolved_tail
        ));
        assert_eq!(
            error
                .completed
                .visits
                .iter()
                .map(|visit| visit.entity_id)
                .collect::<Vec<_>>(),
            [MAIN_BASE_ID, PEASANT_ID]
        );
        assert_eq!(
            error.completed.visits[1].action_outcomes,
            [
                MainBaseConversionActionOutcome::Applied,
                MainBaseConversionActionOutcome::Applied,
                MainBaseConversionActionOutcome::ReplacementSpawned {
                    replacement_entity_id: REPLACEMENT_ID,
                },
            ]
        );
        assert_eq!(
            host.events,
            [
                HostEvent::Resource(MAIN_BASE_CONVERSION_RESOURCE_EVENT_ID),
                HostEvent::DeferredDestroy(PEASANT_ID),
                HostEvent::Spawn(MainBaseReplacementSpawn {
                    source_entity_id: PEASANT_ID,
                    main_base_entity_id: MAIN_BASE_ID,
                    replacement_type: 8,
                    position_raw: PEASANT_POSITION_RAW,
                }),
                HostEvent::Suffix(PEASANT_ID),
            ]
        );
    }
}
