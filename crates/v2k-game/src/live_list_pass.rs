//! Mutation-safe traversal for retail intrusive-list callback passes.
//!
//! Retail does not freeze the candidate list at pass entry. It saves the
//! current node's `next` link before invoking callbacks, applies callback work
//! in order, runs the candidate suffix even when destruction was queued, and
//! then advances to that saved link. Consequently, a tail append can be
//! observed in the same pass only when the old tail has not saved its `next`
//! yet.
//!
//! This contract is established by the accepted Main Base conversion traces:
//! `20260727-235038-base-conversion-pair-tail.txt` and
//! `20260724-040316-base-conversion-selector.txt`. The module deliberately
//! knows nothing about entities or conversion actions; gameplay wiring remains
//! separate until the complete live callback executor is closed.

/// Results from applying one visit's already-closed action journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiveVisitExecution<Id, Outcome> {
    pub entity_id: Id,
    pub action_outcomes: Vec<Outcome>,
}

/// One fallible visit closed completely before its first mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FallibleLiveVisitPlan<Action, Suffix> {
    pub actions: Vec<Action>,
    /// `None` denotes a non-contact/filtered visit. A retained suffix is
    /// committed after every action outcome, including a failed spawn.
    pub suffix: Option<Suffix>,
}

/// Results from one fallible visit after its suffix has run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FallibleLiveVisitExecution<Id, Outcome, SuffixOutcome> {
    pub entity_id: Id,
    pub action_outcomes: Vec<Outcome>,
    pub suffix_outcome: Option<SuffixOutcome>,
}

/// A visit which could not be closed before mutation.
///
/// Earlier completed visits remain committed, matching retail's sequential
/// intrusive-list pass. The rejected visit itself has applied no action and
/// consumed no callback-owned state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FallibleLivePassError<Id, Error, Outcome, SuffixOutcome> {
    pub entity_id: Id,
    pub source: Error,
    /// Visits committed before the unresolved current entry. Their external
    /// outcomes must still be delivered by the caller; retail does not roll
    /// those actions back.
    pub completed_executions: Vec<FallibleLiveVisitExecution<Id, Outcome, SuffixOutcome>>,
}

type FallibleLivePassResult<Id, Outcome, SuffixOutcome, Error> = Result<
    Vec<FallibleLiveVisitExecution<Id, Outcome, SuffixOutcome>>,
    FallibleLivePassError<Id, Error, Outcome, SuffixOutcome>,
>;

/// Traverse a mutable live list using retail's saved-`next` contract.
///
/// `plan_visit` receives an immutable state reference and closes the complete
/// ordered journal for the current node before any action is applied.
/// `apply_action` is invoked for every action, in journal order. Its outcome is
/// recorded but never interpreted as a transaction failure: earlier mutations
/// are retained, later actions are still attempted, and `finish_suffix` always
/// runs. The caller represents deferred destruction as an ordinary action, so
/// queuing it cannot suppress the current node's suffix.
///
/// The next link is read before planning or applying the visit. Mutations to
/// that already-read link therefore cannot redirect the current traversal.
/// Mutations to a later, unvisited node remain observable when that node
/// eventually saves its own next link.
pub(crate) fn run_saved_next_live_pass<State, Id, Action, Outcome>(
    state: &mut State,
    first_entity_id: Option<Id>,
    mut next_live_entity_id: impl FnMut(&State, Id) -> Option<Id>,
    mut plan_visit: impl FnMut(&State, Id) -> Vec<Action>,
    mut apply_action: impl FnMut(&mut State, Id, Action) -> Outcome,
    mut finish_suffix: impl FnMut(&mut State, Id),
) -> Vec<LiveVisitExecution<Id, Outcome>>
where
    Id: Copy,
{
    let mut executions = Vec::new();
    let mut cursor = first_entity_id;

    while let Some(entity_id) = cursor {
        let saved_next = next_live_entity_id(state, entity_id);
        let journal = plan_visit(state, entity_id);
        let action_outcomes = journal
            .into_iter()
            .map(|action| apply_action(state, entity_id, action))
            .collect();

        finish_suffix(state, entity_id);
        executions.push(LiveVisitExecution {
            entity_id,
            action_outcomes,
        });
        cursor = saved_next;
    }

    executions
}

/// Fallible saved-`next` traversal for callback passes with a preflight phase.
///
/// `plan_visit` must close the current contact, callback journal, and suffix
/// before returning. An error stops before the visit's first action; earlier
/// visits are intentionally not rolled back. Once a plan exists, action
/// application and its suffix are infallible host commits. A failed retail
/// spawn is therefore represented as an ordinary action outcome, not `Err`,
/// so the suffix still runs.
pub(crate) fn try_run_saved_next_live_pass<
    State,
    Id,
    Action,
    Outcome,
    Suffix,
    SuffixOutcome,
    Error,
>(
    state: &mut State,
    first_entity_id: Option<Id>,
    mut next_live_entity_id: impl FnMut(&State, Id) -> Option<Id>,
    mut plan_visit: impl FnMut(&State, Id) -> Result<FallibleLiveVisitPlan<Action, Suffix>, Error>,
    mut apply_action: impl FnMut(&mut State, Id, Action) -> Outcome,
    mut finish_suffix: impl FnMut(&mut State, Id, Suffix) -> SuffixOutcome,
) -> FallibleLivePassResult<Id, Outcome, SuffixOutcome, Error>
where
    Id: Copy,
{
    let mut executions = Vec::new();
    let mut cursor = first_entity_id;

    while let Some(entity_id) = cursor {
        // Retail saves this link before callback planning or application.
        let saved_next = next_live_entity_id(state, entity_id);
        let plan = match plan_visit(state, entity_id) {
            Ok(plan) => plan,
            Err(source) => {
                return Err(FallibleLivePassError {
                    entity_id,
                    source,
                    completed_executions: executions,
                });
            }
        };
        let action_outcomes = plan
            .actions
            .into_iter()
            .map(|action| apply_action(state, entity_id, action))
            .collect();
        let suffix_outcome = plan
            .suffix
            .map(|suffix| finish_suffix(state, entity_id, suffix));
        executions.push(FallibleLiveVisitExecution {
            entity_id,
            action_outcomes,
            suffix_outcome,
        });
        cursor = saved_next;
    }

    Ok(executions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(Debug, Default)]
    struct LiveList {
        next: BTreeMap<u32, Option<u32>>,
        events: Vec<Event>,
        pending_destroy: Vec<u32>,
        replacements: Vec<u32>,
    }

    impl LiveList {
        fn chain(ids: &[u32]) -> Self {
            let mut state = Self::default();
            for (index, &id) in ids.iter().enumerate() {
                state.next.insert(id, ids.get(index + 1).copied());
            }
            state
        }

        fn next(&self, id: u32) -> Option<u32> {
            self.next.get(&id).copied().flatten()
        }

        fn insert_after(&mut self, predecessor: u32, entity_id: u32) {
            let displaced = self.next(predecessor);
            self.next.insert(entity_id, displaced);
            *self
                .next
                .get_mut(&predecessor)
                .expect("test predecessor must be live") = Some(entity_id);
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Action {
        InsertAfter { predecessor: u32, entity_id: u32 },
        Notify,
        QueueDestroy,
        Spawn { entity_id: u32, succeeds: bool },
        ContinueAfterFailure,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Outcome {
        Applied,
        SpawnFailed,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        Action { visited: u32, action: Action },
        Suffix(u32),
    }

    fn apply(state: &mut LiveList, visited: u32, action: Action) -> Outcome {
        state.events.push(Event::Action { visited, action });
        match action {
            Action::InsertAfter {
                predecessor,
                entity_id,
            } => {
                state.insert_after(predecessor, entity_id);
                Outcome::Applied
            }
            Action::Notify => Outcome::Applied,
            Action::QueueDestroy => {
                state.pending_destroy.push(visited);
                Outcome::Applied
            }
            Action::Spawn {
                entity_id,
                succeeds,
            } => {
                if succeeds {
                    state.replacements.push(entity_id);
                    Outcome::Applied
                } else {
                    Outcome::SpawnFailed
                }
            }
            Action::ContinueAfterFailure => Outcome::Applied,
        }
    }

    fn finish_suffix(state: &mut LiveList, entity_id: u32) {
        state.events.push(Event::Suffix(entity_id));
    }

    #[test]
    fn append_behind_an_unvisited_old_tail_is_visited_in_the_same_pass() {
        let mut state = LiveList::chain(&[1, 2, 3]);

        let executions = run_saved_next_live_pass(
            &mut state,
            Some(1),
            LiveList::next,
            |_state, entity_id| {
                if entity_id == 2 {
                    vec![Action::InsertAfter {
                        predecessor: 3,
                        entity_id: 4,
                    }]
                } else {
                    Vec::new()
                }
            },
            apply,
            finish_suffix,
        );

        assert_eq!(
            executions
                .iter()
                .map(|execution| execution.entity_id)
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
        assert_eq!(
            state
                .events
                .iter()
                .filter_map(|event| match event {
                    Event::Suffix(id) => Some(*id),
                    Event::Action { .. } => None,
                })
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
    }

    #[test]
    fn append_after_a_predecessor_whose_next_was_saved_is_not_visited() {
        let mut state = LiveList::chain(&[1, 2, 3]);

        let executions = run_saved_next_live_pass(
            &mut state,
            Some(1),
            LiveList::next,
            |_state, entity_id| {
                if entity_id == 2 {
                    vec![Action::InsertAfter {
                        predecessor: 2,
                        entity_id: 4,
                    }]
                } else {
                    Vec::new()
                }
            },
            apply,
            finish_suffix,
        );

        assert_eq!(
            executions
                .iter()
                .map(|execution| execution.entity_id)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(state.next(2), Some(4));
        assert_eq!(state.next(4), Some(3));
    }

    #[test]
    fn failed_spawn_keeps_prior_actions_attempts_later_work_and_runs_suffix() {
        let mut state = LiveList::chain(&[9]);

        let executions = run_saved_next_live_pass(
            &mut state,
            Some(9),
            LiveList::next,
            |_state, _entity_id| {
                vec![
                    Action::Notify,
                    Action::QueueDestroy,
                    Action::Spawn {
                        entity_id: 80,
                        succeeds: false,
                    },
                    Action::ContinueAfterFailure,
                ]
            },
            apply,
            finish_suffix,
        );

        assert_eq!(state.pending_destroy, vec![9]);
        assert!(state.replacements.is_empty());
        assert_eq!(
            executions,
            vec![LiveVisitExecution {
                entity_id: 9,
                action_outcomes: vec![
                    Outcome::Applied,
                    Outcome::Applied,
                    Outcome::SpawnFailed,
                    Outcome::Applied,
                ],
            }]
        );
        assert_eq!(
            state.events,
            vec![
                Event::Action {
                    visited: 9,
                    action: Action::Notify,
                },
                Event::Action {
                    visited: 9,
                    action: Action::QueueDestroy,
                },
                Event::Action {
                    visited: 9,
                    action: Action::Spawn {
                        entity_id: 80,
                        succeeds: false,
                    },
                },
                Event::Action {
                    visited: 9,
                    action: Action::ContinueAfterFailure,
                },
                Event::Suffix(9),
            ]
        );
    }

    #[test]
    fn fallible_plan_stops_before_current_visit_mutation() {
        let mut state = LiveList::chain(&[1, 2, 3]);

        let error = try_run_saved_next_live_pass(
            &mut state,
            Some(1),
            LiveList::next,
            |_state, entity_id| {
                if entity_id == 2 {
                    Err("unresolved callback")
                } else {
                    Ok(FallibleLiveVisitPlan {
                        actions: vec![Action::Notify],
                        suffix: Some(()),
                    })
                }
            },
            apply,
            |state, entity_id, ()| finish_suffix(state, entity_id),
        )
        .unwrap_err();

        assert_eq!(
            error,
            FallibleLivePassError {
                entity_id: 2,
                source: "unresolved callback",
                completed_executions: vec![FallibleLiveVisitExecution {
                    entity_id: 1,
                    action_outcomes: vec![Outcome::Applied],
                    suffix_outcome: Some(()),
                }],
            }
        );
        assert_eq!(
            state.events,
            [
                Event::Action {
                    visited: 1,
                    action: Action::Notify,
                },
                Event::Suffix(1),
            ]
        );
    }
}
