//! Per-level diagnostics for selected peasants, independent of other actors.

use std::{collections::HashSet, fmt::Debug};

use v2k_game::{
    ordinary_type9_attract_attention_production::OrdinaryType9AttractAttentionProductionOutcome as Attract,
    ordinary_type9_go_to_job_production::OrdinaryType9GoToJobProductionOutcome as GoToJob,
    ordinary_type9_run_away_production::OrdinaryType9RunAwayProductionOutcome as RunAway,
    ordinary_type9_wander_production::OrdinaryType9WanderProductionOutcome as Wander,
    specialized_actor_task_production::SpecializedActorTaskProductionOutcome as Outcome,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FailureKind {
    Blocked,
    Dropped,
}

struct Failure<'a> {
    entity_id: u32,
    family: &'static str,
    kind: FailureKind,
    reason: &'a dyn Debug,
}

impl<'a> Failure<'a> {
    fn from_wander(outcome: &'a Wander) -> Option<Self> {
        match outcome {
            Wander::Blocked { entity_id, reason } => Some(Self {
                entity_id: *entity_id,
                family: "Wander",
                kind: FailureKind::Blocked,
                reason,
            }),
            Wander::Dropped { entity_id, reason } => Some(Self {
                entity_id: *entity_id,
                family: "Wander",
                kind: FailureKind::Dropped,
                reason,
            }),
            _ => None,
        }
    }

    fn from_outcome(outcome: &'a Outcome) -> Option<Self> {
        let (entity_id, family, kind, reason): (_, _, _, &dyn Debug) = match outcome {
            Outcome::OrdinaryType9Wander(outcome) => return Self::from_wander(outcome),
            Outcome::OrdinaryType9RunAway(RunAway::RootContinuation { outcome, .. }) => {
                return Self::from_wander(outcome);
            }
            Outcome::OrdinaryType9RunAway(RunAway::Blocked { entity_id, reason }) => {
                (entity_id, "Run Away", FailureKind::Blocked, reason)
            }
            Outcome::OrdinaryType9RunAway(RunAway::Dropped { entity_id, reason }) => {
                (entity_id, "Run Away", FailureKind::Dropped, reason)
            }
            Outcome::OrdinaryType9GoToJob(GoToJob::Blocked { entity_id, reason }) => {
                (entity_id, "Go-To-Job", FailureKind::Blocked, reason)
            }
            Outcome::OrdinaryType9GoToJob(GoToJob::Dropped { entity_id, reason }) => {
                (entity_id, "Go-To-Job", FailureKind::Dropped, reason)
            }
            Outcome::OrdinaryType9AttractAttention(Attract::Blocked { entity_id, reason }) => {
                (entity_id, "Attract Attention", FailureKind::Blocked, reason)
            }
            Outcome::OrdinaryType9AttractAttention(Attract::Dropped { entity_id, reason }) => {
                (entity_id, "Attract Attention", FailureKind::Dropped, reason)
            }
            _ => return None,
        };
        Some(Self {
            entity_id: *entity_id,
            family,
            kind,
            reason,
        })
    }
}

/// Report the first block and first drop for each actor/family in this level.
/// A parked callback can return every frame; another actor or a later drop
/// must remain independently visible without repeating the parked report.
#[derive(Default)]
pub(super) struct Type9TaskDiagnostics {
    reported: HashSet<(u32, &'static str, FailureKind)>,
}

pub(super) struct Type9DiagnosticActor {
    pub active: bool,
    pub authored_spawn_index: Option<usize>,
}

impl Type9TaskDiagnostics {
    pub(super) fn reset_level(&mut self) {
        self.reported.clear();
    }

    pub(super) fn observe(
        &mut self,
        outcome: &Outcome,
        retail_tick: u32,
        lookup_actor: impl FnOnce(u32) -> Option<Type9DiagnosticActor>,
    ) -> Option<String> {
        let failure = Failure::from_outcome(outcome)?;
        let key = (failure.entity_id, failure.family, failure.kind);
        if self.reported.contains(&key) {
            return None;
        }
        let actor = lookup_actor(failure.entity_id);
        // The scheduler also drops receipts for allocations retired normally.
        // Only a still-active actor losing its owner is a movement failure.
        if failure.kind == FailureKind::Dropped && !actor.as_ref().is_some_and(|actor| actor.active)
        {
            return None;
        }
        self.reported.insert(key);
        let authored_spawn_index = actor.and_then(|actor| actor.authored_spawn_index);
        Some(format!(
            "Ordinary Type-9 {} entity {} (spawn {authored_spawn_index:?}, tick {retail_tick}) {:?}: {:?}",
            failure.family, failure.entity_id, failure.kind, failure.reason,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_game::{
        ordinary_type9_attract_attention_production::OrdinaryType9AttractAttentionProductionBlock as AttractBlock,
        ordinary_type9_go_to_job_production::OrdinaryType9GoToJobProductionBlock as GoToJobBlock,
        ordinary_type9_run_away_production::{
            OrdinaryType9RunAwayActivePhase, OrdinaryType9RunAwayProductionBlock as RunAwayBlock,
        },
        ordinary_type9_wander_production::{
            OrdinaryType9WanderProductionBlock as WanderBlock,
            OrdinaryType9WanderProductionDrop as WanderDrop,
        },
    };

    fn blocked(id: u32) -> Outcome {
        Outcome::OrdinaryType9Wander(Wander::Blocked {
            entity_id: id,
            reason: WanderBlock::CallbackMassUnavailable,
        })
    }

    fn active_actor(entity_id: u32) -> Option<Type9DiagnosticActor> {
        Some(Type9DiagnosticActor {
            active: true,
            authored_spawn_index: Some(entity_id.saturating_sub(2) as usize),
        })
    }

    #[test]
    fn completed_tails_and_waits_do_not_consume_failure_reports() {
        let mut diagnostics = Type9TaskDiagnostics::default();
        for outcome in [
            Outcome::OrdinaryType9Wander(Wander::PostBasisTailPending {
                entity_id: 17,
                callback_elapsed_micros: 20_000,
            }),
            Outcome::OrdinaryType9GoToJob(GoToJob::PostBasisTailPending {
                entity_id: 17,
                callback_elapsed_micros: 20_000,
            }),
            Outcome::OrdinaryType9RunAway(RunAway::PostBasisTailPending {
                entity_id: 17,
                completed_phase: OrdinaryType9RunAwayActivePhase::Acquiring,
                callback_elapsed_micros: 20_000,
            }),
            Outcome::OrdinaryType9AttractAttention(Attract::OuterTailComplete {
                entity_id: 17,
                callback_elapsed_micros: 20_000,
            }),
            Outcome::OrdinaryType9Wander(Wander::SchedulerWaiting { entity_id: 17 }),
        ] {
            assert!(diagnostics
                .observe(&outcome, 1, |_| panic!("success must not look up an actor"))
                .is_none());
        }
        let message = diagnostics.observe(&blocked(17), 2, active_actor).unwrap();
        assert!(message.contains("entity 17"));
        assert!(message.contains("tick 2"));
        assert!(message.contains("spawn Some(15)"));
        assert!(message.contains("Blocked: CallbackMassUnavailable"));
    }

    #[test]
    fn failures_are_independent_per_actor_family_and_status_and_reset_per_level() {
        let mut diagnostics = Type9TaskDiagnostics::default();
        assert!(diagnostics.observe(&blocked(12), 1, active_actor).is_some());
        assert!(diagnostics.observe(&blocked(17), 1, active_actor).is_some());
        for tick in 2..100 {
            assert!(diagnostics
                .observe(&blocked(17), tick, |_| panic!(
                    "duplicate must not look up an actor"
                ))
                .is_none());
        }
        for outcome in [
            Outcome::OrdinaryType9GoToJob(GoToJob::Blocked {
                entity_id: 17,
                reason: GoToJobBlock::CallbackMassUnavailable,
            }),
            Outcome::OrdinaryType9RunAway(RunAway::Blocked {
                entity_id: 17,
                reason: RunAwayBlock::CallbackMassUnavailable,
            }),
            Outcome::OrdinaryType9AttractAttention(Attract::Blocked {
                entity_id: 17,
                reason: AttractBlock::CallbackMassUnavailable,
            }),
            Outcome::OrdinaryType9Wander(Wander::Dropped {
                entity_id: 17,
                reason: WanderDrop::OuterTailStateMismatch,
            }),
        ] {
            assert!(diagnostics.observe(&outcome, 100, active_actor).is_some());
            assert!(diagnostics.observe(&outcome, 101, active_actor).is_none());
        }
        diagnostics.reset_level();
        assert!(diagnostics.observe(&blocked(17), 1, active_actor).is_some());
    }

    #[test]
    fn run_away_root_continuation_preserves_its_nested_failure() {
        let mut diagnostics = Type9TaskDiagnostics::default();
        let outcome = Outcome::OrdinaryType9RunAway(RunAway::RootContinuation {
            entity_id: 17,
            outcome: Box::new(Wander::Dropped {
                entity_id: 17,
                reason: WanderDrop::RootPublicationMismatch,
            }),
        });
        let message = diagnostics.observe(&outcome, 75, active_actor).unwrap();
        assert!(message.contains("Wander entity 17"));
        assert!(message.contains("Dropped: RootPublicationMismatch"));
    }

    #[test]
    fn normal_retirement_is_quiet_but_an_active_actor_drop_remains_reportable() {
        let mut diagnostics = Type9TaskDiagnostics::default();
        let outcome = Outcome::OrdinaryType9Wander(Wander::Dropped {
            entity_id: 17,
            reason: WanderDrop::EntityUnavailable,
        });
        assert!(diagnostics.observe(&outcome, 10, |_| None).is_none());
        assert!(diagnostics
            .observe(&outcome, 10, |_| Some(Type9DiagnosticActor {
                active: false,
                authored_spawn_index: Some(15),
            }))
            .is_none());
        let message = diagnostics.observe(&outcome, 11, active_actor).unwrap();
        assert!(message.contains("Dropped: EntityUnavailable"));
    }
}
